#[cfg(simd_sse2_intrinsics)]
type Repr = crate::simd::arch::__m128i;

#[derive(Clone, Copy)]
#[repr(align(16))]
#[cfg(simd_no_intrinsics)]
struct Repr {
    lanes: [i8; 16],
}

/// Represents a [128-bit vector] interpreted as 16 lanes of packed 8-bit integers.
///
/// [128-bit vector]: https://webassembly.github.io/spec/core/syntax/values.html#vectors
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct I8x16(Repr);

impl I8x16 {
    /// Creates a new 128-bit vector whose 16 lanes are filled with the given 8-bit integer value.
    ///
    /// Implements the [`i8x16.splat`] instruction.
    ///
    /// [`i8x16.splat`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-vec
    pub fn splat(x: i8) -> Self {
        #[cfg(simd_sse2_intrinsics)]
        return {
            // SAFETY: check for `sse2` occurs above.
            let lanes = unsafe { crate::simd::arch::_mm_set1_epi8(x) };
            Self(lanes)
        };

        #[cfg(simd_no_intrinsics)]
        return Self(Repr { lanes: [x; 16] });
    }

    /// Retrieves each 8-bit integer lane in the vector.
    pub fn into_lanes(self) -> [i8; 16] {
        #[cfg(simd_sse2_intrinsics)]
        return {
            let bytes = crate::simd::v128::V128::from(self).to_bytes();
            // SAFETY: safe to transmute `i8` and `u8`, and array sizes are the same.
            unsafe { core::mem::transmute::<[u8; 16], [i8; 16]>(bytes) }
        };

        #[cfg(simd_no_intrinsics)]
        return self.0.lanes;
    }
}

impl From<I8x16> for crate::simd::v128::V128 {
    fn from(v: I8x16) -> Self {
        #[cfg(simd_sse2_intrinsics)]
        return v.0.into();

        #[cfg(simd_no_intrinsics)]
        return Self::from_bytes(v.0.lanes.map(|i| i as u8));
    }
}

impl From<crate::simd::v128::V128> for I8x16 {
    fn from(v: crate::simd::v128::V128) -> Self {
        #[cfg(simd_sse2_intrinsics)]
        return Self(v.into());

        #[cfg(simd_no_intrinsics)]
        return Self(Repr {
            lanes: v.to_bytes().map(|u| u as i8),
        });
    }
}

/// Implements the [`i8x16.add`] instruction.
///
/// [`i8x16.add`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-vec
impl core::ops::Add for I8x16 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        #[cfg(simd_sse2_intrinsics)]
        return {
            // SAFETY: check for `sse2` feature occurs above.
            let sum = unsafe { crate::simd::arch::_mm_add_epi8(self.0, rhs.0) };
            Self(sum)
        };

        #[cfg(simd_no_intrinsics)]
        return Self(Repr {
            lanes: [
                self.0.lanes[0].wrapping_add(rhs.0.lanes[0]),
                self.0.lanes[1].wrapping_add(rhs.0.lanes[1]),
                self.0.lanes[2].wrapping_add(rhs.0.lanes[2]),
                self.0.lanes[3].wrapping_add(rhs.0.lanes[3]),
                self.0.lanes[4].wrapping_add(rhs.0.lanes[4]),
                self.0.lanes[5].wrapping_add(rhs.0.lanes[5]),
                self.0.lanes[6].wrapping_add(rhs.0.lanes[6]),
                self.0.lanes[7].wrapping_add(rhs.0.lanes[7]),
                self.0.lanes[8].wrapping_add(rhs.0.lanes[8]),
                self.0.lanes[9].wrapping_add(rhs.0.lanes[9]),
                self.0.lanes[10].wrapping_add(rhs.0.lanes[10]),
                self.0.lanes[11].wrapping_add(rhs.0.lanes[11]),
                self.0.lanes[12].wrapping_add(rhs.0.lanes[12]),
                self.0.lanes[13].wrapping_add(rhs.0.lanes[13]),
                self.0.lanes[14].wrapping_add(rhs.0.lanes[14]),
                self.0.lanes[15].wrapping_add(rhs.0.lanes[15]),
            ],
        });
    }
}

impl core::fmt::Debug for I8x16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[derive(Clone, Copy)]
        struct Lane(i8);

        impl core::fmt::Debug for Lane {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{:#04X}", self.0)
            }
        }

        f.debug_list()
            .entries(self.into_lanes().into_iter().map(Lane))
            .finish()
    }
}
