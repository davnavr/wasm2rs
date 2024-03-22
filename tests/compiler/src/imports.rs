include!(concat!(env!("OUT_DIR"), "/imports.rs"));

#[derive(Debug)]
pub struct TestImports {
    memory: wasm2rs_rt::memory::HeapMemory32,
}

#[allow(non_snake_case)]
impl TestImports {
    fn memory(&self) -> &wasm2rs_rt::memory::HeapMemory32 {
        &self.memory
    }

    fn FORTY(&self) -> &i32 {
        &40
    }

    fn assert_equal(&self, a: i32, b: i32) -> Result<(), wasm2rs_rt::trap::TrapValue> {
        assert_eq!(a, b, "WASM callee messed up");
        Ok(())
    }
}

#[derive(Debug)]
pub struct Imports {
    test_imports: TestImports,
}

impl Imports {
    fn tests(&self) -> &TestImports {
        &self.test_imports
    }
}

wasm2rs_rt::embedder_with_import! {
    pub mod example_embedder(Imports)
}

wasm!(pub mod imports_example use super::example_embedder);

#[test]
fn imports() {
    use wasm2rs_rt::memory::Memory32;

    let test_imports = TestImports {
        memory: wasm2rs_rt::memory::HeapMemory32::with_limits(1, 2).unwrap(),
    };
    let imports = Imports { test_imports };

    let inst =
        imports_example::Instance::instantiate(example_embedder::State::new(imports)).unwrap();

    assert_eq!(inst.funny_life_number(), Ok(42));

    assert_eq!(inst.two_equals_two(), Ok(()));

    inst.write_5_to_5000().unwrap();
    assert_eq!(
        inst.embedder()
            .imports()
            .test_imports
            .memory
            .i32_load::<0>(5000),
        Ok(5)
    );
}

#[test]
fn memory_import_limits_are_checked() {
    // This has the wrong minimum size.
    let memory = wasm2rs_rt::memory::HeapMemory32::new();

    let result = imports_example::Instance::instantiate(example_embedder::State::new(Imports {
        test_imports: TestImports { memory },
    }));

    assert!(
        matches!(&result, Err(e) if e.code() == wasm2rs_rt::trap::TrapCode::MemoryLimitsCheck { memory: 0, limits: wasm2rs_rt::trap::LimitsCheck::Minimum { expected: 1, actual: 0 } }),
        "expected instantiation to fail, got {result:?}",
    )
}
