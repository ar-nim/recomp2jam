//! Song-select and in-play sync messages (plan Phases 3–4).
//!
//! Layouts mirror Memoryer's `Source/Memoryer` field orders
//! (clean-room C#); each type notes its exe anchor, and
//! Memoryer-only shapes stay labeled corroboration — the exe's
//! per-opcode handlers in this family are still open.

use super::error::Error;
use super::message::WireMessage;

/// In-play stat channel (`UpdateGameStats` 4014 kind byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StatKind {
    /// Lifebar value (tick-driven site, file `0x4eaec0`).
    Life = 0,
    /// Jam progress (per-hit Cool/Good tails).
    Jam = 1,
}

impl StatKind {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse a wire byte; unknown channels are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 0–1.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Life),
            1 => Ok(Self::Jam),
            _ => Err(Error::InvalidField {
                field: "stat kind",
                reason: "not 0 or 1",
            }),
        }
    }
}

macro_rules! song_empty {
    ($name:ident, $command:expr) => {
        #[doc = concat!("Empty-body message, command `", stringify!($command), "`")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name;

        impl WireMessage for $name {
            const COMMAND: u16 = $command;

            fn encode_body(&self) -> Vec<u8> {
                Vec::new()
            }

            fn decode_body(body: &[u8]) -> Result<Self, Error> {
                if body.is_empty() {
                    Ok(Self)
                } else {
                    Err(Error::TrailingBytes(body.len()))
                }
            }
        }
    };
}

// Host song flow: start, leave, rank, and list requests carry no
// parameters (Memoryer handlers read nothing but the session).
song_empty!(StartGame, 4010);
song_empty!(ExitPlaying, 4021);
song_empty!(GetMusicList, 4030);
song_empty!(FinalizeRank, 4041);

/// `SetRoomMusic` 4000: host picks the room song.
///
/// Memoryer-corroborated shape (music id + difficulty, matching the
/// `RoomListEntry`/`JoinedRoom` song fields); exe handler open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRoomMusic {
    /// Song id (`u16`, as listed per room).
    pub music_id: u16,
    /// Chart difficulty code (`Difficulty::from_code`).
    pub difficulty: u8,
}

/// Body length of [`SetRoomMusic`]: 2 + 1.
const SET_ROOM_MUSIC_BODY_LEN: usize = 3;

impl WireMessage for SetRoomMusic {
    const COMMAND: u16 = 4000;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SET_ROOM_MUSIC_BODY_LEN);
        body.extend_from_slice(&self.music_id.to_le_bytes());
        body.push(self.difficulty);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SET_ROOM_MUSIC_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SET_ROOM_MUSIC_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SET_ROOM_MUSIC_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - SET_ROOM_MUSIC_BODY_LEN));
        }
        Ok(Self {
            music_id: u16::from_le_bytes([body[0], body[1]]),
            difficulty: body[2],
        })
    }
}

/// `ConfirmMusicLoaded` 4012: client finished loading the chart.
///
/// Fields per `docs/net-protocol.md` §4c (`i32` power-skill id, then
/// speed); the speed byte width follows `JoinedRoom::speed` (`u8`) —
/// Memoryer corroboration, exe byte proof still open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmMusicLoaded {
    /// Power skill-ring id.
    pub power_skill_id: i32,
    /// Scroll-speed code.
    pub speed: u8,
}

/// Body length of [`ConfirmMusicLoaded`]: 4 + 1.
const CONFIRM_MUSIC_LOADED_BODY_LEN: usize = 5;

impl WireMessage for ConfirmMusicLoaded {
    const COMMAND: u16 = 4012;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(CONFIRM_MUSIC_LOADED_BODY_LEN);
        body.extend_from_slice(&self.power_skill_id.to_le_bytes());
        body.push(self.speed);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < CONFIRM_MUSIC_LOADED_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: CONFIRM_MUSIC_LOADED_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > CONFIRM_MUSIC_LOADED_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - CONFIRM_MUSIC_LOADED_BODY_LEN,
            ));
        }
        Ok(Self {
            power_skill_id: i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            speed: body[4],
        })
    }
}

