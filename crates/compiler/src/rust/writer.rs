use std::io::{Result, Write};

/// Helper struct for writing Rust source code.
#[derive(Clone)]
#[must_use = "call .finish()"]
pub struct Writer<O: Write> {
    output: O,
    closing_brackets: u32,
    needs_space: bool,
}

#[allow(missing_docs)]
impl<O: Write> Writer<O> {
    /// Creates a new writer for Rust source code.
    pub fn new(output: O) -> Self {
        Self {
            output,
            closing_brackets: 0,
            needs_space: false,
        }
    }

    fn write_separator_space(&mut self) -> Result<()> {
        if std::mem::replace(&mut self.needs_space, false) {
            self.output.write_all(b" ")?;
        }

        Ok(())
    }

    pub fn open_bracket(&mut self) -> Result<()> {
        self.output.write_all(b"{")?;
        self.closing_brackets.checked_add(1).unwrap();
        Ok(())
    }

    pub fn close_bracket(&mut self) -> Result<()> {
        self.output.write_all(b"}")?;
        self.closing_brackets.checked_sub(1).unwrap();
        Ok(())
    }

    pub fn ident(&mut self, ident: crate::rust::Ident) -> Result<()> {
        self.write_separator_space()?;
        write!(&mut self.output, "{ident}")?;
        self.needs_space = true;
        Ok(())
    }

    pub fn keyword(&mut self, keyword: crate::rust::Keyword) -> Result<()> {
        self.write_separator_space()?;
        write!(&mut self.output, "{keyword}")?;
        self.needs_space = true;
        Ok(())
    }

    /// Writes any remaining closing brackets, then flushes the output stream.
    pub fn finish(mut self) -> Result<O> {
        for _ in 0..self.closing_brackets {
            self.output.write_all(b"}")?;
        }

        self.output.flush()?;
        Ok(self.output)
    }
}

impl<O: std::fmt::Debug + Write> std::fmt::Debug for Writer<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Writer")
            .field("output", &self.output)
            .finish_non_exhaustive()
    }
}
