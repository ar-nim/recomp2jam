//! Playing/battle wire seams and scripted-gateway guards (#293 wire
//! stories). Declared `#[cfg(test)]` by `session.rs`, so `use super::*;`
//! reaches the crate-private session items the drain paths live on.
//!
//! The shell has no generic UI-event queue or relay endpoint table, so
//! the body-ignored forwards are observed through a thread-local
//! [`PlayingWireObservation`] log at the session's wire seam. That log is
//! test-only instrumentation, never a runtime mirror: production decode
//! stays side-effect free, so a long session cannot grow it. The shell
//! consumer of those forwards stays OPEN and is named in the lane report.

#[cfg(test)]
use std::cell::RefCell;

use o2jam_net::play::ScoreEntry;
use o2jam_net::relay::P2PListResponse;
use o2jam_net::{Frame, WireMessage};
use o2jam_shell::screens::game::ResultMemberRecord;
use o2jam_shell::ScreenGame;

/// One observable step of a Playing wire frame at the session seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PlayingWireObservation {
    /// The `(category, opcode, slot, flag)` tuple a body-ignored forward posts.
    UiEvent((u32, u16, u8, u32)),
    /// The 4045 endpoint table decoded from a peer-list frame.
    PeerList(P2PListResponse),
    /// The five 4049 latency words.
    Latency([u32; 5]),
    /// Both 4050 speed writes plus the pinned flag set behind them.
    SpeedWrites([u8; 2], bool),
}

#[cfg(test)]
thread_local! {
    static OBSERVATIONS: RefCell<Vec<PlayingWireObservation>> = const { RefCell::new(Vec::new()) };
}

/// Clear this thread's wire log before one scripted-gateway guard.
#[cfg(test)]
pub(super) fn clear_observations() {
    OBSERVATIONS.with(|log| log.borrow_mut().clear());
}

/// Record one wire step in tests. Production drops the observation:
/// the shell has no event sink, and an unbounded log would leak.
pub(super) fn observe(observation: PlayingWireObservation) {
    #[cfg(test)]
    OBSERVATIONS.with(|log| log.borrow_mut().push(observation));
    #[cfg(not(test))]
    let _ = observation;
}

/// Take this thread's log so a guard can assert its exact sequence.
#[cfg(test)]
pub(super) fn take_observations() -> Vec<PlayingWireObservation> {
    OBSERVATIONS.with(|log| log.borrow_mut().drain(..).collect())
}

/// Write both 4050 speed bytes into the self result record. The exe
/// writes each read byte through the same `+0x2168` staging helper; the
/// final word therefore holds the second byte. Only the speed word is
/// pinned by 4050, so every other record word starts cleared.
pub(super) fn stage_self_speed_writes(game: &mut ScreenGame, member_id: u8, writes: [u8; 2]) {
    for speed in writes {
        let entry = ScoreEntry {
            member_id,
            active: true,
            cool: 0,
            good: 0,
            bad: 0,
            miss: 0,
            max_combo: 0,
            jam: 0,
            score: 0,
            reward: 0,
            level: 0,
            experience: 0,
            result: o2jam_net::play::MatchResult::Draw,
            gem: 0,
            cash_point: 0,
            speed,
            long_note_score: 0,
        };
        game.stage_result_member_record(
            member_id,
            ResultMemberRecord::from_score_entry(&entry, member_id),
        );
    }
}

/// Decode a frame whose opcode is already pinned by its caller, for
/// match arms that must not read a body when the handler does not.
pub(super) fn decode_playing_frame<T: WireMessage>(frame: &Frame) -> Option<T> {
    T::decode_frame(frame).ok()
}

#[cfg(test)]
mod wire_guards {
    use std::io::Write;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::JoinHandle;
    use std::time::Duration;