/// `UpdateGameStats` 4014: mid-song life/jam tick (client to server).
///
/// Usage exe-verified (Life from the tick site, Jam from the Cool/Good
/// score tails); layout is Memoryer corroboration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateGameStats {
    /// Life or Jam channel.
    pub kind: StatKind,
    /// Channel value (`u16`).
    pub value: u16,
    /// Hit sequence number.
    pub sequence: i32,
    /// Running score.
    pub score: u32,
    /// Long-note bonus so far.
    pub long_note_score: i32,
}

/// Body length of [`UpdateGameStats`]: 1 + 2 + 4 + 4 + 4.
const UPDATE_GAME_STATS_BODY_LEN: usize = 15;

impl WireMessage for UpdateGameStats {
    const COMMAND: u16 = 4014;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(UPDATE_GAME_STATS_BODY_LEN);
        body.push(self.kind.as_u8());
        body.extend_from_slice(&self.value.to_le_bytes());
        body.extend_from_slice(&self.sequence.to_le_bytes());
        body.extend_from_slice(&self.score.to_le_bytes());
        body.extend_from_slice(&self.long_note_score.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < UPDATE_GAME_STATS_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: UPDATE_GAME_STATS_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > UPDATE_GAME_STATS_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - UPDATE_GAME_STATS_BODY_LEN,
            ));
        }
        Ok(Self {
            kind: StatKind::from_u8(body[0])?,
            value: u16::from_le_bytes([body[1], body[2]]),
            sequence: i32::from_le_bytes([body[3], body[4], body[5], body[6]]),
            score: u32::from_le_bytes([body[7], body[8], body[9], body[10]]),
            long_note_score: i32::from_le_bytes([body[11], body[12], body[13], body[14]]),
        })
    }
}

/// `GameStatsUpdate` 4015: server echo of one member's stats.
///
/// Peer lifebar plus live ranks in one packet (Memoryer layout);
/// the exe handler is unlocated, so this is a decode stub the play
/// loop will consume for peer bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameStatsUpdate {
    /// Member slot the stats belong to.
    pub member_id: u8,
    /// Raw channel byte; see [`StatKind::from_u8`].
    pub kind_code: u8,
    /// Channel value (`u16`).
    pub value: u16,
    /// Eight live-rank bytes.
    pub ranks: [u8; 8],
}

/// Body length of [`GameStatsUpdate`]: 1 + 1 + 2 + 8.
const GAME_STATS_UPDATE_BODY_LEN: usize = 12;

impl GameStatsUpdate {
    /// Interpret the channel byte. `None` means unknown.
    #[must_use]
    pub fn kind(self) -> Option<StatKind> {
        StatKind::from_u8(self.kind_code).ok()
    }
}

impl WireMessage for GameStatsUpdate {
    const COMMAND: u16 = 4015;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(GAME_STATS_UPDATE_BODY_LEN);
        body.push(self.member_id);
        body.push(self.kind_code);
        body.extend_from_slice(&self.value.to_le_bytes());
        body.extend_from_slice(&self.ranks);
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < GAME_STATS_UPDATE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: GAME_STATS_UPDATE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > GAME_STATS_UPDATE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - GAME_STATS_UPDATE_BODY_LEN,
            ));
        }
        let mut ranks = [0u8; 8];
        ranks.copy_from_slice(&body[4..12]);
        Ok(Self {
            member_id: body[0],
            kind_code: body[1],
            value: u16::from_le_bytes([body[2], body[3]]),
            ranks,
        })
    }
}

/// Read one little-endian `u16` at `at` (bounds checked by callers).
fn word_at(body: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([body[at], body[at + 1]])
}

/// `SubmitScore` 4016: end-of-song tally (client to server).
///
/// Layout corroborated field-for-field by Memoryer
/// (`docs/net-protocol.md` §4c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmitScore {
    /// Cool hits.
    pub cool: u16,
    /// Good hits.
    pub good: u16,
    /// Bad hits.
    pub bad: u16,
    /// Misses.
    pub miss: u16,
    /// Best combo.
    pub max_combo: u16,
    /// Final jam level.
    pub jam: u16,
    /// Best jam level.
    pub max_jam: u16,
    /// Final score.
    pub score: u32,
    /// Remaining life.
    pub life: u8,
    /// Scroll-speed code.
    pub speed: u8,
    /// Long-note bonus total.
    pub long_note_score: i32,
}

