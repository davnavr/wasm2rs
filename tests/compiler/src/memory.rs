include!(concat!(env!("OUT_DIR"), "/memory.rs"));

#[test]
fn basic_read_writes() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    inst.write_my_int().unwrap();
    assert_eq!(inst.read_my_int(), Ok(65));
}
