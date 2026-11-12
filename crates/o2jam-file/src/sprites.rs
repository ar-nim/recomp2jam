//! Sprite bodies: sprite-55 containers (plain, strided, and
//! prefixed), BND rects, and OJA animation tables (envelope plus
//! decoded frame planes, tabulated and recordless).
//!
//! Verified on real 7.55 files (docs/file-formats/opi.md,
//! docs/codec-coverage.md).

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

/// Magic of the animation container (`.oja` bodies):
/// `tag + 0x555A + count + reserved`, then `count` 20-byte frame
/// records and one concatenated `[pixel plane][alpha plane]` blob
/// per frame. See [`parse_oja55`] for the decoded layout (codec gap
/// ⑤); this envelope reader covers the header and frame table only.
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
    /// Trailing word `a`: byte offset of the frame's row-group pixel
    /// plane from the blob start ([`parse_oja55`] steps a container
    /// -specific plane base first).
    pub a: u32,
    /// Trailing word `b`: stored byte length of the pixel plane.
    pub b: u32,
    /// Trailing word `c`: stored byte length of the alpha plane that
    /// follows the pixels.
    pub c: u32,
}

/// A parsed `.oja` header + frame table — the envelope only. The
/// decoded planes live in [`Oja55`] (codec gap ⑤). Verified on real
/// 7.55 `Note_Click.oja` (shrinking centred frames) and
/// `Playing_Effect_Jam.oja` (ten 300x300 flash frames).
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
    // Frame data stays unsliced here; decode planes via parse_oja55.
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

/// One decoded `.oja` animation frame: record geometry, the frame's
/// row-group pixel plane, and its alpha plane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oja55Frame<'a> {
    /// Placement anchor, pixels (negative: centred effects).
    pub x: i16,
    /// Placement anchor, pixels.
    pub y: i16,
    /// Frame width, pixels.
    pub w: u16,
    /// Frame height, pixels.
    pub h: u16,
    /// Record word `a`: byte offset of this frame's pixel plane from
    /// the record-table end. The real containers keep the planes
    /// chained (`a[n + 1] == a[n] + b[n] + c[n]`) with the first
    /// plane straight after the table (smallest shift 0 on all three
    /// 7.55 bodies); [`parse_oja55`] still scans record-stride
    /// shifts, so a body that prefixes the blob keeps parsing.
    pub a: u32,
    /// Record word `b`: stored byte length of the row-group pixel
    /// plane, including its zero terminator word.
    pub b: u32,
    /// Record word `c`: stored byte length of the alpha plane.
    pub c: u32,
    /// Pixel-plane row spans in stored order.
    pub rows: Vec<Sprite55RowSpan<'a>>,
    /// Alpha/intensity plane, sliced verbatim. Its byte count tracks
    /// the covered-pixel count (exact on all ten
    /// `Playing_Effect_Jam.oja` frames; up to 3 bytes short on the
    /// click pair — the packer's alpha pass dropped edge pixels), and
    /// its values rise with pixel luminance (dim ring edge ≈ 0–9,
    /// bright core ≈ 255): an intensity map for the effect. The exe
    /// reads it right after the pixels and uploads it with no codec
    /// between (claims C7); the exact blend role stays open
    /// (docs/claims.md C7, docs/ref-opi-divergences.md H5).
    pub alpha: &'a [u8],
}

/// A decoded `.oja` container (codec gap ⑤): header, per-frame
/// row-group pixel planes, alpha planes, and the recordless frames
/// decoded from the trailing block. Pixel words store 0 as
/// the transparent colour (the header carries no colour-key word;
/// claims A12 key `0x0000`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oja55<'a> {
    /// Container tag (observed: 1).
    pub tag: u16,
    /// Header word after the count (observed: 0; role unknown).
    pub reserved: u16,
    /// Decoded tabulated frames in stored order — the record-defined
    /// animation (the exe reads the table only).
    pub frames: Vec<Oja55Frame<'a>>,
    /// Recordless frames decoded from the trailing block (see
    /// [`Oja55TrailingFrame`]): `Playing_Effect_Jam.oja` stores six
    /// 300x300 flash frames after its ten tabulated ones, with no
    /// frame records.
    pub trailing_frames: Vec<Oja55TrailingFrame<'a>>,
    /// Blob bytes still unaccounted for after the recordless decode.
    /// `Playing_Effect_Jam.oja` opens its trailing block with a
    /// 9,159-byte alpha-profile block (31 canvas rows on the 300-byte
    /// row grid, value 0 past row 30) whose pixel plane is absent —
    /// recorded raw, never invented. Empty on the other real bodies
    /// and on bodies without trailing frames.
    pub trailing: &'a [u8],
}

