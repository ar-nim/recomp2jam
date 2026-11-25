//! Planet options-dialog state and clicks behind one seam (#139).
//!
//! The dialog owns three staged tab sheets, the open tab, the persisted
//! settings, the install root its SAVE writes beside, and the lane armed
//! for a Key-tab rebind. `ScreenGame` keeps flow and modal ownership;
//! this module answers which button a click hit and what it changes.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use super::super::paint::{dialog_rect_hit, paint_dialog_scene, SceneArt};
use super::super::side::{
    PlanetOptionTab, OPTION_GAME_BLEND_NO, OPTION_GAME_BLEND_YES, OPTION_GAME_EFFECT_HIGH,
    OPTION_GAME_EFFECT_LOW, OPTION_GAME_EFFECT_NORMAL, OPTION_GAME_GUIDE_LONG,
    OPTION_GAME_GUIDE_MIDDLE, OPTION_GAME_GUIDE_NOUSE, OPTION_GAME_GUIDE_SHORT, OPTION_GAME_KEYSET,
    OPTION_GAME_LONGNOTE_NONE, OPTION_GAME_LONGNOTE_USED, OPTION_GAME_MODE_FULL,
    OPTION_GAME_MODE_WIN, OPTION_GAME_NOTETYPE1, OPTION_GAME_NOTETYPE2, OPTION_GAME_SOUNDSET,
    OPTION_KEY_3KEY_FIRST, OPTION_KEY_3KEY_MODE, OPTION_KEY_7KEY_MODE, OPTION_KEY_GAMESET,
    OPTION_KEY_LANE_FIRST, OPTION_KEY_SOUNDSET, OPTION_KEY_TEST, OPTION_SOUND_BG_EFFECT_DOWN,
    OPTION_SOUND_BG_EFFECT_UP, OPTION_SOUND_BG_MUSIC_DOWN, OPTION_SOUND_BG_MUSIC_UP,
    OPTION_SOUND_GAMESET, OPTION_SOUND_GAME_EFFECT_DOWN, OPTION_SOUND_GAME_EFFECT_UP,
    OPTION_SOUND_GAME_MUSIC_DOWN, OPTION_SOUND_GAME_MUSIC_UP, OPTION_SOUND_KEYSET,
    PLANET_OPTION_FIRST_TAB,
};
use super::ui_effect::UiEffectId;
use super::{options_key_labels::is_rebindable_dik, options_key_mode};
use crate::avatar::{AvatarFigure, FigureLayer, ScenePlacement};
use crate::options::{dik_of_keycode, OptionsSettings};
use o2jam_render::{Frame, Rgb555Image};

/// What one options click asks the game to do: close the modal and/or
/// play one pinned effect. Keeping both in one result stops the handler
/// from guessing which branch produced the sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct OptionsClick {
    /// The EXIT (or 0x26200012 Sound apply) button was pressed.
    pub(super) close: bool,
    /// Effect cued by this branch, if the #718 map names one.
    pub(super) effect: Option<UiEffectId>,
}

/// The options modal as one state cluster: the tab sheets are keyed by
/// declaration order (Key, Sound, Game), and every click reads the open
/// tab's staged `.bnd` rects — never hardcoded geometry (owner rule
/// `f38f3`).
pub(super) struct OptionsDialog {
    art: [Option<SceneArt>; 3],
    tab: PlanetOptionTab,
    settings: OptionsSettings,
    root: Option<std::path::PathBuf>,
    rebind: Option<usize>,
}

impl OptionsDialog {
    /// Fresh dialog on the exe-pinned opening tab, with default settings
    /// and no staged sheets or armed lane.
    pub(super) fn new() -> Self {
        Self {
            art: [None, None, None],
            tab: PLANET_OPTION_FIRST_TAB,
            settings: OptionsSettings::default(),
            root: None,
            rebind: None,
        }
    }

    /// Stage the sheet the dialog opens on.
    pub(super) fn stage_first_tab(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) {
        self.stage_tab(PLANET_OPTION_FIRST_TAB, placements, figures);
    }

