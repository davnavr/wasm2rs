use crate::memory::{Address, AllocationError, Memory};
use crate::trap::Trap;

/// Error type used when a call to [`AllocateMemory::allocate()`] fails.
///
/// See the documentation for the [`AllocationError`] struct for more information.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocateMemoryError<I: Address> {
    memory: u32,
    error: Option<AllocationError<I>>,
}

impl<I: Address> AllocateMemoryError<I> {
    #[allow(missing_docs)]
    pub fn new(memory: u32, error: Option<AllocationError<I>>) -> Self {
        Self { memory, error }
    }

    /// Gets the index of the linear memory that could not be allocated.
    pub fn memory(&self) -> u32 {
        self.memory
    }
}

impl<I: Address> core::fmt::Display for AllocateMemoryError<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.error {
            Some(error) => write!(f, "memory #{} {error}", self.memory),
            None => write!(f, "could not allocate memory #{}", self.memory),
        }
    }
}

#[cfg(feature = "std")]
impl<I: Address> std::error::Error for AllocateMemoryError<I> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.as_ref().map(|e| e as _)
    }
}

impl From<AllocateMemoryError<u32>> for AllocateMemoryError<u64> {
    fn from(error: AllocateMemoryError<u32>) -> Self {
        Self {
            memory: error.memory,
            error: error.error.map(AllocationError::<u64>::from),
        }
    }
}

// impl<I: Address> From<AllocateMemoryError<I>> for AllocationError<I> {
//     fn from(error: AllocateMemoryError<I>) -> Self {
//         error.error
//     }
// }

/// Trait used for [allocating WebAssembly linear memories].
///
/// [allocating WebAssembly linear memories]: https://webassembly.github.io/spec/core/exec/modules.html#memories
pub trait AllocateMemory<I: Address = u32> {
    /// The linear memory instance.
    type Memory: Memory<I>;

    /// Allocates the linear memory, with the given minimum and maximum number of pages.
    fn allocate<E: Trap<AllocateMemoryError<I>>>(
        self,
        memory: u32,
        minimum: I,
        maximum: I,
    ) -> Result<Self::Memory, E>;
}

/// Implements the [`AllocateMemory`] trait by calling [`HeapMemory::<I>::with_limits()`].
///
/// [`HeapMemory::<I>::with_limits()`]: crate::memory::HeapMemory::with_limits();
#[derive(Clone, Copy, Default)]
#[cfg(feature = "alloc")]
pub struct AllocateHeapMemory<I: Address = u32> {
    _marker: core::marker::PhantomData<I>,
}

#[cfg(feature = "alloc")]
impl<I: Address> AllocateMemory<I> for AllocateHeapMemory<I> {
    type Memory = crate::memory::HeapMemory<I>;

    fn allocate<E: Trap<AllocateMemoryError<I>>>(
        self,
        memory: u32,
        minimum: I,
        maximum: I,
    ) -> Result<Self::Memory, E> {
        crate::memory::HeapMemory::<I>::with_limits(minimum, maximum)
            .map_err(|error| E::trap(AllocateMemoryError::new(memory, Some(error)), None))
    }
}

#[cfg(feature = "alloc")]
impl<I: Address> core::fmt::Debug for AllocateHeapMemory<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AllocateHeapMemory")
    }
}

/// Implements the [`AllocateMemory`] trait by returning an existing [`Memory`] instance.
#[derive(Clone, Copy)]
pub struct ReuseExistingMemory<M: Memory<I>, I: Address = u32> {
    memory: M,
    _marker: core::marker::PhantomData<fn(I)>,
}

impl<M: Memory<I>, I: Address> ReuseExistingMemory<M, I> {
    #[allow(missing_docs)]
    pub const fn new(memory: M) -> Self {
        Self {
            memory,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<M: Memory<I>, I: Address> AllocateMemory<I> for ReuseExistingMemory<M, I> {
    type Memory = M;

    fn allocate<E: Trap<AllocateMemoryError<I>>>(
        self,
        memory: u32,
        minimum: I,
        maximum: I,
    ) -> Result<M, E> {
        match crate::memory::check_limits::<I, crate::trap::TrapOccurred, M>(
            &self.memory,
            memory,
            minimum,
            maximum,
        ) {
            Ok(()) => Ok(self.memory),
            Err(crate::trap::TrapOccurred) => {
                Err(E::trap(AllocateMemoryError::new(memory, None), None))
            } // TODO: Would be nice to explain why reused memory was wrong
        }
    }
}

impl<M: Memory<I> + core::fmt::Debug, I: Address> core::fmt::Debug for ReuseExistingMemory<M, I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ReuseExistingMemory")
            .field(&self.memory)
            .finish()
    }
}
