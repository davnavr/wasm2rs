#[derive(Debug)]
#[must_use]
pub(in crate::convert::code) struct Builder {
    wasm_operand_stack: Vec<crate::ast::ExprId>,
    spilled_wasm_operands: usize,
    buffer: Vec<crate::ast::Statement>,
    ast_arena: crate::ast::Arena,
    attributes: crate::convert::code::Attributes,
    has_return: bool,
}

impl Builder {
    pub(super) fn new(allocations: &crate::Allocations) -> Self {
        // TODO: Value stack should be taken from `allocations`.
        Self {
            // TODO: Reserve space in Vec<ExprId>, collect data on avg. max stack height
            wasm_operand_stack: Vec::new(),
            spilled_wasm_operands: 0,
            buffer: allocations.take_statement_buffer(),
            ast_arena: allocations.take_ast_arena(),
            attributes: crate::convert::code::Attributes {
                call_kind: crate::context::CallKind::Function,
                unwind_kind: crate::context::UnwindKind::Never,
            },
            has_return: false,
        }
    }

    fn fix_spilled_wasm_operands(&mut self) {
        self.spilled_wasm_operands = self
            .spilled_wasm_operands
            .min(self.wasm_operand_stack.len());
    }

    pub(super) fn wasm_operand_stack(&self) -> &[crate::ast::ExprId] {
        &self.wasm_operand_stack
    }

    /// Given `count` operands used as inputs into the given `loop`, this stores all of the
    /// corresponding input [`Expr`]s into temporary local variables. Returns the list of
    /// expressions corresponding to the initial inputs of the `loop`.
    ///
    /// This essentially replaces the top `count` [`Expr`]s on the stack with [`Expr::LoopInput`].
    ///
    /// [`Expr`]: crate::ast::Expr
    /// [`Expr::LoopInput`]: crate::ast::Expr::LoopInput
    pub(super) fn wasm_operand_stack_move_loop_inputs(
        &mut self,
        r#loop: crate::ast::BlockId,
        count: usize,
    ) -> crate::Result<crate::ast::ExprListId> {
        let operands_stack_height = self.wasm_operand_stack.len();
        if cfg!(debug_assertions) && count > operands_stack_height {
            anyhow::bail!(
                "cannot move {count} loop inputs, operand stack height is {operands_stack_height}"
            );
        }

        // Prevent loop inputs from being flushed.
        self.spilled_wasm_operands += count;
        self.flush_operands_to_temporaries()?;

        let to_move = &mut self.wasm_operand_stack[operands_stack_height - count..];
        let initial_inputs = self.ast_arena.allocate_many(to_move.iter().copied())?;

        for (op, number) in to_move.iter_mut().zip(0u32..=u32::MAX) {
            *op = self
                .ast_arena
                .allocate(crate::ast::Expr::LoopInput(crate::ast::LoopInput {
                    r#loop,
                    number,
                }))?;
        }

        Ok(initial_inputs)
    }

    /// Flushes operands and returns a list containing the top `count` values in the operand stack,
    /// **without** popping them.
    ///
    /// This is used when translating the `br_if` instruction.
    pub(super) fn wasm_operand_stack_duplicate_many(
        &mut self,
        count: usize,
    ) -> crate::Result<crate::ast::ExprListId> {
        let operands_stack_height = self.wasm_operand_stack.len();
        if cfg!(debug_assertions) && count > operands_stack_height {
            anyhow::bail!(
                "cannot duplicate {count} values, operand stack height is {operands_stack_height}"
            );
        }

        self.flush_operands_to_temporaries()?;

        Ok(self.ast_arena.allocate_many(
            self.wasm_operand_stack[operands_stack_height - count..]
                .iter()
                .copied(),
        )?)
    }

    // Pops values from the operand stack until it has the given `height`.
    pub(super) fn wasm_operand_stack_truncate(&mut self, height: usize) -> crate::Result<()> {
        if !self.wasm_operand_stack.is_empty() {
            self.flush_operands_to_temporaries()?;
        }

        self.wasm_operand_stack.truncate(height);
        self.fix_spilled_wasm_operands();
        Ok(())
    }

    pub(super) fn wasm_operand_stack_pop_to_height(
        &mut self,
        height: usize,
    ) -> crate::Result<crate::ast::ExprListId> {
        let results = self.wasm_operand_stack.drain(height..);

        // Last result is popped first
        let result_exprs = self.ast_arena.allocate_many(results)?;

        debug_assert_eq!(self.wasm_operand_stack.len(), height);

        self.fix_spilled_wasm_operands();
        Ok(result_exprs)
    }

