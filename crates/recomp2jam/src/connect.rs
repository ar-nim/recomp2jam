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
    ChannelState, CharacterInfo, GatewayConnection, Gender, Request, RoomList, RoomListEntry,
    WireMessage,
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
    /// Fetch character info after authorizing.
    pub character: bool,
    /// `avatar.opa` path: stack the worn figure from the fetched
    /// equipment (`None` skips avatar work; needs `character`).
    pub avatar_opa: Option<String>,
}

/// What the login flow learned.
#[derive(Debug)]
pub struct ConnectReport {
    /// Authorize result code (always success — else this errors).
    pub auth_code: u32,
    /// Character info, when requested.
    pub character: Option<CharacterInfo>,
    /// Worn figure layers stacked from `avatar.opa`, when requested.
    pub avatar_layers: usize,
    /// Channels the gateway listed.
    pub channels: Vec<ChannelState>,
    /// Channel login answer, when a channel was requested.
    pub channel_login: Option<ChannelLoginResponse>,
    /// Room rows, fetched only after a channel login (the room
    /// controller rejects channel outsiders).
    pub rooms: Vec<RoomListEntry>,
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
    let rooms = if channel_login.is_some() {
        fetch_rooms(&mut gateway).context("listing rooms")?
    } else {
        Vec::new()
    };
    // Character info requires channel membership (server-verified:
    // `ChannelAuthorizeAttribute` rejects 2000 outside a channel),
    // so it runs last and only with a channel login.
    let character = if options.character {
        Some(fetch_character(&mut gateway).context("fetching character")?)
    } else {
        None
    };
    let avatar_layers = match (&options.avatar_opa, &character) {
        (Some(path), Some(hero)) => fetch_avatar(path, hero).context("loading avatar")?.len(),
        _ => 0,
    };
    Ok(ConnectReport {
        auth_code,
        character,
        avatar_layers,
        channels,
        channel_login,
        rooms,
    })
}

