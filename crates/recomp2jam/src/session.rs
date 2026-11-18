//! Online session: the transport owner for chat plus the live stats bridge.
//!
//! The shell builds the 3011 packet ([`ScreenGame::take_chat_message`])
//! but owns no transport, so an offline Enter drops it. The session
//! closes that gap: it holds the [`GatewayConnection`] beside the
//! [`ScreenGame`] and transmits the draft on send (Enter). It
//! implements [`GameLoop`], so the window shell can own the session
//! instead of the bare game.
//!
//! Stats bridge (Phase 4 play loop): every paint in `Playing` polls the
//! live score snapshot (row 0 of [`ScreenGame::result_rank_rows`] carries
//! the local points, jam level, and hold bonus, plus [`ScreenGame::life`]
//! for the life word row 0 never carries) and sends one 4014 jam tick
//! through [`live_stats::jam_tick_for_score`] plus one 4014 life tick
//! through [`live_stats::life_tick_for_live_play`] when the snapshot moved
//! since the last send — the per-hit shape: the wire only moves when play
//! does. The same paint then drains waiting 4015 echoes under a short
//! timeout ([`connect::run_song_flow`]'s peer-bar window shape) and folds
//! each into the owned [`PeerBars`] through
//! [`live_stats::apply_peer_update`] and into the shell through
//! [`ScreenGame::apply_peer_echo`], so the Playing paint shows the live
//! gauges. Chat stays send-on-action, as-is.
//!
//! Room join (Room scene): with `--online-join-room` the session
//! queues a `JoinWaiting` (3002) attempt on Room Confirm and stages
//! the joined snapshot through [`ScreenGame::set_joined_room`] — the
//! `connect --join-room` shape — on the per-frame poll before the
//! poll replays the Room → Waiting enter, so a silent server never
//! freezes the frame loop (ticket #172). Without a join target
//! Confirm stays a local enter (the offline walk), and a failed or
//! rejected join keeps the Room scene with a warning, so the next
//! Confirm retries it.
//!
//! Waiting chat (Waiting scene): every paint drains inbound 3012
//! player lines and 3013 administrator lines under the same short
//! timeout as the notice window and folds each into the waiting
//! history through [`ScreenGame::push_waiting_user_message`] and
//! [`ScreenGame::push_waiting_admin_message`] — the same
//! best-effort shape as the 4015 echo drain: keepalives skip
//! silently, foreign ids log and skip, undecodable bodies drop with
//! a trace.
//!
//! Ready and START (Waiting scene): an empty-draft Confirm with a
//! joined room means readiness, not chat — a guest flips its Ready
//! 4008 toggle, a master sends START 4010 only while
//! [`ScreenGame::can_start_game`] holds the all-peers-ready gate,
//! and the shell reaches Loading on the 4011 start broadcast
//! (drained beside the chat lines, applied after). The 4010 send
//! mirrors the headless [`connect::run_song_flow`] StartGame shape
//! (empty body, no 4012 tail — Loading owns that). Member updates
//! (3009 slot, 4009 ready echo) decode beside the chat lines and fold
//! into the session slot/ready mirrors below; the shell exposes no
//! per-seat write yet, so the barrier still proves through staged
//! snapshots.
//!
//! Loading confirm (Loading scene): a joined Confirm sends
//! `ConfirmMusicLoaded` 4012 (power-skill id 0 — no CLI or wire
//! source names one — plus the staged pick's speed, else the joined
//! room's) and enters Playing; a failed send keeps Loading, so the
//! next Confirm retries it. The offline walk keeps its local
//! Loading → Playing enter with no packet. Guests reach Loading on
//! the same 4011 broadcast through
//! `ScreenGame::force_advance_to_loading` — the START Confirm gate
//! passes masters only, so a guest Confirm could never satisfy it.
//!
//! Result tally (Result scene): the first paint in Result after a
//! joined play sends `SubmitScore` 4016 built from the live play
//! score (counts, best combo, jam level and peak, saturated points,
//! life, and hold bonus — the headless `connect::run_song_flow`
//! tally shape), then `FinalizeRank` 4041 and `ExitPlaying` 4021 in
//! the same order, and stages the battle mirror from the joined
//! seats (teams per occupied slot, the local total on our seat, our
//! team claimed) through `ScreenGame::set_battle_teams`. A failed
//! send retries on the next paint; later paints send nothing, and
//! leaving Result clears the latch. Every Result paint then drains
//! the tally answers — 4017 submissions (traced per member), the
//! 4018 ranked table (each active row fills the exe-pinned
//! [`result_records::ResultMemberRecord`] words before its total is
//! re-staged onto the seat, so the verdict turns on
//! server-authoritative peer totals), and 4040
//! rank places (decoded and validated; the gateway never emits it —
//! the `FinalizeRank` broadcast stays commented out server-side).
//! Result input wires (relay shapes pinned by #87, landed in #88):
//! every paint drains the staged result presses through the gateway —
//! RETRY posts `StartGame` 4010 (`0xfaa`, empty), BACK posts
//! `ExitPlaying` 4021 (`0xfb5`, empty, the same body the entry drain
//! sends), and a battle (Live) result entry posts `GetLiveState` 4042
//! (`0xfca`, empty, the exe's battle-global-gated send). The solo
//! entry's event-ticket web call (`ticket_add.php?USER_INDEX_ID=<index>`,
//! file `0x24a338`) is an HTTP GET, not a gateway frame — it stays
//! OPEN until the shell owns a web seam, and the gateway is never
//! asked to stand in for it. Retry-while-siblings-ready stays
//! server-side (the block string VA `0x64a3a0` is pinned in
//! `docs/cresult-anchors.md`); the shell never reimplements the rule.
//!
//! Host song select (Waiting scene): `--online-music-id` stages a
//! pending pick plus the installed `OJNList.dat` catalog; the next
//! master Waiting Confirm queues it, and the per-frame poll sends
//! `SetRoomMusic` 4000, `SetRoomArena` 4002 (awaiting the 4003 echo
//! like the headless song flow), and `GetMusicList` 4030, stages the
//! pick through [`ScreenGame::set_selected_music`] (driving the
//! dialog rows and the waiting `MUSICNAME` caption), then fetches the chart
//! from the catalog container (`Music/o2ma###.ojn` through the
//! loading gate) into the highway — all without blocking the frame
//! loop on a silent server (ticket #172). The room still launches on
//! a later Confirm (START 4010 behind the all-peers-ready gate), so
//! the host sees the staged pick first. Guests keep the READY path,
//! and a failed select keeps the pick for the next Confirm.
//!
//! Lobby send (Channel scene and Room lobby): the shell stages one
//! draft per lobby chat panel ([`ScreenGame::take_lobby_slash`]) beside
//! the panel's history and transmits it on Confirm through
//! [`OnlineSession::flush_channel_action`] (Channel) or
//! [`OnlineSession::flush_room_chat_action`] (Room) — plain chat as
//! 2012, `/w <name> <text>` and `/r <text>` as a 2017 whisper, `/h`
//! and `/help` with no packet — the same peek-send-clear bridge as the
//! waiting 3011 path ([`OnlineSession::flush_chat_action`]). The room
//! lobby chat IS the main-room chat (ticket #37), so both lobbies ride
//! the same opcode pair.
//!
//! Lobby inbox (Channel scene and Room lobby): every paint drains
//! inbound 2013 player lines, 2014 administrator lines, and 2019
//! whispers beside the 2016/2029 notices, folding each into the panel
//! of the lobby on screen through its [`ScreenGame`] push — the same
//! best-effort shape as the 4015 echo drain (undecodable bodies drop
//! with a trace; the shell push rules decide which rows stage).
//!
//! Lobby notices (Channel scene and Room lobby): every paint drains
//! 2016 announcements and 2029 rank notices under the same short
//! timeout as the stats echo window and folds each into that lobby
//! panel through [`ScreenGame::push_lobby_announcement`] and
//! [`ScreenGame::push_lobby_rank_notification`] — the same best-effort
//! shape as the 4015 echo drain: keepalives skip silently, foreign ids
//! log and skip, undecodable bodies drop with a trace. Empty notices
//! and invalid ranks stage no row (the shell push rules), so the drain
//! never invents one.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use o2jam_file::{InstalledMusic, OjnListEntry, OJN_LEVEL_COUNT};
use o2jam_game::{PeerBars, QueueEvent, RankDelivery, RoomTeams};
use o2jam_net::gift::{ClaimGiftResponse, GetGiftMessageList, GetGiftMessageListResponse};
use o2jam_net::play::{
    GameRankUpdate, MusicListAnswer, RoomArenaEcho, ScoreCompleted, ScoreSubmission,
};
use o2jam_net::rooms::{
    Announcement, MainRoomAdminMessage, MainRoomUserMessage, RankNotification, ReceiveWhisper,
    RoomList, RoomSlotAction, RoomSlotUpdate, SendMainRoomMessage, SendWaitingMessage, SendWhisper,
    WaitingAdminMessage, WaitingUserMessage,
};
use o2jam_net::shop::{
    SyncItemPurchase, SyncItemPurchasePrefix, SyncMusicPurchase, SyncMusicPurchaseResponse,
};
use o2jam_net::EmptyMessage;
use o2jam_net::{
    AcquireMusic, AcquireMusicRequest, ChannelLogin, ChannelLoginResponse, ChannelLoginResult,
    ChannelState, CharacterInfo, ConfirmMusicLoaded, CreateResult, CreateRoom, CreateRoomResponse,
    DownloadProgressChanged, Event, ExitPlaying, FinalizeRank, Frame, GameMode, GameStatsUpdate,
    GatewayConnection, Generic, GetMusicList, JoinResult, JoinRoom, JoinRoomResponse, JoinedRoom,
    KeyMode, MemberReadyStateChanged, MusicState, Ready, Request, SetDownloadProgress,
    SetRoomArena, SetRoomMusic, StartGame, SubmitScore, SyncMemberMusicStateEvent,
    SyncMusicDownload, UpdateMusicState, WaitingMusicChanged, WireMessage,
};
use o2jam_shell::screens::game::{LobbyChat, SlashCommand, StartGateVerdict, StartRequest};
use o2jam_shell::screens::SideScreen;
use o2jam_shell::{GameLoop, InputEdge, InputSnapshot, MenuKey, Screen, ScreenGame};

use crate::connect::net_context;
mod attempt;
mod press;
use crate::live_art::restage_live_playing;
use crate::live_stats::{apply_peer_update, jam_tick_for_score, life_tick_for_live_play};
use crate::loading::{
    load_sound_bank, loading_phase_status, prepare_chart, stage_loading_cover, PreparedChart,
    LOADING_GATE_STEPS,
};
use press::{EntryEcho, PressDisposition};
mod result_records;

use result_records::ResultMemberRecord;

/// Short socket window for the per-frame stats section (one tick send
/// plus the echo drain): a paint must never stall the frame loop on a
/// quiet or laggy gateway. A dropped tick retries on the next moved
/// snapshot; a missed echo window retries on the next paint.
const STATS_IO_TIMEOUT: Duration = Duration::from_millis(1);
/// Short socket window for the per-frame chat sections (the
/// 3012/3013 and 2013/2014/2019 drains): the same never-stall shape
/// as the stats section — a missed chat window retries on the next
/// paint.
const CHAT_IO_TIMEOUT: Duration = Duration::from_millis(1);
/// Short socket window for the per-frame notice section (the 2016/2029
/// drain): the same never-stall shape as the stats section — a missed
/// notice window retries on the next paint.
const NOTICE_IO_TIMEOUT: Duration = Duration::from_millis(1);
/// Socket window for the per-frame tally section (the 4017/4018/4040
/// answer drain): wider than the 1 ms chat/stats polls because the
/// server computes rewards between our submit and its broadcast, but
/// still never stalls the frame loop — a late table retries on the
/// next paint while Result stays up.
const TALLY_IO_TIMEOUT: Duration = Duration::from_millis(100);
/// Resting gateway timeout outside the stats section: the transport
/// default (`client.rs` `DEFAULT_IO_TIMEOUT`), restored after every
/// stats section like `run_song_flow` restores `SONG_FLOW_IO_TIMEOUT`.
const SESSION_IO_TIMEOUT: Duration = Duration::from_secs(10);
/// Short socket window for each pending-wire drain (ticket #172):
/// the same never-stall shape as the chat section — one paint moves
/// one leg at most (a send plus whatever answers already arrived),
/// and a quiet gateway simply retries on the next paint.
const PENDING_DRAIN_TIMEOUT: Duration = Duration::from_millis(1);
/// How long a Confirm-paced attempt waits for its answers across
/// paints (ticket #172): the bound a silent server gets before the
/// attempt warns and clears while the game stays responsive. One
/// resting timeout covers the whole sequence — the old blocking
/// exchanges waited one resting timeout per leg.
const PENDING_WIRE_WINDOW: Duration = Duration::from_secs(10);
/// Idle repaint gap for wire-driven lobby scenes (ticket #138): the
/// lobby drains ride `paint` (`poll_lobby_chat` on Channel/Room,
/// `poll_waiting_chat` plus `poll_start` on Waiting, `poll_tally` on
/// Result), so a modest heartbeat keeps chat, START, and tally arriving
/// while the board sits untouched. 250 ms bounds the added latency under
/// a keystroke; the 1 ms drain timeouts bound each heartbeat's cost
/// (the tally send stays join-gated; its 100 ms drain fires on Result).
const SESSION_POLL_HEARTBEAT: Duration = Duration::from_millis(250);
/// C->S ready toggle (4008): the joined-guest Confirm send. Empty
/// body — see [`Ready`]: all five exe send sites push a zero word
/// then the opcode (`push 0; push 0xfa8`, files `0xacebe`, `0xb24f6`,
/// `0xb36a0`, `0xb8450`, `0xb871f`). Memoryer agrees
/// (`WaitingController.Ready()` takes no request). The S->C echo is
/// 4009 (`docs/client-server-plan.md` Phase 3 pairs 4008/4009 like
/// 4000/4001). The old one-flag-byte send was a documented guess —
/// the exe sites refute it.
///
/// S->C start broadcast (4011): answers the host `StartGame` 4010
/// (`docs/client-server-plan.md` Phase 4 pairs 4010/4011).
///
/// `docs/music-availability.md` §4: the server answers `Result = None`
/// (-1) and does not start while any member's music state is not
/// Ready, so the result word decides whether the shell may advance. The
/// exe's waiting scenes ignore the id (it maps to their default arm);
/// the refusal is what this read honours.
const START_BROADCAST_COMMAND: u16 = 4011;

/// Optional local mirror of the exe's music FTP host: a folder holding
/// `o2ma<id>.ojn` charts. Our transport adapter (not an exe setting);
/// without it the transfer engine stays idle and the real FTP client
/// remains OPEN (`docs/music-availability.md` §4). Set before boot so
/// [`OnlineSession::new`] can point the queued downloads at it.
const MUSIC_SOURCE_ENV: &str = "O2JAM_MUSIC_SOURCE";
/// 4011 `StartResult::Success` (Memoryer `StartGameEventData`).
const START_RESULT_SUCCESS: i32 = 0;

/// The result word a 4011 broadcast carries (`i32`, Memoryer
/// `StartGameEventData.Result`): `None` for a body too short to hold
/// one — the bare broadcast our rigs send, which reads as a start.
#[must_use]
fn start_broadcast_result(payload: &[u8]) -> Option<i32> {
    let bytes: [u8; 4] = payload.get(..4)?.try_into().ok()?;
    Some(i32::from_le_bytes(bytes))
}

/// Whether one wire id re-announces the room's music state: the exe's
/// 4022 (a member left the game, VAs `0x4ab658`/`0x4b0e12`), 4024 and
/// 4058 (the two skill changes, VAs `0x4ab90c`/`0x4b10fd`/`0x4b17dc`)
/// arms end with the same 4025 send (`docs/music-availability.md` §2).
#[must_use]
fn announces_room_music_state(frame_id: u16) -> bool {
    matches!(
        Event::from_id(frame_id),
        Some(Event::UserLeaveGame | Event::WaitingSkillChanged | Event::WaitingSkillExChanged)
    )
}

/// Sent-tick dedupe key: the wire moves only when play does (per-hit
/// shape — identical snapshots between judgements send nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TickSnapshot {
    points: u32,
    jam_level: u32,
    bonus: u64,
    life: u16,
}

/// Room Confirm join intent (`--online-join-room`): the room number
/// plus password the session exchanges on Room Confirm. The shell
/// owns the highlight walk but exposes no room-number query, so the
/// join target rides explicit CLI intent instead of the highlight.
#[derive(Debug, Clone)]
struct PendingRoomJoin {
    number: i32,
    password: String,
}

/// Pending interactive channel login (ticket #172): the 1004 send
/// plus the 1005 answer and the 2001 identity fetch resolve on the
/// per-frame poll, never on the Confirm/press path — a silent server
/// leaves the game painting instead of freezing the frame loop.
/// Queue-latest: a re-entry replaces the stale attempt outright.
#[derive(Debug, Clone, Copy)]
struct ChannelLoginAttempt {
    /// Wire-short login pair (validated at staging, like the old
    /// synchronous entry).
    gateway_id: i16,
    /// Wire-short login pair (validated at staging, like the old
    /// synchronous entry).
    channel_id: i16,
    /// Input the advance replays on success.
    origin: EntryEcho,
    /// `true` once the 1004 send went out (the poll retries it).
    login_sent: bool,
    /// `true` once the 2001 fetch went out (the poll retries it).
    identity_sent: bool,
    /// When the silent server stops getting the benefit of the doubt.
    deadline: Instant,
}

/// Pending item-shop buy attempt (ticket #194): the 5000 send plus
/// the 5001 answer resolve on the per-frame poll. A timeout clears
/// the attempt so the next BUYITEM retries it.
#[derive(Debug, Clone, Copy)]
struct ShopBuyAttempt {
    /// When the silent server stops getting the benefit of the doubt.
    deadline: Instant,
}

/// Pending song-shop buy attempt (ticket #202): the 5010 send plus
/// the 5011 answer resolve on the per-frame poll. A timeout clears
/// the attempt so the next buy retries it.
#[derive(Debug, Clone, Copy)]
struct MusicBuyAttempt {
    /// When the silent server stops getting the benefit of the doubt.
    deadline: Instant,
}

/// Pending gift-list fetch (ticket #201): the 5017 send plus the 5018
/// answer resolve on the per-frame poll. The 5018 body is exe-ignored
/// (`docs/gifts.md`); the drain only clears the attempt. A timeout
/// clears it so the next MyRoom entry retries it.
#[derive(Debug, Clone, Copy)]
struct GiftListAttempt {
    /// When the silent server stops getting the benefit of the doubt.
    deadline: Instant,
}

/// Pending gift-claim attempt (ticket #201): the 5023 send plus the
/// 5024 answer resolve on the per-frame poll. The reply carries only
/// a verdict, so the claimed gift id rides here for the success
/// fold into the staged bag. A timeout clears the attempt so the
/// next MOVE retries it.
#[derive(Debug, Clone, Copy)]
struct GiftClaimAttempt {
    /// Claimed gift id for the 5024 success fold.
    gift_id: i32,
    /// When the silent server stops getting the benefit of the doubt.
    deadline: Instant,
}

/// Pending Room Confirm join attempt (ticket #172): the 3002 send
/// plus the 3003 answer resolve on the per-frame poll, never on the
/// Confirm/press path. A timeout clears the attempt but keeps the
/// [`PendingRoomJoin`] intent, so the next Confirm retries it — the
/// old synchronous retry shape. Queue-latest like the login.
#[derive(Debug, Clone)]
struct RoomJoinAttempt {
    /// Room number being joined.
    number: i32,
    /// Room password (empty for open rows).
    password: String,
    /// Input the advance replays on success.
    origin: EntryEcho,
    /// `true` once the 3002 send went out (the poll retries it).
    request_sent: bool,
    /// When the silent server stops getting the benefit of the doubt.
    deadline: Instant,
}

/// Pending master Waiting select attempt (ticket #172): the
/// 4000/4002 sends plus the 4003 echo and the 4030 send plus the 4031
/// answer resolve on the per-frame poll, never on the Confirm path.
/// A timeout clears the attempt but keeps the [`SongChoice`] pick,
/// so the next Confirm retries it — the old synchronous retry shape.
#[derive(Debug, Clone, Copy)]
struct SongSelectAttempt {
    /// Host pick being staged.
    choice: SongChoice,
    /// Which send the poll still owes (each leg sends once; a failed
    /// send retries the same leg next paint, never the earlier ones).
    stage: SelectStage,
    /// When the silent server stops getting the benefit of the doubt.
    deadline: Instant,
}

/// Song-select send stage: the poll sends each leg once, so a quiet
/// server sees exactly one 4000/4002 pair and one 4030 — the old
/// synchronous send-once shape, spread across paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectStage {
    /// 4000/4002 unsent: send both, then await the 4003 echo.
    SendSelect,
    /// 4000/4002 sent: await the 4003 echo.
    AwaitEcho,
    /// Echo matched but the 4030 send failed: resend 4030 only,
    /// then await the 4031 answer.
    SendList,
    /// 4030 sent: await the 4031 answer.
    AwaitList,
}

/// Host song pick (`--online-music-id` plus difficulty/speed/arena):
/// the song id plus difficulty and speed ride `SetRoomMusic` 4000,
/// the stage word rides `SetRoomArena` 4002 — the headless
/// [`crate::connect::run_song_flow`] select shapes. The shell owns
/// the dialog walk but exposes no pick query, so the choice rides
/// explicit CLI intent like the room join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongChoice {
    /// Song id (`SetRoomMusic` music id).
    pub music_id: u16,
    /// Chart difficulty code (`Difficulty::from_code`).
    pub difficulty: u8,
    /// Scroll-speed code (`GameSpeed` ordinal).
    pub speed: u8,
    /// Room stage word (`SetRoomArena` arena).
    pub arena_id: i32,
}

/// Owned catalog row behind the chart fetch: the container token
/// names the `Music/o2ma###.ojn` chart (the shell cover path's token
/// swap); the title and artist feed the loading caption.
#[derive(Debug, Clone, PartialEq)]
struct CatalogRow {
    song_id: u32,
    ojm_file: String,
    title: String,
    artist: String,
}

/// Waiting-room session: shell game plus its gateway transport.
///
/// Single-threaded by construction (the blocking gateway and the game
/// both live here, never shared), so no lock ordering applies.
///
/// The stats mirror (`peers`, `peer_queue`, `peer_ranks`) is session-owned:
/// 4015 echoes fold in through [`apply_peer_update`] every `Playing` paint
/// and forward into the shell through [`ScreenGame::apply_peer_echo`], so
/// the Playing paint shows the live gauges and result rows.
pub struct OnlineSession {
    game: ScreenGame,
    gateway: GatewayConnection,
    peers: PeerBars,
    peer_queue: Option<QueueEvent>,
    peer_ranks: Vec<RankDelivery>,
    stats_sequence: i32,
    last_tick: Option<TickSnapshot>,
    pending_room: Option<PendingRoomJoin>,
    pending_song: Option<SongChoice>,
    /// Live channel login (`None` while no Confirm/press entry is in
    /// flight; the per-frame [`Self::poll_channel_login`] owns it).
    login_attempt: Option<ChannelLoginAttempt>,
    /// Channel the session currently occupies (set on a resolved login or
    /// boot auto-login, cleared on leave or timeout). A swallowed face
    /// press naming this channel skips the wire round trip.
    resident_channel: Option<ChannelState>,
    /// Matching release of the one resident echo currently outstanding
    /// (channel face or room row); no second gate owns this pair.
    resident_echo: Option<press::PressEcho>,
    /// Title of the room the session currently occupies (set on a
    /// resolved join or boot create-and-join, cleared on leave or
    /// timeout). A swallowed row press naming this room replays
    /// locally instead of re-sending a join the server ignores
    /// (duplicate join of an occupied room; ticket #231).
    resident_room: Option<String>,
    /// Live room join (`None` while no Confirm/press join is in
    /// flight; the per-frame [`Self::poll_room_join`] owns it).
    join_attempt: Option<RoomJoinAttempt>,
    /// Live song select (`None` while no master Confirm select is in
    /// flight; the per-frame [`Self::poll_song_select`] owns it).
    select_attempt: Option<SongSelectAttempt>,
    /// Live item-shop buy (`None` while no BUYITEM commit is in
    /// flight; the per-frame [`Self::poll_shop_buy`] owns it).
    shop_buy_attempt: Option<ShopBuyAttempt>,
    /// Live song-shop buy (`None` while no buy commit is in
    /// flight; the per-frame [`Self::poll_music_buy`] owns it).
    music_buy_attempt: Option<MusicBuyAttempt>,
    /// Live gift-list fetch (`None` while no MyRoom entry fetch is in
    /// flight; the per-frame [`Self::poll_gift_list`] owns it).
    gift_list_attempt: Option<GiftListAttempt>,
    /// Live gift claim (`None` while no MOVE claim is in flight; the
    /// per-frame [`Self::poll_gift_claim`] owns it).
    gift_claim_attempt: Option<GiftClaimAttempt>,
    /// Container tokens plus captions behind the chart fetch
    /// (`OJNList.dat` order, owned copies; see [`Self::set_song_catalog`]).
    catalog: Vec<CatalogRow>,
    /// Last staged host pick (`None` until a select exchange stages
    /// one; the shell exposes no selection read-back).
    last_selection: Option<SetRoomMusic>,
    local_user: String,
    /// `true` once a Room Confirm join staged a snapshot: Waiting
    /// Confirm then means START/READY instead of the offline walk.
    joined_waiting: bool,
    /// Joined snapshot mirrored from the 3003 answer: the speed
    /// behind the 4012/4016 sends plus the seats behind the battle
    /// mirror (`None` until a join stages one).
    joined_room: Option<JoinedRoom>,
    /// Guest ready toggle behind `Ready` 4008 (flips per Confirm).
    guest_ready: bool,
    /// `true` once the 4011 start broadcast arrived: the next
    /// draft-free Waiting paint advances the shell to Loading.
    start_announced: bool,
    /// `true` once this Loading visit sent 4012 (cleared on
    /// leaving Loading, so the next song confirms again).
    loaded_sent: bool,
    /// `true` once this Result visit sent the tally triple
    /// (cleared on leaving Result, so the next song tallies again).
    tally_sent: bool,
    /// Silent-audio rig switch: `true` keeps every attach on the null
    /// sink even when hardware exists. A session starts fenced
    /// (`new`) — only the windowed live boot unfences it through
    /// [`Self::set_silent_audio`] with the run's `--no-sound` value, so
    /// no test or rig arm reaches the speakers by omission (#283).
    silent_audio: bool,
    /// Last slot action per seat, from 3009 updates (`None` while the
    /// seat never changed). The shell exposes no per-seat write yet,
    /// so this is the bridge query its paint follow-up will consume.
    slot_actions: [Option<RoomSlotAction>; 8],
    /// Last ready flag per seat, from 4009 echoes (`None` while the
    /// seat never echoed) — same bridge shape as `slot_actions`.
    ready_echoes: [Option<bool>; 8],
    /// Result record words per seat, from 4018 rows (`None` while the
    /// seat never sent an active row). Mirrors the exe's member record
    /// at the offsets pinned in `docs/net-protocol.md` §4c.
    result_records: [Option<result_records::ResultMemberRecord>; 8],
}

impl OnlineSession {
    /// Pair a game with its gateway transport.
    #[must_use]
    pub fn new(game: ScreenGame, gateway: GatewayConnection) -> Self {
        let mut game = game;
        configure_music_source(&mut game, std::env::var_os(MUSIC_SOURCE_ENV));
        Self {
            game,
            gateway,
            peers: PeerBars::new(),
            peer_queue: None,
            peer_ranks: Vec::new(),
            stats_sequence: 0,
            last_tick: None,
            pending_room: None,
            pending_song: None,
            login_attempt: None,
            resident_channel: None,
            resident_echo: None,
            resident_room: None,
            join_attempt: None,
            select_attempt: None,
            shop_buy_attempt: None,
            music_buy_attempt: None,
            gift_list_attempt: None,
            gift_claim_attempt: None,
            catalog: Vec::new(),
            last_selection: None,
            local_user: String::new(),
            joined_waiting: false,
            joined_room: None,
            guest_ready: false,
            start_announced: false,
            loaded_sent: false,
            tally_sent: false,
            silent_audio: true,
            slot_actions: [None; 8],
            ready_echoes: [None; 8],
            result_records: [None; 8],
        }
    }

    /// Remember the Room Confirm join target: the `--online-join-room`
    /// number plus password, attempted on the next Room Confirm
    /// ([`Self::begin_room_join`], resolved by [`Self::poll_room_join`]).
    pub fn set_pending_room_join(&mut self, number: i32, password: &str) -> &mut Self {
        self.pending_room = Some(PendingRoomJoin {
            number,
            password: password.to_owned(),
        });
        self
    }

    /// Remember the local username (`--token`): the addressee inbound
    /// 2019 whisper rows name, since the wire event carries only the
    /// sender.
    pub fn set_local_user(&mut self, name: &str) -> &mut Self {
        self.local_user = name.to_owned();
        self
    }

    /// Remember the host pick (`--online-music-id` plus
    /// difficulty/speed/arena): the next master Waiting Confirm
    /// queues it ([`Self::begin_song_select`], resolved by
    /// [`Self::poll_song_select`]).
    pub fn set_pending_song(&mut self, choice: SongChoice) -> &mut Self {
        self.pending_song = Some(choice);
        self
    }

    /// Paint the pending host pick before its 4000/4002/4030 exchange
    /// resolves. A local Waiting Confirm (no joined room) advances without
    /// the 4031 answer, so this is the only selection the staged catalog
    /// can resolve an album cover from on that walk. Sends no packet; the
    /// answer restages the same pick through [`Self::resolve_song_select`].
    pub fn stage_pending_song_selection(&mut self) -> &mut Self {
        if let Some(choice) = self.pending_song {
            self.game.set_selected_music(SetRoomMusic {
                music_id: choice.music_id,
                difficulty: choice.difficulty,
                speed: choice.speed,
            });
        }
        self
    }

    /// Stage the install's local music availability (`Music` folder
    /// scan, `docs/music-availability.md` §1) into the select-music
    /// dialog's rows. Order-independent with
    /// [`Self::set_song_catalog`]: either staging point takes the
    /// other's answer. `None` (no game data) leaves every row
    /// selectable.
    pub fn set_installed_music(&mut self, installed: Option<InstalledMusic>) -> &mut Self {
        self.game.set_installed_music(installed);
        self
    }

