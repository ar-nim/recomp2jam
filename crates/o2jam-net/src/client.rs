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

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

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

    /// Block until the next complete frame arrives.
    pub fn next_frame(&mut self) -> Result<Frame, Error> {
        loop {
            if let Some(frame) = self.drain_pending()? {
                return Ok(frame);
            }
            let chunk = self.read_chunk()?;
            let frames = self.incoming.push_bytes(&chunk)?;
            if let Some(frame) = frames.into_iter().next() {
                return Ok(frame);
            }
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
        })
    }

    fn drain_pending(&mut self) -> Result<Option<Frame>, Error> {
        let frames = self.incoming.push_bytes(&[])?;
        Ok(frames.into_iter().next())
    }

    fn read_chunk(&mut self) -> Result<Vec<u8>, Error> {
        let mut chunk = vec![0u8; READ_CHUNK_LEN];
        let read = self.stream.read(&mut chunk).map_err(Error::transport)?;
        if read == 0 {
            return Err(Error::ConnectionClosed);
        }
        chunk.truncate(read);
        Ok(chunk)
    }
}

#[cfg(test)]
mod gateway_connection_tests {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    use super::*;
    use crate::frame::encode_frame;

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

    #[test]
    fn surfaces_malformed_length_from_server() {
        let (mut client, mut server) = loopback_pair();
        server.write_all(&[0x01, 0x00, 0x00, 0x00]).unwrap();
        let result = client.next_frame();
        assert!(matches!(result, Err(Error::MissingCommand(1))));
    }
}
