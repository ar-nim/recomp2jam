use super::*;
use crate::avatar::{
    is_playing_backdrop_sprite, playing_backdrop_sprite, playing_member_frame,
    PLAYING_BACKDROP_CODE,
};
use crate::screens::side::{
    cash_charge_close_hit, shop_action_at, ShopAction, CASH_CHARGE_WEB_CODE,
    SHOP_ITEMBUY_FIRST_CODE, SHOP_ITEMGEM_FIRST_CODE, SHOP_ITEMGIFT_FIRST_CODE,
    SHOP_ITEMNAME_FIRST_CODE, SHOP_ITEMPRE_FIRST_CODE, SHOP_ITEMSHOW_FIRST_CODE,
    SHOP_ITEMWON_FIRST_CODE, TUTORIAL_RECEPTOR_SPRITES,
};

#[test]
fn states_key_by_their_block_names() {
    assert_eq!(State::Room.block_name(), "STATE_ROOM");
    assert_eq!(State::Waiting.block_name(), "STATE_WAITING");
    assert_eq!(State::Shop.block_name(), "STATE_SHOP");
    assert_eq!(State::MusicShop.block_name(), "STATE_MUSICSHOP");
    assert_eq!(State::MyRoom.block_name(), "STATE_MYROOM");
    assert_eq!(State::Planet.block_name(), "STATE_PLANET");
    assert_eq!(State::Lanking.block_name(), "STATE_LANKING");
    assert_eq!(State::Result.block_name(), "STATE_RESULT");
    assert_eq!(State::Payment.block_name(), "STATE_PAYMENT");
    assert_eq!(State::Payment2.block_name(), "STATE_PAYMENT2");
    assert_eq!(State::Playing.block_name(), "STATE_PLAYING");
    assert_eq!(State::Avi.block_name(), "STATE_AVI");
    assert_eq!(State::Loading.block_name(), "STATE_LOADING");
    assert_eq!(State::Tutorial.block_name(), "STATE_TUTORIAL");
}

#[test]
fn block_name_round_trips_through_from_block() {
    for state in [
        State::Room,
        State::Waiting,
        State::Shop,
        State::MusicShop,
        State::MyRoom,
        State::Planet,
        State::Lanking,
        State::Result,
        State::Payment,
        State::Payment2,
        State::Playing,
        State::Avi,
        State::Loading,
        State::Tutorial,
    ] {
        assert_eq!(State::from_block(state.block_name()), Some(state));
    }
}

#[test]
fn from_block_rejects_unknown_names() {
    assert_eq!(State::from_block("DIALOG_CHANNEL_INFO"), None);
    assert_eq!(State::from_block(""), None);
}

#[test]
fn declaration_holds_expected_rows_per_state() {
    assert_eq!(STAGE_TABLE.len(), 14);
    for row in STAGE_TABLE {
        if let Some(block) = row.block {
            assert_eq!(block, row.state.block_name());
        }
    }
    let bound = STAGE_TABLE
        .iter()
        .filter(|row| row.arm == PlacementArm::Bound)
        .count();
    assert_eq!(bound, 11, "ticket #10 rows stay bound");
    let playing = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Playing)
        .expect("playing row staged");
    assert_eq!(playing.arm, PlacementArm::PlayingSubset);
    for state in [State::Loading, State::Tutorial] {
        let row = STAGE_TABLE
            .iter()
            .find(|row| row.state == state)
            .expect("full-canvas row staged");
        assert_eq!(row.arm, PlacementArm::FullCanvas);
    }
}

#[test]
fn rows_read_their_own_control_list() {
    for row in STAGE_TABLE {
        if row.archive == StagingArchive::Playing {
            assert_eq!(
                row.archive.list_file(),
                "ControlList_Playing.txt",
                "the Playing archive holds {:?}",
                row.state
            );
        } else {
            assert_eq!(
                row.archive.list_file(),
                "ControlList_Interface.txt",
                "{:?} stages an interface Block",
                row.state
            );
        }
    }
}

#[test]
fn playing_row_carries_no_extras() {
    let row = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Playing)
        .expect("playing row staged");
    assert_eq!(row.block, Some("STATE_PLAYING"));
    assert_eq!(row.archive, StagingArchive::Playing);
    assert!(
        row.extra_sprites.is_empty(),
        "the subset stages the block alone; the skin rides caller-side"
    );
}

#[test]
fn result_row_merges_the_verdict_banner_and_bottom_counter_font_by_name() {
    let row = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Result)
        .expect("result row staged");
    assert_eq!(
        row.extra_sprites,
        &["Result_WinLose.ojs", "Result_NumFont_Blue.ojs"] as &[&str],
        "exe mounts the banner at file 0xc2390 and the bottom-counter \
         font at this+0x10d0c, both outside the block"
    );
}

#[test]
fn payment_rows_carry_no_extras() {
    for state in [State::Payment, State::Payment2] {
        let row = STAGE_TABLE
            .iter()
            .find(|row| row.state == state)
            .expect("payment row staged");
        assert!(
            row.extra_sprites.is_empty(),
            "both billing views stage through their own bound blocks"
        );
    }
}

#[test]
fn full_canvas_rows_name_their_archive_and_extras() {
    let avi = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Avi)
        .expect("avi row staged");
    assert_eq!(avi.block, Some("STATE_AVI"));
    assert_eq!(avi.archive, StagingArchive::Interface);
    assert_eq!(
        avi.arm,
        PlacementArm::Bound,
        "the logo pairs through its bound record, never full-canvas"
    );
    assert!(avi.extra_sprites.is_empty());
    let loading = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Loading)
        .expect("loading row staged");
    assert_eq!(loading.block, None, "no 7.55 list carries loading art");
    assert_eq!(loading.archive, StagingArchive::Playing);
    assert_eq!(loading.extra_sprites, &["MusicLoading.ojs"] as &[&str]);
    let tutorial = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Tutorial)
        .expect("tutorial row staged");
    assert_eq!(
        tutorial.block, None,
        "the interface stub names no bodies; the backdrop ships in Playing1"
    );
    assert_eq!(tutorial.archive, StagingArchive::Playing);
    let mut tutorial_names = vec!["Playing_TutorialBG.ojs", "Guide_Text.ojs", "Guide_Ani.ojs"];
    tutorial_names.extend(TUTORIAL_RECEPTOR_SPRITES);
    assert_eq!(tutorial.extra_sprites, tutorial_names.as_slice());
}

#[test]
fn stage_context_defaults_to_stage_less_solo() {
    let ctx = StageContext::default();
    assert_eq!(ctx.arena_stage, None);
    assert_eq!(ctx.key_mode, 0);
    assert!(!ctx.team_play);
}

/// Minimal sprite-55 container: header + one record + blob.
fn sprite55_blob(w: u16, h: u16, pixels: &[u8]) -> Vec<u8> {
    let mut blob = Vec::new();
    blob.extend_from_slice(&1u16.to_le_bytes());
    blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
    blob.extend_from_slice(&1u16.to_le_bytes());
    blob.extend_from_slice(&0u16.to_le_bytes());
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

/// One synthetic `State_ItemShop` archive: the pinned audit cells at
/// their raw `.bnd` coordinates plus empty rects for the rest of the
/// 186-record bind. No copyrighted archive bytes are used.
fn shop_archive() -> Vec<u8> {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_SHOP 0XA4 0X03",
        "{",
        "BOUND \"State_ItemShop.bnd\"",
        "IDC_STATE_SHOP 0X14000000",
        "IDC_IMAGE_SSHOP_ITEMSHOW1 0X14600035",
        "IDC_TEXT_SSHOP_ITEMNAME1 0X1440002C",
        "IDC_BUTTON_SSHOP_ITEMBUY1 0X14200018 \"Shop_Btn_ItemCheck.ojs\"",
        "IDC_BUTTON_SSHOP_ITEMGIFT1 0X14200020",
        "IDC_TEXT_SSHOP_ITEMWON1 0X14400054",
        "IDC_TEXT_SSHOP_ITEMGEM1 0X1440005C",
        "IDC_BUTTON_SSHOP_ITEMPRE1 0X14200064 \"Shop_ItemPre.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let mut rects = vec![(0, 0, 0, 0); 0xBA];
    rects[0] = (0, 0, 800, 600);
    // Ordinal = (code & 0xFFF) - 1, the exe pairing rule.
    rects[0x34] = (227, 86, 336, 167);
    rects[0x2B] = (231, 69, 331, 81);
    rects[0x17] = (220, 187, 301, 204);
    rects[0x1F] = (261, 187, 301, 204);
    rects[0x53] = (247, 169, 285, 182);
    rects[0x5B] = (300, 169, 338, 182);
    rects[0x63] = (308, 184, 348, 201);
    o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        ("State_ItemShop.bnd", &bnd_blob(&rects)),
        ("Shop_Btn_ItemCheck.ojs", &sprite55_blob(1, 1, &[1])),
        ("Shop_ItemPre.ojs", &sprite55_blob(1, 1, &[2])),
    ])
}

