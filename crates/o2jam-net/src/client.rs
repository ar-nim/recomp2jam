//! Blocking gateway TCP client (standard library only).
//!
//! Phase 1 of `docs/client-server-plan.md`: connect to a
//! Memoryer gateway (default port `15010`, in-exe string at file
//! `0x2513d8`) and move length-framed [`Frame`]s over it. The
//! framing itself stays exe-verified (command id as little-endian
//! `u16` at buffer offset 2, `FUN_005a0550` file `0x1a0550`); the
//! length-after-header semantics still await live interop as the
//! final check.
//!
//! The client is transport only: it sends raw frames and returns
//! received frames. Message bodies stay in [`crate::message`], so
//! the blocked `Authorize` string codec cannot leak in here.

use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::command::Generic;
use super::error::Error;
use super::frame::{encode_frame, Frame, FrameSplitter};

/// Default gateway TCP port (in-exe string at file `0x2513d8`).
pub const DEFAULT_GATEWAY_PORT: u16 = 15010;
/// How long a blocking read waits when no timeout was set.
const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(10);
/// Scratch size for one socket read.
const READ_CHUNK_LEN: usize = 1024;

/// One blocking connection to a Memoryer gateway.
#[derive(Debug)]
pub struct GatewayConnection {
    stream: TcpStream,
    incoming: FrameSplitter,
    pending: VecDeque<Frame>,
}

impl GatewayConnection {
    /// Open a connection to any socket address.
    pub fn connect(address: impl ToSocketAddrs) -> Result<Self, Error> {
        let stream = TcpStream::connect(address).map_err(Error::transport)?;
        Self::wrap(stream)
    }

    /// Open a connection to `host` on the default gateway port.
    pub fn connect_to_gateway(host: &str) -> Result<Self, Error> {
        Self::connect((host, DEFAULT_GATEWAY_PORT))
    }

    /// Send one frame (`id` + raw body bytes).
    pub fn send_frame(&mut self, id: u16, body: &[u8]) -> Result<(), Error> {
        let bytes = encode_frame(id, body);
        self.stream.write_all(&bytes).map_err(Error::transport)
    }

    /// Send a request, then block for the frame under `response`.
    ///
    /// Bodies stay raw so callers can drive opcodes whose layouts are
    /// still open (notably `Authorize` 1000) without guessing codecs.
    pub fn exchange(
        &mut self,
        request_id: u16,
        request_body: &[u8],
        response_id: u16,
    ) -> Result<Frame, Error> {
        self.send_frame(request_id, request_body)?;
        self.await_response(response_id)
    }

    /// Block for the frame under `expected`, skipping keepalive pings.
    ///
    /// Any other non-keepalive id is an [`Error::UnexpectedResponse`],
    /// not a skip: silently dropping server frames would hide
    /// dispatches the caller must handle (alerts 50/51/52, say).
    pub fn await_response(&mut self, expected: u16) -> Result<Frame, Error> {
        loop {
            let frame = self.next_frame()?;
            if is_keepalive(frame.id) {
                continue;
            }
            if frame.id == expected {
                return Ok(frame);
            }
            return Err(Error::UnexpectedResponse {
                expected,
                found: frame.id,
            });
        }
    }

    /// Block until the next complete frame arrives.
    pub fn next_frame(&mut self) -> Result<Frame, Error> {
        loop {
            if let Some(frame) = self.pending.pop_front() {
                return Ok(frame);
            }
            let chunk = self.read_chunk()?;
            let frames = self.incoming.push_bytes(&chunk)?;
            self.pending.extend(frames);
        }
    }

    /// Override the blocking read/write timeouts.
    pub fn set_timeouts(&self, timeout: Option<Duration>) -> Result<(), Error> {
        self.stream
            .set_read_timeout(timeout)
            .map_err(Error::transport)?;
        self.stream
            .set_write_timeout(timeout)
            .map_err(Error::transport)
    }

    /// Wrap an already-connected stream (tests, harnesses).
    pub fn from_stream(stream: TcpStream) -> Result<Self, Error> {
        Self::wrap(stream)
    }

    fn wrap(stream: TcpStream) -> Result<Self, Error> {
        stream
            .set_read_timeout(Some(DEFAULT_IO_TIMEOUT))
            .map_err(Error::transport)?;
        stream
            .set_write_timeout(Some(DEFAULT_IO_TIMEOUT))
            .map_err(Error::transport)?;
        Ok(Self {
            stream,
            incoming: FrameSplitter::default(),
            pending: VecDeque::new(),
        })
    }

