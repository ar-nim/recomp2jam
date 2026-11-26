//! Scripted-gateway guards for the Room wire family (#290, lane room-wire).
//!
//! Declared `#[cfg(test)]` by `session.rs`, so `use super::*` reaches the
//! crate-private session items the drain paths live on. Every guard here
//! names the caller chain from the session paint to the guarded arm; the
//! headless live legs live in `crate::live_room_wire_legs`.

use std::io::Write;
use std::net::TcpStream;

use o2jam_net::frame::encode_frame;
use o2jam_net::message::{
    RoomDescriptorChanged, RoomParameterChanged, RoomSkillChanged, RoomStateChanged,
    RoomUserCountChanged, RoomUserList, RoomUserListEntry,
};
use o2jam_net::rooms::{
    MainRoomAdminMessage, MainRoomUserMessage, MusicPlayRankingAnswer, RankDelta, RankEntry,
    ReceiveWhisper, SendWhisperResponse,
};
use o2jam_net::{Request, RoomListEntry, WireMessage};

use super::session_tests::{paint_frame, paint_until, read_frame, room_session};
use super::*;

/// One staged row that passes the shell's listed-room filter.
fn listed_row(number: i32, title: &str) -> RoomListEntry {
    RoomListEntry {
        number,
        state_code: 1,
        title: title.to_owned(),
        has_password: false,
        music_id: 100,
        difficulty: 0,
        key_mode: 2,
        game_mode: 0,
        speed: 1,
        capacity: 8,
        user_count: 1,
        min_level: 0,
        max_level: 99,
        skills: Vec::new(),
        premium: false,
        room_type: 0,
    }
}

/// Gateway answer to the host's 4000 after a 2006 create: the client
/// still posts 4025, so the server must read that frame before quitting.
fn answer_created_room_music(stream: &mut TcpStream) {
    let select = read_frame(stream);
    assert_eq!(select.id, Request::SetRoomMusic.id());
    let echo = o2jam_net::WaitingMusicChanged {
        music_id: 0x0361,
        difficulty: 0,
        speed: 1,
    };
    stream.write_all(&echo.encode()).unwrap();
    let state = read_frame(stream);
    assert_eq!(state.id, Request::UpdateMusicState.id());
}

#[test]
fn snapshot_2003_from_the_gateway_repaints_the_staged_room_rows() {
    // Live caller chain: main.rs::run_play -> assemble_online_session ->
    // OnlineSession::paint -> poll_wire -> poll_lobby_chat ->
    // drain_lobby_chat -> apply_lobby_frame -> apply_room_list ->
    // ScreenGame::set_rooms -> paint_room_list.
    let (mut session, server) = room_session(|mut stream| {
        let list = o2jam_net::RoomList {
            rooms: vec![listed_row(3, "Fresh")],
        };
        stream.write_all(&list.encode()).unwrap();
    });
    session
        .game_mut()
        .set_rooms(vec![listed_row(3, "Stale"), listed_row(7, "Other")]);
    paint_until(&mut session, |session| {
        session
            .game
            .listed_rooms()
            .iter()
            .any(|room| room.title == "Fresh")
    });
    server.join().unwrap();
    let titles: Vec<&str> = session
        .game
        .listed_rooms()
        .iter()
        .map(|room| room.title.as_str())
        .collect();
    assert_eq!(titles, ["Fresh"]);
}