/// D1/D3 red proof: the exe puts the priced row children 70 px below
/// their raw `.bnd` rects (entry `05b24e64…cb00`, audit per-cell table),
/// while ITEMSHOW and ITEMNAME keep their raw origins.
#[test]
fn shop_stages_priced_children_at_the_exe_origin_but_shelf_art_raw() {
    // Live caller chain (tier 3, archives present):
    //   main.rs::run_play -> live_art::stage_live_scenes
    //   -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    //   -> stage_shop_child_origin -> placed child rects.
    // Archives absent -> skip cleanly (CI has no game data).
    let archive = shop_archive();
    let art = stage_state(&archive, State::Shop, StageContext::default())
        .art
        .expect("shop stages with synthetic bytes");
    let rect_y = |code: u32| {
        art.placements
            .iter()
            .find(|placed| placed.code == code)
            .expect("staged code")
            .rect
            .y
    };
    assert_eq!(
        [
            rect_y(SHOP_ITEMSHOW_FIRST_CODE),
            rect_y(SHOP_ITEMNAME_FIRST_CODE),
            rect_y(SHOP_ITEMWON_FIRST_CODE),
            rect_y(SHOP_ITEMGEM_FIRST_CODE),
        ],
        [86, 69, 239, 239],
        "shelf art stays raw; price cells inherit the exe child origin"
    );
}

/// D2 red proof: the visible 담기 art is at `(220,257)-(301,274)`,
/// so `(238,263)` picks the row; the raw BND center `(260,195)`
/// lands on the figure and must answer nothing (row-pick frame
/// `c84b952f…9922e`, audit D2).
#[test]
fn shop_row_pick_at_the_visible_button_hits_the_first_row() {
    // Live caller chain (tier 3, archives present):
    //   main.rs::run_play -> live_art::stage_live_scenes
    //   -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    //   -> ScreenGame::load_side_scene -> ScreenGame::click_shop
    //   -> shop_action_at -> ShopAction::ToggleRow.
    // Archives absent -> skip cleanly (CI has no game data).
    let archive = shop_archive();
    let art = stage_state(&archive, State::Shop, StageContext::default())
        .art
        .expect("shop stages with synthetic bytes");
    assert_eq!(
        shop_action_at(&art.placements, 238, 263),
        Some(ShopAction::ToggleRow(0)),
        "the exe-pinned visible button picks the row"
    );
    assert_eq!(
        shop_action_at(&art.placements, 260, 195),
        None,
        "the raw BND center is a figure, not a button"
    );
}

/// D1/D2/D3 mutant guard: art-less priced cells and the art-less
/// gift half move at staging; anchored buy/preview keep the raw
/// paint rect and shift their hit rect through the same constant.
#[test]
fn shop_priced_children_share_one_origin_without_double_moving_art() {
    // Live caller chain: main.rs::run_play -> live_art::stage_live_scenes
    // -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    // -> stage_shop_child_origin. Archives absent -> skip cleanly.
    let archive = shop_archive();
    let art = stage_state(&archive, State::Shop, StageContext::default())
        .art
        .expect("shop stages with synthetic bytes");
    let staged = |code: u32| {
        art.placements
            .iter()
            .find(|placed| placed.code == code)
            .expect("staged code")
            .rect
            .y
    };
    assert_eq!(
        [
            staged(SHOP_ITEMWON_FIRST_CODE),
            staged(SHOP_ITEMGEM_FIRST_CODE),
            staged(SHOP_ITEMGIFT_FIRST_CODE),
            staged(SHOP_ITEMBUY_FIRST_CODE),
            staged(SHOP_ITEMPRE_FIRST_CODE),
        ],
        [239, 239, 257, 187, 184],
        "ink and art-less gift move; anchored art keeps its raw rect"
    );
    let buy = art
        .placements
        .iter()
        .find(|placed| placed.code == SHOP_ITEMBUY_FIRST_CODE)
        .expect("staged buy");
    assert!(
        buy.sprite.is_some(),
        "the real buy family carries the sprite anchor"
    );
    assert_eq!(
        shop_action_at(&art.placements, 238, 263),
        Some(ShopAction::ToggleRow(0)),
        "the anchored buy answers at the exe's child origin"
    );
}

/// D1/D2/D3 archive-backed guard: the installed 7.55 bind staged
/// through the live boot seam puts the first visible buy hit at
/// `(238,263)` (exe row-pick frame `c84b952f…9922e`) and the
/// first won price at y 239 (entry frame `05b24e64…cb00`).
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_shop_bnd_stages_the_audit_visible_row_and_price_origin() {
    // Live caller chain (tier 3):
    //   main.rs::run_play -> live_art::stage_live_scenes
    //   -> live_art::stage_interface_scenes -> stage_state(State::Shop)
    //   -> stage_shop_child_origin -> shop_action_at.
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Image/Interface1.opi");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skip: missing install-root interface archive");
        return;
    };
    let art = stage_state(&bytes, State::Shop, StageContext::default())
        .art
        .expect("installed shop stages");
    assert_eq!(
        shop_action_at(&art.placements, 238, 263),
        Some(ShopAction::ToggleRow(0)),
        "the exe row-pick pixel reaches the first staged buy control"
    );
    assert_eq!(
        shop_action_at(&art.placements, 260, 195),
        None,
        "the raw BND center stays on the figure"
    );
    let won = art
        .placements
        .iter()
        .find(|placed| placed.code == SHOP_ITEMWON_FIRST_CODE)
        .expect("installed won cell");
    assert_eq!(
        (won.rect.x, won.rect.y),
        (247, 239),
        "the installed BND rect inherits the one child origin"
    );
}

fn room_archive() -> Vec<u8> {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_ROOM 0x02",
        "{",
        "BOUND \"State_Room.bnd\"",
        "IDC_ROOM 0X13000000",
        "IDC_LIST 0X13500001 \"Room_List.ojs\"",
        "IDC_BUTTON 0X13500002 \"Room_Button.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        (
            "State_Room.bnd",
            &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
        ),
        ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
        ("Room_Button.ojs", &sprite55_blob(2, 2, &[2; 8])),
    ])
}

#[test]
fn avi_stages_its_logo_through_the_bound_pairing() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_AVI 0x01",
        "{",
        "BOUND \"State_Avi.bnd\"",
        "IDC_STATE_AVI 0X18000000",
        "IDC_BACK_LOGO 0X18100001 \"O2mediaLogo.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        ("State_Avi.bnd", &bnd_blob(&[(0, 0, 800, 600)])),
        ("O2mediaLogo.ojs", &sprite55_blob(2, 2, &[1; 8])),
    ]);
    let report = stage_state(&archive, State::Avi, StageContext::default());
    assert!(report.misses.is_empty());
    let art = report.art.expect("avi stages with synthetic bytes");
    assert_eq!(art.placements.len(), 1, "self plus one bound entry");
    assert_eq!(art.placements[0].id, "IDC_BACK_LOGO");
    assert_eq!(art.placements[0].sprite.as_deref(), Some("O2mediaLogo.ojs"));
    assert_eq!(art.placements[0].rect.x, 0);
    assert_eq!(art.placements[0].rect.y, 0);
    assert_eq!(
        art.placements[0].rect.w, 800,
        "sole bind rect is full-bleed"
    );
    assert_eq!(
        art.placements[0].rect.h, 600,
        "sole bind rect is full-bleed"
    );
    assert!(art.figures.contains_key("O2mediaLogo.ojs"));
}

