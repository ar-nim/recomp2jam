//! `recomp2jam` — the game client binary.
//!
//! Day-0: CLI only (`--help`, `info`, `play --autoplay` stub). The windowed
//! client (winit + wgpu + cpal) attaches here once file parsing is verified.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod connect;
#[cfg(test)]
mod data_dir_guard;
mod e2e;
mod ledger_check;
mod live_art;
#[cfg(test)]
mod live_boot_gate;
mod live_dual_client_legs;
#[cfg(test)]
mod live_formats_legs;
#[cfg(test)]
mod live_guard;
mod live_input_legs;
#[cfg(test)]
mod live_leg_support;
mod live_messenger_legs;
mod live_myroom_legs;
mod live_payment_legs;
mod live_planet_channel_legs;
mod live_playing_input_legs;
mod live_playing_render_legs;
mod live_playing_shell_legs;
mod live_playing_wire_legs;
mod live_result_input_legs;
mod live_result_legs;
mod live_room_chat_legs;
mod live_room_input_legs;
mod live_room_seat_legs;
mod live_room_shell_legs;
mod live_room_wire_legs;
mod live_session_tail_legs;
mod live_shop_control_legs;
mod live_stats;
mod live_waiting_seat_legs;
mod live_waiting_wire_legs;
mod live_web_native_legs;
#[cfg(test)]
mod live_window_legs;
mod loading;
mod menu_trace_replay;
mod relay_gate;
mod session;
mod session_walk;
#[cfg(test)]
mod shop_open_bench;
mod shots;
mod trace;

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
        /// RSA launch blob whose account fields fill the 1000 frame
        /// (overrides `--user-id`, `--token`, and `--gender`).
        #[arg(long)]
        launch_blob: Option<String>,
        /// Keep the authorized transport open for N seconds after login
        /// (rig interop: lets two clients share a room on one server).
        #[arg(long, default_value_t = 0)]
        hold_after_join: u64,
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
        /// Fetch wallet balances after character info (needs --character too).
        #[arg(long, requires = "character")]
        wallet: bool,
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
        /// Room stage word: sent as `SetRoomArena` 4002 after the
        /// song select (needs --music-id; the gateway echoes 4003).
        #[arg(long, default_value_t = 0, requires = "music_id")]
        arena_id: i32,
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
    /// Drive `ScreenGame` in-process and write scene PNGs (no window).
    Shots {
        /// Directory for `<scene>.png` files.
        #[arg(long, default_value = "docs/screenshots")]
        out_dir: String,
    },
    /// Windowed-session walk: live online play per stage with PNGs.
    SessionWalk {
        /// Gateway host (local Memoryer).
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Gateway TCP port.
        #[arg(long, default_value_t = o2jam_net::DEFAULT_GATEWAY_PORT)]
        port: u16,
        /// Account id.
        #[arg(long, default_value_t = 1)]
        user_id: i32,
        /// Session token (gateway session guid).
        #[arg(long)]
        token: String,
        /// Directory for `<stage>.png` files.
        #[arg(long, default_value = "docs/screenshots/session")]
        out_dir: String,
    },
    /// Windowed online mode: log in, then walk the real window
    /// (U1): the login flow runs from CLI args (ours-by-design — the
    /// exe takes launcher cmdline), the lobby walk rides real window
    /// input.
    Online {
        /// Gateway host (local Memoryer).
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Gateway TCP port.
        #[arg(long, default_value_t = o2jam_net::DEFAULT_GATEWAY_PORT)]
        port: u16,
        /// Account id.
        #[arg(long, default_value_t = 1)]
        user_id: i32,
        /// Session token (gateway session guid).
        #[arg(long)]
        token: String,
        /// Account gender (male, female, any).
        #[arg(long, default_value = "any")]
        gender: String,
        /// Gateway id to enter (needs --channel-id too).
        #[arg(long, requires = "channel_id")]
        gateway_id: Option<i16>,
        /// Channel id to enter (needs --gateway-id too).
        #[arg(long, requires = "gateway_id")]
        channel_id: Option<i16>,
        /// Create this room on startup and host it (the U1 walk:
        /// a fresh local Memoryer lists no rooms, so the trace hosts).
        #[arg(long)]
        create_room: Option<String>,
        /// Join this room on Room Confirm instead (needs the room to
        /// exist; alternative to --create-room).
        #[arg(long, conflicts_with = "create_room")]
        join_room: Option<i32>,
        /// Password for `--join-room`.
        #[arg(long, default_value = "")]
        room_password: String,
        /// Host song pick: sent on Waiting Confirm (4000/4002/4030
        /// plus the catalog chart fetch).
        #[arg(long)]
        music_id: Option<u16>,
        /// Chart difficulty code (0 EX, 1 NX, 2 HX, 3 RX).
        #[arg(long, default_value_t = 2)]
        difficulty: u8,
        /// Scroll-speed code (select + loaded + tally).
        #[arg(long, default_value_t = 1)]
        speed: u8,
        /// Room stage word (`SetRoomArena` 4002).
        #[arg(long, default_value_t = 0)]
        arena: i32,
        /// Load the splash backdrop from this `.opi` archive.
        #[arg(long)]
        interface: Option<String>,
        /// Silence the audio rig (default: sound plays through the
        /// default output; automated rigs and CI pass this so runs
        /// never reach the speakers).
        #[arg(long)]
        no_sound: bool,
        /// Trace-harness affordance: skip the relay probe and keep
        /// the original channel path for input-routing probes. The
        /// default boot always applies the #244 gate.
        #[arg(long)]
        no_relay_gate: bool,
        /// Fixed-clock test affordance: advance the virtual clock by
        /// this many ms per painted frame (deterministic windowed
        /// traces without an audio device).
        #[arg(long)]
        fixed_clock: Option<u64>,
        /// Frame-capture directory: one PNG per gated paint (screen
        /// change or content change, heartbeat-bounded).
        #[arg(long, requires = "input_log")]
        frame_dump: Option<String>,
        /// Applied-event log (one line per routed event plus one
        /// `frame` line per paint). Test affordance.
        #[arg(long)]
        input_log: Option<String>,
    },
    /// Validate one recomp2jam-input-v1 trace (schema, ordering,
    /// pairs, bounds) without running it.
    ValidateTrace {
        /// Trace JSON path.
        #[arg(long)]
        trace: String,
    },
    /// Replay one shop/MyRoom menu trace twice through the
    /// ScreenGame snapshot seam (synthetic staging, deterministic
    /// repeat check with state-hash comparison).
    ReplayMenuTrace {
        /// Trace JSON path.
        #[arg(long)]
        trace: String,
        /// Synthetic staging family: shop, myroom-equip, myroom-gift.
        #[arg(long)]
        stage: String,
    },
    /// End-to-end window walk: boot to Result through every landed
    /// State with one PNG per step plus `trace.json` (headless: the
    /// same `ScreenGame` seam the winit window runs, null audio).
    E2e {
        /// Evidence directory for `<seq>-<action>.png` plus `trace.json`.
        #[arg(long, default_value = "/home/arnim/scratch/e2e-walk")]
        out_dir: String,
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
        /// Silence the audio rig (default: sound plays through the
        /// default output; automated runs pass this).
        #[arg(long)]
        no_sound: bool,
        /// Open the game window (needs a display; headless CI skips it).
        #[arg(long)]
        window: bool,
        /// Frame-capture directory: one PNG per gated paint (screen
        /// change or content change, heartbeat-bounded).
        #[arg(long, requires = "input_log")]
        frame_dump: Option<String>,
        /// Applied-event log (one line per routed event plus one
        /// `frame` line per paint). Test affordance.
        #[arg(long)]
        input_log: Option<String>,
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
        /// RSA launch blob whose account fields fill the 1000 frame
        /// (overrides `--user-id`, `--token`, and gender).
        #[arg(long, requires = "online")]
        online_launch_blob: Option<String>,
        /// Gateway TCP port for `--online`.
        #[arg(long, default_value_t = o2jam_net::DEFAULT_GATEWAY_PORT)]
        port: u16,
        /// Gateway id to enter for `--online` (needs --channel-id too).
        #[arg(long, requires = "online_channel_id")]
        online_gateway_id: Option<i16>,
        /// Channel id to enter for `--online` (needs --online-gateway-id too).
        #[arg(long, requires = "online_gateway_id")]
        online_channel_id: Option<i16>,
        /// Room number to join on Room Confirm for `--online`
        /// (the session exchanges `JoinWaiting` for it).
        #[arg(long, requires = "online")]
        online_join_room: Option<i32>,
        /// Password for `--online-join-room` (empty when unlocked).
        #[arg(long, default_value = "", requires = "online")]
        online_room_password: String,
        /// Room title to create on `--online` startup (the session runs
        /// the proven `CreateRoom` host exchange, then stages its snapshot).
        #[arg(long, requires = "online")]
        online_create_room: Option<String>,
        /// Host song pick for `--online` (the session sends it on
        /// Waiting Confirm: 4000/4002/4030 plus the catalog chart fetch).
        #[arg(long, requires = "online")]
        online_music_id: Option<u16>,
        /// Difficulty code for `--online-music-id` (0 EX, 1 NX, 2 HX, 3 RX).
        #[arg(long, default_value_t = 2, requires = "online")]
        online_difficulty: u8,
        /// Scroll-speed code for `--online-music-id`.
        #[arg(long, default_value_t = 1, requires = "online")]
        online_speed: u8,
        /// Room stage word for `--online-music-id` (`SetRoomArena` 4002).
        #[arg(long, default_value_t = 0, requires = "online")]
        online_arena: i32,
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

/// CLI fields that assemble one host song flow (`connect --music-id ...`).
///
/// Parameter object (clean-code §2/§6): one named bundle instead of a
/// 20-argument builder, moved out of `run_command` for unit tests.
#[derive(Debug, Clone)]
struct SongFlowParams {
    music_id: Option<u16>,
    difficulty: u8,
    game_speed: u8,
    arena_id: i32,
    member_id: u8,
    power_skill: i32,
    stat_kind: String,
    stat_value: u16,
    stat_seq: i32,
    stat_score: u32,
    cool: u16,
    good: u16,
    bad: u16,
    miss: u16,
    max_combo: u16,
    jam: u16,
    max_jam: u16,
    final_score: u32,
    life: u8,
    long_note: i32,
}

impl SongFlowParams {
    /// Build the host song flow (`None` without `--music-id`).
    ///
    /// # Errors
    ///
    /// The `--stat-kind` word is unknown.
    fn into_song_flow(self) -> Result<Option<connect::SongFlowOptions>> {
        let Some(music_id) = self.music_id else {
            return Ok(None);
        };
        let mut flow = connect::SongFlowOptions::new(music_id, installed_music_state(music_id));
        flow.difficulty = self.difficulty;
        flow.speed = self.game_speed;
        flow.arena_id = self.arena_id;
        flow.member_id = self.member_id;
        flow.power_skill_id = self.power_skill;
        flow.stat_kind = connect::parse_stat_kind(&self.stat_kind)?;
        flow.stat_value = self.stat_value;
        flow.stat_sequence = self.stat_seq;
        flow.stat_score = self.stat_score;
        flow.cool = self.cool;
        flow.good = self.good;
        flow.bad = self.bad;
        flow.miss = self.miss;
        flow.max_combo = self.max_combo;
        flow.jam = self.jam;
        flow.max_jam = self.max_jam;
        flow.final_score = self.final_score;
        flow.life = self.life;
        flow.long_note_score = self.long_note;
        Ok(Some(flow))
    }
}

/// CLI fields that assemble [`connect::ConnectOptions`]
/// (`connect --host ...`).
#[derive(Debug, Clone)]
struct ConnectParams {
    host: String,
    port: u16,
    user_id: i32,
    token: String,
    gender: String,
    launch_blob: Option<String>,
    hold_after_join: u64,
    relay_key1: i32,
    relay_key2: i32,
    gateway_id: Option<i16>,
    channel_id: Option<i16>,
    character: bool,
    wallet: bool,
    avatar_opa: Option<String>,
    join_room: Option<i32>,
    room_password: String,
}

/// Override the hand-built login identity with the exe blob's account
/// fields. Both CLI entry points use this seam so the 1000 frame carries
/// the same user id, token, and gender as the external client's argv.
///
/// # Errors
///
/// The blob cannot be decrypted or its `UserIndexId` is not an `i32`.
fn apply_launch_blob(options: &mut connect::ConnectOptions, blob: Option<&str>) -> Result<()> {
    let Some(blob) = blob else {
        return Ok(());
    };
    let identity = o2jam_net::AuthParameters::launch_identity_from_blob(blob)
        .context("reading launch blob identity")?;
    options.user_id = identity.user_id;
    options.token = identity.token;
    options.gender = identity.gender;
    Ok(())
}

impl ConnectParams {
    /// Assemble the login options, attaching a prebuilt song flow.
    ///
    /// # Errors
    ///
    /// The `--gender` word is unknown.
    fn into_connect_options(
        self,
        song: Option<connect::SongFlowOptions>,
    ) -> Result<connect::ConnectOptions> {
        let mut options = connect::ConnectOptions {
            host: self.host,
            port: self.port,
            user_id: self.user_id,
            token: self.token,
            gender: connect::parse_gender(&self.gender)?,
            relay_key1: self.relay_key1,
            relay_key2: self.relay_key2,
            channel: self.gateway_id.zip(self.channel_id),
            character: self.character,
            wallet: self.wallet,
            avatar_opa: self.avatar_opa,
            join_room: self.join_room,
            room_password: self.room_password,
            song,
        };
        apply_launch_blob(&mut options, self.launch_blob.as_deref())?;
        Ok(options)
    }
}

/// `online` subcommand fields (one named bundle, no flag-argument
/// soup): the U1 windowed online walk — CLI login, then real-window
/// lobby navigation driven by the trace harness.
#[derive(Debug, Clone)]
struct OnlineCli {
    host: String,
    port: u16,
    user_id: i32,
    token: String,
    gender: String,
    gateway_id: Option<i16>,
    channel_id: Option<i16>,
    create_room: Option<String>,
    join_room: Option<i32>,
    room_password: String,
    music_id: Option<u16>,
    difficulty: u8,
    speed: u8,
    arena: i32,
    interface: Option<String>,
    no_sound: bool,
    no_relay_gate: bool,
    fixed_clock: Option<u64>,
    frame_dump: Option<String>,
    input_log: Option<String>,
}

/// `online` subcommand: run the pinned login flow
/// ([`connect::connect_and_login_with_transport`], docs/net-protocol.md
/// §4c), assemble the windowed [`session::OnlineSession`], and run it
/// in the winit loop with the trace-harness capture affordances.
///
/// # Errors
///
/// Whatever login, room setup, or the window reports.
fn run_online(opts: OnlineCli) -> Result<()> {
    let mut game = o2jam_shell::ScreenGame::new();
    game.load_options(&install_root());
    setup_splash(&mut game, &opts.interface)?;
    live_art::stage_relay_down_from_install(&mut game);
    let connect_options = connect::ConnectOptions {
        host: opts.host.clone(),
        port: opts.port,
        user_id: opts.user_id,
        token: opts.token.clone(),
        gender: connect::parse_gender(&opts.gender)?,
        relay_key1: 0,
        relay_key2: 0,
        channel: opts.gateway_id.zip(opts.channel_id),
        // Character info needs channel membership (server-verified),
        // so the live boot fetches it exactly when it logs into a
        // channel (ticket #58).
        character: opts.gateway_id.zip(opts.channel_id).is_some(),
        wallet: false,
        avatar_opa: None,
        join_room: None,
        room_password: String::new(),
        song: None,
    };
    let owned_ids = owned_music_ids();
    tracing::info!(ids = owned_ids.len(), "room-stage music ids staged");
    // Probe before any channel-list exchange: the exe's relay check
    // sits on the reconnect that precedes `03EA` (files `0x8e340`/
    // `0x474630`, capture `~/scratch/p244-capture`).
    let relay_down = !opts.no_relay_gate && relay_gate::relay_down_now();
    let (report, gateway) =
        connect::connect_and_login_with_room_stage(&connect_options, &owned_ids)?;
    tracing::info!(channels = report.channels.len(), "channels listed");
    stage_online_identity(&mut game, report.character.as_ref());
    stage_online_wallet(&mut game, report.cash_sync.as_ref());
    // 4031 acquired ids drive the Music Shop right panel. The v3
    // pair had the 4031 answer on the wire but the rust board stayed
    // row-less while the exe showed titles and download status; join
    // the boot answer to the catalog titles (#878).
    if let Some(answer) = report.music_list {
        game.stage_music_shop_board(answer);
    }
    stage_channel_list_after_relay_gate(&mut game, report.channels, relay_down);
    stage_online_rooms(&mut game, report.rooms);
    let pending_room = opts
        .join_room
        .map(|number| (number, opts.room_password.clone()));
    let pending_song = opts.music_id.map(|music_id| session::SongChoice {
        music_id,
        difficulty: opts.difficulty,
        speed: opts.speed,
        arena_id: opts.arena,
    });
    let mut session = assemble_online_session(
        game,
        gateway,
        Some(&opts.token),
        pending_room,
        pending_song,
        opts.create_room.as_deref(),
    )?;
    // Audio containment (memory 979fb): the windowed live boot is the
    // one arm that unfences the session, and only when the run did not
    // pass --no-sound. Every other arm keeps the fence `new` starts
    // with (#283).
    session.set_silent_audio(opts.no_sound);
    if let Some((gw, ch)) = opts.gateway_id.zip(opts.channel_id) {
        session.set_resident_channel(o2jam_net::ChannelState {
            gateway_id: gw as u16,
            channel_id: ch as u16,
            capacity: 0,
            population: 0,
            active: true,
        });
    }
    if let Some(step_ms) = opts.fixed_clock {
        session.game_mut().enable_fixed_clock(step_ms);
    }
    session.attach_initial_audio(!opts.no_sound);
    run_windowed_session(session, capture_plan(opts.frame_dump, opts.input_log))
}

/// Assemble the windowed online session shared by `play --window
/// --online` and `online`: pair the transport with the game, stage
/// the local username, the Room Confirm join intent, the host song
/// pick, the song catalog, the live scene art, and — when named — a
/// hosted room created through the proven `CreateRoom` exchange.
///
/// # Errors
///
/// A rejected create, a missing room row, or whatever the gateway
/// transport reports.
fn assemble_online_session(
    game: o2jam_shell::ScreenGame,
    transport: o2jam_net::GatewayConnection,
    local_user: Option<&str>,
    pending_room: Option<(i32, String)>,
    pending_song: Option<session::SongChoice>,
    create_room: Option<&str>,
) -> Result<session::OnlineSession> {
    let mut session = session::OnlineSession::new(game, transport);
    if let Some(name) = local_user.filter(|name| !name.is_empty()) {
        session.set_local_user(name);
    }
    if let Some((number, password)) = pending_room {
        session.set_pending_room_join(number, &password);
    }
    if let Some(choice) = pending_song {
        session.set_pending_song(choice);
    }
    stage_session_catalog(&mut session);
    session.stage_live_art();
    // Live art replaces the Loading figures, so resolve the cover after it.
    session.stage_pending_song_selection();
    if let Some(title) = create_room {
        let room = session.create_and_join_room(title)?;
        tracing::info!(title = room.title, "room created");
    }
    Ok(session)
}

/// Wrap a session in the winit shell and run it, attaching the
/// trace-harness capture plan when requested (frame dumps + the
/// applied-event log; test affordances, never gameplay input).
///
/// # Errors
///
/// Whatever the event loop or the paint backend reports.
fn run_windowed_session(
    session: session::OnlineSession,
    capture: Option<o2jam_shell::CapturePlan>,
) -> Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()
        .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
    let shell = o2jam_shell::Shell::new(Box::new(session));
    let shell = match capture {
        Some(plan) => shell.with_capture(plan),
        None => shell,
    };
    shell.run(event_loop)
}

