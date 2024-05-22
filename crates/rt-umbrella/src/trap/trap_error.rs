use crate::trap::Trap;

/// Indicates why a trap occured.
///
/// Used with the [`TrapError`] struct.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TrapCause {
    /// An [**`unreachable`**] instruction was executed. This corresponds to a
    /// [`trap::UnreachableError`].
    ///
    /// [`trap::UnreachableError`]: crate::trap::UnreachableError
    /// [**`unreachable`**]: https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-control
    #[non_exhaustive]
    Unreachable {
        #[allow(missing_docs)]
        error: crate::trap::UnreachableError,
    },
    /// An attempt to convert a float value to an integer failed. This corresponds to a
    /// [`math::ConversionToIntegerError`].
    ///
    /// [`math::ConversionToIntegerError`]: crate::math::ConversionToIntegerError
    #[non_exhaustive]
    ConversionToInteger {
        #[allow(missing_docs)]
        error: crate::math::ConversionToIntegerError,
    },
    /// An integer operation attempted a division by zero. This corresponds to a
    /// [`math::DivisionByZeroError`].
    ///
    /// [`math::DivisionByZeroError`]: crate::math::DivisionByZeroError
    #[non_exhaustive]
    IntegerDivisionByZero {
        #[allow(missing_docs)]
        error: crate::math::DivisionByZeroError,
    },
    /// An integer operation overflowed. This corresponds to a [`math::IntegerOverflowError`].
    ///
    /// [`math::IntegerOverflowError`]: crate::math::IntegerOverflowError
    #[non_exhaustive]
    IntegerOverflow {
        #[allow(missing_docs)]
        error: crate::math::IntegerOverflowError,
    },
    /// A memory access was out of bounds. This corresponds to a [`memory::AccessError`].
    ///
    /// [`memory::AccessError`]: crate::memory::AccessError
    #[non_exhaustive]
    MemoryBoundsCheck {
        #[allow(missing_docs)]
        #[cfg(all(feature = "alloc", feature = "memory"))]
        access: crate::memory::AccessError<u64>,
    },
    //UnalignedAtomicOperation
    /// Instantiating a module failed because linear memory could not be allocated. This
    /// corresponds to a [`store::AllocateMemoryError`].
    ///
    /// [`store::AllocateMemoryError`]: crate::store::AllocateMemoryError
    #[non_exhaustive]
    MemoryAllocationFailure {
        #[allow(missing_docs)]
        #[cfg(all(feature = "alloc", feature = "memory"))]
        error: crate::store::AllocateMemoryError<u64>,
    },
    /// Instantiating a module failed because a linear memory did not have matching limits. This
    /// corresponds to a [`memory::LimitsMismatchError`].
    ///
    /// [`memory::LimitsMismatchError`]: crate::memory::LimitsMismatchError
    #[non_exhaustive]
    MemoryLimitsMismatch {
        #[allow(missing_docs)]
        #[cfg(all(feature = "alloc", feature = "memory"))]
        error: crate::memory::LimitsMismatchError<u64>,
    },
    /// A function reference did not have the correct signature. This corresponds to a
    /// [`func_ref::SignatureMismatchError`], typically originating from a
    /// [`func_ref::FuncRefCastError::SignatureMismatch`].
    ///
    /// [`func_ref::SignatureMismatchError`]: crate::func_ref::SignatureMismatchError
    /// [`func_ref::FuncRefCastError::SignatureMismatch`]: crate::func_ref::FuncRefCastError::SignatureMismatch
    #[non_exhaustive]
    IndirectCallSignatureMismatch {
        #[allow(missing_docs)]
        #[cfg(all(feature = "alloc", feature = "func-ref"))]
        mismatch: crate::func_ref::SignatureMismatchError,
    },
    /// A function reference was null. This corresponds to [`func_ref::FuncRefCastError::Null`].
    ///
    /// [`func_ref::FuncRefCastError::Null`]: crate::func_ref::FuncRefCastError::Null
    #[non_exhaustive]
    NullFunctionReference {
        /// The type that the function reference was expected to have.
        #[cfg(all(feature = "alloc", feature = "func-ref"))]
        expected: &'static crate::func_ref::FuncRefSignature,
    },
    /// The stack space was exhausted, usually due to an infinitely recursive function. This
    /// corresponds to a [`stack::StackOverflowError`].
    ///
    /// See the documentation for [`stack::check_for_overflow()`] for more information.
    ///
    /// [`stack::StackOverflowError`]: crate::stack::StackOverflowError
    /// [`stack::check_for_overflow()`]: crate::stack::check_for_overflow()
    #[non_exhaustive]
    CallStackExhausted {
        #[allow(missing_docs)]
        error: crate::stack::StackOverflowError,
    },
}

// impl From<crate::math::DivisionByZeroError> for TrapCause
// impl From<> for TrapCause

