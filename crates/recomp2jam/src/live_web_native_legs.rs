//! Tier 3 live legs and the build guard for the native web surfaces
//! (spec #750). Boot/gateway/paint helpers come from
//! `crate::live_leg_support` (#926); every leg skips cleanly when the
//! installed archives are absent (CI has no game data, #0a3ba).

#[cfg(test)]
mod live_web_native_leg_tests {
    use crate::live_leg_support::*;
    use crate::session::OnlineSession;

    use o2jam_net::frame::encode_frame;
    use o2jam_net::testing::{fake_gateway, read_frame};
    use o2jam_net::{JoinRoomResponse, JoinedRoom, Request, WireMessage};
    use o2jam_shell::screens::messenger::{WebSheet, WebSheetHost, EXE_SCENE_ROOM};
    use o2jam_shell::screens::side::{
        music_shop_action_at, MusicShopAction, SideScreen, MSHOP_CART_BUY_CODE,
        MSHOP_EX_FIRST_CODE, MSHOP_GENRE_FILTER_FIRST_CODE, ROOM_MUSICSHOP_BUTTON_CODE,
    };
    use o2jam_shell::{GameLoop, InputSnapshot, Screen};
    use std::io::Write;
    use std::sync::mpsc;

    /// One synthetic one-song catalog in the 7.55 `OJNList.dat` shape.
    fn purchase_catalog_blob() -> Vec<u8> {
        let mut record = vec![0u8; 300];
        record[4..8].copy_from_slice(&757u32.to_le_bytes());
        record[8..11].copy_from_slice(b"ojn");
        record[20..24].copy_from_slice(&124f32.to_le_bytes());
        record[112..117].copy_from_slice(b"First");
        record[176..180].copy_from_slice(b"Band");
        record[240..251].copy_from_slice(b"o2ma757.ojm");
        record
    }

    /// The scripted 5011 verdict: a pinned success-family song or the
    /// scripted four-byte refusal from #405. The real failure body
    /// stays OPEN as `E-W-5011`.
    fn purchase_answer(failure: bool) -> Vec<u8> {
        if failure {
            return encode_frame(5011, &[0x01, 0x00, 0x00, 0x00]);
        }
        use o2jam_net::shop::{MusicEntry, SyncMusicPurchaseResponse};
        SyncMusicPurchaseResponse {
            gem: 1000,
            point: 250,
            o2_cash: 5000,
            music_cash: 120,
            item_cash: 340,
            cash_point: 56,
            music_list: vec![MusicEntry {
                music_id: 757,
                day: 14,
                month: 3,
                year: 25,
                hour: 18,
                minute: 45,
            }],
            gift_boxes_raw: Vec::new(),
            free_pass_extension_days: 7,
        }
        .encode()
    }

