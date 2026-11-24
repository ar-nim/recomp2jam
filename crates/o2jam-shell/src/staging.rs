//! Scene staging declaration: one row per State.
//!
//! The client shell stages every bound State through this table —
//! the archive bytes, the [`State`], and the [`StageContext`] cross
//! one seam and staged art comes back. Game rules still live in
//! `o2jam-game`; this module never stores into the game.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use o2jam_file::{Bnd, ControlBlock, ControlList};
use o2jam_render::{rect_from_corners, Rect, NATIVE_HEIGHT, NATIVE_WIDTH};

use crate::avatar::{
    bind_scene_rects, load_archive_sprite_for_stage, load_scene_figures_for_room,
    load_staged_figure, playing_backdrop_sprite, playing_member_selected, scene_block,
    scene_sprite_names, AvatarFigure, PlacementRole, PlayingArtMode, ScenePlacement,
    LOADING_SPRITE, PLAYING_BACKDROP_CODE,
};
use crate::screen::Screen;
use crate::screens::messenger::{
    MESSENGER_FRIENDADD_BLOCK, MESSENGER_INFORMATION_BLOCK, MESSENGER_MAIN_BLOCK,
    MESSENGER_NOTE_BLOCK, MESSENGER_POPUPMENU_BLOCK, MESSENGER_QUESTION_BLOCK,
};
use crate::screens::paint::{CashChargeArt, SceneArt, ROOM_HOVER_SPRITE};
use crate::screens::side::{
    shop_child_geometry, PlanetOptionTab, ShopChildGeometry, SideScreen, CASH_CHARGE_DIALOG_BLOCK,
    CHANNEL_INFO_DIALOG_BLOCK, CREATE_ROOM_DIALOG_BLOCK, GIFT_PAGE, INFORMATION_DIALOG_BLOCK,
    PASSWORD_DIALOG_BLOCK, PAYMENT2_BIND_FILE, PAYMENT2_BLOCK, PAYMENT_BIND_FILE, PAYMENT_BLOCK,
    PAYMENT_PAGE, QUIT_DIALOG_BLOCK, SELECT_MUSIC_DIALOG_BLOCK, SHOPPINGBAG_ADD_DIALOG_BLOCK,
    SHOP_CHILD_ORIGIN_Y, SHOP_EXTMENU_SIDE_CODES, SHOP_SET_CARD_CHILD_CODES,
    TUTORIAL_BACKGROUND_SPRITE, TUTORIAL_COACH_SPRITE, TUTORIAL_INSTRUCTION_SPRITE,
    TUTORIAL_RECEPTOR_SPRITES,
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
        extra_sprites: &[SHOP_MASTER_MASCOT_SPRITE],
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
        extra_sprites: &["Result_WinLose.ojs", "Result_NumFont_Blue.ojs"],
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
        // B7 exe frame `612830b5…e77b8`: backdrop, frame-0 lesson
        // bubble (with `1/13`) and frame-0 coach are all in this scene.
        // The lane kit arm `0x4dd290` adds seven receptor bodies at row
        // 487. Their record anchors stage them; the painter selects one
        // frame per body (key light) instead of stacking them.
        extra_sprites: &[
            TUTORIAL_BACKGROUND_SPRITE,
            TUTORIAL_INSTRUCTION_SPRITE,
            TUTORIAL_COACH_SPRITE,
            TUTORIAL_RECEPTOR_SPRITES[0],
            TUTORIAL_RECEPTOR_SPRITES[1],
            TUTORIAL_RECEPTOR_SPRITES[2],
            TUTORIAL_RECEPTOR_SPRITES[3],
            TUTORIAL_RECEPTOR_SPRITES[4],
            TUTORIAL_RECEPTOR_SPRITES[5],
            TUTORIAL_RECEPTOR_SPRITES[6],
        ],
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
    /// Key options tab (`DIALOG_OPTION_KEY`) over Planet. Its block
    /// is the dialog's own identity, not `PLANET_OPTION_FIRST_TAB`:
    /// the first-open sheet (#359) is a separate paint choice and
    /// must not duplicate this row when it flips to Sound.
    PlanetOption,
    /// Sound options tab (`DIALOG_OPTION_SOUND`) over Planet:
    /// staged so the tab switches flip between genuine sheets.
    PlanetOptionSound,
    /// Game options tab (`DIALOG_OPTION_GAME`) over Planet: staged
    /// so the tab switches flip between genuine sheets.
    PlanetOptionGame,
    /// Quit confirm (`DIALOG_QUESTION1`) over Planet.
    PlanetQuit,
    /// Relay-unavailable notice (`DIALOG_INFORMATION`, one OK) over
    /// Planet, before the Channel dialog loads (#244).
    Information,
    /// Room create sheet (`DIALOG_CREATEROOM`) over Room: declared
    /// and tested only, no caller stages it yet.
    CreateRoom,
    /// Room password sheet (`DIALOG_INPUTPASSWORD`) over Room: declared
    /// and tested only, no caller stages it yet.
    InputPassword,
    /// Cash Charge web window (`DIALOG_CHARGECASH`) over the Item Shop
    /// host board (b8 capture, issue #362).
    ChargeCash,
}

