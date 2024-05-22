//! The `wasm2rs` command line interface.

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Specifies the maximum number of threads to use.
    ///
    /// If set to zero, then the number of threads used is dependent on the `RAYON_NUM_THREADS`
    /// environment variable, or the number of logical CPUs.
    #[arg(long, default_value_t = 0)]
    #[cfg(feature = "rayon")]
    threads: usize,
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Translates a WebAssembly module into `.rs` source code.
    Convert {
        /// Path to the WebAssembly binary module.
        ///
        /// This can be either in the binary format (`.wasm`) or the text format (`.wat`).
        #[arg(short, long)]
        input: std::path::PathBuf,
        /// Path to the Rust source code file.
        ///
        /// If not specified, defaults to the name of the input file with the extension changed.
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Translates WebAssembly specification tests.
    #[cfg(feature = "test-utils")]
    Test {
        /// Specifies the `.wast` files to translate.
        #[arg(short, long)]
        input: Vec<std::path::PathBuf>,
        /// The directory that will contain the generated Rust source code.
        #[arg(long)]
        output_directory: std::path::PathBuf,
    },
}

pub fn main() -> anyhow::Result<std::process::ExitCode> {
    use anyhow::Context;

    let arguments = <Arguments as clap::Parser>::parse();

    #[cfg(feature = "rayon")]
    {
        rayon::ThreadPoolBuilder::new()
            .num_threads(arguments.threads)
            .build_global()
            .context("unable to create global thread pool")?;
    }

    let exit_code = match arguments.command {
        Command::Convert { input, output } => {
            let wasm =
                wat::parse_file(&input).with_context(|| format!("could not parse {input:?}"))?;

            let output_path = output.unwrap_or_else(|| input.with_extension("wasm.rs"));
            let out = std::fs::File::create(&output_path)
                .with_context(|| format!("could not create output file {output_path:?}"))?;

            wasm2rs_convert::Convert::new()
                .convert_from_buffer(&wasm, &mut std::io::BufWriter::with_capacity(4096, out))?;

            std::process::ExitCode::SUCCESS
        }
        #[cfg(feature = "test-utils")]
        Command::Test {
            input,
            output_directory,
        } => {
            let options = wasm2rs_convert::Convert::new();

            let input_files = input
                .into_iter()
                .map(|path| wasm2rs_convert_wast::TestFile {
                    input: path.into(),
                    options: &options,
                })
                .collect::<Vec<_>>();

            let output = wasm2rs_convert_wast::DirectoryOutput::existing(&output_directory);

            match wasm2rs_convert_wast::convert_to_output(&input_files, &output) {
                Ok(_) => std::process::ExitCode::SUCCESS,
                Err(errors) => {
                    use std::io::Write as _;

                    let mut stderr = std::io::stderr().lock();

                    for err in errors.into_iter() {
                        let _ = writeln!(&mut stderr, "{err}");
                    }

                    std::process::ExitCode::FAILURE
                }
            }
        }
    };

    Ok(exit_code)
}
