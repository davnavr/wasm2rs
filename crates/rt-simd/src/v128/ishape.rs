//! Defines operations on the integer interpretations for [`V128`].

use crate::v128;

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

macro_rules! define {
    ($name:ident = [$int:tt; $lanes:tt] as $wasm:literal) => {

impl core::ops::Add for v128::$name {
    type Output = Self;

    #[doc = "Lane-wise twos-complement wrapping integer addition.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".add`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vibinop) "]
    #[doc = "instruction."]
    fn add(self, rhs: Self) -> Self {
        Self(Self::add_impl(self.0, rhs.0))
    }
}

impl core::ops::Sub for v128::$name {
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

    };
}

crate::v128_integer_interpretations!(define);
