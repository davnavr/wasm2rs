//! Provides the foundation for runtime support functionality for WebAssembly modules translated to
//! Rust source code by `wasm2rs`.
//!
//! The `wasm2rs-rt-*` crates each provide runtime support for different aspects of WebAssembly.

#![no_std]
#![cfg_attr(doc_cfg, feature(doc_auto_cfg))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(unsafe_code)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

pub mod symbol;
pub mod trace;
pub mod trap;
