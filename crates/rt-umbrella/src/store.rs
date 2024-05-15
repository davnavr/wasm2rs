//! Types and traits implementing the semantics of [WebAssembly allocation].
//!
//! [WebAssembly allocation]: https://webassembly.github.io/spec/core/exec/modules.html#allocation

#[allow(missing_docs)]
pub trait ModuleAllocation: core::ops::Deref {
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

/// Trait used for [allocating WebAssembly linear memories].
///
/// [allocating WebAssembly linear memories]: https://webassembly.github.io/spec/core/exec/modules.html#memories
pub trait AllocateMemory<I: crate::memory::Address = u32> {
    /// The linear memory instance.
    type Memory: crate::memory::Memory<I>;

    /// Allocates the linear memory, with the given minimum and maximum number of pages.
    fn allocate(
        self,
        minimum: I,
        maximum: I,
    ) -> Result<Self::Memory, crate::memory::AllocationError<I>>;
}

/// Implements the [`AllocateMemory`] trait by calling [`HeapMemory::<I>::with_limits()`].
///
/// [`HeapMemory::<I>::with_limits()`]: crate::memory::HeapMemory::with_limits();
#[derive(Clone, Copy, Default)]
#[cfg(feature = "alloc")]
pub struct AllocateHeapMemory<I: crate::memory::Address = u32> {
    _marker: core::marker::PhantomData<I>,
}

#[cfg(feature = "alloc")]
impl<I: crate::memory::Address> AllocateMemory<I> for AllocateHeapMemory<I> {
    type Memory = crate::memory::HeapMemory<I>;

    fn allocate(
        self,
        minimum: I,
        maximum: I,
    ) -> Result<Self::Memory, crate::memory::AllocationError<I>> {
        crate::memory::HeapMemory::<I>::with_limits(minimum, maximum)
    }
}

#[cfg(feature = "alloc")]
impl<I: crate::memory::Address> core::fmt::Debug for AllocateHeapMemory<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AllocateHeapMemory")
    }
}
