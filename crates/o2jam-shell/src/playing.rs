//! Note highway: seven lanes, judgement line, falling notes.
//!
//! Pure layout + paint over [`LevelChart`] onsets (`lane, ms` from
//! [`LevelChart::hit_onsets_ms`]); timing comes from an explicit
//! [`PlayingView::advance_to`], so tests drive the clock. Layout is
//! staged, not invented: lane columns, the judgement row, note-head
//! extents, receptor pads, burst slices, and gauge boxes scale from
//! the `STATE_PLAYING` block entries and `State_Playing.bnd` rects at
//! native 800x600 (`docs/bnd-semantics.md`, `docs/bnd-audit.md`), via
//! the shared [`scale_rect_to_frame`] scaler.

use crate::avatar::AvatarFigure;
use o2jam_game::{
    lane_effect::{lane_map_for_modifier_byte, SkillRing},
    CoolStreak, Judgement, LaneMap, LongTail, MeasureReshuffle, PeerBars, PillStock, QueueEvent,
    RankDelivery, Score, TimingWindows, JAM_PROGRESS_PER_LEVEL, LIFE_MAX, PEER_SLOT_COUNT,
};
use o2jam_net::GameSpeed;
use o2jam_render::{
    blit_rgb555, blit_rgb555_clipped, draw_number, draw_text, fill_rect, number_width, pack_xrgb,
    scale_rect_to_frame, text_width, visible_spans, Frame, LaneCurtain, Rect, Rgb555Image,
    NATIVE_HEIGHT,
};

/// Lanes across the highway.
pub const HIGHWAY_LANES: u8 = 7;
/// How far ahead notes appear, in song ms.
pub const DEFAULT_LOOKAHEAD_MS: u64 = 2000;
/// Judgement row on the native canvas: the `KeydownImage.ojs`
/// press-down anchors sit on row 487 (archive slot 33), so due notes
/// land there instead of one sixth up from the frame bottom.
pub const NATIVE_LINE_Y: i32 = 487;
/// Lane columns on the native canvas: `(x, width)` from the
/// `Keyeffect1..7.ojs` beam anchors (archive slots 470-476). The
/// `Keyeffect.ojs` / `KeyEffect_Tutorial_*` anchors and the
/// `Note_Image.ojs` head widths agree at 28/22/28/32/28/22/28.
/// Columns are uneven and hug the left of the canvas; they never
/// tile the full width.
pub const NATIVE_LANES: [(i32, u32); HIGHWAY_LANES as usize] = [
    (5, 28),
    (33, 22),
    (55, 28),
    (83, 32),
    (115, 28),
    (143, 22),
    (165, 28),
];
/// Lane columns as native rects (full canvas height; beams run each
/// column from row 0 toward the judgement row).
pub const NATIVE_LANE_RECTS: [Rect; HIGHWAY_LANES as usize] = [
    Rect {
        x: 5,
        y: 0,
        w: 28,
        h: NATIVE_HEIGHT,
    },
    Rect {
        x: 33,
        y: 0,
        w: 22,
        h: NATIVE_HEIGHT,
    },
    Rect {
        x: 55,
        y: 0,
        w: 28,
        h: NATIVE_HEIGHT,
    },
    Rect {
        x: 83,
        y: 0,
        w: 32,
        h: NATIVE_HEIGHT,
    },
    Rect {
        x: 115,
        y: 0,
        w: 28,
        h: NATIVE_HEIGHT,
    },
    Rect {
        x: 143,
        y: 0,
        w: 22,
        h: NATIVE_HEIGHT,
    },
    Rect {
        x: 165,
        y: 0,
        w: 28,
        h: NATIVE_HEIGHT,
    },
];
/// Note-head height on the native canvas: `Note_Image.ojs` heads run
/// 7 rows tall (archive slot 47).
pub const NATIVE_NOTE_H: u32 = 7;
/// Press-down key heights per lane on the native canvas
/// (`KeydownImage.ojs`, archive slot 33): receptor pads and burst
/// slices scale from these instead of fixed pixel boxes.
pub const NATIVE_KEY_H: [u32; HIGHWAY_LANES as usize] = [46, 42, 46, 46, 46, 42, 46];
/// Jam edge-bars on the native canvas: `Jamgauge_left_bar.ojs`
/// `(23,536)` 206x16 and `Jamgauge_right_bar.ojs` `(570,536)` 206x16
/// (archive slots 464-465) sit below the judgement row.
pub const NATIVE_JAM_BARS: [Rect; 2] = [
    Rect {
        x: 23,
        y: 536,
        w: 206,
        h: 16,
    },
    Rect {
        x: 570,
        y: 536,
        w: 206,
        h: 16,
    },
];
/// Jam-level digit fallback row (frame pixels, not scaled):
/// `Note_JamNum.ojs` anchors `(0,0)` (unbound), so the top-center
/// digits keep their prior row.
pub const JAM_DIGIT_FALLBACK_Y: i32 = 4;
/// Peer life-bar box on the native canvas: each of the eight 4015
/// member gauges follows the `Avatar_Info_LifeGauge.ojs` 46x8 extent
/// (archive slot 6; anchors `(0,0)`, unbound), stacked from the flush
/// top-right corner, clear of the note lane.
pub const NATIVE_PEER_BAR: Rect = Rect {
    x: 754,
    y: 0,
    w: 46,
    h: 8,
};
/// Odd-lane skin tint (even lanes keep the flat Playing backdrop so the
/// staged `Playing1.opi` gauges ring unchanged edges; lifecycle §10).
/// Procedural stand-in for the skinned lane panels (sprite-55 bodies in
/// `Playing1.opi`; exact panel codecs TBD in `docs/file-formats/opi.md`).
pub const LANE_SKIN_ODD: u32 = pack_xrgb(0x0E, 0x0E, 0x16);
/// Note-cap highlight row above each falling note (procedural
/// fallback while no skin is staged; staged heads from
/// [`NOTE_IMAGE_SPRITE`] replace the cap and core outright).
pub const NOTE_CAP: u32 = pack_xrgb(0xB0, 0xE0, 0xFF);
/// Long-note tail beam ink (heads keep the `NOTE_BLUE` core below).
pub const TAIL_BEAM: u32 = pack_xrgb(0x20, 0x80, 0xE0);
/// Judgement-line receptor pad at rest (one pad per lane).
pub const RECEPTOR_DIM: u32 = pack_xrgb(0x2A, 0x2A, 0x38);
/// Judgement-line receptor pad flash on a struck lane.
pub const RECEPTOR_LIT: u32 = pack_xrgb(0xC0, 0xE8, 0xFF);
/// Hit-burst column rising above the line after a strike.
pub const HIT_BURST: u32 = pack_xrgb(0xFF, 0xFF, 0xFF);
/// Hit-burst lifetime in song ms (visual only; timing untouched).
pub const BURST_MS: u64 = 150;
/// Staged `Playing1.opi` note-head body: 7 lanes × 3 frames
/// (widths 28/22/28/32/28/22/28; heads run before the tails).
pub const NOTE_IMAGE_SPRITE: &str = "Note_Image.ojs";
/// Staged press-down key body: one image per lane (native row 487
/// sits at the judgement line).
pub const KEY_DOWN_SPRITE: &str = "KeydownImage.ojs";
/// Staged release-beam body: one full-height column per lane (ink
/// runs from row 80 down to the line).
pub const KEY_EFFECT_SPRITE: &str = "Keyeffect.ojs";
/// Staged long-note release body.
pub const LONG_NOTE_EFFECT_SPRITE: &str = "LongNote_Effect.ojt";
/// Staged receptor stem: `KeyEffect_Tutorial_1.ojs` … `_7.ojs`
/// (five full-height frames per lane, glow in the bottom quarter).
pub const TUTORIAL_EFFECT_STEM: &str = "KeyEffect_Tutorial";
/// Note-head frames per lane in [`NOTE_IMAGE_SPRITE`].
pub const NOTE_FRAMES_PER_LANE: usize = 3;
/// Receptor frames per tutorial body.
pub const TUTORIAL_FRAMES_PER_LANE: usize = 5;
/// Receptor animation step in song ms (visual only; timing untouched).
pub const RECEPTOR_FRAME_MS: u64 = 45;

/// One decoded skin image: raw RGB555 pixels plus the source body
/// colorkey (skipped while blitting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinSprite {
    image: Rgb555Image,
    colorkey: u16,
}

impl SkinSprite {
    /// Wrap decoded pixels with their transparent colour key.
    #[must_use]
    pub fn new(image: Rgb555Image, colorkey: u16) -> Self {
        Self { image, colorkey }
    }

    /// Pixel extent.
    #[must_use]
    pub fn extent(&self) -> (usize, usize) {
        (self.image.w, self.image.h)
    }

    /// Lane-centered left edge (wide art overhangs narrow lanes; the
    /// blit clips to the frame, like the original overlap).
    fn centered_x(&self, geom: &HighwayGeometry, lane: u8) -> i32 {
        let column = geom.lane_rect(lane);
        let lane_w = i32::try_from(column.w).unwrap_or(0);
        let art_w = i32::try_from(self.image.w).unwrap_or(0);
        column.x + (lane_w - art_w) / 2
    }

    /// Paint with its top at `top_y`, centered on the lane.
    fn paint_note_at(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, lane: u8, top_y: i32) {
        blit_rgb555(
            frame,
            &self.image.view(),
            self.centered_x(geom, lane),
            top_y,
            Some(self.colorkey),
        );
    }

    /// Paint with its bottom at `bottom_y` (tall columns anchor their
    /// glow rows at the judgement line; overflow clips to the frame).
    fn paint_bottom_anchored(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        bottom_y: i32,
    ) {
        let height = i32::try_from(self.image.h).unwrap_or(0);
        blit_rgb555(
            frame,
            &self.image.view(),
            self.centered_x(geom, lane),
            bottom_y - height,
            Some(self.colorkey),
        );
    }

    /// Paint bottom-anchored but only inside `clip` (burst columns and
    /// tail spans show their slice of a full-height beam).
    fn paint_clipped_to(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        bottom_y: i32,
        clip: &Rect,
    ) {
        let height = i32::try_from(self.image.h).unwrap_or(0);
        blit_rgb555_clipped(
            frame,
            &self.image.view(),
            self.centered_x(geom, lane),
            bottom_y - height,
            Some(self.colorkey),
            Some(clip),
        );
    }
}

/// Staged highway art: per-lane note heads, press-down keys, release
/// beams, receptor frames, and the long-note effect. Empty while
/// nothing is staged: every paint path falls back to the procedural
/// highway (the `real_art.rs` Playing row keys on the fallback inks,
/// so staged lanes only replace their own art).
#[derive(Debug, Clone, Default)]
pub struct PlaySkin {
    note_heads: [Option<SkinSprite>; HIGHWAY_LANES as usize],
    press_keys: [Option<SkinSprite>; HIGHWAY_LANES as usize],
    release_beams: [Option<SkinSprite>; HIGHWAY_LANES as usize],
    receptors: [Vec<SkinSprite>; HIGHWAY_LANES as usize],
    long_effect: Option<SkinSprite>,
}

