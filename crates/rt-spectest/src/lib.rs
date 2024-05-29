//! Runtime support for executing the [WebAssembly specification tests] under `wasm2rs`.
//!
//! [WebAssembly specification tests]: https://github.com/WebAssembly/testsuite

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)] // Unsafe code present in dependencies
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

mod host_ref;
mod imports;

pub use host_ref::HostRef;
pub use imports::SpecTestImports;

/// Runtime support for WebAssembly modules within the specification tests.
pub mod embedder {
    /// Used for WebAssembly modules with no imports, at most one 32-bit linear memory, and at most
    /// one table containing [`FuncRef`]s only.
    ///
    /// [`FuncRef`]: wasm2rs_rt::func_ref::FuncRef
    #[allow(missing_docs)]
    pub mod self_contained {
        #[doc(no_inline)]
        pub use wasm2rs_rt::embedder::self_contained::{
            rt, Imports, Memory0, Module, Store, Table0, Trap,
        };

        pub type ExternRef = crate::HostRef;
    }
}
