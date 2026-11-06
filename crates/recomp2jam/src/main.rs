//! `recomp2jam` — the game client binary.
//!
//! Day-0: CLI only (`--help`, `info`, `play --autoplay` stub). The windowed
//! client (winit + wgpu + cpal) attaches here once file parsing is verified.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod connect;
mod loading;

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
        /// Stack the worn avatar figure from this `avatar.opa`
        /// (needs --character too).
        #[arg(long, requires = "character")]
        avatar_opa: Option<String>,
        /// Room number to join after channel login (needs --channel-id).
        #[arg(long, requires = "channel_id")]
        join_room: Option<i32>,
        /// Password for `--join-room`.
        #[arg(long, default_value = "")]
        room_password: String,
        /// Room song id: runs the host song flow after the room
        /// join (needs --join-room; the room must let us host).
        #[arg(long, requires = "join_room")]
        music_id: Option<u16>,
        /// Chart difficulty code (0 EX, 1 NX, 2 HX, 3 RX).
        #[arg(long, default_value_t = 2, requires = "music_id")]
        difficulty: u8,
        /// Scroll-speed code (select + loaded + tally).
        #[arg(long, default_value_t = 1, requires = "music_id")]
        game_speed: u8,
        /// Room slot the sync steps belong to.
        #[arg(long, default_value_t = 0, requires = "music_id")]
        member_id: u8,
        /// Power skill-ring id for the loaded confirm.
        #[arg(long, default_value_t = 0, requires = "music_id")]
        power_skill: i32,
        /// Tick channel: life or jam.
        #[arg(long, default_value = "jam", requires = "music_id")]
        stat_kind: String,
        /// Channel value for the tick.
        #[arg(long, default_value_t = 3, requires = "music_id")]
        stat_value: u16,
        /// Hit sequence number for the tick.
        #[arg(long, default_value_t = 1, requires = "music_id")]
        stat_seq: i32,
        /// Running score for the tick.
        #[arg(long, default_value_t = 210, requires = "music_id")]
        stat_score: u32,
        /// Cool hits for the tally.
        #[arg(long, default_value_t = 10, requires = "music_id")]
        cool: u16,
        /// Good hits for the tally.
        #[arg(long, default_value_t = 2, requires = "music_id")]
        good: u16,
        /// Bad hits for the tally.
        #[arg(long, default_value_t = 0, requires = "music_id")]
        bad: u16,
        /// Misses for the tally.
        #[arg(long, default_value_t = 0, requires = "music_id")]
        miss: u16,
        /// Best combo for the tally.
        #[arg(long, default_value_t = 12, requires = "music_id")]
        max_combo: u16,
        /// Final jam level for the tally.
        #[arg(long, default_value_t = 1, requires = "music_id")]
        jam: u16,
        /// Best jam level for the tally.
        #[arg(long, default_value_t = 1, requires = "music_id")]
        max_jam: u16,
        /// Final score for the tally.
        #[arg(long, default_value_t = 2210, requires = "music_id")]
        final_score: u32,
        /// Remaining life for the tally.
        #[arg(long, default_value_t = 100, requires = "music_id")]
        life: u8,
        /// Long-note bonus total (tick and tally).
        #[arg(long, default_value_t = 0, requires = "music_id")]
        long_note: i32,
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
        /// Stack the result-screen avatar from this `avatar.opa`.
        #[arg(long)]
        avatar_opa: Option<String>,
        /// Equipped item ids for `--avatar-opa` (`"1,5,9"`).
        #[arg(long, default_value = "")]
        equip: String,
        /// Wear the male sprite variants.
        #[arg(long)]
        avatar_male: bool,
        /// Load the splash backdrop from this `.opi` archive
        /// (`O2mediaLogo.ojs`).
        #[arg(long)]
        interface: Option<String>,
        /// List live channels in the window (needs --host etc.).
        #[arg(long)]
        online: bool,
        /// Gateway host for `--online`.
        #[arg(long, requires = "online")]
        host: Option<String>,
        /// Account id for `--online`.
        #[arg(long, requires = "online")]
        user_id: Option<i32>,
        /// Username for `--online`.
        #[arg(long, requires = "online")]
        token: Option<String>,
        /// Gateway TCP port for `--online`.
        #[arg(long, default_value_t = o2jam_net::DEFAULT_GATEWAY_PORT)]
        port: u16,
        /// Gateway id to enter for `--online` (needs --channel-id too).
        #[arg(long, requires = "online_channel_id")]
        online_gateway_id: Option<i16>,
        /// Channel id to enter for `--online` (needs --online-gateway-id too).
        #[arg(long, requires = "online_gateway_id")]
        online_channel_id: Option<i16>,
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
    run_command(cli.command.unwrap_or(Command::Info))
}

