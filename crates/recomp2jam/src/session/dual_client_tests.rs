//! Tier-2 two-client guards on one fake Memoryer listener.
//!
//! Both clients reach their scene through the real connect/assemble path;
//! every event under test arrives on a socket. `super::*` reaches the
//! session's private clocked paint seam for the Loading gate (#592).

use std::time::Instant;

use o2jam_net::frame::encode_frame;
use o2jam_net::message::MemberStateChanged;
use o2jam_net::rooms::SendWhisper;
use o2jam_net::{
    AcquireMusic, MemberMusicState, MusicState, ReceiveWhisper, RoomRemoved, RoomSlotAction,
    RoomSlotUpdate, SyncMemberMusicStateEvent, WireMessage,
};
use o2jam_render::Rect;
use o2jam_shell::screens::game::StartGateVerdict;
use o2jam_shell::Keymap;
use winit::keyboard::KeyCode;

use super::*;
use crate::live_dual_client_legs::harness::{
    boot_room_client, boot_waiting_client, paint_until, Direction, DualGateway, WireRecord,
};
use crate::live_leg_support::{
    archives_present, joined_room, paint_frame, route_raw_key, staged_rect_pixels,
};
use crate::loading::{LoadingGateArm, LOADING_CONFIRM_AFTER_START, LOADING_PLAYING_AFTER_CONFIRM};

/// Shared identifier for the two wire legs in #592.
pub(crate) const LOADING_TRACE_ID: &str = "dual-loading-4010-4011-4012";

/// Harness-local ordinal recorded with the trace id. It is test metadata,
/// not a field read from the 7.55 client.
pub(crate) const LOADING_SCENARIO_EPOCH: u64 = 1;

/// One clocked paint on the private session seam, without wall sleeps.
fn paint_loading_frame_at(session: &mut OnlineSession, now: Instant) {
    let mut buf = vec![0u32; 800 * 600];
    session.paint_at_instant(
        &mut o2jam_render::Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        },
        now,
    );
}

/// Count one direction/opcode pair in a client's wire leg.
fn count(log: &[WireRecord], direction: Direction, id: u16) -> usize {
    log.iter()
        .filter(|record| record.direction == direction && record.id == id)
        .count()
}

/// First record of one direction/opcode pair in a client's wire leg.
fn find(log: &[WireRecord], direction: Direction, id: u16) -> &WireRecord {
    log.iter()
        .find(|record| record.direction == direction && record.id == id)
        .unwrap_or_else(|| panic!("no {direction:?} {id} record in the wire leg"))
}

/// Every record in both legs must carry the one scenario trace id.
fn assert_shared_trace(legs: &[Vec<WireRecord>], trace_id: &str) {
    for (index, log) in legs.iter().enumerate() {
        assert!(
            log.iter().all(|record| record.trace_id == trace_id),
            "client {index} has records outside trace {trace_id}"
        );
    }
}

/// Native pixels that differ between two painted frames.
fn changed_pixels(before: &[u32], after: &[u32]) -> usize {
    before
        .iter()
        .zip(after)
        .filter(|(was, now)| was != now)
        .count()
}

/// Absolute staged rect of one Waiting seat's DOWNLOADING sprite.
fn waiting_badge_rect(session: &mut OnlineSession, seat: u8) -> Rect {
    let parent_id = format!("IDC_BUTTON_SWAIT_EXTEND{}", seat + 1);
    session
        .game_mut()
        .scene_placements(Screen::Waiting)
        .iter()
        .find(|placed| {
            placed.id == parent_id && placed.sprite.as_deref() == Some("Wait_Downloding.ojs")
        })
        .map(|placed| placed.rect)
        .unwrap_or_else(|| panic!("no staged DOWNLOADING sprite for {parent_id}"))
}

/// Absolute staged rect of the Room chat list.
fn room_list_rect(session: &mut OnlineSession) -> Rect {
    session
        .game_mut()
        .scene_placements(Screen::Room)
        .iter()
        .find(|placed| placed.id == "IDC_LIST_SROOM_CHAT")
        .map(|placed| placed.rect)
        .expect("staged Room chat list")
}

