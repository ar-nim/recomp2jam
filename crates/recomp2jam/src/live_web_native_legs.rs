//! Tier 3 live legs and the build guard for the native web surfaces
//! (spec #750). Boot/gateway/paint helpers come from
//! `crate::live_leg_support` (#926); every leg skips cleanly when the
//! installed archives are absent (CI has no game data, #0a3ba).

#[cfg(test)]
mod live_web_native_leg_tests {
    use crate::live_leg_support::*;
    use crate::session::OnlineSession;

    use std::collections::HashMap;

    use o2jam_audio::NullBackend;
    use o2jam_net::frame::encode_frame;
    use o2jam_net::rooms::{MusicPlayRankingAnswer, RankDelta, RankEntry, SendMainRoomMessage};
    use o2jam_net::testing::{fake_gateway, read_frame};
    use o2jam_net::{JoinRoomResponse, JoinedRoom, Request, WireMessage};
    use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::screens::game::LobbyChat;
    use o2jam_shell::screens::messenger::{
        result_event_page, NativeEventPage, WebSheet, WebSheetHost, EXE_SCENE_ROOM,
        NATIVE_EVENT_NOTICE_RECT, NATIVE_EVENT_NOTICE_TONE, PLAYING_QA_UI_EVENT,
        WM_APP_HIDE_WEB_SHEETS,
    };
    use o2jam_shell::screens::side::{
        music_shop_action_at, music_shop_top100_rects, MusicShopAction, MusicShopFace, SideScreen,
        MSHOP_CART_BUY_CODE, MSHOP_EX_FIRST_CODE, MSHOP_GENRE_FILTER_FIRST_CODE,
        MSHOP_TAB_DELETE_CODE, ROOM_MUSICSHOP_BUTTON_CODE,
    };
    use o2jam_shell::{GameLoop, InputSnapshot, MenuKey, Screen};
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

