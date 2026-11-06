//! The `OJNList.dat` middle region: song-group, song-class, param,
//! triple, and song-wheel bucket tables plus the select-dialog helpers.
//!
//! Verified on real 7.55 files (docs/file-formats/ojn.md).

use crate::{read_u32le, unknown_format, FileError};

/// Byte length of the `OJNList.dat` middle u32 region
/// (`[168600, 195616)`, between the song records and the patch index).
/// Layout: 8-byte header, table 1, table 2, table 3, table 4, table 5,
/// 40-byte footer — the parts below add up exactly
/// (`8 + 3264 + 11244 + 6516 + 1440 + 4504 + 40`; docs/file-formats/ojn.md).
pub const OJN_MID_REGION_LEN: usize = 27016;

/// Magic heading the middle region (first header word on the real file).
pub const OJN_MID_MAGIC: u32 = 63540;

/// Entries in middle table 1 (second header word on the real file).
pub const OJN_MID_TABLE1_COUNT: usize = 204;

/// Bytes per middle-table-1 entry (`song_id, 0, param, 0`).
pub const OJN_MID_TABLE1_ENTRY_LEN: usize = 16;

/// Song slots in middle tables 2 and 5 (and the footer echo).
pub const OJN_MID_SONG_COUNT: usize = 562;

/// Newest song id (middle-table-2/5 head id and footer echo).
pub const OJN_MID_TOP_SONG_ID: u32 = 887;

/// Bytes per middle-table-2 entry (`song_id, listing, 0, param_b, 0`).
pub const OJN_MID_TABLE2_ENTRY_LEN: usize = 20;

/// Entries in middle table 3.
pub const OJN_MID_TABLE3_COUNT: usize = 407;

/// Bytes per middle-table-3 entry (`song_id, param_a, param_b, anchor`).
pub const OJN_MID_TABLE3_ENTRY_LEN: usize = 16;

/// Constant closing every middle-table-3 entry on the real file
/// (`0x77D2E4B7`, heap-address range — residue candidate, semantics OPEN).
pub const OJN_MID_TABLE3_ANCHOR: u32 = 2010252407;

/// Triples in middle table 4.
pub const OJN_MID_TABLE4_COUNT: usize = 120;

/// Bytes per middle-table-4 triple.
pub const OJN_MID_TABLE4_ENTRY_LEN: usize = 12;

/// Marker heading middle table 5 (word before its song count).
pub const OJN_MID_TABLE5_HEAD: u32 = 1;

/// Bytes per middle-table-5 entry (`song_id, category`).
pub const OJN_MID_TABLE5_ENTRY_LEN: usize = 8;

/// Parallel song-wheel lists the client fans `category` out into
/// (global heads `0x77953c..0x77957c`; file `0xbfa5c` dispatch).
/// The STEP2 label filter tests the same 1..=5 domain on inline
/// select-entry `+0x4`, but the wheel pointer vectors and the
/// `+0x650`/`+0x670`/`+0x690` inline `0x238` result lists share no
/// storage — only the bucket value flows across (see `ojn_wheel_slot`).
pub const OJN_WHEEL_LIST_COUNT: usize = 5;

/// Bytes of the middle-region footer (closes with the
/// `OJN_MID_SONG_COUNT`/`OJN_MID_TOP_SONG_ID` echo).
pub const OJN_MID_FOOTER_LEN: usize = 40;

/// One middle-table-1 entry: song id plus a group word.
///
/// The real 7.55 file holds 204 entries: group 1 covers newer ids
/// (887..774, 108 rows), group 2 older ids (506..100, 96 rows). The
/// loader files the three trailing words into song-record slots
/// `+0x144/+0x148/+0x14c` (stride `0x1b4`; stores at file offsets
/// `0x7a390`/`0x7a3ed`/`0x7a44a`). A full-binary objdump survey finds
/// no song-stride (`imul 0x1b4`) read of these slots — the only indexed
/// accesses are the three stores, and same-displacement hits elsewhere
/// use other strides on unrelated structs — so the group word is
/// write-only through the song vector and its meaning stays OPEN.
/// Re-surveyed this pass: song-stride accesses outside the loader touch
/// only `+0x134` / `+0x150` (table-3 `param_a` gate) / `+0x15c` /
/// `+0x17c..0x180` / `+0x194` (table-5 bucket); direct-displacement
/// `0x144/0x148/0x14c` hits (e.g. file `0x221f5`) sit in methods with
/// no direct caller in the image, so their base is unattributed —
/// no consumer site, OPEN stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidGroupEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// Group word (1 or 2 on the real file).
    pub group: u32,
}

/// One middle-table-2 entry: song id plus the selection class and a
/// second tag word.
///
/// The real 7.55 file holds 562 entries (ids 887..100, strictly
/// descending — the reversed song-record order): `listing` is 1
/// (545x) or 2 (17x: ids 385..395, 426, 427, 693..700, all absent
/// from table 3), `param_b` is 0 (512x) or 1 (50x).
///
/// The loader files the four words into song-record slots
/// `+0x16c/+0x170/+0x174/+0x178` (stride `0x1b4`; stores at file
/// offsets `0x7a5c9`/`0x7a629`/`0x7a689`/`0x7a6e9`). Exe consumers pin
/// `listing` down: one song filter branches on `listing == 1` versus
/// the rest (file offset `0x3706f`), and several passes skip songs
/// whose word reads zero (e.g. file offset `0xadc68`). The non-1 arm
/// additionally requires `+0x178 != 0` (file offset `0x37261`), which
/// never holds on real data (both reserved words validate zero), so
/// class 2 never passes that selector. `param_b` has no calling read
/// site yet — OPEN. (Static disassembly survey; REA dossier TBD.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidSongEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// Selection class (1 = standard path, 2 = 17-song special class
    /// routed away from the `listing == 1` selector; see struct docs).
    pub listing: u32,
    /// Second tag word (0 or 1 on the real file).
    pub param_b: u32,
}

