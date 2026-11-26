#![cfg(test)]

//! Tier 3 wire legs for the Playing/battle family (#293).
//!
//! Each leg boots through the real connect entry, walks to Waiting
//! through pointer snapshots, pushes one scripted gateway frame, and
//! observes the painted Waiting board. Missing `install-root` archives
//! skip the leg cleanly (CI has no game data, #0a3ba). The two-client
//! live-battle Playing legs remain OPEN; this file does not fake them.

use std::collections::HashMap;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use o2jam_audio::NullBackend;
use o2jam_net::frame::encode_frame;
use o2jam_net::relay::{P2PListResponse, PeerInfo};
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, Frame, JoinRoomResponse, Request, RoomList,
    SetRoomMusic, WireMessage,
};
use o2jam_shell::screens::messenger::{
    UhpPeerTables, UhpStartSequence, UHP_REPORT_OPCODE, UHP_SUCCESS_CAPTION,
};
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

use crate::live_leg_support::{
    archives_present, boot_live_leg, channel_row, dismiss_splash, enter_room_by_clicks,
    join_waiting_by_click, joined_room, paint, paint_frame, paint_until_screen,
    read_frame_until_eof, room_row, MAX_SCREEN_PAINTS,
};
use crate::loading::{loading_phase_status, LOADING_GATE_STEPS};
use crate::session::OnlineSession;

