//! OJM music containers: M30/OMC inline entries, the counts+offsets
//! new variant, Vorbis recovery, and crypto/stripe helpers.
//!
//! Verified on real 7.55 files (docs/file-formats/ojm.md).

fn check_ogg(data: &[u8], at: usize) -> Result<(), FileError> {
    match data.get(at..at + 4) {
        Some(b"OggS") => Ok(()),
        _ => Err(FileError::UnknownFormat {
            magic: data.get(at..at + 4).unwrap_or_default().to_vec(),
        }),
    }
}

use super::{read_name, read_u16le, read_u32le, FileError};

/// One key-sound / BGM stream inside an `.ojm` container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmSample<'a> {
    /// Sample name (NUL-trimmed ASCII).
    pub name: String,
    /// Sample id (`None` for variants that do not carry one, e.g. OMC).
    pub sample_id: Option<u32>,
    /// Raw OGG Vorbis bytes.
    pub data: &'a [u8],
}

/// M30 sample-id offset: `ref` word selecting the chart strike slot
/// (`docs/file-formats/ojm.md`); the high word is reserved zero.
/// The song-bank loader reads it as `[entry+0x2C]` (file `0x173625`)
/// to register each sample at its own strike slot.
const M30_REF_OFF: usize = 44;

// Loader anchor: the registered id word is the byte-44 `ref`.
const _: () = assert!(M30_REF_OFF == 44);

/// Parse the M30 variant: 20-byte header, then inline 52-byte entries
/// (`name[32] + size + codec + flag + ref + reserved + pcm_bytes`)
/// each followed by its OGG. Field split corroborated by the open2jam
/// OJM documentation (`M30_sample_header`: `ref`/`unk_zero` words).
/// Verified byte-exact on real 7.55 files (docs/file-formats/ojm.md).
///
/// # Errors
///
/// `FileError::TooShort`/`Truncated` when the buffer ends early,
/// `FileError::UnknownFormat` on a bad header or non-OGG body.
pub fn parse_m30(bytes: &[u8]) -> Result<Vec<OjmSample<'_>>, FileError> {
    if bytes.len() < 20 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"M30") {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let count = read_u32le(bytes, 12)? as usize;
    let mut off = read_u32le(bytes, 16)? as usize;
    let mut out = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let name = read_name(bytes, off)?;
        let size = read_u32le(bytes, off + 32)? as usize;
        let sample_id = u32::from(read_u16le(bytes, off + M30_REF_OFF)?);
        if size == 0 {
            // Degenerate entry: no stream follows, and accepting it
            // would stall the walk (end == start).
            return Err(FileError::Truncated {
                need: off + 52 + 1,
                have: bytes.len().min(off + 52),
            });
        }
        let start = off + 52;
        let end = start.checked_add(size).ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
        if end > bytes.len() {
            return Err(FileError::Truncated {
                need: end,
                have: bytes.len(),
            });
        }
        check_ogg(bytes, start)?;
        out.push(OjmSample {
            name,
            sample_id: Some(sample_id),
            data: &bytes[start..end],
        });
        off = end;
    }
    if off != bytes.len() {
        return Err(FileError::Truncated {
            need: off,
            have: bytes.len(),
        });
    }
    Ok(out)
}

/// New-variant OMC/OJM table layout, verified against the in-exe
/// loader (`FUN_0054e930` reads counts at +4/+6 and seeks the offsets
/// at +8/+12; `FUN_0054eca0` reads `0x38`-byte mode-2 and `0x24`-byte
/// mode-3 entries) and on real 7.55 files (docs/file-formats/ojm.md).
/// Sample bytes follow each entry directly (`[entry, data]` chaining).
pub const MODE2_ENTRY_LEN: usize = 0x38;

pub const MODE3_ENTRY_LEN: usize = 0x24;

pub const MODE2_COUNT_OFF: usize = 4;

pub const MODE3_COUNT_OFF: usize = 6;

pub const MODE2_ENTRIES_OFF: usize = 8;

pub const MODE3_ENTRIES_OFF: usize = 12;

pub const MODE2_SIZE_OFF: usize = 52;
pub const MODE3_SIZE_OFF: usize = 32;

/// Mode-2 audio-format offset (`u16`; 1 = PCM on all shipped files).
pub const MODE2_FORMAT_OFF: usize = 32;

