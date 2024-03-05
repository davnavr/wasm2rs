/// Provides options for translating a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
#[derive(Debug)]
pub struct Translation {
    //buffers: dyn Fn() -> Vec<u8>,
    //thread_pool: Option<rayon::ThreadPool>,
    //runtime_crate_path: CratePath,
}

impl Default for Translation {
    fn default() -> Self {
        Self::new()
    }
}

impl Translation {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self {}
    }

    /// Parses the WebAssembly binary module stored in the given buffer, translates it, and
    /// [`Write`]s the resulting Rust source code.
    ///
    /// [`Write`]: std::io::Write
    pub fn compile_binary<O>(self, input: &[u8], output: O)
    // -> Result<(), CompileError>
    where
        O: std::io::Write,
    {
        let _ = (input, output);
        todo!()
    }
}
