//! `o2jam-dump` — inspect O2Jam asset files.
//!
//! Day-0: `identify` (magic sniff) + bounded hexdump. Pretty printers for
//! OJM/OJN arrive with verified parsers.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "o2jam-dump", version, about = "Inspect O2Jam 7.55 asset files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Identify a file's container kind by magic.
    Identify {
        /// File to inspect.
        file: PathBuf,
    },
    /// Hexdump the first N bytes (default 64).
    Hex {
        /// File to inspect.
        file: PathBuf,
        /// Bytes to show.
        #[arg(long, default_value = "64")]
        len: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Identify { file } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            match o2jam_file::detect_kind(&bytes) {
                Ok(kind) => println!("{}: {kind:?}", file.display()),
                Err(e) => println!("{}: unrecognised ({e})", file.display()),
            }
        }
        Command::Hex { file, len } => {
            let bytes =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let len = len.min(bytes.len());
            for (i, chunk) in bytes[..len].chunks(16).enumerate() {
                print!("{:08x}: ", i * 16);
                for b in chunk {
                    print!("{b:02x} ");
                }
                println!();
            }
            println!("({} of {} bytes shown)", len, bytes.len());
        }
    }
    Ok(())
}