/// Build info for `recomp2jam info`.
fn run_info() {
    println!("recomp2jam {}", env!("CARGO_PKG_VERSION"));
    println!("target: O2Jam 7.55 client (C++/CDX/Boost/libvorbis/MSVC2005) -> Rust");
    println!("status: day-0 scaffold; gameplay renderers pending file-parser verification");
}

/// Dispatch one CLI subcommand.
///
/// # Errors
///
/// Whatever the subcommand reports (net, file, audio, window).
fn run_command(command: Command) -> Result<()> {
    match command {
        Command::Info => {
            run_info();
            Ok(())
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
            avatar_opa,
            join_room,
            room_password,
            music_id,
            difficulty,
            game_speed,
            member_id,
            power_skill,
            stat_kind,
            stat_value,
            stat_seq,
            stat_score,
            cool,
            good,
            bad,
            miss,
            max_combo,
            jam,
            max_jam,
            final_score,
            life,
            long_note,
        } => {
            let song = match music_id {
                Some(id) => {
                    let mut flow = connect::SongFlowOptions::new(id);
                    flow.difficulty = difficulty;
                    flow.speed = game_speed;
                    flow.member_id = member_id;
                    flow.power_skill_id = power_skill;
                    flow.stat_kind = connect::parse_stat_kind(&stat_kind)?;
                    flow.stat_value = stat_value;
                    flow.stat_sequence = stat_seq;
                    flow.stat_score = stat_score;
                    flow.cool = cool;
                    flow.good = good;
                    flow.bad = bad;
                    flow.miss = miss;
                    flow.max_combo = max_combo;
                    flow.jam = jam;
                    flow.max_jam = max_jam;
                    flow.final_score = final_score;
                    flow.life = life;
                    flow.long_note_score = long_note;
                    Some(flow)
                }
                None => None,
            };
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
                avatar_opa,
                join_room,
                room_password,
                song,
            };
            let report = connect::connect_and_login(&options)?;
            print_connect_report(&report);
            Ok(())
        }
        Command::Play {
            chart,
            ojm,
            level,
            sound,
            window,
            avatar_opa,
            equip,
            avatar_male,
            interface,
            online,
            host,
            user_id,
            token,
            port,
            online_gateway_id,
            online_channel_id,
        } => run_play(
            chart,
            ojm,
            level,
            sound,
            window,
            avatar_opa,
            equip,
            avatar_male,
            interface,
            online,
            host,
            user_id,
            token,
            port,
            online_gateway_id,
            online_channel_id,
        ),
    }
}

/// `play` subcommand: windowed game or headless autoplay.
///
/// # Errors
///
/// Whatever chart/bank loading, login, audio, or the window reports.
#[allow(clippy::too_many_arguments, reason = "CLI fields pass through once")]
fn run_play(
    chart: Option<String>,
    ojm: Option<String>,
    level: usize,
    sound: bool,
    window: bool,
    avatar_opa: Option<String>,
    equip: String,
    avatar_male: bool,
    interface: Option<String>,
    online: bool,
    host: Option<String>,
    user_id: Option<i32>,
    token: Option<String>,
    port: u16,
    online_gateway_id: Option<i16>,
    online_channel_id: Option<i16>,
) -> Result<()> {
    tracing::info!(?chart, ?window, "play stub");
    if window {
        let mut game = o2jam_shell::ScreenGame::new();
        setup_splash(&mut game, &interface)?;
        setup_online(
            &mut game,
            online,
            &host,
            user_id,
            &token,
            port,
            online_gateway_id,
            online_channel_id,
        )?;
        setup_avatar(&mut game, &avatar_opa, &equip, avatar_male)?;
        setup_chart_audio(&mut game, &chart, &ojm, level)?;
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
        o2jam_shell::Shell::new(Box::new(game)).run(event_loop)?;
        return Ok(());
    }
    let chart_path = chart.ok_or_else(|| {
        anyhow::anyhow!("--chart FILE.ojn is required (add --window for the empty shell)")
    })?;
    run_autoplay(&chart_path, ojm.as_deref(), level, sound)
}

