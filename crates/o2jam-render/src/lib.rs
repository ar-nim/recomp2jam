//! `o2jam-render` — software sprite compositor for the game client.
//!
//! Pure pixel math, no GPU: RGB555 sprites (see `o2jam-file`) blit
//! into XRGB8888 frames that a backend (later: wgpu) uploads.
//! Everything here is headless-testable with synthetic pixels.

/// Expand one little-endian RGB555 pixel to 24-bit RGB.
#[must_use]
pub const fn expand_rgb555(pixel: u16) -> (u8, u8, u8) {
    let r = ((pixel >> 10) & 31) as u8;
    let g = ((pixel >> 5) & 31) as u8;
    let b = (pixel & 31) as u8;
    (
        (r << 3) | (r >> 2),
        (g << 3) | (g >> 2),
        (b << 3) | (b >> 2),
    )
}

/// Pack 8-bit RGB into an XRGB8888 framebuffer word.
#[must_use]
pub const fn pack_xrgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Mutable XRGB8888 framebuffer view.
pub struct Frame<'a> {
    /// Row-major pixels, `w * h` long.
    pub buf: &'a mut [u32],
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
}

/// Integer rectangle (position + size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge (may be negative = off-frame).
    pub x: i32,
    /// Top edge (may be negative = off-frame).
    pub y: i32,
    /// Width in pixels.
    pub w: u32,
    /// Height in pixels.
    pub h: u32,
}

/// RGB555 sprite view: row-major pixels with a per-row stride.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sprite<'a> {
    /// Raw little-endian RGB555 words.
    pub pixels: &'a [u16],
    /// Visible width in pixels.
    pub w: usize,
    /// Visible height in pixels.
    pub h: usize,
    /// Words per row (>= `w`; NPOT rows may pad).
    pub stride: usize,
}

/// Clamp a rect to a `w * h` frame; `None` when fully off-frame.
fn clip(rect: &Rect, w: usize, h: usize) -> Option<(usize, usize, usize, usize)> {
    let x0 = rect.x.max(0) as usize;
    let y0 = rect.y.max(0) as usize;
    let x1 = (i64::from(rect.x) + i64::from(rect.w)).min(w as i64).max(0) as usize;
    let y1 = (i64::from(rect.y) + i64::from(rect.h)).min(h as i64).max(0) as usize;
    (x0 < x1 && y0 < y1).then_some((x0, y0, x1, y1))
}

/// Fill a clipped rectangle with a solid color.
pub fn fill_rect(frame: &mut Frame<'_>, rect: &Rect, color: u32) {
    if frame.buf.len() < frame.w * frame.h {
        return;
    }
    let Some((x0, y0, x1, y1)) = clip(rect, frame.w, frame.h) else {
        return;
    };
    let w = frame.w;
    for row in &mut frame.buf[y0 * w..y1 * w].chunks_exact_mut(w) {
        row[x0..x1].fill(color);
    }
}

/// Owned raw RGB555 image: the blittable form of one sprite-55 body
/// (see `o2jam-file`). Words are native-endian decoded from the
/// little-endian container bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb555Image {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Row-major pixels, `w * h` long.
    pub words: Vec<u16>,
}

impl Rgb555Image {
    /// Decode `w*h` little-endian pixels; `None` on size mismatch.
    #[must_use]
    pub fn decode(bytes: &[u8], w: usize, h: usize) -> Option<Self> {
        if bytes.len() != w * h * 2 {
            return None;
        }
        let (chunks, _) = bytes.as_chunks::<2>();
        let words = chunks
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        Some(Self { w, h, words })
    }

    /// Borrow as a blit-ready sprite (dense stride).
    #[must_use]
    pub fn view(&self) -> Sprite<'_> {
        Sprite {
            pixels: &self.words,
            w: self.w,
            h: self.h,
            stride: self.w,
        }
    }
}

/// Nearest-neighbor source coordinate for one destination coordinate.
fn stretch_index(dst: usize, dst_len: usize, src_len: usize) -> usize {
    dst * src_len / dst_len
}

/// Stretch `src` (`src_w` x `src_h`) over `dst` (`dst_w` x `dst_h`).
pub fn stretch_blit(
    dst: &mut [u32],
    dst_w: usize,
    dst_h: usize,
    src: &[u32],
    src_w: usize,
    src_h: usize,
) {
    if dst_w == 0 || dst_h == 0 || src_w == 0 || src_h == 0 {
        return;
    }
    if dst.len() < dst_w * dst_h || src.len() < src_w * src_h {
        return;
    }
    for y in 0..dst_h {
        for x in 0..dst_w {
            dst[y * dst_w + x] =
                src[stretch_index(y, dst_h, src_h) * src_w + stretch_index(x, dst_w, src_w)];
        }
    }
}

/// Blit a sprite at `(dx, dy)`, skipping `colorkey` pixels when set.
/// Output clips to the frame.
pub fn blit_rgb555(
    frame: &mut Frame<'_>,
    sprite: &Sprite<'_>,
    dx: i32,
    dy: i32,
    colorkey: Option<u16>,
) {
    blit_rgb555_clipped(frame, sprite, dx, dy, colorkey, None);
}

/// Blit a sprite, additionally dropping pixels outside `clip`.
/// Skill-ring curtains (hidden/sudden/dark) render notes through the
/// spans from [`visible_spans`]; pass `None` for an uncurtained blit.
pub fn blit_rgb555_clipped(
    frame: &mut Frame<'_>,
    sprite: &Sprite<'_>,
    dx: i32,
    dy: i32,
    colorkey: Option<u16>,
    clip: Option<&Rect>,
) {
    if sprite.stride == 0
        || frame.buf.len() < frame.w * frame.h
        || sprite.pixels.len() < sprite.stride * sprite.h
    {
        return;
    }
    for row in 0..sprite.h {
        let dest_y = dy + row as i32;
        if dest_y < 0 || dest_y >= frame.h as i32 || is_clipped_y(clip, dest_y) {
            continue;
        }
        for col in 0..sprite.w {
            let dest_x = dx + col as i32;
            if dest_x < 0 || dest_x >= frame.w as i32 || is_clipped_x(clip, dest_x) {
                continue;
            }
            let pixel = sprite.pixels[row * sprite.stride + col];
            if colorkey == Some(pixel) {
                continue;
            }
            let (r, g, b) = expand_rgb555(pixel);
            frame.buf[dest_y as usize * frame.w + dest_x as usize] = pack_xrgb(r, g, b);
        }
    }
}

/// Whether a destination row falls outside the clip rect.
fn is_clipped_y(clip: Option<&Rect>, dest_y: i32) -> bool {
    clip.is_some_and(|rect| dest_y < rect.y || dest_y >= rect.y + rect.h as i32)
}

/// Whether a destination column falls outside the clip rect.
fn is_clipped_x(clip: Option<&Rect>, dest_x: i32) -> bool {
    clip.is_some_and(|rect| dest_x < rect.x || dest_x >= rect.x + rect.w as i32)
}

/// Share of the note highway one curtain edge covers or reveals.
///
/// From the shipped ring descriptions (`Itemdata_Korea.dat` in
/// `Image/avatar.opa`): hidden covers the bottom 20% of the chart,
/// sudden shows only that bottom 20%, dark shows only the middle 20%,
/// and reverse-dark covers the middle 20%.
const CURTAIN_SHARE_DENOMINATOR: u32 = 5;

/// Note-highway curtain worn by a skill ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneCurtain {
    /// Whole highway visible (no ring).
    Full,
    /// Bottom fifth covered (hidden ring).
    Hidden,
    /// Only the bottom fifth visible (sudden ring).
    Sudden,
    /// Only the middle fifth visible (dark ring).
    Dark,
    /// Middle fifth covered (reverse-dark ring).
    ReverseDark,
}

/// Visible spans of a note `highway` under a curtain, top to bottom.
///
/// Notes scroll toward the judgement line at the bottom, so "bottom"
/// spans sit at high `y`. Returns at most two spans (reverse-dark
/// splits the highway); an empty highway yields no spans.
#[must_use]
pub fn visible_spans(curtain: LaneCurtain, highway: &Rect) -> Vec<Rect> {
    if highway.w == 0 || highway.h == 0 {
        return Vec::new();
    }
    let fifth = highway.h / CURTAIN_SHARE_DENOMINATOR;
    let span = |y: i32, h: u32| Rect {
        x: highway.x,
        y,
        w: highway.w,
        h,
    };
    match curtain {
        LaneCurtain::Full => vec![*highway],
        LaneCurtain::Hidden => {
            vec![span(highway.y, highway.h - fifth)]
        }
        LaneCurtain::Sudden => {
            vec![span(highway.y + highway.h as i32 - fifth as i32, fifth)]
        }
        LaneCurtain::Dark => {
            vec![span(highway.y + (2 * fifth) as i32, fifth)]
        }
        LaneCurtain::ReverseDark => vec![
            span(highway.y, 2 * fifth),
            span(highway.y + highway.h as i32 - (2 * fifth) as i32, 2 * fifth),
        ],
    }
}

