//! UDP Live transport: RUDP inner packet types, datagram
//! framing, and the exe-pinned relay UDP ports (`docs/relay.md`).

use crate::error::Error;
use crate::inflate::{compress_zlib_stored, decompress_zlib};
use crate::relay::commands::RelayCommand;
/// Hardcoded relay endpoint ports picked by the in-exe relay selector
/// alongside the three relay IPs (`docs/relay.md` §9). These are the
/// only hardcoded relay ports: a full-image `u16` scan finds `0x998f`
/// at exactly the three selector-leaf stores (files
/// `0x18c73b`–`0x18c77f`) and `0x9990` port-word stores only at the
/// two matching leaves; the five other `0x9990` hits are non-port
/// bytes (table address, relative call, jump table, data). Relay TCP
/// peer-leg ports are server-provided (`GetLiveState`/`GetP2PList`),
/// never immediates, so no `RELAY_TCP_PORTS` constant exists by design.
///
/// Byte-verified immediates: `0x998f`/`0x9990` stored as the port word by
/// the selector leaves at file `0x18c73b`–`0x18c77f`; transport role is
/// inferred from the UHP/UDP context, not proven by those bytes alone.
/// (ERRORS-06: plain constants, no `unwrap`; PERF-07: `u16` literals,
/// no narrowing casts.)
pub const RELAY_UDP_PORTS: [u16; 2] = [39311, 39312];

/// RUDP inner packet types (`Memoryer.Relay` `UdpRelayPacketType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UdpPacketType {
    /// Fire-and-forget (Live lane traffic).
    Unreliable = 1,
    /// Retransmitted until acked (stats, combo packs).
    Reliable = 2,
    /// Bare 4-byte ack, never zlib-wrapped.
    Ack = 3,
}

impl UdpPacketType {
    /// Numeric wire value.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Parse a wire byte; unknown types are an error.
    ///
    /// # Errors
    ///
    /// `Error::InvalidField` outside 1–3.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            1 => Ok(Self::Unreliable),
            2 => Ok(Self::Reliable),
            3 => Ok(Self::Ack),
            _ => Err(Error::InvalidField {
                field: "udp packet type",
                reason: "not 1, 2, or 3",
            }),
        }
    }
}

/// One decoded RUDP datagram.
///
/// `payload` is the relay command frame the inner envelope wraps:
/// `[u16 command][body]` (the envelope's `u32` command word and
/// `innerSize` framing are validated, then stripped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpRelayFrame {
    /// Inner packet type.
    pub packet_type: UdpPacketType,
    /// Inner sequence number (acked by [`UdpRelayFrame::encode_ack`]).
    pub sequence: u8,
    /// Relay command frame: `[u16 command][body]`.
    pub payload: Vec<u8>,
}

/// Bare ack length: `[0x03][seq][0x00][0x00]`, no zlib, no prefix.
const ACK_FRAME_LEN: usize = 4;
/// Datagram length prefix: `u16` uncompressed length.
const DATAGRAM_LEN_LEN: usize = 2;
/// Inner envelope: type + seq + size + `u32` command.
const INNER_HEADER_LEN: usize = 8;

impl UdpRelayFrame {
    /// Decode one UDP datagram: bare ack or length-prefixed zlib.
    ///
    /// # Errors
    ///
    /// Truncation on short input, `Error::InvalidField` on bad
    /// packet types, sizes, commands, or zlib checksums.
    pub fn decode_datagram(datagram: &[u8]) -> Result<Self, Error> {
        if datagram.len() == ACK_FRAME_LEN && datagram[0] == UdpPacketType::Ack.as_u8() {
            return Ok(Self {
                packet_type: UdpPacketType::Ack,
                sequence: datagram[1],
                payload: Vec::new(),
            });
        }
        if datagram.len() < DATAGRAM_LEN_LEN {
            return Err(Error::TruncatedMessage {
                need: DATAGRAM_LEN_LEN,
                have: datagram.len(),
            });
        }
        let uncompressed_len = u16::from_le_bytes([datagram[0], datagram[1]]) as usize;
        let inflated = decompress_zlib(&datagram[DATAGRAM_LEN_LEN..])?;
        if inflated.len() < uncompressed_len {
            return Err(Error::TruncatedMessage {
                need: uncompressed_len,
                have: inflated.len(),
            });
        }
        if inflated.len() > uncompressed_len {
            return Err(Error::TrailingBytes(inflated.len() - uncompressed_len));
        }
        Self::decode_inner(&inflated)
    }

