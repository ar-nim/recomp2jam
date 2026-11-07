//! Sprite bodies: sprite-55 containers (plain, strided, and
//! prefixed), BND rects, and OJA animation tables.
//!
//! Verified on real 7.55 files (docs/file-formats/opi.md).

use super::{dir_end, read_i16le, read_u16le, read_u32le, unknown_format, FileError};

/// Magic of the dominant inner sprite container (`.ojs`, `.ojt`,
/// `.oji` bodies): `tag + 0x0555 + count + colorkey`, then `count`
/// 20-byte image records and one concatenated RGB555 pixel blob.
/// Field layout matches the independent CXO2 spec (their
/// signature/format/frames/colorkey); observed tags 0 and 1 parse
/// identically with exact accounting (CXO2 lists signature 1 as
/// valid — tag 0 is a live 7.55 variant, not corruption).
/// Observed colorkeys: `0x0000`, `0x7D5F`.
/// Verified on all ~9.2k such bodies in the real 7.55 archives
/// (docs/file-formats/opi.md).
pub const SPRITE55_MAGIC: u16 = 0x0555;

/// Sprite55 record length: `x + y + w + h + data_offset + size + 0`.
pub const SPRITE55_RECORD_LEN: usize = 20;

/// Sprite55 header length: `tag + magic + count + colorkey`.
pub const SPRITE55_HEADER_LEN: usize = 8;

/// Magic of the animation container (`.oja` bodies, tag + `0x555A`
/// + count): `count` 20-byte frame records follow the header.
///
/// Frame-data codec TBD; only the header and frame table are verified.
pub const OJA_MAGIC: u16 = 0x555A;

/// Oja header length: `tag + magic + count + reserved`.
pub const OJA_HEADER_LEN: usize = 8;

/// Oja frame-record length: `x + y + w + h + a + b + c`.
pub const OJA_RECORD_LEN: usize = 20;

/// Magic opening every `.bnd` bind section.
pub const BND_MAGIC: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

/// Bnd header length: `magic + count`.
pub const BND_HEADER_LEN: usize = 6;

/// Bnd record length: `(x1, y1, x2, y2)`.
pub const BND_RECORD_LEN: usize = 16;

/// One image inside a sprite-55 container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55Image {
    /// Placement anchor, pixels (often 0,0 for single images).
    pub x: u16,
    /// Placement anchor, pixels.
    pub y: u16,
    /// Width in pixels.
    pub w: u16,
    /// Height in pixels.
    pub h: u16,
    /// Byte offset of this image's data from the pixel-blob start
    /// (end of the record directory). Records chain exactly.
    pub data_offset: usize,
    /// Stored byte count of this image's data.
    pub size: u32,
}

/// A parsed sprite-55 container: header, record directory, and the
/// raw concatenated pixel blob (sliced per image on demand).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55<'a> {
    /// Container tag (observed: 0, 1; meaning TBD).
    pub tag: u16,
    /// Transparent colour key (`0x0000` or `0x7D5F`).
    pub colorkey: u16,
    /// Image records in stored order.
    pub images: Vec<Sprite55Image>,
    /// Concatenated per-image data (raw RGB555 when
    /// [`Sprite55::is_raw_rgb555`] holds; prefixed `.oji` row groups
    /// via [`parse_sprite55_prefixed`]; codec TBD for the compressed
    /// frames otherwise).
    pub blob: &'a [u8],
}

impl<'a> Sprite55<'a> {
    /// Raw stored bytes of one image.
    #[must_use]
    pub fn pixel_data(&self, image: &Sprite55Image) -> &'a [u8] {
        &self.blob[image.data_offset..image.data_offset + image.size as usize]
    }
    /// True when the image is stored as plain `w*h` little-endian
    /// RGB555 pixels (`size == w*h*2`; ~99.7% of real bodies).
    /// Outlier classes return false: prefixed `.oji` frames decode
    /// through [`parse_sprite55_prefixed`], RLE-compressed frames
    /// need their codec mapped first. Strided combo atlases (see
    /// [`parse_sprite55_strided`]) store raw frames and return true.
    #[must_use]
    pub fn is_raw_rgb555(&self, image: &Sprite55Image) -> bool {
        image.size as usize == image.w as usize * image.h as usize * 2
    }
}

