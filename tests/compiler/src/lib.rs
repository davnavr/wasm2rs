//! Test for using `wasm2rs` as a build dependency

include!(concat!(env!("OUT_DIR"), "/simple.rs"));

#[test]
fn add_five_works() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    assert_eq!(inst.add_five(37), Ok(42));
    assert_eq!(inst.add_five(u32::MAX as i32), Ok(4));
}

#[test]
#[should_panic]
fn unreachable_panic() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    let _ = inst.unreachable_instruction();
}
