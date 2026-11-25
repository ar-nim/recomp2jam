//! Song-select dialog text layer: the live captions the
//! `DAILOG_SELECTMUSIC` control list declares as `TEXT` controls
//! (result counts, status line, pager, per-row title/artist/level),
//! inked at the slots their `.bnd`/`SET` templates place.
//!
//! Everything else the dialog shows is a bound sprite: the `.bnd` pairs
//! every `IDC_BUTTON_DSMUSIC_*` entry by code ordinal and the bodies
//! carry their own labels (`SelectMusic_Btn_Step1_all.ojs` names its
//! 전체, `SelectMusic_OK.ojs` its OK), so this layer draws glyphs only —
//! no plate, no scrim. An opaque panel here covers the staged button
//! art and is exactly the black rectangles the owner playtest reported
//! (ticket #272); the pinned reference
//! `reference-musicselect-4x3.png` shows art chrome with live text on
//! top and nothing else.

use super::super::avatar::ScenePlacement;
use super::side::{
    select_pager_caption, select_picked_caption, select_search_caption, select_status_line,
    SelectMusicRow,
};
use o2jam_net::play::SetRoomMusic;
use o2jam_render::{draw_text, pack_xrgb, scale_rect_to_frame, Frame, Rect};

/// Live result-count caption (`검색결과 (n곡)`): the search tab's TEXT
/// control, staged at `Dialog_SelectMusic.bnd` rect 48.
const SEARCH_TAB_TEXT_ID: &str = "IDC_TEXT_DSMUSIC_TABRESULT";
/// Picked-list caption (`선곡결과 (0곡)`): the second tab's TEXT control.
const PICKED_TAB_TEXT_ID: &str = "IDC_TEXT_DSMUSIC_TABPLAYLIST";
/// Status line under the rows (`전체 -> 전체 검색 결과입니다.`).
const STATUS_TEXT_ID: &str = "IDC_TEXT_DSMUSIC_SERACHRESHLT";
/// Pager digits (`3/161`).
const PAGER_TEXT_ID: &str = "IDC_TEXT_DSMUSIC_PAGENUM";
/// Per-row text templates of `SET 0x0B 0x00017000`: the rects are
/// parent-local, so row `n` inks them at its band origin plus the
/// template rect (the clone rule every `SET` member follows,
/// `expand_set_clones`, `docs/ui-loader.md` step 8).
const ROW_TITLE_TEXT_ID: &str = "IDC_TEXT_DSMUSIC_TITLE";
const ROW_ARTIST_TEXT_ID: &str = "IDC_TEXT_DSMUSIC_ARTIST";
const ROW_LEVEL_TEXT_ID: &str = "IDC_TEXT_DSMUSIC_TXT_DIFF";
/// The spriteless row hit-targets (`EXTEND1..10`): one per visible row,
/// top to bottom. Their look is the backdrop's, so they carry no
/// sprite — only geometry.
pub(crate) const ROW_BAND_ID_PREFIX: &str = "IDC_BUTTON_DSMUSIC_EXTEND";

/// GDI cell bearing for select-dialog text ink, in native pixels.
/// `Dialog_SelectMusic.bnd` band 150 plus `SET` template y 6 puts the
/// first row's cell top at y 156; the 7.55 reference frame
/// (`~/scratch/av-reports/frames/ref_musicselect.png`, #349) inks row 1
/// from y 161. The rasterizer anchors cap tops at the cell top, so
/// this bearing moves only the ink, never the staged geometry.
const SELECT_TEXT_INK_BEARING_Y: i32 = 5;

/// Caption ink (white, the exe's dialog text tone).
pub(crate) fn select_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Ink for a cell without a staged source (a personal grade or a
/// catalog row with no level word; they draw the unknown dash).
pub(crate) fn select_dim_ink() -> u32 {
    pack_xrgb(0x90, 0x90, 0x90)
}

/// Ink of the active search tab (reference orange).
pub(crate) fn select_active_ink() -> u32 {
    pack_xrgb(0xFF, 0xA0, 0x20)
}

/// One row's staged text slots (native pixels): the band origin plus
/// the three `SET` template slots cloned onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectRowSlots {
    title: Rect,
    artist: Rect,
    level: Rect,
}

/// Every staged slot the text layer inks. Empty fields mean the staged
/// art names no such control: the exe leaves an unstaged control alone,
/// so nothing inks there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SelectTextSlots {
    search_tab: Option<Rect>,
    picked_tab: Option<Rect>,
    status: Option<Rect>,
    pager: Option<Rect>,
    rows: Vec<SelectRowSlots>,
}

