//! Test for using `wasm2rs` as a build dependency

include!(concat!(env!("OUT_DIR"), "/simple.rs"));

#[test]
fn add_five_works() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    assert_eq!(inst.add_five(37), Ok(42));
    assert_eq!(inst.add_five(u32::MAX as i32), Ok(4));
}

#[test]
fn if_else_block() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    assert_eq!(inst.life(42), Ok(0x4242_4242));
    assert_eq!(inst.life(43), Ok(0xDEAD));
}

#[test]
fn factorial() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();

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
#[should_panic]
fn unreachable_panic() {
    let inst = wasm::Instance::instantiate(wasm::StdRuntime::default()).unwrap();
    let _ = inst.unreachable_instruction();
}
