//! Windowed-session walk: live online play with per-stage screenshots.
//!
//! Headless framebuffer walk (no window, no OS input): it runs the
//! same [`crate::session::OnlineSession`] wiring `play --window
//! --online` builds (authorize, channel, room, Waiting, START,
//! select, loading, playing ticks, tally, rank, result) against a
//! live Memoryer gateway and rasterizes each stage with
//! [`ScreenGame::paint_at`] into `docs/screenshots/session/`.
//! Xvfb or headless buffers only — never the real display.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use o2jam_file::{Bnd, ControlList, OJN_LEVEL_COUNT};
use o2jam_net::play::{MusicListAnswer, RoomArenaEcho};
use o2jam_net::{
    GameRankUpdate, GameStatsUpdate, GatewayConnection, Gender, Request, Response, ScoreCompleted,
    ScoreSubmission, WireMessage,
};
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

use crate::connect::{connect_and_login_with_transport, ConnectOptions};
use crate::session::{OnlineSession, SongChoice};
use crate::shots::{encode_png, SHOT_H, SHOT_W};

/// Walk configuration (one named bundle, no flag-argument soup).
#[derive(Debug, Clone)]
pub struct WalkConfig {
    /// Gateway host.
    pub host: String,
    /// Gateway TCP port.
    pub port: u16,
    /// Account id.
    pub user_id: i32,
    /// Session token (gateway session guid).
    pub token: String,
    /// Gateway id to enter.
    pub gateway_id: i16,
    /// Channel id to enter.
    pub channel_id: i16,
    /// Host song pick.
    pub music_id: u16,
    /// Chart difficulty code.
    pub difficulty: u8,
    /// Scroll-speed code.
    pub speed: u8,
    /// Room stage word.
    pub arena_id: i32,
    /// Screenshot directory.
    pub out_dir: PathBuf,
}

impl WalkConfig {
    /// Local-Memoryer defaults (the handoff rig).
    #[must_use]
    pub fn local(out_dir: PathBuf, token: String) -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 15010,
            user_id: 1,
            token,
            gateway_id: 0,
            channel_id: 0,
            music_id: 757,
            difficulty: 2,
            speed: 1,
            arena_id: 1,
            out_dir,
        }
    }
}

/// One captured stage: its PNG stem plus the opcodes exchanged.
#[derive(Debug, Clone)]
pub struct WalkStage {
    /// PNG stem (`01-connect-avi`, …).
    pub name: &'static str,
    /// Opcodes exchanged to reach the stage.
    pub opcodes: Vec<u16>,
}

/// Ordered stage stems (the screenshot story).
pub const STAGE_NAMES: [&str; 11] = [
    "01-connect-avi",
    "02-channel",
    "03-room",
    "04-waiting",
    "05-ready-start",
    "06-song-select",
    "07-loading",
    "08-playing",
    "09-submit",
    "10-rank",
    "11-result",
];

