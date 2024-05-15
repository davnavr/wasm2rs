//! Provides modules re-exporting runtime support functionality all within a single module.

/// Used for embedding WebAssembly modules that do not have any imports.
///
/// This assumes:
/// - At most, one main linear memory with a 32-bit address space.
/// - No imports of any kind.
#[cfg(feature = "alloc")]
pub mod self_contained {
    pub use crate as rt;

    //pub type Trap

    /// The imports accessible by the WebAssembly module, of which there are none.
    pub type Imports = ();

    /// Contains all of state needed by the allocated WebAssembly module.
    #[allow(missing_docs)]
    #[derive(Debug, Default)]
    pub struct Store {
        pub imports: Imports,
        /// Allocates the WebAssembly module's main memory.
        pub memory0: crate::store::AllocateHeapMemory,
    }

    /// The type used to contain the WebAssembly module state.
    pub type Module<T> = alloc::rc::Rc<T>;
}
