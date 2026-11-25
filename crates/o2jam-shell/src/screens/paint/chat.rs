//! Chat-box paint generation: waiting, channel, and room lobby history
//! rows, draft ink, and focus carets.
//!
//! Kept behind re-exports in `super::paint`, so callers and the
//! existing scenario guards stay unchanged (#139 paint split).

use super::*;

/// Chat history view (`EDIT_SWAIT_CHAT 0x16300014` → rect 19,
/// `LIST_SWAIT_CHAT 0x1680002B` → rect 42, row template
/// `TEXT_SWAIT_CHAT 0x1640003C` under `SET 0x00031000`): one live row
/// per decoded 3012/3013 packet, fed through [`paint_waiting_chat`].
/// Rows come from live packets only — no history is invented.
pub(crate) const WAITING_CHAT_LIST_ID: &str = "IDC_LIST_SWAIT_CHAT";
/// Chat input line (`EDIT_SWAIT_CHAT` → rect 19).
pub(crate) const WAITING_CHAT_EDIT_ID: &str = "IDC_EDIT_SWAIT_CHAT";
/// Native history rect while no staged art names it. File-traced:
/// `State_Waiting.bnd` rect 42 `(24,455)-(511,553)` = `(24,455,487,98)`,
/// paired to `IDC_LIST_SWAIT_CHAT 0x1680002B` by ControlList code
/// ordinal (`code & 0xFFF` = 43 → ordinal 42) — geometry-audit.md
/// Waiting row, verified against `Interface1.opi`.
pub(crate) const WAITING_CHAT_LIST_RECT: Rect = Rect {
    x: 24,
    y: 455,
    w: 487,
    h: 98,
};
/// Native input rect while no staged art names it. File-traced:
/// `State_Waiting.bnd` rect 19 `(20,563)-(460,583)` = `(20,563,440,20)`,
/// paired to `IDC_EDIT_SWAIT_CHAT 0x16300014` by ControlList code
/// ordinal (`code & 0xFFF` = 20 → ordinal 19) — geometry-audit.md
/// Waiting row, verified against `Interface1.opi`.
pub(crate) const WAITING_CHAT_EDIT_RECT: Rect = Rect {
    x: 20,
    y: 563,
    w: 440,
    h: 20,
};
/// Row pitch of the history box, EXE-MEASURED 2026-11-09 (live rig
/// session, six 3011 rows in `~/scratch/geom-open-close/t1-chat-box.png`):
/// glyph tops at 456+17k, five 17 px intervals exact. (The earlier
/// photo-derived pitch agreed at 17.) Only the newest 5 rows render.
pub(crate) const WAITING_CHAT_ROW_H: i32 = 17;
/// Rows visible in the history box: EXE-OBSERVED 5 — the 98 px `LIST`
/// rect fits five 17 px rows and the sixth slot stays empty (rig
/// session 2026-11-09, `t1-chat-box.png`); was a 6-row derivation.
pub(crate) const WAITING_CHAT_VISIBLE_ROWS: usize = 5;
/// Focus-caret inset inside the input rect, EXE-MEASURED (rig session
/// 2026-11-09, `t1-waiting-empty.png`; capture before any canvas click
/// — the caret hides when the edit loses focus): caret ink bbox
/// (23,570) inside `EDIT_SWAIT_CHAT` rect 19 `(20,563,460,583)`.
pub(crate) const WAITING_CHAT_CARET_INSET: (i32, i32) = (3, 7);
/// Focus-caret bar size, EXE-MEASURED: solid core 1 px at x=24, rows
/// 570..578 (rig session 2026-11-09, same capture).
pub(crate) const WAITING_CHAT_CARET_SIZE: (u32, u32) = (2, 9);

/// One waiting-room chat row from a live 3012/3013 packet
/// (`ScreenGame` converts `WaitingUserMessage`/`WaitingAdminMessage`
/// into this display form; the painter never decodes the wire).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingChatMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text (UTF-8, lossy-decoded from the raw wire bytes).
    pub text: String,
    /// True for 3013 administrator lines — selects the `0x71e6` ink
    /// over the 3012 `0xdedede` (see [`chat_admin_ink`]).
    pub admin: bool,
}

