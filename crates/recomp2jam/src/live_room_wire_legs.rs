#![cfg(test)]

//! Tier 3 wire legs for the Room board (lane room-wire).
//!
//! Each leg boots through the real connect entry, walks to Room through
//! pointer snapshots, then pushes one scripted gateway event. Missing
//! `install-root` archives skip the leg cleanly (#0a3ba).

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use o2jam_net::message::{
    RoomDescriptorChanged, RoomSkillChanged, RoomStateChanged, RoomUserCountChanged,
};
use o2jam_net::rooms::SendWhisperResponse;
use o2jam_net::testing::fake_gateway;
use o2jam_net::{AuthResponse, ChannelList, ChannelLoginResponse, Request, WireMessage};

use crate::live_leg_support::{
    archives_present, boot_live_leg, channel_row, dismiss_splash, enter_room_by_clicks, finish,
    paint, paint_frame, read_frame_until_eof, room_row, MAX_SCREEN_PAINTS,
};
use crate::session::OnlineSession;

/// Boot gateway plus the socket's scripted-frame queue.
struct WireBootGateway {
    port: u16,
    server: JoinHandle<()>,
}

/// Boot handshake plus a scripted-frame queue. The queue keeps a leg
/// free of timing races: the test sends an event only after the Room
/// board is on screen, so no Channel drain can consume it first.
fn spawn_wire_gateway() -> (WireBootGateway, Sender<Vec<u8>>) {
    let (send, receive): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let boot = move |mut stream: std::net::TcpStream| {
        assert_eq!(
            read_frame_until_eof(&mut stream).unwrap().id,
            Request::Authorize.id()
        );
        stream
            .write_all(
                &AuthResponse {
                    result_code: 0,
                    tail: Vec::new(),
                }
                .encode(),
            )
            .unwrap();
        assert_eq!(
            read_frame_until_eof(&mut stream).unwrap().id,
            Request::GetChannelList.id()
        );
        stream
            .write_all(
                &ChannelList {
                    channels: vec![channel_row()],
                }
                .encode(),
            )
            .unwrap();
        assert_eq!(
            read_frame_until_eof(&mut stream).unwrap().id,
            Request::ChannelLogin.id()
        );
        stream
            .write_all(
                &ChannelLoginResponse {
                    result_code: 0,
                    restricted: Some(false),
                }
                .encode(),
            )
            .unwrap();
        assert_eq!(
            read_frame_until_eof(&mut stream).unwrap().id,
            Request::GetChannelInfo.id()
        );
        stream
            .write_all(
                &o2jam_net::RoomList {
                    rooms: vec![room_row()],
                }
                .encode(),
            )
            .unwrap();
        while let Ok(frame) = receive.recv() {
            stream.write_all(&frame).unwrap();
        }
    };
    let (port, server) = fake_gateway(boot);
    (WireBootGateway { port, server }, send)
}

/// Paint until the Room row reaches the condition. Counting frames,
/// not wall time, keeps a loaded test fleet out of the assertion.
fn paint_until_row(session: &mut OnlineSession, ready: impl Fn(&mut OnlineSession) -> bool) {
    for _ in 0..MAX_SCREEN_PAINTS {
        paint(session);
        if ready(session) {
            return;
        }
    }
    panic!("scripted wire event never reached the staged Room row");
}

