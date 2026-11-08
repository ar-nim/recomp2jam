//! Song-select and in-play sync messages (plan Phases 3–4).
//!
//! Layouts mirror Memoryer's `Source/Memoryer` field orders
//! (clean-room C#); each type notes its exe anchor, and
//! Memoryer-only shapes stay labeled corroboration — most exe
//! per-opcode handlers in this family are still open, except the
//! 4003 arena echo (see [`RoomArenaEcho`]).

use super::error::Error;
use super::message::WireMessage;

/// In-play stat channel (`UpdateGameStats` 4014 kind word).
///
/// Wire width is live-proven: the gateway parses the kind as
/// Memoryer's `GameUpdateStatsType` (`ushort`; a one-byte kind
/// overruns into the value and the gateway drops the frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum StatKind {
    /// Lifebar value (tick-driven site, file `0x4eaec0`).
    Life = 0,
    /// Jam progress (per-hit Cool/Good tails).
    Jam = 1,
}

impl StatKind {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Parse a wire word; unknown channels are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 0–1.
    pub fn from_u16(value: u16) -> Result<Self, Error> {
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
/// Memoryer-corroborated shape (music id + difficulty + scroll
/// speed, matching the `RoomListEntry`/`JoinedRoom` song fields);
/// live-proven: a speed-less body is dropped by the gateway while
/// the four-byte form updates the room; exe handler open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRoomMusic {
    /// Song id (`u16`, as listed per room).
    pub music_id: u16,
    /// Chart difficulty code (`Difficulty::from_code`).
    pub difficulty: u8,
    /// Scroll-speed code (`GameSpeed` ordinal).
    pub speed: u8,
}

/// Body length of [`SetRoomMusic`]: 2 + 1 + 1.
const SET_ROOM_MUSIC_BODY_LEN: usize = 4;

impl WireMessage for SetRoomMusic {
    const COMMAND: u16 = 4000;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SET_ROOM_MUSIC_BODY_LEN);
        body.extend_from_slice(&self.music_id.to_le_bytes());
        body.push(self.difficulty);
        body.push(self.speed);
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
            speed: body[3],
        })
    }
}

/// `SetRoomArena` 4002: host picks the room stage.
///
/// Single little-endian `i32` carrying the room word the client
/// keeps in global `0x7789dc`: the stage number with the picker's
/// high-word flag (file `0xbb3cf`), read by the waiting map cycle
/// (files `0xb3cc6`-`0xb3ce3`). Width follows the dword store
/// (applier `0x4ad350`, file `0x4ad413`) and `JoinedRoom::arena`
/// (`i32`); Memoryer corroboration, exe handler open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRoomArena {
    /// Room arena word (stage + high-word flag).
    pub arena: i32,
}

/// Body length of [`SetRoomArena`]: one `i32`.
const SET_ROOM_ARENA_BODY_LEN: usize = 4;

impl SetRoomArena {
    /// Send builder: stage word for command 4002.
    #[must_use]
    pub fn new(arena: i32) -> Self {
        Self { arena }
    }

    /// Receive handler: store the word (applier `0x4ad350` keeps it
    /// in global `0x7789dc`, file `0x4ad413`).
    pub fn apply_to_store(&self, arena_word: &mut i32) {
        *arena_word = self.arena;
    }
}

impl WireMessage for SetRoomArena {
    const COMMAND: u16 = 4002;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(SET_ROOM_ARENA_BODY_LEN);
        body.extend_from_slice(&self.arena.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SET_ROOM_ARENA_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SET_ROOM_ARENA_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SET_ROOM_ARENA_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - SET_ROOM_ARENA_BODY_LEN));
        }
        Ok(Self::new(i32::from_le_bytes([
            body[0], body[1], body[2], body[3],
        ])))
    }
}

/// `RoomArenaEcho` 4003: server echo of the room stage.
///
/// Same one-word little-endian `i32` body as [`SetRoomArena`]: the
/// stage number with the picker's high-word flag. Exe-verified
/// handler at VA `0x55f3c0` (file `0x15f3c0`, 48 B): it reads exactly
/// 4 bytes straight into the arena global `0x7789dc` through the
/// packet-reader helper (`0x555d20`, size push at file `0x15f3c3`,
/// dest push at file `0x15f3c5`), then forwards
/// (`0x12`, 4003, the stored word, 0) into the UI event queue
/// (`0x478d50`, opcode push at file `0x15f3da`). No member slot, no
/// result code, no slot guard — the member-broadcast and
/// result-code shapes are ruled out. Reached from the central
/// dispatcher (file `0x158080`) through the 4003-base subtract slot:
/// table index 0 → case at file `0x158655` → the only image-wide
/// relative call into the handler (site file `0x158659`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomArenaEcho {
    /// Room arena word (stage + high-word flag).
    pub arena: i32,
}

impl RoomArenaEcho {
    /// Echo builder: stage word for command 4003.
    #[must_use]
    pub fn new(arena: i32) -> Self {
        Self { arena }
    }

    /// Receive handler: store the word (handler `0x55f3c0` keeps it
    /// in global `0x7789dc`, read size 4 at file `0x15f3c3`).
    pub fn apply_to_store(&self, arena_word: &mut i32) {
        *arena_word = self.arena;
    }
}

impl WireMessage for RoomArenaEcho {
    const COMMAND: u16 = 4003;