impl PlaySkin {
    /// Whether any lane staged art.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.long_effect.is_none()
            && self.note_heads.iter().all(Option::is_none)
            && self.press_keys.iter().all(Option::is_none)
            && self.release_beams.iter().all(Option::is_none)
            && self.receptors.iter().all(Vec::is_empty)
    }

    /// Lanes with a staged note head.
    #[must_use]
    pub fn staged_lanes(&self) -> usize {
        self.note_heads.iter().filter(|head| head.is_some()).count()
    }

    /// Note head for one lane (the first of its triple).
    #[must_use]
    pub fn note_head(&self, lane: u8) -> Option<&SkinSprite> {
        self.note_heads.get(lane as usize)?.as_ref()
    }

    /// Press-down key art for one lane.
    #[must_use]
    pub fn press_key(&self, lane: u8) -> Option<&SkinSprite> {
        self.press_keys.get(lane as usize)?.as_ref()
    }

    /// Release beam for one lane.
    #[must_use]
    pub fn release_beam(&self, lane: u8) -> Option<&SkinSprite> {
        self.release_beams.get(lane as usize)?.as_ref()
    }

    /// Long-note release effect.
    #[must_use]
    pub fn long_effect(&self) -> Option<&SkinSprite> {
        self.long_effect.as_ref()
    }

    /// Receptor frame count for one lane.
    #[must_use]
    pub fn receptor_frame_count(&self, lane: u8) -> usize {
        self.receptors.get(lane as usize).map_or(0, Vec::len)
    }

    /// Receptor frame for one lane: the rest frame while `flash_age_ms`
    /// is `None`, then one step per [`RECEPTOR_FRAME_MS`] (clamped to
    /// the last frame).
    #[must_use]
    pub fn receptor_frame(&self, lane: u8, flash_age_ms: Option<u64>) -> Option<&SkinSprite> {
        let frames = self.receptors.get(lane as usize)?;
        match flash_age_ms {
            None => frames.first(),
            Some(age) => {
                let last = frames.len().saturating_sub(1);
                let step = usize::try_from(age / RECEPTOR_FRAME_MS).unwrap_or(last);
                frames.get(step.min(last))
            }
        }
    }

    /// Stage one lane's note head (out-of-range lanes are ignored).
    #[must_use]
    pub fn with_note_head(mut self, lane: u8, sprite: SkinSprite) -> Self {
        if let Some(slot) = self.note_heads.get_mut(lane as usize) {
            *slot = Some(sprite);
        }
        self
    }

    /// Stage one lane's press-down key art.
    #[must_use]
    pub fn with_press_key(mut self, lane: u8, sprite: SkinSprite) -> Self {
        if let Some(slot) = self.press_keys.get_mut(lane as usize) {
            *slot = Some(sprite);
        }
        self
    }

    /// Stage one lane's release beam.
    #[must_use]
    pub fn with_release_beam(mut self, lane: u8, sprite: SkinSprite) -> Self {
        if let Some(slot) = self.release_beams.get_mut(lane as usize) {
            *slot = Some(sprite);
        }
        self
    }

    /// Stage one lane's receptor frames (rest first, then flash steps).
    #[must_use]
    pub fn with_receptors(mut self, lane: u8, frames: Vec<SkinSprite>) -> Self {
        if let Some(slot) = self.receptors.get_mut(lane as usize) {
            *slot = frames;
        }
        self
    }

    /// Stage the long-note release effect.
    #[must_use]
    pub fn with_long_effect(mut self, sprite: SkinSprite) -> Self {
        self.long_effect = Some(sprite);
        self
    }

    /// Decode every staged body from a read-only archive slice; `None`
    /// when no lane yields art (entries missing or undecodable).
    #[must_use]
    pub fn from_archive(archive: &[u8]) -> Option<Self> {
        let mut skin = Self::default();
        if let Some(body) = archive_body(archive, NOTE_IMAGE_SPRITE) {
            for lane in 0..HIGHWAY_LANES {
                let index = lane as usize * NOTE_FRAMES_PER_LANE;
                if let Some(slot) = skin.note_heads.get_mut(lane as usize) {
                    *slot = lane_sprite(body, index);
                }
            }
        }
        if let Some(body) = archive_body(archive, KEY_DOWN_SPRITE) {
            for lane in 0..HIGHWAY_LANES {
                if let Some(slot) = skin.press_keys.get_mut(lane as usize) {
                    *slot = lane_sprite(body, lane as usize);
                }
            }
        }
        if let Some(body) = archive_body(archive, KEY_EFFECT_SPRITE) {
            for lane in 0..HIGHWAY_LANES {
                if let Some(slot) = skin.release_beams.get_mut(lane as usize) {
                    *slot = lane_sprite(body, lane as usize);
                }
            }
        }
        for lane in 0..HIGHWAY_LANES {
            let frames = archive_body(archive, &tutorial_sprite_name(lane)).map(skin_frames);
            if let Some(frames) = frames.filter(|frames| !frames.is_empty()) {
                if let Some(slot) = skin.receptors.get_mut(lane as usize) {
                    *slot = frames;
                }
            }
        }
        if let Some(body) = archive_body(archive, LONG_NOTE_EFFECT_SPRITE) {
            skin.long_effect = lane_sprite(body, 0);
        }
        (!skin.is_empty()).then_some(skin)
    }
}

/// `KeyEffect_Tutorial_%d.ojs` body name for one lane (1-based).
fn tutorial_sprite_name(lane: u8) -> String {
    format!("{TUTORIAL_EFFECT_STEM}_{}.ojs", lane + 1)
}

/// One archive entry body by stored size (never the reserved stride;
/// the archive stays read-only).
fn archive_body<'archive>(archive: &'archive [u8], entry_name: &str) -> Option<&'archive [u8]> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = entries.iter().find(|entry| entry.name == entry_name)?;
    let size = usize::try_from(entry.size).ok()?;
    let end = entry.data_offset.checked_add(size)?;
    archive.get(entry.data_offset..end)
}

/// One raw-RGB555 image as a skin sprite (`None` for missing images
/// or compressed/outlier frames whose codec is still TBD).
fn lane_sprite(body: &[u8], index: usize) -> Option<SkinSprite> {
    let sprite = o2jam_file::parse_sprite55(body).ok()?;
    let record = sprite.images.get(index)?;
    if !sprite.is_raw_rgb555(record) {
        return None;
    }
    let decoded = Rgb555Image::decode(
        sprite.pixel_data(record),
        record.w as usize,
        record.h as usize,
    )?;
    Some(SkinSprite::new(decoded, sprite.colorkey))
}

/// Every raw image in stored order (tutorial receptor frames).
fn skin_frames(body: &[u8]) -> Vec<SkinSprite> {
    let Ok(sprite) = o2jam_file::parse_sprite55(body) else {
        return Vec::new();
    };
    sprite
        .images
        .iter()
        .filter(|record| sprite.is_raw_rgb555(record))
        .filter_map(|record| {
            Rgb555Image::decode(
                sprite.pixel_data(record),
                record.w as usize,
                record.h as usize,
            )
            .map(|decoded| SkinSprite::new(decoded, sprite.colorkey))
        })
        .collect()
}

/// Scale one native row into the frame (identity on 600).
fn scale_row(native_y: i32, frame_h: usize) -> i32 {
    let scaled = i64::from(native_y) * frame_h as i64 / i64::from(NATIVE_HEIGHT);
    i32::try_from(scaled).unwrap_or(i32::MAX)
}

/// Scale one native extent into the frame (floors; identity on native).
fn scale_extent(native_len: u32, frame_len: usize, native_total: u32) -> u32 {
    u32::try_from(u64::from(native_len) * frame_len as u64 / u64::from(native_total))
        .unwrap_or(u32::MAX)
}

/// Lane layout for one frame size: the staged `STATE_PLAYING`
/// columns and judgement row scaled into the frame (identity on
/// 800x600, proportional on headless test buffers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighwayGeometry {
    /// Scaled lane columns (uneven: 28/22/28/32/28/22/28 at native).
    pub lanes: [Rect; HIGHWAY_LANES as usize],
    /// Scaled judgement row (487 at native).
    pub line_y: i32,
    /// Frame height (notes never draw below the line).
    pub h: i32,
}

impl HighwayGeometry {
    /// Scale the native columns and judgement row into the frame.
    /// (Kept name: the staged columns hug the left, they do not
    /// center; only the scale is frame-relative.)
    #[must_use]
    pub fn centered(frame_w: usize, frame_h: usize) -> Self {
        Self {
            lanes: NATIVE_LANE_RECTS.map(|rect| scale_rect_to_frame(&rect, frame_w, frame_h)),
            line_y: scale_row(NATIVE_LINE_Y, frame_h),
            h: i32::try_from(frame_h).unwrap_or(i32::MAX),
        }
    }

    /// Pixel column of one lane (clamped to lane count).
    #[must_use]
    pub fn lane_rect(&self, lane: u8) -> Rect {
        self.lanes[lane.min(HIGHWAY_LANES - 1) as usize]
    }

    /// Lane-span box above the line: curtain clips and tail spans
    /// read this, so lane gaps and the left offset survive clipping.
    #[must_use]
    pub fn highway_rect(&self) -> Rect {
        let first = self.lanes[0];
        let last = self.lanes[HIGHWAY_LANES as usize - 1];
        Rect {
            x: first.x,
            y: 0,
            w: u32::try_from(last.x + i32::try_from(last.w).unwrap_or(0) - first.x).unwrap_or(0),
            h: u32::try_from(self.line_y.max(0)).unwrap_or(0),
        }
    }

    /// Scaled press-down height of one lane (clamped to lane count).
    fn scaled_key_h(&self, lane: u8) -> i32 {
        let native = NATIVE_KEY_H[lane.min(HIGHWAY_LANES - 1) as usize];
        let frame_h = u32::try_from(self.h.max(0)).unwrap_or(0) as usize;
        i32::try_from(scale_extent(native, frame_h, NATIVE_HEIGHT)).unwrap_or(0)
    }

    /// One falling note box: full lane width, scaled 7-row head
    /// (at least one row, so heads survive tiny probe frames).
    #[must_use]
    pub fn note_rect(&self, lane: u8, top_y: i32) -> Rect {
        let column = self.lane_rect(lane);
        let frame_h = u32::try_from(self.h.max(0)).unwrap_or(0) as usize;
        Rect {
            x: column.x,
            y: top_y,
            w: column.w,
            h: scale_extent(NATIVE_NOTE_H, frame_h, NATIVE_HEIGHT).max(1),
        }
    }

    /// One lane's receptor pad resting on the judgement line: full
    /// lane width, scaled press-down height.
    #[must_use]
    pub fn receptor_pad(&self, lane: u8) -> Rect {
        let column = self.lane_rect(lane);
        let pad_h = self.scaled_key_h(lane);
        Rect {
            x: column.x,
            y: self.line_y - pad_h,
            w: column.w,
            h: u32::try_from(pad_h).unwrap_or(0),
        }
    }

    /// Hit-burst slice: one pad-height block of the lane column
    /// stacked above the receptor pad.
    #[must_use]
    pub fn burst_rect(&self, lane: u8) -> Rect {
        let column = self.lane_rect(lane);
        let pad_h = self.scaled_key_h(lane);
        Rect {
            x: column.x,
            y: self.line_y - 2 * pad_h,
            w: column.w,
            h: u32::try_from(pad_h).unwrap_or(0),
        }
    }
}

/// Scrolling notes for one chart.
#[derive(Debug, Clone, Default)]
pub struct PlayingView {
    /// `(lane, onset ms)`, sorted by onset.
    onsets: Vec<(u8, u64)>,
    /// Current song position in ms.
    now_ms: u64,
    /// Visible window ahead in ms.
    lookahead_ms: u64,
    /// Staged note/receptor art (empty falls back to procedural).
    skin: PlaySkin,
}

impl PlayingView {
    /// Notes to show (empty until a chart loads).
    #[must_use]
    pub fn new(mut onsets: Vec<(u8, u64)>) -> Self {
        onsets.sort_by_key(|(_, at)| *at);
        Self {
            onsets,
            now_ms: 0,
            lookahead_ms: DEFAULT_LOOKAHEAD_MS,
            skin: PlaySkin::default(),
        }
    }

    /// Stage real note/receptor art; clears the procedural fallback
    /// on staged lanes only.
    pub fn set_skin(&mut self, skin: PlaySkin) {
        self.skin = skin;
    }

    /// Drop staged art and restore the full procedural highway.
    pub fn clear_skin(&mut self) {
        self.skin = PlaySkin::default();
    }

    /// Whether any lane staged art.
    #[must_use]
    pub fn has_skin(&self) -> bool {
        !self.skin.is_empty()
    }

    /// Move the song clock (tests and the frame pump call this).
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    /// Remove a struck note so it stops falling; false when absent.
    pub fn strike(&mut self, lane: u8, at_ms: u64) -> bool {
        if let Some(i) = self
            .onsets
            .iter()
            .position(|(hit_lane, hit_at)| *hit_lane == lane && *hit_at == at_ms)
        {
            self.onsets.remove(i);
            true
        } else {
            false
        }
    }

    /// Notes on screen as `(lane, y top)`; `y` runs from the top
    /// (lookahead ahead) to the judgement line (due now).
    #[must_use]
    pub fn visible(&self, geom: &HighwayGeometry) -> Vec<(u8, i32)> {
        let mut notes = Vec::new();
        for (lane, at) in &self.onsets {
            if let Some(y) = y_for_onset(geom, *at, self.now_ms, self.lookahead_ms) {
                notes.push((*lane, y));
            }
        }
        notes
    }

    /// Remap every onset through a skill-ring lane table.
    pub fn remap_lanes(&mut self, map: &LaneMap) {
        for (lane, _) in &mut self.onsets {
            if let Some(destination) = map.destination(*lane) {
                *lane = destination;
            }
        }
        self.onsets.sort_by_key(|(_, at)| *at);
    }

    /// Paint highway, line, and visible notes.
    pub fn paint(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        self.paint_shell(frame, geom);
        for (lane, y) in self.visible(geom) {
            self.paint_note(frame, geom, lane, y);
        }
    }

    /// Paint the highway through a skill-ring curtain: notes outside the
    /// item-text spans (hidden keeps the top 80%, sudden the bottom
    /// 20%, dark the middle 20%, reverse-dark the outer 40% pair —
    /// `o2jam-render::visible_spans`, clip math OPEN) are not drawn.
    /// `Full` paints everything, like [`PlayingView::paint`].
    pub fn paint_curtained(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        curtain: LaneCurtain,
    ) {
        self.paint_shell(frame, geom);
        if curtain == LaneCurtain::Full {
            for (lane, y) in self.visible(geom) {
                self.paint_note(frame, geom, lane, y);
            }
            return;
        }
        let highway = geom.highway_rect();
        let spans = visible_spans(curtain, &highway);
        for (lane, y) in self.visible(geom) {
            if spans
                .iter()
                .any(|span| y >= span.y && y < span.y + span.h as i32)
            {
                self.paint_note(frame, geom, lane, y);
            }
        }
    }

