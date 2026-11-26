//! Tier 3 live legs for the window-input family (#1122).
//! The boot path is the real gateway entry; the window arm uses the
//! public headless press driver because winit has no double-click
//! event. Missing `install-root` archives skip the leg cleanly.

#[cfg(test)]
mod live_window_leg_tests {
    use crate::live_leg_support::*;
    use o2jam_shell::screens::messenger::WindowHandler;
    use o2jam_shell::screens::side::MyRoomNotice;
    use o2jam_shell::Shell;

    #[test]
    fn live_double_click_window_arm_queues_the_pinned_notice_from_boot() {
        // Live chain (Tier 3): `main.rs::run_play` ->
        // `connect::connect_and_login_with_transport` -> `boot_live_leg` ->
        // `OnlineSession::game_mut` -> `ScreenGame::apply_myroom_notice` ->
        // `ScreenGame::post_myroom_notice` (the host the 5035 wire arm
        // uses) -> `ScreenGame::publish_dialog_host_mode` ->
        // `WindowHandler::route_window_left_press` ->
        // `WindowHandler::route_mouse_input` ->
        // `WindowHandler::stage_double_click_at_native` ->
        // `is_double_click_pair` -> `double_click_notice` ->
        // `WindowHandler::queued_ui_notices`.
        // The 5035 wire path calls the same real host in
        // `session.rs::apply_payment_frame`; no test-only error flag is
        // raised here. winit emits no `WM_LBUTTONDBLCLK`, so the two
        // press calls take the same motion/press path as real events.
        if !archives_present() {
            println!("SKIP live_double_click_window_arm_queues_the_pinned_notice_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        session
            .game_mut()
            .apply_myroom_notice(MyRoomNotice::PenaltyCleared);
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);

        let notices = handler.queued_ui_notices();
        assert_eq!(notices.len(), 1, "one double-click queues one notice");
        let notice = notices[0];
        assert_eq!((notice.kind, notice.code, notice.param), (10, 3, 0));
        assert_eq!((notice.x, notice.y), (21, 42));

        drop(handler);
        gateway.server.join().unwrap();
    }

    #[test]
    fn live_double_click_with_no_dialog_from_boot_queues_nothing() {
        // Same boot and same press pair, but the real host has no
        // Information sheet up, so bit 2 is clear and `0x548b10`'s
        // silent arm is the whole claim. Archives absent -> skip.
        if !archives_present() {
            println!("SKIP live_double_click_with_no_dialog_from_boot_queues_nothing: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let session = boot_live_leg(gateway.port, false);
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);

        assert!(
            handler.queued_ui_notices().is_empty(),
            "no dialog must leave the input ring untouched"
        );

        drop(handler);
        gateway.server.join().unwrap();
    }
}
