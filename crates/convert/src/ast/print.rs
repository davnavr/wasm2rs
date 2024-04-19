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

    fn to_str(&self) -> &'static str {
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
}

impl crate::ast::ValType {
    pub(crate) fn print(&self, out: &mut crate::buffer::Writer) {
        match self {
            Self::I32 => out.write_str("i32"),
            Self::I64 => out.write_str("i64"),
            Self::F32 => out.write_str("f32"),
            Self::F64 => out.write_str("f64"),
        }
    }
}

impl crate::ast::Literal {
    fn print(&self, out: &mut crate::buffer::Writer) {
        match self {
            Self::I32(i) if *i <= 9 => write!(out, "{i}i32"),
            Self::I32(i) if *i <= 0xFFFF => write!(out, "{i:#X}i32"),
            Self::I32(i) => write!(out, "{i:#010X}i32"),
            Self::I64(i) if *i <= 9 => write!(out, "{i}i64"),
            Self::I64(i) if *i <= 0xFFFF => write!(out, "{i:#X}i64"),
            Self::I64(i) => write!(out, "{i:#018X}i64"),
            Self::F32(z) => write!(out, "f32::from_bits({z:#010X})"),
            Self::F64(z) => write!(out, "f64::from_bits({z:#018X})"),
        }
    }
}

impl crate::ast::ExprId {
    fn print(
        self,
        out: &mut crate::buffer::Writer,
        arena: &crate::ast::Arena,
        nested: bool,
        calling_conventions: &[crate::context::CallConv],
    ) {
        arena
            .get(self)
            .print(out, arena, nested, calling_conventions)
    }
}

impl crate::ast::ExprListId {
    fn print(
        self,
        out: &mut crate::buffer::Writer,
        arena: &crate::ast::Arena,
        enclosed: bool,
        calling_conventions: &[crate::context::CallConv],
    ) {
        if enclosed {
            out.write_str("(");
        }

        for (i, expr) in arena.get_list(self).iter().enumerate() {
            if i > 0 {
                out.write_str(", ");
            }

            expr.print(out, arena, false, calling_conventions);
        }

        if enclosed {
            out.write_str(")");
        }
    }
}

impl crate::ast::Expr {
    fn print(
        &self,
        out: &mut crate::buffer::Writer<'_>,
        arena: &crate::ast::Arena,
        nested: bool,
        calling_conventions: &[crate::context::CallConv],
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
            Self::Literal(literal) => literal.print(out),
            Self::UnaryOperator { kind, c_1 } => {
                use crate::ast::UnOp;

                macro_rules! rt_math_function {
                    ($name:ident) => {{
                        out.write_str(paths::RT_MATH);
                        out.write_str(concat!("::", stringify!($name), "("));
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(")?");
                    }};
                }

                macro_rules! simple_cast {
                    ($to:ident) => {
                        nested_expr! {
                            c_1.print(out, arena, true, calling_conventions);
                            out.write_str(concat!(" as ", stringify!($to)));
                        }
                    };
                }

                macro_rules! double_cast {
                    ($start:ident as $end:ident) => {
                        nested_expr! {
                            c_1.print(out, arena, true, calling_conventions);
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
                        c_1.print(out, arena, false, calling_conventions);
                        out.write_str(" == 0) as i32");
                    },
                    UnOp::I32Clz => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".leading_zeros() as i32");
                    },
                    UnOp::I64Clz => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".leading_zeros() as i64");
                    },
                    UnOp::I32Ctz => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".trailing_zeros() as i32");
                    },
                    UnOp::I64Ctz => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".trailing_zeros() as i64");
                    },
                    UnOp::I32Popcnt => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".count_ones() as i32");
                    },
                    UnOp::I64Popcnt => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".count_ones() as i64");
                    },
                    UnOp::FxxNeg => nested_expr! {
                        // `::core::ops::Neg` on `f32` and `f64` do the same operation in Rust.
                        out.write_str("-");
                        c_1.print(out, arena, true, calling_conventions);
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
                        c_1.print(out, arena, true, calling_conventions);
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
                        c_1.print(out, arena, false, calling_conventions);
                        out.write_str(") as i32");
                    },
                    UnOp::I64ReinterpretF64 => nested_expr! {
                        out.write_str("f64::to_bits(");
                        c_1.print(out, arena, false, calling_conventions);
                        out.write_str(") as i64");
                    },
                    UnOp::F32ReinterpretI32 => {
                        out.write_str("f32::from_bits(");
                        c_1.print(out, arena, false, calling_conventions);
                        out.write_str(" as u32)");
                    }
                    UnOp::F64ReinterpretI64 => {
                        out.write_str("f64::from_bits(");
                        c_1.print(out, arena, false, calling_conventions);
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
                            c_1.print(out, arena, true, calling_conventions);
                            out.write_str(concat!(" ", $operator, " "));
                            c_2.print(out, arena, true, calling_conventions);
                        }
                    };
                }

                macro_rules! infix_comparison {
                    ($operator:literal $(as $cast:ident)?) => {{
                        out.write_str("(");
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(concat!(
                            $(" as ", stringify!($cast),)?
                            " ",
                            $operator,
                            " ",
                        ));
                        c_2.print(out, arena, true, calling_conventions);
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
                        c_1.print(out, arena, false, calling_conventions);
                        out.write_str(", ");
                        c_2.print(out, arena, false, calling_conventions);
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
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(" << (");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" as u32 % 32)");
                    },
                    BinOp::I64Shl => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(" << (");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" as u64 % 64)");
                    },
                    BinOp::I32ShrS => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(" >> (");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" as u32 % 32)");
                    },
                    BinOp::I64ShrS => nested_expr! {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(" >> (");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" as u64 % 64)");
                    },
                    BinOp::I32ShrU => nested_expr! {
                        out.write_str("(");
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(" as u32 >> (");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" as u32 % 32)) as i32");
                    },
                    BinOp::I64ShrU => nested_expr! {
                        out.write_str("(");
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(" as u64 >> (");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" as u64 % 64) as i64");
                    },
                    BinOp::I32Rotl => {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".rotate_left((");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" % 32) as u32)");
                    }
                    BinOp::I64Rotl => {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".rotate_left((");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" % 64) as u64)");
                    }
                    BinOp::I32Rotr => {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".rotate_right((");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" % 32) as u32)");
                    }
                    BinOp::I64Rotr => {
                        c_1.print(out, arena, true, calling_conventions);
                        out.write_str(".rotate_right((");
                        c_2.print(out, arena, true, calling_conventions);
                        out.write_str(" % 64) as u64)");
                    }
                }
            }
            Self::GetLocal(local) => write!(out, "{local}"),
            Self::Call { callee, arguments } => {
                use crate::context::CallKind;

                let call_conv = &calling_conventions[callee.0 as usize];

                debug_assert!(
                    call_conv.wasm_signature.results().len() <= 1,
                    "functions with multiple results must be invoked in a separate statement"
                );

                debug_assert_eq!(
                    arguments.len(),
                    call_conv.wasm_signature.params().len() as u32,
                    "cannot call {callee} with {} arguments, expected {:?}",
                    arguments.len(),
                    call_conv.wasm_signature.params()
                );

                match call_conv.call_kind {
                    CallKind::Function => out.write_str("Self::"),
                    CallKind::Method => out.write_str("self."),
                }

                write!(out, "{callee}(");

                for (i, arg) in arena.get_list(*arguments).iter().enumerate() {
                    if i > 0 {
                        out.write_str(", ");
                    }

                    arg.print(out, arena, false, calling_conventions);
                }

                out.write_str(")");

                if call_conv.can_unwind() {
                    out.write_str("?");
                }
            }
        }
    }
}

