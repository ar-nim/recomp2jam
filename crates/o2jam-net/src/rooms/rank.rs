//! Ranking messages: 2027/2028 play ranking, 2029 rank
//! notification, 2030/2031 score list, and 2032/2033 max score.

use crate::error::Error;
use crate::message::WireMessage;
use crate::strings::{decode_null_terminated, encode_null_terminated, MAX_STRING_LEN};
/// `RankNotification` 2029 event: a rank-up notice for the lobby
/// displays. Memoryer (`Messages/Events/MainRoom/
/// `RankNotificationEventData.cs`): bool `Invalid` as little-endian
/// `i32` (order 0, nonzero reads true), `i32` music id (order 1),
/// string nickname (order 2), `i32` ranking (order 4 — order 3 was
/// dropped upstream, so the wire order is invalid, music, name,
/// rank). Exe receive: dispatch index `0x1a` (2029) lands case file
/// `0x158556`, which calls handler VA `0x55b4d0` (file `0x15b4d0`).
/// The handler reads the invalid word through the packet-reader
/// helper (`0x555d20`) and drops the packet when nonzero (`cmp` at
/// file `0x15b527`); else it stages music id, nickname, and ranking
/// through `.data` slots `0x77969c`/`0x7796a8`/`0x7796a0` and forwards
/// `(0, 0, 0x7ed, 0x12)` into the ControlList UI queue (`0x478d50`,
/// file `0x15b575`). Observed with GNU objdump 2.47 slices plus
/// Python byte scans over the read-only 7.55 copy; no new REA dossier
/// filed — the field order is Memoryer corroboration against an
/// exe-verified read sequence. (2015 C→S has no client handler: the
/// same table maps index `0x0c` to the default case file `0x158937`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankNotification {
    /// True when the notice carries no rank (client drops it).
    pub invalid: bool,
    /// Ranked song id.
    pub music_id: i32,
    /// Ranked player nickname.
    pub nickname: String,
    /// New rank value.
    pub ranking: i32,
}

impl WireMessage for RankNotification {
    const COMMAND: u16 = 2029;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + 4 + self.nickname.len() + 1 + 4);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.extend_from_slice(&self.music_id.to_le_bytes());
        body.extend_from_slice(&encode_null_terminated(&self.nickname, MAX_STRING_LEN));
        body.extend_from_slice(&self.ranking.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 8 {
            return Err(Error::TruncatedMessage {
                need: 8,
                have: body.len(),
            });
        }
        let invalid = i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0;
        let music_id = i32::from_le_bytes([body[4], body[5], body[6], body[7]]);
        let (nickname, used) = decode_null_terminated(&body[8..], MAX_STRING_LEN)?;
        let tail = &body[8 + used..];
        if tail.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 8 + used + 4,
                have: body.len(),
            });
        }
        if tail.len() > 4 {
            return Err(Error::TrailingBytes(tail.len() - 4));
        }
        let ranking = i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        Ok(Self {
            invalid,
            music_id,
            nickname,
            ranking,
        })
    }
}

/// Rank movement arrow on a `MusicPlayRankingAnswer` row
/// (`Mozart.Metadata.RankDeltaType`, byte-backed: `Up = 0x00`,
/// `Down = 0x01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RankDelta {
    /// Rank improved (or unchanged) since last time.
    Up = 0,
    /// Rank fell.
    Down = 1,
}

impl RankDelta {
    /// Interpret a raw delta byte. `None` means unknown.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Up),
            1 => Some(Self::Down),
            _ => None,
        }
    }
}

/// One row of the `MusicPlayRankingAnswer` 2028 answer: a ranked
/// player's lanking line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankEntry {
    /// Leaderboard place (`0` marks a blank row: the exe pushes a
    /// zeroed row and reads no further fields for it).
    pub rank: i32,
    /// Ranked player nickname (UTF-8, null-terminated on the wire).
    pub nickname: String,
    /// Battles fought.
    pub battles: i32,
    /// Battles won.
    pub wins: i32,
    /// Win rate (Memoryer computes `win / battle`, truncated).
    pub win_rate: i32,
    /// Rank movement since last time.
    pub delta: RankDelta,
    /// Rank movement magnitude.
    pub rank_delta: i32,
}

impl Default for RankEntry {
    fn default() -> Self {
        Self {
            rank: 0,
            nickname: String::new(),
            battles: 0,
            wins: 0,
            win_rate: 0,
            delta: RankDelta::Up,
            rank_delta: 0,
        }
    }
}

