alias f := fmt
alias d := doc

check: fmt
    cargo clippy
    # cargo clippy --package wasm2rs-rt --no-default-features --features alloc,merged
    # cargo clippy --package wasm2rs-rt --no-default-features --features merged

fmt:
    cargo fmt

compiler_test:
    cargo run -- convert -i ./tests/compiler/src/simple.wat
    cd ./tests/compiler && cargo test

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc {{FLAGS}}
