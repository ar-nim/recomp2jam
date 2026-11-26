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
        BILLING_PAYMENT2_STATE_WORD, BILLING_PAYMENT_STATE_WORD,
    };
    use o2jam_shell::Screen;

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
