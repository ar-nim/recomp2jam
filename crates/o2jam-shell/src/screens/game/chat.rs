//! Chat drafts, lobby panels, and the slash grammar behind one seam
//! (#139). The game seam keeps flow, dialog focus, and song-caption
//! lookup; this module owns chat words and their pure decisions.

pub use super::super::paint::ChatSendMode;
use super::super::paint::{ChannelChatMessage, WaitingChatMessage};
use super::{MAX_CHAT_INPUT_CHARS, MAX_MAIN_ROOM_CHAT_ROWS, MAX_WAITING_CHAT_ROWS};
use crate::screen::Screen;
use o2jam_net::message::{KeyMode, RoomUserListEntry};
use o2jam_net::rooms::{
    Announcement, MainRoomAdminMessage, MainRoomUserMessage, RankNotification, ReceiveWhisper,
    RoomListEntry, RoomState, SendMainRoomMessage, SendWaitingMessage, SendWhisper,
    SendWhisperResponse, WaitingAdminMessage, WaitingUserMessage,
};

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

/// The two waiting messenger lamps (`0x1650006B/6C`) share one alert
/// pulse: `FUN_004c0690` starts an 800 ms animation when its argument
/// is non-zero and the lamp is idle, and resets both lamps to their
/// rest frame when it is zero (files `0xc06a0`-`0xc071a`). The pulse
/// source is a messenger notification; the shell has no backend for it,
/// so the on-edge is a named seam and only the off-edge is wired to the
/// staged MESSENGER press (dispatcher file `0xb81e0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct MessengerLamp {
    alert_until_ms: Option<u64>,
}

impl MessengerLamp {
    /// Start (or restart) the 800 ms alert pulse.
    fn pulse(&mut self, now_ms: u64) {
        self.alert_until_ms = Some(now_ms.saturating_add(800));
    }

    /// Reset to the rest frame.
    fn clear(&mut self) {
        self.alert_until_ms = None;
    }

    /// True while the alert animation stands at `now_ms`.
    #[must_use]
    fn lit_at(self, now_ms: u64) -> bool {
        self.alert_until_ms.is_some_and(|end| now_ms < end)
    }
}

/// One lobby chat panel that carries the main-room feed: the unsent
/// draft line plus the live history, oldest first. The Channel scene
/// and the Room lobby each own one (the room's chat IS the main-room
/// chat, ticket #37: `SendMainRoomMessage` 2012 out, 2013/2014 in), so
/// the row folding, the draft cap, and the history bound live here
/// once.
#[derive(Debug, Clone, Default)]
pub struct LobbyChatPanel {
    draft: String,
    history: Vec<ChannelChatMessage>,
}

impl LobbyChatPanel {
    /// Unsent draft line (the 2012 content source).
    #[must_use]
    pub fn draft(&self) -> &str {
        &self.draft
    }

    /// Live history rows, oldest first (empty until a packet arrives).
    #[must_use]
    pub fn history(&self) -> &[ChannelChatMessage] {
        &self.history
    }

    /// Type one character into the draft: control characters never
    /// enter the line and the draft stops at [`MAX_CHAT_INPUT_CHARS`]
    /// characters.
    fn type_char(&mut self, typed: char) {
        if typed.is_control() || self.draft.chars().count() >= MAX_CHAT_INPUT_CHARS {
            return;
        }
        self.draft.push(typed);
    }

    /// Erase the last draft character (Backspace; one whole character,
    /// never half of a multibyte one).
    fn erase_char(&mut self) {
        self.draft.pop();
    }

    /// Stage the draft line, truncated to [`MAX_CHAT_INPUT_CHARS`].
    fn set_draft(&mut self, line: &str) {
        self.draft = line.chars().take(MAX_CHAT_INPUT_CHARS).collect();
    }

    /// Forget the unsent draft.
    fn clear_draft(&mut self) {
        self.draft.clear();
    }

    /// Append one row and bound the history to
    /// [`MAX_MAIN_ROOM_CHAT_ROWS`] (oldest rows drop first).
    fn push_row(&mut self, row: ChannelChatMessage) {
        self.history.push(row);
        let overflow = self.history.len().saturating_sub(MAX_MAIN_ROOM_CHAT_ROWS);
        self.history.drain(..overflow);
    }