impl Dialog {
    /// `ControlList` Block this dialog stages.
    #[must_use]
    pub const fn block_name(self) -> &'static str {
        match self {
            Dialog::SelectMusic => SELECT_MUSIC_DIALOG_BLOCK,
            Dialog::ChannelInfo => CHANNEL_INFO_DIALOG_BLOCK,
            Dialog::PlanetOption => PlanetOptionTab::Key.block_name(),
            Dialog::PlanetOptionSound => PlanetOptionTab::Sound.block_name(),
            Dialog::PlanetOptionGame => PlanetOptionTab::Game.block_name(),
            Dialog::PlanetQuit => QUIT_DIALOG_BLOCK,
            Dialog::Information => INFORMATION_DIALOG_BLOCK,
            Dialog::CreateRoom => CREATE_ROOM_DIALOG_BLOCK,
            Dialog::InputPassword => PASSWORD_DIALOG_BLOCK,
            Dialog::ChargeCash => CASH_CHARGE_DIALOG_BLOCK,
        }
    }

    /// Bodies the sheet declares but never paints: the song dialog's
    /// `?` buttons are hit bands over plates already baked into the
    /// backdrop body (#272). The two mode panels and the three help
    /// sheets stage as figures and paint through
    /// [`SELECT_MUSIC_PAINT_GATED_SPRITES`] (#285).
    #[must_use]
    pub const fn hidden_sprites(self) -> &'static [&'static str] {
        match self {
            Dialog::SelectMusic => SELECT_MUSIC_BACKDROP_BAKED_SPRITES,
            Dialog::ChannelInfo
            | Dialog::PlanetOption
            | Dialog::PlanetOptionSound
            | Dialog::PlanetOptionGame
            | Dialog::PlanetQuit
            | Dialog::Information
            | Dialog::CreateRoom
            | Dialog::InputPassword
            | Dialog::ChargeCash => &[],
        }
    }

    /// State this dialog mounts over.
    #[must_use]
    pub const fn host(self) -> State {
        match self {
            Dialog::SelectMusic => State::Waiting,
            Dialog::ChannelInfo
            | Dialog::PlanetOption
            | Dialog::PlanetOptionSound
            | Dialog::PlanetOptionGame
            | Dialog::PlanetQuit
            | Dialog::Information => State::Planet,
            Dialog::CreateRoom | Dialog::InputPassword => State::Room,
            Dialog::ChargeCash => State::Payment,
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
        block: PlanetOptionTab::Key.block_name(),
    },
    DialogRow {
        dialog: Dialog::PlanetOptionSound,
        host: State::Planet,
        block: PlanetOptionTab::Sound.block_name(),
    },
    DialogRow {
        dialog: Dialog::PlanetOptionGame,
        host: State::Planet,
        block: PlanetOptionTab::Game.block_name(),
    },
    DialogRow {
        dialog: Dialog::PlanetQuit,
        host: State::Planet,
        block: QUIT_DIALOG_BLOCK,
    },
    DialogRow {
        dialog: Dialog::Information,
        host: State::Planet,
        block: INFORMATION_DIALOG_BLOCK,
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
    DialogRow {
        dialog: Dialog::ChargeCash,
        host: State::Payment,
        block: CASH_CHARGE_DIALOG_BLOCK,
    },
];

/// One of the six Messenger sheets staged from its `BOUND` block.
/// The 7.55 ControlList declares 77/6/7/13/6/5 rects for these sheets;
/// spec #315 story 12 stages them at those parsed rects. These are
/// geometry declarations only: the qualified B8/B6 packs never mount
/// a Messenger sheet, so no sheet paints or owns input until an exe
/// mount pin exists (`N-MOUNT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessengerSheet {
    /// Main sheet: 77 rects, friend/note and four chat tabs.
    Main,
    /// Friend-add sheet: 6 rects.
    FriendAdd,
    /// Question sheet: 7 rects.
    Question,
    /// Note sheet: 13 rects.
    Note,
    /// PopUpMenu sheet: 6 rects.
    PopUpMenu,
    /// Information sheet: 5 rects.
    Information,
}

impl MessengerSheet {
    /// `ControlList_Interface.txt` block this sheet stages.
    #[must_use]
    pub const fn block_name(self) -> &'static str {
        match self {
            Self::Main => MESSENGER_MAIN_BLOCK,
            Self::FriendAdd => MESSENGER_FRIENDADD_BLOCK,
            Self::Question => MESSENGER_QUESTION_BLOCK,
            Self::Note => MESSENGER_NOTE_BLOCK,
            Self::PopUpMenu => MESSENGER_POPUPMENU_BLOCK,
            Self::Information => MESSENGER_INFORMATION_BLOCK,
        }
    }

    /// Sheet for a block name, if the family stages it.
    #[must_use]
    pub fn from_block(name: &str) -> Option<Self> {
        MESSENGER_SHEETS
            .iter()
            .copied()
            .find(|sheet| sheet.block_name() == name)
    }
}

