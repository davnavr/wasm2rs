//! Implements the `null` function reference.

#[derive(Clone, Copy, Debug)]
struct NullFuncRef;

const VTABLE: crate::func_ref::RawFuncRefVTable =
    crate::func_ref::RawFuncRefVTable::new(|_, _| None, |_| INSTANCE, |_| (), |_| &NullFuncRef);

const INSTANCE: crate::func_ref::RawFuncRef = crate::func_ref::RawFuncRef::new(
    crate::func_ref::RawFuncRefData {
        pointer: core::ptr::null(),
    },
    &VTABLE,
);

impl crate::func_ref::FuncRef {
    /// Gets the [`null`] function reference.
    ///
    /// [`null`]: https://webassembly.github.io/spec/core/exec/runtime.html#values
    pub const NULL: Self = {
        // SAFETY: I said so.
        unsafe { Self::from_raw(INSTANCE) }
    };
}
