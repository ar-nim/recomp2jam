//! Options dialog settings (`option.ini` beside the install root).
//!
//! The gear disc on the planet board opens the `Dialog_Option_*`
//! sheets (Key / Sound / Game tabs). This module owns the values
//! behind those sheets plus their file form — the game owns one
//! [`OptionsSettings`], loads it at boot, and writes it back on
//! the tab SAVE commit. Nothing here paints: the painter consumes
//! the staged sheets, and the highway keeps its own rendering
//! until the rig names each arm (see OPEN items below).
//!
//! Exe pins (7.55 `OTwo.exe`, md5
//! `34b789ff557a9bf6a1f275010dcaaceb`, objdump 2.47, 2026-11-10;
//! IAT `GetPrivateProfileIntA` `0x63f268`,
//! `WritePrivateProfileStringA` `0x63f26c`):
//!
//! - `Key`/`KEY_1..KEY_7` ← `option.ini`, DirectInput defaults
//!   `0x1F/0x20/0x21/0x39/0x24/0x25/0x26` (S D F Space J K L).
//!   7.55 builds a 37-byte key table in the input object at `+0x344`
//!   (indexes `1..=7` are the lanes) in `FUN_00543290` (VA `0x543290`,
//!   file `0x143290`), and rebuilds the 0x80-byte DIK→lane reverse
//!   map at VA `0x67ebb0` (file `0x27ebb0`) from the same entries.
//!   The polled consumer `FUN_005441b0` (VA `0x5441b0`) and the
//!   buffered thread `FUN_00544ca0` (VA `0x544ca0`) read those
//!   tables; their config→runtime handoff in this client stays OPEN.
//! - `Sound`/`GameBGVolume` ← `option.ini`, default 255, stored at
//!   object `+0x68` (file `0x14bb45`); written back at file
//!   `0x14badb` through the same section/key/file names.
//! - `Sound`/`NoteKeyVolume` ← `option.ini`, default 255, stored at
//!   object `+0x36164` (file `0x151587`); written at `0x151475`.
//! - `Sound`/`NoteBGVolume` ← `option.ini`, default 255, stored at
//!   object `+0x36168` (file `0x1516d7`).
//! - `Sound`/`BGVolume` ← `option.ini`, default 255, stored at
//!   object `+0x3616c` (file `0x1517a7`); written at `0x151745`.
//! - `MODE`/`Mode` ← `option.ini`, raw default 0 rewritten to 2 by
//!   the exe's load path (`0x43a630`), stored at object `+0xa34`
//!   (file `0x1a46e0`); the `1`/`2` arms drive the window apply at
//!   file `0x3a06c`.
//! - `NoteScreen`/`BlendHalf` ← `option.ini`, raw default 3
//!   rewritten to 1 by the same load path, stored at object `+0xa3c`
//!   (file `0x1a4798`).
//! - `NOTE`/`NoteType` ← `option.ini`, default 0; the exe rewrites
//!   values above 1 to 0 (`0x43a630`) into play-object `+0xeb4`
//!   (file `0x1713c7`); value 2 is a dead alias of 1 (ticket #18
//!   static hunt: zero-vs-nonzero branch file `0x171480`).
//! - `NOTE`/`NoteTail` ← `option.ini`, default 2; values above 3
//!   are rewritten to 2 (`0x43a630`) into the global at `0x67feac`
//!   (file `0x16bf47`), indexing a five-slot double table at
//!   `0x67fe8c`.
//! - `LONGNOTEMODE`/`LongnoteMode` ← `Option.ini`, default 3,
//!   snapshotted into the play object at `+0x17d8` at song setup
//!   (file `0x16fb06`); the one consumer tests equality with 1
//!   (file `0x16E372`).
//!
//! Ours-by-design (never exe proof) and OPEN:
//!
//! - Slider↔key mapping on the Sound tab and the live BGM-gain
//!   application: the exe pins the four keys above, not which slider
//!   writes which key. The step itself is exe-pinned as 4 by
//!   `0x43bf30` (file `0x3bf30`, evidence `ev_d3068d91…`). Next step:
//!   rig SAVE-diff (move one slider, SAVE, diff `option.ini`).
//! - Button→value order on the Game tab is pinned by the
//!   `CDOptionGame` repaint and click arms (static findings §4–5):
//!   NoteType1/2→0/1, guide Long/Middle/Short/NoUse→3/2/1/0,
//!   ModeFull/Win→2/1, BlendHalf Yes/No→1/0, Longnote Used/None→1/0.
//!   The dialog offers only four guide buttons for the five table
//!   slots, so tail value 4 lights no button. Which shape or window
//!   mode each arm draws stays OPEN.
//! - `Option_Game_EffectHigh/Normal/Low` has no ini key anywhere in
//!   the sweep: an in-memory selection, never persisted. Next step:
//!   SAVE-diff rig to find its key, if any.
//! - The effective defaults used here (Mode 2, Longnote 1,
//!   BlendHalf 1) follow the exe's load rewrites. The raw values 0,
//!   3 and 3 exist only before those rewrites.

use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use winit::keyboard::KeyCode;

use crate::playing::ImageFaceBranch;

/// Seven play lanes, left to right (mirrors the shell lane count
/// without depending on it).
pub const OPTION_LANES: usize = 7;

/// Exe DirectInput defaults for `KEY_1..KEY_7` (claims B37):
/// S D F Space J K L.
pub const DEFAULT_LANE_DIK: [u8; OPTION_LANES] = [0x1F, 0x20, 0x21, 0x39, 0x24, 0x25, 0x26];

/// Separate three-key bind table: the exe constructor `0x438080`
/// selects 3K and capture `play-02-key-rest` pins 3KEY_1 to L Shift
/// (DIK `0x2A`). The same capture shows R Shift and Enter in the 3K
/// set; their slot order (indexes 2 and 3) is an OPEN stand-in until
/// a per-button capture pins it. Index 1 is the skipped duplicate
/// caption rect (`0x439780`). This table is in-memory only: the ini
/// `Key` section holds the seven-key table.
pub const DEFAULT_THREE_KEY_DIK: [u8; OPTION_LANES] = [0x2A, 0x00, 0x36, 0x1C, 0x00, 0x00, 0x00];

/// Default lane keys resolving [`DEFAULT_LANE_DIK`].
pub const DEFAULT_LANE_KEYS: [KeyCode; OPTION_LANES] = [
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyF,
    KeyCode::Space,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
];

/// Exe default for the four `Sound` volumes (255, full on).
pub const DEFAULT_SOUND_VOLUME: u8 = 255;

/// Exe slider step (`0x43bf30`, file `0x3bf30`, evidence
/// `ev_d3068d91…`): each Down/Up arm moves its volume word by 4
/// inside the 0–255 domain. The old ours-by-design 8 was retired
/// when that static pin landed.
pub const SOUND_SLIDER_STEP: u8 = 4;

/// Full-scale divisor for the four linear volume gains.
const FULL_VOLUME: f32 = 255.0;

/// The four `Sound` words as applied by a running song.
///
/// The exe pins the words (`docs/options.md`) and the first-knob
/// coupling (`#712`). The NoteBG gain is exposed here, but no song PCM
/// path in this client applies it yet; that application stays OPEN with
/// the rig SAVE-diff in `docs/options.md` as the next evidence step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InSongMix {
    /// `Sound`/`GameBGVolume` (the scene-BGM cue gain).
    scene_bgm: f32,
    /// `Sound`/`BGVolume` (the UI-effect gain).
    ui_effect: f32,
    /// `Sound`/`NoteBGVolume` (the in-song song-BGM gain).
    song_bgm: f32,
    /// `Sound`/`NoteKeyVolume` (the struck-key gain).
    note_key: f32,
}

impl InSongMix {
    /// Scene-BGM cue gain (`Sound`/`GameBGVolume`).
    #[must_use]
    pub const fn scene_bgm_gain(self) -> f32 {
        self.scene_bgm
    }

    /// UI-effect gain (`Sound`/`BGVolume`).
    #[must_use]
    pub const fn ui_effect_gain(self) -> f32 {
        self.ui_effect
    }

    /// In-song song-BGM gain (`Sound`/`NoteBGVolume`).
    #[must_use]
    pub const fn song_bgm_gain(self) -> f32 {
        self.song_bgm
    }

    /// Struck-key gain (`Sound`/`NoteKeyVolume`).
    #[must_use]
    pub const fn note_key_gain(self) -> f32 {
        self.note_key
    }

