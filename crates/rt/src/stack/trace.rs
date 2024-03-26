//! Provides the implementation of stack trace capturing for [`wasm2rs_rt`](crate).

/// Represents information about a WebAssembly function that was translated by `wasm2rs`.
#[derive(Clone, Copy, Debug, Hash)]
#[non_exhaustive]
pub struct WasmSymbol {
    //module: &'static str,
    /// A list of the names that the function was [exported] with.
    ///
    /// [exported]: https://webassembly.github.io/spec/core/syntax/modules.html#exports
    pub export_names: &'static [&'static str],
    //signature: WasmSymbolSignature { parameters, results },
    /// The [index] of the function in the WebAssembly module.
    ///
    /// [index]: https://webassembly.github.io/spec/core/syntax/modules.html#indices
    pub index: u32,
    /// An byte offset from the start of the WebAssembly module to the [code section entry]
    /// corresponding to the WebAssembly function.
    ///
    /// [code section entry]: https://webassembly.github.io/spec/core/binary/modules.html#code-section
    pub offset: u32,
}

impl WasmSymbol {
    /// Creates a new [`WasmSymbol`] with an [`index`] and [`offset`].
    ///
    /// [`index`]: WasmSymbol::index
    /// [`offset`]: WasmSymbol::offset
    pub const fn from_index_and_offset(index: u32, offset: u32) -> Self {
        Self {
            export_names: &[],
            index,
            offset,
        }
    }
}

//pub struct WasmSymbolTable { symbols: &'static [(usize, WasmSymbol)] }

#[derive(Clone, Copy, Debug, Hash)]
#[non_exhaustive]
pub struct WasmStackTraceFrame {
    /// Indicates the WebAssembly function that a [`StackTraceFrame`] refers to.
    pub symbol: &'static WasmSymbol,
    /// An offset from the start of the [code section entry] to the WebAssembly instruction.
    pub offset: Option<u32>,
}

#[derive(Debug)]
pub struct StackTraceFrame {
    #[cfg(feature = "backtrace")]
    native_frame: backtrace::BacktraceFrame,
    #[cfg(feature = "backtrace")]
    wasm_frames: alloc::vec::Vec<Option<WasmStackTraceFrame>>,
    #[cfg(not(feature = "backtrace"))]
    wasm_frame: WasmStackTraceFrame,
}

/// Represents a stack trace.
///
/// # Implementation Details
///
/// The [`backtrace`] optional dependency provides the implementation for the collection of stack
/// traces, and requires the `std` feature.
///
/// If the `backtrace` feature is not enabled, or if it does not support the target platform, then
/// the stack trace collection methods do nothing.
#[cfg_attr(feature = "backtrace", derive(Debug))]
pub struct StackTrace {
    #[cfg(feature = "backtrace")]
    frames: std::vec::Vec<StackTraceFrame>,
}

impl StackTrace {
    //pub const EMPTY: Self = Self {};

    //pub fn capture(unresolved: bool, PARAMETER_FOR_WASM_INFO) {}
}

#[cfg(not(feature = "backtrace"))]
impl core::fmt::Debug for StackTrace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StackTrace").finish_non_exhaustive();
    }
}
