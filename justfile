alias f := fmt
alias d := doc
alias t := test

check: fmt
    cargo clippy
    cargo clippy --package wasm2rs-rt --no-default-features --features alloc
    cargo clippy --package wasm2rs-rt --no-default-features

fmt:
    cargo fmt

test:
    cargo test --workspace

# Runs all tests under the Miri interpreter; requires Rust nightly and nextest.
test_miri:
    # miri - https://github.com/rust-lang/miri
    # nextest -  https://github.com/nextest-rs/nextest
    cargo +nightly miri nextest run --workspace

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg doc_cfg" cargo +nightly doc {{FLAGS}}
