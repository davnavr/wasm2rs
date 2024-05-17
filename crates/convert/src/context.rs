//! Types describing a WebAssembly module and the mapping of WebAssembly constructs to Rust.

use crate::ast::{FuncId, GlobalId, MemoryId};

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
        matches!(self.call_kinds[idx], CallKind::Method)
            && matches!(
                self.unwind_kinds[idx],
                UnwindKind::Maybe | UnwindKind::Always
            )
    }
}

/// Index into [`Context::imported_modules`] indicating the module that a WebAssembly import
/// originates from.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(crate) struct ImportedModule(pub(crate) u16);

#[derive(Clone, Copy, Debug)]
pub(crate) struct Import<'ctx, 'wasm> {
    pub(crate) module: &'ctx &'wasm str,
    pub(crate) name: &'ctx &'wasm str,
}

/// Describes the identifier used to refer to the Rust function corresponding to a WebAssembly
/// function.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FunctionName<'wasm> {
    /// The function's WebAssembly export name is used.
    Export(&'wasm str),
    /// An identifier based on the [**funcidx**](FuncId) is used.
    Id(FuncId),
    //Import
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

pub(crate) struct DefinedGlobal {
    pub(crate) id: GlobalId,
    pub(crate) initializer: crate::ast::ExprId,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum GlobalKind<'ctx, 'wasm> {
    /// The global is translated to a Rust `const`.
    Const,
    ImmutableField,
    MutableField {
        import: Option<Import<'ctx, 'wasm>>,
    },
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
    /// Specifies the module each imported memory originated from.
    pub(crate) memory_import_modules: Box<[ImportedModule]>,
    /// Specifies the module each imported global originated from.
    pub(crate) global_import_modules: Box<[ImportedModule]>,
    /// Specifies the name of each WebAssembly function import.
    pub(crate) func_import_names: Box<[&'wasm str]>,
    /// Specifies the name of each WebAssembly memory import.
    pub(crate) memory_import_names: Box<[&'wasm str]>,
    /// Specifies the name of each WebAssembly global import.
    pub(crate) global_import_names: Box<[&'wasm str]>,
    /// Lookup table for each exported WebAssembly function.
    pub(crate) function_export_names: std::collections::HashMap<FuncId, &'wasm str>,
    /// Lookup table for each exported WebAssembly memory.
    pub(crate) memory_export_names: std::collections::HashMap<MemoryId, &'wasm str>,
    /// Lookup table for each exported WebAssembly global.
    pub(crate) global_export_names: std::collections::HashMap<GlobalId, &'wasm str>,
    /// Specifies which WebAssembly memories are exported.
    ///
    /// These are in the order they were specified in the WebAssembly export section.
    pub(crate) memory_exports: Vec<MemoryId>,
    /// Specifies which WebAssembly globals are exported.
    ///
    /// These are in the order they were specified in the WebAssembly export section.
    pub(crate) global_exports: Vec<GlobalId>,
    pub(crate) function_attributes: FunctionAttributes,
    /// Stores the initializer expression for each global defined by the WebAssembly module.
    pub(crate) global_initializers: crate::ast::Arena,
    /// Specifies the WebAssembly globals that correspond to a Rust field. These require
    /// assignment of their initial value within the generated `instantiate()` function.
    ///
    /// These are stored in ascending order.
    pub(crate) instantiate_globals: Vec<GlobalId>,
    /// Specifies the *defined* WebAssembly globals that correspond to a Rust field, and their
    /// initial values.
    pub(crate) defined_globals: std::collections::HashMap<GlobalId, crate::ast::ExprId>,
    /// Specifies the *defined* WebAsembly globals that correspond to a Rust `const`.
    ///
    /// These are stored in ascending order.
    pub(crate) constant_globals: Vec<DefinedGlobal>,
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
            Some(name) if self.function_attributes.can_use_export_name(f) => {
                FunctionName::Export(name)
            }
            Some(_) | None => FunctionName::Id(f),
        }
    }

    // TODO: fn table_name, memory_name, global_name,

    fn import_lookup<'ctx>(
        &'ctx self,
        index: usize,
        modules: &'ctx Box<[ImportedModule]>,
        names: &'ctx Box<[&'wasm str]>,
    ) -> Option<Import<'ctx, 'wasm>> {
        names.get(index).map(|name| {
            assert_eq!(modules.len(), names.len());

            Import {
                module: &self.imported_modules[usize::from(modules[index].0)],
                name,
            }
        })
    }

    pub(crate) fn memory_import(&self, m: MemoryId) -> Option<Import<'_, 'wasm>> {
        self.import_lookup(
            m.0 as usize,
            &self.memory_import_modules,
            &self.memory_import_names,
        )
    }

    pub(crate) fn global_import(&self, g: GlobalId) -> Option<Import<'_, 'wasm>> {
        self.import_lookup(
            g.0 as usize,
            &self.global_import_modules,
            &self.global_import_names,
        )
    }

    pub(crate) fn global_kind(&self, g: GlobalId) -> GlobalKind {
        if self.types.global_at(g.0).mutable {
            GlobalKind::MutableField {
                import: self.global_import(g),
            }
        } else if self
            .constant_globals
            .binary_search_by_key(&g, |global| global.id)
            .is_ok()
        {
            GlobalKind::Const
        } else {
            GlobalKind::ImmutableField
        }
    }

    pub(crate) fn finish(self, allocations: &crate::Allocations) {
        allocations.return_ast_arena(self.global_initializers);
    }
}
