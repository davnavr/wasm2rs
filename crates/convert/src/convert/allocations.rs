use crate::pool::Pool;

/// Allows reusing allocations between multiple conversions of WebAssembly code.
pub struct Allocations {
    func_validator_allocations: Pool<wasmparser::FuncValidatorAllocations>,
    ast_arenas: Pool<crate::ast::Arena>,
    statement_buffers: Pool<Vec<crate::ast::Statement>>,
    byte_buffers: crate::buffer::Pool,
}

impl Default for Allocations {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(missing_docs)]
impl Allocations {
    const fn new() -> Self {
        Self {
            func_validator_allocations: Pool::new(),
            ast_arenas: Pool::new(),
            statement_buffers: Pool::new(),
            byte_buffers: crate::buffer::Pool::new(),
        }
    }

    pub fn take_func_validator_allocations(&self) -> wasmparser::FuncValidatorAllocations {
        self.func_validator_allocations.pop().unwrap_or_default()
    }

    pub fn return_func_validator_allocations(
        &self,
        allocations: wasmparser::FuncValidatorAllocations,
    ) {
        self.func_validator_allocations.push(allocations)
    }

    pub(crate) fn take_ast_arena(&self) -> crate::ast::Arena {
        self.ast_arenas.pop().unwrap_or_default()
    }

    pub(crate) fn return_ast_arena(&self, arena: crate::ast::Arena) {
        self.ast_arenas.push(arena)
    }

    pub(crate) fn take_statement_buffer(&self) -> Vec<crate::ast::Statement> {
        self.statement_buffers.pop().unwrap_or_default()
    }

    pub(crate) fn return_statement_buffer(&self, mut buffer: Vec<crate::ast::Statement>) {
        if buffer.capacity() > 0 {
            buffer.clear();
            self.statement_buffers.push(buffer);
        }
    }

    #[allow(missing_docs)]
    pub fn byte_buffer_pool(&self) -> &crate::buffer::Pool {
        &self.byte_buffers
    }
}

impl std::fmt::Debug for Allocations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Allocations").finish_non_exhaustive()
    }
}
