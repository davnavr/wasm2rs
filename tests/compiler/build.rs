fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    let compile_wasm = |wat: &str, name: &str| {
        let mut out_path = std::path::Path::join(out_dir.as_ref(), name);
        out_path.set_extension("rs");

        let wasm = match wat::parse_str(wat) {
            Ok(ok) => ok,
            Err(e) => panic!("could not encode {name:?} into WebAssembly binary: {e}"),
        };

        let mut output = match std::fs::File::create(&out_path) {
            Ok(ok) => ok,
            Err(e) => panic!("could not open output file {out_path:?}: {e}"),
        };

        if let Err(e) = wasm2rs::Translation::new().compile_from_buffer(&wasm, &mut output) {
            panic!("compilation failed for {name:?}: {e}");
        }
    };

    println!("cargo:rerun-if-changed=src/simple.wat");
    println!("cargo:rerun-if-changed=src/memory.wat");
    compile_wasm(include_str!("./src/simple.wat"), "simple");
    compile_wasm(include_str!("./src/memory.wat"), "memory");
}