/// One middle-table-3 entry: song id plus two param words plus the
/// constant anchor.
///
/// The real 7.55 file holds 407 entries (ids 887..102, sparse,
/// strictly descending): `param_a` reads 0..5000 and `param_b`
/// 0..30000, always in round hundreds. The loader files the three
/// words into song-record slots `+0x150/+0x154/+0x158` (stride `0x1b4`;
/// stores at file offsets `0x7a84b`/`0x7a8ab`/`0x7a90b`). Exe sites gate
/// on `song+0x150 != 0` (e.g. file offsets `0xdd6b6`, `0xe54f0`,
/// `0xeca0c`, `0xecb38`): a nonzero `param_a` enables a per-song
/// feature call, while zero — all 155 row-less songs and rows with
/// `param_a == 0` such as song 887 — skips it. The gated feature's
/// meaning, `param_b`, and the anchor stay OPEN.
/// (Static disassembly survey; REA dossier TBD.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidParamEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// First param word.
    pub param_a: u32,
    /// Second param word.
    pub param_b: u32,
    /// Closing word (always `OJN_MID_TABLE3_ANCHOR`).
    pub anchor: u32,
}

/// One middle-table-4 triple.
///
/// The real 7.55 file holds 120 triples: triple 0 is the
/// `(120, 246, 2)` header-ish word set, triples 1..119 run in
/// 39/40/40 groups over one shared 40-song id set (mostly
/// `(0/1, id, 0/1/2)` shapes; the head triple's `b = 246` completes the
/// 40-set). The loader files each triple into song-record slots
/// `+0x184/+0x188/+0x18c` (stride `0x1b4`; stores at file offsets
/// `0x7aa6d`/`0x7aacd`/`0x7ab2d`), keyed by song-id match. A full-binary
/// objdump survey finds no song-stride (`imul 0x1b4`) read of these
/// slots — the only indexed accesses are the three stores, and
/// same-displacement hits elsewhere use `shl 4` (x16) strides on
/// unrelated structs — so all three words are write-only through the
/// song vector and their semantics stay OPEN. Re-surveyed this pass:
/// direct-displacement `0x184/0x188/0x18c` hits elsewhere are byte/word
/// accesses on smaller structs (e.g. `movb` at file `0x38e51`) with
/// unattributed bases — no consumer site, OPEN stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidTriple {
    /// First word.
    pub a: u32,
    /// Second word (a song id outside the head triple).
    pub b: u32,
    /// Third word.
    pub c: u32,
}

