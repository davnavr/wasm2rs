//! Re-exports target architecture-specific intrinsics.
//!
//! These are only provided on supported platforms when the `simd-intrinsics` feature flag is
//! enabled.

#[cfg(all(target_arch = "x86", target_feature = "sse2"))]
pub(in crate::simd) use core::arch::x86::*;

#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
pub(in crate::simd) use core::arch::x86_64::*;