    /// Outgoing 2012 body for the draft (`None` while the draft is
    /// empty, so no empty send is invented).
    #[must_use]
    fn send_message(&self) -> Option<SendMainRoomMessage> {
        if self.draft.is_empty() {
            return None;
        }
        Some(SendMainRoomMessage {
            content: self.draft.as_bytes().to_vec(),
        })
    }

    /// Outgoing 2017 whisper body for the draft (`None` while the draft
    /// or the recipient is empty, so no empty send is invented).
    #[must_use]
    fn whisper_message(&self, recipient: &str) -> Option<SendWhisper> {
        if self.draft.is_empty() || recipient.is_empty() {
            return None;
        }
        Some(SendWhisper {
            recipient: recipient.to_owned(),
            content: self.draft.clone(),
        })
    }
}

/// Which lobby chat panel a call addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyChat {
    /// Channel scene panel.
    Channel,
    /// Room lobby panel (staged `IDC_LIST_SROOM_CHAT`).
    Room,
}

impl LobbyChat {
    /// Panel the on-screen scene owns (`None`: the scene shows no lobby
    /// chat panel).
    #[must_use]
    pub fn of_screen(screen: Screen) -> Option<Self> {
        match screen {
            Screen::Channel => Some(Self::Channel),
            Screen::Room => Some(Self::Room),
            _ => None,
        }
    }
}

/// One 2018 delivery report the shell can show without guessing a
/// caption: the server's invalid word, recipient, and echoed text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperDelivery {
    /// Server's `invalid` word (`true` = nobody heard the whisper).
    pub delivered: bool,
    /// Recipient the 2017 request named.
    pub recipient: String,
    /// Echoed text, absent for an undelivered whisper.
    pub content: Option<String>,
}

/// Gate reasons the Room row press refuses, named by the exe's pinned
/// message bytes (`docs/pins/room-waiting.md`, `0x493a50`). The
/// already-started arm has no pinned caption here, so it stays absent
/// rather than inventing text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomJoinRefusal {
    /// Locked row (caption at file `0x248178`).
    Password,
    /// Level window excludes the player (file `0x248190`).
    Level { min: i32, max: i32 },
    /// No free seat (file `0x248250`).
    Full,
    /// Game-info still loading (file `0x248208`).
    Loading,
    /// Premium-only room without PT (file `0x2481e8`).
    Premium,
    /// 7Key room below the level in its caption (file `0x248280`).
    SevenKeys,
    /// Song the catalog does not name (file `0x248330`).
    UnregisteredSong,
    /// Started room: this arm has no pinned caption in
    /// `docs/pins/room-waiting.md`, so it stages no popup text.
    Started,
}

/// The held room-list message bytes, read from the pinned 7.55 image
/// (sha256 `83a04541…72c823b`; REA `ev_92af29…`, file offsets
/// `0x248178`..`0x248330`). Own-words meanings are on the variants.
impl RoomJoinRefusal {
    fn template(self) -> &'static [u8] {
        match self {
            Self::Password => PASSWORD_REQUIRED,
            Self::Level { .. } => LEVEL_WINDOW,
            Self::Full => ROOM_FULL,
            Self::Loading => GAME_INFO_LOADING,
            Self::Premium => NEED_PT,
            Self::SevenKeys => SEVEN_KEY_LEVEL,
            Self::UnregisteredSong => UNREGISTERED_SONG,
            Self::Started => &[],
        }
    }

    /// CP949 bytes for the one-OK sheet; an unpinned arm stays empty.
    #[must_use]
    pub fn message(self) -> Vec<u8> {
        let template = self.template();
        let values: Vec<Vec<u8>> = match self {
            Self::Level { min, max } => {
                vec![min.to_string().into_bytes(), max.to_string().into_bytes()]
            }
            _ => Vec::new(),
        };
        let mut message = Vec::with_capacity(template.len());
        let mut index = 0;
        let mut value = 0;
        while index < template.len() {
            if template[index] == b'%'
                && index + 1 < template.len()
                && matches!(template[index + 1], b'd' | b's')
            {
                if let Some(part) = values.get(value) {
                    message.extend_from_slice(part);
                    value += 1;
                } else {
                    // The full-room caption's `%s` argument is not
                    // pinned; keep the literal rather than invent one.
                    message.extend_from_slice(&template[index..index + 2]);
                }
                index += 2;
                continue;
            }
            message.push(template[index]);
            index += 1;
        }
        message
    }
}

