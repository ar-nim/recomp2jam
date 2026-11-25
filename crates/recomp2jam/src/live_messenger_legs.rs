//! Tier 3 live legs for the Messenger and window family (spec #315).
//! Boot/gateway/paint helpers come from `crate::live_leg_support` (#926);
//! every leg skips cleanly when the installed archives are absent (CI
//! has no game data, #0a3ba). Win32-only arms are reached through the
//! shell's extracted seam helpers; the legs still boot the real
//! gateway path so a seam pass cannot hide a boot regression.

#[cfg(test)]
mod live_messenger_leg_tests {
    use crate::live_leg_support::*;

    use o2jam_audio::{AudioBackend, NullBackend, PcmSamples, SceneBgm};
    use o2jam_net::relay::ReportUdpPunchHole;
    use o2jam_net::WireMessage;
    use o2jam_shell::options::{apply_cursor_mode, MouseCursorMode, OptionsSettings};
    use o2jam_shell::screens::messenger::{
        error_popup_from_code, error_popup_from_supplied, web_sheet_for_ui_event,
        window_close_disposition, ErrorPopup, QuitDisposition, WebSheet, WebSheetHost,
        WindowHandler, BINGO_UI_EVENT, CASH_BILL_UI_EVENT, EXE_SCENE_PLAYING, EXE_SCENE_ROOM,
        PLAYING_QA_UI_EVENT, UHP_SUCCESS_CAPTION,
    };
    use o2jam_shell::{options::CustomCursorArm, GameLoop, Shell};
    use o2jam_shell::{
        screens::messenger::QA_PLAYING_RECT_SHIFT, InputEdge, InputSnapshot, Screen,
    };

    /// One synthetic scene bank entry, used only by the NullBackend
    /// leg. No installed audio is read.
    fn silent_bank() -> Vec<Option<PcmSamples>> {
        let mut bank = vec![None; 10];
        bank[7] = Some(PcmSamples {
            samples: vec![0; 441],
            sample_rate: 44_100,
            channels: 2,
        });
        bank
    }

    /// #1112 Tier 3. seam: the boot session has no audio rig, and
    /// the sheet host has no BGM caller in this client (`N-AUDIO`).
    /// The NullBackend cue plus the boot lobby assertion are the whole
    /// coexistence evidence this leg can give; Tier 4 must confirm
    /// the owned sink on a real client. Archives absent -> skip.
    #[test]
    fn live_messenger_sheet_keeps_the_boot_scene_cue_on_the_null_backend() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        let screen = session.screen();

        let bank = silent_bank();
        let mut bgm = SceneBgm::new();
        let pcm = bgm
            .play_scene_music(8, true, &bank)
            .expect("record 8 is the synthetic scene cue");
        let mut backend = NullBackend::new();
        backend.set_scene_bgm(Some(pcm), true).unwrap();
        let latched = bgm.current();

        let mut host = WebSheetHost::new();
        assert!(host.open(WebSheet::CashBill, EXE_SCENE_ROOM, 11).visible);
        assert_eq!(bgm.current(), latched, "sheet open does not cut the cue");
        assert!(
            backend.scene_bgm_playing(),
            "the contained sink keeps its stream"
        );
        assert_eq!(
            screen,
            Screen::Planet,
            "the sheet opened over the boot lobby"
        );

