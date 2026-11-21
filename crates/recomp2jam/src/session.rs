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
//! `ResultMemberRecord` words before its total is
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
//! URL string file `0x24a2d0` / VA `0x64a2d0`; the old `0x24a338`
//! pin named `PointNum.ojs` art) is an HTTP GET, not a gateway frame — it stays
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

use std::collections::VecDeque;
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
    RoomList, RoomListEntry, RoomSlot, RoomSlotAction, RoomSlotState, RoomSlotUpdate, RoomState,
    SendMainRoomMessage, SendWaitingMessage, SendWhisper, WaitingAdminMessage, WaitingUserMessage,
};
use o2jam_net::shop::{
    EquipItem, EquipItemResponse, ExitShop, SellItemResponse, SyncItemPurchase,
    SyncItemPurchaseReply, SyncMusicPurchase, SyncMusicPurchaseResponse,
};
use o2jam_net::wallet::{SyncCashPoint, SyncCashPointResponse};
use o2jam_net::EmptyMessage;
use o2jam_net::{
    AcquireMusic, AcquireMusicRequest, ChannelLogin, ChannelLoginResponse, ChannelLoginResult,
    ChannelState, CharacterInfo, ConfirmMusicLoaded, CreateResult, CreateRoom, CreateRoomResponse,
    DownloadProgressChanged, Event, ExitPlaying, ExitWaiting, FinalizeRank, Frame, GameMode,
    GameStatsUpdate, GatewayConnection, Generic, GetMusicList, JoinResult, JoinRoom,
    JoinRoomResponse, JoinedRoom, KeyMode, MemberReadyStateChanged, MusicState, Ready, Request,
    Response, RoomCreated, RoomRemoved, SetDownloadProgress, SetRoomArena, SetRoomMusic, StartGame,
    SubmitScore, SyncMemberMusicStateEvent, SyncMusicDownload, UpdateMusicState, UserJoinedWaiting,
    UserLeftWaiting, WaitingMusicChanged, WireMessage,
};
use o2jam_shell::screens::game::{
    LobbyChat, ResultMemberRecord, SlashCommand, StartGateVerdict, StartRequest,
};
use o2jam_shell::screens::SideScreen;
use o2jam_shell::{GameLoop, InputEdge, InputSnapshot, MenuKey, Screen, ScreenGame};

use crate::connect::net_context;
mod attempt;
mod press;
use crate::live_art::restage_live_playing;
use crate::live_stats::{apply_peer_update, jam_tick_for_score, life_tick_for_live_play};
use crate::loading::{
    load_sound_bank, loading_phase_status, prepare_chart, stage_loading_cover, LoadingGateArm,
    LoadingGateCue, LoadingGatePacing, PreparedChart, LOADING_GATE_STEPS,
};
use press::{EntryEcho, PressDisposition};

/// 1005 refusal text at exe VA `0x64d740` (file `0x24d740`):
/// `채널 허용 인원이 초과된 상태라서 더 이상 들어갈 수 없습니다.\n\n다른 채널을 이용해 주세요.`
/// REA `ev_4c4e43875c0d39d4029adb3b49c2f4713c64828965311535bc9658595b390554`.
const CHANNEL_FULL_MESSAGE_CP949: &[u8] = &[
    0xC3, 0xA4, 0xB3, 0xCE, 0x20, 0xC7, 0xE3, 0xBF, 0xEB, 0x20, 0xC0, 0xCE, 0xBF, 0xF8, 0xC0, 0xCC,
    0x20, 0xC3, 0xCA, 0xB0, 0xFA, 0xB5, 0xC8, 0x20, 0xBB, 0xF3, 0xC5, 0xC2, 0xB6, 0xF3, 0xBC, 0xAD,
    0x20, 0xB4, 0xF5, 0x20, 0xC0, 0xCC, 0xBB, 0xF3, 0x20, 0xB5, 0xE9, 0xBE, 0xEE, 0xB0, 0xA5, 0x20,
    0xBC, 0xF6, 0x20, 0xBE, 0xF8, 0xBD, 0xC0, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E, 0x0A, 0x0A, 0xB4, 0xD9,
    0xB8, 0xA5, 0x20, 0xC3, 0xA4, 0xB3, 0xCE, 0xC0, 0xBB, 0x20, 0xC0, 0xCC, 0xBF, 0xEB, 0xC7, 0xD8,
    0x20, 0xC1, 0xD6, 0xBC, 0xBC, 0xBF, 0xE4, 0x2E,
];

/// 1005 refusal text at exe VA `0x64d6c8` (file `0x24d6c8`):
/// `유료곡 대전을 활성화하기 위한 서버입니다. 유료곡이 없으신 분은 입장하실 수 없습니다.\n\n다른 서버를 이용해 주세요.`
/// REA `ev_a2583cf8b9bfa43458b195d7862f47fdeacae009ad9cbd34d483dc06f7121e37`.
const CHANNEL_PREMIUM_MESSAGE_CP949: &[u8] = &[
    0xC0, 0xAF, 0xB7, 0xE1, 0xB0, 0xEE, 0x20, 0xB4, 0xEB, 0xC0, 0xFC, 0xC0, 0xBB, 0x20, 0xC8, 0xB0,
    0xBC, 0xBA, 0xC8, 0xAD, 0xC7, 0xCF, 0xB1, 0xE2, 0x20, 0xC0, 0xA7, 0xC7, 0xD1, 0x20, 0xBC, 0xAD,
    0xB9, 0xF6, 0xC0, 0xD4, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E, 0x20, 0xC0, 0xAF, 0xB7, 0xE1, 0xB0, 0xEE,
    0xC0, 0xCC, 0x20, 0xBE, 0xF8, 0xC0, 0xB8, 0xBD, 0xC5, 0x20, 0xBA, 0xD0, 0xC0, 0xBA, 0x20, 0xC0,
    0xD4, 0xC0, 0xE5, 0xC7, 0xCF, 0xBD, 0xC7, 0x20, 0xBC, 0xF6, 0x20, 0xBE, 0xF8, 0xBD, 0xC0, 0xB4,
    0xCF, 0xB4, 0xD9, 0x2E, 0x0A, 0x0A, 0xB4, 0xD9, 0xB8, 0xA5, 0x20, 0xBC, 0xAD, 0xB9, 0xF6, 0xB8,
    0xA6, 0x20, 0xC0, 0xCC, 0xBF, 0xEB, 0xC7, 0xD8, 0x20, 0xC1, 0xD6, 0xBC, 0xBC, 0xBF, 0xE4, 0x2E,
];

/// 1005 refusal text at exe VA `0x64d6a4` (file `0x24d6a4`):
/// `사용자 정보를 읽어오지 못했습니다.`
/// REA `ev_3c7f9db2b1b6ef599ab9785bb312c54bf03032d019aa7e26392776e4d6de60c9`.
const CHANNEL_USER_INFO_MESSAGE_CP949: &[u8] = &[
    0xBB, 0xE7, 0xBF, 0xEB, 0xC0, 0xDA, 0x20, 0xC1, 0xA4, 0xBA, 0xB8, 0xB8, 0xA6, 0x20, 0xC0, 0xD0,
    0xBE, 0xEE, 0xBF, 0xC0, 0xC1, 0xF6, 0x20, 0xB8, 0xF8, 0xC7, 0xDF, 0xBD, 0xC0, 0xB4, 0xCF, 0xB4,
    0xD9, 0x2E,
];

/// 1005 refusal text at exe VA `0x64d68c` (file `0x24d68c`):
/// `채널 정보가 없습니다.`
/// REA `ev_1bfd3059a4523c59d86d10333711fefad7304e7643cdff23f7e7b24e9f6d09b4`.
const CHANNEL_NO_CHANNEL_MESSAGE_CP949: &[u8] = &[
    0xC3, 0xA4, 0xB3, 0xCE, 0x20, 0xC1, 0xA4, 0xBA, 0xB8, 0xB0, 0xA1, 0x20, 0xBE, 0xF8, 0xBD, 0xC0,
    0xB4, 0xCF, 0xB4, 0xD9, 0x2E,
];

/// Pick the one-OK sheet text the 7.55 1005 handler posts for a failed
/// enter (VA `0x55a110`, 8-entry table at `0x55a324`). Result 1 reads
/// the optional `Restricted` word: 1 names the premium text, 0 (or an
/// absent flag) the full-channel text. Codes past the table use the
/// full-channel default arm. Success has no sheet.
fn channel_refusal_message(result_code: u32, restricted: Option<bool>) -> Option<&'static [u8]> {
    match (result_code, restricted) {
        (0, _) => None,
        (1, Some(true)) => Some(CHANNEL_PREMIUM_MESSAGE_CP949),
        (1, _) | (2..=5, _) | (8.., _) => Some(CHANNEL_FULL_MESSAGE_CP949),
        (6, _) => Some(CHANNEL_USER_INFO_MESSAGE_CP949),
        (7, _) => Some(CHANNEL_NO_CHANNEL_MESSAGE_CP949),
    }
}

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
/// The exe's 4017 member guard accepts slots `< 8` (compare at file
/// `0x15fbb6`, REA `ev_a395ece8…`); later slots never reach its UI
/// queue. Our trace mirror applies the same bound.
const SUBMITTER_MEMBER_LIMIT: usize = 8;
/// REA pin of the exe's UI event queue (REA `ev_f56c417…`).
#[cfg(test)]
const RESULT_UI_QUEUE_ADDRESS: u32 = 0x478d50;
/// The two vftable entries whose `0xFB1` arms consume 4017 notices:
/// the Playing states, not Result (REA `ev_17e20066…` / `ev_8299e310…`).
#[cfg(test)]
const RESULT_4017_PLAYING_VTABLE_ENTRIES: [u32; 2] = [0x64ae1c, 0x64b3dc];
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
/// 2009 `RoomUserCountChanged`: Memoryer sends no such frame to a
/// channel observer (p379 taps `d66d693d…01ca7`/`1c9dbcdb…8e119`;
/// probe `0c917c9f…7299d` saw pings only). The join count arrives in
/// 3003 for the joiner and 3004 for the master, never in a board row
/// update, so a scripted 2009 is a labeled no-op — no local mutation.
const ROOM_USER_COUNT_EVENT_COMMAND: u16 = 2009;

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
/// One 2005 create event as the Room-list row the painter reads.
///
/// The broadcast carries the room identity and limits. The remaining
/// fields are pinned by the 2003 row for the same freshly created room
/// (`~/scratch/p376-wire-probe4.log`): waiting, no song yet, X1,
/// eight seats, one seated host, no skills.
fn listed_room_from_created(created: RoomCreated) -> RoomListEntry {
    RoomListEntry {
        number: created.number,
        state_code: RoomState::Waiting as u8,
        title: created.title,
        has_password: created.has_password,
        music_id: 0,
        difficulty: 0,
        key_mode: created.key_mode.as_u8(),
        game_mode: created.game_mode.as_u8(),
        speed: 1,
        capacity: 8,
        user_count: 1,
        min_level: created.min_level,
        max_level: created.max_level,
        skills: Vec::new(),
        premium: created.premium,
        room_type: created.room_type,
    }
}

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

/// Pending equip apply (ticket #419): the 5004 send plus its 5005
/// verdict. The reply's tail roles are still OPEN, so the leg only
/// drains it; the shell owns the local figure restage.
#[derive(Debug, Clone, Copy)]
struct EquipAttempt {
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

/// Why the joined gate cannot post 4012 yet (ERRORS-09: no guessed
/// speed, no silent send). Both cases hold Loading with a retry.
#[derive(Debug)]
enum BlockedLoadingConfirm {
    NoSpeed,
    Gateway(o2jam_net::Error),
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
    /// A 5028 wallet refresh was sent after BACK and its 5029 answer
    /// is still expected. Gates the read-side drain so unrelated
    /// lobby/waiting frames keep their own polls.
    wallet_refresh_expected: bool,
    /// Live song-shop buy (`None` while no buy commit is in
    /// flight; the per-frame [`Self::poll_music_buy`] owns it).
    music_buy_attempt: Option<MusicBuyAttempt>,
    /// Live gift-list fetch (`None` while no MyRoom entry fetch is in
    /// flight; the per-frame [`Self::poll_gift_list`] owns it).
    gift_list_attempt: Option<GiftListAttempt>,
    /// Live gift claim (`None` while no MOVE claim is in flight; the
    /// per-frame [`Self::poll_gift_claim`] owns it).
    gift_claim_attempt: Option<GiftClaimAttempt>,
    /// Live equip apply (`None` while no 5004 apply is in flight; the
    /// per-frame [`Self::poll_equip`] owns it).
    equip_attempt: Option<EquipAttempt>,
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
    /// Wall pacing of the joined gate: cover first, then 4012 at the
    /// pinned span, then Playing after the confirm.
    loading_pacing: LoadingGatePacing,
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
    /// Frames the 07DA/07DB fuse drain read before their owning lobby
    /// or waiting drain could (#380). Replayed at the next paint instead
    /// of being dropped as foreign.
    deferred_frames: VecDeque<Frame>,
    /// Result record words per seat, from 4018 rows (`None` while the
    /// seat never sent an active row). Mirrors the exe's member record
    /// at the offsets pinned in `docs/net-protocol.md` §4c.
    result_records: [Option<ResultMemberRecord>; 8],
    /// Eight 4040 place bytes from the last rank frame (`None` until
    /// one arrives). The exe writes these to slot `+0x217d`, but no
    /// gameplay reader is pinned; this mirror accounts for the wire
    /// words without inventing a rank-column source (#665).
    last_rank_places: Option<[u8; 8]>,
    /// Last 4017 submitter slot in member range (`None` until one
    /// arrives). The exe forwards accepted slots into its UI queue
    /// but no Result-side consumer is pinned, so this stays a trace
    /// instead of an invented marker (#667/#679).
    last_submitter_slot: Option<u8>,
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
            wallet_refresh_expected: false,
            music_buy_attempt: None,
            gift_list_attempt: None,
            gift_claim_attempt: None,
            equip_attempt: None,
            catalog: Vec::new(),
            last_selection: None,
            local_user: String::new(),
            joined_waiting: false,
            joined_room: None,
            guest_ready: false,
            start_announced: false,
            loaded_sent: false,
            loading_pacing: LoadingGatePacing::default(),
            tally_sent: false,
            silent_audio: true,
            slot_actions: [None; 8],
            ready_echoes: [None; 8],
            deferred_frames: VecDeque::new(),
            result_records: [None; 8],
            last_rank_places: None,
            last_submitter_slot: None,
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
        self.set_song_catalog_with_params(entries, labels, &[])
    }

