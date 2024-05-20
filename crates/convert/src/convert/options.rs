/// Function that writes a data segment to some file, returning a path to it.
///
/// This function is passed the index of the data segment and its contents. An implementation
/// is expected to write the contents to a new file, and return a path to it such that the
/// generated code may use [`include_bytes!`].
///
/// To specify a [`DataSegmentWriter`] to use, call [`Convert::data_segment_writer()`].
///
/// # Errors
///
/// - `Ok(None)` is returned if a file could not be created. In this case, the data segment
///   contents are included as a byte string literal.
/// - `Err` is returned if a file could not be created.
///
/// [`Convert::data_segment_writer()`]: crate::Convert::data_segment_writer()
pub type DataSegmentWriter<'a> =
    &'a (dyn Fn(u32, &[u8]) -> std::io::Result<Option<String>> + Send + Sync);

/// Used to specify what debug information is included in the generated Rust code.
///
/// See the documentation for [`Convert::debug_info()`] for more information.
///
/// [`Convert::debug_info()`]: crate::Convert::debug_info()
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DebugInfo {
    /// All debug information is removed.
    Omit,
    /// Only bytecode offsets, function parameter and result types, and function names are
    /// included.
    ///
    /// Function names are taken from:
    /// - The [*import* section].
    /// - The [*export* section].
    /// - If available, the [`name` custom section].
    /// - If available, DWARF debug data (**not yet implemented**).
    ///
    /// [*import* section]: https://webassembly.github.io/spec/core/syntax/modules.html#imports
    /// [*export* section]: https://webassembly.github.io/spec/core/syntax/modules.html#exports
    /// [`name` custom section]: https://webassembly.github.io/spec/core/appendix/custom.html#name-section
    SymbolsOnly,
    /// Includes everything from [`SymbolsOnly`], in addition to file names and line number
    /// information taken from DWARF debug data if it is available (**not yet implemented**).
    ///
    /// [`SymbolsOnly`]: DebugInfo::SymbolsOnly
    LineTablesOnly,
    /// Includes everything from [`LineTablesOnly`], in addition to variable names taken from
    /// the [`name` custom section] and DWARF debug data (**not yet implemented**) if available.
    ///
    /// [`LineTablesOnly`]: DebugInfo::LineTablesOnly
    /// [`name` custom section]: https://webassembly.github.io/spec/core/appendix/custom.html#name-section
    #[default]
    Full,
}

/// Used to specify if stack overflow checks should be generated.
///
/// See the documentation for [`Convert::stack_overflow_checks()`] for more information.
///
/// [`Convert::stack_overflow_checks()`]: crate::Convert::stack_overflow_checks()
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum StackOverflowChecks {
    /// No stack overflow checks are generated.
    #[default]
    Omit,
    /// Stack overflow checks are generated only for direct calls to functions defined in the same
    /// module. Checks are not generated for indirect calls (such as those made via [`funcref`]s
    /// and the [`call_indirect`] instruction) or [imported functions].
    ///
    /// [`funcref`]: https://webassembly.github.io/spec/core/syntax/types.html#syntax-reftype
    /// [`call_indirect`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-control
    /// [imported functions]: https://webassembly.github.io/spec/core/syntax/modules.html#syntax-importdesc
    LocalOnly,
    //Full,
}

/// Methods for specifying code generation options.
impl<'a> crate::Convert<'a> {
    /// Sets the [`Indentation`] used in the generated Rust source code.
    ///
    /// [`Indentation`]: crate::Indentation
    pub fn indentation(&mut self, indentation: crate::Indentation) -> &mut Self {
        self.indentation = indentation;
        self
    }

    /// Sets the function used to write data segment contents to disk.
    ///
    /// For more information, see the documentation for [`DataSegmentWriter`].
    pub fn data_segment_writer(&mut self, writer: DataSegmentWriter<'a>) -> &mut Self {
        self.data_segment_writer = writer;
        self
    }

    /// Allows enabling or disabling the emission of stack overflow detection code. Defaults to
    /// [`StackOverflowChecks::Omit`].
    ///
    /// Stack overflow detection code may be unreliable, and can only provide conservative
    /// estimates for the remaining amount of space on the stack. It also introduces overhead for
    /// each function call, potentially involving thread local variable accesses and other function
    /// calls.
    pub fn stack_overflow_checks(&mut self, setting: StackOverflowChecks) -> &mut Self {
        self.stack_overflow_checks = setting;
        self
    }

    /// Allows specifying what debug information is included in the generated Rust code.
    ///
    /// See the documentation for [`DebugInfo`] for more information.
    ///
    /// Currently, debug information is only used in building stack traces for WebAssembly
    /// instructions that can [trap], though more uses may be available in the future.
    ///
    /// [trap]: https://webassembly.github.io/spec/core/intro/overview.html#trap
    pub fn debug_info(&mut self, level: DebugInfo) -> &mut Self {
        self.debug_info = level;
        self
    }
}
