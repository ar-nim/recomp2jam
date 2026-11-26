//! Tier 3 live legs for the window-input family (#1122).
//! The boot path is the real gateway entry; the window arm uses the
//! public headless press driver because winit has no double-click
//! event. Missing `install-root` archives skip the leg cleanly.

#[cfg(test)]
mod live_window_leg_tests {
    use crate::live_leg_support::*;
    use o2jam_shell::screens::messenger::{WindowHandler, UNRECOGNIZED_ERROR_TEXT};
    use o2jam_shell::Shell;

    #[test]
    fn live_double_click_window_arm_stages_the_pinned_notice_from_boot() {
        // Live chain (Tier 3): `main.rs::run_play` ->
        // `connect::connect_and_login_with_transport` -> `boot_live_leg` ->
        // `Shell::new` -> `WindowHandler::route_window_left_press` ->
        // `WindowHandler::route_mouse_input` ->
        // `WindowHandler::stage_double_click_at_native` ->
        // `is_double_click_pair` -> `double_click_notice` ->
        // `WindowHandler::staged_ui_notices`.
        // winit emits no `WM_LBUTTONDBLCLK`; the two press driver calls
        // take the same motion/press path the real window event arms take.
        if !archives_present() {
            println!("SKIP live_double_click_window_arm_stages_the_pinned_notice_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let session = boot_live_leg(gateway.port, false);
        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        handler.set_window_surface_size(800, 600);
        // The exe sets the control mode word to 0x304 when the one-OK
        // information sheet shows (0x433c90); our dialog host is the
        // same writer, so bit 2 is set for this leg.
        handler.raise_error_from_supplied(UNRECOGNIZED_ERROR_TEXT, true);

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);

        let notices = handler.staged_ui_notices();
        assert_eq!(notices.len(), 1, "one double-click stages one notice");
        let notice = notices[0];
        assert_eq!((notice.kind, notice.code, notice.param), (10, 3, 0));
        assert_eq!((notice.x, notice.y), (21, 42));

        drop(handler);
        gateway.server.join().unwrap();
    }
}