#[test]
fn avi_missing_bind_file_stages_nothing_with_a_miss() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_AVI 0x01",
        "{",
        "BOUND \"State_Avi.bnd\"",
        "IDC_STATE_AVI 0X18000000",
        "IDC_BACK_LOGO 0X18100001 \"O2mediaLogo.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive =
        o2jam_file::test_archive::build_archive(&[("ControlList_Interface.txt", list.as_bytes())]);
    let report = stage_state(&archive, State::Avi, StageContext::default());
    assert!(report.art.is_none(), "a missing bind stages nothing");
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].state, State::Avi);
    assert_eq!(report.misses[0].block, "STATE_AVI");
    assert_eq!(report.misses[0].entry, "State_Avi.bnd");
}

#[test]
fn loading_stages_its_named_body_with_no_block() {
    let archive = o2jam_file::test_archive::build_archive(&[(
        "MusicLoading.ojs",
        &sprite55_blob(2, 2, &[2; 8]),
    )]);
    let report = stage_state(&archive, State::Loading, StageContext::default());
    assert!(report.misses.is_empty(), "no block, no bind, no miss");
    let art = report.art.expect("loading stages with synthetic bytes");
    assert_eq!(art.placements.len(), 1);
    assert_eq!(
        art.placements[0].sprite.as_deref(),
        Some("MusicLoading.ojs")
    );
    assert_eq!(art.placements[0].rect.w, 800);
    assert_eq!(art.placements[0].rect.h, 600);
    assert!(art.figures.contains_key("MusicLoading.ojs"));
}

#[test]
fn tutorial_stages_all_seven_receptor_kits_from_the_archive() {
    // Stage VA `0x4dd290` (file `0xdd3e6`) resolves seven
    // `KeyEffect_Tutorial_%d.ojs` bodies, one per lane. Synthetic sprites
    // stand in for archive bytes; the real row-487 anchors are pinned by
    // the install-guarded painter guard in the shots walk.
    let kit = sprite55_blob(2, 2, &[11; 8]);
    let mut entries: Vec<(&str, &[u8])> = TUTORIAL_RECEPTOR_SPRITES
        .iter()
        .map(|name| (*name, kit.as_slice()))
        .collect();
    entries.extend([
        ("Playing_TutorialBG.ojs", kit.as_slice()),
        ("Guide_Text.ojs", kit.as_slice()),
        ("Guide_Ani.ojs", kit.as_slice()),
    ]);
    let archive = o2jam_file::test_archive::build_archive(&entries);
    let report = stage_state(&archive, State::Tutorial, StageContext::default());
    assert!(report.misses.is_empty(), "all seven kits resolve");
    let art = report.art.expect("seven kits stage");
    for name in TUTORIAL_RECEPTOR_SPRITES {
        assert!(art.figures.contains_key(name), "{name} stages");
        assert!(
            art.placements
                .iter()
                .any(|placed| placed.sprite.as_deref() == Some(name)),
            "{name} has a placement"
        );
    }
}

#[test]
fn tutorial_stages_backdrop_and_both_guide_bodies_with_no_block() {
    // Block-less full-canvas row: the B7 frame's backdrop, instruction
    // bubble (`1/13`) and coach come from three parsed records. Synthetic
    // bodies stand in for the archive bytes; the real layer counts and
    // anchors are pinned by the shots guard.
    let backdrop = sprite55_blob(2, 2, &[3; 8]);
    let instruction = sprite55_blob(2, 2, &[5; 8]);
    let coach = sprite55_blob(2, 2, &[7; 8]);
    let archive = o2jam_file::test_archive::build_archive(&[
        ("Playing_TutorialBG.ojs", &backdrop),
        ("Guide_Text.ojs", &instruction),
        ("Guide_Ani.ojs", &coach),
    ]);
    let report = stage_state(&archive, State::Tutorial, StageContext::default());
    assert_eq!(
        report.misses.len(),
        7,
        "only this fixture's seven kit bodies are missing"
    );
    let art = report.art.expect("tutorial stages with synthetic bytes");
    assert_eq!(art.placements.len(), 3 + TUTORIAL_RECEPTOR_SPRITES.len());
    for name in ["Playing_TutorialBG.ojs", "Guide_Text.ojs", "Guide_Ani.ojs"] {
        assert!(art.figures.contains_key(name), "{name} stages");
        assert!(
            art.placements
                .iter()
                .any(|placed| placed.sprite.as_deref() == Some(name)),
            "{name} has a placement"
        );
    }
}

#[test]
fn full_canvas_miss_keeps_placement_with_empty_slot() {
    let archive = o2jam_file::test_archive::build_archive(&[]);
    let report = stage_state(&archive, State::Loading, StageContext::default());
    let art = report.art.expect("placements survive the miss");
    assert_eq!(art.placements.len(), 1);
    assert!(art.figures.is_empty());
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].state, State::Loading);
    assert_eq!(report.misses[0].block, "STATE_LOADING");
    assert_eq!(report.misses[0].entry, "MusicLoading.ojs");
}

#[test]
fn byname_table_names_the_exe_literal_sheets_beside_playing() {
    let files: Vec<&str> = BYNAME_TABLE.iter().map(|row| row.bind_file).collect();
    assert_eq!(
        files,
        [
            "State_Playing_network.bnd",
            "Avatar_Info.bnd",
            "Map_Avatar.bnd"
        ],
        "the trio the exe loads by name, pinned by #165"
    );
    for row in BYNAME_TABLE {
        assert_eq!(row.archive, StagingArchive::Playing);
        assert_eq!(row.host, State::Playing);
    }
}

#[test]
fn byname_sheet_stages_file_order_rects_with_no_block() {
    let archive = o2jam_file::test_archive::build_archive(&[(
        "State_Playing_network.bnd",
        &bnd_blob(&[(265, 472, 389, 482), (410, 472, 534, 482)]),
    )]);
    let row = ByNameRow {
        bind_file: "State_Playing_network.bnd",
        archive: StagingArchive::Playing,
        host: State::Playing,
    };
    let report = stage_byname_sheet(&archive, &row);
    assert!(report.misses.is_empty());
    let art = report.art.expect("by-name sheet stages its rects");
    assert_eq!(art.placements.len(), 2, "one placement per file-order rect");
    assert_eq!(
        art.placements[0].rect,
        rect_from_corners(265, 472, 389, 482)
    );
    assert_eq!(
        art.placements[1].rect,
        rect_from_corners(410, 472, 534, 482)
    );
    assert!(
        art.placements.iter().all(|placed| placed.sprite.is_none()),
        "no block names a body, so no sprite rides along"
    );
    assert!(art.figures.is_empty());
}

#[test]
fn byname_sheet_missing_file_stages_nothing_with_a_miss() {
    let archive = o2jam_file::test_archive::build_archive(&[]);
    let row = ByNameRow {
        bind_file: "State_Playing_network.bnd",
        archive: StagingArchive::Playing,
        host: State::Playing,
    };
    let report = stage_byname_sheet(&archive, &row);
    assert!(report.art.is_none(), "a missing bind stages nothing");
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].state, State::Playing);
    assert_eq!(report.misses[0].block, "STATE_PLAYING");
    assert_eq!(report.misses[0].entry, "State_Playing_network.bnd");
}

#[test]
fn payment_views_stage_through_their_own_rows() {
    let list = [
        "NUMBER_OF_STATE 0x02",
        "STATE_PAYMENT 0x01",
        "{",
        "BOUND \"State_Payment.bnd\"",
        "IDC_PAY 0X1D000000",
        "IDC_BILL 0X1D100001 \"Pay_Bill.ojs\"",
        "}",
        "STATE_PAYMENT2 0x01",
        "{",
        "BOUND \"State_Payment2.bnd\"",
        "IDC_PAY2 0X1E000000",
        "IDC_GIFT 0X1E100001 \"Pay_Gift.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        ("State_Payment.bnd", &bnd_blob(&[(1, 2, 3, 4)])),
        ("State_Payment2.bnd", &bnd_blob(&[(5, 6, 7, 8)])),
        ("Pay_Bill.ojs", &sprite55_blob(2, 2, &[1; 8])),
        ("Pay_Gift.ojs", &sprite55_blob(2, 2, &[2; 8])),
    ]);
    let ctx = StageContext::default();
    let billing = stage_state(&archive, State::Payment, ctx);
    let gift = stage_state(&archive, State::Payment2, ctx);
    assert!(billing.misses.is_empty());
    assert!(gift.misses.is_empty());
    let billing = billing.art.expect("billing view stages");
    let gift = gift.art.expect("gift view stages");
    assert_eq!(billing.placements.len(), 1);
    assert_eq!(billing.placements[0].rect.x, 1);
    assert!(billing.figures.contains_key("Pay_Bill.ojs"));
    assert_eq!(gift.placements.len(), 1);
    assert_eq!(gift.placements[0].rect.x, 5);
    assert!(gift.figures.contains_key("Pay_Gift.ojs"));
}

