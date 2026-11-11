# U4 ranking packs — evidence log

Wire layouts for `2027 GetMusicPlayRanking`, `2030 GetMusicScoreList`,
`2032 GetMusicMaxScore` (C→S) and `2028`/`2031`/`2033` (S→C),
implemented in `crates/o2jam-net` (`rooms.rs`, `command.rs`).
Read-only walks below; offsets are file offsets unless marked VA.
Version pin: `OTwo.exe` 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`
(`install-root/O2Jam/`, matches `docs/plan/usable-client-plan.md` pin).

## Dispatcher (shared, GNU objdump 2.47 + Python scans, no dossier)

The S→C dispatch subtracts base `0x7d3` (file `0x1582ec`), bounds
`0x20` (file `0x1582f8`), and indexes the address table at file
`0x158940` (mechanics per the 2029 note in `docs/net-protocol.md`
§4c). Relevant rows:

| Index | Opcode | Table entry | Lands |
|---|---|---|---|
| `0x18` | 2027 | `0x558937` | default case (file `0x158937`) — no client handler |
| `0x19` | 2028 | `0x558545` | stub → handler VA `0x55b230` (file `0x15b230`) |
| `0x1a` | 2029 | (existing note) | handler VA `0x55b4d0` |
| `0x1b` | 2030 | `0x558937` | default case — no client handler |
| `0x1c` | 2031 | `0x558567` | stub → handler VA `0x55b590` (file `0x15b590`) |
| `0x1d` | 2032 | `0x558937` | default case — no client handler |
| `0x1e` | 2033 | `0x558578` | stub → handler VA `0x55b640` (file `0x15b640`) |

Each stub is `mov reg,[ebp+8]; push reg; call rel32` (9 bytes);
call targets verified arithmetically from the rel32 bytes.

## 2028 `MusicPlayRankingAnswer` (S→C) — PINNED

REA/Ghidra dossier `extracted/u4-ranking/ev-2028-handler.json`
(`ev_7f09b7fa1ff255e32461d3ade0b273a1554cbbcf31a67deb3190a4b65521855f`,
Ghidra 12.1.2, `rea function … 0x55b230`); independent `rea decompile`
of file `0x15b230` agrees. Handler behavior, in own words:

- Read one `i32` rank word via the packet-reader helper (`0x555d20`).
  Zero → push one default (zeroed `0x124`-byte) row into the rank-row
  vector at `0x77968c`. Nonzero → read a string via the string-reader
  helper (`0x555ea0`), then three `i32`, one `u8`, one `i32` via
  `0x555d20` (pushes at files `0x15b302`/`0x15b30e`/`0x15b31a`/
  `0x15b326`/`0x15b335`), and push the filled row.
- Read an `i32` count. Zero → push ten default rows (display fill, no
  wire data). Otherwise loop that many rows in the same
  word-then-optional-fields shape (signed compare, so negative counts
  paint nothing).
- Forward (`0x12`, `0x7ec`, 0, 0) into the ControlList UI queue
  (`0x478d50`; pushes file `0x15b4a3`, opcode push file `0x15b4a7`).

The word is the row's *rank* (not an invalid-gate: both arms push and
continue, unlike 2029/2033 which skip the packet on nonzero). Field
order corroboration: Memoryer
`Messages/Responses/Ranking/MusicPlayRankingResponse.cs` — `RankEntry`
(rank `int`, nickname string, battles/wins/win-rate `int`s,
`RankDeltaType` byte, rank-delta `int`), leading `Self` plus an
`i32`-counted `Entries` list.

## 2031 `MusicScoreListAnswer` (S→C) — count + width PINNED, row split corroborated

REA/Ghidra dossier `extracted/u4-ranking/ev-2031-handler.json`
(`ev_299779a2…`, `rea function … 0x55b590`); `rea decompile` of file
`0x15b590` agrees. Handler behavior, in own words:

- Read one `i32` result word. Negative → skip rows, still forward.
  (Signed `jge`-style gate; same treatment as 4018's count word.)
- Else read a `u16` count, then that many 6-byte rows (`push 6` at
  file `0x15b5fa`, read call `0x15b603`) appended to the
  per-song/difficulty best table at `0x779814` through `FUN_004c5700`
  (decompiled separately: divides the vector span by 6 — generic
  push-back, no per-field semantics).
- Forward (`0x12`, `0x7ef`, 0, 0); pushes files `0x15b618`/`0x15b61c`.

Row split (`u16` music id + four rank bytes) is Memoryer shape —
`Messages/Responses/Ranking/MusicScoreEntry`: `ushort MusicId` plus a
`Dictionary<Difficulty, Rank>` with both enums byte-backed
(`Difficulty : byte` EX/NX/HX/RX, `Mozart.Metadata.Rank : byte`
None/F/E/D/C/B/A/S/Perfect) and an `Empty`-prefix collection (no
length word, inline values in `Enum.GetValues<Difficulty>()` order) —
against an exe-verified 6-byte width.

### 2031 per-byte difficulty order — PINNED (claim D22, 2026-11 static sweep)

Static sweep (GNU objdump 2.47 slices + Python byte scans + one Ghidra
decompile of the host procedure, `ev_9fb4178a…`) over the read-only
7.55 copy pins the paint-path read. The result/select procedure
`FUN_0043e9b0` scans the best-table vector at `0x779814` row by row:
iterator step `add $0x6` (file `0x40d5b`–`0x40d64`), row `u16` at
byte 0 compared against the select entry's music id (load/compare
files `0x40d78`–`0x40da2`), matched-row iterator kept in a local. It
then reads `row[2 + d]` where `d` is the selected entry's difficulty
word at `entry+0x210`: `movzwl 0x210(edx,eax,1)` (catalog record
stride `0x238`) into the index, `mov 0x2(ecx,eax,1),%dl`,
`mov %dl,0x7797c4` (files `0x40f37`–`0x40f4f`); a byte-identical
second site at files `0x413e1`–`0x413f5`. `entry+0x210` is the chosen
difficulty — 0 = Easy, 1 = Normal, 2 = Hard — stored by the
select-entry builder (VA `0x4441f0`, store file `0x4425d`, ev
`b5c3094a…`; see `docs/file-formats/ojn.md`).

Per-byte order of the 2031 body, exe-pinned: bytes 0–1 = `u16` music
id (the scan key, not difficulty-indexed); byte 2 = Easy best rank;
byte 3 = Normal; byte 4 = Hard. Byte 5 (the fourth rank slot of the
exe-pinned 6-byte row) rides the wire but has no reader on this path.
The cached rank byte lands in global `0x7797c4` and drives the
`Result_Imgs_Rank` frame chooser (reads at files `0xce79`/`0xceb2`/`0xcef3`/`0xcf34`
with compares against 1/6/8 — the `Rank` byte values, matching
None/F/…/S/Perfect). Memoryer's `Enum.GetValues<Difficulty>()` order
EX/NX/HX/RX agrees for slots 0–2; the RX byte stays
Memoryer-only. The write side uses the same split: the best-rank
updater writes `row[2 + index]` (files `0xc5027`–`0xc5069`).

## 2033 `MusicMaxScoreAnswer` (S→C) — PINNED

REA/Ghidra dossier `extracted/u4-ranking/ev-2033-handler.json`
(`ev_45b5f508…`, `rea function … 0x55b640`); `rea decompile` of file
`0x15b640` agrees. Handler behavior, in own words:

- Read one `i32` invalid word (`push 4`, call `0x555d20`, files
  `0x15b654`/`0x15b65d`); nonzero → return without forwarding.
- Else read one `u32` straight into `0x7797c8` (push file `0x15b668`,
  call `0x15b672`), then forward (`0x11`, `0x7f1`, 0, 0) only while
  the ControlList parent (`0x762b44`) shows dialog kind 7.

Memoryer `Messages/Responses/Ranking/MusicMaxScoreResponse.cs`
(bool `Invalid` as `i32`, `u32 MaxScore`) corroborates the two-word
shape against the exe-verified read sequence.

## 2027 request (C→S) — empty, no exe send

Whole-image scans (Python `re`, exact bytes) over `OTwo.exe`,
`O2Net32.dll`, `Player.dll`, `SendWebData.dll`: zero hits for the
`0x7eb` dword and zero hits for `68 eb 07 00 00` (push 2027). The
7.55 client never sends 2027 (lanking board is server-pushed 2028 or
unfetched); Memoryer's `RankingController.GetMusicPlayRanking` reads
no request fields. Client fetch encodes an empty body.

## 2030 request (C→S) — empty, two exe send sites

- Site 1, file `0xac427` (`push 0; push 0x7ee`, indirect `+0x5c`
  send): gated on the menu-state word at `0x77ee86` being `2` (word
  load file `0xac402`, `cmp 2` file `0xac417`) and dword `0x779964`
  being zero (`cmp` file `0xac41c`).
- Site 2, file `0xb2644` (same shape): gated on the seat byte at
  `0x77cd30` being nonzero (load file `0xb2622`) and `0x779964`
  being zero (`cmp` file `0xb2639`).
- Neither site stages body bytes between the gate and the send;
  Memoryer's handler reads no request fields. Empty body both sides.
  The gate globals stay OPEN (no labels pinned).

## 2032 request (C→S) — Memoryer shape, wire staging OPEN

`rea decompile` of the send region (file `0x4151a`, inside the large
`FUN_0043e9b0`; dossier `extracted/u4-ranking/ev-2032-send-function.json`,
`ev_2e7c7bc9…`): once the entry passes the music-list membership
predicate, the code stores the selected music id to `0x7797c0`
(store file `0x41472`) and its note-count word to `0x7797c2` (store
file `0x414f2`), runs the row sprite setter (`0x42f870`), then sends
pin-opcode-only (`push 0; push 0x7f0` at file `0x4151c`, indirect
`+0x64` slot call at file `0x41532`).

Memoryer `Messages/Requests/Ranking/MusicMaxScoreRequest.cs` decodes
`ushort MusicId` + `ushort NoteCount` — matching the two staged
words — so the request encodes them, but the transfer of those words
onto the wire through the `+0x64` slot is unobserved and stays OPEN.

## OPEN list (named next steps)

1. 2031 row per-byte difficulty order: find the result-screen lookup
   over the `0x779814` vector (row scan by music id + index by
   difficulty) and pin the index math.
2. 2032 wire staging: trace the `+0x64` send-slot helper's body
   mechanism (reference: Authorize site file `0x74529`) to confirm or
   deny the two staged words ride the frame.
3. Gate globals `0x77ee86`, `0x77cd30`, `0x779964` labels.
4. No live-server capture of 2027→2028 / 2030→2031 / 2032→2033 against
   Memoryer (server interop unproven; the 2029 note's live-capture
   pattern applies).
