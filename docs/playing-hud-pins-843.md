# #843 — Playing HUD capture pins

Evidence: paired chart-timed boot on HEAD `39e7549d7e39a17767d2a6f2be6541a9d1bb6a80`,
trace `S843-HEAD-PAIR-01`, pack `/home/arnim/scratch/boot-843/pack/p1`, summary
`/home/arnim/scratch/boot-843/paired-pack.json`. Pair A lanes SSIM 0.213 / bottom
HUD 0.056; pair B 0.192 / 0.056. The divergence is on current HEAD, not a stale
frame. Exe SHA-256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

All coordinates below are native 800x600 ink boxes. They enter the shell as
capture-pinned staging data in `o2jam-shell/src/playing.rs`
(`PlayingHudLayout`, `HUD_*` constants) and are scaled through the same
`scale_rect_to_frame` / `scale_scene_origin` path every other staged HUD rect
uses. Painters consume the resulting placements and layout; no paint-side
coordinate literals were added.

| Region | Exe pin | Previous Rust | Fix path |
|---|---|---|---|
| Right life band | y267-500, x241-245 | y260-500 | staged `Life_gauge_left_bar.ojs` anchor `(238,264,12,247)` + `(0,7)`; rewritten placement uses the rect-positioned clone role |
| JAM bar | x16-235 @y536 | x23-228 | staged `Jamgauge_left_bar.ojs` anchor + `(-7,0)`, width +14; procedural fill takes the same pinned rect |
| JAM label | `JAM` x114-137 y541-546 | low-band ink near y552 (the #843 offset was staged but not painted) | `Jam_gauge_text.ojs` re-staged at anchor + `(29,2)`, its absolute record anchor cancelled in the clone rect, and painted after the procedural jam fill so rows 541-546 carry the word |
| Combo word | `MAX COMBO` x32-63 y572-591 | venue `SC...` x13-38 y560 | staged `P2P_MaxCombo_Text.ojs` painted at `(32,572)` through `PlaySkin::combo_word` |
| Own combo zero | x105-113 y570-588 | x86-93 y556-565 | staged `ComboScroe.ojs` digit painted at `(105,570)`; visible at combo 0 |
| Judgement `MISS` | x97-155 y360-375 | x88-118 y447-457 | word centre pinned to `(126,368)`, centre-anchored as the staged word art |
| Bottom tally | COOL/BAD x320/420, y558/578 | x310/405, y560/580 | capture count grid used when the staged field set is present |
| TIME bar | y560-568 | y550-558 | parsed `State_Playing.bnd` rect 2 translated `(0,4)`; the bnd rect stays the archive-less fallback |

## Guards

- `shots::tests::playing_time_fill_stays_inside_the_staged_bar_rect` — live chain:
  `main.rs::run_play` → `ScreenGame::paint_playing` →
  `LivePlay::paint_avatar_scene_with_placements` → `AvatarScene::paint_with_placements`
  → `AvatarLayout::for_frame_placed` → `PlayingBoundFields::from_placements`.
  Data-gated (skips without `install-root`). RED on HEAD:
  `assertion left == right failed`, left `Rect { x: 489, y: 556, w: 254, h: 10 }`,
  right `Rect { x: 489, y: 560, w: 254, h: 10 }`. GREEN after the fix.
- `playing::live_play_tests::jam_label_ink_lands_in_the_paired_rows_when_the_gauge_is_staged`
  — live chain: `main.rs::run_play` → `ScreenGame::paint` →
  `ScreenGame::paint_playing` → `paint_playing_backdrop` →
  `LivePlay::paint_over_staged_backdrop` → `paint_jam_gauge`, then
  `PlayingHudLayout::from_placements` → `pinned_placements` →
  `paint_foreground_placements` → `paint_staged_jam_label`. Data-gated
  (skips without `install-root`); the seam replays the pass order because
  the bug is the staged label's placement relative to the gauge fill.
  RED without the anchor cancellation: `the JAM word must ink the exe's
  row 541; found []`.
- `playing::live_play_tests::capture_pins_rewrite_staged_hud_anchors_when_the_playing_sprites_are_staged`
  — live chain: `main.rs::run_play` → `ScreenGame::paint` →
  `ScreenGame::paint_playing` → `PlayingHudLayout::from_placements` →
  `pinned_placements` → `paint_foreground_placements`. Pure placement-to-rect seam.
- `playing::live_play_tests::capture_pins_move_the_count_grid_and_time_bar_when_fields_are_staged`
  — live chain: `main.rs::run_play` → `ScreenGame::paint_playing` →
  `AvatarScene::paint_with_placements` → `AvatarLayout::for_frame_placed` →
  `tally_origins_native` / `PlayingBoundFields::from_placements`. Pure layout seam.

## OPEN (not implemented here)

- Positive combo and long-note digit frames: the paired chart produced only
  misses, so combo stayed 0 and no hold tail painted positive ink. Next step:
  the same paired boot with a Cool/Good + hold chart, then read the digit boxes.
- Own-vs-peer gauge fill mapping: the exe's right gauge drains after misses;
  whether Rust's left strip is the own gauge needs a wire/paint pin.
- VOL and ghost-label scenario variants: the live path never calls
  `ScreenGame::set_volume`, and the ghost `COOL/GOOD/MISS` ink is baked into
  the venue strip art. A 1:1 member/scenario capture is needed before a verdict.
  The TIME row pin above does not depend on that scenario.

## Pack verification gaps (kept, not hidden)

`scripts/run-manifest.sh verify` exited 1 for the exe pack (duplicate screenshot:
`result-master.png` == `gate-frame-0620-chrome.png`) and for Rust (producer rebuilt
its binary after capture; `x11grab` provenance tool unavailable). The paired PNGs
and their pinned hashes are intact; the anchors above come from those pixels and
do not depend on the manifest duplicate. Re-verification needs a fresh pack, not
a rewrite of this evidence.
