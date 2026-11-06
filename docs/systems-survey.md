# Systems survey — unanalysed 7.55 game systems (static analysis)

Version pin: `AngelO2Jam.exe` 5,029,888 B, sha256 `83a04541…72c823b`.
Method: static analysis only (byte scans + `objdump -d` slices over the
local `install-root/` copy). No disassembly is reproduced here; every
claim names file offsets (VA = file + `0x400000`). No new REA dossier
was issued (the shared snapshot was contended); REA evidence IDs below
are pre-existing ones from cited docs.

Related: `docs/gameplay.md` (judgement/score/pills/jam/rings),
`docs/lifecycle.md` (stages), `docs/net-protocol.md` (packets),
`docs/file-formats/opi.md` (ControlList), `docs/file-formats/ojn.md`.

## 1. Rating-event sink (the `+0xa14` callback) — IDENTIFIED

**What it is.** The play object keeps a nullable rating-event sink
pointer at `+0xa14`. Every judgement site (autoplay, key-press, miss)
checks it for null and, when set, dispatches a small integer event
code through it. When it is null the game still scores locally, so
offline play never needs it.

**How it works.**
- Construction zeroes it: `movl $0` stores at file `0x1e916a`,
  `0x1ee900`, `0x1ee954`, `0x1ee98a`, `0x1ee9d7`, `0x1eea07`
  (VA `0x5e…` constructor thunks) and stack-context inits at file
  `0x204542`, `0x2047eb`, `0x225ae8` (VA `0x60…`/`0x62…`).
- Song setup attaches it: file `0x170325` (VA `0x570325`, inside the
  function starting ~file `0x1702b0`) stores the code address
  `0x56e170` into game `+0xa14`. The attach function is called from
  five loading-gate sites at VA `0x4dbac0`–`0x4dbc5a` (next to the
  old-format loader `FUN_004dc390`), i.e. attach happens during
  song-load, not at boot.
- Chart (re)load detaches it: file `0x1717cc` (function ~`0x1717c0`)
  stores `$0`; called from VA `0x5716b7` and `0x573937` (parser/slot
  init region).
- Dispatch sites (all `null? skip : call`, ~30 of them across VA
  `0x575dd2`–`0x577f8b`, i.e. the whole judgement/playback region):
  autoplay path at file `0x176691`–`0x1766fd` (VA `0x576691…`),
  key-press path at file `0x176bdf`/`0x176c1e`
  (VA `0x576be0`/`0x576c1e`), and a third site at file
  `0x176ea9`/`0x176ee6` (VA `0x576eaa`/`0x576ee6`).
- Event codes observed at call sites: `1` = plain judgement (5-arg
  call carrying note word, rating class, song tick and the note
  field at game `+0x824`), `0xe` (14) = hold-note judgement (same
  shape, taken when the note payload has bit 25 `0x2000000`), `9` =
  third event with a zeroed middle arg (exact meaning open).
- The target `FUN_0056e170` (file `0x16e170`, VA `0x56e170`) is a
  genuine function: prologue, subtracts 1 from the event code,
  rejects codes above `0xf`, and jumps through a 16-entry table at
  VA `0x56ec5c`. So the sink protocol supports up to 16 event
  codes; only 1/9/14 are seen at judgement call sites.

**Verified vs open.** VERIFIED: attach/detach lifecycle, null-guard
shape, event codes 1/9/14, 16-way dispatcher with jump table.
OPEN: what codes 0,2–8,10–13,15 do; what concrete object the sink
notifies (relay `ComboStarted/ComboBroken` packs are the prime
suspect — see `docs/net-protocol.md` §4d ops 6/7); whether the
`+0x824` argument is the song tick.

**User question.** In live multiplayer, which on-screen or network
events fire per judgement (combo toast, opponent lane flash, packet
light)? That tells us which dispatcher case is which.

## 2. Life gauge, HP and fail rules — PARTIAL (UI mapped, HP store open)

**What it is.** A life bar shown during play, a game-over splash at
song end on failure, and (per the score-submit packet) a per-note
life value sent to the server.

