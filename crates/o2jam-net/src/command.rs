//! Command ids (Memoryer `Commands.cs`; exe sends the C->S ones).
//!
//! Odd/even pairing: request `N` C->S, response S->C `N+1`,
//! events S->C broadcasts. Only Phase-1 login/channel opcodes
//! live here; later phases extend these enums.

/// Client-to-server requests (Phase 1 subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Request {
    Authorize = 1000,
    GetChannelList = 1002,
    ChannelLogin = 1004,
    SessionKeys = 1007,
    Terminate = 65520,
}

impl Request {
    /// Wire value on the frame.
    #[must_use]
    pub fn id(self) -> u16 {
        self as u16
    }

    /// Resolve a received id. `None` means not a Phase-1 request.
    #[must_use]
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            1000 => Some(Self::Authorize),
            1002 => Some(Self::GetChannelList),
            1004 => Some(Self::ChannelLogin),
            1007 => Some(Self::SessionKeys),
            65520 => Some(Self::Terminate),
            _ => None,
        }
    }
}

/// Keepalive echo pair (same id both directions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Generic {
    LegacyPing = 6000,
    Ping = 6001,
}

impl Generic {
    /// Wire value on the frame.
    #[must_use]
    pub fn id(self) -> u16 {
        self as u16
    }

    /// Resolve a received id. `None` means not a keepalive.
    #[must_use]
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            6000 => Some(Self::LegacyPing),
            6001 => Some(Self::Ping),
            _ => None,
        }
    }
}

/// Server-to-client responses (Phase 1 subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Response {
    Authorize = 1001,
    GetChannelList = 1003,
    ChannelLogin = 1005,
    SessionKeys = 1008,
}

impl Response {
    /// Wire value on the frame.
    #[must_use]
    pub fn id(self) -> u16 {
        self as u16
    }

    /// Resolve a received id. `None` means not a Phase-1 response.
    #[must_use]
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            1001 => Some(Self::Authorize),
            1003 => Some(Self::GetChannelList),
            1005 => Some(Self::ChannelLogin),
            1008 => Some(Self::SessionKeys),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_request_ids() {
        assert_eq!(Request::from_id(1000), Some(Request::Authorize));
        assert_eq!(Request::from_id(1002), Some(Request::GetChannelList));
        assert_eq!(Request::from_id(1004), Some(Request::ChannelLogin));
        assert_eq!(Request::from_id(1007), Some(Request::SessionKeys));
        assert_eq!(Request::from_id(65520), Some(Request::Terminate));
    }

    #[test]
    fn rejects_unknown_request_id() {
        assert_eq!(Request::from_id(4014), None);
    }

    #[test]
    fn resolves_keepalive_ids() {
        assert_eq!(Generic::from_id(6000), Some(Generic::LegacyPing));
        assert_eq!(Generic::from_id(6001), Some(Generic::Ping));
        assert_eq!(Generic::from_id(1000), None);
    }

    #[test]
    fn resolves_known_response_ids() {
        assert_eq!(Response::from_id(1003), Some(Response::GetChannelList));
        assert_eq!(Response::from_id(1008), Some(Response::SessionKeys));
        assert_eq!(Response::from_id(1002), None);
    }
}
