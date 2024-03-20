//! Helper structs to display WebAssembly types and ids as Rust keywords or identifiers.

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct ValType(pub(in crate::translation) wasmparser::ValType);

impl std::fmt::Display for ValType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            wasmparser::ValType::I32 => f.write_str("i32"),
            wasmparser::ValType::I64 => f.write_str("i64"),
            other => todo!("how to write {other}?"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct LocalId(pub(in crate::translation) u32);

impl std::fmt::Display for LocalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_l_{}", self.0)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct FuncId(pub(in crate::translation) u32);

impl std::fmt::Display for FuncId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_f_{}", self.0)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct MemId(pub(in crate::translation) u32);

impl std::fmt::Display for MemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_mem_{}", self.0)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct DataId(pub(in crate::translation) u32);

impl std::fmt::Display for DataId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_DATA_{}", self.0)
    }
}
