//! Provides traits used for types representing [WebAssembly table] elements.
//!
//! [WebAssembly table]:

/// Trait for values that can be stored in [`Table`]s.
///
/// [`Table`]: wasm2rs-rt-table::Table
pub trait TableElement: Clone + Eq {}

/// Trait for values that can be stored in [`Table`]s with a well defined [`NULL`] value.
///
/// [`Table`]: wasm2rs-rt-table::Table
/// [`NULL`]: NullableTableElement::NULL
pub trait NullableTableElement: TableElement {
    /// The constant [**null**] value.
    ///
    /// [**null**]: https://webassembly.github.io/spec/core/exec/runtime.html#values
    const NULL: Self;

    /// Used to dispose of an unused [`NULL`] value. This method is intended to be used alongside
    /// [`Cell::replace()`], where [`NULL`] is temporarily stored into a [`Cell`] to access its value.
    ///
    /// The default implementation simply calls [`mem::forget()`] if `elem == NULL`, as most
    /// [`TableElement`] implementations are expected to have a [`Drop`] implementation do nothing
    /// for a [`NULL`] value.
    ///
    /// [`NULL`]: NullableTableElement::NULL
    /// [`Cell::replace()`]: core::cell::Cell::replace()
    /// [`Cell`]: core::cell::Cell
    /// [`mem::forget()`]: core::mem::forget()
    fn forget_null(elem: Self) {
        if elem == Self::NULL {
            core::mem::forget(elem);
        }
    }
}