    /// Parse the inflated inner envelope.
    fn decode_inner(inner: &[u8]) -> Result<Self, Error> {
        if inner.len() < INNER_HEADER_LEN {
            return Err(Error::TruncatedMessage {
                need: INNER_HEADER_LEN,
                have: inner.len(),
            });
        }
        let packet_type = UdpPacketType::from_u8(inner[0])?;
        let sequence = inner[1];
        let inner_size = u16::from_le_bytes([inner[2], inner[3]]) as usize;
        if inner_size < 4 {
            return Err(Error::InvalidField {
                field: "relay inner size",
                reason: "smaller than command word",
            });
        }
        let command_word = u32::from_le_bytes([inner[4], inner[5], inner[6], inner[7]]);
        let command = u16::try_from(command_word).map_err(|_| Error::InvalidField {
            field: "relay command",
            reason: "wider than u16",
        })?;
        let body_len = inner_size - 4;
        if inner.len() < INNER_HEADER_LEN + body_len {
            return Err(Error::TruncatedMessage {
                need: INNER_HEADER_LEN + body_len,
                have: inner.len(),
            });
        }
        if inner.len() > INNER_HEADER_LEN + body_len {
            return Err(Error::TrailingBytes(
                inner.len() - INNER_HEADER_LEN - body_len,
            ));
        }
        let mut payload = Vec::with_capacity(2 + body_len);
        payload.extend_from_slice(&command.to_le_bytes());
        payload.extend_from_slice(&inner[INNER_HEADER_LEN..INNER_HEADER_LEN + body_len]);
        Ok(Self {
            packet_type,
            sequence,
            payload,
        })
    }

    /// Build one UDP datagram: inner envelope + stored-block zlib +
    /// `u16` length prefix. Use [`UdpRelayFrame::encode_ack`] for acks.
    #[must_use]
    pub fn encode_datagram(
        packet_type: UdpPacketType,
        sequence: u8,
        command: u16,
        body: &[u8],
    ) -> Vec<u8> {
        let inner_size = u16::try_from(body.len() + 4).unwrap_or(u16::MAX);
        let mut inner = Vec::with_capacity(INNER_HEADER_LEN + body.len());
        inner.push(packet_type.as_u8());
        inner.push(sequence);
        inner.extend_from_slice(&inner_size.to_le_bytes());
        inner.extend_from_slice(&u32::from(command).to_le_bytes());
        inner.extend_from_slice(body);
        let compressed = compress_zlib_stored(&inner);
        let mut datagram = Vec::with_capacity(DATAGRAM_LEN_LEN + compressed.len());
        let prefix = u16::try_from(inner.len()).unwrap_or(u16::MAX);
        datagram.extend_from_slice(&prefix.to_le_bytes());
        datagram.extend_from_slice(&compressed);
        datagram
    }

    /// Build a bare 4-byte ack for `sequence`.
    #[must_use]
    pub fn encode_ack(sequence: u8) -> [u8; ACK_FRAME_LEN] {
        [UdpPacketType::Ack.as_u8(), sequence, 0x00, 0x00]
    }

    /// Relay command id carried in `payload`, if present.
    #[must_use]
    pub fn command_id(&self) -> Option<u16> {
        if self.payload.len() < 2 {
            return None;
        }
        Some(u16::from_le_bytes([self.payload[0], self.payload[1]]))
    }

    /// Relay command carried in `payload`, if known.
    #[must_use]
    pub fn relay_command(&self) -> Option<RelayCommand> {
        self.command_id().and_then(RelayCommand::from_id)
    }

    /// Relay command body (bytes after the command id).
    #[must_use]
    pub fn body(&self) -> &[u8] {
        if self.payload.len() < 2 {
            return &[];
        }
        &self.payload[2..]
    }
}

#[cfg(test)]
mod udp_frame_tests {
    use super::*;

    #[test]
    fn relay_udp_ports_match_exe_selector_immediates() {
        assert_eq!(RELAY_UDP_PORTS, [0x998F, 0x9990]);
        assert_eq!(RELAY_UDP_PORTS[0], 39311);
        assert_eq!(RELAY_UDP_PORTS[1], 39312);
        assert_ne!(RELAY_UDP_PORTS[0], RELAY_UDP_PORTS[1]);
    }

