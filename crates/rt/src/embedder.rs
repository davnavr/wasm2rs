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
