//! Scene staging declaration: one row per State.
//!
//! The client shell stages every bound State through this table —
//! the archive bytes, the [`State`], and the [`StageContext`] cross
//! one seam and staged art comes back. Game rules still live in
//! `o2jam-game`; this module never stores into the game.

use std::collections::HashMap;
use std::path::Path;

use o2jam_file::{Bnd, ControlList};
use o2jam_render::{Rect, NATIVE_HEIGHT, NATIVE_WIDTH};

use crate::avatar::{
    bind_scene_rects, load_archive_sprite_for_stage, load_scene_figures_for_room,
    playing_member_selected, scene_block, scene_sprite_names, AvatarFigure, PlayingArtMode,
    ScenePlacement, LOADING_SPRITE,
};
use crate::screen::Screen;
use crate::screens::side::{SideScreen, TUTORIAL_BACKGROUND_SPRITE};

/// State value keyed by its `STATE_*` Block name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Room,
    Waiting,
    Shop,
    MusicShop,
    MyRoom,
    Planet,
    Lanking,
    Result,
    Payment,
    Payment2,
    Playing,
    Avi,
    Loading,
    Tutorial,
}

impl State {
    /// `ControlList` Block this State stages.
    #[must_use]
    pub const fn block_name(self) -> &'static str {
        match self {
            State::Room => "STATE_ROOM",
            State::Waiting => "STATE_WAITING",
            State::Shop => "STATE_SHOP",
            State::MusicShop => "STATE_MUSICSHOP",
            State::MyRoom => "STATE_MYROOM",
            State::Planet => "STATE_PLANET",
            State::Lanking => "STATE_LANKING",
            State::Result => "STATE_RESULT",
            State::Payment => "STATE_PAYMENT",
            State::Payment2 => "STATE_PAYMENT2",
            State::Playing => "STATE_PLAYING",
            State::Avi => "STATE_AVI",
            State::Loading => "STATE_LOADING",
            State::Tutorial => "STATE_TUTORIAL",
        }
    }

    /// State for a Block name, if the declaration keys one. Every
    /// State answers its own name, even the block-less full-canvas
    /// rows; Dialog Blocks and unknown names stay `None`.
    #[must_use]
    pub fn from_block(name: &str) -> Option<State> {
        ALL_STATES
            .iter()
            .copied()
            .find(|state| state.block_name() == name)
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.block_name())
    }
}

/// Every State the declaration keys, in table order.
const ALL_STATES: &[State] = &[
    State::Room,
    State::Waiting,
    State::Shop,
    State::MusicShop,
    State::MyRoom,
    State::Planet,
    State::Lanking,
    State::Result,
    State::Payment,
    State::Payment2,
    State::Playing,
    State::Avi,
    State::Loading,
    State::Tutorial,
];

/// Which placement rule stages a row: the bound `.bnd` pairing, or
/// one full-canvas placement per staged sprite (art without a
/// `.bnd`: the Avi logo, the Loading gate, the Tutorial backdrop).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementArm {
    /// Block entries paired with the `.bnd` by code ordinal.
    Bound,
    /// One full-canvas placement per staged sprite (art without
    /// a `.bnd`: the Avi logo, the Loading gate, the Tutorial
    /// backdrop).
    FullCanvas,
    /// Playing subset: behind entries, backdrops, then the bound
    /// run (see `bind_scene_rects`); figures follow the context.
    PlayingSubset,
}

/// Which archive a row stages from: the Interface art or the
/// Playing art under `O2Jam/Image/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingArchive {
    Interface,
    Playing,
}

impl StagingArchive {
    /// `ControlList` entry holding this archive's Blocks.
    #[must_use]
    pub const fn list_file(self) -> &'static str {
        match self {
            StagingArchive::Interface => INTERFACE_LIST,
            StagingArchive::Playing => PLAYING_LIST,
        }
    }
}

/// One declaration row per State: the Block to stage (absent for
/// the block-less full-canvas rows), the archive it stages from,
/// the placement arm, plus extra sprite bodies merged in by name.
/// Result carries the exe-mounted verdict banner (`Result_WinLose.ojs`,
/// file `0xc2390`) because its Block names only the backdrop and
/// plates; Loading and Tutorial carry their single bodies by name
/// because no 7.55 list names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageRow {
    /// State this row stages.
    pub state: State,
    /// `ControlList` Block to stage, if any list carries art for
    /// this State.
    pub block: Option<&'static str>,
    /// Archive holding the Block and the bodies.
    pub archive: StagingArchive,
    /// Placement rule: bound `.bnd` pairing or full canvas.
    pub arm: PlacementArm,
    /// Bodies merged by name after the bound run.
    pub extra_sprites: &'static [&'static str],
}

