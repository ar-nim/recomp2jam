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
//!   `0x1F/0x20/0x21/0x39/0x24/0x25/0x26` (S D F Space J K L),
//!   stored at key-table `+0x345..+0x34b` (claims B37).
//! - `Sound`/`GameBGVolume` ← `option.ini`, default 255, stored at
//!   object `+0x68` (file `0x14bb45`); written back at file
//!   `0x14badb` through the same section/key/file names.
//! - `Sound`/`NoteKeyVolume` ← `option.ini`, default 255, stored at
//!   object `+0x36164` (file `0x151587`); written at `0x151475`.
//! - `Sound`/`NoteBGVolume` ← `option.ini`, default 255, stored at
//!   object `+0x36168` (file `0x1516d7`).
//! - `Sound`/`BGVolume` ← `option.ini`, default 255, stored at
//!   object `+0x3616c` (file `0x1517a7`); written at `0x151745`.
//! - `MODE`/`Mode` ← `option.ini`, default 0, stored at object
//!   `+0xa34` (file `0x1a46e0`); the `1`/`2` arms drive the window
//!   apply at file `0x3a06c` (which is Full vs Win stays OPEN).
//! - `NoteScreen`/`BlendHalf` ← `option.ini`, default 3, stored at
//!   object `+0xa3c` (file `0x1a4798`).
//! - `NOTE`/`NoteType` ← `option.ini`, default 0, clamped 0–2
//!   (else 0) into play-object `+0xeb4` (file `0x1713c7`); value 2
//!   is a dead alias of 1 (ticket #18 static hunt: zero-vs-nonzero
//!   branch file `0x171480`, `cmp $0x2` folds at `0xE1C22`-family).
//! - `NOTE`/`NoteTail` ← `option.ini`, default 2, clamped 0–4
//!   (else 2) into the global at `0x67feac` (file `0x16bf47`),
//!   indexing a five-slot double table at `0x67fe8c`.
//! - `LONGNOTEMODE`/`LongnoteMode` ← `Option.ini`, default 3,
//!   snapshotted into the play object at `+0x17d8` at song setup
//!   (file `0x16fb06`); the one consumer tests equality with 1
//!   (file `0x16E372`).
//!
//! Ours-by-design (never exe proof) and OPEN:
//!
//! - Slider↔key mapping on the Sound tab, the slider step (8 here),
//!   and the live BGM-gain application: the exe pins the four keys
//!   above, not which slider writes which key. Next step: rig
//!   SAVE-diff (move one slider, SAVE, diff `option.ini`).
//! - Button→value order on the Game tab (NoteType1→0 vs 1, guide
//!   Long/Middle/Short/NoUse→3/2/1/0, ModeFull/Win→1/2,
//!   BlendHalf Yes/No→1/0, Longnote Used/None→1/0): the dialog
//!   offers exactly two note-type buttons (no third shape — the
//!   dead alias holds in the UI too) and four guide buttons for
//!   five tail values, so tail value 4 lights no button. Next step:
//!   the ticket's exact rig list (one 0-vs-1 shape capture, five
//!   tail captures, one mode 0-vs-1 A/B plus the
//!   timing-vs-rendering call) — until then no arm is named
//!   oval/square and no tail value is named beyond its button.
//! - `Option_Game_EffectHigh/Normal/Low` has no ini key anywhere in
//!   the sweep: an in-memory selection, never persisted. Next step:
//!   SAVE-diff rig to find its key, if any.
//! - `MODE`/`Mode` value 2 (written by the exe's own string path
//!   when the numeric value is 0) and `BlendHalf` raw value 3 keep
//!   their exe-mirror load behavior; the effective defaults used
//!   here (Mode 0, BlendHalf 1) follow the exe's forced values.

use std::path::Path;
use winit::keyboard::KeyCode;

/// Seven play lanes, left to right (mirrors the shell lane count
/// without depending on it).
pub const OPTION_LANES: usize = 7;

/// Exe DirectInput defaults for `KEY_1..KEY_7` (claims B37):
/// S D F Space J K L.
pub const DEFAULT_LANE_DIK: [u8; OPTION_LANES] = [0x1F, 0x20, 0x21, 0x39, 0x24, 0x25, 0x26];

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

