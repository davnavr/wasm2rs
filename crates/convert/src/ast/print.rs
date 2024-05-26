//! Prints Rust source code corresponding to the [`ast`](crate::ast).

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

/// Rust paths to embedder or runtime support code, typically implemented in `wasm2rs-rt`.
mod paths {
    //pub(super) const TRAP: &str = "embedder::Trap";
    pub(super) const RT_MATH: &str = "embedder::rt::math";
    pub(super) const RT_TRAP: &str = "embedder::rt::trap::Trap";
    pub(super) const RT_MEM: &str = "embedder::rt::memory";
}

impl std::fmt::Display for crate::ast::ValType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32 => f.write_str("i32"),
            Self::I64 => f.write_str("i64"),
            Self::F32 => f.write_str("f32"),
            Self::F64 => f.write_str("f64"),
        }
    }
}

impl std::fmt::Display for crate::ast::Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32(i) if *i <= 9 => write!(f, "{i}i32"),
            Self::I32(i) if *i <= 0xFFFF => write!(f, "{i:#X}i32"),
            Self::I32(i) => write!(f, "{i:#010X}i32"),
            Self::I64(i) if *i <= 9 => write!(f, "{i}i64"),
            Self::I64(i) if *i <= 0xFFFF => write!(f, "{i:#X}i64"),
            Self::I64(i) => write!(f, "{i:#018X}i64"),
            Self::F32(z) => write!(f, "f32::from_bits({z:#010X})"),
            Self::F64(z) => write!(f, "f64::from_bits({z:#018X})"),
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
            "{{ const F: embedder::rt::trace::WasmFrame = Instance::{}({}); &F }}",
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

