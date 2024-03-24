//! Runtime support for [references to functions].
//!
//! [references to functions]: https://webassembly.github.io/spec/core/syntax/types.html#reference-types

mod raw;
mod signature;

pub use raw::{RawFuncRef, RawFuncRefData, RawFuncRefVTable};
pub use signature::FuncRefSignature;

use crate::trap::Trap;

/// Error type used when a [`FuncRef`] did not have the correct signature.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct SignatureMismatchError {
    expected: &'static FuncRefSignature,
    actual: &'static FuncRefSignature,
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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(clippy::exhaustive_enums)]
pub enum FuncRefCastError {
    /// The signature of the function reference was not correct.
    SignatureMismatch(SignatureMismatchError),
    /// An attempt was made to [`cast()`] a [`NULL`] function reference.
    ///
    /// [`cast()`]: FuncRef::cast()
    /// [`NULL`]: FuncRef::NULL
    Null {
        /// The signature the [`NULL`] function reference was expected to have.
        ///
        /// [`NULL`]: FuncRef::NULL
        expected: &'static FuncRefSignature,
    },
}

impl FuncRefCastError {
    #[inline(never)]
    #[cold]
    fn trap_cold<T>(self, trap: &T) -> T::Repr
    where
        T: Trap + ?Sized,
    {
        use crate::trap::TrapCode;

        trap.trap(match self {
            Self::Null { expected } => TrapCode::NullFunctionReference {
                expected: Some(expected),
            },
            Self::SignatureMismatch(error) => TrapCode::IndirectCallSignatureMismatch(error),
        })
    }
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

/// Represents a WebAssembly [**`funcref`**].
///
/// The type parameter `E` is the type of values for any errors are returned as a result of calling
/// the function, and is namely used to report WebAssembly [`Trap`]s.
///
/// [**`funcref`**]: https://webassembly.github.io/spec/core/exec/runtime.html#values
pub struct FuncRef<E: 'static> {
    func: Option<RawFuncRef>,
    _marker: core::marker::PhantomData<fn() -> E>,
}

impl<E: 'static> Default for FuncRef<E> {
    fn default() -> Self {
        Self::NULL
    }
}

impl<E: 'static> FuncRef<E> {
    /// Gets the [`null`] function reference.
    ///
    /// [`null`]: https://webassembly.github.io/spec/core/exec/runtime.html#values
    pub const NULL: Self = Self {
        func: None,
        _marker: core::marker::PhantomData,
    };

    /// Returns `true` if this [`FuncRef`] is [`NULL`].
    ///
    /// [`NULL`]: FuncRef::NULL
    pub const fn is_null(&self) -> bool {
        self.func.is_none()
    }

    /// Creates a new [`FuncRef`] from a [`RawFuncRef`].
    ///
    /// # Safety
    ///
    /// The provided [`RawFuncRef`] must meet the requirements specified in its documentation. For
    /// more information, see the documentation for [`RawFuncRefVTable::new()`].
    pub const unsafe fn from_raw(func_ref: RawFuncRef) -> Self {
        Self {
            func: Some(func_ref),
            _marker: core::marker::PhantomData,
        }
    }

    /// Attempts to cast this reference to some exact type.
    ///
    /// This is an implementation detail used to support generated code. Prefer calling the
    /// specialized `call_` functions instead, such as [`call_0_1()`], [`call_1_1()`], etc.
    ///
    /// Generated code and the `call_` functions call [`cast()`] to obtain a function pointer of
    /// type `C` to the referrenced function. Refer to the documentation for
    /// [`FuncRefSignature::of()`] for valid types to use as `C`.
    ///
    /// The function pointer that is produced is safe to call only with the [`RawFuncRefData`] that
    /// is returned alongside it.
    ///
    /// # Errors
    ///
    /// An error is returned if the function reference is not of the correct type, or if `self` is [`NULL`]
    ///
    /// [`cast()`]: FuncRef::cast()
    /// [`NULL`]: FuncRef::NULL
    /// [`call_0_1()`]: Self::call_0_1()
    pub fn cast<C>(&self) -> Result<(&RawFuncRefData, C), FuncRefCastError>
    where
        C: Clone
            + Copy
            + Send
            + Sync
            + core::marker::Unpin
            + core::panic::UnwindSafe
            + core::panic::RefUnwindSafe
            + 'static,
    {
        let expected: &'static FuncRefSignature = &<C as signature::HasFuncRefSignature>::SIGNATURE;

        let func = match &self.func {
            Some(func) => func,
            None => return Err(FuncRefCastError::Null { expected }),
        };

        let invoke = func.vtable().invoke;
        if expected != func.vtable().signature {
            Err(FuncRefCastError::SignatureMismatch(
                SignatureMismatchError {
                    expected,
                    actual: func.vtable().signature,
                },
            ))
        } else if core::mem::size_of::<C>() != core::mem::size_of_val(&invoke) {
            panic!("size mismatch, expected {expected:?} to be the same size as pointer")
        } else {
            // SAFETY: check above ensures sizes are the same.
            // SAFETY: implementor of `RawFuncRefVTable` ensures this.
            let casted = unsafe { core::mem::transmute_copy::<*const (), C>(&invoke) };

            Ok((func.data(), casted))
        }
    }

    /// Obtains a closure to perform a function call with no arguments and at most one return value.
    ///
    /// # Errors
    ///
    /// A [`Trap`] occurs if the function reference is not of the correct type.
    pub fn call_0_1<R, H>(&self, trap: &H) -> Result<R, E>
    where
        H: Trap<Repr = E> + ?Sized,
        R: 'static,
    {
        match self.cast::<unsafe fn(&RawFuncRefData) -> Result<R, E>>() {
            Ok((data, func)) => {
                // SAFETY: only `data` is passed to the `func`.
                unsafe { func(data) }
            }
            Err(err) => Err(err.trap_cold(trap)),
        }
    }
}

impl<E> Clone for FuncRef<E> {
    fn clone(&self) -> Self {
        match &self.func {
            None => Self::NULL,
            Some(func) => {
                // SAFETY: ensured by implementor of `clone` in `RawFuncRef`
                unsafe {
                    let cloned = (func.vtable().clone)(func.data());
                    Self::from_raw(cloned)
                }
            }
        }
    }
}

impl<E> core::fmt::Debug for FuncRef<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let field = match &self.func {
            Some(func) => {
                // SAFETY: ensured by implementor of `debug` in `RawFuncRef`
                unsafe { (func.vtable().debug)(func.data()) }
            }
            None => {
                #[derive(Clone, Copy, Debug)]
                struct Null;

                &Null
            }
        };

        f.debug_tuple("FuncRef").field(field).finish()
    }
}

impl<E> Drop for FuncRef<E> {
    fn drop(&mut self) {
        if let Some(func) = core::mem::take(&mut self.func) {
            // SAFETY: the `func` won't be used after this point, so the data is "moved" out.
            // SAFETY: ensured by implementor.
            unsafe { (func.vtable().drop)(*func.data()) }
        }
    }
}