/// Drive the real Room chat grammar: `/w Beta psst` then Enter.
fn send_room_whisper(session: &mut OnlineSession) {
    let mut keymap = Keymap::new();
    for (code, text) in [
        (KeyCode::Slash, "/"),
        (KeyCode::KeyW, "w"),
        (KeyCode::Space, " "),
        (KeyCode::KeyB, "b"),
        (KeyCode::KeyE, "e"),
        (KeyCode::KeyT, "t"),
        (KeyCode::KeyA, "a"),
        (KeyCode::Space, " "),
        (KeyCode::KeyP, "p"),
        (KeyCode::KeyS, "s"),
        (KeyCode::KeyS, "s"),
        (KeyCode::KeyT, "t"),
    ] {
        let _ = route_raw_key(session, &mut keymap, code, Some(text));
    }
    let _ = route_raw_key(session, &mut keymap, KeyCode::Enter, None);
}

/// #967 tier-2 wire half. Live caller chain:
/// `main.rs::run_play` -> `connect::connect_and_login_with_transport` ->
/// `assemble_online_session` -> `OnlineSession::paint` -> `poll_lobby_chat`
/// -> `apply_lobby_frame` -> `apply_room_removed` -> `ScreenGame::remove_room`
/// -> `paint_room_list`. Pin VA `0x55bc60` / file `0x15bc60`, 2007.
#[test]
fn two_booted_room_clients_drop_the_same_row_when_the_gateway_broadcasts_2007() {
    let trace_id = "two-room-clients-2007";
    let gateway = DualGateway::spawn(joined_room(0, true));
    gateway.set_trace_id(trace_id);
    let (alpha_index, mut alpha) = boot_room_client(&gateway, "Alpha");
    let (beta_index, mut beta) = boot_room_client(&gateway, "Beta");

    let before_alpha = paint_frame(&mut alpha);
    let before_beta = paint_frame(&mut beta);
    assert!(alpha
        .game_mut()
        .listed_rooms()
        .iter()
        .any(|room| room.number == 3));
    assert!(beta
        .game_mut()
        .listed_rooms()
        .iter()
        .any(|room| room.number == 3));

    gateway.broadcast(&[RoomRemoved { number: 3 }.encode()]);
    let alpha_log = gateway.wire_log(alpha_index);
    let beta_log = gateway.wire_log(beta_index);
    assert_eq!(count(&alpha_log, Direction::ServerToClient, 2007), 1);
    assert_eq!(
        count(&beta_log, Direction::ServerToClient, 2007),
        1,
        "the disband broadcast must reach both booted clients"
    );

    paint_until(&mut alpha, |session| {
        session
            .game_mut()
            .listed_rooms()
            .iter()
            .all(|room| room.number != 3)
    });
    paint_until(&mut beta, |session| {
        session
            .game_mut()
            .listed_rooms()
            .iter()
            .all(|room| room.number != 3)
    });
    assert!(changed_pixels(&before_alpha, &paint_frame(&mut alpha)) > 0);
    assert!(changed_pixels(&before_beta, &paint_frame(&mut beta)) > 0);
    assert_eq!(
        find(&alpha_log, Direction::ServerToClient, 2007).payload,
        3i32.to_le_bytes()
    );
    assert_eq!(
        find(&beta_log, Direction::ServerToClient, 2007).payload,
        3i32.to_le_bytes()
    );
    assert_shared_trace(&[alpha_log, beta_log], trace_id);
    gateway.finish(vec![alpha, beta]);
}

