#[derive(Debug)]
#[must_use]
pub(in crate::convert::code) struct Builder<'a> {
    wasm_operand_stack: Vec<crate::ast::ExprId>,
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
            buffer: allocations.take_statement_buffer(),
            ast_arena: allocations.take_ast_arena(),
            calling_convention: crate::context::CallConv {
                call_kind: crate::context::CallKind::Function,
                can_trap: false,
                wasm_signature,
            },
        }
    }

    pub(super) fn wasm_operand_stack(&self) -> &[crate::ast::ExprId] {
        &self.wasm_operand_stack
    }

    pub(super) fn wasm_operand_stack_truncate(&mut self, height: usize) {
        if !self.wasm_operand_stack.is_empty() {
            self.flush_operands_to_temporaries();
        }

        self.wasm_operand_stack.truncate(height);
    }

    pub(super) fn wasm_operand_stack_pop_to_height(
        &mut self,
        height: usize,
    ) -> Result<crate::ast::ExprListId, crate::ast::ArenaError> {
        let results = self.wasm_operand_stack.drain(height..);

        // TODO: Fix, operands are EVALUATED in reverse order in generated code!
        // Last result is popped first, so operands have to be in reverse order.
        let result_exprs = self.ast_arena.allocate_many(results.rev())?;

        debug_assert_eq!(self.wasm_operand_stack.len(), height);

        Ok(result_exprs)
    }

    pub(super) fn wasm_operand_stack_pop_list(
        &mut self,
        count: usize,
    ) -> Result<crate::ast::ExprListId, crate::ast::ArenaError> {
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
    ) -> Result<(), crate::ast::ArenaError> {
        self.ast_arena
            .allocate(operand)
            .map(|expr| self.wasm_operand_stack.push(expr))
    }

    pub(super) fn pop_wasm_operand(&mut self) -> crate::ast::ExprId {
        // Stack underflows are handled later by a `FuncValidator`, so this can't panic.
        if let Some(expr) = self.wasm_operand_stack.pop() {
            expr
        } else {
            todo!("special Expr value for operand stack underflow bugs")
        }
    }

    pub(super) fn flush_operands_to_temporaries(&mut self) {
        todo!("flushing of wasm operand stack to temporaries not yet implemented");
        // TODO: iter_mut for self.wasm_operand_stack, write temporaries
        // TODO: have a height value to keep track of the temporaries that were already spilled
    }

    fn emit_statement_inner(&mut self, statement: crate::ast::Statement) {
        if !self.wasm_operand_stack.is_empty() {
            self.flush_operands_to_temporaries();
        }

        self.buffer.push(statement);
    }

    pub(super) fn emit_statement(&mut self, statement: impl Into<crate::ast::Statement>) {
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
        } = self;

        debug_assert!(wasm_operand_stack.is_empty());

        (
            calling_convention,
            crate::convert::code::Definition { body, arena },
        )
    }
}
