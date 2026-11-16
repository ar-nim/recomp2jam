//! Local music availability (`docs/music-availability.md` §1).
//!
//! A song is available when its `o2ma<id>.ojn` file exists under the
//! install's `Music\` folder. The 7.55 client learns this from one
//! directory scan while it loads `OJNList.dat` (`FindFirstFileA`
//! `\music\*.ojn`, VA `0x47c8ce`), never from a server flag: the
//! server can only annotate a song the client already knows.
//!
//! [`installed_music`] is the pure half of that scan — a list of file
//! names in, the packed seat entries out. The `read_dir` adapter lives
//! at the shell edge (`o2jam_shell::music_install`).

use crate::ojn::OJN_LEVEL_COUNT;

/// File-name prefix of a chart file (`o2ma###.ojn`).
const CHART_PREFIX: &str = "o2ma";
/// Extension the scan matches (pattern VA `0x647a44`).
const CHART_EXT: &str = ".ojn";
/// Installed marker the scan ORs into the packed id (VA `0x47cbec`).
const INSTALLED_MARK: u16 = 0x8000;
/// Low 12 bits hold the music id (`seat+0x38+idx*4`, VA `0x540150`).
const ID_MASK: u16 = 0x0fff;
/// High-nibble shift of the variant/flag word (`seat+0x1038+idx*4`).
const VARIANT_SHIFT: u32 = 12;
/// Installed flag bit inside the variant nibble (§1: "installed flag
/// value is therefore `0x8`").
pub const INSTALLED_VARIANT_BIT: u8 = 0x8;

/// One entry of the local music seat, in the two-slot shape the exe
/// stores (§1): the id word and the variant/flag nibble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstalledSong {
    /// Music id (`packed & 0x0fff`).
    pub id: u16,
    /// Variant nibble (`(packed & 0xf000) >> 12`). A file the scan
    /// found sets bit [`INSTALLED_VARIANT_BIT`], so its nibble is `8`.
    pub variant: u8,
}

impl InstalledSong {
    /// Seat entry for a music id the scan found on disk: the id packed
    /// with the installed mark, split back into its two seat slots.
    ///
    /// The id keeps only its low 12 bits — the exe's seat field is 12
    /// bits wide, so a larger number aliases onto the truncated id.
    #[must_use]
    pub fn installed(id: u16) -> Self {
        let packed = id | INSTALLED_MARK;
        Self {
            id: packed & ID_MASK,
            variant: u8::try_from(u32::from(packed & !ID_MASK) >> VARIANT_SHIFT).unwrap_or(0),
        }
    }

    /// Whether the entry carries the installed mark. Every entry a
    /// directory scan produces does; the flag is read back rather than
    /// assumed because the seat also holds entries from other sources.
    #[must_use]
    pub fn is_installed(self) -> bool {
        self.variant & INSTALLED_VARIANT_BIT != 0
    }
}

/// The local music seat (§1): one entry per `*.ojn` file the scan
/// found, in scan order, duplicates included — the exe appends one
/// entry per directory hit (`seat+0x34` count, VA `0x540130`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledMusic {
    songs: Vec<InstalledSong>,
}

impl InstalledMusic {
    /// Seat entries in scan order.
    #[must_use]
    pub fn songs(&self) -> &[InstalledSong] {
        &self.songs
    }

    /// Number of entries (the exe's seat count).
    #[must_use]
    pub fn len(&self) -> usize {
        self.songs.len()
    }

    /// Whether the scan found no chart file at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    /// First seat entry for one music id, if the scan found it.
    #[must_use]
    pub fn entry(&self, id: u16) -> Option<InstalledSong> {
        self.songs.iter().copied().find(|song| song.id == id)
    }

    /// Whether a chart file for this song id is installed: the song
    /// table's `entry+0x15c` bit `0x8` (§1) in our terms.
    #[must_use]
    pub fn is_installed(&self, id: u16) -> bool {
        self.entry(id).is_some_and(InstalledSong::is_installed)
    }
}

/// Pack one directory listing of `Music\` into the seat (§1).
///
/// Names are matched like the Windows scan's `\music\*.ojn` pattern:
/// the `o2ma` prefix and `.ojn` extension are case-blind, and the file
/// stem must start with the digits of the music id. Everything else —
/// `bgEffect.ojm`, `o2ma.ojn`, a name whose digits never start — is
/// not a chart and stays out of the seat.
#[must_use]
pub fn installed_music<I, S>(names: I) -> InstalledMusic
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    InstalledMusic {
        songs: names
            .into_iter()
            .filter_map(|name| chart_id(name.as_ref()))
            .map(InstalledSong::installed)
            .collect(),
    }
}

/// Music id a file name names (`None` when the name is no chart file).
fn chart_id(name: &str) -> Option<u16> {
    let stem = chart_stem(name)?;
    let digits = stem.get(CHART_PREFIX.len()..)?;
    let digits = digits.split('_').next()?;
    digits.parse().ok()
}

/// Stem of a `o2ma###.ojn` name, checked for prefix and extension
/// (case-blind, like the Windows pattern match).
fn chart_stem(name: &str) -> Option<&str> {
    let plain = name.rsplit(['/', '\\']).next()?;
    let ext_len = CHART_EXT.len();
    let stem_len = plain.len().checked_sub(ext_len)?;
    let (stem, ext) = plain.split_at(stem_len);
    if !ext.eq_ignore_ascii_case(CHART_EXT) {
        return None;
    }
    if !stem
        .get(..CHART_PREFIX.len())?
        .eq_ignore_ascii_case(CHART_PREFIX)
    {
        return None;
    }
    Some(stem)
}

