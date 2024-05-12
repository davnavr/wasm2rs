use crate::{Address, Memory};
use core::{cell::Cell, ptr::NonNull};

/// A [`Memory`] implementation backed by a heap allcation.
///
/// [`Memory`]: crate::Memory
pub struct HeapMemory<I: Address = u32> {
    allocation: Cell<NonNull<Cell<u8>>>,
    /// The length of the linear memory, in bytes.
    ///
    /// This is guaranteed to always be a multiple of the [`crate::PAGE_SIZE`].
    len: Cell<I>,
    /// The maximum number of pages this linear memory can have.
    limit: I,
}

impl<I: Address> Default for HeapMemory<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: Address> HeapMemory<I> {
    /// The alignment to use for the underlying heap allocation.
    ///
    /// This is currently set to 4 KiB, the page size used in most operating systems.
    const ALIGNMENT: usize = 0x1000;

    /// Allocates an empty linear memory.
    pub const fn new() -> Self {
        Self {
            allocation: Cell::new(NonNull::dangling()),
            len: Cell::new(I::ZERO),
            limit: I::MAX_PAGE_COUNT,
        }
    }

    /// Allocates an empty linear memory with a maximum number of allowed pages.
    ///
    /// # Errors
    ///
    /// Returns an error if the `limit` is greater than the [`I::MAX_PAGE_COUNT`].
    ///
    /// [`I::MAX_PAGE_COUNT`]: Address::MAX_PAGE_COUNT
    pub fn with_maximum(limit: I) -> Result<Self, crate::AllocationError<I>> {
        if limit <= I::MAX_PAGE_COUNT {
            let mut mem = Self::new();
            mem.limit = limit;
            Ok(mem)
        } else {
            Err(crate::AllocationError { size: limit })
        }
    }

    /// Allocates a linear memory, with a minimum and maximum number of pages.
    ///
    /// If the `minimum` is greater than `0`, then new pages are allocated.
    ///
    /// # Errors
    ///
    /// Returns an error if the `minimum` number of pages could not be allocated, or if the
    /// `maximum` is greater than the [`I::MAX_PAGE_COUNT`].
    ///
    /// [`I::MAX_PAGE_COUNT`]: Address::MAX_PAGE_COUNT
    pub fn with_limits(minimum: I, maximum: I) -> Result<Self, crate::AllocationError<I>> {
        let mem = Self::with_maximum(maximum)?;
        mem.try_grow(minimum)?;
        Ok(mem)
    }

    /// Returns the size of the linear memory, in bytes.
    pub fn len(&self) -> usize {
        self.len.get().as_()
    }

    /// Returns `true` if the memory has a size of `0`.
    pub fn is_empty(&self) -> bool {
        self.len.get() == I::ZERO
    }

    /// Attempts to increase the size of the linear memory by the given number of pages.
    ///
    /// # Errors
    ///
    /// Returns an error if the pages could not be allocated.
    pub fn try_grow(&self, delta: I) -> Result<(), crate::AllocationError<I>> {
        if delta == I::ZERO {
            return Ok(());
        }

        let error = || crate::AllocationError { size: delta };

        // Calculate the new number of pages
        let old_len = self.len();
        let old_size = self.size();
        let new_size = match old_size.checked_add(&delta) {
            Some(sum) if sum <= self.limit => sum,
            _ => return Err(error()), // Size exceeded or overflow occurred
        };

        debug_assert_ne!(new_size, I::ZERO);

        let is_realloc = old_size == I::ZERO;
        let new_layout = {
            let calculated_size = new_size
                .to_isize()
                .and_then(|new_size| new_size.checked_mul(crate::PAGE_SIZE as isize))
                .ok_or_else(error)?;

            // SAFETY: `calculated_size <= isize::MAX`
            // SAFETY: `SELF::ALIGNMENT` is a non-zero power of two.
            unsafe {
                core::alloc::Layout::from_size_align_unchecked(
                    calculated_size as usize,
                    Self::ALIGNMENT,
                )
            }
        };

        debug_assert!(new_layout.size() > old_len);

        let old_allocation = self.allocation.get().cast::<u8>();
        let pointer: *mut u8 = if is_realloc {
            // SAFETY: arguments are valid since they are from a previous call to `Layout::from_size_align_unchecked()`.
            let old_layout =
                unsafe { core::alloc::Layout::from_size_align_unchecked(old_len, Self::ALIGNMENT) };

            unsafe { alloc::alloc::realloc(old_allocation.as_ptr(), old_layout, new_layout.size()) }
        } else {
            // SAFETY: `layout` size is guaranteed to be non-zero.
            unsafe { alloc::alloc::alloc_zeroed(new_layout) }
        };

        let new_allocation = if let Some(allocation) = NonNull::new(pointer as *mut Cell<u8>) {
            allocation
        } else {
            // `self.allocation` and `self.len` were not modified.
            return Err(error());
        };

        // Need to fill new pages with zeroes.
        if is_realloc {
            let mut new_pages_ptr = NonNull::slice_from_raw_parts(
                new_allocation.cast::<core::mem::MaybeUninit<u8>>(),
                new_layout.size(),
            );

            // SAFETY: `new_pages_ptr` points to uninitialized memory of size `new_layout.size()`.
            // SAFETY: `old_len..` is in bounds because `new_layout.size() > old_len`.
            let new_pages = unsafe { new_pages_ptr.as_mut().get_unchecked_mut(old_len..) };

            new_pages.fill(core::mem::MaybeUninit::new(0));
        }

        // Done allocating, update the size and allocation.
        self.len.set(I::cast_from_usize(new_layout.size()));
        self.allocation.set(new_allocation);
        Ok(())
    }

