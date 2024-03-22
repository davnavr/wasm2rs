//! Implementation for WebAssembly linear memory.

#[cfg(feature = "alloc")]
mod allocation;

#[cfg(feature = "alloc")]
mod heap;

#[cfg(feature = "alloc")]
pub use heap::HeapMemory32;

mod empty;
mod helpers;

pub use empty::EmptyMemory;
pub use helpers::*;

/// The size, in bytes, of a WebAssembly linear memory [page].
///
/// [page]: https://webassembly.github.io/spec/core/exec/runtime.html#page-size
pub const PAGE_SIZE: u32 = 65536;

/// A constant value used to indicate that a [`memory.grow`] operation failed.
///
/// [`memory.grow`]: Memory32::grow()
const MEMORY_GROW_FAILED: u32 = -1i32 as u32;

/// Error type used when the minimum required number of pages for a linear memory could not be
/// allocated.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocationError {
    size: u32,
}

impl AllocationError {
    pub(crate) const fn with_size(size: u32) -> Self {
        Self { size }
    }
}

impl core::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "could not allocate {} pages for memory", self.size)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AllocationError {}

/// Error type used when an attempt to read or write from a [linear memory] fails.
///
/// [`MemoryAccess`]: crate::trap::MemoryAccess
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum AccessError {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    V128,
    /// Used for other memory instructions (e.g. **`memory.copy`** or **`memory.fill`**).
    Other {
        /// The size, in bytes, of the read or write.
        size: u32,
    },
    /// Calculation of the address of the access overflowed.
    AddressOverflow {
        /// The static offset that was added to to cause an overflow.
        offset: u32,
    },
}

impl AccessError {
    /// An access was attempted with the given size.
    ///
    /// This size value may be truncated.
    pub fn with_size(size: usize) -> Self {
        Self::Other {
            size: u32::try_from(size).unwrap_or(u32::MAX),
        }
    }

    /// Generates a trap for this invalid memory access.
    ///
    /// See the documentation for the [`TrapCode::MemoryBoundsCheck`] struct for more information.
    ///
    /// [`TrapCode::MemoryBoundsCheck`]: crate::trap::TrapCode::MemoryBoundsCheck
    pub fn trap<TR>(self, memory: u32, address: u64, trap: &TR) -> TR::Repr
    where
        TR: crate::trap::Trap + ?Sized,
    {
        trap.trap(crate::trap::TrapCode::MemoryBoundsCheck {
            source: self,
            memory,
            address,
        })
    }
}

