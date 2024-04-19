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
            Operator::I32Add => {
                // TODO: Macro for binop, are these popped in the correct order?
                let c_2 = builder.pop_wasm_operand();
                let c_1 = builder.pop_wasm_operand();
                builder.push_wasm_operand(crate::ast::Operator::Binary {
                    kind: crate::ast::BinOp::I32Add,
                    c_1,
                    c_2,
                })?;
            }
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
