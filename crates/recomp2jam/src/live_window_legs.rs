//! Tier 3 live legs for the window-input family (#1122).
//! Both legs boot through the real connect entry, walk into Room, and
//! stage a real 3003 refusal over the gateway. Missing `install-root`
//! archives skip the legs cleanly (CI has no game data).

#[cfg(test)]
mod live_window_leg_tests {
    use std::io::Write;
    use std::sync::mpsc::{self, Receiver};
    use std::thread::JoinHandle;
    use std::time::Duration;

    use crate::live_leg_support::*;
    use crate::session::OnlineSession;
    use o2jam_net::testing::{fake_gateway, read_frame};
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
        WireMessage,
    };
    use o2jam_shell::screens::messenger::WindowHandler;
    use o2jam_shell::{GameLoop, InputSnapshot, Screen, Shell};

    /// Gateway that answers the first 3002 with the pinned room-full
    /// refusal (`-6`) and reports that the frame reached the socket.
    fn spawn_refusal_gateway() -> (u16, Receiver<()>, JoinHandle<()>) {
        let (answered, answer_seen) = mpsc::channel();
        let (port, server) = fake_gateway(move |mut stream: std::net::TcpStream| {
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            stream
                .write_all(
                    &AuthResponse {
                        result_code: 0,
                        tail: Vec::new(),
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

            while let Some(frame) = read_frame_until_eof(&mut stream) {
                if frame.id == Request::JoinWaiting.id() {
                    stream
                        .write_all(
                            &JoinRoomResponse {
                                result_code: u32::from_le_bytes((-6i32).to_le_bytes()),
                                room: None,
                            }
                            .encode(),
                        )
                        .unwrap();
                    answered.send(()).unwrap();
                }
            }
        });
        (port, answer_seen, server)
    }

    /// Boot into Room, press a row, then let the 3003 answer stage the
    /// one-OK refusal. The last paint drains the wire; no snapshot is
    /// frozen after `stage_join_answer_refusal` publishes the host.
    fn boot_with_refusal_sheet_up(port: u16, answer_seen: &Receiver<()>) -> OnlineSession {
        let mut session = boot_live_leg(port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        click_staged(&mut session, Screen::Room, "IDC_BUTTON_SROOM_EXBUTTON1");
        paint(&mut session);
        answer_seen
            .recv_timeout(Duration::from_secs(5))
            .expect("the refused row press was answered");
        paint(&mut session);
        assert!(
            session.game_mut().dialog_owns_input(),
            "the real 3003 sheet owns input before the double-click"
        );
        session
    }

    #[test]
    fn live_double_click_with_a_real_3003_sheet_queues_the_notice() {
        // Live chain: `main.rs::run_play` ->
        // `connect::connect_and_login_with_transport` -> `boot_live_leg` ->
        // staged row press -> `OnlineSession::drain_room_join` ->
        // 3003 result -6 -> `ScreenGame::stage_join_answer_refusal` ->
        // `publish_dialog_host_mode` -> `WindowHandler::route_window_left_press`
        // -> `route_mouse_input` -> `stage_double_click_at_native` ->
        // `is_double_click_pair` -> `double_click_notice` ->
        // `queued_ui_notices`. No snapshot is frozen between the wire
        // staging and the press pair.
        if !archives_present() {
            println!("SKIP live_double_click_with_a_real_3003_sheet_queues_the_notice: missing install-root interface archive");
            return;
        }
        let (port, answer_seen, server) = spawn_refusal_gateway();
        let session = boot_with_refusal_sheet_up(port, &answer_seen);
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);

        let notices = handler.queued_ui_notices();
        assert_eq!(notices.len(), 1, "one double-click queues one notice");
        assert_eq!(
            (notices[0].kind, notices[0].code, notices[0].param),
            (10, 3, 0)
        );
        assert_eq!((notices[0].x, notices[0].y), (21, 42));

        drop(handler);
        server.join().unwrap();
    }

    #[test]
    fn live_double_click_after_the_3003_ok_queues_nothing() {
        // Same boot and sheet, then the real OK snapshot path:
        // `ScreenGame::click_room` -> `clear_join_answer_refusal` ->
        // `publish_dialog_host_mode`. The shared cell must clear at
        // dismissal, not on a later frame, or the gate stays stale.
        if !archives_present() {
            println!("SKIP live_double_click_after_the_3003_ok_queues_nothing: missing install-root interface archive");
            return;
        }
        let (port, answer_seen, server) = spawn_refusal_gateway();
        let mut session = boot_with_refusal_sheet_up(port, &answer_seen);
        let (ok_x, ok_y) = information_ok_centre();
        session.ingest_snapshot(&InputSnapshot::click_at(ok_x, ok_y));
        assert!(
            !session.game_mut().dialog_owns_input(),
            "the OK snapshot dismissed the sheet"
        );
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);

        assert!(
            handler.queued_ui_notices().is_empty(),
            "no dialog must leave the input ring untouched"
        );

        drop(handler);
        server.join().unwrap();
    }
}
