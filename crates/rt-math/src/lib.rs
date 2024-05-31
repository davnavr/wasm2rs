//! Runtime support functions for simple math operations in `wasm2rs`.

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![forbid(unsafe_code)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

mod float;

pub mod nan;

pub use float::{f32_max, f32_min, f64_max, f64_min};

use core::fmt::Display;

/// Error type used if an integer denominator is zero.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct DivisionByZeroError;

/// Error type used if an attempt to convert an integer to a smaller bitwidth fails, or if an
/// integer division operation overflows.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct IntegerOverflowError;

/// Error type used if an attempt was made to convert a *NaN* floating-point value to an integer.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct NanToIntegerError;

// Most of these error messages are taken from the WASM spec tests.
impl Display for DivisionByZeroError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("integer division by zero")
    }
}

impl Display for IntegerOverflowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("integer overflow")
    }
}

impl Display for NanToIntegerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid conversion to integer")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DivisionByZeroError {}

#[cfg(feature = "std")]
impl std::error::Error for IntegerOverflowError {}

#[cfg(feature = "std")]
impl std::error::Error for NanToIntegerError {}

/// Error type used when an integer division operation fails.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IntegerDivisionError {
    /// See [`DivisionByZeroError`].
    DivisionByZero,
    /// See [`IntegerOverflowError`].
    Overflow,
}

impl From<DivisionByZeroError> for IntegerDivisionError {
    fn from(error: DivisionByZeroError) -> Self {
        let _ = error;
        Self::DivisionByZero
    }
}

impl From<IntegerOverflowError> for IntegerDivisionError {
    fn from(error: IntegerOverflowError) -> Self {
        let _ = error;
        Self::Overflow
    }
}

impl Display for IntegerDivisionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DivisionByZero => Display::fmt(&DivisionByZeroError, f),
            Self::Overflow => Display::fmt(&IntegerOverflowError, f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IntegerDivisionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::DivisionByZero => &DivisionByZeroError,
            Self::Overflow => &IntegerOverflowError,
        })
    }
}

/// Error type used when an attempt to convert a floating-point value to an integer would result in a
/// value that is out of range, or the floating-point value was *NaN*.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FloatToIntegerError {
    /// See [`NanToIntegerError`].
    InvalidConversion,
    /// See [`IntegerOverflowError`].
    Overflow,
}

impl From<NanToIntegerError> for FloatToIntegerError {
    fn from(error: NanToIntegerError) -> Self {
        let _ = error;
        Self::InvalidConversion
    }
}

impl From<IntegerOverflowError> for FloatToIntegerError {
    fn from(error: IntegerOverflowError) -> Self {
        let _ = error;
        Self::Overflow
    }
}

impl Display for FloatToIntegerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidConversion => Display::fmt(&NanToIntegerError, f),
            Self::Overflow => Display::fmt(&IntegerOverflowError, f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FloatToIntegerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::InvalidConversion => &NanToIntegerError,
            Self::Overflow => &IntegerOverflowError,
        })
    }
}

macro_rules! int_div {
    {$(
        $signed:ty => $div:ident = $div_name:literal $(as $unsigned:ty)?;
    )*} => {$(
        #[doc = concat!(
            "Implementation for the [`", $div_name, "`] instruction.\n\nCalculates `num / denom`,",
            " trapping on division by zero or overflow.\n\n",
            $(
                "The `num` and `denom` are interpreted as an [`", stringify!($unsigned), "`] ",
                "value, and the resulting [`", stringify!($unsigned), "`] quotient is ",
                "reinterpreted as an [`", stringify!($signed), "`] value.\n\n",
            )?
            "[`", $div_name, "`]: ",
            "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"
        )]
        #[inline]
        pub fn $div(num: $signed, denom: $signed) -> Result<$signed, IntegerDivisionError> {
            match (num $(as $unsigned)?).checked_div(denom $(as $unsigned)?) {
                Some(quot) => Ok(quot as $signed),
                _ if denom == 0 => Err(IntegerDivisionError::DivisionByZero),
                _ => Err(IntegerDivisionError::Overflow),
            }
        }
    )*};
}

