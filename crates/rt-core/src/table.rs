//! Provides traits used for types representing [WebAssembly table] elements.
//!
//! [WebAssembly table]:

/// Trait for values that can be stored in [`Table`]s.
///
/// [`Table`]: wasm2rs-rt-table::Table
pub trait TableElement: Clone {}

/// Trait for values that can be stored in [`Table`]s with a well defined [`NULL`] value.
///
/// [`Table`]: wasm2rs-rt-table::Table
/// [`NULL`]: NullableTableElement::NULL
pub trait NullableTableElement: TableElement {
    /// The constant [**null**] value.
    ///
    /// [**null**]: https://webassembly.github.io/spec/core/exec/runtime.html#values
    const NULL: Self;
}
