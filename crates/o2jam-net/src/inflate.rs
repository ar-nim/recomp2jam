//! Minimal zlib (RFC 1950) + DEFLATE (RFC 1951) decoder.
//!
//! The Live relay UDP path wraps every datagram in zlib: a `u16`
//! uncompressed length followed by the zlib stream (Memoryer.Relay
//! `UdpRelayFrameCodec`). This crate takes no compression dependency, so the
//! decoder here is hand-written: stored, fixed-Huffman, and
//! dynamic-Huffman blocks, with header and Adler-32 checks.
//!
//! Only what the relay needs is supported: no preset dictionaries.
//! Output is capped at 1 MiB per datagram as a zip-bomb guard; real
//! relay frames carry at most 65535 bytes (the length prefix is `u16`).

use super::error::Error;

/// Zip-bomb guard: far above the 65535-byte relay ceiling.
const MAX_OUTPUT_LEN: usize = 1 << 20;
/// DEFLATE sliding window.
const WINDOW_SIZE: usize = 32768;
/// Longest Huffman code (RFC 1951 §3.2.2).
const MAX_BITS: usize = 15;
/// End-of-block symbol.
const END_OF_BLOCK: u16 = 256;

/// Decode one zlib stream into its raw bytes.
///
/// # Errors
///
/// `Error::TruncatedMessage` on short input, `Error::InvalidField`
/// on bad headers, checksums, or malformed DEFLATE data.
pub(crate) fn decompress_zlib(input: &[u8]) -> Result<Vec<u8>, Error> {
    if input.len() < 2 {
        return Err(Error::TruncatedMessage {
            need: 2,
            have: input.len(),
        });
    }
    check_zlib_header(input[0], input[1])?;
    let mut reader = BitReader::new(&input[2..]);
    let mut out = Vec::new();
    loop {
        let is_final = reader.read_bits(1)? != 0;
        match reader.read_bits(2)? {
            0 => decode_stored_block(&mut reader, &mut out)?,
            1 => {
                let (literals, distances) = fixed_tables()?;
                decode_huffman_block(&mut reader, &mut out, &literals, &distances)?;
            }
            2 => {
                let (literals, distances) = decode_dynamic_tables(&mut reader)?;
                decode_huffman_block(&mut reader, &mut out, &literals, &distances)?;
            }
            _ => {
                return Err(Error::InvalidField {
                    field: "deflate block",
                    reason: "reserved block type",
                });
            }
        }
        if is_final {
            break;
        }
    }
    check_checksum(reader.trailer()?, &out)?;
    Ok(out)
}

/// Build a zlib stream with stored (uncompressed) DEFLATE blocks.
///
/// Infallible: any input length splits into `u16`-sized chunks.
pub(crate) fn compress_zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 8);
    // CM = 8 (deflate), CINFO = 7 (32K window); FCHECK makes the
    // header a multiple of 31, with no preset dictionary.
    out.push(0x78);
    out.push(0x01);
    if data.is_empty() {
        out.push(0x01);
        out.extend_from_slice(&[0x00, 0x00, 0xFF, 0xFF]);
    } else {
        let mut done = 0usize;
        for chunk in data.chunks(u16::MAX as usize) {
            done += chunk.len();
            out.push(u8::from(done == data.len()));
            let len = chunk.len() as u16;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&(!len).to_le_bytes());
            out.extend_from_slice(chunk);
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// Adler-32 over raw bytes (zlib trailer order: big-endian).
pub(crate) fn adler32(data: &[u8]) -> u32 {
    const MODULUS: u32 = 65521;
    let mut low: u32 = 1;
    let mut high: u32 = 0;
    for chunk in data.chunks(5552) {
        for &byte in chunk {
            low += u32::from(byte);
            high += low;
        }
        low %= MODULUS;
        high %= MODULUS;
    }
    (high << 16) | low
}

/// Reject anything that is not a plain deflate zlib header.
fn check_zlib_header(first: u8, second: u8) -> Result<(), Error> {
    if first & 0x0F != 8 {
        return Err(Error::InvalidField {
            field: "zlib header",
            reason: "not the deflate method",
        });
    }
    if (u16::from(first) << 8 | u16::from(second)) % 31 != 0 {
        return Err(Error::InvalidField {
            field: "zlib header",
            reason: "bad check bits",
        });
    }
    if second & 0x20 != 0 {
        return Err(Error::InvalidField {
            field: "zlib header",
            reason: "preset dictionary unsupported",
        });
    }
    Ok(())
}

/// Compare the 4 trailer bytes after the final block with Adler-32.
fn check_checksum(trailer: &[u8], decoded: &[u8]) -> Result<(), Error> {
    if trailer.len() != 4 {
        return Err(Error::TruncatedMessage {
            need: 4,
            have: trailer.len(),
        });
    }
    let want = u32::from_be_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    if adler32(decoded) != want {
        return Err(Error::InvalidField {
            field: "zlib checksum",
            reason: "adler32 mismatch",
        });
    }
    Ok(())
}

/// LSB-first reader over the bytes past the zlib header.
struct BitReader<'data> {
    data: &'data [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'data> BitReader<'data> {
    /// Borrow the unread tail starting at `offset`.
    fn new(data: &'data [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Next `count` bits, least-significant first.
    fn read_bits(&mut self, count: u8) -> Result<u32, Error> {
        let mut value = 0u32;
        for shift in 0..count {
            if self.byte_pos >= self.data.len() {
                return Err(Error::TruncatedMessage {
                    need: self.byte_pos + 1,
                    have: self.data.len(),
                });
            }
            let bit = u32::from((self.data[self.byte_pos] >> self.bit_pos) & 1);
            value |= bit << shift;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
        Ok(value)
    }

    /// Next `count` whole bytes (stored blocks are byte-aligned).
    fn read_bytes(&mut self, count: usize) -> Result<&'data [u8], Error> {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        let end = self.byte_pos.saturating_add(count);
        if end > self.data.len() {
            return Err(Error::TruncatedMessage {
                need: end,
                have: self.data.len(),
            });
        }
        let slice = &self.data[self.byte_pos..end];
        self.byte_pos = end;
        Ok(slice)
    }

    /// Bytes after the final block, aligned up (the Adler-32 trailer).
    fn trailer(&mut self) -> Result<&'data [u8], Error> {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        if self.byte_pos > self.data.len() {
            return Err(Error::TruncatedMessage {
                need: self.byte_pos,
                have: self.data.len(),
            });
        }
        Ok(&self.data[self.byte_pos..])
    }
}

