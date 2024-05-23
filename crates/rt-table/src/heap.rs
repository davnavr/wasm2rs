use crate::NullableTableElement;
use core::{cell::Cell, ptr::NonNull};

/// A [`Table`] implementation backed by a heap allcation.
///
/// [`Table`]: crate::Table
pub struct HeapTable<E: NullableTableElement> {
    allocation: Cell<NonNull<Cell<E>>>,
    /// The number of elements in the table.
    ///
    /// # Invariants
    ///
    /// - The `size` cannot exceed [`HeapTable::limits`].
    /// - The [`HeapTable::allocation`] must point to a valid [`[Cell<E>; size]`].
    size: Cell<u32>,
    /// The maximum number of elements this table can have.
    limit: u32,
}

impl<E: NullableTableElement> HeapTable<E> {
    /// Creates an empty table with the specified [`maximum()`] number of elements.
    ///
    /// [`maximum()`]: crate::AnyTable::maximum()
    pub const fn with_maximum(maximum: u32) -> Self {
        Self {
            allocation: Cell::new(NonNull::dangling()),
            size: Cell::new(0),
            limit: maximum,
        }
    }

    /// Creates an empty table with no [`maximum()`] limit.
    ///
    /// [`maximum()`]: crate::AnyTable::maximum()
    pub const fn new() -> Self {
        Self::with_maximum(u32::MAX)
    }

    /// Allocates a table, with the minimum and maximum number of elements.
    ///
    /// If the `minimum` is `0`, then no initial allocation occurs.
    ///
    /// # Errors
    ///
    /// Returns an error if the `minimum` number of pages could not be allocated.
    pub fn with_limits(minimum: u32, maximum: u32) -> Result<Self, crate::AllocationError> {
        let table = Self::with_maximum(maximum);
        table.try_grow(minimum)?;
        Ok(table)
    }

    /// Returns the [`size()`] of the table, in number of elements.
    ///
    /// [`size()`]: crate::AnyTable::size()
    #[allow(clippy::cast_possible_truncation)]
    pub fn len(&self) -> usize {
        // Won't fail, since `try_grow()` would catch an overflow.
        self.size.get() as usize
    }

