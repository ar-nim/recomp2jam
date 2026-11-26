#![cfg(test)]

//! Live-input legs for the Playing focus/acquire family (#293).
//!
//! Each leg boots through the real connect entry and skips cleanly
//! when the installed archives are absent (CI has no game data,
//! #0a3ba). Audio stays on `NullBackend` (containment rule).

use std::collections::HashMap;
use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use o2jam_audio::{NullBackend, PcmSamples};
use o2jam_net::frame::encode_frame;
use o2jam_net::play::SetRoomMusic;
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
    WireMessage,
};
use o2jam_shell::screens::paint::LoadingStatus;
use o2jam_shell::{GameLoop, MenuKey, Screen};

use crate::live_leg_support::{
    archives_present, boot_live_leg, channel_row, dismiss_splash, enter_room_by_clicks, finish,
    join_waiting_by_click, joined_room, paint, paint_frame, paint_until_screen,
    read_frame_until_eof, room_row, spawn_live_leg_gateway, spawn_result_back_gateway,
    LiveLegGateway, MAX_SCREEN_PAINTS,
};
use crate::session::OnlineSession;

// Caption bytes at VA `0x650d68`, `0x650d38` and `0x650d08`
// (`docs/pins/input-msg.md` addendum, ev_0298023742…). Kept as
// literals here so the live leg checks the pin, not the shell's
// own constant.
const ROOM_EXIT_NOTICE: &str = "PLAY중 화면이 전환 되어서 Room으로 나왔습니다.";
const LOBBY_EXIT_NOTICE: &str = "PLAY중 화면이 전환 되어서 로비로 나왔습니다.";
const WAITING_EXIT_NOTICE: &str = "PLAY중 화면이 전환 되어서 대기실로 나왔습니다.";

/// Boot the real online entry, click into the staged room, and walk
/// from Waiting to Loading on the 4010 edge. The room's title word is
/// the one the exe's activation arm inspects.
fn boot_focus_leg(room: o2jam_net::JoinedRoom) -> (OnlineSession, LiveLegGateway) {
    let gateway = spawn_result_back_gateway(room);
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    session
        .game_mut()
        .with_audio(Box::new(NullBackend::new()), HashMap::new());
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
    session.menu_key(MenuKey::Confirm, true);
    paint_until_screen(&mut session, Screen::Loading);
    (session, gateway)
}

/// Synthetic BGM bank: record 9 is the Planet/Channel track cue.
fn focus_cue_bank() -> Vec<Option<PcmSamples>> {
    let mut bank = vec![None; 10];
    bank[8] = Some(PcmSamples {
        samples: vec![9_i16; 441],
        sample_rate: 44_100,
        channels: 1,
    });
    bank
}

#[test]
fn live_focus_off_and_on_reach_the_booted_scene_bgm_stop_hook() {
    // Live caller chain: `main.rs::run_play` ->
    // `connect::connect_and_login_with_transport` ->
    // `assemble_online_session` -> `boot_live_leg` -> `dismiss_splash` ->
    // `GameLoop::focus_changed` -> `OnlineSession::focus_changed` ->
    // `ScreenGame::stop_bgm_for_focus_loss` ->
    // `AudioBackend::set_scene_bgm(None, false)`. Archives absent ->
    // skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_live_leg_gateway(joined_room(0, true));
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    assert_eq!(session.screen(), Screen::Planet);
    session
        .game_mut()
        .with_audio(Box::new(NullBackend::new()), HashMap::new());
    session.game_mut().stage_scene_bgm(focus_cue_bank());
    assert_eq!(
        session
            .game_mut()
            .scene_bgm_selection()
            .map(|cue| cue.record),
        Some(9),
        "the booted scene has a sounding track"
    );

    session.focus_changed(false);
    assert_eq!(
        session.game_mut().scene_bgm_selection(),
        None,
        "focus off stops the track through the live hook"
    );

    session.focus_changed(true);
    assert_eq!(
        session.game_mut().scene_bgm_selection(),
        None,
        "activation restores audio with the exe's silence cue"
    );

    drop(session);
    gateway.server.join().unwrap();
}

/// Boot gateway with a separate writer for a frame pushed after the
/// client reaches Waiting. The 3003 write signal removes the join
/// race the boot walk would otherwise have under test-fleet load.
struct AcquireGateway {
    port: u16,
    join_answer_written: Receiver<()>,
    script: Sender<Vec<u8>>,
    script_written: Receiver<()>,
    server: JoinHandle<()>,
}