#[test]
fn create_2006_success_lands_in_waiting_with_fresh_empty_seats() {
    // Live caller chain: main.rs::run_play -> staged CREATE sheet OK ->
    // OnlineSession::paint -> flush_create_room -> 2004/2006 answer ->
    // 2002/2003 snapshot -> create_and_join_room -> stage_hosted_room ->
    // ScreenGame::set_joined_room (seven empty seat containers).
    let (mut session, server) = room_session(|mut stream| {
        let create = read_frame(&mut stream);
        assert_eq!(create.id, Request::CreateRoom.id());
        let answer = o2jam_net::message::CreateRoomResponse {
            result: o2jam_net::message::CreateResult::Success,
            info: Some(o2jam_net::message::CreatedRoom {
                number: 5,
                premium: false,
                password: String::new(),
                has_password: false,
                min_level: 1,
                max_level: 99,
            }),
        };
        stream.write_all(&answer.encode()).unwrap();
        assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
        let list = o2jam_net::RoomList {
            rooms: vec![listed_row(5, "FreshRoom")],
        };
        stream.write_all(&list.encode()).unwrap();
        answer_created_room_music(&mut stream);
    });
    // Walk the seam the shell uses: the staged CREATE sheet produces
    // the request, then paints drive the create wire and the Waiting
    // enter. The button rect comes from the same staging as live boot.
    use o2jam_render::Rect;
    use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
    use std::collections::HashMap;
    session.game_mut().load_create_room_scene(
        vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_DCROOM_CREATE".to_owned(),
            code: o2jam_shell::screens::side::CREATE_ROOM_CREATE_CODE,
            sprite: None,
            rect: Rect {
                x: 10,
                y: 10,
                w: 50,
                h: 30,
            },
        }],
        HashMap::new(),
    );
    session.game_mut().open_create_room_dialog();
    for character in "FreshRoom".chars() {
        session.game_mut().push_room_dialog_char(character);
    }
    session.game_mut().click_room_dialog(20, 20);
    paint_until(&mut session, |session| session.screen() == Screen::Waiting);
    server.join().unwrap();

    let room = session
        .joined_room
        .as_ref()
        .expect("2006 answer staged a room");
    assert_eq!(room.title, "FreshRoom");
    // The exe clears eight 0xb4-byte seat records; our joined snapshot
    // carries seven declarable seats, all empty after create.
    assert_eq!(room.slots.len(), 7);
    assert!(
        room.slots.iter().all(|slot| slot.member.is_none()),
        "successful create starts with fresh seats"
    );
    for seat in 0..7u8 {
        assert!(session.game.staged_waiting_member(seat).is_none());
    }
}

#[test]
fn scripted_2007_disband_drops_the_named_row_without_a_new_snapshot() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_room_removed -> ScreenGame::remove_room -> paint_room_list.
    // The two-client observer capture stays a rig leg; this is the
    // scripted-gateway half with the pinned 2007 body (`docs/pins/net-b.md`).
    let (mut session, server) = room_session(|mut stream| {
        stream
            .write_all(&encode_frame(2007, &5i32.to_le_bytes()))
            .unwrap();
    });
    session
        .game_mut()
        .set_rooms(vec![listed_row(5, "Gone"), listed_row(7, "Stays")]);
    paint_until(&mut session, |session| {
        session.game.listed_rooms().len() == 1
    });
    server.join().unwrap();
    assert_eq!(session.game.listed_rooms()[0].title, "Stays");
}

#[test]
fn scripted_2008_descriptor_change_rewrites_the_named_row_in_place() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_room_descriptor -> rewrite_room_row -> ScreenGame::upsert_room.
    let change = RoomDescriptorChanged {
        number: 3,
        title: "Renamed".to_owned(),
        descriptor_a: 2,
        descriptor_b: 3,
        descriptor_c: 4,
        secondary_title: "Stage".to_owned(),
        descriptor_d: 5,
        descriptor_e: 6,
        descriptor_word: 0x1234,
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&change.encode()).unwrap();
    });
    session.game_mut().set_rooms(vec![listed_row(3, "Old")]);
    paint_until(&mut session, |session| {
        session
            .game
            .listed_rooms()
            .first()
            .is_some_and(|room| room.title == "Renamed")
    });
    server.join().unwrap();
    assert_eq!(session.game.listed_rooms().len(), 1);
    assert_eq!(session.game.listed_rooms()[0].number, 3);
}

#[test]
fn scripted_2009_user_count_updates_the_room_row_occupancy_readout() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_room_user_count -> rewrite_room_row -> ScreenGame::upsert_room
    // -> paint_room_list's `IDC_TEXT_SROOM_ROOM_USER` well (census ord 33).
    let change = RoomUserCountChanged {
        number: 3,
        capacity: 8,
        user_count: 5,
        descriptor_word: 0x0102,
        descriptor_byte: 7,
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&change.encode()).unwrap();
    });
    session.game_mut().set_rooms(vec![listed_row(3, "Jam")]);
    paint_until(&mut session, |session| {
        session
            .game
            .listed_rooms()
            .first()
            .is_some_and(|room| room.user_count == 5)
    });
    server.join().unwrap();
    let row = &session.game.listed_rooms()[0];
    assert_eq!((row.capacity, row.user_count), (8, 5));
}

