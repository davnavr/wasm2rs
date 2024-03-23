use std::fmt::Write;

const ACTUAL_VARIABLE: &str = "actual";

fn write_result_check(
    out: &mut wasm2rs::buffer::Writer,
    result: &crate::test_case::ResultValue,
    location: &crate::location::Location<'_>,
) {
    use crate::test_case::ResultValue;
    out.write_str("    assert!(");
    match result {
        ResultValue::I32(i) => {
            let _ = write!(out, "{ACTUAL_VARIABLE} == {i}");
        }
        ResultValue::I64(i) => {
            let _ = write!(out, "{ACTUAL_VARIABLE} == {i}");
        }
        ResultValue::F32Bits(bits) => {
            let _ = write!(out, "f32::to_bits({ACTUAL_VARIABLE}) == {bits:#010X}u32");
        }
        ResultValue::F64Bits(bits) => {
            let _ = write!(out, "f64::to_bits({ACTUAL_VARIABLE}) == {bits:#018X}u64");
        }
    }

    out.write_str(", \"expected ");
    match result {
        ResultValue::I32(i) => {
            let _ = write!(out, "{i} ({i:#010X})");
        }
        ResultValue::I64(i) => {
            let _ = write!(out, "{i} ({i:018X})");
        }
        ResultValue::F32Bits(bits) => {
            let _ = write!(out, "{} ({bits:#010X})", f32::from_bits(*bits));
        }
        ResultValue::F64Bits(bits) => {
            let _ = write!(out, "{} ({bits:#018X})", f64::from_bits(*bits));
        }
    }

    let _ = writeln!(out, " but got {ACTUAL_VARIABLE} at {location}\n\");");
}

pub fn write_unit_tests<'wasm>(
    modules: &[crate::test_case::Module<'wasm>],
    wast: &crate::location::Contents<'wasm>,
    output_dir: &std::path::Path,
    buffer_pool: &wasm2rs::buffer::Pool,
) -> crate::Result<Vec<bytes::BytesMut>> {
    let mut out = wasm2rs::buffer::Writer::new(&buffer_pool);
    let _ = writeln!(out, "// Generated from {:?}\n", wast.path);

    {
        let file_name = wast
            .path
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("unable to get file name for {:?}", wast.path))?
            .to_string_lossy();

        let _ = writeln!(
            out,
            "mod {} {{\n\n",
            wasm2rs::rust::SafeIdent::from(file_name.as_ref())
        );
    }

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

        let _ = writeln!(out, "  wasm!(pub mod wasm);\n");

        out.write_str(concat!(
            "  #[test]\n  fn tests() {\n",
            "    let _inst = wasm::Instance::instantiate(Default::default()).expect(\"could not",
            " instantiate module at "
        ));

        let _ = writeln!(out, "{module_location}\");");

        for statement in module.statements.iter() {
            let _ = writeln!(out, "    // {}", wast.location(statement.span));

            match &statement.kind {
                crate::test_case::StatementKind::InvokeFunction {
                    name,
                    arguments,
                    result,
                } => {
                    use crate::test_case::ActionResult;

                    const RESULT_VARIABLE: &str = crate::test_case::Statement::RESULT_VARIABLE;

                    let _ = writeln!(
                        out,
                        "    let {RESULT_VARIABLE} = _inst.{}{arguments};",
                        wasm2rs::rust::SafeIdent::from(*name)
                    );

                    match result {
                        Some(ActionResult::Trap(trap)) => {
                            let _ = writeln!(out, "    assert!(matches!({RESULT_VARIABLE}, Err(ref e) if matches!(e.code(), {trap})), \"expected trap but got {{:?}} at {module_location}\", {RESULT_VARIABLE});",
                            );
                        }
                        Some(ActionResult::Values(values)) => {
                            out.write_str("    ");
                            if values.is_empty() {
                                let _ = writeln!(out, "assert_eq!({RESULT_VARIABLE}, Ok(()), \"unexpected trap {{:?}} at {module_location}\", {RESULT_VARIABLE});");
                            } else {
                                let _ = writeln!(out, "assert!({RESULT_VARIABLE}.is_ok(), \"unexpected trap {{:?}} at {module_location}\", {RESULT_VARIABLE});");

                                out.write_str("    let ");

                                if values.len() > 1 {
                                    out.write_str("(");
                                }

                                for i in 0..values.len() {
                                    if i > 0 {
                                        out.write_str(", ");
                                    }

                                    let _ = write!(out, "actual_{i}");
                                }

                                if values.len() > 1 {
                                    out.write_str(")");
                                }

                                let _ = writeln!(out, " = _result.unwrap();");

                                for (i, result) in values.iter().enumerate() {
                                    let _ =
                                        writeln!(out, "    let {ACTUAL_VARIABLE} = actual_{i};");
                                    write_result_check(&mut out, result, &module_location);
                                }
                            }
                        }
                        None => (),
                    }
                }
            }
        }

        out.write_str("  }\n}\n\n");
    }

    out.write_str("\n}\n");
    Ok(out.finish())
}