/// Build the capture plan from the `--frame-dump`/`--input-log` pair
/// (`None` without a dump directory; the plan's gates bound capture
/// volume during Playing scroll).
fn capture_plan(
    frame_dump: Option<String>,
    input_log: Option<String>,
) -> Option<o2jam_shell::CapturePlan> {
    // Either arm arms the plan: `--input-log` alone must not be a silent
    // no-op (a live-probe incident: three sessions "proved" no-input while
    // the log was never wired).
    if frame_dump.is_none() && input_log.is_none() {
        return None;
    }
    Some(o2jam_shell::CapturePlan {
        dir: frame_dump.map(std::path::PathBuf::from).unwrap_or_default(),
        min_paint_gap: 25,
        heartbeat_paints: 600,
        input_log: input_log.map(std::path::PathBuf::from),
    })
}

/// CLI fields that assemble [`connect::ConnectOptions`] for
/// `play --online` (list-only at startup: no channel entry,
/// character, avatar, or song; the room join rides
/// [`OnlineParams::pending_room_join`] into the session, which
/// exchanges it on Room Confirm).
#[derive(Debug, Clone, Default)]
struct OnlineParams {
    host: Option<String>,
    user_id: Option<i32>,
    token: Option<String>,
    port: u16,
    online_gateway_id: Option<i16>,
    online_channel_id: Option<i16>,
    join_room: Option<i32>,
    room_password: String,
    music_id: Option<u16>,
    difficulty: u8,
    speed: u8,
    arena_id: i32,
}

impl OnlineParams {
    /// Assemble list-only login options (infallible: every field has a
    /// CLI default or an empty fallback). Character info rides the
    /// channel login (the server rejects 2001 outside a channel), so
    /// the fetch arms exactly when a channel is selected. The room
    /// join stays out: startup only lists, and the session exchanges
    /// the pending join on Room Confirm (see
    /// [`OnlineParams::pending_room_join`]).
    fn into_connect_options(self) -> connect::ConnectOptions {
        let channel = self.online_gateway_id.zip(self.online_channel_id);
        connect::ConnectOptions {
            host: self.host.unwrap_or_default(),
            port: self.port,
            user_id: self.user_id.unwrap_or_default(),
            token: self.token.unwrap_or_default(),
            gender: o2jam_net::Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel,
            character: channel.is_some(),
            wallet: false,
            avatar_opa: None,
            join_room: None,
            room_password: String::new(),
            song: None,
        }
    }

    /// Room Confirm join intent for the session: the
    /// `--online-join-room` number plus password (`None` keeps Room
    /// Confirm a local enter, the offline walk).
    fn pending_room_join(&self) -> Option<(i32, String)> {
        self.join_room
            .map(|number| (number, self.room_password.clone()))
    }

    /// Host pick for the session: the `--online-music-id` song plus
    /// difficulty/speed/arena (`None` without a music id keeps Waiting
    /// Confirm on the START/READY path).
    fn pending_song_choice(&self) -> Option<session::SongChoice> {
        self.music_id.map(|music_id| session::SongChoice {
            music_id,
            difficulty: self.difficulty,
            speed: self.speed,
            arena_id: self.arena_id,
        })
    }
}

/// `connect` subcommand: assemble the song flow and login options,
/// run the pinned login flow, print the report. The clap fields stay
/// in [`run_command`]; this is the flow the arm dispatches to.
///
/// # Errors
///
/// Whatever the song-flow build, the login-options build, or the
/// login flow reports.
fn run_connect(song_flow: SongFlowParams, params: ConnectParams) -> Result<()> {
    let song = song_flow.into_song_flow()?;
    let hold_seconds = params.hold_after_join;
    let options = params.into_connect_options(song)?;
    if hold_seconds == 0 {
        let report = connect::connect_and_login(&options)?;
        o2jam_net::connect_report::print_connect_report(&report);
        return Ok(());
    }
    let (report, mut gateway) = connect::connect_and_login_with_transport(&options)?;
    o2jam_net::connect_report::print_connect_report(&report);
    for _ in 0..hold_seconds {
        gateway.send_frame(o2jam_net::Generic::Ping.id(), &[])?;
        // Best-effort echo drain; a missed broadcast is not a login fault.
        let _ = gateway.next_frame();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Ok(())
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
            launch_blob,
            hold_after_join,
            gender,
            relay_key1,
            relay_key2,
            gateway_id,
            channel_id,
            character,
            wallet,
            avatar_opa,
            join_room,
            room_password,
            music_id,
            difficulty,
            game_speed,
            arena_id,
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
        } => run_connect(
            SongFlowParams {
                music_id,
                difficulty,
                game_speed,
                arena_id,
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
            },
            ConnectParams {
                host,
                port,
                user_id,
                token,
                launch_blob,
                hold_after_join,
                gender,
                relay_key1,
                relay_key2,
                gateway_id,
                channel_id,
                character,
                wallet,
                avatar_opa,
                join_room,
                room_password,
            },
        ),
        Command::Shots { out_dir } => {
            for path in shots::write_shots(std::path::Path::new(&out_dir))? {
                println!("wrote {}", path.display());
            }
            Ok(())
        }
        Command::SessionWalk {
            host,
            port,
            user_id,
            token,
            out_dir,
        } => run_session_walk(&host, port, user_id, &token, &out_dir),
        Command::Online {
            host,
            port,
            user_id,
            token,
            gender,
            gateway_id,
            channel_id,
            create_room,
            join_room,
            room_password,
            music_id,
            difficulty,
            speed,
            arena,
            interface,
            no_sound,
            no_relay_gate,
            fixed_clock,
            frame_dump,
            input_log,
        } => run_online(OnlineCli {
            host,
            port,
            user_id,
            token,
            gender,
            gateway_id,
            channel_id,
            create_room,
            join_room,
            room_password,
            music_id,
            difficulty,
            speed,
            arena,
            interface,
            no_sound,
            no_relay_gate,
            fixed_clock,
            frame_dump,
            input_log,
        }),
        Command::ValidateTrace { trace } => {
            let loaded = trace::load_validated(std::path::Path::new(&trace))?;
            println!(
                "trace ok: {} ({} events, clock {} {})",
                loaded.scenario,
                loaded.events.len(),
                loaded.time_base.name,
                loaded.time_base.unit
            );
            Ok(())
        }
        Command::ReplayMenuTrace { trace, stage } => {
            // A bare file name resolves to the committed menu-trace
            // fixtures; anything else loads as a path.
            let loaded = if std::path::Path::new(&trace).exists() {
                trace::load_validated(std::path::Path::new(&trace))?
            } else {
                menu_trace_replay::load_menu_trace(&trace)?
            };
            let report = menu_trace_replay::replay_menu_file(&loaded, &stage)?;
            println!(
                "replay ok: {} ({} applied, {} accepted, final side {:?}, hash {})",
                report.scenario,
                report.applied(),
                report.events.len() - report.applied() - report.rejected(),
                report.final_side,
                report.final_hash
            );
            Ok(())
        }
        Command::E2e { out_dir } => run_e2e(&out_dir),
        Command::Play {
            chart,
            ojm,
            level,
            no_sound,
            window,
            frame_dump,
            input_log,
            avatar_opa,
            equip,
            avatar_male,
            interface,
            online,
            host,
            user_id,
            token,
            online_launch_blob,
            port,
            online_gateway_id,
            online_channel_id,
            online_join_room,
            online_room_password,
            online_create_room,
            online_music_id,
            online_difficulty,
            online_speed,
            online_arena,
        } => run_play(PlayOptions {
            chart,
            ojm,
            level,
            no_sound,
            window,
            frame_dump,
            input_log,
            avatar_opa,
            equip,
            avatar_male,
            interface,
            online,
            host,
            user_id,
            token,
            online_launch_blob,
            port,
            online_gateway_id,
            online_channel_id,
            online_join_room,
            online_room_password,
            online_create_room,
            online_music_id,
            online_difficulty,
            online_speed,
            online_arena,
        }),
    }
}

