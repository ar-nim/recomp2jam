# #349 — song-select A/B (owner's three symptoms)

All captures and measurement scripts stay in scratch; only this
comparison record lives in the repo. No game bytes or capture pixels
are embedded.

## Sides and state match

| Side | Capture | State |
|---|---|---|
| exe (7.55) | `~/scratch/av-reports/frames/ref_musicselect.png` (800x600), the downscale of the pinned `docs/screenshots/reference-musicselect-4x3.png` used by `docs/parity/verify-photo.md` | room master, song panel, result tab, level column `전체` chosen, no help sheet |
| ours, before | `~/scratch/p349-shots/base/musicselect.png` (400x300, upscaled for region metrics) | same dialog state, staged 562-song install catalog, page 1 |
| ours, after | `~/scratch/p349-shots/after/musicselect.png` | same catalog/page as before |

The dialog state (panel, tab, chosen filter, art staging) matches; the
catalog and page differ (exe 1608 songs / page 3/161, ours 562 / page
1/169). That scenario delta is named, not treated as pixel parity.

A separate live Wine pack reached waiting with full audio containment
(`~/scratch/p349-ab/exe4`: `status=PASS`, `memoryer_pid=92326` shared
with the Rust side, `preboot_breach=0`, `on_default_sink=0`,
`route=probe-only` — the Wine client opens no bound stream for the
route check, unlike `p343-wine-role4`'s route=sink leg). The exe's
song dialog is master-only, and the qualified Wine driver joins a
bot-owned room, so it cannot open this dialog; room-create is a known
wall (`docs/wine-boot.md`, Attempts 10/12). The master-view reference
photo above is therefore the exe side for this A/B.

## Region verdicts (native 800x600 coordinates)

| Region | exe | ours before | ours after | Root cause / fix |
|---|---|---|---|---|
| Left level column, inactive face `(30..115, 150..175)` mean luma | 122.9 | 187.4 | 127.2 | `Dialog_SelectMusic.ojs` already bakes the five grey faces and Korean labels; painting `SelectMusic_Btn_Step1_*` frame 0 (a bright hover frame) over them was symptom 1. Inactive level bodies now stay hidden; the chosen `전체` dark frame still paints. |
| Result header `(330..770, 120..150)` orange underline pixels | no underline under 곡명/작곡가 etc.; one under 레벨 | 988 bright underline pixels | 49 (the level sort's own frame) | Six sort bodies plus remove-all and row tag clones repeated art already baked in the sheet (symptom 2). The result tab hides `SelectMusic_Btn_{Diff,Label,Title,Artist,Down,Rank}.ojs`, `SelectMusic_RemoveAll.ojs`, `SelectMusic_RemoveBox.ojs` and `MShop_MMY_Color.ojs`; the level sort keeps its live frame. |
| Difficulty cell `(387..402, 154..215)` | H/N/E letters vary per chart row | E on every row | E, N, H across three rows | `SelectMusic_Diff.ojs` stores E/N/H as frames 0..2; the old rule painted one filter-state frame on every row clone. The frame now follows the row's `SelectMusicRow::slot`. |

Geometry is unchanged: every rect still comes from `Dialog_SelectMusic.bnd`
via `stage_dialog`; only frame choice and paint gating changed.

## Guards (live caller chain named in `crates/recomp2jam/src/shots.rs`)

- `unselected_level_buttons_leave_the_baked_faces_visible` — red: 2277
  mismatched pixels on `SelectMusic_Btn_Step1_Easy.ojs`.
- `result_tab_leaves_baked_header_and_row_tags_visible` — red: 212
  mismatched pixels on `SelectMusic_Btn_Diff.ojs`, then 219 on
  `SelectMusic_Btn_Label.ojs` when the remaining sort bodies were added.
- `three_chart_rows_paint_their_own_difficulty_letter_frame` — red:
  row 1 expected the N frame, got E.

All three were run red before the fix and green after it; the after
shot was regenerated from the fixed tree.