/// Wire width of one [`RankEntry`] past a nonzero rank word:
/// null-terminated nickname + three `i32` + `u8` delta + `i32`.
fn encode_rank_entry(entry: &RankEntry) -> Vec<u8> {
    let mut body = entry.rank.to_le_bytes().to_vec();
    if entry.rank != 0 {
        body.extend_from_slice(&encode_null_terminated(&entry.nickname, MAX_STRING_LEN));
        body.extend_from_slice(&entry.battles.to_le_bytes());
        body.extend_from_slice(&entry.wins.to_le_bytes());
        body.extend_from_slice(&entry.win_rate.to_le_bytes());
        body.push(entry.delta as u8);
        body.extend_from_slice(&entry.rank_delta.to_le_bytes());
    }
    body
}

/// Cursor over a 2028 body; every read bounds-checks. Returns the
/// entry plus bytes consumed (`rank == 0` rows carry no further
/// fields, matching the exe's zeroed-row arm).
fn decode_rank_entry(body: &[u8]) -> Result<(RankEntry, usize), Error> {
    if body.len() < 4 {
        return Err(Error::TruncatedMessage {
            need: 4,
            have: body.len(),
        });
    }
    let rank = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
    if rank == 0 {
        return Ok((RankEntry::default(), 4));
    }
    let (nickname, name_used) = decode_null_terminated(&body[4..], MAX_STRING_LEN)?;
    let tail = &body[4 + name_used..];
    if tail.len() < 13 {
        return Err(Error::TruncatedMessage {
            need: 4 + name_used + 13,
            have: body.len(),
        });
    }
    let delta = RankDelta::from_code(tail[12]).ok_or(Error::InvalidField {
        field: "rank delta type",
        reason: "not 0 (Up) or 1 (Down)",
    })?;
    Ok((
        RankEntry {
            rank,
            nickname,
            battles: i32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]),
            wins: i32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]),
            win_rate: i32::from_le_bytes([tail[8], tail[9], tail[10], tail[11]]),
            delta,
            rank_delta: i32::from_le_bytes([tail[13], tail[14], tail[15], tail[16]]),
        },
        4 + name_used + 17,
    ))
}

/// `GetMusicPlayRanking` 2027 request: fetch the lanking board.
/// Empty body: the exe never sends this opcode (no `0x7eb` push or
/// dword anywhere in `OTwo.exe`, `O2Net32.dll`, `Player.dll`, or
/// `SendWebData.dll`), and Memoryer's `RankingController` handler
/// reads no request fields — the client fetch carries nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetMusicPlayRanking;

impl WireMessage for GetMusicPlayRanking {
    const COMMAND: u16 = 2027;

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

/// `MusicPlayRankingAnswer` 2028 response: the lanking board.
///
/// Exe-verified handler VA `0x55b230` (file `0x15b230`, REA
/// `ev_7f09b7fa…`): one `i32` rank word, then — only when the word is
/// nonzero — a nickname string through the string-reader helper
/// (`0x555ea0`) plus three `i32` reads, one `u8` read, and one `i32`
/// read through the packet-reader helper (`0x555d20`; pushes at files
/// `0x15b302`/`0x15b30e`/`0x15b31a`/`0x15b326`/`0x15b335`), each row
/// staged into the rank-row vector at `0x77968c`; then an `i32` count
/// word, and — only when positive — that many rows in the same shape
/// (zero/negative counts paint no rows; a zero count paints ten blank
/// rows, a display fill the decoder reports as an empty list).
/// Dispatcher: base `0x7d3` index `0x19` holds entry `0x558545`
/// (address table file `0x158940`), whose stub relative-calls the
/// handler. Memoryer (`Messages/Responses/Ranking/
/// `MusicPlayRankingResponse.cs`): rank `int`, nickname string,
/// battles/wins/win-rate `int`s, `RankDeltaType` byte, rank-delta
/// `int`, with a leading self entry plus an `i32`-counted list — the
/// field order corroborates the exe read sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicPlayRankingAnswer {
    /// The viewer's own line (default when its rank word is zero).
    pub self_entry: RankEntry,
    /// Board lines in wire order.
    pub entries: Vec<RankEntry>,
}

impl WireMessage for MusicPlayRankingAnswer {
    const COMMAND: u16 = 2028;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = encode_rank_entry(&self.self_entry);
        let count = i32::try_from(self.entries.len()).unwrap_or(i32::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for entry in &self.entries {
            body.extend_from_slice(&encode_rank_entry(entry));
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        let (self_entry, mut at) = decode_rank_entry(body)?;
        let rest = body.get(at..).ok_or(Error::TruncatedMessage {
            need: at.saturating_add(4),
            have: body.len(),
        })?;
        if rest.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: at + 4,
                have: body.len(),
            });
        }
        let count = i32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
        at += 4;
        let mut entries = Vec::new();
        if count > 0 {
            let count = usize::try_from(count).unwrap_or(usize::MAX);
            entries.reserve(count.min(1024));
            for _ in 0..count {
                let (entry, used) = decode_rank_entry(&body[at..])?;
                entries.push(entry);
                at += used;
            }
        }
        Ok(Self {
            self_entry,
            entries,
        })
    }
}

