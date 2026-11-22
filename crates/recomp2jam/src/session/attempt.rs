//! The single attempt module behind the seven wire legs.
//!
//! Every per-frame wire leg (channel login, room join, song select,
//! shop buy, music buy, gift claim) runs its send/drain
//! through [`OnlineSession::with_drain_timeout`], and the whole set
//! moves as one unit via [`OnlineSession::poll_attempts`]. The rules
//! that keep a silent gateway from freezing the frame loop live here
//! as code, not as comments:
//!
//! * **Timeout-restore**: a short drain window is set before the
//!   section and the resting session timeout is restored afterwards;
//!   a rejected prologue skips the section, a failed restore is traced.
//! * **Retry**: a leg that fails to send keeps its attempt staged so
//!   the next paint retries the *same* leg, never an earlier one.
//! * **Poll order**: the pending-wire drains run before the
//!   lobby/waiting drains, and while an attempt is in flight the
//!   lobby/waiting drains pause outright.

use super::*;
use std::time::Duration;

impl OnlineSession {
    /// Run all wire legs in their pinned order. Call sites use
    /// this seam instead of naming the individual polls.
    /// Current gateway read timeout (for guards).
    #[allow(dead_code)]
    pub(crate) fn gateway_read_timeout(&self) -> Option<Duration> {
        self.gateway.read_timeout().ok().flatten()
    }

    pub(crate) fn poll_attempts(&mut self) {
        self.flush_shop_buy();
        self.poll_shop_buy();
        self.flush_music_buy();
        self.poll_music_buy();
        self.flush_equip();
        self.poll_equip();
        self.flush_myroom_sends();
        self.poll_myroom_replies();
        self.flush_gift_claim();
        self.poll_gift_claim();
        self.poll_channel_login();
        self.poll_room_join();
        self.flush_create_room();
        self.flush_join();
        self.flush_music_send();
        self.poll_song_select();
    }

    /// Run one drain section under a short IO timeout and restore the
    /// resting session timeout afterwards — the timeout-restore rule
    /// every per-frame section shares. A rejected prologue traces and
    /// skips the section; a failed restore traces (the next section's
    /// prologue retries the resting set).
    pub(crate) fn with_drain_timeout(
        &mut self,
        short: Duration,
        name: &str,
        section: impl FnOnce(&mut Self),
    ) {
        if let Err(source) = self.gateway.set_timeouts(Some(short)) {
            tracing::debug!(%source, "{name} section skipped: timeout rejected");
            return;
        }
        section(self);
        if let Err(source) = self.gateway.set_timeouts(Some(SESSION_IO_TIMEOUT)) {
            tracing::debug!(%source, "{name} timeout restore failed");
        }
    }
}