#[test]
fn live_room_wire_2009_user_count_repaints_the_row_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session ->
    // enter_room_by_clicks -> OnlineSession::paint -> poll_lobby_chat ->
    // drain_lobby_chat -> apply_lobby_frame -> apply_room_user_count ->
    // rewrite_room_row -> ScreenGame::upsert_room -> paint_room_list.
    if !archives_present() {
        return;
    }
    let (gateway, send) = spawn_wire_gateway();
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    let change = RoomUserCountChanged {
        number: 3,
        capacity: 8,
        user_count: 5,
        descriptor_word: 0x0102,
        descriptor_byte: 7,
    };
    send.send(change.encode()).unwrap();
    paint_until_row(&mut session, |session| {
        session
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|room| room.number == 3 && room.user_count == 5)
    });
    let row = session
        .game_mut()
        .listed_rooms()
        .iter()
        .find(|room| room.number == 3)
        .expect("boot row still staged");
    assert_eq!((row.capacity, row.user_count), (8, 5));
    let pixels = paint_frame(&mut session);
    assert!(pixels.iter().any(|pixel| *pixel != 0), "Room board painted");
    drop(send);
    finish(session, gateway.server);
}

#[test]
fn live_room_wire_2018_answer_stages_a_delivery_line_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session ->
    // enter_room_by_clicks -> OnlineSession::paint -> poll_lobby_chat ->
    // drain_lobby_chat -> apply_lobby_frame -> apply_whisper_answer ->
    // ScreenGame::push_lobby_user_message -> Room chat list.
    if !archives_present() {
        return;
    }
    let (gateway, send) = spawn_wire_gateway();
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    let answer = SendWhisperResponse {
        invalid: false,
        recipient: "peer".to_owned(),
        content: Some("psst".to_owned()),
    };
    send.send(answer.encode()).unwrap();
    paint_until_row(&mut session, |session| {
        !session.game_mut().room_chat().is_empty()
    });
    let row = &session.game_mut().room_chat()[0];
    assert_eq!(row.sender, "<<");
    assert_eq!(row.text, "peer: psst");
    drop(send);
    finish(session, gateway.server);
}

#[test]
fn live_room_wire_2020_state_and_2025_skills_change_the_row_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session ->
    // enter_room_by_clicks -> OnlineSession::paint -> poll_lobby_chat ->
    // apply_lobby_frame -> apply_room_state / apply_room_skills ->
    // rewrite_room_row -> ScreenGame::upsert_room -> Room badges.
    if !archives_present() {
        return;
    }
    let (gateway, send) = spawn_wire_gateway();
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    send.send(
        RoomStateChanged {
            number: 3,
            state_code: 2,
        }
        .encode(),
    )
    .unwrap();
    send.send(
        RoomSkillChanged {
            number: 3,
            skills: vec![11, 22],
        }
        .encode(),
    )
    .unwrap();
    paint_until_row(&mut session, |session| {
        session
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|room| room.number == 3 && room.state_code == 2 && room.skills == [11, 22])
    });
    let row = session
        .game_mut()
        .listed_rooms()
        .iter()
        .find(|room| room.number == 3)
        .expect("boot row still staged");
    assert_eq!(row.state_code, 2);
    assert_eq!(row.skills, [11, 22]);
    drop(send);
    finish(session, gateway.server);
}

#[test]
fn live_room_wire_2008_descriptor_rewrite_reaches_the_painted_board() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session ->
    // enter_room_by_clicks -> OnlineSession::paint -> poll_lobby_chat ->
    // apply_lobby_frame -> apply_room_descriptor -> rewrite_room_row ->
    // ScreenGame::upsert_room -> paint_room_list.
    if !archives_present() {
        return;
    }
    let (gateway, send) = spawn_wire_gateway();
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    let before = paint_frame(&mut session);
    let change = RoomDescriptorChanged {
        number: 3,
        title: "WireRenamed".to_owned(),
        descriptor_a: 2,
        descriptor_b: 3,
        descriptor_c: 4,
        secondary_title: "Stage".to_owned(),
        descriptor_d: 5,
        descriptor_e: 6,
        descriptor_word: 0x1234,
    };
    send.send(change.encode()).unwrap();
    paint_until_row(&mut session, |session| {
        session
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|room| room.number == 3 && room.title == "WireRenamed")
    });
    let after = paint_frame(&mut session);
    assert_ne!(before, after, "the 2008 rename changed the painted board");
    drop(send);
    finish(session, gateway.server);
}
