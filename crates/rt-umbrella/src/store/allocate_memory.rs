use crate::memory::Address;

/// Error type used when a call to [`AllocateMemory::allocate()`] fails.
///
/// See also the [`memory::AllocationError`] struct.
///
/// [`memory::AllocationError`]: crate::memory::AllocationError
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocateMemoryError<I: Address> {
    memory: u32,
    error: crate::memory::AllocationError<I>,
}

impl<I: Address> AllocateMemoryError<I> {
    #[allow(missing_docs)]
    pub fn new(memory: u32, error: crate::memory::AllocationError<I>) -> Self {
        Self { memory, error }
    }

    /// Gets the index of the linear memory that could not be allocated.
    pub fn memory(&self) -> u32 {
        self.memory
    }
}

impl<I: Address> core::fmt::Display for AllocateMemoryError<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "memory #{} {}", self.memory, self.error)
    }
}

#[cfg(feature = "std")]
impl<I: Address> std::error::Error for AllocateMemoryError<I> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl From<AllocateMemoryError<u32>> for AllocateMemoryError<u64> {
    fn from(error: AllocateMemoryError<u32>) -> Self {
        Self {
            memory: error.memory,
            error: crate::memory::AllocationError::<u64>::from(error.error),
        }
    }
}

impl<I: Address> From<AllocateMemoryError<I>> for crate::memory::AllocationError<I> {
    fn from(error: AllocateMemoryError<I>) -> Self {
        error.error
    }
}

/// Trait used for [allocating WebAssembly linear memories].
///
/// [allocating WebAssembly linear memories]: https://webassembly.github.io/spec/core/exec/modules.html#memories
pub trait AllocateMemory<I: crate::memory::Address = u32> {
    /// The linear memory instance.
    type Memory: crate::memory::Memory<I>;

    /// Allocates the linear memory, with the given minimum and maximum number of pages.
    fn allocate<E: crate::trap::Trap<AllocateMemoryError<I>>>(
        self,
        memory: u32,
        minimum: I,
        maximum: I,
        frame: Option<&'static crate::trace::WasmFrame>,
    ) -> Result<Self::Memory, E>;
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

    fn allocate<E: crate::trap::Trap<AllocateMemoryError<I>>>(
        self,
        memory: u32,
        minimum: I,
        maximum: I,
        frame: Option<&'static crate::trace::WasmFrame>,
    ) -> Result<Self::Memory, E> {
        crate::memory::HeapMemory::<I>::with_limits(minimum, maximum)
            .map_err(|error| E::trap(AllocateMemoryError::new(memory, error), frame))
    }
}

#[cfg(feature = "alloc")]
impl<I: crate::memory::Address> core::fmt::Debug for AllocateHeapMemory<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AllocateHeapMemory")
    }
}
