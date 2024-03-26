alias f := fmt
alias d := doc
alias t := test

check toolchain='+stable': fmt
    cargo {{toolchain}} clippy
    cargo {{toolchain}} clippy --package wasm2rs-rt --no-default-features --features alloc
    cargo {{toolchain}} clippy --package wasm2rs-rt --no-default-features

fmt toolchain='+stable':
    cargo {{toolchain}} fmt

test toolchain='+stable':
    cargo {{toolchain}} test --workspace

# Quickly compiles and runs all tests; requires Rust nightly and nextest.
test_fast threads:
    RUSTFLAGS='-Zthreads={{threads}}' \
    NEXTEST_TEST_THREADS='{{threads}}' \
    cargo +nightly nextest run --workspace

# Runs all tests under the Miri interpreter; requires Rust nightly and nextest.
test_miri:
    # miri - https://github.com/rust-lang/miri
    # nextest -  https://github.com/nextest-rs/nextest
    cargo +nightly miri nextest run --workspace

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg doc_cfg" cargo +nightly doc {{FLAGS}}