impl WaitingChatMessage {
    /// Player chat row (3012).
    #[must_use]
    pub fn user(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: false,
        }
    }

    /// Administrator chat row (3013).
    #[must_use]
    pub fn admin(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: true,
        }
    }
}

/// Display text of one chat row: the enqueue printf's `[%12s] %s`
/// line (format VA `0x64d8d4`, call file `0x15f076`,
/// `docs/channel-fill.md`) — bracket, sender right-aligned to width
/// 12, bracket, space, text. User and administrator rows share the
/// line; the kind only picks the appended ink. ASCII names pad the
/// field exactly like the exe's C runtime; wider multibyte names ride
/// the presentation stand-in (rasterizer owner decision). The former
/// `[ADMIN]` prefix and the photo-guessed 13-wide field are replaced
/// by this pinned format.
#[must_use]
pub fn waiting_chat_row_text(message: &WaitingChatMessage) -> String {
    format!("[{:>12}] {}", message.sender, message.text)
}

/// Rows the history box shows: the newest [`WAITING_CHAT_VISIBLE_ROWS`].
#[must_use]
pub fn waiting_chat_visible_messages(messages: &[WaitingChatMessage]) -> &[WaitingChatMessage] {
    let skip = messages.len().saturating_sub(WAITING_CHAT_VISIBLE_ROWS);
    messages.get(skip..).unwrap_or(&[])
}

/// Ink of 3012 user rows: the exe poller appends with color
/// `0xdedede` (call file `0xb9f8c`, `docs/channel-fill.md`) through
/// the GDI `SetTextColor` path (`docs/gdi-text.md`) — COLORREF
/// `0x00BBGGRR`, symmetric here: light gray.
pub(crate) fn chat_user_ink() -> u32 {
    pack_xrgb(0xDE, 0xDE, 0xDE)
}

/// Ink of 3013 administrator rows: color `0x71e6` (call file
/// `0xb9fbf`) as a COLORREF — red `0xe6`, green `0x71`, blue `0x00`,
/// the warm amber every reference row reads as.
pub(crate) fn chat_admin_ink() -> u32 {
    pack_xrgb(0xE6, 0x71, 0x00)
}

/// Ink of one history row: the 3013 gate picks the admin amber,
/// every other row the user gray.
pub(crate) fn chat_row_ink(admin: bool) -> u32 {
    if admin {
        chat_admin_ink()
    } else {
        chat_user_ink()
    }
}

/// Ink of one display row: the 2018 refusal kind carries the exe's
/// zero value, so it paints no glyphs even though the row is staged.
#[must_use]
pub fn chat_row_ink_for(message: &ChannelChatMessage) -> u32 {
    if message.zero_ink {
        0
    } else {
        chat_row_ink(message.admin)
    }
}

/// Frame y of history row `offset`: the list origin plus
/// [`WAITING_CHAT_ROW_H`] steps (photo-pinned pitch).
#[must_use]
pub(crate) fn waiting_chat_row_y(list_y: i32, offset: usize) -> i32 {
    list_y.saturating_add(
        i32::try_from(offset)
            .unwrap_or(i32::MAX)
            .saturating_mul(WAITING_CHAT_ROW_H),
    )
}

/// Frame origin of the history box: the staged `LIST` rect while bound,
/// else the scaled native rect 42.
pub(crate) fn waiting_chat_list_origin(
    frame: &Frame<'_>,
    placements: &[ScenePlacement],
) -> (i32, i32) {
    if let Some(origin) = waiting_bnd_origin(placements, WAITING_CHAT_LIST_ID, frame) {
        return origin;
    }
    let scaled = scale_rect_to_frame(&WAITING_CHAT_LIST_RECT, frame.w, frame.h);
    (scaled.x, scaled.y)
}