    /// Stage the catalog and its table-3 price words together: the
    /// live boot calls this once so the music-shop price cells read
    /// from parsed data, never from a guessed constant.
    pub fn set_song_catalog_with_params(
        &mut self,
        entries: &[OjnListEntry<'_>],
        labels: &[o2jam_file::OjnMidTagEntry],
        params: &[o2jam_file::OjnMidParamEntry],
    ) -> &mut Self {
        self.game.set_song_catalog_with_labels(entries, labels);
        self.game.stage_music_shop_prices(params);
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
        let request = CreateRoom {
            title: title.to_owned(),
            key_mode: KeyMode::FiveKeys,
            game_mode: GameMode::Normal,
            has_password: false,
            password: String::new(),
            min_level: 1,
            max_level: 99,
            premium: false,
        };
        self.create_room_with_request(&request)
    }

    /// Host path for the CREATE dialog commit (#603): send the body the
    /// staged form actually produced, so the mode and password the
    /// player chose reach the gateway instead of CLI defaults.
    pub fn create_room_with_request(&mut self, request: &CreateRoom) -> anyhow::Result<JoinedRoom> {
        let created = self.exchange_room_frame(
            Request::CreateRoom.id(),
            &request.encode_body(),
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
            // The host's 2006/2003 path carries no slot rows. Give the
            // waiting painter the same seven empty cover containers the
            // 3003 guest answer decodes, so a later 3004 join broadcast
            // has a seat to fill (#379).
            slots: (0..7)
                .map(|index| RoomSlot {
                    index,
                    state_code: RoomSlotState::Unoccupied as i32,
                    member: None,
                })
                .collect(),
            skills: Vec::new(),
            premium: entry.premium,
            champion: None,
            champion_streak: None,
        };
        self.stage_hosted_room(room.clone());
        self.game.set_waiting_room_number(number);
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
        self.game.set_waiting_channel_number(channel.channel_id);
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
                Ok(frame) if frame.id == WaitingMusicChanged::COMMAND => {
                    // The room echoes the pick before its 4003 arena
                    // answer (`full01/tap.log`). A select drain that
                    // merely skipped this frame left the master's own
                    // captions on the pre-pick song until another poll
                    // (#564), so fold it through the same arm the
                    // ordinary waiting drain uses.
                    self.apply_waiting_music_changed(&frame);
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
        self.game.stage_ui_effect_bank(crate::load_ui_effect_bank());
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
    /// (`None` while no active row named that seat). The row painter
    /// reads the per-seat mirror staged into `ScreenGame`; this
    /// read-back pins the write side against the same words.
    #[allow(
        dead_code,
        reason = "write-side read-back for 4018 guards; painter reads the staged shell mirror"
    )]
    #[must_use]
    pub fn result_record(&self, member_id: u8) -> Option<&ResultMemberRecord> {
        self.result_records.get(usize::from(member_id))?.as_ref()
    }

    /// Last decoded 4040 place bytes (`None` while no rank frame
    /// arrived). Read-back for the write-side guard; no painter reads
    /// these words, matching the exe's missing gameplay reader.
    #[cfg(test)]
    #[must_use]
    pub fn last_rank_places(&self) -> Option<[u8; 8]> {
        self.last_rank_places
    }

    /// Last 4017 slot in member range (`None` while none arrived).
    /// Read-back for the trace guard; no Result painter consumes it.
    #[cfg(test)]
    #[must_use]
    pub fn last_submitter_slot(&self) -> Option<u8> {
        self.last_submitter_slot
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
                        // The exe posts a one-OK Information sheet with
                        // result-specific text (#717), then leaves the
                        // Channel dialog, rows, and highlight untouched.
                        if let Some(message) =
                            channel_refusal_message(response.result_code, response.restricted)
                        {
                            self.game.open_channel_refusal_dialog(message);
                        }
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
        if let Some(attempt) = &self.login_attempt {
            self.game
                .set_waiting_channel_number(u16::try_from(attempt.channel_id).unwrap_or_default());
        }
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
                    self.game.set_waiting_room_number(attempt.number);
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
        match self.create_room_with_request(&body) {
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
        if !self.joined_waiting || self.loaded_sent {
            self.enter_playing_from_loading();
            return;
        }
        match self.send_loading_confirm() {
            Ok(()) => {
                self.loading_pacing.clear();
                self.enter_playing_from_loading();
            }
            Err(BlockedLoadingConfirm::NoSpeed) => {
                tracing::warn!("loaded send blocked: no staged speed; Confirm retries it");
            }
            Err(BlockedLoadingConfirm::Gateway(source)) => {
                tracing::warn!(%source, "loaded send failed; Confirm retries it");
                self.loading_pacing.retry_confirm_at(Instant::now());
            }
        }
    }

    /// Post the pinned 4012 shape and latch it. A failure returns the
    /// reason so both the user Confirm and the timed gate can hold.
    fn send_loading_confirm(&mut self) -> Result<(), BlockedLoadingConfirm> {
        let speed = self.tally_speed().ok_or(BlockedLoadingConfirm::NoSpeed)?;
        let loaded = ConfirmMusicLoaded {
            power_skill_id: 0,
            speed,
        };
        self.gateway
            .send_frame(Request::ConfirmMusicLoaded.id(), &loaded.encode_body())
            .map_err(BlockedLoadingConfirm::Gateway)?;
        self.loaded_sent = true;
        Ok(())
    }

    /// The shell owns the Loading -> Playing edge; both the explicit
    /// Confirm and the clocked gate take it through the same call.
    fn enter_playing_from_loading(&mut self) {
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
    /// ride this broadcast — those arrive with 4018). The exe handler
    /// (file `0x15fba0`, `ev_246be0b6…`) drops slots `>= 8` before the
    /// UI queue push; slots outside the eight member records get the
    /// same treatment here. REA finds the queue's consumers in the two
    /// Playing states, not in Result (`ev_16b2e797…`, `ev_f56c417…`).
    fn apply_score_submission(&mut self, frame: &Frame) {
        match ScoreSubmission::decode_frame(frame) {
            Ok(submitted) if usize::from(submitted.member_id) < SUBMITTER_MEMBER_LIMIT => {
                self.last_submitter_slot = Some(submitted.member_id);
                tracing::debug!(member = submitted.member_id, "tally submitted");
            }
            Ok(submitted) => {
                tracing::debug!(
                    member = submitted.member_id,
                    "dropping out-of-range submitter"
                );
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

    /// Fold one 4040 frame: decode the eight place bytes, keep them
    /// for read-back, then trace them. The exe handler (VA `0x559a60`,
    /// file `0x159a60`, `ev_41a169ae…`) forwards each place through the
    /// notify writer (VA `0x5414a0`, `ev_aad816d4…`) into slot `+0x217d`;
    /// a whole-image byte sweep (`ev_94ff31c9…`) finds no gameplay
    /// reader, so the rank column keeps its own source (#665/#680).
    fn apply_rank_update(&mut self, frame: &Frame) {
        match GameRankUpdate::decode_frame(frame) {
            Ok(ranks) => {
                self.last_rank_places = Some(ranks.member_ranks);
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

    /// Send one queued BACK exit (5007 empty body). The shell has
    /// already returned to the lobby; a failed send warns and drops the
    /// queue, since the local transition cannot be retried by another
    /// press on a side screen that is no longer open.
    fn flush_exit_shop(&mut self) {
        if self.game.take_exit_shop().is_none() {
            return;
        }
        if let Err(source) = self.gateway.send_frame(ExitShop::COMMAND, &[]) {
            tracing::warn!(%source, "shop exit send failed after BACK");
            return;
        }
        // The tap emits 5028 next; Memoryer answers with the 5029
        // wallet snapshot that repaints both shop and MyRoom.
        if let Err(source) = self.gateway.send_frame(SyncCashPoint::COMMAND, &[]) {
            tracing::warn!(%source, "wallet refresh send failed after BACK");
        } else {
            self.wallet_refresh_expected = true;
        }
    }

    /// Drain the wallet answer that follows BACK (5028 → 5029),
    /// and any other 5029 snapshot from the gateway. The body is two
    /// words: gem and cash point, the same slots the 5001 prefix and
    /// post-list wallets refresh.
    fn poll_wallet_answer(&mut self) {
        if !self.wallet_refresh_expected {
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "wallet refresh", |session| loop {
            match session.gateway.next_frame() {
                Ok(frame) if frame.id == SyncCashPointResponse::COMMAND => {
                    match SyncCashPointResponse::decode_body(&frame.payload) {
                        Ok(response) => session.game.apply_shop_cash_point(&response),
                        Err(_) => {
                            tracing::debug!(id = frame.id, "dropping undecodable wallet answer")
                        }
                    }
                    session.wallet_refresh_expected = false;
                    break;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-wallet frame in wallet drain");
                }
                Err(_) => {
                    session.wallet_refresh_expected = false;
                    break;
                }
            }
        });
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
                    match SyncItemPurchaseReply::decode_live_152(&frame.payload) {
                        Ok(reply) => self.game.apply_shop_purchase_reply(&reply),
                        Err(_) => {
                            tracing::debug!(id = frame.id, "dropping undecodable shop buy reply");
                        }
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

    /// Read-side sell refund: the live 5002/5003 legs pin the body,
    /// but the shell has no sell press, so an answer can arrive without
    /// a pending attempt. Fold a success refund into the shared wallet
    /// and leave it alone on a failure verdict. Runs before the lobby
    /// drains so a 5003 is never skipped as a foreign frame.
    fn poll_sell_response(&mut self) {
        if self.game.side() != Some(SideScreen::Shop) || self.wire_attempt_pending() {
            return;
        }
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == SellItemResponse::COMMAND => {
                    match SellItemResponse::decode_body(&frame.payload) {
                        Ok(response) => self.game.apply_shop_sell(&response),
                        Err(_) => tracing::debug!(id = frame.id, "dropping undecodable sell reply"),
                    }
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-sell frame in sell drain");
                }
                Err(_) => break,
            }
        }
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

    /// Send the shell's queued equip apply (5004) and open its reply
    /// leg. The shell already mirrors the worn set locally, so a
    /// silent server keeps the local restage but retries on the next
    /// paint when the send itself fails.
    fn flush_equip(&mut self) {
        if self.equip_attempt.is_some() {
            return;
        }
        let Some(equip) = self.game.take_equip_send() else {
            return;
        };
        if let Err(source) = self
            .gateway
            .send_frame(EquipItem::COMMAND, &equip.encode_body())
        {
            tracing::warn!(%source, "equip send failed");
            self.game.requeue_equip_send(equip);
            return;
        }
        self.equip_attempt = Some(EquipAttempt {
            deadline: Instant::now() + PENDING_WIRE_WINDOW,
        });
    }

    /// Per-frame equip leg: drain the 5005 verdict under a short IO
    /// window, then let the normal drains resume. Off MyRoom the
    /// attempt drops, so a late answer cannot fold into another scene.
    fn poll_equip(&mut self) {
        if self.game.side() != Some(SideScreen::MyRoom) {
            if self.equip_attempt.take().is_some() {
                tracing::debug!("equip reply left behind leaving the MyRoom side; dropping it");
            }
            return;
        }
        let Some(attempt) = self.equip_attempt.take() else {
            return;
        };
        if Instant::now() > attempt.deadline {
            tracing::warn!("equip timed out with no server answer; staying on MyRoom");
            return;
        }
        self.with_drain_timeout(PENDING_DRAIN_TIMEOUT, "equip", |session| {
            if let Some(kept) = session.drain_equip(attempt) {
                session.equip_attempt = Some(kept);
            }
        });
    }

    /// Move one equip attempt leg: drain until the socket goes quiet.
    /// The 5005 tail is wire-present but unpinned for local paint, so
    /// the body is validated only; the attempt clears on any reply.
    fn drain_equip(&mut self, attempt: EquipAttempt) -> Option<EquipAttempt> {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == EquipItemResponse::COMMAND => {
                    if EquipItemResponse::decode_body(&frame.payload).is_err() {
                        tracing::debug!(id = frame.id, "dropping undecodable equip reply");
                    }
                    return None;
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-equip frame in equip drain");
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
    /// quiet, folding a valid 5024 answer into the staged bag. The
    /// exe gates the success path on the validity word alone (file
    /// `0xa6135`); the verdict byte is Memoryer-only and never read
    /// (file `0xa6156`). Returns the attempt while it awaits an answer.
    fn drain_gift_claim(&mut self, attempt: GiftClaimAttempt) -> Option<GiftClaimAttempt> {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == 5024 => {
                    match ClaimGiftResponse::decode_body(&frame.payload) {
                        Ok(response) if !response.invalid => {
                            tracing::debug!(
                                verdict = response.result_code,
                                "valid gift claim; verdict byte ignored like the exe"
                            );
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
                let admitted = ResultMemberRecord::from_score_entry(entry, completed.room_master);
                *record = Some(admitted);
                self.game
                    .stage_result_member_record(entry.member_id, admitted);
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
            self.stage_guest_room_song();
            self.game.force_advance_to_loading();
        } else {
            self.game.menu_key(MenuKey::Confirm, true);
        }
        if self.game.screen() != Screen::Waiting {
            self.start_announced = false;
            if self.joined_waiting && self.game.screen() == Screen::Loading {
                self.arm_loading_gate_at(Instant::now());
            }
        }
    }

    /// Arm the joined gate at one explicit instant. The shell's live
    /// caller passes the paint clock; guards pass synthetic instants so
    /// the pinned spans need no sleeps.
    fn arm_loading_gate_at(&mut self, now: Instant) {
        self.loading_pacing.enter_at(now);
    }

    /// One clocked gate advance from `paint`. The cover is already up
    /// on the transition frame; the confirm holds until the chart
    /// arms and the measured span passes, and Playing waits for the
    /// posted 4012.
    fn advance_loading_gate_at(&mut self, now: Instant) {
        if self.game.screen() != Screen::Loading {
            self.loading_pacing.clear();
            return;
        }
        if self.game.loading_gate_ready() {
            if self.loading_pacing.arm_state() == LoadingGateArm::AwaitingChart {
                tracing::debug!("loading chart armed; confirm clock still holds");
            }
            self.loading_pacing.arm_chart_at(now);
        }
        match self.loading_pacing.advance_at(now) {
            LoadingGateCue::None => {}
            LoadingGateCue::SendConfirm => {
                if let Err(reason) = self.send_loading_confirm() {
                    tracing::warn!(?reason, "timed loaded send failed; gate holds and retries");
                    self.loading_pacing.retry_confirm_at(now);
                }
            }
            LoadingGateCue::EnterPlaying => {
                self.enter_playing_from_loading();
                if self.game.screen() == Screen::Loading {
                    // The shell still owns its chart gate. Keep the paint
                    // heartbeat alive so it can finish and retry the handoff
                    // instead of parking the cover (#727).
                    self.loading_pacing.retry_playing_at(now);
                }
            }
        }
    }

    /// Give a joined guest the chart its room already carries. The
    /// 4011 body is empty on the live lane (#355), so the exe's cover
    /// comes from the selected chart's OJN. A guest never picks locally:
    /// its room's song is the selection, and feeding it to
    /// `set_selected_music` runs the same catalog-token cover staging
    /// the master's pick uses. The chart walk then stages the full
    /// loading status the shell's Loading -> Playing gate requires; the
    /// scripted seam pre-arms that status, but the live paint path must
    /// do it here (#727). No song, no chart, no invented status.
    fn stage_guest_room_song(&mut self) {
        let Some(selection) = self.game.room_music_selection() else {
            return;
        };
        let music_id = selection.music_id;
        let level = level_for_difficulty(selection.difficulty);
        self.game.set_selected_music(selection);
        if let Err(source) = self.fetch_and_stage_chart(music_id, level) {
            tracing::warn!(%source, "guest chart staging failed; loading gate holds");
        }
    }

    /// Drain inbound 3012 player lines and 3013 administrator lines
    /// into the waiting history, plus the waiting music-state family
    /// (`4035`/`4037`/`4039`, `docs/music-availability.md` §2), until
    /// the socket goes quiet — the 4015 echo drain's best-effort
    /// shape: keepalives skip silently, foreign ids log and skip
    /// (`ERRORS-19`: nothing is dropped without a trace).
    fn drain_waiting_chat(&mut self) {
        while let Ok(frame) = self.gateway.next_frame() {
            self.apply_waiting_frame(frame);
        }
    }

    /// Apply one waiting-room frame. Shared by the socket drain and the
    /// replay of frames the fuse drain read first, so a 07DA poll can
    /// never eat a join, leave, ready, or disband broadcast (#380).
    fn apply_waiting_frame(&mut self, frame: Frame) {
        match frame.id {
            id if id == WaitingUserMessage::COMMAND => {
                self.apply_waiting_user_message(&frame);
            }
            id if id == WaitingAdminMessage::COMMAND => {
                self.apply_waiting_admin_message(&frame);
            }
            START_BROADCAST_COMMAND => self.apply_start_broadcast(&frame.payload),
            id if id == UserJoinedWaiting::COMMAND => self.apply_user_joined(&frame),
            id if id == UserLeftWaiting::COMMAND => self.apply_user_left(&frame),
            id if id == RoomSlotUpdate::COMMAND => self.apply_slot_update(&frame),
            id if id == MemberReadyStateChanged::COMMAND => self.apply_ready_echo(&frame),
            id if id == SyncMemberMusicStateEvent::COMMAND => {
                self.apply_member_music_state(&frame);
            }
            id if id == AcquireMusic::COMMAND => self.apply_acquire_music(&frame),
            id if id == DownloadProgressChanged::COMMAND => {
                self.apply_download_progress(&frame);
            }
            id if id == WaitingMusicChanged::COMMAND => {
                self.apply_waiting_music_changed(&frame);
            }
            id if announces_room_music_state(id) => self.announce_music_state(),
            id if Generic::from_id(id).is_some() => {}
            _ => tracing::debug!(id = frame.id, "skipping non-chat frame in waiting drain"),
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
        // The session mirror feeds the speed a guest sends on Loading
        // Confirm and the song a later join stages; leaving it at the
        // old snapshot would make the peer's wire disagree with its
        // own captions after the master's echo (#541).
        if let Some(room) = self.joined_room.as_mut() {
            room.music_id = event.music_id;
            room.difficulty = event.difficulty;
            room.speed = event.speed;
        }
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

    /// Fold one 4039 download percentage into its waiting seat
    /// (§2, handler VA `0x559a10`: percentage then member). The bar's
    /// rect/frame mapping stays OPEN, so the byte is staged for
    /// observers instead of driving an invented fill.
    fn apply_download_progress(&mut self, frame: &Frame) {
        match DownloadProgressChanged::decode_frame(frame) {
            Ok(progress) => {
                self.game
                    .set_member_download_progress(progress.member_id, progress.percentage);
                tracing::debug!(
                    member = progress.member_id,
                    percent = progress.percentage,
                    "download progress staged on the seat"
                );
            }
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

    /// Fold one 4009 frame into the ready mirror and the staged seat.
    /// The echo reaches every room member, so the master learns a
    /// guest's READY and a guest learns the master's from this frame
    /// (#379). Undecodable bodies drop with a trace.
    fn apply_ready_echo(&mut self, frame: &Frame) {
        let Ok(echo) = MemberReadyStateChanged::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable ready echo");
            return;
        };
        self.ready_echoes[usize::from(echo.member_id)] = Some(echo.ready);
        self.game.set_member_ready(echo.member_id, echo.ready);
    }

    /// Fold one 3004 join broadcast into the staged seats. Memoryer
    /// excludes the joiner from this event, so this is the master's
    /// only live source for a new peer (#379). The session's own 3003
    /// snapshot moves with the shell so later gates see the same room.
    fn apply_user_joined(&mut self, frame: &Frame) {
        let Ok(joined) = UserJoinedWaiting::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable join broadcast");
            return;
        };
        self.game
            .seat_waiting_member(joined.member_id, joined.member.clone());
        if let Some(room) = self.joined_room.as_mut() {
            room.seat_member(joined.member_id, joined.member);
        }
    }

    /// Fold one 3007 leave broadcast into the staged seats: the
    /// departed cover clears on every remaining member (#379). The
    /// promoted-master word is read but does not change our own seat id.
    fn apply_user_left(&mut self, frame: &Frame) {
        let Ok(left) = UserLeftWaiting::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable leave broadcast");
            return;
        };
        tracing::debug!(
            member = left.member_id,
            new_master = left.new_master_id,
            "room member left"
        );
        self.game.clear_waiting_member_seat(left.member_id);
        self.game.promote_waiting_master(left.new_master_id);
        if let Some(room) = self.joined_room.as_mut() {
            room.clear_member(left.member_id);
        }
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
        while let Ok(frame) = self.gateway.next_frame() {
            self.apply_lobby_frame(chat, frame);
        }
    }

    /// Apply one lobby frame. Shared with fuse-drain replay so a 2005
    /// create or 2007 disband read during a #367 poll still stages (#380).
    fn apply_lobby_frame(&mut self, chat: LobbyChat, frame: Frame) {
        match frame.id {
            id if id == MainRoomUserMessage::COMMAND => {
                self.apply_lobby_user_message(chat, &frame);
            }
            id if id == MainRoomAdminMessage::COMMAND => {
                self.apply_lobby_admin_message(chat, &frame);
            }
            id if id == ReceiveWhisper::COMMAND => self.apply_lobby_whisper(chat, &frame),
            id if id == Announcement::COMMAND => self.apply_announcement(chat, &frame),
            id if id == RankNotification::COMMAND => self.apply_rank_notice(chat, &frame),
            id if id == RoomCreated::COMMAND => self.apply_room_created(&frame),
            id if id == RoomRemoved::COMMAND => self.apply_room_removed(&frame),
            id if id == RoomList::COMMAND => self.apply_room_list(&frame),
            id if id == ROOM_USER_COUNT_EVENT_COMMAND => {
                tracing::debug!(id, "no observer user-count event; row count unchanged");
            }
            id if Generic::from_id(id).is_some() => {}
            _ => tracing::debug!(id = frame.id, "skipping non-chat frame in lobby drain"),
        }
    }

    /// Replay frames the #367 fuse drain read while an unrelated drain
    /// owned the screen. Room/Channel frames go to the lobby handler,
    /// Waiting frames to the waiting handler; nothing is lost just
    /// because a 07DA poll was in flight (#380).
    fn replay_deferred_frames(&mut self) {
        while let Some(frame) = self.deferred_frames.pop_front() {
            match LobbyChat::of_screen(self.game.screen()) {
                Some(chat) => self.apply_lobby_frame(chat, frame),
                None if self.game.screen() == Screen::Waiting => self.apply_waiting_frame(frame),
                None => tracing::debug!(
                    id = frame.id,
                    "dropping deferred frame outside a wire screen"
                ),
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

    /// Stage one live 2005 room-create broadcast. A peer already in the
    /// Room lobby has a 2003 snapshot without this row; the exe repaints
    /// from fresh 2005 data (`docs/captures/t320/findings.md`), so a
    /// missing row here is a stale list, not an empty server.
    ///
    /// The same fold serves a Channel-scene observer: the row is staged
    /// for the next Room-board paint but never injected into the 1003
    /// channel list. The exe's Channel-dialog phase sends no 1004, so no
    /// live 2005 reaches that socket (#506; tap `620be2ec…81a9f`, frames
    /// `9ab9ffcc…9d60`). A Channel-dialog paint on 2005 stays OPEN.
    fn apply_room_created(&mut self, frame: &Frame) {
        let Ok(created) = RoomCreated::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable room-create broadcast");
            return;
        };
        self.game.upsert_room(listed_room_from_created(created));
    }

    /// Fold one 2003 room-list snapshot into the staged rows. The
    /// same handler serves the ordinary lobby drain and frames the
    /// #367 fuse read while its 07DA node was tracked (#569), so a
    /// deferred row-state probe still reaches the list with the
    /// one-OK sheet up. Undecodable bodies drop with a trace.
    fn apply_room_list(&mut self, frame: &Frame) {
        let Ok(list) = RoomList::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable room-list frame");
            return;
        };
        self.game.set_rooms(list.rooms);
    }

    /// Fold one 2007 disband broadcast into the Room list. Memoryer
    /// sends it to the whole channel when the last member exits
    /// (`~/scratch/p380-wire-probe.log`); only the named row goes,
    /// never a whole-list refresh (#380).
    fn apply_room_removed(&mut self, frame: &Frame) {
        let Ok(removed) = RoomRemoved::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable room-remove broadcast");
            return;
        };
        self.game.remove_room(removed.number);
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

    /// #367: on a relay-down Room visit, send our own 07DA poll and
    /// keep a tracked 2011 node until its answer is processed. The
    /// shell owns the 20 s wall-clock gate; this seam only moves the
    /// frames and calls the pump. No sleep, no synthetic timer.
    fn poll_room_user_list_fuse(&mut self) {
        if !self.game.room_no_response_fuse_enabled() || self.game.room_no_response_dialog_open() {
            return;
        }
        if self.game.room_user_list_poll_tracked() {
            self.drain_room_user_list_reply();
        } else {
            self.send_room_user_list_poll();
        }
        self.game.pump_room_user_list_fuse();
    }

    fn send_room_user_list_poll(&mut self) {
        if let Err(source) = self.gateway.send_frame(Request::GetUserList.id(), &[]) {
            tracing::warn!(%source, "room user-list poll not sent; fuse unarmed");
            return;
        }
        self.game.track_room_user_list_send();
    }

    fn drain_room_user_list_reply(&mut self) {
        self.with_drain_timeout(
            PENDING_DRAIN_TIMEOUT,
            "room user list",
            |session| match session.gateway.next_frame() {
                Ok(frame) if frame.id == Response::GetUserList as u16 => {
                    session.game.track_room_user_list_reply();
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    // The socket is a stream: once read, this frame is
                    // gone. Keep it for the owning drain at the next
                    // paint instead of dropping a live broadcast (#380).
                    tracing::debug!(id = frame.id, "deferring non-07DB frame from fuse drain");
                    session.deferred_frames.push_back(frame);
                }
                Err(_) => {}
            },
        );
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
            || self.equip_attempt.is_some()
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
        self.replay_deferred_frames();
        self.flush_channel_logout();
        self.flush_exit_shop();
        self.poll_attempts();
        self.poll_sell_response();
        self.poll_wallet_answer();
        if self.game.screen() == Screen::Room {
            self.poll_room_user_list_fuse();
        }
        self.poll_stats();
        // A pending 07DA node must own the drains below; otherwise a
        // 07DB landing between two of them would be skipped as a
        // foreign frame and the fuse would starve on a live reply.
        let room_poll_pending = self.game.room_user_list_poll_tracked();
        if self.wire_attempt_pending() || room_poll_pending {
            self.poll_start();
        } else {
            self.poll_lobby_chat();
            self.poll_waiting_chat();
            self.poll_music_downloads();
        }
        self.poll_tally();
        self.flush_result_wire();
        self.advance_loading_gate_at(Instant::now());
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
        if key == MenuKey::Back && self.game.screen() == Screen::Waiting && self.joined_waiting {
            // The shell owns the local Room transition; the wire leave
            // is ours, so the remaining members receive 3007 and can
            // clear this seat (#379). A failed send keeps the intent:
            // shell state is unchanged and the next Back retries it.
            let leave = ExitWaiting { exit_type: 0 };
            if let Err(source) = self
                .gateway
                .send_frame(Request::ExitWaiting.id(), &leave.encode_body())
            {
                tracing::warn!(%source, "room leave send failed");
                return;
            }
            self.joined_waiting = false;
            self.joined_room = None;
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
        // The qualified guest gate ignores click, Return and Escape
        // (#614: `docs/wine-boot.md` focus pin, pack P15 `8773f5cc…`).
        // Filter only the live snapshot path; `menu_key` stays the
        // seam the shell and gate drivers use to arm the clock.
        let ignore_player_input = self.joined_waiting && self.game.screen() == Screen::Loading;

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
            if ignore_player_input && is_loading_gate_input(edge) {
                continue;
            }
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
        // The joined Loading gate advances on paint clocks. Without a
        // repaint request the window parks on the cover, so a peer
        // waiting for the timer-driven 4012 never reaches Playing.
        if self.game.screen() == Screen::Loading && self.loading_pacing.is_active() {
            return Some(SESSION_POLL_HEARTBEAT);
        }
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

/// Player edges the 7.55 guest gate ignores while its cover is up
/// (#614): canvas presses, Return and Escape. The lane keys stay
/// outside this pin; the gate's own clock still sends 4012.
fn is_loading_gate_input(edge: &InputEdge) -> bool {
    match *edge {
        InputEdge::PointerDown { .. } | InputEdge::PointerUp { .. } => true,
        InputEdge::Menu {
            key: MenuKey::Confirm | MenuKey::Back,
            ..
        } => true,
        InputEdge::Menu { .. } | InputEdge::Lane { .. } => false,
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
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use o2jam_game::PEER_SLOT_COUNT;
    use o2jam_net::frame::{decode_frame, encode_frame};
    use o2jam_net::{
        AcquireMusic, DownloadProgressChanged, Frame, GameStatsUpdate, GatewayConnection,
        MemberMusicState, MusicState, Request, Response, StatKind, SyncMemberMusicStateEvent,
        UpdateGameStats, WaitingMusicChanged, WireMessage, HEADER_LEN,
    };
    use o2jam_shell::screens::game::{
        LobbyChat, QuitConfirmOutcome, ResultMemberRecord, StartGateVerdict,
    };
    use o2jam_shell::screens::side::RoomDialog;
    use o2jam_shell::{GameLoop, InputSnapshot, MenuKey, Screen, ScreenGame};

    use super::{
        level_for_difficulty, listed_room_from_created, saturate_bonus, OnlineSession, SongChoice,
        RESULT_4017_PLAYING_VTABLE_ENTRIES, RESULT_UI_QUEUE_ADDRESS,
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

    /// One frame off a recording socket, or `None` on a clean client
    /// close. Unlike [`read_frame`], a short read ends the tap instead
    /// of panicking in the server thread.
    fn read_frame_until_eof(stream: &mut TcpStream) -> Option<Frame> {
        let mut header = [0u8; HEADER_LEN];
        if stream.read_exact(&mut header).is_err() {
            return None;
        }
        let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
        let mut bytes = header.to_vec();
        let mut body = vec![0u8; body_len.saturating_sub(HEADER_LEN)];
        stream.read_exact(&mut body).ok()?;
        bytes.extend_from_slice(&body);
        Some(decode_frame(&bytes).unwrap())
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
        game.menu_key(MenuKey::Back, true);
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
        game.menu_key(MenuKey::Back, true);
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
        game.menu_key(MenuKey::Back, true);
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
        game.menu_key(MenuKey::Back, true);
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
        game.menu_key(MenuKey::Back, true);
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

    /// The game's native-canvas paint only (no session poll), for
    /// frame comparisons after a wire event has been drained.
    pub(super) fn painted_game_frame(session: &mut OnlineSession) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        session.game_mut().paint(&mut o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        });
        buf
    }

    /// Paint only when the real window would request a redraw. `None`
    /// parks the loop until input, matching `WindowHandler::about_to_wait`;
    /// a parked guest is the #727 stall.
    fn paint_on_heartbeat(session: &mut OnlineSession, due: &mut Option<Instant>, now: Instant) {
        match *due {
            None => return,
            Some(deadline) if now < deadline => return,
            Some(_) => {}
        }
        paint_frame(session);
        *due = session.idle_repaint_interval().map(|gap| now + gap);
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

    /// First opcode a client posts, or `None` on a bounded wait. Scripted
    /// servers report through a channel so a stalled client fails the
    /// test instead of panicking inside the server thread.
    pub(super) fn first_opcode(stream: &mut TcpStream) -> Option<u16> {
        first_opcode_with_timeout(stream, Duration::from_secs(3))
    }

    /// Same bounded read with a caller-chosen budget for walks whose
    /// first paint stages a real chart (debug Vorbis decode is slow).
    fn first_opcode_with_timeout(stream: &mut TcpStream, timeout: Duration) -> Option<u16> {
        stream.set_read_timeout(Some(timeout)).ok()?;
        let mut header = [0u8; HEADER_LEN];
        stream.read_exact(&mut header).ok()?;
        let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
        let mut bytes = header.to_vec();
        let mut body = vec![0u8; body_len.saturating_sub(HEADER_LEN)];
        stream.read_exact(&mut body).ok()?;
        bytes.extend_from_slice(&body);
        decode_frame(&bytes).ok().map(|frame| frame.id)
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
        game.menu_key(MenuKey::Back, true);
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
        // SCRIPTED-GATEWAY reply probe only: W-a found no exe C2S
        // 5000 on the buy leg, so the probe stages the legacy empty
        // send to keep this 5001 decoder guarded.
        session.game.queue_shop_buy_for_reply_probe();
        assert!(session.shop_buy_attempt.is_none());

        paint_frame(&mut session);

        paint_until(&mut session, |s| s.game.shop_gem_caption() == "GEM 626");
        server.join().unwrap();

        assert!(session.shop_buy_attempt.is_none());
        assert!(session.game.shop_cart().is_empty());
        assert_eq!(session.game.shop_gem_caption(), "GEM 626");
    }

    #[test]
    fn low_gem_reply_completes_without_a_failure_verdict() {
        // Live caller chain:
        //   OnlineSession::paint -> poll_wire -> poll_attempts ->
        //   poll_shop_buy -> drain_shop_buy -> SyncItemPurchaseReply::
        //   decode_live_152 -> ScreenGame::apply_shop_purchase_reply.
        // SCRIPTED-GATEWAY `buy-low-gem.jsonl` (`f6f3177f…957e2`):
        // the 5001 first word is rewritten to gem 17; the 152-byte body
        // has no result field, so the reply completes with the smaller
        // wallet instead of a painted failure notice. The notice itself
        // belongs to the web payment flow and stays OPEN.
        use o2jam_net::frame::encode_frame;
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::{SideScreen, SHOP_BUYITEM_CODE};
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            assert_eq!(read_frame(&mut stream).id, 5000);
            let mut payload = vec![0u8; 152];
            payload[..4].copy_from_slice(&17i32.to_le_bytes());
            stream.write_all(&encode_frame(5001, &payload)).unwrap();
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
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
        game.load_side_scene(
            vec![ScenePlacement {
                id: String::new(),
                code: SHOP_BUYITEM_CODE,
                sprite: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 10,
                    h: 10,
                },
                role: PlacementRole::Single,
            }],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Shop));
        game.shop_cart_push(1);
        game.click_shop(1, 1);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.game.click_shop(1, 1);
        // SCRIPTED-GATEWAY reply probe only; the exe buy leg sends no
        // 5000 (W-a NULL).
        session.game.queue_shop_buy_for_reply_probe();
        paint_frame(&mut session);
        paint_until(&mut session, |s| s.game.shop_gem_caption() == "GEM 17");
        server.join().unwrap();

        assert_eq!(session.game.shop_gem_caption(), "GEM 17");
        assert!(session.game.shop_cart().is_empty());
    }

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
        game.menu_key(MenuKey::Back, true);
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
        // SCRIPTED-GATEWAY reply probe only; the exe buy leg sends no
        // 5000 (W-a NULL).
        session.game.queue_shop_buy_for_reply_probe();

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
    fn item_buy_folds_the_full_live_152_byte_reply_into_one_wallet() {
        // Live caller chain:
        //   OnlineSession::paint -> poll_wire -> poll_attempts ->
        //   poll_shop_buy -> drain_shop_buy -> SyncItemPurchaseReply::
        //   decode_live_152 -> ScreenGame::apply_shop_purchase_reply ->
        //   WalletState::apply_purchase_prefix + apply_purchase_wallets.
        // Readouts: shop_gem_caption + myroom_money_captions.
        // Tier 2 scripted gateway; the body is the pinned 152-byte
        // Memoryer shape with synthetic post-list wallet words.
        use o2jam_net::frame::encode_frame;
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::{
            SideScreen, MYROOM_ITEMCASH_CODE, MYROOM_MUSICCASH_CODE, SHOP_BUYITEM_CODE,
        };
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            assert_eq!(read_frame(&mut stream).id, 5000);
            let mut payload = vec![0u8; 152];
            payload[..4].copy_from_slice(&626i32.to_le_bytes());
            payload[16..20].copy_from_slice(&376i32.to_le_bytes());
            payload[136..140].copy_from_slice(&42i32.to_le_bytes());
            payload[140..144].copy_from_slice(&77i32.to_le_bytes());
            stream.write_all(&encode_frame(5001, &payload)).unwrap();
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
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
        game.load_side_scene(
            vec![ScenePlacement {
                id: String::new(),
                code: SHOP_BUYITEM_CODE,
                sprite: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 10,
                    h: 10,
                },
                role: PlacementRole::Single,
            }],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Shop));
        game.shop_cart_push(1);
        game.click_shop(1, 1);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.game.click_shop(1, 1);
        // SCRIPTED-GATEWAY reply probe only; the exe buy leg sends no
        // 5000 (W-a NULL).
        session.game.queue_shop_buy_for_reply_probe();
        paint_frame(&mut session);
        paint_until(&mut session, |s| s.game.shop_gem_caption() == "GEM 626");
        server.join().unwrap();

        let captions = session.game.myroom_money_captions();
        let caption = |code| {
            captions
                .iter()
                .find(|(placed_code, _)| *placed_code == code)
                .map(|(_, text)| text.as_str())
        };
        assert_eq!(caption(MYROOM_MUSICCASH_CODE), Some("42"));
        assert_eq!(caption(MYROOM_ITEMCASH_CODE), Some("77"));
        assert!(session.game.shop_cart().is_empty());
    }

    #[test]
    fn wallet_refresh_reads_the_live_626_over_zero_5029_body() {
        // Live caller chain:
        //   OnlineSession::paint -> poll_wire -> poll_wallet_answer ->
        //   SyncCashPointResponse::decode_body ->
        //   ScreenGame::apply_shop_cash_point -> WalletState::apply_cash_point.
        // The BACK tap's wire `94b2ea8f…a2508` carries `5029` body
        // `7202000000000000` (gem 626, cash point 0); the exe paints
        // `G 626 / W 0` in the same frame. Tier 2 scripted gateway.
        use o2jam_net::frame::encode_frame;
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::{SideScreen, SHOP_BACK_CODE};
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            let exit = read_frame(&mut stream);
            assert_eq!(exit.id, 5007);
            stream
                .write_all(&encode_frame(
                    5029,
                    &[0x72, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                ))
                .unwrap();
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.load_side_scene(
            vec![ScenePlacement {
                id: String::new(),
                code: SHOP_BACK_CODE,
                sprite: None,
                rect: Rect {
                    x: 744,
                    y: 570,
                    w: 40,
                    h: 25,
                },
                role: PlacementRole::Single,
            }],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Shop));
        game.click_shop(757, 580);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        paint_frame(&mut session);
        server.join().unwrap();

        assert_eq!(session.game.shop_gem_caption(), "GEM 626");
        assert_eq!(session.game.wallet().cash_point, Some(0));
    }

    #[test]
    fn item_shop_back_sends_the_empty_5007_exit() {
        // Live caller chain:
        //   OnlineSession::paint -> poll_wire -> flush_exit_shop ->
        //   ScreenGame::take_exit_shop (queued by click_shop BACK) ->
        //   GatewayConnection::send_frame(5007, []).
        // Tap `ff522720…ef29b270` and wire `94b2ea8f…a2508`.
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::{SideScreen, SHOP_BACK_CODE};
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            let exit = read_frame(&mut stream);
            assert_eq!(exit.id, 5007);
            assert!(exit.payload.is_empty(), "5007 carries an empty body");
            let refresh = read_frame(&mut stream);
            assert_eq!(refresh.id, 5028);
            assert!(refresh.payload.is_empty(), "5028 carries an empty body");
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.load_side_scene(
            vec![ScenePlacement {
                id: String::new(),
                code: SHOP_BACK_CODE,
                sprite: None,
                rect: Rect {
                    x: 744,
                    y: 570,
                    w: 40,
                    h: 25,
                },
                role: PlacementRole::Single,
            }],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Shop));
        game.click_shop(757, 580);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.game.side(), None);
    }

    /// W-a NULL pin (#397, traces `W6FA-BUY-01/02`): a cart-row pick,
    /// BUYITEM and the web page's `(345,385)` OK put no shop-buy opcode
    /// on the game socket. Only the BACK leg's pinned `5007`/`5028`
    /// pair follows. The 5001 seen in the exe tap was BACK-timed.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> OnlineSession::ingest_snapshot ->
    ///   click_side_screen -> click_shop(ToggleRow/CartBuy) ->
    ///   open local web surface -> click_shop_purchase_web(BACK) ->
    ///   flush_exit_shop -> GatewayConnection::send_frame(5007, 5028).
    #[test]
    fn cart_row_buy_leg_is_wire_quiet_except_the_back_5007_family() {
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::{
            SideScreen, SHOP_BACK_CODE, SHOP_BUYITEM_CODE, SHOP_ITEMBUY_FIRST_CODE,
        };
        use std::collections::HashMap;

        let (wire_tx, wire_rx) = mpsc::channel();
        let (port, server) = fake_gateway(move |mut stream| {
            let mut opcodes = Vec::new();
            while let Some(frame) = read_frame_until_eof(&mut stream) {
                opcodes.push(frame.id);
            }
            wire_tx.send(opcodes).expect("the test reads its wire tap");
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.set_shop_catalog(&o2jam_file::AvatarItemTable {
            rows: vec![o2jam_file::AvatarItemRow {
                item_id: 1,
                item_type: 0,
                price: 4500,
                params: [0; 4],
                flags: (0, 0),
                name: "W6Good".to_owned(),
                description: String::new(),
                sprites: Vec::new(),
            }],
        });
        let area = |code, sprite: Option<&str>, x, y, w, h| ScenePlacement {
            id: format!("W6A_{code:X}"),
            code,
            sprite: sprite.map(str::to_owned),
            rect: o2jam_render::Rect { x, y, w, h },
            role: PlacementRole::Single,
        };
        game.load_side_scene(
            vec![
                area(
                    SHOP_ITEMBUY_FIRST_CODE,
                    Some("Shop_Btn_ItemBuy.ojs"),
                    220,
                    187,
                    81,
                    17,
                ),
                area(SHOP_BUYITEM_CODE, None, 17, 563, 49, 23),
                area(SHOP_BACK_CODE, None, 744, 570, 40, 25),
            ],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Shop));
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        session.ingest_snapshot(&InputSnapshot::click_at(238, 263));
        assert_eq!(session.game.shop_cart(), vec![1]);
        session.ingest_snapshot(&InputSnapshot::click_at(41, 574));
        session.ingest_snapshot(&InputSnapshot::click_at(345, 385));

        assert!(session.game.shop_purchase_web_open());
        assert_eq!(session.game.take_shop_buy(), None, "no 5000 was queued");
        session.ingest_snapshot(&InputSnapshot::click_at(757, 580));
        paint_frame(&mut session);
        assert_eq!(session.game.side(), None, "BACK leaves the shop side");
        drop(session);
        server.join().unwrap();
        let opcodes = wire_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            opcodes,
            vec![5007, 5028],
            "the buy leg is quiet; only BACK's 5007/5028 family is sent"
        );
    }

    #[test]
    fn sell_refund_updates_the_gem_wallet_from_a_live_5003_body() {
        // Live caller chain:
        //   OnlineSession::paint -> poll_wire -> poll_sell_response ->
        //   SellItemResponse::decode_body -> ScreenGame::apply_shop_sell ->
        //   WalletState::apply_sell_response -> shop_gem_caption.
        // Tier 2 scripted gateway. The 5003 body is the live uid-2 pin
        // `96bb62ce…25009b`: result 0, refund 14626 (626 + item 376
        // price 14000). The shell has no sell press yet, so this is
        // the read-side fold, not a player-visible sell click.
        use o2jam_net::frame::encode_frame;
        use o2jam_shell::screens::side::SideScreen;

        let (port, server) = fake_gateway(|mut stream| {
            let refund = 14626i32.to_le_bytes();
            let body = [0, 0, 0, 0, refund[0], refund[1], refund[2], refund[3]];
            stream.write_all(&encode_frame(5003, &body)).unwrap();
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.apply_shop_cash_point(&o2jam_net::wallet::SyncCashPointResponse {
            gem: 626,
            cash_point: 0,
        });
        assert!(game.visit_side(SideScreen::Shop));

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.game.shop_gem_caption(), "GEM 14626");
    }

    #[test]
    fn sell_failure_leaves_the_gem_wallet_untouched() {
        // Same live chain as the refund guard. The invalid sell leg
        // `9c9af74a…50d67` answers 5002 slot999 with 5003 `01000000`:
        // a result word with no refund word.
        use o2jam_net::frame::encode_frame;
        use o2jam_shell::screens::side::SideScreen;

        let (port, server) = fake_gateway(|mut stream| {
            stream
                .write_all(&encode_frame(5003, &[1, 0, 0, 0]))
                .unwrap();
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.apply_shop_cash_point(&o2jam_net::wallet::SyncCashPointResponse {
            gem: 626,
            cash_point: 0,
        });
        assert!(game.visit_side(SideScreen::Shop));

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.game.shop_gem_caption(), "GEM 626");
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
        game.menu_key(MenuKey::Back, true);
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
    fn music_buy_failure_round_trip_leaves_the_song_unowned_in_the_scripted_gateway() {
        // Live caller chain (same as the success guard):
        //   OnlineSession::paint -> flush_music_buy (5010) ->
        //   poll_music_buy -> drain_music_buy ->
        //   ScreenGame::apply_music_purchase_response.
        // The 5011 body is a state sync, not an error word: the
        // failure arm is a reply whose acquired list omits the cart
        // song. The wallet still refreshes from the reply words,
        // but no OWNED row may appear from a failed buy.
        // Tier 2 scripted gateway; synthetic catalog only (CI-safe).
        use o2jam_net::shop::{MusicEntry, SyncMusicPurchaseResponse};
        use o2jam_render::Rect;
        use o2jam_shell::avatar::PlacementRole;
        use o2jam_shell::avatar::ScenePlacement;
        use o2jam_shell::screens::side::{SideScreen, MSHOP_CART_BUY_CODE, MSHOP_EX_FIRST_CODE};
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 5010);
            let response = SyncMusicPurchaseResponse {
                gem: 777,
                point: 250,
                o2_cash: 5000,
                music_cash: 120,
                item_cash: 340,
                cash_point: 56,
                music_list: Vec::<MusicEntry>::new(),
                gift_boxes_raw: Vec::new(),
                free_pass_extension_days: 0,
            };
            stream.write_all(&response.encode()).unwrap();
        });

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.set_song_catalog(&o2jam_file::parse_ojn_list(&select_catalog_blob()).unwrap());
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
        game.click_music_shop(424, 529);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        paint_frame(&mut session);
        paint_until(&mut session, |s| s.game.shop_gem_caption() == "GEM 777");
        server.join().unwrap();

        assert!(session.game.music_shop_cart().is_empty());
        assert!(session
            .game
            .music_shop_visible_rows()
            .iter()
            .all(|row| !row.listed));
        assert!(session
            .game
            .select_songs()
            .iter()
            .any(|song| song.song_id == 757));
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
        game.menu_key(MenuKey::Back, true);
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
    fn myroom_equip_sends_5004_through_scripted_gateway() {
        // Live caller chain (#419, EQUIP-APPLY x peer):
        //   window::WindowHandler -> OnlineSession::ingest_snapshot
        //   -> ScreenGame::click_myroom_side -> apply_myroom_equip
        //   -> take_equip_send (5004) -> OnlineSession::poll_attempts.
        // Tier 2 scripted gateway; the 5004 body shape is statically
        // pinned in docs/equip.md (the exe press itself stays OPEN).
        use o2jam_net::frame::encode_frame;
        use o2jam_net::gift::GetGiftMessageListResponse;
        use o2jam_net::shop::{EquipItem, EquipItemResponse};
        use o2jam_shell::screens::side::SideScreen;
        use std::collections::HashMap;

        let (port, server) = fake_gateway(|mut stream| {
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
            assert_eq!(frame.id, 5004);
            let equip = EquipItem::decode_body(&frame.payload).unwrap();
            assert_eq!((equip.slot, equip.item_id), (0, 1));
            let reply = EquipItemResponse {
                result: 0,
                tail: None,
            };
            stream
                .write_all(&encode_frame(5005, &reply.encode_body()))
                .unwrap();
        });

        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        let placement = |code: u32, x: i32| ScenePlacement {
            id: String::new(),
            code,
            sprite: None,
            rect: Rect {
                x,
                y: 10,
                w: 40,
                h: 20,
            },
            role: PlacementRole::Single,
        };
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.set_myroom_items(o2jam_file::AvatarItemTable {
            rows: vec![o2jam_file::AvatarItemRow {
                item_id: 1,
                item_type: 2,
                price: 0,
                params: [0; 4],
                flags: (0, 0),
                name: "Item".to_owned(),
                description: String::new(),
                sprites: Vec::new(),
            }],
        });
        game.set_myroom_character(o2jam_net::CharacterInfo {
            inventory: vec![1],
            ..o2jam_net::CharacterInfo::default()
        });
        game.load_side_scene(
            vec![
                placement(o2jam_shell::screens::side::MYROOM_BAG_FIRST_CODE, 10),
                placement(o2jam_shell::screens::side::MYROOM_APPLY_CODE, 60),
            ],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::MyRoom));
        game.ingest_snapshot(&InputSnapshot::click_at(20, 15));
        game.ingest_snapshot(&InputSnapshot::click_at(70, 15));
        assert!(game.has_pending_equip_send());

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        paint_until(&mut session, |s| !s.game.has_pending_equip_send());
        let _ = server.join();
        assert!(!session.game.has_pending_equip_send());
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
        game.menu_key(MenuKey::Back, true);
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

    /// Stage one present-box gift and answer its MOVE press with a
    /// scripted 5024 body. The exe's 5024 success path (file `0xa6156`)
    /// reads the validity word only; the verdict byte stays
    /// Memoryer-only, so callers supply the body verbatim.
    fn gift_claim_reply_session(reply_body: [u8; 5]) -> (OnlineSession, thread::JoinHandle<()>) {
        use o2jam_net::frame::encode_frame;
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::SideScreen;
        use std::collections::HashMap;

        let (port, server) = fake_gateway(move |mut stream| {
            use o2jam_net::gift::GetGiftMessageListResponse;
            assert_eq!(read_frame(&mut stream).id, 5017);
            let empty = GetGiftMessageListResponse {
                invalid: false,
                messages: Vec::new(),
            };
            stream
                .write_all(&encode_frame(5018, &empty.encode_body()))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, 5023);
            stream.write_all(&encode_frame(5024, &reply_body)).unwrap();
        });

        let area = |code: u32, x: i32| ScenePlacement {
            id: String::new(),
            code,
            sprite: None,
            rect: Rect {
                x,
                y: 10,
                w: 40,
                h: 20,
            },
            role: PlacementRole::Single,
        };
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.set_myroom_items(o2jam_file::AvatarItemTable {
            rows: vec![o2jam_file::AvatarItemRow {
                item_id: 1,
                item_type: 2,
                price: 100,
                params: [0; 4],
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
        game.load_side_scene(
            vec![
                area(o2jam_shell::screens::side::MYROOM_GIFT_FIRST_CODE, 10),
                area(o2jam_shell::screens::side::MYROOM_MOVE_CODE, 100),
            ],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::MyRoom));
        game.click_myroom_side(11, 11);
        game.click_myroom_side(101, 11);

        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        (OnlineSession::new(game, gateway), server)
    }

    #[test]
    fn valid_5024_with_memoryer_verdict_four_moves_the_gift_into_the_bag() {
        // Live caller chain:
        //   OnlineSession::paint -> flush_gift_claim (5023) ->
        //   poll_gift_claim -> drain_gift_claim (5024 valid body
        //   `0000000004`) -> ScreenGame::apply_gift_claim_success.
        // The exe's 5024 handler never reads the verdict slot
        // (`0x14(%ebp)`, `o2jam-net/gift.rs`); only the validity word
        // gates its success path. Tier 2 scripted gateway.
        let (mut session, server) = gift_claim_reply_session([0, 0, 0, 0, 4]);
        paint_until(&mut session, |s| {
            s.gift_claim_attempt.is_none()
                && s.game
                    .myroom_character()
                    .is_some_and(|hero| hero.inventory.contains(&1))
        });
        server.join().unwrap();

        let hero = session.game.myroom_character().expect("staged character");
        assert!(
            hero.item_gifts.is_empty(),
            "valid claim empties the box row"
        );
        assert!(hero.inventory.contains(&1), "claimed item lands in the bag");
    }

    #[test]
    fn invalid_5024_keeps_the_present_box_row() {
        // Live caller chain: OnlineSession::paint -> flush_gift_claim ->
        // poll_gift_claim -> drain_gift_claim (invalid 5024). The exe's
        // validity gate (file `0xa6135`) sends this reply down the error
        // path, so the staged gift stays in the box.
        let (mut session, server) = gift_claim_reply_session([1, 0, 0, 0, 4]);
        paint_until(&mut session, |s| s.gift_claim_attempt.is_none());
        server.join().unwrap();

        let hero = session.game.myroom_character().expect("staged character");
        assert_eq!(hero.item_gifts.len(), 1, "invalid reply keeps the gift row");
        assert!(!hero.inventory.contains(&1), "invalid reply mints no item");
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

    /// #717 live-path seam for a refused 1005: every guard below
    /// drives the real session from Channel Confirm through
    /// `begin_channel_login`, the 1004 wire, the scripted 1005 answer,
    /// `drain_channel_login`, and the shell's Information sheet owner.
    /// The exe pins the sheet, its text, and the absence of sound,
    /// highlight, and row changes (`~/scratch/channel-full-findings.md`,
    /// `docs/parity/channel.md` ticket #717). No install data is read.
    mod refused_enter {
        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        use o2jam_audio::NullBackend;
        use o2jam_net::{ChannelLoginResponse, ChannelState, WireMessage};

        use super::*;
        use crate::session::{
            CHANNEL_FULL_MESSAGE_CP949, CHANNEL_NO_CHANNEL_MESSAGE_CP949,
            CHANNEL_PREMIUM_MESSAGE_CP949, CHANNEL_USER_INFO_MESSAGE_CP949,
        };

        /// Counts every backend emission so a refusal cannot cue sound
        /// through either the key-sound or the scene-BGM path.
        struct SoundEmitterLog {
            inner: NullBackend,
            calls: Rc<RefCell<usize>>,
        }

        impl o2jam_audio::AudioBackend for SoundEmitterLog {
            fn play_pcm(
                &mut self,
                pcm: &o2jam_audio::PcmSamples,
            ) -> Result<(), o2jam_audio::AudioError> {
                *self.calls.borrow_mut() += 1;
                self.inner.play_pcm(pcm)
            }

            fn set_scene_bgm(
                &mut self,
                pcm: Option<&o2jam_audio::PcmSamples>,
                repeat: bool,
            ) -> Result<(), o2jam_audio::AudioError> {
                *self.calls.borrow_mut() += 1;
                self.inner.set_scene_bgm(pcm, repeat)
            }

            fn samples_played(&self) -> u64 {
                self.inner.samples_played()
            }
        }

        /// Drive one 1005 refusal and assert the pinned sheet plus the
        /// three null effects. The scripted gateway sends no 2000 and
        /// hears no frame after the refusal.
        fn refusal_posts_sheet(
            result_code: u32,
            restricted: Option<bool>,
            expected_text: &'static [u8],
        ) {
            let (mut session, server) = channel_session(move |mut stream| {
                assert_eq!(read_frame(&mut stream).id, 1004);
                stream
                    .write_all(
                        &ChannelLoginResponse {
                            result_code,
                            restricted,
                        }
                        .encode(),
                    )
                    .unwrap();
                assert_quiet(&mut stream);
            });
            let rows = vec![
                entry_channel(),
                ChannelState {
                    channel_id: 10,
                    ..entry_channel()
                },
            ];
            session.game.set_channels(rows.clone());
            let sound_calls = Rc::new(RefCell::new(0));
            session.game.with_audio(
                Box::new(SoundEmitterLog {
                    inner: NullBackend::new(),
                    calls: Rc::clone(&sound_calls),
                }),
                HashMap::new(),
            );
            let calls_before = *sound_calls.borrow();
            let cue_before = session.game.scene_bgm_selection();
            let highlight_before = session.game.highlighted_channel();

            session.menu_key(MenuKey::Confirm, true);
            paint_until(&mut session, |session| {
                session.game.channel_refusal_dialog_open()
            });
            server.join().unwrap();

            assert_eq!(session.game.screen(), Screen::Channel);
            assert_eq!(
                session.game.channel_refusal_message(),
                Some(expected_text),
                "result {result_code} (restricted {restricted:?}) posts the pinned text"
            );
            assert_eq!(
                session.game.highlighted_channel(),
                highlight_before,
                "the refusal leaves the highlight where it was"
            );
            assert_eq!(session.game.channel_rows(), rows, "the rows stay unchanged");
            assert_eq!(
                session.game.scene_bgm_selection(),
                cue_before,
                "the refusal never recues the scene"
            );
            assert_eq!(
                *sound_calls.borrow(),
                calls_before,
                "the refusal emits no sound"
            );
        }

        #[test]
        fn result_one_without_restriction_posts_the_full_channel_sheet() {
            refusal_posts_sheet(1, Some(false), CHANNEL_FULL_MESSAGE_CP949);
        }

        #[test]
        fn result_one_with_restriction_posts_the_premium_sheet() {
            refusal_posts_sheet(1, Some(true), CHANNEL_PREMIUM_MESSAGE_CP949);
        }

        #[test]
        fn result_two_posts_the_full_channel_sheet() {
            refusal_posts_sheet(2, None, CHANNEL_FULL_MESSAGE_CP949);
        }

        #[test]
        fn result_three_posts_the_full_channel_sheet() {
            refusal_posts_sheet(3, None, CHANNEL_FULL_MESSAGE_CP949);
        }

        #[test]
        fn result_four_posts_the_full_channel_sheet() {
            refusal_posts_sheet(4, None, CHANNEL_FULL_MESSAGE_CP949);
        }

        #[test]
        fn result_five_posts_the_full_channel_sheet() {
            refusal_posts_sheet(5, None, CHANNEL_FULL_MESSAGE_CP949);
        }

        #[test]
        fn result_six_posts_the_user_information_sheet() {
            refusal_posts_sheet(6, None, CHANNEL_USER_INFO_MESSAGE_CP949);
        }

        #[test]
        fn result_seven_posts_the_no_channel_information_sheet() {
            refusal_posts_sheet(7, None, CHANNEL_NO_CHANNEL_MESSAGE_CP949);
        }

        #[test]
        fn result_past_the_table_posts_the_full_channel_sheet() {
            refusal_posts_sheet(8, None, CHANNEL_FULL_MESSAGE_CP949);
        }

        #[test]
        fn successful_login_posts_no_refusal_sheet() {
            let (mut session, server) =
                channel_session(|mut stream| answer_entry_login(&mut stream));
            session.game.set_channels(vec![entry_channel()]);
            session.menu_key(MenuKey::Confirm, true);
            paint_until(&mut session, |session| {
                session.game.screen() == Screen::Room
            });
            server.join().unwrap();
            assert!(!session.game.channel_refusal_dialog_open());
            assert_eq!(session.game.channel_refusal_message(), None);
        }

        /// #717 OK composition: the sheet's OK at the Information rect
        /// sends no frame, closes only the sheet, and leaves the Channel
        /// dialog, its rows, and its highlight as they were. The exe's
        /// result-7 pack pins the unchanged row and highlight after OK
        /// (`~/scratch/channel-full/win3`, `docs/parity/channel.md`).
        #[test]
        fn ok_dismisses_the_refusal_without_touching_the_channel_dialog() {
            let (mut session, server) = channel_session(|mut stream| {
                assert_eq!(read_frame(&mut stream).id, 1004);
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
            let rows = vec![entry_channel()];
            session.game.set_channels(rows.clone());
            session.menu_key(MenuKey::Confirm, true);
            paint_until(&mut session, |session| {
                session.game.channel_refusal_dialog_open()
            });
            let highlight_before = session.game.highlighted_channel();

            session.ingest_snapshot(&InputSnapshot::click_at(400, 365));

            assert!(!session.game.channel_refusal_dialog_open(), "OK closes");
            assert_eq!(session.game.channel_refusal_message(), None);
            assert_eq!(session.game.screen(), Screen::Channel);
            assert_eq!(session.game.channel_rows(), rows);
            assert_eq!(session.game.highlighted_channel(), highlight_before);
            paint_frame(&mut session);
            assert!(
                !session.game.channel_refusal_dialog_open(),
                "OK stays closed"
            );
            server.join().unwrap();
        }
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
        assert_eq!(
            session.game.waiting_room_number(),
            Some(3),
            "the accepted 3002 number reaches the waiting header"
        );
    }

    #[test]
    fn resident_channel_stages_the_waiting_header_number() {
        // Live caller chain:
        //   main.rs::run_play -> setup_online -> OnlineSession::set_resident_channel
        //   -> ScreenGame::set_waiting_channel_number. Boot residency
        //   and a resolved 1005 login both take this setter.
        let (port, server) = fake_gateway(|mut stream| {
            assert_quiet(&mut stream);
        });
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(ScreenGame::new(), gateway);
        let channel = entry_channel();
        session.set_resident_channel(channel);
        assert_eq!(
            session.game.waiting_channel_number(),
            Some(channel.channel_id),
            "header uses the wire channel id, not the list ordinal"
        );
        server.join().unwrap();
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
    fn room_created_maps_the_wire_descriptor_to_a_listed_row() {
        use o2jam_net::{GameMode, KeyMode, RoomCreated};
        let created = RoomCreated {
            number: 4,
            title: "P376ROOM".to_owned(),
            key_mode: KeyMode::FiveKeys,
            game_mode: GameMode::Normal,
            has_password: false,
            min_level: 1,
            max_level: 99,
            premium: false,
            room_type: 0,
        };
        let row = listed_room_from_created(created);
        assert_eq!(row.number, 4);
        assert_eq!(row.title, "P376ROOM");
        assert_eq!(row.state_code, 1);
        assert_eq!(row.key_mode, KeyMode::FiveKeys.as_u8());
        assert_eq!(row.min_level, 1);
        assert_eq!(row.max_level, 99);
        assert_eq!(row.capacity, 8);
        assert_eq!(row.user_count, 1);
    }

    /// Row lock badge from the exe CREATE arms (ticket #519).
    /// `~/scratch/evidence-room/exe-master-locked-5/exe/phases-create.log`
    /// 03:41:33 logs the locked 2004 body
    /// `4c4f434b4544505300020001313233340000000000` (flag byte 1,
    /// password `1234`) beside `lock-02-password-frame.png` sha256
    /// `b749dc90b242460a9d9f5e172d47cd6f94985cc3e4796fa57ce1189d1a9ee71b`;
    /// `~/scratch/evidence-room/create-fields-2/exe/wire/room-stage.log`
    /// 02:27:01 logs the unlocked arm (flag byte 0) beside
    /// `pass-04-row-unlocked.png` sha256
    /// `734631eb452bfecc815360d9568841fb8b71a4cb7faf0cbdc3f6acc4e43a196c`.
    /// Caller chain: boot → Room → scripted 2005 → `apply_room_created`
    /// → `ScreenGame::upsert_room` → the row's staged lock badge.
    #[test]
    fn create_lock_flag_survives_the_2005_broadcast_into_the_row() {
        use o2jam_net::{CreateRoom, RoomCreated};
        let pinned_bodies = [
            (
                "4c4f434b4544505300020001313233340000000000",
                "LOCKEDPS",
                true,
            ),
            ("454d505459505300020000310000000000", "EMPTYPS", false),
        ];
        for (body_hex, title, locked) in pinned_bodies {
            let create = CreateRoom::decode_body(&hex_bytes(body_hex)).unwrap();
            assert_eq!(create.title, title);
            assert_eq!(create.has_password, locked, "exe lock flag");
            let created = RoomCreated {
                number: 0,
                title: create.title.clone(),
                key_mode: create.key_mode,
                game_mode: create.game_mode,
                has_password: create.has_password,
                min_level: create.min_level,
                max_level: create.max_level,
                premium: create.premium,
                room_type: 0,
            };
            let (mut session, server) = room_session(move |mut stream| {
                stream.write_all(&created.encode()).unwrap();
                assert_quiet(&mut stream);
            });
            paint_until(&mut session, |s| s.game.listed_rooms().len() == 1);
            let row = &session.game.listed_rooms()[0];
            assert_eq!(row.title, title);
            assert_eq!(row.has_password, locked, "row lock badge");
            server.join().unwrap();
        }
    }

    #[test]
    fn room_created_broadcast_stages_a_live_row_for_a_peer_already_in_room() {
        // Live caller chain: `Shell::run` -> `OnlineSession::paint` ->
        // `poll_wire` -> `poll_lobby_chat` (Room scene). Memoryer pushes
        // opcode 2005 when the host creates a room after this peer entered
        // (`~/scratch/p376-wire-probe2.log`); the row must appear without
        // another 2002/2003 refresh.
        use o2jam_net::{GameMode, KeyMode, RoomCreated};
        let created = RoomCreated {
            number: 0,
            title: "P376ROOM".to_owned(),
            key_mode: KeyMode::FiveKeys,
            game_mode: GameMode::Normal,
            has_password: false,
            min_level: 1,
            max_level: 99,
            premium: false,
            room_type: 0,
        };
        let (mut session, server) = room_session(move |mut stream| {
            stream.write_all(&created.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_frame(&mut session);
        server.join().unwrap();
        let titles: Vec<&str> = session
            .game_mut()
            .listed_rooms()
            .iter()
            .map(|room| room.title.as_str())
            .collect();
        assert_eq!(titles, vec!["P376ROOM"]);
    }

    /// One listed row for the #380 disband guards.
    fn listed_p380_room(number: i32, title: &str) -> o2jam_net::RoomListEntry {
        use o2jam_net::{GameMode, KeyMode, RoomCreated};
        listed_room_from_created(RoomCreated {
            number,
            title: title.to_owned(),
            key_mode: KeyMode::FiveKeys,
            game_mode: GameMode::Normal,
            has_password: false,
            min_level: 1,
            max_level: 99,
            premium: false,
            room_type: 0,
        })
    }

    #[test]
    fn last_leave_broadcast_removes_the_disbanded_room_row() {
        // Live caller chain: `Shell::run` -> `OnlineSession::paint` ->
        // `poll_wire` -> `poll_lobby_chat` (Room scene). The tapped
        // two-client probe (`~/scratch/p380-wire-probe.log`, 13:05:03)
        // shows Memoryer broadcasting 2007 (0x07D7, `i32` room number)
        // to the whole channel when the last member exits. The row must
        // disappear without a new 2002/2003 refresh.
        let (mut session, server) = room_session(|mut stream| {
            stream
                .write_all(&encode_frame(2007, &0i32.to_le_bytes()))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session.game_mut().set_rooms(vec![
            listed_p380_room(0, "P380ROOM"),
            listed_p380_room(1, "OTHER"),
        ]);
        paint_until(&mut session, |session| {
            session.game.listed_rooms().len() == 1
        });
        server.join().unwrap();
        let titles: Vec<&str> = session
            .game
            .listed_rooms()
            .iter()
            .map(|room| room.title.as_str())
            .collect();
        assert_eq!(titles, vec!["OTHER"]);
    }

    /// Pixel arm of #621: the observer's painted Room list must match
    /// a list that never had the disbanded row. The wire handler and
    /// the row state were pinned in #380; this guard reads the frame
    /// the live chain `Shell::run` -> `OnlineSession::paint` ->
    /// `poll_wire` -> `poll_lobby_chat` -> `apply_room_removed`
    /// produces. No exe frame pair was captured (`#380` close
    /// comment), so the live before/after capture stays OPEN.
    ///
    /// Ticket #544 live chain (docs/live-path-reachability.md):
    ///   main.rs::run_play -> OnlineSession::paint (Channel/Room) ->
    ///   poll_lobby_chat -> drain_lobby_chat -> apply_lobby_frame ->
    ///   apply_room_removed -> ScreenGame::remove_room -> paint_channel.
    /// Wire pin: `S2C 2007 len=8 hex=00000000` from the WAVE 1 leg2 tap
    /// `e326e3f7…`, with the observer's 2003 poll `d4fa3dc6…`.
    #[test]
    fn observer_paint_drops_the_row_named_by_the_2007_disband_broadcast() {
        let (mut session, server) = room_session(|mut stream| {
            stream
                .write_all(&encode_frame(2007, &0i32.to_le_bytes()))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session.game_mut().set_rooms(vec![
            listed_p380_room(0, "P380ROOM"),
            listed_p380_room(1, "OTHER"),
        ]);
        let before = painted_game_frame(&mut session);
        paint_until(&mut session, |session| {
            session.game.listed_rooms().len() == 1
        });
        let after = painted_game_frame(&mut session);

        let (mut survivor_only, survivor_server) = room_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        survivor_only
            .game_mut()
            .set_rooms(vec![listed_p380_room(1, "OTHER")]);
        let expected = painted_game_frame(&mut survivor_only);
        assert_ne!(before, after, "the disband changed the painted list");
        assert_eq!(
            after, expected,
            "the painted list matches the survivor-only list"
        );
        server.join().unwrap();
        survivor_server.join().unwrap();
    }

    /// Opposite arm: a 3007 leave event is not a disband, so the
    /// observer's paint keeps painting both rows. Same live chain as
    /// the disband guard, with the non-removing opcode from the pinned
    /// two-client probe (`~/scratch/p380-wire-probe.log`).
    #[test]
    fn observer_paint_keeps_the_row_when_only_a_3007_leave_event_arrives() {
        let (mut session, server) = room_session(|mut stream| {
            stream
                .write_all(&encode_frame(3007, &[1, 0, 0, 0, 0, 0, 0, 0]))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session.game_mut().set_rooms(vec![
            listed_p380_room(0, "P380ROOM"),
            listed_p380_room(1, "OTHER"),
        ]);
        let before = painted_game_frame(&mut session);
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        let after = painted_game_frame(&mut session);
        assert_eq!(
            before, after,
            "the non-disband leave event leaves the painted rows intact"
        );
        assert_eq!(session.game.listed_rooms().len(), 2);
        server.join().unwrap();
    }

    /// Ticket #506: the channel-observer arm of the 2005 fold. A peer
    /// that has not sent 1004 gets no 2005 on the real Channel dialog
    /// (boot-1 tap `620be2ec…81a9f`, frame series `9ab9ffcc…9d60`), so
    /// this guard drives the broadcast through a scripted gateway and
    /// pins only what the pinned body proves: the Room-list row is
    /// staged for the following Room-board paint, while the Channel
    /// board itself gains no invented row. Live Channel-dialog 2005
    /// paint stays OPEN with that tap as the named capture.
    ///
    /// Live caller chain: `main.rs::run_play` -> `OnlineSession::paint`
    /// -> `poll_wire` -> `poll_lobby_chat` (Channel) ->
    /// `drain_lobby_chat` -> `apply_lobby_frame` -> `apply_room_created`
    /// -> `ScreenGame::upsert_room` -> Room-board paint.
    /// Bodies: `~/scratch/evidence-room/row-state-3/exe/wire/room-stage.log`
    /// (02:52:09, `30cac4c5…` row frame) and
    /// `~/scratch/evidence-room/exe-master-locked-3/exe/wire/room-stage.log`
    /// (03:31:26). Both were seen by a 1004-bearing observer at Room.
    #[test]
    fn channel_observer_2005_stages_the_room_row_without_repainting_the_channel_board() {
        use o2jam_net::RoomCreated;
        let pinned_bodies = [
            ("00000000524f4f4d455655000100000163000000", "ROOMEVU"),
            (
                "000000004c4f434b4544505331323334000200000000000000",
                "LOCKEDPS1234",
            ),
        ];
        for (body_hex, title) in pinned_bodies {
            let body = hex_bytes(body_hex);
            let created = RoomCreated::decode_body(&body).unwrap();
            assert_eq!(created.title, title, "pinned 2005 body");

            let (mut session, server) = channel_session(move |mut stream| {
                stream.write_all(&encode_frame(2005, &body)).unwrap();
                assert_quiet(&mut stream);
            });
            session.game_mut().set_channels(vec![entry_channel()]);
            let before = painted_game_frame(&mut session);

            paint_until(&mut session, |session| {
                session.game.listed_rooms().len() == 1
            });
            let after = painted_game_frame(&mut session);
            assert_eq!(
                before, after,
                "the Channel dialog paints no room row from a 2005"
            );
            assert_eq!(session.game.listed_rooms()[0].title, title);
            assert_eq!(session.game.channel_rows().len(), 1, "1003 rows stay");

            session.game.menu_key(MenuKey::Confirm, true);
            assert_eq!(session.game.screen(), Screen::Room);
            let painted = painted_game_frame(&mut session);
            let mut expected_room = ScreenGame::new();
            expected_room.menu_key(MenuKey::Back, true);
            expected_room.click_planet(136, 88);
            expected_room.menu_key(MenuKey::Confirm, true);
            expected_room.set_rooms(vec![listed_room_from_created(created)]);
            let mut expected_buf = vec![0u32; 800 * 600];
            expected_room.paint(&mut o2jam_render::Frame {
                buf: &mut expected_buf,
                w: 800,
                h: 600,
            });
            assert_eq!(
                painted, expected_buf,
                "the staged 2005 row paints like a 2003 row"
            );
            server.join().unwrap();
        }
    }

    /// Ticket #508: a join sends 3004 to the master and 3003 to the
    /// joiner, but never 2009 to a channel observer. The evidence-room
    /// tap (`~/scratch/evidence-room/row-state-3/`) shows the observer
    /// frame still `(1/8)` after the pinned 3004 body arrived; the
    /// 1004-bearing probe `0c917c9f…7299d` saw no 2009 either. So this
    /// guard pins the exe behavior: the observer's local count must not
    /// move without a 2009 frame.
    ///
    /// Live caller chain: `main.rs::run_play` -> `OnlineSession::paint`
    /// -> `poll_wire` -> `poll_lobby_chat` (Channel) ->
    /// `drain_lobby_chat` -> `apply_lobby_frame` -> skip -> no row write.
    #[test]
    fn channel_observer_join_broadcast_does_not_mutate_the_local_row_count() {
        let join = hex_bytes(EVIDENCE_ROOM_JOIN_BROADCAST_HEX);
        let (mut session, server) = channel_session(move |mut stream| {
            stream.write_all(&encode_frame(3004, &join)).unwrap();
            assert_quiet(&mut stream);
        });
        let channel = o2jam_net::ChannelState {
            gateway_id: 0,
            channel_id: 0,
            capacity: 8,
            population: 1,
            active: true,
        };
        session.game_mut().set_channels(vec![channel]);
        session
            .game_mut()
            .set_rooms(vec![listed_p380_room(0, "P379ROOM")]);
        let before = painted_game_frame(&mut session);
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        let after = painted_game_frame(&mut session);

        assert_eq!(before, after, "no count repaint from a join event");
        assert_eq!(session.game.channel_rows(), &[channel]);
        assert_eq!(session.game.listed_rooms()[0].user_count, 1);
        server.join().unwrap();
    }

    /// The named no-op for the opcode the exe never sends to an
    /// observer: even a scripted 2009 frame leaves the row count alone.
    /// The body here is synthetic (no exe body was captured), so this
    /// pins only the absence of a local mutation, not a wire layout.
    #[test]
    fn hypothetical_2009_frame_is_a_no_op_on_the_channel_observer() {
        let (mut session, server) = channel_session(|mut stream| {
            stream
                .write_all(&encode_frame(2009, &[0, 0, 0, 0, 8, 0, 0, 0]))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session
            .game_mut()
            .set_rooms(vec![listed_p380_room(0, "P379ROOM")]);
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        assert_eq!(session.game.listed_rooms()[0].user_count, 1);
        server.join().unwrap();
    }

    /// Ticket #512: the relay-down OK sends no game frame; the rows it
    /// was holding are revealed only after the modal closes, and the
    /// next face press starts the pinned entry cadence.
    ///
    /// Live caller chain: boot relay probe ->
    /// `stage_channel_list_after_relay_gate` ->
    /// `hold_channel_rows_for_relay_notice` -> `enter_screen(Channel)`
    /// -> `open_relay_down_dialog` -> `InputSnapshot` OK click ->
    /// `click_planet_modal` -> `dismiss_relay_down_dialog` ->
    /// `set_channels`; then face click -> `dispatch_channel_press` ->
    /// `begin_channel_login` -> `poll_channel_login` (1004, then 2000).
    ///
    /// Wire pin: relay-down boot-2 tap `a33fc8d1697a2843b874feb7034099740c0a4ec1ce1b8a354e10a40387e08913`
    /// and observe `9f942859b94d414dac78b838ce70174f5bd411ece862c5540aafa3e355f2499e`:
    /// OK at 15:37:34/37 sends no game frame; the face click at
    /// 15:37:45 sends 1004 -> 1005, then 4030/4031, 2000/2001,
    /// 2026, 2002/2003, 5028/5029, 2010/2011, 2024. The full boot
    /// reply order is guarded by `connect::tests::room_stage_gateway_sends_exe_wire_sequence`;
    /// this guard covers the interactive OK boundary and the first two
    /// sends the session owns. Pixels: t240 pack `ea5cc6d6…c563`.
    #[test]
    fn relay_down_ok_reveals_rows_first_then_starts_the_pinned_entry_wire() {
        use o2jam_net::{ChannelLoginResponse, CharacterInfo, Gender};
        use std::sync::mpsc;

        // The reveal click at (502,236) hit-tests the channel row through
        // the staged `.bnd` rects; without game data the click misses and
        // the wire order assertion has no meaning. Skip the hidden arm.
        let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Interface1.opi");
        if std::fs::metadata(&data_dir).is_err() {
            return;
        }

        let (ok_pressed, wait_for_ok) = mpsc::channel::<()>();
        let (socket_quiet, wait_for_quiet) = mpsc::channel::<()>();
        let held = vec![
            entry_channel(),
            o2jam_net::ChannelState {
                gateway_id: 0,
                channel_id: 9,
                capacity: 100,
                population: 2,
                active: true,
            },
        ];
        let (mut session, server) = channel_session(move |mut stream| {
            wait_for_ok.recv().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let mut byte = [0u8; 1];
            assert!(
                stream.read_exact(&mut byte).is_err(),
                "OK itself must send no game frame"
            );
            stream.set_read_timeout(None).unwrap();
            socket_quiet.send(()).unwrap();

            let login = read_frame(&mut stream);
            assert_eq!(login.id, 1004, "the face click sends the entry");
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, 2000, "1004 then 2000");
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
            assert_quiet(&mut stream);
        });
        crate::stage_channel_list_after_relay_gate(&mut session.game, held.clone(), true);
        assert!(session.game.relay_down_notice_pending());
        session.game.menu_key(MenuKey::Back, true);
        assert_eq!(session.game.screen(), Screen::Planet);
        assert!(session.game.enter_screen(Screen::Channel));
        assert!(session.game.relay_down_dialog_open());
        assert!(
            session.game.channel_rows().is_empty(),
            "rows held behind OK"
        );
        let before_ok = painted_game_frame(&mut session);

        session.ingest_snapshot(&InputSnapshot::click_at(400, 365));
        assert!(
            !session.game.relay_down_dialog_open(),
            "OK closes the sheet"
        );
        assert_eq!(session.game.channel_rows(), held, "OK reveals the rows");
        // A real frame loop polls between OK and the next press: the
        // sheet's dismissal must leave the socket untouched there.
        paint_frame(&mut session);
        let after_ok = painted_game_frame(&mut session);
        assert_ne!(before_ok, after_ok, "the revealed rows reach the canvas");
        ok_pressed.send(()).unwrap();
        wait_for_quiet.recv().unwrap();

        session.ingest_snapshot(&InputSnapshot::click_at(502, 236));
        paint_until(&mut session, |session| {
            session.game.myroom_character().is_some()
        });
        assert_eq!(session.game.screen(), Screen::Room);
        server.join().unwrap();
    }

    #[test]
    fn disband_broadcast_read_by_the_07da_fuse_still_removes_the_row() {
        // Live-path guard for the drain gap seen in the 13:14 live leg:
        // the #367 07DA/07DB fuse owns the socket while its node is
        // tracked. Its single-frame drain read 2007 *before* the 07DB
        // answer and used to log-and-drop it, so the row survived.
        // The live caller chain is Shell::run -> OnlineSession::paint
        // -> poll_wire -> poll_room_user_list_fuse -> deferred replay
        // -> apply_lobby_frame -> apply_room_removed.
        let (mut session, server) = room_session(|mut stream| {
            stream
                .write_all(&encode_frame(2007, &0i32.to_le_bytes()))
                .unwrap();
            stream.write_all(&encode_frame(2011, &[0; 16])).unwrap();
            // The fuse keeps polling 07DA in Room; answer those without
            // treating the extra traffic as a test failure.
            let deadline = Instant::now() + Duration::from_secs(2);
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            while Instant::now() < deadline {
                let mut header = [0u8; 4];
                if stream.read_exact(&mut header).is_err() {
                    continue;
                }
                let size = u16::from_le_bytes([header[0], header[1]]) as usize;
                let mut rest = vec![0u8; size - HEADER_LEN];
                if stream.read_exact(&mut rest).is_err() {
                    break;
                }
                let mut bytes = header.to_vec();
                bytes.extend_from_slice(&rest);
                if let Ok(frame) = decode_frame(&bytes) {
                    if frame.id == Request::GetUserList.id() {
                        stream.write_all(&encode_frame(2011, &[0; 16])).unwrap();
                    }
                }
            }
        });
        session.game.set_room_no_response_relay_down(true);
        assert!(session.game.track_room_user_list_send());
        session
            .game_mut()
            .set_rooms(vec![listed_p380_room(0, "P380ROOM")]);
        paint_until(&mut session, |session| {
            session.game.listed_rooms().is_empty()
        });
        server.join().unwrap();
        assert!(session.game.listed_rooms().is_empty());
    }

    #[test]
    fn non_last_leave_keeps_the_room_row_for_the_remaining_peer() {
        // The same probe's 13:05:01 leg: guest leaves while the host
        // stays, and the following 2003 still lists P380ROOM. Only a
        // 3007 leave event arrives — never 2007 — so the row survives.
        let (mut session, server) = room_session(|mut stream| {
            stream
                .write_all(&encode_frame(3007, &[1, 0, 0, 0, 0, 0, 0, 0]))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session
            .game_mut()
            .set_rooms(vec![listed_p380_room(0, "P380ROOM")]);
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.listed_rooms()[0].title, "P380ROOM");
    }

    #[test]
    fn master_leave_with_a_seated_peer_keeps_the_room_row() {
        // Probe phase 3 (13:05:06): the master leaves with one peer
        // still seated. The peer gets 3007 with `new_master_id = 1`,
        // no 2007 follows, and the room stays listed.
        let (mut session, server) = room_session(|mut stream| {
            stream
                .write_all(&encode_frame(3007, &[0, 1, 0, 0, 0, 0, 0, 0]))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session
            .game_mut()
            .set_rooms(vec![listed_p380_room(0, "P380SECOND")]);
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        assert_eq!(session.game.listed_rooms()[0].title, "P380SECOND");
    }

    /// Evidence-room lifecycle bodies (ticket #610):
    /// `~/scratch/evidence-room/row-state-6/{rust-master,exe}/wire/
    /// room-stage.log` 03:15:58 / 03:16:13 carry `3007 len=8
    /// hex=0001000000000000` (peer promotion) and `2007 len=4
    /// hex=00000000` (last exit). Both must reach the staged Room
    /// board: 3007 clears the seat but keeps the row, 2007 removes it.
    #[test]
    fn evidence_room_3007_keeps_the_row_and_2007_removes_it() {
        use o2jam_net::frame::encode_frame;

        let mut room = room_with_peer("peer");
        room.slots[0].member = Some(member_on_team("master", false, 0));
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            stream
                .write_all(&encode_frame(3007, &hex_bytes("0001000000000000")))
                .unwrap();
            assert_quiet(&mut stream);
        });
        assert!(session.game.staged_waiting_member(0).is_some());
        paint_until(&mut session, |s| s.game.staged_waiting_member(0).is_none());
        session
            .game_mut()
            .set_rooms(vec![listed_p380_room(0, "P380SECOND")]);
        assert!(session.game_mut().enter_screen(Screen::Room));
        paint_frame(&mut session);
        assert_eq!(
            session.game.listed_rooms().len(),
            1,
            "3007 is not a disband"
        );
        server.join().unwrap();

        let (mut session, server) = room_session(|mut stream| {
            stream
                .write_all(&encode_frame(2007, &hex_bytes("00000000")))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session
            .game_mut()
            .set_rooms(vec![listed_p380_room(0, "P380SECOND")]);
        paint_until(&mut session, |s| s.game.listed_rooms().is_empty());
        assert_eq!(session.screen(), Screen::Room);
        server.join().unwrap();
    }

    #[test]
    fn pending_join_skips_interleaved_broadcasts() {
        // Live servers interleave broadcasts (our own RoomCreated echo
        // included) ahead of the 3003 answer; the join must skip them
        // instead of failing the exchange.
        use o2jam_net::{JoinRoom, JoinRoomResponse};
        let (mut session, server) = room_session(|mut stream| {
            let broadcast = o2jam_net::frame::encode_frame(2005, &[]);
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

    /// Two-song catalog for the #535 echo guard: the picked song 758
    /// runs at 180 BPM while the room still echoes song 757 at 124.
    fn two_tempo_catalog_blob() -> Vec<u8> {
        let record = |id: u32, bpm: f32, title: &[u8; 5]| {
            let mut bytes = vec![0u8; 300];
            bytes[8..11].copy_from_slice(b"ojn");
            bytes[4..8].copy_from_slice(&id.to_le_bytes());
            bytes[20..24].copy_from_slice(&bpm.to_le_bytes());
            bytes[112..117].copy_from_slice(title);
            // `OjnListEntry::is_song` admits a row only when it names
            // an `o2maNNN.ojm` chart; without it the catalog BPM map
            // stays empty and the echo guard compares two `-0-`
            // captions instead of two tempos (#553).
            let chart = format!("o2ma{id}.ojm");
            bytes[240..240 + chart.len()].copy_from_slice(chart.as_bytes());
            bytes
        };
        let mut catalog = record(757, 124.0, b"First");
        catalog.extend_from_slice(&record(758, 180.0, b"Secon"));
        catalog
    }

    /// Caption rects for the #535 echo guard: one cover opens the
    /// staged waiting paint path, BPM and SPEED_NUM carry the ink.
    fn waiting_caption_placements() -> Vec<o2jam_shell::avatar::ScenePlacement> {
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        let placement = |id: &str, x: i32, y: i32, w: u32, h: u32| ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code: 0,
            sprite: None,
            rect: Rect { x, y, w, h },
        };
        vec![
            placement("IDC_IMAGE_SWAIT_COVERAVATAR1", 28, 53, 141, 175),
            placement("IDC_TEXT_SWAIT_BPM", 620, 60, 60, 20),
            placement("IDC_IMAGE_SWAIT_SPEED_NUM", 681, 378, 54, 35),
        ]
    }

    #[test]
    fn a_4001_room_echo_keeps_the_staged_pick_on_bpm_and_speed_captions() {
        // Ticket #553 (master and peer captions follow the room echo):
        // the master half is this guard; the peer half is
        // `a_joining_peer_keeps_the_master_seated_across_a_song_and_speed_echo`,
        // which pins the same 4001 into the session mirror. Both read
        // `ScreenGame::waiting_song`/`waiting_level` at paint time.
        // Ticket #535 live chain: `main.rs::run_play` -> `OnlineSession::paint`
        // (Waiting) -> `poll_waiting_chat` -> `drain_waiting_chat` ->
        // `apply_waiting_music_changed` -> `ScreenGame::set_room_music` ->
        // `paint_at` -> `paint_waiting` -> `paint_waiting_staged` ->
        // `paint_waiting_bpm` / `paint_waiting_speed`. A staged pick
        // (758, 180 BPM, speed 6) must survive an echo naming song 757.
        let catalog = two_tempo_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&catalog).unwrap();
        let placements = waiting_caption_placements();

        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            let echo = WaitingMusicChanged {
                music_id: 757,
                difficulty: 0,
                speed: 2,
            };
            stream.write_all(&echo.encode()).unwrap();
            let state = read_frame(&mut stream);
            assert_eq!(state.id, Request::UpdateMusicState.id());
            assert_quiet(&mut stream);
        });
        session.set_installed_music(Some(o2jam_file::installed_music(Vec::<String>::new())));
        session.set_song_catalog(&entries);
        session
            .game_mut()
            .set_selected_music(o2jam_net::SetRoomMusic {
                music_id: 758,
                difficulty: 2,
                speed: 6,
            });
        let figures = std::collections::HashMap::new();
        session
            .game_mut()
            .load_scene(Screen::Waiting, placements.clone(), figures.clone());

        paint_until(&mut session, |session| {
            session.game.room_music_id() == Some(757)
        });
        server.join().unwrap();

        // A room that already echoes the pick's song and speed is the
        // reference: both caption rects must carry identical pixels.
        let mut reference = ScreenGame::new();
        reference.menu_key(MenuKey::Back, true);
        reference.click_planet(136, 88);
        reference.menu_key(MenuKey::Confirm, true);
        let mut room = joined_snapshot();
        room.music_id = 758;
        room.speed = 6;
        reference.set_joined_room(room);
        reference.menu_key(MenuKey::Confirm, true);
        assert_eq!(reference.screen(), Screen::Waiting);
        reference.set_song_catalog(&entries);
        reference.load_scene(Screen::Waiting, placements.clone(), figures);

        let mut actual_buf = vec![0u32; 800 * 600];
        let mut reference_buf = vec![0u32; 800 * 600];
        session.game.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut actual_buf,
                w: 800,
                h: 600,
            },
            0,
        );
        reference.paint_at(
            &mut o2jam_render::Frame {
                buf: &mut reference_buf,
                w: 800,
                h: 600,
            },
            0,
        );
        for placement in placements {
            assert_eq!(
                o2jam_render::pixels_in_rect(&actual_buf, 800, placement.rect),
                o2jam_render::pixels_in_rect(&reference_buf, 800, placement.rect),
                "{} must carry the staged pick after the 4001 echo",
                placement.id
            );
        }
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

    /// Matched #355 start shape: a joined guest with a room song, a
    /// staged catalog, no local pick, and an empty-body 4011 (the
    /// live-lane pin). The exe's cover is the selected chart's OJN
    /// JPEG; the guest must stage it through the same seam the
    /// native replay path uses before Loading paints.
    ///
    /// Live caller chain: `main.rs::run_online` ->
    /// `assemble_online_session` -> `run_windowed_session` ->
    /// `Shell::paint_backbuffer` -> `OnlineSession::paint` ->
    /// `poll_wire` -> `poll_waiting_chat` -> `poll_start` ->
    /// `OnlineSession::stage_guest_room_song` ->
    /// `ScreenGame::set_selected_music` -> `resolve_song_cover` ->
    /// `ScreenGame::stage_loading_cover` -> `paint_loading_cover`.
    /// The install holds the OJN bytes; the catalog rows are
    /// synthetic. Skip cleanly when the install is absent (CI has
    /// no game data).
    #[test]
    fn matched_guest_start_stages_the_room_song_loading_cover() {
        let chart_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        if !chart_path.is_file() {
            println!("skip: missing installed chart");
            return;
        }
        let catalog_blob = select_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&catalog_blob).unwrap();
        let (port, server) = fake_gateway(|mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        let mut room = guest_snapshot();
        room.music_id = 757;
        room.difficulty = 2;
        room.speed = 1;
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.set_joined_room(room.clone());
        game.set_song_catalog(&entries);
        assert!(!game.has_loading_cover());
        assert!(game.selected_music().is_none());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.joined_room = Some(room);
        session.joined_waiting = true;
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Loading);
        assert!(
            session.game.has_loading_cover(),
            "the matched guest must stage the room song's OJN cover"
        );
    }

    /// #595: the solo host's own `4010/4011` must land on the walked
    /// chart's full-bleed cover, not the planet fallback. Qualified
    /// pack `solo-gate-02` pins the same cover bytes
    /// (`7ea40df2…bf64b`) on the host's own START. This install-guarded
    /// guard walks the live chain host select fetch -> `stage_chart_bytes`
    /// -> 4011 -> `poll_start` -> `paint_loading_cover`; it skips when
    /// CI hides the archives.
    #[test]
    fn solo_host_start_paints_the_staged_song_cover_full_bleed() {
        use o2jam_net::SetRoomMusic;

        let chart_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&chart_path) else {
            println!("skip: missing installed chart");
            return;
        };
        let catalog = select_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&catalog).unwrap();
        let mut room = joined_snapshot();
        room.music_id = 757;
        room.difficulty = 2;
        room.speed = 1;
        let (mut session, server) = joined_waiting_session(room.clone(), |mut stream| {
            assert_eq!(read_frame(&mut stream).id, 4010);
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        session.game.set_song_catalog(&entries);
        session.game.set_selected_music(SetRoomMusic {
            music_id: 757,
            difficulty: 2,
            speed: 1,
        });
        session.stage_chart_bytes(&bytes, 2).unwrap();
        assert!(session.game.has_loading_cover());

        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });
        assert_eq!(session.game.screen(), Screen::Loading);
        assert!(
            session.game.has_loading_cover(),
            "the host's own START keeps the staged cover"
        );

        let expected = {
            let cover = o2jam_shell::avatar::ojn_cover_figure(&bytes).unwrap();
            let (r, g, b) = o2jam_render::expand_rgb555(cover.layer(0).unwrap().image.words[0]);
            o2jam_render::pack_xrgb(r, g, b)
        };
        let frame = painted_game_frame(&mut session);
        assert_eq!(
            frame[0], expected,
            "the first Loading frame shows the cover at the top-left"
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

    /// #581/#598 live caller chain: `Shell::paint_backbuffer` ->
    /// `OnlineSession::paint` -> `poll_wire` ->
    /// `advance_loading_gate_at`. The wall-trace pins (solo-gate-02,
    /// 10 fps watcher) put 4012 at 660.5 ms after 4011 and the first
    /// Playing frame 232-396 ms later. Synthetic instants prove the
    /// spans without sleeping; the gateway is a scripted loopback.
    #[test]
    fn paced_joined_gate_sends_4012_before_it_enters_playing() {
        use crate::loading::{LOADING_CONFIRM_AFTER_START, LOADING_PLAYING_AFTER_CONFIRM};
        use o2jam_net::{ConfirmMusicLoaded, Request};

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

        let entered = Instant::now();
        session.arm_loading_gate_at(entered);
        session.advance_loading_gate_at(
            entered + LOADING_CONFIRM_AFTER_START - Duration::from_millis(1),
        );
        assert_eq!(session.game.screen(), Screen::Loading, "hold before 4012");

        let confirmed = entered + LOADING_CONFIRM_AFTER_START;
        session.advance_loading_gate_at(confirmed);
        assert_eq!(
            session.game.screen(),
            Screen::Loading,
            "the confirm does not create Playing"
        );
        session.advance_loading_gate_at(
            confirmed + LOADING_PLAYING_AFTER_CONFIRM - Duration::from_millis(1),
        );
        assert_eq!(session.game.screen(), Screen::Loading, "hold after 4012");
        session.advance_loading_gate_at(confirmed + LOADING_PLAYING_AFTER_CONFIRM);
        assert_eq!(session.game.screen(), Screen::Playing);
        server.join().unwrap();
    }

    /// #608: a latched chart failure leaves the gate unarmed. The
    /// wall span alone must not post 4012 or hand the guest to
    /// Playing; the arm latch keeps the load honest until staging
    /// succeeds. Live chain: `Shell::paint_backbuffer` ->
    /// `OnlineSession::paint` -> `poll_wire` ->
    /// `advance_loading_gate_at`.
    #[test]
    fn unarmed_loading_gate_holds_4012_at_the_wall_span() {
        use crate::loading::{LoadingGateArm, LOADING_CONFIRM_AFTER_START};

        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });
        session.game_mut().fail_loading_gate();
        assert!(!session.game.loading_gate_ready());

        let entered = Instant::now();
        session.arm_loading_gate_at(entered);
        session.advance_loading_gate_at(
            entered + LOADING_CONFIRM_AFTER_START + Duration::from_millis(1),
        );

        assert_eq!(session.game.screen(), Screen::Loading);
        assert!(!session.loaded_sent, "unarmed gate must not post 4012");
        assert_eq!(
            session.loading_pacing.arm_state(),
            LoadingGateArm::AwaitingChart
        );
        server.join().unwrap();
    }

    /// #608 wire correlate: the observable arm latch opens the 4012
    /// send, and the gateway's S2C 4013 answer follows that send. The
    /// exe arm word itself still needs the stopped-state read; this
    /// guard pins our gate against the P3/P13 wire order.
    #[test]
    fn chart_arm_opens_4012_before_the_4013_answer() {
        use crate::loading::{LoadingGateArm, LOADING_CONFIRM_AFTER_START};

        let (loaded_send, loaded_recv) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(guest_snapshot(), move |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            let loaded = read_frame(&mut stream);
            loaded_send.send(loaded.id).unwrap();
            stream.write_all(&encode_frame(4013, &[1, 1])).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });

        let entered = Instant::now();
        session.arm_loading_gate_at(entered);
        assert_eq!(
            session.loading_pacing.arm_state(),
            LoadingGateArm::AwaitingChart
        );
        session.advance_loading_gate_at(entered + LOADING_CONFIRM_AFTER_START);

        assert_eq!(
            loaded_recv.recv_timeout(Duration::from_secs(4)).unwrap(),
            4012
        );
        assert!(matches!(
            session.loading_pacing.arm_state(),
            LoadingGateArm::ConfirmPosted { .. }
        ));
        server.join().unwrap();
    }

    /// #725 live chain: `run_windowed_session` -> `Shell::about_to_wait`
    /// -> `GameLoop::idle_repaint_interval` -> `OnlineSession::paint` ->
    /// `poll_wire` -> `advance_loading_gate_at`. A guest gets no Confirm
    /// after START, so the wall-paced gate needs the window to repaint
    /// while the cover is up. Without that heartbeat the peer stays on
    /// Loading and never posts 4014 (#723 boot 1).
    #[test]
    fn guest_loading_gate_keeps_the_window_repainting_after_start_broadcast() {
        use super::SESSION_POLL_HEARTBEAT;

        let (loaded_send, loaded_recv) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(guest_snapshot(), move |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            let _ = loaded_send.send(first_opcode(&mut stream));
        });
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });
        assert_eq!(
            session.idle_repaint_interval(),
            Some(SESSION_POLL_HEARTBEAT),
            "the armed joined gate must keep the window repainting"
        );
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Playing
        });
        assert_eq!(session.game.screen(), Screen::Playing);
        assert_eq!(
            loaded_recv.recv_timeout(Duration::from_secs(4)).unwrap(),
            Some(Request::ConfirmMusicLoaded.id())
        );
        server.join().unwrap();
    }

    /// #725 headless two-client walk: one master and one joined peer, each
    /// on its own scripted gateway, painted only when its own idle
    /// heartbeat says the window would repaint. The master driver presses
    /// Confirm on its Loading cover; the peer gets no input after START,
    /// so both seats must reach Playing from the live gate alone.
    #[test]
    fn two_client_gateway_walk_reaches_playing_on_master_and_peer_without_peer_input() {
        let (started_send, started_recv) = mpsc::channel();
        let (master_loaded_send, master_loaded_recv) = mpsc::channel();
        let (peer_loaded_send, peer_loaded_recv) = mpsc::channel();
        let (mut master, master_server) =
            joined_waiting_session(joined_snapshot(), move |mut stream| {
                assert_eq!(read_frame(&mut stream).id, 4010);
                stream.write_all(&encode_frame(4011, &[])).unwrap();
                started_send.send(()).unwrap();
                let _ = master_loaded_send.send(first_opcode(&mut stream));
            });
        let (mut peer, peer_server) =
            joined_waiting_session(guest_snapshot(), move |mut stream| {
                started_recv.recv().unwrap();
                stream.write_all(&encode_frame(4011, &[])).unwrap();
                let _ = peer_loaded_send.send(first_opcode(&mut stream));
            });

        master.menu_key(MenuKey::Confirm, true);
        let first_paint = Instant::now();
        let mut master_due = Some(first_paint);
        let mut peer_due = Some(first_paint);

        let deadline = Instant::now() + Duration::from_secs(6);
        while (master.game.screen() != Screen::Playing || peer.game.screen() != Screen::Playing)
            && Instant::now() < deadline
        {
            let now = Instant::now();
            paint_on_heartbeat(&mut master, &mut master_due, now);
            if master.game.screen() == Screen::Loading {
                master.menu_key(MenuKey::Confirm, true);
                master_due = Some(now);
            }
            paint_on_heartbeat(&mut peer, &mut peer_due, now);
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(master.game.screen(), Screen::Playing, "master never played");
        assert_eq!(
            peer.game.screen(),
            Screen::Playing,
            "peer parked on Loading without an input-driven repaint"
        );
        assert_eq!(
            peer.idle_repaint_interval(),
            Some(Duration::from_nanos(o2jam_shell::TICK_NANOS))
        );
        for (seat, loaded) in [("master", &master_loaded_recv), ("peer", &peer_loaded_recv)] {
            assert_eq!(
                loaded.recv_timeout(Duration::from_secs(4)).unwrap(),
                Some(Request::ConfirmMusicLoaded.id()),
                "{seat} must post its own 4012"
            );
        }
        master_server.join().unwrap();
        peer_server.join().unwrap();
    }

    /// #727 live caller chain: `main.rs::run_online` ->
    /// `assemble_online_session` -> `run_windowed_session` ->
    /// `WindowHandler::about_to_wait` (gap from
    /// `OnlineSession::idle_repaint_interval`) -> `Shell::paint_backbuffer`
    /// -> `OnlineSession::paint` -> `poll_wire` -> `poll_waiting_chat` ->
    /// `poll_start` -> `stage_guest_room_song` ->
    /// `ScreenGame::set_loading_status`; then `advance_loading_gate_at`
    /// -> `enter_playing_from_loading`. No scripted `stage_armed_loading`:
    /// the joined guest must arm the shell gate from its own chart on
    /// the live paint path. Archives absent -> skip cleanly (CI has no
    /// game data).
    #[test]
    fn live_paced_guest_reaches_playing_without_scripted_loading_gate_arming() {
        use o2jam_net::Request;

        let chart_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        if !chart_path.is_file() {
            println!("skip: missing installed chart");
            return;
        }
        let catalog_blob = select_catalog_blob();
        let catalog = o2jam_file::parse_ojn_list(&catalog_blob).unwrap();
        let mut room = guest_snapshot();
        room.music_id = 757;
        room.difficulty = 2;
        room.speed = 1;

        let (loaded_send, loaded_recv) = mpsc::channel();
        let (port, server) = fake_gateway(move |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            let _ = loaded_send.send(first_opcode_with_timeout(
                &mut stream,
                Duration::from_secs(60),
            ));
        });
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.set_joined_room(room.clone());
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.set_song_catalog(&catalog);
        session.joined_room = Some(room);
        session.joined_waiting = true;
        session.set_silent_audio(true);
        session.attach_initial_audio(false);
        session.game.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);

        let mut due = Some(Instant::now());
        let deadline = Instant::now() + Duration::from_secs(60);
        while session.game.screen() != Screen::Playing && Instant::now() < deadline {
            let now = Instant::now();
            paint_on_heartbeat(&mut session, &mut due, now);
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(
            session.game.screen(),
            Screen::Playing,
            "the live-style guest parked on the unarmed Loading gate"
        );
        assert_eq!(
            loaded_recv.recv_timeout(Duration::from_secs(4)).unwrap(),
            Some(Request::ConfirmMusicLoaded.id())
        );
        server.join().unwrap();
    }

    /// #602: a blocked 4012 leaves the gate in Loading and latches no
    /// send. The joined snapshot has no staged speed after the join
    /// mirror is cleared, so `send_loading_confirm` refuses to invent
    /// one; the next Confirm can retry when a speed is staged. Live
    /// chain: boot -> Waiting START -> 4011 -> `poll_start` -> Loading
    /// -> `menu_key(Confirm)` -> `confirm_loading` -> `send_loading_confirm`.
    #[test]
    fn blocked_loaded_confirm_holds_the_solo_gate_in_loading() {
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            assert_eq!(read_frame(&mut stream).id, 4010);
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        session.last_selection = None;
        session.joined_room = None;

        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });
        assert_eq!(session.game.screen(), Screen::Loading);

        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(
            session.game.screen(),
            Screen::Loading,
            "a refused 4012 must not enter Playing"
        );
        assert!(!session.loaded_sent, "no send is latched on a failure");
        server.join().unwrap();
    }

    /// #614: the qualified guest gate ignores Return, Escape and
    /// canvas clicks (`docs/wine-boot.md` focus pin, pack P15
    /// `8773f5cc…`). Live chain: `run_windowed_session` -> `Shell`
    /// -> `GameLoop::ingest_snapshot` ->
    /// `OnlineSession::ingest_snapshot`. The wall-paced gate, not
    /// player input, posts 4012.
    #[test]
    fn joined_loading_ignores_return_escape_and_clicks_on_the_live_input_path() {
        use o2jam_shell::InputEdge;

        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });

        let mut ignored = InputSnapshot::new();
        for key in [MenuKey::Confirm, MenuKey::Back] {
            ignored.push_edge(InputEdge::Menu { key, down: true });
            ignored.push_edge(InputEdge::Menu { key, down: false });
        }
        ignored.push_edge(InputEdge::PointerDown { x: 400, y: 300 });
        ignored.push_edge(InputEdge::PointerUp { x: 400, y: 300 });
        session.ingest_snapshot(&ignored);

        assert_eq!(session.game.screen(), Screen::Loading);
        assert!(!session.loaded_sent, "no input may post 4012");
        server.join().unwrap();
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

    /// #730 terminal arm: the live caller chain is Waiting Confirm →
    /// START 4010 → 4011 → Loading Confirm → 4012 → Playing, then the
    /// window's `GameLoop::paint` (never the explicit-clock `paint_at`
    /// shortcut used by `play_one_note_to_result`). The guards below
    /// walk that chain and read the 4016 the session posts on Result.
    mod live_terminal_state {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::{Arc, Mutex};

        use o2jam_audio::{AudioBackend, NullBackend, PcmSamples};
        use o2jam_net::SubmitScore;

        use super::*;

        /// A clock-only sink whose pushed key-sound frames are the only
        /// song position it reports. This is the 7.55 probe's stall:
        /// Playing cues no scene BGM, so once the last key sample drains
        /// the device clock parks short of the chart's 1.5 s outro.
        struct StalledKeyClock {
            inner: NullBackend,
            pushed_frames: Arc<AtomicU64>,
        }

        impl AudioBackend for StalledKeyClock {
            fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), o2jam_audio::AudioError> {
                let frames = u64::try_from(pcm.samples.len() / usize::from(pcm.channels.max(1)))
                    .unwrap_or(0);
                self.pushed_frames.fetch_add(frames, Ordering::Relaxed);
                self.inner.play_pcm(pcm)
            }

            fn samples_played(&self) -> u64 {
                self.pushed_frames.load(Ordering::Relaxed)
            }
        }

        /// Attach one short key sample and expose the sink's counter so
        /// a guard can prove the sample clock stayed short of the end.
        fn attach_stalled_key_bank(session: &mut OnlineSession) -> Arc<AtomicU64> {
            let pushed_frames = Arc::new(AtomicU64::new(0));
            let backend = StalledKeyClock {
                inner: NullBackend::new(),
                pushed_frames: Arc::clone(&pushed_frames),
            };
            let bank = std::collections::HashMap::from([(
                7u32,
                PcmSamples {
                    samples: vec![0i16; 4_410],
                    sample_rate: 44_100,
                    channels: 1,
                },
            )]);
            session.game_mut().with_audio(Box::new(backend), bank);
            pushed_frames
        }

        /// Drive the joined session from Waiting to Playing over the
        /// wire, with no staged loading-gate shortcut past 4012.
        fn walk_joined_to_playing(session: &mut OnlineSession) {
            session.menu_key(MenuKey::Confirm, true);
            paint_until(session, |session| session.game.screen() == Screen::Loading);
            assert_eq!(session.game.screen(), Screen::Loading);
            session.menu_key(MenuKey::Confirm, true);
            assert_eq!(session.game.screen(), Screen::Playing);
        }

        /// Rebase the wall floor past the chart outro without a sleep,
        /// then spend the remaining frames on the live paint path.
        fn spend_outro(session: &mut OnlineSession) {
            session
                .game_mut()
                .set_playing_wall_elapsed(Duration::from_millis(2_000));
            paint_frame(session);
            assert_eq!(session.game.screen(), Screen::Result, "chart never ended");
        }

        /// Collect the gateway's frames after 4012 until the client
        /// drops at guard end. Unlike
        /// `assert_quiet`, this records the tally instead of forbidding
        /// all traffic: 4014 jam/life ticks share the socket.
        fn read_frames_after_loaded(stream: &mut TcpStream) -> Vec<Frame> {
            assert_eq!(read_frame(stream).id, Request::StartGame.id());
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_eq!(read_frame(stream).id, Request::ConfirmMusicLoaded.id());
            // The chart-end guard rebases the wall floor past the
            // outro with no frames in flight; read until the client
            // drops instead of treating a quiet socket as the end.
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut frames = Vec::new();
            loop {
                let mut header = [0u8; HEADER_LEN];
                match stream.read_exact(&mut header) {
                    Ok(()) => {}
                    Err(_) => break,
                }
                let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
                let mut rest = vec![0u8; body_len.saturating_sub(HEADER_LEN)];
                if stream.read_exact(&mut rest).is_err() {
                    break;
                }
                let mut bytes = header.to_vec();
                bytes.extend_from_slice(&rest);
                if let Ok(frame) = decode_frame(&bytes) {
                    frames.push(frame);
                }
            }
            frames
        }

        /// First pinned 4016 body from the recorded gateway frames.
        fn recorded_tally(frames: &[Frame]) -> SubmitScore {
            let frame = frames
                .iter()
                .find(|frame| frame.id == Request::SubmitScore.id())
                .expect("Result must post C2S 4016");
            SubmitScore::decode_body(&frame.payload).expect("4016 body")
        }

        #[test]
        fn chart_end_walk_reaches_result_and_posts_4016_without_a_fixed_clock() {
            // Live caller chain: joined Waiting Confirm → 4010/4011 →
            // Loading Confirm → 4012 → Playing; the whole chart is
            // struck at its onsets, then only `GameLoop::paint` spends
            // the outro. No `paint_at`, no fixed clock, no end-song
            // shortcut. The key-sound clock parks at 300 ms while the
            // chart ends at 1,500 ms, so only the wall-time terminal
            // floor can carry the chart-end arm (#730).
            let recorded = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&recorded);
            let (mut session, server) =
                joined_waiting_session(joined_snapshot(), move |mut stream| {
                    *sink.lock().unwrap() = read_frames_after_loaded(&mut stream);
                });
            let pushed_frames = attach_stalled_key_bank(&mut session);
            walk_joined_to_playing(&mut session);
            let chart = [(0u8, 0u64), (1, 1), (2, 2)];
            session.game_mut().load_chart(
                chart
                    .iter()
                    .map(|(lane, at_ms)| (*lane, *at_ms, 7u32))
                    .collect(),
            );
            for (lane, at_ms) in chart {
                session.game_mut().strike_at(usize::from(lane), at_ms);
            }
            assert!(
                pushed_frames.load(Ordering::Relaxed) < 44_100,
                "sample clock must stall short of the chart's 1.5 s outro"
            );
            assert_eq!(session.game.screen(), Screen::Playing);

            spend_outro(&mut session);
            paint_frame(&mut session);
            drop(session);
            server.join().unwrap();

            let frames = recorded.lock().unwrap();
            let tally = recorded_tally(&frames);
            assert_eq!(tally.cool, 3, "every chart onset was struck");
            assert_eq!(tally.miss, 0);
            assert_eq!(frames.iter().filter(|f| f.id == 4016).count(), 1);
        }

        #[test]
        fn empty_life_arm_posts_the_pinned_game_over_tally_body() {
            // The exe master's empty-life leg (`result-probe-findings`
            // w7) emptied the bar 20 Misses in and posted 4016 with
            // tallies 0/0/20/0, life 0, speed 1. The chart clock is
            // nowhere near its end here, so only the life arm can
            // reach Result. Same live chain, same window paints.
            let recorded = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&recorded);
            let (mut session, server) =
                joined_waiting_session(joined_snapshot(), move |mut stream| {
                    *sink.lock().unwrap() = read_frames_after_loaded(&mut stream);
                });
            walk_joined_to_playing(&mut session);
            let chart: Vec<_> = (0..20u64).map(|index| (0u8, index * 100, 7u32)).collect();
            session.game_mut().load_chart(chart);
            // One empty strike past the last onset expires every head
            // as a Miss on the same verified advance path.
            session.game_mut().strike_at(6, 10_000);
            assert_eq!(session.game.live_play().life(), 0);
            assert_eq!(session.game.screen(), Screen::Playing);

            paint_frame(&mut session);
            assert_eq!(session.game.screen(), Screen::Result);
            paint_frame(&mut session);
            drop(session);
            server.join().unwrap();

            let frames = recorded.lock().unwrap();
            let tally = recorded_tally(&frames);
            assert_eq!(tally.cool, 0);
            assert_eq!(tally.good, 0);
            assert_eq!(tally.bad, 0);
            assert_eq!(tally.miss, 20);
            assert_eq!(tally.life, 0);
            assert_eq!(tally.speed, 1);
            assert_eq!(tally.score, 0);
            assert_eq!(tally.long_note_score, 0);
            let expected = [
                0u8, 0, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
            ];
            let frame = frames
                .iter()
                .find(|frame| frame.id == 4016)
                .expect("4016 frame");
            assert_eq!(frame.payload, expected, "pinned 24-byte 4016 body");
        }

        /// Script one gateway socket through START and loaded confirm,
        /// then record every frame until that client drops. The
        /// two-client guard needs two independent recordings, so it
        /// owns the listener instead of `fake_gateway`'s one socket.
        fn script_one_client(stream: &mut TcpStream) -> Vec<Frame> {
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            assert_eq!(read_frame(stream).id, Request::StartGame.id());
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_eq!(read_frame(stream).id, Request::ConfirmMusicLoaded.id());
            let mut frames = Vec::new();
            loop {
                let mut header = [0u8; HEADER_LEN];
                if stream.read_exact(&mut header).is_err() {
                    break;
                }
                let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
                let mut rest = vec![0u8; body_len.saturating_sub(HEADER_LEN)];
                if stream.read_exact(&mut rest).is_err() {
                    break;
                }
                let mut bytes = header.to_vec();
                bytes.extend_from_slice(&rest);
                if let Ok(frame) = decode_frame(&bytes) {
                    frames.push(frame);
                }
            }
            frames
        }

        /// A joined Waiting session connected to `port`, staged exactly
        /// like `joined_waiting_session` but with a caller-owned socket.
        fn joined_waiting_client(port: u16) -> OnlineSession {
            let mut game = ScreenGame::new();
            game.menu_key(MenuKey::Back, true);
            game.click_planet(136, 88);
            game.menu_key(MenuKey::Confirm, true);
            assert_eq!(game.screen(), Screen::Room);
            let gateway = GatewayConnection::connect(("127.0.0.1", port)).unwrap();
            let mut session = OnlineSession::new(game, gateway);
            let room = joined_snapshot();
            session.game.set_joined_room(room.clone());
            session.joined_room = Some(room);
            session.joined_waiting = true;
            session.game.menu_key(MenuKey::Confirm, true);
            assert_eq!(session.game.screen(), Screen::Waiting);
            stage_armed_loading(session.game_mut());
            session
        }

        #[test]
        fn two_scripted_clients_reach_chart_end_and_both_submit_4016() {
            // One scripted gateway, two joined clients, each with its
            // own key-sound clock that parks before the chart outro.
            // Both walk the live chain and both must leave Playing on
            // chart-end; one client's Result alone cannot satisfy the
            // server's both-submissions gate (`result-probe-findings`
            // w7). No exe, no install data: this guard runs with
            // install-root hidden.
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let recordings = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&recordings);
            let server = thread::spawn(move || {
                // One reader per socket: a single reader would block on
                // the first client's live socket and never accept the
                // second client's START.
                let mut clients = Vec::new();
                for _ in 0..2 {
                    let (mut stream, _) = listener.accept().unwrap();
                    clients.push(thread::spawn(move || script_one_client(&mut stream)));
                }
                for client in clients {
                    sink.lock().unwrap().push(client.join().unwrap());
                }
            });

            let mut first = joined_waiting_client(port);
            let mut second = joined_waiting_client(port);
            attach_stalled_key_bank(&mut first);
            attach_stalled_key_bank(&mut second);
            walk_joined_to_playing(&mut first);
            walk_joined_to_playing(&mut second);
            let chart = [(0u8, 0u64), (1, 1), (2, 2)];
            for session in [&mut first, &mut second] {
                session.game_mut().load_chart(
                    chart
                        .iter()
                        .map(|(lane, at_ms)| (*lane, *at_ms, 7u32))
                        .collect(),
                );
                for (lane, at_ms) in chart {
                    session.game_mut().strike_at(usize::from(lane), at_ms);
                }
            }
            for session in [&mut first, &mut second] {
                session
                    .game_mut()
                    .set_playing_wall_elapsed(Duration::from_millis(2_000));
            }
            paint_frame(&mut first);
            paint_frame(&mut second);
            assert_eq!(first.game.screen(), Screen::Result);
            assert_eq!(second.game.screen(), Screen::Result);
            paint_frame(&mut first);
            paint_frame(&mut second);
            drop(first);
            drop(second);
            server.join().unwrap();

            let recordings = recordings.lock().unwrap();
            assert_eq!(recordings.len(), 2, "both clients reached the gateway");
            for client_frames in recordings.iter() {
                let tally = recorded_tally(client_frames);
                assert_eq!(tally.cool, 3);
                assert_eq!(tally.miss, 0);
                assert_eq!(
                    client_frames
                        .iter()
                        .filter(|frame| frame.id == Request::SubmitScore.id())
                        .count(),
                    1,
                    "each client submits once"
                );
            }
        }
    }

    #[test]
    fn result_ui_has_no_submitter_marker_from_4017() {
        // #679: REA finds the queue's type-0x12 consumers in the two
        // Playing states, never in Result. The 0xFB1 arm checks a
        // member byte at `+0x33` and writes a member state/animation;
        // it does not name a Result marker. Keep our Result rows
        // independent of the submitter slot until a two-member frame
        // with a non-zero slot says otherwise.
        use o2jam_net::{MatchResult, ScoreCompleted, ScoreEntry, ScoreSubmission};
        assert_eq!(RESULT_UI_QUEUE_ADDRESS, 0x478d50);
        assert_eq!(RESULT_4017_PLAYING_VTABLE_ENTRIES, [0x64ae1c, 0x64b3dc]);
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
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
            stream
                .write_all(&ScoreSubmission { member_id: 3 }.encode())
                .unwrap();
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
                        level: 4,
                        experience: 3000,
                        result: MatchResult::Lose,
                        gem: 12_000,
                        cash_point: 0,
                        speed: 3,
                        long_note_score: 90,
                    },
                ],
            };
            stream.write_all(&completed.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_until(&mut session, |session| session.result_record(1).is_some());
        server.join().unwrap();

        assert_eq!(session.last_submitter_slot(), Some(3));
        let rows = session.game.result_rank_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].judgements, Some([11, 22, 33, 44]));
        assert_eq!(rows[0].score, Some(777));
        assert_eq!(rows[1].judgements, Some([50, 0, 0, 0]));
        assert_eq!(rows[1].score, Some(50_000));
        assert_eq!(rows[0].rank, None);
        assert_eq!(rows[1].rank, None);
    }

    #[test]
    fn score_submission_traces_a_valid_submitter_and_drops_slot_eight_or_more() {
        // #667: the exe's 4017 handler (file `0x15fba0`) reads one
        // member byte, drops it when the slot is not `< 8`, and
        // forwards the rest into the UI queue at `0x478d50`. We have
        // no Result consumer for that notice; this guard pins the
        // traced slot on our side of the seam, including the guard.
        use o2jam_net::{MatchResult, ScoreCompleted, ScoreEntry, ScoreSubmission};
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
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
            stream
                .write_all(&ScoreSubmission { member_id: 2 }.encode())
                .unwrap();
            stream
                .write_all(&ScoreSubmission { member_id: 9 }.encode())
                .unwrap();
            let completed = ScoreCompleted {
                room_master: 0,
                scores: vec![ScoreEntry {
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
                }],
            };
            stream.write_all(&completed.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_until(&mut session, |session| session.result_record(0).is_some());
        server.join().unwrap();
        assert_eq!(session.last_submitter_slot(), Some(2));
    }

    #[test]
    fn rank_update_traces_the_eight_place_bytes_from_the_gateway() {
        // #665: the exe's 4040 handler stores one place byte per seat
        // at `+0x217d` (handler file `0x159a60`, writer file `0x1414ad`).
        // Our gateway never emits it, so a scripted 4040 is the only
        // way to prove the decoder's eight bytes are accounted for
        // instead of silently traced and forgotten.
        use o2jam_net::{GameRankUpdate, MatchResult, ScoreCompleted, ScoreEntry};
        let places = [0u8, 0xff, 1, 0xff, 0xff, 0xff, 0xff, 0xff];
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
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
            stream
                .write_all(
                    &GameRankUpdate {
                        member_ranks: places,
                    }
                    .encode(),
                )
                .unwrap();
            let completed = ScoreCompleted {
                room_master: 0,
                scores: vec![ScoreEntry {
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
                }],
            };
            stream.write_all(&completed.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_until(&mut session, |session| session.last_rank_places().is_some());
        server.join().unwrap();
        assert_eq!(session.last_rank_places(), Some(places));
    }

    #[test]
    fn result_rank_column_ignores_the_4040_place_bytes() {
        // #680: the 4040 place bytes land at slot `+0x217d`, while the
        // rank column reads the locally computed table at `+0x10d2c`.
        // A scripted place that names seat 0 as first must not ink a
        // rank digit; the two-member replay that would confirm this on
        // the exe remains OPEN.
        use o2jam_net::{
            GameRankUpdate, MatchResult, ScoreCompleted, ScoreEntry, RESULT_RANK_COLUMN_OFFSET,
            RESULT_RANK_PLACE_OFFSET,
        };
        assert_eq!(RESULT_RANK_PLACE_OFFSET, 0x217d);
        assert_eq!(RESULT_RANK_COLUMN_OFFSET, 0x10d2c);
        let places = [0u8, 1, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
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
            stream
                .write_all(
                    &GameRankUpdate {
                        member_ranks: places,
                    }
                    .encode(),
                )
                .unwrap();
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
                        level: 4,
                        experience: 3000,
                        result: MatchResult::Lose,
                        gem: 12_000,
                        cash_point: 0,
                        speed: 3,
                        long_note_score: 90,
                    },
                ],
            };
            stream.write_all(&completed.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        play_one_note_to_result(&mut session);
        paint_until(&mut session, |session| session.result_record(1).is_some());
        server.join().unwrap();

        assert_eq!(session.last_rank_places(), Some(places));
        let rows = session.game.result_rank_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, None, "no rank digit from a 4040 place");
        assert_eq!(rows[1].rank, None, "no rank digit from a 4040 place");
        assert_eq!(rows[0].score, Some(777));
        assert_eq!(rows[1].score, Some(50_000));
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

    /// #365 read side: two active 4018 rows must ink both Result rows.
    /// The scripted gateway drives the real session from Waiting to
    /// Result, then hands the tally triple two member records whose
    /// digits differ from the local score (cool 1, score 200).
    ///
    /// Live caller chain: `main.rs::run_play` ->
    /// `run_windowed_session` -> `OnlineSession::paint` -> `poll_tally`
    /// -> `drain_tally_answers` -> `apply_score_completed` ->
    /// `stage_result_records` -> `ScreenGame::stage_result_member_record`
    /// -> `ScreenGame::result_rank_rows` -> `paint_result_rank_rows`
    /// -> `paint_result_rank_row`.
    #[test]
    fn result_rows_read_the_two_staged_4018_member_records() {
        use o2jam_net::{MatchResult, ScoreCompleted, ScoreEntry};

        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        room.slots[2].member = Some(member_on_team("idle", true, 2));
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
                        level: 4,
                        experience: 3000,
                        result: MatchResult::Lose,
                        gem: 12_000,
                        cash_point: 0,
                        speed: 3,
                        long_note_score: 90,
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
                        score: 999_999,
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
        paint_until(&mut session, |session| session.result_record(1).is_some());
        server.join().unwrap();

        let rows = session.game.result_rank_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].judgements, Some([11, 22, 33, 44]));
        assert_eq!(rows[0].combos, Some((55, 66)));
        assert_eq!(rows[0].score, Some(777));
        assert_eq!(rows[0].bonus, Some(88));
        assert_eq!(rows[0].speed, Some(2));
        assert_eq!(rows[1].judgements, Some([50, 0, 0, 0]));
        assert_eq!(rows[1].combos, Some((200, 9)));
        assert_eq!(rows[1].score, Some(50_000));
        assert_eq!(rows[1].bonus, Some(300));
        assert_eq!(rows[1].speed, Some(3));
        // The exe discards `active == 0` rows: no record reaches the
        // third seat, so its digits stay unpainted even though the
        // wire named a huge score there.
        assert_eq!(rows[2].score, None);
        assert_eq!(rows[2].judgements, None);
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

    /// Seated chat round trip from the CP05 tap (ticket #600):
    /// `~/scratch/close-wr/full01/tap.log` 15:43:51 carries
    /// `C2S 3011 len=17 hex=68656c6c6f68656c6c6f323931` and
    /// `S2C 3012 len=25 hex=7465737465720068656c6c6f68656c6c6f32393100`.
    /// Caller chain: boot → Room → Waiting → chat draft → Confirm →
    /// gateway 3011 → inbound 3012 → paint → staged history row.
    #[test]
    fn waiting_chat_round_trip_matches_the_cp05_tap() {
        use o2jam_net::frame::encode_frame;
        let tapped_answer = b"tester\x00hellohello291\x00";
        let (mut session, server) = waiting_session(|mut stream| {
            let sent = read_frame(&mut stream);
            assert_eq!(sent.id, 3011);
            assert_eq!(sent.payload, b"hellohello291".to_vec());
            stream
                .write_all(&encode_frame(3012, tapped_answer))
                .unwrap();
            assert_quiet(&mut stream);
        });
        session.game.set_chat_input("hellohello291");
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |s| s.game.waiting_chat().len() == 1);
        server.join().unwrap();
        let row = &session.game.waiting_chat()[0];
        assert_eq!(row.sender, "tester");
        assert_eq!(row.text, "hellohello291");
        assert!(!row.admin);
    }

    /// #578 wire arm (CP34): the joined peer's chat reaches the master.
    /// Pin: `~/scratch/close-wr/wire-only/peer.log` 15:51:37
    /// `S2C 3012 len=16 hex=74657374657232005749524532393101` -
    /// sender `tester2`, text `WIRE291`, seat 1. Live caller chain:
    /// `OnlineSession::paint` -> `poll_waiting_chat` ->
    /// `drain_waiting_chat` -> `apply_waiting_user_message` ->
    /// `ScreenGame::push_waiting_user_message` -> `paint_waiting_chat`.
    /// The guest's send half is pinned by #600; this is the other seat's
    /// receive half. No install-root read (synthetic gateway).
    #[test]
    fn peer_chat_from_cp34_lands_as_a_master_waiting_row() {
        let cp34_echo = b"tester2\x00WIRE291\x01";
        let (mut session, server) =
            joined_waiting_session(room_with_peer("tester2"), move |mut stream| {
                stream.write_all(&encode_frame(3012, cp34_echo)).unwrap();
                assert_quiet(&mut stream);
            });
        paint_until(&mut session, |session| {
            session.game.waiting_chat().len() == 1
        });
        server.join().unwrap();
        let rows = session.game.waiting_chat();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sender, "tester2");
        assert_eq!(rows[0].text, "WIRE291");
        assert!(!rows[0].admin, "3012 is a player row");
    }

    /// #578 wire arm (CP33): the peer's READY echo reaches the master's
    /// START gate. Pin: `~/scratch/close-wr/wire-only/peer.log` 15:51:32
    /// `ASYNC id=4009 len=2 hex=0101`. Live caller chain:
    /// `OnlineSession::paint` -> `poll_waiting_chat` ->
    /// `drain_waiting_chat` -> `apply_ready_echo` ->
    /// `ScreenGame::set_member_ready` -> `start_gate_verdict`. The two
    /// client frame pair stays OPEN; this gate pins the wire leg.
    #[test]
    fn peer_ready_echo_from_cp33_opens_the_master_start_gate() {
        let mut room = room_with_peer("tester2");
        let peer = room.slots[1].member.as_mut().expect("seated peer");
        peer.ready = false;
        peer.team = 1;
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            stream.write_all(&encode_frame(4009, &[1, 1])).unwrap();
            assert_quiet(&mut stream);
        });
        assert_eq!(
            session.game.start_gate_verdict(),
            StartGateVerdict::PeerNotReady
        );
        paint_until(&mut session, |session| {
            session
                .game
                .staged_waiting_member(1)
                .is_some_and(|member| member.ready)
        });
        server.join().unwrap();
        assert_eq!(
            session.game.start_gate_verdict(),
            StartGateVerdict::Startable
        );
    }

    #[test]
    fn master_chat_send_lands_as_a_waiting_row_from_the_3012_echo() {
        // Ticket #567 live chain (docs/live-path-reachability.md):
        //   main.rs::run_play -> OnlineSession::menu_key (Waiting Confirm)
        //   -> flush_chat_action -> SendWaitingMessage 3011 ->
        //   drain_waiting_chat -> apply_waiting_user_message 3012 ->
        //   ScreenGame::push_waiting_user_message -> paint_waiting_chat.
        // Wire pin: `full01/tap.log` 15:43:51 `C2S 3011 len=17
        // hex=68656c6c6f68656c6c6f323931` -> `S2C 3012`, with the
        // CP05 master chat-row pack `4899ad2a…`.
        use o2jam_net::rooms::WaitingUserMessage;
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let sent = read_frame(&mut stream);
            assert_eq!(sent.id, 3011);
            assert_eq!(sent.payload, b"hellohello291".to_vec());
            let echo = WaitingUserMessage {
                sender: "master".to_owned(),
                content: b"hellohello291".to_vec(),
                member_id: 0,
            };
            stream.write_all(&echo.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        assert!(session.game.is_room_master(), "master role for this leg");
        session.game.set_chat_input("hellohello291");
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.waiting_chat().len() == 1
        });
        server.join().unwrap();
        let rows = session.game.waiting_chat();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sender, "master");
        assert_eq!(rows[0].text, "hellohello291");
        assert!(!rows[0].admin, "3012 is a player row");
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

    /// Bytes pushed to the master by Memoryer at 12:28:14.751
    /// (`~/scratch/p379-wire-probe.log`): 3004 `UserJoinWaiting`, seat 1
    /// named tester2. Kept as the capture, not a hand-built fixture.
    const LIVE_JOIN_BROADCAST_HEX: &str = concat!(
        "017465737465723200010000000172020000010000100000001e000000780100007f0100",
        "008501000000000000000000000000000000000000000000000000000023000000000000",
        "00000000000000000000000000000000000000000000000000010000000000"
    );

    /// Evidence-room leg (`~/scratch/evidence-room/row-state-3/rust-master/
    /// wire/room-stage.log`, 02:53:32): 3004 with 13 music ids. The wire
    /// log caps the hex at 256 chars; the last three bytes come from
    /// the p379 body's field tail. 3004 carries no member-count field
    /// (#551 premise check in `o2jam-net`); the seat id raises the
    /// headcount instead.
    const EVIDENCE_ROOM_JOIN_BROADCAST_HEX: &str = concat!(
        "017432303770726f62650001000000012c01000001000000000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000002300000000",
        "000000000000000000000000000000000000000d00000061f360f35df35cf302f301f300",
        "f3fff2fef2fdf2fbf2f6f2f5f200000000010000000000",
    );

    /// The matching 3003 answer the joining peer received at the same
    /// instant: result 0, own seat 1, master `tester` in slot 0.
    const LIVE_JOIN_ANSWER_HEX: &str = concat!(
        "00000000010150333739524f4f4d0000001d000080000000010000000002000000000100",
        "0000746573746572000700000001a73c0000010001000000000000000000000000000000",
        "000000000000000000000000000000000000000000000000000000000000230000000000",
        "000000000000000000000000000000000000000000000000000001000000000000000000",
        "020000000003000000000400000000050000000006000000000700000000000000000000"
    );

    fn hex_bytes(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect()
    }

    fn room_with_peer(nickname: &str) -> o2jam_net::JoinedRoom {
        use o2jam_net::RoomSlot;
        let mut room = joined_snapshot();
        room.user_count = 2;
        room.slots[1] = RoomSlot {
            index: 1,
            state_code: 1,
            member: Some(member(nickname, false)),
        };
        room
    }

    /// Run the host's real 2004/2006 -> 2002/2003 exchange so the guard
    /// uses the snapshot `create_and_join_room` actually stages, not a
    /// hand-built fixture with seat rows already present.
    fn created_host_snapshot() -> o2jam_net::JoinedRoom {
        use o2jam_net::message::{CreateResult, CreateRoomResponse, CreatedRoom};
        use o2jam_net::{RoomList, RoomListEntry};
        let (mut session, server) = room_session(|mut stream| {
            assert_eq!(read_frame(&mut stream).id, Request::CreateRoom.id());
            let created = CreateRoomResponse {
                result: CreateResult::Success,
                info: Some(CreatedRoom {
                    number: 0,
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
                    &created.encode_body(),
                ))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let entry = RoomListEntry {
                number: 0,
                state_code: 1,
                title: "P379LIVE".to_owned(),
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
        });
        let room = session.create_and_join_room("P379LIVE").unwrap();
        server.join().unwrap();
        room
    }

    #[test]
    fn master_seats_the_joining_peer_from_the_3004_broadcast() {
        // Live caller chain (docs/live-path-reachability.md):
        //   main.rs::run_play -> assemble_online_session (create_room host
        //   path) -> create_and_join_room -> stage_hosted_room
        //   -> OnlineSession::paint (Waiting) -> poll_waiting_chat
        //   -> drain_waiting_chat -> ScreenGame staged seats.
        // Memoryer excludes the joiner from this broadcast, so the host is
        // the only leg that can learn the new member from it (#379).
        let body = hex_bytes(LIVE_JOIN_BROADCAST_HEX);
        let host_room = created_host_snapshot();
        assert_eq!(host_room.slots.len(), 7, "host snapshot must own seat rows");
        let (mut session, server) = joined_waiting_session(host_room, move |mut stream| {
            stream.write_all(&encode_frame(3004, &body)).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(1).is_some()
        });
        server.join().unwrap();
        let peer = session.game.staged_waiting_member(1).unwrap();
        assert_eq!(peer.nickname, "tester2");
        assert_eq!(peer.team, 1);
    }

    /// Ticket #570 paint arm: the master's cover 1 shows the joined
    /// peer's figure, not just the seat's red status mark. Wire pin:
    /// `~/scratch/p379-live/tap-master.log` 3004 seat 1 `tester2` beside
    /// master frame `d42617e1…`; the spec row names the figure parity as
    /// the remaining gap. Live caller chain: `OnlineSession::paint` ->
    /// `poll_waiting_chat` -> `drain_waiting_chat` -> `apply_user_joined`
    /// -> `ScreenGame::seat_waiting_member` -> `GameLoop::paint` ->
    /// `ScreenGame::paint_at` -> `paint_waiting` ->
    /// `paint_waiting_seat_avatars`. The one-pixel figure and seat art
    /// are synthetic, so the hidden-install-root arm is N/A here: the
    /// wire-to-seat staging is guarded above, this pins the paint arm.
    #[test]
    fn joined_peer_figure_paints_in_the_master_seat_slot() {
        use o2jam_render::{pack_xrgb, Rect, Rgb555Image};
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::{AvatarFigure, FigureLayer};
        use std::collections::HashMap;

        let body = hex_bytes(LIVE_JOIN_BROADCAST_HEX);
        let (mut session, server) =
            joined_waiting_session(created_host_snapshot(), move |mut stream| {
                stream.write_all(&encode_frame(3004, &body)).unwrap();
                assert_quiet(&mut stream);
            });
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(1).is_some()
        });
        // Seat 1's parent plus the shared avatar template: the painter
        // adds the parent origin to the template rect (owner rule f38f3).
        let seat_art = vec![
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SWAIT_EXTEND2".to_owned(),
                code: 0x1601_5023,
                sprite: None,
                rect: Rect {
                    x: 200,
                    y: 55,
                    w: 123,
                    h: 150,
                },
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_IMAGE_SWAIT_AVATAR".to_owned(),
                code: 0x1670_002E,
                sprite: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 123,
                    h: 150,
                },
            },
        ];
        session
            .game
            .load_scene(Screen::Waiting, seat_art, HashMap::new());
        let peer_pixel = 129 * 800 + 261; // slot centre: y uses (150 - 1) / 2
        let red = pack_xrgb(255, 0, 0);
        assert_ne!(
            painted_game_frame(&mut session)[peer_pixel],
            red,
            "an unstaged figure leaves the cover plate alone"
        );

        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        session.game.set_waiting_avatar(1, figure);
        assert_eq!(
            painted_game_frame(&mut session)[peer_pixel],
            red,
            "the joined peer's figure covers seat 1 on the master"
        );
        server.join().unwrap();
    }

    /// #551 wire arm: the 3004 seat id is the only live count source
    /// on the master. A vacant seat raises the headcount; a replayed
    /// broadcast for the same seat replaces the member in place and
    /// leaves the count alone. Live caller chain (docs/live-path-
    /// reachability.md): main.rs::run_play -> assemble_online_session
    /// -> stage_hosted_room -> OnlineSession::paint (Waiting) ->
    /// poll_waiting_chat -> drain_waiting_chat -> apply_user_joined ->
    /// JoinedRoom::seat_member. The channel-observer 2009/2003 half
    /// stays OPEN: evidence-room taps saw no 2009 on any observer.
    #[test]
    fn join_broadcast_raises_the_headcount_from_the_pinned_3004_member_id() {
        let body = hex_bytes(EVIDENCE_ROOM_JOIN_BROADCAST_HEX);
        let (mut session, server) =
            joined_waiting_session(created_host_snapshot(), move |mut stream| {
                stream.write_all(&encode_frame(3004, &body)).unwrap();
                stream.write_all(&encode_frame(3004, &body)).unwrap();
                assert_quiet(&mut stream);
            });
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(1).is_some()
        });
        server.join().unwrap();
        let room = session.joined_room.as_ref().expect("host snapshot");
        assert_eq!(room.user_count, 2, "the vacant seat raised the count");
        assert_eq!(room.slots[1].index, 1);
    }

    #[test]
    fn joining_peer_seats_the_master_from_the_3003_answer() {
        // The other direction of the owner's matrix: the joiner reads
        // the room's existing members out of its own 3003 snapshot,
        // which lists slot 0 but not the joiner's own seat.
        let body = hex_bytes(LIVE_JOIN_ANSWER_HEX);
        let (mut session, server) = room_session(move |mut stream| {
            let request = read_frame(&mut stream);
            assert_eq!(request.id, 3002);
            stream.write_all(&encode_frame(3003, &body)).unwrap();
            assert_quiet(&mut stream);
        });
        session.set_pending_room_join(0, "");
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Waiting
                && session.game.staged_waiting_member(0).is_some()
        });
        server.join().unwrap();
        assert_eq!(
            session.game.staged_waiting_member(0).unwrap().nickname,
            "tester"
        );
    }

    #[test]
    fn a_joining_peer_keeps_the_master_seated_across_a_song_and_speed_echo() {
        // Ticket #541 live chain (docs/live-path-reachability.md):
        //   main.rs::run_play -> OnlineSession::paint ->
        //   poll_channel_login / room join -> 3003 snapshot ->
        //   ScreenGame::set_joined_room -> poll_waiting_chat ->
        //   drain_waiting_chat -> apply_waiting_music_changed (4001) ->
        //   ScreenGame::set_room_music -> paint_waiting_seat_avatars.
        // Wire pin: qualified c3 `waiting.png` guest frame plus the
        // `full01/tap.log` 4000/4001 pick echo; the peer's own 3003
        // carries the master seat, which the later echo must not drop.
        let answer = hex_bytes(LIVE_JOIN_ANSWER_HEX);
        let (mut session, server) = room_session(move |mut stream| {
            assert_eq!(read_frame(&mut stream).id, 3002);
            stream.write_all(&encode_frame(3003, &answer)).unwrap();
            let echo = WaitingMusicChanged {
                music_id: 757,
                difficulty: 0,
                speed: 6,
            };
            stream.write_all(&echo.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::UpdateMusicState.id());
            assert_quiet(&mut stream);
        });
        session.set_pending_room_join(0, "");
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Waiting && session.game.room_music_id() == Some(757)
        });
        server.join().unwrap();
        assert_eq!(
            session
                .game
                .staged_waiting_member(0)
                .expect("master stays seated")
                .nickname,
            "tester"
        );
        let room = session.joined_room.as_ref().expect("joined room");
        assert_eq!(room.music_id, 757, "room song follows the echo");
        assert_eq!(room.speed, 6, "room speed follows the echo");
    }

