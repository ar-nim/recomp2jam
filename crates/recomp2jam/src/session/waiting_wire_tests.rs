//! Scripted-gateway guards for the Waiting wire families (#291 wire stories).
//! Declared `#[cfg(test)]` by `session.rs`, so `use super::*;` reaches the
//! crate-private session items the drain paths live on.

use super::session_tests::{joined_snapshot, joined_waiting_session, member_on_team, paint_until};

/// #994: the 3010 room-removal event picks one of the two pinned
/// captions from LOCAL seat state (`0x4bb700`; CP949 strings at
/// `0x64da60`/`0x64da88`). The handler reads no wire body.
mod removal_notice {
    use std::io::Write;

    use o2jam_net::frame::encode_frame;

    use super::*;

    /// One scripted 3010 through the real session drain with the
    /// staged own-seat state word set to `state`. The caption is read
    /// from the same shell accessor the painter uses.
    fn caption_for_own_seat_state(state: u8) -> Option<String> {
        let mut room = joined_snapshot();
        let mut member = member_on_team("HOST", true, 0);
        member.music_state = state;
        room.slots[0].state_code = 1;
        room.slots[0].member = Some(member);
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream.write_all(&encode_frame(3010, &[])).unwrap();
        });
        paint_until(&mut session, |session| {
            session.game.waiting_notice().is_some()
        });
        let caption = session.game.waiting_notice().map(str::to_owned);
        server.join().unwrap();
        caption
    }

    /// Tier 2 scripted-gateway guard. Live caller chain:
    /// `main.rs::run_play` -> `OnlineSession::paint` ->
    /// `poll_waiting_chat` -> `drain_waiting_chat` ->
    /// `apply_waiting_frame` -> `apply_room_closed_notice` ->
    /// `ScreenGame::stage_waiting_wire_caption` ->
    /// `ScreenGame::waiting_notice`. The exe reads no 3010 body bytes;
    /// the staged own seat's state word selects the caption.
    #[test]
    fn removal_cause_selects_a_distinct_caption_from_the_gateway() {
        let plain_caption = caption_for_own_seat_state(0);
        let no_song_caption = caption_for_own_seat_state(2);
        assert_ne!(
            no_song_caption, plain_caption,
            "the two 3010 causes must reach different removal captions"
        );
    }
}
/// Tier-2 scripted-gateway guards for the 2035/3006/3010/3014/4011
/// answer family (stories #1001, #1004, #1006, #1007, #1010).
mod answer_family {
    use std::io::Write;
    use std::sync::mpsc;

    use o2jam_net::message::{
        ExitWaitingAnswer, FreeMusicStatusAnswer, RoomClosedNotice, RoomDisappearedNotice,
        StartGameAnswer,
    };
    use o2jam_net::WireMessage;

    use super::super::session_tests::{
        joined_snapshot, joined_waiting_session, paint_frame, paint_until,
    };
    use super::super::*;

    /// One catalog row at level 20: the 2035 cap is the only way the
    /// auto-pick arm can reject it (the default ceiling is `u16::MAX`).
    fn level_twenty_catalog() -> Vec<u8> {
        let mut record = vec![0u8; 300];
        record[4..8].copy_from_slice(&757u32.to_le_bytes());
        record[8..11].copy_from_slice(b"ojn");
        record[20..24].copy_from_slice(&124f32.to_le_bytes());
        record[24..28].copy_from_slice(&20u32.to_le_bytes());
        record[112..117].copy_from_slice(b"First");
        record[176..180].copy_from_slice(b"Band");
        record[240..251].copy_from_slice(b"o2ma757.ojm");
        record
    }

