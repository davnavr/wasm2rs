include!("ref_func.wasm.rs");

wasm!(pub mod wasm use wasm2rs_rt::embedder::self_contained);

#[test]
fn uninit() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();

    //inst.callTheFunc();

    wasm::Instance::leak(inst);
}