/// Per-difficulty chart presence, the song table's `entry+0x15c` bits
/// 0-2 (§1; mask table VA `0x680724` = `1`/`2`/`4` =
/// Easy/Normal/Hard).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChartMask(u8);

impl ChartMask {
    /// No playable chart (a song the install does not carry).
    pub const NONE: Self = Self(0);
    /// All three charts, the value the music-list handler stamps for a
    /// song the server lists (VA `0x561080`), and the set a local
    /// chart file provides.
    pub const ALL: Self = Self(0x7);

    /// Mask the low three bits (the reader's field is three bits wide).
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0x7)
    }

    /// Raw `entry+0x15c` chart bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Whether the song has no chart at all.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether the chart at one difficulty slot is present.
    #[must_use]
    pub const fn has_chart(self, slot: usize) -> bool {
        slot < OJN_LEVEL_COUNT && self.0 & (1 << slot) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mixed listing the real `Music\` folder carries: charts plus
    /// sound containers and a non-chart sheet.
    fn listing() -> Vec<&'static str> {
        vec![
            "bgEffect.ojm",
            "o2ma757.ojn",
            "o2ma757.ojm",
            "o2ma758.ojn",
            "O2MA999.OJN",
            "BGM.ojm",
        ]
    }

    #[test]
    fn scan_pattern_names_pack_the_installed_seat() {
        // §1: each `*.ojn` hit packs its id with `or $0x8000`, the low
        // 12 bits land in `seat+0x38` and the high nibble in
        // `seat+0x1038`, so id 757 reads back as variant 8.
        let seat = installed_music(listing());
        assert_eq!(seat.len(), 3);
        assert_eq!(
            seat.songs(),
            [
                InstalledSong {
                    id: 757,
                    variant: 8
                },
                InstalledSong {
                    id: 758,
                    variant: 8
                },
                InstalledSong {
                    id: 999,
                    variant: 8
                },
            ]
        );
        assert!(seat.is_installed(757));
        assert!(seat.is_installed(999), "the pattern match is case-blind");
        assert!(!seat.is_installed(760));
        assert_eq!(seat.entry(758).map(|song| song.id), Some(758));
    }

    #[test]
    fn non_chart_names_stay_out_of_the_seat() {
        // The scan matches `\music\*.ojn` only: sound containers, an
        // id-less name, and a directory-prefixed miss contribute no
        // entry.
        let seat = installed_music(vec![
            "bgEffect.ojm",
            "o2ma.ojn",
            "o2maabc.ojn",
            "o2ma757.bnd",
            "",
        ]);
        assert!(seat.is_empty());
        assert!(!seat.is_installed(757));
    }

    #[test]
    fn variant_suffix_and_directory_prefix_keep_the_same_id() {
        // §4 names a downloaded chart `o2ma<id>_<variant>.ojn`; the id
        // comes from the digits before the underscore, and a full path
        // reduces to its file name first.
        let seat = installed_music(vec![r"Music\o2ma999_2.ojn", "Music/o2ma12_0.ojn"]);
        assert_eq!(seat.songs().len(), 2);
        assert_eq!(seat.songs()[0].id, 999);
        assert_eq!(seat.songs()[1].id, 12);
        assert!(seat.songs().iter().all(|song| song.is_installed()));
    }

    #[test]
    fn seat_holds_one_entry_per_file_and_masks_the_id_to_twelve_bits() {
        // The exe appends one seat entry per directory hit (count at
        // `seat+0x34`), so duplicates stay; and the id field is 12 bits
        // wide, so a larger number aliases onto its truncated value:
        // 4096 & 0x0fff = 0.
        let seat = installed_music(vec!["o2ma757.ojn", "o2ma757.ojn", "o2ma4096.ojn"]);
        assert_eq!(seat.len(), 3);
        assert_eq!(seat.songs()[0], seat.songs()[1]);
        assert_eq!(seat.songs()[2].id, 0);
        assert!(seat.is_installed(0));
    }

    #[test]
    fn seat_lookup_answers_by_id_not_by_position() {
        // Availability is a lookup by song id, not "the first entry":
        // a scan that finds 758 first still answers for 757, and an
        // empty listing answers no id at all.
        let seat = installed_music(vec!["o2ma758.ojn", "o2ma757.ojn"]);
        assert_eq!(seat.songs()[0].id, 758, "scan order is kept");
        assert!(seat.is_installed(757));
        assert!(seat.is_installed(758));
        assert!(!seat.is_installed(759));
        let empty = installed_music(Vec::<String>::new());
        assert_eq!(empty, InstalledMusic::default());
        assert!(!empty.is_installed(757));
    }

    #[test]
    fn chart_mask_reads_the_three_difficulty_bits() {
        // §1's mask table: 1 = Easy, 2 = Normal, 4 = Hard; an
        // uninstalled song carries no chart bit at all.
        let all = ChartMask::ALL;
        assert_eq!(all.bits(), 0x7);
        assert!((0..OJN_LEVEL_COUNT).all(|slot| all.has_chart(slot)));
        assert!(!all.has_chart(OJN_LEVEL_COUNT));
        assert!(ChartMask::NONE.is_empty());
        let easy_only = ChartMask::from_bits(0xff);
        assert_eq!(easy_only.bits(), 0x7, "the reader's field is three bits");
        let normal = ChartMask::from_bits(2);
        assert!(normal.has_chart(1));
        assert!(!normal.has_chart(0));
    }
}
