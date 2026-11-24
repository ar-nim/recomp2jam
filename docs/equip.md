# Shop equip + avatar slot semantics (7.55)

Version pinned to `AngelO2Jam.exe` 5,029,888 B,
SHA-256 `83a04541…72c823b` (file offset = VA − `0x400000`;
`.text` VMA `0x401000`, file off `0x1000`).
Claim style: static `objdump -d` analysis in our own words, no
disassembly reproduced. The older sites below carry file offsets only
(the shared REA dossier holds 55 loading/song-path records). The
draw-order pin added in #908 uses the REA 6.9.0 CLI with Ghidra 12.2
and cites its `ev_…` evidence IDs.

## Wire: `EquipItem` 5004 → 5005

Odd/even pair like the rest of game TCP: request C→S `5004`,
response S→C `5005` (`docs/net-protocol.md` §3).

| Message | Id | Body |
|---|---|---|
| `EquipItem` (request) | 5004 (`0x138c`) | `i32` slot, `i32` item id (8 B LE) |
| `EquipItemResponse` | 5005 (`0x138d`) | `i32` result + `Option<[i32; 4]>` tail (`Some` iff result `0`, 20 B total) |

Send builder case for 5004 (VA `0x4a4ec8`, file `0xa4ec8`):
stores the opcode as `u16`, stamps length `4`, then appends both
args as 4-byte words — hence two `i32`. Both C→S call sites push
slot then item: the shop-control path (VA `0x4a4af4`, file
`0xa4af4`) and the apply path (VA `0x4a4bcb`, file `0xa4bcb`).

The canonical S→C handler (VA `0x560cf0`, file `0x160cf0`,
stub `0x558853`) reads the result word via `0x555d20`: nonzero
takes the shared exit, zero reads four more `i32` (20 B total)
and forwards tail words 0 and 2 to queue `0x478d50`. The
`0x4a5fe8` (file `0xa5fe8`) site reached from dispatch
`0x4a5570`/`0x4a5586` is a downstream consumer of the decoded
word (lookup `0x537b80`, table `0x64ca38`), not a wire read:
nonzero routes to the equip-apply function (`0x4a3c80`, file
`0xa3c80`), zero to the remove function (`0x4a3dc0`).

## Slot selection (17 equipment slots)

The client keeps the worn set as `this+0x5c[slot]`, written by
`mov %eax,0x5c(%edx,%ecx,4)` (VAs `0x4a3d53`/`0x4a3d97`, files
`0xa3d53`/`0xa3d97`) — one `i32` item id per slot. That array is
the 17-slot `CharacterInfo.equipment` (`ItemType` 0–16,
`crates/o2jam-net/src/character.rs`).

The slot is derived from the item record (looked up via the
`0x52dba0` helper), not trusted blindly from the wire:

- record byte `+4` = item type, record byte `+0x18` = slot
  (`movzbl` reads at VAs `0x4a3cc9`/`0x4a3cf0`, files
  `0xa3cc9`/`0xa3cf0`; same shape at the send site,
  VAs `0x4a4b5f`/`0x4a4b74`);
- type `5` forces slot `0xb` (11) (`cmpl $0x5` … `movl $0xb`,
  send site VAs `0x4a4b7b`–`0x4a4b81` / apply VA `0x4a3cfd`,
  files `0xa4b7b`–`0xa4b81` / `0xa3cfd`);
- types `0xf`–`0x12` (15–18) take a set-info branch (apply VAs
  `0x4a3d04`–`0x4a3d0e`, file `0xa3d04`–`0xa3d0e`); join against
  `SetInfodata.ojs` stays OPEN;
- final slot is range-checked `0 <= slot < 0x11` (17) (send-site
  VAs `0x4a4baa`–`0x4a4bb4`, files `0xa4baa`–`0xa4bb4`).

## Equipment id rules

