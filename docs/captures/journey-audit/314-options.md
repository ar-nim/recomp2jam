# Journey audit — #314 Spec: Options dialog system

Base: `2105ac6`. No exe boot in this leg; evidence comes from existing
qualified packs and repo code only.

## Evidence anchors (artifact + hash)

| Id | Artifact | Hash / provenance |
|---|---|---|
| `B3` | `~/scratch/side-scenes/b3-dialogs/` exe 7.55 (exe sha256 `83a04541…72c823b`), `artifacts/input.log`: board-prime, gear `(729,580)`, Escape close; `artifacts/screenshots/options.png` | manifest sha256 `4f0428d04ebd8ceba875f67e8eb4daa4d880d7379323b735483cbc3567c176d2`; `options.png` sha256 `333f9baee3795c5cdfe05be230a3857ca31f4d6fb5fad88454c1303c84d6198d` |
| `B4` | `~/scratch/side-scenes/b4-dialogs/` second qualified gear-open run | manifest sha256 `e2241ab5974a66c222cc925f994ffae60b81cd0e1a51905fb059eb8971a506c2`; `options.png` sha256 `6d17a781e4d5e522da5ad72e62c26281ae0d7317b20bde9d7bad73421548043e` |
| `SMOKE18` | `docs/smoke/options-configure-restart.md` — real XTEST trace, ini hash chain, restart | trace sha256 `9cd5aa399cfb4081571c8bd3b88df01453bc923c61ffd1ececaceb0c075d1330`; manifest sha256 `df6a0a7effab1435d19c2db9b3e7019dd04aafa1ca0d92bb8475d83459963b30` |
| `STATIC` | `docs/options.md`, `docs/bnd-semantics.md`, `docs/hover.md`, `docs/geometry-audit.md` | repo files at `2105ac6` |

Our journey: boot → `PLANET_OPTION_FIRST_TAB` (`side.rs:3105`, default `Key`)
→ `load_planet_option_scene` (`game/mod.rs:4121`) → `paint_planet_options`
(`paint.rs:2302`) → `click_planet_modal` (`game/mod.rs:4253`) →
`OptionsSettings` (`options.rs:407`). The exe captures show the gear sheet body
with four volume sliders and the three tab plates; both qualified runs agree.

## Summary matrix

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 6 | 1, 13, 14, 16, 18, 19 |
| MISMATCHED | 1 | 2 |
| BLOCKED | 10 | 3, 4, 6, 8, 9, 10, 15, 17, 20, 21 |
| NA-DESIGN | 5 | 5, 7, 11, 12, 22 |

New issues: #359 (Options gear opens Key tab in Rust; exe opens the Sound
slider page). No mappings to OPEN blocker tickets for the single mismatch.
Mapped, not filed: stories 9/17/21 live gain and in-song mix → **#326**
(Playing wedge); stories 20 frame/hover A/B → **#245/#249/#250** (capture
tooling); story 8 slider-to-key order is one of the spec's six OPEN arms.

