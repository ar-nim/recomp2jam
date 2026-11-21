# Tutorial story matrix — Spec #313 (WAVE 5 final, lane story-tutorial)

Method: `docs/captures/story-matrix/TEMPLATE.md`. Template and the exemplar
matrices (`178`, `179`, `290`, `315`, `293`) were read before grading. Base
`3a8b024`; worktree `/home/arnim/Projects/recomp2jam-wt146`, branch
`cb2/story-tutorial`. Docs-only change: no Rust source, no Cargo file, no push.
The 7.55 executable is the behavior oracle (`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`); Memoryer
is a wire witness, never a source of exe rules. Captures stay in git-ignored
scratch; only hashes and text enter the repo.

Actor mapping: `solo` is the offline `visit_side(Tutorial)` seam and is
`NA-DESIGN` throughout (no gateway Room chrome). `peer` is the single
authenticated seat in the qualified exe packs (`tester`, `host_user=0`); the
packs do not claim a master flag, so this column is not generalized to a room
master. `master` cells stay GAP unless a static dossier applies to every seat.
The `channel` observer cannot see a private side visit. `server` cells are
`NA-DESIGN` for purely local art; where an entry/exit frame is plausible they
are GAP because all Tutorial packs so far have `wire_tap=0`.


## 1. Evidence inventory (before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 behavior oracle for every dossier VA/file offset below |
| `~/scratch/side-scenes/b7-tutorial/` (qualified exe pack, cite `--claim exe` PASS) | citation manifest `064bec85c998dbeb5d9073f312ef949d490a4f8a2cc9d62adfccdb0c1183272d`; citation file `3472f87edf398c3cbd27a1af7f8a52e982b3ba5e063606d405bd4b07ab32f693`; on-disk `manifest.json` file hash `9002b9887be2a26be6b261a1c82494489b4336d7f5df994e8f6f5f14f270996c`; `input.log` `78067e13a4a04c51d1f84450b5bdc6f2c461e22b0f055bfa3de60441f824203f`; `tutorial.png` `612830b57103ce808b6f28739ce9bf71018223d84701516681c1a8ced10e77b8`; `audio/audio.wav` `8464ad3222e6c90c3535fb659ca1982ee9e5b70680e842771428ea6ee5b37dfb` | FIRSTSTEP aim `(689,16)` twice → tutorial frame with bubble, coach, two note bodies and baked `1/13`; chain `planet→channel→room→musicshop→tutorial`; NORMAL popup OK `(400,365)`; `wire_tap=0`; no exit leg, no key sweep. Citation-vs-file hash drift recorded; frame hashes agree |
| `~/scratch/story-tutorial-live/boot1/` (new qualified exe pack, cite `--claim exe` PASS) | citation manifest `ed51d3fda6f4a3c40d39ca07ca1242203cb5e702f7e27e86da2d2f195a6a01b4`; citation file `94dbb5e3656bb3a8ea9dc1866167dd462508f65ac01cf2305ee08b8a93eb494f`; on-disk `manifest.json` `958a7314eaa2d911b0e380a45ac01028cd360dc286e755f8819654f0d1298456`; `input.log` `cee8b7d5eb3d5df1f975d0be2086a950e9210e94fad7dc28b1115126d77f1913`; `tutorial.png` `345f0d8ddc7c8d6c50e5c454200d09aab500c6366294782ab2e148436d35b497`; `tutorial-exit-power.png` `ed1bf56efb45e01ee5f23e6f20f1d33b9f0057fc937c2c4cb93aecf8627b46cc`; `audio/audio.wav` `80a872e365c7c5d351c6d0ace5d33f9f0d86a09560b15e747b9f37cb4b97b457` | Bounded exit leg: click `(777,567)` at 14:08:28 → rig label `room`, visible Room board with no modal. Also re-pins first-page overlay at a different timestamp. 7/7 distinct frames, verify/audio/cite all PASS, `wire_tap=0`, popup probed and dismissed |
| `~/scratch/story-tutorial-live/boot2/` (new qualified exe pack, cite `--claim exe` PASS) | citation manifest `989abc137caec5cb429d98d1b0e19c094032fc1c900a350c423f6ae0e82c088f`; citation file `5ae0efec2835c6b05047a3775dc14a70326ad54e1e3c64872f6c36ea00eabaf8`; on-disk `manifest.json` `490206f61efcb97446a86f6424d9bdaa0e5aee9a764647cc0b230faaf2908f7d`; `input.log` `989c32c25f8d26b6e424cf099fc11113afca47dff93d7aec0f5577ec52da4b4d`; `tutorial.png` `33702a4d0d3f2407acb1b5ea231fad0d62de1130cfec1cf7f65d36a07aecfa8e`; `tutorial-key-s-down.png` `28d1ca44f89b536e5f9ea05816383162c28246dab23d0e174bec1fa108ccce1a`; `tutorial-key-space-down.png` `be76f41dc6d9ab49a0ed10fd9860b71af75265c6e1407bd3753bfa6f0d227f3a`; `tutorial-key-l-down.png` `d662b50583b9d32f248bf766bf0971236397e6d076a4520e898eca37c0e2f33e`; `tutorial-exit-power.png` `232db38bf9bebd3c1b7e1a19ec49d6e275373cefa22321cf98c29d4dfef8e4ed` | Seven SendInput lane keys (`s d f space j k l`, down then up) in Tutorial: each down frame paints the matching key cap yellow and changes the left-highway glow; release clears the cap. HUD tallies stay 0. 21/21 distinct frames, verify/audio/cite PASS, `wire_tap=0`. The rig's `scene_label()` printed `planet` on key frames; the pixels still show the Tutorial overlay — classifier false read, not a transition |
| `~/scratch/hover-sweep/room-btn-firststep-{offA,on}.png`, `docs/hover.md:109` | offA `74b60f142fb55385bfe00973a1001346c0f3a2c333ce3f52da545cf0c640e789`, on `365750398dceb679344708cfad034c4747d0e87c5fdbbef2fca8f3d5c2f4009e` | FIRSTSTEP hover repaints `Room_FirstStep.ojs` body frame 0 on rect `660,1-718,32`; 1679 changed px; no frame-index swap. Unqualified hover pair, but admissible frame + doc pin |
| `docs/captures/static-play/pins.md` (`S-PLAY-B3`) | dispatcher VA `0x4902bb` (file `0x8fd20`); helper `0x495110` (file `0x94110`); `ChangeState` `0x478c50` (file `0x78c50`); install switch `0x477990` (file `0x77990`); ctor `0x4f07f0`; Room dialog branch `0x4908f3`. Planet plates `0x19201005`/`0x19201006` are zeroed at construction (`0x48df50`–`0x48df8b` via setter `0x409bf0`); click handler `0x48e280` only indexes five channel families | Dossier pins the local entry chain and refutes Planet→Tutorial dispatch. Exe wins over the old parity note; contradiction recorded, not reconciled |
| `docs/parity/tutorial.md`, `docs/exe-state-inventory.md:50,95-110`, `docs/parity/planet.md:201,216`, `docs/ojn-bit26.md`, `docs/file-formats/ojn.md` | `Guide_Text.ojs` 13 records, anchor `(374,297)` 322x156; `Guide_Ani.ojs` 12 records, anchor `(645,350)` 138x183; `Playing_TutorialBG.ojs` file `0x24ad1c`; `KeyEffect_Tutorial_%d.ojs` stage `0x4dd290` (file `0xdd3e6`); `Tutorial.ojm` file `0x24b524`; `Tutorial.ojn` id 8888, 118,738 B, 43 events, 51 note payloads; 12 bit-26 kind-9 markers (B22 parser `0x172a4f`–`0x172a6a`); timing row N/A | Static/parser pins. Art anchors and chart shape are data facts; page cadence, note onsets, cue identity and judgement join stay OPEN |
| `docs/screenshots/tutorial.png` (our generated shot) | `01b6a4bfd92636e47e47f954713442f5c293cec3dfcc93a8d359cdd5d92ca24e` (re-hashed in this worktree) | Our client paints #357's frame-0 bubble/coach/counter. Never an exe claim; fix state only |
| journey audit `313-tutorial.md`; issues #357/#358 CLOSED, #326 CLOSED | audit base `2105ac6`: 4 MATCHED / 3 MISMATCHED / 7 BLOCKED / 6 NA-DESIGN; #357 close: chart-timed bodies/key guides, page/coach cadence and OJM cue remain OPEN under #326; #358 close: FIRSTSTEP pinned, Planet dead | Prior verdicts and owner/senior observations. #358 supersedes the stale Planet BLOCKED arm; the new live legs supersede the exit and key-press BLOCKED arms |

