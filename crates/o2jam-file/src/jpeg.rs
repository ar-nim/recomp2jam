//! JPEG album-art bodies (docs/claims.md A37): container sniffing,
//! scan-class classification, and full pixel decode.
//!
//! The 47 `ATI*`/`ati*_{id}.ojs` bodies in `Interface1.opi` are raw
//! JPEG streams (`FF D8 … FF D9`), consumed in-exe by the shipped IJG
//! build (`CDXImageJPG`, RTTI file `0x27d5d8`; IJG region VA
//! `0x75f9f0`). Verified on the real 7.55 archives (client
//! `Interface1.opi` sha256 `d66541…0584`): every body is 800x600 with
//! 3 components — 21 baseline (SOF0) and 26 progressive (SOF2) — so a
//! baseline-only decoder would leave 26 of 47 panels blank. Decode is
//! re-implemented (behavior from these documented facts, never from
//! exe bytes) on the pure-Rust `zune-jpeg` crate, the decoder behind
//! `image` >= 0.25, forced to RGB888 output for every input
//! colorspace.

use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;
use zune_jpeg::JpegDecoder;

use super::{is_jpeg_body, unknown_format, FileError};

/// Bytes per RGB888 pixel.
const RGB_BYTES_PER_PIXEL: usize = 3;

/// Length of the JPEG SOI marker.
const JPEG_SOI_LEN: usize = 2;

/// The JPEG SOI marker (`FF D8`): opens every real JPEG body, and the
/// only framing the OJN-embedded covers guarantee (their stream runs
/// to end of file with no EOI marker).
pub(crate) const JPEG_SOI: [u8; JPEG_SOI_LEN] = [0xFF, 0xD8];

/// One decoded JPEG body: full-resolution RGB888 pixels, ready for
/// [`o2jam_render::Rgb888Image`] / [`o2jam_render::blit_rgb888`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JpegRgbImage {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Row-major RGB888 bytes, 3 per pixel, `width * height * 3` long.
    pub pixels: Vec<u8>,
}

/// Decode one complete JPEG body to RGB888 pixels.
///
/// Both scan classes the 7.55 archives ship decode: baseline (SOF0)
/// and progressive (SOF2); the input must pass the complete-body
/// sniff [`is_jpeg_body`] (`FF D8` … `FF D9`), which every real
/// album-art body satisfies.
///
/// # Errors
///
/// [`FileError::UnknownFormat`] when the body is not a JPEG stream,
/// [`FileError::JpegDecode`] when tables or entropy data are corrupt,
/// [`FileError::Truncated`] when the decoded pixel count disagrees
/// with the claimed dimensions.
pub fn decode_jpeg_body(bytes: &[u8]) -> Result<JpegRgbImage, FileError> {
    if !is_jpeg_body(bytes) {
        return Err(unknown_format(bytes));
    }
    decode_jpeg_pixels(bytes)
}

/// Shared JPEG decode behind the two body sniffs: color space,
/// decoder run, dimensions, and pixel-count check.
fn decode_jpeg_pixels(bytes: &[u8]) -> Result<JpegRgbImage, FileError> {
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    let pixels = decoder
        .decode()
        .map_err(|error| FileError::JpegDecode(error.to_string()))?;
    let info = decoder
        .info()
        .ok_or_else(|| FileError::JpegDecode("decoder reported no image info".to_owned()))?;
    let width = usize::from(info.width);
    let height = usize::from(info.height);
    let expected = width
        .checked_mul(height)
        .and_then(|count| count.checked_mul(RGB_BYTES_PER_PIXEL))
        .ok_or(FileError::Truncated {
            need: usize::MAX,
            have: pixels.len(),
        })?;
    if pixels.len() != expected {
        return Err(FileError::Truncated {
            need: expected,
            have: pixels.len(),
        });
    }
    Ok(JpegRgbImage {
        width,
        height,
        pixels,
    })
}

/// Decode one OJN-embedded album cover (docs/file-formats/ojn.md):
/// the JPEG stream after the last level slice runs to end of file and
/// carries no EOI marker on every shipped 7.55 chart (14/14), so this
/// sniffs only the SOI marker — unlike [`decode_jpeg_body`], which
/// needs the complete-body frame — and decodes what is there.
///
/// # Errors
///
/// [`FileError::UnknownFormat`] when the bytes carry no SOI marker,
/// [`FileError::JpegDecode`] when tables or entropy data are corrupt,
/// [`FileError::Truncated`] when the decoded pixel count disagrees
/// with the claimed dimensions.
pub fn decode_ojn_cover(bytes: &[u8]) -> Result<JpegRgbImage, FileError> {
    if !bytes.starts_with(&JPEG_SOI) {
        return Err(unknown_format(bytes));
    }
    decode_jpeg_pixels(bytes)
}

