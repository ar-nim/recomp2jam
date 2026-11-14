//! Live-boot staging completeness gate (ticket #58).
//!
//! Retro 2026-11-11: the room-dialog staging (#53), the character
//! fetch, and the splash clock (#54) each passed seam tests while the
//! live window stayed broken, because the suite staged everything by
//! hand. This gate drives the real boot entry
//! ([`crate::connect::connect_and_login_with_transport`] plus
//! [`crate::assemble_online_session`]) against a scripted gateway and
//! asserts everything paint and dispatch can consume lands in one
//! boot: the room create/password dialog sheets, the fetched character
//! info, and the splash hold advancing on the wall clock. Skips
//! cleanly when the install-root archives are absent.

#[cfg(test)]
mod live_boot_gate_tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    use o2jam_net::frame::decode_frame;
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, CharacterInfo, Frame, Gender, Request,
        RoomList, WireMessage, HEADER_LEN,
    };
    use o2jam_render::Frame as RenderFrame;
    use o2jam_shell::screens::SideScreen;
    use o2jam_shell::{GameLoop, Screen};

    use crate::{
        assemble_online_session, connect, install_root, setup_online, stage_online_identity,
    };

    /// Read one whole frame off the scripted gateway's socket.
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

    /// The 7.55 interface archive holds the dialog sheets the gate
    /// asserts; without it the gate skips (CI has no game data).
    fn archives_present() -> bool {
        o2jam_shell::staging::read_install_archives(&install_root())
            .interface
            .is_some()
    }

    /// The 7.55 playing archive holds the exe-literal sheets (#166);
    /// without it the gate skips only the by-name asserts below.
    fn playing_present() -> bool {
        o2jam_shell::staging::read_install_archives(&install_root())
            .playing
            .is_some()
    }

    /// Script the boot exchange: authorize, channel list, channel
    /// login, room list, then the `GetCharacterInfo` 2001 fetch the
    /// gate asserts. Returns the port and the served character.
    fn spawn_boot_gateway() -> (u16, thread::JoinHandle<CharacterInfo>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetCharacterInfo.id());
            let hero = CharacterInfo {
                nickname: "gate".to_owned(),
                gender: Gender::Male,
                level: 3,
                ..Default::default()
            };
            stream.write_all(&hero.encode()).unwrap();
            hero
        });
        (port, server)
    }

    /// Boot options the windowed entries assemble for a channel login
    /// (the character flag rides the channel, ticket #58).
    fn boot_options(port: u16) -> connect::ConnectOptions {
        connect::ConnectOptions {
            host: "127.0.0.1".to_owned(),
            port,
            user_id: 7,
            token: "gate".to_owned(),
            gender: Gender::Any,
            relay_key1: 0,
            relay_key2: 0,
            channel: Some((1, 2)),
            character: true,
            wallet: false,
            avatar_opa: None,
            join_room: None,
            room_password: String::new(),
            song: None,
        }
    }

    #[test]
    fn live_boot_stages_everything_paint_and_dispatch_consume() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let (port, server) = spawn_boot_gateway();
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        let game = session.game_mut();
        assert!(
            game.create_room_art_staged(),
            "live boot stages the create-room dialog sheet"
        );
        assert!(
            game.password_art_staged(),
            "live boot stages the password dialog sheet"
        );
        assert!(
            game.payment_art_staged(),
            "live boot stages the payment sheets"
        );
        if playing_present() {
            for sheet in [
                "State_Playing_network.bnd",
                "Avatar_Info.bnd",
                "Map_Avatar.bnd",
            ] {
                assert!(
                    game.byname_art_staged(sheet),
                    "live boot stages the {sheet} literal sheet"
                );
            }
        }
        let hero = game
            .myroom_character()
            .expect("live boot stages the fetched character info");
        assert_eq!(hero.nickname, "gate");
        assert_eq!(hero.level, 3);
        // The splash hold advances on the live wall clock (claims B38):
        // no input, no audio; condition polling only (#52 flake lesson).
        assert_eq!(session.screen(), Screen::Avi);
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut buf = vec![0u32; 16 * 12];
        while session.screen() == Screen::Avi {
            assert!(
                Instant::now() < deadline,
                "splash hold never advanced on the wall clock"
            );
            session.game_mut().paint(&mut RenderFrame {
                buf: &mut buf,
                w: 16,
                h: 12,
            });
            thread::yield_now();
        }
        assert_eq!(session.screen(), Screen::Planet);
        // The live input path reaches the game THROUGH the session's
        // GameLoop impl (the window holds `Box<dyn GameLoop>`). #118's
        // no-input leg: the impl overrode the whole-gesture edge but
        // not the #115 press edges, whose trait defaults were no-ops
        // — presses, releases, and motion vanished while menu_key
        // forwarded fine (Enter worked, clicks and hover did not).
        // The snapshot intake is required now, so the seam cannot
        // recur: every implementor names how it sweeps.
        assert!(
            session.game_mut().planet_modal().is_none(),
            "fresh boot owns a bare planet board"
        );
        {
            let game_loop: &mut dyn o2jam_shell::GameLoop = &mut session;
            game_loop.ingest_snapshot(&o2jam_shell::InputSnapshot::press_at(729, 580)); // gear disc (pinned e2e coord)
            game_loop.ingest_snapshot(&o2jam_shell::InputSnapshot::release_at(729, 580));
        }
        assert!(
            session.game_mut().planet_modal().is_some(),
            "snapshot press/release through the session GameLoop must open the gear dialog"
        );
        session
            .game_mut()
            .menu_key(o2jam_shell::MenuKey::Back, true);
        assert!(
            session.game_mut().planet_modal().is_none(),
            "escape closes the gear dialog after the forwarded gesture"
        );
        let served = server.join().unwrap();
        assert_eq!(served.nickname, "gate");
    }

    /// Ticket #136: side screens opened from the room on the live
    /// path painted fully black. The shots walk fills the side slots
    /// directly (`stage_side_art` + `visit_side`), while the live
    /// boot only staged the same State art into the full-screen
    /// scene slots — which the side overlay never reads — so the
    /// room-entered Shop/MyRoom/MusicShop painted the clear color.
    /// The gate boots through the real entry, enters each side from
    /// the room lobby, and asserts staged art inks the frame.
    #[test]
    fn live_boot_side_entry_paints_staged_art() {
        use o2jam_render::{NATIVE_HEIGHT, NATIVE_WIDTH};
        use o2jam_shell::screens::side::SideScreen;

        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let (port, server) = spawn_boot_gateway();
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(NATIVE_HEIGHT).unwrap_or(1);
        let game = session.game_mut();
        for lobby in [Screen::Planet, Screen::Channel, Screen::Room] {
            assert!(
                game.enter_screen(lobby),
                "gate reaches the room lobby through {lobby:?}"
            );
        }
        for side in [
            SideScreen::Shop,
            SideScreen::MusicShop,
            SideScreen::MyRoom,
            SideScreen::Payment,
        ] {
            assert!(
                game.visit_side(side),
                "room opens {side:?} on the live path"
            );
            let mut buf = vec![0u32; width * height];
            game.paint(&mut RenderFrame {
                buf: &mut buf,
                w: width,
                h: height,
            });
            assert!(
                buf.iter().any(|pixel| *pixel != 0),
                "live-path {side:?} paints fully black from the room"
            );
            assert!(game.leave_side(), "{side:?} returns to the room");
        }

        server.join().unwrap();
    }

    /// Ticket #191 reopen: the shop board must not be empty on the
    /// DEFAULT live boot (no --gateway-id/--channel-id, no
    /// --avatar-opa). The seam guard
    /// `shop_board_stages_goods_from_avatar_source` called
    /// `set_avatar_source` directly and passed while the default boot
    /// never reached it (`stage_online_identity` returned before the
    /// staging call without a fetched character). This gate drives
    /// the real setup path (`setup_online`, the entry `run_play`
    /// `--window --online` uses) with a channel-less login and
    /// asserts the catalog lands.
    ///
    /// Live caller chain:
    ///   main.rs::run_play (`--window --online`, default flags)
    ///     -> setup_online (channel=None, character unFetched)
    ///     -> connect::connect_and_login_with_transport
    ///     -> stage_online_identity (character None)
    ///     -> ScreenGame::set_avatar_source
    ///     -> ShopState::set_catalog
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_boot_default_stages_shop_catalog() {
        if !install_root().join("O2Jam/Image/avatar.opa").exists() {
            println!("skip: missing install-root avatar.opa");
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
        });
        // The default boot: online, but list-only — no gateway or
        // channel ids (so no character fetch), no `--avatar-opa`.
        let mut game = o2jam_shell::ScreenGame::new();
        let host = Some("127.0.0.1".to_owned());
        let user_id = Some(7);
        let token = Some("gate".to_owned());
        let transport = setup_online(
            &mut game, true, &host, user_id, &token, port, None, None, None, "", None, 0, 0, 0,
        )
        .unwrap()
        .expect("online setup returns the live transport");
        let (gateway, pending_room, pending_song) = transport;
        assert!(pending_room.is_none(), "default boot stages no room join");
        assert!(pending_song.is_none(), "default boot stages no song pick");
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        let game = session.game_mut();
        assert!(
            !game.shop_visible_goods().is_empty(),
            "default live boot stages the shop catalog"
        );
        assert!(
            !game.music_shop_visible_rows().is_empty(),
            "default live boot stages the music-shop board from the installed song catalog"
        );
        assert!(
            !game.lanking_visible_songs().is_empty(),
            "default live boot stages the lanking board from the installed song catalog"
        );
        assert!(
            !game.shop_avatar_staged(),
            "default boot stages no avatar pane without a fetched hero (never invented)"
        );
        server.join().unwrap();
    }

    /// Ticket #196: the channel-login boot must reach the avatar pane.
    /// The seam guards stage source and hero by hand; this gate drives
    /// the real entry (`setup_online` with a channel login) and asserts
    /// the equipped hero (real item 1, `avatar.opa` + `Itemdata`) lands
    /// a staged pane through the #193 stacker — the #191 lesson (the
    /// chain above the seam is what breaks).
    ///
    /// Live caller chain:
    ///   main.rs::run_play (`--window --online` channel login)
    ///   -> setup_online -> connect_and_login_with_transport
    ///   -> stage_online_identity (character Some)
    ///   -> ScreenGame::set_myroom_character + set_avatar_source
    ///   -> ScreenGame::restage_shop_avatar (load_worn_figure).
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_boot_shop_avatar_pane_stages_equipped_figure() {
        if !install_root().join("O2Jam/Image/avatar.opa").exists() {
            println!("skip: missing install-root avatar.opa");
            return;
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            let authed = AuthResponse {
                result_code: 0,
                tail: vec![],
            };
            stream.write_all(&authed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            let listed = ChannelList { channels: vec![] };
            stream.write_all(&listed.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            let entered = ChannelLoginResponse {
                result_code: 0,
                restricted: Some(false),
            };
            stream.write_all(&entered.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            let rooms = RoomList { rooms: vec![] };
            stream.write_all(&rooms.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetCharacterInfo.id());
            let mut hero = CharacterInfo {
                nickname: "gate".to_owned(),
                gender: Gender::Male,
                level: 3,
                ..Default::default()
            };
            hero.equipment[0] = 1;
            stream.write_all(&hero.encode()).unwrap();
            hero
        });
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        let game = session.game_mut();
        assert_eq!(
            game.shop_try_on(),
            None,
            "live boot stages no preview, only the equipped pane"
        );
        assert!(
            game.shop_avatar_staged(),
            "channel login stages the shop avatar pane from the equipped hero"
        );
        server.join().unwrap();
    }

    /// Ticket #192 (reopened): the music-shop and lanking boards were
    /// empty on the live boot — `stage_session_catalog` staged only the
    /// select-music dialog while only the `shots` walk staged the board
    /// catalogs. Boots through the real entry and asserts the boards
    /// ride the installed `OJNList.dat` live.
    #[test]
    fn live_boot_stages_music_shop_and_lanking_boards() {
        // Live caller chain:
        //   main.rs::run_play -> assemble_online_session ->
        //   stage_session_catalog ->
        //   session::OnlineSession::set_song_catalog ->
        //   ScreenGame::set_song_catalog ->
        //   MusicShopState::set_catalog + LankingState::set_catalog
        // Archives absent -> skip cleanly (CI has no game data).
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let catalog = install_root().join(crate::SESSION_CATALOG);
        let bytes = match std::fs::read(&catalog) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!("skip: missing {}", catalog.display());
                return;
            }
        };
        if o2jam_file::parse_ojn_list(&bytes).is_err() {
            println!("skip: unparsable {}", catalog.display());
            return;
        }
        let (port, server) = spawn_boot_gateway();
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        let game = session.game_mut();
        assert!(
            !game.music_shop_visible_rows().is_empty(),
            "live boot stages the music-shop board from the installed song catalog"
        );
        assert!(
            !game.lanking_visible_songs().is_empty(),
            "live boot stages the lanking board from the installed song catalog"
        );
        server.join().unwrap();
    }

    /// Ticket #199 close leg (live path): the channel-login boot must
    /// stage the whole MyRoom data path — character record, item-table
    /// bag source, staged worn figure — and the entered avatar room
    /// must paint something the unstaged boot does not.
    ///
    /// Live caller chain:
    ///   main.rs::run_play (`--window --online` with a channel)
    ///   -> setup_online -> connect::connect_and_login_with_transport
    ///   -> stage_online_identity -> ScreenGame::set_myroom_character
    ///     + ScreenGame::set_avatar_source (myroom_items ride here)
    ///   -> live_art::stage_live_scenes (real STATE_MYROOM art)
    ///   -> ScreenGame::enter_screen(Room) -> visit_side(MyRoom)
    ///   -> ScreenGame::paint_at -> paint_side -> paint_myroom_figure
    ///     + paint_myroom_overlay.
    /// The scripted gateway's hero carries server-shaped zeros (no
    /// equipment, no inventory), so the test dresses it from the real
    /// installed item table: the staging path stays 100% real, only
    /// the closet contents are crafted.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_boot_myroom_data_path_paints_on_channel_login() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let avatar_path = install_root().join("O2Jam/Image/avatar.opa");
        let Some(path) = avatar_path.to_str() else {
            println!("skip: non-UTF8 avatar.opa path");
            return;
        };
        let Ok((_, table)) = connect::read_avatar_source(path) else {
            println!("skip: missing or unparsable install-root avatar.opa");
            return;
        };
        if table.rows.is_empty() {
            println!("skip: installed item table holds no rows");
            return;
        }
        let (port, server) = spawn_boot_gateway();
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut hero = report
            .character
            .clone()
            .expect("channel login fetches the character");
        for (slot, row) in table.rows.iter().take(5).enumerate() {
            if let Ok(id) = i32::try_from(row.item_id) {
                hero.equipment[slot] = id;
                hero.inventory.push(id);
            }
        }
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, Some(&hero));
        assert!(
            game.myroom_character().is_some(),
            "channel login stages the character record live"
        );
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        let live = session.game_mut();
        crate::live_art::stage_live_scenes(live);
        assert!(live.enter_screen(Screen::Planet));
        assert!(live.enter_screen(Screen::Channel));
        assert!(live.enter_screen(Screen::Room));
        assert!(live.visit_side(SideScreen::MyRoom));
        let mut painted = vec![0u32; 800 * 600];
        live.paint_at(
            &mut RenderFrame {
                buf: &mut painted,
                w: 800,
                h: 600,
            },
            0,
        );
        let bare = {
            let mut unstaged = o2jam_shell::ScreenGame::new();
            crate::live_art::stage_live_scenes(&mut unstaged);
            assert!(unstaged.enter_screen(Screen::Planet));
            assert!(unstaged.enter_screen(Screen::Channel));
            assert!(unstaged.enter_screen(Screen::Room));
            assert!(unstaged.visit_side(SideScreen::MyRoom));
            let mut buf = vec![0u32; 800 * 600];
            unstaged.paint_at(
                &mut RenderFrame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                0,
            );
            buf
        };
        assert_ne!(
            painted, bare,
            "entered MyRoom paints staged character data live (figure, bag, captions)"
        );
        server.join().unwrap();
    }
}