/// All six sheets in ControlList order. The live boot stages each
/// into its own game slot; nothing here claims a live mount.
pub const MESSENGER_SHEETS: [MessengerSheet; 6] = [
    MessengerSheet::Main,
    MessengerSheet::FriendAdd,
    MessengerSheet::Question,
    MessengerSheet::Note,
    MessengerSheet::PopUpMenu,
    MessengerSheet::Information,
];

/// One declaration row per exe-literal bind load: a `.bnd` file the
/// exe stages by name (files `0x24aba7`/`0x24b080`,
/// `docs/bnd-semantics.md`), with no `ControlList` block to pair it —
/// `ControlList_Playing.txt` declares only `STATE_PLAYING`, so the
/// bound-arm resolver has nothing to mirror for these sheets. The key
/// is the literal file name (the exe's own vocabulary, like the
/// `STATE_*` block names keying [`STAGE_TABLE`]); `host` names the
/// neighbor scene whose slot consumes the art later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByNameRow {
    /// Literal `.bnd` file in the archive.
    pub bind_file: &'static str,
    /// Archive holding the file.
    pub archive: StagingArchive,
    /// Neighbor scene the art stages beside.
    pub host: State,
}

/// The by-name declaration, read as data: one row per literal bind
/// load the exe mounts outside every `ControlList` block (ticket
/// #166). The rows share the one staging seam — [`stage_byname_sheet`]
/// parses each file through the existing `parse_bnd` path and stages
/// the file's own rects — never a second system, never literals.
/// Paint wiring rides a later ticket: the game parks these placements
/// beside the Playing scene.
///
/// Exe evidence: `Avatar_Info.bnd`/`Map_Avatar.bnd` at file
/// `0x24aba7`, `State_Playing_network.bnd` at file `0x24b080`
/// (`docs/bnd-semantics.md`); the single-block Playing list behind
/// the missing `BOUND` rows is pinned by #165.
pub const BYNAME_TABLE: &[ByNameRow] = &[
    ByNameRow {
        bind_file: "State_Playing_network.bnd",
        archive: StagingArchive::Playing,
        host: State::Playing,
    },
    ByNameRow {
        bind_file: "Avatar_Info.bnd",
        archive: StagingArchive::Playing,
        host: State::Playing,
    },
    ByNameRow {
        bind_file: "Map_Avatar.bnd",
        archive: StagingArchive::Playing,
        host: State::Playing,
    },
];

/// Bodies the song-select sheet declares but never stages as figures.
/// Empty since #764: the `LABELQUESTION 0x27200053` and
/// `SEARCHQUESTION 0x27200055` bodies stage again and paint only while
/// their guide sheet is active (#272 keeps their plates visible at open).
/// Both authored bodies have two byte-identical frames, so no distinct
/// hover or help-active face is claimed; the hover face stays OPEN.
pub const SELECT_MUSIC_BACKDROP_BAKED_SPRITES: &[&str] = &[];

/// Result-tab control faces the sheet backdrop already paints (#349).
/// `Dialog_SelectMusic.ojs` bakes the header labels, the check-all /
/// remove-all boxes and the row check boxes; the sort bodies
/// (`SelectMusic_Btn_{Diff,Label,Title,Artist,Down,Rank}.ojs`),
/// `SelectMusic_RemoveAll.ojs`, `SelectMusic_RemoveBox.ojs` and
/// `MShop_MMY_Color.ojs` repeat those
/// faces (and add bright underlines / tags) when painted over it. The
/// result tab shows the baked faces: the pinned exe capture has one
/// orange underline under the level sort and no row remove/tag marks.
/// Their `.bnd` rects still stage for clicks.
pub const SELECT_MUSIC_RESULT_TAB_BAKED_SPRITES: &[&str] = &[
    "SelectMusic_Btn_Diff.ojs",
    "SelectMusic_Btn_Label.ojs",
    "SelectMusic_Btn_Title.ojs",
    "SelectMusic_Btn_Artist.ojs",
    "SelectMusic_Btn_Down.ojs",
    "SelectMusic_Btn_Rank.ojs",
    "SelectMusic_RemoveAll.ojs",
    "SelectMusic_RemoveBox.ojs",
    "MShop_MMY_Color.ojs",
];

/// Row bodies whose frame follows the chart, not a filter state (#349).
/// `SelectMusic_Diff.ojs` stores E/N/H in frames 0..2; one frame per
/// row replaces the earlier single state painted on every row.
pub const SELECT_ROW_DIFFICULTY_SPRITE: &str = "SelectMusic_Diff.ojs";

