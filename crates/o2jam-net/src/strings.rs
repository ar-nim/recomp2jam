//! Null-terminated UTF-8 string fields (`docs/net-protocol.md` §3b).
//!
//! Reference: Encore's `StringMessageField` defaults (open-source
//! `SirusDoma/Encore` framework): UTF-8 bytes, one `0x00` terminator,
//! no length prefix, at most 65535 bytes. Encoding truncates to the
//! cap *including* the terminator; decoding consumes through the
//! terminator and reports the bytes taken so the next field starts
//! at the right offset.

use super::error::Error;

/// Wire cap of a default string field (no `maxLength` override).
pub const MAX_STRING_LEN: usize = u16::MAX as usize;
/// Terminator byte capping every encoded string.
const TERMINATOR: u8 = 0x00;

/// Encode `value` as UTF-8 plus terminator, capped at `max_len`
/// bytes including the terminator (mirrors Encore byte truncation).
#[must_use]
pub fn encode_null_terminated(value: &str, max_len: usize) -> Vec<u8> {
    let capacity = max_len.saturating_sub(1);
    let mut bytes = value.as_bytes();
    if bytes.len() > capacity {
        bytes = &bytes[..capacity];
    }
    let mut out = Vec::with_capacity(bytes.len() + 1);
    out.extend_from_slice(bytes);
    out.push(TERMINATOR);
    out
}

/// Decode one string from the front of `body`: bytes up to the first
/// terminator (UTF-8), plus the terminator's own byte in `consumed`.
///
/// An empty `body` decodes to empty (mirrors Encore); a body with no
/// terminator inside `max_len` is malformed, not truncated.
pub fn decode_null_terminated(body: &[u8], max_len: usize) -> Result<(String, usize), Error> {
    if body.is_empty() {
        return Ok((String::new(), 0));
    }
    let scan = body.len().min(max_len);
    match body[..scan].iter().position(|byte| *byte == TERMINATOR) {
        Some(end) => {
            let text = std::str::from_utf8(&body[..end]).map_err(|_| Error::InvalidField {
                field: "string field",
                reason: "not valid UTF-8",
            })?;
            Ok((text.to_owned(), end + 1))
        }
        None => Err(Error::InvalidField {
            field: "string field",
            reason: "missing null terminator",
        }),
    }
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
    fn truncates_to_cap_including_terminator() {
        assert_eq!(
            encode_null_terminated("abcdef", 4),
            vec![0x61, 0x62, 0x63, 0x00]
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
    fn rejects_body_without_terminator() {
        assert_eq!(
            decode_null_terminated(&[0x61, 0x62, 0x63], MAX_STRING_LEN),
            Err(Error::InvalidField {
                field: "string field",
                reason: "missing null terminator"
            })
        );
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
