//! Side screens reachable from the lobbies (`docs/lifecycle.md` §12).
//!
//! Channel, room, and waiting each link to the shop, music shop, my
//! room, planet, lanking, payment, and tutorial views. Unlike the
//! main-flow scenes, these never use flat-color stand-ins: every
//! view paints real archive art through [`load_scene_figures`].

use std::collections::HashMap;

use super::super::avatar::{
    avatar_item_by_id, bind_scene_rects, load_archive_sprite, load_scene_figures,
    paint_scene_figures, scene_block, scene_sprite_names, AvatarFigure, ScenePlacement,
};
use super::super::screen::Screen;
use o2jam_file::OJN_LEVEL_COUNT;
use o2jam_file::{AvatarItemRow, AvatarItemTable, Bnd, ControlBlock, ControlList, OjnListEntry};
use o2jam_net::play::{MusicListAnswer, SetRoomMusic};
use o2jam_net::shop::{SellItemResponse, SyncItemPurchasePrefix, SyncItemPurchaseWallets};
use o2jam_net::wallet::{
    FreePassType, SyncCashPointResponse, SyncFreePassResponse, SyncFreePassResult,
};
use o2jam_net::{
    CharacterInfo, CreateRoom, Difficulty, GameMode, GameSpeed, GameStatsUpdate, GiftItemInfo,
    GiftMusicInfo, KeyMode, RankDelta, RankEntry,
};
use o2jam_render::{pack_xrgb, Frame, Rect};

/// Item-shop block (`CStateShop`, 164 decl entries).
pub const SHOP_BLOCK: &str = "STATE_SHOP";
/// Item-shop layout (186 rects).
pub const SHOP_BIND_FILE: &str = "State_ItemShop.bnd";
/// Song-shop block (`CStateMShop`, 107 decl entries).
pub const MUSIC_SHOP_BLOCK: &str = "STATE_MUSICSHOP";
/// Song-shop layout (138 rects).
pub const MUSIC_SHOP_BIND_FILE: &str = "State_MShop.bnd";
/// Avatar-room block (106 decl entries; no dedicated state class).
pub const MYROOM_BLOCK: &str = "STATE_MYROOM";
/// Avatar-room layout (111 rects).
pub const MYROOM_BIND_FILE: &str = "State_Myroom.bnd";
/// Solo-mission block (`CStatePlanet`, 17 decl entries).
pub const PLANET_BLOCK: &str = "STATE_PLANET";
/// Solo-mission layout (17 rects).
pub const PLANET_BIND_FILE: &str = "State_Planet.bnd";
/// Ranking block (display-only like channel: no state class).
pub const LANKING_BLOCK: &str = "STATE_LANKING";
/// Ranking layout (73 rects).
pub const LANKING_BIND_FILE: &str = "State_Lanking.bnd";
/// Billing-view block (`CDPayment`).
pub const PAYMENT_BLOCK: &str = "STATE_PAYMENT";
/// Billing-view layout (9 rects).
pub const PAYMENT_BIND_FILE: &str = "State_Payment.bnd";
/// Gift-view block (second `CDPayment` view).
pub const PAYMENT2_BLOCK: &str = "STATE_PAYMENT2";
/// Gift-view layout (12 rects).
pub const PAYMENT2_BIND_FILE: &str = "State_Payment2.bnd";
/// Tutorial backdrop in `Playing1.opi` (stub `0x00` block, so no
/// `ControlList` entries; `CStateTutorial`).
pub const TUTORIAL_BACKGROUND_SPRITE: &str = "Playing_TutorialBG.ojs";

/// Side views from lifecycle §12 (state IDs in
/// `ControlList_Interface.txt`; counts in `docs/file-formats/opi.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideScreen {
    /// Item shop (`CStateShop`).
    Shop,
    /// Song shop (`CStateMShop`, `jsBuyMusic` bridge).
    MusicShop,
    /// Avatar room (`CItemManager`, `CPlayerInfo`).
    MyRoom,
    /// Solo missions (`CStatePlanet`).
    Planet,
    /// Rankings (display-only, no state class).
    Lanking,
    /// Billing views (`CDPayment`: payment + gift).
    Payment,
    /// Guided play (`CStateTutorial`).
    Tutorial,
}

/// Every side screen, in lifecycle §12 order.
pub const ALL_SIDE_SCREENS: [SideScreen; 7] = [
    SideScreen::Shop,
    SideScreen::MusicShop,
    SideScreen::MyRoom,
    SideScreen::Planet,
    SideScreen::Lanking,
    SideScreen::Payment,
    SideScreen::Tutorial,
];

impl SideScreen {
    /// `ControlList` block (`None`: tutorial is a stub-only block).
    #[must_use]
    pub const fn block_name(self) -> Option<&'static str> {
        match self {
            SideScreen::Shop => Some(SHOP_BLOCK),
            SideScreen::MusicShop => Some(MUSIC_SHOP_BLOCK),
            SideScreen::MyRoom => Some(MYROOM_BLOCK),
            SideScreen::Planet => Some(PLANET_BLOCK),
            SideScreen::Lanking => Some(LANKING_BLOCK),
            SideScreen::Payment => Some(PAYMENT_BLOCK),
            SideScreen::Tutorial => None,
        }
    }

    /// `.bnd` layout file (`None`: tutorial has no bind file).
    #[must_use]
    pub const fn bind_file(self) -> Option<&'static str> {
        match self {
            SideScreen::Shop => Some(SHOP_BIND_FILE),
            SideScreen::MusicShop => Some(MUSIC_SHOP_BIND_FILE),
            SideScreen::MyRoom => Some(MYROOM_BIND_FILE),
            SideScreen::Planet => Some(PLANET_BIND_FILE),
            SideScreen::Lanking => Some(LANKING_BIND_FILE),
            SideScreen::Payment => Some(PAYMENT_BIND_FILE),
            SideScreen::Tutorial => None,
        }
    }

    /// Owning client class (`None`: display-only blocks like channel).
    #[must_use]
    pub const fn state_class(self) -> Option<&'static str> {
        match self {
            SideScreen::Shop => Some("CStateShop"),
            SideScreen::MusicShop => Some("CStateMShop"),
            SideScreen::MyRoom | SideScreen::Lanking => None,
            SideScreen::Planet => Some("CStatePlanet"),
            SideScreen::Payment => Some("CDPayment"),
            SideScreen::Tutorial => Some("CStateTutorial"),
        }
    }

    /// Scene-BGM cue this side screen enters with: the `(id, repeat)`
    /// its entry passes the exe's scene-music selector (file `0x151820`
    /// = VA `0x551820`, forwards `(id, loop)` to the bank behind
    /// `soundmgr[0x76832C] + 0x340C0`). The `id` is a one-based record
    /// over `Music/BGM.ojm`; [`o2jam_audio::BGM_SILENCE_ID`] (`-1`)
    /// stops the channel. Pinned per-scene table
    /// (`docs/coverage-audit.md` "Fresh pins", re-read 2026-11-11):
    ///
    /// | Side screen | id | loop |
    /// |---|---|---|
    /// | Shop (`CStateShop`) | 4 | loop |
    /// | MusicShop (`CStateMShop`) | 10 (decimal) | loop |
    /// | MyRoom | 5 | loop |
    /// | Planet (`CStatePlanet`) | 9 | loop |
    /// | Payment / Payment2 (`CDPayment`) | 3 | loop |
    /// | Lanking / Tutorial | silence `-1` | — |
    ///
    /// The music-shop entry pushes decimal `10` (record 10 names the
    /// shop track); the audit's `0x10` shorthand is corrected the
    /// same way [`Screen::bgm_cue`](super::super::screen::Screen::bgm_cue)
    /// corrects it. Lanking has no state class and tutorial holds no
    /// pinned cue (no selector caller names them), so they cue
    /// silence. The launch client (`recomp2jam/src/main.rs`)
    /// delegates its side hook here, so the pairs live in one place
    /// and `ScreenGame` re-cues every side entry through this table.
    #[must_use]
    pub const fn bgm_cue(self) -> (i32, bool) {
        use o2jam_audio::BGM_SILENCE_ID;
        match self {
            SideScreen::Shop => (4, true),
            SideScreen::MusicShop => (10, true),
            SideScreen::MyRoom => (5, true),
            SideScreen::Planet => (9, true),
            SideScreen::Payment => (3, true),
            SideScreen::Lanking | SideScreen::Tutorial => (BGM_SILENCE_ID, false),
        }
    }
}

/// Whether a lobby may open a side screen: any lobby opens any side
/// (lifecycle §12 transition summary).
#[must_use]
pub const fn can_visit_side(from: Screen, side: SideScreen) -> bool {
    let _ = side;
    from.is_side_lobby()
}

/// Whether a side screen may return to a lobby: any side returns to
/// any lobby.
#[must_use]
pub const fn can_return_side(side: SideScreen, to: Screen) -> bool {
    let _ = side;
    to.is_side_lobby()
}

/// Sprite bodies one block names, in file order (empty for unknown
/// blocks).
fn block_sprite_names(list: &ControlList, block: &str) -> Vec<String> {
    scene_block(list, block).map_or_else(Vec::new, scene_sprite_names)
}

/// Sprite bodies a side screen needs, in block order.
///
/// Bound blocks resolve through their `ControlList` entries; the
/// stub-only tutorial resolves to its `Playing1.opi` backdrop, and
/// payment merges both billing views.
#[must_use]
pub fn side_sprite_names(list: &ControlList, side: SideScreen) -> Vec<String> {
    match side {
        SideScreen::Tutorial => vec![TUTORIAL_BACKGROUND_SPRITE.to_owned()],
        SideScreen::Payment => {
            let mut names = block_sprite_names(list, PAYMENT_BLOCK);
            for name in block_sprite_names(list, PAYMENT2_BLOCK) {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            names
        }
        _ => side
            .block_name()
            .map_or_else(Vec::new, |block| block_sprite_names(list, block)),
    }
}

/// Load side art from an OPI/OPA archive, skipping missing or
/// undecodable bodies.
#[must_use]
pub fn load_side_figures(archive: &[u8], names: &[String]) -> HashMap<String, AvatarFigure> {
    load_scene_figures(archive, names)
}

/// One item-shop row (`Itemdata_Korea.dat`): id, Korean name, price.
/// Type/param/flag words stay out: their semantics are still TBD
/// (`o2jam-file` docs), so unverified numbers never reach the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopItemView<'a> {
    /// Item id (1-based, table order).
    pub item_id: u32,
    /// Korean item name.
    pub name: &'a str,
    /// Price.
    pub price: u32,
}

/// Borrow one table row as a shop view.
fn shop_item_view(row: &AvatarItemRow) -> ShopItemView<'_> {
    ShopItemView {
        item_id: row.item_id,
        name: &row.name,
        price: row.price,
    }
}

/// Item-shop rows in table order; empty when the table is missing, so
/// the screen paints its art without rows.
#[must_use]
pub fn shop_items(table: &AvatarItemTable) -> Vec<ShopItemView<'_>> {
    table.rows.iter().map(shop_item_view).collect()
}

/// One music-shop song (`OJNList.dat`): id, title, artist, genre, tempo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MusicShopSong<'a> {
    /// Song id (matches the `o2ma###` file number).
    pub song_id: u32,
    /// Song title.
    pub title: &'a str,
    /// Song artist.
    pub artist: &'a str,
    /// Category id (`u32` at +16; raw value, semantics TBD).
    pub genre: u32,
    /// Tempo in BPM.
    pub bpm: f32,
}

/// Music-shop songs in file order, dead slots filtered out
/// ([`OjnListEntry::is_song`]); empty without catalog data.
#[must_use]
pub fn music_shop_songs<'a>(entries: &'a [OjnListEntry<'a>]) -> Vec<MusicShopSong<'a>> {
    entries
        .iter()
        .filter(|entry| entry.is_song())
        .map(|entry| MusicShopSong {
            song_id: entry.song_id,
            title: &entry.title,
            artist: &entry.artist,
            genre: entry.genre,
            bpm: entry.bpm,
        })
        .collect()
}

/// One music-shop row with live shop flags (`GetMusicList` 4031
/// answer): catalog identity from `OJNList.dat`, availability from
/// the server. No flag invents a song: empty catalog stays empty.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MusicShopListing<'a> {
    /// Catalog song.
    pub song: MusicShopSong<'a>,
    /// `FreeMusic` server option: every song plays free.
    pub free: bool,
    /// Server catalog carries this song id (chart data present).
    pub listed: bool,
}

/// Music-shop rows in catalog order with live flags; a missing
/// answer clears both flags without dropping rows.
#[must_use]
pub fn music_shop_listings<'a>(
    entries: &'a [OjnListEntry<'a>],
    answer: Option<&MusicListAnswer>,
) -> Vec<MusicShopListing<'a>> {
    music_shop_songs(entries)
        .into_iter()
        .map(|song| listing_for_song(song, answer))
        .collect()
}

/// Attach live flags to one catalog song.
fn listing_for_song<'a>(
    song: MusicShopSong<'a>,
    answer: Option<&MusicListAnswer>,
) -> MusicShopListing<'a> {
    let (free, listed) = shop_flags_for(song.song_id, answer);
    MusicShopListing { song, free, listed }
}

/// Read the global free flag plus per-song server presence
/// (`None` answer: no flags, rows still paint).
fn shop_flags_for(song_id: u32, answer: Option<&MusicListAnswer>) -> (bool, bool) {
    match answer {
        None => (false, false),
        Some(answer) => (answer.free_music, lists_song(answer, song_id)),
    }
}

/// Whether the server catalog carries this song id.
fn lists_song(answer: &MusicListAnswer, song_id: u32) -> bool {
    let Ok(wanted) = u16::try_from(song_id) else {
        return false;
    };
    answer.entries.iter().any(|entry| entry.id == wanted)
}

/// Song-select dialog block (`CDSelectMusic`, decl `0x7A`, 134 entries).
/// Spelling keeps the in-archive `DAILOG_` typo verbatim.
pub const SELECT_MUSIC_DIALOG_BLOCK: &str = "DAILOG_SELECTMUSIC";
/// Song-select dialog layout (133 rects).
pub const SELECT_MUSIC_BIND_FILE: &str = "Dialog_SelectMusic.bnd";

/// Sprite bodies the song-select dialog names, in block order.
/// Unknown blocks yield no names; the waiting panel then paints
/// its rows without dialog art.
#[must_use]
pub fn select_music_sprite_names(list: &ControlList) -> Vec<String> {
    block_sprite_names(list, SELECT_MUSIC_DIALOG_BLOCK)
}

// --- Song-select dialog structure (`reference-musicselect-4x3.png`) ---
//
// Anchors below measure the open `DAILOG_SELECTMUSIC` dialog on the
// 800x600 native canvas (the 4:3 reference PNG at 2.4 px per native
// pixel). Painters scale them through `scale_rect_to_frame`, so the
// structure keeps its reference proportions at any frame size over
// the staged dialog art.

/// Difficulty-slot letters for the search-result icon column:
/// `Ex`/`Nx`/`Hx` map to `E`/`N`/`H` (the Easy/Normal/Hard order the
/// life-bar deltas already use).
pub const SELECT_DIFFICULTY_LETTERS: [&str; OJN_LEVEL_COUNT] = ["E", "N", "H"];

/// Song-level band captions, top to bottom. The exe bands are
/// `초급 1–8` / `중급 8–19` / `고급 19–27` / `최고급 27+` (gameplay
/// level gating, file `0x24578e`, dossier `ev_68760cac…`); the CP949
/// captions draw as ASCII stand-ins (the micro-font has no hangul).
pub const SELECT_LEVEL_FILTERS: [&str; 5] = ["ALL", "EASY", "MID", "HIGH", "TOP"];

/// Wheel-bucket captions, top to bottom: the exe key-name table
/// GOLD/BLACK/BLUE/RED/SILVER (`docs/dialog-strings.md`, filter path
/// pinned) plus the ALL lead-in.
pub const SELECT_WHEEL_FILTERS: [&str; 6] = ["ALL", "GOLD", "BLACK", "BLUE", "RED", "SILVER"];

/// Genre captions, top to bottom (reference column 발라드/락/댄스/
/// 힙합,R&B/재즈,펑크/클래식/기타). No exe genre-name table is
/// pinned, so these are reference-transcribed stand-ins.
pub const SELECT_GENRE_FILTERS: [&str; 8] = [
    "ALL",
    "BALLAD",
    "ROCK",
    "DANCE",
    "HIPHOP/R&B",
    "JAZZ/FUNK",
    "CLASSIC",
    "ETC",
];

/// Search-result column captions (reference 라벨/노트/곡명/작곡가/
/// 레벨/다운/듣기/정보/성적; ASCII stand-ins).
pub const SELECT_TABLE_HEADERS: [&str; 9] = [
    "LABEL", "NOTE", "TITLE", "ARTIST", "LVL", "DL", "PLAY", "INFO", "GRADE",
];

/// Staged catalog song: the identity fields the search table names
/// (`OJNList.dat` record; owned copies so the catalog outlives the
/// parse borrow).
#[derive(Debug, Clone, PartialEq)]
pub struct SelectMusicSong {
    /// Song id (matches the `o2ma###` file number).
    pub song_id: u32,
    /// Song title.
    pub title: String,
    /// Song artist.
    pub artist: String,
}

/// Catalog songs in file order, dead slots filtered out exactly like
/// [`music_shop_songs`]; empty without catalog data.
#[must_use]
pub fn select_music_songs(entries: &[OjnListEntry<'_>]) -> Vec<SelectMusicSong> {
    entries
        .iter()
        .filter(|entry| entry.is_song())
        .map(|entry| SelectMusicSong {
            song_id: entry.song_id,
            title: entry.title.clone(),
            artist: entry.artist.clone(),
        })
        .collect()
}

/// One search-result row: one song at one difficulty slot. The
/// reference table repeats the song per chart (`H`/`N`/`E` letters);
/// the staged catalog carries [`OJN_LEVEL_COUNT`] level slots per
/// record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectMusicRow<'a> {
    /// Catalog song this row plays.
    pub song: &'a SelectMusicSong,
    /// Difficulty slot (`0..OJN_LEVEL_COUNT`).
    pub slot: usize,
}

impl SelectMusicRow<'_> {
    /// Icon-column letter ([`SELECT_DIFFICULTY_LETTERS`]).
    #[must_use]
    pub fn letter(self) -> &'static str {
        SELECT_DIFFICULTY_LETTERS[self.slot]
    }

    /// Per-chart level: never staged. The catalog's level word is
    /// range-shaped (`level_flags`, both halves 1..38), not per-chart,
    /// so the cell draws blank rather than an invented number.
    #[must_use]
    pub fn level(self) -> Option<u8> {
        None
    }

    /// Personal grade: never staged (no score table ships with the
    /// client), so the cell draws blank.
    #[must_use]
    pub fn grade(self) -> Option<u8> {
        None
    }
}

/// Search-result rows in catalog order, easy-first; empty without
/// songs. The reference client drops charts the install lacks; the
/// staged data has no availability source, so every slot draws
/// (documented gap).
#[must_use]
pub fn select_music_rows(songs: &[SelectMusicSong]) -> Vec<SelectMusicRow<'_>> {
    let mut rows = Vec::with_capacity(songs.len().saturating_mul(OJN_LEVEL_COUNT));
    for song in songs {
        for slot in 0..OJN_LEVEL_COUNT {
            rows.push(SelectMusicRow { song, slot });
        }
    }
    rows
}

/// Search-tab caption for the row count (reference `검색결과 (1608곡)`).
#[must_use]
pub fn select_search_caption(row_count: usize) -> String {
    format!("SEARCH RESULTS ({row_count})")
}

/// Picked-tab caption (reference `선곡결과 (0곡)`): no picked list is
/// staged, so the count stays zero.
#[must_use]
pub fn select_picked_caption() -> &'static str {
    "SELECTED (0)"
}

/// Status line under the rows: the exe's `%s -> %s 검색 결과입니다.`
/// banner format (VA `0x645a50` family, `docs/dialog-strings.md`) with
/// both filters at the ALL default — nothing stages narrower filters.
#[must_use]
pub fn select_status_line() -> &'static str {
    "ALL -> ALL SEARCH RESULT."
}

/// Pager caption `page/pages` (reference `3/161`): ten rows per page,
/// page 1 while no paging input is staged; an empty table still names
/// page 1.
#[must_use]
pub fn select_pager_caption(row_count: usize) -> String {
    let pages = row_count.div_ceil(SELECT_VISIBLE_ROWS).max(1);
    format!("1/{pages}")
}

/// Chosung search cells in reference order: 14 jamo, `A`..`Z`, then
/// the 숫자/기타 words (42 cells over two rows). The jamo are the
/// 14 simple initial consonants of the CP949 sort set (clusters
/// ㄲ/ㄸ/ㅃ/ㅆ/ㅉ excluded — they sort under their base). Word cells
/// stay `None` (no caption source) and keep their grid slots so the
/// letters land at their reference positions; every `Some` cell
/// rasterizes through the embedded TTF (the micro-font had no
/// CP949 glyphs — that documented gap is closed).
#[must_use]
pub fn select_chosung_cells() -> Vec<Option<char>> {
    const SORT_JAMO: [char; 14] = [
        'ㄱ', 'ㄴ', 'ㄷ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅅ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
    ];
    let mut cells: Vec<Option<char>> = SORT_JAMO.iter().copied().map(Some).collect();
    cells.extend(('A'..='Z').map(Some));
    for _ in 0..SELECT_CHO_SUNG_WORDS {
        cells.push(None);
    }
    cells
}

/// Difficulty-slot ink: easy green, normal yellow, hard red (the
/// reference letter colors).
#[must_use]
pub const fn select_difficulty_ink(slot: usize) -> u32 {
    match slot {
        0 => pack_xrgb(0x30, 0xE0, 0x30),
        1 => pack_xrgb(0xFF, 0xD0, 0x20),
        _ => pack_xrgb(0xFF, 0x50, 0x50),
    }
}

/// Filter-column panels (native pixels): song-level, wheel, and genre
/// columns left to right, each spanning its button run.
pub const SELECT_FILTER_COLUMNS: [Rect; 3] = [
    Rect {
        x: 28,
        y: 95,
        w: 96,
        h: 300,
    },
    Rect {
        x: 130,
        y: 95,
        w: 92,
        h: 300,
    },
    Rect {
        x: 230,
        y: 95,
        w: 100,
        h: 300,
    },
];

/// First filter button top (native y), its vertical pitch, and button
/// height (reference column buttons).
pub const SELECT_FILTER_BUTTON_TOP: i32 = 129;
pub const SELECT_FILTER_BUTTON_PITCH: i32 = 33;
pub const SELECT_FILTER_BUTTON_H: u32 = 26;

/// Search-result tabs (native pixels): active search tab, picked tab.
pub const SELECT_RESULT_TABS: [Rect; 2] = [
    Rect {
        x: 352,
        y: 93,
        w: 140,
        h: 24,
    },
    Rect {
        x: 494,
        y: 93,
        w: 127,
        h: 24,
    },
];

/// Search-table geometry (native pixels): header top, first row top,
/// row pitch, visible rows (the reference shows ten full rows), and
/// the horizontal band all rows share (left edge, width).
pub const SELECT_HEADER_Y: i32 = 127;
pub const SELECT_ROW_TOP: i32 = 150;
pub const SELECT_ROW_PITCH: i32 = 23;
pub const SELECT_VISIBLE_ROWS: usize = 10;
pub const SELECT_TABLE_LEFT: i32 = 344;
pub const SELECT_TABLE_WIDTH: u32 = 437;

/// Search-table row under a native-space click (`None` outside the
/// table band or past the staged rows). Inverse of the
/// [`SELECT_ROW_TOP`]/[`SELECT_ROW_PITCH`] bands; mouse clicks arrive
/// pre-mapped to native space. Only the first [`SELECT_VISIBLE_ROWS`]
/// rows paint, so only they answer.
#[must_use]
pub fn select_row_hit(x: i32, y: i32, row_count: usize) -> Option<usize> {
    let width = i32::try_from(SELECT_TABLE_WIDTH).unwrap_or(0);
    if !(SELECT_TABLE_LEFT..SELECT_TABLE_LEFT + width).contains(&x) {
        return None;
    }
    let slot = y - SELECT_ROW_TOP;
    if slot < 0 {
        return None;
    }
    let index = usize::try_from(slot / SELECT_ROW_PITCH).unwrap_or(usize::MAX);
    (index < row_count.min(SELECT_VISIBLE_ROWS)).then_some(index)
}

/// Header column lefts (native x): the check box plus the nine
/// [`SELECT_TABLE_HEADERS`] captions.
pub const SELECT_HEADER_COLUMNS: [i32; 10] = [344, 360, 392, 417, 565, 627, 650, 673, 692, 715];

/// Status line top and strip height (native pixels), plus the pager
/// text's right edge.
pub const SELECT_STATUS_Y: i32 = 394;
pub const SELECT_STATUS_H: i32 = 18;
pub const SELECT_PAGER_RIGHT: i32 = 756;

/// Search row (native pixels): title/artist radio dots, edit box,
/// search button, all below [`SELECT_SEARCH_Y`].
pub const SELECT_SEARCH_Y: i32 = 417;
pub const SELECT_TITLE_RADIO: Rect = Rect {
    x: 352,
    y: 423,
    w: 8,
    h: 8,
};
pub const SELECT_ARTIST_RADIO: Rect = Rect {
    x: 410,
    y: 423,
    w: 8,
    h: 8,
};
pub const SELECT_SEARCH_BOX: Rect = Rect {
    x: 475,
    y: 417,
    w: 212,
    h: 25,
};
pub const SELECT_SEARCH_BUTTON: Rect = Rect {
    x: 694,
    y: 417,
    w: 38,
    h: 25,
};

