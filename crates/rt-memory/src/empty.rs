/// A [`Memory`] implementation that always has a size of zero.
///
/// [`Memory`]: crate::Memory
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct EmptyMemory;

impl<I: crate::Address> crate::Memory<I> for EmptyMemory {
    fn size(&self) -> I {
        I::ZERO
    }

    fn limit(&self) -> I {
        I::ZERO
    }

    fn grow(&self, _: I) -> I {
        I::max_value()
    }

    fn copy_from_slice(&self, _: I, _: &[u8]) -> crate::BoundsCheck<()> {
        Err(crate::BoundsCheckError)
    }

    fn copy_to_slice(&self, _: I, _: &mut [u8]) -> crate::BoundsCheck<()> {
        Err(crate::BoundsCheckError)
    }
}