    /// Stage one options-dialog tab sheet. The dialog paints the
    /// open tab's sheet when staged; tab switches flip between
    /// staged sheets, unstaged tabs keep the fallback title.
    /// Staging never moves the flow.
    pub(super) fn stage_tab(
        &mut self,
        tab: PlanetOptionTab,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) {
        self.art[Self::tab_slot(tab)] = Some(SceneArt {
            placements,
            figures,
        });
    }

    /// Storage slot for one options tab (declaration order: Key,
    /// Sound, Game).
    const fn tab_slot(tab: PlanetOptionTab) -> usize {
        match tab {
            PlanetOptionTab::Key => 0,
            PlanetOptionTab::Sound => 1,
            PlanetOptionTab::Game => 2,
        }
    }

    /// Open options tab (the dialog paints this tab's staged sheet).
    pub(super) const fn tab(&self) -> PlanetOptionTab {
        self.tab
    }

    /// Sheet staged for the open tab, if any.
    pub(super) fn staged_art(&self) -> Option<&SceneArt> {
        self.art[Self::tab_slot(self.tab)].as_ref()
    }

    /// Switch the open options tab (the `KEYSET`/`SOUNDSET`/
    /// `GAMESET` buttons): flips the painted sheet, keeps the
    /// dialog open, and disarms any pending rebind.
    pub(super) fn switch_tab(&mut self, tab: PlanetOptionTab) {
        self.tab = tab;
        self.rebind = None;
    }

    /// Options-dialog values (every tab's settings).
    pub(super) const fn settings(&self) -> &OptionsSettings {
        &self.settings
    }

    /// Mutable settings, for the session's boot-time profile edits and
    /// tests that stage a value before driving the dialog.
    pub(super) fn settings_mut(&mut self) -> &mut OptionsSettings {
        &mut self.settings
    }

    /// Lane armed for rebind by a Key-tab click (`None` idle).
    pub(super) const fn rebind_armed(&self) -> Option<usize> {
        self.rebind
    }

    /// Forget an armed rebind after a tab switch, EXIT, or Escape.
    pub(super) fn clear_rebind(&mut self) {
        self.rebind = None;
    }

    /// Write a mapped key into the active mode's bind table, then
    /// advance the arm like the exe rebind handlers `0x438e80` /
    /// `0x4395e0`: the DIK lands at `+0x18a + armed`, and an unmapped
    /// key or invalid arm leaves both the table and the arm alone.
    pub(super) fn rebind_armed_lane(
        &mut self,
        lane: usize,
        code: winit::keyboard::KeyCode,
    ) -> bool {
        let is_three_key = self.settings.three_key();
        if self.rebind != Some(lane) || !options_key_mode::is_armable_lane(lane, is_three_key) {
            return false;
        }
        let Some(dik) = dik_of_keycode(code).filter(|dik| is_rebindable_dik(*dik)) else {
            return false;
        };
        let written = if is_three_key {
            self.settings.set_three_key_dik(lane, dik)
        } else {
            self.settings.rebind_lane(lane, code)
        };
        if written {
            self.rebind = Some(options_key_mode::next_armed_lane(lane, is_three_key));
        }
        written
    }

    /// Load `option.ini` beside `install_root` at boot and remember
    /// the root for SAVE commits: a missing or unreadable file
    /// keeps the exe defaults (misses warn, never fail the scene).
    pub(super) fn load(&mut self, install_root: &Path) {
        self.settings = OptionsSettings::load_from_dir(install_root);
        self.root = Some(install_root.to_owned());
    }

