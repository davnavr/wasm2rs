include!(concat!(env!("OUT_DIR"), "/memory.rs"));

#[test]
fn basic_read_writes() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    inst.write_my_int().unwrap();
    assert_eq!(inst.read_my_int(), Ok(65));
}

#[test]
#[should_panic]
fn bounds_checking() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    let _ = inst.out_of_bounds_read();
}

#[test]
fn growing() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    assert_eq!(inst.grow_then_write(), Ok(0xBABA));
}