/// One middle-table-5 entry: song id plus the label bucket.
///
/// The real 7.55 file holds 562 entries (ids 887..100, strictly
/// descending — the same id set as table 2): `category` runs 1..=5
/// (78x1 GOLD, 82x2 BLACK, 157x3 BLUE, 52x4 RED, 193x5 SILVER;
/// see label order below). The loader stores the pair at song-record
/// `+0x190` (id echo) / `+0x194` (file offsets `0x7ac8f` / `0x7acef`,
/// VAs `0x47ac8f` / `0x47acef`).
///
/// Four exe sites jump-table on `song+0x194 - 1` (range-check `> 4`)
/// and append the song-record pointer into parallel wheel lists
/// (vectors of song pointers, stride `0x1b4` records): file `0xbfa5c`
/// (VA `0x4bfa5c`, table file `0xbff7c`) and VA `0x493409`
/// (file `0x93409`, table VA `0x493a3c`) append via `0x47dd60`
/// into the global heads (VAs `0x77953c..0x77957c` in category order),
/// file `0x16121e` (VA `0x56121e`, table file `0x162e2c`) via
/// `0x454b90`/`0x4552b0`, and VA `0x4cc82f` (file `0x8c82f`, table VA
/// `0x4cc964`) into dialog-relative pointer lists (`+0x1d0`/`+0x1e0`/
/// `+0x1f0` family). `category` is the song-wheel label bucket.
///
/// The select-music dialog filters inline `0x238`-byte entries (NOT the
/// wheel pointers) through chained result lists: master `+0x640`
/// → STEP1 level bands (`+0x212`, VA `0x446950`/file `0x46950`) →
/// `+0x650` → STEP2 labels (`+0x4`, VA `0x447050`/file `0x47050`,
/// 6-arm table VA `0x447718`/file `0x47718`, arms at files
/// `0x472c1`/`0x472fa`/`0x47333`/`0x47369`/`0x4739f` testing
/// `entry+0x4 == 1..=5`) → `+0x660` → STEP3 genre (`+0x214`, VA
/// `0x447730`/file `0x47730`) → `+0x670`. Copies use `0x452850`,
/// clears use `0x452a00`. The STEP2 handler (VA `0x43f7e3`/file
/// `0x3f7e3`) stores `code - 0x2720B021` at dialog `+0x6c0`.
///
/// The entry `+0x4` writer is pinned: the `0x4441f0` entry builder
/// (file `0x441f0`) stores `entry+0x4 = song+0x194` at VA `0x44422e`
/// (file `0x4422e`) from the by-value `0x1b4` song-record copy, then
/// appends the `0x238` entry via `0x452850` into dialog `+0x640` at
/// VA `0x4445cd` (file `0x445cd`) or dialog `+0x680` at VA `0x4445e7`
/// (file `0x445e7`).
///
/// Dialog `+0x680` is rebuilt from `+0x640` by two keyed filter
/// functions. The first (VA `0x444630`/file `0x44630`) clears `+0x680`
/// via `0x452a00` at file `0x446c0`, then fans matching `+0x640`
/// entries out through `0x452850` at files `0x44957` (name-keyed first
/// pass), `0x44a08` (index 0), `0x44abe` (index 2), `0x44b74`
/// (index 5), `0x44c2a` (index 6), `0x44ce6` (index 8), and `0x44f61`
/// (second loop). Guards are a byte-range check plus a 2-byte
/// `strncmp` against the key-name table (indexed `0x6454d8` plus fixed
/// `0x645510`/`0x645514`/`0x645518`/`0x64551c`/`0x645520`), under the
/// dialog-mode dispatch on `+0x188` (0/1/else). The fixed arms select
/// the doubled (`ssang`) initials — see [`ojn_search_doubled_initial`]
/// (arms 0/2/5/6/8 → `ㄲ/ㄸ/ㅃ/ㅆ/ㅉ`). The second
/// (VA `0x444fe0`/file `0x44fe0`, clear at file `0x45070`) appends at
/// files `0x4528b` and `0x454fa` with the same keyed shape.
/// (Static disassembly survey; REA dossier TBD.)
///
/// Dialog `+0x690` holds inline `0x238` entries, not retained
/// pointers: five `0x452850` appends in the song-entry builder copy
/// one entry each from a source address returned by `0x453780`
/// (files `0x4298d` / `0x429e3` / `0x42c05` / `0x42de8` / `0x42e13`;
/// docs/file-formats/ojn.md). The copy is synchronous, so the source
/// pointer lifetime ends at return. The `+0x690` count itself reads
/// `(end - begin) / 0x238` (e.g. file `0x42f8f`); the global `0x7797e0`
/// list appended at file `0x42f4f` (cleared at `0x42e63`) is a
/// different list. Two dialog `+0x690` clears use the same empty-guarded
/// count idiom (skip `0x452a00` when the span is empty) — see
/// [`ojn_inline_list_count`]: file `0x468b8` (VA `0x4468b8`, dialog
/// function `0x446630`) and file `0x4de57` (VA `0x44de57`, dialog
/// function `0x44d500`).
///
/// Wheel-versus-filter relation: the five wheel lists and the
/// `+0x650` / `+0x670` / `+0x690` result lists share no storage.
/// Wheel lists hold song-record pointers; result lists hold inline
/// `0x238` copies built by the `0x4441f0` entry builder. Only the
/// bucket value crosses over, via `entry+0x4 = song+0x194`
/// (VA `0x44422e` / file `0x4422e`); the STEP2 filter then tests
/// `entry+0x4 == 1..=5` (VA `0x447050` / file `0x47050`).
/// (Static disassembly survey; REA dossier TBD.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OjnMidTagEntry {
    /// Song id this entry belongs to.
    pub song_id: u32,
    /// Song-wheel list bucket (1..=5 on the real file; indexes the five
    /// `0x77953c..0x77957c` lists via `category - 1`).
    pub category: u32,
}

/// Resolve a table-5 label bucket to its song-wheel list slot.
///
/// The client jump-tables on `category - 1` (range-check `> 4`)
/// into the five wheel lists (dispatch at file `0xbfa5c`, heads
/// `0x77953c..0x77957c` in category order); the STEP2 label filter
/// tests the same 1..=5 domain on inline select-entry `+0x4`
/// (VA `0x447050` / file `0x47050`). Returns the zero-based slot,
/// or `None` when the bucket falls outside 1..=5.
#[must_use]
pub fn ojn_wheel_slot(category: u32) -> Option<usize> {
    let one_based = category.checked_sub(1)?;
    let slot = usize::try_from(one_based).ok()?;
    (slot < OJN_WHEEL_LIST_COUNT).then_some(slot)
}

