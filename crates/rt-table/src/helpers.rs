//! Helper functions for accessing elements and performing other operations on [`Table`]s.
//!
//! Calls to these functions are generated as part of the `wasm2rs` translation process.

#![deny(unsafe_code)]
#![deny(clippy::cast_possible_truncation)]

use crate::{AccessError, AnyTable, BoundsCheck, BoundsCheckError, Table, TableElement};
use wasm2rs_rt_core::{trace::WasmFrame, trap::Trap};

/// Checks that the given [`Table`] has the correct `minimum` and `maximum` number of elements.
/// This implements the [matching of tables] to their expected *limits* during WebAssembly
/// [module instantiation].
///
/// # Errors
///
/// Returns a [`Trap`] if:
/// - The [`size()`] of the [`Table`] is less than the `minimum`.
/// - The [`maximum()`] pages the [`Table`] can have is greater than the `maximum`.
///
/// [matching of tables]: https://webassembly.github.io/spec/core/valid/types.html#tables
/// [module instantiation]: https://webassembly.github.io/spec/core/exec/modules.html#instantiation
/// [`size()`]: AnyTable::size()
/// [`maximum()`]: AnyTable::maximum()
pub fn check_limits<T, E>(table: &T, index: u32, minimum: u32, maximum: u32) -> Result<(), E>
where
    T: AnyTable + ?Sized,
    E: Trap<crate::LimitsMismatchError>,
{
    let actual_minimum = table.size();
    let actual_maximum = table.maximum();
    wasm2rs_rt_core::limit::check(actual_minimum, actual_maximum, minimum, maximum).map_err(|err| {
        use crate::error::LimitsMismatchKind;
        use wasm2rs_rt_core::limit::LimitsCheckError;

        E::trap(
            crate::LimitsMismatchError {
                table: index,
                kind: match err {
                    LimitsCheckError::Invalid => LimitsMismatchKind::Invalid {
                        minimum: actual_minimum,
                        maximum: actual_maximum,
                    },
                    LimitsCheckError::MinimumTooSmall => LimitsMismatchKind::Minimum {
                        actual: actual_minimum,
                        expected: minimum,
                    },
                    LimitsCheckError::MaximumTooLarge => LimitsMismatchKind::Maximum {
                        actual: actual_maximum,
                        expected: maximum,
                    },
                },
            },
            None,
        )
    })
}

/// This implements the [`table.size`] instruction.
///
/// For more information, see the documentation for the [`AnyTable::size()`] method.
///
/// [`table.size`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
pub fn size<T: AnyTable + ?Sized>(table: &T) -> i32 {
    table.size() as i32
}

/// This implements the [`table.grow`] instruction.
///
/// For more information, see the documentation for the [`AnyTable::grow()`] method.
///
/// [`table.grow`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
pub fn grow<T: AnyTable + ?Sized>(table: &T, delta: i32) -> i32 {
    table.grow(delta as u32) as i32
}

#[cold]
#[inline(never)]
fn trap_access_error<E: Trap<AccessError>>(
    table: u32,
    index: u32,
    frame: Option<&'static WasmFrame>,
) -> E {
    E::trap(AccessError { table, index }, frame)
}

