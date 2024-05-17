alias f := fmt
alias d := doc

check: fmt
    cargo clippy
    # cargo clippy --package wasm2rs-rt --no-default-features --features alloc,merged
    # cargo clippy --package wasm2rs-rt --no-default-features --features merged

clippy_rt:
    cargo clippy -p "wasm2rs-rt*" --no-default-features
    cargo clippy -p "wasm2rs-rt*"
    cargo clippy -p "wasm2rs-rt*" --all-features

fmt *FLAGS='':
    cargo fmt {{FLAGS}}

compiler_test:
    cargo run -- convert -i ./tests/compiler/src/simple.wat
    cd ./tests/compiler && cargo test

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc {{FLAGS}}
