alias f := fmt
alias d := doc
alias t := test

check: fmt clippy_tools clippy_rt test

test: test_compiler test_spec
    cargo test --package wasm2rs-convert

clippy_tools:
    cargo clippy --package wasm2rs-convert
    cargo clippy --package wasm2rs-convert --no-default-features
    cargo clippy --package wasm2rs-cli
    cargo clippy --package wasm2rs-cli --all-features

clippy_rt:
    cargo clippy --package "wasm2rs-rt*" --no-default-features
    cargo clippy --package "wasm2rs-rt*"
    cargo clippy --package "wasm2rs-rt*" --all-features

fmt *FLAGS='':
    cargo fmt {{FLAGS}}

test_compiler: clippy_rt
    cargo run -p wasm2rs-cli -- convert -i ./crates/rt-umbrella/tests/wat/simple.wat
    cargo run -p wasm2rs-cli -- convert -i ./crates/rt-umbrella/tests/wat/memory.wat
    cargo run -p wasm2rs-cli -- convert -i ./crates/rt-umbrella/tests/wat/imports.wat
    cargo run -p wasm2rs-cli -- convert -i ./crates/rt-umbrella/tests/wat/ref_func.wat
    cargo test --package wasm2rs-rt --test wat

test_spec run_flags='': && test_spec_run
    cargo run --features test-utils {{run_flags}} -- \
        test \
        -i ./crates/rt-spectest/tests/spec/testsuite/address.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/align.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/conversions.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/endianness.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/fac.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/f32_bitwise.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/f32_cmp.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/f32.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/f64_bitwise.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/f64_cmp.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/f64.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/forward.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/i64.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/int_exprs.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/int_literals.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/labels.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/memory_fill.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/nop.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/return.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/switch.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/traps.wast \
        -i ./crates/rt-spectest/tests/spec/testsuite/unreached-valid.wast \
        --output-directory ./crates/rt-spectest/tests/spec/converted/

test_spec_run:
    cargo test --package wasm2rs-rt-spectest

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc {{FLAGS}}

build_demo_python3:
    cargo run -p wasm2rs-cli -- convert \
        -i ./demo/python3/wasm/python-3.12.0.wasm \
        -o ./demo/python3/src/python3.wasm2.rs
