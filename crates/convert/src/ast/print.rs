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
                    Err(e) => panic!("e"),
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

impl crate::ast::Literal {
    fn print(&self, out: &mut dyn std::fmt::Write) -> std::fmt::Result {
        match self {
            Self::I32(i) => write!(out, "{i:#010X}i32"),
            Self::I64(i) => write!(out, "{i:#018X}i64"),
            Self::F32(z) => write!(out, "::core::primitive::f32::from_bits({z:#010X})"),
            Self::F64(z) => write!(out, "::core::primitive::f64::from_bits({z:#018X})"),
        }
    }
}

impl crate::ast::ExprId {
    fn print(
        self,
        out: &mut dyn std::fmt::Write,
        arena: &crate::ast::Arena,
        nested: bool,
    ) -> std::fmt::Result {
        arena.get(self).print(out, arena, nested)
    }
}

impl crate::ast::ExprListId {
    fn print(
        self,
        out: &mut dyn std::fmt::Write,
        arena: &crate::ast::Arena,
        enclosed: bool,
    ) -> std::fmt::Result {
        if enclosed {
            out.write_char('(')?;
        }

        for (i, expr) in arena.get_list(self).iter().enumerate() {
            if i > 0 {
                out.write_str(", ")?;
            }

            expr.print(out, arena, false)?;
        }

        if enclosed {
            out.write_char(')')?;
        }

        Ok(())
    }
}

impl crate::ast::Expr {
    fn print(
        &self,
        out: &mut dyn std::fmt::Write,
        arena: &crate::ast::Arena,
        nested: bool,
    ) -> std::fmt::Result {
        use crate::ast::{BinOp, Operator};

        // macro_rules! nested_expr {
        //     {$($stmt:stmt)*} => {{
        //         if nested {
        //             out.write_char('(')?;
        //         }
        //
        //         $($stmt)*
        //
        //         if nested {
        //             out.write_char(')')?;
        //         }
        //     }};
        // }

        match self {
            Self::Literal(literal) => literal.print(out)?,
            Self::Operator(op) => match op {
                Operator::Binary { kind, c_1, c_2 } => {
                    macro_rules! bin_op {
                        ($name:literal) => {{
                            out.write_str(concat!($name, "("))?;
                            c_1.print(out, arena, false)?;
                            out.write_str(", ")?;
                            c_2.print(out, arena, false)?;
                            out.write_char(')')?;
                        }};
                    }

                    match *kind {
                        BinOp::I32Add => bin_op!("::core::primitive::i32::wrapping_add"),
                        BinOp::I64Add => bin_op!("::core::primitive::i64::wrapping_add"),
                    }
                }
            },
            Self::Call { callee, arguments } => {
                todo!("cannot generate call, need to figure out if self should be passed")
            }
        }

        Ok(())
    }
}

pub(crate) struct Print<'a> {
    arena: &'a crate::ast::Arena,
    indentation: Indentation,
}

impl<'a> Print<'a> {
    pub(crate) const fn new(arena: &'a crate::ast::Arena, indentation: Indentation) -> Self {
        Self { arena, indentation }
    }

    pub(crate) fn print_statements(
        &self,
        statements: &[crate::ast::Statement],
        out: &mut dyn std::fmt::Write,
    ) -> std::fmt::Result {
        use crate::ast::Statement;

        let mut indent_level = 0usize;
        for (n, stmt) in statements.iter().copied().enumerate() {
            let is_last = n == statements.len() - 1;

            for _ in 0..indent_level {
                out.write_str(self.indentation.to_str())?;
            }

            match stmt {
                Statement::Expr(expr) => {
                    expr.print(out, self.arena, false)?;
                    out.write_char(';')?;
                }
                Statement::Return(results) => {
                    if is_last {
                        out.write_str("return")?;

                        if !results.is_empty() {
                            out.write_char(' ')?;
                        }
                    }

                    results.print(out, self.arena, results.len() != 1)?;

                    if is_last {
                        out.write_char(';')?;
                    }
                }
            }

            out.write_char('\n')?;
        }

        Ok(())
    }
}
