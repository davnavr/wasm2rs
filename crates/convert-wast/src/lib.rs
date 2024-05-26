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
        conversion_options: &wasm2rs_convert::Convert,
        conversion_allocations: &wasm2rs_convert::Allocations,
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

/// An [`Output`] implementation that writes all files into a given directory.
#[derive(Debug)]
pub struct DirectoryOutput<'a> {
    root_directory: &'a std::path::Path,
}

impl<'a> DirectoryOutput<'a> {
    /// Creates files within an existing directory.
    pub fn existing(root_directory: &'a std::path::Path) -> Self {
        Self { root_directory }
    }
}

impl<'a> Output<'a> for DirectoryOutput<'_> {
    fn create_module_file(
        &self,
        script: &std::path::Path,
        sequence: u32,
        id: Option<&wast::token::Id>,
        binary: Vec<u8>,
        conversion_options: &wasm2rs_convert::Convert,
        conversion_allocations: &wasm2rs_convert::Allocations,
    ) -> anyhow::Result<crate::CowPath<'a>> {
        let mut path = std::path::PathBuf::from(self.root_directory);
        let stem = script
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("missing stem for {script:?}"))?;
        path.push(stem);

        match std::fs::create_dir(&path) {
            Err(err) if err.kind() != std::io::ErrorKind::AlreadyExists => {
                return Err(anyhow::Error::new(err))
            }
            _ => (),
        }

        if let Some(name) = id {
            path.push(name.name());
        } else {
            path.push(format!("module_{sequence}"));
        }

        path.set_extension("wasm2.rs");

        {
            let mut convert_output =
                std::fs::File::create(&path).context("unable to create output file")?;

            conversion_options
                .convert_from_buffer_with_allocations(
                    &binary,
                    &mut convert_output,
                    conversion_allocations,
                )
                .context("conversion failed")?;
        }

        Ok(std::path::PathBuf::from(path.strip_prefix(self.root_directory).unwrap()).into())
    }

    fn create_script_file(&self, script: &std::path::Path) -> anyhow::Result<PathWriter<'a>> {
        let mut path = std::path::PathBuf::from(self.root_directory);

        if let Some(stem) = script.file_stem() {
            path.push(stem);
        }

        path.set_extension("wast.rs");
        PathWriter::create_file(path).map_err(Into::into)
    }

    fn create_root_file(&self) -> anyhow::Result<PathWriter<'a>> {
        PathWriter::create_file(self.root_directory.join("include.wasm2.rs")).map_err(Into::into)
    }
}

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
    #[cfg(feature = "rayon")]
    let string_pool = crossbeam_queue::ArrayQueue::<String>::new(rayon::current_num_threads());

    #[cfg(feature = "rayon")]
    struct RentedString<'pool> {
        pool: &'pool crossbeam_queue::ArrayQueue<String>,
        string: String,
    }

    #[cfg(feature = "rayon")]
    impl Drop for RentedString<'_> {
        fn drop(&mut self) {
            self.string.clear();
            self.pool.force_push(std::mem::take(&mut self.string));
        }
    }

    #[cfg(not(feature = "rayon"))]
    let string_pool = std::cell::Cell::new(String::new());

    #[cfg(not(feature = "rayon"))]
    struct RentedString<'pool> {
        pool: &'pool std::cell::Cell<String>,
        string: String,
    }

    #[cfg(not(feature = "rayon"))]
    impl Drop for RentedString<'_> {
        fn drop(&mut self) {
            self.pool.replace(std::mem::take(&mut self.string));
        }
    }

    let conversion_allocations = wasm2rs_convert::Allocations::default();
    let mut results = Vec::<anyhow::Result<_>>::with_capacity(inputs.len());

    let translate_script = |test_file: &'input TestFile| -> anyhow::Result<CowPath<'output>> {
        let mut input_buffer = RentedString {
            pool: &string_pool,
            #[cfg(feature = "rayon")]
            string: string_pool.pop().unwrap_or_default(),
            #[cfg(not(feature = "rayon"))]
            string: string_pool.take(),
        };

        let mut input_read = std::fs::File::open(&test_file.input)
            .with_context(|| format!("could not open input file {:?}", test_file.input))?;

        std::io::Read::read_to_string(&mut input_read, &mut input_buffer.string)
            .with_context(|| format!("could not open script file {:?}", test_file.input))?;

        std::mem::drop(input_read);

        let mut output_write = output
            .create_script_file(&test_file.input)
            .with_context(|| format!("could not create output file for {:?}", test_file.input))?;

        script::convert(
            &mut output_write.file,
            output,
            &test_file.input,
            &input_buffer.string,
            test_file.options,
            &conversion_allocations,
        )
        .with_context(|| format!("could not translate script file {:?}", test_file.input))?;

        Ok(output_write.path)
    };

    #[cfg(not(feature = "rayon"))]
    {
        results.extend(inputs.iter().map(translate_script));
    }

    // This may not handle opening too many files at once correctly. However, the limit on
    // Linux (~1024) and Windows (~512) is far greater than the maximum number of threads typically
    // used by threadpools in `rayon`.
    #[cfg(feature = "rayon")]
    {
        use rayon::prelude::*;

        inputs
            .par_iter()
            .map(translate_script)
            .collect_into_vec(&mut results);
    }

    let mut errors = Vec::new();
    let mut successes = Vec::with_capacity(results.len());
    for result in results.into_iter() {
        match result {
            Ok(success) if errors.is_empty() => successes.push(success),
            Ok(_) => (), // Errors detected
            Err(err) => errors.push(err),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut root_file = output
        .create_root_file()
        .context("could not create root file")
        .map_err(|err| vec![err])?;

    {
        use wasm2rs_convert::write::Write as _;

        let mut out = wasm2rs_convert::write::IoWrite::new(&mut root_file.file);

        writeln!(out, "// Translated {} tests\n", successes.len());

        for (i, path) in successes.iter().enumerate() {
            // Would relative paths work better here?
            let module_path = path
                .canonicalize()
                .with_context(|| format!("could not canonicalize {path:?}"))
                .map_err(|err| vec![err])?;

            writeln!(out, "#[path = {module_path:?}]");
            let input_path = inputs[i].input.as_ref();
            if let Some(stem) = input_path.file_stem() {
                let module_name = stem.to_string_lossy();
                writeln!(
                    out,
                    "#[allow(non_snake_case)]\nmod {};",
                    wasm2rs_convert::ident::SafeIdent::from(module_name.as_ref())
                );
            } else {
                writeln!(out, "mod test_{i}; // Translated from {input_path:?}",);
            }
        }

        out.flush();
        out.into_inner()
            .with_context(|| format!("I/O error while writing {:?}", root_file.path.as_ref()))
            .map_err(|err| vec![err])?;
    }

    Ok(successes)
}