/// Chosung grid row tops and left edge (native pixels).
/// CANDIDATE — `docs/geometry-audit.md` (offsets diverge from
/// `Dialog_SelectMusic.bnd` r87–128: tops 444/462 vs file 440/455,
/// left 352 vs file 345); rig follow-up, do not replace.
pub const SELECT_CHO_SUNG_Y: [i32; 2] = [444, 462];
pub const SELECT_CHO_SUNG_LEFT: i32 = 352;
/// Chosung cell pitch (native pixels). FILE per the audit: the x-step
/// of the INDEX button rects (r87–128) is exactly 15. Fallback for
/// stages without dialog placements; the painter prefers
/// [`select_chosung_layout`].
pub const SELECT_CHO_SUNG_PITCH: i32 = 15;
/// Chosung jamo/letter/word inventory. FILE per the audit: exactly
/// the `Dialog_SelectMusic` INDEX-button inventory (14 jamo r87–100,
/// 13+13 letters, 2 words r127–128).
pub const SELECT_CHO_SUNG_JAMO: usize = 14;
pub const SELECT_CHO_SUNG_WORDS: usize = 2;
/// Chosung row cell counts derived from the jamo/letter/word
/// inventory (FILE values; [`select_chosung_layout`] recounts them
/// from staged placements at runtime).
pub const SELECT_CHO_SUNG_ROW_CELLS: [usize; 2] =
    [SELECT_CHO_SUNG_JAMO + 13, 13 + SELECT_CHO_SUNG_WORDS];

/// Chosung grid geometry counted from staged dialog placements: the
/// INDEX-button inventory (`IDC_BUTTON_DSMUSIC_SERACH_INDEX*`,
/// ordinals 87–128 of `Dialog_SelectMusic.bnd`) gives the row cell
/// counts and the x-pitch — both FILE per `docs/geometry-audit.md`.
/// `None` when no INDEX button stages: callers keep the audited
/// constants above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectChosungLayout {
    /// Painted cells per grid row, top row first.
    pub row_cells: Vec<usize>,
    /// X pitch between adjacent cells (native pixels).
    pub pitch: i32,
}

/// Count the chosung grid from the staged INDEX buttons: group them
/// by rect top (row), sort each row left to right, and take the
/// smallest positive x-step as the pitch.
#[must_use]
pub fn select_chosung_layout(placements: &[ScenePlacement]) -> Option<SelectChosungLayout> {
    let index_rects = index_button_rects(placements);
    if index_rects.is_empty() {
        return None;
    }
    let rows = rows_by_top(index_rects);
    let pitch = rows.iter().filter_map(|row| row_pitch(row)).min()?;
    Some(SelectChosungLayout {
        row_cells: rows.iter().map(Vec::len).collect(),
        pitch,
    })
}

/// Rects of the staged chosung INDEX buttons (ordinals 87–128).
fn index_button_rects(placements: &[ScenePlacement]) -> Vec<Rect> {
    placements
        .iter()
        .filter(|placement| {
            let ordinal = (placement.code & 0xFFF).saturating_sub(1);
            (87..=128).contains(&ordinal)
        })
        .map(|placement| placement.rect)
        .collect()
}

/// Group row rects by their top edge, rows sorted top-to-bottom and
/// each row's rects sorted left-to-right.
fn rows_by_top(mut rects: Vec<Rect>) -> Vec<Vec<Rect>> {
    rects.sort_by_key(|rect| (rect.y, rect.x));
    let mut rows: Vec<Vec<Rect>> = Vec::new();
    for rect in rects {
        match rows.last_mut() {
            Some(row) if row[0].y == rect.y => row.push(rect),
            _ => rows.push(vec![rect]),
        }
    }
    rows
}

/// Smallest positive x-step between adjacent cells of one row.
fn row_pitch(row: &[Rect]) -> Option<i32> {
    row.windows(2)
        .map(|pair| pair[1].x - pair[0].x)
        .filter(|step| *step > 0)
        .min()
}

/// OK button (native pixels; the reference bottom-center pill).
pub const SELECT_OK_BUTTON: Rect = Rect {
    x: 342,
    y: 506,
    w: 113,
    h: 40,
};

/// Catalog title for one song id (`None` for dead slots and unknown ids).
/// Dead-slot filtering matches [`music_shop_songs`]: only song entries resolve.
#[must_use]
pub fn waiting_song_title<'a>(entries: &'a [OjnListEntry<'a>], music_id: u16) -> Option<&'a str> {
    entries
        .iter()
        .find(|entry| is_waiting_song(entry, music_id))
        .map(|entry| entry.title.as_str())
}

/// Whether one catalog entry is the wanted waiting song.
fn is_waiting_song(entry: &OjnListEntry<'_>, music_id: u16) -> bool {
    entry.is_song() && entry.song_id == u32::from(music_id)
}

/// Difficulty label for the waiting song panel (`EX`/`NX`/`HX`/`RX`).
#[must_use]
pub fn waiting_song_difficulty_label(difficulty: u8) -> String {
    match Difficulty::from_code(difficulty) {
        Some(Difficulty::Ex) => "EX".to_owned(),
        Some(Difficulty::Nx) => "NX".to_owned(),
        Some(Difficulty::Hx) => "HX".to_owned(),
        Some(Difficulty::Rx) => "RX".to_owned(),
        None => format!("DIF {difficulty}"),
    }
}

/// Scroll-speed label for the waiting song panel (`X0.5`..`X6`, `XR`).
#[must_use]
pub fn waiting_song_speed_label(speed: u8) -> String {
    match GameSpeed::from_code(speed) {
        Some(GameSpeed::X05) => "X0.5".to_owned(),
        Some(GameSpeed::X10) => "X1".to_owned(),
        Some(GameSpeed::X15) => "X1.5".to_owned(),
        Some(GameSpeed::X20) => "X2".to_owned(),
        Some(GameSpeed::X25) => "X2.5".to_owned(),
        Some(GameSpeed::X30) => "X3".to_owned(),
        Some(GameSpeed::X35) => "X3.5".to_owned(),
        Some(GameSpeed::X40) => "X4".to_owned(),
        Some(GameSpeed::X45) => "X4.5".to_owned(),
        Some(GameSpeed::X50) => "X5".to_owned(),
        Some(GameSpeed::X60) => "X6".to_owned(),
        Some(GameSpeed::Xr) => "XR".to_owned(),
        None => format!("SPD {speed}"),
    }
}

/// Title row for the waiting song panel: chosen id plus catalog title.
#[must_use]
pub fn waiting_song_title_line(music_id: u16, title: &str) -> String {
    format!("M{music_id} {title}")
}

/// Detail row for the waiting song panel: difficulty plus speed.
#[must_use]
pub fn waiting_song_detail_line(selection: &SetRoomMusic) -> String {
    format!(
        "{} {}",
        waiting_song_difficulty_label(selection.difficulty),
        waiting_song_speed_label(selection.speed)
    )
}

/// Equipped my-room rows in slot order, skipping zero and unknown ids
/// exactly like [`load_worn_figure`](super::super::avatar::load_worn_figure)
/// skips unresolvable pieces: one bad id never blanks the room.
#[must_use]
pub fn my_room_equipment<'a>(
    table: &'a AvatarItemTable,
    equipment: &[i32],
) -> Vec<ShopItemView<'a>> {
    equipment
        .iter()
        .filter_map(|id| equipped_item_view(table, *id))
        .collect()
}

/// Borrow one equipped id as a shop view (`None` for zero/unknown ids).
fn equipped_item_view(table: &AvatarItemTable, id: i32) -> Option<ShopItemView<'_>> {
    let item_id = u32::try_from(id).ok()?;
    if item_id == 0 {
        return None;
    }
    avatar_item_by_id(table, item_id).map(shop_item_view)
}

/// My-room rows from fetched character data (`GetCharacterInfo`
/// 2001): the server's 17-slot equipment table in slot order,
/// skipping zero and unknown ids exactly like [`my_room_equipment`].
#[must_use]
pub fn my_room_equipment_from_character<'a>(
    table: &'a AvatarItemTable,
    character: &CharacterInfo,
) -> Vec<ShopItemView<'a>> {
    my_room_equipment(table, &character.equipment)
}

// --- Avatar-room (MyRoom) interaction ---
//
// Control codes from the shipped `STATE_MYROOM` block dump (file
// order, `docs/myroom.md`): 11 art-less slot selectors
// (`0X15200005..0F`), 10 bag images (`0X15600023..2C`), the bag
// pager pair, the gift pager pair plus 5 gift-box cells, and the
// BACK/GOSHOP/VACATE/INVENTORY buttons. Caption codes are the
// `0X1540xxxx` TEXT controls of the same block.

/// First slot-selector button code (`IDC_BUTTON_SMYROOM_INSTRUMENT`).
pub const MYROOM_SLOT_FIRST_CODE: u32 = 0x1520_0005;
/// Slot-selector button count (instrument through shoes).
pub const MYROOM_SLOT_BUTTONS: usize = 11;
/// Slot count as `u32` for code arithmetic without casts.
pub const MYROOM_SLOT_BUTTONS_U32: u32 = 11;
/// Slot button names in code order (ASCII stand-ins from the IDC
/// ids; the exe's Korean slot comments have no pinned string source).
pub const MYROOM_SLOT_NAMES: [&str; MYROOM_SLOT_BUTTONS] = [
    "INSTRUMENT",
    "HAIR",
    "ARTICLE",
    "GLOVES",
    "NECKLACE",
    "JACKET",
    "PANTS",
    "GLASSES",
    "EARRING",
    "CLOTHINGART",
    "SHOES",
];
/// First bag-cell image code (`IDC_IMAGE_SMYROOM_MYBAGITEM1`).
pub const MYROOM_BAG_FIRST_CODE: u32 = 0x1560_0023;
/// Bag cells per page (10 `MYBAGITEM` images).
pub const MYROOM_BAG_CELLS: usize = 10;
/// Bag pager LEFT button (`Shop_Btn_BagLeft.ojs`).
pub const MYROOM_BAG_LEFT_CODE: u32 = 0x1520_002D;
/// Bag pager RIGHT button (`Shop_Btn_BagRight.ojs`).
pub const MYROOM_BAG_RIGHT_CODE: u32 = 0x1520_002E;
/// First gift-box cell code (`IDC_BUTTON_SMYROOM_GIFTBOX_EX1`).
pub const MYROOM_GIFT_FIRST_CODE: u32 = 0x1501_C04A;
/// Gift-box cells per page (5 `GIFTBOX_EX` buttons).
pub const MYROOM_GIFT_CELLS: usize = 5;
/// Gift pager LEFT button (`Myroom_Btn_GiftLeft.ojs`).
pub const MYROOM_GIFT_LEFT_CODE: u32 = 0x1520_0048;
/// Gift pager RIGHT button (`Myroom_Btn_GiftRight.ojs`).
pub const MYROOM_GIFT_RIGHT_CODE: u32 = 0x1520_0049;
/// Gift claim button (`Myroom_Btn_GiftMove.ojs`: moves the
/// selected gift into the bag). The control choice is
/// ours-by-design — no exe click sequence is pinned — while the
/// queued `ClaimGift` 5023 shape is exe-pinned (`docs/gifts.md`);
/// next step: rig walk of the gift-box claim gesture.
pub const MYROOM_MOVE_CODE: u32 = 0x1550_0053;
/// BACK button (`Myroom_Back.ojs`): returns to the opening lobby.
pub const MYROOM_BACK_CODE: u32 = 0x1520_0002;
/// GOSHOP button (`Myroom_GoShop.ojs`): opens the item shop.
/// Records only — nested sides stay rejected, so the hop is OPEN
/// with a leave-then-visit hand step.
pub const MYROOM_GOSHOP_CODE: u32 = 0x1520_0003;
/// Vacate button (`MyRoom_Btn_Vacate.ojs`): clears the slot.
/// Records only — the vacate send shape is unpinned (OPEN).
pub const MYROOM_VACATE_CODE: u32 = 0x1520_0004;
/// INVENTORY button (`Myroom_Inventory.ojs`): commits the
/// selected bag cell (equip-apply). The control choice is
/// ours-by-design — the exe send sites (`0xa4af4` shop-control,
/// `0xa4bcb` apply, `docs/equip.md`) name no control — while the
/// queued `EquipItem` 5004 shape and slot derivation are
/// exe-pinned; next step: rig walk of the myroom apply gesture.
pub const MYROOM_APPLY_CODE: u32 = 0x1520_3034;
/// Nickname caption (`IDC_TEXT_SMYROOM_NICKNAME`).
pub const MYROOM_NICKNAME_CODE: u32 = 0x1540_0037;
/// Level caption (`IDC_TEXT_SMYROOM_LEVEL`).
pub const MYROOM_LEVEL_CODE: u32 = 0x1540_0038;
/// Bonus-point caption (`IDC_TEXT_SMYROOM_BONUSPOINT`).
pub const MYROOM_BONUS_CODE: u32 = 0x1540_0039;
/// Experience caption (`IDC_TEXT_SMYROOM_EXP`).
pub const MYROOM_EXP_CODE: u32 = 0x1540_003A;
/// Penalty-count caption (`IDC_TEXT_SMYROOM_PENALTYCOUNT`).
pub const MYROOM_PENALTY_COUNT_CODE: u32 = 0x1540_0066;
/// Penalty-level caption (`IDC_TEXT_SMYROOM_PENALTYLEVEL`).
pub const MYROOM_PENALTY_LEVEL_CODE: u32 = 0x1540_0067;
/// O2-cash caption (`IDC_TEXT_SMYROOM_O2CASH`).
pub const MYROOM_O2CASH_CODE: u32 = 0x1540_0059;
/// Music-cash caption (`IDC_TEXT_SMYROOM_MUSICCASH`).
pub const MYROOM_MUSICCASH_CODE: u32 = 0x1540_005A;
/// Item-cash caption (`IDC_TEXT_SMYROOM_ITEMCASH`).
pub const MYROOM_ITEMCASH_CODE: u32 = 0x1540_005B;
/// Gift-count caption (`IDC_TEXT_GIFT_NUM`).
pub const MYROOM_GIFT_NUM_CODE: u32 = 0x1540_004F;
/// Gift resource-name caption (`IDC_TEXT_GIFT_ITEMNAME`).
pub const MYROOM_GIFT_ITEMNAME_CODE: u32 = 0x1540_0051;
/// Gift sender caption (`IDC_TEXT_GIFT_SENDER`).
pub const MYROOM_GIFT_SENDER_CODE: u32 = 0x1540_0052;

/// One pager step: which end of the row run the press moves toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MyRoomStep {
    /// LEFT pager button: toward the first rows.
    Previous,
    /// RIGHT pager button: toward the last rows.
    Next,
}

/// One avatar-room control press: the code → action dispatch output.
/// Board input routes here, never through per-button literals at
/// the call site. Geometry never lives here: it rides the staged
/// `.bnd` placements (owner rule `f38f3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MyRoomAction {
    /// Slot-selector button pressed: button ordinal `0..11`.
    /// What the selection filters stays OPEN (`docs/myroom.md`).
    SelectSlot(u8),
    /// Bag pager pressed: turn the owned-item page.
    BagPage(MyRoomStep),
    /// Bag cell pressed: cell index within the page `0..10`.
    PickBagCell(usize),
    /// Gift pager pressed: turn the gift-box page.
    GiftPage(MyRoomStep),
    /// Gift cell pressed: cell index within the page `0..5`.
    PickGiftCell(usize),
    /// INVENTORY pressed: equip the selected bag cell.
    ApplyEquip,
    /// Vacate pressed: records only (send shape OPEN).
    VacateSlot,
    /// BACK pressed: return to the opening lobby.
    BackToLobby,
    /// GOSHOP pressed: records only (nested sides rejected).
    OpenShop,
    /// Gift MOVE pressed: claim the selected gift cell.
    ClaimGift,
}

/// Action for one avatar-room control code (`None` for caption and
/// image codes: text and slot art are not buttons).
#[must_use]
pub fn myroom_action_for_code(code: u32) -> Option<MyRoomAction> {
    if (MYROOM_SLOT_FIRST_CODE..MYROOM_SLOT_FIRST_CODE + MYROOM_SLOT_BUTTONS_U32).contains(&code) {
        let slot = u8::try_from(code - MYROOM_SLOT_FIRST_CODE).ok()?;
        return Some(MyRoomAction::SelectSlot(slot));
    }
    if (MYROOM_BAG_FIRST_CODE..MYROOM_BAG_FIRST_CODE + MYROOM_BAG_CELLS_U32).contains(&code) {
        let cell = usize::try_from(code - MYROOM_BAG_FIRST_CODE).ok()?;
        return Some(MyRoomAction::PickBagCell(cell));
    }
    if (MYROOM_GIFT_FIRST_CODE..MYROOM_GIFT_FIRST_CODE + MYROOM_GIFT_CELLS_U32).contains(&code) {
        let cell = usize::try_from(code - MYROOM_GIFT_FIRST_CODE).ok()?;
        return Some(MyRoomAction::PickGiftCell(cell));
    }
    match code {
        MYROOM_BAG_LEFT_CODE => Some(MyRoomAction::BagPage(MyRoomStep::Previous)),
        MYROOM_BAG_RIGHT_CODE => Some(MyRoomAction::BagPage(MyRoomStep::Next)),
        MYROOM_GIFT_LEFT_CODE => Some(MyRoomAction::GiftPage(MyRoomStep::Previous)),
        MYROOM_GIFT_RIGHT_CODE => Some(MyRoomAction::GiftPage(MyRoomStep::Next)),
        MYROOM_APPLY_CODE => Some(MyRoomAction::ApplyEquip),
        MYROOM_VACATE_CODE => Some(MyRoomAction::VacateSlot),
        MYROOM_BACK_CODE => Some(MyRoomAction::BackToLobby),
        MYROOM_GOSHOP_CODE => Some(MyRoomAction::OpenShop),
        MYROOM_MOVE_CODE => Some(MyRoomAction::ClaimGift),
        _ => None,
    }
}

/// Bag-cell count as `u32` for code arithmetic without casts.
const MYROOM_BAG_CELLS_U32: u32 = 10;
/// Gift-cell count as `u32` for code arithmetic without casts.
const MYROOM_GIFT_CELLS_U32: u32 = 5;

/// Avatar-room action under native pixel (`x`, `y`): the staged
/// `.bnd` placement whose code dispatches and whose rect contains
/// the point. Cell indices count the hit placement among its own
/// family in staged order (file order on staged art), so the painter
/// and the clicker share the same cells with no literals. `None`
/// outside every dispatching placement — unstaged clicks land
/// nowhere (no fallback literals, ADR-0003).
#[must_use]
pub fn myroom_action_at(placements: &[ScenePlacement], x: i32, y: i32) -> Option<MyRoomAction> {
    let mut bag_seen = 0usize;
    let mut gift_seen = 0usize;
    for placed in placements {
        let Some(action) = myroom_action_for_code(placed.code) else {
            continue;
        };
        let indexed = match action {
            MyRoomAction::PickBagCell(_) => {
                let index = bag_seen;
                bag_seen += 1;
                MyRoomAction::PickBagCell(index)
            }
            MyRoomAction::PickGiftCell(_) => {
                let index = gift_seen;
                gift_seen += 1;
                MyRoomAction::PickGiftCell(index)
            }
            _ => action,
        };
        if super::paint::rect_hit(&placed.rect, x, y) {
            return Some(indexed);
        }
    }
    None
}

/// Equipment slot for one item record (`docs/equip.md` VERIFIED
/// static sites: send-builder file `0xa4ec8`, derivation files
/// `0xa3cc9`/`0xa3cf0`/`0xa3cfd`, range check files
/// `0xa4baa..0xa4bb4`): type 5 forces slot 11, otherwise the record
/// slot byte holds, and the final slot must land below 17.
#[must_use]
pub const fn myroom_equip_slot(item_type: u32, record_slot: u8) -> Option<u8> {
    let slot = if (item_type & 0xFF) == 5 {
        11
    } else {
        record_slot
    };
    if slot < 17 {
        Some(slot)
    } else {
        None
    }
}

/// Record slot byte behind one parsed item row: `Itemdata_Korea.dat`
/// file byte +0x18, the top byte of the third param word (file bytes
/// 21..24 of the 30-byte prefix). The file → in-memory record layout
/// join is ASSUMED — the exe reads come from the looked-up record
/// (`0x52dba0`, `docs/equip.md`) — while the pure rule above is
/// exe-pinned; next step: REA trace of the lookup showing the record
/// layout.
#[must_use]
pub fn myroom_record_slot(row: &AvatarItemRow) -> u8 {
    let word = row.params[2];
    u8::try_from((word >> 24) & 0xFF).unwrap_or(0)
}

/// Live avatar-room selection: slot pick, bag page/cursor, gift
/// page/cursor. Slot semantics stay OPEN; the state only records
/// and clamps so painter and clicker share one selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MyRoomState {
    /// Selected slot-selector button ordinal `0..11`.
    selected_slot: u8,
    /// Visible bag page (clamped to the staged page count).
    bag_page: usize,
    /// Selected bag cell within the page (`None` until picked).
    bag_cell: Option<usize>,
    /// Visible gift page (clamped to the staged page count).
    gift_page: usize,
    /// Selected gift cell within the page (`None` until picked).
    gift_cell: Option<usize>,
}

impl Default for MyRoomState {
    /// Cleared selection (same as [`MyRoomState::new`]).
    fn default() -> Self {
        Self::new()
    }
}

impl MyRoomState {
    /// Cleared selection: first slot, first pages, no cursors.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected_slot: 0,
            bag_page: 0,
            bag_cell: None,
            gift_page: 0,
            gift_cell: None,
        }
    }

    /// Selected slot-selector button ordinal.
    #[must_use]
    pub const fn selected_slot(self) -> u8 {
        self.selected_slot
    }

    /// Visible bag page.
    #[must_use]
    pub const fn bag_page(self) -> usize {
        self.bag_page
    }

    /// Selected bag cell within the page, if any.
    #[must_use]
    pub const fn bag_cell(self) -> Option<usize> {
        self.bag_cell
    }

    /// Visible gift page.
    #[must_use]
    pub const fn gift_page(self) -> usize {
        self.gift_page
    }

    /// Selected gift cell within the page, if any.
    #[must_use]
    pub const fn gift_cell(self) -> Option<usize> {
        self.gift_cell
    }

    /// Record a slot-button press; past the eleventh button the
    /// press lands nowhere (button ordinals are `0..11`).
    pub fn select_slot(&mut self, slot: u8) {
        if usize::from(slot) < MYROOM_SLOT_BUTTONS {
            self.selected_slot = slot;
        }
    }

    /// Turn the bag page one step, clamped to `pages` (at least the
    /// first page paints, so an empty count still holds page 0).
    /// Turning clears the cell cursor: rows change under it.
    pub fn turn_bag_page(&mut self, step: MyRoomStep, pages: usize) {
        let last = pages.saturating_sub(1);
        match step {
            MyRoomStep::Previous => {
                self.bag_page = self.bag_page.saturating_sub(1);
            }
            MyRoomStep::Next => {
                self.bag_page = self.bag_page.saturating_add(1).min(last);
            }
        }
        self.bag_cell = None;
    }

    /// Record a bag-cell pick (`None` clears the cursor).
    pub fn pick_bag_cell(&mut self, cell: Option<usize>) {
        if cell.is_none_or(|index| index < MYROOM_BAG_CELLS) {
            self.bag_cell = cell;
        }
    }

    /// Turn the gift page one step, clamped like the bag pager.
    /// Turning clears the gift cursor.
    pub fn turn_gift_page(&mut self, step: MyRoomStep, pages: usize) {
        let last = pages.saturating_sub(1);
        match step {
            MyRoomStep::Previous => {
                self.gift_page = self.gift_page.saturating_sub(1);
            }
            MyRoomStep::Next => {
                self.gift_page = self.gift_page.saturating_add(1).min(last);
            }
        }
        self.gift_cell = None;
    }

    /// Record a gift-cell pick (`None` clears the cursor).
    pub fn pick_gift_cell(&mut self, cell: Option<usize>) {
        if cell.is_none_or(|index| index < MYROOM_GIFT_CELLS) {
            self.gift_cell = cell;
        }
    }
}

/// Bag pages over owned rows: at least page 0 paints, so an empty
/// bag still holds its first (blank) page.
#[must_use]
pub fn myroom_bag_pages(inventory: &[i32], table: &AvatarItemTable) -> usize {
    myroom_owned_rows(table, inventory)
        .len()
        .div_ceil(MYROOM_BAG_CELLS)
        .max(1)
}

/// Owned bag rows in inventory order, skipping zero and unknown ids
/// exactly like [`my_room_equipment`]: one bad id never blanks the bag.
fn myroom_owned_rows<'a>(table: &'a AvatarItemTable, inventory: &[i32]) -> Vec<ShopItemView<'a>> {
    inventory
        .iter()
        .filter_map(|id| equipped_item_view(table, *id))
        .collect()
}

/// Visible bag rows: the owned run sliced to one page of
/// [`MYROOM_BAG_CELLS`] cells. Past the last page paints nothing.
#[must_use]
pub fn myroom_bag_rows<'a>(
    table: &'a AvatarItemTable,
    inventory: &[i32],
    page: usize,
) -> Vec<ShopItemView<'a>> {
    let owned = myroom_owned_rows(table, inventory);
    let start = page.saturating_mul(MYROOM_BAG_CELLS);
    owned
        .into_iter()
        .skip(start)
        .take(MYROOM_BAG_CELLS)
        .collect()
}

/// One gift-box row: item and song gifts share the page run (items
/// first — order is ours-by-design, no exe sequence is pinned).
/// Names resolve from parsed data (item table, song catalog);
/// unknown ids skip exactly like the equipment rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MyRoomGiftView {
    /// Gift id (`GiftItemInfo`/`GiftMusicInfo`).
    pub gift_id: i32,
    /// Item id or song id being claimed.
    pub resource_id: i32,
    /// Resolved resource name (owned: item table and song catalog
    /// are different sources with different borrows).
    pub name: String,
    /// Gifter nickname.
    pub sender: String,
    /// True for song gifts, false for item gifts.
    pub is_music: bool,
}

