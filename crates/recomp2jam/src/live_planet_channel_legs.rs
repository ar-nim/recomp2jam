//! Live-input legs for the Planet board and the Channel dialog.
//! Boot/gateway/paint helpers come from `crate::live_leg_support` (#926);
//! every leg skips cleanly when the installed archives are absent (CI has
//! no game data, #0a3ba).

#[cfg(test)]
mod live_planet_channel_leg_tests {
    use crate::live_leg_support::*;

    use o2jam_net::ChannelState;
    use o2jam_render::Rect;
    use o2jam_shell::screens::game::PlanetModal;
    use o2jam_shell::screens::side::{
        PlanetBand, PLANET_BUTTON_EASY_CODE, PLANET_BUTTON_HARD_CODE, PLANET_BUTTON_NORMAL_CODE,
        PLANET_EXIT_CODE, PLANET_OPTION_CODE,
    };
    use o2jam_shell::{GameLoop, InputEdge, InputSnapshot, MenuKey, Screen};

    /// Painted Channel row band (the four exe-pinned row rects plus the
    /// scroll column), used to prove a scroll arrow moved the viewport.
    const CHANNEL_ROWS_RECT: Rect = Rect {
        x: 216,
        y: 224,
        w: 378,
        h: 212,
    };

    /// Native X plate of the Channel dialog, live-verified 2026-11-09
    /// and pinned by `CHANNEL_DIALOG_X_RECT` (`docs/channel-fill.md`).
    const CHANNEL_X: (i32, i32) = (577, 171);

    /// Scroll arrows on the Channel track: origin (573,213) plus the
    /// bnd-pinned locals (0,0)/(0,196), centres of the 21x20 plates.
    const CHANNEL_SCROLL_UP: (i32, i32) = (583, 223);
    const CHANNEL_SCROLL_DOWN: (i32, i32) = (583, 419);

    /// Enter face of Channel row 0, bnd-local (285,11) over the row
    /// origin (216,224), live-verified 2026-11-09 (`docs/channel-fill.md`).
    const CHANNEL_FACE_ROW_0: (i32, i32) = (524, 246);

    /// Park the pointer away from the Channel rows so a painted-frame
    /// comparison measures the viewport, not a hover plate.
    fn park_pointer_off_rows(session: &mut crate::session::OnlineSession) {
        let mut snapshot = InputSnapshot::new();
        snapshot.set_pointer(Some((10, 10)));
        session.ingest_snapshot(&snapshot);
    }

    /// Six active rows give the down arrow one step of viewport travel;
    /// the boot gateway's single resident row cannot show a scroll step.
    fn six_channel_rows() -> Vec<ChannelState> {
        (2u16..8)
            .map(|channel_id| ChannelState {
                gateway_id: 1,
                channel_id,
                capacity: 100,
                population: 5,
                active: true,
            })
            .collect()
    }

    /// One Escape edge through the same snapshot route as a real key.
    fn press_escape(session: &mut crate::session::OnlineSession) {
        let mut snapshot = InputSnapshot::new();
        snapshot.push_edge(InputEdge::Menu {
            key: MenuKey::Back,
            down: true,
        });
        session.ingest_snapshot(&snapshot);
    }