/// Mode-2 channel-count offset (`u16`; 1 mono, 2 stereo).
pub const MODE2_CHANNELS_OFF: usize = 34;

/// Mode-2 sample-rate offset (`u32` Hz).
pub const MODE2_RATE_OFF: usize = 36;

/// Mode-2 byte-rate offset (`u32`; `rate * channels * 2`).
pub const MODE2_BITRATE_OFF: usize = 40;

/// Mode-2 frame-alignment offset (`u16`; 2 mono, 4 stereo).
pub const MODE2_ALIGN_OFF: usize = 44;

/// Mode-2 bits-per-sample offset (`u16`; 16 on all shipped files).
pub const MODE2_BITS_OFF: usize = 46;

/// Byte offsets of the WAV audio params in a mode-2 entry
/// (open2jam `OMC_WAV_header` field order).
#[derive(Debug, Clone, Copy)]
struct WavParamOffs {
    format: usize,
    channels: usize,
    rate: usize,
    bit_rate: usize,
    align: usize,
    bits: usize,
}

/// Offsets of the WAV audio params for mode-2 entries.
const MODE2_WAV_OFFS: WavParamOffs = WavParamOffs {
    format: MODE2_FORMAT_OFF,
    channels: MODE2_CHANNELS_OFF,
    rate: MODE2_RATE_OFF,
    bit_rate: MODE2_BITRATE_OFF,
    align: MODE2_ALIGN_OFF,
    bits: MODE2_BITS_OFF,
};

/// One entry of a new-variant OMC/OJM table: name, declared sample
/// size, and file offset of the (possibly still encrypted) bytes.
/// Mode-2 entries carry the WAV audio params at +32..+52 (field map
/// corroborated by the open2jam OJM documentation
/// (`OMC_WAV_header`); see `docs/file-formats/ojm.md`). Mode-3
/// entries carry none, so every audio field reads 0 there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmEntry {
    /// Sample name (NUL-trimmed ASCII).
    pub name: String,
    /// Declared sample size in bytes (chunk size at entry +52).
    pub size: u32,
    /// File offset where the sample bytes start.
    pub data_offset: usize,
    /// Audio format (`u16` at entry +32; 1 = PCM on all shipped files).
    pub audio_format: u16,
    /// Channel count (`u16` at entry +34; 1 mono, 2 stereo).
    pub num_channels: u16,
    /// Audio sample rate in Hz (`u32` at entry +36). Verified: 44100
    /// on mono Tutorial key sounds, 22050 on stereo pads, 32000 on
    /// brass, 44100 on DialogBGM/bgEffect/Planet stereo bodies.
    pub sample_rate: u32,
    /// Byte rate (`u32` at entry +40; `rate * channels * 2` on every
    /// shipped entry, e.g. 176400 on stereo 44100 Hz bodies).
    pub bit_rate: u32,
    /// Frame alignment in bytes (`u16` at entry +44; 2 mono, 4 stereo).
    pub block_align: u16,
    /// Bits per sample (`u16` at entry +46; 16 on all shipped files).
    pub bits_per_sample: u16,
}

/// Both entry tables of a new-variant file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjmTables {
    /// Mode-2 entries (`0x38` bytes each, 32-byte names).
    pub mode2: Vec<OjmEntry>,
    /// Mode-3 entries (`0x24` bytes each, old 36-byte shape).
    pub mode3: Vec<OjmEntry>,
}

fn walk_entries(
    bytes: &[u8],
    mut off: usize,
    count: usize,
    entry_len: usize,
    size_off: usize,
    audio_off: Option<WavParamOffs>,
) -> Result<Vec<OjmEntry>, FileError> {
    let mut out = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let end = off.checked_add(entry_len).ok_or(FileError::Truncated {
            need: usize::MAX,
            have: bytes.len(),
        })?;
        if end > bytes.len() {
            return Err(FileError::Truncated {
                need: end,
                have: bytes.len(),
            });
        }
        let name = read_name(bytes, off)?;
        let size = read_u32le(bytes, off + size_off)?;
        let (audio_format, num_channels, sample_rate, bit_rate, block_align, bits_per_sample) =
            match audio_off {
                Some(offs) => (
                    read_u16le(bytes, off + offs.format)?,
                    read_u16le(bytes, off + offs.channels)?,
                    read_u32le(bytes, off + offs.rate)?,
                    read_u32le(bytes, off + offs.bit_rate)?,
                    read_u16le(bytes, off + offs.align)?,
                    read_u16le(bytes, off + offs.bits)?,
                ),
                None => (0, 0, 0, 0, 0, 0),
            };
        let data_offset = end;
        let next = data_offset
            .checked_add(size as usize)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
        if next > bytes.len() {
            return Err(FileError::Truncated {
                need: next,
                have: bytes.len(),
            });
        }
        out.push(OjmEntry {
            name,
            size,
            data_offset,
            audio_format,
            num_channels,
            sample_rate,
            bit_rate,
            block_align,
            bits_per_sample,
        });
        off = next;
    }
    Ok(out)
}

