//! The [WebAssembly test suite], translated by `wasm2rs`.
//!
//! [WebAssembly test suite]: https://github.com/WebAssembly/testsuite

#[path = "converted/include.wasm2.rs"]
mod tests; // Tests must be generated before they are compiled.
