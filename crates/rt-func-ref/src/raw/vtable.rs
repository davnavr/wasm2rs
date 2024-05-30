use crate::{RawFuncRef, RawFuncRefData};

/// Placeholder function pointer type which must be casted to the correct function pointer type
/// before calling.
///
/// See the documentation for [`RawFuncRefVTable::new()`] for more information.
///
/// This is not `*const ()` for maximum portability, as casts from a raw pointer to a function
/// pointer are not available on platforms where the sizes of function pointers and pointers to
/// data differ.
pub type RawFuncRefInvoke = unsafe fn(core::convert::Infallible) -> !;

/// A table of functions that specify the behavior of a [`RawFuncRef`].
#[derive(Clone, Copy, Debug)]
pub struct RawFuncRefVTable {
    pub(crate) invoke: RawFuncRefInvoke,
    pub(crate) signature: &'static crate::FuncRefSignature,
    pub(crate) clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef,
    // WebAssembly GC proposal does **not** add equality for `funcref`.
    // pub(crate) eq: unsafe fn(&RawFuncRefData, &RawFuncRefData) -> bool,
    pub(crate) drop: unsafe fn(data: RawFuncRefData),
    pub(crate) debug:
        unsafe fn(data: &RawFuncRefData, f: *mut core::fmt::Formatter) -> core::fmt::Result,
}

impl RawFuncRefVTable {
    /// Creates a new virtual function table from the provided functions.
    ///
    /// For [`FuncRef`]s, there are no requirements for thread safety, as [`FuncRef`]s are meant to
    /// be used in translated single-threaded WebAssembly modules.
    ///
    /// # `invoke`
    ///
    /// This is the function pointer that is casted then called when the [`FuncRef`] itself is
    /// called. It must be of the same type that the `signature` corresponds to. In other words, if
    /// `invoke` actually is a function pointer of type `F`, then the `signature` must originate
    /// from a call to [`FuncRefSignature::of::<F>()`]. It takes as its first parameter the
    /// [`&RawFuncRefData`], followed by the other parameters. It returns a [`Result`], with return
    /// values stored as a tuple in the `Ok` case, and any errors (namely, WebAssembly [`Trap`]s)
    /// in the `Err` case.
    ///
    /// # `signature`
    ///
    /// This value describes what function pointer `invoke` is.
    ///
    /// # `clone`
    ///
    /// This function is called when the [`FuncRef`] is [`clone`]d.
    ///
    /// # `drop`
    ///
    /// This function is called when the [`FuncRef`] is [`drop`]ped. This function is responsible
    /// for dropping the contents of the [`RawFuncRefData`].
    ///
    /// # `debug`
    ///
    /// This function is called when the [`FuncRef`] is formatted with the [`Debug`] trait.
    ///
    /// [`FuncRef`]: crate::FuncRef
    /// [`FuncRefSignature::of::<F>()`]: crate::FuncRefSignature::of
    /// [`&RawFuncRefData`]: crate::RawFuncRefData
    /// [`Trap`]: wasm2rs_rt_core::trap::Trap
    /// [`clone`]: core::clone::Clone::clone()
    /// [`eq()`]: core::cmp::PartialEq::eq()
    /// [`drop`]: core::ops::Drop
    /// [`Debug`]: core::fmt::Debug
    pub const fn new(
        invoke: RawFuncRefInvoke,
        signature: &'static crate::FuncRefSignature,
        clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef,
        drop: unsafe fn(data: RawFuncRefData),
        debug: unsafe fn(data: &RawFuncRefData, f: &mut core::fmt::Formatter) -> core::fmt::Result,
    ) -> Self {
        Self {
            invoke,
            signature,
            clone,
            drop,
            // Can't store `*mut core::fmt::Formatter` due to `const` requirements.
            // SAFETY: `*mut Formatter` and `&mut Formatter` are ABI compatible.
            debug: unsafe {
                core::mem::transmute::<
                    unsafe fn(&RawFuncRefData, &mut core::fmt::Formatter) -> core::fmt::Result,
                    unsafe fn(&RawFuncRefData, *mut core::fmt::Formatter) -> core::fmt::Result,
                >(debug)
            },
        }
    }

    // TODO: Add an optional clone_invoke method.
}
