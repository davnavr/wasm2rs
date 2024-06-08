//! Runtime support for executing [`wasi_snapshot_preview1`] applications translated by `wasm2rs`.
//!
//! For Rust, C, and C++ compilers, this is the `wasm32-wasip1` target.
//!
//! [`wasi_snapshot_preview1`]: https://github.com/WebAssembly/WASI/tree/snapshot-01

#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)] // Unsafe code present in dependencies
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_alloc)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::exhaustive_enums)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod wasi;

pub use wasi::Wasi;

pub mod api;

/// Provides access to items in the [`wasm2rs_rt_memory`] crate.
pub mod memory {
    #[doc(no_inline)]
    pub use wasm2rs_rt_memory::{
        AccessError, Address, AllocationError, BoundsCheck, BoundsCheckError, EffectiveAddress,
        EmptyMemory, HexDump, LimitsMismatchError, Memory, MemoryExt, PAGE_SIZE,
    };

    pub use wasm2rs_rt_memory_typed as typed;
}