/// One recordless `.oja` frame decoded from the trailing block:
/// `Playing_Effect_Jam.oja` stores six 300x300 flash frames after its
/// ten tabulated ones with no frame records — no anchors, no `a`/`b`/
/// `c` words. The plane walk pins what a record would state: every
/// span stays inside the tabulated frame canvas, each walk lands on
/// its zero terminator, and all six real walks end on the canvas's
/// bottom row (bottom-anchored flash frames).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Oja55TrailingFrame<'a> {
    /// Frame width, pixels (the tabulated frames' canvas; the real
    /// spans reach its full width, `max(x + width) == w`).
    pub w: u16,
    /// Frame height, pixels (the real spans reach its bottom row,
    /// `max(y) == h - 1`).
    pub h: u16,
    /// Pixel-plane row spans in stored order (the same row-group walk
    /// as the tabulated frames).
    pub rows: Vec<Sprite55RowSpan<'a>>,
    /// Mask plane sliced verbatim from the bytes between this frame's
    /// pixel plane and the next frame's plane (the last runs to the
    /// blob end). Unlike the tabulated records' byte-per-covered-
    /// pixel alpha, these masks are canvas-row maps: 300-byte rows,
    /// 299 of them (the first frame's is six bytes short), all zero
    /// past the glow rim, values rising with pixel luminance. The
    /// exact blend role stays open with claims C7; the exe's fixed
    /// per-frame read cannot consume these compacted planes either
    /// (claims C9, docs/claims.md).
    pub alpha: &'a [u8],
}

/// Parse a decoded `.oja` container: magic, chained frame records
/// (`a[n + 1] == a[n] + b[n] + c[n]`), then per frame a row-group
/// pixel plane (`[u16 width][u16 x][u16 y]` + `width` RGB555 words,
/// closed by one zero terminator that must land exactly on `a + b`)
/// and the alpha slice of `c` bytes that follows it. Rows move
/// forward only and spans stay inside their row; effect frames may
/// start and end above rows `0`/`h - 1` (the ring circles only cover
/// the rows they reach, unlike the full-height `.oji` walks). The
/// block after the last frame's planes decodes as recordless
/// trailing frames ([`Oja55TrailingFrame`]) when it holds
/// bottom-anchored plane walks, and stays raw ([`Oja55::trailing`])
/// otherwise.
/// Verified on all three real 7.55 bodies (`Note_Click.oja`,
/// `LongNote_Effect.oja` — byte-identical — and
/// `Playing_Effect_Jam.oja`, docs/codec-coverage.md).
///
/// # Errors
///
/// `FileError::TooShort` when the record table overruns the buffer,
/// `FileError::UnknownFormat` on a broken chain or a plane walk that
/// does not match the layout.
/// One `.oja` v5.5 frame table record: `x`, `y`, `w`, `h`, then the
/// plane offsets `a` (pixel rows), `b` (pixel bytes), `c` (alpha
/// bytes).
type Oja55Record = (i16, i16, u16, u16, u32, u32, u32);

/// Read the `.oja` v5.5 header (tag, magic, count, reserved) and the
/// `count`-entry frame table after it.
fn read_oja55_records(bytes: &[u8]) -> Result<(u16, u16, Vec<Oja55Record>), FileError> {
    if bytes.len() < OJA_HEADER_LEN {
        return Err(FileError::TooShort(bytes.len()));
    }
    if read_u16le(bytes, 2)? != OJA_MAGIC {
        return Err(unknown_format(bytes));
    }
    let tag = read_u16le(bytes, 0)?;
    let count = read_u16le(bytes, 4)? as usize;
    let reserved = read_u16le(bytes, 6)?;
    let mut records = Vec::with_capacity(count.min(1024));
    for i in 0..count {
        let off = OJA_HEADER_LEN + i * OJA_RECORD_LEN;
        records.push((
            read_i16le(bytes, off)?,
            read_i16le(bytes, off + 2)?,
            read_u16le(bytes, off + 4)?,
            read_u16le(bytes, off + 6)?,
            read_u32le(bytes, off + 8)?,
            read_u32le(bytes, off + 12)?,
            read_u32le(bytes, off + 16)?,
        ));
    }
    Ok((tag, reserved, records))
}

/// Validate the frame table: each frame's planes chain straight after
/// the previous one's (`a[n + 1] == a[n] + b[n] + c[n]`), and the table
/// is non-empty (empty tables have no real-body precedent; the plane
/// layout is meaningless without frames).
fn validate_oja55_chain(bytes: &[u8], records: &[Oja55Record]) -> Result<(), FileError> {
    for pair in records.windows(2) {
        let (_, _, _, _, a, b, c) = pair[0];
        let (_, _, _, _, a_next, _, _) = pair[1];
        let end = a
            .checked_add(b)
            .and_then(|end| end.checked_add(c))
            .ok_or_else(|| unknown_format(bytes))?;
        if a_next != end {
            return Err(unknown_format(bytes));
        }
    }
    if records.is_empty() {
        return Err(unknown_format(bytes));
    }
    Ok(())
}

