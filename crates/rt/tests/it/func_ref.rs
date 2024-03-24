use wasm2rs_rt::{
    embedder::State,
    func_ref::FuncRef,
    trap::{TrapCode, TrapValue},
};

#[test]
fn basic_closure_call() {
    let call_counter = std::rc::Rc::new(std::cell::Cell::new(0usize));

    let add_both_then_halve = {
        let counter = call_counter.clone();
        move |a: i32, b: i32| -> Result<i32, TrapValue> {
            counter.set(counter.get() + 1);
            Ok((a + b) / 2)
        }
    };

    let trap = State::<()>::default();
    let func_ref = FuncRef::from_closure_2(add_both_then_halve);

    assert_eq!(
        func_ref.call_2(10, 20, &trap),
        Ok(15),
        "invoking {func_ref:?} did not return the correct results"
    );

    assert_eq!(call_counter.get(), 1);

    let failed_result = func_ref.call_0::<(), _>(&trap);
    assert!(
        matches!(&failed_result, Err(e) if matches!(e.code(), TrapCode::IndirectCallSignatureMismatch(_))),
        "expected call to fail with invalid signature, got {failed_result:?}"
    );

    assert_eq!(call_counter.get(), 1);
}

#[test]
fn null_call() {
    let trap = State::<()>::default();
    let null = FuncRef::<TrapValue>::NULL;

    let result = null.call_3::<i32, i32, i32, i32, _>(1, 2, 3, &trap);
    assert!(
        matches!(&result, Err(e) if matches!(e.code(), TrapCode::NullFunctionReference { .. })),
        "null function reference should not be invoked, got {result:?}"
    );
}