    /// Stage the song catalog (`OJNList.dat` order) into the
    /// select-music dialog and remember its container tokens for the
    /// chart fetch. Dead slots stay out, exactly like the shell
    /// staging — a missing catalog paints dialog art without rows,
    /// and the chart fetch then warns instead of inventing a path.
    pub fn set_song_catalog(&mut self, entries: &[OjnListEntry<'_>]) -> &mut Self {
        self.set_song_catalog_with_labels(entries, &[])
    }

    /// Stage the song catalog with its `OJNList.dat` table-5 label
    /// buckets (the song dialog's label column input, ticket #273);
    /// every other board stages exactly as [`Self::set_song_catalog`]
    /// does.
    pub fn set_song_catalog_with_labels(
        &mut self,
        entries: &[OjnListEntry<'_>],
        labels: &[o2jam_file::OjnMidTagEntry],
    ) -> &mut Self {
        self.game.set_song_catalog_with_labels(entries, labels);
        self.catalog = entries
            .iter()
            .filter(|entry| entry.is_song())
            .map(catalog_row)
            .collect();
        self
    }

    /// Send one room-setup request and wait for its answer past
    /// foreign broadcasts (the headless `await_lenient` shape):
    /// keepalives stay silent, every other skipped id traces,
    /// never dropped quietly.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports.
    fn exchange_room_frame(
        &mut self,
        request_id: u16,
        body: &[u8],
        expected: u16,
    ) -> anyhow::Result<Frame> {
        self.gateway
            .send_frame(request_id, body)
            .map_err(net_context)?;
        loop {
            let frame = self.gateway.next_frame().map_err(net_context)?;
            if frame.id == expected {
                return Ok(frame);
            }
            if Generic::from_id(frame.id).is_none() {
                tracing::debug!(id = frame.id, "skipping foreign frame during room setup");
            }
        }
    }

    /// Create a room on the live gateway and stage its snapshot
    /// (the host path): `CreateRoom` 2004 names the number, then the
    /// `GetChannelInfo` room list sources the row the creator already
    /// occupies — seat 0, no peers — staged through
    /// [`Self::stage_hosted_room`]. No `JoinWaiting`: re-joining our
    /// own room breaks the server master check (`SetRoomMusic` 4000
    /// then rejects us). The arena word stays 0 until the select
    /// exchange proves one ([`Self::note_arena_selected`]). Foreign
    /// broadcasts skip past like the per-frame drains, never failing
    /// the exchange.
    ///
    /// # Errors
    ///
    /// A rejected create, a missing room row, or whatever the
    /// gateway transport reports.
    pub fn create_and_join_room(&mut self, title: &str) -> anyhow::Result<JoinedRoom> {
        let create = CreateRoom {
            title: title.to_owned(),
            key_mode: KeyMode::FiveKeys,
            game_mode: GameMode::Normal,
            has_password: false,
            password: String::new(),
            min_level: 1,
            max_level: 99,
            premium: false,
        };
        let created = self.exchange_room_frame(
            Request::CreateRoom.id(),
            &create.encode_body(),
            CreateRoomResponse::COMMAND,
        )?;
        let created = CreateRoomResponse::decode_frame(&created).map_err(net_context)?;
        if created.result != CreateResult::Success {
            anyhow::bail!("room create rejected: {:?}", created.result);
        }
        let number = created
            .info
            .as_ref()
            .map(|info| info.number)
            .with_context(|| "room create answered without a snapshot")?;
        let listed =
            self.exchange_room_frame(Request::GetChannelInfo.id(), &[], RoomList::COMMAND)?;
        let listed = RoomList::decode_frame(&listed).map_err(net_context)?;
        let entry = listed
            .rooms
            .iter()
            .find(|room| room.number == number)
            .with_context(|| format!("created room {number} missing from the room list"))?;
        let room = JoinedRoom {
            member_id: 0,
            team: 0,
            title: entry.title.clone(),
            music_id: entry.music_id,
            arena: 0,
            key_mode: entry.key_mode,
            game_mode: entry.game_mode,
            difficulty: entry.difficulty,
            speed: entry.speed,
            has_super_manager: false,
            user_count: i32::from(entry.user_count),
            slots: Vec::new(),
            skills: Vec::new(),
            premium: entry.premium,
            champion: None,
            champion_streak: None,
        };
        self.stage_hosted_room(room.clone());
        // Refresh the list too: the login-time snapshot predates the
        // create, and empty slots filter out downstream, so the Room
        // screen shows exactly the numbered rows.
        tracing::info!(number, "room created with number");
        self.game.set_rooms(listed.rooms.clone());
        Ok(room)
    }

    /// Stage genuine 7.55 scene art under the live walk (the shots
    /// `stage_real_art` pattern through [`crate::live_art`]): bound art
    /// plus figures for the lobby and playing scenes, driven by the live
    /// room state. Missing archives keep the fallback paint. Call after
    /// the login lists land; later joins restage Playing on their own.
    pub fn stage_live_art(&mut self) {
        crate::live_art::stage_live_scenes(&mut self.game);
    }

    /// Record the echo-verified stage word into the joined snapshot:
    /// the 4003 answer already proved it took, so both mirrors (the
    /// session tally source and the shell venue reader) move together.
    pub fn note_arena_selected(&mut self, arena: i32) -> &mut Self {
        if let Some(room) = self.joined_room.as_mut() {
            room.arena = arena;
            self.game.set_joined_room(room.clone());
            restage_live_playing(&mut self.game);
        }
        self
    }

    /// Stage a hosted room snapshot (the `CreateRoom` path): the
    /// join exchange is unnecessary for the host, so this marks the
    /// session joined directly and mirrors the waiting-room shape
    /// `join_pending_room` stages for guests.
    pub fn stage_hosted_room(&mut self, room: JoinedRoom) -> &mut Self {
        self.resident_room = Some(room.title.clone());
        self.joined_room = Some(room.clone());
        self.game.set_joined_room(room);
        self.joined_waiting = true;
        restage_live_playing(&mut self.game);
        self
    }

    /// Mutable game behind the session: the walk stages live lists
    /// and paints deterministic `paint_at` instants through it.
    pub fn game_mut(&mut self) -> &mut ScreenGame {
        &mut self.game
    }

    /// Mark the channel the session currently occupies (boot
    /// auto-login or a resolved login), so a face press on it skips
    /// the wire round trip.
    pub fn set_resident_channel(&mut self, channel: ChannelState) {
        self.resident_channel = Some(channel);
    }

    /// Clear channel residency (leave or timeout).
    pub fn clear_resident_channel(&mut self) {
        self.resident_channel = None;
    }

    /// Current shell screen behind the session.
    #[must_use]
    pub fn screen(&self) -> Screen {
        self.game.screen()
    }

    /// Last staged host pick (`None` until a select exchange stages
    /// one). No production reader yet: the Loading chart fetch stages
    /// from it.
    #[allow(
        dead_code,
        reason = "bridge query for the loading follow-up; tested below"
    )]
    #[must_use]
    pub fn selected_song(&self) -> Option<SetRoomMusic> {
        self.last_selection
    }

    /// Stage a host song select attempt over a joined room (ticket
    /// #172): no socket IO — the Confirm path only queues the pick,
    /// and the per-frame [`Self::poll_song_select`] moves the legs
    /// (`SetRoomMusic` 4000, `SetRoomArena` 4002 with the 4003 echo
    /// assertion, then `GetMusicList` 4030 with the 4031 answer — the
    /// headless [`crate::connect::run_song_flow`] select shapes), so
    /// a silent server never freezes the frame loop. Queue-latest: a
    /// repeated Confirm replaces the stale attempt outright.
    pub(crate) fn begin_song_select(&mut self, choice: SongChoice) {
        self.select_attempt = Some(SongSelectAttempt {
            choice,
            stage: SelectStage::SendSelect,
            deadline: Instant::now() + PENDING_WIRE_WINDOW,
        });
    }