#[test]
fn room_state_stages_named_figures_at_bound_rects() {
    let report = stage_state(&room_archive(), State::Room, StageContext::default());
    assert!(report.misses.is_empty());
    let art = report.art.expect("room stages with synthetic bytes");
    assert_eq!(art.placements.len(), 2, "self row takes no rect");
    assert_eq!(art.placements[0].sprite.as_deref(), Some("Room_List.ojs"));
    assert_eq!(art.placements[1].sprite.as_deref(), Some("Room_Button.ojs"));
    assert_eq!(art.placements[0].rect.x, 10);
    assert_eq!(art.placements[1].rect.x, 50);
    assert!(art.figures.contains_key("Room_List.ojs"));
    assert!(art.figures.contains_key("Room_Button.ojs"));
}

fn result_archive() -> Vec<u8> {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_RESULT 0x01",
        "{",
        "BOUND \"State_Result.bnd\"",
        "IDC_RESULT 0X17000000",
        "IDC_BACK 0X17100001 \"State_Result.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        ("State_Result.bnd", &bnd_blob(&[(0, 0, 800, 600)])),
        ("State_Result.ojs", &sprite55_blob(2, 2, &[3; 8])),
        ("Result_WinLose.ojs", &sprite55_blob(2, 2, &[4; 8])),
        ("Result_NumFont_Blue.ojs", &sprite55_blob(2, 2, &[5; 8])),
    ])
}

#[test]
fn result_state_merges_banner_by_name() {
    let report = stage_state(&result_archive(), State::Result, StageContext::default());
    assert!(report.misses.is_empty());
    let art = report.art.expect("result stages with synthetic bytes");
    assert_eq!(
        art.placements.len(),
        1,
        "banner mounts by code, not by rect"
    );
    assert!(art.figures.contains_key("State_Result.ojs"));
    assert!(
        art.figures.contains_key("Result_WinLose.ojs"),
        "exe mounts the banner at file 0xc2390"
    );
    assert!(
        art.figures.contains_key("Result_NumFont_Blue.ojs"),
        "exe mounts the bottom-counter font at this+0x10d0c"
    );
}

#[test]
fn missing_body_keeps_block_with_empty_slot() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_ROOM 0x02",
        "{",
        "BOUND \"State_Room.bnd\"",
        "IDC_ROOM 0X13000000",
        "IDC_LIST 0X13500001 \"Room_List.ojs\"",
        "IDC_BUTTON 0X13500002 \"Room_Button.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        (
            "State_Room.bnd",
            &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
        ),
        ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
    ]);
    let report = stage_state(&archive, State::Room, StageContext::default());
    let art = report.art.expect("a missing body never fails the scene");
    assert_eq!(art.placements.len(), 2, "placements survive the miss");
    assert!(art.figures.contains_key("Room_List.ojs"));
    assert!(!art.figures.contains_key("Room_Button.ojs"));
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].state, State::Room);
    assert_eq!(report.misses[0].block, "STATE_ROOM");
    assert_eq!(report.misses[0].entry, "Room_Button.ojs");
}

#[test]
fn missing_block_contributes_nothing() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_WAITING 0x00",
        "{",
        "BOUND \"State_Waiting.bnd\"",
        "IDC_WAITING 0X14000000",
        "}",
        "",
    ]
    .join("\n");
    let archive =
        o2jam_file::test_archive::build_archive(&[("ControlList_Interface.txt", list.as_bytes())]);
    let report = stage_state(&archive, State::Room, StageContext::default());
    assert!(report.art.is_none());
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].block, "STATE_ROOM");
    assert_eq!(report.misses[0].entry, "STATE_ROOM");
}

#[test]
fn missing_control_list_contributes_nothing() {
    let archive = o2jam_file::test_archive::build_archive(&[]);
    let report = stage_state(&archive, State::Room, StageContext::default());
    assert!(report.art.is_none());
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].entry, "ControlList_Interface.txt");
}

#[test]
fn missing_bind_file_contributes_nothing() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_ROOM 0x01",
        "{",
        "BOUND \"State_Room.bnd\"",
        "IDC_ROOM 0X13000000",
        "IDC_LIST 0X13500001 \"Room_List.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
    ]);
    let report = stage_state(&archive, State::Room, StageContext::default());
    assert!(report.art.is_none());
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].entry, "State_Room.bnd");
}

#[test]
fn corrupt_body_behaves_as_missing() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_ROOM 0x01",
        "{",
        "BOUND \"State_Room.bnd\"",
        "IDC_ROOM 0X13000000",
        "IDC_LIST 0X13500001 \"Room_List.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        ("State_Room.bnd", &bnd_blob(&[(10, 20, 30, 40)])),
        ("Room_List.ojs", b"not a sprite body"),
    ]);
    let report = stage_state(&archive, State::Room, StageContext::default());
    let art = report.art.expect("a corrupt body never fails the scene");
    assert!(art.figures.is_empty());
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].entry, "Room_List.ojs");
}

#[test]
fn placements_survive_total_figure_miss() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_ROOM 0x02",
        "{",
        "BOUND \"State_Room.bnd\"",
        "IDC_ROOM 0X13000000",
        "IDC_LIST 0X13500001 \"Room_List.ojs\"",
        "IDC_BUTTON 0X13500002 \"Room_Button.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        (
            "State_Room.bnd",
            &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
        ),
    ]);
    let report = stage_state(&archive, State::Room, StageContext::default());
    let art = report.art.expect("hit tests still read placements");
    assert_eq!(art.placements.len(), 2);
    assert!(art.figures.is_empty());
    assert_eq!(report.misses.len(), 2);
}

use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[derive(Debug, Default)]
struct WarnFields {
    state: Option<String>,
    block: Option<String>,
    entry: Option<String>,
}

impl Visit for WarnFields {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "state" => self.state = Some(value.to_owned()),
            "block" => self.block = Some(value.to_owned()),
            "entry" => self.entry = Some(value.to_owned()),
            _ => {}
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        let text = rendered
            .strip_prefix('"')
            .and_then(|cut| cut.strip_suffix('"'))
            .unwrap_or(&rendered);
        match field.name() {
            "state" => self.state = Some(text.to_owned()),
            "block" => self.block = Some(text.to_owned()),
            "entry" => self.entry = Some(text.to_owned()),
            _ => {}
        }
    }
}

#[derive(Debug, Default)]
struct WarnRecorder {
    warns: Mutex<Vec<WarnFields>>,
}

impl Subscriber for WarnRecorder {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
        if *event.metadata().level() != tracing::Level::WARN {
            return;
        }
        let mut fields = WarnFields::default();
        event.record(&mut fields);
        self.warns
            .lock()
            .expect("warn ledger readable")
            .push(fields);
    }
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

#[test]
fn miss_logs_warning_with_state_block_and_entry() {
    // One process-global recorder: every thread's warns land
    // here, so the probe uses a body name no other test stages
    // and filters for it instead of counting.
    static GLOBAL_RECORDER: std::sync::OnceLock<Arc<WarnRecorder>> = std::sync::OnceLock::new();
    let recorder = Arc::clone(GLOBAL_RECORDER.get_or_init(|| {
        let recorder = Arc::new(WarnRecorder::default());
        tracing::dispatcher::set_global_default(tracing::dispatcher::Dispatch::new(Arc::clone(
            &recorder,
        )))
        .expect("global warn recorder installs once");
        tracing::callsite::rebuild_interest_cache();
        recorder
    }));
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_ROOM 0x01",
        "{",
        "BOUND \"State_Room.bnd\"",
        "IDC_ROOM 0X13000000",
        "IDC_PROBE 0X13500001 \"Warn_Probe_Body.ojs\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        ("State_Room.bnd", &bnd_blob(&[(10, 20, 30, 40)])),
    ]);
    let _ = stage_state(&archive, State::Room, StageContext::default());
    let warns = recorder.warns.lock().expect("warn ledger readable");
    assert!(
        warns
            .iter()
            .any(|warn| warn.state.as_deref() == Some("STATE_ROOM")
                && warn.block.as_deref() == Some("STATE_ROOM")
                && warn.entry.as_deref() == Some("Warn_Probe_Body.ojs")),
        "miss warns with state, block, entry: {warns:?}"
    );
}

