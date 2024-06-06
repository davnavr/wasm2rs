//! Implements 128-bit vector operations used when the `simd-intrinsics` feature is not enabled, or
//! target architecture-specific intrinsics are unavailable.

use crate::v128;

pub(in crate::v128) type V128 = v128::Bytes;

macro_rules! repr_struct {
    ($name:ident = [$int:tt; $lanes:tt] as $_:literal) => {
        #[derive(Clone, Copy)]
        #[repr(align(16))]
        pub(in crate::v128) struct $name {
            lanes: [$int; $lanes],
        }
    };
}

crate::v128_integer_interpretations!(repr_struct);

macro_rules! lanewise_unop {
    ($name:ident::$op:ident($lhs:ident, $rhs:ident)) => {
        $name {
            lanes: core::array::from_fn(|i| $lhs.lanes[i].$op($rhs.lanes[i]))
        }
    };
}

macro_rules! impl_into_v128 {
    ($vec:ident: [u8; 16]) => {
        $vec.lanes
    };
    ($vec:ident: [$int:ty; $lanes:literal]) => {{
        // Have to convert lanes into little-endian byte order.
        let lanes = $vec.lanes.map(<$int>::to_le);

        // SAFETY: all bit-patterns are valid for arrays of integer types.
        unsafe {
            core::mem::transmute::<[$int; $lanes], [u8; 16]>(lanes)
        }
    }};
}

macro_rules! impl_from_v128 {
    ($vec:ident => [u8; 16]) => {
        $vec.0.bytes
    };
    ($vec:ident => [$int:ty; $lanes:literal]) => {{
        // SAFETY: all bit-patterns are valid for arrays of integer types.
        let lanes = unsafe {
            core::mem::transmute::<[u8; 16], [$int; $lanes]>($vec.0.bytes)
        };

        // Have to convert lanes from little-endian byte order.
        lanes.map(<$int>::from_le)
    }};
}

macro_rules! implementations {
    ($name:ident = [$int:tt; $lanes:tt] as $_:literal) => {

impl v128::$name {
    pub(in crate::v128) const fn splat_impl(x: $int) -> $name {
        $name { lanes: [x; $lanes] }
    }

    pub(in crate::v128) const fn into_lanes_impl(vec: $name) -> [$int; $lanes] {
        vec.lanes
    }

    pub(in crate::v128) fn add_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!($name::wrapping_add(lhs, rhs))
    }

    pub(in crate::v128) fn sub_impl(lhs: $name, rhs: $name) -> $name {
        lanewise_unop!($name::wrapping_add(lhs, rhs))
    }
}

impl From<$name> for v128::V128 {
    fn from(vec: $name) -> Self {
        v128::V128(v128::Bytes { bytes: impl_into_v128!(vec: [$int; $lanes]) })
    }
}

impl From<v128::V128> for $name {
    fn from(vec: v128::V128) -> Self {
        $name { lanes: impl_from_v128!(vec => [$int; $lanes]) }
    }
}

    };
}

crate::v128_integer_interpretations!(implementations);

impl v128::V128 {
    pub(in crate::v128) fn from_bytes_impl(bytes: [u8; 16]) -> v128::Bytes {
        v128::Bytes { bytes }
    }

    pub(in crate::v128) fn to_bytes_impl(self) -> [u8; 16] {
        self.0.bytes
    }
}
