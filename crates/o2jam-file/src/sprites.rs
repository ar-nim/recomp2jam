//! Sprite bodies: sprite-55 containers (plain + strided), BND rects,
//! and OJA animation tables.
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
    /// [`Sprite55::is_raw_rgb555`] holds; codec TBD otherwise).
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
    /// Outlier classes (`.oji` sub-directories/prefixed frames,
    /// RLE-compressed frames) return false and need their codec
    /// mapped before decoding. Strided combo atlases (see
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