#[test]
fn screen_view_maps_bound_scenes_to_states() {
    assert_eq!(state_of_screen(Screen::Room), Some(State::Room));
    assert_eq!(state_of_screen(Screen::Waiting), Some(State::Waiting));
    assert_eq!(state_of_screen(Screen::Planet), Some(State::Planet));
    assert_eq!(state_of_screen(Screen::Result), Some(State::Result));
    assert_eq!(state_of_screen(Screen::Shop), Some(State::Shop));
    assert_eq!(state_of_screen(Screen::MusicShop), Some(State::MusicShop));
    assert_eq!(state_of_screen(Screen::Myroom), Some(State::MyRoom));
    assert_eq!(state_of_screen(Screen::Lanking), Some(State::Lanking));
    assert_eq!(
        state_of_screen(Screen::Playing),
        Some(State::Playing),
        "the Playing subset stages through the declaration"
    );
}

#[test]
fn screen_view_maps_full_canvas_scenes_to_states() {
    assert_eq!(state_of_screen(Screen::Avi), Some(State::Avi));
    assert_eq!(state_of_screen(Screen::Loading), Some(State::Loading));
}

#[test]
fn screen_view_leaves_future_arms_unmapped() {
    assert_eq!(state_of_screen(Screen::Channel), None, "no State class");
}

/// Synthetic `ControlList_Playing.txt` text: the self row, four
/// behind-highway scene entries, one backdrop panel, fillers to
/// index 213, then the bound `EXIT`..`VOL` run. Mirrors the exe
/// window the 17-rect `.bnd` pairs by ordinal.
fn playing_list_text() -> String {
    let mut lines = vec![
        "NUMBER_OF_STATE 0x01".to_owned(),
        "STATE_PLAYING 0x07".to_owned(),
        "{".to_owned(),
        "BOUND \"State_Playing.bnd\"".to_owned(),
        "IDC_PLAYING 0X1B000000".to_owned(),
        "IDC_BACK1 0X1B500071 \"_Playing_BG.ojs\"".to_owned(),
        "IDC_BACK2 0X1B500072 \"Note_BG.ojs\"".to_owned(),
        "IDC_BACK3 0X1B500073 \"P2P7KeyNote_BG.ojs\"".to_owned(),
        "IDC_BACK4 0X1B500074 \"3Key_Note_BG.ojs\"".to_owned(),
        "IDC_MENU 0X1B500075 \"Playing_Menu.ojs\"".to_owned(),
    ];
    for index in 6..213 {
        lines.push(format!("IDC_FILL{index:03} 0X1B000000"));
    }
    lines.push("IDC_EXIT 0X1B000001".to_owned());
    lines.push("IDC_VOL 0X1B000002 \"Vol_Mark.ojs\"".to_owned());
    lines.push("}".to_owned());
    lines.push(String::new());
    lines.join("\n")
}

fn playing_archive_with(bodies: &[&str]) -> Vec<u8> {
    let mut entries: Vec<(&str, Vec<u8>)> = vec![
        ("ControlList_Playing.txt", playing_list_text().into_bytes()),
        (
            "State_Playing.bnd",
            bnd_blob(&[(1, 2, 3, 4), (10, 20, 30, 40)]),
        ),
    ];
    for body in bodies {
        entries.push((*body, sprite55_blob(2, 2, &[1; 8])));
    }
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(name, blob)| (*name, blob.as_slice()))
        .collect();
    o2jam_file::test_archive::build_archive(&refs)
}

fn full_playing_archive() -> Vec<u8> {
    playing_archive_with(&[
        "Playing_NormalBG2.ojs",
        "Playing_BG2.ojs",
        "Note_BG.ojs",
        "P2P7KeyNote_BG.ojs",
        "3Key_Note_BG.ojs",
        "Playing_Menu.ojs",
        "Vol_Mark.ojs",
    ])
}

fn solo_seven_key_context() -> StageContext {
    StageContext {
        arena_stage: Some(2),
        key_mode: 2,
        team_play: false,
        chart_slot: 2,
    }
}

#[test]
fn playing_backdrop_name_cycles_by_chart_slot_mod_three() {
    // The exe setup sprintf's the BACK1 family with `%d = index % 3`,
    // a zero remainder taking 3 (file `0x4dc79f`-`0x4dc7bb`).
    assert_eq!(playing_backdrop_sprite(false, 0), "Playing_NormalBG3.ojs");
    assert_eq!(playing_backdrop_sprite(false, 1), "Playing_NormalBG1.ojs");
    assert_eq!(playing_backdrop_sprite(false, 2), "Playing_NormalBG2.ojs");
    assert_eq!(playing_backdrop_sprite(false, 3), "Playing_NormalBG3.ojs");
    assert_eq!(playing_backdrop_sprite(true, 2), "Playing_P2PBG2.ojs");
}

#[test]
fn playing_backdrop_sprite_membership_matches_runtime_built_names() {
    for name in [
        "Playing_NormalBG1.ojs",
        "Playing_NormalBG2.ojs",
        "Playing_NormalBG3.ojs",
        "Playing_P2PBG1.ojs",
        "Playing_P2PBG2.ojs",
        "Playing_TutorialBG.ojs",
    ] {
        assert!(is_playing_backdrop_sprite(name), "{name} stages behind");
    }
    for other in ["Playing_BG2.ojs", "Note_BG.ojs", "Playing_Menu.ojs"] {
        assert!(!is_playing_backdrop_sprite(other), "{other} is not BACK1");
    }
}

#[test]
fn playing_backdrop_loads_its_mode_arm_frame_only() {
    // The exe setup drives the frame by mode arm (SetFrame `0x42f870`,
    // mode-0 arm frame 2 -> image 1); the shell rides image 1
    // (`playing_member_frame`).
    assert_eq!(playing_member_frame("Playing_NormalBG2.ojs"), Some(1));
    assert_eq!(playing_member_frame("Playing_P2PBG1.ojs"), Some(1));
    assert_eq!(playing_member_frame("Note_BG.ojs"), Some(0));
    assert_eq!(playing_member_frame("Playing_Menu.ojs"), None);
}

#[test]
fn playing_subset_orders_behind_backdrops_then_bound_run() {
    let report = stage_state(
        &full_playing_archive(),
        State::Playing,
        solo_seven_key_context(),
    );
    assert!(report.misses.is_empty(), "misses: {:?}", report.misses);
    let art = report.art.expect("playing stages with synthetic bytes");
    let sprites: Vec<Option<&str>> = art
        .placements
        .iter()
        .map(|placed| placed.sprite.as_deref())
        .collect();
    assert_eq!(
        sprites,
        vec![
            Some("Playing_NormalBG2.ojs"),
            Some("_Playing_BG.ojs"),
            Some("Note_BG.ojs"),
            Some("P2P7KeyNote_BG.ojs"),
            Some("3Key_Note_BG.ojs"),
            Some("Playing_Menu.ojs"),
            None,
            Some("Vol_Mark.ojs"),
        ],
        "BACK1 backdrop, behind entries, backdrops, then the bound run"
    );
    assert_eq!(
        art.placements[0].code, PLAYING_BACKDROP_CODE,
        "the synthetic backdrop placement carries the BACK1 control code"
    );
    assert_eq!(art.placements[6].rect.x, 1, "EXIT takes ordinal 1");
    assert_eq!(art.placements[7].rect.x, 10, "VOL takes ordinal 2");
}

#[test]
fn playing_context_selects_solo_family_members() {
    let report = stage_state(
        &full_playing_archive(),
        State::Playing,
        solo_seven_key_context(),
    );
    let art = report.art.expect("playing stages with synthetic bytes");
    for staged in [
        "_Playing_BG.ojs",
        "Note_BG.ojs",
        "Playing_Menu.ojs",
        "Vol_Mark.ojs",
    ] {
        assert!(
            art.figures.contains_key(staged),
            "{staged} stages in a solo 7-key room"
        );
    }
    for missing in ["P2P7KeyNote_BG.ojs", "3Key_Note_BG.ojs"] {
        assert!(
            !art.figures.contains_key(missing),
            "unselected member {missing} leaves an empty slot"
        );
    }
}