    /// Persist the settings beside the remembered install root (the
    /// tab SAVE commit path). Without a root the settings stay in
    /// memory with a warning — the scene never fails. Exe validators
    /// `0x439170` / `0x4394c0` reject zero or duplicate active binds
    /// before any write; their message text stays OPEN.
    ///
    /// # Errors
    ///
    /// `io::Error` when the binds are invalid or the profile write
    /// or rename fails.
    pub(super) fn save(&self) -> io::Result<()> {
        let Some(root) = self.root.as_ref() else {
            tracing::warn!("options SAVE without an install root; settings stay in memory");
            return Ok(());
        };
        if !self.active_binds_are_valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "key binds must be nonzero and distinct",
            ));
        }
        self.settings.save_to_dir(root)
    }

    /// Exe SAVE rule: each active bind is nonzero and no two active
    /// binds match. Inactive 3K index 1 is ignored by `0x4394c0`.
    fn active_binds_are_valid(&self) -> bool {
        let is_three_key = self.settings.three_key();
        let binds = self.settings.lane_dik_for_mode(is_three_key);
        let active = options_key_mode::active_lane_indexes(is_three_key);
        active.iter().enumerate().all(|(position, &lane)| {
            binds[lane] != 0
                && !active[..position]
                    .iter()
                    .any(|&earlier| binds[earlier] == binds[lane])
        })
    }

    /// Staged hit for one options-dialog code on the open tab
    /// (`false` while the tab's sheet is unstaged — unstaged
    /// dialogs answer no button, Escape still closes them). The Key
    /// tab hides the inactive lane family through the same filter
    /// the painter uses, so hidden buttons cannot be armed.
    fn hit(&self, code: u32, x: i32, y: i32) -> bool {
        if self.tab == PlanetOptionTab::Key
            && !options_key_mode::is_visible(code, self.settings.three_key())
        {
            return false;
        }
        self.staged_art()
            .filter(|art| art.has_figures())
            .is_some_and(|art| dialog_rect_hit(&art.placements, code, x, y))
    }

    /// One click inside the open options tab: EXIT/SAVE/DEFAULT,
    /// tab switches, then the tab's value buttons. SAVE persists
    /// beside the install root and stays open; EXIT closes without
    /// saving; a failed write warns and stays open (never fails
    /// the scene). The #718 map cues only the Sound tab's `0x26200012`
    /// apply arm (id 2) and tab switches (id 9): DEFAULT (`0x26200010`),
    /// SAVE (`0x26200011`) and the other tabs' exits stay silent.
    pub(super) fn click(&mut self, x: i32, y: i32) -> OptionsClick {
        let tab = self.tab;
        if self.hit(tab.exit_code(), x, y) {
            self.clear_rebind();
            return OptionsClick {
                close: true,
                effect: (tab == PlanetOptionTab::Sound).then_some(UiEffectId::SliderApply),
            };
        }
        if self.hit(tab.save_code(), x, y) {
            if let Err(source) = self.save() {
                tracing::warn!(%source, "options SAVE failed; settings stay in memory");
            }
            self.clear_rebind();
            return OptionsClick::default();
        }
        if self.hit(tab.default_code(), x, y) {
            match tab {
                PlanetOptionTab::Key => self.settings.reset_binds(),
                PlanetOptionTab::Sound => self.settings.reset_volumes(),
                PlanetOptionTab::Game => self.settings.reset_game(),
            }
            self.clear_rebind();
            return OptionsClick::default();
        }
        // Pinned #314 P4: while key test is checked, rebind cells and
        // sheet tabs refuse the click instead of acting. The exe
        // answers with an error cue; its id stays OPEN (next step:
        // capture one key-test-gated click's cue on the Wine rig).
        if self.settings.key_test() && self.key_test_gated_click(x, y) {
            return OptionsClick::default();
        }
        for code in [
            OPTION_KEY_SOUNDSET,
            OPTION_KEY_GAMESET,
            OPTION_SOUND_KEYSET,
            OPTION_SOUND_GAMESET,
            OPTION_GAME_KEYSET,
            OPTION_GAME_SOUNDSET,
        ] {
            if self.hit(code, x, y) {
                if let Some(next) = tab.switch_tab(code) {
                    self.switch_tab(next);
                    return OptionsClick {
                        effect: Some(UiEffectId::OptionTabSwitch),
                        ..OptionsClick::default()
                    };
                }
                return OptionsClick::default();
            }
        }
        match tab {
            PlanetOptionTab::Key => self.click_key_tab(x, y),
            PlanetOptionTab::Sound => self.click_sound_tab(x, y),
            PlanetOptionTab::Game => self.click_game_tab(x, y),
        }
        OptionsClick::default()
    }

    /// Whether a point hits one of the controls the key-test flag
    /// holds back: rebind cells, their mode selectors, and sheet tabs.
    /// The checkbox itself is never gated, so the player can uncheck it.
    fn key_test_gated_click(&self, x: i32, y: i32) -> bool {
        if [
            OPTION_KEY_SOUNDSET,
            OPTION_KEY_GAMESET,
            OPTION_SOUND_KEYSET,
            OPTION_SOUND_GAMESET,
            OPTION_GAME_KEYSET,
            OPTION_GAME_SOUNDSET,
            OPTION_KEY_3KEY_MODE,
            OPTION_KEY_7KEY_MODE,
        ]
        .iter()
        .any(|code| self.hit(*code, x, y))
        {
            return true;
        }
        (0..7u32).any(|lane| self.hit(OPTION_KEY_LANE_FIRST + lane, x, y))
            || (0..4u32).any(|lane| {
                let armable = usize::try_from(lane)
                    .is_ok_and(|index| options_key_mode::is_armable_lane(index, true));
                armable && self.hit(OPTION_KEY_3KEY_FIRST + lane, x, y)
            })
    }

    /// Key-tab value buttons: `KEY1..KEY7` arm their lane for
    /// rebind (the next mapped key press sticks), `3KEY_1..4` arm
    /// the kept 3K lanes (index 1 is skipped at its duplicate rect),
    /// `3KEY`/`7KEY` select the key-count family, and `KEYTEST`
    /// flips its checkbox (selection-only, OPEN).
    fn click_key_tab(&mut self, x: i32, y: i32) {
        for lane in 0..7u32 {
            if self.hit(OPTION_KEY_LANE_FIRST + lane, x, y) {
                self.rebind = usize::try_from(lane).ok();
                return;
            }
        }
        for lane in 0..4u32 {
            let armable = usize::try_from(lane)
                .is_ok_and(|index| options_key_mode::is_armable_lane(index, true));
            if armable && self.hit(OPTION_KEY_3KEY_FIRST + lane, x, y) {
                self.rebind = usize::try_from(lane).ok();
                return;
            }
        }
        if self.hit(OPTION_KEY_3KEY_MODE, x, y) {
            self.settings.set_three_key(true);
        } else if self.hit(OPTION_KEY_7KEY_MODE, x, y) {
            self.settings.set_three_key(false);
        } else if self.hit(OPTION_KEY_TEST, x, y) {
            let tested = self.settings.key_test();
            self.settings.set_key_test(!tested);
        }
    }

    /// Sound-tab slider buttons: each Down/Up pair steps its volume
    /// by the exe-pinned 4 inside the 0–255 domain (which slider writes
    /// which `Sound` key stays ours-by-design, OPEN).
    fn click_sound_tab(&mut self, x: i32, y: i32) {
        if self.hit(OPTION_SOUND_BG_MUSIC_DOWN, x, y) {
            self.settings.step_bgm_music(false);
        } else if self.hit(OPTION_SOUND_BG_MUSIC_UP, x, y) {
            self.settings.step_bgm_music(true);
        } else if self.hit(OPTION_SOUND_BG_EFFECT_DOWN, x, y) {
            self.settings.step_bg_effect(false);
        } else if self.hit(OPTION_SOUND_BG_EFFECT_UP, x, y) {
            self.settings.step_bg_effect(true);
        } else if self.hit(OPTION_SOUND_GAME_MUSIC_DOWN, x, y) {
            self.settings.step_game_music(false);
        } else if self.hit(OPTION_SOUND_GAME_MUSIC_UP, x, y) {
            self.settings.step_game_music(true);
        } else if self.hit(OPTION_SOUND_GAME_EFFECT_DOWN, x, y) {
            self.settings.step_game_effect(false);
        } else if self.hit(OPTION_SOUND_GAME_EFFECT_UP, x, y) {
            self.settings.step_game_effect(true);
        }
    }

    /// Game-tab buttons, pinned by the exe's repaint and click arms
    /// (`0x3a7c0`/`0x39f40`): effect arms (in-memory — no ini key
    /// was found), EQ Yes/No → 1/0, NoteType1/2 → 0/1, guides →
    /// 3/2/1/0, ModeFull/Win → 2/1, Longnote Used/None → 1/0.
    fn click_game_tab(&mut self, x: i32, y: i32) {
        if self.hit(OPTION_GAME_EFFECT_HIGH, x, y) {
            self.settings.set_effect_level(0);
        } else if self.hit(OPTION_GAME_EFFECT_NORMAL, x, y) {
            self.settings.set_effect_level(1);
        } else if self.hit(OPTION_GAME_EFFECT_LOW, x, y) {
            self.settings.set_effect_level(2);
        } else if self.hit(OPTION_GAME_BLEND_YES, x, y) {
            self.settings.set_blend_half(1);
        } else if self.hit(OPTION_GAME_BLEND_NO, x, y) {
            self.settings.set_blend_half(0);
        } else if self.hit(OPTION_GAME_NOTETYPE1, x, y) {
            self.settings.set_note_type(0);
        } else if self.hit(OPTION_GAME_NOTETYPE2, x, y) {
            self.settings.set_note_type(1);
        } else if self.hit(OPTION_GAME_GUIDE_LONG, x, y) {
            self.settings.set_note_tail(3);
        } else if self.hit(OPTION_GAME_GUIDE_MIDDLE, x, y) {
            self.settings.set_note_tail(2);
        } else if self.hit(OPTION_GAME_GUIDE_SHORT, x, y) {
            self.settings.set_note_tail(1);
        } else if self.hit(OPTION_GAME_GUIDE_NOUSE, x, y) {
            self.settings.set_note_tail(0);
        } else if self.hit(OPTION_GAME_MODE_FULL, x, y) {
            self.settings.set_mode(2);
        } else if self.hit(OPTION_GAME_MODE_WIN, x, y) {
            self.settings.set_mode(1);
        } else if self.hit(OPTION_GAME_LONGNOTE_USED, x, y) {
            self.settings.set_longnote_mode(1);
        } else if self.hit(OPTION_GAME_LONGNOTE_NONE, x, y) {
            self.settings.set_longnote_mode(0);
        }
    }
}

