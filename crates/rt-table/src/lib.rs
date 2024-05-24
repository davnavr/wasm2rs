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

mod array;
mod empty;
mod error;
mod helpers;
//mod slice; // struct SliceTable<'a> // Lifetimes in wasm2rs modules are not yet supported
mod swap_guard;

pub use array::ArrayTable;
pub use empty::EmptyTable;
pub use error::{AccessError, AllocationError, LimitsMismatchError};
pub use helpers::*;

#[cfg(feature = "alloc")]
mod heap;
#[cfg(feature = "alloc")]
pub use heap::HeapTable;

/// Constant value returned by [`AnyTable::grow()`] used to indicate failure.
pub const GROW_FAILED: u32 = -1i32 as u32;

fn index_to_usize(idx: u32) -> BoundsCheck<usize> {
    usize::try_from(idx).map_err(|_| crate::BoundsCheckError)
}

fn index_into_slice<T>(
    elements: &[T],
    index: impl TryInto<usize>,
    length: impl TryInto<usize>,
) -> BoundsCheck<&[T]> {
    index
        .try_into()
        .ok()
        .and_then(|start| elements.get(start..)?.get(..length.try_into().ok()?))
        .ok_or(BoundsCheckError)
}

/// Trait for common operations shared by [`Table`]s of all element types.
pub trait AnyTable {
    /// Returns the current number of elements in the table.
    ///
    /// This should never be equal to [`GROW_FAILED`].
    fn size(&self) -> u32;

    /// Gets the maximum number of elements the table can contain.
    fn maximum(&self) -> u32;

    /// Increases the size of the table by the specified number of elements, and returns the old
    /// number of elements.
    ///
    /// # Errors
    ///
    /// If the size of the table could not be increased, then [`GROW_FAILED`] is returned.
    fn grow(&self, delta: u32) -> u32;
}

/// Assumes that the `src` and `dst` are actually the same tables.
fn default_clone_from_conservative<E, Dst, Src>(
    dst: &Dst,
    src: &Src,
    dst_idx: u32,
    src_idx: u32,
    len: u32,
) -> BoundsCheck<()>
where
    E: TableElement,
    Dst: Table<E> + ?Sized,
    Src: Table<E> + ?Sized,
{
    // Range math hurts, use 64-bit math instead to avoid dealing with overflows.
    let src_end = u64::from(len) + u64::from(src_idx);
    if (u64::from(len) + u64::from(dst_idx) > u64::from(dst.size()))
        || (src_end > u64::from(src.size()))
    {
        Err(BoundsCheckError)
    } else if src_end >= u64::from(dst_idx) {
        // Overlapping ranges where source comes before destination, have to copy in reverse.
        for (src_i, dst_i) in (src_idx..(src_idx + len))
            .zip(dst_idx..(dst_idx + len))
            .rev()
        {
            dst.set(dst_i, src.get(src_i)?)?;
        }

        Ok(())
    } else {
        // Non-overlapping, can do naive `for` loop.
        for (src_i, dst_i) in (src_idx..(src_idx + len)).zip(dst_idx..(dst_idx + len)) {
            dst.set(dst_i, src.get(src_i)?)?;
        }

        Ok(())
    }
}

/// Trait for implementations of [WebAssembly tables].
///
/// See also the [`TableExt`] trait.
///
/// [WebAssembly tables]: https://webassembly.github.io/spec/core/syntax/modules.html#tables
pub trait Table<E: TableElement>: AnyTable {
    /// Gets the element at the given index.
    ///
    /// This returns a clone of the element rather than a reference to it, as future operations
    /// such as [`Table::set()`] or [`AnyTable::grow()`] would invalidate any returned references.
    ///
    /// # Errors
    ///
    /// Returns an error if the index is greater than or equal to the [`table.size`].
    ///
    /// [`table.size`]: AnyTable::size()
    fn get(&self, idx: u32) -> BoundsCheck<E>;

    /// Replaces the element at the given index with the given value, and returns the old value.
    ///
    /// # Errors
    ///
    /// Returns an error if the index is greater than or equal to the [`table.size`].
    ///
    /// [`table.size`]: AnyTable::size()
    fn replace(&self, idx: u32, new: E) -> BoundsCheck<E>;

    /// Returns a mutable slice containing the table's elements.
    fn as_mut_slice(&mut self) -> &mut [E];

    /// Sets the element at the given index.
    ///
    /// # Errors
    ///
    /// Returns an error if the index is greater than or equal to the [`table.size`].
    ///
    /// [`table.size`]: AnyTable::size()
    fn set(&self, idx: u32, elem: E) -> BoundsCheck<()> {
        let _ = self.replace(idx, elem)?;
        Ok(())
    }

    /// Copies elements (calling [`E::clone()`]) from `src` into the table starting at the
    /// specified index.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of indices `idx..(idx + src.size())` is not in bounds.
    ///
    /// [`E::clone()`]: Clone::clone()
    fn clone_from_slice(&self, idx: u32, src: &[E]) -> BoundsCheck<()> {
        // 64-bit math since I don't know what to do if `idx + u32::from(src.len())` overflows.
        let src_len = u32::try_from(src.len()).map_err(|_| BoundsCheckError)?;
        if u64::from(idx) + u64::from(src_len) > u64::from(self.size()) {
            Err(BoundsCheckError)
        } else {
            // If the `set()` calls are inlined, then the bounds checks above should remove any
            // bounds checks that occur below.

            let indices = idx..(idx + src_len);
            for (elem, i) in src.iter().cloned().zip(indices) {
                self.set(i, elem)?;
            }

            Ok(())
        }
    }