fn fetch_rooms(gateway: &mut GatewayConnection) -> Result<Vec<RoomListEntry>> {
    let frame = gateway.exchange(Request::GetChannelInfo.id(), &[], RoomList::COMMAND)?;
    Ok(RoomList::decode_frame(&frame)?.rooms)
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

/// Stack the worn figure for fetched equipment: read the local
/// `avatar.opa`, parse its item table, and layer every resolvable
/// part. `Any` gender wears the female parity (documented guess).
/// Stack a worn figure from a local `avatar.opa` and explicit
/// equipment ids (shared by `connect --avatar-opa` and
/// `play --window --avatar-opa --equip`).
pub fn build_worn_figure(
    opa_path: &str,
    equipment: &[i32],
    male: bool,
) -> Result<o2jam_shell::AvatarFigure> {
    let archive =
        std::fs::read(opa_path).with_context(|| format!("reading avatar archive {opa_path}"))?;
    let entries = o2jam_file::parse_opi(&archive)?;
    let table_entry = entries
        .iter()
        .find(|entry| entry.name == "Itemdata_Korea.dat")
        .with_context(|| "avatar.opa lacks Itemdata_Korea.dat")?;
    let table_bytes = archive
        .get(table_entry.data_offset..table_entry.data_offset + table_entry.size as usize)
        .with_context(|| "item table out of archive bounds")?;
    let table = o2jam_file::parse_avatar_item_table(table_bytes)?;
    Ok(o2jam_shell::load_worn_figure(
        &archive, &table, equipment, male,
    ))
}

fn fetch_avatar(opa_path: &str, hero: &CharacterInfo) -> Result<o2jam_shell::AvatarFigure> {
    build_worn_figure(opa_path, &hero.equipment, hero.gender == Gender::Male)
}

/// Parse `--equip` (`"1,5,9"`) into item ids; blank means none.
pub fn parse_equipment(list: &str) -> Result<Vec<i32>> {
    if list.trim().is_empty() {
        return Ok(Vec::new());
    }
    list.split(',')
        .map(|part| {
            part.trim()
                .parse::<i32>()
                .with_context(|| format!("bad item id '{part}'"))
        })
        .collect()
}

fn fetch_character(gateway: &mut GatewayConnection) -> Result<CharacterInfo> {
    let frame = gateway.exchange(Request::GetCharacterInfo.id(), &[], CharacterInfo::COMMAND)?;
    let info = CharacterInfo::decode_frame(&frame)?;
    if info.invalid {
        bail!("server flagged the account invalid");
    }
    Ok(info)
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
        frame::decode_frame, AuthResponse, ChannelList, ChannelLoginResponse, Frame, HEADER_LEN,
    };

    use super::*;

    /// Read one whole frame off the fake server's socket.
    fn read_frame(stream: &mut TcpStream) -> Frame {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
        let mut rest = vec![0u8; body_len - HEADER_LEN];
        stream.read_exact(&mut rest).unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&rest);
        decode_frame(&bytes).unwrap()
    }

    fn test_options(
        port: u16,
        channel: Option<(i16, i16)>,
        character: bool,
        avatar_opa: Option<String>,
    ) -> ConnectOptions {
        ConnectOptions {
            host: "127.0.0.1".to_owned(),
            port,
            user_id: 7,
            token: "tester".to_owned(),
            gender: Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel,
            character,
            avatar_opa,
        }
    }

    #[test]
    fn parses_equipment_lists() {
        assert_eq!(parse_equipment("").unwrap(), Vec::<i32>::new());
        assert_eq!(parse_equipment("1,5,9").unwrap(), vec![1, 5, 9]);
        assert_eq!(parse_equipment(" 2 , 3 ").unwrap(), vec![2, 3]);
        assert!(parse_equipment("1,nope").is_err());
    }

    #[test]
    fn builds_worn_figure_from_archive() {
        let archive = opa_blob(&[
            ("Itemdata_Korea.dat", &item_table_blob()),
            ("AA.ojs", &red_sprite()),
        ]);
        let dir = std::env::temp_dir();
        let path = dir.join("recomp2jam-test-build-avatar.opa");
        std::fs::write(&path, &archive).unwrap();
        let figure = build_worn_figure(&path.to_string_lossy(), &[1], false).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(figure.len(), 1);
        assert!(build_worn_figure("nonexistent.opa", &[1], false).is_err());
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
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
        });
        let report = connect_and_login(&test_options(port, Some((1, 2)), false, None)).unwrap();
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
    fn fetches_character_after_channel_login() {
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
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetCharacterInfo.id());
            let hero = CharacterInfo {
                nickname: "tester".to_owned(),
                gender: Gender::Male,
                level: 3,
                ..Default::default()
            };
            stream.write_all(&hero.encode()).unwrap();
        });
        let report = connect_and_login(&test_options(port, Some((1, 2)), true, None)).unwrap();
        server.join().unwrap();
        let hero = report.character.unwrap();
        assert_eq!(hero.nickname, "tester");
        assert_eq!(hero.level, 3);
        assert!(report.channel_login.is_some());
    }

    /// Minimal sprite-55 body: 1x1 red pixel, colorkey black.
    fn red_sprite() -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 1u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x7C]);
        blob
    }

    /// Minimal `.opa`: header + bodies + trailing entry table.
    fn opa_blob(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        let mut blob = vec![0u8; 16];
        blob[..4].copy_from_slice(&1u32.to_le_bytes());
        blob[4..8].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        let mut off = 16u32;
        let mut table = Vec::new();
        for (name, body) in bodies {
            let mut entry = vec![0u8; 152];
            entry[..4].copy_from_slice(&1u32.to_le_bytes());
            entry[4..4 + name.len()].copy_from_slice(name.as_bytes());
            entry[132..136].copy_from_slice(&off.to_le_bytes());
            entry[136..140].copy_from_slice(&(body.len() as u32).to_le_bytes());
            entry[140..144].copy_from_slice(&(body.len() as u32).to_le_bytes());
            table.push(entry);
            off += body.len() as u32;
        }
        let mut out = blob;
        for (_, body) in bodies {
            out.extend_from_slice(body);
        }
        for entry in table {
            out.extend_from_slice(&entry);
        }
        out
    }

    /// Minimal item table: one row (id 1) wearing `AA.oji`.
    fn item_table_blob() -> Vec<u8> {
        let mut row = Vec::new();
        row.extend_from_slice(&1u32.to_le_bytes());
        row.extend_from_slice(&0x106u32.to_le_bytes());
        row.extend_from_slice(&[0u8; 22]);
        row.extend_from_slice(b"\x04\0\0\0hair\x03\0\0\0dsc");
        let mut names: Vec<(u8, &str)> = vec![(1, ""); 42];
        names[0] = (1, "i.oji");
        names[1] = (1, "i.oji");
        names[2] = (1, "AA.oji");
        for (flag, name) in names {
            row.push(flag);
            row.extend_from_slice(&(name.len() as u32).to_le_bytes());
            row.extend_from_slice(name.as_bytes());
        }
        let mut blob = Vec::new();
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(&[0u8; 80]);
        blob.extend_from_slice(&row);
        blob
    }

    #[test]
    fn stacks_worn_avatar_from_local_archive() {
        let archive = opa_blob(&[
            ("Itemdata_Korea.dat", &item_table_blob()),
            ("AA.ojs", &red_sprite()),
        ]);
        let dir = std::env::temp_dir();
        let path = dir.join("recomp2jam-test-avatar.opa");
        std::fs::write(&path, &archive).unwrap();
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
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetCharacterInfo.id());
            let mut hero = CharacterInfo {
                nickname: "tester".to_owned(),
                ..Default::default()
            };
            hero.equipment[0] = 1;
            stream.write_all(&hero.encode()).unwrap();
        });
        let options = test_options(
            port,
            Some((1, 2)),
            true,
            Some(path.to_string_lossy().into_owned()),
        );
        let report = connect_and_login(&options).unwrap();
        server.join().unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(report.avatar_layers, 1);
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
        let outcome = connect_and_login(&test_options(port, None, false, None));
        server.join().unwrap();
        // Debug format carries the whole anyhow chain (`to_string`
        // shows only the outer context).
        let message = format!("{:?}", outcome.unwrap_err());
        assert!(message.contains("authorize rejected"), "{message}");
        assert!(message.contains("InvalidLogin"), "{message}");
    }
}