    /// Boot gateway that answers the pinned 5015 with 5016 and the
    /// synthesized 5010 with one scripted 5011 verdict. Everything is
    /// recorded, so the leg sees the real wire order.
    fn spawn_purchase_gateway(joined: JoinedRoom, failure: bool) -> LiveLegGateway {
        let (reported, reports) = mpsc::channel();
        let (join_written_tx, join_answer_written) = mpsc::channel();
        let (port, server) = fake_gateway(move |mut stream| {
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            stream
                .write_all(
                    &o2jam_net::AuthResponse {
                        result_code: 0,
                        tail: Vec::new(),
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            stream
                .write_all(
                    &o2jam_net::ChannelList {
                        channels: vec![channel_row()],
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            stream
                .write_all(
                    &o2jam_net::ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            stream
                .write_all(
                    &o2jam_net::RoomList {
                        rooms: vec![room_row()],
                    }
                    .encode(),
                )
                .unwrap();
            while let Some(frame) = read_frame_until_eof(&mut stream) {
                if frame.id == Request::JoinWaiting.id() {
                    stream
                        .write_all(
                            &JoinRoomResponse {
                                result_code: 0,
                                room: Some(joined.clone()),
                            }
                            .encode(),
                        )
                        .unwrap();
                    join_written_tx.send(()).unwrap();
                }
                if frame.id == 5015 {
                    stream.write_all(&encode_frame(5016, &[])).unwrap();
                }
                if frame.id == 5010 {
                    stream.write_all(&purchase_answer(failure)).unwrap();
                }
                reported.send(frame).unwrap();
            }
        });
        LiveLegGateway {
            port,
            reports,
            join_answer_written,
            server,
        }
    }

    /// One point in a staged Music Shop control that the shell's own
    /// hit-test resolves to the expected action. Bound rects overlap,
    /// so a control centre is not enough (#886 live-leg precedent).
    fn staged_action_point(
        session: &mut OnlineSession,
        code: u32,
        expected: impl Fn(MusicShopAction) -> bool,
    ) -> (i32, i32) {
        let placements = session.game_mut().staged_side_placements().to_vec();
        session
            .game_mut()
            .staged_side_control_rects(code)
            .into_iter()
            .flat_map(|rect| {
                let width = i32::try_from(rect.w).unwrap_or(0);
                let height = i32::try_from(rect.h).unwrap_or(0);
                (rect.y..rect.y + height)
                    .flat_map(move |y| (rect.x..rect.x + width).map(move |x| (x, y)))
            })
            .find(|(x, y)| music_shop_action_at(&placements, *x, *y).is_some_and(&expected))
            .unwrap_or_else(|| panic!("no staged point for {code:#x}"))
    }

    /// Boot -> Room -> Music Shop -> one cart buy. The row buy's price
    /// rect is tiled differently in this `.bnd`, so the bag entry is
    /// staged at the same seam the #886 live leg uses; every wire
    /// frame and verdict below rides the real gateway path.
    fn boot_to_cart_buy(gateway: &LiveLegGateway) -> OnlineSession {
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let (x, y) = staged_code_centre(&mut session, Screen::Room, ROOM_MUSICSHOP_BUTTON_CODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);
        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::MusicShop),
            "the staged Room tab must mount the Music Shop"
        );

        let blob = purchase_catalog_blob();
        let catalog = o2jam_file::parse_ojn_list(&blob).unwrap();
        session.game_mut().set_song_catalog(&catalog);
        session
            .game_mut()
            .stage_music_shop_prices(&[o2jam_file::OjnMidParamEntry {
                song_id: 757,
                param_a: 0,
                param_b: 40,
                anchor: o2jam_file::OJN_MID_TABLE3_ANCHOR,
            }]);
        session
            .game_mut()
            .set_myroom_character(o2jam_net::CharacterInfo {
                gem: 500,
                o2_cash: 500,
                music_cash: 500,
                item_cash: 500,
                ..o2jam_net::CharacterInfo::default()
            });

        let (x, y) = staged_action_point(&mut session, MSHOP_GENRE_FILTER_FIRST_CODE + 4, |a| {
            matches!(a, MusicShopAction::Genre(_))
        });
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);
        let (x, y) = staged_action_point(&mut session, MSHOP_EX_FIRST_CODE, |a| {
            matches!(a, MusicShopAction::SelectRow(_))
        });
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);
        session.game_mut().music_shop_bag_push(757, true);
        let (x, y) = staged_action_point(&mut session, MSHOP_CART_BUY_CODE, |a| {
            a == MusicShopAction::CartBuy
        });
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        session
    }

    /// Paint until the scripted verdict reaches the shell. Feedback is
    /// sampled after every frame so the 5016/5011 order is observable.
    fn settle_purchase(session: &mut OnlineSession, failure: bool) -> Vec<(bool, bool)> {
        let mut feedback = Vec::new();
        for _ in 0..12 {
            paint(session);
            feedback.push((
                session.game_mut().start_payment_answer_notice(),
                session.game_mut().billing_failure_notice_open(),
            ));
            let verdict_landed = if failure {
                session.game_mut().billing_failure_notice_open()
            } else {
                !session.game_mut().music_download().requests().is_empty()
            };
            if verdict_landed {
                break;
            }
        }
        feedback
    }

    /// Package names that embed or drive a browser engine. The 7.55
    /// pages are gone; none may enter the Rust client's dependency
    /// graph (#1137). Exact package names only, never substrings.
    fn browser_engine_names() -> [&'static str; 13] {
        [
            "webkit",
            "webkit2gtk",
            "webkit2gtk-sys",
            "webview",
            "webview2",
            "webview2-com",
            "wry",
            "tao",
            "cef",
            "cef-sys",
            "electron",
            "gecko",
            "mozjs",
        ]
    }

    /// First locked package that would bring in a browser engine.
    fn locked_browser_engine(lock: &str) -> Option<&'static str> {
        let names = browser_engine_names();
        lock.lines()
            .filter_map(|line| line.strip_prefix("name = "))
            .filter_map(|line| line.trim_matches('"').split_whitespace().next())
            .find_map(|package| names.into_iter().find(|engine| *engine == package))
    }

