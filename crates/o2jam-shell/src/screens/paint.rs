//! Screen scenes: real `.ojs`/`.bnd` art behind each [`Screen`],
//! with the working UI ink on top.
//!
//! Each scene paints its verified archive art first (lifecycle §5–11:
//! `O2mediaLogo.ojs`, channel/room/waiting/playing/loading/result blocks)
//! through the scene kit in [`crate::avatar`] (`load_scene_figures`,
//! `bind_scene_rects`, `anchor_placements`, `paint_scene_figures`).
//! Rect ordinal `(code & 0xFFF) − 1` pairs each entry (Playing: the
//! `EXIT`..`VOLUME` run, entries 213–229, ordinals 1–17); every layer
//! paints 1:1 at its record anchor scaled into the frame — the
//! verified draw chain (the blit adds the anchor to the view
//! position; anchored bodies author their absolute position) — over
//! the view origin its placement stages: a bound rect positions
//! anchor-free bodies (lobby row icons, avatar covers), unbound rects
//! leave the frame origin. Painting stays in file order with the
//! background first.
//! Number-font bodies (`Room_Number.ojs` and kin) paint live values
//! digit by digit; the lobby row icons (`Room_State.ojs`,
//! `Room_Mode.ojs`, `Room_Om.ojs`) paint the highlighted room's
//! frames at their `SET 0x11` template rects; entry codes gate
//! visibility by state byte. The result screen pins its readouts as
//! GDI text on the native canvas and selects verdict/grade frames
//! from the staged bodies (`docs/cresult-anchors.md`). Bare
//! [`paint_screen`] keeps the flat-color fallback while no art is
//! staged; staged scenes never paint flat cards.

use std::collections::HashMap;

use super::super::avatar::{
    anchor_in_frame, paint_behind_placements, paint_foreground_placements, paint_scene_figures,
    scene_block, scene_sprite_names, AvatarFigure, ScenePlacement, AVI_BLOCK, AVI_LOGO_SPRITE,
    CHANNEL_BLOCK, LOADING_BLOCK, LOADING_SPRITE, RESULT_BLOCK, ROOM_BLOCK, WAITING_BLOCK,
    WAIT_COVER_SPRITE, WAIT_TEAM_SPRITES,
};
use super::super::playing::{HighwayGeometry, PlayingView};
use super::super::screen::Screen;
use o2jam_file::ControlList;
use o2jam_game::RoomTeams;
use o2jam_net::{
    rooms::om_icon_frame, ChannelState, Difficulty, GameMode, GameSpeed, JoinedRoom, KeyMode,
    RoomListEntry, RoomSlot, RoomSlotState,
};
use o2jam_render::{
    blit_rgb555, draw_number, draw_text, fill_rect, number_width, pack_xrgb, scale_rect_to_frame,
    text_width, Frame, Rect, Rgb555Image, NATIVE_HEIGHT, NATIVE_WIDTH,
};

/// Gameplay block: real UI lives in `Playing1.opi` (235 entries,
/// lifecycle §10).
pub const PLAYING_BLOCK: &str = "STATE_PLAYING";
/// Gameplay layout bind file: 17 rects over the bound run (entries
/// 213–229, `EXIT`..`VOLUME`), paired by code ordinal 1–17.
pub const PLAYING_BIND_FILE: &str = "State_Playing.bnd";
/// Pre-game room layout bind file.
pub const WAITING_BIND_FILE: &str = "State_Waiting.bnd";
/// Results layout bind file (7 rects).
pub const RESULT_BIND_FILE: &str = "State_Result.bnd";

/// Staged art for one scene: `.bnd` placements plus figures loaded
/// through `load_scene_figures`. Empty while nothing is staged; every
/// scene painter falls back to flat colors then.
#[derive(Debug, Clone, Default)]
pub struct SceneArt {
    /// `.bnd` layout placements in file order.
    pub placements: Vec<ScenePlacement>,
    /// Decoded sprite bodies by archive entry name.
    pub figures: HashMap<String, AvatarFigure>,
}

impl SceneArt {
    /// Whether any figure decoded (placements alone paint nothing).
    #[must_use]
    pub fn has_figures(&self) -> bool {
        !self.figures.is_empty()
    }

    /// Paint staged figures at their verbatim native origins with no
    /// flat fill: art-less states contribute zero staged pixels (an
    /// art-less block stages no controls with art and the render loop
    /// draws stored controls only), and no backbuffer fill is evidenced
    /// in the exe (no `FillRect` import or `Fill`/`Paint`/`Clear` fill
    /// vocabulary; `CWindow`/`CDialogBase` survive only as RTTI names),
    /// so an empty stage leaves the frame untouched and the live ink
    /// layers on top. Callers without any staged art keep the flat
    /// `paint_screen` card fallback.
    pub fn paint_backdrop(&self, frame: &mut Frame<'_>, _screen: Screen) {
        if !self.has_figures() {
            // Pinned no-fill rule (`docs/channel-fill.md`): the exe
            // contributes zero pixels behind art-less states and no
            // fill call is evidenced, so the shell paints nothing and
            // leaves the backbuffer to the presenter.
            return;
        }
        self.paint_overlay(frame);
    }

    /// Paint placed figures over the current frame content (gauges
    /// and panels ring the highway; see [`paint_playing_overlay`]).
    pub fn paint_overlay(&self, frame: &mut Frame<'_>) {
        paint_scene_figures(frame, &self.placements, &self.figures);
    }
}

/// Slot of one screen in the per-scene art table.
#[must_use]
pub const fn scene_index(screen: Screen) -> usize {
    match screen {
        Screen::Avi => 0,
        Screen::Channel => 1,
        Screen::Room => 2,
        Screen::Waiting => 3,
        Screen::Loading => 4,
        Screen::Playing => 5,
        Screen::Result => 6,
    }
}

/// Channel dialog art: code-fetched (`Dialog_ChannelSelectBar.ojs` is a
/// real archive body named by no block), so it is never part of the
/// `STATE_CHANNEL` scene. The single code fetch sits at file `0x8de84`
/// (VA `0x48de84`) inside the builder at file `0x8dd10` (VA `0x48dd10`),
/// beside `ControlList`-parent (`0x762B44`) traffic and ~63 KB below the
/// dialog-open path (`0x4ED650`, file `0xed650`) — the bar is not mounted
/// by `0x4ED650`, and with no `DIALOG_` literal in the image its dialog
/// host stays unpinned. Entry-less mounted art stages through
/// [`unbound_scene_placements`] with an empty id and paints at its
/// record anchor.
pub const DIALOG_BAR_SPRITE: &str = "Dialog_ChannelSelectBar.ojs";

/// Sprite bodies one `ControlList` block names, in file order (empty
/// for unknown blocks).
fn block_sprites(list: &ControlList, block: &str) -> Vec<String> {
    scene_block(list, block).map_or_else(Vec::new, scene_sprite_names)
}

/// Sprite bodies one block names, or the single-sprite fallback when
/// the block is missing or names nothing (splash/loading are
/// one-entry states).
fn block_or_single(list: &ControlList, block: &str, single: &str) -> Vec<String> {
    let names = block_sprites(list, block);
    if names.is_empty() {
        vec![single.to_owned()]
    } else {
        names
    }
}

/// Sprite bodies one main scene needs, in block order (lifecycle
/// §5–11). Splash and loading resolve to their single art sprites
/// (code-mounted, not block-named); channel, room, waiting,
/// playing, and result name their block entries for `bind_scene_rects`
/// (`STATE_CHANNEL` names zero sprites on 7.55 — verified byte walk
/// in `docs/channel-fill.md` — so the channel scene stages nothing:
/// sprite-less entries draw no art per recipe step 6 and the scene
/// keeps its flat background plus the live list).
#[must_use]
pub fn main_scene_sprites(list: &ControlList, screen: Screen) -> Vec<String> {
    match screen {
        Screen::Avi => block_or_single(list, AVI_BLOCK, AVI_LOGO_SPRITE),
        Screen::Channel => block_sprites(list, CHANNEL_BLOCK),
        Screen::Room => block_sprites(list, ROOM_BLOCK),
        Screen::Waiting => block_sprites(list, WAITING_BLOCK),
        Screen::Loading => block_or_single(list, LOADING_BLOCK, LOADING_SPRITE),
        Screen::Playing => block_sprites(list, PLAYING_BLOCK),
        Screen::Result => block_sprites(list, RESULT_BLOCK),
    }
}

/// Placements for code-mounted sprites without a block entry
/// (loading gate): no entry means no id, code,
/// or layout to claim, so each sprite paints 1:1 at its own record
/// anchor with an empty rect — never full-bleed. Entries of bare-
/// `BOUND` blocks keep their real id and code through
/// [`anchor_placements`](super::super::avatar::anchor_placements);
/// this bare-name form is only for entry-less mounted art.
#[must_use]
pub fn unbound_scene_placements(names: &[String]) -> Vec<ScenePlacement> {
    names
        .iter()
        .map(|name| ScenePlacement {
            id: String::new(),
            code: 0,
            sprite: Some(name.clone()),
            rect: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
        })
        .collect()
}

/// Title ink on lobby cards (dark on the light card).
fn title_color() -> u32 {
    pack_xrgb(0x20, 0x20, 0x20)
}

/// Scrim ink behind live UI over staged art: near-black, distinct
/// from every UI ink so text stays readable on bright OPI panels.
fn scrim_color() -> u32 {
    pack_xrgb(0x08, 0x08, 0x10)
}

/// Title ink over staged art (white on the scrim).
fn art_title_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Top-left of a centered title (shared by the card and art scenes).
fn title_origin(frame: &Frame<'_>, title: &str) -> (i32, i32) {
    let text_w = text_width(title) as i32;
    (
        frame.w as i32 / 2 - text_w / 2,
        frame.h as i32 / 2 - frame.h as i32 / 8 + 2,
    )
}

/// Title over staged art: dark scrim plus white ink (the card title
/// stays dark-on-light for the flat fallback).
fn paint_title_over_art(frame: &mut Frame<'_>, screen: Screen) {
    let title = title_of(screen);
    if title.is_empty() {
        return;
    }
    let (x, y) = title_origin(frame, title);
    let text_w = text_width(title) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 4,
            y: y - 2,
            w: (text_w + 8).max(0) as u32,
            h: 9,
        },
        scrim_color(),
    );
    draw_text(frame, x, y, title, art_title_ink());
}

/// Dark strip behind `row_count` channel/room rows so list ink
/// stays readable over staged panels (skips the frame origin where
/// staged art anchors at the native origin).
fn paint_list_scrim(frame: &mut Frame<'_>, row_count: usize) {
    if row_count == 0 || frame.w == 0 || frame.h == 0 {
        return;
    }
    let w = frame.w as i32;
    let h = frame.h as i32;
    fill_rect(
        frame,
        &Rect {
            x: w / 4 - 8,
            y: h / 4 - 2,
            w: (w / 2 + 16).max(0) as u32,
            h: (row_count as i32 * CHANNEL_ROW_H + 4).max(0) as u32,
        },
        scrim_color(),
    );
}

/// Row height of one channel entry.
const CHANNEL_ROW_H: i32 = 8;

/// One channel row: `gateway/channel population/capacity` (both ids
/// ride the `GetChannelList` 1003 row; population over capacity).
#[must_use]
fn channel_row_text(channel: &ChannelState) -> String {
    format!(
        "{}/{} {}/{}",
        channel.gateway_id, channel.channel_id, channel.population, channel.capacity
    )
}

/// Channel rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene).
fn paint_channel_rows(frame: &mut Frame<'_>, channels: &[ChannelState], highlight: usize) {
    paint_list_scrim(frame, channels.len());
    let w = frame.w as i32;
    let h = frame.h as i32;
    let text_x = w / 4;
    for (index, channel) in channels.iter().enumerate() {
        let row = index_as_row(index, h);
        let entry = channel_row_text(channel);
        let ink = if channel.active {
            pack_xrgb(0xFF, 0xFF, 0xFF)
        } else {
            pack_xrgb(0x70, 0x70, 0x70)
        };
        draw_text(frame, text_x, row, &entry, ink);
        if index == highlight {
            paint_cursor(frame, text_x, row);
        }
    }
}

/// Row of list entry `index` (channel and room lists share the shape).
fn index_as_row(index: usize, h: i32) -> i32 {
    h / 4 + i32::try_from(index).unwrap_or(i32::MAX) * CHANNEL_ROW_H
}

/// Amber highlight cursor left of a list row.
fn paint_cursor(frame: &mut Frame<'_>, text_x: i32, y: i32) {
    fill_rect(
        frame,
        &Rect {
            x: text_x - 6,
            y,
            w: 3,
            h: 5,
        },
        pack_xrgb(0xFF, 0xD0, 0x20),
    );
}

/// Paint the online channel list with a highlight cursor.
pub(crate) fn paint_channel_list(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    highlight: usize,
) {
    fill_background(frame, Screen::Channel);
    paint_title(frame, "CHANNEL");
    paint_channel_rows(frame, channels, highlight);
}

/// Paint staged channel art with the live list and cursor on top.
/// With empty art the shell paints no fill (pinned no-fill rule per
/// `docs/channel-fill.md`: the exe stages zero pixels behind art-less
/// states and no fill call is evidenced) and layers only the live
/// list, title, and cursor.
pub fn paint_channel_scene(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    highlight: usize,
    art: &SceneArt,
) {
    art.paint_backdrop(frame, Screen::Channel);
    paint_title_over_art(frame, Screen::Channel);
    paint_channel_rows(frame, channels, highlight);
}

/// One room row: `3 TITLE 5/8`, locked rooms suffixed `!`.
fn room_row_text(room: &RoomListEntry) -> String {
    let mut text = format!(
        "{} {} {}/{}",
        room.number, room.title, room.user_count, room.capacity
    );
    if room.has_password {
        text.push('!');
    }
    text
}

/// Room rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene).
fn paint_room_rows(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    paint_list_scrim(frame, rooms.len());
    let w = frame.w as i32;
    let h = frame.h as i32;
    let text_x = w / 4;
    for (index, room) in rooms.iter().enumerate() {
        let row = index_as_row(index, h);
        draw_text(
            frame,
            text_x,
            row,
            &room_row_text(room),
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        if index == highlight {
            paint_cursor(frame, text_x, row);
        }
    }
}

/// Title anchor of the highlighted-room panel (`SET 0x11` row
/// template in `STATE_ROOM`).
const ROOM_DETAIL_TITLE_ID: &str = "IDC_TEXT_SROOM_ROOMNAME";
/// Seat-count anchor (`ROOM_USER`).
const ROOM_DETAIL_USERS_ID: &str = "IDC_TEXT_SROOM_ROOM_USER";
/// Selected-song anchor.
const ROOM_DETAIL_MUSIC_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE";
/// Difficulty anchor (second music-title slot).
const ROOM_DETAIL_DIFFICULTY_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE2";
/// Scroll-speed icon anchor; text stands in for the icon body.
const ROOM_DETAIL_SPEED_ID: &str = "IDC_IMAGE_SROOM_SPEED";
/// Game-mode icon anchor (`Room_Mode.ojs`); text stands in.
const ROOM_DETAIL_MODE_ID: &str = "IDC_IMAGE_SROOM_MODE";
/// Key-mode anchor (`Room_Om.ojs` option-mode icon).
const ROOM_DETAIL_KEY_ID: &str = "IDC_IMAGE_SROOM_OM";
/// Level-gate text anchor beside `Room_LevelLimit.ojs`.
const ROOM_DETAIL_LEVEL_ID: &str = "IDC_TEXT_SROOM_LEVEL_RANGE";
/// Lock-icon anchor (`Room_Password.ojs`); text stands in.
const ROOM_DETAIL_LOCK_ID: &str = "IDC_IMAGE_SROOM_PASSWORD";

/// Detail lines per panel: title plus the eight entry fields.
const ROOM_DETAIL_LINES: usize = 9;

/// Game-mode label (`NORMAL`/`LIVE`; raw code while unknown).
#[must_use]
fn room_mode_label(game_mode: u8) -> String {
    match GameMode::from_u8(game_mode) {
        Ok(GameMode::Normal) => "NORMAL".to_owned(),
        Ok(GameMode::Live) => "LIVE".to_owned(),
        Err(_) => format!("MODE {game_mode}"),
    }
}

/// Selected-song label (the song table lives server-side, so the id).
#[must_use]
fn room_music_label(music_id: u16) -> String {
    format!("M{music_id}")
}

/// Difficulty label (`EX`/`NX`/`HX`/`RX`; raw code while unknown).
#[must_use]
fn room_difficulty_label(difficulty: u8) -> String {
    match Difficulty::from_code(difficulty) {
        Some(Difficulty::Ex) => "EX".to_owned(),
        Some(Difficulty::Nx) => "NX".to_owned(),
        Some(Difficulty::Hx) => "HX".to_owned(),
        Some(Difficulty::Rx) => "RX".to_owned(),
        None => format!("DIF {difficulty}"),
    }
}

/// Key-mode label (`3KEY`/`5KEY`/`7KEY`; raw code while unknown).
#[must_use]
fn room_key_label(key_mode: u8) -> String {
    match KeyMode::from_u8(key_mode) {
        Ok(KeyMode::ThreeKeys) => "3KEY".to_owned(),
        Ok(KeyMode::FiveKeys) => "5KEY".to_owned(),
        Ok(KeyMode::SevenKeys) => "7KEY".to_owned(),
        Err(_) => format!("KEY {key_mode}"),
    }
}

/// Scroll-speed label (`X0.5`..`X6`, `XR`; raw code while unknown).
#[must_use]
fn room_speed_label(speed: u8) -> String {
    match GameSpeed::from_code(speed) {
        Some(GameSpeed::X05) => "X0.5".to_owned(),
        Some(GameSpeed::X10) => "X1".to_owned(),
        Some(GameSpeed::X15) => "X1.5".to_owned(),
        Some(GameSpeed::X20) => "X2".to_owned(),
        Some(GameSpeed::X25) => "X2.5".to_owned(),
        Some(GameSpeed::X30) => "X3".to_owned(),
        Some(GameSpeed::X35) => "X3.5".to_owned(),
        Some(GameSpeed::X40) => "X4".to_owned(),
        Some(GameSpeed::X45) => "X4.5".to_owned(),
        Some(GameSpeed::X50) => "X5".to_owned(),
        Some(GameSpeed::X60) => "X6".to_owned(),
        Some(GameSpeed::Xr) => "XR".to_owned(),
        None => format!("SPD {speed}"),
    }
}

/// Seat label (`seated/capacity`).
#[must_use]
fn room_capacity_label(room: &RoomListEntry) -> String {
    format!("{}/{}", room.user_count, room.capacity)
}

/// Level-gate label (`LV min-max`).
#[must_use]
fn room_level_label(room: &RoomListEntry) -> String {
    format!("LV {}-{}", room.min_level, room.max_level)
}

/// Lock label (`LOCK` with a password, `OPEN` without).
#[must_use]
fn room_lock_label(has_password: bool) -> &'static str {
    if has_password {
        "LOCK"
    } else {
        "OPEN"
    }
}

/// Lock-line ink: red while a password is set, dim otherwise.
fn room_lock_ink(locked: bool) -> u32 {
    if locked {
        pack_xrgb(0xFF, 0x50, 0x50)
    } else {
        pack_xrgb(0x90, 0x90, 0x90)
    }
}

/// Ordered detail entries for one room: (`State_Room.bnd`
/// control id, text).
#[must_use]
fn room_detail_entries(room: &RoomListEntry) -> [(&'static str, String); ROOM_DETAIL_LINES] {
    [
        (ROOM_DETAIL_TITLE_ID, room.title.clone()),
        (ROOM_DETAIL_USERS_ID, room_capacity_label(room)),
        (ROOM_DETAIL_MUSIC_ID, room_music_label(room.music_id)),
        (
            ROOM_DETAIL_DIFFICULTY_ID,
            room_difficulty_label(room.difficulty),
        ),
        (ROOM_DETAIL_KEY_ID, room_key_label(room.key_mode)),
        (ROOM_DETAIL_MODE_ID, room_mode_label(room.game_mode)),
        (ROOM_DETAIL_SPEED_ID, room_speed_label(room.speed)),
        (ROOM_DETAIL_LEVEL_ID, room_level_label(room)),
        (
            ROOM_DETAIL_LOCK_ID,
            room_lock_label(room.has_password).to_owned(),
        ),
    ]
}

/// Scaled frame origin of one `State_Room.bnd` control (`None`
/// while the staged art names no such control).
#[must_use]
fn room_detail_bnd_origin(
    placements: &[ScenePlacement],
    id: &str,
    frame: &Frame<'_>,
) -> Option<(i32, i32)> {
    let placement = placements.iter().find(|placed| placed.id == id)?;
    let scaled = scale_rect_to_frame(&placement.rect, frame.w, frame.h);
    Some((scaled.x, scaled.y))
}

/// Fallback origin of detail line `slot` without staged art: stacked
/// rows under the list so the list plus cursor keep their room.
fn room_detail_fallback_origin(slot: usize, row_count: usize, frame: &Frame<'_>) -> (i32, i32) {
    let rows = i32::try_from(row_count).unwrap_or(i32::MAX);
    let slot_offset = i32::try_from(slot).unwrap_or(i32::MAX);
    (
        frame.w as i32 / 4,
        frame.h as i32 / 4 + rows * CHANNEL_ROW_H + 4 + slot_offset * CHANNEL_ROW_H,
    )
}

/// One detail line: dark scrim plus ink (shared by the flat fallback
/// and the art scene).
fn paint_room_detail_line(frame: &mut Frame<'_>, x: i32, y: i32, text: &str, ink: u32) {
    let text_w = text_width(text) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 4,
            y: y - 2,
            w: (text_w + 8).max(0) as u32,
            h: 9,
        },
        scrim_color(),
    );
    draw_text(frame, x, y, text, ink);
}

/// Decimal weight of a digit-place id: its trailing decimal run
/// (`ROOMNO100` → 100, `RMNUM_010` → 10, `CHNUM_01` → 1). `None`
/// without a trailing run: `SPEED_NUM` holds one value, not one
/// digit per place.
#[must_use]
fn digit_place_weight(id: &str) -> Option<u32> {
    let digits = id.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    id[id.len() - digits..].parse().ok()
}

/// True for a digit-place weight: powers of ten only. Shared bodies
/// with sequential suffixes (`SHADOW1`…`8`, `JAM_NUM1`…`8`) are frame
/// lists, not digit places, so only 1/10/100/… group into a number.
#[must_use]
fn is_place_weight(mut weight: u32) -> bool {
    if weight == 0 {
        return false;
    }
    while weight.is_multiple_of(10) {
        weight /= 10;
    }
    weight == 1
}

/// Digit places sharing one number-font body, in file order:
/// `(weight, place rect)` per entry whose id carries a place weight
/// (`ROOMNO1/10/100` share `Room_Number.ojs` on real 7.55 art).
#[must_use]
fn font_places(placements: &[ScenePlacement], font: &str) -> Vec<(u32, Rect)> {
    placements
        .iter()
        .filter(|placed| placed.sprite.as_deref() == Some(font))
        .filter_map(|placed| {
            let weight = digit_place_weight(&placed.id)?;
            is_place_weight(weight).then_some((weight, placed.rect))
        })
        .collect()
}

/// Paint `value` through a staged number font: the place of weight
/// `w` shows digit `(value / w) % 10`, blitted 1:1 from the font
/// figure's same-index layer at the place-rect origin plus the
/// layer's record anchor (`Room_Number.ojs` decodes ten digit layers
/// in stored order, each 11x13 over its own 11x13 place rect, so
/// anchor and rect coincide on real art). Missing fonts and short
/// fonts paint nothing; every other staged body is untouched.
fn paint_font_number(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    font: &str,
    value: u32,
) {
    let Some(figure) = figures.get(font) else {
        return;
    };
    for (weight, rect) in font_places(placements, font) {
        let digit = value / weight % 10;
        let layer = usize::try_from(digit)
            .ok()
            .and_then(|index| figure.layer(index));
        if let Some(shown) = layer {
            let dest = scale_rect_to_frame(&rect, frame.w, frame.h);
            blit_rgb555(
                frame,
                &shown.image.view(),
                dest.x + shown.dx,
                dest.y + shown.dy,
                Some(shown.colorkey),
            );
        }
    }
}

/// Number-font bodies staged for this scene: every sprite with two
/// or more digit places. `STATE_ROOM` carries exactly one
/// (`Room_Number.ojs` over `ROOMNO1/10/100`, verified on 7.55).
#[must_use]
fn staged_number_fonts(placements: &[ScenePlacement]) -> Vec<&str> {
    let mut fonts = Vec::new();
    for placed in placements {
        let Some(sprite) = placed.sprite.as_deref() else {
            continue;
        };
        if !fonts.contains(&sprite) && font_places(placements, sprite).len() >= 2 {
            fonts.push(sprite);
        }
    }
    fonts
}

