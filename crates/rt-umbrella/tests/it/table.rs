use wasm2rs_rt::table::{self, AnyTable, BoundsCheckError, Table};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
struct IntRef(Option<u32>);

impl table::TableElement for IntRef {}

impl table::NullableTableElement for IntRef {
    const NULL: Self = Self(None);
}

#[test]
fn array() {
    let table = table::ArrayTable::<IntRef, 7>::new();

    assert_eq!(table.grow(1), 0);
    assert_eq!(table.set(0, IntRef(Some(111))), Ok(()));
    assert_eq!(table.set(1, IntRef(Some(222))), Err(BoundsCheckError));

    assert_eq!(table.grow(2), 1);
    assert_eq!(table.get(0), Ok(IntRef(Some(111))));
    assert_eq!(table.get(1), Ok(IntRef(None)));
    assert_eq!(table.get(2), Ok(IntRef(None)));
    assert_eq!(table.set(2, IntRef(Some(222))), Ok(()));
    assert_eq!(table.get(1), Ok(IntRef(None)));
    assert_eq!(table.get(0), Ok(IntRef(Some(111))));

    assert_eq!(table.size(), 3);
    assert_eq!(table.replace(2, IntRef(Some(333))), Ok(IntRef(Some(222))));
    assert_eq!(table.get(2), Ok(IntRef(Some(333))));

    assert_eq!(table.grow(2), 3);

    let mut buffer = [IntRef(None); 4];
    assert_eq!(
        table.clone_into_slice(1, buffer.as_mut_slice()),
        Ok(()),
        "{table:?}"
    );
    assert_eq!(
        buffer,
        [IntRef(None), IntRef(Some(333)), IntRef(None), IntRef(None),]
    );

    assert_eq!(
        table.clone_into_slice(42, &mut [IntRef(None); 12]),
        Err(BoundsCheckError)
    );

    assert_eq!(table.grow(2), 5);
    let mut items = [
        IntRef(Some(0x11)),
        IntRef(Some(0x22)),
        IntRef(Some(0x33)),
        IntRef(Some(0x44)),
    ];
    assert_eq!(table.clone_from_slice(2, items.as_mut_slice()), Ok(()));
    assert_eq!(table.clone_into_slice(2, buffer.as_mut_slice()), Ok(()));
    assert_eq!(buffer, items);
    assert_eq!(table.get(3), Ok(IntRef(Some(0x22))));

    assert_eq!(table.grow(2), table::GROW_FAILED);
    assert_eq!(table.grow(0), 7);
}

#[test]
#[cfg(feature = "alloc")]
fn heap_table_cannot_allocate() {
    let result = table::HeapTable::<IntRef>::with_limits(3, 1);
    assert!(matches!(result, Err(e) if e.size() == 3), "{result:?}");

    // Can't test request for `u32::MAX` elements. since it could actually succeed on 64-bit systems.
}

#[test]
#[cfg(feature = "alloc")]
fn heap_table() {
    use alloc::boxed::Box;

    #[derive(Clone, Debug, Eq, PartialEq)]
    #[repr(transparent)]
    struct TestRef(Option<Box<u32>>);

    const EMPTY_ELEMENT: TestRef = TestRef(None);

    impl table::TableElement for TestRef {}

    impl table::NullableTableElement for TestRef {
        const NULL: Self = Self(None);
    }

    let table = table::HeapTable::<TestRef>::with_maximum(10);

    assert_eq!(table.get(0), Err(BoundsCheckError), "{table:?}");
    assert_eq!(
        table.set(1, TestRef(None)),
        Err(BoundsCheckError),
        "{table:?}"
    );
    assert!(table.is_empty(), "{table:?}");

    assert_eq!(table.grow(4), 0, "{table:?}");
    assert_eq!(
        table.replace(2, TestRef(Some(Box::new(2222)))),
        Ok(TestRef(None)),
        "{table:?}"
    );
    assert_eq!(table.get(1), Ok(TestRef(None)), "{table:?}");
    assert_eq!(table.get(3), Ok(TestRef(None)), "{table:?}");

    assert_eq!(table.grow(3), 4, "{table:?}");
    assert_eq!(table.get(4), Ok(TestRef(None)), "{table:?}");

    assert_eq!(table.grow(4), table::GROW_FAILED, "{table:?}");
    assert_eq!(table.maximum(), 10, "{table:?}");

    let buffer = [
        TestRef(Some(Box::new(0xAA))),
        TestRef(Some(Box::new(0xBB))),
        TestRef(Some(Box::new(0xCC))),
        TestRef(Some(Box::new(0xDD))),
    ];
    assert_eq!(
        table.clone_from_slice(3, buffer.as_slice()),
        Ok(()),
        "{table:?}"
    );
    assert_eq!(table.grow(3), 7, "{table:?}");
    assert_eq!(table.size(), table.maximum(), "{table:?}");
    let mut result = [EMPTY_ELEMENT; 4];
    assert_eq!(
        table.clone_into_slice(3, result.as_mut_slice()),
        Ok(()),
        "{table:?}"
    );
    assert_eq!(buffer, result, "{table:?}");
}