    #[test]
    fn free_music_cap_from_2035_stops_the_auto_pick_row() {
        // Live chain: main.rs::run_play -> setup_online ->
        // connect::connect_and_login_with_transport -> assemble_online_session
        // -> OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_free_music_status ->
        // ScreenGame::set_auto_pick_level_cap -> auto_pick_waiting_song.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let answer = FreeMusicStatusAnswer {
                result: 0,
                level_cap: Some(7),
            };
            stream.write_all(&answer.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        let catalog = level_twenty_catalog();
        let entries = o2jam_file::parse_ojn_list(&catalog).unwrap();
        session.game.set_song_catalog(&entries);
        written.recv().unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(
            session.game.auto_pick_waiting_song(),
            AutoPickOutcome::Missed
        );
    }

    #[test]
    fn exit_answer_zero_returns_the_waiting_session_to_the_room_board() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_exit_waiting_answer ->
        // ScreenGame::enter_screen(Room) -> paint_room_list.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let answer = ExitWaitingAnswer { result: 0 };
            stream.write_all(&answer.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Room
        });
        server.join().unwrap();
        assert!(!session.joined_waiting, "the answer released the room");
        assert!(
            (0..8u8).all(|seat| session.game.staged_waiting_member(seat).is_none()),
            "all eight seats were released"
        );
    }

    #[test]
    fn a_refused_exit_answer_keeps_the_waiting_scene() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_exit_waiting_answer (non-zero).
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            stream
                .write_all(&ExitWaitingAnswer { result: 4 }.encode())
                .unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert!(session.joined_waiting, "a refused exit keeps the room");
    }

    #[test]
    fn room_closed_event_clears_the_waiting_seats() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_room_closed_notice ->
        // ScreenGame::clear_waiting_member_seat. The caption pair is a
        // shell hand-off: these bytes are pinned, not invented.
        const REMOVED_SONG: &str = "곡을 가지고 있지 않아 퇴장되었습니다.";
        const REMOVED_PLAIN: &str = "당신은 퇴장 당하셨습니다.";
        assert_eq!(REMOVED_SONG.chars().count(), 21);
        assert_eq!(REMOVED_PLAIN.chars().count(), 14);
        let (written_tx, written) = mpsc::channel();
        let mut room = joined_snapshot();
        room.slots[1].state_code = 2;
        room.slots[1].member = Some(o2jam_net::RoomMember {
            nickname: "Peer".to_owned(),
            level: 1,
            gender: 0,
            gem: 0,
            master: false,
            team: 1,
            ready: false,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        });
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream.write_all(&RoomClosedNotice.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(1).is_none()
        });
        server.join().unwrap();
    }

    #[test]
    fn room_disappeared_event_clears_the_waiting_seats() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_room_disappeared_notice ->
        // ScreenGame::clear_waiting_member_seat. The 0x64daa4 caption
        // "공연대기실이 사라졌습니다." needs a shell staging setter.
        let (written_tx, written) = mpsc::channel();
        let mut room = joined_snapshot();
        room.slots[1].state_code = 2;
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream.write_all(&RoomDisappearedNotice.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(1).is_none()
        });
        server.join().unwrap();
    }

    #[test]
    fn start_answer_zero_arms_the_loading_walk() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_start_game_answer ->
        // poll_start_at -> ScreenGame::force_advance_to_loading.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            stream
                .write_all(&StartGameAnswer { result: 0 }.encode())
                .unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });
        server.join().unwrap();
    }

    #[test]
    fn start_answer_refusal_keeps_the_waiting_scene() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_start_game_answer (non-zero).
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            stream
                .write_all(&StartGameAnswer { result: 1 }.encode())
                .unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }
}

/// Tier-2 scripted-gateway guards for the seat/state family (stories
/// #1003, #1005, #1009, #1011, #1012, #1015, #1016).
mod seat_state_family {
    use std::io::Write;
    use std::sync::mpsc;

    use o2jam_net::frame::encode_frame;
    use o2jam_net::message::{MusicLoaded, WaitingMemberRows, WaitingValueMemberEvent};
    use o2jam_net::play::MemberReadyStateChanged;
    use o2jam_net::rooms::{RoomMember, RoomSlotAction, RoomSlotUpdate, UserJoinedWaiting};
    use o2jam_net::WireMessage;

