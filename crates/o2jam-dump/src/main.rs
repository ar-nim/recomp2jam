//! `o2jam-dump` — inspect O2Jam asset files: magic sniffing, OJM
//! sample lists, OPI/OPA entry lists, bounded hexdumps.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "o2jam-dump", version, about = "Inspect O2Jam 7.55 asset files", long_about = None)]
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
    /// List sample streams in an `.ojm` container (M30/OMC).
    Samples {
        /// Container file to inspect.
        file: PathBuf,
    },
    /// List entries in an `.opi`/`.opa` archive.
    Opi {
        /// Archive file to inspect.
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
        Command::Identify { file } => run_identify(&file),
        Command::Samples { file } => run_samples(&file),
        Command::Opi { file } => run_opi(&file),
        Command::Hex { file, len } => run_hex(&file, len),
    }
}

/// Read one input file with a labeled error.
fn read_input(file: &PathBuf) -> Result<Vec<u8>> {
    std::fs::read(file).with_context(|| format!("reading {}", file.display()))
}

/// Report a file's container kind (unrecognized is not an error).
fn run_identify(file: &PathBuf) -> Result<()> {
    let bytes = read_input(file)?;
    match o2jam_file::detect_kind(&bytes) {
        Ok(kind) => println!("{}: {kind:?}", file.display()),
        Err(e) => println!("{}: unrecognised ({e})", file.display()),
    }
    Ok(())
}

/// List sample streams in an `.ojm` container.
fn run_samples(file: &PathBuf) -> Result<()> {
    let bytes = read_input(file)?;
    let samples = o2jam_file::parse_ojm(&bytes)?;
    println!("{}: {} samples", file.display(), samples.len());
    for s in &samples {
        let id = match s.sample_id {
            Some(id) => id.to_string(),
            None => "-".to_string(),
        };
        println!("  id={id:>5} {:>9}B {}", s.data.len(), s.name);
    }
    Ok(())
}

/// List entries in an `.opi`/`.opa` archive.
fn run_opi(file: &PathBuf) -> Result<()> {
    let bytes = read_input(file)?;
    let slots = o2jam_file::parse_opi_slots(&bytes)?;
    let files = slots.iter().filter(|s| !s.is_free()).count();
    println!(
        "{}: {} slots ({} files, {} free)",
        file.display(),
        slots.len(),
        files,
        slots.len() - files
    );
    for (i, s) in slots.iter().enumerate() {
        if s.is_free() {
            println!(
                "  [slot {i}] kind={} free reservation off={} aux={}B",
                s.kind, s.data_offset, s.reserved_size
            );
        } else {
            println!(
                "  [slot {i}] kind={} off={} {:>9}B +{:>4}B {}",
                s.kind, s.data_offset, s.size, s.reserved_size, s.name
            );
        }
    }
    Ok(())
}

/// Hexdump the first `len` bytes.
fn run_hex(file: &PathBuf, len: usize) -> Result<()> {
    let bytes = read_input(file)?;
    let len = len.min(bytes.len());
    for (i, chunk) in bytes[..len].chunks(16).enumerate() {
        print!("{:08x}: ", i * 16);
        for b in chunk {
            print!("{b:02x} ");
        }
        println!();
    }
    println!("({} of {} bytes shown)", len, bytes.len());
    Ok(())
}
