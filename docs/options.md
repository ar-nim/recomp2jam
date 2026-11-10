# Options dialog (`Dialog_Option_*`, ticket #18)

The gear disc opens three staged sheets over the planet board
(`DIALOG_OPTION_KEY` / `DIALOG_OPTION_SOUND` / `DIALOG_OPTION_GAME`,
all three staged through the declaration since #18 — tab switches
ride the staged `KEYSET`/`SOUNDSET`/`GAMESET` codes through their
`.bnd` rects, never literals). Which tab opens first stays OPEN
(first-open stand-in: Key).

## What the dialog does

- Key tab: `KEY1..KEY7` clicks arm the lane; the next mapped key
  press sticks (only keys with a DirectInput code — unmapped keys
  are refused so every persisted `KEY_n` stays exe-readable).
  `3KEY_1..4` arm lanes 0..3 the same way (the 3-key lane mapping
  stays OPEN). `3KEY`/`7KEY` and `KEYTEST` record selection state
  only (behavior OPEN).
- Sound tab: four Down/Up slider pairs step `GameBGVolume` /
  `BGVolume` / `NoteBGVolume` / `NoteKeyVolume` by 8 inside 0–255.
  GameEffect drives the live key-strike gain; BGMusic scales the
  lobby scene-BGM cue. The in-song mix path stays OPEN.
- Game tab: NoteType1/2 → 0/1, guides Long/Middle/Short/NoUse →
  3/2/1/0, EQ Yes/No → 1/0, ModeFull/Win → 1/2, Longnote
  Used/None → 1/0, effects High/Normal/Low → in-memory 0/1/2.
- SAVE commits to `option.ini` beside the install root (stays
  open); EXIT/Escape close without saving; DEFAULT restores the
  tab's exe defaults. Settings load at boot; a restart keeps them.
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
| `Key`/`KEY_1..7` | DIK SDF Space JKL | key-table `+0x345..` | claims B37 | `0x1435xx` family |

Consumers: `Mode` 1/2 arms drive the window apply (file `0x3a06c`);
`NoteType` 0 vs nonzero branch (file `0x171480`, folds at
`0xE1C22`-family — value 2 dead alias); `NoteTail` indexes the
five-slot double table at `0x67fe8c`; `LongnoteMode == 1` single
consumer (file `0x16E372`).

## OPEN (each with its rig next step)

1. Slider↔key mapping, slider step, in-song mix application —
   next step: rig SAVE-diff (move one slider, SAVE, diff
   `option.ini`).
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
`options_key_tab_rebind_refuses_unmappable_keys`,
`options_sound_tab_steps_move_volumes_with_exe_bounds`,
`options_game_tab_buttons_map_to_the_pinned_ini_values`,
`options_tab_switches_ride_the_staged_codes`,
`options_save_commits_beside_the_install_root`,
`options_exit_closes_without_saving`,
`options_default_restores_the_exe_defaults_per_tab`,
`longnote_mode_snapshots_at_playing_entry`,
`key_strikes_play_at_the_game_effect_gain`,
`scene_bgm_cue_scales_with_the_bgmusic_volume`,
`option_tabs_stage_all_three_sheets_when_archives_present`.
