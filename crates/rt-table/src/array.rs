use crate::{BoundsCheck, BoundsCheckError, NullableTableElement};
use core::cell::Cell;

/// A [`Table`] implementation backed by an array.
///
/// The `MAX` specifies the [`Table::maximum()`], and is truncated to a [`u32`] value.
///
/// [`Table`]: crate::Table
/// [`Table::maximum()`]: crate::AnyTable::maximum()
pub struct ArrayTable<E: NullableTableElement, const MAX: usize> {
    // Can't use `Cell<E>` and `Cell<MaybeUninit<E>>` due to potentially differing layouts.
    elements: [Cell<E>; MAX],
    /// Must be less than or equal to the `MAX`.
    size: Cell<u32>,
}

impl<E: NullableTableElement, const MAX: usize> ArrayTable<E, MAX> {
    /// The [`maximum()`] number of elements the table can contain.
    ///
    /// [`maximum()`]: crate::AnyTable::maximum()
    #[allow(clippy::cast_possible_truncation)]
    pub const TRUNCATED_MAX: u32 = if (usize::BITS > u32::BITS) && (MAX > u32::MAX as usize) {
        u32::MAX
    } else {
        MAX as u32
    };

    const UNINIT_ELEM: Cell<E> = Cell::new(E::NULL);

    /// Creates a new empty [`ArrayTable`].
    pub const fn new() -> Self {
        Self {
            elements: [Self::UNINIT_ELEM; MAX],
            size: Cell::new(0),
        }
    }

    /// Creates an [`ArrayTable`] from the given elements, with the [`size()`] set to the maximum.
    ///
    /// [`size()`]: crate::AnyTable::size()
    pub fn from_array(array: [E; MAX]) -> Self {
        Self {
            elements: array.map(Cell::new),
            size: Cell::new(Self::TRUNCATED_MAX),
        }
    }

    //fn into_array

    //fn as_uninit_array_of_cells

    /// Gets a slice over the current elements of the table.
    pub fn as_slice_of_cells(&self) -> &[Cell<E>] {
        let size = self.size.get();

        debug_assert!(size <= Self::TRUNCATED_MAX);

        // SAFETY: invariant that `size <= MAX`.
        unsafe { self.elements.get_unchecked(0..size as usize) }
    }
}

impl<E: NullableTableElement, const MAX: usize> Default for ArrayTable<E, MAX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: NullableTableElement, const MAX: usize> From<[E; MAX]> for ArrayTable<E, MAX> {
    fn from(elements: [E; MAX]) -> Self {
        Self::from_array(elements)
    }
}

impl<E: NullableTableElement, const MAX: usize> crate::AnyTable for ArrayTable<E, MAX> {
    fn size(&self) -> u32 {
        self.size.get()
    }

    fn grow(&self, delta: u32) -> u32 {
        match self.size.get().checked_add(delta) {
            Some(new_size) if new_size <= Self::TRUNCATED_MAX => self.size.replace(new_size),
            _ => crate::GROW_FAILED,
        }
    }

    fn maximum(&self) -> u32 {
        Self::TRUNCATED_MAX
    }
}

impl<E: NullableTableElement, const MAX: usize> crate::Table<E> for ArrayTable<E, MAX> {
    fn get(&self, idx: u32) -> BoundsCheck<E> {
        let cell = self
            .as_slice_of_cells()
            .get(crate::index_to_usize(idx)?)
            .ok_or(BoundsCheckError)?;

        Ok(crate::swap_guard::Guard::access(cell).clone())
    }

    fn replace(&self, idx: u32, elem: E) -> BoundsCheck<E> {
        Ok(self
            .as_slice_of_cells()
            .get(crate::index_to_usize(idx)?)
            .ok_or(BoundsCheckError)?
            .replace(elem))
    }

    fn as_mut_slice(&mut self) -> &mut [E] {
        let size = *self.size.get_mut();

        debug_assert!(size <= Self::TRUNCATED_MAX);

        // SAFETY: invariant that `size <= MAX`.
        let slice: &mut [Cell<E>] = unsafe { self.elements.get_unchecked_mut(0..size as usize) };

        // SAFETY: `Cell<E>` and `E` have the same layout.
        let cell: &mut Cell<[E]> = unsafe { &mut *(slice as *mut [Cell<E>] as *mut Cell<[E]>) };

        cell.get_mut()
    }

    fn set(&self, idx: u32, elem: E) -> BoundsCheck<()> {
        self.as_slice_of_cells()
            .get(crate::index_to_usize(idx)?)
            .ok_or(BoundsCheckError)?
            .set(elem);

        Ok(())
    }

    fn clone_from_slice(&self, idx: u32, src: &[E]) -> BoundsCheck<()> {
        let slice = crate::index_into_slice(self.as_slice_of_cells(), idx, src.len())?;
        for (cloned, dst) in src.iter().cloned().zip(slice) {
            dst.set(cloned);
        }

        Ok(())
    }

    fn clone_into_slice(&self, idx: u32, dst: &mut [E]) -> BoundsCheck<()> {
        let slice = crate::index_into_slice(self.as_slice_of_cells(), idx, dst.len())?;
        for (dst, src) in dst.iter_mut().zip(slice) {
            *dst = crate::swap_guard::Guard::access(src).clone();
        }

        Ok(())
    }

    fn fill(&self, idx: u32, len: u32, elem: E) -> BoundsCheck<()> {
        let dst = crate::index_into_slice(self.as_slice_of_cells(), idx, len)?;
        if let Some((last, head)) = dst.split_last() {
            for cell in head {
                cell.set(elem.clone());
            }

            last.set(elem);
        }

        Ok(())
    }
}

impl<E: NullableTableElement, const MAX: usize> crate::TableExt<E> for ArrayTable<E, MAX> {}

impl<E: NullableTableElement + core::fmt::Debug, const MAX: usize> core::fmt::Debug
    for ArrayTable<E, MAX>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut list = f.debug_list();

        for cell in self.as_slice_of_cells() {
            list.entry(&*crate::swap_guard::Guard::access(cell));
        }

        list.finish()
    }
}
