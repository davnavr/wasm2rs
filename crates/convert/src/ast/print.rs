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
    fn print(self, out: &mut crate::buffer::Writer, arena: &crate::ast::Arena, nested: bool) {
        arena.get(self).print(out, arena, nested)
    }
}

impl crate::ast::ExprListId {
    fn print(self, out: &mut crate::buffer::Writer, arena: &crate::ast::Arena, enclosed: bool) {
        if enclosed {
            out.write_str("(");
        }

        for (i, expr) in arena.get_list(self).iter().enumerate() {
            if i > 0 {
                out.write_str(", ");
            }

            expr.print(out, arena, false);
        }

        if enclosed {
            out.write_str(")");
        }
    }
}

impl crate::ast::Expr {
    fn print(&self, out: &mut crate::buffer::Writer<'_>, arena: &crate::ast::Arena, nested: bool) {
        use crate::ast::{BinOp, Operator};

        // macro_rules! nested_expr {
        //     {$($stmt:stmt)*} => {{
        //         if nested {
        //             out.write_str('(')?;
        //         }
        //
        //         $($stmt)*
        //
        //         if nested {
        //             out.write_str(')')?;
        //         }
        //     }};
        // }

        match self {
            Self::Literal(literal) => literal.print(out),
            Self::Operator(op) => match op {
                Operator::Binary { kind, c_1, c_2 } => {
                    macro_rules! bin_op {
                        ($name:literal) => {{
                            out.write_str(concat!($name, "("));
                            c_1.print(out, arena, false);
                            out.write_str(", ");
                            c_2.print(out, arena, false);
                            out.write_str(")");
                        }};
                    }

                    match *kind {
                        BinOp::I32Add => bin_op!("i32::wrapping_add"),
                        BinOp::I64Add => bin_op!("i64::wrapping_add"),
                    }
                }
            },
            Self::GetLocal(local) => write!(out, "{local}"),
            Self::Call { callee, arguments } => {
                todo!("cannot generate call, need to figure out if self should be passed")
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
                    expr.print(out, arena, false);
                    out.write_str(";");
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

                    results.print(out, arena, results.len() != 1);

                    if calling_convention.can_unwind() {
                        out.write_str(")");
                    }

                    if is_last {
                        out.write_str(";");
                    }
                }
                Statement::Unreachable { function, offset } => {
                    writeln!(
                        out,
                        "return ::core::result::Err(embedder::Trap::with_code());"
                    );
                }
            }

            out.write_str("\n");
        }
    }
}