/// Staged knob controls in exe order (`0x26500013..16`, `0x43b690`,
/// file `0x3b690`, evidence `ev_aefab8e8…`).
const SOUND_KNOB_CODES: [u32; 4] = [0x2650_0013, 0x2650_0014, 0x2650_0015, 0x2650_0016];

/// Exe divisors for those knobs: 255, 255, 255, 204 (NoteKey).
const SOUND_KNOB_DIVISORS: [usize; 4] = [255, 255, 255, 204];

/// Right-cap fill of one staged bar, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KnobFill {
    /// Bar-art columns still visible after the cap moves left.
    visible_art_width: usize,
    /// Exe's `trunc(volume / divisor * track_width)`.
    filled: usize,
    /// Unfilled columns of the staged track.
    remaining: usize,
}

/// Compute one knob's fill from the staged `.bnd` track width and its
/// volume word. Integer division is the exe's float-to-int trunc of the
/// same ratio; the ratio is clamped to the track so the NoteKey
/// divisor 204 cannot paint past the staged art.
fn knob_fill(volume: u8, divisor: usize, track_width: usize, art_width: usize) -> KnobFill {
    let filled = usize::from(volume)
        .saturating_mul(track_width)
        .checked_div(divisor)
        .unwrap_or(0)
        .min(track_width);
    let remaining = track_width - filled;
    KnobFill {
        visible_art_width: if volume == 0 {
            0
        } else {
            art_width.saturating_sub(remaining)
        },
        filled,
        remaining,
    }
}

