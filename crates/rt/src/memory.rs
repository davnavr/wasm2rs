//! Implementation for WebAssembly linear memory.

#[cfg(feature = "alloc")]
mod allocation;

#[cfg(feature = "alloc")]
mod heap;

#[cfg(feature = "alloc")]
pub use heap::HeapMemory32;

mod empty;

pub use empty::EmptyMemory;

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

/// Describes what kind of value was being read or written in a [`MemoryAccess`].
///
/// [`MemoryAccess`]: crate::trap::MemoryAccess
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

impl MemoryAccessPointee {
    fn other_with_size(size: usize) -> Self {
        Self::Other {
            size: u16::try_from(size)
                .ok()
                .and_then(core::num::NonZeroU16::new),
        }
    }
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

impl MemoryAccessError {
    /// Generates a trap for this invalid memory access.
    ///
    /// See the documentation for the [`trap::MemoryAccess`] struct for more information.
    ///
    /// [`trap::MemoryAccess`]: crate::trap::MemoryAccess
    pub fn trap<TR>(self, memory: u32, bound: u64, trap: &TR) -> TR::Repr
    where
        TR: crate::trap::Trap + ?Sized,
    {
        trap.trap(crate::trap::TrapCode::MemoryBoundsCheck(
            crate::trap::MemoryAccess {
                pointee: self.pointee,
                memory,
                bound,
                address: self.address.into(),
            },
        ))
    }
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

macro_rules! unaligned_integer_accesses {
    {
        $($int:ty => $pointee:ident : $load:ident / $store:ident;)*
    } => {$(
        fn $load<M: Memory32 + ?Sized>(mem: &M, addr: u32) -> AccessResult<$int> {
            let mut dst = [0u8; core::mem::size_of::<$int>()];
            match mem.copy_to_slice(addr, &mut dst) {
                Ok(()) => Ok(<$int>::from_le_bytes(dst)),
                Err(mut e) => {
                    e.pointee = MemoryAccessPointee::$pointee;
                    Err(e)
                }
            }
        }

        fn $store<M: Memory32 + ?Sized>(mem: &M, addr: u32, value: $int) -> AccessResult<()> {
            match mem.copy_from_slice(addr, &value.to_le_bytes()) {
                Ok(()) => Ok(()),
                Err(mut e) => {
                    e.pointee = MemoryAccessPointee::$pointee;
                    Err(e)
                }
            }
        }
    )*};
}

unaligned_integer_accesses! {
    i16 => I16 : unaligned_i16_load / unaligned_i16_store;
    i32 => I32 : unaligned_i32_load / unaligned_i32_store;
    i64 => I64 : unaligned_i64_load / unaligned_i64_store;
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
            Bound::Unbounded => (self.size() * PAGE_SIZE).wrapping_sub(1),
        };

        if start_addr > end_addr {
            return Ok(Default::default());
        }

        let mut slice =
            alloc::vec![0u8; usize::try_from(end_addr - start_addr + 1).unwrap_or(usize::MAX)];

        self.copy_to_slice(start_addr, &mut slice)?;
        Ok(slice.into_boxed_slice())
    }

    /// Loads the value of the byte stored at the given address.
    fn i8_load(&self, addr: u32) -> AccessResult<i8> {
        let mut dst = 0u8;
        match self.copy_to_slice(addr, core::slice::from_mut(&mut dst)) {
            Ok(()) => Ok(dst as i8),
            Err(mut err) => {
                err.pointee = MemoryAccessPointee::I8;
                Err(err)
            }
        }
    }

    /// Loads a potentially aligned 32-bit integer from the given address.
    fn i16_load<const A: u8>(&self, addr: u32) -> AccessResult<i16> {
        unaligned_i16_load(self, addr)
    }

    /// Loads a potentially aligned 32-bit integer from the given address.
    fn i32_load<const A: u8>(&self, addr: u32) -> AccessResult<i32> {
        unaligned_i32_load(self, addr)
    }

    /// Loads a potentially aligned 64-bit integer from the given address.
    fn i64_load<const A: u8>(&self, addr: u32) -> AccessResult<i64> {
        unaligned_i64_load(self, addr)
    }

    /// Writes into the byte at the given address.
    fn i8_store(&self, addr: u32, value: i8) -> AccessResult<()> {
        self.copy_from_slice(addr, &[value as u8])
    }

    /// Stores a potentially aligned 16-bit integer into the given address.
    fn i16_store<const A: u8>(&self, addr: u32, value: i16) -> AccessResult<()> {
        unaligned_i16_store(self, addr, value)
    }

    /// Stores a potentially aligned 32-bit integer into the given address.
    fn i32_store<const A: u8>(&self, addr: u32, value: i32) -> AccessResult<()> {
        unaligned_i32_store(self, addr, value)
    }

    /// Stores a potentially aligned 64-bit integer into the given address.
    fn i64_store<const A: u8>(&self, addr: u32, value: i64) -> AccessResult<()> {
        unaligned_i64_store(self, addr, value)
    }
}

//pub trait UnsharedMemory32: Memory32 + core::ops::Deref<Target = [u8]> + core::ops::DerefMut8 where Self: !Sync {}

/// This implements the [`memory.size`] instruction.
///
/// For more information, see the documentation for the [`Memory32::size()`] method.
///
/// [`memory.size`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "memory.size")]
pub fn size<M: Memory32 + ?Sized>(mem: &M) -> i32 {
    mem.size() as i32
}

/// This implements the [`memory.grow`] instruction.
///
/// For more information, see the documentation for the [`Memory32::grow()`] method.
///
/// [`memory.grow`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "memory.grow")]
pub fn grow<M: Memory32 + ?Sized>(mem: &M, delta: i32) -> i32 {
    mem.grow(delta as u32) as i32
}

