//! Command ids (Memoryer `Commands.cs`; exe sends the C->S ones).
//!
//! Odd/even pairing: request `N` C->S, response S->C `N+1`,
//! events S->C broadcasts. Only Phase-1 login/channel opcodes
//! live here; later phases extend these enums.

use crate::macros::wire_enum;

wire_enum! {
    /// Client-to-server requests (Phase 1 login/channel plus the
    /// Phase 3/4 host song-flow requests).
    pub enum Request: u16, resolve from_id, encode id, {
        Authorize = 1000,
        GetChannelList = 1002,
        ChannelLogin = 1004,
        SessionKeys = 1007,
        GetCharacterInfo = 2000,
        GetChannelInfo = 2002,
        /// Room-stage member list the exe asks before it reports its
        /// own music (opcode `0x07da`, #341).
        GetUserList = 2010,
        /// Client's installed catalog ids (opcode `0x07e8`, #341).
        SendMusicList = 2024,
        CreateRoom = 2004,
        SendMainRoomMessage = 2012,
        Announce = 2015,
        SendWhisper = 2017,
        /// Channel exit leave (ticket #175 pin): the exe's channel-scene
        /// exit dispatch (control word 0x13200002) sends this empty via
        /// the standard vtable `+0x5c` send (file 0x48ff90). Memoryer
        /// answers 2022 and then accepts a fresh 1004; without it a
        /// second 1004 draws a 1001 `-1` auth error.
        ChannelLogout = 2021,
        GetMusicPlayRanking = 2027,
        GetMusicScoreList = 2030,
        GetMusicMaxScore = 2032,
        JoinWaiting = 3002,
        /// Waiting-room leave (`0x0BBD`); a `u16` exit type, 0 = Normal.
        /// Memoryer broadcasts 3007 to the remaining room members.
        ExitWaiting = 3005,
        SetRoomMusic = 4000,
        SetRoomArena = 4002,
        SetRoomTeam = 4004,
        Ready = 4008,
        StartGame = 4010,
        ConfirmMusicLoaded = 4012,
        UpdateGameStats = 4014,
        SubmitScore = 4016,
        ExitPlaying = 4021,
        GetMusicList = 4030,
        /// Music state announcement (docs/music-availability.md §2).
        UpdateMusicState = 4025,
        /// The acquire ping a room master sends when its START gate
        /// finds a member without the room song
        /// (docs/music-availability.md §4; exe gate verdict -2, gate
        /// VA `0x4b5560`, send site file `0xb23d2`).
        AcquireMusicRequest = 4034,
        SyncMemberMusicState = 4036,
        SetDownloadProgress = 4038,
        FinalizeRank = 4041,
        /// Download-complete announce (docs/music-availability.md §4).
        SyncMusicDownload = 5012,
        Terminate = 65520,
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

wire_enum! {
    /// Server-to-client responses (Phase 1 subset).
    pub enum Response: u16, resolve from_id, encode id, {
        SubscriptionAlert = 50,
        ForceReauthorize = 51,
        MultiSessionError = 52,
        Authorize = 1001,
        GetChannelList = 1003,
        ChannelLogin = 1005,
        SessionKeys = 1008,
        GetCharacterInfo = 2001,
        RoomList = 2003,
        /// `GetUserList` 2010 answer (opcode `0x07db`, #341).
        GetUserList = 2011,
        CreateRoom = 2006,
        MainRoomUserMessage = 2013,
        MainRoomAdminMessage = 2014,
        Announcement = 2016,
        SendWhisper = 2018,
        /// `ChannelLogout` 2021 answer (ticket #175 probe): `i32` result,
        /// 0 on success. No dedicated decoder yet — the session drain
        /// skips it; decode when a consumer needs the code (OPEN).
        ChannelLogout = 2022,
        GetMusicPlayRanking = 2028,
        GetMusicScoreList = 2031,
        GetMusicMaxScore = 2033,
        JoinRoom = 3003,
    }
}

wire_enum! {
    /// Server-to-client broadcasts (Phase 2 rooms plus the 4015
    /// in-play stats echo and the 4017/4018/4040 tally answers).
    pub enum Event: u16, resolve from_id, encode id, {
        RoomCreated = 2005,
        /// `RoomRemoved` (`0x07D7`): the server disbands a room when
        /// its last member leaves and broadcasts its room number to
        /// the whole channel (#380).
        RoomRemoved = 2007,
        ReceiveWhisper = 2019,
        RankNotification = 2029,
        RoomSlotUpdate = 3009,
        /// `UserJoinWaiting` (`0x0BBC`): one peer took a seat. Memoryer
        /// sends this to the room members except the joiner, which reads
        /// the existing seats from its 3003 answer instead (#379).
        UserJoinWaiting = 3004,
        /// `UserLeaveWaiting` (`0x0BBF`): one peer left a seat (#379).
        UserLeaveWaiting = 3007,
        /// The room's song changed; the client restages it and
        /// re-announces its music state (the exe's 4001 arm, VAs
        /// `0x4ab591`/`0x4b0d4e`; handler VA `0x55f2d0`).
        WaitingMusicChanged = 4001,
        RoomArenaEcho = 4003,
        RoomTeamChanged = 4007,
        UserReadyStateChanged = 4009,
        GameStatsUpdate = 4015,
        /// Per-member music states the server hands out on START
        /// (docs/music-availability.md §2/§4).
        AcquireMusic = 4035,
        /// One member's music state change (§2).
        SyncMemberMusicState = 4037,
        /// One member's download percentage (§2).
        DownloadProgressChanged = 4039,
        /// A member left the game; every waiting room re-announces its
        /// music state (the exe's 4022 arm, VAs `0x4ab658`/`0x4b0e12`).
        UserLeaveGame = 4022,
        /// The room's skills changed; the waiting room re-announces its
        /// music state (VAs `0x4ab90c`/`0x4b10fd`).
        WaitingSkillChanged = 4024,
        /// The room's skill-ring set changed; the waiting room
        /// re-announces its music state (VA `0x4b17dc`).
        WaitingSkillExChanged = 4058,
        ScoreSubmission = 4017,
        ScoreCompleted = 4018,
        GameRankUpdate = 4040,
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
        assert_eq!(Request::from_id(3005), Some(Request::ExitWaiting));
        assert_eq!(Request::from_id(2002), Some(Request::GetChannelInfo));
        assert_eq!(Request::from_id(2010), Some(Request::GetUserList));
        assert_eq!(Request::from_id(2024), Some(Request::SendMusicList));
        assert_eq!(Request::from_id(65520), Some(Request::Terminate));
    }

    #[test]
    fn rejects_unknown_request_id() {
        assert_eq!(Request::from_id(9999), None);
    }

    #[test]
    fn request_ids_round_trip() {
        let all = [
            Request::Authorize,
            Request::GetChannelList,
            Request::ChannelLogin,
            Request::SessionKeys,
            Request::GetCharacterInfo,
            Request::GetChannelInfo,
            Request::GetUserList,
            Request::SendMusicList,
            Request::CreateRoom,
            Request::SendMainRoomMessage,
            Request::Announce,
            Request::SendWhisper,
            Request::ChannelLogout,
            Request::GetMusicPlayRanking,
            Request::GetMusicScoreList,
            Request::GetMusicMaxScore,
            Request::JoinWaiting,
            Request::ExitWaiting,
            Request::SetRoomMusic,
            Request::SetRoomArena,
            Request::SetRoomTeam,
            Request::Ready,
            Request::StartGame,
            Request::ConfirmMusicLoaded,
            Request::UpdateGameStats,
            Request::SubmitScore,
            Request::ExitPlaying,
            Request::GetMusicList,
            Request::SyncMemberMusicState,
            Request::SetDownloadProgress,
            Request::FinalizeRank,
            Request::UpdateMusicState,
            Request::SyncMusicDownload,
            Request::Terminate,
        ];
        for request in all {
            assert_eq!(Request::from_id(request.id()), Some(request));
        }
        assert_eq!(Request::from_id(u16::MAX), None);
    }

    #[test]
    fn resolves_song_request_ids() {
        assert_eq!(Request::from_id(4000), Some(Request::SetRoomMusic));
        assert_eq!(Request::from_id(4004), Some(Request::SetRoomTeam));
        assert_eq!(Request::from_id(4008), Some(Request::Ready));
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
        assert_eq!(Response::from_id(2022), Some(Response::ChannelLogout));
        assert_eq!(Response::from_id(1002), None);
    }

    #[test]
    fn response_ids_round_trip() {
        let all = [
            Response::SubscriptionAlert,
            Response::ForceReauthorize,
            Response::MultiSessionError,
            Response::Authorize,
            Response::GetChannelList,
            Response::ChannelLogin,
            Response::SessionKeys,
            Response::GetCharacterInfo,
            Response::RoomList,
            Response::CreateRoom,
            Response::MainRoomUserMessage,
            Response::MainRoomAdminMessage,
            Response::Announcement,
            Response::SendWhisper,
            Response::GetMusicPlayRanking,
            Response::GetMusicScoreList,
            Response::GetMusicMaxScore,
            Response::JoinRoom,
        ];
        for response in all {
            assert_eq!(Response::from_id(response.id()), Some(response));
        }
        assert_eq!(Response::from_id(0), None);
    }

    #[test]
    fn event_ids_round_trip() {
        let all = [
            Event::RoomCreated,
            Event::RoomRemoved,
            Event::ReceiveWhisper,
            Event::RankNotification,
            Event::RoomSlotUpdate,
            Event::UserJoinWaiting,
            Event::UserLeaveWaiting,
            Event::RoomArenaEcho,
            Event::RoomTeamChanged,
            Event::UserReadyStateChanged,
            Event::GameStatsUpdate,
            Event::AcquireMusic,
            Event::SyncMemberMusicState,
            Event::DownloadProgressChanged,
            Event::ScoreSubmission,
            Event::ScoreCompleted,
            Event::GameRankUpdate,
            Event::WaitingMusicChanged,
            Event::UserLeaveGame,
            Event::WaitingSkillChanged,
            Event::WaitingSkillExChanged,
        ];
        for event in all {
            assert_eq!(Event::from_id(event.id()), Some(event));
        }
        assert_eq!(Event::from_id(0), None);
    }

    #[test]
    fn resolves_room_ids() {
        assert_eq!(Request::from_id(2004), Some(Request::CreateRoom));
        assert_eq!(Request::from_id(2012), Some(Request::SendMainRoomMessage));
        assert_eq!(Request::from_id(2015), Some(Request::Announce));
        assert_eq!(Request::from_id(2017), Some(Request::SendWhisper));
        assert_eq!(Response::from_id(2006), Some(Response::CreateRoom));
        assert_eq!(Response::from_id(2013), Some(Response::MainRoomUserMessage));
        assert_eq!(
            Response::from_id(2014),
            Some(Response::MainRoomAdminMessage)
        );
        assert_eq!(Response::from_id(2016), Some(Response::Announcement));
        assert_eq!(Response::from_id(2018), Some(Response::SendWhisper));
        assert_eq!(Event::from_id(2005), Some(Event::RoomCreated));
        assert_eq!(Event::from_id(2019), Some(Event::ReceiveWhisper));
        assert_eq!(Event::from_id(2029), Some(Event::RankNotification));
        assert_eq!(Event::from_id(2006), None);
    }

    #[test]
    fn resolves_game_stats_update_event() {
        assert_eq!(Event::from_id(3009), Some(Event::RoomSlotUpdate));
        assert_eq!(Event::from_id(4003), Some(Event::RoomArenaEcho));
        assert_eq!(Event::from_id(4007), Some(Event::RoomTeamChanged));
        assert_eq!(Event::from_id(4009), Some(Event::UserReadyStateChanged));
        assert_eq!(Event::from_id(4015), Some(Event::GameStatsUpdate));
        assert_eq!(Event::from_id(4014), None);
    }

    #[test]
    fn resolves_tally_answer_events() {
        assert_eq!(Event::from_id(4017), Some(Event::ScoreSubmission));
        assert_eq!(Event::from_id(2007), Some(Event::RoomRemoved));
        assert_eq!(Event::from_id(3004), Some(Event::UserJoinWaiting));
        assert_eq!(Event::from_id(3007), Some(Event::UserLeaveWaiting));
        assert_eq!(Event::from_id(4018), Some(Event::ScoreCompleted));
        assert_eq!(Event::from_id(4040), Some(Event::GameRankUpdate));
        assert_eq!(Event::from_id(4016), None);
        assert_eq!(Event::from_id(4041), None);
    }

    #[test]
    fn resolves_music_state_family_ids() {
        // docs/music-availability.md §2: 4025/4036/4038 go up, while
        // 4035/4037/4039 arrive as events; 5012 rides the shop range.
        assert_eq!(Request::from_id(4025), Some(Request::UpdateMusicState));
        assert_eq!(Request::from_id(4036), Some(Request::SyncMemberMusicState));
        assert_eq!(Request::from_id(4038), Some(Request::SetDownloadProgress));
        assert_eq!(Request::from_id(5012), Some(Request::SyncMusicDownload));
        assert_eq!(Event::from_id(4035), Some(Event::AcquireMusic));
        assert_eq!(Event::from_id(4037), Some(Event::SyncMemberMusicState));
        assert_eq!(Event::from_id(4039), Some(Event::DownloadProgressChanged));
        assert_eq!(Event::from_id(4025), None);
        assert_eq!(Request::from_id(4035), None);
        assert_eq!(Event::from_id(5012), None);
    }

    #[test]
    fn music_shop_has_no_visitor_request_in_the_shop_family() {
        // Static-sweep guard for #179 story 35: the 5000 shop family
        // holds only the download announce (5012). There is no
        // request that asks to view another seat's Music Shop; the
        // side screen is local through `visit_side(MusicShop)` and
        // must never be graded as a passed visitor cell.
        let known_requests: Vec<u16> = (5000..=5100)
            .filter(|id| Request::from_id(*id).is_some())
            .collect();
        assert_eq!(known_requests, vec![5012]);
        assert_eq!(Request::from_id(5042), None, "no visitor opcode");
        assert_eq!(Response::from_id(5043), None, "no visitor answer");
    }

    #[test]
    fn resolves_the_start_gate_family_ids() {
        // docs/music-availability.md §4: the master's gate sends the
        // acquire ping 4034; the room re-announces 4025 off these four
        // events (exe arms 0x4ab591, 0x4ab658, 0x4ab90c, 0x4b17dc and
        // their normal-mode twins).
        assert_eq!(Request::from_id(4034), Some(Request::AcquireMusicRequest));
        assert_eq!(Event::from_id(4001), Some(Event::WaitingMusicChanged));
        assert_eq!(Event::from_id(4022), Some(Event::UserLeaveGame));
        assert_eq!(Event::from_id(4024), Some(Event::WaitingSkillChanged));
        assert_eq!(Event::from_id(4058), Some(Event::WaitingSkillExChanged));
        assert_eq!(Event::from_id(4034), None);
        assert_eq!(Request::from_id(4001), None);
    }

    #[test]
    fn resolves_ranking_ids() {
        assert_eq!(Request::from_id(2027), Some(Request::GetMusicPlayRanking));
        assert_eq!(Request::from_id(2030), Some(Request::GetMusicScoreList));
        assert_eq!(Request::from_id(2032), Some(Request::GetMusicMaxScore));
        assert_eq!(Response::from_id(2028), Some(Response::GetMusicPlayRanking));
        assert_eq!(Response::from_id(2031), Some(Response::GetMusicScoreList));
        assert_eq!(Response::from_id(2033), Some(Response::GetMusicMaxScore));
        assert_eq!(Request::from_id(2028), None);
        assert_eq!(Response::from_id(2027), None);
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