    /// Background, lane skins, receptors, and the judgement line
    /// (no notes). Even lanes keep the flat Playing backdrop; odd lanes
    /// carry the tint above so staged `Playing1.opi` edge gauges keep
    /// ringing a readable highway. The line spans the staged lane
    /// columns, not the full frame.
    fn paint_shell(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        let w = frame.w as i32;
        fill_rect(
            frame,
            &Rect {
                x: 0,
                y: 0,
                w: w as u32,
                h: frame.h as u32,
            },
            pack_xrgb(0x0A, 0x0A, 0x0A),
        );
        for lane in 0..HIGHWAY_LANES {
            let column = geom.lane_rect(lane);
            if lane % 2 == 1 {
                fill_rect(frame, &column, LANE_SKIN_ODD);
            }
            if column.w > 0 {
                fill_rect(
                    frame,
                    &Rect {
                        x: column.x + column.w as i32 - 1,
                        y: 0,
                        w: 1,
                        h: column.h,
                    },
                    pack_xrgb(0x30, 0x30, 0x30),
                );
            }
            self.paint_receptor(frame, geom, lane, RECEPTOR_DIM, None);
        }
        let span = geom.highway_rect();
        fill_rect(
            frame,
            &Rect {
                x: span.x,
                y: geom.line_y,
                w: span.w,
                h: 2,
            },
            pack_xrgb(0xE0, 0xE0, 0xE0),
        );
    }

    /// One lane's receptor pad resting just above the judgement line
    /// (staged tutorial frame when present, procedural pad otherwise).
    fn paint_receptor(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        ink: u32,
        flash_age_ms: Option<u64>,
    ) {
        if let Some(rest) = self.skin.receptor_frame(lane, flash_age_ms) {
            rest.paint_bottom_anchored(frame, geom, lane, geom.line_y);
            return;
        }
        fill_rect(frame, &geom.receptor_pad(lane), ink);
    }

    /// One falling note: the staged head when present, else the
    /// `NOTE_BLUE` core the soak harness (`real_art.rs` Playing row)
    /// and the shell overlay tests key on, plus its cap highlight.
    fn paint_note(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, lane: u8, y: i32) {
        if let Some(head) = self.skin.note_head(lane) {
            head.paint_note_at(frame, geom, lane, y);
            return;
        }
        let core = geom.note_rect(lane, y);
        fill_rect(
            frame,
            &Rect {
                x: core.x,
                y: core.y - 1,
                w: core.w,
                h: 1,
            },
            NOTE_CAP,
        );
        fill_rect(frame, &core, pack_xrgb(0x40, 0xC0, 0xFF));
    }
}

/// Item-text curtain for one skill ring (geometry stays built in
/// `o2jam-render::visible_spans`): the lane rings show everything,
/// each sight ring covers or reveals its fifth of the highway.
fn curtain_for_ring(ring: SkillRing) -> LaneCurtain {
    match ring {
        SkillRing::Hidden => LaneCurtain::Hidden,
        SkillRing::Sudden => LaneCurtain::Sudden,
        SkillRing::Dark => LaneCurtain::Dark,
        SkillRing::ReverseDark => LaneCurtain::ReverseDark,
        SkillRing::Mirror | SkillRing::Random | SkillRing::Panic => LaneCurtain::Full,
    }
}

/// Jam edge-bars scaled into the frame: the staged left/right
/// `Jamgauge` shells below the judgement row.
fn jam_bar_rects(frame_w: usize, frame_h: usize) -> [Rect; 2] {
    NATIVE_JAM_BARS.map(|rect| scale_rect_to_frame(&rect, frame_w, frame_h))
}

/// Peer life-bar boxes: one staged 46x8 gauge per 4015 member slot,
/// stacked from the scaled top-right corner (kept at least one row
/// tall on tiny probes).
fn peer_bar_rects(frame_w: usize, frame_h: usize) -> Vec<Rect> {
    let corner = scale_rect_to_frame(&NATIVE_PEER_BAR, frame_w, frame_h);
    let bar_h = corner.h.max(1);
    (0..PEER_SLOT_COUNT)
        .map(|i| Rect {
            x: corner.x,
            y: corner.y + i32::try_from(i).unwrap_or(0) * bar_h as i32,
            w: corner.w,
            h: bar_h,
        })
        .collect()
}

/// Filled rows for `progress_in_level` (`0..50`) over a `bar_h`-row jam
/// bar. Integer math keeps the exe's cumulative-progress shape
/// (each level spans [`JAM_PROGRESS_PER_LEVEL`]; no reset on grant).
fn jam_fill_rows(progress_in_level: u64, bar_h: u32) -> u32 {
    let filled = progress_in_level
        .saturating_mul(u64::from(bar_h))
        .checked_div(JAM_PROGRESS_PER_LEVEL)
        .unwrap_or(0);
    u32::try_from(filled).unwrap_or(u32::MAX).min(bar_h)
}

/// Row for one onset at `now_ms` (`None` while struck/past or beyond
/// the lookahead). Shared by falling notes and long-note tail beams so
/// both ride the same scroll speed.
fn y_for_onset(geom: &HighwayGeometry, at_ms: u64, now_ms: u64, lookahead_ms: u64) -> Option<i32> {
    if at_ms <= now_ms {
        return None;
    }
    let ahead = at_ms.saturating_sub(now_ms);
    if ahead > lookahead_ms || lookahead_ms == 0 {
        return None;
    }
    let px_per_ms = geom.line_y as f32 / lookahead_ms as f32;
    Some(geom.line_y - (ahead as f32 * px_per_ms) as i32)
}

/// Word shown for each judgement tier.
fn word_of(judgement: Judgement) -> &'static str {
    match judgement {
        Judgement::Cool => "COOL",
        Judgement::Good => "GOOD",
        Judgement::Bad => "BAD",
        Judgement::Miss => "MISS",
    }
}

/// One hittable note: rated once, by key press or by timeout.
/// Long notes carry a tail: after the head is struck the lane is
/// held until the tail, then released exactly there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingNote {
    lane: u8,
    at_ms: u64,
    tail_ms: Option<u64>,
    sample: u32,
    judged: bool,
    holding: bool,
    tail_judged: bool,
}

impl PendingNote {
    /// Tap note (no tail).
    fn tap(lane: u8, at_ms: u64, sample: u32) -> Self {
        Self {
            lane,
            at_ms,
            tail_ms: None,
            sample,
            judged: false,
            holding: false,
            tail_judged: true,
        }
    }

    /// Long note: head at `at_ms`, release at `tail_ms`.
    fn hold(lane: u8, at_ms: u64, tail_ms: u64, sample: u32) -> Self {
        Self {
            lane,
            at_ms,
            tail_ms: Some(tail_ms),
            sample,
            judged: false,
            holding: false,
            tail_judged: false,
        }
    }

    /// Done when the head is judged and the tail (if any) is judged.
    fn done(&self) -> bool {
        self.judged && self.tail_judged
    }
}

/// Live play: scrolling view plus verified judgement/scoring.
///
/// Key presses rate the nearest unjudged note in that lane inside
/// the bad window; notes aging past it rate Miss on advance. Presses
/// with no note nearby rate nothing (the original's empty-press
/// behavior is unverified, so nothing is guessed).
///
/// Skill rings plug in here without touching net/render: remap lanes
/// through a [`LaneMap`], hide notes behind a [`LaneCurtain`], and ask
/// [`LivePlay::should_rebuild_map`] per measure for the panic-style
/// per-measure reshuffle (`MeasureReshuffle`: rebuild every plain
/// measure, once per hold span — `FUN_00574330`).
#[derive(Debug)]
pub struct LivePlay {
    view: PlayingView,
    pending: Vec<PendingNote>,
    score: Score,
    pills: PillStock,
    peers: PeerBars,
    streak: CoolStreak,
    windows: TimingWindows,
    reshuffle: MeasureReshuffle,
    curtain: LaneCurtain,
    now_ms: u64,
    last_word: Option<(Judgement, u64)>,
    sounds: Vec<u32>,
    /// Last strike per lane in song ms (visual hit-burst only; scoring
    /// and timing never read this).
    lane_flash: [Option<u64>; HIGHWAY_LANES as usize],
    /// Display-only avatar scene captions (level, song, volume).
    scene: AvatarScene,
}

impl Default for LivePlay {
    fn default() -> Self {
        Self {
            view: PlayingView::default(),
            pending: Vec::new(),
            score: Score::default(),
            pills: PillStock::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
            scene: AvatarScene::default(),
        }
    }
}

impl LivePlay {
    /// Notes to play (empty until a chart loads); each onset carries
    /// its key-sound sample id.
    #[must_use]
    pub fn new(onsets: &[(u8, u64, u32)]) -> Self {
        let mut pending: Vec<PendingNote> = onsets
            .iter()
            .map(|(lane, at_ms, sample)| PendingNote::tap(*lane, *at_ms, *sample))
            .collect();
        pending.sort_by_key(|note| note.at_ms);
        Self {
            view: PlayingView::new(
                onsets
                    .iter()
                    .map(|(lane, at_ms, _)| (*lane, *at_ms))
                    .collect(),
            ),
            pending,
            score: Score::default(),
            pills: PillStock::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
            scene: AvatarScene::default(),
        }
    }

    /// Notes with long-note tails: `(lane, head ms, tail ms, sample)`.
    /// A tail equal to (or before) its head degrades to a tap.
    #[must_use]
    pub fn new_holds(entries: &[(u8, u64, u64, u32)]) -> Self {
        let mut pending: Vec<PendingNote> = entries
            .iter()
            .map(|(lane, head_ms, tail_ms, sample)| {
                LongTail::new(*head_ms, *tail_ms).map_or_else(
                    || PendingNote::tap(*lane, *head_ms, *sample),
                    |tail| PendingNote::hold(*lane, tail.head_ms, tail.tail_ms, *sample),
                )
            })
            .collect();
        pending.sort_by_key(|note| note.at_ms);
        Self {
            view: PlayingView::new(
                entries
                    .iter()
                    .map(|(lane, head_ms, _, _)| (*lane, *head_ms))
                    .collect(),
            ),
            pending,
            score: Score::default(),
            pills: PillStock::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
            scene: AvatarScene::default(),
        }
    }

    /// Skill-ring curtain worn for this run (default: none).
    pub fn set_curtain(&mut self, curtain: LaneCurtain) {
        self.curtain = curtain;
    }

    /// Display-only avatar scene captions (level, song, volume).
    #[must_use]
    pub fn scene(&self) -> &AvatarScene {
        &self.scene
    }

    /// Caption the avatar scene (display only; scoring and timing
    /// never read this).
    pub fn scene_mut(&mut self) -> &mut AvatarScene {
        &mut self.scene
    }

    /// Paint the right-side avatar scene plus bottom tally strip over
    /// the highway and staged art. Counts feed from the live
    /// [`Score`]; state is untouched (shared `&self` like a query).
    pub fn paint_avatar_scene(
        &self,
        frame: &mut Frame<'_>,
        avatar: Option<&AvatarFigure>,
        now_ms: u64,
    ) {
        self.scene
            .paint(frame, avatar, &self.score, now_ms, self.song_end_ms());
    }

    /// Song end for the TIME bar: last head/tail plus the 1.5 s outro
    /// from [`Self::is_finished`]; `None` while nothing is charted.
    fn song_end_ms(&self) -> Option<u64> {
        let last = self
            .pending
            .iter()
            .map(|note| note.tail_ms.unwrap_or(note.at_ms))
            .max()?;
        Some(last.saturating_add(1500))
    }

    /// Stage real note/key art into the highway paint (procedural
    /// fallback stays on unstaged lanes).
    pub fn set_skin(&mut self, skin: PlaySkin) {
        self.view.set_skin(skin);
    }

    /// Drop staged art and restore the full procedural highway.
    pub fn clear_skin(&mut self) {
        self.view.clear_skin();
    }

    /// Whether any lane staged art.
    #[must_use]
    pub fn has_skin(&self) -> bool {
        self.view.has_skin()
    }

    /// Song setup for one skill ring: remap lanes through the ring's
    /// table, restart the per-measure reshuffle state, and hang the
    /// ring's item-text curtain.
    ///
    /// Panic keeps reshuffling after setup: whenever
    /// [`Self::should_rebuild_map`] says so, build a fresh table with
    /// [`SkillRing::build_lane_map`] and feed it to [`Self::remap_lanes`].
    pub fn apply_skill_ring(&mut self, ring: SkillRing, next_random: &mut impl FnMut() -> u32) {
        let map = ring.build_lane_map(HIGHWAY_LANES as usize, next_random);
        self.reshuffle = MeasureReshuffle::default();
        self.set_curtain(curtain_for_ring(ring));
        self.remap_lanes(&map);
    }