    use super::super::session_tests::{
        joined_snapshot, joined_waiting_session, paint_frame, paint_until,
    };
    use super::super::*;

    /// A joined room with peer members on the named seats, so the
    /// per-seat state mirrors have a member to update.
    fn room_with_peers(seats: &[u8]) -> o2jam_net::JoinedRoom {
        let mut room = joined_snapshot();
        for seat in seats {
            let mut member = joining_member();
            member.nickname = format!("Peer{seat}");
            room.slots[usize::from(*seat)].state_code = 2;
            room.slots[usize::from(*seat)].member = Some(member);
        }
        room
    }

    /// One staged member with the fields a 3004 body carries.
    fn joining_member() -> RoomMember {
        RoomMember {
            nickname: "Peer".to_owned(),
            level: 12,
            gender: 1,
            gem: 34,
            master: false,
            team: 1,
            ready: false,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        }
    }

    #[test]
    fn join_broadcast_seats_the_member_with_its_staged_fields() {
        // Live chain: main.rs::run_play -> setup_online ->
        // connect::connect_and_login_with_transport -> assemble_online_session
        // -> OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_user_joined ->
        // ScreenGame::seat_waiting_member.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let joined = UserJoinedWaiting {
                member_id: 1,
                member: joining_member(),
            };
            stream.write_all(&joined.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.staged_waiting_member(1).is_some()
        });
        server.join().unwrap();
        let staged = session.game.staged_waiting_member(1).unwrap();
        assert_eq!(staged.nickname, "Peer");
        assert_eq!(staged.level, 12);
        assert_eq!(staged.team, 1);
    }

    #[test]
    fn slot_update_keeps_the_kick_action_in_the_session_mirror() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_slot_update. The CP949 kick
        // caption at 0x64da3c needs a shell caption setter.
        const KICK_CAPTION: &str = "[정보] %s님이 강퇴 당하셨습니다.";
        assert!(KICK_CAPTION.ends_with("강퇴 당하셨습니다."));
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let update = RoomSlotUpdate {
                index: 3,
                action_code: 3,
            };
            stream.write_all(&update.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.slot_actions[3], Some(RoomSlotAction::PlayerKicked));
    }

    #[test]
    fn ready_echo_lights_the_named_seat() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_ready_echo ->
        // ScreenGame::set_member_ready.
        let (written_tx, written) = mpsc::channel();
        let mut room = joined_snapshot();
        room.slots[1].state_code = 2;
        room.slots[1].member = Some(joining_member());
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream
                .write_all(
                    &MemberReadyStateChanged {
                        member_id: 1,
                        ready: true,
                    }
                    .encode(),
                )
                .unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session
                .game
                .staged_waiting_member(1)
                .is_some_and(|member| member.ready)
        });
        server.join().unwrap();
        assert_eq!(session.member_ready(1), Some(true));
    }

    #[test]
    fn loaded_state_echo_stages_the_named_seat() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_music_loaded ->
        // ScreenGame::set_member_music_state (DOWNLOADING badge).
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) =
            joined_waiting_session(room_with_peers(&[2]), move |mut stream| {
                let loaded = MusicLoaded {
                    member_id: 2,
                    state: 4,
                };
                stream.write_all(&loaded.encode()).unwrap();
                written_tx.send(()).unwrap();
            });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.member_music_state(2) == Some(4)
        });
        server.join().unwrap();
    }

    #[test]
    fn counted_member_rows_stage_each_value_one_seat() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_waiting_member_rows ->
        // ScreenGame::set_member_music_state.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) =
            joined_waiting_session(room_with_peers(&[3]), move |mut stream| {
                let event = WaitingMemberRows {
                    rows: vec![
                        o2jam_net::message::WaitingMemberRow {
                            member_id: 3,
                            value: 1,
                            state: Some(4),
                        },
                        o2jam_net::message::WaitingMemberRow {
                            member_id: 4,
                            value: 2,
                            state: None,
                        },
                    ],
                };
                stream.write_all(&event.encode()).unwrap();
                written_tx.send(()).unwrap();
            });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.member_music_state(3) == Some(4)
        });
        server.join().unwrap();
        assert_eq!(session.game.member_music_state(4), None);
    }

    #[test]
    fn member_state_three_byte_body_stages_both_seat_words() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_waiting_chat ->
        // drain_waiting_chat -> apply_waiting_frame ->
        // apply_member_state_changed -> ScreenGame::set_member_music_state
        // + ScreenGame::seat_waiting_member (playing word).
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) =
            joined_waiting_session(room_with_peers(&[5]), move |mut stream| {
                stream.write_all(&encode_frame(4026, &[5, 1, 4])).unwrap();
                written_tx.send(()).unwrap();
            });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.member_music_state(5) == Some(4)
        });
        server.join().unwrap();
        let staged = session.game.staged_waiting_member(5).unwrap();
        assert_eq!(staged.playing_state, 1, "the +0x2130 playing word");
        assert_eq!(staged.music_state, 4, "the +0x217c music word");
    }

    #[test]
    fn waiting_value_member_pair_is_accepted_without_an_invented_stage() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_waiting_value_member. The
        // global's shell destination is a hand-off.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let event = WaitingValueMemberEvent {
                value: 12,
                member_id: 6,
            };
            stream.write_all(&event.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }
}