    fn encode_body(&self) -> Vec<u8> {
        SetRoomArena::new(self.arena).encode_body()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        SetRoomArena::decode_body(body).map(|request| Self::new(request.arena))
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

/// Body length of [`UpdateGameStats`]: 2 + 2 + 4 + 4 + 4.
const UPDATE_GAME_STATS_BODY_LEN: usize = 16;

impl UpdateGameStats {
    /// Life-channel tick (`StatKind::Life`) with its score tail.
    #[must_use]
    pub fn life_tick(value: u16, sequence: i32, score: u32, long_note_score: i32) -> Self {
        Self {
            kind: StatKind::Life,
            value,
            sequence,
            score,
            long_note_score,
        }
    }

    /// Jam-channel tick (`StatKind::Jam`) with its score tail.
    #[must_use]
    pub fn jam_tick(value: u16, sequence: i32, score: u32, long_note_score: i32) -> Self {
        Self {
            kind: StatKind::Jam,
            value,
            sequence,
            score,
            long_note_score,
        }
    }
}

impl WireMessage for UpdateGameStats {
    const COMMAND: u16 = 4014;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(UPDATE_GAME_STATS_BODY_LEN);
        body.extend_from_slice(&self.kind.as_u16().to_le_bytes());
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
            kind: StatKind::from_u16(u16::from_le_bytes([body[0], body[1]]))?,
            value: u16::from_le_bytes([body[2], body[3]]),
            sequence: i32::from_le_bytes([body[4], body[5], body[6], body[7]]),
            score: u32::from_le_bytes([body[8], body[9], body[10], body[11]]),
            long_note_score: i32::from_le_bytes([body[12], body[13], body[14], body[15]]),
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
    /// Raw channel word; see [`StatKind::from_u16`].
    pub kind_code: u16,
    /// Channel value (`u16`).
    pub value: u16,
    /// Eight live-rank bytes.
    pub ranks: [u8; 8],
}

/// Body length of [`GameStatsUpdate`]: 1 + 2 + 2 + 8.
///
/// Wire width is live-proven: the gateway broadcasts 13 bytes with
/// the channel as Memoryer's `GameUpdateStatsType` (`ushort`).
const GAME_STATS_UPDATE_BODY_LEN: usize = 13;

impl GameStatsUpdate {
    /// Interpret the channel word. `None` means unknown.
    #[must_use]
    pub fn kind(self) -> Option<StatKind> {
        StatKind::from_u16(self.kind_code).ok()
    }

    /// Typed stats write for the peer mirror: slot, channel, value.
    ///
    /// `None` when the channel word is unknown; the rank broadcast
    /// below still applies then.
    #[must_use]
    pub fn stats_channel(self) -> Option<(u8, StatKind, u16)> {
        self.kind().map(|kind| (self.member_id, kind, self.value))
    }

    /// Eight live-rank bytes for the peer mirror.
    #[must_use]
    pub fn rank_packet(self) -> [u8; 8] {
        self.ranks
    }
}

impl WireMessage for GameStatsUpdate {
    const COMMAND: u16 = 4015;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(GAME_STATS_UPDATE_BODY_LEN);
        body.push(self.member_id);
        body.extend_from_slice(&self.kind_code.to_le_bytes());
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
        ranks.copy_from_slice(&body[5..13]);
        Ok(Self {
            member_id: body[0],
            kind_code: u16::from_le_bytes([body[1], body[2]]),
            value: u16::from_le_bytes([body[3], body[4]]),
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
/// Single member-slot byte: the gateway indexes its room slots by
/// it (live-proven: slot 0 syncs and broadcasts member 0); the old
/// `music_state` name misread that byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncMemberMusicState {
    /// Room slot the sync belongs to.
    pub member_id: u8,
}

/// Body length of [`SyncMemberMusicState`].
const SYNC_MEMBER_MUSIC_STATE_BODY_LEN: usize = 1;

impl WireMessage for SyncMemberMusicState {
    const COMMAND: u16 = 4036;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.member_id]
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
        Ok(Self { member_id: body[0] })
    }
}

/// One song row of the `GetMusicList` 4031 answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicListEntry {
    /// Song id (`u16`, as selected per room).
    pub id: u16,
    /// Note count on EX.
    pub notes_ex: u16,
    /// Note count on NX.
    pub notes_nx: u16,
    /// Note count on HX.
    pub notes_hx: u16,
}

/// Wire size of one [`MusicListEntry`]: 4 × `u16`.
const MUSIC_LIST_ENTRY_LEN: usize = 8;

/// `GetMusicList` 4031 answer: song catalog plus shop flag.
///
/// Live-proven shape: `i16` entry count, then 8-byte rows, then a
/// one-byte free-music flag; further trailing bytes are ignored so
/// newer gateway fields cannot break the list step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicListAnswer {
    /// Catalog rows.
    pub entries: Vec<MusicListEntry>,
    /// Every song is free (`FreeMusic` server option).
    pub free_music: bool,
}

