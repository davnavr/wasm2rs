//! Unit tests for [`wasm2rs_rt`].

#![no_std]
#![deny(clippy::std_instead_of_alloc)]
#![deny(clippy::alloc_instead_of_core)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

mod func_ref;
