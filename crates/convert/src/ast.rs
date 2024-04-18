//! Contains types modeling a Rust-like syntax tree representing a WebAssembly function body.

mod arena;
mod print;

pub use print::Indentation;

pub(crate) use arena::{Arena, ArenaError, ExprId, ExprListId};
pub(crate) use print::Print;

/// Represents a WebAssembly [function index].
///
/// [function index]: https://webassembly.github.io/spec/core/syntax/modules.html#indices
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FuncId(pub(crate) u32);

impl std::fmt::Display for FuncId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_f{}", self.0)
    }
}

/// Represents a WebAssembly local variable in a function body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LocalId(pub(crate) u32);

impl std::fmt::Display for LocalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_l{}", self.0)
    }
}

pub(crate) enum ValType {
    I32,
    I64,
    F32,
    F64,
}

impl From<wasmparser::ValType> for ValType {
    fn from(ty: wasmparser::ValType) -> Self {
        use wasmparser::ValType;

        match ty {
            ValType::I32 => Self::I32,
            ValType::I64 => Self::I64,
            ValType::F32 => Self::F32,
            ValType::F64 => Self::F64,
            _ => todo!("{ty:?} is not yet supported"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Literal {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinOp {
    /// Wrapping addition on `i32`s (`c_1 + c_2`).
    I32Add,
    // /// `c_1 / c_2`
    // I32Div { c_1: ExprId, c_2: ExprId },
    /// Wrapping addition on `i64`s (`c_1 + c_2`).
    I64Add,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum Expr {
    Literal(Literal),
    Operator(Operator),
    GetLocal(LocalId),
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
