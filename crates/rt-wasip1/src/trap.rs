use crate::api;
use wasm2rs_rt_core::trap;

/// The set of [`Trap`]s that can occur when executing a WebAssembly module utilizing the WASI
/// [`API`](api::Api)s.
#[derive(Clone, Copy, Debug)]
#[allow(clippy::exhaustive_enums)]
pub enum Trap<E: trap::TrapInfo> {
    /// Aborts execution due to a [`Trap`] caused by some instruction.
    ///
    /// [`Trap`]: trap::Trap
    Abort(E),
    /// Aborts execution after [`proc_exit`](api::Api::proc_exit()) was called.
    ProcExit(api::ExitCode),
    /// Aborts execution due to a call to [`proc_raise`](api::Api::proc_raise()).
    ProcRaise(api::Signal),
}

impl<E: trap::TrapInfo> wasm2rs_rt_core::trace::Trace for Trap<E> {
    fn push_wasm_frame(self, frame: &'static wasm2rs_rt_core::trace::WasmFrame) -> Self {
        if let Self::Abort(trap) = self {
            Self::Abort(trap.push_wasm_frame(frame))
        } else {
            self
        }
    }
}

impl<E: trap::TrapInfo> trap::TrapInfo for Trap<E> {}

impl<C, E> trap::Trap<C> for Trap<E>
where
    E: trap::Trap<C>,
    C: core::fmt::Debug,
{
    fn trap(cause: C, frame: Option<&'static wasm2rs_rt_core::trace::WasmFrame>) -> Self {
        Self::Abort(E::trap(cause, frame))
    }
}
