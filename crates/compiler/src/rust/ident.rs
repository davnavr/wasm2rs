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
    pub fn new(name: &'a str) -> Option<Self> {
        if matches!(name, "" | "_") {
            return None;
        }

        if !name.is_ascii() {
            todo!("handling of exotic identifiers is not yet implemented")
        } else {
            let mut chars = name.chars();
            let start = chars.next().unwrap(); // Won't panic, empty case handled above.
            let escaped = if start.is_ascii_uppercase() || start == '_' {
                false
            } else if start.is_ascii_lowercase() {
                true
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
