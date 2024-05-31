const INLINE_LEN: usize = core::mem::size_of::<*const ()>();

/// Allows inclusion of additional data in a closure used within a [`Raw`] function reference.
///
/// [`Raw`]: crate::raw::Raw
#[derive(Clone, Copy)]
pub union Data {
    /// Allows storing heap allocations or pointers to `'static` data.
    pub pointer: *const (),
    /// Allows storing data inline. This can be used to store any pointer-sized value with an
    /// alignment less than or equal to the alignment requirement for a pointer.
    ///
    /// Note that when a struct with padding is stored inline, reading those bytes corresponding to
    /// padding is **undefined behavior**. See the Rust [Unsafe Code Guidelines Reference] for more
    /// information.
    ///
    /// [Unsafe Code Guidelines Reference]: https://rust-lang.github.io/unsafe-code-guidelines/glossary.html#padding
    pub inline: core::mem::MaybeUninit<[u8; INLINE_LEN]>,
}

impl Data {
    /// Initializes the data with the given raw [`pointer`].
    ///
    /// [`pointer`]: Data::pointer
    pub const fn from_ptr<T>(pointer: *const T) -> Self {
        // Since all pointer bit patterns are valid, the `inline` bytes can be read.
        Self {
            pointer: pointer as *const (),
        }
    }

    /// Initializes the data with the given raw mutable [`pointer`].
    ///
    /// [`pointer`]: Data::pointer
    pub const fn from_mut_ptr<T>(pointer: *mut T) -> Self {
        Self::from_ptr(pointer as *const T)
    }

    /// Returns `true` if an instance of `T` can be stored [`inline`].
    ///
    /// See the documentation for the [`inline`] field and the [`try_from_inline()`] method for
    /// more information.
    ///
    /// [`inline`]: Data::inline
    /// [`try_from_inline()`]: Data::try_from_inline()
    pub const fn can_store_inline<T>() -> bool {
        core::mem::size_of::<T>() <= INLINE_LEN
            && core::mem::align_of::<T>() <= core::mem::align_of::<Self>()
    }

    /// Attempts to store the given `value` as [`inline`]d data.
    ///
    /// # Errors
    ///
    /// Returns the `value` if [`size_of::<T>()`] is larger than `size_of::<raw::Data>`, or if
    /// [`align_of::<T>()`] is greater than `align_of::<raw::Data>`.
    ///
    /// [`inline`]: Data::inline
    /// [`size_of::<T>()`]: core::mem::size_of()
    /// [`align_of::<T>()`]: core::mem::align_of()
    pub fn try_from_inline<T>(value: T) -> Result<Self, T> {
        if Self::can_store_inline::<T>() {
            let mut data = Self {
                inline: core::mem::MaybeUninit::uninit(),
            };

            // SAFETY: check for size and alignment occurs above.
            unsafe {
                core::ptr::write(data.inline.as_mut_ptr() as *mut T, value);
            }

            Ok(data)
        } else {
            Err(value)
        }
    }

    fn assert_can_store_inline<T>() {
        assert!(
            Self::can_store_inline::<T>(),
            "reading would result in undefined behavior, {} requires {} bytes aligned to {} bytes",
            core::any::type_name::<T>(),
            core::mem::size_of::<T>(),
            core::mem::align_of::<T>(),
        );
    }

    /// Interprets the [`inline`] data as **containing** a [valid] instance of `T`, and returns a
    /// reference to it.
    ///
    /// If the [`raw::Data`] instead contains a [`pointer`] to an instance of `T`, use the
    /// [`as_by_ref()`] method instead.
    ///
    /// # Panics
    ///
    /// Panics if [`can_store_inline::<T>()`] returns `false`, as constructing a shared reference
    /// to `T` would violate the [*dereferenceable* requirement] leading to undefined behavior.
    ///
    /// # Safety
    ///
    /// The [`inline`] data must actually contain a [valid], initialized instance of `T`.
    /// Additionally, as this creates a shared reference to the [`inline`] data, there must be no
    /// existing exclusive references to it.
    ///
    /// [`inline`]: Data::inline
    /// [valid]: core::ptr#safety
    /// [`raw::Data`]: Data
    /// [`pointer`]: Data::pointer
    /// [`as_by_ref()`]: Data::as_by_ref()
    /// [`can_store_inline::<T>()`]: Data::can_store_inline()
    /// [*dereferenceable* requirement]: core::ptr#safety
    pub unsafe fn as_ref_inline<T>(&self) -> &T {
        Self::assert_can_store_inline::<T>();

        // SAFETY: caller ensures pointer refers to a valid, initialized instance of `T`.
        // SAFETY: the `inline` data lives as long as `self` does.
        unsafe { &*(self.inline.as_ptr() as *const T) }
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
    /// [`pointer`]: Data::pointer
    /// [valid]: core::ptr#safety
    /// [`inline`]: Data::inline
    /// [`as_ref_inline()`]: Data::as_ref_inline()
    pub unsafe fn as_by_ref<T>(&self) -> &T {
        // SAFETY: caller ensures that the contained pointer is valid.
        // SAFETY: the `inline` data lives as long as `self` does.
        unsafe { &*(self.pointer as *const T) }
    }

    /// Reads a `T` out of the [`inline`] data.
    ///
    /// This method is similar to [`MaybeUninit::assume_init_read()`].
    ///
    /// # Panics
    ///
    /// Panics if [`can_store_inline::<T>()`] returns `false`, as reading the `T` in this case
    /// would violate the [*dereferenceable* requirement] for [valid] pointers leading to
    /// undefined behavior.
    ///
    /// # Safety
    ///
    /// Callers must ensure that the data actually contains a [valid], initialized instance of `T`.
    ///
    /// See the documentation for [`ptr::read()`] for more information.
    ///
    /// [`inline`]: Data::inline
    /// [`MaybeUninit::assume_init_read()`]: core::mem::MaybeUninit::assume_init_read()
    /// [`can_store_inline::<T>()`]: Data::can_store_inline()
    /// [*dereferenceable* requirement]: core::ptr#safety
    /// [valid]: core::ptr#safety
    /// [`ptr::read()`]: core::ptr::read()
    pub unsafe fn read<T>(&self) -> T {
        Self::assert_can_store_inline::<T>();

        // SAFETY: caller ensures data contains a valid `T`.
        unsafe { core::ptr::read(self.inline.as_ptr() as *const T) }
    }
}

impl core::fmt::Debug for Data {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Can't read bytes, might be `uninit`.
        f.debug_struct("Data").finish_non_exhaustive()
    }
}
