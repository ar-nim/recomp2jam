//! Online login flow against a Memoryer gateway.
//!
//! Phase 1 of `docs/client-server-plan.md`: connect, authorize,
//! list channels, optionally log into one — all through
//! [`GatewayConnection::exchange`]. Relay keys normally arrive via
//! the launcher (`OTwo.exe … |test|??|addr|port` argv); without one,
//! they default to zero and the server decides.

use std::net::SocketAddr;

use crate::live_stats::{apply_peer_update, jam_tick_for_score, life_tick_for_score};
use anyhow::{bail, Context, Result};
use o2jam_game::{PeerBars, QueueEvent, RankDelivery};
use o2jam_net::{
    play::{MusicListAnswer, RoomArenaEcho},
    Frame,
};
use o2jam_net::{
    AuthResponse, AuthResult, AuthorizeRequest, ChannelList, ChannelLogin, ChannelLoginResponse,
    ChannelState, CharacterInfo, ConfirmMusicLoaded, ExitPlaying, FinalizeRank, GameStatsUpdate,
    GatewayConnection, Gender, Generic, GetMusicList, JoinRoom, JoinRoomResponse, Request,
    RoomList, RoomListEntry, SetDownloadProgress, SetRoomArena, SetRoomMusic, StartGame, StatKind,
    SubmitScore, SyncMemberMusicState, UpdateGameStats, WireMessage,
};

/// Login options (see `recomp2jam connect --help`).
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Gateway host.
    pub host: String,
    /// Gateway TCP port.
    pub port: u16,
    /// Account id.
    pub user_id: i32,
    /// Username.
    pub token: String,
    /// Account gender.
    pub gender: Gender,
    /// Relay key from launcher auth (0 without one).
    pub relay_key1: i32,
    /// Relay key from launcher auth (0 without one).
    pub relay_key2: i32,
    /// `(gateway, channel)` to enter after listing; `None` lists only.
    pub channel: Option<(i16, i16)>,
    /// Fetch character info after authorizing.
    pub character: bool,
    /// `avatar.opa` path: stack the worn figure from the fetched
    /// equipment (`None` skips avatar work; needs `character`).
    pub avatar_opa: Option<String>,
    /// Room number to join after the channel login (`None` skips;
    /// needs a channel).
    pub join_room: Option<i32>,
    /// Password for `--join-room` (empty when unlocked).
    pub room_password: String,
    /// Host song flow after the room join (`None` skips; needs a
    /// room the gateway lets us host).
    pub song: Option<SongFlowOptions>,
}

/// What the login flow learned.
#[derive(Debug)]
pub struct ConnectReport {
    /// Authorize result code (always success — else this errors).
    pub auth_code: u32,
    /// Character info, when requested.
    pub character: Option<CharacterInfo>,
    /// Worn figure layers stacked from `avatar.opa`, when requested.
    pub avatar_layers: usize,
    /// Channels the gateway listed.
    pub channels: Vec<ChannelState>,
    /// Channel login answer, when a channel was requested.
    pub channel_login: Option<ChannelLoginResponse>,
    /// Join answer, when a room was requested.
    pub join: Option<JoinRoomResponse>,
    /// Room rows, fetched only after a channel login (the room
    /// controller rejects channel outsiders).
    pub rooms: Vec<RoomListEntry>,
    /// Song-flow answer, when requested with a room join.
    pub song: Option<SongFlowReport>,
}

/// Parse a `--gender` word.
pub fn parse_gender(word: &str) -> Result<Gender> {
    match word.to_ascii_lowercase().as_str() {
        "male" | "m" | "1" => Ok(Gender::Male),
        "female" | "f" | "0" => Ok(Gender::Female),
        "any" | "unknown" | "" => Ok(Gender::Any),
        _ => bail!("unknown gender '{word}' (want male, female, or any)"),
    }
}

/// Parse a `--stat-kind` word (`UpdateGameStats` channel).
pub fn parse_stat_kind(word: &str) -> Result<StatKind> {
    match word.to_ascii_lowercase().as_str() {
        "life" | "0" => Ok(StatKind::Life),
        "jam" | "1" => Ok(StatKind::Jam),
        _ => bail!("unknown stat kind '{word}' (want life or jam)"),
    }
}

/// Host song-flow tunables (plan Phases 3–4).
///
/// Every field maps to one documented `play.rs` wire field; the flow
/// runs select → arena → list → sync → start → loaded → tick →
/// submit → rank → exit. Needs channel membership plus a room the
/// gateway lets us host (host-only steps reject outsiders).
#[derive(Debug, Clone)]
pub struct SongFlowOptions {
    /// Room song id (`SetRoomMusic` 4000).
    pub music_id: u16,
    /// Chart difficulty code (0 EX, 1 NX, 2 HX, 3 RX).
    pub difficulty: u8,
    /// Scroll-speed code (`SetRoomMusic` + `ConfirmMusicLoaded`).
    pub speed: u8,
    /// Room stage word (`SetRoomArena` 4002, echoed as 4003).
    pub arena_id: i32,
    /// Room slot the sync steps belong to.
    pub member_id: u8,
    /// Power skill-ring id (`ConfirmMusicLoaded` 4012).
    pub power_skill_id: i32,
    /// Life or Jam channel (`UpdateGameStats` 4014).
    pub stat_kind: StatKind,
    /// Channel value for the tick.
    pub stat_value: u16,
    /// Hit sequence number for the tick.
    pub stat_sequence: i32,
    /// Running score for the tick.
    pub stat_score: u32,
    /// Cool hits (`SubmitScore` 4016).
    pub cool: u16,
    /// Good hits.
    pub good: u16,
    /// Bad hits.
    pub bad: u16,
    /// Misses.
    pub miss: u16,
    /// Best combo.
    pub max_combo: u16,
    /// Final jam level.
    pub jam: u16,
    /// Best jam level.
    pub max_jam: u16,
    /// Final score.
    pub final_score: u32,
    /// Remaining life.
    pub life: u8,
    /// Long-note bonus total (tick and tally).
    pub long_note_score: i32,
    /// How long to wait for the peer-bar echo (best effort).
    pub peer_bar_timeout: std::time::Duration,
}

impl SongFlowOptions {
    /// Live-proven synthetic tally for `music_id` (defaults mirror
    /// the fake-gateway test vectors).
    #[must_use]
    pub fn new(music_id: u16) -> Self {
        Self {
            music_id,
            difficulty: 2,
            speed: 1,
            arena_id: 0,
            member_id: 0,
            power_skill_id: 0,
            stat_kind: StatKind::Jam,
            stat_value: 3,
            stat_sequence: 1,
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
            long_note_score: 0,
            peer_bar_timeout: std::time::Duration::from_secs(3),
        }
    }

