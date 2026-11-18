//! Relay-availability gate for the pre-channel-list notice (#244).
//!
//! The 7.55 reconnect helper (file `0x18c6c0`, VA `0x58c6c0`)
//! selects one of three hardcoded relay hosts and one of two port
//! words before it tries the socket. The channel/planet handlers
//! post the one-OK NORMAL-mode message when that attempt fails
//! (files `0x474eca`/`0x8e340`, string file `0x244d98`). This module
//! is our equivalent probe: it checks the same hosts and ports and
//! tells the boot staging which branch to take. It never opens a
//! relay session; reconnection stays out of scope.

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Relay hosts, in the exe's address-table order (file
/// `0x24e240`/`0x24e250`/`0x24e260`, `docs/net-protocol.md:38`).
pub const RELAY_HOSTS: [&str; 3] = ["203.238.136.184", "203.238.136.182", "203.238.136.183"];

/// One connect attempt budget. The exe blocks on its socket; the
/// modern client keeps the boot responsive with a bounded probe.
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(120);

/// Probe the exe-pinned relay endpoints on the live boot path.
#[must_use]
pub fn relay_down_now() -> bool {
    let endpoints: Vec<SocketAddr> = RELAY_HOSTS
        .iter()
        .filter_map(|host| host.parse::<std::net::Ipv4Addr>().ok())
        .flat_map(|host| {
            o2jam_net::relay::RELAY_UDP_PORTS
                .iter()
                .map(move |port| SocketAddr::from((host, *port)))
        })
        .collect();
    relay_down_for(&endpoints)
}

/// `true` when no endpoint answers. An empty slice is down: there
/// is no relay to reach. Tests pass local sockets here, so no test
/// needs the public Internet.
#[must_use]
pub fn relay_down_for(endpoints: &[SocketAddr]) -> bool {
    !endpoints
        .iter()
        .any(|address| TcpStream::connect_timeout(address, PROBE_TIMEOUT).is_ok())
}

#[cfg(test)]
mod relay_gate_tests {
    use std::net::{SocketAddr, TcpListener};

    use super::{relay_down_for, PROBE_TIMEOUT};

    /// Live caller chain: `run_online`/`run_play --online` ->
    /// `relay_gate::relay_down_now` -> `setup_online`/`stage_online`
    /// -> `stage_channel_list_after_relay_gate` ->
    /// `ScreenGame::hold_channel_rows_for_relay_notice` ->
    /// channel entry -> `open_relay_down_dialog`. This seam pins the
    /// probe end of that chain with a local socket instead of the
    /// pinned Internet hosts.
    #[test]
    fn relay_up_when_a_pinned_style_endpoint_answers() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("local relay listener");
        let address: SocketAddr = listener.local_addr().expect("local relay address");
        assert!(!relay_down_for(&[address]), "listening relay is up");
    }

    /// Same chain, negative arm: no socket answers, so boot holds
    /// the channel list for the notice at channel entry (#348).
    #[test]
    fn relay_down_when_every_pinned_style_endpoint_refuses() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("reserved relay port");
        let address: SocketAddr = listener.local_addr().expect("reserved relay address");
        drop(listener);
        assert!(relay_down_for(&[address]), "refusing relay is down");
        assert_eq!(PROBE_TIMEOUT.as_millis(), 120, "bounded boot probe");
    }
}
