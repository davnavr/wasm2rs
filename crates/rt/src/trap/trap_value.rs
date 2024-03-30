struct Inner {
    //rust_backtrace: std::backtrace::Backtrace,
    code: crate::trap::TrapCode,
    #[cfg(feature = "alloc")]
    wasm_backtrace: crate::stack::trace::WasmStackTrace,
}

/// Describes a WebAssembly trap.
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
    pub(crate) fn new(
        code: crate::trap::TrapCode,
        frame: Option<&'static crate::trap::WasmStackTraceFrame>,
    ) -> Self {
        let inner = Inner {
            code,
            #[cfg(feature = "alloc")]
            wasm_backtrace: crate::stack::trace::WasmStackTrace::from(frame.copied()),
        };

        #[cfg(not(feature = "alloc"))]
        let _ = frame;

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

    /// Gets a backtrace capturing the WebAssembly stack frames.
    ///
    /// If the `alloc` feature is not enabled, then an empty stack trace is returned.
    pub fn wasm_stack_trace(&self) -> &crate::stack::trace::WasmStackTrace {
        #[cfg(feature = "alloc")]
        return &self.inner.wasm_backtrace;

        #[cfg(not(feature = "alloc"))]
        return &crate::stack::trace::WasmStackTrace::EMPTY;
    }
}

impl crate::stack::trace::WasmTrace for TrapValue {
    fn push(&mut self, frame: Option<crate::trap::WasmStackTraceFrame>) {
        #[cfg(feature = "alloc")]
        crate::stack::trace::WasmTrace::push(&mut self.inner.wasm_backtrace, frame);

        #[cfg(not(feature = "alloc"))]
        let _ = frame;
    }
}

impl core::cmp::PartialEq for TrapValue {
    fn eq(&self, other: &Self) -> bool {
        // TODO: How does WASM backtrace impact equality? Maybe remove this impl?
        self.code() == other.code()
    }
}

impl core::fmt::Debug for TrapValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TrapValue")
            .field("code", self.code())
            .field("wasm_stack_trace", self.wasm_stack_trace())
            .finish()
    }
}

impl core::fmt::Display for TrapValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // TODO: Include Display impl for WasmStackTrace
        core::fmt::Display::fmt(self.code(), f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TrapValue {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.code())
    }
}
