//! Prints Rust source code corresponding to the [`ast`](crate::ast).

mod expression;
mod statements;

pub(crate) use statements::print_statements;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IndentationKind {
    Omitted,
    Spaces(std::num::NonZeroU8),
    Tab,
}

/// Specifies how the generated Rust source code is indented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct Indentation(IndentationKind);

impl Indentation {
    /// Indentation with the given number of spaces.
    pub const fn spaces(amount: u8) -> Self {
        Self(if let Some(amount) = std::num::NonZeroU8::new(amount) {
            IndentationKind::Spaces(amount)
        } else {
            IndentationKind::Omitted
        })
    }

    /// No indentation is emitted in generated Rust source code.
    pub const OMITTED: Self = Self(IndentationKind::Omitted);

    /// Indentation with four spaces, typical for Rust source code.
    pub const STANDARD: Self = Self::spaces(4);

    /// Indentation with a signle tab (`\t`).
    pub const TAB: Self = Self(IndentationKind::Tab);

    pub(crate) fn to_str(self) -> &'static str {
        match self.0 {
            IndentationKind::Omitted => "",
            IndentationKind::Tab => "\t",
            IndentationKind::Spaces(amount) => {
                const SPACES: &str = match std::str::from_utf8(&[b' '; 255]) {
                    Ok(s) => s,
                    Err(_) => panic!("spaces should be valid UTF-8"),
                };

                &SPACES[..amount.get() as usize]
            }
        }
    }
}

impl Default for Indentation {
    fn default() -> Self {
        Self::STANDARD
    }
}

fn print_indentation(
    out: &mut dyn crate::write::Write,
    indentation: Indentation,
    indent_level: u32,
) {
    for _ in 0..indent_level {
        out.write_str(indentation.to_str());
    }
}

/// Rust paths to embedder or runtime support code, typically implemented in `wasm2rs-rt`.
mod paths {
    //pub(super) const TRAP: &str = "embedder::Trap";
    pub(super) const RT_FUNC_REF: &str = "embedder::rt::func_ref";
    pub(super) const RT_MATH: &str = "embedder::rt::math";
    pub(super) const RT_TRAP: &str = "embedder::rt::trap::Trap";
    pub(super) const RT_MEM: &str = "embedder::rt::memory";
    pub(super) const RT_TABLE: &str = "embedder::rt::table";
}

impl std::fmt::Display for crate::ast::RefType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Extern => f.write_str("embedder::ExternRef"),
            Self::Func => f.write_str("embedder::rt::func_ref::FuncRef<'static, embedder::Trap>"),
        }
    }
}

impl std::fmt::Display for crate::ast::ValType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32 => f.write_str("i32"),
            Self::I64 => f.write_str("i64"),
            Self::F32 => f.write_str("f32"),
            Self::F64 => f.write_str("f64"),
            Self::Ref(ref_ty) => std::fmt::Display::fmt(ref_ty, f),
        }
    }
}

fn print_null_ref(out: &mut dyn crate::write::Write, ref_ty: crate::ast::RefType) {
    use crate::ast::RefType;

    match ref_ty {
        RefType::Extern => {
            out.write_str("<embedder::ExternRef as embedder::table::NullableTableElement>::NULL")
        }
        RefType::Func => {
            out.write_str("embedder::rt::func_ref::FuncRef::<'static, embedder::Trap>::NULL")
        }
    }
}

impl crate::ast::Literal {
    pub(crate) fn print(&self, out: &mut dyn crate::write::Write) {
        match self {
            Self::I32(i) if *i <= 9 => write!(out, "{i}i32"),
            Self::I32(i) if *i <= 0xFFFF => write!(out, "{i:#X}i32"),
            Self::I32(i) => write!(out, "{i:#010X}i32"),
            Self::I64(i) if *i <= 9 => write!(out, "{i}i64"),
            Self::I64(i) if *i <= 0xFFFF => write!(out, "{i:#X}i64"),
            Self::I64(i) => write!(out, "{i:#018X}i64"),
            Self::F32(z) => write!(out, "f32::from_bits({z:#010X})"),
            Self::F64(z) => write!(out, "f64::from_bits({z:#018X})"),
            Self::RefNull(ref_ty) => print_null_ref(out, *ref_ty),
            Self::RefFunc(func) => {
                write!(
                    out,
                    "embedder::rt::table::NullableTableElement::clone_from_cell(&self.{func})"
                );
            }
        }
    }
}

