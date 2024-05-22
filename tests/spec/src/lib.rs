//! The [WebAssembly test suite], translated by `wasm2rs`.
//!
//! [WebAssembly test suite]: https://github.com/WebAssembly/testsuite

#![doc(hidden)]

pub mod nan;

#[path = "generated/include.wasm2.rs"]
mod tests;
