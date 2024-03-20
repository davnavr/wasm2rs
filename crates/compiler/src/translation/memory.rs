use std::fmt::Write;

pub fn write(
    buffer_pool: &crate::buffer::Pool,
    section: wasmparser::MemorySectionReader,
    start_index: u32,
) -> wasmparser::Result<crate::translation::GeneratedLines> {
    let mut field_out = crate::buffer::Writer::new(buffer_pool);
    let mut init_out = crate::buffer::Writer::new(buffer_pool);

    for (result, index) in section.into_iter().zip(start_index..) {
        let memory = result?;
        let id = crate::translation::display::MemId(index);
        let _ = writeln!(
            field_out,
            "    {id}: {}::Memory{index},",
            crate::translation::EMBEDDER_PATH
        );
        let _ = writeln!(
            init_out,
            "      let {id} = {}::init{id}(embedder)?;",
            crate::translation::EMBEDDER_PATH
        );
        debug_assert!(!memory.shared);
        debug_assert!(!memory.memory64);
    }

    Ok(crate::translation::GeneratedLines {
        fields: field_out.finish(),
        inits: init_out.finish(),
        ..Default::default()
    })
}