/// `play` subcommand fields (one named bundle, no flag-argument
/// soup): the chart/autoplay wiring plus the `--online-*` login
/// pass-through.
#[derive(Debug, Clone)]
struct PlayOptions {
    chart: Option<String>,
    ojm: Option<String>,
    level: usize,
    no_sound: bool,
    window: bool,
    frame_dump: Option<String>,
    input_log: Option<String>,
    avatar_opa: Option<String>,
    equip: String,
    avatar_male: bool,
    interface: Option<String>,
    online: bool,
    host: Option<String>,
    user_id: Option<i32>,
    token: Option<String>,
    online_launch_blob: Option<String>,
    port: u16,
    online_gateway_id: Option<i16>,
    online_channel_id: Option<i16>,
    online_join_room: Option<i32>,
    online_room_password: String,
    online_create_room: Option<String>,
    online_music_id: Option<u16>,
    online_difficulty: u8,
    online_speed: u8,
    online_arena: i32,
}

/// `play` subcommand: windowed game or headless autoplay.
///
/// # Errors
///
/// Whatever chart/bank loading, login, audio, or the window reports.
fn run_play(opts: PlayOptions) -> Result<()> {
    tracing::info!(chart = ?opts.chart, window = opts.window, "play stub");
    if opts.window {
        let mut game = o2jam_shell::ScreenGame::new();
        game.load_options(&install_root());
        setup_splash(&mut game, &opts.interface)?;
        live_art::stage_relay_down_from_install(&mut game);
        let gateway = setup_online(
            &mut game,
            opts.online,
            &opts.host,
            opts.user_id,
            &opts.token,
            opts.online_launch_blob.clone(),
            opts.port,
            opts.online_gateway_id,
            opts.online_channel_id,
            opts.online_join_room,
            &opts.online_room_password,
            opts.online_music_id,
            opts.online_difficulty,
            opts.online_speed,
            opts.online_arena,
        )?;
        setup_avatar(&mut game, &opts.avatar_opa, &opts.equip, opts.avatar_male)?;
        setup_chart_audio(
            &mut game,
            &opts.chart,
            &opts.ojm,
            opts.level,
            !opts.no_sound,
        )?;
        match gateway {
            Some((transport, pending_room, pending_song)) => {
                let mut session = assemble_online_session(
                    game,
                    transport,
                    opts.token.as_deref(),
                    pending_room,
                    pending_song,
                    opts.online_create_room.as_deref(),
                )?;
                // Audio containment: with --no-sound every bank attach
                // stays on the null sink even when hardware exists (the
                // session starts fenced, this is the live unfence).
                session.set_silent_audio(opts.no_sound);
                // Boot already logged into the requested channel
                // (setup_online with gateway+channel ids), so mark
                // residency now: a later face press replays locally
                // instead of re-sending a login the server ignores
                // on an occupied session (ticket #230).
                if let (Some(gw), Some(ch)) = (opts.online_gateway_id, opts.online_channel_id) {
                    session.set_resident_channel(o2jam_net::ChannelState {
                        gateway_id: gw as u16,
                        channel_id: ch as u16,
                        capacity: 0,
                        population: 0,
                        active: true,
                    });
                }
                run_windowed_session(
                    session,
                    capture_plan(opts.frame_dump.clone(), opts.input_log.clone()),
                )?;
            }
            None => {
                // Offline walk: the windowed arm stages genuine scene
                // art, the song catalog, and demo lobby lists through
                // one entry (ticket #192) so the boards never go empty
                // on the default boot without a separate staging call.
                // The shop catalog rides the installed avatar.opa table
                // (ticket #191 reopen): stage it too.
                stage_online_identity(&mut game, None);
                stage_offline_windowed(&mut game);
                let event_loop = winit::event_loop::EventLoop::new()
                    .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
                let shell = o2jam_shell::Shell::new(Box::new(game));
                let shell = match capture_plan(opts.frame_dump.clone(), opts.input_log.clone()) {
                    Some(plan) => shell.with_capture(plan),
                    None => shell,
                };
                shell.run(event_loop)?;
            }
        }
        return Ok(());
    }
    let chart_path = opts.chart.ok_or_else(|| {
        anyhow::anyhow!("--chart FILE.ojn is required (add --window for the empty shell)")
    })?;
    run_autoplay(&chart_path, opts.ojm.as_deref(), opts.level, !opts.no_sound)
}

/// `e2e` subcommand: headless boot-to-Result walk with PNGs.
///
/// # Errors
///
/// Whatever the walk wiring or the filesystem reports.
fn run_e2e(out_dir: &str) -> Result<()> {
    let report = e2e::write_e2e_evidence(std::path::Path::new(out_dir))?;
    for step in &report.steps {
        println!(
            "{:02} {:40} screen={:<8} hash={:016x}",
            step.sequence, step.action, step.screen, step.state_hash
        );
    }
    println!("result hash={:016x}", report.result_hash);
    println!(
        "final {} hash={:016x}",
        report.final_screen, report.final_hash
    );
    Ok(())
}

/// `session-walk` subcommand: live windowed-session walk with PNGs.
///
/// # Errors
///
/// Whatever the walk transport, loading gate, or filesystem reports.
fn run_session_walk(host: &str, port: u16, user_id: i32, token: &str, out_dir: &str) -> Result<()> {
    let mut config =
        session_walk::WalkConfig::local(std::path::PathBuf::from(out_dir), token.to_owned());
    config.host = host.to_owned();
    config.port = port;
    config.user_id = user_id;
    for stage in session_walk::drive_walk(&config)? {
        println!("staged {} ({:?})", stage.name, stage.opcodes);
    }
    Ok(())
}

/// Demo lobby lists for the offline windowed walk: four channels
/// (the dialog's visible slice) and three rooms, so the staged scene
/// art renders rows without a server. Synthetic data only — no
/// server semantics ride along.
fn seed_offline_lobby(game: &mut o2jam_shell::ScreenGame) {
    use o2jam_net::{ChannelState, RoomListEntry};
    game.set_channels(
        (0..4)
            .map(|i| ChannelState {
                gateway_id: 0,
                channel_id: i,
                capacity: 100,
                population: if i < 2 { i32::from(80 + i) } else { 0 },
                active: true,
            })
            .collect(),
    );
    game.set_rooms(
        (1..=3)
            .map(|n| RoomListEntry {
                number: n,
                state_code: 1,
                title: format!("Demo Room {n}"),
                has_password: false,
                music_id: 0,
                difficulty: 0,
                key_mode: 2,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 1,
                min_level: 0,
                max_level: 0,
                skills: vec![],
                premium: false,
                room_type: 0,
            })
            .collect(),
    );
}

/// Stage the splash backdrop into a windowed game through the staging
/// declaration: the bound `STATE_AVI` record pairs `IDC_BACK_LOGO` with
/// its `.bnd` rect like every other scene. A missing archive keeps the
/// flat card so CI stays green without game data.
fn setup_splash(game: &mut o2jam_shell::ScreenGame, interface: &Option<String>) -> Result<()> {
    let bytes = match interface {
        Some(opi_path) => std::fs::read(opi_path)
            .with_context(|| format!("reading interface archive {opi_path}"))?,
        None => {
            let Some(bytes) = installed_interface_archive() else {
                return Ok(());
            };
            bytes
        }
    };
    let report = o2jam_shell::staging::stage_state(
        &bytes,
        o2jam_shell::staging::State::Avi,
        o2jam_shell::staging::StageContext::default(),
    );
    if let Some(art) = report.art {
        tracing::info!(
            placements = art.placements.len(),
            figures = art.figures.len(),
            "splash art staged through the declaration"
        );
        game.load_scene(o2jam_shell::Screen::Avi, art.placements, art.figures);
    }
    Ok(())
}

/// Installed interface archive for the offline splash fallback.
const INSTALLED_INTERFACE_OPI: &str = "../../install-root/O2Jam/Image/Interface1.opi";

/// Repository-local install root (git-ignored game data): the edge
/// that supplies game paths to the shell. `option.ini` loads from
/// and saves beside this directory (a missing file keeps the exe
/// defaults, so CI stays green without game data).
fn install_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// High nibble the exe puts on every id in its `SendMusicList` report
/// (Memoryer `MainRoomController.cs:144-167`: the "New" label flag).
const NEW_LABEL_FLAG: u16 = 0xf000;

/// Ids the installed client owns and reports in `SendMusicList` 2024.
///
/// The local catalog (`OJNList.dat`) names the rows; the `Music`
/// directory scan says which rows carry a chart. This install ships
/// thirteen charts beside the catalog and the rest with the later
/// `Music_Full` pack, so the installer batch is the set of chart files
/// within one hour of the catalog timestamp — exactly the thirteen
/// ids in the exe's live report
/// (`~/scratch/matched-ab-339/exe/wire/room-stage.log`). Missing game
/// data answers an empty list; no id is invented.
fn owned_music_ids() -> Vec<u16> {
    let root = install_root();
    let catalog_path = root.join(SESSION_CATALOG);
    let Ok(catalog_bytes) = std::fs::read(&catalog_path) else {
        return Vec::new();
    };
    let Ok(snapshot) = catalog_path.metadata().and_then(|meta| meta.modified()) else {
        return Vec::new();
    };
    let known: std::collections::HashSet<u16> = o2jam_file::parse_ojn_list(&catalog_bytes)
        .map(|rows| {
            rows.into_iter()
                .filter(|row| row.is_song())
                .filter_map(|row| u16::try_from(row.song_id).ok())
                .collect()
        })
        .unwrap_or_default();
    let Ok(charts) = std::fs::read_dir(root.join("O2Jam/Music")) else {
        return Vec::new();
    };
    recent_catalog_ids(
        charts.filter_map(|entry| {
            let entry = entry.ok()?;
            let installed = entry.metadata().ok()?.modified().ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let digits = name.strip_prefix("o2ma")?.split('.').next()?;
            Some((digits.parse::<u16>().ok()?, installed))
        }),
        snapshot,
        &known,
    )
}

/// Pure half of the install scan: catalog rows whose chart belongs to
/// the installer batch around the catalog timestamp, flagged as the
/// exe flags them.
fn recent_catalog_ids<I>(
    charts: I,
    snapshot: std::time::SystemTime,
    known: &std::collections::HashSet<u16>,
) -> Vec<u16>
where
    I: IntoIterator<Item = (u16, std::time::SystemTime)>,
{
    let installer_batch = std::time::Duration::from_secs(60 * 60);
    let mut ids: Vec<u16> = charts
        .into_iter()
        .filter(|(id, installed)| {
            known.contains(id)
                && snapshot.duration_since(*installed).unwrap_or_default() <= installer_batch
        })
        .map(|(id, _)| id | NEW_LABEL_FLAG)
        .collect();
    ids.sort_unstable_by(|left, right| right.cmp(left));
    ids.dedup();
    ids
}