/// Bodies the song-select sheet stages but does not paint at open
/// (ticket #272 staged them hidden; #285 makes them paint-gated).
/// Two families sit against the visible song-select panel:
///
/// - the random panel (`SelectMusic_Random1_BG.ojs` and its five
///   `SelectMusic_Btn_Random1_*` buttons) pairs the *same* `.bnd` rects
///   as the level column's `STEP1_*` buttons — the block's two
///   selection panels, one visible at a time, switched by the top bar's
///   `IDC_BUTTON_DSMUSIC_MUISCSELECT 0x2720001A` /
///   `IDC_BUTTON_DSMUSIC_RANDOMSELECT 0x2720001B`;
/// - the three help overlays behind the `?` buttons
///   (`SelectMusic_*GuidImg.ojs`, ControlList comment `'?' 이미지,
///   가이드 이미지`), each a screen-size guide sheet.
///
/// The pinned reference
/// `docs/screenshots/reference-musicselect-4x3.png` shows the dialog
/// open in song-select mode with no guide sheet, so both families start
/// hidden and only their rects stage. The switch handlers stay OPEN
/// (next evidence: the exe handlers behind the panel-switch codes
/// `0x2720001A`/`0x2720001B`, and behind
/// `IDC_BUTTON_DSMUSIC_LABELQUESTION 0x27200053` /
/// `IDC_BUTTON_DSMUSIC_SEARCHQUESTION 0x27200055` for the guides
/// (VA `0x441a00`, helper VA `0x44f800`).
pub const SELECT_MUSIC_PAINT_GATED_SPRITES: &[&str] = &[
    "SelectMusic_Random1_BG.ojs",
    "SelectMusic_Btn_Random1_all.ojs",
    "SelectMusic_Btn_Random1_Easy.ojs",
    "SelectMusic_Btn_Random1_Intermediate.ojs",
    "SelectMusic_Btn_Random1_High.ojs",
    "SelectMusic_Btn_Random1_Highest.ojs",
    "SelectMusic_LabelGuidImg.ojs",
    "SelectMusic_SearchGuidImg.ojs",
    "SelectMusic_ListGuidImg.ojs",
];

/// Random-panel bodies: painted only while the random-mode switch
/// (`0x2720001B`) is live (VA `0x44f9a7`).
pub const SELECT_MUSIC_RANDOM_SPRITES: &[&str] = &[
    "SelectMusic_Random1_BG.ojs",
    "SelectMusic_Btn_Random1_all.ojs",
    "SelectMusic_Btn_Random1_Easy.ojs",
    "SelectMusic_Btn_Random1_Intermediate.ojs",
    "SelectMusic_Btn_Random1_High.ojs",
    "SelectMusic_Btn_Random1_Highest.ojs",
];

/// Level-column bodies: the panel the `MUISCSELECT` arm restores
/// (VA `0x44fdc0`), hidden while the random panel is live.
pub const SELECT_MUSIC_LEVEL_SPRITES: &[&str] = &[
    "SelectMusic_Btn_Step1_all.ojs",
    "SelectMusic_Btn_Step1_Easy.ojs",
    "SelectMusic_Btn_Step1_Intermediate.ojs",
    "SelectMusic_Btn_Step1_High.ojs",
    "SelectMusic_Btn_Step1_Highest.ojs",
];

/// Help-sheet bodies: one at a time, none at open (VA `0x441a00`).
pub const SELECT_MUSIC_LABEL_GUIDE: &str = "SelectMusic_LabelGuidImg.ojs";
pub const SELECT_MUSIC_SEARCH_GUIDE: &str = "SelectMusic_SearchGuidImg.ojs";
pub const SELECT_MUSIC_LIST_GUIDE: &str = "SelectMusic_ListGuidImg.ojs";

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
    /// Chart slot index (`0` Easy, `1` Normal, `2` Hard): feeds the
    /// Playing BACK1 backdrop's `%d` variant (exe staging function
    /// file `0x4dc79f`–`0x4dc7bb`; the arg semantic stays OPEN, see
    /// `docs/playing-backdrop.md`).
    pub chart_slot: u8,
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

/// Low twelve bits of a control code: the exe's `.bnd` record ordinal.
const PAYMENT_RECT_ORDINAL_MASK: u32 = 0xFFF;
/// Top byte of a control code: the exe's `STATE_*` family
/// (`0x1D` for `STATE_PAYMENT`, `0x1E` for `STATE_PAYMENT2`).
const PAYMENT_CODE_FAMILY_MASK: u32 = 0xFF00_0000;

/// One billing page's client-side chrome map. The page string is a
/// lifecycle label from `docs/web-surfaces.md`, not a live web mount;
/// all geometry comes from the declared Block and its `.bnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaymentPage {
    /// Exe State that declares this sheet.
    pub state: State,
    /// `ControlList` block staging the sheet.
    pub block: &'static str,
    /// Bind file holding the sheet's rects.
    pub bind_file: &'static str,
    /// Lifecycle page label (the web content stays native/blank).
    pub page: &'static str,
    /// Top byte of every control code declared in the block.
    pub code_prefix: u32,
}

impl PaymentPage {
    /// True for a staged control declared in this page's block.
    #[must_use]
    pub const fn owns_code(self, code: u32) -> bool {
        code & PAYMENT_CODE_FAMILY_MASK == self.code_prefix
    }
}

