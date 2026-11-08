//! Modern cross-platform TTF text rasterizer for the shell ink.
//!
//! Owner decision (repo memory + `docs/gdi-text.md`): the exe pins
//! placement, cell size, and ink color; glyph rasterization is
//! presentation, so a modern rasterizer replaces GDI parity. The
//! embedded OFL Noto Sans KR subset (`assets/`, Latin + Hangul) is
//! the one face every cell draws with, covering CP949-sourced Korean
//! (chat, captions, map names — strings are UTF-8 our-side, claim D4
//! in `docs/claims.md`). A parsed-font miss falls back to the legacy
//! bitmap faces in the crate root, so text never blanks.
//!
//! Scale mapping: each [`Cell`] pins one GDI character cell from the
//! exe's font tables (6x12 boxes, 8x16 result, 27x65 loading). The
//! rasterizer spans the cell height across the bitmap-font cell
//! extent — cap height at the cell top, descender depth at the
//! bottom — so glyphs fill their cell like the exe's Gulimche
//! bitmap faces did, and left/top anchoring matches
//! `TextOutA`/`DrawTextA` origins. Coverage snaps to 1-bit ink: the
//! exe's Gulimche/Arial cells are GDI bitmap faces drawn opaque, so
//! glyph pixels carry the pure ink color over any backdrop.

use std::sync::OnceLock;

use ab_glyph::{Font as FontTrait, FontArc, FontVec, GlyphId, PxScale, PxScaleFont, ScaleFont};

use crate::{blend_xrgb, Frame};

/// Bytes of the embedded OFL Noto Sans KR subset (see
/// `assets/README.md` for provenance and the subsetting recipe).
pub const EMBEDDED_FONT_TTF: &[u8] = include_bytes!("../assets/NotoSansKR-Regular-subset.ttf");

/// Lazily parsed embedded face: `None` only when the asset bytes fail
/// to parse, after which every draw takes the bitmap fallback.
static EMBEDDED: OnceLock<Option<RasterFont>> = OnceLock::new();

/// One parsed TrueType/OpenType face ready to rasterize.
#[derive(Debug, Clone)]
pub struct RasterFont {
    /// Parsed face (`ab_glyph`'s owned wrapper).
    face: FontArc,
    /// Measured flat-top cap height (`H`) as an em fraction — the
    /// bitmap-cell scale anchor (Gulimche-style cells fill from the
    /// cap top).
    cap_height_em: f32,
}

impl RasterFont {
    /// Parse font bytes; `None` when they are not a loadable font.
    /// The bytes are copied into the owned face, so any lifetime
    /// parses (`PERF` note: one copy per parse, parses are once-per
    /// face).
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let face = FontArc::from(FontVec::try_from_vec(bytes.to_vec()).ok()?);
        let cap_height_em = cap_height_em(&face);
        Some(Self {
            face,
            cap_height_em,
        })
    }

    /// The parsed face.
    #[must_use]
    pub fn face(&self) -> &FontArc {
        &self.face
    }
}

/// The embedded [`RasterFont`], parsed once, or `None` when the
/// asset failed to parse (callers then draw the bitmap fallback).
#[must_use]
pub fn embedded() -> Option<&'static RasterFont> {
    EMBEDDED
        .get_or_init(|| RasterFont::from_bytes(EMBEDDED_FONT_TTF))
        .as_ref()
}

