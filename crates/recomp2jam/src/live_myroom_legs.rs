//! Tier 3 live legs for the avatar-room family (#423).
//!
//! Boot/gateway/paint helpers come from `crate::live_leg_support` (#926);
//! every leg skips cleanly when `install-root` archives are absent (CI
//! has no game data, #0a3ba). The master-role leg pins our boot path,
//! never exe master parity: `E-M` stays a rig OPEN item (#423).

#[cfg(test)]
mod live_myroom_leg_tests {
    use std::sync::mpsc::{Receiver, RecvTimeoutError};
    use std::time::Duration;

    use crate::live_leg_support::*;
    use crate::session::OnlineSession;

    use o2jam_net::shop::EnterShop;
    use o2jam_net::CharacterInfo;
    use o2jam_net::{EmptyMessage, Frame};
    use o2jam_render::{Rect, NATIVE_WIDTH};
    use o2jam_shell::screens::side::{
        myroom_identity_captions, MyRoomPanel, SideScreen, MYROOM_BAG_FIRST_CODE,
        MYROOM_CASH_POINT_CODE, MYROOM_EXP_CODE, MYROOM_LEVEL_CODE, MYROOM_NICKNAME_CODE,
        MYROOM_PENALTY_COUNT_CODE, MYROOM_PENALTY_LEVEL_CODE, MYROOM_SLOT_FIRST_CODE,
        MYROOM_STATUS_BADGE_CODE, MYROOM_WINRATE_CODE,
    };
    use o2jam_shell::{GameLoop, InputSnapshot, Screen};

    /// The baked STATUS badge and the plate it swaps in, from
    /// `docs/myroom.md` (#369); the leg never invents geometry.
    const STATUS_BADGE_RECT: Rect = Rect {
        x: 147,
        y: 307,
        w: 60,
        h: 23,
    };
    const STATUS_PLATE_RECT: Rect = Rect {
        x: 392,
        y: 22,
        w: 384,
        h: 357,
    };

    /// One gateway frame with a short budget; a missing entry send must
    /// fail the leg, not hang a loaded test fleet.
    fn wait_for_entry_opcode(reports: &Receiver<Frame>, opcode: u16, name: &str) {
        loop {
            match reports.recv_timeout(Duration::from_secs(3)) {
                Ok(frame) if frame.id == opcode => return,
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => panic!("{name} did not arrive within 3s"),
                Err(RecvTimeoutError::Disconnected) => panic!("gateway closed before {name}"),
            }
        }
    }

    /// A staged side control's native centre; geometry comes from the
    /// mounted `.bnd` placement, not a test literal.
    fn click_staged_side_code(session: &mut OnlineSession, code: u32) {
        let rect = session
            .game_mut()
            .staged_side_control_rect(code)
            .unwrap_or_else(|| panic!("no staged MyRoom control for {code:#x}"));
        let (x, y) = rect_centre(rect);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
    }

    /// Changed pixels inside one staged rectangle: proves the face
    /// reached the frame buffer, not just the selection state.
    fn changed_pixels_in_rect(before: &[u32], after: &[u32], rect: Rect) -> usize {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let left = usize::try_from(rect.x).expect("rect left fits usize");
        let top = usize::try_from(rect.y).expect("rect top fits usize");
        let columns = usize::try_from(rect.w).expect("rect width fits usize");
        let rows = usize::try_from(rect.h).expect("rect height fits usize");
        (top..top + rows)
            .flat_map(|y| (left..left + columns).map(move |x| y * width + x))
            .filter(|index| before[*index] != after[*index])
            .count()
    }

