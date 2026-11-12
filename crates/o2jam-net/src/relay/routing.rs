//! Routing preface shared by relay ops 1-7: the `i32` code
//! plus the relay endpoint triple, the dual-port endpoint of ops
//! 2-3, and the adjunct-host constants (`docs/relay.md` §9).

use std::net::{Ipv4Addr, SocketAddrV4};

use crate::error::Error;
/// Adjunct host configured by the shared setup helper entered at file
/// `0x193650` (`docs/relay.md` §9).
///
/// Byte-verified references: the `203.238.136.52` string (file `0x24e368`)
/// is pushed as a call argument at exactly two image-wide sites (files
/// `0x1936e2`, `0x1936fc`), both inside the same setup block that stores
/// [`ADJUNCT_SETUP_NUMBER`]; neither the relay selector (file `0x18c6c0`)
/// nor the UHP controller (file `0x18d580`) references this host. It is
/// therefore a separately configured adjunct endpoint — not a fourth
/// relay-pool member, a UHP helper, or the `CUDPFailTCPConnect` fallback
/// path — whose exact service role stays open.
/// (ERRORS-06: plain constant, no `unwrap`; PERF-07: no narrowing casts.)
pub const ADJUNCT_RELAY_HOST: Ipv4Addr = Ipv4Addr::new(203, 238, 136, 52);

/// Numeric value co-located with the adjunct-host setup (file `0x19370b`).
///
/// Byte-verified immediate: the only `0x1e6c` dword image-wide, stored into
/// a setup-block slot just after the second adjunct-host call. Recorded as
/// a number, not claimed as a port: no socket-connect sequence ties it to
/// the host. (ERRORS-06/PERF-07: plain `u16` literal.)
pub const ADJUNCT_SETUP_NUMBER: u16 = 7788;

/// Serialized size of one relay endpoint: 4 address bytes + `u16` port.
const ENDPOINT_LEN: usize = 6;

/// Serialized size of one `ConnectionEndpoint`: two `u16` ports +
/// 4 address bytes.
const CONNECTION_ENDPOINT_LEN: usize = 8;

/// Serialized size of the routing preface shared by relay ops 1–7:
/// `i32` code + 3 endpoints + `u16` unused.
const ROUTING_LEN: usize = 4 + 3 * ENDPOINT_LEN + 2;

/// Append one relay endpoint (4 address bytes, then port as `u16`).
pub(crate) fn encode_socket(endpoint: SocketAddrV4, body: &mut Vec<u8>) {
    body.extend_from_slice(&endpoint.ip().octets());
    body.extend_from_slice(&endpoint.port().to_le_bytes());
}

/// Split 6 endpoint bytes off the front of `body`.
pub(crate) fn decode_socket(body: &[u8]) -> Result<(SocketAddrV4, &[u8]), Error> {
    if body.len() < ENDPOINT_LEN {
        return Err(Error::TruncatedMessage {
            need: ENDPOINT_LEN,
            have: body.len(),
        });
    }
    let address = Ipv4Addr::new(body[0], body[1], body[2], body[3]);
    let port = u16::from_le_bytes([body[4], body[5]]);
    Ok((SocketAddrV4::new(address, port), &body[ENDPOINT_LEN..]))
}

/// Dual-port endpoint of relay ops 2–3 (`ConnectionEndpoint`:
/// `Port1`, `Port2`, then 4 address bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionEndpoint {
    /// First port (the `IPEndPoint` view of this endpoint).
    pub port1: u16,
    /// Second port.
    pub port2: u16,
    /// IPv4 address.
    pub address: Ipv4Addr,
}

