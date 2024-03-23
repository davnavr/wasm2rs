//! Runtime support for [references to functions].
//!
//! [references to functions]: https://webassembly.github.io/spec/core/syntax/types.html#reference-types

mod raw;

pub use raw::{RawFuncRef, RawFuncRefData, RawFuncRefVTable};

use crate::trap::Trap;

/// Error type used when a [`FuncRef`] did not have the correct signature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignatureMismatchError {
    expected: &'static str,
}

impl core::fmt::Display for SignatureMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "expected function reference signature {:?}",
            self.expected
        )
    }
}

/// Represents a WebAssembly [**`funcref`**].
///
/// [**`funcref`**]: https://webassembly.github.io/spec/core/exec/runtime.html#values
pub struct FuncRef {
    func: RawFuncRef,
}

impl FuncRef {
    /// Creates a new [`FuncRef`] from a [`RawFuncRef`].
    ///
    /// # Safety
    ///
    /// The provided [`RawFuncRef`] must meet the requirements specified in its documentation.
    pub unsafe fn from_raw(func: RawFuncRef) -> Self {
        Self { func }
    }

    fn debug(&self) -> &dyn core::fmt::Debug {
        // SAFETY: ensured by implementor of `debug` in `RawFuncRef`
        unsafe { (self.func.vtable().debug)(self.func.data()) }
    }

    /// Attempts to cast this reference to some exact type.
    ///
    /// # Errors
    ///
    /// A `Trap` occurs if the function reference is not of the correct type.
    fn cast<'f, F, E>(&'f self, trap: &E) -> Result<&'f F, E::Repr>
    where
        F: ?Sized + 'static,
        E: Trap + ?Sized,
        E::Repr: 'static,
    {
        let data: &'f _ = self.func.data();

        // SAFETY: ensured by caller.
        let cast_ptr = unsafe { (self.func.vtable().cast)(data, core::any::TypeId::of::<F>()) };

        match cast_ptr {
            None => Err(trap.trap(crate::trap::TrapCode::FuncRefSignatureMismatch(
                SignatureMismatchError {
                    expected: core::any::type_name::<F>(),
                },
            ))),
            Some(ptr) => Ok(if cfg!(debug_assertions) {
                // TODO: Darn, can't downcast to ?Sized
                match ptr.downcast_ref() {
                    Some(casted) => casted,
                    None => panic!(
                        "bad cast, expected {:?} but got {:?}",
                        core::any::type_name::<F>(),
                        self.debug()
                    ),
                }
            } else {
                // SAFETY: `cast` in `RawFuncRef` ensures the `ptr` is of the requested type.
                unsafe { todo!() }
            }),
        }
    }

    /// Performs a function call with no arguments.
    ///
    /// # Errors
    ///
    /// A `Trap` occurs if the function reference is not of the correct type.
    pub fn call_0<R, E>(&self, trap: &E) -> Result<R, E::Repr>
    where
        E: Trap + ?Sized,
        E::Repr: 'static,
    {
        let func = self.cast::<dyn Fn() -> Result<R, E::Repr>, E>(trap)?;
        func()
    }
}

impl core::fmt::Debug for FuncRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("FuncRef").field(self.debug()).finish()
    }
}

impl Drop for FuncRef {
    fn drop(&mut self) {
        // SAFETY: the `func` won't be used after this point, so the data is "moved" here.
        let data = *self.func.data();

        // SAFETY: ensured by implementor.
        unsafe { (self.func.vtable().drop)(data) }
    }
}
