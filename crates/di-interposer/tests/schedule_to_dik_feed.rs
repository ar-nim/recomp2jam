use di_interposer::{
    buffered_flags, build_feed, encode_feed, events_until, keyboard_state_at, parse_schedule,
    FeedEntry, HEADER_MAGIC, PRESSED_BIT, RECORD_BYTES,
};

const THREE_GROUPS: &str = include_str!("../../../fixtures/di-interposer/three-groups.txt");

fn fixture_feed() -> Vec<FeedEntry> {
    let schedule = parse_schedule(THREE_GROUPS).expect("synthetic fixture must parse");
    build_feed(&schedule, 1_000_000, 80)
}

#[test]
fn parse_schedule_reads_synthetic_chart_groups() {
    let schedule = parse_schedule(THREE_GROUPS).expect("synthetic fixture must parse");

    assert_eq!(schedule.groups.len(), 3);
    assert_eq!(schedule.groups[0].time_ms, 2400);
    assert_eq!(schedule.groups[0].diks, vec![0x1f, 0x20]);
    assert_eq!(schedule.groups[2].time_ms, 90240);
}

#[test]
fn parse_schedule_rejects_duplicate_dik_in_one_group() {
    let error = parse_schedule("2400:1f,1f\n").expect_err("duplicate DIK must fail");

    assert!(matches!(
        error,
        di_interposer::ScheduleError::DuplicateDik { dik: 0x1f, .. }
    ));
}

#[test]
fn build_feed_emits_press_before_release_for_each_dik() {
    let feed = fixture_feed();

    assert_eq!(
        feed[0],
        FeedEntry {
            time_ms: 1_002_400,
            dik: 0x1f,
            pressed: true
        }
    );
    assert_eq!(
        feed[1],
        FeedEntry {
            time_ms: 1_002_400,
            dik: 0x20,
            pressed: true
        }
    );
    assert_eq!(
        feed[2],
        FeedEntry {
            time_ms: 1_002_480,
            dik: 0x1f,
            pressed: false
        }
    );
}

#[test]
fn keyboard_state_at_sets_only_the_scheduled_dik_bytes() {
    let feed = fixture_feed();

    let state = keyboard_state_at(&feed, 1_002_400);

    assert_eq!(state[0x1f], PRESSED_BIT);
    assert_eq!(state[0x20], PRESSED_BIT);
    assert_eq!(state[0x26], 0);
    assert_eq!(state.iter().filter(|&&byte| byte == PRESSED_BIT).count(), 2);
}

#[test]
fn keyboard_state_at_clears_the_dik_after_hold() {
    let feed = fixture_feed();

    let state = keyboard_state_at(&feed, 1_002_480);

    assert_eq!(state[0x1f], 0);
    assert_eq!(state[0x20], 0);
}

#[test]
fn events_until_marks_a_press_with_the_exe_flag() {
    let feed = fixture_feed();

    let events = events_until(&feed, 1_000_000, 1_002_400);
    let flags: Vec<u8> = events.iter().copied().map(buffered_flags).collect();

    assert_eq!(flags, vec![PRESSED_BIT, PRESSED_BIT]);
}

#[test]
fn events_until_marks_a_release_without_the_exe_flag() {
    let feed = fixture_feed();

    let events = events_until(&feed, 1_002_400, 1_002_480);
    let flags: Vec<u8> = events.iter().copied().map(buffered_flags).collect();

    assert_eq!(flags, vec![0, 0]);
}

#[test]
fn encode_feed_writes_the_little_endian_header_and_first_record() {
    let feed = fixture_feed();

    let bytes = encode_feed(&feed).expect("synthetic feed fits the header");

    assert_eq!(&bytes[0..8], HEADER_MAGIC);
    assert_eq!(&bytes[8..16], &[1, 0, 0, 0, 12, 0, 0, 0]);
    assert_eq!(bytes.len(), 16 + 12 * RECORD_BYTES);
    assert_eq!(&bytes[16..24], &1_002_400_u64.to_le_bytes());
    assert_eq!(&bytes[24..28], &0x11f_u32.to_le_bytes());
}

#[test]
fn pinned_chart_feed_compiles_when_the_local_schedule_is_present() {
    let Ok(path) = std::env::var("DI_INTERPOSER_PINNED_SCHEDULE") else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };

    let schedule = parse_schedule(&text).expect("pinned schedule must parse");

    assert_eq!(schedule.groups.len(), 403);
    assert_eq!(schedule.groups[0].time_ms, 2400);
    assert_eq!(schedule.groups[0].diks, vec![0x26]);
    assert_eq!(schedule.groups.last().expect("403 groups").time_ms, 90240);
}