- `0` = slot empty (`CharacterInfo`; `load_worn_figure` skips zero
  cells, then substitutes the default body when no equipped row is a
  body-type item — see #243 below).
- `1..=1541` = `Itemdata_Korea.dat` row ids (walk lands exact at
  EOF; `docs/file-formats/opi.md`).
- Unknown ids, missing archive entries, and non-sprite bodies are
  skipped, never fatal (`load_worn_figure`,
  `crates/o2jam-shell/src/avatar.rs` — unchanged).

## Live local-server equipment (the #171 head-only fact, 2026-11-13)

The local Memoryer rig (`~/memoryer-local`, SQLite `O2JAM.db`) held
**one nonzero equip cell per account**: `t_o2jam_item` rows for both
`USER_INDEX_ID=1` (tester) and `USER_INDEX_ID=2` (tester2) carried
`Equip12 = 35` and zeros elsewhere. Item 35 is the default male face
(row name `남자얼굴 기본`, type word `0x00800105`, low byte `5` — the
type-5 → slot `0xb` rule above, so `Equip12` is the face column).
The 2001 reply therefore carried a one-item kit, and the head-only
room-panel render was CORRECT for that data (ticket #171 round 2;
the stacker stacks what the reply carries). **Contradiction recorded
(#336):** the qualified 2026-11-17 Wine waiting capture
(`c3-waiting-qualified/artifacts/screenshots/waiting.png`, SHA-256
`f394dcaa…1393162`) shows a full pale body under that same face-only
kit. Its wire tap `wire/room-stage.log` 2001 reply carries only item
35 in slot 11 (body offset `0x5D`, inside the 17-`i32` table starting
at `0x31`); the visible body decodes to item 30's
`01M13T0000001.ojs` anchor `(133,110)` (34×89, 12 idle frames).
For that capture path the exe paints the default body beneath worn
face art, so the shell's #336 e2e fixture stacks ids `[30,35]`.
This does not retroactively change the #171 stacker finding; the
exe capture is the behavior oracle and both readings stay visible. Fixture: the tester2 row
now also wears the full-body kit ids 16 (hair), 30 (base body
`몸통`), 376 (top), 383 (bottom), 389 (shoes) at `Equip1..Equip5`
(column placement of kit ids is fixture freedom — the server copies
`Equip{N}` to wire slot `{N-1}` blind, and the stacker resolves ids,
not slot semantics; only the face column is pinned). Live wire probe
(`recomp2jam connect --character` against `127.0.0.1:15010`):
face-only row stacked **60 layers**, kitted row **447 layers**
(male parity) — those counts are the retired stack-every-slot
picker's output; the one-body-per-part picker now stages 12 layers
for the face-only row and 63 for the kitted row (see the worn-figure
mapping below). Fixture (ticket #200, 2026-11-14): the tester2 row
carries `Bag1..3 = 376, 383, 35` (zeros elsewhere, `BAG_EXT_COUNT`
0), so the probed 2001 `inventory` reads `[376, 383, 35, 0×27]` —
the owned-items source the bag pages live (DB backup
`~/memoryer-local/O2JAM.db.pre200.bak`; local DB rows only).

## Empty body slot — default naked figure (#243, 2026-11-16/18)

The local tester's 2001 wire is not a wholly empty table: `Equip12`
holds face item 35 and the other 16 cells are zero (live DB read-only
query: `t_o2jam_item` for accounts 1 and 3, `Equip12=35`;
`t_o2jam_char_attr_item` count 0 is a different table). The exe paints
a full naked body beneath that face; our client painted only the head.
This is the same wire as EV-336-B, so the default body is item 30.

Pinned oracle: `~/scratch/exe-avatar2/waiting-full.png` (800x600 exe
waiting room, naked default avatar at seat 2, READY tag; MD5
`a4254cb47d3c458c5f836e4fee89cc81`) and `avatar-closeup.png` (MD5
`13659098704828e695a703c50df855a0`), verified by senior eyes in the
#243 thread. Exe pin: `AngelO2Jam.exe`, 5,029,888 B, SHA-256
`83a04541b8219ba340681766b33de2bfbb5ff51b813ef4d2e9dd1def72c823b`.
Part evidence: EV-336-B (`Itemdata_Korea.dat` at `avatar.opa` file
offset `142,934,156`, 740,945 B; body row 30, face row 35) and
EV-336-C (body `01M13T0000001.ojs` at file offset `6,129,704`, anchor
`(133,110)`, 34×89, 12 frames; face `01M160000001.ojs` at offset
`6,495,728`, anchor `(134,81)`, 32×35, 12 frames). EV-336-D exe sites
in this file: CAvatar ctor VA `0x52b290` (file `0x12b290`, 66 ms
step), restage VA `0x52b480` (file `0x12b480`), paint VA `0x52b9b0`
(file `0x12b9b0`, 48-step phase). File offset = VA − `0x400000`.

Behavior: when the equipped rows contain no body-type item (type word
`0x01000000`, local table rows 30 and 761), `load_worn_figure` stages
default body 30 beneath the worn pieces, plus default face 35 for a
male wearer or 36 (`여자얼굴 기본`) for a female wearer, and bare arms 31
(`왼팔`, #375 below). Ids already equipped are not staged twice, so a
face-only kit gains the body, arms, and face; a kit with item 30 is
unchanged. The fallback lives at the shared compose seam, so live
seats, the card/room figure, and the shop pane all use it. A face-only
kit is the live case; an all-zero table gets the same default trio.

Guards: `empty_equipment_stages_the_pinned_default_naked_kit` and
`equipped_figure_keeps_its_own_layers_without_default_fallback`
(synthetic sprites; live chain named in each test), plus
`live_boot_empty_body_slot_stages_the_default_naked_figure` (scripted
gateway boot with the face-only wire table, skipped when
`install-root` is absent). No copyrighted bytes were added.

## Bare arms in the default kit (#375, 2026-11-19)

The #243 trio above was incomplete: item 30's `01M13T0000001.ojs`
holds torso/legs/feet and no arm pixels in any of its 12 frames, so
user 1's face-only wire (`Equip12 = 35`, read-only local DB check)
painted a torso with no forearms or hands. The missing pixels come
from item **31** (`왼팔`, type `0x0100001`), whose idle slot 2 is
`01MI13LA0000001.ojs`: one 43x43 image at anchor `(128,121)` carrying
both bare arms and hands. Rows 32 (`왼손`) and 33 (`오른팔`) are
instrument-pose art with no idle variant and are not part of this kit.

Oracle: the p223ab exe/Rust waiting-seat crops of the same avatar
(`~/scratch/p223ab/exe2/cadence/state-00.png`, SHA-256
`f8d03a6f5…f9633a9e`, and `~/scratch/p223ab/rust2/cadence/state-00.png`,
SHA-256 `c4264dbd…f5591`). Decoding the installed 7.55 sprites at
their stored anchors and fitting to the exe crop gives translation
`(-115,-70)`, breathing frame 4, mean absolute per-channel error **0.9**
over the arm sprite's 298 opaque pixels; the current body+face model
misses those pixels by **152.9**. The Rust crop at its own fit
`(-112,-75)` has no arm pixels (85.5 with the arm model, 93.5 without).
This localizes the divergence to item 31 and rules out an idle frame
index error (clue discarded in `~/scratch/p375-findings.md`).

Fix: `compose_worn_rows` prepends item 31 between the default body and
face. Guards: `face_only_wire_kit_stages_the_default_bare_arms`
(synthetic sprites) and `live_face_only_kit_stages_the_pinned_arm_sprite`
(installed `avatar.opa`, skipped when absent), both red on the old
fallback (`[1,1]` / `[12,12]` versus expected `[1,1,1]` / `[12,1,12]`).
`waiting_seat_paints_bare_arms_for_the_face_only_row` pins the visible
waiting-seat painter: zero arm/hand diff pixels before the fix, 200+
after. The scripted live boot guard pins the 25-layer composition.
Version pin as above: `AngelO2Jam.exe` SHA-256 `83a04541…72c823b`.

## Worn-figure animation (pinned: 66 ms step, 48-step phase)

Every probed worn-item body holds **12 chained same-part frames** in
one sprite-55 container: face `01M160000001.ojs` (12 records, anchors
(134,81)/(134,82)/(134,83), 32×35–36), body `01M13T0000001.ojs`
(12 records, (133,109)/(133,110), 33–34×89–90), bottom
`17M14001.ojs` (12 × 22×40 at (139,136)), shoes `17M15001.ojs`
(12 × 36×25), tops `17MB/MG/MK13T001.ojs` (12 records each) —
record-directory walks land exact (`8 + 20·12 + blob == len`),
±1 px anchor/size jitter between neighbours (breathing frames).
Those records are the frames of ONE sprite (our
`AvatarFigure::from_sprite55` stacks them as layers and the shared
painter selects one per animation phase).

Frame counts per (part block, pose) on the live archive, male slots
(`o2jam-dump` probe over 1541 rows): idle pose `I` is a **1-image
still** in 676 blocks (265 of them arm blocks), 12 frames in 37 arm
blocks; every instrument pose `G`/`B`/`K`/`D` is 12 frames (302–306
blocks each). Three blocks carry 11-frame bodies and twelve carry
13-frame bodies.

### Exe pin (file offsets, `objdump -d`; version `AngelO2Jam.exe`
5,029,888 B, SHA-256 `83a04541…72c823b`, file offset = VA −
`0x400000`)

The worn avatar is class **CAvatar** (vtable `0x64c828`, RTTI
complete-object-locator `0x668a78` → `.?AVCAvatar@@`; ctor VA
`0x52b180`, built by the ControlList factories at `0x432d26`,
`0x479bb6`, `0x48a982`, `0x48b624`, size `0x20c`).

- **Step cadence: 66 ms.** The ctor (VA `0x52b290`) stores `0x42` at
  `+0x1f0` (ms per animation step) and starts the phase counter
  `+0x1ec` at 0. The paint (VA `0x52b9b0`, vtable slot 9) asks the
  timer helper VA `0x411ea0` whether `now − +0x1f4 > +0x1f0`, then
  adds `floor((now − last) / 66)` steps to a float remainder at
  `+0x1f8` and wraps the phase over `0x30` = **48** steps (a phase
  above 48 is forced to 1). One phase cycle is 3168 ms; a 12-frame
  body advances one frame every 66 ms (792 ms per cycle, 15.15
  frames/s).
- **One phase for every part**: the paint walks the draw-order row
  and sets each part's frame from the same counter
  (setter VA `0x41e0f0`), so all parts advance together.
- **Frame rule**: a body with more frames than the phase shows the
  phase as its frame; a 12-frame body shows `phase % 12`; a 24-frame
  body `phase % 24`; any other length asks for frame 1, which the
  setter rejects (index ≥ count), so a **1-image body never changes
  pose** and an 11- or 13-frame body freezes on frame 1 after the
  first steps.
- **Pose is static per kit**: the restage (VA `0x52b480`) builds ONE
  pose word per avatar. Bits 3–5 choose the block's variant slot
  (selector VA `0x52e730`: variant 0 → slot base 2, 1 → 4, 2 → 6,
  3 → 8, 4 → 10; bits 8–10 add 10 per part block, bit 6 adds the
  female parity, so slot = `2 + 10·block + 2·variant + female`). The
  variant comes from the row type byte of the item at CAvatar
  `+0x60` — switch table VA `0x52b96c`: **16 guitar → 1, 18 bass →
  2, 15 keyboard → 3, 17 drum → 4**, anything else → 0. That field
  and `+0x64` are written only from the global pair `0x77b52c` /
  `0x77b530` (init `0x52b21e`, restage `0x52b4c8`, reload
  `0x52b624`), which the image initialises to 0 at `0x63d1d3` /
  `0x63d1dd` and never writes again — so a live 7.55 avatar always
  wears variant 0.
- **What the five variants are**: row type bytes `0x0f..0x12` are
  the instrument skins (`Itemdata_Korea.dat`: 15 keyboard
  `피쉬키보드`, 16 guitar `샤인`/`이글 스트라이크`, 17 drum
  `블루쉘 드럼`, 18 bass `피너츠`), and the variant letters match:
  **I** idle (one-image still), **G** guitar, **B** bass, **K**
  keyboard, **D** drum (12-frame playing poses). Wearing an
  instrument therefore changes the pose of *every* part block and
  turns the sleeves into a 12-frame playing animation; a kit with no
  instrument wears the idle pose, whose sleeves and jacket torso are
  a single still image. **The five variants are not animation
  frames; the exe never cycles them.**

### Consequence for ticket #222

The idle hands of a jacketed avatar (item 376's sleeves) are a
1-image still in the exe too, so "hands do not move" is exe parity
for an instrument-less kit; room motion comes from the 12-frame
bodies (base body, hair, face, bottom, shoes) at 66 ms per frame.
Our painter implements this (`AVATAR_BREATHING_TICK_MS = 66`,
`AVATAR_ANIMATION_PHASE_COUNT = 48`, `worn_part_frame`,
`AvatarPose`, `worn_pose`), and the bare-board idle pulse runs at
one step (66 ms) so every step reaches the screen.

### Draw-order table (#908, 2026-11-20)

Oracle: `OTwo.exe` = `AngelO2Jam.exe`, 5,029,888 B, SHA-256
`83a04541b8219ba340681766b33de2bfbb5ff51b813ef4d2e9dd1def72c823b`.
REA 6.9.0 CLI (`rea decompile OTwo.exe <addr>`, provider Ghidra 12.2),
behavior described in our own words; no disassembly is reproduced here.
File offset = VA − `0x400000`.

- Paint VA `0x52b9b0` (file `0x12b9b0`), evidence
  `ev_5a3c251720bd6841558684b970b2b6f7c45bc70f7c26a8126d7569435ff260da`:
  one of five rows is selected by the kit's instrument type (idle 0,
  guitar 1, bass 2, keyboard 3, drum 4 — matching the restage switch
  pinned above). The paint then walks exactly `0x24` = 36 bytes of
  that row. A byte below `0x25` indexes a dword in the 36-slot surface
  array at `this + 0x150`; a nonzero dword gets its frame set and a
  blit at the stored `(+0x1e4, +0x1e8)` origin. `0xff` ends the row.
- Restage VA `0x52b480` (file `0x12b480`), evidence
  `ev_25a328cdd807fb4b48bb126dcb375bcc489d58251a33fb1cce51cfc10fcb85fc`:
  clears `0x90` bytes at `+0x150` (36 dwords) and fills 29 part slots
  from 29 item pairs at `+0x68`; item index 6 also fills `+0x1c8`
  (array slot `0x1e`) and indices 15..18 fill `+0x1cc..+0x1d8`
  (slots `0x1f..0x22`) with secondary bodies.
- Sprite-slot selector VA `0x52e730` (file `0x12e730`), evidence
  `ev_407fcd2991e5ff7c2d30e1b024f09eb661a6a5a9c044219c136bf8790c521fab`:
  the pose bits choose sprite slots 2/4/6/8/10 per part block plus
  `+0/+10/+20/+30` block offsets, confirming the table selects a
  pose row, not an equipment order.
- Constructor VA `0x52b180` (file `0x12b180`), evidence
  `ev_86e850ea16d8c795a5475d1c78ad8550f28cd50a7c1a69b0a646c8ca43cd9b19`:
  zero-initialises the 29 item pairs and the 36 surface dwords.

Table bytes: VA `0x64c770..0x64c823`, file `0x24c770..0x24c823`,
5 rows × 36 B, row stride `0x24`. SHA-256 of the 180 bytes:
`e2274af39ef4558ee176d2a9a5974566c8800958d4a1b3cdf710151e72cb301a`.

| Row (pose) | VA | Bytes up to the first `0xff` (entry count) |
|---|---|---|
| 0 idle | `0x64c770` | `23 0b 1e 14 22 00 0e 0d 0a 01 03 02 13 20 21 09 0c 05 08 07 06 1c 15 16 17 1f` (26) |
| 1 guitar | `0x64c794` | `23 0b 1e 14 22 00 0e 0d 0a 01 20 13 10 1f 03 02 21 09 0c 05 08 07 06 1c 15 16 17` (27) |
| 2 bass | `0x64c7b8` | `23 0b 1e 14 22 00 0e 0d 0a 01 20 13 12 1f 03 02 21 09 0c 05 08 07 06 1c 15 16 17` (27) |
| 3 keyboard | `0x64c7dc` | `23 0b 1e 14 22 00 0e 0d 0a 01 03 02 13 20 21 09 0c 05 08 07 06 04 1c 15 11 1f 16 17` (28) |
| 4 drum | `0x64c800` | `23 0b 1e 14 22 00 0e 0d 0a 13 0f 1f 01 03 02 20 21 09 0c 05 08 07 06 1c 15 16 17` (27) |

Comparison with our painter: `load_worn_figure` walks the 17 equipment
cells in array order and, per item row, the part blocks in ascending
order from `pick_wear_sprites`; `AvatarFigure` blits one frame per run
in that order. The exe's walk is not that order. Its idle row starts
slots `0x23, 0x0b, 0x1e, 0x14, 0x22`, then `0x00, 0x0e, 0x0d, …`;
guitar and bass insert `0x20, 0x13, 0x10/0x12, 0x1f` before `0x03`;
keyboard inserts `0x04, 0x11`; drum starts `0x13, 0x0f, 0x1f`. An
equipment-order walk would start `0x00, 0x01, 0x02, …`. The orders
therefore differ from the first drawn body.

Decision: ADR-0015 keeps equipment-slot paint order and records this
as an OPEN divergence. The table is pinned evidence, not consumed:
the mapping from our equipment cells and part blocks to the exe's 29
part slots and secondary surfaces is not established, so consuming it
now would invent an ordering. Next evidence step: trace the restage
caller back to the 7.55 equipment table, read the 29 pairs in a stopped
Wine session, then compare one overlapping-part capture against the
table order.

Documented divergences, none silent:

1. **Empty pose slot**: the exe returns null and draws no part;
   `pick_wear_sprites` falls back to the block's other parity and
   then to its first non-empty wearer-parity slot, so a garment never
   vanishes. ADR-0016 (#909) records this as an OPEN divergence: the
   parity patch red-proved three guards but broke five out-of-lane
   `connect.rs` fixtures, so no code change landed.
2. **Draw order**: extracted and compared above; ADR-0015 keeps slot
   order and pins the mismatch. OPEN: z-order parity.
3. **Odd-length bodies** (11/13 frames) are rare; the rule above is
   implemented as pinned (frame 0 instead of the exe's rejected
   frame 1, same visible result).

OPEN: no live read of CAvatar `+0x60` (expected `(0,0)`) — next
evidence step is a winedbg stopped-state read on the room avatar.

## `load_worn_figure` mapping (one body per part)

Per nonzero equipment id in slot order: resolve the
`AvatarItemRow` by id, fix the kit pose (`worn_pose`: an instrument
skin selects `G`/`B`/`K`/`D`, else idle), run `pick_wear_sprites`
with that pose, map each filename `.oji` → `.ojs`, parse
`sprite55`, stack decodable layers.

Do not confuse the two "slot" uses: the 17 equipment slots above
select *which item* is worn, while the 42 sprite slots per item
row select *which bodies* are drawn — slots 0–1 are shop icons
(never worn); the rest splits into **4 part blocks of 5 male + 5
female pose variants** (block `b`, variant `v`: male slot
`2+10b+2v`, female slot `3+10b+2v`). Even slots carry male
variants, odd slots female ones (`docs/file-formats/opi.md`).

`pick_wear_sprites` wears **one body per part block** in the kit's
pose, wearer parity first (ticket #222: the five variants of a block
are five *poses of the same part*, so wearing all five printed ghost
hands at one anchor — item 376 stages torso / left arm / right arm in
blocks 0–2, five poses each, and the unisex items repeat one file
across their five slots). An exact empty slot still falls back; ADR-0016
records the exe's draw-nothing divergence. The live tester2 kit
therefore stages 8 bodies (hair, body, torso, left arm, right arm,
bottom, shoes, face) and 63 layers: run lengths
`[12, 12, 1, 1, 1, 12, 12, 12]`.

## Verdict

- `EquipItem` 5004 request layout + slot derivation + 17-slot
  array write: VERIFIED (static sites above).
- `EquipItemResponse` 5005 result word + 4-word success tail:
  VERIFIED (handler file `0x160cf0`, stub `0x558853`); tail-word
  roles: OPEN.
- Per-item stat effects: server-authoritative, no local math
  found (unchanged); avatars cosmetic except skill rings
  (`docs/gameplay.md`).

## Tests (`crates/o2jam-net/src/message.rs`)

`equip_item_encodes_slot_then_item` (exact frame bytes),
`equip_item_round_trips_through_frame`,
`equip_item_rejects_short_body`, `equip_item_rejects_trailing_bytes`,
`equip_item_rejects_wrong_command`,
`equip_item_response_round_trips_through_frame`,
`equip_item_response_zero_item_decodes`,
`equip_item_response_rejects_short_body`,
`equip_item_response_rejects_trailing_bytes`.

## Avatar idle animation (ticket #176 / #222d) — CLOSED by the pin above

The 2026-11-13 attempt found the generic sprite-frame setter (VA
`0x41e0f0`) but could not attribute the avatar's cadence, and #176
was closed as superseded. Ticket #222d resumed the static hunt with
the class identified first (CAvatar vtable `0x64c828`) and pinned the
step (66 ms), the phase cycle (48) and the frame rule; the static
attempts and their negative results stay in the git history of this
file for the record. Implementation lives in
`crates/o2jam-shell/src/avatar.rs` (`AVATAR_BREATHING_TICK_MS`,
`AVATAR_ANIMATION_PHASE_COUNT`, `avatar_animation_phase`,
`worn_part_frame`, `AvatarPose`, `worn_pose`), and the guards
`avatar_animation_phase_clock_guard`,
`avatar_part_frame_follows_the_exe_frame_rule`,
`tt222d_live_kit_moves_between_animation_steps`,
`tt222d_still_part_holds_its_pose_and_never_blanks`,
`tt222d_instrument_pose_animates_the_sleeves`,
`wear_follows_the_kit_pose_in_every_block`,
`worn_pose_reads_the_instrument_row_type`.

