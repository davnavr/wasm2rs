use crate::NullableTableElement;
use core::cell::Cell;

/// Takes a value from a [`Cell`], temporarily replacing it with [`E::NULL`],
/// then moving the value back into the cell on [`Drop`].
///
/// [`E::NULL`]: NullableTableElement::NULL
pub(crate) struct Guard<'a, E: NullableTableElement> {
    cell: &'a Cell<E>,
    contents: core::mem::MaybeUninit<E>,
}

impl<'a, E: NullableTableElement> Guard<'a, E> {
    pub(crate) fn access(cell: &'a Cell<E>) -> Self {
        Self {
            contents: core::mem::MaybeUninit::new(cell.replace(E::NULL)),
            cell,
        }
    }
}

impl<E: NullableTableElement> core::ops::Deref for Guard<'_, E> {
    type Target = E;

    fn deref(&self) -> &E {
        // SAFETY: `contents` are only uninitialized in `drop()`.
        unsafe { self.contents.assume_init_ref() }
    }
}

impl<E: NullableTableElement> Drop for Guard<'_, E> {
    /// Moves the value back into the [`Cell`].
    fn drop(&mut self) {
        // SAFETY: `self.contents` is not read after this point.
        // SAFETY: `self.contents` is still initialized at this point.
        let moved_contents = unsafe { self.contents.assume_init_read() };

        E::forget_null(self.cell.replace(moved_contents));
    }
}
