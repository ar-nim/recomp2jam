# Shop equip + avatar slot semantics (7.55)

Version pinned to `AngelO2Jam.exe` 5,029,888 B,
SHA-256 `83a04541…72c823b` (file offset = VA − `0x400000`;
`.text` VMA `0x401000`, file off `0x1000`).
Claim style: static `objdump -d` analysis in our own words, no
disassembly reproduced. No REA dossier covers these sites (the
shared snapshot holds 55 loading/song-path records only), so file
offsets below are the evidence; no `ev_…` id is cited.

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

- `0` = slot empty (matches `CharacterInfo` and
  `load_worn_figure`, which skips `0`).
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
the stacker stacks what the reply carries). Fixture: the tester2 row
now also wears the full-body kit ids 16 (hair), 30 (base body
`몸통`), 376 (top), 383 (bottom), 389 (shoes) at `Equip1..Equip5`
(column placement of kit ids is fixture freedom — the server copies
`Equip{N}` to wire slot `{N-1}` blind, and the stacker resolves ids,
not slot semantics; only the face column is pinned). Live wire probe
(`recomp2jam connect --character` against `127.0.0.1:15010`):
face-only row stacked **60 layers**, kitted row **447 layers**
(male parity).

## Worn-figure idle frames (structure pinned, cadence OPEN)

Every probed worn-item body holds **12 chained same-part frames** in
one sprite-55 container: face `01M160000001.ojs` (12 records, anchors
(134,81)/(134,82)/(134,83), 32×35–36), body `01M13T0000001.ojs`
(12 records, (133,109)/(133,110), 33–34×89–90), bottom
`17M14001.ojs` (12 × 22×40 at (139,136)), shoes `17M15001.ojs`
(12 × 36×25), tops `17MB/MG/MK13T001.ojs` (12 records each) —
record-directory walks land exact (`8 + 20·12 + blob == len`),
±1 px anchor/size jitter between neighbours (breathing-class pose
sets). `from_sprite55` currently stacks all 12 as coincident layers
(multi-image plain bodies: layers-vs-frames reading OPEN,
`crates/o2jam-shell/src/avatar.rs`). The exe room animates the
avatar/user strip live (capture-region fact, `docs/hover.md`: measured
strip bbox 485,50–726,255 animates between parked-pointer captures),
but no pinned source gives the exe's frame period or order for the
worn figure. **OPEN (named): cadence pin via a Wine dwell capture** —
tick-timed captures of the room avatar region (e.g. 10 Hz for 10 s
under the `recomp2jam-reference-capture` rig), diff to the frame
count (expected cycle length 12), derive the ms-per-frame period;
then implement the frame-cycle tick with that pinned cadence. No
cadence is invented until then.

## `load_worn_figure` mapping (unchanged code)

Per nonzero equipment id in slot order: resolve the
`AvatarItemRow` by id, run `pick_wear_sprites`, map each filename
`.oji` → `.ojs`, parse `sprite55`, stack decodable layers.

Do not confuse the two "slot" uses: the 17 equipment slots above
select *which item* is worn, while the 42 sprite slots per item
row select *which bodies* are drawn — slots 0–1 are shop icons
(never worn); from slot 2 up, even slots carry male variants and
odd slots female ones, with fallback to the other parity, kept in
slot order (`docs/file-formats/opi.md`; exe part logic OPEN).

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