impl WireMessage for MusicListAnswer {
    const COMMAND: u16 = 4031;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(2 + self.entries.len() * MUSIC_LIST_ENTRY_LEN + 1);
        let count = i16::try_from(self.entries.len()).unwrap_or(i16::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for entry in &self.entries {
            body.extend_from_slice(&entry.id.to_le_bytes());
            body.extend_from_slice(&entry.notes_ex.to_le_bytes());
            body.extend_from_slice(&entry.notes_nx.to_le_bytes());
            body.extend_from_slice(&entry.notes_hx.to_le_bytes());
        }
        body.push(u8::from(self.free_music));
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 2 {
            return Err(Error::TruncatedMessage {
                need: 2,
                have: body.len(),
            });
        }
        let count = i16::from_le_bytes([body[0], body[1]]);
        let count = usize::try_from(count).map_err(|_| Error::InvalidField {
            field: "music list count",
            reason: "negative entry count",
        })?;
        let need = 2 + count * MUSIC_LIST_ENTRY_LEN + 1;
        if body.len() < need {
            return Err(Error::TruncatedMessage {
                need,
                have: body.len(),
            });
        }
        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            let at = 2 + index * MUSIC_LIST_ENTRY_LEN;
            entries.push(MusicListEntry {
                id: u16::from_le_bytes([body[at], body[at + 1]]),
                notes_ex: u16::from_le_bytes([body[at + 2], body[at + 3]]),
                notes_nx: u16::from_le_bytes([body[at + 4], body[at + 5]]),
                notes_hx: u16::from_le_bytes([body[at + 6], body[at + 7]]),
            });
        }
        Ok(Self {
            entries,
            free_music: body[2 + count * MUSIC_LIST_ENTRY_LEN] != 0,
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

// `Ready` 4008: the joined-guest readiness toggle (C->S). Empty body:
// all five exe send sites push a zero word then the opcode and send
// indirectly (`push 0; push 0xfa8`, files `0xacebe`, `0xb24f6`,
// `0xb36a0`, `0xb8450`, `0xb871f`) — no flag byte rides the wire.
// Memoryer agrees (`WaitingController.Ready()` takes no request).
song_empty!(Ready, 4008);

/// `MemberReadyStateChanged` 4009 event: one member flipped readiness.
///
/// Two bytes: `u8` member id, then the ready flag as one `0`/`1` byte.
/// Memoryer (`Messages/Events/Waiting/
/// `MemberReadyStateChangedEventData.cs`): byte `MemberId` (order 0),
/// bool `Ready` (order 1). Exe-verified handler at file `0x15f5b0`:
/// one size-1 member read through the packet-reader helper (`0x555d20`,
/// size pushed at file `0x15f5b4`), member guard `< 8` (compare at file
/// `0x15f5c6`, drop past it), then the ready byte reads straight into
/// the member table at slot `+0x32` (size-1 read at file `0x140d77`,
/// dest slot base `+0x32` at file `0x140d7c`) — self base `0x77cd00`
/// when the packet member matches the local id at `0x77cd04`, else
/// slot base `0x77ee90 + member * 0x2190` — then forwards
/// `(0x12, 4009, member, 0)` into the UI event queue (`0x478d50`,
/// pushes at files `0x15f60b`-`0x15f61f`). The same `+0x32` byte gates
/// the host START path (file `0xb5560`: nonzero means ready). Observed
/// with GNU objdump 2.47 slices plus Python byte scans over the
/// read-only 7.55 copy; no new REA dossier filed. Memoryer shapes are
/// corroboration; the two one-byte reads are the byte-verified part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberReadyStateChanged {
    /// Member slot (`< 8`; the exe drops wider values).
    pub member_id: u8,
    /// New readiness.
    pub ready: bool,
}

/// Body length of [`MemberReadyStateChanged`]: 1 + 1.
const MEMBER_READY_STATE_CHANGED_BODY_LEN: usize = 2;

/// Seat count behind the 4009 member guard (`cmp 8`, file `0x15f5c6`).
const READY_GUARD_SEAT_COUNT: u8 = 8;

impl WireMessage for MemberReadyStateChanged {
    const COMMAND: u16 = 4009;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.member_id, u8::from(self.ready)]
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < MEMBER_READY_STATE_CHANGED_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: MEMBER_READY_STATE_CHANGED_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > MEMBER_READY_STATE_CHANGED_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - MEMBER_READY_STATE_CHANGED_BODY_LEN,
            ));
        }
        if body[0] >= READY_GUARD_SEAT_COUNT {
            return Err(Error::InvalidField {
                field: "ready member",
                reason: "not below 8",
            });
        }
        let ready = match body[1] {
            0 => false,
            1 => true,
            _ => {
                return Err(Error::InvalidField {
                    field: "ready flag",
                    reason: "not 0 or 1",
                });
            }
        };
        Ok(Self {
            member_id: body[0],
            ready,
        })
    }
}

/// `ScoreSubmission` 4017: one member's tally reached the tracker.
///
/// Single `u8` member slot. Live-proven: the gateway broadcasts a
/// 1-byte body naming the submitting slot (local Memoryer Full-mode
/// song flow, `00` for the solo member). Exe shape corroboration:
/// case file `0x1586cc` (dispatcher subtract-base slot: byte-table
/// file `0x158a68` index `0x0e` holds slot `0x07`, address-table file
/// `0x1589f4` slot 7 — shared edges `ev_c7b26aac`/`ev_9412e595` in
/// `extracted/rea-4015-dossier.json`) calls handler VA `0x55fba0`
/// (file `0x15fba0`), which performs one size-1 read through the
/// packet-reader helper (`0x555d20`, size push at file `0x15fbad`).
/// Observed with GNU objdump 2.47 slices plus Python 3.14.9 byte
/// scans over the read-only 7.55 copy; no new REA dossier filed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreSubmission {
    /// Room slot whose tally arrived.
    pub member_id: u8,
}

/// Body length of [`ScoreSubmission`]: one member byte.
const SCORE_SUBMISSION_BODY_LEN: usize = 1;

impl WireMessage for ScoreSubmission {
    const COMMAND: u16 = 4017;

    fn encode_body(&self) -> Vec<u8> {
        vec![self.member_id]
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SCORE_SUBMISSION_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: SCORE_SUBMISSION_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > SCORE_SUBMISSION_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - SCORE_SUBMISSION_BODY_LEN));
        }
        Ok(Self { member_id: body[0] })
    }
}

/// Per-member verdict inside [`ScoreCompleted`] (Memoryer
/// `ScoreCompletedEventData.MatchResult`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MatchResult {
    /// Tied top score.
    Draw = 0,
    /// Sole top score.
    Win = 1,
    /// Beaten.
    Lose = 2,
}

impl MatchResult {
    /// Parse a wire byte; unknown verdicts are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 0–2.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Draw),
            1 => Ok(Self::Win),
            2 => Ok(Self::Lose),
            _ => Err(Error::InvalidField {
                field: "match result",
                reason: "not 0, 1, or 2",
            }),
        }
    }
}

