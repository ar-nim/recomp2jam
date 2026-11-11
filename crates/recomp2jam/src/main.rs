//! `recomp2jam` — the game client binary.
//!
//! Day-0: CLI only (`--help`, `info`, `play --autoplay` stub). The windowed
//! client (winit + wgpu + cpal) attaches here once file parsing is verified.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod connect;
mod e2e;
mod ledger_check;
mod live_art;
#[cfg(test)]
mod live_guard;
mod live_stats;
mod loading;
mod session;
mod session_walk;
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
        /// Play sound through the default output (default: null
        /// sink, so rigs never reach the speakers).
        #[arg(long)]
        sound: bool,
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
        let mut flow = connect::SongFlowOptions::new(music_id);
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
        Ok(connect::ConnectOptions {
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
        })
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
    sound: bool,
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
    let connect_options = connect::ConnectOptions {
        host: opts.host.clone(),
        port: opts.port,
        user_id: opts.user_id,
        token: opts.token.clone(),
        gender: connect::parse_gender(&opts.gender)?,
        relay_key1: 0,
        relay_key2: 0,
        channel: opts.gateway_id.zip(opts.channel_id),
        character: false,
        wallet: false,
        avatar_opa: None,
        join_room: None,
        room_password: String::new(),
        song: None,
    };
    let (report, gateway) = connect::connect_and_login_with_transport(&connect_options)?;
    tracing::info!(channels = report.channels.len(), "channels listed");
    game.set_channels(report.channels);
    if !report.rooms.is_empty() {
        tracing::info!(rooms = report.rooms.len(), "rooms listed");
        game.set_rooms(report.rooms);
    }
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
    // Audio containment (memory 979fb): rigs and CI never reach the
    // speakers unless a human passes --sound.
    session.set_silent_audio(!opts.sound);
    if let Some(step_ms) = opts.fixed_clock {
        session.game_mut().enable_fixed_clock(step_ms);
    }
    session.attach_initial_audio(opts.sound);
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
    frame_dump.map(|dir| o2jam_shell::CapturePlan {
        dir: std::path::PathBuf::from(dir),
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
    /// CLI default or an empty fallback). The room join stays out:
    /// startup only lists, and the session exchanges the pending
    /// join on Room Confirm (see [`OnlineParams::pending_room_join`]).
    fn into_connect_options(self) -> connect::ConnectOptions {
        connect::ConnectOptions {
            host: self.host.unwrap_or_default(),
            port: self.port,
            user_id: self.user_id.unwrap_or_default(),
            token: self.token.unwrap_or_default(),
            gender: o2jam_net::Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel: self.online_gateway_id.zip(self.online_channel_id),
            character: false,
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
        } => {
            let song = SongFlowParams {
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
            }
            .into_song_flow()?;
            let options = ConnectParams {
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
                wallet,
                avatar_opa,
                join_room,
                room_password,
            }
            .into_connect_options(song)?;
            let report = connect::connect_and_login(&options)?;
            print_connect_report(&report);
            Ok(())
        }
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
            sound,
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
            sound,
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
        Command::E2e { out_dir } => run_e2e(&out_dir),
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
            online_join_room,
            online_room_password,
            online_create_room,
            online_music_id,
            online_difficulty,
            online_speed,
            online_arena,
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
            online_join_room,
            online_room_password,
            online_create_room,
            online_music_id,
            online_difficulty,
            online_speed,
            online_arena,
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
    online_join_room: Option<i32>,
    online_room_password: String,
    online_create_room: Option<String>,
    online_music_id: Option<u16>,
    online_difficulty: u8,
    online_speed: u8,
    online_arena: i32,
) -> Result<()> {
    tracing::info!(?chart, ?window, "play stub");
    if window {
        let mut game = o2jam_shell::ScreenGame::new();
        game.load_options(&install_root());
        setup_splash(&mut game, &interface)?;
        let gateway = setup_online(
            &mut game,
            online,
            &host,
            user_id,
            &token,
            port,
            online_gateway_id,
            online_channel_id,
            online_join_room,
            &online_room_password,
            online_music_id,
            online_difficulty,
            online_speed,
            online_arena,
        )?;
        setup_avatar(&mut game, &avatar_opa, &equip, avatar_male)?;
        setup_chart_audio(&mut game, &chart, &ojm, level, sound)?;
        match gateway {
            Some((transport, pending_room, pending_song)) => {
                let mut session = assemble_online_session(
                    game,
                    transport,
                    token.as_deref(),
                    pending_room,
                    pending_song,
                    online_create_room.as_deref(),
                )?;
                // Audio containment: without --sound every bank attach
                // stays on the null sink even when hardware exists.
                session.set_silent_audio(!sound);
                run_windowed_session(session, None)?;
            }
            None => {
                // Offline walk: stage the genuine scene art from the
                // installed archives when present (infallible; without
                // game data the flat fallbacks stay), then seed demo
                // lobby lists so the staged rows render for hand
                // verification.
                live_art::stage_live_scenes(&mut game);
                seed_offline_lobby(&mut game);
                let event_loop = winit::event_loop::EventLoop::new()
                    .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
                o2jam_shell::Shell::new(Box::new(game)).run(event_loop)?;
            }
        }
        return Ok(());
    }
    let chart_path = chart.ok_or_else(|| {
        anyhow::anyhow!("--chart FILE.ojn is required (add --window for the empty shell)")
    })?;
    run_autoplay(&chart_path, ojm.as_deref(), level, sound)
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
    let options = params.into_connect_options();
    let (report, gateway) = connect::connect_and_login_with_transport(&options)?;
    tracing::info!(channels = report.channels.len(), "channels listed");
    game.set_channels(report.channels);
    if !report.rooms.is_empty() {
        tracing::info!(rooms = report.rooms.len(), "rooms listed");
        game.set_rooms(report.rooms);
    }
    Ok(Some((gateway, pending_room, pending_song)))
}

/// Song index under `install-root` (the 7.55 catalog the select-music
/// dialog lists; read-only like the archives).
const SESSION_CATALOG: &str = "O2Jam/Image/OJNList.dat";

/// Stage the installed song catalog into the windowed session's
/// select-music dialog. A missing or unparsable catalog keeps the
/// dialog row-less (no fallback rows: invented songs must never reach
/// the host pick); the chart fetch then warns instead of staging.
fn stage_session_catalog(session: &mut session::OnlineSession) {
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
            session.set_song_catalog(&entries);
            tracing::info!("song catalog staged");
        }
        Err(source) => {
            tracing::debug!(%source, "song catalog unparsable; dialog stages row-less");
        }
    }
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

/// Output backend honoring the `--sound` flag: live hardware only on
/// request, the silent null sink otherwise (automated runs must never
/// reach the speakers; CI has no audio device). Mirrors `run_autoplay`.
fn audio_backend(sound: bool) -> Box<dyn o2jam_audio::AudioBackend> {
    if sound {
        match o2jam_audio::CpalBackend::try_new() {
            Ok(backend) => Box::new(backend),
            Err(e) => {
                eprintln!("audio unavailable ({e}); continuing silent");
                Box::new(o2jam_audio::NullBackend::new())
            }
        }
    } else {
        Box::new(o2jam_audio::NullBackend::new())
    }
}

/// Installed scene-BGM bank the launch cue plays (`Music/BGM.ojm`,
/// read-only like the other archives below).
const SCENE_BGM_BANK: &str = "../../install-root/O2Jam/Music/BGM.ojm";

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

/// Decode `Music/BGM.ojm` into one [`None`]-kept slot per record
/// (fetch order = one-based record order) for
/// [`o2jam_audio::SceneBgm`]. A missing or unparsable bank yields
/// an empty one (no fallback tracks: only the shipped records
/// ever sound), so CI stays green without game data. Shared with
/// the online session's audio attach (`session.rs`).
pub(crate) fn load_scene_bgm_bank() -> Vec<Option<o2jam_audio::PcmSamples>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SCENE_BGM_BANK);
    let Ok(bytes) = std::fs::read(&path) else {
        tracing::debug!("scene BGM bank missing; BGM stays silent");
        return Vec::new();
    };
    let Ok(tables) = o2jam_file::parse_new_variant_entries(&bytes) else {
        tracing::debug!("scene BGM bank unparsable; BGM stays silent");
        return Vec::new();
    };
    tables
        .mode2
        .iter()
        .chain(tables.mode3.iter())
        .map(|entry| decode_bgm_record(&bytes, entry))
        .collect()
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
    // Null/silent unless `--sound` (audio containment): the cue
    // through the null sink only counts frames. Staging the bank
    // cues the current (Avi) screen immediately, and every later
    // entry re-cues through the same table.
    let backend = audio_backend(sound);
    let bgm_bank = load_scene_bgm_bank();
    game.with_audio(backend, bank);
    game.stage_scene_bgm(bgm_bank);
    let (cue_id, cue_repeat) = scene_bgm_cue(o2jam_shell::Screen::Avi);
    tracing::info!(?cue_id, ?cue_repeat, "launch BGM cued");
    Ok(())
}

