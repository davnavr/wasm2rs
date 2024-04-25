//! Types describing functions in WebAssembly.

/// Describes a WebAssembly [function import].
///
/// [function import]: https://webassembly.github.io/spec/core/syntax/modules.html#imports
#[derive(Clone, Copy, Debug)]
#[allow(clippy::exhaustive_structs)]
pub struct WasmImportSymbol {
    /// The name of the module that the function was imported from.
    pub module: &'static str,
    /// The name of the function import.
    pub name: &'static str,
}

impl core::fmt::Display for WasmImportSymbol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "(import \"{}\" \"{}\")",
            self.module.escape_default(),
            self.name.escape_default()
        )
    }
}

/// Describes the type of a function parameter or result in a [`WasmSymbolSignature`].
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum WasmValType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
}

impl core::fmt::Display for WasmValType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::V128 => "v128",
            Self::FuncRef => "funcref",
            Self::ExternRef => "externref",
        })
    }
}

/// Describes the parameter and return types of a WebAssembly function.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::exhaustive_structs)]
pub struct WasmSymbolSignature {
    pub parameters: &'static [WasmValType],
    pub results: &'static [WasmValType],
}

impl core::fmt::Display for WasmSymbolSignature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("(param")?;

        for ty in self.parameters {
            write!(f, " {ty}")?;
        }

        f.write_str(") (result")?;

        for ty in self.results {
            write!(f, " {ty}")?;
        }

        f.write_str(")")
    }
}

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum WasmSymbolKind {
    Imported(&'static WasmImportSymbol),
    Defined {
        /// An byte offset from the start of the WebAssembly module to the [code section entry]
        /// corresponding to the WebAssembly function.
        ///
        /// [code section entry]: https://webassembly.github.io/spec/core/binary/modules.html#code-section
        offset: u64,
    },
}

/// Represents information about a WebAssembly function that was translated by `wasm2rs`.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct WasmSymbol {
    //module: &'static str,
    /// A list of the names that the function was [exported] with. May be empty if the function is
    /// not exported.
    ///
    /// [exported]: https://webassembly.github.io/spec/core/syntax/modules.html#exports
    pub export_names: &'static [&'static str],
    /// Specifies the parameter and return types of the function.
    pub signature: &'static WasmSymbolSignature,
    /// The [index] of the function in the WebAssembly module.
    ///
    /// [index]: https://webassembly.github.io/spec/core/syntax/modules.html#indices
    pub index: u32,
    /// Contains information describing the function import or definition.
    pub kind: WasmSymbolKind,
    /// An additional [custom name] given to the function.
    ///
    /// [custom name]: https://webassembly.github.io/spec/core/appendix/custom.html#function-names
    pub custom_name: Option<&'static str>,
}

impl WasmSymbol {
    /// Creates a new [`WasmSymbol`].
    pub const fn new(
        index: u32,
        signature: &'static WasmSymbolSignature,
        kind: WasmSymbolKind,
    ) -> Self {
        Self {
            export_names: &[],
            signature,
            index,
            kind,
            custom_name: None,
        }
    }
}

impl core::fmt::Display for WasmSymbol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "(func (;{};)", self.index)?;

        if let Some(custom_name) = self.custom_name {
            write!(f, " ${custom_name}")?;
        }

        for name in self.export_names {
            write!(f, " (export \"{}\")", name.escape_default())?;
        }

        if let WasmSymbolKind::Imported(import) = self.kind {
            write!(f, " {import}")?;
        }

        write!(f, " {}", self.signature)
    }
}
