//! Compile a chart schedule into the binary feed read by dinput.dll.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use di_interposer::{build_feed, encode_feed, parse_schedule};

#[derive(Parser)]
#[command(
    name = "di-interposer-feed",
    about = "Compile a <ms>:<dik> chart schedule into the DirectInput interposer feed"
)]
struct Arguments {
    /// Chart schedule, one `<song_ms>:<dik>[,<dik>...]` group per line.
    #[arg(long)]
    schedule: PathBuf,

    /// Wall-clock Unix epoch in milliseconds that song time zero maps to.
    #[arg(long)]
    epoch_ms: u64,

    /// How long each DIK stays pressed, in milliseconds.
    #[arg(long, default_value_t = 80)]
    hold_ms: u32,

    /// Destination of the binary feed.
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();
    let text = fs::read_to_string(&arguments.schedule)
        .with_context(|| format!("reading {}", arguments.schedule.display()))?;
    let schedule = parse_schedule(&text)
        .with_context(|| format!("parsing {}", arguments.schedule.display()))?;
    let entries = build_feed(&schedule, arguments.epoch_ms, arguments.hold_ms);
    let bytes = encode_feed(&entries).context("encoding DirectInput feed")?;
    fs::write(&arguments.out, &bytes)
        .with_context(|| format!("writing {}", arguments.out.display()))?;
    println!(
        "groups={} entries={} bytes={} out={}",
        schedule.groups.len(),
        entries.len(),
        bytes.len(),
        arguments.out.display()
    );
    Ok(())
}