/// One exe-pinned GDI character cell (`docs/gdi-text.md` pinned font
/// tables). The cell height drives glyph scale; the width stays
/// pinned for callers, while advances follow the face (the exe draws
/// proportional `TextOutA`/`DrawTextA` runs, not fixed-pitch cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// Gulimche 6x12 w400: TEXT controls, lists, chat, captions
    /// (`0x50f280`, control `+0x164`); also the loading gate's small
    /// face (gate `+0x52c`, `0x4e47e8`-`0x4e483a`, `docs/gdi-text.md`).
    Box,
    /// Gulimche/Arial 8x16 w400: result readouts and rank rows
    /// (builder `+0x64`/`+0x68`, `0x4c2871`-`0x4c289b`).
    Result,
    /// Trebuchet MS 27x65 w400: loading-gate display text (gate
    /// `+0x524`, `0x4e47e8`-`0x4e483a`); `CreateFontA` width 27,
    /// height 65 maps to this cell's `(27, 65)` one-to-one. The
    /// gate's Arial 7x14 (`+0x528`) has no dedicated cell and its
    /// Gulimche 6x12 (`+0x52c`) shares [`Cell::Box`].
    Loading,
}

impl Cell {
    /// Pinned `(width, height)` of the GDI character cell in pixels.
    #[must_use]
    pub const fn size(self) -> (u32, u32) {
        match self {
            Cell::Box => (6, 12),
            Cell::Result => (8, 16),
            Cell::Loading => (27, 65),
        }
    }

    /// Bitmap fallback face for this cell: the micro 3x5 glyphs for
    /// the small box cell, the 5x7 glyphs for the larger cells (the
    /// Loading 27x65 Trebuchet face falls back to the 5x7 bitmap —
    /// an approximation only while the TTF asset is missing).
    #[must_use]
    pub const fn bitmap_micro(self) -> bool {
        matches!(self, Cell::Box)
    }
}

/// Round a glyph-space metric to a pixel coordinate: NaN becomes 0,
/// everything else saturates. Float-to-int `as` is total (saturating)
/// in Rust and no `TryFrom` exists for that direction, so the cast is
/// the defined tool here (a `PERF-07` carve-out).
fn px_i32(value: f32) -> i32 {
    if value.is_nan() {
        0
    } else {
        value.round() as i32
    }
}

/// Measure the flat-top cap height (`H`) as an em fraction at a
/// probe scale; `0.7` when the face lacks the glyph or outline (a
/// sane default close to most sans caps).
#[must_use]
fn cap_height_em(face: &FontArc) -> f32 {
    const PROBE_PX_PER_EM: f32 = 100.0;
    let glyph = face
        .glyph_id('H')
        .with_scale(PxScale::from(PROBE_PX_PER_EM));
    let Some(outlined) = face.outline_glyph(glyph) else {
        return 0.7;
    };
    let height = outlined.px_bounds().height();
    if height > 0.0 {
        height / PROBE_PX_PER_EM
    } else {
        0.7
    }
}

/// Pixels per em that spans the pinned cell height across the
/// bitmap-font cell extent: cap height (the cell top) plus
/// descender depth (the cell bottom). Gulimche-style bitmap cells
/// draw caps from the cell top with descenders at the bottom — the
/// exe's 6x12 look — instead of floating small glyphs inside Noto's
/// tall vertical metrics.
#[must_use]
fn cell_px_per_em(font: &RasterFont, cell: Cell) -> f32 {
    let (_, cell_h) = cell.size();
    let descent_em =
        font.face.descent_unscaled().abs() / font.face.units_per_em().unwrap_or(1000.0);
    let denom = font.cap_height_em + descent_em;
    if denom <= 0.0 {
        return cell_h as f32;
    }
    cell_h as f32 / denom
}

/// Scaled font view at a cell's raster size.
fn scaled_at(font: &RasterFont, cell: Cell) -> PxScaleFont<&FontArc> {
    let scale = PxScale::from(cell_px_per_em(font, cell));
    font.face.as_scaled(scale)
}

/// Scaled advance of `id`, or the blank advance for unsupported chars
/// (glyph 0 = notdef): space width keeps layout stable like the
/// bitmap face's blank-but-advancing bytes.
#[must_use]
fn advance_of(scaled: &PxScaleFont<&FontArc>, id: GlyphId) -> f32 {
    if id.0 == 0 {
        scaled.h_advance(scaled.glyph_id(' '))
    } else {
        scaled.h_advance(id)
    }
}

