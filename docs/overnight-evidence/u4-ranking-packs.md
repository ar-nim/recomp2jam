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
`ushort MusicId` + `ushort NoteCount`. The sweep below corrects the
staged-word identity and narrows the OPEN.

### 2032 +0x64-slot sweep — narrowed OPEN (claim D23, 2026-11)

Static sweep (objdump 2.47 + Python scans + Ghidra decompile of
`FUN_0043e9b0`, `ev_9fb4178a…`).

Send shape: the request goes out as a virtual call on vtable slot
`+0x64` with arguments `(this, 0x7f0, 0)` — `push 0; push 0x7f0;
mov edx,[eax]; mov edx,[edx+0x64]; call edx` (files `0x4151a`–
`0x41532`, inside `FUN_0043e9b0`'s song-pick branch). All seven
image-wide call sites of this slot pass `0` as the third argument
(VAs `0x45e552`, `0x461ef2`, `0x46e53e` (code `0x15`), `0x46f6be`
(`0x1a`), `0x46f722` (`0x29`), `0x4717c8` (`0x30`), `0x474529`
(`0x3e8` Authorize)) — no call site ever hands a payload pointer to
the slot.

Staged state, corrected: right before the send the procedure zeroes a
`0x20`-byte area at `0x7797c0` (memset preface at file `0x40b73`) and
fills `music id u16` at `+0` (store file `0x41472`, from select-entry
record `+0`), `difficulty u16` at `+2` (store file `0x414f0`, from
select-entry record `+0x210` — the chosen difficulty 0/1/2, *not* a
note count; supersedes this log's earlier "note-count word" label),
and the cached best rank `u8` at `+4` (stores files `0x40f4b`/
`0x413f5`, from the `0x779814` best-table lookup). Every observed
reader of the area is dialog UI (e.g. files `0x45c74f` scanning the
`0x1b4` song vector by the staged id, `0x4513f0`); no packet-writer
reference to `0x7797c0`–`0x7797e0` exists image-wide. This matches the
pinned empty-body 2027/2030 requests, which share the `push 0; push
opcode; indirect slot call` send shape.

Verdict: strong static evidence that the 2032 request rides the wire
opcode-only and the staged words are dialog state, not wire fields.
Memoryer's `ushort MusicId + ushort NoteCount` body stays
corroborated-not-verified, and its `NoteCount` naming contradicts the
exe staging (the staged word is the difficulty). Still OPEN
(narrowed): the slot `+0x64` function body — the vtable target is not
statically recovered, so the slot could still copy staged state
internally. Next step per the ticket: proxy rewrite of the 2032 send
(recipe `~/scratch/digits-rig/proxy.py`), or a winedbg stopped-state
read of the dialog object's vtable slot `+0x64` naming the target.

## U4 gate globals sweep — PINNED (claim D24, 2026-11 static sweep)

Whole-image sweep (Python byte scans + objdump 2.47 slices) over the
read-only 7.55 copy (`OTwo.exe`, `O2Net32.dll`, `Player.dll`,
`SendWebData.dll`) naming every reference to the three gate globals,
the `0x779814` best-table district, and the `0x7797c0` staging area.

### `0x779964` — score-list-loaded one-way latch (PINNED)

The only store to this dword in any image is `movl $1,0x779964` at
file `0xb6a39` (VA `0x4b6a39`). It sits in the ranking-dialog notify
procedure's case `0x7ef` — the exact opcode the 2031 answer forwards
into the ControlList queue (dispatch compares files `0xb66d8`/`0xb66e2`,
VA `0x4b66d8`/`0x4b66e2`; case body from VA `0x4b6a07`). So receiving
the 2031 answer arms the latch. Both 2030 send sites consume it as
`cmpl $0x0,0x779964; jne skip-request` (files `0xac41e`, `0xb263b`):
latch clear → send 2030 (`push 0x7ee`, indirect `+0x5c`); latch set →
the dialog opens through the shared builder `FUN_004bd2e0` (seven call
sites image-wide: VAs `0x4aba17`/`0x4ac447`/`0x4b17a2`/`0x4b1d08`/
`0x4b2664`/`0x4b6a0d`/`0x4baa81`) with no new request. No clear site exists image-wide — the byte pattern occurs
only three times (the two gates plus this store) — so the client
fetches the best-score list once per process and then renders every
reopen from the cached `0x779814` table.

### `0x77ee86` — MemberRole word (gate value PINNED, label per D10)

2030 send site 1 gates `role == 2` (Challenger): word load file
`0xac402` (`mov 0x77ee86,%cx`), `cmp $2` file `0xac417`, `jne` past
the send. The role word is one of 37 load sites image-wide, all pure
loads (`66 a1`/`66 8b 0d`/`66 8b 15` forms); no store exists in any
image, so the writer is indirect (member-state block). Semantics per
claim D10: 0 None / 1 Champion / 2 Challenger / 3 Spectator — the
score-list request fires only for the Challenger seat.

### `0x77cd30` — own-seat-occupied byte (PINNED label)

All 50 references are byte loads (`a0`/`8a 0d`/`8a 15` forms), each
immediately copied to a stack slot and consumed as `movzbl`+`test` —
a boolean "seated in a room" gate (e.g. files `0xaad7a`, `0xb2622`,
`0xc4835`, `0xe53af`, `0x15f870`). 2030 send site 2 gates nonzero:
load file `0xb2622`, test file `0xb2635`. No store exists in any
image (indirect writer), matching its siblings in the `0x77cd00`
room-state block (local id byte `0x77cd04`, gate byte `0x77cd05`,
D-claims for 4040). Request fires only while seated.

### `0x779814` best-table district (PINNED structure)

The district is a `std::vector` triplet: begin `0x779814`, end
`0x779818`, cap-end `0x77981c`, with MSVC bounds checks on indexed
access (begin/cap compare + `call 0x5f1eca` at files
`0xc4e78`–`0xc4e9d`). Writers: (a) the 2031 handler append through
`FUN_004c5700` (file `0x15b60d`, pinned earlier); (b) the local
best-rank updater `FUN_004c4dd0`, which scans the rows (stride 6) and
writes `row[2 + index]` (files `0xc5027`–`0xc5069`, same split as the
D22 read side). The updater's only caller is file `0xc3b16`, inside
`FUN_004c38a0` — itself called from the result-screen verdict-fill
neighborhood (files `0xc32a4`/`0xc33af`, beside the `0x77ee86` role
reads at files `0xc32bc`/`0xc33b6`) — so the client updates its cached
best ranks locally after a result, not only from 2031. Readers:
`FUN_0043e9b0` scan (D22) and the 2031 append.

### `0x7797c0` staging district (enumeration complete)

Exactly 7 references image-wide, all dialog code: the memset preface
(file `0x40b74`), music-id stores at file `0x40fc8` (`mov %ax,0x7797c0`,
a second store site from a word-vector element, supplementing file
`0x41472`) and file `0x41472`, difficulty store file `0x414f0`, and
readers files `0x513f0`, `0x5c74f` (`movzwl` scan of the `0x1b4` song
vector by the staged id), `0x5c8fa`, `0x5c90b`. No packet-writer
reference exists image-wide — the staging words are dialog state only,
which keeps the D23 narrowing standing.

## OPEN list (named next steps)

1. 2032 wire staging: the `+0x64` slot takes `(this, opcode, 0)` at
   all seven call sites and no payload pointer; staged words are
   dialog state (music id / difficulty / best rank at `0x7797c0`).
   Remaining: the slot function body — proxy rewrite or stopped-state
   vtable read (claim D23, narrowed).
2. No live-server capture of 2027→2028 / 2030→2031 / 2032→2033 against
   Memoryer (server interop unproven; the 2029 note's live-capture
   pattern applies).
