//! Types and functions for generating Rust unit test functions from `.wast` directives.

mod builder;
mod writer;

pub use builder::Builder;
pub use writer::write_unit_tests;

#[derive(Clone, Copy)]
enum ArgumentValue {
    I32(i32),
    I64(i64),
    //F32(f32),
}

/// Renders the argument as a Rust expression.
impl std::fmt::Display for ArgumentValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32(i) => write!(f, "{i}i32"),
            Self::I64(i) => write!(f, "{i}i64"),
        }
    }
}

impl TryFrom<wast::WastArg<'_>> for ArgumentValue {
    type Error = crate::Error;

    fn try_from(arg: wast::WastArg<'_>) -> crate::Result<Self> {
        use wast::{core::WastArgCore, WastArg};

        Ok(match arg {
            WastArg::Core(arg) => match arg {
                WastArgCore::I32(i) => Self::I32(i),
                WastArgCore::I64(i) => Self::I64(i),
                bad => anyhow::bail!("argument {bad:?} is currently unsupported"),
            },
            WastArg::Component(value) => anyhow::bail!("unsupported argument {value:?}"),
        })
    }
}

struct Arguments {
    arguments: Vec<ArgumentValue>,
}

impl TryFrom<Vec<wast::WastArg<'_>>> for Arguments {
    type Error = crate::Error;

    fn try_from(args: Vec<wast::WastArg<'_>>) -> crate::Result<Self> {
        args.into_iter()
            .map(ArgumentValue::try_from)
            .collect::<crate::Result<_>>()
            .map(|arguments| Self { arguments })
    }
}

/// Renders the arguments as a list of values within the parenthesis of a Rust function call.
impl std::fmt::Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("(")?;
        for (i, arg) in self.arguments.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }

            write!(f, "{arg}")?;
        }
        f.write_str(")")
    }
}

#[derive(Clone, Copy)]
enum ResultValue {
    I32(i32),
    I64(i64),
}

impl ResultValue {
    fn try_convert_vec(results: Vec<wast::WastRet<'_>>) -> crate::Result<Vec<Self>> {
        results.into_iter().map(Self::try_from).collect()
    }
}

/// Renders the result value as a Rust pattern.
impl std::fmt::Display for ResultValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::I32(i) => write!(f, "{i}i32"),
            Self::I64(i) => write!(f, "{i}i64"),
        }
    }
}

impl TryFrom<wast::WastRet<'_>> for ResultValue {
    type Error = crate::Error;

    fn try_from(result: wast::WastRet<'_>) -> crate::Result<Self> {
        use wast::{core::WastRetCore, WastRet};

        Ok(match result {
            WastRet::Core(result) => match result {
                WastRetCore::I32(i) => Self::I32(i),
                WastRetCore::I64(i) => Self::I64(i),
                bad => anyhow::bail!("result {bad:?} is currently unsupported"),
            },
            WastRet::Component(value) => anyhow::bail!("unsupported result {value:?}"),
        })
    }
}

enum TrapReason {
    IntegerDivideByZero,
    IntegerOverflow,
    OutOfBoundsMemoryAccess,
}

impl std::str::FromStr for TrapReason {
    type Err = crate::Error;

    fn from_str(message: &str) -> crate::Result<Self> {
        Ok(match message {
            "integer divide by zero" => Self::IntegerDivideByZero,
            "integer overflow" => Self::IntegerOverflow,
            "out of bounds memory access" => Self::OutOfBoundsMemoryAccess,
            _ => anyhow::bail!("unrecognized trap message {message:?}"),
        })
    }
}

/// Renders the result as a Rust pattern matching a `wasm2rs_rt::trap::TrapCode`.
impl std::fmt::Display for TrapReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("::wasm2rs_rt::trap::TrapCode::")?;
        match self {
            Self::IntegerDivideByZero => f.write_str("IntegerDivisionByZero"),
            Self::IntegerOverflow => f.write_str("IntegerOverflow"),
            Self::OutOfBoundsMemoryAccess => f.write_str("MemoryBoundsCheck { .. }"),
        }
    }
}

enum ActionResult {
    Values(Vec<ResultValue>),
    Trap(TrapReason),
}

/// Renders the result as a Rust pattern.
impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Values(values) => match values.as_slice() {
                [] => f.write_str("Ok(())"),
                [one] => write!(f, "Ok({one})"),
                _ => {
                    f.write_str("Ok((")?;
                    for (i, value) in values.iter().enumerate() {
                        if i > 0 {
                            f.write_str(", ")?;
                        }

                        write!(f, "{value}")?;
                    }
                    f.write_str("))")
                }
            },
            Self::Trap(trap) => std::fmt::Display::fmt(trap, f),
        }
    }
}

enum StatementKind<'wasm> {
    /// Emits a Rust function call, storing the return values into a variable named
    /// [`Statement::RESULT_VARIABLE`].
    InvokeFunction {
        /// Name of the exported function to call.
        name: &'wasm str,
        arguments: Arguments,
        /// If `Some`, indicates that the result is checked with an assertion.
        /// If `None`, then the function is called, but the return value is ignored.
        result: Option<ActionResult>,
    },
}

struct Statement<'wasm> {
    kind: StatementKind<'wasm>,
    /// Refers to the location in the original `.wast` file that this [`Statement`] was
    /// generated from.
    span: wast::token::Span,
}

impl Statement<'_> {
    /// Name of the variable used to store the results of executing a [`Statement`].
    const RESULT_VARIABLE: &'static str = "_result";
}

pub struct Module<'wasm> {
    number: usize,
    id: Option<&'wasm str>,
    span: wast::token::Span,
    statements: Vec<Statement<'wasm>>,
}

pub enum ModuleIdent<'wasm> {
    Numbered(usize),
    Named(wasm2rs::rust::AnyIdent<'wasm>),
}

impl<'wasm> Module<'wasm> {
    pub fn into_ident(&self) -> ModuleIdent<'wasm> {
        if let Some(id) = self.id {
            ModuleIdent::Named(if let Some(valid) = wasm2rs::rust::Ident::new(id) {
                valid.into()
            } else {
                wasm2rs::rust::MangledIdent(id).into()
            })
        } else {
            ModuleIdent::Numbered(self.number)
        }
    }
}

impl std::fmt::Display for ModuleIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numbered(n) => write!(f, "module_{n}"),
            Self::Named(named) => std::fmt::Display::fmt(named, f),
        }
    }
}
