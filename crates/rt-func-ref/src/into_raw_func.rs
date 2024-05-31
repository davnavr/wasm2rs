use crate::{raw, signature::HasFuncRefSignature};

#[cfg(feature = "alloc")]
use alloc::rc::Rc;

/// Trait used for converting closures into [`Raw`] function references.
///
/// # Safety
///
/// See the documentation for [`FuncRef::from_raw()`] for more information.
///
/// [`Raw`]: raw::Raw
/// [`FuncRef::from_raw()`]: crate::FuncRef::from_raw()
pub unsafe trait IntoRawFunc<'a, const ARG_COUNT: usize, Params, Results, Trap>: 'a {
    #[allow(missing_docs)]
    type FnPtr: HasFuncRefSignature;

    #[allow(missing_docs)]
    const VTABLE: &'static raw::VTable;

    #[allow(missing_docs)]
    fn into_raw_data(self) -> raw::Data;
}

macro_rules! define_into_raw_func {
    (($($argument:ident: $parameter:ident),*); $number:literal) => {
        // SAFETY: `VTABLE` implementation is correct.
        #[cfg(feature = "alloc")]
        #[allow(unused_parens)]
        unsafe impl<'a, F, $($parameter,)* R, E> IntoRawFunc<'a, $number, ($($parameter),*), R, E> for Rc<F>
        where
            F: Fn($($parameter),*) -> Result<R, E> + 'a,
            $($parameter: 'static,)*
            E: 'static,
            R: 'static,
        {
            type FnPtr = crate::signature_function_pointer!(($($parameter),*) -> Result<R, E>);

            const VTABLE: &'static raw::VTable = {
                let invoke: Self::FnPtr = |data $(, $argument)*| {
                    // SAFETY: `data` contains a `*const F`.
                    let me: &F = unsafe { data.as_by_ref::<F>() };
                    (me)($($argument),*)
                };

                let clone: unsafe fn(&raw::Data) -> raw::Raw = |data| {
                    // SAFETY: `data` contains a `*const F`.
                    let me: *const F = unsafe { data.read::<*const F>() };

                    // SAFETY: `me` originates from `Rc::into_raw`.
                    unsafe {
                        Self::increment_strong_count(me);
                    }

                    // The `data` contains the same `*const F`.
                    raw::Raw::new(*data, Self::VTABLE)
                };

                let drop: unsafe fn(raw::Data) = |data| {
                    // SAFETY: `data` contains a `*const F`.
                    let me: *const F = unsafe { data.read::<*const F>() };

                    // SAFETY: `me` originates from `Rc::into_raw`.
                    unsafe {
                        Self::from_raw(me);
                    }

                    // `Rc` is automatically dropped
                };

                let debug: unsafe fn(&raw::Data, &mut core::fmt::Formatter) -> _ = |data, f| {
                    // SAFETY: `data` contains a `*const F`.
                    let me: *const F = unsafe { data.read::<*const F>() };

                    // SAFETY: `me` originates from `Rc::into_raw`.
                    let this = unsafe { Self::from_raw(me) };

                    // Prevent the reference count from changing.
                    let this = core::mem::ManuallyDrop::new(this);

                    f.debug_struct("Rc")
                        .field("address", &Self::as_ptr(&this))
                        .field("strong_count", &Self::strong_count(&this))
                        .field("type_name", &core::any::type_name::<F>())
                        .finish()
                };

                // SAFETY: function pointers have the same size.
                let invoke = unsafe { core::mem::transmute::<_, raw::Invoke>(invoke) };

                &raw::VTable::new(
                    invoke,
                    Self::FnPtr::SIGNATURE,
                    clone,
                    drop,
                    debug,
                )
            };

            fn into_raw_data(self) -> raw::Data {
                raw::Data::from_ptr::<F>(Self::into_raw(self))
            }
        }

        // No cfg to determine if size of function pointer matches size of pointer?
        // Assumption true for most Rust programs anyway, could store raw::Invoke in field in raw::Data.
        // Makes IntoRawFunc for fn pointers a bit of a problem...
    };
}

crate::with_parameters!(define_into_raw_func);

#[cfg(feature = "alloc")]
impl<'a, E: 'static> crate::FuncRef<'a, E> {
    /// Creates a new [`FuncRef`] calling the given closure stored within an [`Rc`] smart pointer.
    ///
    /// [`FuncRef`]: crate::FuncRef
    pub fn from_rc<C, const ARG_COUNT: usize, Params, Results>(closure: Rc<C>) -> Self
    where
        Rc<C>: IntoRawFunc<'a, ARG_COUNT, Params, Results, E>,
    {
        let raw = raw::Raw::new(closure.into_raw_data(), Rc::<C>::VTABLE);

        // SAFETY: Genearted `IntoRawFunc` implementations are correct.
        unsafe { Self::from_raw(raw) }
    }
}