    /// Song setup from the room modifier byte (session byte at VA
    /// `0x7789e4`, mirrored from room field `+0x20d`): the verified
    /// lane path (byte 2 shuffles all lanes, byte 0 the narrow table,
    /// anything else plays straight through) with a fresh reshuffle
    /// state and no curtain — the setup copies static visibility
    /// params for bytes 0 and 2, and the remaining clip math is
    /// untraced, so rings with curtains are worn via
    /// [`Self::apply_skill_ring`] instead.
    pub fn apply_modifier_byte(&mut self, modifier: u8, next_random: &mut impl FnMut() -> u32) {
        let map = lane_map_for_modifier_byte(modifier, HIGHWAY_LANES as usize, next_random);
        self.reshuffle = MeasureReshuffle::default();
        self.set_curtain(LaneCurtain::Full);
        self.remap_lanes(&map);
    }

    /// Remap every pending lane through a skill-ring table
    /// (mirror / shuffle built by the `LaneMap` constructors).
    /// Flashes clear: they are frame-space visuals for struck lanes.
    pub fn remap_lanes(&mut self, map: &LaneMap) {
        for note in &mut self.pending {
            if let Some(destination) = map.destination(note.lane) {
                note.lane = destination;
            }
        }
        self.pending.sort_by_key(|note| note.at_ms);
        self.view.remap_lanes(map);
        self.lane_flash = [None; HIGHWAY_LANES as usize];
    }

    /// Whether the lane table needs a fresh build for the next measure
    /// (panic-style ring: every plain measure, once per hold span).
    pub fn should_rebuild_map(&mut self, measure_has_hold: bool) -> bool {
        self.reshuffle.should_rebuild_map(measure_has_hold)
    }

    /// Peer bars mirrored from S→C `GameStatsUpdate` (4015).
    #[must_use]
    pub fn peers(&self) -> &PeerBars {
        &self.peers
    }

    /// Apply one peer stats write (Life-`0`/Jam-`1` u16 to the slot) and
    /// return its `(0x12, type, slot, 0)` queue forward, if any.
    pub fn apply_peer_stats(&mut self, slot: u8, kind: u16, value: u16) -> Option<QueueEvent> {
        self.peers.apply_stats_update(slot, kind, value)
    }

    /// Apply one 8-byte peer rank broadcast; returns per-position
    /// deliveries under the rank-remap rule.
    pub fn apply_peer_ranks(&mut self, packet: [u8; PEER_SLOT_COUNT]) -> Vec<RankDelivery> {
        self.peers.apply_rank_broadcast(packet)
    }

    /// Move the song clock; notes past the bad window rate Miss.
    /// Heads missed by timeout kill their tails too, and tails with
    /// no release by their window end rate Miss (missing the release
    /// is a miss — long-note rule).
    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.view.advance_to(now_ms);
        let windows = self.windows;
        let mut late = Vec::new();
        for note in &mut self.pending {
            if !note.judged && note.at_ms.saturating_add(windows.bad) < now_ms {
                note.judged = true;
                note.tail_judged = true;
                note.holding = false;
                late.push(Judgement::Miss);
            } else if let Some(tail) = note.tail_ms {
                let hold = LongTail {
                    head_ms: note.at_ms,
                    tail_ms: tail,
                };
                if note.holding && !note.tail_judged && hold.is_expired(now_ms, &windows) {
                    note.tail_judged = true;
                    note.holding = false;
                    late.push(Judgement::Miss);
                }
            }
        }
        for judgement in late {
            self.rate(judgement);
        }
    }

    /// Strike a lane: rate the nearest unjudged note in range.
    /// Returns the judgement, or `None` for an empty press.
    pub fn strike_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        let mut best: Option<(usize, u64)> = None;
        for (i, note) in self.pending.iter().enumerate() {
            if note.judged || note.lane != lane {
                continue;
            }
            let gap = note.at_ms.abs_diff(now_ms);
            if gap <= self.windows.bad && best.is_none_or(|(_, seen)| gap < seen) {
                best = Some((i, gap));
            }
        }
        let (i, _) = best?;
        let offset = now_ms as i64 - self.pending[i].at_ms as i64;
        let (lane, at_ms, sample) = (
            self.pending[i].lane,
            self.pending[i].at_ms,
            self.pending[i].sample,
        );
        self.pending[i].judged = true;
        self.pending[i].holding = self.pending[i].tail_ms.is_some();
        self.view.strike(lane, at_ms);
        self.sounds.push(sample);
        self.flash_lane(lane, now_ms);
        let judgement = self.windows.judge(offset);
        self.rate(judgement);
        Some(judgement)
    }

    /// Release a held lane: rate the tail of the nearest held note.
    /// Releasing mid-hold (before the tail window) is a Miss;
    /// a release inside the window judges with the strike tiers.
    /// Returns the judgement, or `None` when no lane is held.
    pub fn release_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        let mut best: Option<(usize, u64)> = None;
        for (i, note) in self.pending.iter().enumerate() {
            if !note.holding || note.tail_judged || note.lane != lane {
                continue;
            }
            let tail = note.tail_ms.unwrap_or(note.at_ms);
            let gap = tail.abs_diff(now_ms);
            if best.is_none_or(|(_, seen)| gap < seen) {
                best = Some((i, gap));
            }
        }
        let (i, _) = best?;
        let tail = LongTail {
            head_ms: self.pending[i].at_ms,
            tail_ms: self.pending[i].tail_ms.unwrap_or(self.pending[i].at_ms),
        };
        self.pending[i].holding = false;
        self.pending[i].tail_judged = true;
        self.flash_lane(lane, now_ms);
        let judgement = if tail.is_early(now_ms, &self.windows) {
            Judgement::Miss
        } else {
            tail.judge_release(now_ms, &self.windows)
        };
        self.rate(judgement);
        Some(judgement)
    }

    /// Running score and combo.
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Take queued key-sound sample ids (one per struck note).
    pub fn drain_sounds(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.sounds)
    }

    /// Song over: every head and tail judged and the tail (1.5 s) played
    /// out. Empty charts never finish on their own (nothing to play).
    #[must_use]
    pub fn is_finished(&self, now_ms: u64) -> bool {
        let Some(last) = self
            .pending
            .iter()
            .map(|note| note.tail_ms.unwrap_or(note.at_ms))
            .max()
        else {
            return false;
        };
        now_ms >= last.saturating_add(1500) && self.pending.iter().all(|note| note.done())
    }

    fn rate(&mut self, judgement: Judgement) {
        self.streak.note(judgement, &mut self.pills);
        self.score.apply_rated(judgement, &mut self.pills);
        self.last_word = Some((judgement, self.now_ms));
    }

    /// Stamp a visual-only hit flash on one lane (never read by
    /// timing or scoring).
    fn flash_lane(&mut self, lane: u8, now_ms: u64) {
        if let Some(slot) = self.lane_flash.get_mut(lane as usize) {
            *slot = Some(now_ms);
        }
    }

    /// Flash age of one lane at `now_ms` (`None` when unlit).
    fn flash_age(&self, lane: u8, now_ms: u64) -> Option<u64> {
        let struck = self.lane_flash.get(lane as usize).copied().flatten()?;
        let age = now_ms.saturating_sub(struck);
        (age < BURST_MS).then_some(age)
    }

    /// Long-note tail beams between head and tail rows (visual only;
    /// heads and releases still rate through `strike_lane` /
    /// `release_lane`). Held heads beam from the judgement line;
    /// untouched holds beam head-to-tail. Beams obey the worn curtain.
    fn paint_hold_tails(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        for note in &self.pending {
            let Some(tail_ms) = note.tail_ms else {
                continue;
            };
            if note.tail_judged || tail_ms <= self.now_ms {
                continue;
            }
            let Some(top) = y_for_onset(geom, tail_ms, self.now_ms, DEFAULT_LOOKAHEAD_MS) else {
                continue;
            };
            let bottom = if note.holding {
                geom.line_y
            } else {
                let Some(head_y) = y_for_onset(geom, note.at_ms, self.now_ms, DEFAULT_LOOKAHEAD_MS)
                else {
                    continue;
                };
                head_y + i32::try_from(geom.note_rect(note.lane, head_y).h).unwrap_or(0)
            };
            if top >= bottom || !self.tail_shown(frame, geom, top) {
                continue;
            }
            self.paint_tail_span(frame, geom, note.lane, note.holding, top, bottom);
        }
    }

    /// One tail span: the staged long effect on held heads, the
    /// staged release beam on untouched holds, else the procedural
    /// [`TAIL_BEAM`] core.
    fn paint_tail_span(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        holding: bool,
        top: i32,
        bottom: i32,
    ) {
        if holding {
            if let Some(art) = self.view.skin.long_effect() {
                art.paint_bottom_anchored(frame, geom, lane, geom.line_y);
                return;
            }
        } else if let Some(beam) = self.view.skin.release_beam(lane) {
            let column = geom.lane_rect(lane);
            beam.paint_clipped_to(
                frame,
                geom,
                lane,
                bottom,
                &Rect {
                    x: column.x,
                    y: top,
                    w: column.w,
                    h: u32::try_from(bottom - top).unwrap_or(0),
                },
            );
            return;
        }
        let column = geom.lane_rect(lane);
        fill_rect(
            frame,
            &Rect {
                x: column.x + column.w as i32 / 2 - 1,
                y: top,
                w: 2,
                h: u32::try_from(bottom - top).unwrap_or(0),
            },
            TAIL_BEAM,
        );
    }

    /// Whether a tail row survives the worn sight-ring curtain (same
    /// span rule as falling notes).
    fn tail_shown(&self, _frame: &Frame<'_>, geom: &HighwayGeometry, y: i32) -> bool {
        if self.curtain == LaneCurtain::Full {
            return true;
        }
        visible_spans(self.curtain, &geom.highway_rect())
            .iter()
            .any(|span| y >= span.y && y < span.y + span.h as i32)
    }

    /// Lit receptor pads plus burst columns on flashed lanes (staged
    /// `KeyEffect_Tutorial_%d` receptors, `KeydownImage` press art, and
    /// `Keyeffect` beams when present; procedural pad and column
    /// otherwise).
    fn paint_hit_bursts(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        for lane in 0..HIGHWAY_LANES {
            let Some(age) = self.flash_age(lane, now_ms) else {
                continue;
            };
            self.paint_lane_flash(frame, geom, lane, age);
        }
    }

    /// One flashed lane: staged beam, animated receptor, and press art
    /// (each falls back to its procedural ink when unstaged).
    fn paint_lane_flash(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        age_ms: u64,
    ) {
        let burst = geom.burst_rect(lane);
        if let Some(beam) = self.view.skin.release_beam(lane) {
            beam.paint_clipped_to(frame, geom, lane, geom.line_y, &burst);
        } else {
            fill_rect(frame, &geom.burst_rect(lane), HIT_BURST);
        }
        self.view
            .paint_receptor(frame, geom, lane, RECEPTOR_LIT, Some(age_ms));
        if let Some(press) = self.view.skin.press_key(lane) {
            press.paint_bottom_anchored(frame, geom, lane, geom.line_y);
        }
    }

    /// Judgement word above the line with the running combo beneath it
    /// (the original stacks combo under the word; the top-left combo
    /// digits stay as the HUD mirror). Words keep plain white ink.
    fn paint_judgement_word(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        let Some((judgement, at)) = self.last_word else {
            return;
        };
        if now_ms.saturating_sub(at) >= 600 {
            return;
        }
        let word = word_of(judgement);
        let text_w = i32::try_from(text_width(word)).unwrap_or(0);
        draw_text(
            frame,
            frame.w as i32 / 2 - text_w / 2,
            geom.line_y - 40,
            word,
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        if self.score.combo > 0 {
            let digits = i32::try_from(number_width(self.score.combo)).unwrap_or(0);
            draw_number(
                frame,
                frame.w as i32 / 2 - digits / 2,
                geom.line_y - 24,
                self.score.combo,
                pack_xrgb(0xFF, 0xFF, 0x40),
            );
        }
    }

    /// Peer life bars: one staged 46x8 gauge per 4015 member slot,
    /// stacked from the top-right corner, each filled green bottom-up
    /// over a dark-red empty bar.
    fn paint_peer_bars(&self, frame: &mut Frame<'_>) {
        let levels = self.peers.life_levels();
        for (level, bar) in levels.iter().zip(peer_bar_rects(frame.w, frame.h)) {
            Self::paint_peer_bar(frame, &bar, *level);
        }
    }

    /// One empty bar plus its life fill, rising from the bottom.
    fn paint_peer_bar(frame: &mut Frame<'_>, bar: &Rect, life: u16) {
        fill_rect(frame, bar, pack_xrgb(0x30, 0x10, 0x10));
        let filled = u32::from(life.min(LIFE_MAX)) * bar.h / u32::from(LIFE_MAX);
        fill_rect(
            frame,
            &Rect {
                x: bar.x,
                y: bar.y + (bar.h - filled) as i32,
                w: bar.w,
                h: filled,
            },
            pack_xrgb(0x20, 0xE0, 0x20),
        );
    }

    /// Jam edge-bars plus level digits from [`JamState`]: Cool feeds +2,
    /// Good feeds +1, each level spans [`JAM_PROGRESS_PER_LEVEL`] cumulative
    /// progress (`FUN_00570260` / `FUN_00570510`), and a pill-saved Bad
    /// feeds like a Cool (`apply_rated`). The bars fill where the staged
    /// `Jamgauge_right_bar.ojs` / `Jamgauge_left_bar.ojs` shells sit
    /// (VAs `0x64b1b8` / `0x64b1d0`, loader `FUN_004e7850`); the digits
    /// stand in for the `Note_JamNum.ojs` (`0x64c0fc` → `0x50d0ae`) art
    /// until staged figures bind.
    fn paint_jam_gauge(&self, frame: &mut Frame<'_>) {
        for bar in jam_bar_rects(frame.w, frame.h) {
            if bar.h == 0 {
                continue;
            }
            let filled = jam_fill_rows(self.score.jam.level_progress(), bar.h);
            Self::paint_jam_bar(frame, &bar, filled);
        }
        self.paint_jam_level(frame);
    }

    /// One empty jam bar plus its progress fill, rising from the bottom.
    fn paint_jam_bar(frame: &mut Frame<'_>, bar: &Rect, filled: u32) {
        let filled = filled.min(bar.h);
        fill_rect(frame, bar, pack_xrgb(0x10, 0x10, 0x30));
        fill_rect(
            frame,
            &Rect {
                x: bar.x,
                y: bar.y + (bar.h - filled) as i32,
                w: bar.w,
                h: filled,
            },
            pack_xrgb(0xFF, 0x80, 0x00),
        );
    }

    /// Jam level digits, top-centered (digit bodies follow `Note_JamNum`).
    fn paint_jam_level(&self, frame: &mut Frame<'_>) {
        let level = self.score.jam.level();
        let digits = i32::try_from(number_width(level)).unwrap_or(0);
        let center = i32::try_from(frame.w).unwrap_or(0) / 2;
        draw_number(
            frame,
            center - digits / 2,
            JAM_DIGIT_FALLBACK_Y,
            level,
            pack_xrgb(0xFF, 0xC0, 0x40),
        );
    }

    /// Paint highway, notes, tails, peer bars, jam gauge, score, combo,
    /// bursts, and word.
    pub fn paint(&mut self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        self.advance_to(now_ms);
        self.view.paint_curtained(frame, geom, self.curtain);
        self.paint_hold_tails(frame, geom);
        self.paint_hit_bursts(frame, geom, now_ms);
        self.paint_peer_bars(frame);
        self.paint_jam_gauge(frame);
        let points = u32::try_from(self.score.points).unwrap_or(u32::MAX);
        draw_number(frame, 16, 16, points, pack_xrgb(0xFF, 0xFF, 0xFF));
        if self.score.combo > 0 {
            draw_number(frame, 16, 32, self.score.combo, pack_xrgb(0xFF, 0xFF, 0x40));
        }
        self.paint_judgement_word(frame, geom, now_ms);
    }
}

