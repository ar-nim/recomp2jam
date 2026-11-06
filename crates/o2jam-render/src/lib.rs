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
    let x1 = (rect.x as i64 + rect.w as i64).min(w as i64).max(0) as usize;
    let y1 = (rect.y as i64 + rect.h as i64).min(h as i64).max(0) as usize;
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

/// Draw one digit with its top-left at `(x, y)`; clips to the frame,
/// draws nothing for out-of-range digits.
pub fn draw_digit(frame: &mut Frame<'_>, x: i32, y: i32, digit: u8, color: u32) {
    if frame.buf.len() < frame.w * frame.h {
        return;
    }
    let Some(rows) = digit_glyph(digit) else {
        return;
    };
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
}
