# Journey audit — #313 Spec: Tutorial state

Base: `2105ac6`. Method: story checklist from `gh issue view 313`; comparator
chosen per story; our journey traced in `crates/`; exe journey reused from
qualified packs only. No exe boot and no product change in this leg.

## Evidence anchors (artifact + hash)

| Id | Artifact | Hash / provenance |
|---|---|---|
| `B7` | `~/scratch/side-scenes/b7-tutorial/` (exe 7.55, `83a04541…72c823b`), `artifacts/input.log`, scene sequence `planet→channel→room→musicshop→tutorial` | manifest sha256 `064bec85c998dbeb5d9073f312ef949d490a4f8a2cc9d62adfccdb0c1183272d`; `tutorial.png` sha256 `612830b57103ce808b6f28739ce9bf71018223d84701516681c1a8ced10e77b8`; `audio/audio.wav` sha256 `8464ad3222e6c90c3535fb659ca1982ee9e5b70680e842771428ea6ee5b37dfb` |
| `B7-AIM` | `B7/artifacts/input.log` FIRSTSTEP aim `(689,16)`, two clicks, `cue=click:room-firststep` | aim image `i03-firststep.png` sha256 `2b1deb31a32de95df5b77520fd4f8a135f05c404820bf93e96852076b6c294ed` (recorded in the same input log) |
| `R-TUT` | `docs/screenshots/tutorial.png` (our generated file-native shot) | sha256 `cef3f966503d14ab0c9e6a99bbabbaa7ce0e0fdcd034e968f122bcb719aa40d0` |
| `STATIC` | `docs/parity/tutorial.md`, `docs/planet.md:84-125,200-216`, `docs/ojn-bit26.md`, `docs/file-formats/ojn.md` | repo files at `2105ac6` |

Our journey in code: boot → `ROOM_CONTROLS` dispatch at
`crates/o2jam-shell/src/screens/game/mod.rs:229` → `visit_side(Tutorial)` →
staging at `crates/o2jam-shell/src/staging.rs:259` and
`crates/o2jam-shell/src/screens/side.rs:4314` → painter/shot at
`crates/recomp2jam/src/shots.rs:300`. Exe journey in `B7` reaches tutorial only
from Room FIRSTSTEP after a MusicShop back; the Planet plate is never clicked.

## Summary matrix

| Verdict | Count | Stories |
|---|---:|---|
| MATCHED | 4 | 1, 3 (backdrop sprite only), 4, 16 |
| MISMATCHED | 3 | 3 (live scene content), 8, 19 |
| BLOCKED | 7 | 2, 7, 9, 10, 11, 17, 20 |
| NA-DESIGN | 6 | 5, 6, 12, 13, 15, 18 |

New issues: #357 (Tutorial scene lacks exe instruction bubble and live chart
overlays), #358 (Tutorial parity note still marks FIRSTSTEP entry unpinned).
Mappings (recorded, not filed): stories 8/9/11/20 chart-judgement and cue
timing → **#326** (Playing evidence wedge); stories 10/17 → **#245/#249/#250**
(capture tooling); story 19's stale doc note is novel and is filed.

Post-audit disposition: #358 replaced `TUTORIAL_ENTRY_NEXT_STEP` with
`PLANET_TUTORIAL_PLATE_NEXT_STEP` and wired the Room FIRSTSTEP helper
globals; story 19's stale parity note is rewritten, and story 13's
condition is refuted by `S-PLAY-B3`. The Planet plate entry stays
BLOCKED (and is statically dead); chart/audio joins stay under #326.

BLOCKED for live lane: Planet TUTORIAL plate entry sweep; Tutorial.ojm cue
moment and BGM re-cue; key-effect press sweep in tutorial; qualified exit
capture; both-entry qualified sweep (Planet half); Loading-gate coexistence.

## Per-story table

