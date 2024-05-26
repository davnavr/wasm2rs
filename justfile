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

test_spec run_flags='': && test_spec_run
    cargo run --features test-utils {{run_flags}} -- \
        test \
        -i ./crates/rt-spectest/tests/spec/testsuite/address.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/align.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/conversions.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/endianness.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/fac.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/forward.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/i64.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/int_exprs.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/int_literals.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/memory_fill.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/traps.wast \
        --output-directory ./crates/rt-spectest/tests/spec/converted/

test_spec_run:
    cargo test --package wasm2rs-rt-spectest

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc {{FLAGS}}
