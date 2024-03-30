//! Traits and functions to support [WebAssembly traps].
//!
//! [WebAssembly traps]: https://webassembly.github.io/spec/core/intro/overview.html#trap

mod trap_value;

pub use crate::stack::trace::WasmStackTraceFrame;
pub use trap_value::TrapValue;

/// Describes which limits a memory or table did not match.
///
/// For memories, the limits are expressed as the number of pages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum LimitsCheck {
    Minimum { expected: u32, actual: u32 },
    Maximum { expected: u32, actual: u32 },
}

impl core::fmt::Display for LimitsCheck {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("expected ")?;

        match self {
            Self::Minimum { .. } => f.write_str("minimum")?,
            Self::Maximum { .. } => f.write_str("maximum")?,
        }

        let (Self::Minimum { expected, actual } | Self::Maximum { expected, actual }) = self;

        write!(f, " of {expected}, but got {actual}")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LimitsCheck {}

/// Indicates why a trap occured.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TrapCode {
    /// An [**`unreachable`**] instruction was executed.
    ///
    /// [**`unreachable`**]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-control
    Unreachable,
    /// An attempt to convert a float value to an integer failed.
    ConversionToInteger,
    /// A memory access was out of bounds.
    MemoryBoundsCheck {
        /// The originating invalid memory access.
        source: crate::memory::AccessError,
        /// The index of the [linear memory] with which the access occured.
        ///
        /// [linear memory]: crate::memory::Memory32
        memory: u32,
        /// The address that was out-of-bounds or misaligned.
        address: u64,
    },
    /// An integer operation attempted a division by zero.
    IntegerDivisionByZero,
    /// An integer operation overflowed.
    IntegerOverflow,
    //UnalignedAtomicOperation
    /// A function reference did not have the correct signature.
    IndirectCallSignatureMismatch(crate::func_ref::SignatureMismatchError),
    /// A function reference was null.
    NullFunctionReference {
        /// The type that the function reference was expected to have.
        expected: Option<&'static crate::func_ref::FuncRefSignature>,
    },
    /// Instantiating a module failed because linear memory could not be allocated.
    MemoryAllocation {
        /// The index of the memory that could not be allocated.
        memory: u32,
        /// The error describing why the memory could not be allocated.
        error: crate::memory::AllocationError,
    },
    /// Instantiating a module failed because a linear memory did not have matching [`limits`].
    ///
    /// [`limits`]: crate::memory::Memory32::limit
    MemoryLimitsCheck {
        /// The index of the memory whose limits did not match.
        memory: u32,
        /// Describes which limit the memory did not match.
        limits: LimitsCheck,
    },
    /// The stack space was exhausted, usually due to an infinitely recursive function.
    ///
    /// See the documentation for [`Trap::trap_stack_overflow()`] for more information.
    CallStackExhausted,
}

impl core::cmp::PartialEq<TrapCode> for &TrapCode {
    fn eq(&self, other: &TrapCode) -> bool {
        <TrapCode as core::cmp::PartialEq<TrapCode>>::eq(*self, other)
    }
}

impl core::cmp::PartialEq<&TrapCode> for TrapCode {
    fn eq(&self, other: &&TrapCode) -> bool {
        <Self as core::cmp::PartialEq<Self>>::eq(self, *other)
    }
}

impl core::fmt::Display for TrapCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unreachable => f.write_str("executed unreachable instruction"),
            Self::ConversionToInteger => f.write_str("invalid conversion to integer"),
            Self::MemoryBoundsCheck {
                source,
                memory,
                address,
            } => write!(f, "at address {address:#X} into memory #{memory}: {source}"),
            Self::IntegerDivisionByZero => f.write_str("integer division by zero"),
            Self::IntegerOverflow => f.write_str("integer overflow"),
            Self::IndirectCallSignatureMismatch(error) => write!(f, "function reference {error}"),
            Self::NullFunctionReference { expected } => {
                if let Some(signature) = expected {
                    write!(f, "expected signature {signature:?} for ")?;
                }

                f.write_str("null function reference")
            }
            Self::MemoryAllocation { memory, error } => {
                write!(f, "{error} #{memory}")
            }
            Self::MemoryLimitsCheck { memory, limits } => {
                write!(f, "{limits} pages in memory #{memory}")
            }
            Self::CallStackExhausted => f.write_str("call stack exhausted"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TrapCode {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MemoryBoundsCheck { source, .. } => Some(source),
            Self::IndirectCallSignatureMismatch(error) => Some(error),
            Self::MemoryAllocation { error, .. } => Some(error),
            Self::MemoryLimitsCheck { limits, .. } => Some(limits),
            _ => None,
        }
    }
}

//struct TrapReason { byte_offset: Option<u64>, code: TrapCode, dwarf: Option<(&'static str, u32, u32)> }

/// Trait for implementing WebAssembly traps.
pub trait Trap {
    /// The type used to describe the WebAssembly trap.
    type Repr: core::fmt::Debug + crate::stack::trace::WasmTrace;

    /// Generates a trap with the given reason and an optional WebAssembly stack frame indicating
    /// the source of the trap in the original WebAssembly function.
    ///
    /// The `wasm2rs` compiler generates calls to this function for instructions that generate a
    /// trap.
    fn trap(&self, code: TrapCode, frame: Option<&'static WasmStackTraceFrame>) -> Self::Repr;

    /// Generates a trap due to a stack overflow condition.
    ///
    /// This function is called by the [`stack::check_for_overflow()`] function if it believes a
    /// stack overflow may occur. This function should avoid allocating too much space on the stack
    /// in order to avoid aborting the process on stack overflow.
    ///
    /// [`stack::check_for_overflow()`]: crate::stack::check_for_overflow()
    #[inline(always)]
    fn trap_stack_overflow(&self) -> Self::Repr {
        self.trap(TrapCode::CallStackExhausted, None)
    }
}

impl<T: Trap + ?Sized> Trap for &T {
    type Repr = T::Repr;

    fn trap(&self, code: TrapCode, frame: Option<&'static WasmStackTraceFrame>) -> Self::Repr {
        <T>::trap(self, code, frame)
    }
}

/// Implements the [`unreachable`] instruction.
///
/// [`unreachable`]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-control
#[inline(never)]
#[cold]
pub fn unreachable<E>(trap: &dyn Trap<Repr = E>, frame: &'static WasmStackTraceFrame) -> E
where
    E: core::fmt::Debug + crate::stack::trace::WasmTrace,
{
    trap.trap(TrapCode::Unreachable, Some(frame))
}
