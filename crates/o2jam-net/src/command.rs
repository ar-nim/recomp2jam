//! Command ids (Memoryer `Commands.cs`; exe sends the C->S ones).
//!
//! Odd/even pairing: request `N` C->S, response S->C `N+1`,
//! events S->C broadcasts. Only Phase-1 login/channel opcodes
//! live here; later phases extend these enums.

/// Client-to-server requests (Phase 1 login/channel plus the
/// Phase 3/4 host song-flow requests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Request {
    Authorize = 1000,
    GetChannelList = 1002,
    ChannelLogin = 1004,
    SessionKeys = 1007,
    GetCharacterInfo = 2000,
    GetChannelInfo = 2002,
    CreateRoom = 2004,
    SendMainRoomMessage = 2012,
    SendWhisper = 2017,
    JoinWaiting = 3002,
    SetRoomMusic = 4000,
    SetRoomArena = 4002,
    StartGame = 4010,
    ConfirmMusicLoaded = 4012,
    UpdateGameStats = 4014,
    SubmitScore = 4016,
    ExitPlaying = 4021,
    GetMusicList = 4030,
    SyncMemberMusicState = 4036,
    SetDownloadProgress = 4038,
    FinalizeRank = 4041,
    Terminate = 65520,
}

impl Request {
    /// Wire value on the frame.
    #[must_use]
    pub fn id(self) -> u16 {
        self as u16
    }

    /// Resolve a received id. `None` means not a known request.
    #[must_use]
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            1000 => Some(Self::Authorize),
            1002 => Some(Self::GetChannelList),
            1004 => Some(Self::ChannelLogin),
            1007 => Some(Self::SessionKeys),
            2000 => Some(Self::GetCharacterInfo),
            2002 => Some(Self::GetChannelInfo),
            2004 => Some(Self::CreateRoom),
            2012 => Some(Self::SendMainRoomMessage),
            2017 => Some(Self::SendWhisper),
            3002 => Some(Self::JoinWaiting),
            4000 => Some(Self::SetRoomMusic),
            4002 => Some(Self::SetRoomArena),
            4010 => Some(Self::StartGame),
            4012 => Some(Self::ConfirmMusicLoaded),
            4014 => Some(Self::UpdateGameStats),
            4016 => Some(Self::SubmitScore),
            4021 => Some(Self::ExitPlaying),
            4030 => Some(Self::GetMusicList),
            4036 => Some(Self::SyncMemberMusicState),
            4038 => Some(Self::SetDownloadProgress),
            4041 => Some(Self::FinalizeRank),
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
    SubscriptionAlert = 50,
    ForceReauthorize = 51,
    MultiSessionError = 52,
    Authorize = 1001,
    GetChannelList = 1003,
    ChannelLogin = 1005,
    SessionKeys = 1008,
    GetCharacterInfo = 2001,
    RoomList = 2003,
    CreateRoom = 2006,
    MainRoomUserMessage = 2013,
    MainRoomAdminMessage = 2014,
    SendWhisper = 2018,
    JoinRoom = 3003,
}

impl Response {
    /// Wire value on the frame.
    #[must_use]
    pub fn id(self) -> u16 {
        self as u16
    }

    /// Resolve a received id. `None` means not a known response.
    #[must_use]
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            50 => Some(Self::SubscriptionAlert),
            51 => Some(Self::ForceReauthorize),
            52 => Some(Self::MultiSessionError),
            1001 => Some(Self::Authorize),
            1003 => Some(Self::GetChannelList),
            1005 => Some(Self::ChannelLogin),
            1008 => Some(Self::SessionKeys),
            2001 => Some(Self::GetCharacterInfo),
            2003 => Some(Self::RoomList),
            2006 => Some(Self::CreateRoom),
            2013 => Some(Self::MainRoomUserMessage),
            2014 => Some(Self::MainRoomAdminMessage),
            2018 => Some(Self::SendWhisper),
            3003 => Some(Self::JoinRoom),
            _ => None,
        }
    }
}