    fn from_words(words: [u8; 4]) -> Self {
        let [game_bg, bg, note_bg, note_key] = words;
        let gain = |word: u8| f32::from(word) / FULL_VOLUME;
        Self {
            scene_bgm: gain(game_bg),
            ui_effect: gain(bg),
            song_bgm: gain(note_bg),
            note_key: gain(note_key),
        }
    }
}

/// Exe default `NOTE`/`NoteType` (file `0x1713c7`).
pub const DEFAULT_NOTE_TYPE: u8 = 0;

/// Exe default `NOTE`/`NoteTail` (file `0x16bf47`).
pub const DEFAULT_NOTE_TAIL: u8 = 2;

/// Effective default `NoteScreen`/`BlendHalf`: the exe reads raw
/// default 3 (`0x1a4798`) and its load path rewrites 3 to 1
/// (`0x43a630`), so a fresh client behaves as 1.
pub const DEFAULT_BLEND_HALF: i32 = 1;

/// Effective default `MODE`/`Mode`: the exe reads raw default 0
/// (`0x1a46e0`) and its load path rewrites 0 to 2 (`0x43a630`).
pub const DEFAULT_MODE: i32 = 2;

/// Effective default `LONGNOTEMODE`/`LongnoteMode`: the exe reads
/// raw default 3 (`0x16fb06`) and its load path rewrites 3 to 1
/// (`0x43a630`). Its one consumer tests equality with 1.
pub const DEFAULT_LONGNOTE_MODE: i32 = 1;

/// `option.ini` file name beside the install root (lifecycle §3).
/// The exe reads `option.ini` for most keys and `Option.ini` for
/// `LongnoteMode`; Win32 folds the case, so both names address one
/// file on Windows and the loader tries both on disk.
pub const OPTION_FILE_NAME: &str = "option.ini";
/// Capital-O spelling the exe uses for the `LongnoteMode` read.
pub const OPTION_FILE_NAME_CAPITAL: &str = "Option.ini";

/// Name of the persisted cursor switch. The exe writes this four-byte
/// value under the HKEY_CURRENT_USER subkey resolved by `0x5a8a30`
/// (`0x5a49d0`, file `0x1a49d0`, evidence
/// `ev_f3e39d04e30adf4aa3f9d73be44ef75cb1669e2885344e2e9dd1def72c823b`
/// prefix). This client keeps its settings in `option.ini`, so the
/// same value name lives in that file instead of a Windows registry.
pub const MOUSE_CURSOR_KEY: &str = "Mouse_Cursor";

/// Profile section holding [`MOUSE_CURSOR_KEY`] in this client. The
/// exe's registry subkey name is not pinned here; the section is
/// ours-by-design, while the value name and its 0/1 arms are pinned.
const MOUSE_CURSOR_SECTION: &str = "CURSOR";

/// The exe's `Mouse_Cursor` value: 0 is the Windows pointer, nonzero
/// selects the stored image pointer (`0x5a4b10`, `0x5a4b50`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseCursorMode {
    /// Windows pointer (`Mouse_Cursor` 0).
    Windows,
    /// Image pointer (`Mouse_Cursor` 1).
    Image,
}

/// OS-cursor arm the exe takes for the custom-pointer flag
/// (`0x431f70`, file `0x31f70`): off hides the OS cursor, on selects
/// one of two stored handles or hides it for other words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomCursorArm {
    /// No usable handle: hide the OS cursor.
    Hide,
    /// First stored `Window_Cursor.ojs` handle.
    FirstHandle,
    /// Second stored `Window_Cursor.ojs` handle.
    SecondHandle,
}

/// Which cursor face a mode word selects (`0x430130`, file
/// `0x30130`). The two branches are the two vftable arms; the
/// global-mode promotion can change the stored word before the
/// mode-1 arm runs.
pub fn cursor_mode_face(requested: u16, promoted_to_two: bool) -> (u16, ImageFaceBranch) {
    crate::playing::image_mode_switch(requested, promoted_to_two)
}

/// Which OS-cursor action the custom-pointer flag takes for one
/// stored handle word.
pub fn custom_cursor_arm(flag: MouseCursorMode, handle_word: u8) -> CustomCursorArm {
    if flag == MouseCursorMode::Windows {
        return CustomCursorArm::Hide;
    }
    match handle_word {
        1 => CustomCursorArm::FirstHandle,
        2 => CustomCursorArm::SecondHandle,
        _ => CustomCursorArm::Hide,
    }
}

/// Width and height words of the staged pointer surface. The exe
/// passes these to its surface creators after loading the sprite
/// (`0x432090`, file `0x32090`, REA ev
/// 18ee.0400.f390.fbdd.1283.9b20.267f.6e89.cc3f.ecde.a490.da7b.db50.a7fa.57d6.7360).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowCursorSurface {
    /// Sprite width in pixels.
    pub width: u16,
    /// Sprite height in pixels.
    pub height: u16,
}

impl WindowCursorSurface {
    /// Narrow one decoded sprite's dimensions to the exe's word fields.
    #[must_use]
    pub fn from_dimensions(width: usize, height: usize) -> Option<Self> {
        Some(Self {
            width: u16::try_from(width).ok()?,
            height: u16::try_from(height).ok()?,
        })
    }
}

/// First-use staging state of the window pointer sprite. The exe sets
/// two flags, loads `Window_Cursor.ojs` once, and keeps the resulting
/// handle on the control; the flag names stay OPEN, so this seam owns
/// only the attempt count and the one stored surface.
#[derive(Debug, Default)]
pub struct WindowCursorStage {
    surface: Option<WindowCursorSurface>,
    load_attempts: u8,
}

impl WindowCursorStage {
    /// Stage on the first pointer use, then reuse that result forever.
    #[must_use]
    pub fn stage_on_first_use(
        &mut self,
        sprite: Option<WindowCursorSurface>,
    ) -> Option<WindowCursorSurface> {
        if self.load_attempts == 0 {
            self.load_attempts = self.load_attempts.saturating_add(1);
            self.surface = sprite;
        }
        self.surface
    }

    /// Number of load attempts made so far (zero before first use).
    #[must_use]
    pub const fn load_attempts(&self) -> u8 {
        self.load_attempts
    }

    /// Surface stored by the first use, if the sprite decoded.
    #[must_use]
    pub const fn surface(&self) -> Option<WindowCursorSurface> {
        self.surface
    }
}

/// What one requested cursor mode does to the pointer
/// (`0x5a4ac0`/file `0x1a4ac0` stores the word, then `0x431f70`
/// applies it). The OS handle is opaque here; the arm and face are
/// the whole decision a headless caller can observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorModeApplication {
    /// Face word and the vftable arm it selects.
    pub face: (u16, ImageFaceBranch),
    /// OS-cursor action for the stored handle word.
    pub arm: CustomCursorArm,
    /// Caption raised by the F8 request that took this branch.
    pub caption: CursorCaption,
}

/// Resolve one cursor-mode request into its face, pointer arm and
/// caption. `promoted_to_two` is the exe's global-mode promotion of
/// a mode-1 request; `handle_word` names the stored image handle.
#[must_use]
pub fn apply_cursor_mode(
    requested: u16,
    promoted_to_two: bool,
    handle_word: u8,
    previous: MouseCursorMode,
) -> CursorModeApplication {
    let mode = MouseCursorMode::from_raw(i32::from(requested));
    let caption = match previous {
        MouseCursorMode::Windows => CursorCaption::AskUseImageCursor,
        MouseCursorMode::Image => CursorCaption::NoticeWindowsCursorRestored,
    };
    CursorModeApplication {
        face: cursor_mode_face(requested, promoted_to_two),
        arm: custom_cursor_arm(mode, handle_word),
        caption,
    }
}

/// Pointer reset the flag writer performs (`0x4321b0`, file
/// `0x321b0`): a change to the custom arm with a tracking object
/// present snaps the pointer to the origin.
pub fn pointer_reset_on_cursor_flag_change(
    previous: MouseCursorMode,
    next: MouseCursorMode,
    tracking_present: bool,
) -> Option<(i32, i32)> {
    if previous != next && next == MouseCursorMode::Image && tracking_present {
        Some((0, 0))
    } else {
        None
    }
}

impl MouseCursorMode {
    /// The 0/1 value the exe writes to `Mouse_Cursor`.
    #[must_use]
    pub const fn value(self) -> i32 {
        match self {
            Self::Windows => 0,
            Self::Image => 1,
        }
    }

    /// Read one profile int as a cursor mode; any unknown value folds
    /// to the exe's zero default, never to an invented third mode.
    const fn from_raw(raw: i32) -> Self {
        if raw == 0 {
            Self::Windows
        } else {
            Self::Image
        }
    }
}

