//! Contains the core code for converting WebAssembly to Rust.

mod allocations;
mod code;
mod constant;
mod options;

pub use allocations::Allocations;
pub use options::{DataSegmentWriter, DebugInfo, StackOverflowChecks};

pub(crate) const FUNC_REF_MAX_PARAM_COUNT: usize = 9;

/// Provides options for converting a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
pub struct Convert<'a> {
    indentation: crate::Indentation,
    data_segment_writer: DataSegmentWriter<'a>,
    // wasm_features: &'a wasmparser::WasmFeatures,
    stack_overflow_checks: StackOverflowChecks,
    debug_info: DebugInfo,
}

impl std::fmt::Debug for Convert<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Convert")
            .field("indentation", &self.indentation)
            .field("stack_overflow_checks", &self.stack_overflow_checks)
            .field("debug_info", &self.debug_info)
            .finish_non_exhaustive()
    }
}

impl Default for Convert<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Convert<'_> {
    /// Gets the default options.
    pub fn new() -> Self {
        Self {
            indentation: Default::default(),
            data_segment_writer: &|_, _| Ok(None),
            // wasm_features: &Self::DEFAULT_SUPPORTED_FEATURES,
            stack_overflow_checks: Default::default(),
            debug_info: Default::default(),
        }
    }
}

#[derive(Default)]
struct Sections<'a> {
    imports: Option<wasmparser::ImportSectionReader<'a>>,
    tables: Option<wasmparser::TableSectionReader<'a>>,
    memories: Option<wasmparser::MemorySectionReader<'a>>,
    //tags: Option<wasmparser::TagSectionReader<'a>>,
    globals: Option<wasmparser::GlobalSectionReader<'a>>,
    exports: Option<wasmparser::ExportSectionReader<'a>>,
    start_function: Option<crate::ast::FuncId>,
    elements: Option<wasmparser::ElementSectionReader<'a>>,
    data: Option<wasmparser::DataSectionReader<'a>>,
    code_count: u32,
}

struct Module<'a> {
    sections: Sections<'a>,
    function_bodies: Vec<code::Code<'a>>,
    types: wasmparser::types::Types,
}

fn validate_payloads(wasm: &[u8]) -> crate::Result<Module<'_>> {
    /// The set of WebAssembly features that are supported by default.
    const SUPPORTED_FEATURES: wasmparser::WasmFeatures = {
        macro_rules! features {
            ($($name:ident),*) => {{
                let features = wasmparser::WasmFeatures::empty();
                $(
                    let features = features.union(wasmparser::WasmFeatures::$name);
                )*
                features
            }};
        }

        features! {
            MUTABLE_GLOBAL,
            SATURATING_FLOAT_TO_INT,
            SIGN_EXTENSION,
            REFERENCE_TYPES,
            MULTI_VALUE,
            BULK_MEMORY,
            FLOATS,
            MULTI_MEMORY
        }
    };

    let mut validator = wasmparser::Validator::new_with_features(SUPPORTED_FEATURES);
    let mut sections = Sections::default();
    let mut function_bodies = Vec::new();

    for result in wasmparser::Parser::new(0).parse_all(wasm) {
        use wasmparser::Payload;

        match result? {
            Payload::Version {
                num,
                encoding,
                range,
            } => {
                validator.version(num, encoding, &range)?;
            }
            Payload::TypeSection(types) => {
                validator.type_section(&types)?;
            }
            Payload::ImportSection(section) => {
                validator.import_section(&section)?;
                sections.imports = Some(section);
            }
            Payload::FunctionSection(section) => {
                validator.function_section(&section)?;
            }
            Payload::TableSection(section) => {
                validator.table_section(&section)?;
                sections.tables = Some(section);
            }
            Payload::MemorySection(section) => {
                validator.memory_section(&section)?;
                sections.memories = Some(section);
            }
            Payload::TagSection(section) => {
                validator.tag_section(&section)?;
                anyhow::bail!("TODO: tag section is currently not supported");
            }
            Payload::GlobalSection(section) => {
                validator.global_section(&section)?;
                sections.globals = Some(section);
            }
            Payload::ExportSection(section) => {
                validator.export_section(&section)?;
                sections.exports = Some(section);
            }
            Payload::StartSection { func, range } => {
                validator.start_section(func, &range)?;
                sections.start_function = Some(crate::ast::FuncId(func));
            }
            Payload::ElementSection(section) => {
                validator.element_section(&section)?;
                sections.elements = Some(section);
            }
            Payload::DataCountSection { count, range } => {
                validator.data_count_section(count, &range)?
            }
            Payload::DataSection(section) => {
                validator.data_section(&section)?;
                sections.data = Some(section);
            }
            Payload::CodeSectionStart {
                count,
                range,
                size: _,
            } => {
                validator.code_section_start(count, &range)?;
                function_bodies.reserve(count as usize);
                sections.code_count = count;
            }
            Payload::CodeSectionEntry(body) => {
                function_bodies.push(code::Code::new(&mut validator, body)?)
            }
            Payload::CustomSection(_section) => {
                // Handling of custom `name`, 'producers' and DWARF sections is not yet implemented.
            }
            Payload::End(offset) => {
                let module = Module {
                    sections,
                    function_bodies,
                    types: validator.end(offset)?,
                };

                return Ok(module);
            }
            // Component model is not yet supported
            Payload::ModuleSection {
                parser: _,
                unchecked_range,
            } => validator.module_section(&unchecked_range)?,
            Payload::InstanceSection(section) => validator.instance_section(&section)?,
            Payload::CoreTypeSection(section) => validator.core_type_section(&section)?,
            Payload::ComponentSection {
                parser: _,
                unchecked_range,
            } => validator.component_section(&unchecked_range)?,
            Payload::ComponentInstanceSection(section) => {
                validator.component_instance_section(&section)?
            }
            Payload::ComponentAliasSection(section) => {
                validator.component_alias_section(&section)?
            }
            Payload::ComponentTypeSection(section) => validator.component_type_section(&section)?,
            Payload::ComponentCanonicalSection(section) => {
                validator.component_canonical_section(&section)?
            }
            Payload::ComponentStartSection { start, range } => {
                validator.component_start_section(&start, &range)?
            }
            Payload::ComponentImportSection(section) => {
                validator.component_import_section(&section)?
            }
            Payload::ComponentExportSection(section) => {
                validator.component_export_section(&section)?
            }
            Payload::UnknownSection {
                id,
                contents: _,
                range,
            } => validator.unknown_section(id, &range)?,
        }
    }

    // Either a `Payload::End` is processed, or an `Err` is returned.
    unreachable!()
}

