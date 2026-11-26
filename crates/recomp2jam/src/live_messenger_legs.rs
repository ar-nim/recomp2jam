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
    use o2jam_render::{NATIVE_HEIGHT, NATIVE_WIDTH};
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

    /// Boot through the live gateway, then hand the session to the
    /// window handler that owns the error arms and the paint path.
    fn boot_error_leg() -> (WindowHandler, std::thread::JoinHandle<()>) {
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        assert_eq!(session.screen(), Screen::Planet);
        let handler = WindowHandler::new(Shell::new(Box::new(session)));
        (handler, gateway.server)
    }

    /// Paint one native frame through the shell's game loop.
    fn paint_handler_frame(handler: &mut WindowHandler) -> Vec<u32> {
        let mut buffer = vec![0u32; NATIVE_WIDTH as usize * NATIVE_HEIGHT as usize];
        handler.paint_popup_frame(&mut buffer, NATIVE_WIDTH as usize, NATIVE_HEIGHT as usize);
        buffer
    }

    /// #1112 Tier 3. seam: the boot session has no audio rig, and
    /// the sheet host has no BGM caller in this client (`N-AUDIO`).
    /// The NullBackend cue plus the boot lobby assertion are the whole
    /// coexistence evidence this leg can give; Tier 4 must confirm
    /// the owned sink on a real client. Archives absent -> skip.
    #[test]
    fn live_messenger_sheet_keeps_the_boot_scene_cue_on_the_null_backend() {
        if !archives_present() {
            println!("SKIP live_messenger_sheet_keeps_the_boot_scene_cue_on_the_null_backend: missing install-root interface archive");
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
            println!("SKIP live_display_mode_change_plans_the_native_rebuild_from_boot: missing install-root interface archive");
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
            println!("SKIP live_cash_bill_and_bingo_ui_events_open_one_sheet_after_the_prior_closes: missing install-root interface archive");
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
            println!("SKIP live_qa_ui_event_uses_the_exe_scene_two_rect_shift: missing install-root interface archive");
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
            println!("SKIP live_both_global_web_sheets_show_then_hide_on_the_app_event: missing install-root interface archive");
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
            println!("SKIP live_uhp_start_names_the_success_caption_and_4046_frame: missing install-root interface archive");
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
            println!("SKIP live_error_popup_accepts_supplied_text_and_mapped_codes: missing install-root interface archive");
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

    /// #1118 Tier 3. seam: no production caller reaches the induced
    /// failure hook; it stands in for the missing fatal-error caller.
    /// The boot session and the window redraw paint path from the hook
    /// to pixels are real. Archives absent -> skip cleanly.
    #[test]
    fn live_supplied_error_paints_a_popup_over_the_boot_frame() {
        if !archives_present() {
            println!("SKIP live_supplied_error_paints_a_popup_over_the_boot_frame: missing install-root interface archive");
            return;
        }
        let (mut handler, server) = boot_error_leg();
        let scene = paint_handler_frame(&mut handler);
        let decision = handler.raise_error_from_supplied("display driver failed", true);
        assert_eq!(
            decision,
            ErrorPopup::ShowMessage("display driver failed".to_owned())
        );
        let first = paint_handler_frame(&mut handler);
        assert!(scene != first, "the supplied error must paint a popup");

        handler.raise_error_from_supplied("surface lock failed", true);
        let second = paint_handler_frame(&mut handler);
        assert!(
            first != second,
            "distinct supplied strings must paint distinct boot ink"
        );

        handler.raise_error_from_supplied("", true);
        let empty = paint_handler_frame(&mut handler);
        assert!(
            first != empty && second != empty,
            "supplied text must differ from an empty-message popup"
        );
        drop(handler);
        server.join().unwrap();
    }

    /// #1119 Tier 3. seam: no production caller reaches the induced
    /// failure hook; it stands in for the missing fatal-error caller.
    /// The code map and the window redraw paint path are real.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_mapped_error_code_paints_the_pinned_text_over_the_boot_frame() {
        if !archives_present() {
            println!("SKIP live_mapped_error_code_paints_the_pinned_text_over_the_boot_frame: missing install-root interface archive");
            return;
        }
        let (mut handler, server) = boot_error_leg();
        let before = paint_handler_frame(&mut handler);
        let decision = handler
            .raise_error_from_code(0x8876_0091, true)
            .expect("a nonzero code stages a popup");
        assert_eq!(
            decision,
            ErrorPopup::ShowMessage("The pixel format was invalid as specified.".to_owned())
        );
        let after = paint_handler_frame(&mut handler);
        assert!(after != before, "the mapped code must change boot pixels");

        handler.dismiss_error_popup();
        handler
            .raise_error_from_code(0xDEAD_BEEF, true)
            .expect("an unmapped code stages the fallback text");
        let fallback_frame = paint_handler_frame(&mut handler);
        assert!(
            after != fallback_frame,
            "the pinned message must differ from the fallback ink"
        );
        drop(handler);
        server.join().unwrap();
    }

    /// #1119 Tier 3. seam: no production caller reaches the induced
    /// failure hook; code zero takes the exe's silent arm before paint.
    /// Archives absent -> skip cleanly.
    // Evidence: docs/pins/input-msg.md:337 (0x5a36d0 shows the box only for a nonzero code).
    #[test]
    fn live_zero_error_code_leaves_the_boot_frame_unchanged() {
        if !archives_present() {
            println!("SKIP live_zero_error_code_leaves_the_boot_frame_unchanged: missing install-root interface archive");
            return;
        }
        let (mut handler, server) = boot_error_leg();
        let before = paint_handler_frame(&mut handler);
        assert_eq!(handler.raise_error_from_code(0, true), None);
        let after = paint_handler_frame(&mut handler);
        assert!(after == before, "code zero must stay silent");
        drop(handler);
        server.join().unwrap();
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
            println!("SKIP live_cursor_request_applies_the_persisted_mode_to_the_pointer_arm: missing install-root interface archive");
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
            println!("SKIP live_window_close_takes_the_boot_session_to_the_exit_decision: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        assert_eq!(session.screen(), Screen::Planet);
        assert_eq!(window_close_disposition(), QuitDisposition::ExitLoop);
        finish(session, gateway.server);
    }
}
