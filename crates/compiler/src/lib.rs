//! Core compiler library for `wasm2rs`.

#![cfg_attr(doc_cfg, feature(doc_auto_cfg))]
#![deny(unsafe_code)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

mod error;
mod func_validator_allocation_pool;
mod translation;

pub mod buffer;
pub mod rust;

pub use anyhow::{Error, Result};
pub use func_validator_allocation_pool::FuncValidatorAllocationPool;
pub use translation::{DataSegmentWriter, Translation};
