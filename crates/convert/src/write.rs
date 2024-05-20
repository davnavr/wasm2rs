//! Helper traits for writing UTF-8.

#![allow(missing_docs)]

/// Trait for writing UTF-8 into streams or buffers.
///
/// This is like the [`std::fmt::Write`] trait, except that it's methods do not return an error.
pub trait Write {
    fn write_str(&mut self, s: &str);

    fn write_fmt(&mut self, args: std::fmt::Arguments);
}

#[must_use = "call .into_inner()"]
pub struct IoWrite<'a> {
    writer: &'a mut dyn std::io::Write,
    result: std::io::Result<()>,
}

impl<'a> IoWrite<'a> {
    pub fn new(writer: &'a mut dyn std::io::Write) -> Self {
        Self {
            writer,
            result: Ok(()),
        }
    }

    pub fn try_borrow_mut(&mut self) -> std::io::Result<&mut dyn std::io::Write> {
        match std::mem::replace(&mut self.result, Ok(())) {
            Ok(()) => Ok(&mut self.writer),
            Err(err) => Err(err),
        }
    }

    pub fn flush(&mut self) {
        if self.result.is_ok() {
            self.result = self.writer.flush();
        }
    }

    pub fn into_inner(self) -> std::io::Result<&'a mut dyn std::io::Write> {
        self.result?;
        Ok(self.writer)
    }
}

impl Write for IoWrite<'_> {
    fn write_str(&mut self, s: &str) {
        if self.result.is_ok() {
            self.result = self.writer.write_all(s.as_bytes());
        }
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments) {
        if self.result.is_ok() {
            self.result = self.writer.write_fmt(args);
        }
    }
}

impl std::fmt::Debug for IoWrite<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IoWrite")
            .field("error", &self.result.as_ref().err())
            .finish_non_exhaustive()
    }
}