/// Gift-box rows in box order (item gifts first, then song gifts)
/// from fetched character data (`GetCharacterInfo` 2001).
#[must_use]
pub fn myroom_gift_views(
    table: &AvatarItemTable,
    songs: &[SelectMusicSong],
    character: &CharacterInfo,
) -> Vec<MyRoomGiftView> {
    let mut views = Vec::new();
    for gift in &character.item_gifts {
        if let Some(view) = gift_item_view(table, gift) {
            views.push(view);
        }
    }
    for gift in &character.music_gifts {
        if let Some(view) = gift_music_view(songs, gift) {
            views.push(view);
        }
    }
    views
}

/// One item gift resolved against the item table (`None` for
/// unknown ids — one bad id never blanks the box).
fn gift_item_view(table: &AvatarItemTable, gift: &GiftItemInfo) -> Option<MyRoomGiftView> {
    let item_id = u32::try_from(gift.item_id).ok()?;
    if item_id == 0 {
        return None;
    }
    let row = avatar_item_by_id(table, item_id)?;
    Some(MyRoomGiftView {
        gift_id: gift.gift_id,
        resource_id: gift.item_id,
        name: row.name.clone(),
        sender: gift.sender.clone(),
        is_music: false,
    })
}

/// One song gift resolved against the staged song catalog (`None`
/// for unlisted songs).
fn gift_music_view(songs: &[SelectMusicSong], gift: &GiftMusicInfo) -> Option<MyRoomGiftView> {
    let wanted = u32::try_from(gift.music_id).ok()?;
    let song = songs.iter().find(|song| song.song_id == wanted)?;
    Some(MyRoomGiftView {
        gift_id: gift.gift_id,
        resource_id: gift.music_id,
        name: song.title.clone(),
        sender: gift.sender.clone(),
        is_music: true,
    })
}

/// Gift-box pages over the staged box run (first page paints even
/// when the box is empty).
#[must_use]
pub fn myroom_gift_pages(views: &[MyRoomGiftView]) -> usize {
    views.len().div_ceil(MYROOM_GIFT_CELLS).max(1)
}

/// Visible gift-box rows: the box run sliced to one page of
/// [`MYROOM_GIFT_CELLS`] cells.
#[must_use]
pub fn myroom_gift_page(views: &[MyRoomGiftView], page: usize) -> &[MyRoomGiftView] {
    let start = page.saturating_mul(MYROOM_GIFT_CELLS);
    let rows = views.get(start..).unwrap_or(&[]);
    rows.get(..rows.len().min(MYROOM_GIFT_CELLS)).unwrap_or(&[])
}

/// Identity captions from fetched character data: every field maps
/// by name identity to its TEXT control (nickname, level,
/// bonus point, experience, penalties, cash wallets). Money
/// controls without a name-identical field (`MGAME_MONEY`,
/// `O2JAM_MONEY`, `O2MONEY`, `CHARGEMONEY`), rank, guild, win-rate
/// and skill counts stay unpainted — their mapping is OPEN
/// (`docs/myroom.md`).
#[must_use]
pub fn myroom_identity_captions(character: &CharacterInfo) -> Vec<(u32, String)> {
    vec![
        (MYROOM_NICKNAME_CODE, character.nickname.clone()),
        (MYROOM_LEVEL_CODE, character.level.to_string()),
        (MYROOM_BONUS_CODE, character.bonus_point.to_string()),
        (MYROOM_EXP_CODE, character.experience.to_string()),
        (
            MYROOM_PENALTY_COUNT_CODE,
            character.penalty_count.to_string(),
        ),
        (
            MYROOM_PENALTY_LEVEL_CODE,
            character.penalty_level.to_string(),
        ),
        (MYROOM_O2CASH_CODE, character.o2_cash.to_string()),
        (MYROOM_MUSICCASH_CODE, character.music_cash.to_string()),
        (MYROOM_ITEMCASH_CODE, character.item_cash.to_string()),
    ]
}

/// Gift captions for the staged box run: the gift count plus the
/// selected cell's resource name and sender (blank until picked).
/// The cell counts within the visible `page`, matching the painted
/// cells. The count format is ours-by-design (no exe caption bytes
/// pinned).
#[must_use]
pub fn myroom_gift_captions(
    views: &[MyRoomGiftView],
    page: usize,
    cell: Option<usize>,
) -> Vec<(u32, String)> {
    let picked = cell.and_then(|index| myroom_gift_page(views, page).get(index));
    vec![
        (MYROOM_GIFT_NUM_CODE, views.len().to_string()),
        (
            MYROOM_GIFT_ITEMNAME_CODE,
            picked.map_or_else(String::new, |view| view.name.clone()),
        ),
        (
            MYROOM_GIFT_SENDER_CODE,
            picked.map_or_else(String::new, |view| view.sender.clone()),
        ),
    ]
}

/// Solo-mission difficulty band: one per `.ojn` level slot (the
/// lifecycle §12 HARD/NORMAL/EASY buttons; `FREE` picks any song).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanetBand {
    /// Level slot 0.
    Easy,
    /// Level slot 1.
    Normal,
    /// Level slot 2.
    Hard,
}

impl PlanetBand {
    /// Button label (lifecycle §12).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Easy => "EASY",
            Self::Normal => "NORMAL",
            Self::Hard => "HARD",
        }
    }

    /// `.ojn` level slot this band plays.
    #[must_use]
    pub const fn level(self) -> usize {
        match self {
            Self::Easy => 0,
            Self::Normal => 1,
            Self::Hard => 2,
        }
    }

    /// Band for a `.ojn` level slot (`None` outside `0..3`).
    #[must_use]
    pub const fn of_level(level: usize) -> Option<Self> {
        match level {
            0 => Some(Self::Easy),
            1 => Some(Self::Normal),
            2 => Some(Self::Hard),
            _ => None,
        }
    }
}

/// Bands in button order (one per `.ojn` level slot).
pub const PLANET_BANDS: [PlanetBand; OJN_LEVEL_COUNT] =
    [PlanetBand::Easy, PlanetBand::Normal, PlanetBand::Hard];
/// Free-pick button label (lifecycle §12): any song, any band.
pub const PLANET_FREE_LABEL: &str = "FREE";

/// One planet mission: a song offered at one difficulty band.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetMission<'a> {
    /// Difficulty band.
    pub band: PlanetBand,
    /// Song id.
    pub song_id: u32,
    /// Song title.
    pub title: &'a str,
}

/// Planet missions in song order, bands easy-first; empty without
/// catalog data.
#[must_use]
pub fn planet_missions<'a>(entries: &'a [OjnListEntry<'a>]) -> Vec<PlanetMission<'a>> {
    let mut missions = Vec::with_capacity(entries.len().saturating_mul(PLANET_BANDS.len()));
    for entry in entries.iter().filter(|entry| entry.is_song()) {
        for band in PLANET_BANDS {
            missions.push(PlanetMission {
                band,
                song_id: entry.song_id,
                title: &entry.title,
            });
        }
    }
    missions
}

/// Distinct catalog songs behind the planet board (dead slots filtered
/// out exactly like [`planet_missions`]): every band offers each song,
/// so this is also the per-band mission count.
#[must_use]
pub fn planet_song_count(entries: &[OjnListEntry<'_>]) -> usize {
    entries.iter().filter(|entry| entry.is_song()).count()
}

/// Planet missions for one band, in song order; empty without catalog
/// data. One band draws one third of [`planet_missions`].
#[must_use]
pub fn planet_missions_for_band<'a>(
    entries: &'a [OjnListEntry<'a>],
    band: PlanetBand,
) -> Vec<PlanetMission<'a>> {
    entries
        .iter()
        .filter(|entry| entry.is_song())
        .map(|entry| mission_for_band(entry, band))
        .collect()
}

/// Offer one catalog song at one band.
fn mission_for_band<'a>(entry: &'a OjnListEntry<'a>, band: PlanetBand) -> PlanetMission<'a> {
    PlanetMission {
        band,
        song_id: entry.song_id,
        title: &entry.title,
    }
}

/// One band plus its mission count: every band offers the whole catalog,
/// so each count equals [`planet_song_count`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetBandCensus {
    /// Difficulty band.
    pub band: PlanetBand,
    /// Missions offered at this band.
    pub song_count: usize,
}

/// Per-band mission counts in button order ([`PLANET_BANDS`]).
#[must_use]
pub fn planet_band_census(entries: &[OjnListEntry<'_>]) -> [PlanetBandCensus; OJN_LEVEL_COUNT] {
    let song_count = planet_song_count(entries);
    PLANET_BANDS.map(|band| PlanetBandCensus { band, song_count })
}

/// Band button caption with its mission count (`EASY (12)`).
#[must_use]
pub fn planet_band_caption(census: &PlanetBandCensus) -> String {
    format!("{} ({})", census.band.name(), census.song_count)
}

/// Free-pick caption over the whole catalog (`FREE (12)`): `FREE` takes
/// any song at any band ([`PLANET_FREE_LABEL`]), so the count is the
/// full catalog, never one band.
#[must_use]
pub fn planet_free_caption(song_count: usize) -> String {
    format!("{PLANET_FREE_LABEL} ({song_count})")
}

/// One clickable board element of the planet scene. Only the five
/// difficulty/mode buttons resolve here (fully pinned `.bnd`
/// geometry); the MENU discs and promotion cells resolve through
/// the control-code dispatch table ([`planet_action_at`]) instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetHit {
    /// HARD button (`신의손`).
    Hard,
    /// NORMAL button (`고수손`).
    Normal,
    /// EASY button (`사람손`).
    Easy,
    /// FREE button (tutorial, backend never wired — P0).
    Free,
    /// DRUM button (quest, backend never wired — P0).
    Drum,
}

impl PlanetHit {
    /// Selectable difficulty band behind this hit (`FREE`/`DRUM` name
    /// no band — their backends were never wired, P0).
    #[must_use]
    pub const fn band(self) -> Option<PlanetBand> {
        match self {
            Self::Hard => Some(PlanetBand::Hard),
            Self::Normal => Some(PlanetBand::Normal),
            Self::Easy => Some(PlanetBand::Easy),
            Self::Free | Self::Drum => None,
        }
    }
}

/// HARD button rect, pinned by the `State_Planet.bnd` walk (rect 1:
/// (313,32)-(459,144); `docs/planet.md`). The buttons do NOT run
/// left-to-right in file order — HARD sits center.
pub const PLANET_HARD_RECT: Rect = Rect {
    x: 313,
    y: 32,
    w: 146,
    h: 112,
};
/// NORMAL button rect, pinned by the `.bnd` walk (rect 2).
pub const PLANET_NORMAL_RECT: Rect = Rect {
    x: 206,
    y: 32,
    w: 99,
    h: 112,
};
/// EASY button rect, pinned by the `.bnd` walk (rect 3).
pub const PLANET_EASY_RECT: Rect = Rect {
    x: 87,
    y: 32,
    w: 99,
    h: 112,
};
/// FREE button rect, pinned by the `.bnd` walk (rect 4).
pub const PLANET_FREE_RECT: Rect = Rect {
    x: 614,
    y: 32,
    w: 99,
    h: 112,
};
/// DRUM button rect, pinned by the `.bnd` walk (rect 5).
pub const PLANET_DRUM_RECT: Rect = Rect {
    x: 510,
    y: 32,
    w: 99,
    h: 112,
};

/// Pinned button rect of one selectable band.
#[must_use]
pub const fn planet_band_rect(band: PlanetBand) -> Rect {
    match band {
        PlanetBand::Hard => PLANET_HARD_RECT,
        PlanetBand::Normal => PLANET_NORMAL_RECT,
        PlanetBand::Easy => PLANET_EASY_RECT,
    }
}

/// Board element under native pixel (`x`, `y`): the pinned `.bnd`
/// button rects, right/bottom edges exclusive (`rect_hit`). `None`
/// everywhere else — MENU discs and promotion cells resolve through
/// [`planet_action_at`], never here.
#[must_use]
pub fn planet_hit(x: i32, y: i32) -> Option<PlanetHit> {
    for (rect, hit) in [
        (PLANET_HARD_RECT, PlanetHit::Hard),
        (PLANET_NORMAL_RECT, PlanetHit::Normal),
        (PLANET_EASY_RECT, PlanetHit::Easy),
        (PLANET_FREE_RECT, PlanetHit::Free),
        (PLANET_DRUM_RECT, PlanetHit::Drum),
    ] {
        if super::paint::rect_hit(&rect, x, y) {
            return Some(hit);
        }
    }
    None
}

/// Live difficulty selection on the planet board. `None` until the
/// player picks a band; `FREE`/`DRUM` clicks never change it (their
/// backends were never wired — P0 in `docs/parity.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanetSelection {
    band: Option<PlanetBand>,
}

impl PlanetSelection {
    /// No pick yet.
    #[must_use]
    pub const fn new() -> Self {
        Self { band: None }
    }

    /// Picked difficulty band, if any.
    #[must_use]
    pub const fn band(self) -> Option<PlanetBand> {
        self.band
    }

    /// Board click at native pixel (`x`, `y`). HARD/NORMAL/EASY record
    /// the pick; FREE/DRUM hit their pinned rects but stay unbound and
    /// change nothing (P0). Returns the hit (`None` outside every
    /// pinned rect).
    pub fn click(&mut self, x: i32, y: i32) -> Option<PlanetHit> {
        let hit = planet_hit(x, y)?;
        if let Some(band) = hit.band() {
            self.band = Some(band);
        }
        Some(hit)
    }

    /// Record a family pick without hit-testing: the dispatch-table
    /// path ([`planet_action_at`]) already resolved the control, so
    /// the game records the band the action names. `FREE`/`DRUM`
    /// never reach here (their action carries no band — P0).
    pub fn select_band(&mut self, band: PlanetBand) {
        self.band = Some(band);
    }
}

/// Planet board control codes (`ControlList_Interface.txt`
/// `STATE_PLANET`, lines 790–805): the five emblem buttons
/// (`0X19201002..06`), the promotion cells (`0X1920000D..0F` —
/// dispatch, carousel semantics OPEN), and the MENU-bar EXIT/OPTION discs
/// (`0X19200010`/`0X19200011`, art `Planet_Btn_Exit.ojs` /
/// `Planet_Btn_Option.ojs`).
pub const PLANET_BUTTON_HARD_CODE: u32 = 0x1920_1002;
/// NORMAL emblem button code (above).
pub const PLANET_BUTTON_NORMAL_CODE: u32 = 0x1920_1003;
/// EASY emblem button code (above).
pub const PLANET_BUTTON_EASY_CODE: u32 = 0x1920_1004;
/// FREE emblem button code (tutorial, backend never wired — P0).
pub const PLANET_BUTTON_FREE_CODE: u32 = 0x1920_1005;
/// DRUM emblem button code (quest, backend never wired — P0).
pub const PLANET_BUTTON_DRUM_CODE: u32 = 0x1920_1006;
/// Promotion carousel LEFT cell code: steps the banner back (the
/// press records; carousel semantics stay OPEN, `docs/planet.md`
/// OPEN 4). Pinned rect rect 12 (353,511)-(379,535) via ordinal
/// `(code & 0xFFF) − 1` — geometry rides staged placements only
/// (ADR-0003: no fallback literals), never this constant.
pub const PLANET_PROMO_LEFT_CODE: u32 = 0x1920_000D;
/// Promotion carousel STOP cell code: halts the banner rotation
/// (press records; semantics OPEN as above). Pinned rect rect 14
/// (387,511)-(413,535); geometry rides staged placements only.
pub const PLANET_PROMO_STOP_CODE: u32 = 0x1920_000E;
/// Promotion carousel RIGHT cell code: steps the banner forward
/// (press records; semantics OPEN as above). Pinned rect rect 13
/// (421,511)-(447,535); geometry rides staged placements only.
pub const PLANET_PROMO_RIGHT_CODE: u32 = 0x1920_000F;
/// Power disc code: opens the quit-confirm popup (capture-pinned,
/// `docs/planet.md` "Real board structure"; message + OK/CANCEL in
/// [`QUIT_MESSAGE_CP949`]).
pub const PLANET_EXIT_CODE: u32 = 0x1920_0010;
/// Gear disc code: opens the options dialog (live-pinned: the gear
/// opens the 옵션 dialog and Escape closes it, `docs/wine-boot.md`
/// input probes; the first-open tab stays OPEN below).
pub const PLANET_OPTION_CODE: u32 = 0x1920_0011;
/// HARD caption code (`IDC_TEXT_SPLANET_HARD`).
pub const PLANET_TEXT_HARD_CODE: u32 = 0x1940_0007;
/// NORMAL caption code (`IDC_TEXT_SPLANET_NORMAL`).
pub const PLANET_TEXT_NORMAL_CODE: u32 = 0x1940_0008;
/// EASY caption code (`IDC_TEXT_SPLANET_EASY`).
pub const PLANET_TEXT_EASY_CODE: u32 = 0x1940_0009;
/// FREE caption code (`IDC_TEXT_SPLANET_FREE`).
pub const PLANET_TEXT_FREE_CODE: u32 = 0x1940_000A;
/// DRUM caption code (`IDC_TEXT_SPLANET_DRUM`).
pub const PLANET_TEXT_DRUM_CODE: u32 = 0x1940_000B;

/// One pressed promotion-carousel cell: which banner control
/// answered the click (LEFT steps back, STOP halts rotation,
/// RIGHT steps forward — what the exe does with the press stays
/// OPEN, `docs/planet.md` OPEN 4; the shell records the press and
/// holds the board). Geometry never lives here: it rides the
/// staged `.bnd` placements (owner rule `f38f3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetPromo {
    /// LEFT cell (`0X1920000D`, rect 12).
    Left,
    /// PLAYSTOP cell (`0X1920000E`, rect 14).
    Stop,
    /// RIGHT cell (`0X1920000F`, rect 13).
    Right,
}

/// One planet-board action: the control-code → action dispatch
/// table's output ([`planet_action_for_code`]). Board input routes
/// here, never through per-button literals at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetAction {
    /// Emblem pressed: `Some(band)` for the lit families
    /// (HARD/NORMAL/EASY — family pick), `None` for the dimmed
    /// QUEST/TUTORIAL emblems (unbound, P0: no backend ever
    /// existed, the pick changes nothing).
    Emblem {
        /// Family band, or `None` for the unbound emblems.
        band: Option<PlanetBand>,
    },
    /// Promotion-carousel cell pressed: the press records (see
    /// [`PlanetPromo`]); carousel semantics stay OPEN.
    Promo {
        /// Which banner control answered the click.
        cell: PlanetPromo,
    },
    /// Power disc: open the quit-confirm popup.
    QuitConfirm,
    /// Gear disc: open the options dialog.
    OptionsDialog,
}

/// Action for one planet control code (`None` for every other
/// code: BACK, self — the promo arms name their cell while the
/// carousel semantics stay OPEN, `docs/planet.md` OPEN 4). The emblem arm is the
/// family-select half of the exe handler (E1,
/// `docs/overnight-evidence/e1-channel-dialog-trigger.md`:
/// family-index write + reconnect); the dialog-open half is the
/// shell's OPEN stand-in link (see `ScreenGame::click_planet`).
#[must_use]
pub const fn planet_action_for_code(code: u32) -> Option<PlanetAction> {
    match code {
        PLANET_BUTTON_HARD_CODE => Some(PlanetAction::Emblem {
            band: Some(PlanetBand::Hard),
        }),
        PLANET_BUTTON_NORMAL_CODE => Some(PlanetAction::Emblem {
            band: Some(PlanetBand::Normal),
        }),
        PLANET_BUTTON_EASY_CODE => Some(PlanetAction::Emblem {
            band: Some(PlanetBand::Easy),
        }),
        PLANET_BUTTON_FREE_CODE | PLANET_BUTTON_DRUM_CODE => {
            Some(PlanetAction::Emblem { band: None })
        }
        PLANET_EXIT_CODE => Some(PlanetAction::QuitConfirm),
        PLANET_OPTION_CODE => Some(PlanetAction::OptionsDialog),
        PLANET_PROMO_LEFT_CODE => Some(PlanetAction::Promo {
            cell: PlanetPromo::Left,
        }),
        PLANET_PROMO_STOP_CODE => Some(PlanetAction::Promo {
            cell: PlanetPromo::Stop,
        }),
        PLANET_PROMO_RIGHT_CODE => Some(PlanetAction::Promo {
            cell: PlanetPromo::Right,
        }),
        _ => None,
    }
}

/// EXIT disc rect, pinned by the `State_Planet.bnd` walk (rect 15:
/// (750,561)-(788,599); `docs/planet.md` ordinal pairing). The
/// S0.3 plan's "bnd rect 26" cannot exist: the bind file carries
/// 17 rects (0..16), and ordinal `(code & 0xFFF) − 1` pairs EXIT
/// (`0X19200010`) to rect 15 and OPTION (`0X19200011`) to rect 16
/// — both rects verified against the shipped bytes 2026-11-10.
pub const PLANET_EXIT_RECT: Rect = Rect {
    x: 750,
    y: 561,
    w: 38,
    h: 38,
};
/// OPTION disc rect, pinned by the `.bnd` walk (rect 16:
/// (710,561)-(748,599); `docs/planet.md`; see [`PLANET_EXIT_RECT`]
/// for the rect-26 correction).
pub const PLANET_OPTION_RECT: Rect = Rect {
    x: 710,
    y: 561,
    w: 38,
    h: 38,
};

/// Board action under native pixel (`x`, `y`): staged `.bnd`
/// rects first (the placement whose code dispatches and whose
/// rect contains the point), the pinned rect table when nothing
/// is staged (art-less parity, like [`planet_hit`]). `None`
/// everywhere else — promotion cells resolve through staged
/// placements only (no fallback literals, ADR-0003), so unstaged
/// promo clicks land nowhere.
#[must_use]
pub fn planet_action_at(placements: &[ScenePlacement], x: i32, y: i32) -> Option<PlanetAction> {
    placements
        .iter()
        .find_map(|placed| {
            planet_action_for_code(placed.code)
                .filter(|_| super::paint::rect_hit(&placed.rect, x, y))
        })
        .or_else(|| planet_action_fallback_at(x, y))
}

/// Fallback arm of [`planet_action_at`]: the pinned `.bnd` rects
/// (buttons from [`PLANET_HARD_RECT`]/kin, discs from
/// [`PLANET_EXIT_RECT`]/[`PLANET_OPTION_RECT`]) for stages without
/// placements. The values are the parsed bind rects verbatim (the
/// audit-DONE pattern); staged art always wins when present.
fn planet_action_fallback_at(x: i32, y: i32) -> Option<PlanetAction> {
    for (rect, code) in [
        (PLANET_HARD_RECT, PLANET_BUTTON_HARD_CODE),
        (PLANET_NORMAL_RECT, PLANET_BUTTON_NORMAL_CODE),
        (PLANET_EASY_RECT, PLANET_BUTTON_EASY_CODE),
        (PLANET_FREE_RECT, PLANET_BUTTON_FREE_CODE),
        (PLANET_DRUM_RECT, PLANET_BUTTON_DRUM_CODE),
        (PLANET_EXIT_RECT, PLANET_EXIT_CODE),
        (PLANET_OPTION_RECT, PLANET_OPTION_CODE),
    ] {
        if super::paint::rect_hit(&rect, x, y) {
            return planet_action_for_code(code);
        }
    }
    None
}

/// One emblem caption: the `TEXT` control code plus its runtime
/// string (CP949 bytes, decoded through `o2jam-text`). The five
/// strings ship composed in the exe (`docs/planet.md` "Emblem
/// caption strings EXE-PINNED"); positions ride the staged
/// `.bnd` rects (ordinals 6..10), never literals here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetCaption {
    /// `TEXT` control code (`0X19400007..0B`).
    pub code: u32,
    /// Runtime string, CP949 bytes.
    pub cp949: &'static [u8],
}

/// Emblem captions in code order (HARD, NORMAL, EASY, FREE, DRUM):
/// `신의손 (고급)` at file `0x247b00` (VA `0x647b00`),
/// `고수손 (중급)` at `0x247af0`, `사람손 (초급)` at `0x247ae0`,
/// `튜토리얼` at `0x247ad4`, `퀘스트` at `0x247acc` (all
/// `docs/planet.md`). Bytes are the CP949 encodings of those
/// strings (decoded and asserted in the side tests).
pub const PLANET_CAPTIONS: [PlanetCaption; 5] = [
    PlanetCaption {
        code: PLANET_TEXT_HARD_CODE,
        cp949: &[
            0xBD, 0xC5, 0xC0, 0xC7, 0xBC, 0xD5, 0x20, 0x28, 0xB0, 0xED, 0xB1, 0xDE, 0x29,
        ],
    },
    PlanetCaption {
        code: PLANET_TEXT_NORMAL_CODE,
        cp949: &[
            0xB0, 0xED, 0xBC, 0xF6, 0xBC, 0xD5, 0x20, 0x28, 0xC1, 0xDF, 0xB1, 0xDE, 0x29,
        ],
    },
    PlanetCaption {
        code: PLANET_TEXT_EASY_CODE,
        cp949: &[
            0xBB, 0xE7, 0xB6, 0xF7, 0xBC, 0xD5, 0x20, 0x28, 0xC3, 0xCA, 0xB1, 0xDE, 0x29,
        ],
    },
    PlanetCaption {
        code: PLANET_TEXT_FREE_CODE,
        cp949: &[0xC6, 0xA9, 0xC5, 0xE4, 0xB8, 0xAE, 0xBE, 0xF3],
    },
    PlanetCaption {
        code: PLANET_TEXT_DRUM_CODE,
        cp949: &[0xC4, 0xF9, 0xBD, 0xBA, 0xC6, 0xAE],
    },
];

/// Caption for one `TEXT` control code (`None` for every other
/// code). The painter looks the code up in the staged placements
/// to find its `.bnd` rect — geometry only ever comes from the
/// parsed bind file (owner rule `f38f3`).
#[must_use]
pub const fn planet_caption_for_code(code: u32) -> Option<PlanetCaption> {
    let mut index = 0;
    while index < PLANET_CAPTIONS.len() {
        let caption = PLANET_CAPTIONS[index];
        if caption.code == code {
            return Some(caption);
        }
        index += 1;
    }
    None
}