/// Lay out every frame's planes at one candidate base shift: `blob_start`
/// plus the record-stride shift `k * OJA_RECORD_LEN`. Returns the frames
/// when every walk lands inside the buffer (`None` asks the caller to
/// try the next shift).
fn layout_oja55_frames<'a>(
    bytes: &'a [u8],
    blob_start: usize,
    shift: usize,
    records: &[Oja55Record],
) -> Option<Vec<Oja55Frame<'a>>> {
    let mut frames = Vec::with_capacity(records.len());
    for &(x, y, w, h, a, b, c) in records {
        let base = usize::try_from(a)
            .ok()
            .and_then(|a| a.checked_add(shift))
            .and_then(|a| a.checked_add(blob_start))?;
        let pixel_len = usize::try_from(b).unwrap_or(usize::MAX);
        let alpha_len = usize::try_from(c).unwrap_or(usize::MAX);
        let pixels_end = base.checked_add(pixel_len)?;
        let alpha_end = pixels_end.checked_add(alpha_len)?;
        if alpha_end > bytes.len() {
            return None;
        }
        let rows = parse_oja_plane_rows(bytes, base, pixels_end, w, h)?;
        frames.push(Oja55Frame {
            x,
            y,
            w,
            h,
            a,
            b,
            c,
            rows,
            alpha: &bytes[pixels_end..alpha_end],
        });
    }
    Some(frames)
}

pub fn parse_oja55(bytes: &[u8]) -> Result<Oja55<'_>, FileError> {
    let (tag, reserved, records) = read_oja55_records(bytes)?;
    let count = records.len();
    let blob_start = dir_end(count, OJA_RECORD_LEN, OJA_HEADER_LEN, bytes.len())?;
    validate_oja55_chain(bytes, &records)?;
    // Plane base: the smallest record-stride shift where every
    // frame's walk lands exactly on `a + b`. All three real 7.55
    // bodies chain their planes straight after the table (shift 0);
    // the scan guards layouts that prefix the blob, and stops once
    // the first frame's planes would run past the blob.
    let (_, _, _, _, a0, b0, c0) = records[0];
    let first_end = a0
        .checked_add(b0)
        .and_then(|end| end.checked_add(c0))
        .ok_or_else(|| unknown_format(bytes))?;
    let mut k = 0usize;
    loop {
        let shift = k * OJA_RECORD_LEN;
        let scan_floor = usize::try_from(first_end)
            .ok()
            .and_then(|end| end.checked_add(blob_start))
            .and_then(|end| end.checked_add(shift));
        let Some(scan_floor) = scan_floor else {
            return Err(unknown_format(bytes));
        };
        if scan_floor > bytes.len() {
            return Err(unknown_format(bytes));
        }
        if let Some(frames) = layout_oja55_frames(bytes, blob_start, shift, &records) {
            let last = records[count - 1];
            let planes_end = usize::try_from(last.4)
                .ok()
                .and_then(|a| a.checked_add(usize::try_from(last.5).unwrap_or(usize::MAX)))
                .and_then(|a| a.checked_add(usize::try_from(last.6).unwrap_or(usize::MAX)))
                .and_then(|a| a.checked_add(shift))
                .and_then(|a| a.checked_add(blob_start))
                .ok_or_else(|| unknown_format(bytes))?;
            // The trailing block decodes as recordless plane+mask
            // frames over the last tabulated record's canvas; a
            // block with no bottom-anchored plane walk stays raw.
            let (leading, trailing_frames) =
                parse_oja55_trailing(&bytes[planes_end..], last.2, last.3);
            return Ok(Oja55 {
                tag,
                reserved,
                frames,
                trailing_frames,
                trailing: leading,
            });
        }
        // No base matched at this stride: try the next record step.
        k += 1;
    }
}

/// Parse the recordless trailing block after the last tabulated
/// frame's planes: an optional leading block of unaccounted bytes,
/// then zero or more `pixel plane + mask` frames that fill the block
/// to its end exactly. Plane starts are the offsets whose
/// [`walk_oja_trailing_plane`] lands; each mask runs to the next
/// plane (the last to the block end), so the accounting is exact by
/// construction. A block with no plane walk at all decodes nothing:
/// the whole block comes back raw (honest negative, never invented
/// frames).
fn parse_oja55_trailing<'a>(
    block: &'a [u8],
    canvas_w: u16,
    canvas_h: u16,
) -> (&'a [u8], Vec<Oja55TrailingFrame<'a>>) {
    let mut frames = Vec::new();
    let mut cursor = 0usize;
    let mut leading_end = None;
    while cursor < block.len() {
        let Some((start, (rows, term))) = (cursor..block.len()).find_map(|off| {
            walk_oja_trailing_plane(block, off, canvas_w, canvas_h).map(|walk| (off, walk))
        }) else {
            break;
        };
        if leading_end.is_none() {
            leading_end = Some(start);
        }
        let alpha_end = (term..block.len())
            .find(|&off| walk_oja_trailing_plane(block, off, canvas_w, canvas_h).is_some())
            .unwrap_or(block.len());
        frames.push(Oja55TrailingFrame {
            w: canvas_w,
            h: canvas_h,
            rows,
            alpha: &block[term..alpha_end],
        });
        cursor = alpha_end;
    }
    (&block[..leading_end.unwrap_or(block.len())], frames)
}

