use anyhow::{Context, Result};

fn main() -> Result<()> {
    let out_dir =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").context("OUT_DIR was not set")?);

    let suite_dir = {
        let mut manifest_dir = std::path::PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR was not set")?,
        );

        manifest_dir.push("testsuite");
        manifest_dir
    };

    const TESTS: &[&str] = &[
        "address", // corresponds to ./testsuite/address.wast
        "align",
        // "block", // TODO: blocked on `call_indirect` support.
        // "br_if",
        "conversions",
        // "data", // TODO: blocked on `global.get` in constant expressions.
        "endianness",
        "fac",
        "forward",
        "i64",
        "int_exprs",
        "int_literals",
        "labels",
        "memory_init",
        // "start", // TODO: Blocked on using imports in spec tests
        "switch",
        "traps",
        "unwind",
    ];

    let mut all_file = String::with_capacity(1024);
    let mut test_files = Vec::with_capacity(TESTS.len());
    for test_name in TESTS.iter().copied() {
        let wast_path = {
            let mut dir = suite_dir.join(test_name);
            dir.set_extension("wast");
            dir
        };

        let rs_dir = out_dir.join(test_name);
        let rs_path = {
            let mut dir = rs_dir.clone();
            dir.set_extension("rs");
            dir
        };

        {
            use std::fmt::Write;
            writeln!(all_file, "include!({rs_path:?});")?;
        }

        match std::fs::create_dir(&rs_dir) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => {
                return Err(anyhow::Error::new(e)
                    .context(format!("could not create output directory {rs_dir:?}")))
            }
        }

        test_files.push(wasm2rs_spectest::TestFile {
            input: wast_path,
            output_file: rs_path,
            output_dir: rs_dir,
        });
    }

    let (warnings, result) = wasm2rs_spectest::translate(&test_files);

    for message in warnings {
        println!("cargo::warning={message}");
    }

    result?;

    let all_file_path = out_dir.join("all.rs");
    std::fs::write(&all_file_path, all_file)
        .with_context(|| format!("could not write {all_file_path:?}"))?;

    Ok(())
}
