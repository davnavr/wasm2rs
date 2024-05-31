//! The [WebAssembly test suite], translated by `wasm2rs`.
//!
//! [WebAssembly test suite]: https://github.com/WebAssembly/testsuite

#[path = "converted/include.wasm2.rs"]
#[rustfmt::skip]
mod tests; // Tests must be generated before they are compiled.
