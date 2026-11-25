#![cfg(test)]

//! Tier 3 wire legs for the Playing/battle family (#293).
//!
//! Each leg boots through the real connect entry, walks to Waiting
//! through pointer snapshots, pushes one scripted gateway frame, and
//! observes the painted Waiting board. Missing `install-root` archives
//! skip the leg cleanly (CI has no game data, #0a3ba). The two-client
//! live-battle Playing legs remain OPEN; this file does not fake them.

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use o2jam_net::frame::encode_frame;
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
    WireMessage,
};

use crate::live_leg_support::{
    archives_present, boot_live_leg, channel_row, dismiss_splash, enter_room_by_clicks,
    join_waiting_by_click, joined_room, paint, paint_frame, read_frame_until_eof, room_row,
    MAX_SCREEN_PAINTS,
};
use crate::session::OnlineSession;

/// Boot gateway plus the scripted S2C writer a live wire leg needs.
struct PlayingWireGateway {
    port: u16,
    join_answer_written: Receiver<()>,
    script: Sender<Vec<u8>>,
    script_written: Receiver<()>,
    server: JoinHandle<()>,
}

impl PlayingWireGateway {
    /// Push one wire frame and wait until the gateway wrote it.
    fn push(&self, bytes: Vec<u8>) {
        self.script.send(bytes).unwrap();
        self.script_written.recv().unwrap();
    }
}

/// Drive the boot handshake, answer the 3002 join, and hold a separate
/// writer so a quiet Waiting client can still receive a pushed frame.
fn spawn_playing_wire_gateway(room: o2jam_net::JoinedRoom) -> PlayingWireGateway {
    let (reported_tx, _reported) = mpsc::channel();
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
            let _ = reported_tx.send(frame);
        }
        writer.join().unwrap();
    });
    PlayingWireGateway {
        port,
        join_answer_written,
        script,
        script_written,
        server,
    }
}

/// Walk from Planet to Waiting through the staged pointer path.
fn boot_into_playing_wire(gateway: &PlayingWireGateway) -> OnlineSession {
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    session
}

/// Close the socket first so the writer thread can end before join.
fn finish(session: OnlineSession, gateway: PlayingWireGateway) {
    drop(session);
    drop(gateway.script);
    gateway.server.join().unwrap();
}

#[test]
fn live_4035_answer_arms_the_local_download_badge_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_wire_at ->
    // poll_waiting_chat -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_acquire_music -> answer_acquire_music ->
    // ScreenGame::arm_local_download_badge -> paint_waiting_seat_marks.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_playing_wire_gateway(joined_room(0, true));
    let mut session = boot_into_playing_wire(&gateway);
    let before = paint_frame(&mut session);
    gateway.push(encode_frame(4035, &[1, 0, 4]));
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().local_download_badge_armed() {
            break;
        }
        paint(&mut session);
    }
    assert!(
        session.game_mut().local_download_badge_armed(),
        "4035 must arm the local seat's badge on the boot walk"
    );
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the arm badge repainted the Waiting board"
    );
    finish(session, gateway);
}
