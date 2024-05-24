/// Error type used when the [`size()`] of a table could not be increased.
///
/// [`size()`]: crate::AnyTable::size()
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocationError {
    pub(crate) size: u32,
}

impl AllocationError {
    /// The number of elements that was requested.
    pub fn size(&self) -> u32 {
        self.size
    }
}

impl core::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "couldn't allocate {} elements", self.size)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AllocationError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum LimitsMismatchKind {
    Invalid { minimum: u32, maximum: u32 },
    Minimum { actual: u32, expected: u32 },
    Maximum { actual: u32, expected: u32 },
}

/// Error type used when a [`Table`]'s limits do not [match]. For more information, see the
/// documentation for the [`check_limits()`] method.
///
/// [`Table`]: crate::Table
/// [match]: https://webassembly.github.io/spec/core/valid/types.html#match-limits
/// [`check_limits()`]: crate::check_limits()
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LimitsMismatchError {
    pub(crate) table: u32,
    pub(crate) kind: LimitsMismatchKind,
}

impl core::fmt::Display for LimitsMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            LimitsMismatchKind::Invalid { minimum, maximum } => write!(
                f,
                "table #{} has {minimum} elements minimum, exceeding its maximum of {maximum}",
                self.table
            ),
            LimitsMismatchKind::Minimum { actual, expected } => write!(
                f,
                "expected {expected} elements minimum for table #{}, but got {actual}",
                self.table
            ),
            LimitsMismatchKind::Maximum { actual, expected } => write!(
                f,
                "expected {expected} elements maximum for table #{}, but got {actual}",
                self.table
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for LimitsMismatchError {}

/// Error type used when an index into a [`Table`] is out of bounds.
///
/// [`Table`]: crate::Table
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AccessError {
    pub(crate) table: u32,
    pub(crate) index: u32,
}

impl core::fmt::Display for AccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "index {} is out of bounds for table #{}",
            self.index, self.table
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AccessError {}

impl From<AccessError> for crate::BoundsCheckError {
    fn from(error: AccessError) -> Self {
        let _ = error;
        Self
    }
}
