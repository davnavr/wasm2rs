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

mod error;
mod script;

pub use error::Error;

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
    /// Creates a file that will containing the Rust code corresponding to a single [`.wast`] file.
    ///
    /// [`.wast`]: wast::Wast
    fn create_script_file(&self, script: &std::path::Path) -> std::io::Result<PathWriter<'a>>;

    /// Creates a file that will contain [`include!`]s for all generated Rust code corresponding
    /// to each [script file].
    ///
    /// [script file]: Output::create_script_file()
    fn create_root_file(&self) -> std::io::Result<PathWriter<'a>>;
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
pub fn convert_to_output<'input>(
    inputs: &'input [TestFile],
    output: &dyn Output<'_>,
) -> Result<(), Error<'input>> {
    use rayon::prelude::*;

    let conversion_allocations = wasm2rs_convert::Allocations::default();
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

    // This may not handle opening too many files at once correctly. However, the limit on
    // Linux (~1024) and Windows (~512) is far greater than the maximum number of threads typically
    // used by threadpools in `rayon`.
    let results = inputs
        .par_iter()
        .map(|test_file| -> Result<(), _> {
            let mut input_buffer = RentedString {
                pool: &string_pool,
                string: string_pool.pop().unwrap_or_default(),
            };

            let mut input_read = std::fs::File::open(&test_file.input).map_err(|cause| {
                Error::with_path_and_cause(&test_file.input, "could not open input file", cause)
            })?;

            std::io::Read::read_to_string(&mut input_read, &mut input_buffer.string).map_err(
                |cause| Error::with_path_and_cause(&test_file.input, "could not read input", cause),
            )?;

            std::mem::drop(input_read);

            let mut output_write =
                output
                    .create_script_file(&test_file.input)
                    .map_err(|cause| {
                        Error::with_path_and_cause(
                            &test_file.input,
                            "could not create output file",
                            cause,
                        )
                    })?;

            script::convert(
                &mut output_write.file,
                &test_file.options,
                &conversion_allocations,
                &test_file.input,
                &input_buffer.string,
            )
        })
        .collect::<Vec<Result<(), Error<'input>>>>();

    Error::collect(results.into_iter().filter_map(Result::err))
}
