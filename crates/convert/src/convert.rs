//! Contains the core code for converting WebAssembly to Rust.

mod allocations;
mod code;
mod constant;
mod options;

pub use allocations::Allocations;
pub use options::{DataSegmentWriter, DebugInfo, StackOverflowChecks};

/// Provides options for converting a [WebAssembly binary module] into a [Rust source file].
///
/// [WebAssembly binary module]: https://webassembly.github.io/spec/core/binary/index.html
/// [Rust source file]: https://doc.rust-lang.org/reference/crates-and-source-files.html
pub struct Convert<'a> {
    indentation: crate::Indentation,
    generated_macro_name: crate::ident::SafeIdent<'a>,
    data_segment_writer: DataSegmentWriter<'a>,
    // wasm_features: &'a wasmparser::WasmFeatures,
    stack_overflow_checks: StackOverflowChecks,
    debug_info: DebugInfo,
}

impl std::fmt::Debug for Convert<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Convert")
            .field("indentation", &self.indentation)
            .field("generated_macro_name", &self.generated_macro_name)
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
            generated_macro_name: crate::ident::Ident::DEFAULT_MACRO_NAME.into(),
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
    const SUPPORTED_FEATURES: wasmparser::WasmFeatures = wasmparser::WasmFeatures {
        mutable_global: true,
        saturating_float_to_int: true,
        sign_extension: true,
        reference_types: false,
        multi_value: true,
        bulk_memory: true,
        simd: false,
        relaxed_simd: false,
        threads: false,
        tail_call: false,
        floats: true,
        multi_memory: true,
        exceptions: false,
        memory64: false,
        extended_const: false,
        component_model: false,
        function_references: false,
        memory_control: false,
        gc: false,
        component_model_values: false,
        component_model_nested_names: false,
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
            Payload::ModuleSection { parser: _, range } => validator.module_section(&range)?,
            Payload::InstanceSection(section) => validator.instance_section(&section)?,
            Payload::CoreTypeSection(section) => validator.core_type_section(&section)?,
            Payload::ComponentSection { parser: _, range } => {
                validator.component_section(&range)?
            }
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
        let mut imported_modules = Vec::with_capacity((import_section.count() as usize).min(2));

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
            let module_idx = if let Some(existing) = imported_modules
                .iter()
                .position(|name| *name == import.module)
            {
                crate::context::ImportedModule(existing as u16)
            } else {
                use anyhow::Context;

                let idx = u16::try_from(imported_modules.len()).with_context(|| {
                    format!("cannot import from more than 65535 modules @ {import_offset:#X}")
                })?;

                imported_modules.push(import.module);
                crate::context::ImportedModule(idx)
            };

            match import.ty {
                TypeRef::Func(_) => {
                    // TODO: set call kind for imported functions
                    // call_kinds.push(crate::context::CallKind::WithEmbedder);

                    func_import_names.push(import.name);
                    func_import_modules.push(module_idx);
                }
                TypeRef::Memory(_) => {
                    memory_import_names.push(import.name);
                    memory_import_modules.push(module_idx);
                    memory_idx.0 += 1;
                }
                TypeRef::Global(global_type) => {
                    global_import_names.push(import.name);
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

        imported_modules.resize(imported_modules.capacity(), "THIS IS A BUG");

        // Don't need to store the capacity of these `Vec`s.
        context.imported_modules = imported_modules.into_boxed_slice();
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

        for result in export_section.into_iter_with_offsets() {
            use wasmparser::ExternalKind;

            let (export_offset, export) = result?;
            match export.kind {
                ExternalKind::Func => {
                    let func_idx = crate::ast::FuncId(export.index);
                    context.function_export_names.insert(func_idx, export.name);
                }
                ExternalKind::Memory => {
                    let mem_idx = crate::ast::MemoryId(export.index);
                    context.memory_exports.push(mem_idx);
                    context.memory_export_names.insert(mem_idx, export.name);
                }
                ExternalKind::Global => {
                    let global_idx = crate::ast::GlobalId(export.index);
                    context.global_exports.push(global_idx);
                    context.global_export_names.insert(global_idx, export.name);
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

    // element sec

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

impl Convert<'_> {
    fn convert_function_definitions<'wasm, 'types>(
        &self,
        types: &'types wasmparser::types::Types,
        allocations: &crate::Allocations,
        attributes: &mut crate::context::FunctionAttributes,
        code: Vec<code::Code<'wasm>>,
    ) -> crate::Result<Vec<code::Definition>> {
        let convert_function_bodies =
            move |call_kind: &mut crate::context::CallKind,
                  unwind_kind: &mut crate::context::UnwindKind,
                  code: code::Code<'wasm>| {
                let (attr, definition) = code.convert(allocations, self, types)?;
                *call_kind = attr.call_kind;
                *unwind_kind = attr.unwind_kind;
                Ok(definition)
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
            call_kinds: vec![crate::context::CallKind::Function; function_count].into_boxed_slice(),
            unwind_kinds: vec![crate::context::UnwindKind::Maybe; function_count]
                .into_boxed_slice(),
        };

        let function_definitions = self.convert_function_definitions(
            &types,
            allocations,
            &mut function_attributes,
            function_bodies,
        )?;

        let context = parse_sections(types, function_attributes, allocations, sections)?;

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

        let printer_options = crate::ast::Print::new(self.indentation, context);
        let write_function_definitions = |(index, definition): (usize, code::Definition)| {
            use crate::context::CallKind;

            let context = printer_options.context();

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

            printer_options.print_statements(
                func_id,
                1,
                &mut out,
                &definition.body,
                &definition.arena,
            );

            out.write_str("}\n");

            // Check if an additional Rust function must be generated to access the function
            // export. A stub function is used to hides implementation details, such as the
            // possible omission of the `&self` parameter in the original function.
            match context.function_export_names.get(&func_id) {
                Some(export) if matches!(function_name, crate::context::FunctionName::Id(_)) => {
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

                    printer_options.print_stub(
                        1,
                        &mut out,
                        func_id,
                        signature.params().len() as u32,
                    );

                    out.write_str("}\n");
                }
                _ => (),
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
        writeln!(o, "macro_rules! {} {{", self.generated_macro_name);
        o.write_str(sp);
        o.write_str(concat!(
            "($vis:vis mod $module:ident use $(:: $embedder_start:ident ::)? ",
            "$($embedder_more:ident)::+) => {\n\n"
        ));

        // TODO: After `$module`, add `$(<$lifetime:lifetime>)?`

        o.write_str("#[allow(non_snake_case)]\n"); // Names might be mangled
        o.write_str("#[allow(dead_code)]\n"); // Some functions may not be called
        o.write_str("#[allow(unreachable_code)]\n"); // Some branches may not be taken (e.g. infinite loops detected by rustc)
        o.write_str("#[allow(unreachable_pub)]\n"); // Macro may be invoked within a non-public module.
        o.write_str("$vis mod $module {\n\n");
        o.write_str("use $(::$embedder_start::)? $($embedder_more)::+ as embedder;\n");
        o.write_str("use embedder::rt::trap::TrapWith as _;\n\n");

        // TODO: Option to specify #[derive(Debug)] impl
        writeln!(o, "pub struct Instance {{\n{sp}imports: embedder::Imports,");

        let defined_memories = ((context.global_import_names.len() as u32)
            ..context.types.memory_count())
            .map(crate::ast::MemoryId);

        for memory_id in defined_memories.clone() {
            writeln!(o, "{sp}{memory_id}: embedder::Memory{},", memory_id.0);
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

        writeln!(o, "}}\n");

        writeln!(o, "impl Instance {{");

        // Write constant globals.
        for global in context.constant_globals.iter() {
            match context.constant_expressions.get(global.initializer) {
                crate::ast::Expr::Literal(literal) => writeln!(
                    o,
                    "{sp}const {:#}: {} = {literal};",
                    global.id,
                    literal.type_of()
                ),
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
                    let _ = write!(out, "{}", std::ascii::escape_default(*b));
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
        for memory_id in context.memory_exports.iter().copied() {
            let export = *context
                .memory_export_names
                .get(&memory_id)
                .expect("memory export did not have a name");

            write!(
                o,
                "\n{sp}pub fn {}(&self) -> &embedder::Memory{} {{\n{sp}{sp}",
                crate::ident::SafeIdent::from(export),
                memory_id.0
            );

            match context.memory_import(memory_id) {
                None => write!(o, "&self.{memory_id}"),
                Some(import) => write!(
                    o,
                    "self.imports.{}().{}()",
                    crate::ident::SafeIdent::from(*import.module),
                    crate::ident::SafeIdent::from(*import.name)
                ),
            }

            writeln!(o, "\n{sp}}}\n");
        }

        // Write exported globals.
        for global_id in context.global_exports.iter().copied() {
            let export = *context
                .global_export_names
                .get(&global_id)
                .expect("global export did not have a name");

            write!(
                o,
                "\n{sp}pub fn {}(&self) -> ",
                crate::ident::SafeIdent::from(export)
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

        // Write module's `instantiate()` method.
        writeln!(
            o,
            "\n{sp}pub fn instantiate(store: embedder::Store) -> ::core::result::Result<embedder::Module<Self>, embedder::Trap> {{"
        );

        writeln!(o, "{sp}{sp}let allocated = Self {{");
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

        // TODO: Initialize tables

        // Allocate the linear memories.
        for memory_id in defined_memories {
            let memory_type = context.types.memory_at(memory_id.0);
            writeln!(
                o,
                "{sp}{sp}{sp}{memory_id}: embedder::rt::store::AllocateMemory::allocate(store.memory{}, {}, {}, {})?,",
                memory_id.0,
                memory_id.0,
                memory_type.initial,
                match memory_type.maximum {
                    Some(maximum) => maximum,
                    None if memory_type.memory64 => u64::MAX,
                    None => u32::MAX.into(),
                });
        }

        writeln!(o, "{sp}{sp}}};");

        writeln!(
            o,
            "{sp}{sp}let mut module = embedder::rt::store::AllocateModule::allocate(store.instance, allocated);"
        );

        let mut got_inst_mut = false;
        let mut make_inst_mut = move |o: &mut crate::write::IoWrite| {
            if !got_inst_mut {
                got_inst_mut = true;
                writeln!(o, "{sp}{sp}let mut inst: &mut Self = embedder::rt::store::ModuleAllocation::get_mut(&mut module);");
            }
        };

        // Initialize the globals to their initial values.
        if !context.instantiate_globals.is_empty() {
            make_inst_mut(&mut o);
        }

        // This has to occur after `inst` is created in case there is self-referential stuff.
        for global in context.instantiate_globals.iter().copied() {
            write!(o, "{sp}{sp}*inst.{global}");

            if context.types.global_at(global.0).mutable {
                o.write_str(".get_mut()");
            }

            o.write_str(" = ");

            if let Some(import) = context.global_import(global) {
                anyhow::bail!("initialize global import {import:?}");
            } else {
                let init = context
                    .defined_globals
                    .get(&global)
                    .expect("missing initializer expression for defined global");

                init.print(&mut o, &context.constant_expressions, false, context);
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
                data_segment.memory.0, data_segment.memory
            );

            data_segment
                .offset
                .print(&mut o, &context.constant_expressions, false, context);

            let data_length = context.data_segment_contents[data_segment.data.0 as usize].len();

            writeln!(
                o,
                ", 0, {data_length}, Self::{}, None)?;",
                data_segment.data
            );
        }

        if let Some(start_function) = context.start_function {
            writeln!(o, "{sp}{sp}// TODO: call {start_function}");
        }

        writeln!(o, "{sp}{sp}Ok(module)");
        writeln!(o, "{sp}}}");

        // Write function definitions and their bodies.
        let output = o.into_inner()?;
        output.flush()?; // If a `BufWriter` is being used, this might allow it to be bypassed.
        crate::buffer::write_all_vectored(
            output,
            &function_items,
            &mut Vec::with_capacity(function_items.len()),
        )?;

        let mut o = crate::write::IoWrite::new(output);
        o.write_str("\n} // impl Instance\n\n");
        o.write_str("} // mod $module\n\n");
        writeln!(o, "{sp}}}"); // ($vis mod $module use $path)
        o.write_str("}\n"); // macro_rules!

        o.into_inner()?.flush()?;
        Ok(())
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
        use anyhow::Context;

        let Ast {
            context,
            function_definitions,
        } = self
            .convert_to_ast(wasm, allocations)
            .context("could not construct AST of WebAssembly module")?;

        self.print_ast(allocations, &context, function_definitions, output)
            .context("could not print Rust source code")?;

        context.finish(allocations);

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
