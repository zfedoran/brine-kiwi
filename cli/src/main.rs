use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

use brine_kiwi_compiler::{compile_schema, compile_schema_to_rust, decode_binary_schema};
use brine_kiwi_compiler::error::KiwiError;
use brine_kiwi::decode_to_json;

#[derive(Parser)]
#[command(name = "brine-kiwi-cli")]
#[command(about = "Compile, decode, or generate Rust from Kiwi schemas", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a `.kiwi` IDL file to a binary `.kiwi.bin`
    Compile {
        /// Input `.kiwi` file
        #[arg(short, long)]
        input: PathBuf,

        /// Output `.kiwi.bin` file (defaults to same name + `.kiwi.bin`)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Decode a `.kiwi.bin` file to JSON (printed to stdout)
    Decode {
        /// Input `.kiwi.bin` file
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Generate Rust code from a `.kiwi` schema, by calling `compile_schema_to_rust`
    GenRust {
        /// Input `.kiwi` schema file
        #[arg(short, long)]
        input: PathBuf,

        /// Output `.rs` file (if omitted, prints to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<(), KiwiError> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile { input, output } => {
            // Read .kiwi text
            let text = fs::read_to_string(input).map_err(KiwiError::Io)?;
            // compile_schema → (Schema, Vec<u8>)
            let (_schema, bin) = compile_schema(&text)?;
            // Determine output path
            let out_path = if let Some(o) = output {
                o.clone()
            } else {
                let mut p = input.clone();
                p.set_extension("kiwi.bin");
                p
            };
            // Write .kiwi.bin
            fs::write(&out_path, &bin).map_err(KiwiError::Io)?;
            println!("Compiled {} → {}", input.display(), out_path.display());
            Ok(())
        }

        Commands::Decode { input } => {
            // Read binary
            let data = fs::read(input).map_err(KiwiError::Io)?;
            // Decode to Schema (and ignore it here)
            let _schema = decode_binary_schema(&data)?;
            // Pretty-print JSON
            let json = decode_to_json(&data)?;
            println!("{}", json);
            Ok(())
        }

        Commands::GenRust { input, output } => {
            // Read .kiwi text
            let text = fs::read_to_string(input).map_err(KiwiError::Io)?;
            // Run compile_schema so parsing, verification, etc. all occur
            let (schema, _bin) = compile_schema(&text)?;
            // Generate Rust source
            let rust_code = compile_schema_to_rust(&schema);
            if let Some(out_path) = output {
                fs::write(out_path, &rust_code).map_err(KiwiError::Io)?;
                println!("Generated Rust code written to {}", out_path.display());
            } else {
                println!("Generated Rust code:\n\n{}", rust_code);
            }
            Ok(())
        }
    }
}