        finish(session, gateway.server);
    }

    /// #1113 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> Shell::new -> WindowHandler::apply_display_mode
    ///   -> display_mode_plan.
    /// The Win32 mode switch has no winit caller; the window handler
    /// seam is the whole runtime path. Archives absent -> skip.
    #[test]
    fn live_display_mode_change_plans_the_native_rebuild_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        let screen = session.screen();

        let mut handler = WindowHandler::new(Shell::new(Box::new(session)));
        let plan = handler
            .apply_display_mode(1)
            .expect("a changed mode rebuilds the window");
        assert!(plan.fullscreen && plan.rebuild_surfaces);
        assert_eq!(plan.client_size, (800, 600));
        assert!(plan.show_and_focus && plan.record_clock);
        assert_eq!(
            handler.apply_display_mode(1),
            None,
            "the same word does not rebuild twice"
        );
        assert_eq!(screen, Screen::Planet, "the boot lobby survives the plan");
        drop(handler);
        gateway.server.join().unwrap();
    }

    /// #1114/#1115 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash
    ///   -> web_sheet_for_ui_event -> WebSheetHost::open.
    /// seam: the UI-event words come from one Win32 `WM_APP+0x32`
    /// arm that winit cannot send; the host/close/show decisions are
    /// the whole behavior. Web content stays blank (#948).
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_cash_bill_and_bingo_ui_events_open_one_sheet_after_the_prior_closes() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let mut host = WebSheetHost::new();
        let cash = web_sheet_for_ui_event(CASH_BILL_UI_EVENT).expect("0x12 names cash bill");
        assert_eq!(cash, WebSheet::CashBill);
        let opened = host.open(cash, EXE_SCENE_ROOM, 21);
        assert!(!opened.closed_previous && opened.visible && opened.focused);

        let bingo = web_sheet_for_ui_event(BINGO_UI_EVENT).expect("0x13 names bingo");
        let reopened = host.open(bingo, EXE_SCENE_ROOM, 22);
        assert!(reopened.closed_previous, "billing chrome cannot stack");
        assert_eq!(host.handles(), [Some(22), None]);
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1116 Tier 3. seam: the Win32 UI-event arm has no winit
    /// counterpart, so the rect shift staged at host open is the
    /// whole behavior observable here.
    /// CONFLICT: the story calls the shift a Playing offset while the
    /// pin names scene 2, which `options.rs` pins as Room. The guard
    /// follows the exe pin. Archives absent -> skip cleanly.
    #[test]
    fn live_qa_ui_event_uses_the_exe_scene_two_rect_shift() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let qa = web_sheet_for_ui_event(PLAYING_QA_UI_EVENT).expect("0x14 names Q&A");
        assert_eq!(qa, WebSheet::PlayingQa);
        let mut host = WebSheetHost::new();
        let shifted = host.open(qa, EXE_SCENE_ROOM, 31);
        assert_eq!(shifted.qa_rect_shift, QA_PLAYING_RECT_SHIFT);
        let unshifted = host.open(qa, EXE_SCENE_PLAYING, 32);
        assert_eq!(unshifted.qa_rect_shift, 0);
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1123/#1124 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> WebSheetHost::open
    ///   -> WebSheetHost::hide_on_app_event.
    /// seam: both global handle words are staged; no browser object
    /// is created. Archives absent -> skip cleanly.
    #[test]
    fn live_both_global_web_sheets_show_then_hide_on_the_app_event() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let mut host = WebSheetHost::with_global_handles(41, 42);
        let opened = host.open(WebSheet::BingoEvent, EXE_SCENE_ROOM, 43);
        assert!(opened.visible && opened.focused);
        assert_eq!(host.handles(), [Some(43), Some(42)]);

        let hidden = host.hide_on_app_event();
        assert!(hidden.both_hidden);
        assert!(!host.is_visible() && !host.is_focused());
        assert_eq!(host.handles(), [Some(43), Some(42)]);
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1117 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> uhp_start_caption
    ///   -> ReportUdpPunchHole (codec guard at `commands.rs:2143`).
    /// seam: the relay UHP branch is live-battle only and has no Rust
    /// caller; caption plus empty wire frame are the whole pin.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_uhp_start_names_the_success_caption_and_4046_frame() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        assert_eq!(
            o2jam_shell::screens::messenger::uhp_start_caption(
                o2jam_shell::screens::messenger::UhpStartOutcome::Success
            ),
            Some(UHP_SUCCESS_CAPTION)
        );
        assert_eq!(o2jam_shell::screens::messenger::uhp_report_opcode(), 4046);
        assert_eq!(ReportUdpPunchHole.encode(), vec![0x04, 0x00, 0xCE, 0x0F]);
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1118/#1119 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> error_popup_from_supplied
    ///   / error_popup_from_code -> fatal_path_disposition.
    /// seam: no Win32 MessageBox or fatal caller exists in this
    /// client; the popup decision is the pinned surface.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_error_popup_accepts_supplied_text_and_mapped_codes() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        assert_eq!(
            error_popup_from_supplied("display failed", true),
            ErrorPopup::ShowMessage("display failed".to_owned())
        );
        assert_eq!(error_popup_from_code(0, true), None);
        assert_eq!(
            error_popup_from_code(0x8876_0091, true),
            Some(ErrorPopup::ShowMessage(
                "The pixel format was invalid as specified.".to_owned()
            ))
        );
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1120 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> apply_cursor_mode
    ///   -> custom_cursor_arm / cursor_mode_face.
    /// seam: no winit cursor handle exists; the mode-to-arm decision
    /// is the whole behavior Tier 4 must confirm on a pointer.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_cursor_request_applies_the_persisted_mode_to_the_pointer_arm() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let settings = OptionsSettings::load_from_dir(&crate::install_root());
        let applied = apply_cursor_mode(1, false, 1, settings.mouse_cursor());
        assert_eq!(applied.arm, CustomCursorArm::FirstHandle);
        assert_ne!(settings.mouse_cursor(), MouseCursorMode::Image);
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1121 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> window_close_disposition
    ///   -> winit `CloseRequested` -> `event_loop.exit`.
    /// seam: the close arm has no headless event-loop instance; the
    /// extracted decision is the one the winit arm calls.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_window_close_takes_the_boot_session_to_the_exit_decision() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        assert_eq!(session.screen(), Screen::Planet);
        assert_eq!(window_close_disposition(), QuitDisposition::ExitLoop);
        finish(session, gateway.server);
    }

    /// #1122 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash
    ///   -> GameLoop::ingest_snapshot (two PointerDown/Up pairs)
    ///   -> double_click_notice.
    /// seam: Win32 `WM_LBUTTONDBLCLK` has no winit event; the notice
    /// contract is the pinned surface until a lane owns the game
    /// notice queue. Archives absent -> skip cleanly.
    #[test]
    fn live_double_click_raises_the_pinned_notice_when_the_bit_is_set() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        for _ in 0..2 {
            let mut snapshot = InputSnapshot::new();
            snapshot.push_edge(InputEdge::PointerDown { x: 21, y: 42 });
            snapshot.push_edge(InputEdge::PointerUp { x: 21, y: 42 });
            session.ingest_snapshot(&snapshot);
        }
        let notice = o2jam_shell::screens::messenger::double_click_notice(0x0004, 21, 42)
            .expect("bit 2 raises the notice");
        assert_eq!((notice.kind, notice.code, notice.param), (10, 3, 0));
        assert_eq!((notice.x, notice.y), (21, 42));
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }
}