/// #974 tier-2 wire half. Live caller chain:
/// `main.rs::run_play` -> `InputSnapshot` keys -> `OnlineSession::flush_room_chat_action`
/// -> C2S 2017 -> fake Memoryer -> S2C 2019 -> `OnlineSession::paint` ->
/// `poll_lobby_chat` -> `apply_lobby_whisper` -> `ScreenGame::push_lobby_whisper`
/// -> `paint_room_chat`. Pin VA `0x55c280` / file `0x15c280`, 2019.
#[test]
fn a_room_whisper_sent_by_one_client_paints_the_sender_and_local_recipient_on_both_lists() {
    let trace_id = "two-room-clients-2019";
    let gateway = DualGateway::spawn(joined_room(0, true));
    gateway.set_trace_id(trace_id);
    let (alpha_index, mut alpha) = boot_room_client(&gateway, "Alpha");
    let (beta_index, mut beta) = boot_room_client(&gateway, "Beta");

    let alpha_list = room_list_rect(&mut alpha);
    let beta_list = room_list_rect(&mut beta);
    let alpha_rest = staged_rect_pixels(&mut alpha, alpha_list);
    let beta_rest = staged_rect_pixels(&mut beta, beta_list);
    send_room_whisper(&mut alpha);
    gateway.wait_for_request(alpha_index, 2017);

    let alpha_sent = gateway.wire_log(alpha_index);
    let whisper = find(&alpha_sent, Direction::ClientToServer, 2017);
    assert_eq!(
        SendWhisper::decode_body(&whisper.payload).unwrap(),
        SendWhisper {
            recipient: "beta".to_owned(),
            content: "psst".to_owned(),
        },
        "one typed Room whisper reached the gateway"
    );

    gateway.broadcast(&[ReceiveWhisper {
        sender: "Alpha".to_owned(),
        content: "psst".to_owned(),
    }
    .encode()]);
    let alpha_log = gateway.wire_log(alpha_index);
    let beta_log = gateway.wire_log(beta_index);
    assert_eq!(count(&alpha_log, Direction::ServerToClient, 2019), 1);
    assert_eq!(
        count(&beta_log, Direction::ServerToClient, 2019),
        1,
        "the whisper fan-out must reach both clients"
    );

    paint_until(&mut alpha, |session| {
        session.game_mut().room_chat().len() == 1
    });
    paint_until(&mut beta, |session| {
        session.game_mut().room_chat().len() == 1
    });
    for (session, recipient) in [(&mut alpha, "Alpha"), (&mut beta, "Beta")] {
        let row = &session.game_mut().room_chat()[0];
        assert_eq!(
            row.sender, "Alpha",
            "the wire sender is the painted display name"
        );
        assert_eq!(row.whisper_recipient.as_deref(), Some(recipient));
    }
    let alpha_painted = staged_rect_pixels(&mut alpha, alpha_list);
    let beta_painted = staged_rect_pixels(&mut beta, beta_list);
    assert!(changed_pixels(&alpha_rest, &alpha_painted) > 0);
    assert!(changed_pixels(&beta_rest, &beta_painted) > 0);
    assert_shared_trace(&[alpha_log, beta_log], trace_id);
    gateway.finish(vec![alpha, beta]);
}

