//! Room-state "no response" fuse (ticket #367).
//!
//! The 7.55 watchdog walks a tracked-send node queue every 5 s and
//! posts a one-OK Information sheet when a node is more than 20,000 ms
//! overdue (`docs/captures/static-367/pin.md`, shell observations
//! `S367-1`/`S367-3`; live node pin in issue #367 comment 5). Our
//! client has no 36 s relay connect block, so the arm is our own
//! 07DA/07DB poll pair on a boot that observed the relay down. This
//! module owns that wall-clock node and the sheet's type-100
//! dismissal queue; the session owns the socket and the game owns
//! the paint/hit tests. Each queued dismissal pops exactly one
//! layer from the open dialog stack (#689), so a fuse over another
//! sheet leaves that sheet up after one OK.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// The exe's overdue literal (20,000 ms at VA `0x558f6b`, file
/// `0x158f6b`): a node is late when elapsed is **strictly greater**.
pub const NO_RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);

/// Source of the monotonic instant the fuse stamps and checks.
/// Production uses [`WallClock`]; guards install a virtual instant.
pub trait NowSource {
    /// Current instant on this source.
    fn now(&self) -> Instant;
}

/// Production source: the real monotonic clock.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WallClock;

impl NowSource for WallClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// The fuse's injection seam: real time, or a fixed instant that
/// only moves when a guard moves it (#743). A virtual clock lets a
/// test wait for a wire write before the 20 s deadline can fire.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FuseClock {
    /// Production: read [`WallClock`] on every stamp and pump.
    #[default]
    Wall,
    /// Guard-controlled instant; the deadline never moves on its own.
    Virtual(Instant),
}

impl FuseClock {
    /// Install a virtual clock at `now`.
    #[must_use]
    pub const fn virtual_at(now: Instant) -> Self {
        Self::Virtual(now)
    }
}

impl NowSource for FuseClock {
    fn now(&self) -> Instant {
        match self {
            Self::Wall => WallClock.now(),
            Self::Virtual(now) => *now,
        }
    }
}
/// Message-queue type the one-OK sheet's OK, Enter, Escape, Space,
/// and mouse release all post (VA `0x433f78`, file `0x33f78`).
pub const DISMISS_TOP_DIALOG_QUEUE_TYPE: u16 = 100;

/// One layer of the open dialog stack, bottom first. The type-100
/// pump disposes of the top layer only; the game owns the state
/// each lower layer names (#689).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DialogLayer {
    /// Room's create/password form.
    RoomSheet,
    /// A planet-board modal (quit confirm, options, or a notice).
    PlanetSheet(super::PlanetModal),
    /// The one-OK no-response sheet; top while `sheet_open` holds.
    FuseNotice,
}

/// Expected answer of the poll that arms the fuse (`07DB`).
const EXPECTED_USER_LIST_REPLY: u16 = o2jam_net::Response::GetUserList as u16;

/// One tracked-send node: `{expected id, stamp}` in the exe's queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrackedUserListPoll {
    expected_reply: u16,
    sent_at: Instant,
}

/// Fuse state carried by `ScreenGame`. The relay observation comes
/// from boot; the tracked node comes from the session's Room poll.
#[derive(Debug, Default)]
pub(super) struct RoomNoResponseFuse {
    clock: FuseClock,
    relay_down_observed: bool,
    poll: Option<TrackedUserListPoll>,
    sheet_open: bool,
    dismissal_queue: VecDeque<u16>,
    dialog_stack: Vec<DialogLayer>,
}

impl RoomNoResponseFuse {
    /// Install a clock source. Production never leaves [`FuseClock::Wall`].
    pub(super) fn set_clock(&mut self, clock: FuseClock) {
        self.clock = clock;
    }

    /// Carry the boot relay probe into Room state (#348 boundary).
    pub(super) fn set_relay_down_observed(&mut self, relay_down: bool) {
        self.relay_down_observed = relay_down;
    }

    /// `true` only in Room after a down relay was observed at boot.
    #[must_use]
    pub(super) fn enabled(&self, in_room: bool) -> bool {
        self.relay_down_observed && in_room
    }