/// Header + record directory of a sprite-55 container, without the
/// pixel blob: shared contract of the plain and strided layouts.
struct Sprite55Dir {
    tag: u16,
    colorkey: u16,
    images: Vec<Sprite55Image>,
    dir_end: usize,
    stored: usize,
}

/// Read the header and chained record directory; the caller checks
/// the blob accounting of its own layout.
fn parse_sprite55_dir(bytes: &[u8]) -> Result<Sprite55Dir, FileError> {
    if bytes.len() < SPRITE55_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    let tag = read_u16le(bytes, 0)?;
    if read_u16le(bytes, 2)? != SPRITE55_MAGIC {
        return Err(unknown_format(bytes));
    }
    let count = read_u16le(bytes, 4)? as usize;
    let colorkey = read_u16le(bytes, 6)?;
    let dir_end = dir_end(count, SPRITE55_RECORD_LEN, SPRITE55_HEADER_LEN, bytes.len())?;
    let mut images = Vec::with_capacity(count.min(4096));
    let mut run = 0usize;
    for i in 0..count {
        let off = SPRITE55_HEADER_LEN + i * SPRITE55_RECORD_LEN;
        let image = Sprite55Image {
            x: read_u16le(bytes, off)?,
            y: read_u16le(bytes, off + 2)?,
            w: read_u16le(bytes, off + 4)?,
            h: read_u16le(bytes, off + 6)?,
            data_offset: read_u32le(bytes, off + 8)? as usize,
            size: read_u32le(bytes, off + 12)?,
        };
        if read_u32le(bytes, off + 16)? != 0 || image.data_offset != run {
            return Err(unknown_format(bytes));
        }
        run = run
            .checked_add(image.size as usize)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
        images.push(image);
    }
    Ok(Sprite55Dir {
        tag,
        colorkey,
        images,
        dir_end,
        stored: run,
    })
}

/// Parse a sprite-55 container: magic, chained record directory,
/// exact total accounting (`8 + 20*count + blob == len`).
/// Verified on all real 7.55 `.ojs`/`.ojt`/`.oji` sprite bodies
/// except the three `Note_Combo*.ojt` strided atlases (see
/// [`parse_sprite55_strided`]).
///
/// # Errors
///
/// `FileError::Truncated` when records or blob overrun the buffer.
pub fn parse_sprite55(bytes: &[u8]) -> Result<Sprite55<'_>, FileError> {
    let dir = parse_sprite55_dir(bytes)?;
    if dir.dir_end + dir.stored != bytes.len() {
        return Err(FileError::Truncated {
            need: dir.dir_end + dir.stored,
            have: bytes.len(),
        });
    }
    Ok(Sprite55 {
        tag: dir.tag,
        colorkey: dir.colorkey,
        images: dir.images,
        blob: &bytes[dir.dir_end..],
    })
}

/// A parsed strided sprite-55 atlas: same header and record table
/// as [`Sprite55`], but each image stores `frames_per_image`
/// consecutive `w*h` RGB555 frames (`Note_Combo4/5/6.ojt` hold 4
/// glow-animation frames per 64x64 record; verified by rendering
/// one centred digit band per 8 KiB tile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55Strided<'a> {
    /// Container tag (observed: 1).
    pub tag: u16,
    /// Transparent colour key (observed: `0x0000`).
    pub colorkey: u16,
    /// Image records in stored order (`size` = one frame).
    pub images: Vec<Sprite55Image>,
    /// Frames stored per image.
    pub frames_per_image: usize,
    /// Concatenated frame data.
    pub blob: &'a [u8],
}

