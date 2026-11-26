#![cfg(test)]

//! Tier 3 shell legs for the Room/Waiting hand-offs (#951, #955).
//!
//! Every leg boots through the real connect entry with a scripted
//! gateway, walks to Waiting through pointer snapshots, and reads the
//! pixels the shell paints from the staged 3003 snapshot. Missing
//! installed archives skip each leg cleanly (CI has no game data).

use o2jam_net::{JoinedRoom, RoomMember, RoomSlot, RoomSlotState};
use o2jam_render::NATIVE_WIDTH;
use o2jam_shell::Screen;

use crate::live_leg_support::*;

/// One 3003 snapshot with the two fields under test varied: the peer's
/// gender byte and the local member's raw wire team. All other fields
/// stay identical so a pixel delta names one of those two sources.
fn scripted_shell_room(
    peer_gender: u8,
    local_wire_team: u8,
    local_seat_in_tail: bool,
) -> JoinedRoom {
    let member = |nickname: &str, gender: u8, team: u8, master: bool| RoomMember {
        nickname: nickname.to_owned(),
        level: 12,
        gender,
        gem: 900,
        master,
        team,
        ready: true,
        music_state: 0,
        // The installed naked kit has different male/female bodies;
        // keeping equipment empty isolates the gender source.
        equipment: [0; 17],
        music_ids: Vec::new(),
        cash_point: 1000,
        free_pass: 0,
        playing_state: 0,
        administrator: false,
        super_manager: false,
    };
    let slots = (0..7u8)
        .map(|index| RoomSlot {
            index,
            state_code: if index < 2 {
                RoomSlotState::Occupied as i32
            } else {
                RoomSlotState::Unoccupied as i32
            },
            // The derived-team leg carries the local member in slot 0;
            // the figure leg uses a pure peer tail so its gender byte
            // owns the first wire member.
            member: match index {
                0 => Some(if local_seat_in_tail {
                    member("HOST", 0, local_wire_team, true)
                } else {
                    member("PEER", peer_gender, 0, false)
                }),
                1 => Some(if local_seat_in_tail {
                    member("PEER", peer_gender, 0, false)
                } else {
                    member("PEER2", 0, 0, false)
                }),
                _ => None,
            },
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
        champion: None,
        champion_streak: None,
    }
}

/// Boot the scripted snapshot through a real gateway and enter Waiting
/// only by pointer snapshots and the 3003 answer.
fn boot_shell_leg(room: JoinedRoom) -> (crate::session::OnlineSession, LiveLegGateway) {
    let gateway = spawn_live_leg_gateway(room);
    let mut session = boot_live_leg_with_prestaged_art(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    assert_eq!(session.screen(), Screen::Waiting);
    (session, gateway)
}

/// Native pixels of one staged control, read from the same frame the
/// window paints.
fn staged_pixels(session: &mut crate::session::OnlineSession, id: &str) -> Vec<u32> {
    let rect = session
        .game_mut()
        .scene_placements(Screen::Waiting)
        .iter()
        .find(|placed| placed.id == id)
        .unwrap_or_else(|| panic!("no staged placement {id}"))
        .rect;
    let frame = paint_frame(session);
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let top = usize::try_from(rect.y).expect("rect top fits usize");
    let left = usize::try_from(rect.x).expect("rect left fits usize");
    let rows = usize::try_from(rect.h).expect("rect height fits usize");
    let columns = usize::try_from(rect.w).expect("rect width fits usize");
    (top..top + rows)
        .flat_map(|y| (left..left + columns).map(move |x| (y, x)))
        .map(|(y, x)| frame[y * width + x])
        .collect()
}

/// #951: a nonzero gender byte selects the exe's paired seat control
/// id, so the peer's worn figure must change even when the byte is not
/// the binary male value.
///
/// Live caller chain up to the staged figure:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> stage_seat_figures
///   -> seat_figure_is_male(gender_control_id).
/// seam: the peer-slot-to-cover index offset is a separate OPEN
/// mapping, so the leg mounts the staged figure on the second cover
/// through `ScreenGame::set_waiting_avatar` before the paint, then
/// follows `ScreenGame::paint_at -> paint_waiting_seat_avatars`.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_nonzero_gender_byte_paints_the_paired_seat_figure() {
    if !archives_present() {
        println!("SKIP live_room_nonzero_gender_byte_paints_the_paired_seat_figure: missing install-root interface archive");
        return;
    }
    let (mut female, female_gateway) = boot_shell_leg(scripted_shell_room(0, 0, false));
    let female_figure = female
        .game_mut()
        .waiting_avatar(1)
        .expect("the zero-gender wire peer staged a figure")
        .clone();
    female.game_mut().set_waiting_avatar(1, female_figure);
    let female_pixels = staged_pixels(&mut female, "IDC_IMAGE_SWAIT_COVERAVATAR2");
    finish(female, female_gateway.server);

    let (mut other, other_gateway) = boot_shell_leg(scripted_shell_room(2, 0, false));
    let other_figure = other
        .game_mut()
        .waiting_avatar(1)
        .expect("the nonzero-gender wire peer staged a figure")
        .clone();
    other.game_mut().set_waiting_avatar(1, other_figure);
    let other_pixels = staged_pixels(&mut other, "IDC_IMAGE_SWAIT_COVERAVATAR2");
    assert_ne!(
        female_pixels, other_pixels,
        "gender byte 2 must select the paired nonzero-gender seat art"
    );
    finish(other, other_gateway.server);
}

/// #955: the local seat's derived team byte comes from the exe's
/// direct team setter, beside the raw wire team still held on the
/// staged member. Its team plate frame must follow that derivation.
///
/// Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> OnlineSession::poll_wire (3003)
///   -> ScreenGame::set_joined_room -> staged_seats -> team plate
///   -> ScreenGame::paint_at -> paint_waiting_seat_marks.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_derived_team_plate_differs_from_the_raw_wire_seat_team() {
    if !archives_present() {
        println!("SKIP live_room_derived_team_plate_differs_from_the_raw_wire_seat_team: missing install-root interface archive");
        return;
    }
    assert_eq!(
        o2jam_shell::screens::paint::staged_derived_team(3),
        4,
        "the wire team takes the exe's one-step increment"
    );
    let (mut plain, plain_gateway) = boot_shell_leg(scripted_shell_room(0, 0, true));
    let plain_pixels = staged_pixels(&mut plain, "IDC_IMAGE_SWAIT_COVERAVATAR1");
    finish(plain, plain_gateway.server);

    let (mut derived, derived_gateway) = boot_shell_leg(scripted_shell_room(0, 3, true));
    let wire_team = derived
        .game_mut()
        .staged_waiting_member(0)
        .expect("the local 3003 seat is staged")
        .team;
    assert_eq!(wire_team, 3, "the raw wire byte stays on the staged member");
    let derived_pixels = staged_pixels(&mut derived, "IDC_IMAGE_SWAIT_COVERAVATAR1");
    assert_ne!(
        plain_pixels, derived_pixels,
        "the derived team value must select another team-plate frame"
    );
    finish(derived, derived_gateway.server);
}