BLOCKED for live lane: tab-switch captures; key rebind and 3-key captures;
slider SAVE-diff; live gain/loudness A/B; DEFAULT value orders; hover/press
frame A/B; longnote snapshot in Playing.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Gear opens exe first sheet | input-response | gear click handled on Planet; `planet_modal=Options`, `load_planet_option_scene` (`game/mod.rs:4121`) | `B3`/`B4` input logs gear click `(729,580)` then `options` in `scene-sequence.txt` | MATCHED | Journey matches. The rig needs a harmless board-prime click first; that is a SendInput canvas artifact, not product behavior. |
| 2 | First-open tab follows exe pin | pixel-region + state-path | `PLANET_OPTION_FIRST_TAB = Key` (`side.rs:3105`); `game/mod.rs:974` initializes tab Key; `SMOKE18` says "gear opened Key" | `B3` `options.png` and `B4` `options.png` (hashes above) show four volume sliders (`배경음악 Vol`, `게임음악/효과음 Vol`, `게임음악 Vol`, `효과음 Vol`) with DEFAULT/SAVE/EXIT | MISMATCHED | Named divergence: our sheet opens the Key bindings page; exe opens the Sound slider page. Novel issue #359. |
| 3 | KEYSET/SOUNDSET/GAMESET switches | input-response | `PlanetOptionTab::switch_tab` (`side.rs:2996`), staged-code guard `options_tab_switches_ride_the_staged_codes` | No qualified tab-click capture; `B3` only proves the sheet opens | BLOCKED | Need one qualified capture per switch. #245 class. |
| 4 | KEY1..7 arm + next mapped key sticks | input-response / state-path | `option_rebind` arm and `rebind_lane` (`options.rs:590`); `SMOKE18` KEY1→`a` with ini `KEY_1=30` | `STATIC` DIK table and write sites `0x1435xx`; no exe live rebind capture | BLOCKED | Our journey is real-input verified; exe side has static pins only. Need one rig rebind capture. |
| 5 | Unmapped keys refused | NA-DESIGN (test) | `options_key_tab_rebind_refuses_unmappable_keys` (`game/mod.rs`), `dik_of_keycode` (`options.rs:198`) | `STATIC` B37 DIK table | NA-DESIGN | Negative mapping test pinned to the DIK table; no journey to compare. |
| 6 | 3KEY_1..4 arm lanes 0..3 | input-response | `OPTION_KEY_3KEY_FIRST` (`side.rs:3029`) + click arm | `STATIC` says the 3-key lane mapping stays OPEN | BLOCKED | No exe capture names the lane order; needs one. |
| 7 | 3KEY/7KEY/KEYTEST selection only | NA-DESIGN (test) | `OPTION_KEY_3KEY_MODE`/`7KEY_MODE`/`KEYTEST` set selection state only | `STATIC` lists the same OPEN arm | NA-DESIGN | Labeled stand-in; no invented behavior on either side. |
| 8 | Slider step 8 inside 0..255 | data-effect | `SOUND_SLIDER_STEP` (`options.rs:97`) and step tests; `SMOKE18` Sound SAVE wrote `GameBGVolume=247` | `B3` `options.png` shows four sliders; `STATIC` pins bounds/default but leaves step OPEN | BLOCKED | Need the SAVE-diff named in `docs/options.md` OPEN arm 1. Our 8 is explicitly ours-by-design, not an exe claim. |
| 9 | GameEffect drives key-strike gain | data-effect | `key_strikes_play_at_the_game_effect_gain` guard | No sound-on exe A/B; `B3` audio is only scene audio | BLOCKED | Needs a gain A/B under the owned null sink; #326 Playing wedge noted. |
| 10 | BGMusic scales lobby BGM | data-effect | `scene_bgm_cue_scales_with_the_bgmusic_volume` guard | `B3` audio recorded but no volume sweep | BLOCKED | Needs one exe volume sweep with artifact+hash; capture-tooling class. |
| 11 | Note option values mapped | NA-DESIGN (test) | `options.rs` mapping functions and `options_game_tab_buttons_map_to_the_pinned_ini_values` | `STATIC` read/write sites and clamps (`0x1713c7`, `0x16bf47`) | NA-DESIGN | Parser/mapping tests over static pins; no player journey required here. |
| 12 | NoteType 2 dead alias | NA-DESIGN (test) | `shape_arm_of_note_type` (`options.rs:142`) folds 2 into 1 | `STATIC` branch at file `0x171480` | NA-DESIGN | Negative mapping guard over a static pin. |
| 13 | SAVE writes `option.ini` | data-effect | `OptionsSettings::save_to_dir` (`options.rs:567`); `SMOKE18` ini chain `f35155ce…→e5852e93…→e6c73112…→2e06946d…` | `STATIC` `WritePrivateProfileStringA` sites for each key | MATCHED | Both journeys end in an ini write with the same section/key names; live exe SAVE-diff remains OPEN arm 1 but no divergence is observed. |
| 14 | EXIT/Escape close without saving | input-response | `click_planet_modal` EXIT arm + Escape close; `options_exit_closes_without_saving` guard; `SMOKE18` Escape closes | `B3`/`B4` input logs inject Escape after capture and leave options | MATCHED | Both press paths return to the bare Planet board. |
| 15 | DEFAULT restores tab defaults | data-effect | `reset_binds`/reset methods + `options_default_restores_the_exe_defaults_per_tab` | `STATIC` defaults pinned; no exe DEFAULT capture | BLOCKED | Button-to-value orders are OPEN arm 2; needs capture before a claim. |
| 16 | Settings load at boot and survive restart | data-effect | `load_from_dir` (`options.rs:469`); `SMOKE18` restart kept ini `2e06946d…` and the same modal/content hash | `STATIC` `GetPrivateProfileIntA` read sites | MATCHED | Our restart journey is hashed; exe read sites are pinned (no live restart capture in the reused packs). |
| 17 | LongnoteMode snapshot at Playing entry | state-path | `play_longnote` + `longnote_mode_snapshots_at_playing_entry` | `STATIC` exe cache at `+0x17d8`, consumer `0x16e372` | BLOCKED | Needs a live Playing entry with an edited switch; #326 noted. |
| 18 | 25/22/23 placements at parsed rects | state-path / data-effect | `option_tabs_stage_all_three_sheets_when_archives_present`; `load_planet_option_tab` (`game/mod.rs:4133`) | `B3`/`B4` sheet at exe rect with parent origin; `STATIC` rect counts | MATCHED | Geometry comes from `.bnd` placements on both sides. First-open sheet content is story 2. |
| 19 | Parent-relative dialog offsets | pixel-region | `dialog_origin` (`paint.rs:2048`), `paint_dialog_scene` (`:2115`) | `SMOKE18` records origin `(217,42)` from `o2jam-dump rect`; `B3` sheet sits there | MATCHED | Both sheet origins agree with the parsed rect data. |
| 20 | Rest/hover/active frames | pixel-region | art delta guards in `docs/parity/art-delta-guards.md` | `B3`/`B4` only rest frames; `b2-dialogs*` attempts to reach further dialogs failed (`status FAILED`) | BLOCKED | Need a qualified hover/press A/B; #245/#249/#250 class. |
| 21 | In-song mix follows volume words | data-effect | `key_strikes_play_at_the_game_effect_gain`, scene cue test | `STATIC` OPEN arm 1, no live mix trace | BLOCKED | Same SAVE-diff + Playing capture as story 8; #326 noted. |
| 22 | Six OPEN arms name rig steps | NA-DESIGN (docs) | `docs/options.md` OPEN 1–6 | `STATIC` same list | NA-DESIGN | Reviewer/documentation story. |