/// Which pinned gate refuses one wire row, if any. The level window
/// uses the same player source as QUICKJOIN; the 7Key level floor is
/// the number in its caption. PT is a row flag here because the shell
/// has no local PT word, and an unknown catalog song is unregistered.
#[must_use]
pub(super) fn room_join_refusal_for(
    room: &RoomListEntry,
    player_level: Option<i32>,
    song_known: bool,
) -> Option<RoomJoinRefusal> {
    if room.state_code == RoomState::Playing as u8 {
        return Some(RoomJoinRefusal::Started);
    }
    if room.state_code == RoomState::Unavailable as u8 {
        return Some(RoomJoinRefusal::Loading);
    }
    if room.has_password {
        return Some(RoomJoinRefusal::Password);
    }
    if room.user_count >= room.capacity {
        return Some(RoomJoinRefusal::Full);
    }
    if room.min_level != 0 || room.max_level != 0 {
        // A boot without the 2001 identity has no local level to
        // compare; the exe's byte source is then unset, so the
        // window cannot refuse.
        let inside = player_level.is_none_or(|level| {
            i32::from(room.min_level) <= level && level <= i32::from(room.max_level)
        });
        if !inside {
            return Some(RoomJoinRefusal::Level {
                min: i32::from(room.min_level),
                max: i32::from(room.max_level),
            });
        }
    }
    if room.premium {
        return Some(RoomJoinRefusal::Premium);
    }
    const SEVEN_KEY_MIN_LEVEL: i32 = 4;
    if room.key_mode == KeyMode::SevenKeys as u8
        && player_level.is_some_and(|level| level < SEVEN_KEY_MIN_LEVEL)
    {
        return Some(RoomJoinRefusal::SevenKeys);
    }
    if room.music_id != 0 && !song_known {
        return Some(RoomJoinRefusal::UnregisteredSong);
    }
    None
}

