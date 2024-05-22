//! Implementation of WebAssembly tables for `wasm2rs`.

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[doc(no_inline)]
pub use wasm2rs_rt_core::{
    table::{NullableTableElement, TableElement},
    BoundsCheck, BoundsCheckError,
};

/// Trait for common operations shared by [`Table`]s of all element types.
pub trait AnyTable {
    /// Returns the current number of elements in the table.
    fn size(&self) -> u32;

    /// Gets the maximum number of elements the table can contain.
    fn maximum(&self) -> u32 {
        u32::MAX
    }
}

/// Trait for implementations of [WebAssembly tables].
///
/// [WebAssembly tables]: https://webassembly.github.io/spec/core/syntax/modules.html#tables
pub trait Table<E: TableElement>: AnyTable {
    /// Gets the element at the given index.
    ///
    /// # Error
    ///
    /// Returns an error if the index is greater than or equal to the [`table.size`].
    ///
    /// [`table.size`]: AnyTable::size()
    fn get(&self, idx: u32) -> BoundsCheck<E>;

    /// Gets the element at the given index.
    ///
    /// # Error
    ///
    /// Returns an error if the index is greater than or equal to the [`table.size`].
    ///
    /// [`table.size`]: AnyTable::size()
    fn set(&self, idx: u32, elem: E) -> BoundsCheck<()>;
}

// pub struct ArrayTable<const N: usize, E: TableElement> {
//     elements: [E; CAP],
// }
