//! Song-select dialog structure painters: the `MUSELECT 0x16200003`
//! dialog caption columns, result tabs, catalog-driven search table,
//! status + pager, search row, chosung grid, and OK pill painted over
//! the staged art (`reference-musicselect-4x3.png`). Shared with the
//! waiting screen's room-master view in [`super::game`].

use super::super::avatar::ScenePlacement;
use super::side::{
    select_chosung_cells, select_chosung_layout, select_difficulty_ink, select_pager_caption,
    select_picked_caption, select_search_caption, select_status_line, SelectMusicRow,
    SELECT_ARTIST_RADIO, SELECT_CHO_SUNG_LEFT, SELECT_CHO_SUNG_PITCH, SELECT_CHO_SUNG_ROW_CELLS,
    SELECT_CHO_SUNG_Y, SELECT_FILTER_BUTTON_H, SELECT_FILTER_BUTTON_PITCH,
    SELECT_FILTER_BUTTON_TOP, SELECT_FILTER_COLUMNS, SELECT_GENRE_FILTERS, SELECT_HEADER_COLUMNS,
    SELECT_HEADER_Y, SELECT_LEVEL_FILTERS, SELECT_OK_BUTTON, SELECT_PAGER_RIGHT,
    SELECT_RESULT_TABS, SELECT_ROW_PITCH, SELECT_ROW_TOP, SELECT_SEARCH_BOX, SELECT_SEARCH_BUTTON,
    SELECT_SEARCH_Y, SELECT_STATUS_H, SELECT_STATUS_Y, SELECT_TABLE_HEADERS, SELECT_TABLE_LEFT,
    SELECT_TABLE_WIDTH, SELECT_TITLE_RADIO, SELECT_VISIBLE_ROWS, SELECT_WHEEL_FILTERS,
};
use o2jam_net::play::SetRoomMusic;
use o2jam_render::font::{draw_text_cell, Cell};
use o2jam_render::{draw_text, fill_rect, pack_xrgb, scale_rect_to_frame, text_width, Frame, Rect};

// --- Song-select dialog structure painters (`reference-musicselect-4x3.png`) ---

/// Filter-column headers (reference 곡레벨선택/래별선택/장르선택;
/// ASCII stand-ins).
const SELECT_FILTER_HEADERS: [&str; 3] = ["LEVEL", "KEY", "GENRE"];

/// Structure panel scrim behind the stand-in captions.
pub(crate) fn select_scrim() -> u32 {
    pack_xrgb(0x10, 0x10, 0x18)
}

/// Caption ink (white).
pub(crate) fn select_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Ink for unknown cells and the inactive tab.
pub(crate) fn select_dim_ink() -> u32 {
    pack_xrgb(0x90, 0x90, 0x90)
}

/// Ink of the active search tab (reference orange).
pub(crate) fn select_active_ink() -> u32 {
    pack_xrgb(0xFF, 0xA0, 0x20)
}

/// Staged title-radio fill (reference green dot).
pub(crate) fn select_radio_ink() -> u32 {
    pack_xrgb(0x30, 0xE0, 0x30)
}

/// Frame-space step `index * pitch` for native row lists (saturates).
pub(crate) fn select_step(index: usize, pitch: i32) -> i32 {
    i32::try_from(index).map_or(i32::MAX, |step| step.saturating_mul(pitch))
}

/// Band height for one native `pitch` (frame rects size in `u32`).
pub(crate) fn select_band_h(pitch: i32) -> u32 {
    u32::try_from(pitch).unwrap_or(0)
}

/// Native-pixel anchor scaled into the frame (the shared art ratios).
pub(crate) fn select_anchor(rect: Rect, frame: &Frame<'_>) -> Rect {
    scale_rect_to_frame(&rect, frame.w, frame.h)
}