    /// Per-frame song-select section in `Waiting`: move the pending
    /// attempt one leg under the short pending drain, ahead of the
    /// waiting-chat drain (which would otherwise eat the 4003/4031
    /// answers as foreign frames). Off Waiting the attempt drops —
    /// the pick stays, so the next master Confirm retries it — and
    /// past the deadline it warns and clears the same way.
    fn poll_song_select(&mut self) {
        if self.game.screen() != Screen::Waiting {
            if self.select_attempt.take().is_some() {
                tracing::debug!("song select left behind leaving Waiting; dropping it");
            }
            return;
        }
        let Some(attempt) = self.select_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            tracing::warn!("song select timed out with no server answer; pick kept");
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "song select", |session| {
            if let Some(kept) = session.drain_song_select(attempt) {
                session.select_attempt = Some(kept);
            }
        });
    }

    /// Move one song-select attempt leg: send what the stage still
    /// owes, then drain until the socket goes quiet. Returns the
    /// attempt while it still awaits an answer (`None` once it
    /// resolved or failed — the pick stays staged for the next retry
    /// either way except on success, which consumes it).
    fn drain_song_select(&mut self, mut attempt: SongSelectAttempt) -> Option<SongSelectAttempt> {
        match attempt.stage {
            SelectStage::SendSelect => {
                let select = SetRoomMusic {
                    music_id: attempt.choice.music_id,
                    difficulty: attempt.choice.difficulty,
                    speed: attempt.choice.speed,
                };
                if let Err(source) = self
                    .gateway
                    .send_frame(Request::SetRoomMusic.id(), &select.encode_body())
                {
                    tracing::debug!(%source, "song select send failed; retrying next paint");
                    return Some(attempt);
                }
                let stage = SetRoomArena::new(attempt.choice.arena_id);
                if let Err(source) = self
                    .gateway
                    .send_frame(Request::SetRoomArena.id(), &stage.encode_body())
                {
                    tracing::debug!(%source, "song arena send failed; retrying next paint");
                    return Some(attempt);
                }
                attempt.stage = SelectStage::AwaitEcho;
            }
            SelectStage::SendList => {
                if let Err(source) = self
                    .gateway
                    .send_frame(Request::GetMusicList.id(), &GetMusicList.encode_body())
                {
                    tracing::debug!(%source, "music list send failed; retrying next paint");
                    return Some(attempt);
                }
                attempt.stage = SelectStage::AwaitList;
            }
            SelectStage::AwaitEcho | SelectStage::AwaitList => {}
        }
        loop {
            match self.gateway.next_frame() {
                Ok(frame)
                    if frame.id == RoomArenaEcho::COMMAND
                        && attempt.stage == SelectStage::AwaitEcho =>
                {
                    let Ok(echo) = RoomArenaEcho::decode_frame(&frame) else {
                        tracing::debug!(id = frame.id, "dropping undecodable arena echo");
                        return None;
                    };
                    if echo.arena != attempt.choice.arena_id {
                        tracing::warn!(
                            sent = attempt.choice.arena_id,
                            got = echo.arena,
                            "arena echo mismatch; pick kept"
                        );
                        return None;
                    }
                    if let Err(source) = self
                        .gateway
                        .send_frame(Request::GetMusicList.id(), &GetMusicList.encode_body())
                    {
                        tracing::debug!(%source, "music list send failed; retrying next paint");
                        attempt.stage = SelectStage::SendList;
                        return Some(attempt);
                    }
                    attempt.stage = SelectStage::AwaitList;
                }
                Ok(frame)
                    if frame.id == MusicListAnswer::COMMAND
                        && attempt.stage == SelectStage::AwaitList =>
                {
                    let Ok(answer) = MusicListAnswer::decode_frame(&frame) else {
                        tracing::debug!(id = frame.id, "dropping undecodable music list");
                        return None;
                    };
                    self.resolve_song_select(answer, attempt.choice);
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-select frame in select drain");
                }
                Err(_) => break,
            }
        }
        Some(attempt)
    }

    /// Stage a completed song select: the pick drives the dialog rows
    /// and the waiting `MUSICNAME` caption through
    /// [`ScreenGame::set_selected_music`], then the chart fetch stages
    /// the highway from the catalog container — the old synchronous
    /// Confirm shape, deferred to the poll. Consumes the pending pick,
    /// so the next master Confirm means START instead of another
    /// select; a failed chart fetch only warns past the staged pick.
    fn resolve_song_select(&mut self, answer: MusicListAnswer, choice: SongChoice) {
        let select = SetRoomMusic {
            music_id: choice.music_id,
            difficulty: choice.difficulty,
            speed: choice.speed,
        };
        self.game.set_selected_music(select);
        self.last_selection = Some(select);
        self.pending_song = None;
        tracing::info!(
            music_id = choice.music_id,
            entries = answer.entries.len(),
            free_music = answer.free_music,
            "song staged"
        );
        if let Err(source) =
            self.fetch_and_stage_chart(choice.music_id, level_for_difficulty(choice.difficulty))
        {
            tracing::warn!(%source, "chart fetch failed; pick stays staged");
        }
    }

    /// Run the loading gate over one chart buffer and stage it into
    /// the highway: OJN open plus level walk ([`prepare_chart`]),
    /// onset build, [`ScreenGame::load_chart`], the walked cover into
    /// the Loading art (a miss keeps the planet fallback), and the
    /// clock-armed loading caption when the staged catalog names the
    /// song (indeterminate otherwise — the shots `stage_loading_status`
    /// honesty rule).
    ///
    /// # Errors
    ///
    /// `FileError` on bad magic, truncation, or an out-of-range level.
    pub fn stage_chart_bytes(
        &mut self,
        chart_bytes: &[u8],
        level: usize,
    ) -> Result<PreparedChart, o2jam_file::FileError> {
        let prepared = prepare_chart(chart_bytes, level)?;
        let events = o2jam_file::parse_ojn_level_events(chart_bytes, level)?;
        let chart = o2jam_game::build_level_chart(prepared.bpm, &events);
        self.game.load_chart(chart.hit_onsets_ms());
        stage_loading_cover(&mut self.game, chart_bytes);
        self.stage_loading_caption(prepared.song_id);
        Ok(prepared)
    }

    /// Fetch the staged catalog's chart for one song id from the
    /// install `Music` folder and stage it into the highway
    /// ([`Self::stage_chart_bytes`]), then attach its key-sound bank
    /// from the catalog container token through the loading gate
    /// ([`Self::attach_bank_for_song`]). The bank leg is best-effort:
    /// a miss stages silent, never failing the chart.
    ///
    /// # Errors
    ///
    /// A song the staged catalog does not name, unreadable chart
    /// bytes, or a chart the loading gate rejects.
    pub fn fetch_and_stage_chart(
        &mut self,
        music_id: u16,
        level: usize,
    ) -> anyhow::Result<PreparedChart> {
        let chart_path = self
            .chart_path_for_song(music_id)
            .with_context(|| format!("no catalog chart file for song {music_id}"))?;
        let chart_bytes = std::fs::read(&chart_path)
            .with_context(|| format!("reading chart {}", chart_path.display()))?;
        let prepared = self
            .stage_chart_bytes(&chart_bytes, level)
            .with_context(|| {
                format!(
                    "loading gate rejected {} level {level}",
                    chart_path.display()
                )
            })?;
        self.attach_bank_for_song(music_id, prepared.format_version);
        Ok(prepared)
    }

    /// Install chart path for one staged song id (`None` while the
    /// catalog names no such song or its token carries no `.ojm`
    /// suffix — never an invented path).
    fn chart_path_for_song(&self, music_id: u16) -> Option<PathBuf> {
        let wanted = u32::from(music_id);
        let row = self.catalog.iter().find(|row| row.song_id == wanted)?;
        install_music_path(&row.ojm_file, "ojn")
    }

    /// Install bank path for one staged song id (same `None` rule as
    /// [`Self::chart_path_for_song`]).
    fn bank_path_for_song(&self, music_id: u16) -> Option<PathBuf> {
        let wanted = u32::from(music_id);
        let row = self.catalog.iter().find(|row| row.song_id == wanted)?;
        install_music_path(&row.ojm_file, "ojm")
    }

    /// Decode the installed bank for one staged song id through the
    /// loading gate ([`load_sound_bank`]): the catalog container token
    /// names the `Music/o2ma###.ojm` image. Misses (no catalog row, no
    /// `.ojm` suffix, unreadable bytes, undecodable image) decode to
    /// an empty bank — silent, never invented.
    pub fn bank_for_song(
        &self,
        music_id: u16,
        format_version: f32,
    ) -> std::collections::HashMap<u32, o2jam_audio::PcmSamples> {
        let Some(bank_path) = self.bank_path_for_song(music_id) else {
            return std::collections::HashMap::new();
        };
        let Ok(bank_bytes) = std::fs::read(&bank_path) else {
            return std::collections::HashMap::new();
        };
        load_sound_bank(&bank_bytes, format_version)
    }

    /// Attach one song's key-sound bank to the game backend so struck
    /// notes emit their samples: the sink comes from [`Self::audio_sink`],
    /// so a fenced session stays on the null sink even on a machine with
    /// hardware. Best-effort: an empty bank keeps the silent play shape,
    /// never failing the staged chart.
    pub fn attach_bank_for_song(&mut self, music_id: u16, format_version: f32) {
        let bank = self.bank_for_song(music_id, format_version);
        if bank.is_empty() {
            return;
        }
        let entries = bank.len();
        let backend = self.audio_sink(true).open();
        self.game.with_audio(backend, bank);
        // Scene BGM stages with the key bank: boot already cued
        // Avi in the windowed setup, and the song attach re-cues
        // the current screen through the same table.
        self.game.stage_scene_bgm(crate::load_scene_bgm_bank());
        tracing::info!(entries, "key sounds attached");
    }

    /// Set the silent-audio rig switch: `true` keeps every bank
    /// attach on the null backend even when audio hardware exists
    /// (automated rigs must never reach the speakers).
    pub fn set_silent_audio(&mut self, silent: bool) -> &mut Self {
        self.silent_audio = silent;
        self
    }

    /// The sink an attach would pick for this session: the live request
    /// meets the rig fence in [`o2jam_audio::SinkChoice::for_session`],
    /// the one decision in the workspace. Pure — a guard can read the
    /// choice without opening a device.
    #[must_use]
    pub fn audio_sink(&self, live_player: bool) -> o2jam_audio::SinkChoice {
        o2jam_audio::SinkChoice::for_session(live_player, self.silent_audio)
    }

    /// Attach the launch audio rig (a fresh backend plus the scene-BGM
    /// bank) so the loading gate sees a rig before any song bank
    /// attaches and the boot BGM cue sounds: the sink comes from
    /// [`Self::audio_sink`], so a fenced session plays the cue on the
    /// null sink (audio containment).
    pub fn attach_initial_audio(&mut self, live: bool) {
        let backend = self.audio_sink(live).open();
        self.game
            .with_audio(backend, std::collections::HashMap::new());
        self.game.stage_scene_bgm(crate::load_scene_bgm_bank());
    }

    /// Stage the clock-armed loading caption for one chart song id
    /// from the staged catalog (`None` match keeps the indeterminate
    /// bar — never an invented caption).
    fn stage_loading_caption(&mut self, song_id: u32) {
        let caption = self
            .game
            .select_songs()
            .iter()
            .find(|song| song.song_id == song_id)
            .map(|song| (song.title.clone(), song.artist.clone()));
        let Some((title, artist)) = caption else {
            return;
        };
        if let Some(status) = loading_phase_status(LOADING_GATE_STEPS, &title, &artist) {
            self.game.set_loading_status(status);
        }
    }

    /// Peer mirror fed by 4015 echoes (empty until the tracker speaks).
    /// No production reader yet: shell paint wiring follows separately.
    #[allow(
        dead_code,
        reason = "bridge query for the shell paint follow-up; tested below"
    )]
    #[must_use]
    pub fn peers(&self) -> &PeerBars {
        &self.peers
    }

    /// Queue forward from the last applied echo (`None` while quiet).
    /// No production reader yet: shell paint wiring follows separately.
    #[allow(
        dead_code,
        reason = "bridge query for the shell paint follow-up; tested below"
    )]
    #[must_use]
    pub fn peer_queue(&self) -> Option<QueueEvent> {
        self.peer_queue
    }

    /// Rank deliveries from the last applied echo (empty while quiet).
    /// No production reader yet: shell paint wiring follows separately.
    #[allow(
        dead_code,
        reason = "bridge query for the shell paint follow-up; tested below"
    )]
    #[must_use]
    pub fn peer_ranks(&self) -> &[RankDelivery] {
        &self.peer_ranks
    }

    /// Last slot action for a seat, from 3009 updates (`None` while
    /// the seat never changed). No production reader yet: shell paint
    /// wiring follows separately.
    #[allow(
        dead_code,
        reason = "bridge query for the shell paint follow-up; tested below"
    )]
    #[must_use]
    pub fn slot_action(&self, index: u8) -> Option<RoomSlotAction> {
        self.slot_actions.get(usize::from(index)).copied().flatten()
    }

    /// Last ready flag for a seat, from 4009 echoes (`None` while the
    /// seat never echoed). No production reader yet: shell paint
    /// wiring follows separately.
    #[allow(
        dead_code,
        reason = "bridge query for the shell paint follow-up; tested below"
    )]
    #[must_use]
    pub fn member_ready(&self, member_id: u8) -> Option<bool> {
        self.ready_echoes
            .get(usize::from(member_id))
            .copied()
            .flatten()
    }

    /// Result record words for a seat, from the last 4018 frame
    /// (`None` while no active row named that seat). No production
    /// reader yet: the shell paint wiring follows separately.
    #[allow(
        dead_code,
        reason = "bridge query for the shell paint follow-up; tested below"
    )]
    #[must_use]
    pub fn result_record(&self, member_id: u8) -> Option<&result_records::ResultMemberRecord> {
        self.result_records.get(usize::from(member_id))?.as_ref()
    }

    /// Transmit the pending chat draft as 3011.
    ///
    /// `Ok(true)` sent one packet; `Ok(false)` means the draft was
    /// empty and the socket stayed untouched. A failed send keeps the
    /// draft, so the next send retries it.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02: `Result`,
    /// propagated with `?`, never unwrapped per ERRORS-06).
    pub fn flush_chat(&mut self) -> Result<bool, o2jam_net::Error> {
        let Some(draft) = self.game.chat_send_message() else {
            return Ok(false);
        };
        self.gateway
            .send_frame(SendWaitingMessage::COMMAND, &draft.encode_body())?;
        self.game.clear_chat_input();
        Ok(true)
    }

    /// Transmit the pending Channel draft as 2012.
    ///
    /// `Ok(true)` sent one packet; `Ok(false)` means the draft was
    /// empty and the socket stayed untouched. A failed send keeps the
    /// draft, so the next send retries it (same shape as [`flush_chat`]).
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02: `Result`,
    /// propagated with `?`, never unwrapped per ERRORS-06).
    #[allow(
        dead_code,
        reason = "direct send entry the Channel guards ride; the menu-key path routes through flush_channel_action"
    )]
    pub fn flush_channel(&mut self) -> Result<bool, o2jam_net::Error> {
        self.flush_lobby_chat(LobbyChat::Channel)
    }

    /// Transmit one lobby chat draft as 2012.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (see [`Self::flush_chat`]).
    fn flush_lobby_chat(&mut self, chat: LobbyChat) -> Result<bool, o2jam_net::Error> {
        let Some(draft) = self.game.lobby_send_message(chat) else {
            return Ok(false);
        };
        self.gateway
            .send_frame(SendMainRoomMessage::COMMAND, &draft.encode_body())?;
        self.game.clear_lobby_input(chat);
        Ok(true)
    }

    /// Transmit the pending Channel draft as a 2017 whisper to
    /// `recipient`.
    ///
    /// `Ok(true)` sent one packet; `Ok(false)` means the draft or the
    /// recipient was empty and the socket stayed untouched. A failed
    /// send keeps the draft, so the next send retries it. A sent
    /// whisper remembers its recipient for `/r`.
    ///
    /// The slash Confirm ([`Self::flush_channel_action`]) routes
    /// through here with the parsed text staged as the draft; the
    /// whisper-target picker calls it directly.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02: `Result`,
    /// propagated with `?`, never unwrapped per ERRORS-06).
    #[allow(
        dead_code,
        reason = "direct whisper entry the Channel guards ride; the menu-key path routes through flush_channel_action"
    )]
    pub fn flush_channel_whisper(&mut self, recipient: &str) -> Result<bool, o2jam_net::Error> {
        self.flush_lobby_whisper(LobbyChat::Channel, recipient)
    }

    /// Transmit one lobby chat draft as a 2017 whisper to `recipient`.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (see [`Self::flush_chat`]).
    fn flush_lobby_whisper(
        &mut self,
        chat: LobbyChat,
        recipient: &str,
    ) -> Result<bool, o2jam_net::Error> {
        let Some(draft) = self.game.lobby_whisper_message(chat, recipient) else {
            return Ok(false);
        };
        self.gateway
            .send_frame(SendWhisper::COMMAND, &draft.encode_body())?;
        self.game.clear_lobby_input(chat);
        self.game.remember_whisper_peer(recipient);
        Ok(true)
    }

    /// Transmit the pending waiting draft as a 2017 whisper to
    /// `recipient` (the waiting-room 2017 send, files `0xb8ffc`/
    /// `0xb9369`).
    ///
    /// `Ok(true)` sent one packet; `Ok(false)` means the draft or the
    /// recipient was empty and the socket stayed untouched. A failed
    /// send keeps the draft, so the next send retries it. A sent
    /// whisper remembers its recipient for `/r`.
    ///
    /// The slash Confirm ([`Self::flush_chat_action`]) routes through
    /// here with the parsed text staged as the draft.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02: `Result`,
    /// propagated with `?`, never unwrapped per ERRORS-06).
    pub fn flush_chat_whisper(&mut self, recipient: &str) -> Result<bool, o2jam_net::Error> {
        let Some(draft) = self.game.chat_whisper_message(recipient) else {
            return Ok(false);
        };
        self.gateway
            .send_frame(SendWhisper::COMMAND, &draft.encode_body())?;
        self.game.clear_chat_input();
        self.game.remember_whisper_peer(recipient);
        Ok(true)
    }

    /// Transmit the pending Channel draft through the slash grammar:
    /// `/w <name> <text>` and `/r <text>` send 2017 (the parsed
    /// recipient routes through [`Self::flush_channel_whisper`]),
    /// `/h` and `/help` consume the draft with no packet, and anything
    /// else sends 2012 as plain chat.
    ///
    /// `Ok(true)` consumed the draft (sent or help); `Ok(false)`
    /// means the draft was empty and the socket stayed untouched. A
    /// failed send keeps the draft, so the next send retries it.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02: `Result`,
    /// propagated with `?`, never unwrapped per ERRORS-06).
    pub fn flush_channel_action(&mut self) -> Result<bool, o2jam_net::Error> {
        self.flush_lobby_action(LobbyChat::Channel)
    }

    /// Transmit the pending Room-lobby draft through the slash grammar:
    /// `/w <name> <text>` and `/r <text>` send 2017, `/h` and `/help`
    /// consume the draft with no packet, anything else sends 2012 as
    /// plain chat (ticket #37).
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (see [`Self::flush_chat`]).
    pub fn flush_room_chat_action(&mut self) -> Result<bool, o2jam_net::Error> {
        self.flush_lobby_action(LobbyChat::Room)
    }

    /// Transmit one lobby chat draft through the slash grammar.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (see [`Self::flush_chat`]).
    fn flush_lobby_action(&mut self, chat: LobbyChat) -> Result<bool, o2jam_net::Error> {
        match self.game.lobby_slash_command(chat) {
            SlashCommand::Empty => Ok(false),
            SlashCommand::Help => {
                self.game.clear_lobby_input(chat);
                Ok(true)
            }
            SlashCommand::Chat(_) => self.flush_lobby_chat(chat),
            SlashCommand::Whisper { recipient, content } => {
                self.game.set_lobby_input(chat, &content);
                self.flush_lobby_whisper(chat, &recipient)
            }
        }
    }

    /// Transmit the pending waiting draft through the slash grammar:
    /// `/w <name> <text>` and `/r <text>` send 2017 (the parsed
    /// recipient routes through [`Self::flush_chat_whisper`]), `/h`
    /// and `/help` consume the draft with no packet, and anything
    /// else sends 3011 as plain chat.
    ///
    /// `Ok(true)` consumed the draft (sent or help); `Ok(false)`
    /// means the draft was empty and the socket stayed untouched. A
    /// failed send keeps the draft, so the next send retries it.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02: `Result`,
    /// propagated with `?`, never unwrapped per ERRORS-06).
    pub fn flush_chat_action(&mut self) -> Result<bool, o2jam_net::Error> {
        match self.game.chat_slash_command() {
            SlashCommand::Empty => Ok(false),
            SlashCommand::Help => {
                self.game.clear_chat_input();
                Ok(true)
            }
            SlashCommand::Chat(_) => self.flush_chat(),
            SlashCommand::Whisper { recipient, content } => {
                self.game.set_chat_input(&content);
                self.flush_chat_whisper(&recipient)
            }
        }
    }

    /// Stage an interactive channel login attempt (ticket #30 entry,
    /// ticket #172 pending shape): no socket IO — the Confirm/press
    /// path only queues the row, and the per-frame
    /// [`Self::poll_channel_login`] sends `ChannelLogin` 1004 for it,
    /// then `GetCharacterInfo` 2001 on the 1005 answer, staged through
    /// the boot path's [`crate::stage_online_identity`] — the same
    /// login-then-character order the windowed boot exchanges
    /// (`connect_and_login_with_transport`), since the server rejects
    /// 2001 outside a channel. Out-of-range ids warn and stage
    /// nothing, keeping Channel. Room rows are not refetched here
    /// (OPEN: the list-only boot stages none, so the Room lobby still
    /// lists the boot snapshot).
    fn begin_channel_login(&mut self, channel: ChannelState, origin: EntryEcho) {
        let (Ok(gateway_id), Ok(channel_id)) = (
            i16::try_from(channel.gateway_id),
            i16::try_from(channel.channel_id),
        ) else {
            tracing::warn!(
                gateway_id = channel.gateway_id,
                channel_id = channel.channel_id,
                "channel ids out of wire-short range; staying in Channel"
            );
            return;
        };
        self.login_attempt = Some(ChannelLoginAttempt {
            gateway_id,
            channel_id,
            origin,
            login_sent: false,
            identity_sent: false,
            deadline: Instant::now() + PENDING_WIRE_WINDOW,
        });
    }

    /// Per-frame channel-login section on `Channel`: move the pending
    /// attempt under the short pending drain, ahead of the lobby-chat
    /// drain (which would otherwise eat the 1005/2001 answers as
    /// foreign frames). Off Channel the attempt drops — a local-only
    /// Back owns the screen then, and no late answer may advance it —
    /// and past the deadline it warns and clears, keeping Channel so
    /// the next entry retries it.
    fn poll_channel_login(&mut self) {
        if self.game.screen() != Screen::Channel {
            if self.login_attempt.take().is_some() {
                tracing::debug!("channel login left behind leaving Channel; dropping it");
            }
            return;
        }
        let Some(attempt) = self.login_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            self.clear_resident_channel();
            self.clear_channel_echo();
            tracing::warn!(
                gateway_id = attempt.gateway_id,
                channel_id = attempt.channel_id,
                "channel login timed out with no server answer; staying in Channel"
            );
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "channel login", |session| {
            if let Some(kept) = session.drain_channel_login(attempt) {
                session.login_attempt = Some(kept);
            }
        });
    }

    /// Move one channel-login attempt leg: send what the stage still
    /// owes, then drain until the socket goes quiet. Returns the
    /// attempt while it still awaits an answer (`None` once it
    /// resolved or failed, so the next entry retries it).
    fn drain_channel_login(
        &mut self,
        mut attempt: ChannelLoginAttempt,
    ) -> Option<ChannelLoginAttempt> {
        if !attempt.login_sent {
            let login = ChannelLogin {
                gateway_id: attempt.gateway_id,
                channel_id: attempt.channel_id,
            };
            if let Err(source) = self
                .gateway
                .send_frame(Request::ChannelLogin.id(), &login.encode_body())
            {
                tracing::debug!(%source, "channel login send failed; retrying next paint");
                return Some(attempt);
            }
            attempt.login_sent = true;
        }
        loop {
            match self.gateway.next_frame() {
                Ok(frame)
                    if frame.id == ChannelLoginResponse::COMMAND && !attempt.identity_sent =>
                {
                    let Ok(response) = ChannelLoginResponse::decode_frame(&frame) else {
                        tracing::debug!(id = frame.id, "dropping undecodable channel login");
                        return None;
                    };
                    if ChannelLoginResult::from_code(response.result_code)
                        != Some(ChannelLoginResult::Success)
                    {
                        tracing::warn!(
                            code = response.result_code,
                            "channel login rejected; staying in Channel"
                        );
                        return None;
                    }
                    if let Err(source) =
                        self.gateway.send_frame(Request::GetCharacterInfo.id(), &[])
                    {
                        tracing::debug!(%source, "character fetch send failed; retrying next paint");
                        return Some(attempt);
                    }
                    attempt.identity_sent = true;
                }
                Ok(frame) if frame.id == CharacterInfo::COMMAND && attempt.identity_sent => {
                    let Ok(hero) = CharacterInfo::decode_frame(&frame) else {
                        tracing::debug!(id = frame.id, "dropping undecodable character info");
                        return None;
                    };
                    if hero.invalid {
                        tracing::warn!("server flagged the account invalid; staying in Channel");
                        return None;
                    }
                    self.resident_channel = Some(ChannelState {
                        gateway_id: attempt.gateway_id as u16,
                        channel_id: attempt.channel_id as u16,
                        capacity: 0,
                        population: 0,
                        active: true,
                    });
                    self.resolve_channel_login(hero, attempt.origin);
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-login frame in login drain");
                }
                Err(_) => break,
            }
        }
        Some(attempt)
    }

    /// Stage a completed channel login: the fetched identity lands
    /// through [`crate::stage_online_identity`], then the advance
    /// replays the originating input — the old synchronous Confirm
    /// shape, deferred to the poll.
    fn resolve_channel_login(&mut self, hero: CharacterInfo, origin: EntryEcho) {
        crate::stage_online_identity(&mut self.game, Some(&hero));
        self.echo_entry(origin);
    }

    /// Whether the boot already logged into this row. Both entry
    /// gestures must skip the 1004 then: Memoryer answers a second
    /// login with 1001 and never stages the channel (issue #331).
    fn is_resident_channel(&self, channel: &ChannelState) -> bool {
        self.resident_channel.as_ref().is_some_and(|resident| {
            (resident.gateway_id, resident.channel_id) == (channel.gateway_id, channel.channel_id)
        })
    }

    /// Channel Confirm with an empty draft: replay the local advance
    /// on a resident row, otherwise queue the login for the highlighted
    /// row; the per-frame poll advances like the offline walk once the
    /// identity stages. With no listed rows the bare local advance
    /// stays.
    fn confirm_channel(&mut self) {
        let Some(channel) = self.game.highlighted_channel() else {
            self.echo_entry(EntryEcho::Confirm);
            return;
        };
        if self.is_resident_channel(&channel) {
            self.echo_entry(EntryEcho::Confirm);
            return;
        }
        self.begin_channel_login(channel, EntryEcho::Confirm);
    }

    /// Stage a Room Confirm join attempt (ticket #172): no socket
    /// IO — the Confirm/press path only queues the pending number
    /// and password, and the per-frame [`Self::poll_room_join`]
    /// sends `JoinWaiting` (3002, the `connect --join-room` request
    /// shape) and stages the 3003 answer's room through
    /// [`ScreenGame::set_joined_room`].
    fn begin_room_join(&mut self, origin: EntryEcho) {
        let Some(pending) = self.pending_room.clone() else {
            return;
        };
        self.join_attempt = Some(RoomJoinAttempt {
            number: pending.number,
            password: pending.password,
            origin,
            request_sent: false,
            deadline: Instant::now() + PENDING_WIRE_WINDOW,
        });
    }

    /// Per-frame room-join section in `Room`: move the pending attempt
    /// under the short pending drain, ahead of the lobby-chat drain
    /// (which would otherwise eat the 3003 answer as a foreign
    /// frame). Off Room the attempt drops — the intent stays, so the
    /// next Room Confirm retries it — and past the deadline it warns
    /// and clears the same way.
    fn poll_room_join(&mut self) {
        if self.game.screen() != Screen::Room {
            if self.join_attempt.take().is_some() {
                tracing::debug!("room join left behind leaving Room; dropping it");
            }
            return;
        }
        let Some(attempt) = self.join_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            tracing::warn!("room join timed out with no server answer; staying in Room");
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "room join", |session| {
            if let Some(kept) = session.drain_room_join(attempt) {
                session.join_attempt = Some(kept);
            }
        });
    }

    /// Move one room-join attempt leg: send the 3002 request unless it
    /// already went out, then drain until the socket goes quiet.
    /// Returns the attempt while it still awaits the answer (`None`
    /// once it resolved or failed — the intent stays, so the next
    /// Confirm retries it). Live broadcasts (our own RoomCreated
    /// echo included) skip past like the old skip-loop, and every
    /// path warns, so no outcome is silent.
    fn drain_room_join(&mut self, mut attempt: RoomJoinAttempt) -> Option<RoomJoinAttempt> {
        if !attempt.request_sent {
            let request = JoinRoom {
                number: attempt.number,
                music_state: 0,
                password: attempt.password.clone(),
            };
            if let Err(source) = self
                .gateway
                .send_frame(Request::JoinWaiting.id(), &request.encode_body())
            {
                tracing::debug!(%source, "room join send failed; retrying next paint");
                return Some(attempt);
            }
            attempt.request_sent = true;
        }
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == JoinRoomResponse::COMMAND => {
                    let Ok(response) = JoinRoomResponse::decode_frame(&frame) else {
                        tracing::debug!(id = frame.id, "dropping undecodable join answer");
                        return None;
                    };
                    if !matches!(
                        JoinResult::from_code(response.result_code),
                        Some(JoinResult::Success)
                    ) {
                        tracing::warn!(
                            code = response.result_code,
                            "room join rejected; staying in Room"
                        );
                        return None;
                    }
                    let Some(room) = response.room else {
                        tracing::warn!("room join answered without a snapshot; staying in Room");
                        return None;
                    };
                    self.resolve_room_join(room, attempt.origin);
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-join frame in join drain");
                }
                Err(_) => break,
            }
        }
        Some(attempt)
    }

    /// Stage a completed room join: the 3003 snapshot lands through
    /// [`ScreenGame::set_joined_room`], then the advance replays the
    /// originating input — the old synchronous Confirm shape,
    /// deferred to the poll.
    fn resolve_room_join(&mut self, room: JoinedRoom, origin: EntryEcho) {
        self.resident_room = Some(room.title.clone());
        self.joined_room = Some(room.clone());
        self.game.set_joined_room(room);
        self.joined_waiting = true;
        restage_live_playing(&mut self.game);
        self.echo_entry(origin);
    }

    /// Waiting Confirm with an empty draft: slash drafts route through
    /// [`Self::flush_chat_action`] first (chat/whisper/help — no
    /// readiness change); with no joined room the offline walk keeps
    /// its local Waiting → Loading enter; a joined master sends
    /// `StartGame` 4010 while the all-peers-ready gate holds; a joined
    /// guest flips its `Ready` 4008 toggle. START/READY failures warn
    /// and keep state, so the next Confirm retries the same send.
    fn confirm_waiting(&mut self) {
        if self.game.chat_slash_command() != SlashCommand::Empty {
            if let Err(source) = self.flush_chat_action() {
                tracing::warn!(%source, "chat send failed; draft kept");
            }
            return;
        }
        if !self.joined_waiting {
            self.game.menu_key(MenuKey::Confirm, true);
            return;
        }
        if self.pending_song.is_some() && self.game.is_room_master() {
            self.confirm_song_select();
            return;
        }
        if self.game.is_room_master() {
            self.confirm_master_start();
        } else {
            self.confirm_guest_ready();
        }
    }

    /// Master Waiting Confirm with a pending host pick: queue the
    /// 4000/4002/4030 select; the per-frame poll stages it and stays
    /// in Waiting — the next Confirm sends START through the
    /// all-peers-ready gate, so the host sees the staged pick first.
    fn confirm_song_select(&mut self) {
        let Some(choice) = self.pending_song else {
            return;
        };
        self.begin_song_select(choice);
    }

    /// Drain one queued song pick (ticket #232): once the select-music
    /// dialog has closed, convert the shell's `pending_music_send`
    /// into a `SongChoice` (arena from the joined room) and hand it to
    /// the existing `confirm_song_select` path. No conversion while
    /// the dialog is open or an attempt is in flight, and no second
    /// send is invented.
    pub fn flush_music_send(&mut self) {
        if self.select_attempt.is_some() {
            return;
        }
        if self.pending_song.is_some() {
            return;
        }
        if self.game.select_music_dialog_open() {
            return;
        }
        let Some(pick) = self.game.take_music_send() else {
            return;
        };
        let arena_id = self
            .joined_room
            .as_ref()
            .map(|room| room.arena)
            .unwrap_or(0);
        self.pending_song = Some(SongChoice {
            music_id: pick.music_id,
            difficulty: pick.difficulty,
            speed: pick.speed,
            arena_id,
        });
        self.confirm_song_select();
    }

    /// Drain one queued create-room commit (ticket #269): the shell's
    /// dialog Commit queues `pending_create_room` (the 2004 body); this
    /// bridge moves the committed title through the existing host path
    /// [`Self::create_and_join_room`] (CreateRoom 2004 -> room list
    /// row -> `stage_hosted_room`), then steps the scene into Waiting.
    /// No second create send is invented. A create already in flight
    /// (joined room staged) is left alone so a retry does not
    /// double-create.
    pub fn flush_create_room(&mut self) {
        if self.joined_room.is_some() && self.joined_waiting {
            let _ = self.game.take_create_room();
            return;
        }
        let Some(body) = self.game.take_create_room() else {
            return;
        };
        match self.create_and_join_room(&body.title) {
            Ok(_) => {
                let _ = self.game.enter_screen(Screen::Waiting);
            }
            Err(source) => {
                tracing::warn!(%source, "create room failed; staying in Room");
            }
        }
    }

    /// Drain one queued password-dialog join (ticket #269): the shell's
    /// password Commit queues `pending_join` (a `JoinRoom` 3002 body)
    /// and steps to Waiting; this bridge sends `JoinWaiting` (3002)
    /// through the gateway and stages the answer's room, mirroring
    /// `resolve_room_join` without re-running the origin replay (the
    /// shell already moved to Waiting). No second join send beyond the
    /// existing 3002 shape is invented.
    pub fn flush_join(&mut self) {
        let Some(body) = self.game.take_join() else {
            return;
        };
        if let Err(source) = self
            .gateway
            .send_frame(Request::JoinWaiting.id(), &body.encode_body())
        {
            tracing::warn!(%source, "password join send failed; staying in Waiting");
            return;
        }
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == JoinRoomResponse::COMMAND => {
                    if let Ok(response) = JoinRoomResponse::decode_frame(&frame) {
                        if matches!(
                            JoinResult::from_code(response.result_code),
                            Some(JoinResult::Success)
                        ) {
                            if let Some(room) = response.room {
                                self.resident_room = Some(room.title.clone());
                                self.joined_room = Some(room.clone());
                                self.game.set_joined_room(room);
                                self.joined_waiting = true;
                                restage_live_playing(&mut self.game);
                                let _ = self.game.enter_screen(Screen::Waiting);
                            }
                        } else {
                            tracing::warn!(
                                code = response.result_code,
                                "password join rejected; staying in Waiting"
                            );
                        }
                    }
                    break;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }

    /// Joined-master Confirm: run the shell's START gate and send what
    /// its verdict asks for (`docs/music-availability.md` §4, exe click
    /// handler VA `0x4b22d0`).
    ///
    /// - startable -> `4010 StartGame` (the `run_song_flow` send shape,
    ///   minus the `ConfirmMusicLoaded` tail, which Loading owns);
    /// - a member lacks the room song -> `4034 AcquireMusicRequest`,
    ///   whose 4035 answer hands that member its download
    ///   ([`Self::apply_acquire_music`]);
    /// - a peer unready / teams unbalanced / a download standing -> no
    ///   packet, the shell's pinned notice says why.
    ///
    /// Loading waits on the 4011 broadcast ([`Self::poll_start`]), never
    /// on this send, and a refused 4011 keeps the room in Waiting.
    fn confirm_master_start(&mut self) {
        let request = self.game.press_start();
        let (id, body, sent) = match request {
            StartRequest::StartGame => (Request::StartGame.id(), StartGame.encode_body(), "start"),
            StartRequest::AcquireMusic => (
                Request::AcquireMusicRequest.id(),
                AcquireMusicRequest.encode_body(),
                "acquire music",
            ),
            StartRequest::Nothing => {
                if let Some(notice) = self.game.waiting_notice() {
                    tracing::warn!(notice, "start blocked by the group gate");
                }
                return;
            }
        };
        if let Err(source) = self.gateway.send_frame(id, &body) {
            tracing::warn!(%source, action = sent, "start send failed; Confirm retries it");
        }
    }

    /// Joined-guest Confirm: flip the local ready toggle and send the
    /// empty `Ready` 4008 ping. A failed send flips the toggle back
    /// (the keep-for-retry shape: the next Confirm sends again). The
    /// shell stays in Waiting either way — Loading arrives on the
    /// 4011 broadcast.
    fn confirm_guest_ready(&mut self) {
        self.guest_ready = !self.guest_ready;
        if let Err(source) = self
            .gateway
            .send_frame(Request::Ready.id(), &Ready.encode_body())
        {
            self.guest_ready = !self.guest_ready;
            tracing::warn!(%source, "ready send failed; Confirm retries it");
        }
        // Fold the final toggle into the seats so the READY button and
        // flags follow it (failures already flipped back above).
        if let Some(id) = self.joined_room.as_ref().map(|room| room.member_id) {
            self.game.set_member_ready(id, self.guest_ready);
        }
    }

    /// Joined Loading Confirm: send `ConfirmMusicLoaded` 4012 (the
    /// `run_song_flow` loaded shape, minus the headless tick tail)
    /// and enter Playing. The chart staged earlier (host select
    /// fetch, or manual `--chart`/`--ojm`), so the send runs at
    /// Playing entry — after staging, never before. A failed send
    /// keeps Loading, so the next Confirm retries it; an unknown
    /// speed waits too (never an invented one). Without a join the
    /// offline walk keeps its local enter with no packet.
    fn confirm_loading(&mut self) {
        if !self.joined_waiting {
            self.game.menu_key(MenuKey::Confirm, true);
            return;
        }
        if self.loaded_sent {
            self.game.menu_key(MenuKey::Confirm, true);
            return;
        }
        let Some(speed) = self.tally_speed() else {
            tracing::warn!("loaded send blocked: no staged speed; Confirm retries it");
            return;
        };
        let loaded = ConfirmMusicLoaded {
            power_skill_id: 0,
            speed,
        };
        if let Err(source) = self
            .gateway
            .send_frame(Request::ConfirmMusicLoaded.id(), &loaded.encode_body())
        {
            tracing::warn!(%source, "loaded send failed; Confirm retries it");
            return;
        }
        self.loaded_sent = true;
        self.game.menu_key(MenuKey::Confirm, true);
    }

    /// Speed behind the 4012/4016 sends: the staged pick first, else
    /// the joined room's (`None` until a select or join stages one).
    fn tally_speed(&self) -> Option<u8> {
        if let Some(select) = self.last_selection {
            return Some(select.speed);
        }
        self.joined_room.as_ref().map(|room| room.speed)
    }

    /// Per-frame stats section in `Playing`: one change-gated 4014 jam
    /// tick, then the 4015 echo drain — both under the short stats
    /// timeout so a quiet gateway never stalls the frame loop.
    fn poll_stats(&mut self) {
        if self.game.screen() != Screen::Playing {
            return;
        }
        self.with_drain_timeout(STATS_IO_TIMEOUT, "stats", |session| {
            session.maybe_send_tick();
            session.drain_peer_echoes();
        });
    }

    /// Live score snapshot behind the ticks: row 0 of the live rank rows
    /// carries the local points, jam level, and hold bonus (`None` only
    /// while the rows cannot name the local seat, never in practice);
    /// the life word rides [`ScreenGame::life`], which row 0 never
    /// carries. Any moved field resends both channels together.
    fn stats_snapshot(&self) -> Option<TickSnapshot> {
        let rows = self.game.result_rank_rows();
        let row = rows.first()?;
        Some(TickSnapshot {
            points: row.score?,
            jam_level: row.combos?.1,
            bonus: row.bonus?,
            life: self.game.life(),
        })
    }

    /// Send one 4014 jam tick plus one 4014 life tick when the live
    /// snapshot moved since the last send (jam first on the current
    /// sequence, life on the next — one sequence step per channel).
    /// A failed send keeps the snapshot unsent, so the next paint
    /// retries it (same keep-on-failure shape as [`flush_chat`]): a
    /// life failure after a sent jam resends the jam once, which the
    /// per-hit dedupe tolerates.
    fn maybe_send_tick(&mut self) {
        let Some(snapshot) = self.stats_snapshot() else {
            return;
        };
        if self.last_tick == Some(snapshot) {
            return;
        }
        let jam = jam_tick_for_score(
            u64::from(snapshot.points),
            snapshot.jam_level,
            self.stats_sequence,
            saturate_bonus(snapshot.bonus),
        );
        let life = life_tick_for_live_play(
            self.game.live_play(),
            snapshot.life,
            self.stats_sequence.saturating_add(1),
        );
        if let Err(source) = self
            .gateway
            .send_frame(Request::UpdateGameStats.id(), &jam.encode_body())
        {
            tracing::debug!(%source, "stats tick send failed; retrying next paint");
            return;
        }
        match self
            .gateway
            .send_frame(Request::UpdateGameStats.id(), &life.encode_body())
        {
            Ok(()) => {
                self.last_tick = Some(snapshot);
                self.stats_sequence = self.stats_sequence.saturating_add(2);
            }
            Err(source) => {
                tracing::debug!(%source, "stats tick send failed; retrying next paint");
            }
        }
    }

    /// Drain waiting 4015 echoes into the peer mirror until the socket
    /// goes quiet: each echo applies through [`apply_peer_update`]
    /// (`run_song_flow`'s peer-bar window shape, minus the wait).
    /// Keepalives skip silently; foreign ids log and skip — the drain is
    /// best-effort, never a dispatch (`ERRORS-19`: nothing is dropped
    /// without a trace).
    fn drain_peer_echoes(&mut self) {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == GameStatsUpdate::COMMAND => self.apply_echo(&frame),
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-stats frame in echo drain");
                }
                Err(_) => break,
            }
        }
    }

    /// Fold one 4015 frame into the mirror (undecodable bodies drop with
    /// a trace; the socket position is already past them) and forward
    /// it into the shell, so the Playing paint shows the live gauges
    /// and result rows from the same echo.
    fn apply_echo(&mut self, frame: &Frame) {
        let Ok(update) = GameStatsUpdate::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable peer echo");
            return;
        };
        let applied = apply_peer_update(&mut self.peers, update);
        self.peer_queue = applied.queue;
        self.peer_ranks = applied.ranks;
        let _ = self.game.apply_peer_echo(
            update.member_id,
            update.kind_code,
            update.value,
            update.rank_packet(),
        );
    }

    /// Result tally section: the first paint in Result after a joined
    /// play submits the live tally triple and stages the battle
    /// mirror, and every joined Result paint drains the tally answers.
    /// A failed send retries on the next paint (same keep-on-failure
    /// shape as [`Self::maybe_send_tick`]); leaving Result clears the
    /// latch so the next song tallies again. The offline walk never
    /// sends.
    fn poll_tally(&mut self) {
        if self.game.screen() != Screen::Result {
            self.tally_sent = false;
            return;
        }
        if !self.joined_waiting {
            return;
        }
        if !self.tally_sent {
            let Some(speed) = self.tally_speed() else {
                tracing::warn!("tally blocked: no staged speed; retrying next paint");
                return;
            };
            match self.send_tally(speed) {
                Ok(()) => self.tally_sent = true,
                Err(source) => {
                    tracing::warn!(%source, "tally send failed; retrying next paint");
                    return;
                }
            }
        }
        self.drain_tally_answers();
    }

    /// Drain waiting tally answers into the battle mirror until the
    /// socket goes quiet — the 4015 echo drain's best-effort shape:
    /// keepalives skip silently, foreign ids log and skip, undecodable
    /// bodies drop with a trace (`ERRORS-19`: nothing is dropped
    /// without a trace).
    fn drain_tally_answers(&mut self) {
        self.with_drain_timeout(TALLY_IO_TIMEOUT, "tally", |session| loop {
            match session.gateway.next_frame() {
                Ok(frame) if frame.id == ScoreSubmission::COMMAND => {
                    session.apply_score_submission(&frame);
                }
                Ok(frame) if frame.id == ScoreCompleted::COMMAND => {
                    session.apply_score_completed(&frame);
                }
                Ok(frame) if frame.id == GameRankUpdate::COMMAND => {
                    session.apply_rank_update(&frame);
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-tally frame in tally drain");
                }
                Err(_) => break,
            }
        });
    }

    /// Fold one 4017 frame: the submitting member is traced (no totals
    /// ride this broadcast — those arrive with 4018).
    fn apply_score_submission(&mut self, frame: &Frame) {
        match ScoreSubmission::decode_frame(frame) {
            Ok(submitted) => {
                tracing::debug!(member = submitted.member_id, "tally submitted");
            }
            Err(source) => {
                tracing::debug!(id = frame.id, %source, "dropping undecodable tally notice");
            }
        }
    }

    /// Fold one 4018 frame: every entry total re-stages onto its seat,
    /// so the verdict turns on server-authoritative peer totals instead
    /// of the local-only staging from the send (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    fn apply_score_completed(&mut self, frame: &Frame) {
        let Ok(completed) = ScoreCompleted::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable score table");
            return;
        };
        self.stage_result_records(&completed);
        self.stage_server_totals(&completed);
    }

    /// Fold one 4040 frame: decoded and validated (the places mirror
    /// the entry order the 4018 table already stages), then traced.
    fn apply_rank_update(&mut self, frame: &Frame) {
        match GameRankUpdate::decode_frame(frame) {
            Ok(ranks) => {
                tracing::debug!(?ranks.member_ranks, "rank places");
            }
            Err(source) => {
                tracing::debug!(id = frame.id, %source, "dropping undecodable rank places");
            }
        }
    }

    /// Result input section: drain the staged result presses through
    /// the gateway — RETRY posts `StartGame` 4010 (`0xfaa`, empty),
    /// BACK posts `ExitPlaying` 4021 (`0xfb5`, empty), and a battle
    /// result entry posts `GetLiveState` 4042 (`0xfca`, empty; the
    /// exe's battle-gated send, `docs/relay.md` §10). A failed send
    /// warns: the screen has already moved on (the tally send keeps
    /// its retry latch instead because Result stays). The offline
    /// shell never reaches this bridge, so its queued presses drain
    /// nowhere — the local screen arms still own the offline walk.
    fn flush_result_wire(&mut self) {
        if let Some(start) = self.game.take_result_retry() {
            self.send_result_frame(Request::StartGame.id(), &start.encode_body(), "retry");
        }
        if let Some(exit) = self.game.take_result_back() {
            self.send_result_frame(Request::ExitPlaying.id(), &exit.encode_body(), "back");
        }
        if let Some(live) = self.game.take_result_battle_entry() {
            self.send_result_frame(
                o2jam_net::relay::GetLiveState::COMMAND,
                &live.encode_body(),
                "battle live-state",
            );
        }
    }

    /// Per-frame channel-leave flush: one queued Room-back
    /// `ChannelLogout` 2021 (empty — the exe's exit dispatch sends it,
    /// ticket #175 pin). It runs before the login poll so a back-then-
    /// re-enter sequence never puts a fresh 1004 on the wire ahead of
    /// the leave: Memoryer answers a second 1004 with a 1001 `-1`
    /// auth error until the leave resets the session (probe, ticket
    /// #175). A failed send warns: the screen has already moved on.
    fn flush_channel_logout(&mut self) {
        if !self.game.take_channel_logout() {
            return;
        }
        self.clear_resident_channel();
        self.clear_channel_echo();
        self.resident_room = None;
        if let Err(source) = self.gateway.send_frame(Request::ChannelLogout.id(), &[]) {
            tracing::warn!(%source, "channel logout send failed");
        }
    }

    fn flush_shop_buy(&mut self) {
        // A buy already in flight must not be re-sent; leave any
        // queued commit staged so it retries once the reply lands.
        if self.shop_buy_attempt.is_some() {
            return;
        }
        if self.game.take_shop_buy().is_some() {
            if let Err(source) = self.gateway.send_frame(SyncItemPurchase::COMMAND, &[]) {
                tracing::warn!(%source, "shop buy send failed");
            } else {
                self.shop_buy_attempt = Some(ShopBuyAttempt {
                    deadline: Instant::now() + PENDING_WIRE_WINDOW,
                });
            }
        }
    }

    /// Per-frame item-shop buy section on the Shop side: move the
    /// pending attempt under the short pending drain. Off the Shop
    /// side the attempt drops (a local Back owns the screen then),
    /// and past the deadline it warns and clears, keeping the side so
    /// the next BUYITEM retries it.
    fn poll_shop_buy(&mut self) {
        if self.game.side() != Some(SideScreen::Shop) {
            if self.shop_buy_attempt.take().is_some() {
                tracing::debug!("shop buy left behind leaving the Shop side; dropping it");
            }
            return;
        }
        let Some(attempt) = self.shop_buy_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            tracing::warn!("shop buy timed out with no server answer; staying on Shop");
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "shop buy", |session| {
            if let Some(kept) = session.drain_shop_buy(attempt) {
                session.shop_buy_attempt = Some(kept);
            }
        });
    }

    /// Move one shop-buy attempt leg: drain until the socket goes
    /// quiet, folding the 5001 wallet prefix on the reply. Returns the
    /// attempt while it still awaits an answer.
    fn drain_shop_buy(&mut self, attempt: ShopBuyAttempt) -> Option<ShopBuyAttempt> {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == 5001 => {
                    if let Ok((prefix, _rest)) =
                        SyncItemPurchasePrefix::decode_prefix(&frame.payload)
                    {
                        self.game.apply_shop_purchase_prefix(&prefix);
                        self.game.apply_shop_purchase();
                    } else {
                        tracing::debug!(id = frame.id, "dropping undecodable shop buy reply");
                    }
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(
                        id = frame.id,
                        "skipping non-shop-buy frame in shop buy drain"
                    );
                }
                Err(_) => break,
            }
        }
        Some(attempt)
    }

    fn flush_music_buy(&mut self) {
        // A buy already in flight must not be re-sent; leave any
        // queued commit staged so it retries once the reply lands.
        if self.music_buy_attempt.is_some() {
            return;
        }
        if self.game.take_music_buy().is_some() {
            if let Err(source) = self.gateway.send_frame(SyncMusicPurchase::COMMAND, &[]) {
                tracing::warn!(%source, "music buy send failed");
            } else {
                self.music_buy_attempt = Some(MusicBuyAttempt {
                    deadline: Instant::now() + PENDING_WIRE_WINDOW,
                });
            }
        }
    }

    /// Per-frame song-shop buy section on the MusicShop side: move
    /// the pending attempt under the short pending drain. Off the
    /// MusicShop side the attempt drops (a local Back owns the
    /// screen then), and past the deadline it warns and clears,
    /// keeping the side so the next buy retries it.
    fn poll_music_buy(&mut self) {
        if self.game.side() != Some(SideScreen::MusicShop) {
            if self.music_buy_attempt.take().is_some() {
                tracing::debug!("music buy left behind leaving the MusicShop side; dropping it");
            }
            return;
        }
        let Some(attempt) = self.music_buy_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            tracing::warn!("music buy timed out with no server answer; staying on MusicShop");
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "music buy", |session| {
            if let Some(kept) = session.drain_music_buy(attempt) {
                session.music_buy_attempt = Some(kept);
            }
        });
    }

    /// Move one song-shop buy attempt leg: drain until the socket
    /// goes quiet, folding the 5011 wallet words on the reply.
    /// Returns the attempt while it still awaits an answer.
    fn drain_music_buy(&mut self, attempt: MusicBuyAttempt) -> Option<MusicBuyAttempt> {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == 5011 => {
                    if let Ok(response) = SyncMusicPurchaseResponse::decode_body(&frame.payload) {
                        self.game.apply_music_purchase_response(&response);
                    } else {
                        tracing::debug!(id = frame.id, "dropping undecodable music buy reply");
                    }
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(
                        id = frame.id,
                        "skipping non-music-buy frame in music buy drain"
                    );
                }
                Err(_) => break,
            }
        }
        Some(attempt)
    }

    fn flush_gift_list(&mut self) {
        if self.gift_list_attempt.is_some() {
            return;
        }
        if self.game.take_gift_list_request().is_some() {
            if let Err(source) = self.gateway.send_frame(GetGiftMessageList::COMMAND, &[]) {
                tracing::warn!(%source, "gift list send failed");
            } else {
                self.gift_list_attempt = Some(GiftListAttempt {
                    deadline: Instant::now() + PENDING_WIRE_WINDOW,
                });
            }
        }
    }

    /// Per-frame gift-list section on the MyRoom side: move the
    /// pending 5017 attempt under the short pending drain. Off the
    /// MyRoom side the attempt drops, and past the deadline it warns
    /// and clears, keeping the side so the next entry retries it.
    fn poll_gift_list(&mut self) {
        if self.game.side() != Some(SideScreen::MyRoom) {
            if self.gift_list_attempt.take().is_some() {
                tracing::debug!("gift list left behind leaving the MyRoom side; dropping it");
            }
            return;
        }
        let Some(attempt) = self.gift_list_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            tracing::warn!("gift list timed out with no server answer; staying on MyRoom");
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "gift list", |session| {
            if let Some(kept) = session.drain_gift_list(attempt) {
                session.gift_list_attempt = Some(kept);
            }
        });
    }

    /// Move one gift-list attempt leg: drain until the socket goes
    /// quiet. The 5018 body is exe-ignored (`docs/gifts.md`), so the
    /// drain only clears the attempt — no staged data changes.
    /// Returns the attempt while it still awaits an answer.
    fn drain_gift_list(&mut self, attempt: GiftListAttempt) -> Option<GiftListAttempt> {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == 5018 => {
                    if GetGiftMessageListResponse::decode_body(&frame.payload).is_err() {
                        tracing::debug!(id = frame.id, "dropping undecodable gift list reply");
                    }
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(
                        id = frame.id,
                        "skipping non-gift-list frame in gift list drain"
                    );
                }
                Err(_) => break,
            }
        }
        Some(attempt)
    }

    fn flush_gift_claim(&mut self) {
        if self.gift_claim_attempt.is_some() {
            return;
        }
        if let Some(claim) = self.game.take_gift_claim() {
            let gift_id = claim.gift_id;
            if let Err(source) = self
                .gateway
                .send_frame(o2jam_net::gift::ClaimGift::COMMAND, &claim.encode_body())
            {
                tracing::warn!(%source, "gift claim send failed");
            } else {
                self.gift_claim_attempt = Some(GiftClaimAttempt {
                    gift_id,
                    deadline: Instant::now() + PENDING_WIRE_WINDOW,
                });
            }
        }
    }

    /// Per-frame gift-claim section on the MyRoom side: move the
    /// pending 5023 attempt under the short pending drain. Off the
    /// MyRoom side the attempt drops, and past the deadline it warns
    /// and clears, keeping the side so the next MOVE retries it.
    fn poll_gift_claim(&mut self) {
        if self.game.side() != Some(SideScreen::MyRoom) {
            if self.gift_claim_attempt.take().is_some() {
                tracing::debug!("gift claim left behind leaving the MyRoom side; dropping it");
            }
            return;
        }
        let Some(attempt) = self.gift_claim_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            tracing::warn!("gift claim timed out with no server answer; staying on MyRoom");
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "gift claim", |session| {
            if let Some(kept) = session.drain_gift_claim(attempt) {
                session.gift_claim_attempt = Some(kept);
            }
        });
    }

    /// Move one gift-claim attempt leg: drain until the socket goes
    /// quiet, folding a successful 5024 verdict into the staged bag.
    /// Returns the attempt while it still awaits an answer.
    fn drain_gift_claim(&mut self, attempt: GiftClaimAttempt) -> Option<GiftClaimAttempt> {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == 5024 => {
                    match ClaimGiftResponse::decode_body(&frame.payload) {
                        Ok(response)
                            if !response.invalid
                                && response.result_code
                                    == o2jam_net::gift::ClaimGiftResult::Success.as_u8() =>
                        {
                            self.game.apply_gift_claim_success(attempt.gift_id);
                        }
                        Ok(_) => {
                            tracing::debug!(id = frame.id, "gift claim rejected; staged box holds");
                        }
                        Err(_) => {
                            tracing::debug!(id = frame.id, "dropping undecodable gift claim reply");
                        }
                    }
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(
                        id = frame.id,
                        "skipping non-gift-claim frame in gift claim drain"
                    );
                }
                Err(_) => break,
            }
        }
        Some(attempt)
    }

    /// Send one result-screen request frame; a failed send warns (the
    /// screen has already moved on — the tally send keeps its retry
    /// latch instead because Result stays).
    fn send_result_frame(&mut self, command: u16, body: &[u8], what: &str) {
        if let Err(source) = self.gateway.send_frame(command, body) {
            tracing::warn!(%source, "result {what} send failed");
        }
    }

    /// Send the end-of-song triple in the headless `run_song_flow`
    /// order — `SubmitScore` 4016, `FinalizeRank` 4041, `ExitPlaying`
    /// 4021 — then stage the battle mirror. Any failed send aborts
    /// the rest, so the next paint retries the whole triple.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02:
    /// `Result`, propagated with `?`, never unwrapped per ERRORS-06).
    fn send_tally(&mut self, speed: u8) -> Result<(), o2jam_net::Error> {
        let tally = self.tally_packet(speed);
        self.gateway
            .send_frame(Request::SubmitScore.id(), &tally.encode_body())?;
        self.gateway
            .send_frame(Request::FinalizeRank.id(), &FinalizeRank.encode_body())?;
        self.gateway
            .send_frame(Request::ExitPlaying.id(), &ExitPlaying.encode_body())?;
        self.stage_battle_teams(u64::from(tally.score));
        Ok(())
    }

    /// Outgoing 4016 tally for the live play: judgement counts, best
    /// combo, jam level and peak, saturated points, life, and hold
    /// bonus — every narrowing conversion saturates (PERF-07:
    /// `TryFrom`, never `as`).
    fn tally_packet(&self, speed: u8) -> SubmitScore {
        let play = self.game.live_play();
        let score = play.score();
        SubmitScore {
            cool: saturate_u16(score.counts[0]),
            good: saturate_u16(score.counts[1]),
            bad: saturate_u16(score.counts[2]),
            miss: saturate_u16(score.counts[3]),
            max_combo: saturate_u16(score.max_combo),
            jam: saturate_u16(score.jam.level()),
            max_jam: saturate_u16(score.jam.max_level()),
            score: saturate_u32(score.points),
            life: saturate_u8(self.game.life()),
            speed,
            long_note_score: saturate_bonus(play.bonus().points()),
        }
    }

    /// Battle mirror skeleton from the joined seats: teams per
    /// occupied slot, no totals yet (`None` while seat-less).
    fn battle_seats(&self) -> Option<RoomTeams> {
        let room = self.joined_room.as_ref()?;
        let mut teams = RoomTeams::new();
        for slot in &room.slots {
            if let Some(member) = slot.member.as_ref() {
                teams.set_team(usize::from(slot.index), member.team);
            }
        }
        Some(teams)
    }

    /// Our team behind the battle verdict (`None` while seat-less).
    fn battle_self_team(&self) -> Option<u8> {
        let room = self.joined_room.as_ref()?;
        room.slots
            .iter()
            .find(|slot| slot.index == room.member_id)
            .and_then(|slot| slot.member.as_ref())
            .map(|member| member.team)
    }

    /// Stage one battle mirror into the shell.
    fn stage_teams(&mut self, teams: RoomTeams) {
        let self_team = self.battle_self_team();
        self.game.set_battle_teams(teams);
        self.game.set_battle_self_team(self_team);
    }

    /// Stage the battle mirror from the joined seats: teams per
    /// occupied slot, the local total on our seat, our team claimed
    /// for the verdict — a solo or seat-less room stages teams with
    /// no winner. The 4018 drain restages server totals over this.
    fn stage_battle_teams(&mut self, local_points: u64) {
        let Some(mut teams) = self.battle_seats() else {
            return;
        };
        if let Some(room) = self.joined_room.as_ref() {
            teams.submit(usize::from(room.member_id), local_points);
        }
        self.stage_teams(teams);
    }

    /// Fill the exe-pinned member record words from the admitted rows of
    /// one 4018 frame. The exe guards member `< 8` (file `0x15fd54`) and
    /// admits only `active == 1` (file `0x15fd8e`); other rows are
    /// consumed without a commit, so they leave no staged record.
    fn stage_result_records(&mut self, completed: &ScoreCompleted) {
        for entry in &completed.scores {
            let Some(record) = self.result_records.get_mut(usize::from(entry.member_id)) else {
                tracing::debug!(
                    member = entry.member_id,
                    "skipping 4018 row outside member range"
                );
                continue;
            };
            if entry.active {
                *record = Some(ResultMemberRecord::from_score_entry(
                    entry,
                    completed.room_master,
                ));
            }
        }
    }

    /// Re-stage the battle mirror from the server-ranked 4018 table:
    /// teams per occupied joined seat, every entry total on its seat.
    /// Server-authoritative — replaces the local-only staging from
    /// the send (a seat-less room stages nothing).
    fn stage_server_totals(&mut self, completed: &ScoreCompleted) {
        let Some(mut teams) = self.battle_seats() else {
            return;
        };
        for entry in &completed.scores {
            teams.submit(usize::from(entry.member_id), u64::from(entry.score));
        }
        self.stage_teams(teams);
    }

    /// Per-frame chat section in `Waiting`: drain inbound 3012/3013
    /// lines under the short chat timeout so a quiet gateway never
    /// stalls the frame loop — the same timeout-restore shape as
    /// [`Self::poll_channel_chat`].
    fn poll_waiting_chat(&mut self) {
        if self.game.screen() != Screen::Waiting {
            return;
        }
        self.with_drain_timeout(CHAT_IO_TIMEOUT, "waiting chat", |session| {
            session.drain_waiting_chat();
        });
        self.poll_start();
    }

    /// Start-broadcast section in `Waiting`: the 4011 drain sets
    /// `start_announced`, and an announced start advances the shell
    /// to Loading through its own Confirm gate — a pending chat draft
    /// holds the advance (peek-only, never eaten) so typed text is
    /// never lost to the transition. Masters advance through Confirm
    /// (the readiness rule still holds them); joined guests advance
    /// through `ScreenGame::force_advance_to_loading`, since the
    /// START gate passes masters only and a guest Confirm could never
    /// satisfy it. Leaving Waiting clears the announcement either way.
    fn poll_start(&mut self) {
        if self.game.screen() != Screen::Waiting {
            self.start_announced = false;
            return;
        }
        if !self.start_announced {
            return;
        }
        if self.game.chat_slash_command() != SlashCommand::Empty {
            return;
        }
        if self.joined_waiting && !self.game.is_room_master() {
            self.game.force_advance_to_loading();
        } else {
            self.game.menu_key(MenuKey::Confirm, true);
        }
        if self.game.screen() != Screen::Waiting {
            self.start_announced = false;
        }
    }

    /// Drain inbound 3012 player lines and 3013 administrator lines
    /// into the waiting history, plus the waiting music-state family
    /// (`4035`/`4037`/`4039`, `docs/music-availability.md` §2), until
    /// the socket goes quiet — the 4015 echo drain's best-effort
    /// shape: keepalives skip silently, foreign ids log and skip
    /// (`ERRORS-19`: nothing is dropped without a trace).
    fn drain_waiting_chat(&mut self) {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == WaitingUserMessage::COMMAND => {
                    self.apply_waiting_user_message(&frame);
                }
                Ok(frame) if frame.id == WaitingAdminMessage::COMMAND => {
                    self.apply_waiting_admin_message(&frame);
                }
                Ok(frame) if frame.id == START_BROADCAST_COMMAND => {
                    self.apply_start_broadcast(&frame.payload);
                }
                Ok(frame) if frame.id == RoomSlotUpdate::COMMAND => {
                    self.apply_slot_update(&frame);
                }
                Ok(frame) if frame.id == MemberReadyStateChanged::COMMAND => {
                    self.apply_ready_echo(&frame);
                }
                Ok(frame) if frame.id == SyncMemberMusicStateEvent::COMMAND => {
                    self.apply_member_music_state(&frame);
                }
                Ok(frame) if frame.id == AcquireMusic::COMMAND => {
                    self.apply_acquire_music(&frame);
                }
                Ok(frame) if frame.id == DownloadProgressChanged::COMMAND => {
                    self.apply_download_progress(&frame);
                }
                Ok(frame) if frame.id == WaitingMusicChanged::COMMAND => {
                    self.apply_waiting_music_changed(&frame);
                }
                Ok(frame) if announces_room_music_state(frame.id) => {
                    self.announce_music_state();
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-chat frame in waiting drain");
                }
                Err(_) => break,
            }
        }
    }

    /// Report each promoted chart the shell finished this frame: `4038`
    /// progress (100 %), then `5012` completion (§4's promote leg).
    /// A fetch still in flight sends nothing and retries next paint.
    fn poll_music_downloads(&mut self) {
        if self.game.screen() != Screen::Waiting {
            return;
        }
        let member_id = self
            .joined_room
            .as_ref()
            .map(|room| room.member_id)
            .unwrap_or(0);
        for completion in self.game.pump_music_downloads() {
            let progress = SetDownloadProgress {
                progress: 100,
                member_id,
            };
            if let Err(source) = self
                .gateway
                .send_frame(Request::SetDownloadProgress.id(), &progress.encode_body())
            {
                tracing::warn!(%source, "download progress send failed");
            }
            let announce = SyncMusicDownload {
                music_id: completion.song_id,
            };
            if let Err(source) = self
                .gateway
                .send_frame(Request::SyncMusicDownload.id(), &announce.encode_body())
            {
                tracing::warn!(%source, "music download announce failed");
            }
        }
    }

    /// Fold one 4037 frame into its waiting seat: one member's music
    /// state (§2, handler VA `0x559990`, the state landing in the
    /// seat's `+0x217c`), then run the exe's retry arm
    /// ([`Self::retry_master_start`]). Undecodable bodies drop with a
    /// trace.
    fn apply_member_music_state(&mut self, frame: &Frame) {
        let Ok(event) = SyncMemberMusicStateEvent::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable music state");
            return;
        };
        self.stage_member_music_state(event.member_id, event.state.code());
        if event.state == MusicState::Ready {
            self.finish_self_music_download();
            self.retry_master_start();
        }
    }

    /// Fold one 4035 frame (the counted member/state list the server
    /// hands out on START, §2/§4) into the waiting seats, then run the
    /// exe's 4035 arms ([`Self::answer_acquire_music`]).
    fn apply_acquire_music(&mut self, frame: &Frame) {
        let Ok(event) = AcquireMusic::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable acquire music");
            return;
        };
        for member in &event.members {
            self.stage_member_music_state(member.member_id, member.state.code());
        }
        self.answer_acquire_music(&event);
    }

    /// The exe's 4035 arms (VA `0x4b1d42` normal mode, `0x4aba51` live
    /// mode), per announced member:
    ///
    /// - a non-zero state means a transfer stands (`scene+0x504 = 1`)
    ///   and no `4010` goes out; when that state belongs to the local
    ///   member, the client **enqueues the room song instead of
    ///   starting** (`StartMusicDownload`, VA `0x4bb1e0`);
    /// - an all-`Ready` list is the exe's retry arm: a master whose
    ///   start request stands re-sends `4010`.
    fn answer_acquire_music(&mut self, event: &AcquireMusic) {
        let self_id = self.joined_room.as_ref().map(|room| room.member_id);
        let self_state = self_id.and_then(|id| {
            event
                .members
                .iter()
                .find(|member| member.member_id == id)
                .map(|member| member.state)
        });
        if event
            .members
            .iter()
            .any(|member| member.state != MusicState::Ready)
        {
            if self_state.is_some_and(|state| state != MusicState::Ready) {
                self.kick_room_song_download();
            }
            return;
        }
        self.finish_self_music_download();
        self.retry_master_start();
    }

    /// The download kick (`docs/music-availability.md` §4): enqueue the
    /// room song and mark the download running, so the START control
    /// only re-arms its face until the transfer lands. The per-frame
    /// pump advances the queue; with no fetch source configured it
    /// stays queued (§4).
    fn kick_room_song_download(&mut self) {
        let Some(song_id) = self.game.room_music_id() else {
            tracing::warn!("download kick without a staged room song");
            return;
        };
        if self.game.begin_music_download(song_id) {
            tracing::info!(song_id, "room song queued for download");
        }
    }

    /// A staged download whose song now reads `Ready` on our own seat is
    /// done: drop the request so the START control stops treating the
    /// room as busy. Slice 4's pump owns the real completion (progress,
    /// promote, `5012`); the seat state the server syncs back is the
    /// signal it reports through.
    fn finish_self_music_download(&mut self) {
        let Some(member_id) = self.joined_room.as_ref().map(|room| room.member_id) else {
            return;
        };
        if self.game.member_music_state(member_id) != Some(MusicState::Ready.code()) {
            return;
        }
        if let Some(song_id) = self.game.room_music_id() {
            self.game.finish_music_download(song_id);
        }
    }

    /// The exe's retry arm (4035 all-`Ready`, 4037, and the leave/slot
    /// updates): a master whose START request stands re-sends `4010`
    /// once the group gate passes again (the exe's master byte `+0x30`
    /// plus the gate verdict, VA `0x4b5560`). A guest never sends.
    fn retry_master_start(&mut self) {
        if !self.joined_waiting
            || !self.game.is_room_master()
            || !self.game.waiting_confirm_armed()
            || self.game.start_gate_verdict() != StartGateVerdict::Startable
        {
            return;
        }
        if let Err(source) = self
            .gateway
            .send_frame(Request::StartGame.id(), &StartGame.encode_body())
        {
            tracing::warn!(%source, "start retry failed; the next event retries it");
        }
    }

    /// Fold one 4001 frame: the room's song changed, so the shell
    /// restages it and the client announces its music state for the new
    /// song (the exe's 4001 arm, VAs `0x4ab591`/`0x4b0d4e`; handler VA
    /// `0x55f2d0` reads id, difficulty and speed).
    fn apply_waiting_music_changed(&mut self, frame: &Frame) {
        let Ok(event) = WaitingMusicChanged::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable room music change");
            return;
        };
        self.game
            .set_room_music(event.music_id, event.difficulty, event.speed);
        self.announce_music_state();
    }

    /// Announce this client's music state for the room's staged song
    /// (`UpdateMusicState` 4025, `docs/music-availability.md` §2): the
    /// exe's state computer VA `0x4be690` followed by the `push 0;
    /// push 0xfb9` send, at every one of its eight sites — the room-song
    /// change, a member leaving the game, and the two skill changes.
    ///
    /// No song stands -> no announcement: song id `0` names no chart
    /// file, and the exe computes against a staged id only.
    fn announce_music_state(&mut self) {
        let Some(song_id) = self.game.room_music_id() else {
            tracing::debug!("music state announcement skipped: no staged room song");
            return;
        };
        let state = UpdateMusicState {
            state: self.game.music_state_for(song_id),
        };
        if let Err(source) = self
            .gateway
            .send_frame(Request::UpdateMusicState.id(), &state.encode_body())
        {
            tracing::warn!(%source, "music state send failed");
        }
    }

    /// Fold one 4011 start broadcast: a `StartResult::Success` (or a
    /// body too short to carry a result — the bare broadcast shape the
    /// rigs send) announces the start, anything else is the server's
    /// refusal (a member still lacks the music) and keeps the room in
    /// Waiting with the download it kicked.
    fn apply_start_broadcast(&mut self, payload: &[u8]) {
        match start_broadcast_result(payload) {
            Some(result) if result != START_RESULT_SUCCESS => {
                tracing::info!(result, "start refused; staying in Waiting");
                self.game.note_waiting_download();
            }
            _ => self.start_announced = true,
        }
    }

    /// One member's music state into the shell seats.
    fn stage_member_music_state(&mut self, member_id: u8, state: u8) {
        tracing::debug!(member = member_id, state, "member music state");
        self.game.set_member_music_state(member_id, state);
    }

    /// Decode one 4039 download percentage (§2, handler VA `0x559a10`:
    /// percentage then member).
    ///
    /// No staging: the percentage has no seat field our client models
    /// yet — the exe forwards it to the waiting download bar
    /// (`Wait_Downloding.ojs`, §4) and the post-START flow that owns
    /// that art is slice 3/4 (#276). Traced, never silently eaten.
    fn apply_download_progress(&mut self, frame: &Frame) {
        match DownloadProgressChanged::decode_frame(frame) {
            Ok(progress) => tracing::debug!(
                member = progress.member_id,
                percent = progress.percentage,
                "download progress (no staged bar yet)"
            ),
            Err(_) => tracing::debug!(id = frame.id, "dropping undecodable download progress"),
        }
    }

    /// Fold one 3009 frame into the slot mirror (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    /// The exe drops wide indexes and unknown actions at the handler,
    /// so the decoder rejects both — a reject here is the same drop.
    fn apply_slot_update(&mut self, frame: &Frame) {
        let Ok(update) = RoomSlotUpdate::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable slot update");
            return;
        };
        self.slot_actions[usize::from(update.index)] = update.action();
    }

    /// Fold one 4009 frame into the ready mirror (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    fn apply_ready_echo(&mut self, frame: &Frame) {
        let Ok(echo) = MemberReadyStateChanged::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable ready echo");
            return;
        };
        self.ready_echoes[usize::from(echo.member_id)] = Some(echo.ready);
    }

    /// Fold one 3012 frame into the waiting history (undecodable
    /// bodies drop with a trace; the socket position is already past
    /// them).
    fn apply_waiting_user_message(&mut self, frame: &Frame) {
        let Ok(message) = WaitingUserMessage::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable waiting line");
            return;
        };
        self.game.push_waiting_user_message(&message);
    }

    /// Fold one 3013 frame into the waiting history (undecodable
    /// bodies drop with a trace; the socket position is already past
    /// them).
    fn apply_waiting_admin_message(&mut self, frame: &Frame) {
        let Ok(message) = WaitingAdminMessage::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable waiting line");
            return;
        };
        self.game.push_waiting_admin_message(&message);
    }

    /// Per-frame lobby chat section in `Channel` and `Room`: drain the
    /// main-room feed under the short notice timeout so a quiet gateway
    /// never stalls the frame loop — the same timeout-restore shape as
    /// [`Self::poll_stats`].
    fn poll_lobby_chat(&mut self) {
        let Some(chat) = LobbyChat::of_screen(self.game.screen()) else {
            return;
        };
        self.with_drain_timeout(NOTICE_IO_TIMEOUT, "lobby chat", |session| {
            session.drain_lobby_chat(chat);
        });
    }

    /// Drain waiting lobby chat lines (2013/2014), incoming whispers
    /// (2019), announcements (2016), and rank notices (2029) into
    /// `chat`'s history until the socket goes quiet — the 4015 echo
    /// drain's best-effort shape, minus the peer mirror: each frame
    /// applies through its shell push, keepalives skip silently,
    /// foreign ids log and skip (`ERRORS-19`: nothing is dropped
    /// without a trace).
    fn drain_lobby_chat(&mut self, chat: LobbyChat) {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == MainRoomUserMessage::COMMAND => {
                    self.apply_lobby_user_message(chat, &frame);
                }
                Ok(frame) if frame.id == MainRoomAdminMessage::COMMAND => {
                    self.apply_lobby_admin_message(chat, &frame);
                }
                Ok(frame) if frame.id == ReceiveWhisper::COMMAND => {
                    self.apply_lobby_whisper(chat, &frame);
                }
                Ok(frame) if frame.id == Announcement::COMMAND => {
                    self.apply_announcement(chat, &frame);
                }
                Ok(frame) if frame.id == RankNotification::COMMAND => {
                    self.apply_rank_notice(chat, &frame);
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-chat frame in lobby drain");
                }
                Err(_) => break,
            }
        }
    }

    /// Fold one 2013 frame into `chat`'s history (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    fn apply_lobby_user_message(&mut self, chat: LobbyChat, frame: &Frame) {
        let Ok(message) = MainRoomUserMessage::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable lobby line");
            return;
        };
        self.game.push_lobby_user_message(chat, &message);
    }

    /// Fold one 2014 frame into `chat`'s history (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    fn apply_lobby_admin_message(&mut self, chat: LobbyChat, frame: &Frame) {
        let Ok(message) = MainRoomAdminMessage::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable lobby line");
            return;
        };
        self.game.push_lobby_admin_message(chat, &message);
    }

    /// Fold one 2019 frame into `chat`'s history (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    /// The wire event carries only the sender, so the row names the
    /// remembered local user as the addressee — and the push remembers
    /// the sender for `/r`.
    fn apply_lobby_whisper(&mut self, chat: LobbyChat, frame: &Frame) {
        let Ok(message) = ReceiveWhisper::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable whisper");
            return;
        };
        let recipient = self.local_user.clone();
        self.game.push_lobby_whisper(chat, &message, &recipient);
    }

    /// Fold one 2016 frame into `chat`'s history (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    /// Empty notices stage no row — the [`ScreenGame`] push rule — so
    /// the drain never consumes a visible slot for one.
    fn apply_announcement(&mut self, chat: LobbyChat, frame: &Frame) {
        let Ok(notice) = Announcement::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable announcement");
            return;
        };
        self.game.push_lobby_announcement(chat, &notice);
    }

    /// Fold one 2029 frame into `chat`'s history (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    /// Invalid packets stage no row — the [`ScreenGame`] push rule —
    /// so the drain never invents a rank.
    fn apply_rank_notice(&mut self, chat: LobbyChat, frame: &Frame) {
        let Ok(notice) = RankNotification::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable rank notice");
            return;
        };
        self.game.push_lobby_rank_notification(chat, &notice);
    }
}