    use o2jam_net::frame::encode_frame;
    use o2jam_net::play::{MatchResult, ScoreEntry};
    use o2jam_net::relay::{LatencyAnswer, P2PListResponse, PeerInfo, PlayingSpeedEvent};
    use o2jam_net::shop::{MusicEntry, SyncMusicPurchaseResponse};
    use o2jam_net::{JoinedRoom, Request};
    use o2jam_render::Rect;
    use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
    use o2jam_shell::screens::game::ResultMemberRecord;
    use o2jam_shell::screens::side::{SideScreen, MSHOP_CART_BUY_CODE, MSHOP_EX_FIRST_CODE};
    use o2jam_shell::MenuKey;

    use super::super::session_tests::{
        joined_snapshot, joined_waiting_session, paint_frame, paint_until, read_frame,
    };
    use super::super::*;
    use super::{clear_observations, take_observations, PlayingWireObservation};

    /// The pinned `+0x2150` default words and cleared per-round fields
    /// from the `0x4d8940` constructor, as a fresh-round model.
    #[derive(Debug, PartialEq, Eq)]
    struct PinnedPlayingRound {
        cleared_round_fields: [u32; 4],
        first_315_word: u32,
        second_315_word: u32,
        pinned_375_word: u32,
        pinned_400_word: u32,
        live_mode_byte: u8,
        initial_flags: bool,
    }

    impl PinnedPlayingRound {
        fn fresh(live_mode_byte: u8) -> Self {
            Self {
                cleared_round_fields: [0; 4],
                first_315_word: 315,
                second_315_word: 315,
                pinned_375_word: 375,
                pinned_400_word: 400,
                live_mode_byte,
                initial_flags: true,
            }
        }
    }

