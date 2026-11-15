//! `o2jam-dump` — inspect O2Jam asset files: magic sniffing, OJM
//! sample lists, OPI/OPA entry lists, bounded hexdumps.

mod dump_ojm;
mod dump_ojn;
mod rect_query;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

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
    /// Resolve native coords for a BND control id from staged
    /// placements.
    #[command(alias = "chrome", alias = "query", alias = "coords")]
    Rect {
        /// BND control id (e.g. `IDC_...`).
        #[arg(value_name = "CONTROL_ID_OR_ARCHIVE")]
        first: Option<String>,
        /// Second positional: control id or archive path.
        #[arg(value_name = "CONTROL_ID_OR_ARCHIVE")]
        second: Option<String>,
        /// BND control id (alternative to positionals).
        #[arg(long)]
        id: Option<String>,
        /// Archive to query.
        #[arg(long, short = 'a')]
        archive: Option<PathBuf>,
        /// Install root to read archives from.
        #[arg(long)]
        root: Option<PathBuf>,
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
        Command::Rect {
            first,
            second,
            id,
            archive,
            root,
        } => run_rect_query(first, second, id, archive, root),
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

/// Resolve a control id + archive path from the flexible positional
/// / flag mix.
fn resolve_rect_query(
    first: Option<String>,
    second: Option<String>,
    id_flag: Option<String>,
    archive_flag: Option<PathBuf>,
    root_flag: Option<PathBuf>,
) -> Result<(String, Vec<u8>)> {
    // Resolve the control id.
    let mut id = id_flag;
    let mut archive_path = archive_flag;
    if let (Some(a), Some(b)) = (first.clone(), second.clone()) {
        if a.starts_with("IDC_") && !b.starts_with("IDC_") {
            if id.is_none() {
                id = Some(a);
            }
            if archive_path.is_none() {
                archive_path = Some(PathBuf::from(b));
            }
        } else if !a.starts_with("IDC_") && b.starts_with("IDC_") {
            if archive_path.is_none() {
                archive_path = Some(PathBuf::from(a));
            }
            if id.is_none() {
                id = Some(b);
            }
        } else if a.starts_with("IDC_") && b.starts_with("IDC_") {
            if id.is_none() {
                id = Some(a);
            }
        } else {
            // both look like paths: first archive, second ignored for id
            if archive_path.is_none() {
                archive_path = Some(PathBuf::from(a));
            }
        }
    } else if let Some(a) = first.clone() {
        if a.starts_with("IDC_") {
            if id.is_none() {
                id = Some(a);
            }
        } else if archive_path.is_none() {
            archive_path = Some(PathBuf::from(a));
        }
    }
    let id = id.ok_or_else(|| anyhow::anyhow!("missing control id"))?;

    // Resolve archive bytes.
    let bytes = if let Some(path) = archive_path {
        rect_query::read_archive(&path)?
    } else if let Some(root) = root_flag {
        read_default_archive(&root)?
    } else {
        // Default install root candidates.
        let mut tried = Vec::new();
        let mut found = None;
        for candidate in default_roots() {
            if let Ok(bytes) = read_default_archive(&candidate) {
                found = Some(bytes);
                break;
            }
            tried.push(candidate.display().to_string());
        }
        found.ok_or_else(|| anyhow::anyhow!("no archive found; tried: {}", tried.join(", ")))?
    };
    Ok((id, bytes))
}

fn default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(root) = std::env::var("O2JAM_INSTALL_ROOT") {
        roots.push(PathBuf::from(root));
    }
    roots.push(PathBuf::from("install-root"));
    // Executable-anchored (target/debug -> workspace root).
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            roots.push(ancestor.join("install-root"));
        }
    }
    // CARGO_MANIFEST_DIR-anchored (repo layout).
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let base = PathBuf::from(manifest);
        if let Some(parent) = base.parent().and_then(|p| p.parent()) {
            roots.push(parent.join("install-root"));
        }
        roots.push(base.join("..").join("..").join("install-root"));
    }
    roots
}

fn read_default_archive(root: &Path) -> Result<Vec<u8>> {
    // Prefer Interface archive; fall back to Playing.
    let interface = root.join("O2Jam/Image/Interface1.opi");
    if interface.exists() {
        return read_input(&interface);
    }
    let playing = root.join("O2Jam/Image/Playing1.opi");
    if playing.exists() {
        return read_input(&playing);
    }
    anyhow::bail!("no archive under {}", root.display())
}

/// Print native coords for a BND control id.
fn run_rect_query(
    first: Option<String>,
    second: Option<String>,
    id: Option<String>,
    archive: Option<PathBuf>,
    root: Option<PathBuf>,
) -> Result<()> {
    let (control_id, bytes) = resolve_rect_query(first, second, id, archive, root)?;
    let rect = rect_query::query_control_rect(&bytes, &control_id)?;
    println!(
        "{}: block={} x={} y={} w={} h={} (x1={},y1={},x2={},y2={})",
        control_id, rect.block, rect.x, rect.y, rect.w, rect.h, rect.x1, rect.y1, rect.x2, rect.y2,
    );
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