/// Genuine interface archive bytes, if present (read-only;
/// a missing archive keeps the flat card so CI stays green without
/// game data).
fn installed_interface_archive() -> Option<Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(INSTALLED_INTERFACE_OPI);
    std::fs::read(path).ok()
}

/// Live gateway plus the Room Confirm join intent (room number and
/// password) and the host song pick for the windowed session.
type OnlineTransport = (
    o2jam_net::GatewayConnection,
    Option<(i32, String)>,
    Option<session::SongChoice>,
);

/// One boundary for the relay-down channel-list gate (#244, #348).
///
/// Live callers: `run_online` and `run_play --online` call
/// [`relay_gate::relay_down_now`] on the boot path, connect and
/// fetch the rows, then hand them here. Down: boot holds the rows
/// without a dialog; the one-OK notice opens at channel entry and
/// holds them until its button (or Back) dismisses it. Up: the
/// existing `set_channels` path is untouched. Scenario tests in
/// `live_boot_gate` call this same function with both arms.
fn stage_channel_list_after_relay_gate(
    game: &mut o2jam_shell::ScreenGame,
    channels: Vec<o2jam_net::ChannelState>,
    relay_down: bool,
) {
    // #367: carry the same boot observation into Room state. This
    // enables our 07DA/07DB fuse; it never opens a dialog by itself.
    game.set_room_no_response_relay_down(relay_down);
    if relay_down {
        game.hold_channel_rows_for_relay_notice(channels);
    } else {
        game.set_channels(channels);
    }
}

/// Authenticate and list channels/rooms into a windowed game.
///
/// Returns the live gateway plus the Room Confirm join intent when
/// online: the caller wraps the transport with the game in an
/// [`session::OnlineSession`] so chat reaches the server, staging the
/// local username and the pending join on it. Startup stays
/// list-only — the session exchanges the join on Room Confirm.
#[allow(clippy::too_many_arguments, reason = "CLI fields pass through once")]
fn setup_online(
    game: &mut o2jam_shell::ScreenGame,
    online: bool,
    host: &Option<String>,
    user_id: Option<i32>,
    token: &Option<String>,
    launch_blob: Option<String>,
    port: u16,
    online_gateway_id: Option<i16>,
    online_channel_id: Option<i16>,
    online_join_room: Option<i32>,
    online_room_password: &str,
    online_music_id: Option<u16>,
    online_difficulty: u8,
    online_speed: u8,
    online_arena: i32,
) -> Result<Option<OnlineTransport>> {
    if !online {
        return Ok(None);
    }
    let params = OnlineParams {
        host: host.clone(),
        user_id,
        token: token.clone(),
        port,
        online_gateway_id,
        online_channel_id,
        join_room: online_join_room,
        room_password: online_room_password.to_owned(),
        music_id: online_music_id,
        difficulty: online_difficulty,
        speed: online_speed,
        arena_id: online_arena,
    };
    let pending_room = params.pending_room_join();
    let pending_song = params.pending_song_choice();
    let mut options = params.into_connect_options();
    apply_launch_blob(&mut options, launch_blob.as_deref())?;
    let relay_down = relay_gate::relay_down_now();
    let (report, gateway) = connect::connect_and_login_with_transport(&options)?;
    tracing::info!(channels = report.channels.len(), "channels listed");
    stage_online_identity(game, report.character.as_ref());
    stage_channel_list_after_relay_gate(game, report.channels, relay_down);
    stage_online_rooms(game, report.rooms);
    Ok(Some((gateway, pending_room, pending_song)))
}

/// Replace the Room-lobby rows with the 2003 reply on every online
/// entry, empty reply included. A channel re-entry that finds no rooms
/// must not keep the previous list: the exe's empty channel has no
/// clickable room row (#376). `set_rooms` drops free slots too.
fn stage_online_rooms(game: &mut o2jam_shell::ScreenGame, rooms: Vec<o2jam_net::RoomListEntry>) {
    if !rooms.is_empty() {
        tracing::info!(rooms = rooms.len(), "rooms listed");
    }
    game.set_rooms(rooms);
}

/// Stage the fetched character identity (`GetCharacterInfo` 2001) into
/// a windowed game (tickets #30 and #58): the room user panel readouts
/// and the MyRoom/Waiting seats ride the record itself, the worn figure
/// stacks from the installed `avatar.opa` with the hero's own equipment
/// and gender (the same stack the shots walk stages), and the archive
/// becomes the waiting-seat source so the self seat and every later
/// peer join restage through it. The decoded source is cached once per
/// install root, so the boot and channel-login landings share it (#755).
/// The item-shop catalog rides the same `Itemdata_Korea.dat` table, so
/// it stages on every boot that
/// lands the installed archive — including the list-only default
/// (no channel, no fetch; ticket #191 reopen). The MyRoom data path
/// rides the same two stagings (ticket #199 diagnose leg, named so a
/// later blank panel traces to its source): the character record
/// (`set_myroom_character` — channel-login boots only, `None` on the
/// list-only default), the bag/gift/info-plate name source (the item
/// table `set_avatar_source` stages beside the shop catalog — every
/// archive landing, even item-less), and the portrait figure (the worn
/// stack below, kept live by the equip restage hook). Infallible by
/// contract: a missing or
/// unstackable archive keeps the figure slots empty, never failing
/// the login walk.
fn stage_online_identity(
    game: &mut o2jam_shell::ScreenGame,
    character: Option<&o2jam_net::CharacterInfo>,
) {
    stage_identity_records(game, character);
    let source = o2jam_shell::read_installed_avatar(&install_root());
    stage_identity_avatar(game, character, source.as_ref());
}

/// Stage identity from a caller-held decode. The live path above
/// supplies the process cache; guards inject the same `Arc` on both
/// landings to pin reuse independently of shared-cache state (#862).
#[cfg(test)]
fn stage_online_identity_with_decoded_avatar(
    game: &mut o2jam_shell::ScreenGame,
    character: Option<&o2jam_net::CharacterInfo>,
    source: &std::sync::Arc<o2jam_shell::AvatarSource>,
) {
    stage_identity_records(game, character);
    stage_identity_avatar(game, character, Some(source));
}

/// Character record and song catalog ride every landing, even without
/// an avatar archive (tickets #191, #192).
fn stage_identity_records(
    game: &mut o2jam_shell::ScreenGame,
    character: Option<&o2jam_net::CharacterInfo>,
) {
    if let Some(hero) = character {
        game.set_myroom_character(hero.clone());
    }

    // The music-shop and lanking boards ride the installed song
    // catalog (`OJNList.dat`): stage it on every identity pass, even
    // when the avatar archive is missing (mirrors the item-shop hook
    // below and ticket #192 reopen). Missing/unparsable catalog leaves
    // the boards row-less; no fallback rows are invented.
    let catalog_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root")
        .join("O2Jam/Image/OJNList.dat");
    if let Ok(bytes) = std::fs::read(&catalog_path) {
        if let Ok(entries) = o2jam_file::parse_ojn_list(&bytes) {
            game.set_song_catalog(&entries);
            tracing::info!("music-shop board staged from identity");
        }
    }
}

/// The avatar half of identity: worn figure, then the shared decode
/// that feeds the waiting seats and the shop board.
fn stage_identity_avatar(
    game: &mut o2jam_shell::ScreenGame,
    character: Option<&o2jam_net::CharacterInfo>,
    source: Option<&std::sync::Arc<o2jam_shell::AvatarSource>>,
) {
    let Some(source) = source else {
        tracing::debug!(
            "avatar.opa missing or unparsable; identity readouts stage, figures stay empty"
        );
        return;
    };
    if let Some(hero) = character {
        let figure = connect::stack_from_decoded_source(
            source,
            &hero.equipment,
            hero.gender == o2jam_net::Gender::Male,
        );
        if !figure.is_empty() {
            // The Room lobby slot and the Result banner show the same
            // worn stack.
            game.load_room_avatar(figure.clone());
            game.load_avatar(figure);
        } else {
            tracing::debug!("character equipment stacks empty; figure slots stay empty");
        }
    }
    // The shop board rides the same `Itemdata_Korea.dat` table as the
    // avatar source (server-authoritative catalog join): stage it here
    // so the live board is never empty after the avatar archive lands,
    // even on the default boot that fetches no character (no channel).

    game.set_decoded_avatar_source(source);

    game.set_decoded_avatar_source(source);
    // #861: the Item Shop's state-3 catalog is the exe's new-arrivals
    // bucket walk (FUN_0049ead0 -> FUN_0052dc90 -> FUN_0049ee90),
    // not the raw Itemdata_Korea.dat order staged above. The Room
    // ITEMSHOP and MyRoom GOSHOP arms both mount state 3, so this one
    // landing feeds both entries. Seam-only setters keep raw order.
    game.stage_exe_item_shop_catalog(source.table());
}

/// Fold the channel-entry 5029 answer into the boot wallet. The exe
/// reads it before 2010/2011 (#850); the shop-BACK path applies the
/// same two words through `ScreenGame::apply_shop_cash_point`.
fn stage_online_wallet(
    game: &mut o2jam_shell::ScreenGame,
    cash_sync: Option<&o2jam_net::wallet::SyncCashPointResponse>,
) {
    if let Some(cash) = cash_sync {
        game.apply_shop_cash_point(cash);
    }
}

/// Song index under `install-root` (the 7.55 catalog the select-music
/// dialog lists; read-only like the archives).
const SESSION_CATALOG: &str = "O2Jam/Image/OJNList.dat";

/// Stage the installed song catalog into the windowed session's
/// select-music dialog. A missing or unparsable catalog keeps the
/// dialog row-less (no fallback rows: invented songs must never reach
/// the host pick); the chart fetch then warns instead of staging.
fn stage_session_catalog(session: &mut session::OnlineSession) {
    // Availability rides the catalog (§1): stage the install's `Music`
    // scan so a catalog song without a chart file shows the
    // not-downloaded notice and refuses the pick. Staged first so the
    // catalog inherits it; without game data the scan answers `None`.
    session.set_installed_music(o2jam_shell::music_install::scan_installed_music());
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root")
        .join(SESSION_CATALOG);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => {
            tracing::debug!("song catalog missing; dialog stages row-less");
            return;
        }
    };
    match o2jam_file::parse_ojn_list(&bytes) {
        Ok(entries) => {
            session.set_song_catalog_with_params(
                &entries,
                &catalog_labels(&bytes),
                &catalog_params(&bytes),
            );
            tracing::info!("song catalog staged");
        }
        Err(source) => {
            tracing::debug!(%source, "song catalog unparsable; dialog stages row-less");
        }
    }
}

/// The catalog's table-5 label buckets (the song dialog's label column
/// input, ticket #273). An unreadable middle region stages no bucket:
/// that column then keeps no row, exactly like a song the table omits.
fn catalog_labels(bytes: &[u8]) -> Vec<o2jam_file::OjnMidTagEntry> {
    match o2jam_file::parse_ojn_list_labels(bytes) {
        Ok(labels) => labels,
        Err(source) => {
            tracing::debug!(%source, "song labels unparsable; label filter keeps no row");
            Vec::new()
        }
    }
}

/// The catalog's table-3 price words (the music-shop GEM/WON cells).
/// An unreadable middle region stages no prices: the price cells
/// stay blank instead of showing an invented word.
fn catalog_params(bytes: &[u8]) -> Vec<o2jam_file::OjnMidParamEntry> {
    match o2jam_file::parse_ojn_list_params(bytes) {
        Ok(params) => params,
        Err(source) => {
            tracing::debug!(%source, "song prices unparsable; music shop keeps blank price cells");
            Vec::new()
        }
    }
}

/// Stage the default offline windowed boot (ticket #192): genuine
/// scene art from the installed archives when present (infallible;
/// without game data the flat fallbacks stay), the song catalog
/// (`OJNList.dat`, feeding the select-music dialog AND the
/// lanking/music-shop boards through `set_song_catalog`), then demo
/// lobby lists so the staged rows render for hand verification.
fn stage_offline_windowed(game: &mut o2jam_shell::ScreenGame) {
    live_art::stage_live_scenes(game);
    stage_installed_song_catalog(game);
    seed_offline_lobby(game);
}

/// Stage the installed song catalog (`OJNList.dat`) into a windowed
/// game: the select-music dialog, the lanking/music-shop boards, and
/// the loading gate's album-cover path ride the same parse (see
/// [`o2jam_shell::ScreenGame::set_song_catalog`]). A missing or
/// unparsable catalog keeps every board row-less and the loading
/// overlay on its planet fallback; no fallback rows are invented.
fn stage_installed_song_catalog(game: &mut o2jam_shell::ScreenGame) {
    // The offline walk stages the same local availability the online
    // boot stages (§1); `None` without game data.
    game.set_installed_music(o2jam_shell::music_install::scan_installed_music());
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root")
        .join(SESSION_CATALOG);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => {
            tracing::debug!("song catalog missing; boards stage row-less");
            return;
        }
    };
    match o2jam_file::parse_ojn_list(&bytes) {
        Ok(entries) => {
            game.set_song_catalog_with_labels(&entries, &catalog_labels(&bytes));
            game.stage_music_shop_prices(&catalog_params(&bytes));
            tracing::info!("offline song catalog staged");
        }
        Err(source) => {
            tracing::debug!(%source, "song catalog unparsable; boards stage row-less");
        }
    }
}