/// Copy one stored block; validates `LEN` against `NLEN`.
fn decode_stored_block(reader: &mut BitReader<'_>, out: &mut Vec<u8>) -> Result<(), Error> {
    let raw = reader.read_bytes(4)?;
    let len = u16::from_le_bytes([raw[0], raw[1]]) as usize;
    let complement = u16::from_le_bytes([raw[2], raw[3]]);
    if len as u16 != !complement {
        return Err(Error::InvalidField {
            field: "deflate stored block",
            reason: "length complement mismatch",
        });
    }
    check_output_room(out.len(), len)?;
    out.extend_from_slice(reader.read_bytes(len)?);
    Ok(())
}

/// Canonical Huffman decoder built from per-symbol bit lengths.
struct HuffmanDecoder {
    /// Symbols ordered by (length, symbol value).
    symbols: Vec<u16>,
    /// First code of each bit length (index 1..=15).
    first_code: [u32; MAX_BITS + 1],
    /// Offset into `symbols` per bit length.
    offsets: [usize; MAX_BITS + 1],
    /// Code count per bit length.
    counts: [u32; MAX_BITS + 1],
}

impl HuffmanDecoder {
    /// Next symbol from the bit stream. A prefix past this length's
    /// code range starts a longer code, so only a 15-bit miss is an
    /// error (short reads surface as truncation from `read_bits`).
    fn decode_symbol(&self, reader: &mut BitReader<'_>) -> Result<u16, Error> {
        let mut code = 0u32;
        for len in 1..=MAX_BITS {
            code = (code << 1) | reader.read_bits(1)?;
            let first = self.first_code[len];
            if code >= first && code < first + self.counts[len] {
                let index = self.offsets[len] + (code - first) as usize;
                return Ok(self.symbols[index]);
            }
        }
        Err(Error::InvalidField {
            field: "deflate code",
            reason: "unknown huffman code",
        })
    }
}

/// Build a decoder; rejects over-subscribed (ambiguous) length sets.
fn decoder_from_lengths(lengths: &[u8]) -> Result<HuffmanDecoder, Error> {
    let mut counts = [0u32; MAX_BITS + 1];
    for &len in lengths {
        if usize::from(len) > MAX_BITS {
            return Err(Error::InvalidField {
                field: "deflate code",
                reason: "code longer than 15 bits",
            });
        }
        if len != 0 {
            counts[usize::from(len)] += 1;
        }
    }
    // Kraft sum scaled by 2^15: over 32768 means ambiguous codes.
    let mut kraft = 0u32;
    for (len, &count) in counts.iter().enumerate().skip(1) {
        kraft += count << (MAX_BITS - len);
    }
    if kraft > (1 << MAX_BITS) {
        return Err(Error::InvalidField {
            field: "deflate code",
            reason: "over-subscribed lengths",
        });
    }
    let mut symbols = Vec::with_capacity(lengths.len());
    let mut offsets = [0usize; MAX_BITS + 1];
    let mut first_code = [0u32; MAX_BITS + 1];
    let mut code = 0u32;
    for (len, &count) in counts.iter().enumerate().skip(1) {
        first_code[len] = code;
        code = (code + count) << 1;
        offsets[len] = symbols.len();
        for (symbol, &len_of) in lengths.iter().enumerate() {
            if usize::from(len_of) == len {
                if let Ok(symbol) = u16::try_from(symbol) {
                    symbols.push(symbol);
                }
            }
        }
    }
    Ok(HuffmanDecoder {
        symbols,
        first_code,
        offsets,
        counts,
    })
}

