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

    pub(crate) fn can_use_export_name(&self, f: FuncId) -> bool {
        let idx = f.0 as usize;

        assert_eq!(self.call_kinds.len(), self.unwind_kinds.len());

        // Currently, `UnwindKind::Always` results in the same generated code as `UnwindKind::Maybe`.
        matches!(self.call_kinds[idx], CallKind::Method) && matches!(self.unwind_kinds[idx], UnwindKind::Maybe | UnwindKind::Always)
    }
}

/// Index into [`Context::imported_modules`] indicating the module that a WebAssembly import
/// originates from.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(crate) struct ImportedModule(pub(crate) u16);

#[derive(Clone, Copy, Debug)]
pub(crate) enum GlobalValue {
    /// The global is initialized with the corresponding expression taken from
    /// [`Context::global_initializers`].
    Initialized(crate::ast::ExprId),
    /// The global is imported.
    Imported,
}

/// Describes the identifier used to refer to the Rust function corresponding to a WebAssembly
/// function.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FunctionName<'wasm> {
    /// The function's WebAssembly export name is used.
    Export(&'wasm str),
    /// An identifier based on the [**funcidx**](FuncId) is used.
    Id(FuncId),
}

impl FunctionName<'_> {
    pub(crate) const fn visibility(&self) -> &str {
        match self {
            Self::Export(_) => "pub ",
            Self::Id(_) => "",
        }
    }
}

impl std::fmt::Display for FunctionName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Export(name) => std::fmt::Display::fmt(&crate::ident::SafeIdent::from(*name), f),
            Self::Id(idx) => std::fmt::Display::fmt(idx, f),
        }
    }
}

/// Stores all information relating to a WebAssembly module and how it's components are accessed
/// when translated to Rust.
#[must_use = "call .finish()"]
pub(crate) struct Context<'wasm> {
    pub(crate) types: wasmparser::types::Types,
    /// Contains the name of each [`ImportedModule`].
    pub(crate) imported_modules: Box<[&'wasm str]>,
    /// Specifies the module each imported function originated from.
    pub(crate) func_import_modules: Box<[ImportedModule]>,
    /// Specifies the module each imported global originated from.
    pub(crate) global_import_modules: Box<[ImportedModule]>,
    /// Specifies the name of each WebAssembly function import.
    pub(crate) func_import_names: Box<[&'wasm str]>,
    /// Specifies the name of each WebAssembly global import.
    pub(crate) global_import_names: Box<[&'wasm str]>,
    /// Lookup table for each exported WebAssembly function.
    pub(crate) function_export_names: std::collections::HashMap<crate::ast::FuncId, &'wasm str>,
    /// Lookup table for each exported WebAssembly global.
    pub(crate) global_export_names: std::collections::HashMap<crate::ast::GlobalId, &'wasm str>,
    /// Specifies which functions are exported.
    pub(crate) function_exports: Vec<crate::ast::FuncId>,
    /// Specifies which globals are exported.
    pub(crate) global_exports: Vec<crate::ast::GlobalId>,
    pub(crate) function_attributes: FunctionAttributes,
    /// Specifies the initial value of each WebAssembly global.
    pub(crate) global_values: Box<[GlobalValue]>,
    /// Stores the initializer expression for each global defined by the WebAssembly module.
    pub(crate) global_initializers: crate::ast::Arena,
    /// Corresponds to the [**start**] component of the WebAssembly module.
    ///
    /// [**start**]: https://webassembly.github.io/spec/core/syntax/modules.html#start-function
    pub(crate) start_function: Option<FuncId>,
}

impl<'wasm> Context<'wasm> {
    pub(crate) fn function_signature(&self, f: FuncId) -> &wasmparser::FuncType {
        self.types[self.types.core_function_at(f.0)].unwrap_func()
    }

    pub(crate) fn function_import_count(&self) -> usize {
        self.func_import_names.len()
    }

    /// Gets the name of the function to use when it is being invoked.
    pub(crate) fn function_name(&self, f: FuncId) -> FunctionName<'wasm> {
        match self.function_export_names.get(&f).copied() {
            Some(name) if self.function_attributes.can_use_export_name(f) => FunctionName::Export(name),
            Some(_) | None => FunctionName::Id(f),
        }
    }

    // /// Returns an iterator over the exported functions that require an additional stub function.
    // ///
    // /// A stub function is used to hides implementation details, such as the possible omission of
    // /// the `&self` parameter in the original function.
    // pub(crate) fn function_export_stubs(&self) -> impl Iterator<> {}

    pub(crate) fn finish(self, allocations: &crate::Allocations) {
        allocations.return_ast_arena(self.global_initializers);
    }
}
