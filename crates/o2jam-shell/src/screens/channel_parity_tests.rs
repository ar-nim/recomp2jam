//! Channel-scene parity guards against the qualified 7.55 frame.
//!
//! The exe frame is local-only evidence; these guards pin the staged
//! geometry the qualified frame exercises. `CHANNEL_PARITY_OUT` lets a
//! developer write one native frame for the pair-SSIM script without
//! adding a screenshot path to the shipped client.

use std::collections::HashMap;

use o2jam_file::data_root::install_root;
use o2jam_net::ChannelState;
use o2jam_render::{pack_xrgb, Frame, Rgb555Image};

use super::super::game::ScreenGame;
use super::{paint_channel_scene, SceneArt, CHANNEL_BAR_SPRITE, DIALOG_BAR_SPRITE};
use crate::avatar::{load_archive_sprite_for_stage, AvatarFigure, FigureLayer};
use crate::screen::Screen;
use crate::staging::{
    read_install_archives, stage_dialog, stage_state, Dialog, StageContext, State,
};
use crate::{GameLoop, InputSnapshot, MenuKey};

const CHANNEL_PARITY_OUT: &str = "CHANNEL_PARITY_OUT";

/// Native canvas dimensions as indices. PERF-07: no silent narrowing.
fn native_dimensions() -> (usize, usize) {
    (
        usize::try_from(o2jam_render::NATIVE_WIDTH).expect("native canvas width fits usize"),
        usize::try_from(o2jam_render::NATIVE_HEIGHT).expect("native canvas height fits usize"),
    )
}

/// The four open channels the qualified exe frame shows: ids 0..3,
/// empty populations, all active. The row label is wire `channel_id + 1`
/// (D25 pin in `docs/parity/channel.md`).
fn qualified_channels() -> Vec<ChannelState> {
    (0..4)
        .map(|channel_id| ChannelState {
            gateway_id: 0,
            channel_id,
            capacity: 100,
            population: 0,
            active: true,
        })
        .collect()
}

/// Board art and dialog art staged exactly as the live walk stages
/// them: the bound Planet block under the code-mounted dialog sheet
/// and select bar. `None` means the install root has no archives.
fn staged_channel_frame() -> Option<ScreenGame> {
    let installed = read_install_archives(&install_root());
    let interface = installed.interface.as_deref()?;
    let board = stage_state(interface, State::Planet, StageContext::default()).art?;
    let mut dialog_figures = stage_dialog(interface, Dialog::ChannelInfo, StageContext::default())
        .art?
        .figures;
    let select_bar = load_archive_sprite_for_stage(interface, DIALOG_BAR_SPRITE, None)?;
    dialog_figures.insert(DIALOG_BAR_SPRITE.to_owned(), select_bar);

    let mut game = ScreenGame::new();
    game.load_scene(Screen::Planet, board.placements, board.figures);
    game.load_scene(Screen::Channel, Vec::new(), dialog_figures);
    game.menu_key(MenuKey::Back, true);
    assert_eq!(game.screen(), Screen::Planet);
    // The qualified driver opened the dialog through the lit HARD
    // emblem and last hovered Ch.03 before parking the pointer. Walk
    // those same pointer edges (#876 hold-hover pin); Up/Down are
    // exe-pinned no-ops (#189), so they cannot arm the row.
    game.click_planet(386, 88);
    assert_eq!(game.screen(), Screen::Channel);
    game.set_channels(qualified_channels());
    game.ingest_snapshot(&InputSnapshot::click_at(300, 340));
    assert_eq!(
        game.highlighted_channel().map(|row| row.channel_id),
        Some(2)
    );
    Some(game)
}

/// Paint one native frame and write it only when the developer sets
/// `CHANNEL_PARITY_OUT` — tests stay side-effect-free in CI.
fn paint_native_frame(game: &mut ScreenGame) -> Vec<u8> {
    let (width, height) = native_dimensions();
    let mut pixels = vec![0u32; width * height];
    game.paint_at(
        &mut Frame {
            buf: &mut pixels,
            w: width,
            h: height,
        },
        0,
    );
    let mut rgb = Vec::with_capacity(pixels.len() * 3);
    for pixel in pixels {
        rgb.push(((pixel >> 16) & 0xFF) as u8);
        rgb.push(((pixel >> 8) & 0xFF) as u8);
        rgb.push((pixel & 0xFF) as u8);
    }
    if let Ok(path) = std::env::var(CHANNEL_PARITY_OUT) {
        let encoded = o2jam_render::png::encode_png(
            o2jam_render::NATIVE_WIDTH,
            o2jam_render::NATIVE_HEIGHT,
            &rgb,
        );
        std::fs::write(path, encoded).expect("developer-chosen parity shot path is writable");
    }
    rgb
}

