//! Traits and functions to support [WebAssembly traps].
//!
//! [WebAssembly traps]: https://webassembly.github.io/spec/core/intro/overview.html#trap

/// Describes what kind of value was being read or written in a [`MemoryAccess`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum MemoryAccessKind {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    V128,
    /// Used for other memory instructions (e.g. **`memory.copy`** or **`memory.fill`**).
    Other,
}

/// Describes a memory access that resulted in a trap.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MemoryAccess {
    #[allow(missing_docs)]
    pub kind: MemoryAccessKind,
    /// The index of the memory into which the access occured.
    pub memory: u32,
    /// The address that was out-of-bounds or misaligned.
    pub address: u64,
    /// The size of the [`memory`], in bytes, at the time the access occured.
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

        f.write_str("read/write")?;

        if self.kind != MemoryAccessKind::Other {
            f.write_str(" of ")?;
            match self.kind {
                MemoryAccessKind::I8 => f.write_str("i8")?,
                MemoryAccessKind::I16 => f.write_str("i16")?,
                MemoryAccessKind::I32 => f.write_str("i32")?,
                MemoryAccessKind::I64 => f.write_str("i64")?,
                MemoryAccessKind::F32 => f.write_str("f32")?,
                MemoryAccessKind::F64 => f.write_str("f64")?,
                MemoryAccessKind::V128 => f.write_str("v128")?,
                MemoryAccessKind::Other => (),
            }
        }

        write!(
            f,
            " at address {} into memory #{} with size {}",
            Address(self.address),
            self.memory,
            Address(self.bound)
        )
    }
}

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
    //UnalignedAtomicOperation
    //NullReference
}

impl core::fmt::Display for TrapCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unreachable => f.write_str("executed unreachable instruction"),
            Self::MemoryBoundsCheck(access) => write!(f, "out-of-bounds {}", access),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TrapCode {}

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
