//! Provides runtime support functionality for WebAssembly modules translated to Rust source code
//! by `wasm2rs`.
//!
//! # Related Crates
//!
//! The implementation for [`wasm2rs-rt`](crate) is actually split into multiple component crates,
//! which are:
//! - [`wasm2rs_rt_core`], which contains code shared among other component crates. It's modules
//!   are re-exported in the crate root.
//! - [`wasm2rs_rt_func_ref`], which provides the implementation for WebAssembly function
//!   references. It is enabled by the [`func-ref`](crate#func-ref) feature flag, and is
//!   re-exported as the [`func_ref`] module.
//! - [`wasm2rs_rt_math`], which contains functions for performing integer and floating-point
//!   arithmetic. It is re-exported as the [`math`] module.
//! - [`wasm2rs_rt_memory`], which provides the implementation for WebAssembly linear memory. It
//!   is enabled by the [`memory`](crate#memory) feature flag, and is re-exported as the [`memory`]
//!   module.
//! - [`wasm2rs_rt_simd`], which provides an implementation for 128-bit SIMD operations. It is
//!   enabled by the [`feature-simd128`](crate#feature-simd128) feature flag, and is re-exported as
//!   the [`simd`] module.
//! - [`wasm2rs_rt_stack`], which provides functionality for checking whether a stack overflow
//!   occurred. It is re-exported as the [`stack`] module. By default, stack overflow checks are a
//!   no-op, and require the [`stack-overflow-checks`](crate#stack-overflow-checks) feature flag
//!   to be enabled.
//!
//! # Feature Flags
//!
//! By default, the [`std`](crate#std), [`memory`](crate#memory),
//! [`simd-intrinsics`](crate#simd-intrinsics),
//! [`feature-reference-types`](crate#feature-reference-types), and
//! [`feature-simd128`](crate#feature-simd128) flags are enabled.
//!
//! ## [`std`]
//!
//! Enables a dependency on the [Rust standard library](std).
//!
//! - Enables: [`alloc`](crate#alloc).
//! - Enabled by: [`default`](crate#feature-flags)
//!
//! ## [`alloc`]
//!
//! Enables a dependency on the [Rust core allocation library](alloc).
//!
//! - Enabled by: [`std`](crate#std), [`default`](crate#feature-flags)
//!
//! ## `func-ref`
//!
//! Provides runtime support for untyped function reference types, enabling the [`func_ref`]
//! module. Adds a dependency on the [`wasm2rs_rt_func_ref`] crate.
//!
//! - Enabled by: [`feature-reference-types`](crate#feature-reference-types),
//!   [`default`](crate#feature-flags)
//!
//! ## [`memory`]
//!
//! Provides runtime support for linear memory, enabling the [`memory`] module. Adds a dependency
//! on the [`wasm2rs_rt_memory`] crate.
//!
//! - Enabled by: [`default`](crate#feature-flags)
//!
//! ## `simd-intrinsics`
//!
//! When the [`feature-simd128`](crate#feature-simd128) flag is enabled, then target
//! architecture-specific SIMD intrinsics *may* be used as the implementation for SIMD operations.
//! See the documentation on the [`simd`] module for more information.
//!
//! - Enabled by: [`default`](crate#feature-flags)
//!
//! ## `stack-overflow-checks`
//!
//! Sets the [`wasm2rs_rt_stack`] crate's `enabled` feature flag. See the documentation on the
//! [`stack`] module for more information.
//!
//! ## `feature-reference-types`
//!
//! Enables runtime support for the [reference types proposal]. Adds a dependency on the
//! [`wasm2rs_rt_func_ref`] crate.
//!
//! - Enables [`func-ref`](crate#func-ref)
//! - Enabled by: [`default`](crate#feature-flags)
//!
//! ## `feature-simd128`
//!
//! Provides runtime support for 128-bit SIMD operations added in the [fixed-width SIMD proposal].
//! Adds a dependency on the [`wasm2rs_rt_simd`] crate. To allow the usage of SIMD intrinsics for
//! the target architecture, see the [`simd-intrinsics`](crate#simd-intrinsics) flag.
//!
//! - Enabled by: [`default`](crate#feature-flags)
//!
//! [reference types proposal]: https://github.com/WebAssembly/reference-types
//! [fixed-width SIMD proposal]: https://github.com/webassembly/simd
//! [`wasm2rs_rt_func_ref`]: rt_func_ref
//! [`wasm2rs_rt_math`]: rt_math
//! [`wasm2rs_rt_memory`]: rt_memory
//! [`wasm2rs_rt_simd`]: rt_simd
//! [`wasm2rs_rt_stack`]: rt_stack

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)] // Unsafe code present in dependencies
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_alloc)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub use rt_math as math;
pub use wasm2rs_rt_core::{global, symbol, trace, trap};

#[cfg(feature = "memory")]
pub use rt_memory as memory;

#[cfg(feature = "feature-simd128")]
pub use rt_simd as simd;

#[cfg(feature = "func-ref")]
pub use rt_func_ref as func_ref;

pub use rt_stack as stack;
