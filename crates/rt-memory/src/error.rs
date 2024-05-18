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

/// Error type used when an address was out of bounds.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct BoundsCheckError;

impl core::fmt::Display for BoundsCheckError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("out-of-bounds address")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BoundsCheckError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum LimitsMismatchKind {
    Minimum,
    Maximum,
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
    pub(crate) actual: I,
    pub(crate) expected: I,
    pub(crate) kind: LimitsMismatchKind,
}

impl<I: Address> core::fmt::Display for LimitsMismatchError<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "expected memory #{} to have {} pages ",
            self.memory, self.expected
        )?;
        f.write_str(match self.kind {
            LimitsMismatchKind::Minimum => "minimum",
            LimitsMismatchKind::Maximum => "maximum",
        })?;
        write!(f, ", but got {}", self.actual)
    }
}

#[cfg(feature = "std")]
impl<I: Address> std::error::Error for LimitsMismatchError<I> {}

impl From<LimitsMismatchError<u32>> for LimitsMismatchError<u64> {
    fn from(error: LimitsMismatchError<u32>) -> Self {
        Self {
            memory: error.memory,
            kind: error.kind,
            actual: error.actual.into(),
            expected: error.expected.into(),
        }
    }
}