/// First SOF marker of a JPEG stream: `Some(0xC0)` baseline, `Some(0xC2)`
/// progressive, and so on; `None` when the stream ends before any SOF
/// (including a scan that starts before a frame). This is the byte-level
/// scan-class evidence behind the 21 + 26 verified split.
#[must_use]
pub fn jpeg_sof_marker(bytes: &[u8]) -> Option<u16> {
    let mut pos = JPEG_SOI_LEN;
    while bytes.get(pos) == Some(&0xFF) {
        while bytes.get(pos) == Some(&0xFF) {
            pos += 1;
        }
        let marker = u16::from(*bytes.get(pos)?);
        pos += 1;
        match marker {
            // Stand-alone markers carry no length segment.
            0x01 | 0xD8 | 0xD9 | 0xD0..=0xD7 => continue,
            // A scan before any frame: malformed, and never decodable.
            0xDA => return None,
            _ => {}
        }
        let seg_len = usize::from(*bytes.get(pos)?) << 8 | usize::from(*bytes.get(pos + 1)?);
        if is_sof_marker(marker) {
            return Some(marker);
        }
        pos += seg_len;
    }
    None
}

/// Whether a marker is in the SOF family (DHT/DAC/JPG reserved
/// markers excluded).
fn is_sof_marker(marker: u16) -> bool {
    (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;
    use crate::parse_opi;
    use o2jam_render::{blit_rgb888, pack_xrgb, Frame, Rgb888Image, NATIVE_HEIGHT, NATIVE_WIDTH};

    fn be16(value: u16) -> [u8; 2] {
        value.to_be_bytes()
    }

    fn u8_or_max(value: usize) -> u8 {
        u8::try_from(value).unwrap_or(u8::MAX)
    }

    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut seg = vec![0xFF, marker];
        seg.extend_from_slice(&be16(
            u16::try_from(payload.len())
                .unwrap_or(u16::MAX)
                .saturating_add(2),
        ));
        seg.extend_from_slice(payload);
        seg
    }

    /// Quantization table 0, all values 1 (coefficients pass through).
    fn dqt() -> Vec<u8> {
        let mut payload = vec![0x00];
        payload.extend_from_slice(&[1u8; 64]);
        segment(0xDB, &payload)
    }

    /// Huffman table with two 2-bit codes (`00` -> symbol 0x00, `01`
    /// -> symbol 0x01): every emitted code is `00` and no unused tree
    /// branch is ever traversed. The DHT count byte is indexed by code
    /// length minus one, so the two length-2 codes sit in `counts[1]`.
    fn dht(class_and_id: u8) -> Vec<u8> {
        let mut counts = [0u8; 16];
        counts[1] = 2;
        let mut payload = vec![class_and_id];
        payload.extend_from_slice(&counts);
        payload.extend_from_slice(&[0x00, 0x01]);
        segment(0xC4, &payload)
    }

    fn sof_frame(components: &[(u8, u8, u8)], marker: u8) -> Vec<u8> {
        let mut payload = vec![8u8];
        payload.extend_from_slice(&be16(8));
        payload.extend_from_slice(&be16(8));
        payload.push(u8_or_max(components.len()));
        for &(id, sampling, quant) in components {
            payload.extend_from_slice(&[id, sampling, quant]);
        }
        segment(marker, &payload)
    }

    fn sos(components: usize) -> Vec<u8> {
        let mut payload = vec![u8_or_max(components)];
        for id in 1..=components {
            payload.extend_from_slice(&[u8_or_max(id), 0x00]);
        }
        payload.extend_from_slice(&[0x00, 0x3F, 0x00]);
        segment(0xDA, &payload)
    }

    /// Entropy stream for `blocks` all-zero blocks: per block a DC
    /// category-0 code (`00`) plus an AC end-of-block code (`00`),
    /// padded with 1s to the byte.
    fn entropy_bytes(blocks: usize) -> Vec<u8> {
        let bits = blocks * 4;
        let mut out = vec![0x00u8; bits / 8];
        if !bits.is_multiple_of(8) {
            out.push(!0u8 >> (bits % 8));
        }
        out
    }

    /// Minimal synthetic JPEG: 8x8, quant table all 1s, all
    /// coefficients zero — every block decodes to mid-gray 128 (the
    /// JPEG level shift adds 128 back). `sof_marker` selects the scan
    /// class (0xC0 baseline, 0xC2 progressive).
    fn minimal_jpeg(components: &[(u8, u8, u8)], sof_marker: u8, mcu_blocks: usize) -> Vec<u8> {
        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend(dqt());
        jpeg.extend(dht(0x00));
        jpeg.extend(dht(0x10));
        jpeg.extend(sof_frame(components, sof_marker));
        jpeg.extend(sos(components.len()));
        jpeg.extend(entropy_bytes(mcu_blocks));
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        jpeg
    }

    const GRAY: &[(u8, u8, u8)] = &[(1, 0x11, 0)];
    const COLOR_444: &[(u8, u8, u8)] = &[(1, 0x11, 0), (2, 0x11, 0), (3, 0x11, 0)];
    const COLOR_420: &[(u8, u8, u8)] = &[(1, 0x22, 0), (2, 0x11, 0), (3, 0x11, 0)];

    fn assert_mid_gray(jpeg: &[u8]) -> JpegRgbImage {
        let image = decode_jpeg_body(jpeg).expect("fixture decodes");
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(image.pixels, vec![128u8; 8 * 8 * RGB_BYTES_PER_PIXEL]);
        image
    }

    #[test]
    fn grayscale_baseline_fixture_decodes_to_mid_gray() {
        assert_mid_gray(&minimal_jpeg(GRAY, 0xC0, 1));
    }

    #[test]
    fn color_444_fixture_decodes_to_neutral_gray() {
        assert_mid_gray(&minimal_jpeg(COLOR_444, 0xC0, 3));
    }

    #[test]
    fn color_420_fixture_decodes_to_neutral_gray() {
        // One 4:2:0 MCU: 4 luma blocks plus one per chroma plane.
        assert_mid_gray(&minimal_jpeg(COLOR_420, 0xC0, 6));
    }

    #[test]
    fn progressive_fixture_decodes_to_neutral_gray() {
        assert_mid_gray(&minimal_jpeg(COLOR_444, 0xC2, 3));
    }

    #[test]
    fn decode_rejects_non_jpeg_and_truncated_bodies() {
        assert!(matches!(
            decode_jpeg_body(b"not a jpeg at all"),
            Err(FileError::UnknownFormat { .. })
        ));
        let jpeg = minimal_jpeg(COLOR_444, 0xC0, 3);
        let half = jpeg.len() / 2;
        assert!(matches!(
            decode_jpeg_body(&jpeg[..half]),
            Err(FileError::UnknownFormat { .. })
        ));
        // Complete framing but corrupt Huffman-table definitions: a
        // decode error, never silent pixels.
        let mut corrupt = jpeg.clone();
        // First DHT: 2 SOI + 71 DQT, then marker(2) + length(2) +
        // class/id(1); the 16 count bytes that follow now claim 128
        // codes while the table carries 2 symbols.
        let counts_at = 2 + 71 + 4 + 1;
        for count in &mut corrupt[counts_at..counts_at + 16] {
            *count = 8;
        }
        assert!(matches!(
            decode_jpeg_body(&corrupt),
            Err(FileError::JpegDecode(_))
        ));
    }

    #[test]
    fn sof_walk_reports_baseline_and_progressive() {
        assert_eq!(jpeg_sof_marker(&minimal_jpeg(GRAY, 0xC0, 1)), Some(0xC0));
        assert_eq!(jpeg_sof_marker(&minimal_jpeg(GRAY, 0xC2, 1)), Some(0xC2));
        assert_eq!(jpeg_sof_marker(&[0xFF, 0xD8]), None);
        assert_eq!(jpeg_sof_marker(&[0xFF, 0xD8, 0x12]), None);
    }

    #[test]
    fn ojn_cover_stream_decodes_without_the_eoi_marker() {
        // The shipped OJN covers run to end of file: same fixture with
        // the EOI stripped decodes to the same mid gray, while the
        // complete-body sniff rightly refuses it.
        let full = minimal_jpeg(COLOR_444, 0xC0, 3);
        let cover = &full[..full.len() - 2];
        assert!(!is_jpeg_body(cover));
        let image = decode_ojn_cover(cover).expect("cover stream decodes");
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(image.pixels, vec![128u8; 8 * 8 * RGB_BYTES_PER_PIXEL]);
    }

    #[test]
    fn ojn_cover_rejects_streams_without_the_soi_marker() {
        assert!(matches!(
            decode_ojn_cover(b"not a jpeg at all"),
            Err(FileError::UnknownFormat { .. })
        ));
        assert!(matches!(
            decode_ojn_cover(b""),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn decoded_fixture_blits_into_frame() {
        // The end-to-end JPEG -> Frame path album-art panels use.
        let decoded = decode_jpeg_body(&minimal_jpeg(COLOR_444, 0xC0, 3)).expect("decodes");
        let image = Rgb888Image::from_rgb888(&decoded.pixels, decoded.width, decoded.height)
            .expect("sizes agree");
        let mut buf = vec![0u32; 10 * 10];
        let mut frame = Frame {
            buf: &mut buf,
            w: 10,
            h: 10,
        };
        blit_rgb888(&mut frame, &image, 1, 2);
        let gray = pack_xrgb(128, 128, 128);
        assert_eq!(frame.buf[0], 0);
        assert_eq!(frame.buf[2 * 10 + 1], gray);
        assert_eq!(frame.buf[2 * 10 + 8], gray);
        assert_eq!(frame.buf[3 * 10 + 1], gray);
        assert_eq!(frame.buf[9 * 10 + 9], 0);
    }

    /// Repository-local `install-root` (git-ignored game data).
    fn interface_archive() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Interface1.opi")
    }

    /// Repository-local `Music` chart folder (git-ignored game data).
    fn music_folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root/O2Jam/Music")
    }

    /// Real OJN covers end to end: all 14 shipped 7.55 charts (old
    /// Tutorial.ojn plus the 13 new-format files) locate a SOI at the
    /// third slice end, and the EOI-less stream decodes to an
    /// 800x600 baseline body (docs/file-formats/ojn.md). Skips when
    /// `install-root` is absent.
    #[test]
    fn real_ojn_covers_extract_and_decode() {
        let Ok(folder) = std::fs::read_dir(music_folder()) else {
            return;
        };
        let mut charts = Vec::new();
        for entry in folder.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "ojn") {
                charts.push(path);
            }
        }
        assert_eq!(charts.len(), 14, "the 7.55 install ships 14 charts");
        for path in &charts {
            let bytes =
                std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let cover = crate::ojn_cover_jpeg(&bytes)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert!(cover.starts_with(&JPEG_SOI), "{}: SOI", path.display());
            let image = decode_ojn_cover(&cover)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(
                (image.width, image.height),
                (800, 600),
                "{}",
                path.display()
            );
        }
    }

    /// Real album-art bodies: 47 JPEGs, 21 baseline + 26 progressive,
    /// all 800x600 3-component; the first (`ATI_1001.ojs`) paints with
    /// its top-left corner within a +-2 IDCT rounding band of the
    /// (200, 200, 200) pinned by an independent decoder (PIL 13.0) —
    /// no invented pixels. Skips when `install-root` is absent.
    #[test]
    fn real_album_art_bodies_decode_and_paint() {
        let Ok(archive) = std::fs::read(interface_archive()) else {
            return;
        };
        let entries = parse_opi(&archive).expect("archive table parses");
        let mut bodies = Vec::new();
        for entry in &entries {
            let Some(size) = usize::try_from(entry.size).ok() else {
                continue;
            };
            let Some(body) = archive.get(entry.data_offset..entry.data_offset + size) else {
                continue;
            };
            if is_jpeg_body(body) {
                bodies.push((entry.name.clone(), body.to_vec()));
            }
        }
        assert_eq!(bodies.len(), 47);
        let mut sof_counts: HashMap<u16, usize> = HashMap::new();
        for (name, body) in &bodies {
            let image = decode_jpeg_body(body).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!((image.width, image.height), (800, 600), "{name}");
            assert_eq!(image.pixels.len(), 800 * 600 * RGB_BYTES_PER_PIXEL);
            *sof_counts
                .entry(jpeg_sof_marker(body).expect("SOF present"))
                .or_default() += 1;
        }
        assert_eq!(sof_counts[&0xC0], 21, "baseline bodies");
        assert_eq!(sof_counts[&0xC2], 26, "progressive bodies");

        let (first_name, first_body) = &bodies[0];
        let decoded = decode_jpeg_body(first_body).expect("first body decodes");
        let image = Rgb888Image::from_rgb888(&decoded.pixels, decoded.width, decoded.height)
            .expect("sizes agree");
        let native_w = usize::try_from(NATIVE_WIDTH).expect("native width");
        let native_h = usize::try_from(NATIVE_HEIGHT).expect("native height");
        let mut buf = vec![0u32; native_w * native_h];
        let mut frame = Frame {
            buf: &mut buf,
            w: native_w,
            h: native_h,
        };
        blit_rgb888(&mut frame, &image, 0, 0);
        let corner = frame.buf[0];
        let close_enough = |channel: u32, want: u8| {
            i32::from(u8::try_from(channel).unwrap_or(u8::MAX)).abs_diff(i32::from(want)) <= 2
        };
        assert!(
            close_enough((corner >> 16) & 0xFF, 200)
                && close_enough((corner >> 8) & 0xFF, 200)
                && close_enough(corner & 0xFF, 200),
            "{first_name} corner {corner:06X} off the pinned (200, 200, 200)"
        );
    }
}
