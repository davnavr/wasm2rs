//! Provides the implementation of stack trace capturing for [`wasm2rs_rt`](crate).

mod wasm_symbol;

// pub use backtrace::{BacktraceFrame as NativeStackTraceFrame, BacktraceSymbol as NativeSymbol};
pub use wasm_symbol::{
    WasmImportSymbol, /* WasmSymbolTable, WasmSymbolTableIter */
    WasmSymbol, WasmSymbolKind, WasmSymbolSignature, WasmValType,
};

/// Stores WebAssembly-specific information for a native stack frame corresponding to a Rust
/// function generated by `wasm2rs`.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct WasmStackTraceFrame {
    /// Indicates the WebAssembly function that a native stack frame refers to.
    pub symbol: &'static WasmSymbol,
    /// A byte offset from the start of the [code section entry] to the WebAssembly instruction.
    ///
    /// [code section entry]: WasmSymbolKind::Defined::offset
    pub offset: u32,
    //location: &'static struct FileLocation { file: &'static str, line: u32, column: u32 },
}

impl WasmStackTraceFrame {
    /// Creates a new stack frame with a [`WasmSymbol`] and a byte offset to the original
    /// WebAssembly instruction.
    ///
    /// Calls to this function are emitted when `wasm2rs` is configured to include stack trace
    /// information in the translated code.
    pub const fn new(symbol: &'static WasmSymbol, offset: u32) -> Self {
        Self { symbol, offset }
    }
}

/// Trait for capturing a stack trace for WebAssembly functions translated to Rust by `wasm2rs`.
pub trait WasmTrace {
    /// Pushes a frame onto the stack, where `None` is used to indicate non-WebAssembly functions.
    fn push(&mut self, frame: Option<WasmStackTraceFrame>);
}

#[derive(Clone)]
#[cfg(not(feature = "alloc"))]
struct WasmStackTraceInner {
    top: Option<WasmStackTraceFrame>,
    others: usize,
}

/// Represents a stack trace for WebAssembly functions translated by `wasm2rs`.
#[derive(Clone)]
pub struct WasmStackTrace {
    #[cfg(feature = "alloc")]
    stack: alloc::vec::Vec<Result<WasmStackTraceFrame, core::num::NonZeroUsize>>,
    #[cfg(not(feature = "alloc"))]
    entries: Option<WasmStackTraceInner>,
}

impl Default for WasmStackTrace {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl From<Option<WasmStackTraceFrame>> for WasmStackTrace {
    fn from(top: Option<WasmStackTraceFrame>) -> Self {
        let mut trace = Self::EMPTY;
        trace.push(top);
        trace
    }
}

impl WasmStackTrace {
    /// An empty stack trace with no frames.
    pub const EMPTY: Self = Self {
        #[cfg(feature = "alloc")]
        stack: alloc::vec::Vec::new(),
        #[cfg(not(feature = "alloc"))]
        entries: None,
    };
}

impl WasmTrace for WasmStackTrace {
    fn push(&mut self, frame: Option<WasmStackTraceFrame>) {
        #[cfg(feature = "alloc")]
        if let Some(wasm_frame) = frame {
            self.stack.push(Ok(wasm_frame));
        } else {
            match self.stack.last_mut() {
                Some(Err(more)) if *more < core::num::NonZeroUsize::MAX => {
                    *more = more.saturating_add(1);
                }
                _ => self.stack.push(Err(core::num::NonZeroUsize::MIN)),
            }
        }

        #[cfg(not(feature = "alloc"))]
        if let Some(entries) = &mut self.entries {
            entries.others = entries.others.saturating_add(1);
        } else {
            self.entries = Some(WasmStackTraceInner {
                top: frame,
                others: 0,
            });
        }
    }
}

impl core::fmt::Debug for WasmStackTrace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut list = f.debug_list();

        #[derive(Debug)]
        struct Omitted;

        #[cfg(feature = "alloc")]
        for entry in self.stack.iter() {
            match entry {
                Ok(frame) => {
                    list.entry(frame);
                }
                Err(omitted) => {
                    list.entries((0usize..omitted.get()).map(|_| Omitted));
                }
            }
        }

        #[cfg(not(feature = "alloc"))]
        if let Some(entries) = &self.entries {
            let omitted = if let Some(top) = &entries.top {
                list.entry(&top);
                0usize
            } else {
                1
            };

            list.entries((0usize..omitted.saturating_add(entries.others)).map(|_| Omitted));
        }

        list.finish()
    }
}

/*
/// Represents a frame in a [`StackTrace`].
#[derive(Clone, Debug)]
pub struct StackTraceFrame {
    #[cfg(feature = "backtrace")]
    native_frame: NativeStackTraceFrame,
    #[cfg(feature = "backtrace")]
    wasm_frames: alloc::vec::Vec<Option<WasmStackTraceFrame>>,
    #[cfg(not(feature = "backtrace"))]
    wasm_frame: WasmStackTraceFrame,
}

/// Represents a stack trace.
///
/// # Implementation Details
///
/// The [`backtrace`] optional dependency provides the implementation for the collection of native
/// stack traces, and requires the `std` feature.
///
/// If the `backtrace` feature is not enabled, or if it does not support the target platform, then
/// then only information collected from a [`WasmStackTrace`] will be available.
#[derive(Clone)]
#[cfg_attr(feature = "backtrace", derive(Debug))]
pub struct StackTrace {
    #[cfg(feature = "backtrace")]
    native_frames: backtrace::Backtrace,
    wasm_symbols: &'static [&'static WasmSymbolTable],
}

// TOOD: Make an immutable struct CapturedStackTrace?

impl StackTrace {
    //pub const EMPTY: Self = Self {};

    /// Captures a [`StackTrace`], using the specified list of [`WasmSymbolTable`]s to determine
    /// which [`NativeSymbol`]s correspond to which [`WasmSymbol`].
    ///
    /// Symbol information is only collected when...
    pub fn capture(
        source_frame: Option<WasmStackTraceFrame>,
        wasm_symbols: &'static [&'static WasmSymbolTable],
    ) {
    }
}

#[cfg(not(feature = "backtrace"))]
impl core::fmt::Debug for StackTrace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StackTrace").finish_non_exhaustive();
    }
}
*/

/// Helper function for pushing a stack trace frame onto an [`Err`] case of a [`Result`].
pub fn push_wasm_frame<T, E>(
    result: core::result::Result<T, E>,
    frame: &'static WasmStackTraceFrame,
) -> core::result::Result<T, E>
where
    E: WasmTrace,
{
    match result {
        Ok(ok) => Ok(ok),
        Err(mut e) => {
            e.push(Some(*frame));
            Err(e)
        }
    }
}
