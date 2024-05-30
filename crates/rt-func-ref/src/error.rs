/// Error type used when a [`FuncRef`] did not have the correct signature.
///
/// [`FuncRef`]: crate::FuncRef
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct SignatureMismatchError {
    pub(crate) expected: &'static crate::FuncRefSignature,
    pub(crate) actual: &'static crate::FuncRefSignature,
}

impl core::fmt::Debug for SignatureMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SignatureMismatchError")
            .field("expected", self.expected)
            .field("actual", self.actual)
            .finish()
    }
}

impl core::fmt::Display for SignatureMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "signature mismatch: expected {:?}, but got {:?}",
            self.expected, self.actual
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SignatureMismatchError {}

/// Error type used with the [`FuncRef::cast()`] function.
///
/// [`FuncRef::cast()`]: crate::FuncRef::cast()
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(clippy::exhaustive_enums)]
pub enum FuncRefCastError {
    /// The signature of the function reference was not correct.
    SignatureMismatch(SignatureMismatchError),
    /// An attempt was made to [`cast()`] a [`NULL`] function reference.
    ///
    /// [`cast()`]: crate::FuncRef::cast()
    /// [`NULL`]: crate::FuncRef::NULL
    Null {
        /// The signature the [`NULL`] function reference was expected to have.
        ///
        /// [`NULL`]: crate::FuncRef::NULL
        expected: &'static crate::FuncRefSignature,
    },
}

impl core::fmt::Display for FuncRefCastError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SignatureMismatch(err) => core::fmt::Display::fmt(err, f),
            Self::Null { expected } => write!(
                f,
                "expected signature {expected:?} for null function reference"
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for FuncRefCastError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SignatureMismatch(err) => Some(err),
            Self::Null { .. } => None,
        }
    }
}
