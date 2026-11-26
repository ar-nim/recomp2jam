//! Tier 3 live-input legs for the Shop button-control stories
//! (#1093–#1097). Boot/gateway/paint helpers come from
//! `crate::live_leg_support`; every leg skips cleanly when the installed
//! archives are absent (CI has no game data, #0a3ba).
//!
//! The cart and Music Shop rank/pager arms already have live legs in
//! `live_input_legs.rs`; these legs cover the two open arms: the gift-box
//! pager and the Waiting select-music rows.

#[cfg(test)]
mod live_shop_control_tests {
    use std::sync::Arc;

    use crate::live_leg_support::*;

    use o2jam_net::shop::EnterShop;
    use o2jam_net::{CharacterInfo, EmptyMessage};
    use o2jam_render::{Rect, NATIVE_WIDTH};
    use o2jam_shell::screens::game::ScreenGame;
    use o2jam_shell::screens::side::{
        select_music_rows, SelectLevelBand, MYROOM_GIFT_FIRST_CODE, MYROOM_GIFT_LEFT_CODE,
        MYROOM_GIFT_RIGHT_CODE,
    };
    use o2jam_shell::{read_installed_avatar, AvatarSource, GameLoop, InputSnapshot, Screen};

    use crate::{install_root, session::OnlineSession};

    /// The PRESENT BOX tab code from `State_Myroom.bnd` (census row 69).
    const PRESENT_BOX_TAB_CODE: u32 = 0x1560_0046;
    /// The installed song index, staged by the same live boot as the
    /// `DAILOG_SELECTMUSIC` art (`shots.rs` uses this archive path).
    const SONG_INDEX_PATH: &str = "O2Jam/Image/OJNList.dat";

    /// Click one staged side control by its dispatcher code; the point
    /// comes from the mounted `.bnd` rect, never from a test literal.
    fn click_staged_side_code(session: &mut OnlineSession, code: u32) {
        let rect = session
            .game_mut()
            .staged_side_control_rect(code)
            .unwrap_or_else(|| panic!("no staged side control for {code:#x}"));
        let (x, y) = rect_centre(rect);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
    }

    /// Count pixels a face change rewrote inside one staged rect.
    fn changed_pixels_in_rect(before: &[u32], after: &[u32], rect: Rect) -> usize {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let top = usize::try_from(rect.y).expect("rect top fits usize");
        let left = usize::try_from(rect.x).expect("rect left fits usize");
        let rows = usize::try_from(rect.h).expect("rect height fits usize");
        let columns = usize::try_from(rect.w).expect("rect width fits usize");
        (top..top + rows)
            .flat_map(|y| (left..left + columns).map(move |x| y * width + x))
            .filter(|index| before[*index] != after[*index])
            .count()
    }

    /// Installed avatar source and the first six item ids from its
    /// table, so six staged gifts make two pages. No item is invented:
    /// the table owns every id.
    fn installed_avatar_with_gift_ids() -> (Arc<AvatarSource>, Vec<i32>) {
        let source = read_installed_avatar(&install_root()).expect("installed avatar table");
        let ids: Vec<i32> = source
            .table()
            .rows
            .iter()
            .take(6)
            .map(|row| i32::try_from(row.item_id).expect("item id fits a wire i32"))
            .collect();
        assert_eq!(ids.len(), 6, "the installed table has six gift items");
        (source, ids)
    }

    /// Stage the installed song index and its label buckets. The
    /// dialog's live filter counts come from these rows, never from a
    /// test-side catalog.
    fn stage_installed_song_index(game: &mut ScreenGame) {
        let bytes = std::fs::read(install_root().join(SONG_INDEX_PATH))
            .expect("installed OJNList.dat behind the archive guard");
        let entries = o2jam_file::parse_ojn_list(&bytes).expect("installed song index parses");
        let labels = o2jam_file::parse_ojn_list_labels(&bytes).unwrap_or_default();
        game.set_song_catalog_with_labels(&entries, &labels);
        assert!(!game.select_songs().is_empty(), "the index stages rows");
    }