fn get_section_count<T>(section: &Option<wasmparser::SectionLimited<'_, T>>) -> u32 {
    section.as_ref().map(|sec| sec.count()).unwrap_or(0)
}

fn parse_sections<'wasm>(
    types: wasmparser::types::Types,
    function_attributes: crate::context::FunctionAttributes,
    function_code_offsets: Box<[u64]>,
    allocations: &crate::Allocations,
    sections: Sections<'wasm>,
) -> crate::Result<crate::context::Context<'wasm>> {
    use anyhow::Context as _;

    let total_function_count = types.core_function_count() as usize;
    let total_memory_count = types.memory_count() as usize;
    let total_global_count = types.global_count() as usize;
    let mut context = crate::context::Context {
        types,
        constant_expressions: allocations.take_ast_arena(),
        function_attributes,
        start_function: sections.start_function,
        function_code_offsets,
        // These will be filled in later.
        imported_modules: Default::default(),
        func_import_modules: Default::default(),
        memory_import_modules: Default::default(),
        global_import_modules: Default::default(),
        func_import_names: Default::default(),
        memory_import_names: Default::default(),
        global_import_names: Default::default(),
        function_export_names: Default::default(),
        global_export_names: std::collections::HashMap::new(),
        memory_export_names: std::collections::HashMap::new(),
        memory_exports: Vec::<crate::ast::MemoryId>::new(),
        global_exports: Vec::<crate::ast::GlobalId>::new(),
        instantiate_globals: Vec::<crate::ast::GlobalId>::new(),
        defined_globals: std::collections::HashMap::new(),
        constant_globals: Vec::with_capacity(total_global_count / 2),
        data_segment_contents: Default::default(),
        active_data_segments: Default::default(),
        declarative_func_elements: Default::default(),
    };

    debug_assert_eq!(
        total_function_count,
        context.function_attributes.call_kinds.len()
    );
    debug_assert_eq!(
        total_function_count,
        context.function_attributes.unwind_kinds.len()
    );

    // Since validator already parsed these sections, maybe it should be fine to `unwrap()`.

    if let Some(import_section) = sections.imports {
        context
            .imported_modules
            .reserve((import_section.count() as usize).min(2));

        let func_import_count = total_function_count - (sections.code_count as usize);
        let memory_import_count =
            total_memory_count - (get_section_count(&sections.memories) as usize);
        let global_import_count =
            total_global_count - (get_section_count(&sections.globals) as usize);

        let mut func_import_modules = Vec::with_capacity(func_import_count);
        let mut memory_import_modules = Vec::with_capacity(memory_import_count);
        let mut global_import_modules = Vec::with_capacity(global_import_count);
        let mut func_import_names = Vec::with_capacity(func_import_count);
        let mut memory_import_names = Vec::with_capacity(memory_import_count);
        let mut global_import_names = Vec::with_capacity(global_import_count);

        // Non-mutable global imports are stored as a field.
        context.instantiate_globals.reserve(global_import_count / 2);

        let mut memory_idx = crate::ast::MemoryId(0);
        let mut global_idx = crate::ast::GlobalId(0);

        for result in import_section.into_iter_with_offsets() {
            use wasmparser::TypeRef;

            let (import_offset, import) = result?;

            let module_idx = if let Some(existing) = context
                .imported_modules
                .iter()
                .position(|name| name.0 == import.module)
            {
                crate::context::ImportedModule(existing as u16)
            } else {
                use anyhow::Context;

                let idx = u16::try_from(context.imported_modules.len()).with_context(|| {
                    format!("cannot import from more than 65535 modules @ {import_offset:#X}")
                })?;

                context
                    .imported_modules
                    .push(crate::context::WasmStr(import.module));

                crate::context::ImportedModule(idx)
            };

            let safe_import_name = crate::context::WasmStr(import.name);
            match import.ty {
                TypeRef::Func(_) => {
                    // TODO: set call kind for imported functions
                    // call_kinds.push(crate::context::CallKind::WithEmbedder);

                    func_import_names.push(safe_import_name);
                    func_import_modules.push(module_idx);
                }
                TypeRef::Memory(_) => {
                    memory_import_names.push(safe_import_name);
                    memory_import_modules.push(module_idx);
                    memory_idx.0 += 1;
                }
                TypeRef::Global(global_type) => {
                    global_import_names.push(safe_import_name);
                    global_import_modules.push(module_idx);
                    if !global_type.mutable {
                        context.instantiate_globals.push(global_idx);
                    }
                    global_idx.0 += 1;
                }
                TypeRef::Tag(_) => anyhow::bail!("tag imports are not yet supported"),
                bad => anyhow::bail!("TODO: Unsupported import {bad:?} @ {import_offset:#X}"),
            }
        }

        // Don't need to store the capacity of these `Vec`s.
        context.func_import_modules = func_import_modules.into_boxed_slice();
        context.memory_import_modules = memory_import_modules.into_boxed_slice();
        context.global_import_modules = global_import_modules.into_boxed_slice();

        context.func_import_names = func_import_names.into_boxed_slice();
        context.memory_import_names = memory_import_names.into_boxed_slice();
        context.global_import_names = global_import_names.into_boxed_slice();

        debug_assert_eq!(
            context.func_import_modules.len(),
            context.func_import_names.len()
        );
        debug_assert_eq!(
            context.memory_import_modules.len(),
            context.memory_import_names.len()
        );
        debug_assert_eq!(
            context.global_import_modules.len(),
            context.global_import_names.len()
        );
    }

    // Don't need to parse `sections.memories`, `types.memory_at()` provides its contents.

    if let Some(global_section) = sections.globals {
        let ids = 0u32..global_section.count();
        for (result, global_idx) in global_section.into_iter_with_offsets().zip(ids) {
            let (global_offset, global) = result?;
            let id = crate::ast::GlobalId(global_idx);
            let initializer = crate::convert::constant::create_ast(
                &global.init_expr,
                &mut context.constant_expressions,
            )
            .with_context(|| {
                format!("invalid initializer expression for {id} @ {global_offset:#X}")
            })?;

            match context.constant_expressions.get(initializer) {
                crate::ast::Expr::Literal(_) if !global.ty.mutable => context
                    .constant_globals
                    .push(crate::context::DefinedGlobal { id, initializer }),
                _ => {
                    context.defined_globals.insert(id, initializer);
                    context.instantiate_globals.push(id);
                }
            }
        }
    }

    // Validation already ensures there are no duplicated export names.
    if let Some(export_section) = sections.exports {
        // This assumes most exports are functions.
        context.function_export_names.reserve(
            (export_section.count() as usize / 2).min(context.types.core_function_count() as usize),
        );

        fn export_lookup_insert<'wasm, I>(
            lookup: &mut crate::context::ExportLookup<'wasm, I>,
            id: I,
            name: crate::context::WasmStr<'wasm>,
        ) where
            I: Eq + std::hash::Hash,
        {
            use std::collections::hash_map::Entry;

            match lookup.entry(id) {
                Entry::Occupied(mut existing) => existing.get_mut().push(name),
                Entry::Vacant(vacant) => {
                    vacant.insert(vec![name]);
                }
            }
        }

        for result in export_section.into_iter_with_offsets() {
            use wasmparser::ExternalKind;

            let (export_offset, export) = result?;
            let export_name = crate::context::WasmStr(export.name);
            match export.kind {
                ExternalKind::Func => {
                    let func_idx = crate::ast::FuncId(export.index);
                    export_lookup_insert(&mut context.function_export_names, func_idx, export_name);
                }
                ExternalKind::Memory => {
                    let mem_idx = crate::ast::MemoryId(export.index);
                    context.memory_exports.push(mem_idx);
                    export_lookup_insert(&mut context.memory_export_names, mem_idx, export_name);
                }
                ExternalKind::Global => {
                    let global_idx = crate::ast::GlobalId(export.index);
                    context.global_exports.push(global_idx);
                    export_lookup_insert(&mut context.global_export_names, global_idx, export_name);
                }
                ExternalKind::Tag => anyhow::bail!("tag exports are not yet supported"),
                bad => anyhow::bail!("TODO: Unsupported export {bad:?} @ {export_offset:#X}"),
            }
        }

        debug_assert_eq!(
            context.memory_exports.len(),
            context.memory_export_names.len()
        );
        debug_assert_eq!(
            context.global_exports.len(),
            context.global_export_names.len()
        );
    }

    if let Some(element_section) = sections.elements {
        for result in element_section.into_iter_with_offsets() {
            use wasmparser::{ElementItems, ElementKind, RefType};

            let (element_segment_offset, element_segment) = result?;

            match element_segment.kind {
                ElementKind::Declared => match element_segment.items {
                    ElementItems::Functions(indices) => {
                        context.declarative_func_elements.reserve(indices.count().try_into().unwrap_or_default());
                        for idx in indices.into_iter() {
                            context.declarative_func_elements.push(crate::ast::ElemFuncRef(crate::ast::FuncId(idx?)));
                        }
                    }
                    ElementItems::Expressions(RefType::FUNCREF, _) => {
                        anyhow::bail!("use crate::convert::constant to get RefFunc instructions from declarative data segment @ {element_segment_offset:#X}");
                    }
                    ElementItems::Expressions(RefType::EXTERNREF, _) => (),
                    ElementItems::Expressions(unknown, _) => anyhow::bail!("unsupported reference type {unknown:?} in declarative data segment @ {element_segment_offset:#X}"),
                },
                ElementKind::Active { table_index: _, offset_expr: _ } => {
                    anyhow::bail!("TODO: active element segment support");
                }
                ElementKind::Passive => {
                    anyhow::bail!("TODO: passive element segment support");
                }
            }
        }
    }

    if let Some(data_section) = sections.data {
        let mut data_segment_contents = Vec::with_capacity(data_section.count() as usize);

        // This assumes most data segments are active.
        context
            .active_data_segments
            .reserve(data_segment_contents.capacity() / 2);

        for result in data_section.into_iter_with_offsets() {
            let (data_segment_offset, data_segment) = result?;

            let data_id = crate::ast::DataId(data_segment_contents.len() as u32);
            data_segment_contents.push(data_segment.data);

            match data_segment.kind {
                wasmparser::DataKind::Active {
                    memory_index,
                    offset_expr,
                } => {
                    let offset = crate::convert::constant::create_ast(
                        &offset_expr,
                        &mut context.constant_expressions,
                    )
                    .with_context(|| {
                        format!(
                            "invalid offset expression for {data_id} @ {data_segment_offset:#X}"
                        )
                    })?;

                    context
                        .active_data_segments
                        .push(crate::context::ActiveDataSegment {
                            memory: crate::ast::MemoryId(memory_index),
                            data: data_id,
                            offset,
                        });
                }
                wasmparser::DataKind::Passive => (),
            }
        }

        context.data_segment_contents = data_segment_contents.into_boxed_slice();
    }

    Ok(context)
}

