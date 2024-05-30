use crate::{signature::HasFuncRefSignature, RawFuncRef, RawFuncRefData, RawFuncRefVTable};

/// Trait used for converting closures into [`RawFuncRef`]s.
///
/// # Safety
///
/// See the documentation for [`FuncRef::from_raw()`] for more information.
///
/// [`RawFuncRef`]: crate::RawFuncRef
/// [`FuncRef::from_raw()`]: crate::FuncRef::from_raw()
pub unsafe trait IntoRawFunc<'a, const ARG_COUNT: usize, Params, Results, Trap>: 'a {
    #[allow(missing_docs)]
    type FnPtr: HasFuncRefSignature;

    #[allow(missing_docs)]
    const VTABLE: &'static RawFuncRefVTable;

    #[allow(missing_docs)]
    fn into_raw_data(self) -> RawFuncRefData;
}

macro_rules! define_into_raw_func {
    (($($argument:ident: $parameter:ident),*); $number:literal) => {
        // SAFETY: `VTABLE` implementation is correct.
        #[cfg(feature = "alloc")]
        #[allow(unused_parens)]
        unsafe impl<'a, F, $($parameter,)* R, E> IntoRawFunc<'a, $number, ($($parameter),*), R, E> for alloc::rc::Rc<F>
        where
            F: Fn($($parameter),*) -> Result<R, E> + 'a,
            $($parameter: 'static,)*
            E: 'static,
            R: 'static,
        {
            type FnPtr = crate::signature_function_pointer!(($($parameter),*) -> Result<R, E>);

            const VTABLE: &'static RawFuncRefVTable = {
                let invoke: Self::FnPtr = |data $(, $argument)*| {
                    // SAFETY: `data` contains a `*const F`.
                    let me: &F = unsafe { data.as_by_ref::<F>() };
                    (me)($($argument),*)
                };

                let clone: unsafe fn(&RawFuncRefData) -> RawFuncRef = |data| {
                    // SAFETY: `data` contains a `*const F`.
                    let me: *const F = unsafe { data.read::<*const F>() };

                    // SAFETY: `me` originates from `Rc::into_raw`.
                    unsafe {
                        Self::increment_strong_count(me);
                    }

                    // The `data` contains the same `*const F`.
                    RawFuncRef::new(*data, Self::VTABLE)
                };

                let drop: unsafe fn(RawFuncRefData) = |data| {
                    // SAFETY: `data` contains a `*const F`.
                    let me: *const F = unsafe { data.read::<*const F>() };

                    // SAFETY: `me` originates from `Rc::into_raw`.
                    unsafe {
                        Self::from_raw(me);
                    }

                    // `Rc` is automatically dropped
                };

                let debug: unsafe fn(&RawFuncRefData, &mut core::fmt::Formatter) -> core::fmt::Result = |data, f| {
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
                let invoke = unsafe {
                    core::mem::transmute::<_, crate::RawFuncRefInvoke>(invoke)
                };

                &RawFuncRefVTable::new(
                    invoke,
                    Self::FnPtr::SIGNATURE,
                    clone,
                    drop,
                    debug,
                )
            };

            fn into_raw_data(self) -> RawFuncRefData {
                RawFuncRefData::from_ptr::<F>(Self::into_raw(self))
            }
        }

        // TODO: Impl for function pointers
    };
}

crate::with_parameters!(define_into_raw_func);
