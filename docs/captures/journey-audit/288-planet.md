# Journey audit — Spec: Planet state (#288)

Comparison leg only. No exe boot, no code change outside this doc, no push.
Base `2105ac6`; worktree branch `cb2/ja-a`. Comparator keys as in
`287-avi.md`. Exe pin for every row: `OTwo.exe` 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

## Per-story table

| Story | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|
| P1. Board renders channels at the pinned layout (checked) | pixel-region / state-path | `crates/o2jam-shell/src/screens/game/mod.rs:5782` staged `State_Planet` block → `crates/o2jam-shell/src/screens/paint.rs:2571` `paint_planet_scene`; soak `channel_layers_list_over_real_dialog_art`; docs/parity/planet.md Composition | t240 qualified planet frame `artifacts/screenshots/planet.png` sha256 `235f53521171be4f1d4c725d16143332fa47b66813f9e1c5b8e8b09fe68ddc21` (also reused as exe leg in `~/scratch/p346/pack/provenance.json`); c1 `planet.png` has the same sha256; `#334` region map (hash `a24a38f9eb9736033e33f511cc403882f0dd86a1f8e1a82dd1ce04a1bbee9070`) measures `board_chrome` 0.878071, `bottom_bar` 0.9574 | MATCHED (layout/state) | Geometry and scene composition are pinned; the scored pixel deltas are isolated in P3 (emblems) and P9 (promo). |
| P2. Promo carousel pulses at the pinned cadence (checked) | timing | `PLANET_PROMO_TICK_MS` 3000 ms accumulator: `crates/o2jam-shell/src/screens/game/mod.rs:4462-4527`, heartbeat call `:5539`, windowed arm `:6586`; guards `promo_tick_*` in `docs/parity/planet.md` Timing/Composition | Static pin: `docs/planet.md` "Promo carousel" + VA `0x48dc4c`/`0x48e4b0` frame logic; no two timed qualified exe frames in any reused pack (spec rig note) | MATCHED (static pin) | Live-lane BLOCKED: two or more timed frames in one qualified run are needed for a 1:1 cadence number; no new boot here. |
| P3. Five channel-family emblems staged per runtime semantics (checked) | pixel-region / data-effect | Same `State_Planet` stage; `docs/select-chrome.md`-style ordinal pairing; guards `planet_emblem_captions_decode_to_the_pinned_strings`, `hovered_emblem_reveals_the_pinned_plate_band`, `quest_and_tutorial_emblems_ignore_hover`; `docs/parity/planet.md` Composition | Exe frame hash `235f5352…dc21`; `~/scratch/native-ab-334/pack/region-metrics.json` sha256 `a24a38f9…9070` and `~/scratch/p346/pack/region-metrics.json` sha256 `f52bf479…2e60b` both score `planet.emblem_strip` SSIM `0.729063` (gate 0.95); p346 wire/server slices equal (`~/scratch/p346/proof.json` sha256 `3bee382f…68802`) | MISMATCHED | Named divergence: our three badge faces/rings are bright (orange rings) while the exe shows dimmed rest badges. Stable across two packs and pre-#348 builds; docs/parity/planet.md still labels the rest-dim OPEN though #113 is closed. Filed #361. |
| P4. Board selection enters Channel (checked) | state-path / wire-sequence | `crates/o2jam-shell/src/screens/game/mod.rs:3877` `click_planet` → `enter(Channel)`; guards `lit_emblem_click_opens_the_channel_dialog_and_records_the_pick`, `planet_confirm_opens_the_channel_dialog`, `flow_allows_every_documented_edge`; `crates/recomp2jam/src/session_walk.rs` windowed walk | t240 `wine-drive.log`: emblem aim (136,88) then channel frame cue `server:03EA` (`docs/captures/t240/findings.md`); c1 `drive.log` same sequence; p343 `proof-final.json` sha256 `20fbe170…cff6` `wire_sequence_equal: true` and `server_slice_equal: true` | MATCHED | A/B wire order for `1002/1003/1004/1005/2000/2001/2002/2003/2010/2011/2024/3002/3003` is identical on both legs in p343. |
| P5. Carousel heartbeat survives repaints without stacking (checked) | NA-DESIGN | Guards `promo_heartbeat_pulses_while_the_board_spins`, `promo_heartbeat_parks_while_stopped`, `promo_heartbeat_parks_while_the_quit_modal_covers_the_board` (`screens/game/mod.rs:13293+`); #170 lineage in `docs/parity/planet.md`; paint path `paint_planet_promo_frame` (`screens/paint.rs`) mounts one staged layer, never stacked siblings | n/a — regression guard story; underlying cadence is P2 | NA-DESIGN | Seam/guard property, not a separate exe-visible journey. |
| P6. Native client driveable planet→channel by control id (checked) | input-response / state-path | `crates/input-drive` control-id trace; Rust leg of `~/scratch/p343-ab/` (`trace-rust.json`) and `~/scratch/p346/trace.json`, both reach `planet → channel`; `crates/recomp2jam/src/menu_trace_replay.rs` | Qualified exe leg uses SendInput pixel aim (136,88) in t240 `wine-drive.log` and c1 `drive.log`; matched wire in p343/p346 proofs (hashes above and `3bee382f…68802`) | MATCHED (state-path) | Stimuli differ by design (exe pixel aim vs Rust control id), so this is a state/wire match, not a same-trace video match. Same-trace paired-video parity stays BLOCKED under #249/#250 (capture tooling), no issue filed. |
| P7. Carousel clock guarded with named accumulator (checked) | NA-DESIGN | `PLANET_PROMO_TICK_MS` guard family in `docs/parity/planet.md` Composition; #167 lineage | n/a — test-guard story; spec already notes the missing rule-7 seam annotation | NA-DESIGN | Guard text is a repo property; no player-visible behavior to compare. |
| P8. Blue "NORMAL mode only" dialog blocks canvas clicks until OK (unchecked) | input-response / pixel-region | `crates/recomp2jam/src/relay_gate.rs:25` → `main.rs:601/1405` `stage_channel_list_after_relay_gate` → `screens/game/mod.rs:1380` hold rows, `:1290` channel-entry open, `:1398` modal owns input, `:4450` OK hit, `:1421` dismiss; live guards in `crates/recomp2jam/src/live_boot_gate.rs:683-783` | Qualified exe popup `~/scratch/p244-capture/artifacts/screenshots/message-box.png` sha256 `baf781b8caa6823b60047901e5a135d3ab77cb7fc60a7031fda02d90e408c1bf` (`citation.txt` PASS, manifest sha256 `e211cc6e8016f7e7048276e3f2fa247744efbaeca6f766a36130a1984e535b20`); t240 `wine-drive.log` `max_blue=0.986` → OK (400,365) → `0.000`; c1 `drive.log` repeats | MATCHED (state/input routing) | Exe sheet is over the Channel dialog, which is where our modal paints; non-OK canvas clicks route through `click_dispatch`'s modal arm. Residual: no explicit "non-OK click ignored" guard, and no qualified Rust popup frame. Boot-placement regression in stale p346 Rust frame maps to OPEN #348, fixed on current main by `51c2197` — not filed. |
| P9. Five dated promo event cells + NEW ITEM ribbon (unchecked) | pixel-region | Banner art staged from `Planet_Promotion.ojs` through the pinned record (`paint_planet_promo_frame`, `screens/paint.rs`); click cells at parsed `.bnd` rects (`docs/planet.md` rects 11-14) | Hashed planet frame `235f5352…dc21` shows ribbon `NEW ITEM` and cells `1일 전국 이용권 / 1일 전국이용권 무한버전(1일) / 7일 전국 이용권 / 30일 전국 이용권 / 90일 전국 이용권`; `docs/wine-boot.md` "NEW ITEM ↔ NEW MUSIC" pages | MATCHED (art inventory) | Both legs stage the same five cells and ribbon from the archive sheet. No staged-art guard enumerating the cells (spec's requested guard) and no rest/hover capture pair; old p346 Rust frame has the #348 boot dialog over the banner, fixed on main. Guard gap is not a new issue (spec story itself owns it). |

## Summary matrix — Spec: Planet state

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 6 | P1 layout, P2 cadence pin, P4 selection edge, P6 drive, P8 dialog routing, P9 promo inventory |
| MISMATCHED | 1 | P3 emblem rest-color delta |
| BLOCKED | 0 | (P2/P9 residuals are named inside their matched rows) |
| NA-DESIGN | 2 | P5, P7 (guard-only stories) |

### Live-lane BLOCKED list (no boots run here)

1. P2 — two or more timed qualified exe/Rust planet frames in one run to confirm the 3000 ms cadence (capture tooling #245/#249/#250).
2. P8 — qualified Rust popup frame under the same stimulus, plus an explicit non-OK canvas-click guard; boot-placement behavior is #348 (code fix `51c2197` already on main).
3. P9 — rest/hover capture pair naming the five event cells and ribbon.
4. P3 — fresh matched A/B on current main to confirm the emblem delta survives the #348 fix (issue #361 acceptance).

### Issue actions

- New issue #361 — "Bug: Planet emblem badges paint bright in rest state instead of exe-dimmed art" (`needs-triage`, `component:planet`, `P2`; number/title/labels read back). Body file `~/scratch/ja-a-288-planet-emblem.md`.
- No new issue for P8: OPEN #348 already owns the boot-vs-channel boundary, and the reused pack predates its fix.

### Blocker-class mappings

- P6 same-trace video parity maps to #245/#249/#250 (capture tooling), BLOCKED for the live lane.
- No #326/#355/#274/#139/#347/#345 mapping for Planet stories; none of these journeys reaches Playing/Loading/readout.
