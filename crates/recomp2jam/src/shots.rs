//! In-process scene screenshots: drive [`ScreenGame`] with scripted
//! `menu_key`/`lane_key` calls, rasterize each scene with `paint_at`,
//! and encode PNGs with the std-only writer below. No OS key
//! injection, no window.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use o2jam_file::{Bnd, ControlList};
use o2jam_game::{RankDelivery, RoomTeams, LIFE_MAX, PEER_SLOT_COUNT, STAT_LIFE};
use o2jam_net::play::SetRoomMusic;
use o2jam_net::rooms::{WaitingAdminMessage, WaitingUserMessage};
use o2jam_net::{ChannelState, GameMode, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::{Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::avatar::{
    bind_scene_rects, load_archive_sprite, load_avi_logo, load_scene_figures,
    load_scene_figures_for_room, load_scene_figures_for_stage, scene_block, scene_sprite_names,
    AvatarFigure, PlayingArtMode, ScenePlacement, AVI_BLOCK, LOADING_SPRITE, RESULT_BLOCK,
    ROOM_BLOCK, WAITING_BLOCK,
};
use o2jam_shell::screens::paint::{PLAYING_BLOCK, RESULT_WINLOSE_SPRITE};
use o2jam_shell::screens::side::SELECT_MUSIC_DIALOG_BLOCK;
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
/// # Errors
///
/// When the scripted walk leaves the expected screen.
pub fn capture_all() -> anyhow::Result<Vec<Shot>> {
    let mut game = seeded_game();
    let mut shots = Vec::with_capacity(13);
    shots.push(shoot(&mut game, "avi", 0)?);
    game.lane_key(3, true);
    shots.push(shoot_checked(&mut game, "channel", Screen::Channel, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    shots.push(shoot_checked(&mut game, "room", Screen::Room, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    shots.push(shoot_checked(&mut game, "waiting", Screen::Waiting, 0)?);
    shots.push(shoot_guest_waiting()?);
    stage_select_music_scene(&mut game);
    shots.push(shoot_checked(&mut game, "musicselect", Screen::Waiting, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    stage_loading_status(&mut game);
    shots.push(shoot_checked(&mut game, "loading", Screen::Loading, 0)?);
    stage_playing_scene(&mut game);
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
const INTERFACE_ARCHIVE: &str = "O2Jam/Image/Interface1.opi";
/// Playing archive under `install-root` (Loading/Playing art).
const PLAYING_ARCHIVE: &str = "O2Jam/Image/Playing1.opi";
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
/// Interface control list entry.
const INTERFACE_LIST: &str = "ControlList_Interface.txt";
/// Playing control list entry.
const PLAYING_LIST: &str = "ControlList_Playing.txt";

/// `STATE_CHANNEL` names zero sprites on 7.55 (bare `BOUND`, no art),
/// and the code-fetched channel bar (`Dialog_ChannelSelectBar.ojs`)
/// has no pinned dialog host — so shots stage no scene art for
/// Channel. The art-less state paints nothing; only the live list,
/// title, and cursor layer on top (pinned no-fill rule per
/// `docs/channel-fill.md`).
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

/// Read one archive read-only, if present (`None` keeps synthetic staging).
fn read_archive(relative: &str) -> Option<Vec<u8>> {
    std::fs::read(install_root().join(relative)).ok()
}

/// Borrow one entry body by its stored size (never the reserved stride).
fn entry_body<'a>(archive: &'a [u8], entry_name: &str) -> Option<&'a [u8]> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = entries.iter().find(|entry| entry.name == entry_name)?;
    let size = usize::try_from(entry.size).ok()?;
    let end = entry.data_offset.checked_add(size)?;
    archive.get(entry.data_offset..end)
}

/// Parse a `ControlList_*.txt` entry from an archive.
fn control_list(archive: &[u8], entry_name: &str) -> Option<ControlList> {
    let body = entry_body(archive, entry_name)?;
    let text = o2jam_file::decode_control_text(body).ok()?;
    o2jam_file::parse_control_list(&text).ok()
}

/// Parse a `.bnd` layout entry from an archive.
fn bind_data(archive: &[u8], bound_file: &str) -> Option<Bnd> {
    let body = entry_body(archive, bound_file)?;
    o2jam_file::parse_bnd(body).ok()
}

/// Full-canvas placements for art without a `.bnd`.
fn full_canvas(names: &[String]) -> Vec<ScenePlacement> {
    names
        .iter()
        .map(|name| ScenePlacement {
            id: String::new(),
            code: 0,
            sprite: Some(name.clone()),
            rect: Rect {
                x: 0,
                y: 0,
                w: NATIVE_WIDTH,
                h: NATIVE_HEIGHT,
            },
        })
        .collect()
}

/// Stage a bound block: named figures plus `.bnd` placements. The
/// room arena stage rides along so `_` runtime refs resolve their
/// stage-numbered siblings (`None` keeps the bare arm; Interface
/// blocks name no `_` refs). The room mode, when given, selects the
/// Playing family members (note backgrounds, life gauges, BP buffer
/// bars — `avatar::playing_member_selected`); unselected members
/// leave empty slots that paint nothing.
fn stage_bound_block(
    archive: &[u8],
    list: &ControlList,
    block_name: &str,
    arena_stage: Option<u16>,
    mode: Option<&PlayingArtMode>,
) -> Option<(Vec<ScenePlacement>, HashMap<String, AvatarFigure>)> {
    let block = scene_block(list, block_name)?;
    let names = scene_sprite_names(block);
    if names.is_empty() {
        return None;
    }
    let figures = match mode {
        Some(mode) => load_scene_figures_for_room(archive, &names, arena_stage, mode),
        None => load_scene_figures_for_stage(archive, &names, arena_stage),
    };
    if figures.is_empty() {
        return None;
    }
    let bnd = bind_data(archive, block.bound_file.as_ref()?)?;
    let placements = bind_scene_rects(block, &bnd);
    if placements.is_empty() {
        return None;
    }
    Some((placements, figures))
}

/// Stage one sprite full-canvas (Channel fallback, Loading gate).
fn stage_single_sprite(
    archive: &[u8],
    sprite_name: &str,
) -> Option<(Vec<ScenePlacement>, HashMap<String, AvatarFigure>)> {
    let names = vec![sprite_name.to_owned()];
    let figures = load_scene_figures(archive, &names);
    if figures.is_empty() {
        return None;
    }
    Some((full_canvas(&names), figures))
}

/// Stage Interface-archive scenes; each scene stages independently
/// so one bad block never blanks the rest. Channel stages nothing:
/// the art-less state paints no fill under the live list.
fn stage_interface_archive(game: &mut ScreenGame, archive: &[u8]) {
    if let Some(logo) = load_avi_logo(archive) {
        game.load_splash(logo);
    }
    let Some(list) = control_list(archive, INTERFACE_LIST) else {
        return;
    };
    for (screen, block) in [
        (Screen::Avi, AVI_BLOCK),
        (Screen::Room, ROOM_BLOCK),
        (Screen::Waiting, WAITING_BLOCK),
    ] {
        if let Some((placements, figures)) = stage_bound_block(archive, &list, block, None, None) {
            game.load_scene(screen, placements, figures);
        }
    }
    stage_result_block(game, archive, &list);
}

/// Stage the result scene: the `STATE_RESULT` block art plus the
/// builder-mounted verdict banner body (`Result_WinLose.ojs`, file
/// `0xc2390` — the block itself carries only the background, the
/// each-result box, the buttons, and the grade plate), so the scene
/// figures carry the banner and the painter selects its frame. A
/// missing body keeps the scene banner-less.
fn stage_result_block(game: &mut ScreenGame, archive: &[u8], list: &ControlList) {
    let Some((placements, mut figures)) =
        stage_bound_block(archive, list, RESULT_BLOCK, None, None)
    else {
        return;
    };
    if let Some(banner) = load_archive_sprite(archive, RESULT_WINLOSE_SPRITE) {
        figures.insert(RESULT_WINLOSE_SPRITE.to_owned(), banner);
    }
    game.load_scene(Screen::Result, placements, figures);
}

/// Stage Playing-archive scenes (Loading gate plus Playing gauges)
/// through the game's room arena stage and key mode: the venue
/// `_Playing_BG.ojs` and the stage-keyed kits resolve their numbered
/// siblings, and the note-background/life-gauge/buffer-bar families
/// stage only the solo member the reference photo pins (lanes left
/// panel, venue right, avatar center-right). The walk room stays a
/// Live duo for the battle verdict, whose P2P family would paint its
/// VS panel over the center while emptying the solo slots — so the
/// shot pins `team_play: false` instead of the room's Live word.
fn stage_playing_archive(game: &mut ScreenGame, archive: &[u8]) {
    if let Some((placements, figures)) = stage_single_sprite(archive, LOADING_SPRITE) {
        game.load_scene(Screen::Loading, placements, figures);
    }
    let Some(list) = control_list(archive, PLAYING_LIST) else {
        return;
    };
    let arena_stage = game.arena_stage();
    let mode = PlayingArtMode {
        key_mode: game.room_key_mode(),
        team_play: false,
    };
    let Some((placements, figures)) = stage_bound_block(
        archive,
        &list,
        PLAYING_BLOCK,
        Some(arena_stage),
        Some(&mode),
    ) else {
        return;
    };
    game.load_scene(Screen::Playing, placements, figures);
    if let Some(skin) = PlaySkin::from_archive(archive) {
        game.set_play_skin(skin);
    }
}

/// Layer genuine 7.55 archive art under every scene when
/// `install-root` is present; missing or unreadable archives keep
/// the synthetic staging. Archives stay read-only, never fail.
fn stage_real_art(game: &mut ScreenGame) {
    if let Some(archive) = read_archive(INTERFACE_ARCHIVE) {
        stage_interface_archive(game, &archive);
    }
    if let Some(archive) = read_archive(PLAYING_ARCHIVE) {
        stage_playing_archive(game, &archive);
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

/// Art-less `STATE_CHANNEL`: stage the empty scene so the walk takes
/// the staged path (`paint_channel_scene` paints no fill under the
/// live list) instead of the flat `paint_channel_list` fallback.
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
fn seeded_game_in(room: JoinedRoom) -> ScreenGame {
    let mut game = ScreenGame::new();
    seed_lobby_lists(&mut game);
    game.set_joined_room(room);
    game.set_battle_teams(result_battle_teams());
    game.set_battle_self_team(Some(RESULT_SELF_TEAM));
    seed_chart(&mut game);
    stage_result_ranks(&mut game);
    stage_peer_gauge(&mut game);
    stage_waiting_captions(&mut game);
    stage_waiting_chat(&mut game);
    stage_real_art(&mut game);
    stage_empty_channel_scene(&mut game);
    game
}

/// Guest waiting instant: the same lane/menu walk as the master shot,
/// from a non-master seat with captions and chat staged but no song
/// selection (the gate hides the dialog), rasterized without checking
/// beyond the Waiting screen.
fn shoot_guest_waiting() -> anyhow::Result<Shot> {
    let mut guest = seeded_guest_game();
    guest.lane_key(3, true);
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
    stage_real_art(&mut game);
    stage_empty_channel_scene(&mut game);
    game
}

/// Drive the solo walk from splash into Result: the same
/// lane/menu/strike script as the battle walk, with the song-pick
/// and playing captions staged at the same points.
fn drive_solo_to_result() -> ScreenGame {
    let mut game = seeded_solo_game();
    game.lane_key(3, true);
    game.menu_key(MenuKey::Confirm, true);
    game.menu_key(MenuKey::Confirm, true);
    stage_select_music_scene(&mut game);
    game.menu_key(MenuKey::Confirm, true);
    stage_playing_scene(&mut game);
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
    if let Some(archive) = read_archive(MUSICSELECT_CATALOG) {
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

/// Stage real song-select dialog art (`DAILOG_SELECTMUSIC` placements
/// plus figures) over the waiting seats; empty art paints nothing.
fn stage_select_music_art(game: &mut ScreenGame) {
    let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
        return;
    };
    let Some(list) = control_list(&archive, INTERFACE_LIST) else {
        return;
    };
    let Some(block) = scene_block(&list, SELECT_MUSIC_DIALOG_BLOCK) else {
        return;
    };
    let names = scene_sprite_names(block);
    if names.is_empty() {
        return;
    }
    let figures = load_scene_figures(&archive, &names);
    if figures.is_empty() {
        return;
    }
    let Some(bound_file) = block.bound_file.as_ref() else {
        return;
    };
    let Some(bnd) = bind_data(&archive, bound_file) else {
        return;
    };
    let placements = bind_scene_rects(block, &bnd);
    if placements.is_empty() {
        return;
    }
    game.load_select_music_scene(placements, figures);
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
    if let Some(archive) = read_archive(AVATAR_ARCHIVE) {
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

/// PNG signature.
const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Encode 8-bit RGB pixels as a PNG (none filter, stored deflate).
pub fn encode_png(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let stride = width as usize * 3;
    let mut raw = Vec::with_capacity(height as usize * (stride + 1));
    for row in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[row * stride..(row + 1) * stride]);
    }
    let mut out = Vec::with_capacity(raw.len() + 64);
    out.extend_from_slice(&PNG_SIG);
    let mut ihdr = [0u8; 13];
    ihdr[0..4].copy_from_slice(&width.to_be_bytes());
    ihdr[4..8].copy_from_slice(&height.to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = 2;
    push_chunk(&mut out, b"IHDR", &ihdr);
    push_chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    push_chunk(&mut out, b"IEND", &[]);
    out
}

/// Wrap raw scanlines in a zlib stream of uncompressed deflate blocks.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    let mut out = Vec::with_capacity(raw.len() + 2 + 5 * blocks.len() + 4);
    out.extend_from_slice(&[0x78, 0x01]);
    for (index, block) in blocks.iter().enumerate() {
        let last = index + 1 == blocks.len();
        out.push(u8::from(last));
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// Append one chunk (length, tag, data, CRC over tag + data).
fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let mut tagged = Vec::with_capacity(4 + data.len());
    tagged.extend_from_slice(tag);
    tagged.extend_from_slice(data);
    out.extend_from_slice(&crc32(&tagged).to_be_bytes());
}

/// IEEE CRC-32, table-free.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Adler-32 checksum.
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut low = 1u32;
    let mut high = 0u32;
    for &byte in data {
        low = (low + u32::from(byte)) % MOD;
        high = (high + low) % MOD;
    }
    (high << 16) | low
}

#[cfg(test)]
mod tests {
    use super::*;
    use o2jam_shell::avatar::{
        anchor_in_frame, load_archive_sprite, paint_foreground_placements, paint_scene_figures,
        playing_member_selected,
    };

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
        for _ in 0..4 {
            game.menu_key(MenuKey::Confirm, true);
        }
        let rgb = frame_rgb(&mut game, 100);
        assert_eq!(rgb.len(), SHOT_W * SHOT_H * 3);
    }

    #[test]
    fn pinned_equipment_stacks_a_worn_figure() {
        let Some(archive) = read_archive(AVATAR_ARCHIVE) else {
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
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let list = control_list(&archive, PLAYING_LIST).expect("control list parses");
        let game = seeded_game();
        assert_eq!(game.arena_stage(), 2);
        let mode = PlayingArtMode {
            key_mode: game.room_key_mode(),
            team_play: game.room_team_play(),
        };
        assert_eq!(mode.key_mode, 2, "the shot's room plays 7 keys");
        assert!(mode.team_play, "the shot's room is a Live duo");
        let Some((_, figures)) = stage_bound_block(
            &archive,
            &list,
            PLAYING_BLOCK,
            Some(game.arena_stage()),
            Some(&mode),
        ) else {
            panic!("playing: STATE_PLAYING staged no figures");
        };
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
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let list = control_list(&archive, PLAYING_LIST).expect("control list parses");
        let block = scene_block(&list, PLAYING_BLOCK).expect("STATE_PLAYING stages");
        let names = scene_sprite_names(block);
        let mode = PlayingArtMode {
            key_mode: 2,
            team_play: false,
        };
        let figures = load_scene_figures_for_room(&archive, &names, Some(2), &mode);
        for staged in ["_Playing_BG.ojs", "Note_BG.ojs", "Life_gauge_left_bar.ojs"] {
            assert!(
                figures.contains_key(staged),
                "{staged} must stage in a solo 7-key room"
            );
        }
        for name in names {
            assert!(
                playing_member_selected(&name, &mode) || !figures.contains_key(&name),
                "unselected member {name} must leave an empty slot"
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
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let list = control_list(&archive, PLAYING_LIST).expect("control list parses");
        let walk = seeded_game();
        assert!(
            walk.room_team_play(),
            "the walk stays Live for the battle verdict"
        );
        let solo = PlayingArtMode {
            key_mode: walk.room_key_mode(),
            team_play: false,
        };
        let Some((placements, figures)) = stage_bound_block(
            &archive,
            &list,
            PLAYING_BLOCK,
            Some(walk.arena_stage()),
            Some(&solo),
        ) else {
            panic!("playing: STATE_PLAYING staged no solo figures");
        };
        assert!(figures.contains_key("Note_BG.ojs"));
        assert!(figures.contains_key("Life_gauge_left_bar.ojs"));
        assert!(!figures.contains_key("P2P7KeyNote_BG.ojs"));
        assert!(!figures.contains_key("BpPoint_Right_bar.ojs"));
        assert!(!figures.contains_key("Life_gauge_back.ojs"));
        let paint_native = |game: &mut ScreenGame| {
            game.lane_key(3, true);
            for _ in 0..4 {
                game.menu_key(MenuKey::Confirm, true);
            }
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
            for _ in 0..4 {
                game.menu_key(MenuKey::Confirm, true);
            }
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
            for _ in 0..4 {
                game.menu_key(MenuKey::Confirm, true);
            }
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
    fn png_carries_signature_and_dimensions() {
        let rgb = vec![255u8; 2 * 3];
        let png = encode_png(2, 1, &rgb);
        assert_eq!(&png[0..8], &PNG_SIG);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..20], [0, 0, 0, 2]);
        assert_eq!(&png[20..24], [0, 0, 0, 1]);
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn png_stored_blocks_round_trip() {
        let mut rgb = vec![0u8; 3 * 2 * 3];
        rgb[0] = 255;
        let png = encode_png(3, 2, &rgb);
        let (w, h, raw) = decode_stored_png(&png);
        assert_eq!((w, h), (3, 2));
        assert_eq!(raw.len(), 2 * (3 * 3 + 1));
        assert_eq!(raw[0], 0);
        assert_eq!(&raw[1..4], &[255, 0, 0]);
    }

    #[test]
    fn walk_covers_all_scenes_plus_solo_result() {
        let shots = capture_all().unwrap();
        let names: Vec<&str> = shots.iter().map(|shot| shot.name).collect();
        assert_eq!(
            names,
            [
                "avi",
                "channel",
                "room",
                "waiting",
                "waiting-guest",
                "musicselect",
                "loading",
                "playing",
                "playing-native",
                "result",
                "result-native",
                "result-solo",
                "result-solo-native"
            ]
        );
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
    /// color, via a 1x1 probe blit (only that pixel survives the
    /// clip). `None` when the body has no decodable layer or no
    /// opaque pixel.
    fn first_opaque_probe(figure: &AvatarFigure) -> Option<(i32, i32, u32)> {
        let layer = figure.layer(0)?;
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
        (probe[0] != 0).then_some((px, py, probe[0]))
    }

    /// Drive every `.bnd`-claimed placement with a decodable body of
    /// one real block through the shared painter, alone on a blank
    /// native frame, and require its first opaque pixel to land at
    /// its own rect origin. Returns the number of entries proven.
    fn bound_entries_paint_on_rects(archive: &[u8], list_entry: &str, block_name: &str) -> usize {
        let Some(list) = control_list(archive, list_entry) else {
            return 0;
        };
        let Some(block) = scene_block(&list, block_name) else {
            return 0;
        };
        let Some(bind_file) = block.bound_file.clone() else {
            return 0;
        };
        let Some(bnd) = bind_data(archive, &bind_file) else {
            return 0;
        };
        let placements = bind_scene_rects(block, &bnd);
        let figures = load_scene_figures(archive, &scene_sprite_names(block));
        let (frame_w, frame_h) = (
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        let mut checked = 0;
        for placed in &placements {
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
            let Some((px, py, ink)) = first_opaque_probe(figure) else {
                continue;
            };
            // Verified draw chain (avatar.rs `placement_view_origin`):
            // anchored bodies draw at their record anchors (frame
            // scaled); anchor-free bodies at their bound rect origins.
            // Most bound bodies duplicate their rect origins byte-exact;
            // `BpPoint_Left_bar.ojs` (2,266) is the photo-pinned
            // disagreeing pair whose anchor wins.
            let expected = if figure.has_anchor_offset() {
                let layer = figure.layer(0).expect("probed figure has a layer");
                let (ax, ay) = anchor_in_frame(layer.dx, layer.dy, frame_w, frame_h);
                (ax, ay)
            } else {
                let scaled = o2jam_render::scale_rect_to_frame(&placed.rect, frame_w, frame_h);
                (scaled.x, scaled.y)
            };
            let mut buf = vec![0u32; frame_w * frame_h];
            paint_scene_figures(
                &mut Frame {
                    buf: &mut buf,
                    w: frame_w,
                    h: frame_h,
                },
                std::slice::from_ref(placed),
                &figures,
            );
            let at = (expected.1 + py) as usize * frame_w + (expected.0 + px) as usize;
            assert_eq!(
                buf[at], ink,
                "{block_name}:{name} must paint at its placement origin"
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
        // their rect origins. The same loop runs the Playing bound
        // run (Playing1.opi) and the Room block (Interface1.opi), so
        // the rule holds per scene and per entry, not for one sprite.
        let mut checked = 0;
        if let Some(archive) = read_archive(PLAYING_ARCHIVE) {
            checked += bound_entries_paint_on_rects(&archive, PLAYING_LIST, PLAYING_BLOCK);
        } else {
            println!("skip: missing {PLAYING_ARCHIVE}");
        }
        if let Some(archive) = read_archive(INTERFACE_ARCHIVE) {
            checked += bound_entries_paint_on_rects(&archive, INTERFACE_LIST, ROOM_BLOCK);
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
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let list = control_list(&archive, PLAYING_LIST).expect("control list parses");
        let block = scene_block(&list, PLAYING_BLOCK).expect("STATE_PLAYING stages");
        let bnd = bind_data(&archive, "State_Playing.bnd").expect("bind file parses");
        let placements = bind_scene_rects(block, &bnd);
        let placed = placements
            .iter()
            .find(|placed| placed.sprite.as_deref() == Some("Normal_TimeBar.ojs"))
            .expect("the bound run stages the TIME bar");
        assert_eq!(placed.rect.x, 489);
        assert_eq!(placed.rect.y, 556);
        assert_eq!(placed.rect.w, 254);
        assert_eq!(placed.rect.h, 10);
        let figures = load_scene_figures(&archive, &scene_sprite_names(block));
        let figure = figures.get("Normal_TimeBar.ojs").expect("body decodes");
        let (px, py, ink) = first_opaque_probe(figure).expect("body has opaque pixels");
        let mut buf = vec![0u32; 800 * 600];
        paint_foreground_placements(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &figures,
        );
        assert_eq!(
            buf[(556 + py as usize) * 800 + (489 + px as usize)],
            ink,
            "TIME-bar art must land at its bound rect origin"
        );
    }

    #[test]
    fn missing_archives_stage_nothing() {
        assert!(read_archive("No/Such.opi").is_none());
        assert!(entry_body(&[], LOADING_SPRITE).is_none());
        assert!(control_list(&[], INTERFACE_LIST).is_none());
        assert!(bind_data(&[], "State_Room.bnd").is_none());
        assert!(stage_single_sprite(&[], LOADING_SPRITE).is_none());
    }

    #[test]
    fn present_archives_stage_key_scenes() {
        let Some(interface) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        assert_channel_block_names_no_sprites(&interface);
        assert!(load_avi_logo(&interface).is_some());
        let Some(playing) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        assert!(stage_single_sprite(&playing, LOADING_SPRITE).is_some());
    }

    /// Pinned 7.55 fact the channel soak relies on: `STATE_CHANNEL`
    /// names zero sprites, so shots stage no scene art for it.
    fn assert_channel_block_names_no_sprites(archive: &[u8]) {
        let list = control_list(archive, INTERFACE_LIST).expect("control list parses");
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
        let mut game = seeded_game();
        game.lane_key(3, true);
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
    fn play_skin_decodes_every_lane_when_present() {
        use o2jam_shell::playing::PlaySkin;
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
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
        assert!(read_archive("No/Such.opi").is_none());
    }

    #[test]
    fn skinned_highway_replaces_procedural_note_ink() {
        use o2jam_render::{pack_xrgb, Frame};
        use o2jam_shell::playing::{HighwayGeometry, LivePlay, PlaySkin};
        const NOTE_BLUE: u32 = pack_xrgb(0x40, 0xC0, 0xFF);
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
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
    /// drives it.
    fn walk_to_playing(game: &mut ScreenGame) {
        game.lane_key(3, true);
        for _ in 0..4 {
            game.menu_key(MenuKey::Confirm, true);
        }
    }

    /// The scripted walk from splash into Waiting, as `capture_all`
    /// drives it (the waiting shot's instant).
    fn walk_to_waiting(game: &mut ScreenGame) {
        game.lane_key(3, true);
        for _ in 0..2 {
            game.menu_key(MenuKey::Confirm, true);
        }
    }

    #[test]
    fn playing_shot_paints_the_staged_combo_digit() {
        use o2jam_shell::playing::HighwayGeometry;
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let Some(skin) = PlaySkin::from_archive(&archive) else {
            panic!("playing: PlaySkin decoded no lanes");
        };
        let font = skin.combo_font().expect("Note_ComboNum.ojs font");
        // Judgement-word combo anchor on the native canvas: centered
        // on the staged lane span, 24 rows above the judgement line
        // (LivePlay::paint_judgement_word).
        let geom = HighwayGeometry::centered(
            usize::try_from(NATIVE_WIDTH).expect("native width fits usize"),
            usize::try_from(NATIVE_HEIGHT).expect("native height fits usize"),
        );
        let span = geom.highway_rect();
        let center_x = span.x + i32::try_from(span.w).expect("span width fits i32") / 2;
        let row = geom.line_y - 24;
        // The exact staged digits the shot paints (tap Cool then hold
        // head Cool: combo 2, judged 100 ms before the paint lands).
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            font.paint_centered(frame, center_x, row, 2)
        });
        // The playing shot's frame at the same instant.
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 5_100));
        assert_eq!(game.screen(), Screen::Playing);
        assert_pixels_came_from_font(&actual, &expected, "combo digit");
    }

    #[test]
    fn playing_shot_paints_the_staged_long_note_caption() {
        // The reference photo's LONGNOTE COMBO caption over the lanes
        // (ink (91,282)-(165,291) game px) stages through the runtime
        // body `Note_LongCombo.ojt` frame 1 and paints over the staged
        // scene art: every plate ink pixel of a lone blit at the
        // pinned origin must appear in the playing shot's frame.
        use o2jam_shell::playing::long_combo_plate_origin;
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
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
        if let Some(archive) = read_archive(MUSICSELECT_CATALOG) {
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
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        let list = control_list(&archive, INTERFACE_LIST).expect("control list parses");
        let block = scene_block(&list, WAITING_BLOCK).expect("STATE_WAITING stages");
        let bound_file = block.bound_file.clone().expect("waiting binds a .bnd");
        let bnd = bind_data(&archive, &bound_file).expect("waiting .bnd parses");
        let placements = bind_scene_rects(block, &bnd);
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
        if read_archive(MUSICSELECT_CATALOG).is_none() {
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
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
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
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // Three Cools across the two records (the hold tail rates as
        // its own judgement, overshooting the stand-in all-Cool
        // denominator), so the walked grade is the 0.95 band's
        // frame 6: image 5 on the plate body.
        let plate = load_archive_sprite(&archive, o2jam_shell::screens::paint::RESULT_RANK_SPRITE)
            .expect("the block's grade plate decodes");
        assert_eq!(plate.len(), 8, "eight stored frames");
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let layer = plate.layer(5).expect("image 5 decodes");
            let (x, y) = anchor_in_frame(layer.dx, layer.dy, 800, 600);
            o2jam_render::blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
        });
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        game.strike_at(0, SCRIPTED_HOLD_STRIKE_MS);
        game.release_at(0, SCRIPTED_HOLD_RELEASE_MS);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        assert_pixels_came_from_font(&actual, &expected, "grade plate");
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
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
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
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // Same struck records as the battle walk, same play data:
        // three Cools across the two records overshoot the stand-in
        // all-Cool denominator, so the walked grade is frame 6
        // (image 5) on the solo walk too.
        let plate = load_archive_sprite(&archive, o2jam_shell::screens::paint::RESULT_RANK_SPRITE)
            .expect("the block's grade plate decodes");
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let layer = plate.layer(5).expect("image 5 decodes");
            let (x, y) = anchor_in_frame(layer.dx, layer.dy, 800, 600);
            o2jam_render::blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
        });
        let mut game = drive_solo_to_result();
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        assert_pixels_came_from_font(&actual, &expected, "solo grade plate");
    }

    #[test]
    fn staging_attempt_never_breaks_the_walk() {
        let shots = capture_all().unwrap();
        assert_eq!(shots.len(), 13);
        for shot in &shots {
            assert!(is_varied(&shot.rgb), "{} paints a flat frame", shot.name);
        }
    }

    #[test]
    fn writer_saves_eleven_scenes_and_keeps_avi() {
        let dir = std::env::temp_dir().join(format!("recomp2jam-shots-{}", std::process::id()));
        let written = write_shots(&dir).unwrap();
        assert_eq!(written.len(), 12);
        assert!(!dir.join("avi.png").exists());
        assert!(dir.join("waiting-guest.png").exists());
        assert!(dir.join("musicselect.png").exists());
        assert!(dir.join("playing-native.png").exists());
        assert!(dir.join("result-native.png").exists());
        assert!(dir.join("result-solo.png").exists());
        assert!(dir.join("result-solo-native.png").exists());
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
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        let Some(list) = control_list(&archive, INTERFACE_LIST) else {
            println!("skip: {INTERFACE_ARCHIVE} has no control list");
            return;
        };
        if scene_block(&list, SELECT_MUSIC_DIALOG_BLOCK).is_none() {
            println!("skip: {SELECT_MUSIC_DIALOG_BLOCK} missing");
            return;
        }
        let mut game = seeded_game();
        game.lane_key(3, true);
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
    fn guest_capture_matches_master_waiting_but_not_select_view() {
        // Artifact verdict: the guest capture replays the master waiting
        // instant seat-for-seat (same captions, chat, and seats — the
        // seat id paints nothing), while the master select view carries
        // the staged dialog and panel on top.
        let shots = capture_all().unwrap();
        let by_name = |name: &str| {
            shots
                .iter()
                .find(|shot| shot.name == name)
                .unwrap_or_else(|| panic!("missing shot {name}"))
        };
        assert_eq!(
            by_name("waiting-guest").rgb,
            by_name("waiting").rgb,
            "guest must replay the master waiting instant"
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

    /// Minimal reader for our own stored-block PNGs (tests only).
    fn decode_stored_png(png: &[u8]) -> (u32, u32, Vec<u8>) {
        assert_eq!(&png[0..8], &PNG_SIG);
        let mut pos = 8;
        let mut dims = (0, 0);
        let mut idat = Vec::new();
        while pos < png.len() {
            let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
            let tag = &png[pos + 4..pos + 8];
            let data = &png[pos + 8..pos + 8 + len];
            if tag == b"IHDR" {
                dims = (
                    u32::from_be_bytes(data[0..4].try_into().unwrap()),
                    u32::from_be_bytes(data[4..8].try_into().unwrap()),
                );
            } else if tag == b"IDAT" {
                idat.extend_from_slice(data);
            } else if tag == b"IEND" {
                break;
            }
            pos += 12 + len;
        }
        assert_eq!(&idat[0..2], &[0x78, 0x01]);
        let mut raw = Vec::new();
        let mut at = 2;
        loop {
            let last = idat[at] == 1;
            let len = u16::from_le_bytes(idat[at + 1..at + 3].try_into().unwrap()) as usize;
            assert_eq!(
                u16::from_le_bytes(idat[at + 3..at + 5].try_into().unwrap()),
                !len as u16
            );
            raw.extend_from_slice(&idat[at + 5..at + 5 + len]);
            at += 5 + len;
            if last {
                break;
            }
        }
        let adler = u32::from_be_bytes(idat[at..at + 4].try_into().unwrap());
        assert_eq!(adler, adler32(&raw));
        (dims.0, dims.1, raw)
    }
}