/// Room number through the staged number fonts: `ROOMNO1/10/100`
/// show the highlighted room's number digit by digit, each in its
/// own `.bnd` rect. No staged font means no digits (the row text
/// already names the number); nothing paints without a room.
fn paint_room_number(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    rooms: &[RoomListEntry],
    highlight: usize,
) {
    let Some(room) = rooms.get(highlight) else {
        return;
    };
    let value = u32::try_from(room.number.max(0)).unwrap_or(u32::MAX);
    for font in staged_number_fonts(&art.placements) {
        paint_font_number(frame, &art.placements, &art.figures, font, value);
    }
}

/// Lobby row-state icon body (`SET 0x11` row template, child 12):
/// frame = the raw state byte (icon site `0x1268ae`), so frames 0/1/2
/// drive its images (docs/room-enums.md).
pub const ROOM_STATE_SPRITE: &str = "Room_State.ojs";
/// Lobby key-mode icon body (child 7): frame 1 = 3-key, frame 2 =
/// 7-key — the key-mode mark, not the battle mark (icon sites
/// `0x1267ad`–`0x1267e2`).
pub const ROOM_MODE_SPRITE: &str = "Room_Mode.ojs";
/// Lobby option-mode icon body (child 8): frames 1–4 depict 1–4
/// pudding mascots, banded by the selected song's level word (band
/// mapper `0x126cb0`).
pub const ROOM_OM_SPRITE: &str = "Room_Om.ojs";
/// Music floor a room row must pass to run the per-row fill (row
/// `+0x88` vs `0x64`, file `0x1251da`); below it the room shows the
/// no-song display state.
pub const ROOM_MUSIC_FLOOR: u16 = 0x64;

/// `Room_State.ojs` frame for one room: the state byte unmodified
/// (icon site `0x1268ae`, docs/room-enums.md).
#[must_use]
fn room_state_icon_frame(state_code: u8) -> u8 {
    state_code
}

/// `Room_Mode.ojs` frame for one room: key mode 0 → frame 1, 2 →
/// frame 2, anything else untouched (`None` — the exe leaves the
/// icon alone; icon sites `0x1267ad`–`0x1267e2`).
#[must_use]
fn room_key_icon_frame(key_mode: u8) -> Option<u8> {
    match KeyMode::from_u8(key_mode) {
        Ok(KeyMode::ThreeKeys) => Some(1),
        Ok(KeyMode::SevenKeys) => Some(2),
        _ => None,
    }
}

/// `Room_Om.ojs` frame for one room: below the pinned music floor the
/// row shows the no-song display state → constant level 1 → frame 1
/// (mapper call `0x1266b9`); past the floor the song's level word
/// maps through the band mapper ([`om_icon_frame`], bands 0–4/5–8/
/// 9–12/13+ → frames 1–4). The shell stages no song table (it lives
/// server-side), so a song without a known level paints nothing
/// rather than a wrong pudding count.
#[must_use]
fn room_om_icon_frame(music_id: u16, level: Option<u16>) -> Option<u8> {
    if music_id < ROOM_MUSIC_FLOOR {
        Some(1)
    } else {
        level.map(om_icon_frame)
    }
}

/// Image index the exe sprite setter paints for frame value `frame`
/// (setter `0x2f870`: store verbatim, clamp against the stored count,
/// subtract 1 before the image lookup — frame *n* paints image *n−1*):
/// saturating at image 0 for frame 0, clamped at the last decoded
/// layer otherwise. `None` for an empty body.
#[must_use]
fn icon_frame_layer(frame: u8, layers: usize) -> Option<usize> {
    if layers == 0 {
        return None;
    }
    Some(usize::from(frame.saturating_sub(1)).min(layers - 1))
}

/// Paint one frame of a staged icon body at its bound placement: the
/// first placement naming `sprite` supplies the rect, the frame's
/// layer blits 1:1 at the rect origin plus its record anchor (the
/// same draw math as [`paint_font_number`]; the row icons anchor at
/// `(0,0)`, so the rect origin positions them). A missing placement,
/// figure, frame, or layer paints nothing — the exe leaves an
/// untouched icon exactly so.
fn paint_icon_frame(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    icon_frame: Option<u8>,
) {
    let Some(icon_frame) = icon_frame else {
        return;
    };
    let Some(placed) = placements
        .iter()
        .find(|placed| placed.sprite.as_deref() == Some(sprite))
    else {
        return;
    };
    let Some(figure) = figures.get(sprite) else {
        return;
    };
    let Some(layer_index) = icon_frame_layer(icon_frame, figure.len()) else {
        return;
    };
    let Some(layer) = figure.layer(layer_index) else {
        return;
    };
    let dest = scale_rect_to_frame(&placed.rect, frame.w, frame.h);
    blit_rgb555(
        frame,
        &layer.image.view(),
        dest.x + layer.dx,
        dest.y + layer.dy,
        Some(layer.colorkey),
    );
}

/// Row icons of the highlighted room: state, key mode, and Om
/// pudding count, each at its `State_Room.bnd` icon rect (the `SET
/// 0x11` row template binds one copy per icon; the exe clones the
/// template per row button). Missing art paints nothing; the detail
/// text draws on top.
fn paint_room_row_icons(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    rooms: &[RoomListEntry],
    highlight: usize,
) {
    let Some(room) = rooms.get(highlight) else {
        return;
    };
    paint_icon_frame(
        frame,
        &art.placements,
        &art.figures,
        ROOM_STATE_SPRITE,
        Some(room_state_icon_frame(room.state_code)),
    );
    paint_icon_frame(
        frame,
        &art.placements,
        &art.figures,
        ROOM_MODE_SPRITE,
        room_key_icon_frame(room.key_mode),
    );
    paint_icon_frame(
        frame,
        &art.placements,
        &art.figures,
        ROOM_OM_SPRITE,
        room_om_icon_frame(room.music_id, None),
    );
}

/// Details of the highlighted room: title, song, difficulty, key
/// mode, game mode, speed, seats, level gate, lock. Lines anchor to
/// their `State_Room.bnd` rects while staged art names them and
/// stack under the list otherwise; nothing paints without a room.
fn paint_room_details(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    placements: Option<&[ScenePlacement]>,
) {
    let Some(room) = rooms.get(highlight) else {
        return;
    };
    for (slot, (id, text)) in room_detail_entries(room).iter().enumerate() {
        let origin = placements
            .and_then(|placed| room_detail_bnd_origin(placed, id, frame))
            .unwrap_or_else(|| room_detail_fallback_origin(slot, rooms.len(), frame));
        let ink = if *id == ROOM_DETAIL_LOCK_ID {
            room_lock_ink(room.has_password)
        } else {
            art_title_ink()
        };
        paint_room_detail_line(frame, origin.0, origin.1, text, ink);
    }
}

/// Paint the online room list with a highlight cursor.
pub(crate) fn paint_room_list(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    fill_background(frame, Screen::Room);
    paint_title(frame, "ROOM");
    paint_room_rows(frame, rooms, highlight);
    paint_room_details(frame, rooms, highlight, None);
}

/// Paint staged lobby art with the live room list and cursor on top.
pub fn paint_room_scene(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
) {
    art.paint_backdrop(frame, Screen::Room);
    paint_title_over_art(frame, Screen::Room);
    paint_room_rows(frame, rooms, highlight);
    paint_room_row_icons(frame, art, rooms, highlight);
    paint_room_details(frame, rooms, highlight, Some(&art.placements));
    paint_room_number(frame, art, rooms, highlight);
}

/// Seats shown on the waiting screen: self plus the seven wire slots.
pub(crate) const WAITING_SEATS: usize = 8;
/// Peer slots carried beside self in [`JoinedRoom`].
const WAITING_PEERS: usize = 7;
/// Waiting rows share the lobby row height.
const WAITING_ROW_H: i32 = 8;

/// One waiting-room seat view: display name plus team/ready state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaitingSeat {
    team: u8,
    name: String,
    kind: SeatKind,
}

/// Seat occupancy: only occupied seats carry a name and ready flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeatKind {
    Occupied { ready: bool },
    Empty,
    Locked,
}

impl WaitingSeat {
    /// Display name (`None` for vacant/locked seats).
    #[must_use]
    pub(crate) fn display_name(&self) -> Option<&str> {
        match self.kind {
            SeatKind::Occupied { .. } => Some(&self.name),
            SeatKind::Empty | SeatKind::Locked => None,
        }
    }

    /// Ready flag (vacant/locked seats are never ready).
    #[must_use]
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.kind, SeatKind::Occupied { ready: true })
    }
}

/// Team button letter (`0` -> `A` .. `7` -> `H`); `?` when out of range.
fn team_letter(team: u8) -> char {
    if team < 8 {
        (b'A' + team) as char
    } else {
        '?'
    }
}

/// Our own seat: name is local-only (the wire sends no self nickname).
fn self_seat(room: &JoinedRoom) -> WaitingSeat {
    WaitingSeat {
        team: room.team,
        name: "YOU".to_owned(),
        kind: SeatKind::Occupied { ready: false },
    }
}

/// Vacant seat placeholder.
fn empty_seat() -> WaitingSeat {
    WaitingSeat {
        team: 0,
        name: String::new(),
        kind: SeatKind::Empty,
    }
}

/// View for one wire slot: named when occupied, locked/empty otherwise.
fn peer_seat(slot: &RoomSlot) -> WaitingSeat {
    match &slot.member {
        Some(member) => WaitingSeat {
            team: member.team,
            name: member.nickname.clone(),
            kind: SeatKind::Occupied {
                ready: member.ready,
            },
        },
        None if slot.state_code == RoomSlotState::Locked as i32 => WaitingSeat {
            team: 0,
            name: String::new(),
            kind: SeatKind::Locked,
        },
        None => empty_seat(),
    }
}

/// Eight seat views: our seat first, then the wire slots, padded empty.
///
/// The wire carries seven peer slots beside self (`JoinedRoom.slots`);
/// together they are the eight waiting-room seats.
#[must_use]
pub(crate) fn waiting_seats(room: &JoinedRoom) -> Vec<WaitingSeat> {
    let mut seats = Vec::with_capacity(WAITING_SEATS);
    seats.push(self_seat(room));
    for slot in room.slots.iter().take(WAITING_PEERS) {
        seats.push(peer_seat(slot));
    }
    while seats.len() < WAITING_SEATS {
        seats.push(empty_seat());
    }
    seats
}

/// One seat row: seat, team, name, flag (`R` ready, `-` waiting).
/// Vacant seats keep their index with a `---`/`LOCK` placeholder.
fn waiting_row_text(index: usize, seat: &WaitingSeat) -> String {
    match seat.kind {
        SeatKind::Occupied { ready } => {
            let flag = if ready { 'R' } else { '-' };
            format!(
                "{} {} {} {}",
                index,
                team_letter(seat.team),
                seat.name,
                flag
            )
        }
        SeatKind::Empty => format!("{index} - ---"),
        SeatKind::Locked => format!("{index} - LOCK"),
    }
}

/// Ready-dot ink: green when ready, dim otherwise.
fn ready_dot_color(ready: bool) -> u32 {
    if ready {
        pack_xrgb(0x30, 0xE0, 0x30)
    } else {
        pack_xrgb(0x50, 0x50, 0x50)
    }
}

/// Top-left of seat `index`: two columns by four rows, relative to frame.
#[must_use]
pub(crate) fn seat_origin(index: usize, w: usize, h: usize) -> (i32, i32) {
    let col = (index % 2) as i32;
    let row = (index / 2) as i32;
    (
        w as i32 / 8 + col * (w as i32 / 2),
        h as i32 / 4 + row * WAITING_ROW_H,
    )
}

/// Seat rows plus ready dots (no background fill: shared by the flat
/// fallback and the art scene).
fn paint_waiting_rows(frame: &mut Frame<'_>, room: &JoinedRoom) {
    for (index, seat) in waiting_seats(room).iter().enumerate() {
        paint_waiting_seat(frame, index, seat);
    }
}

/// Paint the eight waiting-room seats with names and ready flags.
pub(crate) fn paint_waiting_members(frame: &mut Frame<'_>, room: &JoinedRoom) {
    fill_background(frame, Screen::Waiting);
    paint_title(frame, "WAITING");
    paint_waiting_rows(frame, room);
}

/// Live caption sources for the staged waiting rects. Every field
/// has a pinned shell source (`docs/caption-sources.md`): the map
/// name rides the room word, the BPM rides the staged catalog
/// column for the room's song, and the counts ride the staged
/// inventory. `Default` is the no-source state (mismatch `-0-`,
/// zeroed counts, no title) — the exe paints exactly this without
/// a staged record.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WaitingCaptions<'a> {
    /// Staged catalog title for the room's song, if it names one.
    pub song_title: Option<&'a str>,
    /// Staged catalog BPM of the room's song, if it names one.
    pub song_bpm: Option<f32>,
    /// Staged inventory counts, one per skill slot.
    pub skill_counts: [u32; 3],
}

/// Paint staged waiting art (team buttons, avatar covers) with the
/// live seats, ready dots, and pinned captions on top: staged
/// placements anchor every region (covers, team grid, READY, skills,
/// counts, map, BPM, speed, music, wins), while the flat fallback
/// below keeps the invented grid. Without staged cover placements
/// the legacy rows keep their slot so the bare scene still reads.
pub fn paint_waiting_scene(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    art: &SceneArt,
    captions: &WaitingCaptions<'_>,
) {
    art.paint_backdrop(frame, Screen::Waiting);
    paint_title_over_art(frame, Screen::Waiting);
    if waiting_has_staged_covers(&art.placements) {
        paint_waiting_staged(frame, room, &art.placements, captions);
    } else {
        paint_waiting_rows(frame, room);
    }
}

/// Seat-cover control ids in seat order (`COVERAVATAR1..8`, codes
/// `0x1650003D..44` → `.bnd` rects 60..67): each cover rect positions
/// its own seat, anchor-free bodies included (the placement lesson).
const WAITING_COVER_IDS: [&str; 8] = [
    "IDC_IMAGE_SWAIT_COVERAVATAR1",
    "IDC_IMAGE_SWAIT_COVERAVATAR2",
    "IDC_IMAGE_SWAIT_COVERAVATAR3",
    "IDC_IMAGE_SWAIT_COVERAVATAR4",
    "IDC_IMAGE_SWAIT_COVERAVATAR5",
    "IDC_IMAGE_SWAIT_COVERAVATAR6",
    "IDC_IMAGE_SWAIT_COVERAVATAR7",
    "IDC_IMAGE_SWAIT_COVERAVATAR8",
];
/// Team-button control ids in team order (`TEAMA..H`, codes
/// `0x16201004..0B` → `.bnd` rects 3..10).
const WAITING_TEAM_IDS: [&str; 8] = [
    "IDC_BUTTON_SWAIT_TEAMA",
    "IDC_BUTTON_SWAIT_TEAMB",
    "IDC_BUTTON_SWAIT_TEAMC",
    "IDC_BUTTON_SWAIT_TEAMD",
    "IDC_BUTTON_SWAIT_TEAME",
    "IDC_BUTTON_SWAIT_TEAMF",
    "IDC_BUTTON_SWAIT_TEAMG",
    "IDC_BUTTON_SWAIT_TEAMH",
];
/// Room-title text (`TEXT_SWAIT_ROOMNAME`, code `0x16400016` → rect 21).
const WAITING_ROOMNAME_ID: &str = "IDC_TEXT_SWAIT_ROOMNAME";
/// Selected-song text (`TEXT_SWAIT_MUSICNAME`, code `0x16400017` →
/// rect 22, top bar beside the title).
const WAITING_MUSICNAME_ID: &str = "IDC_TEXT_SWAIT_MUSICNAME";
/// Champion win-count text (`TEXT_SWAIT_WINNERSCROE`, code `0x1640006D`
/// → rect 108, the Wins box inside the champion card).
const WAITING_WINNERSCORE_ID: &str = "IDC_TEXT_SWAIT_WINNERSCROE";
/// Scroll-speed value art (`SPEED_NUM`, code `0x16500066` → rect 101).
const WAITING_SPEED_NUM_ID: &str = "IDC_IMAGE_SWAIT_SPEED_NUM";
/// Skill-slot art ids top to bottom (`1/2/3SKILL`, codes
/// `0x16500058..5A` → rects 87..89, the X boxes of the SKILL panel).
const WAITING_SKILL_IDS: [&str; 3] = [
    "IDC_IMAGE_SWAIT_1SKILL",
    "IDC_IMAGE_SWAIT_2SKILL",
    "IDC_IMAGE_SWAIT_3SKILL",
];
/// Map-name text (`TEXT_SWAIT_MAPNAME`, code `0x16400018` →
/// rect 23: `(code & 0xFFF) − 1`, beside the title).
const WAITING_MAPNAME_ID: &str = "IDC_TEXT_SWAIT_MAPNAME";
/// BPM text (`TEXT_SWAIT_BPM`, code `0x16400069` → rect 104).
const WAITING_BPM_ID: &str = "IDC_TEXT_SWAIT_BPM";
/// Skill-count texts (`TEXT_SWAIT_1/2/3SKILLCOUNT`, codes
/// `0x1640005E..60` → rects 93..95, beside the X boxes).
const WAITING_SKILLCOUNT_IDS: [&str; 3] = [
    "IDC_TEXT_SWAIT_1SKILLCOUNT",
    "IDC_TEXT_SWAIT_2SKILLCOUNT",
    "IDC_TEXT_SWAIT_3SKILLCOUNT",
];

/// One staged seat view: the wire member plus the champion mark; the
/// self seat carries no level (the wire sends no self nickname or
/// level — `self_seat` documents the local-only name).
struct StagedSeat {
    name: String,
    level: Option<i32>,
    team: u8,
    ready: bool,
    present: bool,
    locked: bool,
    champion: bool,
}

/// Staged seats in cover order: self first, then the seven wire slots
/// (`JoinedRoom.slots`), padded empty — the same eight seats as
/// [`waiting_seats`], with levels and the champion mark kept.
fn staged_seats(room: &JoinedRoom) -> Vec<StagedSeat> {
    let mut seats = Vec::with_capacity(WAITING_SEATS);
    seats.push(StagedSeat {
        name: "YOU".to_owned(),
        level: None,
        team: room.team,
        ready: false,
        present: true,
        locked: false,
        champion: room.champion == Some(0),
    });
    for (offset, slot) in room.slots.iter().take(WAITING_PEERS).enumerate() {
        let index = u8::try_from(offset + 1).unwrap_or(u8::MAX);
        seats.push(staged_peer_seat(slot, room.champion == Some(index)));
    }
    while seats.len() < WAITING_SEATS {
        seats.push(empty_staged_seat());
    }
    seats
}

/// Staged view of one wire slot.
fn staged_peer_seat(slot: &RoomSlot, champion: bool) -> StagedSeat {
    match &slot.member {
        Some(member) => StagedSeat {
            name: member.nickname.clone(),
            level: Some(member.level),
            team: member.team,
            ready: member.ready,
            present: true,
            locked: false,
            champion,
        },
        None if slot.state_code == RoomSlotState::Locked as i32 => StagedSeat {
            name: String::new(),
            level: None,
            team: 0,
            ready: false,
            present: false,
            locked: true,
            champion: false,
        },
        None => empty_staged_seat(),
    }
}

/// Vacant staged seat.
fn empty_staged_seat() -> StagedSeat {
    StagedSeat {
        name: String::new(),
        level: None,
        team: 0,
        ready: false,
        present: false,
        locked: false,
        champion: false,
    }
}

/// Role prefix of one seat: the champion seat names `CHAMPION`, every
/// other occupied seat `CHALLENGER` (the reference 1v1 pair); rooms
/// without a champion mark name no roles (normal rooms carry none).
fn staged_role(has_champion: bool, seat: &StagedSeat) -> Option<&'static str> {
    if !has_champion || !seat.present {
        return None;
    }
    Some(if seat.champion {
        "CHAMPION"
    } else {
        "CHALLENGER"
    })
}

/// One staged seat line: role plus team plus `Lv` (unknown levels
/// show `-`: the self seat carries no wire level) plus name plus the
/// ready flag — the legacy row shape, role-prefixed.
fn staged_seat_text(seat: &StagedSeat, role: Option<&str>) -> String {
    let flag = if seat.ready { 'R' } else { '-' };
    let level = seat.level.map_or("-".to_owned(), |level| level.to_string());
    let team = team_letter(seat.team);
    match role {
        Some(role) => format!("{role} {team} Lv:{level} {} {flag}", seat.name),
        None => format!("{team} Lv:{level} {} {flag}", seat.name),
    }
}

/// Scaled frame origin of one waiting control (`None` while the staged
/// art names no such control — the exe leaves untouched icons alone).
fn waiting_bnd_origin(
    placements: &[ScenePlacement],
    id: &str,
    frame: &Frame<'_>,
) -> Option<(i32, i32)> {
    let placement = placements.iter().find(|placed| placed.id == id)?;
    let scaled = scale_rect_to_frame(&placement.rect, frame.w, frame.h);
    Some((scaled.x, scaled.y))
}

/// True while one cover binds the staged cover body (or a
/// sprite-less slot, as in synthetic probes): any other body on the
/// cover id stays untouched, like the exe's unmatched icons.
fn cover_stages_body(placements: &[ScenePlacement], cover_id: &str) -> bool {
    placements
        .iter()
        .find(|placed| placed.id == cover_id)
        .is_none_or(|placed| {
            placed
                .sprite
                .as_deref()
                .is_none_or(|sprite| sprite == WAIT_COVER_SPRITE)
        })
}

/// True while staged art binds at least one seat cover: the staged
/// path anchors to verbatim `.bnd` rects, otherwise the legacy grid
/// keeps the bare scene readable.
fn waiting_has_staged_covers(placements: &[ScenePlacement]) -> bool {
    WAITING_COVER_IDS
        .iter()
        .any(|id| placements.iter().any(|placed| &placed.id == id))
}

/// Staged waiting regions, each at its verbatim `.bnd` rect: seat
/// covers, team highlight, title, wins, speed, skills, skill counts,
/// map name, BPM, and song (the song caption resolves through the
/// staged catalog, `M{id}` while it names no such song; the BPM
/// caption resolves through its BPM column, `-0-` on mismatch).
/// `INSTNAME` (`TEXT_SWAIT_INSTNAME`, `0x16400019`) stays unpainted:
/// zero code immediates name it (OPEN 1), so no shell source reaches
/// its rect. `NOTICE_TEXT`/`NOTICEBOARD` (`0x16500062/63`) stay
/// unpainted: the text body is imageless and its dialog `+0x46c`
/// fill is untraced (OPEN 2).
fn paint_waiting_staged(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
    captions: &WaitingCaptions<'_>,
) {
    paint_waiting_title(frame, room, placements);
    paint_waiting_covers(frame, room, placements);
    paint_waiting_team_highlight(frame, room, placements);
    paint_waiting_wins(frame, room, placements);
    paint_waiting_speed(frame, room, placements);
    paint_waiting_skills(frame, room, placements);
    paint_waiting_skillcounts(frame, &captions.skill_counts, placements);
    paint_waiting_mapname(frame, room, placements);
    paint_waiting_bpm(frame, captions.song_bpm, placements);
    paint_waiting_music(frame, room, placements, captions.song_title);
}

/// Room title at the `ROOMNAME` rect (top bar, rect 21).
fn paint_waiting_title(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_ROOMNAME_ID, frame) {
        paint_room_detail_line(frame, x, y, &room.title, art_title_ink());
    }
}

/// Seat lines at the cover rects (rects 60..67): champion names its
/// role and wins box, the challenger its role; empty covers stay
/// untouched so the staged empty panels show (reference bottom row).
/// Locked covers name `LOCK` (their art carries no lock body).
fn paint_waiting_covers(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    let has_champion = room.champion.is_some();
    for (cover_id, seat) in WAITING_COVER_IDS.iter().zip(staged_seats(room)) {
        if !cover_stages_body(placements, cover_id) {
            continue;
        }
        let Some((x, y)) = waiting_bnd_origin(placements, cover_id, frame) else {
            continue;
        };
        if seat.locked {
            paint_room_detail_line(frame, x, y, "LOCK", art_title_ink());
        } else if seat.present {
            let role = staged_role(has_champion, &seat);
            paint_room_detail_line(frame, x, y, &staged_seat_text(&seat, role), art_title_ink());
            paint_cover_ready_dot(frame, x, y, seat.ready);
        }
    }
}

/// Ready dot beside one cover line (the `Wait_ReadyText.ojs` plate is
/// staged art; the dot is the live flag, same inks as the lobby).
fn paint_cover_ready_dot(frame: &mut Frame<'_>, x: i32, y: i32, ready: bool) {
    fill_rect(
        frame,
        &Rect {
            x: x - 6,
            y,
            w: 3,
            h: 5,
        },
        ready_dot_color(ready),
    );
}

/// Team highlight at our team button rect (rects 3..10): the exe
/// highlight arm (`FUN_0048c810`) lights exactly the pressed button;
/// the amber bar is the live mark over the staged `Wait_Team?.ojs`.
fn paint_waiting_team_highlight(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    let index = usize::from(room.team);
    let (Some(button), Some(expected)) =
        (WAITING_TEAM_IDS.get(index), WAIT_TEAM_SPRITES.get(index))
    else {
        return;
    };
    let staged = placements.iter().find(|placed| &placed.id == button);
    if staged.is_none_or(|placed| placed.sprite.as_deref() == Some(*expected)) {
        if let Some((x, y)) = waiting_bnd_origin(placements, button, frame) {
            paint_cursor(frame, x + 6, y);
        }
    }
}

