//! Defines the integer interpretations for [`V128`].

use crate::v128::V128;

macro_rules! sse2_impl {
    // *shape*.splat
    (<[i8; 16]>::splat($x:expr)) => {
        Self(crate::arch::_mm_set1_epi8($x))
    };
    (<[u8; 16]>::splat($x:ident)) => {
        sse2_impl!(<[i8; 16]>::splat($x as i8))
    };
    (<[i16; 8]>::splat($x:expr)) => {
        Self(crate::arch::_mm_set1_epi16($x))
    };
    (<[u16; 8]>::splat($x:ident)) => {
        sse2_impl!(<[i16; 8]>::splat($x as i16))
    };
    (<[i32; 4]>::splat($x:expr)) => {
        Self(crate::arch::_mm_set1_epi32($x))
    };
    (<[u32; 4]>::splat($x:ident)) => {
        sse2_impl!(<[i32; 4]>::splat($x as i32))
    };
    (<[i32; 4]>::splat($x:expr)) => {
        Self(crate::arch::_mm_set1_epi32($x))
    };
    (<[u32; 4]>::splat($x:ident)) => {
        sse2_impl!(<[i32; 4]>::splat($x as i32))
    };
    (<[i64; 2]>::splat($x:expr)) => {
        Self(crate::arch::_mm_set1_epi64x($x))
    };
    (<[u64; 2]>::splat($x:ident)) => {
        sse2_impl!(<[i64; 2]>::splat($x as i64))
    };

    // *shape*.add
    (<[i8; 16]>::add($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_add_epi8($self.0, $rhs.0))
    };
    (<[u8; 16]>::add($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i8; 16]>::add($self, $rhs))
    };
    (<[i16; 8]>::add($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_add_epi16($self.0, $rhs.0))
    };
    (<[u16; 8]>::add($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i16; 8]>::add($self, $rhs))
    };
    (<[i32; 4]>::add($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_add_epi32($self.0, $rhs.0))
    };
    (<[u32; 4]>::add($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i32; 4]>::add($self, $rhs))
    };
    (<[i64; 2]>::add($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_add_epi64($self.0, $rhs.0))
    };
    (<[u64; 2]>::add($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i64; 2]>::add($self, $rhs))
    };

    // *shape*.sub
    (<[i8; 16]>::sub($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_sub_epi8($self.0, $rhs.0))
    };
    (<[u8; 16]>::sub($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i8; 16]>::sub($self, $rhs))
    };
    (<[i16; 8]>::sub($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_sub_epi16($self.0, $rhs.0))
    };
    (<[u16; 8]>::sub($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i16; 8]>::sub($self, $rhs))
    };
    (<[i32; 4]>::sub($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_sub_epi32($self.0, $rhs.0))
    };
    (<[u32; 4]>::sub($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i32; 4]>::sub($self, $rhs))
    };
    (<[i64; 2]>::sub($self:ident, $rhs:ident)) => {
        Self(crate::arch::_mm_sub_epi64($self.0, $rhs.0))
    };
    (<[u64; 2]>::sub($self:ident, $rhs:ident)) => {
        sse2_impl!(<[i64; 2]>::sub($self, $rhs))
    };
}

macro_rules! into_lanes {
    ($self:ident => [u8; 16]) => {
        crate::v128::V128::from($self).to_bytes()
    };
    ($self:ident => [$int:ty; $lanes:literal]) => {{
        let bytes = crate::v128::V128::from($self).to_bytes();
        // SAFETY: all bits are valid in source and destination.
        unsafe { core::mem::transmute::<[u8; 16], [$int; $lanes]>(bytes) }
    }};
}

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