Green Rust guards (`firststep_tutorial_painter_shows_the_first_pinned_guide_frame`,
`firststep_click_from_boot_staged_room_sets_the_pinned_tutorial_globals`,
`payment_tutorial_hand_session_visits_both_and_returns`) are fix state, never an
exe observation. A shared Memoryer state is not a per-client observation.

### Next-evidence IDs

- `N-MASTER` — one qualified master-seat exe leg repeating Room FIRSTSTEP →
  Tutorial → key press → exit; name the frame hashes and input log. No
  server-state shortcut.
- `N-WIRE` — tap the exe socket across one FIRSTSTEP → Tutorial → exit leg;
  log every opcode and body, including the negative case, under one trace id.
- `N-CHART` — chart-timed replay of `Tutorial.ojn` (43 events, 51 payloads) on
  the owned null sink; pair each visible note body/key guide with its event
  time; #326 lineage.
- `N-PAGE` — capture pages 2–13 and the coach's 12 records at their event
  times; if the pages are baked art, xref the `Guide_Text.ojs` record-index
  writer in REA before any client change.
- `N-JUDGE` — REA dossier of the autoplay scheduler → judgement sink join,
  then a timed replay pairing the HUD counters with the chart event that
  scores; key presses off-note only paint key art (boot2).
- `N-AUDIO` — owned-sink A/B around entry and exit with a cue trace that names
  `Tutorial.ojm` and the host BGM resume; route snapshot and no default-sink
  stream. Boot-1 RMS only shows non-silence, not cue identity.