/// Doubled (`ssang`) Korean initial selected by a `+0x680` keyed-rebuild arm.
///
/// VA `0x444630` (file `0x44630`) derives the arm as
/// `code - 0x27200058` (`sub` at file `0x4464c`); codes
/// `0x27200058..0x27200065` route through jump-table entry 8 to the
/// rebuild caller at file `0x3fa45` (`call` at file `0x3fa99`). The first
/// pass appends entries whose converted title matches the indexed key-name
/// entry (`0x6454d8[arm]`, `strncmp` `n = 2`, append at file `0x44957`);
/// arms `0/2/5/6/8` then test a second fixed entry and append on match.
/// Those five are exactly the doublable base consonants, so each arm
/// accepts the `ssang` partner alongside its base initial:
///
/// - arm `0` (`ㄱ` at file `0x2454d0` via slot file `0x2454d8`) also
///   accepts `ㄲ` (U+3132, EUC-KR `A4 A2` at file `0x245498` via slot
///   file `0x245510`; dispatch `cmp` at file `0x44962`, pointer load
///   at file `0x44971`, append at file `0x44a08`);
/// - arm `2` (`ㄷ` at file `0x2454c8` via slot file `0x2454e0`) also
///   accepts `ㄸ` (U+3138, `A4 A8` at file `0x245494` via slot file
///   `0x245514`; `cmp` at file `0x44a18`, load at file `0x44a27`,
///   append at file `0x44abe`);
/// - arm `5` (`ㅂ` at file `0x2454bc` via slot file `0x2454ec`) also
///   accepts `ㅃ` (U+3143, `A4 B3` at file `0x245490` via slot file
///   `0x245518`; `cmp` at file `0x44ace`, load at file `0x44add`,
///   append at file `0x44b74`);
/// - arm `6` (`ㅅ` at file `0x2454b8` via slot file `0x2454f0`) also
///   accepts `ㅆ` (U+3146, `A4 B6` at file `0x24548c` via slot file
///   `0x24551c`; `cmp` at file `0x44b84`, load at file `0x44b93`,
///   append at file `0x44c2a`);
/// - arm `8` (`ㅈ` at file `0x2454b0` via slot file `0x2454f8`) also
///   accepts `ㅉ` (U+3149, `A4 B9` at file `0x245488` via slot file
///   `0x245520`; `cmp` at file `0x44c3a`, load at file `0x44c49`,
///   append at file `0x44ce6`).
///
/// Other arms (`1/3/4/7/9..13`: `ㄴ/ㄹ/ㅁ/ㅇ/ㅊ/ㅋ/ㅌ/ㅍ/ㅎ`) have no
/// doubled form, so they fall through with only the indexed append
/// (plus the second-loop indexed append at file `0x44f61`). Returns
/// the doubled initial, or `None` for arms without one.
/// (Static disassembly survey; REA dossier TBD.)
const OJN_SEARCH_DOUBLED_INITIALS: [(u32, char); 5] = [
    (0, '\u{3132}'),
    (2, '\u{3138}'),
    (5, '\u{3143}'),
    (6, '\u{3146}'),
    (8, '\u{3149}'),
];

/// Resolve a `+0x680` keyed-rebuild arm to its doubled Korean initial.
///
/// See [`OJN_SEARCH_DOUBLED_INITIALS`] for the per-arm evidence: arms
/// `0/2/5/6/8` select `ㄲ/ㄸ/ㅃ/ㅆ/ㅉ`; every other arm returns `None`.
#[must_use]
pub fn ojn_search_doubled_initial(arm: u32) -> Option<char> {
    OJN_SEARCH_DOUBLED_INITIALS
        .iter()
        .find_map(|(known_arm, initial)| (*known_arm == arm).then_some(*initial))
}

/// Byte stride of one inline select entry (`0x238`).
///
/// The select-music dialog result lists (`+0x640` family, `+0x680`,
/// `+0x690`) hold inline `0x238`-byte entries; every count site divides
/// the `(end - begin)` span by this stride (e.g. the `+0x690` count at
/// file `0x42f8f` and the clear guards at files `0x468b8` / `0x4de57`).
pub const OJN_INLINE_ENTRY_LEN: u32 = 0x238;

/// Count inline entries in a `(begin, end)` list span.
///
/// Mirrors the exe guard idiom: an empty span counts zero and the caller
/// skips the clear (`0x452a00`) — see the `+0x690` clears at file offsets
/// `0x468b8` (VA `0x4468b8`, function `0x446630`) and `0x4de57`
/// (VA `0x44de57`, function `0x44d500`). A span that runs backwards
/// (corrupt) also counts zero, so it never triggers a clear.
#[must_use]
pub fn ojn_inline_list_count(begin: u32, end: u32) -> u32 {
    end.saturating_sub(begin) / OJN_INLINE_ENTRY_LEN
}

/// The decoded `OJNList.dat` middle region: header tables plus footer.
///
/// Resolved against exe consumers (see each entry's docs): table 5 is
/// the song-wheel label bucket (STEP2 label filter), table 2's first
/// word is a selection class, table 3's first word is a nonzero-gated
/// feature switch.
/// Table 1 groups, table 2's second word, table 3's second word and
/// anchor, and all of table 4 stay OPEN (table 1 and table 4 are
/// write-only: stored by the loader, never read back — see entry docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OjnMidRegion {
    /// Table 1 group entries in file order.
    pub groups: Vec<OjnMidGroupEntry>,
    /// Table 2 song entries in file order.
    pub songs: Vec<OjnMidSongEntry>,
    /// Table 3 param entries in file order.
    pub params: Vec<OjnMidParamEntry>,
    /// Table 4 triples in file order.
    pub triples: Vec<OjnMidTriple>,
    /// Table 5 bucket entries in file order.
    pub tags: Vec<OjnMidTagEntry>,
    /// Footer words (last two echo `OJN_MID_SONG_COUNT` /
    /// `OJN_MID_TOP_SONG_ID`).
    pub footer: [u32; 10],
}

/// Compare a file `u32` count against an expected table length
/// without narrowing casts (PERF-07).
fn count_matches(actual: u32, expected: usize) -> bool {
    u32::try_from(expected).is_ok_and(|want| actual == want)
}