/// Assert one sampled exe-frame chrome pixel. RGB555 art can differ
/// by two to three levels per channel after decode, so the expected
/// color uses a small tolerance; the position itself is exact.
fn assert_exe_anchor(
    label: &str,
    rgb: &[u8],
    width: usize,
    (x, y): (usize, usize),
    expected: [u8; 3],
    tolerance: u8,
) {
    let offset = y * width * 3 + x * 3;
    let actual = [rgb[offset], rgb[offset + 1], rgb[offset + 2]];
    for (channel, (got, want)) in actual.into_iter().zip(expected).enumerate() {
        assert!(
            got.abs_diff(want) <= tolerance,
            "{label}: channel {channel} read {got}, exe frame reads {want}"
        );
    }
}

#[test]
fn qualified_channel_frame_pins_staged_chrome_at_exe_anchors() {
    // Live caller chain:
    //   main.rs::run_play -> ScreenGame::paint -> paint_at
    //   -> ScreenGame::paint_channel -> paint_channel_scene
    //   -> paint_channel_rows -> paint_channel_scroll_art / paint_channel_highlight.
    // Exe frame: paired-shots-v3/states/channel/exe.png, SHA-256
    // a96aee2684e434dc0f0aa169ce474b6a012d519fbb2ab0eef430b3356a7393c8.
    // Archives absent -> skip cleanly (CI has no game data).
    let Some(mut game) = staged_channel_frame() else {
        eprintln!(
            "SKIP qualified_channel_frame_pins_staged_chrome_at_exe_anchors: install-root absent"
        );
        return;
    };
    assert_eq!(game.channel_rows().len(), 4);
    let (width, height) = native_dimensions();
    let rgb = paint_native_frame(&mut game);
    assert_eq!(rgb.len(), width * height * 3);

    // Sampled from the exe frame, not from our painter:
    // scroll track (`Dialog_ChannelInfo.bnd` ord 11), hover bar
    // (live-measured (216,row top)), sheet metal, EXIT disc.
    assert_exe_anchor("scroll track", &rgb, width, (580, 300), [41, 40, 41], 2);
    assert_exe_anchor("green hover bar", &rgb, width, (216, 340), [140, 186, 0], 3);
    assert_exe_anchor("dialog sheet", &rgb, width, (400, 200), [156, 154, 156], 2);
    assert_exe_anchor("exit disc", &rgb, width, (760, 580), [230, 162, 0], 4);
}

/// One solid body standing in for `ChannelInfo_CH_Back.ojs`: every
/// pixel is opaque green so a staged row band is unambiguous in the
/// framebuffer.
fn solid_bar_figure() -> AvatarFigure {
    const BAR_W: usize = 348;
    const BAR_H: usize = 41;
    let pixels = 0x03E0_u16.to_le_bytes().repeat(BAR_W * BAR_H);
    let mut figure = AvatarFigure::default();
    figure.push_layer(FigureLayer {
        image: Rgb555Image::decode(&pixels, BAR_W, BAR_H).expect("solid bar decodes"),
        dx: 0,
        dy: 0,
        colorkey: 0xFFFF,
    });
    figure
}

#[test]
fn four_staged_channel_rows_paint_all_bnd_row_bands() {
    // seam: this is a pure staged-geometry consumer — the row table
    // and bar mount are pinned in `docs/channel-fill.md` (bnd rects
    // 2..5 and the live memory read), and no runtime caller supplies
    // a different row count. Guard 1 names the live chain.
    let art = SceneArt {
        placements: Vec::new(),
        figures: HashMap::from([(CHANNEL_BAR_SPRITE.to_owned(), solid_bar_figure())]),
    };
    let (width, height) = native_dimensions();
    let mut pixels = vec![0u32; width * height];
    paint_channel_scene(
        &mut Frame {
            buf: &mut pixels,
            w: width,
            h: height,
        },
        &qualified_channels(),
        0,
        0,
        &art,
        None,
        None,
    );

    for (slot, row_top) in [224, 275, 324, 375].into_iter().enumerate() {
        let pixel = pixels[(row_top + 3) * width + 300];
        assert_eq!(
            pixel,
            pack_xrgb(0, 255, 0),
            "slot {slot} at row top {row_top} kept no staged bar"
        );
    }
}
