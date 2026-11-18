//! One owner for press meaning (issue #258).
//!
//! The session's press gate used to re-derive four staged hit tests
//! and echo the resulting input into the game from two places (the
//! resident arms and the poll resolvers). Here a single module reads
//! one [`PressKind`] word owned by the game, returns one
//! [`PressDisposition`] decision, and applies one [`EntryEcho`]. Every
//! staged rect stays owned by the scene that paints it (ADR-0003/0013):
//! the four queries are private to `ScreenGame` behind that word.

use super::*;
use o2jam_shell::screens::game::PressKind;

/// How the session handles one pointer edge: eaten by the gate or
/// passed through to the game. The resident echo's matching release
/// is recorded on the session, not on this decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressDisposition {
    /// Edge eaten; the game never sees it.
    Swallow,
    /// Edge reaches the game unchanged.
    Local,
}

/// The one echo shape: a resident press replays immediately, a gated
/// entry replays after its poll resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryEcho {
    /// Replay `menu_key(Confirm)`.
    Confirm,
    /// Replay `click_at(x, y)`.
    Press { x: i32, y: i32 },
}

/// Which matching release a resident echo consumed (recorded on the
/// session so no two gates own the same press/release pair).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PressEcho {
    Channel { x: i32, y: i32 },
    Room { x: i32, y: i32 },
}

impl PressEcho {
    /// Whether this echo belongs to a channel face.
    fn is_channel(self) -> bool {
        matches!(self, Self::Channel { .. })
    }

    /// Whether this echo owns the given release point.
    fn matches(self, x: i32, y: i32) -> bool {
        match self {
            Self::Channel {
                x: echo_x,
                y: echo_y,
            }
            | Self::Room {
                x: echo_x,
                y: echo_y,
            } => (echo_x, echo_y) == (x, y),
        }
    }
}

impl OnlineSession {
    /// Apply the single echo for one accepted entry: a resident face,
    /// a poll-replayed press, or a resident/no-row Confirm. A gated
    /// pointer edge is swallowed by the caller, so this is the only
    /// path that forwards an entry into `ScreenGame`.
    pub(crate) fn echo_entry(&mut self, echo: EntryEcho) {
        match echo {
            EntryEcho::Confirm => self.game.menu_key(MenuKey::Confirm, true),
            EntryEcho::Press { x, y } => {
                self.game.ingest_snapshot(&InputSnapshot::click_at(x, y));
            }
        }
    }

    /// Handle one pointer edge from the session's snapshot sweep:
    /// return its disposition and queue the wire work the decision
    /// implies (the edge itself never reaches `ScreenGame` here).
    pub(crate) fn dispatch_pointer_edge(&mut self, edge: &InputEdge) -> PressDisposition {
        match *edge {
            InputEdge::PointerDown { x, y } => self.dispatch_pointer_down(x, y),
            InputEdge::PointerUp { x, y } => self.dispatch_pointer_up(x, y),
            _ => PressDisposition::Local,
        }
    }

    /// The matching release of a resident echo is eaten; every other
    /// release (a scroll drag, an ungated press) passes through.
    fn dispatch_pointer_up(&mut self, x: i32, y: i32) -> PressDisposition {
        if let Some(echo) = self.resident_echo {
            if echo.matches(x, y) {
                self.resident_echo = None;
                return PressDisposition::Swallow;
            }
        }
        // Pre-refactor gate behavior: the Waiting confirm face ran
        // `confirm_waiting` on both edges of a click. The golden
        // sequences pin that quirk; changing it belongs to a behavior
        // ticket, not to this pure move.
        if self.game.press_kind_at(x, y) == PressKind::WaitingConfirm {
            self.confirm_waiting();
            return PressDisposition::Swallow;
        }
        PressDisposition::Local
    }

