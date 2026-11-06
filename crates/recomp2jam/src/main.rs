//! `recomp2jam` — the game client binary.
//!
//! Day-0: CLI only (`--help`, `info`, `play --autoplay` stub). The windowed
//! client (winit + wgpu + cpal) attaches here once file parsing is verified.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod connect;

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
    /// Connect to a Memoryer gateway and run the login flow.
    Connect {
        /// Gateway host (Memoryer `Server` address).
        #[arg(long)]
        host: String,
        /// Gateway TCP port (15010 in-exe default).
        #[arg(long, default_value_t = o2jam_net::DEFAULT_GATEWAY_PORT)]
        port: u16,
        /// Account id.
        #[arg(long)]
        user_id: i32,
        /// Username.
        #[arg(long)]
        token: String,
        /// Account gender (male, female, any).
        #[arg(long, default_value = "any")]
        gender: String,
        /// Relay session key 1 (launcher auth; 0 without one).
        #[arg(long, default_value_t = 0)]
        relay_key1: i32,
        /// Relay session key 2 (launcher auth; 0 without one).
        #[arg(long, default_value_t = 0)]
        relay_key2: i32,
        /// Gateway id to enter (needs --channel-id too).
        #[arg(long, requires = "channel_id")]
        gateway_id: Option<i16>,
        /// Channel id to enter (needs --gateway-id too).
        #[arg(long, requires = "gateway_id")]
        channel_id: Option<i16>,
        /// Fetch character info after channel login (needs --channel-id too).
        #[arg(long, requires = "channel_id")]
        character: bool,
    },
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
        Command::Connect {
            host,
            port,
            user_id,
            token,
            gender,
            relay_key1,
            relay_key2,
            gateway_id,
            channel_id,
            character,
        } => {
            let options = connect::ConnectOptions {
                host,
                port,
                user_id,
                token,
                gender: connect::parse_gender(&gender)?,
                relay_key1,
                relay_key2,
                channel: gateway_id.zip(channel_id),
                character,
            };
            let report = connect::connect_and_login(&options)?;
            println!("authorize: {}", connect::name_result(report.auth_code));
            if let Some(hero) = &report.character {
                println!(
                    "character: {} (Lv {}, {:?}, {} XP)",
                    hero.nickname, hero.level, hero.gender, hero.experience
                );
            }
            println!("channels: {}", report.channels.len());
            for channel in &report.channels {
                println!(
                    "  gateway {} channel {}: {}/{} {}",
                    channel.gateway_id,
                    channel.channel_id,
                    channel.population,
                    channel.capacity,
                    if channel.active { "open" } else { "closed" },
                );
            }
            if let Some(login) = &report.channel_login {
                let restricted = match login.restricted {
                    Some(true) => "restricted",
                    Some(false) => "unrestricted",
                    None => "restriction unknown",
                };
                println!(
                    "channel: {} ({restricted})",
                    connect::name_result(login.result_code)
                );
            }
        }
        Command::Play {
            chart,
            ojm,
            level,
            sound,
            window,
        } => {
            tracing::info!(?chart, ?window, "play stub");
            if window {
                let mut game = o2jam_shell::ScreenGame::new();
                if let Some(path) = &chart {
                    let (level_chart, _) = load_level_chart(path, level)?;
                    game.load_chart(
                        level_chart
                            .hit_onsets_ms()
                            .into_iter()
                            .map(|(lane, at_ms, _)| (lane, at_ms))
                            .collect(),
                    );
                    tracing::info!(hits = level_chart.hit_count(), "chart loaded");
                }
                let event_loop = winit::event_loop::EventLoop::new()
                    .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
                o2jam_shell::Shell::new(Box::new(game)).run(event_loop)?;
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

/// Build the strike-slot -> decoded-PCM bank for autoplay.
///
/// M30 entries key by their sample id (verified exe behavior). Banks
/// whose entries carry no id (old-variant OMC streams, new-variant
/// mode-2/mode-3 entries) key by fetch order via
/// [`o2jam_file::OjmTables::resolve_order_slot`] — the documented
/// reconstruction fallback (`docs/file-formats/ojm.md`): in-exe all
/// such entries collapse onto slot 0, so order is the only key that
/// lets autoplay sound them. `format_version` is the chart's `+8`
/// float: it selects the new-variant lane count
/// ([`o2jam_file::ojm_descriptor_for_version`]); OGG bodies (mode-3
/// splits) decode with Vorbis, encrypted bodies with
/// [`o2jam_file::decode_new_variant_pcm`] into the entry header's
/// rate/channels.
/// Decode one OGG body into the bank, skipping undecodable streams.
fn insert_ogg(
    bank: &mut std::collections::HashMap<u32, o2jam_audio::PcmSamples>,
    slot: u32,
    ogg: &[u8],
) {
    if let Ok(pcm) = o2jam_audio::decode_vorbis(ogg) {
        bank.insert(slot, pcm);
    }
}

/// Wrap decoded PCM frames with the entry header's rate/channels.
fn insert_pcm(
    bank: &mut std::collections::HashMap<u32, o2jam_audio::PcmSamples>,
    slot: u32,
    entry: &o2jam_file::OjmEntry,
    samples: Vec<i16>,
) {
    if entry.sample_rate > 0 && entry.num_channels > 0 && !samples.is_empty() {
        bank.insert(
            slot,
            o2jam_audio::PcmSamples {
                samples,
                sample_rate: entry.sample_rate,
                channels: entry.num_channels as u8,
            },
        );
    }
}

fn load_sound_bank(
    bank_bytes: &[u8],
    format_version: f32,
) -> std::collections::HashMap<u32, o2jam_audio::PcmSamples> {
    use o2jam_file::{OjmEntry, NEW_VARIANT_HIGH_BANK};
    let mut bank = std::collections::HashMap::new();
    if bank_bytes.starts_with(b"M30") {
        if let Ok(samples) = o2jam_file::parse_m30(bank_bytes) {
            for sample in samples {
                if let Some(id) = sample.sample_id {
                    insert_ogg(&mut bank, id, sample.data);
                }
            }
        }
        return bank;
    }
    // New-variant first: its entry walk is bounds-checked, and each
    // body must decode (OGG checksum or PCM frames), so garbage
    // headers fall through to the old-variant walk below.
    if let Ok(tables) = o2jam_file::parse_new_variant_entries(bank_bytes) {
        let descriptor = o2jam_file::ojm_descriptor_for_version(format_version);
        let flat: Vec<&OjmEntry> = tables.mode2.iter().chain(tables.mode3.iter()).collect();
        let mut decoded_any = false;
        for (order, entry) in flat.iter().enumerate() {
            let Ok(body) = entry.sample_bytes(bank_bytes) else {
                continue;
            };
            let slot = order as u32;
            if body.starts_with(b"OggS") {
                insert_ogg(&mut bank, slot, body);
            } else if let Ok(frames) = o2jam_file::decode_new_variant_pcm(body, descriptor) {
                insert_pcm(&mut bank, slot, entry, frames);
            }
            decoded_any = decoded_any || bank.contains_key(&slot);
        }
        if decoded_any {
            // High-bank aliases serve bit26-origin strikes, which name
            // the same fetch-order entries (see `resolve_order_slot`).
            let slots: Vec<u32> = bank.keys().copied().collect();
            for slot in slots {
                if let Some(pcm) = bank.get(&slot).cloned() {
                    bank.insert(NEW_VARIANT_HIGH_BANK + slot, pcm);
                }
            }
            return bank;
        }
    }
    if let Ok(samples) = o2jam_file::parse_omc(bank_bytes) {
        for (order, sample) in samples.iter().enumerate() {
            insert_ogg(&mut bank, order as u32, sample.data);
        }
    }
    bank
}

/// Load one chart level from disk (chart + slice checks shared by
/// autoplay and the windowed game).
fn load_level_chart(chart_path: &str, level: usize) -> Result<(o2jam_game::LevelChart, f32)> {
    use anyhow::Context;
    let chart_bytes =
        std::fs::read(chart_path).with_context(|| format!("reading chart {chart_path}"))?;
    let header = o2jam_file::parse_ojn(&chart_bytes)?;
    let slices = o2jam_file::parse_ojn_levels(&chart_bytes)?;
    if slices.get(level).is_none() {
        anyhow::bail!("level {level} missing (chart has {} slices)", slices.len());
    }
    let events = o2jam_file::parse_ojn_level_events(&chart_bytes, level)?;
    let chart = o2jam_game::build_level_chart(header.bpm, &events);
    Ok((chart, header.format_version))
}

/// Headless autoplay: load chart + bank, step a virtual clock (or the
/// wall clock with `--sound`), trigger key sounds, judge perfect.
fn run_autoplay(chart_path: &str, ojm_path: Option<&str>, level: usize, sound: bool) -> Result<()> {
    use anyhow::Context;
    use o2jam_game::{Judgement, Score, Sequencer};
    let (chart, format_version) = load_level_chart(chart_path, level)?;
    tracing::info!(hits = chart.hit_count(), "chart loaded");
    let mut bank = std::collections::HashMap::new();
    if let Some(path) = ojm_path {
        let bank_bytes = std::fs::read(path).with_context(|| format!("reading bank {path}"))?;
        bank = load_sound_bank(&bank_bytes, format_version);
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
    let mut pills = o2jam_game::PillStock::default();
    let mut streak = o2jam_game::CoolStreak::default();
    let mut sounded = 0u64;
    let mut silent = 0u64;
    // Autoplay judges everything Cool; the streak feeds pills exactly
    // as the 7.55 score path does.
    let judge_cool = |score: &mut Score,
                      pills: &mut o2jam_game::PillStock,
                      streak: &mut o2jam_game::CoolStreak| {
        streak.note(Judgement::Cool, pills);
        score.apply_rated(Judgement::Cool, pills);
    };
    if sound {
        let clock = o2jam_game::SongClock::start();
        while !sequencer.is_empty() {
            let now = clock.now_ms();
            for strike in sequencer.pop_due(now) {
                judge_cool(&mut score, &mut pills, &mut streak);
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
                judge_cool(&mut score, &mut pills, &mut streak);
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