/// Fixed-Huffman literal/length and distance tables (RFC 1951 §3.2.6).
fn fixed_tables() -> Result<(HuffmanDecoder, HuffmanDecoder), Error> {
    let mut literals = [0u8; 288];
    for (symbol, slot) in literals.iter_mut().enumerate() {
        *slot = match symbol {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    let distances = [5u8; 32];
    Ok((
        decoder_from_lengths(&literals)?,
        decoder_from_lengths(&distances)?,
    ))
}

/// Code-length alphabet order (RFC 1951 §3.2.7).
const CLEN_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Read the dynamic literal/length + distance tables past the `HLIT`
/// / `HDIST` / `HCLEN` counts.
fn decode_dynamic_tables(
    reader: &mut BitReader<'_>,
) -> Result<(HuffmanDecoder, HuffmanDecoder), Error> {
    let literal_count = reader.read_bits(5)? + 257;
    let distance_count = reader.read_bits(5)? + 1;
    let clen_count = reader.read_bits(4)? + 4;
    let mut clen_lengths = [0u8; CLEN_ORDER.len()];
    for index in 0..clen_count as usize {
        clen_lengths[CLEN_ORDER[index]] = reader.read_bits(3)? as u8;
    }
    let clen_decoder = decoder_from_lengths(&clen_lengths)?;
    let total = literal_count as usize + distance_count as usize;
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        let symbol = clen_decoder.decode_symbol(reader)?;
        if symbol <= 15 {
            let len = u8::try_from(symbol).map_err(|_| Error::InvalidField {
                field: "deflate code",
                reason: "bad length value",
            })?;
            lengths.push(len);
        } else {
            repeat_length(symbol, &mut lengths, reader, total)?;
        }
    }
    let split = literal_count as usize;
    Ok((
        decoder_from_lengths(&lengths[..split])?,
        decoder_from_lengths(&lengths[split..])?,
    ))
}

/// Repeat the previous length or zeros (codes 16–18).
fn repeat_length(
    code: u16,
    lengths: &mut Vec<u8>,
    reader: &mut BitReader<'_>,
    total: usize,
) -> Result<(), Error> {
    let (value, count) = match code {
        16 => {
            let Some(&previous) = lengths.last() else {
                return Err(Error::InvalidField {
                    field: "deflate code",
                    reason: "repeat with no previous length",
                });
            };
            (previous, reader.read_bits(2)? + 3)
        }
        17 => (0, reader.read_bits(3)? + 3),
        18 => (0, reader.read_bits(7)? + 11),
        _ => {
            return Err(Error::InvalidField {
                field: "deflate code",
                reason: "bad length code",
            });
        }
    };
    if lengths.len() + count as usize > total {
        return Err(Error::InvalidField {
            field: "deflate code",
            reason: "too many lengths",
        });
    }
    lengths.extend(std::iter::repeat_n(value, count as usize));
    Ok(())
}

/// Length base + extra bits per RFC 1951 §3.2.5.
const LENGTH_TABLE: [(u16, u8); 29] = [
    (3, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (7, 0),
    (8, 0),
    (9, 0),
    (10, 0),
    (11, 1),
    (13, 1),
    (15, 1),
    (17, 1),
    (19, 2),
    (23, 2),
    (27, 2),
    (31, 2),
    (35, 3),
    (43, 3),
    (51, 3),
    (59, 3),
    (67, 4),
    (83, 4),
    (99, 4),
    (115, 4),
    (131, 5),
    (163, 5),
    (195, 5),
    (227, 5),
    (258, 0),
];

/// Distance base + extra bits per RFC 1951 §3.2.5.
const DISTANCE_TABLE: [(u16, u8); 30] = [
    (1, 0),
    (2, 0),
    (3, 0),
    (4, 0),
    (5, 1),
    (7, 1),
    (9, 2),
    (13, 2),
    (17, 3),
    (25, 3),
    (33, 4),
    (49, 4),
    (65, 5),
    (97, 5),
    (129, 6),
    (193, 6),
    (257, 7),
    (385, 7),
    (513, 8),
    (769, 8),
    (1025, 9),
    (1537, 9),
    (2049, 10),
    (3073, 10),
    (4097, 11),
    (6145, 11),
    (8193, 12),
    (12289, 12),
    (16385, 13),
    (24577, 13),
];

/// Run literals and matches until the end-of-block symbol.
fn decode_huffman_block(
    reader: &mut BitReader<'_>,
    out: &mut Vec<u8>,
    literals: &HuffmanDecoder,
    distances: &HuffmanDecoder,
) -> Result<(), Error> {
    loop {
        let symbol = literals.decode_symbol(reader)?;
        if symbol == END_OF_BLOCK {
            return Ok(());
        }
        if let Ok(byte) = u8::try_from(symbol) {
            check_output_room(out.len(), 1)?;
            out.push(byte);
        } else {
            copy_match(symbol, reader, out, distances)?;
        }
    }
}

/// Copy one length/distance match through the 32K window.
fn copy_match(
    symbol: u16,
    reader: &mut BitReader<'_>,
    out: &mut Vec<u8>,
    distances: &HuffmanDecoder,
) -> Result<(), Error> {
    let entry = LENGTH_TABLE
        .get(usize::from(symbol) - 257)
        .ok_or(Error::InvalidField {
            field: "deflate length",
            reason: "symbol out of range",
        })?;
    let length = usize::from(entry.0) + reader.read_bits(entry.1)? as usize;
    let distance_symbol = distances.decode_symbol(reader)?;
    let entry = DISTANCE_TABLE
        .get(usize::from(distance_symbol))
        .ok_or(Error::InvalidField {
            field: "deflate distance",
            reason: "symbol out of range",
        })?;
    let distance = usize::from(entry.0) + reader.read_bits(entry.1)? as usize;
    if distance == 0 || distance > out.len() || distance > WINDOW_SIZE {
        return Err(Error::InvalidField {
            field: "deflate distance",
            reason: "beyond output start",
        });
    }
    check_output_room(out.len(), length)?;
    let start = out.len() - distance;
    for index in 0..length {
        let byte = out[start + index];
        out.push(byte);
    }
    Ok(())
}

/// Refuse output past the zip-bomb cap before appending.
fn check_output_room(current_len: usize, extra: usize) -> Result<(), Error> {
    if current_len.saturating_add(extra) > MAX_OUTPUT_LEN {
        return Err(Error::InvalidField {
            field: "deflate output",
            reason: "exceeds size cap",
        });
    }
    Ok(())
}

#[cfg(test)]
mod stored_block_tests {
    use super::*;

    #[test]
    fn adler32_matches_reference_value() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn stored_encoder_emits_byte_aligned_blocks() {
        let encoded = compress_zlib_stored(b"ABC");
        assert_eq!(
            encoded,
            vec![
                0x78, 0x01, 0x01, 0x03, 0x00, 0xFC, 0xFF, 0x41, 0x42, 0x43, 0x01, 0x8D, 0x00, 0xC7,
            ]
        );
        assert_eq!(decompress_zlib(&encoded), Ok(b"ABC".to_vec()));
    }

    #[test]
    fn stored_encoder_handles_empty_input() {
        let encoded = compress_zlib_stored(b"");
        assert_eq!(decompress_zlib(&encoded), Ok(Vec::new()));
    }

    #[test]
    fn stored_decoder_rejects_complement_mismatch() {
        let mut encoded = compress_zlib_stored(b"ABC");
        encoded[5] ^= 0xFF;
        assert!(matches!(
            decompress_zlib(&encoded),
            Err(Error::InvalidField { .. })
        ));
    }

    #[test]
    fn decoder_rejects_bad_header_check_bits() {
        assert!(matches!(
            decompress_zlib(&[0x78, 0x02]),
            Err(Error::InvalidField { .. })
        ));
    }

    #[test]
    fn decoder_rejects_checksum_mismatch() {
        let mut encoded = compress_zlib_stored(b"ABC");
        let last = encoded.len() - 1;
        encoded[last] ^= 0xFF;
        assert_eq!(
            decompress_zlib(&encoded),
            Err(Error::InvalidField {
                field: "zlib checksum",
                reason: "adler32 mismatch"
            })
        );
    }

    #[test]
    fn decoder_rejects_truncated_stream() {
        assert_eq!(
            decompress_zlib(&[0x78]),
            Err(Error::TruncatedMessage { need: 2, have: 1 })
        );
    }
}
