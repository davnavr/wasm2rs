//! Contains the core code for translating WebAssembly to Rust.

mod display;
mod export;
mod function;

/// Provides options for translating a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
#[derive(Debug)]
pub struct Translation<'a> {
    //runtime_path: CratePath,
    //visibility: Public|Crate(Option<Path>),
    generated_module_name: crate::rust::SafeIdent<'a>,
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
            generated_module_name: crate::rust::Ident::DEFAULT_MODULE_NAME.into(),
            wasm_features: &Self::DEFAULT_SUPPORTED_FEATURES,
            buffer_pool: None,
        }
    }

    /// Sets the name of the Rust module that is generated to contain all of the translated code.
    pub fn generated_module_name<N>(&mut self, name: N) -> &mut Self
    where
        N: Into<crate::rust::SafeIdent<'a>>,
    {
        self.generated_module_name = name.into();
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
    pub fn buffer_pool(&mut self, pool: &'a crate::buffer::Pool) -> &mut Self {
        self.buffer_pool = Some(pool);
        self
    }
}

enum KnownSection<'a> {
    Export(wasmparser::ExportSectionReader<'a>),
}

struct FunctionValidator<'a> {
    validator: wasmparser::FuncToValidate<wasmparser::ValidatorResources>,
    body: wasmparser::FunctionBody<'a>,
}

struct ModuleContents<'a> {
    sections: Vec<KnownSection<'a>>,
    functions: Vec<FunctionValidator<'a>>,
    types: wasmparser::types::Types,
}

fn parse_wasm_sections<'a>(
    wasm: &'a [u8],
    features: &wasmparser::WasmFeatures,
) -> wasmparser::Result<ModuleContents<'a>> {
    let mut validator = wasmparser::Validator::new_with_features(*features);
    let mut sections = Vec::new();
    let mut functions = Vec::new();

    for result in wasmparser::Parser::new(0).parse_all(wasm) {
        use wasmparser::{Payload, ValidPayload};

        let payload = result?;

        let validated_payload = validator.payload(&payload)?;

        match payload {
            Payload::ExportSection(exports) => sections.push(KnownSection::Export(exports.clone())),
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
                    types,
                })
            }
        }
    }

    unreachable!("missing end payload");
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
        } = parse_wasm_sections(wasm, &self.wasm_features)?;

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

        // Write the functions
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

        todo!()
    }
}
