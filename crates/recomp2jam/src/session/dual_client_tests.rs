//! Tier-2 two-client guards on one fake Memoryer listener.
//!
//! Both clients reach their scene through the real connect/assemble path;
//! every event under test arrives on a socket. `super::*` reaches the
//! session's private clocked paint seam for the Loading gate (#592).

use std::time::Instant;

use o2jam_net::frame::encode_frame;
use o2jam_net::message::MemberStateChanged;
use o2jam_net::{
    AcquireMusic, MemberMusicState, MusicState, ReceiveWhisper, RoomRemoved, RoomSlotAction,
    RoomSlotUpdate, SendWhisper, SyncMemberMusicStateEvent, WireMessage,
};
use o2jam_render::{NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::screens::game::StartGateVerdict;
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

use super::*;
use crate::live_dual_client_legs::harness::{
    assert_harness_tag, boot_room_client, boot_waiting_client, changed_mask, changed_pixels,
    count_records, find_record, paint_frame, paint_until, paint_until_game, room_chat,
    room_chat_rect, send_room_whisper, waiting_badge_rect, Direction, DualGateway,
    ACQUIRE_MUSIC_OPCODE, DISBAND_OPCODE, GUEST_MEMBER_ID, GUEST_NAME, LOADED_CONFIRM_OPCODE,
    MASTER_MEMBER_ID, MASTER_NAME, MEMBER_STATE_OPCODE, MUSIC_STATE_OPCODE, ROOM_NUMBER,
    SLOT_UPDATE_OPCODE, START_ANSWER_OPCODE, START_REQUEST_OPCODE, WHISPER_CONTENT,
    WHISPER_RECEIVE_OPCODE, WHISPER_SENDER, WHISPER_SEND_OPCODE, WHISPER_TARGET,
};
use crate::live_leg_support::{joined_room, paint, staged_rect_pixels};
use crate::loading::{LoadingGateArm, LOADING_CONFIRM_AFTER_START, LOADING_PLAYING_AFTER_CONFIRM};

const LOADING_HARNESS_TAG: &str = "dual-loading-4010-4011-4012";
// Harness label only, not a field read from the 7.55 client.
// An independent Memoryer PID and epoch stay OPEN.
const HARNESS_RUN_ORDINAL: u64 = 1;

/// One clocked paint on the private session seam, without wall sleeps.
fn paint_loading_frame_at(session: &mut OnlineSession, now: Instant) {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
    let mut buf = vec![0u32; width * height];
    session.paint_at_instant(
        &mut o2jam_render::Frame {
            buf: &mut buf,
            w: width,
            h: height,
        },
        now,
    );
}

/// Tier-2 skip notice. CI has no game data (#0a3ba).
fn skip_if_no_archives(guard: &str) -> bool {
    if crate::live_leg_support::archives_present() {
        return false;
    }
    println!("SKIP {guard}: missing install-root interface archive");
    true
}

/// #967 Tier 2. Live chain: `main.rs::run_play` → connect → assembly →
/// `OnlineSession::paint` → `poll_lobby_chat` → `apply_room_removed` →
/// `ScreenGame::remove_room` → `paint_room_list`.
#[test]
fn two_room_clients_drop_the_disbanded_row() {
    let guard = "two_room_clients_drop_the_disbanded_row";
    if skip_if_no_archives(guard) {
        return;
    }
    let gateway = DualGateway::spawn(joined_room(MASTER_MEMBER_ID, true));
    gateway.set_harness_tag(guard);
    let (alpha_index, mut alpha) = boot_room_client(&gateway, MASTER_NAME);
    let (beta_index, mut beta) = boot_room_client(&gateway, GUEST_NAME);

    let before_alpha = paint_frame(&mut alpha);
    let before_beta = paint_frame(&mut beta);
    assert!(alpha
        .game_mut()
        .listed_rooms()
        .iter()
        .any(|room| room.number == ROOM_NUMBER));
    assert!(beta
        .game_mut()
        .listed_rooms()
        .iter()
        .any(|room| room.number == ROOM_NUMBER));

    gateway.broadcast(&[RoomRemoved {
        number: ROOM_NUMBER,
    }
    .encode()]);
    let alpha_log = gateway.wire_log(alpha_index);
    let beta_log = gateway.wire_log(beta_index);
    assert_eq!(
        count_records(&alpha_log, Direction::ServerToClient, DISBAND_OPCODE),
        1
    );
    assert_eq!(
        count_records(&beta_log, Direction::ServerToClient, DISBAND_OPCODE),
        1,
        "the disband broadcast must reach both booted clients"
    );

    let row_gone = |game: &ScreenGame| {
        game.listed_rooms()
            .iter()
            .all(|room| room.number != ROOM_NUMBER)
    };
    paint_until_game(&mut alpha, row_gone);
    paint_until_game(&mut beta, row_gone);
    assert!(changed_pixels(&before_alpha, &paint_frame(&mut alpha)) > 0);
    assert!(changed_pixels(&before_beta, &paint_frame(&mut beta)) > 0);
    assert_eq!(
        find_record(&alpha_log, Direction::ServerToClient, DISBAND_OPCODE).payload,
        ROOM_NUMBER.to_le_bytes()
    );
    assert_eq!(
        find_record(&beta_log, Direction::ServerToClient, DISBAND_OPCODE).payload,
        ROOM_NUMBER.to_le_bytes()
    );
    assert_harness_tag(&[alpha_log, beta_log], guard);
    gateway.finish(vec![alpha, beta]);
}

/// #974 Tier 2. Live chain: Room chat keys → `flush_room_chat_action` →
/// C2S 2017 → fake Memoryer → S2C 2019 → `poll_lobby_chat` →
/// `apply_lobby_whisper` → `paint_room_chat`. The painted sender equals the
/// wire sender; our client has no friend-record display-name lookup
/// (docs/pins/net-b.md:338-344), so that lookup stays OPEN.
#[test]
fn one_whisper_reaches_both_room_chat_lists() {
    let guard = "one_whisper_reaches_both_room_chat_lists";
    if skip_if_no_archives(guard) {
        return;
    }
    let gateway = DualGateway::spawn(joined_room(MASTER_MEMBER_ID, true));
    gateway.set_harness_tag(guard);
    let (alpha_index, mut alpha) = boot_room_client(&gateway, MASTER_NAME);
    let (beta_index, mut beta) = boot_room_client(&gateway, GUEST_NAME);

    let alpha_list = room_chat_rect(&mut alpha);
    let beta_list = room_chat_rect(&mut beta);
    let alpha_rest = staged_rect_pixels(&mut alpha, alpha_list);
    let beta_rest = staged_rect_pixels(&mut beta, beta_list);
    send_room_whisper(&mut alpha);
    gateway.wait_for_request(alpha_index, WHISPER_SEND_OPCODE);
    let alpha_sent_log = gateway.wire_log(alpha_index);
    let sent = find_record(
        &alpha_sent_log,
        Direction::ClientToServer,
        WHISPER_SEND_OPCODE,
    );
    assert_eq!(
        SendWhisper::decode_body(&sent.payload).unwrap(),
        SendWhisper {
            recipient: WHISPER_TARGET.to_owned(),
            content: WHISPER_CONTENT.to_owned(),
        }
    );

    gateway.broadcast(&[ReceiveWhisper {
        sender: WHISPER_SENDER.to_owned(),
        content: WHISPER_CONTENT.to_owned(),
    }
    .encode()]);
    let alpha_log = gateway.wire_log(alpha_index);
    let beta_log = gateway.wire_log(beta_index);
    assert_eq!(
        count_records(
            &alpha_log,
            Direction::ServerToClient,
            WHISPER_RECEIVE_OPCODE
        ),
        1
    );
    assert_eq!(
        count_records(&beta_log, Direction::ServerToClient, WHISPER_RECEIVE_OPCODE),
        1,
        "the whisper fan-out must reach both clients"
    );

    let one_row = |game: &ScreenGame| game.room_chat().len() == 1;
    paint_until_game(&mut alpha, one_row);
    paint_until_game(&mut beta, one_row);
    let alpha_row = room_chat(&mut alpha).remove(0);
    let beta_row = room_chat(&mut beta).remove(0);
    assert_eq!(alpha_row.sender, WHISPER_SENDER);
    assert_eq!(beta_row.sender, WHISPER_SENDER);
    assert_eq!(alpha_row.whisper_recipient.as_deref(), Some(MASTER_NAME));
    assert_eq!(beta_row.whisper_recipient.as_deref(), Some(GUEST_NAME));
    assert!(changed_pixels(&alpha_rest, &staged_rect_pixels(&mut alpha, alpha_list)) > 0);
    assert!(changed_pixels(&beta_rest, &staged_rect_pixels(&mut beta, beta_list)) > 0);
    assert_harness_tag(&[alpha_log, beta_log], guard);
    gateway.finish(vec![alpha, beta]);
}

/// #559 Tier 2. Live chain: Waiting join → `poll_waiting_chat` → 4035 guest
/// badge arm / 4026 / 4037 / 3009 → `paint_waiting_seat_marks` →
/// `start_gate_verdict`. 4026/4037/3009 stay wire-only; 4035 is the pinned
/// paint arm. Two Rust frames are compared to each other only; comparison
/// to the exe A/B badge slice stays OPEN.
#[test]
fn master_and_guest_agree_on_badge_and_start_gate() {
    let guard = "master_and_guest_agree_on_badge_and_start_gate";
    if skip_if_no_archives(guard) {
        return;
    }
    let gateway = DualGateway::spawn_with_rooms(vec![
        joined_room(MASTER_MEMBER_ID, true),
        joined_room(GUEST_MEMBER_ID, true),
    ]);
    gateway.set_harness_tag(guard);
    let (master_index, mut master) = boot_waiting_client(&gateway, MASTER_NAME);
    let (guest_index, mut guest) = boot_waiting_client(&gateway, GUEST_NAME);

    let master_own_rect = waiting_badge_rect(&mut master, MASTER_MEMBER_ID);
    let guest_own_rect = waiting_badge_rect(&mut guest, GUEST_MEMBER_ID);
    let master_peer_state = MemberStateChanged {
        member_id: MASTER_MEMBER_ID,
        playing_state: 0,
        music_state: MusicState::Downloading.code(),
    };
    let guest_peer_music = SyncMemberMusicStateEvent {
        member_id: GUEST_MEMBER_ID,
        state: MusicState::Downloading,
    };
    let slot = RoomSlotUpdate {
        index: GUEST_MEMBER_ID,
        action_code: RoomSlotAction::SlotUnlocked as u8,
    };
    gateway.broadcast(&[
        master_peer_state.encode(),
        guest_peer_music.encode(),
        slot.encode(),
    ]);

    for (index, log) in [
        gateway.wire_log(master_index),
        gateway.wire_log(guest_index),
    ]
    .iter()
    .enumerate()
    {
        let sequence: Vec<u16> = log
            .iter()
            .filter(|record| record.direction == Direction::ServerToClient)
            .filter(|record| {
                [MEMBER_STATE_OPCODE, MUSIC_STATE_OPCODE, SLOT_UPDATE_OPCODE].contains(&record.id)
            })
            .map(|record| record.id)
            .collect();
        assert_eq!(
            sequence,
            vec![MEMBER_STATE_OPCODE, MUSIC_STATE_OPCODE, SLOT_UPDATE_OPCODE],
            "client {index} missed the wire sequence"
        );
    }

    paint_until_game(&mut master, |game| {
        game.member_music_state(GUEST_MEMBER_ID) == Some(MusicState::Downloading.code())
    });
    paint_until_game(&mut guest, |game| {
        game.member_music_state(MASTER_MEMBER_ID) == Some(MusicState::Downloading.code())
    });
    assert_eq!(
        master.slot_action(GUEST_MEMBER_ID),
        Some(RoomSlotAction::SlotUnlocked)
    );
    assert_eq!(
        guest.slot_action(GUEST_MEMBER_ID),
        Some(RoomSlotAction::SlotUnlocked)
    );

    // Rest frames are taken after the wire states stage, before the 4035 arm.
    let master_own_rest = staged_rect_pixels(&mut master, master_own_rect);
    let guest_master_rest = staged_rect_pixels(&mut guest, master_own_rect);
    let guest_own_rest = staged_rect_pixels(&mut guest, guest_own_rect);

    let downloading = AcquireMusic {
        members: vec![MemberMusicState {
            member_id: GUEST_MEMBER_ID,
            state: MusicState::Downloading,
        }],
    };
    gateway.broadcast(&[downloading.encode()]);
    // Paint both clients so each consumes the guest-targeted 4035.
    paint(&mut master);
    paint(&mut guest);
    assert!(
        !master.game_mut().local_download_badge_armed(),
        "a guest-targeted 4035 must not arm the master seat"
    );
    assert!(
        guest.game_mut().local_download_badge_armed(),
        "a guest-targeted 4035 must arm the guest seat"
    );

    for (index, log) in [
        gateway.wire_log(master_index),
        gateway.wire_log(guest_index),
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            count_records(log, Direction::ServerToClient, ACQUIRE_MUSIC_OPCODE),
            1,
            "client {index} must receive the guest-only 4035 paint arm"
        );
    }

    assert_eq!(
        staged_rect_pixels(&mut master, master_own_rect),
        master_own_rest
    );
    assert_ne!(
        staged_rect_pixels(&mut guest, guest_own_rect),
        guest_own_rest
    );
    // Two Rust frames agree on the master seat's change mask. Raw pixels
    // differ by role, and comparison to the exe A/B crop stays OPEN.
    assert_eq!(
        changed_mask(&paint_frame(&mut master), &master_own_rest, master_own_rect),
        changed_mask(
            &paint_frame(&mut guest),
            &guest_master_rest,
            master_own_rect
        ),
        "the two Rust frames must agree before the verdict"
    );
    for session in [&mut master, &mut guest] {
        assert_eq!(
            session.game_mut().start_gate_verdict(),
            StartGateVerdict::MusicDownloading
        );
    }

    let ready = AcquireMusic {
        members: vec![
            MemberMusicState {
                member_id: MASTER_MEMBER_ID,
                state: MusicState::Ready,
            },
            MemberMusicState {
                member_id: GUEST_MEMBER_ID,
                state: MusicState::Ready,
            },
        ],
    };
    let ready_peer_state = MemberStateChanged {
        member_id: GUEST_MEMBER_ID,
        playing_state: 0,
        music_state: MusicState::Ready.code(),
    };
    let ready_peer_music = SyncMemberMusicStateEvent {
        member_id: MASTER_MEMBER_ID,
        state: MusicState::Ready,
    };
    gateway.broadcast(&[
        ready.encode(),
        ready_peer_state.encode(),
        ready_peer_music.encode(),
    ]);
    let cleared = |game: &ScreenGame| {
        !game.local_download_badge_armed()
            && game.start_gate_verdict() == StartGateVerdict::Startable
    };
    paint_until_game(&mut master, cleared);
    paint_until_game(&mut guest, cleared);
    assert_eq!(
        staged_rect_pixels(&mut guest, guest_own_rect),
        guest_own_rest
    );
    assert_eq!(
        staged_rect_pixels(&mut master, master_own_rect),
        master_own_rest
    );
    assert_harness_tag(
        &[
            gateway.wire_log(master_index),
            gateway.wire_log(guest_index),
        ],
        guard,
    );
    gateway.finish(vec![master, guest]);
}