    /// Boot-only gateway for the selector arm. The shared boot helper
    /// records frames only after `GetChannelInfo`, so this one signals
    /// that answer on the join channel. The Tier 2 guard then drains
    /// boot frames before staging the selector and can prove that the
    /// HTTP event page adds no gateway frame.
    fn spawn_armed_selector_gateway() -> LiveLegGateway {
        let (reported, reports) = mpsc::channel();
        let (boot_written_tx, boot_written) = mpsc::channel();
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
            boot_written_tx.send(()).unwrap();
            while let Some(frame) = read_frame_until_eof(&mut stream) {
                reported.send(frame).unwrap();
            }
        });
        LiveLegGateway {
            port,
            reports,
            join_answer_written: boot_written,
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

    /// #1137 policy guard. seam: no browser engine can be
    /// reached from a headless leg; the dependency graph and the
    /// already-merged native host (#948/#1114) are the whole property.
    /// A `WebSheetHost` keeps only handle and visibility words: no
    /// page bytes, URL or renderer exists in the client.
    /// next evidence step: #916 / #1137 — keep the native-host policy
    /// guard and re-audit the dependency graph when a web surface lands.
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

    /// #1150 Tier 3. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport ->
    ///   assemble_online_session -> enter_room_by_clicks ->
    ///   ScreenGame::stage_web_whisper_target (synthesized
    ///   SelectedUserNickName) -> set_lobby_input -> menu_key(Confirm) ->
    ///   OnlineSession::room_chat_took_enter -> flush_room_chat_action ->
    ///   ScreenGame::lobby_slash_command -> C2S 2017.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_web_nickname_report_back_sends_the_2017_chat_frame() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let edit_rect = session
            .game_mut()
            .scene_placements(Screen::Room)
            .iter()
            .find(|placed| placed.id == "IDC_EDIT_SROOM_CHAT")
            .unwrap_or_else(|| panic!("no staged Room chat edit rect"))
            .rect;
        let rest = staged_rect_pixels(&mut session, edit_rect);
        assert!(session.game_mut().stage_web_whisper_target("WebPeer"));
        session
            .game_mut()
            .set_lobby_input(LobbyChat::Room, "hello from page");
        let typed = staged_rect_pixels(&mut session, edit_rect);
        session.menu_key(MenuKey::Confirm, true);
        let body = wait_for_opcode(&gateway.reports, 2017);
        let whisper = o2jam_net::rooms::SendWhisper::decode_body(&body).unwrap();
        let target = session.game_mut().web_whisper_target().map(str::to_owned);
        finish(session, gateway.server);

        assert_eq!(whisper.recipient, "WebPeer");
        assert_eq!(whisper.content, "hello from page");
        assert_eq!(target.as_deref(), Some("WebPeer"));
        assert_ne!(
            rest, typed,
            "the staged whisper draft must ink the chat rect"
        );
    }

    /// One synthetic catalog with the id the cart report-back names.
    fn catalog_blob_for(song_id: u32) -> Vec<u8> {
        let mut blob = purchase_catalog_blob();
        blob[4..8].copy_from_slice(&song_id.to_le_bytes());
        blob
    }

    /// #1150 Tier 3. Live caller chain:
    ///   main.rs::run_play -> enter_room_by_clicks -> staged Music Shop tab ->
    ///   ScreenGame::stage_web_cart_add (synthesized mid + mflag) ->
    ///   MusicShopState::stage_bag_song -> paint_at -> paint_music_shop_cart.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_web_cart_report_back_adds_the_packed_mid_to_the_cart_frame() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_purchase_gateway(joined_room(0, true), false);
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let (x, y) = staged_code_centre(&mut session, Screen::Room, ROOM_MUSICSHOP_BUTTON_CODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);
        assert_eq!(session.game_mut().side(), Some(SideScreen::MusicShop));
        let (x, y) = staged_action_point(&mut session, MSHOP_TAB_DELETE_CODE, |action| {
            action == MusicShopAction::Face(MusicShopFace::Cart)
        });
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);
        assert_eq!(session.game_mut().music_shop_face(), MusicShopFace::Cart);

        let blob = catalog_blob_for(733);
        let catalog = o2jam_file::parse_ojn_list(&blob).unwrap();
        session.game_mut().set_song_catalog(&catalog);
        session
            .game_mut()
            .stage_music_shop_prices(&[o2jam_file::OjnMidParamEntry {
                song_id: 733,
                param_a: 0,
                param_b: 40,
                anchor: o2jam_file::OJN_MID_TABLE3_ANCHOR,
            }]);
        let before = paint_frame(&mut session);
        let payload = [0xDD, 0x02, b'G'];
        let add = session.game_mut().stage_web_cart_add(&payload);
        let after = paint_frame(&mut session);
        let cart = session.game_mut().music_shop_cart().to_vec();
        finish(session, gateway.server);

        assert_eq!(add.unwrap().payload(), payload);
        assert_eq!(cart, vec![733], "packed mid 733 reaches the native cart");
        assert_ne!(before, after, "the added cart row must re-ink the frame");
    }

    /// #1150 Tier 3. Live caller chain:
    ///   main.rs::run_play -> enter_room_by_clicks ->
    ///   ScreenGame::stage_web_document_complete (synthesized browser
    ///   navigation) -> ChatState::stage_web_document_notice ->
    ///   paint_at -> paint_room_chat -> staged chat list row.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_web_document_complete_report_back_inks_the_notice_frame() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let list_rect = session
            .game_mut()
            .scene_placements(Screen::Room)
            .iter()
            .find(|placed| placed.id == "IDC_LIST_SROOM_CHAT")
            .unwrap_or_else(|| panic!("no staged Room chat list rect"))
            .rect;
        let rest = staged_rect_pixels(&mut session, list_rect);
        let url = "http://game1.o2jam.com/billing/imsi/buttons.asp";
        let notify = session.game_mut().stage_web_document_complete(url).unwrap();
        let inked = staged_rect_pixels(&mut session, list_rect);
        let cached = session
            .game_mut()
            .last_web_document_url()
            .map(str::to_owned);
        finish(session, gateway.server);

        assert_eq!(notify.payload(), url.as_bytes());
        assert_eq!(cached.as_deref(), Some(url));
        assert_ne!(rest, inked, "the notify URL must ink the staged notice row");
    }

    /// Same boot-to-Result walk as the Result paint leg (#1073), with
    /// the null audio backend so no sound reaches a speaker. The
    /// native event surface is armed by `enter_screen(Result)` at the
    /// end of this walk.
    fn walk_to_live_result(gateway: &LiveLegGateway) -> OnlineSession {
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        session
            .game_mut()
            .with_audio(Box::new(NullBackend::new()), HashMap::new());
        session
            .game_mut()
            .set_selected_music(o2jam_net::play::SetRoomMusic {
                music_id: 757,
                difficulty: 1,
                speed: 1,
            });
        session
            .game_mut()
            .set_loading_status(o2jam_shell::screens::paint::LoadingStatus {
                progress: 1.0,
                title: "First".to_owned(),
                artist: "Band".to_owned(),
            });
        session.menu_key(MenuKey::Confirm, true);
        paint_until_screen(&mut session, Screen::Loading);
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.screen(), Screen::Playing, "the loaded gate opened");
        session.game_mut().load_chart(vec![(0, 1_000, 7)]);
        session.game_mut().strike_at(0, 1_000);
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
        session.game_mut().paint_at(
            &mut Frame {
                buf: &mut vec![0u32; width * height],
                w: width,
                h: height,
            },
            1_000_000,
        );
        assert_eq!(
            session.screen(),
            Screen::Result,
            "the walk settled in Result"
        );
        session
    }

    /// #1144 Tier 2 armed-selector gateway arm. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> boot_live_leg -> ScreenGame::stage_result_event_selector.
    /// The armed event page is an HTTP GET in the exe, so staging its
    /// selector must add no gateway frame. Result entry's selector
    /// branch is pinned by the Tier 0 game seam test and the Tier 3
    /// zero-selector wire+frame leg.
    #[test]
    fn tier_2_armed_result_selector_names_the_ticket_page_without_a_gateway_frame() {
        let gateway = spawn_armed_selector_gateway();
        let mut session = boot_live_leg(gateway.port, false);
        gateway
            .join_answer_written
            .recv()
            .expect("the gateway answered the boot reads");
        let _ = observed(&gateway.reports);

        session.game_mut().stage_result_event_selector(1);

        let selector = session.game_mut().result_event_selector();
        let later = observed(&gateway.reports);
        finish(session, gateway.server);

        assert_eq!(
            result_event_page(0, 1),
            Some(NativeEventPage::TicketAdd),
            "the armed selector names the ticket page"
        );
        assert_eq!(selector, 1, "the selector word was staged");
        assert!(
            later.is_empty(),
            "the HTTP event page must never become a gateway frame: {later:?}"
        );
    }

    /// #1144 Tier 3 Result wire+frame pair, pinned zero-selector arm:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> walk_to_live_result ->
    ///   ScreenGame::enter_screen(Result) -> result_event_page ->
    ///   ScreenGame::paint_at -> paint_native_event_surface.
    /// The ticket-add request line, the 4018 server answer and the
    /// second hot-time host are OPEN; this gateway shows the C→S
    /// tally wire, not an HTTP GET.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn tier_3_result_zero_selector_pairs_the_tally_wire_with_the_hot_time_frame() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_result_back_gateway(joined_room(0, true));
        let mut session = walk_to_live_result(&gateway);
        let frame = paint_frame(&mut session);
        let band = staged_rect_pixels_from(&frame, NATIVE_EVENT_NOTICE_RECT);
        let page = session.game_mut().native_event_page();
        let sent: Vec<u16> = observed(&gateway.reports)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| *id >= 4014)
            .collect();
        finish(session, gateway.server);

        assert_eq!(page, Some(NativeEventPage::HotTime));
        assert!(
            band.contains(&NATIVE_EVENT_NOTICE_TONE),
            "the hot-time stand-in must ink the native notice band"
        );
        for opcode in [4016, 4041] {
            assert!(
                sent.contains(&opcode),
                "the qualified Result tally wire must carry {opcode}: {sent:?}"
            );
        }
    }

    /// #1145 Tier 3 page-open arm. Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> enter_room_by_clicks ->
    ///   ScreenGame::stage_native_web_ui_event(0x14) ->
    ///   WebSheetHost::open -> NativeEventPage::InGameQa ->
    ///   ScreenGame::paint_at -> paint_native_event_surface ->
    ///   window.rs::web_sheet_visibility_arm ->
    ///   ScreenGame::hide_native_web_host.
    /// No page body, question or outbound link is claimed (#948/#1145).
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn tier_3_qa_page_open_inks_the_labeled_help_stand_in_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let rest = staged_rect_pixels(&mut session, NATIVE_EVENT_NOTICE_RECT);
        let opened = session
            .game_mut()
            .stage_native_web_ui_event(PLAYING_QA_UI_EVENT, EXE_SCENE_ROOM)
            .expect("0x14 names the Q&A page");
        let inked = staged_rect_pixels(&mut session, NATIVE_EVENT_NOTICE_RECT);
        let page = session.game_mut().native_event_page();
        let hidden = session
            .game_mut()
            .hide_native_web_host(WM_APP_HIDE_WEB_SHEETS);
        let hidden_pixels = staged_rect_pixels(&mut session, NATIVE_EVENT_NOTICE_RECT);
        let later = observed(&gateway.reports);
        finish(session, gateway.server);

        assert!(
            opened.visible && opened.focused,
            "the page-open shows/focuses"
        );
        assert_eq!(opened.qa_rect_shift, 50, "scene 2 shifts the Q&A rect");
        assert_eq!(page, Some(NativeEventPage::InGameQa));
        assert_ne!(inked, rest, "the native help stand-in must ink pixels");
        assert!(hidden, "the app hide message hides both handles");
        assert_eq!(hidden_pixels, rest, "the hidden stand-in leaves no ink");
        assert!(later.is_empty(), "a page-open must send no gateway frame");
    }
    fn downloaded_chart_dirs(
        name: &str,
    ) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let scratch = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("scratch");
        let root = scratch.join(name);
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("source");
        let music = root.join("Music");
        std::fs::create_dir_all(&source).expect("synthetic source folder");
        std::fs::create_dir_all(&music).expect("synthetic Music folder");
        std::fs::write(source.join("o2ma757.ojn"), b"synthetic chart")
            .expect("synthetic chart file");
        (root, music, source)
    }

    #[test]
    fn tier_3_purchased_song_completion_sends_one_5012_and_reinks_the_row() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let (root, music, source) = downloaded_chart_dirs("recomp2jam-tier3-purchase-complete");
        let gateway = spawn_purchase_gateway(joined_room(0, true), false);
        let mut session = boot_to_cart_buy(&gateway);
        session
            .game_mut()
            .set_installed_music(Some(o2jam_file::installed_music(["o2ma001.ojn"])));

        // The 5011 alone only queues: no source means no transfer.
        let mut queued_before = Vec::new();
        for _ in 0..12 {
            paint(&mut session);
            queued_before = session
                .game_mut()
                .music_download()
                .requests()
                .iter()
                .map(|request| request.song_id)
                .collect();
            if !queued_before.is_empty() {
                break;
            }
        }
        let row_rect = session
            .game_mut()
            .staged_side_control_rect(MSHOP_EX_FIRST_CODE)
            .expect("the staged row rect owns the installed marker");
        let before = staged_rect_pixels(&mut session, row_rect);
        session
            .game_mut()
            .set_music_download_dirs(music.clone(), Some(source));
        for _ in 0..12 {
            paint(&mut session);
            if session
                .game_mut()
                .select_songs()
                .iter()
                .any(|song| song.song_id == 757 && song.installed)
            {
                break;
            }
        }
        let after = staged_rect_pixels(&mut session, row_rect);
        let body_5012 = wait_for_opcode(&gateway.reports, 5012);
        let mut sent = observed(&gateway.reports);
        sent.push((5012, body_5012));
        let ids: Vec<u16> = sent.iter().map(|(id, _)| *id).collect();
        let bodies: Vec<&Vec<u8>> = sent
            .iter()
            .filter_map(|(id, body)| (*id == 5012).then_some(body))
            .collect();
        let installed = session
            .game_mut()
            .select_songs()
            .iter()
            .find(|song| song.song_id == 757)
            .is_some_and(|song| song.installed);
        let startable = session.game_mut().start_gate_verdict()
            == o2jam_shell::screens::game::StartGateVerdict::Startable;
        let queued_after: Vec<u16> = session
            .game_mut()
            .music_download()
            .requests()
            .iter()
            .map(|request| request.song_id)
            .collect();
        finish(session, gateway.server);
        let promoted = music.join("o2ma757.ojn").exists();
        let staged = music.join("Temp").join("o2ma757.ojn").exists();

        assert_eq!(queued_before, vec![757], "the 5011 must queue one song");
        assert_eq!(queued_after, Vec::<u16>::new(), "the transfer must finish");
        assert_eq!(
            ids.iter().filter(|id| **id == 5012).count(),
            1,
            "one announce: {ids:?}"
        );
        assert_eq!(bodies, vec![&vec![0xF5, 0x02]], "5012 carries the song id");
        assert!(
            promoted && !staged,
            "the chart must land in Music, not Temp"
        );
        assert!(installed && startable, "the bought song must be selectable");
        assert_ne!(before, after, "the installed marker must re-ink the row");
        let _ = std::fs::remove_dir_all(&root);
    }

    fn ranked_row(rank: i32, nickname: &str) -> RankEntry {
        RankEntry {
            rank,
            nickname: nickname.to_owned(),
            battles: 15,
            wins: 9,
            win_rate: 60,
            delta: RankDelta::Down,
            rank_delta: 2,
        }
    }

    fn spawn_top100_gateway(joined: o2jam_net::JoinedRoom) -> LiveLegGateway {
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

            let mut board_sent = false;
            while let Some(frame) = read_frame_until_eof(&mut stream) {
                reported.send(frame.clone()).unwrap();
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
                if frame.id == SendMainRoomMessage::COMMAND && !board_sent {
                    // No 2027 request exists: the board is an
                    // unsolicited answer, as in the scripted tier-2 leg.
                    let answer = MusicPlayRankingAnswer {
                        self_entry: ranked_row(30, "Viewer"),
                        entries: vec![ranked_row(1, "Champ"), ranked_row(2, "Rival")],
                    };
                    stream.write_all(&answer.encode()).unwrap();
                    board_sent = true;
                }
            }
        });
        LiveLegGateway {
            port,
            reports,
            join_answer_written,
            server,
        }
    }

    #[test]
    fn tier_3_scripted_2028_from_boot_inks_the_native_top100_rows() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_top100_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let (x, y) = staged_code_centre(&mut session, Screen::Room, ROOM_MUSICSHOP_BUTTON_CODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);
        assert_eq!(session.game_mut().side(), Some(SideScreen::MusicShop));

        let placements = session.game_mut().staged_side_placements().to_vec();
        let rects = music_shop_top100_rects(&placements);
        assert_eq!(rects.len(), 8, "the eight TOP100 rects must be staged");
        let before = paint_frame(&mut session);
        assert_eq!(session.game_mut().lanking_self_entry(), None);

        session
            .game_mut()
            .set_lobby_input(LobbyChat::Room, "rank please");
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..40 {
            paint(&mut session);
            if session.game_mut().lanking_self_entry().is_some() {
                break;
            }
        }
        let after = paint_frame(&mut session);
        let self_entry = session.game_mut().lanking_self_entry();
        let ranks = session.game_mut().lanking_visible_ranks();
        let nicknames: Vec<String> = ranks.iter().map(|row| row.nickname.clone()).collect();
        let sent: Vec<u16> = observed(&gateway.reports)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| *id == SendMainRoomMessage::COMMAND)
            .collect();
        finish(session, gateway.server);

        assert_eq!(sent, vec![SendMainRoomMessage::COMMAND]);
        assert_eq!(
            self_entry.as_ref().map(|row| row.nickname.as_str()),
            Some("Viewer")
        );
        assert_eq!(nicknames, ["Champ", "Rival"]);
        let first = rects[0];
        assert_ne!(
            staged_rect_pixels_from(&before, first),
            staged_rect_pixels_from(&after, first),
            "the decoded 2028 self line must ink the staged TOP100 rect"
        );
    }
}
