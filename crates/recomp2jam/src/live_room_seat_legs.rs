#![cfg(test)]

//! Tier 3 live legs for the Room roster and Waiting seat readouts.
//!
//! Every leg boots through the real `connect` entry with a scripted
//! 3003 seat snapshot, joins Waiting by pointer snapshots, and reads
//! the staged seats or pixels the shell paints from that snapshot.
//! Missing installed archives skip the leg cleanly (CI has no data).

use o2jam_net::{JoinedRoom, RoomMember, RoomSlot, RoomSlotState};
use o2jam_render::{Rect, NATIVE_WIDTH};
use o2jam_shell::Screen;

use crate::live_leg_support::*;

/// One 3003 seat snapshot: the local member and one peer with distinct
/// values for every roster and readout field under test.
fn scripted_seat_room(peer_ready: bool, streak: Option<i32>) -> JoinedRoom {
    let local = RoomMember {
        nickname: "HOST".to_owned(),
        level: 12,
        gender: 0,
        gem: 900,
        master: true,
        team: 0,
        ready: true,
        music_state: 0,
        equipment: [0; 17],
        music_ids: Vec::new(),
        cash_point: 1000,
        free_pass: 0,
        playing_state: 0,
        administrator: false,
        super_manager: false,
    };
    let peer = RoomMember {
        nickname: "PEER".to_owned(),
        level: 42,
        gender: 1,
        gem: 1234,
        master: false,
        team: 3,
        ready: peer_ready,
        music_state: 0,
        equipment: [
            11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
        ],
        music_ids: vec![100, 200],
        cash_point: 5678,
        free_pass: 2,
        playing_state: 0,
        administrator: false,
        super_manager: false,
    };
    let slots = (0..7u8)
        .map(|index| RoomSlot {
            index,
            state_code: RoomSlotState::Occupied as i32,
            member: Some(match index {
                0 => local.clone(),
                1 => peer.clone(),
                _ => RoomMember {
                    nickname: String::new(),
                    level: 0,
                    gender: 0,
                    gem: 0,
                    master: false,
                    team: 0,
                    ready: false,
                    music_state: 0,
                    equipment: [0; 17],
                    music_ids: Vec::new(),
                    cash_point: 0,
                    free_pass: 0,
                    playing_state: 0,
                    administrator: false,
                    super_manager: false,
                },
            }),
        })
        .collect();
    JoinedRoom {
        member_id: 0,
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
        champion: Some(1),
        champion_streak: streak,
    }
}

/// Boot the scripted seat snapshot through the real gateway and enter
/// Waiting through pointer snapshots only.
fn boot_scripted_seat_leg(room: JoinedRoom) -> (crate::session::OnlineSession, LiveLegGateway) {
    let gateway = spawn_live_leg_gateway(room);
    let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    assert_eq!(session.screen(), Screen::Waiting);
    (session, gateway)
}

/// One staged `.bnd` placement by its authored id.
fn staged_id_rect(session: &mut crate::session::OnlineSession, id: &str) -> Rect {
    session
        .game_mut()
        .scene_placements(Screen::Waiting)
        .iter()
        .find(|placed| placed.id == id)
        .unwrap_or_else(|| panic!("no staged placement {id}"))
        .rect
}

/// Count one exact ink word inside a staged rect of a painted frame.
fn count_ink(frame: &[u32], rect: Rect, ink: u32) -> usize {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let rows = i32::try_from(rect.h).expect("rect height fits i32");
    let columns = i32::try_from(rect.w).expect("rect width fits i32");
    (rect.y..rect.y + rows)
        .flat_map(move |y| (rect.x..rect.x + columns).map(move |x| (y, x)))
        .filter(|(y, x)| {
            frame[usize::try_from(*y).expect("y fits usize") * width
                + usize::try_from(*x).expect("x fits usize")]
                == ink
        })
        .count()
}

/// #290 seats 949/950/952/953/954/959/960: fields of the 3003 member
/// records must reach the same staged seats the Waiting painter reads.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> stage_live_art
///   -> GameLoop::ingest_snapshot (splash, emblem, row, join)
///   -> OnlineSession::poll_wire (3003) -> ScreenGame::set_joined_room
///   -> ScreenGame::staged_waiting_member -> GameLoop::paint_at.
/// One leg covers the sibling member-record stories because they all
/// ride the one 3003 tail into the one staged seat table.
/// Archives absent -> skip cleanly (CI has no data).
#[test]
fn live_room_3003_member_fields_stage_the_roster_and_readouts() {
    if !archives_present() {
        println!("SKIP live_room_3003_member_fields_stage_the_roster_and_readouts: missing install-root interface archive");
        return;
    }
    let _live_leg_guard = hold_live_leg_lock();
    let (mut session, gateway) = boot_scripted_seat_leg(scripted_seat_room(true, None));
    let peer = session
        .game_mut()
        .staged_waiting_member(1)
        .expect("the 3003 peer seat is staged")
        .clone();
    assert_eq!(peer.nickname, "PEER", "member id names the staged seat");
    assert_eq!(peer.level, 42);
    assert_eq!(peer.gem, 1234);
    assert_eq!(peer.cash_point, 5678);
    assert_eq!(peer.free_pass, 2);
    assert!(!peer.master, "the wire peer is not the room master");
    assert_eq!(
        peer.equipment,
        [11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27]
    );
    let local = session
        .game_mut()
        .staged_waiting_member(0)
        .expect("the local 3003 seat is staged");
    assert!(local.master, "the wire master flag reaches the local seat");
    let frame = paint_frame(&mut session);
    assert!(
        frame.iter().any(|word| *word != 0),
        "the staged scene painted"
    );
    finish(session, gateway.server);
}

