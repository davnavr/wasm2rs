//! Traits and functions to support [WebAssembly traps].
//!
//! [WebAssembly traps]: https://webassembly.github.io/spec/core/intro/overview.html#trap

mod trap_value;

pub use trap_value::TrapValue;

/// Indicates why a trap occured.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TrapCode {
    /// An [**`unreachable`**] instruction was executed.
    ///
    /// [**`unreachable`**]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-control
    Unreachable,
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
    //NullReference
    /// Instantiating a module failed because linear memory could not be allocated.
    MemoryInstantiation {
        /// The index of the memory that could not be instantiated.
        memory: u32,
        /// The error describing why the memory could not be allocated.
        error: crate::memory::AllocationError,
    },
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
            Self::MemoryBoundsCheck {
                source,
                memory,
                address,
            } => write!(f, "at address {address:#X} into memory #{memory}: {source}"),
            Self::IntegerDivisionByZero => f.write_str("integer division by zero"),
            Self::IntegerOverflow => f.write_str("integer overflow"),
            Self::MemoryInstantiation { memory, error } => {
                write!(f, "instantiation of memory #{memory} failed: {error}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TrapCode {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MemoryBoundsCheck { source, .. } => Some(source),
            Self::MemoryInstantiation { error, .. } => Some(error),
            _ => None,
        }
    }
}

//struct TrapReason { byte_offset: Option<u64>, code: TrapCode, dwarf: Option<(&'static str, u32, u32)> }

/// Trait for implementing WebAssembly traps.
pub trait Trap {
    /// The type used to describe the WebAssembly trap.
    type Repr: core::fmt::Debug;

    /// Generates a trap with the given reason.
    ///
    /// The `wasm2rs` compiler generates calls to this function for instructions that generate a
    /// trap.
    fn trap(&self, code: TrapCode) -> Self::Repr;
}

impl<T: Trap + ?Sized> Trap for &T {
    type Repr = T::Repr;

    fn trap(&self, code: TrapCode) -> Self::Repr {
        <T>::trap(self, code)
    }
}