    /// Copies elements (calling [`E::clone()`]) from the table starting at the specified index
    /// into `dst`.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of indices `idx..(idx + dst.size())` is not in bounds.
    ///
    /// [`E::clone()`]: Clone::clone()
    fn clone_into_slice(&self, idx: u32, dst: &mut [E]) -> BoundsCheck<()> {
        // Duplicated code from `clone_from_slice()`, go there for more details.
        let dst_len = u32::try_from(dst.len()).map_err(|_| BoundsCheckError)?;
        if u64::from(idx) + u64::from(dst_len) > u64::from(self.size()) {
            Err(BoundsCheckError)
        } else {
            let indices = idx..(idx + dst_len);
            for (elem, i) in dst.iter_mut().zip(indices) {
                *elem = self.get(i)?;
            }

            Ok(())
        }
    }

    /// Moves a range of elements (calling [`E::clone()`]) within the table to another location.
    ///
    /// If elements need to be copied to another table, use the [`TableExt::clone_from()`] method instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the ranges `dst_idx..(dst_idx + len)` or `src_idx..(src_idx + len)` are
    /// not in bounds.
    ///
    /// [`E::clone()`]: Clone::clone()
    fn clone_within(&self, dst_idx: u32, src_idx: u32, len: u32) -> BoundsCheck<()> {
        default_clone_from_conservative(self, self, dst_idx, src_idx, len)
    }

    /// Fills a range with the [`clone`]s of the given element.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of indices `idx..(idx + len))` is not in bounds.
    ///
    /// [`clone`]: Clone::clone()
    fn fill(&self, idx: u32, len: u32, elem: E) -> BoundsCheck<()> {
        // `Cow<E>` could be used for `elem` if it was available in `core`, however, such a change
        // would only benefit the (expected to be rare) case where `len == 0`.

        let end_idx = match idx.checked_add(len) {
            Some(sum) if sum < self.size() => sum,
            _ => return Err(BoundsCheckError),
        };

        if len > 0 {
            let last_idx = end_idx - 1;
            for i in idx..last_idx {
                self.set(i, elem.clone())?;
            }

            // Optimization, don't needlessly copy the last element.
            self.set(last_idx, elem)?;
        }

        Ok(())
    }

    /// Copies elements (calling [`E::clone()`]) from the table starting at the specified index,
    /// returning a boxed slice containing them.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of indices `idx..(idx + len)` is not in bounds.
    ///
    /// [`E::clone()`]: Clone::clone()
    #[cfg(feature = "alloc")]
    fn to_boxed_slice(&self, idx: u32, len: u32) -> BoundsCheck<alloc::boxed::Box<[E]>> {
        let truncated_len = usize::try_from(len).unwrap_or(usize::MAX);

        #[allow(clippy::cast_possible_truncation)]
        let acutal_len = truncated_len as u32;

        // Duplicated code from `clone_from_slice()`, go there for more details.
        if u64::from(idx) + u64::from(acutal_len) > u64::from(self.size()) {
            Err(BoundsCheckError)
        } else {
            let mut elements = alloc::vec::Vec::with_capacity(truncated_len);

            for i in idx..(idx + acutal_len) {
                elements.push(self.get(i)?);
            }

            Ok(elements.into_boxed_slice())
        }
    }
}

const _OBJECT_SAFETY: core::marker::PhantomData<&dyn Table<()>> = core::marker::PhantomData;

/// Provides additional operations on [`Table`]s.
///
/// These methods are not provided in the [`Table`] trait to ensure that it remains [object safe].
/// This is because methods such as [`clone_from()`] are required to have a `Self: Sized`
/// bound even though they do not actually need it.
///
/// [object safe]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
/// [`clone_from()`]: TableExt::clone_from()
pub trait TableExt<E: TableElement>: Table<E> {
    /// Copies elements (calling [`E::clone()`]) from `src` into `self`.
    ///
    /// If `src` and `self` are the same tables, use [`Table::clone_within`] instead.
    ///
    /// # Errors
    ///
    /// Returns an error if the range `dst_idx..(dst_idx + len)` is not in bounds in `self`, or if
    /// the range `src_idx..(src_idx + len)` is not in bounds in `src`.
    ///
    /// [`E::clone()`]: Clone::clone()
    fn clone_from<Src>(&self, src: &Src, dst_idx: u32, src_idx: u32, len: u32) -> BoundsCheck<()>
    where
        Src: Table<E> + ?Sized,
    {
        // It is possible for `self` and `src` to have the same address, even when they are
        // actually "different objects" (e.g. ZSTs and `dyn` shenanigans).

        // It is also possible for `self` and `dst` to refer to the "same object", even when their
        // `size_of_val`s differ (more `dyn` or `&` shenanigans).

        // This will simply take the conservative approach and assume `self` and `src` are the same.
        default_clone_from_conservative(self, src, dst_idx, src_idx, len)
    }
}