/// Billing pages in exe State order. The codes are the shipped
/// block-self controls `IDC_STATE_PAYMENT` (`0x1D000000`) and
/// `IDC_STATE_PAYMENT2` (`0x1E000000`); no geometry is invented.
pub const PAYMENT_PAGES: [PaymentPage; 2] = [
    PaymentPage {
        state: State::Payment,
        block: PAYMENT_BLOCK,
        bind_file: PAYMENT_BIND_FILE,
        page: PAYMENT_PAGE,
        code_prefix: 0x1D00_0000,
    },
    PaymentPage {
        state: State::Payment2,
        block: PAYMENT2_BLOCK,
        bind_file: PAYMENT2_BIND_FILE,
        page: GIFT_PAGE,
        code_prefix: 0x1E00_0000,
    },
];

/// Each billing page staged from its own declaration, keyed by
/// [`PAYMENT_PAGES`] order. Pages stay separate here; the painter
/// still receives one merged sheet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaymentPages {
    art: [Option<StagedArt>; PAYMENT_PAGES.len()],
}

impl PaymentPages {
    /// One page's staged art, if its declaration resolved.
    #[must_use]
    pub fn art_for(&self, page: PaymentPage) -> Option<&StagedArt> {
        let index = PAYMENT_PAGES
            .iter()
            .position(|candidate| *candidate == page)?;
        self.art.get(index).and_then(Option::as_ref)
    }

    /// The merged sheet today's Payment painter consumes, in page
    /// order. Missing pages contribute nothing (ADR-0003).
    #[must_use]
    pub fn into_merged(self) -> StagedArt {
        let mut merged = StagedArt::default();
        for art in self.art {
            let Some(art) = art else {
                continue;
            };
            merged.placements.extend(art.placements);
            merged.figures.extend(art.figures);
        }
        merged
    }
}

/// Stage each billing page through the same State-keyed declaration
/// the live boot uses. A missing Block or bind leaves that page empty.
#[must_use]
pub fn stage_payment_pages(archive: &[u8], ctx: StageContext) -> PaymentPages {
    let mut art: [Option<StagedArt>; PAYMENT_PAGES.len()] = std::array::from_fn(|_| None);
    for (index, page) in PAYMENT_PAGES.iter().enumerate() {
        art[index] = stage_state(archive, page.state, ctx).art;
    }
    PaymentPages { art }
}

/// Staged chrome placements owned by one billing page, in `.bnd`
/// ordinal order (the exe pairs by code, never by file position).
/// Empty input yields an empty map — no fallback rects.
#[must_use]
pub fn payment_page_placements(
    placements: &[ScenePlacement],
    page: PaymentPage,
) -> Vec<&ScenePlacement> {
    let mut mapped: Vec<_> = placements
        .iter()
        .filter(|placed| page.owns_code(placed.code))
        .collect();
    mapped.sort_by_key(|placed| placed.code & PAYMENT_RECT_ORDINAL_MASK);
    mapped
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
    stage_bound_row_hiding(
        archive,
        &StageRow {
            state: row.host,
            block: Some(row.block),
            archive: StagingArchive::Interface,
            arm: PlacementArm::Bound,
            extra_sprites: &[],
        },
        ctx,
        dialog.hidden_sprites(),
    )
}

/// Code-mount body the 7.55 shop entry paints in the bottom-right
/// pane: `IDC_IMAGE_SSHOP_SHOPMASTER` (`0x14600053`) hosts
/// `Shop_Master_main.ojs`, whose record anchor `(654,446)` places the
/// mascot 1:1 on the exe frame (`docs/shop.md`, #877). `ControlList`
/// leaves that control art-less, so the bound arm merges the body by
/// name and attaches it to the parsed rect.
const SHOP_MASTER_MASCOT_SPRITE: &str = "Shop_Master_main.ojs";
pub const SHOP_MASTER_MASCOT_CODE: u32 = 0x1460_0053;

/// Attach the code-mounted shop-mascot body to its bound image
/// control. The `.bnd` rect stays the geometry source; the sprite's
/// own anchor decides where its pixels land (owner `f38f3`).
fn stage_shop_master_mascot(art: &mut StagedArt) {
    for placed in &mut art.placements {
        if placed.code == SHOP_MASTER_MASCOT_CODE {
            placed.sprite = Some(SHOP_MASTER_MASCOT_SPRITE.to_owned());
        }
    }
}

/// Put the exe's child origin on the shop controls that have no
/// sprite anchor to carry it: art-less TEXT ink and the art-less
/// gift half. Anchored ITEMBUY/ITEMPRE placements keep the raw rect
/// (their decoded anchor paints at the exe position) and take the
/// same [`SHOP_CHILD_ORIGIN_Y`] at the hit-test seam. One origin,
/// two consumers, no literal coordinates in either (owner `f38f3`).
fn stage_shop_child_origin(art: &mut StagedArt) {
    for placed in &mut art.placements {
        let moves_rect = matches!(
            shop_child_geometry(placed.code),
            Some(ShopChildGeometry::TextInk | ShopChildGeometry::ArtlessButton)
        );
        if moves_rect {
            placed.rect.y = placed.rect.y.saturating_add(SHOP_CHILD_ORIGIN_Y);
        }
    }
}