/// Micro-font glyph width in pixels (digits only for now; the combo
/// counters, scores, and version stamps need no letters yet).
/// Glyphs are hand-authored below — no font assets involved.
pub const GLYPH_W: usize = 3;
/// Micro-font glyph height in pixels.
pub const GLYPH_H: usize = 5;
/// Horizontal advance per digit: glyph plus 1 px spacing.
pub const GLYPH_ADVANCE: usize = GLYPH_W + 1;

/// 3x5 bitmap rows for one decimal digit (bit 2 = left pixel).
/// `None` when out of range — callers draw nothing.
#[must_use]
pub const fn digit_glyph(digit: u8) -> Option<[u8; GLYPH_H]> {
    match digit {
        0 => Some([0b111, 0b101, 0b101, 0b101, 0b111]),
        1 => Some([0b010, 0b110, 0b010, 0b010, 0b111]),
        2 => Some([0b111, 0b001, 0b111, 0b100, 0b111]),
        3 => Some([0b111, 0b001, 0b111, 0b001, 0b111]),
        4 => Some([0b101, 0b101, 0b111, 0b001, 0b001]),
        5 => Some([0b111, 0b100, 0b111, 0b001, 0b111]),
        6 => Some([0b111, 0b100, 0b111, 0b101, 0b111]),
        7 => Some([0b111, 0b001, 0b010, 0b010, 0b010]),
        8 => Some([0b111, 0b101, 0b111, 0b101, 0b111]),
        9 => Some([0b111, 0b101, 0b111, 0b001, 0b111]),
        _ => None,
    }
}

/// 3x5 bitmap rows for one ASCII letter or mark (bit 2 = left pixel).
/// Lowercase maps to upper; `None` means undrawable (drawn blank).
#[must_use]
pub const fn letter_glyph(ch: u8) -> Option<[u8; GLYPH_H]> {
    match ch {
        b'A' => Some([0b010, 0b101, 0b111, 0b101, 0b101]),
        b'B' => Some([0b110, 0b101, 0b110, 0b101, 0b110]),
        b'C' => Some([0b011, 0b100, 0b100, 0b100, 0b011]),
        b'D' => Some([0b110, 0b101, 0b101, 0b101, 0b110]),
        b'E' => Some([0b111, 0b100, 0b110, 0b100, 0b111]),
        b'F' => Some([0b111, 0b100, 0b110, 0b100, 0b100]),
        b'G' => Some([0b011, 0b100, 0b101, 0b101, 0b011]),
        b'H' => Some([0b101, 0b101, 0b111, 0b101, 0b101]),
        b'I' => Some([0b111, 0b010, 0b010, 0b010, 0b111]),
        b'J' => Some([0b001, 0b001, 0b001, 0b101, 0b010]),
        b'K' => Some([0b101, 0b101, 0b110, 0b101, 0b101]),
        b'L' => Some([0b100, 0b100, 0b100, 0b100, 0b111]),
        b'M' => Some([0b101, 0b111, 0b111, 0b101, 0b101]),
        b'N' => Some([0b110, 0b101, 0b101, 0b101, 0b101]),
        b'O' => Some([0b010, 0b101, 0b101, 0b101, 0b010]),
        b'P' => Some([0b110, 0b101, 0b110, 0b100, 0b100]),
        b'Q' => Some([0b010, 0b101, 0b101, 0b110, 0b011]),
        b'R' => Some([0b110, 0b101, 0b110, 0b101, 0b101]),
        b'S' => Some([0b011, 0b100, 0b010, 0b001, 0b110]),
        b'T' => Some([0b111, 0b010, 0b010, 0b010, 0b010]),
        b'U' => Some([0b101, 0b101, 0b101, 0b101, 0b111]),
        b'V' => Some([0b101, 0b101, 0b101, 0b101, 0b010]),
        b'W' => Some([0b101, 0b101, 0b111, 0b111, 0b101]),
        b'X' => Some([0b101, 0b101, 0b010, 0b101, 0b101]),
        b'Y' => Some([0b101, 0b101, 0b010, 0b010, 0b010]),
        b'Z' => Some([0b111, 0b001, 0b010, 0b100, 0b111]),
        b' ' => Some([0b000, 0b000, 0b000, 0b000, 0b000]),
        b'-' => Some([0b000, 0b000, 0b111, 0b000, 0b000]),
        b'.' => Some([0b000, 0b000, 0b000, 0b000, 0b010]),
        b':' => Some([0b000, 0b010, 0b000, 0b010, 0b000]),
        b'!' => Some([0b010, 0b010, 0b010, 0b000, 0b010]),
        b'/' => Some([0b001, 0b001, 0b010, 0b100, 0b100]),
        _ => None,
    }
}

/// Bitmap rows for one ASCII byte: digits, letters (either case),
/// space, and `-.:!/`. Anything else draws blank but still advances.
#[must_use]
pub const fn glyph_for(ch: u8) -> Option<[u8; GLYPH_H]> {
    if ch.is_ascii_digit() {
        return digit_glyph(ch - b'0');
    }
    letter_glyph(ch.to_ascii_uppercase())
}

/// Blit glyph rows at `(x, y)`; clips to the frame.
fn draw_rows(frame: &mut Frame<'_>, x: i32, y: i32, rows: [u8; GLYPH_H], color: u32) {
    if frame.buf.len() < frame.w * frame.h {
        return;
    }
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..GLYPH_W {
            if bits & (1 << (GLYPH_W - 1 - col)) == 0 {
                continue;
            }
            let (dx, dy) = (x + col as i32, y + row as i32);
            if dx < 0 || dy < 0 || dx >= frame.w as i32 || dy >= frame.h as i32 {
                continue;
            }
            frame.buf[dy as usize * frame.w + dx as usize] = color;
        }
    }
}

/// Draw one digit with its top-left at `(x, y)`; clips to the frame,
/// draws nothing for out-of-range digits.
pub fn draw_digit(frame: &mut Frame<'_>, x: i32, y: i32, digit: u8, color: u32) {
    let Some(rows) = digit_glyph(digit) else {
        return;
    };
    draw_rows(frame, x, y, rows, color);
}

/// Draw one ASCII character (`glyph_for` coverage) at `(x, y)`;
/// undrawable bytes leave a blank advance so layout stays stable.
pub fn draw_char(frame: &mut Frame<'_>, x: i32, y: i32, ch: u8, color: u32) {
    if let Some(rows) = glyph_for(ch) {
        draw_rows(frame, x, y, rows, color);
    }
}

/// Pixel width of `text` in bytes (no trailing spacing).
#[must_use]
pub fn text_width(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.len() * GLYPH_ADVANCE - 1
}

/// Draw ASCII `text` from `x` (top-left), [`GLYPH_ADVANCE`] spacing;
/// returns the x edge after the last byte.
pub fn draw_text(frame: &mut Frame<'_>, x: i32, y: i32, text: &str, color: u32) -> i32 {
    for (i, byte) in text.bytes().enumerate() {
        draw_char(frame, x + (i * GLYPH_ADVANCE) as i32, y, byte, color);
    }
    x + (text.len() * GLYPH_ADVANCE) as i32
}

/// Pixel width of `value` in digits (no trailing spacing).
#[must_use]
pub fn number_width(mut value: u32) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits * GLYPH_ADVANCE - 1
}

/// Draw `value` in decimal from `x` (top-left), digits spaced by
/// [`GLYPH_ADVANCE`]; returns the x edge after the last digit.
pub fn draw_number(frame: &mut Frame<'_>, x: i32, y: i32, value: u32, color: u32) -> i32 {
    let mut digits = [0u8; 10];
    let mut count = 0;
    let mut rest = value;
    loop {
        digits[count] = (rest % 10) as u8;
        count += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    for (i, digit) in digits[..count].iter().rev().enumerate() {
        draw_digit(frame, x + (i * GLYPH_ADVANCE) as i32, y, *digit, color);
    }
    x + (count * GLYPH_ADVANCE) as i32
}

/// Native 7.55 canvas every `.bnd` rect addresses (see
/// `docs/file-formats/opi.md`): 800x600 fullscreen.
pub const NATIVE_WIDTH: u32 = 800;
/// Native canvas height in pixels.
pub const NATIVE_HEIGHT: u32 = 600;

/// One `ControlList` entry bound to its `.bnd` rect on the native
/// canvas: rect `i` lays out entry `i + 1` (entry 0 is the block
/// itself). Callers pass the entries *after* the block header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedControl {
    /// Control code (`IDC_` second word, top byte names the state).
    pub code: u32,
    /// Native-canvas layout rect from the `.bnd` file.
    pub rect: Rect,
    /// Whether the entry names a sprite body.
    pub has_sprite: bool,
}

