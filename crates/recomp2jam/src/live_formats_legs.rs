//! Tier 3 live legs for the file-format staging family (#1106).
//! Boot/gateway/paint helpers come from `crate::live_leg_support` (#926);
//! every leg skips cleanly when the installed archives are absent (CI
//! has no game data, #0a3ba).

#[cfg(test)]
mod live_formats_spec_leg_tests {
    use o2jam_file::data_root::install_root;
    use o2jam_shell::options::{WindowCursorStage, WindowCursorSurface};
    use o2jam_shell::screens::paint::{
        window_cursor_arm, SceneArt, WindowCursorArm, WINDOW_CURSOR_SPRITE,
    };
    use o2jam_shell::staging::read_install_archives;
    use o2jam_shell::Screen;

    use crate::live_leg_support::*;

    /// #1106 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> first pointer use ->
    ///   options::WindowCursorStage::stage_on_first_use ->
    ///   screens::paint::window_cursor_arm.
    /// seam: the headless client has no OS cursor handle, so the one
    /// load and the staged-versus-stand-in arm are the observable
    /// behavior; the pointer's live pixels stay Tier 4 OPEN. The
    /// existing `planet_selection_paints_the_amber_cursor_stand_in`
    /// guard pins the stand-in, not exe pointer art.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_window_cursor_stages_the_exe_sprite_on_first_use() {
        if !archives_present() {
            println!(
                "SKIP live_window_cursor_stages_the_exe_sprite_on_first_use: install-root absent"
            );
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive behind the guard");
        let figure = o2jam_shell::load_archive_sprite(&archive, WINDOW_CURSOR_SPRITE)
            .expect("the installed Window_Cursor.ojs decodes");
        let layer = figure.layer(0).expect("the sprite has one recorded body");
        let surface = WindowCursorSurface::from_dimensions(layer.image.w, layer.image.h)
            .expect("sprite dimensions fit the exe words");

        let mut stage = WindowCursorStage::default();
        assert_eq!(stage.load_attempts(), 0, "no staging before first use");
        assert_eq!(
            window_cursor_arm(&stage, &SceneArt::default()),
            WindowCursorArm::AmberStandIn,
            "the stand-in is the only arm before the first use"
        );
        assert_eq!(stage.stage_on_first_use(Some(surface)), Some(surface));

        let mut art = SceneArt::default();
        art.figures
            .insert(WINDOW_CURSOR_SPRITE.to_owned(), figure.clone());
        assert_eq!(
            window_cursor_arm(&stage, &art),
            WindowCursorArm::Sprite(&figure),
            "the first use mounts the installed exe sprite"
        );
        assert_eq!(stage.load_attempts(), 1, "no second load");
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }
}
