//! Room-screen paint generation: staged panel rows, cloned number
//! digits, detail wells, row icons, hover chrome, and user panel.
//!
//! Kept behind re-exports in `super::paint`, so callers and the
//! existing scenario guards stay unchanged (#139 paint split).

use super::*;

/// One room row: `3 TITLE 5/8`, locked rooms suffixed `!`.
pub(crate) fn room_row_text(room: &RoomListEntry) -> String {
    let mut text = format!(
        "{} {} {}/{}",
        room.number, room.title, room.user_count, room.capacity
    );
    if room.has_password {
        text.push('!');
    }
    text
}

/// Room panel buttons in `ControlList_Interface.txt` file order:
/// `EXBUTTON1,2,3` then `5,4,6` — the file lists 5 before 4, so file
/// order reads the left column top-to-bottom then the right column
/// top-to-bottom. Ordinals `0x1B..0x20` pair `.bnd` rects 26..31
/// (`(26,82)-(314,173)` through `(317,272)-(605,363)`), the verified
/// 2-column x 3-row grid (left `x=26`, right `x=317`; rows `y=82`,
/// `177`, `272`).
pub(crate) const ROOM_PANEL_IDS: [&str; 6] = [
    "IDC_BUTTON_SROOM_EXBUTTON1",
    "IDC_BUTTON_SROOM_EXBUTTON2",
    "IDC_BUTTON_SROOM_EXBUTTON3",
    "IDC_BUTTON_SROOM_EXBUTTON5",
    "IDC_BUTTON_SROOM_EXBUTTON4",
    "IDC_BUTTON_SROOM_EXBUTTON6",
];

/// Control ids of the staged room panels that host the row
/// template, in row order (the `SET` clone instances the painter may
/// show; clones past the listed rooms stay hidden).
#[must_use]
pub(crate) fn room_panel_instances(placements: &[ScenePlacement]) -> Vec<&str> {
    placements
        .iter()
        .filter(|placed| placed.role == PlacementRole::Single)
        .filter(|placed| ROOM_PANEL_IDS.contains(&placed.id.as_str()))
        .map(|placed| placed.id.as_str())
        .collect()
}

/// Native origins of the staged room panels in file order (the
/// panel path positions fields panel-relatively; each field rect
/// scales to the frame on paint. Skips panels the staged art does
/// not name; empty while no staged art names any panel).
pub(crate) fn room_panel_origins(placements: &[ScenePlacement]) -> Vec<(i32, i32)> {
    ROOM_PANEL_IDS
        .iter()
        .filter_map(|id| {
            placements
                .iter()
                .find(|placed| placed.id == *id)
                .map(|placed| (placed.rect.x, placed.rect.y))
        })
        .collect()
}

/// Whether staged art binds any room panel (the panel path owns
/// layout while one is bound; otherwise the legacy row bands do).
#[must_use]
pub(crate) fn has_room_panels(placements: &[ScenePlacement]) -> bool {
    ROOM_PANEL_IDS
        .iter()
        .any(|id| placements.iter().any(|placed| placed.id == *id))
}

/// File-order index of the room panel whose staged `.bnd` rect
/// contains native pixel (`x`, `y`) (`None` while no bound panel
/// covers the point — stale panels past the list never hit).
#[must_use]
pub(crate) fn room_panel_hit(placements: &[ScenePlacement], x: i32, y: i32) -> Option<usize> {
    ROOM_PANEL_IDS
        .iter()
        .position(|id| staged_rect_hit(placements, id, x, y))
}

/// Room rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene). `None` art (or art binding
/// no panel) takes the legacy quarter-frame bands; staged panels
/// take the pinned per-panel fields.
pub(crate) fn paint_room_rows(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: Option<&SceneArt>,
) {
    paint_room_rows_with_hover(frame, rooms, highlight, art, None);
}

/// Same row paint with the staged panel under the pointer, if any.
pub(crate) fn paint_room_rows_with_hover(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: Option<&SceneArt>,
    hovered_panel: Option<usize>,
) {
    match art {
        Some(art) => {
            let origins = room_panel_origins(&art.placements);
            if origins.is_empty() {
                paint_room_fallback_rows(frame, rooms, highlight);
            } else {
                paint_room_panel_rows(frame, rooms, highlight, art, &origins, hovered_panel);
            }
        }
        None => paint_room_fallback_rows(frame, rooms, highlight),
    }
}