impl SelectTextSlots {
    /// Resolve the text layer against one staged dialog's placements.
    #[must_use]
    pub(crate) fn from_placements(placements: &[ScenePlacement]) -> Self {
        Self {
            search_tab: control_rect(placements, SEARCH_TAB_TEXT_ID),
            picked_tab: control_rect(placements, PICKED_TAB_TEXT_ID),
            status: control_rect(placements, STATUS_TEXT_ID),
            pager: control_rect(placements, PAGER_TEXT_ID),
            rows: row_slots(placements),
        }
    }
}

/// Native rect of one staged control (`None` while the art names none).
fn control_rect(placements: &[ScenePlacement], id: &str) -> Option<Rect> {
    placements
        .iter()
        .find(|placed| placed.id == id)
        .map(|placed| placed.rect)
}

/// One slot set per spriteless `EXTEND` band, top to bottom; empty
/// while the `SET` text templates are unstaged.
fn row_slots(placements: &[ScenePlacement]) -> Vec<SelectRowSlots> {
    let (Some(title), Some(artist), Some(level)) = (
        control_rect(placements, ROW_TITLE_TEXT_ID),
        control_rect(placements, ROW_ARTIST_TEXT_ID),
        control_rect(placements, ROW_LEVEL_TEXT_ID),
    ) else {
        return Vec::new();
    };
    let mut bands: Vec<Rect> = placements
        .iter()
        .filter(|placed| placed.sprite.is_none() && placed.id.starts_with(ROW_BAND_ID_PREFIX))
        .map(|placed| placed.rect)
        .collect();
    bands.sort_by_key(|rect| rect.y);
    bands
        .into_iter()
        .map(|band| SelectRowSlots {
            title: slot_on_band(band, title),
            artist: slot_on_band(band, artist),
            level: slot_on_band(band, level),
        })
        .collect()
}

/// One `SET` template slot cloned onto its row band (parent origin plus
/// the parent-local template rect).
fn slot_on_band(band: Rect, template: Rect) -> Rect {
    Rect {
        x: band.x.saturating_add(template.x),
        y: band.y.saturating_add(template.y),
        ..template
    }
}

/// Live captions the game stages for one paint (#285): the result
/// page and an optional status override (a refused pick's notice or
/// the current search banner).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SelectTextLive<'a> {
    /// Zero-based page the pager caption names.
    pub page: usize,
    /// Text replacing the default search banner, if any.
    pub status: Option<&'a str>,
}