/// Native scene width the avatar layout scales from (800x600 canvas).
const SCENE_NATIVE_W: u32 = 800;
/// Native scene height the avatar layout scales from.
const SCENE_NATIVE_H: u32 = 600;
/// `Lv.<n> <name>` line above the life bar (native origin).
const AVATAR_LEVEL_NATIVE: (i32, i32) = (340, 40);
/// Life-bar shell on the native canvas (above the figure).
const AVATAR_LIFE_NATIVE: Rect = Rect {
    x: 340,
    y: 56,
    w: 200,
    h: 12,
};
/// Avatar figure origin on the native canvas (center-right).
const AVATAR_FIGURE_NATIVE: (i32, i32) = (470, 120);
/// Song info line (`X3.5 <title>`) on the native canvas (lower area).
const AVATAR_SONG_NATIVE: (i32, i32) = (340, 420);
/// `VOL <n>` readout below the song line (native origin).
const AVATAR_VOLUME_NATIVE: (i32, i32) = (340, 444);
/// Counts line on the native canvas (left of the tally strip).
const TALLY_COUNTS_NATIVE: (i32, i32) = (16, 556);
/// TIME progress bar on the native canvas (strip center).
const TALLY_TIME_NATIVE: Rect = Rect {
    x: 300,
    y: 560,
    w: 220,
    h: 10,
};
/// Strip song-title line below the TIME bar (native origin).
const TALLY_TITLE_NATIVE: (i32, i32) = (300, 576);
/// EXIT button readout on the native canvas (strip right).
const TALLY_EXIT_NATIVE: (i32, i32) = (740, 556);
/// Caption ink over staged art (white on the scrim).
const AVATAR_TEXT: u32 = pack_xrgb(0xFF, 0xFF, 0xFF);
/// Scrim ink behind caption lines (matches the lobby scrims).
const AVATAR_SCRIM: u32 = pack_xrgb(0x08, 0x08, 0x10);
/// Life-bar empty shell (mirrors the peer bars).
const AVATAR_LIFE_EMPTY: u32 = pack_xrgb(0x30, 0x10, 0x10);
/// Life-bar fill (mirrors the peer bars).
const AVATAR_LIFE_FULL: u32 = pack_xrgb(0x20, 0xE0, 0x20);
/// TIME-bar empty shell.
const AVATAR_TIME_EMPTY: u32 = pack_xrgb(0x20, 0x20, 0x20);
/// TIME-bar progress fill.
const AVATAR_TIME_FILL: u32 = pack_xrgb(0xE0, 0xE0, 0xE0);

/// Scale one avatar-layout axis into the frame (identity on native).
fn scale_scene_axis(native: i32, frame_len: usize, native_total: u32) -> i32 {
    let frame = i32::try_from(frame_len).unwrap_or(i32::MAX);
    let total = i32::try_from(native_total).unwrap_or(1).max(1);
    let scaled = i64::from(native) * i64::from(frame) / i64::from(total);
    i32::try_from(scaled).unwrap_or(i32::MAX)
}

/// Scale one avatar-layout origin into the frame.
fn scale_scene_origin(native: (i32, i32), frame_w: usize, frame_h: usize) -> (i32, i32) {
    (
        scale_scene_axis(native.0, frame_w, SCENE_NATIVE_W),
        scale_scene_axis(native.1, frame_h, SCENE_NATIVE_H),
    )
}

/// Scale one avatar-layout rect into the frame.
fn scale_scene_rect(native: &Rect, frame_w: usize, frame_h: usize) -> Rect {
    scale_rect_to_frame(native, frame_w, frame_h)
}

/// Scaled avatar-scene layout for one frame size: every origin and
/// bar the [`AvatarScene`] painter reads, so paint and tests share
/// one source. Right-side panel opens at native x 330 with the tally
/// strip at native y 540 (`docs/playing-reference.md`, photo-measured).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AvatarLayout {
    /// `Lv.<n> <name>` text origin.
    pub(crate) level_origin: (i32, i32),
    /// Life-bar shell.
    pub(crate) life_bar: Rect,
    /// Avatar figure top-left.
    pub(crate) figure_origin: (i32, i32),
    /// Song info text origin.
    pub(crate) song_origin: (i32, i32),
    /// VOL text origin.
    pub(crate) volume_origin: (i32, i32),
    /// Counts text origin.
    pub(crate) counts_origin: (i32, i32),
    /// TIME progress shell.
    pub(crate) time_bar: Rect,
    /// Strip title text origin.
    pub(crate) strip_title_origin: (i32, i32),
    /// EXIT text origin.
    pub(crate) exit_origin: (i32, i32),
}

impl AvatarLayout {
    /// Scale the native avatar layout into the frame (identity on
    /// 800x600, proportional on headless probe frames).
    #[must_use]
    pub(crate) fn for_frame(frame_w: usize, frame_h: usize) -> Self {
        Self {
            level_origin: scale_scene_origin(AVATAR_LEVEL_NATIVE, frame_w, frame_h),
            life_bar: scale_scene_rect(&AVATAR_LIFE_NATIVE, frame_w, frame_h),
            figure_origin: scale_scene_origin(AVATAR_FIGURE_NATIVE, frame_w, frame_h),
            song_origin: scale_scene_origin(AVATAR_SONG_NATIVE, frame_w, frame_h),
            volume_origin: scale_scene_origin(AVATAR_VOLUME_NATIVE, frame_w, frame_h),
            counts_origin: scale_scene_origin(TALLY_COUNTS_NATIVE, frame_w, frame_h),
            time_bar: scale_scene_rect(&TALLY_TIME_NATIVE, frame_w, frame_h),
            strip_title_origin: scale_scene_origin(TALLY_TITLE_NATIVE, frame_w, frame_h),
            exit_origin: scale_scene_origin(TALLY_EXIT_NATIVE, frame_w, frame_h),
        }
    }
}

/// Scroll-speed label for one room speed code. Mirrors the waiting
/// `room_speed_label` table in `screens::paint` (private there, and
/// other files are out of touch): `X0.5`..`X6`, `XR`, raw fallback.
fn speed_label_for(speed_code: u8) -> String {
    match GameSpeed::from_code(speed_code) {
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
        None => format!("SPD {speed_code}"),
    }
}

/// Right-side avatar scene plus bottom tally strip display state.
///
/// Painted by [`LivePlay::paint_avatar_scene`] over the highway and
/// staged art: life bar plus `Lv.<n> <name>` with the worn figure,
/// song info line (speed plus title), VOL readout, and the bottom
/// tally strip (counts from [`Score`], TIME bar, song title, EXIT).
/// Missing captions paint nothing (waiting-panel precedent); a
/// missing figure paints art without avatar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AvatarScene {
    player_level: Option<u32>,
    player_name: String,
    song_title: String,
    speed_code: Option<u8>,
    volume: Option<u8>,
}

impl AvatarScene {
    /// Caption `Lv.<n> <name>` above the life bar (display only).
    pub fn set_player(&mut self, level: u32, name: &str) {
        self.player_level = Some(level);
        self.player_name = name.to_owned();
    }

    /// Caption the song info line (`X3.5 <title>`, display only).
    pub fn set_song(&mut self, title: &str, speed_code: u8) {
        self.song_title = title.to_owned();
        self.speed_code = Some(speed_code);
    }

    /// Caption the VOL readout (display only).
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = Some(volume);
    }

    /// `Lv.43 <name>`: level, name, or both; empty while unset.
    fn player_line(&self) -> String {
        match (self.player_level, self.player_name.trim()) {
            (Some(level), "") => format!("Lv.{level}"),
            (Some(level), name) => format!("Lv.{level} {name}"),
            (None, "") => String::new(),
            (None, name) => name.to_owned(),
        }
    }

    /// `X3.5 <title>`: speed, title, or both; empty while unset.
    fn song_line(&self) -> String {
        match (self.speed_code, self.song_title.is_empty()) {
            (Some(code), false) => format!("{} {}", speed_label_for(code), self.song_title),
            (Some(code), true) => speed_label_for(code),
            (None, _) => self.song_title.clone(),
        }
    }

    /// `VOL <n>` readout; `None` while unset.
    fn volume_line(&self) -> Option<String> {
        self.volume.map(|volume| format!("VOL {volume}"))
    }

    /// `COOL <n> GOOD <n> BAD <n> MISS <n>` from the live score.
    fn counts_line(score: &Score) -> String {
        format!(
            "COOL {} GOOD {} BAD {} MISS {}",
            score.counts[0], score.counts[1], score.counts[2], score.counts[3]
        )
    }

    /// Paint the right-side scene plus tally strip: captions, life
    /// bar, figure, and counts. Staged art stays underneath;
    /// a missing figure paints art without avatar.
    pub fn paint(
        &self,
        frame: &mut Frame<'_>,
        avatar: Option<&AvatarFigure>,
        score: &Score,
        now_ms: u64,
        song_end_ms: Option<u64>,
    ) {
        let layout = AvatarLayout::for_frame(frame.w, frame.h);
        Self::paint_caption_line(frame, layout.level_origin, &self.player_line());
        Self::paint_life_bar(frame, &layout.life_bar);
        if let Some(figure) = avatar {
            figure.paint(frame, layout.figure_origin.0, layout.figure_origin.1);
        }
        Self::paint_caption_line(frame, layout.song_origin, &self.song_line());
        if let Some(volume) = self.volume_line() {
            Self::paint_caption_line(frame, layout.volume_origin, &volume);
        }
        Self::paint_tally_strip(frame, &layout, score, now_ms, song_end_ms, &self.song_title);
    }

    /// One caption line: dark scrim plus white ink; empty text paints
    /// nothing (missing captions stay off the art).
    fn paint_caption_line(frame: &mut Frame<'_>, origin: (i32, i32), text: &str) {
        if text.is_empty() {
            return;
        }
        let text_w = i32::try_from(text_width(text)).unwrap_or(i32::MAX);
        let scrim_w = u32::try_from(text_w.saturating_add(8)).unwrap_or(u32::MAX);
        fill_rect(
            frame,
            &Rect {
                x: origin.0 - 4,
                y: origin.1 - 2,
                w: scrim_w,
                h: 9,
            },
            AVATAR_SCRIM,
        );
        draw_text(frame, origin.0, origin.1, text, AVATAR_TEXT);
    }

    /// Life-bar shell plus fill. The local life model is OPEN
    /// (`docs/gameplay.md`: no client-side field pinned), so the bar
    /// shows full until the model lands. Inks mirror the peer bars.
    fn paint_life_bar(frame: &mut Frame<'_>, bar: &Rect) {
        if bar.w == 0 || bar.h == 0 {
            return;
        }
        fill_rect(frame, bar, AVATAR_LIFE_EMPTY);
        fill_rect(frame, bar, AVATAR_LIFE_FULL);
    }

    /// TIME progress shell plus fill from the song clock; shell only
    /// while the song end is unknown (empty chart).
    fn paint_time_bar(frame: &mut Frame<'_>, bar: &Rect, now_ms: u64, song_end_ms: Option<u64>) {
        if bar.w == 0 || bar.h == 0 {
            return;
        }
        fill_rect(frame, bar, AVATAR_TIME_EMPTY);
        let filled = match song_end_ms {
            Some(end) if end > 0 => now_ms
                .saturating_mul(u64::from(bar.w))
                .checked_div(end)
                .unwrap_or(0),
            _ => 0,
        };
        let filled = u32::try_from(filled).unwrap_or(u32::MAX).min(bar.w);
        if filled > 0 {
            fill_rect(
                frame,
                &Rect {
                    x: bar.x,
                    y: bar.y,
                    w: filled,
                    h: bar.h,
                },
                AVATAR_TIME_FILL,
            );
        }
    }

    /// Bottom tally strip: counts line, TIME bar, strip title, EXIT.
    /// Counts always feed from the score (zeros included); the title
    /// paints while captioned; EXIT always marks the button.
    fn paint_tally_strip(
        frame: &mut Frame<'_>,
        layout: &AvatarLayout,
        score: &Score,
        now_ms: u64,
        song_end_ms: Option<u64>,
        strip_title: &str,
    ) {
        Self::paint_caption_line(frame, layout.counts_origin, &Self::counts_line(score));
        Self::paint_time_bar(frame, &layout.time_bar, now_ms, song_end_ms);
        Self::paint_caption_line(frame, layout.strip_title_origin, strip_title);
        Self::paint_caption_line(frame, layout.exit_origin, "EXIT");
    }
}

