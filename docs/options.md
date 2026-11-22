# Options dialog (`Dialog_Option_*`, ticket #18)

The gear disc opens three staged sheets over the planet board
(`DIALOG_OPTION_KEY` / `DIALOG_OPTION_SOUND` / `DIALOG_OPTION_GAME`,
all three staged through the declaration since #18 — tab switches
ride the staged `KEYSET`/`SOUNDSET`/`GAMESET` codes through their
`.bnd` rects, never literals).

First-open sheet: **Sound**, the four volume sliders. Exe 7.55
(`OTwo.exe` sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`),
two qualified gear packs:

- `~/scratch/side-scenes/b3-dialogs/`, manifest sha256
  `4f0428d04ebd8ceba875f67e8eb4daa4d880d7379323b735483cbc3567c176d2`,
  `artifacts/screenshots/options.png` sha256
  `333f9baee3795c5cdfe05be230a3857ca31f4d6fb5fad88454c1303c84d6198d`.
- `~/scratch/side-scenes/b4-dialogs/`, manifest sha256
  `e2241ab5974a66c222cc925f994ffae60b81cd0e1a51905fb059eb8971a506c2`,
  `artifacts/screenshots/options.png` sha256
  `6d17a781e4d5e522da5ad72e62c26281ae0d7317b20bde9d7bad73421548043e`.

Both frames show `배경음악 Vol`, `게임음악/효과음 Vol (F2/F1)`,
`게임음악 Vol`, `효과음 Vol`, with DEFAULT/SAVE/EXIT. The former Key
stand-in (first in `ControlList_Interface.txt` file order) is
refuted; this pin replaces it. Tab-switch captures remain OPEN.

The driven live-boot guard
`live_boot_driven_tab_clicks_repaint_sound_and_game_sheets` (#854) presses
the gear, then drives `(509,123)` (Sound GAMESET), `(405,123)` (Game
SOUNDSET) and `(300,125)` (Sound KEYSET) through `GameLoop::ingest_snapshot`,
requiring the painted sheet to change each time. With Sound first-open,
`(405,123)` is the *selected* tab's dead zone, so the journey note's
"Sound click inert" is expected, not a dispatch fault. A paired exe
Key-tab frame stays OPEN (#854).

## What the dialog does

- Key tab: the dialog opens in **3K** (exe constructor `0x438080`;
  capture `play-02-key-rest`). `3KEY`/`7KEY` switch the staged-code
  family shown and clickable: selector, board panel, and lane
  captions (`0x4383d0`). Lanes arm from staged rects; a successful
  mapped key press writes the active mode's bind table and advances
  the arm (7K wraps over seven lanes, 3K over indexes 0, 2, 3;
  `0x438e80` / `0x4395e0`). 3K index 1 shares a staged rect with
  index 2 and is skipped by repaint `0x439780`. Unmapped keys are
  refused, and SAVE rejects zero or duplicate active binds before
  writing (`0x439170` / `0x4394c0`). The paired Key-rest frame
  (`~/scratch/av-verify/options/compare-residual`, exe sha256
  `80693114…384410b`) pins the unchecked chrome state: the key-test
  checkbox shows the empty backdrop box, and all three pad faces are
  the sheet's backdrop faces, not the lit `OptionKey_3Keydown.ojs`
  columns (#845). The Key painter now holds those staged bodies back
  while key test is unchecked; the checked state paints them from the
  same staged controls. OPEN: the 3K default slot order
  for R Shift / Enter, exact panel animation frames, SAVE message
  text, and whether the key-count selection persists.
- Sound tab: four Down/Up slider pairs step `GameBGVolume` /
  `BGVolume` / `NoteBGVolume` / `NoteKeyVolume` by the exe-pinned 4
  inside 0–255. The first knob moves `GameBGVolume` and `BGVolume`
  together (`#712` static pin, `0x551710(..., 1)`).
  `NoteKeyVolume` drives the live key-strike gain, `BGVolume` the UI
  effects, `GameBGVolume` the lobby scene-BGM cue (re-cued on the
  slider click, no scene change), and `NoteBGVolume` the modeled
  in-song song gain. No song PCM path applies that last gain yet;
  the live mix trace stays OPEN.
- Game tab: NoteType1/2 → 0/1, guides Long/Middle/Short/NoUse →
  3/2/1/0, EQ Yes/No → 1/0, ModeFull/Win → 1/2, Longnote
  Used/None → 1/0, effects High/Normal/Low → in-memory 0/1/2.
- SAVE commits to `option.ini` beside the install root (stays
  open); EXIT/Escape close without saving; DEFAULT restores the
  tab's exe defaults (Game's raw `LongnoteMode` goes back to its
  pinned read default 3, which the `== 1` consumer reads as off).
  Settings load at boot; a restart keeps them.
