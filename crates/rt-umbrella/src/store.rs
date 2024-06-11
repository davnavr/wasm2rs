//! Types and traits implementing the semantics of [WebAssembly allocation].
//!
//! [WebAssembly allocation]: https://webassembly.github.io/spec/core/exec/modules.html#allocation

mod allocate_memory;
mod allocate_table;

pub use allocate_memory::{
    AllocateHeapMemory, AllocateMemory, AllocateMemoryError, ReuseExistingMemory,
};
pub use allocate_table::{AllocateHeapTable, AllocateTable, AllocateTableError};

/// Represents a WebAssembly [module instance].
///
/// [module instance]: https://webassembly.github.io/spec/core/exec/runtime.html#syntax-moduleinst
pub trait ModuleAllocation: Clone + core::ops::Deref {
    /// Gets a mutable reference to the underlying value.
    ///
    /// # Panics
    ///
    /// Panics if a mutable reference could not be obtained.
    fn get_mut(&mut self) -> &mut Self::Target;
}

#[cfg(feature = "alloc")]
impl<T> ModuleAllocation for alloc::rc::Rc<T> {
    fn get_mut(&mut self) -> &mut Self::Target {
        <Self>::get_mut(self).expect("value was shared")
    }
}

/// Trait for [allocating a WebAssembly module].
///
/// [allocating a WebAssembly module]: https://webassembly.github.io/spec/core/exec/modules.html#allocation
pub trait AllocateModule<T> {
    /// The module instance.
    type Module: ModuleAllocation<Target = T>;

    /// Allocates the module.
    fn allocate(self, instance: T) -> Self::Module; // Could return Result<Self::Module, AllocationError>
}

/// Implements the [`AllocateModule`] trait by calling [`Rc::new()`].
///
/// [`Rc::new()`]: alloc::rc::Rc::new()
#[derive(Clone, Copy, Debug, Default)]
#[cfg(feature = "alloc")]
pub struct AllocateModuleRc;

#[cfg(feature = "alloc")]
impl<T> AllocateModule<T> for AllocateModuleRc {
    type Module = alloc::rc::Rc<T>;

    fn allocate(self, instance: T) -> Self::Module {
        alloc::rc::Rc::new(instance)
    }
}
