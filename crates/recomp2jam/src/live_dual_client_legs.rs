#![cfg(test)]

//! Deterministic two-client legs for Room, Waiting and Loading fan-out.
//!
//! One loopback listener serves both Rust clients. Every leg boots through
//! the real connect entry and session assembly. Missing `install-root`
//! archives skip the leg cleanly (#0a3ba).

pub(crate) mod harness {
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use o2jam_net::frame::decode_frame;
    use o2jam_net::testing::read_frame;
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, Frame, JoinRoomResponse, Request,
        RoomList, WireMessage,
    };
    use o2jam_render::{Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::screens::paint::ChannelChatMessage;
    use o2jam_shell::{GameLoop, Keymap, Screen, ScreenGame};
    use winit::keyboard::KeyCode;

    use crate::live_leg_support::{
        channel_row, dismiss_splash, enter_room_by_clicks, join_waiting_by_click, paint,
        read_frame_until_eof, room_row, route_raw_key,
    };
    use crate::{
        assemble_online_session, connect, session::OnlineSession,
        stage_channel_list_after_relay_gate, stage_online_identity, stage_online_rooms,
    };

    /// One listener serves exactly two Rust clients in every leg.
    pub(crate) const CLIENT_COUNT: usize = 2;

    /// A slow guest paint must not end the server's read loop (#592 review).
    const GATEWAY_READ_TIMEOUT: Duration = Duration::from_secs(300);
    const SIGNAL_WAIT: Duration = Duration::from_secs(300);
    const POLL_STEP: Duration = Duration::from_millis(2);

    /// Shared scenario vocabulary. These names are test-owned literals,
    /// not values read from the 7.55 client.
    pub(crate) const ROOM_NUMBER: i32 = 3;
    pub(crate) const MASTER_MEMBER_ID: u8 = 0;
    pub(crate) const GUEST_MEMBER_ID: u8 = 1;
    pub(crate) const MASTER_NAME: &str = "Alpha";
    pub(crate) const GUEST_NAME: &str = "Beta";
    pub(crate) const WHISPER_SENDER: &str = MASTER_NAME;
    pub(crate) const WHISPER_TARGET: &str = "beta";
    pub(crate) const WHISPER_CONTENT: &str = "psst";
    const BOOT_USER_ID_BASE: i32 = 7;

    pub(crate) const DISBAND_OPCODE: u16 = 2007;
    pub(crate) const WHISPER_SEND_OPCODE: u16 = 2017;
    pub(crate) const WHISPER_RECEIVE_OPCODE: u16 = 2019;
    pub(crate) const SLOT_UPDATE_OPCODE: u16 = 3009;
    pub(crate) const START_REQUEST_OPCODE: u16 = 4010;
    pub(crate) const START_ANSWER_OPCODE: u16 = 4011;
    pub(crate) const LOADED_CONFIRM_OPCODE: u16 = 4012;
    pub(crate) const MEMBER_STATE_OPCODE: u16 = 4026;
    pub(crate) const MUSIC_STATE_OPCODE: u16 = 4037;
    pub(crate) const ACQUIRE_MUSIC_OPCODE: u16 = 4035;

    const ROOM_CHAT_ID: &str = "IDC_LIST_SROOM_CHAT";
    const DOWNLOAD_BADGE_SPRITE: &str = "Wait_Downloding.ojs";
    const SEAT_PARENT_PREFIX: &str = "IDC_BUTTON_SWAIT_EXTEND";

    /// Which side of the fake socket carried one frame.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Direction {
        ClientToServer,
        ServerToClient,
    }

    /// One frame in a client's wire leg, tagged with the harness label.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct WireRecord {
        pub(crate) direction: Direction,
        pub(crate) harness_tag: String,
        pub(crate) id: u16,
        pub(crate) payload: Vec<u8>,
    }

    #[derive(Clone)]
    struct LoggedFrame {
        harness_tag: String,
        frame: Frame,
    }

    struct ClientChannels {
        index: usize,
        script: Receiver<Vec<u8>>,
        written: Sender<()>,
        requests: Sender<u16>,
        join_written: Sender<()>,
        c2s: Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
        s2c: Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
        harness_tag: Arc<Mutex<String>>,
    }

    /// Server-side handles the reader loop needs after boot.
    struct ReaderChannels {
        index: usize,
        requests: Sender<u16>,
        join_written: Sender<()>,
        c2s: Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
        harness_tag: Arc<Mutex<String>>,
    }

    struct ClientLink {
        script: Sender<Vec<u8>>,
        written: Receiver<()>,
        requests: Receiver<u16>,
        join_written: Receiver<()>,
    }

    /// Fake Memoryer listener shared by both booted clients.
    pub(crate) struct DualGateway {
        port: u16,
        links: Vec<ClientLink>,
        c2s: Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
        s2c: Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
        harness_tag: Arc<Mutex<String>>,
        next_client: AtomicUsize,
        server: JoinHandle<()>,
    }

    impl DualGateway {
        /// Bind a loopback listener with one 3003 snapshot for both clients.
        pub(crate) fn spawn(joined: o2jam_net::JoinedRoom) -> Self {
            Self::spawn_with_rooms(vec![joined; CLIENT_COUNT])
        }

        /// Same listener with per-client snapshots, so master and guest can
        /// hold different member ids (#559, #592).
        pub(crate) fn spawn_with_rooms(rooms: Vec<o2jam_net::JoinedRoom>) -> Self {
            assert_eq!(rooms.len(), CLIENT_COUNT, "two joined snapshots");
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let c2s = Arc::new(Mutex::new(vec![Vec::new(); CLIENT_COUNT]));
            let s2c = Arc::new(Mutex::new(vec![Vec::new(); CLIENT_COUNT]));
            let harness_tag = Arc::new(Mutex::new("dual-client-room".to_owned()));
            let (links, channels) = Self::client_channels(&c2s, &s2c, &harness_tag);
            let server = serve_clients(listener, rooms, channels);
            Self {
                port,
                links,
                c2s,
                s2c,
                harness_tag,
                next_client: AtomicUsize::new(0),
                server,
            }
        }

        /// Process hosting the fake gateway's server threads.
        pub(crate) fn harness_process_id() -> u32 {
            std::process::id()
        }

        /// Label later wire records with a harness-local tag.
        pub(crate) fn set_harness_tag(&self, tag: &str) {
            *self.harness_tag.lock().unwrap() = tag.to_owned();
        }

        /// Boot one client through login, channel staging and assembly.
        pub(crate) fn boot_client(&self, name: &str) -> (usize, OnlineSession) {
            let index = self.next_client.fetch_add(1, Ordering::Relaxed);
            assert!(index < CLIENT_COUNT, "gateway serves two clients");
            let options = connect::ConnectOptions {
                host: "127.0.0.1".to_owned(),
                port: self.port(),
                user_id: i32::try_from(index).unwrap() + BOOT_USER_ID_BASE,
                token: name.to_owned(),
                gender: o2jam_net::Gender::Any,
                relay_key1: 0,
                relay_key2: 0,
                channel: Some((1, 2)),
                character: false,
                wallet: false,
                avatar_opa: None,
                join_room: None,
                room_password: String::new(),
                song: None,
            };
            let (report, gateway) = connect::connect_and_login_with_transport(&options).unwrap();
            let mut game = o2jam_shell::ScreenGame::new();
            stage_online_identity(&mut game, report.character.as_ref());
            stage_channel_list_after_relay_gate(&mut game, report.channels.clone(), false);
            stage_online_rooms(&mut game, report.rooms.clone());
            let mut session =
                assemble_online_session(game, gateway, Some(name), None, None, None).unwrap();
            session.set_resident_channel(channel_row());
            (index, session)
        }

        /// Push the same raw S2C frames to both sockets, in order.
        pub(crate) fn broadcast(&self, frames: &[Vec<u8>]) {
            for frame in frames {
                for link in &self.links {
                    link.script.send(frame.clone()).unwrap();
                }
                for link in &self.links {
                    link.written.recv_timeout(SIGNAL_WAIT).unwrap();
                }
            }
        }

        /// Wait for one C2S opcode on one socket. Frames stay in the log.
        pub(crate) fn wait_for_request(&self, client: usize, want: u16) {
            loop {
                let id = self.links[client]
                    .requests
                    .recv_timeout(SIGNAL_WAIT)
                    .unwrap_or_else(|_| panic!("client {client} never sent opcode {want}"));
                if id == want {
                    return;
                }
            }
        }

        /// Whether one C2S opcode is in the persisted log.
        fn logged_request(&self, client: usize, want: u16) -> bool {
            self.c2s.lock().unwrap()[client]
                .iter()
                .any(|logged| logged.frame.id == want)
        }

        /// Wait for a C2S opcode without consuming a request signal.
        pub(crate) fn wait_for_logged_request(&self, client: usize, want: u16) {
            let deadline = Instant::now() + SIGNAL_WAIT;
            while Instant::now() < deadline {
                if self.logged_request(client, want) {
                    return;
                }
                thread::sleep(POLL_STEP);
            }
            panic!("client {client} never logged opcode {want}");
        }

        /// Paint the client until its timed gate posts one C2S opcode.
        pub(crate) fn paint_until_logged_request(
            &self,
            session: &mut OnlineSession,
            client: usize,
            want: u16,
        ) {
            let deadline = Instant::now() + SIGNAL_WAIT;
            while Instant::now() < deadline {
                paint(session);
                if self.logged_request(client, want) {
                    return;
                }
                thread::sleep(POLL_STEP);
            }
            panic!("client {client} never posted opcode {want} while painting");
        }

        /// Both wire legs, ordered as the fake server saw them.
        pub(crate) fn wire_log(&self, client: usize) -> Vec<WireRecord> {
            let c2s = self.c2s.lock().unwrap();
            let s2c = self.s2c.lock().unwrap();
            c2s[client]
                .iter()
                .map(|logged| self.record(Direction::ClientToServer, logged))
                .chain(
                    s2c[client]
                        .iter()
                        .map(|logged| self.record(Direction::ServerToClient, logged)),
                )
                .collect()
        }

        fn record(&self, direction: Direction, logged: &LoggedFrame) -> WireRecord {
            WireRecord {
                direction,
                harness_tag: logged.harness_tag.clone(),
                id: logged.frame.id,
                payload: logged.frame.payload.clone(),
            }
        }

        /// Drop clients and script senders, then join the gateway threads.
        pub(crate) fn finish(self, sessions: Vec<OnlineSession>) {
            drop(sessions);
            let Self { links, server, .. } = self;
            drop(links);
            server.join().unwrap();
        }

        fn port(&self) -> u16 {
            self.port
        }

        fn client_channels(
            c2s: &Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
            s2c: &Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
            harness_tag: &Arc<Mutex<String>>,
        ) -> (Vec<ClientLink>, Vec<ClientChannels>) {
            (0..CLIENT_COUNT)
                .map(|index| {
                    let (script, script_rx) = mpsc::channel::<Vec<u8>>();
                    let (written_tx, written) = mpsc::channel::<()>();
                    let (requests_tx, requests) = mpsc::channel::<u16>();
                    let (join_tx, join_written) = mpsc::channel::<()>();
                    (
                        ClientLink {
                            script,
                            written,
                            requests,
                            join_written,
                        },
                        ClientChannels {
                            index,
                            script: script_rx,
                            written: written_tx,
                            requests: requests_tx,
                            join_written: join_tx,
                            c2s: Arc::clone(c2s),
                            s2c: Arc::clone(s2c),
                            harness_tag: Arc::clone(harness_tag),
                        },
                    )
                })
                .unzip()
        }

        /// 3003 write signal for one client's staged row click.
        fn join_signal(&self, client: usize) -> &Receiver<()> {
            &self.links[client].join_written
        }
    }

    /// Bind, accept two sockets and join both server handlers.
    fn serve_clients(
        listener: TcpListener,
        rooms: Vec<o2jam_net::JoinedRoom>,
        channels: Vec<ClientChannels>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut handlers = Vec::with_capacity(CLIENT_COUNT);
            for (index, channels) in channels.into_iter().enumerate() {
                let stream = accept_with_retry(&listener);
                let joined = rooms[index].clone();
                handlers.push(thread::spawn(move || {
                    serve_client(stream, joined, channels)
                }));
            }
            for handler in handlers {
                handler.join().unwrap();
            }
        })
    }

    fn accept_with_retry(listener: &TcpListener) -> TcpStream {
        loop {
            match listener.accept() {
                Ok((stream, _)) => return stream,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => panic!("dual gateway accept failed: {error}"),
            }
        }
    }

    /// One accepted socket's boot and steady-state server loop.
    fn serve_client(
        mut stream: TcpStream,
        joined: o2jam_net::JoinedRoom,
        channels: ClientChannels,
    ) {
        stream.set_read_timeout(Some(GATEWAY_READ_TIMEOUT)).unwrap();
        answer_boot(&mut stream, &joined, channels);
    }

    /// The shared 1000/1001/1002/1003 exchange for one socket.
    fn answer_boot(
        stream: &mut TcpStream,
        joined: &o2jam_net::JoinedRoom,
        channels: ClientChannels,
    ) {
        assert_eq!(read_frame(stream).id, Request::Authorize.id());
        stream
            .write_all(
                &AuthResponse {
                    result_code: 0,
                    tail: Vec::new(),
                }
                .encode(),
            )
            .unwrap();
        assert_eq!(read_frame(stream).id, Request::GetChannelList.id());
        stream
            .write_all(
                &ChannelList {
                    channels: vec![channel_row()],
                }
                .encode(),
            )
            .unwrap();
        assert_eq!(read_frame(stream).id, Request::ChannelLogin.id());
        stream
            .write_all(
                &ChannelLoginResponse {
                    result_code: 0,
                    restricted: Some(false),
                }
                .encode(),
            )
            .unwrap();
        assert_eq!(read_frame(stream).id, Request::GetChannelInfo.id());
        stream
            .write_all(
                &RoomList {
                    rooms: vec![room_row()],
                }
                .encode(),
            )
            .unwrap();
        start_writer(stream, channels, joined);
    }

    /// Start the scripted S2C writer, then read C2S frames until EOF.
    fn start_writer(
        stream: &mut TcpStream,
        channels: ClientChannels,
        joined: &o2jam_net::JoinedRoom,
    ) {
        let ClientChannels {
            index,
            script,
            written,
            requests,
            join_written,
            c2s,
            s2c,
            harness_tag,
        } = channels;
        let writer_tag = Arc::clone(&harness_tag);
        let writer_log = Arc::clone(&s2c);
        let mut writer_stream = stream.try_clone().unwrap();
        let writer = thread::spawn(move || {
            for frame in script {
                let decoded = decode_frame(&frame).expect("scripted S2C frame decodes");
                writer_stream.write_all(&frame).unwrap();
                let tag = { writer_tag.lock().unwrap().clone() };
                writer_log.lock().unwrap()[index].push(LoggedFrame {
                    harness_tag: tag,
                    frame: decoded,
                });
                written.send(()).unwrap();
            }
        });
        serve_reader(
            stream,
            joined,
            ReaderChannels {
                index,
                requests,
                join_written,
                c2s,
                harness_tag,
            },
            writer,
        );
    }

    /// Record and dispatch every C2S frame after boot.
    fn serve_reader(
        stream: &mut TcpStream,
        joined: &o2jam_net::JoinedRoom,
        channels: ReaderChannels,
        writer: JoinHandle<()>,
    ) {
        while let Some(frame) = read_frame_until_eof(stream) {
            let tag = { channels.harness_tag.lock().unwrap().clone() };
            channels.c2s.lock().unwrap()[channels.index].push(LoggedFrame {
                harness_tag: tag,
                frame: frame.clone(),
            });
            let _ = channels.requests.send(frame.id);
            answer_join_when_asked(stream, joined, &channels, frame.id);
        }
        writer.join().unwrap();
    }

    fn answer_join_when_asked(
        stream: &mut TcpStream,
        joined: &o2jam_net::JoinedRoom,
        channels: &ReaderChannels,
        id: u16,
    ) {
        if id != Request::JoinWaiting.id() {
            return;
        }
        stream
            .write_all(
                &JoinRoomResponse {
                    result_code: 0,
                    room: Some(joined.clone()),
                }
                .encode(),
            )
            .unwrap();
        let _ = channels.join_written.send(());
    }

    /// Boot one client and walk the staged pointer path to the Room board.
    pub(crate) fn boot_room_client(gateway: &DualGateway, name: &str) -> (usize, OnlineSession) {
        let (index, mut session) = gateway.boot_client(name);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        (index, session)
    }

    /// Boot one client and join the staged Waiting room by row click.
    pub(crate) fn boot_waiting_client(gateway: &DualGateway, name: &str) -> (usize, OnlineSession) {
        let (index, mut session) = boot_room_client(gateway, name);
        join_waiting_by_click(&mut session, gateway.join_signal(index));
        (index, session)
    }

    /// Paint until one condition holds; the predicate cannot mutate state.
    pub(crate) fn paint_until(session: &mut OnlineSession, ready: impl Fn(&OnlineSession) -> bool) {
        for _ in 0..crate::live_leg_support::MAX_SCREEN_PAINTS {
            paint(session);
            if ready(&*session) {
                return;
            }
        }
        panic!("two-client leg never staged the broadcast fields");
    }

    /// Paint until one read-only shell condition holds.
    pub(crate) fn paint_until_game(
        session: &mut OnlineSession,
        ready: impl Fn(&ScreenGame) -> bool,
    ) {
        for _ in 0..crate::live_leg_support::MAX_SCREEN_PAINTS {
            paint(session);
            let game: &ScreenGame = session.game_mut();
            if ready(game) {
                return;
            }
        }
        panic!("two-client leg never staged the shell fields");
    }

    /// Count one direction/opcode pair in one client's wire leg.
    pub(crate) fn count_records(log: &[WireRecord], direction: Direction, id: u16) -> usize {
        log.iter()
            .filter(|record| record.direction == direction && record.id == id)
            .count()
    }

    /// Find one direction/opcode pair in one client's wire leg.
    pub(crate) fn find_record(log: &[WireRecord], direction: Direction, id: u16) -> &WireRecord {
        log.iter()
            .find(|record| record.direction == direction && record.id == id)
            .unwrap_or_else(|| panic!("no {direction:?} {id} record in the wire leg"))
    }

    /// Native pixels that differ between two painted frames.
    pub(crate) fn changed_pixels(before: &[u32], after: &[u32]) -> usize {
        before
            .iter()
            .zip(after)
            .filter(|(was, now)| was != now)
            .count()
    }

    /// Absolute staged rect of one Waiting seat's DOWNLOADING sprite.
    pub(crate) fn waiting_badge_rect(session: &mut OnlineSession, seat: u8) -> Rect {
        let parent_id = format!("{SEAT_PARENT_PREFIX}{}", seat + 1);
        session
            .game_mut()
            .scene_placements(Screen::Waiting)
            .iter()
            .find(|placed| {
                placed.id == parent_id && placed.sprite.as_deref() == Some(DOWNLOAD_BADGE_SPRITE)
            })
            .map(|placed| placed.rect)
            .unwrap_or_else(|| panic!("no staged DOWNLOADING sprite for {parent_id}"))
    }

    /// Absolute staged rect of the Room chat list.
    pub(crate) fn room_chat_rect(session: &mut OnlineSession) -> Rect {
        session
            .game_mut()
            .scene_placements(Screen::Room)
            .iter()
            .find(|placed| placed.id == ROOM_CHAT_ID)
            .map(|placed| placed.rect)
            .unwrap_or_else(|| panic!("no staged Room chat list"))
    }

    /// Drive the real Room chat grammar: `/w Beta psst`, then Enter.
    pub(crate) fn send_room_whisper(session: &mut OnlineSession) {
        let mut keymap = Keymap::new();
        for (code, text) in whisper_keys() {
            let _ = route_raw_key(session, &mut keymap, *code, Some(text));
        }
        let _ = route_raw_key(session, &mut keymap, KeyCode::Enter, None);
    }

    fn whisper_keys() -> &'static [(KeyCode, &'static str)] {
        &[
            (KeyCode::Slash, "/"),
            (KeyCode::KeyW, "w"),
            (KeyCode::Space, " "),
            (KeyCode::KeyB, "b"),
            (KeyCode::KeyE, "e"),
            (KeyCode::KeyT, "t"),
            (KeyCode::KeyA, "a"),
            (KeyCode::Space, " "),
            (KeyCode::KeyP, "p"),
            (KeyCode::KeyS, "s"),
            (KeyCode::KeyS, "s"),
            (KeyCode::KeyT, "t"),
        ]
    }

    /// Every record in each leg must carry the one harness tag.
    pub(crate) fn assert_harness_tag(legs: &[Vec<WireRecord>], tag: &str) {
        for (index, log) in legs.iter().enumerate() {
            assert!(
                log.iter().all(|record| record.harness_tag == tag),
                "client {index} has records outside harness tag {tag}"
            );
        }
    }

    /// One native frame from the session paint path.
    pub(crate) fn paint_frame(session: &mut OnlineSession) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).unwrap();
        let height = usize::try_from(NATIVE_HEIGHT).unwrap();
        let mut buf = vec![0u32; width * height];
        session.paint(&mut o2jam_render::Frame {
            buf: &mut buf,
            w: width,
            h: height,
        });
        buf
    }

    /// Pixels in one absolute staged rect from an already painted frame.
    pub(crate) fn rect_pixels(frame: &[u32], rect: Rect) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).unwrap();
        let top = usize::try_from(rect.y).unwrap();
        let left = usize::try_from(rect.x).unwrap();
        let rows = usize::try_from(rect.h).unwrap();
        let columns = usize::try_from(rect.w).unwrap();
        (top..top + rows)
            .flat_map(|y| (left..left + columns).map(move |x| (y, x)))
            .map(|(y, x)| frame[y * width + x])
            .collect()
    }

    /// Per-pixel change mask for one staged rect against a rest frame.
    pub(crate) fn changed_mask(frame: &[u32], rest: &[u32], rect: Rect) -> Vec<bool> {
        rect_pixels(frame, rect)
            .iter()
            .zip(rest)
            .map(|(now, was)| now != was)
            .collect()
    }

    /// Chat history of one client's Room panel.
    pub(crate) fn room_chat(session: &mut OnlineSession) -> Vec<ChannelChatMessage> {
        session.game_mut().room_chat().to_vec()
    }
}