    /// A joined session already in `Playing`, with the scripted frames
    /// written after the 4011/4012/4013 loading handoff.
    fn playing_gateway_session(
        room: JoinedRoom,
        script: impl FnOnce(&mut std::net::TcpStream) + Send + 'static,
    ) -> (OnlineSession, JoinHandle<()>, Receiver<()>) {
        let (written_tx, written) = mpsc::channel();
        let (mut session, server) = joined_waiting_session(room, move |mut stream| {
            assert_eq!(read_frame(&mut stream).id, Request::StartGame.id());
            stream.write_all(&encode_frame(4011, &[])).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ConfirmMusicLoaded.id());
            stream.write_all(&encode_frame(4013, &[1, 1])).unwrap();
            script(&mut stream);
            written_tx.send(()).unwrap();
        });
        session.menu_key(MenuKey::Confirm, true);
        paint_until(&mut session, |session| {
            session.game.screen() == Screen::Loading
        });
        session.menu_key(MenuKey::Confirm, true);
        assert_eq!(session.game.screen(), Screen::Playing);
        clear_observations();
        (session, server, written)
    }

    /// Two occupied seats in a host room, so 4035 can stage a state
    /// on each of them. The joined snapshot alone has empty seats.
    fn seated_room() -> JoinedRoom {
        let mut room = joined_snapshot();
        let member = |nickname: &str, team: u8| o2jam_net::RoomMember {
            nickname: nickname.to_owned(),
            level: 1,
            gender: 0,
            gem: 0,
            master: false,
            team,
            ready: false,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        };
        room.slots[0].member = Some(member("self", 0));
        room.slots[1].member = Some(member("peer", 1));
        room.user_count = 2;
        room
    }

    /// Two-row synthetic OJN catalog: only the ids the 5011 guard needs.
    fn catalog_bytes(ids: &[u32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for id in ids {
            let mut record = vec![0u8; 300];
            record[4..8].copy_from_slice(&id.to_le_bytes());
            record[8..11].copy_from_slice(b"ojn");
            record[20..24].copy_from_slice(&120f32.to_le_bytes());
            record[24..28].copy_from_slice(&5u32.to_le_bytes());
            record[112..117].copy_from_slice(b"First");
            record[176..180].copy_from_slice(b"Band");
            record[240..251].copy_from_slice(format!("o2ma{id}.ojm").as_bytes());
            bytes.extend_from_slice(&record);
        }
        bytes
    }

    #[test]
    fn result_speed_slot_feeds_the_local_rank_row() {
        // seam: the self member `+0x2168` word is the rank row's speed
        // source (VA `0x563fe0`, file `0x163fe0`, ev
        // `7689.1c2f…693d`; row painter `docs/cresult-anchors.md`).
        let mut game = ScreenGame::new();
        game.set_joined_room(joined_snapshot());
        let entry = ScoreEntry {
            member_id: 0,
            active: true,
            cool: 0,
            good: 0,
            bad: 0,
            miss: 0,
            max_combo: 0,
            jam: 0,
            score: 0,
            reward: 0,
            level: 0,
            experience: 0,
            result: MatchResult::Draw,
            gem: 0,
            cash_point: 0,
            speed: 9,
            long_note_score: 0,
        };
        game.stage_result_member_record(0, ResultMemberRecord::from_score_entry(&entry, 0));
        assert_eq!(game.result_rank_rows()[0].speed, Some(9));
    }

    #[test]
    fn playing_fresh_round_starts_from_pinned_defaults() {
        // seam: the constructor's cleared fields and pinned defaults are
        // the whole behavior (VA `0x4d8940`, file `0xd8940`,
        // ev `8ad9.b71b…c2d3.e8b7`; callers 0x4eb610/0x4e4670/0x4f07f0).
        let previous_round = PinnedPlayingRound {
            cleared_round_fields: [9, 7, 5, 3],
            first_315_word: 12,
            second_315_word: 88,
            pinned_375_word: 1,
            pinned_400_word: 2,
            live_mode_byte: 0,
            initial_flags: false,
        };
        let fresh = PinnedPlayingRound::fresh(1);
        assert_eq!(fresh.cleared_round_fields, [0; 4]);
        assert_eq!(fresh.first_315_word, 315);
        assert_eq!(fresh.second_315_word, 315);
        assert_eq!(fresh.pinned_375_word, 375);
        assert_eq!(fresh.pinned_400_word, 400);
        assert_eq!(fresh.live_mode_byte, 1, "mode comes from the live global");
        assert!(fresh.initial_flags);
        assert_ne!(
            fresh, previous_round,
            "a new round cannot inherit the old one"
        );
    }

    #[test]
    fn acquire_music_4035_stages_every_named_member_and_arms_the_local_badge() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_waiting_chat ->
        // drain_waiting_chat -> apply_waiting_frame -> apply_acquire_music
        // -> stage_member_music_state -> ScreenGame::set_member_music_state
        // + ScreenGame::arm_local_download_badge -> begin_music_download.
        let (mut session, server) = joined_waiting_session(seated_room(), |mut stream| {
            // Count 2: local member 0 is Downloading, member 1 is Ready.
            stream
                .write_all(&encode_frame(4035, &[2, 0, 4, 1, 0]))
                .unwrap();
        });
        session.game.set_room_music(757, 0, 1);
        paint_until(&mut session, |session| {
            session.game.local_download_badge_armed()
        });
        server.join().unwrap();
        assert_eq!(session.game.member_music_state(0), Some(4));
        assert_eq!(session.game.member_music_state(1), Some(0));
        assert_eq!(session.game.music_download().requests()[0].song_id, 757);
    }

    #[test]
    fn sync_music_purchase_5011_rebuilds_the_owned_rows_from_the_gateway() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_waiting_chat ->
        // drain_waiting_chat -> apply_waiting_frame -> 5011 arm ->
        // ScreenGame::apply_music_purchase_response ->
        // MusicShopState::fold_purchased_songs -> owned shop rows.
        let purchase = SyncMusicPurchaseResponse {
            gem: 4242,
            point: 250,
            o2_cash: 5000,
            music_cash: 120,
            item_cash: 340,
            cash_point: 56,
            music_list: vec![
                MusicEntry {
                    music_id: 757,
                    day: 14,
                    month: 3,
                    year: 25,
                    hour: 18,
                    minute: 45,
                },
                MusicEntry {
                    music_id: 123,
                    day: 1,
                    month: 2,
                    year: 25,
                    hour: 6,
                    minute: 7,
                },
            ],
            gift_boxes_raw: Vec::new(),
            free_pass_extension_days: 0,
        };
        let (mut session, server) = joined_waiting_session(joined_snapshot(), move |mut stream| {
            stream.write_all(&purchase.encode()).unwrap();
        });
        let catalog = catalog_bytes(&[757, 123, 999]);
        let entries = o2jam_file::parse_ojn_list(&catalog).unwrap();
        assert_eq!(entries.len(), 3, "the synthetic catalog has three rows");
        session.game.set_song_catalog(&entries);
        let area = |code: u32, x: i32, y: i32, w: u32, h: u32| ScenePlacement {
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
            role: PlacementRole::Single,
        };
        session.game.load_side_scene(
            vec![
                area(MSHOP_EX_FIRST_CODE, 18, 104, 353, 86),
                area(MSHOP_CART_BUY_CODE, 423, 528, 49, 23),
            ],
            std::collections::HashMap::new(),
        );
        assert!(session.game.visit_side(SideScreen::MusicShop));
        // The old owned row must be dropped by the rebuild, not kept.
        session
            .game
            .apply_music_purchase_response(&SyncMusicPurchaseResponse {
                gem: 1,
                point: 0,
                o2_cash: 0,
                music_cash: 0,
                item_cash: 0,
                cash_point: 0,
                music_list: vec![MusicEntry {
                    music_id: 999,
                    day: 1,
                    month: 1,
                    year: 25,
                    hour: 1,
                    minute: 1,
                }],
                gift_boxes_raw: Vec::new(),
                free_pass_extension_days: 0,
            });
        paint_until(&mut session, |session| {
            session.game.shop_gem_caption() == "GEM 4242"
        });
        server.join().unwrap();
        let owned: Vec<u32> = session
            .game
            .music_shop_visible_rows()
            .iter()
            .filter(|row| row.listed)
            .map(|row| row.song.song_id)
            .collect();
        assert!(
            owned.contains(&757) && owned.contains(&123),
            "the 5011 answer must own both acquired songs"
        );
        assert_eq!(session.game.shop_gem_caption(), "GEM 4242");
        // The exe clears both counted lists first; our shell fold only
        // adds ids, so dropping the previously owned 999 stays OPEN.
    }

    #[test]
    fn body_ignored_4052_4054_4056_each_post_one_distinct_event_from_poll_wire() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_stats ->
        // drain_peer_echoes -> 4052/4054/4056 body-ignored arms ->
        // (0x12, opcode, 0, 0) UI forward. Shell event sink: OPEN.
        let (mut session, server, written) = playing_gateway_session(joined_snapshot(), |stream| {
            for (id, body) in [
                (4052, Vec::new()),
                (4052, vec![7u8, 0x33, 0xaa]),
                (4054, Vec::new()),
                (4054, vec![9, 9, 9, 9]),
                (4056, Vec::new()),
                (4056, vec![0xff; 6]),
            ] {
                stream.write_all(&encode_frame(id, &body)).unwrap();
            }
        });
        written.recv_timeout(Duration::from_secs(5)).unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        let observed = take_observations();
        assert_eq!(observed.len(), 6, "one event per frame, empty or padded");
        assert_eq!(
            observed[0], observed[1],
            "4052 padding cannot change the gate event"
        );
        assert_eq!(
            observed[2], observed[3],
            "4054 padding cannot change the forward event"
        );
        assert_eq!(
            observed[4], observed[5],
            "4056 padding cannot change the relay-confirm event"
        );
        let forwarded: Vec<u16> = observed
            .iter()
            .map(|step| match step {
                PlayingWireObservation::UiEvent((_, id, _, _)) => *id,
                other => panic!("unexpected wire step: {other:?}"),
            })
            .collect();
        assert_eq!(forwarded, [4052, 4052, 4054, 4054, 4056, 4056]);
    }

    #[test]
    fn p2p_list_4045_two_peer_frame_stages_signed_count_and_both_endpoints() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_stats ->
        // drain_peer_echoes -> 4045 arm -> `P2PListResponse` endpoint
        // table. The shell has no relay dialer; that leg stays OPEN.
        let response = P2PListResponse {
            peers: vec![
                PeerInfo {
                    member_id: -2,
                    public_endpoint: SocketAddrV4::new(Ipv4Addr::new(203, 0, 113, 5), 4000),
                    local_endpoint: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 5), 4001),
                },
                PeerInfo {
                    member_id: 3,
                    public_endpoint: SocketAddrV4::new(Ipv4Addr::new(203, 0, 113, 6), 4002),
                    local_endpoint: SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 6), 4003),
                },
            ],
        };
        let written = response.clone();
        let (mut session, server, written) =
            playing_gateway_session(joined_snapshot(), move |stream| {
                stream.write_all(&written.encode()).unwrap();
            });
        written.recv_timeout(Duration::from_secs(5)).unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(
            take_observations(),
            vec![PlayingWireObservation::PeerList(response)]
        );
    }

    #[test]
    fn latency_answer_4049_five_words_reach_the_peer_timing_stage() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_stats ->
        // drain_peer_echoes -> 4049 arm -> `LatencyAnswer` five-word
        // stage. The exe's per-member timing slot consumer stays OPEN.
        let answer = LatencyAnswer {
            words: [0x11, 0x2233, 0x4455_6677, 0x8899_aabb, 0xccdd_eeff],
        };
        let mut non_matching = answer;
        non_matching.words[0] = 1;
        non_matching.words[4] = 0;
        let (mut session, server, written_ack) =
            playing_gateway_session(joined_snapshot(), move |stream| {
                stream.write_all(&answer.encode()).unwrap();
                stream.write_all(&non_matching.encode()).unwrap();
            });
        written_ack.recv_timeout(Duration::from_secs(5)).unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(
            take_observations(),
            vec![
                PlayingWireObservation::Latency(answer.words),
                PlayingWireObservation::Latency(non_matching.words)
            ],
            "the matching and non-matching member words must stage all five fields"
        );
    }

    #[test]
    fn playing_speed_4050_two_writes_land_in_the_self_result_slot() {
        // Live chain: main.rs::run_play -> assemble_online_session ->
        // OnlineSession::paint -> poll_wire_at -> poll_stats ->
        // drain_peer_echoes -> 4050 arm -> `stage_self_speed_writes`
        // (both `+0x2168` writes) -> `ResultRankRow::speed` paint seam.
        let event = PlayingSpeedEvent {
            first_speed: 3,
            final_speed: 9,
        };
        let written = event;
        let (mut session, server, written) =
            playing_gateway_session(joined_snapshot(), move |stream| {
                stream.write_all(&written.encode()).unwrap();
            });
        written.recv_timeout(Duration::from_secs(5)).unwrap();
        paint_frame(&mut session);
        server.join().unwrap();
        assert_eq!(
            take_observations(),
            vec![PlayingWireObservation::SpeedWrites([3, 9], true)]
        );
        assert_eq!(
            session.game.result_rank_rows()[0].speed,
            Some(9),
            "the second byte is the last write into the self speed slot"
        );
        // The local row keeps its pinned matching verdict shape.
        assert_eq!(
            session.game.result_rank_rows()[0].judgements,
            Some([0, 0, 0, 0])
        );
    }
}
