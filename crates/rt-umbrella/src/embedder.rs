//! Provides modules re-exporting runtime support functionality all within a single module.

/// Used for embedding WebAssembly modules that do not have any imports.
///
/// This assumes:
/// - At most, one main linear memory with a 32-bit address space.
/// - At most one table containing [`FuncRef`]s.
/// - No imports of any kind.
///
/// [`FuncRef`]: crate::func_ref::FuncRef
#[cfg(feature = "alloc")]
#[allow(missing_docs)]
pub mod self_contained {
    pub use crate as rt;

    pub type Trap = crate::trap::TrapError;
    pub type Imports = ();
    pub type ExternRef = ();
    pub type Memory0 = crate::memory::HeapMemory;
    pub type Table0 = crate::table::HeapTable<crate::func_ref::FuncRef<'static, Trap>>;

    /// Contains all of state needed by the allocated WebAssembly module.
    #[derive(Debug, Default)]
    pub struct Store {
        /// The imports accessible by the WebAssembly module, of which there are none.
        pub imports: Imports, // TODO: Add a trait to initialize the imports, providing the `Module<T>` as an argument
        pub instance: crate::store::AllocateModuleRc,
        pub table0: crate::store::AllocateHeapTable<crate::func_ref::FuncRef<'static, Trap>>,
        /// Allocates the WebAssembly module's main memory.
        pub memory0: crate::store::AllocateHeapMemory,
    }

    pub type Module<T> = alloc::rc::Rc<T>;
}
