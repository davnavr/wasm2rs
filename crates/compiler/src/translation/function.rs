use crate::translation::display::{LocalId, ValType};
use anyhow::Context;
use std::fmt::Write;

pub(in crate::translation) const TRAP_TRAIT: &str = "embedder::rt::trap::Trap";
pub(in crate::translation) const TRAP_CODE: &str = "embedder::rt::trap::TrapCode";

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

    out.write_str(") -> embedder::Result<");
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

#[must_use]
struct LocalStackSpace {
    space: u32,
}

impl LocalStackSpace {
    fn allocate(&mut self, ty: wasmparser::ValType) {
        use wasmparser::ValType;

        self.space = self.space.saturating_add(match ty {
            ValType::I32 | ValType::F32 => 4,
            ValType::I64 | ValType::F64 => (self.space % 8).saturating_add(8),
            ValType::V128 => (self.space % 16).saturating_add(16),
            // Assumed that references require at least 16 bytes, but are pointer aligned
            ValType::Ref(_) => (self.space % 8).saturating_add(16),
        });
    }

    fn finish(self, operand_stack_count: u32) -> u32 {
        // Assumed that some space in the stack will be reused, and that some stack operands will
        // be stored in registers, so arbitrary multiplier is picked here.
        self.space
            .saturating_add(operand_stack_count.saturating_mul(2))
    }
}

fn write_local_variables(
    out: &mut crate::buffer::Writer<'_>,
    validator: &mut Validator,
    mut locals_reader: wasmparser::LocalsReader<'_>,
    param_count: u32,
    local_stack_space: &mut LocalStackSpace,
) -> crate::Result<()> {
    let local_group_count = locals_reader.get_count();
    let mut local = LocalId(param_count);
    for _ in 0..local_group_count {
        use wasmparser::ValType;

        let (count, ty) = locals_reader.read()?;
        validator.define_locals(locals_reader.original_position(), count, ty)?;

        local_stack_space.allocate(ty);

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

#[derive(Default)]
struct OperandTypes {
    types: Vec<Option<wasmparser::ValType>>,
}

impl OperandTypes {
    fn access(&mut self, stack_value: StackValue) -> &mut Option<wasmparser::ValType> {
        let target = stack_value.0 as usize;
        self.types.resize(target + 1, None);
        &mut self.types[target]
    }

    fn assign_many(&mut self, start_height: u32, actual_types: &[wasmparser::ValType]) -> bool {
        let start = start_height as usize;
        if let Some(expected_types) = self.types.get(start..start + actual_types.len()) {
            expected_types
                .iter()
                .zip(actual_types)
                .any(|(e, a)| *e != Some(*a))
        } else {
            true
        }
    }
}

#[derive(Clone, Copy)]
enum StackValueAssignment {
    Reused(StackValue),
    Redefined {
        value: StackValue,
        value_type: wasmparser::ValType,
    },
    Underflow,
}

impl StackValueAssignment {
    fn from_stack_value_with_type(
        types: &mut OperandTypes,
        value: StackValue,
        value_type: wasmparser::ValType,
        assigned: bool,
    ) -> Self {
        match types.access(value) {
            Some(previous_type) if value_type == *previous_type => Self::Reused(value),
            destination => {
                if assigned {
                    *destination = Some(value_type);
                }

                Self::Redefined { value, value_type }
            }
        }
    }

    fn from_stack_value(
        types: &mut OperandTypes,
        value: StackValue,
        value_type: Option<wasmparser::ValType>,
        assigned: bool,
    ) -> Self {
        match value_type {
            Some(value_type) => {
                Self::from_stack_value_with_type(types, value, value_type, assigned)
            }
            None => Self::Underflow,
        }
    }

    fn from_popped_value(types: &mut OperandTypes, value: PoppedValue, assigned: bool) -> Self {
        match value {
            PoppedValue::Pop { value, value_type } => {
                Self::from_stack_value_with_type(types, value, value_type, assigned)
            }
            PoppedValue::Underflow => Self::Underflow,
        }
    }

    fn from_popped_value_with_type(
        types: &mut OperandTypes,
        value: PoppedValue,
        value_type: wasmparser::ValType,
        assigned: bool,
    ) -> Self {
        match value {
            PoppedValue::Pop {
                value,
                value_type: _,
            } => Self::from_stack_value_with_type(types, value, value_type, assigned),
            PoppedValue::Underflow => Self::Underflow,
        }
    }
}

impl std::fmt::Display for StackValueAssignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reused(value) => write!(f, "{value} = "),
            Self::Redefined { value, value_type } => write!(f, "let mut {value}: {value_type} = "),
            Self::Underflow => f.write_str(
                "let ::core::compile_error!(\"code generation bug, attempted to assign below operand stack height\") = "
            ),
        }
    }
}

#[derive(Clone, Copy)]
enum PoppedValue {
    Pop {
        value: StackValue,
        value_type: wasmparser::ValType,
    },
    Underflow,
}