/// #559 tier-2 wire half. Live caller chain:
/// `main.rs::run_play` -> `OnlineSession::paint` -> `poll_waiting_chat` ->
/// `drain_waiting_chat` -> `apply_acquire_music` / `apply_member_state_changed` /
/// `apply_member_music_state` / `apply_slot_update` -> `ScreenGame` seat mirror
/// and `paint_waiting_seat_marks` -> `start_gate_verdict`.
/// 4026/4037/3009 stay wire-only; 4035 is the pinned badge paint arm
/// (`docs/music-availability.md` §2). Pin pack hashes
/// `0580bc8d…cbd636dd` / `20fbe170…cff6`.
#[test]
fn two_waiting_clients_compare_the_downloading_badge_slice_then_agree_on_ready_and_start() {
    if !archives_present() {
        println!("SKIP two_waiting_clients_compare_the_downloading_badge_slice_then_agree_on_ready_and_start: missing install-root interface archive");
        return;
    }
    let trace_id = "two-waiting-clients-4026-4037-3009";
    let gateway = DualGateway::spawn(joined_room(0, true));
    gateway.set_trace_id(trace_id);
    let (alpha_index, mut alpha) = boot_waiting_client(&gateway, "Alpha");
    let (beta_index, mut beta) = boot_waiting_client(&gateway, "Beta");

    let badge_rect = waiting_badge_rect(&mut alpha, 0);
    let alpha_rest = staged_rect_pixels(&mut alpha, badge_rect);
    let beta_rest = staged_rect_pixels(&mut beta, badge_rect);

    let downloading = AcquireMusic {
        members: vec![MemberMusicState {
            member_id: 0,
            state: MusicState::Downloading,
        }],
    };
    let state = MemberStateChanged {
        member_id: 1,
        playing_state: 0,
        music_state: MusicState::Downloading.code(),
    };
    let music = SyncMemberMusicStateEvent {
        member_id: 1,
        state: MusicState::Downloading,
    };
    let slot = RoomSlotUpdate {
        index: 1,
        action_code: RoomSlotAction::SlotUnlocked as u8,
    };
    gateway.broadcast(&[
        downloading.encode(),
        state.encode(),
        music.encode(),
        slot.encode(),
    ]);

    let alpha_log = gateway.wire_log(alpha_index);
    let beta_log = gateway.wire_log(beta_index);
    for (index, log) in [&alpha_log, &beta_log].into_iter().enumerate() {
        let sequence: Vec<u16> = log
            .iter()
            .filter(|record| record.direction == Direction::ServerToClient)
            .filter(|record| [4026, 4037, 3009].contains(&record.id))
            .map(|record| record.id)
            .collect();
        assert_eq!(
            sequence,
            vec![4026, 4037, 3009],
            "client {index} must receive the same wire sequence"
        );
    }

    for session in [&mut alpha, &mut beta] {
        paint_until(session, |session| {
            session.slot_action(1) == Some(RoomSlotAction::SlotUnlocked)
                && session.game_mut().member_music_state(1) == Some(MusicState::Downloading.code())
                && session.game_mut().local_download_badge_armed()
        });
    }
    let alpha_badge = staged_rect_pixels(&mut alpha, badge_rect);
    let beta_badge = staged_rect_pixels(&mut beta, badge_rect);
    assert_eq!(
        alpha_badge, beta_badge,
        "the 07E8 badge slice agrees pixel-for-pixel"
    );
    assert_ne!(
        alpha_badge, alpha_rest,
        "the downloading badge actually inked the slice"
    );
    assert_ne!(
        beta_badge, beta_rest,
        "the second client's badge actually inked the slice"
    );
    for session in [&mut alpha, &mut beta] {
        assert_eq!(
            session.game_mut().start_gate_verdict(),
            StartGateVerdict::MusicDownloading,
            "a downloading peer closes START on both clients"
        );
    }

    let ready = AcquireMusic {
        members: vec![
            MemberMusicState {
                member_id: 0,
                state: MusicState::Ready,
            },
            MemberMusicState {
                member_id: 1,
                state: MusicState::Ready,
            },
        ],
    };
    let ready_music = SyncMemberMusicStateEvent {
        member_id: 1,
        state: MusicState::Ready,
    };
    gateway.broadcast(&[ready.encode(), ready_music.encode()]);
    for session in [&mut alpha, &mut beta] {
        paint_until(session, |session| {
            session.game_mut().member_music_state(1) == Some(MusicState::Ready.code())
                && !session.game_mut().local_download_badge_armed()
        });
    }
    assert_eq!(staged_rect_pixels(&mut alpha, badge_rect), alpha_rest);
    assert_eq!(staged_rect_pixels(&mut beta, badge_rect), beta_rest);
    for session in [&mut alpha, &mut beta] {
        assert_eq!(
            session.game_mut().start_gate_verdict(),
            StartGateVerdict::Startable
        );
    }
    assert_shared_trace(
        &[gateway.wire_log(alpha_index), gateway.wire_log(beta_index)],
        trace_id,
    );
    gateway.finish(vec![alpha, beta]);
}