    /// Outgoing 4014 tick for the configured channel and score tail.
    ///
    /// Funneled through the live-stats score builders so the wire
    /// always carries saturated score state, never a hand-built body.
    #[must_use]
    pub fn tick(&self) -> UpdateGameStats {
        match self.stat_kind {
            StatKind::Life => life_tick_for_score(
                self.stat_value,
                u64::from(self.stat_score),
                self.stat_sequence,
                self.long_note_score,
            ),
            StatKind::Jam => jam_tick_for_score(
                u64::from(self.stat_score),
                u32::from(self.stat_value),
                self.stat_sequence,
                self.long_note_score,
            ),
        }
    }
}

/// What the host song flow learned.
#[derive(Debug)]
pub struct SongFlowReport {
    /// Catalog rows in the `GetMusicList` 4031 answer.
    pub music_entries: usize,
    /// Gateway `FreeMusic` flag from the 4031 answer.
    pub free_music: bool,
    /// Stage word the gateway echoed for our arena select.
    pub arena_echo: RoomArenaEcho,
    /// Peer lifebar echo after our tick (`None` on quiet rooms).
    pub peer_bar: Option<GameStatsUpdate>,
    /// Mirror after applying the echo (untouched on quiet rooms).
    pub peers: PeerBars,
    /// Queue forward from the echo (`None` when quiet/unknown).
    pub peer_queue: Option<QueueEvent>,
    /// Rank deliveries from the echo (empty when quiet).
    pub peer_ranks: Vec<RankDelivery>,
    /// Broadcast ids skipped while awaiting answers (kept, not lost).
    pub skipped: Vec<u16>,
}

/// How long a blocking read waits outside the peer-bar window.
///
/// Mirrors `GatewayConnection`'s own default (`client.rs`
/// `DEFAULT_IO_TIMEOUT`); restored after the peer-bar window so the
/// connection behaves as the login flow left it.
const SONG_FLOW_IO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Read until `expected` arrives, collecting broadcasts on the way.
///
/// Keepalives (6000/6001) stay silent; every other skipped id is
/// reported in `skipped` so no server dispatch vanishes quietly.
fn await_lenient(
    gateway: &mut GatewayConnection,
    expected: u16,
    skipped: &mut Vec<u16>,
) -> Result<Frame, o2jam_net::Error> {
    loop {
        let frame = gateway.next_frame()?;
        if frame.id == expected {
            return Ok(frame);
        }
        if Generic::from_id(frame.id).is_none() {
            skipped.push(frame.id);
        }
    }
}

/// Run the host song flow over an authorized, room-joined gateway.
///
/// Fire-and-forget sends carry the play steps; only the arena echo
/// (4002 → 4003) and the music list (4030 → 4031) plus the peer-bar
/// echo (4015, best effort) await replies. Returns what the list
/// step learned.
pub fn run_song_flow(
    gateway: &mut GatewayConnection,
    options: &SongFlowOptions,
) -> Result<SongFlowReport> {
    let mut skipped = Vec::new();
    send_select(gateway, options)?;
    let arena_echo = select_arena(gateway, options, &mut skipped)?;
    let listed = fetch_music_list(gateway, &mut skipped)?;
    send_sync(gateway, options)?;
    send_start(gateway, options)?;
    send_tick(gateway, options)?;
    let peer_bar = await_peer_bar(gateway, options, &mut skipped);
    let (peers, peer_queue, peer_ranks) = mirror_peer_bar(peer_bar);
    send_tally(gateway, options)?;
    gateway
        .set_timeouts(Some(SONG_FLOW_IO_TIMEOUT))
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    Ok(SongFlowReport {
        music_entries: listed.entries.len(),
        free_music: listed.free_music,
        arena_echo,
        peer_bar,
        peers,
        peer_queue,
        peer_ranks,
        skipped,
    })
}

fn send_select(gateway: &mut GatewayConnection, options: &SongFlowOptions) -> Result<()> {
    let select = SetRoomMusic {
        music_id: options.music_id,
        difficulty: options.difficulty,
        speed: options.speed,
    };
    gateway
        .send_frame(Request::SetRoomMusic.id(), &select.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))
}