impl PoppedValue {
    fn pop(validator: &Validator, depth: u32) -> Self {
        match validator.get_operand_type(depth as usize) {
            Some(Some(value_type)) => {
                // TODO: Basic copying only good for numtype and vectype, have to call Runtime::clone for funcref + externref
                let height = validator.operand_stack_height() - depth - 1;
                PoppedValue::Pop {
                    value: StackValue(height),
                    value_type,
                }
            }
            Some(None) => todo!("case for unreachable value"),
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
            Self::Pop { value, .. } => std::fmt::Display::fmt(&value, f),
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
        popped_first: u32,
        operand_types: &mut OperandTypes,
    ) {
        operand_types.types.fill(None);

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
            let _ = write!(out, "{}", PoppedValue::pop(validator, popped_first));
        } else {
            // TODO: Don't generate temporaries for result values.
            for i in 0..result_count {
                let _ = writeln!(
                    out,
                    "let _r{} = {};",
                    result_count - i - 1,
                    PoppedValue::pop(validator, popped_first + i)
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

fn write_multiple_assignment(
    out: &mut crate::buffer::Writer<'_>,
    types: &[wasmparser::ValType],
    operand_height: u32,
    operand_types: &mut OperandTypes,
) {
    if !types.is_empty() {
        let is_reassignment = operand_types.assign_many(operand_height, types);
        if is_reassignment {
            out.write_str("let ");
        }

        if types.len() > 1 {
            out.write_str("(");
        }

        for i in 0..u32::try_from(types.len()).unwrap_or(u32::MAX) {
            if i > 0 {
                out.write_str(", ");
            }

            let _ = write!(out, "{}", StackValue(i + operand_height));
        }

        if types.len() > 1 {
            out.write_str(")");
        }

        if is_reassignment {
            out.write_str(": ");

            if types.len() > 1 {
                out.write_str("(");
            }

            for (i, ty) in types.iter().enumerate() {
                if i > 0 {
                    out.write_str(", ");
                }

                let _ = write!(out, "{}", ValType(*ty));
            }

            if types.len() > 1 {
                out.write_str(")");
            }
        }

        out.write_str(" = ");
    }
}

/// Writes the start of a block.
///
/// The `operand_height` is the depth of the first result value pushed onto the stack at the
/// end of the block.
fn write_block_start<'a>(
    out: &mut crate::buffer::Writer<'_>,
    types: &'a wasmparser::types::Types,
    label: Label,
    operand_height: u32,
    operand_types: &mut OperandTypes,
    block_ty: &'a wasmparser::BlockType,
    is_loop: bool,
) -> (&'a [wasmparser::ValType], u32) {
    let (argument_types, result_types) = get_block_type(types, block_ty);
    let argument_count = u32::try_from(argument_types.len()).unwrap_or(u32::MAX);
    let result_start_height = operand_height - argument_count;

    if is_loop {
        for i in 0..argument_count {
            let operand = StackValue(result_start_height + i);
            let _ = writeln!(out, "let mut _b_{}{operand} = {operand};", label.0);
        }
    }

    write_multiple_assignment(out, result_types, result_start_height, operand_types);

    let _ = write!(out, "{label}: ");

    (argument_types, result_start_height)
}

mod paths {
    pub(super) const MEMORY: &str = "embedder::rt::memory";
}

macro_rules! access_structs {
    ($($name:ident($id:path) | $checker:ident;)*) => {$(
        struct $name {
            id: $id,
            imported: bool,
        }

        impl $name {
            fn new(index: u32, import_counts: &crate::translation::ImportCounts) -> Self {
                Self {
                    id: $id(index),
                    imported: import_counts.$checker(index),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                if !self.imported {
                    f.write_str("&")?;
                }

                write!(f, "self.{}", self.id)?;

                if self.imported {
                    f.write_str("()")?;
                }

                Ok(())
            }
        }
    )*};
}

access_structs! {
    MemAccess(crate::translation::display::MemId) | is_memory_import;
}

enum Signedness {
    Signed,
    Unsigned,
}

fn write_i8_load(
    out: &mut crate::buffer::Writer<'_>,
    validator: &mut Validator,
    memarg: &wasmparser::MemArg,
    signed: Signedness,
    destination: ValType,
    operand_types: &mut OperandTypes,
    import_counts: &crate::translation::ImportCounts,
) {
    let address = PoppedValue::pop(validator, 0);
    let _ = write!(
        out,
        "{}{}::i8_load::<{}, {}, _, _>({}, {address}, &self.embedder)?",
        StackValueAssignment::from_stack_value_with_type(
            operand_types,
            StackValue(validator.operand_stack_height() - 1),
            destination.0,
            true,
        ),
        paths::MEMORY,
        memarg.offset,
        memarg.memory,
        MemAccess::new(memarg.memory, import_counts),
    );

    if let Signedness::Unsigned = signed {
        out.write_str(" as u8");
    }

    let _ = writeln!(out, " as {destination};");
}

fn write_i16_load(
    out: &mut crate::buffer::Writer<'_>,
    validator: &mut Validator,
    memarg: &wasmparser::MemArg,
    signed: Signedness,
    destination: ValType,
    operand_types: &mut OperandTypes,
    import_counts: &crate::translation::ImportCounts,
) {
    let address = PoppedValue::pop(validator, 0);
    let _ = write!(
        out,
        "{}{}::i16_load::<{}, {}, {}, _, _>({}, {address}, &self.embedder)?",
        StackValueAssignment::from_stack_value_with_type(
            operand_types,
            StackValue(validator.operand_stack_height() - 1),
            destination.0,
            true,
        ),
        paths::MEMORY,
        memarg.offset,
        memarg.align,
        memarg.memory,
        MemAccess::new(memarg.memory, import_counts),
    );

    if let Signedness::Unsigned = signed {
        out.write_str(" as u16");
    }

    let _ = writeln!(out, " as {destination};");
}

/// Generates a Rust statement corresponding to a WebAssembly branch instruction.
///
/// The `relative_depth` is the [WebAssembly label] that specifies the target block to jump to.
///
/// The `popped_before_branch` parameter indicates how many operands are popped from the stack
/// before the operands corresponding the target block's result types (or block's input types in
/// the case of a `loop`) are popped.
///
/// [WebAssembly label]: https://webassembly.github.io/spec/core/syntax/instructions.html#control-instructions
fn write_branch(
    out: &mut crate::buffer::Writer,
    validator: &Validator,
    relative_depth: u32,
    popped_before_branch: u32,
    types: &wasmparser::types::Types,
    operand_types: &mut OperandTypes,
) -> crate::Result<()> {
    if let Some(frame) = validator.get_control_frame(relative_depth as usize) {
        // `validator` will handle bad labels
        let (block_parameters, block_results) = get_block_type(types, &frame.block_type);

        let label = Label(validator.control_stack_height() - relative_depth);
        if frame.kind == wasmparser::FrameKind::Loop {
            let operands_start =
                u32::try_from(frame.height).with_context(|| "operand stack too high")?;

            let block_params = u32::try_from(block_parameters.len())
                .with_context(|| "block has too many parameters")?;

            for i in 0..block_params {
                let _ = writeln!(
                    out,
                    "_b_{}{} = {};",
                    label.0,
                    StackValue(operands_start + (block_params - i - 1)),
                    PoppedValue::pop(validator, popped_before_branch + i),
                );
            }

            let _ = writeln!(out, "continue {label};");

            return Ok(());
        } else if validator.control_stack_height() > 1 || relative_depth == 0 {
            let target = if validator.control_stack_height() == 1 {
                if popped_before_branch == 0 {
                    BranchKind::ImplicitReturn
                } else {
                    // For `br_if``
                    BranchKind::ExplicitReturn
                }
            } else {
                BranchKind::Branch(label)
            };

            target.write_control_flow(
                out,
                validator,
                block_results.len().try_into().unwrap(),
                popped_before_branch,
                operand_types,
            );

            return Ok(());
        }
    }

    let _ = writeln!(
        out,
        "::core::unimplemented!(\"code generation bug, bad branch target {} where control stack \
        was {} frames high\");",
        relative_depth,
        validator.control_stack_height(),
    );

    Ok(())
}

/// Generates a [Rust function] definition corresponding to a [WebAssembly function body].
///
/// [Rust function]: https://doc.rust-lang.org/reference/items/functions.html
/// [WebAssembly function body]: https://webassembly.github.io/spec/core/syntax/modules.html#syntax-func
pub(in crate::translation) fn write_definition(
    out: &mut crate::buffer::Writer<'_>,
    validator: &mut Validator,
    body: &wasmparser::FunctionBody,
    types: &wasmparser::types::Types, // TODO: Remove types parameter, see if validator by itself can be used
    import_counts: &crate::translation::ImportCounts,
    emit_stack_overflow_checks: bool,
) -> crate::Result<()> {
    let func_type =
        wasmparser::WasmModuleResources::type_of_function(validator.resources(), validator.index())
            .expect("could not get function type");

    let func_result_count =
        u32::try_from(func_type.results().len()).with_context(|| "too many results in function")?;

    let _ = write!(
        out,
        "\n    fn {}",
        crate::translation::display::FuncId(validator.index())
    );
    write_definition_signature(out, func_type);
    out.write_str(" {\n");

    // TODO: Make a crate::buffer::IndentedWriter or something

    if emit_stack_overflow_checks {
        let _ = writeln!(out,
            "      embedder::rt::stack::check_for_overflow(Self::STACK_FRAME_SIZE_{}, &self.embedder)?;\n",
            validator.index()
        );
    }

    let mut local_stack_space = LocalStackSpace { space: 0 };

    for ty in func_type.params() {
        local_stack_space.allocate(*ty);
    }

    write_local_variables(
        out,
        validator,
        body.get_locals_reader()?,
        u32::try_from(func_type.params().len()).unwrap_or(u32::MAX),
        &mut local_stack_space,
    )?;

    let mut operand_types = OperandTypes::default();

    let mut operators = body.get_operators_reader()?;
    let mut max_operand_stack_size = 0u32;
    while !operators.eof() {
        use wasmparser::Operator;

        let (op, op_offset) = operators.read_with_offset()?;

        let current_frame = validator
            .get_control_frame(0)
            .with_context(|| "control frame stack was unexpectedly empty")?;

        if current_frame.unreachable && !matches!(op, Operator::End | Operator::Else) {
            // Although code is unreachable, WASM spec still requires it to be validated
            validator.op(op_offset, &op)?;
            // Don't generate Rust code
            continue;
        }

        const MEMORY: &str = paths::MEMORY;
        const MATH: &str = "embedder::rt::math";

        match op {
            Operator::Unreachable => {
                let in_block = validator.control_stack_height() > 1;
                if in_block {
                    out.write_str("return ");
                }

                let _ = write!(
                    out,
                    "::core::result::Result::Err(embedder::rt::trap::unreachable(&self.embedder))"
                );

                if in_block {
                    out.write_str(";\n");
                } else {
                    out.write_str("\n");
                }
            }
            Operator::Nop => (),
            Operator::Block { blockty } => {
                write_block_start(
                    out,
                    types,
                    Label(validator.control_stack_height() + 1),
                    validator.operand_stack_height(),
                    &mut operand_types,
                    &blockty,
                    false,
                );

                let _ = writeln!(out, "{{");
            }
            Operator::Loop { blockty } => {
                let label = validator.control_stack_height() + 1;
                let (input_types, result_start_height) = write_block_start(
                    out,
                    types,
                    Label(label),
                    validator.operand_stack_height(),
                    &mut operand_types,
                    &blockty,
                    true,
                );

                let _ = writeln!(out, "loop {{");

                let result_indices = 0..u32::try_from(input_types.len()).unwrap_or(u32::MAX);
                for (ty, i) in input_types.iter().zip(result_indices) {
                    let operand = StackValue(i + result_start_height);
                    let _ = writeln!(
                        out,
                        "{}_b_{}{operand};",
                        StackValueAssignment::from_stack_value_with_type(
                            &mut operand_types,
                            operand,
                            *ty,
                            true
                        ),
                        label
                    );
                }
            }
            Operator::If { blockty } => {
                write_block_start(
                    out,
                    types,
                    Label(validator.control_stack_height() + 1),
                    validator.operand_stack_height() - 1,
                    &mut operand_types,
                    &blockty,
                    false,
                );

                let _ = writeln!(out, "{{ if {} != 0i32 {{", PoppedValue::pop(validator, 0));
            }
            Operator::Else => {
                let result_count = get_block_type(types, &current_frame.block_type)
                    .1
                    .len()
                    .try_into()
                    .with_context(|| "too many block results")?;

                BranchKind::Block.write_control_flow(
                    out,
                    validator,
                    result_count,
                    0,
                    &mut operand_types,
                );
                let _ = writeln!(out, "}} else {{");
            }
            Operator::End => {
                if validator.control_stack_height() > 1 {
                    let result_count = get_block_type(types, &current_frame.block_type)
                        .1
                        .len()
                        .try_into()
                        .with_context(|| "too many block results")?;

                    // Generate code to write to result variables
                    if !current_frame.unreachable {
                        let kind = if current_frame.kind != wasmparser::FrameKind::Loop {
                            BranchKind::Block
                        } else {
                            BranchKind::Loop(Label(validator.control_stack_height()))
                        };

                        kind.write_control_flow(
                            out,
                            validator,
                            result_count,
                            0,
                            &mut operand_types,
                        );
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
                    }

                    out.write_str("\n");
                } else if !current_frame.unreachable {
                    BranchKind::ImplicitReturn.write_control_flow(
                        out,
                        validator,
                        func_result_count,
                        0,
                        &mut operand_types,
                    );
                }
            }
            Operator::Br { relative_depth } => {
                write_branch(out, validator, relative_depth, 0, types, &mut operand_types)?;
            }
            Operator::BrIf { relative_depth } => {
                let cond = PoppedValue::pop(validator, 0);
                let _ = write!(out, "if {cond} != 0i32 {{\n  ");
                write_branch(out, validator, relative_depth, 1, types, &mut operand_types)?;
                out.write_str("} // br_if\n");
            }
            Operator::BrTable { ref targets } => {
                if !targets.is_empty() {
                    let i = PoppedValue::pop(validator, 0);

                    let _ = writeln!(out, "match {i} {{");

                    for (cond, result) in targets.targets().enumerate() {
                        let label = result?;
                        let _ = write!(out, "  {cond} => {{\n    ");
                        write_branch(out, validator, label, 1, types, &mut operand_types)?;
                        out.write_str("  }\n");
                    }

                    out.write_str("  _ => {\n    ");
                    write_branch(
                        out,
                        validator,
                        targets.default(),
                        1,
                        types,
                        &mut operand_types,
                    )?;
                    out.write_str("  }\n}\n");
                } else {
                    write_branch(
                        out,
                        validator,
                        targets.default(),
                        1,
                        types,
                        &mut operand_types,
                    )?;
                }
            }
            Operator::Return => {
                let kind = if validator.control_stack_height() == 1 {
                    BranchKind::ImplicitReturn
                } else {
                    BranchKind::ExplicitReturn
                };

                kind.write_control_flow(out, validator, func_result_count, 0, &mut operand_types);
            }
            Operator::Call { function_index } => {
                let signature = wasmparser::WasmModuleResources::type_of_function(
                    validator.resources(),
                    function_index,
                )
                .expect("could not get callee type");

                let param_count = u32::try_from(signature.params().len()).unwrap_or(u32::MAX);

                // Writes the results, the first (the leftmost) result is the one that needs to be popped last.
                let result_start_height = validator.operand_stack_height() - param_count;
                write_multiple_assignment(
                    out,
                    signature.results(),
                    result_start_height,
                    &mut operand_types,
                );

                let _ = write!(
                    out,
                    "self.{}(",
                    crate::translation::display::FuncId(function_index)
                );

                // Writes the parameters, the first (the leftmost) parameter is popped last.
                for depth in (0..param_count).rev() {
                    if depth < param_count - 1 {
                        out.write_str(", ");
                    }

                    let _ = write!(out, "{}", PoppedValue::pop(validator, depth));
                }

                out.write_str(")?;\n");
            }
            // Operator::CallIndirect { type_index, table_index, table_byte } => { todo!() }
            Operator::Drop => {
                // TODO: Should `drop` call ::core::mem::drop() for FuncRef/ExternRef?
                let _ = writeln!(
                    out,
                    "// ::core::mem::drop({});",
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::Select | Operator::TypedSelect { ty: _ } => {
                let cond = PoppedValue::pop(validator, 0);
                let val_2 = PoppedValue::pop(validator, 1);
                let val_1 = PoppedValue::pop(validator, 2);
                let _ = writeln!(
                    out,
                    "{}if {cond} != 0i32 {{ {val_1} }} else {{ {val_2} }};",
                    StackValueAssignment::from_popped_value(&mut operand_types, val_1, true)
                );
            }
            Operator::LocalGet { local_index } => {
                let _ = writeln!(
                    out,
                    "{}{};",
                    StackValueAssignment::from_stack_value(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height()),
                        validator.get_local_type(local_index),
                        true
                    ),
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
            Operator::LocalTee { local_index } => {
                let arg = PoppedValue::pop(validator, 0);

                // TODO: For `local.set` and `local.tee`, how will `funcref`/`externref` be copied?
                let _ = write!(out, "{} = {arg};\nlet {arg:#}", LocalId(local_index));

                if let Some(ty) = validator.get_local_type(local_index) {
                    let _ = write!(out, ": {}", ValType(ty));
                }

                let _ = writeln!(out, " = {arg};");
            }
            Operator::GlobalGet { global_index } => {
                let _ = write!(
                    out,
                    "{}",
                    StackValueAssignment::from_stack_value(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height()),
                        wasmparser::WasmModuleResources::global_at(
                            validator.resources(),
                            global_index
                        )
                        .map(|g| g.content_type),
                        true
                    ),
                );

                let id = crate::translation::display::GlobalId(global_index);
                let global_type = types.global_at(global_index);
                let is_imported = import_counts.is_global_import(global_index);
                if !global_type.mutable {
                    if is_imported {
                        out.write_str("*");
                    }

                    // TODO: How to clone global value for non-Copy types?
                    let _ = write!(out, "self.{id}");

                    if is_imported {
                        out.write_str("()");
                    }
                } else {
                    out.write_str("embedder::rt::global::Global::get(");

                    if !is_imported {
                        out.write_str("&");
                    }

                    let _ = write!(out, "self.{id}");

                    if is_imported {
                        out.write_str("()");
                    }

                    out.write_str(")");
                }

                out.write_str(";\n");
            }
            Operator::GlobalSet { global_index } => {
                let new_value = PoppedValue::pop(validator, 0);

                out.write_str("embedder::rt::global::Global::set(");

                if !import_counts.is_global_import(global_index) {
                    out.write_str("&");
                }

                let _ = write!(
                    out,
                    "self.{}",
                    crate::translation::display::GlobalId(global_index)
                );

                if import_counts.is_global_import(global_index) {
                    out.write_str("()");
                }

                let _ = write!(out, ", {new_value})");

                out.write_str(";\n");
            }
            Operator::I32Load { memarg } => {
                let address = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{MEMORY}::i32_load::<{}, {}, {}, _, _>({}, {address}, &self.embedder)?;",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height() - 1),
                        wasmparser::ValType::I32,
                        true
                    ),
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I64Load { memarg } => {
                let address = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{MEMORY}::i64_load::<{}, {}, {}, _, _>({}, {address}, &self.embedder)?;",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height() - 1),
                        wasmparser::ValType::I64,
                        true
                    ),
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::F32Load { memarg } => {
                let address = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}f32::from_bits({MEMORY}::i32_load::<{}, {}, {}, _, _>({}, {address}, &self.embedder)? as u32);",
                    StackValueAssignment::from_stack_value_with_type(&mut operand_types, StackValue(validator.operand_stack_height() - 1), wasmparser::ValType::F32, true),
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::F64Load { memarg } => {
                let address = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}f64::from_bits({MEMORY}::i64_load::<{}, {}, {}, _, _>({}, {address}, &self.embedder)? as u64);",
                    StackValueAssignment::from_stack_value_with_type(&mut operand_types, StackValue(validator.operand_stack_height() - 1), wasmparser::ValType::F64, true),
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I32Load8S { memarg } => {
                write_i8_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Signed,
                    ValType::I32,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I32Load8U { memarg } => {
                write_i8_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Unsigned,
                    ValType::I32,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I32Load16S { memarg } => {
                write_i16_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Signed,
                    ValType::I32,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I32Load16U { memarg } => {
                write_i16_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Unsigned,
                    ValType::I32,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I64Load8S { memarg } => {
                write_i8_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Signed,
                    ValType::I64,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I64Load8U { memarg } => {
                write_i8_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Unsigned,
                    ValType::I64,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I64Load16S { memarg } => {
                write_i16_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Signed,
                    ValType::I64,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I64Load16U { memarg } => {
                write_i16_load(
                    out,
                    validator,
                    &memarg,
                    Signedness::Unsigned,
                    ValType::I64,
                    &mut operand_types,
                    import_counts,
                );
            }
            Operator::I64Load32S { memarg } => {
                let address = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{MEMORY}::i32_load::<{}, {}, {}, _, _>({}, {address}, &self.embedder)? as i64;",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types, StackValue(validator.operand_stack_height() - 1), wasmparser::ValType::I64, true),
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I64Load32U { memarg } => {
                let address = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{MEMORY}::i32_load::<{}, {}, {}, _, _>({}, {address}, &self.embedder)? as u32 as i64;",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types, StackValue(validator.operand_stack_height() - 1), wasmparser::ValType::I64, true),
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I32Store { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i32_store::<{}, {}, {}, _, _>({}, {address}, {to_store}, &self.embedder)?;",
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I64Store { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i64_store::<{}, {}, {}, _, _>({}, {address}, {to_store}, &self.embedder)?;",
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::F32Store { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i32_store::<{}, {}, {}, _, _>({}, {address}, {to_store}.to_bits() as i32, &self.embedder)?;",
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::F64Store { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i64_store::<{}, {}, {}, _, _>({}, {address}, {to_store}.to_bits() as i64, &self.embedder)?;",
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I32Store8 { memarg } | Operator::I64Store8 { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i8_store::<{}, {}, _, _>({}, {address}, {to_store} as i8, &self.embedder)?;",
                    memarg.offset,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I32Store16 { memarg } | Operator::I64Store16 { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i16_store::<{}, {}, {}, _, _>({}, {address}, {to_store} as i16, &self.embedder)?;",
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::I64Store32 { memarg } => {
                let to_store = PoppedValue::pop(validator, 0);
                let address = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{MEMORY}::i32_store::<{}, {}, {}, _, _>({}, {address}, {to_store} as i32, &self.embedder)?;",
                    memarg.offset,
                    memarg.align,
                    memarg.memory,
                    MemAccess::new(memarg.memory, import_counts),
                );
            }
            Operator::MemorySize { mem, mem_byte: _ } => {
                let _ = writeln!(
                    out,
                    "{}{MEMORY}::size({});",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height()),
                        wasmparser::ValType::I32,
                        true
                    ),
                    MemAccess::new(mem, import_counts),
                );
            }
            Operator::MemoryGrow { mem, mem_byte: _ } => {
                let operand = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{MEMORY}::grow({}, {operand});",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        operand,
                        wasmparser::ValType::I32,
                        true
                    ),
                    MemAccess::new(mem, import_counts),
                );
            }
            //Operator::MemoryFill
            Operator::MemoryCopy { dst_mem, src_mem } => {
                let length = PoppedValue::pop(validator, 0);
                let src_addr = PoppedValue::pop(validator, 1);
                let dst_addr = PoppedValue::pop(validator, 2);
                let dst = MemAccess::new(dst_mem, import_counts);
                let src = MemAccess::new(src_mem, import_counts);
                if dst_mem == src_mem {
                    let _ = writeln!(out,
                        "{}::copy_within::<{src_mem}, _, _>({src}, {dst_addr}, {src_addr}, {length}, &self.embedder)?;",
                        paths::MEMORY);
                } else {
                    let _ = writeln!(out,
                        "{}::copy::<{dst_mem}, {src_mem}, _, _, _>({dst}, {src}, {dst_addr}, {src_addr}, {length}, &self.embedder)?;",
                        paths::MEMORY);
                }
            }
            Operator::MemoryInit { data_index, mem } => {
                let length = PoppedValue::pop(validator, 0);
                let data_offset = PoppedValue::pop(validator, 1);
                let mem_offset = PoppedValue::pop(validator, 2);
                let _ = writeln!(
                    out,
                    "{}::init::<{mem}, _, _>({}, {}, {mem_offset}, {data_offset}, {length}, &self.embedder)?;",
                    paths::MEMORY,
                    MemAccess::new(mem, import_counts),
                    crate::translation::display::DataId(data_index),
                );
            }
            Operator::DataDrop { data_index } => {
                let _ = writeln!(out, "// data.drop {data_index}");
            }
            Operator::I32Const { value } => {
                let _ = writeln!(
                    out,
                    "{}{value}i32;",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height()),
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I64Const { value } => {
                let _ = writeln!(
                    out,
                    "{}{value}i64;",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height()),
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::F32Const { value } => {
                let _ = writeln!(
                    out,
                    "{}f32::from_bits({:#010X}u32);",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height()),
                        wasmparser::ValType::F32,
                        true
                    ),
                    value.bits(),
                );
            }
            Operator::F64Const { value } => {
                let _ = writeln!(
                    out,
                    "{}f64::from_bits({:#018X}u64);",
                    StackValueAssignment::from_stack_value_with_type(
                        &mut operand_types,
                        StackValue(validator.operand_stack_height()),
                        wasmparser::ValType::F64,
                        true
                    ),
                    value.bits(),
                );
            }
            Operator::I32Eqz | Operator::I64Eqz => {
                let result_value = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({} == 0) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I32,
                        true
                    ),
                    result_value
                );
            }
            Operator::I32Eq | Operator::I64Eq | Operator::F32Eq | Operator::F64Eq => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}({result_value} == {}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I32,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32Ne | Operator::I64Ne | Operator::F32Ne | Operator::F64Ne => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}({result_value} != {}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I32,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32LtS | Operator::I64LtS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}({c_1} < {c_2}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32LtU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u32) < ({c_2} as u32)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32GtS | Operator::I64GtS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}({c_1} > {c_2}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32GtU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u32) > ({c_2} as u32)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32LeS | Operator::I64LeS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}({c_1} <= {c_2}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32LeU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u32) <= ({c_2} as u32)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32GeS | Operator::I64GeS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}({c_1} >= {c_2}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32GeU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u32) > ({c_2} as u32)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I64LtU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u64) < ({c_2} as u64)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I64GtU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u64) > ({c_2} as u64)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I64LeU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u64) <= ({c_2} as u64)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I64GeU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u64) >= ({c_2} as u64)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::F32Gt | Operator::F64Gt => {
                // TODO: See if Rust's implementation of float comparison follows WebAssembly.
                let z_2 = PoppedValue::pop(validator, 0);
                let z_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}({z_1} > {z_2}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        z_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32Clz => {
                let c = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}i32::leading_zeros({c}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32Ctz => {
                let c = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}i32::trailing_zeros({c}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32Add => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}i32::wrapping_add({result_value}, {});",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I32,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32Sub => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}i32::wrapping_sub({result_value}, {});",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I32,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32Mul => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}i32::wrapping_mul({result_value}, {});",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I32,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I32DivS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i32_div_s({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32DivU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i32_div_u({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32RemS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i32_rem_s({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32RemU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i32_rem_u({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32And | Operator::I64And => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1} & {c_2};",
                    StackValueAssignment::from_popped_value(&mut operand_types, c_1, true),
                );
            }
            Operator::I32Or | Operator::I64Or => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1} | {c_2};",
                    StackValueAssignment::from_popped_value(&mut operand_types, c_1, true),
                );
            }
            Operator::I32Xor | Operator::I64Xor => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1} ^ {c_2};",
                    StackValueAssignment::from_popped_value(&mut operand_types, c_1, true),
                );
            }
            Operator::I32Shl => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1} << ({c_2} as u32 % 32);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32ShrS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1} >> ({c_2} as u32 % 32);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    )
                );
            }
            Operator::I32ShrU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u32) >> ({c_2} as u32 % 32)) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32Rotl => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1}.rotate_left(({c_2} % 32) as u32);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32Rotr => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1}.rotate_right(({c_2} % 32) as u32);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I64Clz => {
                let c = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}(i64::leading_zeros({c}) as i32) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Ctz => {
                let c = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}(i64::trailing_zeros({c}) as i32) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Popcnt => {
                let i = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}i64::count_ones({i}) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        i,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Add => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}i64::wrapping_add({result_value}, {});",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I64,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I64Sub => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}i64::wrapping_sub({result_value}, {});",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I64,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I64Mul => {
                let result_value = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}i64::wrapping_mul({result_value}, {});",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        result_value,
                        wasmparser::ValType::I64,
                        true
                    ),
                    PoppedValue::pop(validator, 0)
                );
            }
            Operator::I64DivS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i64_div_s({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64DivU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i64_div_u({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64RemS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i64_rem_s({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64RemU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{MATH}::i64_rem_u({c_1}, {c_2}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Shl => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1} << ({c_2} as u64 % 64);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64ShrS => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1} >> ({c_2} as u64 % 64);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64ShrU => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}(({c_1} as u64) >> ({c_2} as u64 % 64)) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Rotl => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1}.rotate_left(({c_2} % 64) as u32);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Rotr => {
                let c_2 = PoppedValue::pop(validator, 0);
                let c_1 = PoppedValue::pop(validator, 1);
                let _ = writeln!(
                    out,
                    "{}{c_1}.rotate_right(({c_2} % 64) as u32);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        c_1,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::F32Neg | Operator::F64Neg => {
                // `::core::ops::Neg` on `f32` and `f64` do the same operation in Rust.
                let z = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}-z;",
                    StackValueAssignment::from_popped_value(&mut operand_types, z, true),
                );
            }
            Operator::I32WrapI64 => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{popped} as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32TruncF32S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i32_trunc_f32_s({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32TruncF32U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i32_trunc_f32_u({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32TruncF64S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i32_trunc_f64_s({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32TruncF64U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i32_trunc_f64_u({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I64ExtendI32S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as i32) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64ExtendI32U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}(({popped} as u32) as u64) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64TruncF32S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i64_trunc_f32_s({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64TruncF32U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i64_trunc_f32_u({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64TruncF64S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i64_trunc_f64_s({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64TruncF64U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}embedder::rt::math::i64_trunc_f64_u({popped}, &self.embedder)?;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            // - Rust uses "roundTiesToEven".
            // - WebAssembly specifies round-to-nearest ties-to-even.
            //
            // Are they the same?
            //
            // Rust: https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast
            // WASM: https://webassembly.github.io/spec/core/exec/numerics.html#rounding
            Operator::F32ConvertI32S | Operator::F32ConvertI64S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{popped} as f32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F32,
                        true
                    ),
                );
            }
            Operator::F32ConvertI32U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as u32) as f32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F32,
                        true
                    ),
                );
            }
            Operator::F32ConvertI64U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as u64) as f32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F32,
                        true
                    ),
                );
            }
            Operator::F32DemoteF64 => {
                // TODO: Does Rust's conversion of `f64` to `f32` preserve the "canonical NaN"
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "// f32.demote_f64\n{}{popped} as f32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F32,
                        true
                    ),
                );
            }
            Operator::F64ConvertI32S | Operator::F64ConvertI64S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{popped} as f64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F64,
                        true
                    ),
                );
            }
            Operator::F64ConvertI32U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as u32) as f64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F64,
                        true
                    ),
                );
            }
            Operator::F64ConvertI64U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as u64) as f64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F64,
                        true
                    ),
                );
            }
            // TODO: Does Rust's conversion of `f32` to `f64` preserve the "canonical NaN"
            Operator::F64PromoteF32 => {
                // See https://webassembly.github.io/spec/core/exec/numerics.html#op-promote
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "// f64.promote_f32\n{}{popped} as f64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F64,
                        true
                    ),
                );
            }
            Operator::I32ReinterpretF32 => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}f32::to_bits({popped}) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I64ReinterpretF64 => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}f64::to_bits({popped}) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::F32ReinterpretI32 => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}f32::from_bits({popped} as u32);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F32,
                        true
                    ),
                );
            }
            Operator::F64ReinterpretI64 => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}f64::from_bits({popped} as u64);",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::F64,
                        true
                    ),
                );
            }
            Operator::I32Extend8S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as i8) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32Extend16S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as i16) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I64Extend8S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as i8) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Extend16S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as i16) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64Extend32S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as i32) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            // Float-to-integer saturation operations translate exactly to Rust casts.
            Operator::I32TruncSatF32S | Operator::I32TruncSatF64S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{popped} as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I32TruncSatF32U | Operator::I32TruncSatF64U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as u32) as i32;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I32,
                        true
                    ),
                );
            }
            Operator::I64TruncSatF32S | Operator::I64TruncSatF64S => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}{popped} as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            Operator::I64TruncSatF32U | Operator::I64TruncSatF64U => {
                let popped = PoppedValue::pop(validator, 0);
                let _ = writeln!(
                    out,
                    "{}({popped} as u64) as i64;",
                    StackValueAssignment::from_popped_value_with_type(
                        &mut operand_types,
                        popped,
                        wasmparser::ValType::I64,
                        true
                    ),
                );
            }
            _ => anyhow::bail!("translation of operation is not yet supported: {op:?}"),
        }

        validator.op(op_offset, &op)?;
        max_operand_stack_size = validator.operand_stack_height().max(max_operand_stack_size);
    }

    // Implicit return generated when last `end` is handled.
    validator.finish(operators.original_position())?;

    out.write_str("    }\n");

    if emit_stack_overflow_checks {
        let _ = writeln!(
            out,
            "\n    const STACK_FRAME_SIZE_{}: usize = {};\n",
            validator.index(),
            local_stack_space.finish(max_operand_stack_size)
        );
    }

    Ok(())
}