- `N-LOADING` — one linked Tutorial → Loading capture (side visit plus song
  start) in one pack; Loading-gate coexistence stays unclaimed without it.

## 2. Matrix (26 actions × 5 observer actors = 130 cells)

Cell key: `EVIDENCED`, `GAP`, `LIVE-FILLED`, `OPEN-FIX`, `NA-DESIGN`.
Every GAP names its next evidence step. `self:` and `other:` follow the
template. Full hashes live in §1; short hashes here are labels, not new claims.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-FIRSTSTEP** (self:peer in Room; B7 input log `(689,16)` twice) | NA-DESIGN — offline side seam has no gateway chrome | OWNER-PARKED — #521 OPEN — master FIRSTSTEP→Tutorial entry capture ; FINAL SWEEP #382 (prior GAP): GAP — `<FIRSTSTEP click>` → no master-seat Tutorial frame; OPEN — `N-MASTER` | EVIDENCED — `<click (689,16) on code `0x13200032`>` → Tutorial backdrop + bubble + `1/13`; B7 manifest `064bec85…3272d`, frame `612830b5…77b8`; boot1 frame `345f0d8d…46cc`; dossier `S-PLAY-B3` | NA-DESIGN — side visit is private; no board cell | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no tap names a first-frame opcode; OPEN — `N-WIRE` |
| **HOVER-FIRSTSTEP** (self:peer in Room) | NA-DESIGN — no offline hover claim | OWNER-PARKED — #529 OPEN — master seven-key sweep and exit click capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat hover pair; OPEN — `N-MASTER` | EVIDENCED — `<pointer onto rect 660,1-718,32>` → `Room_FirstStep.ojs` body frame 0 repaints (1679 px changed); `docs/hover.md:109`, frames `74b60f14…e789` / `36575039…009e` | NA-DESIGN — pointer state is client-local | NA-DESIGN — hover pins no wire frame |
| **ENTER-PLANET-PLATE** (self:peer, dimmed code `0x19201006`) | NA-DESIGN — offline Planet has no gateway emblem | EVIDENCED — `<click on the dimmed plate>` → no Tutorial dispatch; control is zeroed at construction and its handler only selects a channel family → dossier `S-PLAY-B3` (`0x48df50`–`0x48df8b`, `0x48e280`), static hover `docs/hover.md:90` | EVIDENCED — same dead control for this seat; B7 never clicked it and the static pin needs no new pixels (`docs/parity/tutorial.md` Input row) | NA-DESIGN — a local plate press has no board row | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no tap proves the absence of a frame; OPEN — `N-WIRE` |
| **RETURN-FROM-MUSICSHOP** (self:peer, side BACK `(757,580)`) | NA-DESIGN — no offline MusicShop leg | OWNER-PARKED — #525 OPEN — master first-page bubble/coach/key row capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat shop-back frame; OPEN — `N-MASTER` | EVIDENCED — `<BACK (757,580)>` → Room board returns before FIRSTSTEP is clicked; B7 `input.log` `78067e13…203f`, room frame `cf3d6b8e…7407`; boot1 repeats the same chain | NA-DESIGN — private side visit | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no opcode paired with side BACK; OPEN — `N-WIRE` |
| **CHART-GLOBALS-INSTALL** (state-path in helper `0x495110`) | NA-DESIGN — offline seam installs no exe globals | EVIDENCED — dossier applies to any logged-in seat: song 8888 + eight globals, then `ChangeState(0xb)`; `S-PLAY-B3`, files `0x94110`, `0x78c50` | EVIDENCED — B7 live entry reaches the same state; dossier `0x495137`–`0x4951bd` names the exact global writes | NA-DESIGN — client-local state | NA-DESIGN — no wire shape claimed for local globals |
| **BACKDROP-PAINT** (state-path, `Playing_TutorialBG.ojs`) | NA-DESIGN — offline shot is not exe evidence | OWNER-PARKED — #525 OPEN — master first-page bubble/coach/key row capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat Tutorial frame; OPEN — `N-MASTER` | EVIDENCED — `<entry>` → full-canvas keyboard/hands/wood-floor backdrop; B7 frame `612830b5…77b8`, boot1 `345f0d8d…46cc`, boot2 `33702a4d…cfa8e`; static file `0x24ad1c` | NA-DESIGN — private canvas | NA-DESIGN — backdrop paint is local |
| **GUIDE-BUBBLE-1OF13** (state-path, `Guide_Text.ojs` record 0) | NA-DESIGN — no exe bubble offline | OWNER-PARKED — #525 OPEN — master first-page bubble/coach/key row capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat bubble frame; OPEN — `N-MASTER` | EVIDENCED — `<entry>` → Korean lesson panel with baked `1/13` at anchor `(374,297)` 322x156; B7 frame `612830b5…77b8`, new pack `345f0d8d…46cc`; our fixed shot `01b6a4bf…24e` is not the claim | NA-DESIGN — private canvas | NA-DESIGN — page art is local |
| **COACH-PAINT** (state-path, `Guide_Ani.ojs` record 0) | NA-DESIGN — no exe coach offline | OWNER-PARKED — #525 OPEN — master first-page bubble/coach/key row capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat coach frame; OPEN — `N-MASTER` | EVIDENCED — `<entry>` → coach figure at anchor `(645,350)` 138x183; same pack frames; #357 staged record 0 at the parsed anchor | NA-DESIGN — private canvas | NA-DESIGN — coach art is local |
| **RECEPTOR-ROW** (state-path, `KeyEffect_Tutorial_%d.ojs`) | NA-DESIGN — offline key row is a shot seam | OWNER-PARKED — #525 OPEN — master first-page bubble/coach/key row capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat receptor frame; OPEN — `N-MASTER` | EVIDENCED — `<entry>` → bottom `S D F SPACE J K L` row under the hands, row 487; B7 frame + stage dossier `0x4dd290` (file `0xdd3e6`), `docs/bnd-audit.md` row 7 | NA-DESIGN — private canvas | NA-DESIGN — row paint is local |
| **KEY-CAPS-ON-PRESS** (self:peer, `s d f space j k l` down/up in Tutorial) | NA-DESIGN — no offline key response claim | OWNER-PARKED — #529 OPEN — master seven-key sweep and exit click capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat key sweep; OPEN — `N-MASTER` | OPEN-FIX — `<lane key down>` → matching key cap turns yellow and the left-highway glow changes; `<up>` → cap clears. boot2 `28d1ca44…1a` (s), `be76f41d…3a` (space), `d662b505…3e` (l), input `989c32c2…4d`. Our painter holds guide frame 0 and responds to no key; fix parked under #326; next step `N-JUDGE` seam join plus `N-MASTER` | NA-DESIGN — key light is client-local | NA-DESIGN — no key opcode pinned |
| **KEY-JUDGEMENT** (self:peer, off-note key press) | NA-DESIGN — no offline judgement journey | OWNER-PARKED — #516 OPEN — off-note key press leaves judgement tallies at zero ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat tally frame; OPEN — `N-MASTER` | P3-PARKED — #699 OPEN — DirectInput interposer foundation for judged-input parity; owns #625/#649/#653 and the Tutorial N-JUDGE arms — #516 OPEN — off-note key press leaves judgement tallies at zero; N-JUDGE needs the DirectInput interposer ; FINAL SWEEP #382 (prior GAP): GAP — `<off-note key>` → HUD `COOL/BAD/GOOD/MISS` stays 0 in every boot2 key frame; no chart-timed judged Tutorial frame exists. OPEN — `N-JUDGE` | NA-DESIGN — private HUD | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no tutorial 4014/4016 shape pinned; OPEN — `N-WIRE` |
| **NOTE-BODIES-ONSET** (state-path, chart-timed note bodies/key guides) | NA-DESIGN — chart replay is not an offline seam | OWNER-PARKED — #549 OPEN — chart-timed note bodies and key guides pinned ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat timed replay; OPEN — `N-MASTER` | OPEN-FIX — `<entry frame>` → two note bodies visible in the left highway (B7/new pack), but their onsets and key-guide cadence are unpinned; our client paints no chart-timed bodies. Fix parked under #326; OPEN — `N-CHART` | NA-DESIGN — private highway | NA-DESIGN — onset timing is client-local |
| **PAGE-CADENCE-2TO13** (state-path, 13 baked `Guide_Text.ojs` pages) | NA-DESIGN — no offline page sequence | OWNER-PARKED — #542 OPEN — pages 2–13 of the lesson captured ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat page frames; OPEN — `N-MASTER` | OPEN-FIX — `<entry>` → only page `1/13` is pinned; 12 later records are staged but never observed in the exe, and our client intentionally holds record 0. OPEN — `N-PAGE` before any record-index change | NA-DESIGN — private lesson text | NA-DESIGN — page index is local |
| **COACH-CADENCE** (state-path, 12 `Guide_Ani.ojs` records) | NA-DESIGN — no offline animation claim | OWNER-PARKED — #546 OPEN — coach's 12 records tied to cadence ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat animation frames; OPEN — `N-MASTER` | OWNER-PARKED — #546 OPEN — coach's 12 records tied to cadence ; FINAL SWEEP #382 (prior GAP): GAP — `<time in lesson>` → only record 0 is pinned; boot2 up-frames show small pose changes but no record index or cadence, so no page/animation switch is claimed. OPEN — `N-PAGE` | NA-DESIGN — private animation | NA-DESIGN — animation index is local |
| **TUTORIAL-OMJ-CUE** (state-path, `Tutorial.ojm` file `0x24b524`) | NA-DESIGN — no offline audio claim | OWNER-PARKED — #552 OPEN — Tutorial.ojm cue at the pinned moment ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat owned-sink A/B; OPEN — `N-MASTER` | OWNER-PARKED — #552 OPEN — Tutorial.ojm cue at the pinned moment ; FINAL SWEEP #382 (prior GAP): GAP — `<entry>` → boot1 owned sink is non-silent around the visit (2 s RMS silence only at 98–102 s, active 102–112 s), but no sample or moment is identified as the OJM cue; `wire_tap=0`. OPEN — `N-AUDIO` | NA-DESIGN — private audio | NA-DESIGN — no cue opcode pinned |
| **EXIT-TO-ROOM** (self:peer, bottom-right chrome `(777,567)`) | NA-DESIGN — offline `leave_side` is a seam, not an exe click | OWNER-PARKED — #529 OPEN — master seven-key sweep and exit click capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat exit frame; OPEN — `N-MASTER` | EVIDENCED — `<click (777,567)>` → next frame is the Room board with no modal; boot1 input `cee8b7d5…1913` / exit `ed1bf56e…7b46`, boot2 input `989c32c2…4d` / exit `232db38b…8e4e`; rig label `room`. B7 had no exit leg | NA-DESIGN — private side visit | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no exit opcode; OPEN — `N-WIRE` |
| **HOST-BGM-RE-CUE** (state-path on exit) | NA-DESIGN — no offline BGM claim | OWNER-PARKED — #555 OPEN — host BGM re-cue on exit with its own moment ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat audio A/B; OPEN — `N-MASTER` | OWNER-PARKED — #555 OPEN — host BGM re-cue on exit with its own moment ; FINAL SWEEP #382 (prior GAP): GAP — `<exit>` → boot1 audio resumes after the 98–102 s gap but the cue source and re-cue moment are unpinned; `docs/smoke/payment-tutorial-hand-session.md` is our seam, not exe audio. OPEN — `N-AUDIO` | NA-DESIGN — private audio | NA-DESIGN — no BGM opcode pinned |
| **NO-TIMER** (state-path, `docs/parity/tutorial.md` Timing row) | NA-DESIGN — no timer design seam | NA-DESIGN — no independent timer behavior claimed for any seat | NA-DESIGN — static parity row is N/A; chart timing is a separate row, not an invented wall-clock timer | NA-DESIGN — no board timer | NA-DESIGN — no wire timer |
| **ART-RECT-DATA** (state-path, parsed anchors) | NA-DESIGN — synthetic placement test is not exe geometry | OWNER-PARKED — #525 OPEN — master first-page bubble/coach/key row capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-seat art frame; OPEN — `N-MASTER` | EVIDENCED — `<entry>` → backdrop plus both guide bodies ink at parsed anchors `(374,297)` / `(645,350)`; dossier `docs/parity/tutorial.md`, pack frame `345f0d8d…46cc`; no literal fallback in the painter | NA-DESIGN — geometry is client-local | NA-DESIGN — rects are not wire state |
| **NO-INVENTED-DISPATCH** (state-path, entry boundary) | NA-DESIGN — seam guard `planet_tutorial_plate_next_step_names_only_the_dead_plate` is test-only | NA-DESIGN — design rule, not a separate master behavior | EVIDENCED — `<Planet plate press>` → no dispatch; FIRSTSTEP is the only live entry, pinned by B7 plus dossier `S-PLAY-B3`. No invented entry ships; the dead arm stays documented | NA-DESIGN — caller chain is local | NA-DESIGN — no invented opcode |
| **CHART-FORMAT-43-51** (data-effect, `Tutorial.ojn`) | NA-DESIGN — parser/test fact, no runtime journey | NA-DESIGN — same parser fact for every seat | NA-DESIGN — 118,738 B, id 8888, 43 events / 51 payloads are `docs/file-formats/ojn.md` data facts; no player-visible observation added | NA-DESIGN — board cannot see chart parse | NA-DESIGN — no wire shape |
| **BIT26-12-BGM-MARKERS** (data-effect, kind-9 markers) | NA-DESIGN — parser/test fact | NA-DESIGN — same parser fact | NA-DESIGN — 12 bit-26 autoplay markers are B22 sample ids (`0x172a4f`–`0x172a6a`), not hold flags; `docs/ojn-bit26.md`. Mechanism stays a static pin until `N-JUDGE` | NA-DESIGN — board cannot see parser flags | NA-DESIGN — no wire shape |
| **CI-SKIP-WITHOUT-DATA** (test seam) | NA-DESIGN — test-only skip guard; no exe claim for any actor | NA-DESIGN — test-only skip guard | NA-DESIGN — `staging/tests.rs` / shots guard skip cleanly with `install-root` absent; seam suffices because no behavior is claimed | NA-DESIGN — test-only invariant | NA-DESIGN — test-only invariant |
| **LIVE-CALLER-CHAIN** (state-path, boot → Room → FIRSTSTEP → Tutorial) | NA-DESIGN — offline boot has no gateway chain | OWNER-PARKED — #521 OPEN — master FIRSTSTEP→Tutorial entry capture ; FINAL SWEEP #382 (prior GAP): GAP — no master-session chain frame; OPEN — `N-MASTER` | EVIDENCED — B7 chain `planet→channel→room→musicshop→tutorial`, input `78067e13…203f`, plus dossier `0x4902bb → 0x495110 → 0x478c50 → 0x4f07f0`; boot1 repeats it and adds the exit | NA-DESIGN — caller chain is local | NA-DESIGN — no wire claim without a tap |
| **LOADING-COEXIST** (state-path, Tutorial side visit plus Loading gate) | NA-DESIGN — no offline song start | OWNER-PARKED — #558 OPEN — Tutorial and Loading coexistence frame ; FINAL SWEEP #382 (prior GAP): GAP — no master-session joint frame; OPEN — `N-MASTER` | OWNER-PARKED — #558 OPEN — Tutorial and Loading coexistence frame ; FINAL SWEEP #382 (prior GAP): GAP — `<song start while the side visit exists>` → no qualified pack visits both; our hand-session guard is a seam. OPEN — `N-LOADING` | NA-DESIGN — private scene pair | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no start opcode paired with a Tutorial frame; OPEN — `N-WIRE` |
| **WIRE-INVENTORY** (self:any, whole Tutorial leg) | NA-DESIGN — offline boot has no gateway wire | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no master-session tap; OPEN — `N-MASTER` | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — `<whole FIRSTSTEP→Tutorial→exit leg>` → all three packs declare `wire_tap=0`; no Tutorial opcode inventory exists. OPEN — `N-WIRE` | NA-DESIGN — inventory produces no board frame | OWNER-PARKED — #539 OPEN — one wire tap across FIRSTSTEP→Tutorial→exit ; FINAL SWEEP #382 (prior GAP): GAP — no opcode/body table pinned; OPEN — `N-WIRE` |

