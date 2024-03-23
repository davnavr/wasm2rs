use std::fmt::Write;

pub fn write_unit_tests<'wasm>(
    modules: Vec<crate::test_case::Module<'wasm>>,
    buffer_pool: &wasm2rs::buffer::Pool,
) -> Vec<bytes::BytesMut> {
    let mut out = wasm2rs::buffer::Writer::new(&buffer_pool);

    out.finish()
}
