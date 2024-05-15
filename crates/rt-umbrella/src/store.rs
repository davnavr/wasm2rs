//! Types and traits implementing the semantics of [WebAssembly allocation].
//!
//! [WebAssembly allocation]: https://webassembly.github.io/spec/core/exec/modules.html#allocation

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