**How it works (traced).**
- Sprite anchors: `Life_gauge_left_bar.ojs` (VA `0x64b1d8`, file
  `0x24b1e8`) and `Life_gauge_right_bar.ojs` (VA `0x64b204`, file
  `0x24b204`) are loaded by the play-scene loader `FUN_004e7850`
  (dossier `ev_40f99221…`) at file `0xe7d10`/`0xe7ca9`
  (VA `0x4e7d10`/`0x4e7ca9`) — the same loader that mounts the jam
  bars, so life and jam gauges share one setup path.
- Fail splash: `Image_GameOver_Image.ojs` (static, VA `0x64af6c`,
  file `0x24af6c`) vs `.ojt` (animated, VA `0x64af88`, file
  `0x24af88`) are selected by a predicate call at file `0xe69a1`
  (VA `0x4e69a1`) inside UI-builder ~`0x4e5d90`: nonzero predicate
  → animated variant, zero → static. The handle is kept at object
  `+0x7c8` and laid out with three positioned elements. A third
  reference at file `0xee0b6` (VA `0x4ee0b6`, inside ~`0x4ed650`)
  belongs to a different (likely result/exit) path.
- Score-adjacent gauge update: scorer `FUN_00570510` (file
  `0x170510`) re-reads game word `+0x3970` after applying a rating
  and, when it changed, issues a virtual UI notification through
  `this+0x4` (vtable+8) with params `(0, 0, 0x66, 0x12)` at file
  `0x1707c6`–`0x1707db` (VA `0x5707c6`–`0x5707db`). `+0x3970`
  gates every rating arm (Cool/Good/Bad/Miss all skip their
  effects when it is zero), consistent with a live/dead or
  HP-zero gate.
- Score field pin: the Miss arm compares game dword `+0x3980`
  against 10 and subtracts 10 or zeroes (file `0x17087f`–`0x17089a`),
  confirming `+0x3980` is the score accumulator.
- The `[-10,-7,-5]` row at file `0x35e844` (VA `0x75e844`, in
  `.data`) has NO code or data references anywhere in the image
  (full-image LE32 scan), so it is either padding-coincidence or
  reached through a computed address; do not cite it as the HP
  table without a reader.

**Verified vs open.** VERIFIED: gauge sprite load sites, GameOver
variant-select shape, `+0x3980` = score, `+0x3970` = rating gate +
UI-update trigger, no in-image refs to `0x75e844`. OPEN: which
field is HP (no `.text` writer to `+0x3970` exists — it is only
read, so HP changes go through the `0x5409xx`/`0x540axx`
sub-object calls or bulk-init copies at VA `0x4df1bf`–`0x4dffb1`);
per-rating HP deltas; the exact fail threshold and whether fail
ends the song early or only picks the splash.

**User question.** What does the life bar do on each judgement in
7.55 (how much does Cool heal vs Bad/Miss damage, and does the
song stop early on empty — or do you always finish and just get
the GameOver splash)?

**Answered by player (2026-11-07, UNVERIFIED in-exe):** max 1000
(possibly 100%); Cool heals +3/+2/+1, Good +2/+1/+0, Bad nothing,
Miss −50/−40/−30 by Easy/Normal/Hard; at 0 the player is dead
(no more input) and returns to the waiting state. Implemented as
`o2jam-game::LifeGauge` with synthetic tests. Binary note:
`[3,2,1]`/`[2,1,0]` fragments exist but are unattributed, and the
unique `[-10,-7,-5]` triple contradicts Bad=no-change, so it
belongs elsewhere until proven.

## 3. Long-note implementation — STRONG LEADS (option plumbing mapped)

**What it is.** Sustained notes with a head, a hold span and a tail,
shaped by three `option.ini`/`Option.ini` keys.

**How it works (traced).**
- `NOTE`/`NoteType` (option.ini): integer 0–2 clamped at file
  `0x1713c7`–`0x171402` (VA `0x5713c7`…), stored at play-object
  `+0xeb4`; plus a per-level string form — level `0..2` is
  formatted as `"Note0"`–`"Note2"` and read with
  `GetPrivateProfileStringA("NOTE", name, …)` at file
  `0x171434`–`0x171447` (VA `0x571434`…), result also stored at
  `+0xeb4` (setter ~`0x571410`, getter ~`0x571460`). So NoteType
  has both a numeric and a per-level string form.
