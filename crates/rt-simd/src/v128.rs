//! Types and operations providing an implementation of the WebAssembly [fixed-width SIMD] proposal.
//!
//! [fixed-width SIMD]: https://github.com/webassembly/simd

// mod i16x8;
// mod i8x16;
mod fshape;
mod interpretations;
mod ishape;

crate::cfg_sse2_intrinsics! {
    #[path = "v128/sse2.rs"]
    mod implementation;
}

crate::cfg_no_intrinsics! {
    #[path = "v128/fallback.rs"]
    mod implementation;
}

pub use interpretations::{F32x4, F64x2, I16x8, I32x4, I64x2, I8x16, U16x8, U32x4, U64x2, U8x16};

#[derive(Clone, Copy)]
#[repr(align(16))]
struct Bytes {
    bytes: [u8; 16],
}

/// Represents a generic [128-bit vector] whose interpretation is not specified.
///
/// # Interpretations
///
/// Specific interpretations of the lanes of a [`V128`] are provided as separate types
/// corresponding to the WebAssembly vector [*shape*s], along with operations (e.g. lane-wise
/// [`Add`]) for those interpretations. These currently include:
/// - `i8x16`: [`I8x16`] or [`U8x16`]
/// - `i16x8`: [`I16x8`] or [`U16x8`]
/// - `i32x4`: [`I32x4`] or [`U32x4`]
/// - `i64x2`: [`I64x2`] or [`U64x2`]
/// - `f32x4`: [`F32x4`]
/// - `f64x2`: [`F64x2`]
///
/// Various [`From`] implementations are provided for interpreting the lanes of a [`V128`]
/// differently.
///
/// # Disabling `simd-intrinsics`
///
/// When the `simd-intrinsics` feature flag is **not** enabled, operations are implemented in
/// normal Rust code (which may be optimized by the Rust compiler's auto-vectorization) rather than
/// target-archecture specific SIMD intrinsics.
///
/// [128-bit vector]: https://webassembly.github.io/spec/core/syntax/values.html#vectors
/// [*shape*s]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-shape
/// [`Add`]: core::ops::Add
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct V128(implementation::V128);

impl Default for V128 {
    fn default() -> Self {
        Self::zero()
    }
}

impl V128 {
    /// Returns a 128-bit vector whose bits are all set to zero.
    pub fn zero() -> Self {
        Self(Self::zero_impl())
    }

    /// Interprets a 128-bit integer value as a 128-bit vector.
    pub fn from_bits(bits: u128) -> Self {
        Self::from_bytes(bits.to_le_bytes())
    }

    /// Returns a 128-bit integer value containing the contents of the 128-bit vector.
    pub fn to_bits(self) -> u128 {
        u128::from_le_bytes(self.to_bytes())
    }

    /// Constructs a 128-bit vector from bytes in little-endian order.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Self::from_bytes_impl(bytes))
    }

    /// Returns the representation of a 128-bit vector as a byte array in little-endian order.
    pub fn to_bytes(self) -> [u8; 16] {
        self.to_bytes_impl()
    }
}

impl core::fmt::UpperHex for V128 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#034X}", self.to_bits())
    }
}

impl core::fmt::LowerHex for V128 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#034x}", self.to_bits())
    }
}

impl core::fmt::Debug for V128 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(self, f)
    }
}