    /// Boot through the real gateway as the room master: join Waiting
    /// on a member-0 3003 answer, leave through the staged BACK plate,
    /// then enter MyRoom from the staged Room tab.
    fn boot_master_myroom() -> (OnlineSession, LiveLegGateway) {
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        assert!(
            session.game_mut().is_room_master(),
            "the member-0 3003 answer stages the local master seat"
        );
        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
        assert_eq!(session.screen(), Screen::Room, "BACK leaves Waiting");
        paint(&mut session);
        click_staged(&mut session, Screen::Room, "IDC_BUTTON_SROOM_MYROOM");
        paint(&mut session);
        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::MyRoom),
            "the staged Room tab mounts MyRoom"
        );
        wait_for_entry_opcode(
            &gateway.reports,
            EnterShop::COMMAND,
            "C2S 5006 MyRoom entry",
        );
        (session, gateway)
    }

    /// #423 Tier 3. RIG OPEN: no qualified master-session exe MyRoom
    /// frame exists (`E-M`, `docs/captures/journey-audit/180-myroom-
    /// story-matrix.md`). This leg pins OUR master-role boot path; it
    /// does not claim exe master parity and cannot close the ticket.
    /// Live caller chain: main.rs::run_play -> connect::connect_and_login_with_transport
    /// -> boot_live_leg_with_prestaged_art -> join_waiting_by_click (member 0)
    /// -> staged Waiting BACK -> staged Room MYROOM tab -> visit_side(MyRoom)
    /// -> GameLoop::ingest_snapshot -> ScreenGame::click_myroom_side
    ///   (badge, bag cell, slot selector) -> MyRoomState -> paint_side.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_master_role_myroom_toggles_faces_and_keeps_selection_local() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let (mut session, gateway) = boot_master_myroom();

        assert_eq!(
            session.game_mut().myroom().panel(),
            MyRoomPanel::Inventory,
            "master entry opens the exe's Inventory face"
        );
        let entry = paint_frame(&mut session);
        let badge = session
            .game_mut()
            .staged_side_control_rect(MYROOM_STATUS_BADGE_CODE)
            .expect("STATE_MYROOM stages the INVENTORY badge");
        assert_eq!(badge, STATUS_BADGE_RECT, "bnd rect matches the pin");
        let plate = session
            .game_mut()
            .staged_side_control_rect(0x1550_0036)
            .expect("STATE_MYROOM stages USERINFO");
        assert_eq!(plate, STATUS_PLATE_RECT, "bnd rect matches the pin");

        click_staged_side_code(&mut session, MYROOM_STATUS_BADGE_CODE);
        assert_eq!(
            session.game_mut().myroom().panel(),
            MyRoomPanel::Status,
            "the badge press flips the face"
        );
        let toggled = paint_frame(&mut session);
        assert!(
            changed_pixels_in_rect(&entry, &toggled, STATUS_PLATE_RECT) > 0,
            "the status plate must ink on the master role"
        );
        assert!(
            changed_pixels_in_rect(&entry, &toggled, STATUS_BADGE_RECT) > 0,
            "the INVENTORY body must replace the baked badge"
        );

        click_staged_side_code(&mut session, MYROOM_STATUS_BADGE_CODE);
        assert_eq!(
            session.game_mut().myroom().panel(),
            MyRoomPanel::Inventory,
            "the second press restores the Inventory face"
        );
        let restored = paint_frame(&mut session);
        assert_eq!(
            changed_pixels_in_rect(&entry, &restored, STATUS_PLATE_RECT),
            0,
            "the round trip restores the first frame"
        );

        click_staged_side_code(&mut session, MYROOM_BAG_FIRST_CODE);
        let picked = session.game_mut().myroom();
        assert_eq!(picked.bag_cell(), Some(0), "bag pick records a cell");
        assert_eq!(picked.bag_page(), 0, "bag pick holds the page");

        click_staged_side_code(&mut session, MYROOM_SLOT_FIRST_CODE + 1);
        let after_slot = session.game_mut().myroom();
        assert_eq!(after_slot.selected_slot(), 1, "hair selector records 1");
        assert_eq!(after_slot.bag_page(), picked.bag_page());
        assert_eq!(after_slot.bag_cell(), picked.bag_cell());

        assert!(
            observed(&gateway.reports).is_empty(),
            "faces and selections are local on the master path"
        );
        finish(session, gateway.server);
    }

    /// #1098 Tier 3. The constructor's cleared words must reach the
    /// STATUS plate after a real boot, not just the pure default guard.
    /// Live chain: main.rs::run_play -> connect::connect_and_login_with_transport
    /// -> boot_live_leg_with_prestaged_art -> join_waiting_by_click ->
    /// staged Waiting BACK -> staged Room MYROOM tab -> visit_side(MyRoom)
    /// -> GameLoop::ingest_snapshot -> ScreenGame::click_myroom_side ->
    /// set_myroom_character(CharacterInfo::default()) -> paint_side_scene ->
    /// myroom_identity_captions. The wire writer of the constructor's
    /// 58-word block stays OPEN; this leg stages its cleared defaults.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_myroom_boot_keeps_cleared_player_info_defaults_on_the_status_plate() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let (mut session, gateway) = boot_master_myroom();
        session
            .game_mut()
            .set_myroom_character(CharacterInfo::default());
        let entry = paint_frame(&mut session);
        click_staged_side_code(&mut session, MYROOM_STATUS_BADGE_CODE);
        assert_eq!(
            session.game_mut().myroom().panel(),
            MyRoomPanel::Status,
            "the badge press opens the plate"
        );
        let status = paint_frame(&mut session);
        let hero = session
            .game_mut()
            .myroom_character()
            .expect("defaults staged on the boot path");
        assert_eq!(hero.gem, 0);
        assert_eq!(hero.battles, 0);
        assert_eq!(hero.wins, 0);
        assert_eq!(hero.losses, 0);
        assert!(hero.equipment.iter().all(|word| *word == 0));
        let captions = myroom_identity_captions(hero);
        let caption = |code| {
            captions
                .iter()
                .find(|(staged, _)| *staged == code)
                .map(|(_, text)| text.as_str())
        };
        assert_eq!(caption(MYROOM_NICKNAME_CODE), Some(""));
        assert_eq!(caption(MYROOM_LEVEL_CODE), Some(" 0"));
        assert_eq!(caption(MYROOM_CASH_POINT_CODE), Some("0 PT"));
        assert_eq!(caption(MYROOM_EXP_CODE), Some(" 0"));
        assert_eq!(caption(MYROOM_WINRATE_CODE), Some("0%"));
        assert_eq!(caption(MYROOM_PENALTY_COUNT_CODE), Some("0"));
        assert_eq!(caption(MYROOM_PENALTY_LEVEL_CODE), Some("Lv 0"));
        assert!(
            changed_pixels_in_rect(&entry, &status, STATUS_PLATE_RECT) > 0,
            "the STATUS plate inked on the boot path"
        );
        finish(session, gateway.server);
    }
}