/// Champion win count at the `WINNERSCORE` rect (rect 108, the Wins
/// box): the wire streak only (`JoinedRoom.champion_streak`); past
/// it nothing paints (per-member tallies stay server-side).
fn paint_waiting_wins(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    let Some(streak) = room.champion_streak else {
        return;
    };
    if room.champion.is_none() {
        return;
    }
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_WINNERSCORE_ID, frame) {
        paint_room_detail_line(frame, x, y, &streak.to_string(), art_title_ink());
    }
}

/// Scroll-speed label at the `SPEED_NUM` rect (rect 101): the shared
/// `X0.5`..`XR` chain (`room-enums.md`, codes 0..11).
fn paint_waiting_speed(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_SPEED_NUM_ID, frame) {
        paint_room_detail_line(frame, x, y, &room_speed_label(room.speed), art_title_ink());
    }
}

/// Skill-ring ids at the `1/2/3SKILL` rects (rects 87..89, the X
/// boxes): empty slots stay untouched so the staged X art shows.
/// The counts paint beside them at the `SKILLCOUNT` rects from the
/// staged inventory ([`paint_waiting_skillcounts`]).
fn paint_waiting_skills(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    for (slot, skill_id) in WAITING_SKILL_IDS.iter().zip(room.skills.iter()) {
        if let Some((x, y)) = waiting_bnd_origin(placements, slot, frame) {
            paint_room_detail_line(frame, x, y, &skill_id.to_string(), art_title_ink());
        }
    }
}

/// Map-cycle names behind the `MAPNAME` caption, in room-word order
/// (map-cycle path, rdata file `0x249990`): the exe centers entry
/// `low == 0 ? 3 : low % 4` of (`None`, `한가한대전장의오후`,
/// `패자의역습`, `원하지않은결말`). The CP949-sourced Korean entries
/// paint their real glyphs now that the embedded Noto Sans KR subset
/// carries Hangul (formerly ASCII stand-ins — the micro-font had no
/// CP949 glyphs). The `0x4ad350` applier `Random`/`NoteFactory` arm
/// stays OPEN (`docs/caption-sources.md` OPEN 3) and never paints.
const WAITING_MAP_CYCLE_NAMES: [&str; 4] =
    ["None", "한가한대전장의오후", "패자의역습", "원하지않은결말"];

/// Map-cycle slot for one room word (files `0xb3cc6`–`0xb3ce3`):
/// the low word, with word 0 selecting slot 3. Byte access keeps
/// the low word without a narrowing cast (PERF-07).
#[must_use]
fn waiting_map_cycle_slot(arena: i32) -> usize {
    let bytes = arena.to_le_bytes();
    let low = u16::from_le_bytes([bytes[0], bytes[1]]);
    if low == 0 {
        WAITING_MAP_CYCLE_NAMES.len() - 1
    } else {
        usize::from(low) % WAITING_MAP_CYCLE_NAMES.len()
    }
}

/// `MAPNAME` caption for one room word: the map-cycle table entry
/// (pinned path only).
#[must_use]
pub fn waiting_map_name(arena: i32) -> &'static str {
    WAITING_MAP_CYCLE_NAMES
        .get(waiting_map_cycle_slot(arena))
        .copied()
        .unwrap_or(WAITING_MAP_CYCLE_NAMES[0])
}

/// Map name at the `MAPNAME` rect: the map-cycle entry of the room
/// word. An unstaged rect stays untouched.
fn paint_waiting_mapname(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_MAPNAME_ID, frame) {
        paint_room_detail_line(frame, x, y, waiting_map_name(room.arena), art_title_ink());
    }
}

/// Mismatch BPM caption (rdata file `0x24a1d0`): the exe stores
/// `-0-` when no song record matches the room song.
const WAITING_BPM_MISMATCH: &str = "-0-";

/// `BPM` caption for one staged catalog lookup: the `OJNList.dat`
/// row +20 BPM column formatted with `%0.f` (rdata file `0x24a1d4`,
/// store file `0xbe1d5`); a missing or non-finite lookup falls back
/// to the pinned mismatch default, never an invented tempo.
#[must_use]
pub fn waiting_bpm_caption(song_bpm: Option<f32>) -> String {
    match song_bpm {
        Some(bpm) if bpm.is_finite() => format!("{bpm:.0}"),
        _ => WAITING_BPM_MISMATCH.to_owned(),
    }
}

/// BPM at the `BPM` rect: the catalog column or `-0-`. An unstaged
/// rect stays untouched.
fn paint_waiting_bpm(frame: &mut Frame<'_>, song_bpm: Option<f32>, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_BPM_ID, frame) {
        paint_room_detail_line(frame, x, y, &waiting_bpm_caption(song_bpm), art_title_ink());
    }
}

/// `SKILLCOUNT` caption for one inventory slot: the count word at
/// owned-item vector `+8` as decimal, space-padded to width 3
/// (helper `0x62c109`); slots 2–3 additionally clamp at 999, so
/// wider counts show `999` there.
#[must_use]
pub fn waiting_skillcount_text(count: u32, cap_at_999: bool) -> String {
    let shown = if cap_at_999 && count > 999 {
        999
    } else {
        count
    };
    format!("{shown:>3}")
}

/// Skill counts at the `1/2/3SKILLCOUNT` rects: the staged inventory
/// counts beside the rings (slot 1 uncapped, slots 2–3 capped at
/// 999). An unstaged rect stays untouched.
fn paint_waiting_skillcounts(
    frame: &mut Frame<'_>,
    counts: &[u32; 3],
    placements: &[ScenePlacement],
) {
    for (slot, (id, count)) in WAITING_SKILLCOUNT_IDS.iter().zip(counts.iter()).enumerate() {
        if let Some((x, y)) = waiting_bnd_origin(placements, id, frame) {
            paint_room_detail_line(
                frame,
                x,
                y,
                &waiting_skillcount_text(*count, slot > 0),
                art_title_ink(),
            );
        }
    }
}

/// Selected-song label at the `MUSICNAME` rect (rect 22, top bar):
/// the staged catalog title for the room's song when it names one
/// (room-fill string flow, `docs/channel-fill.md`), else the bare
/// song id (the song table lives server-side, as in the lobby).
/// The `COVERMUSIC` button, the select arrow, the `Random` cards, and
/// BACK stay staged art — recorded negatives, not gaps (chat now has a
/// live source: [`paint_waiting_chat`]):
///
/// - Random cards: the 117-entry `STATE_WAITING` block names no per-card
///   entries; the big `?` card sits on `RANDMAP` (`0x1650004F` → rect 78)
///   while the small `?` cards sit on the spriteless `EFFECT_1/2` rects
///   (68/69) — caption and frames baked into the staged backdrop.
///   `SelectMusic_Random*.ojs` bodies belong to `DAILOG_SELECTMUSIC`
///   filter buttons, not to waiting cards, and no wire struct carries
///   card picks — so the painter stages art only.
fn paint_waiting_music(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
    song_title: Option<&str>,
) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_MUSICNAME_ID, frame) {
        paint_room_detail_line(
            frame,
            x,
            y,
            &waiting_music_caption(room.music_id, song_title),
            art_title_ink(),
        );
    }
}

/// `MUSICNAME` caption for one room song: the staged catalog title
/// when it names the song, else the bare `M{id}` id (never an
/// invented title — `INSTNAME` stays unpainted for the same reason:
/// no staged source names it).
#[must_use]
fn waiting_music_caption(music_id: u16, song_title: Option<&str>) -> String {
    match song_title {
        Some(title) if !title.is_empty() => title.to_owned(),
        _ => room_music_label(music_id),
    }
}

/// Chat history view (`EDIT_SWAIT_CHAT 0x16300014` → rect 19,
/// `LIST_SWAIT_CHAT 0x1680002B` → rect 42, row template
/// `TEXT_SWAIT_CHAT 0x1640003C` under `SET 0x00031000`): one live row
/// per decoded 3012/3013 packet, fed through [`paint_waiting_chat`].
/// Rows come from live packets only — no history is invented.
const WAITING_CHAT_LIST_ID: &str = "IDC_LIST_SWAIT_CHAT";
/// Chat input line (`EDIT_SWAIT_CHAT` → rect 19).
const WAITING_CHAT_EDIT_ID: &str = "IDC_EDIT_SWAIT_CHAT";
/// Native history rect while no staged art names it
/// (`LIST_SWAIT_CHAT` rect 42 `(24,455,511,553)`).
const WAITING_CHAT_LIST_RECT: Rect = Rect {
    x: 24,
    y: 455,
    w: 487,
    h: 98,
};
/// Native input rect while no staged art names it
/// (`EDIT_SWAIT_CHAT` rect 19 `(20,563,460,583)`).
const WAITING_CHAT_EDIT_RECT: Rect = Rect {
    x: 20,
    y: 563,
    w: 440,
    h: 20,
};
/// Row pitch of the history box: the reference rows sit 17 native px
/// apart (row tops at native y 455.4/472.5/489.8 in
/// `reference-waiting-4x3.png`), and six 17 px slots fill the 98 px
/// `LIST` rect exactly. The `TEXT_SWAIT_CHAT` template carries no
/// per-row staged rect — rows stack from the list origin, so the
/// exact row-feed call stays OPEN while the pitch is photo-pinned.
const WAITING_CHAT_ROW_H: i32 = 17;
/// Rows visible in the history box: the [`WAITING_CHAT_ROW_H`] slots
/// that fit the 98 px `LIST` rect.
const WAITING_CHAT_VISIBLE_ROWS: usize = 6;
/// Focus-caret inset inside the input rect: the reference shows the
/// caret bar starting at native (25, 568) inside `EDIT_SWAIT_CHAT`
/// rect 19 `(20,563,460,583)` (`reference-waiting-4x3.png`, empty
/// input).
const WAITING_CHAT_CARET_INSET: (i32, i32) = (5, 5);
/// Focus-caret bar size: a 2x10 white bar in the same photo.
const WAITING_CHAT_CARET_SIZE: (u32, u32) = (2, 10);

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
fn chat_user_ink() -> u32 {
    pack_xrgb(0xDE, 0xDE, 0xDE)
}

/// Ink of 3013 administrator rows: color `0x71e6` (call file
/// `0xb9fbf`) as a COLORREF — red `0xe6`, green `0x71`, blue `0x00`,
/// the warm amber every reference row reads as.
fn chat_admin_ink() -> u32 {
    pack_xrgb(0xE6, 0x71, 0x00)
}

/// Ink of one history row: the 3013 gate picks the admin amber,
/// every other row the user gray.
fn chat_row_ink(admin: bool) -> u32 {
    if admin {
        chat_admin_ink()
    } else {
        chat_user_ink()
    }
}

/// Frame y of history row `offset`: the list origin plus
/// [`WAITING_CHAT_ROW_H`] steps (photo-pinned pitch).
#[must_use]
fn waiting_chat_row_y(list_y: i32, offset: usize) -> i32 {
    list_y.saturating_add(
        i32::try_from(offset)
            .unwrap_or(i32::MAX)
            .saturating_mul(WAITING_CHAT_ROW_H),
    )
}

/// Frame origin of the history box: the staged `LIST` rect while bound,
/// else the scaled native rect 42.
fn waiting_chat_list_origin(frame: &Frame<'_>, placements: &[ScenePlacement]) -> (i32, i32) {
    if let Some(origin) = waiting_bnd_origin(placements, WAITING_CHAT_LIST_ID, frame) {
        return origin;
    }
    let scaled = scale_rect_to_frame(&WAITING_CHAT_LIST_RECT, frame.w, frame.h);
    (scaled.x, scaled.y)
}

/// Frame origin of the input line: the staged `EDIT` rect while bound,
/// else the scaled native rect 19.
fn waiting_chat_edit_origin(frame: &Frame<'_>, placements: &[ScenePlacement]) -> (i32, i32) {
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
const CHANNEL_CHAT_VISIBLE_ROWS: usize = 6;
/// Channel chat rows share the lobby list pitch (8 px).
const CHANNEL_CHAT_ROW_H: i32 = 8;

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
        }
    }

    /// Rank-notice row (2029): the ranked nickname bracketed, song
    /// id plus rank as the text, in the admin amber (the same `0x7de`
    /// family as the announcement rows).
    #[must_use]
    pub fn rank(nickname: &str, music_id: i32, ranking: i32) -> Self {
        Self {
            sender: nickname.to_owned(),
            text: rank_notice_text(music_id, ranking),
            admin: true,
            whisper_recipient: None,
        }
    }
}

/// Display text of one rank notice (2029): the song id in the lobby
/// `M{id}` shape plus the raw rank value (`M757 1`). The exe stages
/// music id, nickname, and ranking in `.data` slots and forwards a
/// UI-queue event (`0, 0, 0x7ed, 0x12`) rather than a bracket line
/// (`docs/net-protocol.md`), so the exact notice-board wording stays
/// OPEN — this names all three wire fields, joined with no invention
/// beyond the join.
#[must_use]
pub fn rank_notice_text(music_id: i32, ranking: i32) -> String {
    format!("M{music_id} {ranking}")
}

/// Display text of one channel row: the waiting `[%12s] %s` bracket
/// line for channel rows; whisper rows name both ends as
/// `[sender -> recipient] text` so the addressee stays visible;
/// sender-less announcement rows paint their text bare (no bracket
/// without a sender).
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
            chat_row_ink(message.admin),
        );
    }
}

/// Dark strip behind one seat row so seat ink stays readable over
/// staged art (vacant rows get one too: the placeholder must read).
fn paint_seat_scrim(frame: &mut Frame<'_>, x: i32, y: i32, text: &str) {
    let text_w = text_width(text) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 8,
            y: y - 1,
            w: (text_w + 12).max(0) as u32,
            h: 7,
        },
        scrim_color(),
    );
}

/// Paint one seat row plus its ready dot (vacant seats have no dot).
fn paint_waiting_seat(frame: &mut Frame<'_>, index: usize, seat: &WaitingSeat) {
    let occupied = seat.display_name().is_some();
    let (x, y) = seat_origin(index, frame.w, frame.h);
    let row = waiting_row_text(index, seat);
    paint_seat_scrim(frame, x, y, &row);
    let ink = if occupied {
        pack_xrgb(0xFF, 0xFF, 0xFF)
    } else {
        pack_xrgb(0x70, 0x70, 0x70)
    };
    draw_text(frame, x, y, &row, ink);
    if occupied {
        fill_rect(
            frame,
            &Rect {
                x: x - 6,
                y,
                w: 3,
                h: 5,
            },
            ready_dot_color(seat.is_ready()),
        );
    }
}

/// Title tucked just inside the card's top edge.
fn paint_title(frame: &mut Frame<'_>, title: &str) {
    if title.is_empty() {
        return;
    }
    let (x, y) = title_origin(frame, title);
    draw_text(frame, x, y, title, title_color());
}

/// Staged lobby art behind an empty list: verbatim backdrop plus
/// a readable title (no flat card).
pub fn paint_lobby_backdrop(frame: &mut Frame<'_>, screen: Screen, art: &SceneArt) {
    art.paint_backdrop(frame, screen);
    paint_title_over_art(frame, screen);
}

/// Client version stamped on the splash (matches the 7.55 target).
const SPLASH_VERSION: u32 = 755;
/// Splash version color (white on the logo card).
fn version_color() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Paint the splash (logo) scene.
fn paint_avi(frame: &mut Frame<'_>) {
    paint_card(
        frame,
        pack_xrgb(0x08, 0x08, 0x10),
        pack_xrgb(0x20, 0x60, 0xC0),
    );
    let text_w = number_width(SPLASH_VERSION) as i32;
    draw_number(
        frame,
        frame.w as i32 / 2 - text_w / 2,
        frame.h as i32 / 2 - 2,
        SPLASH_VERSION,
        version_color(),
    );
}

/// Paint staged splash art (`O2mediaLogo.ojs`): the real logo
/// replaces the card plus version stamp.
pub fn paint_avi_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Avi);
}

/// Paint a lobby scene (channel / room / waiting share the shape).
fn paint_lobby(frame: &mut Frame<'_>, background: u32) {
    paint_card(frame, background, pack_xrgb(0xD0, 0xD0, 0xD0));
}

/// Loading gate state staged while the song table walks (exe
/// `0x4e47a0`, file `0xe47a0`, ev `68d920c2…`): the song-table
/// fraction plus the song title/artist being loaded. The exe sets
/// `Trebuchet MS`/`Arial` fonts there; the shell micro-font stands
/// in (typography gap). Track and caption rects below stay OPEN
/// placeholders: `MusicLoading.ojs` is one 800x600 body at (0,0)
/// with no bar/caption sub-rect, the `0x4e47a0` dossier carries
/// fonts plus the overlay mount plus the song-table walk with no
/// literal bar/caption rect, and adjacent bar bodies belong to
/// other dialogs — so only the fraction and the caption are live.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LoadingStatus {
    /// Song-table walk fraction (`0.0`..=`1.0`, clamped at paint).
    pub progress: f32,
    /// Song title being loaded (OJN header / `OJNList.dat` record).
    pub title: String,
    /// Song artist being loaded.
    pub artist: String,
}

impl LoadingStatus {
    /// Walk fraction clamped to `0.0`..=`1.0` (NaN saturates to
    /// `0.0`): never `==`-compares floats (PERF-25).
    #[must_use]
    pub fn clamped_progress(&self) -> f32 {
        if self.progress.is_nan() {
            0.0
        } else {
            self.progress.clamp(0.0, 1.0)
        }
    }

    /// `TITLE - ARTIST` caption (`None` while both are blank:
    /// nothing paints rather than a wrong line).
    #[must_use]
    pub fn caption(&self) -> Option<String> {
        let title = self.title.trim();
        let artist = self.artist.trim();
        match (title.is_empty(), artist.is_empty()) {
            (true, true) => None,
            (false, true) => Some(title.to_owned()),
            (true, false) => Some(artist.to_owned()),
            (false, false) => Some(format!("{title} - {artist}")),
        }
    }
}

/// Width of the live fill inside a `track_w` track at `progress`:
/// clamped, then narrowed via `TryFrom`, never `as` (PERF-07).
#[must_use]
fn loading_fill_width(track_w: u32, progress: f32) -> u32 {
    let clamped = if progress.is_nan() {
        0.0
    } else {
        progress.clamp(0.0, 1.0)
    };
    let scaled = f64::from(track_w) * f64::from(clamped);
    let rounded = scaled.round() as u64;
    u32::try_from(rounded).unwrap_or(u32::MAX).min(track_w)
}

/// Track origin of the OPEN placeholder bar: centered `w / 4,
/// h / 2 - 8` with no exe-grounded rect (`MusicLoading.ojs` is a
/// single 800x600 body, `0x4e47a0` names no rect, adjacent bar art
/// belongs to other dialogs; bnd-audit 35).
fn loading_track_origin(frame: &Frame<'_>) -> (i32, i32) {
    (frame.w as i32 / 4, frame.h as i32 / 2 - 8)
}

/// Inner width of the OPEN placeholder track on this frame
/// (`w / 2`: centered placeholder, no exe-grounded width).
fn loading_track_width(frame: &Frame<'_>) -> u32 {
    u32::try_from(frame.w as i32 / 2).unwrap_or(0)
}

/// Song-caption origin: one row below the OPEN placeholder bar
/// (`w / 4, h / 2 + 12` — no verified caption rect).
fn loading_song_origin(frame: &Frame<'_>) -> (i32, i32) {
    (frame.w as i32 / 4, frame.h as i32 / 2 + 12)
}

/// Loading progress bar over a dark track (no background fill:
/// shared by the flat fallback and the art scene). The track rect
/// is an OPEN placeholder; only the fill fraction is live.
fn paint_loading_bar(frame: &mut Frame<'_>) {
    paint_loading_bar_with_progress(frame, 1.0);
}

/// Loading bar with the live song-table walk fraction: the track
/// stays put, the green fill spans `progress` of it.
fn paint_loading_bar_with_progress(frame: &mut Frame<'_>, progress: f32) {
    let (track_x, track_y) = loading_track_origin(frame);
    let track_w = loading_track_width(frame);
    fill_rect(
        frame,
        &Rect {
            x: track_x - 2,
            y: track_y - 2,
            w: track_w.saturating_add(4),
            h: 20,
        },
        scrim_color(),
    );
    let fill_w = loading_fill_width(track_w, progress);
    if fill_w > 0 {
        fill_rect(
            frame,
            &Rect {
                x: track_x,
                y: track_y,
                w: fill_w,
                h: 16,
            },
            pack_xrgb(0x30, 0xB0, 0x30),
        );
    }
}

/// Song title/artist line below the bar (OPEN placeholder row):
/// dark scrim plus white ink, like the lobby detail lines. Blank
/// captions paint nothing.
fn paint_loading_song(frame: &mut Frame<'_>, status: &LoadingStatus) {
    let Some(caption) = status.caption() else {
        return;
    };
    let (x, y) = loading_song_origin(frame);
    paint_room_detail_line(frame, x, y, &caption, art_title_ink());
}

/// Paint the loading gate scene.
fn paint_loading(frame: &mut Frame<'_>) {
    fill_background(frame, Screen::Loading);
    paint_loading_bar(frame);
}

/// Paint the loading gate with its live state: staged genuine
/// `MusicLoading.ojs` art at its authored (record-anchor) geometry
/// first, then the fractional bar plus the song title/artist.
/// Unpainted while the shell carries no walk state: the exe
/// Trebuchet/Arial fonts and the OJN/Vorbis path stay gaps.
pub fn paint_loading_scene_with_status(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    status: &LoadingStatus,
) {
    art.paint_backdrop(frame, Screen::Loading);
    paint_loading_bar_with_progress(frame, status.clamped_progress());
    paint_loading_song(frame, status);
}

/// Paint staged loading art (`MusicLoading.ojs`) with the live
/// progress bar on top. Legacy indeterminate form (full bar, no
/// caption): `game.rs` carries no walk fraction or song names yet,
/// so callers with live state use [`paint_loading_scene_with_status`].
pub fn paint_loading_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    paint_loading_scene_with_status(
        frame,
        art,
        &LoadingStatus {
            progress: 1.0,
            title: String::new(),
            artist: String::new(),
        },
    );
}

/// Battle verdict art mounted by the result builder `FUN_004c2390`
/// (file `0xc2390`): `StateResult_Top.ojs` plus
/// `StateResult_Bottom.ojs` plus `Result_WinLose.ojs` (file `0x24a35c`).
/// The play-scene winner banner `Winner_image.ojs` (file `0x24b014`)
/// names the same verdict; the result scene surfaces both verdict arts.
/// Winner is rank 1 by team score only (`RoomTeams::winner`); team
/// totals stay server-side (per-member 4014/4016). The grade plate
/// runs on the pinned threshold table ([`grade_frame`]).
pub const RESULT_TOP_SPRITE: &str = "StateResult_Top.ojs";
/// Bottom half mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_BOTTOM_SPRITE: &str = "StateResult_Bottom.ojs";
/// Win/lose verdict mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_WINLOSE_SPRITE: &str = "Result_WinLose.ojs";
/// Winner banner from `STATE_PLAYING` (file `0x24b014`), surfaced on
/// the result scene as the same rank-1 verdict.
pub const WINNER_IMAGE_SPRITE: &str = "Winner_image.ojs";
/// Grade plate bound by `STATE_RESULT` rect 7
/// (`IDC_IMAGE_SRESULT_RANKINGFIELD`, `Result_Imgs_Rank.ojs`): eight
/// frames, all anchored (395,531) — 35×41 ×2, 39×41 ×4, 38×41, 79×41
/// (dossier `ev_c61d99f8…`). Frame *n* paints image *n−1*, so grade
/// 1..=8 shows images 0..=7.
pub const RESULT_RANK_SPRITE: &str = "Result_Imgs_Rank.ojs";

/// Music-name box interior on the native canvas: the
/// `Result_EachResult.ojs` body (`STATE_RESULT` rect 4
/// `IDC_IMAGE_SRESULT_RESULTTAP`, 800×350 anchored (0,250)) bakes a
/// default mascot pane into its inner box, and the played song's
/// album art paints over exactly that interior — the reference photo's
/// green panel measures this rect (body-local interior (70,54)–
/// (733,268) plus the (0,250) anchor; the baked frame borders at
/// x 68–69/734–735 and y 52–53/269–270 stay art). The cover section
/// comes from the walked chart's `.ojn` (docs/file-formats/ojn.md); a
/// missing cover leaves the baked box as-is.
pub const RESULT_MUSIC_BOX: Rect = Rect {
    x: 70,
    y: 304,
    w: 664,
    h: 215,
};

/// Sprites whose frames the result painter selects one of (the
/// verdict banner plus the grade plate): kept out of the whole-stack
/// backdrop pass so unused frames never bleed through, then painted
/// at the exe-selected frame only.
const RESULT_FRAME_BODIES: [&str; 2] = [RESULT_WINLOSE_SPRITE, RESULT_RANK_SPRITE];

/// Sprites the result builder `FUN_004c2390` (file `0xc2390`) mounts,
/// in load order: top, bottom, then the win/lose verdict.
#[must_use]
pub fn result_loader_sprites() -> [&'static str; 3] {
    [
        RESULT_TOP_SPRITE,
        RESULT_BOTTOM_SPRITE,
        RESULT_WINLOSE_SPRITE,
    ]
}

