//! Provides macros used to conditionally compile code dependent on the `simd-intrinsics` flag.

#[macro_export]
#[doc(hidden)]
macro_rules! cfg_sse2_intrinsics {
    {
        $($item:item)*
    } => {$(
        #[cfg(all(
            feature = "simd-intrinsics",
            any(target_arch = "x86_64", target_arch = "x86"),
            target_feature = "sse2",
        ))]
        $item
    )*};
}

#[macro_export]
#[doc(hidden)]
macro_rules! cfg_neon_intrinsics {
    {
        $($item:item)*
    } => {$(
        #[cfg(all(
            feature = "simd-intrinsics",
            target_arch = "aarch64",
            target_endian = "little",
            target_feature = "neon",
        ))]
        $item
    )*};
}

#[macro_export]
#[doc(hidden)]
macro_rules! cfg_use_intrinsics {
    {
        $($item:item)*
    } => {$(
        #[cfg(all(
            feature = "simd-intrinsics",
            any(
                // sse2_intrinsics
                all(any(target_arch = "x86_64", target_arch = "x86"), target_feature = "sse2"),
                // neon_intrinsics
                all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"),
            ),
        ))]
        $item
    )*};
}

#[macro_export]
#[doc(hidden)]
macro_rules! cfg_no_intrinsics {
    {
        $($item:item)*
    } => {$(
        #[cfg(not(all(
            feature = "simd-intrinsics",
            any(
                // sse2_intrinsics
                all(any(target_arch = "x86_64", target_arch = "x86"), target_feature = "sse2"),
                // neon_intrinsics
                all(target_arch = "aarch64", target_endian = "little", target_feature = "neon"),
            ),
        )))]
        $item
    )*};
}