/// Send the stage word and assert the gateway echoes it back.
///
/// A mismatched echo means the room stage did not take, so the
/// flow fails instead of playing on the wrong stage.
fn select_arena(
    gateway: &mut GatewayConnection,
    options: &SongFlowOptions,
    skipped: &mut Vec<u16>,
) -> Result<RoomArenaEcho> {
    let stage = SetRoomArena::new(options.arena_id);
    gateway
        .send_frame(Request::SetRoomArena.id(), &stage.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    let frame = await_lenient(gateway, RoomArenaEcho::COMMAND, skipped)
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    let echo = RoomArenaEcho::decode_frame(&frame).map_err(|source| anyhow::anyhow!("{source}"))?;
    if echo.arena != options.arena_id {
        bail!(
            "arena echo mismatch: sent {} but got {}",
            options.arena_id,
            echo.arena
        );
    }
    Ok(echo)
}

fn fetch_music_list(
    gateway: &mut GatewayConnection,
    skipped: &mut Vec<u16>,
) -> Result<MusicListAnswer> {
    gateway
        .send_frame(Request::GetMusicList.id(), &GetMusicList.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    let frame = await_lenient(gateway, MusicListAnswer::COMMAND, skipped)
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    MusicListAnswer::decode_frame(&frame).map_err(|source| anyhow::anyhow!("{source}"))
}

fn send_sync(gateway: &mut GatewayConnection, options: &SongFlowOptions) -> Result<()> {
    let state = SyncMemberMusicState {
        member_id: options.member_id,
    };
    gateway
        .send_frame(Request::SyncMemberMusicState.id(), &state.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    let progress = SetDownloadProgress {
        progress: 100,
        member_id: options.member_id,
    };
    gateway
        .send_frame(Request::SetDownloadProgress.id(), &progress.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))
}

fn send_start(gateway: &mut GatewayConnection, options: &SongFlowOptions) -> Result<()> {
    gateway
        .send_frame(Request::StartGame.id(), &StartGame.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    let loaded = ConfirmMusicLoaded {
        power_skill_id: options.power_skill_id,
        speed: options.speed,
    };
    gateway
        .send_frame(Request::ConfirmMusicLoaded.id(), &loaded.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))
}

fn send_tick(gateway: &mut GatewayConnection, options: &SongFlowOptions) -> Result<()> {
    gateway
        .send_frame(Request::UpdateGameStats.id(), &options.tick().encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))
}

/// Wait briefly for the tracker's peer-bar echo; `None` means the
/// room stayed quiet (solo rooms may), never a hard failure.
fn await_peer_bar(
    gateway: &mut GatewayConnection,
    options: &SongFlowOptions,
    skipped: &mut Vec<u16>,
) -> Option<GameStatsUpdate> {
    gateway.set_timeouts(Some(options.peer_bar_timeout)).ok()?;
    let frame = await_lenient(gateway, GameStatsUpdate::COMMAND, skipped).ok()?;
    GameStatsUpdate::decode_frame(&frame).ok()
}

/// Mirror one 4015 echo into fresh peer bars (quiet rooms stay empty).
fn mirror_peer_bar(
    echo: Option<GameStatsUpdate>,
) -> (PeerBars, Option<QueueEvent>, Vec<RankDelivery>) {
    let mut peers = PeerBars::new();
    let Some(update) = echo else {
        return (peers, None, Vec::new());
    };
    let applied = apply_peer_update(&mut peers, update);
    (peers, applied.queue, applied.ranks)
}

fn send_tally(gateway: &mut GatewayConnection, options: &SongFlowOptions) -> Result<()> {
    let tally = SubmitScore {
        cool: options.cool,
        good: options.good,
        bad: options.bad,
        miss: options.miss,
        max_combo: options.max_combo,
        jam: options.jam,
        max_jam: options.max_jam,
        score: options.final_score,
        life: options.life,
        speed: options.speed,
        long_note_score: options.long_note_score,
    };
    gateway
        .send_frame(Request::SubmitScore.id(), &tally.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    gateway
        .send_frame(Request::FinalizeRank.id(), &FinalizeRank.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))?;
    gateway
        .send_frame(Request::ExitPlaying.id(), &ExitPlaying.encode_body())
        .map_err(|source| anyhow::anyhow!("{source}"))
}

/// Connect, authorize, list channels, and optionally log into one.
pub fn connect_and_login(options: &ConnectOptions) -> Result<ConnectReport> {
    let address = format!("{}:{}", options.host, options.port);
    let mut gateway = GatewayConnection::connect(address.as_str())
        .with_context(|| format!("connecting to {address}"))?;
    let auth_code = authorize(&mut gateway, options).context("authorizing")?;
    let channels = fetch_channels(&mut gateway).context("listing channels")?;
    let channel_login = match options.channel {
        Some((gateway_id, channel_id)) => Some(
            login_channel(&mut gateway, gateway_id, channel_id).context("logging into channel")?,
        ),
        None => None,
    };
    let rooms = if channel_login.is_some() {
        fetch_rooms(&mut gateway).context("listing rooms")?
    } else {
        Vec::new()
    };
    let join = match (options.join_room, &channel_login) {
        (Some(number), Some(_)) => {
            Some(join_room(&mut gateway, number, &options.room_password).context("joining room")?)
        }
        _ => None,
    };
    let song = match (&options.song, &join) {
        (Some(flow), Some(_)) => {
            Some(run_song_flow(&mut gateway, flow).context("running song flow")?)
        }
        _ => None,
    };
    // Character info requires channel membership (server-verified:
    // `ChannelAuthorizeAttribute` rejects 2000 outside a channel),
    // so it runs last and only with a channel login.
    let character = if options.character {
        Some(fetch_character(&mut gateway).context("fetching character")?)
    } else {
        None
    };
    let avatar_layers = match (&options.avatar_opa, &character) {
        (Some(path), Some(hero)) => fetch_avatar(path, hero).context("loading avatar")?.len(),
        _ => 0,
    };
    Ok(ConnectReport {
        auth_code,
        character,
        avatar_layers,
        channels,
        channel_login,
        join,
        rooms,
        song,
    })
}

fn join_room(
    gateway: &mut GatewayConnection,
    number: i32,
    password: &str,
) -> Result<JoinRoomResponse> {
    let request = JoinRoom {
        number,
        music_state: 0,
        password: password.to_owned(),
    };
    let frame = gateway.exchange(
        Request::JoinWaiting.id(),
        &request.encode_body(),
        JoinRoomResponse::COMMAND,
    )?;
    let response = JoinRoomResponse::decode_frame(&frame)?;
    match o2jam_net::JoinResult::from_code(response.result_code) {
        Some(o2jam_net::JoinResult::Success) => Ok(response),
        _ => bail!("join rejected: {}", name_result(response.result_code)),
    }
}

fn fetch_rooms(gateway: &mut GatewayConnection) -> Result<Vec<RoomListEntry>> {
    let frame = gateway.exchange(Request::GetChannelInfo.id(), &[], RoomList::COMMAND)?;
    Ok(RoomList::decode_frame(&frame)?.rooms)
}

/// Human-readable authorize/channel result code.
#[must_use]
pub fn name_result(code: u32) -> String {
    match AuthResult::from_code(code) {
        Some(named) => format!("{named:?} ({code})"),
        None => format!("unknown code {code}"),
    }
}

/// Our socket address as IPv4 (login endpoints are IPv4-only).
fn local_ipv4(gateway: &GatewayConnection) -> Result<std::net::SocketAddrV4> {
    match gateway.local_address()? {
        SocketAddr::V4(endpoint) => Ok(endpoint),
        SocketAddr::V6(_) => bail!("gateway resolved IPv6; login endpoints are IPv4-only"),
    }
}

fn authorize(gateway: &mut GatewayConnection, options: &ConnectOptions) -> Result<u32> {
    let local = local_ipv4(gateway)?;
    let request = AuthorizeRequest::new(
        options.relay_key1,
        options.relay_key2,
        local,
        local,
        options.user_id,
        options.token.clone(),
        options.gender,
    );
    let frame = gateway.exchange(
        Request::Authorize.id(),
        &request.encode_body(),
        AuthResponse::COMMAND,
    )?;
    let response = AuthResponse::decode_frame(&frame)?;
    match AuthResult::from_code(response.result_code) {
        Some(AuthResult::Success) => Ok(response.result_code),
        _ => bail!("authorize rejected: {}", name_result(response.result_code)),
    }
}

/// Read a local avatar archive off disk.
///
/// File IO only: parsing and stacking live in [`stack_worn_figure`],
/// so the login flow never touches the filesystem except through this
/// one helper.
///
/// # Errors
///
/// The path is unreadable.
fn read_avatar_archive(opa_path: &str) -> Result<Vec<u8>> {
    std::fs::read(opa_path).with_context(|| format!("reading avatar archive {opa_path}"))
}

/// Stack the worn figure from an in-memory `avatar.opa` image: parse
/// its item table and layer every resolvable part. `Any` gender wears
/// the female parity (documented guess).
///
/// # Errors
///
/// The bytes are not an `.opa` archive, or its item table is missing
/// or corrupt.
pub fn stack_worn_figure(
    archive: &[u8],
    equipment: &[i32],
    male: bool,
) -> Result<o2jam_shell::AvatarFigure> {
    let entries = o2jam_file::parse_opi(archive)?;
    let table_entry = entries
        .iter()
        .find(|entry| entry.name == "Itemdata_Korea.dat")
        .with_context(|| "avatar.opa lacks Itemdata_Korea.dat")?;
    let table_bytes = archive
        .get(table_entry.data_offset..table_entry.data_offset + table_entry.size as usize)
        .with_context(|| "item table out of archive bounds")?;
    let table = o2jam_file::parse_avatar_item_table(table_bytes)?;
    Ok(o2jam_shell::load_worn_figure(
        archive, &table, equipment, male,
    ))
}

/// Stack a worn figure from a local `avatar.opa` and explicit
/// equipment ids (shared by `connect --avatar-opa` and
/// `play --window --avatar-opa --equip`).
///
/// # Errors
///
/// The archive is unreadable or unstackable (see [`stack_worn_figure`]).
pub fn build_worn_figure(
    opa_path: &str,
    equipment: &[i32],
    male: bool,
) -> Result<o2jam_shell::AvatarFigure> {
    let archive = read_avatar_archive(opa_path)?;
    stack_worn_figure(&archive, equipment, male)
}

/// Stack the worn figure for fetched character info from an in-memory
/// archive image (equipment and `Male` parity come from the hero).
///
/// # Errors
///
/// The archive image is unstackable (see [`stack_worn_figure`]).
fn figure_for_character(archive: &[u8], hero: &CharacterInfo) -> Result<o2jam_shell::AvatarFigure> {
    stack_worn_figure(archive, &hero.equipment, hero.gender == Gender::Male)
}

fn fetch_avatar(opa_path: &str, hero: &CharacterInfo) -> Result<o2jam_shell::AvatarFigure> {
    let archive = read_avatar_archive(opa_path)?;
    figure_for_character(&archive, hero)
}

/// Load one named sprite from an interface archive on disk.
pub fn load_interface_sprite(
    interface_path: &str,
    entry_name: &str,
) -> Result<o2jam_shell::AvatarFigure> {
    let archive = std::fs::read(interface_path)
        .with_context(|| format!("reading interface archive {interface_path}"))?;
    o2jam_shell::load_archive_sprite(&archive, entry_name)
        .with_context(|| format!("no decodable {entry_name} in {interface_path}"))
}

/// Parse `--equip` (`"1,5,9"`) into item ids; blank means none.
pub fn parse_equipment(list: &str) -> Result<Vec<i32>> {
    if list.trim().is_empty() {
        return Ok(Vec::new());
    }
    list.split(',')
        .map(|part| {
            part.trim()
                .parse::<i32>()
                .with_context(|| format!("bad item id '{part}'"))
        })
        .collect()
}

fn fetch_character(gateway: &mut GatewayConnection) -> Result<CharacterInfo> {
    let frame = gateway.exchange(Request::GetCharacterInfo.id(), &[], CharacterInfo::COMMAND)?;
    let info = CharacterInfo::decode_frame(&frame)?;
    if info.invalid {
        bail!("server flagged the account invalid");
    }
    Ok(info)
}

fn fetch_channels(gateway: &mut GatewayConnection) -> Result<Vec<ChannelState>> {
    let frame = gateway.exchange(Request::GetChannelList.id(), &[], ChannelList::COMMAND)?;
    Ok(ChannelList::decode_frame(&frame)?.channels)
}

fn login_channel(
    gateway: &mut GatewayConnection,
    gateway_id: i16,
    channel_id: i16,
) -> Result<ChannelLoginResponse> {
    let request = ChannelLogin {
        gateway_id,
        channel_id,
    };
    let frame = gateway.exchange(
        Request::ChannelLogin.id(),
        &request.encode_body(),
        ChannelLoginResponse::COMMAND,
    )?;
    let response = ChannelLoginResponse::decode_frame(&frame)?;
    match o2jam_net::ChannelLoginResult::from_code(response.result_code) {
        Some(o2jam_net::ChannelLoginResult::Success) => Ok(response),
        _ => bail!(
            "channel login rejected: {}",
            name_result(response.result_code)
        ),
    }
}

#[cfg(test)]
mod connect_flow_tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use o2jam_net::{
        frame::decode_frame, AuthResponse, ChannelList, ChannelLoginResponse, Frame, HEADER_LEN,
    };

    use super::*;

    /// Read one whole frame off the fake server's socket.
    fn read_frame(stream: &mut TcpStream) -> Frame {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
        let mut rest = vec![0u8; body_len - HEADER_LEN];
        stream.read_exact(&mut rest).unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&rest);
        decode_frame(&bytes).unwrap()
    }

    fn test_options(
        port: u16,
        channel: Option<(i16, i16)>,
        character: bool,
        avatar_opa: Option<String>,
    ) -> ConnectOptions {
        ConnectOptions {
            host: "127.0.0.1".to_owned(),
            port,
            user_id: 7,
            token: "tester".to_owned(),
            gender: Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel,
            character,
            avatar_opa,
            join_room: None,
            room_password: String::new(),
            song: None,
        }
    }

    #[test]
    fn loads_interface_sprite_from_disk() {
        let archive = opa_blob(&[("Logo.ojs", &red_sprite())]);
        let dir = std::env::temp_dir();
        let path = dir.join("recomp2jam-test-interface.opi");
        std::fs::write(&path, &archive).unwrap();
        let figure = load_interface_sprite(&path.to_string_lossy(), "Logo.ojs").unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(figure.len(), 1);
        assert!(load_interface_sprite("nonexistent.opi", "Logo.ojs").is_err());
    }

    #[test]
    fn parses_equipment_lists() {
        assert_eq!(parse_equipment("").unwrap(), Vec::<i32>::new());
        assert_eq!(parse_equipment("1,5,9").unwrap(), vec![1, 5, 9]);
        assert_eq!(parse_equipment(" 2 , 3 ").unwrap(), vec![2, 3]);
        assert!(parse_equipment("1,nope").is_err());
    }

    #[test]
    fn builds_worn_figure_from_archive() {
        let archive = opa_blob(&[
            ("Itemdata_Korea.dat", &item_table_blob()),
            ("AA.ojs", &red_sprite()),
        ]);
        let dir = std::env::temp_dir();
        let path = dir.join("recomp2jam-test-build-avatar.opa");
        std::fs::write(&path, &archive).unwrap();
        let figure = build_worn_figure(&path.to_string_lossy(), &[1], false).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(figure.len(), 1);
        assert!(build_worn_figure("nonexistent.opa", &[1], false).is_err());
    }

    #[test]
    fn parses_gender_words() {
        assert_eq!(parse_gender("male").unwrap(), Gender::Male);
        assert_eq!(parse_gender("F").unwrap(), Gender::Female);
        assert_eq!(parse_gender("any").unwrap(), Gender::Any);
        assert!(parse_gender("bogus").is_err());
    }

    #[test]
    fn logs_into_fake_gateway_end_to_end() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
            let login = read_frame(&mut stream);
            assert_eq!(login.id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
        });
        let report = connect_and_login(&test_options(port, Some((1, 2)), false, None)).unwrap();
        server.join().unwrap();
        assert_eq!(report.auth_code, 0);
        assert!(report.channels.is_empty());
        assert_eq!(
            report.channel_login,
            Some(ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            })
        );
    }

    #[test]
    fn fetches_character_after_channel_login() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetCharacterInfo.id());
            let hero = CharacterInfo {
                nickname: "tester".to_owned(),
                gender: Gender::Male,
                level: 3,
                ..Default::default()
            };
            stream.write_all(&hero.encode()).unwrap();
        });
        let report = connect_and_login(&test_options(port, Some((1, 2)), true, None)).unwrap();
        server.join().unwrap();
        let hero = report.character.unwrap();
        assert_eq!(hero.nickname, "tester");
        assert_eq!(hero.level, 3);
        assert!(report.channel_login.is_some());
    }

    /// Minimal sprite-55 body: 1x1 red pixel, colorkey black.
    fn red_sprite() -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 1u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x7C]);
        blob
    }

    /// Minimal `.opa`: header + bodies + trailing entry table.
    fn opa_blob(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        let mut blob = vec![0u8; 16];
        blob[..4].copy_from_slice(&1u32.to_le_bytes());
        blob[4..8].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        let mut off = 16u32;
        let mut table = Vec::new();
        for (name, body) in bodies {
            let mut entry = vec![0u8; 152];
            entry[..4].copy_from_slice(&1u32.to_le_bytes());
            entry[4..4 + name.len()].copy_from_slice(name.as_bytes());
            entry[132..136].copy_from_slice(&off.to_le_bytes());
            entry[136..140].copy_from_slice(&(body.len() as u32).to_le_bytes());
            entry[140..144].copy_from_slice(&(body.len() as u32).to_le_bytes());
            table.push(entry);
            off += body.len() as u32;
        }
        let mut out = blob;
        for (_, body) in bodies {
            out.extend_from_slice(body);
        }
        for entry in table {
            out.extend_from_slice(&entry);
        }
        out
    }

    /// Minimal item table: one row (id 1) wearing `AA.oji`.
    fn item_table_blob() -> Vec<u8> {
        let mut row = Vec::new();
        row.extend_from_slice(&1u32.to_le_bytes());
        row.extend_from_slice(&0x106u32.to_le_bytes());
        row.extend_from_slice(&[0u8; 22]);
        row.extend_from_slice(b"\x04\0\0\0hair\x03\0\0\0dsc");
        let mut names: Vec<(u8, &str)> = vec![(1, ""); 42];
        names[0] = (1, "i.oji");
        names[1] = (1, "i.oji");
        names[2] = (1, "AA.oji");
        for (flag, name) in names {
            row.push(flag);
            row.extend_from_slice(&(name.len() as u32).to_le_bytes());
            row.extend_from_slice(name.as_bytes());
        }
        let mut blob = Vec::new();
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(&[0u8; 80]);
        blob.extend_from_slice(&row);
        blob
    }

    #[test]
    fn stacks_worn_avatar_from_local_archive() {
        let archive = opa_blob(&[
            ("Itemdata_Korea.dat", &item_table_blob()),
            ("AA.ojs", &red_sprite()),
        ]);
        let dir = std::env::temp_dir();
        let path = dir.join("recomp2jam-test-avatar.opa");
        std::fs::write(&path, &archive).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetCharacterInfo.id());
            let mut hero = CharacterInfo {
                nickname: "tester".to_owned(),
                ..Default::default()
            };
            hero.equipment[0] = 1;
            stream.write_all(&hero.encode()).unwrap();
        });
        let options = test_options(
            port,
            Some((1, 2)),
            true,
            Some(path.to_string_lossy().into_owned()),
        );
        let report = connect_and_login(&options).unwrap();
        server.join().unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(report.avatar_layers, 1);
    }

    #[test]
    fn joins_room_after_channel_login() {
        use o2jam_net::{JoinRoomResponse, JoinedRoom};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
            let join = read_frame(&mut stream);
            assert_eq!(join.id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            let asked = read_frame(&mut stream);
            assert_eq!(asked.id, Request::JoinWaiting.id());
            let mut slots = Vec::new();
            for index in 0..7u8 {
                slots.push(o2jam_net::RoomSlot {
                    index,
                    state_code: 0,
                    member: None,
                });
            }
            let answered = JoinRoomResponse {
                result_code: 0,
                room: Some(JoinedRoom {
                    member_id: 0,
                    team: 0,
                    title: "Lobby".to_owned(),
                    music_id: 0,
                    arena: 0,
                    key_mode: 2,
                    game_mode: 0,
                    difficulty: 0,
                    speed: 1,
                    has_super_manager: false,
                    user_count: 1,
                    slots,
                    skills: vec![],
                    premium: false,
                    champion: None,
                    champion_streak: None,
                }),
            };
            stream.write_all(&answered.encode()).unwrap();
        });
        let mut options = test_options(port, Some((1, 2)), false, None);
        options.join_room = Some(3);
        let report = connect_and_login(&options).unwrap();
        server.join().unwrap();
        let room = report.join.unwrap().room.unwrap();
        assert_eq!(room.title, "Lobby");
        assert_eq!(room.user_count, 1);
    }

    #[test]
    fn refuses_failed_authorize() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let rejected = AuthResponse {
                result_code: 0xFFFF_FFFF,
                tail: vec![],
            };
            stream.write_all(&rejected.encode()).unwrap();
        });
        let outcome = connect_and_login(&test_options(port, None, false, None));
        server.join().unwrap();
        // Debug format carries the whole anyhow chain (`to_string`
        // shows only the outer context).
        let message = format!("{:?}", outcome.unwrap_err());
        assert!(message.contains("authorize rejected"), "{message}");
        assert!(message.contains("InvalidLogin"), "{message}");
    }
}
/// Avatar IO split: [`read_avatar_archive`] owns the filesystem,
/// [`stack_worn_figure`] and [`figure_for_character`] own the bytes.
#[cfg(test)]
mod avatar_io_tests {
    use super::*;