impl ConnectionEndpoint {
    /// Append the 8-byte form to `body`.
    pub(crate) fn encode_into(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.port1.to_le_bytes());
        body.extend_from_slice(&self.port2.to_le_bytes());
        body.extend_from_slice(&self.address.octets());
    }

    /// Split 8 endpoint bytes off the front of `body`.
    pub(crate) fn decode_from(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < CONNECTION_ENDPOINT_LEN {
            return Err(Error::TruncatedMessage {
                need: CONNECTION_ENDPOINT_LEN,
                have: body.len(),
            });
        }
        Ok((
            Self {
                port1: u16::from_le_bytes([body[0], body[1]]),
                port2: u16::from_le_bytes([body[2], body[3]]),
                address: Ipv4Addr::new(body[4], body[5], body[6], body[7]),
            },
            &body[CONNECTION_ENDPOINT_LEN..],
        ))
    }
}

/// Routing preface shared by relay ops 1–7 (Memoryer field orders
/// 0–2): session `code`, the 3 relay-server endpoints, `unused`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayRouting {
    /// Session code (private `Code` field; zero in server events).
    ///
    /// The only client-sent value is 0: no nonzero `code`/`unused` writer
    /// was located on any combo send path (the fanout wrappers stage
    /// routing through the `0xeb4d0` helper with pushed size `0x18`, and
    /// client combo sends carry both fields as 0; see `docs/relay.md` §6).
    /// Server-rewritten S→C values stay OPEN (needs a wire capture).
    /// Value pinned in `docs/relay.md` §11
    /// (`ev_e4c4a65ec986bea1dc13e28cbcf9d36c`, observed).
    pub code: i32,
    /// The 3 relay-server endpoints (no count prefix; always 3).
    pub relays: [SocketAddrV4; 3],
    /// Reserved word (private `Unused`; zero observed).
    pub unused: u16,
}

impl RelayRouting {
    /// Append the 24-byte preface to `body`.
    pub(crate) fn encode_into(&self, body: &mut Vec<u8>) {
        body.extend_from_slice(&self.code.to_le_bytes());
        for relay in &self.relays {
            encode_socket(*relay, body);
        }
        body.extend_from_slice(&self.unused.to_le_bytes());
    }

    /// Split the 24-byte preface off the front of `body`.
    pub(crate) fn decode_from(body: &[u8]) -> Result<(Self, &[u8]), Error> {
        if body.len() < ROUTING_LEN {
            return Err(Error::TruncatedMessage {
                need: ROUTING_LEN,
                have: body.len(),
            });
        }
        let code = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let (first, tail) = decode_socket(&body[4..])?;
        let (second, tail) = decode_socket(tail)?;
        let (third, tail) = decode_socket(tail)?;
        let unused = u16::from_le_bytes([tail[0], tail[1]]);
        Ok((
            Self {
                code,
                relays: [first, second, third],
                unused,
            },
            &tail[2..],
        ))
    }
}

/// Read one little-endian `u16` at `at`.
pub(crate) fn word_at(body: &[u8], at: usize) -> Result<u16, Error> {
    if body.len() < at + 2 {
        return Err(Error::TruncatedMessage {
            need: at + 2,
            have: body.len(),
        });
    }
    Ok(u16::from_le_bytes([body[at], body[at + 1]]))
}

#[cfg(test)]
mod adjunct_host_tests {
    use super::*;

    #[test]
    fn adjunct_host_matches_documented_address() {
        assert_eq!(ADJUNCT_RELAY_HOST.octets(), [203, 238, 136, 52]);
    }

    #[test]
    fn adjunct_host_shares_relay_subnet_but_not_pool() {
        let pool = [
            Ipv4Addr::new(203, 238, 136, 184),
            Ipv4Addr::new(203, 238, 136, 182),
            Ipv4Addr::new(203, 238, 136, 183),
        ];
        for member in pool {
            assert_ne!(ADJUNCT_RELAY_HOST, member);
            assert_eq!(ADJUNCT_RELAY_HOST.octets()[..3], member.octets()[..3]);
        }
    }

    #[test]
    fn adjunct_setup_number_matches_exe_immediate() {
        assert_eq!(ADJUNCT_SETUP_NUMBER, 0x1E6C);
        assert_eq!(ADJUNCT_SETUP_NUMBER, 7788);
    }
}
