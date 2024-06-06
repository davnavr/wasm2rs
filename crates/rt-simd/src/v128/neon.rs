//! Implements 128-bit vector operations using [`neon`].

use crate::{v128, intrinsics::neon};

// TODO: Support big-endian ARM targets

pub(in crate::v128) type V128 = neon::uint8x16_t;
pub(in crate::v128) type I8x16 = neon::int8x16_t;
pub(in crate::v128) type U8x16 = neon::uint8x16_t;
pub(in crate::v128) type I16x8 = neon::int16x8_t;
pub(in crate::v128) type U16x8 = neon::uint16x8_t;
pub(in crate::v128) type I32x4 = neon::int32x4_t;
pub(in crate::v128) type U32x4 = neon::uint32x4_t;
pub(in crate::v128) type I64x2 = neon::int64x2_t;
pub(in crate::v128) type U64x2 = neon::uint64x2_t;
pub(in crate::v128) type F32x4 = neon::float32x4_t;
pub(in crate::v128) type F64x2 = neon::float64x2_t;

macro_rules! impl_splat {
    ($x:ident => I8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s8($x) }
    };
    ($x:ident => U8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u8($x) }
    };
    ($x:ident => I16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s16($x) }
    };
    ($x:ident => U16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u16($x) }
    };
    ($x:ident => I32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s32($x) }
    };
    ($x:ident => U32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u32($x) }
    };
    ($x:ident => I64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s64($x) }
    };
    ($x:ident => U64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u64($x) }
    };
    ($x:ident => F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_f32($x) }
    };
    ($x:ident => F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_f64($x) }
    };
}

macro_rules! impl_from_lanes {
    ($lanes:ident => I8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s8($x) }
    };
    ($x:ident => U8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u8($x) }
    };
    ($x:ident => I16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s16($x) }
    };
    ($x:ident => U16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u16($x) }
    };
    ($x:ident => I32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s32($x) }
    };
    ($x:ident => U32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u32($x) }
    };
    ($x:ident => I64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_s64($x) }
    };
    ($x:ident => U64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_u64($x) }
    };
    ($x:ident => F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_f32($x) }
    };
    ($x:ident => F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdupq_n_f64($x) }
    };
}

macro_rules! impl_binop {
    // Add
    ($lhs:ident + $rhs:ident -> I8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_s8($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> U8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_u8($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> I16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_s16($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> U16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_u16($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> I32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_s32($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> U32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_u32($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> I64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_s64($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> U64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_u64($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_f32($lhs, $rhs) }
    };
    ($lhs:ident + $rhs:ident -> F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vaddq_f64($lhs, $rhs) }
    };

    // Sub
    ($lhs:ident - $rhs:ident -> I8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_s8($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> U8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_u8($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> I16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_s16($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> U16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_u16($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> I32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_s32($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> U32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_u32($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> I64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_s64($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> U64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_u64($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_f32($lhs, $rhs) }
    };
    ($lhs:ident - $rhs:ident -> F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vsubq_f64($lhs, $rhs) }
    };

    // Mul
    ($lhs:ident * $rhs:ident -> F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vmulq_f32($lhs, $rhs) }
    };
    ($lhs:ident * $rhs:ident -> F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vmulq_f64($lhs, $rhs) }
    };

    // Div
    ($lhs:ident / $rhs:ident -> F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdivq_f32($lhs, $rhs) }
    };
    ($lhs:ident / $rhs:ident -> F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vdivq_f64($lhs, $rhs) }
    };
}

macro_rules! impl_into_v128 {
    ($vec:ident: I8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_s8($vec) }
    };
    ($vec:ident: U8x16) => {
        $vec
    };
    ($vec:ident: I16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_s16($vec) }
    };
    ($vec:ident: U16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_u16($vec) }
    };
    ($vec:ident: I32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_s32($vec) }
    };
    ($vec:ident: U32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_u32($vec) }
    };
    ($vec:ident: I64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_s64($vec) }
    };
    ($vec:ident: U64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_u64($vec) }
    };
    ($vec:ident: F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_f32($vec) }
    };
    ($vec:ident: F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u8_f64($vec) }
    };
}