use std::io::Write;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use o2jam_net::message::{RoomParameterChanged, RoomUserList, RoomUserListEntry};
use o2jam_net::rooms::SendWhisperResponse;
use o2jam_net::testing::fake_gateway;
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, ChannelState, JoinRoomResponse, Request,
    WireMessage,
};
use o2jam_shell::screens::game::LobbyChat;

/// Boot gateway for a Room-board leg with a chosen channel id and a
/// scripted-frame queue. The frame queue keeps each leg free of wall-
/// clock races: events are sent only after the Room board is on screen.
struct RoomGateway {
    port: u16,
    server: JoinHandle<()>,
}

/// A Room gateway that answers the click's 3002 and still lets the
/// leg inject later broadcasts. The writer thread owns a cloned
/// socket; the reader thread answers the one join and records frames.
struct RosterGateway {
    port: u16,
    server: JoinHandle<()>,
    incoming: Sender<Vec<u8>>,
    join_answer_written: Receiver<()>,
}

/// The four boot answers every live Room leg needs, in wire order.
fn answer_roster_boot(stream: &mut std::net::TcpStream) {
    let mut answer = |id: u16, body: Vec<u8>| {
        assert_eq!(read_frame_until_eof(stream).unwrap().id, id);
        stream.write_all(&body).unwrap();
    };
    answer(
        Request::Authorize.id(),
        AuthResponse {
            result_code: 0,
            tail: Vec::new(),
        }
        .encode(),
    );
    answer(
        Request::GetChannelList.id(),
        ChannelList {
            channels: vec![channel_row()],
        }
        .encode(),
    );
    answer(
        Request::ChannelLogin.id(),
        ChannelLoginResponse {
            result_code: 0,
            restricted: Some(false),
        }
        .encode(),
    );
    answer(
        Request::GetChannelInfo.id(),
        o2jam_net::RoomList {
            rooms: vec![room_row()],
        }
        .encode(),
    );
}

