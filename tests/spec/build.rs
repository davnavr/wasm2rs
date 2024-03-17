fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let suite_dir = {
        let mut manifest_dir =
            std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        manifest_dir.push("testsuite");
        manifest_dir
    };

    let all_file_path = out_dir.as_path().join("all.rs");
    let mut all_file = std::fs::File::create(&all_file_path)
        .unwrap_or_else(|e| panic!("could not create file {all_file_path:?}: {e}"));

    const FILES: &[&str] = &["int_exprs.wast", "int_literals.wast"];

    let mut file_buffer = String::with_capacity(0x20000);
    for wast_name in FILES {
        use std::io::Write as _;

        let wast_path = suite_dir.as_path().join(wast_name);

        let wast_text = {
            file_buffer.clear();
            let file = &mut std::fs::File::open(&wast_path)
                .unwrap_or_else(|e| panic!("could not open test file {wast_path:?}: {e}"));

            std::io::Read::read_to_string(file, &mut file_buffer)
                .unwrap_or_else(|e| panic!("could not read test file {wast_path:?}: {e}"));

            file_buffer.as_str()
        };

        let wast_buf = wast::parser::ParseBuffer::new(wast_text)
            .unwrap_or_else(|e| panic!("could not lex test file {wast_path:?}: {e}"));

        let wast = wast::parser::parse::<wast::Wast>(&wast_buf)
            .unwrap_or_else(|e| panic!("could not parse test file {wast_path:?}: {e}"));

        let rs_file_name = format!("{}.rs", wast_path.file_stem().unwrap().to_str().unwrap());

        // Path to the output translated Rust file.
        let rs_path = out_dir.join(&rs_file_name);

        let mut rs_file = std::fs::File::create(&rs_path)
            .unwrap_or_else(|e| panic!("could not create file {rs_path:?}: {e}"));

        let _ = writeln!(&mut rs_file, "// Generated from {wast_path:?}");

        for directive in wast.directives {
            // Generation of test cases is blocked on 1) export name mangling 2) renaming of generated module
            let (line, col) = directive.span().linecol_in(wast_text);
            println!(
                "cargo:warning={}:{line}:{col} : unsupported directive was skipped",
                wast_path.display()
            );
        }
    }
}
