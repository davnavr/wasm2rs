use crate::{signature::HasFuncRefSignature, RawFuncRef, RawFuncRefData};

macro_rules! define_from_closure {
    (($($argument:ident: $parameter:ident),*); $number:literal) => {
        paste::paste! {
            /// Creates a new [`FuncRef`] used to invoke the given closure.
            ///
            /// If the closure is too large, the closure is moved into an [`Rc`] heap allocation to
            /// ensure that it fits into [`RawFuncRefData`]. See the documentation for the
            /// [`RawFuncRefData::can_store_inline()`] method for more information.
            ///
            /// # Interior Mutability
            ///
            /// If this closure is stored inline, then it is [`clone`]d whenever the [`FuncRef`] itself is
            /// [`clone`]d. This means closures capturing data with interior mutability may not observe
            /// changes made by either the original [`FuncRef`] or the clone. To avoid this, wrap
            /// the closure in an [`Rc`], or only access immutable data within the closure.
            ///
            /// # Panics
            ///
            /// Panics if the `alloc` feature is not enabled when
            /// [`RawFuncRefData::can_store_inline::<C>()`] returns `false`.
            ///
            /// [`FuncRef`]: crate::FuncRef
            /// [`Rc`]: alloc::rc::Rc
            /// [`clone`]: Clone::clone()
            pub fn [<from_closure_ $number>]<$($parameter,)* R, C>(closure: C) -> Self
            where
                $($parameter: 'static,)*
                C: Clone + Fn($($parameter),*) -> Result<R, E> + 'a,
                R: 'static,
            {
                if !RawFuncRefData::can_store_inline::<C>() {
                    #[cfg(not(feature = "alloc"))]
                    panic!(
                        "could not store closure inline, layout requires {} bytes aligned to {} bytes",
                        core::mem::size_of::<C>(),
                        core::mem::align_of::<C>()
                    );

                    #[cfg(feature = "alloc")]
                    return {
                        use crate::IntoRawFunc;
                        use alloc::rc::Rc;

                        let closure = Rc::new(closure);
                        let data = IntoRawFunc::<'a, $number, _, R, E>::into_raw_data(closure);
                        let vtable = <Rc::<C> as IntoRawFunc::<'a, $number, _, R, E>>::VTABLE;

                        // SAFETY: implementation of VTABLE for `Rc` is correct.
                        unsafe { Self::from_raw(RawFuncRef::new(data, vtable)) }
                    };
                }

                trait InlineClosure<'a, $($parameter,)* R, E> {
                    type FnPtr: HasFuncRefSignature;

                    const VTABLE: &'static crate::RawFuncRefVTable;

                    unsafe fn into_raw_data(self) -> RawFuncRefData;
                }

                // SAFETY: `C` always guaranteed to be stored inline.
                impl<'a, $($parameter,)* R, E, C> InlineClosure<'a, $($parameter,)* R, E> for C
                where
                    $($parameter: 'static,)*
                    C: Fn($($parameter),*) -> Result<R, E> + 'a,
                    R: 'static,
                    E: 'static,
                {
                    type FnPtr = crate::signature_function_pointer!(($($parameter),*) -> Result<R, E>);

                    const VTABLE: &'static crate::RawFuncRefVTable = {
                        let invoke: Self::FnPtr = |data $(, $argument)*| {
                            // SAFETY: `data` contains `C`.
                            let me: &C = unsafe { data.as_ref_inline::<C>() };
                            (me)($($argument),*)
                        };

                        let clone: unsafe fn(&RawFuncRefData) -> RawFuncRef = |data| {
                            // SAFETY: `data` contains `C`.
                            let me: &C = unsafe { data.as_ref_inline::<C>() };
                            let clone = me.clone();

                            // SAFETY: `C` is known to be stored inline.
                            let data = unsafe { clone.into_raw_data() };

                            RawFuncRef::new(data, Self::VTABLE)
                        };

                        let drop: unsafe fn(RawFuncRefData) = |data| {
                            // SAFETY: `data` contains `C`.
                            unsafe { data.read::<C>() };

                            // `C` is automatically dropped
                        };

                        let debug: unsafe fn(&RawFuncRefData, &mut core::fmt::Formatter) -> core::fmt::Result = |_, f| {
                            write!(f, "{}", core::any::type_name::<C>())
                        };

                        // SAFETY: function pointers have the same size.
                        let invoke = unsafe {
                            core::mem::transmute::<_, crate::RawFuncRefInvoke>(invoke)
                        };

                        &crate::RawFuncRefVTable::new(
                            invoke,
                            Self::FnPtr::SIGNATURE,
                            clone,
                            drop,
                            debug,
                        )
                    };

                    unsafe fn into_raw_data(self) -> RawFuncRefData {
                        let data = RawFuncRefData::try_from_inline::<C>(self);

                        // SAFETY: Caller ensures `C` can be stored inline.
                        unsafe { data.unwrap_unchecked() }
                    }
                }

                // SAFETY: check for `can_store_inline::<C>()` above ensures this always succeeds.
                let data = unsafe {
                    <C as InlineClosure<'a, $($parameter,)* R, E>>::into_raw_data(closure)
                };

                // SAFETY: implementation of VTABLE for `<C as InlineClosure>` is correct.
                unsafe {
                    Self::from_raw(RawFuncRef::new(data, <C as InlineClosure<'a, $($parameter,)* R, E>>::VTABLE))
                }
            }
        }
    };
}

#[allow(clippy::too_many_arguments)]
impl<'a, E: 'static> crate::FuncRef<'a, E> {
    crate::with_parameters!(define_from_closure);
}
