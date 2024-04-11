//! The entrypoint for converting a WebAssembly byte code into Rust source code.

pub(in crate::convert) struct Code<'a> {
    body: wasmparser::FunctionBody<'a>,
    validator: wasmparser::FuncToValidate<wasmparser::ValidatorResources>,
}

impl<'a> Code<'a> {
    pub(in crate::convert) fn new(validator: &mut wasmparser::Validator, body: wasmparser::FunctionBody<'a>) -> crate::Result<Self> {
        Ok(Self {
            validator: validator.code_section_entry(&body)?,
            body
        })
    }
}