/// Battle verdict: team standing by server-side total (`RoomTeams`
/// mirrors the 4014/4016 accumulation; `Winner_image.ojs` and
/// `Result_WinLose.ojs` render this same comparison server-side).
/// Solo or team, grades play no part (mapping OPEN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleVerdict {
    /// Local team holds rank 1 (exe win-flag 1 → frame 1 → image 0).
    Win,
    /// Local team holds rank 2 (exe win-flag 2 → frame 2 → image 1,
    /// the LOSE! banner of the reference battle photo).
    Lose,
    /// Local team ranks third or worse (exe win-flag anything else
    /// → frame 3 → image 2).
    Other,
}

/// Distinct seated teams in seat order (each `TEAMx` button index once).
#[must_use]
fn seated_battle_teams(teams: &RoomTeams) -> Vec<u8> {
    let mut seen = Vec::new();
    for seat in 0..teams.seats() {
        if let Some(team) = teams.team_of(seat) {
            if !seen.contains(&team) {
                seen.push(team);
            }
        }
    }
    seen
}

/// Zero-based standing of `team` among the seated teams: highest
/// combined total first, ties toward the lower team id (the
/// `RoomTeams::winner` first-seat-wins order). `None` while `team`
/// holds no seat.
#[must_use]
fn battle_team_rank(teams: &RoomTeams, team: u8) -> Option<usize> {
    let mut standings: Vec<(u64, u8)> = seated_battle_teams(teams)
        .into_iter()
        .map(|candidate| (teams.team_total(candidate), candidate))
        .collect();
    standings.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    standings.iter().position(|(_, seated)| *seated == team)
}

/// Standing verdict for the local team: rank 1 wins, rank 2 loses,
/// rank 3+ falls to the generic other arm. `None` while the local
/// team is unknown or unseated (empty room, solo with no seat claimed).
#[must_use]
pub fn battle_verdict(teams: &RoomTeams, self_team: Option<u8>) -> Option<BattleVerdict> {
    match battle_team_rank(teams, self_team?)? {
        0 => Some(BattleVerdict::Win),
        1 => Some(BattleVerdict::Lose),
        _ => Some(BattleVerdict::Other),
    }
}

/// Result verdict state the banner painter maps to a WinLose frame
/// (`FUN_004c3120`): solo plays take the solo banner, battles the
/// win-flag frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultVerdict {
    /// Solo play (the exe's player-count branch): frame 4, the solo
    /// banner image.
    Solo,
    /// Team battle with the resolved rank-1 verdict, if any (`None`
    /// while the winner or the local team is unknown — no banner,
    /// the exe's observer frame-0 arm being OPEN, dossier note 10).
    Battle(Option<BattleVerdict>),
}

/// WinLose frame for one verdict (`FUN_004c3120`, dossier
/// `ev_38f32576…`): solo draws frame 4; a battle win-flag byte of 1
/// draws frame 1, 2 draws frame 2, anything else frame 3. The shell
/// resolves the same standing from team totals (rank 1 → `Win`,
/// rank 2 → `Lose`, rank 3+ → `Other`), so every dossier frame is
/// reachable. Frame 4 clamps to the fourth image at draw time like
/// the exe setter.
#[must_use]
pub fn winlose_frame(verdict: ResultVerdict) -> Option<u8> {
    match verdict {
        ResultVerdict::Solo => Some(4),
        ResultVerdict::Battle(Some(BattleVerdict::Win)) => Some(1),
        ResultVerdict::Battle(Some(BattleVerdict::Lose)) => Some(2),
        ResultVerdict::Battle(Some(BattleVerdict::Other)) => Some(3),
        ResultVerdict::Battle(None) => None,
    }
}

/// Pinned grade thresholds (grade chooser `FUN_004c4dd0`, dossier
/// `ev_615e034c…`): score ≥ max × t lifts the plate to the paired
/// frame, compared in this order. The f64 constants sit at
/// `0x64a500/0x644888/0x64a4f8/0x64a4f0/0x64a4e8/0x64a4e0`.
const GRADE_THRESHOLDS: [(f64, u8); 6] = [
    (0.30, 1),
    (0.50, 2),
    (0.70, 3),
    (0.80, 4),
    (0.90, 5),
    (0.95, 6),
];

/// Grade plate frame 1..=8 for one finished play (`FUN_004c4dd0`):
/// `ratio` climbs the pinned threshold table; a max combo one under
/// the note count promotes grade 6 to 7; a score equal to the max
/// reference gives 8. The exe ratio reads score global `0x77ee60`
/// against reference `this+0x10d14` — neither settleable from the
/// pinned evidence (dossier OPEN 3/9), so callers feed the shell's
/// documented stand-in ratio and the plate stays one step behind the
/// open question.
#[must_use]
pub fn grade_frame(ratio: f64, combo_at_top: bool, score_at_max: bool) -> u8 {
    let mut grade = 0;
    for (threshold, frame) in GRADE_THRESHOLDS {
        if ratio >= threshold {
            grade = grade.max(frame);
        }
    }
    if grade == 6 && combo_at_top {
        grade = 7;
    }
    if score_at_max {
        grade = 8;
    }
    grade
}

/// White ink of the pinned result fields (the exe text helper runs
/// at color `0xffffff`, Arial).
fn result_number_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// True while `frame` is the exe's native canvas: the pinned result
/// anchors live on the 800×600 surface the exe ran on, and the
/// fixed-size glyph text must not scatter over smaller frames (the
/// window and shot pipeline compose at native and scale once at
/// present time).
#[must_use]
fn frame_is_native(frame: &Frame<'_>) -> bool {
    frame.w == usize::try_from(NATIVE_WIDTH).unwrap_or(0)
        && frame.h == usize::try_from(NATIVE_HEIGHT).unwrap_or(0)
}

/// Played points clamped into the exe's 32-bit score field (the
/// `%8d` read; our u64 accumulator saturates instead of wrapping).
#[must_use]
fn result_score_field(score: &o2jam_game::Score) -> u32 {
    u32::try_from(score.points).unwrap_or(u32::MAX)
}

/// Paint the pinned result numbers from the live `Score`
/// (docs/cresult-anchors.md, top-panel painter `FUN_004c38a0`, file
/// `0xc38a0`): COOL/GOOD/BAD/MISS at y=125, MAX/JAM COMBO at y=130,
/// total score at (605,533) — white GDI text under the baked
/// `StateResult_Top` headers, never digit sprites. Each field draws its
/// decimal from the anchor's left edge: every text call in the top
/// painter, the row painter (`FUN_004c3b40`, file `0xc3b40`), and the
/// width wrapper (`FUN_004c43d0`) passes identical trailing text-helper
/// args `(0, 3, 4)`, and the body of `FUN_0042c730` is not dumped, so
/// left-vs-right anchoring stays OPEN (dossier note 5) and the verified
/// passed x is kept as the left edge. JAM COMBO reads the jam level
/// chain (the wire's u16 `jam` field; mapping OPEN 3); non-native
/// frames paint nothing.
fn paint_result_numbers(frame: &mut Frame<'_>, score: &o2jam_game::Score) {
    if !frame_is_native(frame) {
        return;
    }
    let ink = result_number_ink();
    draw_number(frame, RESULT_COOL_X, RESULT_COUNT_Y, score.counts[0], ink);
    draw_number(frame, RESULT_GOOD_X, RESULT_COUNT_Y, score.counts[1], ink);
    draw_number(frame, RESULT_BAD_X, RESULT_COUNT_Y, score.counts[2], ink);
    draw_number(frame, RESULT_MISS_X, RESULT_COUNT_Y, score.counts[3], ink);
    draw_number(
        frame,
        RESULT_MAX_COMBO_X,
        RESULT_COMBO_Y,
        score.max_combo,
        ink,
    );
    draw_number(
        frame,
        RESULT_JAM_COMBO_X,
        RESULT_COMBO_Y,
        score.jam.level(),
        ink,
    );
    draw_number(
        frame,
        RESULT_SCORE_X,
        RESULT_SCORE_Y,
        result_score_field(score),
        ink,
    );
}

/// Pinned result number anchors (docs/cresult-anchors.md, top-panel
/// painter `FUN_004c38a0`): the judgement counts share row y=125,
/// the combo fields sit at y=130, and the total score reads `%8d`
/// at (605,533).
const RESULT_COOL_X: i32 = 151;
/// GOOD column x.
const RESULT_GOOD_X: i32 = 240;
/// BAD column x.
const RESULT_BAD_X: i32 = 337;
/// MISS column x.
const RESULT_MISS_X: i32 = 426;
/// Judgement-count row y.
const RESULT_COUNT_Y: i32 = 125;
/// MAX COMBO column x.
const RESULT_MAX_COMBO_X: i32 = 505;
/// JAM COMBO column x.
const RESULT_JAM_COMBO_X: i32 = 596;
/// Combo row y.
const RESULT_COMBO_Y: i32 = 130;
/// Total score column x.
const RESULT_SCORE_X: i32 = 605;
/// Total score row y.
const RESULT_SCORE_Y: i32 = 533;

/// Rank number column x (`%d` at x=91, drawn only with 2+ players).
const RESULT_RANK_X: i32 = 91;
/// Nickname column x (at 318+22·row, truncates over 100 chars).
const RESULT_ROW_NAME_X: i32 = 128;
/// Judgements column x (`%5d /%5d /%3d /%3d` at 317+22·row).
const RESULT_ROW_JUDGE_X: i32 = 230;
/// Combo-pair column x (`%4d /%4d`: MAX then JAM COMBO).
const RESULT_ROW_COMBO_X: i32 = 416;
/// Row score column x (`%8d`).
const RESULT_ROW_SCORE_X: i32 = 530;
/// Bonus column x (`+ %d`).
const RESULT_ROW_BONUS_X: i32 = 626;
/// Speed column x (`X 0.5`…`X R`).
const RESULT_ROW_SPEED_X: i32 = 685;
/// Row text y for every column but the nickname (317+22·row: only
/// the judgement and nickname heights are pinned, so the other
/// columns share the judgement line).
const RESULT_ROW_TEXT_Y: i32 = 317;
/// Nickname text y (318+22·row).
const RESULT_ROW_NAME_Y: i32 = 318;
/// Row pitch (row base y = 320+22·row).
const RESULT_ROW_PITCH: i32 = 22;
/// Seats on the member table: rows never exceed eight.
const RESULT_RANK_ROWS_MAX: usize = 8;
/// Nickname display limit (the exe truncates over 100 chars).
const RESULT_ROW_NAME_LIMIT: usize = 100;

/// One result rank-table row from live data (row painter
/// `FUN_004c3b40`, file `0xc3b40`): every column is `Option` — rows
/// without data stay unpainted with documented reasons, never
/// invented values. Rank bytes ride `PeerBars` (nonzero deliveries
/// only); names ride the joined room (`YOU` is local-only, the wire
/// sends no self nickname); judgements/combos/score/bonus ride the
/// local `Score`/`LongNoteBonus` (per-member 4014/4016 tallies stay
/// server-side, so those columns stay unpainted on peer rows); speed
/// rides the shared room speed code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRankRow {
    /// Rank number (`%d` at x=91, 2+ players only).
    pub rank: Option<u8>,
    /// Nickname at x=128 (truncated past 100 display chars).
    pub name: Option<String>,
    /// COOL/GOOD/BAD/MISS at x=230 (local row only).
    pub judgements: Option<[u32; 4]>,
    /// MAX then JAM combo at x=416 (local row only).
    pub combos: Option<(u32, u32)>,
    /// Score at x=530 (local row only).
    pub score: Option<u32>,
    /// Hold bonus at x=626 (local row only).
    pub bonus: Option<u64>,
    /// Room speed code at x=685 (every named row shares it).
    pub speed: Option<u8>,
}

/// Nickname as shown: first 100 display chars (the exe truncates
/// over 100 chars of CP949; the shell counts chars).
#[must_use]
fn truncate_result_name(name: &str) -> String {
    if name.chars().count() > RESULT_ROW_NAME_LIMIT {
        name.chars().take(RESULT_ROW_NAME_LIMIT).collect()
    } else {
        name.to_owned()
    }
}

/// Combined judgement text (`%5d /%5d /%3d /%3d`).
#[must_use]
fn result_judgements_text(counts: [u32; 4]) -> String {
    format!(
        "{:>5} /{:>5} /{:>3} /{:>3}",
        counts[0], counts[1], counts[2], counts[3]
    )
}

/// Combined combo text (`%4d /%4d`: MAX then JAM).
#[must_use]
fn result_combos_text(combos: (u32, u32)) -> String {
    format!("{:>4} /{:>4}", combos.0, combos.1)
}

/// Row score text (`%8d`).
#[must_use]
fn result_score_text(score: u32) -> String {
    format!("{score:>8}")
}

/// Row bonus text (`+ %d`).
#[must_use]
fn result_bonus_text(bonus: u64) -> String {
    format!("+ {bonus}")
}

/// Text-line y of display row `row` (`None` on overflow).
#[must_use]
fn result_rank_text_y(row: usize) -> Option<i32> {
    let step = i32::try_from(row).ok()?;
    let offset = step.checked_mul(RESULT_ROW_PITCH)?;
    RESULT_ROW_TEXT_Y.checked_add(offset)
}

/// Nickname-line y of display row `row` (`None` on overflow).
#[must_use]
fn result_rank_name_y(row: usize) -> Option<i32> {
    let step = i32::try_from(row).ok()?;
    let offset = step.checked_mul(RESULT_ROW_PITCH)?;
    RESULT_ROW_NAME_Y.checked_add(offset)
}

/// Paint one rank row: each `Some` column draws white GDI text from
/// its pinned left edge, each `None` column stays untouched.
fn paint_result_rank_row(frame: &mut Frame<'_>, row: usize, entry: &ResultRankRow, ink: u32) {
    let Some(y) = result_rank_text_y(row) else {
        return;
    };
    if let Some(rank) = entry.rank {
        draw_text(frame, RESULT_RANK_X, y, &rank.to_string(), ink);
    }
    if let Some(name) = entry.name.as_ref() {
        if let Some(name_y) = result_rank_name_y(row) {
            let shown = truncate_result_name(name);
            if !shown.is_empty() {
                draw_text(frame, RESULT_ROW_NAME_X, name_y, &shown, ink);
            }
        }
    }
    if let Some(counts) = entry.judgements {
        draw_text(
            frame,
            RESULT_ROW_JUDGE_X,
            y,
            &result_judgements_text(counts),
            ink,
        );
    }
    if let Some(combos) = entry.combos {
        draw_text(
            frame,
            RESULT_ROW_COMBO_X,
            y,
            &result_combos_text(combos),
            ink,
        );
    }
    if let Some(score) = entry.score {
        draw_text(frame, RESULT_ROW_SCORE_X, y, &result_score_text(score), ink);
    }
    if let Some(bonus) = entry.bonus {
        draw_text(frame, RESULT_ROW_BONUS_X, y, &result_bonus_text(bonus), ink);
    }
    if let Some(speed) = entry.speed {
        draw_text(frame, RESULT_ROW_SPEED_X, y, &room_speed_label(speed), ink);
    }
}

/// Paint the rank table (row painter `FUN_004c3b40`): up to eight
/// rows from live data, native canvas only. Empty models paint
/// nothing — the row band keeps its backdrop.
fn paint_result_rank_rows(frame: &mut Frame<'_>, rows: &[ResultRankRow]) {
    if !frame_is_native(frame) {
        return;
    }
    let ink = result_number_ink();
    for (row, entry) in rows.iter().enumerate().take(RESULT_RANK_ROWS_MAX) {
        paint_result_rank_row(frame, row, entry, ink);
    }
}

/// Paint one selected frame of a result body (the verdict banner or
/// the grade plate): frame *n* paints image *n−1* through the
/// saturating [`icon_frame_layer`], blitted 1:1 at the layer's
/// record anchor scaled into the frame — both pinned bodies anchor
/// every frame themselves (banner images at y=190 with their own x;
/// plate frames at (395,531)), so the anchor alone positions them
/// exactly where the exe draws (`docs/cresult-anchors.md`). A
/// missing body, frame, or layer paints nothing.
fn paint_result_body_frame(
    frame: &mut Frame<'_>,
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    body_frame: Option<u8>,
) {
    let Some(body_frame) = body_frame else {
        return;
    };
    let Some(figure) = figures.get(sprite) else {
        return;
    };
    let Some(layer_index) = icon_frame_layer(body_frame, figure.len()) else {
        return;
    };
    let Some(layer) = figure.layer(layer_index) else {
        return;
    };
    let (x, y) = anchor_in_frame(layer.dx, layer.dy, frame.w, frame.h);
    blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
}

/// Stretch the album cover over one rect, opaque: JPEG pixels carry
/// no transparency, and any 5-bit colorkey would punch holes where
/// cover colors match the key (the render cover blit compares decoded
/// colors). Nearest-neighbor sampling matches `blit_image_in_rect`;
/// pixels clip to the frame. 5-bit channels widen to 8 by
/// bit-replication, matching the render RGB555 expansion.
fn paint_album_cover(frame: &mut Frame<'_>, cover: &Rgb555Image, dest: Rect) {
    if cover.w == 0 || cover.h == 0 || dest.w == 0 || dest.h == 0 {
        return;
    }
    let dest_w = dest.w as usize;
    let dest_h = dest.h as usize;
    let frame_w = i32::try_from(frame.w).unwrap_or(i32::MAX);
    let frame_h = i32::try_from(frame.h).unwrap_or(i32::MAX);
    for row in 0..dest_h {
        let Some(y) = i32::try_from(row)
            .ok()
            .and_then(|offset| dest.y.checked_add(offset))
        else {
            continue;
        };
        if y < 0 || y >= frame_h {
            continue;
        }
        let src_row = row * cover.h / dest_h;
        for col in 0..dest_w {
            let Some(x) = i32::try_from(col)
                .ok()
                .and_then(|offset| dest.x.checked_add(offset))
            else {
                continue;
            };
            if x < 0 || x >= frame_w {
                continue;
            }
            let word = cover.words[src_row * cover.w + col * cover.w / dest_w];
            // 5-bit channels widened by bit-replication (the render
            // RGB555 expansion).
            let (r, g, b) = (
                ((word >> 10) & 31) as u8,
                ((word >> 5) & 31) as u8,
                (word & 31) as u8,
            );
            frame.buf[y as usize * frame.w + x as usize] = pack_xrgb(
                (r << 3) | (r >> 2),
                (g << 3) | (g >> 2),
                (b << 3) | (b >> 2),
            );
        }
    }
}

/// Paint the played song's album cover into the music-name box
/// ([`RESULT_MUSIC_BOX`], scaled into the frame like the placements):
/// the exe fills the box interior with the song's art over the baked
/// default pane. `None` paints nothing — the staged box stays as-is.
fn paint_result_album(frame: &mut Frame<'_>, album: Option<&Rgb555Image>) {
    let Some(cover) = album else {
        return;
    };
    paint_album_cover(
        frame,
        cover,
        scale_rect_to_frame(&RESULT_MUSIC_BOX, frame.w, frame.h),
    );
}

/// Paint the result backdrop: the staged scene minus the frame
/// bodies ([`RESULT_FRAME_BODIES`]) in file order, so their unused
/// frames never stack through the whole-layer pass. The Top/Bottom
/// panels (`StateResult_Top.ojs` at (0,0), `StateResult_Bottom.ojs`
/// at (0,250), overlapping y=250..316 with Bottom on top) ride this
/// pass: the fill (`FUN_004c3120`, file `0xc3120`) never issues draw
/// calls for the Top/Bottom row objects (`this+0x60[0..1]`), and the
/// per-frame draw-all (slot 9, `FUN_0048c570`) owns every control —
/// so staging their bodies here is the visible equivalent. How the
/// post-mount commit call registers them with that renderer stays
/// OPEN (dossier note 6). Empty art falls back to the flat
/// background; placements without a loaded figure are skipped.
fn paint_result_backdrop(frame: &mut Frame<'_>, art: &SceneArt) {
    if !art.has_figures() {
        fill_background(frame, Screen::Result);
        return;
    }
    let stacked: Vec<ScenePlacement> = art
        .placements
        .iter()
        .filter(|placed| {
            placed
                .sprite
                .as_deref()
                .is_none_or(|sprite| !RESULT_FRAME_BODIES.contains(&sprite))
        })
        .cloned()
        .collect();
    paint_scene_figures(frame, &stacked, &art.figures);
}

/// Paint the avatar figure, the pinned result numbers, and the rank
/// rows (shared by the flat fallback and the staged scene; the
/// numbers and rows are GDI text the exe paints regardless of art).
/// The album cover paints first into the music-name box so the text
/// layers over it, exactly on the green panel of the reference photo.
fn paint_result_content(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
    rows: &[ResultRankRow],
    album: Option<&Rgb555Image>,
) {
    paint_result_album(frame, album);
    if let Some(figure) = avatar {
        figure.paint(frame, frame.w as i32 / 2 - 32, frame.h as i32 / 2 - 32);
    }
    paint_result_numbers(frame, score);
    paint_result_rank_rows(frame, rows);
}

/// Paint the flat results fallback: card, album cover, avatar figure,
/// the pinned result numbers, and the rank rows. The retired text
/// verdict stays gone — the banner is a sprite frame of the staged art.
pub(crate) fn paint_result(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
    rows: &[ResultRankRow],
    album: Option<&Rgb555Image>,
) {
    paint_card(
        frame,
        pack_xrgb(0x10, 0x10, 0x08),
        pack_xrgb(0xC0, 0xA0, 0x20),
    );
    paint_result_content(frame, avatar, score, rows, album);
}

/// One staged result paint: everything the scene layers over the
/// backdrop, grouped so the painter keeps a readable signature
/// (`docs/cresult-anchors.md` + the album cover in the music-name
/// box).
#[derive(Debug, Clone, Copy)]
pub struct ResultPaint<'a> {
    /// Avatar figure (center card slot; `None` paints without one).
    pub avatar: Option<&'a AvatarFigure>,
    /// Live score behind the pinned GDI numbers.
    pub score: &'a o2jam_game::Score,
    /// Rank-table rows from live data.
    pub rows: &'a [ResultRankRow],
    /// Staged `.bnd` art for the scene.
    pub art: &'a SceneArt,
    /// Verdict the banner frame maps from.
    pub verdict: ResultVerdict,
    /// Grade plate frame (1..=8; `None` paints no plate).
    pub grade: Option<u8>,
    /// Played song's album cover (music-name box; `None` paints the
    /// staged box as-is).
    pub album: Option<&'a Rgb555Image>,
}

/// Paint the staged result scene (`docs/cresult-anchors.md`): the
/// `.bnd`-bound block art first (minus the frame bodies), then the
/// exe-selected verdict banner and grade plate frames, then the
/// played song's album cover in the music-name box, the avatar
/// figure, the pinned GDI numbers, and the rank rows on top — the
/// fill order of `FUN_004c3120` (banner, then top-panel painter,
/// then rows).
pub fn paint_result_scene(frame: &mut Frame<'_>, paint: &ResultPaint<'_>) {
    paint_result_backdrop(frame, paint.art);
    paint_result_body_frame(
        frame,
        &paint.art.figures,
        RESULT_WINLOSE_SPRITE,
        winlose_frame(paint.verdict),
    );
    paint_result_body_frame(
        frame,
        &paint.art.figures,
        RESULT_RANK_SPRITE,
        paint.grade.filter(|g| *g >= 1),
    );
    paint_result_content(frame, paint.avatar, paint.score, paint.rows, paint.album);
}

/// Score digits anchor top-left (`LivePlay` draws points at
/// `(16, 16)` and combo at `(16, 32)`): the behind scene stays out
/// of that digits box, in frame pixels.
/// Frame-space zones the behind-highway scene never paints: the lane
/// columns only. The exe paints the stage backdrop first of all
/// (applier `0x4e7850`, file order `BACK1` before `BACK2`/`BACK3`),
/// so the art fills the frame whole — no HUD cutouts — and the passes
/// after it (staged gauges, avatar-scene captions, tally strip) keep
/// their top slot; only the falling lanes must stay uncovered, since
/// notes ride them.
fn playing_lane_exclusions(frame_w: usize, frame_h: usize) -> Vec<Rect> {
    HighwayGeometry::centered(frame_w, frame_h)
        .lanes
        .into_iter()
        .collect()
}

/// Paint the behind-highway scene with no exclusions: the caller
/// paints the highway after it, so lanes and UI ink cover the scene
/// wherever they overlap. Staged overlay panels are untouched here;
/// see [`paint_playing_overlay`] for the post-highway path.
pub fn paint_playing_backdrop(frame: &mut Frame<'_>, art: &SceneArt) {
    paint_behind_placements(frame, &art.placements, &art.figures, &[]);
}

