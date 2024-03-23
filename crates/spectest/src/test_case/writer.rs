use std::fmt::Write;

pub fn write_unit_tests<'wasm>(
    modules: &[crate::test_case::Module<'wasm>],
    wast: &crate::location::Contents<'wasm>,
    output_dir: &std::path::Path,
    buffer_pool: &wasm2rs::buffer::Pool,
) -> Vec<bytes::BytesMut> {
    let mut out = wasm2rs::buffer::Writer::new(&buffer_pool);
    let _ = writeln!(out, "// Generated from {:?}\n", wast.path);

    let mut module_path = std::path::PathBuf::from(output_dir);
    let mut module_name = String::new();
    for module in modules {
        module_name.clear();
        let _ = write!(&mut module_name, "{}", module.into_ident());

        let module_location = wast.location(module.span);
        let _ = writeln!(out, "// {module_location}\nmod {module_name} {{");

        module_path.push(&module_name);
        module_path.set_extension("rs");

        let _ = writeln!(out, "  include!({module_path:?});\n");

        module_path.pop();

        let _ = writeln!(out, "  wasm!(pub mod wasm)\n");

        out.write_str(concat!(
            "  #[test]\n  fn tests() {\n",
            "    let _inst = wasm::Instance::instantiate(Default::default()).expect(\"could not",
            " instantiate module at "
        ));

        let _ = writeln!(out, "{module_location}\");");

        for statement in module.statements.iter() {
            match &statement.kind {
                crate::test_case::StatementKind::InvokeFunction {
                    name,
                    arguments,
                    result,
                } => {
                    let _ = writeln!(
                        out,
                        "    let {} = _inst.{}{arguments};",
                        crate::test_case::Statement::RESULT_VARIABLE,
                        wasm2rs::rust::SafeIdent::from(*name)
                    );

                    match result {
                        Some(pattern) => {
                            let _ = writeln!(out, "    assert_eq!({}, {pattern}, \"assertion failed at {module_location}\");",
                            crate::test_case::Statement::RESULT_VARIABLE);
                        }
                        None => (),
                    }
                }
            }
        }

        out.write_str("  }\n}\n\n");
    }

    out.finish()
}