/// `GetMusicScoreList` 2030 request: fetch per-song best ranks.
/// Empty body: both exe send sites (`push 0; push 0x7ee`, files
/// `0xac427` and `0xb2644`, indirect `+0x5c` send calls) stage no
/// fields — the first gates on the menu-state word at `0x77ee86`
/// being `2` (load + compare files `0xac402`/`0xac417`), the second on
/// the seat byte at `0x77cd30` being nonzero (load file `0xb2622`),
/// both additionally gated on dword `0x779964` being zero (compares
/// files `0xac41c`/`0xb2639`). Memoryer's `RankingController`
/// handler reads no request fields either. The gate globals stay
/// OPEN (no labels pinned).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetMusicScoreList;

impl WireMessage for GetMusicScoreList {
    const COMMAND: u16 = 2030;

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

/// One song's best ranks in the `MusicScoreListAnswer` 2031 answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicScoreRow {
    /// Song id (`u16`, as selected per room).
    pub music_id: u16,
    /// Best rank marks per difficulty in `Difficulty` order
    /// (EX/NX/HX/RX; `Mozart.Metadata.Rank`: 0 none, 1 F … 7 S,
    /// 8 Perfect).
    pub difficulty_ranks: [u8; 4],
}

/// Wire size of one [`MusicScoreRow`]: `u16` id + four rank bytes.
const MUSIC_SCORE_ROW_LEN: usize = 6;

/// `MusicScoreListAnswer` 2031 response: per-song best ranks.
///
/// Exe-verified handler VA `0x55b590` (file `0x15b590`, REA
/// `ev_299779a2…`): one `i32` result word — negative results skip
/// the rows and still forward — else a `u16` row count, then that many
/// 6-byte rows (`push 6` at file `0x15b5fa`, call `0x15b603`) pushed
/// into the per-song/difficulty best table at `0x779814` through
/// `FUN_004c5700` (which divides the vector span by 6), then forward
/// (`0x12`, 2031, 0, 0). So count and row width are exe-pinned; the
/// row split (`u16` music id + four rank bytes) is Memoryer shape
/// (`Messages/Responses/Ranking/MusicScoreEntry`: `ushort MusicId`
/// plus a `Difficulty → Rank` map, both byte-backed, with an
/// `Empty`-prefix collection = no length word, exactly four inline
/// values in `Enum.GetValues<Difficulty>()` order) against an
/// exe-verified width. The paint-path read that would pin the
/// per-byte difficulty order stays OPEN (next step: the result-screen
/// lookup over `0x779814`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicScoreListAnswer {
    /// Server result word (negative = no rows, still refresh).
    pub result: i32,
    /// Best-rank rows in wire order.
    pub rows: Vec<MusicScoreRow>,
}

impl WireMessage for MusicScoreListAnswer {
    const COMMAND: u16 = 2031;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + 2 + self.rows.len() * MUSIC_SCORE_ROW_LEN);
        body.extend_from_slice(&self.result.to_le_bytes());
        let count = u16::try_from(self.rows.len()).unwrap_or(u16::MAX);
        body.extend_from_slice(&count.to_le_bytes());
        for row in &self.rows {
            body.extend_from_slice(&row.music_id.to_le_bytes());
            body.extend_from_slice(&row.difficulty_ranks);
        }
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < 4 {
            return Err(Error::TruncatedMessage {
                need: 4,
                have: body.len(),
            });
        }
        let result = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        if result < 0 {
            return Ok(Self {
                result,
                rows: Vec::new(),
            });
        }
        if body.len() < 6 {
            return Err(Error::TruncatedMessage {
                need: 6,
                have: body.len(),
            });
        }
        let count = usize::from(u16::from_le_bytes([body[4], body[5]]));
        let need = 6 + count * MUSIC_SCORE_ROW_LEN;
        if body.len() < need {
            return Err(Error::TruncatedMessage {
                need,
                have: body.len(),
            });
        }
        let mut rows = Vec::with_capacity(count);
        for index in 0..count {
            let at = 6 + index * MUSIC_SCORE_ROW_LEN;
            rows.push(MusicScoreRow {
                music_id: u16::from_le_bytes([body[at], body[at + 1]]),
                difficulty_ranks: [body[at + 2], body[at + 3], body[at + 4], body[at + 5]],
            });
        }
        Ok(Self { result, rows })
    }
}

