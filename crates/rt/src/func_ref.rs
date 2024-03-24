//! Runtime support for [references to functions].
//!
//! [references to functions]: https://webassembly.github.io/spec/core/syntax/types.html#reference-types

mod call_null;
mod raw;

pub use raw::{RawFuncRef, RawFuncRefData, RawFuncRefVTable};

use crate::trap::Trap;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct SignatureMismatchErrorData {
    expected_name: fn() -> &'static str,
    // actual_name: fn() -> &'static str,
}

/// Error type used when a [`FuncRef`] did not have the correct signature.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct SignatureMismatchError {
    data: &'static SignatureMismatchErrorData,
}

impl core::fmt::Debug for SignatureMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SignatureMismatchError")
            .field("expected", &(self.data.expected_name)())
            // .field("actual", &(self.data.actual_name)())
            .finish()
    }
}

impl core::fmt::Display for SignatureMismatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // write!(f, "signature mismatch: expected {:?}, but got {:?}", (self.data.expected_name)(), (self.data.actual_name)())
        write!(
            f,
            "signature mismatch: expected {:?}",
            (self.data.expected_name)()
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
    pub const unsafe fn from_raw(func: RawFuncRef) -> Self {
        Self { func }
    }

    fn debug(&self) -> &dyn core::fmt::Debug {
        // SAFETY: ensured by implementor of `debug` in `RawFuncRef`
        unsafe { (self.func.vtable().debug)(self.func.data()) }
    }

    /// Attempts to cast this reference to some exact type.
    ///
    /// This is an implementation detail used to support generated code. Prefer calling the
    /// specialized `call_` functions instead, such as [`call_0()`]. [`call_1()`], etc.
    ///
    /// # Usage
    ///
    /// Generated code and the `call_` functions call `cast` to obtain a function pointer of the
    /// form `unsafe fn(&RawFuncRefData, A0, A1, ..., &E) -> Result<(R0, R1, ...), E::Repr>`, where
    /// `A0, A1, ...` are the function arguments, and `(R0, R1, ...)` are the tuple of the function
    /// results.
    ///
    /// ## Safety
    ///
    /// The function pointer that is produced is safe to call only with the [`RawFuncRefData`]
    /// corresponding to `self`.
    ///
    /// # Errors
    ///
    /// A `Trap` occurs if the function reference is not of the correct type.
    pub fn cast<'f, C, E>(&'f self, trap: &E) -> Result<C, E::Repr>
    where
        C: Copy + 'static,
        E: Trap + 'static + ?Sized,
        E::Repr: 'static,
    {
        let data: &'f _ = self.func.data();

        // SAFETY: ensured by implementor of `convert` from `RawFuncRefVTable`.
        let convert_result =
            unsafe { (self.func.vtable().convert)(data, core::any::TypeId::of::<C>()) };

        match convert_result {
            None => Err(trap.trap(crate::trap::TrapCode::FuncRefSignatureMismatch(
                SignatureMismatchError {
                    data: &SignatureMismatchErrorData {
                        expected_name: core::any::type_name::<C>,
                        // actual_name: ,
                    },
                },
            ))),
            Some(ptr) => {
                if core::mem::size_of::<C>() == core::mem::size_of_val(&ptr) {
                    // SAFETY: check above ensures sizes are the same.
                    // SAFETY: implementor of `convert` from `RawFuncRefVTable` must return `C`.
                    Ok(unsafe { core::mem::transmute_copy(&ptr) })
                } else {
                    panic!(
                        "size mismatch when obtaining {:?} from {:?}",
                        core::any::type_name::<C>(),
                        self.debug()
                    );
                }
            }
        }
    }

    /// Obtains a closure to perform a function call with no arguments.
    ///
    /// # Errors
    ///
    /// A `Trap` occurs if the function reference is not of the correct type.
    pub fn call_0<R, E>(&self, trap: &E) -> Result<R, E::Repr>
    where
        R: 'static,
        E: Trap + 'static + ?Sized,
        E::Repr: 'static,
    {
        let func = self.cast::<unsafe fn(&RawFuncRefData, &E) -> Result<R, E::Repr>, E>(trap)?;

        // SAFETY: the corresponding `self.func.data` is being passed as the argument.
        unsafe { func(self.func.data(), trap) }
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
