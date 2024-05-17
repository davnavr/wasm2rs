//! Provides the implementation for [WebAssembly traps].
//!
//! [WebAssembly traps]: https://webassembly.github.io/spec/core/intro/overview.html#trap

mod trap_error;

pub use trap_error::{TrapCause, TrapError};
pub use wasm2rs_rt_core::trap::{Trap, TrapOccurred, TrapWith};
