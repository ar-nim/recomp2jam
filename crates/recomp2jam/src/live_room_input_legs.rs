//! Live-input legs for the Room cursor switch (#985/#986).
//!
//! Boot/gateway/paint helpers come from `crate::live_leg_support` (#926).
//! Every leg skips cleanly when the installed archives are absent (CI has
//! no game data, #0a3ba).

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;

    // One install-root slot per worktree: F8 tests serialize their
    // write/restore of `option.ini` (CONCURRENCY-02).
    static OPTION_INI_LOCK: Mutex<()> = Mutex::new(());

    use o2jam_file::data_root::install_root as profile_root;
    use o2jam_shell::options::{self, MouseCursorMode};
    use o2jam_shell::screens::game::ScreenGame;
    use o2jam_shell::{route_key_event, GameLoop, InputSnapshot, Keymap, Screen};
    use winit::keyboard::KeyCode;

    use crate::live_leg_support::*;

    /// Runtime root used by the F8 write arm. It honors
    /// `O2JAM_INSTALL_ROOT`, the same override `gate-check.sh` exports
    /// for its hidden-data arm; the compile-time root in
    /// `crate::install_root()` does not. Tests that touch `option.ini`
    /// skip cleanly when this path is unreachable (#0a3ba).
    fn profile_root_available() -> bool {
        profile_root().is_dir()
    }

    /// Save and restore `option.ini` around a test that drives the real
    /// input route. The F8 arm writes the client's one persisted option
    /// store; a leg must never leave another boot with a changed pointer.
    struct OptionIniBackup {
        path: PathBuf,
        original: Option<Vec<u8>>,
    }

    impl OptionIniBackup {
        fn capture() -> Self {
            let path = profile_root().join("option.ini");
            Self {
                original: std::fs::read(&path).ok(),
                path,
            }
        }
    }

    impl Drop for OptionIniBackup {
        fn drop(&mut self) {
            match &self.original {
                Some(bytes) => {
                    if let Err(source) = std::fs::write(&self.path, bytes) {
                        panic!("restore option.ini: {source}");
                    }
                }
                None => {
                    let _ = std::fs::remove_file(&self.path);
                }
            }
        }
    }

    fn persisted_cursor_mode() -> MouseCursorMode {
        options::OptionsSettings::load_from_dir(&profile_root()).mouse_cursor()
    }

    /// A scene-labelled game loop for the window-input seam. The arm reads
    /// only `screen_name`, so a real `ScreenGame` is not needed to prove the
    /// route; the Tier 3 leg below drives the same arm from boot.
    struct SceneProbe(&'static str);

    impl GameLoop for SceneProbe {
        fn lane_key(&mut self, _lane: usize, _down: bool) {}
        fn ingest_snapshot(&mut self, _snapshot: &InputSnapshot) {}
        fn screen_name(&self) -> &'static str {
            self.0
        }
        fn paint(&mut self, _frame: &mut o2jam_render::Frame<'_>) {}
    }

    #[test]
    // Live chain (Tier 1): `WindowHandler::window_event` WM_KEYDOWN arm ->
    // `route_key_event` F8 arm -> `options::switch_cursor_in_install_root`
    // -> `OptionsSettings::switch_mouse_cursor` -> option.ini. The arm
    // captures the raw key, so no lane, menu edge, or chat text leaks.
    fn f8_key_arm_persists_the_cursor_value_in_the_room_scene() {
        if !profile_root_available() {
            println!("skip: install root unreachable");
            return;
        }
        let _lock = OPTION_INI_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _backup = OptionIniBackup::capture();
        let before = persisted_cursor_mode();
        let mut game = SceneProbe("room");
        let route = route_key_event(&mut game, &mut Keymap::new(), KeyCode::F8, true, None);
        assert!(route.captured, "F8 must be consumed at the window boundary");
        assert!(route.edges.is_empty() && route.text.is_none() && !route.erase);
        assert_ne!(
            persisted_cursor_mode(),
            before,
            "F8 must write the exe's 0/1 cursor arm to the one option store"
        );
    }

    #[test]
    // Live chain (Tier 1): `WindowHandler::window_event` WM_KEYDOWN arm ->
    // `route_key_event` F8 arm -> `options::f8_cursor_request_allowed` with
    // the exe's scene-id table. The song scene never reaches the write arm.
    fn f8_key_arm_is_silent_during_a_song() {
        if !profile_root_available() {
            println!("skip: install root unreachable");
            return;
        }
        let _backup = OptionIniBackup::capture();
        let before = persisted_cursor_mode();
        let mut game = SceneProbe("playing");
        let route = route_key_event(&mut game, &mut Keymap::new(), KeyCode::F8, true, None);
        assert!(!route.captured, "a refused F8 stays an undecided raw key");
        assert_eq!(persisted_cursor_mode(), before, "no cursor write in a song");
    }

    #[test]
    // Live chain (Tier 3): `main.rs::run_play` ->
    // `connect::connect_and_login_with_transport` -> `boot_live_leg_with_art`
    // -> `enter_room_by_clicks` (pointer snapshots only) ->
    // `route_raw_key` -> `route_key_event` F8 arm ->
    // `options::switch_cursor_in_install_root` -> option.ini `Mouse_Cursor`.
    // Archives absent -> skip cleanly (CI has no game data).
    fn f8_from_boot_room_flips_the_persisted_mouse_cursor() {
        if !profile_root_available() || !archives_present() {
            println!("skip: install root unreachable");
            return;
        }
        let _lock = OPTION_INI_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let _backup = OptionIniBackup::capture();
        let before = persisted_cursor_mode();
        let gateway = spawn_live_leg_gateway(joined_room(0, false));
        let port = gateway.port;
        let mut session = boot_live_leg_with_art(port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        assert_eq!(
            session.screen(),
            Screen::Room,
            "F8 leg starts on the Room board"
        );

        let route = route_raw_key(&mut session, &mut Keymap::new(), KeyCode::F8, None);
        assert!(route.captured, "the Room scene owns the F8 key");
        let after = persisted_cursor_mode();
        assert_ne!(
            after, before,
            "boot -> Room -> F8 must persist the other arm"
        );
        assert!(
            options::last_cursor_caption().is_some(),
            "the write arm chose one of the two pinned captions"
        );
        paint(&mut session);
        finish(session, gateway.server);
    }

    #[test]
    fn boot_gate_yields_a_fresh_screen_game_without_a_cursor_write() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        let route = route_key_event(&mut game, &mut Keymap::new(), KeyCode::F8, true, None);
        assert!(!route.captured, "splash has no pinned F8 arm");
    }
}