    /// Minimal sprite-55 body: 1x1 red pixel, colorkey black.
    fn red_sprite() -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 1u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x7C]);
        blob
    }

    /// Minimal `.opa`: header + bodies + trailing entry table.
    fn opa_blob(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        let mut blob = vec![0u8; 16];
        blob[..4].copy_from_slice(&1u32.to_le_bytes());
        blob[4..8].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        let mut off = 16u32;
        let mut table = Vec::new();
        for (name, body) in bodies {
            let mut entry = vec![0u8; 152];
            entry[..4].copy_from_slice(&1u32.to_le_bytes());
            entry[4..4 + name.len()].copy_from_slice(name.as_bytes());
            entry[132..136].copy_from_slice(&off.to_le_bytes());
            entry[136..140].copy_from_slice(&(body.len() as u32).to_le_bytes());
            entry[140..144].copy_from_slice(&(body.len() as u32).to_le_bytes());
            table.push(entry);
            off += body.len() as u32;
        }
        let mut out = blob;
        for (_, body) in bodies {
            out.extend_from_slice(body);
        }
        for entry in table {
            out.extend_from_slice(&entry);
        }
        out
    }

    /// Minimal item table: one row (id 1) wearing `AA.oji`.
    fn item_table_blob() -> Vec<u8> {
        let mut row = Vec::new();
        row.extend_from_slice(&1u32.to_le_bytes());
        row.extend_from_slice(&0x106u32.to_le_bytes());
        row.extend_from_slice(&[0u8; 22]);
        row.extend_from_slice(b"\x04\0\0\0hair\x03\0\0\0dsc");
        let mut names: Vec<(u8, &str)> = vec![(1, ""); 42];
        names[0] = (1, "i.oji");
        names[1] = (1, "i.oji");
        names[2] = (1, "AA.oji");
        for (flag, name) in names {
            row.push(flag);
            row.extend_from_slice(&(name.len() as u32).to_le_bytes());
            row.extend_from_slice(name.as_bytes());
        }
        let mut blob = Vec::new();
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(&[0u8; 80]);
        blob.extend_from_slice(&row);
        blob
    }

    fn worn_archive() -> Vec<u8> {
        opa_blob(&[
            ("Itemdata_Korea.dat", &item_table_blob()),
            ("AA.ojs", &red_sprite()),
        ])
    }

    #[test]
    fn stacks_figure_from_memory_without_disk() {
        let archive = worn_archive();
        let figure = stack_worn_figure(&archive, &[1], false).unwrap();
        assert_eq!(figure.len(), 1);
    }

    #[test]
    fn stacking_rejects_bare_buffers() {
        assert!(stack_worn_figure(&[], &[1], false).is_err());
        assert!(stack_worn_figure(&[0u8; 32], &[1], false).is_err());
        let unknown = stack_worn_figure(&worn_archive(), &[999], false).unwrap();
        assert_eq!(unknown.len(), 0);
    }

    #[test]
    fn missing_item_table_stays_an_error() {
        let archive = opa_blob(&[("AA.ojs", &red_sprite())]);
        assert!(stack_worn_figure(&archive, &[1], false).is_err());
    }

    #[test]
    fn archive_reader_round_trips_disk_bytes() {
        let archive = worn_archive();
        let path = std::env::temp_dir().join("recomp2jam-test-avatar-roundtrip.opa");
        std::fs::write(&path, &archive).unwrap();
        let back = read_avatar_archive(&path.to_string_lossy()).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(back, archive);
        assert!(read_avatar_archive("nonexistent-avatar.opa").is_err());
    }

    #[test]
    fn character_equipment_selects_worn_layers() {
        let archive = worn_archive();
        let mut equipped = CharacterInfo {
            nickname: "tester".to_owned(),
            ..Default::default()
        };
        equipped.equipment[0] = 1;
        let figure = figure_for_character(&archive, &equipped).unwrap();
        assert_eq!(figure.len(), 1);
        let bare = CharacterInfo {
            nickname: "tester".to_owned(),
            ..Default::default()
        };
        let nothing = figure_for_character(&archive, &bare).unwrap();
        assert_eq!(nothing.len(), 0);
    }

    #[test]
    fn male_and_other_heroes_both_stack() {
        let archive = worn_archive();
        let mut male = CharacterInfo {
            nickname: "tester".to_owned(),
            gender: Gender::Male,
            ..Default::default()
        };
        male.equipment[0] = 1;
        let mut other = male.clone();
        other.gender = Gender::Any;
        assert_eq!(
            figure_for_character(&archive, &male).unwrap().len(),
            figure_for_character(&archive, &other).unwrap().len(),
        );
    }
}
/// Host song flow over real gateway frames (plan Phases 3–4).
///
/// Fake-gateway proofs for [`run_song_flow`]: the fake server asserts
/// every request id and body, then answers the list step and the
/// peer-bar echo. One env-gated live test replays the same flow
/// against a local Memoryer gateway.
#[cfg(test)]
mod song_flow_tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use o2jam_net::{
        frame::{decode_frame, encode_frame},
        play::{MusicListAnswer, RoomArenaEcho},
        ConfirmMusicLoaded, Frame, GameStatsUpdate, GatewayConnection, Request,
        SetDownloadProgress, SetRoomArena, SetRoomMusic, StatKind, SubmitScore,
        SyncMemberMusicState, UpdateGameStats, WireMessage, HEADER_LEN,
    };

    use super::{run_song_flow, SongFlowOptions};

    /// Read one whole frame off the fake server's socket.
    fn read_frame(stream: &mut TcpStream) -> Frame {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
        let mut rest = vec![0u8; body_len - HEADER_LEN];
        stream.read_exact(&mut rest).unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&rest);
        decode_frame(&bytes).unwrap()
    }

    fn flow_options() -> SongFlowOptions {
        let mut options = SongFlowOptions::new(757);
        options.peer_bar_timeout = Duration::from_millis(200);
        options
    }

    fn list_answer() -> MusicListAnswer {
        use o2jam_net::play::MusicListEntry;
        MusicListAnswer {
            entries: vec![MusicListEntry {
                id: 757,
                notes_ex: 100,
                notes_nx: 200,
                notes_hx: 300,
            }],
            free_music: true,
        }
    }

    #[test]
    fn parses_stat_kind_words() {
        use super::parse_stat_kind;
        assert_eq!(parse_stat_kind("life").unwrap(), StatKind::Life);
        assert_eq!(parse_stat_kind("Jam").unwrap(), StatKind::Jam);
        assert_eq!(parse_stat_kind("1").unwrap(), StatKind::Jam);
        assert!(parse_stat_kind("bogus").is_err());
    }

    #[test]
    fn host_song_flow_reaches_exit_over_fake_gateway() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let select = read_frame(&mut stream);
            assert_eq!(select.id, Request::SetRoomMusic.id());
            assert_eq!(
                SetRoomMusic::decode_body(&select.payload).unwrap(),
                SetRoomMusic {
                    music_id: 757,
                    difficulty: 2,
                    speed: 1,
                }
            );
            // A room broadcast lands before the arena echo (as live:
            // the gateway emits `RoomParamsChanged` on select).
            stream.write_all(&encode_frame(4001, &[])).unwrap();
            let stage = read_frame(&mut stream);
            assert_eq!(stage.id, Request::SetRoomArena.id());
            assert_eq!(
                SetRoomArena::decode_body(&stage.payload).unwrap(),
                SetRoomArena::new(0)
            );
            let echoed = RoomArenaEcho::new(0);
            stream.write_all(&echoed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetMusicList.id());
            stream.write_all(&list_answer().encode()).unwrap();
            let sync = read_frame(&mut stream);
            assert_eq!(sync.id, Request::SyncMemberMusicState.id());
            assert_eq!(
                SyncMemberMusicState::decode_body(&sync.payload).unwrap(),
                SyncMemberMusicState { member_id: 0 }
            );
            let progress = read_frame(&mut stream);
            assert_eq!(progress.id, Request::SetDownloadProgress.id());
            assert_eq!(
                SetDownloadProgress::decode_body(&progress.payload).unwrap(),
                SetDownloadProgress {
                    progress: 100,
                    member_id: 0,
                }
            );
            assert_eq!(read_frame(&mut stream).id, Request::StartGame.id());
            let loaded = read_frame(&mut stream);
            assert_eq!(loaded.id, Request::ConfirmMusicLoaded.id());
            assert_eq!(
                ConfirmMusicLoaded::decode_body(&loaded.payload).unwrap(),
                ConfirmMusicLoaded {
                    power_skill_id: 0,
                    speed: 1,
                }
            );
            let tick = read_frame(&mut stream);
            assert_eq!(tick.id, Request::UpdateGameStats.id());
            let stats = UpdateGameStats::decode_body(&tick.payload).unwrap();
            assert_eq!(stats.kind, StatKind::Jam);
            assert_eq!(stats.value, 3);
            let echo = GameStatsUpdate {
                member_id: 0,
                kind_code: StatKind::Life.as_u16(),
                value: 1000,
                ranks: [0; 8],
            };
            stream.write_all(&echo.encode()).unwrap();
            let tally = read_frame(&mut stream);
            assert_eq!(tally.id, Request::SubmitScore.id());
            let score = SubmitScore::decode_body(&tally.payload).unwrap();
            assert_eq!(score.score, 2210);
            assert_eq!(score.life, 100);
            assert_eq!(read_frame(&mut stream).id, Request::FinalizeRank.id());
            assert_eq!(read_frame(&mut stream).id, Request::ExitPlaying.id());
        });
        let mut gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let report = run_song_flow(&mut gateway, &flow_options()).unwrap();
        server.join().unwrap();
        assert_eq!(report.music_entries, 1);
        assert!(report.free_music);
        assert_eq!(report.arena_echo.arena, 0);
        assert_eq!(report.skipped, vec![4001]);
        let peer = report.peer_bar.unwrap();
        assert_eq!(peer.member_id, 0);
        assert_eq!(peer.kind(), Some(StatKind::Life));
        assert_eq!(peer.value, 1000);
        assert_eq!(report.peers.life(0), Some(1000));
        assert_eq!(report.peers.jam(0), Some(0));
        let queue = report.peer_queue.expect("life echo forwards a queue event");
        assert_eq!(queue.slot, 0);
        assert_eq!(queue.stat_kind, StatKind::Life.as_u16());
        assert_eq!(report.peer_ranks.len(), 8);
        assert_eq!(report.peers.rank_row(0), Some(0));
    }

    #[test]
    fn song_flow_tolerates_a_quiet_room() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::SetRoomMusic.id());
            let stage = read_frame(&mut stream);
            assert_eq!(stage.id, Request::SetRoomArena.id());
            assert_eq!(
                SetRoomArena::decode_body(&stage.payload).unwrap(),
                SetRoomArena::new(0)
            );
            let echoed = RoomArenaEcho::new(0);
            stream.write_all(&echoed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetMusicList.id());
            stream.write_all(&list_answer().encode()).unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                Request::SyncMemberMusicState.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                Request::SetDownloadProgress.id()
            );
            assert_eq!(read_frame(&mut stream).id, Request::StartGame.id());
            assert_eq!(read_frame(&mut stream).id, Request::ConfirmMusicLoaded.id());
            assert_eq!(read_frame(&mut stream).id, Request::UpdateGameStats.id());
            // No peer-bar echo: the client must not hang.
            assert_eq!(read_frame(&mut stream).id, Request::SubmitScore.id());
            assert_eq!(read_frame(&mut stream).id, Request::FinalizeRank.id());
            assert_eq!(read_frame(&mut stream).id, Request::ExitPlaying.id());
        });
        let mut gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let report = run_song_flow(&mut gateway, &flow_options()).unwrap();
        server.join().unwrap();
        assert_eq!(report.music_entries, 1);
        assert_eq!(report.arena_echo.arena, 0);
        assert!(report.peer_bar.is_none());
        assert!(report.skipped.is_empty());
        assert_eq!(report.peers.life_levels(), [0; 8]);
        assert_eq!(report.peer_queue, None);
        assert!(report.peer_ranks.is_empty());
    }

    /// Drive the standard flow against a fake gateway, answering the
    /// arena select with `arena_echoed` and the peer-bar window with
    /// `peer_echo` (`None` stays quiet).
    fn run_flow_with_arena_and_echo(
        arena_sent: i32,
        arena_echoed: i32,
        peer_echo: Option<GameStatsUpdate>,
    ) -> anyhow::Result<super::SongFlowReport> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::SetRoomMusic.id());
            let stage = read_frame(&mut stream);
            assert_eq!(stage.id, Request::SetRoomArena.id());
            assert_eq!(
                SetRoomArena::decode_body(&stage.payload).unwrap(),
                SetRoomArena::new(arena_sent)
            );
            let echoed = RoomArenaEcho::new(arena_echoed);
            stream.write_all(&echoed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetMusicList.id());
            stream.write_all(&list_answer().encode()).unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                Request::SyncMemberMusicState.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                Request::SetDownloadProgress.id()
            );
            assert_eq!(read_frame(&mut stream).id, Request::StartGame.id());
            assert_eq!(read_frame(&mut stream).id, Request::ConfirmMusicLoaded.id());
            let tick = read_frame(&mut stream);
            assert_eq!(tick.id, Request::UpdateGameStats.id());
            assert_eq!(
                UpdateGameStats::decode_body(&tick.payload).unwrap(),
                flow_options().tick()
            );
            if let Some(echo) = peer_echo {
                stream.write_all(&echo.encode()).unwrap();
            }
            assert_eq!(read_frame(&mut stream).id, Request::SubmitScore.id());
            assert_eq!(read_frame(&mut stream).id, Request::FinalizeRank.id());
            assert_eq!(read_frame(&mut stream).id, Request::ExitPlaying.id());
        });
        let mut gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut options = flow_options();
        options.arena_id = arena_sent;
        let report = run_song_flow(&mut gateway, &options);
        server.join().unwrap();
        report
    }

    /// Drive the standard flow against a fake gateway, answering the
    /// peer-bar window with `echo` (`None` stays quiet).
    fn run_flow_with_echo(echo: Option<GameStatsUpdate>) -> super::SongFlowReport {
        run_flow_with_arena_and_echo(0, 0, echo).unwrap()
    }

    #[test]
    fn jam_echo_mirrors_jam_word_and_ranks() {
        let report = run_flow_with_echo(Some(GameStatsUpdate {
            member_id: 1,
            kind_code: StatKind::Jam.as_u16(),
            value: 42,
            ranks: [0; 8],
        }));
        let peer = report.peer_bar.unwrap();
        assert_eq!(peer.kind(), Some(StatKind::Jam));
        assert_eq!(report.peers.jam(1), Some(42));
        assert_eq!(report.peers.life(1), Some(0));
        let queue = report.peer_queue.expect("jam echo forwards a queue event");
        assert_eq!(queue.stat_kind, StatKind::Jam.as_u16());
        assert_eq!(report.peer_ranks.len(), 8);
        assert_eq!(report.peers.rank_row(0), Some(0));
    }

    #[test]
    fn unknown_channel_echo_keeps_bars_but_broadcasts_ranks() {
        let report = run_flow_with_echo(Some(GameStatsUpdate {
            member_id: 1,
            kind_code: 9,
            value: 7,
            ranks: [0; 8],
        }));
        assert_eq!(report.peer_bar.unwrap().kind(), None);
        assert_eq!(report.peer_queue, None);
        assert_eq!(report.peers.life(1), Some(0));
        assert_eq!(report.peers.jam(1), Some(0));
        assert_eq!(report.peer_ranks.len(), 8);
        assert_eq!(report.peers.rank_row(0), Some(0));
    }

    #[test]
    fn arena_stage_sends_word_and_asserts_echo() {
        let report = run_flow_with_arena_and_echo(7, 7, None).unwrap();
        assert_eq!(report.arena_echo.arena, 7);
    }

    #[test]
    fn arena_echo_mismatch_fails_the_flow() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::SetRoomMusic.id());
            let stage = read_frame(&mut stream);
            assert_eq!(stage.id, Request::SetRoomArena.id());
            let echoed = RoomArenaEcho::new(8);
            stream.write_all(&echoed.encode()).unwrap();
        });
        let mut options = flow_options();
        options.arena_id = 7;
        let mut gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let outcome = run_song_flow(&mut gateway, &options);
        server.join().unwrap();
        let message = format!("{:?}", outcome.unwrap_err());
        assert!(message.contains("arena echo mismatch"), "{message}");
    }

    #[test]
    fn song_options_tick_names_its_channel() {
        let mut options = flow_options();
        assert_eq!(options.tick().kind, StatKind::Jam);
        options.stat_kind = StatKind::Life;
        options.stat_value = 1000;
        let tick = options.tick();
        assert_eq!(tick.kind, StatKind::Life);
        assert_eq!(tick.value, 1000);
        assert_eq!(tick.sequence, options.stat_sequence);
    }

    /// Live proof against a local Memoryer gateway (plan Phase 4).
    ///
    /// Guarded: runs only with `O2JAM_LIVE_GATEWAY=host:port` and
    /// `O2JAM_LIVE_TOKEN=session-guid` (mint via the gateway's
    /// `user:authorize` flow; user id 1, channel 0/0). Creates its own
    /// room (so the flow hosts), drives the full song flow, and checks
    /// the list answer plus the peer-bar echo.
    #[test]
    fn live_host_song_flow_against_local_gateway() {
        use super::{connect_and_login, ConnectOptions};
        use o2jam_net::{
            AuthResponse, AuthorizeRequest, ChannelLogin, ChannelLoginResponse, CharacterInfo,
            CreateRoom, CreateRoomResponse, GameMode, Gender, KeyMode,
        };

        let address = std::env::var("O2JAM_LIVE_GATEWAY").unwrap_or_default();
        if address.is_empty() {
            eprintln!("skipping live song flow (set O2JAM_LIVE_GATEWAY=host:port)");
            return;
        }
        let token = std::env::var("O2JAM_LIVE_TOKEN")
            .expect("O2JAM_LIVE_TOKEN must hold a gateway session guid");
        let (host, port) = address.rsplit_once(':').expect("host:port");
        let port: u16 = port.parse().expect("numeric port");

        let options = ConnectOptions {
            host: host.to_owned(),
            port,
            user_id: 1,
            token,
            gender: Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel: Some((0, 0)),
            character: true,
            avatar_opa: None,
            join_room: None,
            room_password: String::new(),
            song: None,
        };
        // Log in on a raw gateway: connect_and_login without a room
        // leaves membership setup to this test (it must host).
        // Channel entry races broadcasts (notably 2026), so every
        // await below skips foreign frames like production does.
        fn expect(gateway: &mut GatewayConnection, want: u16) -> o2jam_net::Frame {
            loop {
                let frame = gateway.next_frame().unwrap();
                if frame.id == want {
                    return frame;
                }
            }
        }
        let mut gateway = GatewayConnection::connect(address.as_str()).unwrap();
        let local = match gateway.local_address().unwrap() {
            std::net::SocketAddr::V4(endpoint) => endpoint,
            std::net::SocketAddr::V6(_) => panic!("gateway resolved IPv6"),
        };
        let auth = AuthorizeRequest::new(0, 0, local, local, 1, options.token.clone(), Gender::Any);
        gateway
            .send_frame(Request::Authorize.id(), &auth.encode_body())
            .unwrap();
        let authed =
            AuthResponse::decode_frame(&expect(&mut gateway, AuthResponse::COMMAND)).unwrap();
        assert_eq!(authed.result_code, 0, "live authorize");
        gateway
            .send_frame(Request::GetChannelList.id(), &[])
            .unwrap();
        assert_eq!(expect(&mut gateway, 1003).id, 1003);
        let login = ChannelLogin {
            gateway_id: 0,
            channel_id: 0,
        };
        gateway
            .send_frame(Request::ChannelLogin.id(), &login.encode_body())
            .unwrap();
        let logged = ChannelLoginResponse::decode_frame(&expect(
            &mut gateway,
            ChannelLoginResponse::COMMAND,
        ))
        .unwrap();
        assert_eq!(logged.result_code, 0, "live channel login");
        gateway
            .send_frame(Request::GetCharacterInfo.id(), &[])
            .unwrap();
        let hero =
            CharacterInfo::decode_frame(&expect(&mut gateway, CharacterInfo::COMMAND)).unwrap();
        assert!(!hero.invalid, "live character valid");

        let create = CreateRoom {
            title: "songflow".to_owned(),
            key_mode: KeyMode::FiveKeys,
            game_mode: GameMode::Normal,
            has_password: false,
            password: String::new(),
            min_level: 1,
            max_level: 99,
            premium: false,
        };
        gateway
            .send_frame(Request::CreateRoom.id(), &create.encode_body())
            .unwrap();
        let created =
            CreateRoomResponse::decode_frame(&expect(&mut gateway, CreateRoomResponse::COMMAND))
                .unwrap();
        assert_eq!(created.result.as_i32(), 0, "live room created");
        assert!(created.info.is_some(), "live room snapshot");

        let mut flow = SongFlowOptions::new(757);
        flow.arena_id = 1;
        let report = run_song_flow(&mut gateway, &flow).unwrap();
        assert!(
            report.music_entries > 0,
            "live catalog non-empty ({})",
            report.music_entries
        );
        assert!(report.free_music, "live gateway runs FreeMusic");
        assert_eq!(
            report.arena_echo.arena, 1,
            "live gateway echoes the arena stage"
        );
        assert!(
            report.peer_bar.is_some(),
            "live tracker echoes the tick (skipped={:?})",
            report.skipped
        );

        // The shared login helper stays the CLI path: prove it still
        // reaches character over the same gateway (separate session).
        // The shared login helper stays the CLI path: prove it still
        // reaches character over the same gateway once this session
        // drains (the gateway holds the token briefly after close).
        drop(gateway);
        let mut helper = Err(anyhow::anyhow!("not attempted"));
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(500));
            helper = connect_and_login(&ConnectOptions {
                song: None,
                ..options.clone()
            });
            if helper.is_ok() {
                break;
            }
        }
        assert!(helper.is_ok(), "helper login still green: {helper:?}");
    }
}
