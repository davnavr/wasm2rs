/// A [`Trap`] implementation that reports traps as normal Rust values.
///
/// [`Trap`]: crate::trap::Trap
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ReturnOnTrap;

impl crate::trap::Trap for ReturnOnTrap {
    type Repr = TrapValue;

    fn trap(&self, code: crate::trap::TrapCode) -> TrapValue {
        TrapValue::new(code)
    }
}

struct Inner {
    //rust_backtrace: std::backtrace::Backtrace,
    //wasm_backtrace: ?,
    code: crate::trap::TrapCode,
}

/// Describes a WebAssembly trap that was reported with [`ReturnOnTrap`].
///
/// If the `alloc` feature is not enabled, heap allocation is not used to store additional
/// information, and only the [`TrapCode`] is stored.
///
/// [`TrapCode`]: crate::trap::TrapCode
#[repr(transparent)]
#[must_use]
pub struct TrapValue {
    #[cfg(feature = "alloc")]
    inner: alloc::boxed::Box<Inner>,
    #[cfg(not(feature = "alloc"))]
    inner: Inner,
}

impl TrapValue {
    fn new(code: crate::trap::TrapCode) -> Self {
        let inner = Inner { code };
        Self {
            #[cfg(feature = "alloc")]
            inner: alloc::boxed::Box::new(inner),
            #[cfg(not(feature = "alloc"))]
            inner,
        }
    }

    /// Gets the cause of this trap.
    pub fn code(&self) -> &crate::trap::TrapCode {
        &self.inner.code
    }
}

impl core::cmp::PartialEq for TrapValue {
    fn eq(&self, other: &Self) -> bool {
        self.code() == other.code()
    }
}

impl core::fmt::Debug for TrapValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TrapValue")
            .field("code", self.code())
            .finish()
    }
}

impl core::fmt::Display for TrapValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.code(), f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TrapValue {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.code())
    }
}
