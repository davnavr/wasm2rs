/// Allows inclusion of additional data in a closure used within a [`RawFuncRef`].
#[derive(Clone, Copy)]
pub union RawFuncRefData {
    /// Allows heap allocations or pointers to `'static` data.
    pub pointer: *const (),
    /// Allows storing data inline.
    pub inline: [core::mem::MaybeUninit<u8>; core::mem::size_of::<*mut ()>()],
}

impl core::fmt::Debug for RawFuncRefData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawFuncRefData").finish_non_exhaustive()
    }
}

/// A table of functions that specify the behavior of a [`RawFuncRef`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawFuncRefVTable {
    pub(in crate::func_ref) convert:
        unsafe fn(data: &RawFuncRefData, id: core::any::TypeId) -> Option<*const ()>,
    // What return type to use?
    //pub(in crate::func_ref) call_fallback: unsafe fn (data: &RawFuncRefData, arguments: &[&dyn core::any::Any]),
    pub(in crate::func_ref) clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef,
    pub(in crate::func_ref) drop: unsafe fn(data: RawFuncRefData),
    pub(in crate::func_ref) debug: unsafe fn(data: &RawFuncRefData) -> &dyn core::fmt::Debug,
}

impl RawFuncRefVTable {
    // TODO: This should be public and needs documentation
    pub(crate) const fn new(
        convert: unsafe fn(data: &RawFuncRefData, id: core::any::TypeId) -> Option<*const ()>,
        clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef,
        drop: unsafe fn(data: RawFuncRefData),
        debug: unsafe fn(data: &RawFuncRefData) -> &dyn core::fmt::Debug,
    ) -> Self {
        Self {
            convert,
            clone,
            drop,
            debug,
        }
    }
}

/// Provides an implementation for a [`FuncRef`].
///
/// [`FuncRef`]: crate::func_ref::FuncRef
pub struct RawFuncRef {
    data: RawFuncRefData,
    vtable: &'static RawFuncRefVTable,
}

impl RawFuncRef {
    /// Creates a new [`RawFuncRef`] from the given `data` with the given `vtable`.
    pub const fn new(data: RawFuncRefData, vtable: &'static RawFuncRefVTable) -> Self {
        Self { data, vtable }
    }

    pub(in crate::func_ref) fn data(&self) -> &RawFuncRefData {
        &self.data
    }

    pub(in crate::func_ref) fn vtable(&self) -> &'static RawFuncRefVTable {
        self.vtable
    }
}

impl core::fmt::Debug for RawFuncRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawFuncRef").finish_non_exhaustive()
    }
}
