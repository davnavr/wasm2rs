//! Generates Rust unit tests from WebAssembly specification test files (`.wast`) using
//! [`wasm2rs`].
//!
//! [`wasm2rs`]: wasm2rs_convert

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![deny(unreachable_pub)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::cast_possible_truncation)]

#[doc(no_inline)]
pub use anyhow::{Error, Result};

/// Maps an input `.wast` file to an output `.rs` file.
#[derive(Debug)]
pub struct TestFile<'cfg> {
    /// Path to the `.wast` file to read.
    pub input: std::path::PathBuf,
    pub options: &'cfg wasm2rs_convert::Convert<'static>,
}

pub struct Conversion {
    // /// Directory that all [`TestFile.input`]'s are relative to.
    // base_input_directory: std::path::PathBuf,
    output_directory: std::path::PathBuf,
}

impl Conversion {
    /// with the directory that will contain all of the generated Rust source code.
    pub fn new(output_directory: std::path::PathBuf) -> Self {
        Self {
            output_directory,
        }
    }
}

pub fn convert_all(
    output_directory: &std::path::Path,
    test_files: &[TestFile],
) -> Result<()> {

}
