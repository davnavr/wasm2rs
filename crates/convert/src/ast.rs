//! Contains types modeling a Rust-like syntax tree representing a WebAssembly function body.

mod arena;

pub(crate) use arena::{Arena, ArenaError, ExprId, ExprListId};

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

impl core::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32(i) => write!(f, "{i:#010X}i32"),
            Self::I64(i) => write!(f, "{i:#018X}i64"),
            Self::F32(z) => write!(f, "::core::primitive::f32::from_bits({z:#010X})"),
            Self::F64(z) => write!(f, "::core::primitive::f64::from_bits({z:#018X})"),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Operator {
    /// `c_1 + c_2` (wrapping)
    I32Add { c_1: ExprId, c_2: ExprId },
    // /// `c_1 / c_2`
    // I32Div { c_1: ExprId, c_2: ExprId },
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
    Expr(ExprId),
    Return(ExprListId),
}
