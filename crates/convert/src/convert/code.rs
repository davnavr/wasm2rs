//! The entrypoint for converting a WebAssembly byte code into Rust source code.

mod builder;

pub(in crate::convert) struct Code<'wasm> {
    body: wasmparser::FunctionBody<'wasm>,
    validator: wasmparser::FuncToValidate<wasmparser::ValidatorResources>,
}

#[must_use]
pub(in crate::convert) struct Definition {
    // Caller is responsible for returning these to the pool.
    pub(in crate::convert) body: Vec<crate::ast::Statement>,
    pub(in crate::convert) arena: crate::ast::Arena,
}

impl Definition {
    pub(in crate::convert) fn finish(self, allocations: &crate::Allocations) {
        allocations.return_statement_buffer(self.body);
        allocations.return_ast_arena(self.arena);
    }
}

type FuncValidator = wasmparser::FuncValidator<wasmparser::ValidatorResources>;

fn get_block_id(validator: &FuncValidator) -> crate::ast::BlockId {
    // Subtract two, 1 for the newly added block, and 1 for the implicit block in the start of
    // every function.
    crate::ast::BlockId(validator.control_stack_height() - 2)
}

fn resolve_block_type(
    types: &wasmparser::types::Types,
    block_type: wasmparser::BlockType,
) -> std::borrow::Cow<'_, wasmparser::FuncType> {
    use std::borrow::Cow;
    use wasmparser::{BlockType, FuncType};

    match block_type {
        BlockType::Empty => Cow::Owned(FuncType::new([], [])),
        BlockType::Type(result) => Cow::Owned(FuncType::new([], [result])),
        BlockType::FuncType(type_idx) => {
            Cow::Borrowed(types[types.core_type_at(type_idx).unwrap_sub()].unwrap_func())
        }
    }
}

fn convert_block_start(
    builder: &mut builder::Builder,
    block_type: wasmparser::BlockType,
    kind: crate::ast::BlockKind,
    types: &wasmparser::types::Types,
    validator: &FuncValidator,
) -> crate::Result<()> {
    let block_type = resolve_block_type(types, block_type);

    let results =
        builder.get_block_results(block_type.results().len(), block_type.params().len())?;

    builder.emit_statement(crate::ast::Statement::BlockStart {
        id: get_block_id(validator),
        results,
        kind,
    })
}

fn calculate_branch_target(
    relative_depth: u32,
    function_type: &wasmparser::FuncType,
    validator: &FuncValidator,
    builder: &mut builder::Builder,
    types: &wasmparser::types::Types,
) -> crate::Result<(crate::ast::BranchTarget, crate::ast::ExprListId)> {
    let frame = validator
        .get_control_frame(relative_depth as usize)
        .unwrap();

    let block_type = resolve_block_type(types, frame.block_type);
    let target;
    let popped_count;
    match (validator.control_stack_height() - relative_depth).checked_sub(2) {
        None => {
            target = crate::ast::BranchTarget::Return;
            popped_count = function_type.results().len();
        }
        Some(id) => {
            let id = crate::ast::BlockId(id);
            if frame.kind == wasmparser::FrameKind::Loop {
                target = crate::ast::BranchTarget::Loop(id);
                popped_count = block_type.params().len();
            } else {
                target = crate::ast::BranchTarget::Block(id);
                popped_count = block_type.results().len();
            }
        }
    };

    Ok((target, builder.wasm_operand_stack_pop_list(popped_count)?))
}

#[derive(Debug)]
pub(in crate::convert) struct Attributes {
    pub(in crate::convert) call_kind: crate::context::CallKind,
    pub(in crate::convert) unwind_kind: crate::context::UnwindKind,
}

