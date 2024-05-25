//! Runtime support for [references to functions] in `wasm2rs`.
//!
//! [references to functions]: https://webassembly.github.io/spec/core/syntax/types.html#reference-types

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod raw;
mod signature;

pub use raw::{RawFuncRef, RawFuncRefData, RawFuncRefVTable};
pub use signature::FuncRefSignature;

use wasm2rs_rt_core::trap::Trap;

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

struct FuncRefPhantom<'a, E: 'static> {
    /// Allows a [`FuncRef`] to reference things that live for at least `'a`.
    _lifetime: &'a (),
    /// A [`FuncRef`] doesn't own an `E`, but it is a function that can return an `E`.
    _error: fn() -> E,
}

/// Represents a WebAssembly [**`funcref`**].
///
/// The type parameter `E` is the type of values for any errors are returned as a result of calling
/// the function, and is namely used to report WebAssembly [`Trap`]s.
///
/// [**`funcref`**]: https://webassembly.github.io/spec/core/exec/runtime.html#values
pub struct FuncRef<'a, E: 'static> {
    func: Option<RawFuncRef>,
    _marker: core::marker::PhantomData<FuncRefPhantom<'a, E>>,
}

impl<E: 'static> Default for FuncRef<'_, E> {
    fn default() -> Self {
        Self::NULL
    }
}

impl<E: 'static> wasm2rs_rt_core::global::GlobalValue for FuncRef<'_, E> {
    const ZERO: Self = Self::NULL;
}