fn spawn_roster_gateway(joined: o2jam_net::JoinedRoom) -> RosterGateway {
    let (incoming, to_send) = mpsc::channel::<Vec<u8>>();
    let (join_written_tx, join_answer_written) = mpsc::channel();
    let (port, server) = fake_gateway(move |mut stream| {
        answer_roster_boot(&mut stream);
        let mut writer = stream.try_clone().unwrap();
        let injector = std::thread::spawn(move || {
            while let Ok(frame) = to_send.recv() {
                writer.write_all(&frame).unwrap();
            }
        });
        while let Some(frame) = read_frame_until_eof(&mut stream) {
            if frame.id == Request::JoinWaiting.id() {
                stream
                    .write_all(
                        &JoinRoomResponse {
                            result_code: 0,
                            room: Some(joined.clone()),
                        }
                        .encode(),
                    )
                    .unwrap();
                join_written_tx.send(()).unwrap();
            }
        }
        injector.join().unwrap();
    });
    RosterGateway {
        port,
        server,
        incoming,
        join_answer_written,
    }
}

/// Boot to the Room board with a gateway that can inject later
/// broadcasts, so the roster leg drives the real socket answers.
fn boot_roster_leg() -> (crate::session::OnlineSession, RosterGateway) {
    let gateway = spawn_roster_gateway(joined_room(0, true));
    let mut session = boot_live_leg_with_art(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    (session, gateway)
}

fn spawn_room_gateway(channel_id: u16) -> (RoomGateway, Sender<Vec<u8>>) {
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
                    tail: vec![],
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
                    channels: vec![ChannelState {
                        gateway_id: 1,
                        channel_id,
                        capacity: 100,
                        population: 1,
                        active: true,
                    }],
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
    (RoomGateway { port, server }, send)
}

fn boot_room_leg(channel_id: u16) -> (crate::session::OnlineSession, RoomGateway, Sender<Vec<u8>>) {
    let (gateway, send) = spawn_room_gateway(channel_id);
    let mut session = boot_live_leg(gateway.port, false);
    dismiss_splash(&mut session);
    enter_room_by_clicks(&mut session);
    (session, gateway, send)
}

/// Native pixels of one staged control on any screen.
fn staged_screen_pixels(
    session: &mut crate::session::OnlineSession,
    screen: Screen,
    id: &str,
) -> Vec<u32> {
    let rect = session
        .game_mut()
        .scene_placements(screen)
        .iter()
        .find(|placed| placed.id == id)
        .unwrap_or_else(|| panic!("no staged placement {id}"))
        .rect;
    let frame = paint_frame(session);
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    let top = usize::try_from(rect.y).expect("rect top fits usize");
    let left = usize::try_from(rect.x).expect("rect left fits usize");
    let rows = usize::try_from(rect.h).expect("rect height fits usize");
    let columns = usize::try_from(rect.w).expect("rect width fits usize");
    (top..top + rows)
        .flat_map(|y| (left..left + columns).map(move |x| (y, x)))
        .map(|(y, x)| frame[y * width + x])
        .collect()
}

/// Paint until a boot-driven Room event reaches the shell seam.
fn paint_until_room(
    session: &mut crate::session::OnlineSession,
    ready: impl Fn(&mut crate::session::OnlineSession) -> bool,
) {
    for _ in 0..MAX_SCREEN_PAINTS * 8 {
        std::thread::yield_now();
        paint(session);
        if ready(session) {
            return;
        }
    }
    panic!("scripted Room event never reached the staged shell state");
}

/// #959: the channel-id-keyed default control ids must reach the
/// paired MODE/OM buttons the Room painter owns.
///
/// Live caller chain for the normal id:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> stage_channel_list_after_relay_gate -> ScreenGame::set_waiting_channel_number
///   -> ScreenGame::paint_at -> paint_room -> paint_channel_button_pair.
/// seam: the premium 0x2fd id enters through the same setter on the
/// booted session (the scripted gateway only offers the resident id),
/// so the A/B pins the painter, not the 1003 rewrite.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_channel_button_pair_uses_the_default_control_ids() {
    if !archives_present() {
        println!("SKIP live_room_channel_button_pair_uses_the_default_control_ids: missing install-root interface archive");
        return;
    }
    let (mut normal, normal_gateway, _) = boot_room_leg(2);
    let normal_mode = staged_screen_pixels(
        &mut normal,
        Screen::Room,
        "IDC_BUTTON_SROOM_SHOW_NORMALMODE",
    );
    finish(normal, normal_gateway.server);

    // The normal boot already staged the resident 1003 channel id;
    // this alternate id enters through the same boot setter so both
    // frames share one pointer path and one gateway shape.
    let (mut premium, premium_gateway, _) = boot_room_leg(2);
    premium.game_mut().set_waiting_channel_number(0x2fd);
    let premium_mode = staged_screen_pixels(
        &mut premium,
        Screen::Room,
        "IDC_BUTTON_SROOM_SHOW_NORMALMODE",
    );
    assert_ne!(
        normal_mode, premium_mode,
        "channel 0x2fd must paint the premium default id pair"
    );
    finish(premium, premium_gateway.server);
}