    /// Stamp the node created when our own 07DA goes on the wire.
    /// One node at a time, exactly like the exe's single surviving
    /// node at `[state+0x4348]`.
    pub(super) fn track_send(&mut self, in_room: bool) -> bool {
        self.track_send_at(in_room, self.clock.now())
    }

    /// Stamp a node at an explicit instant (the wall-clock gate leg).
    fn track_send_at(&mut self, in_room: bool, now: Instant) -> bool {
        if !self.enabled(in_room) || self.poll.is_some() || self.sheet_open {
            return false;
        }
        self.poll = Some(TrackedUserListPoll {
            expected_reply: EXPECTED_USER_LIST_REPLY,
            sent_at: now,
        });
        true
    }

    /// A processed 07DB erases the node before the watchdog can fire.
    pub(super) fn track_reply(&mut self) -> bool {
        self.poll.take().is_some()
    }

    #[must_use]
    pub(super) const fn poll_tracked(&self) -> bool {
        self.poll.is_some()
    }

    /// Walk the node queue. On the first overdue pass, erase the
    /// node and stage one sheet; a fresh node can arm a later modal.
    pub(super) fn pump(&mut self, in_room: bool, covered: &[DialogLayer]) -> bool {
        self.pump_at(in_room, self.clock.now(), covered)
    }

    /// Watchdog pass at an explicit instant (the wall-clock gate leg).
    fn pump_at(&mut self, in_room: bool, now: Instant, covered: &[DialogLayer]) -> bool {
        if !self.enabled(in_room) || self.sheet_open {
            return false;
        }
        let overdue = self.poll.is_some_and(|poll| {
            poll.expected_reply == EXPECTED_USER_LIST_REPLY
                && now.saturating_duration_since(poll.sent_at) > NO_RESPONSE_TIMEOUT
        });
        if !overdue {
            return false;
        }
        self.poll = None;
        self.sheet_open = true;
        self.dialog_stack.clear();
        self.dialog_stack.extend_from_slice(covered);
        self.dialog_stack.push(DialogLayer::FuseNotice);
        true
    }

    #[must_use]
    pub(super) const fn sheet_open(&self) -> bool {
        self.sheet_open
    }

    /// OK posts `(100, 0, NULL, 0)`; the posted record, not the
    /// handler, closes the dialog (static pin `S367-2`).
    pub(super) fn post_dismiss_top(&mut self) {
        self.dismissal_queue
            .push_back(DISMISS_TOP_DIALOG_QUEUE_TYPE);
    }

    #[must_use]
    pub(super) fn queued_dismissal(&self) -> Option<u16> {
        self.dismissal_queue.front().copied()
    }

    /// Pump one queued type-100 record: pop the top dialog layer.
    /// Only the fuse layer clears `sheet_open`; a lower layer stays
    /// owned by the game until its own record reaches the pump.
    pub(super) fn take_dismissal(&mut self) -> Option<DialogLayer> {
        if self.queued_dismissal() != Some(DISMISS_TOP_DIALOG_QUEUE_TYPE) {
            return None;
        }
        self.dismissal_queue.pop_front();
        let layer = self.dialog_stack.pop()?;
        if layer == DialogLayer::FuseNotice {
            self.sheet_open = false;
        }
        Some(layer)
    }

    /// Leaving Room drops the node and any open sheet, but keeps the
    /// boot relay observation for the next Room visit.
    pub(super) fn clear(&mut self) {
        self.poll = None;
        self.sheet_open = false;
        self.dismissal_queue.clear();
        self.dialog_stack.clear();
    }
}

#[cfg(test)]
mod room_fuse_tests {
    use super::{
        DialogLayer, FuseClock, NowSource, RoomNoResponseFuse, WallClock,
        DISMISS_TOP_DIALOG_QUEUE_TYPE, NO_RESPONSE_TIMEOUT,
    };
    use std::time::{Duration, Instant};

    /// A fuse with a node stamped at the guard's virtual instant.
    fn fuse_stamped_at(now: Instant) -> RoomNoResponseFuse {
        let mut fuse = RoomNoResponseFuse::default();
        fuse.set_clock(FuseClock::virtual_at(now));
        fuse.set_relay_down_observed(true);
        assert!(fuse.track_send(true));
        fuse
    }

