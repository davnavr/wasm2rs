/// Error type used when a translation operation fails.
#[repr(transparent)]
pub struct Error {
    inner: Box<ErrorInner>,
}

enum ErrorInner {
    Parser(wasmparser::BinaryReaderError),
    IO(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self {
            inner: ErrorInner::IO(error).into(),
        }
    }
}

impl From<wasmparser::BinaryReaderError> for Error {
    fn from(error: wasmparser::BinaryReaderError) -> Self {
        Self {
            inner: ErrorInner::Parser(error).into(),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &*self.inner {
            ErrorInner::Parser(e) => std::fmt::Debug::fmt(e, f),
            ErrorInner::IO(e) => std::fmt::Debug::fmt(e, f),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &*self.inner {
            ErrorInner::Parser(e) => std::fmt::Display::fmt(e, f),
            ErrorInner::IO(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match &*self.inner {
            ErrorInner::Parser(e) => e,
            ErrorInner::IO(e) => e,
        })
    }
}