- `NOTE`/`NoteTail` (option.ini): integer global clamped 0–4 at
  file `0x16bf47`–`0x16bf75` (VA `0x56bf47`…), kept in the global
  at VA `0x67feac`; read back in note-spawn/render code at file
  `0x16b97b`, `0x16b98e`, `0x16bf5f`–`0x16bf6e`
  (VA `0x56b97b`…, `0x56bf5f`…) and in UI code at file `0xe1c62`,
  `0xe1d35`, `0xe3575`, `0xe36cf`, `0xe8c08`, `0xef27f`, `0xf265f`.
- `LONGNOTEMODE`/`LongnoteMode` (Option.ini, capital O): read at
  play setup at file `0x16fb0d`–`0x16fb17` (VA `0x56fb0d`…) and
  cached at play-object `+0x17d8`, followed by zeroing two `0x1c`
  buffers at `+0x17fc`/`+0x17e0` — i.e. the mode is snapshotted
  per song, not re-read mid-play.
- Parser side (`docs/file-formats/ojn.md`): chart words with bit 26
  (`0x4000000`, `and` at file `0x172a4f`/`0x173061`) set a
  1000-byte hold table; judgement side: note payloads with bit 25
  (`0x2000000`) take the hold branch — the sink gets event `0xe`
  instead of `1` (file `0x1766d1`–`0x1766fd`), and the key-press
  path tests payload bits 24–25 (`0x3000000` mask, `== 0x2000000`
  means hold span; file `0x176c3c`–`0x176c48`).

**Verified vs open.** VERIFIED: all three option reads, clamps,
storage (`+0xeb4`, `0x67feac`, `+0x17d8`), the parser bit-26
marking sites, the judgement bit-25 branches. OPEN: what the
0–2 NoteType values and the 0–4 NoteTail values actually change
on screen; how hold ticks accrue score (the submit packet has a
dedicated `longNoteScore` field — `docs/net-protocol.md` §4c);
whether `LongnoteMode` alters timing or only rendering.

**User question.** What do the in-game long-note options look like
(names/choices in the option dialog), and does holding drain or
heal life per tick?

**Answered by player (2026-11-07):** no long-note options (at most
a cosmetic long-note-combo toggle). Long notes are chart data:
hold from the head, MUST release at the tail (missing the
release is a miss), releasing mid-hold is a miss.

## 4. Result/grade computation — PARTIAL (builder found, strings dead)

**What it is.** The post-song screen (`CStateResult`) showing
win/lose art, numeric fonts, and (allegedly) grade strings.

**How it works (traced).**
- Builder `FUN_004c2390` (~file `0xc2390`, VA `0x4c2390`)
  references `PointNum.ojs` (file `0x24a328`) at file `0xc2814`
  and `Result_WinLose.ojs` (file `0x24a35c`) at file `0xc23d7`;
  contiguous art block at file `0x24a338`–`0x24a38d` holds
  `Result_NumFont_Blue.ojs`, `Result_WinLose.ojs`,
  `StateResult_Bottom.ojs`, `StateResult_Top.ojs` — the full
  result-screen sprite set in one place.
- Grade strings (` ALL COMBO`, `ALL GOOD`, `COOL 95%`, `0 JAM`,
  `LIFE 100%`, `MISS 10`, … at file `0x247362`–`0x2476dd`, VA
  `0x647362`…) have ZERO references in `.text`, `.rdata`,
  `.data` or `.tls` (exhaustive LE32 scan). They are not code
  data. Same verdict for the `xTd` block (see §7): no refs.
  Thematic grouping (COMBO/COOL%/GOOD%/JAM/LIFE/MISS thresholds)
  matches planet-mission clear conditions better than a result
  grade — likely consumed by mission scripts/pages, not the exe.
- Network shape is known: `4016 SubmitScore` carries
  cool/good/bad/miss/maxCombo/jam/maxJam + score + life + speed +
  longNoteScore; `4041 FinalizeRank` closes ranking
  (`docs/net-protocol.md` §4a–4c).