/// Exe-measured room row field origins (native px, panel-relative;
/// rig 2026-11-09 `t2-room-stage.png`,
/// `~/scratch/geom-open-close/findings.md` §2): the number plate at
/// panel +(12,12), the title at +(64,+11), the seat count at
/// +(194,+10). Title/count splits of the old combined row string ride
/// these instead of the photo-era single anchor (+34/+1,
/// `/tmp/aa-room.png` — retired as a misread).
///
/// Digit pitch of the number plate (native px): the pinned on-screen
/// digits sit 11 px apart (38/49/60 on panel 0), the `ROOMNO` place
/// width (`State_Room.bnd` r45..47).
pub(crate) const ROOM_ROW_DIGIT_PITCH: i32 = 11;
/// Place weights of the three room-number digit slots in control
/// ordinal order (ones, tens, hundreds).
pub(crate) const ROOM_ROW_DIGIT_WEIGHTS: [u32; 3] = [1, 10, 100];
/// Staged room-number digit font (the `ROOMNO` places' shared body,
/// `IDC_IMAGE_SROOM_ROOMNO1/10/100`).
pub(crate) const ROOM_NUMBER_SPRITE: &str = "Room_Number.ojs";
/// Panel-relative origin of the number plate (native px): panel
/// origin +12/+12 (rig pin above). Hundreds, tens, ones run
/// left-to-right at the digit pitch.
pub(crate) const ROOM_ROW_NUMBER_ORIGIN: (i32, i32) = (12, 12);
/// Panel-relative origin of the title field (native px): panel
/// origin +64/+11 (rig pin above; the `ROOMNAME` rect sits at +66 —
/// the 2 px run-in is the GDI left margin).
pub(crate) const ROOM_ROW_TITLE_ORIGIN: (i32, i32) = (64, 11);
/// Panel-relative origin of the seat-count field (native px): panel
/// origin +194/+10 (rig pin above — the count rides the row, not
/// the `ROOM_USER` rect, which the capture shows as background).
pub(crate) const ROOM_ROW_COUNT_ORIGIN: (i32, i32) = (194, 10);
/// GDI cap-height share of a TEXT control rect: the live room title
/// spans ~11 px in its 16 px rect, the music line ~14 px in 19 px
/// (`t2-room-stage.png`); text sizes to 70% of the rect height.
pub(crate) const TEXT_CAP_FRACTION: f32 = 0.7;

/// Em extent fitting `text` to a `w`×`h` rect at the exe's GDI cap
/// share: cap height fills [`TEXT_CAP_FRACTION`] of the rect height
/// (the full-height [`cp949_fit_scale`] would undershoot the live
/// glyphs by half). Width still clamps long strings into the rect.
pub(crate) fn room_text_scale(text: &str, w: f32, h: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 || text.is_empty() {
        return 0.0;
    }
    let (width_per_em, cap_em) = match o2jam_text::system_font() {
        Some(font) => (
            o2jam_text::measure_text(font, text, 1.0),
            font.cap_height_em(),
        ),
        None => (o2jam_text::font::measure_text_stand_in(text, 1.0), 1.0),
    };
    if width_per_em <= 0.0 || cap_em <= 0.0 {
        return 0.0;
    }
    let height_fit = TEXT_CAP_FRACTION * h / cap_em;
    height_fit.min(w / width_per_em).max(0.0)
}

/// Paint one room row string into its field rect (native, usually
/// panel-relative): the field origin is the pinned row offset while
/// the size rides the staged template rect — the exe's GDI text
/// fills the TEXT control at its cap share. Bare ink, no scrim.
pub(crate) fn paint_room_text_into_rect(
    frame: &mut Frame<'_>,
    native_rect: Rect,
    text: &str,
    ink: [u8; 4],
) {
    if frame.w == 0 || frame.h == 0 || native_rect.w == 0 || native_rect.h == 0 || text.is_empty() {
        return;
    }
    let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let scale = room_text_scale(text, rect.w as f32, rect.h as f32);
    if scale <= 0.0 {
        return;
    }
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        0,
        0,
        text,
        ink,
        scale,
    );
    composite_text_image(frame, &rect, &image);
}

/// Native rect of one staged template control (`None` while the
/// staged art names no such control — the row field then stays
/// unpainted, like the exe's untouched icons).
#[must_use]
pub(crate) fn staged_field_rect(placements: &[ScenePlacement], id: &str) -> Option<Rect> {
    placements
        .iter()
        .find(|placed| placed.id == id)
        .map(|placed| placed.rect)
}

/// Room number through the staged digit font: the row template's
/// number places are `SET 0x11` clones under the row panel, ordered by
/// control ordinal (ones, tens, hundreds — the authoring order
/// `ROOMNO1 0x1350002E`, `ROOMNO10 0x1350002F`, `ROOMNO100 0x13500030`),
/// each place painting the frame of its digit at its own clone rect.
/// GDI number text (sized to the title band) while no clone stages.
pub(crate) fn paint_room_row_number(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    room: &RoomListEntry,
    panel_id: &str,
) {
    let value = u32::try_from(room.number.max(0)).unwrap_or(0);
    let mut places: Vec<&ScenePlacement> = art
        .placements
        .iter()
        .filter(|placed| placed.role == PlacementRole::Clone)
        .filter(|placed| placed.id == panel_id)
        .filter(|placed| placed.sprite.as_deref() == Some(ROOM_NUMBER_SPRITE))
        .collect();
    places.sort_by_key(|placed| placed.code & 0xFFF);
    if places.is_empty() {
        let Some(panel) = room_panel_origin(&art.placements, panel_id) else {
            return;
        };
        let band =
            staged_field_rect(&art.placements, ROOM_DETAIL_TITLE_ID).map_or(0, |rect| rect.h);
        paint_room_text_into_rect(
            frame,
            Rect {
                x: panel.0 + ROOM_ROW_NUMBER_ORIGIN.0,
                y: panel.1 + ROOM_ROW_NUMBER_ORIGIN.1,
                w: (u32::try_from(ROOM_ROW_DIGIT_PITCH).unwrap_or(0)).saturating_mul(3),
                h: band,
            },
            &value.to_string(),
            caption_ink(art_title_ink()),
        );
        return;
    }
    for (rank, place) in places.iter().enumerate() {
        let Some(weight) = ROOM_ROW_DIGIT_WEIGHTS.get(rank).copied() else {
            break;
        };
        let digit = (value / weight) % 10;
        paint_clone_digit(frame, &art.figures, place, digit);
    }
}

