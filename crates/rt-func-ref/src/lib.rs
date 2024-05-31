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

mod error;
mod from_closure;
mod into_raw_func;
mod invoke;
mod raw;
mod signature;

pub use error::{FuncRefCastError, SignatureMismatchError};
pub use into_raw_func::IntoRawFunc;
pub use raw::{RawFuncRef, RawFuncRefData, RawFuncRefInvoke, RawFuncRefVTable};
pub use signature::FuncRefSignature;

/// Internal API used to generate code for [`FuncRef`]s with differing parameter types.
///
/// This is a workaround for a lack of generic argument types for the [`Fn`] trait in stable Rust.
#[macro_export]
#[doc(hidden)]
macro_rules! with_parameters {
    ($macro:ident) => {
        $macro![(); 0];
        $macro![(a0: A0); 1];
        $macro![(a0: A0, a1: A1); 2];
        $macro![(a0: A0, a1: A1, a2: A2); 3];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3); 4];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4); 5];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5); 6];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6); 7];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7); 8];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8); 9];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9); 10];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10); 11];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11); 12];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12); 13];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12, a13: A13); 14];
        $macro![(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8, a9: A9, a10: A10, a11: A11, a12: A12, a13: A13, a14: A14); 15];
    };
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
/// [`Trap`]: wasm2rs_rt_core::trap::Trap
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
    /// Additionally, the [`RawFuncRefData`] may only reference data lasting for the lifetime `'a`.
    pub const unsafe fn from_raw(func_ref: RawFuncRef) -> Self {
        Self {
            func: Some(func_ref),
            _marker: core::marker::PhantomData,
        }
    }

    /// Checks that this [`FuncRef`] has the signature `C`. More specifically, it checks that its
    /// `invoke` function pointer is ABI compatible with the function pointer type `C`.
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
    /// # Panics
    ///
    /// Panics if `C` does not have the same size as function pointers.
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
        C: signature::HasFuncRefSignature,
    {
        let expected: &FuncRefSignature = <C as signature::HasFuncRefSignature>::SIGNATURE;
        match &self.func {
            Some(func) if expected != func.vtable.signature => Err(
                FuncRefCastError::SignatureMismatch(SignatureMismatchError {
                    expected,
                    actual: func.vtable.signature,
                }),
            ),
            Some(func) => {
                assert_eq!(
                    core::mem::size_of::<C>(),
                    core::mem::size_of::<RawFuncRefInvoke>(),
                    "expected {} to be a function pointer type",
                    core::any::type_name::<C>(),
                );

                // SAFETY: check above ensures sizes are the same.
                // SAFETY: `vtable` implementor ensures `invoke` is ABI compatible with `C`.
                let casted = unsafe {
                    core::mem::transmute_copy::<RawFuncRefInvoke, C>(&func.vtable.invoke)
                };

                Ok((&func.data, casted))
            }
            None => Err(FuncRefCastError::Null { expected }),
        }
    }
}

impl<E> Clone for FuncRef<'_, E> {
    fn clone(&self) -> Self {
        match &self.func {
            None => Self::NULL,
            Some(func) => {
                // SAFETY: ensured by `vtable` implementor.
                unsafe { Self::from_raw((func.vtable.clone)(&func.data)) }
            }
        }
    }

    // fn clone_from(&mut self, source: &Self) {}
}

fn debug_fmt_null(f: &mut core::fmt::Formatter) -> core::fmt::Result {
    #[derive(Clone, Copy, Debug)]
    struct Null;

    f.debug_tuple("FuncRef").field(&Null).finish()
}

impl<E> core::fmt::Debug for FuncRef<'_, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(func) = &self.func {
            #[repr(transparent)]
            struct Inner<'a>(&'a RawFuncRef);

            impl core::fmt::Debug for Inner<'_> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    // SAFETY: ensured by implementor of `debug` in `RawFuncRef`.
                    unsafe { (self.0.vtable.debug)(&self.0.data, f) }
                }
            }

            f.debug_struct("FuncRef")
                .field("function", &Inner(func))
                .field("signature", func.vtable.signature)
                .field("vtable", &(func.vtable as *const RawFuncRefVTable))
                .finish()
        } else {
            debug_fmt_null(f)
        }
    }
}

// WebAssembly GC proposal does **not** add equality for `funcref`.

/* impl<E> PartialEq for FuncRef<'_, E> {
    /// Compares two [`FuncRef`]s for equality.
    ///
    /// This implements the [WebAssembly `ref.is_null` and `ref.eq`] instruction.
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

impl<E> Eq for FuncRef<'_, E> {} */

impl<E> Drop for FuncRef<'_, E> {
    fn drop(&mut self) {
        if let Some(func) = core::mem::take(&mut self.func) {
            // SAFETY: the `func` won't be used after this point, so the data is "moved" out.
            // SAFETY: ensured by `vtable` implementor.
            unsafe { (func.vtable.drop)(func.data) }
        }
    }
}

impl<E> wasm2rs_rt_core::table::TableElement for FuncRef<'_, E> {}

impl<E> wasm2rs_rt_core::table::NullableTableElement for FuncRef<'_, E> {
    const NULL: Self = <Self>::NULL;

    fn is_null(&self) -> bool {
        <Self>::is_null(self)
    }
}
