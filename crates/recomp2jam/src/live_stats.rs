//! Live stats bridge: 4015 frames into the peer mirror, 4014 ticks out.
//!
//! The gateway leg (`o2jam-net::play`) carries raw bodies; the mirror
//! (`o2jam-game::PeerBars`) holds per-slot Life/Jam plus ranks. This
//! module is the only place that translates between them, and the only
//! place that builds outgoing ticks from live score state, so the CLI
//! online flow never spreads wire/mirror knowledge.

use o2jam_game::{PeerBars, QueueEvent, RankDelivery};
use o2jam_net::{GameStatsUpdate, UpdateGameStats};

/// One applied 4015 echo: the stats-queue forward plus rank deliveries.
#[derive(Debug)]
pub struct AppliedPeerUpdate {
    /// `(0x12, type, slot, 0)` forward; `None` on unknown channel/slot.
    pub queue: Option<QueueEvent>,
    /// Per-position deliveries under the rank-remap rule.
    pub ranks: Vec<RankDelivery>,
}

/// Apply one decoded 4015 frame to the mirror: the stats write first,
/// then the 8-byte rank broadcast (which applies even when the channel
/// word is unknown).
pub fn apply_peer_update(peers: &mut PeerBars, update: GameStatsUpdate) -> AppliedPeerUpdate {
    let queue = peers.apply_stats_update(update.member_id, update.kind_code, update.value);
    let ranks = peers.apply_rank_broadcast(update.rank_packet());
    AppliedPeerUpdate { queue, ranks }
}

/// Saturate a running total onto the wire (PERF-07: `TryFrom`, no `as`).
fn saturate_score(points: u64) -> u32 {
    u32::try_from(points).unwrap_or(u32::MAX)
}

/// Saturate a jam level onto its channel word (PERF-07: no `as`).
fn saturate_jam(jam_level: u32) -> u16 {
    u16::try_from(jam_level).unwrap_or(u16::MAX)
}

/// Jam tick from score state: channel value is the jam level.
#[must_use]
pub fn jam_tick_for_score(
    points: u64,
    jam_level: u32,
    sequence: i32,
    long_note_score: i32,
) -> UpdateGameStats {
    UpdateGameStats::jam_tick(
        saturate_jam(jam_level),
        sequence,
        saturate_score(points),
        long_note_score,
    )
}

/// Life tick from score state: life rides alongside the running total.
#[must_use]
pub fn life_tick_for_score(
    life: u16,
    points: u64,
    sequence: i32,
    long_note_score: i32,
) -> UpdateGameStats {
    UpdateGameStats::life_tick(life, sequence, saturate_score(points), long_note_score)
}

/// Jam tick from a live play session's score state.
///
/// Hold bonus stays zero: `LivePlay` tracks no long-note accumulator,
/// so there is nothing honest to report yet.
#[allow(
    dead_code,
    reason = "bridge API for the windowed live path; tested below"
)]
#[must_use]
pub fn jam_tick_for_live_play(play: &o2jam_shell::LivePlay, sequence: i32) -> UpdateGameStats {
    jam_tick_for_score(play.score().points, play.score().jam.level(), sequence, 0)
}

/// Life tick from a live play session's score state.
///
/// Hold bonus stays zero, as in [`jam_tick_for_live_play`].
#[allow(
    dead_code,
    reason = "bridge API for the windowed live path; tested below"
)]
#[must_use]
pub fn life_tick_for_live_play(
    play: &o2jam_shell::LivePlay,
    life: u16,
    sequence: i32,
) -> UpdateGameStats {
    life_tick_for_score(life, play.score().points, sequence, 0)
}

#[cfg(test)]
mod live_stats_tests {
    use o2jam_game::PEER_SLOT_COUNT;
    use o2jam_net::{StatKind, WireMessage};

    use super::*;

    fn life_echo() -> GameStatsUpdate {
        GameStatsUpdate {
            member_id: 2,
            kind_code: StatKind::Life.as_u16(),
            value: 1000,
            ranks: [10, 11, 12, 13, 14, 15, 16, 17],
        }
    }

