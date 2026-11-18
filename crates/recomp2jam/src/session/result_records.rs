//! Result member records staged from the 4018 ScoreCompleted table.
//!
//! The 7.55 result row painter reads fixed words from one
//! `0x2190`-byte member record per seat (`docs/cresult-anchors.md`,
//! `FUN_004c3b40` at file `0xc3b40`). `docs/net-protocol.md` §4c pins the
//! write side: the 4018 row reader (VA `0x5411a0`, file `0x1411a0`) stores
//! the wire fields into those same offsets, and the 4018 head (VA
//! `0x55fbe0`, file `0x15fbe0`) writes the room-master marker. This module
//! models those words directly, so each field carries its exe offset in
//! place of an invented layout. Evidence IDs: `ev_476a9085…`,
//! `ev_e5954df4…`, `ev_cbd5ca48…`, `ev_6902875a…`, `ev_5308dcbd…`,
//! `ev_d19ae823…`. Version pin: 7.55, 5,029,888 B, sha256 `83a04541…72c823b`.

use o2jam_net::play::{MatchResult, ScoreEntry};

/// One seat's result record words, named after their exe offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultMemberRecord {
    /// COOL count — record `+0x2142`.
    pub cool: u16,
    /// GOOD count — record `+0x2144`.
    pub good: u16,
    /// BAD count — record `+0x2146`.
    pub bad: u16,
    /// MISS count — record `+0x2148`.
    pub miss: u16,
    /// Max combo — record `+0x215c`.
    pub max_combo: u16,
    /// Jam combo — record `+0x214c`.
    pub jam_combo: u16,
    /// Long-note word — record `+0x2158` (wire `u32`, decoded `i32`).
    pub long_note_score: i32,
    /// Score — record `+0x2160`.
    pub score: u32,
    /// Reward — record `+0x2164`.
    pub reward: u16,
    /// Draw/win/lose byte — record `+0x2166`.
    pub result: MatchResult,
    /// Scroll speed — record `+0x2168`.
    pub speed: u8,
    /// Level — record `+0x20`.
    pub level: i32,
    /// Experience — record `+0x24`.
    pub experience: i32,
    /// Gem — record `+0x2c`.
    pub gem: i32,
    /// Cash point — record `+0x2128`.
    pub cash_point: i32,
    /// Room-master flag — record `+0x30` (head writer, not the row reader).
    pub room_master: bool,
}

impl ResultMemberRecord {
    /// Stage one admitted 4018 row into the exe-pinned words.
    ///
    /// Wire order and destinations come from the row reader (VA
    /// `0x5411a0`, file `0x1411a0`); `room_master` comes from the head's
    /// `+0x30` writer (files `0x15fc66`/`0x15fcc8`/`0x15fd11`). No field
    /// is inferred from a name or borrowed from a neighbouring record.
    #[must_use]
    pub fn from_score_entry(entry: &ScoreEntry, room_master: u8) -> Self {
        Self {
            cool: entry.cool,
            good: entry.good,
            bad: entry.bad,
            miss: entry.miss,
            max_combo: entry.max_combo,
            jam_combo: entry.jam,
            long_note_score: entry.long_note_score,
            score: entry.score,
            reward: entry.reward,
            result: entry.result,
            speed: entry.speed,
            level: entry.level,
            experience: entry.experience,
            gem: entry.gem,
            cash_point: entry.cash_point,
            room_master: entry.member_id == room_master,
        }
    }
}
