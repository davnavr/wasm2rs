# `wasm2rs`

> *Like `wasm2c`, but safer*

Translates WebAssembly binary modules (`.wasm`) into a Rust source code file (`.rs`).

The compiler is currently provided as a library meant for use in build scripts (`build.rs`),
allowing a Rust project to use the generated source code.

## Usage

First, add `wasm2rs` as a built dependency:

```toml
[build-dependencies]
...
wasm2rs = { path = "/path/to/wasm2rs/crates/compiler/" } # Currently not available on crates.io
```

Next, add code to invoke `wasm2rs` in your build script:

```rust
// In build.rs
let out_dir = std::env::var_os("OUT_DIR").unwrap();
let wasm: &[u8] = /* read a WASM file from somewhere */;
let output_path = std::path::Path::join(out_dir.as_ref(), "my_file.rs");
let mut output = std::fs::File::create(output_path).unwrap();

// You can set compilation settings here
wasm2rs::Translation::new().compile_from_buffer(&wasm, &mut output).expect("compilation failed");
```

To use the generated code, add an include statement somewhere in your code:

```rust
include!(concat!(env!("OUT_DIR"), "/my_file.rs"));
```
