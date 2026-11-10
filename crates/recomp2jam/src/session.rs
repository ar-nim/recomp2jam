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
//! exchanges `JoinWaiting` (3002) on Room Confirm and stages the
//! joined snapshot through [`ScreenGame::set_joined_room`] — the
//! `connect --join-room` shape — before the local Room → Waiting
//! enter. Without a join target Confirm stays a local enter (the
//! offline walk), and a failed or rejected join keeps the Room
//! scene with a warning, so the next Confirm retries it.
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
//! 4018 ranked table (each entry total re-staged onto its seat, so
//! the verdict turns on server-authoritative peer totals), and 4040
//! rank places (decoded and validated; the gateway never emits it —
//! the `FinalizeRank` broadcast stays commented out server-side).
//! The shell's own Result Confirm/Back edges (Waiting for retry,
//! Room for leave) stay untouched.
//!
//! Host song select (Waiting scene): `--online-music-id` stages a
//! pending pick plus the installed `OJNList.dat` catalog; the next
//! master Waiting Confirm sends `SetRoomMusic` 4000, `SetRoomArena`
//! 4002 (awaiting the 4003 echo like the headless song flow), and
//! `GetMusicList` 4030, stages the pick through
//! [`ScreenGame::set_selected_music`] (driving the dialog rows and
//! the waiting song panel), then fetches the chart from the catalog
//! container (`Music/o2ma###.ojn` through the loading gate) into the
//! highway via [`ScreenGame::load_chart`]. The room still launches on
//! a later Confirm (START 4010 behind the all-peers-ready gate), so
//! the host sees the staged pick first. Guests keep the READY path,
//! and a failed select keeps the pick for the next Confirm.
//!
//! Lobby send (Channel scene): the shell stages a second draft
//! ([`ScreenGame::take_channel_slash`]) beside the channel history
//! and transmits it on Confirm through
//! [`OnlineSession::flush_channel_action`] — plain chat as 2012, `/w
//! <name> <text>` and `/r <text>` as a 2017 whisper through
//! [`OnlineSession::flush_channel_whisper`], `/h`/`/help` with no
//! packet — the same peek-send-clear bridge as the waiting 3011 path
//! ([`OnlineSession::flush_chat_action`]).
//!
//! Lobby inbox (Channel scene): every paint drains inbound 2013
//! player lines, 2014 administrator lines, and 2019 whispers beside
//! the 2016/2029 notices, folding each into the channel history
//! through its [`ScreenGame`] push — the same best-effort shape as
//! the 4015 echo drain (undecodable bodies drop with a trace; the
//! shell push rules decide which rows stage).
//!
//! Lobby notices (Channel scene): every paint drains waiting 2016
//! announcements and 2029 rank notices under the same short timeout
//! as the stats echo window and folds each into the channel history
//! through [`ScreenGame::push_channel_announcement`] and
//! [`ScreenGame::push_channel_rank_notification`] — the same
//! best-effort shape as the 4015 echo drain: keepalives skip
//! silently, foreign ids log and skip, undecodable bodies drop with
//! a trace. Empty notices and invalid ranks stage no row (the shell
//! push rules), so the drain never invents one.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use o2jam_file::{OjnListEntry, OJN_LEVEL_COUNT};
use o2jam_game::{PeerBars, QueueEvent, RankDelivery, RoomTeams};
use o2jam_net::play::{
    GameRankUpdate, MusicListAnswer, RoomArenaEcho, ScoreCompleted, ScoreSubmission,
};
use o2jam_net::rooms::{
    Announcement, MainRoomAdminMessage, MainRoomUserMessage, RankNotification, ReceiveWhisper,
    RoomList, RoomSlotAction, RoomSlotUpdate, SendMainRoomMessage, SendWaitingMessage, SendWhisper,
    WaitingAdminMessage, WaitingUserMessage,
};
use o2jam_net::{
    ConfirmMusicLoaded, CreateResult, CreateRoom, CreateRoomResponse, ExitPlaying, FinalizeRank,
    Frame, GameMode, GameStatsUpdate, GatewayConnection, Generic, GetMusicList, JoinResult,
    JoinRoom, JoinRoomResponse, JoinedRoom, KeyMode, MemberReadyStateChanged, Ready, Request,
    SetRoomArena, SetRoomMusic, StartGame, SubmitScore, WireMessage,
};
use o2jam_shell::screens::game::SlashCommand;
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

