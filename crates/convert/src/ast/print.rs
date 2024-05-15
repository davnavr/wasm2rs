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
    pub(super) const RT_MATH: &str = "embedder::rt::math";
    pub(super) const RT_TRAP: &str = "embedder::rt::trap";
    pub(super) const RT_TRAP_CODE: &str = "embedder::rt::trap::TrapCode";
    pub(super) const RT_MEM: &str = "embedder::rt::memory";
}

const INST: &str = "self._inst";

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

impl crate::ast::ExprId {
    fn print(
        self,
        out: &mut crate::buffer::Writer,
        arena: &crate::ast::Arena,
        nested: bool,
        context: &crate::context::Context,
    ) {
        arena.get(self).print(out, arena, nested, context)
    }

    fn print_bool(
        self,
        out: &mut crate::buffer::Writer<'_>,
        arena: &crate::ast::Arena,
        context: &crate::context::Context,
    ) {
        arena.get(self).print_bool(out, arena, context)
    }
}

impl crate::ast::ExprListId {
    fn print(
        self,
        out: &mut crate::buffer::Writer,
        arena: &crate::ast::Arena,
        enclosed: bool,
        context: &crate::context::Context,
    ) {
        if enclosed {
            out.write_str("(");
        }

        for (i, expr) in arena.get_list(self).iter().enumerate() {
            if i > 0 {
                out.write_str(", ");
            }

            expr.print(out, arena, false, context);
        }

        if enclosed {
            out.write_str(")");
        }
    }
}

fn print_call_common<F>(
    out: &mut crate::buffer::Writer,
    callee: crate::ast::FuncId,
    context: &crate::context::Context,
    arguments: F,
) where
    F: FnOnce(&mut crate::buffer::Writer),
{
    use crate::context::CallKind;

    match context.function_attributes.call_kind(callee) {
        CallKind::Function => out.write_str("Self::"),
        CallKind::Method => out.write_str("self."),
    }

    write!(out, "{}(", context.function_name(callee));
    arguments(out);
    out.write_str(")");
}

fn print_call_expr(
    out: &mut crate::buffer::Writer,
    callee: crate::ast::FuncId,
    arguments: crate::ast::ExprListId,
    arena: &crate::ast::Arena,
    context: &crate::context::Context,
) {
    print_call_common(out, callee, context, |out| {
        for (i, arg) in arena.get_list(arguments).iter().enumerate() {
            if i > 0 {
                out.write_str(", ");
            }

            arg.print(out, arena, false, context);
        }
    });

    if context.function_attributes.unwind_kind(callee).can_unwind() {
        out.write_str("?");
    }
}

fn print_memory_offset(out: &mut crate::buffer::Writer, offset: u64, memory64: bool) {
    write!(out, "{offset:#X}");

    // Ensure the Rust compiler won't complain about out-of-range literals.
    if !memory64 && offset > (i32::MAX as u64) {
        out.write_str("u32 as i32");
    } else if memory64 && offset > (i64::MAX as u64) {
        out.write_str("u64 as i64");
    }
}

impl crate::ast::Expr {
    fn print(
        &self,
        out: &mut crate::buffer::Writer<'_>,
        arena: &crate::ast::Arena,
        nested: bool,
        context: &crate::context::Context,
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
                    ($name:ident) => {{
                        out.write_str(paths::RT_MATH);
                        out.write_str(concat!("::", stringify!($name), "("));
                        c_1.print(out, arena, true, context);
                        out.write_str(")?");
                    }};
                }

                macro_rules! simple_cast {
                    ($to:ident) => {
                        nested_expr! {
                            c_1.print(out, arena, true, context);
                            out.write_str(concat!(" as ", stringify!($to)));
                        }
                    };
                }

