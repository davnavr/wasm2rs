//! Contains types modeling a Rust-like syntax tree representing a WebAssembly function body.

mod arena;
mod print;

pub use print::Indentation;

pub(crate) use arena::{Arena, ArenaError, ExprId, ExprListId};
pub(crate) use print::Print;

/// Represents a WebAssembly [function index].
///
/// [function index]: https://webassembly.github.io/spec/core/syntax/modules.html#indices
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct FuncId(pub(crate) u32);

#[derive(Clone, Copy)]
pub(crate) enum Literal {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum BinOp {
    /// Wrapping addition on `i32`s (`c_1 + c_2`).
    I32Add,
    // /// `c_1 / c_2`
    // I32Div { c_1: ExprId, c_2: ExprId },
    /// Wrapping addition on `i64`s (`c_1 + c_2`).
    I64Add,
}

#[derive(Clone, Copy)]
pub(crate) enum Operator {
    /// Represents instructions of the form [*t.binop*] (`binop(c_1, c_2)`).
    ///
    /// [*t.binop*]: https://webassembly.github.io/spec/core/exec/instructions.html#exec-instr-numeric
    Binary {
        kind: BinOp,
        c_1: ExprId,
        c_2: ExprId,
    },
}

macro_rules! from_conversions {
    ($($src:ident => $dst:ty;)*) => {$(
        impl From<$src> for $dst {
            fn from(value: $src) -> $dst {
                <$dst>::$src(value)
            }
        }
    )*};
}

#[derive(Clone, Copy)]
pub(crate) enum Expr {
    Literal(Literal),
    Operator(Operator),
    Call {
        callee: FuncId,
        arguments: ExprListId,
    },
}

from_conversions! {
    Literal => Expr;
    Operator => Expr;
}

#[derive(Clone, Copy)]
pub(crate) enum Statement {
    /// An expression that is evaluated, with any results discarded.
    Expr(ExprId),
    /// Expressions that are evaluated, and used as the return values for the function.
    Return(ExprListId),
}
