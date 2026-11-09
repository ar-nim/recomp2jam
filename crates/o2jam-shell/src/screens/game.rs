//! [`ScreenGame`]: the window's game loop, owning the [`ScreenFlow`]
//! and walking screens on input.

use std::collections::HashMap;

use super::super::avatar::{ojn_cover_figure, AvatarFigure, ScenePlacement, LOADING_COVER_SPRITE};
use super::super::playing::{HighwayGeometry, LivePlay, PlaySkin};
use super::super::screen::{Screen, ScreenFlow};
use super::super::GameLoop;
use super::paint::{
    battle_verdict, channel_row_hit, channel_viewport, has_room_panels, list_row_hit,
    paint_avi_scene, paint_channel_chat, paint_channel_list, paint_channel_scene,
    paint_loading_scene, paint_loading_scene_with_status, paint_lobby_backdrop,
    paint_playing_overlay, paint_result, paint_result_scene, paint_room_list, paint_room_scene,
    paint_screen, paint_waiting_chat, paint_waiting_members, paint_waiting_scene, rect_hit,
    room_panel_hit, scene_index, staged_rect_hit, BattleVerdict, ChannelChatMessage, LoadingStatus,
    ResultPaint, ResultRankRow, ResultVerdict, SceneArt, WaitingCaptions, WaitingChatMessage,
    RESULT_BACK_ID, RESULT_BACK_RECT, RESULT_RETRY_ID, RESULT_RETRY_RECT,
};
use super::side::{
    can_return_side, can_visit_side, paint_side_scene, select_chosung_cells, select_difficulty_ink,
    select_music_rows, select_music_songs, select_pager_caption, select_picked_caption,
    select_search_caption, select_status_line, waiting_song_detail_line, waiting_song_title_line,
    SelectMusicRow, SelectMusicSong, SideScreen, SELECT_ARTIST_RADIO, SELECT_CHO_SUNG_LEFT,
    SELECT_CHO_SUNG_PITCH, SELECT_CHO_SUNG_ROW_CELLS, SELECT_CHO_SUNG_Y, SELECT_FILTER_BUTTON_H,
    SELECT_FILTER_BUTTON_PITCH, SELECT_FILTER_BUTTON_TOP, SELECT_FILTER_COLUMNS,
    SELECT_GENRE_FILTERS, SELECT_HEADER_COLUMNS, SELECT_HEADER_Y, SELECT_LEVEL_FILTERS,
    SELECT_OK_BUTTON, SELECT_PAGER_RIGHT, SELECT_RESULT_TABS, SELECT_ROW_PITCH, SELECT_ROW_TOP,
    SELECT_SEARCH_BOX, SELECT_SEARCH_BUTTON, SELECT_SEARCH_Y, SELECT_STATUS_H, SELECT_STATUS_Y,
    SELECT_TABLE_HEADERS, SELECT_TABLE_LEFT, SELECT_TABLE_WIDTH, SELECT_TITLE_RADIO,
    SELECT_VISIBLE_ROWS, SELECT_WHEEL_FILTERS,
};
use crate::MenuKey;
use o2jam_audio::{AudioBackend, PcmSamples};
use o2jam_file::OjnListEntry;
use o2jam_game::{PeerBars, QueueEvent, RankDelivery, RoomTeams, PEER_SLOT_COUNT};
use o2jam_net::play::SetRoomMusic;
use o2jam_net::rooms::{
    Announcement, MainRoomAdminMessage, MainRoomUserMessage, RankNotification, ReceiveWhisper,
    SendMainRoomMessage, SendWaitingMessage, SendWhisper, WaitingAdminMessage, WaitingUserMessage,
};
use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry};
use o2jam_render::font::{draw_text_cell, Cell};
use o2jam_render::{
    draw_text, fill_rect, pack_xrgb, scale_rect_to_frame, text_width, Frame, Rect, Rgb555Image,
};

/// Stage the Playing backdrop defaults to before a room names one:
/// the exe's room entry replaces a zero stage with a real one (the
/// waiting-room picker's value, store `0x4aacea`, `docs/stage-bga.md`),
/// so the composer never carries stage 0; the first stage stands in
/// for the picker's pick (exact default untraced).
const DEFAULT_ARENA_STAGE: u16 = 1;

/// Live waiting-room chat rows kept per room (oldest first): enough for
/// the 12-row history box plus scrollback, bounded so a loud room
/// cannot grow memory without limit (PERF-07: `usize`, never `as`).
const MAX_WAITING_CHAT_ROWS: usize = 50;

/// Live channel chat rows kept per channel list (oldest first): the
/// waiting-box bound, reused so a loud channel cannot grow memory
/// without limit (PERF-07: `usize`, never `as`).
const MAX_CHANNEL_CHAT_ROWS: usize = 50;

/// Unsent chat draft cap (characters, not bytes: multibyte input
/// counts once per character so no slice ever splits UTF-8).
const MAX_CHAT_INPUT_CHARS: usize = 50;

/// Native click canvas width (the shell maps window pixels into
/// 800x600 before calling [`GameLoop::mouse_click`]).
const CLICK_W: i32 = 800;
/// Native click canvas height (see [`CLICK_W`]).
const CLICK_H: i32 = 600;

/// Waiting confirm-button candidates under the `IDC_BUTTON_SWAIT_*`
/// convention (the `START 0x16202012` / `READY 0x16202013` arms,
/// `docs/room-enums.md`): the exact staged names stay untraced, so
/// clicks only fire while staged art binds one of these ids.
const WAITING_CONFIRM_IDS: [&str; 2] = ["IDC_BUTTON_SWAIT_START", "IDC_BUTTON_SWAIT_READY"];
/// Waiting back-button candidates (same convention caveat as
/// [`WAITING_CONFIRM_IDS`]).
const WAITING_BACK_IDS: [&str; 2] = ["IDC_BUTTON_SWAIT_BACK", "IDC_BUTTON_SWAIT_EXIT"];

/// Splash auto-advance delay in milliseconds: the exe leaves
/// `STATE_AVI` on a timer (`docs/lifecycle.md` section 5), but the exact
/// duration is not pinned by evidence, so this is an approximate
/// stand-in (3 s) until the dispatch call is traced.
const SPLASH_AUTO_ADVANCE_MS: u64 = 3_000;

/// Game loop that shows each screen's scene and walks the flow.
///
/// Input wiring so far: any lane key dismisses the splash into the
/// channel screen, and the splash auto-advances there on its timer.
/// Room/waiting/loading inputs land with their UIs.
pub struct ScreenGame {
    flow: ScreenFlow,
    playing: LivePlay,
    started_at: Option<std::time::Instant>,
    audio: Option<AudioRig>,
    avatar: Option<AvatarFigure>,
    splash: Option<AvatarFigure>,
    channels: Vec<ChannelState>,
    highlight: usize,
    /// Live channel chat rows (2013/2014/2019 packets only, oldest first).
    channel_chat: Vec<ChannelChatMessage>,
    /// Unsent channel draft for the Channel input line (2012 source;
    /// whisper content source for [`Self::channel_whisper_message`]).
    channel_input: String,
    /// Last whisper peer behind `/r` (the most recent incoming 2019
    /// sender or outgoing 2017 recipient; `None` until a whisper
    /// flows either way, so a reply with no peer falls back to chat).
    last_whisper_peer: Option<String>,
    rooms: Vec<RoomListEntry>,
    room_highlight: usize,
    waiting: Option<JoinedRoom>,
    /// Live waiting-room chat rows (3012/3013 packets only, oldest first).
    waiting_chat: Vec<WaitingChatMessage>,
    /// Unsent chat draft for the input line (3011 source).
    chat_input: String,
    selected_music: Option<SetRoomMusic>,
    /// Live loading status (walk fraction + title–artist, lifecycle
    /// §9): `None` paints the indeterminate full bar.
    loading_status: Option<LoadingStatus>,
    /// Room arena stage the Playing `_` sprites resolve through
    /// (`Playing_BG<stage>.ojs` and the staged kits; see
    /// [`Self::arena_stage`]).
    arena_stage: u16,
    /// Staged select-music catalog (`OJNList.dat` order, owned copies;
    /// see [`Self::set_song_catalog`]).
    catalog_songs: Vec<SelectMusicSong>,
    /// Staged catalog tempos by song id (`OJNList.dat` row +20 BPM
    /// column, the waiting `BPM` caption source; the title table
    /// above drops the column).
    catalog_bpm: HashMap<u32, f32>,
    /// Staged catalog container tokens by song id (`OJNList.dat` row
    /// +240 filename, e.g. `o2ma757.ojm`): the album-cover path source
    /// (see [`Self::resolve_song_cover`]).
    catalog_chart_files: HashMap<u32, String>,
    /// Played song's album cover: the OJN cover section of the walked
    /// chart, decoded to RGB555 for the result music-name box (see
    /// [`Self::resolve_song_cover`]). `None` paints the staged box
    /// as-is; nothing is invented.
    song_cover: Option<Rgb555Image>,
    /// Staged inventory counts, one per skill slot (the waiting
    /// `SKILLCOUNT` source; `JoinedRoom.skills` carries the ring ids
    /// only). Zeroed until staged.
    skill_counts: [u32; 3],
    select_music_art: Option<SceneArt>,
    battle_teams: RoomTeams,
    battle_self_team: Option<u8>,
    /// Notes of the loaded chart, remembered for the grade plate's
    /// stand-in ratio (see [`Self::grade`]).
    chart_notes: u32,
    side: Option<SideScreen>,
    side_origin: Option<Screen>,
    side_placements: Vec<ScenePlacement>,
    side_figures: HashMap<String, AvatarFigure>,
    scene_art: [Option<SceneArt>; 7],
}

/// Key-sound bank plus backend for windowed play.
struct AudioRig {
    backend: Box<dyn AudioBackend>,
    bank: HashMap<u32, PcmSamples>,
}

/// Stage word behind one room arena value (the Playing applier's
/// seeding rule, exe `0x4e78db`, `docs/stage-bga.md`): the low 16
/// bits when a high word is set, the full value otherwise. `None`
/// names no stage — a zero stage (room entry replaces zero with a
/// real one, store `0x4aacea`) or a negative word.
#[must_use]
fn arena_stage_word(arena: i32) -> Option<u16> {
    let word = u32::try_from(arena).ok()?;
    let high = word >> 16;
    let stage = if high != 0 { word & 0xFFFF } else { word };
    u16::try_from(stage).ok().filter(|stage| *stage != 0)
}

/// Install `Music` path of one catalog container token: the record's
/// `.ojm` token names its chart file with the `.ojn` extension
/// (`o2ma757.ojm` → `Music/o2ma757.ojn`; the shipped tokens match
/// their chart files 14/14, both generations). `None` when the token
/// carries no `.ojm` extension (no invented fallback path).
#[must_use]
fn chart_cover_path(token: &str) -> Option<std::path::PathBuf> {
    let stem_len = token.len().checked_sub(".ojm".len())?;
    let stem = token
        .get(..stem_len)
        .filter(|_| token[stem_len..].eq_ignore_ascii_case(".ojm"))?;
    Some(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music")
            .join(format!("{stem}.ojn")),
    )
}

/// Decoded cover pixels as an RGB555 image (5-bit channels; the frame
/// blit composes in RGB555 words). `None` when the pixel buffer does
/// not tile the claimed dimensions.
#[must_use]
fn cover_image(decoded: &o2jam_file::JpegRgbImage) -> Option<Rgb555Image> {
    let pixels = decoded.width.checked_mul(decoded.height)?.checked_mul(3)?;
    if decoded.pixels.len() != pixels {
        return None;
    }
    let words = decoded
        .pixels
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| {
            let [r, g, b] = *pixel;
            (u16::from(r) >> 3) << 10 | (u16::from(g) >> 3) << 5 | u16::from(b) >> 3
        })
        .collect();
    Some(Rgb555Image {
        w: decoded.width,
        h: decoded.height,
        words,
    })
}

/// Parsed chat draft behind the lobby/waiting slash grammar: the exe
/// lobbies its 2017 whisper send behind `/w`, `/r`, `/h`, `/help`
/// literals (VAs `0x647d60`+, send at file `0x8fa02`), and the waiting
/// room sends 2017 the same way (files `0xb8ffc`/`0xb9369`). Unknown
/// commands fall back to plain chat, so only these four tokens divert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    /// The draft is empty: nothing to send, so the scene may advance.
    Empty,
    /// Plain chat: the 2012 (Channel) or 3011 (waiting) body.
    Chat(String),
    /// Whisper: the 2017 recipient plus text.
    Whisper {
        /// Named addressee (`/w` name, or the last peer for `/r`).
        recipient: String,
        /// Whisper text (the words after the name).
        content: String,
    },
    /// Help (`/h`, `/help`): local only, no packet.
    Help,
}

/// Split a `/`-draft into its command token plus the rest
/// (`/w bob hi` to `("w", " bob hi")`). `None` while the draft
/// names no command at all.
fn split_command_token(draft: &str) -> Option<(&str, &str)> {
    let body = draft.strip_prefix('/')?;
    let end = body.find(char::is_whitespace).unwrap_or(body.len());
    Some((body.get(..end)?, body.get(end..).unwrap_or("")))
}

/// Split a `/w` tail into name plus text (`"  bob   hi there "` to
/// `("bob", "hi there")`; extra gaps collapse, ends trim).
/// `None` while the name or the text is missing — the draft then
/// falls back to plain chat.
fn parse_whisper_rest(rest: &str) -> Option<(String, String)> {
    let name_start = rest.find(|c: char| !c.is_whitespace())?;
    let unprefixed = rest.get(name_start..)?;
    let name_end = unprefixed
        .find(char::is_whitespace)
        .unwrap_or(unprefixed.len());
    let recipient = unprefixed.get(..name_end)?;
    if recipient.is_empty() {
        return None;
    }
    let content = unprefixed.get(name_end..)?.trim();
    if content.is_empty() {
        return None;
    }
    Some((recipient.to_owned(), content.to_owned()))
}

/// Split a `/r` tail against the last whisper peer (the tail is the
/// whole text). `None` with no peer or no text — the draft then
/// falls back to plain chat.
fn parse_reply_text(rest: &str, reply_to: Option<&str>) -> Option<(String, String)> {
    let peer = reply_to.filter(|peer| !peer.is_empty())?;
    let content = rest.trim();
    if content.is_empty() {
        return None;
    }
    Some((peer.to_owned(), content.to_owned()))
}

/// Parse one draft through the slash grammar. The draft stays whole
/// for `Chat` (unknown or incomplete commands send as-is); `Help`
/// carries no packet; `Empty` names the empty draft.
fn parse_slash_draft(draft: &str, reply_to: Option<&str>) -> SlashCommand {
    if draft.is_empty() {
        return SlashCommand::Empty;
    }
    if !draft.starts_with('/') {
        return SlashCommand::Chat(draft.to_owned());
    }
    let Some((command, rest)) = split_command_token(draft) else {
        return SlashCommand::Chat(draft.to_owned());
    };
    match command {
        "w" => parse_whisper_rest(rest).map_or_else(
            || SlashCommand::Chat(draft.to_owned()),
            |(recipient, content)| SlashCommand::Whisper { recipient, content },
        ),
        "r" => parse_reply_text(rest, reply_to).map_or_else(
            || SlashCommand::Chat(draft.to_owned()),
            |(recipient, content)| SlashCommand::Whisper { recipient, content },
        ),
        "h" | "help" => SlashCommand::Help,
        _ => SlashCommand::Chat(draft.to_owned()),
    }
}

