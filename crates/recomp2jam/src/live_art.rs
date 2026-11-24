//! Live session scene art: genuine 7.55 archive backdrops under the
//! windowed online walk (`run_play --window --online`).
//!
//! Every live screen stages through the State-keyed staging declaration
//! (`stage_state`/`stage_dialog`) — the same seam the shots walk, the
//! soak, and the session walk stage through — driven by live room
//! state instead of shot fixtures: the room arena stage resolves the venue,
//! and the room key mode plus team flag select the Playing family members.
//! Archives stay read-only; missing or unreadable archives keep the current
//! fallback paint, never failing.
//!
//! Differences from the shots staging, by intent: the Playing mode follows
//! the joined room (the shots pin the solo reference composition), the AVI
//! logo never loads here (an explicit `--interface` splash wins — the shell
//! exposes no splash query, so loading would clobber it), and no avatar,
//! catalog, or chat fixtures stage (the live session owns those).

use std::collections::HashMap;
use std::path::PathBuf;

use o2jam_shell::avatar::load_archive_sprite;
use o2jam_shell::avatar::PlayingArtMode;
use o2jam_shell::screens::paint::{
    SceneArt, DIALOG_BAR_SPRITE, RESULT_LIFE_FONT_SPRITE, ROOM_HOVER_SPRITE,
};
use o2jam_shell::screens::side::PlanetOptionTab;
use o2jam_shell::staging::{
    read_install_archives, stage_byname_sheet, stage_cash_charge, stage_dialog,
    stage_messenger_sheet, stage_payment_pages, stage_room_hover_overlay,
    stage_shopping_bag_add_sheet, stage_state, Dialog, StageContext, StagingArchive, State,
    BYNAME_TABLE, MESSENGER_SHEETS,
};
use o2jam_shell::{PlaySkin, Screen, ScreenGame};

/// Repository-local `install-root` (git-ignored game data).
fn install_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../install-root")
}

/// Live Playing mode: the joined room's key mode and team flag (the
/// shots pin solo here; the live walk stages what the room names).
fn live_playing_mode(game: &ScreenGame) -> PlayingArtMode {
    PlayingArtMode {
        key_mode: game.room_key_mode(),
        team_play: game.room_team_play(),
    }
}

/// Art-less `STATE_CHANNEL`: the empty scene takes the staged path
/// (`paint_channel_scene` paints no fill under the live list) instead
/// of the flat `paint_channel_list` fallback — the shots walk does the
/// same. Archive-independent, so it always stages.
fn stage_empty_channel_scene(game: &mut ScreenGame) {
    game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
}

/// Stage Interface-archive scenes through the declaration; each scene
/// stages independently so one bad block never blanks the rest. Myroom
/// stages bound through its row (ruling 1): the archive holds
/// `State_Myroom.bnd`, so the room paints at `.bnd` rects. The Result
/// banner merges through its row; the song-select dialog art stages
/// over the Waiting seats.
fn stage_interface_scenes(game: &mut ScreenGame, archive: &[u8]) {
    let ctx = StageContext::default();
    for (screen, state) in [
        (Screen::Avi, State::Avi),
        (Screen::Room, State::Room),
        (Screen::Waiting, State::Waiting),
        (Screen::Planet, State::Planet),
        (Screen::Lanking, State::Lanking),
        (Screen::Shop, State::Shop),
        (Screen::MusicShop, State::MusicShop),
        (Screen::Myroom, State::MyRoom),
        (Screen::Result, State::Result),
    ] {
        if let Some(art) = stage_state(archive, state, ctx).art {
            game.load_scene(screen, art.placements, art.figures);
        }
    }
    if let Some(figure) = stage_room_hover_overlay(archive) {
        game.merge_scene_figures(
            Screen::Room,
            HashMap::from([(ROOM_HOVER_SPRITE.to_owned(), figure)]),
        );
    }
    stage_select_dialog_art(game, archive);
    stage_channel_dialog(game, archive);
    stage_planet_dialogs(game, archive);
    stage_payment_sheets(game, archive);
    stage_cash_charge_sheet(game, archive);
    stage_messenger_sheets(game, archive);
    stage_shopping_bag_add_parked(game, archive);
}

/// Park the cart-add sheet's parsed rects beside the live shop. The
/// row pick is pinned to the SHOPPING BAG panel face, not to this
/// sheet, so nothing paints it and no add opcode is sent (#685).
fn stage_shopping_bag_add_parked(game: &mut ScreenGame, archive: &[u8]) {
    let Some(art) = stage_shopping_bag_add_sheet(archive, StageContext::default()).art else {
        return;
    };
    game.load_shopping_bag_add_sheet(SceneArt {
        placements: art.placements,
        figures: art.figures,
    });
}

/// Park all six Messenger sheets beside the live scenes: each block
/// stages its own `.bnd` rects through the bound arm, but the B8/B6
/// packs never mount one, so no painter consumes them yet (#622;
/// `N-MOUNT` stays OPEN). A miss keeps that sheet unstaged, never
/// an invented placement.
fn stage_messenger_sheets(game: &mut ScreenGame, archive: &[u8]) {
    let ctx = StageContext::default();
    for sheet in MESSENGER_SHEETS {
        if let Some(art) = stage_messenger_sheet(archive, sheet, ctx).art {
            game.load_messenger_sheet(
                sheet,
                SceneArt {
                    placements: art.placements,
                    figures: art.figures,
                },
            );
        }
    }
}

/// Stage the planet-board modal sheets: all three options tabs
/// (`DIALOG_OPTION_KEY/SOUND/GAME`; the gear opens Sound, pinned by
/// the b3/b4 exe captures) and the quit-confirm (`Dialog_Question1`).
/// Both stay modal
/// art (the game paints them dialog-relative only while open),
/// never scene slots. Each block stages independently; a miss keeps
/// the fallback popup/title. Shared with the shots walk (one
/// implementation for both).
pub(crate) fn stage_planet_dialogs(game: &mut ScreenGame, archive: &[u8]) {
    let ctx = StageContext::default();
    for (dialog, tab) in [
        (Dialog::PlanetOption, PlanetOptionTab::Key),
        (Dialog::PlanetOptionSound, PlanetOptionTab::Sound),
        (Dialog::PlanetOptionGame, PlanetOptionTab::Game),
    ] {
        if let Some(art) = stage_dialog(archive, dialog, ctx).art {
            game.load_planet_option_tab(tab, art.placements, art.figures);
        }
    }
    if let Some(art) = stage_dialog(archive, Dialog::PlanetQuit, ctx).art {
        game.load_planet_quit_scene(art.placements, art.figures);
    }
    stage_relay_down_sheet(game, archive);
    stage_room_dialogs(game, archive);
}

/// Stage the one-OK `Dialog_Information` sheet the relay-down notice
/// paints (#244). `stage_planet_dialogs` loads it on every live
/// boot that stages art; `stage_relay_down_sheet` is the pre-connect
/// entry the boot gate calls so the popup has real `.bnd` geometry
/// before any channel list is shown. Missing archive leaves the
/// fallback plate, never an invented placement.
pub(crate) fn stage_relay_down_sheet(game: &mut ScreenGame, archive: &[u8]) {
    if let Some(art) = stage_dialog(archive, Dialog::Information, StageContext::default()).art {
        game.load_relay_down_scene(art.placements, art.figures);
    }
}

/// Read the installed Interface archive and stage the relay sheet
/// from it. Used by the windowed boot entries before they connect,
/// so the notice is paintable at the moment the gate opens it.
pub(crate) fn stage_relay_down_from_install(game: &mut ScreenGame) {
    let Some(archive) = read_install_archives(&install_root()).interface else {
        return;
    };
    stage_relay_down_sheet(game, &archive);
}

/// Stage the room-lobby dialog sheets (ticket #53): `DIALOG_CREATEROOM`
/// and `DIALOG_INPUTPASSWORD` through the declaration, like every other
/// dialog. Each stages independently; a miss keeps the unstaged dialog
/// path (the modal still owns input, clicks land nowhere).
fn stage_room_dialogs(game: &mut ScreenGame, archive: &[u8]) {
    let ctx = StageContext::default();
    if let Some(art) = stage_dialog(archive, Dialog::CreateRoom, ctx).art {
        game.load_create_room_scene(art.placements, art.figures);
    }
    if let Some(art) = stage_dialog(archive, Dialog::InputPassword, ctx).art {
        game.load_password_scene(art.placements, art.figures);
    }
}

