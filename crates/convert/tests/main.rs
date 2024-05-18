//! Unit tests for code generation.

macro_rules! snapshots {
    ($(
        $name:ident($rust_capacity:literal) = $file:expr;
    )*) => {$(
        #[test]
        fn $name() {
            let wasm = wat::parse_str($file).unwrap();

            let mut rust = Vec::with_capacity($rust_capacity);
            wasm2rs_convert::Convert::new()
                .convert_from_buffer(&wasm, &mut rust)
                .unwrap();

            insta::assert_snapshot!(String::from_utf8_lossy(&rust))
        }
    )*};
}

snapshots! {
    add_5(64) = r#"(module
    (func (export "add_five") (param i32) (result i32)
        local.get 0
        i32.const 5
        i32.add))
"#;
    simple(512) = include_str!("simple.wat");
    memory(256) = include_str!("memory.wat");
    imports(256) = include_str!("imports.wat");
}
