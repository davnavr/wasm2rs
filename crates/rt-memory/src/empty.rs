use crate::{BoundsCheck, BoundsCheckError};

/// A [`Memory`] implementation that always has a size of zero.
///
/// [`Memory`]: crate::Memory
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct EmptyMemory;

impl<I: crate::Address> crate::Memory<I> for EmptyMemory {
    fn size(&self) -> I {
        I::ZERO
    }

    fn maximum(&self) -> I {
        I::ZERO
    }

    fn grow(&self, _: I) -> I {
        I::GROW_FAILED
    }

    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut []
    }

    fn copy_from_slice(&self, addr: I, src: &[u8]) -> BoundsCheck<()> {
        if addr == I::ZERO && src.is_empty() {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }

    fn copy_to_slice(&self, addr: I, dst: &mut [u8]) -> BoundsCheck<()> {
        if addr == I::ZERO && dst.is_empty() {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }

    fn copy_within(&self, dst_addr: I, src_addr: I, len: I) -> BoundsCheck<()> {
        if dst_addr == I::ZERO && src_addr == I::ZERO && len == I::ZERO {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }

    fn i8_load(&self, addr: I) -> BoundsCheck<i8> {
        let _ = addr;
        Err(BoundsCheckError)
    }

    fn i8_store(&self, addr: I, value: i8) -> BoundsCheck<()> {
        let _ = addr;
        let _ = value;
        Err(BoundsCheckError)
    }

    fn i16_load(&self, addr: I) -> BoundsCheck<i16> {
        let _ = addr;
        Err(BoundsCheckError)
    }

    fn i16_store(&self, addr: I, value: i16) -> BoundsCheck<()> {
        let _ = addr;
        let _ = value;
        Err(BoundsCheckError)
    }

    fn i32_load(&self, addr: I) -> BoundsCheck<i32> {
        let _ = addr;
        Err(BoundsCheckError)
    }

    fn i32_store(&self, addr: I, value: i32) -> BoundsCheck<()> {
        let _ = addr;
        let _ = value;
        Err(BoundsCheckError)
    }

    fn i64_load(&self, addr: I) -> BoundsCheck<i64> {
        let _ = addr;
        Err(BoundsCheckError)
    }

    fn i64_store(&self, addr: I, value: i64) -> BoundsCheck<()> {
        let _ = addr;
        let _ = value;
        Err(BoundsCheckError)
    }

    #[cfg(feature = "alloc")]
    fn to_boxed_bytes(&self, idx: I, len: I) -> BoundsCheck<alloc::boxed::Box<[u8]>> {
        if idx == I::ZERO && len == I::ZERO {
            Ok(alloc::boxed::Box::default())
        } else {
            Err(BoundsCheckError)
        }
    }
}

impl<I: crate::Address> crate::MemoryExt<I> for EmptyMemory {}
