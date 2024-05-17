//! Helper functions for performing memory accesses.
//!
//! Calls to these functions are generated as part of the `wasm2rs` translation process.

use crate::{AccessError, Address, BoundsCheck, BoundsCheckError, EffectiveAddress, Memory};
use wasm2rs_rt_core::{trace::WasmFrame, trap::Trap};

/// This implements the [`memory.size`] instruction.
///
/// For more information, see the documentation for the [`Memory::size()`] method.
///
/// [`memory.size`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn size<I: Address, M: Memory<I> + ?Sized>(mem: &M) -> I::Signed {
    mem.size().as_()
}

/// This implements the [`memory.grow`] instruction.
///
/// For more information, see the documentation for the [`Memory::grow()`] method.
///
/// [`memory.grow`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn grow<I: Address, M: Memory<I> + ?Sized>(mem: &M, delta: I::Signed) -> I::Signed {
    mem.grow(I::cast_from_signed(delta)).as_()
}

#[cold]
#[inline(never)]
fn trap_access_error<I, E>(
    memory: u32,
    address: EffectiveAddress<I>,
    frame: Option<&'static WasmFrame>,
) -> E
where
    I: Address,
    E: Trap<AccessError<I>>,
{
    E::trap(AccessError::new(memory, address), frame)
}

/// This implements the [`memory.init`] instruction and [active data segment initialization].
///
/// For more information, see the documentation for the [`Memory::copy_from_slice()`] method.
///
/// [active data segment initialization]: https://webassembly.github.io/spec/core/syntax/modules.html#data-segments
/// [`memory.init`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn init<const MEMORY: u32, I, M, E>(
    mem: &M,
    memory_offset: I::Signed,
    segment_offset: I::Signed,
    length: I::Signed,
    data_segment: &[u8],
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn source<I: Address>(segment: &[u8], offset: I, length: I) -> Option<&[u8]> {
        segment.get(offset.to_usize()?..)?.get(..length.to_usize()?)
    }

    fn inner<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        memory_offset: I,
        segment_offset: I,
        length: I,
        data: &[u8],
    ) -> BoundsCheck<()> {
        source(data, segment_offset, length)
            .ok_or(BoundsCheckError)
            .and_then(|src| mem.copy_from_slice(memory_offset, src))
    }

    let memory_offset = I::cast_from_signed(memory_offset);
    inner(
        mem,
        memory_offset,
        I::cast_from_signed(segment_offset),
        I::cast_from_signed(length),
        data_segment,
    )
    .map_err(|BoundsCheckError| trap_access_error(MEMORY, memory_offset.into(), frame))
}

/// This implements the [`memory.copy`] instruction in the typical case where the source and
/// destination is within the same linear memory.
///
/// For more information, see the documentation for the [`Memory::copy_within()`] method.
///
/// [`memory.copy`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn copy_within<const MEMORY: u32, I, M, E>(
    mem: &M,
    dst_addr: I::Signed,
    src_addr: I::Signed,
    len: I::Signed,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    let dst_addr = I::cast_from_signed(dst_addr);
    let src_addr = I::cast_from_signed(src_addr);
    let len = I::cast_from_signed(len);
    mem.copy_within(dst_addr, src_addr, len)
        .map_err(|BoundsCheckError| {
            trap_access_error(
                MEMORY,
                EffectiveAddress::<I>::with_offset(len, dst_addr),
                frame,
            )
        })
}

/// This implements the [`memory.copy`] instruction in the case where the source and destination
/// memories differ.
///
/// For more information, see the documentation for the [`Memory::copy_from()`] method.
///
/// [`memory.copy`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn copy<const DST_MEM: u32, const SRC_MEM: u32, I, Dst, Src, E>(
    dst: &Dst,
    src: &Src,
    dst_addr: I::Signed,
    src_addr: I::Signed,
    len: I::Signed,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    I: Address,
    Dst: Memory<I>,
    Src: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    let dst_addr = I::cast_from_signed(dst_addr);
    let src_addr = I::cast_from_signed(src_addr);
    let len = I::cast_from_signed(len);
    dst.copy_from(src, dst_addr, src_addr, len)
        .map_err(|BoundsCheckError| {
            let (memory, address) = match src_addr.checked_add(&len) {
                Some(oob) if oob < src.size() => (SRC_MEM, src_addr),
                _ => (DST_MEM, dst_addr),
            };

            trap_access_error(
                memory,
                EffectiveAddress::<I>::with_offset(len, address),
                frame,
            )
        })
}

