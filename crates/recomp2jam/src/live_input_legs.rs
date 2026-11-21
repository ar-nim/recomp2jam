//! One live-input leg per player-visible behavior family (#739).
//!
//! The scripted gateway guards in `session.rs` call `menu_key` and the
//! loading gate directly: fast and broad, but blind to the input
//! mapping that broke live BACK (#734). These legs start at the real
//! `connect` boot entry and drive the shell through pointer snapshots
//! only. They skip cleanly when the installed archives are absent —
//! CI has no game data (#0a3ba).

#[cfg(test)]
mod live_input_leg_tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::mpsc::{self, Receiver};
    use std::thread::JoinHandle;

    use o2jam_net::testing::{fake_gateway, read_frame};
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, ChannelState, Frame, Gender, JoinRoom,
        JoinRoomResponse, Request, RoomList, RoomListEntry, RoomSlotState, WireMessage,
    };
    use o2jam_render::{Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::screens::messenger::MessengerHost;
    use o2jam_shell::screens::paint::dialog_origin;
    use o2jam_shell::screens::side::INFORMATION_OK_CODE;
    use o2jam_shell::staging::{read_install_archives, stage_dialog, Dialog, StageContext};
    use o2jam_shell::{GameLoop, InputEdge, InputSnapshot, Screen};

    use crate::{
        assemble_online_session, connect, install_root, session::OnlineSession,
        stage_channel_list_after_relay_gate, stage_online_identity, stage_online_rooms,
    };

    /// Live boot rows: one channel (the resident one) and one room.
    fn channel_row() -> ChannelState {
        ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 100,
            population: 1,
            active: true,
        }
    }

    fn room_row() -> RoomListEntry {
        RoomListEntry {
            number: 3,
            state_code: 1,
            title: "Lobby".to_owned(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 1,
            min_level: 0,
            max_level: 99,
            skills: Vec::new(),
            premium: false,
            room_type: 0,
        }
    }

    fn boot_options(port: u16) -> connect::ConnectOptions {
        connect::ConnectOptions {
            host: "127.0.0.1".to_owned(),
            port,
            user_id: 7,
            token: "gate".to_owned(),
            gender: Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel: Some((1, 2)),
            character: false,
            wallet: false,
            avatar_opa: None,
            join_room: None,
            room_password: String::new(),
            song: None,
        }
    }

    /// Handles of a live-leg gateway: its port, recorded client frames,
    /// 3003 write signal, and server thread (#754).
    struct LiveLegGateway {
        port: u16,
        reports: Receiver<Frame>,
        join_answer_written: Receiver<()>,
        server: JoinHandle<()>,
    }

    /// Drive the 1000/1001/1002/1003 boot exchange, then record every
    /// later client frame until the socket goes quiet. The channel and
    /// room rows come back on the wire, not from a test-side list. The
    /// 3003 write is signalled so a Waiting click never races the
    /// gateway thread's scheduling under fleet load.
    fn spawn_live_leg_gateway(joined: o2jam_net::JoinedRoom) -> LiveLegGateway {
        let (reported, reports) = mpsc::channel();
        let (join_written_tx, join_answer_written) = mpsc::channel();
        let (port, server) = fake_gateway(move |mut stream| {
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            stream
                .write_all(
                    &AuthResponse {
                        result_code: 0,
                        tail: vec![],
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            stream
                .write_all(
                    &ChannelList {
                        channels: vec![channel_row()],
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            stream
                .write_all(
                    &RoomList {
                        rooms: vec![room_row()],
                    }
                    .encode(),
                )
                .unwrap();

            // The socket stays open until the test drops the session:
            // boot staging reads large archives, so a short quiet
            // timeout here would close under the test's feet (#52).
            while let Some(frame) = read_frame_until_eof(&mut stream) {
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
                    join_written_tx.send(()).unwrap();
                }
                reported.send(frame).unwrap();
            }
        });
        LiveLegGateway {
            port,
            reports,
            join_answer_written,
            server,
        }
    }

    /// One whole frame, or `None` at client close.
    fn read_frame_until_eof(stream: &mut TcpStream) -> Option<Frame> {
        let mut header = [0u8; o2jam_net::frame::HEADER_LEN];
        stream.read_exact(&mut header).ok()?;
        let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
        let mut bytes = header.to_vec();
        let mut body = vec![0u8; body_len.saturating_sub(o2jam_net::frame::HEADER_LEN)];
        stream.read_exact(&mut body).ok()?;
        bytes.extend_from_slice(&body);
        Some(o2jam_net::frame::decode_frame(&bytes).unwrap())
    }

    /// The 3003 snapshot shape: a seven-seat room with one peer. The
    /// host/guest seat and the peer's readiness decide the Waiting
    /// confirm arm the live click must reach.
    fn joined_room(member_id: u8, peer_ready: bool) -> o2jam_net::JoinedRoom {
        let slots = (0..7u8)
            .map(|index| o2jam_net::RoomSlot {
                index,
                state_code: 0,
                member: None,
            })
            .collect();
        let mut room = o2jam_net::JoinedRoom {
            member_id,
            team: if member_id == 0 { 0 } else { 1 },
            title: "Lobby".to_owned(),
            music_id: 757,
            arena: 1,
            key_mode: 2,
            game_mode: 0,
            difficulty: 0,
            speed: 1,
            has_super_manager: false,
            user_count: 2,
            slots,
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        };
        let peer_seat: u8 = if member_id == 0 { 1 } else { 0 };
        // The wire decoder reads a member only for an occupied slot.
        room.slots[usize::from(peer_seat)].state_code = RoomSlotState::Occupied as i32;
        room.slots[usize::from(peer_seat)].member = Some(o2jam_net::RoomMember {
            nickname: "peer".to_owned(),
            level: 1,
            gender: 0,
            gem: 0,
            master: false,
            team: if member_id == 0 { 1 } else { 0 },
            ready: peer_ready,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        });
        room
    }

    /// The real boot entry plus the same resident row the windowed
    /// `run_online` stages after login (#348). `relay_down` arms the
    /// one-OK notice that holds the channel rows.
    fn boot_live_leg(port: u16, relay_down: bool) -> OnlineSession {
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        stage_channel_list_after_relay_gate(&mut game, report.channels.clone(), relay_down);
        stage_online_rooms(&mut game, report.rooms.clone());
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        session.set_resident_channel(channel_row());
        session
    }

    fn archives_present() -> bool {
        read_install_archives(&install_root()).interface.is_some()
    }

    /// Splash dismissal is a Menu edge in a snapshot — the same edge
    /// the window queues for Escape, never a direct `menu_key` call.
    fn dismiss_splash(session: &mut OnlineSession) {
        assert_eq!(session.screen(), Screen::Avi);
        let mut snapshot = InputSnapshot::new();
        snapshot.push_edge(InputEdge::Menu {
            key: o2jam_shell::MenuKey::Back,
            down: true,
        });
        session.ingest_snapshot(&snapshot);
        assert_eq!(session.screen(), Screen::Planet);
    }

    /// Click the centre of a staged `.bnd` placement. No coordinates
    /// are invented here: the painter's own geometry owns the press.
    fn click_staged(session: &mut OnlineSession, screen: Screen, id: &str) {
        let placement = session
            .game_mut()
            .scene_placements(screen)
            .iter()
            .find(|placed| placed.id == id)
            .unwrap_or_else(|| panic!("no staged placement {id} on {screen:?}"));
        let centre = rect_centre(placement.rect);
        session.ingest_snapshot(&InputSnapshot::click_at(centre.0, centre.1));
    }

    /// The OK plate comes from the installed `Dialog_Information.bnd`
    /// sheet, staged by the same `stage_dialog` call as live boot.
    fn information_ok_centre() -> (i32, i32) {
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive present");
        let art = stage_dialog(&archive, Dialog::Information, StageContext::default())
            .art
            .expect("Dialog_Information staged");
        let (origin_x, origin_y) = dialog_origin(&art.placements);
        let ok = art
            .placements
            .iter()
            .find(|placed| placed.code == INFORMATION_OK_CODE)
            .expect("information OK staged");
        let centre = rect_centre(ok.rect);
        (centre.0 + origin_x, centre.1 + origin_y)
    }

    fn rect_centre(rect: Rect) -> (i32, i32) {
        let half_width = i32::try_from(rect.w / 2).expect("rect width fits a frame coordinate");
        let half_height = i32::try_from(rect.h / 2).expect("rect height fits a frame coordinate");
        (rect.x + half_width, rect.y + half_height)
    }

    /// Paint budget for a screen change already driven by input or a
    /// gateway signal. Counting paints, not wall time, keeps a loaded
    /// scheduler out of the assertion (#754).
    const MAX_SCREEN_PAINTS: usize = 64;

    /// Paint until the boot-driven action reaches its next screen.
    fn paint_until_screen(session: &mut OnlineSession, want: Screen) {
        for _ in 0..MAX_SCREEN_PAINTS {
            if session.screen() == want {
                return;
            }
            paint(session);
        }
        assert_eq!(session.screen(), want, "live click never reached {want:?}");
    }

    fn paint(session: &mut OnlineSession) {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits a buffer index");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits a buffer index");
        let mut buf = vec![0u32; width * height];
        session.paint(&mut o2jam_render::Frame {
            buf: &mut buf,
            w: width,
            h: height,
        });
    }

    /// Click path to the Room lobby: the pinned emblem face opens the
    /// channel list, the resident row replays locally on its press.
    fn enter_room_by_clicks(session: &mut OnlineSession) {
        session.ingest_snapshot(&InputSnapshot::click_at(136, 88));
        assert_eq!(session.screen(), Screen::Channel);
        session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        paint_until_screen(session, Screen::Room);
    }

    /// Click path to Waiting: the room row press queues 3002 and the
    /// 3003 answer replays the same press into the shell. Waiting on
    /// the gateway's write signal removes the wall-clock race (#754).
    fn join_waiting_by_click(session: &mut OnlineSession, join_answer_written: &Receiver<()>) {
        click_staged(session, Screen::Room, "IDC_BUTTON_SROOM_EXBUTTON1");
        // Paint first: this frame posts 3002. The gateway writes 3003
        // only after it reads that frame, then signals the write.
        paint(session);
        join_answer_written
            .recv()
            .expect("the gateway wrote the 3003 join answer");
        paint_until_screen(session, Screen::Waiting);
    }

    fn observed(reports: &Receiver<Frame>) -> Vec<(u16, Vec<u8>)> {
        reports
            .try_iter()
            .map(|frame| (frame.id, frame.payload))
            .collect()
    }

    /// Wait for one opcode after a paint: the gateway thread records
    /// frames as it reads them, so an immediate `try_iter` can outrun
    /// the socket (#52). The channel is the signal; a closed channel
    /// means the gateway died before the frame reached it (#754).
    fn wait_for_opcode(reports: &Receiver<Frame>, want: u16) -> Vec<u8> {
        loop {
            match reports.recv() {
                Ok(frame) if frame.id == want => return frame.payload,
                Ok(_) => {}
                Err(_) => panic!("gateway closed before opcode {want}"),
            }
        }
    }

    /// Click a staged placement by code, not by a guessed coordinate.
    fn staged_code_centre(session: &mut OnlineSession, screen: Screen, code: u32) -> (i32, i32) {
        let placement = session
            .game_mut()
            .scene_placements(screen)
            .iter()
            .find(|placed| placed.code == code)
            .unwrap_or_else(|| panic!("no staged placement for {code:#x} on {screen:?}"));
        rect_centre(placement.rect)
    }

    /// Closing the client ends the recorder thread's read loop.
    fn finish(session: OnlineSession, server: JoinHandle<()>) {
        drop(session);
        server.join().unwrap();
    }

    /// Messenger tier-1 leg (spec #315 story 17): one live caller chain
    /// from boot into a Messenger-family P0. The Room board's MESSENGER
    /// control stages from the installed `.bnd`, and the click goes
    /// through the same pointer snapshot a window queues. It ends at
    /// the labeled `unwired` arm: no side screen, no sheet, no wire.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (splash Menu edge)
    ///   -> Screen::Planet -> staged emblem click -> Screen::Channel
    ///   -> staged room-row click -> Screen::Room
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_room_row
    ///   -> ScreenGame::click_room_control -> ScreenGame::room_control_at
    ///   -> RoomControlAction::Unwired (IDC_BUTTON_SROOM_MESSENGER)
    ///   -> ScreenGame::last_room_control_noop.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_room_messenger_press_reaches_its_labeled_p0_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let code = MessengerHost::Room.control_code();
        let (x, y) = staged_code_centre(&mut session, Screen::Room, code);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);

        let noop = session
            .game_mut()
            .room_control_noop()
            .expect("the MESSENGER press reached the labeled P0");
        assert_eq!(noop.id, "IDC_BUTTON_SROOM_MESSENGER");
        assert!(
            noop.next_step.contains("CStateRoom click dispatch"),
            "next step names the host capture: {}",
            noop.next_step
        );
        assert_eq!(session.screen(), Screen::Room, "no sheet replaces the host");
        assert_eq!(session.game_mut().side(), None, "no side screen opens");
        assert!(
            !session.game_mut().messenger_liveness_open(),
            "the liveness sheet is a different pinned arm"
        );
        assert!(
            observed(&gateway.reports).is_empty(),
            "a P0 press invents no Messenger wire: {:?}",
            observed(&gateway.reports)
        );
        finish(session, gateway.server);
    }

    /// Family 1 — scene BACK/exits (#734 class). The complete leg:
    /// boot -> splash -> emblem -> channel row -> room row -> Waiting,
    /// then the staged BACK click posts the pinned leave.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> stage_channel_list_after_relay_gate -> assemble_online_session
    ///   -> GameLoop::ingest_snapshot -> OnlineSession::dispatch_pointer_edge(RoomRow)
    ///   -> poll_room_join (3002/3003) -> echo_entry(Press) -> Waiting
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_waiting_button
    ///   -> ScreenGame::queue_waiting_exit -> OnlineSession::poll_wire
    ///   -> flush_waiting_exit -> send_waiting_leave -> ExitWaiting 3005.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_waiting_back_sends_the_room_leave_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let before = observed(&gateway.reports);
        assert_eq!(
            before.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![Request::JoinWaiting.id()],
            "the join click posted 3002 and nothing else"
        );

        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
        assert_eq!(
            session.screen(),
            Screen::Room,
            "BACK left Waiting on the click"
        );
        paint(&mut session);

        let leave_body = wait_for_opcode(&gateway.reports, Request::ExitWaiting.id());
        assert_eq!(
            leave_body,
            vec![0, 0],
            "the clicked BACK sent the pinned 3005 Normal-exit body"
        );
        let extra_leaves = observed(&gateway.reports)
            .iter()
            .filter(|(id, _)| *id == Request::ExitWaiting.id())
            .count();
        assert_eq!(extra_leaves, 0, "one BACK click posts one leave");
        finish(session, gateway.server);
    }

    /// Rejoin after an exit. A resident-room heuristic must not treat
    /// the row as still joined once 3005 has cleared the seat: the
    /// second identical click is a new join and must post 3002 again.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (first room-row click)
    ///   -> OnlineSession::dispatch_pointer_edge(RoomRow) -> begin_room_join
    ///   -> poll_room_join (3002/3003) -> echo_entry(Press) -> Waiting
    ///   -> GameLoop::ingest_snapshot (BACK click) -> ScreenGame::queue_waiting_exit
    ///   -> OnlineSession::flush_waiting_exit -> send_waiting_leave (3005)
    ///   -> GameLoop::ingest_snapshot (second room-row click)
    ///   -> dispatch_pointer_edge(RoomRow) -> begin_room_join -> 3002.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_rejoin_after_back_sends_a_second_join_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let first = JoinRoom::decode_body(&wait_for_opcode(
            &gateway.reports,
            Request::JoinWaiting.id(),
        ))
        .expect("first join body decodes");
        assert_eq!(first.number, 3, "the first click joined the listed row");

        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
        assert_eq!(session.screen(), Screen::Room, "BACK left Waiting");
        paint(&mut session);
        wait_for_opcode(&gateway.reports, Request::ExitWaiting.id());

        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let second = JoinRoom::decode_body(&wait_for_opcode(
            &gateway.reports,
            Request::JoinWaiting.id(),
        ))
        .expect("rejoin body decodes");
        assert_eq!(
            second.number, first.number,
            "the rejoin joined the same row"
        );
        assert_eq!(
            second.password, first.password,
            "the rejoin kept the row password"
        );
        assert_eq!(
            session.screen(),
            Screen::Waiting,
            "the rejoin entered Waiting"
        );
        finish(session, gateway.server);
    }

    /// Family 2 — dialog OK/dismiss. The relay-down sheet owns the
    /// board until its staged OK plate is clicked; the same pointer
    /// path then reaches the channel row it was holding.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> stage_channel_list_after_relay_gate (holds the 1003 rows)
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (emblem click)
    ///   -> ScreenGame::click_planet -> enter_screen(Channel)
    ///   -> ScreenGame::open_relay_down_dialog -> GameLoop::ingest_snapshot (OK click)
    ///   -> ScreenGame::click_dispatch -> click_planet_modal
    ///   -> ScreenGame::dismiss_relay_down_dialog -> set_channels (rows released).
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_dialog_ok_releases_the_held_channel_rows() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, true);
        dismiss_splash(&mut session);

        session.ingest_snapshot(&InputSnapshot::click_at(136, 88));
        assert_eq!(session.screen(), Screen::Channel);
        assert_eq!(
            session.game_mut().planet_modal(),
            Some(o2jam_shell::screens::game::PlanetModal::RelayDown)
        );
        assert!(
            session.game_mut().channel_rows().is_empty(),
            "the sheet holds the rows"
        );

        let ok = information_ok_centre();
        session.ingest_snapshot(&InputSnapshot::click_at(ok.0, ok.1));
        assert_eq!(
            session.game_mut().planet_modal(),
            None,
            "the clicked OK dismissed the sheet"
        );
        assert_eq!(
            session.game_mut().channel_rows(),
            [channel_row()],
            "OK released the wire rows"
        );

        session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        paint_until_screen(&mut session, Screen::Room);
        finish(session, gateway.server);
    }

    /// Family 3 — Waiting lifecycle. The join is itself a room-row
    /// click; READY/START and BACK then ride the staged button rects.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (room-row click)
    ///   -> OnlineSession::dispatch_pointer_edge(RoomRow) -> set_pending_room_join
    ///   -> begin_room_join -> poll_room_join (3002/3003)
    ///   -> echo_entry(Press) -> Waiting -> GameLoop::ingest_snapshot (READY/START click)
    ///   -> OnlineSession::dispatch_pointer_edge(WaitingConfirm) -> confirm_waiting
    ///   -> Ready 4008 / StartGame 4010 -> GameLoop::ingest_snapshot (BACK click)
    ///   -> ScreenGame::click_waiting_button -> queue_waiting_exit -> poll_wire
    ///   -> flush_waiting_exit -> ExitWaiting 3005.
    /// Archives absent -> skip cleanly (CI has no game data).
    fn waiting_lifecycle_by_clicks(
        member_id: u8,
        peer_ready: bool,
        confirm_id: &str,
        expect_id: u16,
    ) {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let joined = joined_room(member_id, peer_ready);
        let gateway = spawn_live_leg_gateway(joined);
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let before = observed(&gateway.reports);
        assert_eq!(
            before.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![Request::JoinWaiting.id()],
            "the lifecycle leg joined through the room-row click"
        );

        click_staged(&mut session, Screen::Waiting, confirm_id);
        paint(&mut session);
        wait_for_opcode(&gateway.reports, expect_id);
        assert_eq!(
            session.screen(),
            Screen::Waiting,
            "confirm stays in Waiting"
        );

        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
        paint(&mut session);
        wait_for_opcode(&gateway.reports, Request::ExitWaiting.id());
        let extra_leaves = observed(&gateway.reports)
            .iter()
            .filter(|(id, _)| *id == Request::ExitWaiting.id())
            .count();
        assert_eq!(extra_leaves, 0, "one BACK click posts one leave");
        assert_eq!(session.screen(), Screen::Room, "BACK returns to Room");
        finish(session, gateway.server);
    }

    #[test]
    fn live_click_guest_ready_then_back_from_boot() {
        waiting_lifecycle_by_clicks(1, true, "IDC_BUTTON_SWAIT_READY", Request::Ready.id());
    }

    #[test]
    fn live_click_master_start_then_back_from_boot() {
        waiting_lifecycle_by_clicks(0, true, "IDC_BUTTON_SWAIT_START", Request::StartGame.id());
    }
}