impl<'a> Sprite55Strided<'a> {
    /// Raw stored bytes of one animation frame.
    #[must_use]
    pub fn frame_data(&self, image: &Sprite55Image, frame: usize) -> &'a [u8] {
        let start = image.data_offset * self.frames_per_image + frame * image.size as usize;
        &self.blob[start..start + image.size as usize]
    }

    /// True when one frame is stored as plain `w*h` little-endian
    /// RGB555 pixels (`size == w*h*2`).
    #[must_use]
    pub fn is_raw_rgb555(&self, image: &Sprite55Image) -> bool {
        image.size as usize == image.w as usize * image.h as usize * 2
    }
}

/// Parse a strided sprite-55 atlas: shared header/record contract
/// plus `frames_per_image * stored == blob` accounting. Rejects
/// `frames_per_image == 0`.
///
/// # Errors
///
/// `FileError::UnknownFormat` on zero frames, `FileError::Truncated`
/// when records or blob overrun.
pub fn parse_sprite55_strided(
    bytes: &[u8],
    frames_per_image: usize,
) -> Result<Sprite55Strided<'_>, FileError> {
    if frames_per_image == 0 {
        return Err(unknown_format(bytes));
    }
    let dir = parse_sprite55_dir(bytes)?;
    let blob_need = dir
        .stored
        .checked_mul(frames_per_image)
        .ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
    if dir.dir_end + blob_need != bytes.len() {
        return Err(FileError::Truncated {
            need: dir.dir_end + blob_need,
            have: bytes.len(),
        });
    }
    Ok(Sprite55Strided {
        tag: dir.tag,
        colorkey: dir.colorkey,
        images: dir.images,
        frames_per_image,
        blob: &bytes[dir.dir_end..],
    })
}

/// Expand one little-endian RGB555 pixel to 24-bit RGB (replicated
/// low bits, the usual GDI 16-bit expansion; bit 15 unused).
/// Matches the CXO2-documented 16-bit format for magic `0x0555`.
#[must_use]
pub fn rgb555_to_rgb888(pixel: u16) -> (u8, u8, u8) {
    let r5 = ((pixel >> 10) & 31) as u8;
    let g5 = ((pixel >> 5) & 31) as u8;
    let b5 = (pixel & 31) as u8;
    (
        (r5 << 3) | (r5 >> 2),
        (g5 << 3) | (g5 >> 2),
        (b5 << 3) | (b5 >> 2),
    )
}

/// One layout rectangle inside a `.bnd` bind section.
///
/// Corners in absolute coordinates: `(x1, y1)` top-left,
/// `(x2, y2)` bottom-right. A record is claimed not by file
/// position but by code ordinal: the entry whose control code low
/// word is `i + 1` (`(code & 0xFFF) − 1`) copies this record, and
/// an out-of-range ordinal yields the all-zero rect instead (exe
/// `0x548640`, dossier `ev_3a869bb7…`; docs/file-formats/opi.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BndRect {
    /// Left edge.
    pub x1: u32,
    /// Top edge.
    pub y1: u32,
    /// Right edge (exclusive).
    pub x2: u32,
    /// Bottom edge (exclusive).
    pub y2: u32,
}

/// A parsed `.bnd` bind section: magic + `count` corner rects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bnd {
    /// Layout rectangles in file order.
    pub rects: Vec<BndRect>,
}

/// Parse a `.bnd` bind section: `0xFFFFFFFF` magic, `u16` count,
/// then `count` 16-byte `(x1, y1, x2, y2)` records; the size must
/// match exactly (`6 + 16*count`). Verified on all 53 real 7.55
/// `.bnd` sections.
///
/// # Errors
///
/// `FileError::TooShort`/`Truncated` when records overrun the buffer.
pub fn parse_bnd(bytes: &[u8]) -> Result<Bnd, FileError> {
    if bytes.len() < BND_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    if bytes[0..4] != BND_MAGIC {
        return Err(unknown_format(bytes));
    }
    let count = read_u16le(bytes, 4)? as usize;
    let dir_end = dir_end(count, BND_RECORD_LEN, BND_HEADER_LEN, bytes.len())?;
    if dir_end != bytes.len() {
        return Err(FileError::Truncated {
            need: dir_end,
            have: bytes.len(),
        });
    }
    let mut rects = Vec::with_capacity(count.min(4096));
    for i in 0..count {
        let off = BND_HEADER_LEN + i * BND_RECORD_LEN;
        rects.push(BndRect {
            x1: read_u32le(bytes, off)?,
            y1: read_u32le(bytes, off + 4)?,
            x2: read_u32le(bytes, off + 8)?,
            y2: read_u32le(bytes, off + 12)?,
        });
    }
    Ok(Bnd { rects })
}

