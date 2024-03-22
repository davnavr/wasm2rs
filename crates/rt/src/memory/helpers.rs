//! Helper functions for performing memory accesses.
//!
//! Calls to these functions are generated as part of the `wasm2rs` translation process.

use crate::memory::{AccessError, AccessResult, BoundsCheckError, Memory32};
use crate::trap::Trap;

/// This implements the [`memory.size`] instruction.
///
/// For more information, see the documentation for the [`Memory32::size()`] method.
///
/// [`memory.size`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "memory.size")]
pub fn size<Mem: Memory32 + ?Sized>(mem: &Mem) -> i32 {
    mem.size() as i32
}

/// This implements the [`memory.grow`] instruction.
///
/// For more information, see the documentation for the [`Memory32::grow()`] method.
///
/// [`memory.grow`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "memory.grow")]
pub fn grow<Mem: Memory32 + ?Sized>(mem: &Mem, delta: i32) -> i32 {
    mem.grow(delta as u32) as i32
}

/// This implements the [`memory.init`] instruction and [active data segment initialization].
///
/// For more information, see the documentation for the [`Memory32::copy_from_slice()`] method.
///
/// [active data segment initialization]: https://webassembly.github.io/spec/core/syntax/modules.html#data-segments
/// [`memory.init`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn init<const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    data: &[u8],
    memory_offset: i32,
    segment_offset: i32,
    length: i32,
    trap: &Tr,
) -> Result<(), Tr::Repr>
where
    Mem: Memory32 + ?Sized,
    Tr: Trap + ?Sized,
{
    fn get_data_segment(data: &[u8], offset: u32, length: u32) -> Option<&[u8]> {
        let offset = usize::try_from(offset).ok()?;
        let length = usize::try_from(length).ok()?;
        data.get(offset..)?.get(..length)
    }

    let address = memory_offset as u32;
    let size = length as u32;
    get_data_segment(data, segment_offset as u32, size)
        .ok_or(BoundsCheckError)
        .and_then(|src| mem.copy_from_slice(address, src))
        .map_err(|BoundsCheckError| AccessError::Other { size }.trap(MEMORY, address.into(), trap))
}

/// This implements the [`memory.copy`] instruction.
///
/// For more information, see the documentation for the [`Memory32::copy_within()`] method.
///
/// [`memory.copy`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn copy_within<const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    dst_addr: i32,
    src_addr: i32,
    len: i32,
    trap: &Tr,
) -> Result<(), Tr::Repr>
where
    Mem: Memory32 + ?Sized,
    Tr: Trap + ?Sized,
{
    let dst_addr = dst_addr as u32;
    let src_addr = src_addr as u32;
    let size = len as u32;
    match mem.copy_within(dst_addr, src_addr, size) {
        Ok(()) => Ok(()),
        Err(BoundsCheckError) => {
            let address = match src_addr.checked_add(size) {
                None => u64::from(src_addr) + u64::from(size),
                Some(addr) if mem.size() * crate::memory::PAGE_SIZE <= addr => addr.into(),
                _ => u64::from(dst_addr) + u64::from(size),
            };

            Err(AccessError::Other { size }.trap(MEMORY, address, trap))
        }
    }
}

/// This implements the [`memory.copy`] instruction in the case where the source and destination
/// memories differ.
///
/// For more information, see the documentation for the [`Memory32::copy_from()`] method.
///
/// [`memory.copy`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn copy<const DST_MEM: u32, const SRC_MEM: u32, Dst, Src, Tr>(
    dst: &Dst,
    src: &Src,
    dst_addr: i32,
    src_addr: i32,
    len: i32,
    trap: &Tr,
) -> Result<(), Tr::Repr>
where
    Dst: Memory32,
    Src: Memory32 + ?Sized,
    Tr: Trap + ?Sized,
{
    let dst_addr = dst_addr as u32;
    let src_addr = src_addr as u32;
    let size = len as u32;
    match dst.copy_from(src, dst_addr, src_addr, size) {
        Ok(()) => Ok(()),
        Err(BoundsCheckError) => {
            let (mem, address) = match src_addr.checked_add(size) {
                None => (SRC_MEM, u64::from(src_addr) + u64::from(size)),
                Some(addr) if src.size() * crate::memory::PAGE_SIZE <= addr => {
                    (SRC_MEM, addr.into())
                }
                _ => (DST_MEM, u64::from(dst_addr) + u64::from(size)),
            };

            Err(AccessError::Other { size }.trap(mem, address, trap))
        }
    }
}