    /// #1090 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg_with_prestaged_art -> dismiss_splash
    ///   -> GameLoop::ingest_snapshot (staged emblem press/release)
    ///   -> ScreenGame::click_planet -> dispatch_planet_action
    ///   -> ScreenGame::enter(Channel).
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_planet_emblem_clicks_from_boot_open_the_channel_dialog() {
        if !archives_present() {
            println!("SKIP live_planet_emblem_clicks_from_boot_open_the_channel_dialog: missing install-root interface archive");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        dismiss_splash(&mut session);
        assert_eq!(session.screen(), Screen::Planet);

        for (code, band) in [
            (PLANET_BUTTON_HARD_CODE, PlanetBand::Hard),
            (PLANET_BUTTON_NORMAL_CODE, PlanetBand::Normal),
            (PLANET_BUTTON_EASY_CODE, PlanetBand::Easy),
        ] {
            let (x, y) = staged_code_centre(&mut session, Screen::Planet, code);
            session.ingest_snapshot(&InputSnapshot::click_at(x, y));
            assert_eq!(session.screen(), Screen::Channel, "emblem {code:#x}");
            assert_eq!(
                session.game_mut().planet_selection(),
                Some(band),
                "emblem {code:#x} must record its family"
            );

            session.ingest_snapshot(&InputSnapshot::click_at(CHANNEL_X.0, CHANNEL_X.1));
            assert_eq!(session.screen(), Screen::Planet, "X must close Channel");
        }

        finish(session, gateway.server);
    }

    /// #1091 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg_with_prestaged_art -> dismiss_splash
    ///   -> GameLoop::ingest_snapshot (staged disc press/release)
    ///   -> ScreenGame::click_planet -> dispatch_planet_action
    ///   -> PlanetBoardState::apply_action -> PlanetModal.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_planet_options_and_exit_clicks_open_their_dialogs_from_boot() {
        if !archives_present() {
            println!("SKIP live_planet_options_and_exit_clicks_open_their_dialogs_from_boot: missing install-root interface archive");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        dismiss_splash(&mut session);

        let (x, y) = staged_code_centre(&mut session, Screen::Planet, PLANET_OPTION_CODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(
            session.game_mut().planet_modal(),
            Some(PlanetModal::Options),
            "the gear disc must open Options over the bare board"
        );
        assert_eq!(session.screen(), Screen::Planet, "Options keeps its host");

        press_escape(&mut session);
        assert_eq!(
            session.game_mut().planet_modal(),
            None,
            "Escape closes Options"
        );

        let (x, y) = staged_code_centre(&mut session, Screen::Planet, PLANET_EXIT_CODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(
            session.game_mut().planet_modal(),
            Some(PlanetModal::QuitConfirm),
            "the power disc must open the quit-confirm sheet"
        );

        press_escape(&mut session);
        assert_eq!(
            session.game_mut().planet_modal(),
            None,
            "Escape cancels the quit-confirm sheet"
        );
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1092 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg_with_prestaged_art -> dismiss_splash
    ///   -> GameLoop::ingest_snapshot (staged emblem press/release)
    ///   -> ScreenGame::click_planet -> Screen::Channel
    ///   -> GameLoop::ingest_snapshot (scroll arrow / X / row face)
    ///   -> ScreenGame::click_dispatch -> ScreenGame::click_channel_row
    ///   -> ChannelScene::press -> Screen::Planet | Screen::Room.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_channel_dialog_row_x_and_scroll_clicks_answer_from_boot() {
        if !archives_present() {
            println!("SKIP live_channel_dialog_row_x_and_scroll_clicks_answer_from_boot: missing install-root interface archive");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        dismiss_splash(&mut session);

        let (x, y) = staged_code_centre(&mut session, Screen::Planet, PLANET_BUTTON_EASY_CODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(session.screen(), Screen::Channel);

        session.game_mut().set_channels(six_channel_rows());
        park_pointer_off_rows(&mut session);
        let before = staged_rect_pixels_from(&paint_frame(&mut session), CHANNEL_ROWS_RECT);

        session.ingest_snapshot(&InputSnapshot::click_at(
            CHANNEL_SCROLL_DOWN.0,
            CHANNEL_SCROLL_DOWN.1,
        ));
        assert_eq!(
            session.screen(),
            Screen::Channel,
            "the arrow stays in Channel"
        );
        park_pointer_off_rows(&mut session);
        let after = staged_rect_pixels_from(&paint_frame(&mut session), CHANNEL_ROWS_RECT);
        assert_ne!(before, after, "the down arrow must move the visible rows");

        session.ingest_snapshot(&InputSnapshot::click_at(
            CHANNEL_SCROLL_UP.0,
            CHANNEL_SCROLL_UP.1,
        ));
        park_pointer_off_rows(&mut session);
        let restored = staged_rect_pixels_from(&paint_frame(&mut session), CHANNEL_ROWS_RECT);
        assert_eq!(restored, before, "the up arrow must restore the viewport");

        session.ingest_snapshot(&InputSnapshot::click_at(CHANNEL_X.0, CHANNEL_X.1));
        assert_eq!(
            session.screen(),
            Screen::Planet,
            "the X must close the dialog"
        );

        let (x, y) = staged_code_centre(&mut session, Screen::Planet, PLANET_BUTTON_EASY_CODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(session.screen(), Screen::Channel);
        session.ingest_snapshot(&InputSnapshot::click_at(
            CHANNEL_FACE_ROW_0.0,
            CHANNEL_FACE_ROW_0.1,
        ));
        assert_eq!(
            session.screen(),
            Screen::Room,
            "the active row's enter face must enter Room"
        );

        finish(session, gateway.server);
    }
}
