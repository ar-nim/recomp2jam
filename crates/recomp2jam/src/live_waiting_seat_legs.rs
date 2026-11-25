#![cfg(test)]

//! Tier 3 live legs for Waiting seats, avatars and social chrome (#291).
//!
//! Each leg boots through the real connect entry with a scripted 3003
//! seat snapshot, enters Waiting through pointer snapshots, then reads
//! the staged seats and pixels the shell paints from that snapshot.
//! Missing installed archives skip every leg cleanly (CI has no data).

use std::io::Write;
use std::sync::mpsc;

use o2jam_net::frame::encode_frame;
use o2jam_net::rooms::WaitingUserMessage;
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, DownloadProgressChanged, JoinRoomResponse,
    JoinedRoom, MusicState, Request, RoomList, RoomMember, RoomSlot, RoomSlotState,
    SyncMemberMusicStateEvent, WireMessage,
};
use o2jam_render::{Frame, Rect, Rgb555Image, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::avatar::{
    AvatarFigure, FigureLayer, AVATAR_BREATHING_FRAME_COUNT, AVATAR_BREATHING_TICK_MS,
};
use o2jam_shell::screens::game::StartGateVerdict;
use o2jam_shell::{GameLoop, InputSnapshot, Screen};

use crate::live_leg_support::*;
use crate::session::OnlineSession;

/// A scripted 3003 snapshot: local member and one peer in a known seat.
fn scripted_room(member_id: u8, peer_slot: usize) -> JoinedRoom {
    let slots = (0..7u8)
        .map(|index| RoomSlot {
            index,
            state_code: RoomSlotState::Unoccupied as i32,
            member: None,
        })
        .collect();
    let mut room = JoinedRoom {
        member_id,
        team: 0,
        title: "Lobby".to_owned(),
        music_id: 757,
        arena: 1,
        key_mode: 2,
        game_mode: 0,
        difficulty: 0,
        speed: 1,
        has_super_manager: false,
        user_count: 2,
        slots,
        skills: Vec::new(),
        premium: false,
        champion: None,
        champion_streak: None,
    };
    let mut peer = peer_member();
    // The slot-0 creator is the master on both sides of the wire.
    if peer_slot == 0 {
        peer.master = true;
    }
    room.slots[peer_slot] = RoomSlot {
        index: u8::try_from(peer_slot).expect("peer seat below eight"),
        state_code: RoomSlotState::Occupied as i32,
        member: Some(peer),
    };
    let local_slot = usize::from(member_id);
    room.slots[local_slot] = RoomSlot {
        index: member_id,
        state_code: RoomSlotState::Occupied as i32,
        member: Some(local_member(member_id == 0)),
    };
    room
}

fn local_member(master: bool) -> RoomMember {
    RoomMember {
        nickname: "HOST".to_owned(),
        level: 12,
        gender: 0,
        gem: 900,
        master,
        team: 0,
        ready: true,
        music_state: MusicState::Ready.code(),
        equipment: [0; 17],
        music_ids: vec![757],
        cash_point: 1_000,
        free_pass: 0,
        playing_state: 0,
        administrator: false,
        super_manager: false,
    }
}

fn peer_member() -> RoomMember {
    RoomMember {
        nickname: "PEER".to_owned(),
        level: 42,
        gender: 1,
        gem: 1_234,
        master: false,
        team: 1,
        ready: true,
        music_state: MusicState::Ready.code(),
        equipment: [0; 17],
        music_ids: vec![101, 202],
        cash_point: 5_678,
        free_pass: 0,
        playing_state: 0,
        administrator: false,
        super_manager: false,
    }
}

/// Boot gateway that replays the join and, when present, pushes chat
/// rows on the same socket. The rows arrive on the wire, not through a
/// game seam.
fn spawn_seat_gateway(
    room: JoinedRoom,
    chat_rows: Vec<WaitingUserMessage>,
    event_frames: Vec<Vec<u8>>,
) -> LiveLegGateway {
    let (reported, reports) = mpsc::channel();
    let (join_written_tx, join_answer_written) = mpsc::channel();
    let (port, server) = fake_gateway(move |mut stream| {
        assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
        stream
            .write_all(
                &AuthResponse {
                    result_code: 0,
                    tail: vec![],
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
                for row in &chat_rows {
                    stream.write_all(&row.encode()).unwrap();
                }
                for event in &event_frames {
                    stream.write_all(event).unwrap();
                }
                join_written_tx.send(()).unwrap();
            }
            reported.send(frame).unwrap();
        }
    });
    LiveLegGateway {
        port,
        reports,
        join_answer_written,
        server,
    }
}

fn boot_seat_leg(
    room: JoinedRoom,
    chat_rows: Vec<WaitingUserMessage>,
) -> (OnlineSession, LiveLegGateway) {
    let gateway = spawn_seat_gateway(room, chat_rows, Vec::new());
    let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    assert_eq!(session.screen(), Screen::Waiting);
    (session, gateway)
}

/// One staged Waiting placement's absolute native rect.
fn staged_rect(session: &mut OnlineSession, id: &str) -> Rect {
    session
        .game_mut()
        .scene_placements(Screen::Waiting)
        .iter()
        .find(|placed| placed.id == id)
        .unwrap_or_else(|| panic!("Waiting does not stage {id}"))
        .rect
}

fn staged_code_rect(session: &mut OnlineSession, code: u32) -> Rect {
    session
        .game_mut()
        .scene_placements(Screen::Waiting)
        .iter()
        .find(|placed| placed.code == code)
        .unwrap_or_else(|| panic!("Waiting does not stage {code:#x}"))
        .rect
}

/// Paint the game at an explicit clock, so a cadence leg needs no sleep.
fn paint_at_ms(session: &mut OnlineSession, now_ms: u64) -> Vec<u32> {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
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

fn ink_in_rect(frame: &[u32], rect: Rect) -> Vec<u32> {
    staged_rect_pixels_from(frame, rect)
}

/// A 12-record breathing body: each frame inks a different column.
fn breathing_figure() -> AvatarFigure {
    let mut figure = AvatarFigure::default();
    for frame_index in 0..AVATAR_BREATHING_FRAME_COUNT {
        let ink = u16::try_from(0x0400 + frame_index).expect("breathing ink fits RGB555");
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&ink.to_le_bytes(), 1, 1).expect("one ink pixel decodes"),
            dx: i32::try_from(frame_index).expect("frame index fits a pixel offset"),
            dy: 0,
            colorkey: 0,
        });
    }
    figure
}

