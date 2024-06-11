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

impl<E: trap::TrapInfo> From<E> for Trap<E> {
    fn from(trap: E) -> Self {
        Self::Abort(trap)
    }
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

#[cfg(feature = "std")]
impl<E: trap::TrapInfo> std::process::Termination for Trap<E> {
    fn report(self) -> std::process::ExitCode {
        use std::io::Write as _;

        match self {
            Self::ProcExit(exit_code) => {
                if exit_code != api::ExitCode::SUCCESS {
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "Process exited with code {} ({:#X})",
                        exit_code.0 as i32,
                        exit_code.0
                    );
                }

                exit_code.to_exit_code_lossy()
            }
            Self::Abort(trap) => {
                struct TrapDisplay<E: trap::TrapInfo>(E);

                impl<E: trap::TrapInfo> core::fmt::Display for TrapDisplay<E> {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        if let Some(err) = self.0.as_error() {
                            core::fmt::Display::fmt(err, f)
                        } else {
                            core::fmt::Debug::fmt(&self.0, f)
                        }
                    }
                }

                let _ = writeln!(
                    std::io::stderr().lock(),
                    "Trap occurred: {}",
                    TrapDisplay(trap)
                );
                std::process::ExitCode::FAILURE
            }
            Self::ProcRaise(signal) => {
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "Signal raised: {signal:?} ({})",
                    signal as u8
                );
                std::process::ExitCode::FAILURE
            }
        }
    }
}