#[cfg(test)]
mod live_play_tests {
    use super::*;
    use o2jam_game::{STAT_JAM, STAT_LIFE};

    fn geom() -> HighwayGeometry {
        HighwayGeometry::centered(70, 60)
    }

    #[test]
    fn strike_rates_cool_on_exact_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 1);
        assert_eq!(live.score().points, 200);
        assert!(live.view.visible(&geom()).is_empty());
    }

    #[test]
    fn struck_notes_queue_their_samples() {
        let mut live = LivePlay::new(&[(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
        live.strike_lane(2, 1010);
        assert_eq!(live.drain_sounds(), vec![7]);
        assert_eq!(live.drain_sounds(), Vec::<u32>::new());
    }

    #[test]
    fn strike_rates_good_on_early_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 990), Some(Judgement::Good));
        assert_eq!(live.score().points, 100);
    }

    #[test]
    fn strike_rates_bad_on_late_hit() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1020), Some(Judgement::Bad));
        assert_eq!(live.score().combo, 0);
        assert_eq!(live.score().points, 4);
    }

    #[test]
    fn late_notes_rate_miss_on_advance() {
        let mut live = LivePlay::new(&[(1, 1000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.score().counts[3], 1);
        assert_eq!(live.score().points, 0);
    }

    #[test]
    fn empty_press_rates_nothing() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        assert_eq!(live.strike_lane(0, 1000), None);
        assert_eq!(live.score().counts, [0, 0, 0, 0]);
    }

    #[test]
    fn nearest_note_wins_in_lane() {
        let mut live = LivePlay::new(&[(2, 1000, 7), (2, 1100, 9)]);
        assert_eq!(live.strike_lane(2, 1010), Some(Judgement::Good));
        assert_eq!(live.strike_lane(2, 1100), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 2);
    }

    #[test]
    fn paint_shows_score_combo_and_word() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1100,
        );
        // Score "200" top-left, combo "1" below, "COOL" centered.
        assert_eq!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
        assert_eq!(buf[32 * 70 + 17], pack_xrgb(0xFF, 0xFF, 0x40));
        let word_x = 35 - text_width("COOL") as i32 / 2;
        assert_eq!(
            buf[8 * 70 + word_x as usize + 2],
            pack_xrgb(0xFF, 0xFF, 0xFF)
        );
    }

    #[test]
    fn jam_fill_rows_scales_verified_progress_to_height() {
        assert_eq!(jam_fill_rows(0, 24), 0);
        assert_eq!(jam_fill_rows(2, 50), 2);
        assert_eq!(jam_fill_rows(1, 50), 1);
        assert_eq!(jam_fill_rows(50, 24), 24);
        assert_eq!(jam_fill_rows(60, 24), 24);
    }

    #[test]
    fn jam_gauge_empty_shows_shell_without_fill() {
        let mut live = LivePlay::default();
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        let shell = pack_xrgb(0x10, 0x10, 0x30);
        let fill = pack_xrgb(0xFF, 0x80, 0x00);
        // Left `Jamgauge` shell: native `(23,536)` 206x16 scales to
        // x 2..20, row 53 at 70x60.
        assert_eq!(buf[53 * 70 + 2], shell);
        assert!(!buf.contains(&fill));
    }

    #[test]
    fn jam_gauge_cool_and_good_fill_both_edge_bars() {
        let mut live = LivePlay::new(&[(0, 1000, 7), (0, 1100, 7)]);
        live.strike_lane(0, 1000);
        live.strike_lane(0, 1100);
        assert_eq!(live.score().jam.level_progress(), 4);
        // Native frame: 16-row shells fill one row per 50/16 progress.
        let geom = HighwayGeometry::centered(800, 600);
        let mut buf = vec![0u32; 800 * 600];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &geom,
            1100,
        );
        let fill = pack_xrgb(0xFF, 0x80, 0x00);
        let bottom = 536 + 16 - 1;
        assert_eq!(buf[bottom * 800 + 23], fill);
        assert_eq!(buf[bottom * 800 + 570], fill);
        assert_eq!(buf[536 * 800 + 23], pack_xrgb(0x10, 0x10, 0x30));
    }

    #[test]
    fn jam_gauge_grant_advances_level_digits_and_empties_fill() {
        let notes: Vec<(u8, u64, u32)> = (0..25u32)
            .map(|i| (0, 1000 + u64::from(i) * 100, 7))
            .collect();
        let mut live = LivePlay::new(&notes);
        for i in 0..25u32 {
            assert_eq!(
                live.strike_lane(0, 1000 + u64::from(i) * 100),
                Some(Judgement::Cool)
            );
        }
        assert_eq!(
            (live.score().jam.level(), live.score().jam.level_progress()),
            (1, 0)
        );
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            3400,
        );
        let jam_ink = pack_xrgb(0xFF, 0xC0, 0x40);
        // Level 1 glyph top row is 0b010: x 34 rides the flat
        // backdrop (the staged lane span ends at x 16), x 35 inks.
        assert_eq!(buf[4 * 70 + 34], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[4 * 70 + 35], jam_ink);
        assert_eq!(buf[53 * 70 + 2], pack_xrgb(0x10, 0x10, 0x30));
    }

    #[test]
    fn jam_gauge_pill_saved_bad_feeds_progress_like_cool() {
        let notes: Vec<(u8, u64, u32)> = (0..16u32)
            .map(|i| (0, 1000 + u64::from(i) * 100, 7))
            .collect();
        let mut live = LivePlay::new(&notes);
        for i in 0..15u32 {
            assert_eq!(
                live.strike_lane(0, 1000 + u64::from(i) * 100),
                Some(Judgement::Cool)
            );
        }
        assert_eq!(live.score().jam.progress(), 30);
        assert_eq!(live.strike_lane(0, 2500 + 20), Some(Judgement::Bad));
        assert_eq!(live.score().jam.progress(), 32);
        assert_eq!(live.score().combo, 15);
    }

    #[test]
    fn finishes_after_tail_with_all_judged() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        assert!(!live.is_finished(3000));
        live.strike_lane(0, 1000);
        assert!(!live.is_finished(2000));
        assert!(live.is_finished(2500));
    }

    #[test]
    fn empty_chart_never_finishes() {
        let live = LivePlay::default();
        assert!(!live.is_finished(u64::MAX));
    }

    #[test]
    fn hold_rates_head_then_tail_release() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 2000), Some(Judgement::Cool));
        assert_eq!(live.score().combo, 2);
        assert_eq!(live.score().points, 400);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn hold_early_release_is_a_miss() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 1500), Some(Judgement::Miss));
        assert_eq!(live.score().combo, 0);
    }

    #[test]
    fn hold_missing_release_rates_miss_on_advance() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        live.advance_to(2000 + 25 + 1);
        assert_eq!(live.release_lane(2, 2000 + 25 + 1), None);
        assert_eq!(live.score().counts, [1, 0, 0, 1]);
        assert_eq!(live.score().combo, 0);
    }

    #[test]
    fn hold_missing_head_kills_the_tail() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.release_lane(2, 2000), None);
        assert_eq!(live.score().counts[3], 1);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn empty_tail_degrades_to_tap() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 1000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 1000), None);
        assert!(live.is_finished(2500));
    }

    #[test]
    fn lane_remap_moves_pending_and_view() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.remap_lanes(&LaneMap::mirrored(7));
        assert_eq!(live.strike_lane(6, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(0, 1000), None);
    }

    #[test]
    fn lane_remap_keeps_time_order() {
        let mut live = LivePlay::new(&[(0, 1100, 7), (6, 1000, 9)]);
        live.remap_lanes(&LaneMap::mirrored(7));
        assert_eq!(live.strike_lane(0, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(6, 1100), Some(Judgement::Cool));
    }

    #[test]
    fn sudden_curtain_hides_notes_above_the_bottom_fifth() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500), (0, 2900)]);
        view.advance_to(1000);
        assert_eq!(view.visible(&geom).len(), 2);
        let mut full = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut full,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Full,
        );
        let mut sudden = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut sudden,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Sudden,
        );
        let note = pack_xrgb(0x40, 0xC0, 0xFF);
        assert!(full.contains(&note));
        assert!(!sudden.contains(&note));
    }

    #[test]
    fn hidden_curtain_keeps_notes_at_the_top() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 2900)]);
        view.advance_to(1000);
        let mut hidden = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut hidden,
                w: 70,
                h: 60,
            },
            &geom,
            LaneCurtain::Hidden,
        );
        assert!(hidden.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
    }

    #[test]
    fn reshuffle_rebuilds_every_plain_measure() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(true));
        assert!(!live.should_rebuild_map(true));
    }

    fn stub_words(words: Vec<u32>) -> impl FnMut() -> u32 {
        let mut words = words.into_iter();
        move || words.next().expect("stub randomness exhausted")
    }

    #[test]
    fn mirror_ring_flips_lanes_with_no_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let mut never_called = || -> u32 { panic!("mirror draws no randomness") };
        live.apply_skill_ring(SkillRing::Mirror, &mut never_called);
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(6, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn random_ring_shuffles_lanes_with_no_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.apply_skill_ring(
            SkillRing::Random,
            &mut stub_words(vec![3, 1, 4, 8, 5, 9, 6]),
        );
        assert_eq!(live.curtain, LaneCurtain::Full);
        // Fixed draws move source lane 0 to lane 3 (golden order).
        assert_eq!(live.strike_lane(3, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(0, 1000), None);
    }

    #[test]
    fn hidden_ring_keeps_lanes_and_covers_the_bottom() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Hidden, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Hidden);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn sudden_ring_keeps_lanes_and_shows_only_the_bottom() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Sudden, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Sudden);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn dark_ring_keeps_lanes_and_shows_only_the_middle() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Dark, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Dark);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn reverse_dark_ring_keeps_lanes_and_covers_the_middle() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_skill_ring(SkillRing::ReverseDark, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::ReverseDark);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn panic_ring_shuffles_and_rebuilds_each_plain_measure() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.apply_skill_ring(SkillRing::Panic, &mut stub_words(vec![3, 1, 4, 8, 5, 9, 6]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(3, 1000), Some(Judgement::Cool));
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(false));
        assert!(live.should_rebuild_map(true));
        assert!(!live.should_rebuild_map(true));
    }

    #[test]
    fn modifier_byte_two_shuffles_without_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.apply_modifier_byte(2, &mut stub_words(vec![3, 1, 4, 8, 5, 9, 6]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(3, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn modifier_byte_zero_builds_the_narrow_table_without_curtain() {
        let mut live = LivePlay::new(&[(0, 1000, 7), (5, 1100, 9)]);
        live.apply_modifier_byte(0, &mut stub_words(vec![2, 0, 1]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        // Fixed draws move source lane 0 to lane 2; lanes outside
        // the 4 slots stay put.
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.strike_lane(5, 1100), Some(Judgement::Cool));
    }

    #[test]
    fn unknown_modifier_byte_plays_straight_through() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.apply_modifier_byte(9, &mut stub_words(vec![]));
        assert_eq!(live.curtain, LaneCurtain::Full);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
    }

    #[test]
    fn peer_stats_update_forwards_queue_event_and_stores_life() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let event = live.apply_peer_stats(3, STAT_LIFE, 500);
        assert_eq!(event.map(|forward| forward.category), Some(0x12));
        assert_eq!(event.map(|forward| forward.slot), Some(3));
        assert_eq!(live.peers().life(3), Some(500));
        assert_eq!(live.apply_peer_stats(8, STAT_LIFE, 500), None);
        assert_eq!(live.apply_peer_stats(0, 9, 500), None);
        live.apply_peer_stats(0, STAT_JAM, 7);
        assert_eq!(live.peers().jam(0), Some(7));
    }

    #[test]
    fn peer_rank_broadcast_wires_through_global_match() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let deliveries = live.apply_peer_ranks([0; PEER_SLOT_COUNT]);
        assert_eq!(deliveries.len(), PEER_SLOT_COUNT);
        assert!(deliveries.iter().all(|found| found.slot.is_none()));
        assert_eq!(live.peers().rank_row(0), Some(0));
    }

    #[test]
    fn paint_shows_peer_life_fill_over_empty_bar() {
        let full_green = pack_xrgb(0x20, 0xE0, 0x20);
        let empty_red = pack_xrgb(0x30, 0x10, 0x10);
        // Slot 0 gauge: staged 46x8 bar scales to x 65..69, row 0
        // at 70x60 (flush top-right stack).
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let mut bare = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut bare,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        assert_eq!(bare[65], empty_red);
        live.apply_peer_stats(0, STAT_LIFE, LIFE_MAX);
        let mut filled = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut filled,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        assert_eq!(filled[65], full_green);
    }

    #[test]
    fn word_fades_after_timeout() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1700,
        );
        // The faded word leaves the flat backdrop: the word sits past
        // the staged lane span (x 0..16 at 70 wide), which ends clear
        // of the frame-centered word.
        let word_x = 35 - text_width("COOL") as i32 / 2;
        assert_eq!(
            buf[8 * 70 + word_x as usize + 2],
            pack_xrgb(0x0A, 0x0A, 0x0A)
        );
    }

    #[test]
    fn odd_lanes_carry_skin_tint() {
        let mut live = LivePlay::default();
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        assert_eq!(buf[5 * 70], pack_xrgb(0x0A, 0x0A, 0x0A));
        // Lane 3 (odd) spans x 7..8 with its divider on x 8; x 7
        // keeps the skin tint.
        assert_eq!(buf[5 * 70 + 7], LANE_SKIN_ODD);
    }

    #[test]
    fn notes_carry_cap_highlight_over_blue_core() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500)]);
        view.advance_to(1000);
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom,
        );
        let note_y = 48 - (500.0 * 48.0 / 2000.0) as i32;
        assert_eq!(buf[note_y as usize * 70], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[(note_y - 1) as usize * 70], NOTE_CAP);
    }

    #[test]
    fn receptors_rest_above_the_line() {
        let geom = HighwayGeometry::centered(70, 60);
        let view = PlayingView::default();
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom,
        );
        assert_eq!(buf[46 * 70], RECEPTOR_DIM);
        assert_eq!(buf[48 * 70], pack_xrgb(0xE0, 0xE0, 0xE0));
    }

    #[test]
    fn hold_tail_beam_spans_head_to_tail() {
        // Lane 3 sits clear of the edge jam bars and peer strip.
        let mut live = LivePlay::new_holds(&[(3, 1500, 1900, 7)]);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1000,
        );
        assert!(buf.contains(&TAIL_BEAM));
        assert_eq!(buf[30 * 70 + 7], TAIL_BEAM);
    }

    #[test]
    fn strike_lights_receptor_and_burst_then_fades() {
        // Lane 3 sits clear of the edge jam bars and peer strip.
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.strike_lane(3, 1000);
        let mut lit = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut lit,
                w: 70,
                h: 60,
            },
            &geom(),
            1050,
        );
        assert_eq!(lit[46 * 70 + 7], RECEPTOR_LIT);
        assert_eq!(lit[42 * 70 + 7], HIT_BURST);
        let mut faded = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut faded,
                w: 70,
                h: 60,
            },
            &geom(),
            1200,
        );
        assert_eq!(faded[46 * 70 + 7], RECEPTOR_DIM);
        assert_eq!(faded[42 * 70 + 7], LANE_SKIN_ODD);
    }

    #[test]
    fn judgement_word_stacks_combo_beneath() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1100,
        );
        let amber = pack_xrgb(0xFF, 0xFF, 0x40);
        let word_x = 35 - text_width("COOL") as i32 / 2;
        assert_eq!(
            buf[8 * 70 + word_x as usize + 2],
            pack_xrgb(0xFF, 0xFF, 0xFF)
        );
        assert!(
            (24..30).any(|y| (0..70).any(|x| buf[y * 70 + x] == amber)),
            "combo cluster missing under the word"
        );
    }

    #[test]
    fn sudden_curtain_hides_tail_beams() {
        let mut live = LivePlay::new_holds(&[(0, 1500, 2900, 7)]);
        live.set_curtain(LaneCurtain::Sudden);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1000,
        );
        assert!(!buf.contains(&TAIL_BEAM));
    }

    #[test]
    fn remap_clears_lane_flashes() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        live.remap_lanes(&LaneMap::mirrored(7));
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1050,
        );
        // Lane 0 receptor and burst rows ride the staged columns,
        // clear of the edge gauges.
        assert_eq!(buf[46 * 70], RECEPTOR_DIM);
        assert_eq!(buf[42 * 70], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    fn solid_sprite(w: usize, h: usize, word: u16) -> SkinSprite {
        SkinSprite::new(
            Rgb555Image {
                w,
                h,
                words: vec![word; w.saturating_mul(h)],
            },
            0x0000,
        )
    }

    fn solid_ink(word: u16) -> u32 {
        let (r, g, b) = o2jam_render::expand_rgb555(word);
        pack_xrgb(r, g, b)
    }

    #[test]
    fn full_lane_skin_replaces_burst_inks_on_strike() {
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.set_skin(
            PlaySkin::default()
                .with_receptors(3, vec![solid_sprite(8, 6, 0x03E0)])
                .with_press_key(3, solid_sprite(4, 4, 0x7C1F))
                .with_release_beam(3, solid_sprite(8, 64, 0x7FE0)),
        );
        assert!(live.has_skin());
        live.strike_lane(3, 1000);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1050,
        );
        // Burst zone shows the beam, the pad rows the press art (4
        // rows, so the burst peeks above it), and the receptor peeks
        // past its edges.
        assert_eq!(buf[40 * 70 + 7], solid_ink(0x7FE0));
        assert_eq!(buf[46 * 70 + 7], solid_ink(0x7C1F));
        assert!(buf.contains(&solid_ink(0x03E0)));
        assert!(!buf.contains(&RECEPTOR_LIT));
    }

    #[test]
    fn receptor_animates_across_flash_frames() {
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.set_skin(PlaySkin::default().with_receptors(
            3,
            vec![solid_sprite(8, 6, 0x03E0), solid_sprite(8, 6, 0x001F)],
        ));
        live.strike_lane(3, 1000);
        let mut first = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut first,
                w: 70,
                h: 60,
            },
            &geom(),
            1000,
        );
        assert!(first.contains(&solid_ink(0x03E0)));
        assert!(!first.contains(&solid_ink(0x001F)));
        let mut second = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut second,
                w: 70,
                h: 60,
            },
            &geom(),
            1060,
        );
        assert!(second.contains(&solid_ink(0x001F)));
        assert!(!second.contains(&solid_ink(0x03E0)));
    }

    #[test]
    fn skinned_tail_span_shows_beam_art() {
        let mut live = LivePlay::new_holds(&[(3, 1500, 1900, 7)]);
        live.set_skin(PlaySkin::default().with_release_beam(3, solid_sprite(8, 64, 0x7FE0)));
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1000,
        );
        assert!(buf.contains(&solid_ink(0x7FE0)));
        assert!(!buf.contains(&TAIL_BEAM));
    }

    #[test]
    fn held_note_shows_long_effect() {
        let mut live = LivePlay::new_holds(&[(3, 1500, 1900, 7)]);
        live.set_skin(PlaySkin::default().with_long_effect(solid_sprite(8, 8, 0x03FF)));
        live.strike_lane(3, 1500);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            1600,
        );
        assert!(buf.contains(&solid_ink(0x03FF)));
        assert!(!buf.contains(&TAIL_BEAM));
    }

    #[test]
    fn skin_staging_toggles_fallback() {
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        assert!(!live.has_skin());
        live.set_skin(PlaySkin::default().with_note_head(3, solid_sprite(6, 4, 0x7C00)));
        assert!(live.has_skin());
        live.clear_skin();
        assert!(!live.has_skin());
    }

    #[test]
    fn from_archive_binds_all_seven_lanes() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Playing1.opi");
        let Ok(archive) = std::fs::read(&path) else {
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            panic!("Playing1.opi staged no skin art");
        };
        assert_eq!(skin.staged_lanes(), HIGHWAY_LANES as usize);
        let widths = [28, 22, 28, 32, 28, 22, 28];
        for lane in 0..HIGHWAY_LANES {
            let head = skin.note_head(lane).expect("note head per lane");
            assert_eq!(head.extent().0, widths[lane as usize]);
            assert!(skin.press_key(lane).is_some());
            assert!(skin.release_beam(lane).is_some());
            assert_eq!(skin.receptor_frame_count(lane), TUTORIAL_FRAMES_PER_LANE);
        }
        assert!(skin.long_effect().is_some());
    }

    #[test]
    fn from_archive_empty_returns_none() {
        assert!(PlaySkin::from_archive(&[]).is_none());
        assert!(PlaySkin::from_archive(&[0u8; 64]).is_none());
    }
}

