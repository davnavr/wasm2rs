//! Unit tests for code generation.

#[test]
fn add_5() {
    let wasm = wat::parse_str(
        r#"(module
    (func (export "add_five") (param i32) (result i32)
        local.get 0
        i32.const 5
        i32.add))
"#,
    )
    .unwrap();

    let mut rust = Vec::with_capacity(256);
    wasm2rs_convert::Convert::new()
        .convert_from_buffer(&wasm, &mut rust)
        .unwrap();

    insta::assert_snapshot!(String::from_utf8_lossy(&rust))
}
