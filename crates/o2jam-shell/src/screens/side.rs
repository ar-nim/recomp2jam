//! Side screens reachable from the lobbies (`docs/lifecycle.md` §12).
//!
//! Channel, room, and waiting each link to the shop, music shop, my
//! room, planet, lanking, payment, and tutorial views. Unlike the
//! main-flow scenes, these never use flat-color stand-ins: every
//! view paints real archive art through [`load_scene_figures`].

use std::collections::HashMap;

use super::super::avatar::{
    bind_scene_rects, load_scene_figures, paint_scene_figures, scene_block, scene_sprite_names,
    AvatarFigure, ScenePlacement,
};
use super::super::screen::Screen;
use o2jam_file::{Bnd, ControlBlock, ControlList};
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