/// Clone the set-item card's art-less `SET` children under each
/// `SETITEM_EX` parent. The generic `SET` expansion clones only
/// sprite-bearing members; the exe's `FUN_00498bc0` refresher
/// (`ev_5bee92ef…be`) writes title/type/price cells for every card, so
/// the art-less children need the same parent-origin write-back. The
/// parent rects and child templates both come from `.bnd` pairing;
/// no literal card coordinate is introduced.
fn stage_shop_set_card_children(art: &mut StagedArt) {
    let parents: Vec<(String, Rect)> = art
        .placements
        .iter()
        .filter(|placed| SHOP_EXTMENU_SIDE_CODES.contains(&placed.code))
        .map(|placed| (placed.id.clone(), placed.rect))
        .collect();
    let templates: Vec<ScenePlacement> = art
        .placements
        .iter()
        .filter(|placed| {
            SHOP_SET_CARD_CHILD_CODES.contains(&placed.code) && placed.role != PlacementRole::Clone
        })
        .cloned()
        .collect();
    if parents.is_empty() || templates.is_empty() {
        return;
    }
    art.placements.retain(|placed| {
        !(SHOP_SET_CARD_CHILD_CODES.contains(&placed.code) && placed.role != PlacementRole::Clone)
    });
    for (parent_id, parent_rect) in &parents {
        for template in &templates {
            art.placements.push(ScenePlacement {
                role: PlacementRole::Clone,
                id: parent_id.clone(),
                code: template.code,
                sprite: template.sprite.clone(),
                rect: at_parent_origin(template.rect, *parent_rect),
            });
        }
    }
}

/// Native ordinals of the three `Dialog_ChargeCash.bnd` rects, paired
/// by `(code & 0xFFF) - 1` (exe pairing, docs/file-formats/opi.md).
const CASH_CHARGE_FRAME_ORDINAL: u32 = 1;
const CASH_CHARGE_WEB_ORDINAL: u32 = 2;
const CASH_CHARGE_CLOSE_ORDINAL: u32 = 3;

/// One placement's `.bnd` record ordinal.
fn control_ordinal(placed: &ScenePlacement) -> u32 {
    placed.code & 0xFFF
}

/// Find one staged Cash Charge control by its `.bnd` ordinal.
fn cash_charge_control(placements: &[ScenePlacement], ordinal: u32) -> Option<&ScenePlacement> {
    placements
        .iter()
        .find(|placed| control_ordinal(placed) == ordinal)
}

/// Offset a parent-relative `.bnd` rect by the staged frame origin.
fn at_parent_origin(rect: Rect, frame: Rect) -> Rect {
    Rect {
        x: rect.x + frame.x,
        y: rect.y + frame.y,
        ..rect
    }
}

/// Stage the Cash Charge view: the Item Shop host board plus the
/// dialog sheet. The dialog's child rects and sprite anchors are
/// parent-relative in the 7.55 sheet, so the web client rect shifts
/// by the parsed frame origin and the close body paints from that
/// origin plus its own parsed anchor (no literal coordinates).
#[must_use]
pub fn stage_cash_charge(archive: &[u8], ctx: StageContext) -> CashChargeArt {
    let host = stage_state(archive, State::Shop, ctx)
        .art
        .unwrap_or_default();
    let dialog = stage_dialog(archive, Dialog::ChargeCash, ctx)
        .art
        .unwrap_or_default();
    CashChargeArt {
        host: SceneArt {
            placements: host.placements,
            figures: host.figures,
        },
        dialog: stage_cash_charge_dialog(dialog),
    }
}

/// Put the parsed dialog placements in paint order and resolve the
/// parent offset. A missing frame stages nothing (ADR-0003).
fn stage_cash_charge_dialog(bind: StagedArt) -> SceneArt {
    let Some(frame) = cash_charge_control(&bind.placements, CASH_CHARGE_FRAME_ORDINAL) else {
        return SceneArt::default();
    };
    let origin = frame.rect;
    let mut placements = vec![frame.clone()];
    if let Some(web) = cash_charge_control(&bind.placements, CASH_CHARGE_WEB_ORDINAL) {
        placements.push(ScenePlacement {
            rect: at_parent_origin(web.rect, origin),
            ..web.clone()
        });
    }
    if let Some(close) = cash_charge_control(&bind.placements, CASH_CHARGE_CLOSE_ORDINAL) {
        // A clone paints at its rect plus the sprite anchor; the
        // close body's anchor is already dialog-relative.
        placements.push(ScenePlacement {
            role: PlacementRole::Clone,
            rect: origin,
            ..close.clone()
        });
    }
    SceneArt {
        placements,
        figures: bind.figures,
    }
}

