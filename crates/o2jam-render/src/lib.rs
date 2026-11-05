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
    if sprite.stride == 0
        || frame.buf.len() < frame.w * frame.h
        || sprite.pixels.len() < sprite.stride * sprite.h
    {
        return;
    }
    for row in 0..sprite.h {
        let dest_y = dy + row as i32;
        if dest_y < 0 || dest_y >= frame.h as i32 {
            continue;
        }
        for col in 0..sprite.w {
            let dest_x = dx + col as i32;
            if dest_x < 0 || dest_x >= frame.w as i32 {
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
}