/// Tier-2 scripted-gateway guards for the key-mode, pick-echo and
/// skill-list family (stories #1002, #1008, #1013, #1014).
mod descriptor_and_skill_family {
    use std::io::Write;
    use std::sync::mpsc;
    use std::thread::sleep;
    use std::time::Duration;

    use o2jam_net::message::{SetRoomMusicEcho, WaitingSkillChanged, WaitingSkillExChanged};
    use o2jam_net::rooms::KeyModeChanged;
    use o2jam_net::Request;
    use o2jam_net::WireMessage;

    use super::super::session_tests::{
        joined_snapshot, joined_waiting_session, paint_frame, paint_until, read_frame,
    };
    use super::super::*;

    #[test]
    fn key_mode_change_queues_a_fresh_song_echo_for_the_master() {
        // Live chain: main.rs::run_play -> setup_online ->
        // connect::connect_and_login_with_transport -> assemble_online_session
        // -> OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_key_mode_changed ->
        // ScreenGame::apply_key_mode_change -> flush_music_send -> 4000.
        let mut room = joined_snapshot();
        room.music_id = 757;
        room.key_mode = 0;
        let (written_tx, written) = mpsc::channel();
        let (reported_tx, reported) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            let change = KeyModeChanged { key_mode: 2 };
            stream.write_all(&change.encode()).unwrap();
            written_tx.send(()).unwrap();
            let frame = read_frame(&mut stream);
            reported_tx.send(frame.id).unwrap();
        });
        written.recv().unwrap();
        for _ in 0..100 {
            paint_frame(&mut session);
            if let Ok(id) = reported.try_recv() {
                assert_eq!(id, Request::SetRoomMusic.id());
                break;
            }
            sleep(Duration::from_millis(1));
        }
        assert_eq!(session.game.screen(), Screen::Waiting);
        server.join().unwrap();
    }

    #[test]
    fn song_pick_echo_reannounces_the_staged_room_song() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_music_pick_echo ->
        // announce_music_state -> 4025.
        let mut room = joined_snapshot();
        room.music_id = 757;
        let (written_tx, written) = mpsc::channel();
        let (reported_tx, reported) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream
                .write_all(&SetRoomMusicEcho { member_id: 1 }.encode())
                .unwrap();
            written_tx.send(()).unwrap();
            let frame = read_frame(&mut stream);
            reported_tx.send(frame.id).unwrap();
        });
        written.recv().unwrap();
        for _ in 0..100 {
            paint_frame(&mut session);
            if let Ok(id) = reported.try_recv() {
                assert_eq!(id, Request::UpdateMusicState.id());
                break;
            }
            sleep(Duration::from_millis(1));
        }
        server.join().unwrap();
    }

    #[test]
    fn skill_list_stages_the_wire_item_ids_in_the_first_slot() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_waiting_skill_list ->
        // ScreenGame::set_skill_slot_items.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let event = WaitingSkillChanged {
                item_ids: vec![11, 22],
                value: 7,
                member_id: 1,
            };
            stream.write_all(&event.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.skill_slots().selected(0).is_some()
        });
        server.join().unwrap();
        assert_eq!(session.game.skill_slots().selected(0).unwrap().id, 11);
    }

    #[test]
    fn extended_skill_list_stages_the_wire_item_ids_in_the_second_slot() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_waiting_chat -> drain_waiting_chat
        // -> apply_waiting_frame -> apply_waiting_skill_ex_list ->
        // ScreenGame::set_skill_slot_items.
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            let event = WaitingSkillExChanged {
                item_ids: vec![33],
                value: 9,
                member_id: 2,
            };
            stream.write_all(&event.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session.game.skill_slots().selected(1).is_some()
        });
        server.join().unwrap();
        assert_eq!(session.game.skill_slots().selected(1).unwrap().id, 33);
    }
}

