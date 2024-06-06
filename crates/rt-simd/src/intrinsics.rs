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
        _mm_castsi128_ps, _mm_div_pd, _mm_div_ps, _mm_mul_pd, _mm_mul_ps, _mm_set1_epi16,
        _mm_set1_epi32, _mm_set1_epi64x, _mm_set1_epi8, _mm_set1_pd, _mm_set1_ps, _mm_setr_epi8,
        _mm_setzero_pd, _mm_setzero_ps, _mm_setzero_si128, _mm_store_pd, _mm_store_ps,
        _mm_store_si128, _mm_sub_epi16, _mm_sub_epi32, _mm_sub_epi64, _mm_sub_epi8, _mm_sub_pd,
        _mm_sub_ps,
    };
}

}

// #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
// pub(crate) use core::arch::aarch64 as neon;

//use core::arch::aarch64 as arm;
