//! Tier-3 Result paint legs (#1073-#1075, #1080). Boot/gateway/paint
//! helpers come from `crate::live_leg_support` (#926); every leg skips
//! cleanly when the installed archives are absent (CI has no game
//! data, #0a3ba). Sound stays on the null backend during the walk.

#[cfg(test)]
mod live_result_leg_tests {
    use std::collections::HashMap;

    use o2jam_audio::NullBackend;
    use o2jam_render::{expand_rgb555, pack_xrgb, Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::avatar::load_archive_sprite;
    use o2jam_shell::screens::paint::RESULT_RANK_SPRITE;
    use o2jam_shell::staging::read_install_archives;
    use o2jam_shell::{GameLoop, Screen};

    use crate::install_root;
    use crate::live_leg_support::*;
    use crate::session::OnlineSession;

    /// Walk the real boot → Waiting → Playing → Result path under a
    /// null audio backend, with art staged before the gateway connects
    /// (the big archive read must not outrun the gateway quiet timer).
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
        session.menu_key(o2jam_shell::MenuKey::Confirm, true);
        paint_until_screen(&mut session, Screen::Loading);
        session.menu_key(o2jam_shell::MenuKey::Confirm, true);
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
        let _ = observed(&gateway.reports);
        session
    }

    /// #1073-#1075 Tier-3 Result paint capture. Live chain:
    /// `main.rs::run_play` → `connect::connect_and_login_with_transport`
    /// → `assemble_online_session` → `GameLoop::ingest_snapshot` →
    /// `ScreenGame::paint_result` → `paint_result_scene` →
    /// `paint_result_backdrop` / `paint_result_body_frame`, reading the
    /// staged `STATE_RESULT` placements. Archives absent → skip cleanly.
    #[test]
    fn live_result_paint_leg_inks_the_staged_panels_from_boot() {
        if !archives_present() {
            println!("SKIP live_result_paint_leg_inks_the_staged_panels_from_boot: missing install-root interface archive");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let gateway = spawn_result_back_gateway(joined_room(0, true));
        let mut session = walk_to_live_result(&gateway);
        let codes = [
            0x1710_0001u32, // fill
            0x1760_0002,    // rank panel
            0x1760_0003,    // result panel
            0x1750_0004,    // left tab
            0x1720_0005,    // BACK
            0x1720_2006,    // RETRY
            0x1750_0007,    // grade plate
        ];
        for code in codes {
            assert!(
                session
                    .game_mut()
                    .scene_placements(Screen::Result)
                    .iter()
                    .any(|placed| placed.code == code),
                "the staged Result block must carry {code:#x}"
            );
        }
        let frame = paint_frame(&mut session);
        let panel_rects = [
            Rect {
                x: 29,
                y: 316,
                w: 29,
                h: 90,
            },
            Rect {
                x: 29,
                y: 406,
                w: 29,
                h: 90,
            },
            Rect {
                x: 395,
                y: 531,
                w: 79,
                h: 41,
            },
        ];
        for rect in panel_rects {
            let ink = staged_rect_pixels_from(&frame, rect);
            assert!(
                ink.iter().any(|pixel| *pixel != 0),
                "the settled Result frame must ink the staged rect {rect:?}"
            );
        }
        finish(session, gateway.server);
    }

    /// #1080 Tier-3 live-art leg. Live chain:
    /// `main.rs::run_play` → `stage_live_scenes` → `ScreenGame::paint_result`
    /// → `paint_result_backdrop` → `0x17100001` fill body → page blit.
    /// The expected ink comes from the same installed `State_Result.ojs`
    /// body live boot staged; no pixel value is invented.
    /// Archives absent → skip cleanly (CI has no game data).
    #[test]
    fn live_result_fill_body_ink_is_sampled_in_a_settled_result_frame() {
        if !archives_present() {
            println!("SKIP live_result_fill_body_ink_is_sampled_in_a_settled_result_frame: missing install-root interface archive");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive behind the guard");
        let fill = load_archive_sprite(&archive, "State_Result.ojs")
            .expect("the unconditional State_Result fill body decodes");
        let gateway = spawn_result_back_gateway(joined_room(0, true));
        let mut session = walk_to_live_result(&gateway);
        let frame = paint_frame(&mut session);

        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let matches = (0..fill.len())
            .filter_map(|index| fill.layer(index))
            .flat_map(|layer| {
                let image = layer.image.view();
                (0..image.h).flat_map(move |row| {
                    (0..image.w).filter_map(move |col| {
                        let word = image.pixels[row * image.stride + col];
                        if word == 0 || word == layer.colorkey {
                            return None;
                        }
                        let (red, green, blue) = expand_rgb555(word);
                        let x = layer.dx + i32::try_from(col).expect("column fits i32");
                        let y = layer.dy + i32::try_from(row).expect("row fits i32");
                        (x >= 0 && y >= 0 && x < i32::try_from(width).expect("width fits i32"))
                            .then(|| {
                                (
                                    usize::try_from(y).expect("y fits usize") * width
                                        + usize::try_from(x).expect("x fits usize"),
                                    pack_xrgb(red, green, blue),
                                )
                            })
                    })
                })
            })
            .filter(|(offset, expected)| frame.get(*offset) == Some(expected))
            .count();
        // The fill body covers the whole 800x600 page. Other staged
        // bodies reuse some of its RGB555 words, so the guard asks for
        // whole-page coverage, not a single coincidental match.
        assert!(
            matches > 100_000,
            "the settled frame must carry the whole-page fill body's ink ({matches} matches)"
        );
        assert_eq!(
            session
                .game_mut()
                .scene_placements(Screen::Result)
                .iter()
                .find(|placed| placed.code == 0x1710_0001)
                .map(|placed| placed.sprite.as_deref()),
            Some(Some("State_Result.ojs")),
            "the sampled body is the one bound to the fill control"
        );
        assert!(
            session
                .game_mut()
                .scene_placements(Screen::Result)
                .iter()
                .any(|placed| placed.code == 0x1750_0007
                    && placed.sprite.as_deref() == Some(RESULT_RANK_SPRITE)),
            "the same settled scene stages the grade plate"
        );
        finish(session, gateway.server);
    }
}
