//! Contains the core code for translating WebAssembly to Rust.

mod const_expr;
mod data_segment;
mod display;
mod export;
mod function;
mod memory;

const EMBEDDER_PATH: &str = "$(::$embedder_start::)? $($embedder_more)::+";

#[derive(Default)]
struct GeneratedLines {
    items: Vec<bytes::Bytes>,
    fields: Vec<bytes::Bytes>,
    impls: Vec<bytes::Bytes>,
    inits: Vec<bytes::Bytes>, // Vec<Ordered<u8, bytes::Bytes>>,
}

/// Function that writes a data segment to some file, returning a path to it.
///
/// This function is passed the index of the data segment and its contents. An implementation
/// is expected to write the contents to a new file, and return a path to it such that the
/// generated code may use [`include_bytes!`].
///
/// # Errors
///
/// - `Ok(None)` is returned if a file could not be created. In this case, the data segment
///   contents are included as a byte string literal.
/// - `Err` is returned if a file could not be created.
pub type DataSegmentWriter<'a> =
    &'a (dyn Fn(u32, &[u8]) -> std::io::Result<Option<String>> + Send + Sync);

/// Provides options for translating a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
pub struct Translation<'a> {
    generated_macro_name: crate::rust::SafeIdent<'a>,
    data_segment_writer: DataSegmentWriter<'a>,
    wasm_features: &'a wasmparser::WasmFeatures,
    buffer_pool: Option<&'a crate::buffer::Pool>,
}

impl Default for Translation<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Translation<'a> {
    /// The set of WebAssembly features that are supported by default.
    pub const DEFAULT_SUPPORTED_FEATURES: wasmparser::WasmFeatures = wasmparser::WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: true,
        multi_value: true,
        bulk_memory: true,
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

    /// Gets the default options.
    pub fn new() -> Self {
        Self {
            generated_macro_name: crate::rust::Ident::DEFAULT_MACRO_NAME.into(),
            data_segment_writer: &|_, _| Ok(None),
            wasm_features: &Self::DEFAULT_SUPPORTED_FEATURES,
            buffer_pool: None,
        }
    }

    /// Sets the name of the Rust macro that is generated to contain all of the translated code.
    pub fn generated_macro_name<N>(&mut self, name: N) -> &mut Self
    where
        N: Into<crate::rust::SafeIdent<'a>>,
    {
        self.generated_macro_name = name.into();
        self
    }

    /// Sets the WebAssembly features that are supported.
    ///
    /// Attempting to translate a WebAssembly module that uses unsupported features will result in
    /// parser and validation errors.
    pub fn wasm_features(&mut self, features: &'a wasmparser::WasmFeatures) -> &mut Self {
        self.wasm_features = features;
        self
    }

    /// Specifies a pool to take buffers from during translation.
    ///
    /// This is useful if multiple WebAssembly modules are being translated with the same
    /// [`Translation`] options.
    ///
    /// If not set, a new [`Pool`] is created for every translation of one WebAssembly module.
    ///
    /// [`Pool`]: crate::buffer::Pool
    pub fn buffer_pool(&mut self, pool: &'a crate::buffer::Pool) -> &mut Self {
        self.buffer_pool = Some(pool);
        self
    }

    /// Sets the function used to write data segment contents to disk.
    ///
    /// For more information, see the documentation for [`DataSegmentWriter`].
    pub fn data_segment_writer(&mut self, writer: DataSegmentWriter<'a>) -> &mut Self {
        self.data_segment_writer = writer;
        self
    }
}

enum KnownSection<'a> {
    Memory(wasmparser::MemorySectionReader<'a>),
    Export(wasmparser::ExportSectionReader<'a>),
    Data(wasmparser::DataSectionReader<'a>),
}

struct FunctionValidator<'a> {
    validator: wasmparser::FuncToValidate<wasmparser::ValidatorResources>,
    body: wasmparser::FunctionBody<'a>,
}

struct ModuleContents<'a> {
    sections: Vec<KnownSection<'a>>,
    functions: Vec<FunctionValidator<'a>>,
    types: wasmparser::types::Types,
    start_function: Option<u32>,
    memory_import_count: u32,
}