/// Stage the billing sheets on the live boot path.
///
/// `STATE_PAYMENT` + `STATE_PAYMENT2` stage through the declaration,
/// merged exactly like the shots side-walk (`shots.rs` `side_stage_data`). The rows
/// already staged through the side walk; only the live boot path
/// never mounted them. Parked in its own slot — the painter still
/// reads the side slots, so paint wiring rides a later ticket.
fn stage_payment_sheets(game: &mut ScreenGame, archive: &[u8]) {
    let ctx = StageContext::default();
    let merged = stage_payment_pages(archive, ctx).into_merged();
    if merged.placements.is_empty() {
        return;
    }
    game.load_payment_scene(merged.placements, merged.figures);
}

/// Stage the Cash Charge view through the same declaration: the
/// Item Shop host board plus `Dialog_ChargeCash.bnd` chrome. The
/// live boot never visits a side screen, so without this arm the
/// charge plate would open a frame-less view.
fn stage_cash_charge_sheet(game: &mut ScreenGame, archive: &[u8]) {
    let art = stage_cash_charge(archive, StageContext::default());
    if art.dialog.has_figures() {
        game.load_cash_charge_art(art);
    }
}

/// Stage the channel dialog art through the declaration: the bound
/// `DIALOG_CHANNEL_INFO` bodies, plus the hand-staged hover bar. The
/// painter mounts each figure at its bnd-derived origin (bar at row
/// origin +1, bar-local children, scroll bodies on the track);
/// absent figures fall back to the labeled stand-ins. The hover
/// highlight bar (`Dialog_ChannelSelectBar.ojs`, 348x44) stays
/// code-mounted — live-confirmed sprite record at dialog+0x8C
/// (2026-11-09) — and paints at the hovered row's rect.
pub(crate) fn stage_channel_dialog(game: &mut ScreenGame, archive: &[u8]) {
    let staged = stage_dialog(archive, Dialog::ChannelInfo, StageContext::default());
    let mut figures = staged.art.map(|art| art.figures).unwrap_or_default();
    // The hover bar stays hand-staged: the exe mounts it at the Dialog
    // object (`+0x8C`), not from the table (OPEN).
    if let Some(bar) = load_archive_sprite(archive, DIALOG_BAR_SPRITE) {
        figures.insert(DIALOG_BAR_SPRITE.to_owned(), bar);
    }
    if figures.is_empty() {
        return;
    }
    // Placements stay empty: the Channel painter mounts dialog art at
    // its anchors (exe-verified), never at `.bnd` rects.
    game.load_scene(Screen::Channel, Vec::new(), figures);
}

/// Stage real song-select dialog art over the Waiting seats; empty art
/// paints nothing extra.
fn stage_select_dialog_art(game: &mut ScreenGame, archive: &[u8]) {
    let Some(art) = stage_dialog(archive, Dialog::SelectMusic, StageContext::default()).art else {
        return;
    };
    if art.figures.is_empty() {
        return;
    }
    game.cache_select_music_scene(art.placements, art.figures);
}

/// Stage Playing-archive scenes through the live room: the Loading gate
/// sprite plus the Playing gauges through the room arena stage and key
/// mode, and the stage-aware play skin (procedural lanes stay on a miss).
fn stage_playing_scenes(game: &mut ScreenGame, archive: &[u8]) {
    if let Some(art) = stage_state(archive, State::Loading, StageContext::default()).art {
        game.load_scene(Screen::Loading, art.placements, art.figures);
    }
    // Tutorial is a block-less full-canvas row of this archive. Its
    // frame-0 guide art must be parked before FIRSTSTEP can mount it;
    // a miss keeps the fallback backdrop, never an invented plate.
    if let Some(art) = stage_state(archive, State::Tutorial, StageContext::default()).art {
        game.load_tutorial_scene(art.placements, art.figures);
    }
    // The tutorial chart's 51 Easy notes feed the same staged-note
    // seam Playing uses. Missing game data keeps the highway empty.
    if let Ok(bytes) = std::fs::read(install_root().join("O2Jam/Music/Tutorial.ojn")) {
        if let Some(onsets) = crate::loading::tutorial_chart_onsets(&bytes) {
            game.stage_tutorial_chart(&onsets);
        }
    }
    let mode = live_playing_mode(game);
    let chart_slot = game
        .selected_music()
        .map(|music| music.difficulty)
        .unwrap_or(0);
    let ctx = StageContext {
        arena_stage: Some(game.arena_stage()),
        key_mode: mode.key_mode,
        team_play: mode.team_play,
        chart_slot,
    };
    let Some(art) = stage_state(archive, State::Playing, ctx).art else {
        return;
    };
    game.load_scene(Screen::Playing, art.placements, art.figures);
    game.stage_venue_theme();
    if let Some(skin) = PlaySkin::from_archive_for_stage(archive, Some(game.arena_stage())) {
        game.set_play_skin(skin);
    }
    stage_playing_byname_sheets(game, archive);
}

/// Stage the exe-literal Playing sheets beside the Playing scene
/// (ticket #166): `State_Playing_network` + `Avatar_Info` +
/// `Map_Avatar` ride no `ControlList` block, so the by-name rows
/// stage each file's own rects into its game slot. Missing sheets
/// keep their unstaged slots; paint wiring rides a later ticket.
/// Restage-safe: `load_byname_sheet` overwrites per bind file, so a
/// room change re-stages exactly the trio.
fn stage_playing_byname_sheets(game: &mut ScreenGame, archive: &[u8]) {
    for row in BYNAME_TABLE
        .iter()
        .filter(|row| row.archive == StagingArchive::Playing)
    {
        let Some(art) = stage_byname_sheet(archive, row).art else {
            continue;
        };
        game.load_byname_sheet(row.bind_file, art.placements, art.figures);
    }
}

/// Merge the life-readout digit font into the staged Result scene:
/// `PointNum.ojs` mounts at `this+0x10d10` from the PLAYING archive
/// (exe builder `FUN_004c2390`, file `0xc2390`), so the
/// Interface-side stage row cannot name it — both staging paths (the
/// live window and the shots walk) call this after both archives
/// staged. A miss keeps the value-gated digit fields unpainted (their
/// value globals stay OPEN, `docs/cresult-anchors.md`).
pub(crate) fn stage_result_life_font(game: &mut ScreenGame, playing_archive: &[u8]) {
    let Some(font) = load_archive_sprite(playing_archive, RESULT_LIFE_FONT_SPRITE) else {
        return;
    };
    let mut figures = HashMap::new();
    figures.insert(RESULT_LIFE_FONT_SPRITE.to_owned(), font);
    game.merge_scene_figures(Screen::Result, figures);
}

/// Stage genuine 7.55 scene art under every live screen when
/// `install-root` holds the archives; missing or unreadable archives
/// keep the fallback paint. Read-only and infallible: staging never
/// moves the flow, so call it right after the login lists land.
pub fn stage_live_scenes(game: &mut ScreenGame) {
    stage_empty_channel_scene(game);
    let installed = read_install_archives(&install_root());
    if let Some(archive) = &installed.interface {
        stage_interface_scenes(game, archive);
    }
    if let Some(archive) = &installed.playing {
        stage_playing_scenes(game, archive);
        stage_result_life_font(game, archive);
    }
}

/// Re-stage the Playing-archive scenes after the joined room changes
/// (join, host, arena echo): the venue and family members follow the
/// live arena stage and mode. Interface scenes never depend on the
/// room, so they keep their login-time staging.
pub fn restage_live_playing(game: &mut ScreenGame) {
    let Some(archive) = read_install_archives(&install_root()).playing else {
        return;
    };
    stage_playing_scenes(game, &archive);
}

#[cfg(test)]
mod live_art_tests {
    use o2jam_file::data_root::{pack_skip_report, root_has_game_data};
    use o2jam_net::{ChannelState, GameMode, JoinedRoom, RoomListEntry};
    use o2jam_render::{Frame, NATIVE_HEIGHT, NATIVE_WIDTH};
    use o2jam_shell::{GameLoop, InputSnapshot, MenuKey, Screen, ScreenGame};

