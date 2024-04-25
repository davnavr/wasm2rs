//! Provides the [`Trap`] trait.

/// Trait for implementing WebAssembly traps.
pub trait Trap<C: core::fmt::Debug>: core::fmt::Debug {
    /// Generates a trap with the given reason and an optional WebAssembly stack frame indicating
    /// the source of the trap in the original WebAssembly function.
    ///
    /// The `wasm2rs` compiler generates calls to this function for instructions that generate a
    /// trap.
    fn trap(cause: C, frame: Option<&'static crate::trace::WasmFrame>) -> Self
    where
        Self: Sized;

    /// Appends a WebAssembly stack trace frame to the [`Trap`]'s stack trace, if it has one.
    fn push_wasm_frame(&mut self, frame: &'static crate::trace::WasmFrame) {
        let _ = frame;
    }

    /// Attempts to interpret the [`Trap`] as an [`std::error::Error`].
    #[cfg(feature = "std")]
    fn as_error(&self) -> Option<&(dyn std::error::Error + '_)> {
        None
    }
}

/// Helper trait for producing [`Trap`]s out of [`Result`]s
pub trait TrapWith<T, C: core::fmt::Debug> {
    /// Produces a [`Trap`] from a [`Result`]'s [`Err`] case.
    fn trap_with<E: Trap<C>>(self, frame: &'static crate::trace::WasmFrame) -> Result<T, E>;
}

impl<T, C: core::fmt::Debug> TrapWith<T, C> for Result<T, C> {
    #[inline]
    fn trap_with<E: Trap<C>>(self, frame: &'static crate::trace::WasmFrame) -> Result<T, E> {
        match self {
            Ok(ok) => Ok(ok),
            Err(cause) => Err(E::trap(cause, Some(frame))),
        }
    }
}