/// Calculates an address from adding static offset to a dynamic address operand.
///
/// This implements the calculation of the [*effective address*] for WebAssembly memory instructions.
///
/// [*effective address*]: https://webassembly.github.io/spec/core/syntax/instructions.html#memory-instructions
const fn address<const OFFSET: u32>(addr: i32) -> AccessResult<u32> {
    // TODO: See if `i32::overflowing_add` or `i64` arithmetic works better here
    if let Some(effective) = OFFSET.checked_add(addr as u32) {
        Ok(effective)
    } else {
        Err(AccessError::AddressOverflow { offset: OFFSET })
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
pub fn i8_load<const OFFSET: u32, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    trap: &Tr,
) -> Result<i8, Tr::Repr>
where
    Mem: Memory32 + ?Sized,
    Tr: Trap + ?Sized,
{
    fn load<const OFFSET: u32>(mem: &(impl Memory32 + ?Sized), addr: i32) -> AccessResult<i8> {
        mem.i8_load(address::<OFFSET>(addr)?)
            .map_err(|BoundsCheckError| AccessError::I8)
    }

    load::<OFFSET>(mem, addr)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
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
pub fn i16_load<const OFFSET: u32, const ALIGN: u8, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    trap: &Tr,
) -> Result<i16, Tr::Repr>
where
    Mem: Memory32,
    Tr: Trap + ?Sized,
{
    fn load<const OFFSET: u32, const ALIGN: u8>(
        mem: &impl Memory32,
        addr: i32,
    ) -> AccessResult<i16> {
        mem.i16_load::<ALIGN>(address::<OFFSET>(addr)?)
            .map_err(|BoundsCheckError| AccessError::I16)
    }

    load::<OFFSET, ALIGN>(mem, addr)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
}

/// This implements the [`i32.load`] instruction.
///
/// For more information, see the documentation for the [`Memory32::i32_load()`] method.
///
/// [`i32.load`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.load")]
pub fn i32_load<const OFFSET: u32, const ALIGN: u8, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    trap: &Tr,
) -> Result<i32, Tr::Repr>
where
    Mem: Memory32,
    Tr: Trap + ?Sized,
{
    fn load<const OFFSET: u32, const ALIGN: u8>(
        mem: &impl Memory32,
        addr: i32,
    ) -> AccessResult<i32> {
        mem.i32_load::<ALIGN>(address::<OFFSET>(addr)?)
            .map_err(|BoundsCheckError| AccessError::I32)
    }

    load::<OFFSET, ALIGN>(mem, addr)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
}

/// This implements the [`i64.load`], `i64.load32_s` and `i64.load32_u` instructions.
///
/// For more information, see the documentation for the [`Memory32::i64_load()`] method.
///
/// [`i64.load`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i64.load")]
#[doc(alias = "i64.load32_s")]
#[doc(alias = "i64.load32_u")]
pub fn i64_load<const OFFSET: u32, const ALIGN: u8, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    trap: &Tr,
) -> Result<i64, Tr::Repr>
where
    Mem: Memory32,
    Tr: Trap + ?Sized,
{
    fn load<const OFFSET: u32, const ALIGN: u8>(
        mem: &impl Memory32,
        addr: i32,
    ) -> AccessResult<i64> {
        mem.i64_load::<ALIGN>(address::<OFFSET>(addr)?)
            .map_err(|BoundsCheckError| AccessError::I64)
    }

    load::<OFFSET, ALIGN>(mem, addr)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
}

/// This implements the [`iXX.store8`] family of instructions.
///
/// For more information, see the documentation for the [`Memory32::i8_store()`] method.
///
/// [`iXX.store8`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.store16")]
#[doc(alias = "i64.store16")]
pub fn i8_store<const OFFSET: u32, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    value: i8,
    trap: &Tr,
) -> Result<(), Tr::Repr>
where
    Mem: Memory32,
    Tr: Trap + ?Sized,
{
    fn store<const OFFSET: u32>(mem: &impl Memory32, addr: i32, value: i8) -> AccessResult<()> {
        mem.i8_store(address::<OFFSET>(addr)?, value)
            .map_err(|BoundsCheckError| AccessError::I8)
    }

    store::<OFFSET>(mem, addr, value)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
}

/// This implements the [`iXX.store16`] family of instructions.
///
/// For more information, see the documentation for the [`Memory32::i16_store()`] method.
///
/// [`iXX.store16`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.store16")]
#[doc(alias = "i64.store16")]
pub fn i16_store<const OFFSET: u32, const ALIGN: u8, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    value: i16,
    trap: &Tr,
) -> Result<(), Tr::Repr>
where
    Mem: Memory32,
    Tr: Trap + ?Sized,
{
    fn store<const OFFSET: u32, const ALIGN: u8>(
        mem: &impl Memory32,
        addr: i32,
        value: i16,
    ) -> AccessResult<()> {
        mem.i16_store::<ALIGN>(address::<OFFSET>(addr)?, value)
            .map_err(|BoundsCheckError| AccessError::I16)
    }

    store::<OFFSET, ALIGN>(mem, addr, value)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
}

/// This implements the [`i32.store`] and `i64.store32` instructions.
///
/// For more information, see the documentation for the [`Memory32::i32_store()`] method.
///
/// [`i32.store`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i32.store")]
pub fn i32_store<const OFFSET: u32, const ALIGN: u8, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    value: i32,
    trap: &Tr,
) -> Result<(), Tr::Repr>
where
    Mem: Memory32,
    Tr: Trap + ?Sized,
{
    fn store<const OFFSET: u32, const ALIGN: u8>(
        mem: &impl Memory32,
        addr: i32,
        value: i32,
    ) -> AccessResult<()> {
        mem.i32_store::<ALIGN>(address::<OFFSET>(addr)?, value)
            .map_err(|BoundsCheckError| AccessError::I32)
    }

    store::<OFFSET, ALIGN>(mem, addr, value)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
}

/// This implements the [`i64.store`] instruction.
///
/// For more information, see the documentation for the [`Memory32::i64_store()`] method.
///
/// [`i64.store`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
#[doc(alias = "i64.store")]
pub fn i64_store<const OFFSET: u32, const ALIGN: u8, const MEMORY: u32, Mem, Tr>(
    mem: &Mem,
    addr: i32,
    value: i64,
    trap: &Tr,
) -> Result<(), Tr::Repr>
where
    Mem: Memory32,
    Tr: Trap + ?Sized,
{
    fn store<const OFFSET: u32, const ALIGN: u8>(
        mem: &impl Memory32,
        addr: i32,
        value: i64,
    ) -> AccessResult<()> {
        mem.i64_store::<ALIGN>(address::<OFFSET>(addr)?, value)
            .map_err(|BoundsCheckError| AccessError::I64)
    }

    store::<OFFSET, ALIGN>(mem, addr, value)
        .map_err(|err| err.trap(MEMORY, u64::from(addr as u32) + u64::from(OFFSET), trap))
}