    /// Resolve one press against the game's single press word.
    fn dispatch_pointer_down(&mut self, x: i32, y: i32) -> PressDisposition {
        match self.game.press_kind_at(x, y) {
            PressKind::ChannelEnter(channel) => self.dispatch_channel_press(channel, x, y),
            PressKind::RoomRow { number, title } => self.dispatch_room_press(&title, number, x, y),
            PressKind::WaitingConfirm => {
                self.confirm_waiting();
                PressDisposition::Swallow
            }
            PressKind::Local => self.dispatch_local_press(),
        }
    }

    /// A resident channel face replays locally; a non-resident one
    /// queues the login for the poll's echo.
    fn dispatch_channel_press(
        &mut self,
        channel: ChannelState,
        x: i32,
        y: i32,
    ) -> PressDisposition {
        if self.is_resident_channel(&channel) {
            self.resident_echo = Some(PressEcho::Channel { x, y });
            self.echo_entry(EntryEcho::Press { x, y });
            return PressDisposition::Swallow;
        }
        self.begin_channel_login(channel, EntryEcho::Press { x, y });
        PressDisposition::Swallow
    }

    /// A resident room row replays locally (title heuristic: the
    /// joined snapshot carries no wire room id, ticket #231); a
    /// non-resident one stages its number once, then queues the join.
    fn dispatch_room_press(
        &mut self,
        title: &str,
        number: i32,
        x: i32,
        y: i32,
    ) -> PressDisposition {
        if self.resident_room.as_deref() == Some(title) {
            self.resident_echo = Some(PressEcho::Room { x, y });
            self.echo_entry(EntryEcho::Press { x, y });
            return PressDisposition::Swallow;
        }
        self.set_pending_room_join(number, "");
        self.begin_room_join(EntryEcho::Press { x, y });
        PressDisposition::Swallow
    }

    /// A miss on Room is swallowed only while a join is pending;
    /// everywhere else it stays local.
    fn dispatch_local_press(&self) -> PressDisposition {
        if self.game.screen() == Screen::Room && self.pending_room.is_some() {
            return PressDisposition::Swallow;
        }
        PressDisposition::Local
    }

