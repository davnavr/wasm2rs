//! Provides functionality for supporting WebAssembly modules translated to Rust source code by
//! `wasm2rs`.

#![no_std]
#![cfg_attr(doc_cfg, feature(doc_auto_cfg))]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_alloc)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod embedder;
pub mod func_ref;
pub mod global;
pub mod math;
pub mod memory;
pub mod trap;
