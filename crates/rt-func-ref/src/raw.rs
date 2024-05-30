mod data;
mod vtable;

pub use data::RawFuncRefData;
pub use vtable::{RawFuncRefInvoke, RawFuncRefVTable};

/// Provides an implementation for a [`FuncRef`].
///
/// [`FuncRef`]: crate::FuncRef
pub struct RawFuncRef {
    pub(crate) data: RawFuncRefData,
    pub(crate) vtable: &'static RawFuncRefVTable,
}

impl RawFuncRef {
    /// Creates a new [`RawFuncRef`] from the given `data` with the given `vtable`.
    pub const fn new(data: RawFuncRefData, vtable: &'static RawFuncRefVTable) -> Self {
        Self { data, vtable }
    }
}

impl core::fmt::Debug for RawFuncRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawFuncRef")
            .field("vtable", self.vtable)
            .finish_non_exhaustive()
    }
}
