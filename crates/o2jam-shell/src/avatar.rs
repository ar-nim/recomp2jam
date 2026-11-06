//! Avatar figure: decoded sprite-55 images, drawable on results.
//!
//! One parsed body becomes one figure: every raw-RGB555 image is
//! decoded, anything else (compressed/outlier frames) is skipped —
//! their codecs are still TBD in `o2jam-file`. Compositing equipped
//! slots (positions, layering, `SetInfodata` mapping) lands later;
//! for now the figure paints its first image, which is the whole
//! body for single-image sprites.

use o2jam_file::Sprite55;
use o2jam_render::{blit_rgb555, Frame, Rgb555Image};

/// Drawable decoded sprite-55 body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AvatarFigure {
    /// Decoded images in stored order.
    images: Vec<Rgb555Image>,
    /// Transparent colour key from the container.
    colorkey: u16,
}

impl AvatarFigure {
    /// Decode every raw image; empty when none decodes.
    #[must_use]
    pub fn from_sprite55(sprite: &Sprite55<'_>) -> Self {
        let images = sprite
            .images
            .iter()
            .filter(|image| sprite.is_raw_rgb555(image))
            .filter_map(|image| {
                Rgb555Image::decode(sprite.pixel_data(image), image.w as usize, image.h as usize)
            })
            .collect();
        Self {
            images,
            colorkey: sprite.colorkey,
        }
    }

    /// Decoded image count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// Whether any image decoded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// Paint the first image with its top-left at `(x, y)`.
    pub fn paint(&self, frame: &mut Frame<'_>, x: i32, y: i32) {
        if let Some(first) = self.images.first() {
            blit_rgb555(frame, &first.view(), x, y, Some(self.colorkey));
        }
    }
}

#[cfg(test)]
mod avatar_figure_tests {
    use super::*;
    use o2jam_file::parse_sprite55;

    /// Minimal container: header + one record + blob (mirrors the
    /// `o2jam-file` synthetic builders).
    fn sprite55_blob(tag: u16, colorkey: u16, w: u16, h: u16, pixels: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&tag.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&colorkey.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&w.to_le_bytes());
        blob.extend_from_slice(&h.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(pixels);
        blob
    }

    #[test]
    fn decodes_first_image_with_colorkey() {
        // 2x1: colorkey black, then red.
        let blob = sprite55_blob(1, 0x0000, 2, 1, &[0x00, 0x00, 0x00, 0x7C]);
        let sprite = parse_sprite55(&blob).unwrap();
        let figure = AvatarFigure::from_sprite55(&sprite);
        assert_eq!(figure.len(), 1);
        let mut buf = vec![0x123456u32; 2];
        figure.paint(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 1,
            },
            0,
            0,
        );
        assert_eq!(buf, vec![0x123456, o2jam_render::pack_xrgb(255, 0, 0)]);
    }

    #[test]
    fn skips_non_raw_images() {
        // Claims 2x2 but stores 2 bytes: not raw, skipped.
        let blob = sprite55_blob(0, 0x0000, 2, 2, &[0xAA, 0xBB]);
        let sprite = parse_sprite55(&blob).unwrap();
        assert!(AvatarFigure::from_sprite55(&sprite).is_empty());
    }

    #[test]
    fn empty_figure_paints_nothing() {
        let figure = AvatarFigure::default();
        let mut buf = vec![0x123456u32; 4];
        figure.paint(
            &mut Frame {
                buf: &mut buf,
                w: 2,
                h: 2,
            },
            0,
            0,
        );
        assert_eq!(buf, vec![0x123456u32; 4]);
    }
}
