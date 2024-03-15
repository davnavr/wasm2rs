//! Test for using `wasm2rs` as a build dependency

include!(concat!(env!("OUT_DIR"), "/simple.rs"));

#[test]
fn add_five_works() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    assert_eq!(inst.add_five(37), 42);
    assert_eq!(inst.add_five(u32::MAX as i32), 4);
}