const PASSWORD_REQUIRED: &[u8] = &[
    0xC6, 0xD0, 0xBD, 0xBA, 0xBF, 0xF6, 0xB5, 0xE5, 0xB8, 0xA6, 0x20, 0xC0, 0xD4, 0xB7, 0xC2, 0xC7,
    0xCF, 0xBC, 0xBC, 0xBF, 0xE4,
];
const LEVEL_WINDOW: &[u8] = &[
    0x5B, 0xC1, 0xA4, 0xBA, 0xB8, 0x5D, 0xB7, 0xB9, 0xBA, 0xA7, 0xC0, 0xCC, 0x20, 0xB8, 0xC2, 0xC1,
    0xF6, 0x20, 0xBE, 0xCA, 0xBD, 0xC0, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E, 0x0A, 0x0A, 0xC0, 0xD4, 0xC0,
    0xE5, 0xC7, 0xCF, 0xBD, 0xC7, 0x20, 0xBC, 0xF6, 0x20, 0xC0, 0xD6, 0xB4, 0xC2, 0x20, 0xB7, 0xB9,
    0xBA, 0xA7, 0xC0, 0xBA, 0x20, 0x0A, 0x0A, 0x25, 0x64, 0xB7, 0xB9, 0xBA, 0xA7, 0xBF, 0xA1, 0xBC,
    0xAD, 0x20, 0x25, 0x64, 0xB7, 0xB9, 0xBA, 0xA7, 0xB1, 0xEE, 0xC1, 0xF6, 0x20, 0xC0, 0xD4, 0xB4,
    0xCF, 0xB4, 0xD9, 0x2E,
];
const ROOM_FULL: &[u8] = &[
    0xB9, 0xE6, 0xC0, 0xCE, 0xBF, 0xF8, 0xC0, 0xCC, 0x20, 0xB8, 0xF0, 0xB5, 0xCE, 0x20, 0xC3, 0xA1,
    0xBD, 0xC0, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
    0x20, 0x20, 0x0A, 0x0A, 0x28, 0x25, 0x73, 0x29,
];
const GAME_INFO_LOADING: &[u8] = &[
    0xB0, 0xD4, 0xC0, 0xD3, 0xC1, 0xA4, 0xBA, 0xB8, 0x20, 0xB7, 0xCE, 0xB5, 0xF9, 0xC1, 0xDF, 0x20,
    0xBB, 0xF3, 0xC5, 0xC2, 0xBF, 0xA9, 0xBC, 0xAD, 0x0A, 0xC0, 0xD4, 0xC0, 0xE5, 0xC7, 0xD2, 0x20,
    0xBC, 0xF6, 0x20, 0xBE, 0xF8, 0xBD, 0xC0, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E, 0x0A, 0xC0, 0xE1, 0xBD,
    0xC3, 0xC8, 0xC4, 0x20, 0xB4, 0xD9, 0xBD, 0xC3, 0x20, 0xBD, 0xC3, 0xB5, 0xB5, 0xC7, 0xD8, 0xC1,
    0xD6, 0xBC, 0xBC, 0xBF, 0xE4, 0x2E,
];
const NEED_PT: &[u8] = &[
    0x50, 0x54, 0x28, 0xC6, 0xF7, 0xC0, 0xCE, 0xC6, 0xAE, 0x29, 0xB0, 0xA1, 0x20, 0xBA, 0xCE, 0xC1,
    0xB7, 0xC7, 0xD5, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E,
];
const SEVEN_KEY_LEVEL: &[u8] = &[
    0xC0, 0xE1, 0xB1, 0xF1, 0x21, 0x20, 0xC0, 0xCC, 0x20, 0xB9, 0xE6, 0xC0, 0xBA, 0x20, 0x37, 0x4B,
    0x65, 0x79, 0xB8, 0xF0, 0xB5, 0xE5, 0x20, 0xC7, 0xC3, 0xB7, 0xB9, 0xC0, 0xCC, 0x20, 0xC1, 0xDF,
    0xC0, 0xD4, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E, 0x0A, 0x0A, 0xBF, 0xC0, 0xC5, 0xF5, 0xC0, 0xEB, 0xC0,
    0xC7, 0x20, 0xC2, 0xFC, 0xC0, 0xE7, 0xB9, 0xCC, 0x21, 0x20, 0x37, 0x4B, 0x65, 0x79, 0xB8, 0xF0,
    0xB5, 0xE5, 0xB8, 0xA6, 0x20, 0xC1, 0xF1, 0xB1, 0xE2, 0xB7, 0xC1, 0xB8, 0xE9, 0x0A, 0xB7, 0xB9,
    0xBA, 0xA7, 0xC0, 0x34, 0xB8, 0xA6, 0x20, 0xB4, 0xDE, 0xBC, 0xBA, 0xC7, 0xCF, 0xBC, 0xC5, 0xBE,
    0xDF, 0x20, 0xC7, 0xD5, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E, 0x0A, 0x0A, 0xBD, 0xC5, 0xB3, 0xAA, 0xB4,
    0xC2, 0x20, 0xB5, 0xE5, 0xB7, 0xB3, 0xB8, 0xF0, 0xB5, 0xE5, 0x28, 0x33, 0x4B, 0x65, 0x79, 0x29,
    0x20, 0xC7, 0xC3, 0xB7, 0xB9, 0xC0, 0xCC, 0xB8, 0xA6, 0x20, 0xC5, 0xEB, 0xC7, 0xD8, 0x0A, 0xB7,
    0xB9, 0xBA, 0xA7, 0xC0, 0x34, 0xBF, 0xCD, 0x20, 0x37, 0x4B, 0x65, 0x79, 0xB8, 0xF0, 0xB5, 0xE5,
    0xBF, 0xA1, 0x20, 0xB5, 0xB5, 0xC0, 0xFC, 0xC7, 0xD8, 0x20, 0xBA, 0xB8, 0xBC, 0xBC, 0xBF, 0xE4,
    0x21,
];
const UNREGISTERED_SONG: &[u8] = &[
    0xB5, 0xEE, 0xB7, 0xCF, 0xB5, 0xC7, 0xC1, 0xF6, 0x20, 0xBE, 0xCA, 0xC0, 0xBA, 0x20, 0xB0, 0xEE,
    0xC0, 0xBB, 0x0A, 0xBB, 0xE7, 0xBF, 0xEB, 0xC7, 0xCF, 0xB0, 0xED, 0x20, 0xC0, 0xD6, 0xB4, 0xC2,
    0x20, 0xB9, 0xE6, 0xC0, 0xD4, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E,
];