/// #970: one counted 2011 sheet must populate the Room user sheet.
///
/// Live caller chain up to the decoded wire rows:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> enter_room_by_clicks -> OnlineSession::poll_wire
///   -> note_room_user_list -> RoomUserList decode.
/// seam: `OnlineSession::note_room_user_list` has no call to the new
/// `ScreenGame::set_room_user_sheet` yet (session.rs hand-off), so the
/// leg stages the decoded rows through that seam, then follows
/// `ScreenGame::set_room_user_sheet -> ScreenGame::paint_at ->
/// paint_room` to the DESC well. Web content stays out of scope.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_2011_rows_populate_the_staged_user_sheet() {
    if !archives_present() {
        println!("SKIP live_room_2011_rows_populate_the_staged_user_sheet: missing install-root interface archive");
        return;
    }
    let (mut session, gateway, send) = boot_room_leg(2);
    let rest = roster_pixels(&mut session);
    let sheet = RoomUserList {
        users: vec![RoomUserListEntry {
            id: 7,
            username: "alice".to_owned(),
            nickname: "Alice".to_owned(),
            level: 42,
        }],
    };
    send.send(sheet.encode()).unwrap();
    // The frame is on the real socket; the drain has no shell staging
    // call yet (session.rs hand-off), so give the boot loop a few
    // paint turns and stage the decoded rows at the named seam.
    for _ in 0..8 {
        std::thread::yield_now();
        paint(&mut session);
    }
    session.game_mut().open_room_user_sheet(true);
    session.game_mut().set_room_user_sheet(&sheet.users);
    let painted = staged_screen_pixels(&mut session, Screen::Room, "IDC_TEXT_SROOM_DESC");
    assert_ne!(rest, painted, "the 2011 row must fill the user sheet well");
    assert_eq!(session.game_mut().room_user_sheet().len(), 1);
    drop(send);
    finish(session, gateway.server);
}