/// Options-dialog tab: one per `Dialog_Option_*` block
/// (`ControlList_Interface.txt`; the blocks name each other's
/// tab buttons for switching). The gear opens the dialog
/// (live-pinned, `docs/wine-boot.md`); which tab it opens on
/// first stays OPEN (see [`PLANET_OPTION_FIRST_TAB`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetOptionTab {
    /// Key bindings (`DIALOG_OPTION_KEY`, line 1242).
    Key,
    /// Sound sliders (`DIALOG_OPTION_SOUND`, line 1281).
    Sound,
    /// Game effects (`DIALOG_OPTION_GAME`, line 1611).
    Game,
}

impl PlanetOptionTab {
    /// `ControlList` block for this tab.
    #[must_use]
    pub const fn block_name(self) -> &'static str {
        match self {
            Self::Key => "DIALOG_OPTION_KEY",
            Self::Sound => "DIALOG_OPTION_SOUND",
            Self::Game => "DIALOG_OPTION_GAME",
        }
    }

    /// `.bnd` layout file for this tab.
    #[must_use]
    pub const fn bind_file(self) -> &'static str {
        match self {
            Self::Key => "Dialog_Option_Key.bnd",
            Self::Sound => "Dialog_Option_Sound.bnd",
            Self::Game => "Dialog_Option_Game.bnd",
        }
    }

    /// Dialog EXIT code for this tab (`Option_Exit.ojs` buttons:
    /// `0X25200017` / `0X26200012` / `0X2F20000F`, lines 1274 /
    /// 1302 / 1629). Pressing it closes the dialog without saving
    /// (Escape closes it too — live-pinned, `docs/wine-boot.md`);
    /// the tab SAVE button commits to `option.ini`.
    #[must_use]
    pub const fn exit_code(self) -> u32 {
        match self {
            Self::Key => 0x2520_0017,
            Self::Sound => 0x2620_0012,
            Self::Game => 0x2F20_000F,
        }
    }

    /// Dialog SAVE code for this tab (`Option_Save.ojs` buttons:
    /// `0X25200016` / `0X26200011` / `0X2F20000E`). Pressing it
    /// persists the settings beside the install root (the write
    /// path mirrors the exe `WritePrivateProfileStringA` writers —
    /// same section/key/file names, decimal ints).
    #[must_use]
    pub const fn save_code(self) -> u32 {
        match self {
            Self::Key => 0x2520_0016,
            Self::Sound => 0x2620_0011,
            Self::Game => 0x2F20_000E,
        }
    }

    /// Dialog DEFAULT code for this tab (`Option_Default.ojs`
    /// buttons: `0X25200015` / `0X26200010` / `0X2F20000D`).
    /// Pressing it restores the exe defaults for the tab's keys.
    #[must_use]
    pub const fn default_code(self) -> u32 {
        match self {
            Self::Key => 0x2520_0015,
            Self::Sound => 0x2620_0010,
            Self::Game => 0x2F20_000D,
        }
    }

    /// Tab-switch code this tab answers (`SOUNDSET`/`GAMESET` /
    /// `KEYSET` buttons, spriteless entries riding their `.bnd`
    /// rects): the switched-to tab for `code`, if this tab owns it.
    #[must_use]
    pub const fn switch_tab(self, code: u32) -> Option<PlanetOptionTab> {
        match (self, code) {
            (Self::Key, OPTION_KEY_SOUNDSET) => Some(PlanetOptionTab::Sound),
            (Self::Key, OPTION_KEY_GAMESET) => Some(PlanetOptionTab::Game),
            (Self::Sound, OPTION_SOUND_KEYSET) => Some(PlanetOptionTab::Key),
            (Self::Sound, OPTION_SOUND_GAMESET) => Some(PlanetOptionTab::Game),
            (Self::Game, OPTION_GAME_KEYSET) => Some(PlanetOptionTab::Key),
            (Self::Game, OPTION_GAME_SOUNDSET) => Some(PlanetOptionTab::Sound),
            _ => None,
        }
    }
}

/// Key-tab `SOUNDSET` switch code (`IDC_BUTTON_DOPTION_SOUNDSET`).
pub const OPTION_KEY_SOUNDSET: u32 = 0x2560_0018;
/// Key-tab `GAMESET` switch code (`IDC_BUTTON_DOPTION_GAMESET`).
pub const OPTION_KEY_GAMESET: u32 = 0x2560_0019;
/// Sound-tab `KEYSET` switch code (`IDC_IMAGE_DOPTIONSOUND_KEYSET`).
pub const OPTION_SOUND_KEYSET: u32 = 0x2660_000E;
/// Sound-tab `GAMESET` switch code (`IDC_IMAGE_DOPTIONSOUND_GAMESET`).
pub const OPTION_SOUND_GAMESET: u32 = 0x2660_000F;
/// Game-tab `KEYSET` switch code (`IDC_BUTTON_DOPTIONGAME_KEYSET`).
pub const OPTION_GAME_KEYSET: u32 = 0x2F60_0010;
/// Game-tab `SOUNDSET` switch code (`IDC_BUTTON_DOPTIONGAME_SOUNDSET`).
pub const OPTION_GAME_SOUNDSET: u32 = 0x2F60_0011;

/// Key-tab lane buttons `KEY1..KEY7` (`IDC_BUTTON_DOPTION_KEY1..7`,
/// codes `0X2520900C..12`): click arms the lane for rebind, the
/// next mapped key press becomes the lane's key.
pub const OPTION_KEY_LANE_FIRST: u32 = 0x2520_900C;
/// Key-tab 3-key lane buttons (`IDC_BUTTON_DOPTION_3KEY_1..4`,
/// codes `0X25209008..0B`): arm lanes 0..3 the same way
/// (the 3-key lane mapping itself stays OPEN).
pub const OPTION_KEY_3KEY_FIRST: u32 = 0x2520_9008;
/// Key-tab 3-key button (`IDC_BUTTON_DOPTION_3KEY`).
pub const OPTION_KEY_3KEY_MODE: u32 = 0x2520_1003;
/// Key-tab 7-key button (`IDC_BUTTON_DOPTION_7KEY`).
pub const OPTION_KEY_7KEY_MODE: u32 = 0x2520_1004;
/// Key-tab key-test checkbox (`IDC_BUTTON_DOPTION_KEYTEST`).
pub const OPTION_KEY_TEST: u32 = 0x2520_A005;

/// Sound-tab slider codes (`IDC_BUTTON_DOPTIONSOUND_*`): each pair
/// steps its volume down/up by the ours-by-design step.
pub const OPTION_SOUND_BG_MUSIC_DOWN: u32 = 0x2620_0002;
/// Sound-tab slider codes (see [`OPTION_SOUND_BG_MUSIC_DOWN`]).
pub const OPTION_SOUND_BG_MUSIC_UP: u32 = 0x2620_0003;
/// Sound-tab slider codes (see [`OPTION_SOUND_BG_MUSIC_DOWN`]).
pub const OPTION_SOUND_BG_EFFECT_DOWN: u32 = 0x2620_0004;
/// Sound-tab slider codes (see [`OPTION_SOUND_BG_MUSIC_DOWN`]).
pub const OPTION_SOUND_BG_EFFECT_UP: u32 = 0x2620_0005;
/// Sound-tab slider codes (see [`OPTION_SOUND_BG_MUSIC_DOWN`]).
pub const OPTION_SOUND_GAME_MUSIC_DOWN: u32 = 0x2620_0006;
/// Sound-tab slider codes (see [`OPTION_SOUND_BG_MUSIC_DOWN`]).
pub const OPTION_SOUND_GAME_MUSIC_UP: u32 = 0x2620_0007;
/// Sound-tab slider codes (see [`OPTION_SOUND_BG_MUSIC_DOWN`]).
pub const OPTION_SOUND_GAME_EFFECT_DOWN: u32 = 0x2620_0008;
/// Sound-tab slider codes (see [`OPTION_SOUND_BG_MUSIC_DOWN`]).
pub const OPTION_SOUND_GAME_EFFECT_UP: u32 = 0x2620_0009;

/// Game-tab effect arms (`IDC_BUTTON_DOPTIONGAME_EFFECTHIGH` /
/// `EFFECTNORMAL` / `EFFECTLOW`, codes `0X2F209002..04`):
/// in-memory selection, no ini key found (OPEN).
pub const OPTION_GAME_EFFECT_HIGH: u32 = 0x2F20_9002;
/// Game-tab effect arms (see [`OPTION_GAME_EFFECT_HIGH`]).
pub const OPTION_GAME_EFFECT_NORMAL: u32 = 0x2F20_9003;
/// Game-tab effect arms (see [`OPTION_GAME_EFFECT_LOW`]).
pub const OPTION_GAME_EFFECT_LOW: u32 = 0x2F20_9004;
/// Game-tab EQ Yes/No (`IDC_BUTTON_DOPTIONGAME_BLENDHALF_YES` /
/// `_NO`): Yes writes 1, No writes 0 (order ours-by-design).
pub const OPTION_GAME_BLEND_YES: u32 = 0x2F20_9005;
/// Game-tab EQ Yes/No (see [`OPTION_GAME_BLEND_YES`]).
pub const OPTION_GAME_BLEND_NO: u32 = 0x2F20_9006;
/// Game-tab note-shape buttons (`IDC_BUTTON_DOPTIONGAME_NOTETYPE1` /
/// `NOTETYPE2`): the dialog itself offers two arms only — value 2
/// stays a dead alias. Button→value order is ours-by-design.
pub const OPTION_GAME_NOTETYPE1: u32 = 0x2F20_9007;
/// Game-tab note-shape buttons (see [`OPTION_GAME_NOTETYPE1`]).
pub const OPTION_GAME_NOTETYPE2: u32 = 0x2F20_9008;
/// Game-tab guide buttons (`IDC_BUTTON_DOPTIONGAME_NOTEGUIDELONG` /
/// `MIDDLE` / `SHORT` / `NOUSE`): button→tail-value order is
/// ours-by-design (Long→3, Middle→2, Short→1, NoUse→0).
pub const OPTION_GAME_GUIDE_LONG: u32 = 0x2F20_9009;
/// Game-tab guide buttons (see [`OPTION_GAME_GUIDE_LONG`]).
pub const OPTION_GAME_GUIDE_MIDDLE: u32 = 0x2F20_900A;
/// Game-tab guide buttons (see [`OPTION_GAME_GUIDE_LONG`]).
pub const OPTION_GAME_GUIDE_SHORT: u32 = 0x2F20_900B;
/// Game-tab guide buttons (see [`OPTION_GAME_GUIDE_LONG`]).
pub const OPTION_GAME_GUIDE_NOUSE: u32 = 0x2F20_900C;
/// Game-tab window buttons (`IDC_BUTTON_DOPTIONGAME_MODEFULL` /
/// `MODEWIN`): Full→1, Win→2 (order ours-by-design; the `1`/`2`
/// arms drive the window apply at file `0x3a06c`).
pub const OPTION_GAME_MODE_FULL: u32 = 0x2F20_9014;
/// Game-tab window buttons (see [`OPTION_GAME_MODE_FULL`]).
pub const OPTION_GAME_MODE_WIN: u32 = 0x2F20_9015;
/// Game-tab long-note buttons
/// (`IDC_BUTTON_DOPTIONGAME_LONGNOTEMODE_USED` / `_NONE`):
/// Used writes 1, None writes 0 (the consumer tests `== 1`).
pub const OPTION_GAME_LONGNOTE_USED: u32 = 0x2F20_9016;
/// Game-tab long-note buttons (see [`OPTION_GAME_LONGNOTE_USED`]).
pub const OPTION_GAME_LONGNOTE_NONE: u32 = 0x2F20_9017;

/// First-open tab stand-in: the Key block (first in
/// `ControlList_Interface.txt` file order). OPEN: no capture or
/// static pin shows which tab the gear opens first. Named next
/// step — rig capture of a gear click (`~/scratch/planet-verify/`
/// recipe: boot explorer desktop, dismiss the notice, SendInput
/// click the gear disc at native ~(729,580), capture the dialog).
/// Tab switching rides the staged `KEYSET`/`SOUNDSET`/`GAMESET`
/// codes through their `.bnd` rects (no literals).
pub const PLANET_OPTION_FIRST_TAB: PlanetOptionTab = PlanetOptionTab::Key;

/// Channel-list dialog block (`CDChannelInfo`; `ControlList_Interface.txt`
/// `DIALOG_CHANNEL_INFO`). Host State is Planet: the sheet opens over the
/// planet board. The hover bar (`Dialog_ChannelSelectBar.ojs`) is not a
/// block entry — the exe mounts it at the Dialog object (`+0x8C`) — so it
/// stays hand-staged and OPEN while this row stages the bound sheet.
/// Spelling keeps the in-archive `DIALOG_` prefix verbatim.
pub const CHANNEL_INFO_DIALOG_BLOCK: &str = "DIALOG_CHANNEL_INFO";
/// Quit-confirm dialog block: the `MESSAGE` popup the power disc
/// opens (OK/CANCEL, `docs/planet.md` + capture `exit-state.png`).
/// The block match (OK `0X22200002` / CANCEL `0X22200003` / message
/// `0X22400004` over `Dialog_Question1.ojs`) is the best-effort
/// art source; its mount mechanics stay exe-OPEN like the channel
/// dialog's (reference-measured, `docs/channel-fill.md` method).
pub const QUIT_DIALOG_BLOCK: &str = "DIALOG_QUESTION1";
/// Quit-confirm layout (4 rects).
pub const QUIT_DIALOG_BIND_FILE: &str = "Dialog_Question1.bnd";
/// Quit popup OK code (quits the client).
pub const QUIT_OK_CODE: u32 = 0x2220_0002;
/// Quit popup CANCEL code (back to the bare board).
pub const QUIT_CANCEL_CODE: u32 = 0x2220_0003;
/// Quit popup message TEXT code.
pub const QUIT_MESSAGE_CODE: u32 = 0x2240_0004;
/// Quit message string (CP949 bytes): `게임을 종료하시겠습니까?`
/// (capture `exit-state.png`, message plate; the exe string
/// offset is not pinned — the capture is the pin).
pub const QUIT_MESSAGE_CP949: &[u8] = &[
    0xB0, 0xD4, 0xC0, 0xD3, 0xC0, 0xBB, 0x20, 0xC1, 0xBE, 0xB7, 0xE1, 0xC7, 0xCF, 0xBD, 0xC3, 0xB0,
    0xDA, 0xBD, 0xC0, 0xB4, 0xCF, 0xB1, 0xEE, 0x3F,
];

/// Create-room dialog block (`CDCreateRoom`; `ControlList_Interface.txt`
/// `DIALOG_CREATEROOM`, decl `0x09`/`docs/file-formats/opi.md`).
pub const CREATE_ROOM_DIALOG_BLOCK: &str = "DIALOG_CREATEROOM";
/// Create-room layout (`Dialog_CreateRoom.bnd`, 9 rects).
pub const CREATE_ROOM_BIND_FILE: &str = "Dialog_CreateRoom.bnd";
/// Password-entry dialog block (`CDInputPassword`; `DIALOG_INPUTPASSWORD`).
pub const PASSWORD_DIALOG_BLOCK: &str = "DIALOG_INPUTPASSWORD";
/// Password-entry layout (`Dialog_Password.bnd`, 5 rects).
pub const PASSWORD_BIND_FILE: &str = "Dialog_Password.bnd";
/// Create-room CREATE control code (`0X24200002`).
pub const CREATE_ROOM_CREATE_CODE: u32 = 0x2420_0002;
/// Create-room CANCEL control code (`0X24200003`).
pub const CREATE_ROOM_CANCEL_CODE: u32 = 0x2420_0003;
/// Normal-mode toggle code (`0X2420B004`).
pub const CREATE_ROOM_NORMAL_MODE_CODE: u32 = 0x2420_B004;
/// Live-battle-mode toggle code (`0X2420B005`).
pub const CREATE_ROOM_LIVE_MODE_CODE: u32 = 0x2420_B005;
/// 3-key toggle code (`0X2420B006`).
pub const CREATE_ROOM_3KEY_MODE_CODE: u32 = 0x2420_B006;
/// 7-key toggle code (`0X2420B007`).
pub const CREATE_ROOM_7KEY_MODE_CODE: u32 = 0x2420_B007;
/// Title edit code (`0X24300008`).
pub const CREATE_ROOM_TITLE_CODE: u32 = 0x2430_0008;
/// Password edit code (`0X24300009`).
pub const CREATE_ROOM_PASSWORD_CODE: u32 = 0x2430_0009;
/// Room-create button code (`IDC_BUTTON_SROOM_CREATE`, `0X13200007`).
pub const ROOM_CREATE_BUTTON_CODE: u32 = 0x1320_0007;
/// Password-dialog OK code (`0X28200004`, `Dialog_PasswordOK.ojs`).
pub const PASSWORD_OK_CODE: u32 = 0x2820_0004;
/// Password-dialog CANCEL code (`0X28200005`, `Dialog_PasswordNO.ojs`).
pub const PASSWORD_CANCEL_CODE: u32 = 0x2820_0005;
/// Password-dialog edit code (`0X28300003`).
pub const PASSWORD_EDIT_CODE: u32 = 0x2830_0003;
/// Waiting team-button run start (`IDC_BUTTON_SWAIT_TEAMA`, `0X16201004`).
pub const WAITING_TEAM_FIRST_CODE: u32 = 0x1620_1004;
/// Waiting team-button run end (`TEAMH`, `0X1620100B`).
pub const WAITING_TEAM_LAST_CODE: u32 = 0x1620_100B;
/// Sprite bodies the create-room dialog names, in block order.
#[must_use]
pub fn create_room_sprite_names(list: &ControlList) -> Vec<String> {
    block_sprite_names(list, CREATE_ROOM_DIALOG_BLOCK)
}
/// Sprite bodies the password dialog names, in block order.
#[must_use]
pub fn password_sprite_names(list: &ControlList) -> Vec<String> {
    block_sprite_names(list, PASSWORD_DIALOG_BLOCK)
}
/// Team index for one waiting team-button code (`TEAMA..H` to 0..7).
/// Mirrors the dispatch range check `0x16201003 < code < 0x1620100c`
/// (`FUN_004b22d0`, file `0xb22d0`) and the toggle mapper
/// `FUN_0048c810` (file `0x8c810`): the press maps to
/// `code - 0x16201004` and lights exactly that button. `None` outside
/// the run.
#[must_use]
pub const fn waiting_team_for_code(code: u32) -> Option<u8> {
    match code {
        WAITING_TEAM_FIRST_CODE..=WAITING_TEAM_LAST_CODE => {
            match code.wrapping_sub(WAITING_TEAM_FIRST_CODE) {
                0 => Some(0),
                1 => Some(1),
                2 => Some(2),
                3 => Some(3),
                4 => Some(4),
                5 => Some(5),
                6 => Some(6),
                _ => Some(7),
            }
        }
        _ => None,
    }
}
/// One create-room dialog action: the code-to-action dispatch output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateRoomAction {
    /// CREATE pressed: commit the dialog (`None` while the title is empty).
    Commit,
    /// CANCEL pressed: close without a commit.
    Cancel,
    /// Normal-mode toggle.
    NormalMode,
    /// Live-battle-mode toggle.
    LiveMode,
    /// 3-key toggle.
    ThreeKey,
    /// 7-key toggle.
    SevenKey,
    /// Title edit focus.
    FocusTitle,
    /// Password edit focus.
    FocusPassword,
}
/// Action for one create-room control code (`None` otherwise).
#[must_use]
pub const fn create_room_action_for_code(code: u32) -> Option<CreateRoomAction> {
    match code {
        CREATE_ROOM_CREATE_CODE => Some(CreateRoomAction::Commit),
        CREATE_ROOM_CANCEL_CODE => Some(CreateRoomAction::Cancel),
        CREATE_ROOM_NORMAL_MODE_CODE => Some(CreateRoomAction::NormalMode),
        CREATE_ROOM_LIVE_MODE_CODE => Some(CreateRoomAction::LiveMode),
        CREATE_ROOM_3KEY_MODE_CODE => Some(CreateRoomAction::ThreeKey),
        CREATE_ROOM_7KEY_MODE_CODE => Some(CreateRoomAction::SevenKey),
        CREATE_ROOM_TITLE_CODE => Some(CreateRoomAction::FocusTitle),
        CREATE_ROOM_PASSWORD_CODE => Some(CreateRoomAction::FocusPassword),
        _ => None,
    }
}
/// Create-room action under native pixel (`x`, `y`): staged `.bnd`
/// rects only — the placement whose code dispatches and whose rect
/// contains the point (owner rule `f38f3`: geometry from parsed
/// rects, never literals). `None` while no staged rect covers it.
#[must_use]
pub fn create_room_action_at(
    placements: &[ScenePlacement],
    x: i32,
    y: i32,
) -> Option<CreateRoomAction> {
    placements.iter().find_map(|placed| {
        create_room_action_for_code(placed.code)
            .filter(|_| super::paint::rect_hit(&placed.rect, x, y))
    })
}
/// One password-dialog action: the code-to-action dispatch output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordAction {
    /// OK pressed: commit the password.
    Commit,
    /// CANCEL pressed: close without a commit.
    Cancel,
}
/// Action for one password-dialog control code (`None` otherwise).
#[must_use]
pub const fn password_action_for_code(code: u32) -> Option<PasswordAction> {
    match code {
        PASSWORD_OK_CODE => Some(PasswordAction::Commit),
        PASSWORD_CANCEL_CODE => Some(PasswordAction::Cancel),
        _ => None,
    }
}
/// Password-dialog action under native pixel (`x`, `y`): staged `.bnd`
/// rects only (see [`create_room_action_at`]).
#[must_use]
pub fn password_action_at(placements: &[ScenePlacement], x: i32, y: i32) -> Option<PasswordAction> {
    placements.iter().find_map(|placed| {
        password_action_for_code(placed.code).filter(|_| super::paint::rect_hit(&placed.rect, x, y))
    })
}
/// Create-room form behind the dialog: title/password edits plus the
/// two toggle pairs (Normal/Live battle, 3/7 keys). Caps follow the
/// `CreateRoom` wire (`ROOM_TITLE_CAP` 22 / `ROOM_PASSWORD_CAP` 12).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateRoomForm {
    /// Title edit (cap 22).
    pub title: String,
    /// Password edit (cap 12).
    pub password: String,
    /// Battle mode (Live) versus normal.
    pub live_battle: bool,
    /// 7-key arena versus 3-key.
    pub seven_key: bool,
}
impl CreateRoomForm {
    /// Empty form: normal mode, 7-key (the dialog defaults).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Type one char into the title edit (cap 22, control chars refused).
    pub fn push_title_char(&mut self, typed: char) {
        if typed.is_control() || self.title.chars().count() >= 22 {
            return;
        }
        self.title.push(typed);
    }
    /// Erase one title char.
    pub fn pop_title(&mut self) {
        self.title.pop();
    }
    /// Type one char into the password edit (cap 12, control chars refused).
    pub fn push_password_char(&mut self, typed: char) {
        if typed.is_control() || self.password.chars().count() >= 12 {
            return;
        }
        self.password.push(typed);
    }
    /// Erase one password char.
    pub fn pop_password(&mut self) {
        self.password.pop();
    }
    /// Commit body for `CreateRoom` 2004 (`None` while the title is empty:
    /// every observed CREATE ROOM frame holds the title empty with OK
    /// never firing, `docs/wine-boot.md`). An empty password means no lock.
    #[must_use]
    pub fn commit(&self) -> Option<CreateRoom> {
        if self.title.is_empty() {
            return None;
        }
        Some(CreateRoom {
            title: self.title.clone(),
            key_mode: if self.seven_key {
                KeyMode::SevenKeys
            } else {
                KeyMode::ThreeKeys
            },
            game_mode: if self.live_battle {
                GameMode::Live
            } else {
                GameMode::Normal
            },
            has_password: !self.password.is_empty(),
            password: self.password.clone(),
            min_level: 0,
            max_level: 99,
            premium: false,
        })
    }
}
/// Password-entry form behind the join dialog (cap 12, control chars refused).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PasswordForm {
    /// Password edit (cap 12).
    pub password: String,
}
impl PasswordForm {
    /// Empty form.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Type one char (cap 12, control chars refused).
    pub fn push_char(&mut self, typed: char) {
        if typed.is_control() || self.password.chars().count() >= 12 {
            return;
        }
        self.password.push(typed);
    }
    /// Erase one char.
    pub fn pop(&mut self) {
        self.password.pop();
    }
}
/// Room-dialog modal over the Room lobby (create-room or password entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomDialog {
    /// Create-room dialog (`DIALOG_CREATEROOM`).
    CreateRoom,
    /// Password dialog for one locked room number.
    Password {
        /// Room number the password gates.
        number: i32,
    },
}

/// One lanking board row: catalog position, not a score. No score
/// table ships with the client, and the rank-notification path (the
/// 2029/4029 family) has no `o2jam-net` decoder — no new opcode is
/// staged here — so the board lists songs in catalog order and numbers
/// only the position; live 4015 bytes ride [`live_rank_rows`] instead.
/// Never a score: positions and wire bytes only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LankingRow<'a> {
    /// 1-based catalog position.
    pub position: u32,
    /// Song id.
    pub song_id: u32,
    /// Song title.
    pub title: &'a str,
}