**Verified vs open.** VERIFIED: result builder entry, art set,
grade-string absence (full-image), submit/rank packet fields.
OPEN: score→rank/grade mapping (server-side?); which function
fills the result numbers; retry/exit flow into `STATE_WAITING`.

**User question.** Does 7.55 show letter grades (S/A/B/…) anywhere,
or only combo/rate numbers — and what decides the WIN/LOSE art?

**Answered by player (2026-11-07):** grades exist; WIN/LOSE goes
to rank #1 (solo or team) by highest score. Thresholds OPEN.

## 5. Renderer blits — INVENTORIED (DirectDraw/CDX path)

**What it is.** 2D sprite compositing: 16-bit RGB555 art
(`docs/file-formats/opi.md` sprite-55) drawn with color keys,
then handed to a 3D device as textures.

**How it works (traced).**
- Imports: `DDRAW.dll!DirectDrawCreateEx` (IAT RVA `0x23f01c`),
  `DINPUT.dll!DirectInputCreateA`, `DSOUND.dll` — DirectDraw +
  DirectInput + DirectSound, no Direct3D DLL import (D3D arrives
  via statically linked helpers: `m_pd3dDevice->SetTexture(0,
  m_pSurface->GetDDS())` / `GetDDSAlpha()` strings, `Direct3DX`
  error strings, `.\Display\DXSurface.cpp` path string).
- RTTI classes: `CDXSSurface`, `CDXSurface`, `ImageSurface`,
  `SpriteSurface` (file `0x27…` RTTI region) — CDX sprite/surface
  hierarchy intact.
- `DirectDrawCreateEx` has no direct `call`/`lea` reference in
  `.text` (LE32 scan of the IAT VA `0x623f01c` is empty), so the
  device is created through a runtime-resolved pointer (CDX init)
  or COM (`CoCreateInstance` via ole32, imported).
- Failure strings name the blit path: `BitBlt`, `BltFast`,
  `Flip`, surface-lock/restore, clippers, color keys — i.e.
  lock–blit–flip with color-key transparency. Color-key values
  match sprite-55 (`0x0000`, `0x7D5F` in `parse_sprite55`).
- Non-power-of-two scanline unevenness noted in `opi.md`
  (sprite-55 widths) is a CPU-side pitch artifact of this path.

**Verified vs open.** VERIFIED: import set, class names, source
file, dual-surface texture upload shape, blit/Flip vocabulary.
OPEN: display mode (800×600×16?), backbuffer count, frame-pacing
source (`timeGetTime` vs audio clock), exact blit call sites.

**User question.** Did 7.55 run fullscreen or windowed, at what
resolution, and did notes visibly tear/stutter on slow machines
(blit-bound) or stay smooth (flip-bound)?

**Answered by player (2026-11-07):** 7.55 runs fullscreen 800x600;
no windowed mode in this version. Renderer default recorded.)

## 6. Shop, items and equipment — INVENTORIED (tables located)

**What it is.** Cash shop + song shop + avatar equipment, driven by
server packets and two local tables.

**How it works (traced).**
- Local tables (both in `Image/avatar.opa`, 144,761,749 B):
  `SetInfodata.ojs` (slot 7043, offset 136063397, 32463 B;
  header u32 count 210) and `Itemdata_Korea.dat` (slot 7148,
  offset 142934156, 740945 B; header bytes
  `06 06 00 00 00 00 00 00 01 00 00 00 …`). The consumable ring
  rows (`11A080xx`) with CP949 effect text and score/exp/jam
  bonuses are already catalogued in `docs/gameplay.md`.
- Code side: RTTI `CItemManager` (file `0x27eb40`), plus
  `CMyBag`/`CMyCart`/`CBillManager`, shop states `STATE_SHOP`
  (`0xA4`) / `STATE_MUSICSHOP` (`0x6B`) and dialogs
  (`DIALOG_SHOPPINGBAG_ADD`, `DIALOG_CHARGECASH`, …) in
  `docs/lifecycle.md`; packet families 5000–5041
  (buy/sell/equip/enter/exit, cash, gifts, bag) in
  `docs/net-protocol.md` §4a.
