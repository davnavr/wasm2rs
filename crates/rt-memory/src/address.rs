#![allow(clippy::cast_possible_truncation)]

mod private {
    pub trait Integer:
        Copy
        + num_traits::PrimInt
        + num_traits::ConstZero
        + num_traits::ConstOne
        + num_traits::AsPrimitive<u128>
        + num_traits::AsPrimitive<usize>
        + num_traits::ops::overflowing::OverflowingAdd
        + core::ops::AddAssign
        + core::fmt::Debug
        + core::fmt::Display
        + core::fmt::UpperHex
        + core::fmt::LowerHex
        + From<u8>
        + 'static
    {
        fn checked_ilog(self, base: Self) -> Option<u32>;
    }

    impl Integer for u32 {
        fn checked_ilog(self, base: Self) -> Option<u32> {
            <u32>::checked_ilog(self, base)
        }
    }

    impl Integer for i32 {
        fn checked_ilog(self, base: Self) -> Option<u32> {
            <i32>::checked_ilog(self, base)
        }
    }

    impl Integer for u64 {
        fn checked_ilog(self, base: Self) -> Option<u32> {
            <u64>::checked_ilog(self, base)
        }
    }

    impl Integer for i64 {
        fn checked_ilog(self, base: Self) -> Option<u32> {
            <i64>::checked_ilog(self, base)
        }
    }
}

/// Trait for integer types that can be used as indices into linear memory.
///
/// This allows generic linear memory operations over both 32-bit and [64-bit] linear memories.
///
/// [64-bit]: https://github.com/WebAssembly/memory64
pub trait Address:
    private::Integer + num_traits::Unsigned + num_traits::AsPrimitive<Self::Signed>
{
    /// Signed version of the integer address type.
    type Signed: private::Integer + num_traits::Signed + num_traits::AsPrimitive<Self>;

    /// The maximum number of pages that the linear memory can have.
    const MAX_PAGE_COUNT: Self;

    /// Value used when a [`Memory::grow()`](crate::Memory::grow) call fails.
    const GROW_FAILED: Self;

    /// Equivalent to `value as Self`.
    fn cast_from_u32(value: u32) -> Self;

    /// Equivalent to `value as Self`.
    fn cast_from_usize(value: usize) -> Self;

    /// Equivalent to `value as Self`.
    fn cast_from_signed(value: Self::Signed) -> Self;
}

impl Address for u32 {
    type Signed = i32;

    const MAX_PAGE_COUNT: u32 = 65536; // crate::PAGE_SIZE * crate::PAGE_SIZE = u32::MAX + 1
    const GROW_FAILED: u32 = -1i32 as u32;

    fn cast_from_u32(value: u32) -> Self {
        value
    }

    fn cast_from_usize(value: usize) -> u32 {
        value as u32
    }

    fn cast_from_signed(value: i32) -> Self {
        value as u32
    }
}

impl Address for u64 {
    type Signed = i64;

    const MAX_PAGE_COUNT: u64 = 281474976710656; // crate::PAGE_SIZE * MAX_PAGE_COUNT = u64::MAX + 1
    const GROW_FAILED: u64 = -1i64 as u64;

    fn cast_from_u32(value: u32) -> Self {
        value as u64
    }

    fn cast_from_usize(value: usize) -> u64 {
        value as u64
    }

    fn cast_from_signed(value: i64) -> Self {
        value as u64
    }
}

/// Represents an [*effective address*] used for memory accesses in WebAssembly. This is typically
/// calculated by adding static offset to a dynamic address operand.
///
/// This behaves essentially like a `u33` or a `u65` integer.
///
/// [*effective address*]: https://webassembly.github.io/spec/core/syntax/instructions.html#memory-instructions
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct EffectiveAddress<I: Address = u32> {
    high_bit: bool,
    low_bits: I,
}

impl<I: Address> Default for EffectiveAddress<I> {
    fn default() -> Self {
        Self::MIN
    }
}

impl<I: Address> EffectiveAddress<I> {
    /// Creates a new address from the given bits.
    pub const fn from_bits(high_bit: bool, low_bits: I) -> Self {
        Self { high_bit, low_bits }
    }

    const MIN: Self = Self::from_bits(false, I::ZERO);

    /// Adds the `offset` to the given `address`.
    #[inline]
    pub fn with_offset(offset: I, address: I) -> Self {
        let (low_bits, high_bit) =
            num_traits::ops::overflowing::OverflowingAdd::overflowing_add(&address, &offset);
        Self { low_bits, high_bit }
    }

    /// Interprets the `address` as its unsigned counterpart, then adds the `offset` to it.
    #[inline]
    pub fn signed_with_offset(offset: I, address: I::Signed) -> Self {
        Self::with_offset(offset, I::cast_from_signed(address))
    }

    /// Attempts to convert `Self` into an address.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the effective address would overflow the address space.
    pub fn calculate(self) -> crate::BoundsCheck<I> {
        if !self.high_bit {
            Ok(self.low_bits)
        } else {
            Err(crate::BoundsCheckError)
        }
    }

    fn to_u128(self) -> u128 {
        num_traits::AsPrimitive::<u128>::as_(self.low_bits)
            | (1u128 << (core::mem::size_of::<I>() * 8))
    }
}

impl<I: Address> From<I> for EffectiveAddress<I> {
    fn from(low_bits: I) -> Self {
        Self {
            high_bit: false,
            low_bits,
        }
    }
}

impl From<EffectiveAddress<u32>> for u64 {
    fn from(address: EffectiveAddress<u32>) -> Self {
        (address.low_bits as u64) | ((address.high_bit as u64) << 32)
    }
}

impl From<EffectiveAddress<u32>> for EffectiveAddress<u64> {
    fn from(address: EffectiveAddress<u32>) -> Self {
        Self::from(u64::from(address))
    }
}

impl<I: Address> core::cmp::PartialEq<I> for EffectiveAddress<I> {
    fn eq(&self, other: &I) -> bool {
        !self.high_bit && self.low_bits == *other
    }
}

impl<I: Address> core::cmp::Ord for EffectiveAddress<I> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        core::cmp::Ord::cmp(&self.to_u128(), &other.to_u128())
    }
}

impl<I: Address> core::cmp::PartialOrd for EffectiveAddress<I> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(core::cmp::Ord::cmp(self, other))
    }
}

impl<I: Address> core::cmp::PartialOrd<I> for EffectiveAddress<I> {
    fn partial_cmp(&self, other: &I) -> Option<core::cmp::Ordering> {
        Some(core::cmp::Ord::cmp(&self.to_u128(), &I::as_(*other)))
    }
}

macro_rules! effective_address_fmt {
    ($($trait:path,)*) => {$(
        impl<I: Address> $trait for EffectiveAddress<I> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                <u128 as $trait>::fmt(&self.to_u128(), f)
            }
        }
    )*};
}

effective_address_fmt! {
    core::fmt::Debug,
    core::fmt::Display,
    core::fmt::UpperHex,
    core::fmt::LowerHex,
    core::fmt::Binary,
}
