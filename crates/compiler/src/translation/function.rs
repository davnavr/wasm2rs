use crate::translation::display::{FuncId, LocalId, MemId, ValType};
use std::fmt::Write;

pub(in crate::translation) fn write_definition_signature(
    out: &mut crate::buffer::Writer<'_>,
    sig: &wasmparser::FuncType,
) {
    out.write_str("(&self");

    // Write the parameter types
    for (i, ty) in sig.params().iter().enumerate() {
        let _ = write!(
            out,
            ", mut {}: {}",
            LocalId(u32::try_from(i).expect("too many parameters")),
            ValType(*ty)
        );
    }

    let _ = write!(out, ") -> {}::Result<", crate::translation::EMBEDDER_PATH);
    let results = sig.results();

    if results.len() != 1 {
        out.write_str("(");
    }

    // Write the result types
    for (i, ty) in results.iter().enumerate() {
        if i > 0 {
            out.write_str(", ");
        }

        let _ = write!(out, "{}", ValType(*ty));
    }

    if results.len() != 1 {
        out.write_str(")");
    }

    out.write_str(">");
}

type Validator = wasmparser::FuncValidator<wasmparser::ValidatorResources>;

fn write_local_variables(
    out: &mut crate::buffer::Writer<'_>,
    validator: &mut Validator,
    mut locals_reader: wasmparser::LocalsReader<'_>,
    param_count: u32,
) -> wasmparser::Result<()> {
    let local_group_count = locals_reader.get_count();
    let mut local = LocalId(param_count);
    for _ in 0..local_group_count {
        use wasmparser::ValType;

        let (count, ty) = locals_reader.read()?;
        validator.define_locals(locals_reader.original_position(), count, ty)?;

        let default_value = match ty {
            ValType::I32 | ValType::I64 => "0",
            ValType::F32 | ValType::F64 => "0.0",
            _ => "::core::todo!(\"embedder must provide cloning for references\")",
        };

        for _ in 0..count {
            let _ = writeln!(out, "let mut {}: {} = {default_value};", local, ValType(ty),);

            local.0 += 1;
        }
    }

    Ok(())
}

pub(in crate::translation) fn get_function_type(ty: &wasmparser::SubType) -> &wasmparser::FuncType {
    match ty {
        wasmparser::SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: wasmparser::CompositeType::Func(sig),
        } => sig,
        _ => unimplemented!("expected function type, but got unsupported type: {ty:?}"),
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
            let func_type = get_function_type(
                types
                    .get(types.core_type_at(*sig).unwrap_sub())
                    .expect("bad type id"),
            );

            (func_type.params(), func_type.results())
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct StackValue(u32);

impl std::fmt::Display for StackValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_s_{}", self.0)
    }
}

#[derive(Clone, Copy)]
enum PoppedValue {
    Pop(StackValue),
    Underflow,
}

impl PoppedValue {
    fn pop(validator: &Validator, depth: u32) -> Self {
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
    }
}

impl std::fmt::Display for PoppedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pop(v) => std::fmt::Display::fmt(&v, f),
            Self::Underflow if f.alternate() => f.write_str("_"),
            Self::Underflow => f.write_str(
                "::core::unimplemented!(\"code generation bug, operand stack underflow occured\")",
            ),
        }
    }
}

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
    fn write_start(&self, out: &mut crate::buffer::Writer<'_>) {
        match self {
            Self::ExplicitReturn => {
                let _ = write!(out, "return Ok(");
            }
            Self::ImplicitReturn => {
                let _ = write!(out, "Ok(");
            }
            Self::Block => (),
            Self::Loop(label) | Self::Branch(label) => {
                let _ = write!(out, "break {label} ");
            }
        }
    }

    /// Writes a Rust `break` or `return` statement, or an expression for implicitly returning.
    ///
    /// The `result_count` is the number of values returned by the parent WebAssembly block.
    ///
    /// This is used when translating the `return`, `end`, and `br` instructions.
    fn write_control_flow(
        self,
        out: &mut crate::buffer::Writer<'_>,
        validator: &Validator,
        result_count: u32,
    ) {
        if result_count == 0u32 {
            match self {
                BranchKind::ExplicitReturn => {
                    let _ = writeln!(out, "return Ok(());");
                }
                BranchKind::ImplicitReturn => {
                    let _ = writeln!(out, "Ok(())");
                }
                BranchKind::Block => out.write_str("\n"),
                BranchKind::Loop(label) | BranchKind::Branch(label) => {
                    let _ = writeln!(out, "break {label};");
                }
            };
            return;
        } else if result_count == 1 {
            self.write_start(out);
            let _ = write!(out, "{}", PoppedValue::pop(validator, 0));
        } else {
            for i in 0..result_count {
                let _ = writeln!(
                    out,
                    "let _r{} = {};",
                    result_count - i - 1,
                    PoppedValue::pop(validator, i),
                );
            }

            self.write_start(out);
            out.write_str("(");
            for i in 0..result_count {
                if i > 0 {
                    out.write_str(", ");
                }

                let _ = write!(out, "_r{i}");
            }

            out.write_str(")");
        }

        match self {
            BranchKind::ExplicitReturn => {
                out.write_str(");\n");
            }
            BranchKind::ImplicitReturn => {
                out.write_str(")\n");
            }
            BranchKind::Block => out.write_str("\n"),
            BranchKind::Loop(_) | BranchKind::Branch(_) => {
                let _ = writeln!(out, ";");
            }
        };
    }
}

