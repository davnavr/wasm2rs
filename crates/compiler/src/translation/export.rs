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
    name: &str,
    index: crate::translation::display::FuncId,
    types: &wasmparser::types::Types,
) {
    let _ = write!(
        out,
        "$vis fn {}(&self, ",
        crate::rust::SafeIdent::from(name),
    );
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

pub fn write(
    buffer_pool: &crate::buffer::Pool,
    section: wasmparser::ExportSectionReader,
    types: &wasmparser::types::Types,
) -> wasmparser::Result<crate::translation::GeneratedLines> {
    let mut impl_out = crate::buffer::Writer::new(buffer_pool);

    for result in section {
        use wasmparser::ExternalKind;

        let export = result?;
        match export.kind {
            ExternalKind::Func => write_function_export(
                &mut impl_out,
                export.name,
                crate::translation::display::FuncId(export.index),
                types,
            ),
            _ => todo!("unsupported export: {export:?}"),
        }
    }

    Ok(crate::translation::GeneratedLines {
        impls: impl_out.finish(),
        ..Default::default()
    })
}