/// Parse the entry tables of a new-variant OMC/OJM file: counts at
/// +4/+6, table offsets at +8/+12, `[entry, data]` chaining. Sample
/// bytes are NOT validated (bodies may be encrypted); use
/// [`not_decrypt`] plus de-interleaving to recover them.
/// Verified on real 7.55 files (docs/file-formats/ojm.md).
///
/// # Errors
///
/// `FileError::TooShort`/`Truncated` when tables run past the buffer.
pub fn parse_new_variant_entries(bytes: &[u8]) -> Result<OjmTables, FileError> {
    if bytes.len() < 16 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"OMC") && !bytes.starts_with(b"OJM") {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let mode2_count = read_u16le(bytes, MODE2_COUNT_OFF)? as usize;
    let mode3_count = read_u16le(bytes, MODE3_COUNT_OFF)? as usize;
    let mode2_off = read_u32le(bytes, MODE2_ENTRIES_OFF)? as usize;
    let mode3_off = read_u32le(bytes, MODE3_ENTRIES_OFF)? as usize;
    Ok(OjmTables {
        mode2: walk_entries(
            bytes,
            mode2_off,
            mode2_count,
            MODE2_ENTRY_LEN,
            MODE2_SIZE_OFF,
            Some(MODE2_WAV_OFFS),
        )?,
        mode3: walk_entries(
            bytes,
            mode3_off,
            mode3_count,
            MODE3_ENTRY_LEN,
            MODE3_SIZE_OFF,
            None,
        )?,
    })
}

/// Slot-bank offset the exe adds for bit26-flagged strikes: gameplay
/// (entry at file `0x1703f0`) computes `slot = (value & 0xFFF) - 1`
/// (mask `0x170418`, subtract `0x17041e`), plus `0x3E8` when the
/// payload sets bit 26 (test `0x170427`, add `0x170431`); the
/// song-bank loader (file `0x1734xx`) registers bit31-flagged
/// entries at `entry_id + 0x3E8` the same way (bit31 test
/// `0x17361a`, add `0x17364f`).
/// No shipped M30-paired chart sets bit 26; Tutorial.ojn (old format,
/// new-variant bank) sets it on 12 kind-9 intro notes (ids 0-11).
/// See `docs/file-formats/ojm.md`.
pub const NEW_VARIANT_HIGH_BANK: u32 = 0x3E8;

impl OjmEntry {
    /// Byte range of this entry's (possibly still encrypted) sample
    /// body within the file it was parsed from.
    ///
    /// # Errors
    ///
    /// `FileError::Truncated` when the entry body overruns the buffer.
    pub fn sample_bytes<'b>(&self, bytes: &'b [u8]) -> Result<&'b [u8], FileError> {
        let end = self
            .data_offset
            .checked_add(self.size as usize)
            .ok_or(FileError::Truncated {
                need: usize::MAX,
                have: bytes.len(),
            })?;
        bytes
            .get(self.data_offset..end)
            .ok_or(FileError::Truncated {
                need: end,
                have: bytes.len(),
            })
    }
}

impl OjmTables {
    /// Flat fetch order: mode-2 entries first, then mode-3.
    fn flat_index(&self, order: usize) -> Option<&OjmEntry> {
        self.mode2
            .get(order)
            .or_else(|| self.mode3.get(order.wrapping_sub(self.mode2.len())))
    }

