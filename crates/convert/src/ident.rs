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
    /// Creates a new identifier. If the `name` is a [keyword], then it is [escaped].
    ///
    /// # Errors
    ///
    /// Returns [`None`] if the `name` is not a valid [Rust identifier].
    ///
    /// [keyword]: https://doc.rust-lang.org/reference/keywords.html
    /// [escaped]: Ident::is_escaped()
    /// [Rust identifier]: https://doc.rust-lang.org/reference/identifiers.html
    pub fn new(name: &'a str) -> Option<Self> {
        use unicode_xid::UnicodeXID;

        match name {
            "" | "_" => None,
            // Strict keywords
            "as" | "break" | "const" | "continue" | "crate" | "else"
            | "enum" | "extern" | "false" | "fn" | "for" | "if" | "impl"
            | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub"
            | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
            | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while"
            | "async" | "await" | "dyn"
            // Reserved keywords
            | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override"
            | "priv" | "typeof" | "unsized" | "virtual" | "yield" | "try"
            // Weak keywords, conservatively escaped
            | "macro_rules" | "union" => Some(Self {
                name,
                escaped: true,
            }),
            _ => {
                let mut chars = name.chars();
                let start = chars.next().unwrap();
                if (start == '_' || start.is_xid_start()) && chars.all(UnicodeXID::is_xid_continue) {
                    Some(Self {
                        name,
                        escaped: false,
                    })
                } else {
                    None
                }
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