/// Body length of [`SubmitScore`]: 7×2 + 4 + 1 + 1 + 4.
const SUBMIT_SCORE_BODY_LEN: usize = 24;

impl WireMessage for SubmitScore {
    const COMMAND: u16 = 4016;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SUBMIT_SCORE_BODY_LEN);
        for tally in [
            self.cool,
            self.good,
            self.bad,
            self.miss,
            self.max_combo,
            self.jam,
            self.max_jam,
        ] {
            body.extend_from_slice(&tally.to_le_bytes());
        }
        body.extend_from_slice(&self.score.to_le_bytes());
        body.push(self.life);
        body.push(self.speed);
        body.extend_from_slice(&self.long_note_score.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SUBMIT_SCORE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SUBMIT_SCORE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SUBMIT_SCORE_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - SUBMIT_SCORE_BODY_LEN));
        }
        Ok(Self {
            cool: word_at(body, 0),
            good: word_at(body, 2),
            bad: word_at(body, 4),
            miss: word_at(body, 6),
            max_combo: word_at(body, 8),
            jam: word_at(body, 10),
            max_jam: word_at(body, 12),
            score: u32::from_le_bytes([body[14], body[15], body[16], body[17]]),
            life: body[18],
            speed: body[19],
            long_note_score: i32::from_le_bytes([body[20], body[21], body[22], body[23]]),
        })
    }
}

/// `SyncMemberMusicState` 4036: publish our song readiness.
///
/// Single state byte like `JoinRoom::music_state`; Memoryer
/// corroboration, exe byte proof still open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncMemberMusicState {
    /// Song state byte.
    pub music_state: u8,
}

/// Body length of [`SyncMemberMusicState`].
const SYNC_MEMBER_MUSIC_STATE_BODY_LEN: usize = 1;

impl WireMessage for SyncMemberMusicState {
    const COMMAND: u16 = 4036;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.music_state]
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SYNC_MEMBER_MUSIC_STATE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SYNC_MEMBER_MUSIC_STATE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SYNC_MEMBER_MUSIC_STATE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - SYNC_MEMBER_MUSIC_STATE_BODY_LEN,
            ));
        }
        Ok(Self {
            music_state: body[0],
        })
    }
}

/// `SetDownloadProgress` 4038: FTP fetch progress (progress, member).
///
/// Field order per `docs/net-protocol.md` §4c.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDownloadProgress {
    /// Percent downloaded.
    pub progress: u8,
    /// Member slot downloading.
    pub member_id: u8,
}

/// Body length of [`SetDownloadProgress`]: 1 + 1.
const SET_DOWNLOAD_PROGRESS_BODY_LEN: usize = 2;

impl WireMessage for SetDownloadProgress {
    const COMMAND: u16 = 4038;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.progress, self.member_id]
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SET_DOWNLOAD_PROGRESS_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SET_DOWNLOAD_PROGRESS_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SET_DOWNLOAD_PROGRESS_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - SET_DOWNLOAD_PROGRESS_BODY_LEN,
            ));
        }
        Ok(Self {
            progress: body[0],
            member_id: body[1],
        })
    }
}

#[cfg(test)]
mod song_sync_tests {
    use super::*;
    use crate::command::{Event, Request};
    use crate::frame::decode_frame;

