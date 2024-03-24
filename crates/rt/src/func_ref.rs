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
    /// specialized `call_` functions instead, such as [`call_0()`], [`call_1()`], etc.
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
    /// [`call_0()`]: Self::call_0()
    /// [`call_1()`]: Self::call_1()
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
}

#[cfg(feature = "alloc")]
struct TypeName<T> {
    _marker: core::marker::PhantomData<fn() -> T>,
}

#[cfg(feature = "alloc")]
impl<T> TypeName<T> {
    const INSTANCE: Self = Self {
        _marker: core::marker::PhantomData,
    };
}

#[cfg(feature = "alloc")]
impl<T> core::fmt::Debug for TypeName<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Debug::fmt(&core::any::type_name::<T>(), f)
    }
}

macro_rules! helpers {
    {$(
        fn $description:literal $call:ident ($($argument:ident: $param:ident),*)
            / $from_closure:ident;
    )*} => {
        /// Helper functions to perform calls without `unsafe` and [`cast()`], and for creating
        /// new [`FuncRef`]s without calling [`from_raw()`].
        ///
        /// [`cast()`]: FuncRef::cast()
        /// [`from_raw()`]: FuncRef::from_raw()
        #[allow(clippy::too_many_arguments)]
        impl<E: 'static> FuncRef<E> {$(
            #[doc = "Calls the referenced function with "]
            #[doc = $description]
            #[doc = ".\n\nMultiple return values are represented by a tuple.\n\n"]
            #[doc = "# Errors\n\n"]
            #[doc = "A [`Trap`] occurs if the function reference is not of the correct type."]
            pub fn $call<$($param,)* R, H>(&self $(, $argument: $param)* , trap: &H) -> Result<R, E>
            where
                $($param: 'static,)*
                H: Trap<Repr = E> + ?Sized,
                R: 'static,
            {
                match self.cast::<unsafe fn(&RawFuncRefData $(, $param)*) -> Result<R, E>>() {
                    Ok((data, func)) => {
                        // SAFETY: only `data` is passed to the `func`.
                        unsafe { func(data $(, $argument)*) }
                    }
                    Err(err) => Err(err.trap_cold(trap)),
                }
            }

            #[doc = "Creates a new [`FuncRef`] used to invoke the given closure with"]
            #[doc = $description]
            #[doc = ".\n\nIf the closure is too large, a heap allocation is used to ensure that"]
            #[doc = "it fits into [`RawFuncRefData`]."]
            #[cfg(feature = "alloc")]
            pub fn $from_closure<$($param,)* R, C>(closure: C) -> Self
            where
                $($param: 'static,)*
                C: Clone + Fn($($param),*) -> Result<R, E> + 'static,
                R: 'static,
            {
                trait Constants<$($param,)* R, E>: Sized {
                    type FuncPtr: Clone + Copy + Send + Sync + core::marker::Unpin
                        + core::panic::UnwindSafe + core::panic::RefUnwindSafe + 'static;

                    const IS_INLINE: bool;
                    const SIGNATURE: FuncRefSignature;
                    const VTABLE: RawFuncRefVTable;

                    unsafe fn from_data(data: &RawFuncRefData) -> &Self {
                        if Self::IS_INLINE {
                            // SAFETY: `inline` had `Self` written into it.
                            unsafe {
                                &*(data.inline.as_ptr() as *const Self)
                            }
                        } else {
                            // SAFETY: `IS_INLINE` is false here.
                            // SAFETY: `pointer` originates from `Box::<Self>::into_raw`.
                            unsafe {
                                &*(data.pointer as *const Self)
                            }
                        }
                    }

                    fn into_data(closure: Self) -> RawFuncRefData {
                        if Self::IS_INLINE {
                            let mut data = RawFuncRefData::UNINIT;

                            // SAFETY: `IS_INLINE` performs size and alignment check.
                            unsafe {
                                core::ptr::write(data.inline.as_mut_ptr() as *mut Self, closure);
                            }

                            data
                        } else {
                            let boxed = alloc::boxed::Box::<Self>::new(closure);
                            RawFuncRefData {
                                pointer: alloc::boxed::Box::into_raw(boxed) as *const ()
                            }
                        }
                    }
                }

                impl<$($param,)* R, C, E> Constants<$($param,)* R, E> for C
                where
                    $($param: 'static,)*
                    R: 'static,
                    C: Clone + Fn($($param),*) -> Result<R, E> + 'static,
                    E: 'static,
                {
                    type FuncPtr = unsafe fn(&RawFuncRefData $(, $param)*) -> Result<R, E>;

                    const IS_INLINE: bool = {
                        let closure = core::alloc::Layout::new::<C>();
                        let data = core::alloc::Layout::new::<RawFuncRefData>();
                        closure.size() <= data.size() && closure.align() <= data.align()
                    };

                    const SIGNATURE: FuncRefSignature = FuncRefSignature::of::<Self::FuncPtr>();

                    const VTABLE: RawFuncRefVTable = {
                        let invoke: Self::FuncPtr = |data $(, $argument)*| {
                            // SAFETY: `data` refers to a valid `Self`.
                            let me = unsafe { Self::from_data(data) };
                            me($($argument),*)
                        };

                        let clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef = |data| {
                            // SAFETY: `data` refers to a valid `Self`.
                            let me = unsafe { Self::from_data(data) };
                            RawFuncRef::new(C::into_data(me.clone()), &C::VTABLE)
                        };

                        let drop: unsafe fn(data: RawFuncRefData) = |mut data| {
                            if C::IS_INLINE {
                                // SAFETY: `IS_INLINE` is true here.
                                // SAFETY: `inline` contains valid instance of `Self`.
                                unsafe {
                                    core::ptr::drop_in_place(data.inline.as_mut_ptr() as *mut C)
                                }
                            } else {
                                // SAFETY: `IS_INLINE` is false here.
                                // SAFETY: `inline` contains `*mut Self` originating from `Box::into_raw`.
                                let boxed = unsafe {
                                    alloc::boxed::Box::from_raw(data.pointer as *mut () as *mut C)
                                };

                                core::mem::drop(boxed);
                            };
                        };

                        let debug: unsafe fn(data: &RawFuncRefData) -> &dyn core::fmt::Debug = |_| {
                            &TypeName::<C>::INSTANCE
                        };

                        RawFuncRefVTable::new(
                            invoke as *const (),
                            &Self::SIGNATURE,
                            clone,
                            drop,
                            debug,
                        )
                    };
                }

                // SAFETY: `VTABLE` should be implemented correctly.
                unsafe {
                    Self::from_raw(RawFuncRef::new(C::into_data(closure), &C::VTABLE))
                }
            }
        )*}
    };
}

helpers! {
    fn "no arguments" call_0() / from_closure_0;
    fn "one argument" call_1(a0: A0) / from_closure_1;
    fn "two arguments" call_2(a0: A0, a1: A1) / from_closure_2;
    fn "three arguments" call_3(a0: A0, a1: A1, a2: A2) / from_closure_3;
    fn "four arguments" call_4(a0: A0, a1: A1, a2: A2, a3: A3) / from_closure_4;
    fn "five arguments" call_5(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4) / from_closure_5;
    fn "six arguments" call_6(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5) / from_closure_6;
    fn "seven arguments" call_7(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6) / from_closure_7;
    fn "eight arguments" call_8(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7) / from_closure_8;
    fn "nine arguments" call_9(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8) / from_closure_9;
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
        match &self.func {
            Some(func) => {
                // SAFETY: ensured by implementor of `debug` in `RawFuncRef`
                let debug = unsafe { (func.vtable().debug)(func.data()) };

                f.debug_struct("FuncRef")
                    .field("function", debug)
                    .field("signature", func.vtable().signature)
                    .finish()
            }
            None => {
                #[derive(Clone, Copy, Debug)]
                struct Null;

                f.debug_tuple("FuncRef").field(&Null).finish()
            }
        }
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