/// Music state to announce for one room song (`UpdateMusicState` 4025,
/// `docs/music-availability.md` §2).
///
/// Availability is the install's `Music` folder (§1): `Ready` only when
/// `o2ma<id>.ojn` is there, `NoMusic` otherwise — the exe's own two-way
/// answer (VA `0x4be808`). Without game data nothing is installed, so
/// the honest answer stays `NoMusic`; the `NoAccess` arm needs the
/// PT-gate globals (§5.5, OPEN) and is never claimed here.
fn installed_music_state(music_id: u16) -> o2jam_net::MusicState {
    let installed = o2jam_shell::music_install::scan_installed_music()
        .as_ref()
        .is_some_and(|seat| seat.is_installed(music_id));
    o2jam_net::MusicState::for_installed(installed)
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
        let (archive, table) = connect::read_avatar_source(opa_path)?;
        let figure = connect::stack_worn_figure(&archive, &ids, avatar_male)?;
        tracing::info!(layers = figure.len(), "avatar stacked");
        // The Result banner and the Room lobby both show the worn
        // figure (the room's `IDC_IMAGE_SROOM_AVATA` slot, ticket #30).
        game.load_room_avatar(figure.clone());
        game.load_avatar(figure);
        // Waiting seats restage through the same source on every join
        // (peers wear their wire equipment; ticket #40).
        game.set_avatar_source(archive, table);
    }
    Ok(())
}

/// Output backend honoring the `--no-sound` flag: the live sink by
/// default, the silent null sink on opt-out (automated runs must never
/// reach the speakers; CI has no audio device). Routes through the one
/// sink decision ([`o2jam_audio::SinkChoice`]) like the session
/// attaches. Mirrors `run_autoplay`.
fn audio_backend(sound: bool) -> Box<dyn o2jam_audio::AudioBackend> {
    o2jam_audio::SinkChoice::for_session(sound, false).open()
}

/// OJM archives the 7.55 sound-device init loads at boot, in exe bank
/// index order (loader `0x550db0`, loop `0x550ae0`, names at `0x67f730`
/// with a `0x48` stride). Name spelling follows the installed files:
/// Windows folds case, so `bgEffect.ojm`/`BGMwinter.ojm` are the exe's
/// `BGEffect.ojm`/`BGMWinter.ojm`. Observer-target pins: sha256
/// `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`,
/// REA `ev_b9fd1f27433ada2023ce7cb81fa25f6f828821a93b11eb7540478c5b6a562636`
/// (caller chain) and `ev_a9fb8aefefa70e2a45105090df82976fd44527a81a7029fd83d4f64e1a2bfd3e`
/// (bank names). Row `+0` (file `0x27f728`) is the mode-2 base; row `+4`
/// (file `0x27f72c`, `ev_f73477ba029f80c5d7bfcb0b5c9449dd2f5d9260ce880142338f1bfeda132518`)
/// is the mode-3 base used by every scene cue.
const BOOT_BGM_BANK_FILES: [&str; 7] = [
    "bgEffect.ojm",
    "O2PlanetNPC.ojm",
    "BGM.ojm",
    "Event.ojm",
    "Planet.ojm",
    "BGMwinter.ojm",
    "DialogBGM.ojm",
];

/// Bank index holding the scene-BGM archive in the exe's boot table.
const SCENE_BGM_BANK_INDEX: usize = 2;

/// Bank index of `Music/BGMWinter.ojm` (boot table row 5, name at file
/// `0x27f898`; evidence `ev_9bbdb57205ab0ed5a98a941fb5005c7fe526eddb7da8574b7e29a10ab21b1836`).
const RESULT_WORKER_BANK_INDEX: usize = 5;

/// Mode-3 cache-slot base per boot bank row (file `0x27f72c`, stride
/// `0x48`). The loader (`0x550db0`, `ev_75ba7fe104a34f019541c1ba1bb792b8b27aef67f0cc8048e7946b44e9986ba1`)
/// adds each fetched entry's ordinal to its row base and rejects slots
/// at or above `0x17`.
const BOOT_MODE3_BANK_BASES: [usize; 7] = [0, 1, 1, 0xb, 0xc, 0xd, 0x17];

/// Size of the exe's mode-3 cache: slots `0..0x16`, indexed by the
/// one-based cue id minus one (`SceneBgm::resolve`).
const SCENE_WORKER_SLOT_COUNT: usize = 0x17;

/// Installed path of one exe boot bank. The scene bank and the Result
/// worker bank are decoded into the shared cache; the UI effect bank also
/// feeds the separate one-shot emitter.
fn boot_bank_path(index: usize) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Music")
        .join(BOOT_BGM_BANK_FILES[index])
}

/// Bank index holding the UI one-shot archive in the exe's boot table
/// (`0x67f728`, stride `0x48`): row 0 is `bgEffect.ojm`, row 1 is
/// `O2PlanetNPC.ojm` (voice lines, out of this cluster).
const UI_EFFECT_BANK_INDEX: usize = 0;

/// Cache-slot base of the UI effect bank (row `+0` of the executable's
/// boot table). The fetcher (`0x54eca0`) writes each mode-2 entry's
/// fetch ordinal, and the bank loader adds this base to it; ids 1/2/5/9/
/// `0x23` therefore name `bgEffect` entries 2/3/6/10/36, not decimal
/// sample names.
const UI_EFFECT_BANK_BASE: usize = 0;

/// Emitter bound from `0x551870`: ids `>= 0x34` are rejected.
const UI_EFFECT_EMITTER_BOUND: usize = o2jam_shell::screens::game::UI_EFFECT_SLOT_COUNT;

/// Descriptor version for `bgEffect.ojm`: 2.8 selects the exe's 17-way
/// de-interleave plus zeroed-NOT pass (`docs/file-formats/ojm.md`).
const UI_EFFECT_BANK_VERSION: f32 = 2.8;

#[allow(dead_code)]
/// Named UI one-shot cue, beside the scene cue. Returns `None` for a
/// slot the first cluster does not pin (the silent list in the #718 map)
/// or one past the emitter bound. Shell handler branches call
/// `ScreenGame::cue_ui_effect` with the same `UiEffectId`; the client
/// hook keeps the id→cue table visible at the launch entry.
fn ui_effect_cue(effect_id: u8) -> Option<o2jam_shell::screens::game::UiEffectId> {
    o2jam_shell::screens::game::UiEffectId::from_slot(effect_id)
}

/// The cue one screen enters with: delegates to the shell's
/// single-source table ([`o2jam_shell::Screen::bgm_cue`], S0.2
/// pins re-read 2026-11-11: Avi 8 once, Planet/Channel 9 loop,
/// Room 3, Shop 4, Myroom 5, Waiting 6, MusicShop 10, Result
/// `0x13` once, Loading/Playing/Lanking silence `-1`). Kept as
/// the launch client's named hook so every scene entry cues
/// through one call: the game drives it per transition
/// (`ScreenGame::enter_screen`), the launch cue below is only
/// the first.
fn scene_bgm_cue(screen: o2jam_shell::Screen) -> (i32, bool) {
    screen.bgm_cue()
}

/// The cue one side screen enters with: delegates to the shell's
/// single-source table ([`o2jam_shell::screens::SideScreen::bgm_cue`],
/// S0.2 pins re-read 2026-11-11: Shop 4, MusicShop 10, MyRoom 5,
/// Planet 9, Payment/Payment2 3 loop, Lanking/Tutorial silence `-1`).
/// Kept as the launch client's named side hook so every side entry
/// cues through one call; `ScreenGame::visit_side` drives the same
/// table in-process.
#[allow(dead_code)]
fn side_scene_bgm_cue(side: o2jam_shell::screens::SideScreen) -> (i32, bool) {
    side.bgm_cue()
}

/// Decode one bank record (a Vorbis split body) into PCM; blanks
/// and non-Vorbis bodies stay silent. Mode-2 encrypted bodies do
/// not decode in this slice (the shipped `BGM.ojm` holds none).
fn decode_bgm_record(
    bytes: &[u8],
    entry: &o2jam_file::OjmEntry,
) -> Option<o2jam_audio::PcmSamples> {
    let body = entry.sample_bytes(bytes).ok()?;
    if !body.starts_with(b"OggS") {
        return None;
    }
    o2jam_audio::decode_vorbis(body).ok()
}

/// Decode the two banks that pinned scene cues address into the exe's
/// shared worker cache: `BGM.ojm` fills slots `1..=10` and the `bgResult`
/// entry of `BGMwinter.ojm` fills slot `0x13`. An entry's fetch ordinal is
/// added to its boot row base, then converted to the one-based cue id
/// index the mixer uses. Other boot rows stay installed but uncued; only
/// their fetched bytes are avoided, no fallback track is invented.
/// Missing or unparsable archives leave slots empty, so CI stays green.
/// Shared with the online session (`session.rs`).
pub(crate) fn load_scene_bgm_bank() -> Vec<Option<o2jam_audio::PcmSamples>> {
    let mut slots = vec![None; SCENE_WORKER_SLOT_COUNT];
    stage_mode3_records(&mut slots, SCENE_BGM_BANK_INDEX, None);
    stage_mode3_records(
        &mut slots,
        RESULT_WORKER_BANK_INDEX,
        Some(o2jam_audio::RESULT_WORKER_RECORD_ORDINAL),
    );
    slots
}

/// Expected PCM of the Result cue, decoded straight from the installed
/// `BGMwinter.ojm` mode-3 record 6. Test-only: a live leg compares the
/// backend capture against this independent source of truth. `None`
/// means the archive is absent, so the caller skips the leg.
#[cfg(test)]
pub(crate) fn expected_result_cue_pcm() -> Option<o2jam_audio::PcmSamples> {
    let path = boot_bank_path(RESULT_WORKER_BANK_INDEX);
    let bytes = std::fs::read(path).ok()?;
    let tables = o2jam_file::parse_new_variant_entries(&bytes).ok()?;
    let entry = tables
        .mode3
        .get(o2jam_audio::RESULT_WORKER_RECORD_ORDINAL)?;
    decode_bgm_record(&bytes, entry)
}

/// Stage one bank's mode-3 records at `base + ordinal`, or only `wanted`
/// when the caller needs one pinned record. Slot `N` lands at array index
/// `N - 1` because the cue id is one-based; slot 0 has no cue.
fn stage_mode3_records(
    slots: &mut [Option<o2jam_audio::PcmSamples>],
    bank_index: usize,
    only_ordinal: Option<usize>,
) {
    let base = BOOT_MODE3_BANK_BASES[bank_index];
    let Ok(bytes) = std::fs::read(boot_bank_path(bank_index)) else {
        tracing::debug!(
            bank = BOOT_BGM_BANK_FILES[bank_index],
            "boot bank missing; its slots stay silent"
        );
        return;
    };
    let Ok(tables) = o2jam_file::parse_new_variant_entries(&bytes) else {
        tracing::debug!(
            bank = BOOT_BGM_BANK_FILES[bank_index],
            "boot bank unparsable; its slots stay silent"
        );
        return;
    };
    for (ordinal, entry) in tables.mode3.iter().enumerate() {
        if only_ordinal.is_some_and(|record| record != ordinal) {
            continue;
        }
        let Some(slot) = base.checked_add(ordinal) else {
            continue;
        };
        // The loader rejects slots at its bound.
        if slot >= SCENE_WORKER_SLOT_COUNT {
            continue;
        }
        let Some(index) = slot.checked_sub(1) else {
            continue;
        };
        if let Some(target) = slots.get_mut(index) {
            *target = decode_bgm_record(&bytes, entry);
        }
    }
}

/// Decode the tutorial's own bank (`Music/Tutorial.ojm`) mode-3
/// records into the slot order the chart's 12 bit-26 marker ids name
/// (`docs/ojn-bit26.md`). The exe's tutorial state-enter helper loads
/// `Tutorial.ojm` at file `0xf0a50` and calls its playback helper at
/// `0xf0ab0` before chart time; this bank feeds that entry cue. Missing
/// or unparsable data yields no records, so CI stays green without an
/// install and no substitute sample ever sounds.
pub(crate) fn load_tutorial_ojm_bank() -> Vec<Option<o2jam_audio::PcmSamples>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Music/Tutorial.ojm");
    let Ok(bytes) = std::fs::read(&path) else {
        tracing::debug!("tutorial bank missing; its entry cue stays silent");
        return Vec::new();
    };
    let Ok(tables) = o2jam_file::parse_new_variant_entries(&bytes) else {
        tracing::debug!("tutorial bank unparsable; its entry cue stays silent");
        return Vec::new();
    };
    tables
        .mode3
        .iter()
        .map(|entry| decode_bgm_record(&bytes, entry))
        .collect()
}

