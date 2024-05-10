fn main() {
    use cfg_aliases::cfg_aliases;

    println!("cargo::rustc-check-cfg=cfg(simd_intrinsics)");
    println!("cargo::rustc-check-cfg=cfg(simd_sse2_intrinsics)");
    println!("cargo::rustc-check-cfg=cfg(simd_no_intrinsics)");

    cfg_aliases! {
        simd_intrinsics: { feature = "simd-intrinsics" }, // not(miri)
        simd_sse2_intrinsics: { all(simd_intrinsics, target_feature = "sse2") },
        simd_no_intrinsics: { not(any(simd_sse2_intrinsics)) },
    }
}