#[cfg(test)]
mod highway_tests {
    use super::*;

    fn geom() -> HighwayGeometry {
        HighwayGeometry::centered(70, 60)
    }

    #[test]
    fn lanes_follow_staged_keyeffect_columns() {
        // Native 800x600: beam anchors `Keyeffect1..7.ojs` at
        // x 5/33/55/83/115/143/165, judgement row 487 from
        // `KeydownImage.ojs`.
        let geom = HighwayGeometry::centered(800, 600);
        assert_eq!(geom.lanes, NATIVE_LANE_RECTS);
        assert_eq!(geom.line_y, NATIVE_LINE_Y);
        assert_eq!(geom.line_y, 487);
    }

    #[test]
    fn scaled_columns_keep_uneven_staged_widths() {
        // 70x60 probe: uneven 28/22/28/32/28/22/28 heads survive
        // the shared scaler; the lane span hugs the left.
        let geom = geom();
        let xs: Vec<i32> = (0..HIGHWAY_LANES)
            .map(|lane| geom.lane_rect(lane).x)
            .collect();
        let ws: Vec<u32> = (0..HIGHWAY_LANES)
            .map(|lane| geom.lane_rect(lane).w)
            .collect();
        assert_eq!(xs, vec![0, 2, 4, 7, 10, 12, 14]);
        assert_eq!(ws, vec![2, 1, 2, 2, 2, 1, 2]);
        assert_eq!(geom.line_y, 48);
        assert_eq!(
            geom.highway_rect(),
            Rect {
                x: 0,
                y: 0,
                w: 16,
                h: 48
            }
        );
    }

    #[test]
    fn note_box_matches_staged_head_extent() {
        // Heads fill the full lane width at the scaled 7-row height
        // (kept at least one row on tiny probes).
        let geom = geom();
        assert_eq!(
            geom.note_rect(3, 36),
            Rect {
                x: 7,
                y: 36,
                w: 2,
                h: 1
            }
        );
        let native = HighwayGeometry::centered(800, 600);
        assert_eq!(
            native.note_rect(3, 100),
            Rect {
                x: 83,
                y: 100,
                w: 32,
                h: 7
            }
        );
    }

    #[test]
    fn receptor_pad_matches_press_down_height() {
        // Lane 3 press art `KeydownImage.ojs` anchors `(83,487)`
        // 32x46: the pad is that box resting on the line.
        let native = HighwayGeometry::centered(800, 600);
        assert_eq!(
            native.receptor_pad(3),
            Rect {
                x: 83,
                y: 487 - 46,
                w: 32,
                h: 46
            }
        );
        assert_eq!(
            geom().burst_rect(3),
            Rect {
                x: 7,
                y: 48 - 8,
                w: 2,
                h: 4
            }
        );
    }

