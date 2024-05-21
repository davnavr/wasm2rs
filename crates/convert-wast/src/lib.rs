//! Generates Rust unit tests from WebAssembly specification test files ([`.wast`]) using
//! [`wasm2rs`].
//!
//! [`.wast`]: https://github.com/WebAssembly/spec/blob/wg-2.0.draft1/interpreter/README.md
//! [`wasm2rs`]: wasm2rs_convert

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![deny(unreachable_pub)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::cast_possible_truncation)]

use anyhow::Context;

mod script;

type CowPath<'a> = std::borrow::Cow<'a, std::path::Path>;

/// Paris an [`std::io::Write`] implementation with a [`Path`].
///
/// [`Path`]: std::path::Path
#[derive(Debug)]
pub struct PathWriter<'a> {
    path: CowPath<'a>,
    file: std::fs::File,
}

impl<'a> PathWriter<'a> {
    /// Invokes [`File::create()`] with the given path.
    ///
    /// [`File::create()`]: std::fs::File::create()
    pub fn create_file<P: Into<CowPath<'a>>>(path: P) -> std::io::Result<Self> {
        let path = path.into();
        Ok(Self {
            file: std::fs::File::create(&path)?,
            path,
        })
    }

    /// Gets the path.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

/// Trait for creating [`File`]s for a set of WebAssembly script files.
///
/// [`File`]: std::fs::File
pub trait Output<'a>: Sync {
    /// Returns a path to the [`wasm2rs_convert`] output for a WebAssembly module originating from
    /// the given `script` file.
    fn create_module_file(
        &self,
        script: &std::path::Path,
        sequence: u32,
        id: Option<&wast::token::Id>,
        binary: Vec<u8>,
    ) -> anyhow::Result<crate::CowPath<'a>>;

    /// Creates a file that will containing the Rust code corresponding to a single [`.wast`] file.
    ///
    /// [`.wast`]: wast::Wast
    fn create_script_file(&self, script: &std::path::Path) -> anyhow::Result<PathWriter<'a>>;

    /// Creates a file that will contain [`include!`]s for all generated Rust code corresponding
    /// to each [script file].
    ///
    /// [script file]: Output::create_script_file()
    fn create_root_file(&self) -> anyhow::Result<PathWriter<'a>>;
}

//struct DirectoryOutput

/// Maps an input [`.wast`] file to an output `.rs` file.
///
/// Used with the [`convert_to_output()`] function.
///
/// [`.wast`]: wast::Wast
#[derive(Debug)]
pub struct TestFile<'cfg> {
    /// Path to the [`.wast`] file to read.
    ///
    /// [`.wast`]: wast::Wast
    pub input: CowPath<'cfg>,
    /// Specifies the options to use when converting any valid WebAssembly modules
    /// in the [`.wast`] file.
    ///
    /// [`.wast`]: wast::Wast
    pub options: &'cfg wasm2rs_convert::Convert<'static>,
}

/// Converts all of the given [`TestFile`]s, writing the resulting Rust source code to the given
/// [`Output`].
pub fn convert_to_output<'input, 'output>(
    inputs: &'input [TestFile],
    output: &(dyn Output<'output> + '_),
) -> Result<Vec<CowPath<'output>>, Vec<anyhow::Error>> {
    use rayon::prelude::*;

    let string_pool = crossbeam_queue::ArrayQueue::<String>::new(rayon::current_num_threads());

    struct RentedString<'pool> {
        pool: &'pool crossbeam_queue::ArrayQueue<String>,
        string: String,
    }

    impl Drop for RentedString<'_> {
        fn drop(&mut self) {
            self.string.clear();
            self.pool.force_push(std::mem::take(&mut self.string));
        }
    }

    let mut results = Vec::with_capacity(inputs.len());

    // This may not handle opening too many files at once correctly. However, the limit on
    // Linux (~1024) and Windows (~512) is far greater than the maximum number of threads typically
    // used by threadpools in `rayon`.
    inputs
        .par_iter()
        .map(|test_file| -> anyhow::Result<CowPath<'output>> {
            let mut input_buffer = RentedString {
                pool: &string_pool,
                string: string_pool.pop().unwrap_or_default(),
            };

            let mut input_read = std::fs::File::open(&test_file.input)
                .with_context(|| format!("could not open input file {:?}", test_file.input))?;

            std::io::Read::read_to_string(&mut input_read, &mut input_buffer.string)
                .with_context(|| format!("could not open script file {:?}", test_file.input))?;

            std::mem::drop(input_read);

            let mut output_write =
                output
                    .create_script_file(&test_file.input)
                    .with_context(|| {
                        format!("could not create output file for {:?}", test_file.input)
                    })?;

            script::convert(
                &mut output_write.file,
                output,
                &test_file.input,
                &input_buffer.string,
            )
            .with_context(|| format!("could not translate script file {:?}", test_file.input))?;

            Ok(output_write.path)
        })
        .collect_into_vec(&mut results);

    let mut errors = Vec::new();
    let mut successes = Vec::with_capacity(results.len());
    for result in results.into_iter() {
        match result {
            Ok(success) if errors.is_empty() => successes.push(success),
            Ok(_) => (), // Errors detected
            Err(err) => errors.push(err),
        }
    }

    if errors.is_empty() {
        Ok(successes)
    } else {
        Err(errors)
    }
}
