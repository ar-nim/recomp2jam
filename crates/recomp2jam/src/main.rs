//! `recomp2jam` — the game client binary.
//!
//! Day-0: CLI only (`--help`, `info`, `play --autoplay` stub). The windowed
//! client (winit + wgpu + cpal) attaches here once file parsing is verified.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "recomp2jam",
    version,
    about = "O2Jam 7.55 recompilation (Rust)",
    long_about = None
)]
struct Cli {
    /// Increase log verbosity (-v, -vv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show build info and original-stack mapping.
    Info,
    /// Stub gameplay entry point (autoplay with null backend for now).
    Play {
        /// Chart file to load (`.ojn`, verified parsers pending).
        #[arg(long)]
        chart: Option<String>,
        /// Open the game window (needs a display; headless CI skips it).
        #[arg(long)]
        window: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .ok();

    match cli.command.unwrap_or(Command::Info) {
        Command::Info => {
            println!("recomp2jam {}", env!("CARGO_PKG_VERSION"));
            println!("target: O2Jam 7.55 client (C++/CDX/Boost/libvorbis/MSVC2005) -> Rust");
            println!("status: day-0 scaffold; gameplay renderers pending file-parser verification");
        }
        Command::Play { chart, window } => {
            tracing::info!(?chart, ?window, "play stub");
            if !window {
                println!(
                    "play: stub — chart parsers pending (got: {chart:?}). See docs/progress.md"
                );
                return Ok(());
            }
            struct StubGame;
            impl o2jam_shell::GameLoop for StubGame {
                fn lane_key(&mut self, lane: usize, down: bool) {
                    tracing::info!(lane, down, "lane key");
                }
                fn redraw(&mut self) {}
            }
            let event_loop = winit::event_loop::EventLoop::new()
                .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
            o2jam_shell::Shell::new(Box::new(StubGame)).run(event_loop)?;
        }
    }
    Ok(())
}
