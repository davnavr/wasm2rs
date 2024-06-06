//! Implements 128-bit vector operations for `x86` and `x86_64` platforms, utilizing [SSE2]
//! intrinsics and those introduced in later extensions.
//!
//! [SSE2]: https://en.wikipedia.org/wiki/SSE2

use crate::intrinsics::sse2::{self, __m128i};
use crate::v128;

pub(in crate::v128) type V128 = __m128i;
pub(in crate::v128) type I8x16 = __m128i;
pub(in crate::v128) type U8x16 = __m128i;
pub(in crate::v128) type I16x8 = __m128i;
pub(in crate::v128) type U16x8 = __m128i;
pub(in crate::v128) type I32x4 = __m128i;
pub(in crate::v128) type U32x4 = __m128i;
pub(in crate::v128) type I64x2 = __m128i;
pub(in crate::v128) type U64x2 = __m128i;

macro_rules! impl_splat {
    ($x:expr => I8x16) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_set1_epi8($x)
        }
    };
    ($x:expr => I16x8) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_set1_epi16($x)
        }
    };
    ($x:expr => I32x4) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_set1_epi32($x)
        }
    };
    ($x:expr => I64x2) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_set1_epi64x($x)
        }
    };
    ($x:ident => U8x16) => (impl_splat!(($x as i8) => I8x16));
    ($x:ident => U16x8) => (impl_splat!(($x as i16) => I16x8));
    ($x:ident => U32x4) => (impl_splat!(($x as i32) => I32x4));
    ($x:ident => U64x2) => (impl_splat!(($x as i64) => I64x2));
}

macro_rules! impl_into_lanes {
    ($vec:expr => [u8; 16]) => {{
        let mut bytes = v128::Bytes { bytes: [0u8; 16] };

        // SAFETY: module compiled only when `sse2` is enabled.
        // SAFETY: `bytes.bytes` is aligned to 16 bytes.
        unsafe {
            sse2::_mm_storeu_si128(
                (&mut bytes) as *mut v128::Bytes as *mut __m128i,
                $vec,
            );
        }

        bytes.bytes
    }};
    ($vec:expr => [$int:ty; $lanes:literal]) => {{
        let lanes = impl_into_lanes!($vec => [u8; 16]);

        // SAFETY: all bits are valid in source and destination.
        unsafe {
            core::mem::transmute::<[u8; 16], [$int; $lanes]>(lanes)
        }
    }};
}

macro_rules! impl_unop {
    (I8x16::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I8x16)
    };
    (U8x16::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I8x16)
    };
    (I16x8::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I16x8)
    };
    (U16x8::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I16x8)
    };
    (I32x4::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I32x4)
    };
    (U32x4::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I32x4)
    };
    (I64x2::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I64x2)
    };
    (U64x2::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => I64x2)
    };
}

macro_rules! impl_add {
    (($lhs:ident, $rhs:ident) => I8x16) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_add_epi8($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => I16x8) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_add_epi16($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => I32x4) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_add_epi32($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => I64x2) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_add_epi64($lhs, $rhs) }
    };
}

macro_rules! impl_sub {
    (($lhs:ident, $rhs:ident) => I8x16) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_sub_epi8($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => I16x8) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_sub_epi16($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => I32x4) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_sub_epi32($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => I64x2) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_sub_epi64($lhs, $rhs) }
    };
}

macro_rules! implementations {
    ($name:ident = [$int:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) fn splat_impl(x: $int) -> __m128i {
        impl_splat!(x => $name)
    }

    pub(in crate::v128) fn into_lanes_impl(vec: __m128i) -> [$int; $lanes] {
        impl_into_lanes!(vec => [$int; $lanes])
    }

    pub(in crate::v128) fn add_impl(lhs: __m128i, rhs: __m128i) -> __m128i {
        impl_unop!($name::impl_add(lhs, rhs))
    }

    pub(in crate::v128) fn sub_impl(lhs: __m128i, rhs: __m128i) -> __m128i {
        impl_unop!($name::impl_sub(lhs, rhs))
    }
}

impl From<__m128i> for v128::$name {
    fn from(vec: __m128i) -> Self {
        Self(vec)
    }
}

impl From<v128::$name> for __m128i {
    fn from(vec: v128::$name) -> __m128i {
        vec.0
    }
}

    }
}

crate::v128_integer_interpretations!(implementations);

impl v128::V128 {
    pub(in crate::v128) fn from_bytes_impl(bytes: [u8; 16]) -> __m128i {
        // SAFETY: module compiled only when `sse2` is enabled.
        #[allow(clippy::cast_possible_truncation)]
        unsafe {
            sse2::_mm_setr_epi8(
                bytes[15] as i8,
                bytes[14] as i8,
                bytes[13] as i8,
                bytes[12] as i8,
                bytes[11] as i8,
                bytes[10] as i8,
                bytes[9] as i8,
                bytes[8] as i8,
                bytes[7] as i8,
                bytes[6] as i8,
                bytes[5] as i8,
                bytes[4] as i8,
                bytes[3] as i8,
                bytes[2] as i8,
                bytes[1] as i8,
                bytes[0] as i8,
            )
        }
    }

    pub(in crate::v128) fn to_bytes_impl(self) -> [u8; 16] {
        let mut bytes = v128::Bytes { bytes: [0u8; 16] };

        // SAFETY: check for `sse2` target feature occurs above.
        // SAFETY: `bytes.bytes` is aligned to 16 bytes.
        unsafe {
            crate::intrinsics::sse2::_mm_storeu_si128(
                bytes.bytes.as_mut_ptr() as *mut crate::intrinsics::sse2::__m128i,
                self.0,
            );
        }

        bytes.bytes
    }
}
