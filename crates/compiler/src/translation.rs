use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator};
use wasmparser::WasmModuleResources;

/// Provides options for translating a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
#[derive(Debug)]
pub struct Translation {
    //buffers: dyn Fn() -> Vec<u8>,
    //thread_pool: Option<rayon::ThreadPool>,
    //runtime_crate_path: CratePath,
    //visibility: Public|Crate(Option<Path>),
}

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

    fn compile_function(
        &self,
        body: &wasmparser::FunctionBody,
        validator: &mut wasmparser::FuncValidator<wasmparser::ValidatorResources>,
    ) -> crate::Result<Vec<u8>> {
        // Note that write operations on a `Vec` currently always return `Ok`
        use std::io::Write as _;

        let mut b = Vec::new();
        let _ = write!(&mut b, "fn f{}(&self", validator.index());

        let func_type = validator
            .resources()
            .type_of_function(validator.index())
            .unwrap();

        // TODO: Move this to the `rust` module, have function that returns rust identifier/path impl Display for a Wasm ValType
        struct ValType(wasmparser::ValType);

        impl std::fmt::Display for ValType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.0 {
                    wasmparser::ValType::I32 => f.write_str("i32"),
                    wasmparser::ValType::I64 => f.write_str("f64"),
                    other => todo!("how to write {other}?"),
                }
            }
        }

        {
            // Write the parameter types
            for (i, ty) in func_type.params().iter().enumerate() {
                let _ = write!(&mut b, ", l{i}: {}", ValType(*ty));
            }

            let _ = write!(&mut b, ") ");

            let results = func_type.results();
            if !results.is_empty() {
                let _ = write!(&mut b, "-> ");
                if results.len() > 1 {
                    let _ = write!(&mut b, "(");
                }
            }

            // Write the result types
            for (i, ty) in results.iter().enumerate() {
                if i > 0 {
                    let _ = write!(&mut b, ", ");
                }

                let _ = write!(&mut b, "{}", ValType(*ty));
            }

            if results.len() > 1 {
                let _ = write!(&mut b, ")");
            }
        }

        let _ = writeln!(&mut b, " {{");

        let result_count = u32::try_from(func_type.results().len()).expect("too many results");

        // Write local variables
        #[derive(Clone, Copy)]
        #[repr(transparent)]
        struct LocalVar(u32);

        impl std::fmt::Display for LocalVar {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "_l{}", self.0)
            }
        }

        {
            let mut local_index = u32::try_from(func_type.params().len()).unwrap_or(u32::MAX);
            let mut locals_reader = body.get_locals_reader()?;
            let locals_count = locals_reader.get_count();
            for _ in 0..locals_count {
                let (count, ty) = locals_reader.read()?;
                validator.define_locals(locals_reader.original_position(), count, ty)?;

                for _ in 0..count {
                    let _ = writeln!(
                        &mut b,
                        "let mut {}: {} = Default::default();",
                        LocalVar(local_index),
                        ValType(ty)
                    );
                    local_index += 1;
                }
            }
        }

        let mut operators_reader = body.get_operators_reader()?;
        while !operators_reader.eof() {
            let (op, op_offset) = operators_reader.read_with_offset()?;

            #[derive(Clone, Copy)]
            #[repr(transparent)]
            struct StackValue(u32);

            impl std::fmt::Display for StackValue {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "_s{}", self.0)
                }
            }

            #[derive(Clone, Copy)]
            enum PoppedValue {
                Pop(StackValue),
                Underflow,
            }

            impl std::fmt::Display for PoppedValue {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::Pop(v) => std::fmt::Display::fmt(&v, f),
                        Self::Underflow => f.write_str("(::core::unimplemented!(\"code generation bug, operand stack underflow occured\"))"),
                    }
                }
            }

            let pop_value = |depth: u32| {
                match validator.get_operand_type(depth as usize) {
                    Some(Some(_)) => {
                        // TODO: Basic copying only good for numtype and vectype, have to call Runtime::clone for funcref + externref
                        let height = validator.operand_stack_height() - depth - 1;
                        PoppedValue::Pop(StackValue(height))
                    }
                    Some(None) => todo!("generate code for unreachable value, call Runtime::trap"),
                    None => {
                        // A stack underflow should be caught later by the validator
                        PoppedValue::Underflow
                    }
                }
            };

            // For `return` and `end` instructions
            let write_return = |b: &mut Vec<u8>| match result_count {
                0 => {
                    let _ = writeln!(b, "return;");
                }
                1 => {
                    let _ = writeln!(b, "return {};", pop_value(0));
                }
                _ => {
                    for i in 0..result_count {
                        let _ = writeln!(
                            b,
                            "let r{} = {};",
                            StackValue(result_count - i - 1),
                            pop_value(i),
                        );
                    }

                    let _ = write!(b, "return (");
                    for i in 0..result_count {
                        if i > 0 {
                            let _ = write!(b, ", ");
                        }

                        let _ = write!(b, "r{i}");
                    }
                    let _ = writeln!(b, ");");
                }
            };

            use wasmparser::Operator;

            match op {
                Operator::End => match validator.control_stack_height() {
                    0 => unreachable!("control frame stack was unexpectedly empty"),
                    1 => write_return(&mut b),
                    _ => todo!("end of blocks not yet supported"),
                },
                Operator::Return => write_return(&mut b),
                Operator::LocalGet { local_index } => {
                    let _ = writeln!(
                        &mut b,
                        "let {} = {}",
                        StackValue(validator.operand_stack_height()),
                        LocalVar(local_index)
                    );
                }
                Operator::I32Const { value } => {
                    let _ = writeln!(
                        &mut b,
                        "let {} = {value}i32;",
                        StackValue(validator.operand_stack_height()),
                    );
                }
                Operator::I32Add => {
                    let result_value = pop_value(1);
                    let _ = writeln!(
                        &mut b,
                        "let {} = i32::wrapping_add({}, {})",
                        result_value,
                        result_value,
                        pop_value(0)
                    );
                }
                _ => todo!("translate {op:?}"),
            }

            validator.op(op_offset, &op)?;
        }

        // Implicit return generated when last `end` is handled.

        validator.finish(operators_reader.original_position())?;

        let _ = writeln!(&mut b, "}}");
        Ok(b)
    }

    /// Translates an in-memory WebAssembly binary module, and [`Write`]s the resulting Rust source
    /// code to the given output.
    ///
    /// If the `rayon` feature is enabled, portions of the parsing, validation, and translation
    /// process may be run in parallel.
    pub fn compile_from_buffer(
        self,
        wasm: &[u8],
        output: &mut dyn std::io::Write,
    ) -> crate::Result<()> {
        let mut validator = wasmparser::Validator::new_with_features(Self::SUPPORTED_FEATURES);
        let payloads = wasmparser::Parser::new(0)
            .parse_all(wasm)
            .collect::<wasmparser::Result<Vec<_>>>()?;

        let payloads_ref = payloads.as_slice();
        let validate_payloads = move || -> wasmparser::Result<_> {
            let mut functions = Vec::new();

            for payload in payloads_ref {
                use wasmparser::ValidPayload;

                if let wasmparser::Payload::FunctionSection(funcs) = payload {
                    functions.reserve_exact(funcs.count() as usize);
                }

                match validator.payload(payload)? {
                    ValidPayload::Ok | ValidPayload::Parser(_) => (),
                    ValidPayload::Func(func, body) => functions.push((func, body)),
                    ValidPayload::End(types) => return Ok((functions, types)),
                }
            }

            unreachable!("missing end payload");
        };

        #[derive(Default)]
        struct Definitions<'a> {
            imports: Box<[wasmparser::Import<'a>]>,
            exports: Box<[wasmparser::Export<'a>]>,
        }

        let parse_definitions = move || -> wasmparser::Result<_> {
            use wasmparser::Payload;

            let mut definitions = Definitions::default();

            for payload in payloads_ref {
                match payload {
                    Payload::ImportSection(import_sec) => {
                        definitions.imports = import_sec
                            .clone()
                            .into_iter()
                            .collect::<wasmparser::Result<_>>()?
                    }
                    Payload::ExportSection(export_sec) => {
                        definitions.exports = export_sec
                            .clone()
                            .into_iter()
                            .collect::<wasmparser::Result<_>>()?
                    }
                    _ => (),
                }
            }

            Ok(definitions)
        };

        let types;
        let functions;
        let definitions;

        #[cfg(feature = "rayon")]
        {
            let (validation_result, parse_result) =
                rayon::join(validate_payloads, parse_definitions);

            (functions, types) = validation_result?;
            definitions = parse_result?;
        }

        #[cfg(not(feature = "rayon"))]
        {
            (functions, types) = validate_payloads()?;
            definitions = validate_definitions()?;
        }

        #[cfg(feature = "rayon")]
        let function_decls: Vec<_> = {
            use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

            let mut function_decls_unsorted = vec![(0, Vec::new()); functions.len()];

            // TODO: Create a pool of FuncValidatorAllocations
            functions
                .into_par_iter()
                .zip_eq(function_decls_unsorted.par_iter_mut())
                .try_for_each(|((func, body), dst)| {
                    let mut validator = func.into_validator(Default::default());
                    *dst = (
                        validator.index(),
                        self.compile_function(&body, &mut validator)?,
                    );
                    crate::Result::Ok(())
                })?;

            function_decls_unsorted
                .into_iter()
                .map(|(_, b)| b)
                .collect()
        };

        #[cfg(not(feature = "rayon"))]
        let function_decls: Vec<_> = {
            let mut allocs = wasmparser::FuncValidatorAllocations::default();
            functions
                .into_iter()
                .map(|(func, body)| {
                    let mut validator = func.into_validator(core::mem::take(&mut allocs));
                    function_decls.push(self.compile_function(&body, &mut validator)?);
                    allocs = validator.into_allocations();
                    crate::Result::Ok(())
                })
                .collect::<crate::Result<_>>()?;
        };

        writeln!(output, "/* automatically generated by wasm2rs */")?;

        // Same code generated for `return` and last `end`.
        writeln!(output, "#[allow(clippy::needless_return)]")?;

        // Code generator isn't smart (TODO: Is validator.get_control_frame() enough to skip unreachable code)
        writeln!(output, "#[allow(unreachable_code)]")?;

        writeln!(output, "pub struct Instance {{}}")?; // TODO: Insert global variables in struct as public fields
        let _ = (types, definitions);

        writeln!(output, "impl Instance {{")?;

        // TODO: output.write_vectored(bufs)?;
        for buf in function_decls {
            output.write_all(&buf)?;
        }

        writeln!(output, "}}")?;
        output.flush()?;
        Ok(())
    }
}
