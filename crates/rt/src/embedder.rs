//! Default module used when embedding a WebAssembly module with no imports.

use crate::trap::{Trap, TrapCode};

pub use crate as rt;

/// The default memory implementation to use for the WebAssembly module's main memory.
#[cfg(feature = "alloc")]
pub type Memory0 = crate::memory::HeapMemory32;

/// An empty memory implementation to use for the WebAssembly module's main memory.
///
/// If the `alloc` feature is enabled, then heap allocations are used instead.
#[cfg(not(feature = "alloc"))]
pub type Memory0 = crate::memory::EmptyMemory;

/// Type used for the result of WebAssembly computations.
///
/// An `Err` indicates that a trap has occured.
pub type Result<T> = ::core::result::Result<T, crate::trap::TrapValue>;

/// The default embedder state.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct State;

impl State {
    /// Initializes the WebAssembly module's main memory.
    pub fn initialize_mem_0<const IDX: u32, const MIN: u32, const MAX: u32>(
        &self,
    ) -> Result<Memory0> {
        #[cfg(not(feature = "alloc"))]
        return Err(self.trap(TrapCode::MemoryInstantiation {
            memory: IDX,
            error: crate::memory::AllocationError::with_size(MIN),
        }));

        #[cfg(feature = "alloc")]
        return Memory0::with_limits(MIN, MAX)
            .map_err(|error| self.trap(TrapCode::MemoryInstantiation { memory: IDX, error }));
    }
}

impl Trap for State {
    type Repr = crate::trap::TrapValue;

    #[inline(never)]
    fn trap(&self, code: TrapCode) -> Self::Repr {
        <Self::Repr>::new(code)
    }
}

// TODO: Helper macro to make a new embedder module