/// Boot gateway plus the scripted S2C writer a live wire leg needs.
struct PlayingWireGateway {
    port: u16,
    join_answer_written: Receiver<()>,
    script: Sender<Vec<u8>>,
    script_written: Receiver<()>,
    reports: Receiver<Frame>,
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
    let (reported_tx, reports) = mpsc::channel();
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
            if frame.id == Request::StartGame.id() {
                stream.write_all(&encode_frame(4011, &[])).unwrap();
            }
            if frame.id == Request::ConfirmMusicLoaded.id() {
                stream.write_all(&encode_frame(4013, &[1, 1])).unwrap();
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
        reports,
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

/// Arm the Loading gate the same way the session tests do: a
/// validated pick, the armed status caption, and a null audio rig.
fn stage_armed_loading(game: &mut ScreenGame) {
    game.set_selected_music(SetRoomMusic {
        music_id: 757,
        difficulty: 0,
        speed: 0,
    });
    game.with_audio(Box::new(NullBackend::new()), HashMap::new());
    if let Some(status) = loading_phase_status(LOADING_GATE_STEPS, "Song", "Band") {
        game.set_loading_status(status);
    }
}

/// Walk the boot gateway from Planet to the Playing screen through
/// the staged Waiting and Loading confirms. The same scripted answers
/// as the session tests drive the 4011/4013 handoff.
fn enter_playing_wire_from_boot(gateway: &PlayingWireGateway) -> OnlineSession {
    let mut session = boot_into_playing_wire(gateway);
    session.menu_key(MenuKey::Confirm, true);
    paint_until_screen(&mut session, Screen::Loading);
    stage_armed_loading(session.game_mut());
    session.menu_key(MenuKey::Confirm, true);
    paint_until_screen(&mut session, Screen::Playing);
    session
}

/// Close the socket first so the writer thread can end before join.
fn finish(session: OnlineSession, gateway: PlayingWireGateway) {
    drop(session);
    drop(gateway.script);
    gateway.server.join().unwrap();
}

#[test]
fn live_4015_rank_exchange_seats_both_member_places_from_boot() {
    // Live chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> Waiting Confirm -> Loading Confirm ->
    // OnlineSession::paint -> poll_wire_at -> poll_stats ->
    // drain_peer_echoes -> apply_4015_frame -> apply_rank_exchange ->
    // ScreenGame::apply_rank_exchange -> PeerBars::apply_rank_broadcast.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("SKIP live_4015_rank_exchange_seats_both_member_places_from_boot: missing install-root interface archive");
        return;
    }
    let gateway = spawn_playing_wire_gateway(joined_room(0, true));
    let mut session = enter_playing_wire_from_boot(&gateway);
    assert_eq!(session.screen(), Screen::Playing);
    gateway.push(encode_frame(
        4015,
        &[1u8, 1, 0, 1, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ));
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().live_play().peers().rank_row(1) == Some(0)
            && session.game_mut().live_play().peers().rank_row(0) == Some(1)
        {
            break;
        }
        paint(&mut session);
    }
    assert_eq!(
        session.game_mut().live_play().peers().rank_row(1),
        Some(0),
        "member 1 must hold the zero-based place 0 after the boot exchange"
    );
    assert_eq!(
        session.game_mut().live_play().peers().rank_row(0),
        Some(1),
        "member 0 must hold the zero-based place 1 after the boot exchange"
    );
    finish(session, gateway);
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
        println!("SKIP live_4035_answer_arms_the_local_download_badge_from_boot: missing install-root interface archive");
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

/// Paint the Playing board at a fixed clock so a caption, not the
/// song animation, is the only possible frame delta.
fn paint_fixed_caption_frame(session: &mut OnlineSession) -> Vec<u32> {
    let mut buf = vec![0u32; 800 * 600];
    session.game_mut().paint_at(
        &mut o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        },
        0,
    );
    buf
}

#[test]
fn live_uhp_start_after_4045_inks_the_caption_and_sends_one_empty_4046_from_boot() {
    // Live chain: main.rs::run_play ->
    // connect::connect_and_login_with_transport -> assemble_online_session
    // -> boot pointer walk -> Waiting/Loading Confirm ->
    // OnlineSession::paint -> poll_wire_at -> poll_stats ->
    // drain_peer_echoes -> 4045 arm -> UhpPeerTables::from_peer_list ->
    // UhpStartSequence::run -> ScreenGame::post_uhp_start_caption ->
    // ScreenGame::paint_at -> paint_myroom_information; the same run
    // sends one empty 4046 through OnlineSession::send_uhp_report.
    // seam: no real UDP packet or worker thread exists; the injected
    // readiness script stands in for the exe's battle poller.
    // Archives absent -> skip cleanly (CI has no game data).
    if !archives_present() {
        println!("SKIP live_uhp_start_after_4045_inks_the_caption_and_sends_one_empty_4046_from_boot: missing install-root interface archive");
        return;
    }
    let gateway = spawn_playing_wire_gateway(joined_room(0, true));
    let mut session = enter_playing_wire_from_boot(&gateway);
    assert_eq!(session.screen(), Screen::Playing);
    let response = P2PListResponse {
        peers: vec![PeerInfo {
            member_id: 3,
            public_endpoint: SocketAddrV4::new(Ipv4Addr::new(203, 0, 113, 6), 4002),
            local_endpoint: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 6), 4003),
        }],
    };
    gateway.push(response.encode());
    paint_frame(&mut session);
    let before = paint_fixed_caption_frame(&mut session);

    let tables = UhpPeerTables::from_peer_list(&response);
    let trace = {
        let mut sequence = UhpStartSequence::new(vec![false, true], |_| {});
        sequence.run(&tables, &mut session)
    };
    let after = paint_fixed_caption_frame(&mut session);

    assert_eq!(trace.caption, UHP_SUCCESS_CAPTION);
    assert_eq!(
        session.game_mut().uhp_start_caption(),
        Some(UHP_SUCCESS_CAPTION)
    );
    assert!(session.game_mut().uhp_start_completed());
    assert_ne!(before, after, "the UHP caption must ink the Playing board");

    let report = loop {
        let frame = gateway
            .reports
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        if frame.id == UHP_REPORT_OPCODE {
            break frame;
        }
    };
    assert!(report.payload.is_empty(), "4046 must carry an empty body");
    assert!(
        gateway
            .reports
            .try_iter()
            .all(|frame| frame.id != UHP_REPORT_OPCODE),
        "one UHP start sends exactly one 4046"
    );
    finish(session, gateway);
}