macro_rules! define {
    ($module:ident::$name:ident = [$int:tt; $lanes:tt] as $wasm:literal) => {

mod $module {
    #[cfg(simd_sse2_intrinsics)]
    pub(super) type Repr = crate::arch::__m128i;

    #[derive(Clone, Copy)]
    #[repr(align(16))]
    #[cfg(simd_no_intrinsics)]
    pub(super) struct Repr {
        pub(super) lanes: [$int; $lanes],
    }
}

#[doc = concat!("Represents a [`V128`] interpreted as ", stringify!($lanes), " lanes of ")]
#[doc = concat!("packed [`", stringify!($int), "`] values.")]
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct $name($module::Repr);

impl $name {
    #[doc = concat!("Creates a new 128-bit vector whose ", stringify!($lanes), " lanes are ")]
    #[doc = concat!("filled with the given ", stringify!($int), " value.\n\n")]
    #[doc = concat!("This implements the [`", $wasm, ".splat`](")]
    #[doc = "https://webassembly.github.io/spec/core/exec/instructions.html#exec-vec-splat"]
    #[doc = ") instruction."]
    pub fn splat(x: $int) -> Self {
        // SAFETY: check for `sse2` occurs above.
        #[cfg(simd_sse2_intrinsics)]
        return unsafe { sse2_impl!(<[$int; $lanes]>::splat(x)) };

        #[cfg(simd_no_intrinsics)]
        return Self($module::Repr { lanes: [x; $lanes] });
    }

    //pub fn extract_lane

    //pub fn replace_lane

    #[doc = concat!("Retrieves each ", stringify!($int), " lane in the vector.")]
    pub fn into_lanes(self) -> [$int; $lanes] {
        #[cfg(not(simd_no_intrinsics))]
        return into_lanes!(self => [$int; $lanes]);

        #[cfg(simd_no_intrinsics)]
        return self.0.lanes;
    }
}

impl core::ops::Add for $name {
    type Output = Self;

    #[doc = "Lane-wise wrapping integer addition.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".add`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vibinop"]
    #[doc = ") instruction."]
    fn add(self, rhs: Self) -> Self {
        // SAFETY: check for `sse2` occurs above.
        #[cfg(simd_sse2_intrinsics)]
        return unsafe { sse2_impl!(<[$int; $lanes]>::add(self, rhs)) };

        #[cfg(simd_no_intrinsics)]
        return lanewise_op!((self, rhs) => <[$int; $lanes]>::wrapping_add);
    }
}

impl core::ops::Sub for $name {
    type Output = Self;

    #[doc = "Lane-wise wrapping integer subtraction.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".sub`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vibinop"]
    #[doc = ") instruction."]
    fn sub(self, rhs: Self) -> Self {
        // SAFETY: check for `sse2` occurs above.
        #[cfg(simd_sse2_intrinsics)]
        return unsafe { sse2_impl!(<[$int; $lanes]>::sub(self, rhs)) };

        #[cfg(simd_no_intrinsics)]
        return lanewise_op!((self, rhs) => <[$int; $lanes]>::wrapping_sub);
    }
}

/* impl core::ops::Mul for $name {
    type Output = Self;

    #[doc = "Lane-wise wrapping integer multplication.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".mul`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vibinop"]
    #[doc = ") instruction."]
    fn mul(self, rhs: Self) -> Self {
        // SAFETY: check for `sse2` occurs above.
        #[cfg(simd_sse2_intrinsics)]
        return unsafe { sse2_impl!(<[$int; $lanes]>::mul(self, rhs)) };

        #[cfg(simd_no_intrinsics)]
        return lanewise_op!((self, rhs) => <[$int; $lanes]>::wrapping_mul);
    }
} */

impl From<$name> for V128 {
    fn from(vec: $name) -> Self {
        #[cfg(not(simd_no_intrinsics))]
        return vec.0.into();

        #[cfg(simd_no_intrinsics)]
        return Self::from_bytes(vec.0.lanes.map(|i| i as u8));
    }
}

impl From<V128> for $name {
    #[doc = concat!("Interprets the contents of the [`V128`] as ", stringify!($lanes), " lanes ")]
    #[doc = concat!("of packed [`", stringify!($int), "`] values.")]
    fn from(vec: V128) -> Self {
        #[cfg(not(simd_no_intrinsics))]
        return Self(vec.into());
    }
}

#[cfg(all(feature = "simd-intrinsics", target_feature = "sse2"))]
impl From<crate::arch::__m128i> for $name {
    fn from(vec: crate::arch::__m128i) -> Self {
        Self(vec)
    }
}

#[cfg(all(feature = "simd-intrinsics", target_feature = "sse2"))]
impl From<$name> for crate::arch::__m128i {
    fn from(vec: $name) -> Self {
        vec.0
    }
}

impl core::fmt::Debug for $name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(self.into_lanes().into_iter())
            .finish()
    }
}

impl core::fmt::LowerHex for $name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[derive(Clone, Copy)]
        struct Lane($int);

        impl core::fmt::Debug for Lane {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerHex::fmt(&self.0, f)
            }
        }

        f.debug_list()
            .entries(self.into_lanes().into_iter().map(Lane))
            .finish()
    }
}

impl core::fmt::UpperHex for $name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[derive(Clone, Copy)]
        struct Lane($int);

        impl core::fmt::Debug for Lane {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperHex::fmt(&self.0, f)
            }
        }

        f.debug_list()
            .entries(self.into_lanes().into_iter().map(Lane))
            .finish()
    }
}

    };
}

define!(i8x16::I8x16 = [i8; 16] as "i8x16");
define!(u8x16::U8x16 = [u8; 16] as "i8x16");
define!(i16x8::I16x8 = [i16; 8] as "i16x8");
define!(u16x8::U16x8 = [u16; 8] as "i16x8");
define!(i32x4::I32x4 = [i32; 4] as "i32x4");
define!(u32x4::U32x4 = [u32; 4] as "i32x4");
define!(i64x2::I64x2 = [i64; 2] as "i64x2");
define!(u64x2::U64x2 = [u64; 2] as "i64x2");