impl core::fmt::Display for TrapCause {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unreachable { error } => core::fmt::Display::fmt(error, f),
            Self::ConversionToInteger { error } => core::fmt::Display::fmt(error, f),
            Self::IntegerDivisionByZero { error } => core::fmt::Display::fmt(error, f),
            Self::IntegerOverflow { error } => core::fmt::Display::fmt(error, f),
            #[cfg(all(feature = "alloc", feature = "memory"))]
            Self::MemoryBoundsCheck { access } => core::fmt::Display::fmt(access, f),
            #[cfg(not(all(feature = "alloc", feature = "memory")))]
            Self::MemoryBoundsCheck {} => f.write_str("memory access was out of bounds"),
            #[cfg(all(feature = "alloc", feature = "memory"))]
            Self::MemoryAllocationFailure { error } => core::fmt::Display::fmt(error, f),
            #[cfg(not(all(feature = "alloc", feature = "memory")))]
            Self::MemoryAllocationFailure {} => f.write_str("memory allocation failure"),
            #[cfg(all(feature = "alloc", feature = "memory"))]
            Self::MemoryLimitsMismatch { error } => core::fmt::Display::fmt(error, f),
            #[cfg(not(all(feature = "alloc", feature = "memory")))]
            Self::MemoryLimitsMismatch {} => f.write_str("incorrect memory limits"),
            #[cfg(all(feature = "alloc", feature = "func-ref"))]
            Self::IndirectCallSignatureMismatch { mismatch } => {
                write!(f, "function reference {mismatch}")
            }
            #[cfg(not(all(feature = "alloc", feature = "func-ref")))]
            Self::IndirectCallSignatureMismatch {} => {
                f.write_str("function reference signature mismatch")
            }
            Self::NullFunctionReference {
                #[cfg(all(feature = "alloc", feature = "func-ref"))]
                expected,
            } => {
                #[cfg(all(feature = "alloc", feature = "func-ref"))]
                write!(f, "expected signature {expected:?} for ")?;

                f.write_str("null function reference")
            }
            Self::CallStackExhausted { error } => core::fmt::Display::fmt(error, f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TrapCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(all(feature = "alloc", feature = "memory"))]
            Self::MemoryBoundsCheck { access } => Some(access),
            #[cfg(all(feature = "alloc", feature = "memory"))]
            Self::MemoryAllocationFailure { error } => Some(error),
            #[cfg(all(feature = "alloc", feature = "func-ref"))]
            Self::IndirectCallSignatureMismatch { mismatch } => Some(mismatch),
            Self::CallStackExhausted { error } => Some(error),
            _ => None,
        }
    }
}

#[cfg(feature = "alloc")]
struct Inner {
    cause: TrapCause,
    frame: Option<&'static crate::trace::WasmFrame>,
    //backtrace: (backtrace::Backtrace, Vec<Option<crate::trace::WasmFrame>>),
}

/// Describes a WebAssembly trap.
///
/// If the [`alloc`](crate#alloc) feature is not enabled, only the [`TrapCause`] is stored.
#[repr(transparent)]
pub struct TrapError {
    #[cfg(not(feature = "alloc"))]
    cause: TrapCause,
    #[cfg(feature = "alloc")]
    inner: alloc::boxed::Box<Inner>,
}

impl TrapError {
    const _SIZE_CHECK: () = if core::mem::size_of::<Option<Self>>() > core::mem::size_of::<usize>()
    {
        panic!("TrapError is too big")
    };

    fn new(cause: TrapCause, frame: Option<&'static crate::trace::WasmFrame>) -> Self {
        #[cfg(not(feature = "alloc"))]
        return {
            let _ = frame;
            Self { cause }
        };

        #[cfg(feature = "alloc")]
        Self {
            inner: alloc::boxed::Box::new(Inner { cause, frame }),
        }
    }

    /// Gets the reason why the trap occurred.
    pub fn cause(&self) -> &TrapCause {
        #[cfg(not(feature = "alloc"))]
        return &self.cause;

        #[cfg(feature = "alloc")]
        &self.inner.cause
    }

    /// Checks that if the trap occurred due to the specified `reason`. The exact strings used
    /// correspond to those used in the [WebAssembly test suite].
    ///
    /// [WebAssembly test suite]: https://github.com/WebAssembly/testsuite
    pub fn matches_spec_failure(&self, reason: &str) -> bool {
        match self.cause() {
            TrapCause::Unreachable { .. } => reason == "unreachable",
            TrapCause::IntegerDivisionByZero { .. } => reason == "integer divide by zero",
            TrapCause::IntegerOverflow { .. } => reason == "integer overflow",
            TrapCause::ConversionToInteger { .. } => reason == "invalid conversion to integer",
            TrapCause::MemoryBoundsCheck { .. } => reason == "out of bounds memory access",
            _ => false,
        }
    }
}

impl core::cmp::PartialEq for TrapError {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(not(feature = "alloc"))]
        return self.cause == other.cause;

        #[cfg(feature = "alloc")]
        return self.inner.cause == other.inner.cause; // self.backtrace == other.backtrace
    }
}