/// Load the splash backdrop into a windowed game, if requested.
fn setup_splash(game: &mut o2jam_shell::ScreenGame, interface: &Option<String>) -> Result<()> {
    if let Some(opi_path) = interface {
        let logo = connect::load_interface_sprite(opi_path, "O2mediaLogo.ojs")?;
        tracing::info!(layers = logo.len(), "splash art loaded");
        game.load_splash(logo);
    }
    Ok(())
}

/// Authenticate and list channels/rooms into a windowed game.
#[allow(clippy::too_many_arguments, reason = "CLI fields pass through once")]
fn setup_online(
    game: &mut o2jam_shell::ScreenGame,
    online: bool,
    host: &Option<String>,
    user_id: Option<i32>,
    token: &Option<String>,
    port: u16,
    online_gateway_id: Option<i16>,
    online_channel_id: Option<i16>,
) -> Result<()> {
    if !online {
        return Ok(());
    }
    let options = connect::ConnectOptions {
        host: host.clone().unwrap_or_default(),
        port,
        user_id: user_id.unwrap_or_default(),
        token: token.clone().unwrap_or_default(),
        gender: o2jam_net::Gender::Any,
        relay_key1: 0,
        relay_key2: 0,
        channel: online_gateway_id.zip(online_channel_id),
        character: false,
        avatar_opa: None,
        join_room: None,
        room_password: String::new(),
        song: None,
    };
    let report = connect::connect_and_login(&options)?;
    tracing::info!(channels = report.channels.len(), "channels listed");
    game.set_channels(report.channels);
    if !report.rooms.is_empty() {
        tracing::info!(rooms = report.rooms.len(), "rooms listed");
        game.set_rooms(report.rooms);
    }
    Ok(())
}

/// Stack the worn avatar figure into a windowed game, if requested.
fn setup_avatar(
    game: &mut o2jam_shell::ScreenGame,
    avatar_opa: &Option<String>,
    equip: &str,
    avatar_male: bool,
) -> Result<()> {
    if let Some(opa_path) = avatar_opa {
        let ids = connect::parse_equipment(equip)?;
        let figure = connect::build_worn_figure(opa_path, &ids, avatar_male)?;
        tracing::info!(layers = figure.len(), "avatar stacked");
        game.load_avatar(figure);
    }
    Ok(())
}

/// Load a chart plus its key sounds into a windowed game, if requested.
fn setup_chart_audio(
    game: &mut o2jam_shell::ScreenGame,
    chart: &Option<String>,
    ojm: &Option<String>,
    level: usize,
) -> Result<()> {
    if let Some(path) = chart {
        let (level_chart, format_version) = load_level_chart(path, level)?;
        game.load_chart(level_chart.hit_onsets_ms());
        tracing::info!(hits = level_chart.hit_count(), "chart loaded");
        if let Some(bank_path) = ojm {
            let bank_bytes = std::fs::read(bank_path)?;
            let bank = load_sound_bank(&bank_bytes, format_version);
            let backend: Box<dyn o2jam_audio::AudioBackend> =
                match o2jam_audio::CpalBackend::try_new() {
                    Ok(backend) => Box::new(backend),
                    Err(e) => {
                        eprintln!("audio unavailable ({e}); continuing silent");
                        Box::new(o2jam_audio::NullBackend::new())
                    }
                };
            game.with_audio(backend, bank);
            tracing::info!("key sounds attached");
        }
    }
    Ok(())
}