    #[test]
    fn command_ids_match_song_types() {
        assert_eq!(SetRoomMusic::COMMAND, Request::SetRoomMusic as u16);
        assert_eq!(StartGame::COMMAND, Request::StartGame as u16);
        assert_eq!(
            ConfirmMusicLoaded::COMMAND,
            Request::ConfirmMusicLoaded as u16
        );
        assert_eq!(UpdateGameStats::COMMAND, Request::UpdateGameStats as u16);
        assert_eq!(GameStatsUpdate::COMMAND, Event::GameStatsUpdate as u16);
        assert_eq!(SubmitScore::COMMAND, Request::SubmitScore as u16);
        assert_eq!(ExitPlaying::COMMAND, Request::ExitPlaying as u16);
        assert_eq!(GetMusicList::COMMAND, Request::GetMusicList as u16);
        assert_eq!(
            SyncMemberMusicState::COMMAND,
            Request::SyncMemberMusicState as u16
        );
        assert_eq!(
            SetDownloadProgress::COMMAND,
            Request::SetDownloadProgress as u16
        );
        assert_eq!(FinalizeRank::COMMAND, Request::FinalizeRank as u16);
    }

    #[test]
    fn stat_kind_names_both_channels() {
        assert_eq!(StatKind::from_u8(0), Ok(StatKind::Life));
        assert_eq!(StatKind::from_u8(1), Ok(StatKind::Jam));
        assert_eq!(StatKind::Life.as_u8(), 0);
        assert_eq!(StatKind::Jam.as_u8(), 1);
    }

