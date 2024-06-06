//! Implements 128-bit vector operations using [`sse2`].

#[path = "sse2/integer.rs"]
mod integer;
#[path = "sse2/float.rs"]
mod float;

use crate::{v128, intrinsics::sse2};

pub(in crate::v128) type V128 = sse2::__m128i;
pub(in crate::v128) type I8x16 = sse2::__m128i;
pub(in crate::v128) type U8x16 = sse2::__m128i;
pub(in crate::v128) type I16x8 = sse2::__m128i;
pub(in crate::v128) type U16x8 = sse2::__m128i;
pub(in crate::v128) type I32x4 = sse2::__m128i;
pub(in crate::v128) type U32x4 = sse2::__m128i;
pub(in crate::v128) type I64x2 = sse2::__m128i;
pub(in crate::v128) type U64x2 = sse2::__m128i;
pub(in crate::v128) type F32x4 = sse2::__m128;
pub(in crate::v128) type F64x2 = sse2::__m128d;

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
    ($x:expr => F32x4) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_set1_ps($x)
        }
    };
    ($x:expr => F64x2) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_set1_pd($x)
        }
    };
    ($x:ident => U8x16) => (impl_splat!(($x as i8) => I8x16));
    ($x:ident => U16x8) => (impl_splat!(($x as i16) => I16x8));
    ($x:ident => U32x4) => (impl_splat!(($x as i32) => I32x4));
    ($x:ident => U64x2) => (impl_splat!(($x as i64) => I64x2));
}

macro_rules! impl_binop {
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
    (F32x4::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => F32x4)
    };
    (F64x2::$macro:ident($lhs:ident, $rhs:ident)) => {
        $macro!(($lhs, $rhs) => F64x2)
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
    (($lhs:ident, $rhs:ident) => F32x4) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_add_ps($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => F64x2) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_add_pd($lhs, $rhs) }
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
    (($lhs:ident, $rhs:ident) => F32x4) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_sub_ps($lhs, $rhs) }
    };
    (($lhs:ident, $rhs:ident) => F64x2) => {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_sub_pd($lhs, $rhs) }
    };
}

macro_rules! implementations {
    ($name:ident = [$num:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) fn splat_impl(x: $num) -> $name {
        impl_splat!(x => $name)
    }

    pub(in crate::v128) fn add_impl(lhs: $name, rhs: $name) -> $name {
        impl_binop!($name::impl_add(lhs, rhs))
    }

    pub(in crate::v128) fn sub_impl(lhs: $name, rhs: $name) -> $name {
        impl_binop!($name::impl_sub(lhs, rhs))
    }
}

    }
}

crate::v128_interpretations!(implementations);

impl v128::V128 {
    pub(in crate::v128) fn zero_impl() -> sse2::__m128i {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe {
            sse2::_mm_setzero_si128()
        }
    }

    pub(in crate::v128) fn from_bytes_impl(bytes: [u8; 16]) -> sse2::__m128i {
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

        // SAFETY: module compiled only when `sse2` is enabled.
        // SAFETY: `bytes.bytes` is valid to write to, and is aligned to 16 bytes.
        unsafe {
            sse2::_mm_store_si128(
                bytes.bytes.as_mut_ptr() as *mut sse2::__m128i,
                self.0,
            );
        }

        bytes.bytes
    }
}

impl From<sse2::__m128i> for v128::V128 {
    fn from(v: sse2::__m128i) -> Self {
        Self(v)
    }
}

impl From<v128::V128> for sse2::__m128i {
    fn from(v: v128::V128) -> sse2::__m128i {
        v.0
    }
}

impl From<sse2::__m128> for v128::V128 {
    fn from(v: sse2::__m128) -> Self {
        // SAFETY: module compiled only when `sse2` is enabled.
        let v = unsafe { sse2::_mm_castps_si128(v) };
        Self(v)
    }
}

impl From<v128::V128> for sse2::__m128 {
    fn from(v: v128::V128) -> Self {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_castsi128_ps(v.0) }
    }
}

impl From<sse2::__m128d> for v128::V128 {
    fn from(v: sse2::__m128d) -> Self {
        // SAFETY: module compiled only when `sse2` is enabled.
        let v = unsafe { sse2::_mm_castpd_si128(v) };
        Self(v)
    }
}

impl From<v128::V128> for sse2::__m128d {
    fn from(v: v128::V128) -> Self {
        // SAFETY: module compiled only when `sse2` is enabled.
        unsafe { sse2::_mm_castsi128_pd(v.0) }
    }
}
