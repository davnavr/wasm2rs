use crate::ident::{AnyIdent, Ident, MangledIdent};

/// The result of lossily converting a string into a valid Rust identifier.
///
/// This type is meant to be used for translating an arbitrary WASM string into a valid Rust
/// identifier.
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct SafeIdent<'a>(pub(in crate::ident) AnyIdent<'a>);

impl<'a> From<AnyIdent<'a>> for SafeIdent<'a> {
    fn from(ident: AnyIdent<'a>) -> Self {
        Self(match ident {
            AnyIdent::Valid(ident)
                if !ident.is_escaped() && ident.name().starts_with(MangledIdent::START) =>
            {
                AnyIdent::Mangled(MangledIdent(ident.name()))
            }
            _ => ident,
        })
    }
}

impl<'a> From<Ident<'a>> for SafeIdent<'a> {
    fn from(ident: Ident<'a>) -> Self {
        Self::from(AnyIdent::from(ident))
    }
}

impl<'a> From<&'a str> for SafeIdent<'a> {
    fn from(name: &'a str) -> Self {
        Self(match Ident::new(name) {
            Some(ident) if !name.starts_with(MangledIdent::START) => AnyIdent::Valid(ident),
            _ => AnyIdent::Mangled(MangledIdent(name)),
        })
    }
}

impl std::fmt::Display for SafeIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Debug for SafeIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
