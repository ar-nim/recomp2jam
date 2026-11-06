//! In-process scene screenshots: drive [`ScreenGame`] with scripted
//! `menu_key`/`lane_key` calls, rasterize each scene with `paint_at`,
//! and encode PNGs with the std-only writer below. No OS key
//! injection, no window.

use std::path::{Path, PathBuf};

use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
use o2jam_render::Frame;
use o2jam_shell::{GameLoop, MenuKey, Screen, ScreenGame};

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
    game.menu_key(MenuKey::Confirm, true);
    shots.push(shoot_checked(&mut game, "playing", Screen::Playing, 4_500)?);
    game.strike_at(2, 5_000);
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

/// Game preloaded with list rows, one waiting member, and a chart
/// whose note lands at 5 s (visible in Playing, spent by Result).
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
    game
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

/// Waiting room with a single ready host in seat 0.
fn waiting_room() -> JoinedRoom {
    JoinedRoom {
        member_id: 0,
        team: 1,
        title: "Jam".to_owned(),
        music_id: 0,
        arena: 0,
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

/// Paint the current screen into a fresh RGB buffer.
fn frame_rgb(game: &mut ScreenGame, now_ms: u64) -> Vec<u8> {
    let mut buf = vec![0u32; SHOT_W * SHOT_H];
    game.paint_at(
        &mut Frame {
            buf: &mut buf,
            w: SHOT_W,
            h: SHOT_H,
        },
        now_ms,
    );
    to_rgb(&buf)
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
