//! The [WebAssembly test suite], translated by `wasm2rs`.
//!
//! [WebAssembly test suite]: https://github.com/WebAssembly/testsuite

#![doc(hidden)]

include!(concat!(env!("OUT_DIR"), "/all.rs"));