int_div! {
    i32 => i32_div_s = "i32.div_s";
    i32 => i32_div_u = "i32.div_u" as u32;
    i64 => i64_div_s = "i64.div_s";
    i64 => i64_div_u = "i64.div_u" as u64;
}

macro_rules! int_rem {
    {$(
        $signed:ty => $rem:ident = $rem_name:literal $(as $unsigned:ty)?;
    )*} => {$(
        #[doc = concat!(
            "Implementation for the [`", $rem_name, "`] instruction.\n\nCalculates `num % denom`,",
            " trapping on [division by zero].\n\n",
            $(
                "The `num` and `denom` are interpreted as an [`", stringify!($unsigned), "`] ",
                "value, and the resulting [`", stringify!($unsigned), "`] remainder is ",
                "reinterpreted as an [`", stringify!($signed), "`] value.\n\n",
            )?
            "[division by zero]: DivisionByZeroError\n",
            "[`", $rem_name, "`]: ",
            "https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"
        )]
        #[inline]
        pub fn $rem(num: $signed, denom: $signed) -> Result<$signed, DivisionByZeroError>
        {
            if denom == 0 {
                Err(DivisionByZeroError)
            } else {
                Ok((num $(as $unsigned)?).wrapping_rem(denom $(as $unsigned)?) as $signed)
            }
        }
    )*};
}

int_rem! {
    i32 => i32_rem_s = "i32.rem_s";
    i32 => i32_rem_u = "i32.rem_u" as u32;
    i64 => i64_rem_s = "i64.rem_s";
    i64 => i64_rem_u = "i64.rem_u" as u64;
}

macro_rules! prefer_right {
    ($left: ty | $right: ty) => {
        $right
    };
    ($left: ty) => {
        $left
    };
}

macro_rules! iXX_trunc_fXX {
    {$(
        $float:ty => $trunc:ident = $trunc_name:literal -> $int:ty $(as $reinterpret:ty)?;
    )*} => {$(
        #[doc = concat!(
            "Implementation for the [`", $trunc_name, "`] instruction.\n\n",
            "Casts a [`", stringify!($float), "`] value to an [`", stringify!($int), "`], ",
            "[trapping] on [`", stringify!($float), "::NAN`], [`", stringify!($float),
            "::INFINITY`], [`",  stringify!($float), "::NEG_INFINITY`], and if the [`",
            stringify!($float), "`] value is too large to fit into an [`", stringify!($int),
            "`].\n\n",
            $(
                "The result is then reinterpreted as an [`", stringify!($reinterpret), "`] value.",
                "\n\n",
            )?
            "[trapping]: NanToIntegerError\n",
            "[`", $trunc_name,
            "`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-numeric"
        )]
        #[inline]
        pub fn $trunc(
            value: $float
        ) -> Result<prefer_right!($int $(| $reinterpret)?), FloatToIntegerError>
        {
            match <$int as num_traits::cast::NumCast>::from(value) {
                Some(n) => Ok(n $(as $reinterpret)?),
                None => Err(if value.is_nan() {
                    FloatToIntegerError::InvalidConversion
                } else {
                    FloatToIntegerError::Overflow
                }),
            }
        }
    )*};
}

iXX_trunc_fXX! {
    f32 => i32_trunc_f32_s = "i32.trunc_f32_s" -> i32;
    f64 => i32_trunc_f64_s = "i32.trunc_f64_s" -> i32;
    f32 => i32_trunc_f32_u = "i32.trunc_f32_u" -> u32 as i32;
    f64 => i32_trunc_f64_u = "i32.trunc_f64_u" -> u32 as i32;
    f32 => i64_trunc_f32_s = "i64.trunc_f32_s" -> i64;
    f64 => i64_trunc_f64_s = "i64.trunc_f64_s" -> i64;
    f32 => i64_trunc_f32_u = "i64.trunc_f32_u" -> u64 as i64;
    f64 => i64_trunc_f64_u = "i64.trunc_f64_u" -> u64 as i64;
}
