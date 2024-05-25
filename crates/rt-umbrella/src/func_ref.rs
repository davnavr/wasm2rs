//! Provides the implementation for [WebAssembly `funcref`s].
//!
//! [WebAssembly `funcref`s]: https://webassembly.github.io/spec/core/syntax/types.html#reference-types

pub use rt_func_ref::{
    FuncRef, FuncRefCastError, FuncRefSignature, RawFuncRef, RawFuncRefData, RawFuncRefVTable,
    SignatureMismatchError,
};

use crate::trap::Trap;

macro_rules! call_indirect {
    ($(
        fn $description:literal $call:ident ($($argument:ident: $param:ident),*) => $handler:ident;
    )*) => {$(
        #[allow(clippy::too_many_arguments)]
        #[doc = "This implements the [`call_indirect`] in the case where the target function"]
        #[doc = concat!("takes ", $description, ".\n\n")]
        #[doc = "For more information, see the documentation for the [`table::get()`] and the"]
        #[doc = concat!("[`FuncRef::", stringify!($handler), "()`] methods.\n\n")]
        #[doc = "[`call_indirect`]: "]
        #[doc = "https://webassembly.github.io/spec/core/syntax/instructions.html#control-instructions"]
        #[doc = "\n[`table::get()`]: crate::table::get()\n"]
        pub fn $call<'a, const TABLE: u32, $($param,)* R, T, E>(
            table: &T,
            idx: i32,
            $($argument: $param,)*
            frame: Option<&'static crate::trace::WasmFrame>,
        ) -> Result<R, E>
        where
            $($param: 'static,)*
            R: 'static,
            T: crate::table::Table<Element = FuncRef<'a, E>> + ?Sized,
            E: Trap<crate::table::AccessError> + Trap<FuncRefCastError> + 'static,
        {
            crate::table::get::<TABLE, T, E>(table, idx, frame)?.$handler($($argument,)* frame)
        }
    )*};
}

call_indirect! {
    fn "no arguments" call_indirect_0() => call_0;
    fn "one argument" call_indirect_1(a0: A0) => call_1;
    fn "two arguments" call_indirect_2(a0: A0, a1: A1) => call_2;
    fn "three arguments" call_indirect_3(a0: A0, a1: A1, a2: A2) => call_3;
    fn "four arguments" call_indirect_4(a0: A0, a1: A1, a2: A2, a3: A3) => call_4;
    fn "five arguments" call_indirect_5(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4) => call_5;
    fn "six arguments" call_indirect_6(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5) => call_6;
    fn "seven arguments" call_indirect_7(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6) => call_7;
    fn "eight arguments" call_indirect_8(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7) => call_8;
    fn "nine arguments" call_indirect_9(a0: A0, a1: A1, a2: A2, a3: A3, a4: A4, a5: A5, a6: A6, a7: A7, a8: A8) => call_9;
}
