//! Implements the [`Memory`] trait for [`Rc`].

use crate::{Address, BoundsCheck, Memory};
use alloc::rc::Rc;

impl<I: Address, M: Memory<I> + ?Sized> Memory<I> for Rc<M> {
    fn size(&self) -> I {
        M::size(self)
    }

    fn maximum(&self) -> I {
        M::maximum(self)
    }

    fn grow(&self, delta: I) -> I {
        M::grow(self, delta)
    }

    fn copy_to_slice(&self, addr: I, dst: &mut [u8]) -> BoundsCheck<()> {
        M::copy_to_slice(self, addr, dst)
    }

    fn copy_from_slice(&self, addr: I, src: &[u8]) -> BoundsCheck<()> {
        M::copy_from_slice(self, addr, src)
    }

    /// Attempts to get a mutable reference to the linear memory contents.
    ///
    /// # Panics
    ///
    /// Panics if other references to the linear memory exist.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        M::as_bytes_mut(Rc::get_mut(self).expect("other references to the linear memory exist"))
    }

    fn copy_within(&self, dst_addr: I, src_addr: I, len: I) -> BoundsCheck<()> {
        M::copy_within(self, dst_addr, src_addr, len)
    }

    fn fill(&self, addr: I, len: I, value: u8) -> BoundsCheck<()> {
        M::fill(self, addr, len, value)
    }

    fn to_boxed_bytes(&self, idx: I, len: I) -> BoundsCheck<alloc::boxed::Box<[u8]>> {
        M::to_boxed_bytes(self, idx, len)
    }

    fn i8_load(&self, addr: I) -> BoundsCheck<i8> {
        M::i8_load(self, addr)
    }

    fn i16_load(&self, addr: I) -> BoundsCheck<i16> {
        M::i16_load(self, addr)
    }

    fn i32_load(&self, addr: I) -> BoundsCheck<i32> {
        M::i32_load(self, addr)
    }

    fn i64_load(&self, addr: I) -> BoundsCheck<i64> {
        M::i64_load(self, addr)
    }

    fn i8_store(&self, addr: I, value: i8) -> BoundsCheck<()> {
        M::i8_store(self, addr, value)
    }

    fn i16_store(&self, addr: I, value: i16) -> BoundsCheck<()> {
        M::i16_store(self, addr, value)
    }

    fn i32_store(&self, addr: I, value: i32) -> BoundsCheck<()> {
        M::i32_store(self, addr, value)
    }

    fn i64_store(&self, addr: I, value: i64) -> BoundsCheck<()> {
        M::i64_store(self, addr, value)
    }
}

impl<I: Address, M: crate::MemoryExt<I> + ?Sized> crate::MemoryExt<I> for Rc<M> {
    fn copy_from<Src>(&self, src: &Src, dst_addr: I, src_addr: I, len: I) -> BoundsCheck<()>
    where
        Src: Memory<I> + ?Sized,
    {
        M::copy_from(self, src, dst_addr, src_addr, len)
    }
}
