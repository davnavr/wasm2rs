//! Types for reading slices and arrays of structures in linear [`Memory`].

use crate::memory::{Address, BoundsCheck, Memory};
use crate::{MutPtr, Pointee, Ptr};

/// Represents a slice of `T` into linear [`Memory`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Slice<T: Pointee<I>, I: Address = u32> {
    /// A pointer to where the `T`s are in [`Memory`].
    pub items: Ptr<T, I>,
    /// The number of `T`s in the slice.
    pub count: I,
}

/// Represents a mutable slice of `T` into linear [`Memory`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MutSlice<T: Pointee<I>, I: Address = u32> {
    /// A pointer to where the `T`s are in [`Memory`].
    pub items: MutPtr<T, I>,
    /// The number of `T`s in the slice.
    pub count: I,
}

impl<T: Pointee<I>, I: Address> From<MutSlice<T, I>> for Slice<T, I> {
    fn from(slice: MutSlice<T, I>) -> Self {
        Slice {
            items: slice.items.cast_to_ptr(),
            count: slice.count,
        }
    }
}

impl<T: Pointee<I>, I: Address> Slice<T, I> {
    /// Returns an [`Iterator`] yielding `T`s loaded from the given linear [`Memory`].
    pub fn into_iter_load_from<M: Memory<I>>(self, memory: &M) -> IntoIter<'_, M, T, I> {
        IntoIter {
            slice: self,
            memory,
        }
    }
}

impl<I: Address> MutSlice<u8, I> {
    /// Fills the contents of the linear [`Memory`] with bytes from the given slice.
    ///
    /// # Panics
    ///
    /// Panics if the length of the slice is not equal to `src.len()`.
    pub fn copy_from<M: Memory<I>>(&self, memory: &M, src: &[u8]) -> BoundsCheck<()> {
        assert_eq!(src.len(), self.count.as_());
        memory.copy_from_slice(self.items.to_address(), src)
    }
}

//impl IntoIterator for Slice // yields Ptrs

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
    M: Memory<I>,
    T: Pointee<I>,
    I: Address,
{
    type Item = BoundsCheck<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.slice.count = self.slice.count.checked_sub(&I::ONE)?;
        let read_result = self.slice.items.load(self.memory);

        if read_result.is_err() {
            self.slice.count = I::ZERO;
            return Some(read_result);
        }

        Some(
            match self
                .slice
                .items
                .to_address()
                .checked_add(&I::cast_from_usize(T::SIZE))
            {
                Some(new_items) => {
                    self.slice.items = Ptr::from_address(new_items);
                    read_result
                }
                None => {
                    self.slice.count = I::ZERO;
                    Err(crate::memory::BoundsCheckError)
                }
            },
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let min_bytes: usize = self
            .memory
            .size()
            .checked_sub(&self.slice.items.to_address())
            .unwrap_or_default()
            .as_();

        (min_bytes / T::SIZE, self.slice.count.to_usize())
    }
}