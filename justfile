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

test_compiler cargo='cargo': clippy_rt
    cargo run -- convert -i ./tests/compiler/src/simple.wat
    cargo run -- convert -i ./tests/compiler/src/memory.wat
    cargo run -- convert -i ./tests/compiler/src/imports.wat
    cd ./tests/compiler && {{cargo}} test

test_spec:
    cargo run --features test-utils -- \
        test \
        -i ./tests/spec/testsuite/i64.wast \
        --output-directory ./tests/spec/src/generated/
    cd ./tests/spec && cargo test

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc {{FLAGS}}
