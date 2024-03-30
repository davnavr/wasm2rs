/// A valid Rust identifier constructed from an arbitrary string.
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct MangledIdent<'a>(pub &'a str);

impl<'a> From<&'a str> for MangledIdent<'a> {
    fn from(ident: &'a str) -> Self {
        Self(ident)
    }
}

impl MangledIdent<'_> {
    /// Indicates the start of a mangled identifier.
    pub const START: &'static str = "_WR_";
}

impl std::fmt::Display for MangledIdent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        f.write_str(Self::START)?;

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
