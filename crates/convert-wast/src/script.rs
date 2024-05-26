/*
/// Prints the Rust source code corresponding to a [`Script`](crate::script::Script).
mod print;
/// Parses and validates a [`.wast`](wast::Wast) file.
mod validate;

/// Keeps track of the things exported from a WebAssembly module.
///
/// Remember that the string keys originate from [`wasm2rs_convert::ident`], and must *not* be
/// compared against a [`wast::token::Id`].
struct ModuleExports {
    /// A Rust identifier referring to the module instance.
    rust_variable: Box<str>,
    function_exports: std::collections::HashMap<Box<str>, wasmparser::FuncType>,
}

struct Export<'wat> {
    name: wasm2rs_convert::ident::BoxedIdent<'wat>,
    module: Option<wast::token::Id<'wat>>,
}

enum StatementKind<'wat> {
    /// Invokes the Rust method corresponding to a WebAssembly function.
    InvokeFunction {
        callee: Export<'wat>,
        arguments: Arguments,
        /// If `Some`, indicates that the result is checked with an assertion.
        /// If `None`, then the function is called, but the return value is ignored.
        result: Option<ActionResult>,
    },
}

struct Statement<'wat> {
    kind: StatementKind<'wat>,
    /// Refers to the location in the original `.wast` file that this [`Statement`] was
    /// generated from.
    span: wast::token::Span,
}

struct Script<'wat> {
    //module_lookup: std::collections::HashMap<wast::token::Id<'wat>, usize>,
    statements: Vec<Statement>,
}
*/

use anyhow::Context;

trait ResultExt<T> {
    fn with_file(self, path: &std::path::Path, text: &str) -> anyhow::Result<T>;
}

impl<T> ResultExt<T> for Result<T, wast::Error> {
    fn with_file(self, path: &std::path::Path, text: &str) -> anyhow::Result<T> {
        self.map_err(|mut err| {
            err.set_path(path);
            err.set_text(text);
            anyhow::Error::new(err)
        })
    }
}