/// One ranked tally row inside [`ScoreCompleted`] (Memoryer
/// `ScoreCompletedEventData.ScoreEntry`).
///
/// Every nullable field rides the wire as its raw value: the live
/// gateway populates all of them (the publisher assigns every field
/// and skips absent members instead of nulling fields), so a body
/// that ends mid-row is truncated, never sparse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreEntry {
    /// Room slot this row belongs to.
    pub member_id: u8,
    /// Tracker accepted the tally.
    pub active: bool,
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
    /// Final score.
    pub score: u32,
    /// Server-computed reward.
    pub reward: u16,
    /// Member level after the game.
    pub level: i32,
    /// Member experience after the game.
    pub experience: i32,
    /// Draw/win/lose verdict.
    pub result: MatchResult,
    /// Member gem after the game.
    pub gem: i32,
    /// Member cash point after the game.
    pub cash_point: i32,
    /// Scroll-speed code.
    pub speed: u8,
    /// Long-note bonus total.
    pub long_note_score: i32,
}

/// Wire size of one [`ScoreEntry`]:
/// 1 + 4 + 6×2 + 4 + 2 + 4 + 4 + 1 + 4 + 4 + 1 + 4.
const SCORE_ENTRY_LEN: usize = 45;
/// Wire size of the [`ScoreCompleted`] head: `i32` count + `u8` master.
const SCORE_COMPLETED_HEAD_LEN: usize = 5;
/// Most rows one [`ScoreCompleted`] carries (Memoryer `maxCount: 8`).
const SCORE_COMPLETED_MAX_ROWS: usize = 8;

/// Read one little-endian `u32` at `at`.
fn dword_at(body: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]])
}

/// Read one little-endian `i32` at `at`.
fn s32_at(body: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]])
}

/// Parse one [`ScoreEntry`] from exactly [`SCORE_ENTRY_LEN`] bytes.
fn decode_score_entry(chunk: &[u8]) -> Result<ScoreEntry, Error> {
    debug_assert_eq!(chunk.len(), SCORE_ENTRY_LEN);
    let active = match dword_at(chunk, 1) {
        0 => false,
        1 => true,
        _ => {
            return Err(Error::InvalidField {
                field: "score active",
                reason: "not 0 or 1",
            });
        }
    };
    Ok(ScoreEntry {
        member_id: chunk[0],
        active,
        cool: word_at(chunk, 5),
        good: word_at(chunk, 7),
        bad: word_at(chunk, 9),
        miss: word_at(chunk, 11),
        max_combo: word_at(chunk, 13),
        jam: word_at(chunk, 15),
        score: dword_at(chunk, 17),
        reward: word_at(chunk, 21),
        level: s32_at(chunk, 23),
        experience: s32_at(chunk, 27),
        result: MatchResult::from_u8(chunk[31])?,
        gem: s32_at(chunk, 32),
        cash_point: s32_at(chunk, 36),
        speed: chunk[40],
        long_note_score: s32_at(chunk, 41),
    })
}

/// Write one [`ScoreEntry`] in field order.
fn encode_score_entry(entry: &ScoreEntry, body: &mut Vec<u8>) {
    body.push(entry.member_id);
    body.extend_from_slice(&i32::from(entry.active).to_le_bytes());
    for tally in [
        entry.cool,
        entry.good,
        entry.bad,
        entry.miss,
        entry.max_combo,
        entry.jam,
    ] {
        body.extend_from_slice(&tally.to_le_bytes());
    }
    body.extend_from_slice(&entry.score.to_le_bytes());
    body.extend_from_slice(&entry.reward.to_le_bytes());
    body.extend_from_slice(&entry.level.to_le_bytes());
    body.extend_from_slice(&entry.experience.to_le_bytes());
    body.push(entry.result as u8);
    body.extend_from_slice(&entry.gem.to_le_bytes());
    body.extend_from_slice(&entry.cash_point.to_le_bytes());
    body.push(entry.speed);
    body.extend_from_slice(&entry.long_note_score.to_le_bytes());
}

/// `ScoreCompleted` 4018: final ranked tallies for the room.
///
/// `i32` row count, `u8` room-master slot, then that many fixed
/// [`SCORE_ENTRY_LEN`]-byte rows. Live-proven: a solo submit against
/// the local Memoryer Full-mode gateway broadcasts a 50-byte body
/// (count 1, master 0, one full row echoing the 4016 tally plus
/// server rewards/levels — see
/// `score_completed_decodes_live_capture`). Exe shape
/// corroboration: case file `0x1586dd` (byte-table file `0x158a68`
/// index `0x0f` holds slot `0x08`, address-table file `0x1589f4`
/// slot 8 — shared edges `ev_c7b26aac`/`ev_9412e595`) calls handler
/// VA `0x55fbe0` (file `0x15fbe0`), which reads sizes 4/1/1/4 (count,
/// master, member, active flag) then loops over the member table
/// (stride `0x2190` onto base `0x77ee90`, immediates in range).
/// Observed with GNU objdump 2.47 slices plus Python 3.14.9 byte
/// scans over the read-only 7.55 copy; no new REA dossier filed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreCompleted {
    /// Room-master slot.
    pub room_master: u8,
    /// Ranked tally rows (best score first).
    pub scores: Vec<ScoreEntry>,
}

impl WireMessage for ScoreCompleted {
    const COMMAND: u16 = 4018;