#[derive(Clone, Copy)]
struct Label(u32);

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'_l_{}", self.0)
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
    fn write(self, out: &mut crate::buffer::Writer<'_>) {
        for i in 0..self.count {
            let operand = StackValue(self.height + i);
            let _ = writeln!(out, "let mut _b_{}{operand} = {operand};", self.label.0);
        }
    }

    /// Writes the start of a block.
    ///
    /// The `operand_height` is the depth of the first result value pushed onto the stack at the
    /// end of the block.
    fn write_start(
        out: &mut crate::buffer::Writer<'_>,
        types: &wasmparser::types::Types,
        label: Label,
        operand_height: u32,
        block_ty: wasmparser::BlockType,
    ) -> Self {
        let (argument_types, result_types) = get_block_type(types, &block_ty);
        let argument_count = u32::try_from(argument_types.len()).unwrap();
        let result_count = u32::try_from(result_types.len()).unwrap();
        let result_start_height = operand_height - argument_count;

        if result_count > 0 {
            out.write_str("let ");

            if result_count > 1 {
                out.write_str("(");
            }

            for i in 0..result_count {
                if i > 0 {
                    out.write_str(", ");
                }

                let _ = write!(out, "{}", StackValue(i + result_start_height));
            }

            if result_count > 1 {
                out.write_str(")");
            }

            out.write_str(" = ");
        }

        let _ = write!(out, " {label}: ");

        BlockInputs {
            label,
            count: argument_count,
            height: result_start_height,
        }
    }
}

