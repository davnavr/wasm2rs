/// Provides options for compiling a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
#[derive(Debug)]
pub struct Compilation {
    // TODO: Store compilation options here.
    //thread_pool: Option<rayon::ThreadPool>,
}

impl Default for Compilation {
    fn default() -> Self {
        Self::new()
    }
}

impl Compilation {
    /// Gets the default compilation options.
    pub fn new() -> Self {
        Self {}
    }

    /// [`Read`]s a WebAssembly binary module, translates it, and [`Write`]s the resulting Rust
    /// source code.
    ///
    /// [`Read`]: std::io::Read
    /// [`Write`]: std::io::Write
    pub fn compile<I, O>(self, input: I, output: O)
    // -> Result<(), CompileError>
    where
        I: std::io::Read,
        O: std::io::Write,
    {
        let _ = (input, output);
        todo!()
    }
}
