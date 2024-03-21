use std::fmt::Write;

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let suite_dir = {
        let mut manifest_dir =
            std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        manifest_dir.push("testsuite");
        manifest_dir
    };

    let all_file_path = out_dir.join("all.rs");
    let mut all_file = std::fs::File::create(&all_file_path)
        .unwrap_or_else(|e| panic!("could not create file {all_file_path:?}: {e}"));

    const FILES: &[&str] = &["address.wast", "int_exprs.wast", "int_literals.wast"];

    let mut file_buffer = String::with_capacity(0x20000);
    // TODO: See if using rayon can help here
    for wast_name in FILES {
        use std::io::Write as _;

        let wast_path = suite_dir.join(wast_name);

        let wast_text = {
            file_buffer.clear();
            let file = &mut std::fs::File::open(&wast_path)
                .unwrap_or_else(|e| panic!("could not open test file {wast_path:?}: {e}"));

            std::io::Read::read_to_string(file, &mut file_buffer)
                .unwrap_or_else(|e| panic!("could not read test file {wast_path:?}: {e}"));

            file_buffer.as_str()
        };

        let wast_buf = wast::parser::ParseBuffer::new(wast_text)
            .unwrap_or_else(|e| panic!("could not lex test file {wast_path:?}: {e}"));

        let wast = wast::parser::parse::<wast::Wast>(&wast_buf)
            .unwrap_or_else(|e| panic!("could not parse test file {wast_path:?}: {e}"));

        let test_name = wast_path.file_stem().unwrap().to_str().unwrap();

        // Path to directory containing the translated Rust files for this test
        let rs_dir = out_dir.join(test_name);

        match std::fs::create_dir(&rs_dir) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => panic!("could not create directory {rs_dir:?}: {e}"),
        }

        // TODO: See if file buffering helps here.
        let rs_file_path = rs_dir.join("mod.rs");
        let mut rs_file = std::fs::File::create(&rs_file_path)
            .unwrap_or_else(|e| panic!("could not create file {rs_file_path:?}: {e}"));

        #[derive(Clone, Copy)]
        enum SpecValue {
            I32(i32),
            I64(i64),
        }

        impl std::fmt::Display for SpecValue {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::I32(i) => write!(f, "{i}i32"),
                    Self::I64(i) => write!(f, "{i}i64"),
                }
            }
        }

        impl TryFrom<wast::WastArg<'_>> for SpecValue {
            type Error = String;

            fn try_from(arg: wast::WastArg) -> Result<Self, Self::Error> {
                use wast::core::WastArgCore;
                Ok(match arg {
                    wast::WastArg::Core(arg) => match arg {
                        WastArgCore::I32(i) => Self::I32(i),
                        WastArgCore::I64(i) => Self::I64(i),
                        bad => return Err(format!("unsupported argument {bad:?}")),
                    },
                    bad => return Err(format!("component arguments are not supported {bad:?}")),
                })
            }
        }

        impl TryFrom<wast::WastRet<'_>> for SpecValue {
            type Error = String;

            fn try_from(ret: wast::WastRet) -> Result<Self, Self::Error> {
                use wast::core::WastRetCore;
                Ok(match ret {
                    wast::WastRet::Core(ret) => match ret {
                        WastRetCore::I32(i) => Self::I32(i),
                        WastRetCore::I64(i) => Self::I64(i),
                        bad => return Err(format!("unsupported result {bad:?}")),
                    },
                    bad => return Err(format!("component results are not supported {bad:?}")),
                })
            }
        }

        enum SpecTrapReason {
            IntegerDivideByZero,
            IntegerOverflow,
            OutOfBoundsMemoryAccess,
        }

        impl std::fmt::Display for SpecTrapReason {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::IntegerDivideByZero => f.write_str("IntegerDivisionByZero"),
                    Self::IntegerOverflow => f.write_str("IntegerOverflow"),
                    Self::OutOfBoundsMemoryAccess => f.write_str("MemoryBoundsCheck"),
                }
            }
        }

        enum SpecFunctionResults {
            Values(Vec<SpecValue>),
            Trap(SpecTrapReason),
        }

        impl SpecFunctionResults {
            fn trap_with_message(message: &str) -> Option<Self> {
                Some(Self::Trap(match message {
                    "integer divide by zero" => SpecTrapReason::IntegerDivideByZero,
                    "integer overflow" => SpecTrapReason::IntegerOverflow,
                    "out of bounds memory access" => SpecTrapReason::OutOfBoundsMemoryAccess,
                    _ => return None,
                }))
            }
        }

        enum SpecTestKind {
            Function {
                arguments: Vec<SpecValue>,
                results: SpecFunctionResults,
            },
            //Global
        }

        struct SpecTest<'a> {
            export_name: &'a str,
            kind: SpecTestKind,
            assert_span: wast::token::Span,
        }

        struct SpecModule<'a> {
            number: usize,
            id: Option<wast::token::Id<'a>>,
            contents: Vec<u8>,
            tests: Vec<SpecTest<'a>>,
        }

        enum CurrentWat<'a> {
            Parsed {
                number: usize,
                wat: wast::QuoteWat<'a>,
            },
            ParseFailed(wast::Error),
            Encoded(SpecModule<'a>),
        }

        impl<'a> CurrentWat<'a> {
            fn encode(&mut self) -> Result<&mut SpecModule<'a>, &wast::Error> {
                if let Self::Parsed { wat, number } = self {
                    let id = match wat {
                        wast::QuoteWat::Wat(wast::Wat::Module(module)) => module.id,
                        _ => None,
                    };

                    *self = match wat.encode() {
                        Ok(contents) => Self::Encoded(SpecModule {
                            number: *number,
                            id,
                            contents,
                            tests: Vec::new(),
                        }),
                        Err(e) => Self::ParseFailed(e),
                    }
                }

                match self {
                    Self::Encoded(module) => Ok(module),
                    Self::ParseFailed(e) => Err(e),
                    Self::Parsed { .. } => unreachable!(),
                }
            }

            fn get_current(
                wat: &mut Option<Self>,
                span: wast::token::Span,
            ) -> Result<&mut SpecModule<'a>, AssertTranslationError> {
                wat.as_mut()
                    .ok_or_else(|| {
                        AssertTranslationError::with_span(span, "missing module for assertion")
                    })?
                    .encode()
                    .map_err(|e| AssertTranslationError::with_span(span, format!("{e}")))
            }
        }

        let mut spec_module_list = Vec::new();
        let mut module_count = 0usize;
        let mut current_wat = None;

        struct AssertTranslationError {
            span: Option<wast::token::Span>,
            message: std::borrow::Cow<'static, str>,
        }

        impl AssertTranslationError {
            fn with_span<M>(span: wast::token::Span, message: M) -> Self
            where
                M: Into<std::borrow::Cow<'static, str>>,
            {
                Self {
                    span: Some(span),
                    message: message.into(),
                }
            }
        }

        impl From<String> for AssertTranslationError {
            fn from(message: String) -> Self {
                Self {
                    span: None,
                    message: message.into(),
                }
            }
        }

        impl From<&'static str> for AssertTranslationError {
            fn from(message: &'static str) -> Self {
                Self {
                    span: None,
                    message: message.into(),
                }
            }
        }

        let mut write_directive = |directive| {
            use wast::WastDirective;

            match directive {
                WastDirective::Wat(wat) => {
                    if let Some(CurrentWat::Encoded(module)) = current_wat.take() {
                        spec_module_list.push(module);
                    }

                    current_wat = Some(CurrentWat::Parsed {
                        wat,
                        number: module_count,
                    });
                    module_count += 1;
                }
                WastDirective::AssertMalformed { .. } => {
                    if let Some(CurrentWat::Encoded(module)) = current_wat.take() {
                        spec_module_list.push(module);
                    }
                }
                WastDirective::AssertReturn {
                    span,
                    exec,
                    results,
                } => {
                    let wat = CurrentWat::get_current(&mut current_wat, span)?;
                    match exec {
                        wast::WastExecute::Invoke(invoke) => {
                            if invoke.module.is_some() {
                                return Err(AssertTranslationError::with_span(
                                    span,
                                    "assertion with named module is not yet supported",
                                ));
                            } else {
                                wat.tests.push(SpecTest {
                                    export_name: invoke.name,
                                    kind: SpecTestKind::Function {
                                        arguments: invoke
                                            .args
                                            .into_iter()
                                            .map(TryFrom::try_from)
                                            .collect::<Result<_, _>>()?,
                                        results: SpecFunctionResults::Values(
                                            results
                                                .into_iter()
                                                .map(TryFrom::try_from)
                                                .collect::<Result<_, _>>()?,
                                        ),
                                    },
                                    assert_span: span,
                                });
                            }
                        }
                        unknown => {
                            return Err(AssertTranslationError::with_span(
                                span,
                                format!("unsupported assertion {unknown:?}"),
                            ));
                        }
                    }
                }
                WastDirective::AssertTrap {
                    span,
                    exec,
                    message,
                } => {
                    let wat = CurrentWat::get_current(&mut current_wat, span)?;
                    match exec {
                        wast::WastExecute::Invoke(invoke) => {
                            if invoke.module.is_some() {
                                return Err(AssertTranslationError::with_span(
                                    span,
                                    "assertion with named module is not yet supported",
                                ));
                            } else if let Some(results) =
                                SpecFunctionResults::trap_with_message(message)
                            {
                                wat.tests.push(SpecTest {
                                    export_name: invoke.name,
                                    kind: SpecTestKind::Function {
                                        arguments: invoke
                                            .args
                                            .into_iter()
                                            .map(TryFrom::try_from)
                                            .collect::<Result<_, _>>()?,
                                        results,
                                    },
                                    assert_span: span,
                                });
                            } else {
                                return Err(AssertTranslationError::with_span(
                                    span,
                                    format!("unrecognized trap reason: {message:?}"),
                                ));
                            }
                        }
                        unknown => {
                            return Err(AssertTranslationError::with_span(
                                span,
                                format!("unsupported assertion {unknown:?}"),
                            ));
                        }
                    }
                }
                _ => return Err("unsupported directive was skipped".into()),
            }

            Ok(())
        };

        for directive in wast.directives {
            let span = directive.span();
            if let Err(err) = write_directive(directive) {
                let err_span = if let Some(other_span) = err.span {
                    other_span
                } else {
                    span
                };

                let (line, col) = err_span.linecol_in(wast_text);
                println!(
                    "cargo:warning={}:{line}:{col} : {}",
                    wast_path.display(),
                    err.message
                );
            }
        }

        if let Some(CurrentWat::Encoded(last_module)) = current_wat {
            spec_module_list.push(last_module);
        }

        let _ = writeln!(&mut rs_file, "// Generated from {wast_path:?}\n");

        for module in spec_module_list {
            let module_name_buf;
            let module_ident = if let Some(id) = module.id {
                if let Some(valid) = wasm2rs::rust::Ident::new(id.name()) {
                    valid.into()
                } else {
                    wasm2rs::rust::MangledIdent(id.name()).into()
                }
            } else {
                module_name_buf = format!("module_{}", module.number);
                wasm2rs::rust::AnyIdent::from(module_name_buf.as_str())
            };

            let mod_file_path = rs_dir.join(format!("{module_ident}.rs"));
            let mut mod_file = std::fs::File::create(&mod_file_path)
                .unwrap_or_else(|e| panic!("could not create module file {mod_file_path:?}: {e}"));

            let translation_result = wasm2rs::Translation::new()
                .generated_macro_name(module_ident)
                .translate_from_buffer(&module.contents, &mut mod_file);

            if let Err(e) = translation_result {
                panic!("could not translate module {module_ident} from {wast_path:?}: {e}");
            }

            std::mem::drop(mod_file);

            let _ = writeln!(
                &mut rs_file,
                "include!({mod_file_path:?});\n{module_ident}!(pub mod {module_ident});"
            );

            let _ = writeln!(&mut rs_file, "#[test]\nfn assertions_{module_ident}() {{");

            let _ = writeln!(
                &mut rs_file,
                "    let _inst = {module_ident}::Instance::instantiate(Default::default()).unwrap();"
            );

            for (assertion_number, assertion) in module.tests.into_iter().enumerate() {
                struct AssertLocation<'a> {
                    line: usize,
                    column: usize,
                    path: &'a std::path::Path,
                }

                impl std::fmt::Display for AssertLocation<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        writeln!(f, "{}:{}:{}", self.path.display(), self.line, self.column)
                    }
                }

                let assertion_location = {
                    let (line, column) = assertion.assert_span.linecol_in(wast_text);
                    AssertLocation {
                        line,
                        column,
                        path: &wast_path,
                    }
                };

                struct PrintValues<'a>(&'a [SpecValue]);

                impl std::fmt::Display for PrintValues<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        match self.0 {
                            [] if f.alternate() => Ok(()),
                            [] => f.write_str("()"),
                            [val] => write!(f, "{val}"),
                            [first, rest @ ..] => {
                                if !f.alternate() {
                                    f.write_char('(')?;
                                }

                                write!(f, "{first}")?;
                                for result in rest {
                                    write!(f, ", {result}")?;
                                }

                                if !f.alternate() {
                                    f.write_char(')')?;
                                }

                                Ok(())
                            }
                        }
                    }
                }

                let SpecTestKind::Function { arguments, results } = assertion.kind;
                match results {
                    SpecFunctionResults::Values(result_values) => {
                        let _ = writeln!(
                            &mut rs_file,
                            "  assert_eq!(_inst.{}({:#}), Ok({}), \"incorrect values at {assertion_location}\");",
                            wasm2rs::rust::SafeIdent::from(assertion.export_name),
                            PrintValues(&arguments),
                            PrintValues(&result_values)
                        );
                    }
                    SpecFunctionResults::Trap(reason) => {
                        let _ = writeln!(
                            &mut rs_file,
                            "  let result = _inst.{}({:#});",
                            wasm2rs::rust::SafeIdent::from(assertion.export_name),
                            PrintValues(&arguments),
                        );

                        let _ = writeln!(&mut rs_file, "  match result {{");
                        let _ = writeln!(&mut rs_file, "    Ok(values) => panic!(\"expected trap {reason}, but got {{values:?}} at {assertion_location}\"),");
                        let _ = writeln!(&mut rs_file, "    Err(err) => assert!(matches!(err.code(), wasm2rs_rt::trap::TrapCode::{reason} {{ .. }}), \"incorrect trap code, expected {reason} but got {{}} at {assertion_location}\", err.code())");
                        let _ = writeln!(&mut rs_file, "  }}");
                    }
                }
            }

            let _ = writeln!(&mut rs_file, "}}\n");
        }

        let _ = writeln!(
            &mut all_file,
            "mod {} {{\n    include!({rs_file_path:?});\n}}",
            wasm2rs::rust::SafeIdent::from(test_name)
        );
    }
}
