//! Provides the foundation for runtime support functionality for WebAssembly modules translated to
//! Rust source code by `wasm2rs`.
//!
//! The `wasm2rs-rt-*` crates each provide runtime support for different aspects of WebAssembly.

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![forbid(unsafe_code)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

pub mod global;
pub mod limit;
pub mod symbol;
pub mod table;
pub mod thread;
pub mod trace;
pub mod trap;

/// Error type used when an linear memory address or table index was out of bounds.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[allow(clippy::exhaustive_structs)]
pub struct BoundsCheckError;

impl core::fmt::Display for BoundsCheckError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("out-of-bounds index")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BoundsCheckError {}

/// Result type used for functions that need to indicate if an address or index is out of bounds.
pub type BoundsCheck<T> = core::result::Result<T, BoundsCheckError>;