/// Stage one by-name sheet from archive bytes through its row: parse
/// the literal `.bnd` through the existing `parse_bnd` path and stage
/// one placement per rect in file order, verbatim through
/// `rect_from_corners` (the same conversion the bound arm uses). No
/// block pairs the rects and none names a body, so the placements
/// carry no id, code, or sprite and no figure loads (ADR-0003: parsed
/// data, never literals; paint wiring rides a later ticket). A
/// missing bind file records its miss against the host row and stages
/// nothing.
#[must_use]
pub fn stage_byname_sheet(archive: &[u8], row: &ByNameRow) -> StageReport {
    let mut report = StageReport::default();
    let Some(bind) = read_bind(archive, row.bind_file) else {
        push_miss(&mut report, row.host, row.host.block_name(), row.bind_file);
        return report;
    };
    let art = StagedArt {
        placements: byname_placements(&bind),
        figures: HashMap::new(),
    };
    if art.placements.is_empty() {
        return report;
    }
    report.art = Some(art);
    report
}

/// One placement per literal-bind rect in file order: the file's own
/// contents, with no entry to pair — no id, code, or sprite (mirrors
/// `full_canvas_placements`, but with the parsed rects, never the
/// canvas fill).
fn byname_placements(bind: &Bnd) -> Vec<ScenePlacement> {
    bind.rects
        .iter()
        .map(|corners| ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code: 0,
            sprite: None,
            rect: rect_from_corners(corners.x1, corners.y1, corners.x2, corners.y2),
        })
        .collect()
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

/// Stage one parked dialog sheet through the same bound-row path every
/// dialog uses. No dialog row is needed: the sheet's own `BOUND` block
/// and `.bnd` are the geometry source (ADR-0003). The frame keeps its
/// parsed origin; children authored relative to it stage at
/// `frame + child`. Paint wiring stays off until an exe mount pin exists.
fn stage_parked_sheet(
    archive: &[u8],
    state: State,
    block: &'static str,
    ctx: StageContext,
) -> StageReport {
    let mut report = stage_bound_row(
        archive,
        &StageRow {
            state,
            block: Some(block),
            archive: StagingArchive::Interface,
            arm: PlacementArm::Bound,
            extra_sprites: &[],
        },
        ctx,
    );
    if let Some(art) = report.art.as_mut() {
        stage_at_parent_origin(art);
    }
    report
}

/// Stage one Messenger sheet's rects (spec #315 story 13). The B8/B6
/// packs never mount these sheets, so the game parks the geometry.
#[must_use]
pub fn stage_messenger_sheet(
    archive: &[u8],
    sheet: MessengerSheet,
    ctx: StageContext,
) -> StageReport {
    stage_parked_sheet(archive, State::Planet, sheet.block_name(), ctx)
}

/// Stage the `DIALOG_SHOPPINGBAG_ADD` rects (spec #316 story 11). The
/// audit D4 row pick is pinned only to the SHOPPING BAG panel face, so
/// the sheet parks beside the live shop: seven parent-relative rects,
/// no mount and no add opcode (`SHOPPINGBAG_ADD_NEXT_STEP`).
#[must_use]
pub fn stage_shopping_bag_add_sheet(archive: &[u8], ctx: StageContext) -> StageReport {
    stage_parked_sheet(archive, State::Shop, SHOPPINGBAG_ADD_DIALOG_BLOCK, ctx)
}

/// True for the sheet's frame placement: the bound `Dialog_*` body
/// whose rect is the parent origin (the same rule `dialog_origin` uses
/// when painting a staged dialog sheet).
fn is_sheet_frame(placed: &ScenePlacement) -> bool {
    placed
        .sprite
        .as_deref()
        .is_some_and(|sprite| sprite.starts_with("Dialog_"))
}

/// Move one sheet's children from parent-local `.bnd` rects to canvas
/// rects, leaving the frame at its parsed origin. The origin comes
/// only from the staged frame placement — no literal sheet position.
fn stage_at_parent_origin(art: &mut StagedArt) {
    let Some(frame) = art
        .placements
        .iter()
        .find(|placed| is_sheet_frame(placed))
        .map(|placed| placed.rect)
    else {
        return;
    };
    for placed in &mut art.placements {
        if !is_sheet_frame(placed) {
            placed.rect = at_parent_origin(placed.rect, frame);
        }
    }
}

/// Stage one bound row: the Block paired with its `.bnd`, plus the
/// row's extra bodies merged by name.
fn stage_bound_row(archive: &[u8], row: &StageRow, ctx: StageContext) -> StageReport {
    stage_bound_row_hiding(archive, row, ctx, &[])
}