impl crate::ast::Expr {
    pub(crate) fn print(
        &self,
        out: &mut dyn crate::write::Write,
        nested: bool,
        context: &Context,
        function: Option<crate::ast::FuncId>,
    ) {
        macro_rules! nested_expr {
            {$($stmt:stmt;)*} => {{
                if nested {
                    out.write_str("(");
                }

                $($stmt)*

                if nested {
                    out.write_str(")");
                }
            }};
        }

        match self {
            Self::Literal(literal) => write!(out, "{literal}"),
            Self::UnaryOperator { kind, c_1 } => {
                use crate::ast::UnOp;

                macro_rules! rt_math_function {
                    ($name:ident @ $offset:ident) => {{
                        out.write_str(paths::RT_MATH);
                        out.write_str(concat!("::", stringify!($name), "("));
                        c_1.print(out, true, context, function);
                        out.write_str(")");
                        print_trap_with(out, function, *$offset, context.debug_info);
                        out.write_str("?");
                    }};
                }

                macro_rules! simple_cast {
                    ($to:ident) => {
                        nested_expr! {
                            c_1.print(out, true, context, function);
                            out.write_str(concat!(" as ", stringify!($to)));
                        }
                    };
                }

                macro_rules! double_cast {
                    ($start:ident as $end:ident) => {
                        nested_expr! {
                            c_1.print(out, true, context, function);
                            out.write_str(concat!(
                                " as ",
                                stringify!($start),
                                " as ",
                                stringify!($end)
                            ));
                        }
                    };
                }

                match kind {
                    UnOp::IxxEqz => nested_expr! {
                        out.write_str("(");
                        c_1.print(out, false, context, function);
                        out.write_str(" == 0) as i32");
                    },
                    UnOp::I32Clz => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(".leading_zeros() as i32");
                    },
                    UnOp::I64Clz => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(".leading_zeros() as i64");
                    },
                    UnOp::I32Ctz => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(".trailing_zeros() as i32");
                    },
                    UnOp::I64Ctz => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(".trailing_zeros() as i64");
                    },
                    UnOp::I32Popcnt => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(".count_ones() as i32");
                    },
                    UnOp::I64Popcnt => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(".count_ones() as i64");
                    },
                    UnOp::FxxNeg => nested_expr! {
                        // `::core::ops::Neg` on `f32` and `f64` do the same operation in Rust.
                        out.write_str("-");
                        c_1.print(out, true, context, function);
                    },
                    UnOp::I32WrapI64 | UnOp::I32TruncSatFxxS => simple_cast!(i32),
                    UnOp::I32TruncF32S { offset } => rt_math_function!(i32_trunc_f32_s @ offset),
                    UnOp::I32TruncF32U { offset } => rt_math_function!(i32_trunc_f32_u @ offset),
                    UnOp::I32TruncF64S { offset } => rt_math_function!(i32_trunc_f64_s @ offset),
                    UnOp::I32TruncF64U { offset } => rt_math_function!(i32_trunc_f64_u @ offset),
                    UnOp::I64ExtendI32S | UnOp::I64TruncSatFxxS => simple_cast!(i64),
                    UnOp::I64ExtendI32U => double_cast!(u32 as i64),
                    UnOp::I64TruncF32S { offset } => rt_math_function!(i64_trunc_f32_s @ offset),
                    UnOp::I64TruncF32U { offset } => rt_math_function!(i64_trunc_f32_u @ offset),
                    UnOp::I64TruncF64S { offset } => rt_math_function!(i64_trunc_f64_s @ offset),
                    UnOp::I64TruncF64U { offset } => rt_math_function!(i64_trunc_f64_u @ offset),
                    UnOp::F32ConvertIxxS => nested_expr! {
                        // - Rust uses "roundTiesToEven".
                        // - WebAssembly specifies round-to-nearest ties-to-even.
                        //
                        // Are they the same?
                        //
                        // Rust: https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast
                        // WASM: https://webassembly.github.io/spec/core/exec/numerics.html#rounding
                        simple_cast!(f32);
                    },
                    UnOp::F32ConvertI32U => double_cast!(u32 as f32),
                    UnOp::F32ConvertI64U => double_cast!(u64 as f32),
                    UnOp::F32DemoteF64 => nested_expr! {
                        // TODO: Does Rust's conversion of `f64` to `f32` preserve the "canonical NaN"
                        out.write_str("/* f32.demote_f64 */ ");
                        c_1.print(out, true, context, function);
                        out.write_str(" as f32");
                    },
                    UnOp::F64ConvertIxxS => simple_cast!(f64),
                    UnOp::F64ConvertI32U => double_cast!(u32 as f64),
                    UnOp::F64ConvertI64U => double_cast!(u64 as f64),
                    UnOp::F64PromoteF32 => nested_expr! {
                        // TODO: Does Rust's conversion of `f32` to `f64` preserve the "canonical NaN"
                        out.write_str("/* f64.promote_f32 */ ");
                        simple_cast!(f64);
                    },
                    UnOp::I32ReinterpretF32 => nested_expr! {
                        out.write_str("f32::to_bits(");
                        c_1.print(out, false, context, function);
                        out.write_str(") as i32");
                    },
                    UnOp::I64ReinterpretF64 => nested_expr! {
                        out.write_str("f64::to_bits(");
                        c_1.print(out, false, context, function);
                        out.write_str(") as i64");
                    },
                    UnOp::F32ReinterpretI32 => {
                        out.write_str("f32::from_bits(");
                        c_1.print(out, false, context, function);
                        out.write_str(" as u32)");
                    }
                    UnOp::F64ReinterpretI64 => {
                        out.write_str("f64::from_bits(");
                        c_1.print(out, false, context, function);
                        out.write_str(" as u64)");
                    }
                    UnOp::I32Extend8S => double_cast!(i8 as i32),
                    UnOp::I32Extend16S => double_cast!(i16 as i32),
                    UnOp::I64Extend8S => double_cast!(i8 as i64),
                    UnOp::I64Extend16S => double_cast!(i16 as i64),
                    UnOp::I64Extend32S => double_cast!(i32 as i64),
                    // Float-to-integer saturation operations translate exactly to Rust casts.
                    UnOp::I32TruncSatFxxU => double_cast!(u32 as i32),
                    UnOp::I64TruncSatFxxU => double_cast!(u64 as i64),
                }
            }
            Self::BinaryOperator { kind, c_1, c_2 } => {
                use crate::ast::BinOp;

                macro_rules! infix_operator {
                    ($operator:literal) => {
                        nested_expr! {
                            c_1.print(out, true, context, function);
                            out.write_str(concat!(" ", $operator, " "));
                            c_2.print(out, true, context, function);
                        }
                    };
                }

                macro_rules! infix_comparison {
                    ($operator:literal $(as $cast:ident)?) => {{
                        out.write_str("((");
                        c_1.print(out, true, context, function);
                        out.write_str(concat!(
                            $(" as ", stringify!($cast),)?
                            ") ",
                            $operator,
                            " (",
                        ));
                        c_2.print(out, true, context, function);
                        out.write_str(concat!(
                            $(" as ", stringify!($cast),)?
                            ")) as i32"
                        ));
                    }};
                }

                macro_rules! function {
                    ($($name:expr),+) => {{
                        $(out.write_str($name);)+
                        out.write_str("(");
                        c_1.print(out, false, context, function);
                        out.write_str(", ");
                        c_2.print(out, false, context, function);
                        out.write_str(")");
                    }};
                }

                macro_rules! rt_math_function {
                    ($name:ident @ $offset:ident) => {{
                        function!(paths::RT_MATH, concat!("::", stringify!($name)));
                        print_trap_with(out, function, $offset, context.debug_info);
                        out.write_str("?");
                    }};
                }

                match *kind {
                    BinOp::Eq => infix_comparison!("=="),
                    BinOp::Ne => infix_comparison!("!="),
                    BinOp::IxxLtS => infix_comparison!("<"),
                    BinOp::IxxGtS | BinOp::FxxGt => infix_comparison!(">"),
                    BinOp::I32LtU => infix_comparison!("<" as u32),
                    BinOp::I32GtU => infix_comparison!(">" as u32),
                    BinOp::I64LtU => infix_comparison!("<" as u64),
                    BinOp::I64GtU => infix_comparison!(">" as u64),
                    BinOp::IxxLeS => infix_comparison!("<="),
                    BinOp::IxxGeS => infix_comparison!(">="),
                    BinOp::I32LeU => infix_comparison!("<=" as u32),
                    BinOp::I32GeU => infix_comparison!(">=" as u32),
                    BinOp::I64LeU => infix_comparison!("<=" as u64),
                    BinOp::I64GeU => infix_comparison!(">=" as u64),
                    BinOp::I32Add => function!("i32::wrapping_add"),
                    BinOp::I64Add => function!("i64::wrapping_add"),
                    BinOp::I32Sub => function!("i32::wrapping_sub"),
                    BinOp::I64Sub => function!("i64::wrapping_sub"),
                    BinOp::I32Mul => function!("i32::wrapping_mul"),
                    BinOp::I64Mul => function!("i64::wrapping_mul"),
                    BinOp::I32DivS { offset } => rt_math_function!(i32_div_s @ offset),
                    BinOp::I64DivS { offset } => rt_math_function!(i64_div_s @ offset),
                    BinOp::I32DivU { offset } => rt_math_function!(i32_div_u @ offset),
                    BinOp::I64DivU { offset } => rt_math_function!(i64_div_u @ offset),
                    BinOp::I32RemS { offset } => rt_math_function!(i32_rem_s @ offset),
                    BinOp::I64RemS { offset } => rt_math_function!(i64_rem_s @ offset),
                    BinOp::I32RemU { offset } => rt_math_function!(i32_rem_u @ offset),
                    BinOp::I64RemU { offset } => rt_math_function!(i64_rem_u @ offset),
                    BinOp::IxxAnd => infix_operator!("&"),
                    BinOp::IxxOr => infix_operator!("|"),
                    BinOp::IxxXor => infix_operator!("^"),
                    BinOp::I32Shl => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(" << (");
                        c_2.print(out, true, context, function);
                        out.write_str(" as u32 % 32)");
                    },
                    BinOp::I64Shl => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(" << (");
                        c_2.print(out, true, context, function);
                        out.write_str(" as u64 % 64)");
                    },
                    BinOp::I32ShrS => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(" >> (");
                        c_2.print(out, true, context, function);
                        out.write_str(" as u32 % 32)");
                    },
                    BinOp::I64ShrS => nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(" >> (");
                        c_2.print(out, true, context, function);
                        out.write_str(" as u64 % 64)");
                    },
                    BinOp::I32ShrU => nested_expr! {
                        out.write_str("(");
                        c_1.print(out, true, context, function);
                        out.write_str(" as u32 >> (");
                        c_2.print(out, true, context, function);
                        out.write_str(" as u32 % 32)) as i32");
                    },
                    BinOp::I64ShrU => nested_expr! {
                        out.write_str("(");
                        c_1.print(out, true, context, function);
                        out.write_str(" as u64 >> (");
                        c_2.print(out, true, context, function);
                        out.write_str(" as u64 % 64)) as i64");
                    },
                    BinOp::I32Rotl => {
                        c_1.print(out, true, context, function);
                        out.write_str(".rotate_left((");
                        c_2.print(out, true, context, function);
                        out.write_str(" % 32) as u32)");
                    }
                    BinOp::I64Rotl => {
                        c_1.print(out, true, context, function);
                        out.write_str(".rotate_left((");
                        c_2.print(out, true, context, function);
                        out.write_str(" % 64) as u32)");
                    }
                    BinOp::I32Rotr => {
                        c_1.print(out, true, context, function);
                        out.write_str(".rotate_right((");
                        c_2.print(out, true, context, function);
                        out.write_str(" % 32) as u32)");
                    }
                    BinOp::I64Rotr => {
                        c_1.print(out, true, context, function);
                        out.write_str(".rotate_right((");
                        c_2.print(out, true, context, function);
                        out.write_str(" % 64) as u32)");
                    }
                }
            }
            Self::GetLocal(local) => write!(out, "{local}"),
            Self::GetGlobal(global) => match context.wasm.global_kind(*global) {
                crate::context::GlobalKind::Const => write!(out, "Self::{global:#}"),
                crate::context::GlobalKind::ImmutableField => write!(out, "self.{global}"),
                crate::context::GlobalKind::MutableField { import: None } => {
                    write!(out, "self.{global}.get()")
                }
                crate::context::GlobalKind::MutableField {
                    import: Some(import),
                } => todo!("printing of mutable global imports {import:?}"),
            },
            Self::Temporary(temp) => write!(out, "{temp}"),
            Self::LoopInput(input) => write!(out, "{input}"),
            Self::MemoryLoad {
                memory,
                kind,
                address,
                offset,
                instruction_offset,
            } => {
                use crate::ast::{I32StorageSize, I64StorageSize, LoadKind, SignExtensionMode};

                match kind {
                    LoadKind::F32 => out.write_str("f32::from_bits("),
                    LoadKind::F64 => out.write_str("f64::from_bits("),
                    LoadKind::AsI32 { .. } | LoadKind::AsI64 { .. } if nested => out.write_str("("),
                    _ => (),
                }

                out.write_str(paths::RT_MEM);
                out.write_str("::");
                out.write_str(match kind {
                    LoadKind::AsI32 {
                        storage_size: I32StorageSize::I8,
                        ..
                    }
                    | LoadKind::AsI64 {
                        storage_size: I64StorageSize::I8,
                        ..
                    } => "i8",
                    LoadKind::AsI32 {
                        storage_size: I32StorageSize::I16,
                        ..
                    }
                    | LoadKind::AsI64 {
                        storage_size: I64StorageSize::I16,
                        ..
                    } => "i16",
                    LoadKind::I32
                    | LoadKind::F32
                    | LoadKind::AsI64 {
                        storage_size: I64StorageSize::I32,
                        ..
                    } => "i32",
                    LoadKind::I64 | LoadKind::F64 => "i64",
                });

                let memory64 = context.wasm.types.memory_at(memory.0).memory64;
                write!(out, "_load::<{}, ", memory.0);
                out.write_str(if memory64 { "u64" } else { "u32" });
                out.write_str(", _, _>(");
                let memory_ident = context.wasm.memory_ident(*memory);

                if matches!(memory_ident, crate::context::MemoryIdent::Id(_)) {
                    out.write_str("&");
                }

                write!(out, "self.{memory_ident}, {offset}, ");
                address.print(out, false, context, function);

                out.write_str(", ");
                print_frame(out, function, *instruction_offset, context.debug_info);
                out.write_str(")?");

                match kind {
                    LoadKind::F32 => out.write_str(" as u32)"),
                    LoadKind::F64 => out.write_str(" as u64)"),
                    LoadKind::AsI32 {
                        storage_size,
                        sign_extension,
                    } => {
                        if matches!(sign_extension, SignExtensionMode::Unsigned) {
                            out.write_str(match storage_size {
                                crate::ast::I32StorageSize::I8 => " as u8",
                                crate::ast::I32StorageSize::I16 => " as u16",
                            });
                        }

                        out.write_str(" as i32");

                        if nested {
                            out.write_str(")");
                        }
                    }
                    LoadKind::AsI64 {
                        storage_size,
                        sign_extension,
                    } => {
                        if matches!(sign_extension, SignExtensionMode::Unsigned) {
                            out.write_str(match storage_size {
                                crate::ast::I64StorageSize::I8 => " as u8",
                                crate::ast::I64StorageSize::I16 => " as u16",
                                crate::ast::I64StorageSize::I32 => " as u32",
                            });
                        }

                        out.write_str(" as i64");

                        if nested {
                            out.write_str(")");
                        }
                    }
                    _ => (),
                }
            }
            Self::MemorySize(memory) => {
                write!(
                    out,
                    "{}::size(&self.{})",
                    paths::RT_MEM,
                    context.wasm.memory_ident(*memory)
                );
            }
            Self::MemoryGrow { memory, delta } => {
                write!(
                    out,
                    "{}::grow(&self.{}, ",
                    paths::RT_MEM,
                    context.wasm.memory_ident(*memory)
                );
                delta.print(out, nested, context, function);
                out.write_str(")");
            }
            Self::Call {
                callee,
                arguments,
                offset,
            } => {
                debug_assert_eq!(
                    context.wasm.types[context.wasm.types.core_function_at(callee.0)]
                        .unwrap_func()
                        .results()
                        .len(),
                    1,
                    "use Statement::Call instead"
                );

                print_call_expr(out, *callee, *arguments, context, function, *offset)
            }
        }
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
            _ => {
                self.print(out, true, context, function);
                out.write_str(" != 0")
            }
        }
    }
}

