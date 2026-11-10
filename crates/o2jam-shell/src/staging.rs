//! Scene staging declaration: one row per State.
//!
//! The client shell stages every bound State through this table —
//! the archive bytes, the [`State`], and the [`StageContext`] cross
//! one seam and staged art comes back. Game rules still live in
//! `o2jam-game`; this module never stores into the game.

use std::collections::HashMap;
use std::path::Path;

use o2jam_file::{Bnd, ControlBlock, ControlList};
use o2jam_render::{Rect, NATIVE_HEIGHT, NATIVE_WIDTH};

use crate::avatar::{
    bind_scene_rects, load_archive_sprite_for_stage, load_scene_figures_for_room,
    playing_member_selected, scene_block, scene_sprite_names, AvatarFigure, PlayingArtMode,
    ScenePlacement, LOADING_SPRITE,
};
use crate::screen::Screen;
use crate::screens::side::{
    SideScreen, CHANNEL_INFO_DIALOG_BLOCK, CREATE_ROOM_DIALOG_BLOCK, PASSWORD_DIALOG_BLOCK,
    PLANET_OPTION_FIRST_TAB, QUIT_DIALOG_BLOCK, SELECT_MUSIC_DIALOG_BLOCK,
    TUTORIAL_BACKGROUND_SPRITE,
};

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
/// `.bnd`: the Loading gate, the Tutorial backdrop).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementArm {
    /// Block entries paired with the `.bnd` by code ordinal.
    Bound,
    /// One full-canvas placement per staged sprite (art without
    /// a `.bnd`: the Loading gate, the Tutorial backdrop).
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
/// because no 7.55 list names them. Avi pairs its single
/// `IDC_BACK_LOGO` entry through `State_Avi.bnd` like every other
/// bound row (the sole rect is verbatim full-bleed).
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
/// bound rows pair their Block with its `.bnd`; Loading and
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
        arm: PlacementArm::Bound,
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

/// A popup sheet mounted over a host State: one value per dialog row.
/// The key is the `DIALOG_*` Block name the archive already uses, so a
/// sheet cannot drift onto another Scene — each row names its host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialog {
    /// Song pick (`DAILOG_SELECTMUSIC`, archive typo kept) over Waiting.
    SelectMusic,
    /// Channel list (`DIALOG_CHANNEL_INFO`) over the planet board.
    ChannelInfo,
    /// First-open options tab (`DIALOG_OPTION_KEY`, which tab opens
    /// first stays OPEN) over Planet.
    PlanetOption,
    /// Quit confirm (`DIALOG_QUESTION1`) over Planet.
    PlanetQuit,
    /// Room create sheet (`DIALOG_CREATEROOM`) over Room: declared
    /// and tested only, no caller stages it yet.
    CreateRoom,
    /// Room password sheet (`DIALOG_INPUTPASSWORD`) over Room: declared
    /// and tested only, no caller stages it yet.
    InputPassword,
}

impl Dialog {
    /// `ControlList` Block this dialog stages.
    #[must_use]
    pub const fn block_name(self) -> &'static str {
        match self {
            Dialog::SelectMusic => SELECT_MUSIC_DIALOG_BLOCK,
            Dialog::ChannelInfo => CHANNEL_INFO_DIALOG_BLOCK,
            Dialog::PlanetOption => PLANET_OPTION_FIRST_TAB.block_name(),
            Dialog::PlanetQuit => QUIT_DIALOG_BLOCK,
            Dialog::CreateRoom => CREATE_ROOM_DIALOG_BLOCK,
            Dialog::InputPassword => PASSWORD_DIALOG_BLOCK,
        }
    }

    /// State this dialog mounts over.
    #[must_use]
    pub const fn host(self) -> State {
        match self {
            Dialog::SelectMusic => State::Waiting,
            Dialog::ChannelInfo | Dialog::PlanetOption | Dialog::PlanetQuit => State::Planet,
            Dialog::CreateRoom | Dialog::InputPassword => State::Room,
        }
    }

    /// Dialog for a Block name, if the dialog table stages it.
    /// State Blocks stay `None` here; they resolve through
    /// [`State::from_block`].
    #[must_use]
    pub fn from_block(name: &str) -> Option<Dialog> {
        DIALOG_TABLE
            .iter()
            .find(|row| row.block == name)
            .map(|row| row.dialog)
    }
}

/// One declaration row per dialog: the sheet to stage plus the host
/// State it mounts over. Every sheet stages from the Interface
/// archive on the bound arm; the Channel select bar has no row —
/// the exe mounts it at the Dialog object, not from the table (OPEN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogRow {
    /// Dialog this row stages.
    pub dialog: Dialog,
    /// State this dialog mounts over.
    pub host: State,
    /// `ControlList` Block to pair with its `.bnd`.
    pub block: &'static str,
}