/// Every chat word the shell owns: two lobby panels, the waiting
/// room's draft and history, and the last whisper peer. The game seam
/// gates typing by screen and dialog focus and resolves rank-notice
/// song titles; this state owns the words, row bounds, send bodies,
/// and slash decisions (#139 chat cluster).
#[derive(Default)]
pub(super) struct ChatState {
    channel_panel: LobbyChatPanel,
    room_panel: LobbyChatPanel,
    room_join_refusal: Option<RoomJoinRefusal>,
    whisper_delivery: Option<WhisperDelivery>,
    room_user_sheet_open: bool,
    /// Named rows of a decoded 2011 sheet. Web content stays outside
    /// this client; these are the wire names the local stand-in shows.
    room_user_sheet: Vec<RoomUserListEntry>,
    room_user_sheet_gift_on: bool,
    room_user_sheet_url_gift_on: bool,
    waiting_draft: String,
    waiting_history: Vec<WaitingChatMessage>,
    waiting_send_mode: ChatSendMode,
    // `Some` is the local nickname prompt posted by the WHISPER tab;
    // its string is the typed target before the prompt's OK.
    whisper_prompt_target: Option<String>,
    whisper_recipient: Option<String>,
    messenger_lamp: MessengerLamp,
    last_whisper_peer: Option<String>,
}

impl ChatState {
    /// Immutable panel behind [`LobbyChat`].
    pub(super) fn panel(&self, chat: LobbyChat) -> &LobbyChatPanel {
        match chat {
            LobbyChat::Channel => &self.channel_panel,
            LobbyChat::Room => &self.room_panel,
        }
    }

    fn panel_mut(&mut self, chat: LobbyChat) -> &mut LobbyChatPanel {
        match chat {
            LobbyChat::Channel => &mut self.channel_panel,
            LobbyChat::Room => &mut self.room_panel,
        }
    }

    /// Live waiting-room rows, oldest first.
    pub(super) fn waiting_history(&self) -> &[WaitingChatMessage] {
        &self.waiting_history
    }

    /// Unsent waiting-room draft (the 3011 source).
    pub(super) fn waiting_draft(&self) -> &str {
        &self.waiting_draft
    }

    /// Last whisper peer behind `/r` (`None` until a whisper flows).
    pub(super) fn last_whisper_peer(&self) -> Option<&str> {
        self.last_whisper_peer.as_deref()
    }

    /// A fresh channel list resets its chat draft beside the rows.
    pub(super) fn clear_channel_panel(&mut self) {
        *self.panel_mut(LobbyChat::Channel) = LobbyChatPanel::default();
    }

    /// A new room gets its own 3012/3013 history and draft.
    pub(super) fn clear_waiting(&mut self) {
        self.waiting_history.clear();
        self.waiting_draft.clear();
        // The exe's Waiting setup resets `+0x298` to room mode and
        // resets the lamps (files `0xba929`-`0xba94b`, `0xc06a0`).
        self.waiting_send_mode = ChatSendMode::Room;
        self.whisper_prompt_target = None;
        self.whisper_recipient = None;
        self.messenger_lamp.clear();
    }

    /// Outgoing chat mode selected by the staged tab group (exe `+0x298`).
    #[must_use]
    pub(super) const fn waiting_send_mode(&self) -> ChatSendMode {
        self.waiting_send_mode
    }

    /// Recipient staged by the local whisper prompt's OK.
    pub(super) fn whisper_recipient(&self) -> Option<&str> {
        self.whisper_recipient.as_deref()
    }

    /// True while the WHISPER tab's local nickname prompt owns typing.
    #[must_use]
    pub(super) const fn whisper_prompt_open(&self) -> bool {
        self.whisper_prompt_target.is_some()
    }

    /// Text typed into the open nickname prompt.
    pub(super) fn whisper_prompt_target(&self) -> &str {
        self.whisper_prompt_target.as_deref().unwrap_or("")
    }

    /// Commit one staged tab press. The four tabs choose the next send
    /// mode only: the received history is untouched. WHISPER additionally
    /// clears the draft and posts the local nickname prompt
    /// (`FUN_004b7c60`, file `0xb7df6`-`0xb7e24`). `false` for other codes.
    pub(super) fn select_waiting_send_mode(&mut self, code: u32) -> bool {
        let Some(mode) = ChatSendMode::from_code(code) else {
            return false;
        };
        self.waiting_send_mode = mode;
        if mode == ChatSendMode::Whisper {
            self.waiting_draft.clear();
            self.whisper_prompt_target = Some(String::new());
            self.whisper_recipient = None;
        } else {
            self.whisper_prompt_target = None;
        }
        true
    }

