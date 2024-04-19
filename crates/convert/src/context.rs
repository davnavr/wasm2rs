//! Types describing how WebAssembly constructs are accessed in the translated Rust code.

pub(crate) enum CallKind {
    /// No additional argument is added. The generated Rust function is an associated
    /// function.
    Function,
    /// A `self` argument is added. The generated Rust function is a method.
    ///
    /// A function generated with this [`CallKind`] is always correct.
    Method,
    // /// An additional argument is added to access embedder specific data.
    // ///
    // /// This is mainly used for accessing imports.
    // WithEmbedder,
    // /// The function only accesses a linear memory.
    // WithMemory(u32),
}

//enum { NoUnwind, CanUnwind, AlwaysUnwinds }

//enum { Pure, ReadsMemory, Impure }

pub(crate) struct Context {
    pub(crate) call_kind: CallKind,
    pub(crate) can_trap: bool,
}

impl Context {
    /// Returns `true` if the function can trap or produce an exception.
    pub(crate) fn can_unwind(&self) -> bool {
        self.can_trap
    }
}