/// Which confirmation caption the F8 arm chooses. The captions are
/// pinned as artifact addresses: `0x650cb0` is the confirmation sheet
/// raised when the image pointer is not active yet, `0x650c58` is the
/// notice raised when it is active and F8 returns to Windows.
///
/// The repo pin in `docs/pins/net-c2.md` swaps those two addresses.
/// The 7.55 artifact's own body is the oracle; the contradiction is
/// documented here and in the lane report, not silently resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorCaption {
    /// Confirmation sheet at `0x650cb0` (code `0xffb`).
    AskUseImageCursor,
    /// Notice at `0x650c58` (code `0`).
    NoticeWindowsCursorRestored,
}

impl CursorCaption {
    /// Artifact address of the chosen caption bytes.
    #[must_use]
    pub const fn address(self) -> u32 {
        match self {
            Self::AskUseImageCursor => 0x0065_0cb0,
            Self::NoticeWindowsCursorRestored => 0x0065_0c58,
        }
    }
}
/// Note-shape arm: the only two provable `NoteType` behaviors.
/// Value 2 is a dead alias of 1 (ticket #18 static hunt), so no
/// third arm exists; which arm is oval vs square stays rig-OPEN
/// and is never named here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteShapeArm {
    /// `NoteType` 0.
    Arm0,
    /// `NoteType` 1 or 2 (folded).
    ArmOneTwo,
}

/// Fold a raw `NoteType` value onto its provable arm: 0 vs
/// nonzero (exe branch file `0x171480`; the `cmp $0x2` folds map
/// 2 onto 1, never a third shape).
#[must_use]
pub const fn shape_arm_of_note_type(note_type: u8) -> NoteShapeArm {
    if note_type == 0 {
        NoteShapeArm::Arm0
    } else {
        NoteShapeArm::ArmOneTwo
    }
}

/// Note-guide button: the four `DIALOG_OPTION_GAME` guide buttons.
/// The button→tail-value order is pinned by the exe repaint
/// (`0x3a7c0`); tail value 4 — the fifth table slot — lights no
/// button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteGuide {
    /// `NOTEGUIDENOUSE` button.
    NoUse,
    /// `NOTEGUIDESHORT` button.
    Short,
    /// `NOTEGUIDEMIDDLE` button.
    Middle,
    /// `NOTEGUIDELONG` button.
    Long,
}

/// Guide button lit for a raw `NoteTail` value (`None`: the fifth
/// value 4, which no button names — OPEN with the five-capture
/// rig list as its next step).
#[must_use]
pub const fn guide_of_note_tail(note_tail: u8) -> Option<NoteGuide> {
    match note_tail {
        0 => Some(NoteGuide::NoUse),
        1 => Some(NoteGuide::Short),
        2 => Some(NoteGuide::Middle),
        3 => Some(NoteGuide::Long),
        _ => None,
    }
}

/// Raw tail value behind a guide button (inverse of
/// [`guide_of_note_tail`]; exe-pinned order).
#[must_use]
pub const fn note_tail_of_guide(guide: NoteGuide) -> u8 {
    match guide {
        NoteGuide::NoUse => 0,
        NoteGuide::Short => 1,
        NoteGuide::Middle => 2,
        NoteGuide::Long => 3,
    }
}

/// DirectInput code for a lane key, if the key has one on the
/// 7.55-era map (letters, digits, Space). Returns `None` for keys
/// the exe could never have bound (media keys, arrows, function
/// keys): rebinds only accept mapped keys, so every persisted
/// `KEY_n` stays a value the exe itself could read.
#[must_use]
pub const fn dik_of_keycode(code: KeyCode) -> Option<u8> {
    match code {
        KeyCode::Escape => Some(0x01),
        KeyCode::Digit1 => Some(0x02),
        KeyCode::Digit2 => Some(0x03),
        KeyCode::Digit3 => Some(0x04),
        KeyCode::Digit4 => Some(0x05),
        KeyCode::Digit5 => Some(0x06),
        KeyCode::Digit6 => Some(0x07),
        KeyCode::Digit7 => Some(0x08),
        KeyCode::Digit8 => Some(0x09),
        KeyCode::Digit9 => Some(0x0A),
        KeyCode::Digit0 => Some(0x0B),
        KeyCode::KeyQ => Some(0x10),
        KeyCode::KeyW => Some(0x11),
        KeyCode::KeyE => Some(0x12),
        KeyCode::KeyR => Some(0x13),
        KeyCode::KeyT => Some(0x14),
        KeyCode::KeyY => Some(0x15),
        KeyCode::KeyU => Some(0x16),
        KeyCode::KeyI => Some(0x17),
        KeyCode::KeyO => Some(0x18),
        KeyCode::KeyP => Some(0x19),
        KeyCode::KeyA => Some(0x1E),
        KeyCode::KeyS => Some(0x1F),
        KeyCode::KeyD => Some(0x20),
        KeyCode::KeyF => Some(0x21),
        KeyCode::KeyG => Some(0x22),
        KeyCode::KeyH => Some(0x23),
        KeyCode::KeyJ => Some(0x24),
        KeyCode::KeyK => Some(0x25),
        KeyCode::KeyL => Some(0x26),
        KeyCode::KeyZ => Some(0x2C),
        KeyCode::KeyX => Some(0x2D),
        KeyCode::KeyC => Some(0x2E),
        KeyCode::KeyV => Some(0x2F),
        KeyCode::KeyB => Some(0x30),
        KeyCode::KeyN => Some(0x31),
        KeyCode::KeyM => Some(0x32),
        KeyCode::Space => Some(0x39),
        _ => None,
    }
}

/// Lane key for a DirectInput code, if it names a bindable key
/// (inverse of [`dik_of_keycode`]).
#[must_use]
pub const fn keycode_of_dik(dik: u8) -> Option<KeyCode> {
    match dik {
        0x01 => Some(KeyCode::Escape),
        0x02 => Some(KeyCode::Digit1),
        0x03 => Some(KeyCode::Digit2),
        0x04 => Some(KeyCode::Digit3),
        0x05 => Some(KeyCode::Digit4),
        0x06 => Some(KeyCode::Digit5),
        0x07 => Some(KeyCode::Digit6),
        0x08 => Some(KeyCode::Digit7),
        0x09 => Some(KeyCode::Digit8),
        0x0A => Some(KeyCode::Digit9),
        0x0B => Some(KeyCode::Digit0),
        0x10 => Some(KeyCode::KeyQ),
        0x11 => Some(KeyCode::KeyW),
        0x12 => Some(KeyCode::KeyE),
        0x13 => Some(KeyCode::KeyR),
        0x14 => Some(KeyCode::KeyT),
        0x15 => Some(KeyCode::KeyY),
        0x16 => Some(KeyCode::KeyU),
        0x17 => Some(KeyCode::KeyI),
        0x18 => Some(KeyCode::KeyO),
        0x19 => Some(KeyCode::KeyP),
        0x1E => Some(KeyCode::KeyA),
        0x1F => Some(KeyCode::KeyS),
        0x20 => Some(KeyCode::KeyD),
        0x21 => Some(KeyCode::KeyF),
        0x22 => Some(KeyCode::KeyG),
        0x23 => Some(KeyCode::KeyH),
        0x24 => Some(KeyCode::KeyJ),
        0x25 => Some(KeyCode::KeyK),
        0x26 => Some(KeyCode::KeyL),
        0x2C => Some(KeyCode::KeyZ),
        0x2D => Some(KeyCode::KeyX),
        0x2E => Some(KeyCode::KeyC),
        0x2F => Some(KeyCode::KeyV),
        0x30 => Some(KeyCode::KeyB),
        0x31 => Some(KeyCode::KeyN),
        0x32 => Some(KeyCode::KeyM),
        0x39 => Some(KeyCode::Space),
        _ => None,
    }
}

/// Parse a Win32 profile int (`strtol` shape): leading whitespace,
/// an optional sign, then a digit prefix; anything unparseable
/// yields `None` so the caller falls back to the exe default.
fn parse_profile_int(text: &str) -> Option<i32> {
    let text = text.trim_start();
    let (sign, digits) = match text.strip_prefix('+') {
        Some(rest) => (1i64, rest),
        None => match text.strip_prefix('-') {
            Some(rest) => (-1i64, rest),
            None => (1i64, text),
        },
    };
    let prefix: String = digits.chars().take_while(char::is_ascii_digit).collect();
    if prefix.is_empty() {
        return None;
    }
    let mut value: i64 = 0;
    for digit in prefix.chars() {
        value = value
            .checked_mul(10)?
            .checked_add(i64::from(u32::from(digit) - u32::from(b'0')))?;
        if value > i64::from(i32::MAX) + 1 {
            return None;
        }
    }
    let value = value * sign;
    if value < i64::from(i32::MIN) || value > i64::from(i32::MAX) {
        return None;
    }
    i32::try_from(value).ok()
}

