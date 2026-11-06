//! Null-terminated UTF-8 string fields (`docs/net-protocol.md` §3b).
//!
//! Reference: Encore's `StringMessageField` codec, verified live
//! against the real implementation (probe `dotnet run` over
//! `DefaultMessageCodec`): content chars fill up to `max_len`
//! bytes on whole-char boundaries, then a `0x00` terminator iff
//! room remains; over-long content is truncated (never errors).
//! No length prefix. Default cap 65535 (`u16::MAX`).

use super::error::Error;

/// Wire cap of a default string field (no `maxLength` override).
pub const MAX_STRING_LEN: usize = u16::MAX as usize;
/// Terminator byte capping every encoded string.
const TERMINATOR: u8 = 0x00;

/// Encode `value` as UTF-8 plus terminator, capped at `max_len`
/// bytes: whole chars while they fit, then the terminator iff
/// room remains (mirrors Encore byte-for-byte, probe-verified).
#[must_use]
pub fn encode_null_terminated(value: &str, max_len: usize) -> Vec<u8> {
    if max_len == 0 {
        return Vec::new();
    }
    let capacity = max_len - 1;
    let mut out = Vec::new();
    for ch in value.chars() {
        let ch_len = ch.len_utf8();
        if out.len() + ch_len > capacity {
            break;
        }
        let mut buf = [0u8; 4];
        out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
    }
    out.push(TERMINATOR);
    out
}

/// Decode one string from the front of `body`: bytes up to the first
/// terminator (UTF-8), plus the terminator's own byte in `consumed`.
/// No terminator within `max_len` means the field exactly fills its
/// cap (probe-verified: 22-char titles carry no null) — consume
/// `max_len` bytes as the string instead of erroring.
///
/// An empty `body` decodes to empty; invalid UTF-8 is an error.
pub fn decode_null_terminated(body: &[u8], max_len: usize) -> Result<(String, usize), Error> {
    if body.is_empty() {
        return Ok((String::new(), 0));
    }
    let scan = body.len().min(max_len);
    let end = body[..scan]
        .iter()
        .position(|byte| *byte == TERMINATOR)
        .unwrap_or(scan);
    let consumed = if end < scan { end + 1 } else { end };
    let text = std::str::from_utf8(&body[..end]).map_err(|_| Error::InvalidField {
        field: "string field",
        reason: "not valid UTF-8",
    })?;
    Ok((text.to_owned(), consumed))
}

#[cfg(test)]
mod string_codec_tests {
    use super::*;

    #[test]
    fn encodes_ascii_with_terminator() {
        assert_eq!(
            encode_null_terminated("abc", MAX_STRING_LEN),
            vec![0x61, 0x62, 0x63, 0x00]
        );
    }

    #[test]
    fn encodes_multibyte_unchanged() {
        assert_eq!(
            encode_null_terminated("é", MAX_STRING_LEN),
            vec![0xC3, 0xA9, 0x00]
        );
    }

    #[test]
    fn truncates_content_including_terminator() {
        // Encore vector: maxLength counts the terminator.
        assert_eq!(
            encode_null_terminated("abcdef", 4),
            vec![0x61, 0x62, 0x63, 0x00]
        );
    }

    #[test]
    fn truncates_long_content_to_cap() {
        let title: String = "A".repeat(40);
        let mut expected = vec![0x41; 21];
        expected.push(0x00);
        assert_eq!(encode_null_terminated(&title, 22), expected);
    }

    #[test]
    fn truncates_multibyte_on_char_boundaries() {
        assert_eq!(
            encode_null_terminated("\u{e9}\u{e9}\u{e9}", 5),
            vec![0xC3, 0xA9, 0xC3, 0xA9, 0x00]
        );
    }

    #[test]
    fn decodes_through_terminator_and_reports_consumed() {
        let (text, consumed) =
            decode_null_terminated(&[0x61, 0x62, 0x00, 0x7A], MAX_STRING_LEN).unwrap();
        assert_eq!(text, "ab");
        assert_eq!(consumed, 3);
    }

    #[test]
    fn decodes_empty_body_as_empty() {
        assert_eq!(
            decode_null_terminated(&[], MAX_STRING_LEN),
            Ok((String::new(), 0))
        );
    }

    #[test]
    fn decodes_cap_filling_body_without_terminator() {
        let body = vec![0x41; 22];
        let (text, consumed) = decode_null_terminated(&body, 22).unwrap();
        assert_eq!(text, "A".repeat(22));
        assert_eq!(consumed, 22);
    }

    #[test]
    fn rejects_non_utf8_bytes() {
        assert_eq!(
            decode_null_terminated(&[0xFF, 0x00], MAX_STRING_LEN),
            Err(Error::InvalidField {
                field: "string field",
                reason: "not valid UTF-8"
            })
        );
    }

    #[test]
    fn round_trips_client_id_shape() {
        let id = "726PNA-B1YW-28ZG9M-I5SC-Q0OPM6";
        let bytes = encode_null_terminated(id, MAX_STRING_LEN);
        let (text, consumed) = decode_null_terminated(&bytes, MAX_STRING_LEN).unwrap();
        assert_eq!(text, id);
        assert_eq!(consumed, bytes.len());
    }
}