    #[test]
    fn leave_broadcast_removes_the_peer_seat_on_the_master() {
        // 3007 is sent to the remaining room members only. The master's
        // staged seat 1 must disappear without another snapshot.
        let (mut session, server) =
            joined_waiting_session(room_with_peer("tester2"), |mut stream| {
                stream
                    .write_all(&encode_frame(3007, &[1, 0, 0, 0, 0, 0, 0, 0]))
                    .unwrap();
                assert_quiet(&mut stream);
            });
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(1).is_none()
        });
        server.join().unwrap();
        assert!(session.game.staged_waiting_member(1).is_none());
    }

    #[test]
    fn leave_broadcast_removes_the_master_seat_on_the_guest() {
        // Guest leg of the leave matrix: member 0 left, member 1 (us)
        // is the new master. Own seat stays; the departed cover clears.
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream
                .write_all(&encode_frame(3007, &[0, 1, 0, 0, 0, 0, 0, 0]))
                .unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(0).is_none()
        });
        server.join().unwrap();
        assert!(session.game.staged_waiting_member(0).is_none());
        assert_eq!(
            session.game.staged_waiting_member(1).unwrap().nickname,
            "guest"
        );
    }

    #[test]
    fn master_leave_promotes_the_resident_peer_to_room_master() {
        // Live caller chain (docs/live-path-reachability.md):
        //   main.rs::run_play -> OnlineSession::paint (Waiting) ->
        //   poll_waiting_chat -> drain_waiting_chat -> apply_user_left ->
        //   ScreenGame::promote_waiting_master -> is_room_master / seat star.
        // Wire-only CP37 pin in the 291 story matrix: the master leaves
        // with a peer seated; the peer gets `3007 body=0001000000000000`
        // (departing 0, promoted 1) and no 2007. The exe compares the
        // local member id against the announced master id (docs/relay.md
        // section 10, files `0x15d888`-`0x15d8d3`) for its `+0x30` flag,
        // which selects START versus READY and the seat's master emblem.
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream
                .write_all(&encode_frame(3007, &[0, 1, 0, 0, 0, 0, 0, 0]))
                .unwrap();
            assert_quiet(&mut stream);
        });
        assert!(!session.game.is_room_master(), "guest starts as a peer");
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(0).is_none()
        });
        server.join().unwrap();
        assert!(
            session.game.is_room_master(),
            "promotion must arm the peer's master controls"
        );
        assert!(
            session
                .game
                .staged_waiting_member(1)
                .is_some_and(|member| member.master),
            "the promoted seat must carry the master emblem"
        );
    }

    #[test]
    fn ready_echo_lights_the_joining_peer_seat_on_the_master() {
        let (mut session, server) =
            joined_waiting_session(room_with_peer("tester2"), |mut stream| {
                stream.write_all(&encode_frame(4009, &[1, 1])).unwrap();
                assert_quiet(&mut stream);
            });
        paint_until(&mut session, |session| {
            session
                .game
                .staged_waiting_member(1)
                .is_some_and(|member| member.ready)
        });
        server.join().unwrap();
        assert!(session.game.staged_waiting_member(1).unwrap().ready);
    }

    #[test]
    fn ready_echo_lights_the_master_seat_on_the_guest() {
        let mut room = guest_snapshot();
        room.slots[0].member.as_mut().unwrap().ready = false;
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            stream.write_all(&encode_frame(4009, &[0, 1])).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |session| {
            session
                .game
                .staged_waiting_member(0)
                .is_some_and(|member| member.ready)
        });
        server.join().unwrap();
        assert!(session.game.staged_waiting_member(0).unwrap().ready);
    }

    #[test]
    fn waiting_back_sends_the_room_leave_on_the_live_gateway() {
        // Memoryer's 3005 body is a u16 exit type (`0` = Normal);
        // without this send our leave never reaches the other member.
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            let leave = read_frame(&mut stream);
            assert_eq!(leave.id, 3005);
            assert_eq!(leave.payload, vec![0, 0]);
        });
        session.menu_key(MenuKey::Back, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
    }

    /// Ticket #559: the matched-ab-339 member-downloading leg. Live
    /// chain: `OnlineSession::paint` -> `poll_waiting_chat` ->
    /// `drain_waiting_chat` -> `apply_member_music_state` (4037) /
    /// `apply_download_progress` (4039) -> `ScreenGame` seat mirror ->
    /// `start_gate_verdict` (-4, no START frame). The percentage's bar
    /// fill stays OPEN; the pinned wire byte must not be dropped.
    #[test]
    fn downloading_state_and_progress_from_the_gateway_stage_the_seat() {
        use o2jam_net::SyncMemberMusicStateEvent;
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        let state = SyncMemberMusicStateEvent {
            member_id: 1,
            state: MusicState::Downloading,
        };
        let progress = DownloadProgressChanged {
            percentage: 42,
            member_id: 1,
        };
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream.write_all(&state.encode()).unwrap();
            stream.write_all(&progress.encode()).unwrap();
            assert_quiet(&mut stream);
        });
        paint_until(&mut session, |session| {
            session.game.member_download_progress(1) == Some(42)
        });
        server.join().unwrap();
        assert_eq!(
            session.game.member_music_state(1),
            Some(MusicState::Downloading.code()),
            "4037 arms the seat's download badge"
        );
        assert_eq!(
            session.game.member_download_progress(1),
            Some(42),
            "4039 stages the percentage instead of dropping it"
        );
        assert_eq!(
            session.game.start_gate_verdict(),
            StartGateVerdict::MusicDownloading,
            "the exe's -4 verdict keeps START closed"
        );
    }

    /// Ticket #559 gate arm: with a downloading peer the master's START
    /// press arms the face but sends nothing; once the 4037 state clears
    /// the same press sends the pinned 4010. Live chain: `menu_key` ->
    /// `ScreenGame::press_start` -> `StartGateVerdict::MusicDownloading`
    /// -> `OnlineSession` send/no-send.
    #[test]
    fn start_stays_closed_while_a_peer_reports_downloading() {
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member_on_team("peer", true, 1));
        let (mut session, server) = joined_waiting_session(room, |mut stream| {
            let start = read_frame(&mut stream);
            assert_eq!(start.id, 4010);
            assert!(start.payload.is_empty());
            assert_quiet(&mut stream);
        });
        session
            .game
            .set_member_music_state(1, MusicState::Downloading.code());
        assert_eq!(
            session.game.start_gate_verdict(),
            StartGateVerdict::MusicDownloading
        );
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting, "still in Waiting");

        session
            .game
            .set_member_music_state(1, MusicState::Ready.code());
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
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
        game.menu_key(MenuKey::Back, true);
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

    /// Open the staged select dialog with a catalog row, click it, then
    /// click OK — the shared master pick gesture for the 4000 wire guards.
    fn click_master_song_row_and_ok(session: &mut OnlineSession) {
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use std::collections::HashMap;

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
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(400, 150 + 23 + 11));
        assert!(session.game_mut().has_pending_music_send());
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(389, 519));
        assert!(!session.game_mut().select_music_dialog_open());
    }

    #[test]
    fn master_row_pick_echoes_4000_and_4001_over_the_scripted_gateway() {
        // Ticket #564 live chain (docs/live-path-reachability.md):
        //   main.rs::run_play -> OnlineSession::paint ->
        //   ScreenGame::ingest_snapshot (MUSELECT row + OK) ->
        //   flush_music_send -> SetRoomMusic 4000 / SetRoomArena 4002 ->
        //   drain_waiting_chat -> apply_waiting_music_changed 4001 ->
        //   ScreenGame::set_room_music.
        // Wire pin: `full01/tap.log` 15:43:48 `C2S 4000 len=8
        // hex=64000101` -> `S2C 4001 len=8 hex=64000101`.
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let pick = read_frame(&mut stream);
            assert_eq!(pick.id, o2jam_net::Request::SetRoomMusic.id());
            let body = o2jam_net::play::SetRoomMusic::decode_body(&pick.payload).unwrap();
            assert_eq!(body.music_id, 757);
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::SetRoomArena.id()
            );
            let echo = WaitingMusicChanged {
                music_id: 757,
                difficulty: 0,
                speed: 2,
            };
            stream.write_all(&echo.encode()).unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::UpdateMusicState.id()
            );
            assert_quiet(&mut stream);
        });
        click_master_song_row_and_ok(&mut session);
        paint_until(&mut session, |session| {
            session.game.room_music_id() == Some(757)
        });
        server.join().unwrap();
        let room = session.joined_room.as_ref().expect("joined room");
        assert_eq!(room.music_id, 757, "the 4001 echo stages the room song");
        assert_eq!(room.speed, 2, "the 4001 echo stages the room speed");
    }

    #[test]
    fn waiting_select_music_ok_with_staged_pick_reaches_4000_over_scripted_gateway() {
        // Ticket #232 wire guard: a real session walk, row click through
        // ingest_snapshot, OK click at the staged rect, then a paint
        // must put the 4000 SetRoomMusic frame on the wire with the
        // staged music id. Server-side assertion pins the accept clause.
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

        // Stage the dialog overlay, click the 757 row, then OK; polling
        // drives the queued pick onto the wire as 4000.
        click_master_song_row_and_ok(&mut session);

        // 3. Polling drives the queued pick onto the wire as 4000.
        paint_frame(&mut session);

        // 4. Flow advanced into song select (attempt staged, pick
        // consumed from the outbox).
        assert!(!session.game_mut().has_pending_music_send());

        server.join().unwrap();
    }

    /// Local half of joining a new room after the master's 3005 exit:
    /// stage the new room and take the same Room Confirm -> Waiting
    /// edge `joined_waiting_session` uses. Synthetic room, no
    /// install-root read, so both guards run with game data hidden.
    fn reenter_waiting_after_room_exit(session: &mut OnlineSession) {
        let room = joined_snapshot();
        session.joined_room = Some(room.clone());
        session.game.set_joined_room(room);
        session.joined_waiting = true;
        session.game.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    #[test]
    fn waiting_back_disbands_the_staged_pick_before_a_reentry_can_flush_4000() {
        // Ticket #732, exe pin #268 (W8 `wa-pick-okback2`): master BACK
        // sends 3005, the server answers 2007 (room destroyed), so no
        // pick survives the exit. Live caller chain: main.rs::run_play
        // -> OnlineSession::ingest_snapshot -> OnlineSession::menu_key(Back)
        // -> ExitWaiting 3005 -> ScreenGame::menu_key ->
        // ScreenGame::enter_screen(Waiting -> Room) ->
        // ScreenGame::disband_waiting_room ->
        // PendingOutbox::discard_music_send. The next paint must find
        // no body in OnlineSession::flush_music_send.
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            let leave = read_frame(&mut stream);
            assert_eq!(leave.id, 3005, "Waiting BACK sends the room leave");
            assert_eq!(
                leave.payload.len(),
                2,
                "0BBD len=6 minus the 4-byte frame header"
            );
            assert_quiet(&mut stream);
        });
        click_master_song_row_and_ok(&mut session);

        session.menu_key(MenuKey::Back, true);
        assert_eq!(session.game.screen(), Screen::Room);
        assert!(
            !session.game.has_pending_music_send(),
            "the exited room's pick is gone before any re-entry"
        );
        reenter_waiting_after_room_exit(&mut session);

        // Reopen the sheet and tap OK without a fresh row pick — the
        // exact #268 stale-flush gesture, now with no body to flush.
        session
            .game_mut()
            .load_select_music_scene(Vec::new(), std::collections::HashMap::new());
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(389, 519));
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
    }

    #[test]
    fn fresh_pick_after_a_waiting_exit_still_sends_the_pinned_4000() {
        // Ticket #732: the exit clear must not break the pinned wire
        // path. Live chain: main.rs::run_play -> OnlineSession::paint ->
        // ScreenGame::ingest_snapshot -> click_select_music ->
        // pick_select_music_row -> OnlineSession::flush_music_send ->
        // SetRoomMusic 4000 / SetRoomArena 4002. Synthetic catalog, no
        // install-root read (CI-safe with hidden game data).
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            assert_eq!(read_frame(&mut stream).id, 3005);
            let pick = read_frame(&mut stream);
            assert_eq!(pick.id, o2jam_net::Request::SetRoomMusic.id());
            let body = o2jam_net::play::SetRoomMusic::decode_body(&pick.payload).unwrap();
            assert_eq!(body.music_id, 757, "the fresh row id is on the wire");
            assert_eq!(
                read_frame(&mut stream).id,
                o2jam_net::Request::SetRoomArena.id()
            );
            assert_quiet(&mut stream);
        });
        click_master_song_row_and_ok(&mut session);

        session.menu_key(MenuKey::Back, true);
        reenter_waiting_after_room_exit(&mut session);
        click_master_song_row_and_ok(&mut session);
        paint_until(&mut session, |session| {
            !session.game.has_pending_music_send()
        });
        server.join().unwrap();
    }

    /// Staged button for the MUSELECT-wire guards: the click lands on
    /// the control's own `.bnd` rect, never on a literal screen point.
    fn staged_waiting_button(id: &str, x: i32, y: i32) -> o2jam_shell::avatar::ScenePlacement {
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        ScenePlacement {
            id: id.to_owned(),
            code: 0,
            sprite: None,
            rect: o2jam_render::Rect { x, y, w: 80, h: 30 },
            role: PlacementRole::Single,
        }
    }

    #[test]
    fn mu_select_open_sends_no_4051_or_4053_over_scripted_gateway() {
        // Ticket #407: no exe send pin names StartSelectMusic 4051 or
        // CancelSelectMusic 4053, so a master's MUSELECT open must put
        // no frame on the gateway. Live chain: `main.rs::run_play` ->
        // `OnlineSession::paint` -> `ScreenGame::ingest_snapshot` ->
        // `apply_pointer_down` -> `click_waiting_button` -> `select_music`
        // staging; the session drains its outbox on the next paint.
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            assert_quiet(&mut stream);
        });
        let figures = std::collections::HashMap::new();
        session.game_mut().load_scene(
            Screen::Waiting,
            vec![staged_waiting_button("IDC_BUTTON_SWAIT_MUSELECT", 100, 100)],
            figures,
        );
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(140, 115));
        assert!(session.game.select_music_dialog_open());
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
    }

    #[test]
    fn music_select_ok_close_sends_no_4051_or_4053_over_scripted_gateway() {
        // Ticket #407 close leg: the OK confirm closes the dialog and
        // flushes the staged 4000, but never invents a 4051/4053 pair.
        // Live chain: `main.rs::run_play` -> `OnlineSession::paint` ->
        // `ScreenGame::ingest_snapshot` -> `click_select_music` ->
        // `take_music_send` -> `OnlineSession::flush_music_send`.
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            let first = read_frame(&mut stream);
            assert_ne!(first.id, 4051, "no StartSelectMusic send is pinned");
            assert_ne!(first.id, 4053, "no CancelSelectMusic send is pinned");
            assert_eq!(first.id, Request::SetRoomMusic.id(), "4000 still flushes");
            let second = read_frame(&mut stream);
            assert_ne!(second.id, 4051, "no StartSelectMusic send is pinned");
            assert_ne!(second.id, 4053, "no CancelSelectMusic send is pinned");
            assert_eq!(second.id, Request::SetRoomArena.id(), "4002 still flushes");
            assert_quiet(&mut stream);
        });
        let figures = std::collections::HashMap::new();
        session.game_mut().load_scene(
            Screen::Waiting,
            vec![staged_waiting_button("IDC_BUTTON_SWAIT_MUSELECT", 100, 100)],
            figures.clone(),
        );
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(140, 115));
        assert!(session.game.select_music_dialog_open());
        session.game_mut().load_select_music_scene(
            vec![staged_waiting_button("IDC_BUTTON_DSMUSIC_OK", 326, 497)],
            figures,
        );
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&757u32.to_le_bytes());
        rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
        rec[112..117].copy_from_slice(b"First");
        rec[176..180].copy_from_slice(b"Band");
        rec[240..251].copy_from_slice(b"o2ma757.ojm");
        let entries = o2jam_file::parse_ojn_list(&rec).unwrap();
        session.set_song_catalog(&entries);
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(400, 184));
        assert!(session.game_mut().has_pending_music_send());
        session
            .game_mut()
            .ingest_snapshot(&InputSnapshot::click_at(366, 512));
        assert!(!session.game.select_music_dialog_open());
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        println!(
            "after paint screen {:?} dialog {} pending {} selected {:?}",
            session.game.screen(),
            session.game.select_music_dialog_open(),
            session.game.has_pending_music_send(),
            session.selected_song()
        );
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

    /// Empty title refusal is a wire behavior (ticket #514): the exe
    /// tap `~/scratch/evidence-room/create-fields-2/exe/wire/
    /// room-stage.log` 02:26:34 logs `no C2S 2004 count=0` beside
    /// `title-02-refusal-click.png` sha256
    /// `1058be51698dc3c2c0ba61ba0d3e56b42311a68ba4fcb135900a5555ff27a658`.
    /// Caller chain: boot → Room → staged CREATE sheet → clear title →
    /// Commit → paint → the creator socket stays silent.
    #[test]
    fn refused_empty_title_sends_no_create_frame_on_the_gateway() {
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::CREATE_ROOM_CREATE_CODE;
        use std::collections::HashMap;

        let (mut session, server) = room_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.game_mut().load_create_room_scene(
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "CREATE_OK".to_owned(),
                code: CREATE_ROOM_CREATE_CODE,
                sprite: None,
                rect: Rect {
                    x: 10,
                    y: 10,
                    w: 30,
                    h: 30,
                },
            }],
            HashMap::new(),
        );
        session.game_mut().open_create_room_dialog();
        session.game_mut().delete_room_dialog_char();
        assert_eq!(session.game_mut().create_room_form().title, "");
        session.game_mut().click_room_dialog(20, 20);
        assert!(session.game_mut().room_dialog().is_some());
        assert!(session.game_mut().take_create_room().is_none());
        paint_frame(&mut session);
        assert_eq!(session.screen(), Screen::Room);
        server.join().unwrap();
    }

    /// Live-path guard for the exe CREATE wire pin in
    /// `~/scratch/evidence-room/create-fields-2/exe/wire/room-stage.log`
    /// 02:27:01 (`C2S 2004 len=17
    /// hex=454d505459505300020000310000000000`, key byte `02`, empty
    /// password field, frame `pass-02-empty-password-frame.png` sha256
    /// `70318790067b14ba91b45cec4ecd47df14ba7aad5bc54fbefef6147db3db671d`).
    /// Caller chain: boot → Room → staged CREATE sheet → Commit →
    /// `flush_create_room` → `create_and_join_room` → gateway 2004.
    #[test]
    fn create_commit_keeps_the_staged_form_fields_on_the_live_gateway() {
        use o2jam_net::frame::encode_frame;
        use o2jam_net::message::{CreateResult, CreateRoom, CreateRoomResponse, CreatedRoom};
        use o2jam_net::{KeyMode, RoomList, RoomListEntry};
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::{CREATE_ROOM_7KEY_MODE_CODE, CREATE_ROOM_CREATE_CODE};
        use std::collections::HashMap;

        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, Request::CreateRoom.id());
            let create = CreateRoom::decode_body(&frame.payload).unwrap();
            assert_eq!(create.title, "EMPTYPS");
            assert_eq!(create.key_mode, KeyMode::SevenKeys);
            assert!(!create.has_password);
            assert!(create.password.is_empty());
            let created = CreateRoomResponse {
                result: CreateResult::Success,
                info: Some(CreatedRoom {
                    number: 42,
                    premium: false,
                    password: String::new(),
                    has_password: false,
                    min_level: 0,
                    max_level: 99,
                }),
            };
            stream
                .write_all(&encode_frame(
                    CreateRoomResponse::COMMAND,
                    &created.encode_body(),
                ))
                .unwrap();
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, Request::GetChannelInfo.id());
            let list = RoomList {
                rooms: vec![RoomListEntry {
                    number: 42,
                    state_code: 1,
                    title: "EMPTYPS".to_owned(),
                    has_password: false,
                    music_id: 0,
                    difficulty: 0,
                    key_mode: 2,
                    game_mode: 0,
                    speed: 1,
                    capacity: 8,
                    user_count: 1,
                    min_level: 0,
                    max_level: 99,
                    skills: Vec::new(),
                    premium: false,
                    room_type: 0,
                }],
            };
            stream
                .write_all(&encode_frame(RoomList::COMMAND, &list.encode_body()))
                .unwrap();
            assert_quiet(&mut stream);
        });

        let staged = [CREATE_ROOM_CREATE_CODE, CREATE_ROOM_7KEY_MODE_CODE]
            .into_iter()
            .enumerate()
            .map(|(index, code)| ScenePlacement {
                role: PlacementRole::Single,
                id: format!("CREATE_{index}"),
                code,
                sprite: None,
                rect: Rect {
                    x: 10 + index as i32 * 40,
                    y: 10,
                    w: 30,
                    h: 30,
                },
            })
            .collect();
        session
            .game_mut()
            .load_create_room_scene(staged, HashMap::new());
        session.game_mut().open_create_room_dialog();
        session.game_mut().click_room_dialog(50, 20);
        for letter in "EMPTYPS".chars() {
            session.game_mut().push_room_dialog_char(letter);
        }
        session.game_mut().click_room_dialog(20, 20);
        paint_frame(&mut session);

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

    /// #367 live chain: `main.rs::run_online` ->
    /// `stage_channel_list_after_relay_gate` ->
    /// `ScreenGame::set_room_no_response_relay_down` -> Room entry ->
    /// `OnlineSession::paint` -> `poll_wire` -> `poll_room_user_list_fuse`
    /// -> `ScreenGame::track_room_user_list_send_at` /
    /// `track_room_user_list_reply`. This tier-2 scripted-gateway
    /// round trip uses the real boot staging seam and a real socket.
    #[test]
    fn relay_down_room_poll_sends_07da_and_clears_on_07db() {
        let (port, server) = fake_gateway(|mut stream| {
            let request = read_frame(&mut stream);
            assert_eq!(request.id, Request::GetUserList.id(), "our 07DA poll");
            stream
                .write_all(&encode_frame(Response::GetUserList as u16, &[]))
                .unwrap();
        });
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        paint_frame(&mut session);
        assert!(
            session.game.room_user_list_poll_tracked(),
            "send stamps the tracked 07DB node"
        );
        thread::sleep(Duration::from_millis(20));
        paint_frame(&mut session);
        assert!(
            !session.game.room_user_list_poll_tracked(),
            "07DB clears the node"
        );
        assert!(!session.game.room_no_response_dialog_open());
        server.join().unwrap();
    }

    /// #367 dismissal pin: the shared one-OK sheet's OK posts queue
    /// type 100; the pump closes the top dialog; no FFF0 (or any other
    /// frame) goes on the wire. Live chain: boot staging -> Room entry
    /// -> fuse arm -> `paint_room_no_response_dialog` ->
    /// `information_ok_hit` -> `post_room_no_response_dismissal` ->
    /// `pump_dialog_queue`. Enter/Escape/Space share the OK path
    /// through [`ScreenGame::menu_key`] / lane key 3.
    #[test]
    fn room_fuse_ok_posts_queue_type_100_and_sends_no_fff0() {
        let (port, server) = fake_gateway(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_millis(120)))
                .unwrap();
            let mut bytes = [0u8; 64];
            match stream.read(&mut bytes) {
                Ok(0) => panic!("gateway closed"),
                Ok(count) => {
                    let frame = decode_frame(&bytes[..count]).unwrap();
                    assert_ne!(frame.id, 0xfff0, "OK must not post FFF0");
                    panic!("unexpected frame {} after OK", frame.id);
                }
                Err(error) => assert!(
                    matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ),
                    "OK must leave the socket quiet: {error}"
                ),
            }
        });
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);
        let now = Instant::now();
        assert!(game.track_room_user_list_send_at(
            now.checked_sub(Duration::from_secs(21))
                .expect("clock margin")
        ));
        assert!(game.pump_room_user_list_fuse_at(now));
        assert!(game.room_no_response_dialog_open());
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        session.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(400, 365));
        assert_eq!(
            session.game.room_no_response_dismissal_queued(),
            Some(100),
            "the OK handler posts queue type 100"
        );
        paint_frame(&mut session);
        assert!(!session.game.room_no_response_dialog_open());
        server.join().unwrap();
    }

    /// #689 live dismissal stack: two OK taps before the paint queue
    /// two type-100 records. The first pops only the fuse sheet; the
    /// second pops the create-room form it covered, still locally.
    ///
    /// Live caller chain: `main.rs::run_online` -> `OnlineSession::paint`
    /// -> `poll_wire` -> `poll_room_user_list_fuse` ->
    /// `ScreenGame::pump_room_user_list_fuse_at` -> Information OK ->
    /// `post_room_no_response_dismissal` -> `ScreenGame::paint` ->
    /// `pump_dialog_queue`. Named exe click: #367 dossier TEXT
    /// `0x21400003` / OK `0x21200002`; no qualified tapped pack yet.
    #[test]
    fn fuse_ok_pops_one_dialog_level_and_leaves_the_room_sheet_below() {
        let (port, server) = fake_gateway(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_millis(120)))
                .unwrap();
            let mut bytes = [0u8; 64];
            match stream.read(&mut bytes) {
                Ok(0) => panic!("gateway closed"),
                Ok(count) => {
                    let frame = decode_frame(&bytes[..count]).unwrap();
                    panic!("OK must not put frame {} on the wire", frame.id);
                }
                Err(error) => assert!(matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                )),
            }
        });
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);
        game.open_create_room_dialog();
        let now = Instant::now();
        assert!(game.track_room_user_list_send_at(
            now.checked_sub(Duration::from_secs(21))
                .expect("clock margin")
        ));
        assert!(game.pump_room_user_list_fuse_at(now));
        assert!(game.room_no_response_dialog_open());
        assert_eq!(game.room_dialog(), Some(RoomDialog::CreateRoom));
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        session.ingest_snapshot(&InputSnapshot::click_at(400, 365));
        session.ingest_snapshot(&InputSnapshot::click_at(400, 365));
        assert_eq!(
            session.game.room_no_response_dismissal_queued(),
            Some(100),
            "each OK posts one type-100 record"
        );
        paint_frame(&mut session);
        assert!(!session.game.room_no_response_dialog_open());
        assert_eq!(
            session.game.room_dialog(),
            Some(RoomDialog::CreateRoom),
            "one OK pops exactly one level"
        );
        // Only the two queued dismissals belong to this tap pair.
        // Stop the room poll before the second paint so a fresh 07DA
        // cannot be mistaken for a frame the OK handler invented.
        session.game_mut().set_room_no_response_relay_down(false);
        paint_frame(&mut session);
        assert_eq!(
            session.game.room_dialog(),
            None,
            "the next type-100 record pops the covered sheet"
        );
        assert_eq!(session.screen(), Screen::Room);
        server.join().unwrap();
    }

    /// #690 local CANCEL leg. Frame capture name `S316-QUIT-CANCEL-04`
    /// (qualified exe before/after pack is still GAP). The headless
    /// pair below is our client's own pixel proof, not an oracle pack.
    ///
    /// Live caller chain: `main.rs::run_play` -> `run_windowed_session`
    /// -> `Shell::run` -> `WindowHandler::route_mouse_input` ->
    /// `GameLoop::ingest_snapshot` -> `OnlineSession::ingest_snapshot`
    /// -> `ScreenGame::click_planet_modal` ->
    /// `ScreenGame::cancel_quit_confirm`.
    #[test]
    fn quit_cancel_closes_only_the_sheet_and_emits_no_wire_frame() {
        let (port, server) = fake_gateway(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_millis(120)))
                .unwrap();
            let mut bytes = [0u8; 64];
            match stream.read(&mut bytes) {
                Ok(0) => panic!("gateway closed"),
                Ok(count) => {
                    let frame = decode_frame(&bytes[..count]).unwrap();
                    panic!("CANCEL must not put frame {} on the wire", frame.id);
                }
                Err(error) => assert!(matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                )),
            }
        });
        let game = ScreenGame::new();
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.menu_key(MenuKey::Back, true);
        let bare = painted_game_frame(&mut session);

        session.ingest_snapshot(&InputSnapshot::click_at(769, 580));
        assert_eq!(
            session.game.planet_modal(),
            Some(o2jam_shell::screens::game::PlanetModal::QuitConfirm)
        );
        let with_sheet = painted_game_frame(&mut session);
        assert_ne!(with_sheet, bare, "quit sheet painted nothing");

        session.ingest_snapshot(&InputSnapshot::click_at(450, 253));
        paint_frame(&mut session);
        let after = painted_game_frame(&mut session);
        assert_ne!(after, with_sheet, "CANCEL left the sheet on screen");
        assert_eq!(session.game.planet_modal(), None);
        assert_eq!(
            session.game.last_quit_confirm_outcome(),
            Some(QuitConfirmOutcome::Cancelled)
        );
        assert!(!GameLoop::quit_requested(&session), "CANCEL must not quit");
        assert_eq!(session.screen(), Screen::Planet);
        server.join().unwrap();
    }

    /// #691 OK leg. Frame capture name `S316-QUIT-CANCEL-04`; the
    /// bounded exe boot with a post-OK process-state line and exit
    /// socket tap is still an OPEN evidence step.
    ///
    /// Live caller chain: `main.rs::run_play` -> `run_windowed_session`
    /// -> `Shell::run` -> `WindowHandler::handle_window_event`
    /// (`RedrawRequested`) -> `should_exit_after_frame` ->
    /// `GameLoop::quit_requested` -> `OnlineSession::quit_requested`
    /// -> `ScreenGame::quit_confirmed`, fed by the sheet OK edge
    /// through `ScreenGame::confirm_quit`.
    #[test]
    fn quit_ok_requests_exit_without_a_wire_frame() {
        let (port, server) = fake_gateway(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_millis(120)))
                .unwrap();
            let mut bytes = [0u8; 64];
            match stream.read(&mut bytes) {
                Ok(0) => panic!("gateway closed"),
                Ok(count) => {
                    let frame = decode_frame(&bytes[..count]).unwrap();
                    panic!("OK must not put frame {} on the wire", frame.id);
                }
                Err(error) => assert!(matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                )),
            }
        });
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(769, 580);
        assert_eq!(
            game.planet_modal(),
            Some(o2jam_shell::screens::game::PlanetModal::QuitConfirm)
        );
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        session.ingest_snapshot(&InputSnapshot::click_at(350, 253));
        paint_frame(&mut session);

        assert_eq!(
            session.game.last_quit_confirm_outcome(),
            Some(QuitConfirmOutcome::Confirmed)
        );
        assert!(GameLoop::quit_requested(&session), "OK must exit");
        assert_eq!(session.game.planet_modal(), None);
        assert_eq!(session.screen(), Screen::Planet);
        server.join().unwrap();
    }

    /// #569 row-state arm: the 07DA fuse owns the socket while its
    /// node is tracked, so a 2003 room-list probe read by that drain
    /// is deferred. The overdue node then opens the one-OK sheet;
    /// the next paint must replay the deferred 2003 into the row
    /// state even while that modal is up. Live caller chain:
    /// boot relay gate -> `stage_channel_list_after_relay_gate` ->
    /// Room -> `OnlineSession::paint` -> `poll_wire` ->
    /// `poll_room_user_list_fuse` -> `drain_room_user_list_reply` ->
    /// `deferred_frames` -> `replay_deferred_frames` ->
    /// `apply_lobby_frame` -> `ScreenGame::set_rooms`. No qualified
    /// exe frame pair exists yet (story evidence is OPEN).
    #[test]
    fn starved_room_poll_replays_a_2003_row_state_probe_while_the_modal_is_up() {
        let waiting = listed_p380_room(0, "STALLED");
        let playing = o2jam_net::RoomListEntry {
            state_code: 2,
            ..waiting.clone()
        };
        let list = o2jam_net::RoomList {
            rooms: vec![playing],
        };
        let body = list.encode_body();
        let (port, server) = fake_gateway(move |mut stream| {
            stream
                .write_all(&encode_frame(o2jam_net::RoomList::COMMAND, &body))
                .unwrap();
            assert_quiet(&mut stream);
        });
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);
        game.set_rooms(vec![waiting]);
        let now = Instant::now();
        assert!(game.track_room_user_list_send_at(
            now.checked_sub(Duration::from_secs(21))
                .expect("clock margin")
        ));
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);

        paint_frame(&mut session);
        assert!(
            session.game.room_no_response_dialog_open(),
            "the starved node opened the one-OK sheet"
        );
        assert_eq!(
            session.game.listed_rooms()[0].state_code,
            1,
            "the fuse read defers the 2003 instead of dropping it"
        );
        paint_frame(&mut session);
        assert_eq!(
            session.game.listed_rooms()[0].state_code,
            2,
            "the modal does not block row-state replay"
        );
        server.join().unwrap();
    }

    /// Enter, Escape, and Space (our lane-3 key) share the one OK
    /// path; a second dismissal posts nothing once the dialog is gone.
    #[test]
    fn room_fuse_key_dismissals_share_the_type_100_path() {
        for key in [MenuKey::Confirm, MenuKey::Back] {
            let mut game = ScreenGame::new();
            game.menu_key(MenuKey::Back, true);
            game.click_planet(136, 88);
            game.menu_key(MenuKey::Confirm, true);
            crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);
            let now = Instant::now();
            assert!(game.track_room_user_list_send_at(
                now.checked_sub(Duration::from_secs(21))
                    .expect("clock margin")
            ));
            assert!(game.pump_room_user_list_fuse_at(now));
            game.menu_key(key, true);
            assert_eq!(game.room_no_response_dismissal_queued(), Some(100));
            game.pump_dialog_queue();
            assert!(!game.room_no_response_dialog_open());
        }

        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);
        let now = Instant::now();
        assert!(game.track_room_user_list_send_at(
            now.checked_sub(Duration::from_secs(21))
                .expect("clock margin")
        ));
        assert!(game.pump_room_user_list_fuse_at(now));
        game.lane_key(3, true);
        assert_eq!(game.room_no_response_dismissal_queued(), Some(100));
        game.pump_dialog_queue();
        assert!(!game.room_no_response_dialog_open());
    }

    /// Hash the resident peer's staged filter strip and rows as one
    /// value. The exe's W-b frames were byte-identical across every
    /// master tab click; this is the state-side form of that pin.
    fn resident_room_view_hash(game: &o2jam_shell::ScreenGame) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{:?}", game.room_filter()).hash(&mut hasher);
        for room in game.listed_rooms() {
            (
                room.number,
                room.title.as_str(),
                room.game_mode,
                room.user_count,
            )
                .hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Room-seat client with a scripted gateway that records every
    /// opcode it sends. Two of these make the W-b two-client leg.
    fn recording_room_client(
        rooms: Vec<o2jam_net::RoomListEntry>,
    ) -> (OnlineSession, mpsc::Receiver<u16>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (wire_tx, wire_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(20)))
                .unwrap();
            loop {
                let mut header = [0u8; HEADER_LEN];
                match stream.read_exact(&mut header) {
                    Ok(()) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        continue;
                    }
                    Err(_) => break,
                }
                let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
                let mut bytes = header.to_vec();
                let mut body = vec![0u8; body_len.saturating_sub(HEADER_LEN)];
                stream.read_exact(&mut body).unwrap();
                bytes.extend_from_slice(&body);
                wire_tx
                    .send(decode_frame(&bytes).unwrap().id)
                    .expect("the test reads its wire tap");
            }
        });
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.set_rooms(rooms);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        (OnlineSession::new(game, gateway), wire_rx, server)
    }

    /// Ticket #536 re-scope (W6 W-b, traces `W6FB-PEER-01/02`):
    /// ALL/NORMAL/LIVE is a local view over the already-staged 2003
    /// rows. The master's three tab clicks must leave a resident
    /// peer's strip and rows hash-unchanged and put no per-click
    /// frame on either socket; keepalives are the only allowed wire.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> assemble_online_session -> paint ->
    ///   ScreenGame::ingest_snapshot -> click_dispatch ->
    ///   click_room_control -> RoomControlAction::RoomFilter ->
    ///   ScreenGame::refresh_visible_rooms (local rows; no wire).
    /// The peer's second seat is the scripted `recording_room_client`
    /// above, so the tap covers both clients of one room-list leg.
    #[test]
    fn resident_peer_keeps_its_room_filter_and_rows_while_the_master_switches_tabs() {
        use o2jam_net::{GameMode, RoomListEntry};
        use o2jam_render::Rect;
        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use std::collections::HashMap;

        let listed_room = |number, title: &str, game_mode| RoomListEntry {
            number,
            state_code: 1,
            title: title.to_owned(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode,
            speed: 1,
            capacity: 8,
            user_count: 1,
            min_level: 1,
            max_level: 99,
            skills: Vec::new(),
            premium: false,
            room_type: 0,
        };
        let normal = listed_room(0, "W6FILTER", GameMode::Normal.as_u8());
        let live = listed_room(1, "LIVE-ONLY", GameMode::Live.as_u8());
        let rooms = vec![normal, live];
        let (mut master, master_rx, master_server) = recording_room_client(rooms.clone());
        let (mut peer, peer_rx, peer_server) = recording_room_client(rooms);
        let placements = [(0x1320_1003u32, 25), (0x1320_1004, 75), (0x1320_1005, 125)]
            .into_iter()
            .map(|(code, x)| ScenePlacement {
                role: PlacementRole::Single,
                id: format!("FILTER_{code:X}"),
                code,
                sprite: None,
                rect: Rect {
                    x,
                    y: 10,
                    w: 30,
                    h: 30,
                },
            })
            .collect();
        master
            .game_mut()
            .load_scene(Screen::Room, placements, HashMap::new());
        let peer_hash = resident_room_view_hash(&peer.game);

        for (x, expected_filter) in [(75, "Normal"), (125, "Live"), (25, "All")] {
            master.ingest_snapshot(&InputSnapshot::click_at(x, 25));
            paint_frame(&mut master);
            paint_frame(&mut peer);
            assert_eq!(
                format!("{:?}", master.game.room_filter()),
                expected_filter,
                "the master switched to its local {expected_filter} family"
            );
            assert_eq!(
                resident_room_view_hash(&peer.game),
                peer_hash,
                "a resident peer keeps its strip and rows on every master tab click"
            );
        }

        let noop = master
            .game
            .room_control_noop()
            .expect("the tab press still records its wire step");
        assert!(
            noop.next_step.contains("W6FB-PEER-02"),
            "the no-op names the W-b null wire pin, got {}",
            noop.next_step
        );
        let opcode_filter = |rx: &mpsc::Receiver<u16>| {
            rx.try_iter()
                .filter(|id| !matches!(id, 6000 | 6001))
                .collect::<Vec<_>>()
        };
        assert!(
            opcode_filter(&master_rx).is_empty(),
            "the master's tab clicks put no filter opcode on the wire"
        );
        assert!(
            opcode_filter(&peer_rx).is_empty(),
            "the resident peer's tap has no filter or join opcode"
        );
        drop(master);
        drop(peer);
        master_server.join().unwrap();
        peer_server.join().unwrap();
    }

    /// Ticket #693 re-scope (W6 W-c, trace `W6FC-DIALOG-02`): a peer
    /// joins through its own `3002`/`3003` pair while the master's
    /// CREATE ROOM sheet stays up. The master's frame and wire stay
    /// unchanged; no master-side `3004`/`3003` join event is claimed,
    /// staged, or wired by this guard.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> assemble_online_session -> paint ->
    ///   ScreenGame::ingest_snapshot -> click_room_control(Create) ->
    ///   open_create_room_dialog (master sheet stays up) ||
    ///   peer: MenuKey::Confirm -> begin_room_join -> poll_room_join ->
    ///   JoinRoom 3002 -> JoinRoomResponse 3003 -> resolve_room_join.
    #[test]
    fn peer_join_pair_leaves_the_create_room_sheet_and_master_frame_unchanged() {
        use o2jam_net::rooms::{JoinRoom, JoinRoomResponse};
        use o2jam_net::Request;
        use o2jam_shell::screens::side::RoomDialog;

        let answer = hex_bytes(LIVE_JOIN_ANSWER_HEX);
        let decoded = JoinRoomResponse::decode_body(&answer).unwrap();
        assert_eq!(decoded.result_code, 0);
        let room = decoded.room.expect("the 3003 body carries the member rows");
        let members: Vec<&str> = room
            .slots
            .iter()
            .filter_map(|slot| slot.member.as_ref())
            .map(|member| member.nickname.as_str())
            .collect();
        assert!(
            members.contains(&"tester"),
            "the pinned 3003 names the master member, got {members:?}"
        );

        let (mut master, master_rx, master_server) = recording_room_client(vec![]);
        // The rig's 방만들기 click routes through click_room_control(Create)
        // to this same dialog entry; the scripted leg mounts it directly
        // so the peer's wire pair is the only stimulus under test.
        master.game_mut().open_create_room_dialog();
        assert_eq!(
            master.game.room_dialog(),
            Some(RoomDialog::CreateRoom),
            "the staged CREATE button opened the master sheet"
        );
        let master_before = painted_game_frame(&mut master);

        let (mut peer, peer_server) = room_session(move |mut stream| {
            let request = read_frame(&mut stream);
            assert_eq!(request.id, Request::JoinWaiting.id());
            let join = JoinRoom::decode_body(&request.payload).unwrap();
            assert_eq!(join.number, 0);
            assert!(join.password.is_empty());
            stream
                .write_all(&encode_frame(3003, &answer))
                .expect("the scripted peer reply reaches the joiner");
            assert_quiet(&mut stream);
        });
        peer.set_pending_room_join(0, "");
        peer.menu_key(MenuKey::Confirm, true);
        paint_until(&mut peer, |session| {
            session.game.screen() == Screen::Waiting
                && session
                    .game
                    .staged_waiting_member(0)
                    .is_some_and(|member| member.nickname == "tester")
        });
        peer_server.join().unwrap();

        let master_after = painted_game_frame(&mut master);
        assert_eq!(
            master_after, master_before,
            "the peer join repaints nothing under the master's CREATE sheet"
        );
        assert_eq!(
            master.game.room_dialog(),
            Some(RoomDialog::CreateRoom),
            "the master sheet survives the peer join"
        );
        assert!(
            master.game.staged_waiting_member(1).is_none(),
            "no master-side join event is staged from the peer's 3003"
        );
        let master_join_ops: Vec<u16> = master_rx
            .try_iter()
            .filter(|id| matches!(id, 3003 | 3004))
            .collect();
        assert!(
            master_join_ops.is_empty(),
            "the master socket sees no join event, got {master_join_ops:?}"
        );
        drop(master);
        master_server.join().unwrap();
    }
}
