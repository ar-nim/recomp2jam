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
//! the paint/hit tests.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// The exe's overdue literal (20,000 ms at VA `0x558f6b`, file
/// `0x158f6b`): a node is late when elapsed is **strictly greater**.
pub const NO_RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
/// Message-queue type the one-OK sheet's OK, Enter, Escape, Space,
/// and mouse release all post (VA `0x433f78`, file `0x33f78`).
pub const DISMISS_TOP_DIALOG_QUEUE_TYPE: u16 = 100;

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
    relay_down_observed: bool,
    poll: Option<TrackedUserListPoll>,
    sheet_open: bool,
    dismissal_queue: VecDeque<u16>,
}

impl RoomNoResponseFuse {
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
    pub(super) fn track_send(&mut self, in_room: bool, now: Instant) -> bool {
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
    pub(super) fn pump(&mut self, in_room: bool, now: Instant) -> bool {
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

    /// Pump one queued type-100 record: pop the top dialog.
    pub(super) fn take_dismissal(&mut self) -> bool {
        let dismiss = self.queued_dismissal() == Some(DISMISS_TOP_DIALOG_QUEUE_TYPE);
        if dismiss {
            self.dismissal_queue.pop_front();
            self.sheet_open = false;
        }
        dismiss
    }

    /// Leaving Room drops the node and any open sheet, but keeps the
    /// boot relay observation for the next Room visit.
    pub(super) fn clear(&mut self) {
        self.poll = None;
        self.sheet_open = false;
        self.dismissal_queue.clear();
    }
}

#[cfg(test)]
mod room_fuse_tests {
    use super::{RoomNoResponseFuse, DISMISS_TOP_DIALOG_QUEUE_TYPE, NO_RESPONSE_TIMEOUT};
    use std::time::{Duration, Instant};

    /// The exe's overdue test is strictly greater than 20 s; exactly
    /// 20 s still leaves the node pending and paints no sheet.
    #[test]
    fn fuse_uses_a_strict_twenty_second_wall_clock_gate() {
        let mut fuse = RoomNoResponseFuse::default();
        fuse.set_relay_down_observed(true);
        let now = Instant::now();
        assert!(fuse.track_send(true, now));
        assert!(!fuse.pump(true, now + NO_RESPONSE_TIMEOUT));
        assert!(fuse.pump(true, now + NO_RESPONSE_TIMEOUT + Duration::from_millis(1)));
        assert!(!fuse.poll_tracked(), "the overdue node is erased");
    }

    /// Relay-down alone never stages; without the Room state the
    /// watchdog has no sheet to paint.
    #[test]
    fn fuse_ignores_a_starved_node_outside_room() {
        let mut fuse = RoomNoResponseFuse::default();
        let now = Instant::now();
        assert!(!fuse.track_send(true, now), "no relay observation yet");
        fuse.set_relay_down_observed(true);
        assert!(!fuse.track_send(false, now), "no node outside Room");
        assert!(!fuse.pump(false, now + Duration::from_secs(21)));
        assert!(!fuse.sheet_open());
    }

    /// OK's entire effect is one queued dismissal; the queue type is
    /// pinned, and no second pump finds a record to close.
    #[test]
    fn dismissal_queue_carries_only_type_100() {
        let mut fuse = RoomNoResponseFuse::default();
        fuse.set_relay_down_observed(true);
        let now = Instant::now();
        assert!(fuse.track_send(true, now));
        assert!(fuse.pump(true, now + Duration::from_secs(21)));
        fuse.post_dismiss_top();
        assert_eq!(fuse.queued_dismissal(), Some(DISMISS_TOP_DIALOG_QUEUE_TYPE));
        assert!(fuse.take_dismissal());
        assert!(!fuse.sheet_open());
        assert!(!fuse.take_dismissal());
    }
}
