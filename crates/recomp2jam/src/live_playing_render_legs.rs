//! Live Playing render legs (#293). Every leg boots through the real
//! `connect` entry, walks to Playing with `GameLoop` snapshots, and
//! compares the window's painted frame with the staged archive art.
//! Missing archives skip cleanly, so CI with no game data stays green
//! (#0a3ba). Audio is a null rig; no captured sound reaches a speaker.

#[cfg(test)]
mod tests {
    use crate::install_root;
    use crate::live_leg_support::*;
    use crate::session::OnlineSession;
    use o2jam_audio::NullBackend;
    use o2jam_net::SetRoomMusic;
    use o2jam_render::{
        expand_rgb555, frame_image_for_words, pack_xrgb, staged_frame_table_for_count, Frame, Rect,
        Rgb555Image, NATIVE_HEIGHT, NATIVE_WIDTH,
    };
    use o2jam_shell::avatar::{paint_placement_layer, AvatarFigure, FigureLayer, ScenePlacement};
    use o2jam_shell::options::DEFAULT_NOTE_TAIL;
    use o2jam_shell::playing::{
        clear_hold_art_inputs, note_tail_width, set_hold_art_inputs, HighwayGeometry,
    };
    use o2jam_shell::screens::paint::LoadingStatus;
    use o2jam_shell::staging::{
        read_install_archives, stage_state, StageContext, StagedArt, State,
    };
    use o2jam_shell::{GameLoop, InputEdge, InputSnapshot, MenuKey, Screen};
    use std::collections::HashMap;

    /// Both the interface sheet and the Playing archive must be
    /// readable before a live leg stages its own oracle art.
    fn playing_render_archives_present() -> bool {
        archives_present() && read_install_archives(&install_root()).playing.is_some()
    }

    fn native_size() -> (usize, usize) {
        (
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        )
    }

    fn confirm_snapshot() -> InputSnapshot {
        let mut snapshot = InputSnapshot::new();
        snapshot.push_edge(InputEdge::Menu {
            key: MenuKey::Confirm,
            down: true,
        });
        snapshot
    }

    /// `main.rs::run_play` -> `connect::connect_and_login_with_transport`
    /// -> `assemble_online_session` -> `GameLoop::ingest_snapshot` ->
    /// `ScreenGame::enter_screen(Playing)`. The gateway answers master
    /// START with the bare 4011 broadcast; the null rig arms Loading.
    /// A hold chart is staged with the selected option and the two
    /// float inputs the exe's width consumer multiplies.
    fn boot_playing_hold_leg(
        note_tail: u8,
        inputs: Option<(f32, f32)>,
        hold: (u8, u64, u64, u32),
    ) -> (OnlineSession, LiveLegGateway) {
        let gateway = spawn_result_back_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        session.ingest_snapshot(&confirm_snapshot());
        for _ in 0..MAX_SCREEN_PAINTS {
            if session.screen() == Screen::Loading {
                break;
            }
            paint(&mut session);
        }
        assert_eq!(
            session.screen(),
            Screen::Loading,
            "START never armed Loading"
        );

        {
            let game = session.game_mut();
            game.options_mut().set_note_tail(i32::from(note_tail));
            if let Some((chart_scale, speed)) = inputs {
                set_hold_art_inputs(chart_scale, speed);
            } else {
                clear_hold_art_inputs();
            }
            game.set_selected_music(SetRoomMusic {
                music_id: 757,
                difficulty: 0,
                speed: 0,
            });
            game.with_audio(Box::new(NullBackend::new()), HashMap::new());
            game.set_loading_status(LoadingStatus {
                progress: 1.0,
                title: "Song".to_owned(),
                artist: "Band".to_owned(),
            });
            // A far-future hold keeps the life bar alive without inventing
            // a note judgement; no sound is produced by the null rig.
            game.load_chart_holds(vec![hold]);
        }
        // Chart load rebuilds the play state; re-stage the archive skin
        // so the paint leg observes the installed hold art.
        session.stage_live_art();
        session.ingest_snapshot(&confirm_snapshot());
        paint_until_screen(&mut session, Screen::Playing);
        (session, gateway)
    }

    fn boot_playing_render_leg() -> (OnlineSession, LiveLegGateway) {
        boot_playing_hold_leg(DEFAULT_NOTE_TAIL, None, (0, 100_000, 150_000, 7))
    }

