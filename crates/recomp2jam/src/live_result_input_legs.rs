//! Tier-3 Result keyboard-input and staged-paint legs (#1032, #1048,
//! #1079). Boot/gateway/paint helpers come from
//! `crate::live_leg_support` (#926); every leg skips cleanly when the
//! installed archives are absent (CI has no game data, #0a3ba). The
//! walk keeps sound on the null backend.

#[cfg(test)]
mod live_result_input_leg_tests {
    use std::collections::HashMap;
    use std::io::Write;
    use std::sync::mpsc::{self, Receiver};
    use std::time::Duration;

    use o2jam_audio::NullBackend;
    use o2jam_net::frame::encode_frame;
    use o2jam_net::relay::PlayingSpeedEvent;
    use o2jam_net::testing::{fake_gateway, read_frame};
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, Frame, JoinRoomResponse, Request,
        RoomList, WireMessage,
    };
    use o2jam_render::{
        draw_text, expand_rgb555, pack_xrgb, Frame as PaintFrame, NATIVE_HEIGHT, NATIVE_WIDTH,
    };
    use o2jam_shell::avatar::load_archive_sprite;
    use o2jam_shell::playing::{HighwayGeometry, PlaySkin};
    use o2jam_shell::staging::read_install_archives;
    use o2jam_shell::{GameLoop, Keymap, Screen};
    use winit::keyboard::KeyCode;

    use crate::install_root;
    use crate::live_leg_support::*;
    use crate::session::OnlineSession;

    /// Boot → Waiting → Playing → Result under the same gateway the
    /// Result BACK script uses. Art is staged before the socket
    /// connects, so the big archive read cannot outrun its quiet timer.
    fn walk_to_live_playing(
        gateway: &LiveLegGateway,
        expected_speed: Option<u8>,
        note_lane: u8,
    ) -> OnlineSession {
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
        session.menu_key(o2jam_shell::MenuKey::Confirm, true);
        paint_until_screen(&mut session, Screen::Loading);
        session.menu_key(o2jam_shell::MenuKey::Confirm, true);
        assert_eq!(session.screen(), Screen::Playing, "the loaded gate opened");
        session.game_mut().load_chart(vec![(note_lane, 1_000, 7)]);
        session.game_mut().strike_at(usize::from(note_lane), 1_000);
        if let Some(expected) = expected_speed {
            for _ in 0..MAX_SCREEN_PAINTS {
                if session.game_mut().result_rank_rows()[0].speed == Some(expected) {
                    break;
                }
                paint(&mut session);
            }
            assert_eq!(
                session.game_mut().result_rank_rows()[0].speed,
                Some(expected),
                "the Playing wire drain must stage the 4050 record"
            );
        }
        session
    }

    /// Finish the shared boot walk into a settled Result panel.
    fn walk_to_live_result(gateway: &LiveLegGateway, expected_speed: Option<u8>) -> OnlineSession {
        let mut session = walk_to_live_playing(gateway, expected_speed, 0);
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
        session.game_mut().paint_at(
            &mut PaintFrame {
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
        let _ = observed(&gateway.reports);
        session
    }

    /// Boot gateway that also emits the 4050 two-write speed event
    /// behind the 4011 answer. The joined room carries speed 1, so a
    /// painted `X5` can only come from the staged member record.
    fn spawn_result_speed_gateway(joined: o2jam_net::JoinedRoom) -> LiveLegGateway {
        let (reported, reports) = mpsc::channel();
        let (join_written_tx, join_answer_written) = mpsc::channel();
        let (port, server) = fake_gateway(move |mut stream| {
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            stream
                .write_all(
                    &AuthResponse {
                        result_code: 0,
                        tail: vec![],
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            stream
                .write_all(
                    &ChannelList {
                        channels: vec![channel_row()],
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            stream
                .write_all(
                    &RoomList {
                        rooms: vec![room_row()],
                    }
                    .encode(),
                )
                .unwrap();

            let mut exit_count = 0u32;
            while let Some(frame) = read_frame_until_eof(&mut stream) {
                if frame.id == Request::StartGame.id() {
                    stream.write_all(&encode_frame(4011, &[])).unwrap();
                }
                if frame.id == Request::ConfirmMusicLoaded.id() {
                    // The 4050 lands after the 4012 answer, while the
                    // shell is entering Playing: the Playing stats drain
                    // owns it, not the Waiting start poll.
                    stream
                        .write_all(
                            &PlayingSpeedEvent {
                                first_speed: 3,
                                final_speed: 9,
                            }
                            .encode(),
                        )
                        .unwrap();
                }
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
                if frame.id == Request::ExitPlaying.id() {
                    exit_count += 1;
                    if exit_count == 2 {
                        stream.write_all(&encode_frame(4022, &[0u8; 13])).unwrap();
                    }
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

    /// White glyph pixels the text renderer lays for one speed caption
    /// at the pinned row-0 GDI anchor (`docs/cresult-anchors.md`).
    fn speed_caption_pixels(caption: &str) -> Vec<(usize, usize, u32)> {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
        let mut blank = vec![0u32; width * height];
        draw_text(
            &mut PaintFrame {
                buf: &mut blank,
                w: width,
                h: height,
            },
            685,
            317,
            caption,
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        blank
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel != 0)
            .map(|(offset, pixel)| (offset % width, offset / width, *pixel))
            .collect()
    }

    /// Bounded opcode wait: a leg that loses its key arm must fail,
    /// not hang the suite while the gateway stays quiet.
    fn await_opcode(reports: &Receiver<Frame>, want: u16) -> Vec<u8> {
        loop {
            match reports.recv_timeout(Duration::from_secs(5)) {
                Ok(frame) if frame.id == want => return frame.payload,
                Ok(_) => {}
                Err(source) => panic!("gateway missed opcode {want}: {source}"),
            }
        }
    }

    /// #1079 Tier-3 boot leg. Live chain:
    /// `main.rs::run_play` → `connect::connect_and_login_with_transport`
    /// → `assemble_online_session` → `window::route_key` →
    /// `route_key_event` → `ScreenGame::capture_raw_key` →
    /// `press_result_back_arm` → `OnlineSession::flush_result_wire` →
    /// `C2S 4021` → scripted `S2C 4022` → `Screen::Waiting`.
    /// The two exe class-2 codes are Backspace and Tab; Enter has no
    /// Result arm. Archives absent → skip cleanly (CI has no data).
    #[test]
    fn live_result_keyboard_back_sends_4021_and_installs_waiting_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_result_back_gateway(joined_room(0, true));
        let mut session = walk_to_live_result(&gateway, None);
        let before = paint_frame(&mut session);
        let mut keymap = Keymap::new();

        let route = route_raw_key(&mut session, &mut keymap, KeyCode::Backspace, None);
        assert!(route.captured, "the settled panel owns Backspace");
        assert!(route.edges.is_empty(), "no menu edge reaches the shell");
        assert_eq!(session.screen(), Screen::Result, "BACK waits for 4022");
        paint(&mut session);
        let body = await_opcode(&gateway.reports, Request::ExitPlaying.id());
        assert!(body.is_empty(), "the keyed 4021 body stays empty");

        let route = route_raw_key(&mut session, &mut keymap, KeyCode::Enter, None);
        assert!(route.captured, "Confirm has no exe Result arm");
        assert!(route.edges.is_empty(), "Enter queues no Confirm edge");
        paint(&mut session);
        assert!(
            !observed(&gateway.reports)
                .iter()
                .any(|(id, _)| *id == Request::StartGame.id()),
            "no keyboard RETRY frame"
        );

        paint_until_screen(&mut session, Screen::Waiting);
        let after = paint_frame(&mut session);
        assert_ne!(before, after, "the answer changed the painted scene");
        finish(session, gateway.server);
    }

    /// #1032 Tier-3 boot leg. Live chain:
    /// `main.rs::run_play` → `connect::connect_and_login_with_transport`
    /// → `assemble_online_session` → `OnlineSession::poll_wire_at` →
    /// `poll_stats` → `drain_peer_echoes` (4050 two writes) →
    /// `ScreenGame::stage_result_member_record` →
    /// `ScreenGame::paint_result` → `paint_result_rank_rows` →
    /// `paint_result_rank_row` speed caption.
    /// Archives absent → skip cleanly (CI has no data).
    #[test]
    fn live_result_speed_from_4050_is_painted_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_result_speed_gateway(joined_room(0, true));
        let mut session = walk_to_live_result(&gateway, Some(9));
        assert_eq!(
            session.game_mut().result_rank_rows()[0].speed,
            Some(9),
            "the second 4050 byte wins the record slot"
        );
        let frame = paint_frame(&mut session);
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let matches = |expected: &[(usize, usize, u32)]| {
            expected
                .iter()
                .filter(|(x, y, word)| frame[y * width + x] == *word)
                .count()
        };
        let staged = speed_caption_pixels("X5");
        let room_default = speed_caption_pixels("X1");
        assert!(!staged.is_empty(), "the caption renderer has glyph ink");
        assert!(
            matches(&staged) * 5 >= staged.len() * 4,
            "the staged 4050 caption must cover the pinned speed column"
        );
        assert!(
            matches(&staged) > matches(&room_default),
            "the painted speed comes from 4050, not the room's speed 1"
        );
        finish(session, gateway.server);
    }

    /// #1048 Tier-3 boot leg. Live chain:
    /// `main.rs::run_play` → `stage_live_scenes` →
    /// `ScreenGame::set_play_skin` → `GameLoop::ingest_snapshot` (lane
    /// strike) → `ScreenGame::paint_playing` → `LivePlay::paint_hit_bursts`
    /// → `paint_lane_flash` → `paint_staged_burst`. Expected ink comes
    /// from the installed `Note_Click.oja` frame the boot staged; no
    /// pixel or anchor is invented. Archives absent → skip cleanly.
    #[test]
    fn live_staged_hit_burst_lands_as_one_body_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let archive = read_install_archives(&install_root())
            .playing
            .expect("playing archive behind the guard");
        let figure = load_archive_sprite(&archive, "Note_Click.oja")
            .expect("the installed bare-arm burst body decodes");
        let layer = figure.layer(1).expect("the second packed burst frame");
        let gateway = spawn_result_back_gateway(joined_room(0, true));
        let mut session = walk_to_live_playing(&gateway, None, 3);
        let skin =
            PlaySkin::from_archive_for_stage(&archive, Some(session.game_mut().arena_stage()))
                .expect("installed Playing skin stages");
        assert_eq!(skin.hit_burst_frame_count(), 6, "the real ring animation");
        session.game_mut().set_play_skin(skin);
        session.game_mut().strike_at(3, 1_000);

        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
        let mut buf = vec![0u32; width * height];
        session.game_mut().paint_at(
            &mut PaintFrame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            1_050,
        );
        assert_eq!(
            session.screen(),
            Screen::Playing,
            "the burst frame is in play"
        );

        let geom = HighwayGeometry::centered(width, height);
        let lane = geom.lane_rect(3);
        let pad = geom.receptor_pad(3);
        let lane_width = i32::try_from(lane.w).expect("lane width fits i32");
        let art_width = i32::try_from(layer.image.w).expect("art width fits i32");
        let art_height = i32::try_from(layer.image.h).expect("art height fits i32");
        let pad_height = i32::try_from(pad.h).expect("pad height fits i32");
        let left = lane.x + (lane_width - art_width) / 2;
        let top = pad.y + pad_height / 2 - art_height / 2;
        let expected: Vec<(usize, usize, u32)> = layer
            .image
            .words
            .iter()
            .enumerate()
            .filter_map(|(index, word)| {
                if *word == 0 || *word == layer.colorkey {
                    return None;
                }
                let column = index % layer.image.w;
                let row = index / layer.image.w;
                let x = left + i32::try_from(column).expect("column fits i32");
                let y = top + i32::try_from(row).expect("row fits i32");
                if x < 0 || y < 0 || x >= i32::try_from(width).expect("width fits i32") {
                    return None;
                }
                let (red, green, blue) = expand_rgb555(*word);
                Some((
                    usize::try_from(x).expect("x fits usize"),
                    usize::try_from(y).expect("y fits usize"),
                    pack_xrgb(red, green, blue),
                ))
            })
            .collect();
        assert!(expected.len() > 100, "the ring body carries real art");
        let painted = expected
            .iter()
            .filter(|(x, y, word)| buf[y * width + x] == *word)
            .collect::<Vec<_>>();
        assert!(
            painted.len() * 2 >= expected.len(),
            "the installed burst body must land as one painted mass ({} of {})",
            painted.len(),
            expected.len()
        );
        let rows = painted
            .iter()
            .map(|(_, y, _)| *y)
            .collect::<std::collections::HashSet<_>>();
        let columns = painted
            .iter()
            .map(|(x, _, _)| *x)
            .collect::<std::collections::HashSet<_>>();
        assert!(
            rows.len() >= 20 && columns.len() >= 20,
            "the burst must span a body, not scattered runs ({} rows, {} cols)",
            rows.len(),
            columns.len()
        );
        finish(session, gateway.server);
    }
}