/// #290 seats 951: the gender byte rides the same 3003 tail, and the
/// paired control-id choice is pinned by the sibling Tier 0 guard.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> staged_waiting_member.
/// Archives absent -> skip cleanly (CI has no data).
#[test]
fn live_room_gender_byte_from_3003_reaches_the_staged_seat() {
    if !archives_present() {
        println!("SKIP live_room_gender_byte_from_3003_reaches_the_staged_seat: missing install-root interface archive");
        return;
    }
    let _live_leg_guard = hold_live_leg_lock();
    let (mut session, gateway) = boot_scripted_seat_leg(scripted_seat_room(true, None));
    let local_gender = session
        .game_mut()
        .staged_waiting_member(0)
        .expect("local seat staged")
        .gender;
    let peer_gender = session
        .game_mut()
        .staged_waiting_member(1)
        .expect("peer seat staged")
        .gender;
    assert_eq!(local_gender, 0);
    assert_eq!(peer_gender, 1);
    let _frame = paint_frame(&mut session);
    finish(session, gateway.server);
}

/// #290 seats 955/956/957/958: team, ready and master marks come from
/// the staged seat, and READY changes the ink of the staged cover dot.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> ScreenGame::paint_at
///   -> paint_waiting_scene_with_confirm -> paint_cover_ready_dot.
/// Archives absent -> skip cleanly (CI has no data).
#[test]
fn live_room_team_ready_and_master_marks_paint_from_the_wire_seat() {
    if !archives_present() {
        println!("SKIP live_room_team_ready_and_master_marks_paint_from_the_wire_seat: missing install-root interface archive");
        return;
    }
    let _live_leg_guard = hold_live_leg_lock();
    let (mut ready_session, ready_gateway) = boot_scripted_seat_leg(scripted_seat_room(true, None));
    let peer = ready_session
        .game_mut()
        .staged_waiting_member(1)
        .expect("peer seat staged");
    assert_eq!(peer.team, 3, "wire team lands on the peer seat");
    assert!(peer.ready, "wire ready lands on the peer seat");
    let peer_master = ready_session
        .game_mut()
        .staged_waiting_member(1)
        .expect("peer seat staged")
        .master;
    assert!(!peer_master, "only the wire master seat holds the badge");
    let cover = staged_id_rect(&mut ready_session, "IDC_IMAGE_SWAIT_COVERAVATAR2");
    let dot = Rect {
        x: cover.x - 6,
        y: cover.y,
        w: 3,
        h: 5,
    };
    let ready_ink = 0x0030_E030;
    let unready_ink = 0x0050_5050;
    let ready_frame = paint_frame(&mut ready_session);
    assert!(
        count_ink(&ready_frame, dot, ready_ink) > 0,
        "READY dot is lit"
    );
    finish(ready_session, ready_gateway.server);

    let (mut idle_session, idle_gateway) = boot_scripted_seat_leg(scripted_seat_room(false, None));
    let idle_frame = paint_frame(&mut idle_session);
    assert_eq!(
        count_ink(&idle_frame, dot, ready_ink),
        0,
        "an unready wire seat keeps the dot dark"
    );
    assert!(count_ink(&idle_frame, dot, unready_ink) > 0);
    finish(idle_session, idle_gateway.server);
}

/// #290 seat 961: the champion streak from 3003 inks the Wins well;
/// without a streak the well stays unpainted.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> ScreenGame::paint_at
///   -> paint_waiting_staged -> paint_waiting_wins.
/// Archives absent -> skip cleanly (CI has no data).
#[test]
fn live_room_champion_streak_inks_the_wins_well_from_3003() {
    if !archives_present() {
        println!("SKIP live_room_champion_streak_inks_the_wins_well_from_3003: missing install-root interface archive");
        return;
    }
    let _live_leg_guard = hold_live_leg_lock();
    let (mut with_streak, streak_gateway) =
        boot_scripted_seat_leg(scripted_seat_room(true, Some(7)));
    let well = staged_id_rect(&mut with_streak, "IDC_TEXT_SWAIT_WINNERSCROE");
    let streak_frame = paint_frame(&mut with_streak);
    let streak_pixels = staged_rect_pixels_from(&streak_frame, well);
    assert!(
        streak_pixels.iter().any(|word| *word != 0),
        "the staged Wins well shows the wire streak"
    );
    finish(with_streak, streak_gateway.server);

    let (mut without_streak, plain_gateway) =
        boot_scripted_seat_leg(scripted_seat_room(true, None));
    let plain_frame = paint_frame(&mut without_streak);
    let plain_pixels = staged_rect_pixels_from(&plain_frame, well);
    assert_ne!(
        streak_pixels, plain_pixels,
        "no wire streak leaves the Wins well unchanged"
    );
    finish(without_streak, plain_gateway.server);
}