/// #592 Tier 2. Live chain: master Confirm → C2S 4010 → fake Memoryer →
/// S2C 4011 to both → `poll_start` → Loading → master Confirm and guest
/// paced gate → C2S 4012 from both. `harness_tag` is a harness label only;
/// a reserved trace id and an independent Memoryer PID/epoch stay OPEN.
#[test]
fn both_clients_log_the_start_fanout() {
    let guard = "both_clients_log_the_start_fanout";
    if skip_if_no_archives(guard) {
        return;
    }
    let gateway = DualGateway::spawn_with_rooms(vec![
        joined_room(MASTER_MEMBER_ID, true),
        joined_room(GUEST_MEMBER_ID, true),
    ]);
    gateway.set_harness_tag(LOADING_HARNESS_TAG);
    println!(
        "[{LOADING_HARNESS_TAG}] harness_process_id={} harness_run_ordinal={HARNESS_RUN_ORDINAL} dual-socket tap",
        DualGateway::harness_process_id()
    );
    let (master_index, mut master) = boot_waiting_client(&gateway, MASTER_NAME);
    let (guest_index, mut guest) = boot_waiting_client(&gateway, GUEST_NAME);

    master.menu_key(MenuKey::Confirm, true);
    gateway.wait_for_logged_request(master_index, START_REQUEST_OPCODE);
    gateway.broadcast(&[encode_frame(START_ANSWER_OPCODE, &[])]);
    assert_eq!(
        count_records(
            &gateway.wire_log(master_index),
            Direction::ClientToServer,
            START_REQUEST_OPCODE
        ),
        1
    );
    assert_eq!(
        count_records(
            &gateway.wire_log(guest_index),
            Direction::ClientToServer,
            START_REQUEST_OPCODE
        ),
        0
    );
    assert_eq!(
        count_records(
            &gateway.wire_log(master_index),
            Direction::ServerToClient,
            START_ANSWER_OPCODE
        ),
        1
    );
    assert_eq!(
        count_records(
            &gateway.wire_log(guest_index),
            Direction::ServerToClient,
            START_ANSWER_OPCODE
        ),
        1,
        "the guest's wire leg carries its own 4011"
    );

    paint_until(&mut master, |session| session.screen() == Screen::Loading);
    paint_until(&mut guest, |session| session.screen() == Screen::Loading);
    master.menu_key(MenuKey::Confirm, true);
    gateway.wait_for_logged_request(master_index, LOADED_CONFIRM_OPCODE);

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
    gateway.wait_for_logged_request(guest_index, LOADED_CONFIRM_OPCODE);

    let master_log = gateway.wire_log(master_index);
    let guest_log = gateway.wire_log(guest_index);
    for (index, log) in [&master_log, &guest_log].iter().enumerate() {
        assert_eq!(
            count_records(log, Direction::ClientToServer, LOADED_CONFIRM_OPCODE),
            1,
            "client {index} must log its own 4012"
        );
    }
    assert_harness_tag(&[master_log, guest_log], LOADING_HARNESS_TAG);
    println!(
        "[{LOADING_HARNESS_TAG}] harness_process_id={} harness_run_ordinal={HARNESS_RUN_ORDINAL} 4010={} 4011={}/{} 4012={}/{}",
        DualGateway::harness_process_id(),
        count_records(
            &gateway.wire_log(master_index),
            Direction::ClientToServer,
            START_REQUEST_OPCODE
        ),
        count_records(
            &gateway.wire_log(master_index),
            Direction::ServerToClient,
            START_ANSWER_OPCODE
        ),
        count_records(
            &gateway.wire_log(guest_index),
            Direction::ServerToClient,
            START_ANSWER_OPCODE
        ),
        count_records(
            &gateway.wire_log(master_index),
            Direction::ClientToServer,
            LOADED_CONFIRM_OPCODE
        ),
        count_records(
            &gateway.wire_log(guest_index),
            Direction::ClientToServer,
            LOADED_CONFIRM_OPCODE
        ),
    );
    gateway.finish(vec![master, guest]);
}