macro_rules! impl_from_v128 {
    ($vec:ident -> I8x16) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_s8_u8($vec.0) }
    };
    ($vec:ident -> U8x16) => {
        $vec.0
    };
    ($vec:ident -> I16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_s16_u8($vec.0) }
    };
    ($vec:ident -> U16x8) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u16_u8($vec.0) }
    };
    ($vec:ident -> I32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_s32_u8($vec.0) }
    };
    ($vec:ident -> U32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u32_u8($vec.0) }
    };
    ($vec:ident -> I64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_s64_u8($vec.0) }
    };
    ($vec:ident -> U64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_u64_u8($vec.0) }
    };
    ($vec:ident -> F32x4) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_f32_u8($vec.0) }
    };
    ($vec:ident -> F64x2) => {
        // SAFETY: module compiled only when `neon` is enabled.
        unsafe { neon::vreinterpretq_f64_u8($vec.0) }
    };
}

macro_rules! implementations {
    ($name:ident = [$num:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) fn zero_impl() -> $name {
        Self::splat_impl(Default::default())
    }

    pub(in crate::v128) fn splat_impl(x: $num) -> $name {
        impl_splat!(x => $name)
    }

    pub(in crate::v128) fn from_lanes_impl(lanes: [$num; $lanes]) -> $name {
        // Should compile down to the same instructions as `neon::vld1q_XXX`
        // SAFETY: lanes is valid to read from.
        unsafe {
            core::ptr::read_unaligned(lanes.as_ptr() as *const $name)
        }
    }

    pub(in crate::v128) fn into_lanes_impl(vec: $name) -> [$num; $lanes] {
        #[derive(Clone, Copy)]
        #[repr(align(16))]
        struct Lanes {
            lanes: [$num; $lanes]
        }

        let mut lanes = Lanes { lanes: [Default::default(); $lanes] };

        // Should compile down to the same instructions as `neon::vst1q_XXX`
        // SAFETY: lanes is valid to write into, and is aligned to 16 bytes.
        unsafe {
            core::ptr::write_unaligned(lanes.lanes.as_mut_ptr() as *mut $name, vec)
        }

        lanes.lanes
    }

    pub(in crate::v128) fn add_impl(lhs: $name, rhs: $name) -> $name {
        impl_binop!(lhs + rhs -> $name)
    }

    pub(in crate::v128) fn sub_impl(lhs: $name, rhs: $name) -> $name {
        impl_binop!(lhs - rhs -> $name)
    }
}

impl From<$name> for v128::$name {
    fn from(vec: $name) -> Self {
        Self(vec)
    }
}

impl From<v128::$name> for $name {
    fn from(vec: v128::$name) -> $name {
        vec.0
    }
}

impl From<$name> for v128::V128 {
    fn from(vec: $name) -> Self {
        Self(impl_into_v128!(vec: $name))
    }
}

impl From<v128::V128> for $name {
    fn from(vec: v128::V128) -> $name {
        impl_from_v128!(vec -> $name)
    }
}

    };
}

crate::v128_interpretations!(implementations);

macro_rules! implementations_float {
    ($name:ident = [$fnn:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) fn mul_impl(lhs: $name, rhs: $name) -> $name {
        impl_binop!(lhs * rhs -> $name)
    }

    pub(in crate::v128) fn div_impl(lhs: $name, rhs: $name) -> $name {
        impl_binop!(lhs / rhs -> $name)
    }
}

    };
}

crate::v128_float_interpretations!(implementations_float);

impl v128::V128 {
    pub(in crate::v128) fn zero_impl() -> neon::uint8x16_t {
        v128::U8x16::zero_impl()
    }

    pub(in crate::v128) fn from_bytes_impl(bytes: [u8; 16]) -> neon::uint8x16_t {
        v128::U8x16::from_lanes_impl(bytes)
    }

    pub(in crate::v128) fn to_bytes_impl(self) -> [u8; 16] {
        v128::U8x16::into_lanes_impl(self.0)
    }
}
