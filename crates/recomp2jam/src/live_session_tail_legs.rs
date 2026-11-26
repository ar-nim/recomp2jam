#![cfg(test)]

//! Tier 3 wire legs for the Waiting/Room session tail (stories #956,
//! #962, #1015, #1017, #1018). Each leg boots through the real connect
//! entry, walks into Waiting through staged pointer snapshots, then
//! pushes one scripted gateway answer. Missing `install-root`
//! archives skip the leg cleanly (CI has no game data, #0a3ba).

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use o2jam_net::message::MemberStateChanged;
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
    WireMessage,
};
use o2jam_shell::screens::game::JoinAnswerRefusal;
use o2jam_shell::{GameLoop, InputSnapshot, Screen};

use crate::live_leg_support::{
    archives_present, boot_live_leg, channel_row, click_staged, dismiss_splash,
    enter_room_by_clicks, information_ok_centre, join_waiting_by_click, joined_room, paint,
    paint_frame, read_frame_until_eof, room_row, MAX_SCREEN_PAINTS,
};
use crate::session::OnlineSession;

/// Boot gateway with a separate writer for scripted S2C frames. The
/// Waiting client sends no frame after its 3002, so a read-only loop
/// could never reach a queued answer.
struct TailWireGateway {
    port: u16,
    join_answer_written: Receiver<()>,
    script: Sender<Vec<u8>>,
    script_written: Receiver<()>,
    server: JoinHandle<()>,
}

impl TailWireGateway {
    /// Push one whole wire frame and wait for the gateway to write it.
    fn push(&self, bytes: Vec<u8>) {
        self.script.send(bytes).unwrap();
        self.script_written.recv().unwrap();
    }
}

fn spawn_tail_wire_gateway(room: o2jam_net::JoinedRoom) -> TailWireGateway {
    spawn_tail_wire_gateway_with_join_refusal(room, None)
}

/// Every join answers with `refusal`; room entry itself sends no
/// 3002, so the first join is the staged row press.
fn spawn_tail_wire_gateway_with_join_refusal(
    room: o2jam_net::JoinedRoom,
    refusal: Option<i32>,
) -> TailWireGateway {
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
                let answer = match refusal {
                    Some(code) => JoinRoomResponse {
                        result_code: u32::from_le_bytes(code.to_le_bytes()),
                        room: None,
                    },
                    None => JoinRoomResponse {
                        result_code: 0,
                        room: Some(room.clone()),
                    },
                };
                stream.write_all(&answer.encode()).unwrap();
                join_tx.send(()).unwrap();
            }
        }
        writer.join().unwrap();
    });
    TailWireGateway {
        port,
        join_answer_written,
        script,
        script_written,
        server,
    }
}

fn boot_waiting_leg(gateway: &TailWireGateway) -> OnlineSession {
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    session
}

fn finish(session: OnlineSession, gateway: TailWireGateway) {
    drop(session);
    drop(gateway.script);
    gateway.server.join().unwrap();
}

