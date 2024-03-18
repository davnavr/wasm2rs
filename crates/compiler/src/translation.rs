// TODO: Move this to the `rust` module, have function that returns rust identifier/path impl Display for a Wasm ValType
struct ValType(wasmparser::ValType);

impl std::fmt::Display for ValType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            wasmparser::ValType::I32 => f.write_str("i32"),
            wasmparser::ValType::I64 => f.write_str("i64"),
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

#[derive(Clone, Copy)]
struct MemId(u32);

impl std::fmt::Display for MemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_mem_{}", self.0)
    }
}

/// Provides options for translating a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
#[derive(Debug)]
pub struct Translation<'a> {
    //buffers: dyn Fn() -> Vec<u8>,
    //thread_pool: Option<rayon::ThreadPool>,
    //runtime_crate_path: CratePath,
    //visibility: Public|Crate(Option<Path>),
    generated_module_name: crate::rust::SafeIdent<'a>,
}

impl Default for Translation<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Translation<'a> {
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
        Self {
            generated_module_name: crate::rust::SafeIdent::from("wasm"),
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
                "mut {}: {}",
                LocalVar(u32::try_from(i).expect("too many parameters")),
                ValType(*ty)
            );
        }

        let _ = write!(b, ") -> ::core::result::Result<");

        let results = sig.results();

        if results.len() != 1 {
            let _ = write!(b, "(");
        }

        // Write the result types
        for (i, ty) in results.iter().enumerate() {
            if i > 0 {
                let _ = write!(b, ", ");
            }

            let _ = write!(b, "{}", ValType(*ty));
        }

        if results.len() != 1 {
            let _ = write!(b, ")");
        }

        let _ = write!(b, ", <RT as {RT_CRATE_PATH}::trap::Trap>::Repr>");

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
                    let default_value = match ty {
                        wasmparser::ValType::I32 | wasmparser::ValType::I64 => "0",
                        _ => "Default::default()",
                    };

                    let _ = writeln!(
                        &mut b,
                        "let mut {}: {} = {default_value};",
                        LocalVar(local_index),
                        ValType(ty),
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
                    Self::Underflow if f.alternate() => f.write_str("_"),
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

        #[derive(Clone, Copy)]
        struct Label(u32);

        impl std::fmt::Display for Label {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "'l{}", self.0)
            }
        }

        #[must_use]
        struct BlockInputs {
            label: Label,
            count: u32,
            /// Operand stack height at which block stack inputs begin.
            height: u32,
        }

        impl BlockInputs {
            fn write(self, b: &mut Vec<u8>) {
                for i in 0..self.count {
                    let operand = StackValue(self.height + i);
                    let _ = writeln!(b, "let mut _b{}{operand} = {operand};", self.label.0);
                }
            }
        }

        let write_block_start = |b: &mut Vec<u8>, label: Label, operand_height, ty| {
            let (argument_types, result_types) = get_block_type(types, &ty);
            let argument_count = u32::try_from(argument_types.len()).expect("too many parameters");
            let result_count = u32::try_from(result_types.len()).expect("too many results");
            let result_start_height = operand_height - argument_count;

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
                        StackValue(
                            i.checked_add(result_start_height)
                                .expect("too many results")
                        )
                    );
                }

                if result_count > 1 {
                    let _ = write!(b, ")");
                }

                let _ = write!(b, " = ");
            }

            let _ = write!(b, " {label}: ");
            BlockInputs {
                label,
                count: argument_count,
                height: result_start_height,
            }
        };

        #[derive(Clone, Copy)]
        enum BranchKind {
            ExplicitReturn,
            ImplicitReturn,
            Block,
            Loop(Label),
            /// Branch out of a `block` or `if`/`else` block.
            Branch(Label),
        }

        impl BranchKind {
            fn write_start(&self, b: &mut Vec<u8>) {
                match self {
                    Self::ExplicitReturn => {
                        let _ = write!(b, "return Ok(");
                    }
                    Self::ImplicitReturn => {
                        let _ = write!(b, "Ok(");
                    }
                    Self::Block => (),
                    Self::Loop(label) | Self::Branch(label) => {
                        let _ = write!(b, "break {label} ");
                    }
                }
            }
        }

        // For `return`, `end`, and `br` instructions
        let write_control_flow =
            |validator: &_, b: &mut Vec<u8>, kind: BranchKind, result_count| {
                if result_count == 0u32 {
                    let _ = match kind {
                        BranchKind::ExplicitReturn => writeln!(b, "return Ok(());"),
                        BranchKind::ImplicitReturn => writeln!(b, "Ok(())"),
                        BranchKind::Block => writeln!(b),
                        BranchKind::Loop(label) | BranchKind::Branch(label) => {
                            writeln!(b, "break {label};")
                        }
                    };
                    return;
                } else if result_count == 1 {
                    kind.write_start(b);
                    let _ = write!(b, "{}", pop_value(validator, 0));
                } else {
                    for i in 0..result_count {
                        let _ = writeln!(
                            b,
                            "let _r{} = {};",
                            result_count - i - 1,
                            pop_value(validator, i),
                        );
                    }

                    kind.write_start(b);
                    let _ = write!(b, "(");
                    for i in 0..result_count {
                        if i > 0 {
                            let _ = write!(b, ", ");
                        }

                        let _ = write!(b, "_r{i}");
                    }
                    let _ = write!(b, ")");
                }

                let _ = match kind {
                    BranchKind::ExplicitReturn => writeln!(b, ");"),
                    BranchKind::ImplicitReturn => writeln!(b, ")"),
                    BranchKind::Block => writeln!(b),
                    BranchKind::Loop(_) | BranchKind::Branch(_) => {
                        writeln!(b, ";")
                    }
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
                Operator::Unreachable => {
                    let in_block = validator.control_stack_height() > 1;
                    if in_block {
                        let _ = write!(b, "return ");
                    }

                    let _ = write!(b, "::core::result::Result::Err(<RT as {RT_CRATE_PATH}::trap::Trap>::trap(&self._rt, {RT_CRATE_PATH}::trap::TrapCode::Unreachable))");

                    let _ = if in_block {
                        writeln!(b, ";")
                    } else {
                        writeln!(b)
                    };
                }
                Operator::Nop | Operator::Drop => (),
                Operator::Block { blockty } => {
                    let _ = write_block_start(
                        &mut b,
                        Label(validator.control_stack_height() + 1),
                        validator.operand_stack_height(),
                        blockty,
                    );

                    let _ = writeln!(b, "{{");
                }
                Operator::Loop { blockty } => {
                    let inputs = write_block_start(
                        &mut b,
                        Label(validator.control_stack_height() + 1),
                        validator.operand_stack_height(),
                        blockty,
                    );

                    let _ = writeln!(b, "loop {{");
                    inputs.write(&mut b);
                }
                Operator::If { blockty } => {
                    let _ = write_block_start(
                        &mut b,
                        Label(validator.control_stack_height() + 1),
                        validator.operand_stack_height() - 1,
                        blockty,
                    );

                    let _ = writeln!(b, "{{ if {} != 0i32 {{", pop_value(validator, 0));
                }
                Operator::Else => {
                    let result_count = get_block_type(types, &current_frame.block_type)
                        .1
                        .len()
                        .try_into()
                        .expect("too many block results");

                    write_control_flow(validator, &mut b, BranchKind::Block, result_count);
                    let _ = writeln!(b, "}} else {{");
                }
                Operator::End => {
                    if validator.control_stack_height() > 1 {
                        let result_count = get_block_type(types, &current_frame.block_type)
                            .1
                            .len()
                            .try_into()
                            .expect("too many block results");

                        // Generate code to write to result variables
                        if !current_frame.unreachable {
                            write_control_flow(
                                validator,
                                &mut b,
                                if current_frame.kind != wasmparser::FrameKind::Loop {
                                    BranchKind::Block
                                } else {
                                    BranchKind::Loop(Label(validator.control_stack_height()))
                                },
                                result_count,
                            );
                        }

                        let _ = write!(b, "}}");

                        // Extra brackets needed to end `if`/`else`
                        if matches!(
                            current_frame.kind,
                            wasmparser::FrameKind::Else | wasmparser::FrameKind::If
                        ) {
                            let _ = write!(b, "}}");
                        }

                        let _ = if result_count > 0 {
                            writeln!(b, ";")
                        } else {
                            writeln!(b)
                        };
                    } else if !current_frame.unreachable {
                        write_control_flow(
                            validator,
                            &mut b,
                            BranchKind::ImplicitReturn,
                            func_result_count,
                        );
                    }
                }
                Operator::Br { relative_depth } => {
                    if let Some(frame) = validator.get_control_frame(relative_depth as usize) {
                        // `validator` will handle bad labels
                        let (block_parameters, block_results) =
                            get_block_type(types, &frame.block_type);

                        let label = Label(validator.control_stack_height() - relative_depth);
                        if frame.kind == wasmparser::FrameKind::Loop {
                            let operands_start =
                                u32::try_from(frame.height).expect("operand stack too high");

                            for i in 0..u32::try_from(block_parameters.len()).unwrap() {
                                let operand = StackValue(operands_start + i);
                                let _ = writeln!(b, "_b{}{operand} = {operand};", label.0);
                            }

                            let _ = writeln!(b, "continue {label};");
                        } else {
                            write_control_flow(
                                validator,
                                &mut b,
                                BranchKind::Branch(label),
                                block_results
                                    .len()
                                    .try_into()
                                    .expect("too many types for branch"),
                            );
                        }
                    }
                }
                Operator::Return => write_control_flow(
                    validator,
                    &mut b,
                    if validator.control_stack_height() == 1 {
                        BranchKind::ImplicitReturn
                    } else {
                        BranchKind::ExplicitReturn
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
                Operator::LocalSet { local_index } => {
                    let _ = writeln!(
                        &mut b,
                        "{} = {};",
                        LocalVar(local_index),
                        pop_value(validator, 0)
                    );
                }
                Operator::I32Load { memarg } => {
                    let address = pop_value(validator, 0);
                    let _ = writeln!(
                        &mut b,
                        "let {} = {RT_CRATE_PATH}::memory::i32_load::<{}, {}, _, _>(&self.{}, {}i32.wrapping_add({address}), &self._rt)?;",
                        StackValue(validator.operand_stack_height() - 1),
                        memarg.align,
                        memarg.memory,
                        MemId(memarg.memory),
                        memarg.offset
                    );
                }
                Operator::I32Store { memarg } => {
                    let to_store = pop_value(validator, 0);
                    let address = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "{RT_CRATE_PATH}::memory::i32_store::<{}, {}, _, _>(&self.{}, {}i32.wrapping_add({address}), {to_store}, &self._rt)?;",
                        memarg.align,
                        memarg.memory,
                        MemId(memarg.memory),
                        memarg.offset
                    );
                }
                Operator::MemorySize { mem, mem_byte: _ } => {
                    let _ = writeln!(
                        &mut b,
                        "let {}: i32 = {RT_CRATE_PATH}::memory::size(&self.{});",
                        StackValue(validator.operand_stack_height()),
                        MemId(mem),
                    );
                }
                Operator::MemoryGrow { mem, mem_byte: _ } => {
                    let operand = pop_value(validator, 0);
                    let _ = writeln!(
                        &mut b,
                        "let {operand:#}: i32 = {RT_CRATE_PATH}::memory::grow(&self.{}, {operand});",
                        MemId(mem),
                    );
                }
                Operator::I32Const { value } => {
                    let _ = writeln!(
                        &mut b,
                        "let {} = {value}i32;",
                        StackValue(validator.operand_stack_height()),
                    );
                }
                Operator::I64Const { value } => {
                    let _ = writeln!(
                        &mut b,
                        "let {} = {value}i64;",
                        StackValue(validator.operand_stack_height()),
                    );
                }
                Operator::I32Eqz | Operator::I64Eqz => {
                    let result_value = pop_value(validator, 0);
                    let _ = writeln!(
                        &mut b,
                        "let {:#} = ({} == 0) as i32;",
                        result_value, result_value
                    );
                }
                Operator::I32Eq | Operator::I64Eq | Operator::F32Eq | Operator::F64Eq => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = ({result_value} == {}) as i32;",
                        pop_value(validator, 0)
                    );
                }
                Operator::I32Ne | Operator::I64Ne | Operator::F32Ne | Operator::F64Ne => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = ({result_value} != {}) as i32;",
                        pop_value(validator, 0)
                    );
                }
                Operator::I32LtS | Operator::I64LtS => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(&mut b, "let {c_1:#} = ({c_1} < {c_2}) as i32;");
                }
                Operator::I32LtU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = (({c_1} as u32) < ({c_2} as u32)) as i32;"
                    );
                }
                Operator::I64LtU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = (({c_1} as u32) < ({c_2} as u32)) as i32;"
                    );
                }
                Operator::I32Add => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = i32::wrapping_add({result_value}, {});",
                        pop_value(validator, 0)
                    );
                }
                Operator::I32Sub => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = i32::wrapping_sub({result_value}, {});",
                        pop_value(validator, 0)
                    );
                }
                Operator::I32Mul => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = i32::wrapping_mul({result_value}, {});",
                        pop_value(validator, 0)
                    );
                }
                Operator::I32DivS => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i32_div_s({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I32DivU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i32_div_u({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I32RemS => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i32_rem_s({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I32RemU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i32_rem_u({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I32Shl => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(&mut b, "let {c_1:#} = {c_1} << ({c_2} % 32);");
                }
                Operator::I32ShrS => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(&mut b, "let {c_1:#} = {c_1} >> ({c_2} % 32);");
                }
                Operator::I32ShrU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = (({c_1} as u32) >> ({c_2} % 32)) as i32;"
                    );
                }
                Operator::I64Add => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = i64::wrapping_add({result_value}, {});",
                        pop_value(validator, 0)
                    );
                }
                Operator::I64Sub => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = i64::wrapping_sub({result_value}, {});",
                        pop_value(validator, 0)
                    );
                }
                Operator::I64Mul => {
                    let result_value = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {result_value:#} = i64::wrapping_mul({result_value}, {});",
                        pop_value(validator, 0)
                    );
                }
                Operator::I64DivS => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i64_div_s({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I64DivU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i64_div_u({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I64RemS => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i64_rem_s({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I64RemU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = {RT_CRATE_PATH}::math::i64_rem_u({c_1}, {c_2}, &self._rt)?;",
                    );
                }
                Operator::I64Shl => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(&mut b, "let {c_1:#} = {c_1} << ({c_2} % 64);");
                }
                Operator::I64ShrS => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(&mut b, "let {c_1:#} = {c_1} >> ({c_2} % 64);");
                }
                Operator::I64ShrU => {
                    let c_2 = pop_value(validator, 0);
                    let c_1 = pop_value(validator, 1);
                    let _ = writeln!(
                        &mut b,
                        "let {c_1:#} = (({c_1} as u64) >> ({c_2} % 64)) as i64;"
                    );
                }
                Operator::I32WrapI64 => {
                    let popped = pop_value(validator, 0);
                    let _ = writeln!(&mut b, "let {popped:#} = {popped} as i32;");
                }
                Operator::I64ExtendI32S => {
                    let popped = pop_value(validator, 0);
                    let _ = writeln!(&mut b, "let {popped:#} = ({popped} as i32) as i64;");
                }
                Operator::I64ExtendI32U => {
                    let popped = pop_value(validator, 0);
                    let _ = writeln!(
                        &mut b,
                        "let {popped:#} = (({popped} as u32) as u64) as i64;",
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

        let _ = write!(buf, "pub fn {}(&self, ", crate::rust::SafeIdent::from(name),);

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
    /// [`Write`]: std::io::Write
    pub fn compile_from_buffer(
        &self,
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

        {
            let (validation_result, parse_result) =
                rayon::join(validate_payloads, parse_definitions);

            (functions, types) = validation_result?;
            definitions = parse_result?;
        }

        // Generate function bodies
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

        // Generate function exports, no conflict since export names are unique in WebAssembly.
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

        writeln!(output, "/* automatically generated by wasm2rs */")?;

        // Some branches might not be taken, and functions may not be called
        writeln!(
            output,
            "#[allow(unused_labels)]\n#[allow(unreachable_code)]\n#[allow(dead_code)]"
        )?;

        // Names might be mangled
        writeln!(output, "#[allow(non_snake_case)]")?;

        writeln!(output, "pub mod {} {{", self.generated_module_name)?;

        // TODO: Type parameter for imports
        write!(
            output,
            "#[derive(Debug)]\n#[non_exhaustive]\npub struct Instance<RT = StdRuntime> where RT: Runtime,"
        )?;

        // TODO: Should exported tables/globals/memories be exposed as public fields
        // TODO: Insert globals in struct as public fields
        writeln!(output, "{{")?;
        writeln!(output, "_rt: RT,")?;

        for i in 0..types.memory_count() {
            writeln!(output, "{}: RT::Memory{i},", MemId(i))?;
        }

        writeln!(output, "}}")?;

        // Generate `limits` module
        writeln!(output, "pub mod limits {{")?;

        for i in 0..types.memory_count() {
            let mem_type = types.memory_at(i);
            debug_assert!(!mem_type.memory64);

            writeln!(
                output,
                "#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]\n#[repr(u32)]"
            )?;
            writeln!(
                output,
                "pub enum MinMemoryPages{i} {{ Value = {}u32, }}",
                mem_type.initial
            )?;
            writeln!(
                output,
                "#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]\n#[repr(u32)]"
            )?;
            writeln!(
                output,
                "pub enum MaxMemoryPages{i} {{ Value = {}u32, }}",
                mem_type.maximum.unwrap_or(u32::MAX.into())
            )?;
        }

        writeln!(output, "}}")?;

        // Generate `Runtime` trait
        writeln!(output, "pub trait Runtime: {RT_CRATE_PATH}::trap::Trap {{")?;

        for i in 0..types.memory_count() {
            writeln!(output, "type Memory{i}: {RT_CRATE_PATH}::memory::Memory32;")?;
            writeln!(
                output,
                "fn initialize{}(&self, minimum: self::limits::MinMemoryPages{i}, maximum: self::limits::MaxMemoryPages{i}) -> ::core::result::Result<Self::Memory{i}, <Self as {RT_CRATE_PATH}::trap::Trap>::Repr>;",
                MemId(i))?;
        }

        writeln!(output, "}}")?;

        // Generate `StdRuntime` struct
        writeln!(
            output,
            "#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]\npub struct StdRuntime<TR: {RT_CRATE_PATH}::trap::Trap = {RT_CRATE_PATH}::trap::ReturnOnTrap>"
        )?;
        writeln!(output, "{{ _trap: TR }}")?;

        writeln!(
            output,
            "impl<TR: {RT_CRATE_PATH}::trap::Trap> StdRuntime<TR> {{\n pub fn new(trap: TR) -> Self {{\n  Self {{ _trap: trap }}\n }}\n}}")?;

        writeln!(output, "impl<TR: {RT_CRATE_PATH}::trap::Trap> {RT_CRATE_PATH}::trap::Trap for StdRuntime<TR> {{")?;
        writeln!(output, " type Repr = TR::Repr;")?;
        write!(output, " #[cold]\n #[inline(never)]\n fn trap(&self, code: {RT_CRATE_PATH}::trap::TrapCode) -> Self::Repr {{ ")?;
        writeln!(output, " TR::trap(&self._trap, code)\n}}")?;
        writeln!(output, "}}")?;

        writeln!(
            output,
            "impl<TR: {RT_CRATE_PATH}::trap::Trap> Runtime for StdRuntime<TR> {{"
        )?;

        // TODO: Runtime library needs a default memory impl that defaults to array in no_std+no alloc platforms.
        for i in 0..types.memory_count() {
            writeln!(
                output,
                "type Memory{i} = {RT_CRATE_PATH}::memory::HeapMemory32;"
            )?;
            writeln!(
                output,
                "fn initialize{}(&self, minimum: self::limits::MinMemoryPages{i}, maximum: self::limits::MaxMemoryPages{i}) -> ::core::result::Result<Self::Memory{i}, TR::Repr> {{", MemId(i))?;
            writeln!(output, "{RT_CRATE_PATH}::memory::HeapMemory32::with_limits(minimum as u32, maximum as u32).map_err(|error| <Self as {RT_CRATE_PATH}::trap::Trap>::trap(self, {RT_CRATE_PATH}::trap::TrapCode::MemoryInstantiation {{ memory: {i}, error }}))")?;
            writeln!(output, "}}")?;
        }

        writeln!(output, "}}")?;

        writeln!(output, "impl Instance {{")?;

        writeln!(output, "pub fn instantiate() -> ::core::result::Result<Self, {RT_CRATE_PATH}::trap::TrapValue> {{")?;
        writeln!(output, "Self::instantiate_with(Default::default())")?;
        writeln!(output, "}}")?;

        writeln!(output, "}}")?;

        writeln!(output, "impl<RT: Runtime> Instance<RT> {{")?;

        // TODO: output.write_vectored(bufs)?;
        for buf in function_decls {
            output.write_all(&buf)?;
        }

        // TODO: Parameter for imports
        writeln!(output, "pub fn instantiate_with(runtime: RT) -> ::core::result::Result<Self, <RT as {RT_CRATE_PATH}::trap::Trap>::Repr> {{")?;
        // TODO: Call start function
        writeln!(output, "Ok(Self {{")?;

        for i in 0..types.memory_count() {
            let mem = MemId(i);
            writeln!(output, "{mem}: runtime.initialize{mem}(self::limits::MinMemoryPages{i}::Value, self::limits::MaxMemoryPages{i}::Value)?,",)?;
        }

        writeln!(output, "_rt: runtime,\n}})\n}}")?;

        // Write exports
        for buf in function_exports {
            output.write_all(&buf)?;
        }

        writeln!(output, "}}\n}}")?;
        output.flush()?;
        Ok(())
    }
}