/// Paint staged gameplay art around the highway: the behind-highway
/// scene first (`Playing_BG<stage>.ojs` plus the `Note_BG` siblings —
/// see `avatar::PLAYING_BEHIND_SPRITES`; `Note_BG.ojs` rides its
/// exe-selected frame only, `avatar::playing_member_frame`), clipped
/// only around the
/// lane columns per the exe paint order (the backdrop sits at the
/// bottom; staged gauges, captions, and the tally strip paint over
/// it), then the static backdrop panels (`Playing_Menu.ojs`, the
/// life/jam edge gauges, the level plate) and the bound gauges and
/// buttons (`State_Playing.bnd` ordinals 1–17) on top. Lanes stay
/// uncovered and notes keep their top slot. Procedural peer gauges sit
/// above this overlay: `LivePlay::paint_avatar_scene` paints them after,
/// in file order over the venue art they cover.
pub fn paint_playing_overlay(frame: &mut Frame<'_>, art: &SceneArt) {
    let exclusions = playing_lane_exclusions(frame.w, frame.h);
    paint_behind_placements(frame, &art.placements, &art.figures, &exclusions);
    paint_foreground_placements(frame, &art.placements, &art.figures);
}

/// Fill the whole frame with one screen's background.
fn fill_background(frame: &mut Frame<'_>, screen: Screen) {
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: u32::try_from(frame.w).unwrap_or(u32::MAX),
            h: u32::try_from(frame.h).unwrap_or(u32::MAX),
        },
        background_of(screen),
    );
}

/// Background plus a centered card.
fn paint_card(frame: &mut Frame<'_>, background: u32, card: u32) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        background,
    );
    fill_rect(
        frame,
        &Rect {
            x: w / 2 - w / 8,
            y: h / 2 - h / 8,
            w: (w / 4) as u32,
            h: (h / 4) as u32,
        },
        card,
    );
}

/// Background color per screen (lobbies share the card shape).
pub(crate) fn background_of(screen: Screen) -> u32 {
    match screen {
        Screen::Avi => pack_xrgb(0x08, 0x08, 0x10),
        Screen::Channel => pack_xrgb(0x10, 0x20, 0x50),
        Screen::Room => pack_xrgb(0x10, 0x50, 0x20),
        Screen::Waiting => pack_xrgb(0x50, 0x40, 0x10),
        Screen::Loading => pack_xrgb(0x18, 0x18, 0x18),
        Screen::Playing => pack_xrgb(0x0A, 0x0A, 0x0A),
        Screen::Result => pack_xrgb(0x10, 0x10, 0x08),
    }
}

/// Title drawn on each lobby card.
fn title_of(screen: Screen) -> &'static str {
    match screen {
        Screen::Channel => "CHANNEL",
        Screen::Room => "ROOM",
        Screen::Waiting => "WAITING",
        _ => "",
    }
}

/// Paint one screen's scene.
pub fn paint_screen(frame: &mut Frame<'_>, screen: Screen) {
    match screen {
        Screen::Avi => paint_avi(frame),
        Screen::Channel | Screen::Room | Screen::Waiting => {
            paint_lobby(frame, background_of(screen));
            paint_title(frame, title_of(screen));
        }
        Screen::Loading => paint_loading(frame),
        Screen::Playing => {
            PlayingView::default().paint(frame, &HighwayGeometry::centered(frame.w, frame.h));
        }
        Screen::Result => paint_result(frame, None, &o2jam_game::Score::default(), &[], None),
    }
}

#[cfg(test)]
mod paint_tests {
    use super::*;
    use crate::avatar::FigureLayer;
    use crate::screen::Screen;
    use o2jam_net::{JoinedRoom, RoomMember, RoomSlot};
    use o2jam_render::{has_ink_near, near_xrgb};
    use o2jam_render::{pack_xrgb, Frame, Rgb555Image};

    fn painted(screen: Screen, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        paint_screen(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            screen,
        );
        buf
    }

    /// One red pixel figure (colorkey white, so the pixel paints).
    fn red_figure() -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Art staging one red pixel at the frame origin.
    fn corner_art() -> SceneArt {
        SceneArt {
            placements: unbound_scene_placements(&["Logo.ojs".to_owned()]),
            figures: HashMap::from([("Logo.ojs".to_owned(), red_figure())]),
        }
    }

    fn paint_art(screen: Screen, art: &SceneArt, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        let mut frame = Frame {
            buf: &mut buf,
            w,
            h,
        };
        match screen {
            Screen::Avi => paint_avi_scene(&mut frame, art),
            Screen::Channel => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Room => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Waiting => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Loading => paint_loading_scene(&mut frame, art),
            Screen::Playing => {
                paint_screen(&mut frame, screen);
                paint_playing_overlay(&mut frame, art);
            }
            Screen::Result => paint_result_scene(
                &mut frame,
                &ResultPaint {
                    avatar: None,
                    score: &o2jam_game::Score::default(),
                    rows: &[],
                    art,
                    verdict: ResultVerdict::Solo,
                    grade: None,
                    album: None,
                },
            ),
        }
        buf
    }

