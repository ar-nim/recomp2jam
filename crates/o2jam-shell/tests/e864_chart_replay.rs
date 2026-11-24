//! E-864 (#653): the solo 7KEY replay reads its presses from the
//! shipped chart, not from the incidental 757 timeline cited by pack
//! `s8`. Every press enters through the portable input boundary the
//! window uses (`GameLoop::ingest_snapshot`), so the guard exercises
//! the same lane dispatch a real key press uses.
//!
//! The installed chart is local-only. With no `install-root` the test
//! skips cleanly: CI has no game data, and a skip is never a pass.

use std::path::PathBuf;

use o2jam_file::{parse_ojn, parse_ojn_level_events};
use o2jam_game::{build_level_chart, chart_replay::ChartReplay};
use o2jam_render::Frame;
use o2jam_shell::{
    screens::paint::LoadingStatus, GameLoop, InputSnapshot, MenuKey, Screen, ScreenGame,
};

const SONG_ID: u16 = 864;
/// Level 1 is the Normal slice: `o2ma864.ojn` header `+44` holds 554
/// playable notes and its second event slice starts at byte 28392.
const LEVEL: u8 = 1;

fn installed_chart() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Music")
        .join(format!("o2ma{SONG_ID}.ojn"))
}

/// Largest step that lands on every chart onset, so a fixed-clock
/// paint sequence reaches each press at its intended millisecond.
fn onset_step_ms(replay: &ChartReplay) -> u64 {
    replay
        .presses()
        .iter()
        .fold(0u64, |step, press| gcd(step, press.at_ms))
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

#[test]
fn e864_replay_judges_every_parsed_chart_note_through_the_lane_input_path() {
    // Live chain: window snapshot -> GameLoop::ingest_snapshot ->
    // ScreenGame::lane_key -> ScreenGame::strike_at ->
    // LivePlay::strike_lane -> Score::counts. The test enters at the
    // portable snapshot seam; OS key injection stays a rig leg.
    let chart_path = installed_chart();
    if !chart_path.is_file() {
        println!("skip: missing installed chart {}", chart_path.display());
        return;
    }
    let bytes = std::fs::read(&chart_path).expect("read shipped 864 chart");
    let header = parse_ojn(&bytes).expect("parse 864 header");
    assert_eq!(
        header.song_id,
        u32::from(SONG_ID),
        "installed chart is song 864"
    );
    let events = parse_ojn_level_events(&bytes, usize::from(LEVEL)).expect("parse level 1 events");
    let chart = build_level_chart(header.bpm, &events);
    let replay = ChartReplay::new(SONG_ID, LEVEL, &chart);
    assert_eq!(
        replay.note_count(),
        usize::try_from(header.playable_counts[usize::from(LEVEL)]).expect("playable count fits"),
        "replay owns the chart's own playable note count"
    );

    let step_ms = onset_step_ms(&replay);
    assert!(step_ms > 0, "chart onsets give a positive clock step");
    let mut game = ScreenGame::new();
    // Walk the documented boot path, not a direct screen assignment:
    // splash -> planet -> channel -> room -> waiting -> loading.
    game.menu_key(MenuKey::Back, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Loading);
    game.set_selected_music(o2jam_net::play::SetRoomMusic {
        music_id: SONG_ID,
        difficulty: LEVEL,
        speed: 0,
    });
    // The client loads the parsed chart; the replay only supplies keys.
    // Keeping these sources separate is what catches a borrowed timeline.
    game.load_chart(chart.hit_onsets_ms());
    game.with_audio(
        Box::new(o2jam_audio::NullBackend::new()),
        std::collections::HashMap::new(),
    );
    game.set_loading_status(LoadingStatus {
        progress: 1.0,
        title: "Dancing on the fire".to_owned(),
        artist: "DJ Schedule1".to_owned(),
    });
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Playing);
    game.enable_fixed_clock(step_ms);

    let mut pixels = vec![0u32; 16 * 16];
    let mut frame = Frame {
        buf: &mut pixels,
        w: 16,
        h: 16,
    };
    let mut applied_ms = 0u64;
    for press in replay.presses() {
        while applied_ms < press.at_ms {
            game.paint(&mut frame);
            applied_ms = applied_ms.saturating_add(step_ms);
        }
        assert_eq!(
            applied_ms, press.at_ms,
            "paint reaches the intended press time"
        );
        let before = game.live_play().score().counts[0];
        game.ingest_snapshot(&InputSnapshot::lane_key(usize::from(press.lane), true));
        let after = game.live_play().score().counts[0];
        assert_eq!(after, before + 1, "each chart press rates one Cool hit");
    }

    let counts = game.live_play().score().counts;
    assert_eq!(
        usize::try_from(counts.iter().sum::<u32>()).expect("judged total fits"),
        replay.note_count(),
        "every parsed 864 note is judged"
    );
    assert_eq!(
        usize::try_from(counts[0]).expect("cool count fits"),
        replay.note_count(),
        "chart-timed presses rate Cool, not borrowed 757 tallies"
    );
}