impl core::fmt::Display for AccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("out-of-bounds access")?;
        match self {
            Self::I8 => f.write_str(" of i8"),
            Self::I16 => f.write_str(" of i16"),
            Self::I32 => f.write_str(" of i32"),
            Self::I64 => f.write_str(" of i64"),
            Self::F32 => f.write_str(" of f32"),
            Self::F64 => f.write_str(" of f64"),
            Self::V128 => f.write_str(" of v128"),
            Self::Other { size: 0 } => Ok(()),
            Self::Other { size: 1 } => f.write_str("of 1 byte"),
            Self::Other { size } => write!(f, " of {size} bytes"),
            Self::AddressOverflow { offset } => {
                write!(f, ", offset {offset} + address overflowed")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccessError {}

/// Result type used for reads from or writes to [linear memory].
///
/// [linear memory]: Memory32
pub type AccessResult<T> = core::result::Result<T, AccessError>;

/// Error type used when an address was out of bounds.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BoundsCheckError;

impl core::fmt::Display for BoundsCheckError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("out-of-bounds address")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BoundsCheckError {}

/// Result type used for validating that addresses are in bounds.
pub type BoundsCheck<T> = core::result::Result<T, BoundsCheckError>;

macro_rules! unaligned_integer_accesses {
    {
        $($int:ty : $load:ident / $store:ident;)*
    } => {$(
        fn $load<M: Memory32 + ?Sized>(mem: &M, addr: u32) -> BoundsCheck<$int> {
            let mut dst = [0u8; core::mem::size_of::<$int>()];
            match mem.copy_to_slice(addr, &mut dst) {
                Ok(()) => Ok(<$int>::from_le_bytes(dst)),
                Err(e) => Err(e),
            }
        }

        fn $store<M: Memory32 + ?Sized>(mem: &M, addr: u32, value: $int) -> BoundsCheck<()> {
            match mem.copy_from_slice(addr, &value.to_le_bytes()) {
                Ok(()) => Ok(()),
                Err(e) => Err(e),
            }
        }
    )*};
}

unaligned_integer_accesses! {
    i16 : unaligned_i16_load / unaligned_i16_store;
    i32 : unaligned_i32_load / unaligned_i32_store;
    i64 : unaligned_i64_load / unaligned_i64_store;
}

fn default_copy_from<Dst, Src>(
    dst: &Dst,
    src: &Src,
    dst_addr: u32,
    src_addr: u32,
    len: u32,
) -> BoundsCheck<()>
where
    Dst: Memory32 + ?Sized,
    Src: Memory32 + ?Sized,
{
    /// Limit on the number of bytes to copy at a time.
    const BUFFER_SIZE: u32 = 512;

    let mut buffer = [0u8; BUFFER_SIZE as usize];
    let mut written = 0u32;
    while let Some(buffer @ [_, ..]) = buffer.get_mut(..BUFFER_SIZE.min(len - written) as usize) {
        dst.copy_to_slice(dst_addr + written, buffer)?;
        src.copy_from_slice(src_addr + written, buffer)?;

        // `buffer.len() <= BUFFER_SIZE <= u32::MAX`
        #[allow(clippy::cast_possible_truncation)]
        {
            written += buffer.len() as u32;
        }
    }

    Ok(())
}

/// A [WebAssembly linear memory] with a 32-bit address space.
///
/// Some read and write operations take a constant alignment operation `A`, where the alignment is
/// 2 to the power of `A`.
///
/// [WebAssembly linear memory]: https://webassembly.github.io/spec/core/syntax/modules.html#memories
pub trait Memory32 {
    /// Returns the size of the linear memory, in terms of the [`PAGE_SIZE`].
    fn size(&self) -> u32;

    /// Gets the maximum number of pages that this linear memory can have.
    fn limit(&self) -> u32;

    /// Increases the size of the linear memory by the specified number of [pages], and returns the old number of pages.
    ///
    /// The default implementation for this method simply calls [`Memory32::size()`] of `delta` is
    /// `0`, and returns `-1` otherwise.
    ///
    /// # Errors
    ///
    /// If the size of the memory oculd not be increased, then `-1` is returned.
    ///
    /// [pages]: PAGE_SIZE
    fn grow(&self, delta: u32) -> u32 {
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
    fn copy_to_slice(&self, addr: u32, dst: &mut [u8]) -> BoundsCheck<()>;

    /// Copies bytes from the given slice into linear memory starting at the specified address.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of addresses `addr..(addr + dst.len())` is not in bounds.
    fn copy_from_slice(&self, addr: u32, src: &[u8]) -> BoundsCheck<()>;

    /// Copies bytes from the given linear memory into `self`.
    ///
    /// # Errors
    ///
    /// Returns an error if `addr + len` is not in bounds in either of the memories.
    fn copy_from<Src>(&self, src: &Src, dst_addr: u32, src_addr: u32, len: u32) -> BoundsCheck<()>
    where
        Src: Memory32 + ?Sized,
    {
        default_copy_from(self, src, dst_addr, src_addr, len)
    }

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
            Bound::Unbounded => (self.size() * PAGE_SIZE).wrapping_sub(1),
        };

        if start_addr > end_addr {
            return Ok(Default::default());
        }

        let mut slice =
            alloc::vec![0u8; usize::try_from(end_addr - start_addr + 1).unwrap_or(usize::MAX)];

        match self.copy_to_slice(start_addr, &mut slice) {
            Ok(()) => Ok(slice.into_boxed_slice()),
            Err(BoundsCheckError) => Err(AccessError::with_size(slice.len())),
        }
    }

    /// Loads the value of the byte stored at the given address.
    fn i8_load(&self, addr: u32) -> BoundsCheck<i8> {
        let mut dst = 0u8;
        match self.copy_to_slice(addr, core::slice::from_mut(&mut dst)) {
            Ok(()) => Ok(dst as i8),
            Err(e) => Err(e),
        }
    }

    /// Loads a potentially aligned 32-bit integer from the given address.
    fn i16_load<const ALIGN: u8>(&self, addr: u32) -> BoundsCheck<i16> {
        unaligned_i16_load(self, addr)
    }

    /// Loads a potentially aligned 32-bit integer from the given address.
    fn i32_load<const ALIGN: u8>(&self, addr: u32) -> BoundsCheck<i32> {
        unaligned_i32_load(self, addr)
    }

    /// Loads a potentially aligned 64-bit integer from the given address.
    fn i64_load<const ALIGN: u8>(&self, addr: u32) -> BoundsCheck<i64> {
        unaligned_i64_load(self, addr)
    }

    /// Writes into the byte at the given address.
    fn i8_store(&self, addr: u32, value: i8) -> BoundsCheck<()> {
        self.copy_from_slice(addr, &[value as u8])
    }

    /// Stores a potentially aligned 16-bit integer into the given address.
    fn i16_store<const ALIGN: u8>(&self, addr: u32, value: i16) -> BoundsCheck<()> {
        unaligned_i16_store(self, addr, value)
    }

    /// Stores a potentially aligned 32-bit integer into the given address.
    fn i32_store<const ALIGN: u8>(&self, addr: u32, value: i32) -> BoundsCheck<()> {
        unaligned_i32_store(self, addr, value)
    }

    /// Stores a potentially aligned 64-bit integer into the given address.
    fn i64_store<const ALIGN: u8>(&self, addr: u32, value: i64) -> BoundsCheck<()> {
        unaligned_i64_store(self, addr, value)
    }
}

//pub trait UnsharedMemory32: Memory32 + core::ops::Deref<Target = [u8]> + core::ops::DerefMut8 where Self: !Sync {}

struct DisplaySize(u32);

impl core::fmt::Debug for DisplaySize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} * {}", PAGE_SIZE, self.0)
    }
}
