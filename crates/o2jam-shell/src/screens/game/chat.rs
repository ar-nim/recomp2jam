//! Chat drafts, lobby panels, and the slash grammar behind one seam
//! (#139). The game seam keeps flow, dialog focus, and song-caption
//! lookup; this module owns chat words and their pure decisions.

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
pub(super) fn parse_slash_draft(draft: &str, reply_to: Option<&str>) -> SlashCommand {
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