/// #973: an invalid 2018 answer stages a zero-ink refusal row.
///
/// ev_4abe96c99ad41dde2de8e524bed38c46f40c9759baf43149573ce44cd987b161
/// (0x55c190 / file 0x15c190, docs/pins/net-b.md).
///
/// Live caller chain up to the decoded answer:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> enter_room_by_clicks -> OnlineSession::poll_wire
///   -> apply_whisper_answer -> SendWhisperResponse decode.
/// seam: the drain's refusal arm is a session.rs hand-off; the leg
/// stages the refusal through `ScreenGame::push_whisper_refusal`, then
/// follows `ScreenGame::paint_at -> paint_room_chat -> chat_row_ink_for`.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_2018_refusal_inks_no_pixels_in_the_room_chat() {
    if !archives_present() {
        println!("SKIP live_room_2018_refusal_inks_no_pixels_in_the_room_chat: missing install-root interface archive");
        return;
    }
    let (mut session, gateway, send) = boot_room_leg(2);
    let answer = SendWhisperResponse {
        invalid: true,
        recipient: "ghost".to_owned(),
        content: None,
    };
    send.send(answer.encode()).unwrap();
    paint_until_room(&mut session, |session| {
        !session.game_mut().room_chat().is_empty()
    });
    session
        .game_mut()
        .push_whisper_refusal(LobbyChat::Room, &answer.recipient);
    let row = session
        .game_mut()
        .room_chat()
        .last()
        .expect("the refusal row staged")
        .clone();
    assert_eq!(row.text, "ghost");
    assert!(row.zero_ink, "the exe's refusal arm posts with zero ink");
    let painted = paint_frame(&mut session);
    assert!(
        painted.iter().any(|pixel| *pixel != 0),
        "the rest of the staged Room board still paints"
    );
    drop(send);
    finish(session, gateway.server);
}

/// #990: the shifted high nibble of one 2023 word reaches the row's
/// 32-bit member word and its paint arm.
///
/// Live caller chain up to the decoded event:
///   main.rs::run_play -> connect::connect_and_login_with_transport
///   -> assemble_online_session -> enter_room_by_clicks -> OnlineSession::poll_wire
///   -> apply_room_parameter -> RoomParameterChanged decode.
/// seam: session.rs currently stages only the low twelve bits; the
/// leg stages the full word through `ScreenGame::stage_room_member_word`,
/// then follows `ScreenGame::paint_at -> paint_room -> paint_room_member_word`.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_2023_high_nibble_reaches_the_painted_member_word() {
    if !archives_present() {
        println!("SKIP live_room_2023_high_nibble_reaches_the_painted_member_word: missing install-root interface archive");
        return;
    }
    let (mut session, gateway, send) = boot_room_leg(2);
    let rest = staged_screen_pixels(&mut session, Screen::Room, "IDC_TEXT_SROOM_MUSICTITLE");
    let change = RoomParameterChanged {
        number: 3,
        parameter: 0xA123,
        descriptor_a: 0,
        descriptor_b: 0,
    };
    send.send(change.encode()).unwrap();
    paint_until_room(&mut session, |session| {
        session
            .game_mut()
            .listed_rooms()
            .iter()
            .any(|room| room.number == 3 && room.music_id == 0x0123)
    });
    session
        .game_mut()
        .stage_room_member_word(change.number, change.parameter);
    assert_eq!(
        session.game_mut().room_member_word(change.number),
        Some(0xA_0123),
        "the high nibble sits above the low twelve bits"
    );
    let painted = staged_screen_pixels(&mut session, Screen::Room, "IDC_TEXT_SROOM_MUSICTITLE");
    assert_ne!(rest, painted, "the shifted nibble changes the member word");
    drop(send);
    finish(session, gateway.server);
}

