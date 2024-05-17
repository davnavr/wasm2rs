alias f := fmt
alias d := doc

check: fmt clippy_tools clippy_rt

clippy_tools:
    # cargo clippy --package wasm2rs-convert
    # cargo clippy --package wasm2rs-convert --no-default-features
    # cargo clippy --package wasm2rs-cli --all-features

clippy_rt:
    cargo clippy --package "wasm2rs-rt*" --no-default-features
    cargo clippy --package "wasm2rs-rt*"
    cargo clippy --package "wasm2rs-rt*" --all-features

fmt *FLAGS='':
    cargo fmt {{FLAGS}}

test_compiler: clippy_rt
    cargo run -- convert -i ./tests/compiler/src/simple.wat
    cd ./tests/compiler && cargo test

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc {{FLAGS}}
