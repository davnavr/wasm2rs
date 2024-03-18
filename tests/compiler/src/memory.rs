include!(concat!(env!("OUT_DIR"), "/memory.rs"));

#[test]
fn basic_read_writes() {
    let inst = wasm::Instance::instantiate().unwrap();
    inst.write_my_int().unwrap();
    assert_eq!(inst.read_my_int(), Ok(65));
}

#[test]
#[should_panic]
fn bounds_checking() {
    let inst = wasm::Instance::instantiate().unwrap();
    inst.out_of_bounds_read().unwrap();
}

#[test]
fn growing() {
    let inst = wasm::Instance::instantiate().unwrap();
    assert_eq!(inst.grow_then_write(), Ok(0xBABA));
}