#[test]
fn scripted_2011_user_list_answer_is_not_dropped_by_the_lobby_drain() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_room_user_list_fuse -> poll_lobby_chat -> apply_lobby_frame ->
    // note_room_user_list -> ScreenGame::track_room_user_list_reply.
    let users = RoomUserList {
        users: vec![RoomUserListEntry {
            id: 1,
            username: "host".to_owned(),
            nickname: "Host".to_owned(),
            level: 12,
        }],
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&users.encode()).unwrap();
    });
    session.game_mut().set_room_no_response_relay_down(true);
    assert!(session.game_mut().track_room_user_list_send());
    assert!(session.game.room_user_list_poll_tracked());
    paint_until(&mut session, |session| {
        !session.game.room_user_list_poll_tracked()
    });
    server.join().unwrap();
    assert!(session.deferred_frames.is_empty());
}

#[test]
fn scripted_2013_and_2014_broadcasts_stage_two_distinct_lobby_rows() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_lobby_user_message / apply_lobby_admin_message ->
    // ScreenGame::push_lobby_user_message / push_lobby_admin_message.
    let (mut session, server) = room_session(|mut stream| {
        stream
            .write_all(
                &MainRoomUserMessage {
                    sender: "host".to_owned(),
                    content: b"hello".to_vec(),
                }
                .encode(),
            )
            .unwrap();
        stream
            .write_all(
                &MainRoomAdminMessage {
                    sender: "staff".to_owned(),
                    content: b"notice".to_vec(),
                }
                .encode(),
            )
            .unwrap();
    });
    paint_until(&mut session, |session| session.game.room_chat().len() == 2);
    server.join().unwrap();
    let rows = session.game.room_chat();
    assert_eq!(
        (rows[0].sender.as_str(), rows[0].text.as_str()),
        ("host", "hello")
    );
    assert!(!rows[0].admin);
    assert_eq!(
        (rows[1].sender.as_str(), rows[1].text.as_str()),
        ("staff", "notice")
    );
    assert!(rows[1].admin, "2014 rows carry the admin ink flag");
}

#[test]
fn scripted_2018_whisper_answer_stages_a_double_angle_delivery_row() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_whisper_answer -> ScreenGame::push_lobby_user_message ->
    // Room chat list (the exe's `<<` delivery line).
    let answer = SendWhisperResponse {
        invalid: false,
        recipient: "peer".to_owned(),
        content: Some("psst".to_owned()),
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&answer.encode()).unwrap();
    });
    paint_until(&mut session, |session| session.game.room_chat().len() == 1);
    server.join().unwrap();
    let row = &session.game.room_chat()[0];
    assert_eq!(row.sender, "<<");
    assert_eq!(row.text, "peer: psst");
}

#[test]
fn scripted_2019_whisper_names_the_sender_and_keeps_the_recipient() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_lobby_whisper -> ScreenGame::push_lobby_whisper.
    let whisper = ReceiveWhisper {
        sender: "peer".to_owned(),
        content: "psst".to_owned(),
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&whisper.encode()).unwrap();
    });
    session.set_local_user("me");
    paint_until(&mut session, |session| session.game.room_chat().len() == 1);
    server.join().unwrap();
    let row = &session.game.room_chat()[0];
    assert_eq!(row.sender, "peer");
    assert_eq!(row.whisper_recipient.as_deref(), Some("me"));
    assert_eq!(session.game.last_whisper_peer(), Some("peer"));
}

#[test]
fn scripted_2020_state_change_moves_the_row_state_marker() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_room_state -> rewrite_room_row -> ScreenGame::upsert_room.
    let change = RoomStateChanged {
        number: 3,
        state_code: 2,
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&change.encode()).unwrap();
    });
    session.game_mut().set_rooms(vec![listed_row(3, "Jam")]);
    paint_until(&mut session, |session| {
        session
            .game
            .listed_rooms()
            .first()
            .is_some_and(|room| room.state_code == 2)
    });
    server.join().unwrap();
    assert_eq!(session.game.listed_rooms()[0].state_code, 2);
}

#[test]
fn scripted_2025_skill_change_replaces_the_row_skill_badges() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_room_skills -> rewrite_room_row -> ScreenGame::upsert_room.
    let change = RoomSkillChanged {
        number: 3,
        skills: vec![11, 22],
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&change.encode()).unwrap();
    });
    session
        .game_mut()
        .set_rooms(vec![listed_row_with_skills(3, "Jam")]);
    paint_until(&mut session, |session| {
        session
            .game
            .listed_rooms()
            .first()
            .is_some_and(|room| room.skills == [11, 22])
    });
    server.join().unwrap();
    assert_eq!(session.game.listed_rooms()[0].skills, [11, 22]);
}

