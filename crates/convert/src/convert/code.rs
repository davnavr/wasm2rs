//! The entrypoint for converting a WebAssembly byte code into Rust source code.

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

#[must_use]
struct StatementBuilder<'a> {
    wasm_operand_stack: Vec<crate::ast::ExprId>,
    buffer: Vec<crate::ast::Statement>,
    ast_arena: crate::ast::Arena,
    calling_convention: crate::context::CallConv<'a>,
}

impl<'a> StatementBuilder<'a> {
    fn new(allocations: &crate::Allocations, wasm_signature: &'a wasmparser::FuncType) -> Self {
        // TODO: Value stack should be taken from `allocations`.
        Self {
            // TODO: Reserve space in Vec<ExprId>, collect data on avg. max stack height
            wasm_operand_stack: Vec::new(),
            buffer: allocations.take_statement_buffer(),
            ast_arena: allocations.take_ast_arena(),
            calling_convention: crate::context::CallConv {
                call_kind: crate::context::CallKind::Function,
                can_trap: false,
                wasm_signature,
            },
        }
    }

    fn can_trap(&mut self) {
        self.calling_convention.can_trap = true;
    }

    fn push_wasm_operand(
        &mut self,
        operand: impl Into<crate::ast::Expr>,
    ) -> Result<(), crate::ast::ArenaError> {
        self.ast_arena
            .allocate(operand)
            .map(|expr| self.wasm_operand_stack.push(expr))
    }

    fn pop_wasm_operand(&mut self) -> crate::ast::ExprId {
        // Stack underflows are handled later by a `FuncValidator`, so this can't panic.
        if let Some(expr) = self.wasm_operand_stack.pop() {
            expr
        } else {
            todo!("special Expr value for operand stack underflow bugs")
        }
    }

    fn flush_operands_to_temporaries(&mut self) {
        todo!("flushing of wasm operand stack to temporaries not yet implemented");
        // TODO: iter_mut for self.wasm_operand_stack, write temporaries
    }

    fn emit_statement_inner(&mut self, statement: crate::ast::Statement) {
        if !self.wasm_operand_stack.is_empty() {
            self.flush_operands_to_temporaries();
        }

        self.buffer.push(statement);
    }

    fn emit_statement(&mut self, statement: impl Into<crate::ast::Statement>) {
        self.emit_statement_inner(statement.into())
    }

    fn finish(self) -> (crate::context::CallConv<'a>, Definition) {
        let Self {
            wasm_operand_stack,
            buffer: body,
            ast_arena: arena,
            calling_convention,
        } = self;

        debug_assert!(wasm_operand_stack.is_empty());

        (calling_convention, Definition { body, arena })
    }
}

fn convert_impl<'wasm, 'types>(
    mut validator: wasmparser::FuncValidator<wasmparser::ValidatorResources>,
    body: wasmparser::FunctionBody<'wasm>,
    module: &'types crate::convert::Module<'wasm>,
    options: &crate::Convert<'_>,
    allocations: &crate::Allocations,
) -> crate::Result<(crate::context::CallConv<'types>, Definition)> {
    use anyhow::Context;

    let func_id = crate::ast::FuncId(validator.index());
    let func_type = module.types[module.types.core_function_at(func_id.0)].unwrap_func();

    // TODO: Reserve space in Vec<Statement>, collect data on avg. # of statements per byte of code
    let mut builder = StatementBuilder::new(allocations, func_type);

    {
        let mut locals = body
            .get_locals_reader()
            .context("could not obtain locals")?;

        let mut id = crate::ast::LocalId(func_type.params().len() as u32);
        for _ in 0..locals.get_count() {
            let (count, ty) = locals.read().context("could not read local variables")?;
            validator.define_locals(locals.original_position(), count, ty)?;
            for _ in 0..count {
                builder.emit_statement(crate::ast::Statement::LocalDefinition(id, ty.into()));
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
        let current_frame = validator
            .get_control_frame(0)
            .context("control frame stack was unexpectedly empty")?;

        if current_frame.unreachable && !matches!(op, Operator::End | Operator::Else) {
            // Although code is unreachable, WASM spec still requires it to be validated.
            validator.op(op_offset, &op)?;

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
                });
            }
            Operator::Nop => (),
            Operator::End => {
                if validator.control_stack_height() > 1 {
                    anyhow::bail!("TODO: block support not yet implemented");
                } else if !current_frame.unreachable {
                    let result_count = func_type.results().len();

                    debug_assert_eq!(
                        current_frame.height + result_count,
                        builder.wasm_operand_stack.len() - current_frame.height,
                        "value stack height mismatch"
                    );

                    let results = builder.wasm_operand_stack.drain(current_frame.height..);
                    debug_assert_eq!(result_count, results.as_slice().len());

                    // TODO: Fix, operands are EVALUATED in reverse order in generated code!
                    // Last result is popped first, so operands have to be in reverse order.
                    let result_exprs = builder.ast_arena.allocate_many(results.rev())?;

                    debug_assert_eq!(builder.wasm_operand_stack.len(), current_frame.height);

                    builder.emit_statement(crate::ast::Statement::Return(result_exprs));
                }
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
                });
            }
            Operator::I32Const { value } => {
                builder.push_wasm_operand(crate::ast::Literal::I32(value))?;
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

        validator.op(op_offset, &op)?;
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
