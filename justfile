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

# Runs all tests under the Miri interpreter; requires Rust nightly.
test_miri:
    cargo +nightly miri test --workspace

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg doc_cfg" cargo +nightly doc {{FLAGS}}
