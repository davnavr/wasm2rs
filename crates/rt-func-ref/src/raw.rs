const INLINE_LEN: usize = core::mem::size_of::<*const ()>();

/// Allows inclusion of additional data in a closure used within a [`RawFuncRef`].
#[derive(Clone, Copy)]
pub union RawFuncRefData {
    /// Allows heap allocations or pointers to `'static` data.
    pub pointer: *const (),
    /// Allows storing data inline.
    pub inline: [u8; INLINE_LEN],
}

impl RawFuncRefData {
    const _ALIGN_CHECK: () = if core::mem::align_of::<Self>() < core::mem::align_of::<*const ()>() {
        panic!("alignment of RawFuncRefData should allow storing pointers or references")
    };

    /// Creates [`inline`]d data with all bytes set to zero.
    ///
    /// [`inline`]: RawFuncRefData::inline
    pub const ZERO: Self = Self {
        inline: [0u8; core::mem::size_of::<*mut ()>()],
    };

    /// Initializes the data with the given raw [`pointer`].
    ///
    /// [`pointer`]: RawFuncRefData::pointer
    pub const fn from_ptr<T>(pointer: *const T) -> Self {
        // Since all pointer bit patterns are valid, the `inline` bytes can be read.
        Self {
            pointer: pointer as *const (),
        }
    }

    /// Returns true if an instance of `T` can be stored [`inline`].
    ///
    /// [`inline`]: RawFuncRefData::inline
    pub const fn can_store_inline<T>() -> bool {
        core::mem::size_of::<T>() <= INLINE_LEN
            && core::mem::align_of::<T>() <= core::mem::align_of::<Self>()
    }

    /// Attempts to store the given `value` as [`inline`]d data.
    ///
    /// # Errors
    ///
    /// Returns the `value` if [`size_of::<T>()`] is larger than `size_of::<RawFuncRefData>`, or if
    /// [`align_of::<T>()`] is greater than `align_of::<RawFuncRefData>`.
    ///
    /// [`inline`]: RawFuncRefData::inline
    /// [`size_of::<T>()`]: core::mem::size_of()
    /// [`align_of::<T>()`]: core::mem::align_of()
    pub fn try_from_inline<T>(value: T) -> Result<Self, T> {
        if Self::can_store_inline::<T>() {
            let mut data = Self::ZERO;

            // SAFETY: check for size and alignment occurs above.
            unsafe {
                core::ptr::write(data.inline.as_mut_ptr() as *mut T, value);
            }

            Ok(data)
        } else {
            Err(value)
        }
    }

    /// Initializes the data with the given raw mutable [`pointer`].
    ///
    /// [`pointer`]: RawFuncRefData::pointer
    pub const fn from_mut_ptr<T>(pointer: *mut T) -> Self {
        Self::from_ptr(pointer as *const T)
    }

    /// Interprets the [`inline`] data as **containing** a [valid] instance of `T`, and returns a
    /// reference to it.
    ///
    /// If the data instead contains a [`pointer`] to an instance of `T`, use the [`as_by_ref()`]
    /// method instead.
    ///
    /// # Safety
    ///
    /// The [`inline`] data must actually contain a [valid], initialized instance of `T`.
    /// Additionally, as this creates a shared reference to the [`inline`] data, there must be no
    /// existing exclusive references to it.
    ///
    /// [`inline`]: RawFuncRefData::inline
    /// [valid]: core::ptr#safety
    /// [`pointer`]: RawFuncRefData::pointer
    /// [`as_by_ref()`]: RawFuncRefData::as_by_ref()
    pub unsafe fn as_ref_inline<T>(&self) -> &T {
        // SAFETY: caller ensures pointer refers to a valid, initialized instance of `T`.
        // SAFETY: the `inline` data lives as long as `self` does.
        unsafe { &*(&self.inline as *const [u8; INLINE_LEN] as *const T) }
    }

    /// Interprets the [`pointer`] as referring to a [valid] instance of `T`, and returns a
    /// reference to it.
    ///
    /// If the data instead contains an [`inline`] instance of `T`, use the [`as_ref_inline()`]
    /// method instead.
    ///
    /// # Safety
    ///
    /// The data must actually contain a [valid] **pointer** to a valid, initialized instance of
    /// `T`. Additionally, as this creates a shared reference to the [`inline`] data, there must be
    /// no existing exclusive references to it.
    ///
    /// [`pointer`]: RawFuncRefData::pointer
    /// [valid]: core::ptr#safety
    /// [`inline`]: RawFuncRefData::inline
    /// [`as_ref_inline()`]: RawFuncRefData::as_ref_inline()
    pub unsafe fn as_by_ref<T>(&self) -> &T {
        // SAFETY: caller ensures that the contained pointer is valid.
        // SAFETY: the `inline` data lives as long as `self` does.
        unsafe { &*(self.pointer as *const T) }
    }

    /// Reads a `T` out of the [`inline`] data.
    ///
    /// This method is similar to [`MaybeUninit::assume_init_read()`].
    ///
    /// # Safety
    ///
    /// Callers must ensure that the data actually contains a valid, initialized instance of `T`,
    /// and that it is no longer accessed after this method is called.
    ///
    /// See the documentation for [`ptr::read()`] for more information.
    ///
    /// [`inline`]: RawFuncRefData::inline
    /// [`MaybeUninit::assume_init_read()`]: core::mem::MaybeUninit::assume_init_read()
    /// [`ptr::read()`]: core::ptr::read()
    pub unsafe fn read<T>(&self) -> T {
        // SAFETY: caller ensures data contains a `T`.
        unsafe { core::ptr::read(&self.inline as *const [u8; INLINE_LEN] as *const T) }
    }

    pub(crate) fn memcmp(&self, other: &Self) -> bool {
        // SAFETY: all bits of `self` and `other` are initialized.
        // SAFETY: alignment doesn't matter here, since this just reads bytes.
        unsafe { self.inline == other.inline }
    }
}

impl core::fmt::Debug for RawFuncRefData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: All bit patterns for `pointer` are valid
        let pointer = unsafe { self.pointer };
        write!(f, "{pointer:0width$p}", width = INLINE_LEN * 2)
    }
}

/// A table of functions that specify the behavior of a [`RawFuncRef`].
#[derive(Clone, Copy, Debug)]
pub struct RawFuncRefVTable {
    pub(crate) invoke: *const (),
    pub(crate) signature: &'static crate::FuncRefSignature,
    pub(crate) clone: unsafe fn(data: &RawFuncRefData) -> RawFuncRef,
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
    /// [`clone`]: core::clone::Clone::clone
    /// [`drop`]: core::ops::Drop
    /// [`Debug`]: core::fmt::Debug
    pub const fn new(
        invoke: *const (),
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