/// Same staged row with a stale badge, so the 2025 change has an
/// observable before/after instead of an empty-to-filled decode.
fn listed_row_with_skills(number: i32, title: &str) -> RoomListEntry {
    let mut room = listed_row(number, title);
    room.skills = vec![99];
    room
}

#[test]
fn scripted_2023_parameter_change_updates_the_staged_music_word() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame ->
    // apply_room_parameter -> rewrite_room_row -> ScreenGame::upsert_room
    // -> the row's music badge. The shifted high nibble has no row field.
    let change = RoomParameterChanged {
        number: 3,
        parameter: 0xF123,
        descriptor_a: 4,
        descriptor_b: 5,
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&change.encode()).unwrap();
    });
    session.game_mut().set_rooms(vec![listed_row(3, "Jam")]);
    paint_until(&mut session, |session| {
        session
            .game
            .listed_rooms()
            .first()
            .is_some_and(|room| room.music_id == 0x0123)
    });
    server.join().unwrap();
    assert_eq!(session.game.listed_rooms()[0].music_id, 0x0123);
}

#[test]
fn painting_a_gateway_room_list_uses_the_session_paint_not_a_staged_shell_list() {
    // Live caller chain: main.rs::run_play -> OnlineSession::paint ->
    // poll_wire -> poll_lobby_chat -> apply_lobby_frame -> apply_room_list
    // -> ScreenGame::set_rooms -> painted Room board.
    let (mut session, server) = room_session(|mut stream| {
        let list = o2jam_net::RoomList {
            rooms: vec![listed_row(3, "Wire")],
        };
        stream.write_all(&list.encode()).unwrap();
    });
    session.game_mut().set_rooms(vec![listed_row(3, "Shell")]);
    let before = super::session_tests::painted_game_frame(&mut session);
    paint_until(&mut session, |session| {
        session
            .game
            .listed_rooms()
            .first()
            .is_some_and(|room| room.title == "Wire")
    });
    let after = super::session_tests::painted_game_frame(&mut session);
    server.join().unwrap();
    assert_ne!(before, after, "the 2003 frame changed the painted board");
}

#[test]
fn scripted_2011_user_list_populates_the_staged_user_sheet() {
    // #970 wiring. Live caller chain: main.rs::run_play ->
    // OnlineSession::paint -> poll_wire -> poll_lobby_chat ->
    // apply_lobby_frame -> note_room_user_list ->
    // ScreenGame::set_room_user_sheet.
    let users = RoomUserList {
        users: vec![
            RoomUserListEntry {
                id: 1,
                username: "host".to_owned(),
                nickname: "Host".to_owned(),
                level: 12,
            },
            RoomUserListEntry {
                id: 2,
                username: "peer".to_owned(),
                nickname: "Peer".to_owned(),
                level: 30,
            },
        ],
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&users.encode()).unwrap();
    });
    session.game_mut().set_room_no_response_relay_down(true);
    assert!(session.game_mut().track_room_user_list_send());
    paint_until(&mut session, |session| {
        session.game.room_user_sheet().len() == 2
    });
    server.join().unwrap();
    let sheet = session.game.room_user_sheet();
    assert_eq!(sheet[0].nickname, "Host");
    assert_eq!(sheet[1].level, 30);
}

#[test]
fn scripted_2018_refusal_stages_the_zero_ink_row() {
    // #973 wiring. Live caller chain: main.rs::run_play ->
    // OnlineSession::paint -> poll_wire -> poll_lobby_chat ->
    // apply_lobby_frame -> apply_whisper_answer ->
    // ScreenGame::push_whisper_refusal -> ChatState zero-ink row.
    let refusal = o2jam_net::rooms::SendWhisperResponse {
        invalid: true,
        recipient: "Ghost".to_owned(),
        content: None,
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&refusal.encode()).unwrap();
    });
    paint_until(&mut session, |session| {
        session.game.room_chat().iter().any(|row| row.zero_ink)
    });
    server.join().unwrap();
    let refusal_row = session
        .game
        .room_chat()
        .iter()
        .find(|row| row.zero_ink)
        .expect("one refusal row");
    assert!(refusal_row.text.contains("Ghost"));
}

