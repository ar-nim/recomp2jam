//! `recomp2jam` — the game client binary.
//!
//! Day-0: CLI only (`--help`, `info`, `play --autoplay` stub). The windowed
//! client (winit + wgpu + cpal) attaches here once file parsing is verified.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod connect;
mod live_art;
#[cfg(test)]
mod live_guard;
mod live_stats;
mod loading;
mod session;
mod session_walk;
mod shots;

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
    online_music_id: Option<u16>,
    online_difficulty: u8,
    online_speed: u8,
    online_arena: i32,
) -> Result<()> {
    tracing::info!(?chart, ?window, "play stub");
    if window {
        let mut game = o2jam_shell::ScreenGame::new();
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
        setup_chart_audio(&mut game, &chart, &ojm, level)?;
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
        match gateway {
            Some((transport, pending_room, pending_song)) => {
                let mut session = session::OnlineSession::new(game, transport);
                if let Some(name) = token.as_deref().filter(|name| !name.is_empty()) {
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
                o2jam_shell::Shell::new(Box::new(session)).run(event_loop)?;
            }
            None => {
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

/// Load the splash backdrop into a windowed game, if requested.
fn setup_splash(game: &mut o2jam_shell::ScreenGame, interface: &Option<String>) -> Result<()> {
    if let Some(opi_path) = interface {
        let logo = connect::load_interface_sprite(opi_path, "O2mediaLogo.ojs")?;
        tracing::info!(layers = logo.len(), "splash art loaded");
        game.load_splash(logo);
    }
    Ok(())
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
        let (level_chart, format_version) = load_level_chart(game, path, level)?;
        game.load_chart(level_chart.hit_onsets_ms());
        tracing::info!(hits = level_chart.hit_count(), "chart loaded");
        if let Some(bank_path) = ojm {
            let bank_bytes = std::fs::read(bank_path)?;
            let bank = loading::load_sound_bank(&bank_bytes, format_version);
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
