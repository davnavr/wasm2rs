use std::fmt::Write;

const PREFER_LITERAL_LENGTH: usize = 64;

fn write_data_literal(out: &mut crate::buffer::Writer, data: &[u8]) {
    out.write_str("b\"");
    for b in data {
        let _ = write!(out, "{}", std::ascii::escape_default(*b));
    }
    out.write_str("\"");
}

pub fn write(
    buffer_pool: &crate::buffer::Pool,
    section: wasmparser::DataSectionReader,
    writer: crate::DataSegmentWriter,
) -> crate::Result<crate::translation::GeneratedLines> {
    let mut item_out = crate::buffer::Writer::new(buffer_pool);
    let mut init_out = crate::buffer::Writer::new(buffer_pool);

    for (index, result) in (0u32..section.count()).zip(section) {
        use wasmparser::DataKind;

        let data = result?;

        let _ = write!(
            item_out,
            "const {}: &[u8] = ",
            crate::translation::display::MemId(index)
        );

        if data.data.len() <= PREFER_LITERAL_LENGTH {
            write_data_literal(&mut item_out, data.data);
        }
        if let Some(path) = writer(index, data.data)? {
            let _ = write!(
                item_out,
                "::core::include_bytes!({});",
                path.escape_default()
            );
        } else {
            write_data_literal(&mut item_out, data.data);
        }

        item_out.write_str(";\n");

        match data.kind {
            DataKind::Active {
                memory_index,
                offset_expr,
            } => {
                todo!("active data segments not supported")
            }
            DataKind::Passive => (),
        }
    }

    Ok(crate::translation::GeneratedLines {
        items: item_out.finish(),
        inits: init_out.finish(),
        ..Default::default()
    })
}