/// Render one login-flow report as text (authorize, character, avatar,
/// channels, rooms, login, join, song), one line per section.
///
/// Pure: [`print_connect_report`] writes exactly this string, so the
/// golden test below pins the CLI output.
fn format_connect_report(report: &connect::ConnectReport) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "authorize: {}\n",
        connect::name_result(report.auth_code)
    ));
    if let Some(hero) = &report.character {
        text.push_str(&format!(
            "character: {} (Lv {}, {:?}, {} XP)\n",
            hero.nickname, hero.level, hero.gender, hero.experience
        ));
    }
    if report.avatar_layers > 0 {
        text.push_str(&format!("avatar: {} layers worn\n", report.avatar_layers));
    }
    if let Some(wallet) = &report.wallet {
        text.push_str(&format!(
            "wallet: gem {} cash_point {} point {} o2_cash {}\n",
            wallet.gem, wallet.cash_point, wallet.point, wallet.o2_cash
        ));
        if !wallet.skipped.is_empty() {
            text.push_str(&format!("skipped broadcasts: {:?}\n", wallet.skipped));
        }
    }
    text.push_str(&format!("channels: {}\n", report.channels.len()));
    for channel in &report.channels {
        text.push_str(&format!(
            "  gateway {} channel {}: {}/{} {}\n",
            channel.gateway_id,
            channel.channel_id,
            channel.population,
            channel.capacity,
            if channel.active { "open" } else { "closed" },
        ));
    }
    let open_rooms: Vec<_> = report
        .rooms
        .iter()
        .filter(|room| room.number != 0)
        .collect();
    text.push_str(&format!("rooms: {} open\n", open_rooms.len()));
    for room in open_rooms.iter().take(10) {
        text.push_str(&format!(
            "  #{} {:?} {}/{}{}\n",
            room.number,
            room.title,
            room.user_count,
            room.capacity,
            if room.has_password { " locked" } else { "" },
        ));
    }
    if let Some(login) = &report.channel_login {
        let restricted = match login.restricted {
            Some(true) => "restricted",
            Some(false) => "unrestricted",
            None => "restriction unknown",
        };
        text.push_str(&format!(
            "channel: {} ({restricted})\n",
            connect::name_result(login.result_code)
        ));
    }
    if let Some(join) = &report.join {
        match &join.room {
            Some(room) => text.push_str(&format!(
                "room: {} seats, {} in ({})\n",
                room.slots.len(),
                room.user_count,
                room.title
            )),
            None => text.push_str("room: joined without snapshot\n"),
        }
    }
    if let Some(song) = &report.song {
        text.push_str(&format!(
            "music: {} entries (free music: {})\n",
            song.music_entries,
            if song.free_music { "yes" } else { "no" },
        ));
        text.push_str(&format!("arena: {}\n", song.arena_echo.arena));
        match &song.peer_bar {
            Some(peer) => text.push_str(&format!(
                "tick: member {} {:?} = {}\n",
                peer.member_id,
                peer.kind(),
                peer.value,
            )),
            None => text.push_str("tick: no peer echo\n"),
        }
        match &song.peer_queue {
            Some(queue) => text.push_str(&format!(
                "mirror: slot {} channel {} queued, {} ranks, life[0]={}\n",
                queue.slot,
                queue.stat_kind,
                song.peer_ranks.len(),
                song.peers.life_levels()[0],
            )),
            None => text.push_str(&format!(
                "mirror: no queue forward ({} ranks)\n",
                song.peer_ranks.len()
            )),
        }
        if !song.skipped.is_empty() {
            text.push_str(&format!("skipped broadcasts: {:?}\n", song.skipped));
        }
    }
    text
}

