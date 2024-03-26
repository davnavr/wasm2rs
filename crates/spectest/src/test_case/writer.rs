use std::fmt::Write;

const ACTUAL_VARIABLE: &str = "actual";

fn write_result_check(
    out: &mut wasm2rs::buffer::Writer,
    result: &crate::test_case::ResultValue,
    location: &crate::location::Location<'_>,
) {
    use crate::test_case::ResultValue;

    out.write_str("    assert!(\n      ");
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
        // See `tests/spec/src/nan.rs`
        ResultValue::F32CanonicalNan => {
            let _ = write!(out, "crate::nan::is_canonical_f32({ACTUAL_VARIABLE})",);
        }
        ResultValue::F32ArithmeticNan => {
            let _ = write!(out, "crate::nan::is_arithmetic_f32({ACTUAL_VARIABLE})",);
        }
        ResultValue::F64CanonicalNan => {
            let _ = write!(out, "crate::nan::is_canonical_f64({ACTUAL_VARIABLE})",);
        }
        ResultValue::F64ArithmeticNan => {
            let _ = write!(out, "crate::nan::is_arithmetic_f64({ACTUAL_VARIABLE})",);
        }
    }

    out.write_str(",\n      \"expected ");

    match result {
        ResultValue::I32(i) => {
            let _ = write!(out, "{i} ({i:#010X})");
        }
        ResultValue::I64(i) => {
            let _ = write!(out, "{i} ({i:#018X})");
        }
        ResultValue::F32Bits(bits) => {
            let _ = write!(out, "{:e} ({bits:#010X})", f32::from_bits(*bits));
        }
        ResultValue::F64Bits(bits) => {
            let _ = write!(out, "{:e} ({bits:#018X})", f64::from_bits(*bits));
        }
        ResultValue::F32CanonicalNan | ResultValue::F64CanonicalNan => {
            out.write_str("canonical NaN ({:#010X} or {:#010X})");
        }
        ResultValue::F32ArithmeticNan | ResultValue::F64ArithmeticNan => {
            out.write_str("arithmetic NaN");
        }
    }

    out.write_str(" but got ");

    match result {
        ResultValue::I32(_) => {
            let _ = write!(out, "{{{ACTUAL_VARIABLE}}} ({{{ACTUAL_VARIABLE}:#010X}})");
        }
        ResultValue::I64(_) => {
            let _ = write!(out, "{{{ACTUAL_VARIABLE}}} ({{{ACTUAL_VARIABLE}:#018X}})");
        }
        ResultValue::F32Bits(_) | ResultValue::F32CanonicalNan | ResultValue::F32ArithmeticNan => {
            let _ = write!(out, "{{{ACTUAL_VARIABLE}:e}} ({{:#010X}})");
        }
        ResultValue::F64Bits(_) | ResultValue::F64CanonicalNan | ResultValue::F64ArithmeticNan => {
            let _ = write!(out, "{{{ACTUAL_VARIABLE}:e}} ({{:#018X}})");
        }
    }

    let _ = write!(out, " at {location}\",\n");

    match result {
        ResultValue::I32(_) | ResultValue::I64(_) => (),
        ResultValue::F32Bits(_)
        | ResultValue::F32ArithmeticNan
        | ResultValue::F64Bits(_)
        | ResultValue::F64ArithmeticNan => {
            let _ = write!(out, "      {ACTUAL_VARIABLE}.to_bits(),\n");
        }
        ResultValue::F32CanonicalNan => {
            let _ = write!(out, "      crate::nan::CANONICAL_F32,\n");
            let _ = write!(out, "      crate::nan::NEG_CANONICAL_F32,\n");
            let _ = write!(out, "      {ACTUAL_VARIABLE}.to_bits(),\n");
        }
        ResultValue::F64CanonicalNan => {
            let _ = write!(out, "      crate::nan::CANONICAL_F64,\n");
            let _ = write!(out, "      crate::nan::NEG_CANONICAL_F64,\n");
            let _ = write!(out, "      {ACTUAL_VARIABLE}.to_bits(),\n");
        }
    }

    out.write_str("    );\n");
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
            let statement_location = wast.location(statement.span);
            let _ = writeln!(out, "    // {}", statement_location);

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
                            let _ = writeln!(
                                out,
                                "    assert!(\n      \
                                    matches!({RESULT_VARIABLE}, Err(ref e) if matches!(e.code(), {trap})),\n      \
                                    \"expected trap {trap:?} but got {{:?}} at {statement_location}\",\n      \
                                    {RESULT_VARIABLE}\n    \
                                );",
                            );
                        }
                        Some(ActionResult::Values(values)) => {
                            out.write_str("    ");
                            if values.is_empty() {
                                let _ = writeln!(
                                    out,
                                    "assert_eq!(\n      \
                                        {RESULT_VARIABLE},\n      \
                                        Ok(()),\n      \
                                        \"unexpected trap {{:?}} at {statement_location}\",\n      \
                                        {RESULT_VARIABLE}\n    \
                                    );"
                                );
                            } else {
                                let _ = writeln!(
                                    out,
                                    "assert!(\n      \
                                        {RESULT_VARIABLE}.is_ok(),\n      \
                                        \"unexpected trap {{:?}} at {statement_location}\",\n      \
                                        {RESULT_VARIABLE}\n    \
                                    );"
                                );

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
                                    write_result_check(&mut out, result, &statement_location);
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