    #[test]
    fn splash_paints_logo_at_native_origin_without_stamp() {
        // Unbound logo art paints 1:1 at the native origin: logo
        // pixel at the origin, no flat card, no version-stamp white.
        let buf = paint_art(Screen::Avi, &corner_art(), 32, 24);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!buf.contains(&pack_xrgb(0x20, 0x60, 0xC0)));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn flat_fallback_covers_bare_frames() {
        // While no art is staged the flat colors remain (the flat
        // list/member painters depend on them); staged scenes never
        // use them.
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let buf = painted(screen, 16, 12);
            assert_eq!(buf[0], background_of(screen), "{screen:?}");
        }
    }

    #[test]
    fn every_art_scene_hides_flat_fill() {
        // Staged art-first composition: the logo pixel survives at
        // the origin and no flat background shows through.
        let art = corner_art();
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let buf = paint_art(screen, &art, 16, 12);
            assert_eq!(buf[0], pack_xrgb(255, 0, 0), "{screen:?}");
            assert!(
                !buf.iter().all(|pixel| *pixel == background_of(screen)),
                "{screen:?} paints a flat fill"
            );
        }
    }

    #[test]
    fn scene_indexes_cover_every_screen_once() {
        let indexes = [
            scene_index(Screen::Avi),
            scene_index(Screen::Channel),
            scene_index(Screen::Room),
            scene_index(Screen::Waiting),
            scene_index(Screen::Loading),
            scene_index(Screen::Playing),
            scene_index(Screen::Result),
        ];
        let mut sorted = indexes;
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn unbound_placements_pin_origin_without_size() {
        // No `.bnd` means no layout: unbound sprites paint 1:1 at
        // the native origin with an empty rect, never full-bleed.
        let placed = unbound_scene_placements(&["A.ojs".to_owned(), "B.ojs".to_owned()]);
        assert_eq!(placed.len(), 2);
        for placement in &placed {
            assert_eq!(
                placement.rect,
                Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                }
            );
        }
        assert_eq!(placed[0].sprite.as_deref(), Some("A.ojs"));
    }

    #[test]
    fn lobby_titles_stay_readable_over_art() {
        // 128x96 channel backdrop: white title ink over the dark
        // scrim, never the flat card gray.
        let buf = paint_art(Screen::Channel, &corner_art(), 128, 96);
        let scrim = pack_xrgb(0x08, 0x08, 0x10);
        let white = pack_xrgb(0xFF, 0xFF, 0xFF);
        let card = pack_xrgb(0xD0, 0xD0, 0xD0);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&white));
        assert!(buf.contains(&scrim));
        assert!(!buf.contains(&card));
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
            slots: vec![
                RoomSlot {
                    index: 0,
                    state_code: 1,
                    member: Some(member),
                },
                RoomSlot {
                    index: 1,
                    state_code: 0,
                    member: None,
                },
                RoomSlot {
                    index: 2,
                    state_code: 2,
                    member: None,
                },
            ],
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    #[test]
    fn waiting_seats_pad_self_plus_slots_to_eight() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(seats.len(), WAITING_SEATS);
        assert_eq!(seats[0].display_name(), Some("YOU"));
        assert_eq!(seats[1].display_name(), Some("host"));
        assert!(seats[1].is_ready());
        assert!(!seats[0].is_ready());
        assert_eq!(seats[2].display_name(), None);
        assert_eq!(seats[3].kind, SeatKind::Locked);
        assert!(seats[4..].iter().all(|seat| seat.kind == SeatKind::Empty));
    }

    #[test]
    fn waiting_row_text_shows_name_team_and_ready() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(waiting_row_text(1, &seats[1]), "1 A host R");
        assert_eq!(waiting_row_text(0, &seats[0]), "0 B YOU -");
        assert_eq!(waiting_row_text(2, &seats[2]), "2 - ---");
        assert_eq!(waiting_row_text(3, &seats[3]), "3 - LOCK");
    }

    #[test]
    fn waiting_paints_ready_dots_for_seats() {
        let room = waiting_room();
        let mut buf = vec![0u32; 80 * 64];
        paint_waiting_members(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            &room,
        );
        let waiting_bg = background_of(Screen::Waiting);
        let ready_green = pack_xrgb(0x30, 0xE0, 0x30);
        let dim = pack_xrgb(0x50, 0x50, 0x50);
        assert_eq!(buf[0], waiting_bg);
        // Seat 1 (ready host) sits in column 1, row 0: dot at (44, 16).
        let (host_x, host_y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(host_y + 1) as usize * 80 + (host_x - 6 + 1) as usize],
            ready_green
        );
        // Seat 0 (self, not ready) keeps a dim dot.
        let (self_x, self_y) = seat_origin(0, 80, 64);
        assert_eq!(
            buf[(self_y + 1) as usize * 80 + (self_x - 6 + 1) as usize],
            dim
        );
        // Seat 2 (empty) has no dot: the seat scrim shows through.
        let (empty_x, empty_y) = seat_origin(2, 80, 64);
        assert_eq!(
            buf[(empty_y + 1) as usize * 80 + (empty_x - 6 + 1) as usize],
            scrim_color()
        );
    }

    #[test]
    fn paint_is_safe_on_empty_frame() {
        let mut buf = vec![];
        paint_screen(
            &mut Frame {
                buf: &mut buf,
                w: 0,
                h: 0,
            },
            Screen::Playing,
        );
    }

    fn scene_control_text() -> String {
        [
            "NUMBER_OF_STATE 0x03",
            "STATE_CHANNEL 0x02",
            "{",
            "BOUND",
            "IDC_BAR 0X00100000",
            "IDC_SELECT 0X10500001 \"Dialog_ChannelSelectBar.ojs\"",
            "}",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X00100000",
            "IDC_LIST 0X10500001 \"Room_List.ojs\"",
            "}",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn main_sprites_name_channel_dialog_art() {
        let list = o2jam_file::parse_control_list(&scene_control_text()).unwrap();
        assert_eq!(
            main_scene_sprites(&list, Screen::Channel),
            vec!["Dialog_ChannelSelectBar.ojs".to_owned()]
        );
        assert_eq!(
            main_scene_sprites(&list, Screen::Room),
            vec!["Room_List.ojs".to_owned()]
        );
    }

    #[test]
    fn main_sprites_fall_back_to_single_art() {
        let list = o2jam_file::parse_control_list(&scene_control_text()).unwrap();
        assert_eq!(
            main_scene_sprites(&list, Screen::Avi),
            vec![AVI_LOGO_SPRITE.to_owned()]
        );
        assert_eq!(
            main_scene_sprites(&list, Screen::Loading),
            vec![LOADING_SPRITE.to_owned()]
        );
        assert!(main_scene_sprites(&list, Screen::Playing).is_empty());
        assert!(main_scene_sprites(&list, Screen::Result).is_empty());
    }

    /// One 1x1 pixel figure at a native anchor (colorkey white).
    fn anchored_pixel(dx: i32, dy: i32, bytes: &[u8]) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(bytes, 1, 1).unwrap(),
            dx,
            dy,
            colorkey: 0xFFFF,
        });
        figure
    }

    #[test]
    fn playing_overlay_layers_bound_panels_over_backdrop() {
        // Backdrop-first file order: the bound button pixel wins
        // the shared menu-strip pixel, the strip survives beside
        // it, and the lane pixel keeps its highway ink.
        let mut menu = AvatarFigure::default();
        for dx in [750, 751] {
            menu.push_layer(FigureLayer {
                image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
                dx,
                dy: 550,
                colorkey: 0xFFFF,
            });
        }
        let art = SceneArt {
            placements: vec![
                ScenePlacement {
                    id: "IDC_IMAGE_SPLAY_BACK3".to_owned(),
                    code: 0,
                    sprite: Some("Playing_Menu.ojs".to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 0,
                        h: 0,
                    },
                },
                ScenePlacement {
                    id: "IDC_BUTTON_SPLAY_EXIT".to_owned(),
                    code: 0,
                    sprite: Some("Playing_Exit.ojs".to_owned()),
                    rect: Rect {
                        x: 751,
                        y: 550,
                        w: 10,
                        h: 10,
                    },
                },
            ],
            figures: HashMap::from([
                ("Playing_Menu.ojs".to_owned(), menu),
                (
                    "Playing_Exit.ojs".to_owned(),
                    anchored_pixel(751, 550, &[0x1F, 0x00]),
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        assert_eq!(buf[550 * 800 + 750], pack_xrgb(255, 0, 0));
        assert_eq!(buf[550 * 800 + 751], pack_xrgb(0, 0, 255));
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn playing_overlay_skips_panels_without_figures() {
        // A named-but-undecodable panel paints nothing: the highway
        // ink underneath survives.
        let art = SceneArt {
            placements: unbound_scene_placements(&["Missing.ojs".to_owned()]),
            figures: HashMap::new(),
        };
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    /// Behind scene with red pixels across the frame: one in a lane
    /// column (10,100), the venue body box (204,100), and the former
    /// HUD-zone cutouts — judgement band row (400,447), peer-stack
    /// corner (760,4), jam-bar row (600,540) — so the exe-order rule
    /// is observable: only lane columns stay clear.
    fn venue_backdrop_art() -> SceneArt {
        let mut venue = AvatarFigure::default();
        for (dx, dy) in [(10, 100), (204, 100), (400, 447), (760, 4), (600, 540)] {
            venue.push_layer(FigureLayer {
                image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
                dx,
                dy,
                colorkey: 0xFFFF,
            });
        }
        SceneArt {
            placements: vec![ScenePlacement {
                id: "IDC_IMAGE_SPLAY_BACK1".to_owned(),
                code: 0,
                sprite: Some("Playing_BG.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
            }],
            figures: HashMap::from([("Playing_BG.ojs".to_owned(), venue)]),
        }
    }

    #[test]
    fn playing_overlay_keeps_only_the_lanes_clear_of_behind_scene() {
        // Exe paint order: the backdrop sits at the bottom (applier
        // `0x4e7850`, `BACK1` first), so no HUD cutouts remain — the
        // art fills the former score/judgement/peer/jam zones while
        // the lane column alone keeps its highway ink.
        let art = venue_backdrop_art();
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        let highway = pack_xrgb(0x0A, 0x0A, 0x0A);
        assert_eq!(buf[100 * 800 + 10], highway);
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(255, 0, 0));
        assert_eq!(buf[447 * 800 + 400], pack_xrgb(255, 0, 0));
        assert_eq!(buf[4 * 800 + 760], pack_xrgb(255, 0, 0));
        assert_eq!(buf[540 * 800 + 600], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn playing_backdrop_paints_unclipped_before_highway() {
        // The pre-highway API paints the whole scene body with no
        // exclusions: lane and venue pixels turn red alike, and the
        // highway shell covers every one of them after.
        let art = venue_backdrop_art();
        let mut buf = vec![0u32; 800 * 600];
        paint_playing_backdrop(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(255, 0, 0));
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(255, 0, 0));
        assert_eq!(buf[447 * 800 + 400], pack_xrgb(255, 0, 0));
        PlayingView::default().paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &HighwayGeometry::centered(800, 600),
        );
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[100 * 800 + 400], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn playing_overlay_layers_gauges_over_behind_scene() {
        // Bound gauges keep their top slot: the shared strip pixel
        // shows the gauge ink while the scene survives beside it.
        let mut art = venue_backdrop_art();
        art.placements.push(ScenePlacement {
            id: "IDC_BUTTON_SPLAY_EXIT".to_owned(),
            code: 0,
            sprite: Some("Playing_Exit.ojs".to_owned()),
            rect: Rect {
                x: 400,
                y: 100,
                w: 10,
                h: 10,
            },
        });
        art.figures.insert(
            "Playing_Exit.ojs".to_owned(),
            anchored_pixel(400, 100, &[0x1F, 0x00]),
        );
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        assert_eq!(buf[100 * 800 + 400], pack_xrgb(0, 0, 255));
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn avi_scene_shows_logo_without_stamp() {
        // Corner art paints the logo pixel; the stamp row that the
        // flat splash whites out stays unpainted (blank canvas)
        // under staged art.
        let buf = paint_art(Screen::Avi, &corner_art(), 32, 24);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 32 + 11], 0);
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
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
    fn channel_scene_layers_list_over_art() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_channels(),
            0,
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn channel_row_names_gateway_and_channel_ids() {
        let channel = ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        };
        assert_eq!(channel_row_text(&channel), "1/2 5/100");
    }

    #[test]
    fn channel_row_orders_population_over_capacity() {
        let channel = ChannelState {
            gateway_id: 0,
            channel_id: 9,
            capacity: 200,
            population: 7,
            active: false,
        };
        assert_eq!(channel_row_text(&channel), "0/9 7/200");
    }

    #[test]
    fn channel_rows_dim_inactive_channels() {
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_list(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_channels(),
            1,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0x70, 0x70, 0x70)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    fn two_rooms() -> Vec<RoomListEntry> {
        vec![RoomListEntry {
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
        }]
    }

    #[test]
    fn room_scene_layers_rows_over_art() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_rooms(),
            0,
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    /// Room fixture with every detail field set to a distinct value.
    fn detailed_room() -> RoomListEntry {
        RoomListEntry {
            number: 3,
            state_code: 1,
            title: "Jam".to_owned(),
            has_password: true,
            music_id: 757,
            difficulty: 2,
            key_mode: 1,
            game_mode: 1,
            speed: 3,
            capacity: 8,
            user_count: 5,
            min_level: 10,
            max_level: 99,
            skills: vec![],
            premium: false,
            room_type: 0,
        }
    }

    #[test]
    fn room_mode_names_each_game_mode() {
        assert_eq!(room_mode_label(0), "NORMAL");
        assert_eq!(room_mode_label(1), "LIVE");
        assert_eq!(room_mode_label(9), "MODE 9");
    }

    #[test]
    fn room_music_shows_song_id() {
        assert_eq!(room_music_label(757), "M757");
        assert_eq!(room_music_label(0), "M0");
    }

    #[test]
    fn room_difficulty_names_each_band() {
        assert_eq!(room_difficulty_label(0), "EX");
        assert_eq!(room_difficulty_label(1), "NX");
        assert_eq!(room_difficulty_label(2), "HX");
        assert_eq!(room_difficulty_label(3), "RX");
        assert_eq!(room_difficulty_label(9), "DIF 9");
    }

    #[test]
    fn room_key_names_each_key_mode() {
        assert_eq!(room_key_label(0), "3KEY");
        assert_eq!(room_key_label(1), "5KEY");
        assert_eq!(room_key_label(2), "7KEY");
        assert_eq!(room_key_label(7), "KEY 7");
    }

    #[test]
    fn room_speed_names_each_step() {
        let expected = [
            "X0.5", "X1", "X1.5", "X2", "X2.5", "X3", "X3.5", "X4", "X4.5", "X5", "X6", "XR",
        ];
        for (code, label) in expected.iter().enumerate() {
            assert_eq!(&room_speed_label(code as u8), label);
        }
        assert_eq!(room_speed_label(12), "SPD 12");
    }

    #[test]
    fn room_capacity_shows_seats_over_capacity() {
        assert_eq!(room_capacity_label(&detailed_room()), "5/8");
    }

    #[test]
    fn room_level_shows_min_to_max_gate() {
        assert_eq!(room_level_label(&detailed_room()), "LV 10-99");
    }

    #[test]
    fn room_lock_names_password_state() {
        assert_eq!(room_lock_label(true), "LOCK");
        assert_eq!(room_lock_label(false), "OPEN");
        assert_eq!(
            room_lock_ink(true),
            pack_xrgb(0xFF, 0x50, 0x50),
            "locked line reads red"
        );
    }

    #[test]
    fn room_detail_entries_cover_every_panel_line() {
        let entries = room_detail_entries(&detailed_room());
        assert_eq!(entries.len(), ROOM_DETAIL_LINES);
        let ids: Vec<&str> = entries.iter().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "each line anchors to its own rect");
        let by_id = |want: &str| {
            entries
                .iter()
                .find(|(id, _)| *id == want)
                .map(|(_, text)| text.clone())
                .unwrap()
        };
        assert_eq!(by_id(ROOM_DETAIL_TITLE_ID), "Jam");
        assert_eq!(by_id(ROOM_DETAIL_USERS_ID), "5/8");
        assert_eq!(by_id(ROOM_DETAIL_MUSIC_ID), "M757");
        assert_eq!(by_id(ROOM_DETAIL_DIFFICULTY_ID), "HX");
        assert_eq!(by_id(ROOM_DETAIL_KEY_ID), "5KEY");
        assert_eq!(by_id(ROOM_DETAIL_MODE_ID), "LIVE");
        assert_eq!(by_id(ROOM_DETAIL_SPEED_ID), "X2");
        assert_eq!(by_id(ROOM_DETAIL_LEVEL_ID), "LV 10-99");
        assert_eq!(by_id(ROOM_DETAIL_LOCK_ID), "LOCK");
    }

    /// Placements pinning the title rect apart from the rest: the
    /// title lands at native (80, 60), every other line at native
    /// (720, 540).
    fn detail_placements() -> Vec<ScenePlacement> {
        let title = ScenePlacement {
            id: ROOM_DETAIL_TITLE_ID.to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 80,
                y: 60,
                w: 80,
                h: 60,
            },
        };
        let rest = room_detail_entries(&detailed_room())
            .iter()
            .filter(|(id, _)| *id != ROOM_DETAIL_TITLE_ID)
            .map(|(id, _)| ScenePlacement {
                id: (*id).to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 720,
                    y: 540,
                    w: 80,
                    h: 60,
                },
            })
            .collect::<Vec<_>>();
        std::iter::once(title).chain(rest).collect()
    }

    #[test]
    fn room_details_anchor_to_bnd_rects() {
        // 200x150 frame: native (80, 60) scales to (20, 15), so the
        // title scrim opens at (16, 13) — clear of the list scrim
        // (x 42.., y 35..47) and the centered title.
        let rooms = vec![detailed_room()];
        let mut buf = vec![0u32; 200 * 150];
        paint_room_details(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            &rooms,
            0,
            Some(&detail_placements()),
        );
        assert_eq!(buf[13 * 200 + 16], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn room_details_fall_back_under_list_without_staged_ids() {
        // Flat fallback stacks the first detail line under the one
        // list row: x 50, y 49, so its scrim opens at (46, 47) —
        // below the row ink (y 37..42) and the list scrim (y 35..47).
        let rooms = vec![detailed_room()];
        let mut buf = vec![0u32; 200 * 150];
        paint_room_list(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            &rooms,
            0,
        );
        assert_eq!(buf[47 * 200 + 46], scrim_color());
        // List plus cursor keep working under the panel.
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
        // Locked room paints the red lock line.
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0x50, 0x50)));
    }

    #[test]
    fn room_details_stay_empty_without_a_room() {
        let mut buf = vec![0u32; 64 * 48];
        paint_room_list(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[],
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Room));
        assert!(!buf.contains(&scrim_color()));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0x50, 0x50)));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn waiting_scene_keeps_ready_dots_over_art() {
        let art = corner_art();
        let room = waiting_room();
        let mut buf = vec![0u32; 80 * 64];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            &room,
            &art,
            &WaitingCaptions::default(),
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn loading_scene_keeps_bar_over_art() {
        let buf = paint_art(Screen::Loading, &corner_art(), 64, 48);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
    }

    #[test]
    fn result_scene_keeps_small_frames_number_free() {
        // 64x48 is not the exe's native canvas, so the pinned GDI
        // numbers wait (frame_is_native): the staged art and the
        // avatar paint, no white digit ink anywhere.
        let art = corner_art();
        let score = o2jam_game::Score {
            points: 200,
            max_combo: 7,
            ..o2jam_game::Score::default()
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &ResultPaint {
                avatar: Some(&red_figure()),
                score: &score,
                rows: &[],
                art: &art,
                verdict: ResultVerdict::Solo,
                grade: None,
                album: None,
            },
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(255, 0, 0)));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0x40)));
    }

    #[test]
    fn playing_overlay_keeps_highway_marks() {
        let buf = paint_art(Screen::Playing, &corner_art(), 70, 60);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xE0, 0xE0, 0xE0)));
    }

    #[test]
    fn empty_art_falls_back_to_flat() {
        // No figures staged anywhere: Avi paints nothing at all (no
        // logo, no stamp); lobbies keep only the readable title over
        // the untouched frame; loading keeps only the live bar; the
        // highway and result scenes keep their own painters. No staged
        // path invents a fill (`docs/channel-fill.md` no-fill rule).
        let art = SceneArt::default();
        assert!(!art.has_figures());
        let avi = paint_art(Screen::Avi, &art, 16, 12);
        assert!(avi.iter().all(|pixel| *pixel == 0));
        for screen in [Screen::Channel, Screen::Room, Screen::Waiting] {
            let buf = paint_art(screen, &art, 16, 12);
            assert_eq!(buf[0], 0, "{screen:?} paints no fill");
            assert!(
                !buf.iter().all(|pixel| *pixel == background_of(screen)),
                "{screen:?} paints no flat fill"
            );
            assert!(
                has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)),
                "{screen:?} keeps title ink"
            );
        }
        let loading = paint_art(Screen::Loading, &art, 16, 12);
        assert!(!loading.contains(&background_of(Screen::Loading)));
        assert!(loading.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
        let playing = paint_art(Screen::Playing, &art, 16, 12);
        assert_eq!(playing[0], background_of(Screen::Playing));
        let result = paint_art(Screen::Result, &art, 16, 12);
        assert_eq!(result[0], background_of(Screen::Result));
    }

    fn battle_room() -> RoomTeams {
        let mut teams = RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.submit(0, 100_000);
        teams.submit(1, 150_000);
        teams
    }

    #[test]
    fn result_loader_names_top_bottom_and_winlose() {
        assert_eq!(
            result_loader_sprites(),
            [
                "StateResult_Top.ojs",
                "StateResult_Bottom.ojs",
                "Result_WinLose.ojs"
            ]
        );
        assert_eq!(RESULT_WINLOSE_SPRITE, "Result_WinLose.ojs");
        assert_eq!(WINNER_IMAGE_SPRITE, "Winner_image.ojs");
        assert_eq!(RESULT_RANK_SPRITE, "Result_Imgs_Rank.ojs");
    }

    #[test]
    fn battle_verdict_goes_to_rank_one_by_score() {
        let teams = battle_room();
        assert_eq!(teams.winner(), Some(1));
        assert_eq!(battle_verdict(&teams, Some(1)), Some(BattleVerdict::Win));
        assert_eq!(battle_verdict(&teams, Some(0)), Some(BattleVerdict::Lose));
    }

    #[test]
    fn battle_verdict_ranks_three_teams_to_other() {
        let mut teams = RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.set_team(2, 2);
        teams.submit(0, 300_000);
        teams.submit(1, 200_000);
        teams.submit(2, 100_000);
        assert_eq!(battle_verdict(&teams, Some(0)), Some(BattleVerdict::Win));
        assert_eq!(battle_verdict(&teams, Some(1)), Some(BattleVerdict::Lose));
        assert_eq!(battle_verdict(&teams, Some(2)), Some(BattleVerdict::Other));
    }

    #[test]
    fn battle_verdict_stays_empty_without_winner_or_self() {
        let empty = RoomTeams::new();
        assert_eq!(battle_verdict(&empty, Some(0)), None);
        let teams = battle_room();
        assert_eq!(battle_verdict(&teams, None), None);
    }

    /// Pinned but unpainted digit-sprite cells (dossier OPEN 1): the
    /// life readout and the bottom counter run through the
    /// frame-select/draw pair over eight fixed 24 px cells,
    /// least-significant digit rightmost, digit 1 kerned +5 (shared
    /// painter bodies at files `0xc44c0`/`0xc4630`: cells
    /// `last - 168 ..= last` in 24 px steps). Life value =
    /// `0x77ee58 + 0x77ee60`, y=77, last x=460
    /// (292/316/340/364/388/412/436/460); bottom value = `0x76fc66`,
    /// y=536, last x=176. The pitch and kern match `PointNum.ojs`, but
    /// the binding is inference, not observed — so neither paints.
    /// `Result_NumFont_Blue.ojs` (`this+0x10d0c`) has no reader either.
    const RESULT_LIFE_Y: i32 = 77;
    const RESULT_LIFE_LAST_X: i32 = 460;
    const RESULT_BOTTOM_Y: i32 = 536;
    const RESULT_BOTTOM_LAST_X: i32 = 176;
    const RESULT_DIGIT_PITCH: i32 = 24;
    const RESULT_DIGIT_CELLS: usize = 8;

    /// Row geometry (row painter `FUN_004c3b40`, file `0xc3b40`;
    /// dossier OPEN 4/7): row base y = 320 + 22·row. Column x values
    /// now live beside the row model; only the base stays here for
    /// the empty-band check.
    const RESULT_ROW_BASE_Y: i32 = 320;

    /// Cell x values of one eight-cell digit field, rightmost last.
    fn digit_cell_xs(last_x: i32) -> [i32; RESULT_DIGIT_CELLS] {
        let mut xs = [0; RESULT_DIGIT_CELLS];
        let mut index = 0;
        while index < RESULT_DIGIT_CELLS {
            let back = i32::try_from(RESULT_DIGIT_CELLS - 1 - index).unwrap_or(0);
            xs[index] = last_x - back * RESULT_DIGIT_PITCH;
            index += 1;
        }
        xs
    }

    /// Flat native result for `score` plus `rows`, exactly as the
    /// no-art fallback paints it.
    fn flat_native_rows(score: &o2jam_game::Score, rows: &[ResultRankRow]) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            score,
            rows,
            None,
        );
        buf
    }

    /// Flat native result for `score` with no rank rows, exactly as
    /// the no-art fallback paints it.
    fn flat_native_result(score: &o2jam_game::Score) -> Vec<u32> {
        flat_native_rows(score, &[])
    }

    /// Staged native result scene for one verdict and grade.
    fn painted_native_scene(art: &SceneArt, verdict: ResultVerdict, grade: Option<u8>) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &ResultPaint {
                avatar: None,
                score: &o2jam_game::Score::default(),
                rows: &[],
                art,
                verdict,
                grade,
                album: None,
            },
        );
        buf
    }

    /// Frame body with one opaque 2x2 layer per `anchors` entry, each
    /// a distinct solid RGB555 ink (black colorkey) — the pinned
    /// banner/plate bodies, shrunk to probe pixels.
    fn frame_body(anchors: &[(i32, i32)]) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        for (index, (dx, dy)) in anchors.iter().enumerate() {
            figure.push_layer(FigureLayer {
                image: Rgb555Image {
                    w: 2,
                    h: 2,
                    words: vec![0x0001u16 << (index + 1); 4],
                },
                dx: *dx,
                dy: *dy,
                colorkey: 0x0000,
            });
        }
        figure
    }

    /// Solid ink of frame-body layer `index` (see [`frame_body`]).
    fn frame_ink(index: usize) -> u32 {
        let (r, g, b) = o2jam_render::expand_rgb555(0x0001u16 << (index + 1));
        pack_xrgb(r, g, b)
    }

    /// Score with every pinned field set to a distinct full-top-row
    /// digit (the bitmap glyphs of 8/9/3/2/7/5 all carry a lit
    /// top-left pixel; the TTF glyphs anchor the same left edge).
    fn full_score() -> o2jam_game::Score {
        let mut score = o2jam_game::Score {
            points: 87_654_321,
            max_combo: 77,
            counts: [800, 900, 300, 200],
            ..o2jam_game::Score::default()
        };
        for _ in 0..50 {
            score.jam.note(o2jam_game::Judgement::Cool);
        }
        score
    }

    /// One pinned field: ink must light inside the anchor's cell box
    /// (the TTF rasterizes within the cell, antialiased) and the
    /// column left of the anchor must stay flat (the decimal starts
    /// exactly at the pinned x).
    fn assert_field(buf: &[u32], x: i32, y: i32) {
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        let ink = result_number_ink();
        let cell_lit = (x..x + 10).any(|px| (y..y + 18).any(|py| near_xrgb(at(px, py), ink)));
        assert!(cell_lit, "field at ({x},{y}) must light ink");
        for py in y..y + 16 {
            assert!(
                !near_xrgb(at(x - 1, py), ink),
                "left of ({x},{py}) must stay flat"
            );
        }
    }

    #[test]
    fn cool_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_COOL_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn good_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_GOOD_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn bad_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_BAD_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn miss_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_MISS_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn max_combo_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_MAX_COMBO_X,
            RESULT_COMBO_Y,
        );
    }

    #[test]
    fn jam_combo_field_paints_the_jam_level_chain() {
        // 50 Cools climb the jam to level 2 (progress 100 over the
        // level-2 threshold): the JAM COMBO field shows the chain,
        // the wire's u16 `jam` field.
        assert_eq!(full_score().jam.level(), 2);
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_JAM_COMBO_X,
            RESULT_COMBO_Y,
        );
    }

    #[test]
    fn total_score_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_SCORE_X,
            RESULT_SCORE_Y,
        );
    }

    #[test]
    fn result_numbers_clamp_at_the_32_bit_field() {
        // The `%8d` read is the exe's 32-bit field: an oversized u64
        // accumulator saturates instead of wrapping.
        let score = o2jam_game::Score {
            points: u64::from(u32::MAX) + 5,
            ..o2jam_game::Score::default()
        };
        let expected = flat_native_result(&o2jam_game::Score {
            points: u64::from(u32::MAX),
            ..o2jam_game::Score::default()
        });
        let actual = flat_native_result(&score);
        assert_eq!(actual, expected);
    }

    #[test]
    fn result_numbers_wait_for_the_native_canvas() {
        // 70x60 is not the exe canvas: no field paints, matching the
        // small-frame contract of tests/scenes.rs.
        let mut buf = vec![0u32; 70 * 60];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            None,
            &full_score(),
            &[],
            None,
        );
        assert!(!buf.contains(&result_number_ink()));
    }

    #[test]
    fn life_and_bottom_digit_cells_stay_unpainted() {
        // Dossier OPEN 1: the eight-cell digit fields (24 px pitch,
        // digit 1 kerned +5) have no observed font binding, so the
        // shell paints neither — the GDI numbers above never touch
        // the (292..460, 77) or (8..176, 536) cell rows.
        let buf = flat_native_result(&full_score());
        let ink = result_number_ink();
        for x in digit_cell_xs(RESULT_LIFE_LAST_X) {
            let pixel = buf[RESULT_LIFE_Y as usize * 800 + x as usize];
            assert_ne!(pixel, ink, "life cell x={x}");
        }
        for x in digit_cell_xs(RESULT_BOTTOM_LAST_X) {
            let pixel = buf[RESULT_BOTTOM_Y as usize * 800 + x as usize];
            assert_ne!(pixel, ink, "bottom cell x={x}");
        }
        assert_eq!(digit_cell_xs(RESULT_LIFE_LAST_X).len(), 8);
        assert_eq!(digit_cell_xs(RESULT_LIFE_LAST_X)[0], 292);
        assert_eq!(digit_cell_xs(RESULT_BOTTOM_LAST_X)[0], 8);
    }

    /// One blank row (every column `None`): the row band keeps its
    /// backdrop, whatever the score carries.
    fn blank_row() -> ResultRankRow {
        ResultRankRow {
            rank: None,
            name: None,
            judgements: None,
            combos: None,
            score: None,
            bonus: None,
            speed: None,
        }
    }

    /// One fully live row: rank 8, HOST, 825/20/0/2, MAX 320 JAM 5,
    /// 110371 points, +50 bonus, `X2` speed.
    fn live_row() -> ResultRankRow {
        ResultRankRow {
            rank: Some(8),
            name: Some("HOST".to_owned()),
            judgements: Some([825, 20, 0, 2]),
            combos: Some((320, 5)),
            score: Some(110_371),
            bonus: Some(50),
            speed: Some(3),
        }
    }

    /// True while white result ink lands in the `width`-wide text
    /// box at a rank-cell origin (glyph top-left cells are often
    /// blank, so scan right and below the pinned left edge; narrow
    /// boxes keep neighbor columns out of absence checks).
    fn row_ink_near(buf: &[u32], origin: (i32, i32), width: i32) -> bool {
        let (ox, oy) = origin;
        for row in oy..oy + 16 {
            for col in ox..ox + width {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < 600 && c < 800 && near_xrgb(buf[r * 800 + c], result_number_ink()) {
                    return true;
                }
            }
        }
        false
    }

    /// True while white result ink lands near a rank-cell origin.
    fn row_white_near(buf: &[u32], origin: (i32, i32)) -> bool {
        row_ink_near(buf, origin, 72)
    }

    #[test]
    fn empty_rank_rows_leave_the_row_band_unpainted() {
        // No row data, no row ink: the band keeps the flat card —
        // none of the pinned row columns (rank 91, name 128, judge
        // 230, combo 416, score 530, bonus 626, speed 685) take GDI
        // ink, even with a full top-panel score behind them.
        for rows in [&[][..], &[blank_row()][..]] {
            let buf = flat_native_rows(&full_score(), rows);
            for origin in [
                (RESULT_RANK_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y),
                (RESULT_ROW_JUDGE_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_COMBO_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_SCORE_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_BONUS_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_SPEED_X, RESULT_ROW_TEXT_Y),
            ] {
                assert!(!row_white_near(&buf, origin), "row cell {origin:?}");
            }
        }
        assert_eq!(RESULT_ROW_PITCH, 22);
        assert_eq!(RESULT_ROW_BASE_Y, 320);
    }

    #[test]
    fn rank_column_paints_the_live_rank_byte() {
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(&buf, (RESULT_RANK_X, RESULT_ROW_TEXT_Y)));
    }

    #[test]
    fn name_column_paints_the_member_nickname() {
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(&buf, (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y)));
    }

    #[test]
    fn judgements_column_paints_the_combined_counts() {
        // Combined `"%5d /%5d /%3d /%3d"` text, like the exe row
        // painter passes it.
        assert_eq!(
            result_judgements_text([825, 20, 0, 2]),
            "  825 /   20 /  0 /  2"
        );
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_JUDGE_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn combos_column_paints_max_then_jam() {
        // Combined `"%4d /%4d"`: MAX COMBO then JAM COMBO order.
        assert_eq!(result_combos_text((320, 5)), " 320 /   5");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_COMBO_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn score_column_paints_the_live_points() {
        assert_eq!(result_score_text(110_371), "  110371");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_SCORE_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn bonus_column_paints_the_hold_bonus() {
        // `"+ %d"` of the live hold accumulator (dossier OPEN 7
        // leaves the semantic open; the value itself is live).
        assert_eq!(result_bonus_text(50), "+ 50");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_BONUS_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn speed_column_paints_the_shared_speed_label() {
        // Speed code 3 reads `X2` through the shared label chain.
        assert_eq!(room_speed_label(3), "X2");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_SPEED_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn long_nicknames_truncate_past_one_hundred_chars() {
        assert_eq!(
            truncate_result_name(&"HOST".repeat(30)).chars().count(),
            100
        );
        assert_eq!(truncate_result_name("HOST"), "HOST");
        let mut row = blank_row();
        row.name = Some("HOST".repeat(30));
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[row]);
        assert!(row_white_near(&buf, (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y)));
    }

    #[test]
    fn peer_rows_paint_names_without_invented_numbers() {
        // Per-member 4014/4016 tallies stay server-side: a peer row
        // carries its name (and speed) while every number column
        // stays unpainted — never zero-filled.
        let peer = ResultRankRow {
            name: Some("HOST".to_owned()),
            speed: Some(3),
            ..blank_row()
        };
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[peer]);
        assert!(row_white_near(&buf, (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y)));
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_SPEED_X, RESULT_ROW_TEXT_Y)
        ));
        // Narrow boxes: the nickname opens at x=128 inside the
        // rank box, and the speed label opens at x=685 inside the
        // bonus box.
        assert!(!row_ink_near(&buf, (RESULT_RANK_X, RESULT_ROW_TEXT_Y), 37));
        assert!(!row_ink_near(
            &buf,
            (RESULT_ROW_BONUS_X, RESULT_ROW_TEXT_Y),
            58
        ));
        for origin in [
            (RESULT_ROW_JUDGE_X, RESULT_ROW_TEXT_Y),
            (RESULT_ROW_COMBO_X, RESULT_ROW_TEXT_Y),
            (RESULT_ROW_SCORE_X, RESULT_ROW_TEXT_Y),
        ] {
            assert!(!row_white_near(&buf, origin), "peer cell {origin:?}");
        }
    }

    #[test]
    fn rank_rows_stack_at_the_22px_pitch() {
        // Row 1 starts 22 px below row 0 on both text lines.
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[blank_row(), live_row()]);
        assert!(!row_white_near(&buf, (RESULT_RANK_X, RESULT_ROW_TEXT_Y)));
        assert!(row_white_near(
            &buf,
            (RESULT_RANK_X, RESULT_ROW_TEXT_Y + 22)
        ));
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y + 22)
        ));
    }

    #[test]
    fn rank_rows_wait_for_the_native_canvas() {
        // Small frames paint no row ink, like the top-panel fields.
        let mut buf = vec![0u32; 70 * 60];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            None,
            &o2jam_game::Score::default(),
            &[live_row()],
            None,
        );
        assert!(!buf.contains(&result_number_ink()));
    }

    #[test]
    fn rank_rows_cap_at_eight_seats() {
        // Nine live rows in, eight paint: row 8 (y 493) stays
        // backdrop while row 7 (y 471) inks.
        let rows = vec![live_row(); 9];
        let buf = flat_native_rows(&o2jam_game::Score::default(), &rows);
        assert!(row_white_near(
            &buf,
            (RESULT_RANK_X, RESULT_ROW_TEXT_Y + 7 * 22)
        ));
        assert!(!row_white_near(
            &buf,
            (RESULT_RANK_X, RESULT_ROW_TEXT_Y + 8 * 22)
        ));
    }

    #[test]
    fn backdrop_keeps_top_and_bottom_panels() {
        // Dossier note 6: the fill never draws the Top/Bottom row
        // objects itself; the per-frame draw-all owns them. The
        // shell backdrop stages their bodies, skipping only the
        // frame bodies (the verdict placement must not paint).
        let art = SceneArt {
            placements: vec![
                ScenePlacement {
                    id: "IDC_TOP".to_owned(),
                    code: 0,
                    sprite: Some(RESULT_TOP_SPRITE.to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                },
                ScenePlacement {
                    id: "IDC_BOTTOM".to_owned(),
                    code: 0,
                    sprite: Some(RESULT_BOTTOM_SPRITE.to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 250,
                        w: 4,
                        h: 4,
                    },
                },
                ScenePlacement {
                    id: "IDC_VERDICT".to_owned(),
                    code: 0,
                    sprite: Some(RESULT_WINLOSE_SPRITE.to_owned()),
                    rect: Rect {
                        x: 400,
                        y: 100,
                        w: 4,
                        h: 4,
                    },
                },
            ],
            figures: HashMap::from([
                (RESULT_TOP_SPRITE.to_owned(), red_figure()),
                (RESULT_BOTTOM_SPRITE.to_owned(), red_figure()),
                (RESULT_WINLOSE_SPRITE.to_owned(), red_figure()),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_result_backdrop(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        let red = pack_xrgb(255, 0, 0);
        assert_eq!(buf[0], red, "top panel paints");
        assert_eq!(buf[250 * 800], red, "bottom panel paints");
        assert_eq!(buf[100 * 800 + 400], 0, "verdict body skipped");
    }

    #[test]
    fn winlose_frames_follow_the_pinned_rule() {
        // `FUN_004c3120`: solo 4; battle flag 1 -> 1, flag 2 -> 2,
        // else 3. The shell resolves the same standing from team
        // totals, so every dossier frame stays reachable.
        assert_eq!(winlose_frame(ResultVerdict::Solo), Some(4));
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Win))),
            Some(1)
        );
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Lose))),
            Some(2)
        );
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Other))),
            Some(3)
        );
        assert_eq!(winlose_frame(ResultVerdict::Battle(None)), None);
    }

    /// Banner art at the pinned image anchors: images 0/1 at (100,
    /// 190), image 2 at (151,190), image 3 at (126,190).
    fn banner_art() -> SceneArt {
        SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                RESULT_WINLOSE_SPRITE.to_owned(),
                frame_body(&[(100, 190), (100, 190), (151, 190), (126, 190)]),
            )]),
        }
    }

    #[test]
    fn solo_result_paints_the_fourth_banner_image() {
        // Frame 4 clamps to image 3, the solo banner at (126,190);
        // the battle images never paint.
        let buf = painted_native_scene(&banner_art(), ResultVerdict::Solo, None);
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(126, 190), frame_ink(3));
        assert_ne!(at(100, 190), frame_ink(0));
        assert_ne!(at(151, 190), frame_ink(2));
    }

    #[test]
    fn battle_win_paints_the_first_banner_image() {
        let buf = painted_native_scene(
            &banner_art(),
            ResultVerdict::Battle(Some(BattleVerdict::Win)),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(100, 190), frame_ink(0));
        assert_ne!(at(126, 190), frame_ink(3));
        assert_ne!(at(151, 190), frame_ink(2));
    }

    #[test]
    fn battle_lose_paints_the_second_banner_image() {
        let buf = painted_native_scene(
            &banner_art(),
            ResultVerdict::Battle(Some(BattleVerdict::Lose)),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(100, 190), frame_ink(1));
        assert_ne!(at(100, 190), frame_ink(0));
        assert_ne!(at(151, 190), frame_ink(2));
        assert_ne!(at(126, 190), frame_ink(3));
    }

    #[test]
    fn battle_other_paints_the_third_banner_image() {
        let buf = painted_native_scene(
            &banner_art(),
            ResultVerdict::Battle(Some(BattleVerdict::Other)),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(151, 190), frame_ink(2));
        assert_ne!(at(100, 190), frame_ink(0));
        assert_ne!(at(100, 190), frame_ink(1));
        assert_ne!(at(126, 190), frame_ink(3));
    }

    #[test]
    fn unresolved_battle_paints_no_banner() {
        let buf = painted_native_scene(&banner_art(), ResultVerdict::Battle(None), None);
        for ink in [frame_ink(0), frame_ink(1), frame_ink(2), frame_ink(3)] {
            assert!(!buf.contains(&ink));
        }
    }

    /// Grade plate art: eight frames all anchored (395,531), each a
    /// distinct solid ink.
    fn rank_art() -> SceneArt {
        SceneArt {
            placements: vec![ScenePlacement {
                id: "IDC_IMAGE_SRESULT_RANKINGFIELD".to_owned(),
                code: 0x1750_0007,
                sprite: Some(RESULT_RANK_SPRITE.to_owned()),
                rect: Rect {
                    x: 395,
                    y: 531,
                    w: 79,
                    h: 41,
                },
            }],
            figures: HashMap::from([(RESULT_RANK_SPRITE.to_owned(), frame_body(&[(395, 531); 8]))]),
        }
    }

    #[test]
    fn grade_thresholds_climb_the_pinned_table() {
        // `FUN_004c4dd0` f64 compares at 0.30/0.50/0.70/0.80/0.90/
        // 0.95, each lifting the plate one frame.
        let cases = [
            (0.29, 0u8),
            (0.30, 1),
            (0.49, 1),
            (0.50, 2),
            (0.69, 2),
            (0.70, 3),
            (0.79, 3),
            (0.80, 4),
            (0.89, 4),
            (0.90, 5),
            (0.949, 5),
            (0.95, 6),
            (1.0, 6),
        ];
        for (ratio, frame) in cases {
            assert_eq!(grade_frame(ratio, false, false), frame, "ratio {ratio}");
        }
    }

    #[test]
    fn grade_promotes_combo_top_and_max_score() {
        // Max combo == note count - 1 promotes grade 6 only; a score
        // equal to the max reference gives 8 outright.
        assert_eq!(grade_frame(0.95, true, false), 7);
        assert_eq!(grade_frame(0.90, true, false), 5);
        assert_eq!(grade_frame(0.30, false, true), 8);
    }

    #[test]
    fn grade_plate_paints_the_selected_frame_at_its_pinned_anchor() {
        // Grade 3 paints image 2 at (395,531); the wider max-score
        // image stays out.
        let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, Some(3));
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(395, 531), frame_ink(2));
        assert_ne!(at(395, 531), frame_ink(7));
    }

    #[test]
    fn sub_threshold_grades_keep_the_plate_off() {
        // Grade 0 (ratio under 0.30) and no ratio at all paint no
        // letter: the enter painter's frame 0 stays the empty state.
        for grade in [None, Some(0)] {
            let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, grade);
            for index in 0..8 {
                assert!(!buf.contains(&frame_ink(index)));
            }
        }
    }

    #[test]
    fn frame_bodies_never_stack_through_the_backdrop() {
        // The rank placement binds rect 7 like the real block: the
        // whole-layer backdrop pass must skip the body (its last
        // layer would win the stack), leaving only the selected
        // frame.
        let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, Some(1));
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(395, 531), frame_ink(0), "grade 1 paints image 0");
        assert_ne!(at(395, 531), frame_ink(7), "the stack's last layer");
    }

    #[test]
    fn result_scene_paints_staged_art_without_the_invented_verdict() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 64];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 64,
            },
            &ResultPaint {
                avatar: None,
                score: &o2jam_game::Score::default(),
                rows: &[],
                art: &art,
                verdict: ResultVerdict::Battle(Some(BattleVerdict::Win)),
                grade: None,
                album: None,
            },
        );
        // The staged art paints (the corner banner body rides frame 1
        // onto its clamped layer); the retired amber caption ink
        // never comes back.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    /// Red/green 2x1 cover stand-in: the widened inks probe pixels
    /// (5-bit channels widened by replication, so red lands (255,0,0)).
    fn tiny_cover() -> Rgb555Image {
        Rgb555Image {
            w: 2,
            h: 1,
            words: vec![0x7C00, 0x03E0],
        }
    }

    #[test]
    fn album_cover_paints_into_the_music_name_box() {
        // The OJN cover section (game-resolved) stretches opaque over
        // the box interior ([RESULT_MUSIC_BOX]): the left half of the
        // 2x1 cover covers the box's left columns, the right half its
        // right column, and the frame border pixel left of the box
        // stays card ink.
        let mut buf = vec![0u32; 800 * 600];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            &o2jam_game::Score::default(),
            &[],
            Some(&tiny_cover()),
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(70, 304), pack_xrgb(255, 0, 0));
        assert_eq!(at(733, 518), pack_xrgb(0, 255, 0));
        assert_eq!(at(60, 304), pack_xrgb(0x10, 0x10, 0x08));
    }

    #[test]
    fn missing_cover_paints_the_box_as_is() {
        // No resolved cover: the box region keeps whatever the scene
        // stages — the flat card here — with zero album pixels.
        let without = {
            let mut buf = vec![0u32; 800 * 600];
            paint_result(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                None,
                &o2jam_game::Score::default(),
                &[],
                None,
            );
            buf
        };
        let with = {
            let mut buf = vec![0u32; 800 * 600];
            paint_result(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                None,
                &o2jam_game::Score::default(),
                &[],
                Some(&tiny_cover()),
            );
            buf
        };
        let at = |buf: &[u32], x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(&without, 70, 304), pack_xrgb(0x10, 0x10, 0x08));
        assert_eq!(at(&with, 60, 304), at(&without, 60, 304));
    }

    #[test]
    fn album_cover_scales_into_small_frames() {
        // Non-native frames scale the box rect like the placements;
        // the cover still fills it (the window composes at native and
        // scales once, so this is the small-probe path only).
        let mut buf = vec![0u32; 64 * 48];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            None,
            &o2jam_game::Score::default(),
            &[],
            Some(&tiny_cover()),
        );
        let dest = scale_rect_to_frame(&RESULT_MUSIC_BOX, 64, 48);
        let at = |x: i32, y: i32| buf[y as usize * 64 + x as usize];
        assert_eq!(at(dest.x, dest.y), pack_xrgb(255, 0, 0));
        let last_x = dest.x + dest.w as i32 - 1;
        let last_y = dest.y + dest.h as i32 - 1;
        assert_eq!(at(last_x, last_y), pack_xrgb(0, 255, 0));
    }

    /// Ten-layer number font: layer `i` paints digit `i`.
    fn digit_font_figure() -> AvatarFigure {
        let words = [
            0x7C00u16, 0x03E0, 0x001F, 0x7FFF, 0x0000, 0x7FE0, 0x03FF, 0x7C1F, 0x4210, 0x2108,
        ];
        let mut figure = AvatarFigure::default();
        for word in words {
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.to_le_bytes(), 1, 1).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    /// `ROOMNO` digit places in real file order (ones first).
    fn room_number_placements() -> Vec<ScenePlacement> {
        [
            ("IDC_IMAGE_SROOM_ROOMNO1", 30),
            ("IDC_IMAGE_SROOM_ROOMNO10", 20),
            ("IDC_IMAGE_SROOM_ROOMNO100", 10),
        ]
        .into_iter()
        .map(|(id, x)| ScenePlacement {
            id: id.to_owned(),
            code: 0x1350_0001,
            sprite: Some("Room_Number.ojs".to_owned()),
            rect: Rect {
                x,
                y: 10,
                w: 4,
                h: 4,
            },
        })
        .collect()
    }

    #[test]
    fn digit_place_weights_read_trailing_runs() {
        assert_eq!(digit_place_weight("IDC_IMAGE_SROOM_ROOMNO1"), Some(1));
        assert_eq!(digit_place_weight("IDC_IMAGE_SROOM_ROOMNO10"), Some(10));
        assert_eq!(digit_place_weight("IDC_IMAGE_SROOM_ROOMNO100"), Some(100));
        assert_eq!(digit_place_weight("IDC_IMAGE_SWAIT_CHNUM_01"), Some(1));
        assert_eq!(digit_place_weight("IDC_IMAGE_SWAIT_RMNUM_010"), Some(10));
        assert_eq!(digit_place_weight("IDC_IMAGE_SWAIT_SPEED_NUM"), None);
        assert!(is_place_weight(1));
        assert!(is_place_weight(10));
        assert!(is_place_weight(100));
        assert!(!is_place_weight(0));
        assert!(!is_place_weight(8));
        assert!(!is_place_weight(20));
    }

    #[test]
    fn font_numbers_blits_weighted_digits_into_place_rects() {
        // Value 12 over ones-first places: weight picks the digit,
        // not file order — hundreds red (0), tens green (1), ones
        // blue (2).
        let placements = room_number_placements();
        let figures = HashMap::from([("Room_Number.ojs".to_owned(), digit_font_figure())]);
        let mut buf = vec![0u32; 800 * 600];
        paint_font_number(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &figures,
            "Room_Number.ojs",
            12,
        );
        assert_eq!(buf[10 * 800 + 10], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 800 + 20], pack_xrgb(0, 255, 0));
        assert_eq!(buf[10 * 800 + 30], pack_xrgb(0, 0, 255));
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn font_digits_add_their_anchor_to_the_place_rect() {
        // Verified draw math at the font painter too: the place rect
        // positions and the digit layer's anchor adds — anchor (2,1)
        // moves the digit off the rect corner, and nothing clips.
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&0x7C00u16.to_le_bytes(), 1, 1).unwrap(),
            dx: 2,
            dy: 1,
            colorkey: 0xFFFF,
        });
        let placements = vec![ScenePlacement {
            id: "IDC_IMAGE_SROOM_ROOMNO100".to_owned(),
            code: 0x1350_0001,
            sprite: Some("Room_Number.ojs".to_owned()),
            rect: Rect {
                x: 10,
                y: 10,
                w: 4,
                h: 4,
            },
        }];
        let figures = HashMap::from([("Room_Number.ojs".to_owned(), figure)]);
        let mut buf = vec![0u32; 800 * 600];
        paint_font_number(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &figures,
            "Room_Number.ojs",
            5,
        );
        assert_eq!(buf[11 * 800 + 12], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 800 + 10], 0);
    }

    #[test]
    fn font_numbers_skip_missing_fonts_and_layers() {
        let placements = room_number_placements();
        let mut buf = vec![0x123456u32; 64 * 48];
        paint_font_number(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &placements,
            &HashMap::new(),
            "Room_Number.ojs",
            12,
        );
        // Missing font paints nothing at all.
        assert_eq!(buf, vec![0x123456u32; 64 * 48]);
        // Short font (one layer): only the hundreds digit (layer 0)
        // paints — the tens and ones places need layers 1 and 2 and
        // stay untouched. The place rects scale to zero width on the
        // small frame, so the origin lands at the frame corner.
        let short = HashMap::from([("Room_Number.ojs".to_owned(), red_figure())]);
        paint_font_number(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &placements,
            &short,
            "Room_Number.ojs",
            12,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[1], 0x123456);
        assert_eq!(buf[2], 0x123456);
    }

    #[test]
    fn sequential_suffixes_are_frames_not_fonts() {
        // `SHADOW1`…`8` share one body but never form a number.
        let placements: Vec<ScenePlacement> = (1..=8)
            .map(|n| ScenePlacement {
                id: format!("IDC_IMAGE_SPLAY_SHADOW{n}"),
                code: 0x1B50_0000 + n,
                sprite: Some("Avatar_Shadow.oji".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 8,
                    h: 8,
                },
            })
            .collect();
        assert!(staged_number_fonts(&placements).is_empty());
        assert_eq!(
            staged_number_fonts(&room_number_placements()),
            vec!["Room_Number.ojs"]
        );
    }

    #[test]
    fn room_scene_paints_room_number_through_staged_font() {
        let art = SceneArt {
            placements: room_number_placements(),
            figures: HashMap::from([("Room_Number.ojs".to_owned(), digit_font_figure())]),
        };
        let mut rooms = two_rooms();
        rooms[0].number = 12;
        let mut buf = vec![0u32; 800 * 600];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rooms,
            0,
            &art,
        );
        assert_eq!(buf[10 * 800 + 10], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 800 + 20], pack_xrgb(0, 255, 0));
        assert_eq!(buf[10 * 800 + 30], pack_xrgb(0, 0, 255));
    }

    /// Multi-layer icon body: layer `i` paints one solid RGB555
    /// `words[i]` image (colorkey white, so every pixel paints).
    fn icon_figure(words: &[u16]) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        for word in words {
            let pixels = vec![word.to_le_bytes(); 40 * 20].concat();
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&pixels, 40, 20).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    /// The three lobby row-icon placements at their `State_Room.bnd`
    /// template rects, with a distinct-layer figure per icon (green,
    /// red, blue, white in frame order; last layers feed the backdrop
    /// pass, selected frames the icon pass).
    fn room_icon_art() -> SceneArt {
        let placed = |id: &str, code: u32, sprite: &str, x: i32, y: i32| ScenePlacement {
            id: id.to_owned(),
            code,
            sprite: Some(sprite.to_owned()),
            rect: Rect { x, y, w: 40, h: 20 },
        };
        SceneArt {
            placements: vec![
                placed(
                    "IDC_IMAGE_SROOM_MODE",
                    0x1350_0028,
                    ROOM_MODE_SPRITE,
                    400,
                    280,
                ),
                placed("IDC_IMAGE_SROOM_OM", 0x1350_0029, ROOM_OM_SPRITE, 450, 280),
                placed(
                    "IDC_IMAGE_SROOM_STATE",
                    0x1350_002D,
                    ROOM_STATE_SPRITE,
                    500,
                    300,
                ),
            ],
            figures: HashMap::from([
                (
                    ROOM_MODE_SPRITE.to_owned(),
                    icon_figure(&[0x03E0, 0x7C00, 0x001F]),
                ),
                (
                    ROOM_OM_SPRITE.to_owned(),
                    icon_figure(&[0x03E0, 0x7C00, 0x001F, 0x7FE0]),
                ),
                (
                    ROOM_STATE_SPRITE.to_owned(),
                    icon_figure(&[0x03E0, 0x7C00, 0x001F]),
                ),
            ]),
        }
    }

    /// One room row overriding the listed fields.
    fn room_with(state_code: u8, key_mode: u8, music_id: u16) -> RoomListEntry {
        RoomListEntry {
            state_code,
            key_mode,
            music_id,
            ..two_rooms()[0].clone()
        }
    }

    #[test]
    fn icon_frame_layers_clamp_like_the_exe_setter() {
        // Setter `0x2f870`: frame n paints image n−1, saturating at 0
        // for frame 0 and clamping at the last decoded layer.
        assert_eq!(icon_frame_layer(0, 2), Some(0));
        assert_eq!(icon_frame_layer(1, 2), Some(0));
        assert_eq!(icon_frame_layer(2, 2), Some(1));
        assert_eq!(icon_frame_layer(9, 2), Some(1));
        assert_eq!(icon_frame_layer(1, 4), Some(0));
        assert_eq!(icon_frame_layer(4, 4), Some(3));
        assert_eq!(icon_frame_layer(0, 0), None);
    }

    #[test]
    fn room_state_icon_frame_tracks_the_state_byte() {
        assert_eq!(room_state_icon_frame(0), 0);
        assert_eq!(room_state_icon_frame(1), 1);
        assert_eq!(room_state_icon_frame(2), 2);
    }

    #[test]
    fn room_key_icon_frames_follow_the_3_and_7_key_rule() {
        // Icon sites `0x1267ad`–`0x1267e2`: key 0 → frame 1, key 2 →
        // frame 2, anything else untouched.
        assert_eq!(room_key_icon_frame(0), Some(1));
        assert_eq!(room_key_icon_frame(2), Some(2));
        assert_eq!(room_key_icon_frame(1), None);
        assert_eq!(room_key_icon_frame(9), None);
    }

    #[test]
    fn room_om_icon_bands_follow_the_level_mapper() {
        // Mapper `0x126cb0` via `om_icon_frame`: 0–4 → 1, 5–8 → 2,
        // 9–12 → 3, 13+ → 4.
        for (level, frame) in [
            (0u16, 1),
            (4, 1),
            (5, 2),
            (8, 2),
            (9, 3),
            (12, 3),
            (13, 4),
            (99, 4),
        ] {
            assert_eq!(room_om_icon_frame(200, Some(level)), Some(frame), "{level}");
        }
    }

    #[test]
    fn room_om_icon_shows_the_no_song_frame_below_the_music_floor() {
        // Floor compare `0x1251da` + constant level 1 (`0x1266b9`):
        // below the floor the row shows frame 1; past it an unknown
        // level (no staged song table) paints nothing.
        assert_eq!(room_om_icon_frame(0, None), Some(1));
        assert_eq!(room_om_icon_frame(ROOM_MUSIC_FLOOR - 1, None), Some(1));
        assert_eq!(room_om_icon_frame(ROOM_MUSIC_FLOOR, None), None);
        assert_eq!(room_om_icon_frame(u16::MAX, None), None);
    }

    #[test]
    fn room_icons_paint_their_frames_at_the_rect_origins() {
        // State 2 (frame 2 → layer 1 red), key 2 (frame 2 → layer 1
        // red), no song (frame 1 → layer 0 green): each at its own
        // template rect origin, none at the frame corner.
        let art = room_icon_art();
        let rooms = vec![room_with(2, 2, 0)];
        let mut buf = vec![0u32; 800 * 600];
        paint_room_row_icons(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            &rooms,
            0,
        );
        let red = pack_xrgb(255, 0, 0);
        let green = pack_xrgb(0, 255, 0);
        assert_eq!(buf[300 * 800 + 500], red, "state icon frame");
        assert_eq!(buf[280 * 800 + 400], red, "key-mode icon frame");
        assert_eq!(buf[280 * 800 + 450], green, "om icon frame");
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn room_icons_skip_untouched_frames_and_missing_art() {
        // Key 1 leaves the mode icon untouched (the backdrop's last
        // layer stays), a song without a staged level leaves the om
        // icon untouched, and an unstaged body paints nothing.
        let mut art = room_icon_art();
        art.figures.remove(ROOM_STATE_SPRITE);
        let rooms = vec![room_with(1, 1, ROOM_MUSIC_FLOOR)];
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        art.paint_overlay(&mut frame);
        paint_room_row_icons(&mut frame, &art, &rooms, 0);
        let blue = pack_xrgb(0, 0, 255);
        let yellow = pack_xrgb(255, 255, 0);
        assert_eq!(buf[280 * 800 + 400], blue, "mode icon untouched");
        assert_eq!(buf[280 * 800 + 450], yellow, "om icon untouched");
        assert_eq!(buf[300 * 800 + 500], 0, "missing state body paints nothing");
    }

    #[test]
    fn room_scene_layers_icons_under_the_detail_ink() {
        // Full pipeline: the backdrop paints every layer, the icon
        // pass overlays the selected frames, and the detail text
        // draws last — the icon survives beside the small text scrim
        // (asserted inside the icon box, clear of the scrim).
        let art = room_icon_art();
        let rooms = vec![room_with(2, 2, 0)];
        let mut buf = vec![0u32; 800 * 600];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rooms,
            0,
            &art,
        );
        let red = pack_xrgb(255, 0, 0);
        assert_eq!(buf[315 * 800 + 538], red, "state icon at its rect");
        assert_eq!(buf[295 * 800 + 438], red, "key-mode icon at its rect");
    }

    #[test]
    fn entry_less_mounted_art_paints_at_its_authored_anchor_not_the_canvas() {
        // Entry-less mounted art (`Dialog_ChannelSelectBar.ojs` anchors at
        // `(0, 0)`, 347x41 on 7.55) paints 1:1 at that authored anchor —
        // never stretched over the canvas. It is dialog art, not part of
        // the `STATE_CHANNEL` scene (which names zero sprites and stages
        // nothing).
        let art = SceneArt {
            placements: unbound_scene_placements(&[DIALOG_BAR_SPRITE.to_owned()]),
            figures: HashMap::from([(DIALOG_BAR_SPRITE.to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 128 * 96];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 128,
                h: 96,
            },
            &two_channels(),
            0,
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0), "the authored anchor");
        assert_eq!(buf[50 * 128 + 100], 0, "no canvas-wide stretch");
    }

    #[test]
    fn loading_gate_paints_its_full_native_rect() {
        // `MusicLoading.ojs` is one 800x600 body at anchor (0,0):
        // staged unbound it covers exactly its native extent at 1:1 —
        // the art's own geometry is the rect, not a claimed scaler.
        let pixels = [[0x00, 0x7C]; 4 * 3].concat();
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&pixels, 4, 3).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let art = SceneArt {
            placements: unbound_scene_placements(&[LOADING_SPRITE.to_owned()]),
            figures: HashMap::from([(LOADING_SPRITE.to_owned(), figure)]),
        };
        let mut buf = vec![0u32; 32 * 24];
        paint_loading_scene(
            &mut Frame {
                buf: &mut buf,
                w: 32,
                h: 24,
            },
            &art,
        );
        let red = pack_xrgb(255, 0, 0);
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(buf[y * 32 + x], red, "native rect pixel ({x},{y})");
            }
        }
        assert_eq!(buf[4], 0, "nothing past the body's own extent");
    }

    #[test]
    fn loading_fill_spans_the_walk_fraction() {
        // Fractional bar: the track stays put, the green fill spans
        // `progress` of the `w/2` inner width (32x24 frame: track
        // x8 w16, half fill x8..16 at 0.5).
        let art = SceneArt::default();
        let status = LoadingStatus {
            progress: 0.5,
            title: String::new(),
            artist: String::new(),
        };
        let mut buf = vec![0u32; 32 * 24];
        paint_loading_scene_with_status(
            &mut Frame {
                buf: &mut buf,
                w: 32,
                h: 24,
            },
            &art,
            &status,
        );
        let green = pack_xrgb(0x30, 0xB0, 0x30);
        assert_eq!(buf[4 * 32 + 8], green, "fill starts at the track");
        assert_eq!(buf[4 * 32 + 15], green, "half fill reaches x15");
        assert_ne!(buf[4 * 32 + 16], green, "nothing past the fraction");
    }

    #[test]
    fn loading_fill_clamps_out_of_range_progress() {
        // NaN/negative/over-one fractions saturate to empty/full —
        // never a wrapped or inverted fill (PERF-25: no `==`).
        assert_eq!(loading_fill_width(16, f32::NAN), 0);
        assert_eq!(loading_fill_width(16, -0.5), 0);
        assert_eq!(loading_fill_width(16, 0.0), 0);
        assert_eq!(loading_fill_width(16, 2.0), 16);
        assert_eq!(loading_fill_width(16, 1.0), 16);
    }

    #[test]
    fn loading_caption_joins_title_and_artist() {
        // Song-table walk names the song: both halves join, one
        // half alone still names itself, blank paints nothing.
        let both = LoadingStatus {
            progress: 0.0,
            title: "Title".to_owned(),
            artist: "Artist".to_owned(),
        };
        assert_eq!(both.caption(), Some("Title - Artist".to_owned()));
        let title_only = LoadingStatus {
            title: "Title".to_owned(),
            ..LoadingStatus::default()
        };
        assert_eq!(title_only.caption(), Some("Title".to_owned()));
        assert_eq!(LoadingStatus::default().caption(), None);
    }

    #[test]
    fn loading_scene_paints_the_song_caption() {
        // Caption row below the bar (placeholder row 32x24: x8,
        // y24 clipped — so use 64x48: song at y36, scrim behind).
        let art = SceneArt::default();
        let status = LoadingStatus {
            progress: 1.0,
            title: "Title".to_owned(),
            artist: "Artist".to_owned(),
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_loading_scene_with_status(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
            &status,
        );
        assert!(has_ink_near(&buf, art_title_ink()), "caption ink paints");
        assert!(buf.contains(&scrim_color()), "caption scrim paints");
        let blank = vec![0u32; 64 * 48];
        let mut probe = blank.clone();
        paint_loading_scene_with_status(
            &mut Frame {
                buf: &mut probe,
                w: 64,
                h: 48,
            },
            &art,
            &LoadingStatus::default(),
        );
        assert!(
            !probe.contains(&art_title_ink()),
            "blank caption paints no white ink"
        );
    }

    #[test]
    fn loading_bar_geometry_stays_open_placeholder() {
        // OPEN placeholder guard: the centered track/caption math
        // below is not exe-grounded (`MusicLoading.ojs` is a single
        // 800x600 body, `0x4e47a0` names no rect, adjacent bars are
        // other dialogs), so this test pins the placeholder formula
        // and fails loudly if anyone "grounds" it without evidence.
        let frame = Frame {
            buf: &mut vec![0u32; 64 * 48],
            w: 64,
            h: 48,
        };
        assert_eq!(loading_track_origin(&frame), (16, 16));
        assert_eq!(loading_track_width(&frame), 32);
        assert_eq!(loading_song_origin(&frame), (16, 36));
    }

    #[test]
    fn waiting_covers_paint_per_seat_rects() {
        // `Wait_Cover_Avatar.ojs` anchors at (0,0): its eight rect-
        // bound placements (COVERAVATAR1..8, codes 0x1650003D..44)
        // each paint at their own rect origin — the view-from-rect
        // case ui-draw.md leaves open, resolved for anchor-free
        // bodies. Occupied covers then take the live seat line (the
        // scrim covers the art pixel); empty covers stay untouched
        // so the staged empty panels show.
        let placements = (0..8)
            .map(|seat| ScenePlacement {
                id: format!("IDC_IMAGE_SWAIT_COVERAVATAR{}", seat + 1),
                code: 0x1650_003D + seat as u32,
                sprite: Some("Wait_Cover_Avatar.ojs".to_owned()),
                rect: Rect {
                    x: 10 + seat * 60,
                    y: 200 + seat * 45,
                    w: 141,
                    h: 175,
                },
            })
            .collect();
        let art = SceneArt {
            placements,
            figures: HashMap::from([("Wait_Cover_Avatar.ojs".to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
        );
        let red = pack_xrgb(255, 0, 0);
        // Empty seat 2 keeps its staged art pixel: no live line.
        assert_eq!(buf[290 * 800 + 130], red, "empty cover stays art-only");
        // Occupied seat 0 takes the live line: scrim opens at
        // (10 − 4, 200 − 2), covering the art pixel.
        assert_eq!(buf[198 * 800 + 6], scrim_color(), "seat line scrim");
        assert_ne!(buf[200 * 800 + 10], red, "live line covers art");
        assert!(has_ink_near(&buf, art_title_ink()));
        assert_eq!(buf[0], 0, "covers no longer stack at the frame corner");
    }

    /// Verbatim `State_Waiting.bnd` rects for one region probe (local
    /// 116-rect walk; ordinal = `(code & 0xFFF) − 1`).
    fn waiting_probe(id: &str, sprite: Option<&str>, rect: (i32, i32, i32, i32)) -> ScenePlacement {
        ScenePlacement {
            id: id.to_owned(),
            code: 0,
            sprite: sprite.map(str::to_owned),
            rect: Rect {
                x: rect.0,
                y: rect.1,
                w: u32::try_from(rect.2 - rect.0).unwrap_or(0),
                h: u32::try_from(rect.3 - rect.1).unwrap_or(0),
            },
        }
    }

    /// Waiting room with every staged field set: champion seat 1,
    /// streak 3, song 757, speed code 5 (`X3`), one skill ring.
    fn champion_room() -> JoinedRoom {
        let mut room = waiting_room();
        room.champion = Some(1);
        room.champion_streak = Some(3);
        room.music_id = 757;
        room.speed = 5;
        room.skills = vec![2101];
        room
    }

    #[test]
    fn staged_covers_name_champion_and_challenger() {
        // Cover rects 60/61 `(28,53,169,228)`/`(170,53,311,228)`:
        // seat 1 (wire host) names CHAMPION with its wire level,
        // seat 0 (self, no wire level) names CHALLENGER with `-`.
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR1", None, (28, 53, 169, 228)),
            waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR2", None, (170, 53, 311, 228)),
        ];
        let mut buf = vec![background_of(Screen::Waiting); 800 * 600];
        paint_waiting_covers(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[51 * 800 + 166], scrim_color(), "champion scrim");
        assert_eq!(buf[51 * 800 + 24], scrim_color(), "challenger scrim");
        assert!(has_ink_near(&buf, art_title_ink()));
        // Ready dots ride the cover lines: host ready green, self dim.
        assert_eq!(buf[55 * 800 + 165], pack_xrgb(0x30, 0xE0, 0x30));
        assert_eq!(buf[55 * 800 + 23], pack_xrgb(0x50, 0x50, 0x50));
    }

    #[test]
    fn empty_covers_leave_the_staged_panels_untouched() {
        // Cover rect 62 `(312,53,453,228)` is seat 2 (empty wire
        // slot): no scrim, no dot — the staged empty panel shows.
        let placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_COVERAVATAR3",
            None,
            (312, 53, 453, 228),
        )];
        let calm = background_of(Screen::Waiting);
        let mut buf = vec![calm; 800 * 600];
        paint_waiting_covers(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[53 * 800 + 312], calm);
        assert!(!buf.contains(&scrim_color()));
    }

    #[test]
    fn team_highlight_marks_our_team_button() {
        // Team rects 3/4 `(652,65,676,89)`/`(682,65,706,89)` with
        // their staged `Wait_Team?.ojs` bodies: our team (1 = B)
        // takes the amber bar, A stays art-only.
        let placements = vec![
            waiting_probe(
                "IDC_BUTTON_SWAIT_TEAMA",
                Some("Wait_TeamA.ojs"),
                (652, 65, 676, 89),
            ),
            waiting_probe(
                "IDC_BUTTON_SWAIT_TEAMB",
                Some("Wait_TeamB.ojs"),
                (682, 65, 706, 89),
            ),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_team_highlight(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[65 * 800 + 682], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[65 * 800 + 652], 0);
    }

    #[test]
    fn wins_box_shows_the_champion_streak() {
        // `WINNERSCORE` rect 108 `(190,95,240,115)`: the wire
        // streak only; per-member tallies stay server-side.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_WINNERSCROE",
            None,
            (190, 95, 240, 115),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_wins(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[93 * 800 + 186], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn wins_box_stays_empty_without_a_champion() {
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_WINNERSCROE",
            None,
            (190, 95, 240, 115),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_wins(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &placements,
        );
        assert!(!buf.contains(&scrim_color()));
    }

    #[test]
    fn speed_value_anchors_to_the_speed_num_rect() {
        // `SPEED_NUM` rect 101 `(681,378,735,413)`: speed code 5
        // reads `X3` through the shared label chain (reference row).
        let placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_SPEED_NUM",
            None,
            (681, 378, 735, 413),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_speed(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[376 * 800 + 677], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn skill_slots_show_staged_ring_ids_only() {
        // `1SKILL` rect 87 `(701,208,731,238)`: the staged ring id;
        // `2SKILL` (no second ring) stays untouched X art.
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_1SKILL", None, (701, 208, 731, 238)),
            waiting_probe("IDC_IMAGE_SWAIT_2SKILL", None, (701, 246, 731, 276)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_skills(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[206 * 800 + 697], scrim_color());
        assert_eq!(buf[246 * 800 + 701], 0);
    }

    #[test]
    fn song_title_anchors_to_the_musicname_rect() {
        // `MUSICNAME` rect 22 `(573,15,773,38)`: bare song id while
        // the staged catalog names no such song (the song table
        // lives server-side, as in the lobby).
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_MUSICNAME",
            None,
            (573, 15, 773, 38),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_music(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
            None,
        );
        assert_eq!(buf[13 * 800 + 569], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_music_caption_prefers_the_catalog_title() {
        // Room-fill string flow: the staged catalog names the
        // room's song, so `MUSICNAME` carries the title.
        assert_eq!(waiting_music_caption(757, Some("First")), "First");
    }

    #[test]
    fn waiting_music_caption_falls_back_to_song_id() {
        // No staged title (or a blank one): the bare id, never an
        // invented name.
        assert_eq!(waiting_music_caption(757, None), "M757");
        assert_eq!(waiting_music_caption(757, Some("")), "M757");
        assert_eq!(waiting_music_caption(0, None), "M0");
    }

    #[test]
    fn waiting_musicname_paints_the_catalog_title() {
        // Same rect as the id fallback, but the staged title ink:
        // the two captions share the anchor, the source differs.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_MUSICNAME",
            None,
            (573, 15, 773, 38),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_music(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
            Some("First"),
        );
        assert_eq!(buf[13 * 800 + 569], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn staged_seat_text_shows_level_and_name() {
        // Room-fill `Lv.%d:%s` flow: role prefix, team letter, wire
        // level, nickname, ready flag.
        let champion = StagedSeat {
            name: "host".to_owned(),
            level: Some(9),
            team: 0,
            ready: true,
            present: true,
            locked: false,
            champion: true,
        };
        assert_eq!(
            staged_seat_text(&champion, Some("CHAMPION")),
            "CHAMPION A Lv:9 host R"
        );
        assert_eq!(staged_seat_text(&champion, None), "A Lv:9 host R");
        let challenger = StagedSeat {
            name: "YOU".to_owned(),
            level: None,
            team: 1,
            ready: false,
            present: true,
            locked: false,
            champion: false,
        };
        assert_eq!(
            staged_seat_text(&challenger, Some("CHALLENGER")),
            "CHALLENGER B Lv:- YOU -"
        );
    }

    #[test]
    fn waiting_chat_box_paints_no_history_without_a_source() {
        // Chat controls (`EDIT_SWAIT_CHAT` rect 19 `(20,563,460,583)`,
        // `LIST_SWAIT_CHAT` rect 42 `(24,455,511,553)`) name the history
        // view, but no text source reaches the painter: waiting-room chat
        // rides 3011/3012/3013 (opcode numbers only), `o2jam-net` decodes
        // none, and `JoinedRoom` carries no chat field — so staged chat
        // rects stay untouched instead of invented.
        let placements = vec![
            waiting_probe("IDC_LIST_SWAIT_CHAT", None, (24, 455, 511, 553)),
            waiting_probe("IDC_EDIT_SWAIT_CHAT", None, (20, 563, 460, 583)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
            &WaitingCaptions::default(),
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn waiting_chat_row_text_right_aligns_the_sender_in_a_12_wide_field() {
        // The enqueue printf formats `[%12s] %s` (format VA `0x64d8d4`,
        // call file `0x15f076`, `docs/channel-fill.md`): bracket, sender
        // right-aligned to width 12, bracket, space, text.
        let row = WaitingChatMessage::user("host", "hello");
        assert_eq!(waiting_chat_row_text(&row), "[        host] hello");
    }

    #[test]
    fn waiting_chat_rows_stack_at_the_reference_pitch() {
        // Reference rows sit 17 native px apart (rows at y 455.4,
        // 472.5, 489.8 in `reference-waiting-4x3.png`); six 17 px
        // slots fill the 98 px `LIST` rect exactly.
        assert_eq!(waiting_chat_row_y(455, 0), 455);
        assert_eq!(waiting_chat_row_y(455, 1), 472);
        assert_eq!(waiting_chat_row_y(455, WAITING_CHAT_VISIBLE_ROWS - 1), 540);
    }

    #[test]
    fn waiting_chat_row_text_brackets_admin_rows_like_user_rows() {
        // The 3013 stub enqueues the same `[%12s] %s` line; only the
        // appended ink differs (`0x71e6` vs `0xdedede`). The `[ADMIN]`
        // stand-in prefix is gone.
        let row = WaitingChatMessage::admin("op", "quiet");
        assert_eq!(waiting_chat_row_text(&row), "[          op] quiet");
    }

    #[test]
    fn waiting_chat_without_live_rows_paints_only_the_caret() {
        // No packets → no history (the no-invention pin); the input
        // line still shows its focus caret (`reference-waiting-4x3.png`
        // shows exactly this state: empty history-less rows, caret in
        // the edit rect) — a 2x10 white bar inset (5,5) of rect 19.
        let placements = vec![
            waiting_probe("IDC_LIST_SWAIT_CHAT", None, (24, 455, 511, 553)),
            waiting_probe("IDC_EDIT_SWAIT_CHAT", None, (20, 563, 460, 583)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            "",
            &placements,
        );
        let lit: Vec<usize> = buf
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel != 0)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(lit.len(), 20, "only the 2x10 caret may light");
        for index in lit {
            assert_eq!(buf[index], art_title_ink());
            let (x, y) = (index % 800, index / 800);
            assert!((25..=26).contains(&x), "caret x {x}");
            assert!((568..=577).contains(&y), "caret y {y}");
        }
    }

    #[test]
    fn waiting_chat_paints_user_rows_in_the_exe_gray_at_the_list_rect() {
        // The 3012 append ink is COLORREF `0xdedede` (call file
        // `0xb9f8c`, `docs/channel-fill.md`) — light gray, drawn
        // straight on the box art (no per-row scrim band).
        let placements = vec![waiting_probe(
            "IDC_LIST_SWAIT_CHAT",
            None,
            (24, 455, 511, 553),
        )];
        let rows = vec![WaitingChatMessage::user("host", "hello")];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "",
            &placements,
        );
        assert_eq!(buf[453 * 800 + 20], 0, "no scrim band over the row");
        // The probe rides the history band: the white caret below the
        // list is near gray at the 64 ink tolerance and would mask the
        // row ink.
        let row_band = &buf[440 * 800..555 * 800];
        assert!(
            row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xDE, 0xDE, 0xDE))),
            "user gray ink on the row"
        );
        assert!(
            !row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xE6, 0x71, 0x00))),
            "no admin ink on a user row"
        );
    }

    #[test]
    fn waiting_chat_paints_admin_rows_in_the_exe_amber() {
        // The 3013 append ink is COLORREF `0x71e6` (call file
        // `0xb9fbf`) — `0x00BBGGRR` through GDI `SetTextColor`, a warm
        // amber (the reference rows all read warm; `docs/parity.md`
        // carries the photo residual).
        let placements = vec![waiting_probe(
            "IDC_LIST_SWAIT_CHAT",
            None,
            (24, 455, 511, 553),
        )];
        let rows = vec![WaitingChatMessage::admin("op", "quiet")];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "",
            &placements,
        );
        assert_eq!(buf[453 * 800 + 20], 0, "no scrim band over the row");
        assert!(has_ink_near(&buf, pack_xrgb(0xE6, 0x71, 0x00)));
        // User gray belongs to 3012 rows only.
        let row_band = &buf[440 * 800..555 * 800];
        assert!(
            !row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xDE, 0xDE, 0xDE))),
            "no user ink on an admin row"
        );
    }

    #[test]
    fn waiting_chat_shows_only_the_newest_rows() {
        let placements = vec![waiting_probe(
            "IDC_LIST_SWAIT_CHAT",
            None,
            (24, 455, 511, 553),
        )];
        let mut rows = Vec::new();
        for index in 0..WAITING_CHAT_VISIBLE_ROWS + 2 {
            rows.push(WaitingChatMessage::user("host", &format!("line{index}")));
        }
        let visible = waiting_chat_visible_messages(&rows);
        assert_eq!(visible.len(), WAITING_CHAT_VISIBLE_ROWS);
        assert_eq!(visible[0].text, "line2");
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "",
            &placements,
        );
        assert!(buf.contains(&pack_xrgb(0xDE, 0xDE, 0xDE)));
        assert!(!buf.contains(&scrim_color()), "no scrim bands");
    }

    #[test]
    fn waiting_chat_paints_the_draft_at_the_edit_rect_with_the_caret_after_it() {
        let placements = vec![waiting_probe(
            "IDC_EDIT_SWAIT_CHAT",
            None,
            (20, 563, 460, 583),
        )];
        let input = "go";
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            input,
            &placements,
        );
        assert_eq!(buf[561 * 800 + 16], 0, "no scrim band over the draft");
        assert!(has_ink_near(&buf, art_title_ink()));
        // The caret rides the insertion point: inset (5,5) plus the
        // typed width (empty-input case: `reference-waiting-4x3.png`).
        let caret_x = 20 + 5 + text_width(input) as i32;
        assert_eq!(buf[568 * 800 + caret_x as usize], art_title_ink());
        assert_eq!(buf[577 * 800 + (caret_x + 1) as usize], art_title_ink());
    }

    #[test]
    fn waiting_chat_paints_the_caret_on_an_empty_input() {
        let placements = vec![waiting_probe(
            "IDC_EDIT_SWAIT_CHAT",
            None,
            (20, 563, 460, 583),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            "",
            &placements,
        );
        assert_eq!(buf[568 * 800 + 25], art_title_ink());
        assert_eq!(buf[577 * 800 + 26], art_title_ink());
    }

    #[test]
    fn waiting_chat_falls_back_to_native_rects_without_staged_art() {
        let rows = vec![WaitingChatMessage::user("host", "hello")];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "go",
            &[],
        );
        assert_eq!(buf[453 * 800 + 20], 0, "no scrim over the row");
        assert_eq!(buf[561 * 800 + 16], 0, "no scrim over the draft");
        let row_band = &buf[440 * 800..555 * 800];
        assert!(
            row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xDE, 0xDE, 0xDE))),
            "user gray ink on the row"
        );
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn channel_chat_row_text_brackets_like_the_waiting_box() {
        let row = ChannelChatMessage::user("host", "hello");
        assert_eq!(channel_chat_row_text(&row), "[        host] hello");
        let admin = ChannelChatMessage::admin("op", "quiet");
        assert_eq!(channel_chat_row_text(&admin), "[          op] quiet");
    }

    #[test]
    fn channel_whisper_row_names_both_ends() {
        let row = ChannelChatMessage::whisper("host", "me", "psst");
        assert_eq!(
            channel_chat_row_text(&row),
            "[        host ->           me] psst"
        );
    }

    #[test]
    fn channel_chat_shows_only_the_newest_rows() {
        let mut rows = Vec::new();
        for index in 0..CHANNEL_CHAT_VISIBLE_ROWS + 2 {
            rows.push(ChannelChatMessage::user("host", &format!("line{index}")));
        }
        let visible = channel_chat_visible_messages(&rows);
        assert_eq!(visible.len(), CHANNEL_CHAT_VISIBLE_ROWS);
        assert_eq!(visible[0].text, "line2");
    }

    #[test]
    fn channel_chat_without_rows_paints_nothing() {
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[],
            2,
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn channel_chat_paints_user_rows_in_the_exe_gray() {
        let rows = vec![ChannelChatMessage::user("host", "hello")];
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &rows,
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xDE, 0xDE, 0xDE)));
    }

    #[test]
    fn channel_chat_paints_admin_rows_in_the_exe_amber() {
        let rows = vec![ChannelChatMessage::admin("op", "quiet")];
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &rows,
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xE6, 0x71, 0x00)));
    }

    #[test]
    fn announcement_row_paints_bare_text_in_admin_amber() {
        let row = ChannelChatMessage::announcement("hello");
        assert!(row.admin);
        assert_eq!(channel_chat_row_text(&row), "hello");
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[row],
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xE6, 0x71, 0x00)));
    }

    #[test]
    fn rank_row_brackets_nickname_and_names_song_and_rank() {
        let row = ChannelChatMessage::rank("host", 757, 1);
        assert!(row.admin);
        assert_eq!(channel_chat_row_text(&row), "[        host] M757 1");
    }

    #[test]
    fn waiting_random_cards_carry_no_card_data() {
        // The `?` cards name no per-card data: the big card sits on
        // `RANDMAP` (rect 78 `(619,468,708,533)`) and the small cards on
        // the spriteless `EFFECT_1/2` rects (68/69) — caption and frames
        // baked into the staged backdrop, no wire field behind them.
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_RANDMAP", None, (619, 468, 708, 533)),
            waiting_probe("IDC_IMAGE_SWAIT_EFFECT_1", None, (719, 450, 779, 498)),
            waiting_probe("IDC_IMAGE_SWAIT_EFFECT_2", None, (719, 507, 779, 555)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
            &WaitingCaptions::default(),
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn room_title_anchors_to_the_roomname_rect() {
        // `ROOMNAME` rect 21 `(274,15,466,38)`: the joined-room title.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_ROOMNAME",
            None,
            (274, 15, 466, 38),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_title(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[13 * 800 + 270], scrim_color());
    }

    #[test]
    fn unstaged_regions_paint_nothing() {
        // Missing placements paint nothing: the exe leaves untouched
        // icons alone — no invented coordinates anywhere.
        let empty: Vec<ScenePlacement> = Vec::new();
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &empty,
            &WaitingCaptions::default(),
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn staged_scene_prefers_bnd_anchors_over_the_legacy_grid() {
        // Covers staged: the seat line anchors to the cover rect
        // and the legacy grid slot `(100,150)` stays untouched — an
        // empty stage paints no fill (`docs/channel-fill.md`).
        let placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_COVERAVATAR1",
            None,
            (28, 53, 169, 228),
        )];
        let art = SceneArt {
            placements,
            figures: HashMap::new(),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
        );
        assert_eq!(buf[51 * 800 + 24], scrim_color());
        assert_eq!(buf[150 * 800 + 100], 0);
    }

    #[test]
    fn waiting_control_ids_match_their_staged_bodies() {
        // The seat covers stage one named body; the team grid stages
        // one body per team in order (`ControlList_Interface.txt`).
        assert_eq!(WAIT_COVER_SPRITE, "Wait_Cover_Avatar.ojs");
        assert_eq!(WAIT_TEAM_SPRITES[0], "Wait_TeamA.ojs");
        assert_eq!(WAIT_TEAM_SPRITES[7], "Wait_TeamH.ojs");
    }

    #[test]
    fn spriteless_channel_block_stages_no_scene_art() {
        // Shape-faithful `STATE_CHANNEL` skeleton (bare `BOUND`, the
        // real image/button/text/`SET` code classes, zero quoted
        // sprites — `docs/channel-fill.md`): no block entry names art,
        // so the scene stages nothing and the painter keeps its flat
        // background plus the live list.
        let list = o2jam_file::parse_control_list(
            &[
                "NUMBER_OF_STATE 0x01",
                "STATE_CHANNEL 0x04",
                "{",
                "BOUND",
                "IDC_STATE_CHANNEL 0X12000000",
                "IDC_BACK_SCHANNEL_1 0X12100001",
                "IDC_BUTTON_SCHANNEL_BACK 0X12200002",
                "IDC_BUTTON_SCHANNEL_EXBUTTON1 0X12011007",
                "SET 0X02 0X00011000",
                "{",
                "IDC_IMAGE_SCHANNEL_ANI 0X12500011",
                "IDC_IMAGE_SCHANNEL_NUMBER1 0X12500012",
                "}",
                "IDC_TEXT_SCHANNEL_TITLE1 0X1240001E",
                "}",
                "",
            ]
            .join("\n"),
        )
        .unwrap();
        assert!(main_scene_sprites(&list, Screen::Channel).is_empty());
    }

    #[test]
    fn empty_channel_art_paints_no_fill() {
        // Pinned no-fill rule (`docs/channel-fill.md`): no staged
        // figures means no staged ink and no invented fill — the bare
        // corner stays untouched while the live title still paints.
        let art = SceneArt::default();
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[],
            0,
            &art,
        );
        assert_eq!(buf[0], 0);
        assert!(!buf.contains(&background_of(Screen::Channel)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn staged_empty_backdrop_leaves_every_frame_untouched() {
        // The exe contributes zero pixels behind art-less states, so
        // an empty stage paints nothing on any screen: a patterned
        // backbuffer survives `paint_backdrop` verbatim.
        let art = SceneArt::default();
        assert!(!art.has_figures());
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let mut buf = vec![0x123456u32; 16 * 12];
            art.paint_backdrop(
                &mut Frame {
                    buf: &mut buf,
                    w: 16,
                    h: 12,
                },
                screen,
            );
            assert_eq!(buf, vec![0x123456u32; 16 * 12], "{screen:?}");
        }
    }

    #[test]
    fn channel_bar_art_layers_behind_the_live_list() {
        // The code-fetched channel bar (`DIALOG_BAR_SPRITE`, never a
        // `STATE_CHANNEL` entry) stages entry-less at its record anchor:
        // bar ink at the origin, live list and cursor on top, no fill.
        let art = SceneArt {
            placements: unbound_scene_placements(&[DIALOG_BAR_SPRITE.to_owned()]),
            figures: HashMap::from([(DIALOG_BAR_SPRITE.to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_channels(),
            0,
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&background_of(Screen::Channel)));
    }

    #[test]
    fn waiting_caption_ids_name_the_pinned_controls() {
        // Codes from `docs/caption-sources.md`: MAPNAME `0x16400018`,
        // BPM `0x16400069`, skill counts `0x1640005E..60`.
        assert_eq!(WAITING_MAPNAME_ID, "IDC_TEXT_SWAIT_MAPNAME");
        assert_eq!(WAITING_BPM_ID, "IDC_TEXT_SWAIT_BPM");
        assert_eq!(
            WAITING_SKILLCOUNT_IDS,
            [
                "IDC_TEXT_SWAIT_1SKILLCOUNT",
                "IDC_TEXT_SWAIT_2SKILLCOUNT",
                "IDC_TEXT_SWAIT_3SKILLCOUNT",
            ]
        );
    }

    #[test]
    fn waiting_mapname_cycles_the_room_word_table() {
        // Map-cycle path: word 0 selects slot 3, otherwise the low
        // word modulo 4 (files `0xb3cc6`–`0xb3ce3`). The three
        // CP949-sourced Korean names ride the embedded TTF.
        assert_eq!(waiting_map_name(0), "원하지않은결말");
        assert_eq!(waiting_map_name(1), "한가한대전장의오후");
        assert_eq!(waiting_map_name(2), "패자의역습");
        assert_eq!(waiting_map_name(3), "원하지않은결말");
        assert_eq!(waiting_map_name(4), "None");
    }

    #[test]
    fn waiting_mapname_paints_at_the_staged_rect() {
        // MAPNAME probe `(480,15,570,38)`: room word 1 names slot 1,
        // so the scrim opens at (476,13) with white glyph ink.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_MAPNAME",
            None,
            (480, 15, 570, 38),
        )];
        let mut room = champion_room();
        room.arena = 1;
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_mapname(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &room,
            &placements,
        );
        assert_eq!(buf[13 * 800 + 476], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_bpm_formats_whole_tempos() {
        // `%0.f` over the catalog column (store file `0xbe1d5`).
        assert_eq!(waiting_bpm_caption(Some(124.0)), "124");
        assert_eq!(waiting_bpm_caption(Some(124.4)), "124");
        assert_eq!(waiting_bpm_caption(Some(168.8)), "169");
    }

    #[test]
    fn waiting_bpm_defaults_to_mismatch_without_a_record() {
        // Mismatch default `-0-` (rdata file `0x24a1d0`): no staged
        // record, a blank lookup, or a non-finite tempo.
        assert_eq!(waiting_bpm_caption(None), "-0-");
        assert_eq!(waiting_bpm_caption(Some(f32::NAN)), "-0-");
        assert_eq!(waiting_bpm_caption(Some(f32::INFINITY)), "-0-");
    }

    #[test]
    fn waiting_bpm_paints_at_the_staged_rect() {
        // BPM probe `(620,60,680,80)`: staged 124 BPM inks the rect.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_BPM",
            None,
            (620, 60, 680, 80),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_bpm(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            Some(124.0),
            &placements,
        );
        assert_eq!(buf[58 * 800 + 616], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_bpm_paints_the_mismatch_default() {
        // Same rect without a staged record: `-0-` inks it, never a
        // blank or an invented tempo.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_BPM",
            None,
            (620, 60, 680, 80),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_bpm(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            &placements,
        );
        assert_eq!(buf[58 * 800 + 616], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_skillcounts_pad_and_cap_late_slots() {
        // Width-3 decimal (helper `0x62c109`); slots 2–3 clamp at
        // 999 while slot 1 shows the full count.
        assert_eq!(waiting_skillcount_text(7, false), "  7");
        assert_eq!(waiting_skillcount_text(42, true), " 42");
        assert_eq!(waiting_skillcount_text(1500, false), "1500");
        assert_eq!(waiting_skillcount_text(1500, true), "999");
    }

    #[test]
    fn waiting_skillcounts_paint_at_their_staged_rects() {
        // Count probes below the ring rects: `[7, 42, 1500]` inks
        // all three, the late slots through the 999 cap.
        let placements = vec![
            waiting_probe("IDC_TEXT_SWAIT_1SKILLCOUNT", None, (740, 208, 780, 238)),
            waiting_probe("IDC_TEXT_SWAIT_2SKILLCOUNT", None, (740, 246, 780, 276)),
            waiting_probe("IDC_TEXT_SWAIT_3SKILLCOUNT", None, (740, 284, 780, 314)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_skillcounts(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[7, 42, 1500],
            &placements,
        );
        assert_eq!(buf[206 * 800 + 736], scrim_color());
        assert_eq!(buf[244 * 800 + 736], scrim_color());
        assert_eq!(buf[282 * 800 + 736], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_leaves_instname_and_notice_rects_unpainted() {
        // `INSTNAME` names zero code immediates (OPEN 1) and the
        // NOTICE dialog `+0x46c` fill is untraced (OPEN 2): their
        // rects stay untouched while the pinned MAPNAME rect inks.
        let placements = vec![
            waiting_probe("IDC_TEXT_SWAIT_MAPNAME", None, (480, 15, 570, 38)),
            waiting_probe("IDC_TEXT_SWAIT_INSTNAME", None, (480, 40, 570, 60)),
            waiting_probe("IDC_IMAGE_SWAIT_NOTICE_TEXT", None, (100, 455, 400, 475)),
            waiting_probe("IDC_IMAGE_SWAIT_NOTICEBOARD", None, (100, 480, 400, 530)),
        ];
        let mut room = champion_room();
        room.arena = 1;
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &room,
            &placements,
            &WaitingCaptions::default(),
        );
        assert_eq!(buf[13 * 800 + 476], scrim_color(), "mapname inks");
        assert_eq!(buf[40 * 800 + 480], 0, "instname untouched");
        assert_eq!(buf[455 * 800 + 100], 0, "notice text untouched");
        assert_eq!(buf[480 * 800 + 100], 0, "notice board untouched");
    }
}