    fn encode_body(&self) -> Vec<u8> {
        let mut body =
            Vec::with_capacity(SCORE_COMPLETED_HEAD_LEN + self.scores.len() * SCORE_ENTRY_LEN);
        let count = i32::try_from(self.scores.len()).unwrap_or(i32::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        body.push(self.room_master);
        for entry in &self.scores {
            encode_score_entry(entry, &mut body);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < SCORE_COMPLETED_HEAD_LEN {
            return Err(Error::TruncatedMessage {
                need: SCORE_COMPLETED_HEAD_LEN,
                have: body.len(),
            });
        }
        let count = s32_at(body, 0);
        let count = usize::try_from(count).map_err(|_| Error::InvalidField {
            field: "score count",
            reason: "negative entry count",
        })?;
        if count > SCORE_COMPLETED_MAX_ROWS {
            return Err(Error::InvalidField {
                field: "score count",
                reason: "more than 8 entries",
            });
        }
        let need = SCORE_COMPLETED_HEAD_LEN + count * SCORE_ENTRY_LEN;
        if body.len() < need {
            return Err(Error::TruncatedMessage {
                need,
                have: body.len(),
            });
        }
        if body.len() > need {
            return Err(Error::TrailingBytes(body.len() - need));
        }
        let mut scores = Vec::with_capacity(count);
        for index in 0..count {
            let at = SCORE_COMPLETED_HEAD_LEN + index * SCORE_ENTRY_LEN;
            scores.push(decode_score_entry(&body[at..at + SCORE_ENTRY_LEN])?);
        }
        Ok(Self {
            room_master: body[4],
            scores,
        })
    }
}

/// `GameRankUpdate` 4040: leaderboard place per member.
///
/// Eight rank bytes; byte `i` holds the member id on leaderboard
/// place `i` (`0xff` marks an empty place — the same
/// member-ids-plus-`0xff`-pad shape as the 4015 rank bytes,
/// live-proven there per `docs/relay.md` §12). Memoryer-only shape:
/// the `FinalizeRank` broadcast stays commented out server-side, so
/// no live bytes exist; the decoder is ready for the wire form when
/// a gateway emits it. Exe shape corroboration: case file `0x158798`
/// (byte-table file `0x158a68` index `0x25` holds slot `0x12`,
/// address-table file `0x1589f4` slot `0x12` — shared edges
/// `ev_c7b26aac`/`ev_9412e595`) calls handler VA `0x559a60` (file
/// `0x159a60`), which reads byte-sized through the packet-reader
/// helper (`0x555d20`, size push at file `0x159a8b`) and walks the
/// member table (stride `0x2190`, self base `0x77cd00`, table base
/// `0x77ee90`, immediates in range). Observed with GNU objdump 2.47
/// slices plus Python 3.14.9 byte scans over the read-only 7.55 copy;
/// no new REA dossier filed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameRankUpdate {
    /// Member id on each leaderboard place (`0xff` = empty).
    pub member_ranks: [u8; 8],
}

/// Body length of [`GameRankUpdate`]: eight rank bytes.
const GAME_RANK_UPDATE_BODY_LEN: usize = 8;

impl WireMessage for GameRankUpdate {
    const COMMAND: u16 = 4040;

