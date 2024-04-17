//! Contains the core code for converting WebAssembly to Rust.

mod allocations;
mod code;
mod options;

pub use allocations::Allocations;
pub use options::{DataSegmentWriter, DebugInfo, StackOverflowChecks};

/// Provides options for converting a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
pub struct Convert<'a> {
    indentation: crate::Indentation,
    generated_macro_name: crate::ident::SafeIdent<'a>,
    data_segment_writer: DataSegmentWriter<'a>,
    // wasm_features: &'a wasmparser::WasmFeatures,
    stack_overflow_checks: StackOverflowChecks,
    debug_info: DebugInfo,
    allocations: Option<&'a Allocations>,
}

impl std::fmt::Debug for Convert<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Convert")
            .field("indentation", &self.indentation)
            .field("generated_macro_name", &self.generated_macro_name)
            .field("stack_overflow_checks", &self.stack_overflow_checks)
            .field("debug_info", &self.debug_info)
            .finish_non_exhaustive()
    }
}

impl Default for Convert<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Convert<'_> {
    /// Gets the default options.
    pub fn new() -> Self {
        Self {
            indentation: Default::default(),
            generated_macro_name: crate::ident::Ident::DEFAULT_MACRO_NAME.into(),
            data_segment_writer: &|_, _| Ok(None),
            // wasm_features: &Self::DEFAULT_SUPPORTED_FEATURES,
            stack_overflow_checks: Default::default(),
            debug_info: Default::default(),
            allocations: None,
        }
    }
}

struct Module<'a> {
    imports: Option<wasmparser::ImportSectionReader<'a>>,
    tables: Option<wasmparser::TableSectionReader<'a>>,
    memories: Option<wasmparser::MemorySectionReader<'a>>,
    //tags: Option<wasmparser::TagSectionReader<'a>>,
    globals: Option<wasmparser::GlobalSectionReader<'a>>,
    exports: Option<wasmparser::ExportSectionReader<'a>>,
    start_function: Option<u32>,
    elements: Option<wasmparser::ElementSectionReader<'a>>,
    data: Option<wasmparser::DataSectionReader<'a>>,
    types: wasmparser::types::Types,
}

impl<'wasm> Module<'wasm> {
    fn resolve_block_type(
        &self,
        block_type: wasmparser::BlockType,
    ) -> std::borrow::Cow<'_, wasmparser::FuncType> {
        use std::borrow::Cow;
        use wasmparser::{BlockType, FuncType};

        match block_type {
            BlockType::Empty => Cow::Owned(FuncType::new([], [])),
            BlockType::Type(result) => Cow::Owned(FuncType::new([], [result])),
            BlockType::FuncType(type_idx) => Cow::Borrowed(
                self.types[self.types.core_type_at(type_idx).unwrap_sub()].unwrap_func(),
            ),
        }
    }
}

