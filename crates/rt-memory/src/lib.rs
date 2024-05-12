//! Implementation of WebAssembly linear memory for `wasm2rs`.

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod address;
mod empty;
mod helpers;

#[cfg(feature = "alloc")]
mod heap;

pub use address::Address;
pub use empty::EmptyMemory;
pub use helpers::*;

#[cfg(feature = "alloc")]
pub use heap::HeapMemory;

/// The size, in bytes, of a WebAssembly linear memory [page].
///
/// [page]: https://webassembly.github.io/spec/core/exec/runtime.html#page-size
pub const PAGE_SIZE: u32 = 65536;

/// Error type used when the minimum required number of [pages] for a linear memory could not be
/// allocated.
///
/// [pages]: PAGE_SIZE
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocationError<I: Address = u32> {
    size: I,
}

impl<I: Address> AllocationError<I> {
    /// The minimum number of [pages] that was requested.
    ///
    /// [pages]: PAGE_SIZE
    pub fn size(&self) -> I {
        self.size
    }
}

/// Error type used when an attempt to read or write from a linear [`Memory`] fails.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct AccessError<I: Address = u32> {
    memory: u32,
    address: I,
    offset: I,
}

impl<I: Address> AccessError<I> {
    const fn new(memory: u32, offset: I, address: I) -> Self {
        Self {
            memory,
            offset,
            address,
        }
    }
}

impl<I: Address> core::fmt::Display for AccessError<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "invalid access of linear memory #{} at address {:#X}",
            self.memory, self.address
        )?;

        if self.offset > I::ZERO {
            write!(f, " + {:#X}", self.offset)?;

            if let Some(effective_address) = self.address.checked_add(&self.offset) {
                write!(f, " = {effective_address:#X}")?;
            }
        }
        Ok(())
    }
}

/// Error type used when an address was out of bounds.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BoundsCheckError;

impl core::fmt::Display for BoundsCheckError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("out-of-bounds address")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BoundsCheckError {}

/// Result type used for functions that need to indicate if an address is out of bounds.
pub type BoundsCheck<T> = core::result::Result<T, BoundsCheckError>;

macro_rules! unaligned_integer_accesses {
    {
        $($int:ty : $load:ident / $store:ident;)*
    } => {$(
        fn $load<I: Address, M: Memory<I> + ?Sized>(mem: &M, addr: I) -> BoundsCheck<$int> {
            let mut dst = [0u8; core::mem::size_of::<$int>()];
            match mem.copy_to_slice(addr, &mut dst) {
                Ok(()) => Ok(<$int>::from_le_bytes(dst)),
                Err(e) => Err(e),
            }
        }

        fn $store<I, M>(mem: &M, addr: I, value: $int) -> BoundsCheck<()>
        where
            I: Address,
            M: Memory<I> + ?Sized,
        {
            match mem.copy_from_slice(addr, &value.to_le_bytes()) {
                Ok(()) => Ok(()),
                Err(e) => Err(e),
            }
        }
    )*};
}

unaligned_integer_accesses! {
    i16 : unaligned_i16_load / unaligned_i16_store;
    i32 : unaligned_i32_load / unaligned_i32_store;
    i64 : unaligned_i64_load / unaligned_i64_store;
}

fn default_copy_between<I, Dst, Src>(
    dst: &Dst,
    src: &Src,
    dst_addr: I,
    src_addr: I,
    len: I,
) -> BoundsCheck<()>
where
    I: Address,
    Dst: Memory<I> + ?Sized,
    Src: Memory<I> + ?Sized,
{
    /// Limit on the number of bytes to copy at a time.
    const BUFFER_SIZE: usize = 2048;

    let mut buffer = [0u8; BUFFER_SIZE];
    let mut written = I::ZERO;
    while let Some(slice @ [_, ..]) = buffer.get_mut(..BUFFER_SIZE.min((len - written).as_())) {
        dst.copy_to_slice(dst_addr + written, slice)?;
        src.copy_from_slice(src_addr + written, slice)?;

        // `slice.len() <= buffer.len() <= u32::MAX`
        #[allow(clippy::cast_possible_truncation)]
        {
            written += I::cast_from_usize(slice.len());
        }
    }

    Ok(())
}

// mod private {
//     /// Implementation detail to allow downcasting an arbitrary `Memory32` implementation.
//     #[derive(Debug)]
//     pub struct Hidden;
// }

/// Trait for implementations of [WebAssembly linear memory].
///
/// [WebAssembly linear memory]: https://webassembly.github.io/spec/core/syntax/modules.html#memories
pub trait Memory<I: Address = u32> {
    // /// Implementation detail to allow attempts to perform reflection with `self`.
    // #[doc(hidden)]
    // fn try_as_any(&self, _: private::Hidden) -> Option<&dyn core::any::Any> {
    //     None
    // }

