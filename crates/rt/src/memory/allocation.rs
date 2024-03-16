use core::ptr::NonNull;

/// Arbitrary value used for the alignment of the underlying heap allocation.
///
/// This value should allow for aligned accesses of `i32`, `i64`, `v128`, and others.
pub(in crate::memory) const PAGE_ALIGN: usize = 256;

const PAGE_SIZE: core::num::NonZeroUsize =
    if let Some(size) = core::num::NonZeroUsize::new(crate::memory::PAGE_SIZE as usize) {
        size
    } else {
        panic!("bad page size")
    };

/// Backing allocation for a linear memory implementation.
pub(in crate::memory) struct Memory {
    pointer: NonNull<u8>,
    /// The current number of pages.
    size: u32,
    _marker: core::marker::PhantomData<[u8]>,
}

impl Memory {
    pub(in crate::memory) const EMPTY: Self = Self {
        pointer: NonNull::dangling(),
        size: 0,
        _marker: core::marker::PhantomData,
    };

    pub(in crate::memory) fn modify<R>(
        allocation: &core::cell::Cell<Self>,
        f: impl for<'a> FnOnce(&'a mut Self) -> R,
    ) -> R {
        let mut temp = allocation.replace(Self::EMPTY);
        let result = f(&mut temp);
        let _taken = allocation.replace(temp);

        debug_assert!(
            _taken.pointer == Self::EMPTY.pointer && _taken.size == 0,
            "allocation was unexpectedly modified"
        );

        result
    }

    /// Gets the length of the allocation, this is guaranteed to be a multiple of the page size.
    pub(in crate::memory) fn len(&self) -> usize {
        usize::try_from(self.size).expect("grow should ensure size doesn't overflow")
            * PAGE_SIZE.get()
    }

    pub(in crate::memory) fn size(&self) -> u32 {
        self.size
    }

    pub(in crate::memory) fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: `mut` ensures exclusive access of underlying allocation.
        unsafe { NonNull::slice_from_raw_parts(self.pointer, self.len()).as_mut() }
    }

    pub(in crate::memory) fn grow(&mut self, delta: u32) -> Option<u32> {
        if let Some(delta) = core::num::NonZeroU32::new(delta) {
            let new_size = delta.checked_add(self.size)?;
            let new_len = core::num::NonZeroUsize::try_from(new_size)
                .ok()?
                .checked_mul(PAGE_SIZE)?;

            let new_layout =
                core::alloc::Layout::from_size_align(new_len.get(), PAGE_ALIGN).ok()?;

            let is_new_allocation = self.size == 0;
            let allocation = if is_new_allocation {
                // SAFETY: `layout` size is guaranteed to be non-zero.
                unsafe { alloc::alloc::alloc_zeroed(new_layout) }
            } else {
                // SAFETY: if reallocating, size is guaranteed to be positive `NonZeroIsize`.
                // SAFETY: same alignment used for all allocations.
                let layout = unsafe {
                    core::alloc::Layout::from_size_align_unchecked(self.len(), PAGE_ALIGN)
                };

                // SAFETY: `self.pointer` refers to existing allocation using `old_layout`.
                // SAFETY: `Layout::size()` guaranteed to be positive `NonZeroIsize`.
                unsafe { alloc::alloc::realloc(self.pointer.as_ptr(), layout, new_layout.size()) }
            };

            self.pointer = NonNull::new(allocation)?.cast();

            // Need to fill new pages with zeroes
            if is_new_allocation {
                // SAFETY: pointer calculation won't overflow, and refers to within same object.
                // SAFETY: pointer refers to newly allocated pages.
                let new_pages = unsafe {
                    core::slice::from_raw_parts_mut(
                        self.pointer
                            .cast::<core::mem::MaybeUninit<u8>>()
                            .as_ptr()
                            .add(self.len()),
                        new_len.get() - self.len(),
                    )
                };

                new_pages.fill(core::mem::MaybeUninit::new(0));
            }

            Some(core::mem::replace(&mut self.size, new_size.get()))
        } else {
            Some(self.size)
        }
    }
}

// SAFETY: enforced by bound below.
unsafe impl Send for Memory where [u8]: Send {}

impl Drop for Memory {
    fn drop(&mut self) {
        let len = self.len();
        let pointer = core::mem::replace(&mut self.pointer, NonNull::dangling());
        self.size = 0;

        if pointer != NonNull::dangling() {
            // SAFETY: the layout of the allocation was already valid.
            let layout = unsafe { core::alloc::Layout::from_size_align_unchecked(len, PAGE_ALIGN) };

            // SAFETY: `pointer` refers to an allocation with this layout.
            unsafe { alloc::alloc::dealloc(pointer.as_ptr(), layout) }
        }
    }
}

impl core::fmt::Debug for Memory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Allocation")
            .field("pointer", &self.pointer)
            .field("size", &crate::memory::DisplaySize(self.size))
            .finish()
    }
}
