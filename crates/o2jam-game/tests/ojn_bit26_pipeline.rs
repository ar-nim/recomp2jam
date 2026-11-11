//! Synthetic bit-26 chart through the implemented pipeline
//! (claim B50, issue #83): `o2jam-file` decode into
//! `o2jam-game::build_level_chart`, proving the flag survives
//! end-to-end and naming what the current decoder maps it to.

use o2jam_file::parse_ojn_level_events;
use o2jam_game::{build_level_chart, HoldSpan};

/// Build a minimal old-format `.ojn` image (Tutorial-shaped: one
/// populated Easy slice, two empty trailing slices) holding the given
/// framed level-0 events.
fn synthetic_chart(level0_events: &[u8], level0_event_count: u32) -> Vec<u8> {
    const HEADER_LEN: usize = 300;
    let mut blob = vec![0u8; HEADER_LEN];
    blob[0..4].copy_from_slice(&8888u32.to_le_bytes());
    blob[4..8].copy_from_slice(b"ojn\0");
    blob[8..12].copy_from_slice(&2.7f32.to_le_bytes());
    blob[16..20].copy_from_slice(&100f32.to_le_bytes());
    // total/playable counts are not asserted by the pipeline; the
    // slice geometry is what decode validates.
    blob[64..68].copy_from_slice(&level0_event_count.to_le_bytes());
    blob[284..288].copy_from_slice(&(HEADER_LEN as u32).to_le_bytes());
    let end0 = HEADER_LEN + level0_events.len();
    for off in [288usize, 292, 296] {
        blob[off..off + 4].copy_from_slice(&(end0 as u32).to_le_bytes());
    }
    blob.extend_from_slice(level0_events);
    blob
}

fn framed_event(measure: i32, kind: u16, payload: &[u32]) -> Vec<u8> {
    let mut event = Vec::with_capacity(8 + payload.len() * 4);
    event.extend_from_slice(&measure.to_le_bytes());
    event.extend_from_slice(&kind.to_le_bytes());
    event.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    for unit in payload {
        event.extend_from_slice(&unit.to_le_bytes());
    }
    event
}

/// A synthetic bit-26 lane note must decode with the flag preserved
/// in `TimedHit::flags`, classify as neither hold head nor tail, and
/// therefore never arm a judge arm — the exe-side sweep verdict
/// (docs/gameplay.md payload-bit rules) holds through our pipeline.
#[test]
fn synthetic_bit26_lane_note_is_preserved_and_judge_invisible() {
    // flags word 0x0400 (payload bit 26), sample id 0.
    const BIT26_LANE: u32 = 0x0400_0001;
    const PLAIN_TAP: u32 = 0x0000_0001;
    const BIT26_BGM: u32 = 0x0400_0002;
    let level0 = [
        framed_event(0, 2, &[BIT26_LANE]),
        framed_event(4, 3, &[PLAIN_TAP]),
        framed_event(8, 9, &[BIT26_BGM]),
    ]
    .concat();
    let chart = synthetic_chart(&level0, 3);

    let events = parse_ojn_level_events(&chart, 0).expect("decode level 0");
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].values[0], BIT26_LANE);

    let level = build_level_chart(100.0, &events);
    assert_eq!(level.hits.len(), 2, "one lane note per kind-2/3 event");
    let flagged = level
        .hits
        .iter()
        .find(|hit| hit.flags != 0)
        .expect("the bit-26 hit keeps its flags");
    assert_eq!(flagged.flags, 0x0400, "bit 26 survives as flags bit 10");
    assert_eq!(flagged.sample_id, 0);
    for hit in &level.hits {
        let payload = (u32::from(hit.flags) << 16) | (hit.sample_id + 1);
        assert!(!HoldSpan::marks_hold_head(payload));
        assert!(!HoldSpan::marks_hold_tail(payload));
    }
    let backing = &level.backing;
    assert_eq!(backing.len(), 1, "the kind-9 event is backing");
    assert_eq!(backing[0].flags, 0x0400);
    assert_eq!(backing[0].sample_id, 1);
}