    /// Forget a resident echo when its channel residency clears
    /// (leave, timeout) so no stale release is swallowed.
    pub(crate) fn clear_channel_echo(&mut self) {
        if self.resident_echo.is_some_and(PressEcho::is_channel) {
            self.resident_echo = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::session_tests::{
        answer_entry_login, channel_session, entry_channel, guest_snapshot, joined_snapshot,
        joined_waiting_session, paint_frame, paint_until, read_frame, room_session,
    };
    use o2jam_shell::{GameLoop, InputSnapshot, MenuKey, Screen};
    use std::io::Write;

    /// One listed room row for the golden press-sequence guards.
    fn listed_room(number: i32, title: &str) -> o2jam_net::RoomListEntry {
        o2jam_net::RoomListEntry {
            number,
            state_code: 1,
            title: title.to_owned(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 5,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        }
    }

    /// Record one point in a press scenario: screen, observable hash,
    /// and the session words that name the pending attempt and the
    /// single echo's matching release. Same shape for every guard, so
    /// the golden sequence exposes a duplicated or missing echo.
    ///
    /// Once the 2001 identity stages, the hash includes installed
    /// avatar data; CI hides `install-root`. The press sequence is
    /// what is pinned here, so those frames carry a marker instead
    /// of a data-dependent number.
    fn press_trace(session: &OnlineSession, label: &str, out: &mut Vec<String>) {
        let hash = if session.game.myroom_character().is_some() {
            "<staged-identity>".to_owned()
        } else {
            format!("{:016x}", session.state_hash())
        };
        out.push(format!(
            "{label}|{}|{hash}|login={}|join={}|echo={:?}|ready={}",
            session.game.screen_name(),
            session.login_attempt.is_some(),
            session.join_attempt.is_some(),
            session.resident_echo,
            session.guest_ready
        ));
    }

    fn assert_golden_press_sequence(trace: &[String], golden: &[&str]) {
        let expected: Vec<String> = golden.iter().map(|line| (*line).to_owned()).collect();
        assert_eq!(trace, expected, "golden press sequence");
    }

    #[test]
    fn channel_enter_face_golden_press_sequence_echoes_once() {
        // Live chain: main.rs::run_play -> window snapshot ->
        // GameLoop::ingest_snapshot -> OnlineSession::ingest_snapshot
        // -> press dispatch -> ScreenGame::ingest_snapshot.
        let (mut session, server) = channel_session(|mut stream| {
            answer_entry_login(&mut stream);
        });
        session.game.set_channels(vec![entry_channel()]);
        let mut trace = Vec::new();
        press_trace(&session, "start", &mut trace);
        session.ingest_snapshot(&InputSnapshot::press_at(524, 246));
        press_trace(&session, "down", &mut trace);
        session.ingest_snapshot(&InputSnapshot::release_at(524, 246));
        press_trace(&session, "up", &mut trace);
        paint_until(&mut session, |s| s.game.screen() == Screen::Room);
        press_trace(&session, "poll", &mut trace);
        server.join().unwrap();
        assert_golden_press_sequence(
            &trace,
            &[
                "start|channel|1b56fe73a4404ccc|login=false|join=false|echo=None|ready=false",
                "down|channel|1b56fe73a4404ccc|login=true|join=false|echo=None|ready=false",
                "up|channel|1b56fe73a4404ccc|login=true|join=false|echo=None|ready=false",
                "poll|room|<staged-identity>|login=false|join=false|echo=None|ready=false",
            ],
        );
    }

    #[test]
    fn resident_channel_face_golden_press_sequence_echoes_once_without_wire() {
        // Live chain: window snapshot -> OnlineSession::ingest_snapshot
        // -> resident press dispatch -> ScreenGame::ingest_snapshot.
        use std::io::Read;
        let (mut session, server) = channel_session(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(
                stream.read_exact(&mut header).is_err(),
                "resident face press sent a frame on the wire"
            );
        });
        session.game.set_channels(vec![entry_channel()]);
        session.set_resident_channel(entry_channel());
        let mut trace = Vec::new();
        press_trace(&session, "start", &mut trace);
        session.ingest_snapshot(&InputSnapshot::press_at(524, 246));
        press_trace(&session, "down", &mut trace);
        session.ingest_snapshot(&InputSnapshot::release_at(524, 246));
        press_trace(&session, "up", &mut trace);
        server.join().unwrap();
        assert_golden_press_sequence(
            &trace,
            &[
                "start|channel|1b56fe73a4404ccc|login=false|join=false|echo=None|ready=false",
                "down|room|44189648edbbdbcb|login=false|join=false|echo=Some(Channel { x: 524, y: 246 })|ready=false",
                "up|room|44189648edbbdbcb|login=false|join=false|echo=None|ready=false",
            ],
        );
    }

    #[test]
    fn room_row_golden_press_sequence_echoes_once_after_join() {
        // Live chain: window snapshot -> OnlineSession::ingest_snapshot
        // -> room press dispatch -> poll -> ScreenGame::ingest_snapshot.
        use o2jam_net::{JoinRoom, JoinRoomResponse};
        let (mut session, server) = room_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, JoinRoom::COMMAND);
            let asked = JoinRoom::decode_body(&frame.payload).unwrap();
            assert_eq!(asked.number, 3);
            let answered = JoinRoomResponse {
                result_code: 0,
                room: Some(joined_snapshot()),
            };
            stream.write_all(&answered.encode()).unwrap();
        });
        session.game_mut().set_rooms(vec![listed_room(3, "Jam")]);
        let mut trace = Vec::new();
        press_trace(&session, "start", &mut trace);
        session.ingest_snapshot(&InputSnapshot::press_at(400, 154));
        press_trace(&session, "down", &mut trace);
        session.ingest_snapshot(&InputSnapshot::release_at(400, 154));
        press_trace(&session, "up", &mut trace);
        paint_until(&mut session, |s| s.game.screen() == Screen::Waiting);
        press_trace(&session, "poll", &mut trace);
        server.join().unwrap();
        assert_golden_press_sequence(
            &trace,
            &[
                "start|room|44189648edbbdbcb|login=false|join=false|echo=None|ready=false",
                "down|room|44189648edbbdbcb|login=false|join=true|echo=None|ready=false",
                "up|room|44189648edbbdbcb|login=false|join=true|echo=None|ready=false",
                "poll|waiting|24960fb268a54468|login=false|join=false|echo=None|ready=false",
            ],
        );
    }

    #[test]
    fn resident_room_row_golden_press_sequence_echoes_once_without_wire() {
        // Live chain: window snapshot -> OnlineSession::ingest_snapshot
        // -> resident room press dispatch -> ScreenGame::ingest_snapshot.
        // The title match stays a heuristic (no wire room id in state).
        use std::io::Read;
        let (mut session, server) = room_session(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(
                stream.read_exact(&mut header).is_err(),
                "resident room press sent a frame on the wire"
            );
        });
        session.game.set_rooms(vec![listed_room(7, "Lobby")]);
        session.resident_room = Some("Lobby".to_owned());
        let mut trace = Vec::new();
        press_trace(&session, "start", &mut trace);
        session.ingest_snapshot(&InputSnapshot::press_at(400, 154));
        press_trace(&session, "down", &mut trace);
        session.ingest_snapshot(&InputSnapshot::release_at(400, 154));
        press_trace(&session, "up", &mut trace);
        server.join().unwrap();
        assert_golden_press_sequence(
            &trace,
            &[
                "start|room|44189648edbbdbcb|login=false|join=false|echo=None|ready=false",
                "down|waiting|24960fb268a54468|login=false|join=false|echo=Some(Room { x: 400, y: 154 })|ready=false",
                "up|waiting|24960fb268a54468|login=false|join=false|echo=None|ready=false",
            ],
        );
    }

    #[test]
    fn waiting_confirm_golden_press_sequence_pins_the_two_edge_gate() {
        // Live chain: window snapshot -> OnlineSession::ingest_snapshot
        // -> Waiting confirm dispatch -> Ready 4008 (one echo only).
        use o2jam_net::Ready;
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, Ready::COMMAND);
        });
        session.game_mut().load_scene(
            Screen::Waiting,
            vec![o2jam_shell::avatar::ScenePlacement {
                role: o2jam_shell::avatar::PlacementRole::Single,
                id: "IDC_BUTTON_SWAIT_READY".to_owned(),
                code: 0,
                sprite: None,
                rect: o2jam_render::Rect {
                    x: 100,
                    y: 100,
                    w: 200,
                    h: 60,
                },
            }],
            std::collections::HashMap::new(),
        );
        let mut trace = Vec::new();
        press_trace(&session, "start", &mut trace);
        session.ingest_snapshot(&InputSnapshot::click_at(150, 130));
        press_trace(&session, "click", &mut trace);
        server.join().unwrap();
        assert_golden_press_sequence(
            &trace,
            &[
                "start|waiting|b191bed28b2e0fbb|login=false|join=false|echo=None|ready=false",
                "click|waiting|b191bed28b2e0fbb|login=false|join=false|echo=None|ready=false",
            ],
        );
    }

    #[test]
    fn tt230_resident_face_press_replays_without_wire() {
        // Ticket #230: a face press naming the resident channel replays
        // locally with no ChannelLogin on the wire. The resident press
        // at the pinned face point (524, 246) must reach Room while the
        // server sees zero frames (read with a timeout: any send fails
        // the test instead of hanging it).
        // Live chain: main.rs::run_play -> window snapshot ->
        // GameLoop::ingest_snapshot -> OnlineSession::ingest_snapshot
        // -> dispatch_pointer_down -> echo_entry.
        use std::io::Read;
        let (mut session, server) = channel_session(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(
                stream.read_exact(&mut header).is_err(),
                "resident face press sent a frame on the wire"
            );
        });
        session.game.set_channels(vec![entry_channel()]);
        session.set_resident_channel(entry_channel());
        session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        assert_eq!(
            session.game.screen(),
            Screen::Room,
            "resident face press enters Room with no wire round trip"
        );
        server.join().unwrap();
    }

    #[test]
    fn tt331_resident_channel_confirm_replays_without_second_login() {
        // Live chain: run_online boot 1004/1005 -> set_resident_channel
        // -> GameLoop::menu_key(Confirm) -> confirm_channel. Confirm on
        // the resident row must advance locally: a second 1004 draws a
        // 1001 -1 from Memoryer and strands the walk in Channel (issue
        // #331 red proof).
        // Live chain: GameLoop::menu_key(Confirm) -> confirm_channel
        // -> begin_channel_login(EntryEcho::Confirm) -> echo_entry.
        use std::io::Read;
        let (mut session, server) = channel_session(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(
                stream.read_exact(&mut header).is_err(),
                "resident Confirm sent a second ChannelLogin on the wire"
            );
        });
        session.game.set_channels(vec![entry_channel()]);
        session.set_resident_channel(entry_channel());
        session.menu_key(MenuKey::Confirm, true);
        paint_frame(&mut session);
        assert_eq!(
            session.game.screen(),
            Screen::Room,
            "resident Confirm enters Room with no wire round trip"
        );
        server.join().unwrap();
    }

    #[test]
    fn tt231_resident_room_press_replays_without_wire() {
        // Ticket #231: a row press naming the resident room replays
        // locally with no join on the wire. The resident row press at
        // the fallback band (400, 154) must reach Waiting while the
        // server sees zero frames (read with a timeout: any send fails
        // the test instead of hanging it).
        // Live chain: main.rs::run_play -> window snapshot ->
        // GameLoop::ingest_snapshot -> OnlineSession::ingest_snapshot
        // -> dispatch_pointer_down -> echo_entry (title heuristic).
        use o2jam_net::RoomListEntry;
        use std::io::Read;
        let (mut session, server) = room_session(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(
                stream.read_exact(&mut header).is_err(),
                "resident room press sent a frame on the wire"
            );
        });
        session.game.set_rooms(vec![RoomListEntry {
            number: 7,
            state_code: 1,
            title: "Lobby".to_owned(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 5,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        }]);
        session.resident_room = Some("Lobby".to_owned());
        session.ingest_snapshot(&InputSnapshot::click_at(400, 154));
        assert_eq!(
            session.game.screen(),
            Screen::Waiting,
            "resident room press enters Waiting with no wire round trip"
        );
        server.join().unwrap();
    }

    #[test]
    fn waiting_confirm_click_sends_ready_and_stays() {
        // Staged confirm buttons ride the networked Confirm gate (4008
        // for guests); the shell stays in Waiting for the broadcast.
        // Live chain: window snapshot -> OnlineSession::ingest_snapshot
        // -> dispatch_pointer_down(WaitingConfirm) -> confirm_waiting.
        use o2jam_net::Ready;
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, Ready::COMMAND);
        });
        session.game_mut().load_scene(
            Screen::Waiting,
            vec![o2jam_shell::avatar::ScenePlacement {
                role: o2jam_shell::avatar::PlacementRole::Single,
                id: "IDC_BUTTON_SWAIT_READY".to_owned(),
                code: 0,
                sprite: None,
                rect: o2jam_render::Rect {
                    x: 100,
                    y: 100,
                    w: 200,
                    h: 60,
                },
            }],
            std::collections::HashMap::new(),
        );
        session.ingest_snapshot(&InputSnapshot::click_at(150, 130));
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }

    #[test]
    fn waiting_menu_confirm_sends_ready_through_snapshot() {
        // The session's Confirm gates ride the snapshot too: a Menu
        // edge (what the window queues for Enter) reaches the 4008
        // exchange, never the game's local advance.
        // Live chain: window Menu snapshot -> OnlineSession::ingest_snapshot
        // -> menu_key(Confirm) -> confirm_waiting.
        use o2jam_net::Ready;
        let (mut session, server) = joined_waiting_session(guest_snapshot(), |mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, Ready::COMMAND);
        });
        let mut snapshot = InputSnapshot::new();
        snapshot.push_edge(o2jam_shell::InputEdge::Menu {
            key: MenuKey::Confirm,
            down: true,
        });
        session.ingest_snapshot(&snapshot);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
    }
}