/// Boot into Room through pointer snapshots; room entry sends no
/// 3002 here, so the first staged row press owns the refusal.
fn boot_room_before_refused_row(port: u16) -> OnlineSession {
    let mut session = boot_live_leg(port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    session
}

fn changed_pixels(before: &[u32], after: &[u32]) -> usize {
    assert_eq!(before.len(), after.len());
    before
        .iter()
        .zip(after)
        .filter(|(left, right)| left != right)
        .count()
}

fn press_refused_row_and_await_answer(session: &mut OnlineSession, gateway: &TailWireGateway) {
    click_staged(session, Screen::Room, "IDC_BUTTON_SROOM_EXBUTTON1");
    paint(session);
    gateway
        .join_answer_written
        .recv_timeout(Duration::from_secs(5))
        .expect("the row press was answered by a refusal");
    paint(session);
}

/// #1018 Tier 3, first half of the popup pair. Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport ->
///   boot_live_leg -> dismiss_splash -> enter_room_by_clicks ->
///   staged row click -> OnlineSession::poll_attempts -> drain_room_join ->
///   3003 result -6 -> ScreenGame::stage_join_answer_refusal ->
///   paint_room -> paint_information_message.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_3003_refusal_posts_the_pinned_full_room_sheet_from_boot() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_tail_wire_gateway_with_join_refusal(joined_room(0, true), Some(-6));
    let mut session = boot_room_before_refused_row(gateway.port);
    let before = paint_frame(&mut session);
    press_refused_row_and_await_answer(&mut session, &gateway);

    assert_eq!(session.screen(), Screen::Room, "a refusal stays in Room");
    assert!(
        session.game_mut().dialog_owns_input(),
        "the 3003 refusal owns the next click"
    );
    assert_eq!(
        session.game_mut().join_answer_refusal(),
        Some(JoinAnswerRefusal::RoomFull),
        "the pinned room-full caption is staged"
    );
    assert!(
        changed_pixels(&before, &paint_frame(&mut session)) > 0,
        "the refusal sheet paints over the Room board"
    );
    finish(session, gateway);
}

/// #1018 Tier 3, second half of the popup pair. Live caller chain:
///   main.rs::run_play -> ... -> 3003 result -6 ->
///   ScreenGame::stage_join_answer_refusal -> ScreenGame::click_room ->
///   information_ok_hit -> ChatState::clear_join_answer_refusal.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_3003_refusal_ok_dismisses_the_sheet_without_a_new_wire_frame() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_tail_wire_gateway_with_join_refusal(joined_room(0, true), Some(-6));
    let mut session = boot_room_before_refused_row(gateway.port);
    press_refused_row_and_await_answer(&mut session, &gateway);
    assert!(
        session.game_mut().dialog_owns_input(),
        "the sheet is up before OK"
    );
    let before_ok = paint_frame(&mut session);
    let (ok_x, ok_y) = information_ok_centre();
    session.ingest_snapshot(&InputSnapshot::click_at(ok_x, ok_y));

    assert!(
        !session.game_mut().dialog_owns_input(),
        "OK hands input back to the Room board"
    );
    assert_eq!(
        session.game_mut().join_answer_refusal(),
        None,
        "OK clears the staged caption"
    );
    assert!(
        changed_pixels(&before_ok, &paint_frame(&mut session)) > 0,
        "OK removes the painted sheet"
    );
    assert!(
        gateway.join_answer_written.try_recv().is_err(),
        "the dismissal posts no further join"
    );
    finish(session, gateway);
}

#[test]
fn live_4026_three_byte_state_marks_the_peer_seat_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> staged pointer walk -> OnlineSession::paint -> poll_wire_at ->
    // poll_waiting_chat -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_member_state_changed -> ScreenGame::set_member_music_state +
    // ScreenGame::seat_waiting_member -> paint_waiting_seat_marks.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_tail_wire_gateway(joined_room(0, true));
    let mut session = boot_waiting_leg(&gateway);
    let before = paint_frame(&mut session);
    gateway.push(
        MemberStateChanged {
            member_id: 1,
            playing_state: 1,
            music_state: 4,
        }
        .encode(),
    );
    for _ in 0..MAX_SCREEN_PAINTS {
        if session
            .game_mut()
            .staged_waiting_member(1)
            .is_some_and(|member| member.playing_state == 1 && member.music_state == 4)
        {
            break;
        }
        paint(&mut session);
    }
    let staged = session.game_mut().staged_waiting_member(1).unwrap();
    assert_eq!(staged.playing_state, 1, "the PLAYING mark's wire word");
    assert_eq!(staged.music_state, 4, "the DOWNLOADING badge's wire word");
    assert_ne!(before, paint_frame(&mut session), "the seat repainted");
    finish(session, gateway);
}
