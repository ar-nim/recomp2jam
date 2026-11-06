//! Transport errors: truncated input, unknown ids, bad lengths, TCP faults.

use std::io;

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
    /// The socket failed or closed mid-exchange.
    #[error("transport: {0}")]
    Transport(String),
    /// The peer closed the connection mid-frame.
    #[error("connection closed by peer")]
    ConnectionClosed,
    /// A non-keepalive frame arrived while waiting for another command.
    #[error("unexpected response: want {expected:#06x}, got {found:#06x}")]
    UnexpectedResponse { expected: u16, found: u16 },
}

impl Error {
    /// Wrap a socket fault without leaking `io::Error` (not `Eq`).
    // Takes ownership instead of a reference so `map_err` call sites
    // stay point-free; nothing is cloned.
    #[allow(clippy::needless_pass_by_value, reason = "consumed for formatting")]
    #[must_use]
    pub fn transport(source: io::Error) -> Self {
        Self::Transport(source.to_string())
    }
}
