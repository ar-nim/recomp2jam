//! Release-only timing harness for the Item Shop open path (#755).
//!
//! The ignored profile test records stage durations with real 7.55 data;
//! the budget guard runs in the normal suite and skips without game data.
//! Run the profile with:
//! `cargo test --release --bin recomp2jam shop_open_profile -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use o2jam_net::CharacterInfo;
use o2jam_shell::screens::SideScreen;
use o2jam_shell::{GameLoop, MenuKey, ScreenGame};

const AVATAR_ARCHIVE: &str = "O2Jam/Image/avatar.opa";

/// Second identity landing plus shop open must stay well under the
/// measured 135 ms release baseline (debug baseline: 490 ms).
const SECOND_LANDING_AND_OPEN_BUDGET: Duration = Duration::from_millis(75);

fn bench_character() -> CharacterInfo {
    CharacterInfo {
        equipment: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ..CharacterInfo::default()
    }
}

fn install_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

fn avatar_path() -> PathBuf {
    install_root().join(AVATAR_ARCHIVE)
}

/// Walk from the splash to the lobby that hosts the shop button.
fn walk_to_channel(game: &mut ScreenGame) {
    game.menu_key(MenuKey::Back, true);
    game.click_planet(136, 88);
    assert_eq!(game.screen(), o2jam_shell::Screen::Channel);
}

/// Stage the live boot's shop prerequisites from installed data:
/// scene art, channel lobby, character, then the avatar source.
fn staged_shop_game(archive: Vec<u8>, table: o2jam_file::AvatarItemTable) -> ScreenGame {
    let mut game = ScreenGame::new();
    time("stage_live_scenes", || {
        crate::live_art::stage_live_scenes(&mut game)
    });
    walk_to_channel(&mut game);
    game.set_myroom_character(bench_character());
    time("set_avatar_source", || {
        game.set_avatar_source(archive, table);
    });
    game
}

fn read_avatar_source() -> Option<(Vec<u8>, o2jam_file::AvatarItemTable)> {
    crate::connect::read_avatar_source(avatar_path().to_str()?).ok()
}

fn time<F: FnOnce()>(label: &str, operation: F) -> Duration {
    let started = Instant::now();
    operation();
    let elapsed = started.elapsed();
    println!("{label}: {elapsed:?}");
    elapsed
}

/// Measure the stages behind one open, then repeated open/leave cycles.
/// Each cycle needs a fresh open: `leave_side` is part of the gesture.
#[test]
#[ignore = "measurement harness; run in release with --ignored --nocapture"]
fn shop_open_profile_records_stage_durations() {
    let source = Instant::now();
    let Some((archive, table)) = read_avatar_source() else {
        println!("skip: missing or unparsable install-root avatar.opa");
        return;
    };
    println!(
        "connect::read_avatar_source (139 MiB read + table): {:?}",
        source.elapsed()
    );
    for attempt in 1..=3 {
        time(&format!("parse_opi {attempt}"), || {
            o2jam_file::parse_opi(&archive).expect("avatar.opa parses");
        });
        time(&format!("load_worn_figure {attempt}"), || {
            let figure = o2jam_shell::avatar::load_worn_figure(&archive, &table, &[1], false);
            assert!(!figure.is_empty(), "item 1 stacks from installed data");
        });
    }
    let staging = Instant::now();
    let mut game = staged_shop_game(archive.clone(), table.clone());
    println!(
        "boot staging (art + avatar source): {:?}",
        staging.elapsed()
    );
    let hero = bench_character();
    let mut identity_game = ScreenGame::new();
    for attempt in 1..=2 {
        time(&format!("stage_online_identity {attempt}"), || {
            crate::stage_online_identity(&mut identity_game, Some(&hero));
        });
    }
    for attempt in 1..=4 {
        let opened = time(&format!("open {attempt}"), || {
            assert!(game.visit_side(SideScreen::Shop));
        });
        let left = time(&format!("leave {attempt}"), || {
            assert!(game.leave_side());
        });
        println!("cycle {attempt}: open {opened:?} + leave {left:?}");
    }
}

/// The shop board's cost is paid before the click: boot stages the
/// avatar source, then each channel login stages it again before the
/// player can open the shop. Both landings must reuse one decoded
/// source; the open itself must not restart archive work.
#[test]
fn second_identity_landing_and_shop_open_stay_under_budget() {
    // Live caller chain:
    //   main.rs::run_play -> stage_online_identity (boot)
    //   -> session.rs::resolve_channel_login -> stage_online_identity
    //   -> ScreenGame::set_decoded_avatar_source -> visit_side(Shop)
    //   -> ScreenGame::restage_shop_avatar.
    if !avatar_path().is_file() {
        println!("skip: missing install-root avatar.opa");
        return;
    }
    let hero = bench_character();
    let mut boot = ScreenGame::new();
    crate::stage_online_identity(&mut boot, Some(&hero));

    let mut game = ScreenGame::new();
    let started = Instant::now();
    crate::stage_online_identity(&mut game, Some(&hero));
    walk_to_channel(&mut game);
    assert!(game.visit_side(SideScreen::Shop));
    let elapsed = started.elapsed();
    println!("second landing + open: {elapsed:?} (budget {SECOND_LANDING_AND_OPEN_BUDGET:?})");
    assert!(
        elapsed <= SECOND_LANDING_AND_OPEN_BUDGET,
        "repeated identity staging must reuse the decoded avatar source"
    );
}
