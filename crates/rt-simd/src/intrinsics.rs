//! Re-exports target architecture-specific intrinsics.
//!
//! These are only provided on supported platforms when the `simd-intrinsics` feature flag is
//! enabled.

crate::cfg_sse2_intrinsics! {

/// Provides [SSE2] SIMD intrinsics available on `x86` and `x86_64` platforms.
///
/// [SSE2]: https://en.wikipedia.org/wiki/SSE2
pub(crate) mod sse2 {
    #[cfg(target_arch = "x86")]
    use core::arch::x86 as intrin;

    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64 as intrin;

    pub(crate) use intrin::{
        __m128, __m128d, __m128i, _mm_add_epi16, _mm_add_epi32, _mm_add_epi64, _mm_add_epi8,
        _mm_add_pd, _mm_add_ps, _mm_castpd_si128, _mm_castps_si128, _mm_castsi128_pd,
        _mm_castsi128_ps, _mm_div_pd, _mm_div_ps, _mm_loadu_pd, _mm_loadu_ps, _mm_loadu_si128,
        _mm_mul_pd, _mm_mul_ps, _mm_set1_epi16, _mm_set1_epi32, _mm_set1_epi64x, _mm_set1_epi8,
        _mm_set1_pd, _mm_set1_ps, _mm_setr_epi8, _mm_setzero_pd, _mm_setzero_ps, _mm_setzero_si128,
        _mm_store_pd, _mm_store_ps, _mm_store_si128, _mm_sub_epi16, _mm_sub_epi32, _mm_sub_epi64,
        _mm_sub_epi8, _mm_sub_pd, _mm_sub_ps,
    };
}

}

crate::cfg_neon_intrinsics! {

/// Provides [Neon] SIMD intrinsics, currently only available on stable Rust on the `aarch64` platform.
///
/// [Neon]: https://www.arm.com/technologies/neon
pub(crate) mod neon {
    pub(crate) use core::arch::aarch64 as arm;

    // Currently nightly-only
    // pub(crate) core::arch::arm;

    pub(crate) use arm::{
        float32x4_t, float64x2_t, int16x8_t, int32x4_t, int64x2_t, int8x16_t, uint16x8_t,
        uint32x4_t, uint64x2_t, uint8x16_t, vaddq_f32, vaddq_f64, vaddq_s16, vaddq_s32, vaddq_s64,
        vaddq_s8, vaddq_u16, vaddq_u32, vaddq_u64, vaddq_u8, vdivq_f32, vdivq_f64, vdupq_n_f32,
        vdupq_n_f64, vdupq_n_s16, vdupq_n_s32, vdupq_n_s64, vdupq_n_s8, vdupq_n_u16, vdupq_n_u32,
        vdupq_n_u64, vdupq_n_u8, vmulq_f32, vmulq_f64, vreinterpretq_f32_u8, vreinterpretq_f64_u8,
        vreinterpretq_s16_u8, vreinterpretq_s32_u8, vreinterpretq_s64_u8, vreinterpretq_s8_u8,
        vreinterpretq_u16_u8, vreinterpretq_u32_u8, vreinterpretq_u64_u8, vreinterpretq_u8_f32,
        vreinterpretq_u8_f64, vreinterpretq_u8_s16, vreinterpretq_u8_s32, vreinterpretq_u8_s64,
        vreinterpretq_u8_s8, vreinterpretq_u8_u16, vreinterpretq_u8_u32, vreinterpretq_u8_u64,
        vsubq_f32, vsubq_f64, vsubq_s16, vsubq_s32, vsubq_s64, vsubq_s8, vsubq_u16, vsubq_u32,
        vsubq_u64, vsubq_u8,
    };
}

}
