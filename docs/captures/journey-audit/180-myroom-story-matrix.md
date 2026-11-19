# MyRoom story matrix — Spec #180 (WAVE 3, lane MyRoom)

Method: `docs/captures/story-matrix/TEMPLATE.md`; cell shape follows the
Waiting (#291) and Channel (#289) exemplars. Base `5dd2f34`; worktree
`/home/arnim/Projects/recomp2jam-wt138`, branch `cb2/story-myroom`. Docs and
local captures only; no code changes, no pushes. The 7.55 executable is the
behavior oracle (`OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`);
Memoryer is a wire witness, never a substitute for an exe pin. Capture paths
are git-ignored scratch; only hashes and text enter the repo.

## 1. Evidence inventory (before any cell was graded)

| Source | Pin (full sha256 unless noted) | What it names / verdict |
|---|---|---|
| exe identity | `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` | 7.55 behavior oracle; all static codes and rects below |
| `~/scratch/side-scenes/b8-cash` (PASS, cited exe pack) | manifest `f9d4a6953c55b75b4f04db3c0ece198bd2032361ac65dc8275ce07ccb6aa3f8b` (citation text); `artifacts/screenshots/myroom.png` `1743d36d891b4355b3f11a4a99266984159fffd911336ff4de871fd35166fae0` | default Inventory badge + empty equipment grid, empty bag/gift box, bald full-body figure with arms, wallet `G 15527` / `W 0` |
| `~/scratch/live-4/myroom-gesture/exe` (PASS, cited exe pack) | citation `a0e0fd08ef2d917d36be9e590dabce7ebebb1ca69b4214cfb3be87dba633c4a1`; `myroom.png` `5ddb5454abaa17633e0f9da73997d4e03519d51f42cbad184bbe9f9aa7371663`; STATUS `b095a915bb5b9d35e3e7fc26abbf6031bd04ad773d5023f70547149154aeb9c6`; return `c838e4373952e54eade63c140f1d13561ff7dbd0d00f1b80dd9921cdd3c7bfe3`; drive `6da49ffb6d20ac03bced888edb91a4d5bf77fc3e158eea71fad552e0c17c43bf` | panel gesture at rect `(147,307)-(207,330)` / click `(177,318)`: Inventory → STATUS → Inventory; known-stats plate for `tester` (Level 7, Exp. 296, Next Up 5001, G 15527/W 0). Default face is pixel-identical to b8 in the badge/panel regions (MAE 0.000). Citation/manifest-file hash drift recorded in §4 |
| `~/scratch/story-myroom-select/exe2` (exe frames kept, pack label mismatch) | drive `3b24e3b43fc91de9666c72b04607a53e497bfdd7249546604839e79add66f4b6`; citation `784fef60fadb900f78e6cb6a96c79f56dbcbcb28cf3de00b22700ed694b23373`; default `myroom.png` `1743d36d891b4355b3f11a4a99266984159fffd911336ff4de871fd35166fae0`; slot click `5ee043c09a9a7a12e890aa4570b363d0c552177bf61816f52c6b3bfe319ee27a`; bag click `c693cf0734dbf3996d681ac3a0b16aa022adf84b808a1ed77bb015a36cf13ad9`; gift tab `33058c8c5ee8f18bae97db405eb69147ae3fd8532fcf671c76bf195aa046b9cf`; status `2e2cc0f055ca1c273c3d3596867b6244c977bef42dff3b40f313b61949776281` | tester account, empty bag/gift box: slot and bag clicks move only the OS cursor; no selection frame on an empty cell. 12/12 distinct frames, cite PASS; pack status FAILED because scene-probe's last label is `myroom-return`, not the target `myroom-select` (a label mismatch, not a failed checkpoint) |
| `~/scratch/story-myroom-select/exe3` (PASS, cited exe pack, this wave) | citation `bf85d72f1f403bad0ce45c3f99a10be3622e35d7b3fca4f2d501ed05e1cd5498`; drive `1d1ffc4a5ee64cb8893ed333798c39282ba269a050127792955291171470c939`; default `73b560428984f761d9a1e5ef85a2186c84ec772443e106665044253810d603a1`; bag-select `b683604133f3dada75e01af79493b78e6d21559450cf763dc8f8e5b25f9d77eb`; slot-after-bag `25c5997e119f25eb313048ed8a16fee572ea57ec0fb7f8b8f82c163ccf233aa2`; bag2-select `acb8db201219e905fefb65b3c4b01b6ec2d7b698def11843e105ff998affdfc8`; gift tab `6e29e51a334b4e3eba8eb674a1b94e864040cba97531da66b7f96f30db824ed3`; STATUS `929c4817b87c65f661e799455a36a0662587d09b0f9bc56c8a8d85854f8d1bc5`; final `2628dadefffe883a190ec8d0bf963435c642486136e8c21a299b2d4c520adf87` | tester2 account with three owned items and three worn items: blue selection frame on a populated bag cell, moves to cell 2, persists across a slot press; slot press does not filter the bag; PRESENT BOX tab and panel gesture pins; tester2 plate (Level 1, Exp. 0, Next Up 500, G 626/W 0) |
| `docs/myroom.md` + `docs/parity/myroom.md` + journey audit `180-myroom.md` | exe hash as above; codes `0x15200005..0F` slots, `0x15600013..22` images, `0x15600023..2C` bag cells, `0x15500033` MYBAGSELECT, `0x15203034` INVENTORY, `0x15500035/36` SLOTCOVER/USERINFO, TEXT `0x15400010..11`/`0x37..44`/`0x5C..69` | static rect/caption pins; OPEN 1–8 (slot filter, caption sources, gift round trip, selection body) named before this matrix |
| #369 (closed) + close comments | `007c38c` fix; dossier hints `ev_b2a9…`/`ev_3fe1…`/`ev_8ea60…`/`ev_1b8cb…`/`ev_0d3d…` (abbreviated in the tracker, no full ids in repo) | default face and gesture: owner-visible divergence fixed; last residual closed by live-4 pack |
| #305 (open, two criteria MET) | `Myroom_MybagSelect.oji` slot 1305, 86x69; `Window_Cursor.ojs` slot 1303, 18x25; merge `fe0d6aa` | **correction:** orange arrows in captures are the OS cursor, not selection art; the blue archive frame is the selection body. Owner visual confirm was OPEN; this wave supplies the exe frames |
| #375 (closed) | `8de4d28`; default bare arms 30/31/35 | cross-state pin for the full-body figure in the MyRoom avatar rect (`0x15700012`) |
| #376 / #379 / #380 / #348 / #368 / #364 / #361 / #370 | fixes `35a9348` / in flight / OPEN / OPEN / `4261cd3` / `4261cd3` / `4261cd3` / `5dd8abb`; owner observations 2026-11-19 morning | Room/Channel/Planet fixes; they are named in §4's morning-fix ledger, not invented as MyRoom pixels |
| Lane D read-only gateway probe (2026-11-19, no boot) | `/home/arnim/scratch/close-offline-gift-probe.log` `db1a395d5ff126608a6b1d5d533778d422e61fc962c8d20c9ec133379f16f9f0` | Live Memoryer wire only: `C2S 5017` empty → `S2C 5018 len6 hex=000000000000` (invalid 0, i16 count 0) plus a repeat of `5028 → 5029 a73c000000000000`. No exe boot, no DB mutation. Pairs with the peer's empty present-box frame; no claim frame or 5023/5024 body exists |
| this wave's rig | scratch drivers generated by `~/scratch/story-myroom-patch-drivers.py`; Memoryer pid 92326, boot epoch 1794986056 | two exe boots (excluded preflight failure), no wire tap, 25 distinct frames; no keyboard input; NORMAL-mode popup dismissed at every transition |

## 2. Matrix (20 actions × 5 observer actors = 100 cells)

Cell key: `EVIDENCED` / `GAP` / `LIVE-FILLED` / `OPEN-FIX` / `NA-DESIGN`.
Every GAP names a runnable next-evidence step; the ids are defined here:

- `E-M` — one legitimate master-session exe MyRoom boot after the #379
  master handoff; capture first face, toggle, selection and plate against
  the peer frames.
- `E-W` — scripted TCP tap on the exe login socket around MyRoom entry;
  pair `2001 CharacterInfo` / `5017 GetGiftMessageList` bodies with the
  named frame.
- `E-S` — known-stats source pin: read the 2001 fields and map each STATUS
  rect (`0x15400037..44`, `0x1540005C..69`) to its painted caption.
- `E-P` — qualified exe bag-pager click with a >10-item account; capture
  both pages and the page digits.
- `E-Q` — qualified exe equip/vacate gesture with a selected bag cell;
  tap 5004/5005 and capture the figure before/after.
- `E-G` — qualified exe gift-list/claim boot with a gifted account; tap
  5017/5018/5023/5024 and capture the box before/after.
- `E-B` — qualified exe BACK click from MyRoom; capture the return to the
  opening lobby.

| Action (raised by) | solo | master | peer | channel | server |
|---|---|---|---|---|---|
| **ENTER-MYROOM** (self:seat holder from Room board) | NA-DESIGN — offline boot has no gateway session or MyRoom side screen | GAP — no master-session exe frame; OPEN — `E-M` | EVIDENCED — `<room MyRoom button (517,16)>` → Inventory face opens; `myroom.png` `1743d36d…` (b8) / `73b5604…` (exe3) | NA-DESIGN — board observer sees no side screen; design seam is the local `visit_side(MyRoom)` | GAP — entry queues 5017 but no live body; OPEN — `E-W` |
| **FACE-DEFAULT-INVENTORY** (self:local on entry) | NA-DESIGN — no session | GAP — OPEN — `E-M` | EVIDENCED — `<entry>` → `Inventory` badge + equipment grid, not STATUS; `1743d36d…`, `73b5604…`, live-4 `5ddb5454…` | NA-DESIGN — no wire field for the badge; local paint state | NA-DESIGN — static `#369` pin: no wire behavior bound to the face |
| **PANEL-TOGGLE-STATUS** (self:local click `(177,318)`) | NA-DESIGN — no session | GAP — OPEN — `E-M` | EVIDENCED — `<click baked STATUS / staged INVENTORY rect>` → STATUS badge + user-info plate; `b095a915…` (live-4), `929c4817…` (exe3) | NA-DESIGN — modal local gesture | NA-DESIGN — no wire frame on this click |
| **PANEL-TOGGLE-BACK** (self:local second click) | NA-DESIGN — no session | GAP — OPEN — `E-M` | EVIDENCED — `<same click again>` → Inventory badge + grid return; `c838e437…`, `2628dade…` | NA-DESIGN — local gesture | NA-DESIGN — no wire frame on this click |
| **EQUIPMENT-GRID-EMPTY** (self:server-owned empty kit) | NA-DESIGN — no server kit offline | GAP — OPEN — `E-M` | EVIDENCED — `<empty kit>` → 16 gray equipment wells; `1743d36d…` / `5ee043c0…` (tester) | NA-DESIGN — no channel view of a private grid | GAP — 2001 equipment words not tapped on exe leg; OPEN — `E-W` |
| **EQUIPMENT-GRID-WORN** (self:server-owned kit) | NA-DESIGN — no server kit offline | GAP — OPEN — `E-M` | EVIDENCED — `<tester2 kit>` → jacket/pants/shoes in the upper-right wells; `73b5604…`; full-body figure with arms pins #375 | NA-DESIGN — private grid | GAP — OPEN — `E-W` |
| **BAG-GRID-EMPTY** (self:server-owned no items) | NA-DESIGN — no inventory offline | GAP — OPEN — `E-M` | EVIDENCED — `<empty owned list>` → 10 empty MY BAG cells, page `01/03`; `1743d36d…` / `c693cf07…` | NA-DESIGN — private grid | GAP — OPEN — `E-W` |
| **BAG-GRID-OWNED** (self:server-owned 3 items) | NA-DESIGN — no inventory offline | GAP — OPEN — `E-M` | EVIDENCED — `<Bag1..3 = 376,383,35>` → jacket/head/pants occupy the first three cells in server order; `73b5604…` | NA-DESIGN — private grid | GAP — OPEN — `E-W` |
| **BAG-CELL-SELECT** (self:local click cell 1) | NA-DESIGN — no cell offline | GAP — OPEN — `E-M` | EVIDENCED — `<click (171,451) on a populated cell>` → 86x69 blue `Myroom_MybagSelect.oji` frame centered on rect `(135,423)-(208,480)`, diff bbox `(129,417)-(215,486)`; `b6836041…`. The orange arrows are `Window_Cursor.ojs`, not selection art (#305 correction) | NA-DESIGN — local paint state | NA-DESIGN — no wire field for local selection |
| **SELECTION-MOVES** (self:local click cell 2) | NA-DESIGN — no cell offline | GAP — OPEN — `E-M` | EVIDENCED — `<click (260,451)>` → same frame moves to rect `(224,423)-(297,480)`, bbox `(218,417)-(304,486)`; `acb8db20…`. On an empty bag the same click paints no frame (`c693cf07…`), so the frame needs a populated cell | NA-DESIGN — local paint state | NA-DESIGN — no wire field |
| **SLOT-PRESS-NO-FILTER** (self:local click hair selector `(507,203)`) | NA-DESIGN — no slot offline | GAP — OPEN — `E-M` | EVIDENCED — `<slot press after bag pick>` → cursor moves; all three bag rows and the cell-1 selection frame stay unchanged; `25c5997e…`. Partial answer to #180 OPEN 1: this slot does not filter the bag | NA-DESIGN — local bag state | NA-DESIGN — no wire frame for slot press |
| **GIFT-TAB-SWITCH** (self:local click `(142,395)`) | NA-DESIGN — no gift box offline | GAP — OPEN — `E-M` | EVIDENCED — `<PRESENT BOX tab>` → bottom panel becomes gift list; MY BAG tab `(72,395)` restores bag tab and flips the upper badge back to Inventory; `6e29e51a…`, `8515a7e5…` | NA-DESIGN — local tab state | NA-DESIGN — tab click sends no frame |
| **GIFT-LIST-EMPTY** (self:server 5018 with no gifts) | NA-DESIGN — no gift list offline | GAP — OPEN — `E-M` | EVIDENCED — `<gift tab on tester/tester2>` → empty present box, no claim control; `33058c8c…` / `6e29e51a…` | NA-DESIGN — private gift box | LIVE-FILLED — live gateway `C2S 5017` empty → `S2C 5018 len6 hex=000000000000` (`db1a395d…`, no boot); matches the peer's empty present-box frame. Executable-side tap and the 5023/5024 claim round trip stay OPEN — `E-G` |
| **INFO-PLATE-KNOWN-STATS** (self:local toggle to STATUS) | NA-DESIGN — no character info offline | GAP — OPEN — `E-M` | EVIDENCED — `<toggle>` → tester: Level 7 / Exp. 296 / Next Up 5001 / G 15527 / W 0 (`b095a915…`); tester2: Level 1 / Exp. 0 / Next Up 500 / G 626 / W 0 (`929c4817…`) | NA-DESIGN — private info plate | GAP — caption-to-field mapping still OPEN; OPEN — `E-S` |
| **BAG-PAGER** (self:local pager click) | NA-DESIGN — no pager offline | GAP — OPEN — `E-M` | GAP — no exe page-turn frame; OPEN — `E-P` | NA-DESIGN — local pager state | GAP — no page-index wire pin; OPEN — `E-P` |
| **EQUIP-APPLY** (self:local bag pick + INVENTORY) | NA-DESIGN — no apply offline | GAP — OPEN — `E-M` | GAP — static 5004 slot rule only; no exe press/figure frame; OPEN — `E-Q` | NA-DESIGN — equip reply is not a channel row | GAP — 5004/5005 bodies not tapped; OPEN — `E-Q` |
| **VACATE-SLOT** (self:local ITEMSELL) | NA-DESIGN — no slot offline | GAP — OPEN — `E-M` | GAP — exe gesture and send shape unpinned; OPEN — `E-Q` | NA-DESIGN — no channel view | GAP — send shape OPEN (docs/myroom.md OPEN 6); OPEN — `E-Q` |
| **GIFT-CLAIM** (self:local MOVE) | NA-DESIGN — no gift offline | GAP — OPEN — `E-M` | GAP — both captured accounts have empty gift boxes; no claim frame; OPEN — `E-G` | NA-DESIGN — no channel view | GAP — live 5024 verdict unpinned; OPEN — `E-G` |
| **VISITOR-VIEW-OTHERS-MYROOM** (other:master/peer) | NA-DESIGN — no session | NA-DESIGN — no visitor control or opcode found; design seam: `STATE_MYROOM` is a local side screen | NA-DESIGN — same boundary; no observer socket can enter another player's room | NA-DESIGN — board has no MyRoom row state | NA-DESIGN — no visitor request in the 5000 wire family; grade NA-DESIGN, not a passed story |
| **BACK-RETURN** (self:local BACK click) | NA-DESIGN — no lobby offline | GAP — OPEN — `E-M` | GAP — no exe BACK frame from MyRoom; OPEN — `E-B` | NA-DESIGN — local transition | NA-DESIGN — no wire frame for BACK |

Counts: **100 cells** — 14 EVIDENCED, 34 GAP, 1 LIVE-FILLED, 0 OPEN-FIX,
51 NA-DESIGN. Every GAP names an evidence step above. A cell is a story
candidate, not a closure; closing still needs the live caller chain and the
project gates.

## 3. Scenario scripts (simulation, bounded exe lane)

Both scripts share trace prefix `S180-MYROOM`, spec `#180`, state
`STATE_MYROOM (0x6A)`, boot pair `M = host bot uid 3 / master` and the exe
account (`tester` for 01, `tester2` for 02) as the local seat holder. `O`
(channel observer socket) was not attached; no wire claim is made.
Containment: private Xvfb :422/:423, owned null sinks
`rig_null_3476167`/`rig_null_3491712`, `route=sink`,
`on_default_sink=0`, `preboot_breach=0`, peak 0.0 dBFS, exe sha pinned,
`--no-sound` equivalent audio containment by the rig, server pid/epoch
re-read from each pack. The NORMAL-mode popup was captured and dismissed
at its OK `(400,365)` at each boot before Channel; no popup remained at
Channel→Room→MyRoom.

### S180-MYROOM-FACE-01 (boot 1, tester; exe2 pack)

Steps: seq 1 `seat holder` enters Room board (CP01, `room.png`); seq 2
`seat holder` clicks Room MyRoom `(517,16)` (CP02, `myroom.png`
`1743d36d…`); seq 3 `seat holder` clicks `(507,203)` (CP03,
`myroom-slot-select.png` `5ee043c0…`); seq 4 `seat holder` clicks bag cell
`(171,451)` (CP04, `myroom-bag-select.png` `c693cf07…`); seq 5 clicks
PRESENT BOX tab `(142,395)` (CP05, `33058c8c…`); seq 6 clicks `(177,318)`
(CP06, `2e2cc0f0…`); seq 7 clicks BACK? **not run** — this script ends at
STATUS. Cells filled: ENTER-MYROOM × peer, FACE-DEFAULT × peer,
EQUIPMENT-GRID-EMPTY × peer, BAG-GRID-EMPTY × peer, GIFT-LIST-EMPTY × peer,
PANEL-TOGGLE-STATUS × peer, plus the empty-cell negative inside
SELECTION-MOVES × peer. Wire tap id: none (GAP cells remain GAP).
Outcome sentence: "When a seat holder with an empty kit and empty bag
enters MyRoom, the peer's own screen shows the Inventory badge and empty
grids (pack `1743d36d891b4355b3f11a4a99266984159fffd911336ff4de871fd35166fae0`)."

### S180-MYROOM-SELECT-02 (boot 2, tester2; exe3 pack)

Steps: seq 1 Room board (CP11); seq 2 MyRoom entry (CP12, `73b5604…`);
seq 3 bag cell 1 click (CP13, `b6836041…`); seq 4 hair slot click (CP14,
`25c5997e…`); seq 5 bag cell 2 click (CP15, `acb8db20…`); seq 6 gift tab
(CP16, `6e29e51a…`); seq 7 panel toggle (CP17, `929c4817…`); seq 8 MY BAG
tab (CP18, `8515a7e5…`); seq 9 bag cell 1 (CP19, `5d324658…`); seq 10
panel toggle (CP20, final `2628dade…`, STATUS with gift tab open).
Cells filled: EQUIPMENT-GRID-WORN × peer, BAG-GRID-OWNED × peer,
BAG-CELL-SELECT × peer, SELECTION-MOVES × peer, SLOT-PRESS-NO-FILTER × peer,
GIFT-TAB-SWITCH × peer, INFO-PLATE-KNOWN-STATS × peer, and the negative
selection half from script 01. No wire tap was armed; all server-column
cells for these rows stay GAP. Outcome sentence: "When the local user
clicks a populated bag cell in MyRoom, the blue 86x69 archive selection
frame is centered on that cell's staged rect; clicking an empty cell paints
no frame (pack `b683604133f3dada75e01af79493b78e6d21559450cf763dc8f8e5b25f9d77eb`,
negative pack `c693cf0734dbf3996d681ac3a0b16aa022adf84b808a1ed77bb015a36cf13ad9`)."

Boot ledger: attempt 0 failed preflight (`WAYLAND_DISPLAY` leak, exit 10,
no client launched, no cell); boot 1 produced valid frames but pack status
FAILED on a target-label mismatch (documented above); boot 2 PASS. Two of
the four allowed boots are unspent. No server restart or signal; all owned
Xvfb/client/recorder processes were stopped by their own pidfiles.

## 4. Cross-cell contradictions and limits

- **Selection art correction (#305):** orange arrows in both older packs
  decode to `Window_Cursor.ojs` at the two click points (mean per-channel
  error 0.68 per `docs/myroom.md`); they are not a per-state frame. This
  wave's blue 86x69 frame on populated bag cells is the selection body and
  matches the archive sprite size. No frame was seen on an equipment slot
  press, so the "selection frame on both panel faces" part of the owner's
  visual confirm remains only partly proved: the bag face is pinned, the
  STATUS face with a *bag cell* click is confounded by the gift-tab state
  in CP19 (the bottom panel was PRESENT BOX, so the click did not land on
  a bag cell).
- **Panel/tab coupling:** in exe3 the PRESENT BOX tab leaves the upper face
  as-is, while the MY BAG tab at `(72,395)` also restores the Inventory
  badge. Treating the two tab groups as independent would be an invented
  rule; the frames are the claim.
- **Manifest hash drift:** live-4 and both new packs' citation text names
  a manifest hash that differs from the current `manifest.json` file hash
  (live-4 `df03036e…` vs `76b9a67c…`; exe2 `784fef60…` vs `0993945b…`;
  exe3 `bfcd8fb6…` vs `218d2916…`). Every cited frame hash agrees between
  citation, provenance and direct `sha256sum`, so frame pins stand and the
  drift is recorded, not silently repaired.
- **Scratch layout off-by-one:** `~/scratch/p305-art/layout.txt` pairs the
  111 `.bnd` rects with 112 ControlList entries in file order, not by
  `(code & 0xFFF)-1`; from `0x15500032` onward its labels are one entry
  behind `docs/myroom.md`. Never cite that file's labels for a rect; the
  frames and `docs/myroom.md` agree with the ordinal pairing.
- **Role gap:** no qualified exe master MyRoom frame exists. The master
  column stays GAP even though the side screen is local; a master-session
  boot is the next step (`E-M`). A synthetic 3003 master-flag rewrite like
  live-4 boot 4 is a scenario delta, not an exe master observation.
- **No visitor:** static ControlList/opcode scan found no other-player
  MyRoom entry; the 5000 family is shop/gift/equip. Visitor cells are
  `NA-DESIGN` with that seam named, not passed stories.
- **Server column:** no wire tap was armed in this wave. Lane D later added a
  bootless live probe for 5017/5018 and 5028/5029 (`db1a395d…`); it fills the
  empty-gift-list server cell only. All other server-state cells stay GAP even
  though both clients share one Memoryer; a shared server is not a scenario
  delta and is not an exe paint witness either.
- **Morning fixes ledger** (explicit, not silently mapped to MyRoom pixels):
  #369 Inventory default + gesture → rows 2–4 (EVIDENCED); #305 real blue
  frame → rows 9–10 (EVIDENCED, owner visual confirm now has exe frames);
  #375 bare arms → row 6 figure (EVIDENCED in the worn-kit frame);
  #368 Room outline, #364 Channel bar (pointer-state gap, not painter),
  #361 rest emblems, #370 population segments, #376 2005 room row,
  #379 master member visibility, #380 2007 disband, #348 relay popup
  belong to their own states and have no MyRoom cell; they are named here
  so their morning-fix stories are not lost in this spec.

## 5. Gap-to-story hand-off

Ranking by player visibility × reachability:

1. Master-session MyRoom parity (`E-M`): 19 GAP cells; one legitimate
   master boot after the #379 handoff covers the highest count.
2. Server-side source pins (`E-W`/`E-S`/`E-G`): 11 GAP cells; one tapped
   exe login with known 2001/5017/5018 data covers item grid, bag and
   plate mapping.
3. Equip/vacate/gift-claim gestures (`E-Q`/`E-G`): 9 GAP cells; a gifted
   account and a tapped 5004/5005/5023/5024 exchange are needed.
4. Bag pager and BACK (`E-P`/`E-B`): 5 GAP cells; cheap click legs with a
   >10-item account and a BACK capture.
5. Visitor view: no stories; graded NA-DESIGN with the static seam named.

The new story rows were added once to the #180 body under
`## Multi-actor stories` with their evidence class and verify step. The
existing 16 checkbox stories stay as they are; this matrix does not tick
them. Closing any story still needs the live caller chain and the project
gates, and a conformance mismatch becomes a bug, never a story edit.
