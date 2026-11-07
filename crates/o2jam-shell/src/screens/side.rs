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
use o2jam_net::{CharacterInfo, Difficulty, GameSpeed, GameStatsUpdate};
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
/// the 숫자/기타 words (42 cells over two rows). Hangul and word
/// cells are `None` — the micro-font has no CP949 glyphs — and keep
/// their grid slots so the letters land at their reference positions.
#[must_use]
pub fn select_chosung_cells() -> Vec<Option<u8>> {
    let mut cells = vec![None; SELECT_CHO_SUNG_JAMO];
    cells.extend((b'A'..=b'Z').map(Some));
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

/// Chosung grid (native pixels): row tops, left edge, cell pitch, and
/// the cell counts (row 1: 14 jamo + 13 letters; row 2: 13 letters +
/// 2 word cells).
pub const SELECT_CHO_SUNG_Y: [i32; 2] = [444, 462];
pub const SELECT_CHO_SUNG_LEFT: i32 = 352;
pub const SELECT_CHO_SUNG_PITCH: i32 = 15;
pub const SELECT_CHO_SUNG_JAMO: usize = 14;
pub const SELECT_CHO_SUNG_WORDS: usize = 2;
pub const SELECT_CHO_SUNG_ROW_CELLS: [usize; 2] =
    [SELECT_CHO_SUNG_JAMO + 13, 13 + SELECT_CHO_SUNG_WORDS];

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

    fn side_item_table() -> o2jam_file::AvatarItemTable {
        let row = |item_id: u32, name: &str, price: u32| o2jam_file::AvatarItemRow {
            item_id,
            item_type: 0,
            price,
            params: [0; 4],
            flags: (0, 0),
            name: name.to_owned(),
            description: String::new(),
            sprites: Vec::new(),
        };
        o2jam_file::AvatarItemTable {
            rows: vec![row(1, "Hair", 1000), row(2, "Boots", 2500)],
        }
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

    #[test]
    fn select_chosung_cells_span_jamo_letters_and_words() {
        let cells = select_chosung_cells();
        assert_eq!(cells.len(), 42);
        assert!(cells[..SELECT_CHO_SUNG_JAMO].iter().all(Option::is_none));
        let letters: Vec<u8> = cells.iter().filter_map(|cell| *cell).collect();
        assert_eq!(letters, (b'A'..=b'Z').collect::<Vec<_>>());
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
