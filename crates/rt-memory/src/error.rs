use crate::Address;

/// Error type used when the minimum required number of [pages] for a linear memory could not be
/// allocated.
///
/// [pages]: crate::PAGE_SIZE
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AllocationError<I: Address = u32> {
    pub(crate) size: I,
}

impl<I: Address> AllocationError<I> {
    /// The minimum number of [pages] that was requested.
    ///
    /// [pages]: crate::PAGE_SIZE
    pub fn size(&self) -> I {
        self.size
    }
}

impl<I: Address> core::fmt::Display for AllocationError<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "couldn't allocate {} pages", self.size)
    }
}

#[cfg(feature = "std")]
impl<I: Address> std::error::Error for AllocationError<I> {}

impl From<AllocationError<u32>> for AllocationError<u64> {
    fn from(error: AllocationError<u32>) -> Self {
        Self {
            size: error.size.into(),
        }
    }
}

/// Error type used when an attempt to read or write from a linear [`Memory`] fails.
///
/// [`Memory`]: crate::Memory
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct AccessError<I: Address = u32> {
    memory: u32,
    address: crate::EffectiveAddress<I>,
}

impl<I: Address> AccessError<I> {
    pub(crate) const fn new(memory: u32, address: crate::EffectiveAddress<I>) -> Self {
        Self { memory, address }
    }
}

impl<I: Address> core::fmt::Display for AccessError<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "invalid access of linear memory #{} at address {:#X}",
            self.memory, self.address
        )
    }
}

#[cfg(feature = "std")]
impl<I: Address> std::error::Error for AccessError<I> {}

impl From<AccessError<u32>> for AccessError<u64> {
    fn from(error: AccessError<u32>) -> Self {
        Self {
            memory: error.memory,
            address: error.address.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum LimitsMismatchKind<I: Address> {
    Invalid { minimum: I, maximum: I },
    Minimum { actual: I, expected: I },
    Maximum { actual: I, expected: I },
}

impl From<LimitsMismatchKind<u32>> for LimitsMismatchKind<u64> {
    fn from(kind: LimitsMismatchKind<u32>) -> Self {
        match kind {
            LimitsMismatchKind::Invalid { minimum, maximum } => Self::Invalid {
                minimum: minimum.into(),
                maximum: maximum.into(),
            },
            LimitsMismatchKind::Minimum { actual, expected } => Self::Minimum {
                actual: actual.into(),
                expected: expected.into(),
            },
            LimitsMismatchKind::Maximum { actual, expected } => Self::Maximum {
                actual: actual.into(),
                expected: expected.into(),
            },
        }
    }
}

/// Error type used when a linear [`Memory`]'s limits do not [match]. For more information, see the
/// documentation for the [`check_limits()`] method.
///
/// [`Memory`]: crate::Memory
/// [match]: https://webassembly.github.io/spec/core/valid/types.html#match-limits
/// [`check_limits()`]: crate::check_limits()
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LimitsMismatchError<I: Address = u32> {
    pub(crate) memory: u32,
    pub(crate) kind: LimitsMismatchKind<I>,
}

impl<I: Address> core::fmt::Display for LimitsMismatchError<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            LimitsMismatchKind::Invalid { minimum, maximum } => write!(
                f,
                "memory #{} has {minimum} pages minimum, exceeding its maximum of {maximum}",
                self.memory
            ),
            LimitsMismatchKind::Minimum { actual, expected } => write!(
                f,
                "expected {expected} pages minimum for memory #{}, but got {actual}",
                self.memory
            ),
            LimitsMismatchKind::Maximum { actual, expected } => write!(
                f,
                "expected {expected} pages maximum for memory #{}, but got {actual}",
                self.memory
            ),
        }
    }
}

#[cfg(feature = "std")]
impl<I: Address> std::error::Error for LimitsMismatchError<I> {}

impl From<LimitsMismatchError<u32>> for LimitsMismatchError<u64> {
    fn from(error: LimitsMismatchError<u32>) -> Self {
        Self {
            memory: error.memory,
            kind: error.kind.into(),
        }
    }
}
