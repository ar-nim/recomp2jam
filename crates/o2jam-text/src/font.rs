//! System-font rasterization with an ASCII stand-in fallback.
//!
//! The face is the system Noto Sans CJK KR (OFL, never vendored):
//! `/usr/share/fonts/noto-cjk/NotoSansCJK-*.ttc`, whose KR face is
//! index 1 (verified by walking the collection name table — face 0
//! is JP, face 1 is KR). When the collection is absent the loader
//! returns `None` and every draw/measure entry point degrades to
//! the ASCII stand-in: non-ASCII characters become `?` and layout
//! uses a fixed advance, so text still lands somewhere legible
//! instead of vanishing. Tests that need the real face skip
//! cleanly when it is missing.

use std::path::PathBuf;
use std::sync::OnceLock;

use ab_glyph::{Font as FontTrait, FontArc, FontVec, GlyphId, PxScale, PxScaleFont, ScaleFont};

/// Directory holding the system Noto Sans CJK collections.
pub const FONT_DIR: &str = "/usr/share/fonts/noto-cjk";

/// Preferred collection file (its KR face is the client face).
pub const FONT_FILENAME: &str = "NotoSansCJK-Regular.ttc";

/// Face index of the KR face inside a Noto Sans CJK collection
/// (name-table walk: 0 = JP, 1 = KR, 2 = SC, 3 = TC, 4 = HK).
pub const KR_FACE_INDEX: u32 = 1;

/// Advance of one stand-in character, as an em fraction.
pub const ASCII_ADVANCE_EM: f32 = 0.6;

/// Placeholder the stand-in substitutes for non-ASCII characters.
pub const ASCII_PLACEHOLDER: char = '?';

/// One parsed TrueType/OpenType face ready to rasterize.
#[derive(Debug, Clone)]
pub struct RasterFont {
    /// Parsed face (`ab_glyph`'s owned wrapper).
    face: FontArc,
    /// Cap height (`H`) as an em fraction: the cell-top anchor the
    /// exe's bitmap cells fill from (`docs/gdi-text.md`).
    cap_height_em: f32,
}

impl RasterFont {
    /// Parse font bytes at face index 0; `None` when the bytes are
    /// not a loadable font.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Self::from_bytes_at_face(bytes, 0)
    }

    /// Parse font bytes at one face index (collections are parsed
    /// with [`FontVec::try_from_vec_and_index`]); `None` when the
    /// bytes or the index do not resolve.
    #[must_use]
    pub fn from_bytes_at_face(bytes: &[u8], index: u32) -> Option<Self> {
        let vec = FontVec::try_from_vec_and_index(bytes.to_vec(), index).ok()?;
        let face = FontArc::from(vec);
        let cap_height_em = cap_height_em(&face);
        Some(Self {
            face,
            cap_height_em,
        })
    }

    /// Whether the face maps `character` to a real glyph (not the
    /// notdef placeholder).
    #[must_use]
    pub fn has_glyph(&self, character: char) -> bool {
        self.face.glyph_id(character).0 != 0
    }

    /// The parsed face.
    #[must_use]
    pub fn face(&self) -> &FontArc {
        &self.face
    }

    /// Cap height in em: the `H` advance the shaped-text baseline
    /// and the room row fitter size from.
    #[must_use]
    pub fn cap_height_em(&self) -> f32 {
        self.cap_height_em
    }
}

/// Lazily loaded system face: `None` only when the Noto Sans CJK
/// collection is absent or unparseable, not an error path.
static SYSTEM_FONT: OnceLock<Option<RasterFont>> = OnceLock::new();

/// The system [`RasterFont`], parsed once, or `None` when the
/// collection is absent (callers then take the ASCII stand-in).
#[must_use]
pub fn system_font() -> Option<&'static RasterFont> {
    SYSTEM_FONT.get_or_init(load_system_font).as_ref()
}

/// Parse the KR face of the system collection, if present.
fn load_system_font() -> Option<RasterFont> {
    let bytes = std::fs::read(system_font_path()?).ok()?;
    parse_kr_face(&bytes)
}