    /// Returns the size of the linear memory, in terms of the [`PAGE_SIZE`].
    fn size(&self) -> I;

    /// Gets the maximum number of pages that this linear memory can have.
    fn maximum(&self) -> I;

    /// Increases the size of the linear memory by the specified number of [pages], and returns the old number of pages.
    ///
    /// The default implementation for this method simply calls [`Memory::size()`] of `delta` is
    /// `0`, and returns `-1` otherwise.
    ///
    /// # Errors
    ///
    /// If the size of the memory oculd not be increased, then `-1` is returned.
    ///
    /// [pages]: PAGE_SIZE
    fn grow(&self, delta: I) -> I {
        if delta == I::ZERO {
            self.size()
        } else {
            I::max_value()
        }
    }

    /// Copies bytes from linear memory starting at the specified address into the given slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of addresses `addr..(addr + dst.len())` is not in bounds.
    fn copy_to_slice(&self, addr: I, dst: &mut [u8]) -> BoundsCheck<()>;

    /// Copies bytes from the given slice into linear memory starting at the specified address.
    ///
    /// # Errors
    ///
    /// Returns an error if the range of addresses `addr..(addr + dst.len())` is not in bounds.
    fn copy_from_slice(&self, addr: I, src: &[u8]) -> BoundsCheck<()>;

    /// Moves a range of bytes in this linear memory to another location.
    ///
    /// # Errors
    ///
    /// Returns an error if `src_addr + len` or `dst_addr + len` is not in bounds.
    fn copy_within(&self, dst_addr: I, src_addr: I, len: I) -> BoundsCheck<()> {
        default_copy_between(self, self, dst_addr, src_addr, len)
    }

    /// Copies bytes from the given linear memory into `self`.
    ///
    /// # Errors
    ///
    /// Returns an error if `src_addr + len` is not in bounds in the source memory, or if
    /// `dst_addr + len` is not in bounds in `self`.
    fn copy_from<Src>(&self, src: &Src, dst_addr: I, src_addr: I, len: I) -> BoundsCheck<()>
    where
        Src: Memory<I> + ?Sized,
    {
        // If neither `src` or `self` are zero-sized types, then they should refer to the same
        // object if the pointers are equal.
        if core::mem::size_of_val(self) > 0
            && core::mem::size_of_val(src) > 0
            && core::ptr::addr_eq(self as *const Self, src as *const Src)
        {
            self.copy_within(dst_addr, src_addr, len)
        } else {
            default_copy_between(self, src, dst_addr, src_addr, len)
        }
    }

    //fn to_boxed_slice<R>(&self, range: R)
    // where
    //     R: core::ops::RangeBounds<I>,
    //     Self: Sized,
    // {
    // }

    /// Loads the value of the byte stored at the given address.
    fn i8_load(&self, addr: I) -> BoundsCheck<i8> {
        let mut dst = 0u8;
        match self.copy_to_slice(addr, core::slice::from_mut(&mut dst)) {
            Ok(()) => Ok(dst as i8),
            Err(e) => Err(e),
        }
    }

    /// Loads a 16-bit integer from the given address.
    fn i16_load(&self, addr: I) -> BoundsCheck<i16> {
        unaligned_i16_load(self, addr)
    }

    /// Loads a 32-bit integer from the given address.
    fn i32_load(&self, addr: I) -> BoundsCheck<i32> {
        unaligned_i32_load(self, addr)
    }

    /// Loads a 64-bit integer from the given address.
    fn i64_load(&self, addr: I) -> BoundsCheck<i64> {
        unaligned_i64_load(self, addr)
    }

    /// Writes into the byte at the given address.
    fn i8_store(&self, addr: I, value: i8) -> BoundsCheck<()> {
        self.copy_from_slice(addr, &[value as u8])
    }

    /// Stores a 16-bit integer into the given address.
    fn i16_store(&self, addr: I, value: i16) -> BoundsCheck<()> {
        unaligned_i16_store(self, addr, value)
    }

    /// Stores a potentially aligned 32-bit integer into the given address.
    fn i32_store(&self, addr: I, value: i32) -> BoundsCheck<()> {
        unaligned_i32_store(self, addr, value)
    }

    /// Stores a potentially aligned 64-bit integer into the given address.
    fn i64_store(&self, addr: I, value: i64) -> BoundsCheck<()> {
        unaligned_i64_store(self, addr, value)
    }
}

//pub trait UnsharedMemory<I: Address = u32>: Memory<I> + core::ops::Deref<Target = core::cell::Cell<[u8]>> + core::ops::DerefMut where Self: !Sync {}
