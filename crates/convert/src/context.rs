//! Types describing a WebAssembly module and the mapping of WebAssembly constructs to Rust.

use crate::ast::{FuncId, GlobalId, MemoryId, TableId};

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub(crate) struct WasmStr<'wasm>(pub(crate) &'wasm str);

impl<'wasm> From<WasmStr<'wasm>> for crate::ident::SafeIdent<'wasm> {
    fn from(s: WasmStr<'wasm>) -> Self {
        s.0.into()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Import<'ctx, 'wasm> {
    pub(crate) module: &'ctx WasmStr<'wasm>,
    pub(crate) name: &'ctx WasmStr<'wasm>,
}

impl std::fmt::Display for Import<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "imports.{}().{}",
            crate::ident::SafeIdent::from(*self.module),
            crate::ident::SafeIdent::from(*self.name)
        )
    }
}

/// Describes the identifier used to refer to the Rust function corresponding to the *defined*
/// WebAssembly function.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FunctionName<'ctx, 'wasm> {
    /// The function's WebAssembly export name is used.
    Export(&'ctx WasmStr<'wasm>),
    /// An identifier based on the [**funcidx**](FuncId) is used.
    Id(FuncId),
}

impl FunctionName<'_, '_> {
    pub(crate) const fn visibility(&self) -> &'static str {
        match self {
            Self::Export(_) => "pub ",
            Self::Id(_) => "",
        }
    }
}

