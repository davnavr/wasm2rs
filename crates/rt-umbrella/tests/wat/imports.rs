include!("imports.wasm.rs");

mod embedder {
    pub(super) use rt::embedder::self_contained::{Memory0, Module, Trap};
    pub(super) use wasm2rs_rt as rt;

    #[derive(Debug)]
    pub(super) struct TestImports {
        pub(super) private_memory: wasm2rs_rt::memory::HeapMemory,
    }

    #[allow(non_snake_case)]
    impl TestImports {
        pub(super) fn memory(&self) -> &Memory0 {
            &self.private_memory
        }

        pub(super) const fn FORTY(&self) -> i32 {
            40
        }

        pub(super) fn assert_equal(&self, a: i32, b: i32) -> Result<(), Trap> {
            assert_eq!(a, b, "WASM callee messed up");
            Ok(())
        }
    }

    #[derive(Debug)]
    pub(super) struct Imports {
        pub(super) test_imports: TestImports,
    }

    #[allow(non_snake_case)]
    impl Imports {
        pub(super) fn tests(&self) -> &TestImports {
            &self.test_imports
        }
    }

    #[derive(Debug)]
    pub(super) struct Store {
        pub(super) imports: Imports,
        pub(super) instance: rt::store::AllocateModuleRc,
    }
}

wasm!(pub mod imports_example use super::embedder);

#[test]
fn imports() {
    use wasm2rs_rt::memory::Memory;

    let imports = embedder::Imports {
        test_imports: embedder::TestImports {
            private_memory: wasm2rs_rt::memory::HeapMemory::with_limits(1, 2).unwrap(),
        },
    };

    let inst = imports_example::Instance::instantiate(embedder::Store {
        imports,
        instance: wasm2rs_rt::store::AllocateModuleRc,
    })
    .unwrap();

    assert_eq!(inst.funny_life_number(), Ok(42));

    assert_eq!(inst.two_equals_two(), Ok(()));

    inst.write_5_to_5000().unwrap();
    assert_eq!(inst.imports.test_imports.private_memory.i32_load(5000), Ok(5));
}

#[test]
fn memory_import_limits_are_checked() {
    // This has the wrong minimum size.
    let private_memory = wasm2rs_rt::memory::HeapMemory::new();

    let result = imports_example::Instance::instantiate(embedder::Store {
        imports: embedder::Imports {
            test_imports: embedder::TestImports { private_memory },
        },
        instance: wasm2rs_rt::store::AllocateModuleRc,
    });

    assert!(
        //matches!(&result, Err(e) if e.cause() == wasm2rs_rt::trap::TrapCause::MemoryLimitsMismatch { memory: 0, limits: wasm2rs_rt::trap::LimitsCheck::Minimum { expected: 1, actual: 0 } }),
        matches!(&result, Err(e) if matches!(e.cause(), wasm2rs_rt::trap::TrapCause::MemoryLimitsMismatch { .. })),
        "expected instantiation to fail, got error {:?}",
        result.err()
    )
}
