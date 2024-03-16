/// A [`Memory32`] implementation backed by a heap allcation.
///
/// [`Memory32`]: crate::memory::Memory32
pub struct HeapMemory32 {
    allocation: core::cell::Cell<crate::memory::allocation::Memory>,
    /// Maximum number of allowed pages.
    limit: u32,
}

impl Default for HeapMemory32 {
    fn default() -> Self {
        Self::new()
    }
}

impl HeapMemory32 {
    /// Allocates an empty linear memory with a maximum number of allowed pages.
    pub const fn with_maximum(maximum: u32) -> Self {
        Self {
            allocation: core::cell::Cell::new(crate::memory::allocation::Memory::EMPTY),
            limit: maximum,
        }
    }

    /// Allocates an empty linear memory.
    pub const fn new() -> Self {
        Self::with_maximum(u32::MAX)
    }

    fn modify<R>(&self, f: impl FnOnce(&mut crate::memory::allocation::Memory) -> R) -> R {
        crate::memory::allocation::Memory::modify(&self.allocation, f)
    }

    /// Returns the size of the linear memory, in bytes.
    pub fn len(&self) -> usize {
        self.modify(|a| a.len())
    }

    /// Returns `true` if the memory has a size of `0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Allocates a linear memory, with a minimum and maximum number of pages.
    ///
    /// If the `minimum` is greater than `0`, then new pages are allocated.
    pub fn with_limits(minimum: u32, maximum: u32) -> Result<Self, crate::memory::AllocationError> {
        let mut mem = Self::with_maximum(maximum);
        match mem.allocation.get_mut().grow(minimum) {
            Some(_) => Ok(mem),
            None => Err(crate::memory::AllocationError { size: minimum }),
        }
    }

    /// Returns a mutable slice to the linear memory contents.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.allocation.get_mut().as_mut_slice()
    }

    fn modify_addresses<R>(
        &self,
        addr: u32,
        len: usize,
        f: impl FnOnce(&mut [u8]) -> R,
    ) -> crate::memory::AccessResult<R> {
        fn out_of_bounds(addr: u32, len: usize) -> crate::memory::MemoryAccessError {
            crate::memory::MemoryAccessError {
                address: addr,
                pointee: crate::memory::MemoryAccessPointee::Other {
                    size: u16::try_from(len).ok().and_then(core::num::NonZeroU16::new),
                },
            }
        }

        self.modify(move |a| {
            let start_addr = addr as usize;
            match a
                .as_mut_slice()
                .get_mut(start_addr..start_addr.wrapping_add(len))
            {
                Some(slice) => Ok(f(slice)),
                None => Err(out_of_bounds(addr, len)),
            }
        })
    }
}

impl crate::memory::Memory32 for HeapMemory32 {
    fn limit(&self) -> u32 {
        self.limit
    }

    fn size(&self) -> i32 {
        self.modify(|a| a.size()) as i32
    }

    fn grow(&self, delta: i32) -> i32 {
        self.modify(|a| match a.grow(delta as u32) {
            Some(old) => old as i32,
            None => crate::memory::MEMORY_GROW_FAILED,
        })
    }

    fn copy_to_slice(&self, addr: u32, dst: &mut [u8]) -> crate::memory::AccessResult<()> {
        self.modify_addresses(addr, dst.len(), |slice| dst.copy_from_slice(slice))
    }

    fn copy_from_slice(&self, addr: u32, src: &[u8]) -> crate::memory::AccessResult<()> {
        self.modify_addresses(addr, src.len(), |slice| slice.copy_from_slice(src))
    }
}

impl core::fmt::Debug for HeapMemory32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.modify(move |a| {
            f.debug_struct("HeapMemory32")
                .field("allocation", a)
                .field("limit", &crate::memory::DisplaySize(self.limit))
                .finish_non_exhaustive()
        })
    }
}