    use super::{install_root, restage_live_playing, stage_live_scenes, stage_planet_dialogs};
    use o2jam_shell::screens::game::PlanetModal;
    use o2jam_shell::screens::side::{PlanetOptionTab, SideScreen};
    use o2jam_shell::staging::{
        read_install_archives, stage_room_hover_overlay, stage_state, StageContext, StagedArt,
        State,
    };

    /// Archives behind the staging (`true` while `install-root` holds
    /// the 7.55 interface art; the suite stays green without it).
    fn interface_present() -> bool {
        read_install_archives(&install_root()).interface.is_some()
    }

    /// Archives behind the Playing staging.
    fn playing_present() -> bool {
        read_install_archives(&install_root()).playing.is_some()
    }

    /// Skip helper: the staged-pixel comparisons need real archives.
    fn skip_without_archives() -> bool {
        if interface_present() && playing_present() {
            return false;
        }
        println!("skip: missing install-root archives");
        true
    }

    /// One Interface State staged through the declaration, if the
    /// archives hold it.
    fn declared_interface(state: State) -> Option<StagedArt> {
        let archive = read_install_archives(&install_root()).interface?;
        stage_state(&archive, state, StageContext::default()).art
    }

    /// Declared staging as the scene painters' art value.
    fn scene_art_of(staged: StagedArt) -> o2jam_shell::screens::paint::SceneArt {
        o2jam_shell::screens::paint::SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        }
    }

    /// One lobby row for the staged paint path.
    fn lobby_row() -> RoomListEntry {
        RoomListEntry {
            number: 3,
            state_code: 1,
            title: "Jam".to_owned(),
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

    /// Joined room behind the live Playing mode: arena stage 2 with a
    /// 7-key solo table, so the venue and family members resolve.
    fn live_room() -> JoinedRoom {
        JoinedRoom {
            member_id: 0,
            team: 0,
            title: "Jam".to_owned(),
            music_id: 0,
            arena: 2,
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

    /// Game with live login data: channels, rooms, and the joined room.
    fn live_game() -> ScreenGame {
        let mut game = ScreenGame::new();
        game.set_channels(vec![ChannelState {
            gateway_id: 0,
            channel_id: 2,
            capacity: 100,
            population: 5,
            active: true,
        }]);
        game.set_rooms(vec![lobby_row()]);
        game.set_joined_room(live_room());
        game
    }

    /// Walk the offline flow to one screen (lane dismisses the splash
    /// onto the bare board, then one Confirm per edge: emblem →
    /// dialog → room → waiting → loading).
    fn walk_to(game: &mut ScreenGame, target: Screen) {
        game.menu_key(MenuKey::Back, true);
        if target == Screen::Planet {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Channel {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Room {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Waiting {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if target == Screen::Loading {
            return;
        }
        // The last Confirm is the Loading→Playing gate (S7): arm it.
        {
            use crate::loading::{loading_phase_status, LOADING_GATE_STEPS};
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
        game.menu_key(MenuKey::Confirm, true);
    }

    /// Count one exact framebuffer color inside a pinned prefix box.
    fn core_pixels(pixels: &[u32], width: usize, left: usize, width_px: usize, core: u32) -> usize {
        (238..250)
            .flat_map(|row| (left..left + width_px).map(move |col| pixels[row * width + col]))
            .filter(|pixel| *pixel == core)
            .count()
    }

    /// Native-canvas pixels of one explicit-clock paint.
    fn painted_pixels(game: &mut ScreenGame, now_ms: u64) -> Vec<u32> {
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            now_ms,
        );
        buf
    }

    /// Bare versus staged paints of one screen walked the same way.
    fn paint_pair(target: Screen, now_ms: u64) -> (Vec<u32>, Vec<u32>) {
        let mut bare = live_game();
        walk_to(&mut bare, target);
        let bare_pixels = painted_pixels(&mut bare, now_ms);
        let mut staged = live_game();
        stage_live_scenes(&mut staged);
        walk_to(&mut staged, target);
        let staged_pixels = painted_pixels(&mut staged, now_ms);
        (bare_pixels, staged_pixels)
    }

    #[test]
    fn waiting_song_rect_shows_select_button_for_host_and_cover_for_peer() {
        // Live-path guard (ticket #278, owner decision 2026-11-16): the
        // Waiting song rect carries role-swapped bodies. The host sees
        // the blue song-select button with no `BPM` cover; a peer keeps
        // the `BPM` cover and never sees the button. The exe display
        // rule is OPEN (`docs/room-enums.md` "The waiting song rect":
        // next evidence step is a qualified capture, gated by #240), so
        // this guard pins the owner-decision shell rule, not exe
        // behavior.
        //
        // Live caller chain:
        //   main.rs::run_play -> setup_online -> session::OnlineSession::stage_live_art
        //   -> crate::live_art::stage_live_scenes -> stage_interface_scenes
        //   -> stage_state(State::Waiting) -> ScreenGame::paint_at
        //   -> paint_waiting -> paint_waiting_scene
        //   -> waiting_control_armed_for_role (the Waiting role filter).
        //
        // Staging keeps the shipped bindings whatever the role: the
        // `ControlList_Interface.txt` `STATE_WAITING` block names
        // `IDC_BUTTON_SWAIT_MUSELECT 0x16200003 "Wait_MusicSelect.ojs"`
        // (code ordinal 2 → `.bnd` rect 2 `(540,493)-(594,542)`) and
        // `IDC_IMAGE_SWAIT_COVERMUSIC 0x1650001D "Wait_Cover_Music.ojs"`
        // (ordinal 28 → rect 28, authored over the button). The role
        // only decides which of them paints.
        //
        // Archives absent -> skip cleanly (CI has no game data).
        if skip_without_archives() {
            return;
        }
        let mut host = live_game();
        stage_live_scenes(&mut host);
        walk_to(&mut host, Screen::Waiting);
        assert_eq!(host.screen(), Screen::Waiting);

        use o2jam_shell::avatar::ScenePlacement;
        let placements = host.scene_placements(Screen::Waiting);
        let song_select = placements
            .iter()
            .find(|placed: &&ScenePlacement| placed.id == "IDC_BUTTON_SWAIT_MUSELECT")
            .expect("MUSELECT placement missing from staged Waiting art");
        assert_eq!(song_select.code, 0x16200003, "MUSELECT code mismatch");
        assert_eq!(
            song_select.sprite.as_deref(),
            Some("Wait_MusicSelect.ojs"),
            "MUSELECT body must stay the shipped one"
        );

        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        let probe = |pixels: &[u32], (x, y): (usize, usize)| pixels[y * width + x];
        // Inside the album-mode body (`0x1650001C`, rect 27
        // `(533,441)-(601,551)`) and clear of the button and the cover.
        let album_probe = (536, 460);
        // The song rect proper is the union of the three staged bodies
        // (`(533,441)-(601,551)`): a whole-region read, so a role that
        // swaps one body for the other cannot slip past a lucky pixel.
        let song_rect = (533usize, 441usize, 601usize, 551usize);
        let song_body_ids = [
            "IDC_BUTTON_SWAIT_MUSELECT",
            "IDC_IMAGE_SWAIT_COVERMUSIC",
            "IDC_IMAGE_SWAIT_ALBUMMODE",
        ];
        let region = |pixels: &[u32]| -> Vec<u32> {
            let (x0, y0, x1, y1) = song_rect;
            (y0..y1)
                .flat_map(|y| pixels[y * width + x0..y * width + x1].to_vec())
                .collect()
        };
        // First differing pixel between two song-rect reads, as
        // `(x, y) left vs right`, so a role swap reads as one
        // coordinate instead of a pixel dump.
        let first_mismatch = |left: &[u32], right: &[u32]| -> Option<String> {
            let stride = song_rect.2 - song_rect.0;
            left.iter()
                .zip(right)
                .position(|(a, b)| a != b)
                .map(|index| {
                    format!(
                        "({}, {}) {:#010x} vs {:#010x}",
                        song_rect.0 + index % stride,
                        song_rect.1 + index / stride,
                        left[index],
                        right[index]
                    )
                })
        };
        let assert_same_read = |left: &[u32], right: &[u32], what: &str| {
            let mismatch = first_mismatch(left, right);
            assert!(mismatch.is_none(), "{what}: first mismatch {mismatch:?}");
        };
        let assert_reads_differ = |left: &[u32], right: &[u32], what: &str| {
            assert!(first_mismatch(left, right).is_some(), "{what}");
        };

        let host_frame = painted_pixels(&mut host, 0);

        // The peer paint: the same staged scene walked the same way,
        // joined at seat 5 (the creator keeps seat 0, so any other id
        // is a peer — `docs/relay.md` §10).
        let mut peer = live_game();
        stage_live_scenes(&mut peer);
        walk_to(&mut peer, Screen::Waiting);
        let mut peer_room = live_room();
        peer_room.member_id = 5;
        peer.set_joined_room(peer_room);
        let peer_frame = painted_pixels(&mut peer, 0);

        // References computed in-process from the same staged art: the
        // song-rect bodies reduced to one named body, or to none. Each
        // reference paints on the role that shows the kept body, so the
        // role filter keeps it; wherever a dropped body is absent the
        // reference reads what that role's backdrop paints, so no
        // capture pixel is baked in.
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive");
        let staged = stage_state(&archive, State::Waiting, StageContext::default())
            .art
            .expect("waiting art");
        let StagedArt {
            placements,
            figures,
        } = staged;
        let paint_song_bodies = |game: &mut ScreenGame, keep: &[&str]| -> Vec<u32> {
            let kept: Vec<ScenePlacement> = placements
                .iter()
                .filter(|placed| {
                    !song_body_ids.contains(&placed.id.as_str())
                        || keep.contains(&placed.id.as_str())
                })
                .cloned()
                .collect();
            game.load_scene(Screen::Waiting, kept, figures.clone());
            painted_pixels(game, 0)
        };
        let bare = paint_song_bodies(&mut host, &[]);
        let button_only = paint_song_bodies(&mut host, &["IDC_BUTTON_SWAIT_MUSELECT"]);
        let cover_only = paint_song_bodies(&mut peer, &["IDC_IMAGE_SWAIT_COVERMUSIC"]);

        let host_region = region(&host_frame);
        let peer_region = region(&peer_frame);
        let bare_region = region(&bare);
        let button_region = region(&button_only);
        let cover_region = region(&cover_only);

        assert_reads_differ(
            &host_region,
            &bare_region,
            "host must paint the song-select button, not a bare rect",
        );
        assert_same_read(
            &host_region,
            &button_region,
            "host paints the button alone: no BPM cover over it",
        );
        assert_same_read(
            &peer_region,
            &cover_region,
            "peer paints the BPM cover alone: no button under it",
        );
        assert_reads_differ(
            &peer_region,
            &bare_region,
            "peer must not read the bare rect: it keeps the BPM cover",
        );
        assert_reads_differ(
            &host_region,
            &peer_region,
            "the song rect must read differently per role",
        );
        assert_eq!(
            probe(&host_frame, album_probe),
            probe(&bare, album_probe),
            "the album-mode body paints for no role"
        );
        assert_eq!(
            probe(&peer_frame, album_probe),
            probe(&bare, album_probe),
            "the album-mode body paints for no role"
        );
    }
    #[test]
    fn staging_never_moves_the_flow() {
        let mut game = live_game();
        walk_to(&mut game, Screen::Waiting);
        stage_live_scenes(&mut game);
        assert_eq!(game.screen(), Screen::Waiting);
        restage_live_playing(&mut game);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn staging_never_fails_on_an_empty_game() {
        let mut game = ScreenGame::new();
        stage_live_scenes(&mut game);
        restage_live_playing(&mut game);
        assert_eq!(game.screen(), Screen::Avi);
    }

    #[test]
    fn live_boot_waiting_lands_plain_without_select_music_dialog() {
        // Live caller chain:
        //   main.rs::run_play -> assemble_online_session ->
        //   session::OnlineSession::stage_live_art ->
        //   crate::live_art::stage_live_scenes ->
        //   stage_interface_scenes -> stage_select_dialog_art ->
        //   ScreenGame::cache_select_music_scene (cache only).
        // Archives absent -> skip cleanly (CI has no game data).
        if !interface_present() {
            println!("skip: missing install-root interface archive");
            return;
        }
        let mut game = live_game();
        stage_live_scenes(&mut game);
        assert!(
            !game.select_music_dialog_open(),
            "live staging must not auto-open the select-music dialog"
        );
        assert!(
            game.select_music_art_cached(),
            "live staging must still cache the select-music dialog art"
        );
    }

    #[test]
    fn planet_stages_bound_art_when_archives_present() {
        // Seventeen bound entries pair seventeen `.bnd` rects by code
        // ordinal (self included, with the miss rect); the grown figures
        // carry the full-bleed state art. Pixel cover rides the shell
        // scene painter (covered there); the walk cannot reach Planet
        // without invented input, so no pixels assert here.
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::Planet).expect("planet: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert_eq!(placements.len(), 17);
        assert!(figures.contains_key("State_Planet.ojs"));
    }

    #[test]
    fn option_tabs_stage_all_three_sheets_when_archives_present() {
        // All three `DIALOG_OPTION_*` sheets stage through the
        // declaration; each open tab paints its own sheet (three
        // distinct frames — an unstaged tab would paint the shared
        // fallback title three times).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive stages");
        let mut game = live_game();
        stage_planet_dialogs(&mut game, &archive);
        game.menu_key(MenuKey::Back, true);
        game.click_planet(729, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut frames = Vec::new();
        for tab in [
            PlanetOptionTab::Key,
            PlanetOptionTab::Sound,
            PlanetOptionTab::Game,
        ] {
            game.switch_option_tab(tab);
            let mut buf = vec![0u32; width * height];
            game.paint_at(
                &mut Frame {
                    buf: &mut buf,
                    w: width,
                    h: height,
                },
                0,
            );
            frames.push(buf);
        }
        assert_ne!(frames[0], frames[1], "sound sheet paints its own art");
        assert_ne!(frames[1], frames[2], "game sheet paints its own art");
        assert_ne!(frames[0], frames[2], "key sheet paints its own art");
    }

    #[test]
    fn create_dialog_paints_its_sheet_after_live_staging() {
        // Ticket #53: the live boot stages the create-room sheet, so
        // opening the dialog over the same staged lobby inks pixels
        // (unstaged, the open dialog paints nothing extra).
        if skip_without_archives() {
            return;
        }
        let mut game = live_game();
        stage_live_scenes(&mut game);
        walk_to(&mut game, Screen::Room);
        let closed = painted_pixels(&mut game, 0);
        game.open_create_room_dialog();
        game.push_room_dialog_char('x');
        assert_ne!(
            painted_pixels(&mut game, 0),
            closed,
            "CREATE paints no sheet after live staging"
        );
    }

    #[test]
    fn live_staged_create_sheet_commits_a_title() {
        // Ticket #53: the live-staged sheet carries real
        // `Dialog_CreateRoom.bnd` rects, so a CREATE click on the
        // dialog-relative rect (back origin + `.bnd` rect, pinned #114)
        // commits the typed title (the 2004 body).
        use o2jam_shell::screens::paint::dialog_origin;
        use o2jam_shell::screens::side::CREATE_ROOM_CREATE_CODE;
        use o2jam_shell::staging::{stage_dialog, Dialog};
        if skip_without_archives() {
            return;
        }
        let archive = read_install_archives(&install_root())
            .interface
            .expect("interface archive stages");
        let mut game = live_game();
        stage_live_scenes(&mut game);
        walk_to(&mut game, Screen::Room);
        game.open_create_room_dialog();
        game.push_room_dialog_char('x');
        let staged = stage_dialog(&archive, Dialog::CreateRoom, StageContext::default())
            .art
            .expect("create-room dialog stages");
        let create = staged
            .placements
            .iter()
            .find(|placed| placed.code == CREATE_ROOM_CREATE_CODE)
            .expect("CREATE rect stages");
        let (origin_x, origin_y) = dialog_origin(&staged.placements);
        let (cx, cy) = (
            origin_x + create.rect.x + create.rect.w as i32 / 2,
            origin_y + create.rect.y + create.rect.h as i32 / 2,
        );
        game.ingest_snapshot(&InputSnapshot::click_at(cx, cy));
        assert_eq!(game.room_dialog(), None);
        let body = game.take_create_room().expect("committed 2004");
        assert_eq!(body.title, "x");
    }

    #[test]
    fn planet_paints_real_art_when_archives_present() {
        // The walk cannot reach Planet without invented input, so this
        // paints the staged scene directly: real art must ink the frame
        // beyond the flat card.
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art = scene_art_of(declared_interface(State::Planet).expect("planet: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
            None,
            None,
            None,
            o2jam_shell::screens::paint::PlanetPromoPaint::spinning(1),
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "planet staged art paints nothing"
        );
    }

    #[test]
    fn lanking_stages_bound_art_when_archives_present() {
        // Rank rows need the fetch opcode (OPEN), so this pins the
        // staging shape only: placements pair the 73 `.bnd` rects by
        // code ordinal, and the full-bleed state art grows. Pixel cover
        // rides the shell scene painter (covered there).
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::Lanking).expect("lanking: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert!(!placements.is_empty());
        assert!(figures.contains_key("State_Lanking.ojs"));
    }

    #[test]
    fn lanking_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art =
            scene_art_of(declared_interface(State::Lanking).expect("lanking: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_lanking_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "lanking staged art paints nothing"
        );
    }

    /// Game holding declaration art directly, mirroring the live
    /// coverage (empty channel, every interface State, the select and
    /// channel dialogs plus the hand-staged hover bar, the planet
    /// sheets, the Loading gate, the Playing subset through the live
    /// room, and the stage-aware skin). `None` without the archives.
    fn declaration_game() -> Option<ScreenGame> {
        use o2jam_shell::avatar::{load_archive_sprite, PlayingArtMode};
        use o2jam_shell::screens::paint::{DIALOG_BAR_SPRITE, ROOM_HOVER_SPRITE};
        use o2jam_shell::staging::{stage_dialog, Dialog};
        use std::collections::HashMap;
        let installed = read_install_archives(&install_root());
        let interface = installed.interface?;
        let playing = installed.playing?;
        let mut expected = live_game();
        expected.load_scene(Screen::Channel, Vec::new(), HashMap::new());
        let ctx = StageContext::default();
        for (screen, state) in [
            (Screen::Avi, State::Avi),
            (Screen::Room, State::Room),
            (Screen::Waiting, State::Waiting),
            (Screen::Planet, State::Planet),
            (Screen::Lanking, State::Lanking),
            (Screen::Shop, State::Shop),
            (Screen::MusicShop, State::MusicShop),
            (Screen::Myroom, State::MyRoom),
            (Screen::Result, State::Result),
        ] {
            if let Some(art) = stage_state(&interface, state, ctx).art {
                expected.load_scene(screen, art.placements, art.figures);
            }
        }
        if let Some(figure) = o2jam_shell::staging::stage_room_hover_overlay(&interface) {
            expected.merge_scene_figures(
                Screen::Room,
                HashMap::from([(ROOM_HOVER_SPRITE.to_owned(), figure)]),
            );
        }
        if let Some(art) =
            stage_dialog(&interface, Dialog::SelectMusic, StageContext::default()).art
        {
            expected.cache_select_music_scene(art.placements, art.figures);
        }
        let mut channel_figures =
            stage_dialog(&interface, Dialog::ChannelInfo, StageContext::default())
                .art
                .map(|art| art.figures)
                .unwrap_or_default();
        if let Some(bar) = load_archive_sprite(&interface, DIALOG_BAR_SPRITE) {
            channel_figures.insert(DIALOG_BAR_SPRITE.to_owned(), bar);
        }
        expected.load_scene(Screen::Channel, Vec::new(), channel_figures);
        for (dialog, tab) in [
            (Dialog::PlanetOption, PlanetOptionTab::Key),
            (Dialog::PlanetOptionSound, PlanetOptionTab::Sound),
            (Dialog::PlanetOptionGame, PlanetOptionTab::Game),
        ] {
            if let Some(art) = stage_dialog(&interface, dialog, StageContext::default()).art {
                expected.load_planet_option_tab(tab, art.placements, art.figures);
            }
        }
        if let Some(art) = stage_dialog(&interface, Dialog::PlanetQuit, StageContext::default()).art
        {
            expected.load_planet_quit_scene(art.placements, art.figures);
        }
        if let Some(art) = stage_dialog(&interface, Dialog::CreateRoom, StageContext::default()).art
        {
            expected.load_create_room_scene(art.placements, art.figures);
        }
        if let Some(art) =
            stage_dialog(&interface, Dialog::InputPassword, StageContext::default()).art
        {
            expected.load_password_scene(art.placements, art.figures);
        }
        if let Some(art) = stage_state(&playing, State::Loading, StageContext::default()).art {
            expected.load_scene(Screen::Loading, art.placements, art.figures);
        }
        let mode = PlayingArtMode {
            key_mode: expected.room_key_mode(),
            team_play: expected.room_team_play(),
        };
        let playing_ctx = StageContext {
            arena_stage: Some(expected.arena_stage()),
            key_mode: mode.key_mode,
            team_play: mode.team_play,
            chart_slot: 0,
        };
        if let Some(art) = stage_state(&playing, State::Playing, playing_ctx).art {
            expected.load_scene(Screen::Playing, art.placements, art.figures);
        }
        // The venue theme resolver runs at runtime on the live path
        // (stage_playing_scenes, #848); the declaration path applies
        // the same resolver so the no-delta oracle holds.
        expected.stage_venue_theme();
        if let Some(skin) =
            o2jam_shell::PlaySkin::from_archive_for_stage(&playing, Some(expected.arena_stage()))
        {
            expected.set_play_skin(skin);
        }
        Some(expected)
    }

    #[test]
    fn live_matches_declaration_on_walked_screens_when_archives_present() {
        // No-delta oracle: a game staged by the live window must paint
        // every walked screen exactly like a game holding declaration
        // art directly. Myroom is the one intended delta and stays out
        // (the flow never enters it); its bound wiring rides the
        // `myroom_stages_bound_art` row plus the art-equivalence probe.
        if skip_without_archives() {
            return;
        }
        for target in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
        ] {
            let mut expected = declaration_game().expect("guarded game");
            let mut live = live_game();
            stage_live_scenes(&mut live);
            walk_to(&mut expected, target);
            walk_to(&mut live, target);
            assert_eq!(expected.screen(), live.screen());
            let expected_pixels = painted_pixels(&mut expected, 0);
            let live_pixels = painted_pixels(&mut live, 0);
            if expected_pixels != live_pixels {
                let diffs: Vec<String> = (0..expected_pixels.len())
                    .filter(|&k| expected_pixels[k] != live_pixels[k])
                    .take(5)
                    .map(|k| {
                        format!(
                            "px {k} (x={}, y={}): {:#x} vs {:#x}",
                            k % 800,
                            k / 800,
                            expected_pixels[k],
                            live_pixels[k]
                        )
                    })
                    .collect();
                panic!(
                    "live diverges from the declaration on {target:?} ({} diffs; first: {})",
                    (0..expected_pixels.len())
                        .filter(|&k| expected_pixels[k] != live_pixels[k])
                        .count(),
                    diffs.join("; ")
                );
            }
        }
        let mut expected = declaration_game().expect("guarded game");
        let mut live = live_game();
        stage_live_scenes(&mut live);
        for game in [&mut expected, &mut live] {
            game.load_chart(vec![(2u8, 60_000u64, 7u32)]);
        }
        walk_to(&mut expected, Screen::Playing);
        walk_to(&mut live, Screen::Playing);
        assert_eq!(live.screen(), Screen::Playing);
        assert_eq!(
            painted_pixels(&mut expected, 0),
            painted_pixels(&mut live, 0),
            "live diverges from the declaration on Playing"
        );
    }

    #[test]
    fn myroom_stages_bound_art_when_archives_present() {
        // Bound `.bnd` pairing (ruling 1): placements pair the rects by
        // code ordinal and the full-bleed state art grows.
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::MyRoom).expect("myroom: unstaged state");
        assert!(!art.placements.is_empty());
        assert!(art.figures.contains_key("State_Myroom.ojs"));
    }

    #[test]
    fn myroom_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art = scene_art_of(declared_interface(State::MyRoom).expect("myroom: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_myroom_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "myroom staged art paints nothing"
        );
    }

    #[test]
    fn shop_stages_bound_art_when_archives_present() {
        // Catalogue rows need item data joins (OPEN); this pins the
        // staging shape only (gapless ordinal pairing per docs/shop.md).
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::Shop).expect("shop: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert!(!placements.is_empty());
        assert!(figures.contains_key("State_ItemShop.ojs"));
    }

    #[test]
    fn shop_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art = scene_art_of(declared_interface(State::Shop).expect("shop: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_shop_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "shop staged art paints nothing"
        );
    }

    #[test]
    fn music_shop_stages_bound_art_when_archives_present() {
        // Catalogue rows need the song index join (OPEN); this pins
        // the staging shape only.
        if skip_without_archives() {
            return;
        }
        let art = declared_interface(State::MusicShop).expect("music shop: unstaged state");
        let (placements, figures) = (art.placements, art.figures);
        assert!(!placements.is_empty());
        assert!(figures.contains_key("State_MShop.ojs"));
    }

    #[test]
    fn music_shop_paints_real_art_when_archives_present() {
        // Unreachable screens still paint through their scene function
        // directly (see the planet twin).
        use o2jam_render::Frame;
        if skip_without_archives() {
            return;
        }
        let art =
            scene_art_of(declared_interface(State::MusicShop).expect("music shop: unstaged state"));
        let width = usize::try_from(o2jam_render::NATIVE_WIDTH).unwrap_or(1);
        let height = usize::try_from(o2jam_render::NATIVE_HEIGHT).unwrap_or(1);
        let mut buf = vec![0u32; width * height];
        o2jam_shell::screens::paint::paint_music_shop_scene(
            &mut Frame {
                buf: &mut buf,
                w: width,
                h: height,
            },
            &art,
        );
        assert!(
            buf.iter().any(|pixel| *pixel != 0),
            "music shop staged art paints nothing"
        );
    }

    #[test]
    fn room_paint_uses_staged_bound_art_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Room, 0);
        assert_ne!(bare, staged);
    }

    #[test]
    fn room_hover_overlay_stages_from_the_code_mounted_archive_body() {
        // Live chain: main.rs::run_play -> stage_interface_scenes ->
        // stage_room_hover_overlay -> ScreenGame::merge_scene_figures ->
        // paint_room_scene_with_hover. Archives absent -> skip cleanly
        // (CI has no game data). The body has no ControlList entry.
        let Some(interface) = read_install_archives(&install_root()).interface else {
            println!("skip: missing install-root archives");
            return;
        };
        let overlay = stage_room_hover_overlay(&interface).expect("room hover body");
        let layer = overlay.layer(0).expect("one mouse-over body");
        assert_eq!(layer.image.w, 292);
        assert_eq!(layer.image.h, 95);
    }

    #[test]
    fn waiting_paint_uses_staged_bound_art_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Waiting, 0);
        assert_ne!(bare, staged);
    }

    /// Installed catalog rows (`OJNList.dat`), the live caption source;
    /// `None` without game data.
    fn installed_catalog_songs() -> Option<Vec<o2jam_shell::screens::side::SelectMusicSong>> {
        let bytes = std::fs::read(install_root().join("O2Jam/Image/OJNList.dat")).ok()?;
        let entries = o2jam_file::parse_ojn_list(&bytes).ok()?;
        // No install scan here: these tests prove caption geometry, and
        // an unknown availability keeps every row as before (#276).
        let songs = o2jam_shell::screens::side::select_music_songs(&entries, None);
        (!songs.is_empty()).then_some(songs)
    }

    /// The pick the #274 caption guards stage: easy chart at normal
    /// speed (layer 1 of the badge body, the EX1 cell).
    fn ex_pick(song_id: u16) -> o2jam_net::SetRoomMusic {
        o2jam_net::SetRoomMusic {
            music_id: song_id,
            difficulty: 0,
            speed: 1,
        }
    }

    /// Waiting game on the live-staged art: the joined room names
    /// `room_music_id`, the installed catalog is staged, and an optional
    /// host pick carries its own difficulty and speed codes.
    fn staged_waiting_pick_game(
        room_music_id: u16,
        pick: Option<o2jam_net::SetRoomMusic>,
    ) -> ScreenGame {
        let mut game = live_game();
        let mut room = live_room();
        room.music_id = room_music_id;
        game.set_joined_room(room);
        let bytes = std::fs::read(install_root().join("O2Jam/Image/OJNList.dat"))
            .expect("installed catalog bytes");
        let entries = o2jam_file::parse_ojn_list(&bytes).expect("installed catalog parses");
        game.set_song_catalog(&entries);
        if let Some(pick) = pick {
            game.set_selected_music(pick);
        }
        stage_live_scenes(&mut game);
        walk_to(&mut game, Screen::Waiting);
        assert_eq!(game.screen(), Screen::Waiting);
        game
    }

    /// Pinned `MUSICNAME` rect of the live-staged Waiting art
    /// (`State_Waiting.bnd` rect 22, paired by code ordinal — read from
    /// the staged placements, never a literal).
    fn staged_musicname_rect(game: &ScreenGame) -> o2jam_render::Rect {
        game.scene_placements(Screen::Waiting)
            .iter()
            .find(|placed| placed.id == "IDC_TEXT_SWAIT_MUSICNAME")
            .map(|placed| placed.rect)
            .expect("MUSICNAME placement missing from staged Waiting art")
    }

    #[test]
    fn live_waiting_pick_titles_the_bound_musicname_rect() {
        // Live chain: main.rs::run_play -> setup_online ->
        //   stage_online_identity -> stage_live_scenes ->
        //   stage_interface_scenes(State::Waiting) ->
        //   ScreenGame::set_selected_music (session::OnlineSession::
        //   resolve_song_select) -> ScreenGame::paint_at -> paint_waiting
        //   -> paint_waiting_scene -> paint_waiting_staged ->
        //   paint_waiting_music
        // Ticket #274: the master's pick names the `.bnd` `MUSICNAME`
        // TEXT control at once, so a room that names the picked song
        // and a room that names nothing plus the pick ink the same
        // caption inside the pinned rect.
        //
        // Archives absent -> skip cleanly (CI has no game data).
        if skip_without_archives() {
            return;
        }
        let Some(songs) = installed_catalog_songs() else {
            println!("skip: missing installed song catalog");
            return;
        };
        let song_id = u16::try_from(songs[0].song_id).expect("catalog song id fits u16");
        let mut picked = staged_waiting_pick_game(0, Some(ex_pick(song_id)));
        // Stage the explicit pick on both legs: an unclicked Waiting
        // entry keeps the exe's random default (#883), which would ink
        // a yellow caption instead of the room title.
        let mut room_named = staged_waiting_pick_game(song_id, Some(ex_pick(song_id)));
        let rect = staged_musicname_rect(&picked);
        let picked_frame = painted_pixels(&mut picked, 0);
        let named_frame = painted_pixels(&mut room_named, 0);
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        assert!(
            o2jam_render::has_ink_near(
                &o2jam_render::pixels_in_rect(&picked_frame, width, rect),
                o2jam_render::pack_xrgb(0xFF, 0xFF, 0xFF)
            ),
            "the staged pick inks no caption inside the MUSICNAME rect"
        );
        assert!(
            o2jam_render::pixels_in_rect(&picked_frame, width, rect)
                == o2jam_render::pixels_in_rect(&named_frame, width, rect),
            "the caption must carry the staged pick's title, not the room's"
        );
    }

    #[test]
    fn live_waiting_pick_leaves_the_musicname_plate_unscrimmed() {
        // Live chain: main.rs::run_play -> setup_online ->
        //   stage_online_identity -> stage_live_scenes ->
        //   stage_interface_scenes(State::Waiting) ->
        //   ScreenGame::set_selected_music -> ScreenGame::paint_at ->
        //   paint_waiting -> paint_waiting_scene -> paint_waiting_staged ->
        //   paint_waiting_music
        // Ticket #274: the shell used to fill the pixels just outside
        // the bound `MUSICNAME` rect with near-black scrim. Real 7.55
        // art paints its own plate there, so a pick must leave them
        // untouched.
        //
        // Archives absent -> skip cleanly (CI has no game data).
        if skip_without_archives() {
            return;
        }
        let Some(songs) = installed_catalog_songs() else {
            println!("skip: missing installed song catalog");
            return;
        };
        let song_id = u16::try_from(songs[0].song_id).expect("catalog song id fits u16");
        let mut picked = staged_waiting_pick_game(0, Some(ex_pick(song_id)));
        let rect = staged_musicname_rect(&picked);
        let frame = painted_pixels(&mut picked, 0);
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        let corner_x = usize::try_from(rect.x - 4).unwrap_or(0);
        let corner_y = usize::try_from(rect.y - 2).unwrap_or(0);
        assert_ne!(
            frame[corner_y * width + corner_x],
            o2jam_render::pack_xrgb(0x08, 0x08, 0x10),
            "the staged plate corner must not carry the shell scrim"
        );
    }

    /// Bottom edge of the lowest staged seat cover: the seat grid's
    /// lower bound (the chat box sits under it, where the #274 readout
    /// panel used to ink).
    fn seat_grid_bottom(placements: &[o2jam_shell::avatar::ScenePlacement]) -> u32 {
        placements
            .iter()
            .filter(|placed| placed.id.starts_with("IDC_IMAGE_SWAIT_COVERAVATAR"))
            .map(|placed| u32::try_from(placed.rect.y).unwrap_or(0) + placed.rect.h)
            .max()
            .unwrap_or(0)
    }

    /// True while a pixel index lies inside one staged rect.
    fn rect_contains(rect: o2jam_render::Rect, x: usize, y: usize) -> bool {
        let (Ok(px), Ok(py)) = (i32::try_from(x), i32::try_from(y)) else {
            return false;
        };
        rect.x <= px
            && px < rect.x.saturating_add(i32::try_from(rect.w).unwrap_or(0))
            && rect.y <= py
            && py < rect.y.saturating_add(i32::try_from(rect.h).unwrap_or(0))
    }

    /// First differing pixel outside every allowed rect. Unlike the
    /// render helper, this walks all changed pixels so one permitted
    /// region cannot hide a later stray ink.
    fn first_change_outside_allowed(
        before: &[u32],
        after: &[u32],
        w: usize,
        allowed: &[o2jam_render::Rect],
    ) -> Option<(usize, usize)> {
        let width = w.max(1);
        let height = before.len().min(after.len()) / width;
        for y in 0..height {
            for x in 0..width {
                if allowed.iter().any(|rect| rect_contains(*rect, x, y)) {
                    continue;
                }
                if before[y * width + x] != after[y * width + x] {
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// Native rect of one staged Waiting caption, read from placements.
    fn staged_caption_rect(game: &ScreenGame, id: &str) -> o2jam_render::Rect {
        game.scene_placements(Screen::Waiting)
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| placed.rect)
            .unwrap_or_else(|| panic!("{id} placement missing from staged Waiting art"))
    }

    #[test]
    fn live_waiting_pick_inks_nothing_below_the_seat_grid() {
        // Live chain: main.rs::run_play -> setup_online ->
        //   stage_online_identity -> stage_live_scenes ->
        //   stage_interface_scenes(State::Waiting) ->
        //   ScreenGame::set_selected_music -> ScreenGame::paint_at ->
        //   paint_waiting -> paint_waiting_scene -> paint_waiting_staged
        // Ticket #274: the chosen-song readout used to ink a dark panel
        // above the chat rows (a literal `(w/8, h*3/4)` scrim over the
        // staged `State_Waiting.bnd` backdrop). A pick may now re-ink
        // the pinned `MUSICNAME` rect and nothing else.
        //
        // #883 default-state refutation: the v3 waiting-master exe frame
        // shows `?` on all three random cards with a normal 757/2/1 room
        // song and zero clicks. `FUN_004aac80` seeds `DAT_007789dc` from
        // `FUN_004bb2c0` (which always sets bit 31) on Waiting entry
        // (`ev_c10cd3fa4167b8119c9df3156b5e4963987ffa251d55a0344880b4a674b194a9`,
        // `ev_17124597c0cfc47ad96457d9127450d22f7ab63df25d149977533df277b757ed`),
        // so the default entry state mounts the pinned EFFECT_1/2 mini
        // cards (`0x16500045/46`). Those two staged rects are therefore
        // permitted here; any other ink outside the seat grid and BPM
        // caption still fails the guard.
        //
        // Archives absent -> skip cleanly (CI has no game data).
        if skip_without_archives() {
            return;
        }
        let Some(songs) = installed_catalog_songs() else {
            println!("skip: missing installed song catalog");
            return;
        };
        let song_id = u16::try_from(songs[0].song_id).expect("catalog song id fits u16");
        let mut bare = staged_waiting_pick_game(0, None);
        let mut picked = staged_waiting_pick_game(0, Some(ex_pick(song_id)));
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        let grid_bottom = seat_grid_bottom(picked.scene_placements(Screen::Waiting));
        assert!(grid_bottom > 0, "the staged scene carries seat covers");
        let seat_grid = o2jam_render::Rect {
            x: 0,
            y: 0,
            w: u32::try_from(width).unwrap_or(0),
            h: grid_bottom,
        };
        let bpm = staged_caption_rect(&picked, "IDC_TEXT_SWAIT_BPM");
        // The detail painter puts a scrim at (x-4, y-2) around its ink.
        let bpm_with_scrim = o2jam_render::Rect {
            x: bpm.x - 4,
            y: bpm.y - 2,
            w: bpm.w + 8,
            h: bpm.h + 11,
        };
        let placements = picked.scene_placements(Screen::Waiting);
        let effect_rects = [
            ("IDC_IMAGE_SWAIT_EFFECT_1", 0x1650_0045u32),
            ("IDC_IMAGE_SWAIT_EFFECT_2", 0x1650_0046u32),
        ]
        .map(|(id, code)| {
            let placed = placements
                .iter()
                .find(|placed| placed.id == id)
                .unwrap_or_else(|| panic!("{id} placement missing from staged Waiting art"));
            assert_eq!(
                placed.code, code,
                "the pinned mini-card control code must match the staged rect"
            );
            placed.rect
        });
        let bare_frame = painted_pixels(&mut bare, 0);
        let picked_frame = painted_pixels(&mut picked, 0);
        let changed = first_change_outside_allowed(
            &bare_frame,
            &picked_frame,
            width,
            &[seat_grid, bpm_with_scrim, effect_rects[0], effect_rects[1]],
        );
        // #535: the pick's BPM caption sits below the covers, so it
        // may ink there; #883 adds the two default `?` mini-card rects.
        // Every other pixel below the grid stays put.
        assert_eq!(
            changed, None,
            "the pick inked outside the seat grid, BPM and pinned mini-cards: {changed:?}"
        );
        assert_ne!(
            o2jam_render::pixels_in_rect(&bare_frame, width, bpm),
            o2jam_render::pixels_in_rect(&picked_frame, width, bpm),
            "the BPM caption must follow the staged pick"
        );
        for rect in effect_rects {
            assert_ne!(
                o2jam_render::pixels_in_rect(&bare_frame, width, rect),
                o2jam_render::pixels_in_rect(&picked_frame, width, rect),
                "the default entry state must ink each pinned ? mini card"
            );
        }
    }

    /// Pinned `LEVEL` badge rect of the live-staged Waiting art
    /// (`State_Waiting.bnd` rect 26, paired by code ordinal - read from
    /// the staged placements, never a literal).
    fn staged_level_rect(game: &ScreenGame) -> o2jam_render::Rect {
        game.scene_placements(Screen::Waiting)
            .iter()
            .find(|placed| placed.id == "IDC_IMAGE_SWAIT_LEVEL")
            .map(|placed| placed.rect)
            .expect("LEVEL placement missing from staged Waiting art")
    }

    /// Pixels of one staged `Wait_Level.ojs` frame that the painted
    /// buffer does not carry, counted over the frame's own non-colorkey
    /// pixels drawn at the badge rect origin. Zero means the badge
    /// shows exactly that frame.
    fn level_frame_mismatches(
        painted: &[u32],
        width: usize,
        rect: o2jam_render::Rect,
        image: &o2jam_render::Rgb555Image,
        colorkey: u16,
    ) -> usize {
        let left = usize::try_from(rect.x).unwrap_or(usize::MAX);
        let top = usize::try_from(rect.y).unwrap_or(usize::MAX);
        let mut mismatches = 0;
        for y in 0..image.h {
            for x in 0..image.w {
                let word = image.words[y * image.w + x];
                if word == colorkey {
                    continue;
                }
                let (r, g, b) = o2jam_render::expand_rgb555(word);
                let expected = o2jam_render::pack_xrgb(r, g, b);
                let pixel = painted.get((top + y) * width + left + x);
                if pixel != Some(&expected) {
                    mismatches += 1;
                }
            }
        }
        mismatches
    }

    #[test]
    fn live_waiting_badge_paints_the_bound_level_frame() {
        // Live chain: main.rs::run_play -> setup_online ->
        //   stage_online_identity -> stage_live_scenes ->
        //   stage_interface_scenes(State::Waiting) ->
        //   ScreenGame::set_selected_music (session::OnlineSession::
        //   resolve_song_select) -> ScreenGame::paint_at -> paint_waiting
        //   -> paint_waiting_scene -> paint_waiting_staged ->
        //   paint_waiting_level_badge
        // Ticket #275: the `LEVEL` badge paints exactly the staged
        // `Wait_Level.ojs` frame `12*difficulty + speed` the pick names -
        // the EX cell (green) at normal speed and the HX cell (red) at
        // 4x differ, and each reproduces its own frame's pixels. The
        // room that names no song paints no badge frame at all.
        //
        // Archives absent -> skip cleanly (CI has no game data).
        if skip_without_archives() {
            return;
        }
        let Some(songs) = installed_catalog_songs() else {
            println!("skip: missing installed song catalog");
            return;
        };
        let Some(staged) = declared_interface(State::Waiting) else {
            println!("skip: STATE_WAITING does not stage");
            return;
        };
        let body = staged
            .figures
            .get("Wait_Level.ojs")
            .cloned()
            .expect("the staged LEVEL control names Wait_Level.ojs");
        assert_eq!(
            body.len(),
            60,
            "Wait_Level.ojs carries 5 families x 12 cells"
        );
        let song_id = u16::try_from(songs[0].song_id).expect("catalog song id fits u16");
        // #761: the badge family rides the catalog EX level word - the
        // exe shifts EX charts above level 3 to the MX family (+0x30,
        // files 0x4b3372-0x4b33b7). Pin the level by re-staging the
        // catalog on the built games (paint reads the level lazily).
        fn pin_ex_level(game: &mut ScreenGame, level: u16) {
            let bytes = std::fs::read(install_root().join("O2Jam/Image/OJNList.dat"))
                .expect("installed catalog bytes");
            let mut entries = o2jam_file::parse_ojn_list(&bytes).expect("installed catalog parses");
            entries[0].level_flags = u32::from(level);
            game.set_song_catalog(&entries);
        }
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        let badge = |difficulty: u8, speed: u8| o2jam_net::SetRoomMusic {
            music_id: song_id,
            difficulty,
            speed,
        };
        let mut easy = staged_waiting_pick_game(0, Some(badge(0, 1)));
        pin_ex_level(&mut easy, 2);
        let easy_frame = painted_pixels(&mut easy, 0);
        let rect = staged_level_rect(&easy);
        let mut hard = staged_waiting_pick_game(0, Some(badge(2, 7)));
        pin_ex_level(&mut hard, 2);
        let hard_frame = painted_pixels(&mut hard, 0);
        // #761's MX shift: an EX chart above level 3 selects the MX
        // family - layer 0x30 + speed, not the EX cell.
        let mut mx = staged_waiting_pick_game(0, Some(badge(0, 1)));
        pin_ex_level(&mut mx, 5);
        let mx_frame = painted_pixels(&mut mx, 0);
        for (frames, layer, name) in [
            (&easy_frame, 1usize, "EX1 (difficulty 0, speed 1, level 2)"),
            (&hard_frame, 31usize, "HX4 (difficulty 2, speed 7)"),
            (&mx_frame, 0x30usize + 1, "MX1 (EX chart at level 5)"),
        ] {
            let frame = body.layer(layer).expect("staged badge frame");
            assert_eq!(
                level_frame_mismatches(frames, width, rect, &frame.image, frame.colorkey),
                0,
                "the badge must show the {name} frame of Wait_Level.ojs"
            );
        }
        let mut bare = staged_waiting_pick_game(0, None);
        let bare_frame = painted_pixels(&mut bare, 0);
        let frame = body.layer(1).expect("staged EX1 frame");
        assert!(
            level_frame_mismatches(&bare_frame, width, rect, &frame.image, frame.colorkey) > 0,
            "a room with no song must not paint the EX1 badge frame"
        );
    }

    #[test]
    fn loading_paint_uses_staged_gate_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Loading, 0);
        assert_ne!(bare, staged);
    }

    #[test]
    fn playing_paint_uses_staged_venue_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let mut bare = live_game();
        bare.load_chart(vec![(2u8, 60_000u64, 7u32)]);
        walk_to(&mut bare, Screen::Playing);
        let bare_pixels = painted_pixels(&mut bare, 0);
        let mut staged = live_game();
        staged.load_chart(vec![(2u8, 60_000u64, 7u32)]);
        stage_live_scenes(&mut staged);
        walk_to(&mut staged, Screen::Playing);
        assert_eq!(staged.screen(), Screen::Playing);
        let staged_pixels = painted_pixels(&mut staged, 0);
        assert_ne!(bare_pixels, staged_pixels);
    }

    #[test]
    fn result_paint_uses_staged_banner_when_archives_present() {
        if skip_without_archives() {
            return;
        }
        let (bare, staged) = paint_pair(Screen::Playing, 0);
        assert_ne!(bare, staged);
    }

    #[test]
    fn live_stage_inks_the_shop_price_row_with_archive_sprite_glyphs() {
        // Live caller chain (ticket #877): main.rs::run_play ->
        // session::OnlineSession::stage_live_art -> stage_live_scenes
        // -> stage_interface_scenes -> stage_state(State::Shop) ->
        // stage_shop_price_fonts -> staged_shop_price_fonts ->
        // ScreenGame::visit_side -> ScreenGame::mount_staged_side_art
        // -> ScreenGame::paint_at -> ScreenGame::paint_shop ->
        // paint_shop_board -> paint_shop_featured_captions ->
        // paint_shop_card_price_row -> paint_shop_price_sprite_row.
        // The revision this guard protects replaced modern-font ink
        // with staged sprite frames; a seam-only guard could not see
        // a break in the live staging arm above.
        let root = install_root();
        if !root_has_game_data(&root) {
            println!(
                "{}",
                pack_skip_report(&root, "live_stage_inks_shop_price_row_with_sprite_glyphs")
            );
            return;
        }
        let Some(source) = o2jam_shell::read_installed_avatar(&root) else {
            println!(
                "{}",
                pack_skip_report(&root, "live_stage_inks_shop_price_row_with_sprite_glyphs")
            );
            return;
        };
        let mut game = live_game();
        game.set_decoded_avatar_source(&source);
        game.stage_exe_item_shop_catalog(source.table());
        stage_live_scenes(&mut game);
        walk_to(&mut game, Screen::Room);
        assert!(game.visit_side(SideScreen::Shop), "live Shop visit opens");
        assert!(
            game.shop_entry_catalog(),
            "featured cards stage from the catalog"
        );
        assert!(
            o2jam_shell::staging::staged_shop_price_fonts().is_some(),
            "the live Shop stage must publish both runtime font bodies"
        );

        let rendered_core = |word: u16| {
            let (red, green, blue) = o2jam_file::rgb555_to_rgb888(word);
            o2jam_render::pack_xrgb(red, green, blue)
        };
        let pixels = painted_pixels(&mut game, 0);
        let width = usize::try_from(NATIVE_WIDTH).unwrap_or(1);
        // The archive frames carry 11 pixels of green core `0x3706`
        // in frame 11 and 4 of gold core `0x7F20` in frame 10. The
        // no-data text fallback inks different colors (106,197,49)
        // and (255,205,0), so these counts name the sprite raster.
        assert_eq!(
            core_pixels(&pixels, width, 225, 13, rendered_core(0x3706)),
            11,
            "won prefix paints the staged green sprite core"
        );
        assert_eq!(
            core_pixels(&pixels, width, 317, 9, rendered_core(0x7F20)),
            4,
            "gem prefix paints the staged gold sprite core"
        );
    }
}
