include!(concat!(env!("OUT_DIR"), "/simple.rs"));

wasm!(pub mod wasm);

#[test]
fn add_works() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    assert_eq!(inst.add_five(37), Ok(42));
    assert_eq!(inst.add_five(u32::MAX as i32), Ok(4));
    assert_eq!(inst.add_fifteen(10), Ok(25));
}

#[test]
fn if_else_block() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    assert_eq!(inst.life(42), Ok(0x4242_4242));
    assert_eq!(inst.life(43), Ok(0xDEAD));
}

#[test]
fn factorial() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();

    macro_rules! tests {
        ($($input:expr => $output:expr),*) => {$(
            assert_eq!(inst.loop_factorial_unsigned($input), Ok($output));
        )*};
    }

    tests! {
        0 => 1,
        1 => 1,
        2 => 2,
        3 => 6,
        4 => 24,
        5 => 120,
        6 => 720,
        7 => 5040,
        8 => 40320,
        9 => 362880,
        10 => 3628800,
        11 => 39916800,
        12 => 479001600
    }
}

#[test]
fn halting() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    inst.halt_on_even(6).unwrap();
}

#[test]
#[should_panic]
fn unreachable_panic() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    inst.unreachable_instruction().unwrap();
}

#[test]
fn immutable_global() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    assert_eq!(inst.size_of_kibibyte(), Ok(1024));

    inst.increment_counter().unwrap();
    inst.increment_counter().unwrap();
    assert_eq!(inst.counter().get(), 2);
}

#[test]
fn br_if() {
    let inst = wasm::Instance::instantiate(Default::default()).unwrap();
    assert_eq!(inst.trap_on_three(2), Ok(()));

    let result = inst.trap_on_three(3);
    assert!(
        matches!(&result, Err(e) if e.code() == wasm2rs_rt::trap::TrapCode::Unreachable),
        "expected trap unreachable, but got {result:?}"
    );
}