#[cfg(test)]
mod live_dual_client_leg_tests {
    use o2jam_net::frame::encode_frame;
    use o2jam_net::message::MemberStateChanged;
    use o2jam_net::{
        AcquireMusic, MemberMusicState, MusicState, ReceiveWhisper, RoomRemoved, RoomSlotAction,
        RoomSlotUpdate, SendWhisper, SyncMemberMusicStateEvent, WireMessage,
    };
    use o2jam_shell::screens::game::StartGateVerdict;
    use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

    use super::harness::{
        assert_harness_tag, boot_room_client, boot_waiting_client, changed_mask, changed_pixels,
        count_records, find_record, paint_frame, paint_until, paint_until_game, room_chat,
        room_chat_rect, send_room_whisper, waiting_badge_rect, Direction, DualGateway,
        ACQUIRE_MUSIC_OPCODE, DISBAND_OPCODE, GUEST_MEMBER_ID, GUEST_NAME, LOADED_CONFIRM_OPCODE,
        MASTER_MEMBER_ID, MASTER_NAME, MEMBER_STATE_OPCODE, MUSIC_STATE_OPCODE, ROOM_NUMBER,
        SLOT_UPDATE_OPCODE, START_ANSWER_OPCODE, START_REQUEST_OPCODE, WHISPER_CONTENT,
        WHISPER_RECEIVE_OPCODE, WHISPER_SENDER, WHISPER_SEND_OPCODE, WHISPER_TARGET,
    };
    use crate::live_leg_support::{archives_present, joined_room, paint, staged_rect_pixels};

