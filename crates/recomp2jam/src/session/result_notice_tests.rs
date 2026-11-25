//! Tier-2 scripted-gateway guards for the 2016 fixed-rect notice
//! (#1077, pin `FUN_0050ce50` / file `0x10ce50`). The session file
//! declares this module beside the other scripted wire families.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use o2jam_net::rooms::Announcement;
use o2jam_net::testing::fake_gateway;
use o2jam_net::{GatewayConnection, WireMessage};
use o2jam_shell::screens::game::LobbyChat;
use o2jam_shell::{GameLoop, MenuKey, ScreenGame};

use super::session_tests::paint_until;
use super::OnlineSession;

/// Hold the gateway socket open until the client closes, so a slow
/// test scheduler cannot make the write race the notice drain.
fn hold_until_client_closes(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("gateway read timeout");
    let mut byte = [0u8; 1];
    while stream.read(&mut byte).unwrap_or(0) > 0 {}
}

/// One session walked from boot to the Channel board on a gateway
/// that pushes one 2016 frame. The board owns the lobby caption panel
/// the fixed-rect painter reads (`LobbyChat::Channel`).
fn notice_session(content: &str) -> (OnlineSession, std::thread::JoinHandle<()>) {
    let notice = Announcement {
        content: content.to_owned(),
    };
    let (port, server) = fake_gateway(move |mut stream| {
        stream
            .write_all(&notice.encode())
            .expect("2016 announcement reaches the gateway");
        hold_until_client_closes(&mut stream);
    });
    let mut game = ScreenGame::new();
    game.menu_key(MenuKey::Back, true);
    game.click_planet(136, 88);
    let gateway = GatewayConnection::connect(format!("127.0.0.1:{port}").as_str()).unwrap();
    (OnlineSession::new(game, gateway), server)
}

/// #1077 Tier-2 scripted packet leg. Live chain:
/// `main.rs::run_play` → `connect::connect_and_login_with_transport`
/// → `assemble_online_session` → `OnlineSession::paint` →
/// `poll_lobby_chat` → `drain_lobby_chat` → `apply_lobby_frame` →
/// `apply_announcement` → `ScreenGame::push_lobby_announcement` →
/// the fixed-rect caption the Tier-0 guard samples. The empty-text
/// gate keeps a blank 2016 body off the panel; this leg sends a
/// nonempty caption.
#[test]
fn incoming_notice_from_the_gateway_inks_the_fixed_caption() {
    let (mut session, server) = notice_session("NOTICE");
    paint_until(&mut session, |session| {
        !session
            .game
            .lobby_chat(LobbyChat::Channel)
            .history()
            .is_empty()
    });
    server.join().expect("gateway thread joined");
    let staged = session.game.lobby_chat(LobbyChat::Channel).history();
    assert_eq!(staged.len(), 1, "one scripted 2016 caption stages one row");
    assert_eq!(staged[0].text, "NOTICE");
    assert!(staged[0].admin, "the notice uses the admin ink family");
}
