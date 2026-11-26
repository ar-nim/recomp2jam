#![cfg(test)]

//! Tier 3 wire legs for the Waiting answer family (#291).
//!
//! Each leg boots through the real connect entry, walks into Waiting
//! through pointer snapshots, then pushes one scripted gateway answer.
//! Missing `install-root` archives skip the leg cleanly (#0a3ba).

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use o2jam_net::frame::encode_frame;
use o2jam_net::message::{
    ExitWaitingAnswer, FreeMusicStatusAnswer, KeyModeDescriptorChanged, MusicLoaded,
    StartGameAnswer, WaitingMemberRows,
};
use o2jam_net::play::MemberReadyStateChanged;
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, JoinRoomResponse, Request, RoomList,
    WireMessage,
};
use o2jam_shell::screens::game::WaitingRoomDescriptor;

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

/// Paint until a staged wire field reaches `ready`. Counting frames,
/// not wall time, keeps a loaded fleet out of the assertion.
fn paint_until(session: &mut OnlineSession, ready: impl Fn(&mut OnlineSession) -> bool) {
    for _ in 0..MAX_SCREEN_PAINTS {
        if ready(session) {
            return;
        }
        paint(session);
    }
    panic!("live wire leg never staged the scripted fields");
}

/// Two synthetic OJNList rows: the first has level 20, the second
/// level 5. The 2035 cap is the only source that can reject the first
/// row before the auto-pick arm walks to the second.
fn two_level_catalog() -> Vec<u8> {
    [(757u32, 20u32), (758, 5)]
        .into_iter()
        .flat_map(|(song_id, level)| {
            let mut record = vec![0u8; 300];
            record[4..8].copy_from_slice(&song_id.to_le_bytes());
            record[8..11].copy_from_slice(b"ojn");
            record[20..24].copy_from_slice(&124f32.to_le_bytes());
            record[24..28].copy_from_slice(&level.to_le_bytes());
            record[112..117].copy_from_slice(b"First");
            record[176..180].copy_from_slice(b"Band");
            record[240..251].copy_from_slice(b"o2ma757.ojm");
            record
        })
        .collect()
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
        println!("SKIP live_3006_answer_returns_the_waiting_room_to_the_board_from_boot: missing install-root interface archive");
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
        println!("SKIP live_4011_answer_enters_loading_from_boot: missing install-root interface archive");
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
        println!("SKIP live_4013_stages_the_peer_loaded_state_from_boot: missing install-root interface archive");
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
        println!("SKIP live_4029_stages_counted_member_rows_from_boot: missing install-root interface archive");
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
        println!("SKIP live_4009_echo_lights_the_peer_ready_mark_from_boot: missing install-root interface archive");
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
fn live_3001_descriptor_frame_stages_the_waiting_room_descriptor_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame -> apply_key_mode_changed
    // -> stage_key_mode_descriptor -> ScreenGame::stage_waiting_descriptor
    // + ScreenGame::stage_room_member_word.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("SKIP live_3001_descriptor_frame_stages_the_waiting_room_descriptor_from_boot: missing install-root interface archive");
        return;
    }
    let gateway = spawn_waiting_wire_gateway(joined_room(0, true));
    let mut session = boot_into_waiting(&gateway);
    gateway.push(
        KeyModeDescriptorChanged {
            first_string: "WireRoom".to_owned(),
            second_string: "WireSong".to_owned(),
            flag_bytes: [1, 2, 3, 4, 5],
            member_word_flag: 0xF123,
        }
        .encode(),
    );
    paint_until(&mut session, |session| {
        session.game_mut().waiting_descriptor().first_string == "WireRoom"
    });

    let expected = WaitingRoomDescriptor {
        first_string: "WireRoom".to_owned(),
        second_string: "WireSong".to_owned(),
        flag_bytes: [1, 2, 3, 4, 5],
    };
    assert_eq!(session.game_mut().waiting_descriptor(), &expected);
    assert_eq!(
        session.game_mut().room_member_word(3),
        Some(0x000F_0123),
        "the flag word keeps the pinned high-nibble member write"
    );
    let pixels = paint_frame(&mut session);
    assert!(
        pixels.iter().any(|pixel| *pixel != 0),
        "the Waiting board painted after the descriptor frame"
    );
    finish(session, gateway);
}

#[test]
fn live_2035_cap_changes_the_auto_picked_row_from_boot() {
    // Live caller chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> OnlineSession::paint -> poll_waiting_chat
    // -> drain_waiting_chat -> apply_waiting_frame ->
    // apply_free_music_status -> ScreenGame::set_auto_pick_level_cap;
    // then a 3007 leave -> apply_auto_pick_after_seat_change ->
    // ScreenGame::auto_pick_waiting_song -> pending 4000.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("SKIP live_2035_cap_changes_the_auto_picked_row_from_boot: missing install-root interface archive");
        return;
    }
    let mut room = joined_room(0, true);
    room.music_id = 0;
    let gateway = spawn_waiting_wire_gateway(room);
    let mut session = boot_into_waiting(&gateway);
    let catalog_blob = two_level_catalog();
    let catalog = o2jam_file::parse_ojn_list(&catalog_blob).unwrap();
    session.game_mut().set_song_catalog(&catalog);

    let leave = encode_frame(3007, &[1, 0, 0, 0, 0, 0, 0, 0]);
    gateway.push(leave.clone());
    paint_until(&mut session, |session| {
        session.game_mut().room_music_id() == Some(757)
    });
    assert_eq!(
        session.game_mut().room_music_id(),
        Some(757),
        "without a ceiling the first catalog row wins"
    );
    // The 4000 echo is pinned by the seam guard; consume the outbox
    // here so its select-attempt drain cannot swallow the scripted
    // cap frame that follows.
    assert!(session.game_mut().take_music_send().is_some());

    gateway.push(
        FreeMusicStatusAnswer {
            result: 0,
            level_cap: Some(7),
        }
        .encode(),
    );
    gateway.push(leave);
    paint_until(&mut session, |session| {
        session.game_mut().room_music_id() == Some(758)
    });
    assert_eq!(
        session.game_mut().room_music_id(),
        Some(758),
        "the 2035 ceiling rejects level 20 and picks the level 5 row"
    );
    finish(session, gateway);
}