    /// Returns a [`NonNull`] pointer into the underlying heap allocation.
    ///
    /// Callers should be wary of dangling pointers that may result from [`grow`]ing the memory,
    /// as the pointer to the allocation may change.
    ///
    /// [`grow`]: HeapMemory::grow
    pub fn as_non_null_slice(&self) -> NonNull<[Cell<u8>]> {
        // Not `pub`, see documentation
        NonNull::slice_from_raw_parts(self.allocation.get(), self.len())
    }

    /// Returns a slice of [`Cell`]s over the underlying heap allocation.
    ///
    /// # Safety
    ///
    /// Callers must not use the slice after [`grow`]ing the memory, as it may be a dangling reference.
    ///
    /// [`grow`]: HeapMemory::grow
    pub unsafe fn as_slice_of_cells(&self) -> &[Cell<u8>] {
        let slice = self.as_non_null_slice();

        // SAFETY: contents live as long as `&self`, as long as caller does `grow` the memory.
        unsafe { slice.as_ref() }
    }

    /// Returns a mutable slice to the linear memory contents.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        let mut slice =
            NonNull::slice_from_raw_parts(self.allocation.get_mut().cast::<u8>(), self.len());

        // SAFETY: `&mut self` ensures exclusive access to the memory contents.
        // SAFETY: contents live as long as `&mut self`.
        unsafe { slice.as_mut() }
    }
}

fn slice_into<I, L>(memory: &[Cell<u8>], address: I, length: L) -> crate::BoundsCheck<&[Cell<u8>]>
where
    I: Address,
    L: num_traits::AsPrimitive<usize>,
{
    memory
        .get(address.as_()..)
        .and_then(|start| start.get(..length.as_()))
        .ok_or(crate::BoundsCheckError)
}

impl<I: Address> crate::Memory<I> for HeapMemory<I> {
    // fn try_as_any(&self, _: crate::memory::private::Hidden) -> Option<&dyn core::any::Any> {
    //     Some(self)
    // }

    fn maximum(&self) -> I {
        self.limit
    }

    fn size(&self) -> I {
        // Should be optimized to a right but shift.
        I::cast_from_usize(self.len() / (crate::PAGE_SIZE as usize))
    }

    fn grow(&self, delta: I) -> I {
        let old = self.size();
        match self.try_grow(delta) {
            Ok(()) => old,
            Err(_) => I::GROW_FAILED,
        }
    }

    fn copy_to_slice(&self, addr: I, dst: &mut [u8]) -> crate::BoundsCheck<()> {
        // SAFETY: no calls to `grow` occur within this method.
        let memory = unsafe { self.as_slice_of_cells() };

        let src = slice_into(memory, addr, dst.len())?;

        // This should get optimized into a call to `memmove`.
        for (s, d) in src.iter().zip(dst) {
            *d = s.get();
        }

        Ok(())
    }

    fn copy_from_slice(&self, addr: I, src: &[u8]) -> crate::BoundsCheck<()> {
        // SAFETY: no calls to `grow` occur within this method.
        let memory = unsafe { self.as_slice_of_cells() };

        let dst = slice_into(memory, addr, src.len())?;

        // This should get optimized into a call to `memmove`.
        for (s, d) in src.iter().zip(dst) {
            d.set(*s);
        }

        Ok(())
    }

    fn copy_within(&self, dst_addr: I, src_addr: I, len: I) -> crate::BoundsCheck<()> {
        // SAFETY: no calls to `grow` occur within this method.
        let memory = unsafe { self.as_slice_of_cells() };

        let src = slice_into(memory, src_addr, len)?;
        let dst = slice_into(memory, dst_addr, len)?;

        // This should get optimized into a call to `memmove`.
        for (s, d) in src.iter().zip(dst) {
            d.set(s.get());
        }

        Ok(())
    }

    // TODO: Would a `modify()` helper allow exclusive access + easier memmove for `copy_from`?
    // Would have to prevent exposing address of heap allocation to user to avoid surprise U.B.s
    /* fn copy_from<Src>(
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
    } */
}

impl<I: Address> core::panic::UnwindSafe for HeapMemory<I> {}

impl<I: Address> Drop for HeapMemory<I> {
    fn drop(&mut self) {
        if !self.is_empty() {
            // SAFETY: arguments provided are valid because they originate from a previous call.
            let layout = unsafe {
                core::alloc::Layout::from_size_align_unchecked(self.len(), Self::ALIGNMENT)
            };

            // SAFETY: `is_empty()` check ensures the pointer is to a valid allocation.
            unsafe {
                alloc::alloc::dealloc(self.allocation.get().cast::<u8>().as_ptr(), layout);
            }
        }
    }
}

impl<I: Address> core::fmt::Debug for HeapMemory<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HeapMemory")
            .field("allocation", &self.allocation.get())
            .field("size", &self.size())
            .field("maximum", &self.maximum())
            .finish()
    }
}

// TODO: UpperHex stuff to print linear memory contents