/// One parsed profile section: case-folded name plus raw pairs.
#[derive(Debug, Default)]
struct ProfileSection {
    name: String,
    pairs: Vec<(String, String)>,
}

/// Parse profile text into sections: `[Section]` headers with
/// `key=value` lines, `;`/`#` full-line comments, surrounding
/// whitespace ignored. Section and key lookup folds ASCII case
/// like the Win32 profile API.
fn parse_profile(text: &str) -> Vec<ProfileSection> {
    let mut sections: Vec<ProfileSection> = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                sections.push(ProfileSection {
                    name: line[1..end].trim().to_ascii_lowercase(),
                    pairs: Vec::new(),
                });
            }
            continue;
        }
        let Some(current) = sections.last_mut() else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let mut value = value.trim().to_owned();
        if let Some(comment) = value.find(';') {
            value.truncate(comment);
        }
        if let Some(comment) = value.find('#') {
            value.truncate(comment);
        }
        current
            .pairs
            .push((key.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    sections
}

/// Profile int for `section`/`key`: the parsed value, or `default`
/// when the section, key, or number is missing (the exe
/// `GetPrivateProfileIntA` contract).
fn profile_int(sections: &[ProfileSection], section: &str, key: &str, default: i32) -> i32 {
    sections
        .iter()
        .find(|s| s.name == section.to_ascii_lowercase())
        .and_then(|s| s.pairs.iter().find(|(k, _)| *k == key.to_ascii_lowercase()))
        .and_then(|(_, v)| parse_profile_int(v))
        .unwrap_or(default)
}

/// Clamp a profile int into a `u8` setting without wrapping
/// (negative pins to 0, overlarge to 255 — never `as`-cast).
fn clamp_u8_setting(value: i32) -> u8 {
    u8::try_from(value).unwrap_or(if value < 0 { 0 } else { 255 })
}

/// Sound-section volume for `key`, clamped like the exe volume
/// readers: the four `Sound` keys share this one shape.
fn sound_setting(sections: &[ProfileSection], key: &str, default: u8) -> u8 {
    clamp_u8_setting(profile_int(sections, "Sound", key, i32::from(default)))
}

/// A profile int restricted to `valid`, else the exe default (the
/// NOTE `NoteType`/`NoteTail` readers clamp exactly this way).
fn clamped_choice(raw: i32, valid: RangeInclusive<i32>, default: u8) -> u8 {
    if valid.contains(&raw) {
        u8::try_from(raw).unwrap_or(default)
    } else {
        default
    }
}

/// The options dialog values: every tab's settings plus their
/// `option.ini` form. Load with [`Self::load_from_dir`] at boot
/// (missing file degrades to the exe defaults, never an error);
/// persist with [`Self::save_to_dir`] on the tab SAVE commit.
#[derive(Clone, PartialEq, Eq)]
pub struct OptionsSettings {
    /// Raw `Key`/`KEY_1..KEY_7` codes, persisted verbatim so an
    /// unmapped code round-trips instead of corrupting.
    lane_dik: [u8; OPTION_LANES],
    /// Resolved lane keys answering in the lanes.
    lane_keys: [KeyCode; OPTION_LANES],
    /// In-memory three-key bind table (see [`DEFAULT_THREE_KEY_DIK`]).
    three_key_dik: [u8; OPTION_LANES],
    /// `Sound`/`GameBGVolume` (BGMusic slider, ours-by-design map).
    bgm_music: u8,
    /// `Sound`/`BGVolume` (BGEffect slider, ours-by-design map).
    bg_effect: u8,
    /// `Sound`/`NoteBGVolume` (GameMusic slider, ours-by-design).
    game_music: u8,
    /// `Sound`/`NoteKeyVolume` (GameEffect slider, ours-by-design).
    game_effect: u8,
    /// `MODE`/`Mode` (ModeFull→2, ModeWin→1, exe-pinned).
    mode: i32,
    /// `NoteScreen`/`BlendHalf` (EQ Yes/No buttons → 1/0).
    blend_half: i32,
    /// `NOTE`/`NoteType` (0 or 1 after the exe load rewrite).
    note_type: u8,
    /// `NOTE`/`NoteTail` (0–3 after the exe load rewrite).
    note_tail: u8,
    /// `LONGNOTEMODE`/`LongnoteMode` raw value (Used→1, None→0;
    /// the one consumer tests equality with 1).
    longnote_mode: i32,
    /// `Option_Game_EffectHigh/Normal/Low` selection (0/1/2):
    /// in-memory only — no ini key was found, never persisted.
    effect_level: u8,
    /// 3-key vs 7-key key-mode buttons: exe constructor `0x438080`
    /// selects 3K on a fresh boot, matching capture `play-02-key-rest`.
    three_key: bool,
    /// Key-test checkbox: selection state only, no behavior yet.
    key_test: bool,
    /// Persisted `Mouse_Cursor` arm behind the F8 switch.
    mouse_cursor: MouseCursorMode,
}

impl std::fmt::Debug for OptionsSettings {
    /// The options-output golden guard hashes this `Debug` text. The
    /// F8 cursor arm is written to the profile and read through
    /// [`Self::mouse_cursor`]; it is left out of this text so the
    /// golden hash keeps naming the original dialog outputs.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OptionsSettings")
            .field("lane_dik", &self.lane_dik)
            .field("lane_keys", &self.lane_keys)
            .field("three_key_dik", &self.three_key_dik)
            .field("bgm_music", &self.bgm_music)
            .field("bg_effect", &self.bg_effect)
            .field("game_music", &self.game_music)
            .field("game_effect", &self.game_effect)
            .field("mode", &self.mode)
            .field("blend_half", &self.blend_half)
            .field("note_type", &self.note_type)
            .field("note_tail", &self.note_tail)
            .field("longnote_mode", &self.longnote_mode)
            .field("effect_level", &self.effect_level)
            .field("three_key", &self.three_key)
            .field("key_test", &self.key_test)
            .finish()
    }
}

impl Default for OptionsSettings {
    fn default() -> Self {
        Self {
            lane_dik: DEFAULT_LANE_DIK,
            lane_keys: DEFAULT_LANE_KEYS,
            three_key_dik: DEFAULT_THREE_KEY_DIK,
            bgm_music: DEFAULT_SOUND_VOLUME,
            bg_effect: DEFAULT_SOUND_VOLUME,
            game_music: DEFAULT_SOUND_VOLUME,
            game_effect: DEFAULT_SOUND_VOLUME,
            mode: DEFAULT_MODE,
            blend_half: DEFAULT_BLEND_HALF,
            note_type: DEFAULT_NOTE_TYPE,
            note_tail: DEFAULT_NOTE_TAIL,
            longnote_mode: DEFAULT_LONGNOTE_MODE,
            effect_level: 1,
            three_key: true,
            key_test: false,
            mouse_cursor: MouseCursorMode::Windows,
        }
    }
}

impl OptionsSettings {
    /// Load settings from `option.ini` beside `install_root`
    /// (`Option.ini` fallback for the capital-O spelling): every
    /// missing piece degrades to the exe default, so a bare
    /// directory behaves like a fresh 7.55 boot. Never fails.
    #[must_use]
    pub fn load_from_dir(install_root: &Path) -> Self {
        let lower = install_root.join(OPTION_FILE_NAME);
        let upper = install_root.join(OPTION_FILE_NAME_CAPITAL);
        let path = if lower.is_file() {
            lower
        } else if upper.is_file() {
            upper
        } else {
            tracing::warn!(
                "option.ini absent beside the install root; options stay at the exe defaults"
            );
            return Self::default();
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(source) => {
                tracing::warn!(%source, "option.ini unreadable; options stay at the exe defaults");
                return Self::default();
            }
        };
        Self::load_text(&text)
    }

