//! One ink measurement for the exe-pinned TEXT guards.
//!
//! Exe ink is anti-aliased, so these guards never match a colour. They
//! compare each window pixel with that window's top-left pixel (the
//! window background) and count a pixel as ink when the summed channel
//! difference crosses [`INK_DIFF_THRESHOLD`]. The offline exe scan and
//! the in-process frame guard use the same rule.

/// Summed channel difference that separates ink from window background.
pub(crate) const INK_DIFF_THRESHOLD: u32 = 40;

/// Exclusive bounds of one ink block: left, top, right, bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InkBlock {
    pub(crate) bbox: [i32; 4],
    pub(crate) w: i32,
    pub(crate) h: i32,
}

impl InkBlock {
    /// First inked row, used for exact top-bearing assertions.
    pub(crate) fn first_row(self) -> i32 {
        self.bbox[1]
    }
}

/// Measure one text block in an XRGB frame window against its corner.
///
/// `window` is `[left, top, right, bottom]` with exclusive bounds.
/// The caller supplies a window whose corner is the text background;
/// the assert makes an empty or unbounded scan impossible to mistake
/// for a match.
pub(crate) fn ink_block(pixels: &[u32], frame_width: usize, window: [i32; 4]) -> InkBlock {
    let (left, top, right, bottom) = window_bounds(pixels, frame_width, window);
    let background = channels(pixels[top * frame_width + left]);
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for row in top..bottom {
        for column in left..right {
            let difference =
                channel_difference(channels(pixels[row * frame_width + column]), background);
            if difference <= INK_DIFF_THRESHOLD {
                continue;
            }
            let row = i32::try_from(row).expect("row fits i32");
            let column = i32::try_from(column).expect("column fits i32");
            min_x = min_x.min(column);
            min_y = min_y.min(row);
            max_x = max_x.max(column);
            max_y = max_y.max(row);
        }
    }

    assert!(min_x <= max_x, "no ink in window {window:?}");
    InkBlock {
        bbox: [min_x, min_y, max_x, max_y],
        w: max_x - min_x + 1,
        h: max_y - min_y + 1,
    }
}

/// Whether any pixel in the window differs from its corner. Uses the
/// same threshold as [`ink_block`] but does not require ink to exist.
pub(crate) fn has_ink(pixels: &[u32], frame_width: usize, window: [i32; 4]) -> bool {
    let (left, top, right, bottom) = window_bounds(pixels, frame_width, window);
    let background = channels(pixels[top * frame_width + left]);
    (top..bottom).any(|row| {
        (left..right).any(|column| {
            channel_difference(channels(pixels[row * frame_width + column]), background)
                > INK_DIFF_THRESHOLD
        })
    })
}

fn window_bounds(
    pixels: &[u32],
    frame_width: usize,
    window: [i32; 4],
) -> (usize, usize, usize, usize) {
    let [left, top, right, bottom] = window;
    assert!(
        left >= 0 && top >= 0 && right > left && bottom > top,
        "ink window must be ordered and non-empty: {window:?}"
    );
    let left = usize::try_from(left).expect("window left fits usize");
    let top = usize::try_from(top).expect("window top fits usize");
    let right = usize::try_from(right).expect("window right fits usize");
    let bottom = usize::try_from(bottom).expect("window bottom fits usize");
    assert!(
        right <= frame_width && bottom * frame_width <= pixels.len(),
        "ink window {window:?} lies outside the {frame_width}-wide frame"
    );
    (left, top, right, bottom)
}

fn channels(pixel: u32) -> [u32; 3] {
    [(pixel >> 16) & 0xFF, (pixel >> 8) & 0xFF, pixel & 0xFF]
}

fn channel_difference(left: [u32; 3], right: [u32; 3]) -> u32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left.abs_diff(right))
        .sum()
}

#[cfg(test)]
mod ink_measurement_tests {
    use super::*;

    #[test]
    fn ink_block_ignores_window_background_and_measures_ink_extent() {
        let background = 0x0000_0000;
        let ink = 0x00FF_FFFF;
        let mut pixels = vec![background; 4 * 5];
        pixels[5] = ink;
        pixels[6] = ink;
        pixels[10] = ink;

        let block = ink_block(&pixels, 4, [0, 0, 4, 5]);

        assert_eq!(block.bbox, [1, 1, 2, 2]);
        assert_eq!(block.w, 2);
        assert_eq!(block.h, 2);
        assert_eq!(block.first_row(), 1);
    }
}
