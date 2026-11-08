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
//! (3009 slot, 4009 ready echo) drain with a trace; the barrier
//! proves through staged snapshots instead.
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

use std::time::Duration;

use o2jam_game::{PeerBars, QueueEvent, RankDelivery};
use o2jam_net::rooms::{
    Announcement, MainRoomAdminMessage, MainRoomUserMessage, RankNotification, ReceiveWhisper,
    SendMainRoomMessage, SendWaitingMessage, SendWhisper, WaitingAdminMessage, WaitingUserMessage,
};
use o2jam_net::{
    Frame, GameStatsUpdate, GatewayConnection, Generic, JoinResult, JoinRoom, JoinRoomResponse,
    Request, StartGame, WireMessage,
};
use o2jam_shell::screens::game::SlashCommand;
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

use crate::live_stats::{apply_peer_update, jam_tick_for_score, life_tick_for_live_play};

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
/// Resting gateway timeout outside the stats section: the transport
/// default (`client.rs` `DEFAULT_IO_TIMEOUT`), restored after every
/// stats section like `run_song_flow` restores `SONG_FLOW_IO_TIMEOUT`.
const SESSION_IO_TIMEOUT: Duration = Duration::from_secs(10);
/// C->S ready toggle (4008): the joined-guest Confirm send. The S->C
/// echo is 4009 (`docs/client-server-plan.md` Phase 3 pairs 4008/4009
/// like 4000/4001); the toggle body is one flag byte — a documented
/// guess (the ready slot byte is 1 B, `docs/relay.md` §10, and small
/// protocol ints ride `u8`, e.g. 4038): live-gateway proof stays OPEN.
const READY_COMMAND: u16 = 4008;
/// S->C ready echo (4009): drained but unfolded — no decoder staged
/// in this crate for P2, so it skips past with a trace like foreign
/// chat frames.
const READY_ECHO_COMMAND: u16 = 4009;
/// S->C slot update (3009): answers `UpdateSlot` 3008
/// (`docs/client-server-plan.md` Phase 3); drained but unfolded for
/// the same reason — the start barrier proves through staged
/// snapshots instead (see the master-barrier session test).
const SLOT_UPDATE_COMMAND: u16 = 3009;
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
    local_user: String,
    /// `true` once a Room Confirm join staged a snapshot: Waiting
    /// Confirm then means START/READY instead of the offline walk.
    joined_waiting: bool,
    /// Guest ready toggle behind `Ready` 4008 (flips per Confirm).
    guest_ready: bool,
    /// `true` once the 4011 start broadcast arrived: the next
    /// draft-free Waiting paint advances the shell to Loading.
    start_announced: bool,
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
            local_user: String::new(),
            joined_waiting: false,
            guest_ready: false,
            start_announced: false,
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
        let frame = match self.gateway.exchange(
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
        self.game.set_joined_room(room);
        self.joined_waiting = true;
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
        if self.game.is_room_master() {
            self.confirm_master_start();
        } else {
            self.confirm_guest_ready();
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

    /// Joined-guest Confirm: flip the local ready toggle and send
    /// `Ready` 4008 with the new flag byte. A failed send flips the
    /// toggle back (the keep-for-retry shape: the next Confirm sends
    /// the same value again). The shell stays in Waiting either way —
    /// Loading arrives on the 4011 broadcast.
    fn confirm_guest_ready(&mut self) {
        self.guest_ready = !self.guest_ready;
        if let Err(source) = self
            .gateway
            .send_frame(READY_COMMAND, &[u8::from(self.guest_ready)])
        {
            self.guest_ready = !self.guest_ready;
            tracing::warn!(%source, "ready send failed; Confirm retries it");
        }
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
    /// never lost to the transition. The gate passes for the host and
    /// the offline walk; a joined guest stays in Waiting — the shell
    /// exposes no force-advance past its master gate (noted gap).
    /// Leaving Waiting clears the announcement either way.
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
        self.game.menu_key(MenuKey::Confirm, true);
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
                Ok(frame) if frame.id == SLOT_UPDATE_COMMAND || frame.id == READY_ECHO_COMMAND => {
                    tracing::debug!(
                        id = frame.id,
                        "skipping member-update frame in waiting drain"
                    );
                }
                Ok(frame) if Generic::from_id(frame.id).is_some() => {}
                Ok(frame) => {
                    tracing::debug!(id = frame.id, "skipping non-chat frame in waiting drain");
                }
                Err(_) => break,
            }
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
        self.game.menu_key(key, down);
    }

    fn text_input(&mut self, text: &str) {
        self.game.text_input(text);
    }

    fn delete_text(&mut self) {
        self.game.delete_text();
    }

    fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
        self.game.paint(frame);
        self.poll_stats();
        self.poll_channel_chat();
        self.poll_waiting_chat();
    }
}

/// Saturate the hold bonus onto the tick's `i32` tail (PERF-07:
/// `TryFrom`, never `as`).
fn saturate_bonus(bonus: u64) -> i32 {
    i32::try_from(bonus).unwrap_or(i32::MAX)
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

    use super::{saturate_bonus, OnlineSession};

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

    /// Game walked Avi → Channel → Room → Waiting, paired with a live
    /// fake gateway; the server end runs `with_server`.
    fn waiting_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (port, server) = fake_gateway(with_server);
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
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
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(game, gateway);
        session.game.set_joined_room(room);
        session.joined_waiting = true;
        session.game.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Waiting);
        (session, server)
    }

    /// gateway; the server end runs `with_server`.
    fn playing_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let (port, server) = fake_gateway(with_server);
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
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
            let first = read_frame(&mut stream);
            assert_eq!(first.id, 4008);
            assert_eq!(first.payload, vec![1]);
            let second = read_frame(&mut stream);
            assert_eq!(second.id, 4008);
            assert_eq!(second.payload, vec![0]);
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
    fn guest_start_broadcast_stays_waiting_without_force_advance() {
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_quiet(&mut stream);
        });
        for _ in 0..5 {
            paint_frame(&mut session);
        }
        server.join().unwrap();
        // Gap: the shell Confirm gate passes masters only, and the
        // shell exposes no force-advance — the guest waits here until
        // that shell API lands.
        assert_eq!(session.game.screen(), Screen::Waiting);
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
        assert_eq!(session.game.channel_chat()[0].sender, "host");
        assert_eq!(session.game.channel_chat()[0].text, "M757 1");
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
}