/// Generates a [Rust function] definition corresponding to a [WebAssembly function body].
///
/// [Rust function]: https://doc.rust-lang.org/reference/items/functions.html
/// [WebAssembly function body]: https://webassembly.github.io/spec/core/syntax/modules.html#syntax-func
pub(in crate::translation) fn write_definition(
    out: &mut crate::buffer::Writer<'_>,
    validator: &mut Validator,
    body: &wasmparser::FunctionBody,
    types: &wasmparser::types::Types,
) -> wasmparser::Result<()> {
    let func_type =
        wasmparser::WasmModuleResources::type_of_function(validator.resources(), validator.index())
            .expect("could not get function type");

    let func_result_count = u32::try_from(func_type.results().len()).unwrap();

    let _ = write!(out, "\n    fn {}", FuncId(validator.index()));
    write_definition_signature(out, func_type);
    out.write_str(" {\n");

    // TODO: Make a crate::buffer::IndentedWriter or something

    write_local_variables(
        out,
        validator,
        body.get_locals_reader()?,
        u32::try_from(func_type.params().len()).unwrap_or(u32::MAX),
    )?;

    let mut operators = body.get_operators_reader()?;
    while !operators.eof() {
        use wasmparser::Operator;

        let (op, op_offset) = operators.read_with_offset()?;

        let current_frame = validator
            .get_control_frame(0)
            .expect("control frame stack was unexpectedly empty");

        if current_frame.unreachable && !matches!(op, Operator::End | Operator::Else) {
            // Although code is unreachable, WASM spec still requires it to be validated
            validator.op(op_offset, &op)?;
            // Don't generate Rust code
            continue;
        }

        /// Paths to well-known types provided by runtime code.
        enum Paths {
            Embedder,
            Memory,
            Math,
            TrapTrait,
            TrapCode,
        }

        impl std::fmt::Display for Paths {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use crate::translation::EMBEDDER_PATH;
                match self {
                    Self::Embedder => write!(f, "{EMBEDDER_PATH}::State"),
                    Self::Memory => write!(f, "{EMBEDDER_PATH}::rt::memory"),
                    Self::Math => write!(f, "{EMBEDDER_PATH}::rt::math"),
                    Self::TrapTrait => write!(f, "{EMBEDDER_PATH}::rt::trap::Trap"),
                    Self::TrapCode => write!(f, "{EMBEDDER_PATH}::rt::trap::TrapCode"),
                }
            }
        }

        const EMBEDDER: Paths = Paths::Embedder;
        const MEMORY: Paths = Paths::Memory;
        const MATH: Paths = Paths::Math;
        const TRAP_TRAIT: Paths = Paths::TrapTrait;
        const TRAP_CODE: Paths = Paths::TrapCode;

        match op {
            Operator::Unreachable => {
                let in_block = validator.control_stack_height() > 1;
                if in_block {
                    out.write_str("return ");
                }

                let _ = write!(
                    out,
                    "::core::result::Result::Err(<{EMBEDDER} as {TRAP_TRAIT}>::trap(&self._embedder, {TRAP_CODE}::Unreachable))"
                );

                if in_block {
                    out.write_str(";\n");
                } else {
                    out.write_str("\n");
                }
            }
            Operator::Nop | Operator::Drop => (),
            Operator::Block { blockty } => {
                let _ = BlockInputs::write_start(
                    out,
                    types,
                    Label(validator.control_stack_height() + 1),
                    validator.operand_stack_height(),
                    blockty,
                );

                let _ = writeln!(out, "{{");
            }
            Operator::Loop { blockty } => {
                let inputs = BlockInputs::write_start(
                    out,
                    types,
                    Label(validator.control_stack_height() + 1),
                    validator.operand_stack_height(),
                    blockty,
                );

                let _ = writeln!(out, "loop {{");
                inputs.write(out);
            }
            Operator::If { blockty } => {
                let _ = BlockInputs::write_start(
                    out,
                    types,
                    Label(validator.control_stack_height() + 1),
                    validator.operand_stack_height() - 1,
                    blockty,
                );

                let _ = writeln!(out, "{{ if {} != 0i32 {{", PoppedValue::pop(validator, 0));
            }
            Operator::Else => {
                let result_count = get_block_type(types, &current_frame.block_type)
                    .1
                    .len()
                    .try_into()
                    .expect("too many block results");

                BranchKind::Block.write_control_flow(out, validator, result_count);
                let _ = writeln!(out, "}} else {{");
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
                        let kind = if current_frame.kind != wasmparser::FrameKind::Loop {
                            BranchKind::Block
                        } else {
                            BranchKind::Loop(Label(validator.control_stack_height()))
                        };

                        kind.write_control_flow(out, validator, result_count);
                    }

                    out.write_str("}");

                    // Extra brackets needed to end `if`/`else`
                    if matches!(
                        current_frame.kind,
                        wasmparser::FrameKind::Else | wasmparser::FrameKind::If
                    ) {
                        out.write_str("}");
                    }

                    if result_count > 0 {
                        out.write_str(";");
                    } else {
                        out.write_str("\n");
                    }
                } else if !current_frame.unreachable {
                    BranchKind::ImplicitReturn.write_control_flow(
                        out,
                        validator,
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
                            let _ = writeln!(out, "_b_{}{operand} = {operand};", label.0);
                        }

                        let _ = writeln!(out, "continue {label};");
                    } else {
                        BranchKind::Branch(label).write_control_flow(
                            out,
                            validator,
                            block_results.len().try_into().unwrap(),
                        );
                    }
                }
            }
            Operator::Return => {
                let kind = if validator.control_stack_height() == 1 {
                    BranchKind::ImplicitReturn
                } else {
                    BranchKind::ExplicitReturn
                };

                kind.write_control_flow(out, validator, func_result_count);
            }
            Operator::LocalGet { local_index } => {
                let _ = writeln!(
                    out,
                    "let {} = {};",
                    StackValue(validator.operand_stack_height()),
                    LocalId(local_index)
                );
            }
            Operator::LocalSet { local_index } => {
                let _ = writeln!(
                    out,
                    "{} = {};",
                    LocalId(local_index),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32Load { memarg } => {
                let address = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "let {} = {MEMORY}::i32_load::<{}, {}, _, _>(&self.{}, {}i32.wrapping_add({address}), &self._embedder)?;",
                    StackValue(validator.operand_stack_height() - 1),
                    memarg.align,
                    memarg.memory,
                    MemId(memarg.memory),
                    memarg.offset
                );
            }
            Operator::I32Store { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i32_store::<{}, {}, _, _>(&self.{}, {}i32.wrapping_add({address}), {to_store}, &self._embedder)?;",
                    memarg.align,
                    memarg.memory,
                    MemId(memarg.memory),
                    memarg.offset
                );
            }
            Operator::MemorySize { mem, mem_byte: _ } => {
                let _ = writeln!(
                    out,
                    "let {}: i32 = {MEMORY}::size(&self.{});",
                    StackValue(validator.operand_stack_height()),
                    MemId(mem),
                );
            }
            Operator::MemoryGrow { mem, mem_byte: _ } => {
                let operand = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "let {operand:#}: i32 = {MEMORY}::grow(&self.{}, {operand});",
                    MemId(mem),
                );
            }
            Operator::I32Const { value } => {
                let _ = writeln!(
                    out,
                    "let {} = {value}i32;",
                    StackValue(validator.operand_stack_height()),
                );
            }
            Operator::I64Const { value } => {
                let _ = writeln!(
                    out,
                    "let {} = {value}i64;",
                    StackValue(validator.operand_stack_height()),
                );
            }
            Operator::I32Eqz | Operator::I64Eqz => {
                let result_value = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "let {:#} = ({} == 0) as i32;",
                    result_value, result_value
                );
            }
            Operator::I32Eq | Operator::I64Eq | Operator::F32Eq | Operator::F64Eq => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = ({result_value} == {}) as i32;",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32Ne | Operator::I64Ne | Operator::F32Ne | Operator::F64Ne => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = ({result_value} != {}) as i32;",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32LtS | Operator::I64LtS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(out, "let {c_1:#} = ({c_1} < {c_2}) as i32;");
            }
            Operator::I32LtU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = (({c_1} as u32) < ({c_2} as u32)) as i32;"
                );
            }
            Operator::I64LtU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = (({c_1} as u32) < ({c_2} as u32)) as i32;"
                );
            }
            Operator::I32Add => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = i32::wrapping_add({result_value}, {});",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32Sub => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = i32::wrapping_sub({result_value}, {});",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32Mul => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = i32::wrapping_mul({result_value}, {});",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32DivS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i32_div_s({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I32DivU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i32_div_u({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I32RemS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i32_rem_s({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I32RemU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i32_rem_u({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I32Shl => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(out, "let {c_1:#} = {c_1} << ({c_2} % 32);");
            }
            Operator::I32ShrS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(out, "let {c_1:#} = {c_1} >> ({c_2} % 32);");
            }
            Operator::I32ShrU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = (({c_1} as u32) >> ({c_2} % 32)) as i32;"
                );
            }
            Operator::I64Add => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = i64::wrapping_add({result_value}, {});",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I64Sub => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = i64::wrapping_sub({result_value}, {});",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I64Mul => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {result_value:#} = i64::wrapping_mul({result_value}, {});",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I64DivS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i64_div_s({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I64DivU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i64_div_u({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I64RemS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i64_rem_s({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I64RemU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = {MATH}::i64_rem_u({c_1}, {c_2}, &self._embedder)?;",
                );
            }
            Operator::I64Shl => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(out, "let {c_1:#} = {c_1} << ({c_2} % 64);");
            }
            Operator::I64ShrS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(out, "let {c_1:#} = {c_1} >> ({c_2} % 64);");
            }
            Operator::I64ShrU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "let {c_1:#} = (({c_1} as u64) >> ({c_2} % 64)) as i64;"
                );
            }
            Operator::I32WrapI64 => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(out, "let {popped:#} = {popped} as i32;");
            }
            Operator::I64ExtendI32S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(out, "let {popped:#} = ({popped} as i32) as i64;");
            }
            Operator::I64ExtendI32U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(out, "let {popped:#} = (({popped} as u32) as u64) as i64;",);
            }
            _ => todo!("translate {op:?}"),
        }

        validator.op(op_offset, &op)?;
    }

    // Implicit return generated when last `end` is handled.
    validator.finish(operators.original_position())?;

    out.write_str("    }\n");
    Ok(())
}