/// Decode the first boot bank (`Music/bgEffect.ojm`) into the exe's UI
/// effect slots. The fetcher writes each mode-2 entry's ordinal into the
/// field the UI bank loader reads as an id (#718 correction to
/// `docs/file-formats/ojm.md`), so ordinal N lands in slot
/// `bank base + N`. Slots the install does not decode stay empty: no
/// synthetic sample is ever substituted for a missing bank entry.
/// Missing or unparsable data yields an all-empty bank so CI stays
/// green without game data.
pub(crate) fn load_ui_effect_bank() -> Vec<Option<o2jam_audio::PcmSamples>> {
    let mut slots = vec![None; UI_EFFECT_EMITTER_BOUND];
    let Ok(bytes) = std::fs::read(boot_bank_path(UI_EFFECT_BANK_INDEX)) else {
        tracing::debug!("UI effect bank missing; effect cues stay silent");
        return slots;
    };
    let decoded = loading::load_sound_bank(&bytes, UI_EFFECT_BANK_VERSION);
    for (ordinal, pcm) in decoded {
        let Some(slot) = usize::try_from(ordinal)
            .ok()
            .and_then(|ordinal| UI_EFFECT_BANK_BASE.checked_add(ordinal))
        else {
            continue;
        };
        if let Some(entry) = slots.get_mut(slot) {
            *entry = Some(pcm);
        }
    }
    slots
}

/// Load a chart plus its key sounds into a windowed game, if requested.
///
/// The audio rig is also attached without a chart so the launch
/// BGM cue sounds on the plain windowed start: the client boots
/// into the splash screen, whose entry cues the logo track once
/// ([`scene_bgm_cue`]). The game then re-cues every later screen
/// entry through the staged bank (`ScreenGame::enter_screen` →
/// `Screen::bgm_cue`); the online session swaps backends in
/// `session.rs` regardless.
///
/// # Errors
///
/// Whatever chart/bank loading or the BGM cue reports.
fn setup_chart_audio(
    game: &mut o2jam_shell::ScreenGame,
    chart: &Option<String>,
    ojm: &Option<String>,
    level: usize,
    sound: bool,
) -> Result<()> {
    let mut bank = std::collections::HashMap::new();
    if let Some(path) = chart {
        let (level_chart, format_version) = load_level_chart(game, path, level)?;
        game.load_chart(level_chart.hit_onsets_ms());
        tracing::info!(hits = level_chart.hit_count(), "chart loaded");
        if let Some(bank_path) = ojm {
            let bank_bytes = std::fs::read(bank_path)?;
            bank = loading::load_sound_bank(&bank_bytes, format_version);
            tracing::info!("key sounds attached");
        }
    }
    // Null/silent with `--no-sound` (audio containment): the cue
    // through the null sink only counts frames. Staging the bank
    // cues the current (Avi) screen immediately, and every later
    // entry re-cues through the same table.
    let backend = audio_backend(sound);
    let bgm_bank = load_scene_bgm_bank();
    game.with_audio(backend, bank);
    game.stage_scene_bgm(bgm_bank);
    game.set_tutorial_ojm_bank(load_tutorial_ojm_bank());
    game.stage_ui_effect_bank(load_ui_effect_bank());
    let (cue_id, cue_repeat) = scene_bgm_cue(o2jam_shell::Screen::Avi);
    tracing::info!(?cue_id, ?cue_repeat, "launch BGM cued");
    Ok(())
}