/// Drive the full windowed-session walk and write one PNG per stage.
///
/// # Errors
///
/// Whatever the gateway transport, the loading gate, or the
/// filesystem reports (ERRORS-01/02: `Result` + `?`, never `unwrap`
/// per ERRORS-06).
pub fn drive_walk(config: &WalkConfig) -> Result<Vec<WalkStage>> {
    let (report, gateway) = connect_transport(config)?;
    let mut session = build_session(config, gateway)?;
    stage_lobby_art(session.game_mut());
    stage_live_lists(&mut session, &report);
    std::fs::create_dir_all(&config.out_dir)
        .with_context(|| format!("creating screenshot dir {}", config.out_dir.display()))?;
    clear_stage_shots(&config.out_dir)?;
    let mut stages = Vec::with_capacity(STAGE_NAMES.len());
    paint_stage(&mut session, STAGE_NAMES[0], 0, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[0],
        opcodes: vec![Request::Authorize as u16, Response::Authorize as u16],
    });
    session.lane_key(3, true);
    paint_stage(&mut session, STAGE_NAMES[1], 0, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[1],
        opcodes: vec![
            Request::GetChannelList as u16,
            Response::GetChannelList as u16,
            Request::ChannelLogin as u16,
            Response::ChannelLogin as u16,
        ],
    });
    session.menu_key(MenuKey::Confirm, true);
    paint_stage(&mut session, STAGE_NAMES[2], 0, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[2],
        opcodes: vec![Request::GetChannelInfo as u16, Response::RoomList as u16],
    });
    host_walk_room(&mut session)?;
    session.menu_key(MenuKey::Confirm, true);
    require_screen(&session, Screen::Waiting, "room enter")?;
    paint_stage(&mut session, STAGE_NAMES[3], 0, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[3],
        opcodes: vec![
            Request::CreateRoom as u16,
            Response::CreateRoom as u16,
            Request::GetChannelInfo as u16,
            Response::RoomList as u16,
        ],
    });
    run_song_select(&mut session, config)?;
    stage_performance_art(session.game_mut());
    paint_stage(&mut session, STAGE_NAMES[4], 0, &config.out_dir)?;
    stage_select_music_art(session.game_mut());
    paint_stage(&mut session, STAGE_NAMES[5], 0, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[4],
        opcodes: vec![
            Request::SetRoomMusic as u16,
            Request::SetRoomArena as u16,
            RoomArenaEcho::COMMAND,
            Request::GetMusicList as u16,
            MusicListAnswer::COMMAND,
        ],
    });
    stages.push(WalkStage {
        name: STAGE_NAMES[5],
        opcodes: vec![Request::SetRoomMusic as u16],
    });
    send_start_and_drain(&mut session)?;
    paint_stage(&mut session, STAGE_NAMES[6], 0, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[6],
        opcodes: vec![Request::StartGame as u16, start_broadcast_id()],
    });
    confirm_loaded(&mut session)?;
    strike_playing(&mut session);
    drain_playing_ticks(&mut session);
    paint_stage(
        &mut session,
        STAGE_NAMES[7],
        PLAYING_NOW_MS,
        &config.out_dir,
    )?;
    stages.push(WalkStage {
        name: STAGE_NAMES[7],
        opcodes: vec![
            Request::ConfirmMusicLoaded as u16,
            Request::UpdateGameStats as u16,
            GameStatsUpdate::COMMAND,
        ],
    });
    finish_chart(&mut session, config.music_id, config.difficulty)?;
    drain_rank_table(&mut session);
    paint_stage(&mut session, STAGE_NAMES[8], RESULT_NOW_MS, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[8],
        opcodes: vec![Request::SubmitScore as u16],
    });
    drain_rank_table(&mut session);
    paint_stage(&mut session, STAGE_NAMES[9], RESULT_NOW_MS, &config.out_dir)?;
    stages.push(WalkStage {
        name: STAGE_NAMES[9],
        opcodes: vec![
            ScoreSubmission::COMMAND,
            ScoreCompleted::COMMAND,
            GameRankUpdate::COMMAND,
        ],
    });
    paint_stage(
        &mut session,
        STAGE_NAMES[10],
        RESULT_NOW_MS,
        &config.out_dir,
    )?;
    stages.push(WalkStage {
        name: STAGE_NAMES[10],
        opcodes: vec![Request::FinalizeRank as u16, Request::ExitPlaying as u16],
    });
    Ok(stages)
}

/// Remove prior stage PNGs so a partial walk never masquerades
/// as a complete story (only `<out_dir>/*.png` is touched).
fn clear_stage_shots(out_dir: &Path) -> Result<()> {
    let entries = std::fs::read_dir(out_dir)
        .with_context(|| format!("listing screenshot dir {}", out_dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| "listing screenshot dir entry")?;
        let path = entry.path();
        let is_shot = path.extension().is_some_and(|ext| ext == "png");
        if is_shot {
            std::fs::remove_file(&path)
                .with_context(|| format!("clearing stale shot {}", path.display()))?;
        }
    }
    Ok(())
}

