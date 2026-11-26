//! Item Shop open guards (#755, #862).
//!
//! The normal-suite guard pins decoded-source identity through a forced
//! cache eviction; it never asserts wall-clock time. The 75 ms budget
//! lives in the ignored release profile, run alone so full-suite
//! parallelism and concurrent lane builds cannot flake it:
//! `cargo test --release --bin recomp2jam shop_open_profile -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use o2jam_net::CharacterInfo;
use o2jam_shell::screens::SideScreen;
use o2jam_shell::{AvatarSource, GameLoop, MenuKey, ScreenGame};

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

/// The profile carries the wall-clock budget. It is `#[ignore]`d in the
/// normal suite: an explicit release run is a single measurement window,
/// not one test among parallel tests and concurrent lane builds.
#[test]
#[ignore = "measurement harness; run in release with --ignored --nocapture"]
fn shop_open_profile_records_stage_durations() {
    let source = Instant::now();
    let Some((archive, table)) = read_avatar_source() else {
        println!("SKIP shop_open_profile_records_stage_durations: missing or unparsable install-root avatar.opa");
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

/// Stage identity from a caller-held decode and open the shop. This is
/// the #862 seam: only the archive lookup/decode is replaced; the
/// character, catalog, source staging, and shop click stay on the live
/// `stage_online_identity` path.
fn staged_identity_and_shop(hero: &CharacterInfo, source: &Arc<AvatarSource>) -> ScreenGame {
    let mut game = ScreenGame::new();
    crate::stage_online_identity_with_decoded_avatar(&mut game, Some(hero), source);
    walk_to_channel(&mut game);
    assert!(game.visit_side(SideScreen::Shop));
    game
}

fn assert_staged_source(game: &ScreenGame, expected: &Arc<AvatarSource>, attempt: u32) {
    let staged = game
        .staged_avatar_source()
        .unwrap_or_else(|| panic!("attempt {attempt} staged no avatar source"));
    assert!(
        Arc::ptr_eq(staged, expected),
        "attempt {attempt} must reuse the injected decoded avatar source"
    );
}

/// The shop board's cost is paid before the click: boot stages the
/// avatar source, then each channel login stages it again before the
/// player can open the shop. Both landings must reuse one decoded
/// source; the open itself must not restart archive work.
///
/// Live caller chain:
///   main.rs::run_play -> stage_online_identity (boot)
///   -> session.rs::resolve_channel_login -> stage_online_identity
///   -> ScreenGame::set_decoded_avatar_source -> visit_side(Shop).
/// The seam above injects only the decoded source; clearing the shared
/// cache between attempts proves reuse does not depend on that cache.
#[test]
fn repeated_identity_staging_survives_forced_avatar_cache_eviction() {
    if !avatar_path().is_file() {
        println!("SKIP repeated_identity_staging_survives_forced_avatar_cache_eviction: missing install-root avatar.opa");
        return;
    }
    let hero = bench_character();
    let source =
        o2jam_shell::read_installed_avatar(&install_root()).expect("installed avatar decodes");

    let first = staged_identity_and_shop(&hero, &source);
    o2jam_shell::clear_decoded_avatar_cache();
    let second = staged_identity_and_shop(&hero, &source);

    let reloaded = o2jam_shell::read_installed_avatar(&install_root())
        .expect("installed avatar decodes after eviction");
    assert!(
        !Arc::ptr_eq(&source, &reloaded),
        "forced eviction must produce a fresh cached decode"
    );
    assert_staged_source(&first, &source, 1);
    assert_staged_source(&second, &source, 2);
}

/// Wall-clock leg: kept out of the parallel suite, run only in the
/// ignored release profile above. The 75 ms number still measures the
/// live second landing plus shop open with real 7.55 data.
#[test]
#[ignore = "timing budget; run alone in the release shop_open_profile profile"]
fn second_identity_landing_and_shop_open_stay_under_budget() {
    // Live caller chain:
    //   main.rs::run_play -> stage_online_identity (boot)
    //   -> session.rs::resolve_channel_login -> stage_online_identity
    //   -> ScreenGame::set_decoded_avatar_source -> visit_side(Shop)
    //   -> ScreenGame::restage_shop_avatar.
    if !avatar_path().is_file() {
        println!("SKIP second_identity_landing_and_shop_open_stay_under_budget: missing install-root avatar.opa");
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
