use table::AnyTable;
use wasm2rs_rt::table::{self, BoundsCheckError, Table};

#[test]
fn array() {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestRef(Option<u32>);

    impl table::TableElement for TestRef {}

    impl table::NullableTableElement for TestRef {
        const NULL: Self = Self(None);
    }

    let table = table::ArrayTable::<TestRef, 7>::new();

    assert_eq!(table.grow(1), 0);
    assert_eq!(table.set(0, TestRef(Some(111))), Ok(()));
    assert_eq!(table.set(1, TestRef(Some(222))), Err(BoundsCheckError));

    assert_eq!(table.grow(2), 1);
    assert_eq!(table.get(0), Ok(TestRef(Some(111))));
    assert_eq!(table.get(1), Ok(TestRef(None)));
    assert_eq!(table.get(2), Ok(TestRef(None)));
    assert_eq!(table.set(2, TestRef(Some(222))), Ok(()));
    assert_eq!(table.get(1), Ok(TestRef(None)));
    assert_eq!(table.get(0), Ok(TestRef(Some(111))));

    assert_eq!(table.size(), 3);
    assert_eq!(table.replace(2, TestRef(Some(333))), Ok(TestRef(Some(222))));
    assert_eq!(table.get(2), Ok(TestRef(Some(333))));

    assert_eq!(table.grow(2), 3);

    let mut buffer = [TestRef(None); 4];
    assert_eq!(
        table.clone_into_slice(1, buffer.as_mut_slice()),
        Ok(()),
        "{table:?}"
    );
    assert_eq!(
        buffer,
        [
            TestRef(None),
            TestRef(Some(333)),
            TestRef(None),
            TestRef(None),
        ]
    );

    assert_eq!(
        table.clone_into_slice(42, &mut [TestRef(None); 12]),
        Err(BoundsCheckError)
    );

    assert_eq!(table.grow(2), 5);
    let mut items = [
        TestRef(Some(0x11)),
        TestRef(Some(0x22)),
        TestRef(Some(0x33)),
        TestRef(Some(0x44)),
    ];
    assert_eq!(table.clone_from_slice(2, items.as_mut_slice()), Ok(()));
    assert_eq!(table.clone_into_slice(2, buffer.as_mut_slice()), Ok(()));
    assert_eq!(buffer, items);
    assert_eq!(table.get(3), Ok(TestRef(Some(0x22))));

    assert_eq!(table.grow(2), table::GROW_FAILED);
    assert_eq!(table.grow(0), 7);
}
