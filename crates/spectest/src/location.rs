//! Types describing locations into a `.wast` file.

/// Represents a line and column number associated with a file.
#[derive(Clone, Copy)]
pub struct Location<'a> {
    line: u32,
    column: u32,
    path: &'a std::path::Path,
}

impl<'a> Location<'a> {
    pub fn new(path: &'a std::path::Path, span: wast::token::Span, text: &str) -> Self {
        let (line, col) = span.linecol_in(text);
        Self {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            column: u32::try_from(col).unwrap_or(u32::MAX),
            path,
        }
    }
}

impl std::fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.path.display(),
            u64::from(self.line).saturating_add(1),
            u64::from(self.column).saturating_add(1)
        )
    }
}

/// Represents a path to a file and its contents.
#[derive(Clone, Copy)]
pub struct Contents<'a> {
    path: &'a std::path::Path,
    contents: &'a str,
}

impl<'a> Contents<'a> {
    pub fn new(contents: &'a str, path: &'a std::path::Path) -> Self {
        Self { contents, path }
    }

    pub fn location(&self, span: wast::token::Span) -> Location<'a> {
        Location::new(self.path, span, self.contents)
    }
}
