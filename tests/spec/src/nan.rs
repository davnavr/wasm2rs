//! Constants for well-known *NaN* values.
//!
//! Refer to the following documentation:
//! - For the definitions of *arithmetic* and *canonical* *NaN*s: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
//! - For the format of [`f32`] values: https://en.wikipedia.org/wiki/Single-precision_floating-point_format
//! - For the format of [`f64`] values: https://en.wikipedia.org/wiki/Double-precision_floating-point_format

const PAYLOAD_HIGH_BIT_F32: u32 = 1u32 << 22;
const PAYLOAD_LOW_BITS_F32: u32 = 0x003F_FFFF;

pub const CANONICAL_F32: u32 = (0xFFu32 << 23) | PAYLOAD_HIGH_BIT_F32;
pub const NEG_CANONICAL_F32: u32 = CANONICAL_F32 | (1u32 << 31);

pub fn is_canonical_f32(value: f32) -> bool {
    matches!(value.to_bits(), CANONICAL_F32 | NEG_CANONICAL_F32)
}

pub fn is_arithmetic_f32(value: f32) -> bool {
    let bits = value.to_bits();
    value.is_nan() && (bits & PAYLOAD_HIGH_BIT_F32 != 0) && (bits & PAYLOAD_LOW_BITS_F32 != 0)
}

const PAYLOAD_HIGH_BIT_F64: u64 = 1u64 << 51;
const PAYLOAD_LOW_BITS_F64: u64 = 0x0007_FFFF_FFFF_FFFF;

pub const CANONICAL_F64: u64 = (0x07FFu64 << 52) | PAYLOAD_HIGH_BIT_F64;
pub const NEG_CANONICAL_F64: u64 = CANONICAL_F64 | (1u64 << 63);

pub fn is_canonical_f64(value: f64) -> bool {
    matches!(value.to_bits(), CANONICAL_F64 | NEG_CANONICAL_F64)
}

pub fn is_arithmetic_f64(value: f64) -> bool {
    let bits = value.to_bits();
    value.is_nan() && (bits & PAYLOAD_HIGH_BIT_F64 != 0) && (bits & PAYLOAD_LOW_BITS_F64 != 0)
}