/// A one-image body: the exe's idle sleeves hold frame 0 forever.
fn still_figure() -> AvatarFigure {
    let mut figure = AvatarFigure::default();
    let ink = 0x07E0u16;
    figure.push_layer(FigureLayer {
        image: Rgb555Image::decode(&ink.to_le_bytes(), 1, 1).expect("one ink pixel decodes"),
        dx: 0,
        dy: 0,
        colorkey: 0,
    });
    figure
}

/// #991: the painted seat body advances one frame per pinned 66 ms
/// tick, while a one-image idle part holds its pose.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> GameLoop::ingest_snapshot (join)
///   -> GameLoop::idle_repaint_interval (66 ms pulse)
///   -> ScreenGame::paint_at -> paint_waiting_scene
///   -> paint_waiting_seat_avatars -> AvatarFigure::paint_visible_centered.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_waiting_avatars_advance_one_breathing_frame_per_pinned_tick() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_seat_leg(scripted_room(0, 1), Vec::new());
    session.game_mut().set_waiting_avatar(1, breathing_figure());
    session.game_mut().set_waiting_avatar(0, still_figure());
    assert_eq!(
        session.idle_repaint_interval(),
        Some(std::time::Duration::from_millis(AVATAR_BREATHING_TICK_MS)),
        "a staged seat figure owes one repaint pulse per breathing tick"
    );
    let breathing_cover = staged_rect(&mut session, "IDC_IMAGE_SWAIT_COVERAVATAR2");
    let still_cover = staged_rect(&mut session, "IDC_IMAGE_SWAIT_COVERAVATAR1");
    let rest = paint_at_ms(&mut session, 0);
    let one_tick = paint_at_ms(&mut session, AVATAR_BREATHING_TICK_MS);
    let two_ticks = paint_at_ms(&mut session, 2 * AVATAR_BREATHING_TICK_MS);
    assert_ne!(
        ink_in_rect(&rest, breathing_cover),
        ink_in_rect(&one_tick, breathing_cover),
        "one 66 ms tick must repaint the breathing body"
    );
    assert_ne!(
        ink_in_rect(&one_tick, breathing_cover),
        ink_in_rect(&two_ticks, breathing_cover),
        "the next tick must show the next breathing frame"
    );
    assert_eq!(
        ink_in_rect(&rest, still_cover),
        ink_in_rect(&two_ticks, still_cover),
        "a one-image idle part holds its pose across ticks"
    );
    finish(session, gateway.server);
}