/// One animation frame header inside an `.oja` container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjaFrame {
    /// Frame origin, pixels (may be negative: centred effects).
    pub x: i16,
    /// Frame origin, pixels.
    pub y: i16,
    /// Frame width, pixels.
    pub w: u16,
    /// Frame height, pixels.
    pub h: u16,
    /// Trailing word 1 (semantics TBD).
    pub a: u32,
    /// Trailing word 2 (semantics TBD).
    pub b: u32,
    /// Trailing word 3 (semantics TBD).
    pub c: u32,
}

/// A parsed `.oja` header + frame table. Only the envelope is
/// verified (magic, count, coherent frame geometry on real 7.55
/// `Note_Click.oja`: shrinking centred frames); the frame-data
/// codec and the `a`/`b`/`c` words are TBD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oja {
    /// Container tag (observed: 1).
    pub tag: u16,
    /// Frame headers in stored order.
    pub frames: Vec<OjaFrame>,
}

/// Parse an `.oja` envelope: magic + `count` 20-byte frame records
/// (`i16 x/y`, `u16 w/h`, three raw `u32` words).
///
/// # Errors
///
/// `FileError::TooShort` when the frame table overruns the buffer.
pub fn parse_oja(bytes: &[u8]) -> Result<Oja, FileError> {
    if bytes.len() < OJA_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    let tag = read_u16le(bytes, 0)?;
    if read_u16le(bytes, 2)? != OJA_MAGIC {
        return Err(unknown_format(bytes));
    }
    let count = read_u16le(bytes, 4)? as usize;
    // Frame data stays unsliced (codec TBD); only the table must fit.
    let _ = dir_end(count, OJA_RECORD_LEN, OJA_HEADER_LEN, bytes.len())?;
    let mut frames = Vec::with_capacity(count.min(1024));
    for i in 0..count {
        let off = OJA_HEADER_LEN + i * OJA_RECORD_LEN;
        frames.push(OjaFrame {
            x: read_i16le(bytes, off)?,
            y: read_i16le(bytes, off + 2)?,
            w: read_u16le(bytes, off + 4)?,
            h: read_u16le(bytes, off + 6)?,
            a: read_u32le(bytes, off + 8)?,
            b: read_u32le(bytes, off + 12)?,
            c: read_u32le(bytes, off + 16)?,
        });
    }
    Ok(Oja { tag, frames })
}

/// One prefixed row span as the 7.55 bodies store it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55RowSpan<'a> {
    /// X offset of the span inside its image row.
    pub x: u16,
    /// Y row of the span inside its image.
    pub y: u16,
    /// Span pixels, little-endian RGB555 (`width * 2` bytes).
    pub pixels: &'a [u8],
}

/// Prefixed row-group header: `[u16 width][u16 x][u16 y]` before
/// the span's pixels.
pub const SPRITE55_ROW_HEADER_LEN: usize = 6;

/// Zero terminator word closing every prefixed image.
pub const SPRITE55_ROW_TAIL_LEN: usize = 2;

