//! Helper functions for performing memory accesses.
//!
//! Calls to these functions are generated as part of the `wasm2rs` translation process.

use crate::memory::{Memory32, MemoryAccessError, MemoryAccessPointee};
use crate::trap::Trap;

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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
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
    TR: Trap + ?Sized,
{
    match mem.i64_store::<A>(addr as u32, value) {
        Ok(()) => Ok(()),
        Err(err) => Err(err.trap(IDX, mem.size().into(), trap)),
    }
}