impl crate::trace::Trace for TrapError {
    // TODO: Implement fn push_wasm_frame
}

impl Trap<crate::trap::UnreachableError> for TrapError {
    fn trap(
        cause: crate::trap::UnreachableError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(TrapCause::Unreachable { error: cause }, frame)
    }

    // TODO: Make a common `TrapInfo` trait that has `as_error and `as_display` and `Self: Debug + Trace`
    // #[cfg(feature = "std")]
    // fn as_error(&self) -> Option<&(dyn std::error::Error + '_)> {
    //     Some(self)
    // }
}

impl Trap<crate::math::ConversionToIntegerError> for TrapError {
    fn trap(
        cause: crate::math::ConversionToIntegerError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(TrapCause::ConversionToInteger { error: cause }, frame)
    }
}

impl Trap<crate::math::DivisionByZeroError> for TrapError {
    fn trap(
        cause: crate::math::DivisionByZeroError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(TrapCause::IntegerDivisionByZero { error: cause }, frame)
    }
}

impl Trap<crate::math::IntegerOverflowError> for TrapError {
    fn trap(
        cause: crate::math::IntegerOverflowError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(TrapCause::IntegerOverflow { error: cause }, frame)
    }
}

impl Trap<crate::math::IntegerDivisionError> for TrapError {
    fn trap(
        cause: crate::math::IntegerDivisionError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        match cause {
            rt_math::IntegerDivisionError::DivisionByZero => {
                Self::trap(rt_math::DivisionByZeroError, frame)
            }
            rt_math::IntegerDivisionError::Overflow => {
                Self::trap(rt_math::IntegerOverflowError, frame)
            }
        }
    }
}

#[cfg(feature = "memory")]
impl<I> Trap<crate::memory::AccessError<I>> for TrapError
where
    I: crate::memory::Address,
    crate::memory::AccessError<I>: Into<crate::memory::AccessError<u64>>,
{
    fn trap(
        cause: crate::memory::AccessError<I>,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(
            TrapCause::MemoryBoundsCheck {
                #[cfg(feature = "alloc")]
                access: cause.into(),
            },
            frame,
        )
    }
}

#[cfg(feature = "memory")]
impl<I> Trap<crate::store::AllocateMemoryError<I>> for TrapError
where
    I: crate::memory::Address,
    crate::store::AllocateMemoryError<I>: Into<crate::store::AllocateMemoryError<u64>>,
{
    fn trap(
        cause: crate::store::AllocateMemoryError<I>,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(
            TrapCause::MemoryAllocationFailure {
                error: cause.into(),
            },
            frame,
        )
    }
}

#[cfg(feature = "memory")]
impl<I> Trap<crate::memory::LimitsMismatchError<I>> for TrapError
where
    I: crate::memory::Address,
    crate::memory::LimitsMismatchError<I>: Into<crate::memory::LimitsMismatchError<u64>>,
{
    fn trap(
        cause: crate::memory::LimitsMismatchError<I>,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(
            TrapCause::MemoryLimitsMismatch {
                error: cause.into(),
            },
            frame,
        )
    }
}

#[cfg(feature = "func-ref")]
impl Trap<crate::func_ref::SignatureMismatchError> for TrapError {
    fn trap(
        cause: crate::func_ref::SignatureMismatchError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(
            TrapCause::IndirectCallSignatureMismatch {
                #[cfg(feature = "alloc")]
                mismatch: cause,
            },
            frame,
        )
    }
}

#[cfg(feature = "func-ref")]
impl Trap<crate::func_ref::FuncRefCastError> for TrapError {
    fn trap(
        cause: crate::func_ref::FuncRefCastError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        match cause {
            crate::func_ref::FuncRefCastError::SignatureMismatch(error) => Self::trap(error, frame),
            crate::func_ref::FuncRefCastError::Null { expected } => Self::new(
                TrapCause::NullFunctionReference {
                    #[cfg(feature = "alloc")]
                    expected,
                },
                frame,
            ),
        }
    }
}

impl Trap<crate::stack::StackOverflowError> for TrapError {
    fn trap(
        cause: crate::stack::StackOverflowError,
        frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
    ) -> Self {
        Self::new(TrapCause::CallStackExhausted { error: cause }, frame)
    }
}

impl core::fmt::Display for TrapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.cause())?;

        // TODO: Write backtrace and other stuff

        Ok(())
    }
}

impl core::fmt::Debug for TrapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("TrapError");
        s.field("code", self.cause());

        #[cfg(feature = "alloc")]
        s.field("frame", &self.inner.frame);

        s.finish()
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TrapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause())
    }
}

impl From<crate::trap::TrapError> for crate::trap::TrapOccurred {
    fn from(error: crate::trap::TrapError) -> Self {
        let _ = error;
        Self
    }
}