/// #592 tier-2 wire leg. Live caller chain:
/// `main.rs::run_play` -> master `menu_key(Confirm)` -> C2S 4010 ->
/// fake Memoryer -> S2C 4011 to both -> `OnlineSession::poll_start` ->
/// Loading -> master Loading Confirm and guest paced gate -> C2S 4012
/// from both sockets. No scene state is direct-set.
#[test]
fn two_wire_legs_share_one_trace_id_for_the_4010_4011_4012_fanout() {
    if !archives_present() {
        println!("SKIP two_wire_legs_share_one_trace_id_for_the_4010_4011_4012_fanout: missing install-root chart");
        return;
    }
    let gateway = DualGateway::spawn_with_rooms(vec![joined_room(0, true), joined_room(1, true)]);
    gateway.set_trace_id(LOADING_TRACE_ID);
    println!(
        "[{LOADING_TRACE_ID}] server_pid={} scenario_epoch={LOADING_SCENARIO_EPOCH} dual-socket tap",
        DualGateway::server_pid()
    );
    let (master_index, mut master) = boot_waiting_client(&gateway, "Alpha");
    let (guest_index, mut guest) = boot_waiting_client(&gateway, "Beta");

    master.menu_key(MenuKey::Confirm, true);
    gateway.wait_for_logged_request(master_index, 4010);
    gateway.broadcast(&[encode_frame(4011, &[])]);

    let master_log = gateway.wire_log(master_index);
    let guest_log = gateway.wire_log(guest_index);
    assert_eq!(
        count(&master_log, Direction::ClientToServer, 4010),
        1,
        "one master START request drives the fan-out"
    );
    assert_eq!(count(&guest_log, Direction::ClientToServer, 4010), 0);
    assert_eq!(count(&master_log, Direction::ServerToClient, 4011), 1);
    assert_eq!(
        count(&guest_log, Direction::ServerToClient, 4011),
        1,
        "the guest's wire leg carries its own 4011"
    );

    paint_until(&mut master, |session| session.screen() == Screen::Loading);
    paint_until(&mut guest, |session| session.screen() == Screen::Loading);
    master.menu_key(MenuKey::Confirm, true);
    gateway.wait_for_logged_request(master_index, 4012);

    let LoadingGateArm::ChartArmed { entered_at, .. } = guest.loading_pacing.arm_state() else {
        panic!("the joined guest must arm its chart from the 4011 broadcast");
    };
    paint_loading_frame_at(&mut guest, entered_at + LOADING_CONFIRM_AFTER_START);
    assert_eq!(
        guest.screen(),
        Screen::Loading,
        "4012 alone must not enter Playing"
    );
    paint_loading_frame_at(
        &mut guest,
        entered_at + LOADING_CONFIRM_AFTER_START + LOADING_PLAYING_AFTER_CONFIRM,
    );
    gateway.wait_for_logged_request(guest_index, 4012);

    let master_log = gateway.wire_log(master_index);
    let guest_log = gateway.wire_log(guest_index);
    for (index, log) in [&master_log, &guest_log].into_iter().enumerate() {
        assert_eq!(
            count(log, Direction::ClientToServer, 4012),
            1,
            "client {index} must post its own 4012"
        );
    }
    assert_shared_trace(&[master_log, guest_log], LOADING_TRACE_ID);
    println!(
        "[{LOADING_TRACE_ID}] server_pid={} scenario_epoch={LOADING_SCENARIO_EPOCH} 4010={} 4011={}/{} 4012={}/{}",
        DualGateway::server_pid(),
        count(&gateway.wire_log(master_index), Direction::ClientToServer, 4010),
        count(&gateway.wire_log(master_index), Direction::ServerToClient, 4011),
        count(&gateway.wire_log(guest_index), Direction::ServerToClient, 4011),
        count(&gateway.wire_log(master_index), Direction::ClientToServer, 4012),
        count(&gateway.wire_log(guest_index), Direction::ClientToServer, 4012),
    );
    gateway.finish(vec![master, guest]);
}
