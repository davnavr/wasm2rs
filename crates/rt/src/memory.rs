//! Implementation for WebAssembly linear memory.

#[cfg(feature = "alloc")]
mod allocation;

#[cfg(feature = "alloc")]
mod heap;

#[cfg(feature = "alloc")]
pub use heap::HeapMemory32;

/// The size, in bytes, of a WebAssembly linear memory [page].
///
/// [page]: https://webassembly.github.io/spec/core/exec/runtime.html#page-size
pub const PAGE_SIZE: u32 = 65536;

/// A constant value used to indicate that a [`memory.grow`] operation failed.
///
/// [`memory.grow`]: Memory32::grow()
pub const MEMORY_GROW_FAILED: i32 = -1;

/// Error type used when the minimum required number of pages for a linear memory could not be
/// allocated.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocationError {
    size: u32,
}

impl core::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "could not allocate {} pages for memory", self.size)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AllocationError {}

/// Describes what kind of value was being read or written in a [`MemoryAccess`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum MemoryAccessPointee {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    V128,
    /// Used for other memory instructions (e.g. **`memory.copy`** or **`memory.fill`**).
    Other {
        /// The size, in bytes, of the read or write; or `None` if the size is too large to fit.
        size: Option<core::num::NonZeroU16>,
    },
}

impl core::fmt::Display for MemoryAccessPointee {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::I8 => f.write_str("i8"),
            Self::I16 => f.write_str("i16"),
            Self::I32 => f.write_str("i32"),
            Self::I64 => f.write_str("i64"),
            Self::F32 => f.write_str("f32"),
            Self::F64 => f.write_str("f64"),
            Self::V128 => f.write_str("v128"),
            Self::Other { size: Some(size) } => {
                if size.get() == 1u16 {
                    f.write_str("one byte")
                } else {
                    write!(f, "{size} bytes")
                }
            }
            Self::Other { size: None } => f.write_str("unknown type"),
        }
    }
}

/// Error type used when an attempt to read or write from a [linear memory] fails.
///
/// [linear memory]: Memory32
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MemoryAccessError {
    /// The address into the [linear memory] that was out-of-bounds.
    ///
    /// [linear memory]: Memory32
    pub address: u32,
    /// The type of value behind the address.
    pub pointee: MemoryAccessPointee,
}

impl core::fmt::Display for MemoryAccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "out-of-bounds access of {} at {:#010X}",
            self.pointee, self.address
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MemoryAccessError {}

/// Result type used for reads from or writes to [linear memory].
///
/// [linear memory]: Memory32
pub type AccessResult<T> = core::result::Result<T, MemoryAccessError>;

/// A [WebAssembly linear memory] with a 32-bit address space.
///
/// [WebAssembly linear memory]: https://webassembly.github.io/spec/core/syntax/modules.html#memories
pub trait Memory32 {
    /// Returns the size of the linear memory, in terms of the [`PAGE_SIZE`].
    ///
    /// This implements the [`memory.size`] instruction.
    ///
    /// [`memory.size`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
    fn size(&self) -> i32;

    /// Gets the maximum number of pages that this linear memory can have.
    fn limit(&self) -> u32;

    /// Increases the size of the linear memory by the specified number of [pages], and returns the old number of pages.
    ///
    /// This implements the [`memory.grow`] instruction.
    ///
    /// The default implementation for this method simply calls [`Memory32::size()`] of `delta` is
    /// `0`, and returns `-1` otherwise.
    ///
    /// # Errors
    ///
    /// If the size of the memory oculd not be increased, then `-1` is returned.
    ///
    /// [pages]: PAGE_SIZE
    /// [`memory.grow`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
    fn grow(&self, delta: i32) -> i32 {
        if delta == 0 {
            self.size()
        } else {
            MEMORY_GROW_FAILED
        }
    }

    /// Copies bytes from linear memory starting at the specified address into the given slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of addresses `addr..(addr + dst.len())` is not in bounds.
    fn copy_to_slice(&self, addr: u32, dst: &mut [u8]) -> AccessResult<()>;

    /// Copies bytes from the given slice into linear memory starting at the specified address.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of addresses `addr..(addr + dst.len())` is not in bounds.
    fn copy_from_slice(&self, addr: u32, src: &[u8]) -> AccessResult<()>;

    /// Allocates a new boxed slice, and copies the contents of this linear memory at the range of addresses into it.
    ///
    /// # Errors
    ///
    /// If the range of address is out-of-bounds, an error is returned.
    #[cfg(feature = "alloc")]
    fn to_boxed_slice<R>(&self, range: R) -> AccessResult<alloc::boxed::Box<[u8]>>
    where
        R: core::ops::RangeBounds<u32>,
        Self: Sized,
    {
        use core::ops::Bound;

        let start_addr = match range.start_bound() {
            Bound::Included(bound) => *bound,
            Bound::Excluded(bound) => bound.wrapping_add(1),
            Bound::Unbounded => 0,
        };

        let end_addr = match range.end_bound() {
            Bound::Included(bound) => *bound,
            Bound::Excluded(bound) => bound.wrapping_sub(1),
            Bound::Unbounded => ((self.size() as u32) * PAGE_SIZE).wrapping_sub(1),
        };

        if start_addr > end_addr {
            return Ok(Default::default());
        }

        let mut slice =
            alloc::vec![0u8; usize::try_from(end_addr - start_addr + 1).unwrap_or(usize::MAX)];
        self.copy_to_slice(start_addr, &mut slice)?;
        Ok(slice.into_boxed_slice())
    }
}

//pub trait UnsharedMemory32: Memory32 + core::ops::Deref<Target = [u8]> + core::ops::DerefMut8 where Self: !Sync {}

//fn i32_load

struct DisplaySize(u32);

impl core::fmt::Debug for DisplaySize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} * {}", PAGE_SIZE, self.0)
    }
}
