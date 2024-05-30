use crate::RawFuncRefData;
use wasm2rs_rt_core::{trace::WasmFrame, trap::Trap};

macro_rules! define_call {
    (($($argument:ident: $parameter:ident),*); $number:literal) => {
        paste::paste! {
            /// Calls the function.
            ///
            /// Multiple return values are represented by a tuple.
            ///
            /// # Errors
            ///
            /// A [`Trap`] occurs if the function reference is not of the correct type, or if the
            /// function reference was [`NULL`].
            ///
            /// [`NULL`]: crate::FuncRef::NULL
            pub fn [<call_ $number>]<$($parameter,)* R>(
                &self
                $(, $argument: $parameter)*,
                frame: Option<&'static WasmFrame>,
            ) -> Result<R, E>
            where
                $($parameter: 'static,)*
                R: 'static,
            {
                match self.cast::<crate::signature_function_pointer!(($($parameter),*) -> Result<R, E>)>() {
                    Ok((data, func)) => {
                        // SAFETY: ensured by implementor of `vtable.invoke`.
                        unsafe { func(data $(, $argument)*) }
                    }
                    Err(cast_failed) => Err(E::trap(cast_failed, frame)),
                }
            }
        }
    };
}

/// Defines the methods used to invoke the function.
#[allow(clippy::too_many_arguments)]
impl<'a, E: 'static + Trap<crate::FuncRefCastError>> crate::FuncRef<'a, E> {
    crate::with_parameters!(define_call);
}
