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

use crate::avatar::{strided_sprite55, AvatarFigure, FigureLayer, ScenePlacement};
use o2jam_game::{
    lane_effect::{lane_map_for_modifier_byte, SkillRing},
    CoolStreak, HoldSpan, HoldState, Judgement, LaneMap, LifeBar, LongNoteBonus, MeasureReshuffle,
    PeerBars, PillStock, QueueEvent, RankDelivery, Score, TimingWindows, JAM_PROGRESS_PER_LEVEL,
    LIFE_MAX, PEER_SLOT_COUNT,
};
use o2jam_net::GameSpeed;
use o2jam_render::{
    blit_rgb555, blit_rgb555_clipped, draw_number, draw_text, fill_rect, number_width, pack_xrgb,
    panel_content_rect, scale_rect_to_frame, text_width, visible_spans, Frame, LaneCurtain, Rect,
    Rgb555Image, NATIVE_HEIGHT,
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
/// Jam edge bars on the native canvas: the staged
/// `Jamgauge_left_bar.ojs` `(23,536)` 206x16 and
/// `Jamgauge_right_bar.ojs` `(570,536)` 206x16 (archive slots 464-465)
/// below the judgement row.
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
/// Panel-bottom jam bar on the native canvas: `Jam_gauge_bar.ojs`
/// `(4,536)` 191x12 (archive slot 31, labeled by `Jam_gauge_text.ojs`
/// `(85,539)` 29x7) spans the highway panel under the judgement row —
/// the photo's "JAM bar at the panel bottom"
/// (`docs/playing-reference.md`). Where it overlaps an edge bar it
/// paints last (topmost-last, A5).
pub const NATIVE_JAM_BOTTOM_BAR: Rect = Rect {
    x: 4,
    y: 536,
    w: 191,
    h: 12,
};
/// Gauge fills paint inside their shell, one pixel in from each side:
/// the staged life-gauge pair pins the family rule byte-exact
/// (`Life_gauge_back.ojs` `(202,246)` 13x303 shell vs
/// `Life_gauge_bar.ojs` fill `(203,247)` 11x301), so jam fills
/// occupy the shell interior instead of the full shell extent.
const GAUGE_FILL_INSET: u32 = 1;
/// Highway-panel strip content row on the native canvas:
/// `Playing_Exit.ojs` anchors `(751,554)` and `Normal_TimeBar.ojs`
/// `(489,556)` pin the strip band, and `Playing_Menu.ojs` `(204,558)`
/// 596x42 is the strip panel. Score digits center over the bottom
/// jam bar, combo digits take its left edge — the photo's panel
/// strip: MAX COMBO left, score center (`docs/playing-reference.md`).
const STRIP_ROW_NATIVE: i32 = 554;
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
/// Procedural hit-burst column rising above the line after a strike
/// (the stand-in while neither the staged [`HIT_BURST_SPRITE`]
/// explosion nor a staged [`LANE_BEAM_STEM`] / `Keyeffect.ojs` beam
/// column is present).
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
/// runs from row 80 down to the line). Kept as the untouched-hold
/// tail slice only: the exe never names this body (the control list
/// stages the dark `Keyeffect1..7.ojs` bars), and the bright
/// press/held lane beams stage from [`LANE_BEAM_STEM`].
pub const KEY_EFFECT_SPRITE: &str = "Keyeffect.ojs";
/// Staged press/held lane-beam kit: `KeyEffect_7Key1_1.ojs` …
/// `_7.ojs`. The playing scene stages it through the
/// `KeyEffect_7Key%d_%d.ojs` loader format string (file `0x24abf8`),
/// which pairs `Playing_NormalBG%d.ojs` (`0x24ac44`) — the BG-skin
/// kits 1..3 in the archive (plus Black / P2P / Left-Right variants).
/// Kit 1 pins the deterministic default, like [`HIT_BURST_SPRITE`].
/// Each body is five raw full-column frames whose brightest inks are
/// the lane colors — red 255,33,41 (lanes 1/3/5/7), sky-blue
/// 132,181,255 (lanes 2/6, the reference photo's cyan beam), amber
/// 255,214,115 (lane 4) — concentrated at rows 470..479, the glow
/// band over the judgement line (`docs/ui-draw.md`, beam color
/// source).
pub const LANE_BEAM_STEM: &str = "KeyEffect_7Key1";
/// Lane-beam frames per kit body ([`LANE_BEAM_STEM`]).
pub const LANE_BEAM_FRAMES_PER_LANE: usize = 5;
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
/// Staged hit-burst body: the playing screen's explosion art. The
/// `STATE_PLAYING` control list carries `IDC_IMAGE_SPLAY_NOTECLICK1..7`
/// entries (`0x1B500093`–`0x1B500099`, Korean comment "폭발이미지" =
/// explosion image) naming `_Note_Click.ojt`; the playing applier
/// resolves that stem through `%s%d_%d.ojt` (`0x64b154`, site
/// `0x4e814e`, `docs/stage-bga.md`) to `Note_Click<stage>_<variant>.ojt`
/// — 36 stages × 2 variants staged in `Playing1.opi`, each a square
/// multi-frame explosion (stage 1 variant 1: eight raw 128x128 frames,
/// all anchors `(0,0)`, hundreds of distinct colors). The exe picks
/// the variant with `rand()%2+1`; this pins the deterministic stage 1
/// variant 1 body.
pub const HIT_BURST_SPRITE: &str = "Note_Click1_1.ojt";
/// Staged tap-effect `.oja`: the bare-arm body of the `_Note_Click`
/// runtime ref (applier `0x4e8212`, `docs/stage-bga.md`; the in-song
/// arm keeps the stage-numbered [`HIT_BURST_SPRITE`] `.ojt` as the
/// fallback body). Six shrinking ring frames 144x143..39x39 whose
/// negative centred anchors mark frame-centre draw points — the
/// staged burst already paints from the receptor-pad centre.
pub const HIT_BURST_OJA_SPRITE: &str = "Note_Click.oja";
/// Staged jam-effect `.oja`: the bare-arm body of
/// `_Playing_Effect_Jam` (same applier arm, `docs/stage-bga.md`).
/// Ten 300x300 flash frames at anchor `(0, 0)`; the exe draw point
/// is untraced, so the staged flash centers on the highway at the
/// judgement line.
pub const JAM_EFFECT_SPRITE: &str = "Playing_Effect_Jam.oja";
/// Staged hold-ring `.oja`: the literal exe stem `LongNote_Effect.oja`
/// (claims C7, file `0x24b26c`) — byte-identical to
/// [`HIT_BURST_OJA_SPRITE`], so the same six ring frames animate the
/// held span (the static [`LONG_NOTE_EFFECT_SPRITE`] frame 0 stays
/// the fallback body).
pub const HOLD_RING_SPRITE: &str = "LongNote_Effect.oja";
/// Staged score-digit body: `Scroe.ojs` (archive spelling), ten raw
/// ~11x21 frames in digit order 0..=9 (verified on 7.55
/// `Playing1.opi`, archive slot 557; every frame anchors at (0,0)).
pub const SCORE_FONT_SPRITE: &str = "Scroe.ojs";
/// Staged combo-digit body: `Note_ComboNum.ojs`, ten raw frames up
/// to 33x51 in digit order 0..=9 (archive slot 44).
pub const COMBO_FONT_SPRITE: &str = "Note_ComboNum.ojs";
/// Staged combo-milestone glow body: `Note_Combo4.ojt`, the lowest
/// strided glow atlas in `Playing1.opi` — stages 4/5/6 store four
/// 64x64 glow frames per record (196,736 B exact; the other 33 stage
/// bodies are single-frame). The exe applier loads the room stage's
/// own body through `_Note_Combo.ojt` (`Note_Combo<stage>.ojt`,
/// `docs/stage-bga.md`); the stage-less skin pins this deterministic
/// atlas like [`HIT_BURST_SPRITE`], and
/// [`PlaySkin::from_archive_for_stage`] routes the room stage.
pub const COMBO_GLOW_SPRITE: &str = "Note_Combo4.ojt";
/// Combo-digit row above the judgement line: the big photo digits
/// (`docs/playing-reference.md`) and their glow band center here.
/// OPEN (geometry-audit 31): the combo and long-combo digit controls
/// (`IDC_IMAGE_SPLAY_NOTECOMBONUM1..4` / `…_LNCOMBONUM1..4`, codes
/// `0x1B500088`–`0x1B500091`) are ControlList entries 136–145 — past
/// the 17 `State_Playing.bnd` rects, so no parsed `.bnd` rect pairs
/// them (all-zero rect) — and every digit frame anchors `(0,0)`, so
/// the exe draw point is code-driven. `Scroe.ojs` / `PlayTimeNum.ojs`
/// are named by no entry at all (code-loaded through the playing
/// applier). This offset is photo-placed, not file geometry.
/// Named next step: a `rea` static trace of the digit-frame setters
/// for codes `0x1B500088`–`0x1B500092` to recover the code constants
/// before re-anchoring — never a literal.
const COMBO_ROW_ABOVE_LINE: i32 = 24;
/// Staged play-time-digit body: `PlayTimeNum.ojs`, ten raw 27x21
/// frames in digit order 0..=9 (archive slot 96).
pub const PLAY_TIME_FONT_SPRITE: &str = "PlayTimeNum.ojs";
/// Staged long-note combo caption plate: `Note_LongCombo.ojt`, two raw
/// 128x128 frames (archive slot 432; the runtime body — the Playing
/// applier resolves `_Note_LongCombo.ojt` through the `%s.ojt`
/// special stem, site `0x4e819e`, `docs/stage-bga.md`). Block entries
/// 129–134 name it and the `_Note_LongComboNum.ojt` digits; their
/// codes (`0x1B50008C`..`0x1B500091`) pair rect ordinals 139–144, all
/// past the 17 `State_Playing.bnd` rects, so the caption has no
/// `.bnd` rect and rides the photo-pinned box
/// [`NATIVE_LONG_COMBO_PLATE`] instead.
pub const LONG_COMBO_PLATE_SPRITE: &str = "Note_LongCombo.ojt";
/// The clean plate frame: frame 0 duplicates the plate but carries a
/// stray 10-pixel artifact run along its top row; frame 1 carries the
/// plate ink alone.
const LONG_COMBO_PLATE_FRAME: usize = 1;
/// Plate ink offset inside the 128x128 frame: the 74x9 plate ink sits
/// at (52,61)-(125,69) (verified 7.55 `Playing1.opi` slot 432, frame
/// 1; frame 0 adds the stray top-row run).
pub const LONG_COMBO_PLATE_INK: (i32, i32) = (52, 61);
/// LONGNOTE COMBO plate ink box on the native canvas: the reference
/// photo's caption ink reads (91.2,282.1)-(164.6,289.6) game px
/// (bright-ink scan, `docs/playing-reference.md` method) — exactly
/// the staged frame's 74x9 ink at 1:1, so the box pins the draw with
/// no scaling.
pub const NATIVE_LONG_COMBO_PLATE: Rect = Rect {
    x: 91,
    y: 282,
    w: 74,
    h: 9,
};

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

    /// Paint centered on the lane at `center_y` (square explosion
    /// frames ride the receptor-pad center; wide art overhangs the
    /// lane and the blit clips to the frame, like the original overlap).
    fn paint_centered_on(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        center_y: i32,
    ) {
        let height = i32::try_from(self.image.h).unwrap_or(0);
        self.paint_note_at(frame, geom, lane, center_y - height / 2);
    }

    /// Paint with its top-left at `(x, y)` (blits clip to the frame).
    pub fn paint_at(&self, frame: &mut Frame<'_>, x: i32, y: i32) {
        blit_rgb555(frame, &self.image.view(), x, y, Some(self.colorkey));
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

/// Ten digit frames (stored order 0..=9) of one staged bitmap number
/// font. A font binds only with all ten frames present, so a bound
/// font never paints holes.
#[derive(Debug, Clone, Default)]
pub struct DigitFont {
    digits: [Option<SkinSprite>; 10],
}

impl DigitFont {
    /// Bind one font from ten digit frames; `None` unless every digit
    /// frame is present.
    #[must_use]
    pub fn from_frames(frames: Vec<SkinSprite>) -> Option<Self> {
        if frames.len() != 10 {
            return None;
        }
        let mut digits: [Option<SkinSprite>; 10] = Default::default();
        for (slot, frame) in digits.iter_mut().zip(frames) {
            *slot = Some(frame);
        }
        Some(Self { digits })
    }

    /// Frame of one decimal digit (`None` where the font is absent).
    #[must_use]
    pub fn digit(&self, digit: u8) -> Option<&SkinSprite> {
        self.digits.get(usize::from(digit))?.as_ref()
    }

    /// Row height of the font (digit-0 frame height; 0 while absent).
    #[must_use]
    pub fn height(&self) -> u32 {
        self.digit(0).map_or(0, |frame| frame.image.h as u32)
    }

    /// Total left-to-right width of `value` painted in this font.
    #[must_use]
    pub fn width_of(&self, value: u32) -> u32 {
        self.width_of_digits(&decimal_digits(value))
    }

    /// Width of `value` zero-padded to `places` digits.
    #[must_use]
    pub fn padded_width(&self, value: u32, places: usize) -> u32 {
        self.width_of_digits(&padded_digits(value, places))
    }

    /// Paint `value`'s digits left to right from `(x, y)`, each frame
    /// at its stored (0,0) anchor (top-left; every digit frame carries
    /// a zero anchor on 7.55). Blits clip to the frame like any skin
    /// art.
    pub fn paint(&self, frame: &mut Frame<'_>, x: i32, y: i32, value: u32) {
        self.paint_digits(frame, x, y, &decimal_digits(value));
    }

    /// Paint `value` zero-padded to `places` digits (a fixed-width
    /// readout; surplus low digits keep the right end).
    pub fn paint_padded(&self, frame: &mut Frame<'_>, x: i32, y: i32, value: u32, places: usize) {
        self.paint_digits(frame, x, y, &padded_digits(value, places));
    }

    /// Paint `value` centered on `center_x` at row `y`.
    pub fn paint_centered(&self, frame: &mut Frame<'_>, center_x: i32, y: i32, value: u32) {
        let width = i32::try_from(self.width_of(value)).unwrap_or(0);
        self.paint(frame, center_x - width / 2, y, value);
    }

    /// Sum of the digit frames' widths.
    fn width_of_digits(&self, digits: &[u8]) -> u32 {
        digits
            .iter()
            .filter_map(|digit| self.digit(*digit))
            .map(|frame| frame.image.w as u32)
            .sum()
    }

    /// Blit the digits left to right, advancing by each frame width.
    fn paint_digits(&self, frame: &mut Frame<'_>, x: i32, y: i32, digits: &[u8]) {
        let mut pen = x;
        for digit in digits {
            let Some(sprite) = self.digit(*digit) else {
                continue;
            };
            blit_rgb555(frame, &sprite.image.view(), pen, y, Some(sprite.colorkey));
            pen += i32::try_from(sprite.image.w).unwrap_or(0);
        }
    }
}

/// Decimal digits of `value`, most significant first (`0` → `[0]`).
#[must_use]
fn decimal_digits(value: u32) -> Vec<u8> {
    let mut digits = Vec::new();
    let mut rest = value;
    loop {
        digits.push(u8::try_from(rest % 10).unwrap_or(0));
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    digits.reverse();
    digits
}

/// `places` decimal digits of `value`, zero-padded on the left
/// (surplus high digits drop off the left, like a fixed-width readout).
#[must_use]
fn padded_digits(value: u32, places: usize) -> Vec<u8> {
    let mut digits = decimal_digits(value);
    digits.reverse();
    digits.resize(places, 0);
    digits.reverse();
    digits
}

/// Staged highway art: per-lane note heads, press-down keys, release
/// beams, receptor frames, the long-note effect, the hit-burst
/// animation, and the HUD number fonts ([`SCORE_FONT_SPRITE`],
/// [`COMBO_FONT_SPRITE`], [`PLAY_TIME_FONT_SPRITE`]). Empty while
/// nothing is staged: every paint path falls back to the procedural
/// highway and digits (the `real_art.rs` Playing row keys on the
/// fallback inks, so staged lanes only replace their own art).
#[derive(Debug, Clone, Default)]
pub struct PlaySkin {
    note_heads: [Option<SkinSprite>; HIGHWAY_LANES as usize],
    press_keys: [Option<SkinSprite>; HIGHWAY_LANES as usize],
    release_beams: [Option<SkinSprite>; HIGHWAY_LANES as usize],
    lane_beams: [Vec<SkinSprite>; HIGHWAY_LANES as usize],
    receptors: [Vec<SkinSprite>; HIGHWAY_LANES as usize],
    long_effect: Option<SkinSprite>,
    hold_rings: Vec<SkinSprite>,
    hit_bursts: Vec<SkinSprite>,
    jam_flash: Vec<SkinSprite>,
    score_font: Option<DigitFont>,
    combo_font: Option<DigitFont>,
    play_time_font: Option<DigitFont>,
    long_combo_plate: Option<SkinSprite>,
    combo_glow: Vec<SkinSprite>,
}

impl PlaySkin {
    /// Whether any lane or HUD font staged art.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.long_effect.is_none()
            && self.hold_rings.is_empty()
            && self.jam_flash.is_empty()
            && self.hit_bursts.is_empty()
            && self.score_font.is_none()
            && self.combo_font.is_none()
            && self.play_time_font.is_none()
            && self.long_combo_plate.is_none()
            && self.combo_glow.is_empty()
            && self.note_heads.iter().all(Option::is_none)
            && self.press_keys.iter().all(Option::is_none)
            && self.release_beams.iter().all(Option::is_none)
            && self.lane_beams.iter().all(Vec::is_empty)
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

    /// Release beam for one lane (the untouched-hold tail slice).
    #[must_use]
    pub fn release_beam(&self, lane: u8) -> Option<&SkinSprite> {
        self.release_beams.get(lane as usize)?.as_ref()
    }

    /// Steady lane-beam body for one lane ([`LANE_BEAM_STEM`]): the
    /// held lane light reads the body's last, fullest frame.
    #[must_use]
    pub fn lane_beam(&self, lane: u8) -> Option<&SkinSprite> {
        self.lane_beams.get(lane as usize)?.last()
    }

    /// Lane-beam frame for one lane ([`LANE_BEAM_STEM`]): `None` age
    /// reads the steady held-lane frame ([`Self::lane_beam`]); a flash age
    /// steps one frame per [`RECEPTOR_FRAME_MS`], clamped to the last.
    #[must_use]
    pub fn lane_beam_frame(&self, lane: u8, flash_age_ms: Option<u64>) -> Option<&SkinSprite> {
        let frames = self.lane_beams.get(lane as usize)?;
        match flash_age_ms {
            None => frames.last(),
            Some(age) => frames.get(stepped_frame(frames.len(), age)),
        }
    }

    /// Jam-flash frame count ([`JAM_EFFECT_SPRITE`]).
    #[must_use]
    pub fn jam_flash_frame_count(&self) -> usize {
        self.jam_flash.len()
    }

    /// Jam-flash frame for the age since the jam level grant: one
    /// step per [`RECEPTOR_FRAME_MS`], ending after the staged frame
    /// count (the flash plays once, it never freezes on the last
    /// frame; `None` while unstaged or past the window).
    #[must_use]
    pub fn jam_flash_frame(&self, flash_age_ms: u64) -> Option<&SkinSprite> {
        let frames = u64::try_from(self.jam_flash.len()).unwrap_or(0);
        if flash_age_ms >= frames.saturating_mul(RECEPTOR_FRAME_MS) {
            return None;
        }
        self.jam_flash
            .get(stepped_frame(self.jam_flash.len(), flash_age_ms))
    }

    /// Long-note release effect.
    #[must_use]
    pub fn long_effect(&self) -> Option<&SkinSprite> {
        self.long_effect.as_ref()
    }

    /// Hold-ring frame count ([`HOLD_RING_SPRITE`]).
    #[must_use]
    pub fn hold_ring_frame_count(&self) -> usize {
        self.hold_rings.len()
    }

    /// Hold-ring frame for a hold age ([`HOLD_RING_SPRITE`]): the
    /// steady last frame while `hold_age_ms` is `None`, then one step
    /// per [`RECEPTOR_FRAME_MS`], clamped to the last (`None` while
    /// the rings are unstaged — the static `.ojt` effect and the
    /// procedural tail keep their fallback slots).
    #[must_use]
    pub fn hold_ring_frame(&self, hold_age_ms: Option<u64>) -> Option<&SkinSprite> {
        match hold_age_ms {
            None => self.hold_rings.last(),
            Some(age) => self
                .hold_rings
                .get(stepped_frame(self.hold_rings.len(), age)),
        }
    }

    /// Hit-burst frame count ([`HIT_BURST_SPRITE`] bodies stage one
    /// shared animation, not per-lane art).
    #[must_use]
    pub fn hit_burst_frame_count(&self) -> usize {
        self.hit_bursts.len()
    }

    /// Hit-burst frame for a flash age: one step per
    /// [`RECEPTOR_FRAME_MS`], clamped to the last frame (`None` while
    /// the body is unstaged).
    #[must_use]
    pub fn hit_burst_frame(&self, flash_age_ms: u64) -> Option<&SkinSprite> {
        self.hit_bursts
            .get(stepped_frame(self.hit_bursts.len(), flash_age_ms))
    }

    /// Staged score-digit font ([`SCORE_FONT_SPRITE`]).
    #[must_use]
    pub fn score_font(&self) -> Option<&DigitFont> {
        self.score_font.as_ref()
    }

    /// Staged combo-digit font ([`COMBO_FONT_SPRITE`]).
    #[must_use]
    pub fn combo_font(&self) -> Option<&DigitFont> {
        self.combo_font.as_ref()
    }

    /// Staged play-time-digit font ([`PLAY_TIME_FONT_SPRITE`]).
    #[must_use]
    pub fn play_time_font(&self) -> Option<&DigitFont> {
        self.play_time_font.as_ref()
    }

    /// Staged long-note combo caption plate ([`LONG_COMBO_PLATE_SPRITE`]).
    #[must_use]
    pub fn long_combo_plate(&self) -> Option<&SkinSprite> {
        self.long_combo_plate.as_ref()
    }

    /// Combo-glow frame count ([`COMBO_GLOW_SPRITE`]).
    #[must_use]
    pub fn combo_glow_frame_count(&self) -> usize {
        self.combo_glow.len()
    }

    /// Combo-glow frame: the steady first frame while `flash_age_ms`
    /// is `None`, then one step per [`RECEPTOR_FRAME_MS`], clamped to
    /// the last frame. The glow plays across the atlas's stored frames
    /// in order (record-major, the exe's flat consumption), empty
    /// frames included — they paint nothing.
    #[must_use]
    pub fn combo_glow_frame(&self, flash_age_ms: Option<u64>) -> Option<&SkinSprite> {
        match flash_age_ms {
            None => self.combo_glow.first(),
            Some(age) => self
                .combo_glow
                .get(stepped_frame(self.combo_glow.len(), age)),
        }
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
            Some(age) => frames.get(stepped_frame(frames.len(), age)),
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

    /// Stage one lane's kit lane-beam frames ([`LANE_BEAM_STEM`]).
    #[must_use]
    pub fn with_lane_beams(mut self, lane: u8, frames: Vec<SkinSprite>) -> Self {
        if let Some(slot) = self.lane_beams.get_mut(lane as usize) {
            *slot = frames;
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

    /// Stage the hit-burst animation frames.
    #[must_use]
    pub fn with_hit_bursts(mut self, frames: Vec<SkinSprite>) -> Self {
        self.hit_bursts = frames;
        self
    }

    /// Stage the score-digit font.
    #[must_use]
    pub fn with_score_font(mut self, font: DigitFont) -> Self {
        self.score_font = Some(font);
        self
    }

    /// Stage the combo-digit font.
    #[must_use]
    pub fn with_combo_font(mut self, font: DigitFont) -> Self {
        self.combo_font = Some(font);
        self
    }

    /// Stage the play-time-digit font.
    #[must_use]
    pub fn with_play_time_font(mut self, font: DigitFont) -> Self {
        self.play_time_font = Some(font);
        self
    }

    /// Stage the long-note combo caption plate.
    #[must_use]
    pub fn with_long_combo_plate(mut self, plate: SkinSprite) -> Self {
        self.long_combo_plate = Some(plate);
        self
    }

    /// Stage the combo-glow animation frames.
    #[must_use]
    pub fn with_combo_glow(mut self, frames: Vec<SkinSprite>) -> Self {
        self.combo_glow = frames;
        self
    }

    /// Decode every staged body from a read-only archive slice; `None`
    /// when nothing stages (entries missing or undecodable). Stage-less
    /// form: the combo glow rides the deterministic
    /// [`COMBO_GLOW_SPRITE`] atlas.
    #[must_use]
    pub fn from_archive(archive: &[u8]) -> Option<Self> {
        Self::from_archive_for_stage(archive, None)
    }

    /// [`from_archive`] through the room arena stage: the combo glow
    /// stages from the stage's own `Note_Combo<stage>.ojt` atlas (the
    /// exe applier's stage-numbered arm for `_Note_Combo.ojt`). A stage
    /// whose body is missing or not a strided atlas leaves the glow
    /// unstaged — the combo digits keep painting — like every optional
    /// staged body.
    #[must_use]
    pub fn from_archive_for_stage(archive: &[u8], arena_stage: Option<u16>) -> Option<Self> {
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
            let frames = archive_body(archive, &lane_beam_sprite_name(lane)).map(skin_frames);
            if let Some(frames) = frames.filter(|frames| !frames.is_empty()) {
                if let Some(slot) = skin.lane_beams.get_mut(lane as usize) {
                    *slot = frames;
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
        skin.hold_rings = archive_body(archive, HOLD_RING_SPRITE)
            .map(oja_skin_frames)
            .unwrap_or_default();
        skin.jam_flash = archive_body(archive, JAM_EFFECT_SPRITE)
            .map(oja_skin_frames)
            .unwrap_or_default();
        skin.hit_bursts = archive_body(archive, HIT_BURST_OJA_SPRITE)
            .map(oja_skin_frames)
            .filter(|frames| !frames.is_empty())
            .or_else(|| archive_body(archive, HIT_BURST_SPRITE).map(skin_frames))
            .unwrap_or_default();
        skin.score_font = digit_font(archive, SCORE_FONT_SPRITE);
        skin.combo_font = digit_font(archive, COMBO_FONT_SPRITE);
        skin.play_time_font = digit_font(archive, PLAY_TIME_FONT_SPRITE);
        skin.long_combo_plate = archive_body(archive, LONG_COMBO_PLATE_SPRITE)
            .and_then(|body| lane_sprite(body, LONG_COMBO_PLATE_FRAME));
        let glow_name =
            arena_stage.map_or_else(|| COMBO_GLOW_SPRITE.to_owned(), combo_glow_sprite_name);
        skin.combo_glow = archive_body(archive, &glow_name)
            .map(strided_skin_frames)
            .unwrap_or_default();
        (!skin.is_empty()).then_some(skin)
    }
}

/// `KeyEffect_Tutorial_%d.ojs` body name for one lane (1-based).
fn tutorial_sprite_name(lane: u8) -> String {
    format!("{TUTORIAL_EFFECT_STEM}_{}.ojs", lane + 1)
}

/// `KeyEffect_7Key1_%d.ojs` kit body name for one lane (1-based).
fn lane_beam_sprite_name(lane: u8) -> String {
    format!("{LANE_BEAM_STEM}_{}.ojs", lane + 1)
}

/// `Note_Combo%d.ojt` glow body name for one room arena stage (the
/// stage-numbered arm of `_Note_Combo.ojt`).
fn combo_glow_sprite_name(stage: u16) -> String {
    format!("Note_Combo{stage}.ojt")
}

/// Frame index for a flash age: one step per [`RECEPTOR_FRAME_MS`],
/// clamped to the last frame.
fn stepped_frame(count: usize, age_ms: u64) -> usize {
    let last = count.saturating_sub(1);
    usize::try_from(age_ms / RECEPTOR_FRAME_MS)
        .unwrap_or(last)
        .min(last)
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

/// Glow frames of one strided atlas body in stored order (record-major,
/// [`o2jam_file::Sprite55Strided::frame_data`] — the exe consumes the
/// blob flat): empty unless the body parses as a strided atlas with
/// empty unless the body parses as a strided atlas with raw frames
/// (plain single-frame glow stages sit outside the strided gap). Every
/// pixel comes from the parser — none is invented.
fn strided_skin_frames(body: &[u8]) -> Vec<SkinSprite> {
    let Some(sprite) = strided_sprite55(body) else {
        return Vec::new();
    };
    let mut frames = Vec::new();
    for image in &sprite.images {
        if !sprite.is_raw_rgb555(image) {
            continue;
        }
        for frame in 0..sprite.frames_per_image {
            if let Some(decoded) = Rgb555Image::decode(
                sprite.frame_data(image, frame),
                image.w as usize,
                image.h as usize,
            ) {
                frames.push(SkinSprite::new(decoded, sprite.colorkey));
            }
        }
    }
    frames
}

/// `.oja` effect frames of one body in stored order: every frame's
/// row-group pixel plane pastes onto its full `w*h` canvas at the
/// stored transparent word 0 ([`FigureLayer::from_oja_frame`] — one
/// paste, shared with the scene figures). Empty unless the body
/// decodes as a `.oja` container. Anchors stay on the layer: the
/// centred-effect records carry negative frame-centre anchors, and
/// the lane-centred effect paints already draw from the frame
/// centre, so a [`SkinSprite`] carries the pixels alone.
fn oja_skin_frames(body: &[u8]) -> Vec<SkinSprite> {
    let Ok(animation) = o2jam_file::parse_oja55(body) else {
        return Vec::new();
    };
    animation
        .frames
        .iter()
        .map(|frame| {
            let layer = FigureLayer::from_oja_frame(frame);
            SkinSprite::new(layer.image, layer.colorkey)
        })
        .collect()
}

/// One HUD number font from an archive body: ten digit frames in
/// stored order 0..=9; `None` unless every digit decodes as raw
/// RGB555 (a compressed frame would misalign the digit index, so the
/// font stays out and the procedural digits keep painting).
fn digit_font(archive: &[u8], entry_name: &str) -> Option<DigitFont> {
    let body = archive_body(archive, entry_name)?;
    let digits = (0..10usize)
        .map(|index| lane_sprite(body, index))
        .collect::<Option<Vec<_>>>()?;
    DigitFont::from_frames(digits)
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

    /// Paint highway, line, and visible notes over the flat
    /// placeholder backdrop (the no-archive fallback fill).
    pub fn paint(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
        self.paint_backdrop_fill(frame);
        self.paint_curtained_highway(frame, geom, LaneCurtain::Full);
    }

    /// Fill the frame with the flat Playing backdrop: the loop's
    /// placeholder draw, kept only for the no-archive case (ADR-0003).
    fn paint_backdrop_fill(&self, frame: &mut Frame<'_>) {
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
    }

    /// Paint the highway through a skill-ring curtain: notes outside the
    /// item-text spans (hidden keeps the top 80%, sudden the bottom
    /// 20%, dark the middle 20%, reverse-dark the outer 40% pair —
    /// rounded bands of the staged highway box via
    /// `o2jam-render::visible_spans`) are not drawn.
    /// `Full` paints everything, like [`PlayingView::paint`].
    pub fn paint_curtained(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        curtain: LaneCurtain,
    ) {
        self.paint_backdrop_fill(frame);
        self.paint_curtained_highway(frame, geom, curtain);
    }

    /// Paint the highway through a skill-ring curtain over an
    /// already-blitted staged backdrop (A44): the flat placeholder
    /// fill stays out so the staged scene set shows through — it is
    /// ADR-0003's no-archive fallback only.
    pub fn paint_curtained_over_staged(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        curtain: LaneCurtain,
    ) {
        self.paint_curtained_highway(frame, geom, curtain);
    }

    /// Lane shells, receptors, judgement line, and the visible notes
    /// the curtain lets through (no backdrop fill).
    fn paint_curtained_highway(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        curtain: LaneCurtain,
    ) {
        self.paint_highway_shell(frame, geom);
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

    /// Lane tints, separators, receptors, and the judgement line (no
    /// notes, no backdrop fill). Even lanes keep the surface under
    /// them (the flat fallback fill or the staged venue); odd lanes
    /// carry the tint above so staged `Playing1.opi` edge gauges keep
    /// ringing a readable highway. The line spans the staged lane
    /// columns, not the full frame.
    fn paint_highway_shell(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry) {
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

/// Jam gauge shells scaled into the frame: the staged left/right
/// `Jamgauge` edge bars below the judgement row plus the staged
/// panel-bottom bar (last, so it paints over an edge-bar overlap).
fn jam_bar_rects(frame_w: usize, frame_h: usize) -> [Rect; 3] {
    let edge_bars = NATIVE_JAM_BARS.map(|rect| scale_rect_to_frame(&rect, frame_w, frame_h));
    let bottom_bar = scale_rect_to_frame(&NATIVE_JAM_BOTTOM_BAR, frame_w, frame_h);
    [edge_bars[0], edge_bars[1], bottom_bar]
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

/// Filled rows for `progress_in_level` (`0..50`) over a `bar_h`-row
/// gauge interior. Integer math keeps the exe's cumulative-progress
/// shape (each level spans [`JAM_PROGRESS_PER_LEVEL`]; no reset on
/// grant).
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

/// One hittable record: rated once, by key press, key-up strike, or
/// expiry. A long note pairs its head record with a chart tail
/// ([`HoldSpan`]): the head strike arms the hold and the per-lane
/// tail cursor `+0x864` (Cool/Good), the armed hold ticks bonus while
/// its span is open, and the key-up release judges the armed tail
/// one-shot via [`HoldState::judge_tail`] with the shared windows
/// (≥26 ms off = Miss, either side). The expiry scan is inert (its
/// cursors never advance), so a held tail never expires; a missed or
/// Bad head instead marks the tail judged (the verified kill).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingNote {
    lane: u8,
    at_ms: u64,
    tail_ms: Option<u64>,
    sample: u32,
    judged: bool,
    /// Display-only beam flag ("key down on this struck hold"); never
    /// read by timing or scoring — the verified hold flag never clears.
    holding: bool,
    /// Hold strike stamp in song ms (visual ring animation anchor;
    /// never read by timing or scoring).
    holding_since: Option<u64>,
    tail_judged: bool,
    /// Verified hold machine for a long note (`None` on taps).
    hold: Option<HoldState>,
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
            holding_since: None,
            tail_judged: true,
            hold: None,
        }
    }

    /// Long note over a validated [`HoldSpan`] (head to tail record).
    fn hold(lane: u8, span: HoldSpan, sample: u32) -> Self {
        Self {
            lane,
            at_ms: span.head_ms,
            tail_ms: Some(span.tail_ms),
            sample,
            judged: false,
            holding: false,
            holding_since: None,
            tail_judged: false,
            hold: Some(HoldState::new(span)),
        }
    }

    /// Done when the head is judged and the tail (if any) is judged.
    fn done(&self) -> bool {
        self.judged && self.tail_judged
    }
}

/// Live play: scrolling view plus verified judgement/scoring.
///
/// The clock seam: every `now_ms` passed here is song position —
/// `ScreenGame::now_ms` (audio backend `samples_played` through
/// `song_position_ms`) on the live path and explicit song ms in
/// tests. Strike timing (the 6/18/25 ms windows of
/// `FUN_00576540`, file `0x176540`, `ev_f83fe4e4…`), scroll
/// ([`PlayingView::visible`]), miss expiry, hold bonus ticks, and
/// every [`Score`] judgement advance off that position, never the
/// wall clock.
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
    /// Verified hold-bonus accumulator (+10 per held tick, game
    /// `+0x3978`): fed per advance through [`HoldState::hold_tick_due`]
    /// gates; kept separate from [`Score`], like the exe's sub-object.
    bonus: LongNoteBonus,
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
    /// Jam level grant stamp in song ms (visual flash only; scoring
    /// and timing never read this).
    jam_flash_at: Option<u64>,
    /// Display-only avatar scene captions (level, song, volume).
    scene: AvatarScene,
    /// Slots the 4015 peer broadcast has reported: occupancy for the
    /// seat gauges (unset slots paint nothing — the solo room shows
    /// no peer gauges on the reference photo).
    peer_seen: [bool; PEER_SLOT_COUNT],
}

impl Default for LivePlay {
    fn default() -> Self {
        Self {
            view: PlayingView::default(),
            pending: Vec::new(),
            score: Score::default(),
            pills: PillStock::default(),
            bonus: LongNoteBonus::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
            jam_flash_at: None,
            scene: AvatarScene::default(),
            peer_seen: [false; PEER_SLOT_COUNT],
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
            bonus: LongNoteBonus::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
            jam_flash_at: None,
            scene: AvatarScene::default(),
            peer_seen: [false; PEER_SLOT_COUNT],
        }
    }

    /// Notes with long-note tails: `(lane, head ms, tail ms, sample)`.
    /// A tail equal to (or before) its head degrades to a tap.
    #[must_use]
    pub fn new_holds(entries: &[(u8, u64, u64, u32)]) -> Self {
        let mut pending: Vec<PendingNote> = entries
            .iter()
            .map(|(lane, head_ms, tail_ms, sample)| {
                HoldSpan::new(*head_ms, *tail_ms).map_or_else(
                    || PendingNote::tap(*lane, *head_ms, *sample),
                    |span| PendingNote::hold(*lane, span, *sample),
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
            bonus: LongNoteBonus::default(),
            peers: PeerBars::default(),
            streak: CoolStreak::default(),
            windows: TimingWindows::default(),
            reshuffle: MeasureReshuffle::default(),
            curtain: LaneCurtain::Full,
            now_ms: 0,
            last_word: None,
            sounds: Vec::new(),
            lane_flash: [None; HIGHWAY_LANES as usize],
            jam_flash_at: None,
            scene: AvatarScene::default(),
            peer_seen: [false; PEER_SLOT_COUNT],
        }
    }

    /// Skill-ring curtain worn for this run (default: none).
    pub fn set_curtain(&mut self, curtain: LaneCurtain) {
        self.curtain = curtain;
    }

    /// Select the life-bar delta set from the playing chart's difficulty
    /// (0 = Easy, 1 = Normal, 2 = Hard — VERIFIED switch `FUN_00570f90`).
    /// Codes outside 0..=2 (e.g. Rx) select nothing and keep the current
    /// set, like the exe's switch fallthrough (file `0x170fb4`). The bar
    /// stays full: only judgements move it (setter `0x5407b0` pushes
    /// 1000 at init, file `0x4dae7c`).
    pub fn set_life_difficulty(&mut self, difficulty: u8) {
        if let Some(bar) = LifeBar::new(difficulty) {
            self.score.life = bar;
        }
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

    /// Paint the peer gauges plus the right-side avatar scene and bottom
    /// tally strip over the highway and staged art — this is the over-art
    /// layer, so the LONGNOTE COMBO caption plate rides here too (staged
    /// scene art paints after [`Self::paint`], and the caption must cover
    /// it, like the strip readouts). The peer gauges sit on the staged
    /// venue art, so file order puts them here, after the overlay.
    /// Counts feed from the live [`Score`]; state is untouched (shared
    /// `&self` like a query).
    pub fn paint_avatar_scene(
        &self,
        frame: &mut Frame<'_>,
        avatar: Option<&AvatarFigure>,
        now_ms: u64,
    ) {
        self.paint_avatar_scene_with_placements(frame, avatar, now_ms, None);
    }

    /// [`paint_avatar_scene`](Self::paint_avatar_scene) with the
    /// bound strip fields read from staged placements.
    pub fn paint_avatar_scene_with_placements(
        &self,
        frame: &mut Frame<'_>,
        avatar: Option<&AvatarFigure>,
        now_ms: u64,
        placements: Option<&[ScenePlacement]>,
    ) {
        self.paint_peer_bars(frame);
        self.scene.paint_with_placements(
            frame,
            avatar,
            &self.score,
            now_ms,
            self.song_end_ms(),
            placements,
        );
        self.paint_long_combo_caption(frame);
    }

    /// LONGNOTE COMBO caption plate over the lanes through the staged
    /// `Note_LongCombo.ojt` body: the frame blits 1:1 so its 74x9 ink
    /// lands on the photo-pinned box (see [`NATIVE_LONG_COMBO_PLATE`]).
    /// Staged-only — without the body nothing paints. The exe's
    /// show/hide rule is untraced, so the plate paints while staged
    /// (the reference photo shows it standing mid-song).
    fn paint_long_combo_caption(&self, frame: &mut Frame<'_>) {
        let Some(plate) = self.view.skin.long_combo_plate() else {
            return;
        };
        let (origin_x, origin_y) = long_combo_plate_origin(frame.w, frame.h);
        plate.paint_at(frame, origin_x, origin_y);
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
        let event = self.peers.apply_stats_update(slot, kind, value);
        if let Some(seen) = self.peer_seen.get_mut(slot as usize) {
            *seen = true;
        }
        event
    }

    /// Apply one 8-byte peer rank broadcast; returns per-position
    /// deliveries under the rank-remap rule.
    pub fn apply_peer_ranks(&mut self, packet: [u8; PEER_SLOT_COUNT]) -> Vec<RankDelivery> {
        self.peers.apply_rank_broadcast(packet)
    }

    /// Seed the rank-mirror holder bytes (scripted walk fixtures
    /// only): the song path never sets these — only a real 4015
    /// population (unwired) or fixture code does — so ranks stay
    /// hidden until a broadcast with matching holders delivers.
    pub fn set_rank_holders(&mut self, global: u8, stored: [u8; PEER_SLOT_COUNT]) {
        self.peers.set_global_rank(global);
        for (slot, rank) in stored.iter().enumerate() {
            self.peers.set_stored_rank(slot, *rank);
        }
    }

    /// Move the song clock; unjudged heads and taps past the bad
    /// window rate Miss. Held tails never expire: the verified 7.55
    /// expiry scan never advances its per-lane cursors, so its Miss is
    /// unreachable (`docs/gameplay.md`) — a held tail rates only on
    /// its release strike ([`Self::release_lane`] via
    /// [`HoldState::judge_tail`]). A missed head still marks its paired
    /// tail judged (the scan kill — a mark, never a rating), and the
    /// hold-bonus scan then feeds one +10 tick per armed hold whose
    /// span window is still open (strict on both ends).
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
            }
        }
        for judgement in late {
            self.rate(judgement);
        }
        let held_spans = self
            .pending
            .iter()
            .filter_map(|note| note.hold.as_ref())
            .filter(|hold| hold.hold_tick_due(now_ms))
            .count();
        for _ in 0..held_spans {
            self.bonus.hold_tick();
        }
    }

    /// Strike a lane (key-down): rate the nearest unjudged record in
    /// range. Returns the judgement, or `None` for an empty press.
    pub fn strike_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        let i = self.nearest_strike_in_lane(lane, now_ms)?;
        let is_hold = self.pending[i].tail_ms.is_some();
        self.sounds.push(self.pending[i].sample);
        let judgement = self.strike_record(i, now_ms);
        self.pending[i].holding = is_hold;
        self.pending[i].holding_since = is_hold.then_some(now_ms);
        Some(judgement)
    }

    /// Release a lane (key-up): strike the nearest unjudged record in
    /// the lane with the shared strike windows (tails excluded), then
    /// judge the armed hold tail via [`HoldState::judge_tail`] — the
    /// verified `+0x864` model: a Cool/Good head armed the per-lane
    /// tail cursor at its paired tail, and this release rates the tail
    /// against the tail time with the shared windows, one-shot (the
    /// cursor clears on consumption; ≥26 ms off = Miss, either side).
    /// A Bad or missed head leaves nothing armed, so its release rates
    /// nothing on the tail; the armed hold keeps ticking its bonus
    /// either way (the hold flag never clears mid-song). Returns the
    /// tail judgement when one fired, else the struck record's
    /// judgement, or `None`.
    pub fn release_lane(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        self.advance_to(now_ms);
        for note in &mut self.pending {
            if note.lane == lane {
                note.holding = false;
                note.holding_since = None;
            }
        }
        let struck = self
            .nearest_strike_in_lane(lane, now_ms)
            .map(|i| self.strike_record(i, now_ms));
        self.judge_armed_tail(lane, now_ms).or(struck)
    }

    /// Judge the lane's armed hold tail (the verified key-up branch,
    /// judge files `0x176646`–`0x176687`, cursor clear `0x176709`):
    /// consume the one-shot cursor via [`HoldState::judge_tail`], mark
    /// the tail judged, and rate the fired judgement. `+0x864` is one
    /// cursor per lane in-exe; the per-note model judges each armed
    /// hold, so a degenerate same-lane overlap rates one release each.
    /// Returns the last judgement fired, or `None` when nothing was
    /// armed.
    fn judge_armed_tail(&mut self, lane: u8, now_ms: u64) -> Option<Judgement> {
        let windows = self.windows;
        let mut fired = Vec::new();
        for note in &mut self.pending {
            if note.lane != lane {
                continue;
            }
            let Some(hold) = note.hold.as_mut() else {
                continue;
            };
            if let Some(judgement) = hold.judge_tail(now_ms, &windows) {
                note.tail_judged = true;
                fired.push(judgement);
            }
        }
        let last = fired.last().copied();
        for judgement in fired {
            self.rate(judgement);
        }
        last
    }

    /// Index of the nearest unjudged record in `lane` within the bad
    /// window, closer-of before/after the strike time (`None` when the
    /// strike lands on nothing). Shared by the key-down judge and the
    /// key-up candidate walk; tail records are never pending records
    /// here, so both walks see heads only.
    fn nearest_strike_in_lane(&self, lane: u8, now_ms: u64) -> Option<usize> {
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
        best.map(|(i, _)| i)
    }

    /// Strike record `i`: tier the offset with the shared strike
    /// windows, apply the hold wiring (Cool/Good arms the hold and the
    /// `+0x864` tail cursor its release will judge; a Bad strike kills
    /// the paired tail — the verified `0x180` kill, a mark, never a
    /// rating — leaving nothing armed), remove the head from the
    /// scroll, flash the lane, and rate. Returns the tier.
    fn strike_record(&mut self, i: usize, now_ms: u64) -> Judgement {
        let offset = now_ms as i64 - self.pending[i].at_ms as i64;
        let (lane, at_ms, is_hold) = (
            self.pending[i].lane,
            self.pending[i].at_ms,
            self.pending[i].tail_ms.is_some(),
        );
        let judgement = self.windows.judge(offset);
        let note = &mut self.pending[i];
        note.judged = true;
        if let Some(hold) = note.hold.as_mut() {
            hold.apply_head(judgement);
        }
        if judgement == Judgement::Bad && is_hold {
            note.tail_judged = true;
        }
        self.view.strike(lane, at_ms);
        self.flash_lane(lane, now_ms);
        self.rate(judgement);
        judgement
    }

    /// Running score and combo.
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// Live life word (`0..=LIFE_MAX`) behind the avatar gauge: the
    /// public shell read for the Life channel (the rank snapshot row
    /// 0 carries points, jam level, and bonus — never life). Reads
    /// the verified bar directly, so the session tick always
    /// matches the painted gauge.
    #[must_use]
    pub fn life(&self) -> u16 {
        self.score.life.life()
    }

    /// Verified hold-bonus accumulator (game `+0x3978`; the lead
    /// `longNoteScore` candidate for 4016): one +10 tick per held span
    /// per advance while armed and the span window is open.
    #[must_use]
    pub fn bonus(&self) -> &LongNoteBonus {
        &self.bonus
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
        let level_before = self.score.jam.level();
        self.streak.note(judgement, &mut self.pills);
        self.score.apply_rated(judgement, &mut self.pills);
        self.last_word = Some((judgement, self.now_ms));
        if self.score.jam.level() > level_before {
            self.jam_flash_at = Some(self.now_ms);
        }
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
    /// heads rate through `strike_lane` / key-up strikes, tails on the
    /// key-up release strike). Held heads beam from the judgement line;
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
            self.paint_tail_span(frame, geom, note, top, bottom);
        }
    }

    /// One tail span: the staged kit lane beam on held heads (the
    /// exe's held lane light, [`LANE_BEAM_STEM`]), then the staged
    /// `.oja` hold rings stepping from the hold strike
    /// ([`HOLD_RING_SPRITE`]), then the staged static long effect; the
    /// staged release beam on untouched holds, else the procedural
    /// [`TAIL_BEAM`] core.
    fn paint_tail_span(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        note: &PendingNote,
        top: i32,
        bottom: i32,
    ) {
        let lane = note.lane;
        if note.holding {
            if let Some(beam) = self.view.skin.lane_beam_frame(lane, None) {
                beam.paint_bottom_anchored(frame, geom, lane, geom.line_y);
                return;
            }
            let hold_age_ms = note
                .holding_since
                .map(|since| self.now_ms.saturating_sub(since));
            if let Some(ring) = self.view.skin.hold_ring_frame(hold_age_ms) {
                ring.paint_bottom_anchored(frame, geom, lane, geom.line_y);
                return;
            }
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

    /// Lit receptor pads plus burst art on flashed lanes (staged
    /// `Note_Click` bursts over `KeyEffect_Tutorial_%d` receptors,
    /// `KeydownImage` press art, and the kit lane beams when present;
    /// procedural pad and column otherwise).
    fn paint_hit_bursts(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        for lane in 0..HIGHWAY_LANES {
            let Some(age) = self.flash_age(lane, now_ms) else {
                continue;
            };
            self.paint_lane_flash(frame, geom, lane, age);
        }
    }

    /// One flashed lane: staged beam column, animated receptor, press
    /// art, and the staged hit burst over them (each stage falls back
    /// to its procedural ink when unstaged).
    fn paint_lane_flash(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        age_ms: u64,
    ) {
        self.paint_burst_column(frame, geom, lane, age_ms);
        self.view
            .paint_receptor(frame, geom, lane, RECEPTOR_LIT, Some(age_ms));
        if let Some(press) = self.view.skin.press_key(lane) {
            press.paint_bottom_anchored(frame, geom, lane, geom.line_y);
        }
        self.paint_staged_burst(frame, geom, lane, age_ms);
    }

    /// The lane column under the flash: the staged kit lane beam
    /// ([`LANE_BEAM_STEM`], the exe's bright press beam, drawn as the
    /// full fading column it authors), else the staged `Keyeffect`
    /// slice, else the procedural [`HIT_BURST`] column (skipped when
    /// the staged burst art already covers the lane).
    fn paint_burst_column(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        age_ms: u64,
    ) {
        if let Some(beam) = self.view.skin.lane_beam_frame(lane, Some(age_ms)) {
            beam.paint_bottom_anchored(frame, geom, lane, geom.line_y);
        } else if let Some(beam) = self.view.skin.release_beam(lane) {
            let burst = geom.burst_rect(lane);
            beam.paint_clipped_to(frame, geom, lane, geom.line_y, &burst);
        } else if self.view.skin.hit_burst_frame(age_ms).is_none() {
            fill_rect(frame, &geom.burst_rect(lane), HIT_BURST);
        }
    }

    /// The staged hit burst ([`HIT_BURST_SPRITE`]) centered on the
    /// receptor pad. Staged-only: without the body nothing paints (the
    /// column fallback covers the flash). Painted last like the exe's
    /// file order — the `NOTECLICK` entries follow `KEYDOWN` in the
    /// control list (`docs/ui-draw.md`, per-hit effect entries).
    fn paint_staged_burst(
        &self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        lane: u8,
        age_ms: u64,
    ) {
        let Some(burst_frame) = self.view.skin.hit_burst_frame(age_ms) else {
            return;
        };
        let pad = geom.receptor_pad(lane);
        let pad_center_y = pad.y + i32::try_from(pad.h).unwrap_or(0) / 2;
        burst_frame.paint_centered_on(frame, geom, lane, pad_center_y);
    }

    /// Center x of the staged lane columns: the draw point the
    /// judgement word and combo cluster center on (the lane spans run
    /// `5..193` at native, so the cluster rides the lanes, not the
    /// frame center at 400).
    fn highway_center_x(geom: &HighwayGeometry) -> i32 {
        let span = geom.highway_rect();
        span.x + i32::try_from(span.w).unwrap_or(0) / 2
    }

    /// Judgement word above the line with the running combo beneath
    /// it, both centered on the highway. The staged word art is
    /// center-anchored — every `Note_Cool/Good/Bad/Miss.ojs` frame
    /// carries a signed anchor near `(-w/2, -h/2)` (e.g. `Note_Cool`
    /// frame 2 `(-78,-37)` 154x72) — so the draw point is the word
    /// center, and `docs/playing-reference.md` puts the big combo +
    /// tier word over the lanes. Word rows keep the staged offsets
    /// above the judgement line; ink stays plain white. Combo digits
    /// paint through the staged `Note_ComboNum.ojs` font when present
    /// (the big photo digits), procedural glyphs otherwise.
    fn paint_judgement_word(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        let Some((judgement, at)) = self.last_word else {
            return;
        };
        if now_ms.saturating_sub(at) >= 600 {
            return;
        }
        let center_x = Self::highway_center_x(geom);
        let word = word_of(judgement);
        let text_w = i32::try_from(text_width(word)).unwrap_or(0);
        draw_text(
            frame,
            center_x - text_w / 2,
            geom.line_y - 40,
            word,
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        if self.score.combo > 0 {
            self.paint_combo_glow(frame, geom, now_ms.saturating_sub(at));
            match self.view.skin.combo_font() {
                Some(font) => {
                    font.paint_centered(
                        frame,
                        center_x,
                        geom.line_y - COMBO_ROW_ABOVE_LINE,
                        self.score.combo,
                    );
                }
                None => {
                    let digits = i32::try_from(number_width(self.score.combo)).unwrap_or(0);
                    draw_number(
                        frame,
                        center_x - digits / 2,
                        geom.line_y - COMBO_ROW_ABOVE_LINE,
                        self.score.combo,
                        pack_xrgb(0xFF, 0xFF, 0x40),
                    );
                }
            }
        }
    }

    /// Combo glow band behind the digits: the staged strided atlas
    /// ([`COMBO_GLOW_SPRITE`]) steps one frame per [`RECEPTOR_FRAME_MS`]
    /// across the word window (`glow_age_ms` since the judgement; the
    /// combo cluster shows only while the word is fresh). Every atlas
    /// record anchors `(0, 0)`, so the band centers on the combo
    /// cluster point — the digits' draw center — and the word and
    /// digits paint over it. Staged-only: without the atlas nothing
    /// paints (the digits keep their plain band).
    fn paint_combo_glow(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, glow_age_ms: u64) {
        let Some(glow) = self.view.skin.combo_glow_frame(Some(glow_age_ms)) else {
            return;
        };
        let (w, h) = glow.extent();
        let center_x = Self::highway_center_x(geom);
        glow.paint_at(
            frame,
            center_x - i32::try_from(w).unwrap_or(0) / 2,
            geom.line_y - COMBO_ROW_ABOVE_LINE - i32::try_from(h).unwrap_or(0) / 2,
        );
    }

    /// Jam flash over the lanes (staged [`JAM_EFFECT_SPRITE`] frames):
    /// plays once per jam level grant, one frame per
    /// [`RECEPTOR_FRAME_MS`] from the grant stamp, centered on the
    /// highway at the judgement line (the 300x300 frames anchor
    /// `(0, 0)` in the container; the exe draw point is untraced, so
    /// the staged flash rides the lane columns it effects — the blit
    /// clips to the frame). Ignores the sight-ring curtain like the
    /// combo glow (per-hit effect overlays). Staged-only: without the
    /// body nothing paints (the gauge keeps its bars and digits).
    fn paint_jam_flash(&self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        let Some(stamped) = self.jam_flash_at else {
            return;
        };
        let age = now_ms.saturating_sub(stamped);
        let Some(flash) = self.view.skin.jam_flash_frame(age) else {
            return;
        };
        let (w, h) = flash.extent();
        let center_x = Self::highway_center_x(geom);
        flash.paint_at(
            frame,
            center_x - i32::try_from(w).unwrap_or(0) / 2,
            geom.line_y - i32::try_from(h).unwrap_or(0) / 2,
        );
    }

    /// Peer life bars: one staged 46x8 gauge per 4015 member slot,
    /// stacked from the top-right corner, each filled green bottom-up
    /// over a dark-red empty bar.
    fn paint_peer_bars(&self, frame: &mut Frame<'_>) {
        // Only seated peers paint (reference photo: the solo room
        // shows no peer gauges); occupancy rides the 4015 reports.
        for ((level, bar), seen) in self
            .peers
            .life_levels()
            .iter()
            .zip(peer_bar_rects(frame.w, frame.h))
            .zip(self.peer_seen)
        {
            if seen {
                Self::paint_peer_bar(frame, &bar, *level);
            }
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

    /// Jam edge bars plus the panel-bottom bar, and level digits from
    /// [`JamState`]: Cool feeds +2, Good feeds +1, each level spans
    /// [`JAM_PROGRESS_PER_LEVEL`] cumulative progress (`FUN_00570260` /
    /// `FUN_00570510`), and a pill-saved Bad feeds like a Cool
    /// (`apply_rated`). The bars fill where the staged shells sit —
    /// `Jamgauge_right_bar.ojs` / `Jamgauge_left_bar.ojs` (VAs
    /// `0x64b1b8` / `0x64b1d0`, loader `FUN_004e7850`) and the
    /// panel-bottom `Jam_gauge_bar.ojs` (slot 31); fills rise inside
    /// the shell interior ([`GAUGE_FILL_INSET`]). The digits stand in
    /// for the `Note_JamNum.ojs` (`0x64c0fc` → `0x50d0ae`) art until
    /// staged figures bind (its `(0,0)` anchor is unbound).
    fn paint_jam_gauge(&self, frame: &mut Frame<'_>) {
        let progress = self.score.jam.level_progress();
        for bar in jam_bar_rects(frame.w, frame.h) {
            Self::paint_jam_bar(frame, &bar, progress);
        }
        self.paint_jam_level(frame);
    }

    /// One empty jam shell plus its progress fill, rising from the
    /// bottom of the shell interior (one pixel in from each side).
    fn paint_jam_bar(frame: &mut Frame<'_>, bar: &Rect, progress: u64) {
        let interior = panel_content_rect(bar, GAUGE_FILL_INSET);
        let filled = jam_fill_rows(progress, interior.h);
        fill_rect(frame, bar, pack_xrgb(0x10, 0x10, 0x30));
        if filled > 0 {
            fill_rect(
                frame,
                &Rect {
                    x: interior.x,
                    y: interior.y + (interior.h - filled) as i32,
                    w: interior.w,
                    h: filled,
                },
                pack_xrgb(0xFF, 0x80, 0x00),
            );
        }
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

    /// Scaled panel-strip readout origins: `(combo x, score center x,
    /// row)` from the staged bottom jam bar and strip row. Score
    /// digits center over the bar (photo: score center of the panel
    /// strip), combo digits take the bar's left edge (photo: MAX
    /// COMBO left).
    fn strip_origins(frame_w: usize, frame_h: usize) -> (i32, i32, i32) {
        let bar = jam_bar_rects(frame_w, frame_h)[2];
        let row = scale_row(STRIP_ROW_NATIVE, frame_h);
        let bar_w = i32::try_from(bar.w).unwrap_or(0);
        (bar.x, bar.x + bar_w / 2, row)
    }

    /// Score and combo digits in the highway-panel strip (below the
    /// jam bar, on the staged strip row) — replacing the invented
    /// top-left `(16,16)`/`(16,32)` anchors. Score digits paint
    /// through the staged `Scroe.ojs` font when present, procedural
    /// glyphs otherwise; the strip combo keeps the procedural glyphs
    /// (its own 7x10 `ComboScroe.ojs` family is outside this
    /// three-font binding).
    fn paint_panel_strip(&self, frame: &mut Frame<'_>) {
        let (combo_x, score_center_x, row) = Self::strip_origins(frame.w, frame.h);
        let points = u32::try_from(self.score.points).unwrap_or(u32::MAX);
        match self.view.skin.score_font() {
            Some(font) => font.paint_centered(frame, score_center_x, row, points),
            None => {
                let digits = i32::try_from(number_width(points)).unwrap_or(0);
                draw_number(
                    frame,
                    score_center_x - digits / 2,
                    row,
                    points,
                    pack_xrgb(0xFF, 0xFF, 0xFF),
                );
            }
        }
        if self.score.combo > 0 {
            draw_number(
                frame,
                combo_x,
                row,
                self.score.combo,
                pack_xrgb(0xFF, 0xFF, 0x40),
            );
        }
    }

    /// Paint highway, notes, tails, jam gauge, panel-strip score/combo,
    /// bursts, and word over the flat placeholder backdrop (the
    /// no-archive fallback). Peer bars stay out: they ride the over-art
    /// layer ([`Self::paint_avatar_scene`], after the staged overlay).
    pub fn paint(&mut self, frame: &mut Frame<'_>, geom: &HighwayGeometry, now_ms: u64) {
        self.paint_over(frame, geom, now_ms, false);
    }

    /// Same, over an already-blitted staged backdrop (A44): the
    /// loop's placeholder fill stays out and the staged scene set
    /// shows through (ADR-0003 keeps the fill for the no-archive case).
    pub fn paint_over_staged_backdrop(
        &mut self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        now_ms: u64,
    ) {
        self.paint_over(frame, geom, now_ms, true);
    }

    fn paint_over(
        &mut self,
        frame: &mut Frame<'_>,
        geom: &HighwayGeometry,
        now_ms: u64,
        staged_backdrop: bool,
    ) {
        self.advance_to(now_ms);
        if staged_backdrop {
            self.view
                .paint_curtained_over_staged(frame, geom, self.curtain);
        } else {
            self.view.paint_curtained(frame, geom, self.curtain);
        }
        self.paint_hold_tails(frame, geom);
        self.paint_hit_bursts(frame, geom, now_ms);
        self.paint_jam_flash(frame, geom, now_ms);
        self.paint_jam_gauge(frame);
        self.paint_panel_strip(frame);
        self.paint_judgement_word(frame, geom, now_ms);
    }
}

/// Native scene width the avatar layout scales from (800x600 canvas).
const SCENE_NATIVE_W: u32 = 800;
/// Native scene height the avatar layout scales from.
const SCENE_NATIVE_H: u32 = 600;
/// Solo avatar seat box on the native canvas: `Map_Avatar.bnd` rect0,
/// (307,161)-(607,461) — the exe-literal avatar-seat bind
/// (`docs/bnd-semantics.md`: the exe loads `Map_Avatar.bnd` by name).
/// The reference figure stands extent-centered in it.
pub(crate) const AVATAR_SEAT_NATIVE: Rect = Rect {
    x: 307,
    y: 161,
    w: 300,
    h: 300,
};
/// `Lv.<n> <name>` plate origin, seat-relative: the reference photo
/// shows the level box at (399,195) = seat + (92,34), above the life
/// gauge.
const AVATAR_LEVEL_OFFSET: (i32, i32) = (92, 34);
/// Life-gauge shell, seat-relative: the staged
/// `Avatar_Info_LifeGauge.ojs` 46x8 frame at (400,215) = seat +
/// (93,54) — the photo's amber bar directly above the figure's head.
const AVATAR_LIFE_OFFSET: (i32, i32) = (93, 54);
/// Life-gauge shell size: the staged `Avatar_Info_LifeGauge.ojs`
/// frame (archive Playing1.opi).
const AVATAR_LIFE_SIZE: (u32, u32) = (46, 8);
/// Tier-grid origins on the native canvas, photo-measured
/// (`COOL`/`BAD` top row, `GOOD`/`MISS` bottom row).
const TALLY_COUNT_GRID: [(i32, i32); 4] = [(317, 556), (392, 556), (317, 578), (392, 578)];
/// TIME progress bar on the native canvas: staged
/// `Normal_TimeBar.ojs` `(489,556)` 254x10 in the strip
/// (`Playing_Menu.ojs` `(204,558)` 596x42 is the strip panel).
///
/// Documented strip unpaints (verified absent — never invented):
/// - Play-time `MMSS` digits: the staged `PlayTimeNum.ojs` font has
///   no draw anchor — no `.bnd` rect pairs it (`State_Playing.bnd`'s
///   17 rects pair `EXIT`…`VOLUME` only; `State_Playing_network.bnd`'s
///   19 add `Winner_image` (265,472) / `Challenger_image` (410,472)),
///   the exe references the name only in its loader cluster (file
///   `0x24c0d8`), and the reference photo's strip carries no MMSS
///   ink — so the former bar-centered anchor (bnd-audit 31, claims
///   A32 ASSUMED) was invented and is removed, not re-anchored. The
///   font stays staged ([`PlaySkin::play_time_font`]).
/// - `BO` / `EA05` / `O2JAM` edge labels (photo ink (6,571)-(25,584)
///   and (241,575)-(262,596) plus the ⚡ badge (242,554)-(279,575)):
///   no 7.55 source exists — the rect walks above name no such
///   control, the strip panel art bakes only
///   VOL/EQUALIZER/COMBO/COOL/GOOD/BAD/MISS/EXIT, and the exe has no
///   draw string ("EA05" absent; "O2JAM" hits are FTP/server-list
///   context). The photo's labels are baked vintage strip art of a
///   different client build (`docs/parity.md` venue-vintage class);
///   painting them would inject ink 7.55 cannot produce.
const TALLY_TIME_NATIVE: Rect = Rect {
    x: 489,
    y: 556,
    w: 254,
    h: 10,
};
/// Strip speed label origin: the bound `IDC_TEXT_SPLAY_SPEED` rect
/// (487,579) 28x9 (`State_Playing.bnd` rect[15]; the photo's `X3.5`).
/// No-archive fallback only — the painter prefers the parsed
/// placement ([`PlayingBoundFields`]).
const TALLY_SPEED_NATIVE: (i32, i32) = (487, 579);
/// Strip song-title origin: the bound `IDC_TEXT_SPLAY_TITLE` rect
/// (520,579) 146x9 (`State_Playing.bnd` rect[6]). No-archive
/// fallback only (see above).
const TALLY_TITLE_NATIVE: (i32, i32) = (520, 579);
/// Strip volume origin: the bound `IDC_TEXT_SPLAY_VLOLUME` rect
/// (705,579) 44x9 (`State_Playing.bnd` rect[16]; the photo's
/// `VOL 58%`). No-archive fallback only (see above).
const TALLY_VOLUME_NATIVE: (i32, i32) = (705, 579);
/// EXIT readout on the native canvas: staged `Playing_Exit.ojs`
/// anchor `(751,554)` (archive slot 92, 42x42 button art).
/// No-archive fallback only (see above).
const TALLY_EXIT_NATIVE: (i32, i32) = (751, 554);
/// Control ids of the `EXIT`..`VOLUME` run (`State_Playing.bnd`
/// rects 1..=17). Lookup is by id, never by code: run codes
/// collide after the subset window (both `IDC_TEXT_SPLAY_LEVEL1`
/// and `IDC_TEXT_SPLAY_VLOLUME` carry ordinal 17), so only the
/// verified positional window (entries 213–229) pairs them.
const FIELD_EXIT_ID: &str = "IDC_BUTTON_SPLAY_EXIT";
const FIELD_CHAT_WINDOW_ID: &str = "IDC_IMAGE_SPLAY_CHATWINDOW";
const FIELD_TIMEBAR_ID: &str = "IDC_IMAGE_SPLAY_TIMEBAR";
const FIELD_BUFFER1_ID: &str = "IDC_IMAGE_SPLAY_BUFFER1";
const FIELD_BUFFER2_ID: &str = "IDC_IMAGE_SPLAY_BUFFER2";
const FIELD_CHAT_EDIT_ID: &str = "IDC_EDIT_SPLAY_CHAT";
const FIELD_TITLE_ID: &str = "IDC_TEXT_SPLAY_TITLE";
const FIELD_WAITUSER_TEXT_IDS: [&str; 7] = [
    "IDC_IMAGE_SPLAY_WAITUSER_TEXT1",
    "IDC_IMAGE_SPLAY_WAITUSER_TEXT2",
    "IDC_IMAGE_SPLAY_WAITUSER_TEXT3",
    "IDC_IMAGE_SPLAY_WAITUSER_TEXT4",
    "IDC_IMAGE_SPLAY_WAITUSER_TEXT5",
    "IDC_IMAGE_SPLAY_WAITUSER_TEXT6",
    "IDC_IMAGE_SPLAY_WAITUSER_TEXT7",
];
const FIELD_HOTTIME_ID: &str = "IDC_IMAGE_SPLAY_HOTTIME_ALRAM";
const FIELD_SPEED_ID: &str = "IDC_TEXT_SPLAY_SPEED";
const FIELD_VOLUME_ID: &str = "IDC_TEXT_SPLAY_VLOLUME";
/// `State_Playing.bnd` rects of the `EXIT`..`VOLUME` run, exactly as
/// parsed from 7.55 `Playing1.opi` (17 rects, corners `x1,y1`-`x2,y2`;
/// paired by the positional window entries 213–229, per
/// `docs/bnd-semantics.md`). This is the no-archive fallback table:
/// staged art always prefers the parsed placements
/// ([`PlayingBoundFields::from_placements`]), so these literals only
/// pin the art-less layout (audit-DONE pattern). No digit-font
/// control is in this run — the combo / word / long-note / jam /
/// score / play-time digits are unbound (see
/// [`COMBO_ROW_ABOVE_LINE`]), and the tier-count grid below is
/// photo-measured, not file geometry (OPEN, `docs/playing-reference.md`).
///
/// | # | corners | control |
/// |---|---|---|
/// | 1 | (751,554)-(793,596) | `IDC_BUTTON_SPLAY_EXIT` |
/// | 2 | (274,9)-(524,86) | `IDC_IMAGE_SPLAY_CHATWINDOW` |
/// | 3 | (489,556)-(743,566) | `IDC_IMAGE_SPLAY_TIMEBAR` |
/// | 4 | (200,3)-(228,31) | `IDC_IMAGE_SPLAY_BUFFER1` |
/// | 5 | (200,34)-(227,61) | `IDC_IMAGE_SPLAY_BUFFER2` |
/// | 6 | (270,87)-(518,97) | `IDC_EDIT_SPLAY_CHAT` |
/// | 7 | (520,579)-(666,588) | `IDC_TEXT_SPLAY_TITLE` |
/// | 8–14 | (660,11+15i)-(787,23+15i) | `IDC_IMAGE_SPLAY_WAITUSER_TEXT1..7` |
/// | 15 | (710,21)-(776,87) | `IDC_IMAGE_SPLAY_HOTTIME_ALRAM` |
/// | 16 | (487,579)-(515,588) | `IDC_TEXT_SPLAY_SPEED` |
/// | 17 | (705,579)-(749,588) | `IDC_TEXT_SPLAY_VLOLUME` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayingBoundFields {
    /// EXIT button rect (rect 1).
    pub exit: Rect,
    /// Chat window rect (rect 2).
    pub chat_window: Rect,
    /// TIME progress bar rect (rect 3).
    pub time_bar: Rect,
    /// BP buffer gauge rects (rects 4–5).
    pub buffer1: Rect,
    /// BP buffer gauge rects (rects 4–5).
    pub buffer2: Rect,
    /// Chat edit rect (rect 6).
    pub chat_edit: Rect,
    /// Song-title text rect (rect 7).
    pub title: Rect,
    /// Member text rects (rects 8–14). Sprite-less controls: the
    /// rects stage, the strings have no pinned wire source, so
    /// nothing paints them (OPEN strings — never invented).
    pub waituser_texts: [Rect; 7],
    /// HotTime alarm art rect (rect 15).
    pub hottime: Rect,
    /// Speed text rect (rect 16).
    pub speed: Rect,
    /// Volume text rect (rect 17).
    pub volume: Rect,
}

impl PlayingBoundFields {
    /// No-archive fallback: the parsed `State_Playing.bnd` rects
    /// verbatim (see the table above).
    #[must_use]
    pub fn fallback() -> Self {
        Self {
            exit: Rect {
                x: TALLY_EXIT_NATIVE.0,
                y: TALLY_EXIT_NATIVE.1,
                w: 42,
                h: 42,
            },
            chat_window: Rect {
                x: 274,
                y: 9,
                w: 250,
                h: 77,
            },
            time_bar: TALLY_TIME_NATIVE,
            buffer1: Rect {
                x: 200,
                y: 3,
                w: 28,
                h: 28,
            },
            buffer2: Rect {
                x: 200,
                y: 34,
                w: 27,
                h: 27,
            },
            chat_edit: Rect {
                x: 270,
                y: 87,
                w: 248,
                h: 10,
            },
            title: Rect {
                x: TALLY_TITLE_NATIVE.0,
                y: TALLY_TITLE_NATIVE.1,
                w: 146,
                h: 9,
            },
            waituser_texts: [
                Rect {
                    x: 660,
                    y: 11,
                    w: 127,
                    h: 12,
                },
                Rect {
                    x: 660,
                    y: 26,
                    w: 127,
                    h: 12,
                },
                Rect {
                    x: 660,
                    y: 41,
                    w: 127,
                    h: 12,
                },
                Rect {
                    x: 660,
                    y: 56,
                    w: 127,
                    h: 12,
                },
                Rect {
                    x: 660,
                    y: 71,
                    w: 127,
                    h: 12,
                },
                Rect {
                    x: 660,
                    y: 86,
                    w: 127,
                    h: 12,
                },
                Rect {
                    x: 660,
                    y: 101,
                    w: 127,
                    h: 12,
                },
            ],
            hottime: Rect {
                x: 710,
                y: 21,
                w: 66,
                h: 66,
            },
            speed: Rect {
                x: TALLY_SPEED_NATIVE.0,
                y: TALLY_SPEED_NATIVE.1,
                w: 28,
                h: 9,
            },
            volume: Rect {
                x: TALLY_VOLUME_NATIVE.0,
                y: TALLY_VOLUME_NATIVE.1,
                w: 44,
                h: 9,
            },
        }
    }

    /// Field rects read from staged placements: each field takes
    /// its placement's rect by control id when one staged (real
    /// area only — a zero-area placement keeps the fallback), else
    /// the fallback rect. Lookup is by id (codes collide in this
    /// run); file order is irrelevant.
    #[must_use]
    pub fn from_placements(placements: &[ScenePlacement]) -> Self {
        let fallback = Self::fallback();
        let mut waits = fallback.waituser_texts;
        for (slot, id) in waits.iter_mut().zip(FIELD_WAITUSER_TEXT_IDS) {
            *slot = placed_rect(placements, id, *slot);
        }
        Self {
            exit: placed_rect(placements, FIELD_EXIT_ID, fallback.exit),
            chat_window: placed_rect(placements, FIELD_CHAT_WINDOW_ID, fallback.chat_window),
            time_bar: placed_rect(placements, FIELD_TIMEBAR_ID, fallback.time_bar),
            buffer1: placed_rect(placements, FIELD_BUFFER1_ID, fallback.buffer1),
            buffer2: placed_rect(placements, FIELD_BUFFER2_ID, fallback.buffer2),
            chat_edit: placed_rect(placements, FIELD_CHAT_EDIT_ID, fallback.chat_edit),
            title: placed_rect(placements, FIELD_TITLE_ID, fallback.title),
            waituser_texts: waits,
            hottime: placed_rect(placements, FIELD_HOTTIME_ID, fallback.hottime),
            speed: placed_rect(placements, FIELD_SPEED_ID, fallback.speed),
            volume: placed_rect(placements, FIELD_VOLUME_ID, fallback.volume),
        }
    }
}

/// One field's staged rect, or `fallback` when no placement stages
/// it with a real area.
fn placed_rect(placements: &[ScenePlacement], id: &str, fallback: Rect) -> Rect {
    placements
        .iter()
        .find_map(|placement| {
            (placement.id == id && placement.rect.w > 0 && placement.rect.h > 0)
                .then_some(placement.rect)
        })
        .unwrap_or(fallback)
}
/// Caption ink over staged art (white on the scrim).
const AVATAR_TEXT: u32 = pack_xrgb(0xFF, 0xFF, 0xFF);
/// Scrim ink behind caption lines (matches the lobby scrims).
const AVATAR_SCRIM: u32 = pack_xrgb(0x08, 0x08, 0x10);
/// Life-bar empty shell (mirrors the peer bars).
const AVATAR_LIFE_EMPTY: u32 = pack_xrgb(0x30, 0x10, 0x10);
/// Life-bar fill: the staged `Avatar_Info_LifeGauge.ojs` amber frame
/// the reference photo shows (RGB 255,180,16, verified bytes).
const AVATAR_LIFE_FULL: u32 = pack_xrgb(255, 180, 16);
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

/// Blit origin that lands the staged long-note caption plate ink on
/// [`NATIVE_LONG_COMBO_PLATE`] at one frame size: the scaled box
/// origin minus the frame ink offset (the plate blits 1:1, like every
/// staged body, so the offset itself never scales).
#[must_use]
pub fn long_combo_plate_origin(frame_w: usize, frame_h: usize) -> (i32, i32) {
    (
        scale_scene_axis(NATIVE_LONG_COMBO_PLATE.x, frame_w, SCENE_NATIVE_W)
            - LONG_COMBO_PLATE_INK.0,
        scale_scene_axis(NATIVE_LONG_COMBO_PLATE.y, frame_h, SCENE_NATIVE_H)
            - LONG_COMBO_PLATE_INK.1,
    )
}

/// Scaled avatar-scene layout for one frame size: every origin and
/// bar the [`AvatarScene`] painter reads, so paint and tests share
/// one source. Right-side panel opens at native x 330 with the tally
/// strip at native y 540 (`docs/playing-reference.md`, photo-measured).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AvatarLayout {
    /// `Lv.<n> <name>` plate text origin.
    pub(crate) level_origin: (i32, i32),
    /// Life-gauge shell (46x8, above the figure head).
    pub(crate) life_bar: Rect,
    /// Solo avatar seat box: the worn figure extent-centers in it.
    pub(crate) seat: Rect,
    /// Tier-count origins: `COOL`, `BAD`, `GOOD`, `MISS` (grid row-
    /// major as painted).
    pub(crate) counts_origins: [(i32, i32); 4],
    /// TIME progress shell.
    pub(crate) time_bar: Rect,
    /// Strip speed label origin (`X3.5`).
    pub(crate) speed_origin: (i32, i32),
    /// Strip song-title origin.
    pub(crate) title_origin: (i32, i32),
    /// Strip volume origin (`VOL <n>`).
    pub(crate) volume_origin: (i32, i32),
    /// EXIT text origin.
    pub(crate) exit_origin: (i32, i32),
}

impl AvatarLayout {
    /// Scale the native avatar layout into the frame (identity on
    /// 800x600, proportional on headless probe frames); the bound
    /// strip fields use the no-archive fallbacks.
    #[must_use]
    pub(crate) fn for_frame(frame_w: usize, frame_h: usize) -> Self {
        Self::for_frame_placed(frame_w, frame_h, None)
    }

    /// [`for_frame`](Self::for_frame) with the bound strip fields
    /// read from staged placements ([`PlayingBoundFields`]): every
    /// TIMEBAR/TITLE/SPEED/VOLUME/EXIT origin comes from its parsed
    /// `.bnd` rect when staged (owner rule `f38f3`), else its
    /// fallback. Avatar origins and the photo-measured counts grid
    /// are shared verbatim.
    #[must_use]
    pub(crate) fn for_frame_placed(
        frame_w: usize,
        frame_h: usize,
        placements: Option<&[ScenePlacement]>,
    ) -> Self {
        let fields = placements.map_or_else(PlayingBoundFields::fallback, |staged| {
            PlayingBoundFields::from_placements(staged)
        });
        let seat = scale_scene_rect(&AVATAR_SEAT_NATIVE, frame_w, frame_h);
        Self {
            level_origin: scale_scene_origin(seat_offset(AVATAR_LEVEL_OFFSET), frame_w, frame_h),
            life_bar: Rect {
                x: scale_scene_axis(
                    AVATAR_SEAT_NATIVE.x + AVATAR_LIFE_OFFSET.0,
                    frame_w,
                    SCENE_NATIVE_W,
                ),
                y: scale_scene_axis(
                    AVATAR_SEAT_NATIVE.y + AVATAR_LIFE_OFFSET.1,
                    frame_h,
                    SCENE_NATIVE_H,
                ),
                w: scale_extent(AVATAR_LIFE_SIZE.0, frame_w, SCENE_NATIVE_W),
                h: scale_extent(AVATAR_LIFE_SIZE.1, frame_h, SCENE_NATIVE_H),
            },
            seat,
            counts_origins: TALLY_COUNT_GRID
                .map(|origin| scale_scene_origin(origin, frame_w, frame_h)),
            time_bar: scale_scene_rect(&fields.time_bar, frame_w, frame_h),
            speed_origin: scale_scene_origin((fields.speed.x, fields.speed.y), frame_w, frame_h),
            title_origin: scale_scene_origin((fields.title.x, fields.title.y), frame_w, frame_h),
            volume_origin: scale_scene_origin((fields.volume.x, fields.volume.y), frame_w, frame_h),
            exit_origin: scale_scene_origin((fields.exit.x, fields.exit.y), frame_w, frame_h),
        }
    }
}

/// Seat-relative origin as an absolute native origin.
fn seat_offset(offset: (i32, i32)) -> (i32, i32) {
    (
        AVATAR_SEAT_NATIVE.x + offset.0,
        AVATAR_SEAT_NATIVE.y + offset.1,
    )
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

    /// `VOL <n>` readout; `None` while unset.
    fn volume_line(&self) -> Option<String> {
        self.volume.map(|volume| format!("VOL {volume}"))
    }

    /// One line per tier, grid-paint order (`COOL`, `BAD`, `GOOD`,
    /// `MISS` — the photo's 2x2 strip grid).
    fn tier_lines(score: &Score) -> [String; 4] {
        [
            format!("COOL {}", score.counts[0]),
            format!("BAD {}", score.counts[2]),
            format!("GOOD {}", score.counts[1]),
            format!("MISS {}", score.counts[3]),
        ]
    }

    /// Paint the right-side seat scene plus bottom tally strip:
    /// plate + amber life gauge above the extent-centered figure,
    /// then counts grid, TIME bar, speed/title/volume lines, EXIT.
    /// Staged art stays underneath; a missing figure paints art
    /// without avatar.
    pub fn paint(
        &self,
        frame: &mut Frame<'_>,
        avatar: Option<&AvatarFigure>,
        score: &Score,
        now_ms: u64,
        song_end_ms: Option<u64>,
    ) {
        self.paint_layout(
            frame,
            avatar,
            score,
            now_ms,
            song_end_ms,
            AvatarLayout::for_frame(frame.w, frame.h),
        );
    }

    /// [`paint`](Self::paint) with the bound strip fields read from
    /// staged placements ([`PlayingBoundFields`]).
    pub fn paint_with_placements(
        &self,
        frame: &mut Frame<'_>,
        avatar: Option<&AvatarFigure>,
        score: &Score,
        now_ms: u64,
        song_end_ms: Option<u64>,
        placements: Option<&[ScenePlacement]>,
    ) {
        self.paint_layout(
            frame,
            avatar,
            score,
            now_ms,
            song_end_ms,
            AvatarLayout::for_frame_placed(frame.w, frame.h, placements),
        );
    }

    /// Shared scene paint over one layout (the fallback and
    /// placement-driven entries differ only in the strip origins).
    fn paint_layout(
        &self,
        frame: &mut Frame<'_>,
        avatar: Option<&AvatarFigure>,
        score: &Score,
        now_ms: u64,
        song_end_ms: Option<u64>,
        layout: AvatarLayout,
    ) {
        Self::paint_caption_line(frame, layout.level_origin, &self.player_line());
        Self::paint_life_bar(frame, &layout.life_bar, score.life.life());
        if let Some(figure) = avatar {
            figure.paint_extent_centered(frame, &layout.seat, frame.w, frame.h);
        }
        Self::paint_tally_strip(frame, &layout, score, now_ms, song_end_ms, self);
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

    /// Life-bar shell plus fill from the live [`LifeBar`] (VERIFIED
    /// 7.55 local life: per-difficulty deltas drain on Bad/Miss and
    /// fill on Cool/Good; the dead gate freezes it at 0 —
    /// `docs/gameplay.md`). The fill rises bottom-up like the peer
    /// bars over the same inks; the empty shell stays when dead.
    fn paint_life_bar(frame: &mut Frame<'_>, bar: &Rect, life: u16) {
        if bar.w == 0 || bar.h == 0 {
            return;
        }
        fill_rect(frame, bar, AVATAR_LIFE_EMPTY);
        let filled = u32::from(life.min(LIFE_MAX)) * bar.h / u32::from(LIFE_MAX);
        if filled > 0 {
            fill_rect(
                frame,
                &Rect {
                    x: bar.x,
                    y: bar.y + (bar.h - filled) as i32,
                    w: bar.w,
                    h: filled,
                },
                AVATAR_LIFE_FULL,
            );
        }
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

    /// Bottom tally strip: counts grid, TIME bar, speed + title +
    /// volume lines, EXIT. Counts always feed from the score (zeros
    /// included); speed/title paint while captioned; EXIT always
    /// marks the button.
    fn paint_tally_strip(
        frame: &mut Frame<'_>,
        layout: &AvatarLayout,
        score: &Score,
        now_ms: u64,
        song_end_ms: Option<u64>,
        scene: &Self,
    ) {
        for (origin, line) in layout.counts_origins.iter().zip(Self::tier_lines(score)) {
            Self::paint_caption_line(frame, *origin, &line);
        }
        Self::paint_time_bar(frame, &layout.time_bar, now_ms, song_end_ms);
        if let Some(speed) = scene.speed_code {
            Self::paint_caption_line(frame, layout.speed_origin, &speed_label_for(speed));
        }
        if !scene.song_title.is_empty() {
            Self::paint_caption_line(frame, layout.title_origin, &scene.song_title);
        }
        if let Some(volume) = scene.volume_line() {
            Self::paint_caption_line(frame, layout.volume_origin, &volume);
        }
        Self::paint_caption_line(frame, layout.exit_origin, "EXIT");
    }
}

#[cfg(test)]
mod live_play_tests {
    use super::*;
    use crate::avatar::PlacementRole;
    use o2jam_game::{HOLD_TICK_BONUS_POINTS, STAT_JAM, STAT_LIFE};
    use o2jam_render::{has_ink_near, near_xrgb};

    fn geom() -> HighwayGeometry {
        HighwayGeometry::centered(70, 60)
    }

    /// True while ink nears `ink` inside the `probe` box at `origin`
    /// (glyph-edge coverage blends into the backdrop, so probes band
    /// the ink instead of one exact pixel).
    fn ink_near(
        buf: &[u32],
        w: usize,
        h: usize,
        origin: (i32, i32),
        probe: (i32, i32),
        ink: u32,
    ) -> bool {
        let (ox, oy) = origin;
        let (pw, ph) = probe;
        for row in oy..oy + ph {
            for col in ox..ox + pw {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < h && c < w && near_xrgb(buf[r * w + c], ink) {
                    return true;
                }
            }
        }
        false
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
    fn life_difficulty_selects_the_verified_drain_rate() {
        let mut easy = LivePlay::new(&[(0, 1000, 7)]);
        easy.advance_to(1000 + 25 + 1);
        assert_eq!(easy.score().life.life(), LIFE_MAX - 50);
        let mut hard = LivePlay::new(&[(0, 1000, 7)]);
        hard.set_life_difficulty(2);
        hard.advance_to(1000 + 25 + 1);
        assert_eq!(hard.score().life.life(), LIFE_MAX - 30);
    }

    #[test]
    fn selected_life_deltas_refill_on_cool() {
        let mut live = LivePlay::new(&[(0, 1000, 7), (0, 2000, 7)]);
        live.set_life_difficulty(1); // Normal: Cool +2, Miss −40
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.score().life.life(), LIFE_MAX - 40);
        live.strike_lane(0, 2000);
        assert_eq!(live.score().life.life(), LIFE_MAX - 40 + 2);
    }

    #[test]
    fn unknown_life_difficulty_keeps_the_current_set() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.set_life_difficulty(3); // Rx: switch selects nothing (0x170fb4)
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.score().life.life(), LIFE_MAX - 50);
    }

    #[test]
    fn life_read_reports_the_painted_bar_word() {
        // Public Life-channel seam: starts full, drains on Miss,
        // refills on Cool — always the bar word itself, never a copy.
        let mut live = LivePlay::new(&[(0, 1000, 7), (0, 2000, 7)]);
        live.set_life_difficulty(1); // Normal: Miss -40, Cool +2
        assert_eq!(live.life(), LIFE_MAX);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.life(), LIFE_MAX - 40);
        live.strike_lane(0, 2000);
        assert_eq!(live.life(), LIFE_MAX - 40 + 2);
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
        // Score "200" centered over the staged bottom jam bar (probe
        // bar x 0..16 → center 8, strip row 55), combo "1" at the
        // bar's left edge, "COOL" centered over the highway (center 8).
        assert!(ink_near(
            &buf,
            70,
            60,
            (0, 53),
            (18, 8),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
        assert!(ink_near(
            &buf,
            70,
            60,
            (0, 52),
            (10, 10),
            pack_xrgb(0xFF, 0xFF, 0x40)
        ));
        assert!(ink_near(
            &buf,
            70,
            60,
            (0, 6),
            (18, 9),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
    }

    #[test]
    fn jam_fill_rows_scales_verified_progress_to_height() {
        assert_eq!(jam_fill_rows(0, 24), 0);
        assert_eq!(jam_fill_rows(2, 50), 2);
        assert_eq!(jam_fill_rows(1, 50), 1);
        assert_eq!(jam_fill_rows(50, 24), 24);
        assert_eq!(jam_fill_rows(60, 24), 24);
    }

    /// One row span of a synthetic `.oja` pixel plane:
    /// `[u16 width][u16 x][u16 y]` + `width` RGB555 words + the zero
    /// terminator word (the verified plane layout).
    fn oja_span(x: u16, y: u16, pixels: &[u16]) -> Vec<u8> {
        let mut span = Vec::new();
        span.extend_from_slice(&(pixels.len() as u16).to_le_bytes());
        span.extend_from_slice(&x.to_le_bytes());
        span.extend_from_slice(&y.to_le_bytes());
        for word in pixels {
            span.extend_from_slice(&word.to_le_bytes());
        }
        span.extend_from_slice(&0u16.to_le_bytes());
        span
    }

    /// One synthetic `.oja` frame spec: record geometry plus the
    /// pixel and alpha plane bytes.
    type OjaFrameSpec<'a> = ((i16, i16, u16, u16), &'a [u8], &'a [u8]);

    /// Synthetic `.oja` body: header + one 20-byte record per frame
    /// (`x y w h` plus the chained `a = a + b + c` offsets), then each
    /// frame's pixel plane and alpha plane (the `parse_oja55` layout).
    fn oja55_body(frames: &[OjaFrameSpec<'_>]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::OJA_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(frames.len() as u16).to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        let mut offset = 0u32;
        let mut records = Vec::new();
        for ((x, y, w, h), pixels, alpha) in frames {
            records.push((
                *x,
                *y,
                *w,
                *h,
                offset,
                pixels.len() as u32,
                alpha.len() as u32,
            ));
            offset += pixels.len() as u32 + alpha.len() as u32;
        }
        for (x, y, w, h, a, b, c) in records {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&y.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&a.to_le_bytes());
            blob.extend_from_slice(&b.to_le_bytes());
            blob.extend_from_slice(&c.to_le_bytes());
        }
        for (_, pixels, alpha) in frames {
            blob.extend_from_slice(pixels);
            blob.extend_from_slice(alpha);
        }
        blob
    }

    /// Two one-row `.oja` burst frames (stored order, distinct inks).
    fn burst_oja_body() -> Vec<u8> {
        let plane0 = oja_span(0, 0, &[0x7FE0, 0x7FE0]);
        let plane1 = oja_span(0, 0, &[0x03E0, 0x03E0]);
        oja55_body(&[
            ((-1, 0, 2, 1), &plane0, &[0xFF, 0xFF]),
            ((0, 0, 2, 1), &plane1, &[0x80, 0x80]),
        ])
    }

    #[test]
    fn note_click_oja_frames_stage_the_hit_burst() {
        // The `.oja` bare arm of `_Note_Click` wins the burst slot
        // over the stage-1 variant-1 `.ojt` sibling: two stored ring
        // frames, each plane pasted at its span onto the word-0-
        // cleared canvas (the alpha planes stay out until a blended
        // paint).
        let ojt = strided55_body(&[(0, 0, 1, 1); 8], 1, &[0x7C00; 8]);
        let archive = opi_blob(&[
            ("Note_Click.oja", &burst_oja_body()),
            ("Note_Click1_1.ojt", &ojt),
        ]);
        let skin = PlaySkin::from_archive(&archive).unwrap();
        assert_eq!(skin.hit_burst_frame_count(), 2);
        assert_eq!(
            skin.hit_burst_frame(0).unwrap().image.words,
            vec![0x7FE0, 0x7FE0]
        );
        assert_eq!(
            skin.hit_burst_frame(RECEPTOR_FRAME_MS).unwrap().image.words,
            vec![0x03E0, 0x03E0]
        );
    }

    /// Three one-pixel `.oja` flash frames (stored order, distinct
    /// inks) — the synthetic stand-in for the ten 300x300
    /// `Playing_Effect_Jam.oja` frames.
    fn jam_oja_body() -> Vec<u8> {
        let mut frames = Vec::new();
        for ink in [0x7FE0u16, 0x03E0, 0x001F] {
            let plane = oja_span(0, 0, &[ink]);
            frames.push(((0, 0, 1, 1), plane, vec![0xFFu8]));
        }
        let refs: Vec<OjaFrameSpec<'_>> = frames
            .iter()
            .map(|(geom, plane, alpha)| (*geom, plane.as_slice(), alpha.as_slice()))
            .collect();
        oja55_body(&refs)
    }

    #[test]
    fn playing_effect_jam_oja_frames_stage_the_jam_flash() {
        // The `.oja` bare arm of `_Playing_Effect_Jam` stages the jam
        // flash: three stored frames step one per
        // [`RECEPTOR_FRAME_MS`] and the flash ends after the staged
        // frame count (it plays once, never freezing on the last
        // frame). The body alone stages a skin.
        let archive = opi_blob(&[("Playing_Effect_Jam.oja", &jam_oja_body())]);
        let skin = PlaySkin::from_archive(&archive).unwrap();
        assert_eq!(skin.jam_flash_frame_count(), 3);
        assert_eq!(skin.jam_flash_frame(0).unwrap().image.words, vec![0x7FE0]);
        assert_eq!(
            skin.jam_flash_frame(RECEPTOR_FRAME_MS * 2)
                .unwrap()
                .image
                .words,
            vec![0x001F]
        );
        assert!(skin.jam_flash_frame(RECEPTOR_FRAME_MS * 3).is_none());
    }

    #[test]
    fn jam_level_grant_flashes_the_staged_jam_effect() {
        // A jam level grant (26 Cools cross the 50-progress level:
        // 25 fill it exactly, which the strict-greater grant does not
        // fire on) plays the staged flash centered on the highway at
        // the judgement line (highway center 8, line row 48 at 70x60):
        // the flash pixel covers the judgement line, and the flash
        // ends after the staged frame window.
        let notes: Vec<(u8, u64, u32)> = (0..26u32)
            .map(|i| (0, 1000 + u64::from(i) * 100, 7))
            .collect();
        let mut live = LivePlay::new(&notes);
        for i in 0..26u32 {
            assert_eq!(
                live.strike_lane(0, 1000 + u64::from(i) * 100),
                Some(Judgement::Cool)
            );
        }
        // The 26th Cool crosses the threshold: the grant stamps the
        // flash at the last strike (1000 + 25 * 100 = 3500).
        assert_eq!(live.score().jam.level(), 1);
        let mut baseline = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut baseline,
                w: 70,
                h: 60,
            },
            &geom(),
            3500,
        );
        assert_eq!(baseline[48 * 70 + 8], pack_xrgb(0xE0, 0xE0, 0xE0));
        let archive = opi_blob(&[("Playing_Effect_Jam.oja", &jam_oja_body())]);
        live.set_skin(PlaySkin::from_archive(&archive).unwrap());
        let mut flashed = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut flashed,
                w: 70,
                h: 60,
            },
            &geom(),
            3500,
        );
        assert_eq!(flashed[48 * 70 + 8], solid_ink(0x7FE0));
        let mut faded = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut faded,
                w: 70,
                h: 60,
            },
            &geom(),
            3500 + RECEPTOR_FRAME_MS * 3,
        );
        assert_eq!(faded[48 * 70 + 8], pack_xrgb(0xE0, 0xE0, 0xE0));
        assert!(!faded.contains(&solid_ink(0x03E0)));
    }

    /// Two nine-row `.oja` ring frames whose ink sits on the top row
    /// (stored order, distinct inks) — above the burst-slice rows and
    /// receptor pad that paint over a flashed lane's bottom band.
    fn ring_oja_body() -> Vec<u8> {
        let plane0 = oja_span(0, 0, &[0x7C00, 0x7C00]);
        let plane1 = oja_span(0, 0, &[0x03E0, 0x03E0]);
        oja55_body(&[
            ((-1, 0, 2, 9), &plane0, &[0xFF, 0xFF]),
            ((0, 0, 2, 9), &plane1, &[0x80, 0x80]),
        ])
    }

    #[test]
    fn long_note_effect_oja_rings_stage_the_hold_paint() {
        // The literal `LongNote_Effect.oja` stem stages the animated
        // hold rings: the held span reads the steady last frame while
        // no hold age exists, otherwise one ring step per
        // [`RECEPTOR_FRAME_MS`]. The static `LongNote_Effect.ojt`
        // frame stays the fallback body when the `.oja` is absent.
        let archive = opi_blob(&[("LongNote_Effect.oja", &ring_oja_body())]);
        let skin = PlaySkin::from_archive(&archive).unwrap();
        assert_eq!(skin.hold_ring_frame_count(), 2);
        assert_eq!(
            skin.hold_ring_frame(Some(0)).unwrap().image.words,
            vec![0x7C00, 0x7C00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            skin.hold_ring_frame(None).unwrap().image.words,
            vec![0x03E0, 0x03E0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let ojt = strided55_body(&[(0, 0, 1, 1); 2], 1, &[0x7C00, 0x001F]);
        let fallback = opi_blob(&[("LongNote_Effect.ojt", &ojt)]);
        let skin = PlaySkin::from_archive(&fallback).unwrap();
        assert_eq!(skin.hold_ring_frame_count(), 0);
        assert!(skin.hold_ring_frame(Some(0)).is_none());
        assert!(skin.long_effect().is_some());
    }

    #[test]
    fn held_note_paints_the_staged_ring_frames() {
        // The rings join the held span paint over the lane beams (the
        // exe's held lane light) and under nothing: each ring steps
        // per [`RECEPTOR_FRAME_MS`] from the hold strike, freezing on
        // the last frame, and the procedural tail core stays out. The
        // static `.ojt` effect never paints under staged rings.
        let ojt = strided55_body(&[(0, 0, 1, 1)], 1, &[0x001F]);
        let archive = opi_blob(&[
            ("LongNote_Effect.oja", &ring_oja_body()),
            ("LongNote_Effect.ojt", &ojt),
        ]);
        let mut live = LivePlay::new_holds(&[(3, 1500, 1900, 7)]);
        live.set_skin(PlaySkin::from_archive(&archive).unwrap());
        assert!(live.has_skin());
        live.strike_lane(3, 1500);
        let mut first = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut first,
                w: 70,
                h: 60,
            },
            &geom(),
            1500,
        );
        assert!(first.contains(&solid_ink(0x7C00)));
        assert!(!first.contains(&solid_ink(0x03E0)));
        assert!(!first.contains(&solid_ink(0x001F)));
        assert!(!first.contains(&TAIL_BEAM));
        let mut second = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut second,
                w: 70,
                h: 60,
            },
            &geom(),
            1500 + RECEPTOR_FRAME_MS,
        );
        assert!(second.contains(&solid_ink(0x03E0)));
        assert!(!second.contains(&solid_ink(0x7C00)));
        // A long hold freezes on the last ring (no flicker, no
        // restart) instead of replaying or collapsing to the tail.
        let mut steady = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut steady,
                w: 70,
                h: 60,
            },
            &geom(),
            1700,
        );
        assert!(steady.contains(&solid_ink(0x03E0)));
    }

    #[test]
    fn hit_burst_ojt_stages_when_the_oja_arm_is_absent_or_corrupt() {
        // The stage-numbered `.ojt` stays the fallback body: without
        // `Note_Click.oja` (or with one that fails to decode) the
        // eight-frame `.ojt` stages the burst.
        let ojt = strided55_body(&[(0, 0, 1, 1); 8], 1, &[0x7C00; 8]);
        let missing = opi_blob(&[("Note_Click1_1.ojt", &ojt)]);
        assert_eq!(
            PlaySkin::from_archive(&missing)
                .unwrap()
                .hit_burst_frame_count(),
            8
        );
        let corrupt = opi_blob(&[("Note_Click.oja", &[0u8; 8]), ("Note_Click1_1.ojt", &ojt)]);
        assert_eq!(
            PlaySkin::from_archive(&corrupt)
                .unwrap()
                .hit_burst_frame_count(),
            8
        );
    }

    #[test]
    fn note_click_oja_burst_bypasses_the_white_column() {
        // The staged `.oja` rings ride the existing burst slot: the
        // frame centers on the lane-3 pad (rows 44..48, so its center
        // row 46 carries the ink), the burst-slice rows above keep
        // the white column out, and the animation steps across the
        // flash frames.
        let archive = opi_blob(&[("Note_Click.oja", &burst_oja_body())]);
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.set_skin(PlaySkin::from_archive(&archive).unwrap());
        assert!(live.has_skin());
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
        assert!(first.contains(&solid_ink(0x7FE0)));
        assert_eq!(first[46 * 70 + 7], solid_ink(0x7FE0));
        assert_ne!(first[42 * 70 + 7], HIT_BURST);
        let mut second = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut second,
                w: 70,
                h: 60,
            },
            &geom(),
            1050,
        );
        assert!(second.contains(&solid_ink(0x03E0)));
        assert!(!second.contains(&solid_ink(0x7FE0)));
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
        // Native frame: interiors fill one row per 50/14 progress —
        // left bar interior x 24..228 rows 537..550, right 571..775.
        // Progress 4 fills the single bottom interior row (550); the
        // bottom bar interior (10 rows) shows no fill at progress 4
        // (4 * 10 / 50 = 0).
        let fill = pack_xrgb(0xFF, 0x80, 0x00);
        let shell = pack_xrgb(0x10, 0x10, 0x30);
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
        assert_eq!(buf[550 * 800 + 24], fill);
        assert_eq!(buf[550 * 800 + 571], fill);
        assert_eq!(buf[550 * 800 + 23], shell);
        assert_eq!(buf[551 * 800 + 23], shell);
        assert_eq!(buf[536 * 800 + 23], shell);
        assert_eq!(buf[546 * 800 + 5], shell);
    }

    #[test]
    fn jam_gauge_grant_advances_level_digits_and_empties_fill() {
        // 26 Cools: 25 fill the 50-progress level exactly (no grant
        // on the strict-greater compare), the 26th crosses it and
        // leaves 2 of in-level progress.
        let notes: Vec<(u8, u64, u32)> = (0..26u32)
            .map(|i| (0, 1000 + u64::from(i) * 100, 7))
            .collect();
        let mut live = LivePlay::new(&notes);
        for i in 0..26u32 {
            assert_eq!(
                live.strike_lane(0, 1000 + u64::from(i) * 100),
                Some(Judgement::Cool)
            );
        }
        assert_eq!(
            (live.score().jam.level(), live.score().jam.level_progress()),
            (1, 2)
        );
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &geom(),
            3500,
        );
        let jam_ink = pack_xrgb(0xFF, 0xC0, 0x40);
        // The lane span left of the level line stays flat; the line's
        // digits ink from x 35 on (the staged lane span ends at x 16,
        // the level line starts at x 35).
        assert_eq!(buf[4 * 70 + 20], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert!(ink_near(&buf, 70, 60, (33, 1), (8, 10), jam_ink));
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
    fn release_strike_rates_the_armed_tail_exact_cool() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        // The release judges the armed tail one-shot with the shared
        // windows: an exact release is Cool.
        assert_eq!(live.release_lane(2, 2000), Some(Judgement::Cool));
        assert_eq!(live.score().counts, [2, 0, 0, 0]);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn tail_judgement_is_one_shot_per_armed_cursor() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 2000), Some(Judgement::Cool));
        // The consumed cursor rates nothing on the next release.
        assert_eq!(live.release_lane(2, 2000), None);
        assert_eq!(live.score().counts, [2, 0, 0, 0]);
    }

    #[test]
    fn release_tiers_the_tail_with_the_shared_windows() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7), (2, 3000, 4000, 8)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 1990), Some(Judgement::Good)); // |−10|
        assert_eq!(live.strike_lane(2, 3000), Some(Judgement::Cool));
        assert_eq!(live.release_lane(2, 4020), Some(Judgement::Bad)); // |+20|
        assert_eq!(live.score().counts, [2, 1, 1, 0]);
    }

    #[test]
    fn early_release_misses_the_tail_and_bonus_keeps_ticking() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        // Release 500 ms before the tail: the shared windows are
        // symmetric, so |−500| ≥ 26 rates Miss (no "early = safe" rule).
        assert_eq!(live.release_lane(2, 1500), Some(Judgement::Miss));
        assert_eq!(live.score().counts, [1, 0, 0, 1]);
        assert_eq!(live.score().combo, 0);
        // The hold flag never clears mid-song: the armed span keeps
        // ticking its bonus (one tick from the release advance).
        assert_eq!(live.bonus().ticks(), 1);
        // The judged tail never rates a second Miss on expiry.
        live.advance_to(2000 + 25 + 1);
        assert_eq!(live.score().counts, [1, 0, 0, 1]);
    }

    #[test]
    fn held_tail_never_expires_and_late_release_misses() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        // The expiry scan is inert: advancing past the tail rates
        // nothing.
        live.advance_to(2000 + 25 + 1);
        assert_eq!(live.score().counts, [1, 0, 0, 0]);
        // The late release strike judges the armed tail instead:
        // |+26| ≥ 26 → Miss.
        assert_eq!(live.release_lane(2, 2000 + 25 + 1), Some(Judgement::Miss));
        assert_eq!(live.score().counts, [1, 0, 0, 1]);
        assert_eq!(live.score().combo, 0);
    }

    #[test]
    fn hold_missing_head_kills_the_tail() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        live.advance_to(1000 + 25 + 1);
        assert_eq!(live.release_lane(2, 2000), None);
        // The scan kill marks the tail judged without a second rating.
        assert_eq!(live.score().counts, [0, 0, 0, 1]);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn bad_head_strike_kills_the_paired_tail() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1020), Some(Judgement::Bad));
        // The Bad-head kill marks the tail judged, so it never arms
        // (no release judgement) and never rates a second Miss.
        assert_eq!(live.release_lane(2, 2000), None);
        assert_eq!(live.score().counts, [0, 0, 1, 0]);
        live.advance_to(2000 + 25 + 1);
        assert_eq!(live.score().counts, [0, 0, 1, 0]);
        assert_eq!(live.bonus().ticks(), 0);
        assert!(live.is_finished(3500));
    }

    #[test]
    fn keyup_strikes_nearest_record_then_judges_the_armed_tail() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7), (2, 1500, 1500, 9)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        // One key-up walks the lane's unjudged records (tails
        // excluded) and strikes the tap at 1500 (Good), then judges
        // the armed hold tail (|1490 − 2000| = 510 → Miss).
        assert_eq!(live.release_lane(2, 1490), Some(Judgement::Miss));
        assert_eq!(live.score().counts, [1, 1, 0, 1]);
        // The hold flag never clears mid-song: the armed span keeps
        // ticking its bonus through the key-up (one tick per advance).
        live.advance_to(1600);
        assert_eq!(live.bonus().ticks(), 2);
    }

    #[test]
    fn hold_bonus_ticks_only_while_armed_and_span_open() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        assert_eq!(live.strike_lane(2, 1000), Some(Judgement::Cool));
        // One +10 tick per advance inside the open span (strict ends).
        live.advance_to(1500);
        assert_eq!(
            (live.bonus().ticks(), live.bonus().points()),
            (1, HOLD_TICK_BONUS_POINTS)
        );
        live.advance_to(1999);
        assert_eq!(live.bonus().ticks(), 2);
        live.advance_to(2000);
        assert_eq!(live.bonus().ticks(), 2);
    }

    #[test]
    fn hold_bonus_needs_an_armed_hold() {
        let mut live = LivePlay::new_holds(&[(2, 1000, 2000, 7)]);
        live.advance_to(1010); // inside the span, head unstruck
        assert_eq!(live.bonus().ticks(), 0);
        live.advance_to(2000 + 25 + 1); // missed head kills the tail
        assert_eq!(live.bonus().ticks(), 0);
        assert_eq!(live.score().counts, [0, 0, 0, 1]);
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
    fn empty_peer_slots_paint_no_gauges() {
        // Reference photo: the solo room shows no peer gauges — zeroed
        // slots are unset peers, not empty shells to draw.
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        let mut buf = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &HighwayGeometry::centered(70, 60),
            0,
        );
        let empty_red = pack_xrgb(0x30, 0x10, 0x10);
        assert!(!buf.contains(&empty_red), "no seat paints an empty shell");
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
    fn seeded_holders_resolve_a_broadcast_onto_their_slots() {
        // Fixture seeding (the only non-4015 writer): the local seat
        // holds 1, the peer 2, empties the no-holder sentinel — the
        // broadcast then delivers positions 0/1 and leaves the rest
        // zeroed and hidden.
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.set_rank_holders(0xFF, [1, 2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let deliveries = live.apply_peer_ranks([1, 2, 0, 0, 0, 0, 0, 0]);
        assert_eq!(deliveries.len(), 2);
        assert_eq!(live.peers().rank_row(0), Some(1));
        assert_eq!(live.peers().rank_row(1), Some(2));
        assert_eq!(live.peers().rank_row(2), Some(0));
    }

    #[test]
    fn paint_shows_peer_life_fill_over_empty_bar() {
        let full_green = pack_xrgb(0x20, 0xE0, 0x20);
        let empty_red = pack_xrgb(0x30, 0x10, 0x10);
        // Slot 0 gauge: staged 46x8 bar scales to x 65..69, row 0
        // at 70x60 (flush top-right stack). An unset slot paints no
        // shell; a seated peer's shell appears on its first stat.
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
        live.paint_avatar_scene(
            &mut Frame {
                buf: &mut bare,
                w: 70,
                h: 60,
            },
            None,
            0,
        );
        assert_ne!(bare[65], empty_red);
        live.apply_peer_stats(0, STAT_LIFE, 0);
        let mut shell = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut shell,
                w: 70,
                h: 60,
            },
            &geom(),
            0,
        );
        live.paint_avatar_scene(
            &mut Frame {
                buf: &mut shell,
                w: 70,
                h: 60,
            },
            None,
            0,
        );
        assert_eq!(shell[65], empty_red);
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
        live.paint_avatar_scene(
            &mut Frame {
                buf: &mut filled,
                w: 70,
                h: 60,
            },
            None,
            0,
        );
        assert_eq!(filled[65], full_green);
    }

    #[test]
    fn peer_bars_paint_over_staged_venue_art() {
        // File-order rule: the peer gauges sit on the staged venue
        // art, so the over-art layer must cover it — slot 0 stays
        // green where the venue pixel overlaps the bar.
        use crate::avatar::{AvatarFigure, FigureLayer, ScenePlacement};
        use crate::screens::paint::{paint_playing_overlay, SceneArt};
        use o2jam_render::Rgb555Image;
        use std::collections::HashMap;
        let mut venue = AvatarFigure::default();
        venue.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 760,
            dy: 4,
            colorkey: 0xFFFF,
        });
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
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
        };
        let mut live = LivePlay::new(&[]);
        live.apply_peer_stats(0, STAT_LIFE, LIFE_MAX);
        let mut buf = vec![0u32; 800 * 600];
        let geom = HighwayGeometry::centered(800, 600);
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &geom,
            0,
        );
        paint_playing_overlay(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        live.paint_avatar_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            0,
        );
        assert_eq!(buf[4 * 800 + 760], pack_xrgb(0x20, 0xE0, 0x20));
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
        // The faded word leaves the flat backdrop: the word centers on
        // the highway (draw x 1 at 70 wide), whose gap column keeps
        // the backdrop after the 600 ms fade.
        assert_eq!(buf[8 * 70 + 3], pack_xrgb(0x0A, 0x0A, 0x0A));
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
    fn staged_burst_replaces_the_white_column() {
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.set_skin(PlaySkin::default().with_hit_bursts(vec![solid_sprite(10, 10, 0x7FE0)]));
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
        // The burst art centers on the lane-3 pad (rows 44..48, so its
        // center row 46 carries the ink); the burst-slice rows above it
        // keep the white column out (pure white still rides the score
        // and word ink elsewhere).
        assert!(buf.contains(&solid_ink(0x7FE0)));
        assert_eq!(buf[46 * 70 + 7], solid_ink(0x7FE0));
        assert_ne!(buf[42 * 70 + 7], HIT_BURST);
    }

    #[test]
    fn staged_burst_animates_across_flash_frames() {
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.set_skin(PlaySkin::default().with_hit_bursts(vec![
            solid_sprite(10, 10, 0x7FE0),
            solid_sprite(10, 10, 0x03E0),
        ]));
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
        assert!(first.contains(&solid_ink(0x7FE0)));
        assert!(!first.contains(&solid_ink(0x03E0)));
        let mut second = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut second,
                w: 70,
                h: 60,
            },
            &geom(),
            1050,
        );
        assert!(second.contains(&solid_ink(0x03E0)));
        assert!(!second.contains(&solid_ink(0x7FE0)));
    }

    #[test]
    fn held_lane_lights_the_staged_kit_beam_column() {
        // The exe's held lane light rides the `KeyEffect_7Key1_%d`
        // kit bodies (the reference photo's bright cyan column): a
        // held head paints the steady (last, fullest) frame instead
        // of the long-effect art or the procedural tail core.
        let mut live = LivePlay::new_holds(&[(3, 1500, 1900, 7)]);
        live.set_skin(
            PlaySkin::default()
                .with_lane_beams(
                    3,
                    vec![solid_sprite(8, 32, 0x02C0), solid_sprite(8, 32, 0x03FF)],
                )
                .with_long_effect(solid_sprite(8, 8, 0x7FE0)),
        );
        assert!(live.has_skin());
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
        assert!(!buf.contains(&solid_ink(0x02C0)));
        assert!(!buf.contains(&solid_ink(0x7FE0)));
        assert!(!buf.contains(&TAIL_BEAM));
    }

    #[test]
    fn press_flash_steps_the_kit_beam_frames() {
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.set_skin(PlaySkin::default().with_lane_beams(
            3,
            vec![solid_sprite(8, 32, 0x03FF), solid_sprite(8, 32, 0x001F)],
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
        assert!(first.contains(&solid_ink(0x03FF)));
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
        assert!(!second.contains(&solid_ink(0x03FF)));
    }

    #[test]
    fn kit_beams_take_priority_over_the_keyeffect_slice() {
        // The exe-cited kit beams replace the exe-unnamed
        // `Keyeffect.ojs` stand-in at the press-flash column.
        let mut live = LivePlay::new(&[(3, 1000, 7)]);
        live.set_skin(
            PlaySkin::default()
                .with_lane_beams(3, vec![solid_sprite(8, 32, 0x03FF)])
                .with_release_beam(3, solid_sprite(8, 32, 0x7C00)),
        );
        live.strike_lane(3, 1000);
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
        assert!(buf.contains(&solid_ink(0x03FF)));
        assert!(!buf.contains(&solid_ink(0x7C00)));
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
        // Word centers on the highway (center 8, draw x 1); combo
        // digits "1" beneath, centered on the same x.
        assert!(ink_near(
            &buf,
            70,
            60,
            (0, 6),
            (18, 9),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
        assert!(ink_near(&buf, 70, 60, (4, 22), (10, 10), amber));
    }

    /// Codec gap ①: the strided combo-glow atlas (`Note_Combo4/5/6.ojt`)
    /// stages a skin, so an archive carrying only the glow body is no
    /// longer "nothing staged" (the plain layout accounting rejects it).
    #[test]
    fn combo_glow_atlas_stages_a_skin() {
        let frames: Vec<u16> = (0..8u16).map(|frame| 0x7C00 + frame).collect();
        let body = strided55_body(&[(0, 0, 1, 1); 2], 4, &frames);
        let archive = opi_blob(&[("Note_Combo4.ojt", &body)]);
        assert!(PlaySkin::from_archive(&archive).is_some());
    }

    /// Every glow frame carries the parser's frame pixels in stored
    /// order (record-major), and a body that is not a strided atlas
    /// stages nothing (the digits keep painting).
    #[test]
    fn combo_glow_stages_parser_frames_in_stored_order() {
        let frames: Vec<u16> = (0..8u16).map(|frame| 0x7C00 + frame).collect();
        let body = strided55_body(&[(0, 0, 1, 1); 2], 4, &frames);
        let archive = opi_blob(&[("Note_Combo4.ojt", &body)]);
        let skin = PlaySkin::from_archive(&archive).unwrap();
        assert_eq!(skin.combo_glow_frame_count(), 8);
        assert_eq!(
            skin.combo_glow_frame(None).unwrap().image.words,
            vec![0x7C00]
        );
        assert_eq!(
            skin.combo_glow_frame(Some(0)).unwrap().image.words,
            vec![0x7C00]
        );
        assert_eq!(
            skin.combo_glow_frame(Some(RECEPTOR_FRAME_MS * 7))
                .unwrap()
                .image
                .words,
            vec![0x7C07]
        );
        // A stride-1 body is plain: no strided stride answers, the
        // glow stays unstaged, the skin is empty.
        let plain = strided55_body(&[(0, 0, 1, 1)], 1, &[0x7C00]);
        let plain_archive = opi_blob(&[("Note_Combo4.ojt", &plain)]);
        assert!(PlaySkin::from_archive(&plain_archive).is_none());
    }

    /// The glow body follows the room arena stage (the exe applier's
    /// stage-numbered arm); a stage without a strided atlas leaves the
    /// glow unstaged, and the stage-less pin stays
    /// [`COMBO_GLOW_SPRITE`].
    #[test]
    fn combo_glow_stage_routes_the_room_stage_body() {
        let body4 = strided55_body(&[(0, 0, 1, 1); 2], 4, &[0x7C00; 8]);
        let body5 = strided55_body(&[(0, 0, 1, 1); 2], 4, &[0x03E0; 8]);
        let body6 = strided55_body(&[(0, 0, 1, 1); 2], 4, &[0x001F; 8]);
        let archive = opi_blob(&[
            ("Note_Combo4.ojt", &body4),
            ("Note_Combo5.ojt", &body5),
            ("Note_Combo6.ojt", &body6),
        ]);
        for (stage, ink) in [(4, 0x7C00), (5, 0x03E0), (6, 0x001F)] {
            let skin = PlaySkin::from_archive_for_stage(&archive, Some(stage)).unwrap();
            assert_eq!(skin.combo_glow_frame(None).unwrap().image.words, vec![ink]);
        }
        assert!(PlaySkin::from_archive_for_stage(&archive, Some(1)).is_none());
        let stage_less = PlaySkin::from_archive(&archive).unwrap();
        assert_eq!(
            stage_less.combo_glow_frame(None).unwrap().image.words,
            vec![0x7C00]
        );
    }

    /// The glow band centers on the combo cluster point (center x,
    /// row `line_y - 24`): its top-left corner rides clear of the word
    /// and digits, the digits paint over the band, and the band steps
    /// one frame per [`RECEPTOR_FRAME_MS`].
    #[test]
    fn combo_glow_plays_behind_the_combo_digits() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        live.set_skin(
            PlaySkin::default()
                .with_combo_glow(vec![solid_sprite(8, 8, 0x7FE0), solid_sprite(8, 8, 0x001F)]),
        );
        let mut first = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut first,
                w: 70,
                h: 60,
            },
            &geom(),
            1005,
        );
        assert!(ink_near(
            &first,
            70,
            60,
            (0, 18),
            (18, 9),
            solid_ink(0x7FE0)
        ));
        assert!(ink_near(
            &first,
            70,
            60,
            (4, 22),
            (10, 10),
            pack_xrgb(0xFF, 0xFF, 0x40)
        ));
        let mut second = vec![0u32; 70 * 60];
        live.paint(
            &mut Frame {
                buf: &mut second,
                w: 70,
                h: 60,
            },
            &geom(),
            1100,
        );
        assert_eq!(second[20 * 70 + 4], solid_ink(0x001F));
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

    /// One synthetic number font: digit `d` paints a solid frame of
    /// `0x0001 << (d + 1)` (a distinct ink per digit, black colorkey).
    fn digit_font() -> DigitFont {
        let frames = (0..10u8)
            .map(|digit| solid_sprite(8, 4, 0x0001 << (digit + 1)))
            .collect();
        DigitFont::from_frames(frames).expect("ten frames bind one font")
    }

    /// Solid ink of synthetic digit `d` (see `digit_font`).
    fn digit_ink(digit: u8) -> u32 {
        solid_ink(0x0001 << (digit + 1))
    }

    /// Strided sprite-55 atlas body: one 20-byte record per entry,
    /// record `i` chained at data offset `i * w * h * 2`, and a blob of
    /// `records * stride` consecutive `w * h` RGB555 frames (the
    /// `Note_Combo4/5/6.ojt` layout).
    fn strided55_body(
        records: &[(i16, i16, u16, u16)],
        stride: usize,
        frame_words: &[u16],
    ) -> Vec<u8> {
        assert_eq!(records.len() * stride, frame_words.len());
        let frame_bytes = frame_words.len() * 2 / (records.len() * stride);
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(records.len() as u16).to_le_bytes());
        blob.extend_from_slice(&0x0000u16.to_le_bytes());
        for (index, (x, y, w, h)) in records.iter().enumerate() {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&y.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&((index * frame_bytes) as u32).to_le_bytes());
            blob.extend_from_slice(&(frame_bytes as u32).to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
        }
        for word in frame_words {
            blob.extend_from_slice(&word.to_le_bytes());
        }
        blob
    }

    /// Minimal `.opi` container: header + bodies + trailing entry
    /// table (`PlaySkin::from_archive` walks it read-only).
    fn opi_blob(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        use o2jam_file::{
            OPI_ENTRY_LEN, OPI_HEADER_LEN, OPI_NAME_OFF, OPI_OFFSET_OFF, OPI_RESERVED_OFF,
            OPI_SIZE_OFF,
        };
        let mut blob = vec![0u8; OPI_HEADER_LEN];
        blob[..4].copy_from_slice(&1u32.to_le_bytes());
        blob[4..8].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        let mut off = OPI_HEADER_LEN as u32;
        let mut table = Vec::new();
        for (name, body) in bodies {
            let mut entry = vec![0u8; OPI_ENTRY_LEN];
            entry[..4].copy_from_slice(&1u32.to_le_bytes());
            entry[OPI_NAME_OFF..OPI_NAME_OFF + name.len()].copy_from_slice(name.as_bytes());
            entry[OPI_OFFSET_OFF..OPI_OFFSET_OFF + 4].copy_from_slice(&off.to_le_bytes());
            entry[OPI_SIZE_OFF..OPI_SIZE_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            entry[OPI_RESERVED_OFF..OPI_RESERVED_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            table.push(entry);
            off += body.len() as u32;
        }
        for (_, body) in bodies {
            blob.extend_from_slice(body);
        }
        for entry in table {
            blob.extend_from_slice(&entry);
        }
        blob
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
    fn fonts_alone_stage_a_skin() {
        assert!(PlaySkin::default().is_empty());
        assert!(!PlaySkin::default().with_score_font(digit_font()).is_empty());
        assert!(!PlaySkin::default().with_combo_font(digit_font()).is_empty());
        assert!(!PlaySkin::default()
            .with_play_time_font(digit_font())
            .is_empty());
    }

    #[test]
    fn font_digits_read_values_and_padding() {
        assert_eq!(decimal_digits(0), vec![0]);
        assert_eq!(decimal_digits(205), vec![2, 0, 5]);
        assert_eq!(padded_digits(65, 4), vec![0, 0, 6, 5]);
        assert_eq!(padded_digits(1234, 2), vec![3, 4]);
        let font = digit_font();
        assert_eq!(font.height(), 4);
        assert_eq!(font.width_of(205), 24);
        assert_eq!(font.padded_width(65, 4), 32);
        assert!(DigitFont::from_frames(Vec::new()).is_none());
        assert!(font.digit(10).is_none());
    }

    #[test]
    fn staged_score_font_replaces_strip_digits() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000); // Cool: 200 points → "200"
        live.set_skin(PlaySkin::default().with_score_font(digit_font()));
        let mut buf = vec![0u32; 800 * 600];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &HighwayGeometry::centered(800, 600),
            1400,
        );
        // Score "200" centered on the bottom jam bar's center x 99:
        // 24 px wide, so digit 2 opens at 87, the zeros at 95/103,
        // row 554 (the staged strip row).
        assert_eq!(buf[554 * 800 + 87], digit_ink(2));
        assert_eq!(buf[554 * 800 + 95], digit_ink(0));
        assert_eq!(buf[554 * 800 + 103], digit_ink(0));
        // The strip combo keeps its procedural amber glyph: the TTF
        // rasterizes it with antialiased coverage near the bar's
        // left edge (x 0..16 on the strip row).
        assert!(ink_near(
            &buf,
            800,
            600,
            (0, 552),
            (16, 16),
            pack_xrgb(0xFF, 0xFF, 0x40)
        ));
    }

    #[test]
    fn staged_combo_font_replaces_judgement_digits() {
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000); // combo 1 → "1"
        live.set_skin(PlaySkin::default().with_combo_font(digit_font()));
        let mut buf = vec![0u32; 800 * 600];
        live.paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &HighwayGeometry::centered(800, 600),
            1050,
        );
        // Combo "1" centered on the highway center x 99: 8 px wide →
        // x 95..103 on rows 463..467 (line_y − 24), the staged
        // sprite blit keeps exact pixels.
        assert_eq!(buf[465 * 800 + 97], digit_ink(1));
        // The strip combo keeps its procedural amber glyph: the TTF
        // rasterizes it with antialiased coverage around (5,554).
        assert!(
            (0..16)
                .flat_map(|row| (0..16).map(move |col| (col, row)))
                .any(|(col, row)| near_xrgb(
                    buf[(548 + row) * 800 + col],
                    pack_xrgb(0xFF, 0xFF, 0x40)
                )),
            "strip combo amber must ink near (0,548)"
        );
    }

    #[test]
    fn staged_play_time_font_paints_nothing_without_a_verified_anchor() {
        // bnd-audit 31's bar-centered MMSS anchor was invented: no
        // `.bnd` rect pairs `PlayTimeNum.ojs` (17 + 19 rect walk), the
        // exe names it only in the loader cluster (file `0x24c0d8`),
        // and the reference strip shows no MMSS ink. The staged font
        // therefore paints nothing — the TIME bar's shell art alone.
        let mut live = LivePlay::default();
        let mut buf = vec![0u32; 800 * 600];
        let plain = {
            let mut frame = Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            };
            live.paint_avatar_scene(&mut frame, None, 65_000);
            buf.clone()
        };
        live.set_skin(PlaySkin::default().with_play_time_font(digit_font()));
        let mut staged = vec![0u32; 800 * 600];
        live.paint_avatar_scene(
            &mut Frame {
                buf: &mut staged,
                w: 800,
                h: 600,
            },
            None,
            65_000,
        );
        assert_eq!(
            plain, staged,
            "staging PlayTimeNum must not add MMSS ink at an invented anchor"
        );
    }

    #[test]
    fn hud_digits_keep_procedural_fallback_without_fonts() {
        // No fonts staged: every HUD digit keeps its procedural ink
        // and the TIME bar keeps its shell (nothing is ever blank).
        let mut live = LivePlay::new(&[(0, 1000, 7)]);
        live.strike_lane(0, 1000);
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        live.paint(&mut frame, &HighwayGeometry::centered(800, 600), 1050);
        live.paint_avatar_scene(&mut frame, None, 1050);
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF))); // score + word
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0x40))); // combo
        assert!(buf.contains(&pack_xrgb(0x20, 0x20, 0x20))); // TIME shell
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
    fn from_archive_binds_the_hud_number_fonts() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Playing1.opi");
        let Ok(archive) = std::fs::read(&path) else {
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            panic!("Playing1.opi staged no skin art");
        };
        // Verified 7.55 metrics (archive slots 557 / 96 / 44): ten raw
        // digit frames each, ~11x21 score, 27x21 play time, combo up
        // to 33x51.
        let score = skin.score_font().expect("Scroe.ojs font");
        assert_eq!(score.digit(0).expect("digit frame").extent(), (11, 21));
        let time = skin.play_time_font().expect("PlayTimeNum.ojs font");
        assert_eq!(time.digit(0).expect("digit frame").extent(), (27, 21));
        let combo = skin.combo_font().expect("Note_ComboNum.ojs font");
        for digit in 0..10u8 {
            let (w, h) = combo.digit(digit).expect("digit frame").extent();
            assert!(w <= 33 && h <= 51, "combo digit {digit} {w}x{h}");
        }
        assert_eq!(combo.digit(9).expect("digit frame").extent(), (31, 51));
    }

    #[test]
    fn from_archive_binds_the_note_click_burst() {
        let Some(skin) = staged_skin() else {
            println!("skip: missing Playing1.opi");
            return;
        };
        // The `.oja` bare arm of `_Note_Click` stages the burst
        // (verified 7.55 `Note_Click.oja`: six shrinking ring frames
        // 144x143..39x39, centred negative anchors; the stage-1
        // variant-1 `.ojt` stays the fallback body).
        assert_eq!(skin.hit_burst_frame_count(), 6);
        let first = skin.hit_burst_frame(0).expect("first burst frame");
        assert_eq!(first.extent(), (144, 143));
    }

    #[test]
    fn from_archive_binds_the_jam_flash_and_hold_rings() {
        let Some(skin) = staged_skin() else {
            println!("skip: missing Playing1.opi");
            return;
        };
        // Verified 7.55 `Playing_Effect_Jam.oja`: ten 300x300 flash
        // frames at anchor (0, 0). `LongNote_Effect.oja` (byte-
        // identical to `Note_Click.oja`): six shrinking ring frames.
        assert_eq!(skin.jam_flash_frame_count(), 10);
        assert_eq!(
            skin.jam_flash_frame(0).expect("jam frame").extent(),
            (300, 300)
        );
        assert_eq!(skin.hold_ring_frame_count(), 6);
        assert_eq!(
            skin.hold_ring_frame(Some(0)).expect("ring frame").extent(),
            (144, 143)
        );
    }

    #[test]
    fn from_archive_binds_the_kit_lane_beams() {
        let Some(skin) = staged_skin() else {
            println!("skip: missing Playing1.opi");
            return;
        };
        // Verified 7.55 `KeyEffect_7Key1_1..7.ojs` (kit 1 pairs
        // `Playing_NormalBG1.ojs`): five raw full-column frames per
        // lane, widths 29/23/29/36/29/23/29 at 480 rows.
        let widths = [29, 23, 29, 36, 29, 23, 29];
        for lane in 0..HIGHWAY_LANES {
            let beam = skin.lane_beam(lane).expect("kit beam per lane");
            assert_eq!(beam.extent(), (widths[lane as usize], 480));
        }
        assert_eq!(
            skin.lane_beam_frame(1, Some(0)).map(|b| b.extent()),
            Some((23, 480))
        );
    }

    /// Read the pinned archive and stage the full skin (`None` test
    /// skip when the git-ignored game data is absent).
    fn staged_skin() -> Option<PlaySkin> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Playing1.opi");
        let archive = std::fs::read(&path).ok()?;
        PlaySkin::from_archive(&archive)
    }

    /// Paint one skin's over-art layer (avatar scene + captions) alone
    /// on a blank native frame at the playing shot's instant.
    fn paint_over_art_layer(skin: &PlaySkin) -> Vec<u32> {
        let mut live = LivePlay::new(&[]);
        live.set_skin(skin.clone());
        let mut buf = vec![0u32; 800 * 600];
        live.paint_avatar_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            5_100,
        );
        buf
    }

    #[test]
    fn play_time_digits_paint_nowhere_without_a_verified_anchor() {
        // bnd-audit 31 / claims A32 carried the MMSS paint on an
        // assumed bar-centered anchor. Ground truth: no `.bnd` rect
        // pairs `PlayTimeNum.ojs` (17-rect `State_Playing.bnd` and
        // 19-rect `State_Playing_network.bnd` walks cover `EXIT`…
        // `VOLUME` plus `Winner_image`/`Challenger_image` only), the
        // exe names it just in the loader cluster (file `0x24c0d8`),
        // and the reference strip shows no MMSS ink — so the staged
        // font must not paint at all. Skins differing only in the
        // play-time font must paint identical frames.
        let Some(skin) = staged_skin() else {
            println!("skip: missing Playing1.opi");
            return;
        };
        assert!(skin.play_time_font().is_some(), "font stages");
        let mut fontless = skin.clone();
        fontless.play_time_font = None;
        assert_eq!(
            paint_over_art_layer(&skin),
            paint_over_art_layer(&fontless),
            "staged PlayTimeNum must not paint at an invented anchor"
        );
    }

    #[test]
    fn long_combo_caption_paints_the_staged_plate_on_the_photo_box() {
        // The caption stages from the runtime body (`Note_LongCombo.ojt`
        // frame 1, clean 74x9 plate ink at frame offset (52,61)) and
        // blits 1:1 so the ink lands on the photo-pinned box
        // (91,282)-(165,291): every plate ink pixel of a lone blit must
        // appear unchanged in the over-art layer's frame.
        let Some(skin) = staged_skin() else {
            println!("skip: missing Playing1.opi");
            return;
        };
        let plate = skin.long_combo_plate().expect("Note_LongCombo.ojt plate");
        assert_eq!(plate.extent(), (128, 128));
        let (origin_x, origin_y) = long_combo_plate_origin(800, 600);
        assert_eq!((origin_x, origin_y), (39, 221));
        const BACKDROP: u32 = 0x020202;
        let mut expected = vec![BACKDROP; 800 * 600];
        plate.paint_at(
            &mut Frame {
                buf: &mut expected,
                w: 800,
                h: 600,
            },
            origin_x,
            origin_y,
        );
        let actual = paint_over_art_layer(&skin);
        let mut checked = 0;
        for at in 0..expected.len() {
            if expected[at] != BACKDROP {
                assert_eq!(
                    actual[at], expected[at],
                    "plate pixel {at} must land on the photo box"
                );
                checked += 1;
            }
        }
        assert!(checked >= 400, "plate ink covered only {checked} px");
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
        assert_eq!(jam_bar_rects(800, 600)[0], NATIVE_JAM_BARS[0]);
        assert_eq!(jam_bar_rects(800, 600)[1], NATIVE_JAM_BARS[1]);
        assert_eq!(jam_bar_rects(800, 600)[2], NATIVE_JAM_BOTTOM_BAR);
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
        assert_eq!(
            jam_bar_rects(70, 60)[2],
            Rect {
                x: 0,
                y: 53,
                w: 16,
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
    use crate::avatar::PlacementRole;
    use o2jam_render::near_xrgb;

    const WHITE: u32 = pack_xrgb(0xFF, 0xFF, 0xFF);
    /// The staged `Avatar_Info_LifeGauge.ojs` amber the photo shows.
    const LIFE_FILL: u32 = AVATAR_LIFE_FULL;
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
        ink_near(buf, w, h, origin, (64, 16), WHITE)
    }

    /// True while ink nears `ink` inside the `probe` box at `origin`
    /// (glyph-edge coverage blends into the backdrop, so probes band
    /// the ink instead of one exact pixel).
    fn ink_near(
        buf: &[u32],
        w: usize,
        h: usize,
        origin: (i32, i32),
        probe: (i32, i32),
        ink: u32,
    ) -> bool {
        let (ox, oy) = origin;
        let (pw, ph) = probe;
        for row in oy..oy + ph {
            for col in ox..ox + pw {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < h && c < w && near_xrgb(buf[r * w + c], ink) {
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
        paint_score_scene(scene, avatar, &Score::default(), w, h)
    }

    /// Paint with an explicit score (drives the live life-bar fill).
    fn paint_score_scene(
        scene: &AvatarScene,
        avatar: Option<&AvatarFigure>,
        score: &Score,
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
            score,
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
        // The strip's speed and title slots caption independently;
        // the combined display state stays inspectable.
        let scene = captioned_scene();
        assert_eq!(scene.speed_code, Some(6));
        assert_eq!(scene.song_title, "First");
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
    fn tier_lines_feed_the_grid_order() {
        let score = Score {
            counts: [1, 2, 3, 4],
            ..Default::default()
        };
        let lines = AvatarScene::tier_lines(&score);
        assert_eq!(lines, ["COOL 1", "BAD 3", "GOOD 2", "MISS 4"]);
        assert_eq!(
            AvatarScene::tier_lines(&Score::default()),
            ["COOL 0", "BAD 0", "GOOD 0", "MISS 0"]
        );
    }

    #[test]
    fn layout_scales_proportionally() {
        let native = AvatarLayout::for_frame(800, 600);
        assert_eq!(native.level_origin, (399, 195));
        assert_eq!(
            native.life_bar,
            Rect {
                x: 400,
                y: 215,
                w: 46,
                h: 8
            }
        );
        assert_eq!(
            native.seat,
            Rect {
                x: 307,
                y: 161,
                w: 300,
                h: 300
            }
        );
        let probe = AvatarLayout::for_frame(70, 60);
        assert_eq!(probe.level_origin, (34, 19));
    }

    #[test]
    fn figure_extents_centered_in_the_avatar_seat() {
        // Worn figures carry avatar.opa canvas anchors, so the seat
        // painter centers the anchored-extent box, not the first
        // layer: a 1x1 red pixel anchored (40,30) has extent box
        // (40,30)-(41,31), centered in the 300x300 seat at
        // (436,295), and the pixel lands at its anchor offset —
        // (476,325) on the native canvas.
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 40,
            dy: 30,
            colorkey: 0xFFFF,
        });
        let buf = paint_scene(&captioned_scene(), Some(&figure), 800, 600);
        assert_eq!(buf[325 * 800 + 476], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn seat_plate_and_life_gauge_paint_above_the_figure_head() {
        // Measured on the reference photo: the level plate at seat
        // + (92,34) reads above the amber life gauge (46x8, the
        // staged `Avatar_Info_LifeGauge.ojs` frame) at seat + (93,54)
        // — (399,195) and (400,215) native — directly above the
        // figure's head.
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), None, 800, 600);
        assert!(white_near(&buf, 800, 600, layout.level_origin));
        // Full default life: the whole 46x8 gauge shell fills amber.
        assert_eq!(buf[215 * 800 + 400], LIFE_FILL);
        assert_eq!(buf[222 * 800 + 445], LIFE_FILL);
    }

    #[test]
    fn tally_strip_paints_the_measured_grid() {
        // Bottom strip on the reference photo: counts as the 2x2
        // tier grid at (317,556)/(392,556)/(317,578)/(392,578), the
        // TIME bar at the verified (489,556), speed + song title at
        // the bound text rects (487,579)/(520,579), volume at
        // (705,579), EXIT at the staged button (751,554).
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), None, 800, 600);
        assert!(white_near(&buf, 800, 600, layout.counts_origins[0]));
        assert!(white_near(&buf, 800, 600, layout.counts_origins[1]));
        assert!(white_near(&buf, 800, 600, layout.counts_origins[2]));
        assert!(white_near(&buf, 800, 600, layout.counts_origins[3]));
        assert!(white_near(&buf, 800, 600, layout.speed_origin));
        assert!(white_near(&buf, 800, 600, layout.title_origin));
        assert!(white_near(&buf, 800, 600, layout.volume_origin));
        assert!(white_near(&buf, 800, 600, layout.exit_origin));
        assert_eq!(buf[556 * 800 + 489], TIME_EMPTY);
    }

    /// One staged bound-run placement (id + native rect).
    fn field_placement(id: &str, rect: Rect) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code: 0,
            sprite: None,
            rect,
        }
    }

    #[test]
    fn fallback_fields_equal_the_parsed_playing_bind_rects() {
        // The `State_Playing.bnd` 17-rect walk (7.55 `Playing1.opi`;
        // rect # + corners in the [`PlayingBoundFields`] table).
        let fields = PlayingBoundFields::fallback();
        assert_eq!(
            fields.exit,
            Rect {
                x: 751,
                y: 554,
                w: 42,
                h: 42
            }
        );
        assert_eq!(
            fields.chat_window,
            Rect {
                x: 274,
                y: 9,
                w: 250,
                h: 77
            }
        );
        assert_eq!(
            fields.time_bar,
            Rect {
                x: 489,
                y: 556,
                w: 254,
                h: 10
            }
        );
        assert_eq!(
            fields.buffer1,
            Rect {
                x: 200,
                y: 3,
                w: 28,
                h: 28
            }
        );
        assert_eq!(
            fields.buffer2,
            Rect {
                x: 200,
                y: 34,
                w: 27,
                h: 27
            }
        );
        assert_eq!(
            fields.chat_edit,
            Rect {
                x: 270,
                y: 87,
                w: 248,
                h: 10
            }
        );
        assert_eq!(
            fields.title,
            Rect {
                x: 520,
                y: 579,
                w: 146,
                h: 9
            }
        );
        assert_eq!(
            fields.waituser_texts[0],
            Rect {
                x: 660,
                y: 11,
                w: 127,
                h: 12
            }
        );
        assert_eq!(
            fields.waituser_texts[6],
            Rect {
                x: 660,
                y: 101,
                w: 127,
                h: 12
            }
        );
        assert_eq!(
            fields.hottime,
            Rect {
                x: 710,
                y: 21,
                w: 66,
                h: 66
            }
        );
        assert_eq!(
            fields.speed,
            Rect {
                x: 487,
                y: 579,
                w: 28,
                h: 9
            }
        );
        assert_eq!(
            fields.volume,
            Rect {
                x: 705,
                y: 579,
                w: 44,
                h: 9
            }
        );
    }

    #[test]
    fn staged_bounds_win_and_zero_area_keeps_the_fallback() {
        // Owner rule `f38f3`: staged rects own layout; a zero-area
        // placement (ordinal miss) keeps the fallback and must not
        // collapse the field.
        let title_rect = Rect {
            x: 100,
            y: 100,
            w: 50,
            h: 10,
        };
        let staged = vec![
            field_placement(FIELD_TITLE_ID, title_rect),
            field_placement(
                FIELD_SPEED_ID,
                Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
            ),
        ];
        let fields = PlayingBoundFields::from_placements(&staged);
        assert_eq!(fields.title, title_rect);
        assert_eq!(fields.speed, PlayingBoundFields::fallback().speed);
        // Lookup is by id, so file order is irrelevant.
        let mut reversed = staged.clone();
        reversed.reverse();
        assert_eq!(PlayingBoundFields::from_placements(&reversed), fields);
        // An empty stage keeps every fallback.
        assert_eq!(
            PlayingBoundFields::from_placements(&[]),
            PlayingBoundFields::fallback()
        );
    }

    #[test]
    fn staged_layout_matches_the_parsed_bind_origins() {
        // A fully staged run (parsed rects verbatim) lays out the
        // native fallback origins: placement and fallback agree.
        let fallback = PlayingBoundFields::fallback();
        let staged = vec![
            field_placement(FIELD_EXIT_ID, fallback.exit),
            field_placement(FIELD_TIMEBAR_ID, fallback.time_bar),
            field_placement(FIELD_TITLE_ID, fallback.title),
            field_placement(FIELD_SPEED_ID, fallback.speed),
            field_placement(FIELD_VOLUME_ID, fallback.volume),
        ];
        let staged_layout = AvatarLayout::for_frame_placed(800, 600, Some(&staged));
        let plain_layout = AvatarLayout::for_frame(800, 600);
        assert_eq!(staged_layout, plain_layout);
    }

    #[test]
    fn scene_paints_no_captions_over_the_stage_deck() {
        // The photo's stage deck carries no song or volume ink — the
        // song line and VOL readout live in the bottom strip, so the
        // scene positions are gone from the layout.
        let scene = captioned_scene();
        assert_eq!(scene.speed_code, Some(6));
        assert_eq!(scene.volume_line(), Some("VOL 5".to_owned()));
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
        assert!(!white_near(&buf, 800, 600, layout.speed_origin));
        assert!(!white_near(&buf, 800, 600, layout.title_origin));
        assert!(!white_near(&buf, 800, 600, layout.volume_origin));
    }

    #[test]
    fn life_bar_paints_full_fill() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&AvatarScene::default(), None, 800, 600);
        assert_eq!(buf[215 * 800 + 400], LIFE_FILL);
        assert_eq!(
            layout.life_bar,
            Rect {
                x: 400,
                y: 215,
                w: 46,
                h: 8
            }
        );
    }

    #[test]
    fn life_bar_fill_tracks_live_life() {
        let mut score = Score::default();
        score.apply(Judgement::Miss); // Easy miss: 1000 - 50
        let buf = paint_score_scene(&AvatarScene::default(), None, &score, 800, 600);
        // 950/1000 fills 7 of the 8 shell rows: the top row empties.
        assert_eq!(buf[215 * 800 + 400], AVATAR_LIFE_EMPTY);
        assert_eq!(buf[216 * 800 + 400], LIFE_FILL);
    }

    #[test]
    fn dead_gate_freezes_live_scoring_after_the_bar_empties() {
        // Easy bar: 20 Misses empty it (the killing Miss still lands —
        // the scorer entry gate); the 21st strike rates Cool on the
        // windows but the dead gate freezes counts/points/life.
        let notes: Vec<(u8, u64, u32)> = (0..21u32)
            .map(|i| (0, 1000 + u64::from(i) * 100, 7))
            .collect();
        let mut live = LivePlay::new(&notes);
        for i in 0..20u32 {
            live.advance_to(1000 + u64::from(i) * 100 + 26);
        }
        assert!(live.score().life.is_dead());
        assert_eq!(live.score().counts, [0, 0, 0, 20]);
        assert_eq!(live.strike_lane(0, 1000 + 20 * 100), Some(Judgement::Cool));
        assert_eq!(live.score().counts, [0, 0, 0, 20]);
        assert_eq!(live.life(), 0);
        assert_eq!(live.score().points, 0);
    }

    #[test]
    fn dead_life_bar_freezes_at_the_empty_shell() {
        let mut score = Score::default();
        for _ in 0..20 {
            score.apply(Judgement::Miss); // Easy: 20 misses kill the bar
        }
        score.apply(Judgement::Cool); // dead gate freezes the bar at 0
        assert!(score.life.is_dead());
        let buf = paint_score_scene(&AvatarScene::default(), None, &score, 800, 600);
        let bar = AvatarLayout::for_frame(800, 600).life_bar;
        for row in bar.y..bar.y + bar.h as i32 {
            assert_eq!(buf[row as usize * 800 + bar.x as usize], AVATAR_LIFE_EMPTY);
        }
    }

    #[test]
    fn figure_paints_extent_centered_in_the_seat() {
        let figure = red_figure();
        let buf = paint_scene(&captioned_scene(), Some(&figure), 800, 600);
        // 1x1 figure anchored (0,0): extent box centers at (456,310).
        assert_eq!(buf[310 * 800 + 456], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn missing_figure_paints_art_without_avatar() {
        let layout = AvatarLayout::for_frame(800, 600);
        let buf = paint_scene(&captioned_scene(), None, 800, 600);
        assert_eq!(buf[310 * 800 + 456], 0);
        let (x, y) = layout.level_origin;
        assert!(
            white_near(&buf, 800, 600, (x, y)),
            "level line must ink near its origin"
        );
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
        assert_eq!(start[556 * 800 + 489], TIME_EMPTY);
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
        assert_eq!(end[556 * 800 + 489], TIME_FILL);
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
        assert_eq!(unknown[556 * 800 + 489], TIME_EMPTY);
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
        assert!(white_near(&buf, 800, 600, layout.counts_origins[0]));
        assert_eq!(LivePlay::default().song_end_ms(), None);
    }
}
