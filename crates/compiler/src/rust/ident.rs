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
                name.len() > 1
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

#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
enum SafeIdentKind<'a> {
    Valid(Ident<'a>),
    Mangled(MangledIdent<'a>),
}

/// The result of lossily converting a string into a valid Rust identifier.
///
/// This type is meant to be used for translating an arbitrary WASM string into a valid Rust
/// identifier.
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct SafeIdent<'a>(SafeIdentKind<'a>);

impl<'a> From<Ident<'a>> for SafeIdent<'a> {
    fn from(ident: Ident<'a>) -> Self {
        Self(
            if !ident.is_escaped() && ident.name().starts_with(MANGLE_START) {
                SafeIdentKind::Mangled(MangledIdent(ident.name()))
            } else {
                SafeIdentKind::Valid(ident)
            },
        )
    }
}

impl<'a> From<&'a str> for SafeIdent<'a> {
    fn from(name: &'a str) -> Self {
        Self(match Ident::new(name) {
            Some(ident) if !name.starts_with(MANGLE_START) => SafeIdentKind::Valid(ident),
            _ => SafeIdentKind::Mangled(MangledIdent(name)),
        })
    }
}

impl std::fmt::Display for SafeIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            SafeIdentKind::Valid(ident) => std::fmt::Display::fmt(&ident, f),
            SafeIdentKind::Mangled(ident) => std::fmt::Display::fmt(&ident, f),
        }
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
