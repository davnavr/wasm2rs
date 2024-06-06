//! Integer vector operations using [`sse2`].

use crate::intrinsics::sse2::{self, __m128i};
use crate::v128;

macro_rules! impl_into_lanes {
    ($vec:expr => [u8; 16]) => {{
        let mut bytes = v128::Bytes { bytes: [0u8; 16] };

        // SAFETY: module compiled only when `sse2` is enabled.
        // SAFETY: `bytes.bytes` is valid to write to, and is aligned to 16 bytes.
        unsafe {
            sse2::_mm_store_si128(
                (&mut bytes) as *mut v128::Bytes as *mut __m128i,
                $vec,
            );
        }

        bytes.bytes
    }};
    ($vec:expr => [$num:ty; $lanes:literal]) => {{
        let lanes = impl_into_lanes!($vec => [u8; 16]);

        // SAFETY: all bits are valid in source and destination.
        unsafe {
            core::mem::transmute::<[u8; 16], [$num; $lanes]>(lanes)
        }
    }};
}

macro_rules! implementations {
    ($name:ident = [$int:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) fn into_lanes_impl(vec: __m128i) -> [$int; $lanes] {
        impl_into_lanes!(vec => [$int; $lanes])
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

    };
}

crate::v128_integer_interpretations!(implementations);