/// A parsed prefixed sprite-55 container (the `.oji` bodies of
/// `docs/codec-coverage.md`): shared header and record table with
/// [`Sprite55`], but each image stores a run of row groups of `u16`
/// `width`, `x`, `y` words followed by `width` RGB555 pixels, plus
/// one zero terminator word, instead of one raw blob. The `y` words
/// walk rows in order: the first group sits on row 0, a group either
/// repeats the previous row (x runs accumulate along it) or steps one
/// row down, so every row carries at least one span and the walk ends
/// on row `h - 1`. Every span stays inside its row
/// (`x + width <= w`), and pixels no span covers stay colour-key
/// (transparent): the `Playing_Loding_Lankview.oji` bottom rows
/// trim both sides for rounded corners, `BG_Ani.oji` and
/// `Avatar_Shadow.oji` skip transparent margins outright, both
/// MybagSelect buttons paint each row as two edge spans, and the
/// `Playing_ChatWindow.oji` border rows pack up to 18 runs. Verified
/// on all ten prefixed bodies (36 images) in the real 7.55 archives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite55Prefixed<'a> {
    /// Container tag (observed: 1).
    pub tag: u16,
    /// Transparent colour key (observed: `0x0000`).
    pub colorkey: u16,
    /// Image records in stored order (`size` covers the whole
    /// row-group walk of the image).
    pub images: Vec<Sprite55Image>,
    /// Row spans per image, in stored order (row 0 first).
    pub rows: Vec<Vec<Sprite55RowSpan<'a>>>,
}

/// Walk one image's row groups: consecutive groups of `u16 width`,
/// `u16 x`, `u16 y` followed by `width` pixels, closed by one zero
/// word. Rows arrive in order: the first group sits on row 0, a group
/// repeats the previous row (x runs accumulate along it) or steps one
/// row down, and the walk ends on row `h - 1` — every row carries at
/// least one span. Any other layout (raw blob, compressed frames, a
/// dropped, skipped, or reordered row, a span past the right edge) is
/// not a prefixed image.
fn parse_prefixed_rows<'a>(
    blob: &'a [u8],
    image: &Sprite55Image,
    sample: &[u8],
) -> Result<Vec<Sprite55RowSpan<'a>>, FileError> {
    let data = &blob[image.data_offset..image.data_offset + image.size as usize];
    let mut rest = data;
    let mut spans = Vec::with_capacity(image.h as usize);
    let mut last_row = None;
    loop {
        if rest.len() < SPRITE55_ROW_HEADER_LEN + SPRITE55_ROW_TAIL_LEN {
            return Err(unknown_format(sample));
        }
        let width = read_u16le(rest, 0)?;
        let x = read_u16le(rest, 2)?;
        let y = read_u16le(rest, 4)?;
        let row_in_order = match last_row {
            None => y == 0,
            Some(row) => y == row || u32::from(y) == u32::from(row) + 1,
        };
        let span = usize::from(width)
            .checked_mul(2)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: rest.len(),
            })?;
        let need = SPRITE55_ROW_HEADER_LEN
            .checked_add(span)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: rest.len(),
            })?;
        if u32::from(y) >= u32::from(image.h)
            || !row_in_order
            || u32::from(x) + u32::from(width) > u32::from(image.w)
            || rest.len() < need
        {
            return Err(unknown_format(sample));
        }
        spans.push(Sprite55RowSpan {
            x,
            y,
            pixels: &rest[SPRITE55_ROW_HEADER_LEN..need],
        });
        last_row = Some(y);
        rest = &rest[need..];
        if rest.len() == SPRITE55_ROW_TAIL_LEN {
            break;
        }
    }
    if rest.len() != SPRITE55_ROW_TAIL_LEN || read_u16le(rest, 0)? != 0 {
        return Err(unknown_format(sample));
    }
    if last_row.is_none_or(|row| u32::from(row) + 1 != u32::from(image.h)) {
        return Err(unknown_format(sample));
    }
    Ok(spans)
}

/// Parse a prefixed sprite-55 container: shared header/record
/// contract plus the whole-body accounting ([`parse_sprite55`]),
/// then the row-group walk per image. Rejects every non-prefixed
/// layout, including raw and compressed frames.
///
/// # Errors
///
/// `FileError::UnknownFormat` on a row walk that does not match the
/// prefixed layout, `FileError::Truncated`/`TooShort` on broken
/// container accounting.
pub fn parse_sprite55_prefixed(bytes: &[u8]) -> Result<Sprite55Prefixed<'_>, FileError> {
    let dir = parse_sprite55_dir(bytes)?;
    if dir.dir_end + dir.stored != bytes.len() {
        return Err(FileError::Truncated {
            need: dir.dir_end + dir.stored,
            have: bytes.len(),
        });
    }
    let blob = &bytes[dir.dir_end..];
    let rows = dir
        .images
        .iter()
        .map(|image| parse_prefixed_rows(blob, image, bytes))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Sprite55Prefixed {
        tag: dir.tag,
        colorkey: dir.colorkey,
        images: dir.images,
        rows,
    })
}

