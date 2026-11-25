//! Chat drafts, lobby panels, and the slash grammar behind one seam
//! (#139). The game seam keeps flow, dialog focus, and song-caption
//! lookup; this module owns chat words and their pure decisions.

pub use super::super::paint::ChatSendMode;
use super::super::paint::{ChannelChatMessage, WaitingChatMessage};
use super::{MAX_CHAT_INPUT_CHARS, MAX_MAIN_ROOM_CHAT_ROWS, MAX_WAITING_CHAT_ROWS};
use crate::screen::Screen;
use o2jam_net::rooms::{
    Announcement, MainRoomAdminMessage, MainRoomUserMessage, RankNotification, ReceiveWhisper,
    SendMainRoomMessage, SendWaitingMessage, SendWhisper, WaitingAdminMessage, WaitingUserMessage,
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

/// Every chat word the shell owns: two lobby panels, the waiting
/// room's draft and history, and the last whisper peer. The game seam
/// gates typing by screen and dialog focus and resolves rank-notice
/// song titles; this state owns the words, row bounds, send bodies,
/// and slash decisions (#139 chat cluster).
#[derive(Default)]
pub(super) struct ChatState {
    channel_panel: LobbyChatPanel,
    room_panel: LobbyChatPanel,
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
}