/// Parse the KR face of collection `bytes`, falling back to face 0
/// when the KR index does not resolve (or carries no Hangul).
fn parse_kr_face(bytes: &[u8]) -> Option<RasterFont> {
    match RasterFont::from_bytes_at_face(bytes, KR_FACE_INDEX) {
        Some(font) if font.has_glyph('\u{AC00}') => Some(font),
        _ => RasterFont::from_bytes(bytes),
    }
}

/// The collection path to load: the preferred file first, then any
/// `NotoSansCJK-*.ttc` in [`FONT_DIR`], sorted by name.
#[must_use]
pub fn system_font_path() -> Option<PathBuf> {
    let preferred = PathBuf::from(FONT_DIR).join(FONT_FILENAME);
    if preferred.is_file() {
        return Some(preferred);
    }
    let mut names: Vec<String> = std::fs::read_dir(FONT_DIR)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("NotoSansCJK-") && name.ends_with(".ttc"))
        .collect();
    names.sort();
    names
        .into_iter()
        .next()
        .map(|name| PathBuf::from(FONT_DIR).join(name))
}

/// `H` cap height as an em fraction at a probe scale; `0.7` (close
/// to most sans caps) when the face lacks the glyph or outline.
#[must_use]
fn cap_height_em(face: &FontArc) -> f32 {
    const PROBE_PX_PER_EM: f32 = 100.0;
    let glyph = face
        .glyph_id('H')
        .with_scale(PxScale::from(PROBE_PX_PER_EM));
    match face.outline_glyph(glyph) {
        Some(outlined) if outlined.px_bounds().height() > 0.0 => {
            outlined.px_bounds().height() / PROBE_PX_PER_EM
        }
        _ => 0.7,
    }
}

/// One rasterized glyph: coverage bytes (`0..=255`, one per pixel,
/// row-major) plus the placement of the mask origin relative to
/// the baseline position it was rasterized at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    /// Mask width in pixels.
    pub width: u32,
    /// Mask height in pixels.
    pub height: u32,
    /// Pixels from the baseline position to the mask's left edge.
    pub offset_x: i32,
    /// Pixels from the baseline position to the mask's top edge
    /// (negative above the baseline).
    pub offset_y: i32,
    /// Coverage bytes, row-major, `width * height` entries.
    pub coverage: Vec<u8>,
}

impl Mask {
    /// Lit-pixel count (coverage above the half-ink threshold).
    #[must_use]
    pub fn lit_pixels(&self) -> usize {
        self.coverage.iter().filter(|it| **it >= 128).count()
    }
}

/// Rasterize one character at `px_scale` px/em; `None` for
/// unsupported characters (notdef) or outline-less glyphs.
#[must_use]
pub fn rasterize(font: &RasterFont, character: char, px_scale: f32) -> Option<Mask> {
    let scaled = scaled_font(font, px_scale);
    let id = scaled.glyph_id(character);
    if id.0 == 0 {
        return None;
    }
    rasterize_glyph(font, id, scaled)
}

/// Rasterize one glyph id at its scaled size, at the origin.
fn rasterize_glyph(font: &RasterFont, id: GlyphId, scaled: PxScaleFont<&FontArc>) -> Option<Mask> {
    let glyph = id.with_scale(scaled.scale);
    let outlined = font.face.outline_glyph(glyph)?;
    let bounds = outlined.px_bounds();
    let width = non_negative(bounds.width())?;
    let height = non_negative(bounds.height())?;
    let mut coverage = vec![0_u8; (width * height) as usize];
    outlined.draw(|column, row, value| {
        let index = (usize::try_from(row).unwrap_or(0) * width as usize)
            .saturating_add(usize::try_from(column).unwrap_or(0));
        if let Some(cell) = coverage.get_mut(index) {
            *cell = coverage_byte(value);
        }
    });
    Some(Mask {
        width,
        height,
        offset_x: px_i32(bounds.min.x),
        offset_y: px_i32(bounds.min.y),
        coverage,
    })
}

/// Round a glyph metric to pixels: NaN becomes 0, everything else
/// saturates. Float-to-int `as` is total in Rust, the defined
/// tool here.
fn px_i32(value: f32) -> i32 {
    if value.is_nan() {
        0
    } else {
        value.round() as i32
    }
}