                macro_rules! double_cast {
                    ($start:ident as $end:ident) => {
                        nested_expr! {
                            c_1.print(out, arena, true, context);
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
                        c_1.print(out, arena, false, context);
                        out.write_str(" == 0) as i32");
                    },
                    UnOp::I32Clz => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(".leading_zeros() as i32");
                    },
                    UnOp::I64Clz => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(".leading_zeros() as i64");
                    },
                    UnOp::I32Ctz => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(".trailing_zeros() as i32");
                    },
                    UnOp::I64Ctz => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(".trailing_zeros() as i64");
                    },
                    UnOp::I32Popcnt => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(".count_ones() as i32");
                    },
                    UnOp::I64Popcnt => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(".count_ones() as i64");
                    },
                    UnOp::FxxNeg => nested_expr! {
                        // `::core::ops::Neg` on `f32` and `f64` do the same operation in Rust.
                        out.write_str("-");
                        c_1.print(out, arena, true, context);
                    },
                    UnOp::I32WrapI64 | UnOp::I32TruncSatFxxS => simple_cast!(i32),
                    UnOp::I32TruncF32S => rt_math_function!(i32_trunc_f32_s),
                    UnOp::I32TruncF32U => rt_math_function!(i32_trunc_f32_u),
                    UnOp::I32TruncF64S => rt_math_function!(i32_trunc_f64_s),
                    UnOp::I32TruncF64U => rt_math_function!(i32_trunc_f64_u),
                    UnOp::I64ExtendI32S | UnOp::I64TruncSatFxxS => simple_cast!(i64),
                    UnOp::I64ExtendI32U => double_cast!(u32 as i64),
                    UnOp::I64TruncF32S => rt_math_function!(i64_trunc_f32_s),
                    UnOp::I64TruncF32U => rt_math_function!(i64_trunc_f32_u),
                    UnOp::I64TruncF64S => rt_math_function!(i64_trunc_f64_s),
                    UnOp::I64TruncF64U => rt_math_function!(i64_trunc_f64_u),
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
                        c_1.print(out, arena, true, context);
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
                        c_1.print(out, arena, false, context);
                        out.write_str(") as i32");
                    },
                    UnOp::I64ReinterpretF64 => nested_expr! {
                        out.write_str("f64::to_bits(");
                        c_1.print(out, arena, false, context);
                        out.write_str(") as i64");
                    },
                    UnOp::F32ReinterpretI32 => {
                        out.write_str("f32::from_bits(");
                        c_1.print(out, arena, false, context);
                        out.write_str(" as u32)");
                    }
                    UnOp::F64ReinterpretI64 => {
                        out.write_str("f64::from_bits(");
                        c_1.print(out, arena, false, context);
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
                            c_1.print(out, arena, true, context);
                            out.write_str(concat!(" ", $operator, " "));
                            c_2.print(out, arena, true, context);
                        }
                    };
                }

                macro_rules! infix_comparison {
                    ($operator:literal $(as $cast:ident)?) => {{
                        out.write_str("(");
                        c_1.print(out, arena, true, context);
                        out.write_str(concat!(
                            $(" as ", stringify!($cast),)?
                            " ",
                            $operator,
                            " ",
                        ));
                        c_2.print(out, arena, true, context);
                        out.write_str(concat!(
                            $(" as ", stringify!($cast),)?
                            ") as i32"
                        ));
                    }};
                }

                macro_rules! function {
                    ($($name:expr),+) => {{
                        $(out.write_str($name);)+
                        out.write_str("(");
                        c_1.print(out, arena, false, context);
                        out.write_str(", ");
                        c_2.print(out, arena, false, context);
                        out.write_str(")");
                    }};
                }

                macro_rules! rt_math_function {
                    ($name:ident) => {{
                        function!(paths::RT_MATH, concat!("::", stringify!($name)));
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
                    BinOp::I32DivS => rt_math_function!(i32_div_s),
                    BinOp::I64DivS => rt_math_function!(i64_div_s),
                    BinOp::I32DivU => rt_math_function!(i32_div_u),
                    BinOp::I64DivU => rt_math_function!(i64_div_u),
                    BinOp::I32RemS => rt_math_function!(i32_rem_s),
                    BinOp::I64RemS => rt_math_function!(i64_rem_s),
                    BinOp::I32RemU => rt_math_function!(i32_rem_u),
                    BinOp::I64RemU => rt_math_function!(i64_rem_u),
                    BinOp::IxxAnd => infix_operator!("&"),
                    BinOp::IxxOr => infix_operator!("|"),
                    BinOp::IxxXor => infix_operator!("^"),
                    BinOp::I32Shl => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(" << (");
                        c_2.print(out, arena, true, context);
                        out.write_str(" as u32 % 32)");
                    },
                    BinOp::I64Shl => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(" << (");
                        c_2.print(out, arena, true, context);
                        out.write_str(" as u64 % 64)");
                    },
                    BinOp::I32ShrS => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(" >> (");
                        c_2.print(out, arena, true, context);
                        out.write_str(" as u32 % 32)");
                    },
                    BinOp::I64ShrS => nested_expr! {
                        c_1.print(out, arena, true, context);
                        out.write_str(" >> (");
                        c_2.print(out, arena, true, context);
                        out.write_str(" as u64 % 64)");
                    },
                    BinOp::I32ShrU => nested_expr! {
                        out.write_str("(");
                        c_1.print(out, arena, true, context);
                        out.write_str(" as u32 >> (");
                        c_2.print(out, arena, true, context);
                        out.write_str(" as u32 % 32)) as i32");
                    },
                    BinOp::I64ShrU => nested_expr! {
                        out.write_str("(");
                        c_1.print(out, arena, true, context);
                        out.write_str(" as u64 >> (");
                        c_2.print(out, arena, true, context);
                        out.write_str(" as u64 % 64) as i64");
                    },
                    BinOp::I32Rotl => {
                        c_1.print(out, arena, true, context);
                        out.write_str(".rotate_left((");
                        c_2.print(out, arena, true, context);
                        out.write_str(" % 32) as u32)");
                    }
                    BinOp::I64Rotl => {
                        c_1.print(out, arena, true, context);
                        out.write_str(".rotate_left((");
                        c_2.print(out, arena, true, context);
                        out.write_str(" % 64) as u64)");
                    }
                    BinOp::I32Rotr => {
                        c_1.print(out, arena, true, context);
                        out.write_str(".rotate_right((");
                        c_2.print(out, arena, true, context);
                        out.write_str(" % 32) as u32)");
                    }
                    BinOp::I64Rotr => {
                        c_1.print(out, arena, true, context);
                        out.write_str(".rotate_right((");
                        c_2.print(out, arena, true, context);
                        out.write_str(" % 64) as u64)");
                    }
                }
            }
            Self::GetLocal(local) => write!(out, "{local}"),
            Self::GetGlobal(global) => match context.global_kind(*global) {
                crate::context::GlobalKind::Const => write!(out, "Self::{global:#}"),
                crate::context::GlobalKind::ImmutableField => write!(out, "{INST}.{global}"),
                crate::context::GlobalKind::MutableField { import: None } => {
                    write!(out, "{INST}.{global}.get()")
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
            } => {
                use crate::ast::{I32StorageSize, I64StorageSize, LoadKind};

                // TODO: Check if memory is imported.

                match kind {
                    LoadKind::F32 => out.write_str("f32::from_bits("),
                    LoadKind::F64 => out.write_str("f64::from_bits("),
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

                let memory64 = context.types.memory_at(memory.0).memory64;
                write!(out, "_load::<{}, ", memory.0);
                out.write_str(if memory64 { "u64" } else { "u32" });
                write!(out, ", _, _>(&self.{memory}, ");
                print_memory_offset(out, *offset, memory64);
                out.write_str(", ");
                address.print(out, arena, false, context);

                out.write_str(", None"); // TODO: WASM frame info for memory loads.

                out.write_str(")?");

                if matches!(kind, LoadKind::F32 | LoadKind::F64) {
                    out.write_str(")");
                }
            }
            Self::Call { callee, arguments } => {
                print_call_expr(out, *callee, *arguments, arena, context)
            }
        }
    }

    /// Emits Rust code evaluating to a `bool` expression.
    ///
    /// This is used when translating WebAssembly comparison instructions.
    fn print_bool(
        &self,
        out: &mut crate::buffer::Writer<'_>,
        arena: &crate::ast::Arena,
        context: &crate::context::Context,
    ) {
        use crate::ast::BinOp;

        macro_rules! comparison {
            ($c_1:ident $operator:literal $c_2:ident) => {{
                $c_1.print(out, arena, false, context);
                out.write_str(concat!(" ", $operator, " "));
                $c_2.print(out, arena, false, context);
            }};
        }

        match self {
            Self::UnaryOperator {
                kind: crate::ast::UnOp::IxxEqz,
                c_1,
            } => {
                c_1.print(out, arena, false, context);
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
                self.print(out, arena, true, context);
                out.write_str(" != 0")
            }
        }
    }
}