    #[test]
    fn udp_packet_type_names_all_types() {
        assert_eq!(UdpPacketType::from_u8(1), Ok(UdpPacketType::Unreliable));
        assert_eq!(UdpPacketType::from_u8(2), Ok(UdpPacketType::Reliable));
        assert_eq!(UdpPacketType::from_u8(3), Ok(UdpPacketType::Ack));
        assert_eq!(UdpPacketType::Reliable.as_u8(), 2);
    }

    #[test]
    fn udp_packet_type_rejects_unknown_type() {
        assert_eq!(
            UdpPacketType::from_u8(9),
            Err(Error::InvalidField {
                field: "udp packet type",
                reason: "not 1, 2, or 3"
            })
        );
    }

    #[test]
    fn ack_encodes_bare_four_bytes() {
        assert_eq!(UdpRelayFrame::encode_ack(0x2A), [0x03, 0x2A, 0x00, 0x00]);
    }

    #[test]
    fn ack_decodes_without_zlib() {
        let frame = UdpRelayFrame::decode_datagram(&[0x03, 0x2A, 0x00, 0x00]).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Ack);
        assert_eq!(frame.sequence, 0x2A);
        assert!(frame.payload.is_empty());
        assert_eq!(frame.command_id(), None);
        assert_eq!(frame.relay_command(), None);
        assert_eq!(frame.body(), &[]);
    }

    /// Fixed-Huffman datagram from Python `zlib.compress` (level 6):
    /// inner `[01][09][08 00][05 00 00 00][AA BB CC DD]`.
    fn fixed_datagram() -> Vec<u8> {
        vec![
            0x0C, 0x00, 0x78, 0x9C, 0x63, 0xE4, 0xE4, 0x60, 0x60, 0x65, 0x60, 0x60, 0x58, 0xB5,
            0xFB, 0xCC, 0x5D, 0x00, 0x08, 0x41, 0x03, 0x26,
        ]
    }

    #[test]
    fn datagram_decodes_fixed_huffman_block() {
        let frame = UdpRelayFrame::decode_datagram(&fixed_datagram()).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Unreliable);
        assert_eq!(frame.sequence, 0x09);
        assert_eq!(frame.payload, vec![0x05, 0x00, 0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(frame.command_id(), Some(5));
        assert_eq!(frame.relay_command(), Some(RelayCommand::UpdateGameStats));
        assert_eq!(frame.body(), &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    /// Stored-block datagram from Python `zlib.compress` (level 0).
    fn stored_datagram() -> Vec<u8> {
        vec![
            0x0C, 0x00, 0x78, 0x01, 0x01, 0x0C, 0x00, 0xF3, 0xFF, 0x01, 0x09, 0x08, 0x00, 0x05,
            0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD, 0x08, 0x41, 0x03, 0x26,
        ]
    }

    #[test]
    fn datagram_decodes_stored_block() {
        let frame = UdpRelayFrame::decode_datagram(&stored_datagram()).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Unreliable);
        assert_eq!(frame.sequence, 0x09);
        assert_eq!(frame.payload, vec![0x05, 0x00, 0xAA, 0xBB, 0xCC, 0xDD]);
    }

    /// Dynamic-Huffman datagram from Python `zlib.compress` (level 6,
    /// 603-byte text inner): type Unreliable, seq `0x2C`, command 5.
    fn dynamic_datagram() -> Vec<u8> {
        vec![
            0x5B, 0x02, 0x78, 0x9C, 0x55, 0xD1, 0x41, 0x6A, 0x03, 0x31, 0x0C, 0x05, 0xD0, 0x69,
            0xA0, 0xF7, 0xF0, 0x01, 0xBA, 0x90, 0x2C, 0xD9, 0xB2, 0xC8, 0x41, 0xB2, 0x0E, 0x61,
            0x0A, 0x81, 0x29, 0x43, 0x33, 0xAB, 0xDE, 0x3E, 0x06, 0x49, 0x69, 0xE4, 0x8D, 0x36,
            0xFF, 0xF1, 0x0D, 0xFF, 0xE3, 0xEB, 0x72, 0xFA, 0x5C, 0x96, 0xE5, 0xB1, 0x6E, 0xD7,
            0xBF, 0x72, 0xAC, 0xBF, 0x05, 0xCA, 0x6D, 0xDF, 0xB7, 0x79, 0x7E, 0xEE, 0xC7, 0x31,
            0xCF, 0x76, 0xFF, 0x5E, 0x0B, 0x02, 0x40, 0x39, 0x6E, 0xFB, 0x63, 0x2D, 0x70, 0x2E,
            0xFF, 0x61, 0xB4, 0x30, 0x59, 0x58, 0x2C, 0xAC, 0xAA, 0x9E, 0xC5, 0x4A, 0xDC, 0xDE,
            0xF3, 0xD5, 0xF2, 0xDD, 0xF2, 0xC8, 0x01, 0x86, 0x83, 0xCA, 0x5D, 0x53, 0x01, 0x19,
            0x50, 0x03, 0x15, 0x03, 0x88, 0x03, 0x12, 0xA0, 0xD4, 0xC0, 0x06, 0xB0, 0xBA, 0x18,
            0x21, 0xBA, 0x0B, 0x56, 0x1A, 0xA9, 0xA2, 0xB9, 0x68, 0x26, 0xA8, 0x85, 0x68, 0x2E,
            0x3A, 0x4A, 0x4D, 0x1D, 0xDD, 0xC5, 0x30, 0xC1, 0x35, 0x04, 0xBB, 0x10, 0x06, 0x49,
            0x1D, 0x62, 0x62, 0xFE, 0xDF, 0x84, 0x86, 0x20, 0x17, 0xA3, 0x33, 0xA6, 0x8E, 0xE1,
            0x82, 0x4D, 0xB4, 0x1E, 0xA2, 0xBA, 0xD0, 0x21, 0x3D, 0x75, 0xA8, 0x0B, 0x31, 0xD1,
            0x29, 0x04, 0xC6, 0x1A, 0xF3, 0x41, 0x2A, 0x41, 0x1F, 0x9B, 0x7C, 0xED, 0x57, 0x09,
            0xBC, 0x0F, 0x98, 0x17, 0x8F, 0xC9, 0x7D, 0x73, 0x8C, 0x9A, 0xF1, 0x1A, 0x9D, 0x9A,
            0xE8, 0xAC, 0x79, 0x02, 0x5D, 0xDD, 0xB1, 0x42,
        ]
    }

    fn dynamic_text() -> Vec<u8> {
        let mut text = Vec::new();
        for index in 0..12u32 {
            text.extend_from_slice(
                format!(
                    "relay seq {} cool {} miss {} life {} score {}; ",
                    index,
                    index * 3 % 256,
                    index * 7 % 64,
                    1000 - index,
                    index * 12345,
                )
                .as_bytes(),
            );
        }
        text
    }

    #[test]
    fn datagram_decodes_dynamic_huffman_block() {
        let frame = UdpRelayFrame::decode_datagram(&dynamic_datagram()).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Unreliable);
        assert_eq!(frame.sequence, 0x2C);
        assert_eq!(frame.command_id(), Some(5));
        let mut payload = vec![0x05, 0x00];
        payload.extend_from_slice(&dynamic_text());
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn datagram_round_trips_through_stored_encoder() {
        let body = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let datagram = UdpRelayFrame::encode_datagram(UdpPacketType::Reliable, 0x2C, 6, &body);
        let frame = UdpRelayFrame::decode_datagram(&datagram).unwrap();
        assert_eq!(frame.packet_type, UdpPacketType::Reliable);
        assert_eq!(frame.sequence, 0x2C);
        assert_eq!(frame.relay_command(), Some(RelayCommand::ComboBroken));
        assert_eq!(frame.body(), &body);
    }

    #[test]
    fn datagram_rejects_length_prefix_mismatch() {
        let mut datagram = fixed_datagram();
        datagram[0] = 0x0D;
        assert_eq!(
            UdpRelayFrame::decode_datagram(&datagram),
            Err(Error::TruncatedMessage { need: 13, have: 12 })
        );
    }

    #[test]
    fn datagram_rejects_bad_checksum() {
        let mut datagram = stored_datagram();
        let last = datagram.len() - 1;
        datagram[last] ^= 0xFF;
        assert_eq!(
            UdpRelayFrame::decode_datagram(&datagram),
            Err(Error::InvalidField {
                field: "zlib checksum",
                reason: "adler32 mismatch"
            })
        );
    }

    #[test]
    fn datagram_rejects_tiny_input() {
        assert_eq!(
            UdpRelayFrame::decode_datagram(&[0x01]),
            Err(Error::TruncatedMessage { need: 2, have: 1 })
        );
    }
}
