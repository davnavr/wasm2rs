//! Default module used when embedding a WebAssembly module with no imports.

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
