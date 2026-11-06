//! Length-framed `CPacket_Str` messages.
//!
//! Wire layout: `[u16 len][u16 id][body...]`, all little-endian.
//! `len` counts the bytes after the length field (id + body),
//! matching the in-repo UDP relay precedent (`innerSize` style).
//! The exe pins id-at-offset-2 (`FUN_005a0550`); the length
//! semantics follow Memoryer convention and need a live-server
//! check before we call framing VERIFIED.

use super::error::Error;

/// Byte offset of the command id inside a frame (exe-verified).
pub const ID_OFFSET: usize = 2;
/// Header size: length field + command id.
pub const HEADER_LEN: usize = 4;

/// One framed message: command id plus raw payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Command id (see [`crate::command`]).
    pub id: u16,
    /// Bytes after the 4-byte header.
    pub payload: Vec<u8>,
}

/// Serialize id + body into a length-prefixed frame.
#[must_use]
pub fn encode_frame(id: u16, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    let len = (ID_OFFSET + body.len()) as u16;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Parse exactly one frame; rejects trailing bytes.
pub fn decode_frame(buffer: &[u8]) -> Result<Frame, Error> {
    let (frame, consumed) = split_frame(buffer)?;
    if consumed != buffer.len() {
        return Err(Error::TrailingBytes(buffer.len() - consumed));
    }
    Ok(frame)
}

/// Parse the first frame, reporting bytes consumed so a caller can
/// advance over concatenated frames.
fn split_frame(buffer: &[u8]) -> Result<(Frame, usize), Error> {
    if buffer.len() < HEADER_LEN {
        return Err(Error::TruncatedHeader {
            need: HEADER_LEN,
            have: buffer.len(),
        });
    }
    let len = u16::from_le_bytes([buffer[0], buffer[1]]) as usize;
    if len < ID_OFFSET {
        return Err(Error::MissingCommand(len));
    }
    let total = ID_OFFSET + len;
    if buffer.len() < total {
        return Err(Error::TruncatedBody {
            need: total,
            have: buffer.len(),
        });
    }
    let id = u16::from_le_bytes([buffer[ID_OFFSET], buffer[ID_OFFSET + 1]]);
    Ok((
        Frame {
            id,
            payload: buffer[HEADER_LEN..total].to_vec(),
        },
        total,
    ))
}

/// Incremental TCP splitter: feed arbitrary chunks, drain frames.
#[derive(Debug, Default)]
pub struct FrameSplitter {
    buffer: Vec<u8>,
}

impl FrameSplitter {
    /// Buffer another TCP chunk, returning each completed frame.
    ///
    /// Errors on a malformed length and drops the buffer, since
    /// resync is impossible without more protocol knowledge.
    pub fn push_bytes(&mut self, chunk: &[u8]) -> Result<Vec<Frame>, Error> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();
        loop {
            if self.buffer.len() < HEADER_LEN {
                return Ok(frames);
            }
            let len = u16::from_le_bytes([self.buffer[0], self.buffer[1]]) as usize;
            if len < ID_OFFSET {
                self.buffer.clear();
                return Err(Error::MissingCommand(len));
            }
            let total = ID_OFFSET + len;
            if self.buffer.len() < total {
                return Ok(frames);
            }
            let (frame, _) = split_frame(&self.buffer)?;
            self.buffer.drain(..total);
            frames.push(frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_id_at_offset_two_little_endian() {
        let bytes = encode_frame(1004, &[0x01, 0x00]);
        assert_eq!(bytes, vec![0x04, 0x00, 0xEC, 0x03, 0x01, 0x00]);
    }

    #[test]
    fn round_trips_empty_body() {
        let bytes = encode_frame(1002, &[]);
        assert_eq!(bytes, vec![0x02, 0x00, 0xEA, 0x03]);
        assert_eq!(
            decode_frame(&bytes),
            Ok(Frame {
                id: 1002,
                payload: vec![]
            })
        );
    }

    #[test]
    fn rejects_truncated_header() {
        assert_eq!(
            decode_frame(&[0x04, 0x00, 0xEC]),
            Err(Error::TruncatedHeader { need: 4, have: 3 })
        );
    }

    #[test]
    fn rejects_truncated_body() {
        assert_eq!(
            decode_frame(&[0x04, 0x00, 0xEC, 0x03, 0x01]),
            Err(Error::TruncatedBody { need: 6, have: 5 })
        );
    }

    #[test]
    fn rejects_length_without_command() {
        assert_eq!(
            decode_frame(&[0x01, 0x00, 0x00, 0x00]),
            Err(Error::MissingCommand(1))
        );
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = encode_frame(1002, &[]);
        bytes.push(0xFF);
        assert_eq!(decode_frame(&bytes), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn splitter_holds_partial_frame() {
        let mut splitter = FrameSplitter::default();
        let bytes = encode_frame(1004, &[0x01, 0x00, 0x02, 0x00]);
        assert_eq!(splitter.push_bytes(&bytes[..3]), Ok(vec![]));
        let frames = splitter.push_bytes(&bytes[3..]).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].id, 1004);
    }

    #[test]
    fn splitter_yields_multiple_frames_in_one_feed() {
        let mut splitter = FrameSplitter::default();
        let mut bytes = encode_frame(1002, &[]);
        bytes.extend_from_slice(&encode_frame(6001, &[]));
        let frames = splitter.push_bytes(&bytes).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].id, 1002);
        assert_eq!(frames[1].id, 6001);
    }
}