/// Print one login-flow report (authorize, character, avatar, channels,
/// rooms, login, join). Pure output, infallible.
fn print_connect_report(report: &connect::ConnectReport) {
    println!("authorize: {}", connect::name_result(report.auth_code));
    if let Some(hero) = &report.character {
        println!(
            "character: {} (Lv {}, {:?}, {} XP)",
            hero.nickname, hero.level, hero.gender, hero.experience
        );
    }
    if report.avatar_layers > 0 {
        println!("avatar: {} layers worn", report.avatar_layers);
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
    let open_rooms: Vec<_> = report
        .rooms
        .iter()
        .filter(|room| room.number != 0)
        .collect();
    println!("rooms: {} open", open_rooms.len());
    for room in open_rooms.iter().take(10) {
        println!(
            "  #{} {:?} {}/{}{}",
            room.number,
            room.title,
            room.user_count,
            room.capacity,
            if room.has_password { " locked" } else { "" },
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
    if let Some(join) = &report.join {
        match &join.room {
            Some(room) => println!(
                "room: {} seats, {} in ({})",
                room.slots.len(),
                room.user_count,
                room.title
            ),
            None => println!("room: joined without snapshot"),
        }
    }
    if let Some(song) = &report.song {
        println!(
            "music: {} entries (free music: {})",
            song.music_entries,
            if song.free_music { "yes" } else { "no" },
        );
        match &song.peer_bar {
            Some(peer) => println!(
                "tick: member {} {:?} = {}",
                peer.member_id,
                peer.kind(),
                peer.value,
            ),
            None => println!("tick: no peer echo"),
        }
        if !song.skipped.is_empty() {
            println!("skipped broadcasts: {:?}", song.skipped);
        }
    }
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
///
/// Loading gate 4 (`0x54d460`): only mono/stereo decodes enter the bank.
fn insert_ogg(
    bank: &mut std::collections::HashMap<u32, o2jam_audio::PcmSamples>,
    slot: u32,
    ogg: &[u8],
) {
    if let Ok(pcm) = o2jam_audio::decode_vorbis(ogg) {
        if o2jam_file::validate_decoded_channels(pcm.channels).is_ok() {
            bank.insert(slot, pcm);
        }
    }
}

/// Wrap decoded PCM frames with the entry header's rate/channels.
///
/// Loading gate 4 (`0x54d460`): only mono/stereo streams enter the bank.
fn insert_pcm(
    bank: &mut std::collections::HashMap<u32, o2jam_audio::PcmSamples>,
    slot: u32,
    entry: &o2jam_file::OjmEntry,
    samples: Vec<i16>,
) {
    if o2jam_file::validate_decoded_channels(entry.num_channels as u8).is_err() {
        return;
    }
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
            // Codec gate (`0x54d460`): mode-2 entries carry header
            // WAV params, so reject bad shapes before decode;
            // mode-3 splits carry none and gate post-decode instead.
            if order < tables.mode2.len() && loading::check_bank_entry(entry).is_err() {
                continue;
            }
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
///
/// Runs the lifecycle §9 loading gate: select validation is the
/// level-slot range check (`0x5718b0`), staging logs the
/// `MUSIC\Temp\%s%d_%d` extract name (`0x4f8b80`), the open goes
/// through `loading::load_gated_level` (`0x412180`), and the return
/// arms the song clock downstream (`0x4e47a0`).
fn load_level_chart(chart_path: &str, level: usize) -> Result<(o2jam_game::LevelChart, f32)> {
    use anyhow::Context;
    let chart_bytes =
        std::fs::read(chart_path).with_context(|| format!("reading chart {chart_path}"))?;
    let prepared = loading::prepare_chart(&chart_bytes, level)
        .with_context(|| format!("loading gate rejected {chart_path} level {level}"))?;
    let events = o2jam_file::parse_ojn_level_events(&chart_bytes, level)?;
    let chart = o2jam_game::build_level_chart(prepared.bpm, &events);
    let stem = std::path::Path::new(chart_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(chart_path);
    let selection = loading::stage_offline_select(prepared.song_id, level, stem);
    tracing::info!(
        extract = %selection.extract_name,
        level,
        "staged select (0x4f8b80)"
    );
    tracing::info!("{}", loading::handoff_note(&selection, &prepared));
    Ok((chart, prepared.format_version))
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
    // Result grades stay OPEN until an in-exe reference pins the
    // thresholds (`gameplay.md` win rule); never print a grade here.
    if loading::RESULT_GRADES_VERIFIED {
        println!("grades: exe-verified");
    } else {
        println!("grades: OPEN (unimplemented, no in-exe refs)");
    }
    Ok(())
}
