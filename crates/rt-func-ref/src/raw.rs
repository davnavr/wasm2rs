//! Types for describing the behavior and specifying the underlying implementations for
//! [`FuncRef`]s.
//!
//! [`FuncRef`]: crate::FuncRef

mod data;
mod vtable;

pub use data::Data;
pub use vtable::{Invoke, VTable};

/// Provides an implementation for a [`FuncRef`].
///
/// [`FuncRef`]: crate::FuncRef
pub struct Raw {
    pub(crate) data: Data,
    pub(crate) vtable: &'static VTable,
}

impl Raw {
    /// Creates a new [`Raw`] from the given `data` with the given `vtable`.
    pub const fn new(data: Data, vtable: &'static VTable) -> Self {
        Self { data, vtable }
    }
}

impl core::fmt::Debug for Raw {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Raw")
            .field("vtable", self.vtable)
            .finish_non_exhaustive()
    }
}