- Equip path opcode `5004 EquipItem` exists; per-item stat
  effects are server-authoritative (no local stat math found).

**Verified vs open.** VERIFIED: table locations/sizes/headers,
ring catalog, packet family, UI blocks. OPEN: `SetInfodata`
row stride; `Itemdata` row stride beyond the header; avatar stat
effects (client-visible at all?); equip persistence format.

**User question.** Did avatar items change gameplay numbers you
could feel (faster jam gain, more life), or only score bonuses —
and were any items client-side toggles vs server flags?

**Answered by player (2026-11-07):** avatars are cosmetic except
skill rings (random/panic/sudden/hidden/dark), which are the
only gameplay-affecting items.

## 7. Quick inventory hits (for other agents)

- **Option-key table (complete).** File `0x245224`–`0x2452a8`
  (VA `0x645224`…), NUL-padded, read with
  `GetPrivateProfileIntA` (IAT `0x63f268`) /
  `GetPrivateProfileStringA` (IAT `0x63f26c`): section `MODE` key
  `Mode`; `NoteScreen`/`BlendHalf` (default `"1"`); `LONGNOTEMODE`
  (section) / `LongnoteMode` (key) via `Option.ini` (capital O);
  `NOTE`/`NoteTail`, `NOTE`/`NoteType` via `option.ini`
  (lowercase o). No other gameplay keys live here — key binds,
  volumes, resolution are elsewhere (open).
- **`xTd` paired table.** File `0x2454d8`–`0x2455dc`: ~57
  four-byte entries `<byte> 54 64 00` (`?Td`) then `<byte> 53 64
  00` (`?Sd`), one `?Bd` at file `0x245538`, preceded by u16-pair
  rows (`a4 bb`… at file `0x2454a8`) and followed by a pointer
  table at file `0x2455e0`. Referenced from `.text` at file
  `0x448c2`, `0x44973`–`0x44c4b`, `0x44ecc`, `0x451f0`,
  `0x4545f`, `0x46fef`, `0x476b9`–`0x47e12`, `0x108345`,
  `0x10a3e9`, `0x1f0b53`, `0x1f14f4`, `0x1ffb6d` (VA +`0x400000`)
  and from data refs at file `0x3cfa8`–`0x3d193`. First bytes
  descend (`d0…88`, `1c…04`, …) like key codes with a display
  suffix — prime suspect is a key-name table for the option/key
  dialogs. Untouched: decode + consumer identification.
- **OJNList trailer.** File is 719×300 B records + 148 B trailer
  (215848 B total). Records carry per-difficulty levels at
  +280/+284/+288 (e.g. 129/129/129 for song 757), record size
  300 at +292, unknown u32 at +296. The trailer (and the last
  full record) hold 28-byte date-stamped entries: tag `;HD`,
  constant `0xbd11`, descending ids (`0x64`–`0x69`, `0x73`,
  `0x74`, …), zero-terminated `YYYY-MM-DD` (`2009-09-17`,
  `2009-09-16`, `2004-04-09` ×several), 5-byte marker
  `77 30 f7 12 00`. Reads as a patch-added-songs manifest, not
  song records (no titles). Untouched: exact semantics of `;HD`
  and the `0x12f73077` marker.
- **Grade-string block.** See §4 — full-image unreferenced;
  do not spend RE time looking for in-exe readers.
- **Already covered, do not redo.** OJM/M30/OMC/new-variant;
  OJN decrypt/levels/note semantics/judgement/combo/pills/score;
  OPI/OPA inner formats; OJNList layout (records); lifecycle;
  net endpoints/packets; skill rings incl. lane builders
  (`0x173940`/`0x173a10`) and curtains; autoplay; shell/input/
  audio backend.

## Methods note

All addresses above come from LE32 byte-pattern scans of the
image (string VAs, IAT VAs, field displacements) plus
bounded `objdump -d` windows, cross-checked against the
`.text`/`.rdata`/`.data` section map from `objdump -h`.
Function labels `FUN_*` are Ghidra-style conveniences, not claims
of new REA dossiers.