#[test]
fn playing_context_selects_team_family_members() {
    let ctx = StageContext {
        arena_stage: Some(2),
        key_mode: 2,
        team_play: true,
        chart_slot: 2,
    };
    let report = stage_state(&full_playing_archive(), State::Playing, ctx);
    let art = report.art.expect("playing stages with synthetic bytes");
    assert!(art.figures.contains_key("P2P7KeyNote_BG.ojs"));
    assert!(!art.figures.contains_key("Note_BG.ojs"));
    assert!(!art.figures.contains_key("3Key_Note_BG.ojs"));
    assert_eq!(art.placements.len(), 8, "placements ignore the family");
}

#[test]
fn playing_key_mode_selects_note_background() {
    let ctx = StageContext {
        arena_stage: Some(2),
        key_mode: 0,
        team_play: false,
        chart_slot: 2,
    };
    let report = stage_state(&full_playing_archive(), State::Playing, ctx);
    let art = report.art.expect("playing stages with synthetic bytes");
    assert!(art.figures.contains_key("3Key_Note_BG.ojs"));
    assert!(!art.figures.contains_key("Note_BG.ojs"));
    assert!(!art.figures.contains_key("P2P7KeyNote_BG.ojs"));
}

#[test]
fn playing_missing_body_keeps_placement_with_empty_slot() {
    let archive = playing_archive_with(&[
        "Playing_NormalBG2.ojs",
        "Playing_BG2.ojs",
        "Note_BG.ojs",
        "P2P7KeyNote_BG.ojs",
        "3Key_Note_BG.ojs",
        "Playing_Menu.ojs",
    ]);
    let report = stage_state(&archive, State::Playing, solo_seven_key_context());
    let art = report.art.expect("a missing body never fails the scene");
    assert_eq!(art.placements.len(), 8, "placements survive the miss");
    assert!(!art.figures.contains_key("Vol_Mark.ojs"));
    assert_eq!(report.misses.len(), 1);
    assert_eq!(report.misses[0].state, State::Playing);
    assert_eq!(report.misses[0].block, "STATE_PLAYING");
    assert_eq!(report.misses[0].entry, "Vol_Mark.ojs");
}

#[test]
fn side_view_maps_bound_sides_to_states() {
    assert_eq!(state_of_side(SideScreen::Shop), Some(State::Shop));
    assert_eq!(state_of_side(SideScreen::MusicShop), Some(State::MusicShop));
    assert_eq!(state_of_side(SideScreen::MyRoom), Some(State::MyRoom));
    assert_eq!(state_of_side(SideScreen::Planet), Some(State::Planet));
    assert_eq!(state_of_side(SideScreen::Lanking), Some(State::Lanking));
    assert_eq!(
        state_of_side(SideScreen::Payment),
        Some(State::Payment),
        "gift twin merges caller-side"
    );
    assert_eq!(
        state_of_side(SideScreen::Tutorial),
        Some(State::Tutorial),
        "full-canvas arm, ticket #4"
    );
}

/// The b8 capture pins the Cash Charge frame at
/// `(171,3)-(648,595)`; its `.bnd` gives the client and close rects
/// dialog-relative, and the frame origin resolves both to native
/// pixels. Install data is local-only: CI skips cleanly.
#[test]
fn cash_charge_sheet_stages_the_b8_frame_client_and_close() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
    let Some(interface) = read_install_archives(&root).interface else {
        println!("skip: missing install-root interface archive");
        return;
    };
    let art = stage_cash_charge(&interface, StageContext::default());
    assert!(art.host.has_figures(), "Item Shop host board staged");
    let placed = |code: u32| {
        art.dialog
            .placements
            .iter()
            .find(|placed| placed.code == code)
            .cloned()
            .unwrap_or_else(|| panic!("Cash Charge control {code:#x} missing"))
    };
    let frame = placed(0x8310_0001);
    assert_eq!(
        frame.rect,
        Rect {
            x: 171,
            y: 3,
            w: 477,
            h: 592
        }
    );
    let web = placed(CASH_CHARGE_WEB_CODE);
    assert_eq!(
        web.rect,
        Rect {
            x: 190,
            y: 58,
            w: 440,
            h: 506
        }
    );
    let close = placed(0x8320_0003);
    assert_eq!(close.rect, frame.rect, "close anchor starts at the frame");
    assert_eq!(close.role, PlacementRole::Clone);
    assert!(art.dialog.figures.contains_key("Dialog_ChargeCash.ojs"));
    assert!(art.dialog.figures.contains_key("ChargeCash_Btn_Close.ojs"));
}

/// The boot-3 X press at native `(590,23)` must land on the staged
/// close sprite; points just outside its anchor box must not. Install
/// data is local-only: CI skips cleanly.
#[test]
fn cash_charge_close_press_hits_the_b8_sprite_anchor() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
    let Some(interface) = read_install_archives(&root).interface else {
        println!("skip: missing install-root interface archive");
        return;
    };
    let art = stage_cash_charge(&interface, StageContext::default());
    assert!(
        cash_charge_close_hit(&art.dialog, 590, 23),
        "boot-3 press lands on the parsed close sprite"
    );
    assert!(!cash_charge_close_hit(&art.dialog, 569, 23));
    assert!(!cash_charge_close_hit(&art.dialog, 620, 23));
    assert!(!cash_charge_close_hit(&art.dialog, 590, 4));
}

/// Spec #315 story 12: the six `DIALOG_MESSENGER_*` blocks stage at
/// their parsed `.bnd` rects (77/6/7/13/6/5). The B8 pack never mounts
/// these sheets, so this guard pins geometry only — never a live mount.
/// Install data is local-only: CI skips cleanly.
#[test]
fn messenger_sheet_rows_stage_the_pinned_rect_counts() {
    // Live caller chain: main.rs::run_play -> assemble_online_session ->
    // session::OnlineSession::stage_live_art -> live_art::stage_live_scenes
    // -> stage_interface_scenes -> stage_messenger_sheets ->
    // staging::stage_messenger_sheet -> resolve_bound_row ->
    // o2jam_file::parse_bnd (this guard reads the same six blocks).
    // No sheet paints at boot; the staged geometry is the whole claim.
    // Archives absent -> skip cleanly (CI has no game data).
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
    let Some(interface) = read_install_archives(&root).interface else {
        println!("skip: missing install-root interface archive");
        return;
    };
    let expected_counts = [77usize, 6, 7, 13, 6, 5];
    assert_eq!(MESSENGER_SHEETS.len(), expected_counts.len());
    for (sheet, expected) in MESSENGER_SHEETS.into_iter().zip(expected_counts) {
        let block = sheet.block_name();
        let row = StageRow {
            state: State::Planet,
            block: Some(block),
            archive: StagingArchive::Interface,
            arm: PlacementArm::Bound,
            extra_sprites: &[],
        };
        let mut report = StageReport::default();
        let resolved = resolve_bound_row(&interface, &row, &mut report)
            .unwrap_or_else(|| panic!("{block} did not resolve"));
        assert_eq!(
            resolved.bind.rects.len(),
            expected,
            "{block} parsed rect count"
        );
        let staged = stage_messenger_sheet(&interface, sheet, StageContext::default());
        assert!(staged.misses.is_empty(), "{block} staged with misses");
        let art = staged
            .art
            .unwrap_or_else(|| panic!("{block} staged nothing"));
        // `SET` groups clone template members, so placement count can
        // exceed the parsed rect count; every placement comes from the
        // same bound arm the host states use.
        assert!(
            art.placements.len() >= expected,
            "{block} staged placements"
        );
    }
}

