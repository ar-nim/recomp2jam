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
        AuthResponse, ChannelList, ChannelLoginResponse, ChannelState, CharacterInfo, Frame,
        Gender, JoinedRoom, Request, Response, RoomList, RoomListEntry, WireMessage, HEADER_LEN,
    };
    use o2jam_render::{pack_xrgb, Frame as RenderFrame, Rgb555Image};
    use o2jam_shell::avatar::{AvatarFigure, FigureLayer, PlacementRole, ScenePlacement};
    use o2jam_shell::screens::game::PlanetModal;
    use o2jam_shell::screens::paint::{unbound_scene_placements, CHANNEL_DIALOG_SPRITE};
    use o2jam_shell::screens::side::{
        MyRoomPanel, PlanetOptionTab, MYROOM_BAG_FIRST_CODE, MYROOM_SELECT_CODE, MYROOM_VACATE_CODE,
    };
    use o2jam_shell::screens::SideScreen;
    use o2jam_shell::staging::{
        read_install_archives, stage_state, MessengerSheet, StageContext, State, MESSENGER_SHEETS,
    };
    use o2jam_shell::{GameLoop, InputSnapshot, Screen};

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
        spawn_boot_gateway_with_equipment([0; 17])
    }

    /// Same boot, but the fetched hero wears the given equipment table
    /// (the live tester's face-only kit is the #243 oracle).
    fn spawn_boot_gateway_with_equipment(
        equipment: [i32; 17],
    ) -> (u16, thread::JoinHandle<CharacterInfo>) {
        spawn_boot_gateway_with_rooms(equipment, Vec::new())
    }

    /// Same boot, but the 2003 reply carries `rooms` (empty or live).
    fn spawn_boot_gateway_with_rooms(
        equipment: [i32; 17],
        rooms: Vec<RoomListEntry>,
    ) -> (u16, thread::JoinHandle<CharacterInfo>) {
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
            stream.write_all(&RoomList { rooms }.encode()).unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetCharacterInfo.id());
            let hero = CharacterInfo {
                nickname: "gate".to_owned(),
                gender: Gender::Male,
                level: 3,
                equipment,
                ..Default::default()
            };
            stream.write_all(&hero.encode()).unwrap();
            hero
        });
        (port, server)
    }

    /// Script the 1000/1001/1002/1003 boot exchange with real 1003
    /// rows, returning the gateway's observed frame order. The order
    /// is the relay-down story's socket-side pin (close-cp boot-2:
    /// manifest `64caf0c4…8c7208`, citation `30d1b061…2e5290c7`).
    fn spawn_boot_gateway_with_channels(
        channels: Vec<ChannelState>,
    ) -> (u16, thread::JoinHandle<(CharacterInfo, Vec<u16>)>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut order = Vec::new();

            let authorize = read_frame(&mut stream);
            order.push(authorize.id);
            assert_eq!(authorize.id, Request::Authorize.id());
            stream
                .write_all(
                    &AuthResponse {
                        result_code: 0,
                        tail: vec![],
                    }
                    .encode(),
                )
                .unwrap();
            order.push(Response::Authorize.id());

            let listed = read_frame(&mut stream);
            order.push(listed.id);
            assert_eq!(listed.id, Request::GetChannelList.id());
            stream
                .write_all(&ChannelList { channels }.encode())
                .unwrap();
            order.push(Response::GetChannelList.id());

            let login = read_frame(&mut stream);
            order.push(login.id);
            assert_eq!(login.id, Request::ChannelLogin.id());
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            order.push(Response::ChannelLogin.id());

            let rooms = read_frame(&mut stream);
            order.push(rooms.id);
            assert_eq!(rooms.id, Request::GetChannelInfo.id());
            stream
                .write_all(&RoomList { rooms: vec![] }.encode())
                .unwrap();
            order.push(Response::RoomList.id());

            let fetched = read_frame(&mut stream);
            order.push(fetched.id);
            assert_eq!(fetched.id, Request::GetCharacterInfo.id());
            let hero = CharacterInfo {
                nickname: "gate".to_owned(),
                ..Default::default()
            };
            stream.write_all(&hero.encode()).unwrap();
            order.push(Response::GetCharacterInfo.id());
            (hero, order)
        });
        (port, server)
    }

    /// Record the frame clock at every banner change in a window.
    /// The paint seam is the same one the windowed heartbeat drives,
    /// so the returned instants are the client's own advance times.
    fn promo_changes_between(
        game: &mut o2jam_shell::ScreenGame,
        from_ms: u64,
        until_ms: u64,
    ) -> Vec<(u64, u32)> {
        let mut changes = Vec::new();
        let mut shown = game.planet_promo_index();
        let mut buf = vec![0u32; 16 * 12];
        for now_ms in (from_ms..=until_ms).step_by(50) {
            game.paint_at(
                &mut RenderFrame {
                    buf: &mut buf,
                    w: 16,
                    h: 12,
                },
                now_ms,
            );
            let next = game.planet_promo_index();
            if next != shown {
                changes.push((now_ms, next));
                shown = next;
            }
        }
        changes
    }

    /// Waiting room with no wire peers: the self seat still restages
    /// from the fetched hero when the avatar source lands.
    fn self_only_room() -> JoinedRoom {
        JoinedRoom {
            member_id: 0,
            team: 0,
            title: "Jam".to_owned(),
            music_id: 0,
            arena: 2,
            key_mode: 2,
            game_mode: 0,
            difficulty: 0,
            speed: 1,
            has_super_manager: false,
            user_count: 1,
            slots: Vec::new(),
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
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
        // Live caller chain: main.rs::run_play -> setup_online ->
        // connect::connect_and_login_with_transport ->
        // assemble_online_session -> session::OnlineSession::stage_live_art
        // -> live_art::stage_live_scenes -> stage_interface_scenes ->
        // stage_messenger_sheets -> ScreenGame::load_messenger_sheet
        // (the slot this gate inspects). Geometry only; no sheet opens.
        for sheet in MESSENGER_SHEETS {
            assert!(
                game.messenger_sheet_staged(sheet),
                "live boot parks the {} sheet rects",
                sheet.block_name()
            );
            // The parsed rect count is pinned in the staging guard;
            // `SET` clones add placements in MAIN, so only geometry
            // presence is asserted here.
            assert!(
                !game.messenger_sheet_placements(sheet).is_empty(),
                "parked {} sheet carries placements",
                sheet.block_name()
            );
        }
        if playing_present() {
            assert!(
                game.tutorial_art_staged(),
                "live boot stages the tutorial guide bodies before FIRSTSTEP"
            );
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
        // No sheet opens at boot: the pinned packs never mount one
        // and no messenger wire shape exists (story 12 is geometry
        // only; N-MOUNT stays OPEN).
        assert!(
            !game.messenger_liveness_open(),
            "staging never opens a Messenger sheet"
        );
        assert_eq!(
            MessengerSheet::from_block("DIALOG_MESSENGER_MAIN"),
            Some(MessengerSheet::Main)
        );
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

    /// One synthetic 2003 row; `has_password` stays false so a click
    /// on the first band enters Waiting instead of opening a dialog.
    fn listed_room(number: i32, title: &str) -> RoomListEntry {
        RoomListEntry {
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
            user_count: 1,
            min_level: 0,
            max_level: 0,
            skills: Vec::new(),
            premium: false,
            room_type: 0,
        }
    }

    /// Walk a booted game from the splash to the Room lobby.
    fn enter_room_lobby(game: &mut o2jam_shell::ScreenGame) {
        use o2jam_shell::MenuKey;
        // The scripted gateway lists no channels; stage one active row
        // so the offline menu walk has a channel to enter. Rooms stay
        // exactly what the 2003 reply staged.
        game.set_channels(vec![o2jam_net::ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        }]);
        game.menu_key(MenuKey::Back, true);
        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(136, 88));
        // The scripted gateway has no relay, so the boot arms the
        // relay-down notice over the channel frame (#348); dismiss
        // it before the Enter that opens the Room lobby.
        assert_eq!(game.planet_modal(), Some(PlanetModal::RelayDown));
        game.menu_key(MenuKey::Back, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
    }

    /// Live caller chain:
    ///   main.rs::run_play (--window --online) -> setup_online
    ///   -> connect_and_login_with_transport -> 2003 RoomList reply
    ///   -> stage_online_rooms -> ScreenGame::set_rooms.
    /// An empty 2003 after a non-empty one must clear the old rows:
    /// the exe's empty channel has no clickable room row.
    #[test]
    fn empty_2003_on_online_entry_clears_stale_clickable_rows() {
        let (port, server) = spawn_boot_gateway_with_rooms([0; 17], Vec::new());
        let host = Some("127.0.0.1".to_owned());
        let token = Some("gate".to_owned());
        let mut game = o2jam_shell::ScreenGame::new();
        game.set_rooms(vec![listed_room(3, "Stale")]);
        let _transport = setup_online(
            &mut game,
            true,
            &host,
            Some(7),
            &token,
            None,
            port,
            Some(1),
            Some(2),
            None,
            "",
            None,
            0,
            0,
            0,
        )
        .unwrap()
        .expect("online entry returns the live transport");
        assert!(
            game.listed_rooms().is_empty(),
            "an empty 2003 reply must replace the stale room rows"
        );
        enter_room_lobby(&mut game);
        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(400, 154));
        assert_eq!(
            game.screen(),
            Screen::Room,
            "no stale row may stay clickable after an empty list"
        );
        server.join().unwrap();
    }

    /// Same live entry with a non-empty 2003: the staged list is
    /// exactly the wire rows, never a mix with previously staged ones.
    #[test]
    fn non_empty_2003_on_online_entry_replaces_stale_rows_with_wire_rows() {
        let wire_rooms = vec![listed_room(3, "WireOne"), listed_room(7, "WireTwo")];
        let (port, server) = spawn_boot_gateway_with_rooms([0; 17], wire_rooms.clone());
        let host = Some("127.0.0.1".to_owned());
        let token = Some("gate".to_owned());
        let mut game = o2jam_shell::ScreenGame::new();
        game.set_rooms(vec![listed_room(5, "Stale")]);
        let _transport = setup_online(
            &mut game,
            true,
            &host,
            Some(7),
            &token,
            None,
            port,
            Some(1),
            Some(2),
            None,
            "",
            None,
            0,
            0,
            0,
        )
        .unwrap()
        .expect("online entry returns the live transport");
        let titles: Vec<&str> = game
            .listed_rooms()
            .iter()
            .map(|room| room.title.as_str())
            .collect();
        assert_eq!(titles, vec!["WireOne", "WireTwo"]);
        server.join().unwrap();
    }

    /// Ticket #356 live chain: `live_boot_gate` test ->
    /// `connect::connect_and_login_with_transport` ->
    /// `assemble_online_session` -> `OnlineSession`'s `GameLoop::lane_key`
    /// -> `ScreenGame::lane_key`. The exe keeps the splash on Space and
    /// the other lane binds (`~/scratch/wt24-rig/key-space/`); only
    /// Escape is the pinned fast dismiss (`avi-hold/run3-escape/`).
    #[test]
    fn live_boot_lane_press_keeps_the_splash_then_escape_dismisses_it() {
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
        assert_eq!(session.screen(), Screen::Avi);
        {
            let game_loop: &mut dyn GameLoop = &mut session;
            game_loop.lane_key(3, true); // Space, the pinned lane-3 bind
            game_loop.lane_key(3, false);
            assert_eq!(
                game_loop.screen_name(),
                "avi",
                "a live lane press must not skip the splash hold"
            );
        }
        assert_eq!(session.screen(), Screen::Avi);
        session
            .game_mut()
            .menu_key(o2jam_shell::MenuKey::Back, true);
        assert_eq!(session.screen(), Screen::Planet, "Escape reaches Planet");
        server.join().unwrap();
    }

    /// Ticket #359 live chain: `live_boot_gate` test ->
    /// `connect::connect_and_login_with_transport` ->
    /// `assemble_online_session` (stages `Dialog_Option_*.bnd`) ->
    /// `OnlineSession`'s `GameLoop::ingest_snapshot` (gear press/release)
    /// -> `ScreenGame::click_planet` -> `planet_board.open_modal(Options)`
    /// -> `ScreenGame::paint` -> `paint_planet_options` over the open
    /// tab's staged sheet. The b3/b4 exe captures (manifest sha256
    /// `4f0428d0…` / `e2241ab5…`, `options.png` `333f9bae…` / `6d17a781…`)
    /// pin Sound sliders as the first sheet.
    #[test]
    fn live_boot_gear_opens_the_pinned_sound_sheet() {
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
        assert_eq!(session.screen(), Screen::Avi);
        session
            .game_mut()
            .menu_key(o2jam_shell::MenuKey::Back, true);
        assert_eq!(session.screen(), Screen::Planet);
        {
            let game_loop: &mut dyn GameLoop = &mut session;
            game_loop.ingest_snapshot(&o2jam_shell::InputSnapshot::press_at(729, 580));
            game_loop.ingest_snapshot(&o2jam_shell::InputSnapshot::release_at(729, 580));
        }
        assert!(
            session.game_mut().planet_modal().is_some(),
            "gear press opens the options modal"
        );
        assert_eq!(
            session.game_mut().planet_option_tab(),
            PlanetOptionTab::Sound,
            "b3/b4 pin the four volume sliders as the first sheet"
        );
        // The painter reads the open tab's staged `.bnd` sheet: paint
        // Sound, flip to Key through the switch, and require the frame
        // to change. A constant flip with no painter wiring stays red.
        let mut sound_frame = vec![0u32; 800 * 600];
        let mut key_frame = vec![0u32; 800 * 600];
        session.game_mut().paint(&mut RenderFrame {
            buf: &mut sound_frame,
            w: 800,
            h: 600,
        });
        session.game_mut().switch_option_tab(PlanetOptionTab::Key);
        session.game_mut().paint(&mut RenderFrame {
            buf: &mut key_frame,
            w: 800,
            h: 600,
        });
        assert_ne!(sound_frame, key_frame, "the painter shows the open sheet");
        server.join().unwrap();
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

    /// Ticket #362: the exe's Cash Charge window opens from the Item
    /// Shop charge plate at native (92,355) and shows a web client at
    /// `(190,58)-(630,564)` with RGB `(25,24,25)` in the qualified
    /// b8 capture (manifest `f9d4a695...`, image `27fd5435...`).
    ///
    /// Live caller chain:
    ///   main.rs::run_play (`--window --online`)
    ///     -> setup_online -> assemble_online_session
    ///     -> live_boot_gate::live_boot_itemshop_charge_plate_paints_cash_charge_chrome
    ///     -> ScreenGame::visit_side(Shop) -> ScreenGame::click_shop(92,355)
    ///     -> shop_action_at(SHOP_CHARGE_MONEY_CODE) -> visit_side(Payment)
    ///     -> mount_staged_side_art -> paint_side -> paint_cash_charge_frame
    #[test]
    fn live_boot_itemshop_charge_plate_paints_cash_charge_chrome() {
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
        for lobby in [Screen::Planet, Screen::Channel, Screen::Room] {
            assert!(
                game.enter_screen(lobby),
                "gate reaches the shop lobby through {lobby:?}"
            );
        }
        assert!(
            game.cash_charge_art_staged(),
            "live boot stages the Cash Charge frame and close"
        );
        assert!(game.visit_side(SideScreen::Shop));
        game.click_shop(92, 355);
        assert_eq!(
            game.side(),
            Some(SideScreen::Payment),
            "Item Shop charge plate opens the Cash Charge view"
        );
        let mut buf = vec![0u32; 800 * 600];
        game.paint(&mut RenderFrame {
            buf: &mut buf,
            w: 800,
            h: 600,
        });
        // The b8 capture reads RGB (25,24,25) in the client area;
        // the exe's RGB555 body decodes to (24,24,24) here, one step
        // per captured channel. Both pin the same black web client.
        let client = buf[300 * 800 + 400];
        let channels = [(client >> 16) as u8, (client >> 8) as u8, client as u8];
        for (painted, captured) in channels.into_iter().zip([25, 24, 25]) {
            assert!(
                painted.abs_diff(captured) <= 1,
                "exe-pinned web client colour at the Cash Charge rect: {channels:?}"
            );
        }
        assert_ne!(
            buf[300 * 800 + 180],
            client,
            "staged frame chrome borders the web client"
        );
        assert!(
            game.take_shop_buy().is_none(),
            "charge plate queues no purchase send"
        );
        server.join().unwrap();
    }

    /// Reopened #362: the b8 click is delivered as a pointer snapshot, not
    /// as a direct `click_shop` call. This guard drives that whole live
    /// chain and reads pixels before and after, so a Shop -> Payment
    /// switch that stops painting the staged chrome fails here.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> assemble_online_session
    ///   -> OnlineSession::ingest_snapshot(InputSnapshot::click_at(92,355))
    ///   -> ScreenGame::ingest_snapshot -> apply_pointer_down -> click_dispatch
    ///   -> click_side_screen -> click_shop(92,355)
    ///   -> shop_action_at(SHOP_CHARGE_MONEY_CODE) -> leave_side
    ///   -> visit_side(Payment) -> mount_staged_side_art(Payment)
    ///   -> paint_side -> paint_cash_charge_frame
    #[test]
    fn live_boot_charge_plate_snapshot_click_paints_chrome() {
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
        for lobby in [Screen::Planet, Screen::Channel, Screen::Room] {
            assert!(
                session.game_mut().enter_screen(lobby),
                "gate reaches the shop lobby through {lobby:?}"
            );
        }
        assert!(
            session.game_mut().cash_charge_art_staged(),
            "live boot stages the Cash Charge chrome"
        );
        assert!(session.game_mut().visit_side(SideScreen::Shop));
        let mut before = vec![0u32; 800 * 600];
        let mut after = vec![0u32; 800 * 600];
        let paint = |session: &mut crate::session::OnlineSession, buf: &mut [u32]| {
            session.paint(&mut RenderFrame {
                buf,
                w: 800,
                h: 600,
            });
        };
        paint(&mut session, &mut before);

        session.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(92, 355));

        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::Payment),
            "the live snapshot click opens the Cash Charge view"
        );
        paint(&mut session, &mut after);
        let changed = (58..564)
            .flat_map(|y| (190..630).map(move |x| y * 800 + x))
            .filter(|index| before[*index] != after[*index])
            .count();
        assert!(
            changed > (564 - 58) * (630 - 190) / 2,
            "the click repaints the b8 web-client region, changed={changed}"
        );
        let client = after[300 * 800 + 400];
        let channels = [(client >> 16) as u8, (client >> 8) as u8, client as u8];
        for (painted, captured) in channels.into_iter().zip([25, 24, 25]) {
            assert!(
                painted.abs_diff(captured) <= 1,
                "exe-pinned web client colour after the live click: {channels:?}"
            );
        }
        server.join().unwrap();
    }

    /// Ticket #430: the exe's one X press at (590,23) dismisses the
    /// Cash Charge chrome and reveals the Item Shop board (boot 3,
    /// manifest `391212db8ff478efc97b9a761631cee6668441b60d17b164356406580b72ce6c`,
    /// post-X frame `ce0e097066d1f53488af4039c85125e5e927b2cb900bcc7042bce633574ff5fd`).
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> assemble_online_session
    ///   -> OnlineSession::ingest_snapshot(click_at(92,355))
    ///   -> ScreenGame::visit_side(Shop) -> click_shop -> visit_side(Payment)
    ///   -> OnlineSession::ingest_snapshot(click_at(590,23))
    ///   -> ScreenGame::click_side_screen -> cash_charge_close_press -> leave_side
    ///   -> paint_screen(Shop)
    #[test]
    fn live_boot_charge_plate_close_press_returns_to_host_board() {
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
        for lobby in [Screen::Planet, Screen::Channel, Screen::Room] {
            assert!(
                session.game_mut().enter_screen(lobby),
                "gate reaches the shop lobby through {lobby:?}"
            );
        }
        assert!(session.game_mut().visit_side(SideScreen::Shop));
        session.ingest_snapshot(&InputSnapshot::click_at(92, 355));
        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::Payment),
            "the charge plate opens the Cash Charge view"
        );

        session.ingest_snapshot(&InputSnapshot::click_at(590, 23));

        assert_eq!(
            session.game_mut().side(),
            None,
            "the pinned X press dismisses the Cash Charge view"
        );
        let mut buf = vec![0u32; 800 * 600];
        session.paint(&mut RenderFrame {
            buf: &mut buf,
            w: 800,
            h: 600,
        });
        let client = buf[300 * 800 + 400];
        let channels = [(client >> 16) as u8, (client >> 8) as u8, client as u8];
        assert!(
            channels
                .into_iter()
                .zip([25, 24, 25])
                .any(|(a, b)| a.abs_diff(b) > 1),
            "the web-client colour is gone after the close press: {channels:?}"
        );
        assert!(
            session.game_mut().take_shop_buy().is_none(),
            "the close press queues no purchase send"
        );
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
            &mut game, true, &host, user_id, &token, None, port, None, None, None, "", None, 0, 0,
            0,
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
            game.shop_item_portrait_count() > 0,
            "default live boot resolves slot-0 shop portraits for the goods cards (#363)"
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

    /// Ticket #243/#375: the live tester's empty body slot still
    /// carries the default face (item 35, `Equip12`, wire slot 11), so
    /// the exe paints the naked default kit under it: item 30 body,
    /// item 31 bare arms (one image), item 35 face. The #336 pins the
    /// body and face (EV-336-A..D); #375 pins the arms from the exe
    /// crop diff (`docs/equip.md`). The scripted hero carries that
    /// face-only wire table.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online
    ///   -> connect::connect_and_login_with_transport
    ///   -> stage_online_identity -> connect::stack_worn_figure
    ///   -> o2jam_shell::load_worn_figure
    ///   -> ScreenGame::load_room_avatar + ScreenGame::set_avatar_source
    ///   -> stage_seat_figures + restage_shop_avatar.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_boot_empty_body_slot_stages_the_default_naked_figure() {
        if !install_root().join("O2Jam/Image/avatar.opa").exists() {
            println!("skip: missing install-root avatar.opa");
            return;
        }
        let mut face_only = [0; 17];
        face_only[11] = 35;
        let (port, server) = spawn_boot_gateway_with_equipment(face_only);
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        assert!(
            report
                .character
                .as_ref()
                .is_some_and(|hero| hero.equipment[11] == 35
                    && hero
                        .equipment
                        .iter()
                        .enumerate()
                        .all(|(slot, id)| { slot == 11 || *id == 0 })),
            "the gateway hero carries the live face-only equipment table"
        );
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        game.set_joined_room(self_only_room());
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        let game = session.game_mut();

        let (archive, _) = connect::read_avatar_source(
            install_root()
                .join("O2Jam/Image/avatar.opa")
                .to_string_lossy()
                .as_ref(),
        )
        .unwrap();
        let pinned = connect::stack_worn_figure(&archive, &[30, 31, 35], true).unwrap();
        assert_eq!(
            pinned.len(),
            25,
            "body and face carry 12 frames each, bare arms one"
        );

        let room = game.room_avatar().expect("room card stages a figure");
        assert_eq!(room.len(), pinned.len(), "room card stages the default kit");
        assert_eq!(room.extent(), pinned.extent());
        assert!(game.shop_avatar_staged(), "character card stages a figure");
        let seat = game.waiting_avatar(0).expect("self seat stages a figure");
        assert_eq!(seat.len(), pinned.len(), "seat stages the default kit");
        assert_eq!(seat.extent(), pinned.extent());
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
        assert_eq!(
            live.myroom().panel(),
            MyRoomPanel::Inventory,
            "default live MyRoom entry opens the exe's Inventory face (#369)"
        );
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
        // #305: the real selection frame body and its bnd control
        // must land in the same live staging pass, and a picked
        // cell must repaint with the art instead of the amber
        // stand-in. Geometry comes from the staged archive block,
        // never a literal in this gate.
        let interface = read_install_archives(&install_root())
            .interface
            .expect("interface archive checked above");
        let myroom = stage_state(&interface, State::MyRoom, StageContext::default())
            .art
            .expect("MyRoom block stages from the installed archive");
        let selection = myroom
            .placements
            .iter()
            .find(|placed| placed.code == MYROOM_SELECT_CODE)
            .expect("MYBAGSELECT control is staged");
        let sprite = selection
            .sprite
            .as_deref()
            .expect("MYBAGSELECT names a sprite body");
        assert!(
            myroom.figures.contains_key(sprite),
            "Myroom_MybagSelect.oji decodes through the live staging path"
        );
        let cell = myroom
            .placements
            .iter()
            .find(|placed| placed.code == MYROOM_BAG_FIRST_CODE)
            .expect("MYBAGITEM1 is staged");
        live.ingest_snapshot(&InputSnapshot::click_at(
            cell.rect.x + i32::try_from(cell.rect.w / 2).unwrap_or(0),
            cell.rect.y + i32::try_from(cell.rect.h / 2).unwrap_or(0),
        ));
        assert_eq!(live.myroom().bag_cell(), Some(0));
        let mut picked = vec![0u32; 800 * 600];
        live.paint_at(
            &mut RenderFrame {
                buf: &mut picked,
                w: 800,
                h: 600,
            },
            0,
        );
        assert_ne!(
            painted, picked,
            "a live bag pick repaints the staged selection frame"
        );
        let amber = pack_xrgb(0xFF, 0xD0, 0x20);
        assert_eq!(
            picked.iter().filter(|pixel| **pixel == amber).count(),
            0,
            "no amber stand-in survives the live staged path"
        );
        // #532: the bnd-paired ITEMSELL control clears the worn slot
        // on the same live path. No wire shape is pinned, so the
        // press must also leave the session outbox empty (no-op send).
        let vacate = myroom
            .placements
            .iter()
            .find(|placed| placed.code == MYROOM_VACATE_CODE)
            .expect("ITEMSELL control is staged");
        assert!(
            live.myroom_character()
                .is_some_and(|hero| hero.equipment[0] != 0),
            "the live hero wears the first table item before vacating"
        );
        live.ingest_snapshot(&InputSnapshot::click_at(
            vacate.rect.x + i32::try_from(vacate.rect.w / 2).unwrap_or(0),
            vacate.rect.y + i32::try_from(vacate.rect.h / 2).unwrap_or(0),
        ));
        assert_eq!(
            live.myroom_character().map(|hero| hero.equipment[0]),
            Some(0),
            "the ITEMSELL press clears the selected worn slot on the live path"
        );
        server.join().unwrap();
    }

    /// One solid-colour sprite layer, colourkey white so every pixel paints.
    fn solid_figure(w: usize, h: usize, word: [u8; 2], dx: i32, dy: i32) -> AvatarFigure {
        let mut pixels = vec![0u8; w * h * 2];
        for pixel in pixels.as_chunks_mut::<2>().0 {
            pixel.copy_from_slice(&word);
        }
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&pixels, w, h).unwrap(),
            dx,
            dy,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Stage a synthetic Planet board, Channel dialog and one-OK sheet:
    /// the three layers the exe keeps behind the relay notice (#244 pack
    /// `baf781b8…`). No game data, so CI runs the same path.
    fn stage_relay_notice_layers(game: &mut o2jam_shell::ScreenGame) {
        game.load_scene(
            Screen::Planet,
            unbound_scene_placements(&["Board_Backdrop.ojs".to_owned()]),
            [(
                "Board_Backdrop.ojs".to_owned(),
                solid_figure(800, 600, [0x1F, 0x00], 0, 0),
            )]
            .into(),
        );
        game.load_scene(
            Screen::Channel,
            Vec::new(),
            [(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(800, 466, [0xE0, 0x03], 0, 0),
            )]
            .into(),
        );
        game.load_relay_down_scene(
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "information".to_owned(),
                code: 0,
                sprite: Some("Dialog_Information.ojs".to_owned()),
                rect: o2jam_render::Rect {
                    x: 241,
                    y: 206,
                    w: 100,
                    h: 60,
                },
            }],
            [(
                "Dialog_Information.ojs".to_owned(),
                solid_figure(100, 60, [0x00, 0x7C], 241, 206),
            )]
            .into(),
        );
    }

    /// Paint one frame through the live path at 800x600.
    fn paint_relay_frame(game: &mut o2jam_shell::ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut RenderFrame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        buf
    }

    /// Ticket #244 with the #348 boundary: `run_online` ->
    /// `relay_gate::relay_down_now` ->
    /// `stage_channel_list_after_relay_gate` ->
    /// `ScreenGame::hold_channel_rows_for_relay_notice`. The exe's
    /// qualified pack (`~/scratch/p244-capture`, popup sha256
    /// `baf781b8…`) pins the one-OK box at channel entry; boot only
    /// holds the rows, leaving the splash and board untouched.
    #[test]
    fn relay_down_boot_holds_channel_rows_for_the_notice() {
        let channels = vec![o2jam_net::ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 10,
            population: 1,
            active: true,
        }];
        let mut game = o2jam_shell::ScreenGame::new();
        crate::stage_channel_list_after_relay_gate(&mut game, channels, true);
        assert!(game.relay_down_notice_pending());
        assert!(!game.relay_down_dialog_open());
        assert!(game.channel_rows().is_empty());
    }

    /// Boot boundary (#348): the live chain
    /// `run_online` -> `stage_channel_list_after_relay_gate` holds the
    /// relay-down rows without opening the notice. The splash and the
    /// bare planet board stay clean; the notice opens only at the
    /// Planet -> Channel entry edge, over the channel frame. OK still
    /// reveals the held rows in Channel (#244).
    #[test]
    fn relay_down_notice_waits_for_channel_entry_and_leaves_boot_on_avi() {
        let channels = vec![o2jam_net::ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 10,
            population: 1,
            active: true,
        }];
        let mut game = o2jam_shell::ScreenGame::new();
        crate::stage_channel_list_after_relay_gate(&mut game, channels.clone(), true);

        assert_eq!(game.screen(), Screen::Avi, "boot still lands on the splash");
        assert!(
            !game.relay_down_dialog_open(),
            "the notice is not painted before channel entry"
        );
        assert!(game.channel_rows().is_empty(), "rows stay held");
        assert!(
            game.relay_down_notice_pending(),
            "the held rows wait for channel entry"
        );

        assert!(game.enter_screen(Screen::Planet));
        assert_eq!(
            game.screen(),
            Screen::Planet,
            "splash dismiss lands on planet"
        );
        assert!(
            !game.relay_down_dialog_open(),
            "the bare planet board stays clean"
        );

        assert!(game.enter_screen(Screen::Channel));
        assert!(
            game.relay_down_dialog_open(),
            "channel entry opens the notice"
        );
        assert!(
            !game.relay_down_notice_pending(),
            "the notice is open, no longer waiting"
        );
        assert!(
            game.channel_rows().is_empty(),
            "rows stay held behind the notice"
        );
        assert_eq!(
            game.screen(),
            Screen::Channel,
            "the notice sits over the channel frame"
        );

        assert!(game.dismiss_relay_down_dialog());
        assert_eq!(game.channel_rows(), channels, "OK reveals the held rows");
        assert_eq!(game.screen(), Screen::Channel, "OK keeps the channel frame");
    }

    /// Relay-notice composition: the exe's `baf781b8…` frame keeps the
    /// Planet board, Channel dialog and held rows visible behind the
    /// one-OK sheet; our wave-2 frame `deee19d9…` showed black.
    /// Live chain: `run_online` -> `stage_channel_list_after_relay_gate`
    /// -> `hold_channel_rows_for_relay_notice` -> `enter_screen(Channel)`
    /// -> `open_relay_down_dialog` -> `ScreenGame::paint_at` ->
    /// `paint_channel` -> `paint_channel_scene` (board + dialog + held
    /// rows) -> `paint_planet_modal` -> `paint_relay_down_dialog`.
    #[test]
    fn relay_notice_paints_the_board_and_dialog_behind_the_sheet() {
        let channels = vec![ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 10,
            population: 1,
            active: true,
        }];
        let mut held = o2jam_shell::ScreenGame::new();
        stage_relay_notice_layers(&mut held);
        held.hold_channel_rows_for_relay_notice(channels);
        assert!(held.enter_screen(Screen::Planet));
        assert!(held.enter_screen(Screen::Channel));
        assert_eq!(held.planet_modal(), Some(PlanetModal::RelayDown));

        let mut empty = o2jam_shell::ScreenGame::new();
        stage_relay_notice_layers(&mut empty);
        empty.hold_channel_rows_for_relay_notice(Vec::new());
        assert!(empty.enter_screen(Screen::Planet));
        assert!(empty.enter_screen(Screen::Channel));

        let held_frame = paint_relay_frame(&mut held);
        let empty_frame = paint_relay_frame(&mut empty);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        let red = pack_xrgb(0xFF, 0x00, 0x00);
        assert_eq!(held_frame[550 * 800 + 700], blue, "board MENU strip");
        assert_eq!(held_frame[100 * 800 + 700], green, "dialog body");
        assert_eq!(held_frame[220 * 800 + 250], red, "one-OK sheet");
        assert_ne!(
            held_frame[400 * 800 + 250],
            empty_frame[400 * 800 + 250],
            "held rows paint behind the notice"
        );
        assert!(
            held.channel_rows().is_empty(),
            "held rows stay non-interactive until OK"
        );
    }

    /// Ticket #504: the relay-down notice's wire half. Live chain:
    /// `main.rs::run_online` -> `connect::connect_and_login_with_transport`
    /// (1000/1001/1002/1003 on the gateway) -> `stage_channel_list_after_relay_gate`
    /// -> `ScreenGame::hold_channel_rows_for_relay_notice` ->
    /// `enter_screen(Channel)` -> `open_relay_down_dialog` ->
    /// `paint_channel` -> `paint_planet_modal` -> OK at the pinned
    /// (400,365) `Dialog_Information` rect -> `dismiss_relay_down_dialog`.
    /// The exe's close-cp boot-2 pack pins this wire order and the
    /// popup frame before the channel rows become visible.
    ///
    /// Ticket #510 pin: relay-down manifest `64caf0c4…8ebd629bbec7208`,
    /// citation `30d1b061…2e5290c7`, tap `a33fc8d1…a40387e08913`:
    /// 1000/1001/1002/1003 at 15:37:32, popup blue 0.986 at :33, no
    /// gateway frame between the 1003 answer and the sheet. The exact
    /// `order` assertion below is the client half of that witness; the
    /// elapsed popup timing stays an exe-capture pin.
    #[test]
    fn relay_down_notice_holds_the_pinned_1003_rows_until_ok() {
        let row = ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 10,
            population: 3,
            active: true,
        };
        let (port, server) = spawn_boot_gateway_with_channels(vec![row]);
        let (report, _gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let (hero, order) = server.join().unwrap();

        assert_eq!(
            order,
            [
                Request::Authorize.id(),
                Response::Authorize.id(),
                Request::GetChannelList.id(),
                Response::GetChannelList.id(),
                Request::ChannelLogin.id(),
                Response::ChannelLogin.id(),
                Request::GetChannelInfo.id(),
                Response::RoomList.id(),
                Request::GetCharacterInfo.id(),
                Response::GetCharacterInfo.id(),
            ],
            "exe boot wire order 1000/1001/1002/1003 before the notice"
        );
        assert_eq!(report.channels, [row], "1003 carries the held row");

        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, Some(&hero));
        crate::stage_channel_list_after_relay_gate(&mut game, report.channels, true);
        assert_eq!(game.screen(), Screen::Avi, "notice is not a boot sheet");
        assert!(game.relay_down_notice_pending());
        assert!(game.enter_screen(Screen::Planet));
        assert!(game.enter_screen(Screen::Channel));
        assert_eq!(
            game.planet_modal(),
            Some(PlanetModal::RelayDown),
            "1003 rows arrive behind the exe's notice"
        );
        assert!(game.channel_rows().is_empty(), "the notice holds the rows");

        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(400, 365));
        assert_eq!(game.planet_modal(), None, "OK releases the held rows");
        assert_eq!(game.channel_rows(), [row]);
    }

    /// Ticket #393: the blue "NORMAL mode only" sheet owns the
    /// board after a lit emblem click. Live chain:
    /// `main.rs::run_online` -> `connect_and_login_with_transport` ->
    /// `stage_channel_list_after_relay_gate` -> splash dismissal ->
    /// `ScreenGame::click_planet` at the pinned EASY rect ->
    /// `dispatch_planet_action` -> `enter_screen(Channel)` ->
    /// `open_relay_down_dialog` -> `click_dispatch` ->
    /// `click_planet_modal` -> OK at the pinned (400,365) rect.
    /// `docs/captures/t240/wine-drive.log` pins the popup
    /// (`max_blue=0.986`, fallback OK, `max_blue=0.000`).
    #[test]
    fn normal_mode_popup_blocks_canvas_clicks_after_lit_emblem() {
        use std::collections::HashMap;

        use o2jam_shell::avatar::{PlacementRole, ScenePlacement};
        use o2jam_shell::screens::side::{PLANET_BUTTON_EASY_CODE, PLANET_EASY_RECT};

        let row = ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 10,
            population: 3,
            active: true,
        };
        let (port, server) = spawn_boot_gateway_with_channels(vec![row]);
        let (report, _gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let (hero, _) = server.join().unwrap();

        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, Some(&hero));
        crate::stage_channel_list_after_relay_gate(&mut game, report.channels, true);
        game.load_scene(
            Screen::Planet,
            vec![ScenePlacement {
                id: "easy".to_owned(),
                code: PLANET_BUTTON_EASY_CODE,
                sprite: None,
                rect: PLANET_EASY_RECT,
                role: PlacementRole::Single,
            }],
            HashMap::new(),
        );
        // #385: the pinned ESC key leaves Avi; a canvas click is not
        // an exe dismiss edge any more.
        game.menu_key(o2jam_shell::MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);

        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(136, 88));
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.planet_modal(), Some(PlanetModal::RelayDown));

        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(526, 248));
        assert_eq!(
            game.planet_modal(),
            Some(PlanetModal::RelayDown),
            "the sheet still owns the canvas after a row-face click"
        );
        assert!(game.channel_rows().is_empty(), "rows stay behind the sheet");

        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(400, 365));
        assert_eq!(game.planet_modal(), None, "OK clears the blue sheet");
        assert_eq!(game.channel_rows(), [row]);
    }

    /// Ticket #409: conformance of the client's three-record cycle
    /// against the exe's pinned bare-board and dialog windows. Live
    /// chain: `main.rs::run_online` -> `setup_online` ->
    /// `assemble_online_session` -> `OnlineSession::stage_live_art`
    /// -> `ScreenGame::paint_at` -> `tick_planet_promo_at` ->
    /// `PlanetBoardState::tick_promo_clocked`.
    /// Solo gaps `3056.5/2957.9/3052.9` (citation `d135105a…4ef3b0fe`);
    /// guest/dialog gaps `2955.6/3053.6/2956.2` and
    /// `2975.7/2966.4/3066.1` (boot-4 manifest `78e0617b…7c3ba9de8`).
    #[test]
    fn promo_cadence_conforms_on_solo_guest_and_dialog_arms() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        // The exe's slowest and fastest three-gap samples bound the
        // 100 ms-poll cadence on every arm; our clock steps at 3000 ms.
        const MIN_EXE_GAP_MS: u64 = 2_955;
        const MAX_EXE_GAP_MS: u64 = 3_067;

        for user in ["solo", "guest"] {
            let (port, server) = spawn_boot_gateway();
            let (report, gateway) =
                connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
            let mut game = o2jam_shell::ScreenGame::new();
            stage_online_identity(&mut game, report.character.as_ref());
            let mut session =
                assemble_online_session(game, gateway, Some(user), None, None, None).unwrap();
            server.join().unwrap();
            let game = session.game_mut();
            game.menu_key(o2jam_shell::MenuKey::Back, true);
            assert_eq!(game.screen(), Screen::Planet, "{user} reaches the board");

            let changes = promo_changes_between(game, 0, 9_500);
            assert_eq!(changes.len(), 3, "{user}: three record changes");
            let shown: Vec<u32> = changes.iter().map(|(_, index)| *index).collect();
            assert_eq!(shown, [2, 3, 1], "{user}: three-record cycle");
            let instants: Vec<u64> = changes.iter().map(|(ms, _)| *ms).collect();
            let gaps: Vec<u64> = instants.windows(2).map(|pair| pair[1] - pair[0]).collect();
            assert_eq!(gaps.len(), 2, "{user}: two measured gaps");
            assert!(
                gaps.iter()
                    .all(|gap| (MIN_EXE_GAP_MS..=MAX_EXE_GAP_MS).contains(gap)),
                "{user}: cadence inside the exe envelope, got {gaps:?}"
            );
        }

        let (port, server) = spawn_boot_gateway();
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        let mut session =
            assemble_online_session(game, gateway, Some("dialog"), None, None, None).unwrap();
        server.join().unwrap();
        let game = session.game_mut();
        game.menu_key(o2jam_shell::MenuKey::Back, true);
        game.menu_key(o2jam_shell::MenuKey::Confirm, true);
        assert_eq!(
            game.screen(),
            Screen::Channel,
            "dialog raised over the board"
        );

        let held = promo_changes_between(game, 0, 9_500);
        assert!(held.is_empty(), "the dialog arm holds the banner");
        game.enter_screen(Screen::Planet);
        let resumed = promo_changes_between(game, 9_500, 12_500);
        assert_eq!(resumed.len(), 1, "return resumes one tick, not a backlog");
        assert_eq!(resumed[0].1, 2, "the cycle resumes on the next record");
    }

    /// Count bright text pixels and magenta ribbon pixels in one
    /// native-frame region. The two colours separate the five dated
    /// cell captions from the diagonal NEW ITEM ribbon in the pinned
    /// exe frame.
    fn promo_ink_counts(
        frame: &[u32],
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
    ) -> (u32, u32) {
        let (mut bright, mut magenta) = (0, 0);
        for y in top..bottom {
            for x in left..right {
                let pixel = frame[y * 800 + x];
                let red = pixel & 0xff;
                let green = (pixel >> 8) & 0xff;
                let blue = (pixel >> 16) & 0xff;
                if red > 0xb4 && green > 0xb4 && blue > 0xb4 {
                    bright += 1;
                }
                if red > 0x8c && blue > 0x8c && green + 0x3c < red && green + 0x3c < blue {
                    magenta += 1;
                }
            }
        }
        (bright, magenta)
    }

    /// Ticket #400: the five dated promo cells and the NEW ITEM
    /// ribbon are the real banner art, not a flat stand-in. Live
    /// chain: `main.rs::run_online` -> `setup_online` ->
    /// `assemble_online_session` -> `OnlineSession::stage_live_art`
    /// -> `live_art::stage_interface_scenes` -> `ScreenGame::load_scene`
    /// -> `paint_at` -> `paint_planet_scene`. Region counts come from
    /// the exe frame sha256 `235f5352…c21` in `docs/captures/t240/`;
    /// the ±40 envelope covers archive decode and the capture's
    /// screenshot scaling. Skips cleanly without game data (#0a3ba).
    #[test]
    fn promo_banner_stages_five_dated_cells_and_new_item_ribbon() {
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
        server.join().unwrap();
        let game = session.game_mut();
        game.menu_key(o2jam_shell::MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);

        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut RenderFrame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );

        // (label, left, top, right, bottom, exe bright, exe magenta)
        let regions = [
            ("1일", 271, 390, 322, 450, 200, 0),
            ("1일+이벤트", 324, 390, 397, 450, 255, 0),
            ("7일", 400, 390, 456, 450, 173, 0),
            ("30일", 458, 390, 514, 450, 112, 0),
            ("90일", 516, 390, 635, 450, 431, 0),
            ("NEW ITEM", 57, 175, 166, 260, 774, 2478),
        ];
        for (label, left, top, right, bottom, exe_bright, exe_magenta) in regions {
            let (bright, magenta) = promo_ink_counts(&buf, left, top, right, bottom);
            assert!(
                bright.abs_diff(exe_bright) <= 40,
                "{label}: bright ink {bright} differs from exe {exe_bright}"
            );
            assert!(
                magenta.abs_diff(exe_magenta) <= 40,
                "{label}: magenta ink {magenta} differs from exe {exe_magenta}"
            );
        }
    }

    /// Same boot chain, relay-up arm: no notice, the existing
    /// `set_channels` path still loads the rows immediately.
    #[test]
    fn relay_up_boot_keeps_the_existing_channel_path() {
        let channels = vec![o2jam_net::ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 10,
            population: 1,
            active: true,
        }];
        let mut game = o2jam_shell::ScreenGame::new();
        crate::stage_channel_list_after_relay_gate(&mut game, channels.clone(), false);
        assert!(!game.relay_down_dialog_open());
        assert_eq!(game.channel_rows(), channels);
    }

    /// The information sheet comes from `Dialog_Information.bnd`
    /// via the same declaration the live boot stages; skip cleanly
    /// in CI without archives (#0a3ba).
    #[test]
    fn relay_down_sheet_stages_from_the_installed_information_block() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let archive = o2jam_shell::staging::read_install_archives(&install_root())
            .interface
            .expect("interface archive present");
        let mut game = o2jam_shell::ScreenGame::new();
        crate::live_art::stage_relay_down_sheet(&mut game, &archive);
        assert!(game.relay_down_art_staged());
    }

    /// #367 arming boundary: the same Information sheet the relay
    /// notice uses stages on relay-down + Room + a starved tracked
    /// 07DA poll, never at boot or on the bare planet board (#348).
    /// `Instant` is injected: the exe's 20 s wall-clock gate is
    /// checked with an old stamp, with no fabricated 36 s sleep.
    ///
    /// Live caller chain: `main.rs::run_online` ->
    /// `stage_channel_list_after_relay_gate` ->
    /// `ScreenGame::set_room_no_response_relay_down` -> Room entry ->
    /// `OnlineSession::poll_wire` -> `poll_room_user_list_fuse` ->
    /// `ScreenGame::pump_room_user_list_fuse_at` -> `RoomNoResponseFuse::pump`
    /// -> `paint_room_no_response_dialog`. This boot-gate test drives
    /// the same staging seam at tier 3; only the socket and `Instant`
    /// are injected.
    #[test]
    fn room_fuse_sheet_opens_only_in_room_after_07da_poll_starves() {
        use std::time::Instant;

        let mut game = o2jam_shell::ScreenGame::new();
        crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);

        game.pump_room_user_list_fuse();
        assert!(!game.room_no_response_fuse_enabled(), "fuse is Room-only");
        assert!(
            !game.room_no_response_dialog_open(),
            "boot Avi never shows the Room fuse"
        );
        game.enter_screen(Screen::Planet);
        game.pump_room_user_list_fuse();
        assert!(!game.room_no_response_fuse_enabled());
        assert!(
            !game.room_no_response_dialog_open(),
            "the bare planet board never shows the Room fuse"
        );

        game.enter_screen(Screen::Channel);
        assert!(!game.room_no_response_fuse_enabled());
        game.enter_screen(Screen::Room);
        assert!(game.room_no_response_fuse_enabled());
        let now = Instant::now();
        assert!(game.track_room_user_list_send_at(
            now.checked_sub(Duration::from_secs(21))
                .expect("clock margin")
        ));
        game.pump_room_user_list_fuse_at(now);
        assert!(
            game.room_no_response_dialog_open(),
            "starved 07DA node opens the sheet in Room"
        );
    }

    /// The sheet and its OK rect come from the installed
    /// `Dialog_Information.bnd` block staging, not from the shared
    /// fallback aim. Skips cleanly in CI without archives (#0a3ba).
    #[test]
    fn room_fuse_ok_uses_the_staged_information_block_rect() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let archive = o2jam_shell::staging::read_install_archives(&install_root())
            .interface
            .expect("interface archive present");
        let mut game = o2jam_shell::ScreenGame::new();
        crate::live_art::stage_relay_down_sheet(&mut game, &archive);
        crate::stage_channel_list_after_relay_gate(&mut game, vec![], true);
        game.enter_screen(Screen::Planet);
        game.enter_screen(Screen::Channel);
        game.enter_screen(Screen::Room);
        let now = Instant::now();
        assert!(game.track_room_user_list_send_at(
            now.checked_sub(Duration::from_secs(21))
                .expect("clock margin")
        ));
        game.pump_room_user_list_fuse_at(now);
        assert!(game.room_no_response_sheet_staged());

        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(175, 150));
        assert!(
            game.room_no_response_dialog_open(),
            "a click outside the staged OK rect does not dismiss"
        );
        game.ingest_snapshot(&o2jam_shell::InputSnapshot::click_at(400, 365));
        assert_eq!(game.room_no_response_dismissal_queued(), Some(100));
        game.pump_dialog_queue();
        assert!(
            !game.room_no_response_dialog_open(),
            "the staged OK rect dismisses through the dialog queue"
        );
    }
}
