alias f := fmt
alias d := doc
alias t := test

check toolchain='+stable': fmt
    cargo {{toolchain}} clippy
    cargo {{toolchain}} clippy --package wasm2rs-rt --no-default-features --features alloc,merged
    cargo {{toolchain}} clippy --package wasm2rs-rt --no-default-features --features merged

fmt toolchain='+stable':
    cargo {{toolchain}} fmt

test toolchain='+stable':
    cargo {{toolchain}} test --workspace

# Quickly runs all tests; requires nextest.
test_fast toolchain='+stable':
    cargo {{toolchain}} nextest run --workspace

# Runs all tests under the Miri interpreter; requires Rust nightly and nextest.
test_miri:
    # miri - https://github.com/rust-lang/miri
    # nextest -  https://github.com/nextest-rs/nextest
    cargo +nightly miri nextest run --workspace

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg doc_cfg" cargo +nightly doc {{FLAGS}}
