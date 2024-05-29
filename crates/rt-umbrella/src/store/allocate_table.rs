use crate::table::{NullableTableElement, TableElement};
use crate::trap::Trap;

/// Error type used when a call to [`AllocateTable::allocate_null()`] or
/// [`AllocateTable::allocate_with()`] fails.
///
/// See also the [`table::AllocationError`] struct.
///
/// [`table::AllocationError`]: crate::table::AllocationError
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocateTableError {
    table: u32,
    error: crate::table::AllocationError,
}

impl AllocateTableError {
    fn new(table: u32, error: crate::table::AllocationError) -> Self {
        Self { table, error }
    }

    /// Gets the index of the table that could not be allocated.
    pub fn table(&self) -> u32 {
        self.table
    }
}

impl core::fmt::Display for AllocateTableError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "table #{} {}", self.table, self.error)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AllocateTableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl From<AllocateTableError> for crate::table::AllocationError {
    fn from(error: AllocateTableError) -> Self {
        error.error
    }
}

/// Trait used for [allocating WebAssembly tables].
///
/// [allocating WebAssembly tables]: https://webassembly.github.io/spec/core/exec/modules.html#tables
pub trait AllocateTable<E: TableElement>: Sized {
    /// The table instance.
    type Table: crate::table::Table<Element = E>;

    /// Allocates an empty table, with the given minimum and maximum number of elements.
    fn allocate_null<R: Trap<AllocateTableError>>(
        self,
        table: u32,
        minimum: u32,
        maximum: u32,
    ) -> Result<Self::Table, R>
    where
        E: NullableTableElement,
    {
        Self::allocate_with(self, table, E::NULL, minimum, maximum)
    }

    /// Allocates the table, with the given minimum and maximum number of elements, as well as an
    /// element to initially [`fill()`] the table with.
    ///
    /// [`fill()`]: crate::table::Table::fill()
    fn allocate_with<R: Trap<AllocateTableError>>(
        self,
        table: u32,
        init: E,
        minimum: u32,
        maximum: u32,
    ) -> Result<Self::Table, R>;
}

/// Implements the [`AllocateTable`] trait by calling [`HeapTable::<E>::with_limits()`].
///
/// [`HeapTable::<E>::with_limits()`]: crate::table::HeapTable::with_limits();
#[derive(Clone, Copy, Default)]
#[cfg(feature = "alloc")]
pub struct AllocateHeapTable<E: NullableTableElement> {
    _marker: core::marker::PhantomData<fn() -> crate::table::HeapTable<E>>,
}

#[cfg(feature = "alloc")]
fn allocate_heap_table<E, R>(
    table: u32,
    minimum: u32,
    maximum: u32,
) -> Result<crate::table::HeapTable<E>, R>
where
    E: NullableTableElement,
    R: Trap<AllocateTableError>,
{
    crate::table::HeapTable::with_limits(minimum, maximum)
        .map_err(|error| R::trap(AllocateTableError::new(table, error), None))
}

#[cfg(feature = "alloc")]
impl<E: NullableTableElement> AllocateTable<E> for AllocateHeapTable<E> {
    type Table = crate::table::HeapTable<E>;

    fn allocate_null<R: Trap<AllocateTableError>>(
        self,
        table: u32,
        minimum: u32,
        maximum: u32,
    ) -> Result<Self::Table, R> {
        allocate_heap_table::<E, R>(table, minimum, maximum)
    }

    fn allocate_with<R: Trap<AllocateTableError>>(
        self,
        table: u32,
        init: E,
        minimum: u32,
        maximum: u32,
    ) -> Result<Self::Table, R> {
        let mut table = allocate_heap_table::<E, R>(table, minimum, maximum)?;
        crate::table::Table::as_mut_slice(&mut table).fill(init);
        Ok(table)
    }
}

#[cfg(feature = "alloc")]
impl<E: NullableTableElement> core::fmt::Debug for AllocateHeapTable<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AllocateTable")
    }
}