    /// Type one character into the open local nickname prompt.
    pub(super) fn type_whisper_prompt_char(&mut self, typed: char) {
        let Some(target) = self.whisper_prompt_target.as_mut() else {
            return;
        };
        if !typed.is_control() && target.chars().count() < MAX_CHAT_INPUT_CHARS {
            target.push(typed);
        }
    }

    /// Erase one character from the open local nickname prompt.
    pub(super) fn erase_whisper_prompt_char(&mut self) {
        if let Some(target) = self.whisper_prompt_target.as_mut() {
            target.pop();
        }
    }

    /// Accept the local nickname prompt: stage its target for the next
    /// whisper send and remember it for `/r`. `false` when no prompt is open.
    pub(super) fn confirm_whisper_prompt(&mut self) -> bool {
        let Some(target) = self.whisper_prompt_target.take() else {
            return false;
        };
        if !target.is_empty() {
            self.whisper_recipient = Some(target.clone());
            self.remember_whisper_peer(&target);
        }
        true
    }

    /// True while either messenger lamp shows its alert animation.
    #[must_use]
    pub(super) fn messenger_lamp_lit_at(&self, now_ms: u64) -> bool {
        self.messenger_lamp.lit_at(now_ms)
    }

    /// Start the 800 ms messenger lamp alert (named seam: no shell
    /// messenger notification backend feeds it yet).
    pub(super) fn pulse_messenger_lamp(&mut self, now_ms: u64) {
        self.messenger_lamp.pulse(now_ms);
    }

    /// Reset the messenger lamps (`FUN_004c0690(0)` on MESSENGER).
    pub(super) fn clear_messenger_lamp(&mut self) {
        self.messenger_lamp.clear();
    }

    /// Append one waiting 3012 row.
    pub(super) fn push_waiting_user(&mut self, message: &WaitingUserMessage) {
        self.push_waiting_row(&message.sender, &message.content_text(), false);
    }

    /// Append one waiting 3013 admin row.
    pub(super) fn push_waiting_admin(&mut self, message: &WaitingAdminMessage) {
        self.push_waiting_row(&message.sender, &message.content_text(), true);
    }

    fn push_waiting_row(&mut self, sender: &str, text: &str, admin: bool) {
        self.waiting_history.push(WaitingChatMessage {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin,
        });
        let overflow = self
            .waiting_history
            .len()
            .saturating_sub(MAX_WAITING_CHAT_ROWS);
        self.waiting_history.drain(..overflow);
    }

    /// Append one 2013 lobby row.
    pub(super) fn push_lobby_user(&mut self, chat: LobbyChat, message: &MainRoomUserMessage) {
        let row = ChannelChatMessage::user(&message.sender, &message.content_text());
        self.panel_mut(chat).push_row(row);
    }

    /// Append one 2014 lobby admin row.
    pub(super) fn push_lobby_admin(&mut self, chat: LobbyChat, message: &MainRoomAdminMessage) {
        let row = ChannelChatMessage::admin(&message.sender, &message.content_text());
        self.panel_mut(chat).push_row(row);
    }

    /// Append one 2019 whisper row; the sender becomes the `/r` peer.
    pub(super) fn push_lobby_whisper(
        &mut self,
        chat: LobbyChat,
        message: &ReceiveWhisper,
        recipient: &str,
    ) {
        let row = ChannelChatMessage::whisper(&message.sender, recipient, &message.content);
        self.panel_mut(chat).push_row(row);
        self.remember_whisper_peer(&message.sender);
    }

    /// Append one 2016 announcement row, unless the notice is empty.
    pub(super) fn push_lobby_announcement(&mut self, chat: LobbyChat, message: &Announcement) {
        if message.content.is_empty() {
            return;
        }
        let row = ChannelChatMessage::announcement(&message.content);
        self.panel_mut(chat).push_row(row);
    }

    /// Append one 2029 rank row, unless the packet carries no rank.
    /// `song_title` is the game seam's catalog lookup for the wire id.
    pub(super) fn push_lobby_rank(
        &mut self,
        chat: LobbyChat,
        message: &RankNotification,
        song_title: &str,
    ) {
        if message.invalid {
            return;
        }
        let row = ChannelChatMessage::rank(&message.nickname, song_title, message.ranking);
        self.panel_mut(chat).push_row(row);
    }

    /// Stage the waiting draft, truncated to the character cap.
    pub(super) fn set_waiting_draft(&mut self, line: &str) {
        self.waiting_draft = line.chars().take(MAX_CHAT_INPUT_CHARS).collect();
    }