/// Fail loudly when the shell missed its stage (a wrong-screen
/// screenshot would fake the story).
fn require_screen(session: &OnlineSession, want: Screen, step: &str) -> Result<()> {
    let got = session.screen();
    if got == want {
        return Ok(());
    }
    anyhow::bail!("{step}: at {got:?}, want {want:?}")
}

/// Authorize plus list-only login (channels and rooms staged later).
fn connect_transport(
    config: &WalkConfig,
) -> Result<(crate::connect::ConnectReport, GatewayConnection)> {
    let options = ConnectOptions {
        host: config.host.clone(),
        port: config.port,
        user_id: config.user_id,
        token: config.token.clone(),
        gender: Gender::Any,
        relay_key1: 0,
        relay_key2: 0,
        channel: Some((config.gateway_id, config.channel_id)),
        character: false,
        avatar_opa: None,
        join_room: None,
        room_password: String::new(),
        song: None,
    };
    connect_and_login_with_transport(&options).context("windowed walk login")
}

/// Pair the game with its gateway and remember the host pick.
fn build_session(config: &WalkConfig, gateway: GatewayConnection) -> Result<OnlineSession> {
    let mut session = OnlineSession::new(ScreenGame::new(), gateway);
    stage_catalog(&mut session);
    session.set_local_user(&config.token);
    session.set_pending_song(SongChoice {
        music_id: config.music_id,
        difficulty: config.difficulty,
        speed: config.speed,
        arena_id: config.arena_id,
    });
    Ok(session)
}

/// Stage live channels, rooms, and the select caption into the game.
fn stage_live_lists(session: &mut OnlineSession, report: &crate::connect::ConnectReport) {
    session.game_mut().set_channels(report.channels.clone());
    session.game_mut().set_rooms(report.rooms.clone());
}

/// Installed `OJNList.dat` rows (missing files keep the dialog row-less).
fn stage_catalog(session: &mut OnlineSession) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Image/OJNList.dat");
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let Ok(entries) = o2jam_file::parse_ojn_list(&bytes) else {
        return;
    };
    session.set_song_catalog(&entries);
}

/// Interface archive under `install-root` (Avi/Room/Waiting/Result art).
const INTERFACE_ARCHIVE: &str = "O2Jam/Image/Interface1.opi";
/// Playing archive under `install-root` (Loading/Playing art).
const PLAYING_ARCHIVE: &str = "O2Jam/Image/Playing1.opi";
/// Interface control list entry.
const INTERFACE_LIST: &str = "ControlList_Interface.txt";
/// Playing control list entry.
const PLAYING_LIST: &str = "ControlList_Playing.txt";

/// Repository-local `install-root` (git-ignored game data).
fn install_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Read one archive read-only, if present (`None` keeps bare paint).
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

/// Stage a bound block: named figures plus `.bnd` placements. The room
/// arena stage rides along so `_` runtime refs resolve their
/// stage-numbered siblings (`None` keeps the bare arm). The room mode,
/// when given, selects the Playing family members; unselected members
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

/// Stage one sprite full-canvas (the Loading gate).
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
/// builder-mounted verdict banner body (`Result_WinLose.ojs` — the
/// block itself carries only the background, the each-result box,
/// the buttons, and the grade plate), so the scene figures carry
/// the banner and the painter selects its frame. A missing body
/// keeps the scene banner-less.
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
/// stage the room mode's members. The walk rooms play solo, so the
/// room's own team word selects the members.
fn stage_playing_archive(game: &mut ScreenGame, archive: &[u8]) {
    if let Some((placements, figures)) = stage_single_sprite(archive, LOADING_SPRITE) {
        game.load_scene(Screen::Loading, placements, figures);
    }
    let Some(list) = control_list(archive, PLAYING_LIST) else {
        return;
    };
    let mode = PlayingArtMode {
        key_mode: game.room_key_mode(),
        team_play: game.room_team_play(),
    };
    let Some((placements, figures)) = stage_bound_block(
        archive,
        &list,
        PLAYING_BLOCK,
        Some(game.arena_stage()),
        Some(&mode),
    ) else {
        return;
    };
    game.load_scene(Screen::Playing, placements, figures);
    if let Some(skin) = PlaySkin::from_archive(archive) {
        game.set_play_skin(skin);
    }
}

