//! One live-input leg per player-visible behavior family (#739).
//!
//! The scripted gateway guards in `session.rs` call `menu_key` and the
//! loading gate directly: fast and broad, but blind to the input
//! mapping that broke live BACK (#734). These legs start at the real
//! `connect` boot entry and drive the shell through pointer snapshots
//! only. They skip cleanly when the installed archives are absent —
//! CI has no game data (#0a3ba).

#[cfg(test)]
mod live_input_leg_tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::rc::Rc;
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
    use std::sync::Mutex;
    use std::thread::JoinHandle;
    use std::time::Duration;

    use o2jam_audio::{AudioBackend, NullBackend, PcmSamples};
    use o2jam_net::frame::encode_frame;
    use o2jam_net::shop::EnterShop;
    use o2jam_net::testing::{fake_gateway, read_frame};
    use o2jam_net::{
        AuthResponse, ChannelList, ChannelLoginResponse, ChannelState, EmptyMessage, Frame, Gender,
        JoinRoom, JoinRoomResponse, Request, RoomList, RoomListEntry, RoomSlotState, WireMessage,
    };
    use o2jam_render::{Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::screens::game::{LobbyChat, PlanetModal, RoomListFilter, ScreenGame};
    use o2jam_shell::screens::messenger::MessengerHost;
    use o2jam_shell::screens::paint::dialog_origin;

    use o2jam_shell::screens::side::{
        MyRoomLowerTab, MyRoomNotice, MyRoomPanel, SideScreen, INFORMATION_OK_CODE,
        MYROOM_GOSHOP_CODE, OPTION_GAME_KEYSET, OPTION_KEY_3KEY_FIRST, OPTION_KEY_3KEY_MODE,
        OPTION_KEY_7KEY_MODE, OPTION_KEY_LANE_FIRST, OPTION_KEY_SOUNDSET, OPTION_KEY_TEST,
        OPTION_SOUND_GAMESET, OPTION_SOUND_KEYSET, PLANET_OPTION_CODE, SHOP_CATEGORY_FIRST_CODE,
        SHOP_CHILD_ORIGIN_Y, SHOP_EXTMENU_FIRST_CODE, SHOP_EXTMENU_ON_CODE, SHOP_GOMYROOM_CODE,
        SHOP_ITEMBUY_FIRST_CODE, SHOP_SCROLL_RIGHT_CODE, SHOP_SUBMENU_FIRST_CODE,
    };
    use o2jam_shell::staging::{
        read_install_archives, stage_dialog, stage_state, Dialog, StageContext, State,
    };

    use o2jam_shell::{
        route_key_event, GameLoop, InputEdge, InputSnapshot, KeyRoute, Keymap, Screen,
    };
    use winit::keyboard::KeyCode;

    use crate::{
        assemble_online_session, connect, install_root, session::OnlineSession,
        stage_channel_list_after_relay_gate, stage_online_identity, stage_online_rooms,
    };
    use o2jam_shell::screens::side::{PlanetOptionTab, ROOM_OPTION_BUTTON_CODE};

    /// Live boot rows: one channel (the resident one) and one room.
    fn channel_row() -> ChannelState {
        ChannelState {
            gateway_id: 1,
            channel_id: 2,
            capacity: 100,
            population: 1,
            active: true,
        }
    }

    fn room_row() -> RoomListEntry {
        RoomListEntry {
            number: 3,
            state_code: 1,
            title: "Lobby".to_owned(),
            has_password: false,
            music_id: 0,
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
            character: false,
            wallet: false,
            avatar_opa: None,
            join_room: None,
            room_password: String::new(),
            song: None,
        }
    }

    /// Handles of a live-leg gateway: its port, recorded client frames,
    /// 3003 write signal, and server thread (#754).
    struct LiveLegGateway {
        port: u16,
        reports: Receiver<Frame>,
        join_answer_written: Receiver<()>,
        server: JoinHandle<()>,
    }

    /// Drive the 1000/1001/1002/1003 boot exchange, then record every
    /// later client frame until the socket goes quiet. The channel and
    /// room rows come back on the wire, not from a test-side list. The
    /// 3003 write is signalled so a Waiting click never races the
    /// gateway thread's scheduling under fleet load.
    fn spawn_live_leg_gateway(joined: o2jam_net::JoinedRoom) -> LiveLegGateway {
        spawn_leg_gateway(joined, false)
    }

    /// Same boot gateway with the two result answers: 4011 on START
    /// and 4022 only on the second 4021 (the first is the tally
    /// triple's entry exit, the second is the BACK click).
    fn spawn_result_back_gateway(joined: o2jam_net::JoinedRoom) -> LiveLegGateway {
        spawn_leg_gateway(joined, true)
    }

    fn spawn_leg_gateway(
        joined: o2jam_net::JoinedRoom,
        answer_result_back: bool,
    ) -> LiveLegGateway {
        let (reported, reports) = mpsc::channel();
        let (join_written_tx, join_answer_written) = mpsc::channel();
        let (port, server) = fake_gateway(move |mut stream| {
            assert_eq!(read_frame(&mut stream).id, Request::Authorize.id());
            stream
                .write_all(
                    &AuthResponse {
                        result_code: 0,
                        tail: vec![],
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelList.id());
            stream
                .write_all(
                    &ChannelList {
                        channels: vec![channel_row()],
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::ChannelLogin.id());
            stream
                .write_all(
                    &ChannelLoginResponse {
                        result_code: 0,
                        restricted: Some(false),
                    }
                    .encode(),
                )
                .unwrap();
            assert_eq!(read_frame(&mut stream).id, Request::GetChannelInfo.id());
            stream
                .write_all(
                    &RoomList {
                        rooms: vec![room_row()],
                    }
                    .encode(),
                )
                .unwrap();

            // The socket stays open until the test drops the session:
            // boot staging reads large archives, so a short quiet
            // timeout here would close under the test's feet (#52).
            let mut exit_playing_count = 0u32;
            while let Some(frame) = read_frame_until_eof(&mut stream) {
                if answer_result_back && frame.id == Request::StartGame.id() {
                    stream.write_all(&encode_frame(4011, &[])).unwrap();
                }
                if frame.id == Request::ExitPlaying.id() {
                    exit_playing_count += 1;
                    if answer_result_back && exit_playing_count == 2 {
                        stream.write_all(&encode_frame(4022, &[0u8; 13])).unwrap();
                    }
                }
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
                reported.send(frame).unwrap();
            }
        });
        LiveLegGateway {
            port,
            reports,
            join_answer_written,
            server,
        }
    }

    /// One whole frame, or `None` at client close.
    fn read_frame_until_eof(stream: &mut TcpStream) -> Option<Frame> {
        let mut header = [0u8; o2jam_net::frame::HEADER_LEN];
        stream.read_exact(&mut header).ok()?;
        let body_len = usize::from(u16::from_le_bytes([header[0], header[1]]));
        let mut bytes = header.to_vec();
        let mut body = vec![0u8; body_len.saturating_sub(o2jam_net::frame::HEADER_LEN)];
        stream.read_exact(&mut body).ok()?;
        bytes.extend_from_slice(&body);
        Some(o2jam_net::frame::decode_frame(&bytes).unwrap())
    }

    /// The 3003 snapshot shape: a seven-seat room with one peer. The
    /// host/guest seat and the peer's readiness decide the Waiting
    /// confirm arm the live click must reach.
    fn joined_room(member_id: u8, peer_ready: bool) -> o2jam_net::JoinedRoom {
        let slots = (0..7u8)
            .map(|index| o2jam_net::RoomSlot {
                index,
                state_code: 0,
                member: None,
            })
            .collect();
        let mut room = o2jam_net::JoinedRoom {
            member_id,
            team: if member_id == 0 { 0 } else { 1 },
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
        };
        let peer_seat: u8 = if member_id == 0 { 1 } else { 0 };
        // The wire decoder reads a member only for an occupied slot.
        room.slots[usize::from(peer_seat)].state_code = RoomSlotState::Occupied as i32;
        room.slots[usize::from(peer_seat)].member = Some(o2jam_net::RoomMember {
            nickname: "peer".to_owned(),
            level: 1,
            gender: 0,
            gem: 0,
            master: false,
            team: if member_id == 0 { 1 } else { 0 },
            ready: peer_ready,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        });
        room
    }

    /// The real boot entry plus the same resident row the windowed
    /// `run_online` stages after login (#348). `relay_down` arms the
    /// one-OK notice that holds the channel rows.
    fn boot_live_leg(port: u16, relay_down: bool) -> OnlineSession {
        let (report, gateway) =
            connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
        let mut game = o2jam_shell::ScreenGame::new();
        stage_online_identity(&mut game, report.character.as_ref());
        stage_channel_list_after_relay_gate(&mut game, report.channels.clone(), relay_down);
        stage_online_rooms(&mut game, report.rooms.clone());
        let mut session =
            assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
        session.set_resident_channel(channel_row());
        session
    }

    fn archives_present() -> bool {
        read_install_archives(&install_root()).interface.is_some()
    }

    /// Boot path used by `main.rs::run_play`: the session stages live
    /// art before any frame, so dialog rects and figures are real.
    fn boot_live_leg_with_art(port: u16, relay_down: bool) -> OnlineSession {
        let mut session = boot_live_leg(port, relay_down);
        session.stage_live_art();
        session
    }

    /// One staged option control's absolute native rect. Geometry is
    /// always read from the same `.bnd` block live boot stages.
    fn staged_dialog_rect(dialog: Dialog, code: u32) -> Rect {
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive present behind the guard");
        let art = stage_dialog(&archive, dialog, StageContext::default())
            .art
            .expect("option sheet stages with installed bytes");
        let (origin_x, origin_y) = dialog_origin(&art.placements);
        let placed = art
            .placements
            .iter()
            .find(|placed| placed.code == code)
            .unwrap_or_else(|| panic!("no staged rect for {code:#x}"));
        Rect {
            x: origin_x + placed.rect.x,
            y: origin_y + placed.rect.y,
            w: placed.rect.w,
            h: placed.rect.h,
        }
    }

    fn staged_dialog_centre(dialog: Dialog, code: u32) -> (i32, i32) {
        rect_centre(staged_dialog_rect(dialog, code))
    }

    /// Which lobby face the Options sheet was opened from. The dialog
    /// has no per-caller state (#314 static pin), so both entries must
    /// land on the same capture and paint behavior.
    #[derive(Debug, Clone, Copy)]
    enum OptionsEntry {
        Room,
        Planet,
    }

    /// Boot -> splash -> entry click -> Options sheet -> Key tab, all
    /// through pointer snapshots. No direct `switch_option_tab` call.
    fn open_options_key_sheet(session: &mut OnlineSession, entry: OptionsEntry) {
        let (entry_screen, code) = match entry {
            OptionsEntry::Room => (Screen::Room, ROOM_OPTION_BUTTON_CODE),
            OptionsEntry::Planet => (Screen::Planet, PLANET_OPTION_CODE),
        };
        if matches!(entry, OptionsEntry::Room) {
            enter_room_by_clicks(session);
        }
        let (x, y) = staged_code_centre(session, entry_screen, code);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(
            session.game_mut().planet_modal(),
            Some(PlanetModal::Options),
            "{entry:?} entry must open the one Options dialog"
        );
        let (x, y) = staged_dialog_centre(Dialog::PlanetOptionSound, OPTION_SOUND_KEYSET);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(
            session.game_mut().planet_option_tab(),
            PlanetOptionTab::Key,
            "the staged KEYSET tab must reach the Key sheet from {entry:?}"
        );
    }

    /// Send one raw key event through the shared window/headless route
    /// and deliver its queued edges in the next live snapshot.
    fn route_raw_key(
        session: &mut OnlineSession,
        keymap: &mut Keymap,
        code: KeyCode,
        text: Option<&str>,
    ) -> KeyRoute {
        let route = route_key_event(session.game_mut(), keymap, code, true, text);
        let mut snapshot = InputSnapshot::new();
        for edge in &route.edges {
            snapshot.push_edge(*edge);
        }
        session.ingest_snapshot(&snapshot);
        route
    }

    /// Click one staged rebind cell and answer it with a mapped key,
    /// both through the live input seams.
    fn rebind_staged_lane(
        session: &mut OnlineSession,
        keymap: &mut Keymap,
        cell_code: u32,
        key: KeyCode,
        text: &str,
    ) {
        let (x, y) = staged_dialog_centre(Dialog::PlanetOption, cell_code);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        let route = route_raw_key(session, keymap, key, Some(text));
        assert!(
            route.captured && route.text.is_none(),
            "the dialog must own a mapped rebind key: {route:?}"
        );
    }

    /// Observable state around an unmapped key: screen, modal, tab,
    /// armed lane, binds, hash, and the Room draft.
    fn key_sheet_state(session: &mut OnlineSession) -> (String, String) {
        let game = session.game_mut();
        let screen = format!("{:?}", game.screen());
        let modal = format!(
            "{:?}|{:?}|{:?}|{:?}",
            game.planet_modal(),
            game.planet_option_tab(),
            game.option_rebind_armed(),
            game.options().lane_keys(),
        );
        let draft = LobbyChat::of_screen(game.screen())
            .map(|chat| game.lobby_chat(chat).draft().to_owned())
            .unwrap_or_default();
        let observed = format!("{screen}|{modal}|{}|{draft}", game.state_hash());
        (observed, draft)
    }

    /// Native pixels inside one absolute staged rect, from the same
    /// paint call the window makes.
    fn staged_rect_pixels(session: &mut OnlineSession, rect: Rect) -> Vec<u32> {
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

    /// Number of pixels that carry a colour other than the most common
    /// one in a staged control rect. A blanked key face collapses to
    /// its backdrop; a painted face keeps glyph ink.
    fn ink_pixels(pixels: &[u32]) -> usize {
        let mut counts = std::collections::HashMap::new();
        for pixel in pixels {
            *counts.entry(*pixel).or_insert(0usize) += 1;
        }
        let dominant = counts.values().copied().max().unwrap_or(0);
        pixels.len().saturating_sub(dominant)
    }

    /// Sample one absolute staged rect from an already painted frame.
    fn staged_rect_pixels_from(frame: &[u32], rect: Rect) -> Vec<u32> {
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

    /// The exe's 37-byte DIK order (`0x438080` fill, `0x439230` search;
    /// `docs/options.md` and the #314 static pin). A caption's staged
    /// record is the position of its bound DIK in this table.
    const KEY_CAPTION_DIK_ORDER: [u8; 37] = [
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1E, 0x1F, 0x20,
        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32,
        0x33, 0x34, 0x35, 0x36, 0x2A, 0x39, 0x1C,
    ];

    /// Pixels the bound caption must paint in one staged rebind cell.
    /// The expected glyph comes from the same `.bnd`/`.opi` figures live
    /// boot staged; the guard never invents a colour or a coordinate.
    fn bound_caption_pixels(rect: Rect, dik: u8) -> Vec<(usize, usize, u32)> {
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive present behind the guard");
        let art = stage_dialog(&archive, Dialog::PlanetOption, StageContext::default())
            .art
            .expect("key sheet stages with installed bytes");
        let figure = art
            .figures
            .get("OptionKey_Keytext.ojs")
            .expect("key caption atlas stages");
        let record = KEY_CAPTION_DIK_ORDER
            .iter()
            .position(|entry| *entry == dik)
            .expect("bound DIK has a staged caption");
        let layer = figure.layer(record).expect("caption record decodes");
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
        let mut expected = vec![0u32; width * height];
        o2jam_render::blit_rgb555(
            &mut o2jam_render::Frame {
                buf: &mut expected,
                w: width,
                h: height,
            },
            &layer.image.view(),
            rect.x + 4,
            rect.y + 4,
            Some(layer.colorkey),
        );
        let rows = i32::try_from(rect.h).expect("rect height fits i32");
        let columns = i32::try_from(rect.w).expect("rect width fits i32");
        (rect.y..rect.y + rows)
            .flat_map(|y| (rect.x..rect.x + columns).map(move |x| (y, x)))
            .filter_map(|(y, x)| {
                let word = expected[usize::try_from(y).expect("y fits usize") * width
                    + usize::try_from(x).expect("x fits usize")];
                (word != 0).then(|| {
                    (
                        usize::try_from(x).expect("x fits usize"),
                        usize::try_from(y).expect("y fits usize"),
                        word,
                    )
                })
            })
            .collect()
    }

    /// Count bound-caption pixels a frame lost or repainted with another
    /// face. A hover that preserves the key ink leaves this number
    /// unchanged from the rest frame.
    fn caption_mismatches(frame: &[u32], caption: &[(usize, usize, u32)]) -> usize {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        caption
            .iter()
            .filter(|(x, y, word)| frame[y * width + x] != *word)
            .count()
    }

    /// Splash dismissal is a Menu edge in a snapshot — the same edge
    /// the window queues for Escape, never a direct `menu_key` call.
    fn dismiss_splash(session: &mut OnlineSession) {
        assert_eq!(session.screen(), Screen::Avi);
        let mut snapshot = InputSnapshot::new();
        snapshot.push_edge(InputEdge::Menu {
            key: o2jam_shell::MenuKey::Back,
            down: true,
        });
        session.ingest_snapshot(&snapshot);
        assert_eq!(session.screen(), Screen::Planet);
    }

    /// Click the centre of a staged `.bnd` placement. No coordinates
    /// are invented here: the painter's own geometry owns the press.
    fn click_staged(session: &mut OnlineSession, screen: Screen, id: &str) {
        let placement = session
            .game_mut()
            .scene_placements(screen)
            .iter()
            .find(|placed| placed.id == id)
            .unwrap_or_else(|| panic!("no staged placement {id} on {screen:?}"));
        let centre = rect_centre(placement.rect);
        session.ingest_snapshot(&InputSnapshot::click_at(centre.0, centre.1));
    }

    /// The OK plate comes from the installed `Dialog_Information.bnd`
    /// sheet, staged by the same `stage_dialog` call as live boot.
    fn information_ok_centre() -> (i32, i32) {
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive present");
        let art = stage_dialog(&archive, Dialog::Information, StageContext::default())
            .art
            .expect("Dialog_Information staged");
        let (origin_x, origin_y) = dialog_origin(&art.placements);
        let ok = art
            .placements
            .iter()
            .find(|placed| placed.code == INFORMATION_OK_CODE)
            .expect("information OK staged");
        let centre = rect_centre(ok.rect);
        (centre.0 + origin_x, centre.1 + origin_y)
    }

    fn rect_centre(rect: Rect) -> (i32, i32) {
        let half_width = i32::try_from(rect.w / 2).expect("rect width fits a frame coordinate");
        let half_height = i32::try_from(rect.h / 2).expect("rect height fits a frame coordinate");
        (rect.x + half_width, rect.y + half_height)
    }

    /// Paint budget for a screen change already driven by input or a
    /// gateway signal. Counting paints, not wall time, keeps a loaded
    /// scheduler out of the assertion (#754).
    const MAX_SCREEN_PAINTS: usize = 64;

    /// Paint until the boot-driven action reaches its next screen.
    fn paint_until_screen(session: &mut OnlineSession, want: Screen) {
        for _ in 0..MAX_SCREEN_PAINTS {
            if session.screen() == want {
                return;
            }
            paint(session);
        }
        assert_eq!(session.screen(), want, "live click never reached {want:?}");
    }

    fn paint(session: &mut OnlineSession) {
        drop(paint_frame(session));
    }

    /// Paint one native frame and return its pixels, so a guard can
    /// prove a mounted sheet actually changed the picture.
    fn paint_frame(session: &mut OnlineSession) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits a buffer index");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits a buffer index");
        let mut buf = vec![0u32; width * height];
        session.paint(&mut o2jam_render::Frame {
            buf: &mut buf,
            w: width,
            h: height,
        });
        buf
    }

    /// Click path to the Room lobby: the pinned emblem face opens the
    /// channel list, the resident row replays locally on its press.
    fn enter_room_by_clicks(session: &mut OnlineSession) {
        session.ingest_snapshot(&InputSnapshot::click_at(136, 88));
        assert_eq!(session.screen(), Screen::Channel);
        session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        paint_until_screen(session, Screen::Room);
    }

    /// Click path to Waiting: the room row press queues 3002 and the
    /// 3003 answer replays the same press into the shell. Waiting on
    /// the gateway's write signal removes the wall-clock race (#754).
    fn join_waiting_by_click(session: &mut OnlineSession, join_answer_written: &Receiver<()>) {
        click_staged(session, Screen::Room, "IDC_BUTTON_SROOM_EXBUTTON1");
        // Paint first: this frame posts 3002. The gateway writes 3003
        // only after it reads that frame, then signals the write.
        paint(session);
        join_answer_written
            .recv()
            .expect("the gateway wrote the 3003 join answer");
        paint_until_screen(session, Screen::Waiting);
    }

    fn observed(reports: &Receiver<Frame>) -> Vec<(u16, Vec<u8>)> {
        reports
            .try_iter()
            .map(|frame| (frame.id, frame.payload))
            .collect()
    }

    /// Wait for one opcode after a paint: the gateway thread records
    /// frames as it reads them, so an immediate `try_iter` can outrun
    /// the socket (#52). The channel is the signal; a closed channel
    /// means the gateway died before the frame reached it (#754).
    fn wait_for_opcode(reports: &Receiver<Frame>, want: u16) -> Vec<u8> {
        loop {
            match reports.recv() {
                Ok(frame) if frame.id == want => return frame.payload,
                Ok(_) => {}
                Err(_) => panic!("gateway closed before opcode {want}"),
            }
        }
    }

    /// Click a staged placement by code, not by a guessed coordinate.
    fn staged_code_centre(session: &mut OnlineSession, screen: Screen, code: u32) -> (i32, i32) {
        let placement = session
            .game_mut()
            .scene_placements(screen)
            .iter()
            .find(|placed| placed.code == code)
            .unwrap_or_else(|| panic!("no staged placement for {code:#x} on {screen:?}"));
        rect_centre(placement.rect)
    }

    /// Closing the client ends the recorder thread's read loop.
    fn finish(session: OnlineSession, server: JoinHandle<()>) {
        drop(session);
        server.join().unwrap();
    }

    /// #294 T3 live-input leg: the settled Result BACK click must wait
    /// for the scripted 4022 before Waiting installs, with the staged
    /// OPI pane in place (#294 T4). The click goes through the same
    /// pointer snapshot a window queues, and the wire is recorded by
    /// the gateway rather than asserted at a seam.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot
    ///   (splash, emblem, room row, join, BACK) ->
    ///   OnlineSession::ingest_snapshot -> ScreenGame::click_result_button
    ///   -> pending_result_back -> OnlineSession::flush_result_wire ->
    ///   C2S 4021 -> scripted S2C 4022 -> Screen::Waiting.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_result_back_sends_4021_and_installs_waiting_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_result_back_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        // The host's chart fetch is not on this gateway; arm the
        // Loading gate the way `fetch_and_stage_chart` does so the
        // walk can reach the Result click under test.
        session
            .game_mut()
            .with_audio(Box::new(o2jam_audio::NullBackend::new()), HashMap::new());
        session
            .game_mut()
            .set_selected_music(o2jam_net::play::SetRoomMusic {
                music_id: 757,
                difficulty: 1,
                speed: 1,
            });
        session
            .game_mut()
            .set_loading_status(o2jam_shell::screens::paint::LoadingStatus {
                progress: 1.0,
                title: "First".to_owned(),
                artist: "Band".to_owned(),
            });

        session.menu_key(o2jam_shell::MenuKey::Confirm, true);
        paint_until_screen(&mut session, Screen::Loading);
        session.menu_key(o2jam_shell::MenuKey::Confirm, true);
        assert_eq!(session.screen(), Screen::Playing, "the loaded gate opened");
        session.game_mut().load_chart(vec![(0, 1000, 7)]);
        session.game_mut().strike_at(0, 1000);
        let width = usize::try_from(NATIVE_WIDTH).unwrap();
        let height = usize::try_from(NATIVE_HEIGHT).unwrap();
        session.game_mut().paint_at(
            &mut o2jam_render::Frame {
                buf: &mut vec![0u32; width * height],
                w: width,
                h: height,
            },
            1_000_000,
        );
        assert_eq!(session.screen(), Screen::Result);
        assert!(
            session
                .game_mut()
                .scene_placements(Screen::Result)
                .iter()
                .any(|placed| placed.code == 0x1750_0004),
            "the staged OPI result pane is live"
        );
        paint(&mut session);
        let _ = observed(&gateway.reports);
        let before = paint_frame(&mut session);

        let (x, y) = staged_code_centre(&mut session, Screen::Result, 0x1720_0005);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(session.screen(), Screen::Result, "BACK waits for 4022");
        paint(&mut session);
        let exit = wait_for_opcode(&gateway.reports, Request::ExitPlaying.id());
        assert!(exit.is_empty(), "the BACK 4021 body stays empty");
        paint_until_screen(&mut session, Screen::Waiting);
        let after = paint_frame(&mut session);
        assert_ne!(before, after, "the answer changed the painted scene");
        finish(session, gateway.server);
    }

    /// Messenger tier-1 leg (ticket #859): the Room board's pinned
    /// MESSENGER press mounts the same one-OK server-unavailable sheet
    /// the exe paints in wave-2 pack `w2-messenger-exe` (frame
    /// `0ac774e9…`, mean delta 11.88 from the Room board; Rust frame
    /// `29649476…` was inert). The click goes through the same pointer
    /// snapshot a window queues; no wire is invented.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (splash Menu edge)
    ///   -> Screen::Planet -> staged emblem click -> Screen::Channel
    ///   -> staged room-row click -> Screen::Room
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_room_row
    ///   -> ScreenGame::click_room_control -> ScreenGame::room_control_at
    ///   -> RoomControlAction::MessengerLiveness -> ScreenGame::post_myroom_notice
    ///   -> ScreenGame::paint_room -> paint_information_message.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_room_messenger_press_mounts_the_one_ok_liveness_sheet_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let before = paint_frame(&mut session);

        let code = MessengerHost::Room.control_code();
        let (x, y) = staged_code_centre(&mut session, Screen::Room, code);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        paint(&mut session);

        assert_eq!(session.screen(), Screen::Room, "the sheet overlays Room");
        assert_eq!(session.game_mut().side(), None, "no side screen opens");
        assert!(
            session.game_mut().room_control_noop().is_none(),
            "the pinned Room MESSENGER press must not be a labeled no-op"
        );
        assert!(
            session.game_mut().messenger_liveness_open(),
            "the Room press must mount the one-OK liveness sheet"
        );
        assert_eq!(
            session.game_mut().myroom_notice(),
            Some(&MyRoomNotice::MessengerUnavailable),
            "the sheet carries the pinned server-unavailable copy"
        );
        let after = paint_frame(&mut session);
        let changed_pixels = before
            .iter()
            .zip(&after)
            .filter(|(before, after)| before != after)
            .count();
        assert!(
            changed_pixels > 0,
            "the mounted sheet must change the Room board pixels"
        );
        assert!(
            observed(&gateway.reports).is_empty(),
            "a liveness sheet invents no Messenger wire: {:?}",
            observed(&gateway.reports)
        );

        let (ok_x, ok_y) = information_ok_centre();
        session.ingest_snapshot(&InputSnapshot::click_at(ok_x, ok_y));
        paint(&mut session);
        assert!(
            !session.game_mut().messenger_liveness_open(),
            "OK clears the mounted sheet"
        );
        assert_eq!(session.screen(), Screen::Room, "OK keeps the Room board");
        finish(session, gateway.server);
    }

    /// Family 1 — scene BACK/exits (#734 class). The complete leg:
    /// boot -> splash -> emblem -> channel row -> room row -> Waiting,
    /// then the staged BACK click posts the pinned leave.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> stage_channel_list_after_relay_gate -> assemble_online_session
    ///   -> GameLoop::ingest_snapshot -> OnlineSession::dispatch_pointer_edge(RoomRow)
    ///   -> poll_room_join (3002/3003) -> echo_entry(Press) -> Waiting
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_waiting_button
    ///   -> ScreenGame::queue_waiting_exit -> OnlineSession::poll_wire
    ///   -> flush_waiting_exit -> send_waiting_leave -> ExitWaiting 3005.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_waiting_back_sends_the_room_leave_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let before = observed(&gateway.reports);
        assert_eq!(
            before.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![Request::JoinWaiting.id()],
            "the join click posted 3002 and nothing else"
        );

        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
        assert_eq!(
            session.screen(),
            Screen::Room,
            "BACK left Waiting on the click"
        );
        paint(&mut session);

        let leave_body = wait_for_opcode(&gateway.reports, Request::ExitWaiting.id());
        assert_eq!(
            leave_body,
            vec![0, 0],
            "the clicked BACK sent the pinned 3005 Normal-exit body"
        );
        let extra_leaves = observed(&gateway.reports)
            .iter()
            .filter(|(id, _)| *id == Request::ExitWaiting.id())
            .count();
        assert_eq!(extra_leaves, 0, "one BACK click posts one leave");
        finish(session, gateway.server);
    }

    /// Rejoin after an exit. A resident-room heuristic must not treat
    /// the row as still joined once 3005 has cleared the seat: the
    /// second identical click is a new join and must post 3002 again.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (first room-row click)
    ///   -> OnlineSession::dispatch_pointer_edge(RoomRow) -> begin_room_join
    ///   -> poll_room_join (3002/3003) -> echo_entry(Press) -> Waiting
    ///   -> GameLoop::ingest_snapshot (BACK click) -> ScreenGame::queue_waiting_exit
    ///   -> OnlineSession::flush_waiting_exit -> send_waiting_leave (3005)
    ///   -> GameLoop::ingest_snapshot (second room-row click)
    ///   -> dispatch_pointer_edge(RoomRow) -> begin_room_join -> 3002.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_rejoin_after_back_sends_a_second_join_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let first = JoinRoom::decode_body(&wait_for_opcode(
            &gateway.reports,
            Request::JoinWaiting.id(),
        ))
        .expect("first join body decodes");
        assert_eq!(first.number, 3, "the first click joined the listed row");

        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
        assert_eq!(session.screen(), Screen::Room, "BACK left Waiting");
        paint(&mut session);
        wait_for_opcode(&gateway.reports, Request::ExitWaiting.id());

        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let second = JoinRoom::decode_body(&wait_for_opcode(
            &gateway.reports,
            Request::JoinWaiting.id(),
        ))
        .expect("rejoin body decodes");
        assert_eq!(
            second.number, first.number,
            "the rejoin joined the same row"
        );
        assert_eq!(
            second.password, first.password,
            "the rejoin kept the row password"
        );
        assert_eq!(
            session.screen(),
            Screen::Waiting,
            "the rejoin entered Waiting"
        );
        finish(session, gateway.server);
    }

    /// Family 2 — dialog OK/dismiss. The relay-down sheet owns the
    /// board until its staged OK plate is clicked; the same pointer
    /// path then reaches the channel row it was holding.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> stage_channel_list_after_relay_gate (holds the 1003 rows)
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (emblem click)
    ///   -> ScreenGame::click_planet -> enter_screen(Channel)
    ///   -> ScreenGame::open_relay_down_dialog -> GameLoop::ingest_snapshot (OK click)
    ///   -> ScreenGame::click_dispatch -> click_planet_modal
    ///   -> ScreenGame::dismiss_relay_down_dialog -> set_channels (rows released).
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_dialog_ok_releases_the_held_channel_rows() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, true);
        dismiss_splash(&mut session);

        session.ingest_snapshot(&InputSnapshot::click_at(136, 88));
        assert_eq!(session.screen(), Screen::Channel);
        assert_eq!(
            session.game_mut().planet_modal(),
            Some(o2jam_shell::screens::game::PlanetModal::RelayDown)
        );
        assert!(
            session.game_mut().channel_rows().is_empty(),
            "the sheet holds the rows"
        );

        let ok = information_ok_centre();
        session.ingest_snapshot(&InputSnapshot::click_at(ok.0, ok.1));
        assert_eq!(
            session.game_mut().planet_modal(),
            None,
            "the clicked OK dismissed the sheet"
        );
        assert_eq!(
            session.game_mut().channel_rows(),
            [channel_row()],
            "OK released the wire rows"
        );

        session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        paint_until_screen(&mut session, Screen::Room);
        finish(session, gateway.server);
    }

    /// Family 3 — Waiting lifecycle. The join is itself a room-row
    /// click; READY/START and BACK then ride the staged button rects.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (room-row click)
    ///   -> OnlineSession::dispatch_pointer_edge(RoomRow) -> set_pending_room_join
    ///   -> begin_room_join -> poll_room_join (3002/3003)
    ///   -> echo_entry(Press) -> Waiting -> GameLoop::ingest_snapshot (READY/START click)
    ///   -> OnlineSession::dispatch_pointer_edge(WaitingConfirm) -> confirm_waiting
    ///   -> Ready 4008 / StartGame 4010 -> GameLoop::ingest_snapshot (BACK click)
    ///   -> ScreenGame::click_waiting_button -> queue_waiting_exit -> poll_wire
    ///   -> flush_waiting_exit -> ExitWaiting 3005.
    /// Archives absent -> skip cleanly (CI has no game data).
    fn waiting_lifecycle_by_clicks(
        member_id: u8,
        peer_ready: bool,
        confirm_id: &str,
        expect_id: u16,
    ) {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let joined = joined_room(member_id, peer_ready);
        let gateway = spawn_live_leg_gateway(joined);
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        join_waiting_by_click(&mut session, &gateway.join_answer_written);
        let before = observed(&gateway.reports);
        assert_eq!(
            before.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![Request::JoinWaiting.id()],
            "the lifecycle leg joined through the room-row click"
        );

        click_staged(&mut session, Screen::Waiting, confirm_id);
        paint(&mut session);
        wait_for_opcode(&gateway.reports, expect_id);
        assert_eq!(
            session.screen(),
            Screen::Waiting,
            "confirm stays in Waiting"
        );

        click_staged(&mut session, Screen::Waiting, "IDC_BUTTON_SWAIT_BACK");
        paint(&mut session);
        wait_for_opcode(&gateway.reports, Request::ExitWaiting.id());
        let extra_leaves = observed(&gateway.reports)
            .iter()
            .filter(|(id, _)| *id == Request::ExitWaiting.id())
            .count();
        assert_eq!(extra_leaves, 0, "one BACK click posts one leave");
        assert_eq!(session.screen(), Screen::Room, "BACK returns to Room");
        finish(session, gateway.server);
    }

    #[test]
    fn live_click_guest_ready_then_back_from_boot() {
        waiting_lifecycle_by_clicks(1, true, "IDC_BUTTON_SWAIT_READY", Request::Ready.id());
    }

    #[test]
    fn live_click_master_start_then_back_from_boot() {
        waiting_lifecycle_by_clicks(0, true, "IDC_BUTTON_SWAIT_START", Request::StartGame.id());
    }

    /// One staged top tab and the exe aim that lands inside its
    /// `State_Room.bnd` rect. The rect ordinal is `(code & 0xFFF) - 1`
    /// (owner rule `0c6c3`); the guard checks the staged rect, never a
    /// literal painted geometry.
    struct RoomTopTab {
        code: u32,
        id: &'static str,
        aim: (i32, i32),
        side: SideScreen,
        enters_shop: bool,
    }

    /// Every Room top tab sends `EnterShop` 5006 on entry. The older
    /// story 313 packs had no tap, so their silence could not disprove
    /// the wire; wave 2's `w2-tab-exe-firststep` tap pins it (see #855).
    fn room_top_tabs() -> [RoomTopTab; 4] {
        [
            RoomTopTab {
                code: 0x1320_0037,
                id: "IDC_BUTTON_SROOM_MUSICSHOP",
                aim: (350, 16),
                side: SideScreen::MusicShop,
                enters_shop: true,
            },
            RoomTopTab {
                code: 0x1320_0008,
                id: "IDC_BUTTON_SROOM_ITEMSHOP",
                aim: (422, 17),
                side: SideScreen::Shop,
                enters_shop: true,
            },
            RoomTopTab {
                code: 0x1320_0009,
                id: "IDC_BUTTON_SROOM_MYROOM",
                aim: (517, 16),
                side: SideScreen::MyRoom,
                enters_shop: true,
            },
            RoomTopTab {
                code: 0x1320_0032,
                id: "IDC_BUTTON_SROOM_FIRSTSTEP",
                aim: (689, 16),
                side: SideScreen::Tutorial,
                enters_shop: true,
            },
        ]
    }

    /// An entry frame can reach the recorder a beat after the paint;
    /// wait for it, but fail with the tab id instead of hanging when a
    /// dispatch omits the wire.
    fn wait_for_enter_shop(reports: &Receiver<Frame>, tab_id: &str) {
        loop {
            match reports.recv_timeout(Duration::from_secs(3)) {
                Ok(frame) if frame.id == EnterShop::COMMAND => {
                    assert!(
                        frame.payload.is_empty(),
                        "{tab_id} must queue an empty C2S 5006"
                    );
                    return;
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {
                    panic!("{tab_id} must queue empty C2S 5006 within 3s")
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("gateway closed before {tab_id} sent 5006")
                }
            }
        }
    }

    /// Check that the exe aim lands inside the staged `.bnd` rect the
    /// pairing rule assigns to this control code.
    fn assert_tab_aim_is_staged(session: &mut OnlineSession, tab: &RoomTopTab) {
        let placement = session
            .game_mut()
            .scene_placements(Screen::Room)
            .iter()
            .find(|placed| placed.code == tab.code)
            .unwrap_or_else(|| panic!("no staged placement for {:#x}", tab.code));
        assert_eq!(placement.id, tab.id, "code pairs with its bnd control id");
        let rect = placement.rect;
        let right = rect.x + i32::try_from(rect.w).expect("rect width fits i32");
        let bottom = rect.y + i32::try_from(rect.h).expect("rect height fits i32");
        assert!(
            tab.aim.0 >= rect.x && tab.aim.0 < right && tab.aim.1 >= rect.y && tab.aim.1 < bottom,
            "exe aim {:?} misses staged rect {rect:?} of {}",
            tab.aim,
            tab.id
        );
    }

    /// Journey 5/7 side-entry leg (#853). Every aim is pressed from a
    /// freshly returned Room board, so one open side screen cannot
    /// mask the next tab. The shop-family entries must also queue the
    /// empty C2S 5006 that the exe wire tap logs on Music Shop and
    /// MyRoom entry (`~/scratch/journey-matrix/musicshop/exe/wire/
    /// room-stage.log`); Rust sent none before this fix.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (splash Menu edge)
    ///   -> staged emblem click -> Screen::Channel -> staged row click -> Screen::Room
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_room_row
    ///   -> ScreenGame::click_room_control -> ScreenGame::room_control_at
    ///   -> RoomControlAction::EnterShopSide -> ScreenGame::visit_side
    ///   -> PendingOutbox::pending_enter_shop -> OnlineSession::poll_wire
    ///   -> flush_enter_shop -> C2S 5006.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_room_top_tabs_pair_by_bnd_ordinal_and_queue_enter_shop() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        for tab in room_top_tabs() {
            // One fresh boot per aim: the side-sweep rig pressed all
            // four aims in one live session without leaving the first
            // side, which made the later three look inert. A fresh
            // board also keeps each tap inside the gateway's quiet
            // window (its read timeout is five seconds).
            let gateway = spawn_live_leg_gateway(joined_room(0, true));
            let mut session = boot_live_leg(gateway.port, false);
            dismiss_splash(&mut session);
            enter_room_by_clicks(&mut session);

            assert_tab_aim_is_staged(&mut session, &tab);
            session.ingest_snapshot(&InputSnapshot::click_at(tab.aim.0, tab.aim.1));
            paint(&mut session);
            assert_eq!(
                session.game_mut().side(),
                Some(tab.side),
                "{} must open its staged side screen",
                tab.id
            );
            if tab.enters_shop {
                wait_for_enter_shop(&gateway.reports, tab.id);
            }
            finish(session, gateway.server);
        }
    }

    /// Wave-4 boots pass `--join-room`, so the session carries a pending
    /// join intent into the Room board. A top-tab press is a local press
    /// (unlike the row press that joined), and must still reach the staged
    /// ITEMSHOP routing while that intent is unconfirmed. Regression for
    /// #864.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> assemble_online_session
    ///   -> set_pending_room_join -> window::deliver_snapshot
    ///   -> OnlineSession::ingest_snapshot -> dispatch_pointer_edge
    ///   -> dispatch_pointer_down -> dispatch_local_press
    ///   -> ScreenGame::click_dispatch -> click_room_row -> click_room_control
    ///   -> enter_room_side_with_enter_shop -> PendingOutbox::pending_enter_shop
    ///   -> C2S 5006.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_room_itemshop_routes_with_a_pending_join_intent() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        // The wave-4 driver boots with `--join-room 0`.
        session.set_pending_room_join(0, "");
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        assert_eq!(
            session.screen(),
            Screen::Room,
            "channel press reaches the Room board"
        );

        let tabs = room_top_tabs();
        let itemshop = &tabs[1];
        assert_tab_aim_is_staged(&mut session, itemshop);
        session.ingest_snapshot(&InputSnapshot::click_at(itemshop.aim.0, itemshop.aim.1));
        paint(&mut session);
        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::Shop),
            "an unconfirmed pending join must not swallow the ITEMSHOP press"
        );
        wait_for_enter_shop(&gateway.reports, itemshop.id);
        finish(session, gateway.server);
    }

    /// Painted frame from the same live path the window drives.
    fn painted_frame(session: &mut OnlineSession) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits a buffer index");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits a buffer index");
        let mut buf = vec![0u32; width * height];
        session.paint(&mut o2jam_render::Frame {
            buf: &mut buf,
            w: width,
            h: height,
        });
        buf
    }

    /// Board samples from the staged `STATE_SHOP` art. A full frame
    /// from a bare game with only that art in its side slots keeps the
    /// check on the mounted block, never on a literal colour.
    fn staged_shop_samples() -> Vec<u32> {
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive present behind the guard");
        let art = stage_state(&archive, State::Shop, StageContext::default())
            .art
            .expect("STATE_SHOP stages with installed bytes");
        let mut game = ScreenGame::new();
        // The baseline stages the same catalog the live boot stages:
        // the #861 new-arrivals goods change the card pixels, and the
        // guard must keep pinning the mounted scene, not a stale
        // face-catalog frame.
        if let Some(source) = o2jam_shell::read_installed_avatar(&install_root()) {
            game.set_decoded_avatar_source(&source);
            game.stage_exe_item_shop_catalog(source.table());
        }
        game.load_side_scene(art.placements, art.figures);
        for lobby in [Screen::Planet, Screen::Channel, Screen::Room] {
            assert!(game.enter_screen(lobby), "staged walk reaches {lobby:?}");
        }
        assert!(game.visit_side(SideScreen::Shop));
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits a buffer index");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits a buffer index");
        let mut buf = vec![0u32; width * height];
        game.paint(&mut o2jam_render::Frame {
            buf: &mut buf,
            w: width,
            h: height,
        });
        SHOP_SAMPLE_POINTS
            .iter()
            .map(|&(x, y)| buf[y * width + x])
            .collect()
    }

    /// Staged board points away from captions and the avatar pane.
    const SHOP_SAMPLE_POINTS: [(usize, usize); 6] = [
        (400, 100),
        (100, 100),
        (300, 450),
        (600, 450),
        (50, 400),
        (700, 80),
    ];

    fn assert_mounts_item_shop(session: &mut OnlineSession, samples: &[u32], entry: &str) {
        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::Shop),
            "{entry} must mount the Item Shop side state"
        );
        let frame = painted_frame(session);
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits a buffer index");
        for (index, &(x, y)) in SHOP_SAMPLE_POINTS.iter().enumerate() {
            assert_eq!(
                frame[y * width + x],
                samples[index],
                "{entry} paints the staged STATE_SHOP board at ({x},{y})"
            );
        }
    }

    /// Ticket #856 (wave-2 leg 2). The exe's Room ITEMSHOP handler
    /// (`FUN_0048fd20` case `0x13200008`, file `0x8fd20`) and its
    /// MyRoom GOSHOP handler (`FUN_004a3ee0` case `0x15200003`, file
    /// `0xa3ee0`) both call `ChangeState(3)` before the 5006 send in
    /// the Room case: one runtime state pick, one staged Item Shop
    /// block. The guard drives both entries from boot and checks the
    /// same staged board pixels on each.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot
    ///   -> ScreenGame::click_room_control -> visit_side(Shop)
    ///   -> mount_staged_side_art -> paint_side (Room ITEMSHOP at
    ///      the staged `(422,17)` rect), then
    ///   -> ScreenGame::click_myroom_side -> MyRoomAction::OpenShop
    ///   -> leave_side -> visit_side(Shop) -> mount_staged_side_art
    ///      (MyRoom GOSHOP at the staged `0x15200003` rect).
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_itemshop_entry_mounts_the_exe_shop_state_from_room_and_myroom() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let samples = staged_shop_samples();
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);

        let tabs = room_top_tabs();
        let room_tab = &tabs[1];
        assert_tab_aim_is_staged(&mut session, room_tab);
        session.ingest_snapshot(&InputSnapshot::click_at(room_tab.aim.0, room_tab.aim.1));
        assert_mounts_item_shop(&mut session, &samples, "Room ITEMSHOP (422,17)");
        wait_for_enter_shop(&gateway.reports, room_tab.id);

        let mut back = InputSnapshot::new();
        back.push_edge(InputEdge::Menu {
            key: o2jam_shell::MenuKey::Back,
            down: true,
        });
        session.ingest_snapshot(&back);
        assert_eq!(session.game_mut().side(), None, "BACK leaves the shop side");

        session.ingest_snapshot(&InputSnapshot::click_at(517, 16));
        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::MyRoom),
            "Room MYROOM tab opens the avatar room"
        );
        // The aim comes from the staged GOSHOP rect, never a literal.
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive present behind the guard");
        let art = stage_state(&archive, State::MyRoom, StageContext::default())
            .art
            .expect("STATE_MYROOM stages with installed bytes");
        let goshop = art
            .placements
            .iter()
            .find(|placed| placed.code == MYROOM_GOSHOP_CODE)
            .expect("STATE_MYROOM stages the GOSHOP control");
        let aim = (
            goshop.rect.x + i32::try_from(goshop.rect.w).expect("rect width fits i32") / 2,
            goshop.rect.y + i32::try_from(goshop.rect.h).expect("rect height fits i32") / 2,
        );
        session.ingest_snapshot(&InputSnapshot::click_at(aim.0, aim.1));
        assert_mounts_item_shop(&mut session, &samples, "MyRoom GOSHOP");

        finish(session, gateway.server);
    }

    /// Press one staged mode tab by its parsed `.bnd` control id, from
    /// a real boot and through pointer snapshots only. The tab centre
    /// is the painter's rect, never a literal coordinate.
    fn click_staged_room_filter(session: &mut OnlineSession, id: &str) {
        click_staged(session, Screen::Room, id);
    }

    /// Paint one frame and keep the pixels inside the staged [W]/[A]
    /// rects: the area where the exe moves the active-tab frame. The
    /// rects come from the parsed `.bnd` placements, never a literal.
    fn paint_wa_region(session: &mut OnlineSession) -> Vec<u32> {
        let rects: Vec<Rect> = [
            "IDC_BUTTON_SROOM_SHOW_ALLMODE",
            "IDC_BUTTON_SROOM_SHOW_NORMALMODE",
        ]
        .iter()
        .map(|id| {
            session
                .game_mut()
                .scene_placements(Screen::Room)
                .iter()
                .find(|placed| placed.id == *id)
                .unwrap_or_else(|| panic!("no staged placement {id}"))
                .rect
        })
        .collect();
        let frame = paint_frame(session);
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let mut pixels = Vec::new();
        for rect in &rects {
            let top = usize::try_from(rect.y).expect("rect top fits usize");
            let left = usize::try_from(rect.x).expect("rect left fits usize");
            let rows = usize::try_from(rect.h).expect("rect height fits usize");
            let columns = usize::try_from(rect.w).expect("rect width fits usize");
            for y in top..top + rows {
                for x in left..left + columns {
                    pixels.push(
                        *frame
                            .get(y * width + x)
                            .unwrap_or_else(|| panic!("tab pixel ({x},{y}) is in frame")),
                    );
                }
            }
        }
        pixels
    }

    /// Number of changed native pixels in one staged tab region.
    fn changed_pixels(before: &[u32], after: &[u32]) -> usize {
        before
            .iter()
            .zip(after)
            .filter(|(before, after)| before != after)
            .count()
    }

    /// #290 T1: the Room [W]/[A] chrome must flip the local room-family
    /// selection and the visible active-tab frame in both directions.
    /// Exe pin: `FUN_0048fb40` at `0x48fb40` writes mode 0/1/2 at state
    /// `+0x60`, marks the pressed tab through `FUN_0048c810`, resets
    /// `+0x70` to zero, and calls the row repainter `FUN_00492300`; it
    /// sends no wire frame (`ev_ff527707048e48accf48b6f291ad97a0abc6324dfb7b5a8f0df25f3318518c71`,
    /// `ev_e9f783fce3671c057d37820439860dbf27036d606c2c75100eee35d4c61e0b23`,
    /// `ev_e56494975137c66dc51178cefd71cc920ca8b8b38cc2fe7ee179fb6a5c19a76d`).
    /// `docs/hover.md` pins the active tab at body frame 1.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (splash Menu edge)
    ///   -> staged emblem click -> Screen::Channel -> staged row click -> Screen::Room
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_dispatch
    ///   -> ScreenGame::click_room_row -> ScreenGame::click_room_control
    ///   -> ScreenGame::room_control_at -> RoomControlAction::RoomFilter
    ///   -> ScreenGame::refresh_visible_rooms -> ScreenGame::paint_room
    ///   -> paint_room_scene_with_selection (active tab frame 1).
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_room_wa_tabs_toggle_the_selection_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        assert_eq!(session.game_mut().room_filter(), RoomListFilter::All);
        // Room entry answers 3002; drain it so the tab taps are measured
        // against a quiet outbox (the exe arm sends no 2002).
        while gateway.reports.try_recv().is_ok() {}
        let all_pixels = paint_wa_region(&mut session);

        click_staged_room_filter(&mut session, "IDC_BUTTON_SROOM_SHOW_NORMALMODE");
        assert_eq!(
            session.game_mut().room_filter(),
            RoomListFilter::Normal,
            "[A] must select the Normal family"
        );
        let normal_pixels = paint_wa_region(&mut session);
        let moved = changed_pixels(&all_pixels, &normal_pixels);
        assert!(
            moved > 0,
            "[A] must move the active frame off [W] onto [A]; moved {moved} px"
        );

        click_staged_room_filter(&mut session, "IDC_BUTTON_SROOM_SHOW_ALLMODE");
        assert_eq!(
            session.game_mut().room_filter(),
            RoomListFilter::All,
            "[W] must select all families"
        );
        let back_to_all = paint_wa_region(&mut session);
        assert_eq!(
            changed_pixels(&all_pixels, &back_to_all),
            0,
            "[W] restores the initial frame"
        );

        click_staged_room_filter(&mut session, "IDC_BUTTON_SROOM_SHOW_NORMALMODE");
        assert_eq!(
            session.game_mut().room_filter(),
            RoomListFilter::Normal,
            "[A] must toggle back after [W]"
        );
        let normal_again = paint_wa_region(&mut session);
        assert_eq!(
            changed_pixels(&normal_pixels, &normal_again),
            0,
            "[A] toggles deterministically"
        );
        assert!(
            matches!(
                gateway.reports.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Empty)
            ),
            "the local toggle must send no wire frame"
        );

        finish(session, gateway.server);
    }

    /// Press one staged shop control by its ControlList code. The aim
    /// comes from the mounted `.bnd` rect; the caller supplies the
    /// exe child-origin offset only for art-less hit rects.
    fn click_staged_shop_control(session: &mut OnlineSession, code: u32, hit_y_offset: i32) {
        let rect = session
            .game_mut()
            .staged_side_control_rect(code)
            .unwrap_or_else(|| panic!("STATE_SHOP does not stage {code:#X}"));
        let aim = rect_centre(Rect {
            y: rect.y.saturating_add(hit_y_offset),
            ..rect
        });
        session.ingest_snapshot(&InputSnapshot::click_at(aim.0, aim.1));
    }

    /// Pixels inside the staged category tabs and the eight ITEMNAME
    /// cells: the exe scroll and category arms repaint both faces.
    fn shop_browse_pixels(session: &mut OnlineSession) -> Vec<u32> {
        let codes: Vec<u32> = (0..5)
            .map(|slot| SHOP_CATEGORY_FIRST_CODE + slot)
            .chain((0..8).map(|slot| 0x1440_002C + slot))
            .collect();
        let rects: Vec<Rect> = codes
            .iter()
            .map(|code| {
                session
                    .game_mut()
                    .staged_side_control_rect(*code)
                    .unwrap_or_else(|| panic!("STATE_SHOP does not stage {code:#X}"))
            })
            .collect();
        let frame = painted_frame(session);
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits a frame index");
        let mut pixels = Vec::new();
        for rect in rects {
            let left = usize::try_from(rect.x).expect("rect left fits a frame index");
            let top = usize::try_from(rect.y).expect("rect top fits a frame index");
            let columns = usize::try_from(rect.w).expect("rect width fits a frame index");
            let rows = usize::try_from(rect.h).expect("rect height fits a frame index");
            for y in top..top + rows {
                for x in left..left + columns {
                    pixels.push(frame[y * width + x]);
                }
            }
        }
        pixels
    }

    /// Boot -> Room -> Item Shop -> staged browse presses. The exe
    /// pins are `IDC_SCROLL_SSHOP_ITEM` notify `0x14022047`
    /// (`ev_5eb3c20e…`, `ev_7509bf98…`), category/submenu/theme tabs in
    /// `FUN_00498bc0` (`ev_1240440e…`, `ev_d7cb6a21…`), and the
    /// ITEMBUY row arm (`ev_650b7cf9…`). None sends a 5000-family
    /// frame; the old inert guard certified the opposite.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online ->
    ///   connect::connect_and_login_with_transport ->
    ///   assemble_online_session -> GameLoop::ingest_snapshot ->
    ///   ScreenGame::click_room_control -> visit_side(Shop) ->
    ///   GameLoop::ingest_snapshot -> ScreenGame::click_side_screen ->
    ///   ScreenGame::click_shop -> shop_action_at ->
    ///   ShopState::apply_action -> ScreenGame::paint_side ->
    ///   paint_shop_board / paint_side_scene_selected.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_shop_browse_scroll_tabs_and_grid_change_goods_from_boot() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let itemshop = &room_top_tabs()[1];
        assert_tab_aim_is_staged(&mut session, itemshop);
        session.ingest_snapshot(&InputSnapshot::click_at(itemshop.aim.0, itemshop.aim.1));
        assert_eq!(session.game_mut().side(), Some(SideScreen::Shop));
        drop(painted_frame(&mut session));
        wait_for_enter_shop(&gateway.reports, itemshop.id);
        while gateway.reports.try_recv().is_ok() {}

        let entry_goods = session.game_mut().shop_visible_goods();
        let entry_pixels = shop_browse_pixels(&mut session);

        click_staged_shop_control(&mut session, SHOP_SCROLL_RIGHT_CODE, 0);
        assert_eq!(
            session.game_mut().shop_page(),
            1,
            "scroll RIGHT turns the grid"
        );
        assert_ne!(
            session.game_mut().shop_visible_goods(),
            entry_goods,
            "the eight visible goods follow the scroll offset"
        );
        assert!(
            changed_pixels(&entry_pixels, &shop_browse_pixels(&mut session)) > 0,
            "scroll must repaint the name cells"
        );

        click_staged_shop_control(&mut session, SHOP_CATEGORY_FIRST_CODE + 1, 0);
        assert_eq!(
            session.game_mut().shop_category(),
            Some(1),
            "FASHION answers"
        );
        assert_eq!(
            session.game_mut().shop_page(),
            0,
            "category rebuilds page one"
        );
        assert!(
            changed_pixels(&entry_pixels, &shop_browse_pixels(&mut session)) > 0,
            "category must move the tab highlight"
        );

        click_staged_shop_control(&mut session, SHOP_SUBMENU_FIRST_CODE + 2, 0);
        assert_eq!(
            session.game_mut().shop_selected_submenu(),
            Some(SHOP_SUBMENU_FIRST_CODE + 2),
            "SUBMENU3 writes state+0x59"
        );

        let first_buy = session.game_mut().shop_visible_goods()[0].item_id;
        click_staged_shop_control(&mut session, SHOP_ITEMBUY_FIRST_CODE, SHOP_CHILD_ORIGIN_Y);
        assert_eq!(
            session.game_mut().shop_cart(),
            vec![first_buy],
            "ITEMBUY1 checks the visible row into the cart"
        );

        session.game_mut().stage_shop_extmenu_ready(true);
        click_staged_shop_control(&mut session, SHOP_EXTMENU_ON_CODE, 0);
        click_staged_shop_control(&mut session, SHOP_EXTMENU_FIRST_CODE + 2, 0);
        assert_eq!(
            session.game_mut().shop_theme(),
            Some(2),
            "the ext-menu theme tab writes state+0x5C"
        );

        // One more live paint flushes the outbox the same way the
        // window does; silence then means the arms queued nothing.
        drop(painted_frame(&mut session));
        assert_eq!(session.game_mut().take_exit_shop(), None);
        assert_eq!(session.game_mut().take_shop_buy(), None);
        assert!(
            matches!(
                gateway.reports.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Empty)
            ),
            "browse chrome must send no 5000-family frame"
        );
        finish(session, gateway.server);
    }

    /// Boot -> Room -> Item Shop -> staged GO MYROOM press. The exe
    /// case `0x14200003` requests state 4, which installs CStateMyroom
    /// (`ev_541e0dc4…`, `ev_bf900e2a…`, `ev_fcb252ad…`); its neighbor
    /// BACK alone sends 5007 (`ev_58236c4d…`).
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online ->
    ///   connect::connect_and_login_with_transport ->
    ///   assemble_online_session -> GameLoop::ingest_snapshot ->
    ///   ScreenGame::click_room_control -> visit_side(Shop) ->
    ///   GameLoop::ingest_snapshot -> ScreenGame::click_side_screen ->
    ///   ScreenGame::click_shop(GoMyRoom) -> ScreenGame::switch_side ->
    ///   ScreenGame::visit_side(MyRoom) -> mount_staged_side_art.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_shop_go_myroom_enters_myroom_without_an_exit_send() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        let itemshop = &room_top_tabs()[1];
        assert_tab_aim_is_staged(&mut session, itemshop);
        session.ingest_snapshot(&InputSnapshot::click_at(itemshop.aim.0, itemshop.aim.1));
        drop(painted_frame(&mut session));
        wait_for_enter_shop(&gateway.reports, itemshop.id);
        while gateway.reports.try_recv().is_ok() {}

        click_staged_shop_control(&mut session, SHOP_GOMYROOM_CODE, 0);

        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::MyRoom),
            "state 4 installs MyRoom"
        );
        assert!(
            session
                .game_mut()
                .staged_side_control_rect(MYROOM_GOSHOP_CODE)
                .is_some(),
            "the MyRoom panel is staged from State_MyRoom.bnd"
        );
        drop(painted_frame(&mut session));
        assert_eq!(session.game_mut().take_exit_shop(), None);
        assert!(
            matches!(
                gateway.reports.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Empty)
            ),
            "GO MYROOM must not queue a wire frame"
        );
        finish(session, gateway.server);
    }

    /// #314 Tier 3: one raw key walk from a real boot, the staged
    /// Options entry, and a staged rebind cell. The mapped DIK must
    /// write and advance the arm; an unmapped typed key must be eaten
    /// by the dialog with no chat draft, no state change, and no wire.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online
    ///   -> connect::connect_and_login_with_transport -> assemble_online_session
    ///   -> OnlineSession::stage_live_art -> GameLoop::ingest_snapshot
    ///   -> staged Room/Planet Options entry -> ScreenGame::click_options_dialog
    ///   -> OptionsDialog::click -> o2jam_shell::route_key_event
    ///   -> GameLoop::capture_raw_key -> ScreenGame::capture_raw_key
    ///   -> OptionsDialog::rebind_armed_lane.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_options_key_capture_from_room_eats_unmapped_keys_and_rebinds_armed_lane() {
        assert_options_key_capture_from(OptionsEntry::Room);
    }

    /// Planet twin of the Room leg: the exe has one dialog and one
    /// ctor for both entries (#314 static pin), so any difference here
    /// is our focus leak, not an entry-path rule.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online
    ///   -> connect::connect_and_login_with_transport -> assemble_online_session
    ///   -> OnlineSession::stage_live_art -> GameLoop::ingest_snapshot
    ///   -> staged Planet gear -> ScreenGame::click_planet_modal
    ///   -> ScreenGame::click_options_dialog -> OptionsDialog::click
    ///   -> o2jam_shell::route_key_event -> GameLoop::capture_raw_key
    ///   -> ScreenGame::capture_raw_key -> OptionsDialog::rebind_armed_lane.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_options_key_capture_from_planet_eats_unmapped_keys_and_rebinds_armed_lane() {
        assert_options_key_capture_from(OptionsEntry::Planet);
    }

    /// All seven 7K cells answer their mapped DIK through the live
    /// route. The DIKs are from the exe's 37-byte order table.
    fn rebind_all_seven_key_lanes(
        session: &mut OnlineSession,
        keymap: &mut Keymap,
        entry: OptionsEntry,
    ) {
        for (lane, key, text) in [
            (1u32, KeyCode::KeyW, "w"),
            (2, KeyCode::KeyE, "e"),
            (3, KeyCode::KeyR, "r"),
            (4, KeyCode::KeyT, "t"),
            (5, KeyCode::KeyY, "y"),
            (6, KeyCode::KeyU, "u"),
        ] {
            rebind_staged_lane(session, keymap, OPTION_KEY_LANE_FIRST + lane, key, text);
            assert_eq!(
                session.game_mut().options().lane_keys()[usize::try_from(lane).expect("lane fits")],
                key,
                "7K lane {lane} must answer its staged cell from {entry:?}"
            );
        }
    }

    /// The kept 3K cells write their own table; index 1 is hidden by
    /// the exe's skipped staged rect and never armed.
    fn rebind_all_three_key_lanes(session: &mut OnlineSession, keymap: &mut Keymap) {
        let (x, y) = staged_dialog_centre(
            Dialog::PlanetOption,
            o2jam_shell::screens::side::OPTION_KEY_3KEY_MODE,
        );
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert!(session.game_mut().options().three_key(), "3KEY selector");
        for (lane, key, text) in [
            (0u32, KeyCode::KeyZ, "z"),
            (2, KeyCode::KeyX, "x"),
            (3, KeyCode::KeyC, "c"),
        ] {
            rebind_staged_lane(
                session,
                keymap,
                o2jam_shell::screens::side::OPTION_KEY_3KEY_FIRST + lane,
                key,
                text,
            );
            assert_eq!(
                session.game_mut().options().three_key_dik()
                    [usize::try_from(lane).expect("lane fits")],
                o2jam_shell::options::dik_of_keycode(key).expect("mapped DIK"),
                "3K lane {lane} must keep its own bind table"
            );
        }
    }

    fn assert_options_key_capture_from(entry: OptionsEntry) {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg_with_art(gateway.port, false);
        dismiss_splash(&mut session);
        open_options_key_sheet(&mut session, entry);

        let (x, y) = staged_dialog_centre(Dialog::PlanetOption, OPTION_KEY_7KEY_MODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert!(
            !session.game_mut().options().three_key(),
            "the staged 7KEY selector must arm the seven-key family"
        );
        let (x, y) = staged_dialog_centre(Dialog::PlanetOption, OPTION_KEY_LANE_FIRST);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        assert_eq!(
            session.game_mut().option_rebind_armed(),
            Some(0),
            "KEY1 must arm lane 0 from {entry:?}"
        );
        let mut keymap = Keymap::new();
        rebind_staged_lane(
            &mut session,
            &mut keymap,
            OPTION_KEY_LANE_FIRST,
            KeyCode::KeyA,
            "a",
        );
        assert_eq!(
            session.game_mut().option_rebind_armed(),
            Some(1),
            "the exe advances the armed lane after a mapped press"
        );
        assert_eq!(
            session.game_mut().options().lane_keys()[0],
            KeyCode::KeyA,
            "the mapped DIK must land in the armed lane's bind"
        );

        rebind_all_seven_key_lanes(&mut session, &mut keymap, entry);
        rebind_all_three_key_lanes(&mut session, &mut keymap);
        let (x, y) = staged_dialog_centre(Dialog::PlanetOption, OPTION_KEY_7KEY_MODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        rebind_staged_lane(
            &mut session,
            &mut keymap,
            OPTION_KEY_LANE_FIRST,
            KeyCode::KeyA,
            "a",
        );

        let (x, y) = staged_dialog_centre(Dialog::PlanetOption, OPTION_KEY_7KEY_MODE);
        session.ingest_snapshot(&InputSnapshot::click_at(x, y));
        rebind_staged_lane(
            &mut session,
            &mut keymap,
            OPTION_KEY_LANE_FIRST,
            KeyCode::KeyA,
            "a",
        );

        // Drain any lobby poll frame before the measured key; the exe
        // pin is that the key itself sends nothing.
        while gateway.reports.try_recv().is_ok() {}
        let before = key_sheet_state(&mut session);
        let unmapped = route_raw_key(&mut session, &mut keymap, KeyCode::Digit1, Some("1"));
        assert!(
            unmapped.captured && unmapped.text.is_none() && unmapped.edges.is_empty(),
            "an unmapped key must be eaten by the dialog, not typed or routed: {unmapped:?}"
        );
        assert_eq!(
            key_sheet_state(&mut session),
            before,
            "the unmapped key must leave no chat row, bind, tab, or state change"
        );
        let chat_frames = observed(&gateway.reports)
            .iter()
            .filter(|(id, payload)| {
                *id == Request::SendMainRoomMessage.id()
                    || *id == Request::SendWhisper.id()
                    || *id == Request::Announce.id()
                    || payload == b"1"
            })
            .count();
        assert_eq!(
            chat_frames, 0,
            "an eaten key must not appear as chat on the wire"
        );
        finish(session, gateway.server);
    }

    /// #314 Tier 3: pointer off/on/off over a rebind cell and a tab
    /// from both entries. The exe has no hover face on these cells
    /// (static pin P3), so hover must repaint the existing ink: the on
    /// frame differs from rest, keeps its glyph ink, and the third
    /// frame restores the first. No blank frame may appear.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online
    ///   -> connect::connect_and_login_with_transport -> assemble_online_session
    ///   -> OnlineSession::stage_live_art -> GameLoop::ingest_snapshot
    ///   -> staged Room/Planet Options entry -> ScreenGame::click_options_dialog
    ///   -> pointer snapshot -> ScreenGame::paint_at -> paint_dialog_frame_states
    ///   -> paint_dialog_frame_record.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn options_key_hover_preserves_rest_ink_with_distinct_off_on_off_frames() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        for entry in [OptionsEntry::Room, OptionsEntry::Planet] {
            let gateway = spawn_live_leg_gateway(joined_room(0, true));
            let mut session = boot_live_leg_with_art(gateway.port, false);
            dismiss_splash(&mut session);
            open_options_key_sheet(&mut session, entry);
            let (x, y) = staged_dialog_centre(Dialog::PlanetOption, OPTION_KEY_7KEY_MODE);
            session.ingest_snapshot(&InputSnapshot::click_at(x, y));

            // Rebind cell: the bound DIK caption must survive the hover
            // layer. The 7K family's first lane uses the ordinary
            // caption origin, so its rest glyph matches the staged
            // atlas exactly (no labeled stand-in in this assertion).
            let cell = staged_dialog_rect(Dialog::PlanetOption, OPTION_KEY_LANE_FIRST);
            let centre = rect_centre(cell);
            let dik = session.game_mut().options().lane_dik_for_mode(false)[0];
            let caption = bound_caption_pixels(cell, dik);
            session.ingest_snapshot(&InputSnapshot::motion_to(10, 10));
            let cell_off = paint_frame(&mut session);
            session.ingest_snapshot(&InputSnapshot::motion_to(centre.0, centre.1));
            let cell_on = paint_frame(&mut session);
            session.ingest_snapshot(&InputSnapshot::motion_to(10, 10));
            let cell_off_again = paint_frame(&mut session);
            let rest_mismatch = caption_mismatches(&cell_off, &caption);
            assert_eq!(
                rest_mismatch, 0,
                "the rest frame must paint the bound caption"
            );
            assert_eq!(
                caption_mismatches(&cell_on, &caption),
                rest_mismatch,
                "hover must keep the bound key face, not blank or replace its glyph"
            );
            assert_eq!(
                caption_mismatches(&cell_off_again, &caption),
                rest_mismatch,
                "unhover restores the same bound glyph"
            );
            let cell_off_ink = ink_pixels(&staged_rect_pixels_from(&cell_off, cell));
            let cell_on_ink = ink_pixels(&staged_rect_pixels_from(&cell_on, cell));
            assert!(
                cell_off_ink > 0 && cell_on_ink > 0,
                "both frames keep key-area ink"
            );
            assert_ne!(cell_off, cell_on, "the off/on/off frames must be distinct");

            // Tab: the generic hover face may change the body, but it
            // must never reduce the staged selector to a blank plate.
            let tab = staged_dialog_rect(Dialog::PlanetOption, OPTION_KEY_7KEY_MODE);
            let centre = rect_centre(tab);
            session.ingest_snapshot(&InputSnapshot::motion_to(10, 10));
            let tab_off = staged_rect_pixels(&mut session, tab);
            session.ingest_snapshot(&InputSnapshot::motion_to(centre.0, centre.1));
            let tab_on = staged_rect_pixels(&mut session, tab);
            session.ingest_snapshot(&InputSnapshot::motion_to(10, 10));
            let tab_off_again = staged_rect_pixels(&mut session, tab);
            assert!(
                changed_pixels(&tab_off, &tab_on) > 0,
                "tab hover has a face"
            );
            assert!(ink_pixels(&tab_on) > 0, "tab hover keeps its ink");
            assert_eq!(
                changed_pixels(&tab_off, &tab_off_again),
                0,
                "unhover restores the tab rest face"
            );
            finish(session, gateway.server);
        }
    }
    /// #314 Tier 3: every staged Key-tab click keeps the exe dispatch
    /// from both entries. The pins (P4, click arm `0x4383d0`) give the
    /// mode selectors, rebind cells, key-test gate, SAVE/EXIT, and the
    /// Sound/Game footer tabs to the dialog; no per-caller rule exists.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online
    ///   -> connect::connect_and_login_with_transport -> assemble_online_session
    ///   -> OnlineSession::stage_live_art -> GameLoop::ingest_snapshot
    ///   -> staged Room/Planet Options entry -> ScreenGame::click_dispatch
    ///   -> ScreenGame::click_planet_modal -> ScreenGame::click_options_dialog
    ///   -> OptionsDialog::click -> OptionsDialog::click_key_tab / switch_tab.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn options_key_tab_and_cell_clicks_keep_exe_dispatch_from_both_entries() {
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        // Staged Key-tab footer codes from exe click arm 0x4383d0.
        const KEY_SAVE: u32 = 0x2520_0016;
        const KEY_EXIT: u32 = 0x2520_0017;
        for entry in [OptionsEntry::Room, OptionsEntry::Planet] {
            let gateway = spawn_live_leg_gateway(joined_room(0, true));
            let mut session = boot_live_leg_with_art(gateway.port, false);
            dismiss_splash(&mut session);
            open_options_key_sheet(&mut session, entry);
            while gateway.reports.try_recv().is_ok() {}

            let click_key = |session: &mut OnlineSession, code: u32| {
                let (x, y) = staged_dialog_centre(Dialog::PlanetOption, code);
                session.ingest_snapshot(&InputSnapshot::click_at(x, y));
            };
            let click_tab = |session: &mut OnlineSession, dialog: Dialog, code: u32| {
                let (x, y) = staged_dialog_centre(dialog, code);
                session.ingest_snapshot(&InputSnapshot::click_at(x, y));
            };

            click_key(&mut session, OPTION_KEY_7KEY_MODE);
            assert!(
                !session.game_mut().options().three_key(),
                "7KEY from {entry:?}"
            );
            click_key(&mut session, OPTION_KEY_LANE_FIRST);
            assert_eq!(session.game_mut().option_rebind_armed(), Some(0));
            click_key(&mut session, OPTION_KEY_3KEY_MODE);
            assert!(
                session.game_mut().options().three_key(),
                "3KEY from {entry:?}"
            );
            click_key(
                &mut session,
                o2jam_shell::screens::side::OPTION_KEY_3KEY_FIRST,
            );
            assert_eq!(session.game_mut().option_rebind_armed(), Some(0));
            click_key(&mut session, OPTION_KEY_LANE_FIRST);
            assert_eq!(
                session.game_mut().option_rebind_armed(),
                Some(0),
                "hidden 7K cells leave the armed 3K lane alone"
            );

            click_key(&mut session, OPTION_KEY_TEST);
            assert!(
                session.game_mut().options().key_test(),
                "KEYTEST toggles on"
            );
            click_key(&mut session, OPTION_KEY_3KEY_FIRST + 2);
            assert_eq!(
                session.game_mut().option_rebind_armed(),
                Some(0),
                "key-test refuses rebind cells and keeps the prior arm"
            );
            click_tab(&mut session, Dialog::PlanetOption, OPTION_KEY_SOUNDSET);
            assert_eq!(
                session.game_mut().planet_option_tab(),
                PlanetOptionTab::Key,
                "key-test refuses sheet tabs from {entry:?}"
            );
            click_key(&mut session, OPTION_KEY_TEST);
            assert!(
                !session.game_mut().options().key_test(),
                "KEYTEST toggles off"
            );

            click_tab(&mut session, Dialog::PlanetOption, OPTION_KEY_SOUNDSET);
            assert_eq!(
                session.game_mut().planet_option_tab(),
                PlanetOptionTab::Sound
            );
            click_tab(
                &mut session,
                Dialog::PlanetOptionSound,
                OPTION_SOUND_GAMESET,
            );
            assert_eq!(
                session.game_mut().planet_option_tab(),
                PlanetOptionTab::Game
            );
            click_tab(&mut session, Dialog::PlanetOptionGame, OPTION_GAME_KEYSET);
            assert_eq!(session.game_mut().planet_option_tab(), PlanetOptionTab::Key);

            click_key(&mut session, KEY_SAVE);
            assert_eq!(
                session.game_mut().planet_modal(),
                Some(PlanetModal::Options),
                "SAVE commits in the open dialog"
            );
            click_key(&mut session, KEY_EXIT);
            assert_eq!(
                session.game_mut().planet_modal(),
                None,
                "EXIT closes the dialog from {entry:?}"
            );
            let chat_frames = observed(&gateway.reports)
                .iter()
                .filter(|(id, _)| {
                    *id == Request::SendMainRoomMessage.id()
                        || *id == Request::SendWhisper.id()
                        || *id == Request::Announce.id()
                })
                .count();
            assert_eq!(chat_frames, 0, "options clicks must not send chat");
            finish(session, gateway.server);
        }
    }

    /// Exe control codes for the two #180 legs. These are the pinned
    /// dispatcher cases, kept literal here so the guard fails if a
    /// production constant is renamed without a behavior change.
    const MYROOM_STATUS_BADGE_CODE: u32 = 0x1520_3034;
    const MYROOM_MY_BAG_TAB_CODE: u32 = 0x1560_0045;
    const MYROOM_PRESENT_BOX_TAB_CODE: u32 = 0x1560_0046;

    /// Pinned exe geometry from `docs/myroom.md` (#369): the baked
    /// STATUS badge and the STATUS plate it swaps in.
    const STATUS_BADGE_RECT: Rect = Rect {
        x: 147,
        y: 307,
        w: 60,
        h: 23,
    };
    const STATUS_PLATE_RECT: Rect = Rect {
        x: 392,
        y: 22,
        w: 384,
        h: 357,
    };

    /// Click the Room MYROOM tab through the same pointer snapshot a
    /// window queues; the aim comes from the staged `.bnd` control.
    fn enter_myroom_by_click(session: &mut OnlineSession) {
        click_staged(session, Screen::Room, "IDC_BUTTON_SROOM_MYROOM");
        // The windowed frame entry flushes the entry's 5006.
        paint(session);
        assert_eq!(
            session.game_mut().side(),
            Some(SideScreen::MyRoom),
            "the staged MYROOM tab must mount the avatar room"
        );
    }

    /// Click a staged side control by its dispatcher code. The centre
    /// comes from the mounted `.bnd` rect, never from a test literal.
    fn click_staged_side_code(session: &mut OnlineSession, code: u32) -> (i32, i32) {
        let rect = session
            .game_mut()
            .staged_side_control_rect(code)
            .unwrap_or_else(|| panic!("no staged MyRoom control for {code:#x}"));
        let centre = rect_centre(rect);
        session.ingest_snapshot(&InputSnapshot::click_at(centre.0, centre.1));
        centre
    }

    /// Changed pixels inside one staged rectangle, so a guard proves
    /// the repaint landed where the player sees the behavior.
    fn changed_pixels_in_rect(before: &[u32], after: &[u32], rect: Rect) -> usize {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let left = usize::try_from(rect.x).expect("rect left fits usize");
        let top = usize::try_from(rect.y).expect("rect top fits usize");
        let columns = usize::try_from(rect.w).expect("rect width fits usize");
        let rows = usize::try_from(rect.h).expect("rect height fits usize");
        (top..top + rows)
            .flat_map(|y| (left..left + columns).map(move |x| y * width + x))
            .filter(|index| before[*index] != after[*index])
            .count()
    }

    /// Paint at explicit times so a static face cannot pass on one
    /// lucky frame (owner self-verify rule, `docs/rig-runbook.md`).
    fn paint_at_frame(session: &mut OnlineSession, now_ms: u64) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
        let height = usize::try_from(NATIVE_HEIGHT).expect("native height fits usize");
        let mut buf = vec![0u32; width * height];
        session.game_mut().paint_at(
            &mut o2jam_render::Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            now_ms,
        );
        buf
    }

    /// Counts every backend emission: neither MyRoom arm owns a cue.
    struct CueCountBackend {
        inner: NullBackend,
        calls: Rc<RefCell<u64>>,
    }

    impl AudioBackend for CueCountBackend {
        fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), o2jam_audio::AudioError> {
            *self.calls.borrow_mut() += 1;
            self.inner.play_pcm(pcm)
        }

        fn play_key(&mut self, pcm: &PcmSamples, gain: f32) -> Result<(), o2jam_audio::AudioError> {
            *self.calls.borrow_mut() += 1;
            self.inner.play_key(pcm, gain)
        }

        fn set_scene_bgm(
            &mut self,
            pcm: Option<&PcmSamples>,
            repeat: bool,
        ) -> Result<(), o2jam_audio::AudioError> {
            *self.calls.borrow_mut() += 1;
            self.inner.set_scene_bgm(pcm, repeat)
        }

        fn samples_played(&self) -> u64 {
            self.inner.samples_played()
        }
    }

    /// Attach a cue counter after side entry and settle one frame so
    /// the backend swap itself is not counted as a click effect.
    fn attach_cue_counter(session: &mut OnlineSession) -> Rc<RefCell<u64>> {
        let calls = Rc::new(RefCell::new(0));
        session.game_mut().with_audio(
            Box::new(CueCountBackend {
                inner: NullBackend::new(),
                calls: Rc::clone(&calls),
            }),
            std::collections::HashMap::new(),
        );
        paint(session);
        calls
    }

    /// The two #180 legs share one fake gateway shape. Serializing
    /// them keeps a second MyRoom gateway from racing the clock-based
    /// Waiting/Result legs when cargo runs the whole suite in parallel.
    static MYROOM_LIVE_LEG_LOCK: Mutex<()> = Mutex::new(());

    /// #180 T5: the baked STATUS badge click flips the upper-right
    /// face and sends nothing. Statics pin the first-click direction
    /// and the local face write (`FUN_004a3ee0`, file `0xa3ee0`,
    /// `ev_a91d2baf…96d6030`; face helpers `ev_6c8005bd…34ddbb36e10`,
    /// `ev_938054dc…6a4a8301ae4ee6c58e3`,
    /// `ev_edb8ac9c…9087870322800e94a317b0e5e8`). The face-flag writer
    /// and the alternation rule stay OPEN: next step is a two-click
    /// qualified capture plus a watch on writes to the control's `+0x60`.
    /// The live-4 pack cited in `docs/myroom.md` supplies the observed
    /// Inventory -> STATUS -> Inventory round trip.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (splash Menu edge)
    ///   -> staged emblem click -> Screen::Channel -> staged row click -> Screen::Room
    ///   -> click_staged(Screen::Room, IDC_BUTTON_SROOM_MYROOM) -> visit_side(MyRoom)
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_myroom_side(0x15203034)
    ///   -> MyRoomAction::ToggleStatusBadge -> ScreenGame::show_myroom_panel
    ///   -> ScreenGame::paint_at -> paint_side -> staged status plate.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn live_click_on_the_baked_status_badge_opens_the_status_plate_from_boot() {
        let _leg_guard = MYROOM_LIVE_LEG_LOCK.lock().expect("MyRoom leg lock");
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        enter_myroom_by_click(&mut session);
        wait_for_enter_shop(&gateway.reports, "IDC_BUTTON_SROOM_MYROOM");
        let cue_calls = attach_cue_counter(&mut session);
        let cue_baseline = *cue_calls.borrow();

        assert_eq!(
            session.game_mut().myroom().panel(),
            MyRoomPanel::Inventory,
            "live entry opens the exe's Inventory face"
        );
        let badge_rect = session
            .game_mut()
            .staged_side_control_rect(MYROOM_STATUS_BADGE_CODE)
            .expect("STATE_MYROOM stages the INVENTORY control");
        assert_eq!(badge_rect, STATUS_BADGE_RECT, "bnd rect matches the pin");
        let plate_rect = session
            .game_mut()
            .staged_side_control_rect(0x1550_0036)
            .expect("STATE_MYROOM stages the USERINFO plate");
        assert_eq!(plate_rect, STATUS_PLATE_RECT, "bnd rect matches the pin");

        let before = paint_frame(&mut session);
        let aim = click_staged_side_code(&mut session, MYROOM_STATUS_BADGE_CODE);
        assert_eq!(aim, (177, 318), "the logged exe click lands on the badge");
        assert_eq!(
            session.game_mut().myroom().panel(),
            MyRoomPanel::Status,
            "the badge press must flip the face"
        );
        assert!(
            !session.game_mut().has_pending_equip_send(),
            "the badge is not the equip-apply control"
        );
        let after = paint_frame(&mut session);
        assert!(
            changed_pixels_in_rect(&before, &after, STATUS_PLATE_RECT) > 0,
            "the status plate must ink the pinned rect"
        );
        assert!(
            changed_pixels_in_rect(&before, &after, STATUS_BADGE_RECT) > 0,
            "the INVENTORY body must replace the baked badge"
        );
        assert!(
            observed(&gateway.reports).is_empty(),
            "the face flip sends no wire frame: {:?}",
            observed(&gateway.reports)
        );
        assert_eq!(
            *cue_calls.borrow(),
            cue_baseline,
            "the face flip queues no backend cue"
        );
        for now_ms in [0, 20_000, 50_000] {
            let sampled = paint_at_frame(&mut session, now_ms);
            assert!(
                changed_pixels_in_rect(&before, &sampled, STATUS_PLATE_RECT) > 0,
                "the status plate still inks its rect at {now_ms} ms"
            );
            assert!(
                changed_pixels_in_rect(&before, &sampled, STATUS_BADGE_RECT) > 0,
                "the INVENTORY body still inks its rect at {now_ms} ms"
            );
        }

        click_staged_side_code(&mut session, MYROOM_STATUS_BADGE_CODE);
        assert_eq!(
            session.game_mut().myroom().panel(),
            MyRoomPanel::Inventory,
            "the second badge press restores the baked STATUS label"
        );
        let round_trip = paint_frame(&mut session);
        assert_eq!(
            changed_pixels_in_rect(&before, &round_trip, STATUS_PLATE_RECT),
            0,
            "the round trip restores the entry plate pixels"
        );
        finish(session, gateway.server);
    }

    /// #180 T6: PRESENT BOX and MY BAG are a reversible local tab.
    /// The exe dispatcher writes tab word `0x779924` (MY BAG 0, PRESENT
    /// BOX 1) with no wire send (`FUN_004a3ee0`, file `0xa3ee0`,
    /// `ev_a91d2baf…96d6030`); `FUN_004a8ba0` (file `0xa8ba0`,
    /// `ev_0da46dd8…d45cd4daf7`) applies that word to the staged rows.
    /// The old `8515a7e5…` reading was backwards (#915 story 9).
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> setup_online -> connect::connect_and_login_with_transport
    ///   -> assemble_online_session -> GameLoop::ingest_snapshot (splash Menu edge)
    ///   -> staged emblem click -> Screen::Channel -> staged row click -> Screen::Room
    ///   -> click_staged(Screen::Room, IDC_BUTTON_SROOM_MYROOM) -> visit_side(MyRoom)
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_myroom_side(0x15600046)
    ///     -> ShowGiftTab -> paint_side -> gift rows
    ///   -> GameLoop::ingest_snapshot -> ScreenGame::click_myroom_side(0x15600045)
    ///     -> ShowBagTab -> paint_side -> bag rows and badge frame 1.
    /// Archives absent -> skip cleanly (CI has no game data).
    #[test]
    fn my_bag_press_round_trips_the_present_box_and_bag_grid_from_boot() {
        let _leg_guard = MYROOM_LIVE_LEG_LOCK.lock().expect("MyRoom leg lock");
        if !archives_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let gateway = spawn_live_leg_gateway(joined_room(0, true));
        let mut session = boot_live_leg(gateway.port, false);
        dismiss_splash(&mut session);
        enter_room_by_clicks(&mut session);
        enter_myroom_by_click(&mut session);
        wait_for_enter_shop(&gateway.reports, "IDC_BUTTON_SROOM_MYROOM");
        let cue_calls = attach_cue_counter(&mut session);
        let cue_baseline = *cue_calls.borrow();

        assert_eq!(
            session.game_mut().myroom().lower_tab(),
            MyRoomLowerTab::Bag,
            "live entry opens MY BAG"
        );
        let bag_cell = session
            .game_mut()
            .staged_side_control_rect(0x1560_0023)
            .expect("STATE_MYROOM stages MYBAGITEM1");
        let gift_cell = session
            .game_mut()
            .staged_side_control_rect(0x1501_c04a)
            .expect("STATE_MYROOM stages GIFTBOX_EX1");
        let on_bag = paint_frame(&mut session);

        click_staged_side_code(&mut session, MYROOM_PRESENT_BOX_TAB_CODE);
        assert_eq!(
            session.game_mut().myroom().lower_tab(),
            MyRoomLowerTab::Gifts
        );
        let on_gift = paint_frame(&mut session);
        assert!(
            changed_pixels_in_rect(&on_bag, &on_gift, bag_cell) > 0,
            "the bag grid must disappear under PRESENT BOX"
        );
        assert!(
            changed_pixels_in_rect(&on_bag, &on_gift, gift_cell) > 0,
            "the gift rows must appear under PRESENT BOX"
        );

        click_staged_side_code(&mut session, MYROOM_MY_BAG_TAB_CODE);
        assert_eq!(
            session.game_mut().myroom().lower_tab(),
            MyRoomLowerTab::Bag,
            "MY BAG must close the present box"
        );
        let back_on_bag = paint_frame(&mut session);
        assert_eq!(
            changed_pixels_in_rect(&on_bag, &back_on_bag, bag_cell),
            0,
            "the bag grid returns to the boot pixels"
        );
        assert_eq!(
            changed_pixels_in_rect(&on_bag, &back_on_bag, gift_cell),
            0,
            "the gift rows are hidden again"
        );

        click_staged_side_code(&mut session, MYROOM_PRESENT_BOX_TAB_CODE);
        assert_eq!(
            session.game_mut().myroom().lower_tab(),
            MyRoomLowerTab::Gifts
        );
        let gift_again = paint_frame(&mut session);
        assert_eq!(
            changed_pixels_in_rect(&on_gift, &gift_again, gift_cell),
            0,
            "the second PRESENT BOX press restores the gift face"
        );
        assert!(
            observed(&gateway.reports).is_empty(),
            "both tabs stay local: {:?}",
            observed(&gateway.reports)
        );
        assert_eq!(
            *cue_calls.borrow(),
            cue_baseline,
            "neither tab queues a backend cue"
        );
        for now_ms in [0, 20_000, 50_000] {
            let sampled = paint_at_frame(&mut session, now_ms);
            assert!(
                changed_pixels_in_rect(&on_bag, &sampled, bag_cell) > 0,
                "the gift face still hides the bag cell at {now_ms} ms"
            );
            assert!(
                changed_pixels_in_rect(&on_bag, &sampled, gift_cell) > 0,
                "the gift face still inks the gift cell at {now_ms} ms"
            );
        }
        finish(session, gateway.server);
    }
}
