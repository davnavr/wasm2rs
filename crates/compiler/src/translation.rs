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

#[derive(Clone, Copy, Debug)]
pub(crate) enum Error {
    UnknownSection { id: u8, offset: usize },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSection { id, offset } => write!(
                f,
                "encountered unknown section with id {id} at offset {offset:#06X}"
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Default for Translation {
    fn default() -> Self {
        Self::new()
    }
}

impl Translation {
    const SUPPORTED_FEATURES: wasmparser::WasmFeatures = wasmparser::WasmFeatures {
        mutable_global: false,
        saturating_float_to_int: false,
        sign_extension: false,
        reference_types: false,
        multi_value: false,
        bulk_memory: false,
        simd: false,
        relaxed_simd: false,
        threads: false,
        tail_call: false,
        floats: false,
        multi_memory: false,
        exceptions: false,
        memory64: false,
        extended_const: false,
        component_model: false,
        function_references: false,
        memory_control: false,
        gc: false,
        component_model_values: false,
        component_model_nested_names: false,
    };

    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self {}
    }

    /// [`Read`]s a WebAssembly binary module, translates it, and [`Write`]s the resulting Rust
    /// source code.
    ///
    /// Returns the number of bytes read from the `input`.
    ///
    /// [`Read`]: std::io::Read
    /// [`Write`]: std::io::Write
    pub fn compile<I, O>(
        self,
        input: &mut I,
        input_len: Option<usize>,
        mut output: &mut O,
    ) -> crate::Result<usize>
    where
        I: std::io::Read,
        O: std::io::Write,
    {
        let mut validator = wasmparser::Validator::new_with_features(Self::SUPPORTED_FEATURES);
        let mut parse_buffer_offset = 0usize;
        let mut parser = wasmparser::Parser::new(parse_buffer_offset as u64);
        let mut parse_buffer = vec![0u8; input_len.unwrap_or(0x1000)];

        let read_len;
        let validated_types;

        let mut code_section_contents = Vec::new();
        let mut code_section_contents_saved = 0usize..0usize;

        let mut functions_to_validate = Vec::new();
        let mut functions_to_process = Vec::<std::ops::Range<u32>>::new();
        let mut start_func_idx = None;

        loop {
            let eof = input.read(&mut parse_buffer)? < parse_buffer.len();
            match parser.parse(&parse_buffer, eof)? {
                wasmparser::Chunk::NeedMoreData(amount) => {
                    parse_buffer.reserve(amount.try_into().unwrap_or(usize::MAX));
                    continue;
                }
                wasmparser::Chunk::Parsed { consumed, payload } => {
                    use wasmparser::Payload;

                    match payload {
                        Payload::Version {
                            num,
                            encoding,
                            range,
                        } => {
                            validator.version(num, encoding, &range)?;
                        }
                        Payload::TypeSection(section) => {
                            validator.type_section(&section)?;
                        }
                        Payload::ImportSection(section) => {
                            validator.import_section(&section)?;
                            // TODO: Need to collect import info here, process imports later.
                        }
                        Payload::FunctionSection(section) => {
                            validator.function_section(&section)?;
                        }
                        Payload::TableSection(section) => {
                            validator.table_section(&section)?;
                        }
                        Payload::MemorySection(section) => {
                            validator.memory_section(&section)?;
                        }
                        Payload::TagSection(section) => {
                            validator.tag_section(&section)?;
                        }
                        Payload::GlobalSection(section) => {
                            validator.global_section(&section)?;
                        }
                        Payload::ExportSection(section) => {
                            validator.export_section(&section)?;
                            // TODO: Need to collect export info here, process export later.
                        }
                        Payload::StartSection { func, range } => {
                            validator.start_section(func, &range)?;
                            start_func_idx = Some(func);
                        }
                        Payload::ElementSection(section) => {
                            validator.element_section(&section)?;
                        }
                        Payload::DataCountSection { count, range } => {
                            validator.data_count_section(count, &range)?;
                        }
                        Payload::DataSection(section) => {
                            validator.data_section(&section)?;
                        }
                        Payload::CodeSectionStart { count, range, size } => {
                            validator.code_section_start(count, &range)?;
                            let function_count = usize::try_from(count).unwrap_or_default();

                            functions_to_validate.reserve_exact(function_count);
                            functions_to_process.reserve_exact(function_count);

                            let code_section_size = usize::try_from(size).unwrap_or_default();
                            code_section_contents_saved.start = range.start;
                            code_section_contents.reserve_exact(code_section_size);

                            // Copy existing section contents from `parse_buffer`.
                            let remaining_buffer =
                                &parse_buffer[range.start - parse_buffer_offset..];

                            let copied = code_section_size.min(remaining_buffer.len());
                            code_section_contents_saved.end =
                                code_section_contents_saved.start + copied;
                            code_section_contents.extend_from_slice(&remaining_buffer[..copied]);
                        }
                        Payload::CodeSectionEntry(body) => {
                            functions_to_validate.push(validator.code_section_entry(&body)?);

                            let original_range = body.range();

                            // Calculate offsets into the `code_section_contents` buffer where the body is/will be stored.
                            let code_range = if code_section_contents_saved
                                .contains(&original_range.start)
                                && code_section_contents_saved.contains(&original_range.end)
                            {
                                (original_range.start - code_section_contents_saved.start)
                                    ..(original_range.end - code_section_contents_saved.start)
                            } else {
                                let start = code_section_contents.len();

                                // Copy the function body, since it was not already copied to the buffer.
                                code_section_contents.extend_from_slice(
                                    &parse_buffer[original_range.start - parse_buffer_offset..]
                                        [..original_range.len()],
                                );

                                start..code_section_contents.len()
                            };

                            let code_start =
                                u32::try_from(code_range.start).expect("code start overflow");

                            let code_end =
                                u32::try_from(code_range.end).expect("code end overflow");

                            functions_to_process.push(code_start..code_end);
                        }
                        Payload::CustomSection(_) => {
                            // At the moment, `wasm2rs` ignores custom sections.

                            // In the future, the `name` custom section and DWARF debug info sections will be parsed.
                        }
                        Payload::UnknownSection { id, range, .. } => {
                            // Defer to `Validator` to handle unrecognized sections.
                            validator.unknown_section(id, &range)?;
                        }
                        Payload::End(offset) => {
                            read_len = offset;
                            validated_types = validator.end(offset)?;

                            // Free the buffer
                            std::mem::take(&mut parse_buffer);

                            break;
                        }
                        // Component Model Sections, the `Validator` will return an error for these
                        // since `wasm2rs` does not support this feature.
                        Payload::ModuleSection { range, .. } => {
                            validator.module_section(&range)?;
                        }
                        Payload::InstanceSection(section) => {
                            validator.instance_section(&section)?;
                        }
                        Payload::CoreTypeSection(section) => {
                            validator.core_type_section(&section)?;
                        }
                        Payload::ComponentSection { range, .. } => {
                            validator.component_section(&range)?;
                        }
                        Payload::ComponentInstanceSection(section) => {
                            validator.component_instance_section(&section)?
                        }
                        Payload::ComponentAliasSection(section) => {
                            validator.component_alias_section(&section)?
                        }
                        Payload::ComponentTypeSection(section) => {
                            validator.component_type_section(&section)?
                        }
                        Payload::ComponentCanonicalSection(section) => {
                            validator.component_canonical_section(&section)?
                        }
                        Payload::ComponentStartSection { range, .. } => {
                            validator.component_section(&range)?
                        }
                        Payload::ComponentImportSection(section) => {
                            validator.component_import_section(&section)?
                        }
                        Payload::ComponentExportSection(section) => {
                            validator.component_export_section(&section)?
                        }
                    }

                    // Remove the bytes that were read by the parser.
                    parse_buffer.copy_within(consumed.., 0);
                    parse_buffer.truncate(parse_buffer.len() - consumed);
                    parse_buffer_offset += consumed;
                }
            }
        }

        let start_func_idx = start_func_idx;

        Ok(read_len)
    }
}