/// Art-less `STATE_CHANNEL`: stage the empty scene so the walk takes
/// the staged path instead of the flat fallback.
fn stage_empty_channel_scene(game: &mut ScreenGame) {
    game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
}

/// Stage lobby scenes (Avi splash, Room, Waiting, Result, plus the
/// art-less Channel staged path) before the first paint, so every
/// lobby shot layers live UI over genuine archive art. Missing or
/// unreadable archives keep the bare paint; never fails.
fn stage_lobby_art(game: &mut ScreenGame) {
    if let Some(archive) = read_archive(INTERFACE_ARCHIVE) {
        stage_interface_archive(game, &archive);
    }
    stage_empty_channel_scene(game);
}

/// Stage performance scenes (Loading gate plus Playing gauges) once
/// the joined room names its arena stage and key mode, so the venue
/// and gauge families resolve through the walk's own room word.
/// Missing or unreadable archives keep the bare paint; never fails.
fn stage_performance_art(game: &mut ScreenGame) {
    if let Some(archive) = read_archive(PLAYING_ARCHIVE) {
        stage_playing_archive(game, &archive);
    }
}

/// Stage real song-select dialog art (`DAILOG_SELECTMUSIC` placements
/// plus figures) over the waiting seats for the song-select shot;
/// empty art paints nothing. Missing archives keep the bare paint;
/// never fails.
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

/// Create a room and join it back for the full snapshot.
fn host_walk_room(session: &mut OnlineSession) -> Result<()> {
    let title = format!("walk{}", std::process::id() % 1000);
    session.create_and_join_room(&title)?;
    Ok(())
}

/// Run select plus the chart fetch (the host Confirm shape).
fn run_song_select(session: &mut OnlineSession, config: &WalkConfig) -> Result<()> {
    let choice = SongChoice {
        music_id: config.music_id,
        difficulty: config.difficulty,
        speed: config.speed,
        arena_id: config.arena_id,
    };
    session.select_song(choice)?;
    session.note_arena_selected(choice.arena_id);
    let level = usize::from(config.difficulty).min(OJN_LEVEL_COUNT - 1);
    session.fetch_and_stage_chart(config.music_id, level)?;
    Ok(())
}

/// START broadcast id (4011, the host Confirm answer).
fn start_broadcast_id() -> u16 {
    4011
}

/// Send START through the Waiting Confirm gate, then drain the 4011
/// broadcast until the shell reaches Loading. The broadcast follows
/// the send by a server round-trip, far past the 1 ms per-frame
/// drain window, so this polls with rests instead of one paint.
///
/// # Errors
///
/// No Loading within the poll window (the START never took).
fn send_start_and_drain(session: &mut OnlineSession) -> Result<()> {
    session.menu_key(MenuKey::Confirm, true);
    for _ in 0..START_POLLS {
        drain_frames(session);
        if session.screen() == Screen::Loading {
            return Ok(());
        }
        std::thread::sleep(START_REST);
    }
    anyhow::bail!("START broadcast did not advance to Loading")
}

/// START poll rounds before giving up (each round drains + rests).
const START_POLLS: usize = 60;
/// Rest between START polls (the 4011 round-trip window).
const START_REST: std::time::Duration = std::time::Duration::from_millis(50);

/// Confirm the loaded music and enter Playing.
///
/// # Errors
///
/// No Playing after the loaded Confirm (the 4012 never took).
fn confirm_loaded(session: &mut OnlineSession) -> Result<()> {
    session.menu_key(MenuKey::Confirm, true);
    if session.screen() != Screen::Playing {
        session.game_mut().menu_key(MenuKey::Confirm, true);
    }
    require_screen(session, Screen::Playing, "loaded confirm")
}

