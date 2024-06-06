//! Defines operations on the floating-point interpretations for [`V128`].

use crate::v128;

macro_rules! define {
    ($name:ident = [$fnn:tt; $lanes:tt] as $wasm:literal) => {

impl core::ops::Add for v128::$name {
    type Output = Self;

    #[doc = "Lane-wise IEEE-754 addition.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".add`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vfbinop) "]
    #[doc = "instruction."]
    fn add(self, rhs: Self) -> Self {
        Self(Self::add_impl(self.0, rhs.0))
    }
}

impl core::ops::Sub for v128::$name {
    type Output = Self;

    #[doc = "Lane-wise IEEE-754 subtraction.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".sub`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vfbinop"]
    #[doc = ") instruction."]
    fn sub(self, rhs: Self) -> Self {
        Self(Self::sub_impl(self.0, rhs.0))
    }
}

impl core::ops::Mul for v128::$name {
    type Output = Self;

    #[doc = "Lane-wise IEEE-754 multiplication.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".mul`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vfbinop"]
    #[doc = ") instruction."]
    fn mul(self, rhs: Self) -> Self {
        Self(Self::mul_impl(self.0, rhs.0))
    }
}

impl core::ops::Div for v128::$name {
    type Output = Self;

    #[doc = "Lane-wise IEEE-754 division.\n\n"]
    #[doc = concat!("This implements the [`", $wasm, ".div`](")]
    #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-vfbinop"]
    #[doc = ") instruction."]
    fn div(self, rhs: Self) -> Self {
        Self(Self::div_impl(self.0, rhs.0))
    }
}

    };
}

crate::v128_float_interpretations!(define);
