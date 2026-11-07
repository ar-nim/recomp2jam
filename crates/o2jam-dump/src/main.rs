//! `o2jam-dump` — inspect O2Jam asset files: magic sniffing, OJM
//! sample lists, OPI/OPA entry lists, bounded hexdumps.

mod dump_ojm;
mod dump_ojn;

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
    /// Pretty-print an `.ojm` container: variant, entry table, streams.
    DumpOjm {
        /// Container file to inspect.
        file: PathBuf,
    },
    /// Pretty-print an `.ojn` chart: header map, levels, kinds, notes.
    DumpOjn {
        /// Chart file to inspect.
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Identify { file } => run_identify(&file),
        Command::Samples { file } => run_samples(&file),
        Command::Opi { file } => run_opi(&file),
        Command::Hex { file, len } => run_hex(&file, len),
        Command::DumpOjm { file } => run_dump_ojm(&file),
        Command::DumpOjn { file } => run_dump_ojn(&file),
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

/// Pretty-print an `.ojm` container via the verified parsers.
fn run_dump_ojm(file: &PathBuf) -> Result<()> {
    let bytes = read_input(file)?;
    let report = dump_ojm::format_ojm_report(&file.display().to_string(), &bytes)?;
    print!("{report}");
    Ok(())
}

/// Pretty-print an `.ojn` chart via the verified parsers.
fn run_dump_ojn(file: &PathBuf) -> Result<()> {
    let bytes = read_input(file)?;
    let report = dump_ojn::format_ojn_report(&file.display().to_string(), &bytes)?;
    print!("{report}");
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
