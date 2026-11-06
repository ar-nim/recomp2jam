//! Peer lifebar + rank mirror for S→C `GameStatsUpdate` (4015, 7.55).
//!
//! Behavior paraphrased from the executable (never original bytes); each
//! rule names its record in the `rea-4015-ghidra` dossier bundle:
//! - Dispatch: the central S→C switch has an explicit `0xFAF` arm that
//!   forwards the packet to `0x55fa20` and nothing else (`ev_2103360b…`).
//! - Prologue: u8 slot plus u16 discriminator via helper `0x555d20`,
//!   early return unless slot `< 8` (`ev_c3332c95…`).
//! - Branches: discriminator `0` (Life) calls setter `0x5410e0` then the
//!   queue; `1` (Jam) calls setter `0x5410c0` then the queue
//!   (`ev_b21026fd…`).
//! - Setters: Life writes 2 bytes at member sub-object `+0x2150` (game
//!   `+0x3970`); Jam writes 2 bytes at sub-object `+0x214e` (game
//!   `+0x396e`) (`ev_50f72f57…`).
//! - Queue: both paths forward `(0x12, type, slot, 0)` into `0x478d50`
//!   (`ev_50d2917f…`).
//! - Rank loop: 8 outer packet bytes × 8 inner member slots at stride
//!   `0x2190` onto base `0x77ee90`; each byte is delivered (notify
//!   `0x5414a0` with the outer position) to whichever slot holds that
//!   rank — the shared global byte first, then member bytes, first match
//!   wins and the inner scan stops (`ev_0ff34c44…`).

/// Peer slots in the 8-seat member table (stride `0x2190` onto `0x77ee90`).
pub const PEER_SLOT_COUNT: usize = 8;
/// `GameStatsUpdate` discriminator for Life (`ev_b21026fd…`).
pub const STAT_LIFE: u16 = 0;
/// `GameStatsUpdate` discriminator for Jam (`ev_b21026fd…`).
pub const STAT_JAM: u16 = 1;
/// UI queue category both paths forward with (`ev_50d2917f…`).
pub const QUEUE_CATEGORY_PEER: u32 = 0x12;
/// Life clamp: the scorer compares the life word against `0x3e8` (file
/// `0x57094d`), and the mirrored server range is 0–1000.
pub const LIFE_MAX: u16 = 1000;

/// One UI event-queue forward: `(0x12, type, slot, 0)` (`ev_50d2917f…`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueEvent {
    /// Always [`QUEUE_CATEGORY_PEER`].
    pub category: u32,
    /// The packet discriminator ([`STAT_LIFE`] or [`STAT_JAM`]).
    pub stat_kind: u16,
    /// The packet slot index.
    pub slot: u8,
    /// Always zero (trailing flag word).
    pub flag: u32,
}

/// One rank-broadcast delivery: packet byte `rank` at outer `position`
/// reached the slot holding that rank (`ev_0ff34c44…`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankDelivery {
    /// Outer-loop position (0..8) whose notify fired.
    pub position: u8,
    /// Packet byte delivered there.
    pub rank: u8,
    /// Member slot holding the rank; `None` when the shared global byte
    /// matched first and no member was consulted.
    pub slot: Option<usize>,
}

/// In-song peer bars: per-slot Life/Jam words plus the rank bytes the
/// 8-byte broadcast resolves against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeerBars {
    life: [u16; PEER_SLOT_COUNT],
    jam: [u16; PEER_SLOT_COUNT],
    stored_rank: [u8; PEER_SLOT_COUNT],
    global_rank: u8,
    rank_rows: [u8; PEER_SLOT_COUNT],
}

impl PeerBars {
    /// Empty room: every bar zeroed, no ranks held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Life word of `slot`, or `None` when out of range.
    #[must_use]
    pub fn life(&self, slot: usize) -> Option<u16> {
        self.life.get(slot).copied()
    }

    /// Jam word of `slot`, or `None` when out of range.
    #[must_use]
    pub fn jam(&self, slot: usize) -> Option<u16> {
        self.jam.get(slot).copied()
    }