- `LongnoteMode` snapshots into the play setup at Playing entry
  (mirrors the exe `+0x17d8` cache): later edits never reach the
  running song.

## Exe pins (7.55 md5 `34b789ff…`, objdump 2.47, 2026-11-10)

Reads (`GetPrivateProfileIntA`, IAT `0x63f268`, file `option.ini`
except where noted), all with writer twins
(`WritePrivateProfileStringA`, IAT `0x63f26c`, same names):

| Key | Default | Stored | Read site | Write site |
|---|---|---|---|---|
| `Sound`/`GameBGVolume` | 255 | obj `+0x68` | file `0x14bb45` | file `0x14badb` |
| `Sound`/`NoteKeyVolume` | 255 | obj `+0x36164` | file `0x151587` | file `0x151475` |
| `Sound`/`NoteBGVolume` | 255 | obj `+0x36168` | file `0x1516d7` | file `0x1515fe` |
| `Sound`/`BGVolume` | 255 | obj `+0x3616c` | file `0x1517a7` | file `0x151745` |
| `MODE`/`Mode` | 0 | obj `+0xa34` | file `0x1a46e0` | files `0x1a4726`/`0x3a6e8` |
| `NoteScreen`/`BlendHalf` | 3 | obj `+0xa3c` | file `0x1a4798` | file `0x1a47de` |
| `NOTE`/`NoteType` | 0, clamp 0–2 else 0 | play `+0xeb4` | file `0x1713c7` | file `0x171447` |
| `NOTE`/`NoteTail` | 2, clamp 0–4 else 2 | global `0x67feac` | file `0x16bf47` | file `0x16bfd0` |
| `LONGNOTEMODE`/`LongnoteMode` (`Option.ini`) | 3 | play `+0x17d8` | file `0x16fb06` | — |
| `Key`/`KEY_1..7` | DIK SDF Space JKL | input object `+0x344`, lanes at indexes 1..7; builder VA `0x543290` / file `0x143290` | VA `0x543290` | `0x1435xx` family |

The key builder also rebuilds the 0x80-byte DIK→lane reverse map at VA
`0x67ebb0` (file `0x27ebb0`); the polled consumer `FUN_005441b0` (VA
`0x5441b0`) and buffered thread `FUN_00544ca0` (VA `0x544ca0`) read
the two tables. Their config→runtime handoff in this client stays OPEN;
the older `0x173290` file-offset label is contradicted by the fresh pin.

Consumers: `Mode` 1/2 arms drive the window apply (file `0x3a06c`);
`NoteType` 0 vs nonzero branch (file `0x171480`, folds at
`0xE1C22`-family — value 2 dead alias); `NoteTail` indexes the
five-slot double table at `0x67fe8c`; `LongnoteMode == 1` single
consumer (file `0x16E372`).

## OPEN (each with its rig next step)

1. Slider↔key mapping, slider step and first-knob coupling are pinned
   statically (see the addendum below: step 4, control IDs
   `0x26200002..09` / `0x26500013..16`, coupled setter
   `0x551710(..., 1)`). The four words now feed one modeled in-song
   mix; the live song-BGM application and slider-to-caption order still
   need the rig SAVE-diff (move one slider, SAVE, diff `option.ini`)
   plus a Playing capture.
