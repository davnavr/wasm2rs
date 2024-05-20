/// Represents a line and column number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Position {
    line: u16,
    column: u16,
}

impl Position {
    fn into_text(span: wast::token::Span, text: &str) -> Self {
        let (line, col) = span.linecol_in(text);
        Self {
            line: u16::try_from(line).unwrap_or(u16::MAX),
            column: u16::try_from(col).unwrap_or(u16::MAX),
        }
    }
}

type ErrorCause<'a> = Box<dyn std::error::Error + Send + Sync + 'a>;

struct ErrorMessageInner<'a> {
    path: &'a std::path::Path,
    message: std::borrow::Cow<'a, str>,
    cause: Option<ErrorCause<'a>>,
    position: Option<Position>,
}

/// Represents a single error that occurred during conversion.
pub(crate) struct ErrorMessage<'a>(Box<ErrorMessageInner<'a>>);

impl<'a> ErrorMessage<'a> {
    fn new(
        path: &'a std::path::Path,
        message: impl Into<std::borrow::Cow<'a, str>>,
        cause: Option<ErrorCause<'a>>,
        position: Option<Position>,
    ) -> Self {
        Self(Box::new(ErrorMessageInner {
            path,
            message: message.into(),
            cause,
            position,
        }))
    }
}

impl std::fmt::Debug for ErrorMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorMessage")
            .field("path", &self.0.path)
            .field("position", &self.0.position)
            .field("message", &self.0.message)
            .field("cause", &self.0.cause)
            .finish()
    }
}

impl std::fmt::Display for ErrorMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.path.display())?;
        if let Some(position) = self.0.position {
            write!(
                f,
                ":{}:{}",
                u32::from(position.line) + 1,
                u32::from(position.column) + 1
            )?;
        }

        write!(f, ": error: {}", self.0.message)?;

        if let Some(cause) = &self.0.cause {
            write!(f, "\ninfo: {cause:#}")?;
        }

        Ok(())
    }
}

/// Error type used when a conversion fails.
#[derive(Debug)]
pub struct Error<'a> {
    messages: Vec<ErrorMessage<'a>>,
}

impl<'a> From<ErrorMessage<'a>> for Error<'a> {
    fn from(message: ErrorMessage<'a>) -> Self {
        Self {
            messages: vec![message],
        }
    }
}

impl<'a> Error<'a> {
    pub(crate) fn with_path(
        path: &'a std::path::Path,
        message: impl Into<std::borrow::Cow<'a, str>>,
        cause: Option<ErrorCause<'a>>,
    ) -> Self {
        ErrorMessage::new(path, message, cause, None).into()
    }

    pub(crate) fn with_path_and_cause<'cause: 'a>(
        path: &'a std::path::Path,
        message: impl Into<std::borrow::Cow<'a, str>>,
        cause: impl Into<ErrorCause<'cause>>,
    ) -> Self {
        Self::with_path(path, message, Some(cause.into()))
    }

    pub(crate) fn with_position_into_text(
        path: &'a std::path::Path,
        message: impl Into<std::borrow::Cow<'a, str>>,
        cause: Option<ErrorCause<'a>>,
        span: wast::token::Span,
        text: &str,
    ) -> Self {
        ErrorMessage::new(path, message, cause, Some(Position::into_text(span, text))).into()
    }

    pub(crate) fn append(&mut self, mut other: Self) {
        self.messages.append(&mut other.messages);
    }

    pub(crate) fn collect(errors: impl IntoIterator<Item = Self>) -> Result<(), Self> {
        let mut errors = errors.into_iter();
        match errors.next() {
            None => Ok(()),
            Some(mut first) => {
                first.extend(errors);
                Err(first)
            }
        }
    }
}

impl std::iter::FromIterator<Self> for Error<'_> {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        Self {
            messages: iter
                .into_iter()
                .map(|Error { messages }| messages)
                .flatten()
                .collect(),
        }
    }
}

impl std::iter::Extend<Self> for Error<'_> {
    fn extend<T: IntoIterator<Item = Self>>(&mut self, iter: T) {
        self.messages.extend(
            iter.into_iter()
                .map(|Error { messages }| messages)
                .flatten(),
        )
    }
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for error in self.messages.iter() {
            writeln!(f, "{error}")?;
        }

        Ok(())
    }
}
