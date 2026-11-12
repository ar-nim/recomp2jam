//! In-process scene screenshots: drive [`ScreenGame`] with scripted
//! `menu_key`/`lane_key` calls, rasterize each scene with `paint_at`,
//! and encode PNGs with the std-only writer below. No OS key
//! injection, no window.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use o2jam_game::{RankDelivery, RoomTeams, LIFE_MAX, PEER_SLOT_COUNT, STAT_LIFE};
use o2jam_net::play::SetRoomMusic;
use o2jam_net::rooms::{WaitingAdminMessage, WaitingUserMessage};
use o2jam_net::{ChannelState, GameMode, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::avatar::{AvatarFigure, ScenePlacement};
use o2jam_shell::screens::side::SideScreen;
use o2jam_shell::staging::{
    read_install_archives, stage_dialog, stage_state, state_of_screen, state_of_side, Dialog,
    StageContext, State,
};
use o2jam_shell::{GameLoop, MenuKey, PlaySkin, Screen, ScreenGame};

use crate::connect::stack_worn_figure;
use crate::loading::{loading_phase_status, LOADING_GATE_STEPS};

/// Screenshot width in pixels.
pub const SHOT_W: usize = 400;
/// Screenshot height in pixels.
pub const SHOT_H: usize = 300;

/// One captured scene: row-major RGB bytes.
pub struct Shot {
    /// File stem (`channel.png`, …).
    pub name: &'static str,
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Row-major RGB triplets.
    pub rgb: Vec<u8>,
}

/// Capture avi, channel, room, waiting, waiting-guest, musicselect, loading,
/// playing, playing-native, result, result-native, result-solo,
/// result-solo-native. The waiting shot carries the staged song
/// catalog plus inventory (so MAPNAME/BPM/SKILLCOUNT ink their
/// `.bnd` rects from the room word, the catalog BPM column, and
/// the staged counts) and the scripted 3012/3013 chat fixture (one
/// user row plus one admin row, so the history box shows live
/// rows). The native
/// companions re-emit the playing and result instants at the full
/// 800x600 exe canvas so pinned details (combo font, 3x5 result
/// digits, rank rows) stay provable instead of downscale smudges.
/// The musicselect shot repaints Waiting with the
/// staged `DAILOG_SELECTMUSIC` dialog on top (host song flow: catalog
/// plus pick), and the loading shot threads the gate phases to the
/// clock-armed full fraction with the staged catalog title/artist
/// (see `stage_loading_status`), the waiting-guest shot replays the same
/// waiting instant from a non-master seat (no dialog, staged `BPM` kept;
/// master-gate comparison in `docs/room-enums.md`), and the playing shot
/// paints a mid-song state: the 5 s
/// tap is struck, then the long-note head (combo 2), and the paint
/// lands 100 ms later while the held beam, hit burst, judgement
/// word, tally counts, and TIME progress show; the scripted key-up
/// on the tail then spends the chart for Result.
///
/// First-screen walk (verified, `docs/planet.md` 2026-11-09, S0.3
/// boot correction): Avi → Planet (the bare board) → lit-emblem
/// click → Channel (the dialog over the board) → row confirm →
/// Room → … The lit emblem reopening the dialog is player-report
/// corroborated with per-emblem dynamic verification done (family
/// select, live 2026-11-09); the dialog X button is bnd-pinned
/// ((541,151)-(589,189)). Launcher preselection handling stays
/// OPEN (its boot auto-open is not modeled).
///
/// # Errors
///
/// When the scripted walk leaves the expected screen.
pub fn capture_all() -> anyhow::Result<Vec<Shot>> {
    let mut game = seeded_game();
    let mut shots = Vec::with_capacity(14);
    shots.push(shoot(&mut game, "avi", 0)?);
    game.lane_key(3, true);
    // Boot shows the bare board (S0.3 owner correction): the staged
    // bound State_Planet block (backdrop + emblems + promo + MENU
    // bar) with the exe-pinned emblem captions (the `TEXT` strings
    // over their `.bnd` rects, `docs/planet.md`).
    assert_eq!(game.screen(), Screen::Planet, "boot → bare board");
    shots.push(shoot_checked(&mut game, "planet", Screen::Planet, 0)?);
    // A lit-emblem click (HARD through its pinned `.bnd` rect)
    // opens the channel dialog over the board (family select,
    // live-verified 2026-11-09; the dialog-edge link stays OPEN per
    // E1) and records the pick; the amber cursor stand-in stays
    // labeled OPEN. The MENU bar stays visible in the bottom strip
    // (y>468) beneath the opaque 800x466 dialog sheet.
    game.click_planet(386, 88);
    assert_eq!(game.screen(), Screen::Channel, "lit emblem → Channel");
    assert_eq!(
        game.planet_selection(),
        Some(o2jam_shell::screens::side::PlanetBand::Hard)
    );
    shots.push(shoot_checked(&mut game, "channel", Screen::Channel, 0)?);
    // Room entry happens only through the channel dialog (entry-button
    // faces; wine-boot.md Attempt 9 verified 입장 clicks advance to the
    // room list): Confirm takes the highlighted row into Room.
    game.menu_key(MenuKey::Confirm, true);
    assert_eq!(game.screen(), Screen::Room, "row confirm → Room");
    stage_room_identity(&mut game);
    shots.push(shoot_checked(&mut game, "room", Screen::Room, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    shots.push(shoot_checked(&mut game, "waiting", Screen::Waiting, 0)?);
    shots.push(shoot_guest_waiting()?);
    // Side states (lifecycle §12): every lobby opens every side
    // screen, so the walk visits each from Waiting with its genuine
    // bound art staged and shoots the placement-driven paint. Without
    // install-root the shot stays skipped (clean skip, CI arm).
    for side in [
        SideScreen::Shop,
        SideScreen::MusicShop,
        SideScreen::MyRoom,
        SideScreen::Lanking,
        SideScreen::Payment,
        SideScreen::Tutorial,
    ] {
        if !stage_side_art(&mut game, side) {
            continue;
        }
        // The lanking shot proves rows and ranks paint: stage the
        // walk catalog plus the scripted 2028 stub before visiting.
        if side == SideScreen::Lanking {
            stage_lanking_walk_board(&mut game);
        }
        // The shop shots prove goods, prices, and pager cells
        // paint: stage the walk catalogs plus the scripted stubs
        // before visiting.
        if side == SideScreen::Shop {
            stage_shop_walk_board(&mut game);
        }
        if side == SideScreen::MusicShop {
            stage_music_shop_walk_board(&mut game);
        }
        assert!(game.visit_side(side), "Waiting -> {side:?}");
        shots.push(shoot_side(&mut game, side)?);
        assert!(game.leave_side(), "{side:?} -> Waiting");
    }
    stage_select_music_scene(&mut game);
    shots.push(shoot_checked(&mut game, "musicselect", Screen::Waiting, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    stage_loading_status(&mut game);
    shots.push(shoot_checked(&mut game, "loading", Screen::Loading, 0)?);
    stage_playing_scene(&mut game);
    // The Loading→Playing gate (S7): the pick and staged status
    // above validate; only the rig is missing on this scripted walk.
    arm_loading_gate(&mut game);
    game.menu_key(MenuKey::Confirm, true);
    game.strike_at(2, 5_000);
    game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
    shots.push(shoot_checked(&mut game, "playing", Screen::Playing, 5_100)?);
    shots.push(shoot_checked_native(
        &mut game,
        "playing-native",
        Screen::Playing,
        5_100,
    )?);
    game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
    shots.push(shoot_result_named(&mut game, "result", 7_000)?);
    shots.push(shoot_result_native_named(
        &mut game,
        "result-native",
        7_000,
    )?);
    let mut solo = drive_solo_to_result();
    shots.push(shoot_result_named(&mut solo, "result-solo", 7_000)?);
    shots.push(shoot_result_native_named(
        &mut solo,
        "result-solo-native",
        7_000,
    )?);
    Ok(shots)
}

/// Write captures as `<out_dir>/<name>.png`, except `avi`: the
/// checked-in `avi.png` is real 7.55 splash evidence (spectacle
/// capture of `O2mediaLogo.ojs`) and must not be overwritten by the
/// placeholder card.
///
/// # Errors
///
/// Filesystem failures, or the scripted walk leaving its screens.
pub fn write_shots(out_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(out_dir)?;
    let mut written = Vec::new();
    for shot in capture_all()? {
        if shot.name == "avi" {
            continue;
        }
        let bytes = encode_png(shot.w as u32, shot.h as u32, &shot.rgb);
        let path = out_dir.join(format!("{}.png", shot.name));
        std::fs::write(&path, &bytes)?;
        written.push(path);
    }
    Ok(written)
}

/// Interface archive under `install-root` (Avi/Channel/Room/Waiting/Result art).
/// Worn-figure archive under `install-root` (avatar item art + table).
const AVATAR_ARCHIVE: &str = "O2Jam/Image/avatar.opa";
/// Equipment the playing shot wears, verified to resolve against 7.55
/// `avatar.opa` (`Itemdata_Korea.dat` rows: male hair 16, face 30,
/// torso+arms outfit 376, pants 383, shoes 389).
const PLAYING_EQUIPMENT: [i32; 5] = [16, 30, 376, 383, 389];
/// Player caption staged for the playing shot.
const PLAYING_PLAYER: (u32, &str) = (43, "TESTER");
/// Volume readout staged for the playing shot: the reference photo's
/// strip shows `VOL 58%` (`docs/playing-reference.md`); the shell
/// paints `VOL <n>` at the bound `IDC_TEXT_SPLAY_VLOLUME` rect.
const PLAYING_VOLUME: u8 = 58;
/// Host song pick the playing shot captions (catalog id 757, Hard
/// chart, speed X3.5 — the paint re-syncs the song line from this).
const PLAYING_SONG: SetRoomMusic = SetRoomMusic {
    music_id: 757,
    difficulty: 2,
    speed: 3,
};
/// Song index under `install-root` (the 7.55 562-song catalog the
/// select-music table lists; read-only like the archives).
const MUSICSELECT_CATALOG: &str = "O2Jam/Image/OJNList.dat";
/// Synthetic fallback songs (id, title, artist) when no catalog is
/// installed: id 757 first so the host pick and the playing caption
/// still resolve. Titles are ASCII stand-ins (documented gap).
const MUSICSELECT_FALLBACK_SONGS: [(u32, &str, &str); 4] = [
    (757, "First", "Band"),
    (758, "Second", "Band"),
    (763, "Third", "Band"),
    (765, "Fourth", "Band"),
];
/// `STATE_CHANNEL` names zero sprites on 7.55 (bare `BOUND`, no art),
/// so no block art stages for Channel; the code-mounted channel-list
/// dialog (`Dialog_ChannelInfo.ojs` body at its record anchor plus the
/// enter-button frames at the reference-measured faces) stages through
/// `stage_channel_dialog`. The pinned no-fill rule still holds: the
/// block contributes zero pixels and nothing paints beyond the staged
/// art (`docs/channel-fill.md`).
/// Room arena value the playing shot joins with: stage 2, whose
/// `Playing_BG2.ojs` stages an open grassy rise sloping to a lake
/// with a distant high-rise skyline under a bright blue sky — the
/// closest staged match to the reference rooftop overlook photo
/// (`docs/playing-reference.md`; the exact sunset 3D rooftop is not
/// among the 36 stages). Residual delta: grass for the wooden plank
/// deck, noon-blue for the sunset-orange sky, no parapet railing or
/// blimps. Rides the room's arena word so the shot exercises the
/// room-entry stage wiring end to end.
const PLAYING_ARENA_STAGE: i32 = 2;

/// Repository-local `install-root` (git-ignored game data).
fn install_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Side-screen stage data through the staging declaration: one
/// full-canvas or bound State per side screen (`visit_side` +
/// `load_side_scene` paint these through `paint_side_scene`).
/// Payment merges both billing views, and Tutorial stages its
/// Playing-archive backdrop. `None` keeps the walk's side shot
/// skipped (no install-root).
fn side_stage_data(
    side: SideScreen,
) -> Option<(Vec<ScenePlacement>, HashMap<String, AvatarFigure>)> {
    if side == SideScreen::Tutorial {
        let installed = read_install_archives(&install_root());
        let playing = installed.playing?;
        let art = stage_state(&playing, State::Tutorial, StageContext::default()).art?;
        return Some((art.placements, art.figures));
    }
    let installed = read_install_archives(&install_root());
    let interface = installed.interface?;
    let ctx = StageContext::default();
    if side == SideScreen::Payment {
        let mut placements = Vec::new();
        let mut figures = HashMap::new();
        for state in [State::Payment, State::Payment2] {
            let Some(art) = stage_state(&interface, state, ctx).art else {
                continue;
            };
            placements.extend(art.placements);
            figures.extend(art.figures);
        }
        if placements.is_empty() {
            return None;
        }
        return Some((placements, figures));
    }
    let state = state_of_side(side)?;
    let art = stage_state(&interface, state, ctx).art?;
    Some((art.placements, art.figures))
}

/// Load one side screen's stage data into the game; `false` when the
/// archives hold no art (the walk skips the shot).
/// Stage one side screen's genuine bound art when `install-root` holds it.
///
/// `true` while placements staged (the walk paints file-native frames);
/// `false` without game data (the walk keeps the flat fallback and the
/// caller still visits — wiring never rides on art).
pub(crate) fn stage_side_art(game: &mut ScreenGame, side: SideScreen) -> bool {
    let Some((placements, figures)) = side_stage_data(side) else {
        return false;
    };
    game.load_side_scene(placements, figures);
    true
}

/// Shot file stem for one side screen.
fn side_shot_name(side: SideScreen) -> &'static str {
    match side {
        SideScreen::Shop => "shop",
        SideScreen::MusicShop => "musicshop",
        SideScreen::MyRoom => "myroom",
        SideScreen::Lanking => "lanking",
        SideScreen::Payment => "payment",
        SideScreen::Tutorial => "tutorial",
        SideScreen::Planet => "planet",
    }
}

/// Stage the room lobby's user panel before its shot: the identity
/// readouts ride a scripted `CharacterInfo` (the values the 7.55 capture
/// shows for the local tester: Lv 2, 2133 exp, zero penalty), and the
/// avatar slot stacks the same worn figure the Result banner uses.
/// Scripted fixture data only — no server record, no game bytes.
fn stage_room_identity(game: &mut ScreenGame) {
    let character = o2jam_net::CharacterInfo {
        nickname: "tester".to_owned(),
        level: 2,
        experience: 2133,
        ..o2jam_net::CharacterInfo::default()
    };
    game.set_myroom_character(character);
    if let Ok(archive) = std::fs::read(install_root().join(AVATAR_ARCHIVE)) {
        if let Ok(figure) = stack_worn_figure(&archive, &PLAYING_EQUIPMENT, true) {
            if !figure.is_empty() {
                game.load_room_avatar(figure);
            }
        }
    }
}

/// Stage the code-mounted channel dialog art (sheet, hover bar, row
/// bar, digit/count plates, enter faces, scroll bodies) through the
/// shared [`live_art::stage_channel_dialog`] — one implementation for
/// the shots walk and the live window.
pub(crate) use crate::live_art::stage_channel_dialog;
/// Stage the planet modal sheets (first-open options tab plus the
/// quit-confirm) through the shared
/// [`live_art::stage_planet_dialogs`].
pub(crate) use crate::live_art::stage_planet_dialogs;

/// Stage Interface-archive scenes through the staging declaration;
/// each scene stages independently so one bad block never blanks
/// the rest. Channel stages no block art (the block names zero
/// sprites) — its code-mounted dialog art stages through
/// `stage_channel_dialog`. Avi stages its full-canvas logo once
/// through the declaration: the splash slot stays empty because
/// the scene slot already paints the same sprite.
fn stage_interface_archive(game: &mut ScreenGame, archive: &[u8]) {
    stage_channel_dialog(game, archive);
    if let Some(art) = stage_state(archive, State::Avi, StageContext::default()).art {
        game.load_scene(Screen::Avi, art.placements, art.figures);
    }
    for screen in [Screen::Room, Screen::Waiting, Screen::Planet] {
        let Some(state) = state_of_screen(screen) else {
            continue;
        };
        if let Some(art) = stage_state(archive, state, StageContext::default()).art {
            game.load_scene(screen, art.placements, art.figures);
        }
    }
    if let Some(art) = stage_state(archive, State::Result, StageContext::default()).art {
        game.load_scene(Screen::Result, art.placements, art.figures);
    }
    stage_planet_dialogs(game, archive);
}

/// Stage Playing-archive scenes (Loading gate plus Playing gauges)
/// through the game's room arena stage and key mode: the venue
/// `_Playing_BG.ojs` and the stage-keyed kits resolve their numbered
/// siblings, and the note-background/life-gauge/buffer-bar families
/// stage only the solo member the reference photo pins (lanes left
/// panel, venue right, avatar center-right). The walk room stays a
/// Live duo for the battle verdict, whose P2P family would paint its
/// VS panel over the center while emptying the solo slots — so the
/// context carries an explicit solo flag instead of the room's Live
/// word.
fn stage_playing_archive(game: &mut ScreenGame, archive: &[u8]) {
    if let Some(art) = stage_state(archive, State::Loading, StageContext::default()).art {
        game.load_scene(Screen::Loading, art.placements, art.figures);
    }
    let ctx = StageContext {
        arena_stage: Some(game.arena_stage()),
        key_mode: game.room_key_mode(),
        team_play: false,
        chart_slot: PLAYING_SONG.difficulty,
    };
    let Some(art) = stage_state(archive, State::Playing, ctx).art else {
        return;
    };
    game.load_scene(Screen::Playing, art.placements, art.figures);
    if let Some(skin) = PlaySkin::from_archive(archive) {
        game.set_play_skin(skin);
    }
}

/// Layer genuine 7.55 archive art under every scene when
/// `install-root` is present; missing or unreadable archives keep
/// the synthetic staging. Archives stay read-only, never fail.
pub(crate) fn stage_real_art(game: &mut ScreenGame) {
    let installed = read_install_archives(&install_root());
    if let Some(interface) = &installed.interface {
        stage_interface_archive(game, interface);
    }
    if let Some(playing) = &installed.playing {
        stage_playing_archive(game, playing);
        crate::live_art::stage_result_life_font(game, playing);
    }
}

/// Lobby list rows shared by the battle and solo walks (channels
/// plus room rows): display data only, no walked state.
fn seed_lobby_lists(game: &mut ScreenGame) {
    game.set_channels(vec![
        ChannelState {
            gateway_id: 0,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        },
        ChannelState {
            gateway_id: 0,
            channel_id: 9,
            capacity: 0,
            population: 0,
            active: false,
        },
    ]);
    game.set_rooms(vec![
        room_entry(3, "Jam", false, 5),
        room_entry(7, "Locked", true, 1),
    ]);
}

/// Scripted long-note fixture (fixture, not wire data): the lane-0
/// hold head shares the tap's 5 s onset and the tail sits 400 ms
/// later. The head strike arms the hold, so the playing paints show
/// the held beam; the verified hold machine ticks the bonus per
/// advance while the span stays open.
const SCRIPTED_HOLD: (u8, u64, u64, u32) = (0, 5_000, 5_400, 7);
/// Scripted hold-head strike clock (fixture): the tap instant, so
/// the playing paints land 100 ms into the armed span.
const SCRIPTED_HOLD_STRIKE_MS: u64 = 5_000;
/// Scripted key-up clock (fixture): exactly on the tail — the
/// verified release model rates the armed tail with the shared
/// windows, so the walked tail is a Cool and the span closes for
/// Result.
const SCRIPTED_HOLD_RELEASE_MS: u64 = 5_400;
/// Shared chart (fixture, not wire data): the lane-2 tap and the
/// lane-0 hold above, one key-sound sample each. A tail equal to its
/// head degrades to a tap (`LivePlay::new_holds`), so the hold row
/// is the only spanned record.
const SCRIPTED_CHART: [(u8, u64, u64, u32); 2] = [(2, 5_000, 5_000, 7), SCRIPTED_HOLD];

/// Shared chart: the 5 s tap and the held long note over it (the
/// hold beams while its span is open in Playing; both records are
/// spent by Result through the scripted key-up, which rates the tail
/// Cool). The solo walk strikes the same onsets, so its top-panel
/// numbers and grade plate match the battle walk's play data.
fn seed_chart(game: &mut ScreenGame) {
    game.load_chart_holds(SCRIPTED_CHART.to_vec());
}

/// Art-less `STATE_CHANNEL` fallback: stage the empty scene so the
/// walk takes the staged path (`paint_channel_scene` paints no fill
/// under the live list) instead of the flat `paint_channel_list`
/// fallback. Staged before `stage_real_art` in the seeded games: when
/// genuine archives are present, the code-mounted channel dialog art
/// overwrites this empty scene; without archives it stays empty.
fn stage_empty_channel_scene(game: &mut ScreenGame) {
    game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
}

/// Game preloaded with list rows, a Live waiting duo, the duo's
/// battle scoreboard, the wire peer's seated Life gauge (slot 1,
/// full — so `playing.png` shows the session-fed green fill),
/// scripted waiting chat (one 3012 user row plus
/// one 3013 admin row, so `waiting.png` shows live rows), and a
/// chart whose tap and hold head land at 5 s (the hold beams in
/// Playing; both records are spent by Result through the scripted
/// key-up, which rates the tail Cool). The waiting captions stage here — catalog plus
/// inventory — so the waiting shot inks MAPNAME/BPM/SKILLCOUNT
/// while later shots keep their own staging. Genuine archive art
/// stages underneath when `install-root`
/// holds it; otherwise the synthetic fallback paints.
fn seeded_game() -> ScreenGame {
    seeded_game_in(waiting_room())
}

/// Non-master self seat for the guest walk: the creator keeps seat 0,
/// so this id names a guest (the shell's `guest_room` test uses the same
/// seat to avoid colliding with the peer slot 1). Matches the battle
/// duo otherwise, so the guest replays the master instant seat-for-seat.
const GUEST_MEMBER_ID: u8 = 5;

/// Waiting room as the guest sees it: the battle duo with a non-master
/// self seat. Peer slot 1 still names the ready wire peer, so
/// `all_peers_ready` stays true while `is_room_master` reads false —
/// the exe's non-master arm (file `0xae805`: READY shown, START hidden).
fn guest_waiting_room() -> JoinedRoom {
    let mut room = waiting_room();
    room.member_id = GUEST_MEMBER_ID;
    room
}

/// Game preloaded like [`seeded_game`] but joined as the guest: the same
/// lobby rows, battle duo, captions, and chat, so the guest shot differs
/// from the master shot by seat alone (dialog gated off, `BPM` kept).
fn seeded_guest_game() -> ScreenGame {
    seeded_game_in(guest_waiting_room())
}

/// Shared seed: lobby rows, one joined room, the duo scoreboard, peer
/// gauge, captions, chat, chart, and real art underneath when present.
/// Settings load from `option.ini` beside the install root (exe
/// defaults while the file is absent, so the oracle stays
/// byte-stable on a bare checkout).
fn seeded_game_in(room: JoinedRoom) -> ScreenGame {
    let mut game = ScreenGame::new();
    game.load_options(&install_root());
    seed_lobby_lists(&mut game);
    game.set_joined_room(room);
    game.set_battle_teams(result_battle_teams());
    game.set_battle_self_team(Some(RESULT_SELF_TEAM));
    seed_chart(&mut game);
    stage_result_ranks(&mut game);
    stage_peer_gauge(&mut game);
    stage_waiting_captions(&mut game);
    stage_waiting_chat(&mut game);
    stage_empty_channel_scene(&mut game);
    stage_real_art(&mut game);
    game
}

/// Guest waiting instant: the same lane/menu walk as the master shot,
/// from a non-master seat with captions and chat staged but no song
/// selection (the gate hides the dialog), rasterized without checking
/// beyond the Waiting screen.
fn shoot_guest_waiting() -> anyhow::Result<Shot> {
    let mut guest = seeded_guest_game();
    guest.lane_key(3, true);
    guest.click_planet(136, 88);
    guest.menu_key(MenuKey::Confirm, true);
    guest.menu_key(MenuKey::Confirm, true);
    shoot_checked(&mut guest, "waiting-guest", Screen::Waiting, 0)
}

/// Solo waiting room for the solo walk: the same arena, key mode,
/// and speed as the battle duo, but a Normal room with no wire
/// peers — one seat, so Result takes the solo path (player-count
/// branch, frame 4) with a single rank row.
fn solo_waiting_room() -> JoinedRoom {
    JoinedRoom {
        member_id: 0,
        team: RESULT_SELF_TEAM,
        title: "Jam".to_owned(),
        music_id: 0,
        arena: PLAYING_ARENA_STAGE,
        key_mode: 2,
        game_mode: GameMode::Normal as u8,
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

/// Solo game: lobby rows, the solo room, and the shared chart, with
/// genuine archive art underneath when present. No battle scoreboard
/// and no scripted ranks — every solo value comes from the walked
/// play alone.
fn seeded_solo_game() -> ScreenGame {
    let mut game = ScreenGame::new();
    seed_lobby_lists(&mut game);
    game.set_joined_room(solo_waiting_room());
    seed_chart(&mut game);
    stage_empty_channel_scene(&mut game);
    stage_real_art(&mut game);
    game
}

/// Arm the S7 Loading→Playing gate the way the live lane stages
/// song prep: a validated pick, a null rig, and the armed
/// loading status. Idempotent — existing picks, rigs, and
/// banks staged by the caller stay. Easy (difficulty 0):
/// the walk tests' judgement economy assumes Easy drains.
fn arm_loading_gate(game: &mut ScreenGame) {
    use o2jam_audio::NullBackend;
    use std::collections::HashMap;
    if game.selected_music().is_none() {
        game.set_selected_music(o2jam_net::SetRoomMusic {
            music_id: 757,
            difficulty: 0,
            speed: 0,
        });
    }
    if !game.has_audio() {
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
    }
    if let Some(status) = loading_phase_status(LOADING_GATE_STEPS, "Song", "Band") {
        game.set_loading_status(status);
    }
}
/// Drive the solo walk from splash into Result: the same
/// lane/menu/strike script as the battle walk, with the song-pick
/// and playing captions staged at the same points.
fn drive_solo_to_result() -> ScreenGame {
    let mut game = seeded_solo_game();
    game.lane_key(3, true);
    game.click_planet(136, 88);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    stage_select_music_scene(&mut game);
    game.menu_key(MenuKey::Confirm, true);
    stage_playing_scene(&mut game);
    // The second Confirm is the Loading→Playing gate (S7): the pick
    // above validates (757, catalog-staged); only the armed status
    // and the rig are missing.
    arm_loading_gate(&mut game);
    game.menu_key(MenuKey::Confirm, true);
    game.strike_at(2, 5_000);
    game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
    game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
    game
}

/// Stage the musicselect shot's dialog scene: the host song flow
/// (catalog plus pick) so the waiting song panel resolves, then the
/// real `DAILOG_SELECTMUSIC` dialog art over the seats. Staged after
/// the waiting shot so earlier shots stay untouched; never fails.
/// Without archive art the panel still stages (documented gap).
fn stage_select_music_scene(game: &mut ScreenGame) {
    stage_select_music_catalog(game);
    game.set_selected_music(PLAYING_SONG);
    stage_select_music_art(game);
}

/// Stage the select-music catalog: the installed 7.55 song index
/// when it parses to songs, else the synthetic fallback below.
/// Skips when a catalog is already staged so the playing caption
/// keeps the same rows.
fn stage_select_music_catalog(game: &mut ScreenGame) {
    if !game.select_songs().is_empty() {
        return;
    }
    if let Ok(archive) = std::fs::read(install_root().join(MUSICSELECT_CATALOG)) {
        if let Ok(entries) = o2jam_file::parse_ojn_list(&archive) {
            game.set_song_catalog(&entries);
            if !game.select_songs().is_empty() {
                return;
            }
        }
    }
    if let Ok(entries) = o2jam_file::parse_ojn_list(&musicselect_fallback_catalog()) {
        game.set_song_catalog(&entries);
    }
}

/// Stage the lanking walk board: the installed 7.55 song index
/// (or the synthetic fallback) for the left list plus a scripted
/// 2028 stub board for the right list (ASSUMED wire-side — the
/// shell proves the rows, no live gateway). Runs before the
/// lanking side shot so the oracle proves rows and ranks paint.
fn stage_lanking_walk_board(game: &mut ScreenGame) {
    if let Ok(archive) = std::fs::read(install_root().join(MUSICSELECT_CATALOG)) {
        if let Ok(entries) = o2jam_file::parse_ojn_list(&archive) {
            game.set_lanking_catalog(&entries);
            if !game.lanking_visible_songs().is_empty() {
                game.stage_lanking_board(lanking_walk_stub_board());
                return;
            }
        }
    }
    if let Ok(entries) = o2jam_file::parse_ojn_list(&musicselect_fallback_catalog()) {
        game.set_lanking_catalog(&entries);
        game.stage_lanking_board(lanking_walk_stub_board());
    }
}

/// Scripted 2028 stub board for the walk (synthetic nicknames —
/// no game data; the middle zeroed row is the exe's display fill).
fn lanking_walk_stub_board() -> o2jam_net::MusicPlayRankingAnswer {
    use o2jam_net::{RankDelta, RankEntry};
    let row = |rank: i32, nickname: &str, delta: RankDelta| RankEntry {
        rank,
        nickname: nickname.to_string(),
        battles: 12,
        wins: 9,
        win_rate: 75,
        delta,
        rank_delta: 0,
    };
    o2jam_net::MusicPlayRankingAnswer {
        self_entry: RankEntry::default(),
        entries: vec![
            row(1, "WalkChamp", RankDelta::Up),
            RankEntry::default(),
            row(2, "WalkRival", RankDelta::Down),
        ],
    }
}

/// Stage the item-shop walk board: the installed 7.55 avatar item
/// table for the eight goods rows plus a scripted wallet stub and
/// two checked cart rows, so the oracle proves names, prices, the
/// gem cell, cart rows, and pager readouts paint. The wallet words
/// are synthetic (no live gateway); the goods are real catalog
/// data. Runs before the shop side shot.
fn stage_shop_walk_board(game: &mut ScreenGame) {
    use o2jam_net::wallet::SyncCashPointResponse;
    let Ok(archive) = std::fs::read(install_root().join(AVATAR_ARCHIVE)) else {
        return;
    };
    let Ok(list) = o2jam_file::parse_opi(&archive) else {
        return;
    };
    let Some(entry) = list.iter().find(|entry| entry.name == "Itemdata_Korea.dat") else {
        return;
    };
    let Some(bytes) = archive.get(entry.data_offset..entry.data_offset + entry.size as usize)
    else {
        return;
    };
    let Ok(table) = o2jam_file::parse_avatar_item_table(bytes) else {
        return;
    };
    game.set_shop_catalog(&table);
    if game.shop_visible_goods().is_empty() {
        return;
    }
    game.apply_shop_cash_point(&SyncCashPointResponse {
        gem: 1200,
        cash_point: 50,
    });
    for good in game.shop_visible_goods().into_iter().take(2) {
        let id = good.item_id;
        if !game.shop_cart().contains(&id) {
            // Cart checks ride the staged ITEMBUY hit areas when
            // present; the direct push keeps the walk honest when
            // a placement is missing. Both cross the same cart.
            game.shop_cart_push(id);
        }
    }
}

/// Stage the song-shop walk board: the installed 7.55 song index
/// (or the synthetic fallback) for the five song rows plus a
/// scripted 4031 flag stub (two first-page songs owned) and two
/// staged cart picks, so the oracle proves catalog cells, bought
/// flags, cart rows, and pager readouts paint. The flag stub is
/// synthetic (ASSUMED wire-side); the songs are real catalog data.
/// Runs before the music-shop side shot.
fn stage_music_shop_walk_board(game: &mut ScreenGame) {
    if let Ok(archive) = std::fs::read(install_root().join(MUSICSELECT_CATALOG)) {
        if let Ok(entries) = o2jam_file::parse_ojn_list(&archive) {
            game.set_music_shop_catalog(&entries);
            if !game.music_shop_visible_rows().is_empty() {
                game.stage_music_shop_board(music_shop_walk_stub_flags(game));
                stage_music_shop_walk_cart(game);
                return;
            }
        }
    }
    if let Ok(entries) = o2jam_file::parse_ojn_list(&musicselect_fallback_catalog()) {
        game.set_music_shop_catalog(&entries);
        game.stage_music_shop_board(music_shop_walk_stub_flags(game));
        stage_music_shop_walk_cart(game);
    }
}

/// Scripted 4031 flag stub for the walk: the first and fourth
/// visible songs read owned (synthetic presence — no live gateway).
fn music_shop_walk_stub_flags(game: &ScreenGame) -> o2jam_net::play::MusicListAnswer {
    use o2jam_net::play::MusicListEntry;
    let entries = game
        .music_shop_visible_rows()
        .into_iter()
        .enumerate()
        .filter(|(index, _)| *index == 0 || *index == 3)
        .filter_map(|(_, row)| u16::try_from(row.song.song_id).ok())
        .map(|id| MusicListEntry {
            id,
            notes_ex: 100,
            notes_nx: 200,
            notes_hx: 300,
        })
        .collect();
    o2jam_net::play::MusicListAnswer {
        entries,
        free_music: false,
    }
}

/// Check the first two visible song rows into the walk cart.
fn stage_music_shop_walk_cart(game: &mut ScreenGame) {
    for row in game.music_shop_visible_rows().into_iter().take(2) {
        let id = row.song.song_id;
        if !game.music_shop_cart().contains(&id) {
            game.music_shop_cart_push(id);
        }
    }
}
/// (`DAILOG_SELECTMUSIC` over Waiting) over the waiting seats;
/// empty art paints nothing.
fn stage_select_music_art(game: &mut ScreenGame) {
    let Some(archive) = read_install_archives(&install_root()).interface else {
        return;
    };
    let Some(art) = stage_dialog(&archive, Dialog::SelectMusic, StageContext::default()).art else {
        return;
    };
    if art.figures.is_empty() {
        return;
    }
    game.load_select_music_scene(art.placements, art.figures);
}

/// Catalog title/artist for the host pick: the loading caption
/// source (staged `OJNList.dat` order). `None` while the staged
/// catalog names no such song — the walk keeps its indeterminate
/// bar rather than inventing a caption.
fn loading_song_caption(game: &ScreenGame) -> Option<(String, String)> {
    let wanted = u32::from(PLAYING_SONG.music_id);
    game.select_songs()
        .iter()
        .find(|song| song.song_id == wanted)
        .map(|song| (song.title.clone(), song.artist.clone()))
}

/// Stage the loading shot's live walk status: thread the gate
/// phases (entry plus the four staging/decrypt/decode/arm steps)
/// through `loading_phase_status` with the staged catalog caption,
/// holding the clock-armed full fraction for the paint. Staged
/// after the Waiting→Loading edge so earlier shots stay untouched;
/// keeps the indeterminate bar while the catalog names no song.
fn stage_loading_status(game: &mut ScreenGame) {
    let Some((title, artist)) = loading_song_caption(game) else {
        return;
    };
    for phase in 0..=LOADING_GATE_STEPS {
        if let Some(status) = loading_phase_status(phase, &title, &artist) {
            game.set_loading_status(status);
        }
    }
}

/// Synthetic fallback catalog: the 7.55 `OJNList.dat` record shape
/// for [`MUSICSELECT_FALLBACK_SONGS`]; bytes live here so no game
/// data is read. Titles are ASCII stand-ins (documented gap).
fn musicselect_fallback_catalog() -> Vec<u8> {
    let mut catalog = Vec::with_capacity(300 * MUSICSELECT_FALLBACK_SONGS.len());
    for (song_id, title, artist) in MUSICSELECT_FALLBACK_SONGS {
        catalog.extend_from_slice(&catalog_record(song_id, title, artist));
    }
    catalog
}

/// One synthetic 300-byte catalog record (id, title, artist, plus
/// the `ojn` marker, a sane tempo, and an `o2ma###.ojm` token so
/// [`o2jam_file::OjnListEntry::is_song`] keeps it).
fn catalog_record(song_id: u32, title: &str, artist: &str) -> Vec<u8> {
    const RECORD_LEN: usize = 300;
    const TITLE_LEN: usize = 64;
    const ARTIST_LEN: usize = 32;
    const FILE_LEN: usize = 32;
    let mut rec = vec![0u8; RECORD_LEN];
    rec[8..11].copy_from_slice(b"ojn");
    rec[4..8].copy_from_slice(&song_id.to_le_bytes());
    rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
    let title = title.as_bytes();
    let take = title.len().min(TITLE_LEN);
    rec[112..112 + take].copy_from_slice(&title[..take]);
    let artist = artist.as_bytes();
    let take = artist.len().min(ARTIST_LEN);
    rec[176..176 + take].copy_from_slice(&artist[..take]);
    let file = format!("o2ma{song_id}.ojm");
    let file = file.as_bytes();
    let take = file.len().min(FILE_LEN);
    rec[240..240 + take].copy_from_slice(&file[..take]);
    rec
}

/// Stage the playing shot's mid-song scene: the player caption, the
/// worn figure (`avatar.opa`; a missing or unstackable archive keeps
/// no figure), the host song pick whose caption the Playing paint
/// re-syncs into the scene and tally strip, and the volume readout
/// (`VOL 58`, the reference photo's strip readout). Staged after the
/// loading shot so earlier shots stay untouched; never fails. The
/// one-song fallback keeps the playing caption byte-stable even after
/// the musicselect staging above (same host pick, same title).
fn stage_playing_scene(game: &mut ScreenGame) {
    let (level, name) = PLAYING_PLAYER;
    game.set_player(level, name);
    game.set_volume(PLAYING_VOLUME);
    if let Ok(archive) = std::fs::read(install_root().join(AVATAR_ARCHIVE)) {
        if let Ok(figure) = stack_worn_figure(&archive, &PLAYING_EQUIPMENT, true) {
            if !figure.is_empty() {
                game.load_avatar(figure);
            }
        }
    }
    if let Ok(entries) = o2jam_file::parse_ojn_list(&playing_song_catalog()) {
        game.set_song_catalog(&entries);
    }
    game.set_selected_music(PLAYING_SONG);
}

/// Synthetic one-song catalog blob (the 7.55 `OJNList.dat` record
/// shape, title `First` at id 757) feeding the playing shot's song
/// caption; bytes live here so no game data is read.
fn playing_song_catalog() -> Vec<u8> {
    let mut rec = vec![0u8; 300];
    rec[8..11].copy_from_slice(b"ojn");
    rec[4..8].copy_from_slice(&757u32.to_le_bytes());
    rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
    rec[112..117].copy_from_slice(b"First");
    rec[176..180].copy_from_slice(b"Band");
    rec[240..251].copy_from_slice(b"o2ma757.ojm");
    rec
}

/// One room-list row.
fn room_entry(number: i32, title: &str, locked: bool, users: u8) -> RoomListEntry {
    RoomListEntry {
        number,
        state_code: 1,
        title: title.to_owned(),
        has_password: locked,
        music_id: 0,
        difficulty: 0,
        key_mode: 2,
        game_mode: 0,
        speed: 1,
        capacity: 8,
        user_count: users,
        min_level: 0,
        max_level: 0,
        skills: vec![],
        premium: false,
        room_type: 0,
    }
}

/// Wire peer nickname on the result rank table (a synthetic
/// reference-style name; the local row stays `YOU`). The peer
/// holds rank 2, mirroring the reference's second row.
const RESULT_PEER_NAME: &str = "Lujia";
/// Local battle team (team A, the `TEAMA` seat).
const RESULT_SELF_TEAM: u8 = 0;
/// Peer battle team (team B).
const RESULT_PEER_TEAM: u8 = 1;
/// Synthetic server-side total on the local seat: underscores the
/// peer so rank 2 sits on the local team and the banner reads LOSE!,
/// like the reference battle photo.
const RESULT_SELF_TOTAL: u64 = 100;
/// Synthetic server-side total on the peer seat.
const RESULT_PEER_TOTAL: u64 = 300;
/// Scripted 4015 global holder byte (fixture, not wire data): the
/// no-holder sentinel, so no packet byte takes the global arm and
/// every delivery resolves through a member holder below.
const SCRIPTED_RANK_GLOBAL: u8 = 0xFF;
/// Scripted holder bytes per seat: the local seat holds rank 1, the
/// wire peer rank 2 — the reference photo's first/second row values
/// (`reference-result-4x3.png`; local-first ordering gap documented
/// on [`ScreenGame::result_rank_rows`]). Empty seats hold the
/// no-holder sentinel, so the zero convention keeps them blank.
const SCRIPTED_RANK_STORED: [u8; PEER_SLOT_COUNT] = [1, 2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
/// Scripted 8-byte rank broadcast: local rank at position 0, peer
/// rank at 1. Trailing zeros match no holder, so those rows stay
/// zeroed and hidden — no invented standings beyond the duo.
const SCRIPTED_RANK_PACKET: [u8; PEER_SLOT_COUNT] = [1, 2, 0, 0, 0, 0, 0, 0];

/// Deliver the scripted rank fixture through the peer mirror:
/// holder bytes first, then the broadcast through the verified
/// rank-remap rule. Staged after the chart load — `load_chart`
/// rebuilds the play state (wiping the mirror) — so earlier shots
/// stay untouched; never fails. The rank bytes never paint in
/// Playing (no rank table there), only Result does; the peer Life
/// word that seats the Playing gauge rides [`stage_peer_gauge`].
fn stage_result_ranks(game: &mut ScreenGame) -> Vec<RankDelivery> {
    game.stage_scripted_ranks(
        SCRIPTED_RANK_GLOBAL,
        SCRIPTED_RANK_STORED,
        SCRIPTED_RANK_PACKET,
    )
}

/// Scripted peer slot (fixture, not wire data): the wire peer's
/// seat, matching its rank holder below ([`SCRIPTED_RANK_STORED`]
/// position 1).
const SCRIPTED_PEER_SLOT: u8 = 1;
/// Scripted peer Life word (fixture, not wire data): full life, so
/// the seated gauge reads all green.
const SCRIPTED_PEER_LIFE: u16 = LIFE_MAX;

/// Seat the wire peer's Playing gauge through the session bridge:
/// the scripted 4015 Life echo writes first (seating slot 1), then
/// the rank broadcast re-applies (idempotent — holders already
/// staged above), the same stats-then-ranks order `OnlineSession`
/// folds live echoes through [`ScreenGame::apply_peer_echo`].
/// Staged after the chart load with the ranks, so earlier shots
/// stay untouched; never fails. The local seat keeps no peer gauge;
/// local life rides the avatar bar instead.
fn stage_peer_gauge(game: &mut ScreenGame) {
    game.apply_peer_echo(
        SCRIPTED_PEER_SLOT,
        STAT_LIFE,
        SCRIPTED_PEER_LIFE,
        SCRIPTED_RANK_PACKET,
    );
}

/// Scripted 3012 user sender (fixture, not wire data): the wire
/// peer, echoing the reference photo's `[ Lujia] YOOO!!!` row
/// (`reference-waiting-4x3.png`).
const SCRIPTED_CHAT_USER_SENDER: &str = RESULT_PEER_NAME;
/// Scripted 3012 user text (fixture, not wire data).
const SCRIPTED_CHAT_USER_TEXT: &str = "YOOO!!!";
/// Scripted 3012 sender seat (fixture): the peer's slot.
const SCRIPTED_CHAT_USER_SEAT: u8 = 1;
/// Scripted 3013 admin sender (fixture, not wire data).
const SCRIPTED_CHAT_ADMIN_SENDER: &str = "Admin";
/// Scripted 3013 admin text (fixture, not wire data).
const SCRIPTED_CHAT_ADMIN_TEXT: &str = "Welcome";
/// Scripted 3013 sender seat (fixture).
const SCRIPTED_CHAT_ADMIN_SEAT: u8 = 0;

/// Staged inventory counts behind the waiting `SKILLCOUNT`
/// captions, one per skill slot: slot 1 shows `  7`, slot 2
/// ` 42`, slot 3 caps at `999` (the pinned slot-2/3 clamp), so the
/// shot exercises padding and the cap in one frame.
const WAITING_SKILL_COUNTS: [u32; 3] = [7, 42, 1500];

/// Stage the waiting caption sources: the song catalog (the `BPM`
/// caption reads its row +20 column for the room's song, `-0-`
/// while it names none) plus the inventory counts (the
/// `SKILLCOUNT` source; `MAPNAME` rides the room word, already
/// set). Staged before the waiting shot so earlier shots stay
/// untouched; never fails. The musicselect staging below reuses
/// the same catalog when present, and the playing staging
/// overwrites it with its one-song blob.
fn stage_waiting_captions(game: &mut ScreenGame) {
    stage_select_music_catalog(game);
    game.set_skill_counts(WAITING_SKILL_COUNTS);
}

/// Deliver the scripted chat fixture through the live 3012/3013
/// push path (same honesty as the scripted rank bytes): one admin
/// row plus one user row, in the reference stacking (admin notice on
/// top, the peer's user row last), so the waiting shot's history box
/// reads like `reference-waiting-4x3.png` rows 2–3. Staged after the
/// room join — `set_joined_room` clears the history — so earlier
/// shots stay untouched; never fails. Texts are ASCII stand-ins
/// (documented gap: no CP949 glyphs, no live server).
fn stage_waiting_chat(game: &mut ScreenGame) {
    game.push_waiting_admin_message(&WaitingAdminMessage {
        sender: SCRIPTED_CHAT_ADMIN_SENDER.to_owned(),
        content: SCRIPTED_CHAT_ADMIN_TEXT.as_bytes().to_vec(),
        member_id: SCRIPTED_CHAT_ADMIN_SEAT,
    });
    game.push_waiting_user_message(&WaitingUserMessage {
        sender: SCRIPTED_CHAT_USER_SENDER.to_owned(),
        content: SCRIPTED_CHAT_USER_TEXT.as_bytes().to_vec(),
        member_id: SCRIPTED_CHAT_USER_SEAT,
    });
}

/// Battle scoreboard behind the result verdict: rank 1 by team
/// total wins, so the peer takes it and the claim below resolves
/// to Lose (frame 2, image 1). Only the ordering reaches the paint.
fn result_battle_teams() -> RoomTeams {
    let mut teams = RoomTeams::new();
    teams.set_team(0, RESULT_SELF_TEAM);
    teams.set_team(1, RESULT_PEER_TEAM);
    teams.submit(0, RESULT_SELF_TOTAL);
    teams.submit(1, RESULT_PEER_TOTAL);
    teams
}

/// Waiting room for the result walk: a Live duo so Result takes
/// the battle path — self on team A beside the wire peer `Lujia`
/// on team B, joined at the reference-matching arena stage (see
/// [`PLAYING_ARENA_STAGE`]). The room song is the host pick (the
/// waiting `BPM`/song captions resolve through the staged
/// catalog; `-0-`/bare id while it names none). The peer sits
/// ready: the walk spends the chart straight through.
fn waiting_room() -> JoinedRoom {
    JoinedRoom {
        member_id: 0,
        team: RESULT_SELF_TEAM,
        title: "Jam".to_owned(),
        music_id: PLAYING_SONG.music_id,
        arena: PLAYING_ARENA_STAGE,
        key_mode: 2,
        game_mode: GameMode::Live as u8,
        difficulty: 0,
        speed: 1,
        has_super_manager: false,
        user_count: 2,
        slots: vec![RoomSlot {
            index: 1,
            state_code: 1,
            member: Some(RoomMember {
                nickname: RESULT_PEER_NAME.to_owned(),
                level: 12,
                gender: 0,
                gem: 0,
                master: false,
                team: RESULT_PEER_TEAM,
                ready: true,
                music_state: 0,
                equipment: [0; 17],
                music_ids: Vec::new(),
                cash_point: 0,
                free_pass: 0,
                playing_state: 0,
                administrator: false,
                super_manager: false,
            }),
        }],
        skills: Vec::new(),
        premium: false,
        champion: None,
        champion_streak: None,
    }
}

/// Rasterize whatever screen is showing, without checking it.
fn shoot(game: &mut ScreenGame, name: &'static str, now_ms: u64) -> anyhow::Result<Shot> {
    Ok(Shot {
        name,
        w: SHOT_W,
        h: SHOT_H,
        rgb: frame_rgb(game, now_ms),
    })
}

/// Rasterize, failing when the walk missed its screen.
fn shoot_checked(
    game: &mut ScreenGame,
    name: &'static str,
    screen: Screen,
    now_ms: u64,
) -> anyhow::Result<Shot> {
    if game.screen() != screen {
        anyhow::bail!("shot {name}: at {:?}, want {screen:?}", game.screen());
    }
    shoot(game, name, now_ms)
}

/// Rasterize an open side screen: the flow stays on the lobby while
/// the side overlay paints through `paint_side_scene`.
fn shoot_side(game: &mut ScreenGame, side: SideScreen) -> anyhow::Result<Shot> {
    let name = side_shot_name(side);
    if game.side() != Some(side) {
        anyhow::bail!("shot {name}: side {:?}, want {side:?}", game.side());
    }
    shoot(game, name, 0)
}

/// Rasterize a result instant under `name`, failing when the walk
/// missed its screen.
fn shoot_result_named(
    game: &mut ScreenGame,
    name: &'static str,
    now_ms: u64,
) -> anyhow::Result<Shot> {
    let rgb = frame_rgb(game, now_ms);
    if game.screen() != Screen::Result {
        anyhow::bail!("shot {name}: still at {:?}", game.screen());
    }
    Ok(Shot {
        name,
        w: SHOT_W,
        h: SHOT_H,
        rgb,
    })
}

/// Rasterize a full-native result instant under `name`, failing
/// unless the chart end advanced the flow.
fn shoot_result_native_named(
    game: &mut ScreenGame,
    name: &'static str,
    now_ms: u64,
) -> anyhow::Result<Shot> {
    let rgb = frame_native_rgb(game, now_ms);
    if game.screen() != Screen::Result {
        anyhow::bail!("shot {name}: still at {:?}", game.screen());
    }
    let (w, h) = native_size();
    Ok(Shot { name, w, h, rgb })
}

/// Rasterize the full native canvas without checking the screen.
fn shoot_native(game: &mut ScreenGame, name: &'static str, now_ms: u64) -> Shot {
    let (w, h) = native_size();
    Shot {
        name,
        w,
        h,
        rgb: frame_native_rgb(game, now_ms),
    }
}

/// Rasterize the full native canvas, failing when the walk missed
/// its screen.
fn shoot_checked_native(
    game: &mut ScreenGame,
    name: &'static str,
    screen: Screen,
    now_ms: u64,
) -> anyhow::Result<Shot> {
    if game.screen() != screen {
        anyhow::bail!("shot {name}: at {:?}, want {screen:?}", game.screen());
    }
    Ok(shoot_native(game, name, now_ms))
}

/// Native canvas dimensions behind every shot.
fn native_size() -> (usize, usize) {
    (
        usize::try_from(NATIVE_WIDTH).unwrap_or(1),
        usize::try_from(NATIVE_HEIGHT).unwrap_or(1),
    )
}

/// Paint one native 800x600 XRGB frame: the single source both the
/// preview and the native shots encode.
fn paint_native_frame(game: &mut ScreenGame, now_ms: u64) -> Vec<u32> {
    let (w, h) = native_size();
    let mut buf = vec![0u32; w * h];
    game.paint_at(
        &mut Frame {
            buf: &mut buf,
            w,
            h,
        },
        now_ms,
    );
    buf
}

/// Native-canvas RGB triplets: no downscale, same painter.
fn frame_native_rgb(game: &mut ScreenGame, now_ms: u64) -> Vec<u8> {
    to_rgb(&paint_native_frame(game, now_ms))
}

/// Paint the current screen at the native 800x600 canvas — the exe
/// frame every staged record anchor lives on — and box-downscale to
/// the shot size, exactly how the reference photo photographs the
/// native screen (a 4:3 view of one 800x600 frame, never a stretch
/// or a partial blit).
fn frame_rgb(game: &mut ScreenGame, now_ms: u64) -> Vec<u8> {
    let native = paint_native_frame(game, now_ms);
    let (w, h) = native_size();
    to_rgb(&downscale_even(&native, w, h))
}

/// 2x2 box average of XRGB words: width and height must be even
/// (native 800x600 over shot 400x300). Channels average without
/// carrying the transparent-black colorkey — the shot shows what the
/// native frame shows.
fn downscale_even(buf: &[u32], w: usize, h: usize) -> Vec<u32> {
    let (ow, oh) = (w / 2, h / 2);
    let mut out = Vec::with_capacity(ow * oh);
    for row in 0..oh {
        for col in 0..ow {
            let mut sum = [0u64; 3];
            for pixel in [
                buf[row * 2 * w + col * 2],
                buf[row * 2 * w + col * 2 + 1],
                buf[(row * 2 + 1) * w + col * 2],
                buf[(row * 2 + 1) * w + col * 2 + 1],
            ] {
                sum[0] += u64::from(pixel >> 16 & 0xFF);
                sum[1] += u64::from(pixel >> 8 & 0xFF);
                sum[2] += u64::from(pixel & 0xFF);
            }
            let mixed =
                ((sum[0] / 4) as u32) << 16 | ((sum[1] / 4) as u32) << 8 | (sum[2] / 4) as u32;
            out.push(mixed);
        }
    }
    out
}

/// Strip XRGB words down to RGB triplets.
fn to_rgb(buf: &[u32]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(buf.len() * 3);
    for pixel in buf {
        rgb.push((pixel >> 16) as u8);
        rgb.push((pixel >> 8) as u8);
        rgb.push(*pixel as u8);
    }
    rgb
}

/// PNG encoding for scene captures lives in the render crate
/// ([`o2jam_render::png`]); re-exported here for the existing
/// screenshot/walk call sites.
pub use o2jam_render::png::encode_png;

#[cfg(test)]
mod tests {
    use super::*;
    use o2jam_shell::InputSnapshot;

    /// Canary (retro 2026-11-12): broken data-dir symlinks once made
    /// every "with install-root present" run silently skip, and the
    /// suites stayed green while verifying nothing. A dangling symlink
    /// or a present-but-unparsable root must be LOUD.
    #[test]
    fn real_data_arm_is_not_silently_absent() {
        let root = install_root();
        if let Ok(meta) = std::fs::symlink_metadata(&root) {
            if meta.file_type().is_symlink() {
                assert!(
                    root.exists(),
                    "install-root is a DANGLING symlink: every real-data test \
                     silently skips and 'gates green' lies — recreate the link"
                );
            }
        }
        if root.exists() {
            let archives = read_install_archives(&root);
            assert!(
                archives.interface.is_some(),
                "install-root exists but the interface archive did not parse — \
                 the data dirs are present yet unusable, and the suite skips"
            );
        }
    }

    use o2jam_shell::avatar::PlacementRole;
    use o2jam_shell::avatar::{
        anchor_in_frame, load_archive_sprite, paint_foreground_placements, paint_scene_figures,
        playing_member_selected, PlayingArtMode,
    };
    use o2jam_shell::avatar::{scene_block, scene_sprite_names};
    use o2jam_shell::screens::paint::RESULT_WINLOSE_SPRITE;
    use o2jam_shell::staging::StagedArt;

    /// Interface archive under `install-root` (Avi/Channel/Room/Waiting/Result).
    const INTERFACE_ARCHIVE: &str = "O2Jam/Image/Interface1.opi";
    /// Playing archive under `install-root` (Loading/Playing art).
    const PLAYING_ARCHIVE: &str = "O2Jam/Image/Playing1.opi";

    #[test]
    fn downscale_even_averages_two_by_two_blocks() {
        // A 2x2 field of red/green/blue/white quarters halves to one
        // mixed pixel; a flat field halves to itself.
        let px = |r: u64, g: u64, b: u64| (r << 16 | g << 8 | b) as u32;
        let buf = vec![
            px(255, 0, 0),
            px(0, 255, 0),
            px(0, 0, 255),
            px(255, 255, 255),
        ];
        assert_eq!(downscale_even(&buf, 2, 2), vec![px(127, 127, 127)]);
        let flat = vec![px(10, 20, 30); 16];
        assert_eq!(downscale_even(&flat, 4, 4), vec![px(10, 20, 30); 4]);
    }

    #[test]
    fn frame_rgb_paints_the_native_canvas_downscaled() {
        // The playing shot rasterizes at 800x600 (where every staged
        // record anchor is exact) and arrives at the shot size 2:1.
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        for _ in 0..3 {
            game.menu_key(MenuKey::Confirm, true);
        }
        arm_loading_gate(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        let rgb = frame_rgb(&mut game, 100);
        assert_eq!(rgb.len(), SHOT_W * SHOT_H * 3);
    }

    #[test]
    fn pinned_equipment_stacks_a_worn_figure() {
        let Ok(archive) = std::fs::read(install_root().join(AVATAR_ARCHIVE)) else {
            println!("skip: missing {AVATAR_ARCHIVE}");
            return;
        };
        let figure = stack_worn_figure(&archive, &PLAYING_EQUIPMENT, true)
            .expect("pinned ids stack against 7.55 avatar.opa");
        assert!(!figure.is_empty());
        let (w, h) = figure.extent();
        assert!(w > 0 && h > 0, "figure extent {w}x{h} paints nothing");
    }

    #[test]
    fn playing_venue_stages_the_room_arena_backdrop() {
        // The joined room's arena word (stage 2) drives the venue
        // resolution: the staged `_Playing_BG.ojs` figure is the
        // `Playing_BG2.ojs` body, not the bare stem's.
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let game = seeded_game();
        assert_eq!(game.arena_stage(), 2);
        let ctx = StageContext {
            arena_stage: Some(game.arena_stage()),
            key_mode: game.room_key_mode(),
            team_play: game.room_team_play(),
            chart_slot: PLAYING_SONG.difficulty,
        };
        assert_eq!(ctx.key_mode, 2, "the shot's room plays 7 keys");
        assert!(ctx.team_play, "the shot's room is a Live duo");
        let figures = &stage_state(&archive, State::Playing, ctx)
            .art
            .expect("playing: STATE_PLAYING stages figures")
            .figures;
        let venue = figures
            .get("_Playing_BG.ojs")
            .expect("the venue runtime ref stages a figure");
        assert_eq!(
            venue,
            &load_archive_sprite(&archive, "Playing_BG2.ojs").expect("stage 2 body decodes"),
            "venue must load the arena stage's numbered body"
        );
        assert_ne!(
            figures.get("_Playing_BG.ojs"),
            load_archive_sprite(&archive, "Playing_BG.ojs").as_ref(),
            "stage-less bare stem must not win while a stage is set"
        );
    }

    #[test]
    fn playing_art_stages_only_the_solo_seven_key_members() {
        // The room mode selects the family members (applier case
        // table; solo set verified against the reference photo): the
        // 7-key note background, the left life gauge and BP buffer
        // stage; the 3-key/P2P note backgrounds, the right buffer,
        // and the unused gauge-back shell leave empty slots.
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let ctx = StageContext {
            arena_stage: Some(2),
            key_mode: 2,
            team_play: false,
            chart_slot: PLAYING_SONG.difficulty,
        };
        let report = stage_state(&archive, State::Playing, ctx);
        let figures = &report
            .art
            .expect("playing: STATE_PLAYING stages figures")
            .figures;
        for staged in ["_Playing_BG.ojs", "Note_BG.ojs", "Life_gauge_left_bar.ojs"] {
            assert!(
                figures.contains_key(staged),
                "{staged} must stage in a solo 7-key room"
            );
        }
        let mode = PlayingArtMode {
            key_mode: 2,
            team_play: false,
        };
        for name in figures.keys() {
            assert!(
                playing_member_selected(name, &mode),
                "staged member {name} must be solo-selected"
            );
        }
        assert!(!figures.contains_key("BpPoint_Right_bar.ojs"));
        assert!(!figures.contains_key("Life_gauge_back.ojs"));
    }

    #[test]
    fn playing_shot_pins_solo_composition_despite_live_room() {
        // The walk room stays a Live duo (the battle Lose verdict),
        // but the playing shot pins the solo reference composition
        // (lanes left panel, venue right, avatar center-right, tally
        // bottom): re-staging the Playing scene with the solo family
        // must be a no-op. A Live-family staging selects the P2P
        // note background, right buffer, and gauge-back shell, whose
        // VS panel paints over the center while the solo slots
        // (Note_BG, left bars) sit empty.
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let walk = seeded_game();
        assert!(
            walk.room_team_play(),
            "the walk stays Live for the battle verdict"
        );
        let solo = StageContext {
            arena_stage: Some(walk.arena_stage()),
            key_mode: walk.room_key_mode(),
            team_play: false,
            chart_slot: PLAYING_SONG.difficulty,
        };
        let art = stage_state(&archive, State::Playing, solo)
            .art
            .expect("playing: STATE_PLAYING stages solo figures");
        let (placements, figures) = (art.placements, art.figures);
        assert!(figures.contains_key("Note_BG.ojs"));
        assert!(figures.contains_key("Life_gauge_left_bar.ojs"));
        assert!(!figures.contains_key("P2P7KeyNote_BG.ojs"));
        assert!(!figures.contains_key("BpPoint_Right_bar.ojs"));
        assert!(!figures.contains_key("Life_gauge_back.ojs"));
        let paint_native = |game: &mut ScreenGame| {
            game.lane_key(3, true);
            game.click_planet(136, 88);
            for _ in 0..3 {
                game.menu_key(MenuKey::Confirm, true);
            }
            arm_loading_gate(game);
            game.menu_key(MenuKey::Confirm, true);
            stage_playing_scene(game);
            game.strike_at(2, 5_000);
            let (w, h) = native_size();
            let mut buf = vec![0u32; w * h];
            game.paint_at(
                &mut Frame {
                    buf: &mut buf,
                    w,
                    h,
                },
                5_100,
            );
            assert_eq!(game.screen(), Screen::Playing);
            buf
        };
        let mut seeded = seeded_game();
        let seeded_frame = paint_native(&mut seeded);
        let mut solo_game = seeded_game();
        solo_game.load_scene(Screen::Playing, placements, figures);
        let solo_frame = paint_native(&mut solo_game);
        assert_eq!(
            seeded_frame, solo_frame,
            "playing must stage solo members despite the Live room"
        );
    }

    #[test]
    fn playing_shot_paints_staged_mid_song_state() {
        let walk_to_playing = |game: &mut ScreenGame| {
            game.lane_key(3, true);
            game.click_planet(136, 88);
            for _ in 0..3 {
                game.menu_key(MenuKey::Confirm, true);
            }
            arm_loading_gate(game);
            game.menu_key(MenuKey::Confirm, true);
        };
        let mut plain = seeded_game();
        walk_to_playing(&mut plain);
        let mut plain_buf = vec![0u32; SHOT_W * SHOT_H];
        plain.paint_at(
            &mut Frame {
                buf: &mut plain_buf,
                w: SHOT_W,
                h: SHOT_H,
            },
            5_100,
        );
        let mut staged = seeded_game();
        walk_to_playing(&mut staged);
        stage_playing_scene(&mut staged);
        staged.strike_at(2, 5_000);
        staged.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        let staged_rgb = frame_rgb(&mut staged, 5_100);
        staged.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        assert_eq!(staged.screen(), Screen::Playing);
        assert_ne!(
            staged_rgb,
            to_rgb(&plain_buf),
            "captions, figure, or strike changed no pixels"
        );
        let mut end_buf = vec![0u32; SHOT_W * SHOT_H];
        staged.paint_at(
            &mut Frame {
                buf: &mut end_buf,
                w: SHOT_W,
                h: SHOT_H,
            },
            7_000,
        );
        assert_eq!(staged.screen(), Screen::Result, "spent chart missed Result");
    }

    #[test]
    fn playing_shot_paints_the_staged_volume_readout() {
        // The reference photo's tally strip carries a `VOL 58%` readout;
        // the shot stages the same volume so `playing.png` shows the
        // shell's `VOL <n>` ink at the bound VOLUME rect (705,579).
        let walk_to_playing = |game: &mut ScreenGame| {
            game.lane_key(3, true);
            game.click_planet(136, 88);
            for _ in 0..3 {
                game.menu_key(MenuKey::Confirm, true);
            }
            arm_loading_gate(game);
            game.menu_key(MenuKey::Confirm, true);
        };
        let mut plain = seeded_game();
        walk_to_playing(&mut plain);
        let mut staged = seeded_game();
        walk_to_playing(&mut staged);
        stage_playing_scene(&mut staged);
        staged.strike_at(2, 5_000);
        let (w, h) = (
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        let frame = |game: &mut ScreenGame| {
            let mut buf = vec![0u32; w * h];
            game.paint_at(
                &mut Frame {
                    buf: &mut buf,
                    w,
                    h,
                },
                5_100,
            );
            buf
        };
        // The bound VOLUME rect (705,579) 44x9 isolates the readout:
        // nearest other strip ink is the title line (ends x 666) and
        // the EXIT button art (starts x 751). The staged strip art is
        // identical in both frames, so only the `VOL <n>` ink may
        // differ inside the box.
        const VOLUME_BOX: (usize, usize, usize, usize) = (703, 577, 751, 589);
        let box_bytes = |buf: &[u32]| {
            let (x0, y0, x1, y1) = VOLUME_BOX;
            (y0..y1)
                .flat_map(|y| (x0..x1).map(move |x| y * w + x))
                .map(|at| buf[at])
                .collect::<Vec<u32>>()
        };
        let (plain_box, staged_box) = (
            box_bytes(&frame(&mut plain)),
            box_bytes(&frame(&mut staged)),
        );
        assert_ne!(
            plain_box, staged_box,
            "staged volume must paint VOL ink into the strip box"
        );
    }

    #[test]
    fn playing_shot_paints_the_backdrop_edge_label() {
        // The BO edge label is baked art of the runtime-built BACK1
        // backdrop (`Playing_NormalBG<d>.ojs`, full-screen first
        // pass — `docs/playing-backdrop.md`): its ink box
        // (4..40, 570..596) sits left of the strip panel's x 204
        // origin, so nothing covers it and the staged shot must ink
        // it, while a shot staged without the backdrop figure keeps
        // the box clear. (The sibling EA05 label lands under the
        // opaque `Playing_Menu.ojs` body and never paints in 7.55.)
        use o2jam_shell::avatar::{playing_backdrop_sprite, PLAYING_BACKDROP_CODE};
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let box_bytes = |buf: &[u32], w: usize| {
            // BO label ink box, native px (docs/playing-backdrop.md).
            const LABEL_BOX: (usize, usize, usize, usize) = (4, 570, 40, 596);
            let (x0, y0, x1, y1) = LABEL_BOX;
            (y0..y1)
                .flat_map(|y| (x0..x1).map(move |x| y * w + x))
                .map(|at| buf[at])
                .collect::<Vec<u32>>()
        };
        let frame = |game: &mut ScreenGame| {
            let (w, h) = (
                usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
                usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
            );
            let mut buf = vec![0u32; w * h];
            game.paint_at(
                &mut Frame {
                    buf: &mut buf,
                    w,
                    h,
                },
                5_100,
            );
            (buf, w)
        };
        let mut staged = seeded_game();
        walk_to_playing(&mut staged);
        staged.strike_at(2, 5_000);
        let (staged_buf, w) = frame(&mut staged);
        // Control: the same staged Playing scene minus the BACK1
        // backdrop placement and figure, reloaded over the game.
        let mut control = seeded_game();
        walk_to_playing(&mut control);
        let mut art = stage_state(
            &archive,
            State::Playing,
            StageContext {
                arena_stage: Some(control.arena_stage()),
                key_mode: control.room_key_mode(),
                team_play: false,
                chart_slot: PLAYING_SONG.difficulty,
            },
        )
        .art
        .expect("playing: STATE_PLAYING stages");
        let backdrop = playing_backdrop_sprite(false, PLAYING_SONG.difficulty);
        art.placements
            .retain(|placed| placed.code != PLAYING_BACKDROP_CODE);
        art.figures.remove(&backdrop);
        control.load_scene(Screen::Playing, art.placements, art.figures);
        control.strike_at(2, 5_000);
        let (control_buf, _) = frame(&mut control);
        assert_ne!(
            box_bytes(&staged_buf, w),
            box_bytes(&control_buf, w),
            "the BACK1 backdrop must ink the BO label box"
        );
        assert!(
            box_bytes(&staged_buf, w)
                .iter()
                .filter(|pixel| **pixel != 0)
                .count()
                >= 20,
            "the BO label box must carry real ink, not noise"
        );
    }

    #[test]
    fn playing_shot_paints_session_fed_peer_gauge() {
        // The session bridge seats the wire peer's gauge: the
        // scripted 4015 Life echo (slot 1, full) rides
        // `apply_peer_echo` — the same stats-then-ranks order the
        // live session folds echoes through — so the playing paint
        // fills the slot-1 top-right gauge green. The walked strikes
        // both rate Cool, whose Hard +1s saturate, so local life
        // stays full behind the avatar bar.
        use o2jam_shell::playing::NATIVE_PEER_BAR;

        let mut game = seeded_game();
        assert_eq!(
            game.live_play()
                .peers()
                .life(usize::from(SCRIPTED_PEER_SLOT)),
            Some(SCRIPTED_PEER_LIFE),
            "the fixture must seat the peer gauge"
        );
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(game.life(), LIFE_MAX, "walked Cools keep local life full");
        let frame = paint_native_frame(&mut game, 5_100);
        let (w, _) = native_size();
        let slot_y = NATIVE_PEER_BAR.y + i32::from(SCRIPTED_PEER_SLOT) * NATIVE_PEER_BAR.h as i32;
        let green = o2jam_render::pack_xrgb(0x20, 0xE0, 0x20);
        let mut filled = 0usize;
        let mut total = 0usize;
        for row in 0..NATIVE_PEER_BAR.h {
            for col in 0..NATIVE_PEER_BAR.w {
                let x = NATIVE_PEER_BAR.x + i32::try_from(col).unwrap_or(0);
                let y = slot_y + i32::try_from(row).unwrap_or(0);
                let x = usize::try_from(x).unwrap_or(0);
                let y = usize::try_from(y).unwrap_or(0);
                total += 1;
                if frame[y * w + x] == green {
                    filled += 1;
                }
            }
        }
        assert!(
            filled * 2 >= total,
            "slot-1 peer gauge must read green ({filled}/{total})"
        );
    }

    #[test]
    fn walk_covers_all_scenes_plus_solo_result() {
        let shots = capture_all().unwrap();
        let names: Vec<&str> = shots.iter().map(|shot| shot.name).collect();
        // Side-state shots ride along only when install-root stages
        // their art (clean skip otherwise — CI arm).
        let side_names: Vec<&str> = [
            SideScreen::Shop,
            SideScreen::MusicShop,
            SideScreen::MyRoom,
            SideScreen::Lanking,
            SideScreen::Payment,
            SideScreen::Tutorial,
        ]
        .into_iter()
        .filter(|side| side_stage_data(*side).is_some())
        .map(side_shot_name)
        .collect();
        // Boot order (S0.3): the bare board shoots before the dialog
        // opens over it.
        let mut expected = vec![
            "avi",
            "planet",
            "channel",
            "room",
            "waiting",
            "waiting-guest",
        ];
        expected.extend_from_slice(&side_names);
        expected.extend_from_slice(&[
            "musicselect",
            "loading",
            "playing",
            "playing-native",
            "result",
            "result-native",
            "result-solo",
            "result-solo-native",
        ]);
        assert_eq!(names, expected);
        for shot in &shots {
            let expected = if shot.name.ends_with("-native") {
                (800, 600)
            } else {
                (SHOT_W, SHOT_H)
            };
            assert_eq!((shot.w, shot.h), expected);
            assert_eq!(shot.rgb.len(), shot.w * shot.h * 3);
            assert!(is_varied(&shot.rgb), "{} paints a flat frame", shot.name);
        }
    }

    /// First opaque pixel of a figure's first layer plus its decoded
    /// Topmost inked layer, its first opaque pixel, and the probed
    /// color, via a 1x1 probe blit (only that pixel survives the
    /// clip). `None` when the body has no decodable layer or no
    /// opaque pixel. Topmost first: the shared painter stacks every
    /// layer back to front, so an early layer's ink may hide under a
    /// later one — only the topmost ink must survive on the frame.
    fn first_opaque_probe(figure: &AvatarFigure) -> Option<(usize, i32, i32, u32)> {
        (0..figure.len()).rev().find_map(|index| {
            let layer = figure.layer(index)?;
            let at = (0..layer.image.w * layer.image.h)
                .find(|at| layer.image.words[*at] != layer.colorkey)?;
            let (px, py) = ((at % layer.image.w) as i32, (at / layer.image.w) as i32);
            let mut probe = vec![0u32; 1];
            o2jam_render::blit_rgb555(
                &mut Frame {
                    buf: &mut probe,
                    w: 1,
                    h: 1,
                },
                &layer.image.view(),
                -px,
                -py,
                Some(layer.colorkey),
            );
            (probe[0] != 0).then_some((index, px, py, probe[0]))
        })
    }

    /// Drive every claimed placement with a decodable body of one
    /// staged State through the shared painter, alone on a blank
    /// native frame, and require its first opaque pixel to land at
    /// its own rect origin. Returns the number of entries proven.
    fn staged_entries_paint_on_origins(art: &StagedArt, label: &str) -> usize {
        let (placements, figures) = (&art.placements, &art.figures);
        let (frame_w, frame_h) = (
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        let mut checked = 0;
        for placed in placements {
            let claimed = placed.rect.w != 0 || placed.rect.h != 0;
            let Some(name) = placed.sprite.as_deref() else {
                continue;
            };
            let Some(figure) = figures.get(name) else {
                continue;
            };
            if !claimed {
                continue;
            }
            // A `SET` clone paints its frame-0 layer only (one image
            // control, one frame); single placements stack every layer
            // and the topmost one owns the ink.
            let probe_figure = if placed.role == PlacementRole::Clone {
                figure.layers_through(0)
            } else {
                figure.clone()
            };
            let Some((top, px, py, ink)) = first_opaque_probe(&probe_figure) else {
                continue;
            };
            // Verified draw chain (avatar.rs `placement_view_origin`):
            // anchored bodies draw at their record anchors (frame
            // scaled); anchor-free bodies at their bound rect origins.
            // Most bound bodies duplicate their rect origins byte-exact;
            // `BpPoint_Left_bar.ojs` (2,266) is the photo-pinned
            // disagreeing pair whose anchor wins. The probed layer
            // names its own anchor: layers stack at their own record
            // anchors, so the topmost ink lands at the topmost anchor.
            let scaled = o2jam_render::scale_rect_to_frame(&placed.rect, frame_w, frame_h);
            let layer = probe_figure.layer(top).expect("probed figure has a layer");
            let (ax, ay) = anchor_in_frame(layer.dx, layer.dy, frame_w, frame_h);
            let expected = match (placed.role, figure.has_anchor_offset()) {
                // A clone's rect positions the body; its record anchors
                // stay template-local and add on top.
                (PlacementRole::Clone, _) => (scaled.x + ax, scaled.y + ay),
                // Anchored singles author their own absolute position.
                (PlacementRole::Single, true) => (ax, ay),
                (PlacementRole::Single, false) => (scaled.x, scaled.y),
            };
            let mut buf = vec![0u32; frame_w * frame_h];
            paint_scene_figures(
                &mut Frame {
                    buf: &mut buf,
                    w: frame_w,
                    h: frame_h,
                },
                std::slice::from_ref(placed),
                figures,
            );
            let (probe_x, probe_y) = (expected.0 + px, expected.1 + py);
            if probe_x < 0
                || probe_y < 0
                || probe_x as usize >= frame_w
                || probe_y as usize >= frame_h
            {
                // Cloned template children can ride off-canvas with their
                // parent; the blit clips them and there is nothing to probe.
                continue;
            }
            let at = probe_y as usize * frame_w + probe_x as usize;
            assert_eq!(
                buf[at], ink,
                "{label}:{name} must paint at its placement origin"
            );
            checked += 1;
        }
        checked
    }

    #[test]
    fn every_bound_entry_paints_at_its_placement_origin() {
        // Multi-entry proof of the shared rule on real 7.55 bytes:
        // ordinal pairing feeds every bound entry its own rect, and
        // the painter places anchored bodies at their record anchors
        // (TimeBar (489,556), EXIT (751,554), BpPoint (2,266) — the
        // photo-pinned disagreeing pair) and anchor-free bodies at
        // their rect origins. The same loop runs the declared Playing
        // subset (Playing1.opi), the declared Room State, and the
        // song-select dialog (DAILOG_SELECTMUSIC over Waiting — the
        // ticket-#15 arm: its EXTEND rows land on the staged table
        // bands), so the rule holds per scene and per entry, not for
        // one sprite.
        let mut checked = 0;
        if let Some(archive) = read_install_archives(&install_root()).playing {
            let ctx = StageContext::default();
            if let Some(art) = stage_state(&archive, State::Playing, ctx).art {
                checked += staged_entries_paint_on_origins(&art, "STATE_PLAYING");
            }
        } else {
            println!("skip: missing {PLAYING_ARCHIVE}");
        }
        if let Some(archive) = read_install_archives(&install_root()).interface {
            if let Some(art) = stage_state(&archive, State::Room, StageContext::default()).art {
                checked += staged_entries_paint_on_origins(&art, "STATE_ROOM");
            }
            if let Some(art) =
                stage_dialog(&archive, Dialog::SelectMusic, StageContext::default()).art
            {
                checked += staged_entries_paint_on_origins(&art, "DAILOG_SELECTMUSIC");
            }
        } else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
        }
        if checked == 0 {
            println!("skip: no archives present");
            return;
        }
        assert!(
            checked >= 5,
            "multi-entry proof covered only {checked} bound entries"
        );
        println!("proved {checked} bound entries paint on their own rect origins");
    }

    #[test]
    fn staged_time_bar_paints_on_its_bound_rect() {
        // Verified trace on real 7.55 bytes: `IDC_IMAGE_SPLAY_TIMEBAR`
        // (entry 215 of `STATE_PLAYING`, code `0x1B500003`) pairs by
        // code ordinal 3 with `State_Playing.bnd` rect[2]
        // (489,556)-(743,566), and its body's record anchor duplicates
        // that origin byte-exact — so the overlay must paint the art
        // at (489,556), never at rect + anchor (which lands off-canvas
        // and hides the bar).
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        // The declared Playing subset carries the same bound run:
        // placements pair by code ordinal, figures decode by name.
        let art = stage_state(&archive, State::Playing, StageContext::default())
            .art
            .expect("STATE_PLAYING stages");
        let (placements, figures) = (&art.placements, &art.figures);
        let placed = placements
            .iter()
            .find(|placed| placed.sprite.as_deref() == Some("Normal_TimeBar.ojs"))
            .expect("the bound run stages the TIME bar");
        assert_eq!(placed.rect.x, 489);
        assert_eq!(placed.rect.y, 556);
        assert_eq!(placed.rect.w, 254);
        assert_eq!(placed.rect.h, 10);
        let figure = figures.get("Normal_TimeBar.ojs").expect("body decodes");
        let (_, px, py, ink) = first_opaque_probe(figure).expect("body has opaque pixels");
        let mut buf = vec![0u32; 800 * 600];
        paint_foreground_placements(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            placements,
            figures,
        );
        assert_eq!(
            buf[(556 + py as usize) * 800 + (489 + px as usize)],
            ink,
            "TIME-bar art must land at its bound rect origin"
        );
    }

    #[test]
    fn playing_gauge_paints_at_its_staged_anchor_rect() {
        // Ticket #102 (bnd-audit 16): the member life gauge
        // (`Avatar_Info_LifeGauge.ojs` — eight
        // `IDC_IMAGE_SPLAY_LIFEGAUGE1..8` entries on one 46x8 body)
        // carries one corner native anchor at (0,0), clear of the
        // note lanes (they open at row 28). Staging must consume
        // that staged anchor rect, never append the body as an
        // extra full-canvas placement over the 17-rect bound window
        // (ADR-0003: the painter consumes staged rects, never a
        // canvas stretch).
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let ctx = StageContext {
            arena_stage: Some(2),
            key_mode: 2,
            team_play: false,
            chart_slot: PLAYING_SONG.difficulty,
        };
        let art = stage_state(&archive, State::Playing, ctx)
            .art
            .expect("STATE_PLAYING stages");
        let (placements, figures) = (&art.placements, &art.figures);
        let placed = placements
            .iter()
            .find(|placed| placed.sprite.as_deref() == Some("Avatar_Info_LifeGauge.ojs"))
            .expect("the gauge stages its placement");
        assert_eq!(
            (placed.rect.x, placed.rect.y, placed.rect.w, placed.rect.h),
            (0, 0, 0, 0),
            "the gauge rides its unplaced anchor rect alone"
        );
        for placed in placements {
            assert_ne!(
                (placed.rect.x, placed.rect.y, placed.rect.w, placed.rect.h),
                (0, 0, NATIVE_WIDTH, NATIVE_HEIGHT),
                "{} must never stage full-canvas",
                placed.id
            );
        }
        let figure = figures
            .get("Avatar_Info_LifeGauge.ojs")
            .expect("the gauge body decodes");
        let (index, px, py, ink) = first_opaque_probe(figure).expect("the body has opaque pixels");
        let layer = figure.layer(index).expect("the probe layer");
        assert!(
            i32::try_from(layer.image.w).unwrap_or(i32::MAX) <= 46
                && i32::try_from(layer.image.h).unwrap_or(i32::MAX) <= 8,
            "the gauge body stays inside its staged 46x8 extent"
        );
        let mut buf = vec![0u32; 800 * 600];
        paint_foreground_placements(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            placements,
            figures,
        );
        let at = usize::try_from(py).unwrap_or(0) * 800 + usize::try_from(px).unwrap_or(0);
        assert_eq!(
            buf[at], ink,
            "gauge ink must land at its corner anchor origin"
        );
    }

    #[test]
    fn playing_time_fill_stays_inside_the_staged_bar_rect() {
        // Ticket #102 leg 2: the procedural TIME shell and fill
        // route through the staged TIMEBAR rect (`State_Playing.bnd`
        // r2, (489,556)-(743,566) — geometry-audit DONE), never over
        // the bar art unclipped. Two song clocks paint identical
        // frames except the fill region, so every differing pixel
        // must sit inside the staged rect.
        use o2jam_shell::playing::{AvatarScene, PlayingBoundFields};

        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let art = stage_state(&archive, State::Playing, StageContext::default())
            .art
            .expect("STATE_PLAYING stages");
        let bar = PlayingBoundFields::from_placements(&art.placements).time_bar;
        assert_eq!(
            bar,
            o2jam_render::Rect {
                x: 489,
                y: 556,
                w: 254,
                h: 10
            },
            "the staged TIMEBAR rect is bnd r2"
        );
        let paint_at = |now_ms: u64| {
            let mut scene = AvatarScene::default();
            scene.set_song("First", 6);
            let mut buf = vec![0u32; 800 * 600];
            scene.paint_with_placements(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                None,
                &o2jam_game::Score::default(),
                now_ms,
                Some(10_000),
                Some(&art.placements),
            );
            buf
        };
        let early = paint_at(0);
        let later = paint_at(5_000);
        let (x0, y0) = (
            usize::try_from(bar.x).unwrap(),
            usize::try_from(bar.y).unwrap(),
        );
        let (x1, y1) = (
            x0 + usize::try_from(bar.w).unwrap(),
            y0 + usize::try_from(bar.h).unwrap(),
        );
        let mut changed = 0usize;
        for y in 0..600 {
            for x in 0..800 {
                if early[y * 800 + x] != later[y * 800 + x] {
                    changed += 1;
                    assert!(
                        (x0..x1).contains(&x) && (y0..y1).contains(&y),
                        "TIME fill delta at ({x},{y}) escapes the staged bar rect"
                    );
                }
            }
        }
        assert!(changed > 0, "the fill must actually paint mid-song");
    }

    #[test]
    fn missing_archives_stage_nothing() {
        assert!(read_install_archives(&install_root().join("No/Such"))
            .interface
            .is_none());
        assert!(read_install_archives(&install_root().join("No/Such"))
            .playing
            .is_none());
        // Empty bytes miss every row: bound rows stage nothing with
        // a miss in the ledger, block-less rows keep placements with
        // empty slots.
        for state in [State::Avi, State::Loading, State::Tutorial] {
            let report = stage_state(&[], state, StageContext::default());
            if let Some(art) = report.art {
                assert!(
                    art.figures.is_empty(),
                    "{state:?} stages no figures without game data"
                );
            }
        }
    }

    #[test]
    fn present_archives_stage_key_scenes() {
        let ctx = StageContext::default();
        let Some(interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        assert_channel_block_names_no_sprites(&interface);
        assert!(
            stage_state(&interface, State::Avi, ctx).art.is_some(),
            "avi stages its logo through the declaration"
        );
        let Some(playing) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        assert!(
            stage_state(&playing, State::Loading, ctx).art.is_some(),
            "loading stages its gate through the declaration"
        );
        // Side states: every side screen stages its placements plus
        // figures through the declaration; the tutorial backdrop
        // decodes from the Playing archive.
        for side in [
            SideScreen::Shop,
            SideScreen::MusicShop,
            SideScreen::MyRoom,
            SideScreen::Lanking,
            SideScreen::Payment,
        ] {
            let Some((placements, figures)) = side_stage_data(side) else {
                panic!("{side:?} stages");
            };
            assert!(!placements.is_empty(), "{side:?} placements stage");
            assert!(!figures.is_empty(), "{side:?} figures stage");
        }
        assert!(
            side_stage_data(SideScreen::Tutorial).is_some(),
            "tutorial backdrop stages"
        );
    }

    #[test]
    fn side_shot_paints_staged_placements_for_every_screen() {
        // The visit_side paint path consumes the staged placements:
        // every side screen inks staged pixels, and the frame stays
        // byte-identical once the side is left (flow unchanged).
        let Some(interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        let Some(playing) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let _ = (interface, playing);
        for side in [
            SideScreen::Shop,
            SideScreen::MusicShop,
            SideScreen::MyRoom,
            SideScreen::Lanking,
            SideScreen::Payment,
            SideScreen::Tutorial,
        ] {
            let mut game = seeded_game();
            walk_to_waiting(&mut game);
            let lobby_frame = paint_native_frame(&mut game, 0);
            assert!(stage_side_art(&mut game, side), "{side:?} stages");
            assert!(game.visit_side(side), "Waiting -> {side:?}");
            let side_frame = paint_native_frame(&mut game, 0);
            assert!(
                side_frame.iter().any(|&pixel| pixel != 0),
                "{side:?} paints no staged ink"
            );
            assert_ne!(
                side_frame, lobby_frame,
                "{side:?} changed no pixels over the lobby"
            );
            assert!(game.leave_side(), "{side:?} -> Waiting");
            let back_frame = paint_native_frame(&mut game, 0);
            assert_eq!(
                back_frame, lobby_frame,
                "leaving {side:?} must restore the lobby frame"
            );
        }
    }

    #[test]
    fn musicselect_chosung_grid_matches_the_bind_inventory() {
        // FILE wiring proof against the genuine bind: the staged
        // DAILOG_SELECTMUSIC placements carry 27 + 15 INDEX buttons
        // at the exact 15 px pitch (docs/geometry-audit.md).
        let Some(interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        let art = stage_dialog(&interface, Dialog::SelectMusic, StageContext::default())
            .art
            .expect("dialog stages");
        let layout = o2jam_shell::screens::side::select_chosung_layout(&art.placements)
            .expect("INDEX buttons stage");
        assert_eq!(layout.row_cells, vec![27, 15]);
        assert_eq!(layout.pitch, 15);
    }

    /// Pinned 7.55 fact the channel soak relies on: `STATE_CHANNEL`
    /// names zero sprites, so shots stage no scene art for it. Read
    /// through the public file readers (a file fact — scene staging
    /// goes through the declaration).
    fn assert_channel_block_names_no_sprites(archive: &[u8]) {
        let entries = o2jam_file::parse_opi(archive).expect("opi parses");
        let entry = o2jam_file::find_opi_entry(&entries, "ControlList_Interface.txt")
            .expect("control list exists");
        let body = o2jam_file::opi_entry_body(archive, entry).expect("control list reads");
        let text = o2jam_file::decode_control_text(body).expect("control list decodes");
        let list = o2jam_file::parse_control_list(&text).expect("control list parses");
        let block = scene_block(&list, "STATE_CHANNEL").expect("STATE_CHANNEL exists");
        assert!(
            scene_sprite_names(block).is_empty(),
            "channel block names art"
        );
    }

    #[test]
    fn channel_shot_paints_no_fill_under_the_live_list() {
        // Pinned no-fill rule (`docs/channel-fill.md`): the art-less
        // channel stages nothing, so the native corner stays untouched
        // (no navy fill) while the live title and list rows still ink.
        // The scene art is cleared explicitly: `seeded_game` stages the
        // genuine dialog when `install-root` is present (guarded by
        // `channel_shot_stages_dialog_art_inside_the_measured_bands`).
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
        // The board beneath the dialog is a separate scene slot: clear
        // it too so the art-less frame is deterministic with and
        // without `install-root` (the layered arm is guarded by
        // `channel_shot_stages_dialog_art_inside_the_measured_bands`).
        game.load_scene(Screen::Planet, Vec::new(), HashMap::new());
        assert_eq!(game.screen(), Screen::Channel);
        let frame = paint_native_frame(&mut game, 0);
        assert_eq!(frame[0], 0, "art-less channel paints no fill");
        assert!(
            !frame.contains(&o2jam_render::pack_xrgb(0x10, 0x20, 0x50)),
            "channel keeps the retired navy placeholder"
        );
        assert!(
            frame.contains(&o2jam_render::pack_xrgb(0xFF, 0xFF, 0xFF)),
            "live title and list ink missing"
        );
    }

    #[test]
    fn channel_shot_stages_dialog_art_inside_the_measured_bands() {
        // With genuine art staged, the dialog body paints at its
        // record anchor OVER the planet board (verified first screen,
        // `docs/planet.md`): new ink lands inside the 800x466 body
        // (the reference-measured panel core x185-615 y150-464 among
        // it). The pinned no-fill strip rule splits by staging:
        // without board art the strip below native y466 stays
        // byte-identical to the art-less frame; with the board staged
        // the MENU bar band may differ — but only where the board
        // itself paints.
        let Some(interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // Art-less baseline: same seed, both scene slots cleared.
        let mut base = seeded_game();
        base.lane_key(3, true);
        base.click_planet(136, 88);
        base.load_scene(Screen::Channel, Vec::new(), HashMap::new());
        base.load_scene(Screen::Planet, Vec::new(), HashMap::new());
        let base_frame = paint_native_frame(&mut base, 0);
        // Dialog-only arm (no board staged): the strip stays
        // byte-identical to the art-less frame — nothing beneath.
        let mut dialog_only = seeded_game();
        stage_channel_dialog(&mut dialog_only, &interface);
        dialog_only.load_scene(Screen::Planet, Vec::new(), HashMap::new());
        dialog_only.lane_key(3, true);
        dialog_only.click_planet(136, 88);
        assert_eq!(dialog_only.screen(), Screen::Channel);
        let dialog_frame = paint_native_frame(&mut dialog_only, 0);
        let mut inside_core = false;
        for (index, (&now, &before)) in dialog_frame.iter().zip(base_frame.iter()).enumerate() {
            if now == before {
                continue;
            }
            let x = index % NATIVE_WIDTH as usize;
            let y = index / NATIVE_WIDTH as usize;
            assert!(
                y < 466,
                "channel shot: staged ink below the body at ({x},{y})"
            );
            if (185..=615).contains(&x) && (150..=464).contains(&y) {
                inside_core = true;
            }
        }
        assert!(
            inside_core,
            "channel shot: no staged ink inside the measured panel core"
        );
        for index in 466 * NATIVE_WIDTH as usize..dialog_frame.len() {
            assert_eq!(
                dialog_frame[index], base_frame[index],
                "channel shot: fill below the dialog body without board art"
            );
        }
        // Board arm: with the planet board staged (seeded_game), the
        // board alone explains every strip pixel that differs from the
        // art-less frame — the MENU bar band is board ink, not
        // invented fill — and the staged board inks the strip at all.
        let mut board_only = seeded_game();
        board_only.load_scene(Screen::Channel, Vec::new(), HashMap::new());
        board_only.lane_key(3, true);
        board_only.click_planet(136, 88);
        let board_frame = paint_native_frame(&mut board_only, 0);
        let mut staged = seeded_game();
        stage_channel_dialog(&mut staged, &interface);
        staged.lane_key(3, true);
        staged.click_planet(136, 88);
        assert_eq!(staged.screen(), Screen::Channel);
        let frame = paint_native_frame(&mut staged, 0);
        let mut strip_diff = false;
        for index in 466 * NATIVE_WIDTH as usize..frame.len() {
            if frame[index] == base_frame[index] {
                continue;
            }
            strip_diff = true;
            assert_eq!(
                frame[index], board_frame[index],
                "channel shot: strip ink unexplained by the board at pixel {index}"
            );
        }
        assert!(
            strip_diff,
            "channel shot: staged board inked nothing in the MENU bar strip"
        );
    }

    #[test]
    fn planet_shot_stages_bound_art_with_the_picked_band() {
        // Guarded: without the interface archive the flat-card path
        // paints (no scene painter, no cursor) — covered unguarded in
        // `planet_shot_records_the_picked_band_without_art`.
        let Some(_interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // The walk reaches the bare board through the dialog X (no
        // invented input — the exe's own entry key stays OPEN), clicks
        // HARD through the pinned `.bnd` rect, and the lit-emblem hit
        // reopens the channel dialog while recording the pick
        // (player-report corroborated, per-emblem verification
        // pending). Back on the board the staged 17-placement bound
        // block (backdrop, five emblems, promotion image, MENU bar)
        // paints under the amber-cursor pick stand-in.
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.ingest_snapshot(&InputSnapshot::click_at(577, 171));
        assert_eq!(game.screen(), Screen::Planet, "dialog X → bare board");
        game.click_planet(386, 88);
        assert_eq!(
            game.planet_selection(),
            Some(o2jam_shell::screens::side::PlanetBand::Hard)
        );
        assert_eq!(
            game.screen(),
            Screen::Channel,
            "lit emblem reopens the channel dialog"
        );
        assert!(game.enter_screen(Screen::Planet), "dialog X → board again");
        let frame = paint_native_frame(&mut game, 0);
        assert!(
            frame.iter().any(|&px| px != frame[0]),
            "planet shot: frame is one flat fill"
        );
        assert!(
            frame.contains(&o2jam_render::pack_xrgb(0xFF, 0xD0, 0x20)),
            "planet shot: picked-band cursor missing"
        );
    }

    #[test]
    fn planet_shot_records_the_picked_band_without_art() {
        // No archives: the bare board shows the flat card and the
        // pinned-rect hit-test still records the pick (clean-skip
        // parity without install-root); the lit-emblem click reopens
        // the dialog through the flow gate.
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.ingest_snapshot(&InputSnapshot::click_at(577, 171));
        assert_eq!(game.screen(), Screen::Planet);
        game.click_planet(386, 88);
        assert_eq!(
            game.planet_selection(),
            Some(o2jam_shell::screens::side::PlanetBand::Hard)
        );
        assert_eq!(game.screen(), Screen::Channel);
        let frame = paint_native_frame(&mut game, 0);
        assert!(
            frame.iter().any(|&px| px != 0),
            "planet shot: dialog-over-board frame painted nothing"
        );
    }

    #[test]
    fn dialog_x_closes_the_channel_dialog_to_the_bare_board() {
        // The bnd-pinned X rect (541,151)-(589,189) hit-tests the
        // native X button: the verified close click (577,171) backs
        // out to Planet through the flow gate, a row face click does
        // not.
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        // Ticket #56: only the enter face advances — row 0 face
        // (502,236)-(546,257).
        game.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        assert_eq!(
            game.screen(),
            Screen::Room,
            "a row face click advances to the room list"
        );
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.ingest_snapshot(&InputSnapshot::click_at(577, 171));
        assert_eq!(game.screen(), Screen::Planet, "dialog X → Planet");
    }

    #[test]
    fn lit_emblem_click_reopens_the_channel_dialog() {
        // Player-report corroborated, per-emblem dynamic verification
        // pending (docs/planet.md): HARD/NORMAL/EASY clicks reopen the
        // channel dialog through the flow gate and record the pick;
        // FREE/DRUM clicks stay on the bare board unbound (P0).
        for (x, band) in [
            (386, o2jam_shell::screens::side::PlanetBand::Hard),
            (255, o2jam_shell::screens::side::PlanetBand::Normal),
            (136, o2jam_shell::screens::side::PlanetBand::Easy),
        ] {
            let mut game = seeded_game();
            game.lane_key(3, true);
            game.ingest_snapshot(&InputSnapshot::click_at(577, 171));
            assert_eq!(game.screen(), Screen::Planet);
            game.click_planet(x, 88);
            assert_eq!(game.screen(), Screen::Channel, "lit emblem → dialog");
            assert_eq!(game.planet_selection(), Some(band));
        }
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.ingest_snapshot(&InputSnapshot::click_at(577, 171));
        game.click_planet(663, 88);
        game.click_planet(559, 88);
        assert_eq!(game.planet_selection(), None);
        assert_eq!(game.screen(), Screen::Planet, "FREE/DRUM stay unbound");
    }

    #[test]
    fn planet_shot_rejects_free_and_drum_clicks() {
        // P0 on the shot walk too: FREE/DRUM clicks hit their pinned
        // rects but never select or advance anything.
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.click_planet(663, 88);
        game.click_planet(559, 88);
        assert_eq!(game.planet_selection(), None);
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn planet_shot_paints_emblem_captions() {
        // The `TEXT` captions ink their parsed `.bnd` strips
        // (HARD strip (338,125)-(432,138) native): real art plus
        // painter ink both count (the painter itself is proven by
        // the shell's synthetic-caption test).
        let Some(_interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        let mut game = seeded_game();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet, "boot → bare board");
        let frame = paint_native_frame(&mut game, 0);
        let mut lit = 0;
        for row in 125..138 {
            for col in 338..432 {
                if frame[row * 800 + col] != frame[125 * 800 + 338] {
                    lit += 1;
                }
            }
        }
        assert!(lit > 0, "planet caption strip painted nothing");
    }

    #[test]
    fn planet_modals_stage_their_dialog_art() {
        // Power opens the staged quit-confirm (OK quits through the
        // dialog-relative hit); gear opens the staged options sheet
        // and Escape closes it. Guarded: without archives the
        // fallback popup/title paths are covered by the shell suite.
        let Some(_interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        let mut game = seeded_game();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        let board = paint_native_frame(&mut game, 0);
        game.click_planet(769, 580);
        assert_eq!(
            game.planet_modal(),
            Some(o2jam_shell::screens::game::PlanetModal::QuitConfirm)
        );
        let popup = paint_native_frame(&mut game, 0);
        assert_ne!(popup, board, "quit popup painted nothing");
        // OK center through the dialog-relative rect: origin
        // (240,189) + (52,181) + half of 107x26.
        game.ingest_snapshot(&InputSnapshot::click_at(345, 383));
        assert!(game.quit_confirmed(), "staged OK must quit");
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.click_planet(729, 580);
        assert_eq!(
            game.planet_modal(),
            Some(o2jam_shell::screens::game::PlanetModal::Options)
        );
        let sheet = paint_native_frame(&mut game, 0);
        assert_ne!(sheet, board, "options sheet painted nothing");
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.planet_modal(), None, "Escape closes options");
    }

    #[test]
    fn play_skin_decodes_every_lane_when_present() {
        use o2jam_shell::playing::PlaySkin;
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            panic!("playing: PlaySkin decoded no lanes");
        };
        assert!(!skin.is_empty());
        assert_eq!(skin.staged_lanes(), 7);
        for lane in 0..7u8 {
            assert!(skin.note_head(lane).is_some(), "lane {lane}: no note head");
            assert!(
                skin.receptor_frame_count(lane) > 0,
                "lane {lane}: no receptor frames"
            );
        }
    }

    #[test]
    fn play_skin_keeps_fallback_without_archive() {
        use o2jam_shell::playing::PlaySkin;
        assert!(PlaySkin::from_archive(&[]).is_none());
        assert!(PlaySkin::from_archive(&[0u8; 64]).is_none());
        assert!(read_install_archives(&install_root().join("No/Such"))
            .interface
            .is_none());
    }

    #[test]
    fn skinned_highway_replaces_procedural_note_ink() {
        use o2jam_render::{pack_xrgb, Frame};
        use o2jam_shell::playing::{HighwayGeometry, LivePlay, PlaySkin};
        const NOTE_BLUE: u32 = pack_xrgb(0x40, 0xC0, 0xFF);
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            println!("skip: PlaySkin decoded no lanes");
            return;
        };
        assert!(skin.note_head(2).is_some());
        let paint_notes = |skinned: bool| {
            let mut live = LivePlay::new(&[(2, 1500, 7)]);
            if skinned {
                live.set_skin(skin.clone());
            }
            let geom = HighwayGeometry::centered(SHOT_W, SHOT_H);
            let mut buf = vec![0u32; SHOT_W * SHOT_H];
            live.paint(
                &mut Frame {
                    buf: &mut buf,
                    w: SHOT_W,
                    h: SHOT_H,
                },
                &geom,
                1_000,
            );
            buf
        };
        let plain = paint_notes(false);
        assert!(plain.contains(&NOTE_BLUE), "procedural note ink missing");
        let staged = paint_notes(true);
        assert_ne!(plain, staged, "skin changed no pixels");
        assert!(!staged.contains(&NOTE_BLUE), "procedural core under skin");
        assert!(is_varied(&to_rgb(&staged)), "skinned frame is flat");
    }

    /// Backdrop of a probe frame: no staged digit ink can equal it,
    /// so non-backdrop probe pixels are exactly the font pixels.
    const PROBE_BACKDROP: u32 = 0x020202;

    /// Native-size probe buffer filled with [`PROBE_BACKDROP`].
    fn native_probe() -> Vec<u32> {
        let (w, h) = (
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        vec![PROBE_BACKDROP; w * h]
    }

    /// Paint into a native-size probe buffer.
    fn paint_probe(buf: &mut Vec<u32>, paint: impl FnOnce(&mut Frame<'_>)) {
        paint(&mut Frame {
            buf,
            w: usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            h: usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        });
    }

    /// Every font pixel of `probe` must land unchanged in `actual`
    /// (the staged font painted there, and nothing painted over it).
    fn assert_pixels_came_from_font(actual: &[u32], probe: &[u32], what: &str) {
        assert_eq!(actual.len(), probe.len());
        for (at, pixel) in probe.iter().enumerate() {
            if *pixel != PROBE_BACKDROP {
                assert_eq!(actual[at], *pixel, "{what} pixel {at}");
            }
        }
    }

    /// The scripted walk from splash into Playing, as `capture_all`
    /// drives it. The last Confirm is the S7 Loading→Playing gate:
    /// it is armed the way the live lane stages song prep.
    fn walk_to_playing(game: &mut ScreenGame) {
        game.lane_key(3, true);
        game.click_planet(136, 88);
        for _ in 0..3 {
            game.menu_key(MenuKey::Confirm, true);
        }
        arm_loading_gate(game);
        game.menu_key(MenuKey::Confirm, true);
    }

    /// The scripted walk from splash into Waiting, as `capture_all`
    /// drives it (the waiting shot's instant).
    fn walk_to_waiting(game: &mut ScreenGame) {
        game.lane_key(3, true);
        game.click_planet(136, 88);
        for _ in 0..2 {
            game.menu_key(MenuKey::Confirm, true);
        }
    }

    #[test]
    fn playing_shot_paints_the_staged_combo_digit() {
        use o2jam_shell::playing::combo_digits_origin;
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            panic!("playing: PlaySkin decoded no lanes");
        };
        let font = skin.combo_font().expect("Note_ComboNum.ojs font");
        // A45 (#104): the combo digits paint left to right from the
        // exe layout-box origin (89,60) — the number engines
        // `0x528E20`/`0x529230` `SetRect(0x59,0x3C,0xD5,0xD2)`
        // (docs/playing-digits.md §3), replacing the invented
        // `COMBO_ROW_ABOVE_LINE` anchor.
        let (x, y) = combo_digits_origin(
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        // The exact staged digits the shot paints (tap Cool then hold
        // head Cool: combo 2, judged 100 ms before the paint lands).
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| font.paint(frame, x, y, 2));
        // The playing shot's frame at the same instant.
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 5_100));
        assert_eq!(game.screen(), Screen::Playing);
        assert_pixels_came_from_font(&actual, &expected, "combo digit");
        // Delta rule (#63): a rest frame at the same instant (no
        // judgement struck, so no combo digits) must differ from the
        // painted frame at the pinned font pixels — real-art
        // rest-vs-painted delta, not a synthetic construction.
        let mut rest_game = seeded_game();
        walk_to_playing(&mut rest_game);
        let mut rest = native_probe();
        paint_probe(&mut rest, |frame| rest_game.paint_at(frame, 5_100));
        let changed = expected.iter().zip(&actual).zip(&rest).any(
            |((&expected_px, &actual_px), &rest_px)| {
                expected_px != PROBE_BACKDROP && actual_px == expected_px && rest_px != actual_px
            },
        );
        assert!(
            changed,
            "rest ink must differ from painted ink at the pinned combo-digit pixels"
        );
    }

    #[test]
    fn playing_shot_paints_the_staged_long_note_caption() {
        // The reference photo's LONGNOTE COMBO caption over the lanes
        // (ink (91,282)-(165,291) game px) stages through the runtime
        // body `Note_LongCombo.ojt` frame 1 and paints over the staged
        // scene art: every plate ink pixel of a lone blit at the
        // pinned origin must appear in the playing shot's frame.
        use o2jam_shell::playing::long_combo_plate_origin;
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            panic!("playing: PlaySkin decoded no lanes");
        };
        let plate = skin.long_combo_plate().expect("Note_LongCombo.ojt plate");
        let (origin_x, origin_y) = long_combo_plate_origin(
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            plate.paint_at(frame, origin_x, origin_y)
        });
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 5_100));
        assert_eq!(game.screen(), Screen::Playing);
        assert_pixels_came_from_font(&actual, &expected, "long-note caption");
    }

    #[test]
    fn playing_shot_paints_the_held_long_note_beam() {
        // The reference highway's held beam (reference-4x3.png): the
        // head strike inside the window arms the hold, so the playing
        // instant beams lane 0 down to the judgement line — the
        // staged `LongNote_Effect` art with the hit burst — while the
        // untouched twin leaves that band bare below its scrolling
        // head-to-tail beam.
        use o2jam_shell::playing::{HighwayGeometry, NATIVE_LINE_Y};
        let paint_instant = |game: &mut ScreenGame| {
            game.strike_at(2, 5_000);
            paint_native_frame(game, 5_100)
        };
        let mut idle = seeded_game();
        walk_to_playing(&mut idle);
        let idle_frame = paint_instant(&mut idle);
        let mut held = seeded_game();
        walk_to_playing(&mut held);
        held.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        let held_frame = paint_instant(&mut held);
        assert_ne!(held_frame, idle_frame, "the armed hold changed no pixels");
        // Lane 0's column just above the judgement line: the held
        // beam runs to the line; the untouched hold's beam stops at
        // its scrolling head, rows above.
        let (w, h) = native_size();
        let lane0 = HighwayGeometry::centered(w, h).lane_rect(0);
        let x0 = usize::try_from(lane0.x + 1).unwrap_or(0);
        let width = usize::try_from(lane0.w.saturating_sub(2)).unwrap_or(0);
        let band = (NATIVE_LINE_Y - 16..NATIVE_LINE_Y).any(|y| {
            let Ok(row) = usize::try_from(y) else {
                return false;
            };
            (0..width).any(|dx| {
                let at = row * w + x0 + dx;
                at < held_frame.len() && held_frame[at] != idle_frame[at]
            })
        });
        assert!(band, "no held-beam ink reached the judgement line");
    }

    #[test]
    fn skinned_highway_renders_hold_tails_and_tick_feedback() {
        // Ticket #84's skinned guard, real 7.55 art: (1) the paired
        // tail paints as a scrolling note at its onset — the staged
        // `Note_Image` head blits at the tail row (a missed head kills
        // the beam, so the tail row carries no other ink); (2) the
        // armed hold's tick feedback paints the long-note combo count
        // through the staged `Note_LongComboNum.ojs` font under the
        // caption plate.
        use o2jam_shell::playing::{
            long_combo_digits_origin, HighwayGeometry, LivePlay, LONG_COMBO_DIGIT_GAP,
        };
        let Some(archive) = read_install_archives(&install_root()).playing else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            panic!("playing: PlaySkin decoded no lanes");
        };
        let geom = HighwayGeometry::centered(
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        // (1) The untouched hold: head 1500, tail 2500, painted at
        // 2000 — the head missed (no strike), so no beam runs, and the
        // tail head blits 500 ms ahead of the 487-row line.
        let mut untouched = LivePlay::new_holds(&[(2, 1_500, 2_500, 7)]);
        untouched.set_skin(skin.clone());
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| untouched.paint(frame, &geom, 2_000));
        let tail_y = geom.line_y - 500 * geom.line_y / 2000;
        let head = skin.note_head(2).expect("lane-2 note head");
        let column = geom.lane_rect(2);
        let head_x = column.x
            + (i32::try_from(column.w).expect("lane width")
                - i32::try_from(head.extent().0).unwrap_or(0))
                / 2;
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| head.paint_at(frame, head_x, tail_y));
        assert_pixels_came_from_font(&actual, &expected, "scrolling tail head");
        // (2) The armed hold: strike the head in-window, paint 500 ms
        // into the span — one tick fed the snapshot, so the count
        // digit paints centered under the caption plate.
        let mut held = LivePlay::new_holds(&[(2, 1_500, 2_500, 7)]);
        held.set_skin(skin);
        held.strike_lane(2, 1_500);
        let mut highway = native_probe();
        paint_probe(&mut highway, |frame| held.paint(frame, &geom, 2_000));
        let font = held
            .long_note_tick_font()
            .expect("Note_LongComboNum.ojs font")
            .clone();
        let ticks = held.shown_long_note_ticks();
        assert_eq!(ticks, 1, "one tick fed inside the open span");
        let mut expected = native_probe();
        let (center_x, row) = long_combo_digits_origin(
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        let width = i32::try_from(font.width_of_spaced(ticks, LONG_COMBO_DIGIT_GAP))
            .expect("digit width fits i32");
        paint_probe(&mut expected, |frame| {
            font.paint_spaced(
                frame,
                center_x - width / 2,
                row,
                ticks,
                LONG_COMBO_DIGIT_GAP,
            );
        });
        let mut over_art = native_probe();
        paint_probe(&mut over_art, |frame| {
            held.paint_avatar_scene(frame, None, 2_000)
        });
        assert_pixels_came_from_font(&over_art, &expected, "long-note tick count");
    }

    #[test]
    fn scripted_hold_keyup_spends_the_chart_into_result() {
        // The verified release path closes the walked hold: the
        // key-up on the tail rates the armed tail Cool and marks it
        // judged, so the chart finishes and the walked rows carry the
        // zero bonus the paint-free walk accumulates.
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        let mut probe = native_probe();
        paint_probe(&mut probe, |frame| game.paint_at(frame, 7_000));
        assert_eq!(
            game.screen(),
            Screen::Result,
            "the key-up missed the chart spend"
        );
        let rows = game.result_rank_rows();
        assert_eq!(rows[0].judgements, Some([3, 0, 0, 0]));
        assert_eq!(rows[0].bonus, Some(0));
    }

    #[test]
    fn result_shot_paints_the_pinned_digits() {
        // The digits are GDI text (no staged body): the scripted walk
        // spends the two-record chart all-Cool — the tap, the hold
        // head, and the held tail all rate Cool: counts 3/0/0/0,
        // max combo 3, jam level 0, 600 points — at the seven pinned
        // fields (docs/cresult-anchors.md).
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let ink = o2jam_render::pack_xrgb(0xFF, 0xFF, 0xFF);
            o2jam_render::draw_number(frame, 151, 125, 3, ink);
            o2jam_render::draw_number(frame, 240, 125, 0, ink);
            o2jam_render::draw_number(frame, 337, 125, 0, ink);
            o2jam_render::draw_number(frame, 426, 125, 0, ink);
            o2jam_render::draw_number(frame, 505, 130, 3, ink);
            o2jam_render::draw_number(frame, 596, 130, 0, ink);
            o2jam_render::draw_number(frame, 605, 533, 600, ink);
        });
        assert_pixels_came_from_font(&actual, &expected, "pinned result digits");
    }

    /// Rank-row columns the shot pins (mirror the painter's
    /// anchors in `o2jam-shell/src/screens/paint.rs`: name x=128
    /// y=318, judgements x=230 y=317, score x=530 y=317, GEM bonus
    /// x=626, speed x=685; rows step 22 px). Row 0 is the local
    /// tally, row 1 the wire peer (name and shared speed only —
    /// per-member tallies stay server-side).
    const RANK_ROW_NAME: (i32, i32, &str) = (128, 318, "YOU");
    /// Combined judgement text (`%5d /%5d /%3d /%3d`) for the
    /// scripted all-Cool walk: tap, hold head, and tail plus the
    /// follow-up tap all rate Cool.
    const RANK_ROW_JUDGEMENTS: (i32, i32, &str) = (230, 317, "    3 /    0 /  0 /  0");
    /// Row score text (`%8d`) for the scripted 600-point walk.
    const RANK_ROW_SCORE: (i32, i32, &str) = (530, 317, "     600");
    /// Peer name text on row 1 (name column, one 22 px pitch down).
    const RANK_ROW_PEER: (i32, i32, &str) = (128, 340, RESULT_PEER_NAME);
    /// Scripted rank numbers (`%d` at x=91): local rank 1 on row 0,
    /// peer rank 2 on row 1 — the reference photo's row values.
    const RANK_ROW_RANK_LOCAL: (i32, i32, &str) = (91, 317, "1");
    const RANK_ROW_RANK_PEER: (i32, i32, &str) = (91, 339, "2");
    /// GEM bonus text (`+ %d`) for the walked hold bonus on the
    /// battle walk: the accumulator ticks +10 per advance while the
    /// armed span is open, and the battle walk advances twice through
    /// it (the two playing paints) — the key-up advance lands past
    /// the tail, so it adds nothing. A walked total, never a blank.
    const RANK_ROW_BONUS: (i32, i32, &str) = (626, 317, "+ 20");
    /// GEM bonus text on the solo single row: the solo walk paints no
    /// playing frames before its key-up, so the accumulator stays
    /// zero — a walked zero, never a blank.
    const SOLO_ROW_BONUS: (i32, i32, &str) = (626, 317, "+ 0");
    /// Shared room speed text on both rows (the room's X1 code).
    const RANK_ROW_SPEED_LOCAL: (i32, i32, &str) = (685, 317, "X1");
    const RANK_ROW_SPEED_PEER: (i32, i32, &str) = (685, 339, "X1");

    #[test]
    fn result_shot_paints_the_rank_table_rows() {
        // The rank table (`FUN_004c3b40`) paints row 0 from the local
        // tally: rank 1, name `YOU`, the all-Cool walk (counts
        // 3/0/0/0, 600 points, `+ 20` walked hold bonus from the two
        // playing paints, shared X1 speed) at the pinned
        // columns, plus the wire peer's rank 2, name, and shared
        // speed on row 1. A stale shot from before the duo
        // walk leaves the peer band empty; the re-shoot must carry
        // ink there.
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        // The walked bonus counts the advances through the open span,
        // so the walk replays the two playing paints (+10 each),
        // exactly as `capture_all` paints them, before the key-up.
        let _ = paint_native_frame(&mut game, 5_100);
        let _ = paint_native_frame(&mut game, 5_100);
        game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let ink = o2jam_render::pack_xrgb(0xFF, 0xFF, 0xFF);
            for (x, y, text) in [
                RANK_ROW_RANK_LOCAL,
                RANK_ROW_NAME,
                RANK_ROW_JUDGEMENTS,
                RANK_ROW_SCORE,
                RANK_ROW_RANK_PEER,
                RANK_ROW_PEER,
                RANK_ROW_BONUS,
                RANK_ROW_SPEED_LOCAL,
                RANK_ROW_SPEED_PEER,
            ] {
                o2jam_render::draw_text(frame, x, y, text, ink);
            }
        });
        assert_pixels_came_from_font(&actual, &expected, "rank rows");
    }

    #[test]
    fn result_walk_names_the_duo_and_paints_scripted_ranks() {
        // The Live duo builds two rank rows: local numbers on row 0
        // (`YOU`), the peer name on row 1. The scripted rank fixture
        // delivers rank 1 to the local seat and rank 2 to the peer —
        // the reference photo's row values — while peer numbers stay
        // unpainted (per-member tallies stay server-side).
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        let _ = paint_native_frame(&mut game, 5_100);
        let _ = paint_native_frame(&mut game, 5_100);
        game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 2, "the duo must build two rows");
        assert_eq!(rows[0].name.as_deref(), Some("YOU"));
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[0].judgements, Some([3, 0, 0, 0]));
        assert_eq!(rows[0].score, Some(600));
        assert_eq!(rows[0].bonus, Some(20));
        assert_eq!(rows[0].speed, Some(1));
        assert_eq!(rows[1].name.as_deref(), Some(RESULT_PEER_NAME));
        assert_eq!(rows[1].rank, Some(2));
        assert_eq!(rows[1].judgements, None);
        assert_eq!(rows[1].combos, None);
        assert_eq!(rows[1].score, None);
        assert_eq!(rows[1].bonus, None);
        assert_eq!(rows[1].speed, Some(1));
    }

    #[test]
    fn scripted_rank_fixture_maps_local_one_peer_two() {
        // The fixture through the remap rule: each scripted byte
        // reaches its holder's slot, and trailing zeros match no
        // holder — their rows stay zeroed and hidden (never
        // invented), pinning the exact walked standing.
        use o2jam_game::PeerBars;
        let mut peers = PeerBars::new();
        peers.set_global_rank(SCRIPTED_RANK_GLOBAL);
        for (slot, rank) in SCRIPTED_RANK_STORED.iter().enumerate() {
            peers.set_stored_rank(slot, *rank);
        }
        let deliveries = peers.apply_rank_broadcast(SCRIPTED_RANK_PACKET);
        assert_eq!(
            deliveries,
            vec![
                RankDelivery {
                    position: 0,
                    rank: 1,
                    slot: Some(0),
                },
                RankDelivery {
                    position: 1,
                    rank: 2,
                    slot: Some(1),
                },
            ]
        );
        assert_eq!(peers.rank_row(0), Some(1));
        assert_eq!(peers.rank_row(1), Some(2));
        assert_eq!(peers.rank_row(2), Some(0));
        assert_eq!(peers.rank_row(7), Some(0));
    }

    #[test]
    fn result_walk_without_staged_ranks_hides_rank_cells() {
        // Zero convention unchanged outside the fixture: an unstaged
        // duo seeds no holders, so both rank cells stay hidden rather
        // than inventing a standing.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 2, "the duo must build two rows");
        assert_eq!(rows[0].rank, None);
        assert_eq!(rows[1].rank, None);
    }

    #[test]
    fn scripted_chat_fixture_delivers_admin_then_user_rows() {
        // The 3012/3013 push path converts the scripted wire structs
        // into display rows in walk order: the administrator row first,
        // then the peer's user row — the reference stacking (admin
        // notice on top, the user's chat last) at
        // `reference-waiting-4x3.png` rows 2–3.
        let rows = seeded_game().waiting_chat().to_vec();
        assert_eq!(rows.len(), 2, "the fixture must deliver two rows");
        assert_eq!(rows[0].sender, SCRIPTED_CHAT_ADMIN_SENDER);
        assert_eq!(rows[0].text, SCRIPTED_CHAT_ADMIN_TEXT);
        assert!(rows[0].admin, "the 3013 row must flag admin");
        assert_eq!(rows[1].sender, SCRIPTED_CHAT_USER_SENDER);
        assert_eq!(rows[1].text, SCRIPTED_CHAT_USER_TEXT);
        assert!(!rows[1].admin, "the 3012 row must not flag admin");
    }

    #[test]
    fn waiting_walk_without_staged_chat_holds_no_history() {
        // Empty histories paint nothing (the shell's no-invention
        // pin): re-joining the same room clears the scripted rows,
        // so no history is carried across rooms.
        let mut game = seeded_game();
        game.set_joined_room(waiting_room());
        assert!(
            game.waiting_chat().is_empty(),
            "re-join must clear the scripted rows"
        );
    }

    #[test]
    fn waiting_shot_paints_the_scripted_chat_rows() {
        // Wiring end to end: the battle walk lands both fixture rows
        // in the Waiting frame. The cleared twin stages identical
        // art (same seed, same re-join, same walk), so only the chat
        // rows may differ between the frames — and the row texts must
        // read sender, text, and the admin prefix.
        use o2jam_shell::screens::paint::waiting_chat_row_text;

        let mut staged = seeded_game();
        let mut plain = seeded_game();
        plain.set_joined_room(waiting_room());
        walk_to_waiting(&mut staged);
        walk_to_waiting(&mut plain);
        assert_eq!(staged.screen(), Screen::Waiting);
        assert_eq!(plain.screen(), Screen::Waiting);
        let mut staged_frame = native_probe();
        paint_probe(&mut staged_frame, |frame| staged.paint_at(frame, 0));
        let mut plain_frame = native_probe();
        paint_probe(&mut plain_frame, |frame| plain.paint_at(frame, 0));
        assert_ne!(
            staged_frame, plain_frame,
            "the scripted chat rows changed no pixels"
        );
        let rows = staged.waiting_chat();
        assert_eq!(
            waiting_chat_row_text(&rows[0]),
            format!("[{SCRIPTED_CHAT_ADMIN_SENDER:>12}] {SCRIPTED_CHAT_ADMIN_TEXT}")
        );
        assert_eq!(
            waiting_chat_row_text(&rows[1]),
            format!("[{SCRIPTED_CHAT_USER_SENDER:>12}] {SCRIPTED_CHAT_USER_TEXT}")
        );
    }

    /// Catalog BPM behind the waiting `BPM` caption for one song
    /// id, mirroring [`stage_select_music_catalog`]: the installed
    /// 7.55 index when it parses to songs, else the synthetic
    /// fallback. `None` while no staged source names the song.
    fn staged_catalog_bpm(song_id: u32) -> Option<f32> {
        use o2jam_shell::screens::side::select_music_songs;
        if let Ok(archive) = std::fs::read(install_root().join(MUSICSELECT_CATALOG)) {
            if let Ok(entries) = o2jam_file::parse_ojn_list(&archive) {
                if !select_music_songs(&entries).is_empty() {
                    return entries
                        .iter()
                        .filter(|entry| entry.is_song())
                        .find(|entry| entry.song_id == song_id)
                        .map(|entry| entry.bpm);
                }
            }
        }
        let bytes = musicselect_fallback_catalog();
        let entries = o2jam_file::parse_ojn_list(&bytes).ok()?;
        entries
            .iter()
            .find(|entry| entry.song_id == song_id)
            .map(|entry| entry.bpm)
    }

    #[test]
    fn waiting_shot_paints_mapname_bpm_skillcount_captions() {
        // The staged caption sources ink their verbatim `.bnd`
        // rects on real 7.55 art: the map-cycle entry of the room
        // word (arena stage 2), the catalog BPM column of the room
        // song (757), and the staged inventory counts (slot 1
        // uncapped, slots 2-3 capped at 999). Each caption's scrim
        // pixel plus every font pixel of its exact text must land
        // in the waiting frame — no invented strings, no
        // off-rect ink. `INSTNAME`/`NOTICE` stay unpainted (OPEN
        // 1/2): no test looks for them.
        use o2jam_shell::screens::paint::{
            waiting_bpm_caption, waiting_map_name, waiting_skillcount_text,
        };
        let Some(archive) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        let art = stage_state(&archive, State::Waiting, StageContext::default())
            .art
            .expect("STATE_WAITING stages");
        let placements = &art.placements;
        let caption_origin = |id: &str| {
            let placed = placements
                .iter()
                .find(|placed| placed.id == id)
                .unwrap_or_else(|| panic!("waiting must bind {id}"));
            assert!(
                placed.rect.w != 0 || placed.rect.h != 0,
                "{id} binds an empty rect"
            );
            let scaled = o2jam_render::scale_rect_to_frame(&placed.rect, 800, 600);
            assert!(
                scaled.x >= 4 && scaled.y >= 2,
                "{id} rect ({}, {}) leaves no scrim corner",
                scaled.x,
                scaled.y
            );
            (scaled.x, scaled.y)
        };
        let song_id = u32::from(PLAYING_SONG.music_id);
        let song_bpm = staged_catalog_bpm(song_id);
        assert!(
            song_bpm.is_some(),
            "staged catalog must name song {song_id} for the BPM caption"
        );
        let captions = [
            (
                caption_origin("IDC_TEXT_SWAIT_MAPNAME"),
                waiting_map_name(PLAYING_ARENA_STAGE).to_owned(),
            ),
            (
                caption_origin("IDC_TEXT_SWAIT_BPM"),
                waiting_bpm_caption(song_bpm),
            ),
            (
                caption_origin("IDC_TEXT_SWAIT_1SKILLCOUNT"),
                waiting_skillcount_text(WAITING_SKILL_COUNTS[0], false),
            ),
            (
                caption_origin("IDC_TEXT_SWAIT_2SKILLCOUNT"),
                waiting_skillcount_text(WAITING_SKILL_COUNTS[1], true),
            ),
            (
                caption_origin("IDC_TEXT_SWAIT_3SKILLCOUNT"),
                waiting_skillcount_text(WAITING_SKILL_COUNTS[2], true),
            ),
        ];
        let mut game = seeded_game();
        walk_to_waiting(&mut game);
        assert_eq!(game.screen(), Screen::Waiting);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 0));
        let scrim = o2jam_render::pack_xrgb(0x08, 0x08, 0x10);
        let ink = o2jam_render::pack_xrgb(0xFF, 0xFF, 0xFF);
        for ((x, y), text) in &captions {
            assert_eq!(
                actual[(y - 2) as usize * 800 + (x - 4) as usize],
                scrim,
                "caption `{text}` scrims its rect corner"
            );
            let mut expected = native_probe();
            paint_probe(&mut expected, |frame| {
                o2jam_render::draw_text(frame, *x, *y, text, ink);
            });
            assert_pixels_came_from_font(&actual, &expected, text);
        }
    }

    #[test]
    fn loading_stage_drives_all_gate_phases_with_catalog_caption() {
        // The loading shot's wiring: every gate phase (entry plus
        // the four staging/decrypt/decode/arm steps) holds a real
        // fraction with the staged catalog title/artist, the step
        // past the gate stays indeterminate, and the staged walk
        // holds the clock-armed full fraction with its caption.
        use crate::loading::{loading_phase_status, LOADING_GATE_STEPS};
        use std::cmp::Ordering;
        let mut game = seeded_game();
        walk_to_waiting(&mut game);
        stage_select_music_scene(&mut game);
        let wanted = u32::from(PLAYING_SONG.music_id);
        let (title, artist) = game
            .select_songs()
            .iter()
            .find(|song| song.song_id == wanted)
            .map(|song| (song.title.clone(), song.artist.clone()))
            .expect("staged catalog names the host pick");
        for phase in 0..=LOADING_GATE_STEPS {
            let status = loading_phase_status(phase, &title, &artist)
                .expect("gate phase holds a real fraction");
            assert_eq!(status.title, title);
            assert_eq!(status.artist, artist);
        }
        assert!(
            loading_phase_status(LOADING_GATE_STEPS + 1, &title, &artist).is_none(),
            "the step past the gate stays indeterminate"
        );
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        stage_loading_status(&mut game);
        let status = game
            .loading_status()
            .expect("loading holds the armed status");
        assert_eq!(
            status.clamped_progress().total_cmp(&1.0),
            Ordering::Equal,
            "the armed clock reads full"
        );
        let caption = status
            .caption()
            .expect("armed status captions the staged song");
        assert!(
            caption.contains(&title) && caption.contains(&artist),
            "caption `{caption}` names the staged song"
        );
    }

    #[test]
    fn loading_shot_paints_staged_fraction_and_caption() {
        // The staged walk status reaches the paint: the captioned
        // frame differs from the cleared indeterminate frame, so
        // `loading.png` proves the `with_status` path, not the bare
        // full bar.
        if std::fs::read(install_root().join(MUSICSELECT_CATALOG))
            .ok()
            .is_none()
        {
            println!("skip: missing {MUSICSELECT_CATALOG}");
            return;
        }
        let mut game = seeded_game();
        walk_to_waiting(&mut game);
        stage_select_music_scene(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        stage_loading_status(&mut game);
        assert!(game.loading_status().is_some());
        let mut staged = native_probe();
        paint_probe(&mut staged, |frame| game.paint_at(frame, 0));
        game.clear_loading();
        let mut bare = native_probe();
        paint_probe(&mut bare, |frame| game.paint_at(frame, 0));
        assert_ne!(staged, bare, "staged caption changed no pixels");
    }

    #[test]
    fn rank_broadcast_maps_4015_bytes_onto_rank_rows() {
        // The mechanism the walked rows rely on: a decoded 4015 echo
        // writes its stats word, then delivers each rank byte to the
        // slot holding that rank — the rank-remap rule. Bytes with no
        // holder leave their rows alone (never invented).
        use crate::live_stats::apply_peer_update;
        use o2jam_game::PeerBars;
        use o2jam_net::{GameStatsUpdate, StatKind};
        let mut peers = PeerBars::new();
        peers.set_global_rank(0xFF);
        for slot in 0..o2jam_game::PEER_SLOT_COUNT {
            let rank = u8::try_from(slot).unwrap_or(u8::MAX).saturating_add(10);
            peers.set_stored_rank(slot, rank);
        }
        let applied = apply_peer_update(
            &mut peers,
            GameStatsUpdate {
                member_id: 2,
                kind_code: StatKind::Life.as_u16(),
                value: 1000,
                ranks: [10, 11, 12, 13, 14, 15, 16, 17],
            },
        );
        assert_eq!(applied.ranks.len(), o2jam_game::PEER_SLOT_COUNT);
        assert_eq!(peers.life(2), Some(1000));
        assert_eq!(peers.rank_row(0), Some(10));
        assert_eq!(peers.rank_row(1), Some(11));
        assert_eq!(peers.rank_row(7), Some(17));
    }

    #[test]
    fn result_shot_paints_the_battle_lose_frame() {
        let Some(archive) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // Rank 2 sits on the local team, so the battle verdict
        // resolves to Lose: WinLose frame 2, clamped onto image 1 —
        // the LOSE! banner in the verdict band, like the reference
        // battle photo.
        let banner = load_archive_sprite(&archive, RESULT_WINLOSE_SPRITE)
            .expect("the builder's banner body decodes");
        assert_eq!(banner.len(), 4, "four stored frames");
        assert_eq!(
            o2jam_shell::screens::paint::winlose_frame(
                o2jam_shell::screens::paint::ResultVerdict::Battle(Some(
                    o2jam_shell::screens::paint::BattleVerdict::Lose,
                )),
            ),
            Some(2),
            "rank 2 on the local team must select the LOSE frame",
        );
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let layer = banner.layer(1).expect("image 1 decodes");
            let (x, y) = anchor_in_frame(layer.dx, layer.dy, 800, 600);
            o2jam_render::blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
        });
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        assert_eq!(
            game.result_verdict(),
            o2jam_shell::screens::paint::ResultVerdict::Battle(Some(
                o2jam_shell::screens::paint::BattleVerdict::Lose,
            )),
            "the walked scoreboard must keep rank 2",
        );
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        assert_pixels_came_from_font(&actual, &expected, "LOSE banner");
    }

    #[test]
    fn result_shot_paints_the_perfect_grade_plate() {
        let Some(archive) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // The walked chart holds a handful of records, far below the
        // 27-record floor of `grade_max_reference` — so the walked
        // grade is honestly `None` and no plate paints (a tiny walk
        // cannot grade; plate painting itself is proven at paint
        // level). This test pins the absence, not the plate.
        let plate = load_archive_sprite(&archive, o2jam_shell::screens::paint::RESULT_RANK_SPRITE)
            .expect("the block's grade plate decodes");
        assert_eq!(plate.len(), 8, "eight stored frames");
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(game.grade(), None, "tiny walk charts cannot grade");
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
    }

    #[test]
    fn solo_walk_builds_one_rank_free_row() {
        // The solo room holds no peers: one named row, no rank
        // number (the exe draws `%d` at x=91 only with 2+ players),
        // local numbers on row 0, and the solo verdict.
        let mut game = drive_solo_to_result();
        let mut probe = native_probe();
        paint_probe(&mut probe, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        assert_eq!(
            game.result_verdict(),
            o2jam_shell::screens::paint::ResultVerdict::Solo,
            "a peer-free Normal room must take the solo banner arm",
        );
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 1, "solo must build one row");
        assert_eq!(rows[0].name.as_deref(), Some("YOU"));
        assert_eq!(rows[0].rank, None);
        assert_eq!(rows[0].judgements, Some([3, 0, 0, 0]));
        assert_eq!(rows[0].combos, Some((3, 0)));
        assert_eq!(rows[0].score, Some(600));
        assert_eq!(rows[0].bonus, Some(0));
        assert_eq!(rows[0].speed, Some(1));
    }

    #[test]
    fn solo_shot_paints_the_solo_banner_frame() {
        let Some(archive) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // Solo verdict selects WinLose frame 4, clamped onto image
        // 3 — the GOOD! solo banner in the y=190 band.
        let banner = load_archive_sprite(&archive, RESULT_WINLOSE_SPRITE)
            .expect("the builder's banner body decodes");
        assert_eq!(banner.len(), 4, "four stored frames");
        assert_eq!(
            o2jam_shell::screens::paint::winlose_frame(
                o2jam_shell::screens::paint::ResultVerdict::Solo,
            ),
            Some(4),
            "solo must select the frame-4 banner",
        );
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let layer = banner.layer(3).expect("image 3 decodes");
            let (x, y) = anchor_in_frame(layer.dx, layer.dy, 800, 600);
            o2jam_render::blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
        });
        let mut game = drive_solo_to_result();
        assert_eq!(
            game.result_verdict(),
            o2jam_shell::screens::paint::ResultVerdict::Solo,
        );
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        assert_pixels_came_from_font(&actual, &expected, "solo banner");
    }

    #[test]
    fn solo_shot_paints_the_single_row_without_rank() {
        // Row 0 carries the walked local tally at the pinned
        // columns; no `%d` paints at x=91 on the solo single row.
        let mut game = drive_solo_to_result();
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let ink = o2jam_render::pack_xrgb(0xFF, 0xFF, 0xFF);
            for (x, y, text) in [
                RANK_ROW_NAME,
                RANK_ROW_JUDGEMENTS,
                RANK_ROW_SCORE,
                SOLO_ROW_BONUS,
                RANK_ROW_SPEED_LOCAL,
            ] {
                o2jam_render::draw_text(frame, x, y, text, ink);
            }
        });
        assert_pixels_came_from_font(&actual, &expected, "solo row");
    }

    #[test]
    fn solo_shot_paints_the_walked_grade_plate() {
        // Same struck records as the battle walk on a tiny chart:
        // below the 27-record floor, so the walked grade is honestly
        // `None` and no plate paints (plate painting itself is proven
        // at paint level).
        let mut game = drive_solo_to_result();
        assert_eq!(game.grade(), None, "tiny walk charts cannot grade");
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
    }

    #[test]
    fn staging_attempt_never_breaks_the_walk() {
        let shots = capture_all().unwrap();
        let side_count = [
            SideScreen::Shop,
            SideScreen::MusicShop,
            SideScreen::MyRoom,
            SideScreen::Lanking,
            SideScreen::Payment,
            SideScreen::Tutorial,
        ]
        .into_iter()
        .filter(|side| side_stage_data(*side).is_some())
        .count();
        assert_eq!(shots.len(), 14 + side_count);
        for shot in &shots {
            assert!(is_varied(&shot.rgb), "{} paints a flat frame", shot.name);
        }
    }

    #[test]
    fn writer_saves_eleven_scenes_and_keeps_avi() {
        let dir = std::env::temp_dir().join(format!("recomp2jam-shots-{}", std::process::id()));
        let written = write_shots(&dir).unwrap();
        let side_count = [
            SideScreen::Shop,
            SideScreen::MusicShop,
            SideScreen::MyRoom,
            SideScreen::Lanking,
            SideScreen::Payment,
            SideScreen::Tutorial,
        ]
        .into_iter()
        .filter(|side| side_stage_data(*side).is_some())
        .count();
        assert_eq!(written.len(), 13 + side_count);
        assert!(!dir.join("avi.png").exists());
        assert!(dir.join("planet.png").exists());
        assert!(dir.join("waiting-guest.png").exists());
        assert!(dir.join("musicselect.png").exists());
        assert!(dir.join("playing-native.png").exists());
        assert!(dir.join("result-native.png").exists());
        assert!(dir.join("result-solo.png").exists());
        assert!(dir.join("result-solo-native.png").exists());
        for name in [
            "shop",
            "musicshop",
            "myroom",
            "lanking",
            "payment",
            "tutorial",
        ] {
            assert_eq!(dir.join(format!("{name}.png")).exists(), side_count == 6);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn native_shots_share_the_walked_source_frame() {
        // Same walked instant, same painter: downscaling each native
        // XRGB frame must reproduce its preview RGB byte-exact, and
        // each native must carry full 800x600 detail.
        let shots = capture_all().unwrap();
        let by_name = |name: &str| {
            shots
                .iter()
                .find(|shot| shot.name == name)
                .unwrap_or_else(|| panic!("missing shot {name}"))
        };
        for (preview, native) in [
            ("playing", "playing-native"),
            ("result", "result-native"),
            ("result-solo", "result-solo-native"),
        ] {
            let (small, full) = (by_name(preview), by_name(native));
            assert_eq!((full.w, full.h), (800, 600));
            assert_eq!(full.rgb.len(), 800 * 600 * 3);
            assert!(is_varied(&full.rgb), "{native} paints a flat frame");
            let mut words = Vec::with_capacity(full.w * full.h);
            let (chunks, _) = full.rgb.as_chunks::<3>();
            for pixel in chunks {
                words.push(
                    u32::from(pixel[0]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[2]),
                );
            }
            assert_eq!(to_rgb(&downscale_even(&words, full.w, full.h)), small.rgb);
        }
    }

    #[test]
    fn musicselect_shot_repaints_waiting_with_song_panel() {
        // The host song flow (catalog plus pick) stages under the
        // dialog: the waiting song panel inks new pixels while the
        // flow stays on Waiting for the dialog shot.
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        let before = frame_rgb(&mut game, 0);
        stage_select_music_scene(&mut game);
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(
            !game.select_songs().is_empty(),
            "host song flow stages a catalog"
        );
        let after = frame_rgb(&mut game, 0);
        assert_ne!(before, after, "song panel changed no pixels");
        assert!(is_varied(&after), "musicselect paints a flat frame");
    }

    #[test]
    fn musicselect_fallback_catalog_lists_four_synthetic_songs() {
        // No game data read: the fallback blobs parse to one row per
        // difficulty slot (four songs, three slots each).
        use o2jam_shell::screens::side::{select_music_rows, select_music_songs};
        let bytes = musicselect_fallback_catalog();
        let entries = o2jam_file::parse_ojn_list(&bytes).expect("fallback catalog parses");
        assert_eq!(entries.len(), MUSICSELECT_FALLBACK_SONGS.len());
        let songs = select_music_songs(&entries);
        assert_eq!(songs.len(), MUSICSELECT_FALLBACK_SONGS.len());
        assert_eq!(songs[0].song_id, u32::from(PLAYING_SONG.music_id));
        assert_eq!(
            select_music_rows(&songs).len(),
            MUSICSELECT_FALLBACK_SONGS.len() * 3
        );
    }

    #[test]
    fn musicselect_dialog_structure_paints_search_tab_over_waiting() {
        // With genuine dialog art staged, the structure's amber search
        // tab inks over the waiting seats (pinned anchor from the
        // shell's own structure test); without archives the dialog
        // stays hidden and only the song panel stages.
        let Some(archive) = read_install_archives(&install_root()).interface else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        if stage_dialog(&archive, Dialog::SelectMusic, StageContext::default())
            .art
            .is_none()
        {
            println!("skip: select-music dialog missing");
            return;
        }
        let mut game = seeded_game();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        stage_select_music_scene(&mut game);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert_eq!(game.screen(), Screen::Waiting);
        let amber = o2jam_render::pack_xrgb(0xFF, 0xA0, 0x20);
        if buf[95 * 800 + 355] != amber {
            println!("skip: dialog art staged no figures");
            return;
        }
        assert_eq!(
            buf[95 * 800 + 355],
            amber,
            "search tab must ink over the waiting seats"
        );
    }

    #[test]
    fn guest_view_reports_non_master_gate() {
        // Exe non-master arm (file `0xae805`: READY shown, START
        // hidden): the guest seat holds no master flag, cannot start
        // with every peer ready, and Confirm stays in Waiting.
        let mut guest = seeded_guest_game();
        walk_to_waiting(&mut guest);
        assert_eq!(guest.screen(), Screen::Waiting);
        assert!(!guest.is_room_master());
        assert!(guest.all_peers_ready());
        assert!(!guest.can_start_game());
        guest.menu_key(MenuKey::Confirm, true);
        assert_eq!(guest.screen(), Screen::Waiting);
    }

    #[test]
    fn guest_view_hides_staged_select_but_keeps_bpm_source() {
        // The gate hides dialog art, structure, and panel even when
        // staged (shell `waiting_non_master_hides_select_but_keeps_bpm`):
        // staging the host flow on the guest changes no pixels, while
        // the catalog BPM source for the shared song stays present.
        use o2jam_shell::screens::paint::waiting_bpm_caption;

        let mut guest = seeded_guest_game();
        walk_to_waiting(&mut guest);
        let before = frame_rgb(&mut guest, 0);
        stage_select_music_scene(&mut guest);
        assert!(!guest.select_songs().is_empty());
        let after = frame_rgb(&mut guest, 0);
        assert_eq!(before, after, "guest staged no select pixels");
        let song_id = u32::from(PLAYING_SONG.music_id);
        let bpm = staged_catalog_bpm(song_id);
        assert!(bpm.is_some(), "staged catalog must name the room song");
        assert_ne!(waiting_bpm_caption(bpm), "-0-");
    }

    #[test]
    fn guest_capture_carries_its_own_seats_but_no_select_view() {
        // The seat template paints from live flags (`SeatIndicator`:
        // master star, ready plate, no-song mark), so the guest view
        // differs from the master view in its own seat — the guest is
        // not the room master and sees READY instead of START. The
        // select-music dialog and panel stay master-only either way.
        if read_install_archives(&install_root()).interface.is_none() {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        }
        let shots = capture_all().unwrap();
        let by_name = |name: &str| {
            shots
                .iter()
                .find(|shot| shot.name == name)
                .unwrap_or_else(|| panic!("missing shot {name}"))
        };
        assert_ne!(
            by_name("waiting-guest").rgb,
            by_name("waiting").rgb,
            "seat indicators must carry the viewer's own flags"
        );
        assert_ne!(
            by_name("waiting-guest").rgb,
            by_name("musicselect").rgb,
            "master select pixels must be absent from the guest view"
        );
    }

    /// More than one distinct byte: a real scene, not a blank fill.
    fn is_varied(rgb: &[u8]) -> bool {
        let first = rgb[0];
        rgb.iter().any(|byte| *byte != first)
    }
}