    #[test]
    fn stat_kind_rejects_unknown_channel() {
        assert_eq!(
            StatKind::from_u8(9),
            Err(Error::InvalidField {
                field: "stat kind",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn set_room_music_encodes_id_then_difficulty() {
        let select = SetRoomMusic {
            music_id: 757,
            difficulty: 2,
        };
        assert_eq!(select.encode_body(), vec![0xF5, 0x02, 0x02]);
        let frame = decode_frame(&select.encode()).unwrap();
        assert_eq!(frame.id, 4000);
        assert_eq!(SetRoomMusic::decode_frame(&frame), Ok(select));
    }

    #[test]
    fn set_room_music_rejects_short_body() {
        assert_eq!(
            SetRoomMusic::decode_body(&[0xF5, 0x02]),
            Err(Error::TruncatedMessage { need: 3, have: 2 })
        );
    }

    #[test]
    fn empty_song_requests_encode_bare_frames() {
        assert_eq!(StartGame.encode(), vec![0x04, 0x00, 0xAA, 0x0F]);
        assert_eq!(ExitPlaying.encode(), vec![0x04, 0x00, 0xB5, 0x0F]);
        assert_eq!(GetMusicList.encode(), vec![0x04, 0x00, 0xBE, 0x0F]);
        assert_eq!(FinalizeRank.encode(), vec![0x04, 0x00, 0xC9, 0x0F]);
    }

    #[test]
    fn empty_song_requests_reject_payload() {
        let frame = decode_frame(&StartGame.encode()).unwrap();
        assert_eq!(
            StartGame::decode_body(&[0x01]),
            Err(Error::TrailingBytes(1))
        );
        assert_eq!(
            ExitPlaying::decode_frame(&frame),
            Err(Error::UnknownCommand(4010))
        );
    }

    #[test]
    fn confirm_music_loaded_round_trips() {
        let loaded = ConfirmMusicLoaded {
            power_skill_id: 0x0102_0304,
            speed: 5,
        };
        assert_eq!(loaded.encode_body(), vec![0x04, 0x03, 0x02, 0x01, 0x05]);
        let frame = decode_frame(&loaded.encode()).unwrap();
        assert_eq!(frame.id, 4012);
        assert_eq!(ConfirmMusicLoaded::decode_frame(&frame), Ok(loaded));
    }

    #[test]
    fn confirm_music_loaded_rejects_short_body() {
        assert_eq!(
            ConfirmMusicLoaded::decode_body(&[0x01, 0x02]),
            Err(Error::TruncatedMessage { need: 5, have: 2 })
        );
    }

    fn jam_tick() -> UpdateGameStats {
        UpdateGameStats {
            kind: StatKind::Jam,
            value: 0x1234,
            sequence: 0x0102_0304,
            score: 0x0506_0708,
            long_note_score: 0x090A_0B0C,
        }
    }

    #[test]
    fn update_game_stats_encodes_documented_order() {
        assert_eq!(
            jam_tick().encode_body(),
            vec![
                0x01, 0x34, 0x12, 0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06, 0x05, 0x0C, 0x0B, 0x0A,
                0x09,
            ]
        );
    }

    #[test]
    fn update_game_stats_round_trips_through_frame() {
        let frame = decode_frame(&jam_tick().encode()).unwrap();
        assert_eq!(frame.id, 4014);
        assert_eq!(UpdateGameStats::decode_frame(&frame), Ok(jam_tick()));
    }

    #[test]
    fn update_game_stats_rejects_bad_channel() {
        let mut body = jam_tick().encode_body();
        body[0] = 0x09;
        assert_eq!(
            UpdateGameStats::decode_body(&body),
            Err(Error::InvalidField {
                field: "stat kind",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn game_stats_update_decodes_peer_bar() {
        let body = vec![
            0x02, 0x00, 0xE8, 0x03, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        ];
        let update = GameStatsUpdate::decode_body(&body).unwrap();
        assert_eq!(update.member_id, 2);
        assert_eq!(update.kind(), Some(StatKind::Life));
        assert_eq!(update.value, 1000);
        assert_eq!(update.ranks, [1, 2, 3, 4, 5, 6, 7, 8]);
        let frame = decode_frame(&update.encode()).unwrap();
        assert_eq!(frame.id, 4015);
        assert_eq!(GameStatsUpdate::decode_frame(&frame), Ok(update));
    }

    #[test]
    fn game_stats_update_keeps_unknown_channel_raw() {
        let mut update = GameStatsUpdate {
            member_id: 1,
            kind_code: 9,
            value: 7,
            ranks: [0; 8],
        };
        let back = GameStatsUpdate::decode_body(&update.encode_body()).unwrap();
        assert_eq!(back.kind(), None);
        assert_eq!(back.kind_code, 9);
        update.ranks[0] = 1;
        assert_ne!(
            GameStatsUpdate::decode_body(&update.encode_body())
                .unwrap()
                .ranks,
            [0; 8]
        );
    }

    #[test]
    fn game_stats_update_rejects_short_body() {
        assert_eq!(
            GameStatsUpdate::decode_body(&[0x01; 11]),
            Err(Error::TruncatedMessage { need: 12, have: 11 })
        );
    }

    fn final_tally() -> SubmitScore {
        SubmitScore {
            cool: 1,
            good: 2,
            bad: 3,
            miss: 4,
            max_combo: 5,
            jam: 6,
            max_jam: 7,
            score: 0x1122_3344,
            life: 100,
            speed: 3,
            long_note_score: 0x5566_7788,
        }
    }

    #[test]
    fn submit_score_encodes_documented_order() {
        assert_eq!(
            final_tally().encode_body(),
            vec![
                0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x06, 0x00, 0x07, 0x00,
                0x44, 0x33, 0x22, 0x11, 0x64, 0x03, 0x88, 0x77, 0x66, 0x55,
            ]
        );
    }

    #[test]
    fn submit_score_round_trips_through_frame() {
        let frame = decode_frame(&final_tally().encode()).unwrap();
        assert_eq!(frame.id, 4016);
        assert_eq!(SubmitScore::decode_frame(&frame), Ok(final_tally()));
    }

    #[test]
    fn submit_score_rejects_trailing_bytes() {
        let mut body = final_tally().encode_body();
        body.push(0x00);
        assert_eq!(
            SubmitScore::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn sync_member_music_state_round_trips() {
        let state = SyncMemberMusicState { music_state: 2 };
        assert_eq!(state.encode_body(), vec![0x02]);
        let frame = decode_frame(&state.encode()).unwrap();
        assert_eq!(frame.id, 4036);
        assert_eq!(SyncMemberMusicState::decode_frame(&frame), Ok(state));
    }

    #[test]
    fn set_download_progress_encodes_progress_first() {
        let progress = SetDownloadProgress {
            progress: 75,
            member_id: 3,
        };
        assert_eq!(progress.encode_body(), vec![75, 3]);
        let frame = decode_frame(&progress.encode()).unwrap();
        assert_eq!(frame.id, 4038);
        assert_eq!(SetDownloadProgress::decode_frame(&frame), Ok(progress));
    }
}
