//! Types for reading slices and arrays of structures in linear [`Memory`].

use crate::{Ptr, Pointee};
use crate::memory::{Memory, Address};

/// Represents a slice of `T` into linear [`Memory`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Slice<T: Pointee<I>, I: Address = u32> {
    /// A pointer to where the `T`s are in [`Memory`].
    pub items: Ptr<T, I>,
    /// The number of `T`s in the slice.
    pub count: I,
}

//impl IntoIterator for Slice // returns Ptr

/// Yields instances of `T` from [`Memory`] by calling [`T::load_from()`] for every item in a
/// [`Slice`].
///
/// [`T::load_from()`]: Pointee::load_from()
#[derive(Clone, Copy, Debug)]
pub struct IntoIter<'a, M: Memory<I>, T: Pointee<I>, I: Address = u32> {
    slice: Slice<T, I>,
    memory: &'a M,
}

impl<'a, M, T, I> Iterator for IntoIter<'a, M, T, I>
where
    M: Memory<I>, T: Pointee<I>, I: Address,
{
    type Item = crate::memory::BoundsCheck<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.slice.count = self.slice.count.checked_sub(&I::ONE)?;
        let read_result = self.slice.items.load(self.memory);

        if read_result.is_err() {
            self.slice.count = I::ZERO;
            return Some(read_result);
        }

        Some(match self.slice.items.to_address().checked_add(&I::cast_from_usize(T::SIZE)) {
            Some(new_items) => {
                self.slice.items = Ptr::from_address(new_items);
                read_result
            }
            None => {
                self.slice.count = I::ZERO;
                Err(crate::memory::BoundsCheckError)
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let min_bytes: usize = self.memory.size().checked_sub(&self.slice.items.to_address()).unwrap_or(I::ZERO).as_();

        (min_bytes / T::SIZE, self.slice.count.to_usize())
    }
}
