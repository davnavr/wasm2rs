/// Allows inclusion of additional data in a closure used within a [`RawFuncRef`].
#[derive(Clone, Copy)]
pub union RawFuncRefData {
    /// Allows heap allocations or pointers to `'static` data.
    pub pointer: *const (),
    /// Allows storing data inline.
    pub inline: [core::mem::MaybeUninit<u8>; core::mem::size_of::<*mut ()>()],
}

impl RawFuncRefData {
    /// Creates [`inline`]d data with all bytes uninitialized.
    ///
    /// [`inline`]: RawFuncRefData::inline
    pub const UNINIT: Self = Self {
        inline: [core::mem::MaybeUninit::uninit(); core::mem::size_of::<*mut ()>()],
    };
}

impl core::fmt::Debug for RawFuncRefData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawFuncRefData").finish_non_exhaustive()
    }
}

/// A table of functions that specify the behavior of a [`RawFuncRef`].
#[derive(Clone, Copy, Debug)]
pub struct RawFuncRefVTable {
    pub(crate) invoke: *const (),
    pub(crate) signature: &'static crate::FuncRefSignature,
    pub(crate) clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef,
    pub(crate) drop: unsafe fn(data: RawFuncRefData),
    pub(crate) debug: unsafe fn(data: &RawFuncRefData) -> &dyn core::fmt::Debug,
}

impl RawFuncRefVTable {
    /// Creates a new virtual function table from the provided functions.
    ///
    /// For [`FuncRef`]s, there are no requirements for thread safety, as [`FuncRef`]s are meant to
    /// be used in translated single-threaded WebAssembly modules.
    ///
    /// # `invoke`
    ///
    /// This is actually a function pointer is called when the [`FuncRef`] itself is called. It
    /// must be of the same type that the `signature` corresponds to. In other words, if `invoke`
    /// is of type `F`, then the `signature` must originate from a call to
    /// [`FuncRefSignature::of::<F>()`]. It takes as its first parameter the [`&RawFuncRefData`],
    /// followed by the other parameters. It returns a [`Result`], with return values stored as a
    /// tuple in the `Ok` case, and any errors (namely, WebAssembly [`Trap`]s) in the `Err` case.
    ///
    /// # `signature`
    ///
    /// This value describes what function pointer `invoke` is.
    ///
    /// # `clone`
    ///
    /// This function is called when the [`FuncRef`] is [`clone`]d. The original [`FuncRef`] should not
    /// be dropped after this function is called.
    ///
    /// # `drop`
    ///
    /// This function is called when the [`FuncRef`] is [`drop`]ped.
    ///
    /// # `debug`
    ///
    /// This function is called when the [`FuncRef`] is formatted with the [`Debug`] trait. The
    /// original [`FuncRef`] should not be dropped after this function is called.
    ///
    /// [`FuncRef`]: crate::FuncRef
    /// [`FuncRefSignature::of::<F>()`]: crate::FuncRefSignature::of
    /// [`&RawFuncRefData`]: crate::RawFuncRefData
    /// [`Trap`]: wasm2rs_rt_core::trap::Trap
    /// [`clone`]: core::clone::Clone::clone
    /// [`drop`]: core::ops::Drop
    /// [`Debug`]: core::fmt::Debug
    pub const fn new(
        invoke: *const (),
        signature: &'static crate::FuncRefSignature,
        clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef,
        drop: unsafe fn(data: RawFuncRefData),
        debug: unsafe fn(data: &RawFuncRefData) -> &dyn core::fmt::Debug,
    ) -> Self {
        Self {
            invoke,
            signature,
            clone,
            drop,
            debug,
        }
    }
}

/// Provides an implementation for a [`FuncRef`].
///
/// [`FuncRef`]: crate::FuncRef
pub struct RawFuncRef {
    data: RawFuncRefData,
    vtable: &'static RawFuncRefVTable,
}

impl RawFuncRef {
    /// Creates a new [`RawFuncRef`] from the given `data` with the given `vtable`.
    pub const fn new(data: RawFuncRefData, vtable: &'static RawFuncRefVTable) -> Self {
        Self { data, vtable }
    }

    pub(crate) fn data(&self) -> &RawFuncRefData {
        &self.data
    }

    pub(crate) fn vtable(&self) -> &'static RawFuncRefVTable {
        self.vtable
    }
}

impl core::fmt::Debug for RawFuncRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawFuncRef").finish_non_exhaustive()
    }
}
