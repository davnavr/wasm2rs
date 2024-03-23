//! Generates Rust unit tests from WebAssembly specification test files (`.wast`).

#![deny(unsafe_code)]
#![deny(clippy::cast_possible_truncation)]

pub use anyhow::{Error, Result};
pub use wast;

/// Maps an input `.wast` file to an output `.rs` file.
#[derive(Debug)]
pub struct TestFile {
    /// Path to the `.wast` file to read.
    pub input: std::path::PathBuf,
    /// Path the the `.rs` file to generate.
    pub output: std::path::PathBuf,
}

fn translate_impl(files: &[TestFile], warnings: &mut Vec<String>) -> Result<()> {
    todo!()
}

/// Translates all of the given test files.
///
/// Returns a list of warnings alongside if the translation was sucessful.
pub fn translate(files: &[TestFile]) -> (Vec<String>, anyhow::Result<()>) {
    let mut warnings = Vec::new();
    let result = translate_impl(files, &mut warnings);
    (warnings, result)
}