use crate::live_art::restage_live_playing;
use crate::live_stats::{apply_peer_update, jam_tick_for_score, life_tick_for_live_play};
use crate::loading::{
    load_sound_bank, loading_phase_status, prepare_chart, stage_loading_cover, PreparedChart,
    LOADING_GATE_STEPS,
};

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
/// C->S ready toggle (4008): the joined-guest Confirm send. Empty
/// body — see [`Ready`]: all five exe send sites push a zero word
/// then the opcode (`push 0; push 0xfa8`, files `0xacebe`, `0xb24f6`,
/// `0xb36a0`, `0xb8450`, `0xb871f`). Memoryer agrees
/// (`WaitingController.Ready()` takes no request). The S->C echo is
/// 4009 (`docs/client-server-plan.md` Phase 3 pairs 4008/4009 like
/// 4000/4001). The old one-flag-byte send was a documented guess —
/// the exe sites refute it.
/// S->C start broadcast (4011): answers the host `StartGame` 4010
/// (`docs/client-server-plan.md` Phase 4 pairs 4010/4011). The id
/// alone advances the shell — no body is staged here — so no decoder
/// is needed.
const START_BROADCAST_COMMAND: u16 = 4011;

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
    /// Silent-audio rig switch: set by the launch client when the
    /// run must never touch an audio device (null-sink containment
    /// in automated rigs; memory 979fb). With `true`, bank attaches
    /// use the null backend even when hardware exists.
    silent_audio: bool,
    /// Last slot action per seat, from 3009 updates (`None` while the
    /// seat never changed). The shell exposes no per-seat write yet,
    /// so this is the bridge query its paint follow-up will consume.
    slot_actions: [Option<RoomSlotAction>; 8],
    /// Last ready flag per seat, from 4009 echoes (`None` while the
    /// seat never echoed) — same bridge shape as `slot_actions`.
    ready_echoes: [Option<bool>; 8],
}