/// Split `len` bytes off the cursor; errors when they overrun.
fn take_region<'bytes>(
    bytes: &'bytes [u8],
    cursor: &mut usize,
    len: usize,
) -> Result<&'bytes [u8], FileError> {
    let end = cursor.checked_add(len).ok_or(FileError::Truncated {
        need: usize::MAX,
        have: bytes.len(),
    })?;
    let window = bytes.get(*cursor..end).ok_or(FileError::Truncated {
        need: end,
        have: bytes.len(),
    })?;
    *cursor = end;
    Ok(window)
}

/// Parse middle table 1: `OJN_MID_TABLE1_COUNT` entries of
/// (`song_id`, 0, `group`, 0). Requires the exact table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a nonzero reserved word, or a group outside 1..=2.
pub fn parse_ojn_mid_table1(bytes: &[u8]) -> Result<Vec<OjnMidGroupEntry>, FileError> {
    let need = OJN_MID_TABLE1_COUNT * OJN_MID_TABLE1_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let mut entries = Vec::with_capacity(OJN_MID_TABLE1_COUNT);
    let mut off = 0;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let reserved_a = read_u32le(bytes, off + 4)?;
        let group = read_u32le(bytes, off + 8)?;
        let reserved_b = read_u32le(bytes, off + 12)?;
        if reserved_a != 0 || reserved_b != 0 || group == 0 || group > 2 {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidGroupEntry { song_id, group });
        off += OJN_MID_TABLE1_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 2: `u32` count plus `OJN_MID_SONG_COUNT` entries
/// of (`song_id`, `listing`, 0, `param_b`, 0). Requires the exact
/// table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a wrong count, a nonzero reserved word, or a class
/// outside its observed domain (`listing` 1..=2, `param_b` 0..=1).
pub fn parse_ojn_mid_table2(bytes: &[u8]) -> Result<Vec<OjnMidSongEntry>, FileError> {
    let need = 4 + OJN_MID_SONG_COUNT * OJN_MID_TABLE2_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let count = read_u32le(bytes, 0)?;
    if !count_matches(count, OJN_MID_SONG_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut entries = Vec::with_capacity(OJN_MID_SONG_COUNT);
    let mut off = 4;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let listing = read_u32le(bytes, off + 4)?;
        let reserved_a = read_u32le(bytes, off + 8)?;
        let param_b = read_u32le(bytes, off + 12)?;
        let reserved_b = read_u32le(bytes, off + 16)?;
        if reserved_a != 0 || reserved_b != 0 || listing == 0 || listing > 2 || param_b > 1 {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidSongEntry {
            song_id,
            listing,
            param_b,
        });
        off += OJN_MID_TABLE2_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 3: `u32` count plus `OJN_MID_TABLE3_COUNT`
/// entries of (`song_id`, `param_a`, `param_b`, `anchor`). Requires
/// the exact table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a wrong count, or an anchor that is not
/// `OJN_MID_TABLE3_ANCHOR`.
pub fn parse_ojn_mid_table3(bytes: &[u8]) -> Result<Vec<OjnMidParamEntry>, FileError> {
    let need = 4 + OJN_MID_TABLE3_COUNT * OJN_MID_TABLE3_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let count = read_u32le(bytes, 0)?;
    if !count_matches(count, OJN_MID_TABLE3_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut entries = Vec::with_capacity(OJN_MID_TABLE3_COUNT);
    let mut off = 4;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let param_a = read_u32le(bytes, off + 4)?;
        let param_b = read_u32le(bytes, off + 8)?;
        let anchor = read_u32le(bytes, off + 12)?;
        if anchor != OJN_MID_TABLE3_ANCHOR {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidParamEntry {
            song_id,
            param_a,
            param_b,
            anchor,
        });
        off += OJN_MID_TABLE3_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 4: `OJN_MID_TABLE4_COUNT` raw triples.
/// Requires the exact table length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table.
pub fn parse_ojn_mid_table4(bytes: &[u8]) -> Result<Vec<OjnMidTriple>, FileError> {
    let need = OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let mut entries = Vec::with_capacity(OJN_MID_TABLE4_COUNT);
    let mut off = 0;
    while off < need {
        entries.push(OjnMidTriple {
            a: read_u32le(bytes, off)?,
            b: read_u32le(bytes, off + 4)?,
            c: read_u32le(bytes, off + 8)?,
        });
        off += OJN_MID_TABLE4_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse middle table 5: (`OJN_MID_TABLE5_HEAD`, `OJN_MID_SONG_COUNT`)
/// plus entries of (`song_id`, `category`). Requires the exact length.
///
/// # Errors
///
/// `FileError::Truncated` on a short table, `FileError::UnknownFormat`
/// on a long table, a wrong head/count, or a bucket outside 1..=5.
pub fn parse_ojn_mid_table5(bytes: &[u8]) -> Result<Vec<OjnMidTagEntry>, FileError> {
    let need = 8 + OJN_MID_SONG_COUNT * OJN_MID_TABLE5_ENTRY_LEN;
    if bytes.len() < need {
        return Err(FileError::Truncated {
            need,
            have: bytes.len(),
        });
    }
    if bytes.len() > need {
        return Err(unknown_format(bytes));
    }
    let head = read_u32le(bytes, 0)?;
    let count = read_u32le(bytes, 4)?;
    if head != OJN_MID_TABLE5_HEAD || !count_matches(count, OJN_MID_SONG_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut entries = Vec::with_capacity(OJN_MID_SONG_COUNT);
    let mut off = 8;
    while off < need {
        let song_id = read_u32le(bytes, off)?;
        let category = read_u32le(bytes, off + 4)?;
        if category == 0 || category > 5 {
            return Err(unknown_format(&bytes[off..off + 4]));
        }
        entries.push(OjnMidTagEntry { song_id, category });
        off += OJN_MID_TABLE5_ENTRY_LEN;
    }
    Ok(entries)
}

/// Parse the `OJNList.dat` middle region: magic + table-1 count, the
/// five tables, and the footer. Requires exactly `OJN_MID_REGION_LEN`
/// bytes.
///
/// # Errors
///
/// `FileError::Truncated` on a short region, `FileError::UnknownFormat`
/// on a long region, a wrong magic/count, or any malformed table.
pub fn parse_ojn_mid_region(bytes: &[u8]) -> Result<OjnMidRegion, FileError> {
    if bytes.len() < OJN_MID_REGION_LEN {
        return Err(FileError::Truncated {
            need: OJN_MID_REGION_LEN,
            have: bytes.len(),
        });
    }
    if bytes.len() > OJN_MID_REGION_LEN {
        return Err(unknown_format(bytes));
    }
    let magic = read_u32le(bytes, 0)?;
    let group_count = read_u32le(bytes, 4)?;
    if magic != OJN_MID_MAGIC || !count_matches(group_count, OJN_MID_TABLE1_COUNT) {
        return Err(unknown_format(&bytes[..4]));
    }
    let mut cursor = 8;
    let groups = parse_ojn_mid_table1(take_region(
        bytes,
        &mut cursor,
        OJN_MID_TABLE1_COUNT * OJN_MID_TABLE1_ENTRY_LEN,
    )?)?;
    let songs = parse_ojn_mid_table2(take_region(
        bytes,
        &mut cursor,
        4 + OJN_MID_SONG_COUNT * OJN_MID_TABLE2_ENTRY_LEN,
    )?)?;
    let params = parse_ojn_mid_table3(take_region(
        bytes,
        &mut cursor,
        4 + OJN_MID_TABLE3_COUNT * OJN_MID_TABLE3_ENTRY_LEN,
    )?)?;
    let triples = parse_ojn_mid_table4(take_region(
        bytes,
        &mut cursor,
        OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN,
    )?)?;
    let tags = parse_ojn_mid_table5(take_region(
        bytes,
        &mut cursor,
        8 + OJN_MID_SONG_COUNT * OJN_MID_TABLE5_ENTRY_LEN,
    )?)?;
    let footer_bytes = take_region(bytes, &mut cursor, OJN_MID_FOOTER_LEN)?;
    let mut footer = [0u32; 10];
    for (slot, word) in footer.iter_mut().enumerate() {
        *word = read_u32le(footer_bytes, slot * 4)?;
    }
    if !count_matches(footer[8], OJN_MID_SONG_COUNT) || footer[9] != OJN_MID_TOP_SONG_ID {
        return Err(unknown_format(&footer_bytes[..4]));
    }
    Ok(OjnMidRegion {
        groups,
        songs,
        params,
        triples,
        tags,
        footer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_table1_blob(group: u32) -> Vec<u8> {
        // Synthetic ids mirror the real shape (descending); values are
        // arbitrary but in-domain.
        let mut blob = Vec::with_capacity(OJN_MID_TABLE1_COUNT * OJN_MID_TABLE1_ENTRY_LEN);
        for song_id in (684..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
            blob.extend_from_slice(&group.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table1_parses_ids_and_groups() {
        let blob = full_table1_blob(2);
        let entries = parse_ojn_mid_table1(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_TABLE1_COUNT);
        assert_eq!(
            entries[0],
            OjnMidGroupEntry {
                song_id: 887,
                group: 2
            }
        );
        assert_eq!(entries[OJN_MID_TABLE1_COUNT - 1].group, 2);
    }

    #[test]
    fn mid_table1_rejects_bad_group() {
        let mut blob = full_table1_blob(1);
        blob[8..12].copy_from_slice(&3u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table1(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table1_rejects_nonzero_reserved() {
        let mut blob = full_table1_blob(1);
        blob[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table1(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table1_rejects_short_and_long_input() {
        let blob = full_table1_blob(1);
        assert!(matches!(
            parse_ojn_mid_table1(&blob[..blob.len() - 1]),
            Err(FileError::Truncated { .. })
        ));
        let mut long = blob.clone();
        long.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_ojn_mid_table1(&long),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn full_table2_blob(listing: u32, param_b: u32) -> Vec<u8> {
        let mut blob = Vec::with_capacity(4 + OJN_MID_SONG_COUNT * OJN_MID_TABLE2_ENTRY_LEN);
        let count = u32::try_from(OJN_MID_SONG_COUNT).unwrap();
        blob.extend_from_slice(&count.to_le_bytes());
        for song_id in (326..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&listing.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
            blob.extend_from_slice(&param_b.to_le_bytes());
            blob.extend_from_slice(&0u32.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table2_parses_count_and_classes() {
        let blob = full_table2_blob(2, 1);
        let entries = parse_ojn_mid_table2(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_SONG_COUNT);
        assert_eq!(
            entries[0],
            OjnMidSongEntry {
                song_id: 887,
                listing: 2,
                param_b: 1
            }
        );
    }

    #[test]
    fn mid_table2_rejects_bad_count_and_classes() {
        let mut blob = full_table2_blob(1, 0);
        blob[0..4].copy_from_slice(&561u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table2(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut blob = full_table2_blob(3, 0);
        assert!(matches!(
            parse_ojn_mid_table2(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        blob = full_table2_blob(1, 2);
        assert!(matches!(
            parse_ojn_mid_table2(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn full_table3_blob() -> Vec<u8> {
        let mut blob = Vec::with_capacity(4 + OJN_MID_TABLE3_COUNT * OJN_MID_TABLE3_ENTRY_LEN);
        let count = u32::try_from(OJN_MID_TABLE3_COUNT).unwrap();
        blob.extend_from_slice(&count.to_le_bytes());
        for song_id in (481..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&2000u32.to_le_bytes());
            blob.extend_from_slice(&5000u32.to_le_bytes());
            blob.extend_from_slice(&OJN_MID_TABLE3_ANCHOR.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table3_parses_params_and_anchor() {
        let blob = full_table3_blob();
        let entries = parse_ojn_mid_table3(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_TABLE3_COUNT);
        assert_eq!(
            entries[0],
            OjnMidParamEntry {
                song_id: 887,
                param_a: 2000,
                param_b: 5000,
                anchor: OJN_MID_TABLE3_ANCHOR
            }
        );
    }

    #[test]
    fn mid_table3_rejects_bad_anchor() {
        let mut blob = full_table3_blob();
        let last = blob.len() - 4;
        blob[last..].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table3(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table4_parses_triples() {
        let mut blob = Vec::with_capacity(OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN);
        for i in 0..u32::try_from(OJN_MID_TABLE4_COUNT).unwrap() {
            blob.extend_from_slice(&i.to_le_bytes());
            blob.extend_from_slice(&(873 - i).to_le_bytes());
            blob.extend_from_slice(&2u32.to_le_bytes());
        }
        let triples = parse_ojn_mid_table4(&blob).unwrap();
        assert_eq!(triples.len(), OJN_MID_TABLE4_COUNT);
        assert_eq!(triples[0], OjnMidTriple { a: 0, b: 873, c: 2 });
        assert_eq!(
            triples[OJN_MID_TABLE4_COUNT - 1],
            OjnMidTriple {
                a: 119,
                b: 754,
                c: 2
            }
        );
    }

    #[test]
    fn mid_table4_rejects_short_and_long_input() {
        let need = OJN_MID_TABLE4_COUNT * OJN_MID_TABLE4_ENTRY_LEN;
        assert!(matches!(
            parse_ojn_mid_table4(&vec![0u8; need - 4]),
            Err(FileError::Truncated { .. })
        ));
        assert!(matches!(
            parse_ojn_mid_table4(&vec![0u8; need + 4]),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    fn full_table5_blob(category: u32) -> Vec<u8> {
        let mut blob = Vec::with_capacity(8 + OJN_MID_SONG_COUNT * OJN_MID_TABLE5_ENTRY_LEN);
        blob.extend_from_slice(&OJN_MID_TABLE5_HEAD.to_le_bytes());
        let count = u32::try_from(OJN_MID_SONG_COUNT).unwrap();
        blob.extend_from_slice(&count.to_le_bytes());
        for song_id in (326..=887u32).rev() {
            blob.extend_from_slice(&song_id.to_le_bytes());
            blob.extend_from_slice(&category.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_table5_parses_ids_and_buckets() {
        let blob = full_table5_blob(5);
        let entries = parse_ojn_mid_table5(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_SONG_COUNT);
        assert_eq!(
            entries[0],
            OjnMidTagEntry {
                song_id: 887,
                category: 5
            }
        );
    }

    #[test]
    fn mid_table5_rejects_bad_head_count_and_bucket() {
        let mut blob = full_table5_blob(1);
        blob[0..4].copy_from_slice(&2u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table5(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut blob = full_table5_blob(1);
        blob[8..12].copy_from_slice(&0u32.to_le_bytes());
        blob[12..16].copy_from_slice(&6u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_table5(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_table5_category_indexes_five_lists() {
        // The client jump-tables on `category - 1` into five parallel
        // song lists (dispatches at file offsets `0xbfa5c` and
        // `0x16121e`): every stored bucket must index that table, so
        // only 1..=5 parses and each value resolves to a wheel slot.
        let blob = full_table5_blob(5);
        let entries = parse_ojn_mid_table5(&blob).unwrap();
        assert_eq!(entries.len(), OJN_MID_SONG_COUNT);
        for entry in &entries {
            assert!(ojn_wheel_slot(entry.category).is_some());
        }
    }

    #[test]
    fn wheel_slot_maps_each_label_bucket_to_its_own_list() {
        // Categories 1..=5 index the five `0x77953c..0x77957c` wheel
        // lists via `category - 1` (dispatch at file `0xbfa5c`).
        for (category, want) in [1u32, 2, 3, 4, 5].iter().zip(0..5) {
            assert_eq!(ojn_wheel_slot(*category), Some(want));
        }
    }

    #[test]
    fn wheel_slot_rejects_buckets_outside_one_to_five() {
        // `0`, `6`, and `u32::MAX` fail the exe range-check (`> 4`
        // on `category - 1`), so no wheel list applies.
        for category in [0u32, 6, 7, u32::MAX] {
            assert_eq!(ojn_wheel_slot(category), None);
        }
    }

    #[test]
    fn inline_list_count_divides_span_by_entry_stride() {
        // The exe counts inline entries as `(end - begin) / 0x238`:
        // the `+0x690` count at file `0x42f8f` and the clear guards at
        // files `0x468b8` and `0x4de57`.
        assert_eq!(ojn_inline_list_count(0x1000, 0x1000), 0);
        assert_eq!(
            ojn_inline_list_count(0x1000, 0x1000 + OJN_INLINE_ENTRY_LEN),
            1
        );
        assert_eq!(
            ojn_inline_list_count(0x1000, 0x1000 + 2 * OJN_INLINE_ENTRY_LEN),
            2
        );
    }

    #[test]
    fn inline_list_count_clamps_inverted_span_to_zero() {
        // A backwards span (corrupt) counts as empty, so the caller
        // skips the `0x452a00` clear instead of dividing garbage.
        assert_eq!(ojn_inline_list_count(0x2000, 0x1000), 0);
    }

    #[test]
    fn search_arms_select_their_doubled_initials() {
        // VA `0x444630` fixed arms append a second `+0x680` list entry
        // when the title matches the `ssang` partner: `cmp` sites at
        // files `0x44962`/`0x44a18`/`0x44ace`/`0x44b84`/`0x44c3a` with
        // pointer loads at `0x44971`/`0x44a27`/`0x44add`/`0x44b93`/
        // `0x44c49` and appends at `0x44a08`/`0x44abe`/`0x44b74`/
        // `0x44c2a`/`0x44ce6`.
        for (arm, want) in [
            (0u32, '\u{3132}'),
            (2u32, '\u{3138}'),
            (5u32, '\u{3143}'),
            (6u32, '\u{3146}'),
            (8u32, '\u{3149}'),
        ] {
            assert_eq!(ojn_search_doubled_initial(arm), Some(want));
        }
    }

    #[test]
    fn search_arms_without_doubled_form_select_none() {
        // `ㄴ/ㄹ/ㅁ/ㅇ/ㅊ/ㅋ/ㅌ/ㅍ/ㅎ` (arms 1/3/4/7/9..13) have no
        // `ssang` partner, so the VA `0x444630` chain falls through
        // with only the indexed append (files `0x44957`/`0x44f61`).
        for arm in [1u32, 3, 4, 7, 9, 10, 11, 12, 13, u32::MAX] {
            assert_eq!(ojn_search_doubled_initial(arm), None);
        }
    }

    fn full_mid_region_blob() -> Vec<u8> {
        let mut blob = Vec::with_capacity(OJN_MID_REGION_LEN);
        blob.extend_from_slice(&OJN_MID_MAGIC.to_le_bytes());
        let group_count = u32::try_from(OJN_MID_TABLE1_COUNT).unwrap();
        blob.extend_from_slice(&group_count.to_le_bytes());
        blob.extend_from_slice(&full_table1_blob(1));
        blob.extend_from_slice(&full_table2_blob(1, 0));
        blob.extend_from_slice(&full_table3_blob());
        for _ in 0..OJN_MID_TABLE4_COUNT {
            blob.extend_from_slice(&0u32.to_le_bytes());
            blob.extend_from_slice(&246u32.to_le_bytes());
            blob.extend_from_slice(&2u32.to_le_bytes());
        }
        blob.extend_from_slice(&full_table5_blob(1));
        for word in [2u32, 173, 50, 0, 166, 50, 0, 0, 562, 887] {
            blob.extend_from_slice(&word.to_le_bytes());
        }
        blob
    }

    #[test]
    fn mid_region_parses_all_tables_and_footer() {
        let blob = full_mid_region_blob();
        assert_eq!(blob.len(), OJN_MID_REGION_LEN);
        let region = parse_ojn_mid_region(&blob).unwrap();
        assert_eq!(region.groups.len(), OJN_MID_TABLE1_COUNT);
        assert_eq!(region.songs.len(), OJN_MID_SONG_COUNT);
        assert_eq!(region.params.len(), OJN_MID_TABLE3_COUNT);
        assert_eq!(region.triples.len(), OJN_MID_TABLE4_COUNT);
        assert_eq!(region.tags.len(), OJN_MID_SONG_COUNT);
        assert_eq!(region.groups[0].song_id, 887);
        assert_eq!(region.songs[0].song_id, 887);
        assert_eq!(region.params[0].anchor, OJN_MID_TABLE3_ANCHOR);
        assert_eq!(region.tags[0].category, 1);
        assert_eq!(region.footer[8], u32::try_from(OJN_MID_SONG_COUNT).unwrap());
        assert_eq!(region.footer[9], OJN_MID_TOP_SONG_ID);
    }

    #[test]
    fn mid_region_rejects_bad_magic_and_footer() {
        let mut blob = full_mid_region_blob();
        blob[0..4].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_region(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
        let mut blob = full_mid_region_blob();
        let last = blob.len() - 4;
        blob[last..].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            parse_ojn_mid_region(&blob),
            Err(FileError::UnknownFormat { .. })
        ));
    }

    #[test]
    fn mid_region_rejects_short_and_long_input() {
        let blob = full_mid_region_blob();
        assert!(matches!(
            parse_ojn_mid_region(&blob[..blob.len() - 4]),
            Err(FileError::Truncated { .. })
        ));
        let mut long = blob.clone();
        long.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            parse_ojn_mid_region(&long),
            Err(FileError::UnknownFormat { .. })
        ));
    }
}