    fn read_chunk(&mut self) -> Result<Vec<u8>, Error> {
        let mut chunk = vec![0u8; READ_CHUNK_LEN];
        let read = self.stream.read(&mut chunk).map_err(|source| {
            if source.kind() == ErrorKind::WouldBlock {
                Error::Transport("read timed out waiting for server frame".to_owned())
            } else {
                Error::transport(source)
            }
        })?;
        if read == 0 {
            return Err(Error::ConnectionClosed);
        }
        chunk.truncate(read);
        Ok(chunk)
    }
}

/// Keepalive echo ids (`6000`/`6001`) carry no session meaning.
fn is_keepalive(id: u16) -> bool {
    Generic::from_id(id).is_some()
}

#[cfg(test)]
mod gateway_connection_tests {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    use super::*;
    use crate::command::Request;
    use crate::frame::{decode_frame, encode_frame, ID_OFFSET};
    use crate::message::{ChannelList, WireMessage};

    fn loopback_pair() -> (GatewayConnection, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = thread::spawn(move || listener.accept().unwrap().0);
        let client = GatewayConnection::connect(address).unwrap();
        let server = accepted.join().unwrap();
        (client, server)
    }

    #[test]
    fn delivers_server_frame_to_blocking_reader() {
        let (mut client, mut server) = loopback_pair();
        server
            .write_all(&encode_frame(1003, &[0x00, 0x00, 0x00, 0x00]))
            .unwrap();
        let frame = client.next_frame().unwrap();
        assert_eq!(frame.id, 1003);
        assert_eq!(frame.payload, vec![0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn round_trips_client_ping_through_loopback_server() {
        let (mut client, mut server) = loopback_pair();
        client.send_frame(6001, &[]).unwrap();
        let mut header = [0u8; 4];
        server.read_exact(&mut header).unwrap();
        assert_eq!(header, [0x02, 0x00, 0x71, 0x17]);
    }

    #[test]
    fn reports_peer_close_instead_of_blocking_forever() {
        let (mut client, server) = loopback_pair();
        drop(server);
        assert_eq!(client.next_frame(), Err(Error::ConnectionClosed));
    }

    /// Read one whole frame off the test server's socket.
    fn read_request_frame(server: &mut TcpStream) -> Frame {
        let mut header = [0u8; 4];
        server.read_exact(&mut header).unwrap();
        let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
        let mut rest = vec![0u8; body_len - ID_OFFSET];
        server.read_exact(&mut rest).unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&rest);
        decode_frame(&bytes).unwrap()
    }

    #[test]
    fn exchange_returns_empty_channel_list_for_list_request() {
        let (mut client, mut server) = loopback_pair();
        let responder = thread::spawn(move || {
            let request = read_request_frame(&mut server);
            assert_eq!(request.id, Request::GetChannelList.id());
            assert!(request.payload.is_empty());
            let empty = ChannelList { channels: vec![] };
            server.write_all(&empty.encode()).unwrap();
        });
        let frame = client
            .exchange(Request::GetChannelList.id(), &[], ChannelList::COMMAND)
            .unwrap();
        responder.join().unwrap();
        assert_eq!(
            ChannelList::decode_frame(&frame).unwrap(),
            ChannelList { channels: vec![] }
        );
    }

    #[test]
    fn await_response_skips_keepalive_ping_before_answer() {
        let (mut client, mut server) = loopback_pair();
        server.write_all(&encode_frame(6001, &[])).unwrap();
        server
            .write_all(&encode_frame(1003, &[0x00, 0x00, 0x00, 0x00]))
            .unwrap();
        let frame = client.await_response(1003).unwrap();
        assert_eq!(frame.id, 1003);
    }

    #[test]
    fn await_response_rejects_wrong_command_id() {
        let (mut client, mut server) = loopback_pair();
        server.write_all(&encode_frame(1008, &[0x00; 51])).unwrap();
        assert_eq!(
            client.await_response(1003),
            Err(Error::UnexpectedResponse {
                expected: 1003,
                found: 1008
            })
        );
    }

    #[test]
    fn surfaces_malformed_length_from_server() {
        let (mut client, mut server) = loopback_pair();
        server.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap();
        let result = client.next_frame();
        assert!(matches!(result, Err(Error::MissingCommand(1))));
    }
}
