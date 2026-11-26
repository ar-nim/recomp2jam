//! Shared scaffolding for the live-input legs (#739, split for #926).
//!
//! One boot/gateway/paint helper set serves every per-family leg file, so
//! parallel lanes append tests in their own files instead of interleaving
//! at one append point. Leg files `use crate::live_leg_support::*;` and
//! skip cleanly when the installed archives are absent (CI has no game
//! data, #0a3ba).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, MutexGuard};
use std::thread::JoinHandle;

use o2jam_net::frame::encode_frame;
use o2jam_net::testing::{fake_gateway, read_frame};
use o2jam_net::{
    AuthResponse, ChannelList, ChannelLoginResponse, ChannelState, Frame, Gender, JoinRoomResponse,
    Request, RoomList, RoomListEntry, RoomSlotState, WireMessage,
};
use o2jam_render::{Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::screens::game::{LobbyChat, PlanetModal};
use o2jam_shell::screens::paint::dialog_origin;

use o2jam_shell::screens::side::{INFORMATION_OK_CODE, OPTION_SOUND_KEYSET, PLANET_OPTION_CODE};
use o2jam_shell::staging::{read_install_archives, stage_dialog, Dialog, StageContext};

use o2jam_shell::{route_key_event, GameLoop, InputEdge, InputSnapshot, KeyRoute, Keymap, Screen};
use winit::keyboard::KeyCode;

use crate::{
    assemble_online_session, connect, install_root, session::OnlineSession,
    stage_channel_list_after_relay_gate, stage_online_identity, stage_online_rooms,
};
use o2jam_shell::screens::side::{PlanetOptionTab, ROOM_OPTION_BUTTON_CODE};

/// One lock for every booted live leg. A dual-client leg runs two gateways;
/// without this, its threads can starve another leg's gateway and trip a
/// wall-clock answer wait in the full suite.
static LIVE_LEG_LOCK: Mutex<()> = Mutex::new(());

/// Hold this guard for a whole leg, after its archive check. The guard
/// releases the lock on every exit, including a panic or a SKIP return.
///
/// A poisoned lock is recovered, not propagated: one failing leg must
/// report as ONE failure. Propagating the poison turned a single
/// wallet-leg timeout into 118 failures on 2026-11-26 and hid which
/// leg actually broke. The lock guards no data (it is `()`), so there
/// is no inconsistent state to protect.
pub(crate) fn hold_live_leg_lock() -> MutexGuard<'static, ()> {
    LIVE_LEG_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Live boot rows: one channel (the resident one) and one room.
pub(crate) fn channel_row() -> ChannelState {
    ChannelState {
        gateway_id: 1,
        channel_id: 2,
        capacity: 100,
        population: 1,
        active: true,
    }
}

pub(crate) fn room_row() -> RoomListEntry {
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

pub(crate) fn boot_options(port: u16) -> connect::ConnectOptions {
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
pub(crate) struct LiveLegGateway {
    pub(crate) port: u16,
    pub(crate) reports: Receiver<Frame>,
    pub(crate) join_answer_written: Receiver<()>,
    pub(crate) server: JoinHandle<()>,
}

/// Drive the 1000/1001/1002/1003 boot exchange, then record every
/// later client frame until the socket goes quiet. The channel and
/// room rows come back on the wire, not from a test-side list. The
/// 3003 write is signalled so a Waiting click never races the
/// gateway thread's scheduling under fleet load.
pub(crate) fn spawn_live_leg_gateway(joined: o2jam_net::JoinedRoom) -> LiveLegGateway {
    spawn_leg_gateway(joined, false)
}

/// Same boot gateway with the two result answers: 4011 on START
/// and 4022 only on the second 4021 (the first is the tally
/// triple's entry exit, the second is the BACK click).
pub(crate) fn spawn_result_back_gateway(joined: o2jam_net::JoinedRoom) -> LiveLegGateway {
    spawn_leg_gateway(joined, true)
}

pub(crate) fn spawn_leg_gateway(
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
pub(crate) fn read_frame_until_eof(stream: &mut TcpStream) -> Option<Frame> {
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
pub(crate) fn joined_room(member_id: u8, peer_ready: bool) -> o2jam_net::JoinedRoom {
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
pub(crate) fn boot_live_leg(port: u16, relay_down: bool) -> OnlineSession {
    let (report, gateway) = connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
    let mut game = o2jam_shell::ScreenGame::new();
    stage_online_identity(&mut game, report.character.as_ref());
    stage_channel_list_after_relay_gate(&mut game, report.channels.clone(), relay_down);
    stage_online_rooms(&mut game, report.rooms.clone());
    let mut session =
        assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
    session.set_resident_channel(channel_row());
    session
}

pub(crate) fn archives_present() -> bool {
    read_install_archives(&install_root()).interface.is_some()
}

/// Boot path used by `main.rs::run_play`: the session stages live
/// art before any frame, so dialog rects and figures are real.
pub(crate) fn boot_live_leg_with_art(port: u16, relay_down: bool) -> OnlineSession {
    let mut session = boot_live_leg(port, relay_down);
    session.stage_live_art();
    session
}

/// Same live boot, but the interface archive is staged before the
/// socket connects. The fake gateway starts its five-second quiet
/// timer only after `accept`, so archive reads cannot close the
/// socket under a loaded test fleet.
pub(crate) fn boot_live_leg_with_prestaged_art(port: u16, relay_down: bool) -> OnlineSession {
    let mut game = o2jam_shell::ScreenGame::new();
    crate::live_art::stage_live_scenes(&mut game);
    let (report, gateway) = connect::connect_and_login_with_transport(&boot_options(port)).unwrap();
    stage_online_identity(&mut game, report.character.as_ref());
    stage_channel_list_after_relay_gate(&mut game, report.channels.clone(), relay_down);
    stage_online_rooms(&mut game, report.rooms.clone());
    let mut session =
        assemble_online_session(game, gateway, Some("gate"), None, None, None).unwrap();
    session.set_resident_channel(channel_row());
    session
}

/// One staged option control's absolute native rect. Geometry is
/// always read from the same `.bnd` block live boot stages.
pub(crate) fn staged_dialog_rect(dialog: Dialog, code: u32) -> Rect {
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

pub(crate) fn staged_dialog_centre(dialog: Dialog, code: u32) -> (i32, i32) {
    rect_centre(staged_dialog_rect(dialog, code))
}

/// Which lobby face the Options sheet was opened from. The dialog
/// has no per-caller state (#314 static pin), so both entries must
/// land on the same capture and paint behavior.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OptionsEntry {
    Room,
    Planet,
}

/// Boot -> splash -> entry click -> Options sheet -> Key tab, all
/// through pointer snapshots. No direct `switch_option_tab` call.
pub(crate) fn open_options_key_sheet(session: &mut OnlineSession, entry: OptionsEntry) {
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
pub(crate) fn route_raw_key(
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
pub(crate) fn rebind_staged_lane(
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
pub(crate) fn key_sheet_state(session: &mut OnlineSession) -> (String, String) {
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
pub(crate) fn staged_rect_pixels(session: &mut OnlineSession, rect: Rect) -> Vec<u32> {
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
pub(crate) fn ink_pixels(pixels: &[u32]) -> usize {
    let mut counts = std::collections::HashMap::new();
    for pixel in pixels {
        *counts.entry(*pixel).or_insert(0usize) += 1;
    }
    let dominant = counts.values().copied().max().unwrap_or(0);
    pixels.len().saturating_sub(dominant)
}

/// Sample one absolute staged rect from an already painted frame.
pub(crate) fn staged_rect_pixels_from(frame: &[u32], rect: Rect) -> Vec<u32> {
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
pub(crate) const KEY_CAPTION_DIK_ORDER: [u8; 37] = [
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1E, 0x1F, 0x20, 0x21,
    0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34,
    0x35, 0x36, 0x2A, 0x39, 0x1C,
];

/// Pixels the bound caption must paint in one staged rebind cell.
/// The expected glyph comes from the same `.bnd`/`.opi` figures live
/// boot staged; the guard never invents a colour or a coordinate.
pub(crate) fn bound_caption_pixels(rect: Rect, dik: u8) -> Vec<(usize, usize, u32)> {
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
pub(crate) fn caption_mismatches(frame: &[u32], caption: &[(usize, usize, u32)]) -> usize {
    let width = usize::try_from(NATIVE_WIDTH).expect("native width fits usize");
    caption
        .iter()
        .filter(|(x, y, word)| frame[y * width + x] != *word)
        .count()
}

/// Splash dismissal is a Menu edge in a snapshot — the same edge
/// the window queues for Escape, never a direct `menu_key` call.
pub(crate) fn dismiss_splash(session: &mut OnlineSession) {
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
pub(crate) fn click_staged(session: &mut OnlineSession, screen: Screen, id: &str) {
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
pub(crate) fn information_ok_centre() -> (i32, i32) {
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

pub(crate) fn rect_centre(rect: Rect) -> (i32, i32) {
    let half_width = i32::try_from(rect.w / 2).expect("rect width fits a frame coordinate");
    let half_height = i32::try_from(rect.h / 2).expect("rect height fits a frame coordinate");
    (rect.x + half_width, rect.y + half_height)
}

/// Paint budget for a screen change already driven by input or a
/// gateway signal. Counting paints, not wall time, keeps a loaded
/// scheduler out of the assertion (#754).
pub(crate) const MAX_SCREEN_PAINTS: usize = 64;

/// Paint until the boot-driven action reaches its next screen.
pub(crate) fn paint_until_screen(session: &mut OnlineSession, want: Screen) {
    for _ in 0..MAX_SCREEN_PAINTS {
        if session.screen() == want {
            return;
        }
        paint(session);
    }
    assert_eq!(session.screen(), want, "live click never reached {want:?}");
}

pub(crate) fn paint(session: &mut OnlineSession) {
    drop(paint_frame(session));
}

/// Paint one native frame and return its pixels, so a guard can
/// prove a mounted sheet actually changed the picture.
pub(crate) fn paint_frame(session: &mut OnlineSession) -> Vec<u32> {
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
pub(crate) fn enter_room_by_clicks(session: &mut OnlineSession) {
    session.ingest_snapshot(&InputSnapshot::click_at(136, 88));
    assert_eq!(session.screen(), Screen::Channel);
    session.ingest_snapshot(&InputSnapshot::click_at(524, 246));
    paint_until_screen(session, Screen::Room);
}

/// Click path to Waiting: the room row press queues 3002 and the
/// 3003 answer replays the same press into the shell. Waiting on
/// the gateway's write signal removes the wall-clock race (#754).
pub(crate) fn join_waiting_by_click(
    session: &mut OnlineSession,
    join_answer_written: &Receiver<()>,
) {
    click_staged(session, Screen::Room, "IDC_BUTTON_SROOM_EXBUTTON1");
    // Paint first: this frame posts 3002. The gateway writes 3003
    // only after it reads that frame, then signals the write.
    paint(session);
    join_answer_written
        .recv()
        .expect("the gateway wrote the 3003 join answer");
    paint_until_screen(session, Screen::Waiting);
}

pub(crate) fn observed(reports: &Receiver<Frame>) -> Vec<(u16, Vec<u8>)> {
    reports
        .try_iter()
        .map(|frame| (frame.id, frame.payload))
        .collect()
}

/// Wait for one opcode after a paint: the gateway thread records
/// frames as it reads them, so an immediate `try_iter` can outrun
/// the socket (#52). The channel is the signal; a closed channel
/// means the gateway died before the frame reached it (#754).
pub(crate) fn wait_for_opcode(reports: &Receiver<Frame>, want: u16) -> Vec<u8> {
    loop {
        match reports.recv() {
            Ok(frame) if frame.id == want => return frame.payload,
            Ok(_) => {}
            Err(_) => panic!("gateway closed before opcode {want}"),
        }
    }
}

/// Click a staged placement by code, not by a guessed coordinate.
pub(crate) fn staged_code_centre(
    session: &mut OnlineSession,
    screen: Screen,
    code: u32,
) -> (i32, i32) {
    let placement = session
        .game_mut()
        .scene_placements(screen)
        .iter()
        .find(|placed| placed.code == code)
        .unwrap_or_else(|| panic!("no staged placement for {code:#x} on {screen:?}"));
    rect_centre(placement.rect)
}

/// Closing the client ends the recorder thread's read loop.
pub(crate) fn finish(session: OnlineSession, server: JoinHandle<()>) {
    drop(session);
    server.join().unwrap();
}