/// The staging declaration, read as data: one row per State. The
/// bound rows pair their Block with its `.bnd`; Avi, Loading, and
/// Tutorial stage full-canvas; Playing stages the subset through
/// the context. Dialog (ticket #5) rows extend this table.
pub const STAGE_TABLE: &[StageRow] = &[
    StageRow {
        state: State::Room,
        block: Some("STATE_ROOM"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Waiting,
        block: Some("STATE_WAITING"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Shop,
        block: Some("STATE_SHOP"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::MusicShop,
        block: Some("STATE_MUSICSHOP"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::MyRoom,
        block: Some("STATE_MYROOM"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Planet,
        block: Some("STATE_PLANET"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Lanking,
        block: Some("STATE_LANKING"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Result,
        block: Some("STATE_RESULT"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &["Result_WinLose.ojs"],
    },
    StageRow {
        state: State::Payment,
        block: Some("STATE_PAYMENT"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Payment2,
        block: Some("STATE_PAYMENT2"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::Bound,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Playing,
        block: Some("STATE_PLAYING"),
        archive: StagingArchive::Playing,
        arm: PlacementArm::PlayingSubset,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Avi,
        block: Some("STATE_AVI"),
        archive: StagingArchive::Interface,
        arm: PlacementArm::FullCanvas,
        extra_sprites: &[],
    },
    StageRow {
        state: State::Loading,
        block: None,
        archive: StagingArchive::Playing,
        arm: PlacementArm::FullCanvas,
        extra_sprites: &[LOADING_SPRITE],
    },
    StageRow {
        state: State::Tutorial,
        block: None,
        archive: StagingArchive::Playing,
        arm: PlacementArm::FullCanvas,
        extra_sprites: &[TUTORIAL_BACKGROUND_SPRITE],
    },
];

/// Control-list entry holding every Interface Block.
const INTERFACE_LIST: &str = "ControlList_Interface.txt";
/// Control-list entry holding every Playing Block.
const PLAYING_LIST: &str = "ControlList_Playing.txt";

/// Stage-side inputs for one staging call. Playing is the only
/// consumer today (arena venue, key mode, team flag); the bound arm
/// threads only the arena stage (`_` runtime refs), while the Playing
/// subset reads the whole context.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StageContext {
    /// Room arena stage for `_` runtime refs.
    pub arena_stage: Option<u16>,
    /// Wire key mode (`0` 3-key, `1` 5-key, `2` 7-key).
    pub key_mode: u8,
    /// Team/versus room.
    pub team_play: bool,
}

/// Art one staging call returns: layout plus figures. The caller
/// stores it (the game keeps its `load_*` slots); this module never
/// holds the game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StagedArt {
    /// Placements in block file order: `.bnd` rects on the bound
    /// arm, one full-canvas placement per staged sprite otherwise.
    pub placements: Vec<ScenePlacement>,
    /// Decoded bodies by archive name; a missed body leaves its
    /// slot empty while its placement survives.
    pub figures: HashMap<String, AvatarFigure>,
}

/// One staging miss: the State, the Block, and the entry name name
/// the hole. `entry` is the missing body name, the missing Block
/// name, or the missing bind-file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageMiss {
    /// State being staged.
    pub state: State,
    /// Block being staged.
    pub block: &'static str,
    /// Missing body, Block, or bind-file name.
    pub entry: String,
}

/// Outcome of one staging call: the art plus the miss ledger. Every
/// miss also logs a `tracing` warning with the same three names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageReport {
    /// Staged art, if the row resolved to at least one placement.
    /// A missing Block or `.bnd` stages nothing.
    pub art: Option<StagedArt>,
    /// One entry per miss, in stage order.
    pub misses: Vec<StageMiss>,
}

/// Stage one State from archive bytes through its row's arm: the
/// bound Block paired with its `.bnd` (the avatar kit's pairing),
/// one full-canvas placement per staged sprite, or the Playing
/// subset (behind entries, backdrops, then the bound run, with
/// figures through the context's room mode) — plus the row's
/// extra bodies merged by name. Returns the art with the miss
/// ledger; each miss logs a warning. Never fails a Scene: a
/// missing Block or `.bnd` contributes nothing, a missing body
/// keeps its placement with an empty figure slot.
#[must_use]
pub fn stage_state(archive: &[u8], state: State, ctx: StageContext) -> StageReport {
    let Some(row) = STAGE_TABLE.iter().find(|row| row.state == state) else {
        return StageReport::default();
    };
    match row.arm {
        PlacementArm::Bound => stage_bound_row(archive, row, ctx),
        PlacementArm::FullCanvas => stage_full_canvas_row(archive, row, ctx),
        PlacementArm::PlayingSubset => stage_playing_row(archive, row, ctx),
    }
}

/// Stage one bound row: the Block paired with its `.bnd`, plus the
/// row's extra bodies merged by name.
fn stage_bound_row(archive: &[u8], row: &StageRow, ctx: StageContext) -> StageReport {
    let mut report = StageReport::default();
    let Some(block_name) = row.block else {
        return report;
    };
    let list_file = row.archive.list_file();
    let Some(list) = read_control_list(archive, list_file) else {
        push_miss(&mut report, row.state, block_name, list_file);
        return report;
    };
    let Some(block) = scene_block(&list, block_name) else {
        push_miss(&mut report, row.state, block_name, block_name);
        return report;
    };
    let names = scene_sprite_names(block);
    let Some(bind_name) = block.bound_file.clone() else {
        push_miss(&mut report, row.state, block_name, "(no BOUND line)");
        return report;
    };
    let Some(bind) = read_bind(archive, &bind_name) else {
        push_miss(&mut report, row.state, block_name, &bind_name);
        return report;
    };
    let mut art = StagedArt {
        placements: bind_scene_rects(block, &bind),
        figures: HashMap::new(),
    };
    load_figures(
        archive,
        row,
        ctx,
        &staged_names(&names, row.extra_sprites),
        &mut art,
        &mut report,
    );
    if art.placements.is_empty() {
        return report;
    }
    report.art = Some(art);
    report
}

/// Stage one full-canvas row: the Block's sprite names (when a
/// list carries the Block) plus the row's extra bodies, each on
/// its own full-canvas placement. Block-less rows skip the list
/// lookup with no miss: no 7.55 list carries their art.
fn stage_full_canvas_row(archive: &[u8], row: &StageRow, ctx: StageContext) -> StageReport {
    let mut report = StageReport::default();
    let block_names: Vec<String> = if let Some(block_name) = row.block {
        let list_file = row.archive.list_file();
        let Some(list) = read_control_list(archive, list_file) else {
            push_miss(&mut report, row.state, block_name, list_file);
            return report;
        };
        let Some(block) = scene_block(&list, block_name) else {
            push_miss(&mut report, row.state, block_name, block_name);
            return report;
        };
        scene_sprite_names(block)
    } else {
        Vec::new()
    };
    let names = staged_names(&block_names, row.extra_sprites);
    let mut art = StagedArt {
        placements: full_canvas_placements(&names),
        figures: HashMap::new(),
    };
    load_figures(archive, row, ctx, &names, &mut art, &mut report);
    if art.placements.is_empty() {
        return report;
    }
    report.art = Some(art);
    report
}

/// Stage one Playing-subset row: behind entries, backdrops, then
/// the bound run, with figures through the context's room mode
/// (unselected family members leave empty slots with no miss),
/// plus the row's extra bodies merged by name.
fn stage_playing_row(archive: &[u8], row: &StageRow, ctx: StageContext) -> StageReport {
    let mut report = StageReport::default();
    let Some(block_name) = row.block else {
        return report;
    };
    let list_file = row.archive.list_file();
    let Some(list) = read_control_list(archive, list_file) else {
        push_miss(&mut report, row.state, block_name, list_file);
        return report;
    };
    let Some(block) = scene_block(&list, block_name) else {
        push_miss(&mut report, row.state, block_name, block_name);
        return report;
    };
    let names = scene_sprite_names(block);
    let Some(bind_name) = block.bound_file.clone() else {
        push_miss(&mut report, row.state, block_name, "(no BOUND line)");
        return report;
    };
    let Some(bind) = read_bind(archive, &bind_name) else {
        push_miss(&mut report, row.state, block_name, &bind_name);
        return report;
    };
    let mut art = StagedArt {
        placements: bind_scene_rects(block, &bind),
        figures: load_playing_figures(archive, &names, ctx, &mut report, row.state, block_name),
    };
    for extra in row.extra_sprites {
        match load_archive_sprite_for_stage(archive, extra, ctx.arena_stage) {
            Some(figure) => {
                art.figures.insert((*extra).to_owned(), figure);
            }
            None => push_miss(&mut report, row.state, block_name, extra),
        }
    }
    if art.placements.is_empty() {
        return report;
    }
    report.art = Some(art);
    report
}

/// Playing-subset figures through the room mode in the context: only
/// the family members the context selects load (the rest leave empty
/// slots without a miss), and the selected note background loads its
/// exe-selected frame only. A selected member that fails to decode
/// leaves its slot empty with a miss, like the bound arm.
fn load_playing_figures(
    archive: &[u8],
    names: &[String],
    ctx: StageContext,
    report: &mut StageReport,
    state: State,
    block: &'static str,
) -> HashMap<String, AvatarFigure> {
    let mode = PlayingArtMode {
        key_mode: ctx.key_mode,
        team_play: ctx.team_play,
    };
    let figures = load_scene_figures_for_room(archive, names, ctx.arena_stage, &mode);
    for name in names {
        if is_scrollbar_params(name) || !playing_member_selected(name, &mode) {
            continue;
        }
        if !figures.contains_key(name) {
            push_miss(report, state, block, name);
        }
    }
    figures
}

/// Block sprite names plus the row's extras merged in by name,
/// without repeats: one entry per staged body.
fn staged_names(block_names: &[String], extras: &[&str]) -> Vec<String> {
    let mut names: Vec<String> = block_names.to_vec();
    for extra in extras {
        if !names.iter().any(|name| name == extra) {
            names.push((*extra).to_owned());
        }
    }
    names
}

/// One full-canvas placement per staged sprite on the native
/// canvas: art without a `.bnd` fills the frame.
fn full_canvas_placements(names: &[String]) -> Vec<ScenePlacement> {
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

/// Decode every staged body through the room arena stage, skipping
/// scrollbar parameters (never files). A missing or undecodable
/// body logs its miss and leaves its slot empty while its
/// placement survives.
fn load_figures(
    archive: &[u8],
    row: &StageRow,
    ctx: StageContext,
    names: &[String],
    art: &mut StagedArt,
    report: &mut StageReport,
) {
    let block = row.block.unwrap_or(row.state.block_name());
    for name in names {
        if is_scrollbar_params(name) {
            continue;
        }
        match load_archive_sprite_for_stage(archive, name, ctx.arena_stage) {
            Some(figure) => {
                art.figures.insert(name.clone(), figure);
            }
            None => push_miss(report, row.state, block, name),
        }
    }
}

/// Record one miss and log its warning with the State, the Block,
/// and the entry name.
fn push_miss(report: &mut StageReport, state: State, block: &'static str, entry: &str) {
    tracing::warn!(state = %state, block, entry, "scene staging miss: slot stays empty");
    report.misses.push(StageMiss {
        state,
        block,
        entry: entry.to_owned(),
    });
}

/// True for an `O2_SBS_*` scrollbar parameter: never a file
/// (recipe step 9), so never a miss.
fn is_scrollbar_params(name: &str) -> bool {
    name.starts_with("O2_SBS_")
}

/// Parse a `ControlList_*.txt` entry from an archive.
fn read_control_list(archive: &[u8], name: &str) -> Option<ControlList> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = o2jam_file::find_opi_entry(&entries, name)?;
    let body = o2jam_file::opi_entry_body(archive, entry).ok()?;
    let text = o2jam_file::decode_control_text(body).ok()?;
    o2jam_file::parse_control_list(&text).ok()
}

/// Parse a `.bnd` layout entry from an archive.
fn read_bind(archive: &[u8], name: &str) -> Option<Bnd> {
    let entries = o2jam_file::parse_opi(archive).ok()?;
    let entry = o2jam_file::find_opi_entry(&entries, name)?;
    let body = o2jam_file::opi_entry_body(archive, entry).ok()?;
    o2jam_file::parse_bnd(body).ok()
}

/// State behind a main-flow Screen, if the declaration stages it.
/// Channel has no State class of its own.
#[must_use]
pub const fn state_of_screen(screen: Screen) -> Option<State> {
    match screen {
        Screen::Room => Some(State::Room),
        Screen::Waiting => Some(State::Waiting),
        Screen::Planet => Some(State::Planet),
        Screen::Result => Some(State::Result),
        Screen::Shop => Some(State::Shop),
        Screen::MusicShop => Some(State::MusicShop),
        Screen::Myroom => Some(State::MyRoom),
        Screen::Lanking => Some(State::Lanking),
        Screen::Avi => Some(State::Avi),
        Screen::Loading => Some(State::Loading),
        Screen::Playing => Some(State::Playing),
        Screen::Channel => None,
    }
}

/// State behind a side Screen, if the declaration stages it.
/// Payment names the billing view; its gift twin merges caller-side.
#[must_use]
pub const fn state_of_side(side: SideScreen) -> Option<State> {
    match side {
        SideScreen::Shop => Some(State::Shop),
        SideScreen::MusicShop => Some(State::MusicShop),
        SideScreen::MyRoom => Some(State::MyRoom),
        SideScreen::Planet => Some(State::Planet),
        SideScreen::Lanking => Some(State::Lanking),
        SideScreen::Payment => Some(State::Payment),
        SideScreen::Tutorial => Some(State::Tutorial),
    }
}

/// Interface archive path under an install root.
pub const INTERFACE_ARCHIVE_RELATIVE: &str = "O2Jam/Image/Interface1.opi";
/// Playing archive path under an install root.
pub const PLAYING_ARCHIVE_RELATIVE: &str = "O2Jam/Image/Playing1.opi";

/// Archives read from an install root: `None` per missing file, so
/// callers keep their flat fallbacks with no game data present.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallArchives {
    /// Interface art (`ControlList_Interface.txt` Blocks).
    pub interface: Option<Vec<u8>>,
    /// Playing art (Loading gate, Playing gauges).
    pub playing: Option<Vec<u8>>,
}

/// Read the staging archives under `root`: the repository-local
/// `install-root/` for development and tests, the client's own
/// install directory for the shipped game. No `env!`: the edge
/// that boots the client decides the root.
#[must_use]
pub fn read_install_archives(root: &Path) -> InstallArchives {
    InstallArchives {
        interface: read_archive_file(&root.join(INTERFACE_ARCHIVE_RELATIVE)),
        playing: read_archive_file(&root.join(PLAYING_ARCHIVE_RELATIVE)),
    }
}

/// Read one archive read-only, if present.
fn read_archive_file(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

#[cfg(test)]
mod staging_tests {
    use super::*;

    #[test]
    fn states_key_by_their_block_names() {
        assert_eq!(State::Room.block_name(), "STATE_ROOM");
        assert_eq!(State::Waiting.block_name(), "STATE_WAITING");
        assert_eq!(State::Shop.block_name(), "STATE_SHOP");
        assert_eq!(State::MusicShop.block_name(), "STATE_MUSICSHOP");
        assert_eq!(State::MyRoom.block_name(), "STATE_MYROOM");
        assert_eq!(State::Planet.block_name(), "STATE_PLANET");
        assert_eq!(State::Lanking.block_name(), "STATE_LANKING");
        assert_eq!(State::Result.block_name(), "STATE_RESULT");
        assert_eq!(State::Payment.block_name(), "STATE_PAYMENT");
        assert_eq!(State::Payment2.block_name(), "STATE_PAYMENT2");
        assert_eq!(State::Playing.block_name(), "STATE_PLAYING");
        assert_eq!(State::Avi.block_name(), "STATE_AVI");
        assert_eq!(State::Loading.block_name(), "STATE_LOADING");
        assert_eq!(State::Tutorial.block_name(), "STATE_TUTORIAL");
    }

    #[test]
    fn block_name_round_trips_through_from_block() {
        for state in [
            State::Room,
            State::Waiting,
            State::Shop,
            State::MusicShop,
            State::MyRoom,
            State::Planet,
            State::Lanking,
            State::Result,
            State::Payment,
            State::Payment2,
            State::Playing,
            State::Avi,
            State::Loading,
            State::Tutorial,
        ] {
            assert_eq!(State::from_block(state.block_name()), Some(state));
        }
    }

    #[test]
    fn from_block_rejects_unknown_names() {
        assert_eq!(State::from_block("DIALOG_CHANNEL_INFO"), None);
        assert_eq!(State::from_block(""), None);
    }

    #[test]
    fn declaration_holds_expected_rows_per_state() {
        assert_eq!(STAGE_TABLE.len(), 14);
        for row in STAGE_TABLE {
            if let Some(block) = row.block {
                assert_eq!(block, row.state.block_name());
            }
        }
        let bound = STAGE_TABLE
            .iter()
            .filter(|row| row.arm == PlacementArm::Bound)
            .count();
        assert_eq!(bound, 10, "ticket #3 rows stay bound");
        let playing = STAGE_TABLE
            .iter()
            .find(|row| row.state == State::Playing)
            .expect("playing row staged");
        assert_eq!(playing.arm, PlacementArm::PlayingSubset);
        for state in [State::Avi, State::Loading, State::Tutorial] {
            let row = STAGE_TABLE
                .iter()
                .find(|row| row.state == state)
                .expect("full-canvas row staged");
            assert_eq!(row.arm, PlacementArm::FullCanvas);
        }
    }

    #[test]
    fn rows_read_their_own_control_list() {
        for row in STAGE_TABLE {
            if row.archive == StagingArchive::Playing {
                assert_eq!(
                    row.archive.list_file(),
                    "ControlList_Playing.txt",
                    "the Playing archive holds {:?}",
                    row.state
                );
            } else {
                assert_eq!(
                    row.archive.list_file(),
                    "ControlList_Interface.txt",
                    "{:?} stages an interface Block",
                    row.state
                );
            }
        }
    }

    #[test]
    fn playing_row_carries_no_extras() {
        let row = STAGE_TABLE
            .iter()
            .find(|row| row.state == State::Playing)
            .expect("playing row staged");
        assert_eq!(row.block, Some("STATE_PLAYING"));
        assert_eq!(row.archive, StagingArchive::Playing);
        assert!(
            row.extra_sprites.is_empty(),
            "the subset stages the block alone; the skin rides caller-side"
        );
    }

    #[test]
    fn result_row_merges_the_verdict_banner_by_name() {
        let row = STAGE_TABLE
            .iter()
            .find(|row| row.state == State::Result)
            .expect("result row staged");
        assert_eq!(
            row.extra_sprites,
            &["Result_WinLose.ojs"] as &[&str],
            "exe mounts the banner at file 0xc2390, outside the block"
        );
    }

    #[test]
    fn payment_rows_carry_no_extras() {
        for state in [State::Payment, State::Payment2] {
            let row = STAGE_TABLE
                .iter()
                .find(|row| row.state == state)
                .expect("payment row staged");
            assert!(
                row.extra_sprites.is_empty(),
                "both billing views stage through their own bound blocks"
            );
        }
    }

    #[test]
    fn full_canvas_rows_name_their_archive_and_extras() {
        let avi = STAGE_TABLE
            .iter()
            .find(|row| row.state == State::Avi)
            .expect("avi row staged");
        assert_eq!(avi.block, Some("STATE_AVI"));
        assert_eq!(avi.archive, StagingArchive::Interface);
        assert!(avi.extra_sprites.is_empty());
        let loading = STAGE_TABLE
            .iter()
            .find(|row| row.state == State::Loading)
            .expect("loading row staged");
        assert_eq!(loading.block, None, "no 7.55 list carries loading art");
        assert_eq!(loading.archive, StagingArchive::Playing);
        assert_eq!(loading.extra_sprites, &["MusicLoading.ojs"] as &[&str]);
        let tutorial = STAGE_TABLE
            .iter()
            .find(|row| row.state == State::Tutorial)
            .expect("tutorial row staged");
        assert_eq!(
            tutorial.block, None,
            "the interface stub names no bodies; the backdrop ships in Playing1"
        );
        assert_eq!(tutorial.archive, StagingArchive::Playing);
        assert_eq!(
            tutorial.extra_sprites,
            &["Playing_TutorialBG.ojs"] as &[&str]
        );
    }

    #[test]
    fn stage_context_defaults_to_stage_less_solo() {
        let ctx = StageContext::default();
        assert_eq!(ctx.arena_stage, None);
        assert_eq!(ctx.key_mode, 0);
        assert!(!ctx.team_play);
    }

    /// Minimal sprite-55 container: header + one record + blob.
    fn sprite55_blob(w: u16, h: u16, pixels: &[u8]) -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&w.to_le_bytes());
        blob.extend_from_slice(&h.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(pixels);
        blob
    }

    fn bnd_blob(rects: &[(u32, u32, u32, u32)]) -> Vec<u8> {
        let mut blob = vec![0xFF; 4];
        blob.extend_from_slice(&(rects.len() as u16).to_le_bytes());
        for (x1, y1, x2, y2) in rects {
            blob.extend_from_slice(&x1.to_le_bytes());
            blob.extend_from_slice(&y1.to_le_bytes());
            blob.extend_from_slice(&x2.to_le_bytes());
            blob.extend_from_slice(&y2.to_le_bytes());
        }
        blob
    }

    fn room_archive() -> Vec<u8> {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_LIST 0X13500001 \"Room_List.ojs\"",
            "IDC_BUTTON 0X13500002 \"Room_Button.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            (
                "State_Room.bnd",
                &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
            ),
            ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
            ("Room_Button.ojs", &sprite55_blob(2, 2, &[2; 8])),
        ])
    }

    #[test]
    fn avi_stages_its_logo_full_canvas_without_a_bind_file() {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_AVI 0x01",
            "{",
            "BOUND \"State_Avi.bnd\"",
            "IDC_STATE_AVI 0X18000000",
            "IDC_BACK_LOGO 0X18100001 \"O2mediaLogo.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            ("O2mediaLogo.ojs", &sprite55_blob(2, 2, &[1; 8])),
        ]);
        let report = stage_state(&archive, State::Avi, StageContext::default());
        assert!(report.misses.is_empty());
        let art = report.art.expect("avi stages with synthetic bytes");
        assert_eq!(art.placements.len(), 1, "one sprite, one canvas");
        assert_eq!(art.placements[0].sprite.as_deref(), Some("O2mediaLogo.ojs"));
        assert_eq!(art.placements[0].rect.x, 0);
        assert_eq!(art.placements[0].rect.y, 0);
        assert_eq!(art.placements[0].rect.w, 800, "native canvas width");
        assert_eq!(art.placements[0].rect.h, 600, "native canvas height");
        assert!(art.figures.contains_key("O2mediaLogo.ojs"));
    }

    #[test]
    fn loading_stages_its_named_body_with_no_block() {
        let archive = o2jam_file::test_archive::build_archive(&[(
            "MusicLoading.ojs",
            &sprite55_blob(2, 2, &[2; 8]),
        )]);
        let report = stage_state(&archive, State::Loading, StageContext::default());
        assert!(report.misses.is_empty(), "no block, no bind, no miss");
        let art = report.art.expect("loading stages with synthetic bytes");
        assert_eq!(art.placements.len(), 1);
        assert_eq!(
            art.placements[0].sprite.as_deref(),
            Some("MusicLoading.ojs")
        );
        assert_eq!(art.placements[0].rect.w, 800);
        assert_eq!(art.placements[0].rect.h, 600);
        assert!(art.figures.contains_key("MusicLoading.ojs"));
    }

    #[test]
    fn tutorial_stages_its_named_body_with_no_block() {
        let archive = o2jam_file::test_archive::build_archive(&[(
            "Playing_TutorialBG.ojs",
            &sprite55_blob(2, 2, &[3; 8]),
        )]);
        let report = stage_state(&archive, State::Tutorial, StageContext::default());
        assert!(report.misses.is_empty(), "no block, no bind, no miss");
        let art = report.art.expect("tutorial stages with synthetic bytes");
        assert_eq!(art.placements.len(), 1);
        assert_eq!(
            art.placements[0].sprite.as_deref(),
            Some("Playing_TutorialBG.ojs")
        );
        assert!(art.figures.contains_key("Playing_TutorialBG.ojs"));
    }

    #[test]
    fn full_canvas_miss_keeps_placement_with_empty_slot() {
        let archive = o2jam_file::test_archive::build_archive(&[]);
        let report = stage_state(&archive, State::Loading, StageContext::default());
        let art = report.art.expect("placements survive the miss");
        assert_eq!(art.placements.len(), 1);
        assert!(art.figures.is_empty());
        assert_eq!(report.misses.len(), 1);
        assert_eq!(report.misses[0].state, State::Loading);
        assert_eq!(report.misses[0].block, "STATE_LOADING");
        assert_eq!(report.misses[0].entry, "MusicLoading.ojs");
    }

    #[test]
    fn payment_views_stage_through_their_own_rows() {
        let list = [
            "NUMBER_OF_STATE 0x02",
            "STATE_PAYMENT 0x01",
            "{",
            "BOUND \"State_Payment.bnd\"",
            "IDC_PAY 0X1D000000",
            "IDC_BILL 0X1D100001 \"Pay_Bill.ojs\"",
            "}",
            "STATE_PAYMENT2 0x01",
            "{",
            "BOUND \"State_Payment2.bnd\"",
            "IDC_PAY2 0X1E000000",
            "IDC_GIFT 0X1E100001 \"Pay_Gift.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            ("State_Payment.bnd", &bnd_blob(&[(1, 2, 3, 4)])),
            ("State_Payment2.bnd", &bnd_blob(&[(5, 6, 7, 8)])),
            ("Pay_Bill.ojs", &sprite55_blob(2, 2, &[1; 8])),
            ("Pay_Gift.ojs", &sprite55_blob(2, 2, &[2; 8])),
        ]);
        let ctx = StageContext::default();
        let billing = stage_state(&archive, State::Payment, ctx);
        let gift = stage_state(&archive, State::Payment2, ctx);
        assert!(billing.misses.is_empty());
        assert!(gift.misses.is_empty());
        let billing = billing.art.expect("billing view stages");
        let gift = gift.art.expect("gift view stages");
        assert_eq!(billing.placements.len(), 1);
        assert_eq!(billing.placements[0].rect.x, 1);
        assert!(billing.figures.contains_key("Pay_Bill.ojs"));
        assert_eq!(gift.placements.len(), 1);
        assert_eq!(gift.placements[0].rect.x, 5);
        assert!(gift.figures.contains_key("Pay_Gift.ojs"));
    }

    #[test]
    fn room_state_stages_named_figures_at_bound_rects() {
        let report = stage_state(&room_archive(), State::Room, StageContext::default());
        assert!(report.misses.is_empty());
        let art = report.art.expect("room stages with synthetic bytes");
        assert_eq!(art.placements.len(), 2, "self row takes no rect");
        assert_eq!(art.placements[0].sprite.as_deref(), Some("Room_List.ojs"));
        assert_eq!(art.placements[1].sprite.as_deref(), Some("Room_Button.ojs"));
        assert_eq!(art.placements[0].rect.x, 10);
        assert_eq!(art.placements[1].rect.x, 50);
        assert!(art.figures.contains_key("Room_List.ojs"));
        assert!(art.figures.contains_key("Room_Button.ojs"));
    }

    fn result_archive() -> Vec<u8> {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_RESULT 0x01",
            "{",
            "BOUND \"State_Result.bnd\"",
            "IDC_RESULT 0X17000000",
            "IDC_BACK 0X17100001 \"State_Result.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            ("State_Result.bnd", &bnd_blob(&[(0, 0, 800, 600)])),
            ("State_Result.ojs", &sprite55_blob(2, 2, &[3; 8])),
            ("Result_WinLose.ojs", &sprite55_blob(2, 2, &[4; 8])),
        ])
    }

    #[test]
    fn result_state_merges_banner_by_name() {
        let report = stage_state(&result_archive(), State::Result, StageContext::default());
        assert!(report.misses.is_empty());
        let art = report.art.expect("result stages with synthetic bytes");
        assert_eq!(
            art.placements.len(),
            1,
            "banner mounts by code, not by rect"
        );
        assert!(art.figures.contains_key("State_Result.ojs"));
        assert!(
            art.figures.contains_key("Result_WinLose.ojs"),
            "exe mounts the banner at file 0xc2390"
        );
    }

    #[test]
    fn missing_body_keeps_block_with_empty_slot() {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_LIST 0X13500001 \"Room_List.ojs\"",
            "IDC_BUTTON 0X13500002 \"Room_Button.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            (
                "State_Room.bnd",
                &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
            ),
            ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
        ]);
        let report = stage_state(&archive, State::Room, StageContext::default());
        let art = report.art.expect("a missing body never fails the scene");
        assert_eq!(art.placements.len(), 2, "placements survive the miss");
        assert!(art.figures.contains_key("Room_List.ojs"));
        assert!(!art.figures.contains_key("Room_Button.ojs"));
        assert_eq!(report.misses.len(), 1);
        assert_eq!(report.misses[0].state, State::Room);
        assert_eq!(report.misses[0].block, "STATE_ROOM");
        assert_eq!(report.misses[0].entry, "Room_Button.ojs");
    }

    #[test]
    fn missing_block_contributes_nothing() {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_WAITING 0x00",
            "{",
            "BOUND \"State_Waiting.bnd\"",
            "IDC_WAITING 0X14000000",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[(
            "ControlList_Interface.txt",
            list.as_bytes(),
        )]);
        let report = stage_state(&archive, State::Room, StageContext::default());
        assert!(report.art.is_none());
        assert_eq!(report.misses.len(), 1);
        assert_eq!(report.misses[0].block, "STATE_ROOM");
        assert_eq!(report.misses[0].entry, "STATE_ROOM");
    }

    #[test]
    fn missing_control_list_contributes_nothing() {
        let archive = o2jam_file::test_archive::build_archive(&[]);
        let report = stage_state(&archive, State::Room, StageContext::default());
        assert!(report.art.is_none());
        assert_eq!(report.misses.len(), 1);
        assert_eq!(report.misses[0].entry, "ControlList_Interface.txt");
    }

    #[test]
    fn missing_bind_file_contributes_nothing() {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x01",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_LIST 0X13500001 \"Room_List.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
        ]);
        let report = stage_state(&archive, State::Room, StageContext::default());
        assert!(report.art.is_none());
        assert_eq!(report.misses.len(), 1);
        assert_eq!(report.misses[0].entry, "State_Room.bnd");
    }

    #[test]
    fn corrupt_body_behaves_as_missing() {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x01",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_LIST 0X13500001 \"Room_List.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            ("State_Room.bnd", &bnd_blob(&[(10, 20, 30, 40)])),
            ("Room_List.ojs", b"not a sprite body"),
        ]);
        let report = stage_state(&archive, State::Room, StageContext::default());
        let art = report.art.expect("a corrupt body never fails the scene");
        assert!(art.figures.is_empty());
        assert_eq!(report.misses.len(), 1);
        assert_eq!(report.misses[0].entry, "Room_List.ojs");
    }

    #[test]
    fn placements_survive_total_figure_miss() {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_LIST 0X13500001 \"Room_List.ojs\"",
            "IDC_BUTTON 0X13500002 \"Room_Button.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            (
                "State_Room.bnd",
                &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
            ),
        ]);
        let report = stage_state(&archive, State::Room, StageContext::default());
        let art = report.art.expect("hit tests still read placements");
        assert_eq!(art.placements.len(), 2);
        assert!(art.figures.is_empty());
        assert_eq!(report.misses.len(), 2);
    }

    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    #[derive(Debug, Default)]
    struct WarnFields {
        state: Option<String>,
        block: Option<String>,
        entry: Option<String>,
    }

    impl Visit for WarnFields {
        fn record_str(&mut self, field: &Field, value: &str) {
            match field.name() {
                "state" => self.state = Some(value.to_owned()),
                "block" => self.block = Some(value.to_owned()),
                "entry" => self.entry = Some(value.to_owned()),
                _ => {}
            }
        }
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            let rendered = format!("{value:?}");
            let text = rendered
                .strip_prefix('"')
                .and_then(|cut| cut.strip_suffix('"'))
                .unwrap_or(&rendered);
            match field.name() {
                "state" => self.state = Some(text.to_owned()),
                "block" => self.block = Some(text.to_owned()),
                "entry" => self.entry = Some(text.to_owned()),
                _ => {}
            }
        }
    }

    #[derive(Debug, Default)]
    struct WarnRecorder {
        warns: Mutex<Vec<WarnFields>>,
    }

    impl Subscriber for WarnRecorder {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, event: &Event<'_>) {
            if *event.metadata().level() != tracing::Level::WARN {
                return;
            }
            let mut fields = WarnFields::default();
            event.record(&mut fields);
            self.warns
                .lock()
                .expect("warn ledger readable")
                .push(fields);
        }
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
    }

    #[test]
    fn miss_logs_warning_with_state_block_and_entry() {
        // One process-global recorder: every thread's warns land
        // here, so the probe uses a body name no other test stages
        // and filters for it instead of counting.
        static GLOBAL_RECORDER: std::sync::OnceLock<Arc<WarnRecorder>> = std::sync::OnceLock::new();
        let recorder = Arc::clone(GLOBAL_RECORDER.get_or_init(|| {
            let recorder = Arc::new(WarnRecorder::default());
            tracing::dispatcher::set_global_default(tracing::dispatcher::Dispatch::new(
                Arc::clone(&recorder),
            ))
            .expect("global warn recorder installs once");
            tracing::callsite::rebuild_interest_cache();
            recorder
        }));
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x01",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_PROBE 0X13500001 \"Warn_Probe_Body.ojs\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            ("State_Room.bnd", &bnd_blob(&[(10, 20, 30, 40)])),
        ]);
        let _ = stage_state(&archive, State::Room, StageContext::default());
        let warns = recorder.warns.lock().expect("warn ledger readable");
        assert!(
            warns
                .iter()
                .any(|warn| warn.state.as_deref() == Some("STATE_ROOM")
                    && warn.block.as_deref() == Some("STATE_ROOM")
                    && warn.entry.as_deref() == Some("Warn_Probe_Body.ojs")),
            "miss warns with state, block, entry: {warns:?}"
        );
    }

    #[test]
    fn screen_view_maps_bound_scenes_to_states() {
        assert_eq!(state_of_screen(Screen::Room), Some(State::Room));
        assert_eq!(state_of_screen(Screen::Waiting), Some(State::Waiting));
        assert_eq!(state_of_screen(Screen::Planet), Some(State::Planet));
        assert_eq!(state_of_screen(Screen::Result), Some(State::Result));
        assert_eq!(state_of_screen(Screen::Shop), Some(State::Shop));
        assert_eq!(state_of_screen(Screen::MusicShop), Some(State::MusicShop));
        assert_eq!(state_of_screen(Screen::Myroom), Some(State::MyRoom));
        assert_eq!(state_of_screen(Screen::Lanking), Some(State::Lanking));
        assert_eq!(
            state_of_screen(Screen::Playing),
            Some(State::Playing),
            "the Playing subset stages through the declaration"
        );
    }

    #[test]
    fn screen_view_maps_full_canvas_scenes_to_states() {
        assert_eq!(state_of_screen(Screen::Avi), Some(State::Avi));
        assert_eq!(state_of_screen(Screen::Loading), Some(State::Loading));
    }

    #[test]
    fn screen_view_leaves_future_arms_unmapped() {
        assert_eq!(state_of_screen(Screen::Channel), None, "no State class");
    }

    /// Synthetic `ControlList_Playing.txt` text: the self row, four
    /// behind-highway scene entries, one backdrop panel, fillers to
    /// index 213, then the bound `EXIT`..`VOL` run. Mirrors the exe
    /// window the 17-rect `.bnd` pairs by ordinal.
    fn playing_list_text() -> String {
        let mut lines = vec![
            "NUMBER_OF_STATE 0x01".to_owned(),
            "STATE_PLAYING 0x07".to_owned(),
            "{".to_owned(),
            "BOUND \"State_Playing.bnd\"".to_owned(),
            "IDC_PLAYING 0X1B000000".to_owned(),
            "IDC_BACK1 0X1B500071 \"_Playing_BG.ojs\"".to_owned(),
            "IDC_BACK2 0X1B500072 \"Note_BG.ojs\"".to_owned(),
            "IDC_BACK3 0X1B500073 \"P2P7KeyNote_BG.ojs\"".to_owned(),
            "IDC_BACK4 0X1B500074 \"3Key_Note_BG.ojs\"".to_owned(),
            "IDC_MENU 0X1B500075 \"Playing_Menu.ojs\"".to_owned(),
        ];
        for index in 6..213 {
            lines.push(format!("IDC_FILL{index:03} 0X1B000000"));
        }
        lines.push("IDC_EXIT 0X1B000001".to_owned());
        lines.push("IDC_VOL 0X1B000002 \"Vol_Mark.ojs\"".to_owned());
        lines.push("}".to_owned());
        lines.push(String::new());
        lines.join("\n")
    }

    fn playing_archive_with(bodies: &[&str]) -> Vec<u8> {
        let mut entries: Vec<(&str, Vec<u8>)> = vec![
            ("ControlList_Playing.txt", playing_list_text().into_bytes()),
            (
                "State_Playing.bnd",
                bnd_blob(&[(1, 2, 3, 4), (10, 20, 30, 40)]),
            ),
        ];
        for body in bodies {
            entries.push((*body, sprite55_blob(2, 2, &[1; 8])));
        }
        let refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(name, blob)| (*name, blob.as_slice()))
            .collect();
        o2jam_file::test_archive::build_archive(&refs)
    }

    fn full_playing_archive() -> Vec<u8> {
        playing_archive_with(&[
            "Playing_BG2.ojs",
            "Note_BG.ojs",
            "P2P7KeyNote_BG.ojs",
            "3Key_Note_BG.ojs",
            "Playing_Menu.ojs",
            "Vol_Mark.ojs",
        ])
    }

    fn solo_seven_key_context() -> StageContext {
        StageContext {
            arena_stage: Some(2),
            key_mode: 2,
            team_play: false,
        }
    }

    #[test]
    fn playing_subset_orders_behind_backdrops_then_bound_run() {
        let report = stage_state(
            &full_playing_archive(),
            State::Playing,
            solo_seven_key_context(),
        );
        assert!(report.misses.is_empty(), "misses: {:?}", report.misses);
        let art = report.art.expect("playing stages with synthetic bytes");
        let sprites: Vec<Option<&str>> = art
            .placements
            .iter()
            .map(|placed| placed.sprite.as_deref())
            .collect();
        assert_eq!(
            sprites,
            vec![
                Some("_Playing_BG.ojs"),
                Some("Note_BG.ojs"),
                Some("P2P7KeyNote_BG.ojs"),
                Some("3Key_Note_BG.ojs"),
                Some("Playing_Menu.ojs"),
                None,
                Some("Vol_Mark.ojs"),
            ],
            "behind entries, backdrops, then the bound run"
        );
        assert_eq!(art.placements[5].rect.x, 1, "EXIT takes ordinal 1");
        assert_eq!(art.placements[6].rect.x, 10, "VOL takes ordinal 2");
    }

    #[test]
    fn playing_context_selects_solo_family_members() {
        let report = stage_state(
            &full_playing_archive(),
            State::Playing,
            solo_seven_key_context(),
        );
        let art = report.art.expect("playing stages with synthetic bytes");
        for staged in [
            "_Playing_BG.ojs",
            "Note_BG.ojs",
            "Playing_Menu.ojs",
            "Vol_Mark.ojs",
        ] {
            assert!(
                art.figures.contains_key(staged),
                "{staged} stages in a solo 7-key room"
            );
        }
        for missing in ["P2P7KeyNote_BG.ojs", "3Key_Note_BG.ojs"] {
            assert!(
                !art.figures.contains_key(missing),
                "unselected member {missing} leaves an empty slot"
            );
        }
    }

    #[test]
    fn playing_context_selects_team_family_members() {
        let ctx = StageContext {
            arena_stage: Some(2),
            key_mode: 2,
            team_play: true,
        };
        let report = stage_state(&full_playing_archive(), State::Playing, ctx);
        let art = report.art.expect("playing stages with synthetic bytes");
        assert!(art.figures.contains_key("P2P7KeyNote_BG.ojs"));
        assert!(!art.figures.contains_key("Note_BG.ojs"));
        assert!(!art.figures.contains_key("3Key_Note_BG.ojs"));
        assert_eq!(art.placements.len(), 7, "placements ignore the family");
    }

    #[test]
    fn playing_key_mode_selects_note_background() {
        let ctx = StageContext {
            arena_stage: Some(2),
            key_mode: 0,
            team_play: false,
        };
        let report = stage_state(&full_playing_archive(), State::Playing, ctx);
        let art = report.art.expect("playing stages with synthetic bytes");
        assert!(art.figures.contains_key("3Key_Note_BG.ojs"));
        assert!(!art.figures.contains_key("Note_BG.ojs"));
        assert!(!art.figures.contains_key("P2P7KeyNote_BG.ojs"));
    }

    #[test]
    fn playing_missing_body_keeps_placement_with_empty_slot() {
        let archive = playing_archive_with(&[
            "Playing_BG2.ojs",
            "Note_BG.ojs",
            "P2P7KeyNote_BG.ojs",
            "3Key_Note_BG.ojs",
            "Playing_Menu.ojs",
        ]);
        let report = stage_state(&archive, State::Playing, solo_seven_key_context());
        let art = report.art.expect("a missing body never fails the scene");
        assert_eq!(art.placements.len(), 7, "placements survive the miss");
        assert!(!art.figures.contains_key("Vol_Mark.ojs"));
        assert_eq!(report.misses.len(), 1);
        assert_eq!(report.misses[0].state, State::Playing);
        assert_eq!(report.misses[0].block, "STATE_PLAYING");
        assert_eq!(report.misses[0].entry, "Vol_Mark.ojs");
    }

    #[test]
    fn side_view_maps_bound_sides_to_states() {
        assert_eq!(state_of_side(SideScreen::Shop), Some(State::Shop));
        assert_eq!(state_of_side(SideScreen::MusicShop), Some(State::MusicShop));
        assert_eq!(state_of_side(SideScreen::MyRoom), Some(State::MyRoom));
        assert_eq!(state_of_side(SideScreen::Planet), Some(State::Planet));
        assert_eq!(state_of_side(SideScreen::Lanking), Some(State::Lanking));
        assert_eq!(
            state_of_side(SideScreen::Payment),
            Some(State::Payment),
            "gift twin merges caller-side"
        );
        assert_eq!(
            state_of_side(SideScreen::Tutorial),
            Some(State::Tutorial),
            "full-canvas arm, ticket #4"
        );
    }

    #[test]
    fn install_loader_reads_archives_from_given_root() {
        let root = test_scratch_root("reads");
        let interface_dir = root.join("O2Jam/Image");
        std::fs::create_dir_all(&interface_dir).expect("scratch dir writable");
        let interface = o2jam_file::test_archive::build_archive(&[("a.ojs", b"a")]);
        let playing = o2jam_file::test_archive::build_archive(&[("b.ojs", b"b")]);
        std::fs::write(interface_dir.join("Interface1.opi"), &interface)
            .expect("scratch file writable");
        std::fs::write(interface_dir.join("Playing1.opi"), &playing)
            .expect("scratch file writable");
        let loaded = read_install_archives(&root);
        assert_eq!(loaded.interface, Some(interface));
        assert_eq!(loaded.playing, Some(playing));
    }

    #[test]
    fn install_loader_skips_cleanly_without_game_data() {
        let root = test_scratch_root("missing");
        std::fs::create_dir_all(&root).expect("scratch dir writable");
        let loaded = read_install_archives(&root);
        assert_eq!(loaded, InstallArchives::default());
    }

    /// Scratch dir under `target/`: hermetic, never `/tmp`, and
    /// git-ignored with the rest of the build tree.
    fn test_scratch_root(case: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/staging-loader-test")
            .join(case)
    }

    #[test]
    fn scrollbar_params_never_warn() {
        let list = [
            "NUMBER_OF_STATE 0x01",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X13000000",
            "IDC_LIST 0X13500001 \"Room_List.ojs\"",
            "IDC_SCROLL 0X13021016 \"O2_SBS_VERT\t7\t\t100\"",
            "}",
            "",
        ]
        .join("\n");
        let archive = o2jam_file::test_archive::build_archive(&[
            ("ControlList_Interface.txt", list.as_bytes()),
            (
                "State_Room.bnd",
                &bnd_blob(&[(10, 20, 30, 40), (50, 60, 70, 80)]),
            ),
            ("Room_List.ojs", &sprite55_blob(2, 2, &[1; 8])),
        ]);
        let report = stage_state(&archive, State::Room, StageContext::default());
        assert!(report.art.is_some());
        assert!(
            report.misses.is_empty(),
            "scrollbar params are never files: {:?}",
            report.misses
        );
    }
}