/// This implements the [`table.init`] instruction and [active element segment initialization].
///
/// For more information, see the documentation for the [`Table::clone_from_slice()`] method.
///
/// [`table.init`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
/// [active element segment initialization]: https://webassembly.github.io/spec/core/syntax/modules.html#element-segments
pub fn init<const TABLE: u32, R, T, E>(
    table: &T,
    table_idx: i32,
    segment_idx: i32,
    length: i32,
    element_segment: &[R],
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    R: TableElement,
    T: Table<R> + ?Sized,
    E: Trap<AccessError>,
{
    fn source<R>(elements: &[R], offset: u32, len: u32) -> Option<&[R]> {
        elements
            .get(usize::try_from(offset).ok()?..)
            .and_then(|remaining| remaining.get(..usize::try_from(len).ok()?))
    }

    fn inner<R: TableElement>(
        dst: &(impl Table<R> + ?Sized),
        dst_idx: u32,
        src_idx: u32,
        len: u32,
        src: &[R],
    ) -> BoundsCheck<()> {
        dst.clone_from_slice(dst_idx, source(src, src_idx, len).ok_or(BoundsCheckError)?)
    }

    let dst_idx = table_idx as u32;
    let len = length as u32;
    inner(table, dst_idx, segment_idx as u32, len, element_segment)
        .map_err(|BoundsCheckError| trap_access_error(TABLE, dst_idx.saturating_add(len), frame))
}

/// This implements the [`table.copy`] instruction in the typical case where the source and
/// destination is within the same table.
///
/// For more information, see the documentation for the [`Table::clone_within()`] method.
///
/// [`table.copy`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
pub fn copy_within<const TABLE: u32, R, T, E>(
    table: &T,
    dst_idx: i32,
    src_idx: i32,
    length: i32,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    R: TableElement,
    T: Table<R> + ?Sized,
    E: Trap<AccessError>,
{
    let dst_idx = dst_idx as u32;
    let src_idx = src_idx as u32;
    let len = length as u32;
    table
        .clone_within(dst_idx, src_idx, len)
        .map_err(|BoundsCheckError| trap_access_error(TABLE, dst_idx.saturating_add(len), frame))
}

/// This implements the [`table.copy`] instruction in the case where the source and destination
/// tables differ.
///
/// For more information, see the documentation for the [`TableExt::clone_from()`] method.
///
/// [`table.copy`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
/// [`TableExt::clone_from()`]: crate::TableExt::clone_from()
pub fn copy<const DST_TBL: u32, const SRC_TBL: u32, R, Dst, Src, E>(
    dst: &Dst,
    src: &Src,
    dst_idx: i32,
    src_idx: i32,
    len: i32,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    R: TableElement,
    Dst: crate::TableExt<R> + ?Sized,
    Src: Table<R> + ?Sized,
    E: Trap<AccessError>,
{
    let dst_idx = dst_idx as u32;
    let src_idx = src_idx as u32;
    let len = len as u32;
    dst.clone_from(src, dst_idx, src_idx, len)
        .map_err(|BoundsCheckError| {
            let (table, index) = match src_idx.checked_add(len) {
                Some(sum) if sum < src.size() => (DST_TBL, dst_idx),
                _ => (SRC_TBL, src_idx),
            };

            trap_access_error(table, index, frame)
        })
}

/// This implements the [`table.fill`] instruction.
///
/// For more information, see the documentation for the [`Table::fill()`] method.
///
/// [`table.fill`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
pub fn fill<const TABLE: u32, R, T, E>(
    table: &T,
    idx: i32,
    elem: R,
    len: i32,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    R: TableElement,
    T: Table<R> + ?Sized,
    E: Trap<AccessError>,
{
    let index = idx as u32;
    let length = len as u32;
    table
        .fill(index, length, elem)
        .map_err(|BoundsCheckError| trap_access_error(TABLE, index.saturating_add(length), frame))
}

/// This implements the [`table.set`] instruction.
///
/// For more information, see the documentation for the [`Table::set()`] method.
///
/// [`table.set`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
pub fn set<const TABLE: u32, R, T, E>(
    table: &T,
    idx: i32,
    elem: R,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    R: TableElement,
    T: Table<R> + ?Sized,
    E: Trap<AccessError>,
{
    table
        .set(idx as u32, elem)
        .map_err(|BoundsCheckError| trap_access_error(TABLE, idx as u32, frame))
}

/// This implements the [`table.get`] instruction.
///
/// For more information, see the documentation for the [`Table::get()`] method.
///
/// [`table.get`]: https://webassembly.github.io/spec/core/syntax/instructions.html#table-instructions
pub fn get<const TABLE: u32, R, T, E>(
    table: &T,
    idx: i32,
    frame: Option<&'static WasmFrame>,
) -> Result<R, E>
where
    R: TableElement,
    T: Table<R> + ?Sized,
    E: Trap<AccessError>,
{
    table
        .get(idx as u32)
        .map_err(|BoundsCheckError| trap_access_error(TABLE, idx as u32, frame))
}
