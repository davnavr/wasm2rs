//! WASI port of the CPython interpreter translated by `wasm2rs`.
//!
//! This uses precompiled WebAssembly modules provided by [VMware Labs WLR].
//!
//! [VMware Labs WLR]: https://github.com/vmware-labs/webassembly-language-runtimes

mod embedder {
    pub use wasm2rs_rt::embedder::self_contained::{rt, Module, Table0};

    pub type Trap = wasm2rs_rt_wasip1::Trap<wasm2rs_rt::trap::TrapError>;

    pub type Memory0 = std::rc::Rc<wasm2rs_rt::memory::HeapMemory>;

    pub type WasiApi =
        wasm2rs_rt_wasip1::StdApi<Memory0, wasm2rs_rt::trap::TrapError>;

    #[derive(Debug)]
    pub struct Imports {
        pub wasi: wasm2rs_rt_wasip1::Wasi<WasiApi>,
    }

    impl Imports {
        pub fn wasi_snapshot_preview1(&self) -> &wasm2rs_rt_wasip1::Wasi<WasiApi> {
            &self.wasi
        }
    }

    #[derive(Debug)]
    pub struct Store {
        pub imports: Imports,
        pub instance: wasm2rs_rt::store::AllocateModuleRc,
        pub table0:
            wasm2rs_rt::store::AllocateHeapTable<wasm2rs_rt::func_ref::FuncRef<'static, Trap>>,
        pub memory0: wasm2rs_rt::store::ReuseExistingMemory<Memory0>,
    }
}

#[rustfmt::skip]
include!("generated/python3.wasm2.rs");

wasm!(pub mod python3 use super::embedder);

fn main() -> std::process::ExitCode {
    let wasi_api = wasm2rs_rt_wasip1::StdApiBuilder::new()
        .inherit_standard_streams_without_sanitation()
        .build();

    let memory = std::rc::Rc::new(embedder::Memory0::default());

    let wasi = wasm2rs_rt_wasip1::Wasi::new(memory.clone(), wasi_api);

    let store = embedder::Store {
        imports: embedder::Imports { wasi },
        instance: Default::default(),
        table0: Default::default(),
        memory0: wasm2rs_rt::store::ReuseExistingMemory::new(memory),
    };

    macro_rules! return_on_trap {
        ($expr:expr) => {
            match $expr {
                Ok(value) => value,
                Err(trap) => std::process::Termination::report(trap),
            }
        };
    }

    let inst = return_on_trap!(python3::Instance::instantiate(store));

    return_on_trap!(inst.wasi_vfs_pack_fs());
    return_on_trap!(inst._start());

    std::process::ExitCode::SUCCESS
}