/// Frame origin of the input line: the staged `EDIT` rect while bound,
/// else the scaled native rect 19.
pub(crate) fn waiting_chat_edit_origin(
    frame: &Frame<'_>,
    placements: &[ScenePlacement],
) -> (i32, i32) {
    if let Some(origin) = waiting_bnd_origin(placements, WAITING_CHAT_EDIT_ID, frame) {
        return origin;
    }
    let scaled = scale_rect_to_frame(&WAITING_CHAT_EDIT_RECT, frame.w, frame.h);
    (scaled.x, scaled.y)
}

/// Paint live chat rows plus the input line: the newest visible rows
/// stack from the history origin at the photo-pinned 17 px pitch,
/// straight on the box art (the reference rows carry no scrim band),
/// each in its appended ink — 3012 user rows `0xdedede`, 3013 admin
/// rows `0x71e6` ([`chat_row_ink`]) — the draft paints at the input
/// origin and the focus caret rides the insertion point (inset plus
/// typed width — the reference shows the caret on an empty input).
/// Rows come only from live packets; the caret is the input-line
/// state, not history. Missing placements fall back to the scaled
/// native rects so the flat fallback still reads.
pub fn paint_waiting_chat(
    frame: &mut Frame<'_>,
    messages: &[WaitingChatMessage],
    input: &str,
    placements: &[ScenePlacement],
) {
    let (list_x, list_y) = waiting_chat_list_origin(frame, placements);
    for (offset, message) in waiting_chat_visible_messages(messages).iter().enumerate() {
        draw_text(
            frame,
            list_x,
            waiting_chat_row_y(list_y, offset),
            &waiting_chat_row_text(message),
            chat_row_ink(message.admin),
        );
    }
    let (edit_x, edit_y) = waiting_chat_edit_origin(frame, placements);
    let caret_x =
        edit_x + WAITING_CHAT_CARET_INSET.0 + i32::try_from(text_width(input)).unwrap_or(i32::MAX);
    let caret_y = edit_y + WAITING_CHAT_CARET_INSET.1;
    if !input.is_empty() {
        draw_text(frame, edit_x, edit_y, input, art_title_ink());
    }
    fill_rect(
        frame,
        &Rect {
            x: caret_x,
            y: caret_y,
            w: WAITING_CHAT_CARET_SIZE.0,
            h: WAITING_CHAT_CARET_SIZE.1,
        },
        art_title_ink(),
    );
}

/// Rows visible in the channel history box: the newest
/// [`CHANNEL_CHAT_VISIBLE_ROWS`], mirroring the waiting box.
pub(crate) const CHANNEL_CHAT_VISIBLE_ROWS: usize = 6;
/// Channel chat rows share the lobby list pitch (8 px).
pub(crate) const CHANNEL_CHAT_ROW_H: i32 = 8;

/// One channel chat row from a live 2013/2014/2016/2019/2029 packet
/// (`ScreenGame` converts the wire message into this display form;
/// the painter never decodes the wire). Announcements (2016) carry
/// notice text with no sender; rank notices (2029) bracket the
/// ranked nickname around the song-plus-rank text; both ride the
/// admin amber like 2014 rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelChatMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text (UTF-8, lossy-decoded from the raw wire bytes).
    pub text: String,
    /// True for 2014 administrator lines (the 3013 `0x71e6` ink).
    pub admin: bool,
    /// Whisper addressee for 2019 rows (`None` for channel lines).
    pub whisper_recipient: Option<String>,
    /// True for the exe's 2018 refusal arm, which posts its failure
    /// name with zero ink (`0x55c190`, file `0x15c190`; REA
    /// `ev 4abe.96c9…`). All other row kinds keep their coloured ink.
    pub zero_ink: bool,
}

