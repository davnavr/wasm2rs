use std::fmt::Write;

#[derive(Clone, Copy, Debug)]
pub struct ExportEntry {
    pub name: u32,
    pub index: u32,
}

#[derive(Default, Debug)]
pub struct Exports<'a> {
    pub names: Vec<&'a str>,
    pub functions: Vec<ExportEntry>,
}

fn write_function_export(
    out: &mut crate::buffer::Writer,
    index: crate::translation::display::FuncId,
    types: &wasmparser::types::Types,
) {
    let func_type = crate::translation::function::get_function_type(
        types.get(types.core_function_at(index.0)).unwrap(),
    );
    crate::translation::function::write_definition_signature(out, func_type);
    let _ = write!(out, " {{ self.{index}(");

    let param_count = u32::try_from(func_type.params().len()).unwrap();
    for i in 0..param_count {
        if i > 0 {
            out.write_str(", ");
        }

        let _ = write!(out, "{}", crate::translation::display::LocalId(i));
    }

    out.write_str(") }\n");
}

pub fn write<'a>(
    buffer_pool: &crate::buffer::Pool,
    section: wasmparser::ExportSectionReader<'a>,
    types: &wasmparser::types::Types,
) -> crate::Result<crate::translation::GeneratedLines> {
    let mut impl_out = crate::buffer::Writer::new(buffer_pool);

    impl_out.write_str("    // Exports\n");

    let mut func_export_symbols = indexmap::IndexMap::<u32, Vec<&'a str>>::with_capacity(
        usize::try_from(section.count()).unwrap_or_default() / 4,
    );

    for result in section {
        use wasmparser::ExternalKind;

        let export = result?;
        let _ = write!(
            impl_out,
            "    $vis fn {}",
            crate::rust::SafeIdent::from(export.name),
        );

        match export.kind {
            ExternalKind::Func => {
                write_function_export(
                    &mut impl_out,
                    crate::translation::display::FuncId(export.index),
                    types,
                );

                let export_names = func_export_symbols.entry(export.index).or_default();
                export_names.push(export.name);
            }
            ExternalKind::Memory => {
                let index = crate::translation::display::MemId(export.index);
                let _ = writeln!(
                    impl_out,
                    "(&self) -> &embedder::Memory{} {{ &self.{index} }}",
                    index.0
                );
            }
            ExternalKind::Global => {
                let index = crate::translation::display::GlobalId(export.index);
                let global_type = types.global_at(index.0);
                let value_type = crate::translation::display::ValType(global_type.content_type);

                let _ = write!(impl_out, "(&self) -> &");
                if global_type.mutable {
                    let _ = write!(impl_out, "embedder::rt::global::Global<{value_type}>");
                } else {
                    let _ = write!(impl_out, "{value_type}");
                }

                let _ = writeln!(impl_out, "  {{ &self.{index} }}");
            }
            _ => todo!("unsupported export: {export:?}"),
        }
    }

    impl_out.write_str("\n");

    for (func, names) in func_export_symbols.into_iter() {
        let _ = write!(
            impl_out,
            "    const {}: embedder::rt::trap::WasmSymbol = {{ \
            let mut s = embedder::rt::trap::WasmSymbol::from_index_and_offset({func}, Self::{}); \
            s.export_names = &[",
            crate::translation::display::FuncSymbol(func),
            crate::translation::display::CodeOffset(func),
        );

        for (i, name) in names.into_iter().enumerate() {
            if i > 0 {
                impl_out.write_str(", ");
            }

            let _ = write!(impl_out, "\"{}\"", name.escape_default());
        }

        impl_out.write_str("]; s };\n");
    }

    Ok(crate::translation::GeneratedLines {
        impls: impl_out.finish(),
        ..Default::default()
    })
}