/// Server-to-client broadcasts (Phase 2 rooms plus the 4015
/// in-play stats echo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Event {
    RoomCreated = 2007,
    ReceiveWhisper = 2019,
    RoomArenaEcho = 4003,
    GameStatsUpdate = 4015,
}

impl Event {
    /// Wire value on the frame.
    #[must_use]
    pub fn id(self) -> u16 {
        self as u16
    }

    /// Resolve a received id. `None` means not a known broadcast.
    #[must_use]
    pub fn from_id(id: u16) -> Option<Self> {
        match id {
            2007 => Some(Self::RoomCreated),
            2019 => Some(Self::ReceiveWhisper),
            4003 => Some(Self::RoomArenaEcho),
            4015 => Some(Self::GameStatsUpdate),
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
        assert_eq!(Request::from_id(2000), Some(Request::GetCharacterInfo));
        assert_eq!(Request::from_id(3002), Some(Request::JoinWaiting));
        assert_eq!(Request::from_id(2002), Some(Request::GetChannelInfo));
        assert_eq!(Request::from_id(65520), Some(Request::Terminate));
    }

    #[test]
    fn rejects_unknown_request_id() {
        assert_eq!(Request::from_id(9999), None);
    }

    #[test]
    fn resolves_song_request_ids() {
        assert_eq!(Request::from_id(4000), Some(Request::SetRoomMusic));
        assert_eq!(Request::from_id(4010), Some(Request::StartGame));
        assert_eq!(Request::from_id(4012), Some(Request::ConfirmMusicLoaded));
        assert_eq!(Request::from_id(4014), Some(Request::UpdateGameStats));
        assert_eq!(Request::from_id(4016), Some(Request::SubmitScore));
        assert_eq!(Request::from_id(4021), Some(Request::ExitPlaying));
        assert_eq!(Request::from_id(4030), Some(Request::GetMusicList));
        assert_eq!(Request::from_id(4036), Some(Request::SyncMemberMusicState));
        assert_eq!(Request::from_id(4038), Some(Request::SetDownloadProgress));
        assert_eq!(Request::from_id(4041), Some(Request::FinalizeRank));
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
        assert_eq!(Response::from_id(2003), Some(Response::RoomList));
        assert_eq!(Response::from_id(1002), None);
    }

    #[test]
    fn resolves_room_ids() {
        assert_eq!(Request::from_id(2004), Some(Request::CreateRoom));
        assert_eq!(Request::from_id(2012), Some(Request::SendMainRoomMessage));
        assert_eq!(Request::from_id(2017), Some(Request::SendWhisper));
        assert_eq!(Response::from_id(2006), Some(Response::CreateRoom));
        assert_eq!(Response::from_id(2013), Some(Response::MainRoomUserMessage));
        assert_eq!(
            Response::from_id(2014),
            Some(Response::MainRoomAdminMessage)
        );
        assert_eq!(Response::from_id(2018), Some(Response::SendWhisper));
        assert_eq!(Event::from_id(2007), Some(Event::RoomCreated));
        assert_eq!(Event::from_id(2019), Some(Event::ReceiveWhisper));
        assert_eq!(Event::from_id(2006), None);
    }

    #[test]
    fn resolves_game_stats_update_event() {
        assert_eq!(Event::from_id(4003), Some(Event::RoomArenaEcho));
        assert_eq!(Event::from_id(4015), Some(Event::GameStatsUpdate));
        assert_eq!(Event::from_id(4014), None);
    }

    #[test]
    fn resolves_character_info_id() {
        assert_eq!(Response::from_id(2001), Some(Response::GetCharacterInfo));
        assert_eq!(Response::from_id(2000), None);
    }

    #[test]
    fn resolves_session_alert_ids() {
        assert_eq!(Response::from_id(50), Some(Response::SubscriptionAlert));
        assert_eq!(Response::from_id(51), Some(Response::ForceReauthorize));
        assert_eq!(Response::from_id(52), Some(Response::MultiSessionError));
        assert_eq!(Response::from_id(53), None);
    }
}