/// Keep the left `visible_width` columns of a staged bar record. The
/// exe clips the static bar art at the right cap instead of moving a
/// record (`docs/captures/t713/findings.md`, `Option_Sound_BGMusicBar.ojs`
/// rec0 at `(283,179)` and `Option_Sound_GameMusicBar.ojs` rec0 at
/// `(283,360)`).
fn crop_bar_figure(figure: &AvatarFigure, visible_width: usize) -> Option<AvatarFigure> {
    let layer = figure.layer(0)?;
    let width = visible_width.min(layer.image.w);
    let mut words = Vec::with_capacity(width.saturating_mul(layer.image.h));
    for row in 0..layer.image.h {
        let start = row.checked_mul(layer.image.w)?;
        let end = start.checked_add(width)?;
        words.extend_from_slice(layer.image.words.get(start..end)?);
    }
    let mut cropped = AvatarFigure::default();
    cropped.push_layer(FigureLayer {
        image: Rgb555Image {
            w: width,
            h: layer.image.h,
            words,
        },
        dx: layer.dx,
        dy: layer.dy,
        colorkey: layer.colorkey,
    });
    Some(cropped)
}

/// Volume word behind each staged knob (the shell's ours-by-design
/// slider↔key map). The exe's first-knob BGVolume/GameBGVolume
/// coupling is applied in `OptionsSettings::step_bgm_music`; which
/// caption owns that knob stays rig-OPEN.
fn sound_knob_volumes(settings: &OptionsSettings) -> [u8; 4] {
    [
        settings.bgm_music(),
        settings.bg_effect(),
        settings.game_music(),
        settings.game_effect(),
    ]
}