impl OnlineSession {
    /// Pair a game with its gateway transport.
    #[must_use]
    pub fn new(game: ScreenGame, gateway: GatewayConnection) -> Self {
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
            catalog: Vec::new(),
            last_selection: None,
            local_user: String::new(),
            joined_waiting: false,
            joined_room: None,
            guest_ready: false,
            start_announced: false,
            loaded_sent: false,
            tally_sent: false,
            silent_audio: false,
            slot_actions: [None; 8],
            ready_echoes: [None; 8],
        }
    }

    /// Remember the Room Confirm join target: the `--online-join-room`
    /// number plus password, exchanged on the next Room Confirm
    /// ([`Self::join_pending_room`]).
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
    /// exchanges it ([`Self::select_song`]).
    pub fn set_pending_song(&mut self, choice: SongChoice) -> &mut Self {
        self.pending_song = Some(choice);
        self
    }

    /// Stage the song catalog (`OJNList.dat` order) into the
    /// select-music dialog and remember its container tokens for the
    /// chart fetch. Dead slots stay out, exactly like the shell
    /// staging — a missing catalog paints dialog art without rows,
    /// and the chart fetch then warns instead of inventing a path.
    pub fn set_song_catalog(&mut self, entries: &[OjnListEntry<'_>]) -> &mut Self {
        self.game.set_song_catalog(entries);
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
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        loop {
            let frame = self
                .gateway
                .next_frame()
                .map_err(|source| anyhow::anyhow!("{source}"))?;
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
    /// broadcasts skip past like [`Self::await_select_frame`], never
    /// failing the exchange.
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
        let created = CreateRoomResponse::decode_frame(&created)
            .map_err(|source| anyhow::anyhow!("{source}"))?;
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
        let listed =
            RoomList::decode_frame(&listed).map_err(|source| anyhow::anyhow!("{source}"))?;
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

    /// Run the host select exchange over a joined room: `SetRoomMusic`
    /// 4000, `SetRoomArena` 4002 with the 4003 echo assertion, then
    /// `GetMusicList` 4030 with the 4031 answer — the headless
    /// [`crate::connect::run_song_flow`] select shapes — and stage the
    /// pick through [`ScreenGame::set_selected_music`], driving the
    /// dialog rows and the waiting song panel. A successful exchange
    /// consumes the pending pick, so the next master Confirm means
    /// START instead of another select. Returns the music-list answer. Runs under the resting session timeout: the exchange is
    /// Confirm-paced, never per-frame, so it may block like the Room
    /// Confirm join.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports, or an arena echo that
    /// does not repeat the sent stage word (the room stage did not
    /// take, so nothing is staged).
    pub fn select_song(&mut self, choice: SongChoice) -> anyhow::Result<MusicListAnswer> {
        let select = SetRoomMusic {
            music_id: choice.music_id,
            difficulty: choice.difficulty,
            speed: choice.speed,
        };
        self.gateway
            .send_frame(Request::SetRoomMusic.id(), &select.encode_body())
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        let stage = SetRoomArena::new(choice.arena_id);
        self.gateway
            .send_frame(Request::SetRoomArena.id(), &stage.encode_body())
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        let mut skipped = Vec::new();
        let echo_frame = self
            .await_select_frame(RoomArenaEcho::COMMAND, &mut skipped)
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        let echo = RoomArenaEcho::decode_frame(&echo_frame)
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        if echo.arena != choice.arena_id {
            anyhow::bail!(
                "arena echo mismatch: sent {} but got {}",
                choice.arena_id,
                echo.arena
            );
        }
        self.gateway
            .send_frame(Request::GetMusicList.id(), &GetMusicList.encode_body())
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        let list_frame = self
            .await_select_frame(MusicListAnswer::COMMAND, &mut skipped)
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        let answer = MusicListAnswer::decode_frame(&list_frame)
            .map_err(|source| anyhow::anyhow!("{source}"))?;
        if !skipped.is_empty() {
            tracing::debug!(?skipped, "select skipped broadcasts");
        }
        self.game.set_selected_music(select);
        self.last_selection = Some(select);
        self.pending_song = None;
        Ok(answer)
    }

    /// Read until `expected` arrives (the headless `await_lenient`
    /// shape): keepalives stay silent; every other skipped id lands in
    /// `skipped`, never dropped quietly.
    fn await_select_frame(
        &mut self,
        expected: u16,
        skipped: &mut Vec<u16>,
    ) -> Result<Frame, o2jam_net::Error> {
        loop {
            let frame = self.gateway.next_frame()?;
            if frame.id == expected {
                return Ok(frame);
            }
            if Generic::from_id(frame.id).is_none() {
                skipped.push(frame.id);
            }
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
    /// notes emit their samples: live hardware plays through
    /// `CpalBackend` unless the silent-audio rig switch holds
    /// ([`Self::set_silent_audio`]), headless falls back to
    /// `NullBackend` (the `main.rs` windowed shape). Best-effort: an
    /// empty bank keeps the silent play shape, never failing the
    /// staged chart.
    pub fn attach_bank_for_song(&mut self, music_id: u16, format_version: f32) {
        let bank = self.bank_for_song(music_id, format_version);
        if bank.is_empty() {
            return;
        }
        let entries = bank.len();
        let backend: Box<dyn o2jam_audio::AudioBackend> = if self.silent_audio {
            Box::new(o2jam_audio::NullBackend::new())
        } else {
            match o2jam_audio::CpalBackend::try_new() {
                Ok(backend) => Box::new(backend),
                Err(_) => Box::new(o2jam_audio::NullBackend::new()),
            }
        };
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

    /// Attach the launch audio rig (a fresh backend plus the scene-BGM
    /// bank) so the loading gate sees a rig before any song bank
    /// attaches and the boot BGM cue sounds: live hardware only when
    /// `live` holds and the silent-audio rig switch is off, the null
    /// sink otherwise (audio containment).
    pub fn attach_initial_audio(&mut self, live: bool) {
        let backend: Box<dyn o2jam_audio::AudioBackend> = if live && !self.silent_audio {
            match o2jam_audio::CpalBackend::try_new() {
                Ok(backend) => Box::new(backend),
                Err(source) => {
                    eprintln!("audio unavailable ({source}); continuing silent");
                    Box::new(o2jam_audio::NullBackend::new())
                }
            }
        } else {
            Box::new(o2jam_audio::NullBackend::new())
        };
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
    pub fn flush_channel(&mut self) -> Result<bool, o2jam_net::Error> {
        let Some(draft) = self.game.channel_send_message() else {
            return Ok(false);
        };
        self.gateway
            .send_frame(SendMainRoomMessage::COMMAND, &draft.encode_body())?;
        self.game.clear_channel_input();
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
    pub fn flush_channel_whisper(&mut self, recipient: &str) -> Result<bool, o2jam_net::Error> {
        let Some(draft) = self.game.channel_whisper_message(recipient) else {
            return Ok(false);
        };
        self.gateway
            .send_frame(SendWhisper::COMMAND, &draft.encode_body())?;
        self.game.clear_channel_input();
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
        match self.game.channel_slash_command() {
            SlashCommand::Empty => Ok(false),
            SlashCommand::Help => {
                self.game.clear_channel_input();
                Ok(true)
            }
            SlashCommand::Chat(_) => self.flush_channel(),
            SlashCommand::Whisper { recipient, content } => {
                self.game.set_channel_input(&content);
                self.flush_channel_whisper(&recipient)
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

    /// Exchange the pending Room Confirm join and stage the joined
    /// snapshot: `JoinWaiting` (3002, the `connect --join-room`
    /// request shape) for the pending number and password, then the
    /// 3003 answer's room through [`ScreenGame::set_joined_room`].
    /// `true` staged one room; `false` keeps the Room scene — a
    /// failed exchange, a rejected code, or an unstaged snapshot
    /// every path warns, so no outcome is silent and the next Confirm
    /// retries the join.
    fn join_pending_room(&mut self) -> bool {
        let Some(pending) = self.pending_room.clone() else {
            return false;
        };
        let request = JoinRoom {
            number: pending.number,
            music_state: 0,
            password: pending.password,
        };
        // Skip-loop, not the strict transport exchange: live broadcasts
        // (our own RoomCreated echo included) routinely interleave the
        // answer, and no consumer folds them mid-join.
        let frame = match self.exchange_room_frame(
            Request::JoinWaiting.id(),
            &request.encode_body(),
            JoinRoomResponse::COMMAND,
        ) {
            Ok(frame) => frame,
            Err(source) => {
                tracing::warn!(%source, "room join failed; staying in Room");
                return false;
            }
        };
        let response = match JoinRoomResponse::decode_frame(&frame) {
            Ok(response) => response,
            Err(source) => {
                tracing::debug!(id = frame.id, %source, "dropping undecodable join answer");
                return false;
            }
        };
        if !matches!(
            JoinResult::from_code(response.result_code),
            Some(JoinResult::Success)
        ) {
            tracing::warn!(
                code = response.result_code,
                "room join rejected; staying in Room"
            );
            return false;
        }
        let Some(room) = response.room else {
            tracing::warn!("room join answered without a snapshot; staying in Room");
            return false;
        };
        self.joined_room = Some(room.clone());
        self.game.set_joined_room(room);
        self.joined_waiting = true;
        restage_live_playing(&mut self.game);
        true
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

    /// Master Waiting Confirm with a pending host pick: run the
    /// 4000/4002/4030 select exchange, then fetch the chart from the
    /// staged catalog, and stay in Waiting — the next Confirm sends
    /// START through the all-peers-ready gate, so the host sees the
    /// staged pick first. A failed select keeps the pick (the next
    /// Confirm retries it); a failed chart fetch warns past a staged
    /// pick (missing install files warn, manual `--chart`/`--ojm`
    /// still stages).
    fn confirm_song_select(&mut self) {
        let Some(choice) = self.pending_song else {
            return;
        };
        match self.select_song(choice) {
            Ok(answer) => {
                self.pending_song = None;
                tracing::info!(
                    music_id = choice.music_id,
                    entries = answer.entries.len(),
                    free_music = answer.free_music,
                    "song staged"
                );
                if let Err(source) = self
                    .fetch_and_stage_chart(choice.music_id, level_for_difficulty(choice.difficulty))
                {
                    tracing::warn!(%source, "chart fetch failed; pick stays staged");
                }
            }
            Err(source) => {
                tracing::warn!(%source, "song select failed; Confirm retries it");
            }
        }
    }

    /// Joined-master Confirm: send `StartGame` 4010 ([`Request::StartGame`]
    /// with the [`StartGame`] empty body — the `run_song_flow` send
    /// shape, minus the `ConfirmMusicLoaded` tail, which the Loading
    /// section owns). Blocked while any occupied peer is unready — the
    /// shell [`ScreenGame::can_start_game`] gate — with a warning and
    /// no packet. Loading waits on the 4011 broadcast
    /// ([`Self::poll_start`]), never on this send.
    fn confirm_master_start(&mut self) {
        if !self.game.can_start_game() {
            tracing::warn!("start blocked: every occupied peer must be ready");
            return;
        }
        if let Err(source) = self
            .gateway
            .send_frame(Request::StartGame.id(), &StartGame.encode_body())
        {
            tracing::warn!(%source, "start send failed; Confirm retries it");
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
        if let Err(source) = self.gateway.set_timeouts(Some(STATS_IO_TIMEOUT)) {
            tracing::debug!(%source, "stats section skipped: timeout rejected");
            return;
        }
        self.maybe_send_tick();
        self.drain_peer_echoes();
        if let Err(source) = self.gateway.set_timeouts(Some(SESSION_IO_TIMEOUT)) {
            tracing::debug!(%source, "stats timeout restore failed");
        }
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
        if let Err(source) = self.gateway.set_timeouts(Some(TALLY_IO_TIMEOUT)) {
            tracing::debug!(%source, "tally section skipped: timeout rejected");
            return;
        }
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == ScoreSubmission::COMMAND => {
                    self.apply_score_submission(&frame);
                }
                Ok(frame) if frame.id == ScoreCompleted::COMMAND => {
                    self.apply_score_completed(&frame);
                }
                Ok(frame) if frame.id == GameRankUpdate::COMMAND => {
                    self.apply_rank_update(&frame);
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-tally frame in tally drain");
                }
                Err(_) => break,
            }
        }
        if let Err(source) = self.gateway.set_timeouts(Some(SESSION_IO_TIMEOUT)) {
            tracing::debug!(%source, "tally timeout restore failed");
        }
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
        if let Err(source) = self.gateway.set_timeouts(Some(CHAT_IO_TIMEOUT)) {
            tracing::debug!(%source, "waiting chat section skipped: timeout rejected");
            return;
        }
        self.drain_waiting_chat();
        if let Err(source) = self.gateway.set_timeouts(Some(SESSION_IO_TIMEOUT)) {
            tracing::debug!(%source, "waiting chat timeout restore failed");
        }
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
    /// into the waiting history until the socket goes quiet — the
    /// 4015 echo drain's best-effort shape: keepalives skip silently,
    /// foreign ids log and skip (`ERRORS-19`: nothing is dropped
    /// without a trace).
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
                    self.start_announced = true;
                }
                Ok(frame) if frame.id == RoomSlotUpdate::COMMAND => {
                    self.apply_slot_update(&frame);
                }
                Ok(frame) if frame.id == MemberReadyStateChanged::COMMAND => {
                    self.apply_ready_echo(&frame);
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-chat frame in waiting drain");
                }
                Err(_) => break,
            }
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

    /// Per-frame notice section in `Channel`: drain waiting 2016/2029
    /// frames under the short notice timeout so a quiet gateway never
    /// stalls the frame loop — the same timeout-restore shape as
    /// [`Self::poll_stats`].
    fn poll_channel_chat(&mut self) {
        if self.game.screen() != Screen::Channel {
            return;
        }
        if let Err(source) = self.gateway.set_timeouts(Some(NOTICE_IO_TIMEOUT)) {
            tracing::debug!(%source, "notice section skipped: timeout rejected");
            return;
        }
        self.drain_channel_chat();
        if let Err(source) = self.gateway.set_timeouts(Some(SESSION_IO_TIMEOUT)) {
            tracing::debug!(%source, "notice timeout restore failed");
        }
    }

    /// Drain waiting channel lines (2013/2014), incoming whispers
    /// (2019), announcements (2016), and rank notices (2029) into the
    /// channel history until the socket goes quiet — the 4015 echo
    /// drain's best-effort shape, minus the peer mirror: each frame
    /// applies through its shell push, keepalives skip silently,
    /// foreign ids log and skip (`ERRORS-19`: nothing is dropped
    /// without a trace).
    fn drain_channel_chat(&mut self) {
        loop {
            match self.gateway.next_frame() {
                Ok(frame) if frame.id == MainRoomUserMessage::COMMAND => {
                    self.apply_channel_user_message(&frame);
                }
                Ok(frame) if frame.id == MainRoomAdminMessage::COMMAND => {
                    self.apply_channel_admin_message(&frame);
                }
                Ok(frame) if frame.id == ReceiveWhisper::COMMAND => {
                    self.apply_channel_whisper(&frame);
                }
                Ok(frame) if frame.id == Announcement::COMMAND => {
                    self.apply_announcement(&frame);
                }
                Ok(frame) if frame.id == RankNotification::COMMAND => {
                    self.apply_rank_notice(&frame);
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-chat frame in channel drain");
                }
                Err(_) => break,
            }
        }
    }

    /// Fold one 2013 frame into the channel history (undecodable
    /// bodies drop with a trace; the socket position is already past
    /// them).
    fn apply_channel_user_message(&mut self, frame: &Frame) {
        let Ok(message) = MainRoomUserMessage::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable channel line");
            return;
        };
        self.game.push_channel_user_message(&message);
    }

    /// Fold one 2014 frame into the channel history (undecodable
    /// bodies drop with a trace; the socket position is already past
    /// them).
    fn apply_channel_admin_message(&mut self, frame: &Frame) {
        let Ok(message) = MainRoomAdminMessage::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable channel line");
            return;
        };
        self.game.push_channel_admin_message(&message);
    }

    /// Fold one 2019 frame into the channel history (undecodable
    /// bodies drop with a trace; the socket position is already past
    /// them). The wire event carries only the sender, so the row
    /// names the remembered local user as the addressee — and the
    /// push remembers the sender for `/r`.
    fn apply_channel_whisper(&mut self, frame: &Frame) {
        let Ok(message) = ReceiveWhisper::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable whisper");
            return;
        };
        let recipient = self.local_user.clone();
        self.game.push_channel_whisper(&message, &recipient);
    }

    /// Fold one 2016 frame into the channel history (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    /// Empty notices stage no row — the [`ScreenGame`] push rule — so
    /// the drain never consumes a visible slot for one.
    fn apply_announcement(&mut self, frame: &Frame) {
        let Ok(notice) = Announcement::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable announcement");
            return;
        };
        self.game.push_channel_announcement(&notice);
    }

    /// Fold one 2029 frame into the channel history (undecodable bodies
    /// drop with a trace; the socket position is already past them).
    /// Invalid packets stage no row — the [`ScreenGame`] push rule —
    /// so the drain never invents a rank.
    fn apply_rank_notice(&mut self, frame: &Frame) {
        let Ok(notice) = RankNotification::decode_frame(frame) else {
            tracing::debug!(id = frame.id, "dropping undecodable rank notice");
            return;
        };
        self.game.push_channel_rank_notification(&notice);
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
            match self.flush_channel_action() {
                Ok(sent) if !sent => self.game.menu_key(key, down),
                Ok(_) => {}
                Err(source) => {
                    tracing::warn!(%source, "lobby send failed; draft kept");
                }
            }
            return;
        }
        if key == MenuKey::Confirm && self.game.screen() == Screen::Room {
            if self.pending_room.is_none() || self.join_pending_room() {
                self.game.menu_key(key, down);
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

    fn mouse_click(&mut self, x: i32, y: i32) {
        // Click evidence states (P0: every path labeled, OPEN stays open):
        // Channel rows ride reference-measured bands (exe-OPEN) but send
        // nothing either way, matching Confirm on an empty draft.
        // Room clicks mirror Confirm: join first, advance on success.
        if self.game.screen() == Screen::Room {
            if let Some(number) = self.game.room_number_at(x, y) {
                self.set_pending_room_join(number, "");
                if self.join_pending_room() {
                    self.game.mouse_click(x, y);
                }
                return;
            }
            // A click that hits no room never sends: only Confirm (with
            // an explicit `--online-join-room`) retries a pending join.
            if self.pending_room.is_none() {
                self.game.mouse_click(x, y);
            }
            return;
        }
        // Waiting confirm rides the networked Confirm gate (4008/4010
        // with the song-select/START exchange behind it); other Waiting
        // clicks stay local exactly like their menu equivalents.
        if self.game.screen() == Screen::Waiting && self.game.waiting_confirm_hit(x, y) {
            self.confirm_waiting();
            return;
        }
        // Forward canvas clicks like lane keys: Channel/Room clicks
        // advance locally with no lobby send (Confirm on an empty draft
        // sends nothing either). Waiting-confirm clicks ride the local
        // advance without the 4008 exchange, so keyboard Confirm stays
        // the verified path for starting games (OPEN).
        self.game.mouse_click(x, y);
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

    fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
        self.game.paint(frame);
        if self.game.screen() != Screen::Loading {
            self.loaded_sent = false;
        }
        self.poll_stats();
        self.poll_channel_chat();
        self.poll_waiting_chat();
        self.poll_tally();
    }
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
mod session_tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use o2jam_game::PEER_SLOT_COUNT;
    use o2jam_net::frame::{decode_frame, encode_frame};
    use o2jam_net::{
        Frame, GameStatsUpdate, GatewayConnection, StatKind, UpdateGameStats, WireMessage,
        HEADER_LEN,
    };
    use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

    use super::{level_for_difficulty, saturate_bonus, OnlineSession, SongChoice};

    /// One whole frame off the fake server's socket.
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

    /// Fake gateway on loopback: returns its port plus the server thread
    /// running `with_server` (5 s reads, like the login-flow tests).
    fn fake_gateway(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            with_server(stream);
        });
        (port, server)
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
    fn channel_session(
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
    fn room_session(
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
    fn joined_snapshot() -> o2jam_net::JoinedRoom {
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
    fn member(nickname: &str, ready: bool) -> o2jam_net::RoomMember {
        o2jam_net::RoomMember {
            nickname: nickname.to_owned(),
            level: 1,
            gender: 0,
            gem: 0,
            master: false,
            team: 0,
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

    /// Guest snapshot: our seat is 1 under a ready master at seat 0.
    fn guest_snapshot() -> o2jam_net::JoinedRoom {
        let mut room = joined_snapshot();
        room.member_id = 1;
        room.user_count = 2;
        room.slots[0].member = Some(member("host", true));
        room.slots[1].member = Some(member("guest", false));
        room
    }

    /// Game in Waiting with a staged joined snapshot, paired with a
    /// live fake gateway; the server end runs `with_server`. Stages
    /// the snapshot the Room Confirm join would have staged (that
    /// exchange is covered above) and marks the session joined, so
    /// Waiting Confirm means START/READY instead of the offline walk.
    fn joined_waiting_session(
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
    fn paint_frame(session: &mut OnlineSession) {
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        session.paint(&mut frame);
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

    #[test]
    fn channel_confirm_without_a_draft_advances_to_room() {
        let (mut session, server) = channel_session(|mut stream| {
            assert_quiet(&mut stream);
        });
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
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
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert!(session.game.is_room_master());
        assert_eq!(session.game.room_key_mode(), 2);
    }

    #[test]
    fn channel_click_advances_to_room_without_sends() {
        // The windowed live path routes canvas clicks through the
        // session wrapper: a channel-row click must reach the inner
        // game exactly like keyboard Confirm (empty draft sends nothing).
        use o2jam_net::ChannelState;
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
        session.mouse_click(400, 240);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
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
        session.mouse_click(400, 154);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
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
        assert!(session.join_pending_room());
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Room);
    }

    #[test]
    fn waiting_confirm_click_sends_ready_and_stays() {
        // Staged confirm buttons ride the networked Confirm gate (4008
        // for guests); the shell stays in Waiting for the broadcast.
        use o2jam_net::Ready;
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, Ready::COMMAND);
        });
        session.game_mut().load_scene(
            Screen::Waiting,
            vec![o2jam_shell::avatar::ScenePlacement {
                id: "IDC_BUTTON_SWAIT_READY".to_owned(),
                code: 0,
                sprite: None,
                rect: o2jam_render::Rect {
                    x: 100,
                    y: 100,
                    w: 200,
                    h: 60,
                },
            }],
            std::collections::HashMap::new(),
        );
        session.mouse_click(150, 130);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
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
        room.slots[1].member = Some(member("peer", false));
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
        ready_room.slots[1].member = Some(member("peer", true));
        session.game.set_joined_room(ready_room);
        assert!(session.game.can_start_game());
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    #[test]
    fn start_broadcast_advances_host_to_loading() {
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member("peer", true));
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

    #[test]
    fn loading_confirm_sends_4012_then_enters_playing() {
        use o2jam_net::{ConfirmMusicLoaded, Request};
        let mut room = joined_snapshot();
        room.slots[1].member = Some(member("peer", true));
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
        room.slots[1].member = Some(member("peer", true));
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
        room.slots[1].member = Some(member("peer", true));
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
        paint_frame(&mut session);
        server.join().unwrap();
        // Local-only staging said Win (200 vs empty); the 4018 table
        // restages both seats, so our team-0 total loses to team 1.
        assert_eq!(session.game.screen(), Screen::Result);
        assert_eq!(
            session.game.battle_verdict(),
            Some(o2jam_shell::screens::paint::BattleVerdict::Lose)
        );
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.member_ready(1), Some(true));
        assert_eq!(session.member_ready(0), None);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
        server.join().unwrap();
        // The 4015 echo skipped past the notice drain, so slot 2 keeps
        // its default zero instead of the echo's 1000.
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        paint_frame(&mut session);
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
        let (mut session, server) = joined_waiting_session(joined_snapshot(), |mut stream| {
            answer_select(&mut stream, 0);
            assert_quiet(&mut stream);
        });
        let answer = session.select_song(host_choice()).unwrap();
        server.join().unwrap();
        assert_eq!(answer.entries.len(), 1);
        assert!(answer.free_music);
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
        let outcome = session.select_song(host_choice());
        server.join().unwrap();
        let message = format!("{:?}", outcome.unwrap_err());
        assert!(message.contains("arena echo mismatch"), "{message}");
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

    #[test]
    fn level_maps_difficulty_codes_onto_slots() {
        assert_eq!(level_for_difficulty(0), 0);
        assert_eq!(level_for_difficulty(1), 1);
        assert_eq!(level_for_difficulty(2), 2);
        assert_eq!(level_for_difficulty(3), 2);
        assert_eq!(level_for_difficulty(9), 2);
    }
}
