use crate::Error;

/// Keeps track of the things exported from a WebAssembly module.
///
/// Remember that the string keys originate from [`wasm2rs_convert::ident`], and must *not* be
/// compared against a [`wast::token::Id`].
struct ModuleExports {
    function_exports: std::collections::HashMap<Box<str>, wasmparser::FuncType>,
}

struct Contents<'wat> {
    module_names: Vec<wasm2rs_convert::ident::BoxedIdent<'wat>>,
    /// Maps [`wast::token::Id`]s to an index into [`Test::module_names`].
    module_lookup: std::collections::HashMap<&'wat str, usize>,
    //intentionally_malformed_modules
}

trait ResultExt<'a, T> {
    fn with_file<M>(
        self,
        path: &'a std::path::Path,
        text: &str,
        message: impl FnOnce() -> M,
    ) -> Result<T, Error<'a>>
    where
        M: Into<std::borrow::Cow<'a, str>>;
}

impl<'a, T> ResultExt<'a, T> for Result<T, wast::Error> {
    fn with_file<M>(
        self,
        path: &'a std::path::Path,
        text: &str,
        message: impl FnOnce() -> M,
    ) -> Result<T, Error<'a>>
    where
        M: Into<std::borrow::Cow<'a, str>>,
    {
        self.map_err(|mut cause| {
            cause.set_path(path);
            cause.set_text(text);
            let span = cause.span();
            Error::with_position_into_text(path, message(), Some(cause.into()), span, text)
        })
    }
}

pub(crate) fn convert<'path>(
    output: &mut (dyn std::io::Write + '_),
    config: &wasm2rs_convert::Convert<'_>,
    allocations: &wasm2rs_convert::Allocations,
    script_path: &'path std::path::Path,
    script_text: &str,
) -> Result<(), crate::Error<'path>> {
    use wasm2rs_convert::write::Write as _;

    let script_buffer =
        wast::parser::ParseBuffer::new(script_text)
            .with_file(script_path, script_text, || "could not lex input")?;

    let script: wast::Wast =
        wast::parser::parse(&script_buffer)
            .with_file(script_path, script_text, || "could not parse input")?;

    let current_module: Option<ModuleExports> = None;
    let mut string_buf = String::new();
    let mut running_errors = Vec::new();

    let mut out = wasm2rs_convert::write::IoWrite::new(output);
    writeln!(out, "// Generated from {script_path:?}\n");
    writeln!(out, "#[test]\nfn execute() {{\n");

    for directive in script.directives.into_iter() {
        use wast::WastDirective;

        macro_rules! commit_result {
            ($result:expr) => {
                match $result {
                    Ok(value) => value,
                    Err(error) => {
                        running_errors.push(error);

                        // This parser is dumb, but if `continue` was used then an error that causes
                        // other errors would make the source hard to determine.
                        break;
                    }
                }
            };
        }

        match directive {
            WastDirective::Wat(mut wat) => {
                // A top-level `<module>` is validated and instantiated, see
                // https://github.com/WebAssembly/spec/blob/wg-2.0.draft1/interpreter/README.md#scripts
                let wasm = commit_result!(wat.encode().with_file(script_path, script_text, || {
                    "expected valid WebAssembly module"
                }));

                writeln!(out, "let module = {{");

                let mut has_imports = false;

                commit_result!(config
                    .convert_from_buffer_with_intermediate(&wasm, allocations, |context| {
                        has_imports = context.has_imports();
                        out.try_borrow_mut().map_err(Into::into)
                    })
                    .map_err(|cause| {
                        Error::with_path_and_cause(
                            script_path,
                            "could not convert WebAssembly module",
                            cause,
                        )
                    }));

                // TODO: Need to disallow specifying macro name
                writeln!(
                    out,
                    "\nwasm!(pub mod wasm use ::wasm2rs_rt::embedder::self_contained);\n"
                );
                writeln!(
                    out,
                    "wasm::Instance::instantiate(Default::default()).unwrap()\n\n}}\n"
                );

                if has_imports {
                    running_errors.push(Error::with_path(
                        &script_path,
                        "module imports are not yet supported",
                        None,
                    ));

                    // running_errors.push(Error::with_position_into_text(
                    //     script_path,
                    //     "module imports are not yet supported",
                    //     None,
                    //     wat.span(),
                    //     script_text,
                    // ));
                }
            }
            WastDirective::AssertMalformed {
                span: _,
                module: _,
                message: _,
            } => {
                // The maintainers of `wasmparser` already run specification tests.
                //module.to_test
            }
            // WastDirective::AssertInvalid
            // WastDirective::Register
            // WastDirective::Invoke
            // WastDirective::AssertTrap
            // WastDirective::AssertReturn
            unsupported => {
                running_errors.push(Error::with_position_into_text(
                    script_path,
                    format!("unknown directive {unsupported:?}"),
                    None,
                    unsupported.span(),
                    script_text,
                ));
            }
        }
    }

    writeln!(out, "\n}}");

    out.flush();
    if let Err(io_error) = out.into_inner() {
        running_errors.push(Error::with_path_and_cause(
            script_path,
            "I/O error occurred while writing",
            io_error,
        ));
    }

    Error::collect(running_errors)
}
