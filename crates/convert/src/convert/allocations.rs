/// Allows reusing allocations between multiple conversions of WebAssembly code.
/// [`Convert`]
#[derive(Default)]
pub struct Allocations {
    func_validator_allocations: crossbeam_queue::SegQueue<wasmparser::FuncValidatorAllocations>,
    // TODO: Include buffers if needed.
}

#[allow(missing_docs)]
impl Allocations {
    pub fn take_func_validator_allocations(&self) -> wasmparser::FuncValidatorAllocations {
        self.func_validator_allocations.pop().unwrap_or_default()
    }

    pub fn return_func_validator_allocations(
        &self,
        allocations: wasmparser::FuncValidatorAllocations,
    ) {
        self.func_validator_allocations.push(allocations)
    }
}

impl std::fmt::Debug for Allocations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Allocations").finish_non_exhaustive()
    }
}