/// Bind entry codes to layout rects in file order, truncating to
/// the shortest input (the Playing subset ships fewer rects than
/// entries, so extra entries ride unlaid-out).
#[must_use]
pub fn bind_controls(codes: &[u32], has_sprite: &[bool], rects: &[Rect]) -> Vec<PlacedControl> {
    codes
        .iter()
        .zip(has_sprite.iter())
        .zip(rects.iter())
        .map(|((code, sprite), rect)| PlacedControl {
            code: *code,
            rect: *rect,
            has_sprite: *sprite,
        })
        .collect()
}

/// Build a frame rect from absolute corners, saturating inverted
/// edges to an empty rect at the top-left corner.
#[must_use]
pub fn rect_from_corners(x1: u32, y1: u32, x2: u32, y2: u32) -> Rect {
    let (x, y) = (x1.min(x2), y1.min(y2));
    Rect {
        x: x.min(i32::MAX as u32) as i32,
        y: y.min(i32::MAX as u32) as i32,
        w: x2.saturating_sub(x1),
        h: y2.saturating_sub(y1),
    }
}

/// Scale one native axis into frame pixels.
fn scale_axis(value: i32, frame_len: usize, native_len: u32) -> i32 {
    (i64::from(value) * frame_len as i64 / i64::from(native_len)) as i32
}

/// Scale a native-canvas rect into frame pixels (identity on
/// 800x600, proportional on headless test buffers).
#[must_use]
pub fn scale_rect_to_frame(rect: &Rect, frame_w: usize, frame_h: usize) -> Rect {
    Rect {
        x: scale_axis(rect.x, frame_w, NATIVE_WIDTH),
        y: scale_axis(rect.y, frame_h, NATIVE_HEIGHT),
        w: scale_extent(rect.w, frame_w, NATIVE_WIDTH),
        h: scale_extent(rect.h, frame_h, NATIVE_HEIGHT),
    }
}

/// Scale one native extent into frame pixels.
fn scale_extent(value: u32, frame_len: usize, native_len: u32) -> u32 {
    (u64::from(value) * frame_len as u64 / u64::from(native_len)) as u32
}

/// Center an `inner_w` x `inner_h` box in the frame.
#[must_use]
pub fn center_rect(inner_w: usize, inner_h: usize, frame_w: usize, frame_h: usize) -> Rect {
    Rect {
        x: frame_w.saturating_sub(inner_w) as i32 / 2,
        y: frame_h.saturating_sub(inner_h) as i32 / 2,
        w: inner_w.min(frame_w) as u32,
        h: inner_h.min(frame_h) as u32,
    }
}

/// Blit an RGB555 image stretched into `dest`, skipping `colorkey`
/// words. A size match is a plain 1:1 blit; otherwise each dest
/// pixel samples its nearest source pixel.
#[allow(clippy::cast_possible_wrap)]
pub fn blit_image_in_rect(frame: &mut Frame<'_>, image: &Rgb555Image, colorkey: u16, dest: &Rect) {
    if !blit_target_ready(frame, image, dest) {
        return;
    }
    for row in 0..dest.h as usize {
        for col in 0..dest.w as usize {
            blit_cover_pixel(frame, image, colorkey, dest, row, col);
        }
    }
}

/// Whether a cover blit has nonzero geometry and backing stores.
fn blit_target_ready(frame: &Frame<'_>, image: &Rgb555Image, dest: &Rect) -> bool {
    image.w > 0
        && image.h > 0
        && dest.w > 0
        && dest.h > 0
        && frame.buf.len() >= frame.w * frame.h
        && image.words.len() >= image.w * image.h
}

/// Blit one cover-blit pixel with nearest-neighbor sampling.
#[allow(clippy::cast_possible_wrap)]
fn blit_cover_pixel(
    frame: &mut Frame<'_>,
    image: &Rgb555Image,
    colorkey: u16,
    dest: &Rect,
    row: usize,
    col: usize,
) {
    let dest_x = dest.x + col as i32;
    let dest_y = dest.y + row as i32;
    if dest_x < 0 || dest_y < 0 || dest_x >= frame.w as i32 || dest_y >= frame.h as i32 {
        return;
    }
    let word = image.words[stretch_index(row, dest.h as usize, image.h) * image.w
        + stretch_index(col, dest.w as usize, image.w)];
    if word == colorkey {
        return;
    }
    let (r, g, b) = expand_rgb555(word);
    frame.buf[dest_y as usize * frame.w + dest_x as usize] = pack_xrgb(r, g, b);
}

/// One sprite layer painted into a scene frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneLayer<'a> {
    /// Decoded sprite pixels.
    pub image: &'a Rgb555Image,
    /// Transparent word of the source body.
    pub colorkey: u16,
    /// Frame-space destination (cover-blitted when sizes differ).
    pub dest: Rect,
}

/// Fill `background`, then paint layers back to front. Later layers
/// cover earlier ones; colorkey pixels leave the canvas untouched.
pub fn paint_scene_layers(frame: &mut Frame<'_>, background: u32, layers: &[SceneLayer<'_>]) {
    fill_frame(frame, background);
    for layer in layers {
        blit_image_in_rect(frame, layer.image, layer.colorkey, &layer.dest);
    }
}

/// Fill the whole frame when it has backing pixels.
fn fill_frame(frame: &mut Frame<'_>, background: u32) {
    let (w, h) = (frame.w, frame.h);
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
}

/// Maximum alpha for blending: fully opaque source.
const ALPHA_OPAQUE: u16 = 255;

/// Unpack an XRGB8888 word into 8-bit channels.
fn unpack_xrgb(pixel: u32) -> (u8, u8, u8) {
    (
        u8::try_from((pixel >> 16) & 0xFF).unwrap_or(u8::MAX),
        u8::try_from((pixel >> 8) & 0xFF).unwrap_or(u8::MAX),
        u8::try_from(pixel & 0xFF).unwrap_or(u8::MAX),
    )
}

/// Blend one 8-bit channel: alpha 0 keeps dst, 255 takes src.
fn blend_channel(src: u8, dst: u8, alpha: u8) -> u8 {
    let mixed = u16::from(src) * u16::from(alpha)
        + u16::from(dst) * (ALPHA_OPAQUE - u16::from(alpha))
        + 127;
    u8::try_from(mixed / ALPHA_OPAQUE).unwrap_or(u8::MAX)
}

/// Blend src over dst with byte alpha (0 = transparent, 255 = opaque).
#[must_use]
pub fn blend_xrgb(src: u32, dst: u32, alpha: u8) -> u32 {
    if alpha == u8::MAX {
        return src;
    }
    if alpha == 0 {
        return dst;
    }
    let (sr, sg, sb) = unpack_xrgb(src);
    let (dr, dg, db) = unpack_xrgb(dst);
    pack_xrgb(
        blend_channel(sr, dr, alpha),
        blend_channel(sg, dg, alpha),
        blend_channel(sb, db, alpha),
    )
}

/// Fill a clipped rectangle with a translucent color.
/// Alpha 0 paints nothing; 255 is a solid fill_rect.
pub fn fill_rect_alpha(frame: &mut Frame<'_>, rect: &Rect, color: u32, alpha: u8) {
    if alpha == 0 {
        return;
    }
    if alpha == u8::MAX {
        fill_rect(frame, rect, color);
        return;
    }
    if frame.buf.len() < frame.w * frame.h {
        return;
    }
    let Some((x0, y0, x1, y1)) = clip(rect, frame.w, frame.h) else {
        return;
    };
    for row in frame.buf[y0 * frame.w..y1 * frame.w].chunks_exact_mut(frame.w) {
        for pixel in &mut row[x0..x1] {
            *pixel = blend_xrgb(color, *pixel, alpha);
        }
    }
}

/// Whether a scaled sprite blit has geometry and backing pixels.
fn scaled_blit_ready(frame: &Frame<'_>, sprite: &Sprite<'_>, dest: &Rect) -> bool {
    dest.w > 0
        && dest.h > 0
        && sprite.w > 0
        && sprite.h > 0
        && sprite.stride >= sprite.w
        && frame.buf.len() >= frame.w * frame.h
        && sprite.pixels.len() >= sprite.stride * sprite.h
}

/// One nearest-neighbor sample of a scaled sprite blit.
#[derive(Debug, Clone, Copy)]
struct ScaledPixel {
    col: usize,
    row: usize,
    dest_w: usize,
    dest_h: usize,
    colorkey: Option<u16>,
}

/// Frame position of one scaled sample, or None on overflow.
fn scaled_dest_xy(dest: &Rect, col: usize, row: usize) -> Option<(i32, i32)> {
    let x = dest.x.checked_add(i32::try_from(col).unwrap_or(i32::MAX))?;
    let y = dest.y.checked_add(i32::try_from(row).unwrap_or(i32::MAX))?;
    Some((x, y))
}

