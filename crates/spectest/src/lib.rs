//! Generates Rust unit tests from WebAssembly specification test files (`.wast`).

#![deny(unsafe_code)]
#![deny(clippy::cast_possible_truncation)]

pub use anyhow::{Error, Result};
pub use wast;

mod pools;
mod test_case;

use anyhow::Context;

/// Maps an input `.wast` file to an output `.rs` file.
#[derive(Debug)]
pub struct TestFile {
    /// Path to the `.wast` file to read.
    pub input: std::path::PathBuf,
    /// Path the the `.rs` file to generate.
    pub output: std::path::PathBuf,
}

struct Location<'a> {
    line: u32,
    column: u32,
    path: &'a std::path::Path,
}

impl<'a> Location<'a> {
    fn new(path: &'a std::path::Path, span: wast::token::Span, text: &str) -> Self {
        let (line, col) = span.linecol_in(text);
        Self {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            column: u32::try_from(col).unwrap_or(u32::MAX),
            path,
        }
    }
}

impl std::fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.path.display(),
            u64::from(self.line).saturating_add(1),
            u64::from(self.column).saturating_add(1)
        )
    }
}

struct FileContents<'a> {
    path: &'a std::path::Path,
    contents: &'a str,
}

impl<'a> FileContents<'a> {
    fn location(&self, span: wast::token::Span) -> Location<'a> {
        Location::new(self.path, span, self.contents)
    }
}

fn translate_one(file: &TestFile, warnings: &mut Vec<String>, pools: &pools::Pools) -> Result<()> {
    let mut wast_file = std::fs::File::open(&file.input)
        .with_context(|| format!("could not open test file {:?}", file.input))?;

    let file_size = wast_file
        .metadata()
        .ok()
        .map(|metadata| metadata.len())
        .unwrap_or_default();

    let wast_text = {
        let mut buf = pools.strings.take_buffer(file_size as usize);

        std::io::Read::read_to_string(&mut wast_file, &mut buf)
            .with_context(|| format!("could not read test file {:?}", file.input))?;

        std::mem::drop(wast_file);
        buf
    };

    let wast_buf = wast::parser::ParseBuffer::new(&wast_text)
        .with_context(|| format!("could not lex test file {:?}", file.input))?;

    let wast = wast::parser::parse::<wast::Wast>(&wast_buf)
        .with_context(|| format!("could not parse test file {:?}", file.input))?;

    let wast_contents = FileContents {
        path: &file.input,
        contents: &wast_text,
    };
    let mut test_cases = test_case::Builder::new(pools, &wast_contents);

    let mut emit_warning = |span: wast::token::Span, message: &dyn std::fmt::Display| {
        warnings.push(format!("{} : {message}", wast_contents.location(span)))
    };

    for directive in wast.directives {
        use wast::WastDirective;

        match directive {
            WastDirective::Wat(wat) => test_cases.module(wat)?,
            WastDirective::Invoke(invoke) => test_cases.invoke(invoke)?,
            WastDirective::AssertMalformed { .. } | WastDirective::AssertInvalid { .. } => (), // We aren't testing wasmparser
            _ => {
                emit_warning(directive.span(), &"unsupported directive was skipped");
                break;
            }
        }
    }

    //test_cases

    // Done with the WAST text
    pools.strings.return_buffer(wast_text);

    todo!()
}

/// Translates all of the given test files.
///
/// Returns a list of warnings alongside if the translation was sucessful.
pub fn translate(files: &[TestFile]) -> (Vec<String>, anyhow::Result<()>) {
    use rayon::prelude::*;

    let string_pool = Default::default();
    let pools = pools::Pools {
        strings: &string_pool,
    };

    let (warnings, result) = files
        .par_iter()
        .map(|file| {
            let mut local_warnings = Vec::new();
            let result = translate_one(file, &mut local_warnings, &pools);
            (local_warnings, result)
        })
        .collect::<(Vec<Vec<String>>, Result<()>)>();

    (warnings.into_iter().flatten().collect(), result)
}