/// Walk one recordless trailing pixel plane candidate from `start`:
/// the same row groups as [`parse_oja_plane_rows`], over the last
/// tabulated record's canvas, but with the plane end unknown — the
/// zero terminator is discovered. A candidate is a plane only when
/// its walk ends on the canvas's bottom row: all six real trailing
/// frames are bottom-anchored flash frames, while the mask bytes
/// between planes only ever produce walks that stop high (their
/// bytes are value profiles, not row groups). Returns the spans and
/// the offset just past the terminator.
fn walk_oja_trailing_plane<'a>(
    block: &'a [u8],
    start: usize,
    w: u16,
    h: u16,
) -> Option<(Vec<Sprite55RowSpan<'a>>, usize)> {
    let mut spans = Vec::new();
    let mut off = start;
    let mut last_y: Option<u16> = None;
    while off + SPRITE55_ROW_TAIL_LEN <= block.len() {
        let width = usize::from(read_u16le(block, off).ok()?);
        if width == 0 {
            let anchored = spans
                .last()
                .is_some_and(|span: &Sprite55RowSpan| u32::from(span.y) + 1 == u32::from(h));
            return anchored.then_some({
                let end = off + SPRITE55_ROW_TAIL_LEN;
                (spans, end)
            });
        }
        let pixels_end = off
            .checked_add(SPRITE55_ROW_HEADER_LEN)?
            .checked_add(width.checked_mul(2)?)?;
        if pixels_end > block.len() {
            return None;
        }
        let x = read_u16le(block, off + 2).ok()?;
        let y = read_u16le(block, off + 4).ok()?;
        if usize::from(x) + width > usize::from(w) || y >= h || last_y.is_some_and(|row| y < row) {
            return None;
        }
        spans.push(Sprite55RowSpan {
            x,
            y,
            pixels: &block[off + SPRITE55_ROW_HEADER_LEN..pixels_end],
        });
        last_y = Some(y);
        off = pixels_end;
    }
    None
}

/// Walk one `.oja` pixel plane: consecutive `[u16 width][u16 x][u16 y]`
/// row groups followed by `width` RGB555 pixel words, closed by one
/// zero terminator word that must land exactly on `end`. Rows move
/// forward only; effect frames may start and end above rows `0` and
/// `h - 1` (the ring circles only reach the rows they cover — the
/// jam flash's early frames paint rows 12..146 of 300). Any span
/// outside its row, any backward row step, a missing or misplaced
/// terminator is not a plane.
fn parse_oja_plane_rows<'a>(
    blob: &'a [u8],
    start: usize,
    end: usize,
    w: u16,
    h: u16,
) -> Option<Vec<Sprite55RowSpan<'a>>> {
    let mut spans = Vec::new();
    let mut off = start;
    let mut last_y: Option<u16> = None;
    while off + SPRITE55_ROW_TAIL_LEN <= end {
        let width = usize::from(read_u16le(blob, off).ok()?);
        if width == 0 {
            return (off + SPRITE55_ROW_TAIL_LEN == end).then_some(spans);
        }
        let pixels_end = off + SPRITE55_ROW_HEADER_LEN + width * 2;
        if pixels_end > end {
            return None;
        }
        let x = read_u16le(blob, off + 2).ok()?;
        let y = read_u16le(blob, off + 4).ok()?;
        if usize::from(x) + width > usize::from(w)
            || usize::from(y) >= usize::from(h)
            || last_y.is_some_and(|row| y < row)
        {
            return None;
        }
        spans.push(Sprite55RowSpan {
            x,
            y,
            pixels: &blob[off + SPRITE55_ROW_HEADER_LEN..pixels_end],
        });
        last_y = Some(y);
        off = pixels_end;
    }
    None
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

#[cfg(test)]
mod oja55_tests {
    use std::path::PathBuf;

    use super::*;
    use crate::parse_opi;

    /// One `.oja` pixel-plane row group: `[u16 width][u16 x][u16 y]`
    /// plus `width` little-endian RGB555 words.
    fn oja_row_group(width: u16, x: u16, y: u16, words: &[u16]) -> Vec<u8> {
        assert_eq!(usize::from(width), words.len());
        let mut group = Vec::new();
        group.extend_from_slice(&width.to_le_bytes());
        group.extend_from_slice(&x.to_le_bytes());
        group.extend_from_slice(&y.to_le_bytes());
        for word in words {
            group.extend_from_slice(&word.to_le_bytes());
        }
        group
    }

    /// One decoded frame's stored planes: the row-group walk (with
    /// its zero terminator) followed by the alpha bytes.
    fn oja_plane(groups: &[Vec<u8>], alpha: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let mut pixels = Vec::new();
        for group in groups {
            pixels.extend_from_slice(group);
        }
        pixels.extend_from_slice(&0u16.to_le_bytes());
        (pixels, alpha.to_vec())
    }