/// Tier-2 scripted-gateway guards for the 5027 peer music-list family
/// (story #1017). The caption read-back stays at the shell seam: the
/// Waiting `MUSICNAME` caption follows the room song, so a peer's list
/// must not move it.
mod music_list_family {
    use std::io::Write;
    use std::sync::mpsc;

    use o2jam_net::frame::encode_frame;

    use super::super::session_tests::{
        joined_snapshot, joined_waiting_session, member_on_team, paint_until,
    };

    // ev_c129c15e1d0b02f2c35a4032d00f81ce2b3de6336c433efbe78d5439ce250068
    // (0x563880 / file 0x163880, docs/pins/net-c1.md).
    #[test]
    fn peer_music_list_lands_on_the_named_seat_and_leaves_our_caption_alone() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_waiting_chat ->
        // drain_waiting_chat -> apply_waiting_frame ->
        // apply_member_music_list -> ScreenGame::seat_waiting_member ->
        // paint_waiting_music (room caption source).
        let mut room = joined_snapshot();
        room.music_id = 757;
        room.slots[0].member = Some(member_on_team("HOST", true, 0));
        room.slots[2].state_code = 2;
        room.slots[2].member = Some(member_on_team("PEER", false, 1));
        let mut body = vec![2u8];
        body.extend_from_slice(&2i32.to_le_bytes());
        body.extend_from_slice(&757u16.to_le_bytes());
        body.extend_from_slice(&900u16.to_le_bytes());
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream.write_all(&encode_frame(5027, &body)).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session
                .game
                .staged_waiting_member(2)
                .is_some_and(|member| member.music_ids.len() == 2)
        });
        server.join().unwrap();
        assert_eq!(
            session.game.staged_waiting_member(2).unwrap().music_ids,
            vec![757, 900],
            "the 0x540db0 row belongs to the named peer seat"
        );
        assert!(
            session
                .game
                .staged_waiting_member(0)
                .is_some_and(|member| member.music_ids.is_empty()),
            "our own row keeps its list"
        );
        assert_eq!(
            session.game.room_music_id(),
            Some(757),
            "the Waiting caption keeps the room song"
        );
    }
}

/// Tier-2 scripted-gateway guard for the 4043 live-state drain arm
/// (story #956). The team byte reaches the peer seat with the exe's
/// `+1` store increment (`0x540d40`, file `0x140d40`).
mod live_state_team_family {
    use std::io::Write;
    use std::sync::mpsc;