    /// Every life word, slot-ordered (paint snapshot).
    #[must_use]
    pub fn life_levels(&self) -> [u16; PEER_SLOT_COUNT] {
        self.life
    }

    /// Rank row shown at outer `position`, or `None` when out of range.
    #[must_use]
    pub fn rank_row(&self, position: usize) -> Option<u8> {
        self.rank_rows.get(position).copied()
    }

    /// Remember the rank byte member `slot` holds (out of range ignored).
    pub fn set_stored_rank(&mut self, slot: usize, rank: u8) {
        if let Some(held) = self.stored_rank.get_mut(slot) {
            *held = rank;
        }
    }

    /// Remember the shared global rank byte checked before members.
    pub fn set_global_rank(&mut self, rank: u8) {
        self.global_rank = rank;
    }

    /// Apply one `GameStatsUpdate` body: slot u8 plus Life-`0`/Jam-`1`
    /// u16 write to the member sub-object, then the queue forward.
    ///
    /// Guarded exactly like the exe (`ev_c3332c95…`, `ev_b21026fd…`):
    /// slot `>= 8` or a discriminator other than `0`/`1` stores nothing
    /// and forwards nothing.
    pub fn apply_stats_update(&mut self, slot: u8, kind: u16, value: u16) -> Option<QueueEvent> {
        let index = slot_index(slot)?;
        match kind {
            STAT_LIFE => self.life[index] = value,
            STAT_JAM => self.jam[index] = value,
            _ => return None,
        }
        Some(QueueEvent {
            category: QUEUE_CATEGORY_PEER,
            stat_kind: kind,
            slot,
            flag: 0,
        })
    }

    /// Apply the 8-byte rank broadcast: each packet byte is delivered to
    /// whichever slot holds that rank — global byte first, then member
    /// bytes, first match wins (`ev_0ff34c44…`). Unmatched bytes deliver
    /// nothing and leave their row alone.
    pub fn apply_rank_broadcast(&mut self, packet: [u8; PEER_SLOT_COUNT]) -> Vec<RankDelivery> {
        let mut deliveries = Vec::new();
        for (index, rank) in packet.iter().enumerate() {
            let Some(position) = position_of(index) else {
                continue;
            };
            if let Some(delivery) = self.deliver_rank(position, *rank) {
                deliveries.push(delivery);
            }
        }
        deliveries
    }

    /// Deliver one packet byte at `position`, recording its row.
    fn deliver_rank(&mut self, position: u8, rank: u8) -> Option<RankDelivery> {
        if rank == self.global_rank {
            return Some(self.record(position, rank, None));
        }
        let slot = self.match_slot(rank)?;
        Some(self.record(position, rank, Some(slot)))
    }

    /// Record a delivery in its row and describe it.
    fn record(&mut self, position: u8, rank: u8, slot: Option<usize>) -> RankDelivery {
        self.rank_rows[usize::from(position)] = rank;
        RankDelivery {
            position,
            rank,
            slot,
        }
    }

    /// First slot holding `rank` (inner-scan order, stop on match).
    fn match_slot(&self, rank: u8) -> Option<usize> {
        self.stored_rank.iter().position(|held| *held == rank)
    }
}

/// Member-table index for a packet slot (`< 8` guard, `ev_c3332c95…`).
fn slot_index(slot: u8) -> Option<usize> {
    let index = usize::from(slot);
    (index < PEER_SLOT_COUNT).then_some(index)
}