/// `GetMusicMaxScore` 2032 request: fetch one song's best score.
///
/// The exe send (file `0x4151c`, in `FUN_0043e9b0`, REA
/// `ev_2e7c7bc9…`) pushes only the opcode (`push 0; push 0x7f0`,
/// indirect `+0x64` slot call) once the entry passes the music-list
/// membership predicate — no body bytes are staged at the site. The
/// two parameter words sit in adjacent globals by then: the selected
/// music id at `0x7797c0` (store file `0x41472`) and its note-count
/// word at `0x7797c2` (store file `0x414f2`). Memoryer
/// (`Messages/Requests/Ranking/MusicMaxScoreRequest.cs`) decodes
/// `ushort MusicId` + `ushort NoteCount`, which matches those two
/// staged words — so the request encodes them, but the wire transfer
/// of the globals through the `+0x64` send slot is unobserved and
/// stays OPEN (next step: trace the `+0x64` slot helper's body
/// mechanism, e.g. at the Authorize site file `0x74529`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetMusicMaxScore {
    /// Song id to look up.
    pub music_id: u16,
    /// Note count selecting the difficulty (EX/NX/HX match).
    pub note_count: u16,
}

/// Body length of [`GetMusicMaxScore`]: `u16` + `u16`.
const GET_MUSIC_MAX_SCORE_BODY_LEN: usize = 4;

impl WireMessage for GetMusicMaxScore {
    const COMMAND: u16 = 2032;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(GET_MUSIC_MAX_SCORE_BODY_LEN);
        body.extend_from_slice(&self.music_id.to_le_bytes());
        body.extend_from_slice(&self.note_count.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < GET_MUSIC_MAX_SCORE_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: GET_MUSIC_MAX_SCORE_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > GET_MUSIC_MAX_SCORE_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - GET_MUSIC_MAX_SCORE_BODY_LEN,
            ));
        }
        Ok(Self {
            music_id: u16::from_le_bytes([body[0], body[1]]),
            note_count: u16::from_le_bytes([body[2], body[3]]),
        })
    }
}

/// `MusicMaxScoreAnswer` 2033 response: one song's best score.
///
/// Exe-verified handler VA `0x55b640` (file `0x15b640`, REA
/// `ev_45b5f508…`): one `i32` invalid word — nonzero skips the
/// packet — else one `u32` max-score read straight into `0x7797c8`
/// (push file `0x15b668`, call `0x15b672`), then the dialog-7 gate:
/// forward (`0x11`, 2033, 0, 0) only while the ControlList parent
/// (`0x762b44`) shows dialog kind 7. Dispatcher index `0x1e` holds
/// entry `0x558578`, whose stub relative-calls the handler. Memoryer
/// (`Messages/Responses/Ranking/MusicMaxScoreResponse.cs`): bool
/// `Invalid` as `i32` + `u32 MaxScore` — corroborates the two-word
/// shape against the exe-verified read sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicMaxScoreAnswer {
    /// True when the score carries no value (client drops it).
    pub invalid: bool,
    /// Best score on the song.
    pub max_score: u32,
}

/// Body length of [`MusicMaxScoreAnswer`]: `i32` + `u32`.
const MUSIC_MAX_SCORE_ANSWER_BODY_LEN: usize = 8;

impl WireMessage for MusicMaxScoreAnswer {
    const COMMAND: u16 = 2033;

    fn encode_body(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(MUSIC_MAX_SCORE_ANSWER_BODY_LEN);
        body.extend_from_slice(&i32::from(self.invalid).to_le_bytes());
        body.extend_from_slice(&self.max_score.to_le_bytes());
        body
    }

    fn decode_body(body: &[u8]) -> Result<Self, Error> {
        if body.len() < MUSIC_MAX_SCORE_ANSWER_BODY_LEN {
            return Err(Error::TruncatedMessage {
                need: MUSIC_MAX_SCORE_ANSWER_BODY_LEN,
                have: body.len(),
            });
        }
        if body.len() > MUSIC_MAX_SCORE_ANSWER_BODY_LEN {
            return Err(Error::TrailingBytes(
                body.len() - MUSIC_MAX_SCORE_ANSWER_BODY_LEN,
            ));
        }
        Ok(Self {
            invalid: i32::from_le_bytes([body[0], body[1], body[2], body[3]]) != 0,
            max_score: u32::from_le_bytes([body[4], body[5], body[6], body[7]]),
        })
    }
}