/// #999: the playing-state byte from 3003 lands on the staged seat and
/// switches the seat's `Wait_Playing.ojs` mark.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> ScreenGame::staged_waiting_member
///   -> ScreenGame::paint_at -> paint_waiting_scene_with_confirm
///   -> paint_waiting_seat_marks (PLAYING).
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_waiting_playing_state_from_3003_inks_the_seat_mark() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let mut playing_room = scripted_room(0, 1);
    playing_room.slots[1].member.as_mut().unwrap().playing_state = 1;
    let (mut playing, gateway) = boot_seat_leg(playing_room, Vec::new());
    assert_eq!(
        playing
            .game_mut()
            .staged_waiting_member(1)
            .unwrap()
            .playing_state,
        1,
        "the wire playing byte reaches the staged peer seat"
    );
    let peer_cover = staged_rect(&mut playing, "IDC_IMAGE_SWAIT_COVERAVATAR2");
    let playing_frame = paint_frame(&mut playing);
    finish(playing, gateway.server);

    let (mut idle, idle_gateway) = boot_seat_leg(scripted_room(0, 1), Vec::new());
    let idle_frame = paint_frame(&mut idle);
    assert_ne!(
        ink_in_rect(&playing_frame, peer_cover),
        ink_in_rect(&idle_frame, peer_cover),
        "a peer in game must change the staged seat mark"
    );
    finish(idle, idle_gateway.server);
}

/// #997/#998: the slot-derived group pair marks the host and the
/// non-host covers, and the group word follows the pinned 1/2/3 rule.
/// The pair is a derivation from the wire seat index, not a wire field;
/// the live leg pins what reaches the staged roster and its marks.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> staged_waiting_member
///   -> ScreenGame::paint_at -> paint_waiting_seat_marks (MASTER/READY).
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_waiting_slot_group_marks_the_master_and_non_host_seats() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let mut room = scripted_room(0, 3);
    room.slots[3].member.as_mut().unwrap().ready = false;
    let (mut session, gateway) = boot_seat_leg(room, Vec::new());
    let host = session.game_mut().staged_waiting_member(0).unwrap().clone();
    let peer = session.game_mut().staged_waiting_member(3).unwrap().clone();
    assert_eq!(o2jam_net::relay::slot_position_marks(0), (1, true));
    assert_eq!(o2jam_net::relay::slot_position_marks(3), (2, false));
    assert_eq!(o2jam_net::relay::slot_position_marks(1), (3, true));
    assert!(host.master, "slot 0 is the master's derived group");
    assert!(!peer.master && !peer.ready, "slot 3 carries the other pair");
    let master_cover = staged_code_rect(&mut session, 0x1650_003D);
    let peer_cover = staged_code_rect(&mut session, 0x1650_0040);
    let frame = paint_frame(&mut session);
    assert!(!ink_in_rect(&frame, master_cover).is_empty());
    assert!(!ink_in_rect(&frame, peer_cover).is_empty());
    finish(session, gateway.server);
}

/// #995: the owned-song ids decoded from 3003 reach the staged peer
/// seat. The picker's own filter remains OPEN: no shell consumer of
/// `RoomMember.music_ids` exists yet, so this leg cannot close the
/// player-visible song-picker claim.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> staged_waiting_member.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_waiting_member_music_ids_reach_the_staged_seat() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_seat_leg(scripted_room(0, 1), Vec::new());
    let peer = session.game_mut().staged_waiting_member(1).unwrap();
    assert_eq!(peer.music_ids, vec![101u16, 202]);
    let _frame = paint_frame(&mut session);
    finish(session, gateway.server);
}

/// #992: chat rows, team values and the master emblem stage on both
/// sides from the wire. Team presses take the staged rect and queue
/// 4004/4008 on the live socket.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003/3012)
///   -> ScreenGame::push_waiting_user_message -> paint_waiting_chat
///   -> paint_waiting_seat_marks (MASTER/TEAM); a staged TEAMB click
///   -> ScreenGame::click_waiting_button -> flush_team_send -> 4004/4008.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_waiting_chat_team_and_master_emblem_stage_on_both_sides() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let chat = vec![
        WaitingUserMessage {
            sender: "host".to_owned(),
            content: b"host row".to_vec(),
            member_id: 0,
        },
        WaitingUserMessage {
            sender: "peer".to_owned(),
            content: b"peer row".to_vec(),
            member_id: 1,
        },
    ];
    for member_id in [0u8, 1u8] {
        let peer_slot = if member_id == 0 { 1 } else { 0 };
        let peer_id = u8::try_from(peer_slot).expect("peer seat below eight");
        let (mut session, gateway) =
            boot_seat_leg(scripted_room(member_id, peer_slot), chat.clone());
        for _ in 0..MAX_SCREEN_PAINTS {
            if session.game_mut().waiting_chat().len() == 2 {
                break;
            }
            paint(&mut session);
        }
        assert_eq!(session.game_mut().waiting_chat().len(), 2);
        assert!(
            session.game_mut().staged_waiting_member(0).unwrap().master,
            "both sides read the wire master emblem seat"
        );
        assert_eq!(
            session
                .game_mut()
                .staged_waiting_member(peer_id)
                .unwrap()
                .team,
            1,
            "both sides read the wire team"
        );
        let chat_rect = staged_rect(&mut session, "IDC_LIST_SWAIT_CHAT");
        let mut boss_marks: Vec<Rect> = session
            .game_mut()
            .scene_placements(Screen::Waiting)
            .iter()
            .filter(|placed| placed.code == 0x1650_002F)
            .map(|placed| placed.rect)
            .collect();
        boss_marks.sort_by_key(|rect| (rect.y, rect.x));
        assert!(
            boss_marks.len() >= 2,
            "the seat template stages a boss mark per cover"
        );
        let frame = paint_frame(&mut session);
        assert!(!ink_in_rect(&frame, chat_rect).is_empty());
        let master_ink = ink_pixels(&ink_in_rect(&frame, boss_marks[0]));
        let guest_ink = ink_pixels(&ink_in_rect(&frame, boss_marks[1]));
        assert!(
            master_ink > guest_ink,
            "the wire master emblem inks the creator seat on both sides"
        );
        finish(session, gateway.server);
    }
}