/// A non-negative pixel extent (`u32`), or `None` when the float
/// extent is negative or non-finite.
fn non_negative(extent: f32) -> Option<u32> {
    if extent.is_finite() && extent >= 0.0 {
        Some(extent.round() as u32)
    } else {
        None
    }
}

/// Quantize rasterizer coverage (`0.0..=1.0`) to a byte.
#[must_use]
fn coverage_byte(coverage: f32) -> u8 {
    coverage.clamp(0.0, 1.0).mul_add(255.0, 0.5) as u8
}

/// Scaled font view at `px_scale` pixels per em.
fn scaled_font(font: &RasterFont, px_scale: f32) -> PxScaleFont<&FontArc> {
    font.face.as_scaled(PxScale::from(px_scale))
}

/// Scaled advance of one glyph id (notdef takes the space width,
/// so unsupported characters still advance layout).
fn advance_of(scaled: &PxScaleFont<&FontArc>, id: GlyphId) -> f32 {
    if id.0 == 0 {
        scaled.h_advance(scaled.glyph_id(' '))
    } else {
        scaled.h_advance(id)
    }
}

/// Pixel advance of `text` with `font` at `px_scale` px/em.
#[must_use]
pub fn measure_text(font: &RasterFont, text: &str, px_scale: f32) -> f32 {
    let scaled = scaled_font(font, px_scale);
    text.chars()
        .map(|character| advance_of(&scaled, scaled.glyph_id(character)))
        .sum()
}

/// The ASCII stand-in for `text`: ASCII characters verbatim,
/// everything else becomes [`ASCII_PLACEHOLDER`].
#[must_use]
pub fn ascii_stand_in(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_ascii() {
                character
            } else {
                ASCII_PLACEHOLDER
            }
        })
        .collect()
}

/// Pixel advance of `text` under the ASCII stand-in (fixed
/// [`ASCII_ADVANCE_EM`] advance per character).
#[must_use]
pub fn measure_text_stand_in(text: &str, px_scale: f32) -> f32 {
    text.chars().count() as f32 * ASCII_ADVANCE_EM * px_scale
}

/// An RGBA8 image that [`draw_text`] paints into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA bytes, row-major, `width * height * 4` entries.
    pub rgba: Vec<u8>,
}