/// Spec #315 story 13: each Messenger sheet's `BOUND` block authors
/// children relative to its frame rect, so staging must add the parsed
/// frame origin to every child without moving the frame itself.
#[test]
fn messenger_sheet_children_stage_at_the_parsed_parent_origin() {
    // Live caller chain: main.rs::run_play -> assemble_online_session ->
    // session::OnlineSession::stage_live_art -> live_art::stage_live_scenes
    // -> stage_interface_scenes -> stage_messenger_sheets ->
    // staging::stage_messenger_sheet -> stage_bound_row -> parse_bnd.
    // The guard compares that result with the same bound arm before the
    // parent offset is applied; no mounted sheet is claimed. Archives
    // absent -> skip cleanly (CI has no game data).
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
    let Some(interface) = read_install_archives(&root).interface else {
        println!("skip: missing install-root interface archive");
        return;
    };
    for sheet in MESSENGER_SHEETS {
        let row = StageRow {
            state: State::Planet,
            block: Some(sheet.block_name()),
            archive: StagingArchive::Interface,
            arm: PlacementArm::Bound,
            extra_sprites: &[],
        };
        let raw = stage_bound_row(&interface, &row, StageContext::default())
            .art
            .unwrap_or_else(|| panic!("{} did not stage raw rects", sheet.block_name()));
        let staged = stage_messenger_sheet(&interface, sheet, StageContext::default())
            .art
            .unwrap_or_else(|| panic!("{} did not stage sheet rects", sheet.block_name()));
        let frame = raw
            .placements
            .iter()
            .find(|placed| {
                placed
                    .sprite
                    .as_deref()
                    .is_some_and(|sprite| sprite.starts_with("Dialog_"))
            })
            .unwrap_or_else(|| panic!("{} has no frame placement", sheet.block_name()));
        assert_eq!(
            staged.placements.len(),
            raw.placements.len(),
            "{} placement count",
            sheet.block_name()
        );
        let mut children = 0;
        for (before, after) in raw.placements.iter().zip(&staged.placements) {
            let is_frame =
                before.sprite.as_deref() == frame.sprite.as_deref() && before.rect == frame.rect;
            let expected = if is_frame {
                before.rect
            } else {
                children += 1;
                Rect {
                    x: before.rect.x + frame.rect.x,
                    y: before.rect.y + frame.rect.y,
                    ..before.rect
                }
            };
            assert_eq!(
                after.rect,
                expected,
                "{} child {:?} must sit at its parent origin",
                sheet.block_name(),
                before.id
            );
        }
        assert!(
            children > 0,
            "{} has children to offset",
            sheet.block_name()
        );
    }
}

#[test]
fn install_loader_reads_archives_from_given_root() {
    let root = test_scratch_root("reads");
    let interface_dir = root.join("O2Jam/Image");
    std::fs::create_dir_all(&interface_dir).expect("scratch dir writable");
    let interface = o2jam_file::test_archive::build_archive(&[("a.ojs", b"a")]);
    let playing = o2jam_file::test_archive::build_archive(&[("b.ojs", b"b")]);
    std::fs::write(interface_dir.join("Interface1.opi"), &interface)
        .expect("scratch file writable");
    std::fs::write(interface_dir.join("Playing1.opi"), &playing).expect("scratch file writable");
    let loaded = read_install_archives(&root);
    assert_eq!(loaded.interface, Some(interface));
    assert_eq!(loaded.playing, Some(playing));
}

#[test]
fn install_loader_skips_cleanly_without_game_data() {
    let root = test_scratch_root("missing");
    std::fs::create_dir_all(&root).expect("scratch dir writable");
    let loaded = read_install_archives(&root);
    assert_eq!(loaded, InstallArchives::default());
}

/// Scratch dir under `target/`: hermetic, never `/tmp`, and
/// git-ignored with the rest of the build tree.
fn test_scratch_root(case: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/staging-loader-test")
        .join(case)
}

#[test]
fn scrollbar_params_never_warn() {
    let list = [
        "NUMBER_OF_STATE 0x01",
        "STATE_ROOM 0x02",
        "{",
        "BOUND \"State_Room.bnd\"",
        "IDC_ROOM 0X13000000",
        "IDC_LIST 0X13500001 \"Room_List.ojs\"",
        "IDC_SCROLL 0X13021016 \"O2_SBS_VERT\t7\t\t100\"",
        "}",
        "",
    ]
    .join("\n");
    let archive = o2jam_file::test_archive::build_archive(&[
        ("ControlList_Interface.txt", list.as_bytes()),
        (
            "State_Room.bnd",
            &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
        ),
        ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
    ]);
    let report = stage_state(&archive, State::Room, StageContext::default());
    assert!(report.art.is_some());
    assert!(
        report.misses.is_empty(),
        "scrollbar params are never files: {:?}",
        report.misses
    );
}
#[test]
fn dialog_rows_name_host_state_and_block() {
    assert_eq!(DIALOG_TABLE.len(), 10);
    for row in DIALOG_TABLE {
        assert_eq!(row.block, row.dialog.block_name());
        assert_eq!(row.host, row.dialog.host());
    }
}

#[test]
fn dialogs_key_by_their_block_names() {
    assert_eq!(Dialog::SelectMusic.block_name(), "DAILOG_SELECTMUSIC");
    assert_eq!(Dialog::ChannelInfo.block_name(), "DIALOG_CHANNEL_INFO");
    assert_eq!(Dialog::PlanetOption.block_name(), "DIALOG_OPTION_KEY");
    assert_eq!(
        Dialog::PlanetOptionSound.block_name(),
        "DIALOG_OPTION_SOUND"
    );
    assert_eq!(Dialog::PlanetOptionGame.block_name(), "DIALOG_OPTION_GAME");
    assert_eq!(Dialog::PlanetQuit.block_name(), "DIALOG_QUESTION1");
    assert_eq!(Dialog::CreateRoom.block_name(), "DIALOG_CREATEROOM");
    assert_eq!(Dialog::InputPassword.block_name(), "DIALOG_INPUTPASSWORD");
    assert_eq!(Dialog::ChargeCash.block_name(), "DIALOG_CHARGECASH");
}

#[test]
fn dialog_hosts_follow_the_mount() {
    assert_eq!(Dialog::SelectMusic.host(), State::Waiting);
    assert_eq!(Dialog::ChannelInfo.host(), State::Planet);
    assert_eq!(Dialog::PlanetOption.host(), State::Planet);
    assert_eq!(Dialog::PlanetOptionSound.host(), State::Planet);
    assert_eq!(Dialog::PlanetOptionGame.host(), State::Planet);
    assert_eq!(Dialog::PlanetQuit.host(), State::Planet);
    assert_eq!(Dialog::CreateRoom.host(), State::Room);
    assert_eq!(Dialog::InputPassword.host(), State::Room);
}

#[test]
fn dialog_block_name_round_trips_through_from_block() {
    for dialog in [
        Dialog::SelectMusic,
        Dialog::ChannelInfo,
        Dialog::PlanetOption,
        Dialog::PlanetOptionSound,
        Dialog::PlanetOptionGame,
        Dialog::PlanetQuit,
        Dialog::CreateRoom,
        Dialog::InputPassword,
    ] {
        assert_eq!(Dialog::from_block(dialog.block_name()), Some(dialog));
    }
    assert_eq!(Dialog::from_block("STATE_ROOM"), None);
    assert_eq!(Dialog::from_block("Dialog_ChannelSelectBar.ojs"), None);
}

/// Synthetic archive holding one dialog block: `named` bodies are
/// block entries (table-driven), `unnamed` bodies sit in the archive
/// with no block entry (never staged, like the code-mounted bar).
fn dialog_archive(block: &str, bind: &str, named: &[&str], unnamed: &[&str]) -> Vec<u8> {
    let entries = named
        .iter()
        .enumerate()
        .map(|(index, body)| format!("IDC_ROW{index} 0X8C10000{} \"{body}\"", index + 1))
        .collect::<Vec<_>>()
        .join("\n");
    let list = [
        "NUMBER_OF_DIALOG 0x01".to_owned(),
        format!("{block} 0x02"),
        "{".to_owned(),
        format!("BOUND \"{bind}\""),
        "IDC_DIALOG 0X8C000000".to_owned(),
        entries,
        "}".to_owned(),
        String::new(),
    ]
    .join("\n");
    let bind_blob = bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]);
    let owned: Vec<Vec<u8>> = named
        .iter()
        .chain(unnamed.iter())
        .map(|_| sprite55_blob(2, 2, &[1; 8]))
        .collect();
    let mut files: Vec<(&str, &[u8])> = vec![
        ("ControlList_Interface.txt", list.as_bytes()),
        (bind, &bind_blob),
    ];
    for (name, blob) in named.iter().chain(unnamed.iter()).zip(owned.iter()) {
        files.push((name, blob));
    }
    o2jam_file::test_archive::build_archive(&files)
}

