use crate::{BoundsCheck, BoundsCheckError};

/// A [`Table`] implementation that always has a size of zero.
///
/// [`Table`]: crate::Table
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct EmptyTable;

impl crate::AnyTable for EmptyTable {
    fn size(&self) -> u32 {
        0
    }

    fn maximum(&self) -> u32 {
        0
    }

    fn grow(&self, delta: u32) -> u32 {
        if delta == 0 {
            0
        } else {
            crate::GROW_FAILED
        }
    }
}

impl<E: crate::TableElement> crate::Table<E> for EmptyTable {
    fn get(&self, idx: u32) -> BoundsCheck<E> {
        let _ = idx;
        Err(BoundsCheckError)
    }

    fn replace(&self, idx: u32, elem: E) -> BoundsCheck<E> {
        let _ = idx;
        let _ = elem;
        Err(BoundsCheckError)
    }

    fn as_mut_slice(&mut self) -> &mut [E] {
        &mut []
    }

    fn clone_from_slice(&self, idx: u32, src: &[E]) -> BoundsCheck<()> {
        if idx == 0 && src.is_empty() {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }

    fn clone_into_slice(&self, idx: u32, dst: &mut [E]) -> BoundsCheck<()> {
        if idx == 0 && dst.is_empty() {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }

    fn clone_within(&self, dst_idx: u32, src_idx: u32, len: u32) -> BoundsCheck<()> {
        if dst_idx == 0 && src_idx == 0 && len == 0 {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }

    fn fill(&self, idx: u32, len: u32, elem: E) -> BoundsCheck<()> {
        let _ = elem;
        if idx == 0 && len == 0 {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }

    #[cfg(feature = "alloc")]
    fn to_boxed_slice(&self, idx: u32, len: u32) -> BoundsCheck<alloc::boxed::Box<[E]>> {
        if idx == 0 && len == 0 {
            Ok(alloc::boxed::Box::default())
        } else {
            Err(BoundsCheckError)
        }
    }
}

impl<E: crate::TableElement> crate::TableExt<E> for EmptyTable {
    fn clone_from<Src>(&self, src: &Src, dst_idx: u32, src_idx: u32, len: u32) -> BoundsCheck<()>
    where
        Src: crate::Table<E> + ?Sized,
    {
        let _ = src;
        let _ = src_idx;
        if dst_idx == 0 && len == 0 {
            Ok(())
        } else {
            Err(BoundsCheckError)
        }
    }
}