pub(crate) struct Print<'wasm, 'ctx> {
    indentation: Indentation,
    context: &'ctx crate::context::Context<'wasm>,
}

impl<'wasm, 'ctx> Print<'wasm, 'ctx> {
    pub(crate) const fn new(
        indentation: Indentation,
        context: &'ctx crate::context::Context<'wasm>,
    ) -> Self {
        Self {
            indentation,
            context,
        }
    }

    pub(crate) fn context(&self) -> &'ctx crate::context::Context<'wasm> {
        self.context
    }

    fn write_indentation(&self, out: &mut crate::buffer::Writer, indent_level: u32) {
        for _ in 0..indent_level {
            out.write_str(self.indentation.to_str());
        }
    }

    pub(crate) fn print_statements(
        &self,
        function: crate::ast::FuncId,
        mut indent_level: u32,
        out: &mut crate::buffer::Writer,
        statements: &[crate::ast::Statement],
        arena: &crate::ast::Arena,
    ) {
        use crate::ast::Statement;

        for (n, stmt) in statements.iter().copied().enumerate() {
            let is_last = n == statements.len() - 1;

            if !matches!(
                stmt,
                Statement::BlockEnd { .. }
                    | Statement::Else { .. }
                    | Statement::BlockEndUnreachable { .. }
            ) {
                self.write_indentation(out, indent_level);
            }

            match stmt {
                Statement::Expr(expr) => {
                    debug_assert!(!is_last, "expected a terminator statement");

                    expr.print(out, arena, false, self.context);
                    out.write_str(";");
                }
                Statement::Call {
                    callee,
                    arguments,
                    results,
                    result_count,
                } => {
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
                    print_call_expr(out, callee, arguments, arena, self.context);
                    out.write_str(";");
                }
                Statement::Branch {
                    target: crate::ast::BranchTarget::Return,
                    values: results,
                    condition,
                } => {
                    if let Some(condition) = condition {
                        out.write_str("if ");
                        condition.print_bool(out, arena, self.context);
                        out.write_str(" { ");
                    }

                    let can_unwind = self
                        .context
                        .function_attributes
                        .unwind_kind(function)
                        .can_unwind();
                    if !is_last {
                        out.write_str("return");

                        if !results.is_empty() || can_unwind {
                            out.write_str(" ");
                        }
                    }

                    if can_unwind {
                        out.write_str("Ok(");
                    }

                    results.print(out, arena, results.len() != 1, self.context);

                    if can_unwind {
                        out.write_str(")");
                    }

                    if condition.is_some() {
                        out.write_str("; }");
                    } else if !is_last {
                        out.write_str(";");
                    }
                }
                Statement::Branch {
                    target: crate::ast::BranchTarget::Block(block),
                    values,
                    condition,
                } => {
                    if let Some(condition) = condition {
                        out.write_str("if ");
                        condition.print_bool(out, arena, self.context);
                        out.write_str(" { ");
                    }

                    write!(out, "break {block}");

                    if !values.is_empty() {
                        out.write_str(" ");

                        values.print(out, arena, values.len() > 1, self.context);
                    }

                    out.write_str(";");

                    if condition.is_some() {
                        out.write_str("}");
                    }
                }
                Statement::Branch {
                    target: crate::ast::BranchTarget::Loop(target),
                    values,
                    condition,
                } => {
                    if let Some(condition) = condition {
                        out.write_str("if ");
                        condition.print_bool(out, arena, self.context);
                        out.write_str(" { ");
                    }

                    for (i, expr) in arena.get_list(values).iter().enumerate() {
                        write!(
                            out,
                            "{} = ",
                            crate::ast::LoopInput {
                                r#loop: target,
                                number: i as u32
                            }
                        );

                        expr.print(out, arena, false, self.context);
                        out.write_str(";\n");
                        self.write_indentation(out, indent_level);
                    }

                    write!(out, "continue {target};");

                    if condition.is_some() {
                        out.write_str("}");
                    }
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
                    value.print(out, arena, false, self.context);
                    out.write_str(";");
                }
                Statement::SetLocal { local, value } => {
                    write!(out, "{local} = ");
                    value.print(out, arena, false, self.context);
                    out.write_str(";");
                }
                Statement::SetGlobal { global, value } => {
                    out.write_str("embedder::rt::global::Global::set(");

                    if let Some(import) = self.context.global_import(global) {
                        todo!("set global import {import:?}");
                    } else {
                        write!(out, "&self.{global}")
                    }

                    out.write_str(", ");
                    value.print(out, arena, false, self.context);
                    out.write_str(");")
                }
                Statement::BlockStart { id, results, kind } => {
                    debug_assert!(!is_last);

                    if let crate::ast::BlockKind::Loop { inputs } = kind {
                        for (i, expr) in arena.get_list(inputs).iter().enumerate() {
                            write!(
                                out,
                                "let mut {} = ",
                                crate::ast::LoopInput {
                                    r#loop: id,
                                    number: i as u32
                                }
                            );

                            expr.print(out, arena, false, self.context);
                            writeln!(out, ";");
                            self.write_indentation(out, indent_level);
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
                        condition.print_bool(out, arena, self.context);
                        out.write_str(" {");
                    }

                    indent_level += 1;
                }
                Statement::Else { previous_results } => {
                    debug_assert!(!is_last);

                    if !previous_results.is_empty() {
                        self.write_indentation(out, indent_level);
                        previous_results.print(
                            out,
                            arena,
                            previous_results.len() > 1,
                            self.context,
                        );
                        out.write_str("\n");
                    }

                    indent_level -= 1;

                    self.write_indentation(out, indent_level);
                    out.write_str("} else {");

                    indent_level += 1;
                }
                Statement::BlockEnd { id, kind, results } => {
                    debug_assert!(!is_last);

                    if !results.is_empty() {
                        self.write_indentation(out, indent_level);
                    }

                    if matches!(kind, crate::ast::BlockKind::Loop { .. }) {
                        write!(out, "break {id}");

                        if !results.is_empty() {
                            out.write_str(" ");
                        };
                    }

                    if !results.is_empty() {
                        results.print(out, arena, results.len() > 1, self.context);

                        if matches!(kind, crate::ast::BlockKind::Loop { .. }) {
                            out.write_str(";");
                        }

                        out.write_str("\n");
                    }

                    indent_level -= 1;

                    self.write_indentation(out, indent_level);

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

                    self.write_indentation(out, indent_level);

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
                } => {
                    use crate::ast::StoreKind;

                    // TODO: Check if memory is imported.

                    out.write_str(paths::RT_MEM);
                    out.write_str("::");
                    out.write_str(match kind {
                        StoreKind::I8 => "i8",
                        StoreKind::I16 => "i16",
                        StoreKind::I32 | StoreKind::AsI32 | StoreKind::F32 => "i32",
                        StoreKind::I64 | StoreKind::F64 => "i64",
                    });
                    write!(out, "_store::<{}, ", memory.0);
                    let memory64 = self.context.types.memory_at(memory.0).memory64;
                    out.write_str(if memory64 { "u64" } else { "u32" });
                    write!(out, ", _, _>(&self.{memory}, ");
                    print_memory_offset(out, offset, memory64);
                    out.write_str(", ");
                    address.print(out, arena, false, self.context);
                    out.write_str(", ");

                    match kind {
                        StoreKind::F32 => out.write_str("f32::to_bits("),
                        StoreKind::F64 => out.write_str("f64::to_bits("),
                        _ => (),
                    }

                    value.print(
                        out,
                        arena,
                        matches!(kind, StoreKind::I8 | StoreKind::I16 | StoreKind::AsI32),
                        self.context,
                    );

                    match kind {
                        StoreKind::I8 => out.write_str(" as i8"),
                        StoreKind::I16 => out.write_str(" as i16"),
                        StoreKind::AsI32 => out.write_str(" as i32"),
                        StoreKind::F32 => out.write_str(") as i32"),
                        StoreKind::F64 => out.write_str(") as i64"),
                        _ => (),
                    }

                    out.write_str(", None"); // TODO: WASM frame info for memory stores.

                    out.write_str(")?;");
                }
                Statement::Unreachable { function, offset } => {
                    write!(
                        out,
                        "return ::core::result::Err({}::Trap::with_code(\
                            {}::Unreachable, \
                            todo!(\"how to encode {function} @ {offset:#X}\"\
                        )));",
                        paths::RT_TRAP,
                        paths::RT_TRAP_CODE,
                    );
                }
            }

            out.write_str("\n");
        }
    }

    pub(crate) fn print_stub(
        &self,
        indent_level: u32,
        out: &mut crate::buffer::Writer,
        function: crate::ast::FuncId,
        arguments: u32,
    ) {
        self.write_indentation(out, indent_level);
        print_call_common(out, function, self.context, |out| {
            for i in 0u32..arguments {
                if i > 0 {
                    out.write_str(", ");
                }

                write!(out, "{}", crate::ast::LocalId(i));
            }
        });
        out.write_str("\n");
    }
}