pub(crate) struct Context<'ctx, 'wasm> {
    pub(crate) wasm: &'ctx crate::context::Context<'wasm>,
    pub(crate) arena: &'ctx crate::ast::Arena,
    pub(crate) debug_info: crate::DebugInfo,
}

impl crate::ast::ExprId {
    pub(crate) fn print(
        self,
        out: &mut dyn crate::write::Write,
        nested: bool,
        context: &Context,
        function: Option<crate::ast::FuncId>,
    ) {
        context
            .arena
            .get(self)
            .print(out, nested, context, function)
    }

    fn print_bool(
        self,
        out: &mut dyn crate::write::Write,
        context: &Context,
        function: Option<crate::ast::FuncId>,
    ) {
        context.arena.get(self).print_bool(out, context, function)
    }
}

impl crate::ast::ExprListId {
    fn print(
        self,
        out: &mut dyn crate::write::Write,
        enclosed: bool,
        context: &Context,
        function: Option<crate::ast::FuncId>,
    ) {
        if enclosed {
            out.write_str("(");
        }

        for (i, expr) in context.arena.get_list(self).iter().enumerate() {
            if i > 0 {
                out.write_str(", ");
            }

            expr.print(out, false, context, function);
        }

        if enclosed {
            out.write_str(")");
        }
    }
}

fn print_call_common<F>(
    out: &mut dyn crate::write::Write,
    callee: crate::ast::FuncId,
    context: &crate::context::Context,
    arguments: F,
) where
    F: FnOnce(&mut dyn crate::write::Write),
{
    use crate::context::CallKind;

    match context.function_attributes.call_kind(callee) {
        CallKind::Function => out.write_str("Self::"),
        CallKind::Method => out.write_str("self."),
    }

    write!(out, "{}(", context.function_ident(callee));
    arguments(out);
    out.write_str(")");
}

#[derive(Clone, Copy)]
struct FrameRef {
    maker: crate::ast::MakeFrame,
    instruction_offset: u32,
}

impl FrameRef {
    fn try_new(
        function: Option<crate::ast::FuncId>,
        instruction_offset: u32,
        debug_info: crate::DebugInfo,
    ) -> Option<FrameRef> {
        match function {
            Some(id) if debug_info != crate::DebugInfo::Omit => Some(FrameRef {
                maker: crate::ast::MakeFrame(id),
                instruction_offset,
            }),
            _ => None,
        }
    }
}

impl std::fmt::Display for FrameRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ const F: embedder::rt::trace::WasmFrame = Module::{}({}); &F }}",
            self.maker, self.instruction_offset,
        )
    }
}

fn print_call_expr(
    out: &mut dyn crate::write::Write,
    callee: crate::ast::FuncId,
    arguments: crate::ast::ExprListId,
    context: &Context,
    caller: Option<crate::ast::FuncId>,
    code_offset: u32,
) {
    print_call_common(out, callee, context.wasm, |out| {
        for (i, arg) in context.arena.get_list(arguments).iter().enumerate() {
            if i > 0 {
                out.write_str(", ");
            }

            arg.print(out, false, context, caller);
        }
    });

    if context
        .wasm
        .function_attributes
        .unwind_kind(callee)
        .can_unwind()
    {
        if let Some(frame) = FrameRef::try_new(caller, code_offset, context.debug_info) {
            write!(out, ".unwind_with({frame})");
        }

        out.write_str("?");
    }
}