    const LOADING_HARNESS_TAG: &str = "dual-loading-4010-4011-4012";
    // Harness label only, not a field read from the 7.55 client.
    // An independent Memoryer PID and epoch stay OPEN.
    const HARNESS_RUN_ORDINAL: u64 = 1;

    /// Skip notice used by every Tier 3 leg in this file.
    fn skip_if_no_archives(guard: &str) -> bool {
        if archives_present() {
            return false;
        }
        println!("SKIP {guard}: missing install-root interface archive");
        true
    }

    /// #967 Tier 3. Live chain: `main.rs::run_play` → connect → assembly →
    /// pointer clicks into Room → `OnlineSession::paint` → `poll_lobby_chat`
    /// → `apply_room_removed` → `ScreenGame::remove_room` → `paint_room_list`.
    #[test]
    fn two_room_clients_drop_the_disbanded_row() {
        let guard = "two_room_clients_drop_the_disbanded_row";
        if skip_if_no_archives(guard) {
            return;
        }
        let gateway = DualGateway::spawn(joined_room(MASTER_MEMBER_ID, true));
        gateway.set_harness_tag(guard);
        let (alpha_index, mut alpha) = boot_room_client(&gateway, MASTER_NAME);
        let (beta_index, mut beta) = boot_room_client(&gateway, GUEST_NAME);

        let alpha_before = paint_frame(&mut alpha);
        let beta_before = paint_frame(&mut beta);
        assert!(alpha
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|r| r.number == ROOM_NUMBER));
        assert!(beta
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|r| r.number == ROOM_NUMBER));

        gateway.broadcast(&[RoomRemoved {
            number: ROOM_NUMBER,
        }
        .encode()]);
        let alpha_log = gateway.wire_log(alpha_index);
        let beta_log = gateway.wire_log(beta_index);
        assert_eq!(
            count_records(&alpha_log, Direction::ServerToClient, DISBAND_OPCODE),
            1
        );
        assert_eq!(
            count_records(&beta_log, Direction::ServerToClient, DISBAND_OPCODE),
            1,
            "both observer clients must log the disband broadcast"
        );

        let row_gone = |game: &ScreenGame| {
            game.listed_rooms()
                .iter()
                .all(|room| room.number != ROOM_NUMBER)
        };
        paint_until_game(&mut alpha, row_gone);
        paint_until_game(&mut beta, row_gone);
        assert!(changed_pixels(&alpha_before, &paint_frame(&mut alpha)) > 0);
        assert!(changed_pixels(&beta_before, &paint_frame(&mut beta)) > 0);
        assert_eq!(
            find_record(&alpha_log, Direction::ServerToClient, DISBAND_OPCODE).payload,
            ROOM_NUMBER.to_le_bytes()
        );
        assert_eq!(
            find_record(&beta_log, Direction::ServerToClient, DISBAND_OPCODE).payload,
            ROOM_NUMBER.to_le_bytes()
        );
        assert_harness_tag(&[alpha_log, beta_log], guard);
        gateway.finish(vec![alpha, beta]);
    }

    /// #974 Tier 3. Live chain: Room chat keys → `flush_room_chat_action`
    /// → C2S 2017 → fake Memoryer → S2C 2019 to both → `poll_lobby_chat`
    /// → `apply_lobby_whisper` → `paint_room_chat`. The painted sender is
    /// the wire sender; our client has no friend-record display-name lookup
    /// (docs/pins/net-b.md:338-344), so that lookup stays OPEN.
    #[test]
    fn one_whisper_reaches_both_room_chat_lists() {
        let guard = "one_whisper_reaches_both_room_chat_lists";
        if skip_if_no_archives(guard) {
            return;
        }
        let gateway = DualGateway::spawn(joined_room(MASTER_MEMBER_ID, true));
        gateway.set_harness_tag(guard);
        let (alpha_index, mut alpha) = boot_room_client(&gateway, MASTER_NAME);
        let (beta_index, mut beta) = boot_room_client(&gateway, GUEST_NAME);

        let alpha_list = room_chat_rect(&mut alpha);
        let beta_list = room_chat_rect(&mut beta);
        let alpha_rest = staged_rect_pixels(&mut alpha, alpha_list);
        let beta_rest = staged_rect_pixels(&mut beta, beta_list);
        send_room_whisper(&mut alpha);
        gateway.wait_for_request(alpha_index, WHISPER_SEND_OPCODE);
        let alpha_sent_log = gateway.wire_log(alpha_index);
        let sent = find_record(
            &alpha_sent_log,
            Direction::ClientToServer,
            WHISPER_SEND_OPCODE,
        );
        let whisper = SendWhisper::decode_body(&sent.payload).unwrap();
        assert_eq!(whisper.recipient, WHISPER_TARGET);
        assert_eq!(whisper.content, WHISPER_CONTENT);
        gateway.broadcast(&[ReceiveWhisper {
            sender: WHISPER_SENDER.to_owned(),
            content: WHISPER_CONTENT.to_owned(),
        }
        .encode()]);
        let alpha_log = gateway.wire_log(alpha_index);
        let beta_log = gateway.wire_log(beta_index);
        assert_eq!(
            count_records(
                &alpha_log,
                Direction::ServerToClient,
                WHISPER_RECEIVE_OPCODE
            ),
            1
        );
        assert_eq!(
            count_records(&beta_log, Direction::ServerToClient, WHISPER_RECEIVE_OPCODE),
            1,
            "the whisper must reach both painted clients"
        );

        let one_row = |game: &ScreenGame| game.room_chat().len() == 1;
        paint_until_game(&mut alpha, one_row);
        paint_until_game(&mut beta, one_row);
        let alpha_row = room_chat(&mut alpha).remove(0);
        let beta_row = room_chat(&mut beta).remove(0);
        assert_eq!(alpha_row.sender, WHISPER_SENDER);
        assert_eq!(beta_row.sender, WHISPER_SENDER);
        assert_eq!(alpha_row.whisper_recipient.as_deref(), Some(MASTER_NAME));
        assert_eq!(beta_row.whisper_recipient.as_deref(), Some(GUEST_NAME));
        assert!(changed_pixels(&alpha_rest, &staged_rect_pixels(&mut alpha, alpha_list)) > 0);
        assert!(changed_pixels(&beta_rest, &staged_rect_pixels(&mut beta, beta_list)) > 0);
        assert_harness_tag(&[alpha_log, beta_log], guard);
        gateway.finish(vec![alpha, beta]);
    }

    /// #559 Tier 3. Live chain: Waiting join → `poll_waiting_chat` → 4035
    /// guest badge arm / 4026 / 4037 / 3009 → `paint_waiting_seat_marks`
    /// → `start_gate_verdict`. 4026/4037/3009 stay wire-only; 4035 is the
    /// pinned paint arm. Two Rust frames are compared to each other only;
    /// comparison to the exe A/B badge slice stays OPEN.
    #[test]
    fn master_and_guest_agree_on_badge_and_start_gate() {
        let guard = "master_and_guest_agree_on_badge_and_start_gate";
        if skip_if_no_archives(guard) {
            return;
        }
        let gateway = DualGateway::spawn_with_rooms(vec![
            joined_room(MASTER_MEMBER_ID, true),
            joined_room(GUEST_MEMBER_ID, true),
        ]);
        gateway.set_harness_tag(guard);
        let (master_index, mut master) = boot_waiting_client(&gateway, MASTER_NAME);
        let (guest_index, mut guest) = boot_waiting_client(&gateway, GUEST_NAME);

        let master_own_rect = waiting_badge_rect(&mut master, MASTER_MEMBER_ID);
        let guest_own_rect = waiting_badge_rect(&mut guest, GUEST_MEMBER_ID);
        let master_peer_state = MemberStateChanged {
            member_id: MASTER_MEMBER_ID,
            playing_state: 0,
            music_state: MusicState::Downloading.code(),
        };
        let guest_peer_music = SyncMemberMusicStateEvent {
            member_id: GUEST_MEMBER_ID,
            state: MusicState::Downloading,
        };
        let slot = RoomSlotUpdate {
            index: GUEST_MEMBER_ID,
            action_code: RoomSlotAction::SlotUnlocked as u8,
        };
        gateway.broadcast(&[
            master_peer_state.encode(),
            guest_peer_music.encode(),
            slot.encode(),
        ]);

        for (index, log) in [
            gateway.wire_log(master_index),
            gateway.wire_log(guest_index),
        ]
        .iter()
        .enumerate()
        {
            let sequence: Vec<u16> = log
                .iter()
                .filter(|record| record.direction == Direction::ServerToClient)
                .filter(|record| {
                    [MEMBER_STATE_OPCODE, MUSIC_STATE_OPCODE, SLOT_UPDATE_OPCODE]
                        .contains(&record.id)
                })
                .map(|record| record.id)
                .collect();
            assert_eq!(
                sequence,
                vec![MEMBER_STATE_OPCODE, MUSIC_STATE_OPCODE, SLOT_UPDATE_OPCODE],
                "client {index} missed the wire sequence"
            );
        }

        paint_until_game(&mut master, |game| {
            game.member_music_state(GUEST_MEMBER_ID) == Some(MusicState::Downloading.code())
        });
        paint_until_game(&mut guest, |game| {
            game.member_music_state(MASTER_MEMBER_ID) == Some(MusicState::Downloading.code())
        });
        assert_eq!(
            master.slot_action(GUEST_MEMBER_ID),
            Some(RoomSlotAction::SlotUnlocked)
        );
        assert_eq!(
            guest.slot_action(GUEST_MEMBER_ID),
            Some(RoomSlotAction::SlotUnlocked)
        );

        // Rest frames are taken after the wire states stage, before the 4035 arm.
        let master_own_rest = staged_rect_pixels(&mut master, master_own_rect);
        let guest_master_rest = staged_rect_pixels(&mut guest, master_own_rect);
        let guest_own_rest = staged_rect_pixels(&mut guest, guest_own_rect);

        let downloading = AcquireMusic {
            members: vec![MemberMusicState {
                member_id: GUEST_MEMBER_ID,
                state: MusicState::Downloading,
            }],
        };
        gateway.broadcast(&[downloading.encode()]);
        // Paint both clients so each consumes the guest-targeted 4035.
        paint(&mut master);
        paint(&mut guest);
        assert!(
            !master.game_mut().local_download_badge_armed(),
            "a guest-targeted 4035 must not arm the master seat"
        );
        assert!(
            guest.game_mut().local_download_badge_armed(),
            "a guest-targeted 4035 must arm the guest seat"
        );

        for (index, log) in [
            gateway.wire_log(master_index),
            gateway.wire_log(guest_index),
        ]
        .iter()
        .enumerate()
        {
            assert_eq!(
                count_records(log, Direction::ServerToClient, ACQUIRE_MUSIC_OPCODE),
                1,
                "client {index} must receive the guest-only 4035 paint arm"
            );
        }

        assert_eq!(
            staged_rect_pixels(&mut master, master_own_rect),
            master_own_rest
        );
        assert_ne!(
            staged_rect_pixels(&mut guest, guest_own_rect),
            guest_own_rest
        );
        // Two Rust frames agree on the master seat's change mask. Raw pixels
        // differ by role, and comparison to the exe A/B crop stays OPEN.
        assert_eq!(
            changed_mask(&paint_frame(&mut master), &master_own_rest, master_own_rect),
            changed_mask(
                &paint_frame(&mut guest),
                &guest_master_rest,
                master_own_rect
            ),
            "the two Rust frames must agree before the verdict"
        );
        for session in [&mut master, &mut guest] {
            assert_eq!(
                session.game_mut().start_gate_verdict(),
                StartGateVerdict::MusicDownloading
            );
        }

        let ready = AcquireMusic {
            members: vec![
                MemberMusicState {
                    member_id: MASTER_MEMBER_ID,
                    state: MusicState::Ready,
                },
                MemberMusicState {
                    member_id: GUEST_MEMBER_ID,
                    state: MusicState::Ready,
                },
            ],
        };
        let ready_peer_state = MemberStateChanged {
            member_id: GUEST_MEMBER_ID,
            playing_state: 0,
            music_state: MusicState::Ready.code(),
        };
        let ready_peer_music = SyncMemberMusicStateEvent {
            member_id: MASTER_MEMBER_ID,
            state: MusicState::Ready,
        };
        gateway.broadcast(&[
            ready.encode(),
            ready_peer_state.encode(),
            ready_peer_music.encode(),
        ]);
        let cleared = |game: &ScreenGame| {
            !game.local_download_badge_armed()
                && game.start_gate_verdict() == StartGateVerdict::Startable
        };
        paint_until_game(&mut master, cleared);
        paint_until_game(&mut guest, cleared);
        assert_eq!(
            staged_rect_pixels(&mut guest, guest_own_rect),
            guest_own_rest
        );
        assert_eq!(
            staged_rect_pixels(&mut master, master_own_rect),
            master_own_rest
        );
        assert_harness_tag(
            &[
                gateway.wire_log(master_index),
                gateway.wire_log(guest_index),
            ],
            guard,
        );
        gateway.finish(vec![master, guest]);
    }

    /// #592 Tier 3. Both painted clients reach Loading from one 4010/4011,
    /// then both log their own C2S 4012. `harness_tag` is a harness label
    /// only; a reserved trace id and an independent Memoryer PID/epoch stay
    /// OPEN.
    #[test]
    fn both_clients_reach_loading_and_log_4012() {
        let guard = "both_clients_reach_loading_and_log_4012";
        if skip_if_no_archives(guard) {
            return;
        }
        let gateway = DualGateway::spawn_with_rooms(vec![
            joined_room(MASTER_MEMBER_ID, true),
            joined_room(GUEST_MEMBER_ID, true),
        ]);
        gateway.set_harness_tag(LOADING_HARNESS_TAG);
        println!(
            "[{LOADING_HARNESS_TAG}] harness_process_id={} harness_run_ordinal={HARNESS_RUN_ORDINAL} live leg",
            DualGateway::harness_process_id()
        );
        let (master_index, mut master) = boot_waiting_client(&gateway, MASTER_NAME);
        let (guest_index, mut guest) = boot_waiting_client(&gateway, GUEST_NAME);

        master.menu_key(MenuKey::Confirm, true);
        gateway.wait_for_request(master_index, START_REQUEST_OPCODE);
        gateway.broadcast(&[encode_frame(START_ANSWER_OPCODE, &[])]);
        assert_eq!(
            count_records(
                &gateway.wire_log(master_index),
                Direction::ServerToClient,
                START_ANSWER_OPCODE
            ),
            1
        );
        assert_eq!(
            count_records(
                &gateway.wire_log(guest_index),
                Direction::ServerToClient,
                START_ANSWER_OPCODE
            ),
            1
        );
        paint_until(&mut master, |session| session.screen() == Screen::Loading);
        paint_until(&mut guest, |session| session.screen() == Screen::Loading);

        master.menu_key(MenuKey::Confirm, true);
        gateway.wait_for_logged_request(master_index, LOADED_CONFIRM_OPCODE);
        gateway.paint_until_logged_request(&mut guest, guest_index, LOADED_CONFIRM_OPCODE);
        let master_log = gateway.wire_log(master_index);
        let guest_log = gateway.wire_log(guest_index);
        for (index, log) in [&master_log, &guest_log].iter().enumerate() {
            assert_eq!(
                count_records(log, Direction::ClientToServer, LOADED_CONFIRM_OPCODE),
                1,
                "client {index} must log its own 4012"
            );
        }
        assert_harness_tag(&[master_log, guest_log], LOADING_HARNESS_TAG);
        println!(
            "[{LOADING_HARNESS_TAG}] harness_process_id={} harness_run_ordinal={HARNESS_RUN_ORDINAL} live 4012={}/{}",
            DualGateway::harness_process_id(),
            count_records(&gateway.wire_log(master_index), Direction::ClientToServer, LOADED_CONFIRM_OPCODE),
            count_records(&gateway.wire_log(guest_index), Direction::ClientToServer, LOADED_CONFIRM_OPCODE),
        );
        gateway.finish(vec![master, guest]);
    }
}