/// Paint the dialog's live text over the staged art: tab counts, status
/// line, pager, then one line per staged row (title, artist, level).
///
/// `live.status` replaces the status line while a refused pick's
/// caption is up (the pinned not-downloaded notice, §3), or while a
/// search is staged. `live.page` is the pager's current page (#285).
pub(crate) fn paint_select_text(
    frame: &mut Frame<'_>,
    slots: &SelectTextSlots,
    rows: &[SelectMusicRow<'_>],
    picked: Option<SetRoomMusic>,
    live: SelectTextLive<'_>,
) {
    paint_slot(
        frame,
        slots.search_tab,
        &select_search_caption(rows.len()),
        select_active_ink(),
    );
    paint_slot(
        frame,
        slots.picked_tab,
        select_picked_caption(),
        select_dim_ink(),
    );
    paint_slot(
        frame,
        slots.status,
        live.status.unwrap_or(select_status_line()),
        select_ink(),
    );
    paint_slot(
        frame,
        slots.pager,
        &select_pager_caption(rows.len(), live.page),
        select_ink(),
    );
    for (slot, row) in slots.rows.iter().zip(rows) {
        paint_select_row(frame, slot, *row, picked);
    }
}

/// One search-result row: the catalog title, artist and per-chart level
/// at their `SET` slots (a zeroed level keeps the dash). The note and
/// grade icons are staged art (their bodies carry the letter stamps),
/// so the layer inks no letters; the picked row's title takes the
/// active-tab orange.
fn paint_select_row(
    frame: &mut Frame<'_>,
    slot: &SelectRowSlots,
    row: SelectMusicRow<'_>,
    picked: Option<SetRoomMusic>,
) {
    let title_ink = if is_picked_row(row, picked) {
        select_active_ink()
    } else {
        select_ink()
    };
    paint_slot(frame, Some(slot.title), &row.song.title, title_ink);
    paint_slot(frame, Some(slot.artist), &row.song.artist, select_ink());
    paint_slot(
        frame,
        Some(slot.level),
        &unknown_cell(row.level()),
        select_dim_ink(),
    );
}

/// Unstaged-column text: a level or grade without a source draws the
/// dash, never an invented number.
fn unknown_cell(value: Option<u8>) -> String {
    value.map_or_else(|| "-".to_owned(), |number| number.to_string())
}

/// Whether a search row is the room song: song id plus difficulty slot
/// both match the staged pick. The slot codes repeat Easy/Normal/Hard in
/// `Difficulty` code order, so a plain equality on the slot's code names
/// the picked chart.
pub(crate) fn is_picked_row(row: SelectMusicRow<'_>, picked: Option<SetRoomMusic>) -> bool {
    picked.is_some_and(|song| {
        u32::from(song.music_id) == row.song.song_id
            && u8::try_from(row.slot).ok() == Some(song.difficulty)
    })
}

/// One caption at its staged rect origin, scaled into the frame. An
/// unstaged slot inks nothing (the exe draws no glyph for a control the
/// sheet does not carry).
fn paint_slot(frame: &mut Frame<'_>, slot: Option<Rect>, text: &str, ink: u32) {
    let Some(rect) = slot else {
        return;
    };
    let spot = scale_rect_to_frame(&rect, frame.w, frame.h);
    let ink_y = spot.y.saturating_add(SELECT_TEXT_INK_BEARING_Y);
    draw_text(frame, spot.x, ink_y, text, ink);
}

#[cfg(test)]
mod tests {
    use super::super::super::avatar::PlacementRole;
    use super::super::side::{select_music_rows, staged_song, SelectMusicFilter, SelectMusicSong};
    use super::*;

    /// One staged placement at a native rect.
    fn placed(id: &str, code: u32, rect: Rect) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect,
        }
    }

    /// The dialog's `SET` text templates plus two spriteless row bands.
    fn dialog_placements() -> Vec<ScenePlacement> {
        vec![
            placed(
                SEARCH_TAB_TEXT_ID,
                0x2740_0033,
                Rect {
                    x: 356,
                    y: 94,
                    w: 126,
                    h: 15,
                },
            ),
            placed(
                PICKED_TAB_TEXT_ID,
                0x2740_0034,
                Rect {
                    x: 504,
                    y: 94,
                    w: 118,
                    h: 15,
                },
            ),
            placed(
                STATUS_TEXT_ID,
                0x2740_0042,
                Rect {
                    x: 338,
                    y: 390,
                    w: 315,
                    h: 14,
                },
            ),
            placed(
                PAGER_TEXT_ID,
                0x2740_0048,
                Rect {
                    x: 669,
                    y: 390,
                    w: 54,
                    h: 13,
                },
            ),
            placed(
                ROW_TITLE_TEXT_ID,
                0x2740_0012,
                Rect {
                    x: 79,
                    y: 6,
                    w: 149,
                    h: 12,
                },
            ),
            placed(
                ROW_ARTIST_TEXT_ID,
                0x2740_0013,
                Rect {
                    x: 233,
                    y: 6,
                    w: 74,
                    h: 12,
                },
            ),
            placed(
                ROW_LEVEL_TEXT_ID,
                0x2740_0014,
                Rect {
                    x: 309,
                    y: 6,
                    w: 15,
                    h: 12,
                },
            ),
            placed(
                "IDC_BUTTON_DSMUSIC_EXTEND1",
                0x2701_7004,
                Rect {
                    x: 331,
                    y: 150,
                    w: 431,
                    h: 23,
                },
            ),
            placed(
                "IDC_BUTTON_DSMUSIC_EXTEND2",
                0x2701_7005,
                Rect {
                    x: 331,
                    y: 173,
                    w: 431,
                    h: 23,
                },
            ),
        ]
    }

    /// One catalog song (`First`/`Band`).
    fn songs() -> Vec<SelectMusicSong> {
        vec![staged_song(757)]
    }

    /// Any pixel inside `rect` carries `ink`.
    fn has_ink(buf: &[u32], rect: Rect, ink: u32) -> bool {
        let width = 800usize;
        (rect.y..rect.y + rect.h as i32)
            .flat_map(|y| (rect.x..rect.x + rect.w as i32).map(move |x| (x, y)))
            .any(|(x, y)| buf[y as usize * width + x as usize] == ink)
    }

    /// First row with exact caption ink inside a rect.
    fn first_ink_row(buf: &[u32], rect: Rect, ink: u32) -> Option<i32> {
        (rect.y..rect.y + rect.h as i32).find(|&y| {
            (rect.x..rect.x + rect.w as i32).any(|x| buf[y as usize * 800 + x as usize] == ink)
        })
    }

    #[test]
    fn select_row_ink_starts_at_the_gdi_cell_bearing_not_the_template_top() {
        // The 7.55 frame inks row 1 at y 161 while its staged cell top
        // is y 156; the five-pixel GDI bearing is part of the anchor.
        let mut buf = vec![pack_xrgb(0x40, 0x40, 0x40); 800 * 600];
        let slots = SelectTextSlots::from_placements(&dialog_placements());
        let songs = songs();
        let rows = select_music_rows(&songs, SelectMusicFilter::ALL);
        paint_select_text(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &slots,
            &rows,
            None,
            SelectTextLive::default(),
        );
        let title = Rect {
            x: 410,
            y: 156,
            w: 149,
            h: 12,
        };
        assert_eq!(first_ink_row(&buf, title, select_ink()), Some(161));
    }

    #[test]
    fn row_slots_clone_the_set_templates_onto_each_band() {
        // FILE wiring: row text rides the band origin plus the
        // parent-local TEXT template rect (79,6)/(233,6)/(309,6), so
        // row 2 inks its artist at 331+233 on y 173+6.
        let slots = SelectTextSlots::from_placements(&dialog_placements());
        assert_eq!(slots.rows.len(), 2);
        assert_eq!(
            slots.rows[0].title,
            Rect {
                x: 410,
                y: 156,
                w: 149,
                h: 12
            }
        );
        assert_eq!(
            slots.rows[1].artist,
            Rect {
                x: 564,
                y: 179,
                w: 74,
                h: 12
            }
        );
        assert_eq!(
            slots.rows[1].level,
            Rect {
                x: 640,
                y: 179,
                w: 15,
                h: 12
            }
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn unstaged_dialog_names_no_text_slots() {
        // Art-less stage: nothing inks, no invented geometry.
        let slots = SelectTextSlots::from_placements(&[]);
        assert_eq!(slots, SelectTextSlots::default());
        assert!(slots.rows.is_empty());
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn bands_without_set_templates_stage_no_rows() {
        // Row bands alone cannot place text: without the `SET`
        // templates the rows stay empty rather than falling back to a
        // measurement.
        let placements = vec![placed(
            "IDC_BUTTON_DSMUSIC_EXTEND1",
            0x2701_7004,
            Rect {
                x: 331,
                y: 150,
                w: 431,
                h: 23,
            },
        )];
        assert!(SelectTextSlots::from_placements(&placements)
            .rows
            .is_empty());
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn text_layer_inks_at_the_staged_slots_and_draws_no_plate() {
        // The frame starts on the retired scrim tone (0x101018, what the
        // old plates filled): after the layer every slot holds glyph
        // ink, and the slot area keeps that background everywhere the
        // glyphs do not reach — no plate, no fill.
        let retired_scrim = pack_xrgb(0x10, 0x10, 0x18);
        let background = pack_xrgb(0x40, 0x40, 0x40);
        let mut buf = vec![background; 800 * 600];
        let slots = SelectTextSlots::from_placements(&dialog_placements());
        let songs = songs();
        let rows = select_music_rows(&songs, SelectMusicFilter::ALL);
        paint_select_text(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &slots,
            &rows,
            None,
            SelectTextLive::default(),
        );
        let tabs = Rect {
            x: 356,
            y: 94,
            w: 126,
            h: 15,
        };
        assert!(
            has_ink(&buf, tabs, select_active_ink()),
            "search tab count inks"
        );
        assert!(
            has_ink(
                &buf,
                Rect {
                    x: 338,
                    y: 390,
                    w: 315,
                    h: 14
                },
                select_ink()
            ),
            "status inks"
        );
        assert!(
            has_ink(
                &buf,
                Rect {
                    x: 410,
                    y: 156,
                    w: 149,
                    h: 12
                },
                select_ink()
            ),
            "title inks"
        );
        assert!(
            has_ink(
                &buf,
                Rect {
                    x: 564,
                    y: 156,
                    w: 74,
                    h: 12
                },
                select_ink()
            ),
            "artist inks"
        );
        assert!(
            has_ink(
                &buf,
                Rect {
                    x: 640,
                    y: 156,
                    w: 15,
                    h: 12
                },
                select_dim_ink()
            ),
            "the level cell draws the unknown dash"
        );
        // The retired plates filled wholesale: the slot margin keeps the
        // background and no pixel anywhere carries the scrim tone.
        assert_eq!(
            buf[(tabs.y + tabs.h as i32 - 1) as usize * 800 + tabs.x as usize],
            background
        );
        assert!(!buf.contains(&retired_scrim), "no scrim plate is painted");
    }
}