    /// Returns `true` if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.size.get() == 0
    }

    /// Attempts to increase the [`size()`] of the table by the given number of elements. Returns
    /// the old number of elements.
    ///
    /// # Errors
    ///
    /// Returns an error if space for the additional elements could not be allocated.
    ///
    /// [`size()`]: crate::AnyTable::size()
    pub fn try_grow(&self, delta: u32) -> Result<u32, crate::AllocationError> {
        // Similar to `HeapMemory::try_grow()`.
        let old_size = self.size.get();
        if delta == 0 {
            return Ok(old_size);
        }

        let error = move || crate::AllocationError { size: delta };

        let new_size = match old_size.checked_add(delta) {
            Some(sum) if sum <= self.limit => sum,
            _ => return Err(error()),
        };

        if core::mem::size_of::<E>() == 0 {
            todo!("support for ZSTs in HeapTable is not implemented");

            // self.size.set(new_size);
            // return Ok(new_size)
        }

        debug_assert!(new_size > old_size);

        let is_realloc = old_size != 0;
        let new_layout = usize::try_from(new_size)
            .ok()
            .and_then(|len| core::alloc::Layout::array::<Cell<E>>(len).ok())
            .ok_or_else(error)?;

        let pointer: *mut u8 = if is_realloc {
            let old_pointer = self.allocation.get().cast::<u8>().as_ptr();

            // SAFETY: `Ok` is returned since the previous call to `Layout::array()` return `Ok`.
            #[allow(clippy::cast_possible_truncation)] // `old_size` is known to fit in an `usize`.
            let old_layout = unsafe {
                core::alloc::Layout::array::<Cell<E>>(old_size as usize).unwrap_unchecked()
            };

            // SAFETY: `self.allocation.get().cast::<u8>()` originates from global allocator.
            unsafe { alloc::alloc::realloc(old_pointer, old_layout, new_layout.size()) }
        } else {
            // SAFETY: `new_layout` size is guranteed to be non-zero.
            unsafe { alloc::alloc::alloc(new_layout) }
        };

        let new_allocation = if let Some(allocation) = NonNull::new(pointer as *mut Cell<E>) {
            allocation
        } else {
            // `self.allocation` and `self.size` were not modified.
            return Err(error());
        };

        // Fill new elements with `E::NULL`.
        let full_elements = {
            // `Layout::array()` calculation ensures no overflow occurs.
            #[allow(clippy::cast_possible_truncation)]
            let mut new_allocation = NonNull::slice_from_raw_parts(
                new_allocation.cast::<core::mem::MaybeUninit<Cell<E>>>(),
                new_size as usize,
            );

            // SAFETY: The allocation is new, so there is exclusive access to it.
            // SAFETY: `MaybeUninit<Cell<E>>` and `Cell<E>` have the same layout.
            unsafe { new_allocation.as_mut() }
        };

        // SAFETY: the range is in bounds, since `new_size > old_size`.
        let new_elements = unsafe { full_elements.get_unchecked_mut(old_size as usize..) };

        for uninit in new_elements.iter_mut() {
            uninit.write(Cell::new(E::NULL));
        }

        self.allocation.set(new_allocation);
        self.size.set(new_size);
        Ok(old_size)
    }

    /// Returns a raw pointer to the underlying allocation containing the table's elements.
    ///
    /// Callers should be wary of any dangling pointers that could occur if
    /// [`HeapTable::try_grow()`] is called.
    fn as_ptr(&self) -> *const [Cell<E>] {
        core::ptr::slice_from_raw_parts(self.allocation.get().as_ptr(), self.len())
    }

    /// Returns a mutable raw pointer to the underlying allocation.
    ///
    /// See [`HeapTable::as_ptr()`] for more information.
    fn as_mut_ptr(&mut self) -> *mut [Cell<E>] {
        core::ptr::slice_from_raw_parts_mut(self.allocation.get_mut().as_ptr(), self.len())
    }

    /// Returns a slice containing the tables elements.
    ///
    /// # Safety
    ///
    /// A reference to the returned slice must not exist when [`HeapTable::try_grow()`] is called.
    unsafe fn as_slice_of_cells(&self) -> &[Cell<E>] {
        // SAFETY: slice lives for `self` as long as `try_grow()` isn't called.
        unsafe { &*self.as_ptr() }
    }
}

impl<E: NullableTableElement> Default for HeapTable<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Obtains a boxed slice containing the table's elements without creating a new allocation.
impl<E: NullableTableElement> From<HeapTable<E>> for alloc::boxed::Box<[Cell<E>]> {
    fn from(table: HeapTable<E>) -> Self {
        let table = core::mem::ManuallyDrop::new(table);
        let allocation =
            core::ptr::slice_from_raw_parts_mut(table.allocation.get().as_ptr(), table.len());

        // SAFETY: `ptr` originates from global allocator.
        // SAFETY: `ptr` refers to allocation using the exact layout of `E`.
        // SAFETY: `ptr` refers to a valid `[E; length]`.
        unsafe { alloc::boxed::Box::from_raw(allocation) }
    }
}

/// Obtains a [`Vec`] containing the table's elements without creating a new allocation.
///
/// [`Vec`]: alloc::vec::Vec
impl<E: NullableTableElement> From<HeapTable<E>> for alloc::vec::Vec<Cell<E>> {
    fn from(table: HeapTable<E>) -> Self {
        alloc::boxed::Box::<[Cell<E>]>::from(table).into()
    }
}

// /// Attempts to create a [`HeapTable`] from the elements stored in the given boxed slice without creating a new heap allocation.
// impl TryFrom<Box<Cell<E>>>
// impl TryFrom<Vec<Cell<E>>> // fill to capacity