/// Every negative 3003 result must stage the pinned one-OK caption
/// without entering Waiting. The seven caption strings cover eight
/// result words: `-6` and `-4` share the room-full text.
#[test]
fn scripted_3003_refusals_post_a_sheet_for_every_pinned_result_code() {
    // Live caller chain: main.rs::run_play -> assemble_online_session ->
    // OnlineSession::poll_attempts -> poll_room_join -> drain_room_join ->
    // JoinRoomResponse 3003 -> ScreenGame::stage_join_answer_refusal ->
    // paint_room -> paint_information_message.
    use o2jam_net::{JoinRoom, JoinRoomResponse};
    use o2jam_shell::screens::game::JoinAnswerRefusal;

    let cases = [
        (-8i32, JoinAnswerRefusal::InvalidRoomNumber),
        (-7, JoinAnswerRefusal::AllSongFreeUser),
        (-6, JoinAnswerRefusal::RoomFull),
        (-5, JoinAnswerRefusal::GameInProgress),
        (-4, JoinAnswerRefusal::RoomFull),
        (-3, JoinAnswerRefusal::WrongPassword),
        (-2, JoinAnswerRefusal::SingleRoom),
        (-1, JoinAnswerRefusal::AdministratorContact),
    ];
    for (signed_code, expected) in cases {
        let result_code = u32::from_le_bytes(signed_code.to_le_bytes());
        let (mut session, server) = room_session(move |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            let answer = JoinRoomResponse {
                result_code,
                room: None,
            };
            stream.write_all(&answer.encode()).unwrap();
        });
        session.set_pending_room_join(3, "");
        session.menu_key(MenuKey::Confirm, true);
        for _ in 0..3 {
            paint_frame(&mut session);
        }
        assert!(
            session.game.dialog_owns_input(),
            "result {signed_code} posts a one-OK sheet"
        );
        assert_eq!(session.screen(), Screen::Room, "refusal stays in Room");
        assert_eq!(
            session.game.join_answer_refusal(),
            Some(expected),
            "result {signed_code} maps to the pinned caption"
        );
        assert!(
            !expected.message().is_empty(),
            "the pinned caption has exe bytes"
        );
        server.join().unwrap();
    }
}

#[test]
fn scripted_2023_parameter_stages_the_member_word_high_nibble() {
    // #990 wiring. Live caller chain: main.rs::run_play ->
    // OnlineSession::paint -> poll_wire -> poll_lobby_chat ->
    // apply_lobby_frame -> apply_room_parameter ->
    // ScreenGame::stage_room_member_word.
    let change = RoomParameterChanged {
        number: 3,
        parameter: 0x8123,
        descriptor_a: 0,
        descriptor_b: 0,
    };
    let (mut session, server) = room_session(move |mut stream| {
        stream.write_all(&change.encode()).unwrap();
    });
    session.game_mut().set_rooms(vec![listed_row(3, "Jam")]);
    paint_until(&mut session, |session| {
        session.game.room_member_word(3) == Some(0x0008_0123)
    });
    server.join().unwrap();
    assert_eq!(session.game.room_member_word(3), Some(0x0008_0123));
    assert_eq!(session.game.listed_rooms()[0].music_id, 0x123);
}

#[test]
fn scripted_3003_snapshot_seats_the_native_room_roster() {
    use super::session_tests::{joined_snapshot, member_on_team};
    // #1134. Live caller chain: main.rs::run_play -> poll_room_join ->
    // drain_room_join -> OnlineSession::resolve_room_join ->
    // ScreenGame::set_room_roster_from_joined -> ChatState::set_room_user_sheet.
    let mut room = joined_snapshot();
    let mut host = member_on_team("Host", true, 0);
    host.master = true;
    host.level = 21;
    room.slots[0].member = Some(host);
    let mut peer = member_on_team("Peer", false, 1);
    peer.level = 9;
    room.slots[1].member = Some(peer);
    room.user_count = 2;
    let (mut session, server) = room_session(|_stream| {});
    session.resolve_room_join(room, EntryEcho::Confirm);
    server.join().unwrap();

    let sheet = session.game.room_user_sheet();
    assert_eq!(sheet.len(), 2, "both 3003 seats must join the roster");
    assert_eq!(sheet[0].nickname, "Host");
    assert_eq!(sheet[0].level, 21);
    assert_eq!(sheet[1].nickname, "Peer");
    assert_eq!(sheet[1].id, 1, "the seat index is the roster id");
}

