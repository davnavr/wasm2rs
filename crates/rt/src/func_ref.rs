//! Runtime support for [references to functions].
//!
//! [references to functions]: https://webassembly.github.io/spec/core/syntax/types.html#reference-types

mod raw;

pub use raw::{RawFuncRef, RawFuncRefData, RawFuncRefVTable};

use crate::trap::Trap;

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

    /// Attempts to cast this reference to some exact type.
    ///
    /// # Errors
    ///
    /// A `Trap` occurs if the function reference is not of the correct type.
    fn cast<'f, F, E>(&'f self, trap: &E) -> Result<&F, E::Repr>
    where
        F: ?Sized,
        E: Trap + ?Sized,
    {
        todo!()
    }

    /// Performs a function call with no arguments.
    ///
    /// # Errors
    ///
    /// A `Trap` occurs if the function reference is not of the correct type.
    pub fn call_0<R, E>(&self, trap: &E) -> Result<R, E::Repr>
    where
        E: Trap + ?Sized,
    {
        let func = self.cast::<dyn Fn() -> Result<R, E::Repr>, E>(trap)?;
        func()
    }
}

impl core::fmt::Debug for FuncRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: ensured by implementor.
        let data = unsafe { (self.func.vtable().debug)(self.func.data()) };

        f.debug_tuple("FuncRef").field(data).finish()
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
