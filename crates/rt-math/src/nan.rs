//! Constants and functions for testing *NaN* values.
//!
//! The functions in this module are mainly used in the [specification tests], where an assertion
//! can test that a floating-point value is `nan:canonical` or `nan:arithmetic`.
//!
//! Refer to the [WebAssembly specification] for the exact definitions for *arithmetic* and *canonical NaN*s.
//!
//! [specification tests]: https://github.com/WebAssembly/spec/blob/wg-2.0.draft1/interpreter/README.md#scripts
//! [WebAssembly specification]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point

const F32_PAYLOAD_HIGH_BIT: u32 = 1u32 << 22;
const F32_PAYLOAD_LOW_BITS: u32 = 0x003F_FFFF;

/// The [*canonical NaN*] value for [`f32`]s.
///
/// [*canonical NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub const F32_CANONICAL: u32 = (0xFFu32 << 23) | F32_PAYLOAD_HIGH_BIT;

/// The negative [*canonical NaN*] value for [`f32`]s.
///
/// [*canonical NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub const F32_NEG_CANONICAL: u32 = F32_CANONICAL | (1u32 << 31);

/// Checks if the given [`f32`] is a [positive] or [negative] [*canonical NaN*].
///
/// [positive]: F32_CANONICAL
/// [negative]: F32_NEG_CANONICAL
/// [*canonical NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub fn is_canonical_f32(value: f32) -> bool {
    matches!(value.to_bits(), F32_CANONICAL | F32_NEG_CANONICAL)
}

/// Checks if the given [`f32`] is an [*arithmetic NaN*].
///
/// [*arithmetic NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub fn is_arithmetic_f32(value: f32) -> bool {
    let bits = value.to_bits();
    value.is_nan() && (bits & F32_PAYLOAD_HIGH_BIT != 0) && (bits & F32_PAYLOAD_LOW_BITS != 0)
}

const PAYLOAD_HIGH_BIT_F64: u64 = 1u64 << 51;
const PAYLOAD_LOW_BITS_F64: u64 = 0x0007_FFFF_FFFF_FFFF;

/// The [*canonical NaN*] value for [`f64`]s.
///
/// [*canonical NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub const F64_CANONICAL: u64 = (0x07FFu64 << 52) | PAYLOAD_HIGH_BIT_F64;

/// The negative [*canonical NaN*] value for [`f64`]s.
///
/// [*canonical NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub const F64_NEG_CANONICAL: u64 = F64_CANONICAL | (1u64 << 63);

/// Checks if the given [`f64`] is a [positive] or [negative] [*canonical NaN*].
///
/// [positive]: F64_CANONICAL
/// [negative]: F64_NEG_CANONICAL
/// [*canonical NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub fn is_canonical_f64(value: f64) -> bool {
    matches!(value.to_bits(), F64_CANONICAL | F64_NEG_CANONICAL)
}

/// Checks if the given [`f64`] is an [*arithmetic NaN*].
///
/// [*arithmetic NaN*]: https://webassembly.github.io/spec/core/syntax/values.html#floating-point
pub fn is_arithmetic_f64(value: f64) -> bool {
    let bits = value.to_bits();
    value.is_nan() && (bits & PAYLOAD_HIGH_BIT_F64 != 0) && (bits & PAYLOAD_LOW_BITS_F64 != 0)
}