/// One clone body painted at its rect with the layer index `frame`
/// (digit frames are stored in digit order: layer *n* is glyph *n*,
/// `Room_Number.ojs` decodes ten 11x13 digit layers).
pub(crate) fn paint_clone_digit(
    frame: &mut Frame<'_>,
    figures: &HashMap<String, AvatarFigure>,
    place: &ScenePlacement,
    digit: u32,
) {
    let Some(sprite) = place.sprite.as_deref() else {
        return;
    };
    let Some(layer) = figures
        .get(sprite)
        .and_then(|figure| figure.layer(usize::try_from(digit).unwrap_or(usize::MAX)))
    else {
        return;
    };
    let dest = scale_rect_to_frame(&place.rect, frame.w, frame.h);
    blit_rgb555(
        frame,
        &layer.image.view(),
        dest.x + layer.dx,
        dest.y + layer.dy,
        Some(layer.colorkey),
    );
}

/// Code-mounted mouse-over outline (`Room_MOver.ojs`, 292x95 body).
/// The exe paints it over the hovered row template; the table names no
/// control for it, so the figure is hand-staged like the channel select
/// bar (`docs/hover.md` negative pin for the empty-list arm, live
/// positive pin in issue #368).
pub const ROOM_HOVER_SPRITE: &str = "Room_MOver.ojs";

/// Native origin of one staged room panel (`None` when the staged art
/// binds no such panel).
pub(crate) fn room_panel_origin(
    placements: &[ScenePlacement],
    panel_id: &str,
) -> Option<(i32, i32)> {
    placements
        .iter()
        .find(|placed| placed.id == panel_id)
        .map(|placed| (placed.rect.x, placed.rect.y))
}

/// Code-mounted mouse-over outline over one staged panel. The body
/// is two pixels larger than the `.bnd` panel on each side, so its
/// origin is the centered difference between the placed rect and the
/// decoded layer — geometry comes from the staged art, never a
/// painter literal. The outline paints last, above panel art and row
/// ink (live #368 pair: detail ink stays on the highlighted panel).
pub(crate) fn paint_room_panel_hover_outline(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    panels: &[&str],
    hovered_panel: Option<usize>,
) {
    let Some(index) = hovered_panel else {
        return;
    };
    let Some(panel_id) = panels.get(index).copied() else {
        return;
    };
    let Some(placed) = art.placements.iter().find(|placed| placed.id == panel_id) else {
        return;
    };
    let Some(figure) = art
        .figures
        .get(ROOM_HOVER_SPRITE)
        .filter(|figure| !figure.is_empty())
    else {
        return;
    };
    let Some(layer) = figure.layer(0) else {
        return;
    };
    let (Ok(rect_w), Ok(rect_h), Ok(layer_w), Ok(layer_h)) = (
        i32::try_from(placed.rect.w),
        i32::try_from(placed.rect.h),
        i32::try_from(layer.image.w),
        i32::try_from(layer.image.h),
    ) else {
        return;
    };
    let origin_x = placed
        .rect
        .x
        .saturating_add(rect_w.saturating_sub(layer_w) / 2);
    let origin_y = placed
        .rect
        .y
        .saturating_add(rect_h.saturating_sub(layer_h) / 2);
    let (dx, dy) = anchor_in_frame(origin_x, origin_y, frame.w, frame.h);
    blit_rgb555(
        frame,
        &layer.image.view(),
        dx + layer.dx,
        dy + layer.dy,
        Some(layer.colorkey),
    );
}

/// Room rows at their staged `.bnd` panel origins, one live room per
/// panel: the number at its pinned plate origin, the title and the
/// seat count at theirs, each sized to its template rect — the live
/// splits (`findings.md` §2) replace the old combined string. Bare
/// on the panel art (no scrim: the panels carry their own chrome).
/// Rooms past the staged panels stay off-page (the Left/Right pager
/// is untraced); the highlight cursor rides the selected panel, while
/// the mouse-over outline rides the hovered panel.
pub(crate) fn paint_room_panel_rows(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
    origins: &[(i32, i32)],
    hovered_panel: Option<usize>,
) {
    let ink = caption_ink(art_title_ink());
    let panels = room_panel_instances(&art.placements);
    for (index, room) in rooms.iter().enumerate() {
        let Some((panel_x, panel_y)) = origins.get(index).copied() else {
            break;
        };
        let Some(panel_id) = panels.get(index).copied() else {
            break;
        };
        paint_room_row_number(frame, art, room, panel_id);
        if let Some(title) = staged_field_rect(&art.placements, ROOM_DETAIL_TITLE_ID) {
            paint_room_text_into_rect(
                frame,
                Rect {
                    x: panel_x + ROOM_ROW_TITLE_ORIGIN.0,
                    y: panel_y + ROOM_ROW_TITLE_ORIGIN.1,
                    w: title.w,
                    h: title.h,
                },
                &room.title,
                ink,
            );
        }
        if let Some(count) = staged_field_rect(&art.placements, ROOM_DETAIL_USERS_ID) {
            paint_room_text_into_rect(
                frame,
                Rect {
                    x: panel_x + ROOM_ROW_COUNT_ORIGIN.0,
                    y: panel_y + ROOM_ROW_COUNT_ORIGIN.1,
                    w: count.w,
                    h: count.h,
                },
                &room_capacity_label(room),
                ink,
            );
        }
        if index == highlight {
            let cursor = anchor_in_frame(
                panel_x + ROOM_ROW_NUMBER_ORIGIN.0,
                panel_y + ROOM_ROW_NUMBER_ORIGIN.1,
                frame.w,
                frame.h,
            );
            paint_cursor(frame, cursor.0, cursor.1);
        }
    }
    paint_room_panel_hover_outline(frame, art, &panels, hovered_panel);
}