/// This implements the [`memory.init`] instruction and [active data segment initialization].
///
/// For more information, see the documentation for the [`Memory32::copy_from_slice()`] method.
///
/// [active data segment initialization]: https://webassembly.github.io/spec/core/syntax/modules.html#data-segments
/// [`memory.init`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn init<const IDX: u32, M, TR>(
    mem: &M,
    data: &[u8],
    memory_offset: i32,
    segment_offset: i32,
    length: i32,
    trap: &TR,
) -> Result<(), TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    fn get_data_segment(data: &[u8], offset: u32, length: u32) -> Option<&[u8]> {
        let offset = usize::try_from(offset).ok()?;
        let length = usize::try_from(length).ok()?;
        data.get(offset..)?.get(..length)
    }

    let result = if let Some(src) = get_data_segment(data, segment_offset as u32, length as u32) {
        mem.copy_from_slice(memory_offset as u32, src)
    } else {
        Err(MemoryAccessError {
            address: memory_offset as u32,
            pointee: MemoryAccessPointee::Other {
                size: u16::try_from(length as u32)
                    .ok()
                    .and_then(core::num::NonZeroU16::new),
            },
        })
    };

    match result {
        Ok(()) => Ok(()),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`iXX.load8_s` and `iXX.load8_u`] family of instructions.
///
/// For more information, see the documentation for the [`Memory32::i8_load()`] method.
///
/// [`iXX.load8_s` and `iXX.load8_u`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.load8_s")]
#[doc(alias = "i32.load8_u")]
#[doc(alias = "i64.load8_s")]
#[doc(alias = "i64.load8_u")]
pub fn i8_load<const IDX: u32, M, TR>(mem: &M, addr: i32, trap: &TR) -> Result<i8, TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i8_load(addr as u32) {
        Ok(value) => Ok(value),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`iXX.load16_s` and `iXX.load16_u`] family of instructions.
///
/// For more information, see the documentation for the [`Memory32::i16_load()`] method.
///
/// [`iXX.load16_s` and `iXX.load16_u`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.load16_s")]
#[doc(alias = "i32.load16_u")]
#[doc(alias = "i64.load16_s")]
#[doc(alias = "i64.load16_u")]
pub fn i16_load<const A: u8, const IDX: u32, M, TR>(
    mem: &M,
    addr: i32,
    trap: &TR,
) -> Result<i16, TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i16_load::<A>(addr as u32) {
        Ok(value) => Ok(value),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`i32.load`] instruction.
///
/// For more information, see the documentation for the [`Memory32::i32_load()`] method.
///
/// [`i32.load`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.load")]
pub fn i32_load<const A: u8, const IDX: u32, M, TR>(
    mem: &M,
    addr: i32,
    trap: &TR,
) -> Result<i32, TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i32_load::<A>(addr as u32) {
        Ok(value) => Ok(value),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`i64.load`], `i64.load32_s` and `i64.load32_u` instructions.
///
/// For more information, see the documentation for the [`Memory32::i64_load()`] method.
///
/// [`i64.load`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i64.load")]
#[doc(alias = "i64.load32_s")]
#[doc(alias = "i64.load32_u")]
pub fn i64_load<const A: u8, const IDX: u32, M, TR>(
    mem: &M,
    addr: i32,
    trap: &TR,
) -> Result<i64, TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i64_load::<A>(addr as u32) {
        Ok(value) => Ok(value),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`iXX.store8`] family of instructions.
///
/// For more information, see the documentation for the [`Memory32::i8_store()`] method.
///
/// [`iXX.store8`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.store16")]
#[doc(alias = "i64.store16")]
pub fn i8_store<const IDX: u32, M, TR>(
    mem: &M,
    addr: i32,
    value: i8,
    trap: &TR,
) -> Result<(), TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i8_store(addr as u32, value) {
        Ok(()) => Ok(()),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`iXX.store16`] family of instructions.
///
/// For more information, see the documentation for the [`Memory32::i16_store()`] method.
///
/// [`iXX.store16`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.store16")]
#[doc(alias = "i64.store16")]
pub fn i16_store<const A: u8, const IDX: u32, M, TR>(
    mem: &M,
    addr: i32,
    value: i16,
    trap: &TR,
) -> Result<(), TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i16_store::<A>(addr as u32, value) {
        Ok(()) => Ok(()),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`i32.store`] and `i64.store32` instructions.
///
/// For more information, see the documentation for the [`Memory32::i32_store()`] method.
///
/// [`i32.store`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.store")]
pub fn i32_store<const A: u8, const IDX: u32, M, TR>(
    mem: &M,
    addr: i32,
    value: i32,
    trap: &TR,
) -> Result<(), TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i32_store::<A>(addr as u32, value) {
        Ok(()) => Ok(()),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

/// This implements the [`i64.store`] instruction.
///
/// For more information, see the documentation for the [`Memory32::i64_store()`] method.
///
/// [`i64.store`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i64.store")]
pub fn i64_store<const A: u8, const IDX: u32, M, TR>(
    mem: &M,
    addr: i32,
    value: i64,
    trap: &TR,
) -> Result<(), TR::Repr>
where
    M: Memory32 + ?Sized,
    TR: crate::trap::Trap + ?Sized,
{
    match mem.i64_store::<A>(addr as u32, value) {
        Ok(()) => Ok(()),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}

struct DisplaySize(u32);

impl core::fmt::Debug for DisplaySize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} * {}", PAGE_SIZE, self.0)
    }
}