fn validate_payloads<'a>(wasm: &'a [u8]) -> crate::Result<(Module<'a>, Vec<code::Code<'a>>)> {
    /// The set of WebAssembly features that are supported by default.
    const SUPPORTED_FEATURES: wasmparser::WasmFeatures = wasmparser::WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: false,
        multi_value: true,
        bulk_memory: true,
        simd: false,
        relaxed_simd: false,
        threads: false,
        tail_call: false,
        floats: true,
        multi_memory: true,
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

    let mut validator = wasmparser::Validator::new_with_features(SUPPORTED_FEATURES);
    let mut imports = None;
    let mut tables = None;
    let mut memories = None;
    //let mut tags = None;
    let mut globals = None;
    let mut exports = None;
    let mut start_function = None;
    let mut elements = None;
    let mut data = None;
    let mut function_bodies = Vec::new();

    for result in wasmparser::Parser::new(0).parse_all(wasm) {
        use wasmparser::Payload;

        match result? {
            Payload::Version {
                num,
                encoding,
                range,
            } => {
                validator.version(num, encoding, &range)?;
            }
            Payload::TypeSection(types) => {
                validator.type_section(&types)?;
            }
            Payload::ImportSection(section) => {
                validator.import_section(&section)?;
                imports = Some(section);
            }
            Payload::FunctionSection(section) => {
                validator.function_section(&section)?;
            }
            Payload::TableSection(section) => {
                validator.table_section(&section)?;
                tables = Some(section);
            }
            Payload::MemorySection(section) => {
                validator.memory_section(&section)?;
                memories = Some(section);
            }
            Payload::TagSection(section) => {
                validator.tag_section(&section)?;
                anyhow::bail!("TODO: tag section is currently not supported");
            }
            Payload::GlobalSection(section) => {
                validator.global_section(&section)?;
                globals = Some(section);
            }
            Payload::ExportSection(section) => {
                validator.export_section(&section)?;
                exports = Some(section);
            }
            Payload::StartSection { func, range } => {
                validator.start_section(func, &range)?;
                start_function = Some(func);
            }
            Payload::ElementSection(section) => {
                validator.element_section(&section)?;
                elements = Some(section);
            }
            Payload::DataCountSection { count, range } => {
                validator.data_count_section(count, &range)?
            }
            Payload::DataSection(section) => {
                validator.data_section(&section)?;
                data = Some(section);
            }
            Payload::CodeSectionStart {
                count,
                range,
                size: _,
            } => {
                validator.code_section_start(count, &range)?;
                function_bodies.reserve(count as usize);
            }
            Payload::CodeSectionEntry(body) => {
                function_bodies.push(code::Code::new(&mut validator, body)?)
            }
            Payload::CustomSection(_section) => {
                // Handling of custom `name`, 'producers' and DWARF sections is not yet implemented.
            }
            Payload::End(offset) => {
                let module = Module {
                    imports,
                    tables,
                    memories,
                    globals,
                    exports,
                    start_function,
                    elements,
                    data,
                    types: validator.end(offset)?,
                };

                return Ok((module, function_bodies));
            }
            // Component model is not yet supported
            Payload::ModuleSection { parser: _, range } => validator.module_section(&range)?,
            Payload::InstanceSection(section) => validator.instance_section(&section)?,
            Payload::CoreTypeSection(section) => validator.core_type_section(&section)?,
            Payload::ComponentSection { parser: _, range } => {
                validator.component_section(&range)?
            }
            Payload::ComponentInstanceSection(section) => {
                validator.component_instance_section(&section)?
            }
            Payload::ComponentAliasSection(section) => {
                validator.component_alias_section(&section)?
            }
            Payload::ComponentTypeSection(section) => validator.component_type_section(&section)?,
            Payload::ComponentCanonicalSection(section) => {
                validator.component_canonical_section(&section)?
            }
            Payload::ComponentStartSection { start, range } => {
                validator.component_start_section(&start, &range)?
            }
            Payload::ComponentImportSection(section) => {
                validator.component_import_section(&section)?
            }
            Payload::ComponentExportSection(section) => {
                validator.component_export_section(&section)?
            }
            Payload::UnknownSection {
                id,
                contents: _,
                range,
            } => validator.unknown_section(id, &range)?,
        }
    }

    // Either a `Payload::End` is processed, or an `Err` is returned.
    unreachable!()
}

impl Convert<'_> {
    /// Converts an in-memory WebAssembly binary module, and [`Write`]s the resulting Rust source
    /// code to the given output.
    ///
    /// # Errors
    ///
    /// An error will be returned if the WebAssembly module could not be parsed, the module
    /// [could not be validated], or if an error occured while writing to the `output`.
    ///
    /// [`Write`]: std::io::Write
    /// [could not be validated]: https://webassembly.github.io/spec/core/valid/index.html
    pub fn convert_from_buffer(
        &self,
        wasm: &[u8],
        output: &mut dyn std::io::Write,
    ) -> crate::Result<()> {
        use anyhow::Context;

        let (module, code) = validate_payloads(&wasm)?;

        let new_allocations;
        let allocations = match &self.allocations {
            Some(existing) => existing,
            None => {
                new_allocations = Allocations::default();
                &new_allocations
            }
        };

        let convert_function_bodies = |(index, code): (usize, code::Code)| -> crate::Result<_> {
            code.convert(&module, &self, allocations)
                .with_context(|| format!("could not format function #{index}"))
        };

        let function_definitions: Vec<code::Definition>;

        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;

            function_definitions = code
                .into_par_iter()
                .enumerate()
                .map(convert_function_bodies)
                .collect::<crate::Result<_>>()?;
        }

        #[cfg(not(feature = "rayon"))]
        {
            function_bodies = code
                .into_iter()
                .enumerate()
                .map(convert_function_bodies)
                .collect::<crate::Result<_>>()?;
        }

        let printer_options = crate::ast::Print::new(self.indentation);
        let write_function_definitions = |(index, definition): (usize, code::Definition)| {
            // TODO: Use some average # of Rust bytes per # of Wasm bytes
            let mut out = crate::buffer::Writer::new(allocations.byte_buffer_pool());

            let id = crate::ast::FuncId(index as u32);
            // TODO: Write function signature

            printer_options.print_statements(&mut out, &definition.arena, &definition.body);

            definition.finish(allocations);
            out.finish()
        };

        let function_items: Vec<crate::buffer::Buffer>;

        {
            use rayon::prelude::*;

            function_items = function_definitions
                .into_par_iter()
                .enumerate()
                .map(write_function_definitions)
                .flatten()
                .collect();
        }

        #[cfg(not(feature = "rayon"))]
        {
            function_items = function_definitions
                .into_iter()
                .enumerate()
                .map(write_function_definitions)
                .flatten()
                .collect();
        }

        crate::buffer::write_all_vectored(
            output,
            &function_items,
            &mut Vec::with_capacity(function_items.len()),
        )?;

        Ok(())
    }
}
