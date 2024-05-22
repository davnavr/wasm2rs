//! Runtime support for executing the [WebAssembly specification tests] under `wasm2rs`.
//!
//! [WebAssembly specification tests]: https://github.com/WebAssembly/testsuite

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)] // Unsafe code present in dependencies
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

mod host_ref;
mod imports;

pub use host_ref::HostRef;
pub use imports::SpecTestImports;
