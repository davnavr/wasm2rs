//! Traits and functions to support [WebAssembly traps].
//!
//! [WebAssembly traps]: https://webassembly.github.io/spec/core/intro/overview.html#trap

/// Describes a memory access that resulted in a trap.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MemoryAccess {
    /// The type of the value that the `address` refers to.
    pub pointee: crate::memory::MemoryAccessPointee,
    /// The index of the [linear memory] with which the access occured.
    ///
    /// [linear memory]: crate::memory::Memory32
    pub memory: u32,
    /// The address that was out-of-bounds or misaligned.
    pub address: u64,
    /// The size of the [`memory`], in bytes, at the time the access occured.
    ///
    /// [`memory`]: MemoryAccess::memory
    pub bound: u64,
}

impl core::fmt::Display for MemoryAccess {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct Address(u64);

        impl core::fmt::Display for Address {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                if let Ok(address) = u32::try_from(self.0) {
                    write!(f, "{address:#010X}")
                } else {
                    write!(f, "{:#018X}", self.0)
                }
            }
        }

        write!(
            f,
            "read/write of {} at address {} into memory #{} with size {}",
            self.pointee,
            Address(self.address),
            self.memory,
            Address(self.bound)
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MemoryAccess {}

/// Indicates why a trap occured.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TrapCode {
    /// An [**`unreachable`**] instruction was executed.
    ///
    /// [**`unreachable`**]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-control
    Unreachable,
    /// A memory access was out of bounds.
    MemoryBoundsCheck(MemoryAccess),
    /// An integer or floating point operation attempted a division by zero.
    DivisionByZero,
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

impl core::fmt::Display for TrapCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unreachable => f.write_str("executed unreachable instruction"),
            Self::MemoryBoundsCheck(access) => write!(f, "out-of-bounds {}", access),
            Self::DivisionByZero => f.write_str("division by zero"),
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
            Self::MemoryBoundsCheck(access) => Some(access),
            Self::MemoryInstantiation { memory: _, error } => Some(error),
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
