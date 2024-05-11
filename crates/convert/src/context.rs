//! Types describing a WebAssembly module and the mapping of WebAssembly constructs to Rust.

use crate::ast::FuncId;

#[derive(Clone, Copy, Debug)]
pub(crate) enum CallKind {
    /// No additional argument is added. The generated Rust function is an associated
    /// function.
    Function,
    /// A `self` argument is added. The generated Rust function is a method.
    Method,
    // /// An additional argument is added to access embedder specific data.
    // ///
    // /// This is mainly used for accessing imports.
    // WithEmbedder,
    // /// The function only accesses a linear memory.
    // WithMemory(u32),
}

/// Describes whether a WebAssembly function may [trap] or throw an exception.
///
/// [trap]: https://webassembly.github.io/spec/core/intro/overview.html#trap
#[derive(Clone, Copy, Debug)]
pub(crate) enum UnwindKind {
    /// Calling the function is statically known to never result in a trap or an exception being
    /// thrown.
    Never,
    /// Calling the function *may* result in a trap or an exception being thrown.
    Maybe,
    /// Calling the function will always result in a trap or exception being thrown, meaning the
    /// function will never return normally.
    Always,
}

impl UnwindKind {
    pub(crate) fn can_unwind(self) -> bool {
        match self {
            Self::Never => false,
            Self::Always | Self::Maybe => true,
        }
    }
}

//pub(crate) enum Purity { Pure, ReadsMemory, Impure }

pub(crate) struct FunctionAttributes {
    /// Specifies how each WebAssembly function translated to Rust is invoked.
    pub(crate) call_kinds: Box<[CallKind]>,
    /// Describes how each WebAssembly function may unwind.
    pub(crate) unwind_kinds: Box<[UnwindKind]>,
}

impl FunctionAttributes {
    pub(crate) fn call_kind(&self, f: FuncId) -> CallKind {
        self.call_kinds[f.0 as usize]
    }

    pub(crate) fn unwind_kind(&self, f: FuncId) -> UnwindKind {
        self.unwind_kinds[f.0 as usize]
    }
}

/// Stores all information relating to a WebAssembly module and how it's components are accessed
/// when translated to Rust.
pub(crate) struct Context<'wasm> {
    pub(crate) types: wasmparser::types::Types,
    pub(crate) imported_modules: Box<[&'wasm str]>,
    /// Specifies the module each imported function originated from.
    pub(crate) func_import_modules: Box<[u16]>,
    /// Specifies the name of each WebAssembly function import.
    pub(crate) func_import_names: Box<[&'wasm str]>,
    pub(crate) function_attributes: FunctionAttributes,
    /// Correspodns to the [**start**] component of the WebAssembly module.
    ///
    /// [**start**]: https://webassembly.github.io/spec/core/syntax/modules.html#start-function
    pub(crate) start_function: Option<FuncId>,
}

impl Context<'_> {
    pub(crate) fn function_signature(&self, f: FuncId) -> &wasmparser::FuncType {
        self.types[self.types.core_function_at(f.0)].unwrap_func()
    }

    pub(crate) fn function_import_count(&self) -> usize {
        self.func_import_names.len()
    }
}

// #[derive(Clone, Copy, Debug)]
// pub(crate) enum GlobalValue<'a> {
//     Constant(crate::ast::Literal),
//     Defined {
//         value: crate::ast::ExprId,
//         r#type: wasmparser::GlobalType,
//     },
//     Imported {
//         module: &'a str,
//         name: &'a str,
//         r#type: wasmparser::GlobalType,
//     },
// }

//impl GlobalValue
//fn r#type(&self) ->