/// One 3004 member body for the roster legs.
fn roster_join_frame(seat: u8, nickname: &str) -> Vec<u8> {
    let joined = o2jam_net::rooms::UserJoinedWaiting {
        member_id: seat,
        member: o2jam_net::RoomMember {
            nickname: nickname.to_owned(),
            level: 14,
            gender: 0,
            gem: 0,
            master: false,
            team: seat,
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
    };
    joined.encode()
}

/// Native pixels of the staged DESC well that shows the roster.
fn roster_pixels(session: &mut crate::session::OnlineSession) -> Vec<u32> {
    staged_screen_pixels(session, Screen::Room, "IDC_TEXT_SROOM_DESC")
}

/// Leave Waiting through the staged BACK control, then paint Room.
fn return_to_room(session: &mut crate::session::OnlineSession) {
    click_staged(session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
    paint_until_screen(session, Screen::Room);
    paint(session);
}

fn leave_frame(seat: u8, promoted: u8) -> Vec<u8> {
    o2jam_net::rooms::UserLeftWaiting {
        member_id: seat,
        new_master_id: promoted,
        premium: false,
        has_super_manager: false,
    }
    .encode()
}

/// #1134 Tier 3. Live caller chain:
///   main.rs::run_play -> connect::connect_and_login_with_transport ->
///   assemble_online_session -> enter_room_by_clicks -> join_waiting_by_click
///   -> OnlineSession::poll_room_join -> resolve_room_join ->
///   ScreenGame::set_room_roster_from_joined -> paint_at -> paint_room.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_3003_snapshot_inks_the_native_player_list_from_boot() {
    if !archives_present() {
        println!("SKIP live_room_3003_snapshot_inks_the_native_player_list_from_boot: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_roster_leg();
    session.game_mut().open_room_user_sheet(false);
    let rest = roster_pixels(&mut session);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    return_to_room(&mut session);
    let inked = roster_pixels(&mut session);
    let roster = session.game_mut().room_user_sheet().to_vec();
    drop(gateway.incoming);
    finish(session, gateway.server);

    assert_eq!(roster.len(), 1, "the 3003 peer is the native roster row");
    assert_eq!(roster[0].nickname, "peer");
    assert_ne!(rest, inked, "the staged DESC well inks the joined row");
}

/// #1134 Tier 3. Live caller chain:
///   main.rs::run_play -> join_waiting_by_click -> OnlineSession::poll_wire
///   -> apply_waiting_frame -> apply_user_joined ->
///   ScreenGame::seat_room_roster_member -> paint_at -> paint_room.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_3004_broadcast_repaints_the_added_roster_row() {
    if !archives_present() {
        println!("SKIP live_room_3004_broadcast_repaints_the_added_roster_row: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_roster_leg();
    session.game_mut().open_room_user_sheet(false);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    return_to_room(&mut session);
    let rest = roster_pixels(&mut session);

    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    gateway
        .incoming
        .send(roster_join_frame(2, "NewPeer"))
        .unwrap();
    paint_until_room(&mut session, |session| {
        session.game_mut().room_user_sheet().len() == 2
    });
    return_to_room(&mut session);
    let inked = roster_pixels(&mut session);
    let roster = session.game_mut().room_user_sheet().to_vec();
    drop(gateway.incoming);
    finish(session, gateway.server);

    assert_eq!(roster.len(), 2);
    assert_eq!(roster[1].nickname, "NewPeer");
    assert_ne!(rest, inked, "the new seat inks inside the staged well");
}

/// #1134 Tier 3. Same boot chain, leave arm:
///   OnlineSession::poll_wire -> apply_waiting_frame -> apply_user_left ->
///   ScreenGame::clear_room_roster_member -> paint_at -> paint_room.
/// Archives absent -> skip cleanly (CI has no game data).
#[test]
fn live_room_3007_broadcast_repaints_the_leave_and_promotes_the_master() {
    if !archives_present() {
        println!("SKIP live_room_3007_broadcast_repaints_the_leave_and_promotes_the_master: missing install-root interface archive");
        return;
    }
    let (mut session, gateway) = boot_roster_leg();
    session.game_mut().open_room_user_sheet(false);
    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    return_to_room(&mut session);
    let rest = roster_pixels(&mut session);

    join_waiting_by_click(&mut session, &gateway.join_answer_written);
    gateway
        .incoming
        .send(roster_join_frame(2, "NewPeer"))
        .unwrap();
    gateway.incoming.send(leave_frame(1, 2)).unwrap();
    paint_until_room(&mut session, |session| {
        session.game_mut().room_roster_master() == Some(2)
    });
    return_to_room(&mut session);
    let inked = roster_pixels(&mut session);
    let roster = session.game_mut().room_user_sheet().to_vec();
    let master = session.game_mut().room_roster_master();
    drop(gateway.incoming);
    finish(session, gateway.server);

    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].nickname, "NewPeer", "the promoted peer remains");
    assert_eq!(master, Some(2));
    assert_ne!(rest, inked, "the departed row leaves the staged well");
}