impl ChannelChatMessage {
    /// Player chat row (2013).
    #[must_use]
    pub fn user(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: false,
            whisper_recipient: None,
            zero_ink: false,
        }
    }

    /// Administrator chat row (2014).
    #[must_use]
    pub fn admin(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: true,
            whisper_recipient: None,
            zero_ink: false,
        }
    }

    /// Incoming whisper row (2019): the recipient is the local
    /// addressee, kept so the row names both ends.
    #[must_use]
    pub fn whisper(sender: &str, recipient: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: false,
            whisper_recipient: Some(recipient.to_owned()),
            zero_ink: false,
        }
    }

    /// Lobby announcement row (2016): sender-less notice text in the
    /// admin amber — the exe fans the string out as `0x7de`
    /// admin-message rows (`docs/net-protocol.md`), so the painter
    /// inks it like a 2014 row and prints it bare (no bracket
    /// without a sender).
    #[must_use]
    pub fn announcement(text: &str) -> Self {
        Self {
            sender: String::new(),
            text: text.to_owned(),
            admin: true,
            whisper_recipient: None,
            zero_ink: false,
        }
    }

    /// 2018 refusal row: the exe reads one name and posts it with a
    /// zero ink value, so the row paints no glyphs at all (#973).
    #[must_use]
    pub fn refusal(name: &str) -> Self {
        Self {
            sender: String::new(),
            text: name.to_owned(),
            admin: false,
            whisper_recipient: None,
            zero_ink: true,
        }
    }

    /// Rank-notice row (2029): the pinned notice-board sentence
    /// ([`rank_notice_board_text`]) in the admin amber (the same
    /// `0x7de` family as the announcement rows). The nickname rides
    /// inside the sentence, so the row carries no sender and paints
    /// bare; callers pass `M{id}` while the staged catalog names no
    /// song.
    #[must_use]
    pub fn rank(nickname: &str, song_title: &str, ranking: i32) -> Self {
        Self {
            sender: String::new(),
            text: rank_notice_board_text(nickname, song_title, ranking),
            admin: true,
            whisper_recipient: None,
            zero_ink: false,
        }
    }
}

/// Notice-board text of one rank notice (2029) in the exe field order:
/// nickname, song title, ranking. The five board writers match the staged
/// music id against the song table, then format nickname plus resolved
/// title plus ranking through VA `0x647f98` (file `0x247f98`) — brackets
/// around the title and the rank, Korean particles between. The shell
/// stages no song table, so callers pass `M{id}` while it names no song.
#[must_use]
pub fn rank_notice_board_text(nickname: &str, song_title: &str, ranking: i32) -> String {
    format!("{nickname} 음악전사님께서 [{song_title}] 곡에 새로운 랭킹 [{ranking}]위가 되셨습니다~")
}

/// Display text of one channel row: the waiting `[%12s] %s` bracket
/// line for channel rows; whisper rows name both ends as
/// `[sender -> recipient] text` so the addressee stays visible;
/// sender-less rows (announcements and rank notices) paint their text
/// bare (no bracket without a sender).
#[must_use]
pub fn channel_chat_row_text(message: &ChannelChatMessage) -> String {
    if let Some(recipient) = message.whisper_recipient.as_deref() {
        return format!(
            "[{:>12} -> {:>12}] {}",
            message.sender, recipient, message.text
        );
    }
    if message.sender.is_empty() {
        return message.text.clone();
    }
    format!("[{:>12}] {}", message.sender, message.text)
}

/// Rows the channel history shows: the newest [`CHANNEL_CHAT_VISIBLE_ROWS`].
#[must_use]
pub fn channel_chat_visible_messages(messages: &[ChannelChatMessage]) -> &[ChannelChatMessage] {
    let skip = messages.len().saturating_sub(CHANNEL_CHAT_VISIBLE_ROWS);
    messages.get(skip..).unwrap_or(&[])
}

/// Paint live channel rows below the channel list: the newest visible
/// rows stack at the lobby 8 px pitch, straight on the scene (no scrim
/// band), each in its appended ink — 2014 admin rows, 2016
/// announcements, and 2029 rank notices amber, every other row user
/// gray ([`chat_row_ink`]). Rows come only from live packets; empty
/// histories paint nothing.
pub fn paint_channel_chat(
    frame: &mut Frame<'_>,
    messages: &[ChannelChatMessage],
    channel_rows: usize,
) {
    for (offset, message) in channel_chat_visible_messages(messages).iter().enumerate() {
        let step = i32::try_from(offset).unwrap_or(i32::MAX);
        let y = index_as_row(channel_rows, frame.h as i32)
            .saturating_add(CHANNEL_ROW_H)
            .saturating_add(step.saturating_mul(CHANNEL_CHAT_ROW_H));
        draw_text(
            frame,
            frame.w as i32 / 4,
            y,
            &channel_chat_row_text(message),
            chat_row_ink_for(message),
        );
    }
}

