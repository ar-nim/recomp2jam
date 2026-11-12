//! Cursor over a message body; every read bounds-checks.
//!
//! One shared reader backs all response decoders; it replaces the
//! near-verbatim private copies that `character` and `rooms` each
//! carried. Error shapes (`TruncatedMessage`, `InvalidField`,
//! `TrailingBytes`) are part of the pinned wire contract.

use super::error::Error;
use super::rooms::ROOM_TITLE_LEN;
use super::strings::{decode_null_terminated, MAX_STRING_LEN};

/// Cursor over a response body; every read bounds-checks.
pub(crate) struct Reader<'body> {
    body: &'body [u8],
    pos: usize,
}

impl<'body> Reader<'body> {
    /// Cursor at the start of `body`.
    pub(crate) fn new(body: &'body [u8]) -> Self {
        Self { body, pos: 0 }
    }

    /// Next `count` bytes, advancing the cursor.
    pub(crate) fn take(&mut self, count: usize) -> Result<&'body [u8], Error> {
        if self.body.len() - self.pos < count {
            return Err(Error::TruncatedMessage {
                need: self.pos + count,
                have: self.body.len(),
            });
        }
        let bytes = &self.body[self.pos..self.pos + count];
        self.pos += count;
        Ok(bytes)
    }

    pub(crate) fn read_i16(&mut self) -> Result<i16, Error> {
        let bytes = self.take(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16, Error> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32, Error> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32, Error> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// One `0`/`1` byte as bool.
    pub(crate) fn read_flag(&mut self, field: &'static str) -> Result<bool, Error> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidField {
                field,
                reason: "not 0 or 1",
            }),
        }
    }

    /// One `0`/`1` `i32` word as bool.
    pub(crate) fn read_int_flag(&mut self, field: &'static str) -> Result<bool, Error> {
        match self.read_i32()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidField {
                field,
                reason: "not 0 or 1",
            }),
        }
    }

    /// One `0`/`1` `i16` word as bool.
    pub(crate) fn read_short_flag(&mut self, field: &'static str) -> Result<bool, Error> {
        match self.read_i16()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidField {
                field,
                reason: "not 0 or 1",
            }),
        }
    }

    /// Null-terminated string under the default wire cap.
    pub(crate) fn read_string(&mut self) -> Result<String, Error> {
        self.read_capped_string(MAX_STRING_LEN)
    }

    /// Null-terminated room title under its `maxLength: 21` cap.
    pub(crate) fn read_title(&mut self) -> Result<String, Error> {
        self.read_capped_string(ROOM_TITLE_LEN)
    }

    /// Element count prefix; negative counts are malformed.
    pub(crate) fn read_count(&mut self, field: &'static str, wide: bool) -> Result<usize, Error> {
        let count = if wide {
            self.read_i32()?
        } else {
            i32::from(self.read_i16()?)
        };
        if count < 0 {
            return Err(Error::InvalidField {
                field,
                reason: "negative count",
            });
        }
        Ok(count as usize)
    }

    /// No bytes may remain after the last field.
    pub(crate) fn finish(&self) -> Result<(), Error> {
        if self.pos != self.body.len() {
            return Err(Error::TrailingBytes(self.body.len() - self.pos));
        }
        Ok(())
    }

    /// Bytes not yet consumed.
    pub(crate) fn remaining(&self) -> usize {
        self.body.len() - self.pos
    }

    /// Null-terminated string under `max_len`, advancing past the
    /// terminator (or the exact cap when the field fills it).
    fn read_capped_string(&mut self, max_len: usize) -> Result<String, Error> {
        let (text, used) = decode_null_terminated(&self.body[self.pos..], max_len)?;
        self.pos += used;
        Ok(text)
    }
}

#[cfg(test)]
mod reader_tests {
    use super::*;

    fn reader_of(bytes: &[u8]) -> Reader<'_> {
        Reader::new(bytes)
    }

    #[test]
    fn reads_little_endian_scalars_in_order() {
        let mut reader = reader_of(&[
            0x01, 0x34, 0x12, 0x78, 0x56, 0x78, 0x56, 0x34, 0x12, 0x78, 0x56, 0x34, 0x12,
        ]);
        assert_eq!(reader.read_u8().unwrap(), 0x01);
        assert_eq!(reader.read_u16().unwrap(), 0x1234);
        assert_eq!(reader.read_i16().unwrap(), 0x5678);
        assert_eq!(reader.read_u32().unwrap(), 0x12345678);
        assert_eq!(reader.read_i32().unwrap(), 0x12345678);
        reader.finish().unwrap();
    }

    #[test]
    fn truncated_take_reports_need_and_have() {
        let mut reader = reader_of(&[0x01]);
        assert_eq!(
            reader.read_i32(),
            Err(Error::TruncatedMessage { need: 4, have: 1 })
        );
    }

    #[test]
    fn flag_rejects_bytes_other_than_zero_or_one() {
        let mut reader = reader_of(&[0x02]);
        assert_eq!(
            reader.read_flag("test flag"),
            Err(Error::InvalidField {
                field: "test flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn int_flag_rejects_words_other_than_zero_or_one() {
        let word = 2i32.to_le_bytes();
        let mut reader = reader_of(&word);
        assert_eq!(
            reader.read_int_flag("test int flag"),
            Err(Error::InvalidField {
                field: "test int flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn short_flag_reads_zero_and_one_words() {
        let words = [0i16.to_le_bytes(), 1i16.to_le_bytes()].concat();
        let mut reader = reader_of(&words);
        assert!(!reader.read_short_flag("first").unwrap());
        assert!(reader.read_short_flag("second").unwrap());
        reader.finish().unwrap();
    }

    #[test]
    fn short_flag_rejects_words_other_than_zero_or_one() {
        let word = 2i16.to_le_bytes();
        let mut reader = reader_of(&word);
        assert_eq!(
            reader.read_short_flag("test short flag"),
            Err(Error::InvalidField {
                field: "test short flag",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn count_rejects_negative_prefix() {
        let mut reader = reader_of(&[0xFF, 0xFF]);
        assert_eq!(
            reader.read_count("test count", false),
            Err(Error::InvalidField {
                field: "test count",
                reason: "negative count"
            })
        );
    }

    #[test]
    fn finish_rejects_trailing_bytes() {
        let reader = reader_of(&[0x01, 0x02]);
        assert_eq!(reader.finish(), Err(Error::TrailingBytes(2)));
    }

    #[test]
    fn title_uses_room_title_cap() {
        // 21 content bytes with no terminator: fills the cap exactly.
        let mut reader = reader_of(&[0x41; ROOM_TITLE_LEN]);
        assert_eq!(reader.read_title().unwrap(), "A".repeat(ROOM_TITLE_LEN));
        reader.finish().unwrap();
    }
}
