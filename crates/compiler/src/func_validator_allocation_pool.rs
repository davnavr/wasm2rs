/// Allows reusing allocations between [`FuncValidator`].
///
/// [`FuncValidator`]: wasmparser::FuncValidator
#[derive(Default)]
pub struct FuncValidatorAllocationPool {
    pool: crossbeam_queue::SegQueue<wasmparser::FuncValidatorAllocations>,
}

impl FuncValidatorAllocationPool {
    #[allow(missing_docs)]
    pub fn take_allocations(&self) -> wasmparser::FuncValidatorAllocations {
        self.pool.pop().unwrap_or_default()
    }

    #[allow(missing_docs)]
    pub fn return_allocations(&self, allocations: wasmparser::FuncValidatorAllocations) {
        self.pool.push(allocations)
    }
}

impl std::fmt::Debug for FuncValidatorAllocationPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncValidatorAllocationPool")
            .finish_non_exhaustive()
    }
}