/// Mid-song paint instant (the shots walk strikes the 5 s tap here).
const PLAYING_NOW_MS: u64 = 5_100;

/// Strike two lanes at the paint instant (hit bursts over the live chart).
fn strike_playing(session: &mut OnlineSession) {
    session.game_mut().strike_at(2, PLAYING_NOW_MS);
    session.game_mut().strike_at(3, PLAYING_NOW_MS);
}

/// Far-future clock: spends the chart so the paint reaches Result.
const RESULT_NOW_MS: u64 = 10_000_000;

/// Spend the chart (far-future paint advances Playing to Result).
///
/// Strikes every onset at its own clock first: `is_finished`
/// needs every note judged, and the far-future paint alone never
/// judges the untouched tail. Exact-onset strikes rate Cool, so the
/// tally submits the honestly played count sheet.
///
/// # Errors
///
/// Unreadable chart bytes, a rejected loading gate, or a chart that
/// ignored the far-future clock (still Playing).
fn finish_chart(session: &mut OnlineSession, music_id: u16, difficulty: u8) -> Result<()> {
    let struck = strike_full_chart(session, music_id, difficulty)?;
    tracing::info!(struck, "walk chart spent");
    let mut buf = vec![0u32; 800 * 600];
    let mut frame = Frame {
        buf: &mut buf,
        w: 800,
        h: 600,
    };
    session.game_mut().paint_at(&mut frame, RESULT_NOW_MS);
    require_screen(session, Screen::Result, "chart finish")
}

/// Strike every chart onset at its own clock (the autoplay shape:
/// exact-onset strikes rate Cool through the verified windows).
/// Returns the judged note count.
///
/// # Errors
///
/// Unreadable chart bytes or a rejected loading gate.
fn strike_full_chart(session: &mut OnlineSession, music_id: u16, difficulty: u8) -> Result<usize> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../install-root/O2Jam/Music")
        .join(format!("o2ma{music_id}.ojn"));
    let bytes =
        std::fs::read(&path).with_context(|| format!("reading walk chart {}", path.display()))?;
    let level = usize::from(difficulty).min(OJN_LEVEL_COUNT - 1);
    let prepared = crate::loading::prepare_chart(&bytes, level)
        .with_context(|| format!("loading gate rejected {}", path.display()))?;
    let events = o2jam_file::parse_ojn_level_events(&bytes, level)
        .with_context(|| format!("walking level events {}", path.display()))?;
    let chart = o2jam_game::build_level_chart(prepared.bpm, &events);
    let onsets = chart.hit_onsets_ms();
    let judged = onsets.len();
    for (lane, at_ms, _) in onsets {
        session.game_mut().strike_at(usize::from(lane), at_ms);
    }
    Ok(judged)
}
/// drains broadcasts without touching the screenshot clock).
fn drain_frames(session: &mut OnlineSession) {
    let mut buf = vec![0u32; 800 * 600];
    let mut frame = Frame {
        buf: &mut buf,
        w: 800,
        h: 600,
    };
    session.paint(&mut frame);
}

/// Poll Playing until the tick echo folds into the peer mirror (the
/// server answers our 4014s with 4015 after a round-trip, far past
/// one 1 ms paint drain). Quiet rooms keep the local-only mirror.
fn drain_playing_ticks(session: &mut OnlineSession) {
    for _ in 0..TICK_POLLS {
        drain_frames(session);
        if session.screen() != Screen::Playing {
            break;
        }
        std::thread::sleep(TICK_REST);
    }
}

/// Poll Result until the ranked table lands (the server computes
/// rewards between our submit and its 4018 broadcast). A quiet
/// table keeps the local-only result; the warn says so.
fn drain_rank_table(session: &mut OnlineSession) {
    for _ in 0..RANK_POLLS {
        drain_frames(session);
        if !session.peer_ranks().is_empty() {
            return;
        }
        std::thread::sleep(RANK_REST);
    }
    tracing::warn!("rank table not observed; result stays local-only");
}