#[test]
fn select_music_stages_over_waiting() {
    let archive = dialog_archive(
        "DAILOG_SELECTMUSIC",
        "Dialog_SelectMusic.bnd",
        &["Select_Song.ojs", "Select_Artist.ojs"],
        &[],
    );
    let report = stage_dialog(&archive, Dialog::SelectMusic, StageContext::default());
    assert!(report.misses.is_empty(), "{:?}", report.misses);
    let art = report
        .art
        .expect("select-music stages with synthetic bytes");
    assert_eq!(art.placements.len(), 2);
    assert!(art.figures.contains_key("Select_Song.ojs"));
    assert!(art.figures.contains_key("Select_Artist.ojs"));
}

#[test]
fn channel_info_stages_over_planet() {
    let archive = dialog_archive(
        "DIALOG_CHANNEL_INFO",
        "Dialog_ChannelInfo.bnd",
        &["Dialog_ChannelInfo.ojs", "ChannelInfo_CH_Back.ojs"],
        &[],
    );
    let report = stage_dialog(&archive, Dialog::ChannelInfo, StageContext::default());
    assert!(report.misses.is_empty(), "{:?}", report.misses);
    let art = report
        .art
        .expect("channel info stages with synthetic bytes");
    assert_eq!(art.placements.len(), 2);
    assert!(art.figures.contains_key("Dialog_ChannelInfo.ojs"));
}

#[test]
fn planet_sheets_stage_over_planet() {
    let option = dialog_archive(
        "DIALOG_OPTION_KEY",
        "Dialog_Option_Key.bnd",
        &["Dialog_Option_Key.ojs"],
        &[],
    );
    let quit = dialog_archive(
        "DIALOG_QUESTION1",
        "Dialog_Question1.bnd",
        &["Dialog_Question1.ojs"],
        &[],
    );
    let ctx = StageContext::default();
    let option_art = stage_dialog(&option, Dialog::PlanetOption, ctx)
        .art
        .expect("option sheet stages");
    let quit_art = stage_dialog(&quit, Dialog::PlanetQuit, ctx)
        .art
        .expect("quit sheet stages");
    assert!(option_art.figures.contains_key("Dialog_Option_Key.ojs"));
    assert!(quit_art.figures.contains_key("Dialog_Question1.ojs"));
}

/// Ticket #244: the relay-down notice uses the same declaration as
/// every other popup. `DIALOG_INFORMATION` is a named block over
/// Planet, and its `Info_OK.ojs` body stages with the sheet.
#[test]
fn information_dialog_stages_the_one_ok_sheet() {
    assert_eq!(Dialog::Information.block_name(), "DIALOG_INFORMATION");
    assert_eq!(Dialog::Information.host(), State::Planet);
    assert_eq!(
        Dialog::from_block("DIALOG_INFORMATION"),
        Some(Dialog::Information)
    );
    let archive = dialog_archive(
        "DIALOG_INFORMATION",
        "Dialog_Information.bnd",
        &["Dialog_Information.ojs", "Info_OK.ojs"],
        &[],
    );
    let art = stage_dialog(&archive, Dialog::Information, StageContext::default())
        .art
        .expect("information sheet stages with synthetic bytes");
    assert!(art.figures.contains_key("Dialog_Information.ojs"));
    assert!(art.figures.contains_key("Info_OK.ojs"));
}

#[test]
fn room_dialogs_stage_without_a_caller() {
    let create = dialog_archive(
        "DIALOG_CREATEROOM",
        "Dialog_CreateRoom.bnd",
        &["Dialog_CreateRoom.ojs"],
        &[],
    );
    let password = dialog_archive(
        "DIALOG_INPUTPASSWORD",
        "Dialog_Password.bnd",
        &["Dialog_Password.ojs"],
        &[],
    );
    let ctx = StageContext::default();
    let create_art = stage_dialog(&create, Dialog::CreateRoom, ctx)
        .art
        .expect("create-room stages");
    let password_art = stage_dialog(&password, Dialog::InputPassword, ctx)
        .art
        .expect("input-password stages");
    assert!(create_art.figures.contains_key("Dialog_CreateRoom.ojs"));
    assert!(password_art.figures.contains_key("Dialog_Password.ojs"));
}

#[test]
fn table_driven_bodies_leave_the_select_bar_to_the_caller() {
    let archive = dialog_archive(
        "DIALOG_CHANNEL_INFO",
        "Dialog_ChannelInfo.bnd",
        &["Dialog_ChannelInfo.ojs"],
        &["Dialog_ChannelSelectBar.ojs"],
    );
    let report = stage_dialog(&archive, Dialog::ChannelInfo, StageContext::default());
    let art = report
        .art
        .expect("channel info stages with synthetic bytes");
    assert!(
        !art.figures.contains_key("Dialog_ChannelSelectBar.ojs"),
        "the bar is code-mounted at the Dialog object, never table-driven"
    );
}
#[test]
fn probe_waiting_top_overlaps() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Image/Interface1.opi");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("MISSING INSTALL");
        return;
    };
    let report = crate::staging::stage_state(
        &bytes,
        crate::staging::State::Waiting,
        crate::staging::StageContext::default(),
    );
    let art = report.art.expect("stages");
    let mut top: Vec<_> = art.placements.iter().filter(|p| p.rect.y < 60).collect();
    top.sort_by_key(|p| (p.rect.x, p.rect.y));
    for p in &top {
        eprintln!(
            "{} code={:08X} rect={},{},{}x{} role={:?}",
            p.id, p.code, p.rect.x, p.rect.y, p.rect.w, p.rect.h, p.role
        );
    }
    // pairwise overlap among top-row buttons
    for (i, a) in top.iter().enumerate() {
        for b in &top[i + 1..] {
            let overlap_x =
                a.rect.x < b.rect.x + b.rect.w as i32 && b.rect.x < a.rect.x + a.rect.w as i32;
            let overlap_y =
                a.rect.y < b.rect.y + b.rect.h as i32 && b.rect.y < a.rect.y + a.rect.h as i32;
            if overlap_x && overlap_y {
                eprintln!("OVERLAP {} vs {}", a.id, b.id);
            }
        }
    }
}

#[test]
fn probe_waiting_buttons_all() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Image/Interface1.opi");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("MISSING INSTALL");
        return;
    };
    let report = crate::staging::stage_state(
        &bytes,
        crate::staging::State::Waiting,
        crate::staging::StageContext::default(),
    );
    let art = report.art.expect("stages");
    let mut all: Vec<_> = art.placements.iter().collect();
    all.sort_by_key(|p| (p.rect.y, p.rect.x));
    for p in &all {
        if p.id.contains("BUTTON") || p.rect.y < 45 {
            eprintln!(
                "{} code={:08X} rect={},{},{}x{} role={:?}",
                p.id, p.code, p.rect.x, p.rect.y, p.rect.w, p.rect.h, p.role
            );
        }
    }
}

#[test]
fn probe_waiting_right_side() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Image/Interface1.opi");
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let report = crate::staging::stage_state(
        &bytes,
        crate::staging::State::Waiting,
        crate::staging::StageContext::default(),
    );
    let art = report.art.expect("stages");
    let mut all: Vec<_> = art.placements.iter().filter(|p| p.rect.x >= 600).collect();
    all.sort_by_key(|p| (p.rect.y, p.rect.x));
    for p in &all {
        eprintln!(
            "{} code={:08X} rect={},{},{}x{} role={:?}",
            p.id, p.code, p.rect.x, p.rect.y, p.rect.w, p.rect.h, p.role
        );
    }
    // all pairwise overlaps excluding the backdrop
    let items: Vec<_> = art.placements.iter().filter(|p| p.rect.w < 800).collect();
    for (i, a) in items.iter().enumerate() {
        for b in &items[i + 1..] {
            let ox = a.rect.x < b.rect.x + b.rect.w as i32 && b.rect.x < a.rect.x + a.rect.w as i32;
            let oy = a.rect.y < b.rect.y + b.rect.h as i32 && b.rect.y < a.rect.y + a.rect.h as i32;
            if ox && oy && a.id != b.id {
                eprintln!("OVERLAP {} vs {}", a.id, b.id);
            }
        }
    }
}
