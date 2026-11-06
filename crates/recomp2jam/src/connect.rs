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
    ChannelState, CharacterInfo, GatewayConnection, Gender, JoinRoom, JoinRoomResponse, Request,
    RoomList, RoomListEntry, WireMessage,
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
    /// Room number to join after the channel login (`None` skips;
    /// needs a channel).
    pub join_room: Option<i32>,
    /// Password for `--join-room` (empty when unlocked).
    pub room_password: String,
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
    /// Join answer, when a room was requested.
    pub join: Option<JoinRoomResponse>,
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
    let join = match (options.join_room, &channel_login) {
        (Some(number), Some(_)) => {
            Some(join_room(&mut gateway, number, &options.room_password).context("joining room")?)
        }
        _ => None,
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
        join,
        rooms,
    })
}

fn join_room(
    gateway: &mut GatewayConnection,
    number: i32,
    password: &str,
) -> Result<JoinRoomResponse> {
    let request = JoinRoom {
        number,
        music_state: 0,
        password: password.to_owned(),
    };
    let frame = gateway.exchange(
        Request::JoinWaiting.id(),
        &request.encode_body(),
        JoinRoomResponse::COMMAND,
    )?;
    let response = JoinRoomResponse::decode_frame(&frame)?;
    match o2jam_net::JoinResult::from_code(response.result_code) {
        Some(o2jam_net::JoinResult::Success) => Ok(response),
        _ => bail!("join rejected: {}", name_result(response.result_code)),
    }
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

/// Load one named sprite from an interface archive on disk.
pub fn load_interface_sprite(
    interface_path: &str,
    entry_name: &str,
) -> Result<o2jam_shell::AvatarFigure> {
    let archive = std::fs::read(interface_path)
        .with_context(|| format!("reading interface archive {interface_path}"))?;
    o2jam_shell::load_archive_sprite(&archive, entry_name)
        .with_context(|| format!("no decodable {entry_name} in {interface_path}"))
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
            join_room: None,
            room_password: String::new(),
        }
    }

    #[test]
    fn loads_interface_sprite_from_disk() {
        let archive = opa_blob(&[("Logo.ojs", &red_sprite())]);
        let dir = std::env::temp_dir();
        let path = dir.join("recomp2jam-test-interface.opi");
        std::fs::write(&path, &archive).unwrap();
        let figure = load_interface_sprite(&path.to_string_lossy(), "Logo.ojs").unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(figure.len(), 1);
        assert!(load_interface_sprite("nonexistent.opi", "Logo.ojs").is_err());
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
    fn joins_room_after_channel_login() {
        use o2jam_net::{JoinRoomResponse, JoinedRoom};
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
            let join = read_frame(&mut stream);
            assert_eq!(join.id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            let asked = read_frame(&mut stream);
            assert_eq!(asked.id, Request::JoinWaiting.id());
            let mut slots = Vec::new();
            for index in 0..7u8 {
                slots.push(o2jam_net::RoomSlot {
                    index,
                    state_code: 0,
                    member: None,
                });
            }
            let answered = JoinRoomResponse {
                result_code: 0,
                room: Some(JoinedRoom {
                    member_id: 0,
                    team: 0,
                    title: "Lobby".to_owned(),
                    music_id: 0,
                    arena: 0,
                    key_mode: 2,
                    game_mode: 0,
                    difficulty: 0,
                    speed: 1,
                    has_super_manager: false,
                    user_count: 1,
                    slots,
                    skills: vec![],
                    premium: false,
                    champion: None,
                    champion_streak: None,
                }),
            };
            stream.write_all(&answered.encode()).unwrap();
        });
        let mut options = test_options(port, Some((1, 2)), false, None);
        options.join_room = Some(3);
        let report = connect_and_login(&options).unwrap();
        server.join().unwrap();
        let room = report.join.unwrap().room.unwrap();
        assert_eq!(room.title, "Lobby");
        assert_eq!(room.user_count, 1);
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

/// Host song flow over real gateway frames (plan Phases 3–4).
///
/// Test-gated drivers for every new song opcode: production CLI
/// wiring (`connect` flags in `main.rs`) is a later step outside
/// this task's touch list, so the flow lives here as the
/// executable proof against a fake gateway.
#[cfg(test)]
mod song_flow_tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use o2jam_net::{
        frame::{decode_frame, encode_frame},
        ConfirmMusicLoaded, ExitPlaying, FinalizeRank, Frame, GameStatsUpdate, GatewayConnection,
        GetMusicList, Request, SetDownloadProgress, SetRoomMusic, StartGame, StatKind, SubmitScore,
        SyncMemberMusicState, UpdateGameStats, WireMessage, HEADER_LEN,
    };

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

    type NetResult<T> = Result<T, o2jam_net::Error>;

    fn send_select(gateway: &mut GatewayConnection) -> NetResult<()> {
        let select = SetRoomMusic {
            music_id: 757,
            difficulty: 2,
        };
        gateway.send_frame(Request::SetRoomMusic.id(), &select.encode_body())
    }

    fn fetch_music_list(gateway: &mut GatewayConnection) -> NetResult<Frame> {
        gateway.exchange(
            Request::GetMusicList.id(),
            &GetMusicList.encode_body(),
            music_list_answer(),
        )
    }

    /// `GetMusicList` 4031 answer id (body still opaque).
    fn music_list_answer() -> u16 {
        4031
    }

    fn send_sync(gateway: &mut GatewayConnection) -> NetResult<()> {
        let state = SyncMemberMusicState { music_state: 2 };
        gateway.send_frame(Request::SyncMemberMusicState.id(), &state.encode_body())?;
        let progress = SetDownloadProgress {
            progress: 100,
            member_id: 0,
        };
        gateway.send_frame(Request::SetDownloadProgress.id(), &progress.encode_body())
    }

    fn send_start(gateway: &mut GatewayConnection) -> NetResult<()> {
        gateway.send_frame(Request::StartGame.id(), &StartGame.encode_body())?;
        let loaded = ConfirmMusicLoaded {
            power_skill_id: 0,
            speed: 1,
        };
        gateway.send_frame(Request::ConfirmMusicLoaded.id(), &loaded.encode_body())
    }

    fn send_jam_tick(gateway: &mut GatewayConnection) -> NetResult<()> {
        let tick = UpdateGameStats {
            kind: StatKind::Jam,
            value: 3,
            sequence: 1,
            score: 210,
            long_note_score: 0,
        };
        gateway.send_frame(Request::UpdateGameStats.id(), &tick.encode_body())
    }

    fn await_peer_bar(gateway: &mut GatewayConnection) -> NetResult<GameStatsUpdate> {
        let frame = gateway.await_response(GameStatsUpdate::COMMAND)?;
        GameStatsUpdate::decode_frame(&frame)
    }

    fn send_tally(gateway: &mut GatewayConnection) -> NetResult<()> {
        let tally = SubmitScore {
            cool: 10,
            good: 2,
            bad: 0,
            miss: 0,
            max_combo: 12,
            jam: 1,
            max_jam: 1,
            score: 2210,
            life: 100,
            speed: 1,
            long_note_score: 0,
        };
        gateway.send_frame(Request::SubmitScore.id(), &tally.encode_body())?;
        gateway.send_frame(Request::FinalizeRank.id(), &FinalizeRank.encode_body())?;
        gateway.send_frame(Request::ExitPlaying.id(), &ExitPlaying.encode_body())
    }

    /// Drive select → sync → start → tick → score → rank → exit.
    fn drive_host_song_flow(gateway: &mut GatewayConnection) -> NetResult<(u16, GameStatsUpdate)> {
        send_select(gateway)?;
        let listed = fetch_music_list(gateway)?;
        send_sync(gateway)?;
        send_start(gateway)?;
        send_jam_tick(gateway)?;
        let peer = await_peer_bar(gateway)?;
        send_tally(gateway)?;
        Ok((listed.id, peer))
    }

    #[test]
    fn host_song_flow_reaches_score_over_fake_gateway() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::SetRoomMusic.id());
            assert_eq!(read_frame(&mut stream).id, Request::GetMusicList.id());
            stream
                .write_all(&encode_frame(music_list_answer(), &[]))
                .unwrap();
            assert_eq!(
                read_frame(&mut stream).id,
                Request::SyncMemberMusicState.id()
            );
            assert_eq!(
                read_frame(&mut stream).id,
                Request::SetDownloadProgress.id()
            );
            assert_eq!(read_frame(&mut stream).id, Request::StartGame.id());
            let loaded = read_frame(&mut stream);
            assert_eq!(loaded.id, Request::ConfirmMusicLoaded.id());
            assert_eq!(
                ConfirmMusicLoaded::decode_body(&loaded.payload).unwrap(),
                ConfirmMusicLoaded {
                    power_skill_id: 0,
                    speed: 1,
                }
            );
            let tick = read_frame(&mut stream);
            assert_eq!(tick.id, Request::UpdateGameStats.id());
            let stats = UpdateGameStats::decode_body(&tick.payload).unwrap();
            assert_eq!(stats.kind, StatKind::Jam);
            let echo = GameStatsUpdate {
                member_id: 0,
                kind_code: StatKind::Life.as_u8(),
                value: 1000,
                ranks: [0; 8],
            };
            stream.write_all(&echo.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::SubmitScore.id());
            assert_eq!(read_frame(&mut stream).id, Request::FinalizeRank.id());
            assert_eq!(read_frame(&mut stream).id, Request::ExitPlaying.id());
        });
        let mut gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let (listed_id, peer) = drive_host_song_flow(&mut gateway).unwrap();
        server.join().unwrap();
        assert_eq!(listed_id, music_list_answer());
        assert_eq!(peer.member_id, 0);
        assert_eq!(peer.kind(), Some(StatKind::Life));
        assert_eq!(peer.value, 1000);
    }
}