    #[test]
    fn applies_life_echo_to_mirror_bars_and_ranks() {
        let mut peers = PeerBars::new();
        peers.set_global_rank(0xFF);
        for slot in 0..PEER_SLOT_COUNT {
            let rank = u8::try_from(slot).unwrap_or(u8::MAX).saturating_add(10);
            peers.set_stored_rank(slot, rank);
        }
        let applied = apply_peer_update(&mut peers, life_echo());
        let queue = applied.queue.expect("life echo forwards a queue event");
        assert_eq!(queue.slot, 2);
        assert_eq!(queue.stat_kind, StatKind::Life.as_u16());
        assert_eq!(peers.life(2), Some(1000));
        assert_eq!(peers.jam(2), Some(0));
        assert_eq!(applied.ranks.len(), PEER_SLOT_COUNT);
        assert_eq!(peers.rank_row(0), Some(10));
        assert_eq!(peers.rank_row(7), Some(17));
    }

    #[test]
    fn applies_jam_echo_to_mirror_jam_word() {
        let mut peers = PeerBars::new();
        let update = GameStatsUpdate {
            member_id: 1,
            kind_code: StatKind::Jam.as_u16(),
            value: 42,
            ranks: [0; 8],
        };
        let applied = apply_peer_update(&mut peers, update);
        assert_eq!(peers.jam(1), Some(42));
        assert_eq!(peers.life(1), Some(0));
        assert!(applied.queue.is_some());
        assert_eq!(applied.ranks.len(), PEER_SLOT_COUNT);
    }

    #[test]
    fn unknown_channel_stores_nothing_but_still_broadcasts_ranks() {
        let mut peers = PeerBars::new();
        let update = GameStatsUpdate {
            member_id: 1,
            kind_code: 9,
            value: 7,
            ranks: [0; 8],
        };
        let applied = apply_peer_update(&mut peers, update);
        assert_eq!(applied.queue, None);
        assert_eq!(peers.life(1), Some(0));
        assert_eq!(peers.jam(1), Some(0));
        assert_eq!(applied.ranks.len(), PEER_SLOT_COUNT);
        assert_eq!(peers.rank_row(0), Some(0));
    }

    #[test]
    fn out_of_range_slot_forwards_nothing_but_keeps_ranks() {
        let mut peers = PeerBars::new();
        let update = GameStatsUpdate {
            member_id: 8,
            kind_code: StatKind::Life.as_u16(),
            value: 500,
            ranks: [0; 8],
        };
        let applied = apply_peer_update(&mut peers, update);
        assert_eq!(applied.queue, None);
        assert_eq!(peers.life_levels(), [0; PEER_SLOT_COUNT]);
        assert_eq!(applied.ranks.len(), PEER_SLOT_COUNT);
    }

    #[test]
    fn jam_tick_for_live_play_uses_score_and_jam_level() {
        let mut play = o2jam_shell::LivePlay::new(&[(0, 1000, 7)]);
        play.strike_lane(0, 1000);
        let tick = jam_tick_for_live_play(&play, 1);
        assert_eq!(tick.kind, StatKind::Jam);
        assert_eq!(tick.value, 0);
        assert_eq!(tick.score, 200);
        assert_eq!(tick.sequence, 1);
        let frame = o2jam_net::frame::decode_frame(&tick.encode()).unwrap();
        assert_eq!(frame.id, 4014);
    }

    #[test]
    fn life_tick_for_live_play_carries_life_and_score() {
        let mut play = o2jam_shell::LivePlay::new(&[(0, 1000, 7)]);
        play.strike_lane(0, 1000);
        let tick = life_tick_for_live_play(&play, 1000, 2);
        assert_eq!(tick.kind, StatKind::Life);
        assert_eq!(tick.value, 1000);
        assert_eq!(tick.score, 200);
    }

    #[test]
    fn score_ticks_saturate_instead_of_wrapping() {
        let tick = jam_tick_for_score(u64::from(u32::MAX) + 1, u32::from(u16::MAX) + 1, 0, 0);
        assert_eq!(tick.score, u32::MAX);
        assert_eq!(tick.value, u16::MAX);
    }
}