/// Whether a frame position lies inside the frame.
fn dest_in_frame(frame: &Frame<'_>, dest_x: i32, dest_y: i32) -> bool {
    dest_x >= 0 && dest_y >= 0 && dest_x < frame.w as i32 && dest_y < frame.h as i32
}

/// Paint one nearest-neighbor pixel of a scaled sprite blit.
fn paint_scaled_pixel(frame: &mut Frame<'_>, sprite: &Sprite<'_>, dest: &Rect, pixel: ScaledPixel) {
    let Some((dest_x, dest_y)) = scaled_dest_xy(dest, pixel.col, pixel.row) else {
        return;
    };
    if !dest_in_frame(frame, dest_x, dest_y) {
        return;
    }
    let word = sprite.pixels[stretch_index(pixel.row, pixel.dest_h, sprite.h) * sprite.stride
        + stretch_index(pixel.col, pixel.dest_w, sprite.w)];
    if pixel.colorkey == Some(word) {
        return;
    }
    let (r, g, b) = expand_rgb555(word);
    frame.buf[dest_y as usize * frame.w + dest_x as usize] = pack_xrgb(r, g, b);
}

/// Blit a sprite stretched over dest, skipping colorkey pixels.
/// Same-size dest is a plain blit; otherwise nearest-neighbor scale.
pub fn blit_rgb555_scaled(
    frame: &mut Frame<'_>,
    sprite: &Sprite<'_>,
    dest: &Rect,
    colorkey: Option<u16>,
) {
    if !scaled_blit_ready(frame, sprite, dest) {
        return;
    }
    let dest_w = usize::try_from(dest.w).unwrap_or(0);
    let dest_h = usize::try_from(dest.h).unwrap_or(0);
    for row in 0..dest_h {
        for col in 0..dest_w {
            paint_scaled_pixel(
                frame,
                sprite,
                dest,
                ScaledPixel {
                    col,
                    row,
                    dest_w,
                    dest_h,
                    colorkey,
                },
            );
        }
    }
}

/// Paint one scaled sprite pixel blended over the frame.
fn paint_scaled_alpha_pixel(
    frame: &mut Frame<'_>,
    sprite: &Sprite<'_>,
    dest: &Rect,
    pixel: ScaledPixel,
    alpha: u8,
) {
    let Some((dest_x, dest_y)) = scaled_dest_xy(dest, pixel.col, pixel.row) else {
        return;
    };
    if !dest_in_frame(frame, dest_x, dest_y) {
        return;
    }
    let word = sprite.pixels[stretch_index(pixel.row, pixel.dest_h, sprite.h) * sprite.stride
        + stretch_index(pixel.col, pixel.dest_w, sprite.w)];
    if pixel.colorkey == Some(word) {
        return;
    }
    let (r, g, b) = expand_rgb555(word);
    let at = dest_y as usize * frame.w + dest_x as usize;
    frame.buf[at] = blend_xrgb(pack_xrgb(r, g, b), frame.buf[at], alpha);
}

/// Blit a sprite stretched over dest with global alpha over the frame.
/// Alpha 0 paints nothing; 255 is opaque.
pub fn blit_rgb555_scaled_alpha(
    frame: &mut Frame<'_>,
    sprite: &Sprite<'_>,
    dest: &Rect,
    colorkey: Option<u16>,
    alpha: u8,
) {
    if alpha == 0 || !scaled_blit_ready(frame, sprite, dest) {
        return;
    }
    if alpha == u8::MAX {
        blit_rgb555_scaled(frame, sprite, dest, colorkey);
        return;
    }
    let dest_w = usize::try_from(dest.w).unwrap_or(0);
    let dest_h = usize::try_from(dest.h).unwrap_or(0);
    for row in 0..dest_h {
        for col in 0..dest_w {
            paint_scaled_alpha_pixel(
                frame,
                sprite,
                dest,
                ScaledPixel {
                    col,
                    row,
                    dest_w,
                    dest_h,
                    colorkey,
                },
                alpha,
            );
        }
    }
}

/// Blit an image stretched into dest with colorkey skip and global alpha.
/// Alpha 0 paints nothing; 255 matches blit_image_in_rect.
pub fn blit_image_in_rect_alpha(
    frame: &mut Frame<'_>,
    image: &Rgb555Image,
    colorkey: u16,
    dest: &Rect,
    alpha: u8,
) {
    if alpha == 0 || !blit_target_ready(frame, image, dest) {
        return;
    }
    if alpha == u8::MAX {
        blit_image_in_rect(frame, image, colorkey, dest);
        return;
    }
    let dest_w = usize::try_from(dest.w).unwrap_or(0);
    let dest_h = usize::try_from(dest.h).unwrap_or(0);
    for row in 0..dest_h {
        for col in 0..dest_w {
            let Some(dest_x) = dest.x.checked_add(i32::try_from(col).unwrap_or(i32::MAX)) else {
                continue;
            };
            let Some(dest_y) = dest.y.checked_add(i32::try_from(row).unwrap_or(i32::MAX)) else {
                continue;
            };
            if dest_x < 0 || dest_y < 0 || dest_x >= frame.w as i32 || dest_y >= frame.h as i32 {
                continue;
            }
            let word = image.words[stretch_index(row, dest_h, image.h) * image.w
                + stretch_index(col, dest_w, image.w)];
            if word == colorkey {
                continue;
            }
            let (r, g, b) = expand_rgb555(word);
            let at = dest_y as usize * frame.w + dest_x as usize;
            frame.buf[at] = blend_xrgb(pack_xrgb(r, g, b), frame.buf[at], alpha);
        }
    }
}

/// Large glyph width in pixels: 5-wide titles and HUD digits.
pub const BIG_GLYPH_W: usize = 5;
/// Large glyph height in pixels.
pub const BIG_GLYPH_H: usize = 7;
/// Horizontal advance per large glyph: glyph plus 1 px spacing.
pub const BIG_GLYPH_ADVANCE: usize = BIG_GLYPH_W + 1;

/// 5x7 bitmap rows for one decimal digit (bit 4 = left pixel).
#[must_use]
pub const fn big_digit_glyph(digit: u8) -> Option<[u8; BIG_GLYPH_H]> {
    match digit {
        0 => Some([
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ]),
        1 => Some([
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ]),
        2 => Some([
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ]),
        3 => Some([
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ]),
        4 => Some([
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ]),
        5 => Some([
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ]),
        6 => Some([
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ]),
        7 => Some([
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ]),
        8 => Some([
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ]),
        9 => Some([
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ]),
        _ => None,
    }
}

