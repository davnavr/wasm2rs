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

fn convert_block_start(
    builder: &mut builder::Builder,
    block_type: wasmparser::BlockType,
    kind: crate::ast::BlockKind,
    module: &crate::convert::Module,
    validator: &FuncValidator,
) -> crate::Result<()> {
    let block_type = module.resolve_block_type(block_type);

    let results =
        builder.get_block_results(block_type.results().len(), block_type.params().len())?;

    builder.emit_statement(crate::ast::Statement::BlockStart {
        id: crate::ast::BlockId(validator.control_stack_height()),
        results,
        kind,
    })
}

fn convert_impl<'wasm, 'types>(
    mut validator: FuncValidator,
    body: wasmparser::FunctionBody<'wasm>,
    module: &'types crate::convert::Module<'wasm>,
    options: &crate::Convert<'_>,
    allocations: &crate::Allocations,
) -> crate::Result<(crate::context::CallConv<'types>, Definition)> {
    use anyhow::Context;

    let func_id = crate::ast::FuncId(validator.index());
    let func_type = module.types[module.types.core_function_at(func_id.0)].unwrap_func();

    // TODO: Reserve space in Vec<Statement>, collect data on avg. # of statements per byte of code
    let mut builder = builder::Builder::new(allocations, func_type);

    {
        let mut locals = body
            .get_locals_reader()
            .context("could not obtain locals")?;

        let mut id = crate::ast::LocalId(func_type.params().len() as u32);
        for _ in 0..locals.get_count() {
            let (count, ty) = locals.read().context("could not read local variables")?;
            validator.define_locals(locals.original_position(), count, ty)?;
            for _ in 0..count {
                builder.emit_statement(crate::ast::Statement::LocalDefinition(id, ty.into()))?;
                id.0 += 1;
            }
        }
    }

    let mut operators = body
        .get_operators_reader()
        .context("could not obtain operators")?;

    while !operators.eof() {
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

        match op {
            Operator::Unreachable => {
                builder.can_trap();
                builder.emit_statement(crate::ast::Statement::Unreachable {
                    function: func_id,
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
                    module,
                    &validator,
                )?;
            }
            Operator::Loop { blockty } => {
                // See `convert_block_start`
                let block_id = crate::ast::BlockId(validator.control_stack_height());
                let block_type = module.resolve_block_type(blockty);
                let input_count = block_type.params().len();
                let result_count = block_type.params().len();

                let inputs = builder.wasm_operand_stack_move_loop_inputs(block_id, input_count)?;
                let results = builder.get_block_results(result_count, input_count)?;

                builder.emit_statement(crate::ast::Statement::BlockStart {
                    id: block_id,
                    results,
                    kind: crate::ast::BlockKind::Loop { inputs },
                })?;
            }
            Operator::If { blockty } => {
                let condition = builder.pop_wasm_operand();
                convert_block_start(
                    &mut builder,
                    blockty,
                    crate::ast::BlockKind::If { condition },
                    module,
                    &validator,
                )?;
            }
            Operator::Else => {
                let block_type = module.resolve_block_type(current_frame.block_type);

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
                if current_frame.unreachable {
                    builder.wasm_operand_stack_truncate(current_frame.height)?;
                }

                if validator.control_stack_height() >= 1 {
                    let result_count = module
                        .resolve_block_type(current_frame.block_type)
                        .results()
                        .len();

                    let results = builder.wasm_operand_stack_pop_list(result_count)?;

                    builder.push_block_results(result_count)?;
                    builder.emit_statement(crate::ast::Statement::BlockEnd {
                        id: crate::ast::BlockId(validator.control_stack_height()),
                        kind: match current_frame.kind {
                            wasmparser::FrameKind::Block => crate::ast::BlockKind::Block,
                            wasmparser::FrameKind::Else | wasmparser::FrameKind::If => {
                                crate::ast::BlockKind::If { condition: () }
                            }
                            bad => anyhow::bail!("TODO: support for {bad:?}"),
                        },
                        results,
                    })?;
                } else if !current_frame.unreachable {
                    let result_count = func_type.results().len();

                    debug_assert_eq!(
                        current_frame.height + result_count,
                        builder.wasm_operand_stack().len() - current_frame.height,
                        "value stack height mismatch ({:?})",
                        builder.wasm_operand_stack()
                    );

                    let results = builder.wasm_operand_stack_pop_to_height(current_frame.height)?;

                    debug_assert_eq!(result_count, results.len() as usize);

                    builder.emit_statement(crate::ast::Statement::Return(results))?;
                }
            }
            // Operator::Br { relative_depth } => {}
            Operator::Return => {
                // Unlike the last `end` instruction, `return` allows values on the stack that
                // weren't popped.

                let result_count = func_type.results().len();

                debug_assert!(builder.wasm_operand_stack().len() >= result_count);

                let results = builder.wasm_operand_stack_pop_list(result_count)?;

                // Any values that weren't popped are spilled into temporaries.
                builder.emit_statement(crate::ast::Statement::Return(results))?;
            }
            Operator::Call { function_index } => {
                let signature =
                    module.types[module.types.core_function_at(function_index)].unwrap_func();

                let arguments = builder.wasm_operand_stack_pop_list(signature.params().len())?;

                debug_assert!(
                    signature.results().len() <= 1,
                    "multi results not yet supported, have to put results into temporaries"
                );

                // TODO: Fix, call_conv of current function has to support call_convs of called functions, so fix it up later
                builder.can_trap();
                builder.needs_self();

                builder.push_wasm_operand(crate::ast::Expr::Call {
                    callee: crate::ast::FuncId(function_index),
                    arguments,
                })?;
            }
            Operator::LocalGet { local_index } => {
                builder.push_wasm_operand(crate::ast::Expr::GetLocal(crate::ast::LocalId(
                    local_index,
                )))?;
            }
            Operator::LocalSet { local_index } => {
                let value = builder.pop_wasm_operand();
                builder.emit_statement(crate::ast::Statement::LocalSet {
                    local: crate::ast::LocalId(local_index),
                    value,
                })?;
            }
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
            Operator::I32TruncF32S => un_op!(I32TruncF32S),
            Operator::I32TruncF32U => un_op!(I32TruncF32U),
            Operator::I32TruncF64S => un_op!(I32TruncF64S),
            Operator::I32TruncF64U => un_op!(I32TruncF64U),
            Operator::I64ExtendI32S => un_op!(I64ExtendI32S),
            Operator::I64ExtendI32U => un_op!(I64ExtendI32U),
            Operator::I64TruncF32S => un_op!(I64TruncF32S),
            Operator::I64TruncF32U => un_op!(I64TruncF32U),
            Operator::I64TruncF64S => un_op!(I64TruncF64S),
            Operator::I64TruncF64U => un_op!(I64TruncF64U),
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

        if !operators.eof() {
            // Ensure that the two operand stacks stay in sync
            debug_assert_eq!(
                validator.operand_stack_height() as usize,
                builder.wasm_operand_stack().len(),
                "expected operand stack {:?} after {op:?} (top of validator's stack was {:?})",
                builder.wasm_operand_stack(),
                validator.get_operand_type(0),
            );
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
    pub(in crate::convert) fn convert<'types>(
        self,
        module: &'types crate::convert::Module<'wasm>,
        options: &crate::Convert<'_>,
        allocations: &crate::Allocations,
    ) -> crate::Result<(crate::context::CallConv<'types>, Definition)> {
        use anyhow::Context;

        let validator = self
            .validator
            .into_validator(allocations.take_func_validator_allocations());

        let index = validator.index();

        convert_impl(validator, self.body, module, options, allocations)
            .with_context(|| format!("could not format function #{index}"))
    }
}
