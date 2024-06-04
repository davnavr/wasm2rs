//! Implements WebAssembly SIMD operations for `wasm2rs`.
//!
//! Currently, only [fixed width 128-bit SIMD] is supported, provided in the [`v128`] module.
//!
//! # Utilizing SIMD Intrinsics
//!
//! If the `simd-intrinsic` feature is enabled, then [architecture-specific SIMD intrinsics] are
//! used to implement vector operations rather than relying on the Rust compiler's
//! auto-vectorization. SIMD Intrinsics are used on the following target architectures when the
//! corresponding [target features] are enabled:
//!
//! - `x86` and `x86-64`: requires `sse2`
//!   - Note that common targets such as `x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`
//!     already enable the `sse2` target feature by default.
//!
//! The `simd-intrinsic` feature flag is provided to allow testing the fallback implementation of
//! SIMD operations which doesn't use SIMD intrinsics.
//!
//! [fixed width 128-bit SIMD]: https://github.com/webassembly/simd
//! [architecture-specific SIMD intrinsics]: core::arch
//! [target features]: https://doc.rust-lang.org/reference/attributes/codegen.html#available-features

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

mod cfg;
mod intrinsics;

pub mod v128;