/// Staged room-lobby chat list (`IDC_LIST_SROOM_CHAT 0x13800014`).
pub(crate) const ROOM_CHAT_LIST_CODE: u32 = 0x1380_0014;
/// Staged room-lobby chat input line (`IDC_EDIT_SROOM_CHAT 0x13300015`).
pub(crate) const ROOM_CHAT_EDIT_CODE: u32 = 0x1330_0015;
/// Row pitch of the room history box. The waiting box is exe-measured
/// at 17 px for the same CListBox-plus-`TEXT_*_CHAT` widget pair
/// ([`WAITING_CHAT_ROW_H`]); the room box is UNMEASURED, so this is a
/// labelled borrow (next step: rig capture of a live room chat list,
/// `~/scratch/planet-verify/` recipe).
pub(crate) const ROOM_CHAT_ROW_H: i32 = 17;
/// Focus-caret inset inside the room input rect: the waiting box's
/// exe-measured inset ([`WAITING_CHAT_CARET_INSET`]) borrowed for the
/// taller-by-one-pixel room edit (UNMEASURED, same next step).
pub(crate) const ROOM_CHAT_CARET_INSET: (i32, i32) = WAITING_CHAT_CARET_INSET;

/// Paint the room lobby chat panel (ticket #37): the newest history
/// rows stack from the staged `IDC_LIST_SROOM_CHAT` origin at
/// [`ROOM_CHAT_ROW_H`], each in its appended ink ([`chat_row_ink`]),
/// and the draft line inks the staged `IDC_EDIT_SROOM_CHAT` origin with
/// the focus caret at the borrowed inset. Both rects come from the
/// parsed staged block (owner rule `f38f3`): without them the painter
/// draws nothing (no fallback geometry is invented). Rows come only
/// from live packets.
pub fn paint_room_chat(
    frame: &mut Frame<'_>,
    messages: &[ChannelChatMessage],
    input: &str,
    placements: &[ScenePlacement],
) {
    let (frame_w, frame_h) = (frame.w, frame.h);
    let staged = |code: u32| {
        placements
            .iter()
            .find(|placed| placed.code == code)
            .map(|placed| scale_rect_to_frame(&placed.rect, frame_w, frame_h))
    };
    if let Some(list) = staged(ROOM_CHAT_LIST_CODE) {
        let pitch = u32::try_from(ROOM_CHAT_ROW_H).unwrap_or(1);
        let rows = usize::try_from(list.h / pitch).unwrap_or(0);
        let skip = messages.len().saturating_sub(rows);
        for (offset, message) in messages.get(skip..).unwrap_or(&[]).iter().enumerate() {
            let step = i32::try_from(offset).unwrap_or(i32::MAX);
            draw_text(
                frame,
                list.x,
                list.y.saturating_add(step.saturating_mul(ROOM_CHAT_ROW_H)),
                &channel_chat_row_text(message),
                chat_row_ink_for(message),
            );
        }
    }
    let Some(edit) = staged(ROOM_CHAT_EDIT_CODE) else {
        return;
    };
    if !input.is_empty() {
        draw_text(frame, edit.x, edit.y, input, art_title_ink());
    }
    let caret_x = edit
        .x
        .saturating_add(ROOM_CHAT_CARET_INSET.0)
        .saturating_add(i32::try_from(text_width(input)).unwrap_or(i32::MAX));
    fill_rect(
        frame,
        &Rect {
            x: caret_x,
            y: edit.y.saturating_add(ROOM_CHAT_CARET_INSET.1),
            w: WAITING_CHAT_CARET_SIZE.0,
            h: WAITING_CHAT_CARET_SIZE.1,
        },
        art_title_ink(),
    );
}