    /// #1137 Tier 3 build guard. seam: no browser engine can be
    /// reached from a headless leg; the dependency graph and the
    /// already-merged native host (#948/#1114) are the whole property.
    /// A `WebSheetHost` keeps only handle and visibility words: no
    /// page bytes, URL or renderer exists in the client.
    #[test]
    fn no_browser_engine_dependency_or_rendered_page_in_the_client() {
        let lock_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock");
        let lock =
            std::fs::read_to_string(lock_path).expect("the workspace lock file is committed");
        assert_eq!(
            locked_browser_engine(&lock),
            None,
            "a browser engine must stay out of the client dependency graph"
        );

        let mut host = WebSheetHost::new();
        let transition = host.open(WebSheet::CashBill, EXE_SCENE_ROOM, 11);
        assert!(transition.visible && transition.focused);
        assert_eq!(host.active_sheet(), Some(WebSheet::CashBill));
    }

    /// #1128/#1130/#1136 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> boot_live_leg_with_art ->
    ///   GameLoop::ingest_snapshot (splash, room, Music Shop tab, row,
    ///   BUYITEM) -> ScreenGame::run_music_shop_checkout ->
    ///   OnlineSession::flush_start_payment -> C2S 5015 -> S2C 5016 ->
    ///   OnlineSession::flush_music_buy -> C2S 5010 -> S2C 5011 ->
    ///   ScreenGame::apply_music_purchase_response -> owned row ink.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn tier_3_native_music_purchase_from_boot_folds_the_5011_success_body() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_purchase_gateway(joined_room(0, true), false);
        let mut session = boot_to_cart_buy(&gateway);
        let row_rect = session
            .game_mut()
            .staged_side_control_rect(MSHOP_EX_FIRST_CODE)
            .expect("the staged row rect owns the owned badge");
        let before = staged_rect_pixels(&mut session, row_rect);
        let feedback = settle_purchase(&mut session, false);
        let after = staged_rect_pixels(&mut session, row_rect);
        let sent: Vec<u16> = observed(&gateway.reports)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| *id == 5015 || *id == 5010)
            .collect();
        let queued: Vec<u16> = session
            .game_mut()
            .music_download()
            .requests()
            .iter()
            .map(|request| request.song_id)
            .collect();
        finish(session, gateway.server);

        assert_eq!(
            sent,
            vec![5015, 5010],
            "boot must send 5015 then 5010: {sent:?}"
        );
        assert_eq!(
            queued,
            vec![757],
            "the bought song must join the transfer queue"
        );
        assert!(
            feedback.iter().any(|(start, _)| *start),
            "the pinned 5016 notice must appear before the verdict"
        );
        assert_ne!(before, after, "the owned badge must re-ink the staged row");
    }

    /// #1129/#1139/#1140 Tier 3. Same live chain, refusal arm:
    ///   ... -> drain_music_buy -> MusicPurchaseReply::Failure ->
    ///   ScreenGame::post_music_purchase_failure_notice ->
    ///   paint_side -> paint_billing_failure_notice.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn tier_3_native_music_purchase_from_boot_inks_the_5011_refusal_notice() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_purchase_gateway(joined_room(0, true), true);
        let mut session = boot_to_cart_buy(&gateway);
        let feedback = settle_purchase(&mut session, true);
        let frame = paint_frame(&mut session);
        let band = o2jam_shell::screens::paint::BILLING_FAILURE_NOTICE_RECT;
        let pixel = band.y as usize * 800 + band.x as usize;
        let painted = frame[pixel];
        let sent: Vec<u16> = observed(&gateway.reports)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| *id == 5015 || *id == 5010)
            .collect();
        let cart = session.game_mut().music_shop_cart().to_vec();
        finish(session, gateway.server);

        assert_eq!(
            sent,
            vec![5015, 5010],
            "boot must send 5015 then 5010: {sent:?}"
        );
        assert_eq!(painted, o2jam_render::pack_xrgb(0x67, 0xBA, 0xF0));
        assert_eq!(cart, vec![757], "the refused buy keeps the cart");
        let start_at = feedback
            .iter()
            .position(|(start, _)| *start)
            .expect("5016 notice");
        let verdict_at = feedback
            .iter()
            .position(|(_, fail)| *fail)
            .expect("5011 notice");
        assert!(
            start_at < verdict_at,
            "5016 must ink before 5011: {feedback:?}"
        );
    }
}