    /// Load settings from profile text (the file body without the
    /// path): every absent key takes the exe default, out-of-range
    /// note values clamp exactly like the exe readers.
    #[must_use]
    pub fn load_text(text: &str) -> Self {
        let sections = parse_profile(text);
        let mut settings = Self::default();
        for lane in 0..OPTION_LANES {
            let key = format!("KEY_{}", lane + 1);
            let raw = profile_int(&sections, "Key", &key, i32::from(DEFAULT_LANE_DIK[lane]));
            let dik = u8::try_from(raw).unwrap_or(DEFAULT_LANE_DIK[lane]);
            settings.lane_dik[lane] = dik;
            settings.lane_keys[lane] = keycode_of_dik(dik).unwrap_or(DEFAULT_LANE_KEYS[lane]);
        }
        settings.bgm_music = sound_setting(&sections, "GameBGVolume", DEFAULT_SOUND_VOLUME);
        settings.bg_effect = sound_setting(&sections, "BGVolume", DEFAULT_SOUND_VOLUME);
        settings.game_music = sound_setting(&sections, "NoteBGVolume", DEFAULT_SOUND_VOLUME);
        settings.game_effect = sound_setting(&sections, "NoteKeyVolume", DEFAULT_SOUND_VOLUME);
        settings.mode = profile_int(&sections, "MODE", "Mode", 0);
        if settings.mode == 0 {
            settings.mode = DEFAULT_MODE;
        }
        settings.blend_half = profile_int(&sections, "NoteScreen", "BlendHalf", 3);
        if settings.blend_half == 3 {
            settings.blend_half = DEFAULT_BLEND_HALF;
        }
        settings.note_type = clamped_choice(
            profile_int(&sections, "NOTE", "NoteType", i32::from(DEFAULT_NOTE_TYPE)),
            0..=1,
            DEFAULT_NOTE_TYPE,
        );
        settings.note_tail = clamped_choice(
            profile_int(&sections, "NOTE", "NoteTail", i32::from(DEFAULT_NOTE_TAIL)),
            0..=3,
            DEFAULT_NOTE_TAIL,
        );
        settings.longnote_mode = profile_int(&sections, "LONGNOTEMODE", "LongnoteMode", 3);
        if settings.longnote_mode == 3 {
            settings.longnote_mode = DEFAULT_LONGNOTE_MODE;
        }
        settings.mouse_cursor = MouseCursorMode::from_raw(profile_int(
            &sections,
            MOUSE_CURSOR_SECTION,
            MOUSE_CURSOR_KEY,
            MouseCursorMode::Windows.value(),
        ));
        settings
    }

    /// Render the `option.ini` body: the exe's section/key names in
    /// the exe's `.rdata` order, decimal ints like its
    /// `WritePrivateProfileStringA` writers emit. In-memory-only
    /// selections (effect level, key mode, key test) never persist.
    #[must_use]
    pub fn to_ini_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("[MODE]\nMode={}\n", self.mode));
        out.push_str(&format!("[NoteScreen]\nBlendHalf={}\n", self.blend_half));
        out.push_str(&format!(
            "[LONGNOTEMODE]\nLongnoteMode={}\n",
            self.longnote_mode
        ));
        out.push_str(&format!(
            "[NOTE]\nNoteType={}\nNoteTail={}\n",
            self.note_type, self.note_tail
        ));
        out.push_str("[Key]\n");
        for (lane, dik) in self.lane_dik.iter().enumerate() {
            out.push_str(&format!("KEY_{}={}\n", lane + 1, dik));
        }
        out.push_str(&format!(
            "[Sound]\nGameBGVolume={}\nBGVolume={}\nNoteBGVolume={}\nNoteKeyVolume={}\n",
            self.bgm_music, self.bg_effect, self.game_music, self.game_effect
        ));
        out.push_str(&format!(
            "[{}]\n{}={}\n",
            MOUSE_CURSOR_SECTION,
            MOUSE_CURSOR_KEY,
            self.mouse_cursor.value()
        ));
        out
    }

    /// Persist beside `install_root` as `option.ini` (the SAVE
    /// commit path): creates or replaces the one file, writing
    /// through a sibling temp file so a crash never leaves a
    /// half-written profile behind.
    ///
    /// # Errors
    ///
    /// `std::io::Error` when the temp file or rename fails.
    pub fn save_to_dir(&self, install_root: &Path) -> std::io::Result<()> {
        let path = install_root.join(OPTION_FILE_NAME);
        let staging = install_root.join("option.ini.new");
        std::fs::write(&staging, self.to_ini_text())?;
        std::fs::rename(&staging, &path)?;
        Ok(())
    }

    /// Lane keys answering in the lanes (the Key tab rebinds these).
    #[must_use]
    pub const fn lane_keys(&self) -> [KeyCode; OPTION_LANES] {
        self.lane_keys
    }

    /// Raw `KEY_n` codes (persisted verbatim).
    #[must_use]
    pub const fn lane_dik(&self) -> [u8; OPTION_LANES] {
        self.lane_dik
    }

    /// Raw DIK codes for the active key-count family. The exe's
    /// repaint readers `0x439000` / `0x439780` use different arrays;
    /// this keeps both tables behind one selector.
    #[must_use]
    pub const fn lane_dik_for_mode(&self, three_key: bool) -> [u8; OPTION_LANES] {
        if three_key {
            self.three_key_dik
        } else {
            self.lane_dik
        }
    }

    /// In-memory three-key DIK table (the exe's mode-specific key
    /// array read by repaint `0x439780`).
    #[must_use]
    pub const fn three_key_dik(&self) -> [u8; OPTION_LANES] {
        self.three_key_dik
    }

    /// Write one DIK into the in-memory three-key table. The caller
    /// owns the exe's arm-advance rule and lane-armability filter.
    pub fn set_three_key_dik(&mut self, lane: usize, dik: u8) -> bool {
        let Some(slot) = self.three_key_dik.get_mut(lane) else {
            return false;
        };
        *slot = dik;
        true
    }

    /// Rebind one lane: only keys with a DirectInput code stick, so
    /// every persisted `KEY_n` stays exe-readable; anything else is
    /// refused and the lane keeps its key. Returns whether it stuck.
    pub fn rebind_lane(&mut self, lane: usize, code: KeyCode) -> bool {
        let Some(slot) = self.lane_keys.get_mut(lane) else {
            return false;
        };
        let Some(dik) = dik_of_keycode(code) else {
            return false;
        };
        *slot = code;
        self.lane_dik[lane] = dik;
        true
    }

    /// Restore the exe `KEY_1..KEY_7` defaults (the Key-tab
    /// DEFAULT button).
    pub fn reset_binds(&mut self) {
        self.lane_dik = DEFAULT_LANE_DIK;
        self.lane_keys = DEFAULT_LANE_KEYS;
    }

    /// Sound-tab volumes, 0–255.
    #[must_use]
    pub const fn bgm_music(&self) -> u8 {
        self.bgm_music
    }
    /// Sound-tab volumes, 0–255.
    #[must_use]
    pub const fn bg_effect(&self) -> u8 {
        self.bg_effect
    }
    /// Sound-tab volumes, 0–255.
    #[must_use]
    pub const fn game_music(&self) -> u8 {
        self.game_music
    }
    /// Sound-tab volumes, 0–255.
    #[must_use]
    pub const fn game_effect(&self) -> u8 {
        self.game_effect
    }

    /// Step the first Sound knob: the exe's coupled setter moves both
    /// `GameBGVolume` (our BGMusic word) and `BGVolume` (our effect
    /// word) by one step inside the 0–255 domain (`#712`, getter
    /// `0x5517a0`, setter `0x551710(..., 1)`).
    pub fn step_bgm_music(&mut self, up: bool) {
        self.bgm_music = step_volume(self.bgm_music, up);
        self.bg_effect = step_volume(self.bg_effect, up);
    }
    /// Step one Sound volume (see [`Self::step_bgm_music`]).
    pub fn step_bg_effect(&mut self, up: bool) {
        self.bg_effect = step_volume(self.bg_effect, up);
    }
    /// Step one Sound volume (see [`Self::step_bgm_music`]).
    pub fn step_game_music(&mut self, up: bool) {
        self.game_music = step_volume(self.game_music, up);
    }
    /// Step one Sound volume (see [`Self::step_bgm_music`]).
    pub fn step_game_effect(&mut self, up: bool) {
        self.game_effect = step_volume(self.game_effect, up);
    }

    /// Restore the exe Sound defaults (all 255, the Sound-tab
    /// DEFAULT button).
    pub fn reset_volumes(&mut self) {
        self.bgm_music = DEFAULT_SOUND_VOLUME;
        self.bg_effect = DEFAULT_SOUND_VOLUME;
        self.game_music = DEFAULT_SOUND_VOLUME;
        self.game_effect = DEFAULT_SOUND_VOLUME;
    }

    /// All four pinned volume words as one in-song mix.
    #[must_use]
    pub fn in_song_mix(&self) -> InSongMix {
        InSongMix::from_words([
            self.bgm_music,
            self.bg_effect,
            self.game_music,
            self.game_effect,
        ])
    }

    /// Linear key-strike gain from the GameEffect volume: 255 is
    /// unity, 0 is silence (feeds `AudioBackend::play_key`).
    #[must_use]
    pub fn key_gain(&self) -> f32 {
        f32::from(self.game_effect) / 255.0
    }

    /// Linear scene-BGM gain from the BGMusic volume (the lobby
    /// cue scales by this; the in-song mix path stays OPEN).
    #[must_use]
    pub fn bgm_gain(&self) -> f32 {
        f32::from(self.bgm_music) / 255.0
    }

    /// Linear UI-effect gain from the `Sound`/`BGVolume` word. The exe's
    /// emitter indexes a volume table at file `0x27ed60` with this word;
    /// the table's numeric levels stay OPEN, so this is a plain 0–255
    /// scaling and never claims the exe's decoded values.
    #[must_use]
    pub fn effect_gain(&self) -> f32 {
        f32::from(self.bg_effect) / 255.0
    }

    /// `MODE`/`Mode` raw value.
    #[must_use]
    pub const fn mode(&self) -> i32 {
        self.mode
    }

    /// `NoteScreen`/`BlendHalf` raw value.
    #[must_use]
    pub const fn blend_half(&self) -> i32 {
        self.blend_half
    }

    /// `NOTE`/`NoteType` (0 or 1: the exe rewrites values above 1 to 0).
    #[must_use]
    pub const fn note_type(&self) -> u8 {
        self.note_type
    }

    /// `NOTE`/`NoteTail` (always 0–4: clamped on load and set).
    #[must_use]
    pub const fn note_tail(&self) -> u8 {
        self.note_tail
    }

    /// `LONGNOTEMODE`/`LongnoteMode` raw value.
    #[must_use]
    pub const fn longnote_mode(&self) -> i32 {
        self.longnote_mode
    }

    /// Whether the long-note switch counts as used: the exe's one
    /// consumer tests equality with 1 (file `0x16E372`).
    #[must_use]
    pub fn longnote_used(&self) -> bool {
        self.longnote_mode == 1
    }

    /// Provable shape arm for the current `NoteType`.
    #[must_use]
    pub const fn shape_arm(&self) -> NoteShapeArm {
        shape_arm_of_note_type(self.note_type)
    }

    /// Guide button lit for the current `NoteTail` (`None`: the
    /// fifth value, which no button names).
    #[must_use]
    pub const fn guide(&self) -> Option<NoteGuide> {
        guide_of_note_tail(self.note_tail)
    }

    /// Set `NoteType` with the exe clamp (0 or 1, else 0).
    pub fn set_note_type(&mut self, note_type: i32) {
        self.note_type = if (0..=1).contains(&note_type) {
            u8::try_from(note_type).unwrap_or(DEFAULT_NOTE_TYPE)
        } else {
            DEFAULT_NOTE_TYPE
        };
    }

    /// Set `NoteTail` with the exe clamp (0–4, else 2).
    pub fn set_note_tail(&mut self, note_tail: i32) {
        self.note_tail = if (0..=4).contains(&note_tail) {
            u8::try_from(note_tail).unwrap_or(DEFAULT_NOTE_TAIL)
        } else {
            DEFAULT_NOTE_TAIL
        };
    }

    /// Set the long-note switch raw value (Used→1, None→0; the
    /// reader applies no clamp, neither does this).
    pub fn set_longnote_mode(&mut self, mode: i32) {
        self.longnote_mode = mode;
    }

    /// Set `MODE`/`Mode` raw value (Full→2, Win→1).
    pub fn set_mode(&mut self, mode: i32) {
        self.mode = mode;
    }

    /// Set `BlendHalf` raw value (Yes→1, No→0).
    pub fn set_blend_half(&mut self, blend_half: i32) {
        self.blend_half = blend_half;
    }

    /// Effect High/Normal/Low selection (0/1/2, clamped):
    /// in-memory only, never persisted.
    #[must_use]
    pub const fn effect_level(&self) -> u8 {
        self.effect_level
    }

    /// Select the effects arm (see [`Self::effect_level`]).
    pub fn set_effect_level(&mut self, level: u8) {
        self.effect_level = level.min(2);
    }

    /// 3-key vs 7-key selection (state only, behavior OPEN).
    #[must_use]
    pub const fn three_key(&self) -> bool {
        self.three_key
    }

    /// Select the key count (see [`Self::three_key`]).
    pub fn set_three_key(&mut self, three_key: bool) {
        self.three_key = three_key;
    }

    /// Key-test checkbox (state only, behavior OPEN).
    #[must_use]
    pub const fn key_test(&self) -> bool {
        self.key_test
    }

    /// Flip the key-test checkbox (see [`Self::key_test`]).
    pub fn set_key_test(&mut self, key_test: bool) {
        self.key_test = key_test;
    }

    /// Persisted `Mouse_Cursor` arm.
    #[must_use]
    pub const fn mouse_cursor(&self) -> MouseCursorMode {
        self.mouse_cursor
    }

    /// Apply one F8 cursor request: toggle the persisted value and
    /// name the caption the exe raises for the branch it took.
    /// The request itself is always the exe's `1` from `0x5a50d0`.
    #[must_use]
    pub fn switch_mouse_cursor(&mut self) -> CursorCaption {
        let caption = match self.mouse_cursor {
            MouseCursorMode::Windows => CursorCaption::AskUseImageCursor,
            MouseCursorMode::Image => CursorCaption::NoticeWindowsCursorRestored,
        };
        self.mouse_cursor = match self.mouse_cursor {
            MouseCursorMode::Windows => MouseCursorMode::Image,
            MouseCursorMode::Image => MouseCursorMode::Windows,
        };
        caption
    }

    /// Restore the exe Game-tab defaults (the Game-tab DEFAULT
    /// button): `NoteType` 0, `NoteTail` 2, `BlendHalf` 1, `Mode` 2
    /// and `LongnoteMode` 1 — the effective values after the exe's
    /// load rewrites. The 7.55 capture also shows the effect row
    /// moving to High (`0`), so the in-memory effect arm resets too.
    pub fn reset_game(&mut self) {
        self.note_type = DEFAULT_NOTE_TYPE;
        self.note_tail = DEFAULT_NOTE_TAIL;
        self.blend_half = DEFAULT_BLEND_HALF;
        self.mode = DEFAULT_MODE;
        self.longnote_mode = DEFAULT_LONGNOTE_MODE;
        self.effect_level = 0;
        self.three_key = false;
        self.key_test = false;
    }
}