/// Lay `text` out and return the running cursor after it.
#[must_use]
fn cursor_after(font: &RasterFont, text: &str, cell: Cell) -> f32 {
    let scaled = scaled_at(font, cell);
    let mut cursor = 0.0;
    for ch in text.chars() {
        cursor += advance_of(&scaled, scaled.glyph_id(ch));
    }
    cursor
}

/// Draw `text` with a parsed font at `(x, y)`, cell-anchored
/// top-left; returns the x edge after the last glyph. Off-frame
/// glyphs clip per pixel, mirroring the viewport-bound GDI draws.
fn draw_ttf_text(
    font: &RasterFont,
    frame: &mut Frame<'_>,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    cell: Cell,
) -> i32 {
    if frame.buf.len() < frame.w * frame.h {
        return x;
    }
    let scaled = scaled_at(font, cell);
    // i32 origin to f32 glyph space: anchor pixels are exact in f32
    // at every pinned cell size. The baseline pins the cap top to
    // the cell top (bitmap-cell anchoring, like the exe's draws).
    let baseline = y as f32 + font.cap_height_em * cell_px_per_em(font, cell);
    let mut cursor = x as f32;
    for ch in text.chars() {
        let id = scaled.glyph_id(ch);
        if id.0 != 0 {
            paint_glyph(
                frame,
                font.face(),
                id,
                scaled.scale,
                cursor,
                baseline,
                color,
            );
        }
        cursor += advance_of(&scaled, id);
    }
    px_i32(cursor)
}

/// Rasterize one glyph at a baseline position, blending coverage
/// into the frame.
fn paint_glyph(
    frame: &mut Frame<'_>,
    face: &FontArc,
    id: GlyphId,
    scale: PxScale,
    cursor: f32,
    baseline: f32,
    color: u32,
) {
    let glyph = id.with_scale_and_position(scale, ab_glyph::point(cursor, baseline));
    let Some(outlined) = face.outline_glyph(glyph) else {
        return;
    };
    let bounds = outlined.px_bounds();
    let left = px_i32(bounds.min.x);
    let top = px_i32(bounds.min.y);
    outlined.draw(|column, row, coverage| {
        let px = left.saturating_add(i32::try_from(column).unwrap_or(i32::MAX));
        let py = top.saturating_add(i32::try_from(row).unwrap_or(i32::MAX));
        paint_coverage(frame, px, py, coverage, color);
    });
}

/// Paint one glyph pixel with its quantized ink ([`coverage_alpha`],
/// 1-bit). Coordinates were verified non-negative before the call.
fn paint_coverage(frame: &mut Frame<'_>, x: i32, y: i32, coverage: f32, color: u32) {
    let alpha = coverage_alpha(coverage);
    if alpha == 0 || x < 0 || y < 0 || x >= frame.w as i32 || y >= frame.h as i32 {
        return;
    }
    let Ok(row) = usize::try_from(y) else {
        return;
    };
    let Ok(column) = usize::try_from(x) else {
        return;
    };
    let at = row * frame.w + column;
    frame.buf[at] = blend_xrgb(color, frame.buf[at], alpha);
}

/// Quantize rasterizer coverage to 1-bit ink: coverage ≥ 0.5 paints
/// the pure ink color, everything below paints nothing. The exe's
/// Gulimche/Arial cells are GDI bitmap faces (drawn opaque, no
/// antialiasing), so glyph pixels are pure ink — backdrop-
/// independent, like the bitmap faces this rasterizer replaces.
#[must_use]
fn coverage_alpha(coverage: f32) -> u8 {
    if coverage >= 0.5 {
        u8::MAX
    } else {
        0
    }
}