/// Lanking rows in catalog order, dead slots filtered out; empty
/// without catalog data.
#[must_use]
pub fn lanking_rows<'a>(entries: &'a [OjnListEntry<'a>]) -> Vec<LankingRow<'a>> {
    entries
        .iter()
        .filter(|entry| entry.is_song())
        .enumerate()
        .filter_map(|(index, entry)| lanking_row(index, entry))
        .collect()
}

/// Number one catalog entry by its order (`None` past `u32` range).
fn lanking_row<'a>(index: usize, entry: &'a OjnListEntry<'a>) -> Option<LankingRow<'a>> {
    let position = u32::try_from(index.checked_add(1)?).ok()?;
    Some(LankingRow {
        position,
        song_id: entry.song_id,
        title: &entry.title,
    })
}

/// One live rank row (`GameStatsUpdate` 4015 broadcast): member slot
/// plus its live rank byte. Catalog scores never ship with the
/// client, so only these wire bytes feed the board — never invented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveRankRow {
    /// Member slot (0-based, broadcast order).
    pub slot: u8,
    /// Live rank byte for that slot.
    pub rank: u8,
}

/// Live rank rows in slot order, zero bytes skipped (empty seats);
/// `None` yields no rows: the board paints its art without rows.
#[must_use]
pub fn live_rank_rows(update: Option<&GameStatsUpdate>) -> Vec<LiveRankRow> {
    let Some(update) = update else {
        return Vec::new();
    };
    update
        .ranks
        .iter()
        .enumerate()
        .filter_map(|(index, rank)| live_rank_row(index, *rank))
        .collect()
}

/// Number one broadcast byte (`None` for empty seats and
/// out-of-range slots).
fn live_rank_row(index: usize, rank: u8) -> Option<LiveRankRow> {
    if rank == 0 {
        return None;
    }
    let slot = u8::try_from(index).ok()?;
    Some(LiveRankRow { slot, rank })
}

/// Board caption with the row count (`RANKING (12)`); empty catalogs
/// still name the board with zero rows.
#[must_use]
pub fn lanking_caption(row_count: usize) -> String {
    format!("RANKING ({row_count})")
}

/// One song's catalog position (`None` for dead slots and unknown ids).
/// Dead-slot filtering matches [`lanking_rows`]: only song entries resolve.
#[must_use]
pub fn lanking_row_for_song<'a>(
    entries: &'a [OjnListEntry<'a>],
    song_id: u32,
) -> Option<LankingRow<'a>> {
    entries
        .iter()
        .filter(|entry| entry.is_song())
        .enumerate()
        .find(|(_, entry)| entry.song_id == song_id)
        .and_then(|(index, entry)| lanking_row(index, entry))
}

// --- Lanking board parity (ticket #20) ---
//
// The board is two lists (`docs/lanking.md` block walk, all codes
// exe-pinned there, rects confirmed on the staged 73-placement
// dump): the left `LVIEW` shows five catalog songs per page under
// the `LVIEW_EX_1`..`_5` hit areas (`0x23`..`0x27`, 353x86 each
// tiling the song box) with sort buttons TITLE/DIFF/BPM/ARTIST
// (`0x0F`..`0x12`) and its `LPAGE` pager (`0x15`/`0x16`); the
// right `RVIEW` shows the fetched `2028 MusicPlayRankingAnswer`
// lines — the viewer's own line on `RVIEW_EX_1`, ten board lines
// on `RVIEW_EX_2`..`_11` (`0x35`..`0x3F`, corroborated by the
// answer's self-plus-ten shape and the zero-count ten-fill rule,
// `docs/overnight-evidence/u4-ranking-packs.md`) — with
// `RANK`/`NICKNAME`/`BATTLE_CONT`/`WIN_COUNT`/`WIN_PERSENT`/`UPDOWM`
// texts under its `RPAGE` pager (`0x17`/`0x18`). The row LANK
// button (`0x31`) and the `LVIEW_EX` hit areas pick the song row;
// the picked title inks the `MUSICNAME` text (`0x34`). The
// `RVIEW_EX` toggles stay display-only OPEN (no consumer pins
// them — next step: live row-click capture): row picks ride the
// left hit areas. Everything else on the screen (BATTLE/TOP100,
// tier buttons, search, REFRESH, GEM/CASH texts) stays
// display-only OPEN with the rig capture as its next step.
//
// Sort directions, the LPAGE/RPAGE left/right pairing, and the
// self-line-on-`RVIEW_EX_1` read are ASSUMED (no live click
// capture yet): the codes, hit areas, and counts are exe proof,
// the rest names its next step (SendInput sweep on the live board
// per the `~/scratch/planet-verify/` recipe).

/// Sort-button codes (`IDC_BUTTON_LANKING_TITLE/DIFF/BPM/ARTIST`).
pub const LANKING_SORT_TITLE_CODE: u32 = 0x5220_000F;
pub const LANKING_SORT_DIFF_CODE: u32 = 0x5220_0010;
pub const LANKING_SORT_BPM_CODE: u32 = 0x5220_0011;
pub const LANKING_SORT_ARTIST_CODE: u32 = 0x5220_0012;
/// Left pager codes (`IDC_BUTTON_LANKING_LPAGE_LEFT/RIGHT`):
/// left steps back, right steps forward (ASSUMED pairing).
pub const LANKING_LPAGE_PREV_CODE: u32 = 0x5220_0015;
pub const LANKING_LPAGE_NEXT_CODE: u32 = 0x5220_0016;
/// Right pager codes (`IDC_BUTTON_LANKING_RPAGE_LEFT/RIGHT`):
/// same left/right read as the left pager (ASSUMED pairing).
pub const LANKING_RPAGE_PREV_CODE: u32 = 0x5220_0017;
pub const LANKING_RPAGE_NEXT_CODE: u32 = 0x5220_0018;
/// Left-list TEXT template codes (`IDC_TEXT_LVIEW_TITLE` … `DIFF`).
pub const LANKING_LVIEW_TEXT_CODES: [u32; 6] = [
    0x5240_002B,
    0x5240_002C,
    0x5240_002D,
    0x5240_002E,
    0x5240_002F,
    0x5240_0030,
];
/// Right-list TEXT template codes (`IDC_TEXT_RVIEW_LANK` … `UPDOWM`).
pub const LANKING_RVIEW_TEXT_CODES: [u32; 6] = [
    0x5240_0040,
    0x5240_0041,
    0x5240_0042,
    0x5240_0043,
    0x5240_0044,
    0x5240_0045,
];
/// Picked-song caption (`IDC_TEXT_LANKING_MUSICNAME`).
pub const LANKING_MUSICNAME_CODE: u32 = 0x5240_0034;
/// Per-row LANK button (`IDC_IMAGE_LVIEW_LANK`): one staged
/// template rect tiled over the staged left slots.
pub const LANKING_ROW_LANK_CODE: u32 = 0x5220_0031;
/// First left-row hit area (`IDC_BUTTON_LVIEW_EX_1`); the five run
/// contiguously through `+4` (block walk + staged 353x86 tiling).
pub const LANKING_LVIEW_TOGGLE_FIRST_CODE: u32 = 0x5204_1023;
/// Left-row hit areas on the board (five staged `LVIEW_EX` rects).
pub const LANKING_LVIEW_TOGGLE_COUNT: usize = 5;
/// First right-row hit area (`IDC_BUTTON_RVIEW_EX_1`); the eleven
/// run contiguously through `+10` (block walk + staged dump).
pub const LANKING_RVIEW_TOGGLE_FIRST_CODE: u32 = 0x5204_2035;
/// Right-row hit areas on the board (slot zero: the viewer's own
/// line; slots 1..=10: the board page).
pub const LANKING_RVIEW_TOGGLE_COUNT: usize = 11;
/// Visible song rows (five staged `LVIEW_EX` hit areas).
pub const LANKING_LVIEW_ROWS: usize = 5;
/// Visible board rows past the self line (ten staged `RVIEW_EX`
/// slots, corroborated by the answer's self-plus-ten shape and
/// the zero-count ten-fill rule).
pub const LANKING_RVIEW_ROWS: usize = 10;

/// Left-list sort key, one per exe-named sort button. Sort
/// directions are ASSUMED ascending (no live click capture yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LankingSort {
    /// `IDC_BUTTON_LANKING_TITLE` (`0x0F`).
    Title,
    /// `IDC_BUTTON_LANKING_DIFF` (`0x10`): by chart levels.
    Diff,
    /// `IDC_BUTTON_LANKING_BPM` (`0x11`).
    Bpm,
    /// `IDC_BUTTON_LANKING_ARTIST` (`0x12`).
    Artist,
}

impl LankingSort {
    /// Sort key behind one staged control code (`None` elsewhere).
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        match code {
            LANKING_SORT_TITLE_CODE => Some(Self::Title),
            LANKING_SORT_DIFF_CODE => Some(Self::Diff),
            LANKING_SORT_BPM_CODE => Some(Self::Bpm),
            LANKING_SORT_ARTIST_CODE => Some(Self::Artist),
            _ => None,
        }
    }
}

/// One board click that the exe codes name: sorts, pagers, and row
/// picks. Anything else (tiers, search, REFRESH, the `RVIEW_EX`
/// toggles) lands nowhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LankingAction {
    /// A sort button answered.
    Sort(LankingSort),
    /// Left pager stepped back.
    LViewPrev,
    /// Left pager stepped forward.
    LViewNext,
    /// Right pager stepped back.
    RViewPrev,
    /// Right pager stepped forward.
    RViewNext,
    /// Left-list visible slot picked (0-based on the page).
    SelectRow(usize),
}

/// Board action under native pixel (`x`, `y`): the staged placement
/// whose parsed `.bnd` rect contains the point and whose exe code
/// names a sort, a pager, a left-row hit area, or the row LANK
/// button (owner rule `f38f3`: geometry from parsed rects, never
/// literals). `None` elsewhere — unnamed controls stay display-only.
#[must_use]
pub fn lanking_action_at(placements: &[ScenePlacement], x: i32, y: i32) -> Option<LankingAction> {
    // The shared row LANK button tiles all eleven rows by the
    // staged pitch, so its slots hit-test before the exact pass
    // (the template rect alone only covers row zero).
    if let Some(slot) = lanking_lank_slot(placements, x, y) {
        return Some(LankingAction::SelectRow(slot));
    }
    placements.iter().find_map(|placed| {
        if !super::paint::rect_hit(&placed.rect, x, y) {
            return None;
        }
        lanking_action_for_code(placed.code)
    })
}

/// Action behind one hit control code (`None` for the shared row
/// LANK button — its slots resolve through the staged left slots
/// in [`lanking_lank_slot`] — and for every unnamed control,
/// including the `RVIEW_EX` toggles, which stay display-only OPEN).
fn lanking_action_for_code(code: u32) -> Option<LankingAction> {
    if let Some(sort) = LankingSort::from_code(code) {
        return Some(LankingAction::Sort(sort));
    }
    match code {
        LANKING_LPAGE_PREV_CODE => Some(LankingAction::LViewPrev),
        LANKING_LPAGE_NEXT_CODE => Some(LankingAction::LViewNext),
        LANKING_RPAGE_PREV_CODE => Some(LankingAction::RViewPrev),
        LANKING_RPAGE_NEXT_CODE => Some(LankingAction::RViewNext),
        _ => lanking_lview_row(code),
    }
}

/// Left-list slot behind one `LVIEW_EX` hit-area code (`None`
/// outside the five).
fn lanking_lview_row(code: u32) -> Option<LankingAction> {
    let slot = code.checked_sub(LANKING_LVIEW_TOGGLE_FIRST_CODE)?;
    let slot = usize::try_from(slot).ok()?;
    (slot < LANKING_LVIEW_TOGGLE_COUNT).then_some(LankingAction::SelectRow(slot))
}

/// Staged left-row hit areas in top-to-bottom order (the five
/// `LVIEW_EX` rects tiling the song box).
#[must_use]
pub fn lanking_lview_slots(placements: &[ScenePlacement]) -> Vec<Rect> {
    lanking_slot_rects(
        placements,
        LANKING_LVIEW_TOGGLE_FIRST_CODE,
        LANKING_LVIEW_TOGGLE_COUNT,
    )
}

/// Staged right-row hit areas in top-to-bottom order (slot zero:
/// the viewer's own line; the rest: the board page).
#[must_use]
pub fn lanking_rview_slots(placements: &[ScenePlacement]) -> Vec<Rect> {
    lanking_slot_rects(
        placements,
        LANKING_RVIEW_TOGGLE_FIRST_CODE,
        LANKING_RVIEW_TOGGLE_COUNT,
    )
}

/// Staged hit areas for one contiguous toggle run, top-to-bottom.
fn lanking_slot_rects(placements: &[ScenePlacement], first: u32, count: usize) -> Vec<Rect> {
    let mut slots: Vec<Rect> = placements
        .iter()
        .filter(|placed| {
            placed
                .code
                .checked_sub(first)
                .is_some_and(|slot| (slot as usize) < count)
        })
        .map(|placed| placed.rect)
        .collect();
    slots.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)));
    slots
}

/// Left-list slot behind the shared row LANK button: the staged
/// template tiled over the staged left slots — the same tiling the
/// painter uses, so painted LANK art answers clicks (`None` when
/// the template or the slots are unstaged, or outside the staged
/// slots).
fn lanking_lank_slot(placements: &[ScenePlacement], x: i32, y: i32) -> Option<usize> {
    let template = placements
        .iter()
        .find(|placed| placed.code == LANKING_ROW_LANK_CODE)?;
    let slots = lanking_lview_slots(placements);
    let base = slots.first()?;
    slots.iter().enumerate().find_map(|(slot, area)| {
        let rect = lanking_tiled_rect(template.rect, base, area);
        super::paint::rect_hit(&rect, x, y).then_some(slot)
    })
}

/// One tiled slot rect: the template shifted by the staged slot
/// origin off the staged base origin (no pitch inference — every
/// slot rides its own staged rect).
#[must_use]
pub fn lanking_tiled_rect(template: Rect, base: &Rect, slot: &Rect) -> Rect {
    Rect {
        x: template.x + slot.x - base.x,
        y: template.y + slot.y - base.y,
        w: template.w,
        h: template.h,
    }
}

/// One left-list song: catalog identity the `LVIEW` texts name
/// (owned copies so the board outlives the parse borrow).
#[derive(Debug, Clone, PartialEq)]
pub struct LankingSong {
    /// Song id (matches the `o2ma###` file number).
    pub song_id: u32,
    /// Song title (`TITLE` cell).
    pub title: String,
    /// Song artist (`ARTIST` cell).
    pub artist: String,
    /// Label area (`EDITOR` cell, e.g. `NoteFactory`).
    pub editor: String,
    /// Category id (`GENRE` cell; names unpinned — see
    /// [`lanking_genre_caption`]).
    pub genre: u32,
    /// Tempo in BPM (`BPM` cell).
    pub bpm: f32,
    /// Chart levels Easy/Normal/Hard (`DIFF` cell and sort key;
    /// the `level_flags` halves plus the `stable_minor` low word —
    /// the chart-file read shape, ASSUMED on the list record).
    pub levels: [u16; 3],
}

/// Left-list songs in catalog order, dead slots filtered out
/// ([`OjnListEntry::is_song`]); empty without catalog data.
#[must_use]
pub fn lanking_songs(entries: &[OjnListEntry<'_>]) -> Vec<LankingSong> {
    entries
        .iter()
        .filter(|entry| entry.is_song())
        .map(|entry| LankingSong {
            song_id: entry.song_id,
            title: entry.title.clone(),
            artist: entry.artist.clone(),
            editor: entry.label.clone(),
            genre: entry.genre,
            bpm: entry.bpm,
            levels: lanking_levels(entry),
        })
        .collect()
}

/// Chart levels off one list record: the `level_flags` halves read
/// Easy/Normal and the `stable_minor` low word reads Hard on the
/// chart file (`crates/o2jam-file/src/ojn.rs`); the same shape here
/// is ASSUMED (values stay list-shaped 0..50, clamped by `u16`).
fn lanking_levels(entry: &OjnListEntry<'_>) -> [u16; 3] {
    let easy = u16::try_from(entry.level_flags & 0xFFFF).unwrap_or(u16::MAX);
    let normal = u16::try_from(entry.level_flags >> 16).unwrap_or(u16::MAX);
    let hard = u16::try_from(entry.stable_minor & 0xFFFF).unwrap_or(u16::MAX);
    [easy, normal, hard]
}

/// Sort one board page in place (stable: ties keep catalog order).
/// Directions are ASSUMED ascending — next step: live sort-click
/// capture per the `~/scratch/planet-verify/` recipe.
pub fn sort_lanking_songs(songs: &mut [LankingSong], sort: LankingSort) {
    match sort {
        LankingSort::Title => songs.sort_by(|a, b| a.title.cmp(&b.title)),
        LankingSort::Artist => songs.sort_by(|a, b| a.artist.cmp(&b.artist)),
        LankingSort::Bpm => songs.sort_by(|a, b| a.bpm.total_cmp(&b.bpm)),
        LankingSort::Diff => songs.sort_by(|a, b| {
            a.levels[2]
                .cmp(&b.levels[2])
                .then(a.levels[1].cmp(&b.levels[1]))
                .then(a.levels[0].cmp(&b.levels[0]))
        }),
    }
}

/// Visible window of a `per_page`-row list on `page` (clamped:
/// past-the-end pages read empty, never wrap).
#[must_use]
pub fn lanking_page_window(len: usize, per_page: usize, page: usize) -> (usize, usize) {
    let start = page.saturating_mul(per_page).min(len);
    let end = start.saturating_add(per_page).min(len);
    (start, end)
}

/// Genre cell text: the reference-transcribed filter caption
/// ([`SELECT_GENRE_FILTERS`]) when the id names one, else the raw
/// id digits (genre names stay unpinned — stand-in, documented
/// with the filters).
#[must_use]
pub fn lanking_genre_caption(genre: u32) -> String {
    let index = usize::try_from(genre).unwrap_or(usize::MAX);
    if (1..SELECT_GENRE_FILTERS.len()).contains(&index) {
        return SELECT_GENRE_FILTERS[index].to_string();
    }
    genre.to_string()
}

/// Left-list `DIFF` cell text (`easy/normal/hard`; format
/// ours-by-design, values catalog DATA).
#[must_use]
pub fn lanking_diff_caption(levels: [u16; 3]) -> String {
    format!("{}/{}/{}", levels[0], levels[1], levels[2])
}

/// Left-list `BPM` cell text (tempo digits, no float compare).
#[must_use]
pub fn lanking_bpm_caption(bpm: f32) -> String {
    format!("{bpm}")
}

/// Right-list row texts off one fetched 2028 rank line, in `RVIEW`
/// text order: rank place, nickname, battles, wins, win rate,
/// movement arrow (`UP`/`DOWN` ASCII stand-ins — arrow rendering
/// unpinned). A zero-rank line is the exe's display fill (the
/// zeroed-row arm, `docs/overnight-evidence/u4-ranking-packs.md`):
/// every cell stays blank.
#[must_use]
pub fn lanking_rank_cells(entry: &RankEntry) -> [String; 6] {
    if entry.rank == 0 {
        return [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
    }
    let arrow = match entry.delta {
        RankDelta::Up => "UP",
        RankDelta::Down => "DOWN",
    };
    [
        entry.rank.to_string(),
        entry.nickname.clone(),
        entry.battles.to_string(),
        entry.wins.to_string(),
        entry.win_rate.to_string(),
        arrow.to_string(),
    ]
}

/// Billing page behind `STATE_PAYMENT` (lifecycle §12 `CDPayment` view).
pub const PAYMENT_PAGE: &str = "payment_input.aspx";
/// Billing page behind `STATE_PAYMENT2` (gift view).
pub const GIFT_PAGE: &str = "gift_input.aspx";

/// One billing view: its `ControlList` block and its web page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaymentLabel {
    /// `ControlList` block this view paints.
    pub block: &'static str,
    /// Billing page suffix (host comes from the web bridge).
    pub page: &'static str,
}

/// Both billing views in block order: payment, then gift.
#[must_use]
pub const fn payment_labels() -> [PaymentLabel; 2] {
    [
        PaymentLabel {
            block: PAYMENT_BLOCK,
            page: PAYMENT_PAGE,
        },
        PaymentLabel {
            block: PAYMENT2_BLOCK,
            page: GIFT_PAGE,
        },
    ]
}

/// Lifecycle-verified path behind `STATE_PAYMENT`: the
/// `.../np/billing/payment/payment_input.aspx` web view (lifecycle §12).
/// The host comes from the web bridge; only the path is staged.
pub const PAYMENT_URL_PATH: &str = "np/billing/payment/payment_input.aspx";
/// Lifecycle-verified path behind `STATE_PAYMENT2` (gift view): the
/// `.../billing/gift/gift_input.aspx` web view (lifecycle §12).
pub const GIFT_URL_PATH: &str = "billing/gift/gift_input.aspx";

impl PaymentLabel {
    /// Lifecycle-verified URL path for this view (payment vs gift by block).
    #[must_use]
    pub fn url_path(self) -> &'static str {
        if self.block == PAYMENT2_BLOCK {
            GIFT_URL_PATH
        } else {
            PAYMENT_URL_PATH
        }
    }
}

/// View caption pairing the block with its billing path
/// (`STATE_PAYMENT np/billing/payment/payment_input.aspx`).
#[must_use]
pub fn payment_view_caption(label: &PaymentLabel) -> String {
    format!("{} {}", label.block, label.url_path())
}

/// Live wallet balances for the payment/result displays, fed only by
/// decoded wire data: the 5029 two-word refresh (`SyncCashPointResponse`)
/// and the 5001 purchase words (`SyncItemPurchasePrefix` plus
/// `SyncItemPurchaseWallets`). Every amount starts unknown so the
/// screen never shows a value without data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WalletState {
    /// Gem balance (5029 word 1, 5001 prefix word 1).
    pub gem: Option<i32>,
    /// Cash points (5029 word 2, 5001 prefix word 4).
    pub cash_point: Option<i32>,
    /// Distribution points (5001 prefix word 2).
    pub point: Option<i32>,
    /// Premium cash baseline (5001 prefix word 3).
    pub o2_cash: Option<i32>,
    /// Music-shop cash (5001 wallet word 1).
    pub music_cash: Option<i32>,
    /// Item-shop cash (5001 wallet word 2).
    pub item_cash: Option<i32>,
}

impl WalletState {
    /// Fold a 5029 wallet refresh into the wallet.
    pub fn apply_cash_point(&mut self, response: &SyncCashPointResponse) {
        self.gem = Some(response.gem);
        self.cash_point = Some(response.cash_point);
    }

    /// Fold a 5003 sell refund into the wallet: the success gem word
    /// shares the gem slot the 5029/5001 words update. Error verdicts
    /// carry no gem word, so the balance stays untouched.
    pub fn apply_sell_response(&mut self, response: &SellItemResponse) {
        if let Some(gem) = response.gem {
            self.gem = Some(gem);
        }
    }

    /// Fold the 5001 purchase prefix words into the wallet.
    pub fn apply_purchase_prefix(&mut self, prefix: &SyncItemPurchasePrefix) {
        self.gem = Some(prefix.gem);
        self.point = Some(prefix.point);
        self.o2_cash = Some(prefix.o2_cash);
        self.cash_point = Some(prefix.cash_point);
    }

    /// Fold the 5001 post-list wallet pair into the wallet.
    pub fn apply_purchase_wallets(&mut self, wallets: &SyncItemPurchaseWallets) {
        self.music_cash = Some(wallets.music_cash);
        self.item_cash = Some(wallets.item_cash);
    }

    /// Wallet rows in display order: gem first, then cash lines.
    #[must_use]
    pub fn rows(self) -> [WalletRow; WALLET_ROW_COUNT] {
        [
            WalletRow::named(GEM_LABEL, self.gem),
            WalletRow::named(CASH_POINT_LABEL, self.cash_point),
            WalletRow::named(POINT_LABEL, self.point),
            WalletRow::named(O2_CASH_LABEL, self.o2_cash),
            WalletRow::named(MUSIC_CASH_LABEL, self.music_cash),
            WalletRow::named(ITEM_CASH_LABEL, self.item_cash),
        ]
    }
}

/// One wallet row: a label plus its live amount (`None` until the
/// matching wire words arrive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletRow {
    /// Row label (a `*_LABEL` constant below).
    pub label: &'static str,
    /// Live amount; `None` means no wire data yet.
    pub amount: Option<i32>,
}

impl WalletRow {
    /// Pair a label with its amount.
    #[must_use]
    pub const fn named(label: &'static str, amount: Option<i32>) -> Self {
        Self { label, amount }
    }
}

/// Wallet rows behind the payment/result displays, in order.
pub const WALLET_ROW_COUNT: usize = 6;
/// Gem row label (TOTAL GEM bar and first wallet row).
pub const GEM_LABEL: &str = "GEM";
/// Cash-point row label.
pub const CASH_POINT_LABEL: &str = "CASH POINT";
/// Distribution-point row label.
pub const POINT_LABEL: &str = "POINT";
/// Premium-cash row label.
pub const O2_CASH_LABEL: &str = "O2 CASH";
/// Music-shop cash row label.
pub const MUSIC_CASH_LABEL: &str = "MUSIC CASH";
/// Item-shop cash row label.
pub const ITEM_CASH_LABEL: &str = "ITEM CASH";
/// Placeholder where no wire data arrived yet (never a zero).
pub const UNKNOWN_AMOUNT: &str = "--";

/// Format one amount: digits from wire data, placeholder otherwise.
#[must_use]
pub fn wallet_amount_caption(amount: Option<i32>) -> String {
    amount.map_or_else(|| UNKNOWN_AMOUNT.to_owned(), |value| value.to_string())
}

/// TOTAL GEM bar caption (`TOTAL GEM 1200`); unknown until a gem
/// word arrives.
#[must_use]
pub fn total_gem_caption(state: &WalletState) -> String {
    format!("TOTAL {} {}", GEM_LABEL, wallet_amount_caption(state.gem))
}

/// One wallet row caption (`GEM 1200`); unknown amounts stay blank.
#[must_use]
pub fn wallet_row_caption(row: &WalletRow) -> String {
    format!("{} {}", row.label, wallet_amount_caption(row.amount))
}

/// Every wallet row caption in display order.
#[must_use]
pub fn wallet_row_captions(state: &WalletState) -> Vec<String> {
    state.rows().iter().map(wallet_row_caption).collect()
}

