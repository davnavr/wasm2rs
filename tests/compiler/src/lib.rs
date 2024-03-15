//! Test for using `wasm2rs` as a build dependency

pub mod simple {
    include!(concat!(env!("OUT_DIR"), "/simple.rs"));
}
