//! Planet options-dialog state and clicks behind one seam (#139).
//!
//! The dialog owns three staged tab sheets, the open tab, the persisted
//! settings, the install root its SAVE writes beside, and the lane armed
//! for a Key-tab rebind. `ScreenGame` keeps flow and modal ownership;
//! this module answers which button a click hit and what it changes.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use super::super::paint::{dialog_rect_hit, SceneArt};
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
use crate::avatar::{AvatarFigure, ScenePlacement};
use crate::options::OptionsSettings;

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

    /// Bind a mapped key to the armed lane and disarm it.
    pub(super) fn rebind_armed_lane(
        &mut self,
        lane: usize,
        code: winit::keyboard::KeyCode,
    ) -> bool {
        if self.settings.rebind_lane(lane, code) {
            self.rebind = None;
            true
        } else {
            false
        }
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
    /// memory with a warning — the scene never fails.
    ///
    /// # Errors
    ///
    /// `io::Error` when the profile write or rename fails.
    pub(super) fn save(&self) -> io::Result<()> {
        let Some(root) = self.root.as_ref() else {
            tracing::warn!("options SAVE without an install root; settings stay in memory");
            return Ok(());
        };
        self.settings.save_to_dir(root)
    }

    /// Staged hit for one options-dialog code on the open tab
    /// (`false` while the tab's sheet is unstaged — unstaged
    /// dialogs answer no button, Escape still closes them).
    fn hit(&self, code: u32, x: i32, y: i32) -> bool {
        self.staged_art()
            .filter(|art| art.has_figures())
            .is_some_and(|art| dialog_rect_hit(&art.placements, code, x, y))
    }

    /// One click inside the open options tab: EXIT/SAVE/DEFAULT,
    /// tab switches, then the tab's value buttons. SAVE persists
    /// beside the install root and stays open; EXIT closes without
    /// saving; a failed write warns and stays open (never fails
    /// the scene). Returns `true` only for EXIT, which asks the
    /// game to close the modal it owns.
    pub(super) fn click(&mut self, x: i32, y: i32) -> bool {
        let tab = self.tab;
        if self.hit(tab.exit_code(), x, y) {
            self.clear_rebind();
            return true;
        }
        if self.hit(tab.save_code(), x, y) {
            if let Err(source) = self.save() {
                tracing::warn!(%source, "options SAVE failed; settings stay in memory");
            }
            self.clear_rebind();
            return false;
        }
        if self.hit(tab.default_code(), x, y) {
            match tab {
                PlanetOptionTab::Key => self.settings.reset_binds(),
                PlanetOptionTab::Sound => self.settings.reset_volumes(),
                PlanetOptionTab::Game => self.settings.reset_game(),
            }
            self.clear_rebind();
            return false;
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
                }
                return false;
            }
        }
        match tab {
            PlanetOptionTab::Key => self.click_key_tab(x, y),
            PlanetOptionTab::Sound => self.click_sound_tab(x, y),
            PlanetOptionTab::Game => self.click_game_tab(x, y),
        }
        false
    }

    /// Key-tab value buttons: `KEY1..KEY7` arm their lane for
    /// rebind (the next mapped key press sticks), `3KEY_1..4` arm
    /// lanes 0..3 the same way (the 3-key lane mapping stays
    /// OPEN), `3KEY`/`7KEY` record the key-count selection, and
    /// `KEYTEST` flips its checkbox (both selection-only, OPEN).
    fn click_key_tab(&mut self, x: i32, y: i32) {
        for lane in 0..7u32 {
            if self.hit(OPTION_KEY_LANE_FIRST + lane, x, y) {
                self.rebind = usize::try_from(lane).ok();
                return;
            }
        }
        for lane in 0..4u32 {
            if self.hit(OPTION_KEY_3KEY_FIRST + lane, x, y) {
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
    /// by the ours-by-design step inside the 0–255 domain (which
    /// slider writes which `Sound` key is ours-by-design, OPEN).
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
