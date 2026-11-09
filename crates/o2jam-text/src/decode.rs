//! CP949 byte-to-text decoding for 7.55 client strings.
//!
//! The exe consumes stored text as CP949 (K claim C10,
//! `docs/claims.md`): one converter at VA `0x406360` calls
//! `MultiByteToWideChar` with codepage `0x3b5` (949) at VA
//! `0x40639a`. Our-side strings are UTF-8 (claim D4 in the same
//! file), so this module is the single boundary where raw client
//! bytes become text. Mirrors `o2jam-file::decode_control_text` so
//! both paths accept exactly the same repertoire; neither invents
//! a mapping beyond the WHATWG `euc-kr` index.

/// Why a CP949 byte string could not decode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The byte string holds a sequence the CP949 index rejects
    /// (for example an unpaired lead byte): `sample` is the head
    /// of the offending input, for diagnostics only.
    #[error("malformed CP949 byte sequence (head: {sample:?})")]
    Malformed {
        /// First bytes of the rejected input.
        sample: Vec<u8>,
    },
}

/// Decode CP949 bytes to UTF-8 text, strictly.
///
/// # Errors
///
/// Returns [`DecodeError::Malformed`] when the input holds a byte
/// sequence outside the CP949 repertoire, instead of guessing.
pub fn decode_cp949(bytes: &[u8]) -> Result<String, DecodeError> {
    let (decoded, _, had_errors) = encoding_rs::EUC_KR.decode(bytes);
    if had_errors {
        return Err(malformed(bytes));
    }
    Ok(decoded.into_owned())
}

/// Decode CP949 bytes to UTF-8 text, lossily.
///
/// Unmappable bytes become `U+FFFD`, so this never fails. Prefer
/// [`decode_cp949`] when a silently replaced glyph would hide a
/// bad file.
#[must_use]
pub fn decode_cp949_lossy(bytes: &[u8]) -> String {
    encoding_rs::EUC_KR.decode(bytes).0.into_owned()
}

/// Build the [`DecodeError::Malformed`] diagnostic for `bytes`.
fn malformed(bytes: &[u8]) -> DecodeError {
    const SAMPLE_LEN: usize = 4;
    DecodeError::Malformed {
        sample: bytes[..SAMPLE_LEN.min(bytes.len())].to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn korean_syllables_decode_as_cp949() {
        // U+D55C U+AE00 (한글): high bytes UTF-8 cannot parse.
        let korean = [0xC7, 0xD1, 0xB1, 0xDB];
        assert_eq!(decode_cp949(&korean).unwrap(), "한글");
        assert!(String::from_utf8(korean.to_vec()).is_err());
    }

    #[test]
    fn ascii_runs_mixed_with_korean_decode_verbatim() {
        let mut blob = b"Lv.43 ".to_vec();
        blob.extend_from_slice(&[0xC7, 0xD1]);
        assert_eq!(decode_cp949(&blob).unwrap(), "Lv.43 한");
    }

    #[test]
    fn empty_input_decodes_to_empty_text() {
        assert_eq!(decode_cp949(&[]).unwrap(), "");
        assert_eq!(decode_cp949_lossy(&[]), "");
    }

    #[test]
    fn unpaired_lead_byte_is_an_error_not_lossy_text() {
        assert_eq!(
            decode_cp949(&[0x61, 0x81]),
            Err(DecodeError::Malformed {
                sample: vec![0x61, 0x81],
            })
        );
    }

    #[test]
    fn lossy_decode_replaces_unmappable_bytes() {
        let decoded = decode_cp949_lossy(&[0x61, 0x81]);
        assert!(decoded.starts_with('a'));
        assert!(decoded.contains('\u{FFFD}'));
    }

    #[test]
    fn malformed_sample_truncates_to_four_bytes() {
        let error = decode_cp949(&[0x81, 0x81, 0x81, 0x81, 0x81]).unwrap_err();
        assert_eq!(
            error,
            DecodeError::Malformed {
                sample: vec![0x81, 0x81, 0x81, 0x81],
            }
        );
        assert_eq!(
            error.to_string(),
            "malformed CP949 byte sequence (head: [129, 129, 129, 129])"
        );
    }
}
