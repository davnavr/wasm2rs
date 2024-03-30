//! Contains the core code for converting WebAssembly to Rust.

mod func_validator_allocation_pool;
mod options;

pub use func_validator_allocation_pool::FuncValidatorAllocationPool;
pub use options::{DataSegmentWriter, DebugInfo, StackOverflowChecks};

/// Provides options for converting a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
pub struct Convert<'a> {
    generated_macro_name: crate::ident::SafeIdent<'a>,
    data_segment_writer: DataSegmentWriter<'a>,
    // wasm_features: &'a wasmparser::WasmFeatures,
    stack_overflow_checks: StackOverflowChecks,
    debug_info: DebugInfo,
    buffer_pool: Option<&'a crate::buffer::Pool>,
    func_validator_allocation_pool: Option<&'a crate::FuncValidatorAllocationPool>,
}

impl Default for Convert<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Convert<'_> {
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

    /// Gets the default options.
    pub fn new() -> Self {
        Self {
            generated_macro_name: crate::ident::Ident::DEFAULT_MACRO_NAME.into(),
            data_segment_writer: &|_, _| Ok(None),
            // wasm_features: &Self::DEFAULT_SUPPORTED_FEATURES,
            stack_overflow_checks: Default::default(),
            debug_info: Default::default(),
            buffer_pool: None,
            func_validator_allocation_pool: None,
        }
    }
}

impl std::fmt::Debug for Convert<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Convert")
            .field("generated_macro_name", &self.generated_macro_name)
            .field("stack_overflow_checks", &self.stack_overflow_checks)
            .field("debug_info", &self.debug_info)
            .finish_non_exhaustive()
    }
}
