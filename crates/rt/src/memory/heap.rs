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
            None => Err(crate::memory::AllocationError::with_size(minimum)),
        }
    }

    /// Returns a mutable slice to the linear memory contents.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.allocation.get_mut().as_mut_slice()
    }

    fn modify_addresses<T, E>(
        &self,
        addr: u32,
        len: usize,
        op: impl FnOnce(&mut [u8]) -> Result<T, E>,
        err: E,
    ) -> Result<T, E> {
        self.modify(move |a| {
            let start_addr = addr as usize;
            match a
                .as_mut_slice()
                .get_mut(start_addr..start_addr.wrapping_add(len))
            {
                Some(slice) => op(slice),
                None => Err(err),
            }
        })
    }
}

impl crate::memory::Memory32 for HeapMemory32 {
    fn try_as_any(&self, _: crate::memory::private::Hidden) -> Option<&dyn core::any::Any> {
        Some(self)
    }

    fn limit(&self) -> u32 {
        self.limit
    }

    fn size(&self) -> u32 {
        self.modify(|a| a.size())
    }

    fn grow(&self, delta: u32) -> u32 {
        self.modify(|a| match a.grow(delta) {
            Some(old) => old,
            None => crate::memory::MEMORY_GROW_FAILED,
        })
    }

    fn copy_to_slice(&self, addr: u32, dst: &mut [u8]) -> crate::memory::BoundsCheck<()> {
        self.modify_addresses(
            addr,
            dst.len(),
            |slice| {
                dst.copy_from_slice(slice);
                Ok(())
            },
            crate::memory::BoundsCheckError,
        )
    }

    fn copy_from_slice(&self, addr: u32, src: &[u8]) -> crate::memory::BoundsCheck<()> {
        self.modify_addresses(
            addr,
            src.len(),
            |slice| {
                slice.copy_from_slice(src);
                Ok(())
            },
            crate::memory::BoundsCheckError,
        )
    }

    fn copy_within(
        &self,
        dst_addr: u32,
        src_addr: u32,
        len: u32,
    ) -> crate::memory::BoundsCheck<()> {
        self.modify(|mem| {
            let dst_index = dst_addr as usize;
            let src_index = src_addr as usize;
            let size = len as usize;
            let slice = mem.as_mut_slice();

            // Check that the source is in bounds.
            let src = src_index..(src_index.checked_add(size)?);
            let _ = slice.get(src.clone())?;

            // Check that the destination is also in bounds.
            let dst = dst_index..(dst_index.checked_add(size)?);
            let _ = slice.get(dst)?;

            slice.copy_within(src, dst_index);
            Some(())
        })
        .ok_or(crate::memory::BoundsCheckError)
    }

    fn copy_from<Src>(
        &self,
        src: &Src,
        dst_addr: u32,
        src_addr: u32,
        len: u32,
    ) -> crate::memory::BoundsCheck<()>
    where
        Src: crate::memory::Memory32 + ?Sized,
    {
        // Workaround for specialization being unstable in the current version of Rust.
        // For where `Src` is statically known, this should be optimized.
        if let Some(src) = src.try_as_any(crate::memory::private::Hidden) {
            if let Some(src) = src.downcast_ref::<Self>() {
                // Common case breaks if `self` and `src` are the same memory.
                if core::ptr::eq::<Self>(self, src) {
                    return crate::memory::Memory32::copy_within(self, dst_addr, src_addr, len);
                }

                // Fallthrough
            } else if src.is::<crate::memory::EmptyMemory>() {
                return if dst_addr == 0 && len == 0 && (src_addr as usize) <= self.len() {
                    Ok(())
                } else {
                    Err(crate::memory::BoundsCheckError)
                };
            }

            // Fallthrough to the common case.
        }

        self.modify_addresses(
            dst_addr,
            len as usize,
            |dst| src.copy_to_slice(src_addr, dst),
            crate::memory::BoundsCheckError,
        )
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
