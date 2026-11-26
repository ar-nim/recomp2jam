//! Tier 3 live legs for the Payment/Payment2 spec families (#296/#297).
//! Boot/gateway/paint helpers come from `crate::live_leg_support` (#926);
//! every leg skips cleanly when the installed archives are absent (CI
//! has no game data, #0a3ba). The web content itself stays outside
//! this client by owner pin (`docs/parity/payment.md`).

#[cfg(test)]
mod live_payment_spec_leg_tests {
    use crate::live_leg_support::*;

    use o2jam_shell::screens::messenger::{
        billing_window_arm, print_screen_file_name, BillingWebEvent, BillingWebHandle,
        BillingWebPage, BillingWindowArm, PrintScreenAction, ScreenShotCounter,
        BILLING_PAYMENT2_STATE_WORD, BILLING_PAYMENT_STATE_WORD, SCREENSHOT_DIMENSIONS,
        SCREENSHOT_FOLDER_SUFFIX,
    };
    use o2jam_shell::Screen;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// #1099 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> window hub 0x5a5190
    ///   (`WM_APP+0x32`, state word 12) -> billing_window_arm ->
    ///   BillingWebHandle::rebuild (exe 0x4f3d00) ->
    ///   ClosedPrevious then Mounted.
    /// seam: no WebCtrl32 window exists in the winit client, so the
    /// close-before-mount order is the whole behavior observable here.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_payment_web_rebuild_closes_the_old_window_before_the_new_page() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        assert_eq!(
            billing_window_arm(0x232, BILLING_PAYMENT_STATE_WORD, true),
            BillingWindowArm::RebuildPayment
        );
        let mut handle = BillingWebHandle::with_handle(61);
        let steps = handle.rebuild(BillingWebPage::Payment, "2", 62);
        assert_eq!(steps.first(), Some(&BillingWebEvent::ClosedPrevious(61)));
        assert_eq!(handle.handle(), Some(62));
        assert!(steps.iter().any(|step| matches!(
            step,
            BillingWebEvent::Mounted { url, .. }
                if url.ends_with("payment_input.aspx?USER_INDEX_ID=2")
        )));
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1100/#469 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> window hub 0x5a5190
    ///   (`WM_APP+0x32`, state word 13) -> billing_window_arm ->
    ///   BillingWebHandle::rebuild (exe 0x4f5020 -> 0x4f4bd0) ->
    ///   jamcard/coupon URL with USER_INDEX_ID.
    /// seam: the gift-page branch condition stays unpinned, so the
    /// leg supplies the explicit branch and pins the two URL shapes;
    /// no qualified exe gift mount frame is claimed (#469 stays OPEN).
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_gift_web_rebuild_keeps_both_pinned_urls_on_the_session_id() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        assert_eq!(
            billing_window_arm(0x232, BILLING_PAYMENT2_STATE_WORD, true),
            BillingWindowArm::RebuildGift
        );
        let mut handle = BillingWebHandle::with_handle(71);
        let jamcard = handle.rebuild(BillingWebPage::JamCard, "2", 72);
        assert_eq!(jamcard.first(), Some(&BillingWebEvent::ClosedPrevious(71)));
        assert!(jamcard.iter().any(|step| matches!(
            step,
            BillingWebEvent::Mounted { url, .. }
                if url == "http://game1.o2jam.com/client/jamcard.asp?USER_INDEX_ID=2"
        )));

        let coupon = handle.rebuild(BillingWebPage::Coupon, "2", 73);
        assert_eq!(coupon.first(), Some(&BillingWebEvent::ClosedPrevious(72)));
        assert!(coupon.iter().any(|step| matches!(
            step,
            BillingWebEvent::Mounted { url, .. }
                if url == "http://o2jam.nopp.co.kr/client/coupon2/coupon.phtml?USER_INDEX_ID=2"
        )));
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1101/#1102 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> 0x5a5190 key-down gate
    ///   (`0x2c` outside states 10/11) -> 0x418dc0 -> ScreenShotCounter::press.
    /// seam: no player Print Screen writer or JPEG encoder exists in
    /// this client; path/name, 800x600/quality 80 and the exhausted
    /// notice branch are the whole headless claim. JPEG bytes stay
    /// OPEN for a Tier 4 Wine leg.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_print_screen_counts_presses_and_notices_when_the_counter_is_empty() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let mut counter = ScreenShotCounter::new(2);
        assert_eq!(
            counter.press(10),
            PrintScreenAction::Suppressed { state: 10 },
            "state 10 writes nothing"
        );
        let first = counter.press(0);
        assert!(matches!(
            first,
            PrintScreenAction::Write {
                width: 800,
                height: 600,
                quality: 80,
                ..
            }
        ));
        let second = counter.press(0);
        match (first, second) {
            (
                PrintScreenAction::Write {
                    file_name: first, ..
                },
                PrintScreenAction::Write {
                    file_name: second, ..
                },
            ) => {
                assert_ne!(first, second, "each press gets a new name");
                assert_eq!(first, print_screen_file_name(0));
                assert_eq!(second, print_screen_file_name(1));
            }
            other => panic!("two available presses must write, got {other:?}"),
        }
        assert_eq!(counter.press(0), PrintScreenAction::Notice);
        assert_eq!(counter.used(), 2);
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// Unique scratch root under `~/scratch`: the live leg writes real
    /// bytes here and never in a user's Pictures folder.
    struct ScratchRoot(PathBuf);

    impl ScratchRoot {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock follows the epoch")
                .as_nanos();
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/home/arnim"));
            let dir = home.join("scratch").join(format!(
                "screenshot-live-{label}-{}-{nanos}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).expect("scratch root is available");
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for ScratchRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// #1101 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> ScreenShotCounter::capture
    ///   -> PrintScreenAction::write_jpeg -> image::JpegEncoder.
    /// seam: the headless walk cannot inject a winit Snapshot event, so
    /// writing one synthetic native frame from the booted session is the
    /// whole headless claim. A Tier 4 Wine key/file leg stays OPEN.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_print_screen_writer_writes_a_numbered_jpeg_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let scratch = ScratchRoot::new("writer");
        let width = usize::try_from(SCREENSHOT_DIMENSIONS.0).expect("800 fits usize");
        let height = usize::try_from(SCREENSHOT_DIMENSIONS.1).expect("600 fits usize");
        let frame = vec![o2jam_render::pack_xrgb(16, 200, 16); width * height];
        let mut counter = ScreenShotCounter::new(2);

        let result = counter.capture(0, scratch.path(), &frame);

        assert!(result.is_written(), "booted session writes a screenshot");
        let expected = scratch
            .path()
            .join(SCREENSHOT_FOLDER_SUFFIX)
            .join(print_screen_file_name(0));
        assert_eq!(result.path(), Some(expected.as_path()));
        let bytes = std::fs::read(&expected).expect("screenshot file exists");
        assert!(bytes.len() > 2, "JPEG bytes exist");
        assert_eq!(&bytes[..2], &[0xff, 0xd8], "JPEG SOI marker");
        assert_eq!(&bytes[bytes.len() - 2..], &[0xff, 0xd9], "JPEG EOI marker");
        assert_eq!(counter.used(), 1, "a successful write consumes one name");
        assert_eq!(session.screen(), Screen::Planet);

        finish(session, gateway.server);
    }

    /// #1102 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> dismiss_splash -> 0x5a5190 key-down gate
    ///   -> 0x418dc0 -> ScreenShotCounter::press ->
    ///   PrintScreenAction::notice_event.
    /// seam: no player Print Screen writer or UI queue exists in this
    /// client, so the exact event words are the whole headless claim;
    /// no caption is invented.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_exhausted_print_screen_posts_the_pinned_event_words() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);

        let notice = ScreenShotCounter::new(0).press(0);
        let [code, subcode, third, fourth] = notice
            .notice_event()
            .expect("an exhausted press posts the UI event");
        assert_eq!(
            (code, subcode, third, fourth),
            (0x12, 0x67, 0, 0),
            "UI event (0x12, 0x67, 0, 0)"
        );

        finish(session, gateway.server);
    }
}