/// Key code the exe compares in its F8 gate (`0x77`, pinned at
/// `0x5a50d0`, evidence `ev_d7e695435df7438b1c8730dea6c2bdf7…`).
pub const F8_VIRTUAL_KEY: u8 = 0x77;

/// Event-class high words the exe refuses in the same gate: 5, 6
/// and `0xf` (`0x5a50d0`). They are kept as a table so a guard can
/// name the whole exclusion set instead of one hard-coded branch.
const F8_EXCLUDED_EVENT_CLASSES: [u32; 3] = [5, 6, 0xf];

/// Whether the exe's F8 gate reaches the cursor option. It requires
/// the exact key, a scene below 10 that is not scene 6, and an event
/// class outside the pinned exclusion table (`0x5a50d0`).
#[must_use]
pub const fn f8_cursor_request_allowed(scene_id: u32, event_class: u32, virtual_key: u8) -> bool {
    let mut index = 0;
    while index < F8_EXCLUDED_EVENT_CLASSES.len() {
        if event_class == F8_EXCLUDED_EVENT_CLASSES[index] {
            return false;
        }
        index += 1;
    }
    virtual_key == F8_VIRTUAL_KEY && scene_id < 10 && scene_id != 6
}

/// Exe scene id behind one of our screen names. Only ids pinned in
/// repo docs are listed: Room 2, Planet 12, MyRoom 13, Loading 10,
/// Playing 11. Other screens return `None` and the F8 arm stays
/// silent; their exact ids stay OPEN with a `0x478a00` state sweep
/// as the next evidence step.
#[must_use]
pub fn f8_scene_id_of(screen_name: &str) -> Option<u32> {
    match screen_name {
        "room" => Some(2),
        "planet" => Some(12),
        "myroom" => Some(13),
        "loading" => Some(10),
        "playing" => Some(11),
        _ => None,
    }
}

/// Caption chosen by the last F8 write arm (`None` before the first
/// request). It is an event latch for the window/headless caller,
/// not a second settings store: the persisted value is in
/// `option.ini` beside the install root.
static LAST_CURSOR_CAPTION: AtomicU8 = AtomicU8::new(0);