impl Image {
    /// A blank (transparent-black) image of `width` by `height`.
    #[must_use]
    pub fn blank(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    /// Paint one coverage cell with `color`, blended over the
    /// existing bytes. Coordinates outside the image clip away.
    fn paint(&mut self, x: i32, y: i32, coverage: u8, color: [u8; 4]) {
        if coverage == 0 {
            return;
        }
        let (Some(column), Some(row)) = (usize::try_from(x).ok(), usize::try_from(y).ok()) else {
            return;
        };
        if column >= self.width as usize || row >= self.height as usize {
            return;
        }
        let at = (row * self.width as usize + column) * 4;
        blend_rgba(&mut self.rgba[at..at + 4], color, coverage);
    }
}

/// Blend `coverage`-weighted `color` over one RGBA cell.
fn blend_rgba(cell: &mut [u8], color: [u8; 4], coverage: u8) {
    let weight = usize::from(coverage);
    for index in 0..4 {
        let ink = usize::from(color[index]);
        let back = usize::from(cell[index]);
        cell[index] = ((ink * weight + back * (255 - weight)) / 255) as u8;
    }
}

/// Draw `text` with `font` on `image` at `(x, y)` (left/top
/// anchored, like `TextOutA` origins in `docs/gdi-text.md`) at
/// `px_scale` px/em; returns the x edge after the last glyph.
/// With `font = None` the ASCII stand-in paints fixed cells so
/// layout survives without the system face.
pub fn draw_text(
    image: &mut Image,
    font: Option<&RasterFont>,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
    px_scale: f32,
) -> i32 {
    match font {
        Some(font) => draw_shaped_text(image, font, x, y, text, color, px_scale),
        None => draw_stand_in_text(image, x, y, text, color, px_scale),
    }
}

/// Draw `cp949_bytes` (decoded lossily when malformed) with
/// [`draw_text`].
pub fn draw_text_cp949(
    image: &mut Image,
    font: Option<&RasterFont>,
    x: i32,
    y: i32,
    cp949_bytes: &[u8],
    color: [u8; 4],
    px_scale: f32,
) -> i32 {
    let text = crate::decode::decode_cp949_lossy(cp949_bytes);
    draw_text(image, font, x, y, &text, color, px_scale)
}

/// Draw shaped text: each glyph rasterized at the running cursor,
/// cap top pinned to the cell top.
fn draw_shaped_text(
    image: &mut Image,
    font: &RasterFont,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
    px_scale: f32,
) -> i32 {
    let scaled = scaled_font(font, px_scale);
    let baseline = y as f32 + font.cap_height_em * px_scale;
    let mut cursor = x as f32;
    for character in text.chars() {
        let id = scaled.glyph_id(character);
        paint_mask_at(image, font, id, scaled.scale, cursor, baseline, color);
        cursor += advance_of(&scaled, id);
    }
    px_i32(cursor)
}

/// Paint glyph `id`'s mask with its origin at `(cursor, baseline)`.
fn paint_mask_at(
    image: &mut Image,
    font: &RasterFont,
    id: GlyphId,
    scale: PxScale,
    cursor: f32,
    baseline: f32,
    color: [u8; 4],
) {
    let glyph = id.with_scale_and_position(scale, ab_glyph::point(cursor, baseline));
    let Some(outlined) = font.face.outline_glyph(glyph) else {
        return;
    };
    let bounds = outlined.px_bounds();
    let left = px_i32(bounds.min.x);
    let top = px_i32(bounds.min.y);
    outlined.draw(|column, row, value| {
        image.paint(
            left.saturating_add(px_i32(column as f32)),
            top.saturating_add(px_i32(row as f32)),
            coverage_byte(value),
            color,
        );
    });
}

/// Draw the ASCII stand-in: one fixed cell per character of
/// [`ascii_stand_in`], filled where the character stands in for
/// visible ink and blank for the space cell.
fn draw_stand_in_text(
    image: &mut Image,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 4],
    px_scale: f32,
) -> i32 {
    let advance = (ASCII_ADVANCE_EM * px_scale).round() as i32;
    let height = px_i32(px_scale);
    for (index, character) in ascii_stand_in(text).chars().enumerate() {
        if character != ' ' {
            paint_stand_in_cell(image, x + index as i32 * advance, y, advance, height, color);
        }
    }
    x.saturating_add(text.chars().count() as i32 * advance)
}