impl ScreenGame {
    /// Start at the splash screen. The song clock starts here as
    /// the boot instant, so splash paints measure elapsed time since
    /// construction; entering Playing re-arms it as the song clock.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flow: ScreenFlow::new(),
            playing: LivePlay::new(&[]),
            started_at: Some(std::time::Instant::now()),
            audio: None,
            avatar: None,
            splash: None,
            channels: Vec::new(),
            highlight: 0,
            channel_chat: Vec::new(),
            channel_input: String::new(),
            last_whisper_peer: None,
            rooms: Vec::new(),
            room_highlight: 0,
            waiting: None,
            waiting_chat: Vec::new(),
            chat_input: String::new(),
            selected_music: None,
            loading_status: None,
            arena_stage: DEFAULT_ARENA_STAGE,
            catalog_songs: Vec::new(),
            catalog_bpm: HashMap::new(),
            catalog_chart_files: HashMap::new(),
            song_cover: None,
            skill_counts: [0, 0, 0],
            select_music_art: None,
            battle_teams: RoomTeams::new(),
            battle_self_team: None,
            chart_notes: 0,
            side: None,
            side_origin: None,
            side_placements: Vec::new(),
            side_figures: HashMap::new(),
            scene_art: std::array::from_fn(|_| None),
        }
    }

    /// Load the result-screen avatar figure.
    pub fn load_avatar(&mut self, figure: AvatarFigure) -> &mut Self {
        self.avatar = Some(figure);
        self
    }

    /// Load real splash art (e.g. `O2mediaLogo.ojs`); the placeholder
    /// shows while none is loaded.
    pub fn load_splash(&mut self, figure: AvatarFigure) -> &mut Self {
        self.splash = Some(figure);
        self
    }

    /// Attach key sounds; struck notes play through the backend.
    /// Missing samples stay silent (no bank entry, no sound).
    pub fn with_audio(
        &mut self,
        backend: Box<dyn AudioBackend>,
        bank: HashMap<u32, PcmSamples>,
    ) -> &mut Self {
        self.audio = Some(AudioRig { backend, bank });
        self
    }

    /// Load a chart's onsets (with key-sound ids) into the highway.
    /// The selected chart's difficulty re-applies to the fresh
    /// [`LivePlay`] (see [`Self::apply_life_difficulty`]); the note
    /// count feeds the result grade's ratio.
    pub fn load_chart(&mut self, onsets: Vec<(u8, u64, u32)>) {
        self.chart_notes = u32::try_from(onsets.len()).unwrap_or(u32::MAX);
        self.playing = LivePlay::new(&onsets);
        self.apply_life_difficulty();
    }

    /// Stage real note/key art into the highway paint (procedural
    /// fallback stays on unstaged lanes).
    pub fn set_play_skin(&mut self, skin: PlaySkin) {
        self.playing.set_skin(skin);
    }

    /// Caption the avatar scene with the local player (`Lv.<n> <name>`;
    /// display only, scoring untouched).
    pub fn set_player(&mut self, level: u32, name: &str) {
        self.playing.scene_mut().set_player(level, name);
    }

    /// Caption the avatar scene song line (`X3.5 <title>`); the waiting
    /// choice re-syncs each Playing paint while one is selected.
    pub fn set_song_caption(&mut self, title: &str, speed_code: u8) {
        self.playing.scene_mut().set_song(title, speed_code);
    }

    /// Caption the VOL readout (display only).
    pub fn set_volume(&mut self, volume: u8) {
        self.playing.scene_mut().set_volume(volume);
    }

    /// Play every queued key sound through the attached backend.
    fn pump_sounds(&mut self) {
        let Some(rig) = self.audio.as_mut() else {
            self.playing.drain_sounds();
            return;
        };
        for sample in self.playing.drain_sounds() {
            if let Some(pcm) = rig.bank.get(&sample) {
                let _ = rig.backend.play_pcm(pcm);
            }
        }
    }

    /// Song position: wall clock since entering Playing (saturates).
    fn now_ms(&mut self) -> u64 {
        let start = *self.started_at.get_or_insert_with(std::time::Instant::now);
        u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Step the flow, restarting the song clock on Playing entry.
    fn enter(&mut self, next: Screen) {
        if self.flow.go(next).is_ok() && next == Screen::Playing {
            self.started_at = Some(std::time::Instant::now());
        }
    }

    /// Strike a lane at an explicit clock, playing its key sound.
    /// Tests use this; live keys read the wall clock.
    pub fn strike_at(&mut self, lane: usize, now_ms: u64) {
        self.playing.strike_lane(lane as u8, now_ms);
        self.pump_sounds();
    }

    /// Release a lane (key-up) at an explicit clock, judging the
    /// armed hold tail through the verified release path. Tests use
    /// this; live key-ups read the wall clock.
    pub fn release_at(&mut self, lane: usize, now_ms: u64) {
        self.playing.release_lane(lane as u8, now_ms);
        self.pump_sounds();
    }

    /// Load a chart with long-note spans: `(lane, head ms, tail ms,
    /// sample)` — a tail at or before its head degrades to a tap
    /// ([`LivePlay::new_holds`]). The chart's difficulty re-applies
    /// and the note count feeds the grade ratio, like
    /// [`Self::load_chart`].
    pub fn load_chart_holds(&mut self, entries: Vec<(u8, u64, u64, u32)>) {
        self.chart_notes = u32::try_from(entries.len()).unwrap_or(u32::MAX);
        self.playing = LivePlay::new_holds(&entries);
        self.apply_life_difficulty();
    }

    /// Screen currently shown.
    #[must_use]
    pub fn screen(&self) -> Screen {
        self.flow.current()
    }

    /// Channels from the last online list (empty offline). Refreshing
    /// the list clears the channel chat history: rows come from that
    /// list's 2013/2014/2016/2019/2029 packets only, never carried
    /// across lists.
    pub fn set_channels(&mut self, channels: Vec<ChannelState>) -> &mut Self {
        self.channels = channels;
        self.highlight = 0;
        self.channel_chat.clear();
        self.channel_input.clear();
        self
    }

    /// Rooms from the last online list (empty offline). Free slots are
    /// not listed: the server numbers rooms from the first free slot
    /// (so a real first room is number 0) and marks free slots with
    /// an empty title — filter on both, the data stays unfiltered
    /// upstream. Corroborated against the reference server layout
    /// (`AddRoom` first-free-slot ids, `GetRoomList` empty titles);
    /// the 7.55 display rule itself stays OPEN.
    pub fn set_rooms(&mut self, rooms: Vec<RoomListEntry>) -> &mut Self {
        self.rooms = rooms.into_iter().filter(Self::is_listed_room).collect();
        self.room_highlight = 0;
        self
    }

    /// Whether a room-list row names a real room (see [`Self::set_rooms`]).
    fn is_listed_room(room: &RoomListEntry) -> bool {
        room.number != 0 || !room.title.is_empty()
    }

    /// Remember the joined room; Waiting paints its eight seats, and
    /// the room's arena word sets the Playing backdrop stage (zero or
    /// negative words keep the current stage: the exe replaces a zero
    /// stage on room entry, it never stores one). Joining a new room
    /// clears the chat history and draft: rows come from that room's
    /// 3012/3013 packets only, never carried across rooms.
    pub fn set_joined_room(&mut self, room: JoinedRoom) -> &mut Self {
        if let Some(stage) = arena_stage_word(room.arena) {
            self.arena_stage = stage;
        }
        self.waiting = Some(room);
        self.waiting_chat.clear();
        self.chat_input.clear();
        self
    }

    /// Append one player chat line (3012) to the waiting history.
    pub fn push_waiting_user_message(&mut self, message: &WaitingUserMessage) -> &mut Self {
        self.push_chat_row(&message.sender, &message.content_text(), false);
        self
    }

    /// Append one administrator chat line (3013) to the waiting history.
    pub fn push_waiting_admin_message(&mut self, message: &WaitingAdminMessage) -> &mut Self {
        self.push_chat_row(&message.sender, &message.content_text(), true);
        self
    }

    /// Live waiting-room chat rows, oldest first (empty until a
    /// 3012/3013 packet arrives).
    #[must_use]
    pub fn waiting_chat(&self) -> &[WaitingChatMessage] {
        &self.waiting_chat
    }

    /// Append one player chat line (2013) to the channel history.
    pub fn push_channel_user_message(&mut self, message: &MainRoomUserMessage) -> &mut Self {
        self.push_channel_row(ChannelChatMessage::user(
            &message.sender,
            &message.content_text(),
        ));
        self
    }

    /// Append one administrator chat line (2014) to the channel history.
    pub fn push_channel_admin_message(&mut self, message: &MainRoomAdminMessage) -> &mut Self {
        self.push_channel_row(ChannelChatMessage::admin(
            &message.sender,
            &message.content_text(),
        ));
        self
    }

    /// Append one incoming whisper (2019) to the channel history.
    /// `recipient` names the local addressee (the wire event carries
    /// only the sender), so the row keeps its whisper context.
    pub fn push_channel_whisper(&mut self, message: &ReceiveWhisper, recipient: &str) -> &mut Self {
        self.push_channel_row(ChannelChatMessage::whisper(
            &message.sender,
            recipient,
            &message.content,
        ));
        self.remember_whisper_peer(&message.sender);
        self
    }

    /// Append one lobby announcement (2016) to the channel history:
    /// the exe fans the notice text out as `0x7de` admin-message rows
    /// (`docs/net-protocol.md`), so the row paints bare in the admin
    /// amber — no sender brackets a sender-less notice. Empty notices
    /// stage no row, so they never consume a visible slot.
    pub fn push_channel_announcement(&mut self, message: &Announcement) -> &mut Self {
        if message.content.is_empty() {
            return self;
        }
        self.push_channel_row(ChannelChatMessage::announcement(&message.content));
        self
    }

    /// Append one rank notice (2029) to the channel history, unless the
    /// packet carries no rank: the exe drops the packet when the
    /// invalid word is nonzero (`docs/net-protocol.md`). The row paints
    /// the pinned notice-board sentence (`rank_notice_board_text` field
    /// order nickname/title/rank, `docs/gifts.md`) in the admin amber;
    /// the song title resolves through the staged catalog, `M{id}`
    /// while it names no such song.
    pub fn push_channel_rank_notification(&mut self, message: &RankNotification) -> &mut Self {
        if message.invalid {
            return self;
        }
        let catalog_title = u16::try_from(message.music_id)
            .ok()
            .and_then(|id| self.catalog_song_title(id));
        let fallback;
        let song_title = match catalog_title {
            Some(title) => title,
            None => {
                fallback = format!("M{}", message.music_id);
                fallback.as_str()
            }
        };
        self.push_channel_row(ChannelChatMessage::rank(
            &message.nickname,
            song_title,
            message.ranking,
        ));
        self
    }

    /// Live channel chat rows, oldest first (empty until a
    /// 2013/2014/2016/2019/2029 packet arrives).
    #[must_use]
    pub fn channel_chat(&self) -> &[ChannelChatMessage] {
        &self.channel_chat
    }

    /// Stage the unsent input line (the 3011 source on send).
    /// Overlong lines truncate to [`MAX_CHAT_INPUT_CHARS`] characters.
    pub fn set_chat_input(&mut self, line: &str) -> &mut Self {
        self.chat_input = line.chars().take(MAX_CHAT_INPUT_CHARS).collect();
        self
    }

    /// Type one character into the waiting draft. Only the Waiting
    /// scene accepts typing; control characters never enter the line
    /// and the draft stops at [`MAX_CHAT_INPUT_CHARS`] characters.
    pub fn push_chat_char(&mut self, typed: char) {
        if self.flow.current() != Screen::Waiting || typed.is_control() {
            return;
        }
        if self.chat_input.chars().count() >= MAX_CHAT_INPUT_CHARS {
            return;
        }
        self.chat_input.push(typed);
    }

    /// Type one character into the Channel draft. Only the Channel
    /// scene accepts typing; control characters never enter the line
    /// and the draft stops at [`MAX_CHAT_INPUT_CHARS`] characters.
    pub fn push_channel_char(&mut self, typed: char) {
        if self.flow.current() != Screen::Channel || typed.is_control() {
            return;
        }
        if self.channel_input.chars().count() >= MAX_CHAT_INPUT_CHARS {
            return;
        }
        self.channel_input.push(typed);
    }

    /// Erase the last draft character (Backspace). Only the Waiting
    /// scene accepts it; an empty draft stays empty (`String::pop`
    /// removes one whole character, never half of a multibyte one).
    pub fn delete_chat_char(&mut self) {
        if self.flow.current() != Screen::Waiting {
            return;
        }
        self.chat_input.pop();
    }

    /// Erase the last Channel draft character (Backspace). Only the
    /// Channel scene accepts it; an empty draft stays empty.
    pub fn delete_channel_char(&mut self) {
        if self.flow.current() != Screen::Channel {
            return;
        }
        self.channel_input.pop();
    }
    #[must_use]
    pub fn chat_input(&self) -> &str {
        &self.chat_input
    }

    /// Outgoing chat packet for the draft (`None` while the input
    /// line is empty, so no empty send is invented). Callers send
    /// the returned body, then [`Self::clear_chat_input`].
    #[must_use]
    pub fn chat_send_message(&self) -> Option<SendWaitingMessage> {
        if self.chat_input.is_empty() {
            return None;
        }
        Some(SendWaitingMessage {
            content: self.chat_input.as_bytes().to_vec(),
        })
    }

    /// Forget the unsent input line.
    pub fn clear_chat_input(&mut self) -> &mut Self {
        self.chat_input.clear();
        self
    }

    /// Take the 3011 body for the draft and clear the input line so
    /// one Enter sends once (`None` while the draft is empty). The
    /// online session transmits the packet (see `recomp2jam::session`);
    /// the shell owns no transport, so offline takes still drop it here.
    pub fn take_chat_message(&mut self) -> Option<SendWaitingMessage> {
        let message = self.chat_send_message()?;
        self.clear_chat_input();
        Some(message)
    }

    /// Stage the unsent Channel input line (the 2012 source on send).
    /// Overlong lines truncate to [`MAX_CHAT_INPUT_CHARS`] characters.
    pub fn set_channel_input(&mut self, line: &str) -> &mut Self {
        self.channel_input = line.chars().take(MAX_CHAT_INPUT_CHARS).collect();
        self
    }

    /// Live Channel input line.
    #[must_use]
    pub fn channel_input(&self) -> &str {
        &self.channel_input
    }

    /// Outgoing lobby chat packet for the Channel draft (`None` while
    /// the input line is empty, so no empty send is invented).
    /// Callers send the returned body, then [`Self::clear_channel_input`].
    #[must_use]
    pub fn channel_send_message(&self) -> Option<SendMainRoomMessage> {
        if self.channel_input.is_empty() {
            return None;
        }
        Some(SendMainRoomMessage {
            content: self.channel_input.as_bytes().to_vec(),
        })
    }

    /// Outgoing whisper packet for the Channel draft to `recipient`
    /// (`None` while the draft or the recipient is empty, so no empty
    /// send is invented). The draft carries the whisper text; the
    /// recipient rides beside it (the wire 2017 shape).
    #[must_use]
    pub fn channel_whisper_message(&self, recipient: &str) -> Option<SendWhisper> {
        if self.channel_input.is_empty() || recipient.is_empty() {
            return None;
        }
        Some(SendWhisper {
            recipient: recipient.to_owned(),
            content: self.channel_input.clone(),
        })
    }

    /// Forget the unsent Channel input line.
    pub fn clear_channel_input(&mut self) -> &mut Self {
        self.channel_input.clear();
        self
    }

    /// Take the 2012 body for the Channel draft and clear the input
    /// line so one Enter sends once (`None` while the draft is
    /// empty). The online session transmits the packet (see
    /// `recomp2jam::session`); the shell owns no transport, so
    /// offline takes still drop it here.
    pub fn take_channel_message(&mut self) -> Option<SendMainRoomMessage> {
        let message = self.channel_send_message()?;
        self.clear_channel_input();
        Some(message)
    }

    /// Take the 2017 body for the Channel draft to `recipient` and
    /// clear the input line so one action sends once (`None` while
    /// the draft or the recipient is empty). The recipient becomes
    /// the `/r` peer, like an incoming whisper does.
    pub fn take_channel_whisper(&mut self, recipient: &str) -> Option<SendWhisper> {
        let message = self.channel_whisper_message(recipient)?;
        self.clear_channel_input();
        self.remember_whisper_peer(recipient);
        Some(message)
    }

    /// Outgoing whisper packet for the waiting draft to `recipient`
    /// (`None` while the draft or the recipient is empty, so no empty
    /// send is invented). The draft carries the whisper text; the
    /// recipient rides beside it (the wire 2017 shape, like the
    /// Channel whisper — the exe sends 2017 from the waiting room too,
    /// files `0xb8ffc`/`0xb9369`).
    #[must_use]
    pub fn chat_whisper_message(&self, recipient: &str) -> Option<SendWhisper> {
        if self.chat_input.is_empty() || recipient.is_empty() {
            return None;
        }
        Some(SendWhisper {
            recipient: recipient.to_owned(),
            content: self.chat_input.clone(),
        })
    }

    /// Take the 2017 body for the waiting draft to `recipient` and
    /// clear the input line so one action sends once (`None` while
    /// the draft or the recipient is empty). The recipient becomes
    /// the `/r` peer.
    pub fn take_chat_whisper(&mut self, recipient: &str) -> Option<SendWhisper> {
        let message = self.chat_whisper_message(recipient)?;
        self.clear_chat_input();
        self.remember_whisper_peer(recipient);
        Some(message)
    }

    /// Last whisper peer behind `/r` (`None` until a whisper flows
    /// either way).
    #[must_use]
    pub fn last_whisper_peer(&self) -> Option<&str> {
        self.last_whisper_peer.as_deref()
    }

    /// Remember one whisper peer for `/r`: the newest incoming 2019
    /// sender or outgoing 2017 recipient wins. Empty names never
    /// become the peer (a reply to nobody falls back to chat).
    pub fn remember_whisper_peer(&mut self, peer: &str) -> &mut Self {
        if !peer.is_empty() {
            self.last_whisper_peer = Some(peer.to_owned());
        }
        self
    }

    /// Peek the Channel draft through the slash grammar (the exe lobby
    /// parser beside the 2017 send at file `0x8fa02`).
    #[must_use]
    pub fn channel_slash_command(&self) -> SlashCommand {
        parse_slash_draft(&self.channel_input, self.last_whisper_peer.as_deref())
    }

    /// Peek the waiting draft through the same slash grammar (the exe
    /// sends 2017 from the waiting room too, files `0xb8ffc`/`0xb9369`).
    #[must_use]
    pub fn chat_slash_command(&self) -> SlashCommand {
        parse_slash_draft(&self.chat_input, self.last_whisper_peer.as_deref())
    }

    /// Take the Channel draft through the slash grammar and clear the
    /// input line so one Enter acts once (`Empty` leaves the empty
    /// draft alone). A whisper take remembers its recipient for `/r`.
    pub fn take_channel_slash(&mut self) -> SlashCommand {
        let command = self.channel_slash_command();
        if command != SlashCommand::Empty {
            self.clear_channel_input();
        }
        if let SlashCommand::Whisper { ref recipient, .. } = command {
            self.remember_whisper_peer(recipient);
        }
        command
    }

    /// Take the waiting draft through the slash grammar and clear the
    /// input line so one Enter acts once (`Empty` leaves the empty
    /// draft alone). A whisper take remembers its recipient for `/r`.
    pub fn take_chat_slash(&mut self) -> SlashCommand {
        let command = self.chat_slash_command();
        if command != SlashCommand::Empty {
            self.clear_chat_input();
        }
        if let SlashCommand::Whisper { ref recipient, .. } = command {
            self.remember_whisper_peer(recipient);
        }
        command
    }

    /// Append one chat row and bound the history to
    /// [`MAX_WAITING_CHAT_ROWS`] (oldest rows drop first).
    fn push_chat_row(&mut self, sender: &str, text: &str, admin: bool) {
        self.waiting_chat.push(WaitingChatMessage {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin,
        });
        self.truncate_waiting_chat();
    }

    /// Drop the oldest rows past the history cap.
    fn truncate_waiting_chat(&mut self) {
        let overflow = self
            .waiting_chat
            .len()
            .saturating_sub(MAX_WAITING_CHAT_ROWS);
        self.waiting_chat.drain(..overflow);
    }

    /// Append one channel row and bound the history to
    /// [`MAX_CHANNEL_CHAT_ROWS`] (oldest rows drop first).
    fn push_channel_row(&mut self, row: ChannelChatMessage) {
        self.channel_chat.push(row);
        self.truncate_channel_chat();
    }

    /// Drop the oldest channel rows past the history cap.
    fn truncate_channel_chat(&mut self) {
        let overflow = self
            .channel_chat
            .len()
            .saturating_sub(MAX_CHANNEL_CHAT_ROWS);
        self.channel_chat.drain(..overflow);
    }

    /// Room arena stage the Playing `_` sprites resolve through: the
    /// default until a joined room names one, the room's arena word
    /// afterwards (`docs/stage-bga.md` global `0x7789dc`). Loaders
    /// build `Playing_BG<stage>.ojs` and the stage-keyed kits from it.
    #[must_use]
    pub fn arena_stage(&self) -> u16 {
        self.arena_stage
    }

    /// Wire key mode of the joined room (7-key while none is joined):
    /// the Playing art families split members by it
    /// (`avatar::PlayingArtMode`).
    #[must_use]
    pub fn room_key_mode(&self) -> u8 {
        self.waiting
            .as_ref()
            .map_or(o2jam_net::KeyMode::SevenKeys as u8, |room| room.key_mode)
    }

    /// Whether the joined room plays team/versus (`Live`); solo
    /// (`Normal`) otherwise, including while no room is joined.
    #[must_use]
    pub fn room_team_play(&self) -> bool {
        self.waiting
            .as_ref()
            .and_then(|room| o2jam_net::GameMode::from_u8(room.game_mode).ok())
            .is_some_and(|mode| mode == o2jam_net::GameMode::Live)
    }

    /// Self room-master state: the joined room's seat id names our
    /// seat, and seat 0 is the creator — the 4043 header derives the
    /// local `+0x30` flag as 1 exactly when the requesting member id
    /// is 0 (files `0x159bdf`/`0x159c1b`/`0x159c4d`, `docs/relay.md`
    /// §10). The waiting setup swaps START/READY on this flag (file
    /// `0xae38e`: the master arm enables `START 0x16202012` and hides
    /// `READY 0x16202013`; the non-master arm at file `0xae805`
    /// reverses them), and the `MUSELECT 0x16200003` click handler
    /// drops non-master clicks (file `0xb2622`, `je 0xb268d`).
    #[must_use]
    pub fn is_room_master(&self) -> bool {
        self.waiting
            .as_ref()
            .is_some_and(|room| room.member_id == 0)
    }

    /// Every occupied peer seat ready: the `START 0x16202012` click
    /// path runs this check (file `0xb5560`) — it skips our own seat
    /// (member id at `0x77cd04`), requires each occupied seat
    /// (`[i + 0x778a16] == 1`) to carry a nonzero ready byte
    /// (`[i * 0x2190 + 0x77eec2]`, slot base `0x77ee90` + `0x32`),
    /// and returns -1 while any occupied peer is unready, so START
    /// errors instead of emitting. Vacant/locked seats (`None`)
    /// never block.
    #[must_use]
    pub fn all_peers_ready(&self) -> bool {
        let Some(room) = self.waiting.as_ref() else {
            return false;
        };
        room.slots.iter().all(|slot| {
            slot.member
                .as_ref()
                .is_none_or(|member| slot.index == room.member_id || member.ready)
        })
    }

    /// The master may start only with every peer ready; non-masters
    /// never start — they ready up (4008, owned by the session
    /// bridge), so their Confirm stays in Waiting.
    #[must_use]
    pub fn can_start_game(&self) -> bool {
        self.is_room_master() && self.all_peers_ready()
    }

    /// Run the waiting Confirm gate shared by Enter and clicks:
    /// slash-free drafts advance offline walkers and ready masters;
    /// guests and masters with unready peers stay put (see
    /// [`Self::can_start_game`]).
    fn confirm_waiting(&mut self) {
        if self.take_chat_slash() == SlashCommand::Empty
            && (self.waiting.is_none() || self.can_start_game())
        {
            self.enter(Screen::Loading);
        }
    }

    /// Advance a joined guest to Loading on the 4011 start broadcast.
    /// The `START 0x16202012` gate passes masters only (file
    /// `0xb5560`), so a guest Confirm can never satisfy
    /// [`Self::can_start_game`]; the session calls this on 4011
    /// instead of Confirm. Masters never advance here — they keep
    /// the readiness rule through Confirm — and neither do the
    /// offline walk (no joined room), other screens, or a pending
    /// draft (peek-only, so typed text is never eaten). `true`
    /// entered Loading.
    pub fn force_advance_to_loading(&mut self) -> bool {
        if self.flow.current() != Screen::Waiting
            || self.waiting.is_none()
            || self.is_room_master()
            || self.chat_slash_command() != SlashCommand::Empty
        {
            return false;
        }
        self.enter(Screen::Loading);
        true
    }

    /// Channel row click: highlight the band, then advance like
    /// Confirm while the draft is action-free (slash drafts consume
    /// the click instead of starting).
    fn click_channel_row(&mut self, _x: i32, y: i32) {
        // Viewport-relative bands: the visible slot maps back through
        // the highlight-following viewport (see `channel_viewport`).
        let (top, shown) = channel_viewport(self.highlight, self.channels.len());
        let Some(slot) = channel_row_hit(y, shown) else {
            return;
        };
        let index = top + slot;
        self.highlight = index;
        if self.take_channel_slash() == SlashCommand::Empty {
            self.enter(Screen::Room);
        }
    }

    /// Room row click: staged panels first (verbatim `.bnd` rects in
    /// file order), legacy row bands without them. Either path
    /// highlights the row and advances like Confirm.
    fn click_room_row(&mut self, x: i32, y: i32) {
        let Some(index) = self.room_index_at(x, y) else {
            return;
        };
        self.room_highlight = index;
        self.enter(Screen::Waiting);
    }

    /// Room row index under a click, staged panels first (see
    /// [`Self::click_room_row`]).
    fn room_index_at(&self, x: i32, y: i32) -> Option<usize> {
        let staged = self.room_panel_click(x, y);
        let band = || list_row_hit(CLICK_W, CLICK_H, self.rooms.len(), x, y);
        staged
            .or_else(band)
            .filter(|index| *index < self.rooms.len())
    }

    /// Room number under a click (`None` while no listed room covers
    /// the point). The session bridge joins it before advancing, so a
    /// click sends what Confirm sends.
    #[must_use]
    pub fn room_number_at(&self, x: i32, y: i32) -> Option<i32> {
        self.room_index_at(x, y)
            .and_then(|index| self.rooms.get(index))
            .map(|room| room.number)
    }

    /// Staged room-panel click: file-order panel index covering
    /// (`x`, `y`) (`None` while no bound panel covers the point).
    fn room_panel_click(&self, x: i32, y: i32) -> Option<usize> {
        let placements = &self.scene_art(Screen::Room)?.placements;
        if !has_room_panels(placements) {
            return None;
        }
        room_panel_hit(placements, x, y)
    }

    /// Whether a click hits a staged Waiting confirm button
    /// (START/READY ids only; art-less clicks land nowhere — no pinned
    /// fallback geometry names these buttons). The session bridge runs
    /// its networked Confirm gate on a hit instead of the local one.
    #[must_use]
    pub fn waiting_confirm_hit(&self, x: i32, y: i32) -> bool {
        let Some(art) = self.scene_art(Screen::Waiting) else {
            return false;
        };
        WAITING_CONFIRM_IDS
            .iter()
            .any(|id| staged_rect_hit(&art.placements, id, x, y))
    }

    /// Waiting button click: staged START/READY runs the Confirm
    /// gate, staged BACK leaves for rooms. Art-less clicks land
    /// nowhere — no pinned fallback geometry names these buttons.
    fn click_waiting_button(&mut self, x: i32, y: i32) {
        let Some(art) = self.scene_art(Screen::Waiting) else {
            return;
        };
        let placements = &art.placements;
        let back = WAITING_BACK_IDS
            .iter()
            .any(|id| staged_rect_hit(placements, id, x, y));
        let confirm = WAITING_CONFIRM_IDS
            .iter()
            .any(|id| staged_rect_hit(placements, id, x, y));
        if back {
            self.enter(Screen::Room);
        } else if confirm {
            self.confirm_waiting();
        }
    }

    /// Hit one result button: the staged `.bnd` rect verbatim where
    /// bound, the pinned `docs/cresult-anchors.md` rect otherwise.
    fn result_button_hit(&self, id: &str, fallback: &Rect, x: i32, y: i32) -> bool {
        match self.scene_art(Screen::Result) {
            Some(art) if art.placements.iter().any(|placed| placed.id == id) => {
                staged_rect_hit(&art.placements, id, x, y)
            }
            _ => rect_hit(fallback, x, y),
        }
    }

    /// Result button click: RETRY re-enters Waiting, BACK leaves
    /// for rooms — the keyboard Confirm/Back arms on mouse.
    fn click_result_button(&mut self, x: i32, y: i32) {
        if self.result_button_hit(RESULT_RETRY_ID, &RESULT_RETRY_RECT, x, y) {
            self.enter(Screen::Waiting);
        } else if self.result_button_hit(RESULT_BACK_ID, &RESULT_BACK_RECT, x, y) {
            self.enter(Screen::Room);
        }
    }

    /// Remember the host song choice (`SetRoomMusic` 4000 body);
    /// Waiting paints its title and difficulty panel, and the chosen
    /// chart's difficulty byte selects the playing life-bar deltas.
    /// Also resolves the album cover (see [`Self::resolve_song_cover`]).
    pub fn set_selected_music(&mut self, selection: SetRoomMusic) -> &mut Self {
        self.selected_music = Some(selection);
        self.apply_life_difficulty();
        self.resolve_song_cover();
        self
    }

    /// Stage a live loading status (real gate fractions only — see
    /// `recomp2jam::loading`; unknown stays `None`, never invented).
    pub fn set_loading_status(&mut self, status: LoadingStatus) -> &mut Self {
        self.loading_status = Some(status);
        self
    }

    /// Forget the walk state: Loading paints indeterminate again.
    pub fn clear_loading(&mut self) -> &mut Self {
        self.loading_status = None;
        self
    }

    /// Live loading status, if a real gate walk staged one.
    #[must_use]
    pub fn loading_status(&self) -> Option<&LoadingStatus> {
        self.loading_status.as_ref()
    }

    /// Stage the walked chart's OJN cover into the loading scene under
    /// [`LOADING_COVER_SPRITE`]: the full-bleed song illustration the
    /// loading painter stretches first, replacing the genuine
    /// `MusicLoading.ojs` planet fallback (the exe mounts that overlay
    /// only when the song load returns 0, file `0xe47a0`). `true` while
    /// the bytes decode to a cover; any miss — non-chart bytes, unknown
    /// magic, undecodable stream — drops a previously staged cover, so
    /// the miss keeps the planet fallback. Archives stay read-only;
    /// nothing is invented.
    pub fn stage_loading_cover(&mut self, chart_bytes: &[u8]) -> bool {
        match ojn_cover_figure(chart_bytes) {
            Some(figure) => {
                self.scene_art[scene_index(Screen::Loading)]
                    .get_or_insert_with(SceneArt::default)
                    .figures
                    .insert(LOADING_COVER_SPRITE.to_owned(), figure);
                true
            }
            None => {
                self.clear_loading_cover();
                false
            }
        }
    }

    /// Whether the walked chart's OJN cover is staged for the loading
    /// gate (present means the chart load succeeded, so the painter
    /// skips the planet fallback).
    #[must_use]
    pub fn has_loading_cover(&self) -> bool {
        self.scene_art[scene_index(Screen::Loading)]
            .as_ref()
            .is_some_and(|art| art.figures.contains_key(LOADING_COVER_SPRITE))
    }

    /// Drop any staged loading cover so the next paint falls back to
    /// the planet overlay.
    fn clear_loading_cover(&mut self) {
        if let Some(art) = self.scene_art[scene_index(Screen::Loading)].as_mut() {
            art.figures.remove(LOADING_COVER_SPRITE);
        }
    }

    /// Seed the playing life bar from the selected chart's difficulty
    /// (Ex/Nx/Hx → Easy/Normal/Hard deltas per `FUN_00570f90`); codes
    /// without a verified arm (Rx, unknown) keep the current set. Runs
    /// on both setup points — chart load and song selection — so the
    /// flow order never matters.
    fn apply_life_difficulty(&mut self) {
        let Some(selection) = self.selected_music.as_ref() else {
            return;
        };
        self.playing.set_life_difficulty(selection.difficulty);
    }

    /// Stage the song catalog (`OJNList.dat` order, owned copies) for
    /// the select-music table and the chosen-song caption. Dead slots
    /// stay out; a missing catalog paints dialog art without rows.
    /// The BPM column stages alongside for the waiting `BPM` caption,
    /// and the container tokens for the album-cover path (see
    /// [`Self::resolve_song_cover`]).
    pub fn set_song_catalog(&mut self, entries: &[OjnListEntry<'_>]) -> &mut Self {
        self.catalog_songs = select_music_songs(entries);
        self.catalog_bpm = entries
            .iter()
            .filter(|entry| entry.is_song())
            .map(|entry| (entry.song_id, entry.bpm))
            .collect();
        self.catalog_chart_files = entries
            .iter()
            .filter(|entry| entry.is_song())
            .map(|entry| (entry.song_id, entry.ojm_file.clone()))
            .collect();
        self.resolve_song_cover();
        self
    }

    /// Load the selected song's album cover from the staged catalog
    /// plus the install's `Music` folder. Runs on both staging points —
    /// catalog and song selection — so the flow order never matters.
    /// The selection's id names a catalog record, whose container token
    /// (`o2ma757.ojm`) swaps to the chart file (`o2ma757.ojn`; the
    /// shipped tokens match their chart files 14/14) and the OJN cover
    /// section ([`o2jam_file::ojn_cover_jpeg`],
    /// [`o2jam_file::decode_ojn_cover`], docs/file-formats/ojn.md)
    /// decodes to the box image. Any miss — no selection, no catalog
    /// record, no install, undecodable cover — leaves `None` and the
    /// staged music-name box paints as-is; nothing is invented.
    fn resolve_song_cover(&mut self) {
        self.song_cover = None;
        self.clear_loading_cover();
        let Some(selection) = self.selected_music.as_ref() else {
            return;
        };
        let Some(token) = self.catalog_chart_files.get(&u32::from(selection.music_id)) else {
            return;
        };
        let Some(path) = chart_cover_path(token) else {
            return;
        };
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Ok(cover) = o2jam_file::ojn_cover_jpeg(&bytes) else {
            return;
        };
        let Ok(decoded) = o2jam_file::decode_ojn_cover(&cover) else {
            return;
        };
        self.song_cover = cover_image(&decoded);
        // The same walked bytes feed the loading scene: the cover the
        // gate walked is the illustration the loading painter stretches.
        self.stage_loading_cover(&bytes);
    }

    /// Stage real song-select dialog art (`DAILOG_SELECTMUSIC` placements
    /// plus figures loaded through `load_scene_figures`). Waiting layers
    /// it over the seats; empty art paints nothing extra.
    pub fn load_select_music_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.select_music_art = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Staged select-music catalog songs (catalog order, dead slots
    /// filtered out). Empty without catalog data.
    #[must_use]
    pub fn select_songs(&self) -> &[SelectMusicSong] {
        &self.catalog_songs
    }

    /// Played song's album cover, if it resolved from the walked
    /// chart's OJN cover section (see [`Self::resolve_song_cover`]).
    #[must_use]
    pub fn song_cover(&self) -> Option<&Rgb555Image> {
        self.song_cover.as_ref()
    }

    /// Catalog title for one song id, if staged.
    fn catalog_song_title(&self, music_id: u16) -> Option<&str> {
        let wanted = u32::from(music_id);
        self.catalog_songs
            .iter()
            .find(|song| song.song_id == wanted)
            .map(|song| song.title.as_str())
    }

    /// Catalog title for the chosen song, if staged.
    fn selected_song_title(&self) -> Option<&str> {
        let selection = self.selected_music.as_ref()?;
        self.catalog_song_title(selection.music_id)
    }

    /// Catalog title for the joined room's song, if staged: the
    /// `MUSICNAME` caption source (room-fill string flow,
    /// `docs/channel-fill.md`); `None` paints the bare `M{id}`
    /// fallback.
    fn room_song_title(&self) -> Option<&str> {
        let room = self.waiting.as_ref()?;
        self.catalog_song_title(room.music_id)
    }

    /// Catalog BPM for one song id, if a live song record stages it.
    fn catalog_song_bpm(&self, music_id: u16) -> Option<f32> {
        self.catalog_bpm.get(&u32::from(music_id)).copied()
    }

    /// Catalog BPM for the joined room's song, if staged: the `BPM`
    /// caption source (`OJNList.dat` row +20 column); `None` paints
    /// the pinned `-0-` mismatch default.
    fn room_song_bpm(&self) -> Option<f32> {
        let room = self.waiting.as_ref()?;
        self.catalog_song_bpm(room.music_id)
    }

    /// Stage the owned-item inventory counts, one per skill slot (the
    /// waiting `SKILLCOUNT` caption source; `JoinedRoom.skills`
    /// carries the ring ids only).
    pub fn set_skill_counts(&mut self, counts: [u32; 3]) -> &mut Self {
        self.skill_counts = counts;
        self
    }

    /// Staged inventory counts, one per skill slot (zeroed until
    /// staged).
    #[must_use]
    pub fn skill_counts(&self) -> [u32; 3] {
        self.skill_counts
    }

    /// Waiting song panel rows: title row plus difficulty row.
    /// `None` while the choice or its title is missing; dialog art
    /// still paints then.
    fn selected_song_lines(&self) -> Option<[String; 2]> {
        let selection = self.selected_music.as_ref()?;
        let title = self.selected_song_title()?;
        Some([
            waiting_song_title_line(selection.music_id, title),
            waiting_song_detail_line(selection),
        ])
    }

    /// Stage battle totals (`RoomTeams` mirrors the server-side 4014/4016
    /// accumulation); the result verdict reads rank 1 from these totals.
    pub fn set_battle_teams(&mut self, teams: RoomTeams) -> &mut Self {
        self.battle_teams = teams;
        self
    }

    /// Claim the local battle team (winner art compares this seat team
    /// against rank 1; `None` clears the claim and hides the verdict).
    pub fn set_battle_self_team(&mut self, team: Option<u8>) -> &mut Self {
        self.battle_self_team = team;
        self
    }

    /// Deliver scripted rank bytes through the peer mirror (shot-walk
    /// fixture only — same honesty as the duo nicknames): seeds the
    /// holder bytes, then broadcasts the packet through the verified
    /// rank-remap rule (`PeerBars::apply_rank_broadcast`). Returns the
    /// per-position deliveries. Zero convention unchanged: unmatched
    /// bytes leave their rows zeroed and hidden.
    pub fn stage_scripted_ranks(
        &mut self,
        global: u8,
        stored: [u8; PEER_SLOT_COUNT],
        packet: [u8; PEER_SLOT_COUNT],
    ) -> Vec<RankDelivery> {
        self.playing.set_rank_holders(global, stored);
        self.playing.apply_peer_ranks(packet)
    }

    /// Live play behind the highway: the session's tick source
    /// (Life/Jam channels read score, life, and bonus through it).
    #[must_use]
    pub fn live_play(&self) -> &LivePlay {
        &self.playing
    }

    /// Live life word for the session Life channel: the avatar
    /// gauge's bar value (row 0 of [`Self::result_rank_rows`]
    /// carries points, jam level, and bonus — never life).
    #[must_use]
    pub fn life(&self) -> u16 {
        self.playing.life()
    }

    /// Fold one live 4015 echo into the Playing peer mirror: the
    /// stats write first (seating its gauge), then the 8-byte rank
    /// broadcast (filling result rows under the remap rule) — the
    /// same order the session bridge applies, so the session mirror
    /// and the painted bars never diverge. Returns the queue forward
    /// plus the rank deliveries.
    pub fn apply_peer_echo(
        &mut self,
        slot: u8,
        kind: u16,
        value: u16,
        packet: [u8; PEER_SLOT_COUNT],
    ) -> (Option<QueueEvent>, Vec<RankDelivery>) {
        let queue = self.playing.apply_peer_stats(slot, kind, value);
        let ranks = self.playing.apply_peer_ranks(packet);
        (queue, ranks)
    }

    /// Rank-1 battle verdict for the claimed local team, if known.
    #[must_use]
    pub fn battle_verdict(&self) -> Option<BattleVerdict> {
        battle_verdict(&self.battle_teams, self.battle_self_team)
    }

    /// Result verdict for the banner: solo rooms (and no room at
    /// all) take the solo banner; team battles carry the rank-1
    /// verdict when it resolves (`FUN_004c3120` branches).
    #[must_use]
    pub fn result_verdict(&self) -> ResultVerdict {
        if self.room_team_play() {
            ResultVerdict::Battle(self.battle_verdict())
        } else {
            ResultVerdict::Solo
        }
    }

    /// Saturated local points for the rank row's `%8d` field (the
    /// exe's 32-bit read; the u64 accumulator saturates — PERF-07:
    /// `TryFrom`, never `as`).
    fn saturate_rank_score(points: u64) -> u32 {
        u32::try_from(points).unwrap_or(u32::MAX)
    }

    /// Rank number for display row `position`: the `PeerBars` byte
    /// there, hidden solo (the exe draws `%d` at x=91 only with 2+
    /// players) and skipped while zero (empty seats ride zero bytes
    /// — the `live_rank_rows` convention in `side.rs`). Zero means
    /// no broadcast delivered a rank there yet: the song path never
    /// seeds stored ranks, so only a real 4015 broadcast with
    /// matching members fills this in.
    fn result_row_rank(peers: &PeerBars, position: usize, row_count: usize) -> Option<u8> {
        if row_count < 2 {
            return None;
        }
        let rank = peers.rank_row(position)?;
        (rank != 0).then_some(rank)
    }

    /// Live result rank rows: the local `Score`/`LongNoteBonus` on
    /// row 0, `PeerBars` rank bytes per position, joined-room member
    /// names in seat order (self `YOU` first — local-only, the wire
    /// sends no self nickname — then the occupied wire slots, capped
    /// at eight seats). Peer rows carry no
    /// judgements/combos/score/bonus (per-member 4014/4016 tallies
    /// stay server-side, so those columns stay unpainted there);
    /// speed is the shared room code on every named row, absent
    /// offline.
    #[must_use]
    pub fn result_rank_rows(&self) -> Vec<ResultRankRow> {
        let score = self.playing.score();
        let bonus_points = self.playing.bonus().points();
        let peers = self.playing.peers();
        let room = self.waiting.as_ref();
        let speed = room.map(|joined| joined.speed);
        let mut names = vec!["YOU".to_owned()];
        if let Some(joined) = room {
            for slot in joined.slots.iter().take(7) {
                if let Some(member) = slot.member.as_ref() {
                    names.push(member.nickname.clone());
                }
            }
        }
        let row_count = names.len().min(8);
        names
            .into_iter()
            .take(8)
            .enumerate()
            .map(|(position, name)| {
                let local = position == 0;
                ResultRankRow {
                    rank: Self::result_row_rank(peers, position, row_count),
                    name: Some(name),
                    judgements: local.then_some(score.counts),
                    combos: local.then_some((score.max_combo, score.jam.level())),
                    score: local.then_some(Self::saturate_rank_score(score.points)),
                    bonus: local.then_some(bonus_points),
                    speed,
                }
            })
            .collect()
    }

    /// Grade plate frame for the finished play, if a chart loaded.
    /// The ratio runs the live scorer total against the pinned
    /// `grade_max_reference` denominator for the chart's note word
    /// (docs/cresult-anchors.md) through the pinned table
    /// [`result_grade_frame`]. The word counts chart onsets; the
    /// exe's 26 unscored records are a documented gap.
    #[must_use]
    pub fn grade(&self) -> Option<u8> {
        let word = u16::try_from(self.chart_notes).unwrap_or(u16::MAX);
        super::paint::result_grade_frame(self.playing.score(), word)
    }

    /// Stage real main-flow art for one screen: `.bnd` placements
    /// (via `bind_scene_rects`, or full-canvas for unbound blocks)
    /// plus figures loaded through `load_scene_figures`. Paint
    /// layers the live UI over it; empty art keeps the flat fallback.
    pub fn load_scene(
        &mut self,
        screen: Screen,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.scene_art[scene_index(screen)] = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Staged art for one screen, if any.
    fn scene_art(&self, screen: Screen) -> Option<&SceneArt> {
        self.scene_art[scene_index(screen)].as_ref()
    }

    /// Open side screen, if any.
    #[must_use]
    pub fn side(&self) -> Option<SideScreen> {
        self.side
    }

    /// Stage real side art: `.bnd` placements plus figures loaded
    /// through [`load_scene_figures`](super::super::avatar::load_scene_figures).
    pub fn load_side_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.side_placements = placements;
        self.side_figures = figures;
        self
    }

    /// Open a side screen from a lobby; rejects other origins and
    /// remembers the lobby to return to.
    pub fn visit_side(&mut self, side: SideScreen) -> bool {
        if self.side.is_some() || !can_visit_side(self.flow.current(), side) {
            return false;
        }
        self.side_origin = Some(self.flow.current());
        self.side = Some(side);
        true
    }

    /// Return to the lobby that opened the side screen.
    pub fn leave_side(&mut self) -> bool {
        let (Some(side), Some(origin)) = (self.side, self.side_origin) else {
            return false;
        };
        if !can_return_side(side, origin) {
            return false;
        }
        self.side = None;
        self.side_origin = None;
        true
    }

    /// Paint with an explicit clock (tests use this; live paint reads
    /// the wall clock).
    pub fn paint_at(&mut self, frame: &mut Frame<'_>, now_ms: u64) {
        if self.side.is_some() {
            paint_side_scene(frame, &self.side_placements, &self.side_figures);
            return;
        }
        if self.flow.current() == Screen::Avi && now_ms >= SPLASH_AUTO_ADVANCE_MS {
            let _ = self.flow.go(Screen::Channel);
        }
        if self.flow.current() == Screen::Playing && self.playing.is_finished(now_ms) {
            let _ = self.flow.go(Screen::Result);
        }
        match self.flow.current() {
            Screen::Playing => self.paint_playing(frame, now_ms),
            Screen::Avi => self.paint_avi(frame),
            Screen::Channel => self.paint_channel(frame),
            Screen::Room => self.paint_room(frame),
            Screen::Waiting => self.paint_waiting(frame),
            Screen::Loading => self.paint_loading(frame),
            Screen::Result => self.paint_result(frame),
        }
    }

    /// Splash: staged logo art when present, else the flat card;
    /// the separately loaded splash figure paints on top either way.
    fn paint_avi(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Avi) {
            Some(art) => paint_avi_scene(frame, art),
            None => paint_screen(frame, Screen::Avi),
        }
        if let Some(logo) = &self.splash {
            logo.paint(frame, 0, 0);
        }
    }

    /// Channel: staged art behind the live list, flat card fallback.
    /// Live 2013/2014/2019 rows paint below the list in both cases;
    /// empty histories paint nothing.
    fn paint_channel(&self, frame: &mut Frame<'_>) {
        match (self.scene_art(Screen::Channel), self.channels.is_empty()) {
            (Some(art), false) => {
                paint_channel_scene(frame, &self.channels, self.highlight, art);
            }
            (Some(art), true) => paint_lobby_backdrop(frame, Screen::Channel, art),
            (None, false) => paint_channel_list(frame, &self.channels, self.highlight),
            (None, true) => paint_screen(frame, Screen::Channel),
        }
        paint_channel_chat(frame, &self.channel_chat, self.channels.len());
    }

    /// Room lobby: staged art behind the live list, flat card fallback.
    fn paint_room(&self, frame: &mut Frame<'_>) {
        match (self.scene_art(Screen::Room), self.rooms.is_empty()) {
            (Some(art), false) => {
                paint_room_scene(frame, &self.rooms, self.room_highlight, art);
            }
            (Some(art), true) => paint_lobby_backdrop(frame, Screen::Room, art),
            (None, false) => paint_room_list(frame, &self.rooms, self.room_highlight),
            (None, true) => paint_screen(frame, Screen::Room),
        }
    }

    /// Waiting: staged art behind the live seats, flat card fallback,
    /// then the live chat box, then the staged song-select dialog: art,
    /// its structure overlay (catalog-driven table), and the
    /// chosen-song panel. The staged MAPNAME/BPM/SKILLCOUNT captions
    /// paint from the room word, the catalog BPM column, and the
    /// staged inventory (`INSTNAME`/`NOTICE` stay unpainted, OPEN 1/2).
    ///
    /// Chat rows come from live 3012/3013 packets only
    /// ([`Self::push_waiting_user_message`]/[`Self::push_waiting_admin_message`])
    /// and paint at the `EDIT_SWAIT_CHAT 0x16300014`/`LIST_SWAIT_CHAT
    /// 0x1680002B` rects through the `TEXT_SWAIT_CHAT 0x1640003C`
    /// template stacking ([`paint_waiting_chat`]); empty histories paint
    /// nothing. The `Random` `?` cards name no per-card entries (big card
    /// on `RANDMAP 0x1650004F` → rect 78, small cards on spriteless
    /// `EFFECT_1/2` rects 68/69); `SelectMusic_Random*.ojs` bodies belong
    /// to `DAILOG_SELECTMUSIC` filter buttons. Cards stay staged art:
    /// `paint_waiting_staged` never paints those rects (see the
    /// `paint_waiting_music` docs and the chat/cards absence tests).
    fn paint_waiting(&self, frame: &mut Frame<'_>) {
        let captions = WaitingCaptions {
            song_title: self.room_song_title(),
            song_bpm: self.room_song_bpm(),
            skill_counts: self.skill_counts,
        };
        match (self.scene_art(Screen::Waiting), &self.waiting) {
            (Some(art), Some(room)) => {
                paint_waiting_scene(frame, room, art, &captions);
                paint_waiting_chat(frame, &self.waiting_chat, &self.chat_input, &art.placements);
            }
            (Some(art), None) => paint_lobby_backdrop(frame, Screen::Waiting, art),
            (None, Some(room)) => {
                paint_waiting_members(frame, room);
                paint_waiting_chat(frame, &self.waiting_chat, &self.chat_input, &[]);
            }
            (None, None) => paint_screen(frame, Screen::Waiting),
        }
        // Song selection is the room master's alone (the `MUSELECT
        // 0x16200003` click handler drops non-master clicks, file
        // `0xb2622`): only a master self sees the dialog and the
        // chosen-song panel. The staged `BPM` caption stays for
        // everyone (non-masters pick speed from it).
        if self.is_room_master() {
            self.paint_select_music_art(frame);
            self.paint_select_music_structure(frame);
            self.paint_selected_song_panel(frame);
        }
    }

    /// Paint the song-select dialog structure over the staged art:
    /// filter columns, result tabs, the catalog-driven search table,
    /// status + pager, search row, chosung grid, and the OK pill.
    /// Rows come from the staged catalog alone; no art is invented for
    /// the check or 다운/듣기/정보 icon cells (documented gaps).
    fn paint_select_music_structure(&self, frame: &mut Frame<'_>) {
        if self.select_music_art.is_none() {
            return;
        }
        let rows = select_music_rows(&self.catalog_songs);
        paint_select_filter_columns(frame);
        paint_select_result_tabs(frame, rows.len());
        paint_select_table(frame, &rows);
        paint_select_status_row(frame, rows.len());
        paint_select_search_row(frame);
        paint_select_chosung_rows(frame);
        paint_select_ok(frame);
    }

    /// Layer staged song-select dialog figures over the waiting seats.
    /// Placements without a loaded figure paint nothing.
    fn paint_select_music_art(&self, frame: &mut Frame<'_>) {
        let Some(art) = self.select_music_art.as_ref() else {
            return;
        };
        art.paint_overlay(frame);
    }

    /// Paint the chosen-song panel rows (title, then difficulty).
    /// Missing choices or titles paint nothing: art without rows.
    fn paint_selected_song_panel(&self, frame: &mut Frame<'_>) {
        let Some([title_line, detail_line]) = self.selected_song_lines() else {
            return;
        };
        Self::paint_song_panel_line(frame, 0, &title_line);
        Self::paint_song_panel_line(frame, 1, &detail_line);
    }

    /// Panel rows stacked below the seat grid: title row 0, detail row 1.
    const SONG_PANEL_ROWS: usize = 2;
    /// Row step matching the lobby list height.
    const SONG_PANEL_ROW_H: i32 = 8;

    /// Top-left of song panel row `slot` (bottom-left, below the seats).
    fn song_panel_origin(frame: &Frame<'_>, slot: usize) -> (i32, i32) {
        let width = i32::try_from(frame.w).unwrap_or(i32::MAX);
        let height = i32::try_from(frame.h).unwrap_or(i32::MAX);
        let step = i32::try_from(slot).unwrap_or(i32::MAX);
        (width / 8, height * 3 / 4 + step * Self::SONG_PANEL_ROW_H)
    }

    /// One song panel row: dark scrim plus white ink.
    fn paint_song_panel_line(frame: &mut Frame<'_>, slot: usize, text: &str) {
        if slot >= Self::SONG_PANEL_ROWS || text.is_empty() {
            return;
        }
        let (x, y) = Self::song_panel_origin(frame, slot);
        let text_w = u32::try_from(text_width(text)).unwrap_or(u32::MAX);
        fill_rect(
            frame,
            &Rect {
                x: x - 4,
                y: y - 2,
                w: text_w.saturating_add(8),
                h: 9,
            },
            pack_xrgb(0x08, 0x08, 0x10),
        );
        draw_text(frame, x, y, text, pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    /// Loading gate: staged art behind the progress bar. A staged
    /// walk fraction paints the live bar plus title–artist; unknown
    /// (no status, or no art) keeps the indeterminate full bar.
    fn paint_loading(&self, frame: &mut Frame<'_>) {
        match (
            self.scene_art(Screen::Loading),
            self.loading_status.as_ref(),
        ) {
            (Some(art), Some(status)) => {
                paint_loading_scene_with_status(frame, art, status);
            }
            (Some(art), None) => paint_loading_scene(frame, art),
            (None, _) => paint_screen(frame, Screen::Loading),
        }
    }

    /// Gameplay: highway first, then staged gauges/panels ring the
    /// frame edges while notes keep falling underneath, then the
    /// right-side avatar scene (life bar, figure, song, VOL) with the
    /// bottom tally strip over both. Missing figures paint art
    /// without avatar.
    fn paint_playing(&mut self, frame: &mut Frame<'_>, now_ms: u64) {
        let geom = HighwayGeometry::centered(frame.w, frame.h);
        self.playing.paint(frame, &geom, now_ms);
        if let Some(art) = self.scene_art(Screen::Playing) {
            paint_playing_overlay(frame, art);
        }
        self.sync_song_caption();
        self.playing
            .paint_avatar_scene(frame, self.avatar.as_ref(), now_ms);
    }

    /// Re-caption the song line from the waiting choice while one is
    /// selected (manual captions stand while nothing is selected).
    fn sync_song_caption(&mut self) {
        let Some(selection) = self.selected_music.as_ref() else {
            return;
        };
        let speed_code = selection.speed;
        let title = self.selected_song_title().unwrap_or("").to_owned();
        self.playing.scene_mut().set_song(&title, speed_code);
    }

    /// Results: staged art (`StateResult_Top/Bottom.ojs` plus
    /// `Result_WinLose.ojs` via `FUN_004c2390`, file `0xc2390`) behind
    /// the exe-selected verdict banner and grade plate frames, the
    /// avatar figure, the pinned GDI numbers from the live score,
    /// and the rank rows from live peers plus room names. The played
    /// song's album cover paints into the music-name box when it
    /// resolved (see [`Self::resolve_song_cover`]).
    fn paint_result(&self, frame: &mut Frame<'_>) {
        let album = self.song_cover.as_ref();
        match self.scene_art(Screen::Result) {
            Some(art) => paint_result_scene(
                frame,
                &ResultPaint {
                    avatar: self.avatar.as_ref(),
                    score: self.playing.score(),
                    rows: &self.result_rank_rows(),
                    art,
                    verdict: self.result_verdict(),
                    grade: self.grade(),
                    album,
                },
            ),
            None => paint_result(
                frame,
                self.avatar.as_ref(),
                self.playing.score(),
                &self.result_rank_rows(),
                album,
            ),
        }
    }
}

impl Default for ScreenGame {
    fn default() -> Self {
        Self::new()
    }
}

// --- Song-select dialog structure painters (`reference-musicselect-4x3.png`) ---

/// Filter-column headers (reference 곡레벨선택/래별선택/장르선택;
/// ASCII stand-ins).
const SELECT_FILTER_HEADERS: [&str; 3] = ["LEVEL", "KEY", "GENRE"];

/// Structure panel scrim behind the stand-in captions.
fn select_scrim() -> u32 {
    pack_xrgb(0x10, 0x10, 0x18)
}

/// Caption ink (white).
fn select_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Ink for unknown cells and the inactive tab.
fn select_dim_ink() -> u32 {
    pack_xrgb(0x90, 0x90, 0x90)
}

/// Ink of the active search tab (reference orange).
fn select_active_ink() -> u32 {
    pack_xrgb(0xFF, 0xA0, 0x20)
}

/// Staged title-radio fill (reference green dot).
fn select_radio_ink() -> u32 {
    pack_xrgb(0x30, 0xE0, 0x30)
}

/// Frame-space step `index * pitch` for native row lists (saturates).
fn select_step(index: usize, pitch: i32) -> i32 {
    i32::try_from(index).map_or(i32::MAX, |step| step.saturating_mul(pitch))
}

/// Band height for one native `pitch` (frame rects size in `u32`).
fn select_band_h(pitch: i32) -> u32 {
    u32::try_from(pitch).unwrap_or(0)
}

/// Native-pixel anchor scaled into the frame (the shared art ratios).
fn select_anchor(rect: Rect, frame: &Frame<'_>) -> Rect {
    scale_rect_to_frame(&rect, frame.w, frame.h)
}

/// One native x scaled into the frame.
fn select_x(x: i32, frame: &Frame<'_>) -> i32 {
    select_anchor(
        Rect {
            x,
            y: 0,
            w: 0,
            h: 0,
        },
        frame,
    )
    .x
}

/// One native y scaled into the frame.
fn select_y(y: i32, frame: &Frame<'_>) -> i32 {
    select_anchor(
        Rect {
            x: 0,
            y,
            w: 0,
            h: 0,
        },
        frame,
    )
    .y
}

/// One caption on a scrim panel at its native anchor.
fn paint_select_caption(frame: &mut Frame<'_>, panel: Rect, text: &str, ink: u32) {
    let spot = select_anchor(panel, frame);
    fill_rect(frame, &spot, select_scrim());
    draw_text(frame, spot.x + 2, spot.y + 2, text, ink);
}

/// The three filter columns: header band plus one button per caption
/// (song-level bands, wheel buckets, genres).
fn paint_select_filter_columns(frame: &mut Frame<'_>) {
    let filters: [&[&str]; 3] = [
        &SELECT_LEVEL_FILTERS,
        &SELECT_WHEEL_FILTERS,
        &SELECT_GENRE_FILTERS,
    ];
    for (index, column) in SELECT_FILTER_COLUMNS.iter().enumerate() {
        paint_select_column(frame, *column, SELECT_FILTER_HEADERS[index], filters[index]);
    }
}

/// One filter column: header band on the panel top, then one button
/// per caption from [`SELECT_FILTER_BUTTON_TOP`] at its pitch.
fn paint_select_column(frame: &mut Frame<'_>, panel: Rect, header: &str, captions: &[&str]) {
    let band = Rect {
        w: panel.w,
        h: SELECT_FILTER_BUTTON_H,
        ..panel
    };
    paint_select_caption(frame, band, header, select_ink());
    for (index, caption) in captions.iter().enumerate() {
        let button = Rect {
            y: SELECT_FILTER_BUTTON_TOP + select_step(index, SELECT_FILTER_BUTTON_PITCH),
            h: SELECT_FILTER_BUTTON_H,
            ..panel
        };
        paint_select_caption(frame, button, caption, select_ink());
    }
}

/// Result tabs: the search tab active (amber caption), the picked tab
/// dim at its staged-zero count.
fn paint_select_result_tabs(frame: &mut Frame<'_>, row_count: usize) {
    paint_select_caption(
        frame,
        SELECT_RESULT_TABS[0],
        &select_search_caption(row_count),
        select_active_ink(),
    );
    paint_select_caption(
        frame,
        SELECT_RESULT_TABS[1],
        select_picked_caption(),
        select_dim_ink(),
    );
}

/// Search table: header captions on a band, then up to
/// [`SELECT_VISIBLE_ROWS`] catalog rows.
fn paint_select_table(frame: &mut Frame<'_>, rows: &[SelectMusicRow<'_>]) {
    paint_select_table_header(frame);
    for (index, row) in rows.iter().take(SELECT_VISIBLE_ROWS).enumerate() {
        paint_select_table_row(frame, index, *row);
    }
}

/// Header band across the table plus the nine column captions.
fn paint_select_table_header(frame: &mut Frame<'_>) {
    let band = select_anchor(
        Rect {
            x: SELECT_TABLE_LEFT,
            y: SELECT_HEADER_Y,
            w: SELECT_TABLE_WIDTH,
            h: select_band_h(SELECT_ROW_PITCH),
        },
        frame,
    );
    fill_rect(frame, &band, select_scrim());
    let y = select_y(SELECT_HEADER_Y + 2, frame);
    for (caption, x) in SELECT_TABLE_HEADERS
        .iter()
        .zip(SELECT_HEADER_COLUMNS[1..].iter())
    {
        draw_text(frame, select_x(*x, frame), y, caption, select_ink());
    }
}

/// One search row: scrim band, difficulty letter, catalog title and
/// artist; the level and grade cells draw the unknown dash, and the
/// 다운/듣기/정보 icon cells stay blank (no staged art for them).
fn paint_select_table_row(frame: &mut Frame<'_>, index: usize, row: SelectMusicRow<'_>) {
    let top = SELECT_ROW_TOP + select_step(index, SELECT_ROW_PITCH);
    let band = select_anchor(
        Rect {
            x: SELECT_TABLE_LEFT,
            y: top,
            w: SELECT_TABLE_WIDTH,
            h: select_band_h(SELECT_ROW_PITCH - 1),
        },
        frame,
    );
    fill_rect(frame, &band, select_scrim());
    let y = select_y(top + 2, frame);
    draw_text(
        frame,
        select_x(SELECT_HEADER_COLUMNS[2], frame),
        y,
        row.letter(),
        select_difficulty_ink(row.slot),
    );
    draw_text(
        frame,
        select_x(SELECT_HEADER_COLUMNS[3], frame),
        y,
        &row.song.title,
        select_ink(),
    );
    draw_text(
        frame,
        select_x(SELECT_HEADER_COLUMNS[4], frame),
        y,
        &row.song.artist,
        select_ink(),
    );
    draw_text(
        frame,
        select_x(SELECT_HEADER_COLUMNS[5], frame),
        y,
        "-",
        select_dim_ink(),
    );
    draw_text(
        frame,
        select_x(SELECT_HEADER_COLUMNS[9], frame),
        y,
        "-",
        select_dim_ink(),
    );
}

/// Status strip: the ALL -> ALL banner plus the 1/pages pager,
/// right-aligned to [`SELECT_PAGER_RIGHT`].
fn paint_select_status_row(frame: &mut Frame<'_>, row_count: usize) {
    let band = select_anchor(
        Rect {
            x: SELECT_TABLE_LEFT,
            y: SELECT_STATUS_Y,
            w: SELECT_TABLE_WIDTH,
            h: select_band_h(SELECT_STATUS_H),
        },
        frame,
    );
    fill_rect(frame, &band, select_scrim());
    let y = select_y(SELECT_STATUS_Y + 4, frame);
    draw_text(
        frame,
        select_x(SELECT_TABLE_LEFT, frame),
        y,
        select_status_line(),
        select_ink(),
    );
    let pager = select_pager_caption(row_count);
    draw_text(
        frame,
        select_x(SELECT_PAGER_RIGHT, frame) - text_width(&pager) as i32,
        y,
        &pager,
        select_ink(),
    );
}

/// Search row: staged title radio (green), empty artist radio, edit
/// box, and the search button. Captions 곡명/작곡가/검색 draw as
/// ASCII stand-ins.
fn paint_select_search_row(frame: &mut Frame<'_>) {
    let y = select_y(SELECT_SEARCH_Y + 8, frame);
    fill_rect(
        frame,
        &select_anchor(SELECT_TITLE_RADIO, frame),
        select_radio_ink(),
    );
    fill_rect(
        frame,
        &select_anchor(SELECT_ARTIST_RADIO, frame),
        select_scrim(),
    );
    draw_text(
        frame,
        select_x(SELECT_TITLE_RADIO.x + 12, frame),
        y,
        "TITLE",
        select_ink(),
    );
    draw_text(
        frame,
        select_x(SELECT_ARTIST_RADIO.x + 12, frame),
        y,
        "ARTIST",
        select_ink(),
    );
    fill_rect(
        frame,
        &select_anchor(SELECT_SEARCH_BOX, frame),
        select_scrim(),
    );
    paint_select_caption(frame, SELECT_SEARCH_BUTTON, "SEARCH", select_ink());
}

/// Chosung grid rows: the 14 initial jamo plus the A–Z letters draw
/// at their grid slots through the embedded TTF (`Cell::Box`, the
/// 6x12 box cell); the two word cells stay blank (no caption
/// source).
fn paint_select_chosung_rows(frame: &mut Frame<'_>) {
    let cells = select_chosung_cells();
    let mut first = 0;
    for (row, count) in SELECT_CHO_SUNG_ROW_CELLS.iter().enumerate() {
        let y = select_y(SELECT_CHO_SUNG_Y[row] + 2, frame);
        for cell in 0..*count {
            let Some(letter) = cells.get(first + cell).copied().flatten() else {
                continue;
            };
            let caption = letter.to_string();
            draw_text_cell(
                frame,
                select_x(
                    SELECT_CHO_SUNG_LEFT + select_step(cell, SELECT_CHO_SUNG_PITCH),
                    frame,
                ),
                y,
                &caption,
                select_ink(),
                Cell::Box,
            );
        }
        first += count;
    }
}

/// The bottom-center OK pill: scrim plus centered white caption.
fn paint_select_ok(frame: &mut Frame<'_>) {
    let button = select_anchor(SELECT_OK_BUTTON, frame);
    fill_rect(frame, &button, select_scrim());
    let width = i32::try_from(text_width("OK")).unwrap_or(0);
    let height = i32::try_from(Cell::Box.size().1).unwrap_or(0);
    draw_text(
        frame,
        button.x + (button.w as i32 - width) / 2,
        button.y + (button.h as i32 - height) / 2,
        "OK",
        select_ink(),
    );
}

impl GameLoop for ScreenGame {
    fn lane_key(&mut self, lane: usize, down: bool) {
        if !down {
            return;
        }
        match self.flow.current() {
            Screen::Avi => {
                let _ = self.flow.go(Screen::Channel);
            }
            Screen::Playing => {
                let now = self.now_ms();
                self.strike_at(lane, now);
            }
            _ => {}
        }
    }

    fn menu_key(&mut self, key: MenuKey, down: bool) {
        if !down {
            return;
        }
        if self.side.is_some() {
            if key == MenuKey::Back {
                self.leave_side();
            }
            return;
        }
        match (self.flow.current(), key) {
            (Screen::Channel, MenuKey::Up) => {
                self.highlight = self.highlight.saturating_sub(1);
            }
            (Screen::Channel, MenuKey::Down) => {
                let last = self.channels.len().saturating_sub(1);
                self.highlight = (self.highlight + 1).min(last);
            }
            (Screen::Room, MenuKey::Up) => {
                self.room_highlight = self.room_highlight.saturating_sub(1);
            }
            (Screen::Room, MenuKey::Down) => {
                let last = self.rooms.len().saturating_sub(1);
                self.room_highlight = (self.room_highlight + 1).min(last);
            }
            (Screen::Loading, MenuKey::Confirm) => self.enter(Screen::Playing),
            (Screen::Room, MenuKey::Back) => self.enter(Screen::Channel),
            (Screen::Channel, MenuKey::Confirm) => {
                if self.take_channel_slash() == SlashCommand::Empty {
                    self.enter(Screen::Room);
                }
            }
            (Screen::Waiting, MenuKey::Back) | (Screen::Result, MenuKey::Back) => {
                self.enter(Screen::Room);
            }
            (Screen::Room, MenuKey::Confirm)
            | (Screen::Loading, MenuKey::Back)
            | (Screen::Result, MenuKey::Confirm) => self.enter(Screen::Waiting),
            // `START 0x16202012` runs the all-ready check (file
            // `0xb5560`): the master starts only with every occupied
            // peer ready, and non-masters never start — they ready up
            // (4008, owned by the session bridge), so their Confirm
            // stays in Waiting. With no joined room there is no
            // master state, and the offline walk keeps advancing.
            // Slash drafts (`/w`, `/r`, `/h`, `/help`) consume the
            // Enter without starting: only an empty draft advances.
            (Screen::Waiting, MenuKey::Confirm) => self.confirm_waiting(),
            _ => {}
        }
    }

    fn text_input(&mut self, text: &str) {
        match self.flow.current() {
            Screen::Waiting => {
                for typed in text.chars() {
                    self.push_chat_char(typed);
                }
            }
            Screen::Channel => {
                for typed in text.chars() {
                    self.push_channel_char(typed);
                }
            }
            _ => {}
        }
    }

    fn delete_text(&mut self) {
        match self.flow.current() {
            Screen::Waiting => self.delete_chat_char(),
            Screen::Channel => self.delete_channel_char(),
            _ => {}
        }
    }

    fn mouse_click(&mut self, x: i32, y: i32) {
        if self.side.is_some() {
            return;
        }
        match self.flow.current() {
            Screen::Avi => {
                let _ = self.flow.go(Screen::Channel);
            }
            Screen::Channel => self.click_channel_row(x, y),
            Screen::Room => self.click_room_row(x, y),
            Screen::Waiting => self.click_waiting_button(x, y),
            Screen::Result => self.click_result_button(x, y),
            Screen::Loading | Screen::Playing => {}
        }
    }

    fn paint(&mut self, frame: &mut Frame<'_>) {
        let now = self.now_ms();
        self.paint_at(frame, now);
    }
}

#[cfg(test)]
mod game_tests {
    use super::super::paint::{
        background_of, channel_chat_row_text, rank_notice_board_text, seat_origin, RESULT_MUSIC_BOX,
    };
    use super::*;
    use crate::avatar::AvatarFigure;
    use crate::screen::Screen;
    use crate::{GameLoop, MenuKey};
    use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
    use o2jam_render::{has_ink_near, near_xrgb};
    use o2jam_render::{pack_xrgb, Frame};
    use std::path::PathBuf;

    #[test]
    fn loaded_chart_scales_notes_down_the_highway() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 1500, 7)]);
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
        ] {
            game.flow.go(next).unwrap();
        }
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            1000,
        );
        let note_y = 48 - (500.0 * 48.0 / 2000.0) as i32;
        // Lane 2 spans x 4..6 at 70 wide (native x55 w28); the note
        // insets one pixel each side.
        assert_eq!(buf[note_y as usize * 70 + 4], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn strikes_play_banked_samples() {
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut game = ScreenGame::new();
        let bank = HashMap::from([(
            7u32,
            PcmSamples {
                samples: vec![0i16; 8],
                sample_rate: 8000,
                channels: 1,
            },
        )]);
        game.with_audio(Box::new(NullBackend::new()), bank);
        game.load_chart(vec![(2, 5000, 7), (2, 6000, 999)]);
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
        ] {
            game.flow.go(next).unwrap();
        }
        game.strike_at(2, 5000);
        assert_eq!(game.playing.score().combo, 1);
        assert!(game.playing.drain_sounds().is_empty());
    }

    #[test]
    fn result_paints_loaded_avatar_over_card() {
        use o2jam_file::{parse_sprite55, SPRITE55_MAGIC};
        // 2x1 sprite-55: transparent black, then red.
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 2u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x7C]);
        let sprite = parse_sprite55(&blob).unwrap();
        let mut game = ScreenGame::new();
        game.load_avatar(AvatarFigure::from_sprite55(&sprite));
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            game.flow.go(next).unwrap();
        }
        // 256x256 result: card spans 96..160 both ways and the
        // figure paints at (96, 96); the red pixel lands at
        // (97, 96), the transparent one keeps card gray.
        let mut buf = vec![0u32; 256 * 256];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 256,
                h: 256,
            },
            0,
        );
        assert_eq!(buf[96 * 256 + 97], pack_xrgb(255, 0, 0));
        assert_eq!(buf[96 * 256 + 96], pack_xrgb(0xC0, 0xA0, 0x20));
    }

    fn walk_to_playing(game: &mut ScreenGame) {
        use MenuKey;
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
    }

    #[test]
    fn menu_walks_lobbies_to_playing() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
        assert!(game.started_at.is_some());
    }

    #[test]
    fn menu_backs_out_of_lobbies() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn chart_end_moves_to_result_with_score() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            7000,
        );
        assert_eq!(game.screen(), Screen::Result);
        // 70x60 is not the native canvas, so the pinned number
        // fields wait (frame_is_native); the score lives in state
        // (`playing.score()`).
        assert_ne!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn unfinished_chart_stays_playing() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            4995,
        );
        assert_eq!(game.screen(), Screen::Playing);
    }

    #[test]
    fn life_read_reports_the_avatar_gauge_word() {
        // Life-channel seam: full until play moves it, always the
        // bar word behind the gauge (rank row 0 never carries it).
        let mut game = ScreenGame::new();
        assert_eq!(game.life(), o2jam_game::LIFE_MAX);
        game.load_chart(vec![(0, 1000, 7), (0, 2000, 7)]);
        game.strike_at(0, 1000);
        assert_eq!(game.life(), game.live_play().score().life.life());
    }

    #[test]
    fn peer_echo_seats_a_gauge_in_playing_paint() {
        // Session-mirror wiring: one Life echo seats slot 0 (queue
        // forward plus the all-zero rank broadcast), and the Playing
        // paint fills its top-right gauge green.
        let mut game = ScreenGame::new();
        game.load_chart(vec![(0, 5000, 7)]);
        walk_to_playing(&mut game);
        let (queue, ranks) =
            game.apply_peer_echo(0, o2jam_game::STAT_LIFE, o2jam_game::LIFE_MAX, [0; 8]);
        assert!(queue.is_some());
        assert_eq!(ranks.len(), 8);
        assert_eq!(game.live_play().peers().life(0), Some(o2jam_game::LIFE_MAX));
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            0,
        );
        assert_eq!(buf[65], pack_xrgb(0x20, 0xE0, 0x20));
    }

    fn two_channels() -> Vec<ChannelState> {
        vec![
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
        ]
    }

    #[test]
    fn highlight_moves_with_bounds() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Up, true);
        game.menu_key(MenuKey::Up, true);
        game.menu_key(MenuKey::Up, true);
        // Down,Down clamps at 1; three Ups clamp back at 0.
        let mut probe = ScreenGame::new();
        probe.set_channels(two_channels());
        probe.lane_key(0, true);
        probe.menu_key(MenuKey::Down, true);
        probe.menu_key(MenuKey::Down, true);
        assert_eq!(game.highlight, 0);
        assert_eq!(probe.highlight, 1);
        // Empty list never moves.
        let mut bare = ScreenGame::new();
        bare.lane_key(0, true);
        bare.menu_key(MenuKey::Down, true);
        assert_eq!(bare.highlight, 0);
    }

    #[test]
    fn channel_confirm_enters_highlighted_channel() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Down, true);
        assert_eq!(game.highlight, 1);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        assert_eq!(game.highlight, 1);
    }

    fn waiting_game_with_button(id: &str, rect: Rect) -> ScreenGame {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.load_scene(
            Screen::Waiting,
            vec![ScenePlacement {
                id: id.to_owned(),
                code: 0,
                sprite: None,
                rect,
            }],
            HashMap::new(),
        );
        game
    }

    #[test]
    fn click_dismisses_splash_into_channels() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        game.mouse_click(400, 300);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn click_selects_channel_row_and_enters_rooms() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        // Second row band: y in [282, 332), x anywhere on the row.
        game.mouse_click(300, 290);
        assert_eq!(game.highlight, 1);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn click_outside_channel_rows_keeps_channel() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.mouse_click(10, 10);
        game.mouse_click(700, 152);
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn click_with_slash_draft_consumes_without_advancing() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.set_channel_input("/w bob hi");
        game.mouse_click(300, 240);
        assert_eq!(game.screen(), Screen::Channel);
        assert!(game.channel_input().is_empty());
    }

    #[test]
    fn click_selects_room_row_and_enters_waiting() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        // First row band: y in [150, 158).
        game.mouse_click(300, 154);
        assert_eq!(game.room_highlight, 0);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn click_selects_staged_room_panel() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        // Left-column second panel, file order index 1
        // (`IDC_BUTTON_SROOM_EXBUTTON2`, (26,177)-(314,268)).
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                id: "IDC_BUTTON_SROOM_EXBUTTON2".to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 26,
                    y: 177,
                    w: 288,
                    h: 91,
                },
            }],
            HashMap::new(),
        );
        game.mouse_click(100, 200);
        assert_eq!(game.room_highlight, 1);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn click_staged_ready_enters_loading() {
        let mut game = waiting_game_with_button(
            "IDC_BUTTON_SWAIT_READY",
            Rect {
                x: 600,
                y: 100,
                w: 80,
                h: 30,
            },
        );
        game.mouse_click(640, 115);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn click_staged_back_returns_to_rooms() {
        let mut game = waiting_game_with_button(
            "IDC_BUTTON_SWAIT_BACK",
            Rect {
                x: 20,
                y: 560,
                w: 80,
                h: 30,
            },
        );
        game.mouse_click(60, 575);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn click_without_staged_buttons_ignores_waiting() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.mouse_click(640, 115);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    fn result_game() -> ScreenGame {
        let mut game = ScreenGame::new();
        for next in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            game.flow.go(next).unwrap();
        }
        assert_eq!(game.screen(), Screen::Result);
        game
    }

    #[test]
    fn click_retries_result_into_waiting() {
        let mut game = result_game();
        // Pinned RETRY rect (625,570)-(708,592).
        game.mouse_click(666, 581);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn click_leaves_result_into_rooms() {
        let mut game = result_game();
        // Pinned BACK rect (711,569)-(794,593).
        game.mouse_click(752, 581);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn click_outside_result_buttons_keeps_result() {
        let mut game = result_game();
        game.mouse_click(400, 300);
        assert_eq!(game.screen(), Screen::Result);
    }

    #[test]
    fn channel_list_paints_text_and_cursor() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        // 64x48 list: rows from y 18, text x 16; '2' top bar first.
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        // 64x48 list: rows from y 18, text x 16; the row text inks
        // near its origin, the amber cursor fills solid at x 10.
        assert!(white_near(&buf, 64, 48, (16, 18)));
        assert_eq!(buf[18 * 64 + 10], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[0], background_of(Screen::Channel));
    }

    fn two_rooms() -> Vec<RoomListEntry> {
        vec![
            RoomListEntry {
                number: 3,
                state_code: 1,
                title: "Jam".to_owned(),
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
            },
            RoomListEntry {
                number: 7,
                state_code: 1,
                title: "Locked".to_owned(),
                has_password: true,
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
            },
        ]
    }

    #[test]
    fn room_highlight_moves_with_bounds() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Down, true);
        assert_eq!(game.room_highlight, 1);
        game.menu_key(MenuKey::Up, true);
        assert_eq!(game.room_highlight, 0);
    }

    #[test]
    fn room_highlight_selects_details_panel() {
        // 200x150 fits all nine fallback detail lines; the open room
        // paints OPEN dim while the locked room paints LOCK red.
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(super::MenuKey::Confirm, true);
        let mut buf = vec![0u32; 200 * 150];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0x90, 0x90, 0x90)));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0x50, 0x50)));
        game.menu_key(super::MenuKey::Down, true);
        let mut buf = vec![0u32; 200 * 150];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0x50, 0x50)));
        // List plus cursor keep working under the panel.
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn room_list_paints_rows_and_cursor() {
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(super::MenuKey::Confirm, true);
        // 64x48 list: rows from y 12, text x 16; '3' top bar first.
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        // 64x48 list: rows from y 12, text x 16; the row text inks
        // near its origin, the amber cursor fills solid at x 10.
        assert!(white_near(&buf, 64, 48, (16, 12)));
        assert_eq!(buf[12 * 64 + 10], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[0], background_of(Screen::Room));
    }

    fn waiting_room() -> JoinedRoom {
        let member = RoomMember {
            nickname: "host".to_owned(),
            level: 9,
            gender: 0,
            gem: 0,
            master: true,
            team: 0,
            ready: true,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        };
        JoinedRoom {
            member_id: 0,
            team: 1,
            title: "Jam".to_owned(),
            music_id: 0,
            arena: 0,
            key_mode: 2,
            game_mode: 0,
            difficulty: 0,
            speed: 1,
            has_super_manager: false,
            user_count: 2,
            slots: vec![RoomSlot {
                index: 0,
                state_code: 1,
                member: Some(member),
            }],
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    fn walk_to_waiting(game: &mut ScreenGame) {
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn waiting_paints_joined_members_with_ready_flags() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 80 * 64];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Waiting));
        // Ready host sits at seat 1: green ready dot left of its row.
        let (x, y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(y + 1) as usize * 80 + (x - 6 + 1) as usize],
            pack_xrgb(0x30, 0xE0, 0x30)
        );
    }

    #[test]
    fn waiting_without_room_paints_lobby_card() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 16 * 12];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 16,
                h: 12,
            },
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Waiting));
    }

    fn waiting_song_catalog_blob() -> Vec<u8> {
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&757u32.to_le_bytes());
        rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
        rec[112..117].copy_from_slice(b"First");
        rec[176..180].copy_from_slice(b"Band");
        rec[240..251].copy_from_slice(b"o2ma757.ojm");
        rec
    }

    fn waiting_song_selection() -> o2jam_net::play::SetRoomMusic {
        o2jam_net::play::SetRoomMusic {
            music_id: 757,
            difficulty: 2,
            speed: 3,
        }
    }

    fn stage_waiting_song(game: &mut ScreenGame) {
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_selected_music(waiting_song_selection());
    }

    #[test]
    fn chart_cover_path_maps_the_catalog_token_to_the_chart_file() {
        // The record's `.ojm` token names its chart file with the
        // `.ojn` extension under the install's Music folder (14/14
        // shipped tokens match); case-blind like the Windows client;
        // a non-token names no path.
        let numbered = chart_cover_path("o2ma757.ojm").unwrap();
        assert!(numbered
            .to_str()
            .is_some_and(|path| path.ends_with("Music/o2ma757.ojn")));
        let old = chart_cover_path("Tutorial.ojm").unwrap();
        assert!(old
            .to_str()
            .is_some_and(|path| path.ends_with("Music/Tutorial.ojn")));
        let upper = chart_cover_path("O2MA999.OJM").unwrap();
        assert!(upper
            .to_str()
            .is_some_and(|path| path.ends_with("Music/O2MA999.ojn")));
        assert!(chart_cover_path("bgEffect.zip").is_none());
    }

    #[test]
    fn cover_image_converts_jpeg_pixels_to_rgb555() {
        let decoded = o2jam_file::JpegRgbImage {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 0, 255, 0],
        };
        let image = cover_image(&decoded).unwrap();
        assert_eq!((image.w, image.h), (2, 1));
        assert_eq!(image.words, vec![0x7C00, 0x03E0]);
        let short = o2jam_file::JpegRgbImage {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 0, 255],
        };
        assert!(cover_image(&short).is_none());
    }

    #[test]
    fn selection_without_a_catalog_record_keeps_no_cover() {
        // The cover path runs through the staged catalog's container
        // token; a selection with no staged record resolves nothing
        // (regardless of the install), so the box stays as-is.
        let mut game = ScreenGame::new();
        game.set_selected_music(waiting_song_selection());
        assert!(game.song_cover().is_none());
    }

    #[test]
    fn staged_catalog_and_selection_load_the_walked_cover() {
        // End to end on a real install: the 757 token maps to
        // `Music/o2ma757.ojn`, whose OJN cover section decodes to the
        // 800x600 album art (docs/file-formats/ojn.md). Skips when
        // `install-root` is absent.
        let chart = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(_) = std::fs::read(&chart) else {
            return;
        };
        let mut game = ScreenGame::new();
        stage_waiting_song(&mut game);
        let cover = game.song_cover().expect("the walked cover resolves");
        assert_eq!((cover.w, cover.h), (800, 600));
    }

    #[test]
    fn result_paint_layers_the_cover_over_the_scene_ink() {
        // The full loop: the result scene paints, the cover fills the
        // box interior at its pinned rect (dest (0,0) samples the
        // cover's own (0,0)), driven by the staged catalog + selection.
        // Skips when `install-root` is absent.
        let chart = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&chart) else {
            return;
        };
        let cover_bytes = o2jam_file::ojn_cover_jpeg(&bytes).unwrap();
        let cover = cover_image(&o2jam_file::decode_ojn_cover(&cover_bytes).unwrap());
        let mut game = ScreenGame::new();
        stage_waiting_song(&mut game);
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            7_000,
        );
        assert_eq!(game.screen(), Screen::Result);
        let at =
            |x: i32, y: i32| buf[usize::try_from(y).unwrap() * 800 + usize::try_from(x).unwrap()];
        // The box interior's top-left pixel carries the cover's
        // top-left ink, widened to the frame (the wiring target).
        let word = cover.as_ref().expect("cover resolves").words[0];
        let (r, g, b) = (
            (((word >> 10) & 31) as u8) << 3 | (((word >> 10) & 31) as u8) >> 2,
            (((word >> 5) & 31) as u8) << 3 | (((word >> 5) & 31) as u8) >> 2,
            ((word & 31) as u8) << 3 | ((word & 31) as u8) >> 2,
        );
        assert_eq!(
            at(RESULT_MUSIC_BOX.x, RESULT_MUSIC_BOX.y),
            pack_xrgb(r, g, b)
        );
    }

    fn paint_waiting_frame(game: &mut ScreenGame, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            0,
        );
        buf
    }

    #[test]
    fn waiting_selected_song_paints_title_and_difficulty_rows() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // Panel sits below the seats: title `M...` at (10, 48),
        // detail `H...` at (10, 56); each line inks inside its row.
        assert!(ink_near(
            &buf,
            80,
            64,
            (10, 48),
            (20, 8),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
        assert!(ink_near(
            &buf,
            80,
            64,
            (10, 56),
            (20, 8),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
        // Seats keep painting under the panel.
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    /// Staged waiting covers plus the `MUSICNAME` top-bar rect
    /// (rects 60 and 22), no figures: TEXT-only stage for the song
    /// caption tests.
    fn musicname_scene() -> (Vec<ScenePlacement>, HashMap<String, AvatarFigure>) {
        (
            vec![
                ScenePlacement {
                    id: "IDC_IMAGE_SWAIT_COVERAVATAR1".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 28,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                },
                ScenePlacement {
                    id: "IDC_TEXT_SWAIT_MUSICNAME".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 573,
                        y: 15,
                        w: 200,
                        h: 23,
                    },
                },
            ],
            HashMap::new(),
        )
    }

    /// Waiting game on the staged TEXT scene with the room naming
    /// song 757, catalog optional.
    fn musicname_game(with_catalog: bool) -> ScreenGame {
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.music_id = 757;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        if with_catalog {
            let blob = waiting_song_catalog_blob();
            let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
            game.set_song_catalog(&entries);
        }
        let (placements, figures) = musicname_scene();
        game.load_scene(Screen::Waiting, placements, figures);
        game
    }

    #[test]
    fn waiting_musicname_resolves_the_catalog_title() {
        // Room-fill string flow end to end: the staged catalog names
        // the room's song, so `MUSICNAME` inks the title scrim plus
        // white ink — and the same scene without a catalog inks a
        // different frame (the bare id), never an invented name.
        let mut titled = musicname_game(true);
        let mut bare = musicname_game(false);
        let hit = paint_waiting_frame(&mut titled, 800, 600);
        let miss = paint_waiting_frame(&mut bare, 800, 600);
        // Both ink the caption row white; the title and the bare id
        // differ, so the frames differ (paint.rs pins the exact
        // strings and the rect scrim).
        assert!(has_ink_near(&hit, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&miss, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert_ne!(hit, miss);
    }

    #[test]
    fn waiting_without_catalog_paints_art_without_rows() {
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        game.set_selected_music(waiting_song_selection());
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // Dialog art lands at the origin; the unresolved panel leaves
        // the bottom strip on the flat background.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[48 * 80 + 10], background_of(Screen::Waiting));
        assert_eq!(buf[56 * 80 + 10], background_of(Screen::Waiting));
    }

    #[test]
    fn select_music_structure_layers_over_waiting_seats() {
        // The dialog is modal: with its art staged, the structure's
        // table band covers the seat grid while the art and the
        // chosen-song panel stay visible on top.
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(white_near(&buf, 80, 64, (10, 48)));
        let (x, y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(y + 1) as usize * 80 + (x - 6 + 1) as usize],
            select_scrim(),
            "the modal dialog structure covers the seats"
        );
    }

    #[test]
    fn set_song_catalog_stages_select_songs() {
        let mut game = ScreenGame::new();
        assert!(game.select_songs().is_empty());
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        assert_eq!(game.select_songs().len(), 1);
        assert_eq!(game.select_songs()[0].song_id, 757);
        assert_eq!(game.select_songs()[0].title, "First");
        assert_eq!(game.select_songs()[0].artist, "Band");
    }

    #[test]
    fn waiting_captions_carry_catalog_bpm_and_inventory_counts() {
        // Room song 757 stages the catalog BPM column (124.0); the
        // inventory stages alongside, zeroed until set.
        let mut game = ScreenGame::new();
        assert_eq!(game.skill_counts(), [0, 0, 0]);
        let mut room = waiting_room();
        room.music_id = 757;
        game.set_joined_room(room);
        assert_eq!(game.room_song_bpm(), None);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_skill_counts([7, 42, 1500]);
        assert_eq!(game.room_song_bpm(), Some(124.0));
        assert_eq!(game.catalog_song_bpm(757), Some(124.0));
        assert_eq!(game.catalog_song_bpm(999), None);
        assert_eq!(game.skill_counts(), [7, 42, 1500]);
    }

    #[test]
    fn waiting_staged_captions_ink_their_rects() {
        // Staged waiting covers plus three caption rects: the map
        // name (room word 1), the catalog BPM (124), and the first
        // inventory count ink their rects.
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.music_id = 757;
        room.arena = 1;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_skill_counts([7, 42, 1500]);
        game.load_scene(
            Screen::Waiting,
            vec![
                ScenePlacement {
                    id: "IDC_IMAGE_SWAIT_COVERAVATAR1".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 28,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                },
                ScenePlacement {
                    id: "IDC_TEXT_SWAIT_MAPNAME".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 480,
                        y: 15,
                        w: 90,
                        h: 23,
                    },
                },
                ScenePlacement {
                    id: "IDC_TEXT_SWAIT_BPM".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 620,
                        y: 60,
                        w: 60,
                        h: 20,
                    },
                },
                ScenePlacement {
                    id: "IDC_TEXT_SWAIT_1SKILLCOUNT".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 740,
                        y: 208,
                        w: 40,
                        h: 30,
                    },
                },
            ],
            HashMap::new(),
        );
        let buf = paint_waiting_frame(&mut game, 800, 600);
        let scrim = pack_xrgb(0x08, 0x08, 0x10);
        assert_eq!(buf[13 * 800 + 476], scrim, "mapname scrim");
        assert_eq!(buf[58 * 800 + 616], scrim, "bpm scrim");
        assert_eq!(buf[206 * 800 + 736], scrim, "skillcount scrim");
        assert!(
            has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)),
            "caption ink paints"
        );
    }

    /// Waiting game with dialog art and the one-song catalog staged:
    /// the structure-paint scenario.
    fn select_structure_game() -> ScreenGame {
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        stage_waiting_song(&mut game);
        game
    }

    #[test]
    fn select_structure_paints_regions_over_dialog_art() {
        let mut game = select_structure_game();
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        // Staged art stays visible behind the structure.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        // Filter column one: header scrim plus EASY button at its
        // reference pitch (129 + 33).
        assert_eq!(buf[95 * 800 + 30], select_scrim());
        assert_eq!(buf[162 * 800 + 30], select_scrim());
        // Row one: green E letter at the icon column (text inks
        // inside the band), white title ink at the TITLE column,
        // unknown dashes at LVL and GRADE.
        assert!(ink_near(
            &buf,
            800,
            600,
            (392, 150),
            (10, 16),
            pack_xrgb(0x30, 0xE0, 0x30)
        ));
        assert!(white_near(&buf, 800, 600, (417, 150)));
        assert!(ink_near(
            &buf,
            800,
            600,
            (628, 152),
            (12, 16),
            select_dim_ink()
        ));
        assert!(ink_near(
            &buf,
            800,
            600,
            (716, 152),
            (12, 16),
            select_dim_ink()
        ));
        // Search tab (amber caption), status banner, pager, and the
        // centered OK caption.
        assert!(ink_near(
            &buf,
            800,
            600,
            (355, 95),
            (40, 16),
            select_active_ink()
        ));
        assert!(white_near(&buf, 800, 600, (346, 398)));
        assert!(white_near(&buf, 800, 600, (744, 398)));
        assert!(white_near(&buf, 800, 600, (380, 523)));
        // Chosung letter A (row 1, cell 14) at its grid slot.
        assert!(white_near(&buf, 800, 600, (563, 444)));
    }

    #[test]
    fn select_structure_paints_without_catalog_rows() {
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        // The zero-row search tab still counts (amber ink); no row
        // band paints, so the row area keeps the waiting backdrop.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(ink_near(
            &buf,
            800,
            600,
            (355, 95),
            (40, 16),
            select_active_ink()
        ));
        assert_eq!(
            buf[160 * 800 + 400],
            super::super::paint::background_of(Screen::Waiting)
        );
    }

    #[test]
    fn select_structure_stays_hidden_without_dialog_art() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // No dialog art staged: the structure never paints (the
        // scaled filter-column anchor stays waiting background).
        assert_eq!(
            buf[10 * 80 + 2],
            super::super::paint::background_of(Screen::Waiting)
        );
    }

    #[test]
    fn splash_shows_loaded_art() {
        use crate::avatar::{AvatarFigure, FigureLayer};
        let mut logo = AvatarFigure::default();
        logo.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut game = ScreenGame::new();
        game.load_splash(logo);
        let mut buf = vec![0u32; 8 * 8];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[63], pack_xrgb(0x08, 0x08, 0x10));
    }

    #[test]
    fn lane_key_dismisses_splash_into_channel() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        let mut buf = vec![0u32; 16 * 12];
        game.paint(&mut Frame {
            buf: &mut buf,
            w: 16,
            h: 12,
        });
        assert_eq!(buf[0], background_of(Screen::Channel));
    }

    #[test]
    fn lane_release_keeps_splash() {
        let mut game = ScreenGame::new();
        game.lane_key(0, false);
        assert_eq!(game.screen(), Screen::Avi);
    }

    fn paint_splash_frame(game: &mut ScreenGame, now_ms: u64) {
        let mut buf = vec![0u32; 16 * 12];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 16,
                h: 12,
            },
            now_ms,
        );
    }

    #[test]
    fn splash_stays_before_timeout() {
        let mut game = ScreenGame::new();
        paint_splash_frame(&mut game, SPLASH_AUTO_ADVANCE_MS - 1);
        assert_eq!(game.screen(), Screen::Avi);
    }

    #[test]
    fn splash_auto_advances_at_timeout() {
        let mut game = ScreenGame::new();
        paint_splash_frame(&mut game, SPLASH_AUTO_ADVANCE_MS);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn splash_auto_advances_after_timeout() {
        let mut game = ScreenGame::new();
        paint_splash_frame(&mut game, SPLASH_AUTO_ADVANCE_MS + 1_000);
        assert_eq!(game.screen(), Screen::Channel);
    }

    fn walk_to_lobby(game: &mut ScreenGame, lobby: Screen) {
        game.lane_key(3, true);
        if lobby == Screen::Channel {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if lobby == Screen::Room {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn side_screens_open_from_every_lobby() {
        use super::super::side::ALL_SIDE_SCREENS;
        for lobby in [Screen::Channel, Screen::Room, Screen::Waiting] {
            for side in ALL_SIDE_SCREENS {
                let mut game = ScreenGame::new();
                walk_to_lobby(&mut game, lobby);
                assert!(game.visit_side(side), "{lobby:?} -> {side:?}");
                assert_eq!(game.side(), Some(side));
                assert_eq!(game.screen(), lobby);
            }
        }
    }

    #[test]
    fn side_screens_reject_main_flow_origins() {
        use super::super::side::ALL_SIDE_SCREENS;
        for side in ALL_SIDE_SCREENS {
            let mut splash = ScreenGame::new();
            assert!(!splash.visit_side(side));
            assert_eq!(splash.side(), None);
        }
        for side in ALL_SIDE_SCREENS {
            let mut game = ScreenGame::new();
            walk_to_playing(&mut game);
            assert!(!game.visit_side(side));
            assert_eq!(game.side(), None);
        }
    }

    #[test]
    fn side_visit_rejects_nested_sides() {
        use super::super::side::SideScreen;
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Room);
        assert!(game.visit_side(SideScreen::Shop));
        assert!(!game.visit_side(SideScreen::Planet));
        assert_eq!(game.side(), Some(SideScreen::Shop));
    }

    #[test]
    fn side_back_key_returns_to_lobby() {
        use super::super::side::SideScreen;
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Room);
        assert!(game.visit_side(SideScreen::MyRoom));
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn side_leaves_without_entry() {
        let mut game = ScreenGame::new();
        assert!(!game.leave_side());
    }

    #[test]
    fn side_paints_loaded_art() {
        use super::super::side::SideScreen;
        use crate::avatar::{AvatarFigure, FigureLayer, ScenePlacement};
        use o2jam_render::{Rect, Rgb555Image};
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Channel);
        game.load_side_scene(
            vec![ScenePlacement {
                id: "IDC_SHOP".to_owned(),
                code: 0,
                sprite: Some("Shop_Item.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            }],
            HashMap::from([("Shop_Item.ojs".to_owned(), figure)]),
        );
        assert!(game.visit_side(SideScreen::Shop));
        let mut buf = vec![0u32; 8 * 8];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(game.screen(), Screen::Channel);
    }

    /// One red pixel figure for staged scene art.
    fn red_scene_figure() -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Full-canvas placement pinning one sprite at the frame origin.
    fn corner_scene_art(name: &str) -> (Vec<ScenePlacement>, HashMap<String, AvatarFigure>) {
        use crate::screens::paint::{unbound_scene_placements, SceneArt};
        let _ = SceneArt::default();
        let names = vec![name.to_owned()];
        let figures = HashMap::from([(name.to_owned(), red_scene_figure())]);
        (unbound_scene_placements(&names), figures)
    }

    #[test]
    fn scene_art_backs_channel_list() {
        let (placements, figures) = corner_scene_art("Dialog_ChannelSelectBar.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Channel, placements, figures);
        game.set_channels(two_channels());
        game.lane_key(3, true);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn empty_channel_art_clears_stale_splash_pixels() {
        // Live Channel stages no art (`stage_empty_channel_scene`), so
        // entering from the splash must not leave logo pixels behind:
        // the list paints over device black, matching the pinned
        // art-less shot (`docs/screenshots/channel.png`).
        const STALE_SPLASH: u32 = pack_xrgb(0xEB, 0xE3, 0xE4);
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
        game.set_channels(two_channels());
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        let mut buf = vec![STALE_SPLASH; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert!(
            !buf.contains(&STALE_SPLASH),
            "live channel keeps splash pixels"
        );
        assert_eq!(buf[0], 0, "live channel clears to device black");
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn staged_room_art_paints_live_room_rows() {
        // Live Room shows staged panels but no row ink: narrow whether
        // the painter drops rows when art is staged (data present).
        let (placements, figures) = corner_scene_art("Room_ButtonRoom.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Room, placements, figures);
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert!(
            has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)),
            "room rows missing under staged art"
        );
    }

    #[test]
    fn listed_rooms_keep_titled_zero_numbered_first_room() {
        // The server numbers rooms from the first free slot, so the
        // first room is number 0; free slots share the number but
        // carry an empty title.
        let titled_first = RoomListEntry {
            number: 0,
            state_code: 1,
            title: "smoke".to_owned(),
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
        };
        let free_slot = RoomListEntry {
            number: 0,
            state_code: 0,
            title: String::new(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 0,
            game_mode: 0,
            speed: 1,
            capacity: 0,
            user_count: 0,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        };
        assert!(ScreenGame::is_listed_room(&titled_first));
        assert!(!ScreenGame::is_listed_room(&free_slot));
        assert!(ScreenGame::is_listed_room(&two_rooms()[0]));
    }

    #[test]
    fn room_number_at_names_clicked_rows() {
        // Band fallback needs no staged art: native bands resolve the
        // row whose number the session bridge joins before advancing.
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        assert_eq!(game.room_number_at(400, 154), Some(3));
        assert_eq!(game.room_number_at(400, 162), Some(7));
        assert_eq!(game.room_number_at(400, 100), None);
    }

    #[test]
    fn waiting_confirm_hit_needs_staged_buttons() {
        // Art-less clicks land nowhere (no pinned fallback geometry).
        let mut game = ScreenGame::new();
        assert!(!game.waiting_confirm_hit(150, 130));
        let placements = vec![crate::avatar::ScenePlacement {
            id: "IDC_BUTTON_SWAIT_READY".to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 100,
                y: 100,
                w: 200,
                h: 60,
            },
        }];
        game.load_scene(Screen::Waiting, placements, HashMap::new());
        assert!(game.waiting_confirm_hit(150, 130));
        assert!(!game.waiting_confirm_hit(10, 10));
    }

    #[test]
    fn channel_rows_ink_at_measured_native_tops() {
        // Reference-measured anchors (`/tmp/chan2-list.png`): row text
        // tops at 232+50·index on a native frame, scaled like staged
        // placements elsewhere.
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(white_near(&buf, 800, 600, (200, 232)));
        assert!(ink_near(
            &buf,
            800,
            600,
            (502, 236),
            (24, 12),
            pack_xrgb(150, 148, 151)
        ));
        assert!(ink_near(
            &buf,
            800,
            600,
            (200, 282),
            (64, 9),
            pack_xrgb(0x70, 0x70, 0x70)
        ));
    }

    #[test]
    fn channel_row_hit_maps_measured_bands() {
        assert_eq!(channel_row_hit(240, 10), Some(0));
        assert_eq!(channel_row_hit(281, 10), Some(0));
        assert_eq!(channel_row_hit(282, 10), Some(1));
        assert_eq!(channel_row_hit(100, 10), None);
        assert_eq!(channel_row_hit(732, 10), None);
        assert_eq!(channel_row_hit(240, 0), None);
    }

    #[test]
    fn bound_room_panels_paint_numbered_rows() {
        // Live Room stages bound panels yet shows no row ink: narrow
        // whether the panel path drops rows when data is present.
        // Rows sit inside the panel at the reference-measured inset,
        // not at the panel corner.
        let placements = vec![crate::avatar::ScenePlacement {
            id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 26,
                y: 82,
                w: 288,
                h: 91,
            },
        }];
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Room, placements, HashMap::new());
        game.set_rooms(vec![RoomListEntry {
            number: 0,
            state_code: 1,
            title: "smoke".to_owned(),
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
        }]);
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(white_near(&buf, 800, 600, (60, 83)));
    }

    #[test]
    fn channel_viewport_follows_highlight() {
        assert_eq!(channel_viewport(0, 10), (0, 4));
        assert_eq!(channel_viewport(5, 10), (2, 4));
        assert_eq!(channel_viewport(9, 10), (6, 4));
        assert_eq!(channel_viewport(0, 2), (0, 2));
        assert_eq!(channel_viewport(0, 0), (0, 0));
    }

    #[test]
    fn channel_pager_names_visible_slice() {
        // Display-only readout: no clicks bind (P0).
        let mut game = ScreenGame::new();
        game.set_channels(six_channels());
        game.lane_key(3, true);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(white_near(&buf, 800, 600, (400, 446)));
    }

    #[test]
    fn channel_click_maps_through_viewport() {
        // Highlight on row 5 shows rows 2-5; the top visible band
        // selects row 2.
        let mut game = ScreenGame::new();
        game.set_channels(six_channels());
        game.lane_key(3, true);
        for _ in 0..5 {
            game.menu_key(MenuKey::Down, true);
        }
        assert_eq!(game.highlight, 5);
        game.mouse_click(400, 240);
        assert_eq!(game.highlight, 2);
        assert_eq!(game.screen(), Screen::Room);
    }

    fn six_channels() -> Vec<ChannelState> {
        (0u16..6)
            .map(|id| ChannelState {
                gateway_id: 0,
                channel_id: id,
                capacity: 100,
                population: 5,
                active: true,
            })
            .collect()
    }

    #[test]
    fn channel_pager_clears_stale_text_on_scroll() {
        // The pager redraws in place on a persistent backbuffer: stale
        // glyphs outside the new text must clear.
        let mut game = ScreenGame::new();
        game.set_channels(six_channels());
        game.lane_key(3, true);
        for _ in 0..5 {
            game.menu_key(MenuKey::Down, true);
        }
        let mut buf = vec![pack_xrgb(0xFF, 0xFF, 0xFF); 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert_eq!(buf[446 * 800], 0);
        assert!(white_near(&buf, 800, 600, (400, 446)));
    }

    #[test]
    fn scene_art_backs_waiting_seats() {
        let (placements, figures) = corner_scene_art("Wait_TeamA.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Waiting, placements, figures);
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 80 * 64];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn scene_art_overlays_playing_without_hiding_notes() {
        let (placements, figures) = corner_scene_art("Playing_Panel.ojs");
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 1500, 7)]);
        game.load_scene(Screen::Playing, placements, figures);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            1000,
        );
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
    }

    #[test]
    fn scene_art_backs_result_without_small_frame_digits() {
        let (placements, figures) = corner_scene_art("Result_WinLose.ojs");
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        game.load_scene(Screen::Result, placements, figures);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            7000,
        );
        assert_eq!(game.screen(), Screen::Result);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        // Small frame: the pinned number fields stay off (the solo
        // banner body paints its corner layer instead).
        assert_ne!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn scene_art_backs_loading_bar() {
        use MenuKey;
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
    }

    fn battle_teams_self_wins() -> o2jam_game::RoomTeams {
        let mut teams = o2jam_game::RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.submit(0, 100_000);
        teams.submit(1, 150_000);
        teams
    }

    fn finish_song(game: &mut ScreenGame) {
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(game);
        game.strike_at(2, 5000);
    }

    fn paint_result_frame(game: &mut ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 70 * 70];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 70,
            },
            7000,
        );
        buf
    }

    #[test]
    fn result_without_battle_claim_paints_no_verdict() {
        let mut game = ScreenGame::new();
        finish_song(&mut game);
        assert_eq!(game.result_verdict(), ResultVerdict::Solo);
        let buf = paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        // The retired caption inks never come back; the banner is a
        // sprite frame of the staged art (none here).
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_verdict_is_battle_only_in_team_play() {
        // No room: solo banner. A joined Live room carries the
        // rank-1 verdict once teams and the claim resolve.
        let mut game = ScreenGame::new();
        assert_eq!(game.result_verdict(), ResultVerdict::Solo);
        let mut live = waiting_room();
        live.game_mode = o2jam_net::GameMode::Live as u8;
        game.set_joined_room(live);
        assert_eq!(game.result_verdict(), ResultVerdict::Battle(None));
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        assert_eq!(
            game.result_verdict(),
            ResultVerdict::Battle(Some(BattleVerdict::Win))
        );
        game.set_battle_self_team(Some(0));
        assert_eq!(
            game.result_verdict(),
            ResultVerdict::Battle(Some(BattleVerdict::Lose))
        );
    }

    #[test]
    fn result_rows_carry_local_score_solo_without_rank_or_speed() {
        // Solo offline: one local row only. The rank hides (2+
        // players only) and speed waits for a room; the numbers
        // ride the live score, the bonus the live hold
        // accumulator (zero with no holds — live, not invented).
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, Some("YOU".to_owned()));
        assert_eq!(rows[0].rank, None);
        assert_eq!(rows[0].speed, None);
        assert_eq!(rows[0].judgements, Some([1, 0, 0, 0]));
        assert_eq!(rows[0].combos, Some((1, 0)));
        assert_eq!(rows[0].score, Some(200));
        assert_eq!(rows[0].bonus, Some(0));
    }

    #[test]
    fn result_rows_name_peers_and_share_the_room_speed() {
        // Joined room with the host seated: self plus the wire
        // member, both naming the shared room speed code.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, Some("YOU".to_owned()));
        assert_eq!(rows[1].name, Some("host".to_owned()));
        assert_eq!(rows[0].speed, Some(1));
        assert_eq!(rows[1].speed, Some(1));
    }

    #[test]
    fn result_rows_leave_peer_numbers_unpainted() {
        // Per-member 4014/4016 tallies stay server-side: only the
        // local row carries numbers; the peer row paints its name
        // (and speed/rank when live) with every number unpainted.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let rows = game.result_rank_rows();
        assert!(rows[0].judgements.is_some());
        assert!(rows[0].combos.is_some());
        assert!(rows[0].score.is_some());
        assert!(rows[0].bonus.is_some());
        assert_eq!(rows[1].judgements, None);
        assert_eq!(rows[1].combos, None);
        assert_eq!(rows[1].score, None);
        assert_eq!(rows[1].bonus, None);
    }

    #[test]
    fn result_rows_hide_ranks_until_a_broadcast_lands() {
        // The song path never seeds stored ranks, so the mirror
        // holds zeros and both rows stay rank-less (the zero-byte
        // convention) — never a fabricated standing.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let rows = game.result_rank_rows();
        assert_eq!(rows[0].rank, None);
        assert_eq!(rows[1].rank, None);
    }

    #[test]
    fn result_rows_show_scripted_ranks_after_a_staged_broadcast() {
        // Scripted walk fixture: the local seat holds rank 1, the
        // wire peer rank 2 — the broadcast delivers each byte to its
        // holder through the rank-remap rule, and only those rows
        // paint (trailing zeros stay hidden).
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let deliveries = game.stage_scripted_ranks(
            0xFF,
            [1, 2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            [1, 2, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(deliveries.len(), 2);
        let rows = game.result_rank_rows();
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[1].rank, Some(2));
    }

    #[test]
    fn result_rows_skip_vacant_seats_and_cap_at_eight() {
        // The fixture seats one member beside two vacant slots, so
        // only two rows build; seven occupied wire slots cap the
        // table at eight (self plus seven).
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        assert_eq!(game.result_rank_rows().len(), 2);
        let mut full = waiting_room();
        full.slots = (0..7)
            .map(|index| RoomSlot {
                index,
                state_code: 1,
                member: Some(RoomMember {
                    nickname: format!("peer{index}"),
                    level: 1,
                    gender: 0,
                    gem: 0,
                    master: false,
                    team: 0,
                    ready: true,
                    music_state: 0,
                    equipment: [0; 17],
                    music_ids: Vec::new(),
                    cash_point: 0,
                    free_pass: 0,
                    playing_state: 0,
                    administrator: false,
                    super_manager: false,
                }),
            })
            .collect();
        game.set_joined_room(full);
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[7].name, Some("peer6".to_owned()));
    }

    #[test]
    fn grade_runs_the_pinned_table_on_the_chart() {
        // One-note chart fully Cool: the reference word holds a single
        // record, so `grade_max_reference` is zero and the plate stays
        // off — no chart can read as graded below 27 records
        // (PINNED `FUN_004c22c0` minus 26).
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        assert_eq!(game.grade(), None);
        assert_eq!(game.playing.score().points, 200);
    }

    #[test]
    fn grade_stays_below_the_lowest_threshold_on_a_miss() {
        // The timeout miss floors the points at zero: ratio 0 stays
        // under 0.30, the plate's empty state.
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            5_100,
        );
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        assert_eq!(game.playing.score().counts[3], 1);
        assert_eq!(game.grade(), None);
    }

    #[test]
    fn grade_needs_a_chart() {
        // No chart, no ratio: the plate waits (frame None).
        let game = ScreenGame::new();
        assert_eq!(game.grade(), None);
    }

    #[test]
    fn result_win_paints_no_invented_verdict_for_rank_one() {
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        assert_eq!(game.battle_verdict(), Some(BattleVerdict::Win));
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        // The retired text verdict never comes back; the banner is a
        // sprite frame of the staged art (none in this flat frame).
        assert_ne!(buf[48 * 70 + 16], pack_xrgb(0xFF, 0xD0, 0x20));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn result_lose_paints_no_invented_verdict_off_rank_one() {
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(0));
        assert_eq!(game.battle_verdict(), Some(BattleVerdict::Lose));
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        assert_ne!(buf[48 * 70 + 16], pack_xrgb(0x90, 0x90, 0x90));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_scene_art_paints_without_the_invented_verdict() {
        use crate::screens::paint::unbound_scene_placements;
        let names = vec![
            "Winner_image.ojs".to_owned(),
            "Result_WinLose.ojs".to_owned(),
        ];
        let figures = HashMap::from([
            (names[0].clone(), red_scene_figure()),
            (names[1].clone(), red_scene_figure()),
        ]);
        let placements = unbound_scene_placements(&names);
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        game.load_scene(Screen::Result, placements, figures);
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        // The staged winner/winlose art paints; the retired text
        // caption ink does not.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    fn paint_playing_frame(game: &mut ScreenGame, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            1000,
        );
        buf
    }

    /// True while white text ink lands near a caption origin (glyph
    /// top-left cells are often blank, so scan the scrim box).
    /// True while glyph ink lights in a 64x9 box at `origin`: the
    /// TTF rasterizes inside the cell with antialiased coverage, so
    /// probes band the ink color instead of one exact pixel.
    fn white_near(buf: &[u32], w: usize, h: usize, origin: (i32, i32)) -> bool {
        ink_near(buf, w, h, origin, (64, 9), pack_xrgb(0xFF, 0xFF, 0xFF))
    }

    /// True while ink nears `ink` inside the `probe` box at `origin`.
    fn ink_near(
        buf: &[u32],
        w: usize,
        h: usize,
        origin: (i32, i32),
        probe: (i32, i32),
        ink: u32,
    ) -> bool {
        let (ox, oy) = origin;
        let (pw, ph) = probe;
        for row in oy..oy + ph {
            for col in ox..ox + pw {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < h && c < w && near_xrgb(buf[r * w + c], ink) {
                    return true;
                }
            }
        }
        false
    }

    fn playing_avatar_game() -> ScreenGame {
        use o2jam_file::{parse_sprite55, SPRITE55_MAGIC};
        // 2x1 sprite-55: transparent black, then red (result-test shape).
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 2u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x7C]);
        let sprite = parse_sprite55(&blob).unwrap();
        let mut game = ScreenGame::new();
        game.load_avatar(AvatarFigure::from_sprite55(&sprite));
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.set_player(43, "HOST");
        game.set_song_caption("First", 6);
        game.set_volume(5);
        game.strike_at(2, 5000);
        game
    }

    #[test]
    fn playing_paints_avatar_scene_with_captions_and_tally() {
        use crate::playing::AvatarLayout;
        let mut game = playing_avatar_game();
        let buf = paint_playing_frame(&mut game, 800, 600);
        assert_eq!(game.screen(), Screen::Playing);
        let layout = AvatarLayout::for_frame(800, 600);
        // Worn figure extent-centers in the seat: the 2x1 sprite's
        // red pixel (layer x 1) lands right of the centered box origin.
        assert_eq!(
            buf[310 * 800 + 457],
            pack_xrgb(255, 0, 0),
            "figure extent-centers in the seat"
        );
        // Life gauge shows full amber; plate, speed, song, VOL, counts,
        // EXIT ink white.
        assert_eq!(buf[215 * 800 + 400], pack_xrgb(255, 180, 16));
        for origin in [
            layout.level_origin,
            layout.speed_origin,
            layout.title_origin,
            layout.volume_origin,
            layout.counts_origins[0],
            layout.exit_origin,
        ] {
            assert!(white_near(&buf, 800, 600, origin));
        }
        // The struck note's COOL feeds the tally through the score.
        assert_eq!(game.playing.score().counts[0], 1);
    }

    #[test]
    fn playing_without_figure_paints_scene_without_avatar() {
        use crate::playing::AvatarLayout;
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.set_player(43, "HOST");
        let buf = paint_playing_frame(&mut game, 800, 600);
        let layout = AvatarLayout::for_frame(800, 600);
        // No figure: the seat center keeps the highway backdrop while
        // the plate still paints.
        assert_eq!(
            buf[310 * 800 + 456],
            pack_xrgb(0x0A, 0x0A, 0x0A),
            "empty seat shows the backdrop"
        );
        let (lx, ly) = layout.level_origin;
        assert!(white_near(&buf, 800, 600, (lx, ly)));
    }

    #[test]
    fn playing_syncs_song_line_from_waiting_choice() {
        use crate::playing::AvatarLayout;
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_selected_music(waiting_song_selection());
        walk_to_playing(&mut game);
        let buf = paint_playing_frame(&mut game, 800, 600);
        // `M757` catalog title with speed X2 (code 3): the strip lines ink
        // at the bound text rects.
        let layout = AvatarLayout::for_frame(800, 600);
        assert!(white_near(&buf, 800, 600, layout.speed_origin));
        assert!(white_near(&buf, 800, 600, layout.title_origin));
    }

    /// Paint Playing past the (5000 ms) note's bad window, rating the
    /// timeout Miss on the explicit clock.
    fn paint_playing_after_timeout_miss(game: &mut ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            5000 + 25 + 1,
        );
        buf
    }

    #[test]
    fn selected_chart_difficulty_drains_the_avatar_life_bar() {
        use crate::playing::AvatarLayout;
        // Chart load before selection: the choice still drives deltas.
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        game.set_selected_music(waiting_song_selection()); // difficulty 2 = Hard
        walk_to_playing(&mut game);
        let buf = paint_playing_after_timeout_miss(&mut game);
        // Hard Miss drains 30 (Easy would drain 50); the bar paints
        // the live 970/1000 amber fill (top row empty, rows below lit).
        assert_eq!(game.playing.score().life.life(), 1000 - 30);
        let bar = AvatarLayout::for_frame(800, 600).life_bar;
        assert_eq!(
            buf[bar.y as usize * 800 + bar.x as usize],
            pack_xrgb(0x30, 0x10, 0x10)
        );
        assert_eq!(
            buf[(bar.y + 1) as usize * 800 + bar.x as usize],
            pack_xrgb(255, 180, 16)
        );
    }

    #[test]
    fn selection_before_chart_load_still_sets_life_difficulty() {
        let mut game = ScreenGame::new();
        game.set_selected_music(waiting_song_selection()); // difficulty 2 = Hard
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let _ = paint_playing_after_timeout_miss(&mut game);
        assert_eq!(game.playing.score().life.life(), 1000 - 30);
    }

    #[test]
    fn unselected_charts_drain_on_the_easy_default() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let _ = paint_playing_after_timeout_miss(&mut game);
        // No selection: the bar keeps the exe's init set (difficulty
        // byte defaults to Easy, file `0x9515c`).
        assert_eq!(game.playing.score().life.life(), 1000 - 50);
    }

    #[test]
    fn arena_stage_defaults_to_a_real_stage_before_any_room() {
        // Exe room entry replaces a zero stage with a real one (store
        // `0x4aacea`), so the composer never carries stage 0.
        let game = ScreenGame::new();
        assert_eq!(game.arena_stage(), DEFAULT_ARENA_STAGE);
        assert_ne!(game.arena_stage(), 0);
    }

    #[test]
    fn joined_room_arena_sets_the_backdrop_stage() {
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.arena = 7;
        game.set_joined_room(room);
        assert_eq!(game.arena_stage(), 7);
        let mut next = waiting_room();
        next.arena = 36;
        game.set_joined_room(next);
        assert_eq!(game.arena_stage(), 36);
    }

    #[test]
    fn zero_arena_keeps_the_current_stage() {
        // Room entry replaces zero with a real stage, it never stores
        // one, so a zero word names no stage and the default stands.
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.arena = 0;
        game.set_joined_room(room);
        assert_eq!(game.arena_stage(), DEFAULT_ARENA_STAGE);
    }

    #[test]
    fn arena_high_word_seeds_the_low_stage_word() {
        // Applier seeding (`0x4e78db`): the low word when a high word
        // is set, the full value otherwise; negative names no stage.
        assert_eq!(arena_stage_word((1 << 16) | 5), Some(5));
        assert_eq!(arena_stage_word(5), Some(5));
        assert_eq!(arena_stage_word(0), None);
        assert_eq!(arena_stage_word(-1), None);
    }

    fn loading_status_at_half() -> LoadingStatus {
        LoadingStatus {
            progress: 0.5,
            title: "First".to_owned(),
            artist: "Band".to_owned(),
        }
    }

    fn walk_to_loading(game: &mut ScreenGame) {
        use MenuKey;
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn loading_status_starts_empty() {
        assert_eq!(ScreenGame::new().loading_status(), None);
    }

    #[test]
    fn set_loading_status_remembers_the_walk() {
        let mut game = ScreenGame::new();
        game.set_loading_status(loading_status_at_half());
        assert_eq!(game.loading_status(), Some(&loading_status_at_half()));
    }

    #[test]
    fn clear_loading_restores_the_indeterminate_bar() {
        let mut game = ScreenGame::new();
        game.set_loading_status(loading_status_at_half());
        game.clear_loading();
        assert_eq!(game.loading_status(), None);
    }

    #[test]
    fn loading_paints_the_staged_walk_fraction() {
        // 64x48: track x16 w32 y16; half fill spans x16..32.
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        game.set_loading_status(loading_status_at_half());
        walk_to_loading(&mut game);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[16 * 64 + 16], pack_xrgb(0x30, 0xB0, 0x30));
        assert_ne!(buf[16 * 64 + 40], pack_xrgb(0x30, 0xB0, 0x30));
    }

    #[test]
    fn loading_paints_the_staged_song_caption() {
        // Song row sits one placeholder row below the bar (y36).
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        game.set_loading_status(loading_status_at_half());
        walk_to_loading(&mut game);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert!(white_near(&buf, 64, 48, (16, 36)));
    }

    #[test]
    fn loading_without_a_walk_keeps_the_full_bar() {
        // No status: the legacy indeterminate full bar (x16..48).
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        walk_to_loading(&mut game);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(game.loading_status(), None);
        assert_eq!(buf[16 * 64 + 47], pack_xrgb(0x30, 0xB0, 0x30));
    }

    #[test]
    fn loading_cover_miss_keeps_the_planet_fallback() {
        // Garbage bytes stage nothing: no cover flag, and the staged
        // planet body survives for the fallback paint.
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        assert!(!game.stage_loading_cover(b"garbage"));
        assert!(!game.stage_loading_cover(&[]));
        assert!(!game.has_loading_cover());
        let art = game.scene_art(Screen::Loading).expect("loading art staged");
        assert!(art.figures.contains_key("MusicLoading.ojs"));
        assert!(!art.figures.contains_key(LOADING_COVER_SPRITE));
    }

    #[test]
    fn loading_cover_stages_the_walked_chart() {
        // End to end on a real install: song 757's walked bytes decode
        // to the 800x600 illustration, staged beside the planet body.
        // Skips when `install-root` is absent.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        assert!(game.stage_loading_cover(&bytes));
        assert!(game.has_loading_cover());
        let art = game.scene_art(Screen::Loading).expect("loading art staged");
        assert!(
            art.figures.contains_key("MusicLoading.ojs"),
            "planet fallback survives beside the cover"
        );
        let cover = art
            .figures
            .get(LOADING_COVER_SPRITE)
            .expect("cover staged under its sprite name");
        assert_eq!(cover.len(), 1);
        let layer = cover.layer(0).expect("one opaque layer");
        assert_eq!((layer.image.w, layer.image.h), (800, 600));
    }

    #[test]
    fn loading_cover_miss_clears_a_staged_cover() {
        // A later miss drops the earlier cover (stale art must never
        // hide the planet fallback). Skips when `install-root` is absent.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        assert!(game.stage_loading_cover(&bytes));
        assert!(!game.stage_loading_cover(b"garbage"));
        assert!(!game.has_loading_cover());
        let art = game.scene_art(Screen::Loading).expect("loading art staged");
        assert!(art.figures.contains_key("MusicLoading.ojs"));
        assert!(!art.figures.contains_key(LOADING_COVER_SPRITE));
    }

    #[test]
    fn selecting_the_walked_song_stages_its_loading_cover() {
        // Catalog + selection resolve reads the same walked bytes the
        // gate walks, so real loads feed the loading scene with no
        // extra call. Skips when `install-root` is absent.
        let chart = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(_) = std::fs::read(&chart) else {
            return;
        };
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        stage_waiting_song(&mut game);
        assert!(game.has_loading_cover());
    }

    fn chat_user(sender: &str, text: &str) -> o2jam_net::rooms::WaitingUserMessage {
        o2jam_net::rooms::WaitingUserMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
            member_id: 1,
        }
    }

    fn chat_admin(sender: &str, text: &str) -> o2jam_net::rooms::WaitingAdminMessage {
        o2jam_net::rooms::WaitingAdminMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
            member_id: 7,
        }
    }

    fn channel_user(sender: &str, text: &str) -> o2jam_net::rooms::MainRoomUserMessage {
        o2jam_net::rooms::MainRoomUserMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
        }
    }

    fn channel_admin(sender: &str, text: &str) -> o2jam_net::rooms::MainRoomAdminMessage {
        o2jam_net::rooms::MainRoomAdminMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
        }
    }

    fn channel_whisper(sender: &str, text: &str) -> o2jam_net::rooms::ReceiveWhisper {
        o2jam_net::rooms::ReceiveWhisper {
            sender: sender.to_owned(),
            content: text.to_owned(),
        }
    }

    fn channel_announcement(text: &str) -> o2jam_net::rooms::Announcement {
        o2jam_net::rooms::Announcement {
            content: text.to_owned(),
        }
    }

    fn channel_rank(
        nickname: &str,
        music_id: i32,
        ranking: i32,
    ) -> o2jam_net::rooms::RankNotification {
        o2jam_net::rooms::RankNotification {
            invalid: false,
            music_id,
            nickname: nickname.to_owned(),
            ranking,
        }
    }

    #[test]
    fn waiting_chat_starts_empty() {
        let game = ScreenGame::new();
        assert!(game.waiting_chat().is_empty());
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.chat_send_message(), None);
    }

    #[test]
    fn pushing_a_user_message_stages_one_chat_row() {
        let mut game = ScreenGame::new();
        game.push_waiting_user_message(&chat_user("host", "hello"));
        assert_eq!(game.waiting_chat().len(), 1);
        assert_eq!(game.waiting_chat()[0].sender, "host");
        assert_eq!(game.waiting_chat()[0].text, "hello");
        assert!(!game.waiting_chat()[0].admin);
    }

    #[test]
    fn pushing_an_admin_message_marks_the_row() {
        let mut game = ScreenGame::new();
        game.push_waiting_admin_message(&chat_admin("op", "quiet"));
        assert_eq!(game.waiting_chat().len(), 1);
        assert!(game.waiting_chat()[0].admin);
    }

    #[test]
    fn joining_a_room_clears_chat_history_and_draft() {
        let mut game = ScreenGame::new();
        game.push_waiting_user_message(&chat_user("host", "hello"));
        game.set_chat_input("draft");
        game.set_joined_room(waiting_room());
        assert!(game.waiting_chat().is_empty());
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn channel_chat_starts_empty() {
        let game = ScreenGame::new();
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn pushing_a_channel_user_message_stages_one_row() {
        let mut game = ScreenGame::new();
        game.push_channel_user_message(&channel_user("host", "hello"));
        assert_eq!(game.channel_chat().len(), 1);
        assert_eq!(game.channel_chat()[0].sender, "host");
        assert_eq!(game.channel_chat()[0].text, "hello");
        assert!(!game.channel_chat()[0].admin);
        assert_eq!(game.channel_chat()[0].whisper_recipient, None);
    }

    #[test]
    fn pushing_a_channel_admin_message_marks_the_row() {
        let mut game = ScreenGame::new();
        game.push_channel_admin_message(&channel_admin("op", "quiet"));
        assert_eq!(game.channel_chat().len(), 1);
        assert!(game.channel_chat()[0].admin);
    }

    #[test]
    fn pushing_a_channel_whisper_keeps_the_recipient() {
        let mut game = ScreenGame::new();
        game.push_channel_whisper(&channel_whisper("host", "psst"), "me");
        assert_eq!(game.channel_chat().len(), 1);
        assert_eq!(game.channel_chat()[0].sender, "host");
        assert_eq!(game.channel_chat()[0].text, "psst");
        assert_eq!(
            game.channel_chat()[0].whisper_recipient.as_deref(),
            Some("me")
        );
    }

    #[test]
    fn pushing_a_channel_announcement_stages_a_bare_admin_row() {
        let mut game = ScreenGame::new();
        game.push_channel_announcement(&channel_announcement("hello"));
        assert_eq!(game.channel_chat().len(), 1);
        assert!(game.channel_chat()[0].admin);
        assert_eq!(channel_chat_row_text(&game.channel_chat()[0]), "hello");
    }

    #[test]
    fn empty_announcement_stages_no_row() {
        let mut game = ScreenGame::new();
        game.push_channel_announcement(&channel_announcement(""));
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn pushing_a_channel_rank_notification_paints_the_board_sentence() {
        let mut game = ScreenGame::new();
        game.push_channel_rank_notification(&channel_rank("host", 757, 1));
        assert_eq!(game.channel_chat().len(), 1);
        assert!(game.channel_chat()[0].admin);
        assert_eq!(
            channel_chat_row_text(&game.channel_chat()[0]),
            rank_notice_board_text("host", "M757", 1)
        );
    }

    #[test]
    fn channel_rank_notification_resolves_the_catalog_title() {
        let mut game = ScreenGame::new();
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.push_channel_rank_notification(&channel_rank("host", 757, 1));
        assert_eq!(
            channel_chat_row_text(&game.channel_chat()[0]),
            rank_notice_board_text("host", "First", 1)
        );
    }

    #[test]
    fn invalid_rank_notification_stages_no_row() {
        let mut game = ScreenGame::new();
        game.push_channel_rank_notification(&o2jam_net::rooms::RankNotification {
            invalid: true,
            music_id: 757,
            nickname: "host".to_owned(),
            ranking: 1,
        });
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn refreshing_channels_clears_notice_rows() {
        let mut game = ScreenGame::new();
        game.push_channel_announcement(&channel_announcement("hello"));
        game.push_channel_rank_notification(&channel_rank("host", 757, 1));
        game.set_channels(two_channels());
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn channel_notices_share_the_fifty_row_bound() {
        let mut game = ScreenGame::new();
        for index in 0..55 {
            game.push_channel_announcement(&channel_announcement(&format!("line{index}")));
        }
        assert_eq!(game.channel_chat().len(), 50);
        assert_eq!(game.channel_chat()[0].text, "line5");
    }

    #[test]
    fn refreshing_channels_clears_the_channel_history() {
        let mut game = ScreenGame::new();
        game.push_channel_user_message(&channel_user("host", "hello"));
        game.set_channels(two_channels());
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn channel_history_bounds_to_fifty_rows() {
        let mut game = ScreenGame::new();
        for index in 0..55 {
            game.push_channel_user_message(&channel_user("host", &format!("line{index}")));
        }
        assert_eq!(game.channel_chat().len(), 50);
        assert_eq!(game.channel_chat()[0].text, "line5");
    }

    #[test]
    fn chat_send_builds_a_3011_body_from_the_input_line() {
        let mut game = ScreenGame::new();
        game.set_chat_input("go");
        assert_eq!(game.chat_input(), "go");
        let send = game.chat_send_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        game.clear_chat_input();
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.chat_send_message(), None);
    }

    #[test]
    fn take_chat_message_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        assert_eq!(game.take_chat_message(), None);
        game.set_chat_input("go");
        let send = game.take_chat_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.take_chat_message(), None);
    }

    #[test]
    fn channel_send_builds_a_2012_body_from_the_input_line() {
        let mut game = ScreenGame::new();
        game.set_channel_input("go");
        assert_eq!(game.channel_input(), "go");
        let send = game.channel_send_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        game.clear_channel_input();
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.channel_send_message(), None);
    }

    #[test]
    fn take_channel_message_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        assert_eq!(game.take_channel_message(), None);
        game.set_channel_input("go");
        let send = game.take_channel_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.take_channel_message(), None);
    }

    #[test]
    fn channel_whisper_needs_both_draft_and_recipient() {
        let mut game = ScreenGame::new();
        assert_eq!(game.channel_whisper_message("host"), None);
        game.set_channel_input("psst");
        assert_eq!(game.channel_whisper_message(""), None);
        let whisper = game.channel_whisper_message("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
    }

    #[test]
    fn take_channel_whisper_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        game.set_channel_input("psst");
        let whisper = game.take_channel_whisper("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.take_channel_whisper("host"), None);
    }

    #[test]
    fn take_channel_whisper_remembers_the_peer_for_reply() {
        let mut game = ScreenGame::new();
        game.set_channel_input("psst");
        game.take_channel_whisper("host");
        assert_eq!(game.last_whisper_peer(), Some("host"));
    }

    #[test]
    fn channel_whisper_draft_parses_name_and_text() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w host psst");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "psst".to_owned(),
            }
        );
    }

    #[test]
    fn channel_whisper_draft_collapses_extra_gaps() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w  host   hi there ");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "hi there".to_owned(),
            }
        );
    }

    #[test]
    fn waiting_draft_parses_whisper_the_same_way() {
        let mut game = ScreenGame::new();
        game.set_chat_input("/w host psst");
        assert_eq!(
            game.chat_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "psst".to_owned(),
            }
        );
    }

    #[test]
    fn whisper_draft_without_text_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w host");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/w host".to_owned())
        );
        game.set_channel_input("/w");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/w".to_owned())
        );
    }

    #[test]
    fn reply_targets_the_last_whisper_sender() {
        let mut game = ScreenGame::new();
        game.push_channel_whisper(&channel_whisper("host", "psst"), "me");
        assert_eq!(game.last_whisper_peer(), Some("host"));
        game.set_channel_input("/r hello");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "hello".to_owned(),
            }
        );
    }

    #[test]
    fn reply_without_a_peer_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        assert_eq!(game.last_whisper_peer(), None);
        game.set_channel_input("/r hello");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/r hello".to_owned())
        );
    }

    #[test]
    fn reply_without_text_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        game.push_channel_whisper(&channel_whisper("host", "psst"), "me");
        game.set_chat_input("/r");
        assert_eq!(
            game.chat_slash_command(),
            SlashCommand::Chat("/r".to_owned())
        );
    }

    #[test]
    fn help_commands_parse_without_sending() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/h");
        assert_eq!(game.channel_slash_command(), SlashCommand::Help);
        game.set_channel_input("/help");
        assert_eq!(game.channel_slash_command(), SlashCommand::Help);
        game.set_chat_input("/help");
        assert_eq!(game.chat_slash_command(), SlashCommand::Help);
    }

    #[test]
    fn unknown_slash_command_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/dance party");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/dance party".to_owned())
        );
        game.set_chat_input("/dance party");
        assert_eq!(
            game.chat_slash_command(),
            SlashCommand::Chat("/dance party".to_owned())
        );
    }

    #[test]
    fn empty_draft_parses_to_empty() {
        let game = ScreenGame::new();
        assert_eq!(game.channel_slash_command(), SlashCommand::Empty);
        assert_eq!(game.chat_slash_command(), SlashCommand::Empty);
    }

    #[test]
    fn take_channel_slash_consumes_a_whisper_and_remembers_the_peer() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w host psst");
        assert_eq!(
            game.take_channel_slash(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "psst".to_owned(),
            }
        );
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.last_whisper_peer(), Some("host"));
        assert_eq!(game.take_channel_slash(), SlashCommand::Empty);
    }

    #[test]
    fn take_chat_slash_consumes_help_without_a_peer() {
        let mut game = ScreenGame::new();
        game.set_chat_input("/help");
        assert_eq!(game.take_chat_slash(), SlashCommand::Help);
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.last_whisper_peer(), None);
    }

    #[test]
    fn chat_whisper_needs_both_draft_and_recipient() {
        let mut game = ScreenGame::new();
        assert_eq!(game.chat_whisper_message("host"), None);
        game.set_chat_input("psst");
        assert_eq!(game.chat_whisper_message(""), None);
        let whisper = game.chat_whisper_message("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
    }

    #[test]
    fn take_chat_whisper_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        game.set_chat_input("psst");
        let whisper = game.take_chat_whisper("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.last_whisper_peer(), Some("host"));
        assert_eq!(game.take_chat_whisper("host"), None);
    }

    #[test]
    fn channel_confirm_with_a_whisper_draft_consumes_without_advancing() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        game.set_channel_input("/w host psst");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.last_whisper_peer(), Some("host"));
    }

    #[test]
    fn waiting_confirm_with_a_help_draft_consumes_without_starting() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.set_chat_input("/help");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn typing_appends_to_the_channel_draft_only_while_there() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        game.text_input("hi");
        assert_eq!(game.channel_input(), "hi");
        assert_eq!(game.chat_input(), "");
        game.take_channel_message();
        walk_to_waiting(&mut game);
        game.text_input("yo");
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.chat_input(), "yo");
    }

    #[test]
    fn channel_backspace_erases_one_character_only_while_there() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.text_input("héllo");
        game.delete_text();
        assert_eq!(game.channel_input(), "héll");
        game.clear_channel_input();
        walk_to_waiting(&mut game);
        game.delete_text();
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn channel_confirm_with_a_draft_sends_and_stays_in_channel() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.set_channel_input("go");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.channel_input(), "");
    }

    #[test]
    fn refreshing_channels_clears_the_channel_draft() {
        let mut game = ScreenGame::new();
        game.set_channel_input("draft");
        game.set_channels(two_channels());
        assert_eq!(game.channel_input(), "");
    }

    #[test]
    fn waiting_paints_live_chat_rows_without_staged_art() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        let calm = paint_waiting_frame(&mut game, 80, 64);
        game.push_waiting_user_message(&chat_user("host", "hello"));
        let live = paint_waiting_frame(&mut game, 80, 64);
        assert_ne!(calm, live);
        assert!(live.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn waiting_without_packets_paints_no_chat_history() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        assert!(game.waiting_chat().is_empty());
    }

    #[test]
    fn typing_appends_to_the_draft_only_while_waiting() {
        let mut game = ScreenGame::new();
        game.text_input("hi");
        assert_eq!(game.chat_input(), "");
        walk_to_waiting(&mut game);
        game.text_input("hi");
        assert_eq!(game.chat_input(), "hi");
    }

    #[test]
    fn typing_ignores_control_characters() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.text_input("a\u{0}b\n");
        assert_eq!(game.chat_input(), "ab");
    }

    #[test]
    fn typing_stops_at_fifty_characters() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.text_input(&"x".repeat(60));
        assert_eq!(game.chat_input().chars().count(), 50);
        game.text_input("y");
        assert_eq!(game.chat_input().chars().count(), 50);
    }

    #[test]
    fn staging_an_overlong_line_truncates_to_fifty_characters() {
        let mut game = ScreenGame::new();
        game.set_chat_input(&"x".repeat(60));
        assert_eq!(game.chat_input().chars().count(), 50);
    }

    #[test]
    fn backspace_erases_one_character_only_while_waiting() {
        let mut game = ScreenGame::new();
        game.delete_text();
        assert_eq!(game.chat_input(), "");
        walk_to_waiting(&mut game);
        game.delete_text();
        assert_eq!(game.chat_input(), "");
        game.text_input("héllo");
        game.delete_text();
        assert_eq!(game.chat_input(), "héll");
    }

    #[test]
    fn confirm_with_a_draft_sends_and_stays_in_waiting() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.text_input("go");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn confirm_with_an_empty_draft_advances_to_loading() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    /// Seat of a non-master self: the creator keeps seat 0, so any
    /// other seat id is a guest (`+0x30` derives from id 0,
    /// `docs/relay.md` §10).
    fn guest_room() -> JoinedRoom {
        let mut room = waiting_room();
        room.member_id = 5;
        room.slots[0].index = 1;
        room
    }

    #[test]
    fn waiting_non_master_hides_select_but_keeps_bpm() {
        // Only the master opens music selection (file `0xb2622`
        // drops non-master `MUSELECT` clicks): a guest self sees no
        // dialog art, no structure, and no chosen-song panel — but
        // the staged `BPM` caption stays (guests pick speed from it).
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        let mut room = guest_room();
        room.music_id = 757;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        stage_waiting_song(&mut game);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.load_scene(
            Screen::Waiting,
            vec![
                ScenePlacement {
                    id: "IDC_IMAGE_SWAIT_COVERAVATAR1".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 28,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                },
                ScenePlacement {
                    id: "IDC_TEXT_SWAIT_BPM".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 620,
                        y: 60,
                        w: 60,
                        h: 20,
                    },
                },
            ],
            HashMap::new(),
        );
        assert!(!game.is_room_master());
        let buf = paint_waiting_frame(&mut game, 800, 600);
        // No dialog art at the origin, no structure scrims, no
        // chosen-song panel rows below the seats (staged art without
        // figures leaves the frame untouched, so absence reads 0) —
        // but the BPM scrim inks its rect.
        assert_eq!(buf[0], 0);
        assert_eq!(buf[95 * 800 + 30], 0);
        assert_eq!(buf[450 * 800 + 100], 0);
        assert_eq!(buf[458 * 800 + 100], 0);
        assert_eq!(buf[58 * 800 + 616], pack_xrgb(0x08, 0x08, 0x10));
    }

    #[test]
    fn waiting_non_master_confirm_stays_in_waiting() {
        // Guests never start (no `START` arm for them, file
        // `0xae805`): their Confirm readies up through the session
        // bridge instead, so the shell stays in Waiting.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        walk_to_waiting(&mut game);
        assert!(!game.is_room_master());
        assert!(game.all_peers_ready());
        assert!(!game.can_start_game());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn guest_force_advance_reaches_loading_on_broadcast() {
        // Guests never pass the Confirm gate (no `START` arm for
        // them, file `0xae805`), so the 4011 broadcast advances them
        // through the force path instead.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        walk_to_waiting(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_refuses_master_who_keeps_the_confirm_gate() {
        // Masters keep the readiness rule through Confirm: the force
        // path never advances them, even with every peer ready.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        assert!(game.can_start_game());
        assert!(!game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Waiting);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_refuses_the_offline_walk() {
        // With no joined room the offline walk advances through
        // Confirm, never through the force path.
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        assert!(!game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Waiting);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_holds_while_a_draft_is_pending() {
        // A pending draft holds the advance (peek-only): the text
        // survives, and clearing it unblocks the next broadcast.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        walk_to_waiting(&mut game);
        game.set_chat_input("hi");
        assert!(!game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Waiting);
        assert_eq!(game.chat_input(), "hi");
        game.clear_chat_input();
        assert!(game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_refuses_other_screens() {
        // Off Waiting there is nothing to advance: the call stays a
        // no-op instead of teleporting the flow.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        assert!(!game.force_advance_to_loading());
        assert_ne!(game.screen(), Screen::Loading);
    }

    #[test]
    fn waiting_master_start_needs_every_peer_ready() {
        // `START 0x16202012` runs the all-ready check (file
        // `0xb5560`): an unready occupied peer blocks the master,
        // and the start fires once every peer is ready.
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.slots[0].index = 1;
        room.slots[0]
            .member
            .as_mut()
            .expect("fixture seats a peer")
            .ready = false;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        assert!(game.is_room_master());
        assert!(!game.all_peers_ready());
        assert!(!game.can_start_game());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.waiting.as_mut().expect("room joined").slots[0]
            .member
            .as_mut()
            .expect("fixture seats a peer")
            .ready = true;
        assert!(game.can_start_game());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }
}
