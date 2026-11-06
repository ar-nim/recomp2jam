//! Framing errors: truncated input, unknown ids, bad lengths.

use thiserror::Error;

/// What can go wrong splitting frames or decoding messages.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Error {
    /// Fewer bytes than the frame header (`HEADER_LEN`).
    #[error("frame truncated: need {need} header bytes, have {have}")]
    TruncatedHeader { need: usize, have: usize },
    /// Length field promises more bytes than buffered.
    #[error("frame truncated: length says {need} bytes, have {have}")]
    TruncatedBody { need: usize, have: usize },
    /// Declared length cannot hold a command id.
    #[error("frame too short for a command id: length {0}")]
    MissingCommand(usize),
    /// Command id has no Phase-1 meaning yet.
    #[error("unknown command id {0:#06x}")]
    UnknownCommand(u16),
    /// Payload is longer than the message layout allows.
    #[error("trailing bytes after message body: {0} left")]
    TrailingBytes(usize),
    /// Payload is shorter than the message layout requires.
    #[error("message truncated: need {need} bytes, have {have}")]
    TruncatedMessage { need: usize, have: usize },
    /// A field value is outside its valid range.
    #[error("invalid {field}: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
}
