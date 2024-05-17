include!("memory.wasm.rs");

wasm!(pub mod wasm use wasm2rs_rt::embedder::self_contained);

#[test]
fn basic_read_writes() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    inst.write_my_int().unwrap();
    assert_eq!(inst.read_my_int(), Ok(65));
}

#[test]
#[should_panic]
fn bounds_checking() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    inst.out_of_bounds_read().unwrap();
}

#[test]
fn growing() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    assert_eq!(inst.grow_then_write(), Ok(0xBABA));
}

#[test]
fn active_data_segments() {
    use wasm2rs_rt::memory::Memory;
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    assert_eq!(inst.mem().i32_load(1234), Ok(0x0403_0201));
}