/// #992: a staged TEAM press reaches the live socket in exe order.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_waiting_team_press_sends_4004_then_4008_from_boot() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_seat_leg(scripted_room(1, 0), Vec::new());
    let (x, y) = staged_code_centre(&mut session, Screen::Waiting, 0x1620_1005);
    session.ingest_snapshot(&InputSnapshot::click_at(x, y));
    paint(&mut session);
    let _team = wait_for_opcode(&gateway.reports, 4004);
    let _ready = wait_for_opcode(&gateway.reports, 4008);
    finish(session, gateway.server);
}

/// #993: a 4039 percentage on the wire is cleared when 4037 reports
/// Ready, so the START gate opens with no stale progress. The 5012 send
/// site and the 4026 broadcast remain OPEN (music-availability §5.3).
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (4039/4037)
///   -> ScreenGame::set_member_music_state -> start_gate_verdict.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_waiting_ready_answer_clears_the_4039_progress_and_opens_start() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let mut room = scripted_room(0, 1);
    room.slots[1].member.as_mut().unwrap().music_state = MusicState::Downloading.code();
    let progress = DownloadProgressChanged {
        percentage: 42,
        member_id: 1,
    };
    let ready = SyncMemberMusicStateEvent {
        member_id: 1,
        state: MusicState::Ready,
    };
    let gateway = spawn_seat_gateway(room, Vec::new(), vec![progress.encode(), ready.encode()]);
    let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.game_mut().member_music_state(1) == Some(MusicState::Ready.code()) {
            break;
        }
        paint(&mut session);
    }
    assert_eq!(
        session.game_mut().member_download_progress(1),
        None,
        "4037 Ready clears the 4039 percentage"
    );
    assert_eq!(
        session.game_mut().member_music_state(1),
        Some(MusicState::Ready.code())
    );
    assert_eq!(
        session.game_mut().start_gate_verdict(),
        StartGateVerdict::Startable
    );
    finish(session, gateway.server);
}

/// #994 Tier 3 caption capture: two 3010 causes must ink different
/// captions in the staged Waiting notice line after a real boot.
///
/// Live caller chain (once the missing handler lands):
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3010)
///   -> ScreenGame::waiting_notice -> paint_waiting_scene_with_confirm
///   -> paint_waiting_staged (notice over the room name plate).
///
/// BLOCKED: the 3010 caption arm does not exist, so both causes leave
/// the same pixels. Ignored until the senior re-slices the
/// shell/session caption path; run with `--ignored` to see the red.
#[test]
#[ignore = "#994: shell/session 3010 removal-caption arm outside this lane's files"]
fn live_waiting_removal_cause_captures_different_notice_pixels() {
    if !archives_present() {
        println!("skip: missing install-root interface archive");
        return;
    }
    let mut captions = Vec::new();
    for cause in [0u8, 1u8] {
        let gateway = spawn_seat_gateway(
            scripted_room(0, 1),
            Vec::new(),
            vec![encode_frame(3010, &[cause])],
        );
        let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        for _ in 0..MAX_SCREEN_PAINTS {
            if session.game_mut().waiting_notice().is_some() {
                break;
            }
            paint(&mut session);
        }
        let notice = staged_rect(&mut session, "IDC_TEXT_SWAIT_ROOMNAME");
        captions.push(ink_in_rect(&paint_frame(&mut session), notice));
        finish(session, gateway.server);
    }
    assert_ne!(
        captions[0], captions[1],
        "the two 3010 causes must paint different removal captions"
    );
}