/// Live free-pass status for the payment display, fed only by the
/// decoded 5026 body (`SyncFreePassResponse`): the result code plus
/// the pass kind and expiry seconds when the body carries a snapshot.
/// Every field starts unknown so the screen never shows a value
/// without data; nothing beyond these decoded fields reaches the
/// screen (the tail stays exe-ignored per `docs/gifts.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FreePassState {
    /// Decoded result code (`None`: no 5026 yet, or an unknown code).
    pub result: Option<SyncFreePassResult>,
    /// Active pass kind (only when the body carries a snapshot).
    pub kind: Option<FreePassType>,
    /// Pass seconds remaining (only when the body carries a snapshot).
    pub expiry_secs: Option<i32>,
}

impl FreePassState {
    /// Fold a 5026 response into the display state. A result-only body
    /// carries no snapshot, so kind and expiry clear back to unknown.
    pub fn apply_free_pass_response(&mut self, response: &SyncFreePassResponse) {
        self.result = SyncFreePassResult::from_u32(response.result_code);
        let snapshot = response.info.as_ref();
        self.kind = snapshot.map(|info| info.free_pass_type);
        self.expiry_secs = snapshot.map(|info| info.expiry_secs);
    }
}

/// Free-pass result row label.
pub const FREE_PASS_RESULT_LABEL: &str = "FREE PASS RESULT";
/// Free-pass kind row label.
pub const FREE_PASS_KIND_LABEL: &str = "FREE PASS";
/// Free-pass expiry row label.
pub const FREE_PASS_EXPIRY_LABEL: &str = "EXPIRY";
/// Free-pass rows behind the payment display, in order.
pub const FREE_PASS_ROW_COUNT: usize = 3;

/// Display name for one decoded pass kind: every known
/// `FreePassType` flag names itself, never a wire guess.
#[must_use]
pub const fn free_pass_kind_label(kind: FreePassType) -> &'static str {
    match kind {
        FreePassType::None => "NONE",
        FreePassType::AllMusic => "ALL MUSIC",
        FreePassType::UnusedFreePlay1 => "UNUSED",
        FreePassType::NewMusic => "NEW MUSIC",
        FreePassType::FreePlay => "FREE PLAY",
        FreePassType::FreePlayAlt1 => "FREE PLAY",
        FreePassType::Top100 => "TOP 100",
        FreePassType::PremiumTime => "PREMIUM TIME",
        FreePassType::UnusedFreePlay2 => "UNUSED",
        FreePassType::Top100Extended => "TOP 100 EXTENDED",
        FreePassType::FreePlayAlt2 => "FREE PLAY",
        FreePassType::UnusedFreePlay3 => "UNUSED",
        FreePassType::FreePlayAlt3 => "FREE PLAY",
    }
}

/// Format one decoded result: known verdicts name themselves,
/// unknown codes stay at the placeholder (never guessed).
#[must_use]
pub fn free_pass_result_caption(result: Option<SyncFreePassResult>) -> String {
    match result {
        Some(SyncFreePassResult::Success) => "SUCCESS".to_owned(),
        Some(SyncFreePassResult::Unknown) => "UNKNOWN".to_owned(),
        None => UNKNOWN_AMOUNT.to_owned(),
    }
}

/// Format one decoded pass kind, placeholder until a snapshot arrives.
#[must_use]
pub fn free_pass_kind_caption(kind: Option<FreePassType>) -> String {
    kind.map_or_else(
        || UNKNOWN_AMOUNT.to_owned(),
        |pass| free_pass_kind_label(pass).to_owned(),
    )
}

/// Format one decoded expiry: seconds digits from wire data,
/// placeholder otherwise (reuses the wallet unknown rule).
#[must_use]
pub fn free_pass_expiry_caption(expiry_secs: Option<i32>) -> String {
    wallet_amount_caption(expiry_secs)
}

/// One free-pass row caption (`FREE PASS PREMIUM TIME`); unknown
/// fields stay at the placeholder.
#[must_use]
pub fn free_pass_row_caption(label: &str, caption: &str) -> String {
    format!("{label} {caption}")
}

/// Every free-pass row caption in display order: result, kind, expiry.
#[must_use]
pub fn free_pass_row_captions(state: &FreePassState) -> Vec<String> {
    vec![
        free_pass_row_caption(
            FREE_PASS_RESULT_LABEL,
            &free_pass_result_caption(state.result),
        ),
        free_pass_row_caption(FREE_PASS_KIND_LABEL, &free_pass_kind_caption(state.kind)),
        free_pass_row_caption(
            FREE_PASS_EXPIRY_LABEL,
            &free_pass_expiry_caption(state.expiry_secs),
        ),
    ]
}

/// Tutorial backdrop figure from the archive (`Playing1.opi` art for
/// the stub-only `CStateTutorial` block). `None` when the entry is
/// missing or undecodable: the screen still paints, just without rows.
#[must_use]
pub fn load_tutorial_backdrop(archive: &[u8]) -> Option<AvatarFigure> {
    load_archive_sprite(archive, TUTORIAL_BACKGROUND_SPRITE)
}

/// Pair a side block with its `.bnd` rects.
#[must_use]
pub fn bind_side_scene(block: &ControlBlock, bnd: &Bnd) -> Vec<ScenePlacement> {
    bind_scene_rects(block, bnd)
}

/// Paint placed side figures at their layout origins (placements
/// without a loaded figure are skipped).
pub fn paint_side_scene(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
) {
    paint_scene_figures(frame, placements, figures);
}

#[cfg(test)]
mod side_tests {
    use super::*;

    fn side_control_text() -> String {
        [
            "NUMBER_OF_STATE 0x03",
            "STATE_SHOP 0x02",
            "{",
            "BOUND \"State_ItemShop.bnd\"",
            "IDC_SHOP 0X00100000",
            "IDC_ITEM 0X10500001 \"Shop_Item.ojs\"",
            "IDC_GEM 0X10500002 \"Shop_ItemGem.ojs\"",
            "}",
            "STATE_PAYMENT 0x01",
            "{",
            "BOUND \"State_Payment.bnd\"",
            "IDC_PAY 0X00100000",
            "IDC_BILL 0X10500001 \"Pay_Bill.ojs\"",
            "}",
            "STATE_PAYMENT2 0x01",
            "{",
            "BOUND \"State_Payment2.bnd\"",
            "IDC_PAY2 0X00100000",
            "IDC_GIFT 0X10500001 \"Pay_Gift.ojs\"",
            "}",
            "",
        ]
        .join("\n")
    }

    /// Minimal container: header + one record + blob (mirrors the
    /// `o2jam-file` synthetic builders).
    fn sprite55_blob(tag: u16, colorkey: u16, w: u16, h: u16, pixels: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&tag.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&colorkey.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&w.to_le_bytes());
        blob.extend_from_slice(&h.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(pixels);
        blob
    }

