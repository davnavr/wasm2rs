//! Types and operations providing an implementation of the WebAssembly [fixed-width SIMD] proposal.
//!
//! [fixed-width SIMD]: https://github.com/webassembly/simd

// mod i16x8;
// mod i8x16;
mod ishape;

pub use ishape::{I16x8, I8x16, U8x16};

#[cfg(simd_sse2_intrinsics)]
type Repr = crate::arch::__m128i;

#[derive(Clone, Copy)]
#[repr(align(16))]
#[cfg(simd_no_intrinsics)]
struct Repr {
    bits: u128,
}

/// Represents a generic [128-bit vector] whose interpretation is not specified.
///
/// # Interpretations
///
/// Specific interpretations of the lanes of a [`V128`] are provided as separate types, along with
/// operations (e.g. lane-wise [`Add`]) for those interpretations. These currently include:
/// - [`I8x16`]
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
/// [`Add`]: core::ops::Add
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct V128(Repr);

impl V128 {
    /// Interprets a 128-bit integer value as a 128-bit vector.
    pub fn from_bits(bits: u128) -> Self {
        #[cfg(simd_sse2_intrinsics)]
        return Self::from_bytes(bits.to_le_bytes());

        #[cfg(simd_no_intrinsics)]
        return Self(Repr { bits });
    }

    /// Returns a 128-bit integer value containing the contents of the 128-bit vector.
    pub fn to_bits(self) -> u128 {
        #[cfg(not(simd_no_intrinsics))]
        return u128::from_le_bytes(self.to_bytes());

        #[cfg(simd_no_intrinsics)]
        return self.0.bits;
    }

    /// Constructs a 128-bit vector from bytes in little-endian order.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        #[cfg(simd_sse2_intrinsics)]
        return {
            // SAFETY: check for `sse2` target feature occurs above.
            #[allow(clippy::cast_possible_truncation)]
            let v = unsafe {
                crate::arch::_mm_setr_epi8(
                    bytes[15] as i8,
                    bytes[14] as i8,
                    bytes[13] as i8,
                    bytes[12] as i8,
                    bytes[11] as i8,
                    bytes[10] as i8,
                    bytes[9] as i8,
                    bytes[8] as i8,
                    bytes[7] as i8,
                    bytes[6] as i8,
                    bytes[5] as i8,
                    bytes[4] as i8,
                    bytes[3] as i8,
                    bytes[2] as i8,
                    bytes[1] as i8,
                    bytes[0] as i8,
                )
            };

            Self(v)
        };

        #[cfg(simd_no_intrinsics)]
        return Self::from_bits(u128::from_le_bytes(bytes));
    }

    /// Returns the representation of a 128-bit vector as a byte array in little-endian order.
    pub fn to_bytes(self) -> [u8; 16] {
        #[cfg(not(simd_no_intrinsics))]
        #[repr(align(16))]
        struct Bytes {
            bytes: [u8; 16],
        }

        #[cfg(not(simd_no_intrinsics))]
        let mut bytes = Bytes { bytes: [0u8; 16] };

        #[cfg(simd_sse2_intrinsics)]
        return {
            // SAFETY: check for `sse2` target feature occurs above.
            // SAFETY: `bytes.bytes` is aligned to 16 bytes.
            unsafe {
                crate::arch::_mm_storeu_si128(
                    (&mut bytes) as *mut Bytes as *mut crate::arch::__m128i,
                    self.0,
                );
            }

            bytes.bytes
        };

        #[cfg(simd_no_intrinsics)]
        return self.0.bits.to_le_bytes();
    }
}

#[cfg(all(feature = "simd-intrinsics", target_feature = "sse2"))]
impl From<crate::arch::__m128i> for V128 {
    fn from(v: crate::arch::__m128i) -> Self {
        Self(v)
    }
}

#[cfg(all(feature = "simd-intrinsics", target_feature = "sse2"))]
impl From<V128> for crate::arch::__m128i {
    fn from(v: V128) -> Self {
        v.0
    }
}

#[cfg(all(feature = "simd-intrinsics", target_feature = "sse2"))]
impl From<crate::arch::__m128> for V128 {
    fn from(v: crate::arch::__m128) -> Self {
        // SAFETY: this is compiled only when the `sse2` target feature is enabled.
        let v = unsafe { crate::arch::_mm_castps_si128(v) };
        Self(v)
    }
}

#[cfg(all(feature = "simd-intrinsics", target_feature = "sse2"))]
impl From<V128> for crate::arch::__m128 {
    fn from(v: V128) -> Self {
        // SAFETY: this is compiled only when the `sse2` target feature is enabled.
        unsafe { crate::arch::_mm_castsi128_ps(v.0) }
    }
}

impl core::fmt::Debug for V128 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#034X}", self.to_bits())
    }
}
