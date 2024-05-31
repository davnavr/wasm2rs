use wasm2rs_rt::{
    func_ref::FuncRef,
    trap::{TrapCause, TrapError},
};

#[test]
#[cfg(feature = "alloc")]
fn basic_closure_call() {
    let call_counter = alloc::rc::Rc::new(std::cell::Cell::new(0usize));

    let add_both_then_halve = {
        let counter = call_counter.clone();
        move |a: i32, b: i32| -> Result<i32, TrapError> {
            counter.set(counter.get() + 1);
            Ok((a + b) / 2)
        }
    };

    let func_ref = FuncRef::<TrapError>::from_closure_2(add_both_then_halve);

    assert_eq!(
        func_ref.call_2(10, 20, None),
        Ok(15),
        "invoking {func_ref:?} did not return the correct results"
    );

    assert_eq!(call_counter.get(), 1);

    let failed_result = func_ref.call_0::<()>(None);
    assert!(
        matches!(&failed_result, Err(e) if matches!(e.cause(), TrapCause::IndirectCallSignatureMismatch { .. })),
        "expected call to fail with invalid signature, got {failed_result:?}"
    );

    assert_eq!(call_counter.get(), 1);

    let cloned = func_ref.clone();
    assert_eq!(func_ref.call_2(1, 2, None), Ok(1));
    assert_eq!(cloned.call_2(2, 3, None), Ok(2));
    assert_eq!(call_counter.get(), 3);
    assert!(!func_ref.is_null());
}

#[test]
fn inlined_closure() {
    let value = 5i32;
    let closure = move |x: i32| Ok(x.wrapping_add(value));
    let func_ref = FuncRef::<TrapError>::from_closure_1(closure);
    assert_eq!(func_ref.call_1(5i32, None), Ok(10i32));
}

struct BigData {
    a: u32,
    b: u32,
    c: u64,
}

#[test]
#[cfg(feature = "alloc")]
fn big_data_closure_on_heap() {
    let big_data = BigData { a: 1, b: 2, c: 3 };

    let closure = move || -> Result<i64, _> {
        Ok((u64::from(big_data.a) + u64::from(big_data.b) + big_data.c) as i64)
    };

    let func_ref = FuncRef::<TrapError>::from_closure_0(closure);
    assert_eq!(func_ref.call_0(None), Ok(6i64));
}

#[test]
#[should_panic]
#[cfg(not(feature = "alloc"))]
fn big_data_closure_panic() {
    let big_data = BigData { a: 6, b: 7, c: 8 };

    let closure = move || -> Result<i64, _> {
        Ok((u64::from(big_data.a) + u64::from(big_data.b) + big_data.c) as i64)
    };

    let _ = FuncRef::<TrapError>::from_closure_0(closure);
}

#[test]
fn null_call() {
    let null = FuncRef::<TrapError>::NULL;

    let result = null.call_3::<i32, i32, i32, i32>(1, 2, 3, None);
    assert!(
        matches!(&result, Err(e) if matches!(e.cause(), TrapCause::NullFunctionReference { .. })),
        "null function reference should not be invoked, got {result:?}"
    );
}

#[test]
fn padding_bytes() {
    let byte = 10u8;
    let int = 32i32;

    let closure = move || Ok(i32::from(byte) + int);

    assert_eq!(core::mem::size_of_val(&closure), 8);

    let func_ref = FuncRef::<TrapError>::from_closure_0(closure);

    let cloned = func_ref.clone();

    #[cfg(feature = "std")]
    std::println!("{func_ref:?} vs {cloned:?}");
}