#[test]
fn scripted_3004_join_broadcast_adds_one_roster_row() {
    // #1134. Live caller chain: main.rs::run_play -> OnlineSession::poll_wire
    // -> poll_waiting_chat -> apply_waiting_frame -> apply_user_joined ->
    // ScreenGame::seat_room_roster_member -> ChatState seat-keyed row.
    use super::session_tests::{joined_snapshot, member_on_team};

    let room = joined_snapshot();
    let (mut session, server) = room_session(|_stream| {});
    session.game_mut().set_room_roster_from_joined(&room);
    let join = o2jam_net::rooms::UserJoinedWaiting {
        member_id: 2,
        member: member_on_team("NewPeer", false, 2),
    };
    session.apply_waiting_frame(Frame {
        id: o2jam_net::rooms::UserJoinedWaiting::COMMAND,
        payload: join.encode_body(),
    });
    server.join().unwrap();

    let sheet = session.game.room_user_sheet();
    assert_eq!(sheet.len(), 1, "one 3004 broadcast adds one row");
    assert_eq!(sheet[0].id, 2);
    assert_eq!(sheet[0].nickname, "NewPeer");
}

#[test]
fn scripted_3007_leave_broadcast_removes_the_row_and_promotes_the_master() {
    // #1134. Live caller chain: main.rs::run_play -> OnlineSession::poll_wire
    // -> poll_waiting_chat -> apply_waiting_frame -> apply_user_left ->
    // ScreenGame::clear_room_roster_member -> ChatState master word.
    use super::session_tests::{joined_snapshot, member_on_team};

    let room = joined_snapshot();
    let (mut session, server) = room_session(|_stream| {});
    session.game_mut().set_room_roster_from_joined(&room);
    let join = o2jam_net::rooms::UserJoinedWaiting {
        member_id: 2,
        member: member_on_team("NewPeer", false, 2),
    };
    session.apply_waiting_frame(Frame {
        id: o2jam_net::rooms::UserJoinedWaiting::COMMAND,
        payload: join.encode_body(),
    });
    let leave = o2jam_net::rooms::UserLeftWaiting {
        member_id: 2,
        new_master_id: 1,
        premium: false,
        has_super_manager: false,
    };
    session.apply_waiting_frame(Frame {
        id: o2jam_net::rooms::UserLeftWaiting::COMMAND,
        payload: leave.encode_body(),
    });
    server.join().unwrap();

    let sheet = session.game.room_user_sheet();
    assert!(sheet.is_empty(), "the departed seat leaves the roster");
    assert_eq!(session.game.room_roster_master(), Some(1));
}

/// One decoded 2028 line with a known place and nickname.
fn ranked_row(rank: i32, nickname: &str) -> RankEntry {
    RankEntry {
        rank,
        nickname: nickname.to_owned(),
        battles: 12,
        wins: 8,
        win_rate: 66,
        delta: RankDelta::Up,
        rank_delta: 1,
    }
}

#[test]
fn scripted_2028_answer_stages_the_native_top100_rows_from_the_gateway() {
    // Live caller chain: main.rs::run_play -> assemble_online_session ->
    // OnlineSession::paint -> poll_wire_at -> poll_lobby_chat ->
    // drain_lobby_chat -> apply_lobby_frame -> apply_rank_answer ->
    // ScreenGame::stage_lanking_board -> LankingState::stage_board.
    // seam: none. The Room lobby drain is the live owner of the music-shop
    // overlay's wire; a 2028 answer must survive it without a request.
    let (mut session, server) = room_session(|mut stream| {
        let answer = MusicPlayRankingAnswer {
            self_entry: ranked_row(42, "Viewer"),
            entries: vec![ranked_row(1, "Champ"), ranked_row(2, "Rival")],
        };
        stream.write_all(&answer.encode()).unwrap();
    });
    paint_until(&mut session, |session| {
        session.game.lanking_self_entry().is_some()
    });
    server.join().unwrap();

    let self_entry = session.game.lanking_self_entry().expect("viewer line");
    let ranks = session.game.lanking_visible_ranks();
    let nicknames: Vec<&str> = ranks.iter().map(|row| row.nickname.as_str()).collect();
    assert_eq!(self_entry.nickname, "Viewer");
    assert_eq!(nicknames, ["Champ", "Rival"]);
}

#[test]
fn absent_2028_answer_leaves_the_native_top100_rows_unstaged() {
    // Empty state is explicit: no answer means no self line and no rows,
    // never a synthesized standings table. Live chain checked here:
    // main.rs::run_play -> OnlineSession::paint -> poll_wire_at ->
    // poll_lobby_chat -> apply_lobby_frame; with no 2028 frame the
    // apply_rank_answer arm is never taken, so nothing is staged.
    let (mut session, server) = room_session(|_stream| {});
    paint_frame(&mut session);
    server.join().unwrap();

    assert_eq!(session.game.lanking_self_entry(), None);
    assert!(session.game.lanking_visible_ranks().is_empty());
}