    fn encode_body(&self) -> Vec<u8> {
        self.member_ranks.to_vec()
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < GAME_RANK_UPDATE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: GAME_RANK_UPDATE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > GAME_RANK_UPDATE_BODY_LEN {
            return Err(Error::TrailingBytes(body.len() - GAME_RANK_UPDATE_BODY_LEN));
        }
        let mut member_ranks = [0u8; GAME_RANK_UPDATE_BODY_LEN];
        member_ranks.copy_from_slice(&body[..GAME_RANK_UPDATE_BODY_LEN]);
        Ok(Self { member_ranks })
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
        assert_eq!(Ready::COMMAND, Request::Ready as u16);
        assert_eq!(StartGame::COMMAND, Request::StartGame as u16);
        assert_eq!(
            ConfirmMusicLoaded::COMMAND,
            Request::ConfirmMusicLoaded as u16
        );
        assert_eq!(UpdateGameStats::COMMAND, Request::UpdateGameStats as u16);
        assert_eq!(GameStatsUpdate::COMMAND, Event::GameStatsUpdate as u16);
        assert_eq!(SubmitScore::COMMAND, Request::SubmitScore as u16);
        assert_eq!(
            MemberReadyStateChanged::COMMAND,
            Event::UserReadyStateChanged as u16
        );
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
    fn command_ids_match_tally_answers() {
        use crate::command::Event;
        assert_eq!(ScoreSubmission::COMMAND, Event::ScoreSubmission as u16);
        assert_eq!(ScoreCompleted::COMMAND, Event::ScoreCompleted as u16);
        assert_eq!(GameRankUpdate::COMMAND, Event::GameRankUpdate as u16);
        assert_eq!(ScoreSubmission::COMMAND, 4017);
        assert_eq!(ScoreCompleted::COMMAND, 4018);
        assert_eq!(GameRankUpdate::COMMAND, 4040);
    }

    #[test]
    fn match_result_names_win_draw_lose() {
        assert_eq!(MatchResult::from_u8(0), Ok(MatchResult::Draw));
        assert_eq!(MatchResult::from_u8(1), Ok(MatchResult::Win));
        assert_eq!(MatchResult::from_u8(2), Ok(MatchResult::Lose));
        assert_eq!(
            MatchResult::from_u8(3),
            Err(Error::InvalidField {
                field: "match result",
                reason: "not 0, 1, or 2"
            })
        );
    }

    #[test]
    fn score_submission_round_trips_member_byte() {
        let submitted = ScoreSubmission { member_id: 2 };
        assert_eq!(submitted.encode_body(), vec![0x02]);
        let frame = decode_frame(&submitted.encode()).unwrap();
        assert_eq!(frame.id, 4017);
        assert_eq!(ScoreSubmission::decode_frame(&frame), Ok(submitted));
    }

    #[test]
    fn score_submission_rejects_empty_body() {
        assert_eq!(
            ScoreSubmission::decode_body(&[]),
            Err(Error::TruncatedMessage { need: 1, have: 0 })
        );
    }

    #[test]
    fn score_submission_rejects_trailing_bytes() {
        assert_eq!(
            ScoreSubmission::decode_body(&[0x00, 0x01]),
            Err(Error::TrailingBytes(1))
        );
    }

    fn live_score_entry() -> ScoreEntry {
        ScoreEntry {
            member_id: 0,
            active: true,
            cool: 10,
            good: 20,
            bad: 5,
            miss: 3,
            max_combo: 100,
            jam: 9,
            score: 123_456,
            reward: 213,
            level: 2,
            experience: 1911,
            result: MatchResult::Win,
            gem: 10133,
            cash_point: 0,
            speed: 1,
            long_note_score: 50,
        }
    }

    fn live_completed_body() -> Vec<u8> {
        vec![
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x14, 0x00,
            0x05, 0x00, 0x03, 0x00, 0x64, 0x00, 0x09, 0x00, 0x40, 0xe2, 0x01, 0x00, 0xd5, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x77, 0x07, 0x00, 0x00, 0x01, 0x95, 0x27, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x32, 0x00, 0x00, 0x00,
        ]
    }

    #[test]
    fn score_completed_decodes_live_capture() {
        // Solo submit against the local Memoryer Full-mode gateway:
        // count 1, master 0, one full row echoing the 4016 tally
        // (10/20/5/3, combo 100, jam 9, 123456, life survived) plus
        // server rewards and post-game levels.
        let body = live_completed_body();
        assert_eq!(body.len(), 50);
        let completed = ScoreCompleted::decode_body(&body).unwrap();
        assert_eq!(completed.room_master, 0);
        assert_eq!(completed.scores, vec![live_score_entry()]);
        assert_eq!(completed.encode_body(), body);
        let frame = decode_frame(&completed.encode()).unwrap();
        assert_eq!(frame.id, 4018);
        assert_eq!(ScoreCompleted::decode_frame(&frame), Ok(completed));
    }

    #[test]
    fn score_completed_rejects_negative_count() {
        assert_eq!(
            ScoreCompleted::decode_body(&[0xFF, 0xFF, 0xFF, 0xFF, 0x00]),
            Err(Error::InvalidField {
                field: "score count",
                reason: "negative entry count"
            })
        );
    }

    #[test]
    fn score_completed_rejects_overfull_count() {
        assert_eq!(
            ScoreCompleted::decode_body(&[0x09, 0x00, 0x00, 0x00, 0x00]),
            Err(Error::InvalidField {
                field: "score count",
                reason: "more than 8 entries"
            })
        );
    }

    #[test]
    fn score_completed_rejects_truncated_row() {
        let mut body = live_completed_body();
        body.pop();
        assert_eq!(
            ScoreCompleted::decode_body(&body),
            Err(Error::TruncatedMessage { need: 50, have: 49 })
        );
    }

    #[test]
    fn score_completed_rejects_trailing_bytes() {
        let mut body = live_completed_body();
        body.push(0x00);
        assert_eq!(
            ScoreCompleted::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn score_completed_rejects_bad_verdict() {
        let mut body = live_completed_body();
        body[5 + 31] = 0x09;
        assert_eq!(
            ScoreCompleted::decode_body(&body),
            Err(Error::InvalidField {
                field: "match result",
                reason: "not 0, 1, or 2"
            })
        );
    }

    #[test]
    fn score_completed_rejects_bad_active_flag() {
        let mut body = live_completed_body();
        body[5 + 1] = 0x09;
        assert_eq!(
            ScoreCompleted::decode_body(&body),
            Err(Error::InvalidField {
                field: "score active",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn game_rank_update_round_trips_leader_places() {
        let ranks = GameRankUpdate {
            member_ranks: [1, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        };
        assert_eq!(
            ranks.encode_body(),
            vec![0x01, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
        let frame = decode_frame(&ranks.encode()).unwrap();
        assert_eq!(frame.id, 4040);
        assert_eq!(GameRankUpdate::decode_frame(&frame), Ok(ranks));
    }

    #[test]
    fn game_rank_update_rejects_short_body() {
        assert_eq!(
            GameRankUpdate::decode_body(&[0x01; 7]),
            Err(Error::TruncatedMessage { need: 8, have: 7 })
        );
    }

    #[test]
    fn game_rank_update_rejects_trailing_bytes() {
        assert_eq!(
            GameRankUpdate::decode_body(&[0x01; 9]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn stat_kind_names_both_channels() {
        assert_eq!(StatKind::from_u16(0), Ok(StatKind::Life));
        assert_eq!(StatKind::from_u16(1), Ok(StatKind::Jam));
        assert_eq!(StatKind::Life.as_u16(), 0);
        assert_eq!(StatKind::Jam.as_u16(), 1);
    }

    #[test]
    fn stat_kind_rejects_unknown_channel() {
        assert_eq!(
            StatKind::from_u16(0x0301),
            Err(Error::InvalidField {
                field: "stat kind",
                reason: "not 0 or 1"
            })
        );
    }

    #[test]
    fn set_room_music_encodes_id_difficulty_speed() {
        let select = SetRoomMusic {
            music_id: 757,
            difficulty: 2,
            speed: 1,
        };
        assert_eq!(select.encode_body(), vec![0xF5, 0x02, 0x02, 0x01]);
        let frame = decode_frame(&select.encode()).unwrap();
        assert_eq!(frame.id, 4000);
        assert_eq!(SetRoomMusic::decode_frame(&frame), Ok(select));
    }

    #[test]
    fn set_room_music_rejects_speed_less_body() {
        assert_eq!(
            SetRoomMusic::decode_body(&[0xF5, 0x02, 0x02]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn set_room_arena_command_is_4002() {
        assert_eq!(SetRoomArena::COMMAND, 4002);
    }

    #[test]
    fn set_room_arena_encodes_stage_word() {
        let arena = SetRoomArena::new(7);
        assert_eq!(arena.encode_body(), vec![0x07, 0x00, 0x00, 0x00]);
        let frame = decode_frame(&arena.encode()).unwrap();
        assert_eq!(frame.id, 4002);
        assert_eq!(SetRoomArena::decode_frame(&frame), Ok(arena));
    }

    #[test]
    fn set_room_arena_round_trips_high_word_flag() {
        let arena = SetRoomArena::new(i32::MIN + 4);
        assert_eq!(arena.encode_body(), vec![0x04, 0x00, 0x00, 0x80]);
        let frame = decode_frame(&arena.encode()).unwrap();
        assert_eq!(SetRoomArena::decode_frame(&frame), Ok(arena));
    }

    #[test]
    fn set_room_arena_applies_to_store() {
        let mut store = 0;
        SetRoomArena::new(5).apply_to_store(&mut store);
        assert_eq!(store, 5);
    }

    #[test]
    fn set_room_arena_rejects_short_body() {
        assert_eq!(
            SetRoomArena::decode_body(&[0x07, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn set_room_arena_rejects_trailing_bytes() {
        let mut body = SetRoomArena::new(7).encode_body();
        body.push(0x00);
        assert_eq!(
            SetRoomArena::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn room_arena_echo_command_is_4003() {
        assert_eq!(RoomArenaEcho::COMMAND, 4003);
    }

    #[test]
    fn room_arena_echo_encodes_plain_stage() {
        let echo = RoomArenaEcho::new(7);
        assert_eq!(echo.encode_body(), vec![0x07, 0x00, 0x00, 0x00]);
        let frame = decode_frame(&echo.encode()).unwrap();
        assert_eq!(frame.id, 4003);
        assert_eq!(RoomArenaEcho::decode_frame(&frame), Ok(echo));
    }

    #[test]
    fn room_arena_echo_round_trips_high_word_flag() {
        let echo = RoomArenaEcho::new(i32::MIN + 4);
        assert_eq!(echo.encode_body(), vec![0x04, 0x00, 0x00, 0x80]);
        let frame = decode_frame(&echo.encode()).unwrap();
        assert_eq!(RoomArenaEcho::decode_frame(&frame), Ok(echo));
    }

    #[test]
    fn room_arena_echo_shares_the_request_word() {
        for word in [1, 7, 36, i32::MIN + 4] {
            assert_eq!(
                RoomArenaEcho::new(word).encode_body(),
                SetRoomArena::new(word).encode_body()
            );
        }
    }

    #[test]
    fn room_arena_echo_applies_to_store() {
        let mut store = 0;
        RoomArenaEcho::new(9).apply_to_store(&mut store);
        assert_eq!(store, 9);
    }

    #[test]
    fn room_arena_echo_rejects_short_body() {
        assert_eq!(
            RoomArenaEcho::decode_body(&[0x07, 0x00, 0x00]),
            Err(Error::TruncatedMessage { need: 4, have: 3 })
        );
    }

    #[test]
    fn room_arena_echo_rejects_trailing_bytes() {
        let mut body = RoomArenaEcho::new(7).encode_body();
        body.push(0x00);
        assert_eq!(
            RoomArenaEcho::decode_body(&body),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn room_arena_echo_rejects_foreign_frame() {
        let request_frame = decode_frame(&SetRoomArena::new(7).encode()).unwrap();
        assert_eq!(
            RoomArenaEcho::decode_frame(&request_frame),
            Err(Error::UnknownCommand(4002))
        );
        let echo_frame = decode_frame(&RoomArenaEcho::new(7).encode()).unwrap();
        assert_eq!(
            SetRoomArena::decode_frame(&echo_frame),
            Err(Error::UnknownCommand(4003))
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
                0x01, 0x00, 0x34, 0x12, 0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06, 0x05, 0x0C, 0x0B,
                0x0A, 0x09,
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
            0x02, 0x00, 0x00, 0xE8, 0x03, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
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
    fn game_stats_update_decodes_live_capture() {
        // Solo echo against local Memoryer (score 210, jam 3): the
        // leader plus empty places, `0xff`-padded.
        let solo = decode_frame(&[
            0x11, 0x00, 0xaf, 0x0f, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff,
        ])
        .unwrap();
        assert_eq!(solo.id, 4015);
        let update = GameStatsUpdate::decode_frame(&solo).unwrap();
        assert_eq!(update.member_id, 0);
        assert_eq!(update.kind(), Some(StatKind::Jam));
        assert_eq!(update.value, 3);
        assert_eq!(
            update.ranks,
            [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );

        // Duo lead flip: A leads at 9000, then B's 9500 retakes it.
        let leading = decode_frame(&[
            0x11, 0x00, 0xaf, 0x0f, 0x00, 0x01, 0x00, 0x07, 0x00, 0x00, 0x01, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff,
        ])
        .unwrap();
        let flipped = decode_frame(&[
            0x11, 0x00, 0xaf, 0x0f, 0x01, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff,
        ])
        .unwrap();
        let before = GameStatsUpdate::decode_frame(&leading).unwrap();
        let after = GameStatsUpdate::decode_frame(&flipped).unwrap();
        assert_eq!(
            before.rank_packet(),
            [0, 1, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
        assert_eq!(
            after.rank_packet(),
            [1, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
        assert_eq!(after.stats_channel(), Some((1, StatKind::Jam, 4)));
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
            GameStatsUpdate::decode_body(&[0x01; 12]),
            Err(Error::TruncatedMessage { need: 13, have: 12 })
        );
    }

    #[test]
    fn stat_ticks_name_their_channel() {
        let life = UpdateGameStats::life_tick(1000, 7, 2210, 0);
        assert_eq!(life.kind, StatKind::Life);
        assert_eq!(life.value, 1000);
        assert_eq!(life.sequence, 7);
        let jam = UpdateGameStats::jam_tick(3, 1, 210, 0);
        assert_eq!(jam.kind, StatKind::Jam);
        let frame = decode_frame(&jam.encode()).unwrap();
        assert_eq!(frame.id, 4014);
        assert_eq!(UpdateGameStats::decode_frame(&frame), Ok(jam));
    }

    #[test]
    fn stats_channel_types_the_peer_mirror_write() {
        let update = GameStatsUpdate {
            member_id: 2,
            kind_code: StatKind::Jam.as_u16(),
            value: 42,
            ranks: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        assert_eq!(update.stats_channel(), Some((2, StatKind::Jam, 42)));
        assert_eq!(update.rank_packet(), [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn stats_channel_stays_empty_on_unknown_kind() {
        let update = GameStatsUpdate {
            member_id: 1,
            kind_code: 9,
            value: 7,
            ranks: [0; 8],
        };
        assert_eq!(update.stats_channel(), None);
        assert_eq!(update.rank_packet(), [0; 8]);
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
        let state = SyncMemberMusicState { member_id: 2 };
        assert_eq!(state.encode_body(), vec![0x02]);
        let frame = decode_frame(&state.encode()).unwrap();
        assert_eq!(frame.id, 4036);
        assert_eq!(SyncMemberMusicState::decode_frame(&frame), Ok(state));
    }

    #[test]
    fn music_list_answer_round_trips() {
        let answer = MusicListAnswer {
            entries: vec![
                MusicListEntry {
                    id: 757,
                    notes_ex: 100,
                    notes_nx: 200,
                    notes_hx: 300,
                },
                MusicListEntry {
                    id: 758,
                    notes_ex: 0,
                    notes_nx: 0,
                    notes_hx: 0,
                },
            ],
            free_music: true,
        };
        let frame = decode_frame(&answer.encode()).unwrap();
        assert_eq!(frame.id, 4031);
        assert_eq!(MusicListAnswer::decode_frame(&frame), Ok(answer));
    }

    #[test]
    fn music_list_answer_reads_count_then_flag() {
        let body = vec![
            0x01, 0x00, 0xF5, 0x02, 0x64, 0x00, 0xC8, 0x00, 0x2C, 0x01, 0x01,
        ];
        let answer = MusicListAnswer::decode_body(&body).unwrap();
        assert_eq!(answer.entries.len(), 1);
        assert_eq!(answer.entries[0].id, 757);
        assert_eq!(answer.entries[0].notes_hx, 300);
        assert!(answer.free_music);
    }

    #[test]
    fn music_list_answer_rejects_negative_count() {
        assert_eq!(
            MusicListAnswer::decode_body(&[0xFF, 0xFF]),
            Err(Error::InvalidField {
                field: "music list count",
                reason: "negative entry count"
            })
        );
    }

    #[test]
    fn music_list_answer_rejects_truncated_row() {
        assert_eq!(
            MusicListAnswer::decode_body(&[0x01, 0x00, 0xF5]),
            Err(Error::TruncatedMessage { need: 11, have: 3 })
        );
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

    #[test]
    fn ready_sends_an_empty_body() {
        assert!(Ready.encode_body().is_empty());
        assert_eq!(Ready.encode(), vec![0x04, 0x00, 0xA8, 0x0F]);
        let frame = decode_frame(&Ready.encode()).unwrap();
        assert_eq!(frame.id, 4008);
        assert_eq!(Ready::decode_frame(&frame), Ok(Ready));
    }

    #[test]
    fn ready_rejects_payload() {
        assert_eq!(Ready::decode_body(&[0x01]), Err(Error::TrailingBytes(1)));
    }

    #[test]
    fn member_ready_state_changed_encodes_member_then_flag() {
        let ready = MemberReadyStateChanged {
            member_id: 1,
            ready: true,
        };
        assert_eq!(ready.encode_body(), vec![0x01, 0x01]);
        let frame = decode_frame(&ready.encode()).unwrap();
        assert_eq!(frame.id, 4009);
        assert_eq!(frame.id, Event::UserReadyStateChanged as u16);
        assert_eq!(MemberReadyStateChanged::decode_frame(&frame), Ok(ready));
    }

    #[test]
    fn member_ready_state_changed_decodes_cleared_flag() {
        let cleared = MemberReadyStateChanged::decode_body(&[0x02, 0x00]).unwrap();
        assert_eq!(cleared.member_id, 2);
        assert!(!cleared.ready);
    }

    #[test]
    fn member_ready_state_changed_rejects_short_body() {
        assert_eq!(
            MemberReadyStateChanged::decode_body(&[0x01]),
            Err(Error::TruncatedMessage { need: 2, have: 1 })
        );
    }

    #[test]
    fn member_ready_state_changed_rejects_trailing_bytes() {
        assert_eq!(
            MemberReadyStateChanged::decode_body(&[0x01, 0x01, 0x00]),
            Err(Error::TrailingBytes(1))
        );
    }

    #[test]
    fn member_ready_state_changed_rejects_wide_member() {
        assert_eq!(
            MemberReadyStateChanged::decode_body(&[0x08, 0x01]),
            Err(Error::InvalidField {
                field: "ready member",
                reason: "not below 8"
            })
        );
    }

    #[test]
    fn member_ready_state_changed_rejects_bad_flag() {
        assert_eq!(
            MemberReadyStateChanged::decode_body(&[0x01, 0x02]),
            Err(Error::InvalidField {
                field: "ready flag",
                reason: "not 0 or 1"
            })
        );
    }
}
