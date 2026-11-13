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

## Avatar idle animation (ticket #176) — OPEN

### What is pinned

- Every probed worn body carries 12 chained breathing frames (verified:
  `avatar.opa` bodies such as `01M010010001.ojs` decode to exactly 12
  sprite-55 images sharing one record anchor, e.g. `(130, 78)`; the
  single-frame siblings such as `01M01001.ojs` carry 1). The current
  `load_worn_figure` flattens all 12 images into `AvatarFigure.layers`,
  so they stack at paint time instead of animating. Treating them as
  animation frames (paint one per clock) is the OPEN implementation arm.
- The paint site is `AvatarFigure::paint_visible_centered` (room panel
  `paint_room_user_panel` and waiting seats `paint_figure_in_seat_slot`
  in `crates/o2jam-shell/src/screens/paint.rs`); both call it with a
  `&Rect` slot and no clock today.
- Timed-cycle precedent: `PLANET_PROMO_TICK_MS` (3000 ms) via
  `tick_planet_promo_at` / `tick_promo_clocked` in
  `crates/o2jam-shell/src/screens/game/planet.rs`, pulsed by the
  `idle_repaint_interval` heartbeat (`Screen::Planet` arm, 1000 ms
  heartbeat in `crates/o2jam-shell/src/screens/game/mod.rs`).

### Static attempt (2026-11-13)

- Located the generic sprite-frame machinery in `OTwo.exe` (VA/file
  per `docs/claims.md` A10): frame index lives at sprite-control
  `+0x134` (file `0x1e050`/`0x1e06e`), frame table at `+0x120`
  (stride `0x14`, signed anchor words at record `+0`/`+2`), and the
  setter at VA `0x41e0f0` (file `0x1e0f0`) stores the index only when
  it is below the image count returned by the getter at `0x41e140`.
- A direct cross-reference scan of the image for the setter/getter VAs
  (`0x41e0f0`, `0x41e120`, `0x41e0d0`, `0x41e140`) returned zero hits:
  they are reached through the sprite control's vtable, not by direct
  `call`, so the avatar-specific advance site could not be isolated by
  byte scan. No avatar-class (`CAvatar`) method references survived as
  strings beyond the RTTI name at file `0x27ea50`; `Avatar.opa` string
  refs (files `0xa739`, `0x10ffb`) are archive-load sites, not paint
  or timer sites.
- No immediate resembling a breathing cadence (no `cmp`/timer against
  a small constant) could be attributed to the avatar paint district
  from bytes alone.

### Empirical attempt (2026-11-13)

- Launched `OTwo.exe` (SHA `83a04541…72c823b`) under `scripts/rig-run.sh`
  on display `:101` with a null PulseAudio sink and the local Memoryer
  (`127.0.0.1:15010`, token for `USER_INDEX_ID=2`); the rig produced a
  bright frame (`bright=4800`), confirming the client boots, but driving
  it through login, channel select, and into a room/waiting scene with a
  worn figure requires the full SendInput sequence (X-plate template
  detection + dialog dismissal) documented in
  `~/scratch/button-unblock/`. That sequence was not completed within
  this attempt, so no dwell capture and no frame-change timestamps were
  obtained.

### Result

- **Cadence: OPEN.** No ms/frame value is pinned; no value is invented.
- **Driver: OPEN.** The generic sprite-frame setter is pinned
  (`0x41e0f0`/`+0x134`/`+0x120`), but the avatar-specific advance
  (paint-clock accumulator vs. timer tick, and the cadence constant)
  is not.
- **Implementation: not landed.** Per the ticket hard rule, no code
  change ships without a pinned cadence. The natural landing shape,
  once pinned, is: add a frame index derived as `f(now_ms, cadence)` to
  `paint_visible_centered` (and thread `now_ms` through
  `paint_room_user_panel` / `paint_figure_in_seat_slot`), select one
  image per 12-frame body instead of stacking all 12, and give the
  Room/Waiting scenes an `idle_repaint_interval` pulse (reusing the
  250 ms net-drain heartbeat if it covers the cadence, else a
  dedicated arm) — mirroring `tick_planet_promo_at`.
- **Guard: not added.** A failing-first guard (staged worn figure,
  clock advanced by cadence → painted frame advances; clock pinned → no
  advance) is specified but cannot be written against an unpinned
  number.

Next evidence step: complete the SendInput-driven Wine dwell capture on
a worn-figure scene (room panel or waiting seat) and record frame-change
timestamps over 10–20 s; in parallel, static-trace the vtable callers of
`0x41e0f0` to find the avatar advance site and its cadence immediate.