/// [`stage_bound_row`] with bodies the sheet names but opens hidden:
/// their placements keep their rects (clicks and geometry stay
/// file-derived) while no figure loads, so the slot paints nothing —
/// the same empty-slot arm the Playing mode families use.
fn stage_bound_row_hiding(
    archive: &[u8],
    row: &StageRow,
    ctx: StageContext,
    hidden: &[&str],
) -> StageReport {
    let mut report = StageReport::default();
    let Some(resolved) = resolve_bound_row(archive, row, &mut report) else {
        return report;
    };
    let mut art = StagedArt {
        placements: bind_scene_rects(&resolved.block, &resolved.bind),
        figures: HashMap::new(),
    };
    if row.state == State::Shop {
        stage_shop_child_origin(&mut art);
        stage_shop_set_card_children(&mut art);
        stage_shop_master_mascot(&mut art);
    }
    let visible_names: Vec<String> = staged_names(&resolved.names, row.extra_sprites)
        .into_iter()
        .filter(|name| !hidden.contains(&name.as_str()))
        .collect();
    load_figures(
        archive,
        row.state,
        resolved.block_name,
        ctx,
        &visible_names,
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

/// Stage one Playing-subset row: the runtime-built BACK1 backdrop
/// first (the exe Playing setup re-stages `IDC_IMAGE_SPLAY_BACK1`
/// onto the mode-family full-screen body before every other pass —
/// `docs/playing-backdrop.md`), then behind entries, backdrops, and
/// the bound run, with figures through the context's room mode
/// (unselected family members leave empty slots with no miss), plus
/// the row's extra bodies merged by name.
fn stage_playing_row(archive: &[u8], row: &StageRow, ctx: StageContext) -> StageReport {
    let mut report = StageReport::default();
    let Some(resolved) = resolve_bound_row(archive, row, &mut report) else {
        return report;
    };
    let block_name = resolved.block_name;
    let backdrop = playing_backdrop_sprite(ctx.team_play, ctx.chart_slot);
    let mut names = vec![backdrop.clone()];
    names.extend(resolved.names.iter().cloned());
    let mut art = StagedArt {
        placements: bind_scene_rects(&resolved.block, &resolved.bind),
        figures: load_playing_figures(archive, &names, ctx, &mut report, row.state, block_name),
    };
    art.placements.insert(0, backdrop_placement(&backdrop));
    for extra in row.extra_sprites {
        match load_staged_figure(archive, extra, ctx.arena_stage) {
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

/// The synthetic BACK1 backdrop placement: the exe builds the
/// staging record on the stack (no ControlList entry names the
/// runtime-built body), so the placement carries the BACK1 code with
/// the all-zero rect — anchored bodies draw at their record anchor
/// `(0,0)` full-screen, never at the rect.
fn backdrop_placement(sprite: &str) -> ScenePlacement {
    ScenePlacement {
        role: PlacementRole::Single,
        id: String::new(),
        code: PLAYING_BACKDROP_CODE,
        sprite: Some(sprite.to_owned()),
        rect: Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        },
    }
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
            role: PlacementRole::Single,
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
        match load_staged_figure(archive, name, ctx.arena_stage) {
            Some(figure) => {
                art.figures.insert(name.clone(), figure);
            }
            None => push_miss(report, state, block_name, name),
        }
    }
}

/// Decode the code-mounted room-panel mouse-over body. The 7.55
/// `ControlList_Interface.txt` names no control for it (the exe
/// mounts it in code), so the bound `STATE_ROOM` run cannot stage it.
/// Callers merge this figure into the Room scene like the channel
/// dialog's hand-staged select bar. Missing art leaves no overlay.
#[must_use]
pub fn stage_room_hover_overlay(archive: &[u8]) -> Option<AvatarFigure> {
    load_archive_sprite_for_stage(archive, ROOM_HOVER_SPRITE, None)
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
/// Each archive is shared immutable bytes, not a per-caller copy.
/// `Arc<Vec<u8>>` moves the file buffer into the allocation: converting
/// to `Arc<[u8]>` would copy all 284 MB on first staging.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallArchives {
    /// Interface art (`ControlList_Interface.txt` Blocks).
    pub interface: Option<Arc<Vec<u8>>>,
    /// Playing art (Loading gate, Playing gauges).
    pub playing: Option<Arc<Vec<u8>>>,
}

/// One decoded pair per install root, filled on first access.
/// The key is the caller-supplied root: #736 can move root resolution
/// to runtime without changing what this cache means.
static DECODED_ARCHIVE_CACHE: OnceLock<Mutex<HashMap<PathBuf, InstallArchives>>> = OnceLock::new();

/// Read the staging archives under `root`: the repository-local
/// `install-root/` for development and tests, the client's own
/// install directory for the shipped game. No `env!`: the edge
/// that boots the client decides the root. Repeated reads of one
/// root share the same decoded bytes for the life of the process.
#[must_use]
pub fn read_install_archives(root: &Path) -> InstallArchives {
    if !root.is_dir() {
        return InstallArchives::default();
    }
    let cache = DECODED_ARCHIVE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache
        .entry(key)
        .or_insert_with(|| decode_install_archives(root))
        .clone()
}

/// Read both archives once when their root is first staged.
fn decode_install_archives(root: &Path) -> InstallArchives {
    InstallArchives {
        interface: read_archive_file(&root.join(INTERFACE_ARCHIVE_RELATIVE)),
        playing: read_archive_file(&root.join(PLAYING_ARCHIVE_RELATIVE)),
    }
}

/// Read one archive read-only, if present, as shared immutable bytes.
fn read_archive_file(path: &Path) -> Option<Arc<Vec<u8>>> {
    let bytes = std::fs::read(path).ok()?;
    Some(Arc::new(bytes))
}

#[cfg(test)]
#[path = "staging/tests.rs"]
mod staging_tests;