#[cfg(test)]
mod sprite55_prefixed_tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::parse_opi;

    /// Container blob in the plain sprite-55 shape (header + one
    /// record per image + concatenated stored data).
    fn prefixed55_body(images: &[(u16, u16, u16, u16, Vec<u8>)]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(images.len() as u16).to_le_bytes());
        blob.extend_from_slice(&0x0000u16.to_le_bytes());
        let mut run = 0u32;
        for (x, y, w, h, data) in images {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&y.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&run.to_le_bytes());
            blob.extend_from_slice(&(data.len() as u32).to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
            run += data.len() as u32;
        }
        for (_, _, _, _, data) in images {
            blob.extend_from_slice(data);
        }
        blob
    }

    /// One row group: `[u16 width][u16 x][u16 y]` + pixel words.
    fn row_group(width: u16, x: u16, y: u16, words: &[u16]) -> Vec<u8> {
        assert_eq!(words.len(), width as usize);
        let mut group = Vec::new();
        group.extend_from_slice(&width.to_le_bytes());
        group.extend_from_slice(&x.to_le_bytes());
        group.extend_from_slice(&y.to_le_bytes());
        for word in words {
            group.extend_from_slice(&word.to_le_bytes());
        }
        group
    }

    /// One prefixed image's stored data: the row groups plus the
    /// zero terminator word.
    fn prefixed_image(groups: &[Vec<u8>]) -> Vec<u8> {
        let mut data = Vec::new();
        for group in groups {
            data.extend_from_slice(group);
        }
        data.extend_from_slice(&0u16.to_le_bytes());
        data
    }

    /// The full-row-plus-trimmed-row shape (4x2): row 0 covers the
    /// whole width, row 1 starts at x 1 and stops one pixel short —
    /// the `Playing_Loding_Lankview.oji` rounded-corner shape.
    fn trimmed_body() -> Vec<u8> {
        let data = prefixed_image(&[
            row_group(4, 0, 0, &[0x7C00, 0x03E0, 0x001F, 0x7C00]),
            row_group(2, 1, 1, &[0x03E0, 0x001F]),
        ]);
        prefixed55_body(&[(3, 5, 4, 2, data)])
    }

    #[test]
    fn prefixed_walks_row_spans_in_stored_order() {
        let body = trimmed_body();
        let sprite = parse_sprite55_prefixed(&body).unwrap();
        assert_eq!(sprite.tag, 1);
        assert_eq!(sprite.colorkey, 0x0000);
        assert_eq!(sprite.images.len(), 1);
        assert_eq!(sprite.images[0].w, 4);
        assert_eq!(sprite.rows.len(), 1);
        let spans = &sprite.rows[0];
        assert_eq!(spans.len(), 2);
        assert_eq!((spans[0].x, spans[0].y), (0, 0));
        assert_eq!(
            spans[0].pixels,
            &[0x00, 0x7C, 0xE0, 0x03, 0x1F, 0x00, 0x00, 0x7C]
        );
        assert_eq!((spans[1].x, spans[1].y), (1, 1));
        assert_eq!(spans[1].pixels, &[0xE0, 0x03, 0x1F, 0x00]);
    }

    /// Repeated-y shape (codec gap ④): a middle row stored as two
    /// x runs with a transparent gap between them — the MybagSelect
    /// edge-span shape. Spans keep stored order and share the row's
    /// y; the walk never invents a span for the gap.
    #[test]
    fn repeated_y_groups_accumulate_x_runs_on_one_row() {
        let data = prefixed_image(&[
            row_group(4, 0, 0, &[0x7C00, 0x03E0, 0x001F, 0x7C00]),
            row_group(1, 0, 1, &[0x03E0]),
            row_group(2, 2, 1, &[0x001F, 0x7C00]),
            row_group(4, 0, 2, &[0x7C00, 0x03E0, 0x001F, 0x7C00]),
        ]);
        let body = prefixed55_body(&[(0, 0, 4, 3, data)]);
        let sprite = parse_sprite55_prefixed(&body).unwrap();
        let spans = &sprite.rows[0];
        assert_eq!(spans.len(), 4);
        assert_eq!((spans[0].x, spans[0].y), (0, 0));
        assert_eq!((spans[1].x, spans[1].y), (0, 1));
        assert_eq!(spans[1].pixels, &[0xE0, 0x03]);
        assert_eq!((spans[2].x, spans[2].y), (2, 1));
        assert_eq!(spans[2].pixels, &[0x1F, 0x00, 0x00, 0x7C]);
        assert_eq!((spans[3].x, spans[3].y), (0, 2));
    }

    /// Repository-local `install-root` (git-ignored game data).
    fn install_archive(name: &str) -> Option<Vec<u8>> {
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image")
                .join(name),
        )
        .ok()
    }

    /// Real repeated-y bodies (codec gap ④): the in-play chat window
    /// and both my-bag buttons walk with repeated y words — rows
    /// arrive in order and some rows carry several x runs (each
    /// MybagSelect row paints two edge spans; one chat-window border
    /// row packs 18). Container accounting lands byte-exact. Skips
    /// when `install-root` is absent.
    /// One real repeated-y body's pinned shape: archive and body
    /// names, image `w×h`, span count, and rows grouped by how many
    /// x runs paint them: `(runs, row count)`.
    struct RepeatedYCase {
        archive: &'static str,
        body: &'static str,
        w: u16,
        h: u16,
        span_count: usize,
        rows_per_run_count: &'static [(usize, usize)],
    }

    #[test]
    fn repeated_y_oji_bodies_walk_from_the_archives() {
        let cases = [
            RepeatedYCase {
                archive: "Interface1.opi",
                body: "Myroom_MybagSelect.oji",
                w: 86,
                h: 69,
                span_count: 134,
                rows_per_run_count: &[(1, 4), (2, 65)],
            },
            RepeatedYCase {
                archive: "Interface1.opi",
                body: "Shop_MybagSelect.oji",
                w: 86,
                h: 69,
                span_count: 134,
                rows_per_run_count: &[(1, 4), (2, 65)],
            },
            RepeatedYCase {
                archive: "Playing1.opi",
                body: "Playing_ChatWindow.oji",
                w: 596,
                h: 558,
                span_count: 671,
                rows_per_run_count: &[(1, 461), (2, 96), (18, 1)],
            },
        ];
        for case in cases {
            let name = case.body;
            let Some(bytes) = install_archive(case.archive) else {
                return;
            };
            let entries = parse_opi(&bytes).expect("archive table parses");
            let entry = entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("{name} in {}", case.archive));
            let body = &bytes[entry.data_offset..entry.data_offset + entry.size as usize];
            let sprite =
                parse_sprite55_prefixed(body).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(sprite.images.len(), 1, "{name}");
            let image = &sprite.images[0];
            assert_eq!((image.w, image.h), (case.w, case.h), "{name}");
            let spans = &sprite.rows[0];
            assert_eq!(spans.len(), case.span_count, "{name}");
            let mut rows_per_y: BTreeMap<u16, usize> = BTreeMap::new();
            for span in spans {
                *rows_per_y.entry(span.y).or_insert(0) += 1;
            }
            let mut run_counts: BTreeMap<usize, usize> = BTreeMap::new();
            for row_runs in rows_per_y.into_values() {
                *run_counts.entry(row_runs).or_insert(0) += 1;
            }
            let histogram: Vec<(usize, usize)> = run_counts.into_iter().collect();
            assert_eq!(
                histogram, case.rows_per_run_count,
                "{name} rows per run count"
            );
            assert_eq!(spans.last().map(|span| span.y), Some(case.h - 1), "{name}");
            if name.ends_with("MybagSelect.oji") {
                // Leading full-width fill row of one stored colour:
                // the decode invents no pixel outside the spans.
                assert_eq!((spans[0].x, spans[0].y), (0, 0), "{name}");
                assert_eq!(spans[0].pixels, &[0xDD, 0x00].repeat(usize::from(case.w)));
            }
            if name == "Playing_ChatWindow.oji" {
                assert_eq!((spans[0].x, spans[0].y), (0, 0));
                assert_eq!(spans[0].pixels.len(), 2);
                assert_eq!((spans[1].x, spans[1].y), (30, 0));
                assert_eq!(spans[1].pixels.len(), 702);
            }
        }
    }

    #[test]
    fn prefixed_rejects_every_other_layout() {
        // A plain raw blob is not a row-group walk (no terminator).
        let raw = prefixed55_body(&[(0, 0, 4, 2, [0xAA, 0xBB].repeat(8))]);
        assert!(parse_sprite55_prefixed(&raw).is_err());
        // Compressed-style stored bytes (the negative-delta body
        // class) walk as neither groups nor terminator.
        let junk = prefixed55_body(&[(0, 0, 4, 2, [0x51, 0x00].repeat(3))]);
        assert!(parse_sprite55_prefixed(&junk).is_err());
        // Non-zero terminator word.
        let mut bad_tail = trimmed_body();
        *bad_tail.last_mut().unwrap() = 1;
        assert!(parse_sprite55_prefixed(&bad_tail).is_err());
        // A dropped last row breaks the row coverage (the walk must
        // end on row h-1).
        let short = prefixed_image(&[row_group(4, 0, 0, &[0x7C00; 4])]);
        let short_body = prefixed55_body(&[(0, 0, 4, 2, short)]);
        assert!(parse_sprite55_prefixed(&short_body).is_err());
        // A first row past row 0 skips the top row.
        let skipped_top = prefixed_image(&[
            row_group(4, 0, 1, &[0x7C00; 4]),
            row_group(4, 0, 1, &[0x03E0; 4]),
        ]);
        let skipped_top_body = prefixed55_body(&[(0, 0, 4, 2, skipped_top)]);
        assert!(parse_sprite55_prefixed(&skipped_top_body).is_err());
        // A +2 row jump leaves a row unpainted.
        let skipped_middle = prefixed_image(&[
            row_group(4, 0, 0, &[0x7C00; 4]),
            row_group(4, 0, 2, &[0x03E0; 4]),
        ]);
        let skipped_middle_body = prefixed55_body(&[(0, 0, 4, 3, skipped_middle)]);
        assert!(parse_sprite55_prefixed(&skipped_middle_body).is_err());
        // A row revisited after advancing walks out of order.
        let revisited = prefixed_image(&[
            row_group(4, 0, 0, &[0x7C00; 4]),
            row_group(4, 0, 1, &[0x03E0; 4]),
            row_group(4, 0, 0, &[0x001F; 4]),
        ]);
        let revisited_body = prefixed55_body(&[(0, 0, 4, 2, revisited)]);
        assert!(parse_sprite55_prefixed(&revisited_body).is_err());
        // A span past the right edge breaks the row containment.
        let wide = prefixed_image(&[
            row_group(4, 0, 0, &[0x7C00; 4]),
            row_group(4, 1, 1, &[0x03E0; 4]),
        ]);
        let wide_body = prefixed55_body(&[(0, 0, 4, 2, wide)]);
        assert!(parse_sprite55_prefixed(&wide_body).is_err());
        // Rows out of stored order break the positional y walk.
        let reordered = prefixed_image(&[
            row_group(4, 0, 1, &[0x7C00; 4]),
            row_group(4, 0, 0, &[0x03E0; 4]),
        ]);
        let reordered_body = prefixed55_body(&[(0, 0, 4, 2, reordered)]);
        assert!(parse_sprite55_prefixed(&reordered_body).is_err());
        // Container-level accounting still applies (one byte short).
        let body = trimmed_body();
        assert!(parse_sprite55_prefixed(&body[..body.len() - 1]).is_err());
        assert!(matches!(
            parse_sprite55_prefixed(b"short"),
            Err(FileError::TooShort(_))
        ));
    }
}
