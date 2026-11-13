//! Shared scripted-gateway test helpers (ticket #203, ADR-0014).

use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::client::GatewayConnection;
use crate::frame::{decode_frame, encode_frame, HEADER_LEN};
use crate::Frame;

/// Read one whole frame off a TCP stream.
pub fn read_frame(stream: &mut TcpStream) -> Frame {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).unwrap();
    let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
    let mut rest = vec![0u8; body_len - HEADER_LEN];
    stream.read_exact(&mut rest).unwrap();
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(&rest);
    decode_frame(&bytes).unwrap()
}

/// Encode a frame (re-export convenience).
pub fn encode(id: u16, payload: &[u8]) -> Vec<u8> {
    encode_frame(id, payload)
}

/// Loopback gateway connection + server stream.
pub fn loopback_pair() -> (GatewayConnection, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = std::thread::spawn(move || listener.accept().unwrap().0);
    let client = GatewayConnection::connect(address).unwrap();
    let server = accepted.join().unwrap();
    (client, server)
}

/// Fake gateway on loopback: port + server thread.
pub fn fake_gateway(with_server: impl FnOnce(TcpStream) + Send + 'static) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        with_server(stream);
    });
    (port, server)
}