    #[test]
    fn jam_and_peer_boxes_match_staged_shells() {
        assert_eq!(jam_bar_rects(800, 600), NATIVE_JAM_BARS);
        // Peer bars follow the staged 46x8 life gauge, stacked flush
        // top-right: slot `i` sits `i` bars below the corner.
        let native: Vec<Rect> = (0..PEER_SLOT_COUNT)
            .map(|i| Rect {
                x: 754,
                y: i as i32 * 8,
                w: 46,
                h: 8,
            })
            .collect();
        assert_eq!(peer_bar_rects(800, 600), native);
        assert_eq!(
            peer_bar_rects(70, 60)[0],
            Rect {
                x: 65,
                y: 0,
                w: 4,
                h: 1
            }
        );
        assert_eq!(
            jam_bar_rects(70, 60)[0],
            Rect {
                x: 2,
                y: 53,
                w: 18,
                h: 1
            }
        );
        assert_eq!(
            jam_bar_rects(70, 60)[1],
            Rect {
                x: 49,
                y: 53,
                w: 18,
                h: 1
            }
        );
    }

    #[test]
    fn due_note_sits_on_line_and_future_note_tops_out() {
        let mut view = PlayingView::new(vec![(0, 1000), (3, 3000)]);
        let geom = geom();
        view.advance_to(1000);
        // Onset == now is due (struck), not falling.
        assert_eq!(view.visible(&geom), vec![(3, 0)]);
        view.advance_to(2000);
        assert_eq!(view.visible(&geom), vec![(3, 24)]);
    }

    #[test]
    fn old_and_distant_notes_stay_hidden() {
        let mut view = PlayingView::new(vec![(1, 500), (2, 9999)]);
        view.advance_to(1000);
        assert!(view.visible(&geom()).is_empty());
    }

    #[test]
    fn paint_draws_line_and_note_pixels() {
        let geom = HighwayGeometry::centered(70, 60);
        let mut view = PlayingView::new(vec![(0, 1500)]);
        view.advance_to(1000);
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom,
        );
        // Judgement line spans the staged lanes; the note sits above it.
        assert_eq!(buf[48 * 70], pack_xrgb(0xE0, 0xE0, 0xE0));
        assert_eq!(buf[48 * 70 + 69], pack_xrgb(0x0A, 0x0A, 0x0A));
        let note_y = 48 - (500.0 * 48.0 / 2000.0) as i32;
        assert_eq!(buf[note_y as usize * 70], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn empty_view_paints_highway_without_notes() {
        let geom = HighwayGeometry::centered(21, 12);
        let view = PlayingView::default();
        assert!(view.visible(&geom).is_empty());
        let mut buf = vec![0u32; 21 * 12];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 21,
                h: 12,
            },
            &geom,
        );
        assert_eq!(buf[9 * 21], pack_xrgb(0xE0, 0xE0, 0xE0));
    }

    fn solid_sprite(w: usize, h: usize, word: u16) -> SkinSprite {
        SkinSprite::new(
            Rgb555Image {
                w,
                h,
                words: vec![word; w.saturating_mul(h)],
            },
            0x0000,
        )
    }

    fn solid_ink(word: u16) -> u32 {
        let (r, g, b) = o2jam_render::expand_rgb555(word);
        pack_xrgb(r, g, b)
    }

    #[test]
    fn skinned_note_head_replaces_blue_core() {
        let mut view = PlayingView::new(vec![(0, 1500)]);
        view.advance_to(1000);
        view.set_skin(PlaySkin::default().with_note_head(0, solid_sprite(6, 4, 0x7C00)));
        assert!(view.has_skin());
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
        );
        assert!(buf.contains(&solid_ink(0x7C00)));
        assert!(!buf.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
    }

    #[test]
    fn receptor_rest_shows_tutorial_frame() {
        let mut view = PlayingView::default();
        view.set_skin(PlaySkin::default().with_receptors(0, vec![solid_sprite(6, 6, 0x03E0)]));
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
        );
        assert!(buf.contains(&solid_ink(0x03E0)));
        assert!(buf.contains(&RECEPTOR_DIM));
    }

    #[test]
    fn sudden_curtain_hides_skinned_notes() {
        let mut view = PlayingView::new(vec![(0, 2900)]);
        view.advance_to(1000);
        view.set_skin(PlaySkin::default().with_note_head(0, solid_sprite(6, 4, 0x7C00)));
        let mut buf = vec![0u32; 70 * 60];
        view.paint_curtained(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            LaneCurtain::Sudden,
        );
        assert!(!buf.contains(&solid_ink(0x7C00)));
    }

    #[test]
    fn clear_skin_restores_blue_core() {
        let mut view = PlayingView::new(vec![(0, 1500)]);
        view.advance_to(1000);
        view.set_skin(PlaySkin::default().with_note_head(0, solid_sprite(6, 4, 0x7C00)));
        view.clear_skin();
        assert!(!view.has_skin());
        let mut buf = vec![0u32; 70 * 60];
        view.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
        );
        assert!(buf.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
        assert!(!buf.contains(&solid_ink(0x7C00)));
    }
}

#[cfg(test)]
mod avatar_scene_tests {
    use super::*;
    use crate::avatar::FigureLayer;

    const WHITE: u32 = pack_xrgb(0xFF, 0xFF, 0xFF);
    const LIFE_FILL: u32 = pack_xrgb(0x20, 0xE0, 0x20);
    const TIME_FILL: u32 = pack_xrgb(0xE0, 0xE0, 0xE0);
    const TIME_EMPTY: u32 = pack_xrgb(0x20, 0x20, 0x20);

    fn captioned_scene() -> AvatarScene {
        let mut scene = AvatarScene::default();
        scene.set_player(43, "HOST");
        scene.set_song("First", 6);
        scene.set_volume(5);
        scene
    }

    /// True while white text ink lands near a caption origin (glyph
    /// top-left cells are often blank, so the origin pixel itself may
    /// show the scrim; the scrim box spans one row above through eight
    /// below the origin).
    fn white_near(buf: &[u32], w: usize, h: usize, origin: (i32, i32)) -> bool {
        let (ox, oy) = origin;
        for row in oy..oy + 9 {
            for col in ox..ox + 64 {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < h && c < w && buf[r * w + c] == WHITE {
                    return true;
                }
            }
        }
        false
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

    fn paint_scene(
        scene: &AvatarScene,
        avatar: Option<&AvatarFigure>,
        w: usize,
        h: usize,
    ) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        scene.paint(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            avatar,
            &Score::default(),
            0,
            Some(3000),
        );
        buf
    }

    #[test]
    fn player_line_formats_level_and_name() {
        assert_eq!(captioned_scene().player_line(), "Lv.43 HOST");
        let mut level_only = AvatarScene::default();
        level_only.set_player(7, "");
        assert_eq!(level_only.player_line(), "Lv.7");
        let name_only = AvatarScene {
            player_name: "HOST".to_owned(),
            ..Default::default()
        };
        assert_eq!(name_only.player_line(), "HOST");
        assert_eq!(AvatarScene::default().player_line(), "");
    }

    #[test]
    fn song_line_shows_speed_and_title() {
        assert_eq!(captioned_scene().song_line(), "X3.5 First");
        let speed_only = AvatarScene {
            speed_code: Some(6),
            ..Default::default()
        };
        assert_eq!(speed_only.song_line(), "X3.5");
        let title_only = AvatarScene {
            song_title: "First".to_owned(),
            ..Default::default()
        };
        assert_eq!(title_only.song_line(), "First");
        assert_eq!(AvatarScene::default().song_line(), "");
    }

    #[test]
    fn volume_line_formats_readout() {
        assert_eq!(captioned_scene().volume_line(), Some("VOL 5".to_owned()));
        assert_eq!(AvatarScene::default().volume_line(), None);
    }

    #[test]
    fn speed_label_covers_known_and_unknown_codes() {
        assert_eq!(speed_label_for(6), "X3.5");
        assert_eq!(speed_label_for(0), "X0.5");
        assert_eq!(speed_label_for(200), "SPD 200");
    }

    #[test]
    fn counts_line_feeds_all_four_tiers() {
        let score = Score {
            counts: [1, 2, 3, 4],
            ..Default::default()
        };
        assert_eq!(
            AvatarScene::counts_line(&score),
            "COOL 1 GOOD 2 BAD 3 MISS 4"
        );
        assert_eq!(
            AvatarScene::counts_line(&Score::default()),
            "COOL 0 GOOD 0 BAD 0 MISS 0"
        );
    }

    #[test]
    fn layout_scales_proportionally() {
        let native = AvatarLayout::for_frame(800, 600);
        assert_eq!(native.level_origin, AVATAR_LEVEL_NATIVE);
        assert_eq!(native.life_bar, AVATAR_LIFE_NATIVE);
        assert_eq!(native.figure_origin, AVATAR_FIGURE_NATIVE);
        let probe = AvatarLayout::for_frame(70, 60);
        assert_eq!(probe.level_origin, (29, 4));
    }

    #[test]
    fn level_line_paints_over_scrim() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), None, 800, 600);
        assert!(white_near(&buf, 800, 600, layout.level_origin));
        assert!(buf.contains(&AVATAR_SCRIM));
    }

    #[test]
    fn missing_captions_paint_nothing() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&AvatarScene::default(), None, 800, 600);
        let (x, y) = layout.level_origin;
        assert_eq!(buf[y as usize * 800 + x as usize], 0);
        assert!(!white_near(&buf, 800, 600, layout.level_origin));
        let (x, y) = layout.song_origin;
        assert_eq!(buf[y as usize * 800 + x as usize], 0);
        let (x, y) = layout.volume_origin;
        assert_eq!(buf[y as usize * 800 + x as usize], 0);
    }

    #[test]
    fn life_bar_paints_full_fill() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&AvatarScene::default(), None, 800, 600);
        assert_eq!(buf[56 * 800 + 340], LIFE_FILL);
        assert_eq!(layout.life_bar, AVATAR_LIFE_NATIVE);
    }

    #[test]
    fn figure_paints_at_layout_origin() {
        let figure = red_figure();
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), Some(&figure), 800, 600);
        let (x, y) = layout.figure_origin;
        assert_eq!(buf[y as usize * 800 + x as usize], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn missing_figure_paints_art_without_avatar() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), None, 800, 600);
        let (x, y) = layout.figure_origin;
        assert_eq!(buf[y as usize * 800 + x as usize], 0);
        let (x, y) = layout.level_origin;
        assert_eq!(buf[y as usize * 800 + x as usize], WHITE);
    }

    #[test]
    fn song_and_volume_lines_paint() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), None, 800, 600);
        assert!(white_near(&buf, 800, 600, layout.song_origin));
        assert!(white_near(&buf, 800, 600, layout.volume_origin));
    }

    #[test]
    fn tally_strip_shows_counts_title_time_and_exit() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), None, 800, 600);
        assert!(white_near(&buf, 800, 600, layout.counts_origin));
        assert!(white_near(&buf, 800, 600, layout.strip_title_origin));
        assert!(white_near(&buf, 800, 600, layout.exit_origin));
        assert_eq!(buf[560 * 800 + 300], TIME_EMPTY);
    }

    #[test]
    fn time_bar_fills_with_song_progress() {
        let scene = AvatarScene::default();
        let layout = AvatarLayout::for_frame(800, 600);
        let mut start = vec![0u32; 800 * 600];
        scene.paint(
            &mut Frame {
                buf: &mut start,
                w: 800,
                h: 600,
            },
            None,
            &Score::default(),
            0,
            Some(3000),
        );
        assert_eq!(start[560 * 800 + 300], TIME_EMPTY);
        let mut end = vec![0u32; 800 * 600];
        scene.paint(
            &mut Frame {
                buf: &mut end,
                w: 800,
                h: 600,
            },
            None,
            &Score::default(),
            3000,
            Some(3000),
        );
        assert_eq!(end[560 * 800 + 300], TIME_FILL);
        let mut unknown = vec![0u32; 800 * 600];
        scene.paint(
            &mut Frame {
                buf: &mut unknown,
                w: 800,
                h: 600,
            },
            None,
            &Score::default(),
            3000,
            None,
        );
        assert_eq!(unknown[560 * 800 + 300], TIME_EMPTY);
        let _ = layout;
    }

    #[test]
    fn live_play_routes_score_and_clock_to_scene() {
        let mut live = LivePlay::new(&[(2, 1000, 7)]);
        live.scene_mut().set_player(43, "HOST");
        live.strike_lane(2, 1000);
        assert_eq!(live.scene().player_line(), "Lv.43 HOST");
        assert_eq!(live.song_end_ms(), Some(2500));
        let mut buf = vec![0u32; 800 * 600];
        live.paint_avatar_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            1000,
        );
        let layout = AvatarLayout::for_frame(800, 600);
        assert!(white_near(&buf, 800, 600, layout.counts_origin));
        assert_eq!(LivePlay::default().song_end_ms(), None);
    }
}