struct Ast<'wasm> {
    context: crate::context::Context<'wasm>,
    function_definitions: Vec<code::Definition>,
}

/// Provides access to information collected by a WebAssembly module during conversion to Rust.
///
/// See also the [`Convert::convert_from_buffer_with_intermediate()`] method.
#[derive(Clone, Copy, Debug)]
pub struct Intermediate<'conv, 'wasm> {
    context: &'conv crate::context::Context<'wasm>,
}

impl Convert<'_> {
    fn convert_function_definitions<'wasm, 'types>(
        &self,
        types: &'types wasmparser::types::Types,
        allocations: &crate::Allocations,
        attributes: &mut crate::context::FunctionAttributes,
        code: Vec<code::Code<'wasm>>,
    ) -> crate::Result<(Vec<code::Definition>, Vec<u64>)> {
        let convert_function_bodies =
            move |call_kind: &mut crate::context::CallKind,
                  unwind_kind: &mut crate::context::UnwindKind,
                  code: code::Code<'wasm>| {
                let offset = code.code_section_entry_offset();
                let (attr, definition) = code.convert(allocations, types)?;
                *call_kind = attr.call_kind;
                *unwind_kind = attr.unwind_kind;
                Ok((definition, offset))
            };

        let import_count = types.core_function_count() as usize - code.len();
        let call_kinds = &mut attributes.call_kinds[import_count..];
        let unwind_kinds = &mut attributes.unwind_kinds[import_count..];

        #[cfg(not(feature = "rayon"))]
        return {
            assert_eq!(code.len(), call_kinds.len());
            assert_eq!(code.len(), unwind_kinds.len());

            code.into_iter()
                .zip(call_kinds)
                .zip(unwind_kinds)
                .map(|((code, call_kind), unwind_kind)| {
                    convert_function_bodies(call_kind, unwind_kind, code)
                })
                .collect::<crate::Result<_>>()
        };

        #[cfg(feature = "rayon")]
        return {
            use rayon::prelude::*;

            code.into_par_iter()
                .zip_eq(call_kinds)
                .zip_eq(unwind_kinds)
                .map(|((code, call_kind), unwind_kind)| {
                    convert_function_bodies(call_kind, unwind_kind, code)
                })
                .collect::<crate::Result<_>>()
        };
    }

    fn convert_to_ast<'wasm>(
        &self,
        wasm: &'wasm [u8],
        allocations: &Allocations,
    ) -> crate::Result<Ast<'wasm>> {
        use anyhow::Context;

        let Module {
            sections,
            function_bodies,
            types,
        } = validate_payloads(wasm).context("validation failed")?;

        // TODO: Helper struct to return objects to `Allocations` even if an `Err` is returned.

        let function_count = types.core_function_count() as usize;
        let mut function_attributes = crate::context::FunctionAttributes {
            call_kinds: vec![crate::context::CallKind::Method; function_count].into_boxed_slice(),
            unwind_kinds: vec![crate::context::UnwindKind::Maybe; function_count]
                .into_boxed_slice(),
        };

        let (function_definitions, function_offsets) = self.convert_function_definitions(
            &types,
            allocations,
            &mut function_attributes,
            function_bodies,
        )?;

        let context = parse_sections(
            types,
            function_attributes,
            function_offsets.into_boxed_slice(),
            allocations,
            sections,
        )?;

        Ok(Ast {
            context,
            function_definitions,
        })
    }

    fn call_data_segment_writer(
        &self,
        data: &[u8],
        index: crate::ast::DataId,
    ) -> crate::Result<Option<String>> {
        use anyhow::Context;
        (self.data_segment_writer)(index.0, data)
            .with_context(|| format!("could not write contents of {index}"))
    }

    // TODO: Move this to `print.rs`
    fn print_ast(
        &self,
        allocations: &Allocations,
        context: &crate::context::Context,
        function_definitions: Vec<code::Definition>,
        output: &mut dyn std::io::Write,
    ) -> crate::Result<()> {
        use crate::buffer::Writer;
        use crate::write::Write as _;

        fn print_result_type(
            out: &mut Writer,
            types: &[wasmparser::ValType],
            mut f: impl FnMut(&mut Writer, u32),
        ) {
            for (ty, i) in types.iter().copied().zip(0u32..=u32::MAX) {
                if i > 0 {
                    out.write_str(", ");
                }

                f(out, i);
                write!(out, "{}", crate::ast::ValType::from(ty));
            }
        }

        fn print_param_types(out: &mut Writer, signature: &wasmparser::FuncType) {
            print_result_type(out, signature.params(), |out, i| {
                write!(out, "mut {}: ", crate::ast::LocalId(i))
            })
        }

        fn print_return_types(
            out: &mut crate::buffer::Writer,
            signature: &wasmparser::FuncType,
            unwind_kind: crate::context::UnwindKind,
        ) {
            if unwind_kind.can_unwind() {
                out.write_str("::core::result::Result<");
            }

            let types = signature.results();

            // if !matches(unwind_kind, crate::context::UnwindKind::Always)
            if types.len() != 1 {
                out.write_str("(");
            }

            print_result_type(out, types, |_, _| ());

            if types.len() != 1 {
                out.write_str(")");
            }
            // else { out.write_str("!"); } // `!` is currently nightly-only.

            // Remove this when `!` is supported.
            if matches!(unwind_kind, crate::context::UnwindKind::Always) {
                out.write_str(" /* ! */");
            }

            if unwind_kind.can_unwind() {
                out.write_str(", embedder::Trap>");
            }
        }

        let write_function_definitions = |(index, definition): (usize, code::Definition)| {
            use crate::context::CallKind;

            // TODO: Use some average # of Rust bytes per # of Wasm bytes
            let mut out = Writer::new(allocations.byte_buffer_pool());

            let func_id = crate::ast::FuncId((context.function_import_count() + index) as u32);
            let signature = context.function_signature(func_id);

            let function_name = context.function_name(func_id);
            write!(out, "\n{}fn {function_name}(", function_name.visibility());

            let call_kind = context.function_attributes.call_kind(func_id);
            match call_kind {
                CallKind::Function => (),
                CallKind::Method => out.write_str("&self"),
                // CallKind::WithEmbedder => out.write_str("_embedder: &embedder::State"),
            }

            let has_parameters = signature.params().is_empty();
            if !matches!(call_kind, CallKind::Function) && !has_parameters {
                out.write_str(", ");
            }

            print_param_types(&mut out, signature);

            out.write_str(")");

            // Omit return type for functions that return nothing and can't unwind
            let unwind_kind = context.function_attributes.unwind_kind(func_id);
            let has_return_types = !signature.results().is_empty() || unwind_kind.can_unwind();
            if has_return_types {
                out.write_str(" -> ");

                print_return_types(&mut out, signature, unwind_kind)
            }

            out.write_str(" {\n");

            crate::ast::print::print_statements(
                &mut out,
                &crate::ast::print::Context {
                    wasm: context,
                    arena: &definition.arena,
                    debug_info: self.debug_info,
                },
                func_id,
                self.indentation,
                1,
                &definition.body,
            );

            out.write_str("}\n");

            // Check if an additional Rust function must be generated to access the function
            // export. A stub function is used to hides implementation details, such as the
            // possible omission of the `&self` parameter in the original function.
            if let Some(export_names) = context.function_export_names.get(&func_id) {
                let export_names = if matches!(function_name, crate::context::FunctionName::Id(_)) {
                    export_names.as_slice()
                } else {
                    // Function already uses the first export name, so exclude it
                    &export_names[1..]
                };

                for export in export_names.iter() {
                    write!(
                        out,
                        "\npub fn {}(&self",
                        crate::ident::SafeIdent::from(*export)
                    );

                    if !has_parameters {
                        out.write_str(", ");
                    }

                    print_param_types(&mut out, signature);
                    out.write_str(") -> ");
                    print_return_types(&mut out, signature, crate::context::UnwindKind::Maybe);

                    out.write_str(" {\n");

                    crate::ast::print::print_stub(
                        &mut out,
                        func_id,
                        context,
                        self.indentation,
                        1,
                        signature.params().len() as u32,
                    );

                    out.write_str("}\n");
                }
            }

            definition.finish(allocations);
            out.finish()
        };

        let function_items: Vec<crate::buffer::Buffer>;

        {
            use rayon::prelude::*;

            function_items = function_definitions
                .into_par_iter()
                .enumerate()
                .map(write_function_definitions)
                .flatten()
                .collect();
        }

        #[cfg(not(feature = "rayon"))]
        {
            function_items = function_definitions
                .into_iter()
                .enumerate()
                .map(write_function_definitions)
                .flatten()
                .collect();
        }

        // Write the file contents
        let sp = self.indentation.to_str();
        let mut o = crate::write::IoWrite::new(output);
        writeln!(o, "// Generated by wasm2rs {}", env!("CARGO_PKG_VERSION"));
        writeln!(o, "macro_rules! {} {{", crate::ident::Ident::MACRO_NAME);
        o.write_str(sp);
        o.write_str(concat!(
            "($vis:vis mod $module:ident use $(:: $embedder_start:ident ::)? ",
            "$($embedder_more:ident)::+) => {\n\n"
        ));

        // TODO: After `$module`, add `$(<$lifetime:lifetime>)?`

        o.write_str("#[allow(non_snake_case)]\n"); // Names might be mangled
        o.write_str("#[allow(dead_code)]\n"); // Some functions may not be called
        o.write_str("#[allow(unreachable_code)]\n"); // Some branches may not be taken (e.g. infinite loops detected by rustc)
        o.write_str("#[allow(unreachable_pub)]\n"); // Macro may be invoked within a non-public module
        o.write_str("#[allow(unused_mut)]\n"); // Variables may be marked mut
        o.write_str("#[allow(unused_imports)]\n"); // Sometimes `TrapWith` or `UnwindWith` isn't used
        o.write_str("$vis mod $module {\n\n");
        o.write_str("use $(::$embedder_start::)? $($embedder_more)::+ as embedder;\n");
        o.write_str("use embedder::rt::{trap::TrapWith as _, trace::UnwindWith as _};\n\n");

        // TODO: Option to specify #[derive(Debug)] impl
        writeln!(
            o,
            "pub struct Module {{\n{sp}pub imports: embedder::Imports,"
        );

        let defined_memories = ((context.memory_import_names.len() as u32)
            ..context.types.memory_count())
            .map(crate::ast::MemoryId);

        for memory_id in defined_memories.clone() {
            writeln!(
                o,
                "{sp}{}: embedder::Memory{},",
                crate::context::MemoryIdent::Id(memory_id),
                memory_id.0
            );
        }

        for global_id in context.instantiate_globals.iter() {
            let global_type = context.types.global_at(global_id.0);
            write!(o, "{sp}{global_id}: ");

            if global_type.mutable {
                o.write_str("embedder::rt::global::Global<");
            }

            write!(o, "{}", crate::ast::ValType::from(global_type.content_type));

            if global_type.mutable {
                o.write_str(">");
            }

            writeln!(o, ",");
        }

        // Could allow option to specify lazy loading for these function references rather than
        // paying for the cost of construction upfront.
        for elem in context.declarative_func_elements.iter() {
            // Have to use cell so that it can be replaced with `NULL` later.
            writeln!(
                o,
                "{sp}{elem}: ::core::cell::Cell<embedder::rt::func_ref::FuncRef<'static, embedder::Trap>>,",
            );
        }

        o.write_str(concat!(
            "} // struct Module\n\n",
            "#[repr(transparent)]\n",
            "pub struct Instance(::core::option::Option<embedder::Module<Module>>);\n\n"
        ));

        writeln!(o, "impl Module {{");

        // Write constant globals.
        for global in context.constant_globals.iter() {
            match context.constant_expressions.get(global.initializer) {
                crate::ast::Expr::Literal(literal) => {
                    write!(o, "{sp}const {:#}: {} = ", global.id, literal.type_of());
                    literal.print(&mut o);
                    o.write_str(";\n");
                }
                bad => unreachable!(
                    "expected global {} to be constant, but got {bad:?}",
                    global.id
                ),
            }
        }

        // Element segments cannot be writen as a constant, since FuncRef requires an already
        // instantiated module.

        // Write the data segments contents.
        for (data, data_id) in context
            .data_segment_contents
            .iter()
            .zip((0u32..=u32::MAX).map(crate::ast::DataId))
        {
            write!(o, "{sp}const {data_id}: &'static [u8] = ");

            /// If the `data.len() <=` this, then a literal byte string is generated.
            const PREFER_LITERAL_LENGTH: usize = 64;

            fn write_byte_string(out: &mut dyn crate::write::Write, data: &[u8]) {
                out.write_str("b\"");
                for b in data {
                    write!(out, "{}", std::ascii::escape_default(*b));
                }
                out.write_str("\"");
            }

            if data.len() <= PREFER_LITERAL_LENGTH {
                write_byte_string(&mut o, data);
            } else if let Some(path) = self.call_data_segment_writer(data, data_id)? {
                write!(o, "::core::include_bytes!({});", path.escape_default());
            } else {
                write_byte_string(&mut o, data);
            }

            o.write_str(";\n\n");
        }

        // Write exported memories.
        let exported_memories = context.memory_exports.iter().flat_map(|id| {
            context
                .memory_export_names
                .get(id)
                .expect("memory export did not have a name")
                .iter()
                .zip(std::iter::repeat(*id))
        });

        for (export, memory_id) in exported_memories {
            write!(
                o,
                "\n{sp}pub fn {}(&self) -> &embedder::Memory{} {{\n{sp}{sp}",
                crate::ident::SafeIdent::from(*export),
                memory_id.0
            );

            let identifier = context.memory_ident(memory_id);

            if matches!(identifier, crate::context::MemoryIdent::Id(_)) {
                o.write_str("&");
            }

            writeln!(o, "self.{identifier}\n{sp}}}\n");
        }

        // Write exported globals.
        let exported_globals = context.global_exports.iter().flat_map(|id| {
            context
                .global_export_names
                .get(id)
                .expect("memory export did not have a name")
                .iter()
                .zip(std::iter::repeat(*id))
        });

        for (export, global_id) in exported_globals {
            write!(
                o,
                "\n{sp}pub fn {}(&self) -> ",
                crate::ident::SafeIdent::from(*export)
            );

            let global_type = context.types.global_at(global_id.0);

            if global_type.mutable {
                write!(o, "&embedder::rt::global::Global<");
            }

            write!(o, "{}", crate::ast::ValType::from(global_type.content_type));

            if global_type.mutable {
                o.write_str(">");
            }

            write!(o, " {{\n{sp}{sp}");
            match context.global_kind(global_id) {
                crate::context::GlobalKind::Const => write!(o, "Self::{global_id:#}"),
                crate::context::GlobalKind::ImmutableField => write!(o, "self.{global_id}"),
                crate::context::GlobalKind::MutableField { import: None } => {
                    write!(o, "&self.{global_id}");
                }
                crate::context::GlobalKind::MutableField {
                    import: Some(import),
                } => todo!("printing of mutable global imports {import:?}"),
            }
            write!(o, "\n{sp}}}\n");
        }

        // Write function definitions and their bodies.
        let output = o.into_inner()?;
        output.flush()?; // If a `BufWriter` is being used, this might allow it to be bypassed.
        crate::buffer::write_all_vectored(
            output,
            &function_items,
            &mut Vec::with_capacity(function_items.len()),
        )?;

        let mut o = crate::write::IoWrite::new(output);

        // Write function symbols used in stack traces.
        if self.debug_info != DebugInfo::Omit {
            let func_import_count = context.func_import_names.len();
            for (unwind_kind, func_id) in context.function_attributes.unwind_kinds
                [func_import_count..]
                .iter()
                .zip((func_import_count as u32..=u32::MAX).map(crate::ast::FuncId))
            {
                let export_names = context.function_export_names.get(&func_id);

                // Non-exported functions that cannot trap are excluded.
                if !unwind_kind.can_unwind() && export_names.is_none() {
                    continue;
                }

                write!(
                    o,
                    "{sp}const {}: embedder::rt::symbol::WasmSymbol = {{",
                    crate::ast::SymbolName(func_id),
                );
                write!(
                    o,
                    " let mut sym = embedder::rt::symbol::WasmSymbol::new({}, &",
                    func_id.0,
                );

                // Write function signature.
                fn write_result_types(
                    o: &mut dyn crate::write::Write,
                    types: &[wasmparser::ValType],
                ) {
                    o.write_str("&[");

                    for (i, ty) in types.iter().enumerate() {
                        use wasmparser::ValType;

                        if i > 0 {
                            o.write_str(", ");
                        }

                        o.write_str("embedder::rt::symbol::WasmValType::");
                        o.write_str(match ty {
                            ValType::I32 => "I32",
                            ValType::I64 => "I64",
                            ValType::F32 => "F32",
                            ValType::F64 => "F64",
                            ValType::V128 => "V128",
                            ValType::Ref(wasmparser::RefType::FUNCREF) => "FuncRef",
                            ValType::Ref(wasmparser::RefType::EXTERNREF) => "ExternRef",
                            ValType::Ref(unknown) => {
                                todo!("unknown reference type in signature {unknown:?}")
                            }
                        });
                    }

                    o.write_str("]");
                }

                let signature = context.function_signature(func_id);
                o.write_str("embedder::rt::symbol::WasmSymbolSignature { parameters: ");
                write_result_types(&mut o, signature.params());
                o.write_str(", results: ");
                write_result_types(&mut o, signature.results());

                // Import or definition?
                o.write_str(" }, embedder::rt::symbol::WasmSymbolKind::");
                if let Some(import) = context.function_import(func_id) {
                    write!(
                        o,
                        "Imported(&embedder::rt::symbol::WasmImportSymbol {{ module: \"{}\", name: \"{}\" }})",
                        import.module.0.escape_default(),
                        import.name.0.escape_default(),
                    );
                } else {
                    write!(
                        o,
                        "Defined {{ offset: {} }}",
                        context.function_code_offsets
                            [func_id.0 as usize - context.func_import_names.len()]
                    );
                }

                o.write_str(");");

                if let Some(export_names) = export_names {
                    o.write_str(" sym.export_names = &[");

                    for (i, name) in export_names.iter().enumerate() {
                        if i > 0 {
                            o.write_str(", ");
                        }

                        write!(o, "\"{}\"", name.0.escape_default());
                    }

                    o.write_str("];");
                }

                // Custom name
                // write!(o, " sym.custom_name = Some(\"{}\");", custom_name.escape_default());

                o.write_str(" sym };\n");

                // Generate a function to produce a `WasmFrame`
                writeln!(
                    o,
                    "{sp}const fn {}(o: u32) -> embedder::rt::trace::WasmFrame {{ embedder::rt::trace::WasmFrame::new(&Self::{}, o) }}",
                    crate::ast::MakeFrame(func_id),
                    crate::ast::SymbolName(func_id)
                )
            }
        }

        o.write_str("\n} // impl Module\n\n");

        o.write_str("impl Instance {\n");

        // Write function to instantiate the module.
        writeln!(
            o,
            "{sp}pub fn instantiate(store: embedder::Store) -> ::core::result::Result<Self, embedder::Trap> {{"
        );

        writeln!(o, "{sp}{sp}let allocated = Module {{");
        writeln!(o, "{sp}{sp}{sp}imports: store.imports,");

        for global in context.instantiate_globals.iter() {
            write!(o, "{sp}{sp}{sp}{global}: ");
            if context.types.global_at(global.0).mutable {
                o.write_str("embedder::rt::global::Global::<_>::ZERO");
            } else {
                o.write_str("Default::default()");
            }
            o.write_str(",\n");
        }

        // Technically speaking the limits of the imported memories and tables should be checked before
        // linear memories are "allocated".

        fn print_memory_limits(
            out: &mut dyn crate::write::Write,
            memory_type: &wasmparser::MemoryType,
        ) {
            write!(out, "{}, ", memory_type.initial);
            match memory_type.maximum {
                Some(maximum) => write!(out, "{maximum}"),
                None if memory_type.memory64 => write!(
                    out,
                    "<u64 as embedder::rt::memory::Address>::MAX_PAGE_COUNT"
                ),
                None => write!(
                    out,
                    "<u32 as embedder::rt::memory::Address>::MAX_PAGE_COUNT"
                ),
            }
        }

        // Allocate the linear memories.
        for memory_id in defined_memories {
            let memory_type = context.types.memory_at(memory_id.0);
            write!(
                o,
                "{sp}{sp}{sp}{}: embedder::rt::store::AllocateMemory::allocate(store.memory{}, {}, ",
                crate::context::MemoryIdent::Id(memory_id),
                memory_id.0,
                memory_id.0);

            print_memory_limits(&mut o, &memory_type);
            o.write_str(")?,\n");
        }

        // Functions referred to in declarative element segments.
        for func in context.declarative_func_elements.iter() {
            writeln!(
                o,
                "{sp}{sp}{sp}{func}: ::core::cell::Cell::new(embedder::rt::func_ref::FuncRef::NULL),"
            );
        }

        writeln!(o, "{sp}{sp}}};");

        writeln!(
            o,
            "{sp}{sp}let mut module = embedder::rt::store::AllocateModule::allocate(store.instance, allocated);"
        );

        // Allocate functions referred to in declarative element segments upfront.
        for func in context.declarative_func_elements.iter() {
            let param_count = context.function_signature(func.0).params().len();

            writeln!(o, "{sp}{sp}let _rec = module.clone();");
            write!(
                o,
                "{sp}{sp}module.{}.set(embedder::rt::func_ref::FuncRef::<embedder::Trap>::from_closure_{}(move |",
                func,
                param_count,
            );

            for i in 0..param_count {
                if i > 0 {
                    o.write_str(", ");
                }

                write!(o, "_{i}");
            }

            o.write_str("| ");

            // Cannot use `crate::ast::print::print_stub` here.
            let can_unwind = context.function_attributes.unwind_kind(func.0).can_unwind();
            if !can_unwind {
                o.write_str("Ok(");
            }

            match context.function_attributes.call_kind(func.0) {
                crate::context::CallKind::Method => o.write_str("_rec."),
                crate::context::CallKind::Function => o.write_str("Module::"),
            }

            write!(o, "{}(", context.function_ident(func.0));

            for i in 0..param_count {
                if i > 0 {
                    o.write_str(", ");
                }

                write!(o, "_{i}");
            }

            if !can_unwind {
                o.write_str(")");
            }

            o.write_str(")));\n");
        }

        // TODO: Check table import limits.

        // Check imported linear memory limits.
        for memory_id in
            (0u32..(context.memory_import_names.len() as u32)).map(crate::ast::MemoryId)
        {
            let import = context.memory_import(memory_id).expect("memory import");
            let memory_type = context.types.memory_at(memory_id.0);
            write!(
                o,
                "{sp}{sp}embedder::rt::memory::check_limits(module.{}, {}, ",
                crate::context::MemoryIdent::Import(import),
                memory_id.0,
            );

            print_memory_limits(&mut o, &memory_type);
            o.write_str(")?;\n");
        }

        let mut got_inst_mut = false;
        let mut make_inst_mut = move |o: &mut crate::write::IoWrite| {
            if !got_inst_mut {
                got_inst_mut = true;
                writeln!(o, "{sp}{sp}let mut inst: &mut Module = embedder::rt::store::ModuleAllocation::get_mut(&mut module);");
            }
        };

        // Initialize the globals to their initial values.
        if !context.instantiate_globals.is_empty() {
            make_inst_mut(&mut o);
        }

        // This has to occur after `inst` is created in case there is self-referential stuff.
        for global in context.instantiate_globals.iter().copied() {
            write!(o, "{sp}{sp}");

            let is_mutable = context.types.global_at(global.0).mutable;

            if is_mutable {
                o.write_str("*");
            }

            write!(o, "inst.{global}");

            if is_mutable {
                o.write_str(".get_mut()");
            }

            o.write_str(" = ");

            if let Some(import) = context.global_import(global) {
                write!(o, "inst.{import}()");
            } else {
                let init = context
                    .defined_globals
                    .get(&global)
                    .expect("missing initializer expression for defined global");

                init.print(
                    &mut o,
                    false,
                    &crate::ast::print::Context {
                        wasm: context,
                        arena: &context.constant_expressions,
                        debug_info: self.debug_info,
                    },
                    None,
                );
            }

            o.write_str(";\n");
        }

        // TODO: Copy element segments.

        // Copy active data segments.
        if !context.active_data_segments.is_empty() {
            make_inst_mut(&mut o);
        }

        for data_segment in context.active_data_segments.iter() {
            // TODO: Fix, data segment initialization for imported memories is wrong.
            // TODO: Helper that takes &mut [u8] of linear memory could be used for initializing
            // data segments.
            write!(
                o,
                "{sp}{sp}embedder::rt::memory::init::<{}, _, _, embedder::Trap>(&inst.{}, ",
                data_segment.memory.0,
                context.memory_ident(data_segment.memory)
            );

            data_segment.offset.print(
                &mut o,
                false,
                &crate::ast::print::Context {
                    wasm: context,
                    arena: &context.constant_expressions,
                    debug_info: self.debug_info,
                },
                None,
            );

            let data_length = context.data_segment_contents[data_segment.data.0 as usize].len();

            writeln!(
                o,
                ", 0, {data_length}, Module::{}, None)?;",
                data_segment.data
            );
        }

        if let Some(start_function) = context.start_function {
            writeln!(
                o,
                "{sp}{sp}core::todo!(\"call start function #{}\");",
                start_function.0
            );
        }

        writeln!(o, "{sp}{sp}Ok(Self(Some(module)))");
        writeln!(o, "{sp}}}");

        // Write function to get a new `Module`
        writeln!(
            o,
            "\n{sp}pub fn leak(module: Self) -> embedder::Module<Module> {{"
        );
        writeln!(
            o,
            "{sp}{sp}let mut module = ::core::mem::ManuallyDrop::new(module);"
        );
        writeln!(
            o,
            "{sp}{sp}::core::mem::take(&mut module.0).unwrap()\n{sp}}}"
        );

        o.write_str("} // impl Instance\n");

        o.write_str("\nimpl ::core::ops::Deref for Instance {\n");
        write!(o, "{sp}type Target = embedder::Module<Module>;\n\n");
        writeln!(o, "{sp}fn deref(&self) -> &Self::Target {{");
        write!(
            o,
            "{sp}{sp}self.0.as_ref().unwrap()\n{sp}}}\n}} // impl Deref\n"
        );

        o.write_str("\nimpl ::core::ops::Drop for Instance {\n");
        writeln!(o, "\n{sp}fn drop(&mut self) {{");

        writeln!(o, "{sp}{sp}if embedder::rt::thread::panicking() {{");
        writeln!(o, "{sp}{sp}{sp}return;\n{sp}{sp}}}\n");

        writeln!(
            o,
            "{sp}{sp}let _module = embedder::rt::store::ModuleAllocation::get_mut(self.0.as_mut().unwrap());"
        );

        // Any `FuncRef`'s within the module are replaced with `NULL` so there are no
        // cyclic references leading to `Rc<Self>` never being freed.

        // Replace all function references originating from declarative element segments.
        for func in context.declarative_func_elements.iter() {
            writeln!(
                o,
                "{sp}{sp}*_module.{func}.get_mut() = embedder::rt::func_ref::FuncRef::NULL;"
            );
        }

        // TODO: Free all passive element segments.

        // TODO: Free all tables containing FuncRefs.

        writeln!(o, "{sp}}}\n}} // impl Drop");

        o.write_str("\n} // mod $module\n\n");
        writeln!(o, "{sp}}}"); // ($vis mod $module use $path)
        o.write_str("}\n"); // macro_rules!

        o.into_inner()?.flush()?;
        Ok(())
    }

    /// Allows access to information collected about the WebAssembly module during conversion.
    ///
    /// If this information is not needed, use [`convert_from_buffer_with_allocations()`] instead.
    ///
    /// [`convert_from_buffer_with_allocations()`]: Convert::convert_from_buffer_with_allocations()
    pub fn convert_from_buffer_with_intermediate<'wasm, O, F>(
        &self,
        wasm: &'wasm [u8],
        allocations: &Allocations,
        intermediate: F,
    ) -> crate::Result<O>
    where
        F: FnOnce(Intermediate<'_, 'wasm>) -> crate::Result<O>,
        O: std::io::Write,
    {
        use anyhow::Context;

        let Ast {
            context,
            function_definitions,
        } = self
            .convert_to_ast(wasm, allocations)
            .context("could not construct AST of WebAssembly module")?;

        let mut output = intermediate(Intermediate { context: &context })
            .context("could not get output writere")?;

        self.print_ast(allocations, &context, function_definitions, &mut output)
            .context("could not print Rust source code")?;

        context.finish(allocations);

        Ok(output)
    }

    /// Allows reusing [`Allocations`] between multiple calls to [`convert_from_buffer()`]. This is
    /// useful if multiple WebAssembly modules are being converted.
    ///
    /// [`convert_from_buffer()`]: Self::convert_from_buffer
    pub fn convert_from_buffer_with_allocations(
        &self,
        wasm: &[u8],
        output: &mut dyn std::io::Write,
        allocations: &Allocations,
    ) -> crate::Result<()> {
        self.convert_from_buffer_with_intermediate(wasm, allocations, |_| Ok(output))?;
        Ok(())
    }

    /// Converts an in-memory WebAssembly binary module, and [`Write`]s the resulting Rust source
    /// code to the given output.
    ///
    /// To reuse [`Allocations`], use the [`convert_from_buffer_with_allocations()`] method
    /// instead.
    ///
    /// # Errors
    ///
    /// An error will be returned if the WebAssembly module could not be parsed, the module
    /// [could not be validated], or if an error occured while writing to the `output`.
    ///
    /// [`Write`]: std::io::Write
    /// [`convert_from_buffer_with_allocations()`]: Convert::convert_from_buffer_with_allocations()
    /// [could not be validated]: https://webassembly.github.io/spec/core/valid/index.html
    pub fn convert_from_buffer(
        &self,
        wasm: &[u8],
        output: &mut dyn std::io::Write,
    ) -> crate::Result<()> {
        Self::convert_from_buffer_with_allocations(self, wasm, output, &Allocations::default())
    }
}

impl<'conv, 'wasm> Intermediate<'conv, 'wasm> {
    /// Provides access to type information about the WebAssembly module.
    pub fn types(&self) -> &'conv wasmparser::types::Types {
        &self.context.types
    }

    /// Returns `true` if the module has imports of any kind.
    pub fn has_imports(&self) -> bool {
        self.context.has_imports()
    }

    // pub fn function_imports_indices(&self) -> impl ExactSizeIterator<Item = (crate::ast::FuncId, &'conv crate::ident::BoxedIdent<'wasm>)> + 'conv {
    //     self.context.func_import_names
    // }

    /// Returns an [`Iterator`] over the function exports in *arbitrary* order, yielding their
    /// names and types.
    pub fn function_export_types(
        &self,
    ) -> impl Iterator<Item = (crate::ident::SafeIdent<'wasm>, &'conv wasmparser::FuncType)> + 'conv
    {
        self.context
            .function_export_names
            .iter()
            .flat_map(|(id, names)| {
                let signature: &'conv _ = self.context.function_signature(*id);
                names
                    .iter()
                    .copied()
                    .map(crate::ident::SafeIdent::from)
                    .zip(std::iter::repeat(signature))
            })
    }
}
