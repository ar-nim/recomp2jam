//! Live-input legs for the Room chat/row family (#945-#948,
//! #971-#974). Boot/gateway/paint helpers come from
//! `crate::live_leg_support` (#926); every leg skips cleanly when the
//! installed archives are absent (CI has no game data, #0a3ba).

#[cfg(test)]
mod live_room_chat_leg_tests {
    use crate::live_leg_support::*;
    use o2jam_net::rooms::{MainRoomAdminMessage, MainRoomUserMessage, SendWhisperResponse};
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
        WireMessage,
    };
    use o2jam_render::{pack_xrgb, Rect};
    use o2jam_shell::screens::game::{JoinAnswerRefusal, LobbyChat, RoomJoinRefusal};
    use o2jam_shell::{GameLoop, InputSnapshot, Keymap, Screen};
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::mpsc::{self, Receiver};
    use std::thread::JoinHandle;
    use std::time::Duration;
    use winit::keyboard::KeyCode;

    /// Gateway handles for the started-row server refusal.
    struct StartedRoomGateway {
        port: u16,
        posted_opcodes: Receiver<u16>,
        answer_written: Receiver<()>,
        server: JoinHandle<()>,
    }

    fn expect_boot_request(stream: &mut TcpStream, want: u16) {
        let frame = o2jam_net::testing::read_frame(stream);
        assert_eq!(frame.id, want, "boot request {want}");
    }

    fn send_response(stream: &mut TcpStream, message: &impl WireMessage) {
        stream.write_all(&message.encode()).unwrap();
    }

    /// The five-frame login/channel/room-list exchange every live leg
    /// boots through.
    fn answer_boot_exchange(stream: &mut TcpStream) {
        expect_boot_request(stream, Request::Authorize.id());
        send_response(
            stream,
            &AuthResponse {
                result_code: 0,
                tail: vec![],
            },
        );
        expect_boot_request(stream, Request::GetChannelList.id());
        send_response(
            stream,
            &ChannelList {
                channels: vec![channel_row()],
            },
        );
        expect_boot_request(stream, Request::ChannelLogin.id());
        send_response(
            stream,
            &ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            },
        );
        expect_boot_request(stream, Request::GetChannelInfo.id());
        send_response(
            stream,
            &RoomList {
                rooms: vec![room_row()],
            },
        );
    }

    /// The shared boot gateway answers 3002 with success. This one
    /// answers the started row with the pinned 3003 result -5 instead.
    fn spawn_started_room_gateway() -> StartedRoomGateway {
        let (posted, posted_opcodes) = mpsc::channel();
        let (answer_written_tx, answer_written) = mpsc::channel();
        let (port, server) = o2jam_net::testing::fake_gateway(move |mut stream| {
            answer_boot_exchange(&mut stream);
            while let Some(frame) = read_frame_until_eof(&mut stream) {
                posted.send(frame.id).unwrap();
                if frame.id == Request::JoinWaiting.id() {
                    send_response(
                        &mut stream,
                        &JoinRoomResponse {
                            result_code: u32::from_le_bytes((-5i32).to_le_bytes()),
                            room: None,
                        },
                    );
                    answer_written_tx.send(()).unwrap();
                }
            }
        });
        StartedRoomGateway {
            port,
            posted_opcodes,
            answer_written,
            server,
        }
    }

    /// Paint until the 3003 verdict reaches the sheet painter.
    fn paint_until_in_progress_sheet(session: &mut crate::session::OnlineSession) {
        for _ in 0..MAX_SCREEN_PAINTS {
            if session.game_mut().join_answer_refusal() == Some(JoinAnswerRefusal::GameInProgress) {
                return;
            }
            paint(session);
        }
        panic!("the 3003 result -5 never posted the in-progress sheet");
    }

    /// Absolute native rect of one staged Room placement. Geometry is
    /// read from the same `.bnd` block the painter consumes.
    fn staged_room_rect(session: &mut crate::session::OnlineSession, id: &str) -> Rect {
        session
            .game_mut()
            .scene_placements(Screen::Room)
            .iter()
            .find(|placed| placed.id == id)
            .unwrap_or_else(|| panic!("no staged Room placement {id}"))
            .rect
    }

    fn changed_pixels(before: &[u32], after: &[u32]) -> usize {
        before
            .iter()
            .zip(after)
            .filter(|(was, now)| was != now)
            .count()
    }

    /// #945 Room ord 20. Live caller chain:
    ///   main.rs::run_play -> setup_online ->
    ///   connect::connect_and_login_with_transport -> assemble_online_session ->
    ///   GameLoop::ingest_snapshot (splash, emblem, resident row) ->
    ///   ScreenGame::text_input -> push_lobby_char -> ChatState::type_lobby_char ->
    ///   paint_room_chat -> InputSnapshot Enter ->
    ///   OnlineSession::room_chat_took_enter -> flush_room_chat_action ->
    ///   flush_lobby_chat -> C2S 2012.
    /// Pin `0x48ef00` / file `0x8ef00`, REA `ev_8d49ab59…`.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_room_typed_draft_inks_the_staged_edit_rect_and_sends_one_2012_from_boot() {
        if !archives_present() {
            println!("SKIP live_room_typed_draft_inks_the_staged_edit_rect_and_sends_one_2012_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let edit_rect = staged_room_rect(&mut session, "IDC_EDIT_SROOM_CHAT");
        let rest = staged_rect_pixels(&mut session, edit_rect);
        let mut keymap = Keymap::new();
        for (code, text) in [
            (KeyCode::KeyP, "p"),
            (KeyCode::KeyI, "i"),
            (KeyCode::KeyN, "n"),
            (KeyCode::KeyG, "g"),
        ] {
            let _ = route_raw_key(&mut session, &mut keymap, code, Some(text));
        }
        assert_eq!(
            session.game_mut().lobby_chat(LobbyChat::Room).draft(),
            "ping",
            "four live key edges reach the Room draft"
        );
        let typed = staged_rect_pixels(&mut session, edit_rect);
        assert!(
            changed_pixels(&rest, &typed) > 0,
            "the draft inks the staged IDC_EDIT_SROOM_CHAT rect"
        );

        let _ = route_raw_key(&mut session, &mut keymap, KeyCode::Enter, None);
        let body = wait_for_opcode(&gateway.reports, 2012);
        assert_eq!(body, b"ping".to_vec(), "one Enter sends the 2012 body");
        assert_eq!(
            session.game_mut().lobby_chat(LobbyChat::Room).draft(),
            "",
            "the sent draft clears"
        );
        finish(session, gateway.server);
    }

    /// #946/#971/#972/#974 Room ord 19 and 2013/2014/2019 shell half.
    /// Live chain:
    ///   main.rs::run_play -> connect -> assemble_online_session ->
    ///   GameLoop::ingest_snapshot -> Screen::Room -> ScreenGame::push_lobby_user_message
    ///   / push_lobby_admin_message -> ChatState::push_lobby_user / admin ->
    ///   paint_room_chat -> chat_row_ink.
    /// Wire codec/drain for these opcodes belongs to the room-wire lane;
    /// this leg stages decoded rows at the shell seam. Pin `0x48f360`,
    /// REA `ev_7ebf3473…`. Archives absent -> skip cleanly.
    #[test]
    fn live_room_chat_rows_ink_two_distinct_senders_in_the_staged_list_from_boot() {
        if !archives_present() {
            println!("SKIP live_room_chat_rows_ink_two_distinct_senders_in_the_staged_list_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let list_rect = staged_room_rect(&mut session, "IDC_LIST_SROOM_CHAT");
        let rest = staged_rect_pixels(&mut session, list_rect);
        session.game_mut().push_lobby_user_message(
            LobbyChat::Room,
            &MainRoomUserMessage {
                sender: "alice".to_owned(),
                content: b"hello".to_vec(),
            },
        );
        session.game_mut().push_lobby_admin_message(
            LobbyChat::Room,
            &MainRoomAdminMessage {
                sender: "op".to_owned(),
                content: b"quiet".to_vec(),
            },
        );
        let rows = session.game_mut().room_chat();
        assert_eq!(rows.len(), 2, "both decoded rows are staged");
        assert_eq!(rows[0].sender, "alice");
        assert!(rows[1].admin, "the second sender is an admin");

        let painted = staged_rect_pixels(&mut session, list_rect);
        assert!(
            changed_pixels(&rest, &painted) > 0,
            "the two rows change the staged list rect"
        );
        assert!(
            painted.contains(&pack_xrgb(0xDE, 0xDE, 0xDE)),
            "the 2013 row carries the pinned light-gray ink"
        );
        assert!(
            painted.contains(&pack_xrgb(0xE6, 0x71, 0x00)),
            "the 2014 row carries the pinned amber ink"
        );
        finish(session, gateway.server);
    }

    /// #947 Room ord 26-31. Live chain (eligible):
    ///   main.rs::run_play -> connect -> assemble_online_session ->
    ///   InputSnapshot staged row press -> OnlineSession::dispatch_room_press ->
    ///   C2S 3002 -> S2C 3003 -> ScreenGame::click_room_row -> Screen::Waiting.
    /// Refusal: the same staged press resolves `PressKind::Local`, reaches
    /// `ScreenGame::click_room_row`, stages the pinned one-OK sheet and
    /// queues no frame. Pin `0x493a50`, REA `ev_92af29…`.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_room_row_press_joins_when_eligible_and_refuses_without_a_frame_when_not() {
        if !archives_present() {
            println!("SKIP live_room_row_press_joins_when_eligible_and_refuses_without_a_frame_when_not: missing install-root interface archive");
            return;
        }
        let eligible_gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut eligible = boot_live_leg_with_art(eligible_gateway.port, false);
        dismiss_splash(&mut eligible);
        enter_room_by_clicks(&mut eligible);
        let mut eligible_room = room_row();
        // No song or key-mode gate: the eligible arm tests the join
        // wire, not the refusal table.
        eligible_room.music_id = 0;
        eligible_room.key_mode = 0;
        eligible_room.min_level = 0;
        eligible_room.max_level = 0;
        eligible.game_mut().set_rooms(vec![eligible_room]);
        join_waiting_by_click(&mut eligible, &eligible_gateway.join_answer_written);
        assert_eq!(eligible.screen(), Screen::Waiting, "eligible row joins");
        finish(eligible, eligible_gateway.server);

        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let mut refused_room = room_row();
        refused_room.music_id = 9999;
        refused_room.min_level = 0;
        refused_room.max_level = 0;
        let rest = paint_frame(&mut session);
        session.game_mut().set_rooms(vec![refused_room]);
        click_staged(&mut session, Screen::Room, "IDC_BUTTON_SROOM_EXBUTTON1");
        paint(&mut session);
        assert_eq!(session.screen(), Screen::Room, "refused row stays in Room");
        assert_eq!(
            session.game_mut().room_join_refusal(),
            Some(RoomJoinRefusal::UnregisteredSong),
            "the unregistered-song caption arm is staged"
        );
        assert!(
            changed_pixels(&rest, &paint_frame(&mut session)) > 0,
            "the refusal sheet changes the Room frame"
        );
        assert!(
            gateway.reports.try_iter().all(|frame| frame.id != 3002),
            "a refused row posts no join"
        );
        let (ok_x, ok_y) = information_ok_centre();
        session.ingest_snapshot(&InputSnapshot::click_at(ok_x, ok_y));
        assert_eq!(
            session.game_mut().room_join_refusal(),
            None,
            "the one-OK sheet dismisses"
        );
        finish(session, gateway.server);
    }

    /// #947 started-room refusal, Room ord 26-31. Live chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport ->
    ///   assemble_online_session -> InputSnapshot staged row press ->
    ///   ScreenGame::press_kind_at -> OnlineSession::dispatch_room_press ->
    ///   C2S 3002 -> S2C 3003 (-5) -> OnlineSession::stage_3003_refusal ->
    ///   ScreenGame::stage_join_answer_refusal -> paint_information_message.
    /// The pinned in-progress caption comes from the server answer; the
    /// exe has no started-room local refusal in `0x493a50`.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_started_room_row_press_posts_3002_and_the_minus_five_sheet_stays_in_room() {
        if !archives_present() {
            println!("SKIP live_started_room_row_press_posts_3002_and_the_minus_five_sheet_stays_in_room: missing install-root interface archive");
            return;
        }
        let gateway = spawn_started_room_gateway();
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let mut started = room_row();
        started.state_code = o2jam_net::RoomState::Playing as u8;
        session.game_mut().set_rooms(vec![started]);
        let rest = paint_frame(&mut session);
        click_staged(&mut session, Screen::Room, "IDC_BUTTON_SROOM_EXBUTTON1");
        paint(&mut session);
        gateway
            .answer_written
            .recv_timeout(Duration::from_secs(5))
            .expect("the started row posted 3002 and got the -5 answer");
        assert!(
            gateway
                .posted_opcodes
                .try_iter()
                .any(|id| id == Request::JoinWaiting.id()),
            "the started-room press posts 3002"
        );
        paint_until_in_progress_sheet(&mut session);

        let sheet_frame = paint_frame(&mut session);
        assert_eq!(session.screen(), Screen::Room, "the refusal stays in Room");
        assert_eq!(
            session.game_mut().room_join_refusal(),
            None,
            "no invented local started-room caption"
        );
        assert_eq!(
            session.game_mut().join_answer_refusal(),
            Some(JoinAnswerRefusal::GameInProgress),
            "the server owns the in-progress caption"
        );
        assert!(
            changed_pixels(&rest, &sheet_frame) > 0,
            "the in-progress sheet changes the Room frame"
        );

        let (ok_x, ok_y) = information_ok_centre();
        session.ingest_snapshot(&InputSnapshot::click_at(ok_x, ok_y));
        assert_eq!(
            session.game_mut().join_answer_refusal(),
            None,
            "OK dismisses the server sheet"
        );
        assert_eq!(session.screen(), Screen::Room, "still Room after OK");
        finish(session, gateway.server);
    }

    /// #948 Room ord 69/70. Live chain:
    ///   main.rs::run_play -> connect -> assemble_online_session ->
    ///   ScreenGame::open_room_user_sheet -> paint_room -> ScreenGame::exit_room_to_channel
    ///   -> ChatState::close_room_user_sheet.
    /// The exe closes a browser HWND (`0x495a30`, REA `ev_96389df4…`);
    /// this client has no web bridge, so the staged DESC well stands in
    /// for the sheet and only the close/refresh behavior is pinned.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_room_exit_closes_the_user_sheet_and_refreshes_gift_on_from_boot() {
        if !archives_present() {
            println!("SKIP live_room_exit_closes_the_user_sheet_and_refreshes_gift_on_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let desc_rect = staged_room_rect(&mut session, "IDC_TEXT_SROOM_DESC");
        let rest = staged_rect_pixels(&mut session, desc_rect);
        session.game_mut().open_room_user_sheet(true);
        let mounted = staged_rect_pixels(&mut session, desc_rect);
        assert!(
            changed_pixels(&rest, &mounted) > 0,
            "the mounted user sheet changes the staged DESC well"
        );
        let mut keymap = Keymap::new();
        let _ = route_raw_key(&mut session, &mut keymap, KeyCode::Escape, None);
        assert_eq!(session.screen(), Screen::Channel, "Room exit returns");
        assert!(
            !session.game_mut().room_user_sheet_open(),
            "the sheet is closed on teardown"
        );
        assert!(
            session.game_mut().room_user_sheet_url_gift_on(),
            "the URL state refreshes from the gift-on word"
        );
        // Re-enter the same resident row: the sheet must not come
        // back with the next room's panel (#948).
        session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        paint_until_screen(&mut session, Screen::Room);
        assert_eq!(
            staged_rect_pixels(&mut session, desc_rect),
            rest,
            "the DESC well is bare again after re-entry"
        );
        finish(session, gateway.server);
    }

    /// #973 shell half of opcode 2018. Live chain:
    ///   main.rs::run_play -> connect -> assemble_online_session ->
    ///   ScreenGame::push_send_whisper_response -> ChatState::push_whisper_response
    ///   -> paint_channel_chat.
    /// The 2018 drain belongs to the room-wire lane; this leg drives the
    /// decoded answer at the shell seam. Archives absent -> skip cleanly.
    #[test]
    fn live_room_2018_answer_keeps_the_delivery_verdict_from_boot() {
        if !archives_present() {
            println!("SKIP live_room_2018_answer_keeps_the_delivery_verdict_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        session
            .game_mut()
            .push_send_whisper_response(&SendWhisperResponse {
                invalid: false,
                recipient: "peer".to_owned(),
                content: Some("hi".to_owned()),
            });
        assert_eq!(
            session
                .game_mut()
                .whisper_delivery()
                .map(|delivery| delivery.delivered && delivery.recipient == "peer"),
            Some(true),
            "a delivered answer keeps the recipient"
        );
        assert_eq!(
            session.game_mut().room_chat().len(),
            1,
            "echo paints one row"
        );
        session
            .game_mut()
            .push_send_whisper_response(&SendWhisperResponse {
                invalid: true,
                recipient: "peer".to_owned(),
                content: None,
            });
        assert_eq!(
            session.game_mut().whisper_delivery().map(|d| d.delivered),
            Some(false),
            "a refused answer replaces the verdict"
        );
        assert_eq!(
            session.game_mut().room_chat().len(),
            1,
            "a refusal carries no echoed row"
        );
        finish(session, gateway.server);
    }
}
