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
        /// OJM bank for key sounds (`.ojm`; old-variant OGG only).
        #[arg(long)]
        ojm: Option<String>,
        /// Difficulty level (0-based; default 0).
        #[arg(long, default_value = "0")]
        level: usize,
        /// Play sound through the default output (needs audio hardware).
        #[arg(long)]
        sound: bool,
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
        Command::Play {
            chart,
            ojm,
            level,
            sound,
            window,
        } => {
            tracing::info!(?chart, ?window, "play stub");
            if window && chart.is_none() {
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
                return Ok(());
            }
            let chart_path = chart.ok_or_else(|| {
                anyhow::anyhow!("--chart FILE.ojn is required (add --window for the empty shell)")
            })?;
            run_autoplay(&chart_path, ojm.as_deref(), level, sound)?;
        }
    }
    Ok(())
}

/// Headless autoplay: load chart + bank, step a virtual clock (or the
/// wall clock with `--sound`), trigger key sounds, judge perfect.
fn run_autoplay(chart_path: &str, ojm_path: Option<&str>, level: usize, sound: bool) -> Result<()> {
    use anyhow::Context;
    use o2jam_game::{Judgement, Score, Sequencer};
    let chart_bytes =
        std::fs::read(chart_path).with_context(|| format!("reading chart {chart_path}"))?;
    let header = o2jam_file::parse_ojn(&chart_bytes)?;
    let slices = o2jam_file::parse_ojn_levels(&chart_bytes)?;
    let slice = slices.get(level).ok_or_else(|| {
        anyhow::anyhow!("level {level} missing (chart has {} slices)", slices.len())
    })?;
    let events = o2jam_file::parse_ojn_level_events(&chart_bytes, level)?;
    let chart = o2jam_game::build_level_chart(header.bpm, &events);
    tracing::info!(
        hits = chart.hit_count(),
        slice = ?slice,
        "chart loaded"
    );
    let mut bank = std::collections::HashMap::new();
    if let Some(path) = ojm_path {
        let bank_bytes = std::fs::read(path).with_context(|| format!("reading bank {path}"))?;
        for sample in o2jam_file::parse_ojm(&bank_bytes)? {
            if let Some(id) = sample.sample_id {
                if let Ok(pcm) = o2jam_audio::decode_vorbis(sample.data) {
                    bank.insert(id, pcm);
                }
            }
        }
        tracing::info!(samples = bank.len(), "bank decoded");
    }
    let mut backend: Box<dyn o2jam_audio::AudioBackend> = if sound {
        match o2jam_audio::CpalBackend::try_new() {
            Ok(backend) => Box::new(backend),
            Err(e) => {
                eprintln!("audio unavailable ({e}); continuing silent");
                Box::new(o2jam_audio::NullBackend::new())
            }
        }
    } else {
        Box::new(o2jam_audio::NullBackend::new())
    };
    let mut sequencer = Sequencer::new(&chart);
    let mut score = Score::default();
    let mut sounded = 0u64;
    let mut silent = 0u64;
    if sound {
        let clock = o2jam_game::SongClock::start();
        while !sequencer.is_empty() {
            let now = clock.now_ms();
            for strike in sequencer.pop_due(now) {
                score.apply(Judgement::Cool);
                if let Some(pcm) = bank.get(&strike.sample_id) {
                    backend.play_pcm(pcm)?;
                    sounded += 1;
                } else {
                    silent += 1;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    } else {
        let mut now = 0u64;
        while !sequencer.is_empty() {
            for strike in sequencer.pop_due(now) {
                score.apply(Judgement::Cool);
                if bank.contains_key(&strike.sample_id) {
                    sounded += 1;
                } else {
                    silent += 1;
                }
            }
            now += 5;
        }
    }
    println!(
        "autoplay: {} hits+backing, sounded {sounded}, silent {silent}, score {} (max combo {})",
        score.counts.iter().sum::<u32>(),
        score.points,
        score.max_combo
    );
    Ok(())
}
