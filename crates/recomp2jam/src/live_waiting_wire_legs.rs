#![cfg(test)]

//! Tier 3 wire legs for the Waiting answer family (#291).
//!
//! Each leg boots through the real connect entry, walks into Waiting
//! through pointer snapshots, then pushes one scripted gateway answer.
//! Missing `install-root` archives skip the leg cleanly (#0a3ba).

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use o2jam_net::message::{
    ExitWaitingAnswer, MusicLoaded, RoomDisappearedNotice, StartGameAnswer, WaitingMemberRows,
};
use o2jam_net::play::MemberReadyStateChanged;
use o2jam_net::rooms::RoomSlotUpdate;
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

/// Boot gateway plus the two command channels a wire leg needs: the
/// 3003 answer signal and the scripted S2C queue.
struct WaitingWireGateway {
    port: u16,
    join_answer_written: Receiver<()>,
    script: Sender<Vec<u8>>,
    script_written: Receiver<()>,
    server: JoinHandle<()>,
}

impl WaitingWireGateway {
    /// Push one binary wire frame and wait until the gateway wrote it.
    fn push(&self, bytes: Vec<u8>) {
        self.script.send(bytes).unwrap();
        self.script_written.recv().unwrap();
    }
}

/// One boot handshake, then a join answer and a writer thread for the
/// scripted frames. The writer is separate because a waiting client
/// sends no frames after its 3002, so a read-only gateway loop could
/// never reach a queued answer.
fn spawn_waiting_wire_gateway(room: o2jam_net::JoinedRoom) -> WaitingWireGateway {
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
    WaitingWireGateway {
        port,
        join_answer_written,
        script,
        script_written,
        server,
    }
}

/// Walk from Planet to Waiting through the staged pointer path and
/// return a session with the scripted answer already queued.
fn boot_into_waiting(gateway: &WaitingWireGateway) -> OnlineSession {
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    session
}

/// Paint until the boot walk reaches `want`; counting frames, not wall
/// time, keeps a loaded fleet out of the assertion.
fn paint_until_screen(session: &mut OnlineSession, want: o2jam_shell::Screen) {
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.screen() == want {
            return;
        }
        paint(session);
    }
    panic!("live wire leg never reached {want:?}");
}

/// Close the client and the gateway; dropping the script sender ends
/// the writer thread so `server.join()` cannot hang.
fn finish(session: OnlineSession, gateway: WaitingWireGateway) {
    drop(session);
    drop(gateway.script);
    gateway.server.join().unwrap();
}

#[test]
fn live_3006_answer_returns_the_waiting_room_to_the_board_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_exit_waiting_answer -> ScreenGame::enter_screen(Room).
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, true));
    let mut session = boot_into_waiting(&gateway);
    let waiting_frame = paint_frame(&mut session);
    gateway.push(ExitWaitingAnswer { result: 0 }.encode());
    paint_until_screen(&mut session, o2jam_shell::Screen::Room);
    let room_frame = paint_frame(&mut session);
    assert_ne!(waiting_frame, room_frame, "the Room board repainted");
    finish(session, gateway);
}

#[test]
fn live_4011_answer_enters_loading_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_start_game_answer -> poll_start_at -> Screen::Loading.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, true));
    let mut session = boot_into_waiting(&gateway);
    let waiting_frame = paint_frame(&mut session);
    gateway.push(StartGameAnswer { result: 0 }.encode());
    paint_until_screen(&mut session, o2jam_shell::Screen::Loading);
    let loading_frame = paint_frame(&mut session);
    assert_ne!(waiting_frame, loading_frame, "the Loading cover repainted");
    finish(session, gateway);
}

#[test]
fn live_4013_stages_the_peer_loaded_state_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame -> apply_music_loaded ->
    // ScreenGame::set_member_music_state -> paint_waiting_members.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, true));
    let mut session = boot_into_waiting(&gateway);
    let before = paint_frame(&mut session);
    gateway.push(
        MusicLoaded {
            member_id: 1,
            state: 4,
        }
        .encode(),
    );
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().member_music_state(1) == Some(4) {
            break;
        }
        paint(&mut session);
    }
    assert_eq!(session.game_mut().member_music_state(1), Some(4));
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the seat badge repainted"
    );
    finish(session, gateway);
}

