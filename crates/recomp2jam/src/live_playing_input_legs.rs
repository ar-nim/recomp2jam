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
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
    WireMessage,
};
use o2jam_shell::{GameLoop, Screen};

use crate::live_leg_support::{
    archives_present, boot_live_leg, channel_row, dismiss_splash, enter_room_by_clicks,
    join_waiting_by_click, joined_room, paint, paint_frame, read_frame_until_eof, room_row,
    spawn_live_leg_gateway, MAX_SCREEN_PAINTS,
};

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