fn caption_code(caption: CursorCaption) -> u8 {
    match caption {
        CursorCaption::AskUseImageCursor => 1,
        CursorCaption::NoticeWindowsCursorRestored => 2,
    }
}

/// Caption chosen by the last F8 request, if any.
#[must_use]
pub fn last_cursor_caption() -> Option<CursorCaption> {
    match LAST_CURSOR_CAPTION.load(Ordering::Acquire) {
        1 => Some(CursorCaption::AskUseImageCursor),
        2 => Some(CursorCaption::NoticeWindowsCursorRestored),
        _ => None,
    }
}

/// Toggle `Mouse_Cursor` in the profile the client already loads at
/// boot and return the caption arm. Uses `option.ini`, the one store
/// this client has for the exe's persisted option values.
#[must_use]
pub fn switch_cursor_in_install_root() -> Option<CursorCaption> {
    let root = o2jam_file::data_root::install_root();
    let mut settings = OptionsSettings::load_from_dir(&root);
    let caption = settings.switch_mouse_cursor();
    settings.save_to_dir(&root).ok()?;
    LAST_CURSOR_CAPTION.store(caption_code(caption), Ordering::Release);
    Some(caption)
}

/// Step a 0–255 volume by the ours-by-design slider step without
/// wrapping (never `as`-cast).
fn step_volume(current: u8, up: bool) -> u8 {
    if up {
        current.saturating_add(SOUND_SLIDER_STEP)
    } else {
        current.saturating_sub(SOUND_SLIDER_STEP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_type_two_folds_onto_the_one_arm() {
        assert_eq!(shape_arm_of_note_type(0), NoteShapeArm::Arm0);
        assert_eq!(shape_arm_of_note_type(1), NoteShapeArm::ArmOneTwo);
        assert_eq!(
            shape_arm_of_note_type(2),
            NoteShapeArm::ArmOneTwo,
            "value 2 is a dead alias of 1: no third shape"
        );
    }

    #[test]
    fn ini_load_rewrites_note_tail_above_three_to_two() {
        let settings = OptionsSettings::load_text("[NOTE]\nNoteTail=4\n");
        assert_eq!(settings.note_tail(), 2);
        let settings = OptionsSettings::load_text("[NOTE]\nNoteTail=9\n");
        assert_eq!(settings.note_tail(), 2);
        let settings = OptionsSettings::load_text("[NOTE]\n");
        assert_eq!(settings.note_tail(), 2);
    }

    #[test]
    fn ini_load_rewrites_note_type_above_one_to_zero() {
        let settings = OptionsSettings::load_text("[NOTE]\nNoteType=2\n");
        assert_eq!(settings.note_type(), 0, "the exe folds 2 to 0 on load");
        let settings = OptionsSettings::load_text("[NOTE]\nNoteType=7\n");
        assert_eq!(settings.note_type(), 0);
        let settings = OptionsSettings::load_text("[NOTE]\nNoteType=1\n");
        assert_eq!(settings.note_type(), 1);
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn fifth_tail_value_lights_no_guide_button() {
        assert_eq!(guide_of_note_tail(0), Some(NoteGuide::NoUse));
        assert_eq!(guide_of_note_tail(3), Some(NoteGuide::Long));
        assert_eq!(
            guide_of_note_tail(4),
            None,
            "the fifth slot has no button: the rig names it, not the code"
        );
    }

    #[test]
    fn ini_load_rewrites_the_raw_defaults_of_mode_longnote_and_blend_half() {
        let settings = OptionsSettings::load_text("");
        assert_eq!(settings.mode(), 2, "raw Mode 0 is rewritten to 2");
        assert_eq!(settings.longnote_mode(), 1, "raw 3 is rewritten to 1");
        assert_eq!(settings.blend_half(), 1, "raw 3 is rewritten to 1");
        let settings = OptionsSettings::load_text(
            "[MODE]\nMode=0\n[LONGNOTEMODE]\nLongnoteMode=3\n[NoteScreen]\nBlendHalf=3\n",
        );
        assert_eq!(
            (
                settings.mode(),
                settings.longnote_mode(),
                settings.blend_half()
            ),
            (2, 1, 1)
        );
    }

    #[test]
    fn longnote_switch_counts_only_value_one_as_used() {
        let mut settings = OptionsSettings::load_text("[LONGNOTEMODE]\nLongnoteMode=0\n");
        assert!(!settings.longnote_used(), "raw 0 stays off");
        settings.set_longnote_mode(1);
        assert!(settings.longnote_used());
        settings.set_longnote_mode(0);
        assert!(!settings.longnote_used());
    }

    #[test]
    fn sound_volumes_default_to_full_on() {
        let settings = OptionsSettings::load_text("");
        assert_eq!(settings.bgm_music(), 255);
        assert_eq!(settings.bg_effect(), 255);
        assert_eq!(settings.game_music(), 255);
        assert_eq!(settings.game_effect(), 255);
        assert_eq!(settings.key_gain(), 1.0);
        assert_eq!(settings.bgm_gain(), 1.0);
    }

    #[test]
    fn first_sound_knob_moves_game_music_and_effect_words_together() {
        // #712 static pin: the first Sound knob's setter writes both
        // BGVolume and GameBGVolume (file `0x16bfd0`-era setter family,
        // getter `0x5517a0`, coupled setter `0x551710(..., 1)`).
        let mut settings = OptionsSettings::default();
        settings.step_bgm_music(false);
        assert_eq!(settings.bgm_music(), 255 - SOUND_SLIDER_STEP);
        assert_eq!(
            settings.bg_effect(),
            255 - SOUND_SLIDER_STEP,
            "the first knob moves the coupled word too"
        );
        assert_eq!(settings.game_music(), 255, "other words stay put");
        assert_eq!(settings.game_effect(), 255, "other words stay put");
    }

    #[test]
    fn slider_steps_stay_inside_the_byte_domain() {
        let mut settings = OptionsSettings::default();
        settings.step_game_effect(false);
        assert_eq!(settings.game_effect(), 255 - SOUND_SLIDER_STEP);
        for _ in 0..64 {
            settings.step_game_effect(false);
        }
        assert_eq!(settings.game_effect(), 0);
        assert_eq!(settings.key_gain(), 0.0);
        settings.step_game_effect(true);
        assert_eq!(settings.game_effect(), SOUND_SLIDER_STEP);
    }

    #[test]
    fn option_ini_round_trip_keeps_every_section() {
        let mut settings = OptionsSettings::default();
        assert!(settings.rebind_lane(0, KeyCode::KeyA));
        assert!(!settings.rebind_lane(7, KeyCode::KeyA));
        assert!(!settings.rebind_lane(1, KeyCode::F1));
        settings.step_bgm_music(false);
        settings.set_note_type(1);
        settings.set_note_tail(3);
        settings.set_longnote_mode(1);
        settings.set_mode(1);
        settings.set_blend_half(0);
        let text = settings.to_ini_text();
        let reloaded = OptionsSettings::load_text(&text);
        assert_eq!(reloaded, settings);
    }

    #[test]
    fn missing_option_ini_falls_back_to_the_exe_defaults() {
        let settings = OptionsSettings::load_text("[MODE]\nMode=abc\n");
        assert_eq!(settings.mode(), DEFAULT_MODE);
        assert_eq!(settings.lane_keys(), DEFAULT_LANE_KEYS);
        assert_eq!(settings.lane_dik(), DEFAULT_LANE_DIK);
    }

    #[test]
    fn unknown_dik_keeps_the_lane_default_but_preserves_the_raw() {
        let settings = OptionsSettings::load_text("[Key]\nKEY_1=222\n");
        assert_eq!(
            settings.lane_keys()[0],
            KeyCode::KeyS,
            "unmapped codes never break the lane"
        );
        assert_eq!(settings.lane_dik()[0], 222);
        let text = settings.to_ini_text();
        assert!(
            text.contains("KEY_1=222"),
            "raw codes round-trip instead of corrupting"
        );
    }

    #[test]
    fn profile_lookup_ignores_case_like_the_win32_api() {
        let settings = OptionsSettings::load_text("[sound]\ngamebgvolume=100\n");
        assert_eq!(settings.bgm_music(), 100);
    }

    #[test]
    fn dik_map_covers_the_seven_exe_defaults_both_ways() {
        for (lane, dik) in DEFAULT_LANE_DIK.iter().enumerate() {
            assert_eq!(keycode_of_dik(*dik), Some(DEFAULT_LANE_KEYS[lane]));
            assert_eq!(dik_of_keycode(DEFAULT_LANE_KEYS[lane]), Some(*dik));
        }
    }

    #[test]
    // seam: the two mode-vftable arms are the pinned contract; this client keeps no cursor vftable
    fn cursor_mode_switch_names_both_vftable_arms() {
        assert_eq!(cursor_mode_face(2, false), (2, ImageFaceBranch::ModeTwo));
        assert_eq!(cursor_mode_face(1, false), (1, ImageFaceBranch::ModeOne));
        assert_eq!(
            cursor_mode_face(1, true),
            (2, ImageFaceBranch::ModeOne),
            "a global-mode promotion stores 2 but keeps the mode-1 face"
        );
    }

    #[test]
    // seam: OS-cursor arm decision is the whole behavior until a live handle is installed; the pointer capture stays OPEN
    fn custom_cursor_arm_hides_or_selects_one_of_two_handles() {
        assert_eq!(
            custom_cursor_arm(MouseCursorMode::Windows, 1),
            CustomCursorArm::Hide
        );
        assert_eq!(
            custom_cursor_arm(MouseCursorMode::Image, 1),
            CustomCursorArm::FirstHandle
        );
        assert_eq!(
            custom_cursor_arm(MouseCursorMode::Image, 2),
            CustomCursorArm::SecondHandle
        );
        assert_eq!(
            custom_cursor_arm(MouseCursorMode::Image, 7),
            CustomCursorArm::Hide
        );
    }

    #[test]
    // seam: flag-reset contract has no live writer hook; the pointer leg stays OPEN
    fn custom_cursor_flag_change_resets_the_pointer_to_the_origin() {
        assert_eq!(
            pointer_reset_on_cursor_flag_change(
                MouseCursorMode::Windows,
                MouseCursorMode::Image,
                true
            ),
            Some((0, 0))
        );
        assert_eq!(
            pointer_reset_on_cursor_flag_change(
                MouseCursorMode::Image,
                MouseCursorMode::Image,
                true
            ),
            None,
            "an unchanged flag does not move the pointer"
        );
        assert_eq!(
            pointer_reset_on_cursor_flag_change(
                MouseCursorMode::Windows,
                MouseCursorMode::Image,
                false
            ),
            None,
            "no tracking object means no pointer to reset"
        );
    }

    #[test]
    // Live chain (Tier 0 logic; Tier 3 twin in
    // `live_room_input_legs.rs`): `main.rs::run_play` ->
    // `Shell::run` -> `WindowHandler::route_key_with_text` ->
    // `route_key_event` -> `options::switch_cursor_in_install_root`
    // -> `OptionsSettings::switch_mouse_cursor`. Both registry-value
    // branches are pinned: 0 -> 1 with the confirmation caption,
    // 1 -> 0 with the return-to-Windows caption.
    fn f8_switch_persists_both_mouse_cursor_branches() {
        let mut settings = OptionsSettings::load_text("[CURSOR]\nMouse_Cursor=0\n");
        assert_eq!(settings.mouse_cursor(), MouseCursorMode::Windows);
        assert_eq!(
            settings.switch_mouse_cursor(),
            CursorCaption::AskUseImageCursor,
            "a fresh boot asks before selecting the image pointer"
        );
        assert_eq!(settings.mouse_cursor(), MouseCursorMode::Image);
        let mut reloaded = OptionsSettings::load_text(&settings.to_ini_text());
        assert_eq!(
            reloaded.mouse_cursor(),
            MouseCursorMode::Image,
            "value persists"
        );
        assert_eq!(
            reloaded.switch_mouse_cursor(),
            CursorCaption::NoticeWindowsCursorRestored,
            "the second F8 returns to the Windows pointer"
        );
        assert_eq!(reloaded.mouse_cursor(), MouseCursorMode::Windows);
    }

    #[test]
    // Live chain (Tier 1): `WindowHandler::window_event` key-down ->
    // `route_key_event` -> `options::f8_cursor_request_allowed`.
    // The scene-id table is the exe's own gate (`0x5a50d0`): a song
    // scene and the pinned excluded ids can never reach the write.
    fn f8_gate_accepts_room_only_outside_the_excluded_scene_and_event_table() {
        assert!(f8_cursor_request_allowed(2, 0, F8_VIRTUAL_KEY), "Room");
        for excluded_scene in [6, 10, 11, 12, 0xff] {
            assert!(
                !f8_cursor_request_allowed(excluded_scene, 0, F8_VIRTUAL_KEY),
                "scene {excluded_scene} must not switch the cursor"
            );
        }
        for excluded_class in F8_EXCLUDED_EVENT_CLASSES {
            assert!(!f8_cursor_request_allowed(
                2,
                excluded_class,
                F8_VIRTUAL_KEY
            ));
        }
        assert!(!f8_cursor_request_allowed(2, 0, 0x77 + 1));
    }

    #[test]
    fn f8_scene_id_lookup_uses_only_pinned_scene_ids() {
        assert_eq!(f8_scene_id_of("room"), Some(2));
        assert_eq!(f8_scene_id_of("playing"), Some(11));
        assert_eq!(f8_scene_id_of("waiting"), None, "unpinned scene stays OPEN");
    }

    #[test]
    fn requested_cursor_mode_applies_to_the_face_and_pointer_arm() {
        // seam: the F8 work persists and toggles `Mouse_Cursor`,
        // but no winit cursor handle exists here. The face/arm/caption
        // decision is the whole pointer behavior Tier 4 can confirm.
        let applied = apply_cursor_mode(1, false, 1, MouseCursorMode::Windows);
        assert_eq!(applied.face, (1, ImageFaceBranch::ModeOne));
        assert_eq!(applied.arm, CustomCursorArm::FirstHandle);
        assert_eq!(applied.caption, CursorCaption::AskUseImageCursor);
        assert_eq!(
            apply_cursor_mode(1, true, 2, MouseCursorMode::Windows).face,
            (2, ImageFaceBranch::ModeOne),
            "global promotion changes the word, not the mode-1 arm"
        );
        assert_eq!(
            apply_cursor_mode(1, false, 2, MouseCursorMode::Windows).arm,
            CustomCursorArm::SecondHandle
        );
        assert_eq!(
            apply_cursor_mode(1, false, 7, MouseCursorMode::Windows).arm,
            CustomCursorArm::Hide,
            "an unmapped handle hides the OS cursor"
        );
        assert_eq!(
            apply_cursor_mode(0, false, 1, MouseCursorMode::Image).arm,
            CustomCursorArm::Hide,
            "the Windows mode clears the custom pointer"
        );
    }

    #[test]
    fn window_cursor_sprite_stages_only_on_the_first_use() {
        // Live chain (Tier 0 logic; Tier 3 twin in
        // `recomp2jam/src/live_formats_legs.rs`): `main.rs::run_play` ->
        // `stage_online_identity` -> first pointer use ->
        // `options::WindowCursorStage::stage_on_first_use`.
        // The exe sets its two first-use flags, loads the one sprite,
        // and stores one handle (`0x432090`, file `0x32090`).
        let mut stage = WindowCursorStage::default();
        let first = WindowCursorSurface::from_dimensions(32, 32).expect("32x32 fits");
        let later = WindowCursorSurface::from_dimensions(64, 64).expect("64x64 fits");

        assert_eq!(stage.load_attempts(), 0, "boot stages no pointer art");
        assert_eq!(stage.stage_on_first_use(Some(first)), Some(first));
        assert_eq!(
            stage.stage_on_first_use(Some(later)),
            Some(first),
            "a second pointer use reuses the stored surface"
        );
        assert_eq!(stage.load_attempts(), 1, "the load happens only once");
    }

    // 7.55 exe pin: `0x432090` / file `0x32090`, REA ev
    // 18ee.0400.f390.fbdd.1283.9b20.267f.6e89.cc3f.ecde.a490.da7b.db50.a7fa.57d6.7360.
    // seam: no OS handle exists headless; one attempt and its stored
    // surface are the whole first-use contract a live pointer capture
    // can confirm (Tier 4 is OPEN).
    #[test]
    fn window_cursor_missing_sprite_stays_unstaged_after_one_attempt() {
        let mut stage = WindowCursorStage::default();
        let later = WindowCursorSurface::from_dimensions(32, 32).expect("32x32 fits");

        assert_eq!(stage.stage_on_first_use(None), None);
        assert_eq!(stage.stage_on_first_use(Some(later)), None);
        assert_eq!(stage.load_attempts(), 1, "a miss is not retried");
    }

    #[test]
    fn window_cursor_surface_rejects_dimensions_wider_than_a_word() {
        assert!(WindowCursorSurface::from_dimensions(usize::from(u16::MAX) + 1, 1).is_none());
    }
}