fn parse_wasm_sections<'a>(
    wasm: &'a [u8],
    features: &wasmparser::WasmFeatures,
) -> wasmparser::Result<ModuleContents<'a>> {
    let mut validator = wasmparser::Validator::new_with_features(*features);
    let mut sections = Vec::new();
    let mut functions = Vec::new();
    let mut start_function = None;
    let mut memory_definition_count = 0;

    for result in wasmparser::Parser::new(0).parse_all(wasm) {
        use wasmparser::{Payload, ValidPayload};

        let payload = result?;
        let validated_payload = validator.payload(&payload)?;
        match payload {
            Payload::MemorySection(memories) => {
                memory_definition_count = memories.count();
                sections.push(KnownSection::Memory(memories));
            }
            Payload::ExportSection(exports) => sections.push(KnownSection::Export(exports)),
            Payload::StartSection { func, range: _ } => start_function = Some(func),
            Payload::DataSection(data) => sections.push(KnownSection::Data(data)),
            _ => (),
        }

        match validated_payload {
            ValidPayload::Ok | ValidPayload::Parser(_) => (),
            ValidPayload::Func(validator, body) => {
                functions.push(FunctionValidator { validator, body })
            }
            ValidPayload::End(types) => {
                return Ok(ModuleContents {
                    sections,
                    functions,
                    start_function,
                    memory_import_count: types.memory_count() - memory_definition_count,
                    types,
                })
            }
        }
    }

    unreachable!("missing end payload");
}

fn write_all_vectored(
    output: &mut dyn std::io::Write,
    bytes: Vec<bytes::Bytes>,
) -> std::io::Result<()> {
    // TODO: Actually call write_vectored
    for buffer in bytes.into_iter() {
        output.write_all(&buffer)?;
    }

    Ok(())
}