/// Print one login-flow report (authorize, character, avatar, channels,
/// rooms, login, join). Pure output, infallible.
fn print_connect_report(report: &connect::ConnectReport) {
    print!("{}", format_connect_report(report));
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
/// wall clock with `--sound`), trigger key sounds, judge perfect.
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
    use o2jam_game::{PeerBars, RankDelivery};
    use o2jam_net::{
        ChannelLoginResponse, ChannelState, CharacterInfo, GameStatsUpdate, Gender,
        JoinRoomResponse, JoinedRoom, RoomListEntry, RoomSlot, StatKind,
    };

    use o2jam_net::play::RoomArenaEcho;

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
        // Windowed runs without `--sound` must never touch audio
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
    fn setup_splash_defaults_to_installed_logo() {
        // Offline splash without `--interface` stages the installed
        // logo; a missing archive keeps the flat card (CI skip rule).
        let archive = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Interface1.opi");
        if !archive.exists() {
            println!("skip: missing install-root Interface1.opi");
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

    fn room_row(number: i32, title: &str, locked: bool) -> RoomListEntry {
        RoomListEntry {
            number,
            state_code: 1,
            title: title.to_owned(),
            has_password: locked,
            music_id: 0,
            difficulty: 0,
            key_mode: 5,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 3,
            min_level: 1,
            max_level: 99,
            skills: Vec::new(),
            premium: false,
            room_type: 0,
        }
    }

    fn full_report() -> connect::ConnectReport {
        let hero = CharacterInfo {
            nickname: "tester".to_owned(),
            gender: Gender::Male,
            level: 3,
            experience: 250,
            ..Default::default()
        };
        let mut peers = PeerBars::new();
        let queue = peers.apply_stats_update(0, 0, 1000).unwrap();
        let slots: Vec<RoomSlot> = (0..7u8)
            .map(|index| RoomSlot {
                index,
                state_code: 0,
                member: None,
            })
            .collect();
        connect::ConnectReport {
            auth_code: 0,
            character: Some(hero),
            wallet: Some(connect::WalletReport {
                gem: 5,
                cash_point: 6,
                point: 7,
                o2_cash: 10,
                skipped: Vec::new(),
            }),
            avatar_layers: 2,
            channels: vec![
                ChannelState {
                    gateway_id: 0,
                    channel_id: 1,
                    capacity: 100,
                    population: 12,
                    active: true,
                },
                ChannelState {
                    gateway_id: 0,
                    channel_id: 2,
                    capacity: 100,
                    population: 100,
                    active: false,
                },
            ],
            channel_login: Some(ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            }),
            join: Some(JoinRoomResponse {
                result_code: 0,
                room: Some(JoinedRoom {
                    member_id: 0,
                    team: 0,
                    title: "Lobby".to_owned(),
                    music_id: 0,
                    arena: 0,
                    key_mode: 5,
                    game_mode: 0,
                    difficulty: 0,
                    speed: 1,
                    has_super_manager: false,
                    user_count: 3,
                    slots,
                    skills: Vec::new(),
                    premium: false,
                    champion: None,
                    champion_streak: None,
                }),
            }),
            rooms: vec![room_row(0, "Empty", false), room_row(7, "Lobby", true)],
            song: Some(connect::SongFlowReport {
                music_entries: 4,
                free_music: true,
                arena_echo: RoomArenaEcho::new(3),
                peer_bar: Some(GameStatsUpdate {
                    member_id: 0,
                    kind_code: 0,
                    value: 1000,
                    ranks: [0; 8],
                }),
                peers,
                peer_queue: Some(queue),
                peer_ranks: vec![
                    RankDelivery {
                        position: 0,
                        rank: 5,
                        slot: Some(0),
                    },
                    RankDelivery {
                        position: 1,
                        rank: 6,
                        slot: None,
                    },
                ],
                skipped: vec![4001],
            }),
        }
    }

    #[test]
    fn full_report_renders_the_golden_text() {
        let expected = concat!(
            "authorize: Success (0)\n",
            "character: tester (Lv 3, Male, 250 XP)\n",
            "avatar: 2 layers worn\n",
            "wallet: gem 5 cash_point 6 point 7 o2_cash 10\n",
            "channels: 2\n",
            "  gateway 0 channel 1: 12/100 open\n",
            "  gateway 0 channel 2: 100/100 closed\n",
            "rooms: 1 open\n",
            "  #7 \"Lobby\" 3/8 locked\n",
            "channel: Success (0) (unrestricted)\n",
            "room: 7 seats, 3 in (Lobby)\n",
            "music: 4 entries (free music: yes)\n",
            "arena: 3\n",
            "tick: member 0 Some(Life) = 1000\n",
            "mirror: slot 0 channel 0 queued, 2 ranks, life[0]=1000\n",
            "skipped broadcasts: [4001]\n",
        );
        assert_eq!(format_connect_report(&full_report()), expected);
    }

    #[test]
    fn minimal_report_renders_the_golden_text() {
        let report = connect::ConnectReport {
            auth_code: 99,
            character: None,
            wallet: None,
            avatar_layers: 0,
            channels: Vec::new(),
            channel_login: None,
            join: None,
            rooms: Vec::new(),
            song: None,
        };
        let expected = "authorize: unknown code 99\nchannels: 0\nrooms: 0 open\n";
        assert_eq!(format_connect_report(&report), expected);
    }

    #[test]
    fn quiet_room_covers_the_fallback_lines() {
        let report = connect::ConnectReport {
            auth_code: 0,
            character: None,
            wallet: None,
            avatar_layers: 0,
            channels: Vec::new(),
            channel_login: Some(ChannelLoginResponse {
                result_code: 0,
                restricted: None,
            }),
            join: Some(JoinRoomResponse {
                result_code: 0,
                room: None,
            }),
            rooms: Vec::new(),
            song: Some(connect::SongFlowReport {
                music_entries: 0,
                free_music: false,
                arena_echo: RoomArenaEcho::new(0),
                peer_bar: None,
                peers: PeerBars::new(),
                peer_queue: None,
                peer_ranks: Vec::new(),
                skipped: Vec::new(),
            }),
        };
        let expected = concat!(
            "authorize: Success (0)\n",
            "channels: 0\n",
            "rooms: 0 open\n",
            "channel: Success (0) (restriction unknown)\n",
            "room: joined without snapshot\n",
            "music: 0 entries (free music: no)\n",
            "arena: 0\n",
            "tick: no peer echo\n",
            "mirror: no queue forward (0 ranks)\n",
        );
        assert_eq!(format_connect_report(&report), expected);
    }
}

/// Scene-BGM hook tests: the cue table against itself and the
/// shipped `BGM.ojm`. Containment: the null audio path only.
#[cfg(test)]
mod scene_bgm_tests {
    use o2jam_audio::{AudioBackend, NullBackend, PcmSamples, SceneBgm};
    use o2jam_shell::screens::SideScreen;
    use o2jam_shell::Screen;

    use super::{load_scene_bgm_bank, scene_bgm_cue, side_scene_bgm_cue, SCENE_BGM_BANK};

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
    fn result_cue_leaves_scene_bgm_silent_until_its_bank_is_found() {
        // OPEN: result cues no `BGM.ojm` record (id `0x13` past its
        // 10 records), so the channel silences. Next step: identify
        // which bank the worker holds when `0x4C1B90` cues it.
        let bank = bank_with_track(3);
        assert_eq!(scene_bgm_cue(Screen::Result), (0x13, false));
        let bgm = cue_through_null_backend(Screen::Result, &bank);
        assert!(!bgm.is_playing());
    }

    #[test]
    fn shipped_bank_records_select_the_shipped_tracks() {
        // Record 3 names the room track on the installed bank;
        // skipping cleanly without game data (CI rule).
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SCENE_BGM_BANK);
        if !path.exists() {
            println!("skip: missing {}", path.display());
            return;
        }
        let bank = load_scene_bgm_bank();
        assert_eq!(bank.len(), 10, "BGM.ojm: 0 mode-2 + 10 mode-3 records");
        assert!(bank[0].is_none() && bank[1].is_none() && bank[6].is_none());
        let mut bgm = SceneBgm::new();
        for record in [3, 4, 5, 6, 8, 9, 10] {
            let pcm = bgm.play_scene_music(record, true, &bank);
            assert!(pcm.is_some(), "record {record} decodes");
            assert!(!pcm.unwrap().samples.is_empty());
        }
        assert_eq!(bgm.play_scene_music(19, false, &bank), None);
    }
}