Counts after #382 final sweep: **130 cells** — 15 EVIDENCED, 3 OPEN-FIX, 80 NA-DESIGN, 31 OWNER-PARKED, 1 P3-PARKED. Every disposition names a pack/pin/ticket or an owner item.

## 3. Scenario scripts (executed and planned)

| Field | `S313-TUTORIAL-EXIT-01` (executed, boot1) | `S313-TUTORIAL-KEYS-02` (executed, boot2) |
|---|---|---|
| `spec` / `state` | `#313` / `CStateTutorial` side visit, host Room | `#313` / same |
| `trace_id` | `S313-B1-TESTER` | `S313-B2-TESTER` |
| `boot_pair` | `P = tester / single authenticated seat, host_user=0`; no second client, no host bot | same |
| `containment` | private Xvfb `:436`, owned null sink `rig_null_3786618`, `route=sink`, `on_default_sink=0`, `preboot_breach=0`, no wire tap, Memoryer pid 92326 / epoch 1794986056 untouched | private Xvfb `:437`, owned null sink, same route/lane discipline, no wire tap, same server epoch |
| `steps` | 01 boot → dismiss NORMAL popup → Channel enter → Room → MusicShop BACK → FIRSTSTEP `(689,16)` → 02 exit chrome `(777,567)` | same walk, then 03–16: send `s d f space j k l` down/up → 17 exit chrome |
| `checkpoint` | CP01 `tutorial.png` `345f0d8d…46cc`; CP02 `tutorial-exit-power.png` `ed1bf56e…7b46`, input `cee8b7d5…1913`, no wire | CP01 `tutorial.png` `33702a4d…cfa8e`; CP02 14 key frames (down/up per lane); CP03 exit `232db38b…8e4e`, input `989c32c2…4d`, no wire |
| `outcome` | exit click returns the Room board; no modal | each down frame lights the matching key cap; release clears it; tallies stay 0; exit returns Room |
| `deviation` | popup probed at every transition; NORMAL box captured `(400,365)` and dismissed on boot; no focus loss | rig `scene_label()` false-read `planet` on key frames; pixels remain Tutorial, so no transition was logged |

