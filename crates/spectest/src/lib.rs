//! Generates Rust unit tests from WebAssembly specification test files (`.wast`).

#![deny(unsafe_code)]
#![deny(clippy::cast_possible_truncation)]

pub use anyhow::{Error, Result};
pub use wast;

mod location;
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

    let wast_contents = location::Contents::new(&wast_text, &file.input);
    let mut test_cases = test_case::Builder::new(&wast_contents);

    let mut emit_warning = |span: wast::token::Span, message: &dyn std::fmt::Display| {
        warnings.push(format!("{} : {message}", wast_contents.location(span)))
    };

    let mut skip_module = false;
    for directive in wast.directives {
        use wast::{WastDirective, WastExecute};

        match directive {
            WastDirective::Wat(wat) => {
                skip_module = false;
                test_cases.module(wat)?;
            }
            WastDirective::Invoke(invoke) => {
                if !skip_module {
                    test_cases.invoke(invoke)?
                }
            }
            WastDirective::AssertTrap {
                span,
                exec,
                message,
            } => match exec {
                _ if skip_module => (),
                WastExecute::Invoke(invoke) => test_cases
                    .assert_trap_invoke(invoke, message)
                    .with_context(|| format!("in {}", wast_contents.location(span)))?,
                WastExecute::Get { .. } => anyhow::bail!(
                    "{} : assert_trap with globals is not yet supported",
                    wast_contents.location(span)
                ),
                WastExecute::Wat(_) => emit_warning(span, &"assertion of WAT is not supported"),
            },
            WastDirective::AssertReturn {
                span,
                exec,
                results,
            } => match exec {
                _ if skip_module => (),
                WastExecute::Invoke(invoke) => test_cases
                    .assert_return_invoke(invoke, results)
                    .with_context(|| format!("in {}", wast_contents.location(span)))?,
                WastExecute::Get { .. } => anyhow::bail!(
                    "{} : assert_return with globals is not yet supported",
                    wast_contents.location(span)
                ),
                WastExecute::Wat(_) => emit_warning(span, &"assertion of WAT is not supported"),
            },
            WastDirective::AssertMalformed { .. } | WastDirective::AssertInvalid { .. } => {
                // We aren't testing wasmparser
                skip_module = true;
            }
            _ => {
                emit_warning(directive.span(), &"unsupported directive was skipped");
                break;
            }
        }
    }

    let (test_modules, to_translate) = test_cases.finish();

    let invoke_wasm2rs = || -> crate::Result<()> {
        todo!("write and get the module name too so a zip is needed");
    };

    let (unit_tests, translation) = rayon::join(
        || test_case::write_unit_tests(test_modules, &pools.buffers),
        invoke_wasm2rs,
    );

    // Done with the WAST text
    pools.strings.return_buffer(wast_text);

    todo!()
}

/// Translates all of the given test files.
///
/// Returns a list of warnings alongside if the translation was sucessful.
pub fn translate(files: &[TestFile]) -> (Vec<String>, anyhow::Result<()>) {
    use rayon::prelude::*;

    let string_pool = crate::pools::StringPool::default();
    let buffer_pool = wasm2rs::buffer::Pool::default();
    let pools = pools::Pools {
        strings: &string_pool,
        buffers: &buffer_pool,
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