fn write_indentation(
    out: &mut dyn crate::write::Write,
    indentation: Indentation,
    indent_level: u32,
) {
    for _ in 0..indent_level {
        out.write_str(indentation.to_str());
    }
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

fn print_loop_inputs(
    out: &mut dyn crate::write::Write,
    context: &Context,
    function: crate::ast::FuncId,
    indentation: Indentation,
    indent_level: u32,
    r#loop: crate::ast::BlockId,
    values: crate::ast::ExprListId,
) {
    for (expr, number) in context.arena.get_list(values).iter().zip(0u32..=u32::MAX) {
        write!(out, "{} = ", crate::ast::LoopInput { r#loop, number });

        expr.print(out, false, context, Some(function));
        out.write_str(";\n");
        write_indentation(out, indentation, indent_level);
    }
}

#[derive(Clone, Copy, Debug)]
struct BranchTableCase {
    target: crate::ast::BranchTarget,
    values: crate::ast::ExprListId,
}

// Contains code duplicated with cases for `Statement::Branch` in `print_statements()`
fn print_branch_table_case(
    out: &mut dyn crate::write::Write,
    context: &Context,
    function: crate::ast::FuncId,
    indentation: Indentation,
    indent_level: u32,
    is_last: bool,
    case: BranchTableCase,
) {
    match case.target {
        crate::ast::BranchTarget::Return => {
            let can_unwind = context
                .wasm
                .function_attributes
                .unwind_kind(function)
                .can_unwind();

            if !is_last {
                out.write_str("return ");

                if !case.values.is_empty() || can_unwind {
                    out.write_str(" ");
                }
            }

            if can_unwind {
                out.write_str("Ok(");
            }

            case.values.print(
                out,
                case.values.len() > 1 || (case.values.is_empty() && can_unwind),
                context,
                Some(function),
            );

            if can_unwind {
                out.write_str(")");
            }
        }
        crate::ast::BranchTarget::Block(block) => {
            write!(out, "break {block}");

            if !case.values.is_empty() {
                out.write_str(" ");

                case.values
                    .print(out, case.values.len() > 1, context, Some(function));
            }
        }
        crate::ast::BranchTarget::Loop(r#loop) => {
            if !case.values.is_empty() {
                out.write_str("{\n");
                write_indentation(out, indentation, indent_level + 1);

                print_loop_inputs(
                    out,
                    context,
                    function,
                    indentation,
                    indent_level + 1,
                    r#loop,
                    case.values,
                );
            }

            write!(out, "continue {}", r#loop);

            if !case.values.is_empty() {
                out.write_str("\n");
                write_indentation(out, indentation, indent_level);
                out.write_str("}");
            }
        }
    }

    out.write_str(",\n");
}

pub(crate) fn print_statements(
    out: &mut dyn crate::write::Write,
    context: &Context,
    function: crate::ast::FuncId,
    indentation: Indentation,
    mut indent_level: u32,
    statements: &[crate::ast::Statement],
) {
    use crate::ast::Statement;

    for (n, stmt) in statements.iter().copied().enumerate() {
        let is_last = n == statements.len() - 1;

        // Special handling of indentations for `return`s.
        if !matches!(
            stmt,
            Statement::BlockEnd { .. }
                | Statement::Else { .. }
                | Statement::BlockEndUnreachable { .. }
                | Statement::Branch {
                    target: crate::ast::BranchTarget::Return,
                    ..
                }
        ) {
            write_indentation(out, indentation, indent_level);
        }

        match stmt {
            Statement::Expr(expr) => {
                use crate::ast::Expr;

                debug_assert!(!is_last, "expected a terminator statement");

                match context.arena.get(expr) {
                    Expr::Literal(literal) => write!(out, "// {literal}"),
                    // No side effects or sub-expressions to evauluate.
                    Expr::GetGlobal(_)
                    | Expr::GetLocal(_)
                    | Expr::LoopInput(_)
                    | Expr::Temporary(_)
                    | Expr::MemorySize(_) => (),
                    expr => {
                        const DISCARD: &str = "let _ = ";

                        match expr {
                            Expr::Call { callee, .. } => {
                                let has_error = context
                                    .wasm
                                    .function_attributes
                                    .unwind_kind(callee)
                                    .can_unwind();

                                let returns_values =
                                    !context.wasm.function_signature(callee).results().is_empty();

                                if has_error || returns_values {
                                    out.write_str(DISCARD);
                                }
                            }
                            Expr::UnaryOperator { .. }
                            | Expr::BinaryOperator { .. }
                            | Expr::MemoryLoad { .. } => out.write_str(DISCARD),
                            _ => (),
                        }

                        expr.print(out, false, context, Some(function));
                        out.write_str(";");
                    }
                }
            }
            Statement::Call {
                callee,
                arguments,
                results,
                offset,
            } => {
                if let Some((results, result_count)) = results {
                    out.write_str("let ");

                    if result_count.get() > 1 {
                        out.write_str("(");
                    }

                    for i in 0..result_count.get() {
                        if i > 0 {
                            out.write_str(", ");
                        }

                        write!(out, "{}", crate::ast::TempId(results.0 + i));
                    }

                    if result_count.get() > 1 {
                        out.write_str(")");
                    }

                    out.write_str(" = ");
                }

                print_call_expr(out, callee, arguments, context, Some(function), offset);
                out.write_str(";");
            }
            Statement::Branch {
                target: crate::ast::BranchTarget::Return,
                values: results,
                condition,
            } => {
                let can_unwind = context
                    .wasm
                    .function_attributes
                    .unwind_kind(function)
                    .can_unwind();

                let has_return =
                    condition.is_some() || can_unwind || !results.is_empty() || !is_last;

                if has_return {
                    write_indentation(out, indentation, indent_level);
                }

                if let Some(condition) = condition {
                    out.write_str("if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" { ");
                }

                if !is_last {
                    out.write_str("return");

                    if !results.is_empty() || can_unwind {
                        out.write_str(" ");
                    }
                }

                if can_unwind {
                    out.write_str("Ok(");
                }

                results.print(
                    out,
                    results.len() > 1 || (results.is_empty() && can_unwind),
                    context,
                    Some(function),
                );

                if can_unwind {
                    out.write_str(")");
                }

                if condition.is_some() {
                    out.write_str("; }");
                } else if !is_last {
                    out.write_str(";");
                }

                if has_return {
                    out.write_str("\n");
                }
            }
            Statement::Branch {
                target: crate::ast::BranchTarget::Block(block),
                values,
                condition,
            } => {
                if let Some(condition) = condition {
                    out.write_str("if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" { ");
                }

                write!(out, "break {block}");

                if !values.is_empty() {
                    out.write_str(" ");

                    values.print(out, values.len() > 1, context, Some(function));
                }

                out.write_str(";");

                if condition.is_some() {
                    out.write_str("}");
                }
            }
            Statement::Branch {
                target: crate::ast::BranchTarget::Loop(r#loop),
                values,
                condition,
            } => {
                if let Some(condition) = condition {
                    out.write_str("if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" { ");
                }

                print_loop_inputs(
                    out,
                    context,
                    function,
                    indentation,
                    indent_level,
                    r#loop,
                    values,
                );

                write!(out, "continue {};", r#loop);

                if condition.is_some() {
                    out.write_str("}");
                }
            }
            Statement::BranchTable {
                values,
                targets,
                default_target,
                comparand,
            } => {
                out.write_str("match ");
                comparand.print(out, false, context, Some(function));
                out.write_str(" {\n");

                for (i, target) in context
                    .arena
                    .get_branch_targets(targets)
                    .iter()
                    .copied()
                    .enumerate()
                {
                    write_indentation(out, indentation, indent_level + 1);
                    write!(out, "{i} => ");
                    print_branch_table_case(
                        out,
                        context,
                        function,
                        indentation,
                        indent_level + 1,
                        is_last,
                        BranchTableCase { target, values },
                    );
                }

                write_indentation(out, indentation, indent_level + 1);
                out.write_str("_ => ");
                print_branch_table_case(
                    out,
                    context,
                    function,
                    indentation,
                    indent_level + 1,
                    is_last,
                    BranchTableCase {
                        target: default_target,
                        values,
                    },
                );

                write_indentation(out, indentation, indent_level);
                out.write_str("}");
            }
            Statement::DefineLocal(local, ty) => {
                use crate::ast::ValType;

                write!(out, "let mut {local} = ");
                match ty {
                    ValType::I32 => out.write_str("0i32"),
                    ValType::I64 => out.write_str("0i64"),
                    ValType::F32 => out.write_str("0f32"),
                    ValType::F64 => out.write_str("0f64"),
                }

                out.write_str(";");
            }
            Statement::Temporary { temporary, value } => {
                write!(out, "let {temporary} = ");
                value.print(out, false, context, Some(function));
                out.write_str(";");
            }
            Statement::SetLocal { local, value } => {
                write!(out, "{local} = ");
                value.print(out, false, context, Some(function));
                out.write_str(";");
            }
            Statement::SetGlobal { global, value } => {
                out.write_str("embedder::rt::global::Global::set(");

                if let Some(import) = context.wasm.global_import(global) {
                    todo!("set global import {import:?}");
                } else {
                    write!(out, "&self.{global}")
                }

                out.write_str(", ");
                value.print(out, false, context, Some(function));
                out.write_str(");")
            }
            Statement::BlockStart { id, results, kind } => {
                debug_assert!(!is_last);

                if let crate::ast::BlockKind::Loop { inputs } = kind {
                    for (i, expr) in context.arena.get_list(inputs).iter().enumerate() {
                        write!(
                            out,
                            "let mut {} = ",
                            crate::ast::LoopInput {
                                r#loop: id,
                                number: i as u32
                            }
                        );

                        expr.print(out, false, context, Some(function));
                        writeln!(out, ";");
                        write_indentation(out, indentation, indent_level);
                    }
                }

                if let Some(results) = results {
                    out.write_str("let ");

                    if results.count.get() > 1 {
                        out.write_str("(");
                    }

                    for i in 0..results.count.get() {
                        if i > 0 {
                            out.write_str(", ");
                        }

                        write!(out, "{}", crate::ast::TempId(results.start.0 + i));
                    }

                    if results.count.get() > 1 {
                        out.write_str(")");
                    }

                    out.write_str(" = ");
                }

                write!(out, "{id}: ");

                if matches!(kind, crate::ast::BlockKind::Loop { .. }) {
                    out.write_str("loop ");
                }

                out.write_str("{");

                if let crate::ast::BlockKind::If { condition } = kind {
                    out.write_str(" if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" {");
                }

                indent_level += 1;
            }
            Statement::Else { previous_results } => {
                debug_assert!(!is_last);

                if !previous_results.is_empty() {
                    write_indentation(out, indentation, indent_level);
                    previous_results.print(
                        out,
                        previous_results.len() > 1,
                        context,
                        Some(function),
                    );
                    out.write_str("\n");
                }

                indent_level -= 1;

                write_indentation(out, indentation, indent_level);
                out.write_str("} else {");

                indent_level += 1;
            }
            Statement::BlockEnd { id, kind, results } => {
                debug_assert!(!is_last);

                let is_loop = matches!(kind, crate::ast::BlockKind::Loop { .. });
                if is_loop || !results.is_empty() {
                    write_indentation(out, indentation, indent_level);
                }

                if is_loop {
                    write!(out, "break {id}");

                    if !results.is_empty() {
                        out.write_str(" ");
                    };
                }

                if !results.is_empty() {
                    results.print(out, results.len() > 1, context, Some(function));

                    if is_loop {
                        out.write_str(";");
                    }
                }

                if is_loop || !results.is_empty() {
                    out.write_str("\n");
                }

                indent_level -= 1;

                write_indentation(out, indentation, indent_level);

                if let crate::ast::BlockKind::If { condition: () } = kind {
                    out.write_str("} ");
                }

                out.write_str("}");

                if !results.is_empty() {
                    out.write_str(";");
                }

                write!(out, " // {id}");
            }
            Statement::BlockEndUnreachable {
                id,
                kind,
                has_results,
            } => {
                indent_level -= 1;

                write_indentation(out, indentation, indent_level);

                if let crate::ast::BlockKind::If { condition: () } = kind {
                    out.write_str("} ");
                }

                out.write_str("}");

                if has_results {
                    out.write_str(";");
                }

                write!(out, " // {id}");
            }
            Statement::MemoryStore {
                memory,
                kind,
                address,
                value,
                offset,
                instruction_offset,
            } => {
                use crate::ast::StoreKind;

                out.write_str(paths::RT_MEM);
                out.write_str("::");
                out.write_str(match kind {
                    StoreKind::I8 => "i8",
                    StoreKind::I16 => "i16",
                    StoreKind::I32 | StoreKind::AsI32 | StoreKind::F32 => "i32",
                    StoreKind::I64 | StoreKind::F64 => "i64",
                });
                write!(out, "_store::<{}, ", memory.0);
                let memory64 = context.wasm.types.memory_at(memory.0).memory64;
                out.write_str(if memory64 { "u64" } else { "u32" });
                out.write_str(", _, _>(");
                print_memory(out, memory, context.wasm);
                write!(out, ", {offset}, ",);
                address.print(out, false, context, Some(function));
                out.write_str(", ");

                match kind {
                    StoreKind::F32 => out.write_str("f32::to_bits("),
                    StoreKind::F64 => out.write_str("f64::to_bits("),
                    _ => (),
                }

                value.print(
                    out,
                    matches!(kind, StoreKind::I8 | StoreKind::I16 | StoreKind::AsI32),
                    context,
                    Some(function),
                );

                match kind {
                    StoreKind::I8 => out.write_str(" as i8"),
                    StoreKind::I16 => out.write_str(" as i16"),
                    StoreKind::AsI32 => out.write_str(" as i32"),
                    StoreKind::F32 => out.write_str(") as i32"),
                    StoreKind::F64 => out.write_str(") as i64"),
                    _ => (),
                }

                out.write_str(", ");
                print_frame(out, Some(function), instruction_offset, context.debug_info);
                out.write_str(")?;");
            }
            Statement::MemoryFill {
                memory,
                address,
                byte,
                length,
                instruction_offset,
            } => {
                out.write_str(paths::RT_MEM);
                write!(out, "::fill::<{}, _, _, _>(", memory.0);
                print_memory(out, memory, context.wasm);
                out.write_str(", ");
                address.print(out, false, context, Some(function));
                out.write_str(", ");
                byte.print(out, false, context, Some(function));
                out.write_str(", ");
                length.print(out, false, context, Some(function));
                out.write_str(", ");
                print_frame(out, Some(function), instruction_offset, context.debug_info);
                out.write_str(")?;");
            }
            Statement::Unreachable { offset } => {
                out.write_str("return ::core::result::Result::Err(");

                write!(
                    out,
                    "{}::trap(embedder::rt::trap::UnreachableError",
                    paths::RT_TRAP
                );

                out.write_str(", ");
                print_frame(out, Some(function), offset, context.debug_info);
                out.write_str("));");
            }
        }

        // Special handling for newlines for `return`s.
        if !matches!(
            stmt,
            Statement::Branch {
                target: crate::ast::BranchTarget::Return,
                ..
            }
        ) {
            out.write_str("\n");
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
    write_indentation(out, indentation, indent_level);
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