    /// The one wall-clock leg: production's [`WallClock`] reads a real
    /// `Instant`, and the exe's overdue test is strictly greater than
    /// 20 s (exact 20 s leaves the node pending). Every other fuse
    /// guard drives [`FuseClock::Virtual`] instead (#743).
    #[test]
    fn fuse_uses_a_strict_twenty_second_wall_clock_gate() {
        let mut fuse = RoomNoResponseFuse::default();
        assert_eq!(fuse.clock, FuseClock::Wall);
        fuse.set_relay_down_observed(true);
        let sent_at = WallClock.now();
        assert!(fuse.track_send_at(true, sent_at));
        assert!(
            !fuse.pump(true, &[]),
            "a fresh node is not overdue on the real clock"
        );
        assert!(!fuse.pump_at(true, sent_at + NO_RESPONSE_TIMEOUT, &[]));
        assert!(fuse.pump_at(
            true,
            sent_at + NO_RESPONSE_TIMEOUT + Duration::from_millis(1),
            &[]
        ));
        assert!(!fuse.poll_tracked(), "the overdue node is erased");
    }

    /// A virtual deadline never moves by itself: the sheet opens only
    /// after the guard advances the injected instant past 20 s.
    #[test]
    fn fuse_waits_for_the_virtual_deadline_before_staging_a_sheet() {
        let sent_at = Instant::now();
        let mut fuse = fuse_stamped_at(sent_at);

        assert!(!fuse.pump(true, &[]), "the guard has not advanced time");
        fuse.set_clock(FuseClock::virtual_at(
            sent_at + NO_RESPONSE_TIMEOUT + Duration::from_millis(1),
        ));
        assert!(fuse.pump(true, &[]));
        assert!(fuse.sheet_open());
    }

    /// Relay-down alone never stages; without the Room state the
    /// watchdog has no sheet to paint, whatever the virtual clock reads.
    #[test]
    fn fuse_ignores_a_starved_node_outside_room() {
        let mut fuse = RoomNoResponseFuse::default();
        let now = Instant::now();
        assert!(!fuse.track_send(true), "no relay observation yet");
        fuse.set_clock(FuseClock::virtual_at(now));
        fuse.set_relay_down_observed(true);
        assert!(!fuse.track_send(false), "no node outside Room");
        fuse.set_clock(FuseClock::virtual_at(now + Duration::from_secs(21)));
        assert!(!fuse.pump(false, &[]));
        assert!(!fuse.sheet_open());
    }

    /// OK's entire effect is one queued dismissal; the queue type is
    /// pinned, and no second pump finds a record to close.
    #[test]
    fn dismissal_queue_carries_only_type_100() {
        let sent_at = Instant::now();
        let mut fuse = fuse_stamped_at(sent_at);
        fuse.set_clock(FuseClock::virtual_at(sent_at + Duration::from_secs(21)));
        assert!(fuse.pump(true, &[]));
        fuse.post_dismiss_top();
        assert_eq!(fuse.queued_dismissal(), Some(DISMISS_TOP_DIALOG_QUEUE_TYPE));
        assert_eq!(fuse.take_dismissal(), Some(DialogLayer::FuseNotice));
        assert!(!fuse.sheet_open());
        assert_eq!(fuse.take_dismissal(), None);
    }

    /// seam: the dismissal order is the whole behavior here; the
    /// live two-tap chain is guarded in `recomp2jam::session_tests`.
    #[test]
    fn type_100_records_pop_the_fuse_before_its_covered_sheet() {
        let sent_at = Instant::now();
        let mut fuse = fuse_stamped_at(sent_at);
        fuse.set_clock(FuseClock::virtual_at(sent_at + Duration::from_secs(21)));
        assert!(fuse.pump(true, &[DialogLayer::RoomSheet]));
        fuse.post_dismiss_top();
        fuse.post_dismiss_top();

        assert_eq!(fuse.take_dismissal(), Some(DialogLayer::FuseNotice));
        assert_eq!(fuse.take_dismissal(), Some(DialogLayer::RoomSheet));
        assert_eq!(fuse.take_dismissal(), None);
    }
}