/// Playing tick poll rounds (each round drains + rests).
const TICK_POLLS: usize = 20;
/// Rest between tick polls (the 4015 round-trip window).
const TICK_REST: std::time::Duration = std::time::Duration::from_millis(50);
/// Rank poll rounds (each round drains up to 100 ms + rests).
const RANK_POLLS: usize = 40;
/// Rest between rank polls (the tally compute window).
const RANK_REST: std::time::Duration = std::time::Duration::from_millis(100);

/// Paint one stage at `now_ms` and write `<out_dir>/<name>.png`.
fn paint_stage(
    session: &mut OnlineSession,
    name: &'static str,
    now_ms: u64,
    out_dir: &Path,
) -> Result<PathBuf> {
    let rgb = shot_rgb(session, now_ms)?;
    let width = u32::try_from(SHOT_W).map_err(|_| anyhow::anyhow!("shot width overflows u32"))?;
    let height = u32::try_from(SHOT_H).map_err(|_| anyhow::anyhow!("shot height overflows u32"))?;
    let bytes = encode_png(width, height, &rgb);
    let path = out_dir.join(format!("{name}.png"));
    std::fs::write(&path, &bytes)
        .with_context(|| format!("writing stage shot {}", path.display()))?;
    Ok(path)
}

/// Native-paint plus 2:1 downscale triplets for one stage instant.
fn shot_rgb(session: &mut OnlineSession, now_ms: u64) -> Result<Vec<u8>> {
    let w = usize::try_from(NATIVE_WIDTH).map_err(|_| anyhow::anyhow!("native width unknown"))?;
    let h = usize::try_from(NATIVE_HEIGHT).map_err(|_| anyhow::anyhow!("native height unknown"))?;
    let mut buf = vec![0u32; w * h];
    session.game_mut().paint_at(
        &mut Frame {
            buf: &mut buf,
            w,
            h,
        },
        now_ms,
    );
    Ok(to_rgb(&downscale_even(&buf, w, h)))
}

/// 2x2 box average (native 800x600 over shot 400x300).
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
            out.push(
                (u32::try_from(sum[0] / 4).unwrap_or(u32::MAX) << 16)
                    | (u32::try_from(sum[1] / 4).unwrap_or(u32::MAX) << 8)
                    | u32::try_from(sum[2] / 4).unwrap_or(u32::MAX),
            );
        }
    }
    out
}

/// Strip XRGB words down to RGB triplets.
fn to_rgb(buf: &[u32]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(buf.len() * 3);
    for pixel in buf {
        rgb.push(u8::try_from(pixel >> 16 & 0xFF).unwrap_or(0xFF));
        rgb.push(u8::try_from(pixel >> 8 & 0xFF).unwrap_or(0xFF));
        rgb.push(u8::try_from(*pixel & 0xFF).unwrap_or(0xFF));
    }
    rgb
}

#[cfg(test)]
mod session_walk_tests {
    use super::*;

    #[test]
    fn stage_names_cover_every_windowed_step() {
        assert_eq!(STAGE_NAMES.len(), 11);
        assert_eq!(STAGE_NAMES[0], "01-connect-avi");
        assert_eq!(STAGE_NAMES[3], "04-waiting");
        assert_eq!(STAGE_NAMES[7], "08-playing");
        assert_eq!(STAGE_NAMES[10], "11-result");
    }