#[cfg(test)]
mod live_cash_bill_host_leg_tests {
    use crate::live_leg_support::*;

    use o2jam_shell::screens::messenger::{
        cash_charge_publisher_for_ui_event, BillingWebEvent, BillingWebHandle, CashBillPublisher,
        CASH_BILL_UI_EVENT,
    };
    use o2jam_shell::screens::side::SideScreen;
    use o2jam_shell::Screen;

    const PINNED_CASH_HOSTS: [&str; 5] = [
        "o2jam.monawa.com",
        "o2jam.gameangel.com",
        "o2jam.gameon.nate.com",
        "o2jam.pdbox.co.kr",
        "o2jam.nopp.co.kr",
    ];
    const PINNED_CASH_PATH: &str = "client/cash_bill/index.phtml";

    /// #1143 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg_with_art -> dismiss_splash -> window hub
    ///   0x5a5190 (`WM_APP+0x32`, UI event 0x12) ->
    ///   cash_charge_publisher_for_ui_event(publisher) ->
    ///   BillingWebHandle::rebuild_cash_bill_charge -> one mounted
    ///   cash-bill URL per publisher, with no new wire frame.
    /// seam: the exe host-selection condition at `0x45e010` is OPEN;
    /// this leg supplies the publisher and never claims an exe gate.
    /// Archives absent -> skip cleanly.
    #[test]
    fn live_native_cash_charge_covers_all_five_publisher_hosts_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        assert_eq!(session.screen(), Screen::Room);
        let wire_before = observed(&gateway.reports).len();

        let mut handle = BillingWebHandle::with_handle(61);
        for (index, publisher) in CashBillPublisher::ALL.into_iter().enumerate() {
            let new_handle = 62 + u16::try_from(index).expect("five hosts fit u16");
            let selected = cash_charge_publisher_for_ui_event(CASH_BILL_UI_EVENT, publisher)
                .expect("0x12 arms the cash charge from boot");
            let steps = handle.rebuild_cash_bill_charge(selected, "2", new_handle);
            let mounted = steps
                .last()
                .and_then(|step| match step {
                    BillingWebEvent::Mounted { url, .. } => Some(url.as_str()),
                    BillingWebEvent::ClosedPrevious(_) => None,
                })
                .expect("a live charge arm mounts one page");
            assert_eq!(
                mounted,
                format!(
                    "http://{}/{PINNED_CASH_PATH}?USER_INDEX_ID=2",
                    PINNED_CASH_HOSTS[index]
                ),
                "booted publisher {publisher:?} must mount its pinned URL"
            );
            assert_eq!(
                steps.first(),
                Some(&BillingWebEvent::ClosedPrevious(new_handle - 1))
            );
            paint(&mut session);
        }
        assert_eq!(
            observed(&gateway.reports).len(),
            wire_before,
            "five charge arms send no new game wire"
        );

        // Render-over-time: the booted flow can mount the staged native
        // cash sheet; no web client is created to produce those pixels.
        let before_sheet = paint_frame(&mut session);
        assert!(
            session.game_mut().visit_side(SideScreen::Payment),
            "the cash-charge sheet opens from the booted lobby"
        );
        let with_sheet = paint_frame(&mut session);
        assert_ne!(
            before_sheet, with_sheet,
            "the native cash sheet changes the painted frame"
        );

        finish(session, gateway.server);
    }
}