    /// Order-based strike-slot resolution for banks whose entries carry
    /// no sample id (old-variant OMC names/sizes only; new-variant
    /// mode-2/mode-3 entries): slot N names the Nth fetched entry,
    /// mode-2 table first, then mode-3. Slots in the high bank
    /// ([`NEW_VARIANT_HIGH_BANK`] + N, from bit26-flagged strikes)
    /// resolve to the same Nth entry.
    ///
    /// This is our documented reconstruction fallback, NOT exe behavior
    /// (claims C28/C29, sibling S4): the exe keys every bank by
    /// explicit id. Strike time derives the slot from the chart
    /// payload alone (`& 0xFFF` − 1, +`0x3E8` on bit26; ops pinned in
    /// [`NEW_VARIANT_HIGH_BANK`]). Load time registers each fetched
    /// entry at its own id word `[entry+0x2C]` (bit31 of
    /// `[entry+0x28]` adds the same `0x3E8`, usage-gated; loader reads
    /// `0x173625`/`0x17364a`, gate `0x173628`), but the fetcher copies
    /// only the 32 name bytes for new variants (`strncpy` call sites
    /// `0x14ee4a`/`0x14ef21`), so id-less entries all collapse onto
    /// slot 0 (last wins) and only `sample_id` 0 can sound. No order-
    /// or name-based lookup exists on the load/play paths. Returns
    /// `None` past the last entry.
    /// Verified against Tutorial.ojn/ojm: lane ids 0-29 name the 30
    /// mode-2 key sounds, plain kind-9 id 30 names `bgsplit-01`, and the
    /// 12 bit26 intro notes name mode-2 entries 0-11 again.
    #[must_use]
    pub fn resolve_order_slot(&self, slot: u32) -> Option<&OjmEntry> {
        let order = slot.checked_sub(NEW_VARIANT_HIGH_BANK).unwrap_or(slot);
        let order = usize::try_from(order).ok()?;
        self.flat_index(order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> OjmEntry {
        OjmEntry {
            name: name.to_string(),
            size: 8,
            data_offset: 0,
            audio_format: 1,
            num_channels: 1,
            sample_rate: 44100,
            bit_rate: 88200,
            block_align: 2,
            bits_per_sample: 16,
        }
    }

    #[test]
    fn order_slot_fallback_keeps_distinct_slots_exe_collapses_to_zero() {
        // Scenario: exe semantics register every id-less new-variant
        // entry at slot 0 (name-only fetch leaves the id word zero;
        // last wins). Our order-keyed fallback deliberately diverges
        // by naming the Nth fetched entry; this pins the divergence so
        // the fallback cannot be mistaken for exe behavior (C29).
        let tables = OjmTables {
            mode2: vec![entry("apiano1"), entry("apiano2")],
            mode3: vec![],
        };
        assert_ne!(tables.resolve_order_slot(0), tables.resolve_order_slot(1));
        // Plain slots below the high bank never alias down into it.
        assert!(tables
            .resolve_order_slot(NEW_VARIANT_HIGH_BANK - 1)
            .is_none());
    }
}

/// Decoded OJM descriptor: what the in-exe loader derives from the
/// per-song version float (`FUN_0054e7c0`: `major = (int)(v + 0.01)`,
/// `minor = (int)(frac(v + 0.01) * 10)`, descriptor
/// `major << 16 | minor`). The chart carries the float at header +8
/// ([`OjnChartFile::format_version`]; 2.7 on Tutorial.ojn, 2.9 on
/// decrypted o2ma757.ojn); the file magic only matters when the float
/// is exactly 0.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjmDescriptor {
    /// Stripe-lane count for de-interleaving, if the descriptor asks
    /// for any (`17` at `>= 0x20008`, `16` at `>= 0x20006`).
    pub lanes: Option<usize>,
    /// Bitwise-NOT pass (`>= 0x20005`). Mode-1 samples force the
    /// `0xFF`/`0x00` seed; mode-2/3 samples run the zeroed schedule.
    pub not: bool,
}

/// Derive the loader descriptor from a per-song version float.
/// Verified mapping: 2.9 (o2ma songs) selects 17-way + NOT, 2.7
/// (Tutorial) selects 16-way + NOT, 2.4 and below select plain reads
/// (`docs/file-formats/ojm.md`).
#[must_use]
pub fn ojm_descriptor_for_version(version: f32) -> OjmDescriptor {
    let shifted = f64::from(version) + 0.01;
    let major = shifted as u32;
    let minor = ((shifted - f64::from(major)) * 10.0) as u32;
    let descriptor = (major << 16) | minor;
    OjmDescriptor {
        lanes: if descriptor >= 0x20008 {
            Some(17)
        } else if descriptor >= 0x20006 {
            Some(16)
        } else {
            None
        },
        not: descriptor >= 0x20005,
    }
}

/// Decode a mode-2 style encrypted body to raw PCM16 samples: stripe
/// de-interleave, then the zero-seeded [`not_decrypt`] pass the exe
/// runs for mode-2/3 (keys stay zeroed; only the mode-1 branch forces
/// `0xFF`/`0x00`). Rate and channel counts come from the entry header
/// ([`OjmEntry::sample_rate`], [`OjmEntry::num_channels`]); bodies must
/// hold whole little-endian frames.
/// Verified on Tutorial.ojm pads (16-way tail energy matches the
/// plain-PCM O2PlanetNPC.ojm ground truth; 17-way clips at full
/// scale there).
///
/// # Errors
///
/// `FileError::UnknownFormat` on undecodable lane layouts.
pub fn decode_new_variant_pcm(
    data: &[u8],
    descriptor: OjmDescriptor,
) -> Result<Vec<i16>, FileError> {
    let mut plain = match descriptor.lanes {
        Some(lanes) => deinterleave_stripes(data, lanes)?,
        None => data.to_vec(),
    };
    if descriptor.not {
        not_decrypt(&mut plain, 0x00, 0x00);
    }
    if plain.len() % 2 != 0 {
        return Err(FileError::Truncated {
            need: plain.len() + 1,
            have: plain.len(),
        });
    }
    let (frames, _) = plain.as_chunks::<2>();
    Ok(frames
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect())
}

/// In-place bitwise-NOT stream decipher used for encrypted OJM
/// samples (`FUN_0054f9e0`): each byte is inverted when
/// `(key1 << key2) & 0x80` is set; `key2` cycles 0..7 and `key1`
/// tracks the previous ciphertext byte. The loader seeds both from
/// the stream rewind (`0xFF`, `0x00`).
/// Verified against real 7.55 bodies (docs/file-formats/ojm.md).
pub fn not_decrypt(buf: &mut [u8], mut key1: u8, mut key2: u8) {
    for byte in buf.iter_mut() {
        let cipher = *byte;
        if (key1 << key2) & 0x80 != 0 {
            *byte = !*byte;
        }
        if key2 >= 7 {
            key2 = 0;
            key1 = cipher;
        } else {
            key2 += 1;
        }
    }
}

/// De-interleave map, row-selected by `size % 16`.
/// Copied from the game executable (file offsets 0x27f1d0..+512) as
/// functional data required for interop; see docs/file-formats/ojm.md.
const DEINTERLEAVE_16: [[u16; 16]; 16] = [
    [14, 2, 9, 4, 0, 7, 1, 6, 8, 15, 10, 5, 12, 3, 13, 11], // row 0
    [7, 2, 10, 11, 3, 5, 13, 8, 4, 0, 12, 6, 15, 14, 1, 9], // row 1
    [12, 13, 3, 0, 6, 9, 10, 1, 7, 8, 2, 11, 14, 4, 15, 5], // row 2
    [8, 3, 4, 13, 6, 5, 11, 2, 12, 7, 9, 10, 15, 14, 0, 1], // row 3
    [15, 2, 12, 13, 0, 4, 1, 5, 7, 3, 9, 6, 11, 10, 8, 14], // row 4
    [0, 4, 11, 15, 13, 12, 6, 5, 7, 1, 2, 3, 8, 9, 10, 14], // row 5
    [3, 8, 7, 6, 9, 14, 13, 0, 10, 11, 4, 5, 12, 2, 1, 15], // row 6
    [4, 14, 15, 5, 8, 7, 11, 0, 1, 6, 2, 12, 9, 3, 10, 13], // row 7
    [6, 13, 14, 7, 10, 11, 0, 1, 12, 15, 2, 3, 8, 9, 4, 5], // row 8
    [10, 12, 0, 8, 9, 13, 3, 4, 5, 14, 15, 1, 2, 11, 6, 7], // row 9
    [5, 6, 12, 4, 13, 15, 7, 14, 8, 1, 9, 2, 10, 11, 0, 3], // row 10
    [11, 15, 4, 14, 3, 1, 0, 2, 13, 12, 6, 7, 5, 9, 8, 10], // row 11
    [3, 2, 1, 0, 4, 12, 13, 11, 5, 6, 15, 14, 7, 9, 10, 8], // row 12
    [9, 10, 0, 7, 8, 6, 3, 4, 1, 2, 5, 11, 14, 15, 13, 12], // row 13
    [10, 6, 9, 12, 11, 7, 8, 0, 15, 3, 1, 2, 5, 13, 14, 4], // row 14
    [13, 0, 1, 14, 2, 3, 8, 11, 7, 12, 9, 5, 10, 15, 4, 6], // row 15
];

/// De-interleave map, row-selected by `size % 17`.
/// Copied from the game executable (file offsets 0x27f3d0..+578) as
/// functional data required for interop; see docs/file-formats/ojm.md.
const DEINTERLEAVE_17: [[u16; 17]; 17] = [
    [16, 14, 2, 9, 4, 0, 7, 1, 6, 8, 15, 10, 5, 12, 3, 13, 11], // row 0
    [7, 2, 10, 11, 3, 5, 13, 8, 4, 0, 12, 6, 15, 14, 16, 1, 9], // row 1
    [12, 13, 3, 0, 6, 9, 10, 1, 7, 8, 16, 2, 11, 14, 4, 15, 5], // row 2
    [8, 3, 4, 13, 6, 5, 11, 16, 2, 12, 7, 9, 10, 15, 14, 0, 1], // row 3
    [15, 2, 12, 13, 0, 4, 1, 5, 7, 3, 9, 16, 6, 11, 10, 8, 14], // row 4
    [0, 4, 11, 16, 15, 13, 12, 6, 5, 7, 1, 2, 3, 8, 9, 10, 14], // row 5
    [3, 16, 8, 7, 6, 9, 14, 13, 0, 10, 11, 4, 5, 12, 2, 1, 15], // row 6
    [4, 14, 16, 15, 5, 8, 7, 11, 0, 1, 6, 2, 12, 9, 3, 10, 13], // row 7
    [6, 13, 14, 7, 16, 10, 11, 0, 1, 12, 15, 2, 3, 8, 9, 4, 5], // row 8
    [10, 12, 0, 8, 9, 13, 3, 4, 5, 16, 14, 15, 1, 2, 11, 6, 7], // row 9
    [5, 6, 12, 4, 13, 15, 7, 14, 8, 1, 9, 2, 16, 10, 11, 0, 3], // row 10
    [11, 15, 4, 14, 3, 1, 0, 2, 13, 12, 6, 7, 5, 16, 9, 8, 10], // row 11
    [3, 2, 1, 0, 4, 12, 13, 11, 16, 5, 6, 15, 14, 7, 9, 10, 8], // row 12
    [9, 10, 0, 7, 8, 6, 16, 3, 4, 1, 2, 5, 11, 14, 15, 13, 12], // row 13
    [10, 6, 9, 12, 11, 16, 7, 8, 0, 15, 3, 1, 2, 5, 13, 14, 4], // row 14
    [13, 0, 1, 14, 2, 3, 8, 11, 7, 12, 9, 5, 10, 15, 4, 6, 16], // row 15
    [1, 14, 2, 3, 13, 11, 7, 0, 8, 12, 9, 6, 15, 16, 5, 10, 4], // row 16
];

/// Scatter de-interleave for encrypted OJM samples (`FUN_0054f860`
/// 16-way, `FUN_0054f920` 17-way): the file holds `lanes` stripes of
/// `size / lanes` bytes plus a `size % lanes` tail; stripe `i` lands at
/// plain offset `row[size % lanes][i]`. Only the two lane counts the
/// loader implements are accepted.
/// Verified on real 7.55 bodies (docs/file-formats/ojm.md).
///
/// # Errors
///
/// `FileError::UnknownFormat` for unsupported lane counts.
pub fn deinterleave_stripes(data: &[u8], lanes: usize) -> Result<Vec<u8>, FileError> {
    let size = data.len();
    let row: &[u16] = match lanes {
        16 => &DEINTERLEAVE_16[size % 16],
        17 => &DEINTERLEAVE_17[size % 17],
        _ => {
            return Err(FileError::UnknownFormat {
                magic: data[..4.min(size)].to_vec(),
            });
        }
    };
    let quotient = size / lanes;
    let mut out = vec![0u8; size];
    for (stripe, slot) in row.iter().enumerate() {
        let from = stripe * quotient;
        let to = *slot as usize * quotient;
        out[to..to + quotient].copy_from_slice(&data[from..from + quotient]);
    }
    out[lanes * quotient..].copy_from_slice(&data[lanes * quotient..]);
    Ok(out)
}

/// Exact inverse of [`deinterleave_stripes`]: scatter plain stripes
/// back into file order. Encryption-side helper for building
/// synthetic fixtures and round-trip tests; the game never runs this
/// direction (it only decodes shipped bodies).
///
/// # Errors
///
/// `FileError::UnknownFormat` for unsupported lane counts.
pub fn interleave_stripes(data: &[u8], lanes: usize) -> Result<Vec<u8>, FileError> {
    let size = data.len();
    let row: &[u16] = match lanes {
        16 => &DEINTERLEAVE_16[size % 16],
        17 => &DEINTERLEAVE_17[size % 17],
        _ => {
            return Err(FileError::UnknownFormat {
                magic: data[..4.min(size)].to_vec(),
            });
        }
    };
    let quotient = size / lanes;
    let mut out = vec![0u8; size];
    for (stripe, slot) in row.iter().enumerate() {
        let from = *slot as usize * quotient;
        let to = stripe * quotient;
        out[to..to + quotient].copy_from_slice(&data[from..from + quotient]);
    }
    out[lanes * quotient..].copy_from_slice(&data[lanes * quotient..]);
    Ok(out)
}

/// CRC-32 (ISO 3309, polynomial `0x04C11DB7`) used by OGG page
/// checksums. Bitwise and allocation-free; bank loading recovers a
/// few dozen bodies at most.
/// CRC-32 over an OGG page body (test-visible helper).
#[must_use]
pub fn crc32_ogg(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// True when `data` opens with a complete, checksum-valid OGG page:
/// magic, segment table, and body all present, CRC-32 (computed with
/// the stored checksum zeroed) matching the stored checksum. A bare
/// `OggS` prefix is not enough: a wrong-lane de-interleave of an
/// encrypted body can preserve the head bytes while scrambling the
/// page, so candidates need the full check.
fn ogg_first_page_valid(data: &[u8]) -> bool {
    if data.len() < 27 || &data[..4] != b"OggS" {
        return false;
    }
    let segments = data[26] as usize;
    let table_end = 27 + segments;
    if data.len() < table_end {
        return false;
    }
    let body_len: usize = data[27..table_end].iter().map(|&b| b as usize).sum();
    let page_end = table_end + body_len;
    if data.len() < page_end {
        return false;
    }
    let stored = u32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let mut page = data[..page_end].to_vec();
    page[22..26].copy_from_slice(&[0, 0, 0, 0]);
    crc32_ogg(&page) == stored
}

/// Recover one new-variant sample body to plain OGG bytes:
/// de-interleave (17-way, then 16-way) plus the zero-seeded
/// [`not_decrypt`] pass the exe runs for mode-2/3, accepting the
/// first candidate whose opening OGG page checksum-validates.
/// Mode-3 bodies ship raw, so the undecrypted bytes are the last
/// resort. Only bodies that validate as OGG come back here;
/// raw-PCM bodies (Tutorial key sounds) need
/// [`decode_new_variant_pcm`] with the entry audio header instead.
/// Verified on real 7.55 bodies (docs/file-formats/ojm.md).
///
/// # Errors
///
/// `FileError::UnknownFormat` when the body is neither OGG nor PCM.
pub fn recover_new_variant_sample(data: &[u8]) -> Result<Vec<u8>, FileError> {
    if ogg_first_page_valid(data) {
        return Ok(data.to_vec());
    }
    for lanes in [17, 16] {
        let Ok(mut plain) = deinterleave_stripes(data, lanes) else {
            continue;
        };
        not_decrypt(&mut plain, 0x00, 0x00);
        if ogg_first_page_valid(&plain) {
            return Ok(plain);
        }
    }
    Err(FileError::UnknownFormat {
        magic: data[..4.min(data.len())].to_vec(),
    })
}

/// Parse the old-variant OMC/OJM tables: 16-byte header with the
/// first entry offset at +12, then 36-byte entries (`name[32] +
/// size`) each followed by its OGG. Blank (all-zero) entries are
/// skipped. New-variant files (counts at +4/+6, `0x38`-byte
/// entries) need [`parse_new_variant_entries`] instead: their bodies
/// may be encrypted, so no OGG check applies here.
/// Verified byte-exact on real 7.55 files (docs/file-formats/ojm.md).
///
/// # Errors
///
/// `FileError::TooShort`/`Truncated` when the buffer ends early,
/// `FileError::UnknownFormat` on a bad header or non-OGG body.
/// One old-variant table entry read: either a blank (all-zero)
/// padding slot, or one OGG sample plus the offset just past its
/// body.
enum OmcStep<'a> {
    /// Blank slot: no stream follows; entry was padding.
    Blank,
    /// One OGG sample and the offset just past its body.
    Sample { sample: OjmSample<'a>, end: usize },
}

/// Validate the old-variant OMC/OJM header (16-byte floor, `OMC`/`OJM`
/// magic) and return the first entry offset (+12).
fn check_omc_header(bytes: &[u8]) -> Result<usize, FileError> {
    if bytes.len() < 16 {
        return Err(FileError::TooShort(bytes.len()));
    }
    if !bytes.starts_with(b"OMC") && !bytes.starts_with(b"OJM") {
        return Err(FileError::UnknownFormat {
            magic: bytes[..4.min(bytes.len())].to_vec(),
        });
    }
    let off = read_u32le(bytes, 12)? as usize;
    if off < 16 || off > bytes.len() {
        return Err(FileError::Truncated {
            need: off,
            have: bytes.len(),
        });
    }
    Ok(off)
}

/// Read one 36-byte old-variant table entry (`name[32] + size`) and
/// its OGG body. Blank (all-zero) entries skip; a zero size on a named
/// entry is degenerate (the walk would stall: end == start).
fn read_omc_entry<'a>(bytes: &'a [u8], off: usize) -> Result<OmcStep<'a>, FileError> {
    let name = read_name(bytes, off)?;
    let size = read_u32le(bytes, off + 32)? as usize;
    if name.is_empty() && size == 0 {
        return Ok(OmcStep::Blank);
    }
    if size == 0 {
        // Degenerate entry: no stream follows, and accepting it
        // would stall the walk (end == start).
        return Err(FileError::Truncated {
            need: off + 36 + 1,
            have: bytes.len().min(off + 36),
        });
    }
    let start = off + 36;
    let end = start.checked_add(size).ok_or(FileError::Truncated {
        need: usize::MAX,
        have: bytes.len(),
    })?;
    if end > bytes.len() {
        return Err(FileError::Truncated {
            need: end,
            have: bytes.len(),
        });
    }
    check_ogg(bytes, start)?;
    Ok(OmcStep::Sample {
        sample: OjmSample {
            name,
            sample_id: None,
            data: &bytes[start..end],
        },
        end,
    })
}

pub fn parse_omc(bytes: &[u8]) -> Result<Vec<OjmSample<'_>>, FileError> {
    let mut off = check_omc_header(bytes)?;
    let mut out = Vec::new();
    loop {
        if off == bytes.len() {
            // Clean end (also covers fully encrypted bodies with no
            // raw streams: entry offset == file length).
            return Ok(out);
        }
        match read_omc_entry(bytes, off)? {
            OmcStep::Blank => off += 36,
            OmcStep::Sample { sample, end } => {
                out.push(sample);
                off = end;
            }
        }
        if off == bytes.len() {
            return Ok(out);
        }
        if off > bytes.len() {
            return Err(FileError::Truncated {
                need: off,
                have: bytes.len(),
            });
        }
        if out.len() > 100_000 {
            return Err(FileError::UnknownFormat {
                magic: bytes[..4.min(bytes.len())].to_vec(),
            });
        }
        // Next entry header must fit.
        if off + 36 > bytes.len() {
            return Err(FileError::Truncated {
                need: off + 36,
                have: bytes.len(),
            });
        }
    }
}