impl<E: NullableTableElement> crate::AnyTable for HeapTable<E> {
    fn size(&self) -> u32 {
        self.size.get()
    }

    fn grow(&self, delta: u32) -> u32 {
        self.try_grow(delta).unwrap_or(crate::GROW_FAILED)
    }

    fn maximum(&self) -> u32 {
        self.limit
    }
}

impl<E: NullableTableElement> crate::Table<E> for HeapTable<E> {
    fn get(&self, idx: u32) -> crate::BoundsCheck<E> {
        // SAFETY: no `try_grow()` calls in this method.
        let elements = unsafe { self.as_slice_of_cells() };

        let cell = elements
            .get(crate::index_to_usize(idx)?)
            .ok_or(crate::BoundsCheckError)?;

        Ok(crate::swap_guard::Guard::access(cell).clone())
    }

    fn replace(&self, idx: u32, elem: E) -> crate::BoundsCheck<E> {
        // SAFETY: no `try_grow()` calls in this method.
        let elements = unsafe { self.as_slice_of_cells() };

        Ok(elements
            .get(crate::index_to_usize(idx)?)
            .ok_or(crate::BoundsCheckError)?
            .replace(elem))
    }

    fn as_mut_slice(&mut self) -> &mut [E] {
        let ptr = self.as_mut_ptr();

        // SAFETY: `&mut self` ensures exclusive access.
        // SAFETY: allocation lives for `&self`.
        let cell: &mut Cell<[E]> = unsafe { &mut *(ptr as *mut Cell<[E]>) };

        cell.get_mut()
    }

    fn set(&self, idx: u32, elem: E) -> crate::BoundsCheck<()> {
        // SAFETY: no `try_grow()` calls in this method.
        let elements = unsafe { self.as_slice_of_cells() };

        elements
            .get(crate::index_to_usize(idx)?)
            .ok_or(crate::BoundsCheckError)?
            .set(elem);

        Ok(())
    }

    fn clone_from_slice(&self, idx: u32, src: &[E]) -> wasm2rs_rt_core::BoundsCheck<()> {
        // SAFETY: no `try_grow()` calls in this method.
        let elements = unsafe { self.as_slice_of_cells() };

        let dst = elements
            .get(crate::index_to_usize(idx)?..)
            .and_then(|slice| slice.get(..src.len()))
            .ok_or(crate::BoundsCheckError)?;

        for (d, s) in dst.iter().zip(src.iter().cloned()) {
            d.set(s);
        }

        Ok(())
    }

    fn clone_into_slice(&self, idx: u32, dst: &mut [E]) -> wasm2rs_rt_core::BoundsCheck<()> {
        // SAFETY: no `try_grow()` calls in this method.
        let elements = unsafe { self.as_slice_of_cells() };

        let src = elements
            .get(crate::index_to_usize(idx)?..)
            .and_then(|slice| slice.get(..dst.len()))
            .ok_or(crate::BoundsCheckError)?;

        for (d, s) in dst.iter_mut().zip(src) {
            *d = crate::swap_guard::Guard::access(s).clone();
        }

        Ok(())
    }
}

impl<E: NullableTableElement> crate::TableExt<E> for HeapTable<E> {}

impl<E: NullableTableElement> Drop for HeapTable<E> {
    fn drop(&mut self) {
        // SAFETY: pointer is valid exclusive reference to `[E; size]`.
        // SAFETY: can drop, since allocation will no longer be used after this point.
        unsafe {
            core::ptr::drop_in_place::<[Cell<E>]>(self.as_mut_ptr());
        }
    }
}

impl<E: NullableTableElement + core::fmt::Debug> core::fmt::Debug for HeapTable<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut list = f.debug_list();

        // SAFETY: no `try_grow()` calls in this method.
        let elements = unsafe { self.as_slice_of_cells() };

        for cell in elements {
            list.entry(&*crate::swap_guard::Guard::access(cell));
        }

        list.finish()
    }
}