impl std::fmt::Display for FunctionName<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Export(name) => std::fmt::Display::fmt(&crate::ident::SafeIdent::from(**name), f),
            Self::Id(FuncId(idx)) => write!(f, "_f{idx}"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum FunctionIdent<'ctx, 'wasm> {
    Name(FunctionName<'ctx, 'wasm>),
    Import(Import<'ctx, 'wasm>),
}

impl std::fmt::Display for FunctionIdent<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(name) => std::fmt::Display::fmt(name, f),
            Self::Import(import) => std::fmt::Display::fmt(import, f),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum TableIdent<'ctx, 'wasm> {
    Id(TableId),
    Import(Import<'ctx, 'wasm>),
}

impl std::fmt::Display for TableIdent<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(TableId(id)) => write!(f, "_tbl_{id}"),
            Self::Import(import) => write!(f, "{import}()"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum MemoryIdent<'ctx, 'wasm> {
    Id(MemoryId),
    Import(Import<'ctx, 'wasm>),
}

impl std::fmt::Display for MemoryIdent<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(MemoryId(0)) => f.write_str("_m"), // Main memory is given a shorter name.
            Self::Id(MemoryId(id)) => write!(f, "_mem_{id}"),
            Self::Import(import) => write!(f, "{import}()"),
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

#[derive(Debug)]
pub(crate) enum FuncElements {
    Indices(Vec<crate::ast::ElemFuncRef>),
    Expressions(Vec<crate::ast::ExprId>),
}

#[derive(Debug)]
pub(crate) struct ActiveFuncElements {
    pub(crate) table: crate::ast::TableId,
    pub(crate) id: crate::ast::ElementId,
    /// Evaluates to an index specifying where into the [table] the element segment's contents are
    /// copied.
    ///
    /// [table]: ActiveFuncElements::table
    pub(crate) offset: crate::ast::ExprId,
    pub(crate) elements: FuncElements,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ActiveDataSegment {
    pub(crate) data: crate::ast::DataId,
    pub(crate) memory: MemoryId,
    /// Evaluates to an address specifying where into the linear memory the data segment's contents
    /// are copied.
    pub(crate) offset: crate::ast::ExprId,
}

pub(crate) type ExportLookup<'wasm, I> = std::collections::HashMap<I, Vec<WasmStr<'wasm>>>;

/// Stores all information relating to a WebAssembly module and how it's components are accessed
/// when translated to Rust.
#[must_use = "call .finish()"]
pub(crate) struct Context<'wasm> {
    pub(crate) types: wasmparser::types::Types,
    /// Stores any constant expressions used within the WebAssembly module. These include:
    /// - The initializer expression for each defined global.
    /// - The offset for active data segments.
    pub(crate) constant_expressions: crate::ast::Arena,
    /// Contains the name of each [`ImportedModule`].
    pub(crate) imported_modules: Vec<WasmStr<'wasm>>,
    /// Specifies the module each imported function originated from.
    pub(crate) func_import_modules: Box<[ImportedModule]>,
    /// Specifies the module each imported table originated from.
    pub(crate) table_import_modules: Box<[ImportedModule]>,
    /// Specifies the module each imported memory originated from.
    pub(crate) memory_import_modules: Box<[ImportedModule]>,
    /// Specifies the module each imported global originated from.
    pub(crate) global_import_modules: Box<[ImportedModule]>,
    /// Specifies the name of each WebAssembly function import.
    pub(crate) func_import_names: Box<[WasmStr<'wasm>]>, // TODO: Maybe store (BoxedIdent, WasmStr)?
    /// Specifies the name of each WebAssembly table import.
    pub(crate) table_import_names: Box<[WasmStr<'wasm>]>,
    /// Specifies the name of each WebAssembly memory import.
    pub(crate) memory_import_names: Box<[WasmStr<'wasm>]>,
    /// Specifies the name of each WebAssembly global import.
    pub(crate) global_import_names: Box<[WasmStr<'wasm>]>,
    /// Lookup table for each exported WebAssembly function.
    pub(crate) function_export_names: ExportLookup<'wasm, FuncId>,
    /// Lookup table for each exported WebAssembly table.
    pub(crate) table_export_names: ExportLookup<'wasm, TableId>,
    /// Lookup table for each exported WebAssembly memory.
    pub(crate) memory_export_names: ExportLookup<'wasm, MemoryId>,
    /// Lookup table for each exported WebAssembly global.
    pub(crate) global_export_names: ExportLookup<'wasm, GlobalId>,
    /// Specifies which WebAssembly tables are exported.
    ///
    /// These are in the order they were specified in the WebAssembly export section.
    pub(crate) table_exports: Vec<TableId>,
    /// Specifies which WebAssembly memories are exported.
    ///
    /// These are in the order they were specified in the WebAssembly export section.
    pub(crate) memory_exports: Vec<MemoryId>,
    /// Specifies which WebAssembly globals are exported.
    ///
    /// These are in the order they were specified in the WebAssembly export section.
    pub(crate) global_exports: Vec<GlobalId>,
    pub(crate) function_attributes: FunctionAttributes,
    /// Specifies the offset to the code section entry for each defined WebAssembly function.
    pub(crate) function_code_offsets: Box<[u64]>,
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
    /// Specifies the WebAssembly module's active element segments that copy [`funcref`]s to a
    /// table.
    ///
    /// These are stored in ascending *elemidx* order.
    pub(crate) active_func_elements: Vec<ActiveFuncElements>,
    /// Specifies the contents of each WebAssembly data segment.
    pub(crate) data_segment_contents: Box<[&'wasm [u8]]>,
    /// Specifies the WebAssembly module's active data segments.
    ///
    /// These are stored in ascending *dataidx* order.
    pub(crate) active_data_segments: Vec<ActiveDataSegment>,
    /// The concatenation of all the WebAssembly module's declarative data segments (containing
    /// `funcref`s only).
    pub(crate) declarative_func_elements: Vec<crate::ast::ElemFuncRef>,
}

impl<'wasm> Context<'wasm> {
    pub(crate) fn has_imports(&self) -> bool {
        !self.func_import_names.is_empty()
            && !self.memory_import_names.is_empty()
            && !self.global_import_names.is_empty()
    }

    pub(crate) fn function_signature(&self, f: FuncId) -> &wasmparser::FuncType {
        self.types[self.types.core_function_at(f.0)].unwrap_func()
    }

    pub(crate) fn function_import_count(&self) -> usize {
        self.func_import_names.len()
    }

    /// Gets the name of the function to use when it is defined.
    pub(crate) fn function_name(&self, f: FuncId) -> FunctionName<'_, 'wasm> {
        match self.function_export_names.get(&f) {
            Some(name) if self.function_attributes.can_use_export_name(f) => {
                FunctionName::Export(name.first().unwrap())
            }
            Some(_) | None => FunctionName::Id(f),
        }
    }

    /// Gets the name of the function to use when it is called.
    pub(crate) fn function_ident(&self, f: FuncId) -> FunctionIdent<'_, 'wasm> {
        self.function_import(f)
            .map(FunctionIdent::Import)
            .unwrap_or_else(move || FunctionIdent::Name(self.function_name(f)))
    }

    pub(crate) fn table_ident(&self, t: TableId) -> TableIdent<'_, 'wasm> {
        self.table_import(t)
            .map(TableIdent::Import)
            .unwrap_or_else(move || TableIdent::Id(t))
    }

    pub(crate) fn memory_ident(&self, m: MemoryId) -> MemoryIdent<'_, 'wasm> {
        self.memory_import(m)
            .map(MemoryIdent::Import)
            .unwrap_or_else(move || MemoryIdent::Id(m))
    }

    // TODO: fn table_ident, global_ident

    fn import_lookup<'ctx>(
        &'ctx self,
        index: usize,
        modules: &'ctx [ImportedModule],
        names: &'ctx [WasmStr<'wasm>],
    ) -> Option<Import<'ctx, 'wasm>> {
        names.get(index).map(|name| {
            assert_eq!(modules.len(), names.len());

            Import {
                module: &self.imported_modules[usize::from(modules[index].0)],
                name,
            }
        })
    }

    pub(crate) fn function_import(&self, f: FuncId) -> Option<Import<'_, 'wasm>> {
        self.import_lookup(
            f.0 as usize,
            &self.func_import_modules,
            &self.func_import_names,
        )
    }

    pub(crate) fn table_import(&self, t: TableId) -> Option<Import<'_, 'wasm>> {
        self.import_lookup(
            t.0 as usize,
            &self.table_import_modules,
            &self.table_import_names,
        )
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
        allocations.return_ast_arena(self.constant_expressions);
    }
}

impl std::fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("import_modules", &self.imported_modules)
            .field("function_import_names", &self.func_import_names)
            .field("memory_import_names", &self.memory_import_names)
            .field("global_import_names", &self.global_import_names)
            .field("function_export_names", &self.function_export_names)
            .field("memory_export_names", &self.memory_export_names)
            .field("global_export_names", &self.global_export_names)
            .field("memory_exports", &self.memory_exports)
            .field("global_exports", &self.global_exports)
            .field("start_function", &self.start_function)
            .finish_non_exhaustive()
    }
}
