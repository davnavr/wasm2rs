//! Implements 128-bit vector operations used when the `simd-intrinsics` feature is not enabled, or
//! target architecture-specific intrinsics are unavailable.

use crate::v128;

pub(in crate::v128) type V128 = v128::Bytes;

macro_rules! repr_struct {
    ($name:ident = [$num:tt; $lanes:tt] as $_:literal) => {
        #[derive(Clone, Copy)]
        #[repr(align(16))]
        pub(in crate::v128) struct $name {
            lanes: [$num; $lanes],
        }
    };
}

crate::v128_interpretations!(repr_struct);

macro_rules! lanewise_unop {
    ($name:ident::$op:ident($lhs:ident, $rhs:ident)) => {
        $name {
            lanes: core::array::from_fn(|i| $lhs.lanes[i].$op($rhs.lanes[i]))
        }
    };
    (($lhs:ident, $rhs:ident) -> $name:ident = |$left:ident, $right:ident| $result:expr) => {
        $name {
            lanes: core::array::from_fn(|i| {
                let $left = $lhs.lanes[i];
                let $right = $rhs.lanes[i];
                $result
            })
        }
    };
}

macro_rules! lane_to_le {
    (f32) => {
        |f: f32| f.to_bits().to_le()
    };
    (f64) => {
        |f: f64| f.to_bits().to_le()
    };
    ($int:ty) => {
        <$int>::to_le
    };
}

macro_rules! impl_into_v128 {
    ($vec:ident: [u8; 16]) => {
        $vec.lanes
    };
    ($vec:ident: [$num:tt; $lanes:literal]) => {{
        // Have to convert lanes into little-endian byte order.
        let lanes = $vec.lanes.map(lane_to_le!($num));

        // SAFETY: all bit-patterns are valid for arrays of integer/float types.
        unsafe {
            core::mem::transmute::<[_; $lanes], [u8; 16]>(lanes)
        }
    }};
}

macro_rules! lane_from_le {
    (f32) => {
        |n| f32::from_bits(u32::from_le(n))
    };
    (f64) => {
        |n| f64::from_bits(u64::from_le(n))
    };
    ($int:ty) => {
        <$int>::to_le
    };
}

macro_rules! impl_from_v128 {
    ($vec:ident => [u8; 16]) => {
        $vec.0.bytes
    };
    ($vec:ident => [$num:tt; $lanes:literal]) => {{
        // SAFETY: all bit-patterns are valid for arrays of integer.float types.
        let lanes = unsafe {
            core::mem::transmute::<[u8; 16], [_; $lanes]>($vec.0.bytes)
        };

        // Have to convert lanes from little-endian byte order.
        lanes.map(lane_from_le!($num))
    }};
}

macro_rules! implementations_common {
    ($name:ident = [$num:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) fn from_lanes_impl(lanes: [$num; $lanes]) -> $name {
        $name { lanes }
    }

    pub(in crate::v128) const fn splat_impl(x: $num) -> $name {
        $name { lanes: [x; $lanes] }
    }

    pub(in crate::v128) const fn into_lanes_impl(vec: $name) -> [$num; $lanes] {
        vec.lanes
    }
}

impl From<$name> for v128::V128 {
    fn from(vec: $name) -> Self {
        v128::V128(v128::Bytes { bytes: impl_into_v128!(vec: [$num; $lanes]) })
    }
}

impl From<v128::V128> for $name {
    fn from(vec: v128::V128) -> Self {
        $name { lanes: impl_from_v128!(vec => [$num; $lanes]) }
    }
}

    };
}

crate::v128_interpretations!(implementations_common);

macro_rules! implementations_integer {
    ($name:ident = [$int:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) const fn zero_impl() -> $name {
        $name { lanes: [0; $lanes] }
    }

    pub(in crate::v128) fn add_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!($name::wrapping_add(lhs, rhs))
    }

    pub(in crate::v128) fn sub_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!($name::wrapping_sub(lhs, rhs))
    }
}

    };
}

crate::v128_integer_interpretations!(implementations_integer);

macro_rules! implementations_float {
    ($name:ident = [$fnn:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) const fn zero_impl() -> $name {
        $name { lanes: [0.0; $lanes] }
    }

    pub(in crate::v128) fn add_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!((lhs, rhs) -> $name = |left, right| left + right)
    }

    pub(in crate::v128) fn sub_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!((lhs, rhs) -> $name = |left, right| left - right)
    }

    pub(in crate::v128) fn mul_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!((lhs, rhs) -> $name = |left, right| left * right)
    }

    pub(in crate::v128) fn div_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!((lhs, rhs) -> $name = |left, right| left / right)
    }
}

    };
}

crate::v128_float_interpretations!(implementations_float);

impl v128::V128 {
    pub(in crate::v128) fn zero_impl() -> v128::Bytes {
        v128::Bytes { bytes: [0; 16] }
    }

    pub(in crate::v128) fn from_bytes_impl(bytes: [u8; 16]) -> v128::Bytes {
        v128::Bytes { bytes }
    }

    pub(in crate::v128) fn to_bytes_impl(self) -> [u8; 16] {
        self.0.bytes
    }
}
