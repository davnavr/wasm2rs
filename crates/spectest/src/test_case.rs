//! Types and functions for generating Rust unit test functions from `.wast` directives.

mod builder;

pub use builder::Builder;

enum StatementKind {
    InvokeFunction {
        //arguments: Vec<>,
    },
}

struct Statement {
    kind: StatementKind,
    span: wast::token::Span,
}

pub struct Module<'wasm> {
    number: usize,
    id: Option<&'wasm str>,
    span: wast::token::Span,
    definition: wast::QuoteWat<'wasm>,
    statements: Vec<Statement>,
}
