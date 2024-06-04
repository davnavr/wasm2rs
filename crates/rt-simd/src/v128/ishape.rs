//! Defines the integer interpretations for [`V128`].

use crate::v128::{implementation, V128};

macro_rules! define_mul {
    (I8x16 = $($_:tt)+) => {
        // Undefined
    };
    (U8x16 = $($_:tt)+) => {
        // Undefined
    };
    ($name:ident = [$int:tt; $lanes:tt] as $wasm:literal) => {
        impl core::ops::Mul for $name {
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
        }
    }
}

// TODO: splat, From/Into<V128>, Debug, lane methods, etc. should be shared with fshape.
macro_rules! define {
    ($name:ident = [$int:tt; $lanes:tt] as $wasm:literal) => {

#[doc = concat!("Represents a [`V128`] interpreted as ", stringify!($lanes), " lanes of ")]
#[doc = concat!("packed [`", stringify!($int), "`] values.\n\n")]
#[doc = concat!("Corresponds to the [`", $wasm, "`](")]
#[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-shape)"]
#[doc = "interpretation in WebAssembly."]
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct $name(pub(in crate::v128) implementation::$name);

impl $name {
    #[doc = concat!("Creates a new 128-bit vector whose ", stringify!($lanes), " lanes are ")]
    #[doc = concat!("filled with the given ", stringify!($int), " value.\n\n")]
    #[doc = concat!("This implements the [`", $wasm, ".splat`](")]
    #[doc = "https://webassembly.github.io/spec/core/exec/instructions.html#exec-vec-splat"]
    #[doc = ") instruction."]
    pub fn splat(x: $int) -> Self {
        Self(Self::splat_impl(x))
    }

    //pub fn extract_lane

    //pub fn replace_lane

    #[doc = concat!("Returns an array containing each ", stringify!($int), " lane in the vector.")]
    pub fn into_lanes(self) -> [$int; $lanes] {
        Self::into_lanes_impl(self.0)
    }
}

impl core::ops::Add for $name {
    type Output = Self;

    #[doc = "Lane-wise twos-complement wrapping integer addition.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".add`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vibinop) "]
    #[doc = "instruction."]
    fn add(self, rhs: Self) -> Self {
        Self(Self::add_impl(self.0, rhs.0))
    }
}

impl core::ops::Sub for $name {
    type Output = Self;

    #[doc = "Lane-wise twos-complement wrapping integer subtraction.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".sub`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vibinop"]
    #[doc = ") instruction."]
    fn sub(self, rhs: Self) -> Self {
        Self(Self::sub_impl(self.0, rhs.0))
    }
}

// define_mul!($name = [$int; $lanes] as $wasm);

impl From<$name> for V128 {
    fn from(vec: $name) -> Self {
        vec.0.into()
    }
}

impl From<V128> for $name {
    #[doc = concat!("Interprets the contents of the [`V128`] as ", stringify!($lanes), " lanes ")]
    #[doc = concat!("of packed [`", stringify!($int), "`] values.")]
    fn from(vec: V128) -> Self {
        Self(vec.into())
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

#[doc(hidden)]
#[macro_export]
macro_rules! v128_integer_interpretations {
    ($macro:ident) => {
        $macro!(I8x16 = [i8; 16] as "i8x16");
        $macro!(U8x16 = [u8; 16] as "i8x16");
        $macro!(I16x8 = [i16; 8] as "i16x8");
        $macro!(U16x8 = [u16; 8] as "i16x8");
        $macro!(I32x4 = [i32; 4] as "i32x4");
        $macro!(U32x4 = [u32; 4] as "i32x4");
        $macro!(I64x2 = [i64; 2] as "i64x2");
        $macro!(U64x2 = [u64; 2] as "i64x2");
    };
}

v128_integer_interpretations!(define);