pub(crate) struct Print<'types, 'a> {
    indentation: Indentation,
    // TODO: Info about globals, memories, etc.
    // TODO: Info about function signatures and CallKinds
    calling_conventions: &'a [crate::context::CallConv<'types>],
}

impl<'types, 'a> Print<'types, 'a> {
    pub(crate) const fn new(
        indentation: Indentation,
        calling_conventions: &'a [crate::context::CallConv<'types>],
    ) -> Self {
        Self {
            indentation,
            calling_conventions,
        }
    }

    pub(crate) fn print_statements(
        &self,
        out: &mut crate::buffer::Writer<'_>,
        arena: &crate::ast::Arena,
        calling_convention: &crate::context::CallConv<'types>,
        statements: &[crate::ast::Statement],
    ) {
        use crate::ast::Statement;

        let mut indent_level = 0usize;
        for (n, stmt) in statements.iter().copied().enumerate() {
            let is_last = n == statements.len() - 1;

            for _ in 0..indent_level {
                out.write_str(self.indentation.to_str());
            }

            match stmt {
                Statement::Expr(expr) => {
                    debug_assert!(!is_last, "expected a terminator statement");

                    expr.print(out, arena, false, self.calling_conventions);
                }
                Statement::Return(results) => {
                    if is_last {
                        out.write_str("return");

                        if !results.is_empty() || calling_convention.can_unwind() {
                            out.write_str(" ");
                        }
                    }

                    if calling_convention.can_unwind() {
                        out.write_str("Ok(");
                    }

                    results.print(out, arena, results.len() != 1, self.calling_conventions);

                    if calling_convention.can_unwind() {
                        out.write_str(")");
                    }
                }
                Statement::LocalDefinition(local, ty) => {
                    use crate::ast::ValType;

                    write!(out, "let mut {local} = ");
                    match ty {
                        ValType::I32 => out.write_str("0i32"),
                        ValType::I64 => out.write_str("0i64"),
                        ValType::F32 => out.write_str("0f32"),
                        ValType::F64 => out.write_str("0f64"),
                    }
                }
                Statement::LocalSet { local, value } => {
                    write!(out, "{local} = ");
                    value.print(out, arena, false, self.calling_conventions);
                }
                Statement::Unreachable { function, offset } => {
                    writeln!(
                        out,
                        "return ::core::result::Err(embedder::Trap::with_code())"
                    );
                }
            }

            if is_last {
                out.write_str(";");
            }

            out.write_str("\n");
        }
    }
}
