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
    CHANNEL_BLOCK, LOADING_SPRITE, RESULT_BLOCK, ROOM_BLOCK, WAITING_BLOCK,
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
    text_width, Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH,
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
    /// flat fill, so the OPI art keeps its `.bnd` layout. Empty art falls
    /// back to the flat background; placements without a loaded
    /// figure are skipped.
    pub fn paint_backdrop(&self, frame: &mut Frame<'_>, screen: Screen) {
        if !self.has_figures() {
            fill_background(frame, screen);
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

/// Channel dialog bar: `STATE_CHANNEL` names zero sprites (verified
/// on 7.55), so the channel scene stages this code-mounted bar
/// instead — the same fallback the real-art soak uses. The bar
/// carries no block entry, so it stages with an empty id and paints
/// at its record anchor.
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
/// (code-mounted, not block-named); channel falls back to the
/// code-mounted dialog bar (`STATE_CHANNEL` names zero sprites);
/// room, waiting, playing, and result name their block entries for
/// `bind_scene_rects`.
#[must_use]
pub fn main_scene_sprites(list: &ControlList, screen: Screen) -> Vec<String> {
    match screen {
        Screen::Avi => block_or_single(list, AVI_BLOCK, AVI_LOGO_SPRITE),
        Screen::Channel => {
            let names = block_sprites(list, CHANNEL_BLOCK);
            if names.is_empty() {
                vec![DIALOG_BAR_SPRITE.to_owned()]
            } else {
                names
            }
        }
        Screen::Room => block_sprites(list, ROOM_BLOCK),
        Screen::Waiting => block_sprites(list, WAITING_BLOCK),
        Screen::Loading => block_or_single(list, "STATE_LOADING", LOADING_SPRITE),
        Screen::Playing => block_sprites(list, PLAYING_BLOCK),
        Screen::Result => block_sprites(list, RESULT_BLOCK),
    }
}

/// Placements for code-mounted sprites without a block entry
/// (channel dialog bar, loading gate): no entry means no id, code,
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

/// Channel rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene).
fn paint_channel_rows(frame: &mut Frame<'_>, channels: &[ChannelState], highlight: usize) {
    paint_list_scrim(frame, channels.len());
    let w = frame.w as i32;
    let h = frame.h as i32;
    let text_x = w / 4;
    for (index, channel) in channels.iter().enumerate() {
        let row = index_as_row(index, h);
        let entry = format!(
            "{} {}/{}",
            channel.channel_id, channel.population, channel.capacity
        );
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

/// Paint staged waiting art (team buttons, avatar covers) with the
/// live seats and ready dots on top.
pub fn paint_waiting_scene(frame: &mut Frame<'_>, room: &JoinedRoom, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Waiting);
    paint_title_over_art(frame, Screen::Waiting);
    paint_waiting_rows(frame, room);
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

/// Loading progress bar over a dark track (no background fill:
/// shared by the flat fallback and the art scene).
fn paint_loading_bar(frame: &mut Frame<'_>) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: w / 4 - 2,
            y: h / 2 - 10,
            w: (w / 2 + 4).max(0) as u32,
            h: 20,
        },
        scrim_color(),
    );
    fill_rect(
        frame,
        &Rect {
            x: w / 4,
            y: h / 2 - 8,
            w: (w / 2) as u32,
            h: 16,
        },
        pack_xrgb(0x30, 0xB0, 0x30),
    );
}

/// Paint the loading gate scene.
fn paint_loading(frame: &mut Frame<'_>) {
    fill_background(frame, Screen::Loading);
    paint_loading_bar(frame);
}

/// Paint staged loading art (`MusicLoading.ojs`) with the live
/// progress bar on top.
pub fn paint_loading_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Loading);
    paint_loading_bar(frame);
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

/// Battle verdict: rank 1 by team score wins (`RoomTeams::winner`).
/// Solo or team, grades play no part (mapping OPEN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleVerdict {
    /// Local team holds rank 1.
    Win,
    /// Another team holds rank 1.
    Lose,
}

/// Rank-1 verdict for the local team: `Win` when it holds the top
/// total, `Lose` otherwise. `None` while the winner or the local
/// team is unknown (empty room, solo with no seat claimed).
#[must_use]
pub fn battle_verdict(teams: &RoomTeams, self_team: Option<u8>) -> Option<BattleVerdict> {
    let winner = teams.winner()?;
    let local = self_team?;
    if local == winner {
        Some(BattleVerdict::Win)
    } else {
        Some(BattleVerdict::Lose)
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
/// draws frame 1 and 2 draws frame 2, anything else frame 3. The
/// shell's verdict model only resolves rank-1 (flag 1) or not, so
/// the flag-2 arm stays unreachable here; frame 4 clamps to the
/// fourth image at draw time like the exe setter.
#[must_use]
pub fn winlose_frame(verdict: ResultVerdict) -> Option<u8> {
    match verdict {
        ResultVerdict::Solo => Some(4),
        ResultVerdict::Battle(Some(BattleVerdict::Win)) => Some(1),
        ResultVerdict::Battle(Some(BattleVerdict::Lose)) => Some(3),
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
/// (docs/cresult-anchors.md, top-panel painter `FUN_004c38a0`):
/// COOL/GOOD/BAD/MISS at y=125, MAX/JAM COMBO at y=130, total score
/// at (605,533) — white GDI text under the baked `StateResult_Top`
/// headers, never digit sprites. Each field draws its decimal from
/// the anchor's left edge: the exe printf widths (`%5d`/`%3d`/
/// `%4d`/`%8d`) ride a text helper whose left/right anchoring is
/// OPEN (dossier note 5), so the verified passed x is kept as the
/// left edge. JAM COMBO reads the jam level chain (the wire's u16
/// `jam` field; mapping OPEN 3); non-native frames paint nothing.
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

/// Paint the result backdrop: the staged scene minus the frame
/// bodies ([`RESULT_FRAME_BODIES`]) in file order, so their unused
/// frames never stack through the whole-layer pass. Empty art falls
/// back to the flat background; placements without a loaded figure
/// are skipped.
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

/// Paint the avatar figure and the pinned result numbers (shared by
/// the flat fallback and the staged scene; the numbers are GDI text
/// the exe paints regardless of the art).
fn paint_result_content(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
) {
    if let Some(figure) = avatar {
        figure.paint(frame, frame.w as i32 / 2 - 32, frame.h as i32 / 2 - 32);
    }
    paint_result_numbers(frame, score);
}

/// Paint the flat results fallback: card, avatar figure, and the
/// pinned result numbers. The retired text verdict stays gone — the
/// banner is a sprite frame of the staged art.
pub(crate) fn paint_result(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
) {
    paint_card(
        frame,
        pack_xrgb(0x10, 0x10, 0x08),
        pack_xrgb(0xC0, 0xA0, 0x20),
    );
    paint_result_content(frame, avatar, score);
}

/// Paint the staged result scene (`docs/cresult-anchors.md`): the
/// `.bnd`-bound block art first (minus the frame bodies), then the
/// exe-selected verdict banner and grade plate frames, then the
/// avatar figure and the pinned GDI numbers on top — the fill order
/// of `FUN_004c3120` (banner, then top-panel painter, then rows).
pub fn paint_result_scene(
    frame: &mut Frame<'_>,
    avatar: Option<&AvatarFigure>,
    score: &o2jam_game::Score,
    art: &SceneArt,
    verdict: ResultVerdict,
    grade: Option<u8>,
) {
    paint_result_backdrop(frame, art);
    paint_result_body_frame(
        frame,
        &art.figures,
        RESULT_WINLOSE_SPRITE,
        winlose_frame(verdict),
    );
    paint_result_body_frame(
        frame,
        &art.figures,
        RESULT_RANK_SPRITE,
        grade.filter(|g| *g >= 1),
    );
    paint_result_content(frame, avatar, score);
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
/// uncovered and notes keep their top slot.
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
        Screen::Result => paint_result(frame, None, &o2jam_game::Score::default()),
    }
}

#[cfg(test)]
mod paint_tests {
    use super::*;
    use crate::avatar::FigureLayer;
    use crate::screen::Screen;
    use o2jam_net::{JoinedRoom, RoomMember, RoomSlot};
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
                None,
                &o2jam_game::Score::default(),
                art,
                ResultVerdict::Solo,
                None,
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
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
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
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
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
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
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
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
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
        assert!(buf.contains(&art_title_ink()));
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
        assert!(buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        // Locked room paints the red lock line.
        assert!(buf.contains(&pack_xrgb(0xFF, 0x50, 0x50)));
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
            Some(&red_figure()),
            &score,
            &art,
            ResultVerdict::Solo,
            None,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(255, 0, 0)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
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
        // No figures staged: the backdrop keeps the flat fallback
        // so a bare frame never shows garbage. Staged scenes (see
        // `every_art_scene_hides_flat_fill`) never take this path.
        let art = SceneArt::default();
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            assert!(!art.has_figures());
            let buf = paint_art(screen, &art, 16, 12);
            assert_eq!(buf[0], background_of(screen), "{screen:?}");
        }
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
    fn battle_verdict_stays_empty_without_winner_or_self() {
        let empty = RoomTeams::new();
        assert_eq!(battle_verdict(&empty, Some(0)), None);
        let teams = battle_room();
        assert_eq!(battle_verdict(&teams, None), None);
    }

    /// Flat native result for `score`, exactly as the no-art
    /// fallback paints it.
    fn flat_native_result(score: &o2jam_game::Score) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            score,
        );
        buf
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
            None,
            &o2jam_game::Score::default(),
            art,
            verdict,
            grade,
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
    /// digit (the 3x5 glyphs of 8/9/3/2/7/5 all carry a lit
    /// top-left pixel).
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

    /// One pinned field: the glyph pixel at `(x, y)` must light and
    /// the pixel left of the anchor must stay flat (the decimal
    /// starts exactly at the pinned x).
    fn assert_field(buf: &[u32], x: i32, y: i32) {
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(x, y), result_number_ink(), "field at ({x},{y})");
        assert_ne!(at(x - 1, y), result_number_ink(), "left of ({x},{y})");
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
        );
        assert!(!buf.contains(&result_number_ink()));
    }

    #[test]
    fn winlose_frames_follow_the_pinned_rule() {
        // `FUN_004c3120`: solo 4; battle flag 1 -> 1, flag 2 -> 2,
        // else 3. The shell resolves flag 1 (rank 1) or not; the
        // flag-2 arm stays unreachable (dossier OPEN 10).
        assert_eq!(winlose_frame(ResultVerdict::Solo), Some(4));
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Win))),
            Some(1)
        );
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Lose))),
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
    fn battle_lose_paints_the_third_banner_image() {
        let buf = painted_native_scene(
            &banner_art(),
            ResultVerdict::Battle(Some(BattleVerdict::Lose)),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(151, 190), frame_ink(2));
        assert_ne!(at(100, 190), frame_ink(0));
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
            None,
            &o2jam_game::Score::default(),
            &art,
            ResultVerdict::Battle(Some(BattleVerdict::Win)),
            None,
        );
        // The staged art paints (the corner banner body rides frame 1
        // onto its clamped layer); the retired amber caption ink
        // never comes back.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
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
    fn channel_bar_paints_at_its_authored_anchor_not_the_canvas() {
        // `Dialog_ChannelSelectBar.ojs` anchors at `(0, 0)` (347x41 on
        // 7.55): staged unbound it paints 1:1 at that authored anchor —
        // never stretched over the canvas.
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
    fn waiting_covers_paint_per_seat_rects() {
        // `Wait_Cover_Avatar.ojs` anchors at (0,0): its eight rect-
        // bound placements (COVERAVATAR1..8, codes 0x1650003D..44)
        // each paint at their own rect origin — the view-from-rect
        // case ui-draw.md leaves open, resolved for anchor-free
        // bodies.
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
        );
        let red = pack_xrgb(255, 0, 0);
        for seat in 0..8u32 {
            let x = 10 + seat * 60;
            let y = 200 + seat * 45;
            assert_eq!(
                buf[y as usize * 800 + x as usize],
                red,
                "cover {seat} at its seat rect"
            );
        }
        assert_eq!(buf[0], 0, "covers no longer stack at the frame corner");
    }

    #[test]
    fn channel_falls_back_to_the_dialog_bar() {
        // `STATE_CHANNEL` names zero sprites on 7.55: the scene
        // stages the code-mounted dialog bar instead.
        let list = o2jam_file::parse_control_list(
            &[
                "NUMBER_OF_STATE 0x01",
                "STATE_CHANNEL 0x27",
                "{",
                "BOUND",
                "IDC_STATE_CHANNEL 0X12000000",
                "IDC_BACK 0X12100001",
                "}",
                "",
            ]
            .join("\n"),
        )
        .unwrap();
        assert_eq!(
            main_scene_sprites(&list, Screen::Channel),
            vec![DIALOG_BAR_SPRITE.to_owned()]
        );
    }
}
