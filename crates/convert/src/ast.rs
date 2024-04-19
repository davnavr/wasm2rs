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

#[derive(Clone, Copy, Debug)]
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
pub(crate) enum UnOp {
    /// Compares an integer value to `0`. Corresponds to the `i32.eqz` and `i64.eqz` instuctions.
    IxxEqz,
    I32Clz,
    I64Clz,
    I32Ctz,
    I64Ctz,
    I32Popcnt,
    I64Popcnt,
    /// Corresponds to the `f32.neg` and `f64.neg` instructions.
    FxxNeg,
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertIxxS,
    F32ConvertI32U,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertIxxS,
    F64ConvertI32U,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,
    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,
    I32TruncSatFxxS,
    I32TruncSatFxxU,
    I64TruncSatFxxS,
    I64TruncSatFxxU,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinOp {
    /// Equality comparison (`c_1 == c_2`). Corresponds to the `i32.eq`, `i64.eq`, `f32.eq`, and
    /// `f64.eq` instructions.
    Eq,
    Ne,
    /// Signed integer comparison (`c_1 < c_2`). Corresponds to the `i32.lt_s` and `i64.lt_s`
    /// instructions.
    IxxLtS,
    IxxGtS,
    I32LtU,
    I32GtU,
    I64LtU,
    I64GtU,
    IxxLeS,
    IxxGeS,
    I32LeU,
    I32GeU,
    I64LeU,
    I64GeU,
    FxxGt,
    /// Wrapping addition on `i32`s (`c_1 + c_2`).
    I32Add,
    /// Wrapping addition on `i64`s (`c_1 + c_2`).
    I64Add,
    I32Sub,
    I64Sub,
    I32Mul,
    I64Mul,
    /// Signed division on `i32`s, trapping when the denominator is `0` (`c_1 / c_2`). Corresponds
    /// to the `i32.div_s` instruction.
    I32DivS,
    I64DivS,
    /// Signed division on `i64`s, trapping when the denominator is `0` (`c_1 / c_2`). Corresponds
    /// to the `i64.div_u` instruction.
    I32DivU,
    I64DivU,
    I32RemS,
    I64RemS,
    I32RemU,
    I64RemU,
    /// Bitwise integer AND (`c_1 & c_2`). Corresponds to the `i32.and` and `i64.and` instructions.
    IxxAnd,
    /// Bitwise integer OR (`c_1 | c_2`). Corresponds to the `i32.or` and `i64.or` instructions.
    IxxOr,
    /// Bitwise integer XOR (`c_1 | c_2`). Corresponds to the `i32.xor` and `i64.xor` instructions.
    IxxXor,
    I32Shl,
    I64Shl,
    I32ShrS,
    I64ShrS,
    I32ShrU,
    I64ShrU,
    I32Rotl,
    I64Rotl,
    I32Rotr,
    I64Rotr,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Expr {
    Literal(Literal),
    /// Represents instructions of the form [*t.unop*] (`unop(c_1)`).
    ///
    /// [*t.unop*]: https://webassembly.github.io/spec/core/exec/instructions.html#exec-instr-numeric
    UnaryOperator {
        kind: UnOp,
        c_1: ExprId,
    },
    /// Represents instructions of the form [*t.binop*] (`binop(c_1, c_2)`).
    ///
    /// [*t.binop*]: https://webassembly.github.io/spec/core/exec/instructions.html#exec-instr-numeric
    BinaryOperator {
        kind: BinOp,
        c_1: ExprId,
        c_2: ExprId,
    },
    GetLocal(LocalId),
    Call {
        callee: FuncId,
        arguments: ExprListId,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Statement {
    /// An expression that is evaluated, with any results discarded.
    Expr(ExprId),
    /// Expressions that are evaluated, and used as the return values for the function.
    Return(ExprListId),
    /// Defines a local variable. These statements should be placed at the start of the function.
    ///
    /// These correspond to the local variables of a WebAssembly code section entry.
    LocalDefinition(LocalId, ValType),
    /// Assigns a value to a local variable. Corresponds to the [`local.set`] instruction.
    ///
    /// [`local.set`]: https://webassembly.github.io/spec/core/syntax/instructions.html#variable-instructions
    LocalSet { local: LocalId, value: ExprId },
    /// Corresponds to the [`unreachable`] instruction, which always produces a trap.
    ///
    /// [`unreachable`]: https://webassembly.github.io/spec/core/syntax/instructions.html#control-instructions
    Unreachable {
        function: FuncId,
        /// An offset from the start of the code section entry of the function to the `unreachable`
        /// instruction.
        offset: u32,
    },
}

macro_rules! from_conversions {
    ($($src:ident => $dst:ident::$case:ident;)*) => {$(
        impl From<$src> for $dst {
            fn from(value: $src) -> $dst {
                <$dst>::$case(value)
            }
        }
    )*};
}

from_conversions! {
    Literal => Expr::Literal;
    ExprId => Statement::Expr;
}
