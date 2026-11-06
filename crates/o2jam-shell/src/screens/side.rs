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
use o2jam_net::{CharacterInfo, Difficulty, GameSpeed, GameStatsUpdate};
use o2jam_render::Frame;

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

/// One lanking board row: catalog position, not a score. No score
/// table ships with the client, so the board lists songs in catalog
/// order and numbers only the position.
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