impl OnlineSession {
    /// Room-lobby Enter: `true` when the chat draft consumed it (one
    /// 2012/2017 send, or a send that failed and therefore kept the
    /// draft). An open room dialog owns the keyboard and the join/advance
    /// path, so the draft stays untouched then.
    fn room_chat_took_enter(&mut self) -> bool {
        if self.game.room_dialog().is_some() {
            return false;
        }
        match self.flush_room_chat_action() {
            Ok(sent) => sent,
            Err(source) => {
                tracing::warn!(%source, "room chat send failed; draft kept");
                true
            }
        }
    }

    /// Run the networked press gate for one edge. The press module
    /// owns the four hit tests, the decision, and the single echo;
    /// this seam only translates its disposition into gate truth.
    fn swallow_networked_edge(&mut self, edge: &InputEdge) -> bool {
        match self.dispatch_pointer_edge(edge) {
            PressDisposition::Swallow => true,
            PressDisposition::Local => false,
        }
    }

    /// `true` while a Confirm-paced attempt still awaits its answers
    /// (see the drain-order note on [`Self::poll_wire`]).
    fn wire_attempt_pending(&self) -> bool {
        self.login_attempt.is_some()
            || self.join_attempt.is_some()
            || self.select_attempt.is_some()
            || self.shop_buy_attempt.is_some()
            || self.music_buy_attempt.is_some()
            || self.gift_list_attempt.is_some()
            || self.gift_claim_attempt.is_some()
    }

    /// Drain one frame's gateway traffic before the canvas paints it.
    /// This order is what makes a wire-driven scene change visible in
    /// the same frame (#347: the 4011 guest advance used to land after
    /// `ScreenGame::paint`, leaving Waiting pixels under a loading
    /// label until unrelated input forced a redraw).
    ///
    /// Pending-wire polls run before the lobby/waiting drains:
    /// those skip foreign frames, so a later order would eat the
    /// 1005/2001, 3003, and 4003/4031 answers before they resolve.
    /// While an attempt is still in flight the lobby/waiting
    /// drains pause outright: a single-shot answer landing between
    /// two drains in one paint would otherwise die as a foreign
    /// frame with no retry. Chat resumes — nothing is lost, the
    /// socket buffers it — once the attempt resolves or times out;
    /// START still advances through `poll_start` meanwhile.
    /// The queued channel leave goes out first: a back-then-re-enter
    /// sequence must never put a fresh 1004 on the wire ahead of the
    /// 2021 leave (Memoryer answers it with a 1001 `-1` auth error
    /// until the leave resets the session, ticket #175 probe).
    fn poll_wire(&mut self) {
        self.flush_channel_logout();
        self.poll_attempts();
        self.poll_stats();
        if self.wire_attempt_pending() {
            self.poll_start();
        } else {
            self.poll_lobby_chat();
            self.poll_waiting_chat();
            self.poll_music_downloads();
        }
        self.poll_tally();
        self.flush_result_wire();
    }
}

impl GameLoop for OnlineSession {
    fn lane_key(&mut self, lane: usize, down: bool) {
        self.game.lane_key(lane, down);
    }

    fn menu_key(&mut self, key: MenuKey, down: bool) {
        if !down {
            return;
        }
        if key == MenuKey::Confirm && self.game.screen() == Screen::Waiting {
            self.confirm_waiting();
            return;
        }
        if key == MenuKey::Confirm && self.game.screen() == Screen::Channel {
            // A Room Back can queue the 2021 leave without a paint in
            // between. Send it first: the stale resident row must not
            // replay locally while the server still holds the channel.
            self.flush_channel_logout();
            match self.flush_channel_action() {
                Ok(sent) if !sent => self.confirm_channel(),
                Ok(_) => {}
                Err(source) => {
                    tracing::warn!(%source, "lobby send failed; draft kept");
                }
            }
            return;
        }
        if key == MenuKey::Confirm && self.game.screen() == Screen::Room {
            // A pending room chat draft consumes the Enter as one
            // 2012 send (ticket #37); an empty draft keeps the
            // join-then-advance path below.
            if self.room_chat_took_enter() {
                return;
            }
            if self.pending_room.is_none() {
                self.game.menu_key(key, down);
            } else {
                self.begin_room_join(EntryEcho::Confirm);
            }
            return;
        }
        if key == MenuKey::Confirm && self.game.screen() == Screen::Loading {
            self.confirm_loading();
            return;
        }
        self.game.menu_key(key, down);
    }

    fn text_input(&mut self, text: &str) {
        self.game.text_input(text);
    }

    fn delete_text(&mut self) {
        self.game.delete_text();
    }

    fn ingest_snapshot(&mut self, snapshot: &InputSnapshot) {
        // Snapshot-native sweep with the networked gates the driver
        // path used to own in the legacy whole-gesture edge: one snapshot carries the
        // tap, so the gate and the local advance read the same frame.
        // Channel enter-face presses queue the entry attempt (1004
        // login plus the 2001 identity fetch resolve on the poll)
        // before the poll replays the press, mirroring Confirm; other
        // Channel presses stay local exactly like their menu
        // equivalents. Room presses mirror Confirm: the join attempt
        // queues first, the poll replays the press on success — a
        // press that hits no room never sends unless a join is
        // already pending. Waiting
        // confirm rides the networked Confirm gate (4008/4010 with
        // the song-select poll and START behind it) and never reaches
        // the game; other Waiting presses stay local exactly like
        // their menu equivalents. Click evidence states (P0: every
        // path labeled, OPEN stays open). Anything unclaimed advances
        // locally with no send, like lane keys. Menu edges ride the
        // session's Confirm gates (Waiting/Channel/Room/Loading),
        // exactly like the legacy direct `menu_key` path — the game
        // never sees a gated Confirm.
        let mut surviving = InputSnapshot::new();
        surviving.set_pointer(snapshot.pointer());
        for edge in snapshot.edges() {
            if let InputEdge::Menu { key, down } = *edge {
                self.menu_key(key, down);
                continue;
            }
            if self.swallow_networked_edge(edge) {
                continue;
            }
            surviving.push_edge(*edge);
        }
        self.game.ingest_snapshot(&surviving);
    }

    fn quit_requested(&self) -> bool {
        self.game.quit_requested()
    }

    fn screen_name(&self) -> &'static str {
        self.game.screen_name()
    }

    fn state_hash(&self) -> u64 {
        self.game.state_hash()
    }

    fn idle_repaint_interval(&self) -> Option<std::time::Duration> {
        if let Some(gap) = self.game.idle_repaint_interval() {
            return Some(gap);
        }
        match self.game.screen() {
            Screen::Channel | Screen::Room | Screen::Waiting | Screen::Result => {
                Some(SESSION_POLL_HEARTBEAT)
            }
            _ => None,
        }
    }

    fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
        self.poll_wire();
        self.game.paint(frame);
        if self.game.screen() != Screen::Loading {
            self.loaded_sent = false;
        }
    }
}

/// Point the transfer engine at the configured local mirror. No
/// configured source leaves the queue idle: an install without a
/// mirror never invents a chart file (§4).
fn configure_music_source(game: &mut ScreenGame, source: Option<std::ffi::OsString>) {
    let Some(source) = source else {
        return;
    };
    game.set_music_download_dirs(
        o2jam_shell::music_install::install_music_dir(),
        Some(std::path::PathBuf::from(source)),
    );
}

/// Saturate the hold bonus onto the tick's `i32` tail (PERF-07:
/// `TryFrom`, never `as`).
fn saturate_bonus(bonus: u64) -> i32 {
    i32::try_from(bonus).unwrap_or(i32::MAX)
}

/// Saturate one tally count onto the 4016 `u16` fields (PERF-07:
/// `TryFrom`, never `as`).
fn saturate_u16(count: u32) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

/// Saturated local points for the 4016 score field (the shell reads
/// the same word for its rank row; PERF-07: `TryFrom`, never `as`).
fn saturate_u32(points: u64) -> u32 {
    u32::try_from(points).unwrap_or(u32::MAX)
}

/// Saturated life word for the 4016 life byte (the avatar gauge
/// reads `u16`; PERF-07: `TryFrom`, never `as`).
fn saturate_u8(life: u16) -> u8 {
    u8::try_from(life).unwrap_or(u8::MAX)
}

/// Borrow one catalog entry as an owned chart-fetch row.
fn catalog_row(entry: &OjnListEntry<'_>) -> CatalogRow {
    CatalogRow {
        song_id: entry.song_id,
        ojm_file: entry.ojm_file.clone(),
        title: entry.title.clone(),
        artist: entry.artist.clone(),
    }
}

/// Install `Music` folder under the repository-local `install-root`
/// (git-ignored game data; the same layout `shots.rs` reads the
/// catalog and the shell reads covers from).
fn install_music_dir() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root/O2Jam/Music")
}

/// Chart/bank file for one catalog container token (`o2ma757.ojm` ->
/// `Music/o2ma757.ojn` / `Music/o2ma757.ojm`): the shell cover path's
/// token swap, generalized over the extension. `None` when the token
/// carries no `.ojm` suffix — no invented fallback path.
fn install_music_path(token: &str, extension: &str) -> Option<PathBuf> {
    let stem_len = token.len().checked_sub(".ojm".len())?;
    let (stem, suffix) = token.split_at(stem_len);
    if !suffix.eq_ignore_ascii_case(".ojm") {
        return None;
    }
    Some(install_music_dir().join(format!("{stem}.{extension}")))
}

/// Chart level slot for one difficulty code (EX/NX/HX map onto the
/// three OJN slots; RX and unknown codes clamp onto Hard).
/// PERF-07: `from`, never `as`.
fn level_for_difficulty(difficulty: u8) -> usize {
    usize::from(difficulty).min(OJN_LEVEL_COUNT - 1)
}

