# Playing right-side stage pipeline (BGA) — 7.55

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`;
`Playing1.opi` 221,737,026 B (SHA-256 `8c1c7566…aed9eed40`), `Interface1.opi`
75,100,702 B. File offset = VA − `0x400000`. Method: read-only byte walks with
Python 3.14 over the installed archives plus read-only objdump static analysis
of the exe; **no REA dossier IDs issued** (Ghidra projects in `/tmp` were empty
and the exe was not re-analyzed headless — same precedent as the strike-slot
verdicts in `docs/file-formats/ojm.md`). No disassembly reproduced.

## Answer

The right side of the Playing scene is a **stage backdrop chosen in the
waiting room, not per-song BGA**. The Playing control list binds one entry,
`IDC_IMAGE_SPLAY_BACK1` code `0x1B500071`, to the sprite `_Playing_BG.ojs`
(CP949 comment "공연장 그림들", stage pictures; `ControlList_Playing.txt`,
slot 632 of `Playing1.opi`, file offset 221604482, 16577 B). The playing-scene
applier `FUN_004e7850` (VA `0x4e7850`) strips the `_` prefix and extension and
appends the **stage number** held in global `0x7789dc`, fetching
`Playing_BG<stage>.ojs` from `Playing1.opi`. The stage number (1..36) is picked
and cycled by room-side code and confirmed to the server with protocol 4002.
It renders once, static for the whole song, at the sprite anchor `(204,0)`,
596x558 — the whole right area above the bottom strip. Avatars, shadows,
nickname plates and gauges (entries `0x1B7xxxxx`/`0x1B5xxxxx`) paint over it.
A song with "no BGA" does not exist: OJM files carry audio only, the OJN's
embedded JPEG is select-screen album art, and the stage family always exists,
so the backdrop always shows.

## Source art (archive side)

- `Playing_BG.ojs` + `Playing_BG1..36.ojs` (`Playing1.opi` slots 54, 55–90,
  436): 37 files, each exactly 665164 B, one raw RGB555 frame, anchor `(204,0)`,
  596x558, colorkey `0x0000` — covers x 204..800, y 0..558 (the right side plus
  the area behind the highway's right edge). Painted in file order before
  `IDC_IMAGE_SPLAY_BACK2` `Note_BG.ojs` (2 frames, anchors `(20,0)` and
  `(567,0)`, 213x484 — the highway back strips) and `IDC_IMAGE_SPLAY_BACK3`
  `Playing_Menu.ojs` (`(204,558)`, 596x42 — bottom-right menu strip).
- `Playing_BG19Ani.ojs` (slot 66, 1151464 B at data offset 18418794):
  4-frame animation 498x289 at `(286,0)`, colorkey `0x7D5F` (header words
  `(1, 1365, 4, 0x7D5F, 286, 0, 498, 289)`) — the only animated stage
  variant found, and an archive orphan in 7.55: no exe literal names it
  (byte search finds no `BG19Ani`, `BG19`, `Playing_BG`, or `BG_Ani`), no
  `.txt` in either archive references it, and no traced builder can form
  its name (close-out in OPEN 1).
- Stage-numbered effect kits share the same key: `Note_{Miss,Bad,Good,Cool,
  Combo,ComboNum}<1..36>.ojt`, `Note_Click<1..36>_<1|2>.ojt`,
  `Playing_Effect_Jam<1..36>.ojt` — the whole visual kit is stage-keyed.

## Stage selection (room side)

- Global `0x7789dc` = the stage value; `0x7789e0` = click-effect variant;
  neighbours `0x7789d8` (selected-song id: song-select store file `0x3f379`,
  init default `0x22b8` = 8888 "none" file `0x95137`, sentinel compares files
  `0xd8806`/`0x1719c0`; the picker compares it as song ids against `0x28e`/
  `0x28f` = 654/655, both present in the 562-record `OJNList.dat` walk) and
  `0x7789e4`..`0x7789e7` (key mode/difficulty bytes) are the room-setup block.
  Flag overlay on the same dword: file `0x15caa8` ORs a `0xF000`-masked word
  shifted to the high half into `0x7789d8`, so masked readers (`& 0x0FFFFFFF`
  at file `0x1719d6`) stay robust while the BPM full-dword compare
  (file `0xbe0d2`) matches only when the overlay is clear.
- Picker `0x4bb2c0`: reads the waiting-room preview control `0x1650001A`
  (`IDC_IMAGE_SWAIT_MAP` = `Wait_Map.ojs` in `State_Waiting`), returns a stage
  value with bit 31 (`0x80000000`) always set (close-out in OPEN 4).
  The normal path rolls a random stage (seeded `rand`, mod the
  control count, plus one) and re-rolls past a 4-entry avoid list at VA
  `0x649ac4` (file `0x249ac4`). Songs 654/655 bypass the roll: two
  immediate compares on the selected-song id (files `0xbb324`/`0xbb330`)
  store stage 4 directly and return it with the high-word flag set (file
  `0xbb3cf`), which the room setter and the `_` resolver read as stage 4.
  Verdict — hardcoded event-song override, not map data and not
  stage-locked content: the two songs form one pair (adjacent
  `OJNList.dat` rows 354/355, shared artist `presented by 거북이`,
  Sirius/Everfree `.o2SE` labels, Christmas `Rudolph` title fragments)
  pinned to stage 4 by id compare. Their catalog rows carry no stage
  column (BPM 120/130, normal counts and slices, listing 1, category
  BLUE, params `(2000, 0)` — all overlapping neighbours such as 647/658/
  659 of the same GRADATION-45 family), so map data cannot drive the
  choice; and stage 4 holds nothing unique (`Playing_BG4.ojs` is one of
  37 identical-size 665164 B stills, the only animated variant is stage
  19, OJM carries audio only). Callers `0x4aace5` (default-if-zero on room
  entry: virtual call slot `+0x60` at `0x4aacd1`, then store to `0x7789dc`
  at `0x4aacea`), `0x4b06d9`, `0x4ba86f`, `0x4bb63d` (cycle:
  advance/change stage).
- The stage change rides protocol **4002** (`0xFA2` pushed before a vtable
  slot `+0x5c` call at `0x4bb600`/`0x4bb64d`) — a server-visible room state.
- Applier `0x4ad350(arg)` stores `arg` to `0x7789dc` (file `0x4ad413`), after
  range-checking the low word (`& 0xFFFF`, must be `≤ 0x24` = 36, files
  `0xad3db`–`0xad3ea`; above exits). It then branches on `arg >> 16`: set →
  control `0x16500072` (`IDC_IMAGE_SWAIT_P2P_MAP` = `Wait_Map_P2P.ojs`,
  file `0x4ad42d`); clear → `0x1650004F` + parity frames (file `0x4ad554`:
  `stage % 2`, 0 → 2). The high word is a room-display flag (waiting-room
  preview branch selector), not an animated-stage marker (close-out in
  OPEN 4). Callers `0x4aacfc` (forwards the flagged picker-issued global),
  `0x4ab272` (forwards the global as-is), `0x4ab63a` (forwards a
  caller-supplied plain stage, so the clear path stays reachable).

## `_` runtime resolution (applier `0x4e7850`)

Walks the 41 Playing entries (arg0 base + index, loop bound `0x29` at
`0x4e7958`); entries 0..20 take a 10-case jump table (`0x4e85ac`, selector
bytes `0x4e85d4`), the rest the generic path at `0x4e7fde`:

1. Entry record is `0x11c` bytes (`imul …,0x11c` at `0x4e7ffc`); byte 2 of the
   code dword must be nonzero (`and 0xff0000`, `0x4e8008`), else skipped.
2. `_` test: first sprite-name byte == `0x5f` (`cmp ecx,0x5f` at `0x4e8031`).
   Non-`_` names load verbatim via the slot loader `0x48b2d0` (`0x4e8382`),
   results stored at state+`0x53c`+idx·4 and +`0x6dc`+idx·4 (`0x4e8399`/
   `0x4e83d0`).
3. `_` path: copies the record (`memcpy 0x11c`, `0x4e805f`), `strlen` of the
   sprite name (`0x5f1bc0` at `0x4e8067`), then `strncpy` (`0x5f2ad0`,
   `0x4e80ef`) of name+1 with length strlen−5 — dropping the `_` and the
   4-char extension: `_Playing_BG.ojs` → stem `Playing_BG`.
4. Classification is **exact case-insensitive equality** (`_stricmp` wrapper
   `0x5f34ff`; its worker `0x5f33f7` folds `'A'..'Z'` by `+0x20` and compares
   whole strings — substring readings are ruled out). With the archive-open
   flag (display singleton `0x76816c` → object at `+0x80`, flags bit 2 via
   `0x4189b0`→`0x415d40`, tested at `0x4e8108`) set (in-song):

   | stem (case-insensitive) | format (sprintf `0x5f2c4d`) | example |
   |---|---|---|
   | `Note_Click` | `%s%d_%d.ojt` (`0x64b154`, site `0x4e814e`) | `Note_Click7_2.ojt` |
   | `Note_LongCombo` / `Note_LongComboNum` | `%s.ojt` (`0x64b128`, `0x4e819e`) | bare |
   | anything else, ext `ojs` | `%s%d.ojs` (`0x64b118`, `0x4e81da`) | `Playing_BG7.ojs` |
   | anything else, other ext | `%s%d.%s` (`0x64b110`, `0x4e8205`) | `Note_Miss7.ojt` |

   The two `%d` args are locals seeded in the applier head (`0x4e78db`–
   `0x4e792e`): stage = high word of `0x7789dc` set ? low word : full value;
   variant = `rand()%2 + 1` (call `0x5f7236` at `0x4e78fe`) or `0x7789e0 + 1`.
   Flag clear (archive bit 2 unset, e.g. UI-preview contexts) resolves bare
   names instead (`0x4e8212`–`0x4e8303`): `Note_Click`→`Note_Click.oja`,
   `Playing_Effect_Jam`→`Playing_Effect_Jam.oja`,
   `Note_LongCombo`/`Num`→`%s.ojt`, ext `ojs`→`%s%d.ojs` (still numbered),
   else→`%s.ojs` (base art: `Note_Miss.ojs` etc.).
5. The formatted name replaces the sprite field in the record copy, and the
   same slot loader fetches it (`0x48b2d0` at `0x4e8313`).

## Placement and schedule

- Draw placement is the sprite anchor alone — `(204,0)` 596x558 for the stage
  (`docs/ui-draw.md`: the blit path adds the per-frame anchor to the view
  position; `.bnd` rects are not read at draw time). `State_Playing.bnd`'s 17
  rects serve only the `EXIT`…`VOLUME` tail (entries 213–229,
  `docs/bnd-semantics.md`) — none covers the right side, consistent.
- The stage is **static on the song timeline**: chosen before the room starts
  the song, rendered continuously; no timeline-driven BGA events exist. Only
  per-hit effects (key/click/jam kits, stage-numbered) and standard sprite
  animation (`0x42f040` frame advance) move over it.

## No-BGA behavior

- There is no song-supplied BGA in the 7.55 client to be missing. OJM
  containers carry audio only — every verified payload is OggS page chains or
  PCM (`docs/file-formats/ojm.md`); no image section exists.
- The OJN-embedded JPEG (bytes after the last level slice; all three slice-end
  words at header +288/+292/+296 equal the JPEG offset) is album art for the
  select/album panels, alongside the 47 `ATI*` raw-JPEG `.ojs` bodies in
  `Interface1.opi` (`docs/file-formats/opi.md` body classes) — none of it
  paints on the Playing scene.
- Because the room guarantees `0x7789dc != 0` before the song (store at
  `0x4aacea`) and all 37 numbered stages exist, the backdrop always renders.
  If the numbered name were absent, the slot loader would simply leave the
  slot empty and the layer would draw nothing — there is no dedicated
  default-stage fallback image; the unnumbered `Playing_BG.ojs` is reachable
  only through the bit-clear bare-name arm.

## Shot arena pick (rooftop reference)

The walk in `crates/recomp2jam/src/shots.rs` joins its room at arena
stage 2 (`PLAYING_ARENA_STAGE`), so `playing.png`/`playing-native.png`
paint `Playing_BG2.ojs` behind the avatar. Picked by decoding all 37
`Playing_BG.ojs` + `Playing_BG<1..36>.ojs` bodies (each one 596x558 raw
RGB555 frame, anchor `(204,0)`) from `Playing1.opi` and comparing them
against the rooftop sunset venue in `docs/screenshots/reference-4x3.png`:
stage 2 is the closest compositional match — an open foreground ground
plane sloping to water, a distant high-rise skyline across it, and open
sky above, mirroring the rooftop's deck → parapet → towers → sky stack.
No stage reproduces the venue: the 3D rooftop (plank deck, railing,
sunset-orange sky, blimps) is not among the painted 2D stages.

Residual delta, documented honestly: grass foreground for the wooden
plank deck; noon-blue sky for the sunset-orange clouds; no parapet
railing, no blimps, lake for the rocky hill. The waiting `MAPNAME`
caption for arena 2 reads `Losers Counterattack` (map-cycle table,
low word mod table length).

## OPEN

1. CLOSED — `Playing_BG19Ani.ojs` has no consumer in 7.55 (archive orphan,
   no frame schedule). The `%s%d%s` sites build `SMI_`-family and music-temp
   names, never stage names: the format lives at file `0x24656c` beside
   `SMI_NoImage.ojs`, and the four call sites push stems `SMI_`
   (VA `0x646580`; files `0x5c969`, `0x12159a`) and `o2ma` (VA `0x64ba00`;
   files `0xf82c7`, `0xf846d`, suffixes `.ojn`/`.ojm`); the three
   `%s%d_%d%s` sites (files `0x5c941`/`0xf829f`/`0x121572`, format file
   `0x246574`) push the same stems. The three stage appliers build only
   `%s%d.ojs`, `%s%d.%s`, `%s%d_%d.ojt`, and bare arms (files `0xe8143`/
   `0xe81fa`/`0xe81cf` and twins) — no `Ani`-suffix arm exists, and no `Ani`
   stem literal exists outside `Guide_Ani.ojs`/`Notice_Ani.ojs` (files
   `0x24b5a2`/`0x24c093`). The only Playing control list names
   `_Playing_BG.ojs` (`BACK1`); no `.txt` in either archive names `BG19Ani`
   or `BG_Ani`. The 4 frames are never staged, so the standard frame advance
   (`0x42f040`) never runs on them — there is no schedule to trace.
2. CLOSED — stage paints first, avatar paints second. The venue
   `BACK1` (`IDC_IMAGE_SPLAY_BACK1`, code `0x1B500071`, ordinal 113)
   stages through the `_` applier `FUN_004e7850` (VA `0x4e7850`,
   41-entry walk, bound `0x29` at file `0xe7958`) into the
   behind-highway set (entries 113–114, 209–212, file order
   back-to-front), and the list render loop `0x5dec30`
   (`ev_ff581cfa…`, `docs/ui-draw.md`) paints file order with the
   full-screen entry first. The avatar seats are type-`0x70` controls
   with no sprite (runtime-resolved from `avatar.opa`); the shell
   avatar pass `LivePlay::paint_avatar_scene`
   (`crates/o2jam-shell/src/playing.rs`) paints over the highway and
   all staged art, and `AvatarScene::paint` draws the `Lv` plate plus
   life bar, then `figure.paint_extent_centered` in the
   `Map_Avatar.bnd` seat (307,161)-(607,461), then the tally strip.
   The photo avatar-scene box (`docs/playing-reference.md`) pins the
   same stack: figure over backdrop. Layering confirms it: `.ojs`
   rides layer 1 and `.oji` rides layer 2 (`SetLayer` `0x424100`,
   `ev_015a885f…`), so the `Avatar_Shadow.oji` seats composite above
   the `.ojs` stage even though file order alone would put `BACK1`
   (ordinal 113) above the low-ordinal shadows. Rule: stage never
   occludes avatar; avatar occludes stage inside the seat box.
3. Who calls the three twin appliers (`0x4e7850` and the `0x4ee8f5`/`0x4f2c73`
   clusters): addresses sit adjacently in `.rdata` at `0x64ae48`–`0x64ae50`,
   suggesting a state-vtable dispatch (solo/P2P/tutorial variants unattributed).
4. CLOSED — the stage high word is a room-display flag, not an
   animated-stage marker. The picker sets bit 31 (`0x80000000`) on every
   return: defaults `0x80000001` (files `0xbb31a`/`0xbb3bc`/`0xbb3c5`: null
   preview control, null art, zero frames) and `or` at file `0xbb3cf` for
   the random path and the event-song path (stage 4 → `0x80000004`) alike —
   so the flag rides all stages 1..36 plus the defaults, uncorrelated with
   the stage-19-only `Ani` file. The setter range-checks the low word
   (`& 0xFFFF`, `≤ 0x24`, files `0xad3db`–`0xad3ea`) and uses the high word
   only to pick the waiting-room preview branch (set → `0x16500072` P2P-map
   art at file `0xad42d`; clear → `0x1650004F` plus parity/SMI art at file
   `0xad554`). The `_` resolver likewise strips it to recover the low-word
   stage for the static `%s%d.ojs` build (files `0xe78df`–`0xe78f8`).
5. Protocol 4002's exact packet layout (stage id width, who may change it) —
   `docs/net-protocol.md` cross-check pending.