/// Load one chart level from disk into the game (chart + slice checks
/// plus the shared [`loading::load_sound_bank`] key-sound decode)
/// shared by autoplay and the windowed game).
///
/// Runs the lifecycle §9 loading gate: select validation is the
/// level-slot range check (`0x5718b0`), staging logs the
/// `MUSIC\Temp\%s%d_%d` extract name (`0x4f8b80`), the open goes
/// through `loading::load_gated_level` (`0x412180`), the walked cover
/// paints the Loading art (a miss keeps the planet fallback), and the
/// return arms the song clock downstream (`0x4e47a0`).
fn load_level_chart(
    game: &mut o2jam_shell::ScreenGame,
    chart_path: &str,
    level: usize,
) -> Result<(o2jam_game::LevelChart, f32)> {
    use anyhow::Context;
    let chart_bytes =
        std::fs::read(chart_path).with_context(|| format!("reading chart {chart_path}"))?;
    let prepared = loading::prepare_chart(&chart_bytes, level)
        .with_context(|| format!("loading gate rejected {chart_path} level {level}"))?;
    let events = o2jam_file::parse_ojn_level_events(&chart_bytes, level)?;
    let chart = o2jam_game::build_level_chart(prepared.bpm, &events);
    loading::stage_loading_cover(game, &chart_bytes);
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
/// wall clock unless `--no-sound`), trigger key sounds, judge perfect.
fn run_autoplay(chart_path: &str, ojm_path: Option<&str>, level: usize, sound: bool) -> Result<()> {
    use anyhow::Context;
    use o2jam_game::{Judgement, Score, Sequencer};
    // Headless keeps no Loading scene: the staged cover drops with
    // this temp game, while the highway chart below still autoplays.
    let mut loading_game = o2jam_shell::ScreenGame::new();
    let (chart, format_version) = load_level_chart(&mut loading_game, chart_path, level)?;
    tracing::info!(hits = chart.hit_count(), "chart loaded");
    let mut bank = std::collections::HashMap::new();
    if let Some(path) = ojm_path {
        let bank_bytes = std::fs::read(path).with_context(|| format!("reading bank {path}"))?;
        bank = loading::load_sound_bank(&bank_bytes, format_version);
        tracing::info!(samples = bank.len(), "bank decoded");
    }
    let mut backend: Box<dyn o2jam_audio::AudioBackend> = audio_backend(sound);
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

/// Builders and the report golden test (backlog item 5).
#[cfg(test)]
mod cli_build_tests {
    use o2jam_net::{Gender, StatKind};

    use super::*;

    fn song_params(music_id: Option<u16>) -> SongFlowParams {
        SongFlowParams {
            music_id,
            difficulty: 2,
            game_speed: 1,
            arena_id: 0,
            member_id: 0,
            power_skill: 0,
            stat_kind: "jam".to_owned(),
            stat_value: 3,
            stat_seq: 1,
            stat_score: 210,
            cool: 10,
            good: 2,
            bad: 0,
            miss: 0,
            max_combo: 12,
            jam: 1,
            max_jam: 1,
            final_score: 2210,
            life: 100,
            long_note: 0,
        }
    }

    fn connect_params() -> ConnectParams {
        ConnectParams {
            host: "127.0.0.1".to_owned(),
            port: 15010,
            user_id: 7,
            token: "tester".to_owned(),
            gender: "any".to_owned(),
            launch_blob: None,
            hold_after_join: 0,
            relay_key1: 0,
            relay_key2: 0,
            gateway_id: Some(1),
            channel_id: Some(2),
            character: false,
            wallet: false,
            avatar_opa: None,
            join_room: None,
            room_password: String::new(),
        }
    }

    #[test]
    fn song_flow_absent_without_music_id() {
        let song = song_params(None).into_song_flow().unwrap();
        assert!(song.is_none());
    }

    #[test]
    fn audio_backend_off_queues_silently_without_hardware() {
        // Windowed runs with `--no-sound` must never touch audio
        // hardware: the silent sink counts queued frames with no device
        // open, so this stays green on headless CI. A live backend
        // reports its own playhead instead of the queued count.
        use o2jam_audio::PcmSamples;
        let mut backend = audio_backend(false);
        let pcm = PcmSamples {
            samples: vec![1000i16; 100],
            sample_rate: 44100,
            channels: 2,
        };
        backend.play_pcm(&pcm).unwrap();
        assert_eq!(backend.samples_played(), 50);
    }

    #[test]
    fn play_defaults_to_sound_on() {
        // #120: audio is live unless `--no-sound` opts out — a bare
        // run reaches the speakers.
        let cli = Cli::try_parse_from(["recomp2jam", "play"]).unwrap();
        let Command::Play { no_sound, .. } = cli.command.unwrap() else {
            panic!("expected play subcommand");
        };
        assert!(!no_sound);
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn no_sound_flag_opts_out_of_audio() {
        // #120: `--no-sound` flips the rig silent on both windowed
        // subcommands; the session then stays on the null sink.
        let cli = Cli::try_parse_from(["recomp2jam", "play", "--no-sound"]).unwrap();
        let Command::Play { no_sound, .. } = cli.command.unwrap() else {
            panic!("expected play subcommand");
        };
        assert!(no_sound);
        let cli = Cli::try_parse_from(["recomp2jam", "online", "--token", "tester", "--no-sound"])
            .unwrap();
        let Command::Online { no_sound, .. } = cli.command.unwrap() else {
            panic!("expected online subcommand");
        };
        assert!(no_sound);
    }

    #[test]
    fn setup_splash_defaults_to_installed_logo() {
        // Offline splash without `--interface` stages the installed
        // logo; a missing archive keeps the flat card (CI skip rule).
        let archive = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Interface1.opi");
        if !archive.exists() {
            println!(
                "SKIP setup_splash_defaults_to_installed_logo: missing install-root Interface1.opi"
            );
            return;
        }
        let mut staged = o2jam_shell::ScreenGame::new();
        setup_splash(&mut staged, &None).unwrap();
        let mut bare = o2jam_shell::ScreenGame::new();
        let mut staged_buf = vec![0u32; 64 * 48];
        let mut bare_buf = vec![0u32; 64 * 48];
        staged.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut staged_buf,
                w: 64,
                h: 48,
            },
            0,
        );
        bare.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut bare_buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_ne!(staged_buf, bare_buf, "installed logo paints nothing");
    }

    #[test]
    fn song_flow_maps_every_cli_field() {
        let params = SongFlowParams {
            music_id: Some(757),
            difficulty: 3,
            game_speed: 2,
            arena_id: 7,
            member_id: 4,
            power_skill: 11,
            stat_kind: "life".to_owned(),
            stat_value: 1000,
            stat_seq: 9,
            stat_score: 4242,
            cool: 1,
            good: 2,
            bad: 3,
            miss: 4,
            max_combo: 5,
            jam: 6,
            max_jam: 7,
            final_score: 8000,
            life: 90,
            long_note: 12,
        };
        let flow = params.into_song_flow().unwrap().unwrap();
        assert_eq!(flow.music_id, 757);
        assert_eq!(flow.difficulty, 3);
        assert_eq!(flow.speed, 2);
        assert_eq!(flow.arena_id, 7);
        assert_eq!(flow.member_id, 4);
        assert_eq!(flow.power_skill_id, 11);
        assert_eq!(flow.stat_kind, StatKind::Life);
        assert_eq!(flow.stat_value, 1000);
        assert_eq!(flow.stat_sequence, 9);
        assert_eq!(flow.stat_score, 4242);
        assert_eq!(flow.cool, 1);
        assert_eq!(flow.good, 2);
        assert_eq!(flow.bad, 3);
        assert_eq!(flow.miss, 4);
        assert_eq!(flow.max_combo, 5);
        assert_eq!(flow.jam, 6);
        assert_eq!(flow.max_jam, 7);
        assert_eq!(flow.final_score, 8000);
        assert_eq!(flow.life, 90);
        assert_eq!(flow.long_note_score, 12);
    }

    #[test]
    fn song_flow_rejects_unknown_stat_kind() {
        let params = SongFlowParams {
            music_id: Some(757),
            stat_kind: "bogus".to_owned(),
            ..song_params(None)
        };
        assert!(params.into_song_flow().is_err());
    }

    #[test]
    fn connect_pairs_channel_ids() {
        let song = song_params(Some(757)).into_song_flow().unwrap();
        let options = connect_params().into_connect_options(song).unwrap();
        assert_eq!(options.host, "127.0.0.1");
        assert_eq!(options.port, 15010);
        assert_eq!(options.user_id, 7);
        assert_eq!(options.token, "tester");
        assert_eq!(options.gender, Gender::Any);
        assert_eq!(options.channel, Some((1, 2)));
        assert!(options.song.is_some());
    }

    #[test]
    fn connect_takes_1000_identity_from_launch_blob() {
        let blob = o2jam_net::AuthParameters {
            user_index_id: "2".to_owned(),
            username: "blob-session-token".to_owned(),
            gender: "1".to_owned(),
            ..o2jam_net::AuthParameters::default()
        }
        .encode();
        let params = ConnectParams {
            launch_blob: Some(blob),
            ..connect_params()
        };

        let options = params.into_connect_options(None).unwrap();

        assert_eq!(options.user_id, 2);
        assert_eq!(options.token, "blob-session-token");
        assert_eq!(options.gender, Gender::Male);
    }

    #[test]
    fn connect_lists_without_channel() {
        let params = ConnectParams {
            gateway_id: None,
            channel_id: Some(2),
            ..connect_params()
        };
        let options = params.into_connect_options(None).unwrap();
        assert_eq!(options.channel, None);
        assert!(options.song.is_none());
    }

    #[test]
    fn connect_rejects_unknown_gender() {
        let params = ConnectParams {
            gender: "bogus".to_owned(),
            ..connect_params()
        };
        assert!(params.into_connect_options(None).is_err());
    }

    #[test]
    fn online_defaults_without_ids() {
        let params = OnlineParams {
            port: 15010,
            ..OnlineParams::default()
        };
        assert_eq!(params.pending_room_join(), None);
        let options = params.into_connect_options();
        assert_eq!(options.host, "");
        assert_eq!(options.user_id, 0);
        assert_eq!(options.token, "");
        assert_eq!(options.gender, Gender::Any);
        assert_eq!(options.channel, None);
        assert!(!options.character);
        assert!(options.avatar_opa.is_none());
        assert!(options.join_room.is_none());
        assert!(options.song.is_none());
    }

    #[test]
    fn online_pending_join_carries_number_and_password() {
        let params = OnlineParams {
            port: 15010,
            join_room: Some(3),
            room_password: "secret".to_owned(),
            ..OnlineParams::default()
        };
        assert_eq!(params.pending_room_join(), Some((3, "secret".to_owned())));
        // Startup stays list-only: the session exchanges the pending
        // join on Room Confirm instead.
        assert!(params.into_connect_options().join_room.is_none());
    }

    #[test]
    fn online_pending_song_maps_cli_choice() {
        let params = OnlineParams {
            port: 15010,
            music_id: Some(757),
            difficulty: 2,
            speed: 1,
            arena_id: 0,
            ..OnlineParams::default()
        };
        assert_eq!(
            params.pending_song_choice(),
            Some(session::SongChoice {
                music_id: 757,
                difficulty: 2,
                speed: 1,
                arena_id: 0,
            })
        );
    }

    #[test]
    fn online_pending_song_absent_without_music_id() {
        let params = OnlineParams {
            port: 15010,
            ..OnlineParams::default()
        };
        assert_eq!(params.pending_song_choice(), None);
    }

    /// Synthetic chart blob (the 7.55 OJN header shape, mirroring the
    /// loading-gate tests): song 757 with an empty level slice and no
    /// cover section, so the gate passes while the planet fallback stays.
    fn coverless_chart_blob() -> Vec<u8> {
        let mut blob = vec![0u8; 300];
        blob[0..4].copy_from_slice(&757u32.to_le_bytes());
        blob[4..8].copy_from_slice(b"ojn\0");
        blob[8..12].copy_from_slice(&2.9f32.to_le_bytes());
        blob[16..20].copy_from_slice(&124f32.to_le_bytes());
        blob[64..68].copy_from_slice(&0u32.to_le_bytes());
        blob[68..72].copy_from_slice(&0u32.to_le_bytes());
        blob[72..76].copy_from_slice(&0u32.to_le_bytes());
        blob[284..288].copy_from_slice(&300u32.to_le_bytes());
        blob[288..292].copy_from_slice(&300u32.to_le_bytes());
        blob[292..296].copy_from_slice(&300u32.to_le_bytes());
        blob[296..300].copy_from_slice(&300u32.to_le_bytes());
        blob
    }

    /// Spill `bytes` into a scratch `.ojn` file for [`load_level_chart`].
    fn scratch_chart_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "recomp2jam-cover-{name}-{}.ojn",
            std::process::id()
        ));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn level_chart_without_cover_keeps_the_planet_fallback() {
        let path = scratch_chart_file("coverless", &coverless_chart_blob());
        let mut game = o2jam_shell::ScreenGame::new();
        let outcome = load_level_chart(&mut game, path.to_str().unwrap(), 0);
        std::fs::remove_file(&path).ok();
        let (chart, _) = outcome.unwrap();
        assert_eq!(chart.hit_count(), 0);
        assert!(!game.has_loading_cover());
    }

    #[test]
    fn level_chart_rejects_garbage_without_staging() {
        let path = scratch_chart_file("garbage", b"garbage");
        let mut game = o2jam_shell::ScreenGame::new();
        let outcome = load_level_chart(&mut game, path.to_str().unwrap(), 0);
        std::fs::remove_file(&path).ok();
        assert!(outcome.is_err());
        assert!(!game.has_loading_cover());
    }

    #[test]
    fn walked_cover_paints_the_windowed_loading_scene() {
        // Install-guarded: song 757's OJN cover section stages into
        // the Loading art. Skips when `install-root` is absent.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        if !path.is_file() {
            println!("SKIP walked_cover_paints_the_windowed_loading_scene: install-root absent");
            return;
        }
        let mut game = o2jam_shell::ScreenGame::new();
        let (chart, _) = load_level_chart(&mut game, path.to_str().unwrap(), 0).unwrap();
        assert!(chart.hit_count() > 0);
        assert!(game.has_loading_cover());
    }

    #[test]
    fn online_pairs_online_ids() {
        let options = OnlineParams {
            host: Some("127.0.0.1".to_owned()),
            user_id: Some(7),
            token: Some("tester".to_owned()),
            port: 15010,
            online_gateway_id: Some(0),
            online_channel_id: Some(0),
            ..OnlineParams::default()
        }
        .into_connect_options();
        assert_eq!(options.host, "127.0.0.1");
        assert_eq!(options.channel, Some((0, 0)));
    }

    #[test]
    fn installer_charts_report_flagged_catalog_ids_in_descending_order() {
        use std::time::{Duration, SystemTime};

        let catalog_snapshot = SystemTime::now();
        let installed = catalog_snapshot - Duration::from_secs(21);
        let earlier_pack = catalog_snapshot - Duration::from_secs(86_400);
        let known = std::collections::HashSet::from([757, 865, 100]);
        let ids = recent_catalog_ids(
            [
                (757, installed),
                (865, installed),
                (100, earlier_pack),
                (999, installed),
            ],
            catalog_snapshot,
            &known,
        );
        assert_eq!(ids, [0xf361, 0xf2f5]);
    }

    #[test]
    fn online_params_fetch_character_info_only_with_a_channel() {
        // Ticket #30: the GetCharacterInfo fetch rides the channel
        // login (the server rejects 2001 outside a channel), so the
        // option arms exactly when a channel is selected and the
        // list-only walk stays fetch-free.
        let with_channel = OnlineParams {
            online_gateway_id: Some(1),
            online_channel_id: Some(2),
            ..OnlineParams::default()
        }
        .into_connect_options();
        assert!(with_channel.character);
        assert!(!OnlineParams::default().into_connect_options().character);
    }

    /// Room lobby walked to its screen with only the avatar slot
    /// staged: the paint join asserts on the slot pixels alone.
    fn room_with_avatar_slot() -> o2jam_shell::ScreenGame {
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::GameLoop;
        let mut game = o2jam_shell::ScreenGame::new();
        game.set_channels(vec![o2jam_net::ChannelState {
            gateway_id: 0,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        }]);
        game.menu_key(o2jam_shell::MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(o2jam_shell::MenuKey::Confirm, true);
        assert_eq!(game.screen(), o2jam_shell::Screen::Room);
        game.load_scene(
            o2jam_shell::Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_IMAGE_SROOM_AVATA".to_owned(),
                code: 0x1370_0013,
                sprite: None,
                rect: o2jam_render::Rect {
                    x: 0,
                    y: 0,
                    w: 123,
                    h: 150,
                },
            }],
            std::collections::HashMap::new(),
        );
        game
    }

    /// Painted Room-scene pixels: 800x600, the native canvas.
    fn paint_room(game: &mut o2jam_shell::ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        buf
    }

    /// Count the painted pixels inside the avatar slot rect
    /// (0,0,123,150): a 2D region, not a flat buffer prefix.
    fn slot_ink(buf: &[u32]) -> usize {
        (0..150)
            .flat_map(|y| (0..123).map(move |x| buf[y * 800 + x]))
            .filter(|pixel| *pixel != 0)
            .count()
    }

    #[test]
    fn staged_online_identity_paints_the_room_avatar_slot() {
        // Install-guarded (ticket #30): staging the fetched identity
        // (the 2001 record plus the figure stacked from the installed
        // avatar.opa with the hero's equipment) makes the Room scene
        // paint the avatar slot; without the staging the same slot
        // stays empty. Skips when install-root is absent.
        let avatar_relative = o2jam_shell::avatar_source::AVATAR_ARCHIVE_RELATIVE;
        if !install_root().join(avatar_relative).exists() {
            println!("SKIP staged_online_identity_paints_the_room_avatar_slot: missing install-root {avatar_relative}");
            return;
        }
        // The live tester2 row (`t_o2jam_item` USER_INDEX_ID=2,
        // ticket #171 round 2): kit ids at Equip1..5, the default
        // male face 35 at Equip12 (pinned type-5 → slot 0xb rule),
        // zeros elsewhere — `o2jam-shell`'s `LIVE_TESTER2_EQUIPMENT_ROW`
        // documents the row and its live-wire probe numbers.
        let mut hero = o2jam_net::CharacterInfo {
            nickname: "tester".to_owned(),
            level: 2,
            experience: 2133,
            ..o2jam_net::CharacterInfo::default()
        };
        hero.equipment = [16, 30, 376, 383, 389, 0, 0, 0, 0, 0, 0, 35, 0, 0, 0, 0, 0];
        let unstaged = paint_room(&mut room_with_avatar_slot());
        assert_eq!(
            slot_ink(&unstaged),
            0,
            "without the identity the avatar slot stays empty"
        );
        let mut staged = room_with_avatar_slot();
        stage_online_identity(&mut staged, Some(&hero));
        let staged = paint_room(&mut staged);
        assert!(
            slot_ink(&staged) > 0,
            "the staged identity paints the avatar slot"
        );
    }
}

/// Scene-BGM hook tests: the cue table against itself and the
/// shipped `BGM.ojm`. Containment: the null audio path only.
#[cfg(test)]
mod scene_bgm_tests {
    use o2jam_audio::{AudioBackend, NullBackend, PcmSamples, SceneBgm};
    use o2jam_shell::screens::SideScreen;
    use o2jam_shell::Screen;

    use super::{
        boot_bank_path, decode_bgm_record, load_scene_bgm_bank, load_tutorial_ojm_bank,
        scene_bgm_cue, side_scene_bgm_cue, BOOT_BGM_BANK_FILES, BOOT_MODE3_BANK_BASES,
        RESULT_WORKER_BANK_INDEX, SCENE_BGM_BANK_INDEX, SCENE_WORKER_SLOT_COUNT,
    };
    use o2jam_audio::{
        RESULT_WORKER_BANK_BASE_SLOT, RESULT_WORKER_RECORD_ORDINAL, RESULT_WORKER_SLOT,
    };

    /// Ten-slot bank (records 1..=10) holding one synthetic track in
    /// `record` and blanks elsewhere — the shipped bank's shape
    /// (10 fetch-order records, blanks at 1, 2, and 7).
    fn bank_with_track(record: usize) -> Vec<Option<PcmSamples>> {
        let mut bank = vec![None; 10];
        bank[record - 1] = Some(PcmSamples {
            samples: vec![1000i16; 441],
            sample_rate: 44100,
            channels: 1,
        });
        bank
    }

    /// Cue a screen through a silent (null) backend: the selection
    /// latches and the cue counts its frames.
    fn cue_through_null_backend(screen: Screen, bank: &[Option<PcmSamples>]) -> SceneBgm {
        let mut bgm = SceneBgm::new();
        let mut backend = NullBackend::new();
        let (id, repeat) = scene_bgm_cue(screen);
        let pcm = bgm.play_scene_music(id, repeat, bank);
        backend.set_scene_bgm(pcm, repeat).unwrap();
        if pcm.is_some() {
            assert_eq!(backend.samples_played(), 441);
        }
        bgm
    }

    #[test]
    fn avi_launch_cue_selects_the_logo_record_once() {
        let bank = bank_with_track(8);
        assert_eq!(scene_bgm_cue(Screen::Avi), (8, false));
        let bgm = cue_through_null_backend(Screen::Avi, &bank);
        assert_eq!(bgm.current().map(|sel| sel.record), Some(8));
    }

    #[test]
    fn planet_cue_loops_the_main_room_record() {
        let bank = bank_with_track(9);
        assert_eq!(scene_bgm_cue(Screen::Planet), (9, true));
        let bgm = cue_through_null_backend(Screen::Planet, &bank);
        assert_eq!(bgm.current().map(|sel| sel.record), Some(9));
    }

    #[test]
    fn channel_cue_reuses_the_planet_dialog_track() {
        let bank = bank_with_track(9);
        assert_eq!(scene_bgm_cue(Screen::Channel), (9, true));
        let bgm = cue_through_null_backend(Screen::Channel, &bank);
        assert_eq!(bgm.current().map(|sel| sel.record), Some(9));
    }