/// Ours-by-design slider step (0–255 domain): the exe pins the
/// bounds and defaults, not the step. Next step: rig SAVE-diff.
pub const SOUND_SLIDER_STEP: u8 = 8;

/// Exe default `NOTE`/`NoteType` (file `0x1713c7`).
pub const DEFAULT_NOTE_TYPE: u8 = 0;

/// Exe default `NOTE`/`NoteTail` (file `0x16bf47`).
pub const DEFAULT_NOTE_TAIL: u8 = 2;

/// Effective default `NoteScreen`/`BlendHalf`: the exe reads raw
/// default 3 (file `0x1a4798`) and its string path forces 1, so a
/// fresh client behaves as 1.
pub const DEFAULT_BLEND_HALF: i32 = 1;

/// Exe default `MODE`/`Mode` (file `0x1a46e0`).
pub const DEFAULT_MODE: i32 = 0;

/// Exe default `LONGNOTEMODE`/`LongnoteMode` (file `0x16fb06`):
/// raw 3, whose only consumer tests equality with 1 — so a fresh
/// client behaves as off.
pub const DEFAULT_LONGNOTE_MODE: i32 = 3;

/// `option.ini` file name beside the install root (lifecycle §3).
/// The exe reads `option.ini` for most keys and `Option.ini` for
/// `LongnoteMode`; Win32 folds the case, so both names address one
/// file on Windows and the loader tries both on disk.
pub const OPTION_FILE_NAME: &str = "option.ini";
/// Capital-O spelling the exe uses for the `LongnoteMode` read.
pub const OPTION_FILE_NAME_CAPITAL: &str = "Option.ini";

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
/// The button→tail-value order is ours-by-design (OPEN, see the
/// module docs); tail value 4 — the fifth table slot — lights no
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
/// [`guide_of_note_tail`]; ours-by-design order).
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

/// The options dialog values: every tab's settings plus their
/// `option.ini` form. Load with [`Self::load_from_dir`] at boot
/// (missing file degrades to the exe defaults, never an error);
/// persist with [`Self::save_to_dir`] on the tab SAVE commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsSettings {
    /// Raw `Key`/`KEY_1..KEY_7` codes, persisted verbatim so an
    /// unmapped code round-trips instead of corrupting.
    lane_dik: [u8; OPTION_LANES],
    /// Resolved lane keys answering in the lanes.
    lane_keys: [KeyCode; OPTION_LANES],
    /// `Sound`/`GameBGVolume` (BGMusic slider, ours-by-design map).
    bgm_music: u8,
    /// `Sound`/`BGVolume` (BGEffect slider, ours-by-design map).
    bg_effect: u8,
    /// `Sound`/`NoteBGVolume` (GameMusic slider, ours-by-design).
    game_music: u8,
    /// `Sound`/`NoteKeyVolume` (GameEffect slider, ours-by-design).
    game_effect: u8,
    /// `MODE`/`Mode` (ModeFull/Win buttons, order ours-by-design).
    mode: i32,
    /// `NoteScreen`/`BlendHalf` (EQ Yes/No buttons → 1/0).
    blend_half: i32,
    /// `NOTE`/`NoteType`, clamped 0–2 like the exe.
    note_type: u8,
    /// `NOTE`/`NoteTail`, clamped 0–4 like the exe.
    note_tail: u8,
    /// `LONGNOTEMODE`/`LongnoteMode` raw value (Used→1, None→0;
    /// the one consumer tests equality with 1).
    longnote_mode: i32,
    /// `Option_Game_EffectHigh/Normal/Low` selection (0/1/2):
    /// in-memory only — no ini key was found, never persisted.
    effect_level: u8,
    /// 3-key vs 7-key key-mode buttons: selection state only, the
    /// lane-count behavior stays OPEN.
    three_key: bool,
    /// Key-test checkbox: selection state only, no behavior yet.
    key_test: bool,
}

