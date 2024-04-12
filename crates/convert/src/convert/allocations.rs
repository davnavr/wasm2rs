use crossbeam_queue::SegQueue;

/// Allows reusing allocations between multiple conversions of WebAssembly code.
pub struct Allocations {
    func_validator_allocations: SegQueue<wasmparser::FuncValidatorAllocations>,
    ast_arenas: SegQueue<crate::ast::Arena>,
    // TODO: Include buffers if needed.
}

impl Default for Allocations {
    fn default() -> Self {
        Self::default()
    }
}

#[allow(missing_docs)]
impl Allocations {
    const fn new() -> Self {
        Self {
            func_validator_allocations: SegQueue::new(),
            ast_arenas: SegQueue::new(),
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
}

impl std::fmt::Debug for Allocations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Allocations").finish_non_exhaustive()
    }
}
