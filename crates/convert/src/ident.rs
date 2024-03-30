//! Contains types representing Rust identifiers.

mod any_ident;
mod mangled_ident;
mod safe_ident;

pub use any_ident::AnyIdent;
pub use mangled_ident::MangledIdent;
pub use safe_ident::SafeIdent;

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
    /// Identifier for the default name for the generated Rust macro.
    pub const DEFAULT_MACRO_NAME: Self = Self::unescaped("wasm");
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