impl Translation<'_> {
    /// Translates an in-memory WebAssembly binary module, and [`Write`]s the resulting Rust source
    /// code to the given output.
    ///
    /// # Errors
    ///
    /// An error will be returned if the WebAssembly module could not be parsed, the module
    /// [could not be validated], or if an error occured while writing to the `output`.
    ///
    /// [`Write`]: std::io::Write
    /// [could not be validated]: https://webassembly.github.io/spec/core/valid/index.html
    pub fn translate_from_buffer(
        &self,
        wasm: &[u8],
        output: &mut dyn std::io::Write,
    ) -> crate::Result<()> {
        use rayon::prelude::*;

        let ModuleContents {
            sections,
            functions,
            types,
            start_function,
            memory_import_count,
        } = parse_wasm_sections(wasm, self.wasm_features)?;

        let func_validator_allocation_pool =
            crossbeam_queue::SegQueue::<wasmparser::FuncValidatorAllocations>::new();

        let new_buffer_pool;
        let buffer_pool = match self.buffer_pool {
            Some(existing) => existing,
            None => {
                new_buffer_pool = crate::buffer::Pool::default();
                &new_buffer_pool
            }
        };

        // Generate Rust code for the functions
        let function_decls = functions
            .into_par_iter()
            .map(|func| {
                let mut out = crate::buffer::Writer::new(buffer_pool);
                let mut validator = func.validator.into_validator(
                    if let Some(allocs) = func_validator_allocation_pool.pop() {
                        allocs
                    } else {
                        Default::default()
                    },
                );

                function::write_definition(&mut out, &mut validator, &func.body, &types)?;
                func_validator_allocation_pool.push(validator.into_allocations());
                Ok(out.finish())
            })
            .collect::<wasmparser::Result<Vec<_>>>()?;

        std::mem::drop(func_validator_allocation_pool);

        // Generate globals, exports, memories, tables, and other things
        let mut item_lines = Vec::new();
        let mut field_lines = Vec::new();
        let mut init_lines = Vec::new();
        let mut impl_line_groups = function_decls;

        // Note that because `sections` is in a consistent order, all of these contents will be in
        // a consistent order too.
        {
            let contents = sections
                .into_par_iter()
                .map(|section| match section {
                    KnownSection::Memory(memories) => {
                        memory::write(buffer_pool, memories, memory_import_count)
                            .map_err(Into::into)
                    }
                    KnownSection::Export(exports) => {
                        export::write(buffer_pool, exports, &types).map_err(Into::into)
                    }
                    KnownSection::Data(data) => {
                        data_segment::write(buffer_pool, data, self.data_segment_writer)
                    }
                })
                .collect::<Vec<crate::Result<_>>>();

            let mut impl_lines = Vec::new();

            for result in contents.into_iter() {
                let mut lines = result?;
                item_lines.append(&mut lines.items);
                field_lines.append(&mut lines.fields);
                init_lines.append(&mut lines.inits);
                impl_lines.append(&mut lines.impls);
            }

            impl_line_groups.push(impl_lines);
        }

        let field_lines = field_lines;
        let init_lines = init_lines;
        let impl_line_groups = impl_line_groups;

        // Write the file contents
        writeln!(
            output,
            "// automatically generated by wasm2rs\nmacro_rules! {} {{",
            self.generated_macro_name
        )?;

        output.write_all(
            concat!(
                "    ($vis:vis mod $module:ident use $(:: $embedder_start:ident ::)? $($embedder_more:ident)::+) => {\n",
                // Names might be mangled
                "#[allow(non_snake_case)]\n",
                // Some functions may not be called
                "#[allow(dead_code)]\n",
                // Some branches may not be taken (e.g. infinite loops detected by `rustc`)
                "#[allow(unreachable_code)]\n",
                "$vis mod $module {\n",
                //
                "  #[derive(Debug)]\n  #[non_exhaustive]\n",
                "  $vis struct Instance {\n",
            )
            .as_bytes(),
        )?;

        writeln!(output, "    _embedder: {EMBEDDER_PATH}::State,")?;

        // Write fields
        write_all_vectored(output, field_lines)?;

        // Write methods
        output.write_all(concat!("  }\n\n  impl Instance {\n").as_bytes())?;
        for impl_lines in impl_line_groups {
            write_all_vectored(output, impl_lines)?;
        }

        // Write instantiate function
        writeln!(
            output,
            "    $vis fn instantiate(embedder: {}::State) -> {}::Result<Self> {{",
            EMBEDDER_PATH, EMBEDDER_PATH,
        )?;
        write_all_vectored(output, init_lines)?;
        writeln!(output, "      let instantiated = Self {{")?;

        for i in 0..types.memory_count() {
            writeln!(
                output,
                "        {},",
                display::MemId(i + memory_import_count)
            )?;
        }

        writeln!(output, "        _embedder: embedder,\n      }};\n")?;

        if let Some(start_index) = start_function {
            writeln!(output, "      self.{}()?;", display::FuncId(start_index))?;
        } else {
            output.write_all(b"      // No start function\n")?;
        }

        output.write_all(b"\n      Ok(instantiated)\n    }\n  }\n}\n")?; // impl Instance

        // Other macro cases
        output.write_all(b"    };\n    ($vis:vis mod $module:ident) => {\n")?;
        writeln!(
            output,
            "        {}!{{$vis mod $module use ::wasm2rs_rt::embedder}}\n    }};",
            self.generated_macro_name
        )?;

        writeln!(
            output,
            "    (use $(:: $embedder_start:ident ::)? $($embedder_more:ident)::+) => {{ {}!{{mod wasm use $embedder}} }};",
            self.generated_macro_name
        )?;

        writeln!(
            output,
            "    ($vis:vis) => {{ {}!{{$vis mod wasm}} }};",
            self.generated_macro_name
        )?;

        writeln!(
            output,
            "    () => {{ {}!{{pub}} }};\n}}",
            self.generated_macro_name
        )?;

        output.flush().map_err(Into::into)
    }
}

impl std::fmt::Debug for Translation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Translation")
            .field("generated_macro_name", &self.generated_macro_name)
            .field("wasm_features", self.wasm_features)
            .finish_non_exhaustive()
    }
}