/// Room rows at the legacy quarter-frame anchor (flat fallback and
/// art-less staged scenes only; the staged path above owns layout
/// while panels are bound).
pub(crate) fn paint_room_fallback_rows(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
) {
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
pub(crate) const ROOM_DETAIL_TITLE_ID: &str = "IDC_TEXT_SROOM_ROOMNAME";
/// Seat-count anchor (`ROOM_USER`).
pub(crate) const ROOM_DETAIL_USERS_ID: &str = "IDC_TEXT_SROOM_ROOM_USER";
/// Selected-song anchor.
pub(crate) const ROOM_DETAIL_MUSIC_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE";
/// Difficulty anchor (second music-title slot).
pub(crate) const ROOM_DETAIL_DIFFICULTY_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE2";
/// Scroll-speed icon anchor; text stands in for the icon body.
pub(crate) const ROOM_DETAIL_SPEED_ID: &str = "IDC_IMAGE_SROOM_SPEED";
/// Game-mode icon anchor (`Room_Mode.ojs`); text stands in.
pub(crate) const ROOM_DETAIL_MODE_ID: &str = "IDC_IMAGE_SROOM_MODE";
/// Key-mode anchor (`Room_Om.ojs` option-mode icon).
pub(crate) const ROOM_DETAIL_KEY_ID: &str = "IDC_IMAGE_SROOM_OM";
/// Level-gate text anchor beside `Room_LevelLimit.ojs`.
pub(crate) const ROOM_DETAIL_LEVEL_ID: &str = "IDC_TEXT_SROOM_LEVEL_RANGE";
/// Lock-icon anchor (`Room_Password.ojs`); text stands in.
pub(crate) const ROOM_DETAIL_LOCK_ID: &str = "IDC_IMAGE_SROOM_PASSWORD";

/// Detail lines per panel: title plus the eight entry fields.
pub(crate) const ROOM_DETAIL_LINES: usize = 9;

/// Game-mode label (`NORMAL`/`LIVE`; raw code while unknown).
#[must_use]
pub(crate) fn room_mode_label(game_mode: u8) -> String {
    match GameMode::from_u8(game_mode) {
        Ok(GameMode::Normal) => "NORMAL".to_owned(),
        Ok(GameMode::Live) => "LIVE".to_owned(),
        Err(_) => format!("MODE {game_mode}"),
    }
}

/// Selected-song label (the song table lives server-side, so the id).
#[must_use]
pub(crate) fn room_music_label(music_id: u16) -> String {
    format!("M{music_id}")
}

/// Difficulty label (`EX`/`NX`/`HX`/`RX`; raw code while unknown).
#[must_use]
pub(crate) fn room_difficulty_label(difficulty: u8) -> String {
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
pub(crate) fn room_key_label(key_mode: u8) -> String {
    match KeyMode::from_u8(key_mode) {
        Ok(KeyMode::ThreeKeys) => "3KEY".to_owned(),
        Ok(KeyMode::FiveKeys) => "5KEY".to_owned(),
        Ok(KeyMode::SevenKeys) => "7KEY".to_owned(),
        Err(_) => format!("KEY {key_mode}"),
    }
}

/// Scroll-speed label (`X0.5`..`X6`, `XR`; raw code while unknown).
#[must_use]
pub(crate) fn room_speed_label(speed: u8) -> String {
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
pub(crate) fn room_capacity_label(room: &RoomListEntry) -> String {
    format!("{}/{}", room.user_count, room.capacity)
}

/// Level-gate label (`LV min-max`).
#[must_use]
pub(crate) fn room_level_label(room: &RoomListEntry) -> String {
    format!("LV {}-{}", room.min_level, room.max_level)
}

/// Lock label (`LOCK` with a password, `OPEN` without).
#[must_use]
pub(crate) fn room_lock_label(has_password: bool) -> &'static str {
    if has_password {
        "LOCK"
    } else {
        "OPEN"
    }
}

/// Lock-line ink: red while a password is set, dim otherwise.
pub(crate) fn room_lock_ink(locked: bool) -> u32 {
    if locked {
        pack_xrgb(0xFF, 0x50, 0x50)
    } else {
        pack_xrgb(0x90, 0x90, 0x90)
    }
}

/// Ordered detail entries for one room: (`State_Room.bnd`
/// control id, text).
#[must_use]
pub(crate) fn room_detail_entries(
    room: &RoomListEntry,
) -> [(&'static str, String); ROOM_DETAIL_LINES] {
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

/// Fallback origin of detail line `slot` without staged art: stacked
/// rows under the list so the list plus cursor keep their room.
pub(crate) fn room_detail_fallback_origin(
    slot: usize,
    row_count: usize,
    frame: &Frame<'_>,
) -> (i32, i32) {
    let rows = i32::try_from(row_count).unwrap_or(i32::MAX);
    let slot_offset = i32::try_from(slot).unwrap_or(i32::MAX);
    (
        frame.w as i32 / 4,
        frame.h as i32 / 4 + rows * CHANNEL_ROW_H + 4 + slot_offset * CHANNEL_ROW_H,
    )
}

/// One detail line: dark scrim plus ink (shared by the flat fallback
/// and the art scene).
pub(crate) fn paint_room_detail_line(frame: &mut Frame<'_>, x: i32, y: i32, text: &str, ink: u32) {
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

/// Padlock body of the row template (`IDC_IMAGE_SROOM_PASSWORD`,
/// `0x1350002C`): the real sprite has one 15x16 image, so frame 1
/// paints the padlock and flag 0 leaves the site untouched (exe row
/// fill `0x126872`–`0x12687d`, docs/room-enums.md).
pub const ROOM_PASSWORD_SPRITE: &str = "Room_Password.ojs";

/// Whether a staged password clone belongs to a locked live row. The
/// one-image body must not paint for flag 0; an unowned placement is
/// chrome and keeps its normal paint.
fn password_clone_is_locked(
    placed: &ScenePlacement,
    panels: &[&str],
    rooms: &[RoomListEntry],
) -> bool {
    if placed.role != PlacementRole::Clone || placed.sprite.as_deref() != Some(ROOM_PASSWORD_SPRITE)
    {
        return true;
    }
    let Some(index) = panels.iter().position(|panel| *panel == placed.id) else {
        return true;
    };
    rooms.get(index).is_some_and(|room| room.has_password)
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
pub(crate) fn room_state_icon_frame(state_code: u8) -> u8 {
    state_code
}

/// `Room_Mode.ojs` frame for one room: key mode 0 → frame 1, 2 →
/// frame 2, anything else untouched (`None` — the exe leaves the
/// icon alone; icon sites `0x1267ad`–`0x1267e2`).
#[must_use]
pub(crate) fn room_key_icon_frame(key_mode: u8) -> Option<u8> {
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
pub(crate) fn room_om_icon_frame(music_id: u16, level: Option<u16>) -> Option<u8> {
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
pub(crate) fn icon_frame_layer(frame: u8, layers: usize) -> Option<usize> {
    if layers == 0 {
        return None;
    }
    Some(usize::from(frame.saturating_sub(1)).min(layers - 1))
}

/// Paint one frame of a staged row-template icon under one panel
/// instance: the `SET` clone owned by `panel_id` supplies the rect
/// (parent origin already added by staging), and the selected frame
/// blits 1:1 at that rect plus its record anchor (clones keep
/// template-local anchors, see [`PlacementRole::Clone`]). A missing
/// clone, figure, frame, or layer paints nothing — the exe leaves an
/// untouched icon exactly so.
pub(crate) fn paint_icon_frame(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    icon_frame: Option<u8>,
    panel_id: &str,
) {
    let Some(icon_frame) = icon_frame else {
        return;
    };
    let Some(placed) = placements.iter().find(|placed| {
        placed.id == panel_id
            && placed.role == PlacementRole::Clone
            && placed.sprite.as_deref() == Some(sprite)
    }) else {
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

/// Row icons of every populated room: state, key mode, and Om
/// pudding count, painted from that row's wire fields into the
/// `SET 0x11` clone staged under its panel (`Room_State.ojs`
/// `(203,63)`, `Room_Mode.ojs` `(9,62)`, `Room_Om.ojs` `(56,62)` —
/// panel `(26,82)` + rect in the 7.55 room capture,
/// `/home/arnim/scratch/ac-room.png`). Panels past the list keep
/// their bare plate: the exe clones the template only for populated
/// rows. Missing art paints nothing; the detail text draws on top.
pub(crate) fn paint_room_row_icons(frame: &mut Frame<'_>, art: &SceneArt, rooms: &[RoomListEntry]) {
    let panel_ids = room_panel_instances(&art.placements);
    for (index, room) in rooms.iter().enumerate() {
        let Some(panel_id) = panel_ids.get(index) else {
            break;
        };
        paint_icon_frame(
            frame,
            &art.placements,
            &art.figures,
            ROOM_STATE_SPRITE,
            Some(room_state_icon_frame(room.state_code)),
            panel_id,
        );
        paint_icon_frame(
            frame,
            &art.placements,
            &art.figures,
            ROOM_MODE_SPRITE,
            room_key_icon_frame(room.key_mode),
            panel_id,
        );
        paint_icon_frame(
            frame,
            &art.placements,
            &art.figures,
            ROOM_OM_SPRITE,
            room_om_icon_frame(room.music_id, None),
            panel_id,
        );
    }
}

/// Icon body behind one icon-rect detail label (`None` for the
/// text-only fields): the label stands in for the icon body, so a
/// staged figure silences it and the icon pass owns the rect.
pub(crate) fn room_icon_sprite_for_detail(id: &str) -> Option<&'static str> {
    if id == ROOM_DETAIL_MODE_ID {
        Some(ROOM_MODE_SPRITE)
    } else if id == ROOM_DETAIL_KEY_ID {
        Some(ROOM_OM_SPRITE)
    } else {
        None
    }
}

/// Details of the highlighted room: song, difficulty, key mode,
/// game mode, speed, level gate, lock — the template fields the row
/// string does not carry (number, title, and seat count ride the
/// pinned row fields instead; the count's `ROOM_USER` rect paints
/// background in the live capture). Staged details paint at the
/// highlighted panel's origin plus their `State_Room.bnd` rects, sized
/// to the rects at real sizes (the TEXT rects are 16-20 px tall; the
/// icon-rect labels ride the same fit); icon-rect labels stand down
/// while their icon figure stages (the icon pass owns the rect).
/// Without staged art the stacked fallback lines keep the list
/// readable. Nothing paints without a room, or without a staged
/// panel under the highlight.
pub(crate) fn paint_room_details(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: Option<&SceneArt>,
) {
    let Some(room) = rooms.get(highlight) else {
        return;
    };
    let Some(art) = art else {
        for (slot, (id, text)) in room_detail_entries(room).iter().enumerate() {
            let ink = if *id == ROOM_DETAIL_LOCK_ID {
                room_lock_ink(room.has_password)
            } else {
                art_title_ink()
            };
            let (x, y) = room_detail_fallback_origin(slot, rooms.len(), frame);
            paint_room_detail_line(frame, x, y, text, ink);
        }
        return;
    };
    let placements = &art.placements;
    let Some((panel_x, panel_y)) = room_panel_origins(placements).get(highlight).copied() else {
        return;
    };
    for (id, text) in room_detail_entries(room).iter() {
        // Number, title, and seat count ride the pinned row fields
        // instead of the template rects (the count's `ROOM_USER` rect
        // paints background in the live capture).
        if *id == ROOM_DETAIL_TITLE_ID || *id == ROOM_DETAIL_USERS_ID {
            continue;
        }
        // A staged icon figure owns its rect; the text label stands
        // down (it is the figure-less stand-in).
        if room_icon_sprite_for_detail(id).is_some_and(|sprite| art.figures.contains_key(sprite)) {
            continue;
        }
        let Some(placement) = placements.iter().find(|entry| entry.id == *id) else {
            continue;
        };
        let ink = if *id == ROOM_DETAIL_LOCK_ID {
            room_lock_ink(room.has_password)
        } else {
            art_title_ink()
        };
        paint_room_text_into_rect(
            frame,
            Rect {
                x: placement.rect.x + panel_x,
                y: placement.rect.y + panel_y,
                w: placement.rect.w,
                h: placement.rect.h,
            },
            text,
            caption_ink(ink),
        );
    }
}

/// Room top-band chrome the 7.55 client never shows (`ticket #169`):
/// `MYCAPSULE`/`EVENT`/`MYBAND` stage faithfully (the #135 census
/// pins every top-band rect) but the room-setup path clears each
/// control's draw-path words, so the identical rects never collide on
/// screen. Pinned: the three codes occur exactly once each in
/// `AngelO2Jam.exe` (file `0x8ee00`/`0x8ee27`/`0x8ee4e`, the three
/// pushes of the room-setup tail at files `0x8edff`/`0x8ee26`/`0x8ee4d`),
/// each followed by a `GetByCode 0x48bcf0` lookup (`ev_2398edc5…`) and
/// zeroing writes through the attribute setters `0x409bd0`/`0x409bf0`
/// (control words `+0x20`/`+0x24`, read by the control draw path —
/// `ev_890684f1…`); both arms of the enclosing conditional (file
/// `0x8ec9e`) converge on that tail, so the clear is unconditional,
/// and no other code in the binary names these controls (full-image
/// LE32 sweep: one hit per code, the pushes above). The reference
/// room capture (`~/scratch/ac-room.png`) shows none of the three.
/// Residual OPEN: cached-pointer writes cannot be excluded by a static
/// sweep — next step a stopped-state read of the three controls'
/// `+0x20`/`+0x24` words in a live room (the #113 method).
/// Staging stays frozen: this gates paint and dispatch only.
pub(crate) const ROOM_BUNDLED_CHROME_CODES: [u32; 3] = [0x1320_0038, 0x1320_0039, 0x1320_003A];

/// True while the staged room control `code` may paint and answer
/// input: everything stages, but the exe-hidden bundled chrome (see
/// [`ROOM_BUNDLED_CHROME_CODES`]) never reaches the screen.
pub(crate) fn room_chrome_visible(code: u32) -> bool {
    !ROOM_BUNDLED_CHROME_CODES.contains(&code)
}

/// Paint the online room list with a highlight cursor.
pub(crate) fn paint_room_list(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    fill_background(frame, Screen::Room);
    paint_title(frame, "ROOM");
    paint_room_rows(frame, rooms, highlight, None);
    paint_room_details(frame, rooms, highlight, None);
}

/// Paint staged lobby art with the live room list and cursor on top.
///
/// `hovered_code` is the staged chrome control under the pointer, if
/// any: the exe repaints that control at full body frame 0 while the
/// pointer sits on it (`docs/hover.md`). Row panels stay static — they
/// are not chrome controls, so the pointer over a panel repaints
/// nothing (negative evidence: 0 changed px on all six panels,
/// `Room_MOver.ojs` unmatched in either state).
pub fn paint_room_scene(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
    hovered_code: Option<u32>,
) {
    paint_room_scene_with_hover(frame, rooms, highlight, art, hovered_code, None);
}

/// Same staged scene, with the room panel under the pointer named so
/// the code-mounted mouse-over art can follow it.
pub fn paint_room_scene_with_hover(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
    hovered_code: Option<u32>,
    hovered_panel: Option<usize>,
) {
    paint_room_scene_with_selection(
        frame,
        rooms,
        highlight,
        art,
        hovered_code,
        hovered_panel,
        None,
    );
}

/// Same staged room board, with the selected mode tab pinned at body
/// frame 1 and the other two at frame 0. The exe's click arm marks the
/// pressed tab through `FUN_0048c810` and resets the others (REA
/// `ev_e9f783fce3671c057d37820439860dbf27036d606c2c75100eee35d4c61e0b23`);
/// `docs/hover.md` measures frame 1 on the active tab. Thus a mode
/// switch moves the frame with the selection (#290). `None` keeps the
/// older all-layers stack for callers that pin no tab.
/// Staged mode-filter tab codes, paired by `(code & 0xFFF) - 1` with
/// the `State_Room.bnd` entries (owner rule `0c6c3`).
pub(crate) const ROOM_MODE_TAB_CODES: [u32; 3] = [0x1320_1003, 0x1320_1004, 0x1320_1005];

/// One mode tab's frame: 1 when active, 0 when a sibling, `None` when
/// no selection was staged. The frame choice comes from the exe's
/// click arm (REA `ev_e9f783fce3671c057d37820439860dbf27036d606c2c75100eee35d4c61e0b23`).
fn selected_mode_tab_frame(active_tab_code: Option<u32>, placed: &ScenePlacement) -> Option<usize> {
    let active = active_tab_code?;
    (ROOM_MODE_TAB_CODES.contains(&placed.code) && placed.role == PlacementRole::Single)
        .then(|| usize::from(active == placed.code))
}

pub fn paint_room_scene_with_selection(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
    hovered_code: Option<u32>,
    hovered_panel: Option<usize>,
    active_tab_code: Option<u32>,
) {
    let panels = room_panel_instances(&art.placements);
    art.paint_backdrop_live_selected(
        frame,
        Screen::Room,
        |placed| {
            room_chrome_visible(placed.code)
                && clone_instance_live(placed, &panels, rooms.len())
                && password_clone_is_locked(placed, &panels, rooms)
        },
        |placed| selected_mode_tab_frame(active_tab_code, placed),
    );
    paint_room_row_icons(frame, art, rooms);
    paint_room_details(frame, rooms, highlight, Some(art));
    // Rows paint last so the live list and highlight cursor stay
    // visible over the staged template bodies (UI ink on top): the
    // `SET 0x11` template rects are panel-relative while rows ride
    // per-panel rects, so the template art overlaps its own panel —
    // the exe's per-row template clone offsets stay OPEN past the
    // title/number/count pins (`findings.md` §2).
    paint_room_rows_with_hover(frame, rooms, highlight, Some(art), hovered_panel);
    paint_room_hover(frame, art, hovered_code);
}

/// Hover repaint for one staged room chrome button (`docs/hover.md`
/// 2026-11-10 rig sweep): the exe repaints the hovered control at
/// full body frame 0, so the shell repaints that frame alone over the
/// stacked rest paint. The rest-state desaturation transform is
/// measured but unexplained (rest matches neither body record), so the
/// rest paint keeps today's stacked look — labeled OPEN, next step a
/// stopped-state winedbg draw-state read per `docs/hover.md`. Active
/// filter/tab codes never reach this painter (the resolver holds them
/// at rest — EXE-pinned no-change), so every code here repaints layer
/// 0. Mode-filter tabs now rest at their selected frame 0/1 through
/// [`paint_room_scene_with_selection`]; their exact desaturated rest
/// blend stays OPEN with the same stop-state draw read as the next
/// step. A missing placement, sprite, or frame paints nothing.
pub(crate) fn paint_room_hover(frame: &mut Frame<'_>, art: &SceneArt, hovered_code: Option<u32>) {
    let Some(code) = hovered_code else {
        return;
    };
    let Some(placed) = art.placements.iter().find(|placed| placed.code == code) else {
        return;
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placed
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    paint_placement_layer(frame, placed, &art.figures, 0, shares_body, None);
}

/// Room user-panel control ids (`STATE_ROOM` text/image wells; the
/// rects pair through the verified ordinal rule).
pub(crate) const ROOM_NICKNAME_ID: &str = "IDC_TEXT_SROOM_NICKNAME";
/// Level well (`Level` row).
pub(crate) const ROOM_PANEL_LEVEL_ID: &str = "IDC_TEXT_SROOM_MYLEVEL";
/// Experience well (`Exp.` row).
pub(crate) const ROOM_PANEL_EXP_ID: &str = "IDC_TEXT_SROOM_EXP";
/// Grade well (`정·직` row).
pub(crate) const ROOM_PANEL_GRADE_ID: &str = "IDC_TEXT_SROOM_WINLOSE";
/// Cash-point well (`Cash Point` row).
pub(crate) const ROOM_PANEL_CASH_ID: &str = "IDC_TEXT_SROOM_BONUSPOINT";
/// Next-up-exp well (`Next Up Exp.` row, code `0x13400042`, staged
/// `(720,377)-(777,388)`): the 7.55 room capture shows the current
/// experience there (`2133`, equal to `Exp.`), so the panel paints
/// `CharacterInfo.experience` pending a two-account rig capture.
pub(crate) const ROOM_PANEL_NEXT_UP_EXP_ID: &str = "IDC_TEXT_SROOM_NEXTUPEXP";
/// Penalty-count well (`패널티횟수` row).
pub(crate) const ROOM_PANEL_PENALTY_COUNT_ID: &str = "IDC_TEXT_SROOM_PENALTYCOUNT";
/// Penalty-level well (`패널티레벨` row).
pub(crate) const ROOM_PANEL_PENALTY_LEVEL_ID: &str = "IDC_TEXT_SROOM_PENALTYLEVEL";
/// Band well (`나의밴드` row).
pub(crate) const ROOM_PANEL_BAND_ID: &str = "IDC_TEXT_SROOM_GUILD";
/// Worn-figure slot (`IDC_IMAGE_SROOM_AVATA`, code `0x13700013`).
pub(crate) const ROOM_AVATAR_ID: &str = "IDC_IMAGE_SROOM_AVATA";

/// Placeholder the exe paints in the grade and band wells: the 7.55
/// room capture (`/home/arnim/scratch/ac-room.png`, tester Lv 2) shows
/// `준비중..` in both, beside real values in the other wells.
pub(crate) const ROOM_PANEL_PLACEHOLDER: &str = "준비중..";

/// Paint the room's user panel: the worn figure in its staged slot and
/// the identity readouts in their staged text wells.
///
/// Wells come from the parsed `.bnd` rects (owner rule `f38f3`); a well
/// the staged art does not name stays unpainted. Values ride
/// `GetCharacterInfo` 2001 (`CharacterInfo`) — the same record the
/// MyRoom scene reads — and the nickname line prefixes the level, as
/// the 7.55 room capture shows (`Lv.2 tester`).
///
/// `Next Up Exp.` carries the current experience: the 7.55 room capture
/// (`/home/arnim/scratch/ac-room.png`, tester Lv 2) shows `2133` there,
/// equal to the `Exp.` well. Single-capture pin (n=1): OPEN whether it
/// tracks Exp or a level threshold — next step is a two-account rig
/// capture at different exp/level, plus identifying the setter among
/// the twelve `0x50f520` sites in `FUN_00491af0`.
///
/// OPEN wells (nothing painted, next steps named):
/// - `MYRANK` / `NOTICE_TEXT` / `DESC`: web-served text, no client
///   source (the capture shows notice text in `DESC`, so the bottom
///   plate paints bare).
pub fn paint_room_user_panel(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    character: Option<&CharacterInfo>,
    figure: Option<&AvatarFigure>,
    now_ms: u64,
) {
    if let (Some(figure), Some(slot)) = (figure, staged_field_rect(placements, ROOM_AVATAR_ID)) {
        let slot = scale_rect_to_frame(&slot, frame.w, frame.h);
        figure.paint_visible_centered(frame, &slot, frame.w, frame.h, now_ms);
    }
    if let Some(character) = character {
        let ink = caption_ink(art_title_ink());
        let nickname = format!("Lv.{} {}", character.level, character.nickname);
        paint_room_panel_value(frame, placements, ROOM_NICKNAME_ID, &nickname, ink);
        paint_room_panel_number(frame, placements, ROOM_PANEL_LEVEL_ID, character.level, ink);
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_EXP_ID,
            character.experience,
            ink,
        );
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_CASH_ID,
            character.bonus_point,
            ink,
        );
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_NEXT_UP_EXP_ID,
            character.experience,
            ink,
        );
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_PENALTY_COUNT_ID,
            i32::from(character.penalty_count),
            ink,
        );
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_PENALTY_LEVEL_ID,
            i32::from(character.penalty_level),
            ink,
        );
    }
    // The grade and band wells carry the exe's own placeholder.
    let ink = caption_ink(art_title_ink());
    for id in [ROOM_PANEL_GRADE_ID, ROOM_PANEL_BAND_ID] {
        paint_room_panel_value(frame, placements, id, ROOM_PANEL_PLACEHOLDER, ink);
    }
}

/// Paint the first two default equipment control ids into the staged
/// MODE/OM button pair of the first room panel (#959). The ids come
/// from the codec's channel-keyed sets (`0x540e40`); without a staged
/// panel nothing is painted. Both labels are id readouts: no exe glyph
/// mapping for these ids is pinned, so the painter invents no art.
pub fn paint_channel_button_pair(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    ids: [u32; 5],
) {
    let ink = caption_ink(art_title_ink());
    for (id, control) in ids.iter().zip([
        "IDC_BUTTON_SROOM_SHOW_NORMALMODE",
        "IDC_BUTTON_SROOM_SHOW_BATTLEMODE",
    ]) {
        let Some(rect) = staged_field_rect(placements, control) else {
            continue;
        };
        paint_text_into_rect(frame, rect, &format!("{id:#x}"), ink);
    }
}

/// Paint the full 32-bit member word a 2023 event staged into the
/// selected-song well of the first room panel (#990). The low twelve
/// bits ride the normal music label; the shifted high nibble is shown
/// as the hex word so the OR is visible without inventing an exe
/// caption.
pub fn paint_room_member_word(frame: &mut Frame<'_>, placements: &[ScenePlacement], word: u32) {
    let Some(rect) = staged_field_rect(placements, ROOM_DETAIL_MUSIC_ID) else {
        return;
    };
    paint_text_into_rect(
        frame,
        rect,
        &format!("W{word:#x}"),
        caption_ink(art_title_ink()),
    );
}

/// One integer into its staged well.
pub(crate) fn paint_room_panel_number(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    id: &str,
    value: i32,
    ink: [u8; 4],
) {
    paint_room_panel_value(frame, placements, id, &value.to_string(), ink);
}

/// One value into its staged well: left-anchored, sized to the rect at
/// the exe's GDI cap share (the same fit the row fields use).
pub(crate) fn paint_room_panel_value(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    id: &str,
    text: &str,
    ink: [u8; 4],
) {
    let Some(rect) = staged_field_rect(placements, id) else {
        return;
    };
    paint_text_into_rect(frame, rect, text, ink);
}
