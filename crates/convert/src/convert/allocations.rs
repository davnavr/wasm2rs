#[cfg(feature = "crossbeam-queue")]
type Pool<T> = crossbeam_queue::SegQueue<T>; // TODO: Use Mutex<Vec<T>> instead, have thread_local::ThreadLocal<Vec<T>> alongside

#[cfg(not(feature = "crossbeam-queue"))]
struct Pool<T> {
    pool: std::cell::RefCell<Vec<T>>,
}

#[cfg(not(feature = "crossbeam-queue"))]
impl<T> Pool<T> {
    const fn new() -> Self {
        Self {
            pool: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn pop(&self) -> Option<T> {
        self.pool.borrow_mut().pop()
    }

    fn push(&self, value: T) {
        self.pool.borrow_mut().push(value)
    }
}

/// Allows reusing allocations between multiple conversions of WebAssembly code.
pub struct Allocations {
    func_validator_allocations: Pool<wasmparser::FuncValidatorAllocations>,
    ast_arenas: Pool<crate::ast::Arena>,
    statement_buffers: Pool<Vec<crate::ast::Statement>>,
    // TODO: Include buffers if needed.
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
}

impl std::fmt::Debug for Allocations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Allocations").finish_non_exhaustive()
    }
}