2. Button→value orders (shape arms, guide order, Full/Win order) —
   next step: the ticket's exact rig list (one 0-vs-1 shape capture
   to name oval vs square; five tail captures to name the fifth;
   one mode 0-vs-1 A/B plus the timing-vs-rendering call).
3. Tail value 4 lights no button (four buttons, five slots) —
   same five-capture list.
4. `EffectHigh/Normal/Low` ini key (none found in the sweep) —
   next step: rig SAVE-diff.
5. `Mode` value 2 (exe string path) and `BlendHalf` raw 3 keep
   exe-mirror load behavior with effective defaults 0/1 —
   next step: boot-trace the string-path defaults.
6. Highway pixel application of shape/tail/mode — nothing paints
   from names alone; same rig list as (2).

Guards: `options::tests::*` (13: mapping, clamps, round-trip,
DIK), `options_key_tab_rebind_answers_on_the_new_lane`,
`three_key_lane_buttons_arm_lanes_zero_through_three` (same live chain as
the 7-lane rebind; the 4-button arm is the whole behavior, 3-key lane
mapping stays OPEN),
`key_tab_rebinds_all_seven_lanes_to_their_dik_codes` (seam suffices: the
dialog-to-DIK mapping is the whole behavior; live chain `main.rs::run_online`
→ `run_windowed_session` → `window::InputHandler::route_key` →
`GameLoop::capture_raw_key` → `OptionsSettings::rebind_lane`),
`options_key_tab_rebind_refuses_unmappable_keys`,
`options_sound_tab_steps_move_volumes_with_exe_bounds`,
`sound_tab_four_sliders_step_by_eight_and_stay_in_byte_bounds` (live chain:
`main.rs::run_online` → `run_windowed_session` →
`window::InputHandler::route_mouse_edge` → `GameLoop::ingest_snapshot` →
`ScreenGame::click_dispatch` → `click_planet` → `click_planet_modal` →
`click_options_dialog` → `OptionsDialog::click_sound_tab` →
`OptionsSettings::step_*_volume`),
`options_game_tab_buttons_map_to_the_pinned_ini_values`,
`options_tab_switches_ride_the_staged_codes`,
`options_tab_switches_open_each_own_staged_sheet` (all six sheet-owned
switch codes, Live chain: `main.rs::run_online` → `run_windowed_session` →
`window::InputHandler::route_mouse_edge` → `GameLoop::ingest_snapshot` →
`ScreenGame::apply_pointer_down` → `click_dispatch` → `click_planet` →
`click_planet_modal` → `click_options_dialog` → `OptionsDialog::click` →
`PlanetOptionTab::switch_tab`),
`options_save_commits_beside_the_install_root`,
`options_exit_closes_without_saving`,
`options_default_restores_the_exe_defaults_per_tab`,
`game_tab_default_restores_the_pinned_longnote_raw_default` (live chain:
`main.rs::run_online` → `run_windowed_session` →
`window::InputHandler::route_mouse_edge` → `GameLoop::ingest_snapshot` →
`ScreenGame::click_dispatch` → `click_planet` → `click_planet_modal` →
`click_options_dialog` → `OptionsDialog::click` → `OptionsSettings::reset_game`),
`longnote_mode_snapshots_at_playing_entry`,
`running_song_keeps_the_longnote_mode_from_the_options_tab` (live chain:
`main.rs::run_online` → `run_windowed_session` →
`window::InputHandler::route_mouse_edge` →
`GameLoop::ingest_snapshot` → `ScreenGame::click_dispatch` →
`click_planet` → `click_planet_modal` → `click_options_dialog` →
`OptionsDialog::click_game_tab` → Playing entry snapshot →
`ScreenGame::playing_longnote_used`; the visual effect of the exe's
`0x16e372` compare stays OPEN),
`key_strikes_play_at_the_game_effect_gain` (unity → one step → mute,
  live chain: `main.rs::run_online` → `run_windowed_session` →
  `window::InputHandler::route_key` → `GameLoop::lane_key` →
  `ScreenGame::strike_at` → `pump_sounds` → `AudioBackend::play_key`),
