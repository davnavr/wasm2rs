//! Core compiler library for `wasm2rs`.

#![cfg_attr(doc_cfg, feature(doc_auto_cfg))]
#![deny(unsafe_code)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

mod translation;

pub mod rust;

pub use translation::Translation;