/// Converts the `.wast` script into Rust source code.
///
/// Currently, this is implemented as a very naive source-to-source conversion with basically no
/// checking. If the input `.wast` is invalid, then attempting to compile the output Rust code
/// should result in a compiler error.
pub(crate) fn convert(
    write: &mut (dyn std::io::Write + '_),
    output: &(dyn crate::Output<'_> + '_),
    script_path: &std::path::Path,
    script_text: &str,
    conversion_options: &wasm2rs_convert::Convert,
    conversion_allocations: &wasm2rs_convert::Allocations,
) -> Result<(), anyhow::Error> {
    use wasm2rs_convert::write::Write as _;

    let script_buffer = wast::parser::ParseBuffer::new(script_text)
        .with_file(script_path, script_text)
        .context("could not lex input")?;

    let script: wast::Wast = wast::parser::parse(&script_buffer)
        .with_file(script_path, script_text)
        .context("could not parse input")?;

    let mut module_count = 0u32;

    let mut out = wasm2rs_convert::write::IoWrite::new(write);
    writeln!(out, "// Generated from {script_path:?}\n");
    writeln!(out, "#[test]\nfn execute() {{");

    for directive in script.directives.into_iter() {
        use wast::WastDirective;

        match directive {
            WastDirective::Wat(mut wat) => {
                let wat_span = wat.span();

                let module_id = match &wat {
                    wast::QuoteWat::Wat(wast::Wat::Module(wast::core::Module { id, .. })) => *id,
                    _ => None,
                };

                // A top-level `<module>` is validated and instantiated, see
                // https://github.com/WebAssembly/spec/blob/wg-2.0.draft1/interpreter/README.md#scripts
                let wasm = wat
                    .encode()
                    .with_file(script_path, script_text)
                    .with_context(|| {
                        let mut err =
                            wast::Error::new(wat_span, "expected valid WebAssembly module".into());
                        err.set_text(script_text);
                        err
                    })?;

                let path_to_include = output
                    .create_module_file(
                        script_path,
                        module_count,
                        module_id.as_ref(),
                        wasm,
                        conversion_options,
                        conversion_allocations,
                    )
                    .with_context(|| {
                        let mut err =
                            wast::Error::new(wat_span, "could not obtain path to module".into());
                        err.set_text(script_text);
                        err
                    })?;

                module_count += 1;

                writeln!(out, "    let current = {{");
                out.write_str("        mod module {\n");
                writeln!(
                    out,
                    "            ::core::include!({:?});",
                    path_to_include.as_ref()
                );

                // TODO: Generate module imports.

                writeln!(
                    out,
                    "            wasm!(mod module use ::wasm2rs_rt::embedder::self_contained);"
                );
                out.write_str("            pub use module::*;\n        }\n");

                let (line, col) = wat_span.linecol_in(script_text);
                write!(
                    out,
                    "        module::Instance::instantiate(Default::default()).expect(\"successful module instantiation for {}:{}:{}\")",
                    script_path.display(),
                    line.saturating_add(1),
                    col.saturating_add(1),
                );

                out.write_str("\n    };\n");
            }
            WastDirective::AssertMalformed { span, .. } => {
                // The maintainers of `wasmparser` already run specification tests.
                //module.to_test

                let (line, col) = span.linecol_in(script_text);
                writeln!(
                    out,
                    "\n    // Skipped `assert_malformed` in {}:{}:{}",
                    script_path.display(),
                    line.saturating_add(1),
                    col.saturating_add(1),
                );
            }
            WastDirective::AssertInvalid { span, .. } => {
                let (line, col) = span.linecol_in(script_text);
                writeln!(
                    out,
                    "\n    // Skipped `assert_invalid` in {}:{}:{}",
                    script_path.display(),
                    line.saturating_add(1),
                    col.saturating_add(1),
                );
            }
            // WastDirective::Register
            WastDirective::Invoke(wast::WastInvoke {
                span,
                module: None,
                name,
                args,
            }) => {
                write!(
                    out,
                    "\n    println!(\"{{:?}}\", current.{}(",
                    wasm2rs_convert::ident::SafeIdent::from(name)
                );

                // Duplicated code.
                for (i, arg) in args.into_iter().enumerate() {
                    use wast::core::WastArgCore;

                    if i > 0 {
                        out.write_str(", ");
                    }

                    match arg {
                        wast::WastArg::Core(core_arg) => match core_arg {
                            WastArgCore::I32(n) => write!(out, "{n}i32"),
                            WastArgCore::I64(n) => write!(out, "{n}i64"),
                            WastArgCore::F32(z) => {
                                write!(out, "f32::from_bits({:#010X}u32)", z.bits)
                            }
                            WastArgCore::F64(z) => {
                                write!(out, "f64::from_bits({:#018X}u64)", z.bits)
                            }
                            WastArgCore::RefExtern(_)
                            | WastArgCore::RefHost(_)
                            | WastArgCore::RefNull(_) => out.write_str(
                                "::core::todo!(\"reference type arguments not yet supported\")",
                            ),
                            WastArgCore::V128(_) => {
                                out.write_str("todo!(\"V128 arguments not yet supported\")")
                            }
                        },
                        wast::WastArg::Component(arg) => {
                            let mut err = wast::Error::new(
                                span,
                                format!("compontent arguments are not supported: {arg:?}"),
                            );
                            err.set_text(script_text);
                            return Err(anyhow::Error::new(err));
                        }
                    }
                }

                out.write_str("));\n");
            }
            // // Quick and dirty code gen, better way is in old code, see /crates/spectest/test_case.rs
            // // This should check the statements to see if the exports and modules they refer to actually exist.
            WastDirective::AssertTrap {
                span: assert_span,
                exec:
                    wast::WastExecute::Invoke(wast::WastInvoke {
                        span: invoke_span,
                        module: None,
                        name,
                        args,
                    }),
                message,
            } => {
                write!(
                    out,
                    "\n    let result = current.{}(",
                    wasm2rs_convert::ident::SafeIdent::from(name)
                );

                for (i, arg) in args.into_iter().enumerate() {
                    use wast::core::WastArgCore;

                    if i > 0 {
                        out.write_str(", ");
                    }

                    match arg {
                        wast::WastArg::Core(core_arg) => match core_arg {
                            WastArgCore::I32(n) => write!(out, "{n}i32"),
                            WastArgCore::I64(n) => write!(out, "{n}i64"),
                            WastArgCore::F32(z) => {
                                write!(out, "f32::from_bits({:#010X}u32)", z.bits)
                            }
                            WastArgCore::F64(z) => {
                                write!(out, "f64::from_bits({:#018X}u64)", z.bits)
                            }
                            WastArgCore::RefExtern(_)
                            | WastArgCore::RefHost(_)
                            | WastArgCore::RefNull(_) => out.write_str(
                                "::core::todo!(\"reference type arguments not yet supported\")",
                            ),
                            WastArgCore::V128(_) => {
                                out.write_str("todo!(\"V128 arguments not yet supported\")")
                            }
                        },
                        wast::WastArg::Component(arg) => {
                            let mut err = wast::Error::new(
                                invoke_span,
                                format!("compontent arguments are not supported: {arg:?}"),
                            );
                            err.set_text(script_text);
                            return Err(anyhow::Error::new(err));
                        }
                    }
                }

                let (line, col) = assert_span.linecol_in(script_text);
                writeln!(
                    out,
                    ").expect_err(\"expected trap in {}:{}:{}\");",
                    script_path.display(),
                    line.saturating_add(1),
                    col.saturating_add(1)
                );

                write!(
                    out,
                    "    assert!(result.matches_spec_failure(\"{}\"), \"incorrect trap in {}:{}:{}",
                    message.escape_default(),
                    script_path.display(),
                    line.saturating_add(1),
                    col.saturating_add(1)
                );
                out.write_str(", got {result:?}\");\n");
            }
            WastDirective::AssertReturn {
                span: assert_span,
                exec:
                    wast::WastExecute::Invoke(wast::WastInvoke {
                        span: invoke_span,
                        module: None,
                        name,
                        args,
                    }),
                results,
            } => {
                write!(
                    out,
                    "\n    let result = current.{}(",
                    wasm2rs_convert::ident::SafeIdent::from(name)
                );

                for (i, arg) in args.into_iter().enumerate() {
                    use wast::core::WastArgCore;

                    if i > 0 {
                        out.write_str(", ");
                    }

                    match arg {
                        wast::WastArg::Core(core_arg) => match core_arg {
                            WastArgCore::I32(n) => write!(out, "{n}i32"),
                            WastArgCore::I64(n) => write!(out, "{n}i64"),
                            WastArgCore::F32(z) => {
                                write!(out, "f32::from_bits({:#010X}u32)", z.bits)
                            }
                            WastArgCore::F64(z) => {
                                write!(out, "f64::from_bits({:#018X}u64)", z.bits)
                            }
                            WastArgCore::RefExtern(_)
                            | WastArgCore::RefHost(_)
                            | WastArgCore::RefNull(_) => out.write_str(
                                "::core::todo!(\"reference type arguments not yet supported\")",
                            ),
                            WastArgCore::V128(_) => {
                                out.write_str("todo!(\"V128 arguments not yet supported\")")
                            }
                        },
                        wast::WastArg::Component(arg) => {
                            let mut err = wast::Error::new(
                                invoke_span,
                                format!("compontent arguments are not supported: {arg:?}"),
                            );
                            err.set_text(script_text);
                            return Err(anyhow::Error::new(err));
                        }
                    }
                }

                let (line, col) = assert_span.linecol_in(script_text);
                writeln!(
                    out,
                    ").expect(\"unexpected trap in {}:{}:{}\");",
                    script_path.display(),
                    line.saturating_add(1),
                    col.saturating_add(1)
                );

                let many_results = results.len() > 1;
                for (index, result) in results.into_iter().enumerate() {
                    use wast::core::{NanPattern, WastRetCore};

                    struct ResultVariable(Option<usize>);

                    impl std::fmt::Display for ResultVariable {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            f.write_str("result")?;
                            if let Some(index) = self.0 {
                                write!(f, ".{index}")?;
                            }
                            Ok(())
                        }
                    }

                    let actual = ResultVariable(if many_results { Some(index) } else { None });

                    out.write_str("    assert!(");

                    let result = match result {
                        wast::WastRet::Core(core_ret) => {
                            match core_ret {
                                WastRetCore::I32(expected) => {
                                    write!(out, "{actual} == {expected}i32")
                                }
                                WastRetCore::I64(expected) => {
                                    write!(out, "{actual} == {expected}i64")
                                }
                                WastRetCore::F32(NanPattern::Value(expected)) => {
                                    write!(out, "f32::to_bits({actual}) == {:#010X}", expected.bits)
                                }
                                WastRetCore::F64(NanPattern::Value(expected)) => {
                                    write!(out, "f64::to_bits({actual}) == {:#010X}", expected.bits)
                                }
                                WastRetCore::F32(NanPattern::CanonicalNan) => write!(
                                    out,
                                    "::wasm2rs_rt::math::nan::is_canonical_f32({actual})"
                                ),
                                WastRetCore::F64(NanPattern::CanonicalNan) => write!(
                                    out,
                                    "::wasm2rs_rt::math::nan::is_canonical_f64({actual})"
                                ),
                                WastRetCore::F32(NanPattern::ArithmeticNan) => write!(
                                    out,
                                    "::wasm2rs_rt::math::nan::is_arithmetic_f32({actual})"
                                ),
                                WastRetCore::F64(NanPattern::ArithmeticNan) => write!(
                                    out,
                                    "::wasm2rs_rt::math::nan::is_arithmetic_f64({actual})"
                                ),
                                _ => {
                                    let mut err = wast::Error::new(
                                        assert_span,
                                        format!("unsupported result: {core_ret:?}"),
                                    );
                                    err.set_text(script_text);
                                    return Err(anyhow::Error::new(err));
                                }
                            }

                            core_ret
                        }
                        wast::WastRet::Component(ret) => {
                            let mut err = wast::Error::new(
                                assert_span,
                                format!("compontent results are not supported: {ret:?}"),
                            );
                            err.set_text(script_text);
                            return Err(anyhow::Error::new(err));
                        }
                    };

                    write!(
                        out,
                        ", \"invalid result in {}:{}:{}",
                        script_path.display(),
                        line.saturating_add(1),
                        col.saturating_add(1)
                    );

                    match result {
                        WastRetCore::I32(expected) => {
                            write!(out, ", expected {expected} ({expected:#010X}) got ");
                            out.write_str("{actual} ({actual:#010X})\", actual=");
                            write!(out, "{actual}");
                        }
                        WastRetCore::I64(expected) => {
                            write!(out, ", expected {expected} ({expected:#018X})");
                            out.write_str(" got {actual} ({actual:#018X})\", actual=");
                            write!(out, "{actual}");
                        }
                        WastRetCore::F32(NanPattern::Value(expected)) => {
                            write!(
                                out,
                                "expected {} ({:#010X}) got ",
                                f32::from_bits(expected.bits),
                                expected.bits
                            );
                            out.write_str("{} ({:#010X})\", ");
                            write!(out, "{actual}, f32::to_bits({actual})");
                        }
                        WastRetCore::F64(NanPattern::Value(expected)) => {
                            write!(
                                out,
                                "expected {} ({:#018X}) got ",
                                f64::from_bits(expected.bits),
                                expected.bits
                            );
                            out.write_str("{} ({:#018X})\", ");
                            write!(out, "{actual}, f64::to_bits({actual})");
                        }
                        WastRetCore::F32(NanPattern::CanonicalNan) => {
                            out.write_str(
                                concat!(
                                    "expected canonical NaN ({:#010X} or {:#010X}) got {} ({:#010X})\", ",
                                    "::wasm2rs_rt::math::nan::F32_CANONICAL, ",
                                    "::wasm2rs_rt::math::nan::F32_NEG_CANONICAL, ",
                                )
                            );
                            write!(out, "{actual}, f32::to_bits({actual})");
                        }
                        WastRetCore::F64(NanPattern::CanonicalNan) => {
                            out.write_str(
                                concat!(
                                    "expected canonical NaN ({:#018X} or {:#018X}) got {} ({:#018X})\", ",
                                    "::wasm2rs_rt::math::nan::F64_CANONICAL, ",
                                    "::wasm2rs_rt::math::nan::F64_NEG_CANONICAL, ",
                                )
                            );
                            write!(out, "{actual}, f64::to_bits({actual})");
                        }
                        WastRetCore::F32(NanPattern::ArithmeticNan) => {
                            out.write_str("expected arithmetic NaN got {} ({:#010X})\", ");
                            write!(out, "{actual}, f32::to_bits({actual})");
                        }
                        WastRetCore::F64(NanPattern::ArithmeticNan) => {
                            out.write_str("expected arithmetic NaN got {} ({:#018X})\", ");
                            write!(out, "{actual}, f64::to_bits({actual})");
                        }
                        _ => out.write_str("\""),
                    }

                    writeln!(out, ");");
                }
            }
            WastDirective::AssertExhaustion {
                span: assert_span,
                call: _,
                message: _,
            } => {
                out.write_str("\n    #[warn(unused_variables)]\n");
                let (line, col) = assert_span.linecol_in(script_text);
                write!(
                    out,
                    "    let skipped_assertion = \
                    eprintln!(\"skipped assertion in {}:{}:{}: stack overflow checking is not yet supported\");",
                    script_path.display(),
                    line.saturating_add(1),
                    col.saturating_add(1)
                );
            }
            unsupported => {
                let mut err = wast::Error::new(
                    unsupported.span(),
                    format!("encountered unsupported directive {unsupported:?}"),
                );
                err.set_text(script_text);
                return Err(anyhow::Error::new(err));
            }
        }
    }

    writeln!(out, "}}");

    out.flush();
    out.into_inner()
        .context("I/O error occurred while writing output")?;

    Ok(())
}