fn convert_impl(
    allocations: &crate::Allocations,
    options: &crate::Convert<'_>,
    types: &wasmparser::types::Types,
    body: wasmparser::FunctionBody<'_>,
    mut validator: FuncValidator,
) -> crate::Result<(Attributes, Definition)> {
    use anyhow::Context;

    let func_id = crate::ast::FuncId(validator.index());
    let func_type = types[types.core_function_at(func_id.0)].unwrap_func();

    // TODO: Reserve space in Vec<Statement>, collect data on avg. # of statements per byte of code
    let mut builder = builder::Builder::new(allocations);

    {
        let mut locals = body
            .get_locals_reader()
            .context("could not obtain locals")?;

        let mut id = crate::ast::LocalId(func_type.params().len() as u32);
        for _ in 0..locals.get_count() {
            let (count, ty) = locals.read().context("could not read local variables")?;
            validator.define_locals(locals.original_position(), count, ty)?;
            for _ in 0..count {
                builder.emit_statement(crate::ast::Statement::DefineLocal(id, ty.into()))?;
                id.0 += 1;
            }
        }
    }

    let mut operators = body
        .get_operators_reader()
        .context("could not obtain operators")?;

    while !operators.eof() {
        use crate::ast::{I32StorageSize, I64StorageSize, LoadKind, SignExtensionMode, StoreKind};
        use wasmparser::Operator;

        let (op, op_offset) = operators.read_with_offset()?;

        // `unwrap()` not used in case `read_with_offset` erroneously returns too many
        // operators.
        let current_frame = *validator
            .get_control_frame(0)
            .context("control frame stack was unexpectedly empty")?;

        // Validates all instructions, even "unreachable" ones
        validator.op(op_offset, &op)?;

        if current_frame.unreachable && !matches!(op, Operator::End | Operator::Else) {
            // Don't generate Rust code for unreachable instructions.
            continue;
        }

        macro_rules! un_op {
            ($name:ident) => {{
                let c_1 = builder.pop_wasm_operand();
                builder.push_wasm_operand(crate::ast::Expr::UnaryOperator {
                    kind: crate::ast::UnOp::$name,
                    c_1,
                })?;
            }};
        }

        macro_rules! bin_op {
            ($name:ident) => {{
                let c_2 = builder.pop_wasm_operand();
                let c_1 = builder.pop_wasm_operand();
                builder.push_wasm_operand(crate::ast::Expr::BinaryOperator {
                    kind: crate::ast::BinOp::$name,
                    c_1,
                    c_2,
                })?;
            }};
        }

        macro_rules! bin_op_trapping {
            ($name:ident) => {{
                builder.can_trap();
                bin_op!($name);
            }};
        }

        macro_rules! memory_load {
            ($memarg:ident => $kind:expr) => {{
                builder.can_trap();
                builder.needs_self();
                let address = builder.pop_wasm_operand();
                builder.push_wasm_operand(crate::ast::Expr::MemoryLoad {
                    memory: crate::ast::MemoryId($memarg.memory),
                    kind: $kind,
                    address,
                    offset: $memarg.offset,
                })?;
            }};
        }

        macro_rules! memory_store {
            ($memarg:ident => $kind:ident) => {{
                builder.can_trap();
                builder.needs_self();
                let value = builder.pop_wasm_operand();
                let address = builder.pop_wasm_operand();
                builder.emit_statement(crate::ast::Statement::MemoryStore {
                    memory: crate::ast::MemoryId($memarg.memory),
                    kind: StoreKind::$kind,
                    address,
                    value,
                    offset: $memarg.offset,
                })?;
            }};
        }

        match op {
            Operator::Unreachable => {
                builder.can_trap();
                builder.wasm_operand_stack_truncate(validator.operand_stack_height() as usize)?;
                builder.emit_statement(crate::ast::Statement::Unreachable {
                    offset: u32::try_from(op_offset - body.range().start).unwrap_or(u32::MAX),
                })?;
            }
            Operator::Nop => (),
            Operator::Block { blockty } => {
                // Could avoid generating blocks if `current_frame.unreachable`
                convert_block_start(
                    &mut builder,
                    blockty,
                    crate::ast::BlockKind::Block,
                    types,
                    &validator,
                )?;
            }
            Operator::Loop { blockty } => {
                // See `convert_block_start`
                let block_id = get_block_id(&validator);
                let block_type = resolve_block_type(types, blockty);
                let input_count = block_type.params().len();
                let result_count = block_type.results().len();

                let inputs = builder.wasm_operand_stack_move_loop_inputs(block_id, input_count)?;
                let results = builder.get_block_results(result_count, input_count)?;

                builder.emit_statement(crate::ast::Statement::BlockStart {
                    id: block_id,
                    results,
                    kind: crate::ast::BlockKind::Loop { inputs },
                })?;

                // Push the loops inputs back onto the stack
                for number in 0u32..(input_count as u32) {
                    builder.push_wasm_operand(crate::ast::Expr::LoopInput(
                        crate::ast::LoopInput {
                            r#loop: block_id,
                            number,
                        },
                    ))?;
                }
            }
            Operator::If { blockty } => {
                let condition = builder.pop_wasm_operand();
                convert_block_start(
                    &mut builder,
                    blockty,
                    crate::ast::BlockKind::If { condition },
                    types,
                    &validator,
                )?;
            }
            Operator::Else => {
                let block_type = resolve_block_type(types, current_frame.block_type);

                let previous_results =
                    builder.wasm_operand_stack_pop_list(block_type.results().len())?;

                debug_assert_eq!(current_frame.height, builder.wasm_operand_stack().len());

                // Re-apply the input operands
                for i in 0..block_type.params().len() {
                    builder.push_wasm_operand(crate::ast::Expr::Temporary(crate::ast::TempId(
                        (current_frame.height + i) as u32,
                    )))?;
                }

                builder.emit_statement(crate::ast::Statement::Else { previous_results })?;
            }
            Operator::End => {
                if validator.control_stack_height() >= 1 {
                    let id = crate::ast::BlockId(validator.control_stack_height() - 1);
                    let result_count = resolve_block_type(types, current_frame.block_type)
                        .results()
                        .len();

                    let kind = match current_frame.kind {
                        wasmparser::FrameKind::Block => crate::ast::BlockKind::Block,
                        wasmparser::FrameKind::Loop => crate::ast::BlockKind::Loop { inputs: () },
                        wasmparser::FrameKind::Else | wasmparser::FrameKind::If => {
                            crate::ast::BlockKind::If { condition: () }
                        }
                        bad => anyhow::bail!("TODO: support for {bad:?}"),
                    };

                    if !current_frame.unreachable {
                        let results = builder.wasm_operand_stack_pop_list(result_count)?;

                        builder.push_block_results(result_count)?;
                        builder.emit_statement(crate::ast::Statement::BlockEnd {
                            id,
                            kind,
                            results,
                        })?;
                    } else {
                        builder.emit_statement(crate::ast::Statement::BlockEndUnreachable {
                            id,
                            kind,
                            has_results: result_count > 0,
                        })?;

                        builder.wasm_operand_stack_truncate(
                            validator.operand_stack_height() as usize
                        )?;

                        builder.push_block_results(result_count)?;
                    }
                } else if current_frame.unreachable {
                    builder.wasm_operand_stack_truncate(current_frame.height)?;
                } else {
                    let result_count = func_type.results().len();

                    debug_assert_eq!(
                        current_frame.height + result_count,
                        builder.wasm_operand_stack().len() - current_frame.height,
                        "value stack height mismatch ({:?})",
                        builder.wasm_operand_stack()
                    );

                    let results = builder.wasm_operand_stack_pop_to_height(current_frame.height)?;

                    debug_assert_eq!(result_count, results.len() as usize);

                    builder.emit_statement(crate::ast::Statement::r#return(results))?;
                }
            }
            Operator::Br { relative_depth } => {
                let (target, values) = calculate_branch_target(
                    relative_depth,
                    func_type,
                    &validator,
                    &mut builder,
                    types,
                )?;

                builder.wasm_operand_stack_truncate(validator.operand_stack_height() as usize)?;
                builder.emit_statement(crate::ast::Statement::Branch {
                    target,
                    values,
                    condition: None,
                })?;
            }
            Operator::BrIf { relative_depth } => {
                let condition = builder.pop_wasm_operand();
                let (target, values) = calculate_branch_target(
                    relative_depth,
                    func_type,
                    &validator,
                    &mut builder,
                    types,
                )?;

                builder.wasm_operand_stack_truncate(validator.operand_stack_height() as usize)?;
                builder.emit_statement(crate::ast::Statement::Branch {
                    target,
                    values,
                    condition: Some(condition),
                })?;
            }
            Operator::Return => {
                // Unlike the last `end` instruction, `return` allows values on the stack that
                // weren't popped.

                let result_count = func_type.results().len();

                debug_assert!(builder.wasm_operand_stack().len() >= result_count);

                let results = builder.wasm_operand_stack_pop_list(result_count)?;

                // Any values that weren't popped are spilled into temporaries.
                builder.emit_statement(crate::ast::Statement::r#return(results))?;
            }
            Operator::Call { function_index } => {
                let signature = types[types.core_function_at(function_index)].unwrap_func();

                let result_count = signature.results().len();
                let arguments = builder.wasm_operand_stack_pop_list(signature.params().len())?;
                let callee = crate::ast::FuncId(function_index);

                // TODO: Fix, call_conv of current function has to support call_convs of called functions, so fix it up later
                builder.can_trap();
                builder.needs_self();

                if result_count == 1 {
                    builder.push_wasm_operand(crate::ast::Expr::Call { callee, arguments })?;
                } else {
                    // Multiple results are translated into Rust tuples, which need to be destructured.
                    let results =
                        std::num::NonZeroU32::new(result_count as u32).map(|result_count| {
                            (
                                crate::ast::TempId(builder.wasm_operand_stack().len() as u32),
                                result_count,
                            )
                        });

                    builder.emit_statement(crate::ast::Statement::Call {
                        callee,
                        arguments,
                        results,
                    })?;

                    builder.push_block_results(result_count)?;
                }
            }
            //Operator::CallIndirect
            Operator::Drop => {
                let expr = builder.pop_wasm_operand();
                builder.emit_statement(expr)?;
            }
            Operator::LocalGet { local_index } => {
                builder.push_wasm_operand(crate::ast::Expr::GetLocal(crate::ast::LocalId(
                    local_index,
                )))?;
            }
            Operator::LocalSet { local_index } => {
                let value = builder.pop_wasm_operand();
                builder.emit_statement(crate::ast::Statement::SetLocal {
                    local: crate::ast::LocalId(local_index),
                    value,
                })?;
            }
            //Operator::LocalTee
            Operator::GlobalGet { global_index } => {
                builder.needs_self(); // TODO: If global is const, don't need `self`.
                builder.push_wasm_operand(crate::ast::Expr::GetGlobal(crate::ast::GlobalId(
                    global_index,
                )))?;
            }
            Operator::GlobalSet { global_index } => {
                let value = builder.pop_wasm_operand();
                builder.needs_self(); // TODO: If global is const, don't need `self`.
                builder.emit_statement(crate::ast::Statement::SetGlobal {
                    global: crate::ast::GlobalId(global_index),
                    value,
                })?;
            }
            Operator::I32Load { memarg } => memory_load!(memarg => LoadKind::I32),
            Operator::I64Load { memarg } => memory_load!(memarg => LoadKind::I64),
            Operator::F32Load { memarg } => memory_load!(memarg => LoadKind::F32),
            Operator::F64Load { memarg } => memory_load!(memarg => LoadKind::F64),
            Operator::I32Load8S { memarg } => {
                memory_load!(memarg => LoadKind::AsI32 {
                    storage_size: I32StorageSize::I8,
                    sign_extension: SignExtensionMode::Signed,
                })
            }
            Operator::I32Load8U { memarg } => {
                memory_load!(memarg => LoadKind::AsI32 {
                    storage_size: I32StorageSize::I8,
                    sign_extension: SignExtensionMode::Unsigned,
                })
            }
            Operator::I32Load16S { memarg } => {
                memory_load!(memarg => LoadKind::AsI32 {
                    storage_size: I32StorageSize::I16,
                    sign_extension: SignExtensionMode::Signed,
                })
            }
            Operator::I32Load16U { memarg } => {
                memory_load!(memarg => LoadKind::AsI32 {
                    storage_size: I32StorageSize::I16,
                    sign_extension: SignExtensionMode::Unsigned,
                })
            }
            Operator::I64Load8S { memarg } => {
                memory_load!(memarg => LoadKind::AsI64 {
                    storage_size: I64StorageSize::I8,
                    sign_extension: SignExtensionMode::Signed,
                })
            }
            Operator::I64Load8U { memarg } => {
                memory_load!(memarg => LoadKind::AsI64 {
                    storage_size: I64StorageSize::I8,
                    sign_extension: SignExtensionMode::Unsigned,
                })
            }
            Operator::I64Load16S { memarg } => {
                memory_load!(memarg => LoadKind::AsI64 {
                    storage_size: I64StorageSize::I16,
                    sign_extension: SignExtensionMode::Signed,
                })
            }
            Operator::I64Load16U { memarg } => {
                memory_load!(memarg => LoadKind::AsI64 {
                    storage_size: I64StorageSize::I16,
                    sign_extension: SignExtensionMode::Unsigned,
                })
            }
            Operator::I64Load32S { memarg } => {
                memory_load!(memarg => LoadKind::AsI64 {
                    storage_size: I64StorageSize::I32,
                    sign_extension: SignExtensionMode::Signed,
                })
            }
            Operator::I64Load32U { memarg } => {
                memory_load!(memarg => LoadKind::AsI64 {
                    storage_size: I64StorageSize::I32,
                    sign_extension: SignExtensionMode::Unsigned,
                })
            }
            Operator::I32Store { memarg } => memory_store!(memarg => I32),
            Operator::I64Store { memarg } => memory_store!(memarg => I64),
            Operator::F32Store { memarg } => memory_store!(memarg => F32),
            Operator::F64Store { memarg } => memory_store!(memarg => F64),
            Operator::I32Store8 { memarg } | Operator::I64Store8 { memarg } => {
                memory_store!(memarg => I8);
            }
            Operator::I32Store16 { memarg } | Operator::I64Store16 { memarg } => {
                memory_store!(memarg => I16);
            }
            Operator::I64Store32 { memarg } => memory_store!(memarg => AsI32),
            Operator::MemorySize { mem, mem_byte: _ } => {
                builder.needs_self();
                builder
                    .push_wasm_operand(crate::ast::Expr::MemorySize(crate::ast::MemoryId(mem)))?;
            }
            Operator::MemoryGrow { mem, mem_byte: _ } => {
                builder.needs_self();
                let delta = builder.pop_wasm_operand();
                builder.push_wasm_operand(crate::ast::Expr::MemoryGrow {
                    memory: crate::ast::MemoryId(mem),
                    delta,
                })?;
            }
            // Misc. memory instructions
            Operator::I32Const { value } => {
                builder.push_wasm_operand(crate::ast::Literal::I32(value))?;
            }
            Operator::I64Const { value } => {
                builder.push_wasm_operand(crate::ast::Literal::I64(value))?;
            }
            Operator::F32Const { value } => {
                builder.push_wasm_operand(crate::ast::Literal::F32(value.bits()))?;
            }
            Operator::F64Const { value } => {
                builder.push_wasm_operand(crate::ast::Literal::F64(value.bits()))?;
            }
            Operator::I32Eqz | Operator::I64Eqz => un_op!(IxxEqz),
            Operator::I32Eq | Operator::I64Eq | Operator::F32Eq | Operator::F64Eq => bin_op!(Eq),
            Operator::I32Ne | Operator::I64Ne | Operator::F32Ne | Operator::F64Ne => bin_op!(Ne),
            Operator::I32LtS | Operator::I64LtS => bin_op!(IxxLtS),
            Operator::I32LtU => bin_op!(I32LtU),
            Operator::I32GtS | Operator::I64GtS => bin_op!(IxxGtS),
            Operator::I32GtU => bin_op!(I32GtU),
            Operator::I64LtU => bin_op!(I64LtU),
            Operator::I64GtU => bin_op!(I64GtU),
            Operator::I32LeS | Operator::I64LeS => bin_op!(IxxLeS),
            Operator::I32LeU => bin_op!(I32LeU),
            Operator::I32GeS | Operator::I64GeS => bin_op!(IxxGeS),
            Operator::I32GeU => bin_op!(I32GeU),
            Operator::I64LeU => bin_op!(I64LeU),
            Operator::I64GeU => bin_op!(I64GeU),
            // TODO: See if Rust's implementation of float comparison follows WebAssembly.
            Operator::F32Gt | Operator::F64Gt => bin_op!(FxxGt),
            Operator::I32Clz => un_op!(I32Clz),
            Operator::I64Clz => un_op!(I64Clz),
            Operator::I32Ctz => un_op!(I32Ctz),
            Operator::I64Ctz => un_op!(I64Ctz),
            Operator::I32Popcnt => un_op!(I32Popcnt),
            Operator::I64Popcnt => un_op!(I64Popcnt),
            Operator::I32Add => bin_op!(I32Add),
            Operator::I64Add => bin_op!(I64Add),
            Operator::I32Sub => bin_op!(I32Sub),
            Operator::I64Sub => bin_op!(I64Sub),
            Operator::I32Mul => bin_op!(I32Mul),
            Operator::I64Mul => bin_op!(I64Mul),
            Operator::I32DivS => bin_op_trapping!(I32DivS),
            Operator::I64DivS => bin_op_trapping!(I64DivS),
            Operator::I32DivU => bin_op_trapping!(I32DivU),
            Operator::I64DivU => bin_op_trapping!(I64DivU),
            Operator::I32RemS => bin_op_trapping!(I32RemS),
            Operator::I64RemS => bin_op_trapping!(I64RemS),
            Operator::I32RemU => bin_op_trapping!(I32RemU),
            Operator::I64RemU => bin_op_trapping!(I64RemU),
            Operator::I32And | Operator::I64And => bin_op!(IxxAnd),
            Operator::I32Or | Operator::I64Or => bin_op!(IxxOr),
            Operator::I32Xor | Operator::I64Xor => bin_op!(IxxXor),
            Operator::I32Shl => bin_op!(I32Shl),
            Operator::I64Shl => bin_op!(I64Shl),
            Operator::I32ShrS => bin_op!(I32ShrS),
            Operator::I64ShrS => bin_op!(I64ShrS),
            Operator::I32ShrU => bin_op!(I32ShrU),
            Operator::I64ShrU => bin_op!(I64ShrU),
            Operator::I32Rotl => bin_op!(I32Rotl),
            Operator::I64Rotl => bin_op!(I64Rotl),
            Operator::I32Rotr => bin_op!(I32Rotr),
            Operator::I64Rotr => bin_op!(I64Rotr),
            Operator::F32Neg | Operator::F64Neg => un_op!(FxxNeg),
            Operator::I32WrapI64 => un_op!(I32WrapI64),
            Operator::I32TruncF32S => {
                builder.can_trap();
                un_op!(I32TruncF32S);
            }
            Operator::I32TruncF32U => {
                builder.can_trap();
                un_op!(I32TruncF32U)
            }
            Operator::I32TruncF64S => {
                builder.can_trap();
                un_op!(I32TruncF64S)
            }
            Operator::I32TruncF64U => {
                builder.can_trap();
                un_op!(I32TruncF64U)
            }
            Operator::I64ExtendI32S => un_op!(I64ExtendI32S),
            Operator::I64ExtendI32U => un_op!(I64ExtendI32U),
            Operator::I64TruncF32S => {
                builder.can_trap();
                un_op!(I64TruncF32S)
            }
            Operator::I64TruncF32U => {
                builder.can_trap();
                un_op!(I64TruncF32U)
            }
            Operator::I64TruncF64S => {
                builder.can_trap();
                un_op!(I64TruncF64S)
            }
            Operator::I64TruncF64U => {
                builder.can_trap();
                un_op!(I64TruncF64U)
            }
            Operator::F32ConvertI32S | Operator::F32ConvertI64S => un_op!(F32ConvertIxxS),
            Operator::F32ConvertI32U => un_op!(F32ConvertI32U),
            Operator::F32ConvertI64U => un_op!(F32ConvertI64U),
            Operator::F32DemoteF64 => un_op!(F32DemoteF64),
            Operator::F64ConvertI32S | Operator::F64ConvertI64S => un_op!(F64ConvertIxxS),
            Operator::F64ConvertI32U => un_op!(F64ConvertI32U),
            Operator::F64ConvertI64U => un_op!(F64ConvertI64U),
            Operator::F64PromoteF32 => un_op!(F64PromoteF32),
            Operator::I32ReinterpretF32 => un_op!(I32ReinterpretF32),
            Operator::I64ReinterpretF64 => un_op!(I64ReinterpretF64),
            Operator::F32ReinterpretI32 => un_op!(F32ReinterpretI32),
            Operator::F64ReinterpretI64 => un_op!(F64ReinterpretI64),
            Operator::I32Extend8S => un_op!(I32Extend8S),
            Operator::I32Extend16S => un_op!(I32Extend16S),
            Operator::I64Extend8S => un_op!(I64Extend8S),
            Operator::I64Extend16S => un_op!(I64Extend16S),
            Operator::I64Extend32S => un_op!(I64Extend32S),
            Operator::I32TruncSatF32S | Operator::I32TruncSatF64S => un_op!(I32TruncSatFxxS),
            Operator::I32TruncSatF32U | Operator::I32TruncSatF64U => un_op!(I32TruncSatFxxU),
            Operator::I64TruncSatF32S | Operator::I64TruncSatF64S => un_op!(I64TruncSatFxxS),
            Operator::I64TruncSatF32U | Operator::I64TruncSatF64U => un_op!(I64TruncSatFxxU),
            _ => anyhow::bail!("translation of operation is not yet supported: {op:?}"),
        }

        if !operators.eof() && cfg!(debug_assertions) {
            // Ensure that the two operand stacks stay in sync
            let validator_height = validator.operand_stack_height() as usize;
            let builder_height = builder.wasm_operand_stack().len();
            if validator_height != builder_height {
                anyhow::bail!(
                    "expected {validator_height} items on stack, but got {builder_height} \
                    items ({:?}) after {op:?} (top of validator's stack was {:?})\
                    , at {op_offset:#X}",
                    builder.wasm_operand_stack(),
                    validator.get_operand_type(0),
                );
            }
        }
    }

    // `Statement::Return` already generated when last `end` is handled.
    validator.finish(operators.original_position())?;

    allocations.return_func_validator_allocations(validator.into_allocations());

    // TODO: Collect info for optimizations (e.g. what locals were mutated?).
    Ok(builder.finish())
}

impl<'wasm> Code<'wasm> {
    pub(in crate::convert) fn new(
        validator: &mut wasmparser::Validator,
        body: wasmparser::FunctionBody<'wasm>,
    ) -> crate::Result<Self> {
        Ok(Self {
            validator: validator.code_section_entry(&body)?,
            body,
        })
    }

    /// Converts a [WebAssembly function body] into a series of [`Statement`]s modeling Rust
    /// source code.
    ///
    /// To ensure allocations are properly reused, the returned `Vec<Statement>` should be returned
    /// to the pool of [`crate::Allocations`].
    ///
    /// [WebAssembly function body]: https://webassembly.github.io/spec/core/syntax/modules.html#syntax-func
    /// [`Statement`]: crate::ast::Statement
    pub(in crate::convert) fn convert(
        self,
        allocations: &crate::Allocations,
        options: &crate::Convert<'_>,
        types: &wasmparser::types::Types,
    ) -> crate::Result<(Attributes, Definition)> {
        use anyhow::Context;

        let validator = self
            .validator
            .into_validator(allocations.take_func_validator_allocations());

        let index = validator.index();

        convert_impl(allocations, options, types, self.body, validator)
            .with_context(|| format!("could not convert function #{index}"))
    }
}