    /// Type one waiting character (controls and overlong drafts drop).
    pub(super) fn type_waiting_char(&mut self, typed: char) {
        if typed.is_control() || self.waiting_draft.chars().count() >= MAX_CHAT_INPUT_CHARS {
            return;
        }
        self.waiting_draft.push(typed);
    }

    /// Erase one waiting draft character.
    pub(super) fn erase_waiting_char(&mut self) {
        self.waiting_draft.pop();
    }

    /// Forget the waiting draft.
    pub(super) fn clear_waiting_draft(&mut self) {
        self.waiting_draft.clear();
    }

    /// Outgoing 3011 body for the waiting draft.
    pub(super) fn waiting_send_message(&self) -> Option<SendWaitingMessage> {
        if self.waiting_draft.is_empty() {
            return None;
        }
        Some(SendWaitingMessage {
            content: self.waiting_draft.as_bytes().to_vec(),
        })
    }

    /// Take the 3011 body and clear the draft so one Enter sends once.
    pub(super) fn take_waiting_message(&mut self) -> Option<SendWaitingMessage> {
        let message = self.waiting_send_message()?;
        self.clear_waiting_draft();
        Some(message)
    }

    /// Outgoing 2017 whisper body for the waiting draft.
    pub(super) fn waiting_whisper_message(&self, recipient: &str) -> Option<SendWhisper> {
        if self.waiting_draft.is_empty() || recipient.is_empty() {
            return None;
        }
        Some(SendWhisper {
            recipient: recipient.to_owned(),
            content: self.waiting_draft.clone(),
        })
    }

    /// Take the 2017 body, clear the draft, and remember the peer.
    pub(super) fn take_waiting_whisper(&mut self, recipient: &str) -> Option<SendWhisper> {
        let message = self.waiting_whisper_message(recipient)?;
        self.clear_waiting_draft();
        self.remember_whisper_peer(recipient);
        Some(message)
    }

    /// Stage one lobby draft, truncated to the character cap.
    pub(super) fn set_lobby_draft(&mut self, chat: LobbyChat, line: &str) {
        self.panel_mut(chat).set_draft(line);
    }

    /// Type one character into one lobby draft.
    pub(super) fn type_lobby_char(&mut self, chat: LobbyChat, typed: char) {
        self.panel_mut(chat).type_char(typed);
    }

    /// Erase one character from one lobby draft.
    pub(super) fn erase_lobby_char(&mut self, chat: LobbyChat) {
        self.panel_mut(chat).erase_char();
    }

    /// Outgoing 2012 body for one lobby draft.
    pub(super) fn lobby_send_message(&self, chat: LobbyChat) -> Option<SendMainRoomMessage> {
        self.panel(chat).send_message()
    }

    /// Outgoing 2017 body for one lobby draft.
    pub(super) fn lobby_whisper_message(
        &self,
        chat: LobbyChat,
        recipient: &str,
    ) -> Option<SendWhisper> {
        self.panel(chat).whisper_message(recipient)
    }

    /// Forget one lobby draft.
    pub(super) fn clear_lobby_draft(&mut self, chat: LobbyChat) {
        self.panel_mut(chat).clear_draft();
    }

    /// Take the 2012 body and clear one lobby draft.
    pub(super) fn take_lobby_message(&mut self, chat: LobbyChat) -> Option<SendMainRoomMessage> {
        let message = self.lobby_send_message(chat)?;
        self.clear_lobby_draft(chat);
        Some(message)
    }

    /// Take the 2017 body, clear one lobby draft, and remember the peer.
    pub(super) fn take_lobby_whisper(
        &mut self,
        chat: LobbyChat,
        recipient: &str,
    ) -> Option<SendWhisper> {
        let message = self.lobby_whisper_message(chat, recipient)?;
        self.clear_lobby_draft(chat);
        self.remember_whisper_peer(recipient);
        Some(message)
    }

    /// Remember one whisper peer for `/r`: the newest name wins; an
    /// empty name never becomes the peer.
    pub(super) fn remember_whisper_peer(&mut self, peer: &str) {
        if !peer.is_empty() {
            self.last_whisper_peer = Some(peer.to_owned());
        }
    }

    /// Peek one lobby draft through the slash grammar.
    pub(super) fn lobby_slash_command(&self, chat: LobbyChat) -> SlashCommand {
        parse_slash_draft(self.panel(chat).draft(), self.last_whisper_peer())
    }