/// Outer-loop position for an enumerate index (always `< 8` here).
fn position_of(index: usize) -> Option<u8> {
    u8::try_from(index).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn life_write_stores_value_and_forwards_queue_event() {
        let mut peers = PeerBars::new();
        let event = peers.apply_stats_update(3, STAT_LIFE, 500);
        assert_eq!(
            event,
            Some(QueueEvent {
                category: QUEUE_CATEGORY_PEER,
                stat_kind: STAT_LIFE,
                slot: 3,
                flag: 0,
            })
        );
        assert_eq!(peers.life(3), Some(500));
    }

    #[test]
    fn jam_write_stores_value_and_forwards_queue_event() {
        let mut peers = PeerBars::new();
        let event = peers.apply_stats_update(1, STAT_JAM, 42);
        assert_eq!(event.map(|forward| forward.stat_kind), Some(STAT_JAM));
        assert_eq!(peers.jam(1), Some(42));
        assert_eq!(peers.life(1), Some(0));
    }

    #[test]
    fn out_of_range_slot_writes_nothing_and_forwards_nothing() {
        let mut peers = PeerBars::new();
        assert_eq!(peers.apply_stats_update(8, STAT_LIFE, 500), None);
        assert_eq!(peers.life_levels(), [0; PEER_SLOT_COUNT]);
    }

    #[test]
    fn unknown_discriminator_writes_nothing_and_forwards_nothing() {
        let mut peers = PeerBars::new();
        assert_eq!(peers.apply_stats_update(2, 2, 500), None);
        assert_eq!(peers.life(2), Some(0));
        assert_eq!(peers.jam(2), Some(0));
    }

    #[test]
    fn rank_broadcast_delivers_each_byte_to_its_rank_holder() {
        let mut peers = PeerBars::new();
        peers.set_global_rank(0xFF);
        for slot in 0..PEER_SLOT_COUNT {
            #[allow(clippy::cast_possible_truncation)]
            peers.set_stored_rank(slot, slot as u8 + 10);
        }
        let packet = [12, 10, 99, 11, 99, 13, 14, 15];
        let deliveries = peers.apply_rank_broadcast(packet);
        assert_eq!(
            deliveries,
            vec![
                RankDelivery {
                    position: 0,
                    rank: 12,
                    slot: Some(2),
                },
                RankDelivery {
                    position: 1,
                    rank: 10,
                    slot: Some(0),
                },
                RankDelivery {
                    position: 3,
                    rank: 11,
                    slot: Some(1),
                },
                RankDelivery {
                    position: 5,
                    rank: 13,
                    slot: Some(3),
                },
                RankDelivery {
                    position: 6,
                    rank: 14,
                    slot: Some(4),
                },
                RankDelivery {
                    position: 7,
                    rank: 15,
                    slot: Some(5),
                },
            ]
        );
        assert_eq!(peers.rank_row(0), Some(12));
        assert_eq!(peers.rank_row(1), Some(10));
        assert_eq!(peers.rank_row(2), Some(0));
    }

    #[test]
    fn rank_broadcast_prefers_the_global_byte_without_consulting_members() {
        let mut peers = PeerBars::new();
        peers.set_global_rank(7);
        peers.set_stored_rank(4, 7);
        let deliveries = peers.apply_rank_broadcast([7, 8, 8, 8, 8, 8, 8, 8]);
        assert_eq!(
            deliveries,
            vec![RankDelivery {
                position: 0,
                rank: 7,
                slot: None,
            }]
        );
    }

    #[test]
    fn rank_broadcast_first_member_match_wins() {
        let mut peers = PeerBars::new();
        peers.set_global_rank(0xFF);
        peers.set_stored_rank(2, 9);
        peers.set_stored_rank(5, 9);
        let deliveries = peers.apply_rank_broadcast([9, 9, 9, 9, 9, 9, 9, 9]);
        assert!(deliveries.iter().all(|found| found.slot == Some(2)));
        assert_eq!(deliveries.len(), PEER_SLOT_COUNT);
    }

    #[test]
    fn life_levels_snapshot_reports_every_slot() {
        let mut peers = PeerBars::new();
        peers.apply_stats_update(0, STAT_LIFE, LIFE_MAX);
        peers.apply_stats_update(7, STAT_LIFE, 250);
        let levels = peers.life_levels();
        assert_eq!((levels[0], levels[7]), (LIFE_MAX, 250));
        assert_eq!(peers.life(PEER_SLOT_COUNT), None);
        assert_eq!(peers.jam(PEER_SLOT_COUNT), None);
        assert_eq!(peers.rank_row(PEER_SLOT_COUNT), None);
    }
}