    use o2jam_net::relay::{LiveStateResponse, MemberInfo, MemberLiveState};
    use o2jam_net::WireMessage;

    use super::super::session_tests::{
        joined_snapshot, joined_waiting_session, member_on_team, paint_until,
    };

    /// One active peer with raw wire team 3 and all other fields empty.
    fn live_snapshot() -> LiveStateResponse {
        let mut members = std::array::from_fn(|_| MemberLiveState::default());
        members[1] = MemberLiveState {
            active: true,
            info: Some(MemberInfo {
                member_id: 1,
                nickname: "WIREPEER".to_owned(),
                level: 12,
                gender: 0,
                gem: 34,
                team: 3,
                ready: false,
                music_state: 0,
                equipments: [0; 17],
                music_ids: vec![757],
                cash_point: 0,
                free_pass: 0,
                playing_state: 0,
                live_extra: 0,
                is_room_master: false,
                win_streak: 0,
            }),
        };
        LiveStateResponse {
            member_id: 0,
            user_count: 2,
            win_streak: 0,
            members,
        }
    }

    #[test]
    fn live_state_4043_lands_the_incremented_team_on_the_named_peer() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_waiting_chat ->
        // drain_waiting_chat -> apply_waiting_frame -> apply_live_state ->
        // ScreenGame::seat_waiting_member -> paint_waiting_seat_marks.
        let mut room = joined_snapshot();
        room.slots[1].state_code = 2;
        room.slots[1].member = Some(member_on_team("PEER", false, 0));
        let snapshot = live_snapshot();
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            stream.write_all(&snapshot.encode()).unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            session
                .game
                .staged_waiting_member(1)
                .is_some_and(|member| member.team == 4)
        });
        server.join().unwrap();
        let staged = session.game.staged_waiting_member(1).unwrap();
        assert_eq!(staged.team, 4, "wire team 3 stores as 4 in the seat");
        assert_eq!(staged.music_ids, vec![757], "the same row's music list");
    }
}

/// Tier-2 scripted-gateway guard for the 2026 premium-time handler
/// (story #962). The exact description text stays OPEN; the row format
/// and bitmask line counts are pinned in `o2jam-net::relay`.
mod premium_time_family {
    use std::io::Write;
    use std::sync::mpsc;

    use o2jam_net::frame::encode_frame;
    use o2jam_net::relay::PREMIUM_TIME_COMMAND;

    use super::super::session_tests::{channel_session, paint_until};

    #[test]
    fn premium_time_2026_stages_its_rows_on_the_scripted_gateway() {
        // Live chain: main.rs::run_online -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_lobby_chat ->
        // drain_lobby_chat -> apply_lobby_frame -> apply_premium_time_list.
        let mut body = 2u32.to_le_bytes().to_vec();
        body.extend_from_slice(&0x1234u16.to_le_bytes());
        body.extend_from_slice(&[1, 2, 3, 4, 5]);
        body.extend_from_slice(&0x0807u16.to_le_bytes());
        body.extend_from_slice(&[8, 9, 10, 0, 0]);
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = channel_session(move |mut stream| {
            stream
                .write_all(&encode_frame(PREMIUM_TIME_COMMAND, &body))
                .unwrap();
            written_tx.send(()).unwrap();
        });
        written.recv().unwrap();
        paint_until(&mut session, |session| {
            !session.premium_time_rows.is_empty()
        });
        server.join().unwrap();
        assert_eq!(session.premium_time_rows.len(), 2);
        assert_eq!(session.premium_time_rows[0].song_id, 0x1234);
        assert_eq!(session.premium_time_rows[0].status, [1, 2, 3, 4, 5]);
        assert_eq!(session.premium_time_rows[1].song_id, 0x0807);
    }
}
