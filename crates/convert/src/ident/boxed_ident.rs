use std::borrow::Cow;

/// Trait implemented for identifiers.
pub trait Identifier<'a>: Copy + std::fmt::Display + 'a {
    /// Converts the identifier into a [`Cow`] string.
    fn to_cow_str(&self) -> Cow<'a, str> {
        self.to_string().into()
    }
}

impl<'a> Identifier<'a> for crate::ident::Ident<'a> {
    fn to_cow_str(&self) -> Cow<'a, str> {
        if self.is_escaped() {
            use std::fmt::Write;

            let mut s = String::with_capacity(Self::ESCAPE.len() + self.name().len());
            let _ = write!(&mut s, "{self}");
            s.into()
        } else {
            Cow::Borrowed(self.name)
        }
    }
}

impl<'a> Identifier<'a> for crate::ident::MangledIdent<'a> {}

impl<'a> Identifier<'a> for crate::ident::AnyIdent<'a> {
    fn to_cow_str(&self) -> Cow<'a, str> {
        match self {
            Self::Valid(valid) => valid.to_cow_str(),
            Self::Mangled(mangled) => mangled.to_cow_str(),
        }
    }
}

impl<'a> Identifier<'a> for crate::ident::SafeIdent<'a> {
    fn to_cow_str(&self) -> Cow<'a, str> {
        self.0.to_cow_str()
    }
}

/// A heap-allocated [`Ident`].
///
/// [`Ident`]: crate::ident::Ident
#[derive(Clone, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BoxedIdent<'a>(std::borrow::Cow<'a, str>); // could use beef::lean::Cow

impl<'a> BoxedIdent<'a> {
    /// Creates a heap allocation to store the given identifier.
    pub fn new<I: Identifier<'a>>(ident: &I) -> Self {
        Self(ident.to_cow_str())
    }

    /// Creates a [`BoxedIdent`] from a string without checking that it is actually a valid Rust
    /// identifier.
    pub(crate) fn from_str_unchecked<S>(ident: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self(ident.into())
    }

    /// Converts an arbitrary string into a [`SafeIdent`], then allocates it on the heap.
    ///
    /// [`SafeIdent`]: crate::ident::SafeIdent
    pub fn from_str_safe(s: &'a str) -> Self {
        Self::new(&crate::ident::SafeIdent::from(s))
    }

    /// Returns a string slice over the valid Rust identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Gets an [`Ident`] from this [`BoxedIdent`].
    ///
    /// [`Ident`]: crate::ident::Ident
    pub fn to_ident(&self) -> crate::ident::Ident<'_> {
        if self.as_str().starts_with(crate::ident::Ident::ESCAPE) {
            crate::ident::Ident {
                name: &self.as_str()[2..],
                escaped: true,
            }
        } else {
            crate::ident::Ident {
                name: &self.as_str(),
                escaped: false,
            }
        }
    }
}

impl AsRef<str> for BoxedIdent<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Debug for BoxedIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for BoxedIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}