/// The dialog declaration, read as data: one row per sheet the client
/// stages. The 2 room rows are declared and tested only until a caller
/// stages them; the select bar stays hand-staged and OPEN.
pub const DIALOG_TABLE: &[DialogRow] = &[
    DialogRow {
        dialog: Dialog::SelectMusic,
        host: State::Waiting,
        block: SELECT_MUSIC_DIALOG_BLOCK,
    },
    DialogRow {
        dialog: Dialog::ChannelInfo,
        host: State::Planet,
        block: CHANNEL_INFO_DIALOG_BLOCK,
    },
    DialogRow {
        dialog: Dialog::PlanetOption,
        host: State::Planet,
        block: PLANET_OPTION_FIRST_TAB.block_name(),
    },
    DialogRow {
        dialog: Dialog::PlanetQuit,
        host: State::Planet,
        block: QUIT_DIALOG_BLOCK,
    },
    DialogRow {
        dialog: Dialog::CreateRoom,
        host: State::Room,
        block: CREATE_ROOM_DIALOG_BLOCK,
    },
    DialogRow {
        dialog: Dialog::InputPassword,
        host: State::Room,
        block: PASSWORD_DIALOG_BLOCK,
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

/// Stage one dialog from archive bytes through the bound arm: the
/// Block paired with its `.bnd`, like [`stage_state`] dispatches for
/// [`PlacementArm::Bound`]. Returns the art with the miss ledger;
/// each miss logs a warning naming the host State. The Channel select
/// bar is not in the table, so callers hand-stage it on top of this
/// art. Never fails a Scene.
#[must_use]
pub fn stage_dialog(archive: &[u8], dialog: Dialog, ctx: StageContext) -> StageReport {
    let Some(row) = DIALOG_TABLE.iter().find(|row| row.dialog == dialog) else {
        return StageReport::default();
    };
    stage_bound_row(
        archive,
        &StageRow {
            state: row.host,
            block: Some(row.block),
            archive: StagingArchive::Interface,
            arm: PlacementArm::Bound,
            extra_sprites: &[],
        },
        ctx,
    )
}

/// One bound row resolved to stager inputs: the Block name for
/// the miss ledger, the sprite names in file order, the Block for
/// the `.bnd` pairing, and the parsed bind layout. The Block is an
/// owned clone because the list it parses from is local to the
/// lookup (OWNERSHIP-10: never borrow across the return).
struct ResolvedBoundRow {
    block_name: &'static str,
    names: Vec<String>,
    block: ControlBlock,
    bind: Bnd,
}

/// Resolve one bound row through its archive: the list file, the
/// Block, its names, and its `.bnd`. Both the bound arm and the
/// Playing subset stage through this seam. A missing list, Block,
/// or bind file records its miss and resolves nothing; a block-less
/// row resolves nothing with no miss.
fn resolve_bound_row(
    archive: &[u8],
    row: &StageRow,
    report: &mut StageReport,
) -> Option<ResolvedBoundRow> {
    let block_name = row.block?;
    let list_file = row.archive.list_file();
    let Some(list) = read_control_list(archive, list_file) else {
        push_miss(report, row.state, block_name, list_file);
        return None;
    };
    let Some(block) = scene_block(&list, block_name) else {
        push_miss(report, row.state, block_name, block_name);
        return None;
    };
    let names = scene_sprite_names(block);
    let Some(bind_name) = block.bound_file.clone() else {
        push_miss(report, row.state, block_name, "(no BOUND line)");
        return None;
    };
    let Some(bind) = read_bind(archive, &bind_name) else {
        push_miss(report, row.state, block_name, &bind_name);
        return None;
    };
    Some(ResolvedBoundRow {
        block_name,
        names,
        block: block.clone(),
        bind,
    })
}

/// Stage one bound row: the Block paired with its `.bnd`, plus the
/// row's extra bodies merged by name.
fn stage_bound_row(archive: &[u8], row: &StageRow, ctx: StageContext) -> StageReport {
    let mut report = StageReport::default();
    let Some(resolved) = resolve_bound_row(archive, row, &mut report) else {
        return report;
    };
    let mut art = StagedArt {
        placements: bind_scene_rects(&resolved.block, &resolved.bind),
        figures: HashMap::new(),
    };
    load_figures(
        archive,
        row.state,
        resolved.block_name,
        ctx,
        &staged_names(&resolved.names, row.extra_sprites),
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
    load_figures(
        archive,
        row.state,
        row.block.unwrap_or(row.state.block_name()),
        ctx,
        &names,
        &mut art,
        &mut report,
    );
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
    let Some(resolved) = resolve_bound_row(archive, row, &mut report) else {
        return report;
    };
    let block_name = resolved.block_name;
    let mut art = StagedArt {
        placements: bind_scene_rects(&resolved.block, &resolved.bind),
        figures: load_playing_figures(
            archive,
            &resolved.names,
            ctx,
            &mut report,
            row.state,
            block_name,
        ),
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
/// placement survives. The caller passes the already-resolved Block
/// name for the miss ledger; this loader never re-derives it.
fn load_figures(
    archive: &[u8],
    state: State,
    block_name: &'static str,
    ctx: StageContext,
    names: &[String],
    art: &mut StagedArt,
    report: &mut StageReport,
) {
    for name in names {
        if is_scrollbar_params(name) {
            continue;
        }
        match load_archive_sprite_for_stage(archive, name, ctx.arena_stage) {
            Some(figure) => {
                art.figures.insert(name.clone(), figure);
            }
            None => push_miss(report, state, block_name, name),
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
#[path = "staging/tests.rs"]
mod staging_tests;
