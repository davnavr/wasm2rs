//! Implements 128-bit vector operations used when the `simd-intrinsics` feature is not enabled, or
//! target architecture-specific intrinsics are unavailable.

use crate::v128;

pub(in crate::v128) type V128 = v128::Bytes;

crate::v128_integer_interpretations!(repr_struct);

#[cfg(simd_no_intrinsics)]
macro_rules! lanewise_op {
    (($self:ident, rhs:ident) => <[$int:ty; $lanes:literal]>::$op:ident) => {
        ::core::array::from_fn::<$int, $lanes, _>(|i| {
            $self.0.lanes[i]
                .from_le()
                .$op($rhs.0.lanes[i].from_le())
                .to_le()
        })
    };
}

macro_rules! implementations {
    ($name:ident = [$int:tt; $lanes:tt] as $_:literal) => {
        impl v128::$name {
            pub(in crate::v128) fn splat_impl(x: $int) -> $name {
                $name { lanes: [x; $lanes] }
            }

            pub(in crate::v128) fn into_lanes_impl(vec: __m128i) -> [$int; $lanes] {
                impl_into_lanes!(vec => [$int; $lanes])
            }
        }
    };
}

crate::v128_integer_interpretations!(implementations);
