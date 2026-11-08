//! Online waiting-room session: the transport owner for chat.
//!
//! The shell builds the 3011 packet ([`ScreenGame::take_chat_message`])
//! but owns no transport, so an offline Enter drops it. The session
//! closes that gap: it holds the [`GatewayConnection`] beside the
//! [`ScreenGame`] and transmits the draft on send (Enter). It
//! implements [`GameLoop`], so the window shell can own the session
//! instead of the bare game.
//!
//! Pattern for stats/rank traffic to follow: read shell state, encode
//! through `o2jam_net` (as [`connect::run_song_flow`] does for the
//! 4014 tick and 4016 tally), and send through the owned gateway. Chat
//! sends on user action; stats/rank sends poll per frame or per tick.

use o2jam_net::rooms::SendWaitingMessage;
use o2jam_net::{GatewayConnection, WireMessage};
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

/// Waiting-room session: shell game plus its gateway transport.
///
/// Single-threaded by construction (the blocking gateway and the game
/// both live here, never shared), so no lock ordering applies.
pub struct OnlineSession {
    game: ScreenGame,
    gateway: GatewayConnection,
}

impl OnlineSession {
    /// Pair a game with its gateway transport.
    #[must_use]
    pub fn new(game: ScreenGame, gateway: GatewayConnection) -> Self {
        Self { game, gateway }
    }

    /// Transmit the pending chat draft as 3011.
    ///
    /// `Ok(true)` sent one packet; `Ok(false)` means the draft was
    /// empty and the socket stayed untouched. A failed send keeps the
    /// draft, so the next send retries it.
    ///
    /// # Errors
    ///
    /// Whatever the gateway transport reports (ERRORS-01/02: `Result`,
    /// propagated with `?`, never unwrapped per ERRORS-06).
    pub fn flush_chat(&mut self) -> Result<bool, o2jam_net::Error> {
        let Some(draft) = self.game.chat_send_message() else {
            return Ok(false);
        };
        self.gateway
            .send_frame(SendWaitingMessage::COMMAND, &draft.encode_body())?;
        self.game.clear_chat_input();
        Ok(true)
    }
}

impl GameLoop for OnlineSession {
    fn lane_key(&mut self, lane: usize, down: bool) {
        self.game.lane_key(lane, down);
    }

    fn menu_key(&mut self, key: MenuKey, down: bool) {
        if !down {
            return;
        }
        if key == MenuKey::Confirm && self.game.screen() == Screen::Waiting {
            match self.flush_chat() {
                Ok(sent) if !sent => self.game.menu_key(key, down),
                Ok(_) => {}
                Err(source) => {
                    tracing::warn!(%source, "chat send failed; draft kept");
                }
            }
            return;
        }
        self.game.menu_key(key, down);
    }

    fn text_input(&mut self, text: &str) {
        self.game.text_input(text);
    }

    fn delete_text(&mut self) {
        self.game.delete_text();
    }

    fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
        self.game.paint(frame);
    }
}

#[cfg(test)]
mod session_tests {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    use o2jam_net::frame::decode_frame;
    use o2jam_net::{Frame, GatewayConnection, HEADER_LEN};
    use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

    use super::OnlineSession;

    /// One whole frame off the fake server's socket.
    fn read_frame(stream: &mut TcpStream) -> Frame {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).unwrap();
        let body_len = u16::from_le_bytes([header[0], header[1]]) as usize;
        let mut rest = vec![0u8; body_len - HEADER_LEN];
        stream.read_exact(&mut rest).unwrap();
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&rest);
        decode_frame(&bytes).unwrap()
    }

    /// Game walked Avi → Channel → Room → Waiting, paired with a live
    /// fake gateway; the server end runs `with_server`.
    fn waiting_session(
        with_server: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (OnlineSession, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            with_server(stream);
        });
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        (OnlineSession::new(game, gateway), server)
    }

    #[test]
    fn flush_sends_pending_draft_as_3011() {
        let (mut session, server) = waiting_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 3011);
            assert_eq!(frame.payload, b"go".to_vec());
        });
        session.game.set_chat_input("go");
        assert!(session.flush_chat().unwrap());
        assert_eq!(session.game.chat_input(), "");
        server.join().unwrap();
    }

    #[test]
    fn flush_sends_nothing_while_the_draft_is_empty() {
        let (mut session, server) = waiting_session(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(stream.read_exact(&mut header).is_err());
        });
        assert!(!session.flush_chat().unwrap());
        server.join().unwrap();
    }

    #[test]
    fn confirm_sends_the_draft_and_stays_waiting() {
        let (mut session, server) = waiting_session(|mut stream| {
            let frame = read_frame(&mut stream);
            assert_eq!(frame.id, 3011);
            assert_eq!(frame.payload, b"hi".to_vec());
        });
        session.game.set_chat_input("hi");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Waiting);
        assert_eq!(session.game.chat_input(), "");
    }

    #[test]
    fn confirm_without_a_draft_still_advances_to_loading() {
        let (mut session, server) = waiting_session(|mut stream| {
            stream
                .set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(stream.read_exact(&mut header).is_err());
        });
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.screen(), Screen::Loading);
    }

    #[test]
    fn confirm_outside_waiting_sends_nothing() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(150)))
                .unwrap();
            let mut header = [0u8; 4];
            assert!(stream.read_exact(&mut header).is_err());
        });
        let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
        let mut session = OnlineSession::new(ScreenGame::new(), gateway);
        session.game.set_chat_input("typed early");
        session.menu_key(MenuKey::Confirm, true);
        server.join().unwrap();
        assert_eq!(session.game.chat_input(), "typed early");
    }
}
