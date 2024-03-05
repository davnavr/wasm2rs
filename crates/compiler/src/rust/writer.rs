use std::io::{Result, Write};

/// Helper struct for writing Rust source code.
#[derive(Clone)]
#[must_use = "call .finish()"]
pub struct Writer<O: Write> {
    output: O,
    closing_brackets: u32,
}

#[allow(missing_docs)]
impl<O: Write> Writer<O> {
    /// Creates a new writer for Rust source code.
    pub fn new(output: O) -> Self {
        Self {
            output,
            closing_brackets: 0,
        }
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
        write!(&mut self.output, "{ident}")
    }

    /// Writes any remaining closing brackets, then flushes the output stream.
    pub fn finish(mut self) -> Result<O> {
        for _ in 0..self.closing_brackets {
            self.output.write_all(b"}")?;
        }

        self.closing_brackets = 0;
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
