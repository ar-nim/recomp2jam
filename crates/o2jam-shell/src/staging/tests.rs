use super::*;

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
    assert_eq!(bound, 10, "ticket #3 rows stay bound");
    let playing = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Playing)
        .expect("playing row staged");
    assert_eq!(playing.arm, PlacementArm::PlayingSubset);
    for state in [State::Avi, State::Loading, State::Tutorial] {
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
fn result_row_merges_the_verdict_banner_by_name() {
    let row = STAGE_TABLE
        .iter()
        .find(|row| row.state == State::Result)
        .expect("result row staged");
    assert_eq!(
        row.extra_sprites,
        &["Result_WinLose.ojs"] as &[&str],
        "exe mounts the banner at file 0xc2390, outside the block"
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
    assert_eq!(
        tutorial.extra_sprites,
        &["Playing_TutorialBG.ojs"] as &[&str]
    );
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
fn avi_stages_its_logo_full_canvas_without_a_bind_file() {
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
        ("O2mediaLogo.ojs", &sprite55_blob(2, 2, &[1; 8])),
    ]);
    let report = stage_state(&archive, State::Avi, StageContext::default());
    assert!(report.misses.is_empty());
    let art = report.art.expect("avi stages with synthetic bytes");
    assert_eq!(art.placements.len(), 1, "one sprite, one canvas");
    assert_eq!(art.placements[0].sprite.as_deref(), Some("O2mediaLogo.ojs"));
    assert_eq!(art.placements[0].rect.x, 0);
    assert_eq!(art.placements[0].rect.y, 0);
    assert_eq!(art.placements[0].rect.w, 800, "native canvas width");
    assert_eq!(art.placements[0].rect.h, 600, "native canvas height");
    assert!(art.figures.contains_key("O2mediaLogo.ojs"));
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
fn tutorial_stages_its_named_body_with_no_block() {
    let archive = o2jam_file::test_archive::build_archive(&[(
        "Playing_TutorialBG.ojs",
        &sprite55_blob(2, 2, &[3; 8]),
    )]);
    let report = stage_state(&archive, State::Tutorial, StageContext::default());
    assert!(report.misses.is_empty(), "no block, no bind, no miss");
    let art = report.art.expect("tutorial stages with synthetic bytes");
    assert_eq!(art.placements.len(), 1);
    assert_eq!(
        art.placements[0].sprite.as_deref(),
        Some("Playing_TutorialBG.ojs")
    );
    assert!(art.figures.contains_key("Playing_TutorialBG.ojs"));
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
    }
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
            Some("_Playing_BG.ojs"),
            Some("Note_BG.ojs"),
            Some("P2P7KeyNote_BG.ojs"),
            Some("3Key_Note_BG.ojs"),
            Some("Playing_Menu.ojs"),
            None,
            Some("Vol_Mark.ojs"),
        ],
        "behind entries, backdrops, then the bound run"
    );
    assert_eq!(art.placements[5].rect.x, 1, "EXIT takes ordinal 1");
    assert_eq!(art.placements[6].rect.x, 10, "VOL takes ordinal 2");
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
    };
    let report = stage_state(&full_playing_archive(), State::Playing, ctx);
    let art = report.art.expect("playing stages with synthetic bytes");
    assert!(art.figures.contains_key("P2P7KeyNote_BG.ojs"));
    assert!(!art.figures.contains_key("Note_BG.ojs"));
    assert!(!art.figures.contains_key("3Key_Note_BG.ojs"));
    assert_eq!(art.placements.len(), 7, "placements ignore the family");
}

#[test]
fn playing_key_mode_selects_note_background() {
    let ctx = StageContext {
        arena_stage: Some(2),
        key_mode: 0,
        team_play: false,
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
        "Playing_BG2.ojs",
        "Note_BG.ojs",
        "P2P7KeyNote_BG.ojs",
        "3Key_Note_BG.ojs",
        "Playing_Menu.ojs",
    ]);
    let report = stage_state(&archive, State::Playing, solo_seven_key_context());
    let art = report.art.expect("a missing body never fails the scene");
    assert_eq!(art.placements.len(), 7, "placements survive the miss");
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
    assert_eq!(DIALOG_TABLE.len(), 6);
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
    assert_eq!(Dialog::PlanetQuit.block_name(), "DIALOG_QUESTION1");
    assert_eq!(Dialog::CreateRoom.block_name(), "DIALOG_CREATEROOM");
    assert_eq!(Dialog::InputPassword.block_name(), "DIALOG_INPUTPASSWORD");
}

#[test]
fn dialog_hosts_follow_the_mount() {
    assert_eq!(Dialog::SelectMusic.host(), State::Waiting);
    assert_eq!(Dialog::ChannelInfo.host(), State::Planet);
    assert_eq!(Dialog::PlanetOption.host(), State::Planet);
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