#[cfg(test)]
pub(super) mod session_tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use o2jam_game::PEER_SLOT_COUNT;
    use o2jam_net::frame::{decode_frame, encode_frame};
    use o2jam_net::{
        AcquireMusic, Frame, GameStatsUpdate, GatewayConnection, MemberMusicState, MusicState,
        Request, StatKind, SyncMemberMusicStateEvent, UpdateGameStats, WaitingMusicChanged,
        WireMessage, HEADER_LEN,
    };
    use o2jam_shell::screens::game::LobbyChat;
    use o2jam_shell::{GameLoop, InputSnapshot, MenuKey, Screen, ScreenGame};

    use super::{
        level_for_difficulty, result_records::ResultMemberRecord, saturate_bonus, OnlineSession,
        SongChoice,
    };

    /// One whole frame off the fake server's socket.
    pub(super) fn read_frame(stream: &mut TcpStream) -> Frame {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
        let mut rest = vec![0u8; body_len - HEADER_LEN];
        stream.read_exact(&mut rest).unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&rest);
        decode_frame(&bytes).unwrap()
    }

    /// Fake gateway on loopback: returns its port plus the server thread
    /// running `with_server` (5 s reads, like the login-flow tests).
    fn fake_gateway(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (u16, thread::JoinHandle<()>) {
        o2jam_net::testing::fake_gateway(with_server)
    }

    /// Stage the S7 loading gate the way the live lane does
    /// (`stage_chart_bytes` stages the armed caption on song prep):
    /// a validated pick, the armed loading status, and a null rig —
    /// so a following Loading Confirm is allowed to arm the clock.
    fn stage_armed_loading(game: &mut ScreenGame) {
        use crate::loading::{loading_phase_status, LOADING_GATE_STEPS};
        use o2jam_audio::NullBackend;
        use o2jam_net::SetRoomMusic;
        use std::collections::HashMap;
        // Easy (difficulty 0): the hosted defaults the cleared
        // builders used before the gate existed (life drains
        // 50 on a Miss); changing it here would retitle the
        // tests' judgement economy.
        game.set_selected_music(SetRoomMusic {
            music_id: 757,
            difficulty: 0,
            speed: 0,
        });
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        if let Some(status) = loading_phase_status(LOADING_GATE_STEPS, "Song", "Band") {
            game.set_loading_status(status);
        }
    }

    /// Game walked Avi → Channel → Room → Waiting, paired with a live
    /// fake gateway; the server end runs `with_server`.
    fn waiting_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (port, server) = fake_gateway(with_server);
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        stage_armed_loading(&mut game);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        (OnlineSession::new(game, gateway), server)
    }

    /// Game walked Avi → Channel, paired with a live fake gateway; the
    /// server end runs `with_server`.
    pub(super) fn channel_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (port, server) = fake_gateway(with_server);
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        (OnlineSession::new(game, gateway), server)
    }
    /// Game walked Avi → Channel → Room, paired with a live fake
    /// gateway; the server end runs `with_server`.
    pub(super) fn room_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (port, server) = fake_gateway(with_server);
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        (OnlineSession::new(game, gateway), server)
    }

    /// Minimal joined snapshot: master seat of a 7-key room titled
    /// Lobby (the `connect --join-room` answer shape).
    pub(super) fn joined_snapshot() -> o2jam_net::JoinedRoom {
        let mut slots = Vec::new();
        for index in 0..7u8 {
            slots.push(o2jam_net::RoomSlot {
                index,
                state_code: 0,
                member: None,
            });
        }
        o2jam_net::JoinedRoom {
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
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    /// One seated member for staged snapshots (level/gender/gem
    /// zeros — only nickname and readiness matter to the gate).
    /// One seated peer on a given team. The server hands every joining
    /// member the first unused team (Memoryer `Room.Attach`), so a room
    /// with more than one member always spans distinct teams — which is
    /// exactly what the exe's team-balance gate (VA `0x4b5560`) needs to
    /// pass. Both members on one team is a shape the server never sends.
    fn member_on_team(nickname: &str, ready: bool, team: u8) -> o2jam_net::RoomMember {
        o2jam_net::RoomMember {
            nickname: nickname.to_owned(),
            level: 1,
            gender: 0,
            gem: 0,
            master: false,
            team,
            ready,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        }
    }

    /// A seated peer on team A (slot 0's own team).
    fn member(nickname: &str, ready: bool) -> o2jam_net::RoomMember {
        member_on_team(nickname, ready, 0)
    }

    /// Guest snapshot: our seat is 1 under a ready master at seat 0.
    /// Teams follow the server's first-unused-team rule (master A, guest
    /// B), so the team-balance gate passes on readiness alone.
    pub(super) fn guest_snapshot() -> o2jam_net::JoinedRoom {
        let mut room = joined_snapshot();
        room.member_id = 1;
        room.team = 1;
        room.user_count = 2;
        room.slots[0].member = Some(member("host", true));
        room.slots[1].member = Some(member_on_team("guest", false, 1));
        room
    }

    /// Game in Waiting with a staged joined snapshot, paired with a
    /// live fake gateway; the server end runs `with_server`. Stages
    /// the snapshot the Room Confirm join would have staged (that
    /// exchange is covered above) and marks the session joined, so
    /// Waiting Confirm means START/READY instead of the offline walk.
    pub(super) fn joined_waiting_session(
        room: o2jam_net::JoinedRoom,
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (port, server) = fake_gateway(with_server);
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.game.set_joined_room(room.clone());
        session.joined_room = Some(room);
        session.joined_waiting = true;
        session.game.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
        stage_armed_loading(session.game_mut());
        (session, server)
    }

    /// gateway; the server end runs `with_server`.
    fn playing_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (port, server) = fake_gateway(with_server);
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        // The next Confirm is the Loading→Playing gate (S7): arm it
        // the way the live lane stages song prep.
        stage_armed_loading(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        (OnlineSession::new(game, gateway), server)
    }

    /// One native-canvas paint through the session: game frame first,
    /// then the stats section (tick poll plus echo drain).
    pub(super) fn paint_frame(session: &mut OnlineSession) {
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        session.paint(&mut frame);
    }

    /// How long a drained-state assertion may wait for the fake
    /// gateway's pushes to land. Generous on purpose: the deadline
    /// only burns when the session genuinely never reaches the
    /// expected state, never on a loaded scheduler.
    const DRAIN_POLL_TIMEOUT: Duration = Duration::from_secs(5);

    /// Repaint until `condition` holds, instead of trusting one paint
    /// to absorb everything the fake gateway pushed. The session's
    /// drains read under a 1 ms quiet timeout, so under CPU
    /// contention a single paint can outrun the server thread's
    /// writes and observe an empty socket (issue #52); polling the
    /// drained state removes the wall-clock race without weakening
    /// the assertions that follow.
    pub(super) fn paint_until(
        session: &mut OnlineSession,
        condition: impl Fn(&OnlineSession) -> bool,
    ) {
        let deadline = std::time::Instant::now() + DRAIN_POLL_TIMEOUT;
        loop {
            paint_frame(session);
            if condition(session) {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "session never reached the expected state within {DRAIN_POLL_TIMEOUT:?}"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    /// Read one 4014 tick off the fake server's socket and decode it.
    fn read_tick(stream: &mut TcpStream) -> UpdateGameStats {
        let frame = read_frame(stream);
        assert_eq!(frame.id, UpdateGameStats::COMMAND);
        UpdateGameStats::decode_body(&frame.payload).unwrap()
    }

    /// Assert the fake server hears nothing for a beat (stats silence).
    fn assert_quiet(stream: &mut TcpStream) {
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        let mut header = [0u8; 4];
        assert!(stream.read_exact(&mut header).is_err());
    }

    #[test]
    fn flush_sends_pending_draft_as_3011() {
        let (mut session, server) = waiting_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 3011);
            assert_eq!(frame.payload, b"go".to_vec());
        });
        session.game.set_chat_input("go");
        assert!(session.flush_chat().unwrap());
        assert_eq!(session.game.chat_input(), "");
        server.join().unwrap();
    }

    #[test]
    fn flush_sends_nothing_while_the_draft_is_empty() {
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        assert!(!session.flush_chat().unwrap());
        server.join().unwrap();
    }

    #[test]
    fn wire_driven_scenes_keep_a_poll_heartbeat_while_planet_idles() {
        // Ticket #138: the lobby drains ride `paint` (chat on
        // Channel/Room, chat plus START on Waiting, tally on Result),
        // so the session owes a modest heartbeat there — while Planet
        // and Loading park the loop with no timer of their own.
        use o2jam_render::Frame;

        use super::SESSION_POLL_HEARTBEAT;
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(
            session.idle_repaint_interval(),
            Some(SESSION_POLL_HEARTBEAT)
        );
        assert!(session.game.enter_screen(Screen::Loading));
        assert_eq!(session.idle_repaint_interval(), None);
        assert!(session.game.enter_screen(Screen::Playing));
        assert_eq!(
            session.idle_repaint_interval(),
            Some(Duration::from_nanos(o2jam_shell::TICK_NANOS))
        );
        session.game.load_chart_holds(vec![(2, 5_000, 5_000, 7)]);
        session.game.strike_at(2, 5_000);
        let mut buf = vec![0u32; 64 * 64];
        session.game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 64,
            },
            7_000,
        );
        assert_eq!(session.game.screen(), Screen::Result);
        assert_eq!(
            session.idle_repaint_interval(),
            Some(SESSION_POLL_HEARTBEAT)
        );
        for screen in [Screen::Room, Screen::Channel] {
            assert!(session.game.enter_screen(screen));
            assert_eq!(
                session.idle_repaint_interval(),
                Some(SESSION_POLL_HEARTBEAT),
                "{screen:?} keeps its poll heartbeat"
            );
        }
        assert!(session.game.enter_screen(Screen::Planet));
        assert_eq!(session.idle_repaint_interval(), None);
        server.join().unwrap();
    }

    #[test]
    fn confirm_sends_the_draft_and_stays_waiting() {
        let (mut session, server) = waiting_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 3011);
            assert_eq!(frame.payload, b"hi".to_vec());
        });
        session.game.set_chat_input("hi");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(session.game.chat_input(), "");
    }

    #[test]
    fn confirm_without_a_draft_still_advances_to_loading() {
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Loading);
    }

    #[test]
    fn confirm_outside_waiting_sends_nothing() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(stream.read_exact(&mut header).is_err());
        });
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(ScreenGame::new(), gateway);
        session.game.set_chat_input("typed early");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.chat_input(), "typed early");
    }

    #[test]
    fn flush_channel_sends_pending_draft_as_2012() {
        let (mut session, server) = channel_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2012);
            assert_eq!(frame.payload, b"go".to_vec());
        });
        session.game.set_channel_input("go");
        assert!(session.flush_channel().unwrap());
        assert_eq!(session.game.channel_input(), "");
        server.join().unwrap();
    }

    #[test]
    fn flush_channel_sends_nothing_while_the_draft_is_empty() {
        let (mut session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        assert!(!session.flush_channel().unwrap());
        server.join().unwrap();
    }

    #[test]
    fn channel_confirm_sends_the_draft_and_stays_in_channel() {
        let (mut session, server) = channel_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2012);
            assert_eq!(frame.payload, b"hi".to_vec());
        });
        session.game.set_channel_input("hi");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Channel);
        assert_eq!(session.game.channel_input(), "");
    }

    /// Script the interactive channel-entry exchange (ticket #30):
    /// the 1004 login answers success, then the 2001 character answer
    /// the gate asserts.
    pub(super) fn answer_entry_login(stream: &mut TcpStream) {
        use o2jam_net::{ChannelLoginResponse, CharacterInfo, Gender};
        let login = read_frame(stream);
        assert_eq!(login.id, 1004);
        stream
            .write_all(
                &ChannelLoginResponse {
                    result_code: 0,
                    restricted: Some(false),
                }
                .encode(),
            )
            .unwrap();
        let fetch = read_frame(stream);
        assert_eq!(fetch.id, 2000);
        stream
            .write_all(
                &CharacterInfo {
                    nickname: "gate".to_owned(),
                    gender: Gender::Male,
                    level: 3,
                    ..Default::default()
                }
                .encode(),
            )
            .unwrap();
    }

    /// One listed channel row behind the interactive entry tests.
    #[test]
    fn shop_buy_round_trips_through_scripted_gateway() {
        use o2jam_net::frame::encode_frame;
        use o2jam_render::Rect;
        use o2jam_shell::avatar::PlacementRole;
        use o2jam_shell::avatar::ScenePlacement;
        use o2jam_shell::screens::side::SideScreen;
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 5000);
            let mut payload = vec![0u8; 152];
            payload[0..4].copy_from_slice(&626i32.to_le_bytes());
            payload[148..152].copy_from_slice(&2912127i32.to_le_bytes());
            stream.write_all(&encode_frame(5001, &payload)).unwrap();
        });

        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);

        game.set_shop_catalog(&o2jam_file::AvatarItemTable {
            rows: vec![o2jam_file::AvatarItemRow {
                item_id: 1,
                item_type: 0,
                price: 100,
                params: [0; 4],
                flags: (0, 0),
                name: "Good01".to_owned(),
                description: String::new(),
                sprites: Vec::new(),
            }],
        });
        let placement = ScenePlacement {
            id: String::new(),
            code: o2jam_shell::screens::side::SHOP_BUYITEM_CODE,
            sprite: None,
            rect: Rect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            },
            role: PlacementRole::Single,
        };
        game.load_side_scene(vec![placement], HashMap::new());
        assert!(game.visit_side(SideScreen::Shop));
        game.shop_cart_push(1);
        assert_eq!(game.shop_cart(), vec![1]);
        game.click_shop(1, 1);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.game.click_shop(1, 1);
        assert!(session.shop_buy_attempt.is_none());

        paint_frame(&mut session);

        paint_until(&mut session, |s| s.game.shop_gem_caption() == "GEM 626");
        server.join().unwrap();

        assert!(session.shop_buy_attempt.is_none());
        assert!(session.game.shop_cart().is_empty());
        assert_eq!(session.game.shop_gem_caption(), "GEM 626");
    }

    /// Insufficient-funds arm stays OPEN: no distinct 5001 failure
    /// reply is evidenced in `docs/shop-buy.md` (the 5001 handler
    /// returns wallet words on every reply, with no result code), so
    /// there is no separate failure shape to script. Probe bytes for
    /// the success reply used above: 152 B, first word `gem=626` LE,
    /// wallets zero, trailing word `2912127`.
    #[test]
    #[ignore = "OPEN #194: no distinct 5001 insufficient-funds reply evidenced; probe bytes 152B gem=626 tail=2912127"]
    fn shop_buy_insufficient_funds_arm_open() {}

    #[test]
    fn wallet_single_source_refreshes_shop_and_myroom_after_item_buy() {
        // Live caller chain:
        //   OnlineSession::paint (GameLoop, the windowed frame entry
        //   main.rs::run_play drives) -> poll_shop_buy -> drain_shop_buy
        //   -> ScreenGame::apply_shop_purchase_prefix -> WalletState::apply_purchase_prefix.
        //   Readouts: ScreenGame::shop_gem_caption + ScreenGame::myroom_money_captions.
        // Tier 2 scripted gateway. Synthetic catalogs only: no archives read (CI-safe).
        use o2jam_net::frame::encode_frame;
        use o2jam_render::Rect;
        use o2jam_shell::avatar::PlacementRole;
        use o2jam_shell::avatar::ScenePlacement;
        use o2jam_shell::screens::side::{SideScreen, MYROOM_O2CASH_CODE};
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 5000);
            let mut payload = vec![0u8; 152];
            payload[0..4].copy_from_slice(&626i32.to_le_bytes());
            payload[4..8].copy_from_slice(&7i32.to_le_bytes());
            payload[8..12].copy_from_slice(&450i32.to_le_bytes());
            payload[12..16].copy_from_slice(&30i32.to_le_bytes());
            payload[148..152].copy_from_slice(&2912127i32.to_le_bytes());
            stream.write_all(&encode_frame(5001, &payload)).unwrap();
        });

        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        // A staged character funds the wallet first: the post-buy
        // MyRoom caption must follow the reply (450), not this 900.
        game.set_myroom_character(o2jam_net::CharacterInfo {
            o2_cash: 900,
            ..o2jam_net::CharacterInfo::default()
        });

        game.set_shop_catalog(&o2jam_file::AvatarItemTable {
            rows: vec![o2jam_file::AvatarItemRow {
                item_id: 1,
                item_type: 0,
                price: 100,
                params: [0; 4],
                flags: (0, 0),
                name: "Good01".to_owned(),
                description: String::new(),
                sprites: Vec::new(),
            }],
        });
        let placement = ScenePlacement {
            id: String::new(),
            code: o2jam_shell::screens::side::SHOP_BUYITEM_CODE,
            sprite: None,
            rect: Rect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            },
            role: PlacementRole::Single,
        };
        game.load_side_scene(vec![placement], HashMap::new());
        assert!(game.visit_side(SideScreen::Shop));
        game.shop_cart_push(1);
        assert_eq!(game.shop_cart(), vec![1]);
        game.click_shop(1, 1);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.game.click_shop(1, 1);

        paint_frame(&mut session);

        paint_until(&mut session, |s| s.game.shop_gem_caption() == "GEM 626");
        server.join().unwrap();

        assert!(session.game.shop_cart().is_empty());
        assert_eq!(session.game.shop_gem_caption(), "GEM 626");
        let myroom_o2 = session
            .game
            .myroom_money_captions()
            .into_iter()
            .find(|(code, _)| *code == MYROOM_O2CASH_CODE)
            .map(|(_, text)| text);
        assert_eq!(myroom_o2.as_deref(), Some("450"));
    }

    #[test]
    fn music_buy_round_trips_through_scripted_gateway() {
        // Live caller chain:
        //   OnlineSession::paint (GameLoop, the windowed frame entry
        //   main.rs::run_play drives) -> flush_music_buy (5010 send)
        //   -> poll_music_buy -> drain_music_buy
        //   -> ScreenGame::apply_music_purchase_response
        //   -> MusicShopState::fold_purchased_songs (board)
        //   + WalletState::apply_music_purchase (wallet).
        //   Readouts: ScreenGame::shop_gem_caption + ScreenGame::myroom_money_captions
        //   + music_shop_visible_rows (OWNED) + ScreenGame::select_songs.
        // Tier 2 scripted gateway. Synthetic catalogs only: no archives read (CI-safe).
        use o2jam_net::shop::{MusicEntry, SyncMusicPurchaseResponse};
        use o2jam_render::Rect;
        use o2jam_shell::avatar::PlacementRole;
        use o2jam_shell::avatar::ScenePlacement;
        use o2jam_shell::screens::side::{
            SideScreen, MSHOP_CART_BUY_CODE, MSHOP_EX_FIRST_CODE, MYROOM_MUSICCASH_CODE,
        };
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 5010);
            let response = SyncMusicPurchaseResponse {
                gem: 1000,
                point: 250,
                o2_cash: 5000,
                music_cash: 120,
                item_cash: 340,
                cash_point: 56,
                music_list: vec![MusicEntry {
                    music_id: 757,
                    day: 14,
                    month: 3,
                    year: 25,
                    hour: 18,
                    minute: 45,
                }],
                gift_boxes_raw: Vec::new(),
                free_pass_extension_days: 7,
            };
            stream.write_all(&response.encode()).unwrap();
        });

        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);

        let blob = select_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        // Stage through the live-boot entry (stage_online_identity /
        // stage_session_catalog fan the same parse out to the select
        // table and both boards), never the shop-only setter.
        game.set_song_catalog(&entries);
        let area = |code: u32, x: i32, y: i32, w: u32, h: u32| ScenePlacement {
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
            role: PlacementRole::Single,
        };
        game.load_side_scene(
            vec![
                area(MSHOP_EX_FIRST_CODE, 18, 104, 353, 86),
                area(MSHOP_CART_BUY_CODE, 423, 528, 49, 23),
            ],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::MusicShop));
        game.click_music_shop(19, 105);
        assert_eq!(game.music_shop_cart(), vec![757]);
        game.click_music_shop(424, 529);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        paint_frame(&mut session);

        paint_until(&mut session, |s| s.game.shop_gem_caption() == "GEM 1000");
        server.join().unwrap();

        assert!(session.game.music_shop_cart().is_empty());
        assert_eq!(session.game.shop_gem_caption(), "GEM 1000");
        // The bought song joins the owned board (its shop row paints
        // OWNED) and stays in the song-select catalog.
        assert!(session
            .game
            .select_songs()
            .iter()
            .any(|song| song.song_id == 757));
        let owned: Vec<u32> = session
            .game
            .music_shop_visible_rows()
            .iter()
            .filter(|row| row.listed)
            .map(|row| row.song.song_id)
            .collect();
        assert_eq!(owned, vec![757]);
        let myroom_music = session
            .game
            .myroom_money_captions()
            .into_iter()
            .find(|(code, _)| *code == MYROOM_MUSICCASH_CODE)
            .map(|(_, text)| text);
        assert_eq!(myroom_music.as_deref(), Some("120"));
    }

    #[test]
    fn myroom_gift_list_fetch_round_trips_through_scripted_gateway() {
        // Live caller chain:
        //   OnlineSession::paint (GameLoop, the windowed frame entry
        //   main.rs::run_play drives) -> flush_gift_list (5017 send)
        //   -> poll_gift_list -> drain_gift_list (5018, exe-ignored body).
        // Tier 2 scripted gateway. Synthetic table + character only:
        // no archives read (CI-safe).
        use o2jam_net::frame::encode_frame;
        use o2jam_net::gift::GetGiftMessageListResponse;
        use o2jam_shell::screens::side::SideScreen;
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 5017);
            assert!(frame.payload.is_empty());
            let reply = GetGiftMessageListResponse {
                invalid: false,
                messages: Vec::new(),
            };
            stream
                .write_all(&encode_frame(5018, &reply.encode_body()))
                .unwrap();
        });

        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.set_myroom_items(o2jam_file::AvatarItemTable { rows: Vec::new() });
        game.set_myroom_character(o2jam_net::CharacterInfo::default());
        game.load_side_scene(Vec::new(), HashMap::new());
        assert!(game.visit_side(SideScreen::MyRoom));

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        assert!(session.gift_list_attempt.is_none());

        paint_frame(&mut session);
        paint_until(&mut session, |s| s.gift_list_attempt.is_none());
        server.join().unwrap();

        assert!(session.gift_list_attempt.is_none());
        assert!(session.gift_claim_attempt.is_none());
    }

    #[test]
    fn myroom_gift_claim_round_trips_and_lands_in_bag_through_scripted_gateway() {
        // Live caller chain:
        //   OnlineSession::paint (GameLoop, the windowed frame entry
        //   main.rs::run_play drives) -> flush_gift_claim (5023 send)
        //   -> poll_gift_claim -> drain_gift_claim (5024 Success)
        //   -> ScreenGame::apply_gift_claim_success (bag fold).
        // Tier 2 scripted gateway. Synthetic table + character only:
        // no archives read (CI-safe).
        use o2jam_net::frame::encode_frame;
        use o2jam_net::gift::{ClaimGift, ClaimGiftResponse};
        use o2jam_shell::screens::side::SideScreen;
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            use o2jam_net::gift::GetGiftMessageListResponse;
            let entry = read_frame(&mut stream);
            assert_eq!(entry.id, 5017);
            let empty = GetGiftMessageListResponse {
                invalid: false,
                messages: Vec::new(),
            };
            stream
                .write_all(&encode_frame(5018, &empty.encode_body()))
                .unwrap();
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 5023);
            let claim = ClaimGift::decode_body(&frame.payload).unwrap();
            assert_eq!(claim.gift_id, 7);
            assert_eq!(claim.resource_id, 1);
            let reply = ClaimGiftResponse {
                invalid: false,
                result_code: 0,
            };
            stream
                .write_all(&encode_frame(5024, &reply.encode_body()))
                .unwrap();
        });

        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        let cell = ScenePlacement {
            id: String::new(),
            code: o2jam_shell::screens::side::MYROOM_GIFT_FIRST_CODE,
            sprite: None,
            rect: Rect {
                x: 10,
                y: 10,
                w: 40,
                h: 20,
            },
            role: PlacementRole::Single,
        };
        let move_button = ScenePlacement {
            id: String::new(),
            code: o2jam_shell::screens::side::MYROOM_MOVE_CODE,
            sprite: None,
            rect: Rect {
                x: 100,
                y: 10,
                w: 40,
                h: 20,
            },
            role: PlacementRole::Single,
        };
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.set_myroom_items(o2jam_file::AvatarItemTable {
            rows: vec![o2jam_file::AvatarItemRow {
                item_id: 1,
                item_type: 2,
                price: 100,
                params: [0, 0, 0, 0],
                flags: (0, 0),
                name: "Gear1".to_owned(),
                description: String::new(),
                sprites: Vec::new(),
            }],
        });
        let mut hero = o2jam_net::CharacterInfo {
            nickname: "Tester".to_owned(),
            ..o2jam_net::CharacterInfo::default()
        };
        hero.item_gifts.push(o2jam_net::GiftItemInfo {
            gift_id: 7,
            item_id: 1,
            sender: "Giver".to_owned(),
        });
        game.set_myroom_character(hero);
        game.load_side_scene(vec![cell, move_button], HashMap::new());
        assert!(game.visit_side(SideScreen::MyRoom));
        game.click_myroom_side(11, 11);
        game.click_myroom_side(101, 11);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        paint_frame(&mut session);
        paint_until(&mut session, |s| {
            s.gift_claim_attempt.is_none()
                && s.game
                    .myroom_character()
                    .is_some_and(|hero| hero.inventory.contains(&1))
        });
        server.join().unwrap();

        let hero = session.game.myroom_character().expect("staged character");
        assert!(hero.item_gifts.is_empty(), "claimed gift leaves the box");
        assert!(hero.inventory.contains(&1), "claimed item lands in the bag");
        assert!(session.gift_claim_attempt.is_none());
    }

    pub(super) fn entry_channel() -> o2jam_net::ChannelState {
        o2jam_net::ChannelState {
            gateway_id: 0,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        }
    }

    #[test]
    fn channel_confirm_logs_in_and_stages_character() {
        // Ticket #30: Confirm on a channel row exchanges the 1004
        // login plus the 2001 identity fetch before advancing, so the
        // first Room paint already carries the staged identity.
        // Ticket #172: the exchange resolves on the per-frame poll —
        // Confirm only queues it, paints move it.
        let (mut session, server) = channel_session(|mut stream| {
            answer_entry_login(&mut stream);
        });
        session.game.set_channels(vec![entry_channel()]);
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
        let hero = session
            .game
            .myroom_character()
            .expect("interactive entry stages the fetched character info");
        assert_eq!(hero.nickname, "gate");
        assert_eq!(hero.level, 3);
    }

    #[test]
    fn channel_face_press_logs_in_and_stages_character() {
        // Ticket #30, live press path: the windowed client delivers a
        // press snapshot, so the enter-face press must exchange
        // before the Room paint like Confirm does. Ticket #172: the
        // press queues the attempt; the poll replays it on success.
        let (mut session, server) = channel_session(|mut stream| {
            answer_entry_login(&mut stream);
        });
        session.game.set_channels(vec![entry_channel()]);
        session.ingest_snapshot(&InputSnapshot::press_at(524, 246));
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
        let hero = session
            .game
            .myroom_character()
            .expect("face-press entry stages the fetched character info");
        assert_eq!(hero.nickname, "gate");
    }

    #[test]
    fn tt331_confirm_on_nonresident_channel_still_sends_login() {
        // The residency shortcut must stay row-specific: a different
        // row still logs in and stages the 2001 identity.
        let (mut session, server) = channel_session(|mut stream| {
            answer_entry_login(&mut stream);
        });
        let other_channel = o2jam_net::ChannelState {
            channel_id: 1,
            ..entry_channel()
        };
        session.game.set_channels(vec![entry_channel()]);
        session.set_resident_channel(other_channel);
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
        assert_eq!(
            session
                .game
                .myroom_character()
                .expect("non-resident Confirm stages the identity")
                .nickname,
            "gate"
        );
    }

    #[test]
    fn channel_confirm_stays_on_rejected_login() {
        // A rejected 1005 answers no identity fetch (no 2000 follows)
        // and keeps Channel, so the next Confirm retries the entry.
        use o2jam_net::ChannelLoginResponse;
        let (mut session, server) = channel_session(|mut stream| {
            let login = read_frame(&mut stream);
            assert_eq!(login.id, 1004);
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 7,
                        restricted: None,
                    }
                    .encode(),
                )
                .unwrap();
            assert_quiet(&mut stream);
        });
        session.game.set_channels(vec![entry_channel()]);
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Channel);
        assert!(session.game.myroom_character().is_none());
    }

    #[test]
    fn channel_login_without_reply_keeps_painting_until_late_reply() {
        // Ticket #172: the login must not block the frame loop while
        // the server stays quiet — paints continue on Channel, and a
        // late 1005 plus the 2001 identity still completes the entry.
        // The blocking exchange fails this: Confirm only returns once
        // the reply arrives, so the game is already in Room after it.
        use o2jam_net::{ChannelLoginResponse, CharacterInfo, Gender};
        let (mut session, server) = channel_session(|mut stream| {
            let login = read_frame(&mut stream);
            assert_eq!(login.id, 1004);
            // Half a second of server silence: the old code sat inside
            // the exchange for all of it; the pending login paints on.
            thread::sleep(Duration::from_millis(500));
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            let fetch = read_frame(&mut stream);
            assert_eq!(fetch.id, 2000);
            stream
                .write_all(
                    &CharacterInfo {
                        nickname: "gate".to_owned(),
                        gender: Gender::Male,
                        level: 3,
                        ..Default::default()
                    }
                    .encode(),
                )
                .unwrap();
        });
        session.game.set_channels(vec![entry_channel()]);
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        assert_eq!(session.game.screen(), Screen::Channel);
        assert!(session.game.myroom_character().is_none());
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        server.join().unwrap();
        let hero = session
            .game
            .myroom_character()
            .expect("late reply stages the fetched character info");
        assert_eq!(hero.nickname, "gate");
    }

    #[test]
    fn channel_login_no_reply_never_blocks_paint() {
        // Ticket #172, the freeze itself: a server that takes the
        // 1004 and never answers must not stall the frame loop — the
        // old blocking exchange sat in the read for the whole resting
        // timeout. The stonewall sleeps past it, detached: joining it
        // would park the test longer than the login window it proves.
        let (mut session, _server) = channel_session(|mut stream| {
            let login = read_frame(&mut stream);
            assert_eq!(login.id, 1004);
            thread::sleep(Duration::from_secs(12));
        });
        session.game.set_channels(vec![entry_channel()]);
        let started = std::time::Instant::now();
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..20 {
            paint_frame(&mut session);
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "twenty paints took {:?} with no server reply",
            started.elapsed()
        );
        assert_eq!(session.game.screen(), Screen::Channel);
    }

    #[test]
    fn channel_reentry_after_quick_back_paints_through() {
        // Ticket #172 repro: enter, quick back (the pinned 2021 leave
        // goes out, ticket #175), re-enter. The stonewalled second
        // 1004 must not freeze the game.
        let (mut session, _server) = channel_session(|mut stream| {
            answer_entry_login(&mut stream);
            // The pinned leave (empty 2021) with its probe-observed
            // `2022` answer, so the scripted session state stays
            // consistent with Memoryer's.
            let logout = read_frame(&mut stream);
            assert_eq!(logout.id, 2021);
            assert!(logout.payload.is_empty());
            stream
                .write_all(&encode_frame(2022, &0i32.to_le_bytes()))
                .unwrap();
            // The re-entry login: taken, never answered past the login
            // window, like Memoryer holding the stale session. Sleeps
            // past the resting timeout detached (see the no-reply
            // test): joining it would park the test.
            let second = read_frame(&mut stream);
            assert_eq!(second.id, 1004);
            thread::sleep(Duration::from_secs(12));
        });
        session.game.set_channels(vec![entry_channel()]);
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        // Quick back: Room Back returns to the channel list, sending
        // the pinned leave (the script consumes and answers it).
        session.menu_key(MenuKey::Back, true);
        assert_eq!(session.game.screen(), Screen::Channel);
        let started = std::time::Instant::now();
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "five re-entry paints took {:?} with no server reply",
            started.elapsed()
        );
        assert_eq!(session.game.screen(), Screen::Channel);
    }

    #[test]
    fn channel_back_sends_pinned_logout_and_reentry_completes() {
        // Ticket #175: the exe's channel exit sends `ChannelLogout`
        // 2021 (empty body — dispatch word 0x13200002, send at file
        // 0x48ff90), and Memoryer then answers the next 1004 (probe:
        // without the leave the second 1004 draws a 1001 -1 auth
        // error and never a 1005, while 2021 answers 2022 result 0
        // and the follow-up 1004 succeeds). The scripted gateway
        // replays that pinned order across the full
        // enter -> back -> re-enter sequence.
        let (mut session, server) = channel_session(|mut stream| {
            answer_entry_login(&mut stream);
            // Quick back: the pinned leave must hit the wire.
            let logout = read_frame(&mut stream);
            assert_eq!(logout.id, 2021);
            assert!(logout.payload.is_empty(), "pinned empty logout body");
            // The probe-observed leave answer: `i32` 0.
            stream
                .write_all(&encode_frame(2022, &0i32.to_le_bytes()))
                .unwrap();
            // Re-entry against the reset session completes.
            answer_entry_login(&mut stream);
        });
        session.game.set_channels(vec![entry_channel()]);
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        session.menu_key(MenuKey::Back, true);
        assert_eq!(session.game.screen(), Screen::Channel);
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
    }

    #[test]
    fn channel_login_expiry_clears_pending_and_ignores_late_reply() {
        // Ticket #172, the timeout leg: past the window the attempt
        // warns and clears (staying in Channel), so a reply that
        // arrives after the deadline stages nothing.
        use o2jam_net::ChannelLoginResponse;
        let (mut session, server) = channel_session(|mut stream| {
            let login = read_frame(&mut stream);
            assert_eq!(login.id, 1004);
            thread::sleep(Duration::from_millis(400));
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            // The cleared attempt never fetches: no 2000 follows, and
            // the drained 1005 answers nothing.
            assert_quiet(&mut stream);
        });
        session.game.set_channels(vec![entry_channel()]);
        session.menu_key(MenuKey::Confirm, true);
        paint_frame(&mut session);
        assert!(session.login_attempt.is_some());
        session
            .login_attempt
            .as_mut()
            .expect("login in flight")
            .deadline = std::time::Instant::now() - Duration::from_secs(1);
        paint_frame(&mut session);
        assert!(session.login_attempt.is_none());
        thread::sleep(Duration::from_millis(500));
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Channel);
        assert!(session.game.myroom_character().is_none());
    }

    #[test]
    fn room_join_without_reply_keeps_painting_until_late_reply() {
        // Ticket #172 for the Room Confirm join: the 3002 wait must
        // not block the frame loop either — paints continue in Room,
        // and the late 3003 still stages the snapshot and advances.
        use o2jam_net::{JoinRoom, JoinRoomResponse};
        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            thread::sleep(Duration::from_millis(400));
            let answered = JoinRoomResponse {
                result_code: 0,
                room: Some(joined_snapshot()),
            };
            stream.write_all(&answered.encode()).unwrap();
        });
        session.set_pending_room_join(3, "");
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        assert_eq!(session.game.screen(), Screen::Room);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Waiting
        });
        server.join().unwrap();
        assert!(session.game.is_room_master());
    }

    #[test]
    fn song_select_without_reply_keeps_painting_until_late_reply() {
        // Ticket #172 for the master Waiting select: the 4000/4002
        // echo wait must not block the frame loop — paints continue in
        // Waiting with the pick still pending, and the late 4003 plus
        // 4031 still stage it.
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            use o2jam_net::play::{MusicListAnswer, MusicListEntry, RoomArenaEcho};
            use o2jam_net::{Request, SetRoomArena, SetRoomMusic};
            let select = read_frame(&mut stream);
            assert_eq!(select.id, Request::SetRoomMusic.id());
            assert!(SetRoomMusic::decode_body(&select.payload).is_ok());
            stream.write_all(&encode_frame(4001, &[])).unwrap();
            let stage = read_frame(&mut stream);
            assert_eq!(stage.id, Request::SetRoomArena.id());
            assert!(SetRoomArena::decode_body(&stage.payload).is_ok());
            thread::sleep(Duration::from_millis(400));
            stream.write_all(&RoomArenaEcho::new(0).encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetMusicList.id());
            let listed = MusicListAnswer {
                entries: vec![MusicListEntry {
                    id: 757,
                    notes_ex: 100,
                    notes_nx: 200,
                    notes_hx: 300,
                }],
                free_music: true,
            };
            stream.write_all(&listed.encode()).unwrap();
        });
        session.set_pending_song(host_choice());
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        assert!(session.pending_song.is_some());
        assert_eq!(session.selected_song(), None);
        paint_until(&mut session, |session| session.selected_song().is_some());
        server.join().unwrap();
        assert_eq!(
            session.selected_song().map(|select| select.music_id),
            Some(757)
        );
    }

    #[test]
    fn bank_for_unnamed_song_stages_silent() {
        let (session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        assert!(session.bank_for_song(757, 2.9).is_empty());
        server.join().unwrap();
    }

    #[test]
    fn attach_with_no_bank_keeps_silent_play() {
        let (mut session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.attach_bank_for_song(757, 2.9);
        session.game.strike_at(2, 5_100);
        server.join().unwrap();
    }

    #[test]
    fn flush_whisper_sends_pending_draft_as_2017() {
        use o2jam_net::rooms::SendWhisper;
        let (mut session, server) = channel_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2017);
            let whisper = SendWhisper::decode_body(&frame.payload).unwrap();
            assert_eq!(whisper.recipient, "host");
            assert_eq!(whisper.content, "psst");
        });
        session.game.set_channel_input("psst");
        assert!(session.flush_channel_whisper("host").unwrap());
        assert_eq!(session.game.channel_input(), "");
        server.join().unwrap();
    }

    #[test]
    fn flush_whisper_sends_nothing_without_draft_or_recipient() {
        let (mut session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        assert!(!session.flush_channel_whisper("host").unwrap());
        session.game.set_channel_input("psst");
        assert!(!session.flush_channel_whisper("").unwrap());
        assert_eq!(session.game.channel_input(), "psst");
        server.join().unwrap();
    }

    #[test]
    fn channel_slash_whisper_sends_parsed_text_as_2017() {
        use o2jam_net::rooms::SendWhisper;
        let (mut session, server) = channel_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2017);
            let whisper = SendWhisper::decode_body(&frame.payload).unwrap();
            assert_eq!(whisper.recipient, "host");
            assert_eq!(whisper.content, "psst");
        });
        session.game.set_channel_input("/w host psst");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Channel);
        assert_eq!(session.game.channel_input(), "");
        assert_eq!(session.game.last_whisper_peer(), Some("host"));
    }

    #[test]
    fn waiting_slash_whisper_sends_parsed_text_as_2017() {
        use o2jam_net::rooms::SendWhisper;
        let (mut session, server) = waiting_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2017);
            let whisper = SendWhisper::decode_body(&frame.payload).unwrap();
            assert_eq!(whisper.recipient, "host");
            assert_eq!(whisper.content, "psst");
        });
        session.game.set_chat_input("/w host psst");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(session.game.chat_input(), "");
        assert_eq!(session.game.last_whisper_peer(), Some("host"));
    }

    #[test]
    fn slash_reply_answers_the_last_whisper_sender() {
        use o2jam_net::rooms::{ReceiveWhisper, SendWhisper};
        let (mut session, server) = channel_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2017);
            let whisper = SendWhisper::decode_body(&frame.payload).unwrap();
            assert_eq!(whisper.recipient, "host");
            assert_eq!(whisper.content, "hello");
        });
        session.game.push_channel_whisper(
            &ReceiveWhisper {
                sender: "host".to_owned(),
                content: "psst".to_owned(),
            },
            "me",
        );
        session.game.set_channel_input("/r hello");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.channel_input(), "");
    }

    #[test]
    fn slash_help_sends_nothing_and_clears_the_channel_draft() {
        let (mut session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.game.set_channel_input("/help");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Channel);
        assert_eq!(session.game.channel_input(), "");
    }

    #[test]
    fn slash_help_sends_nothing_and_clears_the_waiting_draft() {
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.game.set_chat_input("/h");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(session.game.chat_input(), "");
    }

    #[test]
    fn unknown_slash_command_falls_back_to_plain_chat() {
        let (mut session, server) = channel_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2012);
            assert_eq!(frame.payload, b"/dance party".to_vec());
        });
        session.game.set_channel_input("/dance party");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.channel_input(), "");
    }

    #[test]
    fn waiting_unknown_slash_command_falls_back_to_plain_chat() {
        let (mut session, server) = waiting_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 3011);
            assert_eq!(frame.payload, b"/dance party".to_vec());
        });
        session.game.set_chat_input("/dance party");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.chat_input(), "");
    }

    #[test]
    fn room_confirm_joins_pending_room_and_enters_waiting() {
        use o2jam_net::{JoinRoom, JoinRoomResponse};
        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            let asked = JoinRoom::decode_body(&frame.payload).unwrap();
            assert_eq!(asked.number, 3);
            assert!(asked.password.is_empty());
            let answered = JoinRoomResponse {
                result_code: 0,
                room: Some(joined_snapshot()),
            };
            stream.write_all(&answered.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        session.set_pending_room_join(3, "");
        assert!(!session.game.is_room_master());
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Waiting
        });
        server.join().unwrap();
        assert!(session.game.is_room_master());
        assert_eq!(session.game.room_key_mode(), 2);
    }

    #[test]
    fn ch190_scripted_gateway_inactive_row_refuses_face_enter_and_paints_dimmed() {
        // Ticket #190 seam guard (scripted gateway): a 1003 row the
        // gateway marks inactive must refuse entry end to end through
        // the real session wrapper, and paint with the dimmed ink.
        // Live caller chain:
        //   winit pointer down -> window.rs::route_input
        //   -> InputEdge::PointerDown -> OnlineSession::swallow_networked_edge
        //   -> ScreenGame::press_kind_at (Local on an inactive row)
        //   -> no begin_channel_login; ScreenGame::ingest_snapshot moves
        //      the highlight only
        //   paint -> ScreenGame::paint -> paint_channel_scene
        //   -> paint_channel_rows -> paint_channel_face (early return on
        //      !active) with dimmed ink (0x707070) on the row.
        use o2jam_net::ChannelState;
        use o2jam_render::{pack_xrgb, Frame};
        let (mut session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.game_mut().set_channels(vec![
            ChannelState {
                gateway_id: 0,
                channel_id: 2,
                capacity: 100,
                population: 5,
                active: true,
            },
            ChannelState {
                gateway_id: 0,
                channel_id: 9,
                capacity: 0,
                population: 0,
                active: false,
            },
        ]);
        // Inactive row's enter face (row 1) must not start a login.
        session.ingest_snapshot(&InputSnapshot::click_at(524, 297));
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        session.paint(&mut frame);
        assert_eq!(session.game.screen(), Screen::Channel);
        assert!(
            buf.contains(&pack_xrgb(0x70, 0x70, 0x70)),
            "inactive row must paint with the dimmed ink"
        );
        server.join().unwrap();
    }

    #[test]
    fn channel_click_logs_in_and_stages_character() {
        // The windowed live path routes canvas clicks through the
        // session wrapper: a channel face click must reach the inner
        // game exactly like keyboard Confirm — ticket #30 extends
        // that parity to the entry exchange (1004 login plus the 2001
        // identity fetch), so the first Room paint stages identity.
        use o2jam_net::ChannelState;
        let (mut session, server) = channel_session(|mut stream| {
            answer_entry_login(&mut stream);
        });
        session.game_mut().set_channels(vec![
            ChannelState {
                gateway_id: 0,
                channel_id: 2,
                capacity: 100,
                population: 5,
                active: true,
            },
            ChannelState {
                gateway_id: 0,
                channel_id: 9,
                capacity: 0,
                population: 0,
                active: false,
            },
        ]);
        // Ticket #56: only the enter face enters — row 0 face
        // (502,236)-(546,257). Ticket #172: the click queues the
        // attempt; the poll replays it on success.
        session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        server.join().unwrap();
        let hero = session
            .game
            .myroom_character()
            .expect("click entry stages the fetched character info");
        assert_eq!(hero.nickname, "gate");
    }

    #[test]
    fn scripted_gateway_channel_ids_move_the_row_label() {
        // Claim D25 (#92) evidence method without Wine: the scripted
        // gateway's 1003 channel_id rewrite (the proxy-recipe
        // equivalent on the session seam) must move the row label
        // the painter consumes. Same rows repaint a byte-identical
        // frame; ids moved underneath repaint a different one — the
        // value-source question as a pixel read, no rig required.
        use o2jam_net::ChannelState;
        let (mut session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        let row = |channel_id| ChannelState {
            gateway_id: 0,
            channel_id,
            capacity: 100,
            population: 5,
            active: true,
        };
        let painted = |session: &mut OnlineSession| {
            let mut buf = vec![0u32; 800 * 600];
            session.game_mut().paint(&mut o2jam_render::Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            });
            buf
        };
        session.game_mut().set_channels(vec![row(2)]);
        let before = painted(&mut session);
        assert_eq!(
            before,
            painted(&mut session),
            "same wire rows paint a stable frame"
        );
        // The proxy rewrite: same row, channel_id moved 2 -> 42.
        session.game_mut().set_channels(vec![row(42)]);
        let moved = painted(&mut session);
        assert_ne!(
            before, moved,
            "the row label must follow the wire channel_id (#92, D25)"
        );
        server.join().unwrap();
    }

    #[test]
    fn synthetic_2026_body_leaves_the_channel_promo_selection_to_the_carousel() {
        // Live caller chain: fake gateway S2C:2026 ->
        // OnlineSession::paint -> poll_lobby_chat (foreign-id skip) ->
        // ScreenGame::paint -> paint_channel_scene -> paint_planet_board
        // -> paint_planet_promo_frame. Neither 1003 nor the 2026
        // MusicPremiumTimeList carries a banner field; the exe's
        // carousel word `this+0x80` is the only selector (docs/planet.md).
        use o2jam_render::{pack_xrgb, Rgb555Image};
        use o2jam_shell::avatar::{AvatarFigure, FigureLayer, PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::PLANET_PROMO_IMAGE_CODE;

        let mut premium_body = Vec::new();
        premium_body.extend_from_slice(&1i32.to_le_bytes());
        premium_body.extend_from_slice(&757i32.to_le_bytes());
        premium_body.extend_from_slice(&3600i32.to_le_bytes());
        let (mut session, server) = channel_session(move |mut stream| {
            stream
                .write_all(&encode_frame(2026, &premium_body))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session
            .game_mut()
            .set_channels(vec![o2jam_net::ChannelState {
                gateway_id: 0,
                channel_id: 2,
                capacity: 100,
                population: 5,
                active: true,
            }]);
        // Live boot stages the art-less STATE_CHANNEL block, so the
        // staged channel painter (and the board beneath it) runs.
        session.game_mut().load_scene(
            Screen::Channel,
            Vec::new(),
            std::collections::HashMap::new(),
        );

        let sprite = "Planet_Promotion.ojs";
        let mut banner = AvatarFigure::default();
        for word in [0x7C00u16, 0x03E0, 0x001F] {
            banner.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.to_le_bytes().repeat(16), 4, 4).unwrap(),
                dx: 49,
                dy: 163,
                colorkey: 0xFFFF,
            });
        }
        session.game_mut().load_scene(
            Screen::Planet,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_PROMO_IMAGE_CODE,
                sprite: Some(sprite.to_owned()),
                rect: o2jam_render::Rect {
                    x: 49,
                    y: 163,
                    w: 701,
                    h: 333,
                },
            }],
            std::collections::HashMap::from([(sprite.to_owned(), banner)]),
        );

        let paint_frame = |session: &mut OnlineSession| {
            let mut buf = vec![0u32; 800 * 600];
            session.paint(&mut o2jam_render::Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            });
            buf[164 * 800 + 50]
        };
        assert_eq!(paint_frame(&mut session), pack_xrgb(0xFF, 0x00, 0x00));
        server.join().unwrap();
        session.game_mut().tick_planet_promo();
        assert_eq!(
            paint_frame(&mut session),
            pack_xrgb(0x00, 0xFF, 0x00),
            "the carousel index, not the wire body, moves the banner"
        );
    }

    #[test]
    fn room_enter_sends_the_draft_as_one_2012() {
        // Ticket #37: the room lobby chat is the main-room chat, so
        // Enter sends the same 2012 body the Channel scene sends.
        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 2012);
            assert_eq!(frame.payload, b"hello".to_vec());
            assert_quiet(&mut stream);
        });
        session.game_mut().set_lobby_input(LobbyChat::Room, "hello");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
        assert_eq!(session.game.lobby_chat(LobbyChat::Room).draft(), "");
    }

    #[test]
    fn room_enter_without_a_draft_advances_to_waiting() {
        let (mut session, server) = room_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    #[test]
    fn room_dialog_keeps_the_leftover_draft_out_of_the_enter() {
        let (mut session, server) = room_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.game_mut().open_create_room_dialog();
        session.game_mut().set_lobby_input(LobbyChat::Room, "hello");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.lobby_chat(LobbyChat::Room).draft(), "hello");
    }

    #[test]
    fn room_lobby_folds_inbound_2013_into_the_room_history() {
        use o2jam_net::rooms::MainRoomUserMessage;
        let line = MainRoomUserMessage {
            sender: "host".to_owned(),
            content: b"hello".to_vec(),
        };
        let (mut session, server) = room_session(move |mut stream| {
            stream.write_all(&line.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.room_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.room_chat().len(), 1);
        assert_eq!(session.game.room_chat()[0].sender, "host");
        assert_eq!(session.game.room_chat()[0].text, "hello");
        assert!(session.game.channel_chat().is_empty());
    }

    #[test]
    fn room_click_joins_numbered_room_and_enters_waiting() {
        // Clicks mirror Confirm: the clicked row's number joins first,
        // and the screen advances only on a staged snapshot.
        use o2jam_net::{JoinRoom, JoinRoomResponse, RoomListEntry};
        fn room_row(number: i32, title: &str) -> RoomListEntry {
            RoomListEntry {
                number,
                state_code: 1,
                title: title.to_owned(),
                has_password: false,
                music_id: 0,
                difficulty: 0,
                key_mode: 2,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 5,
                min_level: 0,
                max_level: 0,
                skills: vec![],
                premium: false,
                room_type: 0,
            }
        }
        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            let asked = JoinRoom::decode_body(&frame.payload).unwrap();
            assert_eq!(asked.number, 3);
            assert!(asked.password.is_empty());
            let answered = JoinRoomResponse {
                result_code: 0,
                room: Some(joined_snapshot()),
            };
            stream.write_all(&answered.encode()).unwrap();
        });
        session
            .game_mut()
            .set_rooms(vec![room_row(3, "Jam"), room_row(7, "Locked")]);
        // Ticket #172: the click queues the join; the poll replays
        // it once the snapshot stages.
        session.ingest_snapshot(&InputSnapshot::click_at(400, 154));
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Waiting
        });
        server.join().unwrap();
    }

    #[test]
    fn pending_join_skips_interleaved_broadcasts() {
        // Live servers interleave broadcasts (our own RoomCreated echo
        // included) ahead of the 3003 answer; the join must skip them
        // instead of failing the exchange.
        use o2jam_net::{JoinRoom, JoinRoomResponse};
        let (mut session, server) = room_session(|mut stream| {
            let broadcast = o2jam_net::frame::encode_frame(2007, &[]);
            stream.write_all(&broadcast).unwrap();
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            let answered = JoinRoomResponse {
                result_code: 0,
                room: Some(joined_snapshot()),
            };
            stream.write_all(&answered.encode()).unwrap();
        });
        session.set_pending_room_join(3, "");
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Waiting
        });
        server.join().unwrap();
    }

    #[test]
    fn room_confirm_rejected_stays_in_room() {
        use o2jam_net::{JoinRoom, JoinRoomResponse};
        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            let answered = JoinRoomResponse {
                result_code: 0xFFFF_FFFD,
                room: None,
            };
            stream.write_all(&answered.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        session.set_pending_room_join(3, "wrong");
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
        assert!(!session.game.is_room_master());
    }

    #[test]
    fn room_confirm_foreign_frame_stays_in_room() {
        use o2jam_net::{JoinRoom, MainRoomUserMessage};
        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            let stray = MainRoomUserMessage {
                sender: "host".to_owned(),
                content: b"hi".to_vec(),
            };
            stream.write_all(&stray.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        session.set_pending_room_join(3, "");
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
        assert!(!session.game.is_room_master());
    }

    #[test]
    fn room_confirm_without_pending_advances_locally() {
        let (mut session, server) = room_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert!(!session.game.is_room_master());
    }

    #[test]
    fn guest_confirm_sends_ready_toggle_and_stays_waiting() {
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            // The 4008 toggle rides an empty body (exe send sites push
            // a zero word then the opcode): both Confirms send bare
            // frames while the toggle flips underneath.
            let first = read_frame(&mut stream);
            assert_eq!(first.id, 4008);
            assert!(first.payload.is_empty());
            let second = read_frame(&mut stream);
            assert_eq!(second.id, 4008);
            assert!(second.payload.is_empty());
            assert_quiet(&mut stream);
        });
        assert!(!session.game.is_room_master());
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    #[test]
    fn guest_chat_draft_sends_chat_not_ready() {
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 3011);
            assert_eq!(frame.payload, b"hi".to_vec());
            assert_quiet(&mut stream);
        });
        session.game.set_chat_input("hi");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(session.game.chat_input(), "");
    }

    #[test]
    fn master_blocked_until_every_peer_ready() {
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", false, 1));
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            // The blocked first Confirm sends nothing: the first
            // frame on the wire is the gated START below.
            let start = read_frame(&mut stream);
            assert_eq!(start.id, 4010);
            assert!(start.payload.is_empty());
            assert_quiet(&mut stream);
        });
        assert!(session.game.is_room_master());
        assert!(!session.game.can_start_game());
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
        let mut ready_room = joined_snapshot();
        ready_room.slots[1].member = Some(member_on_team("peer", true, 1));
        session.game.set_joined_room(ready_room);
        assert!(session.game.can_start_game());
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    /// Joined master room with one ready peer and a staged room song
    /// (`joined_snapshot` has neither): the shape a `3003` answer gives
    /// a host waiting to start.
    fn startable_room() -> o2jam_net::JoinedRoom {
        let mut room = joined_snapshot();
        room.music_id = 757;
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        room
    }

    #[test]
    fn no_music_member_blocks_start_and_kicks_the_room_song_download() {
        // docs/music-availability.md §4, live chain: `main.rs::run_play`
        // -> `assemble_online_session` -> `OnlineSession::paint`
        // (Waiting) -> `poll_waiting_chat` -> `drain_waiting_chat` ->
        // `apply_acquire_music` -> `answer_acquire_music` ->
        // `kick_room_song_download` -> `ScreenGame::begin_music_download`.
        //
        // The START press runs the gate (VA `0x4b5560`): a peer whose
        // seat reports NoMusic answers 4034 AcquireMusicRequest instead
        // of 4010, the 4035 answer hands the room its states, and the
        // local member enqueues the room song instead of starting. Once
        // the peer reports Ready (4037), the master's retry arm sends
        // 4010 and the server's Success broadcast reaches Loading.
        let mut peer = member_on_team("peer", true, 1);
        peer.music_state = MusicState::NoMusic.code();
        let mut room = startable_room();
        room.slots[1].member = Some(peer);
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            let acquire = read_frame(&mut stream);
            assert_eq!(acquire.id, Request::AcquireMusicRequest.id());
            assert!(acquire.payload.is_empty());
            let states = AcquireMusic {
                members: vec![
                    MemberMusicState {
                        member_id: 0,
                        state: MusicState::Downloading,
                    },
                    MemberMusicState {
                        member_id: 1,
                        state: MusicState::NoMusic,
                    },
                ],
            };
            stream.write_all(&states.encode()).unwrap();
            // A member still lacks the music: no 4010 on this arm.
            assert_quiet(&mut stream);
            let ready = SyncMemberMusicStateEvent {
                member_id: 1,
                state: MusicState::Ready,
            };
            stream.write_all(&ready.encode()).unwrap();
            let start = read_frame(&mut stream);
            assert_eq!(start.id, 4010);
            assert!(start.payload.is_empty());
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
        for _ in 0..50 {
            if session.game.music_download().is_running() {
                break;
            }
            paint_frame(&mut session);
        }
        assert!(
            session.game.music_download().is_running(),
            "the 4035 answer kicks the room song download"
        );
        assert_eq!(session.game.music_download().requests().len(), 1);
        assert_eq!(session.game.screen(), Screen::Waiting);
        for _ in 0..50 {
            if session.game.screen() == Screen::Loading {
                break;
            }
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Loading);
    }

    #[test]
    fn a_promoted_song_download_reports_progress_completion_and_becomes_ready() {
        // docs/music-availability.md §4, live chain: `main.rs::run_play`
        // -> `assemble_online_session` -> `OnlineSession::paint` (Waiting)
        // -> `poll_music_downloads` -> `ScreenGame::pump_music_downloads`
        // -> `MusicDownload::pump` (Temp promote) -> `4038`/`5012`.
        // The source is a synthetic local mirror; no game data or FTP
        // network is read, and both folders are throwaway.
        let root = std::env::temp_dir().join("recomp2jam-download-session");
        let source = root.join("source");
        let music = root.join("Music");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&source).expect("source dir");
        std::fs::create_dir_all(&music).expect("music dir");
        std::fs::write(source.join("o2ma757.ojn"), b"synthetic").expect("source chart");

        let (mut session, server) = joined_waiting_session(startable_room(), |mut stream| {
            let progress = read_frame(&mut stream);
            assert_eq!(progress.id, Request::SetDownloadProgress.id());
            assert_eq!(progress.payload, [100, 0]);
            let done = read_frame(&mut stream);
            assert_eq!(done.id, Request::SyncMusicDownload.id());
            assert_eq!(done.payload, 757u16.to_le_bytes());
            assert_quiet(&mut stream);
        });
        session
            .game
            .set_music_download_dirs(music.clone(), Some(source));
        session
            .game
            .set_installed_music(Some(o2jam_file::installed_music(Vec::<String>::new())));
        assert!(session.game.begin_music_download(757));
        assert_eq!(session.game.music_state_for(757), MusicState::NoMusic);
        assert!(!music.join("o2ma757.ojn").exists());
        for _ in 0..50 {
            if !session.game.music_download().is_running() {
                break;
            }
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert!(music.join("o2ma757.ojn").exists(), "chart promoted");
        assert!(!music.join("Temp").join("o2ma757.ojn").exists());
        assert_eq!(session.game.music_state_for(757), MusicState::Ready);
        assert_eq!(session.game.screen(), Screen::Waiting);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_refused_start_broadcast_keeps_the_room_in_waiting() {
        // §4: the server answers `StartResult::None` (-1) while a member
        // lacks the music and starts nothing. The result word decides —
        // our Loading must wait for a Success.
        let (mut session, server) = joined_waiting_session(startable_room(), |mut stream| {
            let start = read_frame(&mut stream);
            assert_eq!(start.id, 4010);
            stream
                .write_all(&encode_frame(4011, &(-1i32).to_le_bytes()))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..50 {
            if session.game.waiting_notice().is_some() {
                break;
            }
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(
            session.game.waiting_notice(),
            Some(o2jam_shell::screens::game::WAITING_START_DOWNLOAD_NOTICE),
            "the refusal word was read and the download notice stands"
        );
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    #[test]
    fn the_room_song_change_announces_the_music_state() {
        // §2/§4, live chain: `main.rs::run_play` -> `OnlineSession::paint`
        // (Waiting) -> `poll_waiting_chat` -> `drain_waiting_chat` ->
        // `apply_waiting_music_changed` -> `announce_music_state`.
        //
        // The exe's 4001 arm (VA `0x4ab591`) restages the room song and
        // sends 4025 for it. An install scan that lists no chart file
        // answers NoMusic — the honest byte, never the old blind Ready.
        let (mut session, server) = joined_waiting_session(startable_room(), |mut stream| {
            let changed = WaitingMusicChanged {
                music_id: 900,
                difficulty: 1,
                speed: 3,
            };
            stream.write_all(&changed.encode()).unwrap();
            let state = read_frame(&mut stream);
            assert_eq!(state.id, Request::UpdateMusicState.id());
            assert_eq!(state.payload, vec![MusicState::NoMusic.code()]);
            assert_quiet(&mut stream);
        });
        session.set_installed_music(Some(o2jam_file::installed_music(Vec::<String>::new())));
        for _ in 0..30 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.room_music_id(), Some(900));
    }

    #[test]
    fn a_member_leaving_the_game_and_a_skill_change_reannounce_the_music_state() {
        // §2: the exe's 4022 (VA `0x4ab658`), 4024 (`0x4ab90c`) and 4058
        // (`0x4b17dc`) arms all end in the same 4025 send — the room
        // re-announces after a member leaves the game and after either
        // skill change. Live chain: `OnlineSession::paint` (Waiting) ->
        // `poll_waiting_chat` -> `drain_waiting_chat` ->
        // `announces_room_music_state` -> `announce_music_state`.
        use o2jam_net::Event;
        let (mut session, server) = joined_waiting_session(startable_room(), |mut stream| {
            for event in [
                Event::UserLeaveGame,
                Event::WaitingSkillChanged,
                Event::WaitingSkillExChanged,
            ] {
                stream.write_all(&encode_frame(event.id(), &[0])).unwrap();
                let state = read_frame(&mut stream);
                assert_eq!(state.id, Request::UpdateMusicState.id());
                assert_eq!(state.payload, vec![MusicState::Ready.code()]);
            }
            assert_quiet(&mut stream);
        });
        for _ in 0..30 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.room_music_id(), Some(757));
    }

    #[test]
    fn start_broadcast_advances_host_to_loading() {
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            let start = read_frame(&mut stream);
            assert_eq!(start.id, 4010);
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
        for _ in 0..50 {
            if session.game.screen() == Screen::Loading {
                break;
            }
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Loading);
    }

    #[test]
    fn guest_start_broadcast_advances_to_loading() {
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        assert!(!session.game.is_room_master());
        // No Confirm is pressed: the START gate passes masters
        // only, so the guest reaches Loading on the broadcast
        // through the force-advance path.
        for _ in 0..50 {
            if session.game.screen() == Screen::Loading {
                break;
            }
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Loading);
    }

    /// Matched #347 start shape: a joined guest receives the host's
    /// 4011 broadcast while Waiting, with no local Confirm after it.
    /// Live caller chain: `run_online` -> `assemble_online_session` ->
    /// `run_windowed_session` -> `Shell::run` -> `RedrawRequested` ->
    /// `Shell::paint_backbuffer` -> `OnlineSession::paint` ->
    /// `ScreenGame::paint` -> `paint_loading_scene_with_status`.
    /// This seam uses a fake 4011 and a synthetic cover figure because
    /// it pins the same state->paint handoff without a Wine rig; the
    /// live pixel gate is the #346 A/B pack re-run.
    #[test]
    fn guest_start_broadcast_paints_loading_art_on_the_transition_frame() {
        use o2jam_render::{pack_xrgb, Rgb555Image};
        use o2jam_shell::avatar::{AvatarFigure, FigureLayer, LOADING_COVER_SPRITE};

        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        assert_eq!(session.game.screen(), Screen::Waiting);
        let mut cover = AvatarFigure::default();
        cover.push_layer(FigureLayer {
            image: Rgb555Image::decode(&0x7C00u16.to_le_bytes(), 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        session.game_mut().merge_scene_figures(
            Screen::Loading,
            std::collections::HashMap::from([(LOADING_COVER_SPRITE.to_owned(), cover)]),
        );

        let mut buf = vec![0u32; 800 * 600];
        let mut frame = o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        session.paint(&mut frame);

        assert_eq!(session.game.screen(), Screen::Loading);
        assert_eq!(
            buf[0],
            pack_xrgb(0xFF, 0x00, 0x00),
            "transition frame holds Loading art"
        );
        server.join().unwrap();
    }

    #[test]
    fn loading_confirm_sends_4012_then_enters_playing() {
        use o2jam_net::{ConfirmMusicLoaded, Request};
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            let start = read_frame(&mut stream);
            assert_eq!(start.id, 4010);
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            let loaded = read_frame(&mut stream);
            assert_eq!(loaded.id, Request::ConfirmMusicLoaded.id());
            assert_eq!(
                ConfirmMusicLoaded::decode_body(&loaded.payload).unwrap(),
                ConfirmMusicLoaded {
                    power_skill_id: 0,
                    speed: 1,
                }
            );
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..50 {
            if session.game.screen() == Screen::Loading {
                break;
            }
            paint_frame(&mut session);
        }
        assert_eq!(session.game.screen(), Screen::Loading);
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Playing);
    }

    #[test]
    fn loading_confirm_offline_sends_nothing() {
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Loading);
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Playing);
    }

    #[test]
    fn guest_loading_confirm_sends_room_speed_as_4012() {
        use o2jam_net::{ConfirmMusicLoaded, Request};
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            let loaded = read_frame(&mut stream);
            assert_eq!(loaded.id, Request::ConfirmMusicLoaded.id());
            assert_eq!(
                ConfirmMusicLoaded::decode_body(&loaded.payload).unwrap(),
                ConfirmMusicLoaded {
                    power_skill_id: 0,
                    speed: 1,
                }
            );
            assert_quiet(&mut stream);
        });
        for _ in 0..50 {
            if session.game.screen() == Screen::Loading {
                break;
            }
            paint_frame(&mut session);
        }
        assert_eq!(session.game.screen(), Screen::Loading);
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Playing);
    }

    /// Walk a joined host from Waiting to Result: START, 4011,
    /// Loading Confirm (4012), one struck note, then the finished
    /// chart auto-advances to Result on an explicit clock.
    fn play_one_note_to_result(session: &mut OnlineSession) {
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..50 {
            if session.game.screen() == Screen::Loading {
                break;
            }
            paint_frame(session);
        }
        assert_eq!(session.game.screen(), Screen::Loading);
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Playing);
        session.game.load_chart(vec![(0, 1000, 7)]);
        session.game.strike_at(0, 1000);
        let mut buf = vec![0u32; 800 * 600];
        session.game.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            1_000_000,
        );
        assert_eq!(session.game.screen(), Screen::Result);
    }

    #[test]
    fn result_entry_submits_live_tally_then_finalizes_and_exits() {
        use o2jam_net::{ConfirmMusicLoaded, Request, SubmitScore};
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            assert_eq!(read_frame(&mut stream).id, 4010);
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            let loaded = read_frame(&mut stream);
            assert_eq!(loaded.id, Request::ConfirmMusicLoaded.id());
            assert_eq!(
                ConfirmMusicLoaded::decode_body(&loaded.payload).unwrap(),
                ConfirmMusicLoaded {
                    power_skill_id: 0,
                    speed: 1,
                }
            );
            let tally = read_frame(&mut stream);
            assert_eq!(tally.id, Request::SubmitScore.id());
            assert_eq!(
                SubmitScore::decode_body(&tally.payload).unwrap(),
                SubmitScore {
                    cool: 1,
                    good: 0,
                    bad: 0,
                    miss: 0,
                    max_combo: 1,
                    jam: 0,
                    max_jam: 0,
                    score: 200,
                    life: 255,
                    speed: 1,
                    long_note_score: 0,
                }
            );
            assert_eq!(read_frame(&mut stream).id, Request::FinalizeRank.id());
            assert_eq!(read_frame(&mut stream).id, Request::ExitPlaying.id());
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_frame(&mut session);
        paint_frame(&mut session);
        server.join().unwrap();
        // One triple total (the second paint resends nothing), and
        // the shell still owns the retry edge back to Waiting.
        assert_eq!(session.game.screen(), Screen::Result);
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    #[test]
    fn result_drain_folds_server_totals_into_battle_verdict() {
        use o2jam_net::{GameRankUpdate, MatchResult, ScoreCompleted, ScoreEntry, ScoreSubmission};
        let mut room = joined_snapshot();
        room.slots[0].member = Some(member("host", true));
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        room.slots[1].member.as_mut().unwrap().team = 1;
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            assert_eq!(read_frame(&mut stream).id, 4010);
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::ConfirmMusicLoaded.id()
            );
            loop {
                if read_frame(&mut stream).id == o2jam_net::Request::SubmitScore.id() {
                    break;
                }
            }
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::FinalizeRank.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::ExitPlaying.id()
            );
            // Tally answers: our 200 stands, the peer's 50000 takes
            // team 1 past us — server-authoritative, like the live
            // gateway's 4017/4018 pair.
            let submitted = ScoreSubmission { member_id: 0 };
            stream.write_all(&submitted.encode()).unwrap();
            let completed = ScoreCompleted {
                room_master: 1,
                scores: vec![
                    ScoreEntry {
                        member_id: 1,
                        active: true,
                        cool: 50,
                        good: 0,
                        bad: 0,
                        miss: 0,
                        max_combo: 200,
                        jam: 9,
                        score: 50_000,
                        reward: 300,
                        level: 3,
                        experience: 2000,
                        result: MatchResult::Win,
                        gem: 11000,
                        cash_point: 0,
                        speed: 1,
                        long_note_score: 60,
                    },
                    ScoreEntry {
                        member_id: 0,
                        active: true,
                        cool: 1,
                        good: 0,
                        bad: 0,
                        miss: 0,
                        max_combo: 1,
                        jam: 0,
                        score: 200,
                        reward: 90,
                        level: 2,
                        experience: 1911,
                        result: MatchResult::Lose,
                        gem: 10133,
                        cash_point: 0,
                        speed: 1,
                        long_note_score: 0,
                    },
                ],
            };
            stream.write_all(&completed.encode()).unwrap();
            let ranks = GameRankUpdate {
                member_ranks: [1, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            };
            stream.write_all(&ranks.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_until(&mut session, |s| {
            s.game.battle_verdict() == Some(o2jam_shell::screens::paint::BattleVerdict::Lose)
        });
        server.join().unwrap();
        // Local-only staging said Win (200 vs empty); the 4018 table
        // restages both seats, so our team-0 total loses to team 1.
        assert_eq!(session.game.screen(), Screen::Result);
        assert_eq!(
            session.game.battle_verdict(),
            Some(o2jam_shell::screens::paint::BattleVerdict::Lose)
        );
    }

    /// Live caller chain: `main.rs::run_play` → `setup_online` →
    /// `run_windowed_session` → `OnlineSession::paint` → `poll_tally` →
    /// `drain_tally_answers` → `apply_score_completed` →
    /// `ResultMemberRecord::from_score_entry` → this test's read-back.
    /// A scripted gateway sends one active and one inactive 4018 row;
    /// the exe admits only `active == 1` (file `0x15fd8e`).
    #[test]
    fn result_drain_stages_4018_rows_into_exe_record_fields() {
        use o2jam_net::{MatchResult, ScoreCompleted, ScoreEntry};

        let mut room = joined_snapshot();
        room.slots[0].member = Some(member("host", true));
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            assert_eq!(read_frame(&mut stream).id, 4010);
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::ConfirmMusicLoaded.id()
            );
            loop {
                if read_frame(&mut stream).id == o2jam_net::Request::SubmitScore.id() {
                    break;
                }
            }
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::FinalizeRank.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::ExitPlaying.id()
            );
            let completed = ScoreCompleted {
                room_master: 0,
                scores: vec![
                    ScoreEntry {
                        member_id: 0,
                        active: true,
                        cool: 11,
                        good: 22,
                        bad: 33,
                        miss: 44,
                        max_combo: 55,
                        jam: 66,
                        score: 777,
                        reward: 88,
                        level: 3,
                        experience: 2000,
                        result: MatchResult::Win,
                        gem: 11_000,
                        cash_point: 125,
                        speed: 2,
                        long_note_score: 60,
                    },
                    ScoreEntry {
                        member_id: 2,
                        active: false,
                        cool: 99,
                        good: 99,
                        bad: 99,
                        miss: 99,
                        max_combo: 99,
                        jam: 99,
                        score: 99_999,
                        reward: 99,
                        level: 9,
                        experience: 9,
                        result: MatchResult::Lose,
                        gem: 9,
                        cash_point: 9,
                        speed: 9,
                        long_note_score: 9,
                    },
                ],
            };
            stream.write_all(&completed.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_until(&mut session, |session| session.result_record(0).is_some());
        server.join().unwrap();

        let expected = ResultMemberRecord {
            cool: 11,
            good: 22,
            bad: 33,
            miss: 44,
            max_combo: 55,
            jam_combo: 66,
            long_note_score: 60,
            score: 777,
            reward: 88,
            result: MatchResult::Win,
            speed: 2,
            level: 3,
            experience: 2000,
            gem: 11_000,
            cash_point: 125,
            room_master: true,
        };
        assert_eq!(session.result_record(0), Some(&expected));
        assert_eq!(session.result_record(2), None);
    }

    /// Wire-shape replay for the result input arms (#87, wired in
    /// #88): the scripted gateway drives the real session bridge to
    /// the Result boundary and captures every frame the entry drain
    /// emits, then the retry press lands inside the gateway's read
    /// window so the replay records the pinned send shape: retry
    /// queues `StartGame` 4010 (`0xfaa`, empty) through the session
    /// bridge. The three exe-named arms are pinned as exact wire
    /// bytes — back = `ExitPlaying` 4021 (`0xfb5`, empty — the frame
    /// the entry drain already emits), battle entry = `GetLiveState`
    /// 4042 (`0xfca`, empty).
    #[test]
    fn result_wire_replay_names_the_retry_back_and_battle_entry_shapes() {
        use o2jam_net::relay::GetLiveState;
        use o2jam_net::{ExitPlaying, StartGame};

        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::StartGame.id()
            );
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::ConfirmMusicLoaded.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::SubmitScore.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::FinalizeRank.id()
            );
            // Result entry already emits the back arm's frame: the
            // exe's result BACK posts `0xfb5` (`ExitPlaying` 4021,
            // `docs/cresult-anchors.md` `ev_77ce98f1…`) and the
            // bridge sends the same empty frame at the song-end drain.
            let exit = read_frame(&mut stream);
            assert_eq!(exit.id, ExitPlaying::COMMAND);
            assert!(exit.payload.is_empty());
            // The retry press arrives inside this window: #88 wires
            // the pinned body, so the quiet assertion #87 recorded
            // becomes the `StartGame` send read.
            let retry = read_frame(&mut stream);
            assert_eq!(retry.id, StartGame::COMMAND);
            assert!(retry.payload.is_empty());
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_frame(&mut session);
        assert_eq!(session.game.screen(), Screen::Result);
        // Retry press: the exe posts relay event `0xfaa` here
        // (`FUN_004c2f00` range, `ev_77ce98f1…`); the shell queues
        // the pinned `StartGame` body and the local arm owns the
        // edge back to Waiting. The bridge sends on the next paint.
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
        paint_frame(&mut session);
        server.join().unwrap();
        // The pinned wire shapes, whole frames: 4010 live-observed
        // empty from the exe (`docs/wine-boot.md`), 4042 byte-verified
        // empty (`docs/relay.md` §10, sole `0x0FCA` push-immediate at
        // file `0xc365c`, gated on the battle global `0x7789e6`), 4021
        // empty both directions (Memoryer reads nothing; the playtest
        // walk's `0FB5` left game was accepted live).
        for expected in [
            (&StartGame.encode()[..], &[0x04, 0x00, 0xAA, 0x0F][..]),
            (&ExitPlaying.encode()[..], &[0x04, 0x00, 0xB5, 0x0F][..]),
            (&GetLiveState.encode()[..], &[0x04, 0x00, 0xCA, 0x0F][..]),
        ] {
            assert_eq!(expected.0, expected.1);
            let frame = decode_frame(expected.0).unwrap();
            assert!(frame.payload.is_empty(), "pinned body stays empty");
        }
    }

    #[test]
    fn result_back_press_sends_the_pinned_exit_playing_body() {
        // The BACK press queues the same pinned empty `ExitPlaying`
        // 4021 (`0xfb5`) the entry drain already sends: the gateway
        // reads the drain's frame first, the press's second.
        use o2jam_net::ExitPlaying;
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::StartGame.id()
            );
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::ConfirmMusicLoaded.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::SubmitScore.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::FinalizeRank.id()
            );
            let drain_exit = read_frame(&mut stream);
            assert_eq!(drain_exit.id, ExitPlaying::COMMAND);
            assert!(drain_exit.payload.is_empty());
            let press_exit = read_frame(&mut stream);
            assert_eq!(press_exit.id, ExitPlaying::COMMAND);
            assert!(press_exit.payload.is_empty());
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_frame(&mut session);
        assert_eq!(session.game.screen(), Screen::Result);
        session.menu_key(MenuKey::Back, true);
        assert_eq!(session.game.screen(), Screen::Room);
        paint_frame(&mut session);
        server.join().unwrap();
    }

    #[test]
    fn guest_tally_stages_winning_teams_and_leaves_to_room() {
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            let loaded = read_frame(&mut stream);
            assert_eq!(loaded.id, o2jam_net::Request::ConfirmMusicLoaded.id());
            let tally = read_frame(&mut stream);
            assert_eq!(tally.id, o2jam_net::Request::SubmitScore.id());
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::FinalizeRank.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::ExitPlaying.id()
            );
            assert_quiet(&mut stream);
        });
        for _ in 0..50 {
            if session.game.screen() == Screen::Loading {
                break;
            }
            paint_frame(&mut session);
        }
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Playing);
        session.game.load_chart(vec![(0, 1000, 7)]);
        session.game.strike_at(0, 1000);
        let mut buf = vec![0u32; 800 * 600];
        session.game.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            1_000_000,
        );
        paint_frame(&mut session);
        // Both seats ride team 0 with only the local total staged,
        // so the claimed local team takes rank 1.
        assert_eq!(
            session.game.battle_verdict(),
            Some(o2jam_shell::screens::paint::BattleVerdict::Win)
        );
        // The shell still owns the leave edge back to Room.
        session.menu_key(MenuKey::Back, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
    }

    #[test]
    fn waiting_user_message_drains_into_waiting_history() {
        use o2jam_net::rooms::WaitingUserMessage;
        let line = WaitingUserMessage {
            sender: "host".to_owned(),
            content: b"hello".to_vec(),
            member_id: 1,
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            stream.write_all(&line.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.waiting_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.waiting_chat().len(), 1);
        assert_eq!(session.game.waiting_chat()[0].sender, "host");
        assert_eq!(session.game.waiting_chat()[0].text, "hello");
        assert!(!session.game.waiting_chat()[0].admin);
    }

    #[test]
    fn waiting_admin_message_drains_into_waiting_history() {
        use o2jam_net::rooms::WaitingAdminMessage;
        let line = WaitingAdminMessage {
            sender: "op".to_owned(),
            content: b"quiet".to_vec(),
            member_id: 0,
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            stream.write_all(&line.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.waiting_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.waiting_chat().len(), 1);
        assert_eq!(session.game.waiting_chat()[0].sender, "op");
        assert_eq!(session.game.waiting_chat()[0].text, "quiet");
        assert!(session.game.waiting_chat()[0].admin);
    }

    #[test]
    fn undecodable_waiting_line_drops_and_drain_continues() {
        use o2jam_net::rooms::{WaitingAdminMessage, WaitingUserMessage};
        let line = WaitingAdminMessage {
            sender: "op".to_owned(),
            content: b"quiet".to_vec(),
            member_id: 0,
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            stream
                .write_all(&encode_frame(WaitingUserMessage::COMMAND, &[]))
                .unwrap();
            stream.write_all(&line.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.waiting_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.waiting_chat().len(), 1);
        assert_eq!(session.game.waiting_chat()[0].text, "quiet");
        assert!(session.game.waiting_chat()[0].admin);
    }

    #[test]
    fn slot_update_drains_into_the_slot_mirror() {
        use o2jam_net::{RoomSlotAction, RoomSlotUpdate};
        let lock = RoomSlotUpdate {
            index: 2,
            action_code: 2,
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            stream.write_all(&lock.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| {
            s.slot_action(2) == Some(RoomSlotAction::SlotLocked)
        });
        server.join().unwrap();
        assert_eq!(session.slot_action(2), Some(RoomSlotAction::SlotLocked));
        assert_eq!(session.slot_action(1), None);
    }

    #[test]
    fn ready_echo_drains_into_the_ready_mirror() {
        use o2jam_net::MemberReadyStateChanged;
        let echo = MemberReadyStateChanged {
            member_id: 1,
            ready: true,
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            stream.write_all(&echo.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.member_ready(1) == Some(true));
        server.join().unwrap();
        assert_eq!(session.member_ready(1), Some(true));
        assert_eq!(session.member_ready(0), None);
    }

    #[test]
    fn waiting_seats_take_the_music_state_family() {
        // Live caller chain (docs/live-path-reachability.md):
        //   main.rs::run_play -> assemble_online_session ->
        //   OnlineSession::paint (Waiting) -> poll_waiting_chat ->
        //   drain_waiting_chat -> apply_member_music_state (4037) /
        //   apply_acquire_music (4035) -> ScreenGame::set_member_music_state
        //   (the seat `+0x217c` byte, docs/music-availability.md §2).
        use o2jam_net::{AcquireMusic, MemberMusicState, MusicState, SyncMemberMusicStateEvent};
        let single = SyncMemberMusicStateEvent {
            member_id: 1,
            state: MusicState::NoMusic,
        };
        let acquire = AcquireMusic {
            members: vec![
                MemberMusicState {
                    member_id: 0,
                    state: MusicState::Downloading,
                },
                MemberMusicState {
                    member_id: 2,
                    state: MusicState::Ready,
                },
            ],
        };
        let mut room = guest_snapshot();
        room.user_count = 3;
        room.slots[2].member = Some(member_on_team("third", true, 2));
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream.write_all(&single.encode()).unwrap();
            stream.write_all(&acquire.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.member_music_state(2) == Some(0));
        server.join().unwrap();
        assert_eq!(
            session.game.member_music_state(1),
            Some(MusicState::NoMusic.code()),
            "4037 stages the named seat"
        );
        assert_eq!(
            session.game.member_music_state(0),
            Some(MusicState::Downloading.code()),
            "4035 stages every listed seat"
        );
        assert_eq!(session.game.member_music_state(2), Some(0));
        assert_eq!(session.game.member_music_state(3), None);
    }

    #[test]
    fn undecodable_member_update_drops_and_drain_continues() {
        use o2jam_net::{MemberReadyStateChanged, RoomSlotUpdate};
        let echo = MemberReadyStateChanged {
            member_id: 3,
            ready: false,
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            // Wide index the exe handler drops: the drain rejects it
            // the same way and keeps draining.
            stream
                .write_all(&encode_frame(RoomSlotUpdate::COMMAND, &[0x08, 0x02]))
                .unwrap();
            stream.write_all(&echo.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.member_ready(3) == Some(false));
        server.join().unwrap();
        assert_eq!(session.slot_action(7), None);
        assert_eq!(session.member_ready(3), Some(false));
    }

    #[test]
    fn keepalive_and_foreign_frames_skip_during_waiting_drain() {
        use o2jam_net::rooms::{MainRoomUserMessage, WaitingUserMessage};
        let line = WaitingUserMessage {
            sender: "host".to_owned(),
            content: b"hello".to_vec(),
            member_id: 1,
        };
        let stray = MainRoomUserMessage {
            sender: "host".to_owned(),
            content: b"lobby".to_vec(),
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            stream.write_all(&encode_frame(6001, &[])).unwrap();
            stream.write_all(&stray.encode()).unwrap();
            stream.write_all(&line.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.waiting_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.waiting_chat().len(), 1);
        assert_eq!(session.game.waiting_chat()[0].text, "hello");
        assert!(session.game.channel_chat().is_empty());
    }

    #[test]
    fn channel_user_and_admin_messages_drain_into_history() {
        use o2jam_net::rooms::{MainRoomAdminMessage, MainRoomUserMessage};
        let player = MainRoomUserMessage {
            sender: "host".to_owned(),
            content: b"hello".to_vec(),
        };
        let admin = MainRoomAdminMessage {
            sender: "op".to_owned(),
            content: b"quiet".to_vec(),
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&player.encode()).unwrap();
            stream.write_all(&admin.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.channel_chat().len() == 2);
        server.join().unwrap();
        assert_eq!(session.game.channel_chat().len(), 2);
        assert_eq!(session.game.channel_chat()[0].sender, "host");
        assert_eq!(session.game.channel_chat()[0].text, "hello");
        assert!(!session.game.channel_chat()[0].admin);
        assert_eq!(session.game.channel_chat()[1].sender, "op");
        assert_eq!(session.game.channel_chat()[1].text, "quiet");
        assert!(session.game.channel_chat()[1].admin);
    }

    #[test]
    fn incoming_whisper_drains_into_history() {
        use o2jam_net::rooms::ReceiveWhisper;
        let whisper = ReceiveWhisper {
            sender: "host".to_owned(),
            content: "psst".to_owned(),
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&whisper.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        session.set_local_user("me");
        paint_until(&mut session, |s| s.game.channel_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.channel_chat().len(), 1);
        assert_eq!(session.game.channel_chat()[0].sender, "host");
        assert_eq!(session.game.channel_chat()[0].text, "psst");
        assert_eq!(
            session.game.channel_chat()[0].whisper_recipient,
            Some("me".to_owned())
        );
        assert_eq!(session.game.last_whisper_peer(), Some("host"));
    }

    #[test]
    fn undecodable_channel_line_drops_and_drain_continues() {
        use o2jam_net::rooms::{MainRoomAdminMessage, MainRoomUserMessage};
        let admin = MainRoomAdminMessage {
            sender: "op".to_owned(),
            content: b"quiet".to_vec(),
        };
        let (mut session, server) = channel_session(move |mut stream| {
            // Invalid UTF-8 sender: the line drops instead of staging.
            stream
                .write_all(&encode_frame(MainRoomUserMessage::COMMAND, &[0xFF, 0x00]))
                .unwrap();
            stream.write_all(&admin.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.channel_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.channel_chat().len(), 1);
        assert_eq!(session.game.channel_chat()[0].text, "quiet");
        assert!(session.game.channel_chat()[0].admin);
    }

    #[test]
    fn announcement_drains_into_channel_history() {
        use o2jam_net::rooms::Announcement;
        let notice = Announcement {
            content: "hello".to_owned(),
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&notice.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.channel_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.channel_chat().len(), 1);
        assert_eq!(session.game.channel_chat()[0].text, "hello");
        assert!(session.game.channel_chat()[0].admin);
    }

    #[test]
    fn rank_notice_drains_into_channel_history() {
        use o2jam_net::rooms::RankNotification;
        let notice = RankNotification {
            invalid: false,
            music_id: 757,
            nickname: "host".to_owned(),
            ranking: 1,
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&notice.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.channel_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.channel_chat().len(), 1);
        assert_eq!(session.game.channel_chat()[0].sender, "");
        assert_eq!(
            session.game.channel_chat()[0].text,
            "host 음악전사님께서 [M757] 곡에 새로운 랭킹 [1]위가 되셨습니다~"
        );
    }

    #[test]
    fn invalid_rank_notice_stages_no_row_over_the_wire() {
        use o2jam_net::rooms::RankNotification;
        let notice = RankNotification {
            invalid: true,
            music_id: 757,
            nickname: "host".to_owned(),
            ranking: 1,
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&notice.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_frame(&mut session);
        server.join().unwrap();
        assert!(session.game.channel_chat().is_empty());
    }

    #[test]
    fn empty_announcement_stages_no_row_over_the_wire() {
        use o2jam_net::rooms::Announcement;
        let notice = Announcement {
            content: String::new(),
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&notice.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_frame(&mut session);
        server.join().unwrap();
        assert!(session.game.channel_chat().is_empty());
    }

    #[test]
    fn keepalive_and_foreign_frames_skip_silently_during_notice_drain() {
        use o2jam_net::rooms::Announcement;
        let echo = GameStatsUpdate {
            member_id: 2,
            kind_code: StatKind::Life.as_u16(),
            value: 1000,
            ranks: [10, 11, 12, 13, 14, 15, 16, 17],
        };
        let notice = Announcement {
            content: "hello".to_owned(),
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&encode_frame(6001, &[])).unwrap();
            stream.write_all(&echo.encode()).unwrap();
            stream.write_all(&notice.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        // The 4015 echo skipped past the notice drain, so slot 2 keeps
        // its default zero instead of the echo's 1000.
        paint_until(&mut session, |s| s.game.channel_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.peers().life(2), Some(0));
        assert_eq!(session.game.channel_chat().len(), 1);
        assert_eq!(session.game.channel_chat()[0].text, "hello");
    }

    #[test]
    fn undecodable_rank_notice_drops_and_drain_continues() {
        use o2jam_net::rooms::{Announcement, RankNotification};
        let notice = Announcement {
            content: "hello".to_owned(),
        };
        let (mut session, server) = channel_session(move |mut stream| {
            stream
                .write_all(&encode_frame(RankNotification::COMMAND, &[0u8; 3]))
                .unwrap();
            stream.write_all(&notice.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.channel_chat().len() == 1);
        server.join().unwrap();
        assert_eq!(session.game.channel_chat().len(), 1);
        assert_eq!(session.game.channel_chat()[0].text, "hello");
    }

    #[test]
    fn notices_stay_undrained_outside_channel() {
        use o2jam_net::rooms::Announcement;
        let notice = Announcement {
            content: "hello".to_owned(),
        };
        let (mut session, server) = waiting_session(move |mut stream| {
            stream.write_all(&notice.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |s| s.game.channel_chat().is_empty());
        server.join().unwrap();
        assert!(session.game.channel_chat().is_empty());
    }

    #[test]
    fn tick_sends_jam_channel_with_live_score() {
        let (mut session, server) = playing_session(|mut stream| {
            let jam = read_tick(&mut stream);
            assert_eq!(jam.kind, StatKind::Jam);
            assert_eq!(jam.score, 200);
            assert_eq!(jam.sequence, 0);
            let life = read_tick(&mut stream);
            assert_eq!(life.kind, StatKind::Life);
            assert_eq!(life.value, 1000);
            assert_eq!(life.score, 200);
            assert_eq!(life.sequence, 1);
            assert_quiet(&mut stream);
        });
        session.game.load_chart(vec![(0, 1000, 7)]);
        session.game.strike_at(0, 1000);
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.game.life(), 1000);
        assert!(session.peer_queue().is_none());
        assert_eq!(session.peers().life_levels(), [0; PEER_SLOT_COUNT]);
    }

    #[test]
    fn tick_skips_repeat_snapshots() {
        let (mut session, server) = playing_session(|mut stream| {
            read_tick(&mut stream);
            read_tick(&mut stream);
            assert_quiet(&mut stream);
        });
        session.game.load_chart(vec![(0, 1000, 7)]);
        session.game.strike_at(0, 1000);
        paint_frame(&mut session);
        paint_frame(&mut session);
        server.join().unwrap();
    }

    #[test]
    fn tick_resends_after_score_changes() {
        let (mut session, server) = playing_session(|mut stream| {
            let first_jam = read_tick(&mut stream);
            assert_eq!(first_jam.kind, StatKind::Jam);
            assert_eq!(first_jam.score, 200);
            assert_eq!(first_jam.sequence, 0);
            let first_life = read_tick(&mut stream);
            assert_eq!(first_life.kind, StatKind::Life);
            assert_eq!(first_life.value, 1000);
            assert_eq!(first_life.sequence, 1);
            let second_jam = read_tick(&mut stream);
            assert_eq!(second_jam.score, 400);
            assert_eq!(second_jam.sequence, 2);
            let second_life = read_tick(&mut stream);
            assert_eq!(second_life.kind, StatKind::Life);
            assert_eq!(second_life.score, 400);
            assert_eq!(second_life.sequence, 3);
        });
        session.game.load_chart(vec![(0, 1000, 7), (0, 2000, 7)]);
        session.game.strike_at(0, 1000);
        paint_frame(&mut session);
        session.game.strike_at(0, 2000);
        paint_frame(&mut session);
        server.join().unwrap();
    }

    #[test]
    fn life_tick_follows_a_life_drain() {
        // A Miss drains the Easy bar 1000 -> 950; the moved snapshot
        // resends both channels, and the Life tick carries the bar
        // word the avatar gauge paints.
        let (mut session, server) = playing_session(|mut stream| {
            let jam = read_tick(&mut stream);
            assert_eq!(jam.kind, StatKind::Jam);
            assert_eq!(jam.score, 0);
            let life = read_tick(&mut stream);
            assert_eq!(life.kind, StatKind::Life);
            assert_eq!(life.value, 950);
            assert_eq!(life.score, 0);
        });
        session.game.load_chart(vec![(0, 1000, 7)]);
        session.game.strike_at(0, 1000 + 25 + 1);
        assert_eq!(session.game.life(), 950);
        paint_frame(&mut session);
        server.join().unwrap();
    }

    #[test]
    fn tick_stays_quiet_outside_playing() {
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.game.load_chart(vec![(0, 1000, 7)]);
        session.game.strike_at(0, 1000);
        paint_frame(&mut session);
        server.join().unwrap();
        assert!(session.peer_queue().is_none());
    }

    #[test]
    fn echo_applies_to_peer_mirror() {
        let echo = GameStatsUpdate {
            member_id: 2,
            kind_code: StatKind::Life.as_u16(),
            value: 1000,
            ranks: [10, 11, 12, 13, 14, 15, 16, 17],
        };
        let (mut session, server) = playing_session(move |mut stream| {
            stream.write_all(&echo.encode()).unwrap();
            let jam = read_tick(&mut stream);
            assert_eq!(jam.kind, StatKind::Jam);
            let life = read_tick(&mut stream);
            assert_eq!(life.kind, StatKind::Life);
        });
        paint_until(&mut session, |s| s.peers().life(2) == Some(1000));
        server.join().unwrap();
        assert_eq!(session.peers().life(2), Some(1000));
        assert_eq!(session.peers().jam(2), Some(0));
        let queue = session
            .peer_queue()
            .expect("life echo forwards a queue event");
        assert_eq!(queue.slot, 2);
        assert_eq!(queue.stat_kind, StatKind::Life.as_u16());
        // Fresh song-path mirror holds no rank holders, so unmatched
        // rank bytes deliver nothing and their rows stay zero (the
        // verified remap rule in `PeerBars::apply_rank_broadcast`).
        assert!(session.peer_ranks().is_empty());
        assert_eq!(session.peers().rank_row(0), Some(0));
        // The same echo seats the shell gauge: slot 2 paints its
        // full green fill in the Playing mirror.
        assert_eq!(session.game.live_play().peers().life(2), Some(1000));
        let mut buf = vec![0u32; 70 * 60];
        session.game.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            0,
        );
        assert_eq!(buf[2 * 70 + 65], o2jam_render::pack_xrgb(0x20, 0xE0, 0x20));
    }

    #[test]
    fn unknown_channel_echo_keeps_bars_but_broadcasts_ranks() {
        let echo = GameStatsUpdate {
            member_id: 1,
            kind_code: 9,
            value: 7,
            ranks: [0; 8],
        };
        let (mut session, server) = playing_session(move |mut stream| {
            stream.write_all(&echo.encode()).unwrap();
            read_tick(&mut stream);
            read_tick(&mut stream);
        });
        paint_until(&mut session, |s| s.peer_ranks().len() == PEER_SLOT_COUNT);
        server.join().unwrap();
        assert_eq!(session.peer_queue(), None);
        assert_eq!(session.peers().life(1), Some(0));
        assert_eq!(session.peers().jam(1), Some(0));
        assert_eq!(session.peer_ranks().len(), PEER_SLOT_COUNT);
        // The shell mirror follows the same rule: no stats write, no
        // seated gauge — but the all-zero broadcast still resolves.
        assert_eq!(session.game.live_play().peers().life(1), Some(0));
        assert_eq!(session.game.live_play().peers().rank_row(0), Some(0));
    }

    #[test]
    fn keepalive_skips_silently_during_echo_drain() {
        let echo = GameStatsUpdate {
            member_id: 0,
            kind_code: StatKind::Jam.as_u16(),
            value: 3,
            ranks: [0; 8],
        };
        let (mut session, server) = playing_session(move |mut stream| {
            stream.write_all(&encode_frame(6001, &[])).unwrap();
            stream.write_all(&echo.encode()).unwrap();
            read_tick(&mut stream);
            read_tick(&mut stream);
        });
        session.game.load_chart(vec![(0, 1000, 7)]);
        session.game.strike_at(0, 1000);
        paint_until(&mut session, |s| s.peers().jam(0) == Some(3));
        server.join().unwrap();
        assert_eq!(session.peers().jam(0), Some(3));
        assert_eq!(session.game.live_play().peers().jam(0), Some(3));
        // All-zero packet matches the fresh mirror's zero holders, so
        // every position delivers (same shape `run_song_flow` proves).
        assert_eq!(session.peer_ranks().len(), PEER_SLOT_COUNT);
        assert_eq!(session.peers().rank_row(0), Some(0));
    }

    #[test]
    fn bonus_saturates_instead_of_wrapping() {
        assert_eq!(saturate_bonus(u64::from(u32::MAX) + 1), i32::MAX);
        assert_eq!(saturate_bonus(12), 12);
    }

    /// Synthetic one-song catalog blob (the 7.55 `OJNList.dat` record
    /// shape, mirroring the loading-gate tests): song 757 backed by
    /// `o2ma757.ojm`, plus one dead zero slot the staging filters out.
    fn select_catalog_blob() -> Vec<u8> {
        let mut record = vec![0u8; 300];
        record[4..8].copy_from_slice(&757u32.to_le_bytes());
        record[8..11].copy_from_slice(b"ojn");
        record[20..24].copy_from_slice(&124f32.to_le_bytes());
        record[112..117].copy_from_slice(b"First");
        record[176..180].copy_from_slice(b"Band");
        record[240..251].copy_from_slice(b"o2ma757.ojm");
        let mut blob = record;
        blob.extend_from_slice(&vec![0u8; 300]);
        blob
    }

    /// Synthetic chart blob (the 7.55 OJN header shape, mirroring the
    /// loading-gate tests): song 757 with an empty level slice.
    fn chart_blob() -> Vec<u8> {
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

    /// Session in Waiting with the synthetic catalog staged (dialog
    /// rows plus chart-fetch tokens, no gateway traffic yet).
    fn catalog_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (mut session, server) = waiting_session(with_server);
        let blob = select_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        session.set_song_catalog(&entries);
        (session, server)
    }

    /// Host pick behind the select tests (song 757, HX, X1, arena 0).
    fn host_choice() -> SongChoice {
        SongChoice {
            music_id: 757,
            difficulty: 2,
            speed: 1,
            arena_id: 0,
        }
    }

    /// Fake-gateway host select script: asserts the 4000 pick body,
    /// skips one 4001 broadcast past the arena wait (the live
    /// room-broadcast shape), asserts the 4002 stage, echoes
    /// `arena_echo` back as 4003, asserts the 4030 list ask, and
    /// answers 4031 with song 757.
    fn answer_select(stream: &mut TcpStream, arena_echo: i32) {
        use o2jam_net::play::{MusicListAnswer, MusicListEntry, RoomArenaEcho};
        use o2jam_net::{Request, SetRoomArena, SetRoomMusic};
        let select = read_frame(stream);
        assert_eq!(select.id, Request::SetRoomMusic.id());
        assert_eq!(
            SetRoomMusic::decode_body(&select.payload).unwrap(),
            SetRoomMusic {
                music_id: 757,
                difficulty: 2,
                speed: 1,
            }
        );
        stream.write_all(&encode_frame(4001, &[])).unwrap();
        let stage = read_frame(stream);
        assert_eq!(stage.id, Request::SetRoomArena.id());
        assert_eq!(
            SetRoomArena::decode_body(&stage.payload).unwrap(),
            SetRoomArena::new(0)
        );
        stream
            .write_all(&RoomArenaEcho::new(arena_echo).encode())
            .unwrap();
        assert_eq!(read_frame(stream).id, Request::GetMusicList.id());
        let listed = MusicListAnswer {
            entries: vec![MusicListEntry {
                id: 757,
                notes_ex: 100,
                notes_nx: 200,
                notes_hx: 300,
            }],
            free_music: true,
        };
        stream.write_all(&listed.encode()).unwrap();
    }

    #[test]
    fn catalog_stages_dialog_rows_and_tokens() {
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        let blob = select_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        session.set_song_catalog(&entries);
        server.join().unwrap();
        assert_eq!(session.game.select_songs().len(), 1);
        assert_eq!(session.game.select_songs()[0].song_id, 757);
        assert_eq!(session.game.select_songs()[0].title, "First");
        let chart = session.chart_path_for_song(757).unwrap();
        assert_eq!(chart.file_name().unwrap(), "o2ma757.ojn");
        assert!(session.chart_path_for_song(9999).is_none());
    }

    #[test]
    fn select_song_sends_4000_4002_4030_and_stages_pick() {
        // Ticket #172: Confirm only queues the pick (the scripted
        // server asserts the 4000/4002/4030 bodies); the per-frame
        // poll moves the legs and stages it.
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            answer_select(&mut stream, 0);
            assert_quiet(&mut stream);
        });
        session.set_pending_song(host_choice());
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| session.selected_song().is_some());
        server.join().unwrap();
        assert!(session.pending_song.is_none());
        assert_eq!(
            session.selected_song(),
            Some(o2jam_net::SetRoomMusic {
                music_id: 757,
                difficulty: 2,
                speed: 1,
            })
        );
    }

    #[test]
    fn select_song_rejects_arena_mismatch_without_staging() {
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            use o2jam_net::play::RoomArenaEcho;
            use o2jam_net::{Request, SetRoomArena, SetRoomMusic};
            let select = read_frame(&mut stream);
            assert_eq!(select.id, Request::SetRoomMusic.id());
            assert!(SetRoomMusic::decode_body(&select.payload).is_ok());
            stream.write_all(&encode_frame(4001, &[])).unwrap();
            let stage = read_frame(&mut stream);
            assert_eq!(stage.id, Request::SetRoomArena.id());
            assert_eq!(
                SetRoomArena::decode_body(&stage.payload).unwrap(),
                SetRoomArena::new(0)
            );
            stream.write_all(&RoomArenaEcho::new(7).encode()).unwrap();
            // The client bails on the mismatch: no 4030 follows.
            assert_quiet(&mut stream);
        });
        session.set_pending_song(host_choice());
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert!(session.pending_song.is_some());
        assert_eq!(session.selected_song(), None);
    }

    #[test]
    fn guest_confirm_with_pending_pick_sends_ready_not_select() {
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 4008);
            assert_quiet(&mut stream);
        });
        session.set_pending_song(host_choice());
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(session.selected_song(), None);
        assert!(session.pending_song.is_some());
    }

    #[test]
    fn master_confirm_selects_pending_song_then_starts_next_press() {
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            answer_select(&mut stream, 0);
            let start = read_frame(&mut stream);
            assert_eq!(start.id, 4010);
            assert_quiet(&mut stream);
        });
        session.set_pending_song(host_choice());
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| session.selected_song().is_some());
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert!(session.pending_song.is_none());
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(
            session.selected_song().map(|select| select.music_id),
            Some(757)
        );
    }

    #[test]
    fn stage_chart_bytes_gates_and_loads_highway() {
        let (mut session, server) = catalog_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        let chart = session.stage_chart_bytes(&chart_blob(), 0).unwrap();
        server.join().unwrap();
        assert_eq!(chart.song_id, 757);
        assert_eq!(chart.event_count, 0);
        let status = session.game.loading_status().unwrap();
        assert_eq!(status.progress, 1.0);
        assert_eq!(status.title, "First");
        assert_eq!(status.artist, "Band");
    }

    #[test]
    fn stage_chart_bytes_rejects_bad_level() {
        let (mut session, server) = catalog_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        // Reset the builder-staged armed state to the unarmed
        // baseline the assertion below assumes (the error path must
        // stage nothing new).
        session.game.clear_loading();
        assert!(session.stage_chart_bytes(&chart_blob(), 9).is_err());
        server.join().unwrap();
        assert!(session.game.loading_status().is_none());
    }

    #[test]
    fn stage_chart_bytes_without_cover_keeps_the_planet_fallback() {
        let (mut session, server) = catalog_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        let chart = session.stage_chart_bytes(&chart_blob(), 0).unwrap();
        server.join().unwrap();
        assert_eq!(chart.song_id, 757);
        assert!(!session.game.has_loading_cover());
    }

    #[test]
    fn stage_chart_bytes_reject_stages_no_cover() {
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        assert!(session.stage_chart_bytes(b"garbage", 0).is_err());
        server.join().unwrap();
        assert!(!session.game.has_loading_cover());
    }

    #[test]
    fn walked_cover_paints_the_session_loading_scene() {
        // Install-guarded: song 757's OJN cover section stages into
        // the Loading art. Skips when `install-root` is absent.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let (mut session, server) = waiting_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        let chart = session.stage_chart_bytes(&bytes, 0).unwrap();
        server.join().unwrap();
        assert_eq!(chart.song_id, 757);
        assert!(session.game.has_loading_cover());
    }

    /// Live chain: `main.rs::run_online` -> `assemble_online_session` ->
    /// `OnlineSession::stage_pending_song_selection` ->
    /// `ScreenGame::set_selected_music` -> `resolve_song_cover` ->
    /// `stage_loading_cover` -> `paint_loading` -> `paint_loading_cover`.
    /// The replayed Waiting Confirm is a local advance (no joined room, no
    /// 4031 answer), so boot staging is the only cover source on that walk.
    /// The install holds the OJN bytes; the catalog rows here are synthetic.
    /// Skip cleanly when the install is absent (CI has no game data).
    #[test]
    fn local_waiting_advance_paints_the_pending_host_picks_installed_cover() {
        let chart_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        if !chart_path.is_file() {
            println!("skip: missing installed chart");
            return;
        }
        let (port, server) = fake_gateway(|_stream| {});
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        let catalog_blob = select_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&catalog_blob).unwrap();
        session.set_song_catalog(&entries);
        assert!(!session.game.has_loading_cover());
        session.set_pending_song(host_choice());
        session.stage_pending_song_selection();
        assert_eq!(
            session.game.selected_music().map(|pick| pick.music_id),
            Some(757),
            "the CLI host pick must stage before the 4031 answer"
        );
        assert!(
            session.game.has_loading_cover(),
            "the staged catalog must resolve the pending pick's cover"
        );
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Loading);
        server.join().unwrap();
    }

    #[test]
    fn timeout_restore_after_drain_section() {
        let (mut session, server) = channel_session(|_stream| {});
        session.with_drain_timeout(std::time::Duration::from_millis(1), "test", |_s| {});
        assert_eq!(
            session.gateway_read_timeout(),
            Some(std::time::Duration::from_secs(10)),
            "drain section must restore the resting timeout"
        );
        server.join().unwrap();
    }

    #[test]
    fn retry_keeps_attempt_on_quiet_socket() {
        let (mut session, server) = channel_session(|_stream| {});
        let kept = session.drain_shop_buy(super::ShopBuyAttempt {
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(10),
        });
        assert!(
            kept.is_some(),
            "a quiet socket must keep the attempt for retry"
        );
        server.join().unwrap();
    }

    #[test]
    fn poll_order_pending_pauses_lobby_drain() {
        // With an attempt in flight, wire_attempt_pending must be true
        // so the paint path skips the lobby/waiting drains.
        let (mut session, server) = channel_session(|_stream| {});
        session.shop_buy_attempt = Some(super::ShopBuyAttempt {
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(10),
        });
        assert!(session.wire_attempt_pending());
        server.join().unwrap();
    }

    #[test]
    fn level_maps_difficulty_codes_onto_slots() {
        assert_eq!(level_for_difficulty(0), 0);
        assert_eq!(level_for_difficulty(1), 1);
        assert_eq!(level_for_difficulty(2), 2);
        assert_eq!(level_for_difficulty(3), 2);
        assert_eq!(level_for_difficulty(9), 2);
    }

    #[test]
    fn waiting_select_music_ok_with_staged_pick_reaches_4000_over_scripted_gateway() {
        // Ticket #232 wire guard: a real session walk, row click through
        // ingest_snapshot, OK click at the staged rect, then a paint
        // must put the 4000 SetRoomMusic frame on the wire with the
        // staged music id. Server-side assertion pins the accept clause.
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use std::collections::HashMap;

        let room = joined_snapshot();
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, o2jam_net::Request::SetRoomMusic.id());
            let body = o2jam_net::play::SetRoomMusic::decode_body(&frame.payload).unwrap();
            assert_eq!(body.music_id, 757);
            // begin_song_select also sends SetRoomArena 4002 right after.
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, o2jam_net::Request::SetRoomArena.id());
            assert_quiet(&mut stream);
        });

        // Stage the dialog overlay with a pinned OK rect plus a catalog
        // so the row click commits song 757 (Normal slot).
        let ok_placement = ScenePlacement {
            id: "IDC_BUTTON_DSMUSIC_OK".to_owned(),
            code: 0x27200002,
            sprite: None,
            rect: Rect {
                x: 326,
                y: 497,
                w: 126,
                h: 45,
            },
            role: PlacementRole::Single,
        };
        session
            .game_mut()
            .load_select_music_scene(vec![ok_placement], HashMap::new());
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&757u32.to_le_bytes());
        rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
        rec[112..117].copy_from_slice(b"First");
        rec[176..180].copy_from_slice(b"Band");
        rec[240..251].copy_from_slice(b"o2ma757.ojm");
        let entries = o2jam_file::parse_ojn_list(&rec).unwrap();
        session.set_song_catalog(&entries);

        // 1. Row click stages the pick.
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(400, 150 + 23 + 11));
        assert!(session.game_mut().has_pending_music_send());

        // 2. OK click closes the dialog.
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(389, 519));
        assert!(!session.game_mut().select_music_dialog_open());

        // 3. Polling drives the queued pick onto the wire as 4000.
        paint_frame(&mut session);

        // 4. Flow advanced into song select (attempt staged, pick
        // consumed from the outbox).
        assert!(!session.game_mut().has_pending_music_send());

        server.join().unwrap();
    }

    #[test]
    fn waiting_create_room_commit_stages_room_over_scripted_gateway() {
        use o2jam_net::frame::encode_frame;
        use o2jam_net::message::{CreateResult, CreateRoom, CreateRoomResponse, CreatedRoom};
        use o2jam_net::{RoomList, RoomListEntry};
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use std::collections::HashMap;

        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, o2jam_net::Request::CreateRoom.id());
            let body = CreateRoom::decode_body(&frame.payload).unwrap();
            assert_eq!(body.title, "x");
            let resp = CreateRoomResponse {
                result: CreateResult::Success,
                info: Some(CreatedRoom {
                    number: 42,
                    premium: false,
                    password: String::new(),
                    has_password: false,
                    min_level: 1,
                    max_level: 99,
                }),
            };
            stream
                .write_all(&encode_frame(
                    CreateRoomResponse::COMMAND,
                    &resp.encode_body(),
                ))
                .unwrap();
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, o2jam_net::Request::GetChannelInfo.id());
            let entry = RoomListEntry {
                number: 42,
                state_code: 1,
                title: "x".to_owned(),
                has_password: false,
                music_id: 0,
                difficulty: 0,
                key_mode: 2,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 1,
                min_level: 1,
                max_level: 99,
                skills: Vec::new(),
                premium: false,
                room_type: 0,
            };
            let list = RoomList { rooms: vec![entry] };
            stream
                .write_all(&encode_frame(RoomList::COMMAND, &list.encode_body()))
                .unwrap();
            assert_quiet(&mut stream);
        });

        let placements = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_DCROOM_CREATE".to_owned(),
            code: o2jam_shell::screens::side::CREATE_ROOM_CREATE_CODE,
            sprite: None,
            rect: Rect {
                x: 10,
                y: 10,
                w: 50,
                h: 30,
            },
        }];
        session
            .game_mut()
            .load_create_room_scene(placements, HashMap::new());
        session.game_mut().open_create_room_dialog();
        session.game_mut().push_room_dialog_char('x');
        session.game_mut().click_room_dialog(20, 20);
        assert!(session.game_mut().room_dialog().is_none());

        paint_frame(&mut session);

        assert_eq!(session.screen(), Screen::Waiting);
        assert_eq!(
            session.joined_room.as_ref().map(|r| r.title.as_str()),
            Some("x")
        );

        server.join().unwrap();
    }

    #[test]
    fn live_created_room_master_paints_start_not_ready() {
        // Live-path guard (ticket #277): a real scripted-gateway
        // create-room walk reaches Waiting as host, stages the genuine
        // 7.55 art, paints a frame, and must show the master control
        // set (START present, READY absent). Guard-green/live-broken
        // (#191-#193): the seam filter pins `IDC_BUTTON_SWAIT_START`
        // vs `IDC_BUTTON_SWAIT_READY`, but the live backdrop also
        // carries `IDC_IMAGE_SWAIT_READYBUTTEONINACTIVE`
        // (`Waiting_Ready_Inactive.ojs`) at the same rect, which the
        // filter leaves unfiltered and paints over START for the host.
        // Live caller chain:
        //   assemble_online_session -> session.stage_live_art()
        //   -> session.create_and_join_room() -> stage_hosted_room
        //   -> ScreenGame::set_joined_room -> self.waiting (member_id 0)
        //   paint -> OnlineSession::paint -> ScreenGame::paint
        //   -> paint_waiting (Some art, Some room)
        //   -> paint_waiting_scene(frame, room, art, captions)
        //   -> art.paint_backdrop_live(Screen::Waiting, keep)
        //     keep: START/MASTER_SKILLSEL iff is_room_master (member_id==0),
        //           READY/SKILLSEL iff !is_room_master
        //   -> paint_waiting_staged / paint_waiting_seat_avatars / ...
        use o2jam_net::frame::encode_frame;
        use o2jam_net::message::{CreateResult, CreateRoomResponse, CreatedRoom};
        use o2jam_net::{RoomList, RoomListEntry};
        use o2jam_render::Frame;
        use o2jam_shell::screens::paint::SceneArt;
        use std::path::PathBuf;

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
        if o2jam_shell::staging::read_install_archives(&root)
            .interface
            .is_none()
        {
            return;
        }

        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, o2jam_net::Request::CreateRoom.id());
            let resp = CreateRoomResponse {
                result: CreateResult::Success,
                info: Some(CreatedRoom {
                    number: 42,
                    premium: false,
                    password: String::new(),
                    has_password: false,
                    min_level: 1,
                    max_level: 99,
                }),
            };
            stream
                .write_all(&encode_frame(
                    CreateRoomResponse::COMMAND,
                    &resp.encode_body(),
                ))
                .unwrap();
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, o2jam_net::Request::GetChannelInfo.id());
            let entry = RoomListEntry {
                number: 42,
                state_code: 1,
                title: "x".to_owned(),
                has_password: false,
                music_id: 0,
                difficulty: 0,
                key_mode: 2,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 1,
                min_level: 1,
                max_level: 99,
                skills: Vec::new(),
                premium: false,
                room_type: 0,
            };
            let list = RoomList { rooms: vec![entry] };
            stream
                .write_all(&encode_frame(RoomList::COMMAND, &list.encode_body()))
                .unwrap();
            assert_quiet(&mut stream);
        });

        session.stage_live_art();

        let placements = vec![o2jam_shell::avatar::ScenePlacement {
            role: o2jam_shell::avatar::PlacementRole::Single,
            id: "IDC_BUTTON_DCROOM_CREATE".to_owned(),
            code: o2jam_shell::screens::side::CREATE_ROOM_CREATE_CODE,
            sprite: None,
            rect: o2jam_render::Rect {
                x: 10,
                y: 10,
                w: 50,
                h: 30,
            },
        }];
        session
            .game_mut()
            .load_create_room_scene(placements, std::collections::HashMap::new());
        session.game_mut().open_create_room_dialog();
        session.game_mut().push_room_dialog_char('x');
        session.game_mut().click_room_dialog(20, 20);
        assert!(session.game_mut().room_dialog().is_none());

        paint_until(&mut session, |s| s.screen() == Screen::Waiting);

        // Live paint (what the host actually sees).
        let mut live_buf = vec![0u32; 800 * 600];
        let mut live_frame = Frame {
            buf: &mut live_buf,
            w: 800,
            h: 600,
        };
        session.paint(&mut live_frame);

        assert!(session.game.is_room_master(), "host must be room master");
        assert!(
            !session.game.waiting_confirm_armed(),
            "no start request stands before the click"
        );

        // Reference paint: same scene art, but only the master's START
        // control at the shared rect (READY + its inactive overlay
        // excluded). The live frame must match it in that rect; any
        // difference means a non-master sprite (READY / inactive
        // overlay) is showing for the master.
        let archive = o2jam_shell::staging::read_install_archives(&root)
            .interface
            .unwrap();
        let report = o2jam_shell::staging::stage_state(
            &archive,
            o2jam_shell::staging::State::Waiting,
            o2jam_shell::staging::StageContext::default(),
        );
        let staged = report.art.expect("Waiting art staged");
        let scene_art = SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        };
        let mut ref_buf = vec![0u32; 800 * 600];
        let mut ref_frame = Frame {
            buf: &mut ref_buf,
            w: 800,
            h: 600,
        };
        // The master's own START body at its rest record: ticket #284
        // made the confirm face paint exactly one record (#284: the
        // body carries 3, the exe paints one frame), so the reference
        // is that one record, not the whole stack.
        let start = scene_art
            .placements
            .iter()
            .find(|placed| placed.id == "IDC_BUTTON_SWAIT_START")
            .expect("START staged");
        let shared = o2jam_shell::avatar::shared_sprite_names(&scene_art.placements);
        let shares_body = start
            .sprite
            .as_deref()
            .is_some_and(|sprite| shared.contains(sprite));
        o2jam_shell::avatar::paint_placement_layer(
            &mut ref_frame,
            start,
            &scene_art.figures,
            0,
            shares_body,
            None,
        );

        // Compare the shared control rect (628,126)-(770,163).
        let (rx, ry, rw, rh) = (628i32, 126i32, 142i32, 37i32);
        for dy in 0..rh {
            for dx in 0..rw {
                let x = (rx + dx) as usize;
                let y = (ry + dy) as usize;
                if live_buf[y * 800 + x] != ref_buf[y * 800 + x] {
                    panic!(
                        "READY-shown-for-master: live Waiting paint at ({},{}) differs from START-only reference (live={:#010x}, ref={:#010x})",
                        x, y, live_buf[y * 800 + x], ref_buf[y * 800 + x]
                    );
                }
            }
        }

        server.join().unwrap();
    }

    /// The role-owned Waiting confirm body (START for a master, READY
    /// for a guest) and its `.bnd` rect in the staged Waiting art.
    fn confirm_control(is_room_master: bool) -> &'static str {
        if is_room_master {
            "IDC_BUTTON_SWAIT_START"
        } else {
            "IDC_BUTTON_SWAIT_READY"
        }
    }

    /// The staged Waiting art the live join stages (ticket #284 live
    /// guards), or `None` when this checkout has no game data.
    fn staged_waiting_art() -> Option<o2jam_shell::screens::paint::SceneArt> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
        let archive = o2jam_shell::staging::read_install_archives(&root).interface?;
        let report = o2jam_shell::staging::stage_state(
            &archive,
            o2jam_shell::staging::State::Waiting,
            o2jam_shell::staging::StageContext::default(),
        );
        let staged = report.art?;
        Some(o2jam_shell::screens::paint::SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        })
    }

    /// A frame carrying one record of the staged confirm body, painted
    /// alone: the reference the live face must match.
    fn confirm_record_reference(
        scene_art: &o2jam_shell::screens::paint::SceneArt,
        confirm: &str,
        record: usize,
    ) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        let placed = scene_art
            .placements
            .iter()
            .find(|placed| placed.id == confirm)
            .expect("confirm control staged");
        let shared = o2jam_shell::avatar::shared_sprite_names(&scene_art.placements);
        let shares_body = placed
            .sprite
            .as_deref()
            .is_some_and(|sprite| shared.contains(sprite));
        o2jam_shell::avatar::paint_placement_layer(
            &mut frame,
            placed,
            &scene_art.figures,
            record,
            shares_body,
            None,
        );
        buf
    }

    /// The confirm placement's rect as `(x, y, w, h)`.
    fn confirm_rect(
        scene_art: &o2jam_shell::screens::paint::SceneArt,
        confirm: &str,
    ) -> (i32, i32, usize, usize) {
        let rect = scene_art
            .placements
            .iter()
            .find(|placed| placed.id == confirm)
            .expect("confirm control staged")
            .rect;
        (rect.x, rect.y, rect.w as usize, rect.h as usize)
    }

    /// One native-canvas paint of the live session.
    fn paint_live(session: &mut OnlineSession) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        session.paint(&mut frame);
        buf
    }

    /// Assert the live rect carries every opaque pixel of the reference
    /// record (the body's colour-key leaves the rest of the rect showing
    /// the backdrop, which the single-record reference does not paint).
    fn compare_record_ink(
        live: &[u32],
        reference: &[u32],
        rect: (i32, i32, usize, usize),
        what: &str,
    ) {
        let (rx, ry, rw, rh) = rect;
        let mut inked = 0usize;
        for dy in 0..rh {
            for dx in 0..rw {
                let x = (rx + dx as i32) as usize;
                let y = (ry + dy as i32) as usize;
                let index = y * 800 + x;
                if reference[index] == 0 {
                    continue;
                }
                inked += 1;
                assert_eq!(
                    live[index], reference[index],
                    "{what}: live Waiting paint at ({x},{y}) differs from its record"
                );
            }
        }
        assert!(inked > 0, "{what}: the reference record paints no ink");
    }

    /// Pixels of the rect where both records ink but differ: the
    /// discrimination the face assertions rely on.
    fn differing_ink(first: &[u32], second: &[u32], rect: (i32, i32, usize, usize)) -> usize {
        let (rx, ry, rw, rh) = rect;
        let mut count = 0usize;
        for dy in 0..rh {
            for dx in 0..rw {
                let index = ((ry + dy as i32) as usize) * 800 + (rx + dx as i32) as usize;
                if first[index] != 0 && second[index] != 0 && first[index] != second[index] {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn live_guest_ready_shows_the_armed_third_record() {
        // Ticket #284 live guard: a guest's READY body carries 3 raw
        // records, and the seat-ready update arms the third one while
        // the local ready byte stands and clears it back to the rest
        // record when the byte drops (VA `0x4b5850`, file
        // `0xb590b`-`0xb594f`). Live chain: `main::run_online` ->
        // `Shell::run` -> `Session::paint` -> `ScreenGame::paint` ->
        // `paint_waiting` -> `paint_waiting_scene` ->
        // `paint_waiting_confirm_face`; the ready byte arrives from
        // `OnlineSession::confirm_guest_ready` -> `ScreenGame::set_member_ready`.
        let Some(scene_art) = staged_waiting_art() else {
            return;
        };

        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            // The Confirm's 4008 ready toggle rides an empty body.
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 4008);
            assert!(frame.payload.is_empty());
            assert_quiet(&mut stream);
        });
        session.game_mut().load_scene(
            Screen::Waiting,
            scene_art.placements.clone(),
            scene_art.figures.clone(),
        );

        let confirm = confirm_control(false);
        let rect = confirm_rect(&scene_art, confirm);
        let rest = confirm_record_reference(&scene_art, confirm, 0);
        let armed = confirm_record_reference(&scene_art, confirm, 2);
        assert!(
            differing_ink(&rest, &armed, rect) > 0,
            "the staged READY body's first and third records must differ for this guard to bite"
        );

        // Not ready yet: the rest record.
        let live = paint_live(&mut session);
        assert!(!session.game.waiting_confirm_armed());
        compare_record_ink(&live, &rest, rect, "guest at rest");

        // Confirm readies the local seat: the third record.
        session.menu_key(MenuKey::Confirm, true);
        assert!(session.game.waiting_confirm_armed(), "READY is armed");
        let live = paint_live(&mut session);
        compare_record_ink(&live, &armed, rect, "guest armed");

        // Un-ready drops it back to the rest record.
        session.game.set_member_ready(1, false);
        let live = paint_live(&mut session);
        compare_record_ink(&live, &rest, rect, "guest un-ready");

        server.join().unwrap();
    }

    #[test]
    fn live_master_start_arm_shows_the_armed_third_record() {
        // Ticket #284 live guard (master side): the START click arms the
        // control's press state word (VA `0x4b22d0`, file `0xb237d`) and
        // the face paints the body's third record until the game begins.
        // Live chain: `main::run_online` -> `Shell::run` ->
        // `Session::paint` -> `ScreenGame::paint` -> `paint_waiting` ->
        // `paint_waiting_scene` -> `paint_waiting_confirm_face`; the arm
        // arrives from `OnlineSession::confirm_master_start` ->
        // `ScreenGame::arm_waiting_start`.
        let Some(scene_art) = staged_waiting_art() else {
            return;
        };

        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            // The gated START leaves as a 4010 with an empty body.
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 4010);
            assert!(frame.payload.is_empty());
            assert_quiet(&mut stream);
        });
        session.game_mut().load_scene(
            Screen::Waiting,
            scene_art.placements.clone(),
            scene_art.figures.clone(),
        );
        assert!(session.game.is_room_master());

        let confirm = confirm_control(true);
        let rect = confirm_rect(&scene_art, confirm);
        let rest = confirm_record_reference(&scene_art, confirm, 0);
        let armed = confirm_record_reference(&scene_art, confirm, 2);
        assert!(
            differing_ink(&rest, &armed, rect) > 0,
            "the staged START body's first and third records must differ for this guard to bite"
        );

        let live = paint_live(&mut session);
        assert!(!session.game.waiting_confirm_armed());
        compare_record_ink(&live, &rest, rect, "master at rest");

        // Confirm issues the start request: the third record.
        session.menu_key(MenuKey::Confirm, true);
        assert!(session.game.waiting_confirm_armed(), "START is armed");
        let live = paint_live(&mut session);
        compare_record_ink(&live, &armed, rect, "master armed");

        server.join().unwrap();
    }

    #[test]
    fn password_join_commit_stages_room_over_scripted_gateway() {
        // Ticket #269 wire guard (join side): a real session walk,
        // password dialog commit queues the join body, then a paint
        // must put the 3002 JoinWaiting frame on the wire with the
        // committed password and stage the returned room. Server-side
        // assertion pins the accept clause.
        use o2jam_net::frame::encode_frame;
        use o2jam_net::{JoinRoom, JoinRoomResponse};
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use std::collections::HashMap;

        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, o2jam_net::Request::JoinWaiting.id());
            let body = JoinRoom::decode_body(&frame.payload).unwrap();
            assert_eq!(body.number, 7);
            assert_eq!(body.password, "pw");
            let room = joined_snapshot();
            let resp = JoinRoomResponse {
                result_code: 0,
                room: Some(room),
            };
            stream
                .write_all(&encode_frame(
                    JoinRoomResponse::COMMAND,
                    &resp.encode_body(),
                ))
                .unwrap();
            assert_quiet(&mut stream);
        });

        let placements = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_DPASSWORD_OK".to_owned(),
            code: o2jam_shell::screens::side::PASSWORD_OK_CODE,
            sprite: None,
            rect: Rect {
                x: 10,
                y: 10,
                w: 50,
                h: 30,
            },
        }];
        session
            .game_mut()
            .load_password_scene(placements, HashMap::new());
        session.game_mut().open_password_dialog(7);
        session.game_mut().push_room_dialog_char('p');
        session.game_mut().push_room_dialog_char('w');
        session.game_mut().click_room_dialog(20, 20);
        assert!(session.game_mut().room_dialog().is_none());
        assert_eq!(session.screen(), Screen::Waiting);

        paint_frame(&mut session);

        assert_eq!(session.screen(), Screen::Waiting);
        assert!(session.joined_room.is_some());
        assert!(session.game_mut().take_join().is_none());

        server.join().unwrap();
    }
}
