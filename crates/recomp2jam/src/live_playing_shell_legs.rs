//! Tier 3 live-path legs for the Playing shell family (#1033-#1036,
//! #1051-#1068). Boot and paint helpers come from
//! `crate::live_leg_support` (#926). Every leg returns early when the
//! installed archives are absent (CI has no game data, #0a3ba).

#[cfg(test)]
mod tests {
    use o2jam_net::play::SetRoomMusic;
    use o2jam_render::{Rgb555Image, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::playing::{PlaySkin, SkinSprite};
    use o2jam_shell::screens::paint::LoadingStatus;
    use o2jam_shell::Screen;

    use crate::live_leg_support::*;
    use crate::session::OnlineSession;

    /// Walk the boot session through the state table into the armed
    /// Loading gate. The gateway's quiet timer can expire while an
    /// unattended leg stages archives, so scene transitions stay on
    /// the shell seam; input-driven walks belong to live_input_legs.
    fn walk_to_playing(session: &mut OnlineSession) {
        for screen in [
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
        ] {
            assert!(session.game_mut().enter_screen(screen), "{screen:?} entry");
        }
        session.game_mut().set_selected_music(SetRoomMusic {
            music_id: 757,
            difficulty: 1,
            speed: 1,
        });
        session.game_mut().set_loading_status(LoadingStatus {
            progress: 1.0,
            title: "First".to_owned(),
            artist: "Band".to_owned(),
        });
        assert!(
            session.game_mut().loading_gate_ready(),
            "the live shell must arm the song clock"
        );
        assert!(session.game_mut().enter_screen(Screen::Playing));
        assert_eq!(session.screen(), Screen::Playing, "the loaded gate opened");
    }

    fn marker_sprite(word: u16) -> SkinSprite {
        SkinSprite::new(
            Rgb555Image {
                w: 10,
                h: 10,
                words: vec![word; 100],
            },
            0,
        )
    }

    fn paint_at_frame(session: &mut OnlineSession, now_ms: u64) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits a frame index");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits a frame index");
        let mut buffer = vec![0u32; width * height];
        session.game_mut().paint_at(
            &mut o2jam_render::Frame {
                buf: &mut buffer,
                w: width,
                h: height,
            },
            now_ms,
        );
        buffer
    }

    #[test]
    // Tier 3 live chain: `main.rs::run_online` ->
    // `connect::connect_and_login_with_transport` ->
    // `boot_live_leg_with_art` -> `attach_initial_audio` ->
    // `ScreenGame::enter_screen` -> `Screen::bgm_cue` ->
    // `SceneBgm::cue`. The backend's own cue index remains OPEN; this
    // leg reads the shell's latched record from the live boot.
    // Archives absent -> skip cleanly (CI has no game data).
    fn live_playing_shell_scene_cues_replace_the_active_record_from_boot() {
        if !archives_present() {
            println!("skip: install archives absent");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, false));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        // Same null-sink bank attach as `main.rs::run_online`, with
        // audio contained for an unattended leg (#979fb).
        session.attach_initial_audio(false);
        assert_eq!(
            session
                .game_mut()
                .scene_bgm_selection()
                .map(|cue| cue.record),
            Some(9),
            "the Planet entry must latch the pinned BGM record"
        );
        walk_to_playing(&mut session);
        assert_eq!(
            session.game_mut().scene_bgm_selection(),
            None,
            "Playing replaces the active cue with silence"
        );
        paint(&mut session);
        finish(session, gateway.server);
    }

    #[test]
    // Tier 3 live chain: `main.rs::run_online` -> live boot ->
    // `ScreenGame::enter_screen(Playing)` -> `ScreenGame::set_play_skin`
    // -> `ScreenGame::paint_at` -> `LivePlay::paint` -> staged frame
    // selection. No real key or display: the clock is explicit and
    // the backend stays null (audio containment).
    // Archives absent -> skip cleanly (CI has no game data).
    fn live_playing_shell_flash_frames_change_across_paint_at_from_boot() {
        if !archives_present() {
            println!("skip: install archives absent");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, false));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        session.attach_initial_audio(false);
        walk_to_playing(&mut session);
        // One chart note and a two-frame synthetic burst: a strike at
        // 1000 ms arms the flash, then the explicit clock selects the
        // second staged frame at 1050 ms.
        session.game_mut().load_chart(vec![(3, 1000, 0)]);
        session.game_mut().set_play_skin(
            PlaySkin::default().with_hit_bursts(vec![marker_sprite(0x7FE0), marker_sprite(0x03E0)]),
        );
        session.game_mut().strike_at(3, 1000);
        let first = paint_at_frame(&mut session, 1000);
        let second = paint_at_frame(&mut session, 1050);
        assert_ne!(
            first, second,
            "the painted Playing frame must follow the explicit clock"
        );
        paint(&mut session);
        finish(session, gateway.server);
    }
}