impl Default for OptionsSettings {
    fn default() -> Self {
        Self {
            lane_dik: DEFAULT_LANE_DIK,
            lane_keys: DEFAULT_LANE_KEYS,
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
            three_key: false,
            key_test: false,
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
        settings.bgm_music = clamp_u8_setting(profile_int(
            &sections,
            "Sound",
            "GameBGVolume",
            i32::from(DEFAULT_SOUND_VOLUME),
        ));
        settings.bg_effect = clamp_u8_setting(profile_int(
            &sections,
            "Sound",
            "BGVolume",
            i32::from(DEFAULT_SOUND_VOLUME),
        ));
        settings.game_music = clamp_u8_setting(profile_int(
            &sections,
            "Sound",
            "NoteBGVolume",
            i32::from(DEFAULT_SOUND_VOLUME),
        ));
        settings.game_effect = clamp_u8_setting(profile_int(
            &sections,
            "Sound",
            "NoteKeyVolume",
            i32::from(DEFAULT_SOUND_VOLUME),
        ));
        settings.mode = profile_int(&sections, "MODE", "Mode", DEFAULT_MODE);
        settings.blend_half = profile_int(&sections, "NoteScreen", "BlendHalf", DEFAULT_BLEND_HALF);
        let note_type = profile_int(&sections, "NOTE", "NoteType", i32::from(DEFAULT_NOTE_TYPE));
        settings.note_type = if (0..=2).contains(&note_type) {
            u8::try_from(note_type).unwrap_or(DEFAULT_NOTE_TYPE)
        } else {
            DEFAULT_NOTE_TYPE
        };
        let note_tail = profile_int(&sections, "NOTE", "NoteTail", i32::from(DEFAULT_NOTE_TAIL));
        settings.note_tail = if (0..=4).contains(&note_tail) {
            u8::try_from(note_tail).unwrap_or(DEFAULT_NOTE_TAIL)
        } else {
            DEFAULT_NOTE_TAIL
        };
        settings.longnote_mode = profile_int(
            &sections,
            "LONGNOTEMODE",
            "LongnoteMode",
            DEFAULT_LONGNOTE_MODE,
        );
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

    /// Step one Sound volume by one slider step up or down,
    /// clamped to the 0–255 domain (the Down/Up buttons).
    pub fn step_bgm_music(&mut self, up: bool) {
        self.bgm_music = step_volume(self.bgm_music, up);
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

    /// `NOTE`/`NoteType` (always 0–2: clamped on load and set).
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

    /// Set `NoteType` with the exe clamp (0–2, else 0).
    pub fn set_note_type(&mut self, note_type: i32) {
        self.note_type = if (0..=2).contains(&note_type) {
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

    /// Set `MODE`/`Mode` raw value (Full/Win button order is
    /// ours-by-design).
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

    /// Restore the exe Game-tab defaults (the Game-tab DEFAULT
    /// button): `NoteType` 0, `NoteTail` 2, `BlendHalf` 1
    /// (effective), `Mode` 0, long-note off. In-memory selections
    /// return to Normal / 7-key / untested.
    pub fn reset_game(&mut self) {
        self.note_type = DEFAULT_NOTE_TYPE;
        self.note_tail = DEFAULT_NOTE_TAIL;
        self.blend_half = DEFAULT_BLEND_HALF;
        self.mode = DEFAULT_MODE;
        self.longnote_mode = 0;
        self.effect_level = 1;
        self.three_key = false;
        self.key_test = false;
    }
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
    fn note_tail_clamps_to_five_slots_defaulting_to_two() {
        let settings = OptionsSettings::load_text("[NOTE]\nNoteTail=4\n");
        assert_eq!(settings.note_tail(), 4);
        let settings = OptionsSettings::load_text("[NOTE]\nNoteTail=9\n");
        assert_eq!(
            settings.note_tail(),
            2,
            "out of range falls back to the exe default, like file 0x16bf47"
        );
        let settings = OptionsSettings::load_text("[NOTE]\n");
        assert_eq!(settings.note_tail(), 2);
    }

    #[test]
    fn note_type_clamps_to_three_values_defaulting_to_zero() {
        let settings = OptionsSettings::load_text("[NOTE]\nNoteType=2\n");
        assert_eq!(settings.note_type(), 2);
        assert_eq!(settings.shape_arm(), NoteShapeArm::ArmOneTwo);
        let settings = OptionsSettings::load_text("[NOTE]\nNoteType=7\n");
        assert_eq!(settings.note_type(), 0);
    }

    #[test]
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
    fn longnote_switch_counts_only_value_one_as_used() {
        let mut settings = OptionsSettings::default();
        assert!(!settings.longnote_used(), "fresh default 3 behaves as off");
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
        settings.set_note_tail(4);
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
}
