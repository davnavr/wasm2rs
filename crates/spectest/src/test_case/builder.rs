use crate::test_case::{ActionResult, Module, ResultValue, StatementKind};
use anyhow::Context;

type ModuleLookup<'wasm> = std::collections::HashMap<&'wasm str, usize>;

pub struct Builder<'a, 'wasm> {
    file_contents: &'a crate::location::Contents<'wasm>,
    modules: Vec<Module<'wasm>>,
    module_contents: Vec<wast::QuoteWat<'wasm>>,
    module_lookup: ModuleLookup<'wasm>,
}

/// Gets the module associated with an assertion or command.
fn get_module<'a, 'wasm>(
    id: &Option<wast::token::Id<'wasm>>,
    modules: &'a mut [Module<'wasm>],
    lookup: &ModuleLookup<'wasm>,
    span: wast::token::Span,
    file_contents: &crate::location::Contents<'wasm>,
) -> crate::Result<&'a mut Module<'wasm>> {
    if let Some(name) = id {
        if let Some(index) = lookup.get(name.name()) {
            Ok(&mut modules[*index])
        } else {
            Err(anyhow::anyhow!(
                "{} : no module with the name ${} exists",
                name.name(),
                file_contents.location(name.span())
            ))
        }
    } else {
        modules.last_mut().ok_or_else(|| {
            anyhow::anyhow!(
                "{} : no module exists at this point",
                file_contents.location(span)
            )
        })
    }
}

impl<'a, 'wasm> Builder<'a, 'wasm> {
    pub fn new(file_contents: &'a crate::location::Contents<'wasm>) -> Self {
        Self {
            file_contents,
            modules: Vec::new(),
            module_contents: Vec::new(),
            module_lookup: Default::default(),
        }
    }

    /// Translates top-level `(module)` commands into a Rust function call to instantiate a
    /// `wasm2rs` translated WebAssembly module.
    pub fn module(&mut self, wat: wast::QuoteWat<'wasm>) -> crate::Result<()> {
        use wast::{QuoteWat, Wat};

        // An optional name the module is referred to in some assertions (ex: `$my_module`).
        let (span, id) = match &wat {
            QuoteWat::Wat(
                Wat::Module(wast::core::Module { span, id, .. })
                | Wat::Component(wast::component::Component { span, id, .. }),
            ) => (*span, id.as_ref()),
            QuoteWat::QuoteModule(span, _) | QuoteWat::QuoteComponent(span, _) => (*span, None),
        };

        let number = self.modules.len();

        if let Some(name) = id {
            if self.module_lookup.insert(name.name(), number).is_some() {
                anyhow::bail!(
                    "{} : module definition with name {:?} was already defined",
                    self.file_contents.location(name.span()),
                    name.name(),
                )
            }
        }

        self.modules.push(Module {
            number,
            id: id.map(wast::token::Id::name),
            span,
            statements: Vec::new(),
        });

        self.module_contents.push(wat);

        Ok(())
    }

    fn invoke_with_results(
        &mut self,
        invoke: wast::WastInvoke<'wasm>,
        result: Option<ActionResult>,
    ) -> crate::Result<()> {
        let module = get_module(
            &invoke.module,
            &mut self.modules,
            &self.module_lookup,
            invoke.span,
            &self.file_contents,
        )
        .with_context(|| {
            format!(
                "could not find module for invoke in {}",
                self.file_contents.location(invoke.span)
            )
        })?;

        module.statements.push(crate::test_case::Statement {
            kind: StatementKind::InvokeFunction {
                name: invoke.name,
                arguments: crate::test_case::Arguments::try_from(invoke.args).with_context(
                    || {
                        format!(
                            "could not convert arguments in {}",
                            self.file_contents.location(invoke.span)
                        )
                    },
                )?,
                result,
            },
            span: invoke.span,
        });

        Ok(())
    }

    /// Translates top-level `(invoke)` actions into a Rust function call.
    pub fn invoke(&mut self, invoke: wast::WastInvoke<'wasm>) -> crate::Result<()> {
        self.invoke_with_results(invoke, None)
    }

    /// Translates an `(assert_return)` assertion that calls a function and expects certain result
    /// values.
    pub fn assert_return_invoke(
        &mut self,
        invoke: wast::WastInvoke<'wasm>,
        results: Vec<wast::WastRet<'wasm>>,
    ) -> crate::Result<()> {
        self.invoke_with_results(
            invoke,
            Some(ActionResult::Values(ResultValue::try_convert_vec(results)?)),
        )
    }

    /// Translatesan `(assert_return)` assertion that calls a function and expects a trap.
    pub fn assert_trap_invoke(
        &mut self,
        invoke: wast::WastInvoke<'wasm>,
        message: &'wasm str,
    ) -> crate::Result<()> {
        self.invoke_with_results(invoke, Some(ActionResult::Trap(message.parse()?)))
    }

    pub fn finish(self) -> (Vec<Module<'wasm>>, Vec<wast::QuoteWat<'wasm>>) {
        debug_assert_eq!(self.modules.len(), self.module_contents.len());
        (self.modules, self.module_contents)
    }
}
