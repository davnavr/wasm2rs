//! Defines the interpretations for [`V128`].

use crate::v128::{implementation, V128};

macro_rules! define {
    ($name:ident = [$num:tt; $lanes:tt] as $wasm:literal) => {

#[doc = concat!("Represents a [`V128`] interpreted as ", stringify!($lanes), " lanes of ")]
#[doc = concat!("packed [`", stringify!($num), "`] values.\n\n")]
#[doc = concat!("Corresponds to the [`", $wasm, "`](")]
#[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-shape)"]
#[doc = "interpretation in WebAssembly."]
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct $name(pub(in crate::v128) implementation::$name);

impl $name {
    #[doc = concat!("Creates a new 128-bit vector whose ", stringify!($lanes), " lanes are ")]
    #[doc = concat!("filled with the given ", stringify!($num), " value.\n\n")]
    #[doc = concat!("This implements the [`", $wasm, ".splat`](")]
    #[doc = "https://webassembly.github.io/spec/core/exec/instructions.html#exec-vec-splat"]
    #[doc = ") instruction."]
    pub fn splat(x: $num) -> Self {
        Self(Self::splat_impl(x))
    }

    //pub fn extract_lane

    //pub fn replace_lane

    //pub fn from_lanes(lanes: [$num; $lanes]) -> Self {}

    #[doc = concat!("Returns an array containing each [`", stringify!($num), "`] lane in the")]
    #[doc = "vector."]
    pub fn into_lanes(self) -> [$num; $lanes] {
        Self::into_lanes_impl(self.0)
    }
}

//impl From<[$num; $lanes]> for $name {}

//impl From<[$num; $lanes]> for V128 {}

impl From<$name> for [$num; $lanes] {
    #[doc = concat!("Calls [`", stringify!($name), "::into_lanes()`].")]
    fn from(vec: $name) -> Self {
        vec.into_lanes()
    }
}

impl From<$name> for V128 {
    fn from(vec: $name) -> Self {
        vec.0.into()
    }
}

impl From<V128> for $name {
    #[doc = concat!("Interprets the contents of the [`V128`] as ", stringify!($lanes), " lanes ")]
    #[doc = concat!("of packed [`", stringify!($num), "`] values.")]
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

impl core::ops::Add<&$name> for &$name {
    type Output = $name;

    fn add(self, rhs: &$name) -> $name {
        *self + *rhs
    }
}

impl core::ops::Add<&$name> for $name {
    type Output = $name;

    fn add(self, rhs: &$name) -> $name {
        self + *rhs
    }
}

impl<'a> core::ops::Add<$name> for &'a $name {
    type Output = $name;

    fn add(self, rhs: $name) -> $name {
        *self + rhs
    }
}

impl core::ops::AddAssign for $name {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl core::ops::Sub<&$name> for &$name {
    type Output = $name;

    fn sub(self, rhs: &$name) -> $name {
        *self - *rhs
    }
}

impl core::ops::Sub<&$name> for $name {
    type Output = $name;

    fn sub(self, rhs: &$name) -> $name {
        self - *rhs
    }
}

impl<'a> core::ops::Sub<$name> for &'a $name {
    type Output = $name;

    fn sub(self, rhs: $name) -> $name {
        *self - rhs
    }
}

impl core::ops::SubAssign for $name {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

// TODO: Mul/Div on borrows and Mul/DivAssign.
// TODO: Bitwise operations on borrows and assign.

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

#[doc(hidden)]
#[macro_export]
macro_rules! v128_float_interpretations {
    ($macro:ident) => {
        $macro!(F32x4 = [f32; 4] as "f32x4");
        $macro!(F64x2 = [f64; 2] as "f64x2");
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! v128_interpretations {
    ($macro:ident) => {
        $crate::v128_integer_interpretations!($macro);
        $crate::v128_float_interpretations!($macro);
    };
}

v128_interpretations!(define);