    /// An `.oja` body in the real layout: header, records whose `a`
    /// words chain through the plane sizes, an optional unexplained
    /// leading block before the first plane, then the concatenated
    /// `[pixel plane][alpha plane]` per frame. `extra` appends
    /// untalphabet trailing bytes after the last plane (the
    /// `Playing_Effect_Jam.oja` shape).
    #[allow(clippy::type_complexity)]
    fn oja55_body(
        leading: &[u8],
        extra: &[u8],
        frames: &[((i16, i16, u16, u16), (Vec<u8>, Vec<u8>))],
    ) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&OJA_MAGIC.to_le_bytes());
        blob.extend_from_slice(&(frames.len() as u16).to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        let mut run = 0u32;
        for ((x, y, w, h), (pixels, alpha)) in frames {
            blob.extend_from_slice(&x.to_le_bytes());
            blob.extend_from_slice(&y.to_le_bytes());
            blob.extend_from_slice(&w.to_le_bytes());
            blob.extend_from_slice(&h.to_le_bytes());
            blob.extend_from_slice(&run.to_le_bytes());
            blob.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
            blob.extend_from_slice(&(alpha.len() as u32).to_le_bytes());
            run += (pixels.len() + alpha.len()) as u32;
        }
        blob.extend_from_slice(leading);
        for (_, (pixels, alpha)) in frames {
            blob.extend_from_slice(pixels);
            blob.extend_from_slice(alpha);
        }
        blob.extend_from_slice(extra);
        blob
    }

    /// Two 4x3 frames: frame 0 paints its full first row (a red-green
    /// run), frame 1 paints a trimmed row — the ring shape.
    fn two_frame_body() -> Vec<u8> {
        let (p0, a0) = oja_plane(
            &[oja_row_group(4, 0, 0, &[0x7C00, 0x03E0, 0x001F, 0x7C01])],
            &[0x40, 0x80, 0xFF, 0x40],
        );
        let (p1, a1) = oja_plane(&[oja_row_group(2, 1, 1, &[0x03E0, 0x001F])], &[0xFF, 0xFF]);
        oja55_body(
            &[],
            &[],
            &[((0, 0, 4, 3), (p0, a0)), ((2, 1, 4, 3), (p1, a1))],
        )
    }

    #[test]
    fn oja55_walks_planes_and_slices_alpha() {
        let body = two_frame_body();
        let oja = parse_oja55(&body).unwrap();
        assert_eq!(oja.tag, 1);
        assert_eq!(oja.reserved, 0);
        assert_eq!(oja.trailing, b"");
        assert_eq!(oja.frames.len(), 2);
        let frame = &oja.frames[0];
        assert_eq!((frame.x, frame.y, frame.w, frame.h), (0, 0, 4, 3));
        assert_eq!((frame.a, frame.b, frame.c), (0, 16, 4));
        assert_eq!(frame.rows.len(), 1);
        assert_eq!((frame.rows[0].x, frame.rows[0].y), (0, 0));
        assert_eq!(
            frame.rows[0].pixels,
            &[0x00, 0x7C, 0xE0, 0x03, 0x1F, 0x00, 0x01, 0x7C]
        );
        assert_eq!(frame.alpha, &[0x40, 0x80, 0xFF, 0x40]);
        let frame = &oja.frames[1];
        assert_eq!((frame.x, frame.y, frame.w, frame.h), (2, 1, 4, 3));
        assert_eq!((frame.a, frame.b, frame.c), (20, 12, 2));
        assert_eq!((frame.rows[0].x, frame.rows[0].y), (1, 1));
        assert_eq!(frame.alpha, &[0xFF, 0xFF]);
    }

    /// `Playing_Effect_Jam.oja` opens its blob with an 80-byte block
    /// of serialized record copies before the first plane; the base
    /// scan steps over it in record strides and decodes every frame.
    /// (Synthetic: the real 7.55 bodies all chain at shift 0 — the
    /// scan guards prefixed layouts regardless.)
    #[test]
    fn oja55_scans_the_plane_base_past_a_leading_block() {
        let (p0, a0) = oja_plane(&[oja_row_group(4, 0, 0, &[0x7C00; 4])], &[0x40]);
        let (p1, a1) = oja_plane(&[oja_row_group(2, 1, 1, &[0x03E0; 2])], &[0x80]);
        let leading = [0xA5u8; 80];
        let body = oja55_body(
            &leading,
            &[],
            &[((0, 0, 4, 3), (p0, a0)), ((2, 1, 4, 3), (p1, a1))],
        );
        let oja = parse_oja55(&body).unwrap();
        assert_eq!(oja.frames.len(), 2);
        assert_eq!((oja.frames[0].a, oja.frames[0].b), (0, 16));
        assert_eq!(oja.frames[0].rows[0].pixels, &[0x00, 0x7C].repeat(4));
        assert_eq!((oja.frames[1].a, oja.frames[1].b), (17, 12));
        assert_eq!(oja.trailing, b"");
    }

    /// Untalphabet trailing bytes that hold no bottom-anchored plane
    /// walk ride along untouched as raw bytes (no invented frames).
    #[test]
    fn oja55_keeps_untabulated_trailing_bytes() {
        let (p0, a0) = oja_plane(&[oja_row_group(4, 0, 0, &[0x7C00; 4])], &[0x40]);
        let body = oja55_body(&[], &[0x99; 10], &[((0, 0, 4, 3), (p0, a0))]);
        let oja = parse_oja55(&body).unwrap();
        assert_eq!(oja.frames.len(), 1);
        assert!(oja.trailing_frames.is_empty());
        assert_eq!(oja.trailing, &[0x99; 10]);
    }

    /// Recordless trailing frames decode: after the tabulated frame,
    /// a leading unaccounted block, then `plane + mask` pairs that
    /// fill the block to its end. Each trailing plane must end on
    /// the tabulated canvas's bottom row (here row 2 of 4x3); the
    /// masks are opaque slices between the planes.
    #[test]
    fn oja55_decodes_recordless_trailing_frames() {
        let (p0, a0) = oja_plane(&[oja_row_group(4, 0, 0, &[0x7C00; 4])], &[0x40]);
        let (t0_p, m0) = oja_plane(&[oja_row_group(4, 0, 2, &[0x03E0; 4])], &[0xAA, 0xBB]);
        let (t1_p, m1) = oja_plane(
            &[
                oja_row_group(2, 0, 1, &[0x001F; 2]),
                oja_row_group(4, 0, 2, &[0x7C01; 4]),
            ],
            &[0xCC],
        );
        let mut extra = vec![0x11, 0x22];
        extra.extend_from_slice(&t0_p);
        extra.extend_from_slice(&m0);
        extra.extend_from_slice(&t1_p);
        extra.extend_from_slice(&m1);
        let body = oja55_body(&[], &extra, &[((0, 0, 4, 3), (p0, a0))]);
        let oja = parse_oja55(&body).unwrap();
        assert_eq!(oja.frames.len(), 1);
        assert_eq!(oja.trailing, &[0x11, 0x22], "leading block stays raw");
        assert_eq!(oja.trailing_frames.len(), 2);
        let frame = &oja.trailing_frames[0];
        assert_eq!((frame.w, frame.h), (4, 3));
        assert_eq!(frame.rows.len(), 1);
        assert_eq!((frame.rows[0].x, frame.rows[0].y), (0, 2));
        assert_eq!(frame.alpha, &[0xAA, 0xBB]);
        let frame = &oja.trailing_frames[1];
        assert_eq!(frame.rows.len(), 2);
        assert_eq!((frame.rows[0].x, frame.rows[0].y), (0, 1));
        assert_eq!((frame.rows[1].x, frame.rows[1].y), (0, 2));
        assert_eq!(frame.alpha, &[0xCC]);
    }

    /// Mask bytes that happen to contain an early-terminated walk do
    /// not split a trailing frame: the bottom-row rule rejects them
    /// and the real plane still decodes from its true start.
    #[test]
    fn oja55_skips_mask_walks_that_stop_above_the_bottom_row() {
        let (p0, a0) = oja_plane(&[oja_row_group(4, 0, 0, &[0x7C00; 4])], &[0x40]);
        let mut leading = oja_row_group(2, 0, 0, &[0x7C01; 2]);
        leading.extend_from_slice(&0u16.to_le_bytes());
        leading.extend_from_slice(&[0x55; 6]);
        let (t0_p, m0) = oja_plane(&[oja_row_group(4, 0, 2, &[0x03E0; 4])], &[0xAA]);
        let mut extra = leading.clone();
        extra.extend_from_slice(&t0_p);
        extra.extend_from_slice(&m0);
        let body = oja55_body(&[], &extra, &[((0, 0, 4, 3), (p0, a0))]);
        let oja = parse_oja55(&body).unwrap();
        assert_eq!(oja.trailing_frames.len(), 1);
        assert_eq!(oja.trailing, leading);
        assert_eq!(oja.trailing_frames[0].rows[0].y, 2);
        assert_eq!(oja.trailing_frames[0].alpha, &[0xAA]);
    }

    /// A trailing block whose only walks stop above the bottom row is
    /// not a plane: the whole block stays raw (honest negative).
    #[test]
    fn oja55_leaves_bottom_free_trailing_blocks_raw() {
        let (p0, a0) = oja_plane(&[oja_row_group(4, 0, 0, &[0x7C00; 4])], &[0x40]);
        let mut high = oja_row_group(4, 0, 1, &[0x03E0; 4]);
        high.extend_from_slice(&0u16.to_le_bytes());
        let body = oja55_body(&[], &high, &[((0, 0, 4, 3), (p0, a0))]);
        let oja = parse_oja55(&body).unwrap();
        assert!(oja.trailing_frames.is_empty());
        assert_eq!(oja.trailing, high);
    }

    #[test]
    fn oja55_rejects_broken_layouts() {
        // Broken chain: frame 1's `a` must equal a0 + b0 + c0.
        let (p0, a0) = oja_plane(&[oja_row_group(4, 0, 0, &[0x7C00; 4])], &[0x40]);
        let (p1, a1) = oja_plane(&[oja_row_group(2, 1, 1, &[0x03E0; 2])], &[0x80]);
        let mut body = oja55_body(
            &[],
            &[],
            &[
                ((0, 0, 4, 3), (p0.clone(), a0.clone())),
                ((2, 1, 4, 3), (p1, a1)),
            ],
        );
        let a1_off = OJA_HEADER_LEN + OJA_RECORD_LEN + 8;
        body[a1_off..a1_off + 4].copy_from_slice(&21u32.to_le_bytes());
        assert!(parse_oja55(&body).is_err());

        // Misplaced plane length: shrinking `b` by the terminator
        // leaves the walk without its zero word at `a + b`.
        let body = oja55_body(&[], &[], &[((0, 0, 4, 3), (p0.clone(), a0.clone()))]);
        let b_off = OJA_HEADER_LEN + 12;
        let mut short = body.clone();
        let b = u32::from_le_bytes(short[b_off..b_off + 4].try_into().unwrap()) - 2;
        short[b_off..b_off + 4].copy_from_slice(&b.to_le_bytes());
        assert!(parse_oja55(&short).is_err());

        // A backward row step is not a plane walk.
        let (backward, alpha) = oja_plane(
            &[
                oja_row_group(2, 0, 1, &[0x7C00; 2]),
                oja_row_group(2, 0, 0, &[0x03E0; 2]),
            ],
            &[0x40],
        );
        let body = oja55_body(&[], &[], &[((0, 0, 4, 3), (backward, alpha))]);
        assert!(parse_oja55(&body).is_err());

        // A span past the row edge breaks the containment.
        let (wide, alpha) = oja_plane(&[oja_row_group(4, 1, 0, &[0x7C00; 4])], &[0x40]);
        let body = oja55_body(&[], &[], &[((0, 0, 4, 3), (wide, alpha))]);
        assert!(parse_oja55(&body).is_err());

        // An alpha length overrunning the blob rejects every base:
        // bump the record's `c` past the bytes the builder stored.
        let mut body = oja55_body(&[], &[], &[((0, 0, 4, 3), (p0, vec![0x40; 64]))]);
        let c_off = OJA_HEADER_LEN + 16;
        let c = u32::from_le_bytes(body[c_off..c_off + 4].try_into().unwrap()) + 1;
        body[c_off..c_off + 4].copy_from_slice(&c.to_le_bytes());
        assert!(parse_oja55(&body).is_err());

        // Container-level guards: bad magic and a short header.
        let mut magic = two_frame_body();
        magic[2..4].copy_from_slice(&0x0555u16.to_le_bytes());
        assert!(parse_oja55(&magic).is_err());
        assert!(matches!(parse_oja55(b"short"), Err(FileError::TooShort(_))));
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

    /// Archive body bytes by exact name.
    fn archive_body(archive: &[u8], name: &str) -> Vec<u8> {
        let entries = parse_opi(archive).expect("archive table parses");
        let entry = entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("{name} in archive"));
        archive[entry.data_offset..entry.data_offset + entry.size as usize].to_vec()
    }

    /// The tap and long-note effect envelopes (codec gap ⑤, real
    /// 7.55 bytes): six shrinking ring frames with centred anchors,
    /// pixel planes ending in a terminator exactly at `a + b`, and
    /// alpha planes sliced at `c`. `LongNote_Effect.oja` is
    /// byte-identical to `Note_Click.oja`. The covered-pixel count
    /// runs 1-3 bytes over `c` (the packer's alpha pass dropped edge
    /// pixels) — pinned here per frame, not assumed away. Skips
    /// when `install-root` is absent.
    #[test]
    fn oja_click_bodies_decode_from_the_archive() {
        let Some(archive) = install_archive("Playing1.opi") else {
            return;
        };
        let click = archive_body(&archive, "Note_Click.oja");
        let long = archive_body(&archive, "LongNote_Effect.oja");
        assert_eq!(click, long, "the two shipped click bodies are identical");
        let oja = parse_oja55(&click).unwrap();
        assert_eq!(oja.tag, 1);
        assert_eq!(oja.reserved, 0);
        assert_eq!(oja.trailing, b"");
        let dims = [
            (144, 143),
            (113, 113),
            (97, 96),
            (77, 77),
            (59, 59),
            (39, 39),
        ];
        let anchors = [
            (-72, -72),
            (-56, -57),
            (-48, -49),
            (-38, -39),
            (-29, -30),
            (-19, -20),
        ];
        let first_spans = [
            (125, 0, 2),
            (94, 0, 2),
            (77, 0, 2),
            (57, 0, 3),
            (39, 0, 2),
            (19, 0, 1),
        ];
        let shortfalls = [1usize, 2, 1, 3, 2, 0];
        assert_eq!(oja.frames.len(), 6);
        for (i, frame) in oja.frames.iter().enumerate() {
            assert_eq!((frame.w, frame.h), dims[i], "frame {i} dims");
            assert_eq!((frame.x, frame.y), anchors[i], "frame {i} anchor");
            let span = &frame.rows[0];
            assert_eq!(
                (span.x, span.y, span.pixels.len() / 2),
                first_spans[i],
                "frame {i} first span"
            );
            let covered: usize = frame.rows.iter().map(|row| row.pixels.len() / 2).sum();
            assert_eq!(
                covered - frame.alpha.len(),
                shortfalls[i],
                "frame {i} alpha shortfall"
            );
            assert_eq!(frame.alpha.len(), frame.c as usize, "frame {i} alpha len");
        }
    }

    /// The jam effect envelope (codec gap ⑤, real 7.55 bytes): ten
    /// 300x300 flash frames at anchor (0, 0), every frame's alpha
    /// plane covering its pixels exactly (`c == covered`), then a
    /// 1,623,039-byte recordless trailing block: a 9,159-byte
    /// alpha-profile orphan, then six more frames — one rows-25..299
    /// partial top and five full canvases — each a bottom-anchored
    /// row-group plane plus a canvas-row mask, pinned per frame.
    /// Skips when `install-root` is absent.
    #[test]
    fn oja_jam_body_decodes_from_the_archive() {
        let Some(archive) = install_archive("Playing1.opi") else {
            return;
        };
        let body = archive_body(&archive, "Playing_Effect_Jam.oja");
        let oja = parse_oja55(&body).unwrap();
        assert_eq!(oja.frames.len(), 10);
        assert_eq!(oja.trailing_frames.len(), 6);
        assert_eq!(oja.trailing.len(), 9_159);
        assert_eq!(&oja.trailing[..3], &[0x20, 0x19, 0x13]);
        let spans = [136, 135, 157, 210, 439, 774, 1095, 1231, 1218, 1204];
        let first_spans = [
            (154, 24, 1),
            (154, 12, 2),
            (137, 1, 12),
            (103, 1, 103),
            (54, 1, 25),
            (5, 1, 202),
            (6, 1, 233),
            (5, 1, 74),
            (5, 1, 73),
            (4, 1, 75),
        ];
        for (i, frame) in oja.frames.iter().enumerate() {
            assert_eq!((frame.w, frame.h), (300, 300), "frame {i} dims");
            assert_eq!((frame.x, frame.y), (0, 0), "frame {i} anchor");
            assert_eq!(frame.rows.len(), spans[i], "frame {i} span count");
            let span = &frame.rows[0];
            assert_eq!(
                (span.x, span.y, span.pixels.len() / 2),
                first_spans[i],
                "frame {i} first span"
            );
            let covered: usize = frame.rows.iter().map(|row| row.pixels.len() / 2).sum();
            assert_eq!(
                covered,
                usize::try_from(frame.c).unwrap(),
                "frame {i} alpha exact"
            );
            assert_eq!(frame.alpha.len(), usize::try_from(frame.c).unwrap());
        }
        // The recordless frames (no records): span count, first span,
        // covered pixels, and mask bytes per frame. Every plane ends
        // on the canvas bottom row; every mask is a canvas-row map
        // (299 rows of 300 bytes; the first frame's is six short).
        let trailing = [
            (281, (0, 25), 82_494, 89_694),
            (300, (0, 0), 90_000, 89_700),
            (300, (0, 0), 90_000, 89_700),
            (300, (0, 0), 90_000, 89_700),
            (300, (0, 0), 90_000, 89_700),
            (300, (0, 0), 90_000, 89_700),
        ];
        for (i, frame) in oja.trailing_frames.iter().enumerate() {
            assert_eq!((frame.w, frame.h), (300, 300), "trailing frame {i} dims");
            assert_eq!(frame.rows.len(), trailing[i].0, "trailing frame {i} spans");
            let span = &frame.rows[0];
            assert_eq!(
                (span.x, span.y, span.pixels.len() / 2),
                (trailing[i].1 .0, trailing[i].1 .1, 300),
                "trailing frame {i} first span"
            );
            let last = frame.rows.last().expect("rows");
            assert_eq!((last.x, last.y, last.pixels.len() / 2), (0, 299, 300));
            let covered: usize = frame.rows.iter().map(|row| row.pixels.len() / 2).sum();
            assert_eq!(covered, trailing[i].2, "trailing frame {i} covered");
            assert_eq!(frame.alpha.len(), trailing[i].3, "trailing frame {i} mask");
        }
        // The first two masks have emptied their bottom canvas rows;
        // later frames' flash core still glows there.
        assert!(
            oja.trailing_frames[0].alpha[..].ends_with(&[0u8; 300]),
            "first mask tail is zero"
        );
        assert!(
            oja.trailing_frames[1].alpha[..].ends_with(&[0u8; 300]),
            "second mask tail is zero"
        );
    }
}