/// This implements the [**i*nn*.load8_*sx***] instructions.
///
/// For more information, see the documentation for the [`Memory::i8_load()`] method.
///
/// [**i*nn*.load8_*sx***]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i8_load<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    frame: Option<&'static WasmFrame>,
) -> Result<i8, E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn load<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
    ) -> BoundsCheck<i8> {
        mem.i8_load(address.calculate()?)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    load(mem, address).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}

/// This implements the [**i*nn*.load16_*sx***] instructions.
///
/// For more information, see the documentation for the [`Memory::i16_load()`] method.
///
/// [**i*nn*.load16_*sx***]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i16_load<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    frame: Option<&'static WasmFrame>,
) -> Result<i16, E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn load<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
    ) -> BoundsCheck<i16> {
        mem.i16_load(address.calculate()?)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    load(mem, address).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}

/// This implements the [`i32.load`] instruction.
///
/// For more information, see the documentation for the [`Memory::i32_load()`] method.
///
/// [`i32.load`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i32_load<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    frame: Option<&'static WasmFrame>,
) -> Result<i32, E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn load<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
    ) -> BoundsCheck<i32> {
        mem.i32_load(address.calculate()?)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    load(mem, address).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}

/// This implements the [`i64.load` and **i64.load32_*sx***] instructions.
///
/// For more information, see the documentation for the [`Memory::i64_load()`] method.
///
/// [`i64.load` and **i64.load32_*sx***]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i64_load<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    frame: Option<&'static WasmFrame>,
) -> Result<i64, E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn load<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
    ) -> BoundsCheck<i64> {
        mem.i64_load(address.calculate()?)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    load(mem, address).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}

/// This implements the [**i*nn*.store8**] instructions.
///
/// For more information, see the documentation for the [`Memory::i8_store()`] method.
///
/// [**i*nn*.store8**]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i8_store<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    c: i8,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn store<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
        c: i8,
    ) -> BoundsCheck<()> {
        mem.i8_store(address.calculate()?, c)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    store(mem, address, c).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}

/// This implements the [**i*nn*.store16**] family of instructions.
///
/// For more information, see the documentation for the [`Memory::i16_store()`] method.
///
/// [**i*nn*.store16**]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i16_store<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    c: i16,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn store<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
        c: i16,
    ) -> BoundsCheck<()> {
        mem.i16_store(address.calculate()?, c)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    store(mem, address, c).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}

/// This implements the [`i32.store` and `i64.store32`] instructions.
///
/// For more information, see the documentation for the [`Memory::i32_store()`] method.
///
/// [`i32.store` and `i64.store32`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i32_store<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    c: i32,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn store<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
        c: i32,
    ) -> BoundsCheck<()> {
        mem.i32_store(address.calculate()?, c)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    store(mem, address, c).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}

/// This implements the [`i64.store`] instruction.
///
/// For more information, see the documentation for the [`Memory::i64_store()`] method.
///
/// [`i64.store`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory
pub fn i64_store<const MEMORY: u32, I, M, E>(
    mem: &M,
    offset: I::Signed,
    addr: I::Signed,
    c: i64,
    frame: Option<&'static WasmFrame>,
) -> Result<(), E>
where
    I: Address,
    M: Memory<I> + ?Sized,
    E: Trap<AccessError<I>>,
{
    fn store<I: Address>(
        mem: &(impl Memory<I> + ?Sized),
        address: EffectiveAddress<I>,
        c: i64,
    ) -> BoundsCheck<()> {
        mem.i64_store(address.calculate()?, c)
    }

    let address = EffectiveAddress::<I>::signed_with_offset(I::cast_from_signed(offset), addr);
    store(mem, address, c).map_err(|BoundsCheckError| trap_access_error(MEMORY, address, frame))
}