/// Paint one stand-in cell: an ink border around a blank interior.
fn paint_stand_in_cell(image: &mut Image, x: i32, y: i32, width: i32, height: i32, color: [u8; 4]) {
    for row in 0..height {
        for column in 0..width {
            if row == 0 || column == 0 || row == height - 1 || column == width - 1 {
                image.paint(x + column, y + row, u8::MAX, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The system face, or `None` when this machine has no Noto
    /// Sans CJK collection (font-dependent scenarios skip).
    fn system_face() -> Option<&'static RasterFont> {
        system_font()
    }

    /// Lit (non-blank) pixel count of an image.
    fn lit_image_pixels(image: &Image) -> usize {
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|cell| **cell != [0, 0, 0, 0])
            .count()
    }

    #[test]
    fn garbage_bytes_are_not_a_font() {
        assert!(RasterFont::from_bytes(&[]).is_none());
        assert!(RasterFont::from_bytes(&[0x00, 0x01, 0x02, 0x03]).is_none());
        assert!(RasterFont::from_bytes_at_face(&[0x00; 64], 99).is_none());
    }

    #[test]
    fn ascii_stand_in_maps_non_ascii_to_question_marks() {
        assert_eq!(ascii_stand_in("Lv.43 한글"), "Lv.43 ??");
        assert_eq!(ascii_stand_in("한글"), "??");
        assert_eq!(ascii_stand_in("abc XYZ 09"), "abc XYZ 09");
        assert_eq!(ascii_stand_in(""), "");
    }

    #[test]
    fn absent_font_draws_the_ascii_stand_in_cells() {
        let mut image = Image::blank(64, 32);
        let end = draw_text(&mut image, None, 2, 2, "한글", [255, 255, 255, 255], 16.0);
        assert!(end > 2, "stand-in layout must advance");
        assert!(lit_image_pixels(&image) > 0, "stand-in cells must paint");
        // Spaces stay blank cells (no ink, same advance).
        let mut spaced = Image::blank(64, 32);
        let spaced_end = draw_text(&mut spaced, None, 2, 2, " ", [255, 255, 255, 255], 16.0);
        assert_eq!(spaced_end, 2 + (ASCII_ADVANCE_EM * 16.0).round() as i32);
        assert_eq!(lit_image_pixels(&spaced), 0, "space paints nothing");
    }

    #[test]
    fn stand_in_measure_counts_every_character() {
        assert_eq!(measure_text_stand_in("한글", 10.0), 2.0 * 0.6 * 10.0);
        assert_eq!(measure_text_stand_in("", 10.0), 0.0);
    }

    #[test]
    fn cp949_draw_without_font_reaches_the_stand_in() {
        let mut image = Image::blank(64, 32);
        let end = draw_text_cp949(
            &mut image,
            None,
            2,
            2,
            &[0xC7, 0xD1, 0xB1, 0xDB],
            [255, 255, 255, 255],
            16.0,
        );
        let mut plain = Image::blank(64, 32);
        let plain_end = draw_text(&mut plain, None, 2, 2, "한글", [255, 255, 255, 255], 16.0);
        assert_eq!(end, plain_end);
        assert_eq!(lit_image_pixels(&image), lit_image_pixels(&plain));
    }

    #[test]
    fn system_face_rasterizes_latin_and_hangul_when_present() {
        let Some(font) = system_face() else {
            return;
        };
        assert!(font.has_glyph('A'));
        assert!(font.has_glyph('한'));
        // U+10FFFF is unassigned in every font (notdef wherever it lands).
        assert!(!font.has_glyph('\u{10FFFF}'));
        let latin = rasterize(font, 'A', 32.0).unwrap();
        assert!(latin.lit_pixels() > 20, "A must rasterize ink");
        let hangul = rasterize(font, '한', 32.0).unwrap();
        assert!(hangul.lit_pixels() > 20, "한 must rasterize ink");
        assert_ne!(rasterize(font, '한', 32.0), rasterize(font, '글', 32.0));
    }

    #[test]
    fn system_face_draws_shaped_text_when_present() {
        let Some(font) = system_face() else {
            return;
        };
        let mut image = Image::blank(128, 64);
        let end = draw_text(
            &mut image,
            Some(font),
            4,
            4,
            "한글 OK",
            [255, 255, 255, 255],
            32.0,
        );
        assert!(end > 4, "shaped layout must advance");
        assert!(lit_image_pixels(&image) > 100, "shaped text must paint ink");
        let width = measure_text(font, "한글 OK", 32.0);
        assert_eq!(end - 4, px_i32(width));
    }

    #[test]
    fn system_face_cp949_draw_matches_decoded_draw_when_present() {
        let Some(font) = system_face() else {
            return;
        };
        let mut cp949 = Image::blank(128, 64);
        let cp949_end = draw_text_cp949(
            &mut cp949,
            Some(font),
            4,
            4,
            &[0xC7, 0xD1, 0xB1, 0xDB],
            [255, 255, 255, 255],
            32.0,
        );
        let mut text = Image::blank(128, 64);
        let text_end = draw_text(
            &mut text,
            Some(font),
            4,
            4,
            "한글",
            [255, 255, 255, 255],
            32.0,
        );
        assert_eq!(cp949_end, text_end);
        assert_eq!(cp949.rgba, text.rgba);
    }

    #[test]
    fn shaped_text_clips_at_image_edges() {
        let Some(font) = system_face() else {
            return;
        };
        let mut image = Image::blank(16, 16);
        draw_text(
            &mut image,
            Some(font),
            -8,
            -8,
            "한글",
            [255, 255, 255, 255],
            32.0,
        );
        draw_text(
            &mut image,
            Some(font),
            12,
            12,
            "한글",
            [255, 255, 255, 255],
            32.0,
        );
        assert!(lit_image_pixels(&image) > 0, "in-frame halves must paint");
    }
}
