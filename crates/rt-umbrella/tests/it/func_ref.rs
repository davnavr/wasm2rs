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
    assert_ne!(func_ref, FuncRef::<TrapError>::NULL);

    #[cfg(not(miri))] // `const`s are not equal in miri?
    assert_eq!(func_ref, cloned); // Addresses of the `Rc` are the same
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