    /// #1094 Tier 3: the gift-list pager is reachable from a real boot,
    /// wraps LEFT, clamps RIGHT, and drops the picked row. The static
    /// `CCBtnGiftList` constructor guards cover class identity; this leg
    /// pins the player-visible pager arm named by the pin.
    ///
    /// Live caller chain: `main.rs::run_play` ->
    /// `connect::connect_and_login_with_transport` ->
    /// `boot_live_leg_with_prestaged_art` -> `enter_room_by_clicks` ->
    /// staged Room MYROOM tab -> `visit_side(MyRoom)` ->
    /// `OnlineSession::ingest_snapshot` -> `ScreenGame::click_myroom_side`
    /// (PRESENT BOX, GIFTBOX_EX1, GIFT_LEFT/RIGHT) ->
    /// `MyRoomState::turn_gift_page` -> `paint_side`.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_gift_pager_wraps_clamps_and_clears_the_picked_row_from_boot() {
        if !archives_present() {
            println!("SKIP live_gift_pager_wraps_clamps_and_clears_the_picked_row_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        let (avatar, gift_ids) = installed_avatar_with_gift_ids();
        let gifts = gift_ids
            .into_iter()
            .enumerate()
            .map(|(index, item_id)| o2jam_net::GiftItemInfo {
                gift_id: i32::try_from(index).expect("six gifts fit an i32"),
                item_id,
                sender: "Giver".to_owned(),
            })
            .collect();
        session.game_mut().set_myroom_character(CharacterInfo {
            item_gifts: gifts,
            ..CharacterInfo::default()
        });
        session.game_mut().set_decoded_avatar_source(&avatar);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        click_staged(&mut session, Screen::Room, "IDC_BUTTON_SROOM_MYROOM");
        paint(&mut session);
        assert_eq!(
            session.game_mut().side(),
            Some(o2jam_shell::screens::side::SideScreen::MyRoom),
            "the staged tab mounts MyRoom"
        );
        wait_for_opcode(&gateway.reports, EnterShop::COMMAND);
        while gateway.reports.try_recv().is_ok() {}

        let gift_cell = session
            .game_mut()
            .staged_side_control_rect(MYROOM_GIFT_FIRST_CODE)
            .expect("STATE_MYROOM stages GIFTBOX_EX1");
        click_staged_side_code(&mut session, PRESENT_BOX_TAB_CODE);
        assert_eq!(
            session.game_mut().myroom().gift_page(),
            0,
            "the present box opens on its first page"
        );
        click_staged_side_code(&mut session, MYROOM_GIFT_FIRST_CODE);
        assert_eq!(
            session.game_mut().myroom().gift_cell(),
            Some(0),
            "the gift row records its pick"
        );
        // A picked gift row sends the pinned 5023 claim. Drain that
        // frame before checking the two pager presses, which must be
        // local (census rows 71/72).
        paint(&mut session);
        wait_for_opcode(&gateway.reports, 5023);
        while gateway.reports.try_recv().is_ok() {}
        let before = paint_frame(&mut session);

        click_staged_side_code(&mut session, MYROOM_GIFT_LEFT_CODE);
        assert_eq!(
            session.game_mut().myroom().gift_page(),
            1,
            "LEFT wraps from the first page to the last"
        );
        assert_eq!(
            session.game_mut().myroom().gift_cell(),
            None,
            "turning clears the picked row"
        );
        let wrapped = paint_frame(&mut session);
        assert!(
            changed_pixels_in_rect(&before, &wrapped, gift_cell) > 0,
            "the wrapped page must re-ink the staged gift cell"
        );

        click_staged_side_code(&mut session, MYROOM_GIFT_RIGHT_CODE);
        assert_eq!(
            session.game_mut().myroom().gift_page(),
            1,
            "RIGHT clamps at the last page"
        );
        let wire = observed(&gateway.reports);
        assert!(
            wire.is_empty(),
            "both pager presses stay wire quiet: {wire:?}"
        );
        finish(session, gateway.server);
    }

    /// #1096 Tier 3: Waiting's select-music button opens the real
    /// `DAILOG_SELECTMUSIC` and a staged filter row narrows the installed
    /// catalog. The class pin stays tied to this dialog by the lifecycle
    /// doc; the triage's Music Shop attribution stays an OPEN conflict.
    ///
    /// Live caller chain: `main.rs::run_play` ->
    /// `connect::connect_and_login_with_transport` ->
    /// `boot_live_leg_with_prestaged_art` -> `join_waiting_by_click` ->
    /// `OnlineSession::ingest_snapshot` ->
    /// `ScreenGame::click_waiting_button` (MUSELECT) ->
    /// `load_select_music_scene` -> `click_select_filter` ->
    /// `select_music_rows`.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_select_music_row_filter_narrows_the_dialog_from_boot() {
        if !archives_present() {
            println!("SKIP live_select_music_row_filter_narrows_the_dialog_from_boot: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        assert!(
            session.game_mut().is_room_master(),
            "the member-0 answer stages the master who owns MUSELECT"
        );
        stage_installed_song_index(session.game_mut());
        while gateway.reports.try_recv().is_ok() {}

        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_MUSELECT");
        assert!(
            session.game_mut().select_music_dialog_open(),
            "the staged master button mounts the song dialog"
        );
        let unfiltered_count = {
            let game = session.game_mut();
            select_music_rows(game.select_songs(), game.select_filter()).len()
        };
        assert!(unfiltered_count > 0, "the installed index stages rows");

        let easy = session
            .game_mut()
            .select_music_placements()
            .and_then(|placements| {
                placements
                    .iter()
                    .find(|placed| placed.id == "IDC_BUTTON_DSMUSIC_STEP1_EASY")
                    .map(|placed| placed.rect)
            })
            .expect("the staged dialog names its STEP1 EASY button");
        let (x, y) = rect_centre(easy);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(
            session.game_mut().select_filter().level,
            SelectLevelBand::Easy,
            "the staged filter button stores its level band"
        );
        let (filtered_count, all_in_band) = {
            let game = session.game_mut();
            let rows = select_music_rows(game.select_songs(), game.select_filter());
            (
                rows.len(),
                rows.iter()
                    .all(|row| SelectLevelBand::Easy.keeps(row.song.chart_level(row.slot))),
            )
        };
        assert!(
            filtered_count < unfiltered_count,
            "the live filter drops rows ({unfiltered_count} -> {filtered_count})"
        );
        assert!(all_in_band, "every live row sits in the clicked band");
        assert!(
            observed(&gateway.reports).is_empty(),
            "the dialog and filter send no wire frame"
        );
        finish(session, gateway.server);
    }
}
