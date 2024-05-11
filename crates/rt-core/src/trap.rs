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
    fn push_wasm_frame(self, frame: &'static crate::trace::WasmFrame) -> Self
    where
        Self: Sized,
    {
        let _ = frame;
        self
    }

    /// Attempts to interpret the [`Trap`] as an [`std::error::Error`].
    #[cfg(feature = "std")]
    fn as_error(&self) -> Option<&(dyn std::error::Error + '_)> {
        None
    }
}

/// Implementation of a [`Trap`] that simply indicates that it occurred, without storing additional
/// information.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[allow(clippy::exhaustive_structs)]
pub struct TrapOccurred;

impl core::fmt::Display for TrapOccurred {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("WebAssembly trap occurred")
    }
}

impl std::error::Error for TrapOccurred {}

impl<C: core::fmt::Debug> Trap<C> for TrapOccurred {
    fn trap(_: C, _: Option<&'static crate::trace::WasmFrame>) -> Self {
        Self
    }

    fn push_wasm_frame(self, _: &'static crate::trace::WasmFrame) -> Self {
        self
    }

    #[cfg(feature = "std")]
    fn as_error(&self) -> Option<&(dyn std::error::Error + '_)> {
        Some(self)
    }
}

#[cfg(feature = "anyhow")]
impl<C> Trap<C> for anyhow::Error
where
    C: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
{
    fn trap(cause: C, frame: Option<&'static crate::trace::WasmFrame>) -> Self {
        let mut err = anyhow::anyhow!(cause);
        if let Some(frame) = frame {
            err = err.context(frame);
        }
        err
    }

    fn push_wasm_frame(self, frame: &'static crate::trace::WasmFrame) -> Self {
        self.context(frame)
    }

    #[cfg(feature = "std")]
    fn as_error(&self) -> Option<&(dyn std::error::Error + '_)> {
        Some(self.as_ref())
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
