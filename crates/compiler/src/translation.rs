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
#[derive(Clone, Copy)]
#[repr(transparent)]
struct LocalVar(u32);

impl std::fmt::Display for LocalVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_l{}", self.0)
    }
}

const RT_CRATE_PATH: &str = "::wasm2rs_rt";

fn get_function_type(ty: &wasmparser::SubType) -> &wasmparser::FuncType {
    if let wasmparser::SubType {
        is_final: true,
        supertype_idx: None,
        composite_type: wasmparser::CompositeType::Func(sig),
    } = ty
    {
        sig
    } else {
        unimplemented!("expected function type, but got unsupported type: {ty:?}")
    }
}

fn get_block_type<'a>(
    types: &'a wasmparser::types::Types,
    ty: &'a wasmparser::BlockType,
) -> (&'a [wasmparser::ValType], &'a [wasmparser::ValType]) {
    use wasmparser::BlockType;

    match ty {
        BlockType::Empty => (&[], &[]),
        BlockType::Type(result) => (&[], std::slice::from_ref(result)),
        BlockType::FuncType(sig) => {
            let func_type =
                get_function_type(types.get(types.core_type_at(*sig).unwrap_sub()).unwrap());
            (func_type.params(), func_type.results())
        }
    }
}

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
        multi_value: true,
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

    fn write_function_signature(
        &self,
        sig: &wasmparser::FuncType,
        b: &mut Vec<u8>,
    ) -> wasmparser::Result<()> {
        use std::io::Write as _;

        // Write the parameter types
        for (i, ty) in sig.params().iter().enumerate() {
            if i > 0 {
                let _ = write!(b, ", ");
            }

            let _ = write!(
                b,
                "{}: {}",
                LocalVar(u32::try_from(i).expect("too many parameters")),
                ValType(*ty)
            );
        }

        let _ = write!(b, ") ");

        let results = sig.results();
        if !results.is_empty() {
            let _ = write!(b, "-> ");
            if results.len() > 1 {
                let _ = write!(b, "(");
            }
        }

        // Write the result types
        for (i, ty) in results.iter().enumerate() {
            if i > 0 {
                let _ = write!(b, ", ");
            }

            let _ = write!(b, "{}", ValType(*ty));
        }

        if results.len() > 1 {
            let _ = write!(b, ")");
        }

        Ok(())
    }

    fn compile_function(
        &self,
        body: &wasmparser::FunctionBody,
        validator: &mut wasmparser::FuncValidator<wasmparser::ValidatorResources>,
        types: &wasmparser::types::Types,
    ) -> wasmparser::Result<Vec<u8>> {
        use wasmparser::WasmModuleResources as _;

        // Note that write operations on a `Vec` currently always return `Ok`
        use std::io::Write as _;

        let mut b = Vec::new();
        let _ = write!(&mut b, "fn _f{}(&self, ", validator.index());

        let func_type = validator
            .resources()
            .type_of_function(validator.index())
            .unwrap();

        self.write_function_signature(func_type, &mut b)?;

        let _ = writeln!(&mut b, " {{");

        let func_result_count =
            u32::try_from(func_type.results().len()).expect("too many function results");

        // Write local variables
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
                    Self::Underflow => f.write_str("::core::unimplemented!(\"code generation bug, operand stack underflow occured\")"),
                }
            }
        }

        let pop_value = |validator: &wasmparser::FuncValidator<_>, depth: u32| {
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

        struct Label(u32);

        impl std::fmt::Display for Label {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "'l{}", self.0)
            }
        }

        let write_block_start = |b: &mut Vec<u8>, label: Label, operand_height, ty| {
            let result_count =
                u32::try_from(get_block_type(types, &ty).1.len()).expect("too many results");

            if result_count > 0 {
                let _ = write!(b, "let ");

                if result_count > 1 {
                    let _ = write!(b, "(");
                }

                for i in 0..result_count {
                    if i > 0 {
                        let _ = write!(b, ", ");
                    }

                    let _ = write!(
                        b,
                        "{}",
                        StackValue(i.checked_add(operand_height).expect("too many results"))
                    );
                }

                if result_count > 1 {
                    let _ = write!(b, ")");
                }

                let _ = write!(b, " = ");
            }

            let _ = write!(b, " {label}: ");
        };

        enum EndKind {
            ExplicitReturn,
            ImplicitReturn,
            //Branch(Label)
            Block,
        }

        impl std::fmt::Display for EndKind {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::ExplicitReturn => f.write_str("return "),
                    //Self::Branch(label) => write!(f, "break {label}"),
                    Self::Block | Self::ImplicitReturn => Ok(()),
                }
            }
        }

        // For `return` and `end` instructions
        let write_end = |validator: &_, b: &mut Vec<u8>, kind: EndKind, result_count| {
            match result_count {
                0u32 => {
                    let _ = write!(b, "{kind}");
                }
                1 => {
                    let _ = write!(b, "{kind}{}", pop_value(validator, 0));
                }
                _ => {
                    for i in 0..result_count {
                        let _ = writeln!(
                            b,
                            "let _r{} = {};",
                            result_count - i - 1,
                            pop_value(validator, i),
                        );
                    }

                    let _ = write!(b, "{kind}(");
                    for i in 0..result_count {
                        if i > 0 {
                            let _ = write!(b, ", ");
                        }

                        let _ = write!(b, "_r{i}");
                    }
                    let _ = write!(b, ")");
                }
            }

            let _ = if matches!(kind, EndKind::ExplicitReturn) {
                writeln!(b, ";")
            } else {
                writeln!(b)
            };
        };

        let mut operators_reader = body.get_operators_reader()?;
        while !operators_reader.eof() {
            use wasmparser::Operator;

            let (op, op_offset) = operators_reader.read_with_offset()?;

            let current_frame = validator
                .get_control_frame(0)
                .expect("control frame stack was unexpectedly empty");

            if current_frame.unreachable && !matches!(op, Operator::End | Operator::Else) {
                // Although code is unreachable, WASM spec still requires it to be validated
                validator.op(op_offset, &op)?;
                // Don't generate Rust code
                continue;
            }

            match op {
                Operator::Nop => (),
                Operator::Block { blockty } => {
                    write_block_start(
                        &mut b,
                        Label(validator.control_stack_height() + 1),
                        validator.operand_stack_height(),
                        blockty,
                    );
                    let _ = writeln!(b, "{{");
                }
                Operator::End => match validator.control_stack_height() {
                    0 => unreachable!(),
                    1 if current_frame.unreachable => (),
                    1 => write_end(
                        validator,
                        &mut b,
                        EndKind::ImplicitReturn,
                        func_result_count,
                    ),
                    _ => {
                        let result_count = get_block_type(types, &current_frame.block_type)
                            .1
                            .len()
                            .try_into()
                            .expect("too many block results");

                        write_end(validator, &mut b, EndKind::Block, result_count);
                        let _ = write!(b, "}}");

                        let _ = if result_count > 0 {
                            writeln!(b, ";")
                        } else {
                            writeln!(b)
                        };
                    }
                },
                // TODO: Implicit return if height is 0
                Operator::Return => write_end(
                    validator,
                    &mut b,
                    if validator.control_stack_height() == 1 {
                        EndKind::ImplicitReturn
                    } else {
                        EndKind::ExplicitReturn
                    },
                    func_result_count,
                ),
                Operator::LocalGet { local_index } => {
                    let _ = writeln!(
                        &mut b,
                        "let {} = {};",
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
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {} = i32::wrapping_add({}, {});",
                        result_value,
                        result_value,
                        pop_value(validator, 0)
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

    fn write_function_export(
        &self,
        name: &str,
        index: u32,
        types: &wasmparser::types::Types,
        buf: &mut Vec<u8>,
    ) -> crate::Result<()> {
        use std::io::Write as _;

        let _ = write!(
            buf,
            "pub fn {}(&self, ",
            crate::rust::Ident::new(name).expect("TODO: implement name mangling")
        );

        let func_type = get_function_type(types.get(types.core_function_at(index)).unwrap());
        self.write_function_signature(func_type, buf)?;
        let _ = write!(buf, " {{ self._f{index}(");

        for i in 0..u32::try_from(func_type.params().len()).expect("too many parameters") {
            if i > 0 {
                let _ = write!(buf, ", ");
            }

            let _ = write!(buf, "{}", LocalVar(i));
        }

        let _ = writeln!(buf, ") }}");
        Ok(())
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

        #[derive(Clone, Copy, Debug)]
        struct ExportEntry {
            name: u32,
            index: u32,
        }

        #[derive(Default, Debug)]
        struct Definitions<'a> {
            imports: Box<[wasmparser::Import<'a>]>,
            export_names: Box<[&'a str]>,
            function_exports: Box<[ExportEntry]>,
        }

        // TODO: parse sections in parallel with rayon
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
                        let mut export_names = Vec::with_capacity(export_sec.count() as usize);
                        let mut function_exports = Vec::with_capacity(export_names.capacity());
                        for result in export_sec.clone() {
                            use wasmparser::ExternalKind;

                            let export = result?;
                            let name = u32::try_from(export_names.len()).expect("too many exports");
                            export_names.push(export.name);
                            match export.kind {
                                ExternalKind::Func => {
                                    function_exports.push(ExportEntry {
                                        name,
                                        index: export.index,
                                    });
                                }
                                _ => todo!("unsupported export: {export:?}"),
                            }
                        }

                        definitions.export_names = export_names.into_boxed_slice();
                        definitions.function_exports = function_exports.into_boxed_slice();
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

        // Generate function bodies
        #[cfg(feature = "rayon")]
        let function_decls: Vec<_> = {
            use rayon::prelude::*;

            let types = &types;
            let mut function_decls_unsorted = vec![(0, Vec::new()); functions.len()];

            // TODO: Zip keeps order of items, remove the extra Vec
            // TODO: Create a pool of FuncValidatorAllocations
            functions
                .into_par_iter()
                .zip_eq(function_decls_unsorted.par_iter_mut())
                .try_for_each(|((func, body), dst)| {
                    let mut validator = func.into_validator(Default::default());
                    *dst = (
                        validator.index(),
                        self.compile_function(&body, &mut validator, types)?,
                    );
                    crate::Result::Ok(())
                })?;

            // Ensure that functions are emitted in the same order.
            function_decls_unsorted.par_sort_unstable_by_key(|(n, _)| *n);

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

        // Generate function exports, no conflict since export names are unique in WebAssembly.
        #[cfg(feature = "rayon")]
        let function_exports: Vec<_> = {
            use rayon::prelude::*;

            let export_names = definitions.export_names.as_ref();
            let mut translations = vec![Vec::new(); definitions.function_exports.len()];

            Vec::from(definitions.function_exports)
                .into_par_iter()
                .zip_eq(translations.par_iter_mut())
                .try_for_each(|(export, buf)| {
                    self.write_function_export(
                        export_names[export.name as usize],
                        export.index,
                        &types,
                        buf,
                    )
                })?;

            translations
        };

        #[cfg(not(feature = "rayon"))]
        todo!("compilation without rayon currently unsupported");

        writeln!(output, "/* automatically generated by wasm2rs */")?;

        // Some branches might not be taken
        writeln!(output, "#[allow(unused_labels)]")?;

        writeln!(output, "pub mod wasm {{")?;

        // TODO: Type parameter for imports
        write!(
            output,
            "pub struct Instance<RT = StdRuntime> where RT: Runtime,"
        )?;

        // TODO: Insert global variables in struct as public fields
        writeln!(output, "{{")?;
        writeln!(output, "_rt: RT,")?;
        writeln!(output, "}}")?;

        writeln!(
            output,
            "pub trait Runtime: {RT_CRATE_PATH}::trap::Trap {{}}"
        )?;

        writeln!(
            output,
            "#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]\npub struct StdRuntime;"
        )?;
        writeln!(output, "impl {RT_CRATE_PATH}::trap::Trap for StdRuntime {{")?;
        writeln!(output, "type Repr = ::core::convert::Infallible;")?;
        write!(output, "#[cold]\n#[inline(never)]\nfn trap(&self, code: {RT_CRATE_PATH}::trap::TrapCode) -> Self::Repr {{ ")?;
        writeln!(output, "::core::panic!(\"{{code}}\") }}")?;
        writeln!(output, "}}")?;
        writeln!(output, "impl Runtime for StdRuntime {{}}")?;

        writeln!(output, "impl<RT: Runtime> Instance<RT> {{")?;

        // TODO: output.write_vectored(bufs)?;
        for buf in function_decls {
            output.write_all(&buf)?;
        }

        // TODO: Parameter for imports
        writeln!(output, "pub fn instantiate(runtime: RT) -> ::core::result::Result<Self, <RT as {RT_CRATE_PATH}::trap::Trap>::Repr> {{")?;
        // TODO: Call start function
        writeln!(output, "Ok(Self {{ _rt: runtime }})")?;
        writeln!(output, "}}")?;

        for buf in function_exports {
            output.write_all(&buf)?;
        }

        writeln!(output, "}}}}")?;
        output.flush()?;
        Ok(())
    }
}