    #[test]
    fn local_defaults_match_the_handoff_rig() {
        let config = WalkConfig::local(PathBuf::from("/tmp/walk"), "tester".to_owned());
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 15010);
        assert_eq!(config.user_id, 1);
        assert_eq!(config.gateway_id, 0);
        assert_eq!(config.channel_id, 0);
        assert_eq!(config.music_id, 757);
    }

    #[test]
    fn staging_without_readable_archives_keeps_the_flow() {
        // Archive-agnostic: with or without install-root, every
        // staging entry never fails and never walks the flow.
        let mut game = ScreenGame::new();
        stage_lobby_art(&mut game);
        stage_performance_art(&mut game);
        stage_select_music_art(&mut game);
        assert_eq!(game.screen(), Screen::Avi);
    }

    #[test]
    fn garbage_archive_bytes_stage_nothing() {
        // Every fallback layer rejects undecodable bytes, so a
        // corrupt archive keeps the bare paint, never partial art.
        let garbage = vec![0xFFu8; 64];
        assert!(entry_body(&garbage, "nope.ojs").is_none());
        assert!(control_list(&garbage, INTERFACE_LIST).is_none());
        assert!(bind_data(&garbage, "State_Waiting.bnd").is_none());
        assert!(stage_single_sprite(&garbage, LOADING_SPRITE).is_none());
    }

    #[test]
    fn walk_staging_resolves_bound_scenes_from_real_archives() {
        let Some(interface) = read_archive(INTERFACE_ARCHIVE) else {
            eprintln!("skipping walk staging (missing {INTERFACE_ARCHIVE})");
            return;
        };
        let Some(playing) = read_archive(PLAYING_ARCHIVE) else {
            eprintln!("skipping walk staging (missing {PLAYING_ARCHIVE})");
            return;
        };
        let list = control_list(&interface, INTERFACE_LIST).expect("interface list parses");
        for block in [AVI_BLOCK, ROOM_BLOCK, WAITING_BLOCK, RESULT_BLOCK] {
            let staged = stage_bound_block(&interface, &list, block, None, None);
            assert!(
                staged.is_some_and(
                    |(placements, figures)| !placements.is_empty() && !figures.is_empty()
                ),
                "{block} stages placements plus figures"
            );
        }
        let list = control_list(&playing, PLAYING_LIST).expect("playing list parses");
        let mode = PlayingArtMode {
            key_mode: 1,
            team_play: false,
        };
        let staged = stage_bound_block(&playing, &list, PLAYING_BLOCK, Some(1), Some(&mode));
        assert!(
            staged
                .is_some_and(|(placements, figures)| !placements.is_empty() && !figures.is_empty()),
            "STATE_PLAYING stages placements plus figures"
        );
        assert!(stage_single_sprite(&playing, LOADING_SPRITE).is_some());
    }

    #[test]
    fn downscale_halves_a_flat_field() {
        let flat = vec![0x000A_141Eu32; 16];
        assert_eq!(downscale_even(&flat, 4, 4), vec![0x000A_141Eu32; 4]);
    }

    #[test]
    fn rgb_strips_one_xrgb_word() {
        assert_eq!(to_rgb(&[0x12_34_56]), vec![0x12, 0x34, 0x56]);
    }

    /// Guarded: runs only with `O2JAM_LIVE_GATEWAY=host:port` and
    /// `O2JAM_LIVE_TOKEN=session-guid`. Drives the whole
    /// windowed-session walk and asserts one PNG per stage.
    #[test]
    fn walk_reaches_result_against_local_gateway() {
        let address = std::env::var("O2JAM_LIVE_GATEWAY").unwrap_or_default();
        if address.is_empty() {
            eprintln!("skipping session walk (set O2JAM_LIVE_GATEWAY=host:port)");
            return;
        }
        let token =
            std::env::var("O2JAM_LIVE_TOKEN").expect("O2JAM_LIVE_TOKEN must hold a session guid");
        let dir = std::env::temp_dir().join(format!("walk-test-{}", std::process::id()));
        let config = WalkConfig::local(dir.clone(), token);
        let stages = drive_walk(&config).expect("live session walk");
        assert_eq!(stages.len(), STAGE_NAMES.len());
        for stage in &stages {
            assert!(dir.join(format!("{}.png", stage.name)).is_file());
            assert!(!stage.opcodes.is_empty());
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
