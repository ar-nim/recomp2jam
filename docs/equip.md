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
| `EquipItemResponse` | 5005 (`0x138d`) | `i32` item id (4 B LE); `0` = remove/failure path |

Send builder case for 5004 (VA `0x4a4ec8`, file `0xa4ec8`):
stores the opcode as `u16`, stamps length `4`, then appends both
args as 4-byte words — hence two `i32`. Both C→S call sites push
slot then item: the shop-control path (VA `0x4a4af4`, file
`0xa4af4`) and the apply path (VA `0x4a4bcb`, file `0xa4bcb`).

The S→C handler is dispatched by `cmpl $0x138d` / `je 0x4a5fe8`
(VAs `0x4a5570`/`0x4a5586`, files `0xa5570`/`0xa5586`) and reads a
single `i32` (VA `0x4a5fe8`, file `0xa5fe8`): nonzero routes to
the equip-apply function (`0x4a3c80`, file `0xa3c80`), zero to the
remove function (`0x4a3dc0`).

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
- `EquipItemResponse` 5005 single-`i32` read + zero/nonzero
  apply/remove routing: VERIFIED (handler site above); any
  further body fields: OPEN (handler reads exactly one `i32`).
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
