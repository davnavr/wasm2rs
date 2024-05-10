#![allow(clippy::cast_possible_truncation)]

mod private {
    pub trait Integer:
        num_traits::PrimInt
        + num_traits::ConstZero
        + num_traits::ConstOne
        + core::ops::AddAssign
        + core::fmt::Debug
        + core::fmt::Display
        + core::fmt::UpperHex
        + core::fmt::LowerHex
        + 'static
    {
    }

    impl Integer for u32 {}
    impl Integer for i32 {}
}

/// Trait for integer types that can be used as indices into linear memory.
///
/// This allows generic linear memory operations over both 32-bit and [64-bit] linear memories.
///
/// [64-bit]: https://github.com/WebAssembly/memory64
pub trait Address:
    private::Integer
    + num_traits::Unsigned
    + num_traits::AsPrimitive<usize>
    + num_traits::AsPrimitive<Self::Signed>
{
    /// Signed version of the integer address type.
    type Signed: private::Integer + num_traits::Signed + num_traits::AsPrimitive<Self>;

    /// The maximum number of pages that the linear memory can have.
    const MAX_PAGE_COUNT: Self;

    /// Equivalent to `value as Self`.
    fn cast_from_usize(value: usize) -> Self;

    /// Equivalent to `value as Self`.
    fn cast_from_signed(value: Self::Signed) -> Self;
}

impl Address for u32 {
    type Signed = i32;

    const MAX_PAGE_COUNT: u32 = 65536; // crate::PAGE_SIZE * crate::PAGE_SIZE = u32::MAX + 1

    fn cast_from_usize(value: usize) -> u32 {
        value as u32
    }

    fn cast_from_signed(value: Self::Signed) -> Self {
        value as u32
    }
}