    #[test]
    fn room_shop_myroom_waiting_cues_loop_their_records() {
        let cases = [
            (Screen::Room, 3),
            (Screen::Shop, 4),
            (Screen::Myroom, 5),
            (Screen::Waiting, 6),
        ];
        for (screen, record) in cases {
            let bank = bank_with_track(record);
            let id = i32::try_from(record).unwrap();
            assert_eq!(scene_bgm_cue(screen), (id, true));
            let bgm = cue_through_null_backend(screen, &bank);
            assert_eq!(bgm.current().map(|sel| sel.record), Some(record));
        }
    }

    #[test]
    fn music_shop_cue_loops_record_ten() {
        let bank = bank_with_track(10);
        assert_eq!(scene_bgm_cue(Screen::MusicShop), (10, true));
        let bgm = cue_through_null_backend(Screen::MusicShop, &bank);
        assert_eq!(bgm.current().map(|sel| sel.record), Some(10));
    }

    #[test]
    fn side_screens_cue_their_pinned_records() {
        let cases = [
            (SideScreen::Shop, 4),
            (SideScreen::MusicShop, 10),
            (SideScreen::MyRoom, 5),
            (SideScreen::Planet, 9),
            (SideScreen::Payment, 3),
        ];
        for (side, record) in cases {
            assert_eq!(side_scene_bgm_cue(side), (record, true));
        }
    }

    #[test]
    fn unpinned_screens_stay_silent() {
        for screen in [Screen::Loading, Screen::Playing, Screen::Lanking] {
            assert_eq!(scene_bgm_cue(screen).0, o2jam_audio::BGM_SILENCE_ID);
        }
        assert_eq!(
            side_scene_bgm_cue(SideScreen::Lanking).0,
            o2jam_audio::BGM_SILENCE_ID
        );
        assert_eq!(
            side_scene_bgm_cue(SideScreen::Tutorial).0,
            o2jam_audio::BGM_SILENCE_ID
        );
    }

    #[test]
    fn result_cue_plays_bg_result_from_the_winter_worker_bank() {
        // Live chain: main.rs::setup_chart_audio -> load_scene_bgm_bank
        // -> ScreenGame::stage_scene_bgm -> ScreenGame::enter_screen(Result)
        // -> Screen::bgm_cue -> SceneBgm::play_scene_music(0x13, once)
        // -> worker slot 0x13. Static exe chain: 0x4C1B90 -> 0x551820
        // -> 0x54BC50 -> worker 0x54BD60; boot loader 0x550DB0 maps
        // BGMwinter ordinal 6 onto slot 0xD + 6 (docs/coverage-audit.md).
        // This guard decodes the installed bank and skips without data.
        let result_id = i32::try_from(RESULT_WORKER_SLOT).expect("slot fits i32");
        assert_eq!(scene_bgm_cue(Screen::Result), (result_id, false));
        assert_eq!(
            BOOT_MODE3_BANK_BASES[RESULT_WORKER_BANK_INDEX],
            RESULT_WORKER_BANK_BASE_SLOT
        );
        assert_eq!(RESULT_WORKER_RECORD_ORDINAL, 6);
        let path = boot_bank_path(RESULT_WORKER_BANK_INDEX);
        if !path.exists() {
            println!(
                "SKIP result_cue_plays_bg_result_from_the_winter_worker_bank: missing {}",
                path.display()
            );
            return;
        }
        let bytes = std::fs::read(&path).expect("winter bank readable");
        let tables = o2jam_file::parse_new_variant_entries(&bytes).expect("winter tables");
        assert_eq!(tables.mode3.len(), 10);
        let bg_result = decode_bgm_record(&bytes, &tables.mode3[RESULT_WORKER_RECORD_ORDINAL])
            .expect("bgResult decodes");
        let bank = load_scene_bgm_bank();
        assert_eq!(
            bank.get(RESULT_WORKER_SLOT - 1),
            Some(&Some(bg_result.clone()))
        );

        let mut bgm = SceneBgm::new();
        let mut backend = NullBackend::new();
        let pcm = bgm.play_scene_music(result_id, false, &bank);
        backend
            .set_scene_bgm(pcm, false)
            .expect("null backend accepts the cue");
        assert_eq!(
            bgm.current()
                .map(|selection| (selection.record, selection.repeat)),
            Some((RESULT_WORKER_SLOT, false))
        );
        assert!(backend.samples_played() > 0, "the result track has PCM");
    }

    #[test]
    fn boot_bank_table_names_all_seven_exe_archives() {
        assert_eq!(
            BOOT_BGM_BANK_FILES,
            [
                "bgEffect.ojm",
                "O2PlanetNPC.ojm",
                "BGM.ojm",
                "Event.ojm",
                "Planet.ojm",
                "BGMwinter.ojm",
                "DialogBGM.ojm",
            ]
        );
        assert_eq!(BOOT_MODE3_BANK_BASES.len(), BOOT_BGM_BANK_FILES.len());
        assert_eq!(SCENE_WORKER_SLOT_COUNT, 0x17);
        assert_eq!(
            boot_bank_path(RESULT_WORKER_BANK_INDEX).file_name(),
            Some(std::ffi::OsStr::new("BGMwinter.ojm"))
        );
    }

    #[test]
    fn scene_bank_path_comes_from_the_third_boot_bank() {
        assert_eq!(SCENE_BGM_BANK_INDEX, 2);
        assert_eq!(
            boot_bank_path(SCENE_BGM_BANK_INDEX).file_name(),
            Some(std::ffi::OsStr::new("BGM.ojm"))
        );
    }

    #[test]
    fn shipped_bank_records_select_the_shipped_tracks() {
        // Record 3 names the room track on the installed bank;
        // skipping cleanly without game data (CI rule).
        let path = boot_bank_path(SCENE_BGM_BANK_INDEX);
        if !path.exists() {
            println!(
                "SKIP shipped_bank_records_select_the_shipped_tracks: missing {}",
                path.display()
            );
            return;
        }
        let bank = load_scene_bgm_bank();
        assert_eq!(bank.len(), SCENE_WORKER_SLOT_COUNT);
        assert!(bank[1].is_none(), "BGM.ojm blank record 2");
        assert!(bank[6].is_none(), "BGM.ojm blank record 7");
        let mut bgm = SceneBgm::new();
        for record in [3, 4, 5, 6, 8, 9, 10] {
            let pcm = bgm.play_scene_music(record, true, &bank);
            assert!(pcm.is_some(), "record {record} decodes");
            assert!(!pcm.expect("record PCM").samples.is_empty());
        }
        assert_eq!(bgm.play_scene_music(0x18, false, &bank), None);
    }

    #[test]
    fn tutorial_bank_decodes_the_twelve_shipped_mode3_records() {
        // Live chain: main.rs::run_play -> setup_chart_audio ->
        // load_tutorial_ojm_bank -> ScreenGame::set_tutorial_ojm_bank
        // -> FIRSTSTEP entry cue. Archival guard: skips in a bare CI
        // worktree; no fallback sample is invented for a missing bank.
        let bank = load_tutorial_ojm_bank();
        if bank.is_empty() {
            println!("SKIP tutorial_bank_decodes_the_twelve_shipped_mode3_records: no install-root Tutorial.ojm");
            return;
        }
        assert_eq!(bank.len(), 12, "bgsplit-01..12 in fetch order");
        assert!(
            bank.iter().all(Option::is_some),
            "every shipped BGM split decodes"
        );
    }
}

/// UI-effect hook tests: the pinned id set and the installed `bgEffect`
/// bank. Containment: no audio device is opened here.
#[cfg(test)]
mod ui_effect_tests {
    use super::{
        boot_bank_path, load_ui_effect_bank, ui_effect_cue, UI_EFFECT_BANK_BASE,
        UI_EFFECT_BANK_INDEX, UI_EFFECT_EMITTER_BOUND,
    };
    use o2jam_shell::screens::game::UiEffectId;

    #[test]
    fn pinned_ui_effect_ids_name_the_exe_handler_cues() {
        // seam: the id→cue table is the whole behavior of this launch
        // hook; the handler branches that call it are guarded in the
        // shell crate (see `ui_effect_cue_walk`).
        let cases = [
            (0x01, UiEffectId::DialogAccept),
            (0x02, UiEffectId::DialogDecline),
            (0x05, UiEffectId::PlanetExit),
            (0x09, UiEffectId::ChannelRowJoin),
            (0x23, UiEffectId::ChannelExit),
        ];
        for (slot, expected) in cases {
            assert_eq!(ui_effect_cue(slot), Some(expected), "slot {slot:#04x}");
        }
        assert_eq!(ui_effect_cue(0x00), None, "slot 0 is a blank bank entry");
        assert_eq!(UiEffectId::SliderApply.slot(), 2, "apply shares slot 2");
        assert_eq!(
            ui_effect_cue(u8::try_from(UI_EFFECT_EMITTER_BOUND).unwrap_or(u8::MAX)),
            None,
            "emitter rejects ids at its bound"
        );
        assert_eq!(UI_EFFECT_BANK_INDEX, 0);
        assert_eq!(UI_EFFECT_BANK_BASE, 0);
    }

    #[test]
    fn installed_effect_bank_decodes_the_first_cluster_slots() {
        // Install-guarded: no game data leaves this unclaimed (CI has
        // no install-root). The ids are cache slots, so these are
        // `bgEffect` entries 2/3/6/10/36 by fetch ordinal.
        let path = boot_bank_path(UI_EFFECT_BANK_INDEX);
        if !path.exists() {
            println!(
                "SKIP installed_effect_bank_decodes_the_first_cluster_slots: missing {}",
                path.display()
            );
            return;
        }
        let bank = load_ui_effect_bank();
        assert_eq!(bank.len(), UI_EFFECT_EMITTER_BOUND);
        for slot in [0x01_u8, 0x02, 0x05, 0x09, 0x23] {
            assert!(ui_effect_cue(slot).is_some(), "slot {slot:#04x} is pinned");
            assert!(
                bank[usize::from(slot)].is_some(),
                "slot {slot:#04x} has no decoded PCM"
            );
        }
    }
}

/// Ticket #192 (reopened): the music-shop and lanking boards were
/// empty on the default offline boot — the windowed arm staged scene
/// art and demo lobby lists but never the song catalog, and only the
/// `shots` walk staged the board catalogs. Guards the offline staging
/// entry so the boards ride the installed `OJNList.dat` live.
#[cfg(test)]
mod offline_music_board_tests {
    use super::*;

    #[test]
    fn offline_boot_stages_music_shop_and_lanking_boards() {
        // Live caller chain:
        //   main.rs::run_play (offline windowed arm) ->
        //   stage_offline_windowed -> stage_installed_song_catalog ->
        //   ScreenGame::set_song_catalog ->
        //   MusicShopState::set_catalog + LankingState::set_catalog
        // Archives absent -> skip cleanly (CI has no game data).
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root")
            .join(SESSION_CATALOG);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!(
                    "SKIP offline_boot_stages_music_shop_and_lanking_boards: missing {}",
                    path.display()
                );
                return;
            }
        };
        if o2jam_file::parse_ojn_list(&bytes).is_err() {
            println!(
                "SKIP offline_boot_stages_music_shop_and_lanking_boards: unparsable {}",
                path.display()
            );
            return;
        }
        let mut game = o2jam_shell::ScreenGame::new();
        stage_offline_windowed(&mut game);
        assert!(
            !game.music_shop_visible_rows().is_empty(),
            "offline boot stages the music-shop board from {SESSION_CATALOG}"
        );
        assert!(
            !game.lanking_visible_songs().is_empty(),
            "offline boot stages the lanking board from {SESSION_CATALOG}"
        );
    }

    #[test]
    fn online_identity_without_character_stages_music_shop_and_lanking_boards() {
        // Live caller chain (`recomp2jam online`, no channel login):
        //   main.rs::run_online -> stage_online_identity(None)
        //   -> ScreenGame::set_song_catalog ->
        //   MusicShopState::set_catalog + LankingState::set_catalog.
        // run_online never calls stage_session_catalog, so without the
        // ticket-#192 hook the boards stay row-less live.
        // Archives absent -> skip cleanly (CI has no game data).
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root")
            .join(SESSION_CATALOG);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!("SKIP online_identity_without_character_stages_music_shop_and_lanking_boards: missing {}", path.display());
                return;
            }
        };
        if o2jam_file::parse_ojn_list(&bytes).is_err() {
            println!("SKIP online_identity_without_character_stages_music_shop_and_lanking_boards: unparsable {}", path.display());
            return;
        }
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, None);
        assert!(
            !game.music_shop_visible_rows().is_empty(),
            "online identity pass stages the music-shop board from {SESSION_CATALOG}"
        );
        assert!(
            !game.lanking_visible_songs().is_empty(),
            "online identity pass stages the lanking board from {SESSION_CATALOG}"
        );
    }
}
