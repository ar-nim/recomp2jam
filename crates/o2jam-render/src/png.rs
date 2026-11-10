//! Minimal std-only PNG writer (none filter, stored deflate).
//!
//! Frame-capture evidence (the windowed `--frame-dump` affordance) and
//! the scene screenshot walkers share this one encoder: no external
//! image dependency, deterministic bytes, RGB8 output.

/// PNG signature.
const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Encode 8-bit RGB pixels (width*height*3 bytes) as a PNG.
#[must_use]
pub fn encode_png(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let stride = width as usize * 3;
    let mut raw = Vec::with_capacity(height as usize * (stride + 1));
    for row in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[row * stride..(row + 1) * stride]);
    }
    let mut out = Vec::with_capacity(raw.len() + 64);
    out.extend_from_slice(&PNG_SIG);
    let mut ihdr = [0u8; 13];
    ihdr[0..4].copy_from_slice(&width.to_be_bytes());
    ihdr[4..8].copy_from_slice(&height.to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = 2;
    push_chunk(&mut out, b"IHDR", &ihdr);
    push_chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    push_chunk(&mut out, b"IEND", &[]);
    out
}

/// Wrap raw scanlines in a zlib stream of uncompressed deflate blocks.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    let mut out = Vec::with_capacity(raw.len() + 2 + 5 * blocks.len() + 4);
    out.extend_from_slice(&[0x78, 0x01]);
    for (index, block) in blocks.iter().enumerate() {
        let last = index + 1 == blocks.len();
        out.push(u8::from(last));
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// Append one chunk (length, tag, data, CRC over tag + data).
fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let mut tagged = Vec::with_capacity(4 + data.len());
    tagged.extend_from_slice(tag);
    tagged.extend_from_slice(data);
    out.extend_from_slice(&crc32(&tagged).to_be_bytes());
}

/// IEEE CRC-32, table-free.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Adler-32 checksum.
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut low = 1u32;
    let mut high = 0u32;
    for &byte in data {
        low = (low + u32::from(byte)) % MOD;
        high = (high + low) % MOD;
    }
    (high << 16) | low
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_carries_signature_and_dimensions() {
        let rgb = [0x11, 0x22, 0x33];
        let png = encode_png(1, 1, &rgb);
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        // IHDR length + tag follow the signature; dimensions sit first.
        assert_eq!(&png[8..12], &13u32.to_be_bytes());
        assert_eq!(&png[12..16], &b"IHDR"[..]);
        assert_eq!(&png[16..20], &1u32.to_be_bytes());
        assert_eq!(&png[20..24], &1u32.to_be_bytes());
        // 8-bit RGB color type.
        assert_eq!(png[24], 8);
        assert_eq!(png[25], 2);
    }

    #[test]
    fn png_stored_blocks_round_trip() {
        let rgb: Vec<u8> = (0..18).map(|i| (i % 7) as u8).collect();
        let png = encode_png(3, 2, &rgb);
        // zlib header + one final stored block + adler trailer.
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        let idat = png
            .windows(4)
            .position(|window| window == b"IDAT")
            .expect("IDAT chunk present");
        let data_len =
            u32::from_be_bytes([png[idat - 4], png[idat - 3], png[idat - 2], png[idat - 1]])
                as usize;
        let stored = &png[idat + 4..idat + 4 + data_len];
        assert_eq!(&stored[..2], &[0x78, 0x01]);
        assert_eq!(stored[2], 1, "one final stored block");
        let len = u16::from_le_bytes([stored[3], stored[4]]) as usize;
        assert_eq!(len, 2 * (3 * 3 + 1), "two filtered scanlines");
    }

    #[test]
    fn adler32_matches_the_reference_value() {
        // "Wikipedia" is the classic worked example.
        let raw = b"Wikipedia";
        assert_eq!(adler32(raw), 0x11E6_0398);
    }
}