    pub(super) fn wasm_operand_stack_pop_list(
        &mut self,
        count: usize,
    ) -> crate::Result<crate::ast::ExprListId> {
        if cfg!(debug_assertions) && count > self.wasm_operand_stack.len() {
            anyhow::bail!(
                "attempted to pop {count} values, but operand stack contained {} ({:?})",
                self.wasm_operand_stack.len(),
                self.wasm_operand_stack
            );
        }

        self.wasm_operand_stack_pop_to_height(self.wasm_operand_stack.len() - count)
    }

    pub(super) fn can_trap(&mut self) {
        self.attributes.unwind_kind = crate::context::UnwindKind::Maybe;
    }

    pub(super) fn needs_self(&mut self) {
        self.attributes.call_kind = crate::context::CallKind::Method;
    }

    pub(super) fn push_wasm_operand(
        &mut self,
        operand: impl Into<crate::ast::Expr>,
    ) -> crate::Result<()> {
        self.wasm_operand_stack
            .push(self.ast_arena.allocate(operand)?);
        Ok(())
    }

    pub(super) fn pop_wasm_operand(&mut self) -> crate::ast::ExprId {
        let popped = self.wasm_operand_stack.pop().unwrap();
        self.fix_spilled_wasm_operands();
        popped
    }

    pub(super) fn get_block_results(
        &self,
        result_count: usize,
        input_count: usize,
    ) -> crate::Result<Option<crate::ast::BlockResults>> {
        debug_assert!(
            input_count <= self.wasm_operand_stack.len(),
            "expected block to pop {input_count} inputs, but operand stack contained {} values ({:?})",
            self.wasm_operand_stack.len(),
            self.wasm_operand_stack
        );

        Ok(
            std::num::NonZeroU32::new(result_count as u32).map(|count| crate::ast::BlockResults {
                start: crate::ast::TempId((self.spilled_wasm_operands - input_count) as u32),
                count,
            }),
        )
    }

    pub(super) fn push_block_results(&mut self, count: usize) -> crate::Result<()> {
        let current_height = self.wasm_operand_stack.len();
        debug_assert_eq!(current_height, self.spilled_wasm_operands);

        self.wasm_operand_stack.reserve(count);
        for i in 0..count {
            self.push_wasm_operand(crate::ast::Expr::Temporary(crate::ast::TempId(
                (current_height + i) as u32,
            )))?;
        }

        self.spilled_wasm_operands += count;
        Ok(())
    }

    pub(super) fn flush_operands_to_temporaries(&mut self) -> crate::Result<()> {
        // Could have argument indicate # of operands to preserve (e.g. block arguments), but this
        // works fine as is.

        for (i, value) in self.wasm_operand_stack[self.spilled_wasm_operands..]
            .iter_mut()
            .enumerate()
        {
            if !matches!(self.ast_arena.get(*value), crate::ast::Expr::Temporary(_)) {
                let id = crate::ast::TempId((i + self.spilled_wasm_operands) as u32);
                self.buffer.push(crate::ast::Statement::Temporary {
                    temporary: id,
                    value: *value,
                });
                *value = self.ast_arena.allocate(crate::ast::Expr::Temporary(id))?;
            }
        }

        self.spilled_wasm_operands = self.wasm_operand_stack.len();
        Ok(())
    }

    fn emit_statement_inner(&mut self, statement: crate::ast::Statement) -> crate::Result<()> {
        if !self.wasm_operand_stack.is_empty() {
            self.flush_operands_to_temporaries()?;
        }

        self.buffer.push(statement);

        if matches!(
            statement,
            crate::ast::Statement::Branch {
                target: crate::ast::BranchTarget::Return,
                ..
            }
        ) {
            self.has_return = true;
        }

        Ok(())
    }

    pub(super) fn emit_statement(
        &mut self,
        statement: impl Into<crate::ast::Statement>,
    ) -> crate::Result<()> {
        self.emit_statement_inner(statement.into())
    }

    pub(super) fn finish(
        self,
    ) -> (
        crate::convert::code::Attributes,
        crate::convert::code::Definition,
    ) {
        let Self {
            wasm_operand_stack,
            buffer: body,
            ast_arena: arena,
            mut attributes,
            spilled_wasm_operands: _,
            has_return,
        } = self;

        debug_assert!(wasm_operand_stack.is_empty());

        if !has_return && matches!(attributes.unwind_kind, crate::context::UnwindKind::Maybe) {
            // Function does not return normally.
            attributes.unwind_kind = crate::context::UnwindKind::Always;
        }

        (attributes, crate::convert::code::Definition { body, arena })
    }
}
