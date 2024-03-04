alias f := fmt
alias d := doc

check: fmt
    cargo clippy

fmt:
    cargo fmt

# Generate documentation; requires Rust nightly.
doc *FLAGS='--all-features':
    RUSTDOCFLAGS="--cfg doc_cfg" cargo +nightly doc {{FLAGS}}
