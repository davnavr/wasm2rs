//! Helper traits for writing UTF-8.

/// Trait for writing UTF-8 into streams or buffers.
///
/// This is like the [`std::fmt::Write`] trait, except that it's methods do not return an error.
pub(crate) trait Write {
    fn write_str(&mut self, s: &str);
    fn write_fmt(&mut self, args: std::fmt::Arguments);
}

#[must_use = "call .into_inner()"]
pub(crate) struct IoWrite<'a> {
    writer: &'a mut dyn std::io::Write,
    result: std::io::Result<()>,
}

impl<'a> IoWrite<'a> {
    pub(crate) fn new(writer: &'a mut dyn std::io::Write) -> Self {
        Self {
            writer,
            result: Ok(()),
        }
    }

    pub(crate) fn into_inner(self) -> std::io::Result<&'a mut dyn std::io::Write> {
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
