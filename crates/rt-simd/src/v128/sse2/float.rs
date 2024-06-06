//! Floating-point vector operations using [`sse2`].

use crate::v128;
use crate::intrinsics::sse2::{self, __m128, __m128d};

impl v128::F32x4 {
    pub(in crate::v128) fn into_lanes_impl(vec: __m128) -> [f32; 4] {
        #[derive(Clone, Copy)]
        #[repr(align(16))]
        struct Lanes {
            lanes: [f32; 4],
        }

        let mut lanes = Lanes { lanes: [0.0; 4] };

        // SAFETY: module compiled only when `sse2` is enabled.
        // SAFETY: `lanes.lanes` is valid to write to, and is aligned to 16 bytes.
        unsafe {
            sse2::_mm_store_ps(lanes.lanes.as_mut_ptr(), vec)
        }

        lanes.lanes
    }

    pub(in crate::v128) fn mul_impl(lhs: __m128, rhs: __m128) -> __m128 {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_mul_ps(lhs, rhs)
        }
    }

    pub(in crate::v128) fn div_impl(lhs: __m128, rhs: __m128) -> __m128 {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_div_ps(lhs, rhs)
        }
    }
}

impl v128::F64x2 {
    pub(in crate::v128) fn into_lanes_impl(vec: __m128d) -> [f64; 2] {
        #[derive(Clone, Copy)]
        #[repr(align(16))]
        struct Lanes {
            lanes: [f64; 2],
        }

        let mut lanes = Lanes { lanes: [0.0; 2] };

        // SAFETY: module compiled only when `sse2` is enabled.
        // SAFETY: `lanes.lanes` is valid to write to, and is aligned to 16 bytes.
        unsafe {
            sse2::_mm_store_pd(lanes.lanes.as_mut_ptr(), vec)
        }

        lanes.lanes
    }

    pub(in crate::v128) fn mul_impl(lhs: __m128d, rhs: __m128d) -> __m128d {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_mul_pd(lhs, rhs)
        }
    }

    pub(in crate::v128) fn div_impl(lhs: __m128d, rhs: __m128d) -> __m128d {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_div_pd(lhs, rhs)
        }
    }
}

macro_rules! implementations {
    ($name:ident = [$fnn:tt; $lanes:tt] as $_:literal) => {

impl From<v128::implementation::$name> for v128::$name {
    fn from(vec: v128::implementation::$name) -> Self {
        Self(vec)
    }
}

impl From<v128::$name> for v128::implementation::$name {
    fn from(vec: v128::$name) -> v128::implementation::$name {
        vec.0
    }
}

    };
}

crate::v128_float_interpretations!(implementations);