/// Decimal ASCII bytes of `value`, most-significant first (no
/// leading zeros). Returns the buffer and its used length.
#[must_use]
fn decimal_ascii(value: u32) -> ([u8; 10], usize) {
    let mut bytes = [0u8; 10];
    let mut count = 0;
    let mut rest = value;
    loop {
        bytes[count] = b'0' + u8::try_from(rest % 10).unwrap_or(0);
        count += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    bytes[..count].reverse();
    (bytes, count)
}

/// ASCII text view of a decimal digit buffer (the decode is
/// infallible for ASCII; `unwrap_or` guards instead of panicking,
/// `ERRORS-06`).
#[must_use]
fn decimal_view(digits: &[u8]) -> &str {
    core::str::from_utf8(digits).unwrap_or("")
}

/// Draw `text` with `font` when parsed, the bitmap faces otherwise.
/// Box maps to the micro 3x5 face; Result/Loading to the 5x7 face.
pub fn draw_text_with(
    font: Option<&RasterFont>,
    frame: &mut Frame<'_>,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    cell: Cell,
) -> i32 {
    match font {
        Some(font) => draw_ttf_text(font, frame, x, y, text, color, cell),
        None if cell.bitmap_micro() => crate::draw_bitmap_micro_line(frame, x, y, text, color),
        None => crate::draw_bitmap_big_line(frame, x, y, text, color),
    }
}

/// Draw `value` in decimal with `font` or the bitmap fallback.
pub fn draw_number_with(
    font: Option<&RasterFont>,
    frame: &mut Frame<'_>,
    x: i32,
    y: i32,
    value: u32,
    color: u32,
    cell: Cell,
) -> i32 {
    let (digits, count) = decimal_ascii(value);
    draw_text_with(
        font,
        frame,
        x,
        y,
        decimal_view(&digits[..count]),
        color,
        cell,
    )
}

/// Pixel width of `text` with `font`, or the bitmap face width.
#[must_use]
pub fn text_width_with(font: Option<&RasterFont>, text: &str, cell: Cell) -> usize {
    match font {
        Some(font) => usize::try_from(px_i32(cursor_after(font, text, cell))).unwrap_or(0),
        None if cell.bitmap_micro() => crate::bitmap_micro_text_width(text),
        None => crate::bitmap_big_text_width(text),
    }
}

/// Pixel width of `value` in decimal with `font`, or bitmap width.
#[must_use]
pub fn number_width_with(font: Option<&RasterFont>, value: u32, cell: Cell) -> usize {
    match font {
        Some(_) => {
            let (digits, count) = decimal_ascii(value);
            text_width_with(font, decimal_view(&digits[..count]), cell)
        }
        None if cell.bitmap_micro() => crate::bitmap_micro_number_width(value),
        None => crate::bitmap_big_number_width(value),
    }
}

/// Draw `text` at `(x, y)` with the embedded font (bitmap fallback
/// when the asset is missing) at `cell`; returns the x edge.
pub fn draw_text_cell(
    frame: &mut Frame<'_>,
    x: i32,
    y: i32,
    text: &str,
    color: u32,
    cell: Cell,
) -> i32 {
    draw_text_with(embedded(), frame, x, y, text, color, cell)
}

/// Draw `value` in decimal with the embedded font at `cell`.
pub fn draw_number_cell(
    frame: &mut Frame<'_>,
    x: i32,
    y: i32,
    value: u32,
    color: u32,
    cell: Cell,
) -> i32 {
    draw_number_with(embedded(), frame, x, y, value, color, cell)
}

/// Pixel width of `text` with the embedded font at `cell`.
#[must_use]
pub fn text_width_cell(text: &str, cell: Cell) -> usize {
    text_width_with(embedded(), text, cell)
}

/// Pixel width of `value` in decimal with the embedded font at `cell`.
#[must_use]
pub fn number_width_cell(value: u32, cell: Cell) -> usize {
    number_width_with(embedded(), value, cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Blank XRGB background the draw tests start from.
    const BG: u32 = 0;

    /// Draw `text` at (4, 4) on a `w` x `h` buffer; returns the
    /// buffer (blank `BG` background) and the layout end x.
    fn drawn(text: &str, cell: Cell, w: usize, h: usize) -> (Vec<u32>, i32) {
        let mut buf = vec![BG; w * h];
        let mut frame = Frame {
            buf: &mut buf,
            w,
            h,
        };
        let end = draw_text_cell(&mut frame, 4, 4, text, 0xFF_FF_FF, cell);
        (buf, end)
    }

    /// Lit-pixel count of a drawn buffer.
    fn lit_pixels(buf: &[u32]) -> usize {
        buf.iter().filter(|pixel| **pixel != BG).count()
    }

    #[test]
    fn embedded_asset_parses_with_latin_and_hangul_coverage() {
        let Some(font) = embedded() else {
            panic!("the committed font asset must parse");
        };
        let scaled = scaled_at(font, Cell::Box);
        for ch in ['A', 'z', '0', '한', '글', 'ㄱ', 'ㅎ'] {
            assert_ne!(scaled.glyph_id(ch).0, 0, "{ch} must map a real glyph");
        }
        assert_eq!(scaled.glyph_id('★').0, 0, "unsupported chars map notdef");
    }

    #[test]
    fn font_from_bytes_rejects_garbage() {
        assert!(RasterFont::from_bytes(&[]).is_none());
        assert!(RasterFont::from_bytes(&[0x00, 0x01, 0x02, 0x03]).is_none());
    }

    #[test]
    fn latin_draws_ink_at_the_box_cell() {
        let (buf, end) = drawn("OK", Cell::Box, 32, 24);
        assert!(lit_pixels(&buf) > 10, "O and K must light pixels");
        assert!(end > 4, "layout must advance past the origin");
    }

    #[test]
    fn hangul_renders_real_glyphs_not_blanks() {
        // CP949-sourced chat/caption text arrives as UTF-8 (claim D4):
        // each syllable must rasterize real ink, and different
        // syllables must rasterize different ink.
        let (han, _) = drawn("한", Cell::Box, 32, 24);
        let (geul, _) = drawn("글", Cell::Box, 32, 24);
        let han_lit = lit_pixels(&han);
        let geul_lit = lit_pixels(&geul);
        assert!(han_lit >= 8, "한 must rasterize, not blank: {han_lit} px");
        assert!(geul_lit >= 8, "글 must rasterize, not blank: {geul_lit} px");
        assert_ne!(han, geul, "한 and 글 draw distinct glyphs");
    }

    #[test]
    fn jamo_cells_draw_for_the_chosung_grid() {
        // The sort-grid initial consonants (U+3131..) closed their
        // documented blank gap with this face.
        let (buf, _) = drawn("ㄱ", Cell::Box, 32, 24);
        assert!(lit_pixels(&buf) >= 4, "ㄱ must rasterize");
    }

    #[test]
    fn missing_font_falls_back_to_the_bitmap_faces() {
        let mut buf = vec![BG; 32 * 16];
        let mut frame = Frame {
            buf: &mut buf,
            w: 32,
            h: 16,
        };
        let end = draw_text_with(None, &mut frame, 0, 0, "HI!", 0xFF_FF_FF, Cell::Box);
        assert_eq!(end, i32::try_from(3 * crate::GLYPH_ADVANCE).unwrap_or(0));
        assert_eq!(frame.buf[0], 0xFF_FF_FF, "'H' stem paints");
        // The big fallback face serves the larger cells.
        let mut big = vec![BG; 48 * 16];
        let mut big_frame = Frame {
            buf: &mut big,
            w: 48,
            h: 16,
        };
        let big_end = draw_text_with(None, &mut big_frame, 0, 0, "HI!", 0xFF_FF_FF, Cell::Result);
        assert_eq!(
            big_end,
            i32::try_from(3 * crate::BIG_GLYPH_ADVANCE).unwrap_or(0)
        );
        assert_eq!(big_frame.buf[0], 0xFF_FF_FF);
    }

    #[test]
    fn missing_font_widths_fall_back_to_bitmap_formulas() {
        assert_eq!(
            text_width_with(None, "HI!", Cell::Box),
            3 * crate::GLYPH_ADVANCE - 1
        );
        assert_eq!(
            text_width_with(None, "HI!", Cell::Result),
            3 * crate::BIG_GLYPH_ADVANCE - 1
        );
        assert_eq!(
            number_width_with(None, 507, Cell::Box),
            3 * crate::GLYPH_ADVANCE - 1
        );
    }

    #[test]
    fn loading_cell_pins_the_gate_trebuchet_face() {
        // Gate `0x4e47a0` (`0x4e47e8`-`0x4e483a`, `docs/gdi-text.md`):
        // Trebuchet MS 27x65 w400 at `+0x524` maps one-to-one to
        // this cell; Arial 7x14 (`+0x528`) has no dedicated cell and
        // Gulimche 6x12 (`+0x52c`) shares Box.
        assert_eq!(Cell::Loading.size(), (27, 65));
        assert_eq!(Cell::Box.size(), (6, 12));
        assert!(!Cell::Loading.bitmap_micro());
        assert!(Cell::Box.bitmap_micro());
    }

    #[test]
    fn cells_map_to_pinned_gdi_sizes() {
        assert_eq!(Cell::Box.size(), (6, 12));
        assert_eq!(Cell::Result.size(), (8, 16));
        assert_eq!(Cell::Loading.size(), (27, 65));
        // The same string must scale strictly with the pinned cell.
        let box_w = text_width_cell("OK", Cell::Box);
        let result_w = text_width_cell("OK", Cell::Result);
        let loading_w = text_width_cell("OK", Cell::Loading);
        assert!(box_w > 0);
        assert!(result_w > box_w, "{result_w} must exceed {box_w}");
        assert!(loading_w > result_w, "{loading_w} must exceed {result_w}");
    }

    #[test]
    fn unsupported_char_advances_blank() {
        let (blank, _) = drawn("★", Cell::Box, 32, 24);
        assert_eq!(lit_pixels(&blank), 0, "notdef draws nothing");
        // The blank advance keeps mixed-script layout stable: the
        // run with the unsupported char must lay out wider than the
        // run without it.
        let plain_w = text_width_cell("AB", Cell::Box);
        let with_blank_w = text_width_cell("A★B", Cell::Box);
        assert!(
            with_blank_w > plain_w,
            "{with_blank_w} must exceed {plain_w}"
        );
    }

    #[test]
    fn numbers_render_decimal_without_leading_zeros() {
        let (buf, _) = drawn("0", Cell::Box, 32, 24);
        assert!(lit_pixels(&buf) > 4, "zero must draw its glyph");
        let (_, end) = drawn("507", Cell::Result, 48, 32);
        assert_eq!(
            end - 4,
            i32::try_from(number_width_cell(507, Cell::Result)).unwrap_or(0)
        );
    }

    #[test]
    fn glyphs_clip_at_frame_edges() {
        let mut buf = vec![BG; 12 * 12];
        let mut frame = Frame {
            buf: &mut buf,
            w: 12,
            h: 12,
        };
        // A partially off-frame origin clips per pixel without
        // panicking and still paints the in-frame half of the glyph.
        draw_text_cell(&mut frame, -3, -3, "한", 0xFF_FF_FF, Cell::Box);
        draw_text_cell(&mut frame, 10, 10, "한글", 0xFF_FF_FF, Cell::Box);
        assert!(frame.buf.iter().any(|pixel| pixel != &BG));
        // Fully off-frame draws leave the frame untouched.
        let mut clean = vec![BG; 12 * 12];
        let mut clean_frame = Frame {
            buf: &mut clean,
            w: 12,
            h: 12,
        };
        draw_text_cell(&mut clean_frame, 900, 900, "한", 0xFF_FF_FF, Cell::Box);
        assert!(clean_frame.buf.iter().all(|pixel| pixel == &BG));
    }
}
