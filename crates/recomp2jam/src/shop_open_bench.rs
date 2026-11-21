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

fn install_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

fn avatar_path() -> PathBuf {
    install_root().join(AVATAR_ARCHIVE)
}

/// Stage the live boot's shop prerequisites from installed data:
/// scene art, channel lobby, character, then the avatar source.
fn staged_shop_game(archive: Vec<u8>, table: o2jam_file::AvatarItemTable) -> ScreenGame {
    let mut game = ScreenGame::new();
    time("stage_live_scenes", || {
        crate::live_art::stage_live_scenes(&mut game)
    });
    // The splash-to-channel walk a player takes before the shop click.
    game.menu_key(MenuKey::Back, true);
    game.click_planet(136, 88);
    assert_eq!(game.screen(), o2jam_shell::Screen::Channel);
    game.set_myroom_character(CharacterInfo {
        equipment: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ..CharacterInfo::default()
    });
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
    let mut identity_game = ScreenGame::new();
    for attempt in 1..=2 {
        time(&format!("stage_online_identity {attempt}"), || {
            crate::stage_online_identity(&mut identity_game, None);
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
