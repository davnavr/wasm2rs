//! Core conversion library for `wasm2rs`, responsible for converting [WebAssembly binary modules]
//! into [Rust source code].
//!
//! [WebAssembly binary modules]: https://webassembly.github.io/spec/core/binary/modules.html#binary-module
//! [Rust source code]: https://doc.rust-lang.org/reference/items.html

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![deny(unreachable_pub)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(clippy::exhaustive_enums)]

pub mod buffer;
pub mod ident;

mod ast;
mod context;
mod convert;
mod pool;
mod write;

#[doc(no_inline)]
pub use anyhow::{Error, Result};

pub use ast::Indentation;
pub use convert::{Allocations, Convert, DataSegmentWriter, DebugInfo, StackOverflowChecks};
