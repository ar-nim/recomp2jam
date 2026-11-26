#![cfg(test)]

//! Deterministic two-client legs for Room, Waiting and Loading fan-out.
//!
//! Both clients boot through the real connect entry and session assembly;
//! one fake Memoryer listener owns both sockets and routes every broadcast.
//! Missing `install-root` archives skip these legs cleanly (#0a3ba).

pub(crate) mod harness {
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use crate::live_leg_support::{
        channel_row, dismiss_splash, enter_room_by_clicks, join_waiting_by_click, paint,
        read_frame_until_eof, room_row,
    };
    use crate::{
        assemble_online_session, connect, session::OnlineSession,
        stage_channel_list_after_relay_gate, stage_online_identity, stage_online_rooms,
    };
    use o2jam_net::frame::decode_frame;
    use o2jam_net::testing::read_frame;
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, Frame, JoinRoomResponse, Request,
        RoomList, WireMessage,
    };

    /// One listener serves exactly two Rust clients in every leg.
    pub(crate) const CLIENT_COUNT: usize = 2;

    const IO_WAIT: Duration = Duration::from_secs(10);

    /// Whichever side of the fake socket carried one frame.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum Direction {
        ClientToServer,
        ServerToClient,
    }

    /// One frame in a client's wire leg, tagged with the scenario trace.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct WireRecord {
        pub(crate) direction: Direction,
        pub(crate) trace_id: String,
        pub(crate) id: u16,
        pub(crate) payload: Vec<u8>,
    }

    #[derive(Clone)]
    struct LoggedFrame {
        trace_id: String,
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
        trace_id: Arc<Mutex<String>>,
    }

    /// Server-side handles the reader loop needs after boot.
    struct ReaderChannels {
        index: usize,
        requests: Sender<u16>,
        join_written: Sender<()>,
        c2s: Arc<Mutex<Vec<Vec<LoggedFrame>>>>,
        trace_id: Arc<Mutex<String>>,
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
        trace_id: Arc<Mutex<String>>,
        next_client: AtomicUsize,
        server: JoinHandle<()>,
    }

    impl DualGateway {
        /// Bind a loopback listener and start one server handler per client.
        pub(crate) fn spawn(joined: o2jam_net::JoinedRoom) -> Self {
            Self::spawn_with_rooms(vec![joined; CLIENT_COUNT])
        }

        /// Same listener with one 3003 room snapshot per client, so a
        /// master and a guest can share the fake gateway (#592).
        pub(crate) fn spawn_with_rooms(rooms: Vec<o2jam_net::JoinedRoom>) -> Self {
            assert_eq!(rooms.len(), CLIENT_COUNT, "two joined snapshots");
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let c2s: Arc<Mutex<Vec<Vec<LoggedFrame>>>> =
                Arc::new(Mutex::new(vec![Vec::new(); CLIENT_COUNT]));
            let s2c: Arc<Mutex<Vec<Vec<LoggedFrame>>>> =
                Arc::new(Mutex::new(vec![Vec::new(); CLIENT_COUNT]));
            let trace_id = Arc::new(Mutex::new("dual-client-room".to_owned()));

            let mut links = Vec::with_capacity(CLIENT_COUNT);
            let mut channels = Vec::with_capacity(CLIENT_COUNT);
            for index in 0..CLIENT_COUNT {
                let (script, script_rx) = mpsc::channel::<Vec<u8>>();
                let (written_tx, written) = mpsc::channel::<()>();
                let (requests_tx, requests) = mpsc::channel::<u16>();
                let (join_tx, join_written) = mpsc::channel::<()>();
                channels.push(ClientChannels {
                    index,
                    script: script_rx,
                    written: written_tx,
                    requests: requests_tx,
                    join_written: join_tx,
                    c2s: Arc::clone(&c2s),
                    s2c: Arc::clone(&s2c),
                    trace_id: Arc::clone(&trace_id),
                });
                links.push(ClientLink {
                    script,
                    written,
                    requests,
                    join_written,
                });
            }
            // Accept on the server thread: `spawn` must return before
            // either client can dial the listener (#926 boot pattern).
            let server = thread::spawn(move || {
                let mut handlers = Vec::with_capacity(CLIENT_COUNT);
                for (index, channels) in channels.into_iter().enumerate() {
                    let stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                                continue
                            }
                            Err(error) => panic!("dual gateway accept failed: {error}"),
                        }
                    };
                    let joined = rooms[index].clone();
                    handlers.push(thread::spawn(move || {
                        serve_client(stream, joined, channels)
                    }));
                }
                for handler in handlers {
                    handler.join().unwrap();
                }
            });
            Self {
                port,
                links,
                c2s,
                s2c,
                trace_id,
                next_client: AtomicUsize::new(0),
                server,
            }
        }

        /// Process hosting the fake gateway's server threads.
        pub(crate) fn server_pid() -> u32 {
            std::process::id()
        }

        /// Tag later wire records with one shared scenario trace id.
        pub(crate) fn set_trace_id(&self, trace_id: &str) {
            *self.trace_id.lock().unwrap() = trace_id.to_owned();
        }

        /// Boot one Rust client through login, channel staging and assembly.
        pub(crate) fn boot_client(&self, name: &str) -> (usize, OnlineSession) {
            let index = self.next_client.fetch_add(1, Ordering::Relaxed);
            assert!(index < CLIENT_COUNT, "gateway serves two clients");
            let options = connect::ConnectOptions {
                host: "127.0.0.1".to_owned(),
                port: self.port,
                user_id: i32::try_from(index).unwrap() + 7,
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

        /// 3003 write signal for one client's staged row click.
        pub(crate) fn join_signal(&self, client: usize) -> &Receiver<()> {
            &self.links[client].join_written
        }

        /// Push the same raw S2C frames to both sockets, in order.
        pub(crate) fn broadcast(&self, frames: &[Vec<u8>]) {
            for frame in frames {
                for link in &self.links {
                    link.script.send(frame.clone()).unwrap();
                }
                for link in &self.links {
                    link.written.recv_timeout(IO_WAIT).unwrap();
                }
            }
        }

        /// Wait for one C2S opcode on one socket. Frames stay in the log.
        pub(crate) fn wait_for_request(&self, client: usize, want: u16) {
            loop {
                let id = self.links[client]
                    .requests
                    .recv_timeout(IO_WAIT)
                    .unwrap_or_else(|_| panic!("client {client} never sent opcode {want}"));
                if id == want {
                    return;
                }
            }
        }

        /// Wait until one C2S opcode is present in the persisted wire log.
        /// The log, not a consumed signal, is the observation: a guard can
        /// inspect a request even when an earlier wait drained its signal.
        pub(crate) fn wait_for_logged_request(&self, client: usize, want: u16) {
            let deadline = Instant::now() + IO_WAIT;
            while Instant::now() < deadline {
                let seen = self.c2s.lock().unwrap()[client]
                    .iter()
                    .any(|logged| logged.frame.id == want);
                if seen {
                    return;
                }
                thread::sleep(Duration::from_millis(5));
            }
            panic!("client {client} never logged opcode {want}");
        }

        /// Both wire legs, ordered as the fake server saw them.
        pub(crate) fn wire_log(&self, client: usize) -> Vec<WireRecord> {
            let c2s = self.c2s.lock().unwrap();
            let s2c = self.s2c.lock().unwrap();
            c2s[client]
                .iter()
                .map(|logged| WireRecord {
                    direction: Direction::ClientToServer,
                    trace_id: logged.trace_id.clone(),
                    id: logged.frame.id,
                    payload: logged.frame.payload.clone(),
                })
                .chain(s2c[client].iter().map(|logged| WireRecord {
                    direction: Direction::ServerToClient,
                    trace_id: logged.trace_id.clone(),
                    id: logged.frame.id,
                    payload: logged.frame.payload.clone(),
                }))
                .collect()
        }

        /// Drop clients and script senders, then join the gateway threads.
        pub(crate) fn finish(self, sessions: Vec<OnlineSession>) {
            drop(sessions);
            let Self { links, server, .. } = self;
            drop(links);
            server.join().unwrap();
        }
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

        let ClientChannels {
            index,
            script,
            written,
            requests,
            join_written,
            c2s,
            s2c,
            trace_id,
        } = channels;
        let writer_trace = Arc::clone(&trace_id);
        let writer_s2c = Arc::clone(&s2c);
        let mut writer_stream = stream.try_clone().unwrap();
        let writer = thread::spawn(move || {
            for frame in script {
                let decoded = decode_frame(&frame).expect("scripted S2C frame decodes");
                writer_stream.write_all(&frame).unwrap();
                let trace = { writer_trace.lock().unwrap().clone() };
                writer_s2c.lock().unwrap()[index].push(LoggedFrame {
                    trace_id: trace,
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
                trace_id,
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
            let trace = { channels.trace_id.lock().unwrap().clone() };
            channels.c2s.lock().unwrap()[channels.index].push(LoggedFrame {
                trace_id: trace,
                frame: frame.clone(),
            });
            let _ = channels.requests.send(frame.id);
            if frame.id == Request::JoinWaiting.id() {
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
        }
        writer.join().unwrap();
    }

    /// One accepted socket's boot and steady-state server loop.
    fn serve_client(
        mut stream: TcpStream,
        joined: o2jam_net::JoinedRoom,
        channels: ClientChannels,
    ) {
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        answer_boot(&mut stream, &joined, channels);
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

    /// Paint until one boot-driven condition holds; no wall-clock sleeps.
    pub(crate) fn paint_until(
        session: &mut OnlineSession,
        ready: impl Fn(&mut OnlineSession) -> bool,
    ) {
        for _ in 0..crate::live_leg_support::MAX_SCREEN_PAINTS {
            paint(session);
            if ready(session) {
                return;
            }
        }
        panic!("two-client leg never staged the broadcast fields");
    }
}

#[cfg(test)]
mod live_dual_client_leg_tests {
    use o2jam_net::frame::encode_frame;
    use o2jam_net::message::MemberStateChanged;
    use o2jam_net::{
        AcquireMusic, MemberMusicState, MusicState, ReceiveWhisper, RoomRemoved, RoomSlotAction,
        RoomSlotUpdate, SyncMemberMusicStateEvent, WireMessage,
    };
    use o2jam_render::Rect;
    use o2jam_shell::screens::game::StartGateVerdict;
    use o2jam_shell::{Keymap, Screen};
    use winit::keyboard::KeyCode;

    use super::harness::{
        boot_room_client, boot_waiting_client, paint_until, Direction, DualGateway, WireRecord,
    };
    use crate::live_leg_support::{
        archives_present, joined_room, paint_frame, route_raw_key, staged_rect_pixels,
    };

    /// Count one direction/opcode pair in a client's wire leg.
    fn count(log: &[WireRecord], direction: Direction, id: u16) -> usize {
        log.iter()
            .filter(|record| record.direction == direction && record.id == id)
            .count()
    }

    /// First record of one direction/opcode pair, with its payload.
    fn payload_of(log: &[WireRecord], direction: Direction, id: u16) -> &[u8] {
        &log.iter()
            .find(|record| record.direction == direction && record.id == id)
            .unwrap_or_else(|| panic!("no {direction:?} {id} record in the wire leg"))
            .payload
    }

    /// Native pixels that differ between two painted frames.
    fn changed_pixels(before: &[u32], after: &[u32]) -> usize {
        before
            .iter()
            .zip(after)
            .filter(|(was, now)| was != now)
            .count()
    }

    /// Absolute staged rect of one Waiting seat's DOWNLOADING sprite.
    fn waiting_badge_rect(session: &mut crate::session::OnlineSession, seat: u8) -> Rect {
        let parent_id = format!("IDC_BUTTON_SWAIT_EXTEND{}", seat + 1);
        session
            .game_mut()
            .scene_placements(Screen::Waiting)
            .iter()
            .find(|placed| {
                placed.id == parent_id && placed.sprite.as_deref() == Some("Wait_Downloding.ojs")
            })
            .map(|placed| placed.rect)
            .unwrap_or_else(|| panic!("no staged DOWNLOADING sprite for {parent_id}"))
    }

    /// Absolute staged rect of the Room chat list.
    fn room_list_rect(session: &mut crate::session::OnlineSession) -> Rect {
        session
            .game_mut()
            .scene_placements(Screen::Room)
            .iter()
            .find(|placed| placed.id == "IDC_LIST_SROOM_CHAT")
            .map(|placed| placed.rect)
            .expect("staged Room chat list")
    }

    /// Drive the real Room chat grammar: `/w Beta psst` then Enter.
    fn send_room_whisper(session: &mut crate::session::OnlineSession) {
        let mut keymap = Keymap::new();
        for (code, text) in [
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
        ] {
            let _ = route_raw_key(session, &mut keymap, code, Some(text));
        }
        let _ = route_raw_key(session, &mut keymap, KeyCode::Enter, None);
    }

    /// #967 Tier 3. Live caller chain:
    /// `main.rs::run_play` -> `connect::connect_and_login_with_transport` ->
    /// `assemble_online_session` -> pointer clicks into Room -> `OnlineSession::paint`
    /// -> `poll_lobby_chat` -> `apply_room_removed` -> `ScreenGame::remove_room`
    /// -> `paint_room_list`. Pin `0x55bc60` / `0x15bc60`, 2007,
    /// ev `f71b.c31f…fa06.3e82`. Archives absent -> skip cleanly.
    #[test]
    fn live_two_room_clients_drop_the_same_row_when_the_gateway_broadcasts_2007() {
        if !archives_present() {
            println!("SKIP live_two_room_clients_drop_the_same_row_when_the_gateway_broadcasts_2007: missing install-root interface archive");
            return;
        }
        let trace_id = "live-two-room-clients-2007";
        let gateway = DualGateway::spawn(joined_room(0, true));
        gateway.set_trace_id(trace_id);
        let (alpha_index, mut alpha) = boot_room_client(&gateway, "Alpha");
        let (beta_index, mut beta) = boot_room_client(&gateway, "Beta");

        let alpha_before = paint_frame(&mut alpha);
        let beta_before = paint_frame(&mut beta);
        assert!(alpha
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|room| room.number == 3));
        assert!(beta
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|room| room.number == 3));

        gateway.broadcast(&[RoomRemoved { number: 3 }.encode()]);
        let alpha_log = gateway.wire_log(alpha_index);
        let beta_log = gateway.wire_log(beta_index);
        assert_eq!(count(&alpha_log, Direction::ServerToClient, 2007), 1);
        assert_eq!(
            count(&beta_log, Direction::ServerToClient, 2007),
            1,
            "both observer clients must log the disband broadcast"
        );

        paint_until(&mut alpha, |session| {
            session
                .game_mut()
                .listed_rooms()
                .iter()
                .all(|room| room.number != 3)
        });
        paint_until(&mut beta, |session| {
            session
                .game_mut()
                .listed_rooms()
                .iter()
                .all(|room| room.number != 3)
        });
        assert!(changed_pixels(&alpha_before, &paint_frame(&mut alpha)) > 0);
        assert!(changed_pixels(&beta_before, &paint_frame(&mut beta)) > 0);
        assert_eq!(
            payload_of(&alpha_log, Direction::ServerToClient, 2007),
            3i32.to_le_bytes()
        );
        assert_eq!(
            payload_of(&beta_log, Direction::ServerToClient, 2007),
            3i32.to_le_bytes()
        );
        gateway.finish(vec![alpha, beta]);
    }

    /// #974 Tier 3. Live caller chain:
    /// `main.rs::run_play` -> Room chat keys -> `OnlineSession::flush_room_chat_action`
    /// -> C2S 2017 -> fake Memoryer -> S2C 2019 to both -> `OnlineSession::paint`
    /// -> `poll_lobby_chat` -> `apply_lobby_whisper` -> `paint_room_chat`.
    /// Pin `0x55c280` / `0x15c280`, 2019, ev `e462.0242…e455`.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_two_room_clients_paint_the_2019_whisper_from_one_send() {
        if !archives_present() {
            println!("SKIP live_two_room_clients_paint_the_2019_whisper_from_one_send: missing install-root interface archive");
            return;
        }
        let trace_id = "live-two-room-clients-2019";
        let gateway = DualGateway::spawn(joined_room(0, true));
        gateway.set_trace_id(trace_id);
        let (alpha_index, mut alpha) = boot_room_client(&gateway, "Alpha");
        let (beta_index, mut beta) = boot_room_client(&gateway, "Beta");

        let alpha_list = room_list_rect(&mut alpha);
        let beta_list = room_list_rect(&mut beta);
        let alpha_rest = staged_rect_pixels(&mut alpha, alpha_list);
        let beta_rest = staged_rect_pixels(&mut beta, beta_list);
        send_room_whisper(&mut alpha);
        gateway.wait_for_request(alpha_index, 2017);
        gateway.broadcast(&[ReceiveWhisper {
            sender: "Alpha".to_owned(),
            content: "psst".to_owned(),
        }
        .encode()]);
        let alpha_log = gateway.wire_log(alpha_index);
        let beta_log = gateway.wire_log(beta_index);
        assert_eq!(count(&alpha_log, Direction::ServerToClient, 2019), 1);
        assert_eq!(
            count(&beta_log, Direction::ServerToClient, 2019),
            1,
            "the whisper must reach both painted clients"
        );

        paint_until(&mut alpha, |session| {
            session.game_mut().room_chat().len() == 1
        });
        paint_until(&mut beta, |session| {
            session.game_mut().room_chat().len() == 1
        });
        assert_eq!(alpha.game_mut().room_chat()[0].sender, "Alpha");
        assert_eq!(
            alpha.game_mut().room_chat()[0].whisper_recipient.as_deref(),
            Some("Alpha")
        );
        assert_eq!(beta.game_mut().room_chat()[0].sender, "Alpha");
        assert_eq!(
            beta.game_mut().room_chat()[0].whisper_recipient.as_deref(),
            Some("Beta")
        );
        assert!(changed_pixels(&alpha_rest, &staged_rect_pixels(&mut alpha, alpha_list)) > 0);
        assert!(changed_pixels(&beta_rest, &staged_rect_pixels(&mut beta, beta_list)) > 0);
        gateway.finish(vec![alpha, beta]);
    }

    /// #559 Tier 3. Live caller chain:
    /// `main.rs::run_play` -> join by staged row click -> `OnlineSession::paint`
    /// -> `poll_waiting_chat` -> 4035 paint arm / 4026 / 4037 / 3009 frames
    /// -> `paint_waiting_seat_marks` -> `start_gate_verdict`.
    /// 4026/4037/3009 stay wire-only; 4035 is the pinned badge arm.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_two_waiting_clients_agree_on_the_badge_slice_and_start_gate() {
        if !archives_present() {
            println!("SKIP live_two_waiting_clients_agree_on_the_badge_slice_and_start_gate: missing install-root interface archive");
            return;
        }
        let gateway = DualGateway::spawn(joined_room(0, true));
        gateway.set_trace_id("live-two-waiting-clients-4026-4037-3009");
        let (alpha_index, mut alpha) = boot_waiting_client(&gateway, "Alpha");
        let (beta_index, mut beta) = boot_waiting_client(&gateway, "Beta");

        let badge_rect = waiting_badge_rect(&mut alpha, 0);
        let alpha_rest = staged_rect_pixels(&mut alpha, badge_rect);
        let beta_rest = staged_rect_pixels(&mut beta, badge_rect);
        let downloading = AcquireMusic {
            members: vec![MemberMusicState {
                member_id: 0,
                state: MusicState::Downloading,
            }],
        };
        let state = MemberStateChanged {
            member_id: 1,
            playing_state: 0,
            music_state: MusicState::Downloading.code(),
        };
        let music = SyncMemberMusicStateEvent {
            member_id: 1,
            state: MusicState::Downloading,
        };
        let slot = RoomSlotUpdate {
            index: 1,
            action_code: RoomSlotAction::SlotUnlocked as u8,
        };
        gateway.broadcast(&[
            downloading.encode(),
            state.encode(),
            music.encode(),
            slot.encode(),
        ]);
        let alpha_log = gateway.wire_log(alpha_index);
        let beta_log = gateway.wire_log(beta_index);
        for (index, log) in [&alpha_log, &beta_log].into_iter().enumerate() {
            let sequence: Vec<u16> = log
                .iter()
                .filter(|record| record.direction == Direction::ServerToClient)
                .filter(|record| [4026, 4037, 3009].contains(&record.id))
                .map(|record| record.id)
                .collect();
            assert_eq!(
                sequence,
                vec![4026, 4037, 3009],
                "client {index} missed the wire sequence"
            );
        }
        for session in [&mut alpha, &mut beta] {
            paint_until(session, |session| {
                session.slot_action(1) == Some(RoomSlotAction::SlotUnlocked)
                    && session.game_mut().local_download_badge_armed()
            });
        }
        assert_eq!(
            staged_rect_pixels(&mut alpha, badge_rect),
            staged_rect_pixels(&mut beta, badge_rect),
            "the 07E8 DOWNLOADING slice must agree pixel-for-pixel"
        );
        assert_ne!(staged_rect_pixels(&mut alpha, badge_rect), alpha_rest);
        assert_ne!(staged_rect_pixels(&mut beta, badge_rect), beta_rest);
        for session in [&mut alpha, &mut beta] {
            assert_eq!(
                session.game_mut().start_gate_verdict(),
                StartGateVerdict::MusicDownloading
            );
        }

        let ready = AcquireMusic {
            members: vec![
                MemberMusicState {
                    member_id: 0,
                    state: MusicState::Ready,
                },
                MemberMusicState {
                    member_id: 1,
                    state: MusicState::Ready,
                },
            ],
        };
        let ready_music = SyncMemberMusicStateEvent {
            member_id: 1,
            state: MusicState::Ready,
        };
        gateway.broadcast(&[ready.encode(), ready_music.encode()]);
        for session in [&mut alpha, &mut beta] {
            paint_until(session, |session| {
                !session.game_mut().local_download_badge_armed()
                    && session.game_mut().member_music_state(1) == Some(MusicState::Ready.code())
            });
        }
        assert_eq!(staged_rect_pixels(&mut alpha, badge_rect), alpha_rest);
        assert_eq!(staged_rect_pixels(&mut beta, badge_rect), beta_rest);
        for session in [&mut alpha, &mut beta] {
            assert_eq!(
                session.game_mut().start_gate_verdict(),
                StartGateVerdict::Startable
            );
        }
        gateway.finish(vec![alpha, beta]);
    }

    /// #592 Tier 3 wire leg is the session-tier guard: the clocked Loading
    /// gate needs the private `paint_at_instant` seam in `session.rs`.
    /// These live legs stay on the two painted scenes above.
    #[test]
    fn live_two_client_trace_id_is_shared_across_the_loading_fanout_guard() {
        const LOADING_TRACE_ID: &str = "dual-loading-4010-4011-4012";
        const LOADING_SCENARIO_EPOCH: u64 = 1;

        if !archives_present() {
            println!("SKIP live_two_client_trace_id_is_shared_across_the_loading_fanout_guard: missing install-root chart");
            return;
        }
        let gateway =
            DualGateway::spawn_with_rooms(vec![joined_room(0, true), joined_room(1, true)]);
        gateway.set_trace_id(LOADING_TRACE_ID);
        println!(
            "[{LOADING_TRACE_ID}] server_pid={} scenario_epoch={LOADING_SCENARIO_EPOCH} live leg",
            DualGateway::server_pid()
        );
        let (master_index, mut master) = boot_waiting_client(&gateway, "Alpha");
        let (guest_index, mut guest) = boot_waiting_client(&gateway, "Beta");
        use o2jam_shell::GameLoop;
        master.menu_key(o2jam_shell::MenuKey::Confirm, true);
        gateway.wait_for_request(master_index, 4010);
        gateway.broadcast(&[encode_frame(4011, &[])]);
        let master_log = gateway.wire_log(master_index);
        let guest_log = gateway.wire_log(guest_index);
        assert_eq!(count(&master_log, Direction::ServerToClient, 4011), 1);
        assert_eq!(count(&guest_log, Direction::ServerToClient, 4011), 1);
        paint_until(&mut master, |session| session.screen() == Screen::Loading);
        paint_until(&mut guest, |session| session.screen() == Screen::Loading);
        assert!(master_log
            .iter()
            .all(|record| record.trace_id == LOADING_TRACE_ID));
        assert!(guest_log
            .iter()
            .all(|record| record.trace_id == LOADING_TRACE_ID));
        gateway.finish(vec![master, guest]);
    }
}