`first_sound_knob_moves_game_music_and_effect_words_together`
  (seam suffices: the coupled setter is the whole behavior),
`in_song_mix_follows_all_four_pinned_volume_words` (live chain:
  `main.rs::run_online` → `run_windowed_session` →
  `window::InputHandler::route_mouse_edge` → `GameLoop::ingest_snapshot` →
  `ScreenGame::click_dispatch` → `click_planet` → `click_planet_modal` →
  `click_options_dialog` → `OptionsSettings::step_*` → `InSongMix`;
  the NoteBG song PCM path stays OPEN),
`scene_bgm_cue_scales_with_the_bgmusic_volume`,
`bgmusic_slider_re_cues_the_lobby_track_without_a_scene_change` (live chain:
`main.rs::run_online` → `run_windowed_session` →
`window::InputHandler::route_mouse_edge` → `GameLoop::ingest_snapshot` →
`ScreenGame::click_dispatch` → `click_planet` → `click_planet_modal` →
`click_options_dialog` → `apply_scene_bgm` → `AudioBackend::set_scene_bgm`),
`option_tabs_stage_all_three_sheets_when_archives_present`,
`options_sheets_paint_rest_hover_and_active_frames` and
`options_tab_press_holds_its_active_frame_until_release` (the three
sheet frame states are a labeled stand-in; the census and next capture
step live in `docs/dialog-frame-states.md`).

## Static wiring addendum (ticket #712, 7.55 sha256 `83a045…`)

REA static pins, recorded in `~/scratch/options-static-findings.md` with the
full evidence IDs. VAs map to file offsets by subtracting `0x400000`.

- Sheet classes: RTTI names `CDOption` / `CDOptionGame` / `CDOptionSound` at
  `0x67d8c4` / `0x67d8e0` / `0x67d900`; ctors `0x437e80` / `0x439960` /
  `0x43adb0`; primary vftables `0x644fb4` / `0x645148` / `0x6452ac`. One scene
  dispatcher `0x476330` constructs them at cases 5 (Key), 6 (Sound), `0xf`
  (Game). (ev `ev_f37bff54…`, `ev_519b6283…`)
- Key captions are sprite frames, not text. `0x438080` builds the 37-entry DIK
  order array at object `+0x198`; `0x439230` finds a bound DIK's index there;
  `0x439000` / `0x439780` paint frame `index + 1` on controls `0x2520900c..12`
  / `0x25209008..0b` with three offset classes. (ev `ev_303bfa40…`,
  `ev_16bed920…`, `ev_6973bcac…`, `ev_76f933d4…`)
- Slider click arms `0x43bf30` step the volume word by **4** (not our 8),
  clamped to 0 and 255; the NoteBG pair uses `0x33` as its lower bound. The
  knobs are repainted by `0x43b690` from `trunc(volume / divisor * track_width)`
  with divisor 255 (204 for NoteKey). Control IDs `0x26200002..09` are the
  click arms; `0x26500013..16` are the four knobs. The first knob's setter
  `0x551710(..., 1)` also writes `GameBGVolume`, so exe moves BGVolume and
  GameBGVolume together. (ev `ev_d3068d91…`, `ev_aefab8e8…`)
- Game-tab click arm `0x439f40`: `0x2f209007/008` = NoteType 1/0;
  `0x2f209009..00c` = NoteTail 3/2/1/0; `0x2f209014/015` = Mode 2/1;
  `0x2f209016/017` = LongnoteMode 1/0; `0x2f209005/006` = BlendHalf 1/0;
  `0x2f209002..004` = in-memory note shape only. Load/save `0x43a630` /
  `0x43ab00` match the table above. (ev `ev_1f10e5d7…`,
  `ev_784693a1…`)