/// 5x7 bitmap rows for one ASCII letter or mark (bit 4 = left pixel).
#[must_use]
pub const fn big_letter_glyph(ch: u8) -> Option<[u8; BIG_GLYPH_H]> {
    match ch {
        b'A' => Some([
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
        b'B' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ]),
        b'C' => Some([
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ]),
        b'D' => Some([
            0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100,
        ]),
        b'E' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ]),
        b'F' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
        b'G' => Some([
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ]),
        b'H' => Some([
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
        b'I' => Some([
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ]),
        b'J' => Some([
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ]),
        b'K' => Some([
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ]),
        b'L' => Some([
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10001, 0b11111,
        ]),
        b'M' => Some([
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ]),
        b'N' => Some([
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ]),
        b'O' => Some([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        b'P' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
        b'Q' => Some([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ]),
        b'R' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ]),
        b'S' => Some([
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
        b'T' => Some([
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
        b'U' => Some([
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        b'V' => Some([
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ]),
        b'W' => Some([
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ]),
        b'X' => Some([
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ]),
        b'Y' => Some([
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
        b'Z' => Some([
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ]),
        b' ' => Some([
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ]),
        b'-' => Some([
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ]),
        b'.' => Some([
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ]),
        b':' => Some([
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ]),
        b'!' => Some([
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100,
        ]),
        b'/' => Some([
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ]),
        _ => None,
    }
}

/// Bitmap rows for one ASCII byte in the large face: digits, letters
/// (either case), space, and marks. Else blank-but-advancing.
#[must_use]
pub const fn big_glyph_for(ch: u8) -> Option<[u8; BIG_GLYPH_H]> {
    if ch.is_ascii_digit() {
        return big_digit_glyph(ch - b'0');
    }
    big_letter_glyph(ch.to_ascii_uppercase())
}

/// Blit large glyph rows at (x, y); clips to the frame.
fn draw_big_rows(frame: &mut Frame<'_>, x: i32, y: i32, rows: [u8; BIG_GLYPH_H], color: u32) {
    if frame.buf.len() < frame.w * frame.h {
        return;
    }
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..BIG_GLYPH_W {
            if bits & (1 << (BIG_GLYPH_W - 1 - col)) == 0 {
                continue;
            }
            let Some(dx) = x.checked_add(i32::try_from(col).unwrap_or(i32::MAX)) else {
                continue;
            };
            let Some(dy) = y.checked_add(i32::try_from(row).unwrap_or(i32::MAX)) else {
                continue;
            };
            if dx < 0 || dy < 0 || dx >= frame.w as i32 || dy >= frame.h as i32 {
                continue;
            }
            frame.buf[dy as usize * frame.w + dx as usize] = color;
        }
    }
}

/// Draw one large digit; out-of-range digits draw nothing.
pub fn draw_big_digit(frame: &mut Frame<'_>, x: i32, y: i32, digit: u8, color: u32) {
    let Some(rows) = big_digit_glyph(digit) else {
        return;
    };
    draw_big_rows(frame, x, y, rows, color);
}

/// Draw one large ASCII character (big_glyph_for coverage).
pub fn draw_big_char(frame: &mut Frame<'_>, x: i32, y: i32, ch: u8, color: u32) {
    if let Some(rows) = big_glyph_for(ch) {
        draw_big_rows(frame, x, y, rows, color);
    }
}

/// Pixel width of text in the large face (no trailing spacing).
#[must_use]
pub fn big_text_width(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.len() * BIG_GLYPH_ADVANCE - 1
}

/// Draw ASCII text in the large face; returns the x edge after it.
pub fn draw_big_text(frame: &mut Frame<'_>, x: i32, y: i32, text: &str, color: u32) -> i32 {
    for (i, byte) in text.bytes().enumerate() {
        let Some(dx) = x.checked_add(i32::try_from(i * BIG_GLYPH_ADVANCE).unwrap_or(i32::MAX))
        else {
            break;
        };
        draw_big_char(frame, dx, y, byte, color);
    }
    x.checked_add(i32::try_from(text.len() * BIG_GLYPH_ADVANCE).unwrap_or(i32::MAX))
        .unwrap_or(i32::MAX)
}

/// Pixel width of value in large digits (no trailing spacing).
#[must_use]
pub fn big_number_width(mut value: u32) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits * BIG_GLYPH_ADVANCE - 1
}

/// Draw value in decimal in the large face; returns the x edge.
pub fn draw_big_number(frame: &mut Frame<'_>, x: i32, y: i32, value: u32, color: u32) -> i32 {
    let mut digits = [0u8; 10];
    let mut count = 0;
    let mut rest = value;
    loop {
        digits[count] = u8::try_from(rest % 10).unwrap_or(0);
        count += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    for (i, digit) in digits[..count].iter().rev().enumerate() {
        let Some(dx) = x.checked_add(i32::try_from(i * BIG_GLYPH_ADVANCE).unwrap_or(i32::MAX))
        else {
            break;
        };
        draw_big_digit(frame, dx, y, *digit, color);
    }
    x.checked_add(i32::try_from(count * BIG_GLYPH_ADVANCE).unwrap_or(i32::MAX))
        .unwrap_or(i32::MAX)
}

/// Interior of a panel: rect inset by border_width on every side.
/// Oversized borders collapse to an empty rect at the panel origin.
#[must_use]
pub fn panel_content_rect(rect: &Rect, border_width: u32) -> Rect {
    let inset = i32::try_from(border_width).unwrap_or(i32::MAX);
    let shrink = border_width.saturating_mul(2);
    let Some(x) = rect.x.checked_add(inset) else {
        return Rect {
            x: rect.x,
            y: rect.y,
            w: 0,
            h: 0,
        };
    };
    let Some(y) = rect.y.checked_add(inset) else {
        return Rect {
            x: rect.x,
            y: rect.y,
            w: 0,
            h: 0,
        };
    };
    Rect {
        x,
        y,
        w: rect.w.saturating_sub(shrink),
        h: rect.h.saturating_sub(shrink),
    }
}

/// Paint a border ring of border_width px over rect.
/// Width 0 paints nothing; oversized widths fill the whole rect.
pub fn draw_rect_border(frame: &mut Frame<'_>, rect: &Rect, color: u32, border_width: u32) {
    if border_width == 0 || rect.w == 0 || rect.h == 0 {
        return;
    }
    let edge = border_width.min(rect.w).min(rect.h);
    let Some(bottom_y) = rect
        .y
        .checked_add(i32::try_from(rect.h - edge).unwrap_or(0))
    else {
        return;
    };
    let Some(right_x) = rect
        .x
        .checked_add(i32::try_from(rect.w - edge).unwrap_or(0))
    else {
        return;
    };
    let sides = [
        Rect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: edge,
        },
        Rect {
            x: rect.x,
            y: bottom_y,
            w: rect.w,
            h: edge,
        },
        Rect {
            x: rect.x,
            y: rect.y,
            w: edge,
            h: rect.h,
        },
        Rect {
            x: right_x,
            y: rect.y,
            w: edge,
            h: rect.h,
        },
    ];
    for side in &sides {
        fill_rect(frame, side, color);
    }
}

/// Paint a panel: optional solid fill plus an opaque border ring.
pub fn draw_panel(
    frame: &mut Frame<'_>,
    rect: &Rect,
    fill: Option<u32>,
    border: u32,
    border_width: u32,
) {
    if let Some(color) = fill {
        fill_rect(frame, rect, color);
    }
    draw_rect_border(frame, rect, border, border_width);
}

/// Paint a panel with a translucent fill plus an opaque border ring.
/// Fill alpha 0 skips the fill; 255 is a solid draw_panel.
pub fn draw_panel_alpha(
    frame: &mut Frame<'_>,
    rect: &Rect,
    fill: u32,
    alpha: u8,
    border: u32,
    border_width: u32,
) {
    if alpha == u8::MAX {
        draw_panel(frame, rect, Some(fill), border, border_width);
        return;
    }
    if alpha != 0 {
        fill_rect_alpha(frame, rect, fill, alpha);
    }
    draw_rect_border(frame, rect, border, border_width);
}

#[cfg(test)]
mod fidelity_tests {
    use super::*;

    #[test]
    fn blend_edges_keep_endpoints() {
        let src = pack_xrgb(0xFF, 0x00, 0x00);
        let dst = pack_xrgb(0x00, 0x00, 0xFF);
        assert_eq!(blend_xrgb(src, dst, 0), dst);
        assert_eq!(blend_xrgb(src, dst, 255), src);
    }

    #[test]
    fn blend_half_mixes_channels() {
        let src = pack_xrgb(0xFF, 0xFF, 0xFF);
        assert_eq!(blend_xrgb(src, 0x000000, 128), pack_xrgb(0x80, 0x80, 0x80));
    }

    #[test]
    fn translucent_fill_blends_inside_only() {
        let mut buf = vec![pack_xrgb(0x00, 0x00, 0xFF); 4 * 4];
        let mut frame = Frame {
            buf: &mut buf,
            w: 4,
            h: 4,
        };
        fill_rect_alpha(
            &mut frame,
            &Rect {
                x: 1,
                y: 1,
                w: 2,
                h: 2,
            },
            pack_xrgb(0xFF, 0x00, 0x00),
            255,
        );
        assert_eq!(frame.buf[5], pack_xrgb(0xFF, 0x00, 0x00));
        assert_eq!(frame.buf[0], pack_xrgb(0x00, 0x00, 0xFF));
        fill_rect_alpha(
            &mut frame,
            &Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 4,
            },
            pack_xrgb(0x00, 0x00, 0x00),
            0,
        );
        assert_eq!(frame.buf[0], pack_xrgb(0x00, 0x00, 0xFF));
    }

    #[test]
    fn half_fill_blends_toward_source() {
        let mut buf = vec![0x000000u32; 1];
        let mut frame = Frame {
            buf: &mut buf,
            w: 1,
            h: 1,
        };
        fill_rect_alpha(
            &mut frame,
            &Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            pack_xrgb(0xFF, 0xFF, 0xFF),
            128,
        );
        assert_eq!(frame.buf[0], pack_xrgb(0x80, 0x80, 0x80));
    }

    #[test]
    fn scaled_blit_upscales_single_pixel() {
        let sprite = Sprite {
            pixels: &[0x7C00],
            w: 1,
            h: 1,
            stride: 1,
        };
        let mut buf = vec![0u32; 2 * 2];
        blit_rgb555_scaled(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 2,
            },
            &sprite,
            &Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            },
            None,
        );
        assert_eq!(buf, vec![pack_xrgb(255, 0, 0); 4]);
    }

    #[test]
    fn scaled_blit_skips_colorkey_and_clips() {
        let sprite = Sprite {
            pixels: &[0x7C00, 0x0000],
            w: 2,
            h: 1,
            stride: 2,
        };
        let mut buf = vec![0x123456u32; 4];
        blit_rgb555_scaled(
            &mut Frame {
                buf: &mut buf,
                w: 4,
                h: 1,
            },
            &sprite,
            &Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 1,
            },
            Some(0x0000),
        );
        let red = pack_xrgb(255, 0, 0);
        assert_eq!(buf, vec![red, red, 0x123456, 0x123456]);
    }

    #[test]
    fn scaled_blit_ignores_empty_geometry() {
        let sprite = Sprite {
            pixels: &[0x7C00],
            w: 1,
            h: 1,
            stride: 1,
        };
        let mut buf = vec![0x123456u32; 1];
        blit_rgb555_scaled(
            &mut Frame {
                buf: &mut buf,
                w: 1,
                h: 1,
            },
            &sprite,
            &Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 1,
            },
            None,
        );
        assert_eq!(buf, vec![0x123456u32]);
    }

    #[test]
    fn scaled_alpha_blends_toward_sprite() {
        let sprite = Sprite {
            pixels: &[0x7FFF],
            w: 1,
            h: 1,
            stride: 1,
        };
        let mut buf = vec![0x000000u32; 1];
        blit_rgb555_scaled_alpha(
            &mut Frame {
                buf: &mut buf,
                w: 1,
                h: 1,
            },
            &sprite,
            &Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            None,
            128,
        );
        assert_eq!(buf[0], pack_xrgb(0x80, 0x80, 0x80));
    }

    #[test]
    fn scaled_alpha_zero_paints_nothing() {
        let sprite = Sprite {
            pixels: &[0x7FFF],
            w: 1,
            h: 1,
            stride: 1,
        };
        let mut buf = vec![0x123456u32; 1];
        blit_rgb555_scaled_alpha(
            &mut Frame {
                buf: &mut buf,
                w: 1,
                h: 1,
            },
            &sprite,
            &Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            None,
            0,
        );
        assert_eq!(buf, vec![0x123456u32]);
    }

    #[test]
    fn cover_alpha_matches_opaque_cover() {
        let image = Rgb555Image::decode(&[0x00, 0x7C, 0xE0, 0x03], 2, 1).unwrap();
        let mut plain = vec![0u32; 2];
        let mut faded = vec![0u32; 2];
        let dest = Rect {
            x: 0,
            y: 0,
            w: 2,
            h: 1,
        };
        blit_image_in_rect(
            &mut Frame {
                buf: &mut plain,
                w: 2,
                h: 1,
            },
            &image,
            0xFFFF,
            &dest,
        );
        blit_image_in_rect_alpha(
            &mut Frame {
                buf: &mut faded,
                w: 2,
                h: 1,
            },
            &image,
            0xFFFF,
            &dest,
            255,
        );
        assert_eq!(plain, faded);
    }

    #[test]
    fn big_digits_match_their_bitmaps() {
        for digit in 0..10u8 {
            let mut buf = vec![0u32; BIG_GLYPH_W * BIG_GLYPH_H];
            let mut frame = Frame {
                buf: &mut buf,
                w: BIG_GLYPH_W,
                h: BIG_GLYPH_H,
            };
            draw_big_digit(&mut frame, 0, 0, digit, 0xFFFFFF);
            let rows = big_digit_glyph(digit).unwrap();
            for (y, bits) in rows.iter().enumerate() {
                for x in 0..BIG_GLYPH_W {
                    let lit = bits & (1 << (BIG_GLYPH_W - 1 - x)) != 0;
                    assert_eq!(frame.buf[y * BIG_GLYPH_W + x] != 0, lit);
                }
            }
        }
        assert_eq!(big_digit_glyph(10), None);
    }

    #[test]
    fn big_letters_match_their_bitmaps() {
        for ch in b'A'..=b'Z' {
            let mut buf = vec![0u32; BIG_GLYPH_W * BIG_GLYPH_H];
            let mut frame = Frame {
                buf: &mut buf,
                w: BIG_GLYPH_W,
                h: BIG_GLYPH_H,
            };
            draw_big_char(&mut frame, 0, 0, ch, 0xFFFFFF);
            let rows = big_letter_glyph(ch).unwrap();
            for (y, bits) in rows.iter().enumerate() {
                for x in 0..BIG_GLYPH_W {
                    let lit = bits & (1 << (BIG_GLYPH_W - 1 - x)) != 0;
                    assert_eq!(frame.buf[y * BIG_GLYPH_W + x] != 0, lit);
                }
            }
        }
    }

    #[test]
    fn big_text_lays_out_with_spacing() {
        let hi = [72u8, 73u8, 33u8];
        let text = core::str::from_utf8(&hi).unwrap();
        assert_eq!(big_text_width(text), 3 * BIG_GLYPH_ADVANCE - 1);
        let mut buf = vec![0u32; 20 * BIG_GLYPH_H];
        let mut frame = Frame {
            buf: &mut buf,
            w: 20,
            h: BIG_GLYPH_H,
        };
        let end = draw_big_text(&mut frame, 0, 0, text, 0xFFFFFF);
        assert_eq!(end, 18);
        assert_eq!(frame.buf[0], 0xFFFFFF);
        assert_eq!(big_glyph_for(97), big_glyph_for(65));
        assert_eq!(big_glyph_for(64), None);
    }

    #[test]
    fn big_number_renders_without_leading_zeros() {
        assert_eq!(big_number_width(0), BIG_GLYPH_W);
        assert_eq!(big_number_width(507), 3 * BIG_GLYPH_ADVANCE - 1);
        let mut buf = vec![0u32; 20 * BIG_GLYPH_H];
        let mut frame = Frame {
            buf: &mut buf,
            w: 20,
            h: BIG_GLYPH_H,
        };
        let end = draw_big_number(&mut frame, 0, 0, 507, 0xFFFFFF);
        assert_eq!(end, 18);
        assert_eq!(frame.buf[0], 0xFFFFFF);
    }

    #[test]
    fn rect_border_paints_only_the_ring() {
        let mut buf = vec![0u32; 4 * 4];
        let mut frame = Frame {
            buf: &mut buf,
            w: 4,
            h: 4,
        };
        draw_rect_border(
            &mut frame,
            &Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 4,
            },
            0xFFFFFF,
            1,
        );
        assert_eq!(frame.buf[0], 0xFFFFFF);
        assert_eq!(frame.buf[5], 0);
        assert_eq!(frame.buf[3 * 4 + 3], 0xFFFFFF);
        draw_rect_border(
            &mut frame,
            &Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 4,
            },
            0xFFFFFF,
            0,
        );
        assert_eq!(frame.buf[5], 0);
    }

    #[test]
    fn panel_content_insets_by_border() {
        let rect = Rect {
            x: 2,
            y: 2,
            w: 10,
            h: 8,
        };
        assert_eq!(
            panel_content_rect(&rect, 1),
            Rect {
                x: 3,
                y: 3,
                w: 8,
                h: 6
            }
        );
        assert_eq!(panel_content_rect(&rect, 99).w, 0);
    }

    #[test]
    fn panel_paints_fill_with_opaque_border() {
        let mut buf = vec![0u32; 5 * 5];
        let mut frame = Frame {
            buf: &mut buf,
            w: 5,
            h: 5,
        };
        draw_panel(
            &mut frame,
            &Rect {
                x: 0,
                y: 0,
                w: 5,
                h: 5,
            },
            Some(0x112233),
            0xFFFFFF,
            1,
        );
        assert_eq!(frame.buf[0], 0xFFFFFF);
        assert_eq!(frame.buf[2 * 5 + 2], 0x112233);
    }

    #[test]
    fn translucent_panel_blends_fill_keeps_border() {
        let mut buf = vec![0x000000u32; 3 * 3];
        let mut frame = Frame {
            buf: &mut buf,
            w: 3,
            h: 3,
        };
        draw_panel_alpha(
            &mut frame,
            &Rect {
                x: 0,
                y: 0,
                w: 3,
                h: 3,
            },
            pack_xrgb(0xFF, 0xFF, 0xFF),
            128,
            pack_xrgb(0xFF, 0x00, 0x00),
            1,
        );
        assert_eq!(frame.buf[0], pack_xrgb(0xFF, 0x00, 0x00));
        assert_eq!(frame.buf[4], pack_xrgb(0x80, 0x80, 0x80));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_maps_primaries() {
        assert_eq!(expand_rgb555(0x7C00), (255, 0, 0));
        assert_eq!(expand_rgb555(0x03E0), (0, 255, 0));
        assert_eq!(expand_rgb555(0x001F), (0, 0, 255));
        assert_eq!(expand_rgb555(0x0000), (0, 0, 0));
    }

    #[test]
    fn fill_clips_to_frame() {
        let mut buf = vec![0u32; 4 * 4];
        let mut frame = Frame {
            buf: &mut buf,
            w: 4,
            h: 4,
        };
        fill_rect(
            &mut frame,
            &Rect {
                x: -1,
                y: -1,
                w: 3,
                h: 3,
            },
            0xFFFFFF,
        );
        assert_eq!(frame.buf[0], 0xFFFFFF);
        assert_eq!(frame.buf[5], 0xFFFFFF);
        assert_eq!(frame.buf[10], 0);
        assert_eq!(frame.buf[15], 0);
    }

    #[test]
    fn blit_skips_colorkey_and_clips() {
        // 2x2 sprite: red, key / green, blue.
        let sprite = Sprite {
            pixels: &[0x7C00, 0x0000, 0x03E0, 0x001F],
            w: 2,
            h: 2,
            stride: 2,
        };
        let mut buf = vec![0x123456u32; 3 * 3];
        let mut frame = Frame {
            buf: &mut buf,
            w: 3,
            h: 3,
        };
        blit_rgb555(&mut frame, &sprite, 1, 1, Some(0x0000));
        assert_eq!(frame.buf[4], pack_xrgb(255, 0, 0));
        assert_eq!(frame.buf[5], 0x123456);
        assert_eq!(frame.buf[7], pack_xrgb(0, 255, 0));
        assert_eq!(frame.buf[8], pack_xrgb(0, 0, 255));
    }

    #[test]
    fn blit_rejects_bad_geometry() {
        let mut buf = vec![0u32; 4];
        let mut frame = Frame {
            buf: &mut buf,
            w: 2,
            h: 2,
        };
        let empty = Sprite {
            pixels: &[0u16; 2],
            w: 2,
            h: 2,
            stride: 0,
        };
        blit_rgb555(&mut frame, &empty, 0, 0, None);
        let short = Sprite {
            pixels: &[0u16; 1],
            w: 1,
            h: 1,
            stride: 1,
        };
        blit_rgb555(&mut frame, &short, 5, 5, None);
        assert_eq!(frame.buf, vec![0u32; 4]);
    }

    fn highway() -> Rect {
        Rect {
            x: 10,
            y: 0,
            w: 70,
            h: 100,
        }
    }

    #[test]
    fn full_curtain_keeps_the_whole_highway() {
        assert_eq!(
            visible_spans(LaneCurtain::Full, &highway()),
            vec![highway()]
        );
    }

    #[test]
    fn hidden_curtain_covers_the_bottom_fifth() {
        assert_eq!(
            visible_spans(LaneCurtain::Hidden, &highway()),
            vec![Rect {
                x: 10,
                y: 0,
                w: 70,
                h: 80,
            }]
        );
    }

    #[test]
    fn sudden_curtain_shows_only_the_bottom_fifth() {
        assert_eq!(
            visible_spans(LaneCurtain::Sudden, &highway()),
            vec![Rect {
                x: 10,
                y: 80,
                w: 70,
                h: 20,
            }]
        );
    }

    #[test]
    fn dark_curtain_shows_only_the_middle_fifth() {
        assert_eq!(
            visible_spans(LaneCurtain::Dark, &highway()),
            vec![Rect {
                x: 10,
                y: 40,
                w: 70,
                h: 20,
            }]
        );
    }

    #[test]
    fn reverse_dark_curtain_splits_top_and_bottom_fifths() {
        assert_eq!(
            visible_spans(LaneCurtain::ReverseDark, &highway()),
            vec![
                Rect {
                    x: 10,
                    y: 0,
                    w: 70,
                    h: 40,
                },
                Rect {
                    x: 10,
                    y: 60,
                    w: 70,
                    h: 40,
                },
            ]
        );
    }

    #[test]
    fn empty_highway_has_no_visible_spans() {
        let flat = Rect {
            x: 0,
            y: 0,
            w: 70,
            h: 0,
        };
        assert!(visible_spans(LaneCurtain::Sudden, &flat).is_empty());
    }

    #[test]
    fn clipped_blit_drops_pixels_outside_the_curtain() {
        // 1x4 red note strip blitted over the whole highway.
        let sprite = Sprite {
            pixels: &[0x7C00, 0x7C00, 0x7C00, 0x7C00],
            w: 1,
            h: 4,
            stride: 1,
        };
        let red = pack_xrgb(255, 0, 0);
        for curtain in [LaneCurtain::Sudden, LaneCurtain::Dark] {
            let spans = visible_spans(
                curtain,
                &Rect {
                    x: 0,
                    y: 0,
                    w: 1,
                    h: 20,
                },
            );
            assert_eq!(spans.len(), 1);
            let mut buf = vec![0u32; 20];
            let mut frame = Frame {
                buf: &mut buf,
                w: 1,
                h: 20,
            };
            blit_rgb555_clipped(&mut frame, &sprite, 0, 16, None, spans.first());
            let lit: Vec<usize> = frame
                .buf
                .iter()
                .enumerate()
                .filter(|(_, pixel)| **pixel == red)
                .map(|(row, _)| row)
                .collect();
            match curtain {
                LaneCurtain::Sudden => assert_eq!(lit, vec![16, 17, 18, 19]),
                _ => assert_eq!(lit, Vec::<usize>::new()),
            }
        }
    }

    #[test]
    fn clipped_blit_without_clip_matches_plain_blit() {
        let sprite = Sprite {
            pixels: &[0x7C00, 0x03E0],
            w: 2,
            h: 1,
            stride: 2,
        };
        let mut plain = vec![0u32; 2];
        let mut clipped = vec![0u32; 2];
        blit_rgb555(
            &mut Frame {
                buf: &mut plain,
                w: 2,
                h: 1,
            },
            &sprite,
            0,
            0,
            None,
        );
        blit_rgb555_clipped(
            &mut Frame {
                buf: &mut clipped,
                w: 2,
                h: 1,
            },
            &sprite,
            0,
            0,
            None,
            None,
        );
        assert_eq!(plain, clipped);
    }

    #[test]
    fn all_digits_match_their_bitmaps() {
        for digit in 0..10u8 {
            let mut buf = vec![0u32; GLYPH_W * GLYPH_H];
            let mut frame = Frame {
                buf: &mut buf,
                w: GLYPH_W,
                h: GLYPH_H,
            };
            draw_digit(&mut frame, 0, 0, digit, 0xFFFFFF);
            let rows = digit_glyph(digit).unwrap();
            for (y, bits) in rows.iter().enumerate() {
                for x in 0..GLYPH_W {
                    let lit = bits & (1 << (GLYPH_W - 1 - x)) != 0;
                    assert_eq!(frame.buf[y * GLYPH_W + x] != 0, lit, "{digit} at {x},{y}");
                }
            }
        }
        assert_eq!(digit_glyph(10), None);
    }

    #[test]
    fn digit_draw_clips_off_frame() {
        let mut buf = vec![0u32; 2 * 2];
        let mut frame = Frame {
            buf: &mut buf,
            w: 2,
            h: 2,
        };
        draw_digit(&mut frame, -1, -1, 8, 0xFFFFFF);
        draw_digit(&mut frame, 0, 0, 10, 0xFFFFFF);
        assert_eq!(frame.buf, vec![0, 0xFFFFFF, 0xFFFFFF, 0xFFFFFF]);
    }

    #[test]
    fn number_renders_decimal_without_leading_zeros() {
        let mut buf = vec![0u32; 12 * GLYPH_H];
        let mut frame = Frame {
            buf: &mut buf,
            w: 12,
            h: GLYPH_H,
        };
        let end = draw_number(&mut frame, 0, 0, 507, 0xFFFFFF);
        assert_eq!(end, 12);
        assert_eq!(number_width(507), 11);
        assert_eq!(number_width(0), GLYPH_W);
        // '5' top bar, gap column, '0' top-left pixel.
        assert_eq!(frame.buf[0], 0xFFFFFF);
        assert_eq!(frame.buf[3], 0);
        assert_eq!(frame.buf[4], 0xFFFFFF);
        assert_eq!(frame.buf[8], 0xFFFFFF);
    }

    #[test]
    fn all_letters_match_their_bitmaps() {
        for ch in b'A'..=b'Z' {
            let mut buf = vec![0u32; GLYPH_W * GLYPH_H];
            let mut frame = Frame {
                buf: &mut buf,
                w: GLYPH_W,
                h: GLYPH_H,
            };
            draw_char(&mut frame, 0, 0, ch, 0xFFFFFF);
            let rows = letter_glyph(ch).unwrap();
            for (y, bits) in rows.iter().enumerate() {
                for x in 0..GLYPH_W {
                    let lit = bits & (1 << (GLYPH_W - 1 - x)) != 0;
                    assert_eq!(frame.buf[y * GLYPH_W + x] != 0, lit, "{ch} at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn lowercase_maps_to_upper() {
        assert_eq!(glyph_for(b'a'), glyph_for(b'A'));
        assert_eq!(glyph_for(b'z'), glyph_for(b'Z'));
        assert_eq!(glyph_for(b'5'), digit_glyph(5));
        assert_eq!(glyph_for(b'@'), None);
    }

    #[test]
    fn text_lays_out_with_spacing() {
        let mut buf = vec![0u32; 16 * GLYPH_H];
        let mut frame = Frame {
            buf: &mut buf,
            w: 16,
            h: GLYPH_H,
        };
        let end = draw_text(&mut frame, 0, 0, "HI!", 0xFFFFFF);
        assert_eq!(end, 12);
        assert_eq!(text_width("HI!"), 11);
        assert_eq!(text_width(""), 0);
        // 'H' stems, gap column, 'I' top bar.
        assert_eq!(frame.buf[0], 0xFFFFFF);
        assert_eq!(frame.buf[3], 0);
        assert_eq!(frame.buf[4], 0xFFFFFF);
        assert_eq!(frame.buf[5], 0xFFFFFF);
        assert_eq!(frame.buf[6], 0xFFFFFF);
    }

    #[test]
    fn text_skips_undrawable_bytes_blank() {
        let mut buf = vec![0u32; 8 * GLYPH_H];
        let mut frame = Frame {
            buf: &mut buf,
            w: 8,
            h: GLYPH_H,
        };
        draw_text(&mut frame, 0, 0, "A@", 0xFFFFFF);
        assert_eq!(frame.buf[0], 0);
        assert_eq!(frame.buf[1], 0xFFFFFF);
        assert!(frame.buf[4..8].iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn rgb555_image_decodes_and_blits() {
        // 2x1: red, green (little-endian words).
        let image = Rgb555Image::decode(&[0x00, 0x7C, 0xE0, 0x03], 2, 1).unwrap();
        assert_eq!(image.words, vec![0x7C00, 0x03E0]);
        let mut buf = vec![0u32; 2];
        let mut frame = Frame {
            buf: &mut buf,
            w: 2,
            h: 1,
        };
        blit_rgb555(&mut frame, &image.view(), 0, 0, None);
        assert_eq!(frame.buf, vec![pack_xrgb(255, 0, 0), pack_xrgb(0, 255, 0)]);
    }

    #[test]
    fn rgb555_image_rejects_size_mismatch() {
        assert_eq!(Rgb555Image::decode(&[0x00, 0x00], 2, 1), None);
        assert_eq!(
            Rgb555Image::decode(&[], 0, 0).unwrap().words,
            Vec::<u16>::new()
        );
    }

    #[test]
    fn stretch_doubles_each_pixel() {
        let src = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let mut dst = vec![0u32; 4 * 4];
        stretch_blit(&mut dst, 4, 4, &src, 2, 2);
        assert_eq!(
            dst,
            vec![
                0xAA, 0xAA, 0xBB, 0xBB, 0xAA, 0xAA, 0xBB, 0xBB, 0xCC, 0xCC, 0xDD, 0xDD, 0xCC, 0xCC,
                0xDD, 0xDD,
            ]
        );
    }

    #[test]
    fn stretch_shrinks_from_top_left_samples() {
        let src: Vec<u32> = (0..16).collect();
        let mut dst = vec![0u32; 2 * 2];
        stretch_blit(&mut dst, 2, 2, &src, 4, 4);
        assert_eq!(dst, vec![0, 2, 8, 10]);
    }

    #[test]
    fn stretch_ignores_bad_geometry() {
        let mut dst = vec![0u32; 4];
        stretch_blit(&mut dst, 0, 2, &[1, 2, 3, 4], 2, 2);
        stretch_blit(&mut dst, 2, 2, &[1, 2], 2, 2);
        assert_eq!(dst, vec![0u32; 4]);
    }

    #[test]
    fn number_zero_renders_single_glyph() {
        let mut buf = vec![0u32; GLYPH_W * GLYPH_H];
        let mut frame = Frame {
            buf: &mut buf,
            w: GLYPH_W,
            h: GLYPH_H,
        };
        draw_number(&mut frame, 0, 0, 0, 0xFFFFFF);
        assert_eq!(frame.buf[0], 0xFFFFFF);
        assert_eq!(frame.buf[1], 0xFFFFFF);
        assert_eq!(frame.buf[2], 0xFFFFFF);
        assert_eq!(frame.buf[4], 0);
    }

    #[test]
    fn bind_pairs_codes_with_rects_in_order() {
        let rects = vec![
            Rect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            },
            Rect {
                x: 20,
                y: 20,
                w: 10,
                h: 10,
            },
        ];
        let bound = bind_controls(&[0x20, 0x50], &[true, false], &rects);
        assert_eq!(bound.len(), 2);
        assert_eq!(bound[0].code, 0x20);
        assert!(bound[0].has_sprite);
        assert_eq!(bound[0].rect, rects[0]);
        assert_eq!(bound[1].rect, rects[1]);
    }

    #[test]
    fn bind_truncates_to_the_shortest_input() {
        // Playing ships fewer rects than entries: extras stay unlaid-out.
        let rects = vec![Rect {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        }];
        let bound = bind_controls(&[1, 2, 3], &[true, true, true], &rects);
        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0].code, 1);
    }

    #[test]
    fn corners_build_sized_rects() {
        let rect = rect_from_corners(30, 3, 612, 595);
        assert_eq!(
            rect,
            Rect {
                x: 30,
                y: 3,
                w: 582,
                h: 592
            }
        );
        assert_eq!(rect_from_corners(5, 5, 5, 9).w, 0);
        assert_eq!(rect_from_corners(9, 9, 5, 5).w, 0);
    }

    #[test]
    fn scale_keeps_native_identity() {
        let rect = Rect {
            x: 100,
            y: 150,
            w: 200,
            h: 120,
        };
        assert_eq!(scale_rect_to_frame(&rect, 800, 600), rect);
    }

    #[test]
    fn scale_shrinks_proportionally() {
        let rect = Rect {
            x: 400,
            y: 300,
            w: 800,
            h: 600,
        };
        assert_eq!(
            scale_rect_to_frame(&rect, 80, 60),
            Rect {
                x: 40,
                y: 30,
                w: 80,
                h: 60
            }
        );
    }

    #[test]
    fn center_places_box_mid_frame() {
        assert_eq!(
            center_rect(4, 2, 10, 8),
            Rect {
                x: 3,
                y: 3,
                w: 4,
                h: 2
            }
        );
        assert_eq!(
            center_rect(20, 20, 10, 8),
            Rect {
                x: 0,
                y: 0,
                w: 10,
                h: 8
            }
        );
    }

    #[test]
    fn cover_matches_plain_blit_at_same_size() {
        let image = Rgb555Image::decode(&[0x00, 0x7C, 0xE0, 0x03], 2, 1).unwrap();
        let mut buf = vec![0u32; 2];
        blit_image_in_rect(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 1,
            },
            &image,
            0xFFFF,
            &Rect {
                x: 0,
                y: 0,
                w: 2,
                h: 1,
            },
        );
        assert_eq!(buf, vec![pack_xrgb(255, 0, 0), pack_xrgb(0, 255, 0)]);
    }

    #[test]
    fn cover_stretches_and_skips_colorkey() {
        // 2x1 red/key stretched over 4x1: key columns stay background.
        let image = Rgb555Image::decode(&[0x00, 0x7C, 0x00, 0x00], 2, 1).unwrap();
        let mut buf = vec![0x123456u32; 4];
        blit_image_in_rect(
            &mut Frame {
                buf: &mut buf,
                w: 4,
                h: 1,
            },
            &image,
            0x0000,
            &Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 1,
            },
        );
        let red = pack_xrgb(255, 0, 0);
        assert_eq!(buf, vec![red, red, 0x123456, 0x123456]);
    }

    #[test]
    fn cover_ignores_empty_targets() {
        let image = Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap();
        let mut buf = vec![0x123456u32; 4];
        blit_image_in_rect(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 2,
            },
            &image,
            0xFFFF,
            &Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 2,
            },
        );
        assert_eq!(buf, vec![0x123456u32; 4]);
    }

    #[test]
    fn scene_layers_paint_back_to_front() {
        let red = Rgb555Image::decode(&[0x00, 0x7C, 0x00, 0x7C], 2, 1).unwrap();
        let green = Rgb555Image::decode(&[0xE0, 0x03, 0xE0, 0x03], 2, 1).unwrap();
        let mut buf = vec![0u32; 2];
        paint_scene_layers(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 1,
            },
            0x000000,
            &[
                SceneLayer {
                    image: &red,
                    colorkey: 0xFFFF,
                    dest: Rect {
                        x: 0,
                        y: 0,
                        w: 2,
                        h: 1,
                    },
                },
                SceneLayer {
                    image: &green,
                    colorkey: 0xFFFF,
                    dest: Rect {
                        x: 1,
                        y: 0,
                        w: 1,
                        h: 1,
                    },
                },
            ],
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[1], pack_xrgb(0, 255, 0));
    }
}
