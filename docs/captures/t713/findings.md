# T713 — Options playthrough: dynamic control visuals (one boot window, 2026-11-20)

Ticket: #713 (parent Spec #314). This is a hash-pinned **pointer**, not a
citable run dir: the PNG captures and audio stay in the git-ignored pack
`~/scratch/options-pt/win1/exe/` (copyrighted exe bytes never enter the
repo). The prefixed manifest, provenance, citation and input log are
committed here; `scripts/rig-provenance.sh check-record docs/captures/t713`
passes.

**Answer in one line:** one boot opens all three `DIALOG_OPTION_*` sheets and
captures 14 states. The KEY glyph swap and both GAME-tab radio changes are
pinned to exact staged records; the slider bar records are static and the
exe clips the fill at the right end; the arming face and one guide-level aim
are pinned as a null result and a lost stimulus respectively.

## Pins

| Item | Value |
|---|---|
| Client | `OTwo.exe` == `AngelO2Jam.exe`, 5,029,888 B, sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` (7.55) |
| Art | `Interface1.opi` sha256 `d66541198965d622ff0177b306d916885ae4d64ae07b2c2264fcbdd850d50584` |
| Build / worktree | `9a51b54`, `cb2/options-playthrough` from `origin/main` |
| Boot / drive | `rig-run.sh run :170` → `rig-wine.sh launch argv.txt` → `wine-scene.py --target options` → scratch `drive_options.py` (14 captures) → `rig-run.sh stop` |
| Input | winegcc `sendinput-helper.exe.so` via `rig-wine.sh send` only; no XTEST canvas click |
| Audio | owned sink `rig_null_1186805`, manifest fields `route=sink preboot_breach=0 on_default_sink=0`; peak 0.0 dBFS |
| Memoryer | pid 92326, boot epoch 1794986056, gateway 15010 |
| Verify | `run-manifest.sh verify` PASS; `rig-provenance.sh cite --claim exe` emitted; live-lane lock `options-pt` held boot→last shot |

No crash dialog appeared (pre-click probe ran first); the NORMAL-mode popup
was cleared by the kit before canvas clicks.

## Geometry (measured on this boot)

- Each sheet's single background record (`Dialog_Option_Game.ojs` rec0,
  blob `dbdf4a796a4f184f…`, 365x515 at anchor (217,42)) paints at screen
  **(217,42)**; Game's rec0 matches 182490/183282 opaque pixels (100%).
- `.bnd` control rects are parent-relative: clicking rect-local + (220,43)
  hits every aimed control, while bound art paints at rect-local/anchor +
  **(217,42)** — a 3 px difference that tickets 714–716 must preserve.
- Tab centres: KEY (299,124), SOUND (408,124), GAME (512,124); gear (729,580).

## Capture inventory (full 800x600 PNG, sha256)

`planet.png` and `options.png` come from `wine-scene.py`; hashes are in
`wine-provenance.json`. Playthrough shots:

| shot | state | sha256 (first 16) |
|---|---|---|
| `play-01-sound-open` | gear re-open, Sound | `8324481211241abe…` |
| `play-02-key-rest` | KEY sheet, 3KEY mode, L Shift | `80693114d98b5aa6…` |
| `play-03-key1-armed` | after clicking `3KEY_1` | `cca1bf1c630a0cf8…` |
| `play-04-key1-bound` | after `a` sticks | `85ffe1eb2492a1b5…` |
| `play-05-sound-rest` | SOUND rest | `113d16f6b3bab430…` |
| `play-06-gamemusic-down` | GameBGVolume left arrow | `6e9198e2151fac7b…` |
| `play-07-gamemusic-restored` | right arrow | `5d594cc6025d01d1…` |
| `play-08-bgmusic-down` | BGVolume left arrow | `8e51dd533193996d…` |
| `play-09-game-rest` | GAME rest | `e4b8354ccf1ffd8d…` |
| `play-10-notetype2` | clicked selected NoteType2 | `f6ae214c3359997b…` |
| `play-11-notetype1-restored` | NoteType1 selected | `cff7aa094f777d8c…` |
| `play-12-guide-short` | guide aim missed | `01419bd21246a779…` |
| `play-13-guide-long-restored` | guide aim missed | `adffc3aa790b53a4…` |
| `play-14-game-default` | DEFAULT pressed | `bdb8550ab59944da…` |

Full hashes, per-pair diffs, zoomed crops and the record-match log are in
`~/scratch/options-pt/analysis/` (`captures.json`, `pairs.json`, `focus.json`,
`records.json`, `record-identification.log`), kept outside the pack so
provenance stays clean.

## Per-control verdicts

Record identification follows `docs/hover.md`: staged record opaque pixels
matched against the capture at its anchor; the hash is of the raw RGB555
blob inside `Interface1.opi`.

### KEY sheet — `3KEY_1` (the sheet opens in 3KEY mode, not KEY1..7)

- **Arm: NULL RESULT (pin).** `play-02` → `play-03`: dialog-scoped diff 446 px,
  bbox (299,124)-(341,388); only the cursor leaves the tab and lands on the
  button (214 px at (323,363)-(341,385)). No `OptionKey_Keydown.ojs` record
  lights; the click still arms, since the next key press sticks.
- **Stick: PINNED.** `play-03` → `play-04`: 867 px label change. Before =
  `OptionKey_Keytext.ojs` **rec34** ("L Shift", 28x29, blob sha
  `74a87546a7cf5f5f…`) 538/543 at (318,336). After = **rec12** ("A", 23x24,
  blob `b5bb098952599acb…`) 462/463 at (322,340) — glyph centred 4 px in.
- Re-run: KEY1..7 in 7KEY mode; arm face with the cursor parked off-button.

### SOUND sheet — GameBGVolume / BGVolume bars

- First GameBGVolume left-arrow click: **NULL RESULT (pin)**.
  `Option_Sound_GameMusicBar.ojs` rec0 (239x10, blob `9407e0ac1e06e4cb…`)
  matches 2366/2366 in both `play-05` and `play-06` at (283,360); only the
  119 px cursor blob changes.
- Next right-arrow click (`play-07`): rec0 falls to 2329/2366; the delta is
  38 px at the bar's **right cap** (517,360)-(522,370), rightmost fill pixel
  x=520→518. `Dialog_Option_Sound.ojs` rec0 (blob `dd8b97a40f0ffc2e…`)
  match drops 173599→173034/183282: the sheet background paints the fill,
  clipped short — **no staged record moves**.
- BGVolume left-arrow (`play-08`): `Option_Sound_BGMusicBar.ojs` rec0 (blob
  `720b1cc7f9515d02…`) 2366/2366 → 2298/2366; rightmost fill pixel x=520→515,
  68 px at (514,179)-(522,189). Same static-record / clipped-fill rule.
- OPEN: step↔pixel curve and left/right arrow polarity (first left click
  painted nothing; the first shrink followed the right click). Re-run with
  SAVE-diff of `option.ini`, ≥2 s settle, two consecutive steps.
- `NoteBGVolume` / `NoteKeyVolume` were not clicked (plan named one + one).

### GAME sheet — NoteType (toggle-style)

- `play-10` (clicked the already-selected NoteType2) → `play-11`: **PINNED**
  radio flip, 454 px across both checkboxes, bbox (270,327)-(418,348).
- All `Option_Game_*` checkbox bodies carry the SAME 16x16 record: blob
  `d490ea87963c2fc95d06523cc8ce50418085002cbe68378bf760cecbd9981be6`
  (body `Option_Game_NoteType2.ojs` `5281caad…`). It is the yellow checkmark,
  painted at rect-local + (217,42): (392,325) for NoteType2 (256/256),
  (268,325) for NoteType1 after the flip (227/256). Unselected = the sheet
  background showing through; no separate unchecked record.
- `play-09` → `play-10`: **NULL RESULT (pin)** — 119 px of cursor only.

### GAME sheet — guide level: LOST TO TIME

- `play-09` → `play-12` / `play-13`: middle stays selected; 27/54 px are
  cursor tip. BND guide row y 353-369 → screen 395-411; aims landed at y=412.
- Re-run: click (407,403) and (279,403); the checkmark record above applies.

### GAME sheet — DEFAULT

- `play-13` → `play-14`: **PINNED**, dialog diff 6525 px. Effect row 488 px:
  boot `3D 이펙트 사용함` → DEFAULT `고급` (High). NoteType row 224 px:
  NoteType1 clears, NoteType2 (롱노트) re-checks. Guide row unchanged.
- Button face: `Option_Default.ojs` rec0 (87x26, blob `0a30681f26137dfa…`)
  matches 2100/2190 at (290,519) in the hovered post-click state; at rest it
  is 377/2190 and rec1 1143/2190 — the desaturated-rest / frame-0-hover rule
  of `docs/hover.md`.
- Boot state ≠ DEFAULT state (Effect 3D vs High): the sheet opens from
  `option.ini`, DEFAULT restores exe defaults. Sharpens Spec #314 story 15.

## Stated gaps

Arming face, KEY1..7, guide flip, slider polarity/curve, two unclicked slider
families, tail values 3/4 and mode flags stay OPEN with the next evidence
step named above and in Spec #314 OPEN arms 2–6.