impl<E: 'static> FuncRef<'_, E> {
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
    ///
    /// Additionally, the [`RawFuncRefData`] may only contain references of the lifetime `'a`.
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

#[cfg(not(feature = "alloc"))]
#[inline(never)]
#[cold]
fn closure_requires_heap_allocation(layout: core::alloc::Layout) -> ! {
    enum Reason {
        Size(usize),
        Align(usize),
    }

    impl core::fmt::Display for Reason {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::Size(size) => write!(
                    f,
                    "size of {size} bytes exceeds {}",
                    core::mem::size_of::<RawFuncRefData>()
                ),
                Self::Align(size) => write!(
                    f,
                    "required alignment {size} exceeds {}",
                    core::mem::align_of::<RawFuncRefData>()
                ),
            }
        }
    }

    panic!(
        "closure requires a heap allocation: {}",
        if layout.size() > core::mem::size_of::<RawFuncRefData>() {
            Reason::Size(layout.size())
        } else {
            Reason::Align(layout.align())
        }
    )
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
        impl<'a, E: 'static + Trap<FuncRefCastError>> FuncRef<'a, E> {$(
            #[doc = "Calls the referenced function with "]
            #[doc = $description]
            #[doc = ".\n\nMultiple return values are represented by a tuple.\n\n"]
            #[doc = "# Errors\n\n"]
            #[doc = "A [`Trap`] occurs if the function reference is not of the correct type, or "]
            pub fn $call<$($param,)* R>(
                &self
                $(, $argument: $param)*,
                frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>,
            ) -> Result<R, E>
            where
                $($param: 'static,)*
                R: 'static,
            {
                match self.cast::<unsafe fn(&RawFuncRefData $(, $param)*) -> Result<R, E>>() {
                    Ok((data, func)) => {
                        // SAFETY: only `data` is passed to the `func`.
                        unsafe { func(data $(, $argument)*) }
                    }
                    Err(cast_failed) => Err(E::trap(cast_failed, frame)),
                }
            }

            #[doc = "Creates a new [`FuncRef`] used to invoke the given closure with"]
            #[doc = $description]
            #[doc = ".\n\nIf the closure is too large, a heap allocation is used to ensure that"]
            #[doc = "it fits into [`RawFuncRefData`].\n\n"]
            #[doc = "# Panics\n\n"]
            #[doc = "Panics if the `alloc` feature is not enabled when\n"]
            #[doc = "[`RawFuncRefData::can_store_inline::<C>()`] returns `false`.\n\n"]
            #[doc = "[`RawFuncRefData::can_store_inline::<C>()`]: RawFuncRefData::can_store_inline()\n"]
            pub fn $from_closure<$($param,)* R, C>(closure: C) -> Self
            where
                $($param: 'static,)*
                C: Clone + Fn($($param),*) -> Result<R, E> + 'a,
                R: 'static,
            {
                #[cfg(feature = "alloc")]
                use alloc::boxed::Box;

                trait Constants<'a, $($param,)* R, E>: Sized {
                    // These traits are automatically implemented by all (safe) function pointers.
                    type FuncPtr: Clone + Copy + Send + Sync + core::marker::Unpin
                        + core::panic::UnwindSafe + core::panic::RefUnwindSafe + 'static;

                    const SIGNATURE: FuncRefSignature;
                    const VTABLE: RawFuncRefVTable;

                    unsafe fn from_data(data: &RawFuncRefData) -> &Self {
                        if RawFuncRefData::can_store_inline::<Self>() {
                            // SAFETY: check above ensures the closure was actually stored inline.
                            return unsafe { data.as_ref_inline::<Self>() };
                        }

                        #[cfg(not(feature = "alloc"))]
                        unreachable!();

                        #[cfg(feature = "alloc")]
                        return {
                            // SAFETY: check above ensures the closure was actually stored behind a pointer.
                            let boxed_ptr: &Box<Self> = unsafe { data.as_by_ref::<Box<Self>>() };

                            // `*const Box<T>` and `*const T` should both allow reading a `T`, but
                            // just in case it is U.B., this gets the reference to `T` "properly".
                            boxed_ptr.as_ref()
                        };
                    }

                    fn into_data(closure: Self) -> RawFuncRefData {
                        match RawFuncRefData::try_from_inline(closure) {
                            Ok(data) => data,
                            #[cfg(not(feature = "alloc"))]
                            Err(_) => {
                                closure_requires_heap_allocation(core::alloc::Layout::new::<Self>())
                            }
                            #[cfg(feature = "alloc")]
                            Err(closure) => {
                                let boxed = Box::<Self>::new(closure);
                                RawFuncRefData::from_mut_ptr::<Self>(Box::into_raw(boxed))
                            },
                        }
                    }
                }

                impl<'a, $($param,)* R, C, E> Constants<'a, $($param,)* R, E> for C
                where
                    $($param: 'static,)*
                    R: 'static,
                    C: Clone + Fn($($param),*) -> Result<R, E> + 'a,
                    E: 'static,
                {
                    type FuncPtr = unsafe fn(&RawFuncRefData $(, $param)*) -> Result<R, E>;

                    const SIGNATURE: FuncRefSignature = FuncRefSignature::of::<Self::FuncPtr>();

                    const VTABLE: RawFuncRefVTable = {
                        let invoke: Self::FuncPtr = |data $(, $argument)*| {
                            // SAFETY: `data` refers to a valid `Self`.
                            let me = unsafe { Self::from_data(data) };
                            (me)($($argument),*)
                        };

                        let clone: unsafe fn(&RawFuncRefData) -> RawFuncRef = |data| {
                            // SAFETY: `data` refers to a valid `Self`.
                            let me = unsafe { Self::from_data(data) };
                            RawFuncRef::new(C::into_data(me.clone()), &C::VTABLE)
                        };

                        let drop: unsafe fn(RawFuncRefData);
                        if RawFuncRefData::can_store_inline::<C>() {
                            drop = |data| {
                                // SAFETY: check above ensures closure was stored inline.
                                // SAFETY: `inline` contains valid instance of `C`.
                                let _ = unsafe { data.read::<C>() };
                            };
                        } else {
                            #[cfg(feature = "alloc")]
                            unsafe fn drop_behind_box<C>(data: RawFuncRefData) {
                                // SAFETY: check above ensures a `*mut C` was stored.
                                let raw = unsafe { data.read::<*mut C>() };

                                // SAFETY: the pointer originates from a previous `Box::into_raw` call.
                                let _ = unsafe { Box::from_raw(raw) };
                            }

                            #[cfg(feature = "alloc")]
                            {
                                drop = drop_behind_box::<C>;
                            }

                            #[cfg(not(feature = "alloc"))]
                            unreachable!();
                        };

                        let debug: unsafe fn(&RawFuncRefData, &mut core::fmt::Formatter) -> core::fmt::Result = |data, f| {
                            f.debug_struct("Closure")
                                .field("type_name", &core::any::type_name::<C>())
                                .field("data", data)
                                .finish()
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

                // SAFETY: `VTABLE` is implemented correctly.
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

impl<E> Clone for FuncRef<'_, E> {
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

impl<E> core::fmt::Debug for FuncRef<'_, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(func) = &self.func {
            #[repr(transparent)]
            struct Inner<'a>(&'a RawFuncRef);

            impl core::fmt::Debug for Inner<'_> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    // SAFETY: ensured by implementor of `debug` in `RawFuncRef`.
                    unsafe { (self.0.vtable().debug)(self.0.data(), f) }
                }
            }

            f.debug_struct("FuncRef")
                .field("function", &Inner(func))
                .field("signature", func.vtable().signature)
                .field("vtable", &(func.vtable() as *const RawFuncRefVTable))
                .finish()
        } else {
            #[derive(Clone, Copy, Debug)]
            struct Null;

            f.debug_tuple("FuncRef").field(&Null).finish()
        }
    }
}

impl<E> PartialEq for FuncRef<'_, E> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.func, &other.func) {
            (None, None) => true,
            (Some(self_data), Some(other_data)) => {
                core::ptr::eq(self_data.vtable(), other_data.vtable())
                    && self_data.data().memcmp(other_data.data())
            }
            _ => false,
        }
    }
}

impl<E> Eq for FuncRef<'_, E> {}

impl<E> Drop for FuncRef<'_, E> {
    fn drop(&mut self) {
        if let Some(func) = core::mem::take(&mut self.func) {
            // SAFETY: the `func` won't be used after this point, so the data is "moved" out.
            // SAFETY: ensured by implementor.
            unsafe { (func.vtable().drop)(*func.data()) }
        }
    }
}

impl<E> wasm2rs_rt_core::table::TableElement for FuncRef<'_, E> {}

impl<E> wasm2rs_rt_core::table::NullableTableElement for FuncRef<'_, E> {
    const NULL: Self = <Self>::NULL;
}
