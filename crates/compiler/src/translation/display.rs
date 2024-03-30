//! Helper structs to display WebAssembly types and ids as Rust keywords or identifiers.

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct ValType(pub(in crate::translation) wasmparser::ValType);

impl ValType {
    pub(in crate::translation) const I32: Self = Self(wasmparser::ValType::I32);
    pub(in crate::translation) const I64: Self = Self(wasmparser::ValType::I64);
}

impl std::fmt::Display for ValType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            wasmparser::ValType::I32 => f.write_str("i32"),
            wasmparser::ValType::I64 => f.write_str("i64"),
            wasmparser::ValType::F32 => f.write_str("f32"),
            wasmparser::ValType::F64 => f.write_str("f64"),
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
pub(in crate::translation) struct FuncSymbol(pub(in crate::translation) u32);

impl std::fmt::Display for FuncSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_F{}_SYMBOL", self.0)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct FuncSignature(pub(in crate::translation) u32);

impl std::fmt::Display for FuncSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_F{}_SIG", self.0)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(in crate::translation) struct FuncExportSymbols(pub(in crate::translation) u32);

impl std::fmt::Display for FuncExportSymbols {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_F{}_EXPORT_NAMES", self.0)
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
pub(in crate::translation) struct GlobalId(pub(in crate::translation) u32);

impl std::fmt::Display for GlobalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_gbl_{}", self.0)
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
