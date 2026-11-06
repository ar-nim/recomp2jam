//! Online login flow against a Memoryer gateway.
//!
//! Phase 1 of `docs/client-server-plan.md`: connect, authorize,
//! list channels, optionally log into one — all through
//! [`GatewayConnection::exchange`]. Relay keys normally arrive via
//! the launcher (`OTwo.exe … |test|??|addr|port` argv); without one,
//! they default to zero and the server decides.

use std::net::SocketAddr;

use anyhow::{bail, Context, Result};
use o2jam_net::{
    AuthResponse, AuthResult, AuthorizeRequest, ChannelList, ChannelLogin, ChannelLoginResponse,
    ChannelState, GatewayConnection, Gender, Request, WireMessage,
};

/// Login options (see `recomp2jam connect --help`).
pub struct ConnectOptions {
    /// Gateway host.
    pub host: String,
    /// Gateway TCP port.
    pub port: u16,
    /// Account id.
    pub user_id: i32,
    /// Username.
    pub token: String,
    /// Account gender.
    pub gender: Gender,
    /// Relay key from launcher auth (0 without one).
    pub relay_key1: i32,
    /// Relay key from launcher auth (0 without one).
    pub relay_key2: i32,
    /// `(gateway, channel)` to enter after listing; `None` lists only.
    pub channel: Option<(i16, i16)>,
}

/// What the login flow learned.
#[derive(Debug)]
pub struct ConnectReport {
    /// Authorize result code (always success — else this errors).
    pub auth_code: u32,
    /// Channels the gateway listed.
    pub channels: Vec<ChannelState>,
    /// Channel login answer, when a channel was requested.
    pub channel_login: Option<ChannelLoginResponse>,
}

/// Parse a `--gender` word.
pub fn parse_gender(word: &str) -> Result<Gender> {
    match word.to_ascii_lowercase().as_str() {
        "male" | "m" | "1" => Ok(Gender::Male),
        "female" | "f" | "0" => Ok(Gender::Female),
        "any" | "unknown" | "" => Ok(Gender::Any),
        _ => bail!("unknown gender '{word}' (want male, female, or any)"),
    }
}

/// Connect, authorize, list channels, and optionally log into one.
pub fn connect_and_login(options: &ConnectOptions) -> Result<ConnectReport> {
    let address = format!("{}:{}", options.host, options.port);
    let mut gateway = GatewayConnection::connect(address.as_str())
        .with_context(|| format!("connecting to {address}"))?;
    let auth_code = authorize(&mut gateway, options).context("authorizing")?;
    let channels = fetch_channels(&mut gateway).context("listing channels")?;
    let channel_login = match options.channel {
        Some((gateway_id, channel_id)) => Some(
            login_channel(&mut gateway, gateway_id, channel_id).context("logging into channel")?,
        ),
        None => None,
    };
    Ok(ConnectReport {
        auth_code,
        channels,
        channel_login,
    })
}

/// Human-readable authorize/channel result code.
#[must_use]
pub fn name_result(code: u32) -> String {
    match AuthResult::from_code(code) {
        Some(named) => format!("{named:?} ({code})"),
        None => format!("unknown code {code}"),
    }
}

/// Our socket address as IPv4 (login endpoints are IPv4-only).
fn local_ipv4(gateway: &GatewayConnection) -> Result<std::net::SocketAddrV4> {
    match gateway.local_address()? {
        SocketAddr::V4(endpoint) => Ok(endpoint),
        SocketAddr::V6(_) => bail!("gateway resolved IPv6; login endpoints are IPv4-only"),
    }
}

fn authorize(gateway: &mut GatewayConnection, options: &ConnectOptions) -> Result<u32> {
    let local = local_ipv4(gateway)?;
    let request = AuthorizeRequest::new(
        options.relay_key1,
        options.relay_key2,
        local,
        local,
        options.user_id,
        options.token.clone(),
        options.gender,
    );
    let frame = gateway.exchange(
        Request::Authorize.id(),
        &request.encode_body(),
        AuthResponse::COMMAND,
    )?;
    let response = AuthResponse::decode_frame(&frame)?;
    match AuthResult::from_code(response.result_code) {
        Some(AuthResult::Success) => Ok(response.result_code),
        _ => bail!("authorize rejected: {}", name_result(response.result_code)),
    }
}

fn fetch_channels(gateway: &mut GatewayConnection) -> Result<Vec<ChannelState>> {
    let frame = gateway.exchange(Request::GetChannelList.id(), &[], ChannelList::COMMAND)?;
    Ok(ChannelList::decode_frame(&frame)?.channels)
}

fn login_channel(
    gateway: &mut GatewayConnection,
    gateway_id: i16,
    channel_id: i16,
) -> Result<ChannelLoginResponse> {
    let request = ChannelLogin {
        gateway_id,
        channel_id,
    };
    let frame = gateway.exchange(
        Request::ChannelLogin.id(),
        &request.encode_body(),
        ChannelLoginResponse::COMMAND,
    )?;
    let response = ChannelLoginResponse::decode_frame(&frame)?;
    match o2jam_net::ChannelLoginResult::from_code(response.result_code) {
        Some(o2jam_net::ChannelLoginResult::Success) => Ok(response),
        _ => bail!(
            "channel login rejected: {}",
            name_result(response.result_code)
        ),
    }
}

#[cfg(test)]
mod connect_flow_tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use o2jam_net::{
        frame::decode_frame, AuthResponse, ChannelList, ChannelLoginResponse, Frame, ID_OFFSET,
    };

    use super::*;

    /// Read one whole frame off the fake server's socket.
    fn read_frame(stream: &mut TcpStream) -> Frame {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
        let mut rest = vec![0u8; body_len - ID_OFFSET];
        stream.read_exact(&mut rest).unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&rest);
        decode_frame(&bytes).unwrap()
    }

    fn test_options(port: u16, channel: Option<(i16, i16)>) -> ConnectOptions {
        ConnectOptions {
            host: "127.0.0.1".to_owned(),
            port,
            user_id: 7,
            token: "tester".to_owned(),
            gender: Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel,
        }
    }

    #[test]
    fn parses_gender_words() {
        assert_eq!(parse_gender("male").unwrap(), Gender::Male);
        assert_eq!(parse_gender("F").unwrap(), Gender::Female);
        assert_eq!(parse_gender("any").unwrap(), Gender::Any);
        assert!(parse_gender("bogus").is_err());
    }

    #[test]
    fn logs_into_fake_gateway_end_to_end() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
            let login = read_frame(&mut stream);
            assert_eq!(login.id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
        });
        let report = connect_and_login(&test_options(port, Some((1, 2)))).unwrap();
        server.join().unwrap();
        assert_eq!(report.auth_code, 0);
        assert!(report.channels.is_empty());
        assert_eq!(
            report.channel_login,
            Some(ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            })
        );
    }

    #[test]
    fn refuses_failed_authorize() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let rejected = AuthResponse {
                result_code: 0xFFFF_FFFF,
                tail: vec![],
            };
            stream.write_all(&rejected.encode()).unwrap();
        });
        let outcome = connect_and_login(&test_options(port, None));
        server.join().unwrap();
        // Debug format carries the whole anyhow chain (`to_string`
        // shows only the outer context).
        let message = format!("{:?}", outcome.unwrap_err());
        assert!(message.contains("authorize rejected"), "{message}");
        assert!(message.contains("InvalidLogin"), "{message}");
    }
}