Planned only (fills no cell): `S313-TUTORIAL-MASTER-03` uses `N-MASTER`;
`S313-TUTORIAL-WIRE-04` uses `N-WIRE`; `S313-TUTORIAL-CHART-05` uses
`N-CHART` / `N-PAGE` / `N-AUDIO`. No planned script has a frame hash.

## 4. Boot ledger (cap of 3 client boots; 2 spent)

| Boot | Display / sink | Actors | Scene | Wire tap | Result |
|---|---|---|---|---|---|
| attempt 0 | none | none | none | none | **No client launched.** `rig-wine-run.sh` has no `--no-host` option (rc=2). No cell filled |
| attempt 1 | none | none | none | none | **No client launched.** Preflight refused the worktree's missing `install-root` (all four data slots claimed). A private reflink copy under `~/scratch/story-tutorial-data/` was used from then on; no shared slot touched |
| boot 1 | `:436` / `rig_null_3786618` | `tester` single seat | boot → Planet → Channel → Room → MusicShop → Tutorial → Room | none | PASS: verify 0, audio 0, cite 0; 7/7 distinct frames; exit pin filled; no default-sink stream; popup dismissed |
| boot 2 | `:437` / owned null sink | `tester` single seat | same walk plus seven key down/up pairs and exit | none | PASS: verify 0, audio 0, cite 0; 21/21 distinct frames; key-effects pin filled; classifier deviation recorded |

Preflight polled `ss -tnp | grep 15010` and `ss -ltnp | grep 15010` before
each launch; one lane at a time, shared with the dialogs and payment lanes.
Memoryer was never restarted or signalled. The scratch driver patch
(`~/scratch/story-tutorial-patch-*.py`) restored the tracked `wine-scene.py`
after each boot; `git status` shows only the intended docs. No copyrighted
bytes were added to the repo. Boot 3 is unspent: the remaining cells need a
master seat, a wire tap, a chart/cue trace or a REA dossier, not another pixel
boot.

## 5. Count re-parse and story hand-off

Statuses were re-parsed from §2 one token per intersection: **130 cells —
15 EVIDENCED, 32 GAP, 0 LIVE-FILLED, 3 OPEN-FIX, 80 NA-DESIGN**. EVIDENCED
cells carry pack hashes, dossier VAs or
measured frame pairs; every GAP names one `N-*` step. OPEN-FIX cells name the
exe pin and the parked client fix; they do not close a story. The spec body
gets one logged edit under `## Multi-actor stories (WAVE 5, #381 story
matrix)`, with the original 20 stories untouched, and is read back before the
#381 completion comment.
