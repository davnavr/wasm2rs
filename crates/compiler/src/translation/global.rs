use std::fmt::Write;

pub fn write(
    buffer_pool: &crate::buffer::Pool,
    section: wasmparser::GlobalSectionReader,
    start_index: u32,
) -> wasmparser::Result<crate::translation::GeneratedLines> {
    let mut field_out = crate::buffer::Writer::new(buffer_pool);
    let mut init_out = crate::buffer::Writer::new(buffer_pool);

    for (result, index) in section.into_iter().zip(start_index..) {
        let global = result?;
        let id = crate::translation::display::GlobalId(index);
        let val_type = crate::translation::display::ValType(global.ty.content_type);

        let _ = write!(field_out, "    {id}: ");
        if global.ty.mutable {
            let _ = write!(field_out, "embedder::rt::Global<{val_type}>");
        } else {
            let _ = write!(field_out, "{val_type}");
        }

        field_out.write_str(",\n");

        let _ = write!(init_out, "let {id} = ");
        if global.ty.mutable {
            init_out.write_str("embedder::rt::Global::new(");
        }

        crate::translation::const_expr::write(&mut init_out, &global.init_expr)?;

        if global.ty.mutable {
            init_out.write_str(")");
        }

        init_out.write_str(";\n");
    }

    Ok(crate::translation::GeneratedLines {
        fields: field_out.finish(),
        inits: init_out.finish(),
        ..Default::default()
    })
}
