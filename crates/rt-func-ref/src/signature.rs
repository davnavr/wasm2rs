/// Describes the argument and result types of a [`RawFuncRef`].
///
/// [`RawFuncRef`]: crate::RawFuncRef
#[derive(Clone, Copy)]
pub struct FuncRefSignature {
    type_id: fn() -> core::any::TypeId,
    name: fn() -> &'static str,
}

impl FuncRefSignature {
    /// Gets a [`FuncRefSignature`] corresponding to the given type.
    ///
    /// # Warning
    ///
    /// It is very easy to use this function correctly, which may result in unexpected [`SignatureMismatchError`]s if
    /// used to create a [`RawFuncRef`].
    ///
    /// The type parameter `F` **should** be a function pointer in the following form:
    ///
    /// ```ignore
    /// unsafe fn(&RawFuncRefData, A0, A1, ...) -> Result<(R0, R1, ...), E>
    /// ```
    ///
    /// where `A0, A1, ...` are the function arguments, and `(R0, R1, ...)` are the tuple of the function results.
    ///
    /// To prevent accidental usage with types that aren't [function pointer]s, `F` is constrained
    /// to implement traits that *all* [function pointer]s implement.
    ///
    /// [`SignatureMismatchError`]: crate::SignatureMismatchError
    /// [`RawFuncRef`]: crate::RawFuncRef
    /// [function pointer]: fn
    pub const fn of<F>() -> Self
    where
        F: Copy
            + Send
            + Sync
            + core::marker::Unpin
            + core::panic::UnwindSafe
            + core::panic::RefUnwindSafe
            + 'static,
    {
        Self {
            type_id: core::any::TypeId::of::<F>,
            name: core::any::type_name::<F>,
        }
    }

    /// Gets the [`TypeId`] corresponding to the underlying function pointer type.
    ///
    /// For what this [`TypeId`] actually identifies, refer to the documentation for [`FuncRefSignature::of()`].
    ///
    /// [`TypeId`]: core::any::TypeId
    pub fn type_id(&self) -> core::any::TypeId {
        (self.type_id)()
    }
}

/// Internal API used to associate [`FuncRefSignature`]s with function pointer types.
#[allow(missing_docs)]
pub trait HasFuncRefSignature {
    const SIGNATURE: &'static FuncRefSignature;
}

impl<F> HasFuncRefSignature for F
where
    F: Copy
        + Send
        + Sync
        + core::marker::Unpin
        + core::panic::UnwindSafe
        + core::panic::RefUnwindSafe
        + 'static,
{
    const SIGNATURE: &'static FuncRefSignature = &FuncRefSignature::of::<F>();
}

impl core::fmt::Debug for FuncRefSignature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&(self.name)(), f)
    }
}

impl PartialEq for FuncRefSignature {
    fn eq(&self, other: &Self) -> bool {
        (self.type_id)() == (other.type_id)()
    }
}

impl Eq for FuncRefSignature {}

impl core::hash::Hash for FuncRefSignature {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&self.type_id(), state)
    }
}