fn spawn_acquire_gateway(room: o2jam_net::JoinedRoom) -> AcquireGateway {
    let (join_tx, join_answer_written) = mpsc::channel();
    let (script, script_rx) = mpsc::channel::<Vec<u8>>();
    let (script_written_tx, script_written) = mpsc::channel();
    let (port, server) = fake_gateway(move |mut stream: std::net::TcpStream| {
        assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
        stream
            .write_all(
                &AuthResponse {
                    result_code: 0,
                    tail: Vec::new(),
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

        let mut writer = stream.try_clone().unwrap();
        let writer = thread::spawn(move || {
            for frame in script_rx {
                writer.write_all(&frame).unwrap();
                script_written_tx.send(()).unwrap();
            }
        });
        while let Some(frame) = read_frame_until_eof(&mut stream) {
            if frame.id == Request::JoinWaiting.id() {
                stream
                    .write_all(
                        &JoinRoomResponse {
                            result_code: 0,
                            room: Some(room.clone()),
                        }
                        .encode(),
                    )
                    .unwrap();
                join_tx.send(()).unwrap();
            }
        }
        writer.join().unwrap();
    });
    AcquireGateway {
        port,
        join_answer_written,
        script,
        script_written,
        server,
    }
}

#[test]
fn live_4035_acquire_arms_the_local_badge_from_boot() {
    // Live caller chain: `main.rs::run_play` ->
    // `connect::connect_and_login_with_transport` ->
    // `assemble_online_session` -> boot pointer walk ->
    // `OnlineSession::paint` -> `poll_wire_at` -> `apply_acquire_music` ->
    // `post_acquire_music_ui_events` + `answer_acquire_music` ->
    // `ScreenGame::arm_local_download_badge` ->
    // `paint_waiting_seat_marks`. The exact event sequence is pinned by
    // the Tier 2 `tier_2_4035_posts_one_ui_event_per_row_and_one_for_an_empty_list`
    // guard; this leg proves the same frame reaches the booted scene.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_acquire_gateway(joined_room(0, true));
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    let before = paint_frame(&mut session);

    // Two acquired rows: the local member is Ready, the peer is
    // Downloading — the #773 `4035 0200000104` shape.
    gateway
        .script
        .send(encode_frame(4035, &[2, 0, 1, 1, 4]))
        .unwrap();
    gateway.script_written.recv().unwrap();
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().local_download_badge_armed() {
            break;
        }
        paint(&mut session);
    }
    assert!(
        session.game_mut().local_download_badge_armed(),
        "4035 arms the local badge on the booted Waiting board"
    );
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the acquire frame repainted the Waiting board"
    );

    drop(session);
    drop(gateway.script);
    gateway.server.join().unwrap();
}

/// Tier 3 focus-off/on leg for the Playing exit (#1046). The off edge
/// must not change the scene; the on edge installs Room with the
/// pinned caption. Audio stays on the null backend.
#[test]
fn live_focus_return_from_playing_installs_room_from_boot() {
    // Live caller chain: `main.rs::run_play` ->
    // `connect::connect_and_login_with_transport` ->
    // `assemble_online_session` -> boot pointer walk -> Loading Confirm ->
    // `OnlineSession::focus_changed` -> `ScreenGame::focus_changed` ->
    // `ScreenGame::apply_focus_change` -> `ScreenGame::enter_screen(Room)`.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_focus_leg(joined_room(0, true));
    session.menu_key(MenuKey::Confirm, true);
    assert_eq!(session.screen(), Screen::Playing, "the gate handed off");
    let before = paint_frame(&mut session);

    session.focus_changed(false);
    assert_eq!(
        session.screen(),
        Screen::Playing,
        "focus loss only records the pause"
    );
    session.focus_changed(true);
    assert_eq!(session.screen(), Screen::Room, "activation installs Room");
    assert_eq!(
        session.game_mut().focus_exit_notice(),
        Some(ROOM_EXIT_NOTICE),
        "the pinned Room notice is staged"
    );
    assert_ne!(before, paint_frame(&mut session), "the exit repainted");
    finish(session, gateway.server);
}

/// Tier 3 focus-off/on leg for Loading with an empty room title (#1046).
#[test]
fn live_focus_return_from_loading_empty_title_clears_seats_and_installs_room_from_boot() {
    // Live caller chain: `main.rs::run_play` ->
    // `connect::connect_and_login_with_transport` ->
    // `assemble_online_session` -> boot pointer walk -> Loading ->
    // `OnlineSession::focus_changed` -> `ScreenGame::apply_focus_change` ->
    // `ScreenGame::clear_waiting_seats` ->
    // `ScreenGame::enter_screen(Screen::Room)`. Archives absent -> skip.
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let mut room = joined_room(0, true);
    room.title.clear();
    let (mut session, gateway) = boot_focus_leg(room);
    assert!(
        session.game_mut().staged_waiting_member(1).is_some(),
        "the booted room has a peer seat"
    );
    let before = paint_frame(&mut session);

    session.focus_changed(false);
    assert_eq!(session.screen(), Screen::Loading, "focus loss stays put");
    session.focus_changed(true);
    assert_eq!(session.screen(), Screen::Room, "activation installs Room");
    assert_eq!(
        session.game_mut().focus_exit_notice(),
        Some(LOBBY_EXIT_NOTICE),
        "an empty title posts the pinned lobby notice"
    );
    assert_eq!(
        session.game_mut().staged_waiting_member(1),
        None,
        "the Waiting seats were cleared"
    );
    assert_ne!(before, paint_frame(&mut session), "the exit repainted");
    finish(session, gateway.server);
}

/// Tier 3 focus-off/on leg for Loading with a named room (#1046).
#[test]
fn live_focus_return_from_loading_named_room_installs_waiting_from_boot() {
    // Live caller chain: `main.rs::run_play` ->
    // `connect::connect_and_login_with_transport` ->
    // `assemble_online_session` -> boot pointer walk -> Loading ->
    // `OnlineSession::focus_changed` -> `ScreenGame::apply_focus_change`
    // -> `ScreenGame::enter_screen(Screen::Waiting)`. Archives absent -> skip.
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_focus_leg(joined_room(0, true));
    let before = paint_frame(&mut session);

    session.focus_changed(false);
    assert_eq!(session.screen(), Screen::Loading, "focus loss stays put");
    session.focus_changed(true);
    assert_eq!(
        session.screen(),
        Screen::Waiting,
        "activation installs Waiting"
    );
    assert_eq!(
        session.game_mut().focus_exit_notice(),
        Some(WAITING_EXIT_NOTICE),
        "a named room posts the pinned Waiting notice"
    );
    assert!(
        session.game_mut().staged_waiting_member(1).is_some(),
        "the named room keeps its seats"
    );
    assert_ne!(before, paint_frame(&mut session), "the exit repainted");
    finish(session, gateway.server);
}