fn print_trap_with(
    out: &mut dyn crate::write::Write,
    function: Option<crate::ast::FuncId>,
    instruction_offset: u32,
    debug_info: crate::DebugInfo,
) {
    if let Some(frame) = FrameRef::try_new(function, instruction_offset, debug_info) {
        write!(out, ".trap_with(Some({frame}))");
    }
}

fn print_frame(
    out: &mut dyn crate::write::Write,
    function: Option<crate::ast::FuncId>,
    instruction_offset: u32,
    debug_info: crate::DebugInfo,
) {
    match FrameRef::try_new(function, instruction_offset, debug_info) {
        Some(frame) => write!(out, "Some({frame})"),
        None => out.write_str("None"),
    }
}

fn print_table(
    out: &mut dyn crate::write::Write,
    table: crate::ast::TableId,
    context: &crate::context::Context,
) {
    let table_ident = context.table_ident(table);

    if matches!(table_ident, crate::context::TableIdent::Id(_)) {
        out.write_str("&");
    }

    write!(out, "self.{table_ident}");
}

/// Prints a Rust expression evaluating to something that can be passed to functions expecting a
/// parameter implementing `wasm2rs_rt::memory::Memory`.
fn print_memory(
    out: &mut dyn crate::write::Write,
    memory: crate::ast::MemoryId,
    context: &crate::context::Context,
) {
    let memory_ident = context.memory_ident(memory);
    if matches!(memory_ident, crate::context::MemoryIdent::Id(_)) {
        out.write_str("&");
    }

    write!(out, "self.{memory_ident}");
}

impl crate::ast::Expr {
    pub(crate) fn print(
        &self,
        out: &mut dyn crate::write::Write,
        nested: bool,
        context: &Context,
        function: Option<crate::ast::FuncId>,
    ) {
        expression::print_expression(self, out, nested, context, function)
    }

    /// Emits Rust code evaluating to a `bool` expression.
    ///
    /// This is used when translating WebAssembly comparison instructions.
    fn print_bool(
        &self,
        out: &mut dyn crate::write::Write,
        context: &Context,
        function: Option<crate::ast::FuncId>,
    ) {
        use crate::ast::BinOp;

        macro_rules! comparison {
            ($c_1:ident $operator:literal $c_2:ident) => {{
                $c_1.print(out, false, context, function);
                out.write_str(concat!(" ", $operator, " "));
                $c_2.print(out, false, context, function);
            }};
        }

        match self {
            Self::UnaryOperator {
                kind: crate::ast::UnOp::IxxEqz,
                c_1,
            } => {
                c_1.print(out, false, context, function);
                out.write_str(" == 0")
            }
            Self::BinaryOperator {
                kind: BinOp::Eq,
                c_1,
                c_2,
            } => comparison!(c_1 "==" c_2),
            Self::BinaryOperator {
                kind: BinOp::Ne,
                c_1,
                c_2,
            } => comparison!(c_1 "!=" c_2),
            // TODO: Write more simplified comparisons
            Self::RefIsNull(reference) => {
                reference.print(out, false, context, function);
                out.write_str(" == embedder::rt::table::NullableTableElement::NULL");
            }
            _ => {
                self.print(out, true, context, function);
                out.write_str(" != 0")
            }
        }
    }
}

pub(crate) fn print_stub(
    out: &mut dyn crate::write::Write,
    function: crate::ast::FuncId,
    context: &crate::context::Context,
    indentation: Indentation,
    indent_level: u32,
    arguments: u32,
) {
    print_indentation(out, indentation, indent_level);
    let returns_result = context
        .function_attributes
        .unwind_kind(function)
        .can_unwind();

    if !returns_result {
        out.write_str("Ok(");
    }

    print_call_common(out, function, context, |out| {
        for i in 0u32..arguments {
            if i > 0 {
                out.write_str(", ");
            }

            write!(out, "{}", crate::ast::LocalId(i));
        }
    });

    if !returns_result {
        out.write_str(")");
    }

    out.write_str("\n");
}
