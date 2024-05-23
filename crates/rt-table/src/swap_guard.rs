#![deny(unsafe_code)]

use crate::NullableTableElement;
use core::cell::Cell;

/// Takes a value from a [`Cell`], temporarily replacing it with [`E::NULL`],
/// then moving the value back into the cell on [`Drop`].
///
/// [`E::NULL`]: NullableTableElement::NULL
pub(crate) struct Guard<'a, E: NullableTableElement> {
    cell: &'a Cell<E>,
    contents: E,
}

impl<'a, E: NullableTableElement> Guard<'a, E> {
    pub(crate) fn access(cell: &'a Cell<E>) -> Self {
        Self {
            contents: cell.replace(E::NULL),
            cell,
        }
    }
}

impl<E: NullableTableElement> core::ops::Deref for Guard<'_, E> {
    type Target = E;

    fn deref(&self) -> &E {
        &self.contents
    }
}

impl<E: NullableTableElement> Drop for Guard<'_, E> {
    /// Moves the value back into the [`Cell`].
    fn drop(&mut self) {
        self.cell
            .set(core::mem::replace(&mut self.contents, E::NULL));
    }
}
