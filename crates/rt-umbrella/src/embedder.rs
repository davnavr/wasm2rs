//! Provides modules re-exporting runtime support functionality all within a single module.

/// Used for embedding WebAssembly modules that do not have any imports.
///
/// This assumes:
/// - At most, one main linear memory with a 32-bit address space.
/// - No imports of any kind.
#[cfg(feature = "alloc")]
#[allow(missing_docs)]
pub mod self_contained {
    pub use crate as rt;

    pub type Trap = crate::trap::TrapError;

    pub type Imports = ();

    /// Contains all of state needed by the allocated WebAssembly module.
    #[derive(Debug, Default)]
    pub struct Store {
        /// The imports accessible by the WebAssembly module, of which there are none.
        pub imports: Imports, // TODO: Add a trait to initialize the imports, providing the `Module<T>` as an argument
        pub instance: crate::store::AllocateModuleRc,
        /// Allocates the WebAssembly module's main memory.
        pub memory0: crate::store::AllocateHeapMemory,
    }

    pub type Module<T> = alloc::rc::Rc<T>;
}
