#[derive(Debug)]
#[must_use]
pub(in crate::convert::code) struct Builder<'a> {
    wasm_operand_stack: Vec<crate::ast::ExprId>,
    spilled_wasm_operands: usize,
    buffer: Vec<crate::ast::Statement>,
    ast_arena: crate::ast::Arena,
    calling_convention: crate::context::CallConv<'a>,
}

impl<'a> Builder<'a> {
    pub(super) fn new(
        allocations: &crate::Allocations,
        wasm_signature: &'a wasmparser::FuncType,
    ) -> Self {
        // TODO: Value stack should be taken from `allocations`.
        Self {
            // TODO: Reserve space in Vec<ExprId>, collect data on avg. max stack height
            wasm_operand_stack: Vec::new(),
            spilled_wasm_operands: 0,
            buffer: allocations.take_statement_buffer(),
            ast_arena: allocations.take_ast_arena(),
            calling_convention: crate::context::CallConv {
                call_kind: crate::context::CallKind::Function,
                can_trap: false,
                wasm_signature,
            },
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

        // TODO: Fix, operands are EVALUATED in reverse order in generated code!
        // Last result is popped first, so operands have to be in reverse order.
        let result_exprs = self.ast_arena.allocate_many(results.rev())?;

        debug_assert_eq!(self.wasm_operand_stack.len(), height);

        self.fix_spilled_wasm_operands();
        Ok(result_exprs)
    }

    pub(super) fn wasm_operand_stack_pop_list(
        &mut self,
        count: usize,
    ) -> crate::Result<crate::ast::ExprListId> {
        self.wasm_operand_stack_pop_to_height(self.wasm_operand_stack.len() - count)
    }

    pub(super) fn can_trap(&mut self) {
        self.calling_convention.can_trap = true;
    }

    pub(super) fn needs_self(&mut self) {
        self.calling_convention.call_kind = crate::context::CallKind::Method;
    }

    pub(super) fn push_wasm_operand(
        &mut self,
        operand: impl Into<crate::ast::Expr>,
    ) -> crate::Result<()> {
        Ok(self
            .wasm_operand_stack
            .push(self.ast_arena.allocate(operand)?))
    }

    pub(super) fn pop_wasm_operand(&mut self) -> crate::ast::ExprId {
        let popped = self.wasm_operand_stack.pop().unwrap();
        self.fix_spilled_wasm_operands();
        popped
    }

    pub(super) fn flush_operands_to_temporaries(&mut self) -> crate::Result<()> {
        for (i, value) in self.wasm_operand_stack[self.spilled_wasm_operands..]
            .iter_mut()
            .enumerate()
        {
            let id = crate::ast::TempId((i + self.spilled_wasm_operands) as u32);
            self.buffer
                .push(crate::ast::Statement::Temporary(id, *value));
            *value = self.ast_arena.allocate(crate::ast::Expr::Temporary(id))?;
        }

        self.spilled_wasm_operands = self.wasm_operand_stack.len();
        Ok(())
    }

    fn emit_statement_inner(&mut self, statement: crate::ast::Statement) -> crate::Result<()> {
        if !self.wasm_operand_stack.is_empty() {
            self.flush_operands_to_temporaries()?;
        }

        self.buffer.push(statement);
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
        crate::context::CallConv<'a>,
        crate::convert::code::Definition,
    ) {
        let Self {
            wasm_operand_stack,
            buffer: body,
            ast_arena: arena,
            calling_convention,
            spilled_wasm_operands: _,
        } = self;

        debug_assert!(wasm_operand_stack.is_empty());

        (
            calling_convention,
            crate::convert::code::Definition { body, arena },
        )
    }
}