    fn archive_blob(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        use o2jam_file::{
            OPI_ENTRY_LEN, OPI_HEADER_LEN, OPI_NAME_OFF, OPI_OFFSET_OFF, OPI_RESERVED_OFF,
            OPI_SIZE_OFF,
        };
        let mut blob = vec![0u8; OPI_HEADER_LEN];
        blob[..4].copy_from_slice(&1u32.to_le_bytes());
        blob[4..8].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        let mut off = OPI_HEADER_LEN as u32;
        let mut table = Vec::new();
        for (name, body) in bodies {
            let mut entry = vec![0u8; OPI_ENTRY_LEN];
            entry[..4].copy_from_slice(&1u32.to_le_bytes());
            entry[OPI_NAME_OFF..OPI_NAME_OFF + name.len()].copy_from_slice(name.as_bytes());
            entry[OPI_OFFSET_OFF..OPI_OFFSET_OFF + 4].copy_from_slice(&off.to_le_bytes());
            entry[OPI_SIZE_OFF..OPI_SIZE_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            entry[OPI_RESERVED_OFF..OPI_RESERVED_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            table.push(entry);
            off += body.len() as u32;
        }
        for body in bodies.iter().map(|(_, body)| body) {
            blob.extend_from_slice(body);
        }
        for entry in table {
            blob.extend_from_slice(&entry);
        }
        blob
    }

    fn bnd_blob(rects: &[(u32, u32, u32, u32)]) -> Vec<u8> {
        let mut blob = vec![0xFF; 4];
        blob.extend_from_slice(&(rects.len() as u16).to_le_bytes());
        for (x1, y1, x2, y2) in rects {
            blob.extend_from_slice(&x1.to_le_bytes());
            blob.extend_from_slice(&y1.to_le_bytes());
            blob.extend_from_slice(&x2.to_le_bytes());
            blob.extend_from_slice(&y2.to_le_bytes());
        }
        blob
    }

    #[test]
    fn lists_all_seven_side_screens() {
        assert_eq!(ALL_SIDE_SCREENS.len(), 7);
        for side in ALL_SIDE_SCREENS {
            assert!(ALL_SIDE_SCREENS.contains(&side));
        }
    }

    #[test]
    fn shop_maps_to_item_shop_block() {
        assert_eq!(SideScreen::Shop.block_name(), Some(SHOP_BLOCK));
        assert_eq!(SideScreen::Shop.bind_file(), Some(SHOP_BIND_FILE));
        assert_eq!(SideScreen::Shop.state_class(), Some("CStateShop"));
    }

    #[test]
    fn music_shop_maps_to_mshop_block() {
        assert_eq!(SideScreen::MusicShop.block_name(), Some(MUSIC_SHOP_BLOCK));
        assert_eq!(
            SideScreen::MusicShop.bind_file(),
            Some(MUSIC_SHOP_BIND_FILE)
        );
        assert_eq!(SideScreen::MusicShop.state_class(), Some("CStateMShop"));
    }

    #[test]
    fn myroom_maps_to_avatar_room_block() {
        assert_eq!(SideScreen::MyRoom.block_name(), Some(MYROOM_BLOCK));
        assert_eq!(SideScreen::MyRoom.bind_file(), Some(MYROOM_BIND_FILE));
        assert_eq!(SideScreen::MyRoom.state_class(), None);
    }

    #[test]
    fn planet_maps_to_solo_mission_block() {
        assert_eq!(SideScreen::Planet.block_name(), Some(PLANET_BLOCK));
        assert_eq!(SideScreen::Planet.bind_file(), Some(PLANET_BIND_FILE));
        assert_eq!(SideScreen::Planet.state_class(), Some("CStatePlanet"));
    }

    #[test]
    fn side_bgm_cues_follow_the_pinned_table() {
        assert_eq!(SideScreen::Shop.bgm_cue(), (4, true));
        assert_eq!(SideScreen::MusicShop.bgm_cue(), (10, true));
        assert_eq!(SideScreen::MyRoom.bgm_cue(), (5, true));
        assert_eq!(SideScreen::Planet.bgm_cue(), (9, true));
        assert_eq!(SideScreen::Payment.bgm_cue(), (3, true));
        assert_eq!(
            SideScreen::Lanking.bgm_cue(),
            (o2jam_audio::BGM_SILENCE_ID, false)
        );
        assert_eq!(
            SideScreen::Tutorial.bgm_cue(),
            (o2jam_audio::BGM_SILENCE_ID, false)
        );
    }

    #[test]
    fn planet_button_hits_follow_the_pinned_bnd_rects() {
        // Hit-test inverse of the pinned `State_Planet.bnd` rects
        // (docs/planet.md bind walk): the buttons do NOT run
        // left-to-right in file order — HARD sits center at x313.
        let center = |rect: &Rect| (rect.x + rect.w as i32 / 2, rect.y + rect.h as i32 / 2);
        let (hard_x, hard_y) = center(&PLANET_HARD_RECT);
        let (normal_x, normal_y) = center(&PLANET_NORMAL_RECT);
        let (easy_x, easy_y) = center(&PLANET_EASY_RECT);
        let (free_x, free_y) = center(&PLANET_FREE_RECT);
        let (drum_x, drum_y) = center(&PLANET_DRUM_RECT);
        assert_eq!(planet_hit(hard_x, hard_y), Some(PlanetHit::Hard));
        assert_eq!(planet_hit(normal_x, normal_y), Some(PlanetHit::Normal));
        assert_eq!(planet_hit(easy_x, easy_y), Some(PlanetHit::Easy));
        assert_eq!(planet_hit(free_x, free_y), Some(PlanetHit::Free));
        assert_eq!(planet_hit(drum_x, drum_y), Some(PlanetHit::Drum));
        // Buttons are disjoint bands across the same strip.
        assert!(hard_x > normal_x && normal_x > easy_x);
        assert!(free_x > drum_x && drum_x > hard_x);
        let _ = (easy_y, drum_y);
    }

    #[test]
    fn planet_button_hit_edges_are_exclusive() {
        // Right/bottom edges exclusive, tops/lefts inclusive — the
        // shared `rect_hit` convention the staged path already uses.
        assert_eq!(planet_hit(313, 32), Some(PlanetHit::Hard));
        assert_eq!(planet_hit(458, 143), Some(PlanetHit::Hard));
        assert_eq!(planet_hit(459, 32), None);
        assert_eq!(planet_hit(313, 144), None);
        assert_eq!(planet_hit(86, 32), None);
        // The strip between buttons and the promotion image hits
        // nothing (rect 11 starts at y163).
        assert_eq!(planet_hit(400, 150), None);
    }

    #[test]
    fn planet_free_and_drum_clicks_stay_unbound() {
        // P0: FREE (tutorial) and DRUM (quest) back ends were never
        // wired; their clicks hit the pinned rects but must not
        // select anything or advance any flow.
        let mut selection = PlanetSelection::new();
        assert_eq!(selection.band(), None);
        let free = selection.click(614 + 49, 32 + 56).unwrap();
        assert_eq!(free, PlanetHit::Free);
        assert_eq!(free.band(), None);
        assert_eq!(selection.band(), None, "FREE click must stay unbound");
        let drum = selection.click(510 + 49, 32 + 56).unwrap();
        assert_eq!(drum, PlanetHit::Drum);
        assert_eq!(selection.band(), None, "DRUM click must stay unbound");
        // HARD then records the pick; FREE again does not clear it.
        selection.click(313 + 73, 32 + 56);
        assert_eq!(selection.band(), Some(PlanetBand::Hard));
        selection.click(614 + 49, 32 + 56);
        assert_eq!(selection.band(), Some(PlanetBand::Hard));
    }

    #[test]
    fn planet_band_rects_match_the_pinned_buttons() {
        assert_eq!(planet_band_rect(PlanetBand::Hard), PLANET_HARD_RECT);
        assert_eq!(planet_band_rect(PlanetBand::Normal), PLANET_NORMAL_RECT);
        assert_eq!(planet_band_rect(PlanetBand::Easy), PLANET_EASY_RECT);
    }

    #[test]
    fn planet_dispatch_table_maps_codes_to_actions() {
        // One table, keyed by control code — the game never branches
        // per-button. Emblems name their family band; the dimmed
        // emblems name none (P0); power and gear name their dialogs.
        assert_eq!(
            planet_action_for_code(PLANET_BUTTON_HARD_CODE),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Hard)
            })
        );
        assert_eq!(
            planet_action_for_code(PLANET_BUTTON_NORMAL_CODE),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Normal)
            })
        );
        assert_eq!(
            planet_action_for_code(PLANET_BUTTON_EASY_CODE),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Easy)
            })
        );
        assert_eq!(
            planet_action_for_code(PLANET_BUTTON_FREE_CODE),
            Some(PlanetAction::Emblem { band: None })
        );
        assert_eq!(
            planet_action_for_code(PLANET_BUTTON_DRUM_CODE),
            Some(PlanetAction::Emblem { band: None })
        );
        assert_eq!(
            planet_action_for_code(PLANET_EXIT_CODE),
            Some(PlanetAction::QuitConfirm)
        );
        assert_eq!(
            planet_action_for_code(PLANET_OPTION_CODE),
            Some(PlanetAction::OptionsDialog)
        );
        assert_eq!(
            planet_action_for_code(PLANET_PROMO_LEFT_CODE),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Left
            })
        );
        assert_eq!(
            planet_action_for_code(PLANET_PROMO_STOP_CODE),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Stop
            })
        );
        assert_eq!(
            planet_action_for_code(PLANET_PROMO_RIGHT_CODE),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Right
            })
        );
        // Self/BACK codes never dispatch.
        assert_eq!(planet_action_for_code(0x1900_0000), None);
    }

    #[test]
    fn planet_promo_cells_answer_clicks_at_their_staged_rects() {
        // Staged placements carry the parsed `.bnd` rects verbatim
        // (rects 12..14: LEFT (353,511)-(379,535), STOP
        // (387,511)-(413,535), RIGHT (421,511)-(447,535) —
        // `docs/planet.md` bind walk, ordinal pairing). Each cell
        // answers inside its rect and stays silent outside it.
        let staged = vec![
            ScenePlacement {
                id: "IDC_BUTTON_SPLANET_PROMOTION_LEFT".to_owned(),
                code: PLANET_PROMO_LEFT_CODE,
                sprite: None,
                rect: Rect {
                    x: 353,
                    y: 511,
                    w: 26,
                    h: 24,
                },
            },
            ScenePlacement {
                id: "IDC_BUTTON_SPLANET_PROMOTION_PLAYSTOP".to_owned(),
                code: PLANET_PROMO_STOP_CODE,
                sprite: None,
                rect: Rect {
                    x: 387,
                    y: 511,
                    w: 26,
                    h: 24,
                },
            },
            ScenePlacement {
                id: "IDC_BUTTON_SPLANET_PROMOTION_RIGHT".to_owned(),
                code: PLANET_PROMO_RIGHT_CODE,
                sprite: None,
                rect: Rect {
                    x: 421,
                    y: 511,
                    w: 26,
                    h: 24,
                },
            },
        ];
        assert_eq!(
            planet_action_at(&staged, 366, 523),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Left
            })
        );
        assert_eq!(
            planet_action_at(&staged, 400, 523),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Stop
            })
        );
        assert_eq!(
            planet_action_at(&staged, 434, 523),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Right
            })
        );
        assert_eq!(planet_action_at(&staged, 400, 500), None);
        // Without staged art the promo cells are unpinned: no
        // fallback literals (ADR-0003), so clicks land nowhere.
        assert_eq!(planet_action_at(&[], 366, 523), None);
        assert_eq!(planet_action_at(&[], 400, 523), None);
        assert_eq!(planet_action_at(&[], 434, 523), None);
    }

    #[test]
    fn planet_dispatch_hit_tests_the_staged_bnd_rects() {
        // Staged placements win: the EXIT action resolves through
        // the placement carrying the exit code and its parsed rect,
        // wherever that rect sits (here deliberately off-fallback).
        let staged = vec![ScenePlacement {
            id: "IDC_BUTTON_SPLANET_EXIT".to_owned(),
            code: PLANET_EXIT_CODE,
            sprite: Some("Planet_Btn_Exit.ojs".to_owned()),
            rect: Rect {
                x: 10,
                y: 10,
                w: 20,
                h: 20,
            },
        }];
        assert_eq!(
            planet_action_at(&staged, 15, 15),
            Some(PlanetAction::QuitConfirm)
        );
        assert_eq!(planet_action_at(&staged, 15, 35), None);
        // Without placements the pinned fallback still resolves the
        // emblems and both MENU discs (art-less parity).
        assert_eq!(
            planet_action_at(&[], 386, 88),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Hard)
            })
        );
        assert_eq!(
            planet_action_at(&[], 769, 580),
            Some(PlanetAction::QuitConfirm)
        );
        assert_eq!(
            planet_action_at(&[], 729, 580),
            Some(PlanetAction::OptionsDialog)
        );
        assert_eq!(planet_action_at(&[], 400, 300), None);
    }

    #[test]
    fn planet_emblem_captions_decode_to_the_pinned_strings() {
        // Each caption's CP949 bytes decode to its `docs/planet.md`
        // runtime string (file offsets in the table doc above).
        let pinned = [
            (PLANET_TEXT_HARD_CODE, "신의손 (고급)"),
            (PLANET_TEXT_NORMAL_CODE, "고수손 (중급)"),
            (PLANET_TEXT_EASY_CODE, "사람손 (초급)"),
            (PLANET_TEXT_FREE_CODE, "튜토리얼"),
            (PLANET_TEXT_DRUM_CODE, "퀘스트"),
        ];
        assert_eq!(PLANET_CAPTIONS.len(), pinned.len());
        for ((code, expected), caption) in pinned.iter().zip(PLANET_CAPTIONS.iter()) {
            assert_eq!(caption.code, *code);
            assert_eq!(
                planet_caption_for_code(*code),
                Some(*caption),
                "caption lookup for {code:#x}"
            );
            assert_eq!(
                o2jam_text::decode_cp949(caption.cp949).expect("pinned CP949"),
                *expected
            );
        }
        assert_eq!(planet_caption_for_code(0x1940_0006), None);
        assert_eq!(
            o2jam_text::decode_cp949(QUIT_MESSAGE_CP949).expect("quit CP949"),
            "게임을 종료하시겠습니까?"
        );
    }

    #[test]
    fn planet_option_tabs_name_their_blocks_and_exits() {
        assert_eq!(PlanetOptionTab::Key.block_name(), "DIALOG_OPTION_KEY");
        assert_eq!(PlanetOptionTab::Sound.block_name(), "DIALOG_OPTION_SOUND");
        assert_eq!(PlanetOptionTab::Game.block_name(), "DIALOG_OPTION_GAME");
        assert_eq!(PlanetOptionTab::Key.bind_file(), "Dialog_Option_Key.bnd");
        assert_eq!(
            PlanetOptionTab::Sound.bind_file(),
            "Dialog_Option_Sound.bnd"
        );
        assert_eq!(PlanetOptionTab::Game.bind_file(), "Dialog_Option_Game.bnd");
        assert_eq!(PlanetOptionTab::Key.exit_code(), 0x2520_0017);
        assert_eq!(PlanetOptionTab::Sound.exit_code(), 0x2620_0012);
        assert_eq!(PlanetOptionTab::Game.exit_code(), 0x2F20_000F);
        // First-open tab is an OPEN stand-in (file order), never a pin.
        assert_eq!(PLANET_OPTION_FIRST_TAB, PlanetOptionTab::Key);
    }

    #[test]
    fn lanking_is_display_only_without_state_class() {
        assert_eq!(SideScreen::Lanking.block_name(), Some(LANKING_BLOCK));
        assert_eq!(SideScreen::Lanking.bind_file(), Some(LANKING_BIND_FILE));
        assert_eq!(SideScreen::Lanking.state_class(), None);
    }

    #[test]
    fn payment_covers_both_billing_views() {
        assert_eq!(SideScreen::Payment.block_name(), Some(PAYMENT_BLOCK));
        assert_eq!(SideScreen::Payment.bind_file(), Some(PAYMENT_BIND_FILE));
        assert_eq!(SideScreen::Payment.state_class(), Some("CDPayment"));
        assert_eq!(PAYMENT2_BLOCK, "STATE_PAYMENT2");
        assert_eq!(PAYMENT2_BIND_FILE, "State_Payment2.bnd");
    }

    #[test]
    fn tutorial_is_stub_with_backdrop_art() {
        assert_eq!(SideScreen::Tutorial.block_name(), None);
        assert_eq!(SideScreen::Tutorial.bind_file(), None);
        assert_eq!(SideScreen::Tutorial.state_class(), Some("CStateTutorial"));
        assert_eq!(TUTORIAL_BACKGROUND_SPRITE, "Playing_TutorialBG.ojs");
    }

    #[test]
    fn lobbies_visit_every_side_screen() {
        for lobby in [Screen::Channel, Screen::Room, Screen::Waiting] {
            for side in ALL_SIDE_SCREENS {
                assert!(can_visit_side(lobby, side), "{lobby:?} -> {side:?}");
            }
        }
    }

    #[test]
    fn main_flow_never_opens_side_screens() {
        for screen in [
            Screen::Avi,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            for side in ALL_SIDE_SCREENS {
                assert!(!can_visit_side(screen, side), "{screen:?} -> {side:?}");
            }
        }
    }

    #[test]
    fn every_side_returns_to_any_lobby() {
        for side in ALL_SIDE_SCREENS {
            for lobby in [Screen::Channel, Screen::Room, Screen::Waiting] {
                assert!(can_return_side(side, lobby), "{side:?} -> {lobby:?}");
            }
        }
    }

    #[test]
    fn sides_never_return_to_main_flow() {
        for side in ALL_SIDE_SCREENS {
            for screen in [
                Screen::Avi,
                Screen::Loading,
                Screen::Playing,
                Screen::Result,
            ] {
                assert!(!can_return_side(side, screen), "{side:?} -> {screen:?}");
            }
        }
    }

    #[test]
    fn waiting_team_codes_map_run_to_zero_through_seven() {
        assert_eq!(waiting_team_for_code(WAITING_TEAM_FIRST_CODE), Some(0));
        assert_eq!(waiting_team_for_code(0x1620_1005), Some(1));
        assert_eq!(waiting_team_for_code(WAITING_TEAM_LAST_CODE), Some(7));
        assert_eq!(waiting_team_for_code(0x1620_1003), None);
        assert_eq!(waiting_team_for_code(0x1620_100C), None);
    }

    #[test]
    fn create_room_dialog_dispatch_names_every_control() {
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_CREATE_CODE),
            Some(CreateRoomAction::Commit)
        );
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_CANCEL_CODE),
            Some(CreateRoomAction::Cancel)
        );
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_NORMAL_MODE_CODE),
            Some(CreateRoomAction::NormalMode)
        );
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_LIVE_MODE_CODE),
            Some(CreateRoomAction::LiveMode)
        );
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_3KEY_MODE_CODE),
            Some(CreateRoomAction::ThreeKey)
        );
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_7KEY_MODE_CODE),
            Some(CreateRoomAction::SevenKey)
        );
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_TITLE_CODE),
            Some(CreateRoomAction::FocusTitle)
        );
        assert_eq!(
            create_room_action_for_code(CREATE_ROOM_PASSWORD_CODE),
            Some(CreateRoomAction::FocusPassword)
        );
        assert_eq!(create_room_action_for_code(0x2400_0000), None);
        assert_eq!(
            password_action_for_code(PASSWORD_OK_CODE),
            Some(PasswordAction::Commit)
        );
        assert_eq!(
            password_action_for_code(PASSWORD_CANCEL_CODE),
            Some(PasswordAction::Cancel)
        );
    }

    #[test]
    fn create_room_form_commit_gates_title_and_locks_password() {
        let mut form = CreateRoomForm::new();
        assert!(form.commit().is_none());
        for ch in "Jam Room".chars() {
            form.push_title_char(ch);
        }
        let open = form.commit().expect("titled form commits");
        assert_eq!(open.title, "Jam Room");
        assert!(!open.has_password);
        form.push_password_char('x');
        let locked = form.commit().expect("titled form commits");
        assert!(locked.has_password);
        assert_eq!(locked.password, "x");
    }

    #[test]
    fn room_dialog_forms_bound_text_at_caps() {
        let mut form = CreateRoomForm::new();
        for _ in 0..30 {
            form.push_title_char('a');
        }
        assert_eq!(form.title.chars().count(), 22);
        for _ in 0..20 {
            form.push_password_char('b');
        }
        assert_eq!(form.password.chars().count(), 12);
        form.pop_title();
        assert_eq!(form.title.chars().count(), 21);
        let mut password = PasswordForm::new();
        for _ in 0..20 {
            password.push_char('c');
        }
        assert_eq!(password.password.chars().count(), 12);
        password.pop();
        assert_eq!(password.password.chars().count(), 11);
    }

    #[test]
    fn side_sprites_resolve_in_block_order() {
        let list = o2jam_file::parse_control_list(&side_control_text()).unwrap();
        assert_eq!(
            side_sprite_names(&list, SideScreen::Shop),
            vec!["Shop_Item.ojs".to_owned(), "Shop_ItemGem.ojs".to_owned()]
        );
        assert!(side_sprite_names(&list, SideScreen::MyRoom).is_empty());
    }

    #[test]
    fn payment_merges_both_billing_blocks() {
        let list = o2jam_file::parse_control_list(&side_control_text()).unwrap();
        assert_eq!(
            side_sprite_names(&list, SideScreen::Payment),
            vec!["Pay_Bill.ojs".to_owned(), "Pay_Gift.ojs".to_owned()]
        );
    }

    #[test]
    fn tutorial_resolves_to_its_backdrop() {
        let list = o2jam_file::parse_control_list(&side_control_text()).unwrap();
        assert_eq!(
            side_sprite_names(&list, SideScreen::Tutorial),
            vec![TUTORIAL_BACKGROUND_SPRITE.to_owned()]
        );
    }

    #[test]
    fn side_figures_load_through_archive_wiring() {
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = archive_blob(&[("Shop_Item.ojs", &red)]);
        let figures = load_side_figures(
            &archive,
            &["Shop_Item.ojs".to_owned(), "Missing.ojs".to_owned()],
        );
        assert_eq!(figures.len(), 1);
        assert!(figures.contains_key("Shop_Item.ojs"));
        assert!(load_side_figures(b"garbage", &["Shop_Item.ojs".to_owned()]).is_empty());
    }

    #[test]
    fn side_scene_paints_loaded_figures() {
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = archive_blob(&[("Shop_Item.ojs", &red)]);
        let figures = load_side_figures(&archive, &["Shop_Item.ojs".to_owned()]);
        let list = o2jam_file::parse_control_list(&side_control_text()).unwrap();
        let block = scene_block(&list, SHOP_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(0, 0, 800, 600), (0, 0, 800, 600)])).unwrap();
        let placed = bind_side_scene(block, &bnd);
        assert_eq!(placed.len(), 2);
        let mut buf = vec![0u32; 8 * 8];
        paint_side_scene(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            &placed,
            &figures,
        );
        assert_eq!(buf[0], o2jam_render::pack_xrgb(255, 0, 0));
    }

    fn side_item_row(item_id: u32, name: &str, price: u32) -> o2jam_file::AvatarItemRow {
        o2jam_file::AvatarItemRow {
            item_id,
            item_type: 0,
            price,
            params: [0; 4],
            flags: (0, 0),
            name: name.to_owned(),
            description: String::new(),
            sprites: Vec::new(),
        }
    }

    fn side_item_table() -> o2jam_file::AvatarItemTable {
        o2jam_file::AvatarItemTable {
            rows: vec![
                side_item_row(1, "Hair", 1000),
                side_item_row(2, "Boots", 2500),
            ],
        }
    }

    fn myroom_twelve_item_table() -> o2jam_file::AvatarItemTable {
        o2jam_file::AvatarItemTable {
            rows: (1..=12)
                .map(|id| side_item_row(id, &format!("Item{id}"), id * 100))
                .collect(),
        }
    }

    fn caption_text(captions: &[(u32, String)], code: u32) -> Option<&str> {
        captions
            .iter()
            .find(|(at, _)| *at == code)
            .map(|(_, text)| text.as_str())
    }

    fn side_song_record(song_id: u32, bpm: f32, title: &str, ojm_file: &str) -> Vec<u8> {
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&song_id.to_le_bytes());
        rec[20..24].copy_from_slice(&bpm.to_le_bytes());
        rec[112..112 + title.len()].copy_from_slice(title.as_bytes());
        rec[176..180].copy_from_slice(b"Band");
        rec[240..240 + ojm_file.len()].copy_from_slice(ojm_file.as_bytes());
        rec
    }

    fn side_catalog() -> Vec<u8> {
        let mut blob = side_song_record(757, 124.0, "First", "o2ma757.ojm");
        // Zeroed slot with magic: dead, filtered out everywhere.
        let mut dead = vec![0u8; 300];
        dead[8..11].copy_from_slice(b"ojn");
        blob.extend_from_slice(&dead);
        blob.extend_from_slice(&side_song_record(770, 168.0, "Second", "o2ma770.ojm"));
        blob
    }

    #[test]
    fn shop_lists_every_item_in_table_order() {
        let table = side_item_table();
        let items = shop_items(&table);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].item_id, 1);
        assert_eq!(items[0].name, "Hair");
        assert_eq!(items[0].price, 1000);
        assert_eq!(items[1].item_id, 2);
        assert_eq!(items[1].name, "Boots");
        let empty = o2jam_file::AvatarItemTable { rows: Vec::new() };
        assert!(shop_items(&empty).is_empty());
    }

    #[test]
    fn music_shop_lists_only_real_songs() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let songs = music_shop_songs(&entries);
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].song_id, 757);
        assert_eq!(songs[0].title, "First");
        assert_eq!(songs[0].artist, "Band");
        assert_eq!(songs[0].bpm, 124.0);
        assert_eq!(songs[1].song_id, 770);
        assert!(music_shop_songs(&[]).is_empty());
    }

    #[test]
    fn my_room_resolves_equipment_skipping_gaps() {
        let table = side_item_table();
        let worn = my_room_equipment(&table, &[2, 0, 999, 1]);
        assert_eq!(worn.len(), 2);
        assert_eq!(worn[0].item_id, 2);
        assert_eq!(worn[1].item_id, 1);
        assert!(my_room_equipment(&table, &[]).is_empty());
        assert!(my_room_equipment(&table, &[0, -1, 999]).is_empty());
    }

    fn side_character(equipment: [i32; 17]) -> CharacterInfo {
        CharacterInfo {
            equipment,
            ..CharacterInfo::default()
        }
    }

    fn side_equipment(first: &[i32]) -> [i32; 17] {
        let mut equipment = [0i32; 17];
        for (slot, id) in first.iter().enumerate().take(17) {
            equipment[slot] = *id;
        }
        equipment
    }

    #[test]
    fn my_room_reads_fetched_character_equipment() {
        let table = side_item_table();
        let character = side_character(side_equipment(&[2, 0, 999, 1]));
        let worn = my_room_equipment_from_character(&table, &character);
        assert_eq!(worn.len(), 2);
        assert_eq!(worn[0].item_id, 2);
        assert_eq!(worn[1].item_id, 1);
    }

    #[test]
    fn my_room_character_without_gear_wears_nothing() {
        let table = side_item_table();
        assert!(my_room_equipment_from_character(&table, &side_character([0; 17])).is_empty());
        assert!(
            my_room_equipment_from_character(&table, &side_character(side_equipment(&[999])))
                .is_empty()
        );
    }

    #[test]
    fn myroom_slot_codes_cover_eleven_buttons_in_order() {
        // The shipped `STATE_MYROOM` block carries 11 art-less slot
        // selectors (`0X15200005..0F`): each code picks its button
        // ordinal, and the next code is no slot at all.
        for (index, name) in MYROOM_SLOT_NAMES.iter().enumerate() {
            let slot = u8::try_from(index).expect("eleven slots fit");
            let code = MYROOM_SLOT_FIRST_CODE + u32::try_from(index).expect("small index");
            assert_eq!(
                myroom_action_for_code(code),
                Some(MyRoomAction::SelectSlot(slot)),
                "slot button {name}"
            );
        }
        assert_ne!(
            myroom_action_for_code(MYROOM_SLOT_FIRST_CODE + MYROOM_SLOT_BUTTONS_U32),
            Some(MyRoomAction::SelectSlot(0))
        );
    }

    #[test]
    fn myroom_dispatch_names_bag_gift_and_chrome_codes() {
        assert_eq!(
            myroom_action_for_code(MYROOM_BAG_LEFT_CODE),
            Some(MyRoomAction::BagPage(MyRoomStep::Previous))
        );
        assert_eq!(
            myroom_action_for_code(MYROOM_BAG_RIGHT_CODE),
            Some(MyRoomAction::BagPage(MyRoomStep::Next))
        );
        assert_eq!(
            myroom_action_for_code(MYROOM_GIFT_LEFT_CODE),
            Some(MyRoomAction::GiftPage(MyRoomStep::Previous))
        );
        assert_eq!(
            myroom_action_for_code(MYROOM_GIFT_RIGHT_CODE),
            Some(MyRoomAction::GiftPage(MyRoomStep::Next))
        );
        assert_eq!(
            myroom_action_for_code(MYROOM_APPLY_CODE),
            Some(MyRoomAction::ApplyEquip)
        );
        assert_eq!(
            myroom_action_for_code(MYROOM_VACATE_CODE),
            Some(MyRoomAction::VacateSlot)
        );
        assert_eq!(
            myroom_action_for_code(MYROOM_BACK_CODE),
            Some(MyRoomAction::BackToLobby)
        );
        assert_eq!(
            myroom_action_for_code(MYROOM_GOSHOP_CODE),
            Some(MyRoomAction::OpenShop)
        );
        // Caption bodies never dispatch: text controls are not buttons.
        assert_eq!(myroom_action_for_code(MYROOM_NICKNAME_CODE), None);
    }

    fn myroom_hit_placement(code: u32, x: i32, y: i32) -> ScenePlacement {
        ScenePlacement {
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w: 40, h: 20 },
        }
    }

    #[test]
    fn myroom_action_hit_rides_staged_rects() {
        // Hit-testing reads staged `.bnd` placements only (owner rule
        // `f38f3`): no fallback literals, unstaged clicks land nowhere.
        let placements = vec![
            myroom_hit_placement(MYROOM_SLOT_FIRST_CODE + 2, 10, 10),
            myroom_hit_placement(MYROOM_BAG_FIRST_CODE + 4, 100, 10),
            myroom_hit_placement(MYROOM_GIFT_FIRST_CODE + 1, 200, 10),
            myroom_hit_placement(MYROOM_BAG_RIGHT_CODE, 300, 10),
        ];
        assert_eq!(
            myroom_action_at(&placements, 11, 11),
            Some(MyRoomAction::SelectSlot(2))
        );
        assert_eq!(
            myroom_action_at(&placements, 101, 11),
            Some(MyRoomAction::PickBagCell(0))
        );
        assert_eq!(
            myroom_action_at(&placements, 201, 11),
            Some(MyRoomAction::PickGiftCell(0))
        );
        assert_eq!(
            myroom_action_at(&placements, 301, 11),
            Some(MyRoomAction::BagPage(MyRoomStep::Next))
        );
        assert_eq!(myroom_action_at(&placements, 0, 0), None);
        assert_eq!(myroom_action_at(&[], 11, 11), None);
    }

    #[test]
    fn myroom_equip_slot_follows_the_exe_derivation() {
        // `docs/equip.md`: type 5 forces slot 11, otherwise the
        // record slot byte holds, range-checked below 17.
        assert_eq!(myroom_equip_slot(1, 3), Some(3));
        assert_eq!(myroom_equip_slot(5, 3), Some(11));
        assert_eq!(myroom_equip_slot(5, 0), Some(11));
        assert_eq!(myroom_equip_slot(1, 17), None);
        assert_eq!(myroom_equip_slot(1, 255), None);
    }

    #[test]
    fn myroom_record_slot_reads_file_byte_0x18() {
        // The slot byte is file byte +0x18: the top byte of the
        // third param word (bytes 21..24 of the 30-byte prefix).
        let mut row = side_item_row(1, "Hair", 1000);
        row.params = [0, 0, 0x0300_0000, 0];
        assert_eq!(myroom_record_slot(&row), 3);
    }

    #[test]
    fn myroom_selection_starts_cleared_and_clamps() {
        let mut state = MyRoomState::new();
        assert_eq!(state.selected_slot(), 0);
        assert_eq!(state.bag_page(), 0);
        assert_eq!(state.bag_cell(), None);
        state.select_slot(3);
        assert_eq!(state.selected_slot(), 3);
        // Past the eleventh button the press lands nowhere.
        state.select_slot(11);
        assert_eq!(state.selected_slot(), 3);
        // Page turns clamp to the staged page count.
        state.turn_bag_page(MyRoomStep::Next, 3);
        assert_eq!(state.bag_page(), 1);
        state.turn_bag_page(MyRoomStep::Previous, 3);
        assert_eq!(state.bag_page(), 0);
        state.turn_bag_page(MyRoomStep::Previous, 3);
        assert_eq!(state.bag_page(), 0);
        state.pick_bag_cell(Some(4));
        assert_eq!(state.bag_cell(), Some(4));
        state.turn_bag_page(MyRoomStep::Next, 3);
        assert_eq!(state.bag_cell(), None);
    }

    #[test]
    fn myroom_bag_pages_hold_ten_owned_rows() {
        let table = myroom_twelve_item_table();
        let inventory: Vec<i32> = (1..=12).collect();
        assert_eq!(myroom_bag_pages(&inventory, &table), 2);
        let first = myroom_bag_rows(&table, &inventory, 0);
        assert_eq!(first.len(), 10);
        assert_eq!(first[0].item_id, 1);
        let second = myroom_bag_rows(&table, &inventory, 1);
        assert_eq!(second.len(), 2);
        assert_eq!(second[0].item_id, 11);
        // Zero and unknown ids never occupy a bag cell.
        let sparse = myroom_bag_rows(&table, &[0, 999, 1], 0);
        assert_eq!(sparse.len(), 1);
        assert_eq!(sparse[0].item_id, 1);
        assert!(myroom_bag_rows(&table, &[], 0).is_empty());
        assert!(myroom_bag_rows(&table, &inventory, 7).is_empty());
    }

    #[test]
    fn myroom_gift_views_resolve_names_and_senders() {
        let table = side_item_table();
        let songs = vec![SelectMusicSong {
            song_id: 757,
            title: "First".to_owned(),
            artist: "Band".to_owned(),
        }];
        let mut character = CharacterInfo::default();
        character.item_gifts.push(GiftItemInfo {
            gift_id: 7,
            item_id: 1,
            sender: "Giver".to_owned(),
        });
        character.music_gifts.push(GiftMusicInfo {
            gift_id: 9,
            music_id: 757,
            sender: "DJ".to_owned(),
        });
        let views = myroom_gift_views(&table, &songs, &character);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].name, "Hair");
        assert_eq!(views[0].sender, "Giver");
        assert!(!views[0].is_music);
        assert_eq!(views[1].name, "First");
        assert_eq!(views[1].sender, "DJ");
        assert!(views[1].is_music);
        assert_eq!(myroom_gift_pages(&views), 1);
    }

    #[test]
    fn myroom_identity_captions_carry_character_fields() {
        let character = CharacterInfo {
            nickname: "Tester".to_owned(),
            level: 12,
            experience: 3450,
            o2_cash: 500,
            ..CharacterInfo::default()
        };
        let captions = myroom_identity_captions(&character);
        assert_eq!(
            caption_text(&captions, MYROOM_NICKNAME_CODE),
            Some("Tester")
        );
        assert_eq!(caption_text(&captions, MYROOM_LEVEL_CODE), Some("12"));
        assert_eq!(caption_text(&captions, MYROOM_EXP_CODE), Some("3450"));
        assert_eq!(caption_text(&captions, MYROOM_O2CASH_CODE), Some("500"));
    }

    fn side_music_answer(free_music: bool) -> MusicListAnswer {
        use o2jam_net::play::MusicListEntry;
        MusicListAnswer {
            entries: vec![MusicListEntry {
                id: 757,
                notes_ex: 100,
                notes_nx: 200,
                notes_hx: 300,
            }],
            free_music,
        }
    }

    #[test]
    fn music_shop_listings_carry_live_free_flags() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let listings = music_shop_listings(&entries, Some(&side_music_answer(true)));
        assert_eq!(listings.len(), 2);
        assert_eq!(listings[0].song.song_id, 757);
        assert!(listings[0].free);
        assert!(listings[0].listed);
        assert!(listings[1].free);
        assert!(!listings[1].listed);
    }

    #[test]
    fn music_shop_listings_clear_flags_without_answer() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let listings = music_shop_listings(&entries, None);
        assert_eq!(listings.len(), 2);
        assert!(!listings[0].free);
        assert!(!listings[0].listed);
        let paid = music_shop_listings(&entries, Some(&side_music_answer(false)));
        assert!(!paid[0].free);
        assert!(paid[0].listed);
    }

    #[test]
    fn music_shop_listings_stay_empty_without_catalog() {
        assert!(music_shop_listings(&[], Some(&side_music_answer(true))).is_empty());
        assert!(music_shop_listings(&[], None).is_empty());
    }

    fn select_music_control_text() -> String {
        [
            "NUMBER_OF_DIALOG 0x01",
            "DAILOG_SELECTMUSIC 0x02",
            "{",
            "BOUND \"Dialog_SelectMusic.bnd\"",
            "IDC_SEL 0X00100000",
            "IDC_ALBUM 0X10500001 \"AlbumList.ojs\"",
            "IDC_LOAD 0X10500002 \"MusicLoading.ojs\"",
            "}",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn select_music_dialog_names_entries_in_block_order() {
        let list = o2jam_file::parse_control_list(&select_music_control_text()).unwrap();
        assert_eq!(SELECT_MUSIC_DIALOG_BLOCK, "DAILOG_SELECTMUSIC");
        assert_eq!(SELECT_MUSIC_BIND_FILE, "Dialog_SelectMusic.bnd");
        assert_eq!(
            select_music_sprite_names(&list),
            vec!["AlbumList.ojs".to_owned(), "MusicLoading.ojs".to_owned()]
        );
    }

    #[test]
    fn select_music_dialog_stays_empty_without_block() {
        let list = o2jam_file::parse_control_list(&side_control_text()).unwrap();
        assert!(select_music_sprite_names(&list).is_empty());
    }

    #[test]
    fn select_filters_name_three_columns() {
        assert_eq!(SELECT_LEVEL_FILTERS, ["ALL", "EASY", "MID", "HIGH", "TOP"]);
        assert_eq!(
            SELECT_WHEEL_FILTERS,
            ["ALL", "GOLD", "BLACK", "BLUE", "RED", "SILVER"]
        );
        assert_eq!(
            SELECT_GENRE_FILTERS,
            [
                "ALL",
                "BALLAD",
                "ROCK",
                "DANCE",
                "HIPHOP/R&B",
                "JAZZ/FUNK",
                "CLASSIC",
                "ETC"
            ]
        );
    }

    #[test]
    fn select_table_headers_name_nine_columns() {
        assert_eq!(
            SELECT_TABLE_HEADERS,
            ["LABEL", "NOTE", "TITLE", "ARTIST", "LVL", "DL", "PLAY", "INFO", "GRADE"]
        );
    }

    #[test]
    fn select_songs_walk_catalog_skipping_dead_slots() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let songs = select_music_songs(&entries);
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].song_id, 757);
        assert_eq!(songs[0].title, "First");
        assert_eq!(songs[0].artist, "Band");
        assert_eq!(songs[1].song_id, 770);
        assert_eq!(songs[1].artist, "Band");
        assert!(select_music_songs(&[]).is_empty());
    }

    #[test]
    fn select_rows_repeat_each_song_per_difficulty_slot() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let songs = select_music_songs(&entries);
        let rows = select_music_rows(&songs);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].song.song_id, 757);
        assert_eq!(rows[0].slot, 0);
        assert_eq!(rows[0].letter(), "E");
        assert_eq!(rows[2].slot, 2);
        assert_eq!(rows[2].letter(), "H");
        assert_eq!(rows[3].song.song_id, 770);
        assert_eq!(rows[3].slot, 0);
        assert_eq!(rows[5].slot, 2);
        assert!(select_music_rows(&[]).is_empty());
    }

    #[test]
    fn select_row_hit_maps_the_search_table_bands() {
        // Row bands run at the audited pitch inside the table band;
        // tops inclusive, past the staged rows never.
        assert_eq!(select_row_hit(400, 150, 6), Some(0));
        assert_eq!(select_row_hit(400, 172, 6), Some(0));
        assert_eq!(select_row_hit(400, 173, 6), Some(1));
        assert_eq!(select_row_hit(780, 196, 6), Some(2));
        assert_eq!(select_row_hit(781, 160, 6), None);
        assert_eq!(select_row_hit(343, 160, 6), None);
        assert_eq!(select_row_hit(400, 149, 6), None);
        assert_eq!(select_row_hit(400, 160, 1), Some(0));
        assert_eq!(select_row_hit(400, 173, 1), None);
        assert_eq!(select_row_hit(400, 160, 0), None);
        assert_eq!(select_row_hit(400, 379, 30), Some(9));
        assert_eq!(select_row_hit(400, 380, 30), None);
    }

    #[test]
    fn select_rows_leave_levels_and_grades_undrawn() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let songs = select_music_songs(&entries);
        let rows = select_music_rows(&songs);
        assert!(rows.iter().all(|row| row.level().is_none()));
        assert!(rows.iter().all(|row| row.grade().is_none()));
    }

    #[test]
    fn select_captions_count_rows_and_pages() {
        assert_eq!(select_search_caption(1608), "SEARCH RESULTS (1608)");
        assert_eq!(select_picked_caption(), "SELECTED (0)");
        assert_eq!(select_pager_caption(1608), "1/161");
        assert_eq!(select_pager_caption(0), "1/1");
        assert_eq!(select_pager_caption(10), "1/1");
        assert_eq!(select_pager_caption(11), "1/2");
    }

    #[test]
    fn select_status_line_names_the_all_filter_path() {
        assert_eq!(select_status_line(), "ALL -> ALL SEARCH RESULT.");
    }

    /// Synthetic INDEX-button placements in the audited file shape:
    /// `counts` cells per row (top row first) at `pitch` x-step,
    /// codes carrying ordinals 87.. like the genuine bind pairing.
    fn index_button_placements(counts: &[usize], pitch: i32) -> Vec<ScenePlacement> {
        let mut placements = Vec::new();
        let mut ordinal = 87usize;
        for (row, count) in counts.iter().enumerate() {
            for cell in 0..*count {
                let code = 0x8C00_0000u32 | (ordinal as u32 + 1);
                placements.push(ScenePlacement {
                    id: String::new(),
                    code,
                    sprite: None,
                    rect: Rect {
                        x: 345 + pitch * cell as i32,
                        y: 440 + 15 * row as i32,
                        w: 14,
                        h: 14,
                    },
                });
                ordinal += 1;
            }
        }
        placements
    }

    #[test]
    fn select_chosung_layout_counts_the_index_button_inventory() {
        // FILE per the audit: the grid derives from the INDEX-button
        // inventory (27 cells over 15 below) and the x-pitch is the
        // exact 15 px file step.
        let layout = select_chosung_layout(&index_button_placements(&[27, 15], 15))
            .expect("INDEX buttons stage a layout");
        assert_eq!(layout.row_cells, vec![27, 15]);
        assert_eq!(layout.pitch, 15);
    }

    #[test]
    fn select_chosung_layout_uses_the_smallest_row_pitch() {
        // A divergent pitch must reach the painter: letters land by
        // the staged file step, not the constant.
        let layout = select_chosung_layout(&index_button_placements(&[27, 15], 20))
            .expect("INDEX buttons stage a layout");
        assert_eq!(layout.row_cells, vec![27, 15]);
        assert_eq!(layout.pitch, 20);
    }

    #[test]
    fn select_chosung_layout_falls_back_without_index_buttons() {
        assert_eq!(select_chosung_layout(&[]), None);
        let unrelated = vec![ScenePlacement {
            id: String::new(),
            code: 0x8C20_0002,
            sprite: None,
            rect: Rect {
                x: 1,
                y: 2,
                w: 3,
                h: 4,
            },
        }];
        assert_eq!(select_chosung_layout(&unrelated), None);
    }

    #[test]
    fn select_chosung_cells_span_jamo_letters_and_words() {
        let cells = select_chosung_cells();
        assert_eq!(cells.len(), 42);
        let jamo: Vec<char> = cells[..SELECT_CHO_SUNG_JAMO]
            .iter()
            .filter_map(|cell| *cell)
            .collect();
        assert_eq!(
            jamo,
            [
                'ㄱ', 'ㄴ', 'ㄷ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅅ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ'
            ],
            "the 14 simple initial jamo must fill the sort grid"
        );
        let letters: Vec<char> = cells.iter().filter_map(|cell| *cell).skip(14).collect();
        assert_eq!(letters, ('A'..='Z').collect::<Vec<_>>());
        assert_eq!(cells[40], None);
        assert_eq!(cells[41], None);
    }

    #[test]
    fn select_inks_follow_the_reference_letters() {
        assert_eq!(SELECT_DIFFICULTY_LETTERS, ["E", "N", "H"]);
        assert_eq!(select_difficulty_ink(0), pack_xrgb(0x30, 0xE0, 0x30));
        assert_eq!(select_difficulty_ink(1), pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(select_difficulty_ink(2), pack_xrgb(0xFF, 0x50, 0x50));
    }

    #[test]
    fn select_layout_pins_reference_anchors() {
        // Native 800x600 measurements of reference-musicselect-4x3.png:
        // three filter columns, ten 23 px rows under a 127-top header,
        // and the bottom-center OK pill.
        assert_eq!(
            SELECT_FILTER_COLUMNS,
            [
                Rect {
                    x: 28,
                    y: 95,
                    w: 96,
                    h: 300
                },
                Rect {
                    x: 130,
                    y: 95,
                    w: 92,
                    h: 300
                },
                Rect {
                    x: 230,
                    y: 95,
                    w: 100,
                    h: 300
                }
            ]
        );
        assert_eq!(SELECT_FILTER_BUTTON_TOP, 129);
        assert_eq!(SELECT_FILTER_BUTTON_PITCH, 33);
        assert_eq!(SELECT_ROW_TOP, 150);
        assert_eq!(SELECT_ROW_PITCH, 23);
        assert_eq!(SELECT_VISIBLE_ROWS, 10);
        assert_eq!(SELECT_HEADER_COLUMNS[3], 417);
        assert_eq!(SELECT_CHO_SUNG_LEFT, 352);
        assert_eq!(SELECT_CHO_SUNG_PITCH, 15);
        assert_eq!(
            SELECT_OK_BUTTON,
            Rect {
                x: 342,
                y: 506,
                w: 113,
                h: 40
            }
        );
    }

    #[test]
    fn waiting_song_title_resolves_only_real_songs() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        assert_eq!(waiting_song_title(&entries, 757), Some("First"));
        assert_eq!(waiting_song_title(&entries, 770), Some("Second"));
        assert_eq!(waiting_song_title(&entries, 999), None);
        assert_eq!(waiting_song_title(&[], 757), None);
    }

    #[test]
    fn waiting_song_labels_name_difficulty_and_speed() {
        assert_eq!(waiting_song_difficulty_label(0), "EX");
        assert_eq!(waiting_song_difficulty_label(1), "NX");
        assert_eq!(waiting_song_difficulty_label(2), "HX");
        assert_eq!(waiting_song_difficulty_label(3), "RX");
        assert_eq!(waiting_song_difficulty_label(9), "DIF 9");
        assert_eq!(waiting_song_speed_label(1), "X1");
        assert_eq!(waiting_song_speed_label(3), "X2");
        assert_eq!(waiting_song_speed_label(11), "XR");
        assert_eq!(waiting_song_speed_label(12), "SPD 12");
    }

    #[test]
    fn waiting_song_lines_carry_id_title_and_choices() {
        use o2jam_net::play::SetRoomMusic;
        let selection = SetRoomMusic {
            music_id: 757,
            difficulty: 2,
            speed: 3,
        };
        assert_eq!(waiting_song_title_line(757, "First"), "M757 First");
        assert_eq!(waiting_song_detail_line(&selection), "HX X2");
    }

    fn side_rank_update(ranks: [u8; 8]) -> GameStatsUpdate {
        GameStatsUpdate {
            member_id: 0,
            kind_code: 0,
            value: 0,
            ranks,
        }
    }

    #[test]
    fn lanking_live_rows_follow_broadcast_order_skipping_seats() {
        let rows = live_rank_rows(Some(&side_rank_update([1, 0, 3, 0, 0, 0, 0, 2])));
        assert_eq!(
            rows,
            vec![
                LiveRankRow { slot: 0, rank: 1 },
                LiveRankRow { slot: 2, rank: 3 },
                LiveRankRow { slot: 7, rank: 2 },
            ]
        );
    }

    #[test]
    fn lanking_live_rows_stay_empty_without_broadcast() {
        assert!(live_rank_rows(None).is_empty());
        assert!(live_rank_rows(Some(&side_rank_update([0; 8]))).is_empty());
    }

    #[test]
    fn planet_bands_cover_every_level_slot() {
        assert_eq!(PlanetBand::of_level(0), Some(PlanetBand::Easy));
        assert_eq!(PlanetBand::of_level(1), Some(PlanetBand::Normal));
        assert_eq!(PlanetBand::of_level(2), Some(PlanetBand::Hard));
        assert_eq!(PlanetBand::of_level(3), None);
        for band in PLANET_BANDS {
            assert_eq!(PlanetBand::of_level(band.level()), Some(band));
        }
        assert_eq!(
            [
                PlanetBand::Easy.name(),
                PlanetBand::Normal.name(),
                PlanetBand::Hard.name()
            ],
            ["EASY", "NORMAL", "HARD"]
        );
        assert_eq!(PLANET_FREE_LABEL, "FREE");
    }

    #[test]
    fn planet_missions_pair_every_song_with_every_band() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let missions = planet_missions(&entries);
        assert_eq!(missions.len(), 6);
        assert_eq!(missions[0].band, PlanetBand::Easy);
        assert_eq!(missions[0].song_id, 757);
        assert_eq!(missions[2].band, PlanetBand::Hard);
        assert_eq!(missions[3].song_id, 770);
        assert_eq!(missions[5].band, PlanetBand::Hard);
        assert!(planet_missions(&[]).is_empty());
    }

    #[test]
    fn planet_missions_for_band_lists_only_the_picked_band() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let hard = planet_missions_for_band(&entries, PlanetBand::Hard);
        assert_eq!(hard.len(), 2);
        assert!(hard.iter().all(|mission| mission.band == PlanetBand::Hard));
        assert_eq!(hard[0].song_id, 757);
        assert_eq!(hard[1].title, "Second");
        assert!(planet_missions_for_band(&[], PlanetBand::Easy).is_empty());
    }

    #[test]
    fn planet_song_count_skips_dead_slots() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        assert_eq!(planet_song_count(&entries), 2);
        assert_eq!(planet_song_count(&[]), 0);
    }

    #[test]
    fn planet_band_census_counts_every_song_per_band() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let census = planet_band_census(&entries);
        assert_eq!(
            census.map(|entry| entry.band),
            [PlanetBand::Easy, PlanetBand::Normal, PlanetBand::Hard]
        );
        assert!(census.iter().all(|entry| entry.song_count == 2));
        assert!(planet_band_census(&[])
            .iter()
            .all(|entry| entry.song_count == 0));
    }

    #[test]
    fn planet_captions_carry_band_and_song_count() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let census = planet_band_census(&entries);
        assert_eq!(planet_band_caption(&census[0]), "EASY (2)");
        assert_eq!(planet_band_caption(&census[2]), "HARD (2)");
        assert_eq!(planet_free_caption(planet_song_count(&entries)), "FREE (2)");
        assert_eq!(planet_free_caption(0), "FREE (0)");
    }

    #[test]
    fn lanking_positions_follow_catalog_order() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let rows = lanking_rows(&entries);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].position, 1);
        assert_eq!(rows[0].song_id, 757);
        assert_eq!(rows[0].title, "First");
        assert_eq!(rows[1].position, 2);
        assert_eq!(rows[1].song_id, 770);
        assert!(lanking_rows(&[]).is_empty());
    }

    #[test]
    fn lanking_row_for_song_resolves_catalog_position() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let row = lanking_row_for_song(&entries, 770).unwrap();
        assert_eq!(row.position, 2);
        assert_eq!(row.title, "Second");
        assert_eq!(lanking_row_for_song(&entries, 757).unwrap().position, 1);
    }

    #[test]
    fn lanking_row_for_song_misses_dead_slots_and_unknown_ids() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        assert_eq!(lanking_row_for_song(&entries, 999), None);
        assert_eq!(lanking_row_for_song(&[], 757), None);
    }

    #[test]
    fn lanking_caption_counts_rows() {
        assert_eq!(lanking_caption(2), "RANKING (2)");
        assert_eq!(lanking_caption(0), "RANKING (0)");
    }

    fn lanking_button(code: u32, x: i32, y: i32) -> ScenePlacement {
        ScenePlacement {
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w: 40, h: 16 },
        }
    }

    #[test]
    fn lanking_clicks_answer_the_exe_named_sort_and_pager_codes() {
        // Every named control answers at its exe code; the tier
        // button (unnamed for this ticket) lands nowhere.
        let placements = vec![
            lanking_button(LANKING_SORT_TITLE_CODE, 10, 10),
            lanking_button(LANKING_SORT_DIFF_CODE, 60, 10),
            lanking_button(LANKING_SORT_BPM_CODE, 110, 10),
            lanking_button(LANKING_SORT_ARTIST_CODE, 160, 10),
            lanking_button(LANKING_LPAGE_PREV_CODE, 10, 40),
            lanking_button(LANKING_LPAGE_NEXT_CODE, 60, 40),
            lanking_button(LANKING_RPAGE_PREV_CODE, 110, 40),
            lanking_button(LANKING_RPAGE_NEXT_CODE, 160, 40),
            lanking_button(0x5220_0009, 210, 10),
        ];
        assert_eq!(
            lanking_action_at(&placements, 15, 15),
            Some(LankingAction::Sort(LankingSort::Title))
        );
        assert_eq!(
            lanking_action_at(&placements, 65, 15),
            Some(LankingAction::Sort(LankingSort::Diff))
        );
        assert_eq!(
            lanking_action_at(&placements, 115, 15),
            Some(LankingAction::Sort(LankingSort::Bpm))
        );
        assert_eq!(
            lanking_action_at(&placements, 165, 15),
            Some(LankingAction::Sort(LankingSort::Artist))
        );
        assert_eq!(
            lanking_action_at(&placements, 15, 45),
            Some(LankingAction::LViewPrev)
        );
        assert_eq!(
            lanking_action_at(&placements, 65, 45),
            Some(LankingAction::LViewNext)
        );
        assert_eq!(
            lanking_action_at(&placements, 115, 45),
            Some(LankingAction::RViewPrev)
        );
        assert_eq!(
            lanking_action_at(&placements, 165, 45),
            Some(LankingAction::RViewNext)
        );
        assert_eq!(lanking_action_at(&placements, 215, 15), None);
        assert_eq!(lanking_action_at(&placements, 500, 500), None);
    }

    #[test]
    fn lanking_row_picks_answer_the_left_hit_areas() {
        // Five staged `LVIEW_EX` hit areas name the five visible
        // slots; the shared LANK template tiles over them; the
        // `RVIEW_EX` toggles stay display-only (no consumer pins
        // them) and land nowhere.
        let mut placements: Vec<ScenePlacement> = (0i32..5)
            .map(|slot| {
                lanking_button(
                    LANKING_LVIEW_TOGGLE_FIRST_CODE + slot as u32,
                    700,
                    100 + slot * 84,
                )
            })
            .collect();
        placements.push(lanking_button(LANKING_ROW_LANK_CODE, 600, 100));
        placements.push(lanking_button(LANKING_RVIEW_TOGGLE_FIRST_CODE, 500, 400));
        assert_eq!(
            lanking_lview_slots(&placements).len(),
            5,
            "five staged hit areas"
        );
        assert_eq!(
            lanking_action_at(&placements, 705, 105),
            Some(LankingAction::SelectRow(0))
        );
        assert_eq!(
            lanking_action_at(&placements, 705, 440),
            Some(LankingAction::SelectRow(4))
        );
        assert_eq!(lanking_action_at(&placements, 705, 600), None);
        // The shared LANK button resolves its slot through the
        // staged left slots.
        assert_eq!(
            lanking_action_at(&placements, 605, 105),
            Some(LankingAction::SelectRow(0))
        );
        assert_eq!(
            lanking_action_at(&placements, 605, 275),
            Some(LankingAction::SelectRow(2))
        );
        // The right-row toggle lands nowhere (display-only OPEN).
        assert_eq!(lanking_action_at(&placements, 505, 405), None);
        // No LANK template staged: its clicks land nowhere, while
        // the hit areas still answer.
        let bare: Vec<ScenePlacement> = placements[..5].to_vec();
        assert_eq!(
            lanking_action_at(&bare, 605, 105),
            None,
            "unstaged LANK template answers nothing"
        );
        assert_eq!(
            lanking_action_at(&bare, 705, 105),
            Some(LankingAction::SelectRow(0))
        );
    }

    #[test]
    fn lanking_songs_read_catalog_fields_and_sort_keys() {
        let blob = side_catalog();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        let mut songs = lanking_songs(&entries);
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title, "First");
        assert_eq!(songs[0].artist, "Band");
        assert_eq!(songs[0].bpm, 124.0);
        sort_lanking_songs(&mut songs, LankingSort::Bpm);
        assert_eq!(songs[0].song_id, 757);
        sort_lanking_songs(&mut songs, LankingSort::Title);
        assert_eq!(songs[0].title, "First");
        assert_eq!(songs[1].title, "Second");
        let (start, end) = lanking_page_window(songs.len(), LANKING_LVIEW_ROWS, 0);
        assert_eq!((start, end), (0, 2));
        assert_eq!(
            lanking_page_window(songs.len(), LANKING_LVIEW_ROWS, 1),
            (2, 2)
        );
    }

    #[test]
    fn lanking_rank_cells_render_fetched_lines_and_blank_fill() {
        let entry = RankEntry {
            rank: 3,
            nickname: "Rival".to_string(),
            battles: 40,
            wins: 25,
            win_rate: 62,
            delta: RankDelta::Down,
            rank_delta: 2,
        };
        assert_eq!(
            lanking_rank_cells(&entry),
            [
                "3".to_string(),
                "Rival".to_string(),
                "40".to_string(),
                "25".to_string(),
                "62".to_string(),
                "DOWN".to_string(),
            ]
        );
        // The exe's zeroed display-fill row paints no cells.
        assert!(lanking_rank_cells(&RankEntry::default())
            .iter()
            .all(String::is_empty));
    }

    #[test]
    fn payment_labels_name_both_billing_views() {
        let labels = payment_labels();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].block, PAYMENT_BLOCK);
        assert_eq!(labels[0].page, PAYMENT_PAGE);
        assert_eq!(labels[0].page, "payment_input.aspx");
        assert_eq!(labels[1].block, PAYMENT2_BLOCK);
        assert_eq!(labels[1].page, GIFT_PAGE);
        assert_eq!(labels[1].page, "gift_input.aspx");
    }

    #[test]
    fn payment_url_paths_name_lifecycle_verified_billing_views() {
        let labels = payment_labels();
        assert_eq!(labels[0].url_path(), PAYMENT_URL_PATH);
        assert_eq!(
            labels[0].url_path(),
            "np/billing/payment/payment_input.aspx"
        );
        assert_eq!(labels[1].url_path(), GIFT_URL_PATH);
        assert_eq!(labels[1].url_path(), "billing/gift/gift_input.aspx");
    }

    #[test]
    fn payment_view_captions_pair_block_with_path() {
        let labels = payment_labels();
        assert_eq!(
            payment_view_caption(&labels[0]),
            "STATE_PAYMENT np/billing/payment/payment_input.aspx"
        );
        assert_eq!(
            payment_view_caption(&labels[1]),
            "STATE_PAYMENT2 billing/gift/gift_input.aspx"
        );
    }

    #[test]
    fn wallet_starts_empty_without_wire_data() {
        let state = WalletState::default();
        assert_eq!(state.gem, None);
        assert!(state.rows().iter().all(|row| row.amount.is_none()));
        assert_eq!(
            wallet_row_captions(&state),
            vec![
                "GEM --".to_owned(),
                "CASH POINT --".to_owned(),
                "POINT --".to_owned(),
                "O2 CASH --".to_owned(),
                "MUSIC CASH --".to_owned(),
                "ITEM CASH --".to_owned(),
            ]
        );
    }

    #[test]
    fn wallet_applies_cash_point_refresh_words() {
        let mut state = WalletState::default();
        state.apply_cash_point(&SyncCashPointResponse {
            gem: 1200,
            cash_point: 30,
        });
        assert_eq!(state.gem, Some(1200));
        assert_eq!(state.cash_point, Some(30));
        assert_eq!(state.point, None);
    }

    #[test]
    fn wallet_applies_sell_success_gem_word() {
        let mut state = WalletState::default();
        state.apply_cash_point(&SyncCashPointResponse {
            gem: 1200,
            cash_point: 30,
        });
        state.apply_sell_response(&SellItemResponse {
            result: 0,
            gem: Some(1500),
        });
        assert_eq!(state.gem, Some(1500));
        assert_eq!(state.cash_point, Some(30));
        assert_eq!(state.point, None);
    }

    #[test]
    fn wallet_keeps_gem_on_sell_error() {
        let mut state = WalletState::default();
        state.apply_cash_point(&SyncCashPointResponse {
            gem: 1200,
            cash_point: 30,
        });
        state.apply_sell_response(&SellItemResponse {
            result: -96,
            gem: None,
        });
        assert_eq!(state.gem, Some(1200));
        assert_eq!(state.cash_point, Some(30));
    }

    #[test]
    fn wallet_applies_purchase_prefix_words() {
        let mut state = WalletState::default();
        state.apply_purchase_prefix(&SyncItemPurchasePrefix {
            gem: 1200,
            point: 7,
            o2_cash: 450,
            cash_point: 30,
        });
        assert_eq!(state.gem, Some(1200));
        assert_eq!(state.point, Some(7));
        assert_eq!(state.o2_cash, Some(450));
        assert_eq!(state.cash_point, Some(30));
        assert_eq!(state.music_cash, None);
    }

    #[test]
    fn wallet_applies_purchase_wallet_pair() {
        let mut state = WalletState::default();
        state.apply_purchase_wallets(&SyncItemPurchaseWallets {
            music_cash: 450,
            item_cash: 125,
        });
        assert_eq!(state.music_cash, Some(450));
        assert_eq!(state.item_cash, Some(125));
        assert_eq!(state.gem, None);
    }

    #[test]
    fn wallet_keeps_latest_wire_words() {
        let mut state = WalletState::default();
        state.apply_cash_point(&SyncCashPointResponse {
            gem: 1200,
            cash_point: 30,
        });
        state.apply_purchase_prefix(&SyncItemPurchasePrefix {
            gem: 900,
            point: 7,
            o2_cash: 450,
            cash_point: 25,
        });
        assert_eq!(state.gem, Some(900));
        assert_eq!(state.cash_point, Some(25));
    }

    #[test]
    fn total_gem_bar_names_live_gem() {
        let mut state = WalletState::default();
        assert_eq!(total_gem_caption(&state), "TOTAL GEM --");
        state.apply_cash_point(&SyncCashPointResponse {
            gem: 1200,
            cash_point: 30,
        });
        assert_eq!(total_gem_caption(&state), "TOTAL GEM 1200");
    }

    #[test]
    fn wallet_rows_follow_display_order() {
        let mut state = WalletState::default();
        state.apply_purchase_prefix(&SyncItemPurchasePrefix {
            gem: 1200,
            point: 7,
            o2_cash: 450,
            cash_point: 30,
        });
        state.apply_purchase_wallets(&SyncItemPurchaseWallets {
            music_cash: 450,
            item_cash: 125,
        });
        let rows = state.rows();
        assert_eq!(rows.len(), WALLET_ROW_COUNT);
        assert_eq!(
            rows.map(|row| row.label),
            [
                GEM_LABEL,
                CASH_POINT_LABEL,
                POINT_LABEL,
                O2_CASH_LABEL,
                MUSIC_CASH_LABEL,
                ITEM_CASH_LABEL
            ]
        );
        assert_eq!(
            wallet_row_captions(&state),
            vec![
                "GEM 1200".to_owned(),
                "CASH POINT 30".to_owned(),
                "POINT 7".to_owned(),
                "O2 CASH 450".to_owned(),
                "MUSIC CASH 450".to_owned(),
                "ITEM CASH 125".to_owned(),
            ]
        );
    }

    #[test]
    fn wallet_row_caption_hides_unknown_amounts() {
        let row = WalletRow::named(GEM_LABEL, None);
        assert_eq!(wallet_row_caption(&row), "GEM --");
        let funded = WalletRow::named(GEM_LABEL, Some(1200));
        assert_eq!(wallet_row_caption(&funded), "GEM 1200");
    }

    fn side_free_pass_info(kind: FreePassType, expiry_secs: i32) -> SyncFreePassResponse {
        use o2jam_net::wallet::UserFreePassInfo;
        SyncFreePassResponse {
            result_code: 0,
            info: Some(UserFreePassInfo {
                gem: 1,
                point: 2,
                o2_cash: 3,
                music_cash: 4,
                item_cash: 5,
                free_pass_type: kind,
                cash_point: 6,
                duration: String::new(),
                expiry_secs,
            }),
        }
    }

    #[test]
    fn free_pass_starts_unknown_without_wire_data() {
        let state = FreePassState::default();
        assert_eq!(state.result, None);
        assert_eq!(state.kind, None);
        assert_eq!(state.expiry_secs, None);
        assert_eq!(
            free_pass_row_captions(&state),
            vec![
                "FREE PASS RESULT --".to_owned(),
                "FREE PASS --".to_owned(),
                "EXPIRY --".to_owned(),
            ]
        );
    }

    #[test]
    fn free_pass_applies_result_only_body_without_snapshot() {
        let mut state = FreePassState::default();
        state.apply_free_pass_response(&SyncFreePassResponse {
            result_code: 0,
            info: None,
        });
        assert_eq!(state.result, Some(SyncFreePassResult::Success));
        assert_eq!(state.kind, None);
        assert_eq!(state.expiry_secs, None);
        assert_eq!(
            free_pass_row_captions(&state),
            vec![
                "FREE PASS RESULT SUCCESS".to_owned(),
                "FREE PASS --".to_owned(),
                "EXPIRY --".to_owned(),
            ]
        );
    }

    #[test]
    fn free_pass_applies_snapshot_kind_and_expiry() {
        let mut state = FreePassState::default();
        state.apply_free_pass_response(&side_free_pass_info(FreePassType::PremiumTime, 3600));
        assert_eq!(state.result, Some(SyncFreePassResult::Success));
        assert_eq!(state.kind, Some(FreePassType::PremiumTime));
        assert_eq!(state.expiry_secs, Some(3600));
        assert_eq!(
            free_pass_row_captions(&state),
            vec![
                "FREE PASS RESULT SUCCESS".to_owned(),
                "FREE PASS PREMIUM TIME".to_owned(),
                "EXPIRY 3600".to_owned(),
            ]
        );
    }

    #[test]
    fn free_pass_clears_snapshot_on_result_only_follow_up() {
        let mut state = FreePassState::default();
        state.apply_free_pass_response(&side_free_pass_info(FreePassType::Top100, 60));
        state.apply_free_pass_response(&SyncFreePassResponse {
            result_code: 0,
            info: None,
        });
        assert_eq!(state.kind, None);
        assert_eq!(state.expiry_secs, None);
        assert_eq!(free_pass_kind_caption(state.kind), "--");
        assert_eq!(free_pass_expiry_caption(state.expiry_secs), "--");
    }

    #[test]
    fn free_pass_renders_placeholder_on_unknown_result_code() {
        let mut state = FreePassState::default();
        state.apply_free_pass_response(&SyncFreePassResponse {
            result_code: 7,
            info: None,
        });
        assert_eq!(state.result, None);
        assert_eq!(free_pass_result_caption(state.result), "--");
        assert_eq!(
            free_pass_row_captions(&state)[0],
            "FREE PASS RESULT --".to_owned()
        );
    }

    #[test]
    fn free_pass_names_unknown_verdict_without_guessing() {
        let mut state = FreePassState::default();
        state.apply_free_pass_response(&SyncFreePassResponse {
            result_code: 0xFFFF_FFA2,
            info: None,
        });
        assert_eq!(state.result, Some(SyncFreePassResult::Unknown));
        assert_eq!(free_pass_result_caption(state.result), "UNKNOWN");
    }

    #[test]
    fn free_pass_kind_labels_name_every_flag() {
        assert_eq!(free_pass_kind_label(FreePassType::None), "NONE");
        assert_eq!(free_pass_kind_label(FreePassType::AllMusic), "ALL MUSIC");
        assert_eq!(free_pass_kind_label(FreePassType::NewMusic), "NEW MUSIC");
        assert_eq!(
            free_pass_kind_label(FreePassType::Top100Extended),
            "TOP 100 EXTENDED"
        );
        assert_eq!(
            free_pass_kind_label(FreePassType::PremiumTime),
            "PREMIUM TIME"
        );
        assert_eq!(free_pass_kind_caption(None), "--");
        assert_eq!(
            free_pass_kind_caption(Some(FreePassType::Top100)),
            "TOP 100"
        );
    }

    #[test]
    fn tutorial_backdrop_loads_from_archive() {
        let red = sprite55_blob(1, 0x0000, 1, 1, &[0x00, 0x7C]);
        let archive = archive_blob(&[(TUTORIAL_BACKGROUND_SPRITE, &red)]);
        assert_eq!(load_tutorial_backdrop(&archive).unwrap().len(), 1);
        assert!(load_tutorial_backdrop(b"garbage").is_none());
        assert!(load_tutorial_backdrop(&archive_blob(&[])).is_none());
    }

    #[test]
    fn side_scene_skips_figures_without_art() {
        let list = o2jam_file::parse_control_list(&side_control_text()).unwrap();
        let block = scene_block(&list, SHOP_BLOCK).unwrap();
        let bnd = o2jam_file::parse_bnd(&bnd_blob(&[(0, 0, 8, 8), (0, 0, 8, 8)])).unwrap();
        let placed = bind_side_scene(block, &bnd);
        let mut buf = vec![0x123456u32; 8 * 8];
        paint_side_scene(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            &placed,
            &HashMap::new(),
        );
        assert_eq!(buf, vec![0x123456u32; 8 * 8]);
    }
}
