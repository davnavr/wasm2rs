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
    /// Translates and executes WebAssembly specification tests.
    Test {
        /// Specifies the `.wast` files to translate and execute.
        #[arg(short, long)]
        input: Vec<std::path::PathBuf>,
        /// If set, the generated Rust source code is not executed.
        #[arg(long)]
        no_run: bool,
    },
}

pub fn main() -> anyhow::Result<()> {
    use anyhow::Context;

    let arguments = <Arguments as clap::Parser>::parse();

    #[cfg(feature = "rayon")]
    {
        rayon::ThreadPoolBuilder::new()
            .num_threads(arguments.threads)
            .build_global()
            .context("unable to create global thread pool")?;
    }

    match arguments.command {
        Command::Convert { input, output } => {
            let wasm =
                wat::parse_file(&input).with_context(|| format!("could not parse {input:?}"))?;

            let output_path = output.unwrap_or_else(|| input.with_extension("wasm.rs"));
            let out = std::fs::File::create(&output_path)
                .with_context(|| format!("could not create output file {output_path:?}"))?;

            wasm2rs_convert::Convert::new()
                .convert_from_buffer(&wasm, &mut std::io::BufWriter::with_capacity(4096, out))?;

            Ok(())
        }
        Command::Test {
            input: _,
            no_run: _,
        } => anyhow::bail!("specification tests are not yet supported"),
    }
}
