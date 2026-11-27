//! Tier 3 live legs for the window-input family (#1122).
//! Both legs boot through the real connect entry, walk into Room, and
//! stage a real 3003 refusal over the gateway. Missing `install-root`
//! archives skip the legs cleanly (CI has no game data).

#[cfg(test)]
mod live_window_leg_tests {
    use std::io::Write;
    use std::sync::mpsc::{self, Receiver};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use crate::live_leg_support::*;
    use crate::session::OnlineSession;
    use o2jam_net::testing::{fake_gateway, read_frame};
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
        WireMessage,
    };
    use o2jam_shell::screens::messenger::WindowHandler;
    use o2jam_shell::screens::side::PLANET_OPTION_CODE;
    use o2jam_shell::{GameLoop, InputSnapshot, Screen, Shell};

    /// Drive one press pair through the window arm and return the
    /// queued notice, if the mode word opened the gate.
    fn double_click(handler: &mut WindowHandler) {
        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);
    }

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
    /// frozen after `stage_join_answer_refusal` stages the sheet.
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
        // `ScreenGame::dialog_owns_input` ->
        // `WindowHandler::route_window_left_press` -> `route_mouse_input`
        // -> `stage_double_click_at_native` -> `is_double_click_pair` ->
        // `double_click_notice` -> `queued_ui_notices`. No snapshot is
        // frozen between the wire staging and the press pair.
        if !archives_present() {
            println!("SKIP live_double_click_with_a_real_3003_sheet_queues_the_notice: missing install-root interface archive");
            return;
        }
        let (port, answer_seen, server) = spawn_refusal_gateway();
        let session = boot_with_refusal_sheet_up(port, &answer_seen);
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);

        double_click(&mut handler);

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
        // `ScreenGame::dialog_owns_input`. The live query clears with
        // the sheet, so a later frame cannot leave a stale gate.
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

        double_click(&mut handler);

        assert!(
            handler.queued_ui_notices().is_empty(),
            "no dialog must leave the input ring untouched"
        );

        drop(handler);
        server.join().unwrap();
    }

    #[test]
    fn live_window_queued_planet_options_release_then_double_click_queues_the_notice() {
        // Live chain: `main.rs::run_online` ->
        // `connect::connect_and_login_with_transport` -> `boot_live_leg_with_art`
        // -> `WindowHandler::queue_window_left_press` (gear press/release)
        // -> `WindowHandler::deliver_snapshot` -> `OnlineSession::ingest_snapshot`
        // -> `apply_pointer_up` -> `planet_pointer_up` ->
        // `dispatch_planet_action` -> `planet_board.open_modal(Options)` ->
        // `WindowHandler::route_window_left_press` -> `route_mouse_input` ->
        // `stage_double_click_at_native` -> `control_mode_word` ->
        // `double_click_notice` -> `queued_ui_notices`. The release reaches
        // the game only at the frame boundary, before the event-time
        // press pair, so the live query sees the sheet from this frame.
        if !archives_present() {
            println!("SKIP live_window_queued_planet_options_release_then_double_click_queues_the_notice: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        let (x, y) = staged_code_centre(&mut session, Screen::Planet, PLANET_OPTION_CODE);
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);

        handler.queue_window_left_press(f64::from(x), f64::from(y));
        let release_frame = handler.deliver_snapshot();
        assert_eq!(
            release_frame.edges().len(),
            2,
            "the queued gear press and release reach one frame"
        );

        // ADR-0013 freezes input once per frame. A press pair that is
        // only queued reaches the game through `ingest_snapshot`, not
        // the event-time mouse arm, so it cannot form the exe's
        // double-click edge. The event-time pair below is the arm.
        handler.queue_window_left_press(21.0, 42.0);
        handler.queue_window_left_press(21.0, 42.0);
        handler.deliver_snapshot();
        assert!(
            handler.queued_ui_notices().is_empty(),
            "queued edges alone cannot stage the double-click (ADR-0013)"
        );

        double_click(&mut handler);

        let notices = handler.queued_ui_notices();
        assert_eq!(notices.len(), 1, "the event-time pair queues one notice");
        assert_eq!((notices[0].kind, notices[0].code), (10, 3));

        drop(handler);
        gateway.server.join().unwrap();
    }

    #[test]
    fn live_double_click_after_the_paint_pump_closes_the_last_dialog_queues_nothing() {
        // Live chain: `main.rs::run_online` -> `boot_live_leg` ->
        // `enter_room_by_clicks` -> `pump_room_user_list_fuse_at` (virtual
        // clock for the 20 s gate) -> Information OK ->
        // `post_room_no_response_dismissal` -> `OnlineSession::paint` ->
        // `ScreenGame::paint` -> `pump_dialog_queue` ->
        // `close_dismissed_dialog_layer` -> `WindowHandler::route_window_left_press`
        // -> `stage_double_click_at_native` -> `control_mode_word` ->
        // `double_click_notice`. The pump closes the sheet during paint;
        // no snapshot follows it before the press pair.
        if !archives_present() {
            println!("SKIP live_double_click_after_the_paint_pump_closes_the_last_dialog_queues_nothing: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        session.game_mut().set_room_no_response_relay_down(true);
        let now = Instant::now();
        assert!(session.game_mut().track_room_user_list_send_at(
            now.checked_sub(Duration::from_secs(21))
                .expect("clock margin")
        ));
        assert!(session.game_mut().pump_room_user_list_fuse_at(now));
        assert!(
            session.game_mut().dialog_owns_input(),
            "the fuse sheet owns input before OK"
        );
        let (ok_x, ok_y) = information_ok_centre();
        session.ingest_snapshot(&InputSnapshot::click_at(ok_x, ok_y));
        assert_eq!(
            session.game_mut().room_no_response_dismissal_queued(),
            Some(100),
            "OK posts the type-100 dismissal"
        );
        paint(&mut session);
        assert!(
            !session.game_mut().dialog_owns_input(),
            "the paint pump closed the last dialog"
        );
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);

        double_click(&mut handler);

        assert!(
            handler.queued_ui_notices().is_empty(),
            "a dialog closed by the paint pump must not hold the gate"
        );

        drop(handler);
        gateway.server.join().unwrap();
    }
}
