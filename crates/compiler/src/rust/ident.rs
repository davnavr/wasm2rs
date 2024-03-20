use std::fmt::Write;

const MANGLE_START: &str = "_WR_";

/// A Rust [identifier].
///
/// [identifier]: https://doc.rust-lang.org/reference/identifiers.html
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Ident<'a> {
    name: &'a str,
    escaped: bool,
}

impl<'a> Ident<'a> {
    /// Creates a new identifier.
    ///
    /// # Errors
    ///
    /// Returns `None` if the `name` is not a valid identifier, or is a keyword.
    pub fn new(name: &'a str) -> Option<Self> {
        if matches!(name, "" | "_") {
            return None;
        }

        if !name.is_ascii() {
            // TODO: Handling of exotic identifiers is not yet implemented.
            None
        } else {
            let mut chars = name.chars();
            let start = chars.next().unwrap(); // Won't panic, empty case handled above.
            let escaped = if start.is_ascii_uppercase() || start == '_' {
                false
            } else if start.is_ascii_lowercase() {
                // Assumes any identifier that consists only of ASCII lowercase is a keyword.
                // If it is just a single character, then it definitely isn't a keyword though.
                name.len() > 1 && !name.contains(|c: char| c.is_ascii_digit())
            } else {
                // Invalid start of identifier
                return None;
            };

            if chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
                Some(Self { name, escaped })
            } else {
                None
            }
        }
    }

    /// Returns `true` if the identifier is prefixed with `r#` to avoid conflicts with keywords in
    /// future Rust versions.
    pub fn is_escaped(&self) -> bool {
        self.escaped
    }

    /// Gets the identifier.
    pub fn name(&self) -> &'a str {
        self.name
    }

    const fn unescaped(name: &'a str) -> Self {
        Self {
            name,
            escaped: false,
        }
    }
}

impl Ident<'static> {
    /// Identifier for the [`i32`] primitive type.
    pub const PRIM_I32: Self = Self::unescaped("i32");

    /// Identifier for the [`i64`] primitive type.
    pub const PRIM_I64: Self = Self::unescaped("i64");

    /// Identifier for the `wasm2rs` runtime support crate.
    pub const NAME_RT_CRATE: Self = Self::unescaped("wasm2rs_rt");

    /// Identifier for the default name for the generated Rust module.
    pub const DEFAULT_MODULE_NAME: Self = Self::unescaped("wasm");
}

impl std::fmt::Display for Ident<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.escaped {
            f.write_str("r#")?;
        }

        f.write_str(self.name)
    }
}

impl std::fmt::Debug for Ident<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

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

impl<'a> From<SafeIdent<'a>> for AnyIdent<'a> {
    fn from(ident: SafeIdent<'a>) -> Self {
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

/// The result of lossily converting a string into a valid Rust identifier.
///
/// This type is meant to be used for translating an arbitrary WASM string into a valid Rust
/// identifier.
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct SafeIdent<'a>(AnyIdent<'a>);

impl<'a> From<AnyIdent<'a>> for SafeIdent<'a> {
    fn from(ident: AnyIdent<'a>) -> Self {
        Self(match ident {
            AnyIdent::Valid(ident)
                if !ident.is_escaped() && ident.name().starts_with(MANGLE_START) =>
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
            Some(ident) if !name.starts_with(MANGLE_START) => AnyIdent::Valid(ident),
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

/// A valid Rust identifier constructed from an arbitrary string.
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct MangledIdent<'a>(pub &'a str);

impl std::fmt::Display for MangledIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(MANGLE_START)?;

        for c in self.0.chars() {
            match c {
                _ if c.is_ascii_alphanumeric() => f.write_char(c)?,
                '_' => f.write_str("__")?,
                '.' => f.write_str("_o")?,
                '-' => f.write_str("_L")?,
                _ => {
                    let n = c as u32;
                    let width = if n > 0xFFFF {
                        6
                    } else if n > 0xFF {
                        4
                    } else {
                        2
                    };

                    write!(f, "_x{n:0width$X}")?
                }
            }
        }

        Ok(())
    }
}

impl std::fmt::Debug for MangledIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}