/// Sound-sheet figures with each staged bar clipped to its volume.
fn sound_bar_figures(art: &SceneArt, settings: &OptionsSettings) -> HashMap<String, AvatarFigure> {
    let mut figures = art.figures.clone();
    for ((code, divisor), volume) in SOUND_KNOB_CODES
        .iter()
        .zip(SOUND_KNOB_DIVISORS)
        .zip(sound_knob_volumes(settings))
    {
        let Some(placement) = art.placements.iter().find(|placed| placed.code == *code) else {
            continue;
        };
        let Some(sprite) = placement.sprite.as_deref() else {
            continue;
        };
        let Some(figure) = art.figures.get(sprite) else {
            continue;
        };
        let track_width = usize::try_from(placement.rect.w).unwrap_or_default();
        let Some(layer) = figure.layer(0).filter(|_| track_width > 0) else {
            continue;
        };
        let fill = knob_fill(volume, divisor, track_width, layer.image.w);
        if fill.visible_art_width == layer.image.w {
            continue;
        }
        if let Some(cropped) = crop_bar_figure(figure, fill.visible_art_width) {
            figures.insert(sprite.to_owned(), cropped);
        }
    }
    figures
}

/// Paint the staged Sound sheet with each bar's right cap clipped to
/// its volume word, in the same back-to-front pass as any other dialog.
/// Both directions come from the words, never from click state.
pub(super) fn paint_sound_tab_with_knobs(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    settings: &OptionsSettings,
) {
    paint_dialog_scene(
        frame,
        &SceneArt {
            placements: art.placements.clone(),
            figures: sound_bar_figures(art, settings),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knob_fill_truncates_the_exe_ratio_from_the_staged_track_width() {
        // 229 px is the staged BGBAR rect (`Dialog_Option_Sound.bnd`
        // record 19); 239 px is the BGMusicBar rec0 art width (#713).
        let full = knob_fill(255, 255, 229, 239);
        assert_eq!(
            full,
            KnobFill {
                visible_art_width: 239,
                filled: 229,
                remaining: 0
            }
        );
        let down = knob_fill(251, 255, 229, 239);
        assert_eq!(
            down,
            KnobFill {
                visible_art_width: 235,
                filled: 225,
                remaining: 4
            }
        );
        let empty = knob_fill(0, 255, 229, 239);
        assert_eq!(empty.visible_art_width, 0, "volume 0 hides the bar");
    }

    #[test]
    fn note_key_knob_uses_divisor_204_and_saturates_at_the_track_end() {
        assert_eq!(knob_fill(255, 204, 229, 239).remaining, 0);
        let lowered = knob_fill(200, 204, 229, 239);
        assert_eq!(lowered.filled, 224);
        assert_eq!(lowered.remaining, 5);
        assert_eq!(lowered.visible_art_width, 234);
    }
}