#[test]
fn live_4029_stages_counted_member_rows_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_waiting_member_rows -> ScreenGame::set_member_music_state ->
    // paint_waiting_members.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, true));
    let mut session = boot_into_waiting(&gateway);
    let before = paint_frame(&mut session);
    gateway.push(
        WaitingMemberRows {
            rows: vec![o2jam_net::message::WaitingMemberRow {
                member_id: 1,
                value: 1,
                state: Some(3),
            }],
        }
        .encode(),
    );
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().member_music_state(1) == Some(3) {
            break;
        }
        paint(&mut session);
    }
    assert_eq!(session.game_mut().member_music_state(1), Some(3));
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the seat badge repainted"
    );
    finish(session, gateway);
}

#[test]
fn live_4009_echo_lights_the_peer_ready_mark_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame -> apply_ready_echo ->
    // ScreenGame::set_member_ready -> paint_waiting_members.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, false));
    let mut session = boot_into_waiting(&gateway);
    let before = paint_frame(&mut session);
    gateway.push(
        MemberReadyStateChanged {
            member_id: 1,
            ready: true,
        }
        .encode(),
    );
    for _ in 0..MAX_SCREEN_PAINTS {
        if session
            .game_mut()
            .staged_waiting_member(1)
            .is_some_and(|member| member.ready)
        {
            break;
        }
        paint(&mut session);
    }
    assert!(session
        .game_mut()
        .staged_waiting_member(1)
        .is_some_and(|member| member.ready));
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the ready mark repainted"
    );
    finish(session, gateway);
}

#[test]
fn live_3009_kick_shows_the_staged_member_nickname_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame -> apply_slot_update ->
    // ScreenGame::stage_player_kicked_caption -> paint_waiting ->
    // paint_information_message.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let mut room = joined_room(0, true);
    room.slots[1]
        .member
        .as_mut()
        .expect("the join answer seats the peer")
        .nickname = "KICKME".to_owned();
    let gateway = spawn_waiting_wire_gateway(room);
    let mut session = boot_into_waiting(&gateway);
    let before = paint_frame(&mut session);
    gateway.push(
        RoomSlotUpdate {
            index: 1,
            action_code: 3,
        }
        .encode(),
    );
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().waiting_wire_caption_text().is_some() {
            break;
        }
        paint(&mut session);
    }
    assert_eq!(
        session.game_mut().waiting_wire_caption_text().as_deref(),
        Some("[정보] KICKME님이 강퇴 당하셨습니다."),
        "the kick sheet names the evicted seat"
    );
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the kick sheet inked into the frame"
    );
    finish(session, gateway);
}

#[test]
fn live_3014_shows_the_room_disappeared_caption_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_room_disappeared_notice -> ScreenGame::stage_waiting_wire_caption
    // -> paint_waiting -> WaitingCaptions notice.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, true));
    let mut session = boot_into_waiting(&gateway);
    let before = paint_frame(&mut session);
    gateway.push(RoomDisappearedNotice.encode());
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().waiting_notice() == Some("공연대기실이 사라졌습니다.") {
            break;
        }
        paint(&mut session);
    }
    assert_eq!(
        session.game_mut().waiting_notice(),
        Some("공연대기실이 사라졌습니다."),
        "the pinned 3014 caption is staged"
    );
    assert!(
        session.game_mut().staged_waiting_member(1).is_none(),
        "3014 released the staged peer seat"
    );
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the notice repainted the Waiting line"
    );
    finish(session, gateway);
}

#[test]
fn live_4011_settings_mismatch_posts_the_config_error_sheet_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_start_game_answer -> start_settings_word_mismatches ->
    // ScreenGame::stage_waiting_wire_caption -> paint_waiting ->
    // paint_information_message.
    // The writer of the exe's 0x768570 settings word stays OPEN; this
    // leg stages a mismatching value before the wire answer.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, true));
    let mut session = boot_into_waiting(&gateway);
    session.game_mut().set_room_settings_word(Some(999));
    let before = paint_frame(&mut session);
    gateway.push(StartGameAnswer { result: 0 }.encode());
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().waiting_notice().is_some() {
            break;
        }
        paint(&mut session);
    }
    assert_eq!(
        session.game_mut().waiting_notice(),
        Some("게임설정 조작 오류로 게임을 중단합니다.."),
        "the fifth 4011 caption is the local settings mismatch"
    );
    assert_eq!(session.screen(), o2jam_shell::Screen::Waiting);
    assert_ne!(
        before,
        paint_frame(&mut session),
        "the refusal sheet inked into the frame"
    );
    finish(session, gateway);
}
