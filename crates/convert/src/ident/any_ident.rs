use crate::ident::{Ident, MangledIdent};

/// Represents a valid Rust identifier, or a mangled one.
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum AnyIdent<'a> {
    /// A valid Rust identifier.
    Valid(Ident<'a>),
    /// An arbitrary string converted to a valid Rust identifier.
    Mangled(MangledIdent<'a>),
}

impl<'a> From<Ident<'a>> for AnyIdent<'a> {
    fn from(ident: Ident<'a>) -> Self {
        Self::Valid(ident)
    }
}

impl<'a> From<MangledIdent<'a>> for AnyIdent<'a> {
    fn from(ident: MangledIdent<'a>) -> Self {
        Self::Mangled(ident)
    }
}

impl<'a> From<crate::ident::SafeIdent<'a>> for AnyIdent<'a> {
    fn from(ident: crate::ident::SafeIdent<'a>) -> Self {
        ident.0
    }
}

impl<'a> From<&'a str> for AnyIdent<'a> {
    fn from(name: &'a str) -> Self {
        if let Some(ident) = Ident::new(name) {
            Self::Valid(ident)
        } else {
            Self::Mangled(MangledIdent(name))
        }
    }
}

impl std::fmt::Display for AnyIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid(ident) => std::fmt::Display::fmt(&ident, f),
            Self::Mangled(ident) => std::fmt::Display::fmt(&ident, f),
        }
    }
}

impl std::fmt::Debug for AnyIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
