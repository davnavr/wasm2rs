/// A [`Memory32`] implementation that always has a size of zero.
///
/// [`Memory32`]: crate::memory::Memory32
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct EmptyMemory;

impl crate::memory::Memory32 for EmptyMemory {
    fn size(&self) -> u32 {
        0
    }

    fn limit(&self) -> u32 {
        0
    }

    fn grow(&self, _: u32) -> u32 {
        crate::memory::MEMORY_GROW_FAILED
    }

    fn copy_from_slice(&self, _: u32, _: &[u8]) -> crate::memory::BoundsCheck<()> {
        Err(crate::memory::BoundsCheckError)
    }

    fn copy_to_slice(&self, _: u32, _: &mut [u8]) -> crate::memory::BoundsCheck<()> {
        Err(crate::memory::BoundsCheckError)
    }
}
