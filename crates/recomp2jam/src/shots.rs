//! In-process scene screenshots: drive [`ScreenGame`] with scripted
//! `menu_key`/`lane_key` calls, rasterize each scene with `paint_at`,
//! and encode PNGs with the std-only writer below. No OS key
//! injection, no window.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use o2jam_file::{Bnd, ControlList};
use o2jam_net::play::SetRoomMusic;
use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::{Frame, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};
use o2jam_shell::avatar::{
    bind_scene_rects, load_archive_sprite, load_avi_logo, load_scene_figures,
    load_scene_figures_for_room, load_scene_figures_for_stage, scene_block, scene_sprite_names,
    AvatarFigure, PlayingArtMode, ScenePlacement, AVI_BLOCK, LOADING_SPRITE, RESULT_BLOCK,
    ROOM_BLOCK, WAITING_BLOCK,
};
use o2jam_shell::screens::paint::{PLAYING_BLOCK, RESULT_WINLOSE_SPRITE};
use o2jam_shell::{GameLoop, MenuKey, PlaySkin, Screen, ScreenGame};

use crate::connect::stack_worn_figure;

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

/// Capture avi, channel, room, waiting, loading, playing, result.
/// The playing shot paints a mid-song state: the 5 s note is struck
/// first (combo 1) and the paint lands 100 ms later, while the
/// judgement word, hit burst, tally counts, and TIME progress show.
///
/// # Errors
///
/// When the scripted walk leaves the expected screen.
pub fn capture_all() -> anyhow::Result<Vec<Shot>> {
    let mut game = seeded_game();
    let mut shots = Vec::with_capacity(7);
    shots.push(shoot(&mut game, "avi", 0)?);
    game.lane_key(3, true);
    shots.push(shoot_checked(&mut game, "channel", Screen::Channel, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    shots.push(shoot_checked(&mut game, "room", Screen::Room, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    shots.push(shoot_checked(&mut game, "waiting", Screen::Waiting, 0)?);
    game.menu_key(MenuKey::Confirm, true);
    shots.push(shoot_checked(&mut game, "loading", Screen::Loading, 0)?);
    stage_playing_scene(&mut game);
    game.menu_key(MenuKey::Confirm, true);
    game.strike_at(2, 5_000);
    shots.push(shoot_checked(&mut game, "playing", Screen::Playing, 5_100)?);
    shots.push(shoot_result(&mut game, 7_000)?);
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
/// Interface control list entry.
const INTERFACE_LIST: &str = "ControlList_Interface.txt";
/// Playing control list entry.
const PLAYING_LIST: &str = "ControlList_Playing.txt";

/// `STATE_CHANNEL` names zero sprites, so shots stage the real
/// channel dialog bar full-canvas instead.
const CHANNEL_FALLBACK_SPRITE: &str = "Dialog_ChannelSelectBar.ojs";
/// Room arena value the playing shot joins with: stage 5, whose
/// `Playing_BG5.ojs` stages the warm wooden-deck floor the avatar
/// stands on — the closest staged match to the reference rooftop
/// photo (`docs/playing-reference.md`; the exact 3D rooftop is not
/// among the 36 stages). Rides the room's arena word so the shot
/// exercises the room-entry stage wiring end to end.
const PLAYING_ARENA_STAGE: i32 = 5;

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
/// so one bad block never blanks the rest.
fn stage_interface_archive(game: &mut ScreenGame, archive: &[u8]) {
    if let Some((placements, figures)) = stage_single_sprite(archive, CHANNEL_FALLBACK_SPRITE) {
        game.load_scene(Screen::Channel, placements, figures);
    }
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
/// through the game's room arena stage and room mode: the venue
/// `_Playing_BG.ojs` and the stage-keyed kits resolve their numbered
/// siblings, and the note-background/life-gauge/ buffer-bar families
/// stage only the member the joined room selects.
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
        team_play: game.room_team_play(),
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

/// Game preloaded with list rows, one waiting member, and a chart
/// whose note lands at 5 s (visible in Playing, spent by Result).
/// Genuine archive art stages underneath when `install-root` holds
/// it; otherwise the synthetic fallback paints.
fn seeded_game() -> ScreenGame {
    let mut game = ScreenGame::new();
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
    game.set_joined_room(waiting_room());
    game.load_chart(vec![(2, 5_000, 7)]);
    stage_real_art(&mut game);
    game
}

/// Stage the playing shot's mid-song scene: the player caption, the
/// worn figure (`avatar.opa`; a missing or unstackable archive keeps
/// no figure), the host song pick whose caption the Playing paint
/// re-syncs into the scene and tally strip, and the volume readout
/// (`VOL 58`, the reference photo's strip readout). Staged after the
/// loading shot so earlier shots stay untouched; never fails.
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

/// Waiting room with a single ready host in seat 0, joined at the
/// reference-matching arena stage (see [`PLAYING_ARENA_STAGE`]).
fn waiting_room() -> JoinedRoom {
    JoinedRoom {
        member_id: 0,
        team: 1,
        title: "Jam".to_owned(),
        music_id: 0,
        arena: PLAYING_ARENA_STAGE,
        key_mode: 2,
        game_mode: 0,
        difficulty: 0,
        speed: 1,
        has_super_manager: false,
        user_count: 2,
        slots: vec![RoomSlot {
            index: 0,
            state_code: 1,
            member: Some(RoomMember {
                nickname: "host".to_owned(),
                level: 9,
                gender: 0,
                gem: 0,
                master: true,
                team: 0,
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

/// Rasterize Result, failing unless the chart end advanced the flow.
fn shoot_result(game: &mut ScreenGame, now_ms: u64) -> anyhow::Result<Shot> {
    let rgb = frame_rgb(game, now_ms);
    if game.screen() != Screen::Result {
        anyhow::bail!("shot result: still at {:?}", game.screen());
    }
    Ok(Shot {
        name: "result",
        w: SHOT_W,
        h: SHOT_H,
        rgb,
    })
}

/// Paint the current screen at the native 800x600 canvas — the exe
/// frame every staged record anchor lives on — and box-downscale to
/// the shot size, exactly how the reference photo photographs the
/// native screen (a 4:3 view of one 800x600 frame, never a stretch
/// or a partial blit).
fn frame_rgb(game: &mut ScreenGame, now_ms: u64) -> Vec<u8> {
    let (w, h) = (
        usize::try_from(NATIVE_WIDTH).unwrap_or(1),
        usize::try_from(NATIVE_HEIGHT).unwrap_or(1),
    );
    let mut buf = vec![0u32; w * h];
    game.paint_at(
        &mut Frame {
            buf: &mut buf,
            w,
            h,
        },
        now_ms,
    );
    to_rgb(&downscale_even(&buf, w, h))
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
        // The joined room's arena word (stage 5) drives the venue
        // resolution: the staged `_Playing_BG.ojs` figure is the
        // `Playing_BG5.ojs` body, not the bare stem's.
        let Some(archive) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        let list = control_list(&archive, PLAYING_LIST).expect("control list parses");
        let game = seeded_game();
        assert_eq!(game.arena_stage(), 5);
        let mode = PlayingArtMode {
            key_mode: game.room_key_mode(),
            team_play: game.room_team_play(),
        };
        assert_eq!(mode.key_mode, 2, "the shot's room plays 7 keys");
        assert!(!mode.team_play, "the shot's room is solo");
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
            &load_archive_sprite(&archive, "Playing_BG5.ojs").expect("stage 5 body decodes"),
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
        let figures = load_scene_figures_for_room(&archive, &names, Some(5), &mode);
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
        let staged_rgb = frame_rgb(&mut staged, 5_100);
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
    fn walk_covers_seven_distinct_scenes() {
        let shots = capture_all().unwrap();
        let names: Vec<&str> = shots.iter().map(|shot| shot.name).collect();
        assert_eq!(
            names,
            ["avi", "channel", "room", "waiting", "loading", "playing", "result"]
        );
        for shot in &shots {
            assert_eq!((shot.w, shot.h), (SHOT_W, SHOT_H));
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
        assert!(entry_body(&[], CHANNEL_FALLBACK_SPRITE).is_none());
        assert!(control_list(&[], INTERFACE_LIST).is_none());
        assert!(bind_data(&[], "State_Room.bnd").is_none());
        assert!(stage_single_sprite(&[], CHANNEL_FALLBACK_SPRITE).is_none());
        assert!(stage_single_sprite(&[], LOADING_SPRITE).is_none());
    }

    #[test]
    fn present_archives_stage_key_scenes() {
        let Some(interface) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        assert!(stage_single_sprite(&interface, CHANNEL_FALLBACK_SPRITE).is_some());
        assert!(load_avi_logo(&interface).is_some());
        let Some(playing) = read_archive(PLAYING_ARCHIVE) else {
            println!("skip: missing {PLAYING_ARCHIVE}");
            return;
        };
        assert!(stage_single_sprite(&playing, LOADING_SPRITE).is_some());
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
        // The exact staged digits the shot paints (one Cool strike:
        // combo 1, judged 100 ms before the paint lands).
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            font.paint_centered(frame, center_x, row, 1)
        });
        // The playing shot's frame at the same instant.
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
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
    fn result_shot_paints_the_pinned_digits() {
        // The digits are GDI text (no staged body): the scripted walk
        // spends the one-note chart with one Cool — counts 1/0/0/0,
        // combo 1, jam level 0, 200 points — at the seven pinned
        // fields (docs/cresult-anchors.md).
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let ink = o2jam_render::pack_xrgb(0xFF, 0xFF, 0xFF);
            o2jam_render::draw_number(frame, 151, 125, 1, ink);
            o2jam_render::draw_number(frame, 240, 125, 0, ink);
            o2jam_render::draw_number(frame, 337, 125, 0, ink);
            o2jam_render::draw_number(frame, 426, 125, 0, ink);
            o2jam_render::draw_number(frame, 505, 130, 1, ink);
            o2jam_render::draw_number(frame, 596, 130, 0, ink);
            o2jam_render::draw_number(frame, 605, 533, 200, ink);
        });
        assert_pixels_came_from_font(&actual, &expected, "pinned result digits");
    }

    #[test]
    fn result_shot_paints_the_solo_verdict_frame() {
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // Solo play draws WinLose frame 4, clamped onto image 3 —
        // the 547x65 solo banner anchored (126,190).
        let banner = load_archive_sprite(&archive, RESULT_WINLOSE_SPRITE)
            .expect("the builder's banner body decodes");
        assert_eq!(banner.len(), 4, "four stored frames");
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let layer = banner.layer(3).expect("image 3 decodes");
            let (x, y) = anchor_in_frame(layer.dx, layer.dy, 800, 600);
            o2jam_render::blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
        });
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        assert_pixels_came_from_font(&actual, &expected, "solo banner");
    }

    #[test]
    fn result_shot_paints_the_perfect_grade_plate() {
        let Some(archive) = read_archive(INTERFACE_ARCHIVE) else {
            println!("skip: missing {INTERFACE_ARCHIVE}");
            return;
        };
        // One Cool equals the all-Cool max, so the pinned
        // equal-to-max arm gives grade 8: image 7, the 79x41 frame
        // anchored (395,531).
        let plate = load_archive_sprite(&archive, o2jam_shell::screens::paint::RESULT_RANK_SPRITE)
            .expect("the block's grade plate decodes");
        assert_eq!(plate.len(), 8, "eight stored frames");
        let mut expected = native_probe();
        paint_probe(&mut expected, |frame| {
            let layer = plate.layer(7).expect("image 7 decodes");
            let (x, y) = anchor_in_frame(layer.dx, layer.dy, 800, 600);
            o2jam_render::blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
        });
        let mut game = seeded_game();
        walk_to_playing(&mut game);
        game.strike_at(2, 5_000);
        let mut actual = native_probe();
        paint_probe(&mut actual, |frame| game.paint_at(frame, 7_000));
        assert_eq!(game.screen(), Screen::Result);
        assert_pixels_came_from_font(&actual, &expected, "grade plate");
    }

    #[test]
    fn staging_attempt_never_breaks_the_walk() {
        let shots = capture_all().unwrap();
        assert_eq!(shots.len(), 7);
        for shot in &shots {
            assert!(is_varied(&shot.rgb), "{} paints a flat frame", shot.name);
        }
    }

    #[test]
    fn writer_saves_six_scenes_and_keeps_avi() {
        let dir = std::env::temp_dir().join(format!("recomp2jam-shots-{}", std::process::id()));
        let written = write_shots(&dir).unwrap();
        assert_eq!(written.len(), 6);
        assert!(!dir.join("avi.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
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
