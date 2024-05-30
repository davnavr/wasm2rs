//! Provides traits used for types representing [WebAssembly table] elements.
//!
//! [WebAssembly table]: https://webassembly.github.io/spec/core/syntax/modules.html#syntax-table

mod swap_guard;

/// Trait for values that can be stored in [`Table`]s.
///
/// [`Table`]: wasm2rs-rt-table::Table
pub trait TableElement: Clone {}

impl TableElement for () {}

/// Trait for values that can be stored in [`Table`]s with a well defined [`NULL`] value.
///
/// [`Table`]: wasm2rs-rt-table::Table
/// [`NULL`]: NullableTableElement::NULL
pub trait NullableTableElement: TableElement {
    /// The constant [**null**] value.
    ///
    /// [**null**]: https://webassembly.github.io/spec/core/exec/runtime.html#values
    const NULL: Self;

    /// Returns `true` if `self` is the [`NULL`] value.
    ///
    /// [`NULL`]: NullableTableElement::NULL
    fn is_null(&self) -> bool;

    /// Used to dispose of an unused [`NULL`] value. This method is intended to be used alongside
    /// [`Cell::replace()`], where [`NULL`] is temporarily stored into a [`Cell`] to access its value.
    ///
    /// The default implementation simply calls [`mem::forget()`] if [`elem.is_null()`], as most
    /// [`TableElement`] implementations are expected to have a [`Drop`] implementation do nothing
    /// for a [`NULL`] value.
    ///
    /// [`NULL`]: NullableTableElement::NULL
    /// [`Cell::replace()`]: core::cell::Cell::replace()
    /// [`Cell`]: core::cell::Cell
    /// [`mem::forget()`]: core::mem::forget()
    /// [`elem.is_null()`]: NullableTableElement::is_null()
    fn forget_null(elem: Self) {
        if elem.is_null() {
            core::mem::forget(elem);
        }
    }

    /// Clones the value stored in the given [`Cell`].
    ///
    /// [`Cell`]: core::cell::Cell
    fn clone_from_cell(cell: &core::cell::Cell<Self>) -> Self {
        // TODO: Replace the duplicate code rt-table/swap_guard.rs
        swap_guard::Guard::access(cell).clone()
    }

    /// Accesses the contents of the [`Cell`] with the given closure. When this method returns, the
    /// element (which may have been modified or replaced) is written back into the [`Cell`].
    ///
    /// If the closure accesses the contents of the [`Cell`], they **may** observe a [`NULL`]
    /// value.
    ///
    /// [`Cell`]: core::cell::Cell
    /// [`NULL`]: NullableTableElement::NULL
    fn with_cell_contents<F, R>(cell: &core::cell::Cell<Self>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let mut contents = swap_guard::Guard::access(cell);
        f(&mut contents)
    }
}

impl NullableTableElement for () {
    const NULL: () = ();

    fn is_null(&self) -> bool {
        true
    }

    fn forget_null(_: ()) {}

    fn clone_from_cell(_: &core::cell::Cell<()>) {}

    fn with_cell_contents<F, R>(_: &core::cell::Cell<()>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        f(&mut ())
    }
}