| # | Story (short) | Comparator | Our journey ref | Exe evidence | Verdict | Notes |
|---|---|---|---|---|---|---|
| 1 | Room FIRSTSTEP entry | input-response | `game/mod.rs:229` routed `0x13200032→Side(Tutorial)`; `staged_firststep_button_enters_the_tutorial_flow` (`:20568`) | `B7-AIM` click `(689,16)` → `scene-sequence.txt` ends `tutorial`; `tutorial.png` hash in `B7` | MATCHED | Journey shape matches: room chrome click opens guided-play side visit; our seam test asserts the same state. Content divergence is story 3/8. |
| 2 | Dimmed Planet TUTORIAL plate entry | input-response | `side.rs:2574` FREE emblem P0; `paint.rs:1906` static plate; `planet_side_and_tutorial_clicks_land_nowhere` (`game/mod.rs:17566`) | `STATIC` (`docs/planet.md:200` code `0x19201006` static, 0 changed px); no qualified click to this plate | BLOCKED | No exe dispatch pin; needs one qualified SendInput sweep of the dimmed plate. Live-lane blocker #245 class. |
| 3 | Full-canvas backdrop | pixel-region | `staging.rs:259` + `side.rs:4314` stage `Playing_TutorialBG.ojs`; `R-TUT` shows keyboard/hands/wood floor | `B7` `tutorial.png` (hash above) shows the same backdrop, plus a Korean instruction bubble, coach figure, live notes and `1/13` | MISMATCHED | Backdrop sprite matches; live scene content does not. Divergence in story 8 row; issue filed there. |
| 4 | Seven receptor kits at row 487 | pixel-region | `paint.rs` shot path draws staged receptor row; `R-TUT` bottom keys `S D F space J K L` | `B7` `tutorial.png` same bottom key row under the hands | MATCHED | Pixel-region read of the pinned row; no invented geometry in our paint path. |
| 5 | Old-format 43 events / 51 payloads | NA-DESIGN (parser/test) | `o2jam-file/src/ojn.rs:1036` real-header test; `ojn.rs:1150` tutorial-shaped level walk | `STATIC` (`docs/file-formats/ojn.md` 118,738 B / id 8888) | NA-DESIGN | Format-parsing fact; test-verified, no runtime journey to compare. |
| 6 | 12 bit-26 BGM markers | NA-DESIGN (parser/test) | `o2jam-file/tests/ojn_bit26_sweep.rs:83` asserts 12 hits on Tutorial kind-9 records; `o2jam-game/src/lib.rs:60` autoplay scheduler | `STATIC` (`docs/ojn-bit26.md`, B22 sites `0x172a4f`–`0x172a6a`) | NA-DESIGN | Parser/test-verified marker semantics; no invented hold flags. |
| 7 | Tutorial.ojm cue at pinned moment | data-effect / wire-sequence | `side.rs:175` tutorial scene cue is `BGM_SILENCE_ID`; no chart OJM loader on the side visit; `main.rs:2566` guard | `B7` `audio/audio.wav` hash above is non-silent, but no cue moment is pinned by the pack | BLOCKED | Need a qualified audio/wire trace naming the OJM cue moment. Mapped to #326 ( Playing wedge) for the cue-timing half. |
| 8 | Autoplay drives judgement | pixel-region + state-path | `o2jam-game/src/lib.rs:60` scheduler exists but is not joined to `SideScreen::Tutorial`; `R-TUT` HUD has zeroed counters, no instruction bubble | `B7` `tutorial.png` shows instruction bubble, coach figure, note highway content and `1/13`; HUD counters present | MISMATCHED | Named divergence: live instruction/notes overlay absent from our tutorial. Chart timing half → #326; bubble/notes are novel → #357. |
| 9 | Key presses during autoplay give effects | input-response | `planet_side_and_tutorial_clicks_land_nowhere` (`game/mod.rs:17566`); `lane_effect.rs` effects only in Playing | `B7` input log has no key injection while in tutorial; no qualified press capture | BLOCKED | Need one qualified key sweep in tutorial. Timing/effect class notes #326; not filed. |
| 10 | Leaving Tutorial returns host lobby | state-path | `staged_firststep_button_enters_the_tutorial_flow` asserts `leave_side()` returns to `Screen::Room` | `B7` stops in tutorial; no qualified dismissal capture | BLOCKED | Need one qualified before/after capture of exit. Capture-tooling class #245/#249/#250. |
| 11 | Host BGM re-cued on return | data-effect | `side.rs:175` scene cue `BGM_SILENCE_ID`; no re-cue on `leave_side` test | `B7` has no return leg; audio hash above covers entry only | BLOCKED | Needs the qualified exit+audio capture from story 10. Timing class #326 noted; not filed. |
| 12 | No timed behavior beyond chart | NA-DESIGN (static parity) | `docs/parity/tutorial.md` Timing row N/A | `STATIC` same row | NA-DESIGN | No timer journey exists on either side. |
| 13 | Refuse entry with no dispatch pin | NA-DESIGN (premise refuted) | `TUTORIAL_ENTRY_NEXT_STEP` (`side.rs:4053`) still describes an unpinned entry | `B7-AIM` pins the Room dispatch by live click | NA-DESIGN | Story's condition no longer holds; keep the constant as stale text until doc issue (story 19) lands. |
| 14 | All art at parsed rects | state-path / data-effect | `staging/tests.rs:321` synthetic stage test; `shots.rs:2485` archive-present staging guard | `B7` `tutorial.png` art sits at the exe rects | MATCHED | Staging path and live art agree at the backdrop/key-row level. |
| 15 | CI skip without install-root | NA-DESIGN (test-only) | `staging/tests.rs:321-334`; shots guard skip | n/a | NA-DESIGN | CI behavior; no player journey. |
| 16 | One live caller chain from boot | state-path | `ROOM_CONTROLS` table is reached from the boot input seam; `live_boot_gate.rs:125` covers staged art; test at `game/mod.rs:20568` | `B7` full chain `planet→channel→room→(musicshop)→tutorial` in `scene-sequence.txt` | MATCHED | Both chains reach the state; Planet alternate entry remains story 2. |
| 17 | Qualified sweep of both entry controls | input-response | FIRSTSTEP covered by `B7-AIM`; Planet plate has no sweep | `B7-AIM` one control only | BLOCKED | Planet half of the sweep missing; #245/#249/#250 capture-tooling class. |
| 18 | Every OPEN arm names next step | NA-DESIGN (docs) | `side.rs:4053`, `docs/parity/tutorial.md` Input row | `STATIC` next-step text | NA-DESIGN | Documentation/reviewer story. |
| 19 | FIRSTSTEP contradiction recorded | NA-DESIGN? → MISMATCHED | `docs/parity/tutorial.md` Input row still says OPEN; `side.rs:4053` still says unpinned | `B7-AIM` plus `B7` `tutorial.png` pin the dispatch as live | MISMATCHED | Named divergence: our record contradicts the qualified exe pack. Novel doc issue #358. |
| 20 | Tutorial coexists with Loading gate | state-path | side visit and Loading gate share `ScreenGame`; no joint guard | no qualified pack visits both | BLOCKED | Needs a linked tutorial→loading capture; #326 (Loading gate) and #347 lineage noted, not filed. |