    /// Peek the waiting draft through the same slash grammar.
    pub(super) fn waiting_slash_command(&self) -> SlashCommand {
        parse_slash_draft(self.waiting_draft(), self.last_whisper_peer())
    }

    /// Take one lobby draft through the slash grammar, clearing a
    /// non-empty command and remembering a whisper recipient.
    pub(super) fn take_lobby_slash(&mut self, chat: LobbyChat) -> SlashCommand {
        let command = self.lobby_slash_command(chat);
        if command != SlashCommand::Empty {
            self.clear_lobby_draft(chat);
        }
        if let SlashCommand::Whisper { recipient, .. } = &command {
            self.remember_whisper_peer(recipient);
        }
        command
    }

    /// Take the waiting draft through the slash grammar.
    pub(super) fn take_waiting_slash(&mut self) -> SlashCommand {
        let command = self.waiting_slash_command();
        if command != SlashCommand::Empty {
            self.clear_waiting_draft();
        }
        if let SlashCommand::Whisper { recipient, .. } = &command {
            self.remember_whisper_peer(recipient);
        }
        command
    }

    /// Refusal the current row press staged (painted by the game seam).
    pub(super) fn room_join_refusal(&self) -> Option<RoomJoinRefusal> {
        self.room_join_refusal
    }

    /// Stage the gate verdict; a later row press replaces it.
    pub(super) fn stage_room_join_refusal(&mut self, refusal: RoomJoinRefusal) {
        self.room_join_refusal = Some(refusal);
    }

    /// Forget the verdict when the one-OK sheet is dismissed.
    pub(super) fn clear_room_join_refusal(&mut self) {
        self.room_join_refusal = None;
    }

    /// Latest 2018 delivery verdict.
    pub(super) fn whisper_delivery(&self) -> Option<&WhisperDelivery> {
        self.whisper_delivery.as_ref()
    }

    /// Stage one decoded 2018 answer in both lobby panels: an echoed
    /// text paints one row, while a refusal only changes the verdict.
    pub(super) fn push_whisper_response(&mut self, message: &SendWhisperResponse) {
        self.whisper_delivery = Some(WhisperDelivery {
            delivered: !message.invalid,
            recipient: message.recipient.clone(),
            content: message.content.clone(),
        });
        if let Some(content) = message.content.as_deref().filter(|text| !text.is_empty()) {
            let row = ChannelChatMessage::user(&message.recipient, content);
            self.panel_mut(LobbyChat::Channel).push_row(row.clone());
            self.panel_mut(LobbyChat::Room).push_row(row);
        }
    }

    /// Stage the named rows of one 2011 answer. A later answer
    /// replaces the sheet; an empty answer clears it (#970).
    pub(super) fn set_room_user_sheet(&mut self, users: &[RoomUserListEntry]) {
        self.room_user_sheet = users.to_vec();
    }

    /// Named rows currently staged for the Room user sheet.
    pub(super) fn room_user_sheet(&self) -> &[RoomUserListEntry] {
        &self.room_user_sheet
    }

    /// Stage one 2018 refusal: the exe posts the named failure with
    /// zero ink and no echoed text (`0x55c190`, REA `ev 4abe.96c9…`).
    pub(super) fn push_whisper_refusal(&mut self, chat: LobbyChat, name: &str) {
        let row = ChannelChatMessage::refusal(name);
        self.panel_mut(chat).push_row(row);
    }

    /// Mount the local stand-in for the exe's web user-list sheet.
    pub(super) fn open_room_user_sheet(&mut self, gift_on: bool) {
        self.room_user_sheet_open = true;
        self.room_user_sheet_gift_on = gift_on;
    }

    /// Close the sheet and rebuild its URL word from the gift-on flag
    /// (exe `0x495a30`; the web view itself is outside this client).
    pub(super) fn close_room_user_sheet(&mut self) {
        self.room_user_sheet_open = false;
        self.room_user_sheet_url_gift_on = self.room_user_sheet_gift_on;
    }

    pub(super) fn room_user_sheet_open(&self) -> bool {
        self.room_user_sheet_open
    }

    pub(super) fn room_user_sheet_gift_on(&self) -> bool {
        self.room_user_sheet_gift_on
    }

    pub(super) fn room_user_sheet_url_gift_on(&self) -> bool {
        self.room_user_sheet_url_gift_on
    }
}