/// One native x scaled into the frame.
pub(crate) fn select_x(x: i32, frame: &Frame<'_>) -> i32 {
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
pub(crate) fn select_y(y: i32, frame: &Frame<'_>) -> i32 {
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
pub(crate) fn paint_select_caption(frame: &mut Frame<'_>, panel: Rect, text: &str, ink: u32) {
    let spot = select_anchor(panel, frame);
    fill_rect(frame, &spot, select_scrim());
    draw_text(frame, spot.x + 2, spot.y + 2, text, ink);
}

/// The three filter columns: header band plus one button per caption
/// (song-level bands, wheel buckets, genres).
pub(crate) fn paint_select_filter_columns(frame: &mut Frame<'_>) {
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
pub(crate) fn paint_select_column(
    frame: &mut Frame<'_>,
    panel: Rect,
    header: &str,
    captions: &[&str],
) {
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
pub(crate) fn paint_select_result_tabs(frame: &mut Frame<'_>, row_count: usize) {
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
/// [`SELECT_VISIBLE_ROWS`] catalog rows. The picked row (if the room
/// song names one) draws its difficulty letter in the active-tab
/// orange — a stand-in highlight (the exe's picked-row rendering is
/// unpinned, OPEN).
/// Whether a search row is the room song: song id plus difficulty
/// slot both match the staged pick. The slot codes repeat
/// Easy/Normal/Hard in `Difficulty` code order, so a plain equality
/// on the slot's code names the picked chart.
pub(crate) fn is_picked_row(row: SelectMusicRow<'_>, picked: Option<SetRoomMusic>) -> bool {
    picked.is_some_and(|song| {
        u32::from(song.music_id) == row.song.song_id
            && u8::try_from(row.slot).ok() == Some(song.difficulty)
    })
}

pub(crate) fn paint_select_table(
    frame: &mut Frame<'_>,
    rows: &[SelectMusicRow<'_>],
    picked: Option<SetRoomMusic>,
) {
    paint_select_table_header(frame);
    for (index, row) in rows.iter().take(SELECT_VISIBLE_ROWS).enumerate() {
        paint_select_table_row(frame, index, *row, picked);
    }
}

/// Header band across the table plus the nine column captions.
pub(crate) fn paint_select_table_header(frame: &mut Frame<'_>) {
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
/// The picked row's letter inks active-orange (stand-in highlight,
/// see [`paint_select_table`]).
pub(crate) fn paint_select_table_row(
    frame: &mut Frame<'_>,
    index: usize,
    row: SelectMusicRow<'_>,
    picked: Option<SetRoomMusic>,
) {
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
    let letter_ink = if is_picked_row(row, picked) {
        select_active_ink()
    } else {
        select_difficulty_ink(row.slot)
    };
    draw_text(
        frame,
        select_x(SELECT_HEADER_COLUMNS[2], frame),
        y,
        row.letter(),
        letter_ink,
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
pub(crate) fn paint_select_status_row(frame: &mut Frame<'_>, row_count: usize) {
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
pub(crate) fn paint_select_search_row(frame: &mut Frame<'_>) {
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
/// source). Row cell counts and the x-pitch come from the staged
/// INDEX-button inventory when the dialog art stages
/// ([`select_chosung_layout`], FILE per `docs/geometry-audit.md`);
/// without placements the audited constants drive the grid.
pub(crate) fn paint_select_chosung_rows(frame: &mut Frame<'_>, placements: &[ScenePlacement]) {
    let layout = select_chosung_layout(placements);
    let cells = select_chosung_cells();
    let mut first = 0;
    let row_cells: &[usize] = layout
        .as_ref()
        .map_or(&SELECT_CHO_SUNG_ROW_CELLS, |layout| &layout.row_cells);
    let pitch = layout
        .as_ref()
        .map_or(SELECT_CHO_SUNG_PITCH, |layout| layout.pitch);
    for (row, count) in row_cells.iter().enumerate() {
        let Some(y_native) = SELECT_CHO_SUNG_Y.get(row).copied() else {
            break;
        };
        let y = select_y(y_native + 2, frame);
        for cell in 0..*count {
            let Some(letter) = cells.get(first + cell).copied().flatten() else {
                continue;
            };
            let caption = letter.to_string();
            draw_text_cell(
                frame,
                select_x(SELECT_CHO_SUNG_LEFT + select_step(cell, pitch), frame),
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
pub(crate) fn paint_select_ok(frame: &mut Frame<'_>) {
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