    /// Re-stage the same Playing art live boot installed. The oracle
    /// reads the installed archive, never a test-side literal rect.
    fn stage_playing_art(session: &mut OnlineSession) -> StagedArt {
        let installed = read_install_archives(&install_root());
        let Some(playing) = installed.playing else {
            return StagedArt::default();
        };
        let game = session.game_mut();
        let context = StageContext {
            arena_stage: Some(game.arena_stage()),
            key_mode: game.room_key_mode(),
            team_play: game.room_team_play(),
            chart_slot: 0,
        };
        stage_state(&playing, State::Playing, context)
            .art
            .unwrap_or_default()
    }

    fn placement_named<'a>(art: &'a StagedArt, id: &str) -> &'a ScenePlacement {
        art.placements
            .iter()
            .find(|placed| placed.id == id)
            .unwrap_or_else(|| panic!("no staged placement {id}"))
    }

    /// Paint one Playing frame at an explicit song instant. The leg
    /// samples the same instant before and after the option change.
    fn paint_playing_at(session: &mut OnlineSession, now_ms: u64) -> Vec<u32> {
        let (width, height) = native_size();
        let mut buf = vec![0u32; width * height];
        session.game_mut().paint_at(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            now_ms,
        );
        buf
    }

    /// Widest row where two frames disagree inside one staged lane
    /// column. The hold beam is the only action-layer ink that moves
    /// when the `NoteTail` selection changes at a fixed song instant.
    fn widest_hold_diff(before: &[u32], after: &[u32], column: Rect) -> usize {
        let (width, height) = native_size();
        let left = usize::try_from(column.x.max(0)).unwrap_or(0);
        let right = usize::try_from(
            (column.x + i32::try_from(column.w).unwrap_or(0))
                .clamp(0, i32::try_from(width).unwrap_or(0)),
        )
        .unwrap_or(width);
        (0..height)
            .map(|row| {
                (left..right.min(width))
                    .filter(|x| before[row * width + x] != after[row * width + x])
                    .count()
            })
            .max()
            .unwrap_or(0)
    }

    fn figure_named<'a>(art: &'a StagedArt, placed: &ScenePlacement) -> &'a AvatarFigure {
        let sprite = placed.sprite.as_deref().unwrap_or("no sprite");
        art.figures
            .get(sprite)
            .unwrap_or_else(|| panic!("no staged figure {sprite}"))
    }

    fn layer_named<'a>(
        art: &'a StagedArt,
        placed: &ScenePlacement,
        index: usize,
    ) -> &'a FigureLayer {
        figure_named(art, placed)
            .layer(index)
            .unwrap_or_else(|| panic!("no staged layer {index}"))
    }

    fn layer_frames(art: &StagedArt, placed: &ScenePlacement) -> Vec<Rgb555Image> {
        let figure = figure_named(art, placed);
        (0..figure.len())
            .filter_map(|index| figure.layer(index).map(|layer| layer.image.clone()))
            .collect()
    }

    /// Paint one staged layer into a blank native frame, using the same
    /// public painter the Playing scene calls. Only the expected ink is
    /// nonzero, so the live frame can be compared pixel for pixel.
    fn expected_layer_pixels(art: &StagedArt, placed: &ScenePlacement, index: usize) -> Vec<u32> {
        let (width, height) = native_size();
        let mut buf = vec![0u32; width * height];
        paint_placement_layer(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            placed,
            &art.figures,
            index,
            false,
            None,
        );
        buf
    }

    fn ink(pixels: &[u32]) -> usize {
        pixels.iter().filter(|pixel| **pixel != 0).count()
    }

    fn matched_ink(actual: &[u32], expected: &[u32]) -> usize {
        actual
            .iter()
            .zip(expected)
            .filter(|(got, want)| **want != 0 && got == want)
            .count()
    }

    fn assert_ink_matches(actual: &[u32], expected: &[u32], label: &str, percent: usize) {
        let total = ink(expected);
        assert!(total > 0, "{label}: staged body has no ink");
        let matched = matched_ink(actual, expected);
        assert!(
            matched.saturating_mul(100) >= total.saturating_mul(percent),
            "{label}: only {matched}/{total} staged pixels reached the painted frame (need {percent}%)"
        );
    }

    fn ink_bounds(pixels: &[u32]) -> Option<(usize, usize, usize, usize)> {
        let (width, _) = native_size();
        let mut bounds: Option<(usize, usize, usize, usize)> = None;
        for (at, pixel) in pixels.iter().enumerate() {
            if *pixel == 0 {
                continue;
            }
            let x = at % width;
            let y = at / width;
            bounds = Some(match bounds {
                None => (x, y, x + 1, y + 1),
                Some((left, top, right, bottom)) => {
                    (left.min(x), top.min(y), right.max(x + 1), bottom.max(y + 1))
                }
            });
        }
        bounds
    }

    /// Every authored key word in an anchored body must stay absent
    /// from the painted frame at its decoded destination color.
    fn assert_colorkey_stays_out(
        actual: &[u32],
        placed: &ScenePlacement,
        figure: &AvatarFigure,
        layer: &FigureLayer,
    ) {
        let (red, green, blue) = expand_rgb555(layer.colorkey);
        let key_color = pack_xrgb(red, green, blue);
        let (width, height) = native_size();
        // The painter's own origin rule: anchored bodies draw at their
        // record offset, anchor-free bodies at the staged rect origin.
        let (start_x, start_y) = if figure.has_anchor_offset() {
            (layer.dx, layer.dy)
        } else {
            (placed.rect.x, placed.rect.y)
        };
        let mut checked = 0usize;
        for (at, word) in layer.image.words.iter().enumerate() {
            if *word != layer.colorkey {
                continue;
            }
            let column = i32::try_from(at % layer.image.w).expect("column fits a frame coordinate");
            let row = i32::try_from(at / layer.image.w).expect("row fits a frame coordinate");
            let Some(x) = start_x.checked_add(column) else {
                continue;
            };
            let Some(y) = start_y.checked_add(row) else {
                continue;
            };
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (
                usize::try_from(x).expect("x fits a frame index"),
                usize::try_from(y).expect("y fits a frame index"),
            );
            if x >= width || y >= height {
                continue;
            }
            assert_ne!(
                actual[y * width + x],
                key_color,
                "the authored color key leaked into the painted frame at ({x},{y})"
            );
            checked += 1;
        }
        assert!(checked > 0, "the staged body has no key pixels to check");
    }

    #[test]
    fn live_colorkeyed_playing_figure_uploads_from_boot() {
        // Live chain: main.rs::run_play -> connect -> boot_live_leg_with_art
        // -> GameLoop::ingest_snapshot -> GameLoop::paint ->
        // ScreenGame::paint_playing -> paint_placement_layer ->
        // o2jam_render::blit_rgb555. Archives absent -> skip cleanly.
        if !playing_render_archives_present() {
            println!("SKIP live_colorkeyed_playing_figure_uploads_from_boot: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_IMAGE_SPLAY_HOTTIME_ALRAM");
        let expected = expected_layer_pixels(&art, placed, 0);
        let painted = paint_frame(&mut session);
        assert_ink_matches(&painted, &expected, "hot-time frame", 100);
        assert_colorkey_stays_out(
            &painted,
            placed,
            figure_named(&art, placed),
            layer_named(&art, placed, 0),
        );
        finish(session, gateway.server);
    }

    #[test]
    fn live_playing_row_copy_skips_the_authored_key_from_boot() {
        // Same chain as above through `paint_placement_layer` on the
        // chat-window body, whose row runs carry the 0x7D5F key.
        if !playing_render_archives_present() {
            println!(
                "SKIP live_playing_row_copy_skips_the_authored_key_from_boot: install-root absent"
            );
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_IMAGE_SPLAY_CHATWINDOW");
        let expected = expected_layer_pixels(&art, placed, 0);
        let painted = paint_frame(&mut session);
        assert_ink_matches(&painted, &expected, "chat-window rows", 95);
        assert_colorkey_stays_out(
            &painted,
            placed,
            figure_named(&art, placed),
            layer_named(&art, placed, 0),
        );
        finish(session, gateway.server);
    }

    #[test]
    fn live_packed_run_body_lands_each_run_at_its_staged_offset_from_boot() {
        // Live chain ends at the staged `Playing_ChatWindow.oji` body:
        // its repeated-Y/X runs decode into the layer this leg checks.
        if !playing_render_archives_present() {
            println!("SKIP live_packed_run_body_lands_each_run_at_its_staged_offset_from_boot: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_IMAGE_SPLAY_CHATWINDOW");
        let expected = expected_layer_pixels(&art, placed, 0);
        let painted = paint_frame(&mut session);
        assert!(
            ink(&expected) > 20_000,
            "the packed-run body must carry many run pixels"
        );
        assert_ink_matches(&painted, &expected, "packed chat-window runs", 95);
        finish(session, gateway.server);
    }

    #[test]
    fn live_frame_index_word_selects_the_painted_stage_frame_from_boot() {
        // The EXIT control stages two frames. The Tier 0 selector is
        // the oracle for the frame the live scene paints; the source
        // of the shell's index word stays a documented hand-off.
        if !playing_render_archives_present() {
            println!("SKIP live_frame_index_word_selects_the_painted_stage_frame_from_boot: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_BUTTON_SPLAY_EXIT");
        let frames = layer_frames(&art, placed);
        let count = u16::try_from(frames.len()).expect("staged frame count fits a word");
        let selected = frame_image_for_words(&frames, 1, count).expect("frame 1 is staged");
        assert_eq!(selected.words, frames[1].words);
        let painted = paint_frame(&mut session);
        let frame_one = expected_layer_pixels(&art, placed, 1);
        let frame_zero = expected_layer_pixels(&art, placed, 0);
        assert_ink_matches(&painted, &frame_one, "selected EXIT frame", 80);
        assert!(
            matched_ink(&painted, &frame_one) > matched_ink(&painted, &frame_zero),
            "the selected frame must win over the other staged frame"
        );
        finish(session, gateway.server);
    }

    #[test]
    fn live_frame_count_bounds_the_staged_playing_animation_from_boot() {
        // The count word bounds selection to the staged EXIT frames;
        // the live painter shows one frame, never the whole stack.
        if !playing_render_archives_present() {
            println!("SKIP live_frame_count_bounds_the_staged_playing_animation_from_boot: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_BUTTON_SPLAY_EXIT");
        let frames = layer_frames(&art, placed);
        assert!(frame_image_for_words(&frames, 1, 2).is_some());
        assert!(frame_image_for_words(&frames, 1, 1).is_none());
        let painted = paint_frame(&mut session);
        let shown = expected_layer_pixels(&art, placed, 1);
        let other = expected_layer_pixels(&art, placed, 0);
        assert_ink_matches(&painted, &shown, "count-bounded frame", 80);
        assert!(
            matched_ink(&painted, &shown) > matched_ink(&painted, &other),
            "only the selected frame may paint, not the whole stack"
        );
        finish(session, gateway.server);
    }

    #[test]
    fn live_signed_anchor_lands_the_staged_playing_figure_from_boot() {
        // `BpPoint_Left_bar.ojs` anchors at (2,266), not at its bound
        // rect (200,3); the signed record offset must move the ink.
        if !playing_render_archives_present() {
            println!("SKIP live_signed_anchor_lands_the_staged_playing_figure_from_boot: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_IMAGE_SPLAY_BUFFER1");
        let layer = layer_named(&art, placed, 0);
        assert!(
            layer.dx != 0 || layer.dy != 0,
            "body carries a signed anchor"
        );
        let expected = expected_layer_pixels(&art, placed, 0);
        let painted = paint_frame(&mut session);
        assert_ink_matches(&painted, &expected, "anchored buffer bar", 100);
        let bounds = ink_bounds(&expected).expect("the anchored body has ink");
        assert!(
            i32::try_from(bounds.1).expect("top fits i32") >= layer.dy,
            "ink must start at the record anchor, not at the bound rect"
        );
        finish(session, gateway.server);
    }

    #[test]
    fn live_staged_playing_geometry_travels_with_the_figure_from_boot() {
        // The painter consumes the same staged rect and figure the boot
        // installed; no paint-side geometry is invented for this leg.
        if !playing_render_archives_present() {
            println!("SKIP live_staged_playing_geometry_travels_with_the_figure_from_boot: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_IMAGE_SPLAY_BUFFER1");
        let live = session
            .game_mut()
            .scene_placements(Screen::Playing)
            .iter()
            .find(|entry| entry.id == placed.id)
            .unwrap_or_else(|| panic!("live scene lost {}", placed.id))
            .clone();
        assert_eq!(live.rect, placed.rect, "the staged rect travels unchanged");
        let painted = paint_frame(&mut session);
        let expected = expected_layer_pixels(&art, placed, 0);
        assert_ink_matches(&painted, &expected, "staged buffer geometry", 100);
        let bounds = ink_bounds(&expected).expect("staged body has ink");
        let layer = layer_named(&art, placed, 0);
        assert_eq!(bounds.2 - bounds.0, layer.image.w);
        assert_eq!(bounds.3 - bounds.1, layer.image.h);
        finish(session, gateway.server);
    }

    #[test]
    fn live_transformed_playing_image_keeps_the_staged_position_and_size_from_boot() {
        // The time bar's recorded geometry stays its measured rect and
        // size in the painted frame. The exe's transform floats remain
        // a shell-side hand-off, but no ink leaves the staged measure.
        if !playing_render_archives_present() {
            println!("SKIP live_transformed_playing_image_keeps_the_staged_position_and_size_from_boot: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_IMAGE_SPLAY_TIMEBAR");
        let expected = expected_layer_pixels(&art, placed, 0);
        let bounds = ink_bounds(&expected).expect("the time bar has ink");
        assert_eq!(
            bounds,
            (
                usize::try_from(placed.rect.x).expect("x fits usize"),
                usize::try_from(placed.rect.y).expect("y fits usize"),
                usize::try_from(
                    placed.rect.x + i32::try_from(placed.rect.w).expect("width fits i32")
                )
                .expect("right fits usize"),
                usize::try_from(
                    placed.rect.y + i32::try_from(placed.rect.h).expect("height fits i32")
                )
                .expect("bottom fits usize"),
            ),
            "the staged position and size must survive the draw"
        );
        let painted = paint_frame(&mut session);
        assert_ink_matches(&painted, &expected, "transformed time bar", 30);
        finish(session, gateway.server);
    }

    #[test]
    fn live_control_frame_count_stages_one_frame_from_boot() {
        // The control's count word bounds the staged frame table; the
        // live scene paints one of those frames in one control rect.
        if !playing_render_archives_present() {
            println!(
                "SKIP live_control_frame_count_stages_one_frame_from_boot: install-root absent"
            );
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let (mut session, gateway) = boot_playing_render_leg();
        let art = stage_playing_art(&mut session);
        let placed = placement_named(&art, "IDC_BUTTON_SPLAY_EXIT");
        let frames = layer_frames(&art, placed);
        let count = u16::try_from(frames.len()).expect("staged count fits a word");
        assert_eq!(
            staged_frame_table_for_count(&frames, count).len(),
            frames.len()
        );
        assert_eq!(staged_frame_table_for_count(&frames, 1).len(), 1);
        let painted = paint_frame(&mut session);
        let shown = expected_layer_pixels(&art, placed, 1);
        assert_ink_matches(&painted, &shown, "count-bounded EXIT control", 80);
        finish(session, gateway.server);
    }

    #[test]
    fn live_staged_hold_art_width_follows_the_selected_note_tail_pair() {
        // Live chain: main.rs::run_play -> connect::connect_and_login_with_transport
        // -> assemble_online_session -> stage_live_art ->
        // GameLoop::ingest_snapshot -> ScreenGame::enter_screen(Playing) ->
        // ScreenGame::paint_at -> ScreenGame::paint_playing ->
        // LivePlay::paint_hold_tails -> LivePlay::paint_tail_span.
        // Archives absent -> skip cleanly (CI has no game data).
        if !playing_render_archives_present() {
            println!("SKIP live_staged_hold_art_width_follows_the_selected_note_tail_pair: install-root absent");
            return;
        }
        let _live_leg_guard = hold_live_leg_lock();
        let hold = (3, 150_000, 151_000, 7);
        let (mut session, gateway) = boot_playing_hold_leg(0, Some((1.0, 80.0)), hold);
        // The same staged Playing art is present in both samples.
        let _art = stage_playing_art(&mut session);
        let narrow = paint_playing_at(&mut session, 149_500);

        session.game_mut().options_mut().set_note_tail(3);
        session.game_mut().load_chart_holds(vec![hold]);
        session.stage_live_art();
        let wide = paint_playing_at(&mut session, 149_500);

        let (width, height) = native_size();
        let lane = HighwayGeometry::centered(width, height).lane_rect(3);
        let measured = widest_hold_diff(&narrow, &wide, lane);
        let expected = usize::try_from(note_tail_width(3, 1.0, 80.0)).expect("width fits usize");
        assert!(
            measured > 0,
            "pair 0 and pair 3 must change the staged hold width"
        );
        assert!(
            measured <= expected,
            "the changed ink must stay inside the {expected}-px hold width, got {measured}"
        );

        clear_hold_art_inputs();
        session
            .game_mut()
            .options_mut()
            .set_note_tail(i32::from(DEFAULT_NOTE_TAIL));
        finish(session, gateway.server);
    }
}
