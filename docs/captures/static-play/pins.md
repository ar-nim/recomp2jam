# Static pins — Playing + Tutorial + Loading static debts

Offline leg only: no exe boot, no server, no code or tracker writes.
Worktree `cb2/static-play` at `origin/main` `373b670b24ae3027b6146c895d6982f06b5ef231`.

Target: `/home/arnim/scratch/run-full/OTwo.exe`, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`,
32-bit PE, `.text` VMA `0x401000` / file `0x1000`, so file offset = VA − `0x400000`.

Tools: `rea` 6.8.0 CLI, GNU objdump 2.47, radare2 6.2.2, python3 3.14.
REA MCP tools were not registered in this session; REA CLI Ghidra binding was
unavailable (installed Ghidra 12.2, REA 6.8.0 accepts 12.1.x). REA CLI
Evidence IDs below are real CLI IDs for artifact identity/sections; behavior
IDs `S-PLAY-B1`…`B4` are this leg's local static-analysis IDs, not REA
Evidence. Prior dossier IDs from repo docs are marked "reused" and their full
forms are not present in the reused packs. No disassembly or decompiled code
is stored here. Incremental log: `/home/arnim/scratch/static-play-findings.md`.

Evidence IDs:

| ID | Source | Scope |
|---|---|---|
| `ev_665795be85766c09666a5ef901eb071a2cdc24fd2887cc61eaa0332aa2a72dbe` | REA CLI `inspect-with-objdump file_headers` | exe identity |
| `ev_04299f42538c76424eef10c4d0aec581874a418000002d5b0dc08fa42183756e` | REA CLI `inspect-with-objdump section_headers` | VA ↔ file mapping |
| `ev_4164031c…` | reused prior pin | old `0x550db0` pointer |
| `ev_68d920c2…` | reused prior pin | loading gate `0x4e47a0` |
| `S-PLAY-B1` … `S-PLAY-B4` | this leg, objdump/r2/python3 | behavior below |

## 1 — VA `0x550db0` and VA `0x67fe8c` (`S-PLAY-B1`, `S-PLAY-B2`)

### `0x550db0` (file `0x150db0`) — boot-time OJM bank loader

Behavior: one call loads one of seven background/UI OJM sample banks. It
creates a record reader, opens the bank named by `0x67f730 + index*0x48`
(`Music/BGEffect.ojm`, `Music/O2PlanetNPC.ojm`, `Music/BGM.ojm`,
`Music/Event.ojm`, `Music/Planet.ojm`, `Music/BGMWinter.ojm`,
`Music/DialogBGM.ojm`), then reads records until the reader is empty. Codes
below `0x17` go to the new-variant reader `0x54f580` and allocator `0x54b9f0`;
codes below `0x34` go to the M30 reader `0x54ef60`, slot base
`0x67f728 + index*0x48`, sample array `manager+0x34150 + slot*4`, allocator
`0x54ca60`. Return 1 on completion, 0 on open/read failure.

Offsets: entry `0x550db0`; open `0x550df9`–`0x550e0b`; record pump
`0x54ebe0` at `0x550e41` / `0x54eca0` at `0x550e6b`, `0x5511f7`;
M30 slot `0x550f96`; sample store `0x5510b0`; decode `0x5510fc`;
success return `0x551212`; end `0x55124c` (`ret 4`). Rizin size 1183 B,
one stack arg (bank index) plus `ecx` manager.

Callers (exhaustive `E8 rel32` + whole-file pointer scan):
`0x550b08` (only call) → `0x550ae0` loops indices 0..6 (`cmp 7` at
`0x550afb`) → `0x5a5afd` in `0x5a5710` (sound-device init; warning string at
`0x650d98`) → `0x5ad5d0`/`0x5ad5f6` in `0x5ad350` (mode 1/2 start) →
`0x5fb34a` from PE entry `0x5fb3b8`. No indirect pointer to `0x550db0` or
`0x550ae0` exists in the file.

Verdict for story 11: **classified, not a per-frame loop**. The inner record
loop runs to exhaustion inside one boot-time call. No paint, judgement,
window, note or DirectDraw code is in this function. The "per-frame
background-effect loop" label in `docs/lifecycle.md:162`,
`docs/parity/playing.md:7`, `docs/parity/playing-input-timing-plan.md` and
`docs/captures/journey-audit/293-playing.md:50` is a contradiction; no
cadence or judgement adjacency can be attributed to this VA. No live
divergence claim follows.

### `0x67fe8c` (file `0x27fe8c`) — NoteTail option float-pair table

Shape: 8-byte stride, two 32-bit floats per slot, read as
`[index*8 + 0x67fe8c]` and `[index*8 + 0x67fe90]`. First four pairs are
`(0,0)`, `(16,14)`, `(30,25)`, `(54,50)`. Bytes at `0x67feac` are the
`NoteTail` option index (default 2); `0x67feb0` begins a separate 7-lane
integer table, so the fifth "slot" is not a normal style pair. This corrects
the repo's "five-slot double table" wording (`docs/options.md:53-54`,
`docs/claims/30-gameplay.md` B45).

Only three immediate references exist in `.text`:

| Consumer | Behavior | Callers |
|---|---|---|
| `0x56b8f0` (file `0x16b8f0`) | constructor of the composite display object at Playing game-object `+0xfec`; builds seven 0x0c-byte lane records and three 0xac-byte records, loads the two floats at `0x56b97f`/`0x56b992` into `+0x6c`/`+0x70` | `0x56f701` (Playing setup) |
| `0x56bf40` (file `0x16bf40`) | reloads `NOTE`/`NoteTail` from `option.ini` (strings `0x645234`, `0x645288`, `0x645294`), clamps 0..4 with fallback 2, reloads floats (`0x56bf88`, `0x56bf9a`), calls `0x56bed0` | `0x4e3515`, `0x4e35fc`, `0x4e366f`, `0x4e8ba7`, `0x4e8c8e`, `0x4ef21f`, `0x4f25ff` |
| `0x56bfb0` (file `0x16bfb0`) | setter with explicit index, clamps 0..4, writes `value + '0'` to `option.ini`, stores index at `0x67feac` (`0x56bff0`), reloads floats (`0x56bfff`, `0x56c011`), calls `0x56bed0` | `0x56c04f` in cycle wrapper `0x56c030` (`(index+1) mod 4` at `0x56c03f`), called from click path `0x4e1d2f` |

`0x56bed0` (file `0x16bed0`) combines the pair with object floats at `+0x64`/
`+0x68`, divides by 160.0 (constant at `0x64dc08`), clamps to 0..120 and
stores the result at `+4` (`0x56bedc`–`0x56bf37`; second constant 1.0 at
`0x643fb0`). Downstream in Playing setup: `0x5710d6` calls `0x56bda0`, a
`timeGetTime`-gated step of `+0` toward `+4` with 1000 ms gate / 20 ms
hysteresis; `0x5710e4` calls `0x56c060` (seven lane records); `0x57118c`
calls `0x56c670` once per lane.

Verdict for story 12: table consumers are the option-driven Playing display
object (constructor/reload/setter plus time-gated derived value), not a
per-style hold-art frame switch in the note path. Static half is classified;
the pixel half still needs the qualified staged-art capture (map to #326 +
#245). Senior should decide whether B45's "double table" wording is amended.

## 2 — #358: FIRSTSTEP → Tutorial entry, Planet plate dead (`S-PLAY-B3`)

The Room control chain is pinned end to end:

1. `0x48fd20` (file `0x8fd20`) — Room control dispatcher. Byte selector at
   `0x490560` indexes jump table `0x490524`; code `0x13200032`
   (`IDC_BUTTON_SROOM_FIRSTSTEP`) selects arm `0x4902bb` → `call 0x495110`
   at `0x4902c1`.
2. `0x495110` (file `0x94110`) — shared entry helper. Sets tutorial globals
   `0x7789d8 = 0x22b8` (song id 8888), `0x7789dc = 1`, `0x7789e0 = 1`,
   bytes `0x7789e4 = 2`, `0x7789e7 = 0`, `0x7789e9 = 1`, then `0x7789ec = 1`,
   `0x7789f4 = 1` (`0x495137`–`0x4951b2`). Pushes state id `0xb` and calls
   `ChangeState` `0x478c50` at `0x4951b5`/`0x4951bd`.
3. `0x478c50` (file `0x78c50`) — writes requested id to parent `+0x54`
   (`0x478c71`).
4. `0x477990` (file `0x77990`) — state-install switch. Converts pending id
   with `id − 2` and table `0x4789c4` (`0x478018`–`0x47802b`); index 9
   (state id 11) selects arm `0x4785fb`, allocates `0x91c` bytes
   (`0x47860a`), calls `CStateTutorial` constructor `0x4f07f0` (`0x478633`),
   stores it at parent `+0xf6c` (`0x478669`).
5. A second caller of the helper is Room method `0x490870` (pointer at
   `0x647be4`), dialog branch `0x7d1` → `0x495110` at `0x4908f3`; the dialog
   owner of `0x7d1` stays OPEN.

Planet TUTORIAL/QUEST plates have no Tutorial dispatch:

- Only file references to `0x19201005` / `0x19201006` are `0x48df51`,
  `0x48df71` (Planet construction) and `0x48e29f` (click range check).
- `0x48df50`–`0x48df8b` fetches both controls and writes 0 via setter
  `0x409bf0` (stores argument at control `+0x24`): disabled at construction,
  matching the always-dim live observations in `docs/planet.md:216` and
  `docs/hover.md:90` (used as behavior lead, not as sole proof).
- Planet click handler `0x48e280` (file `0x8e280`) accepts the whole range
  `0x19201002`..`0x19201006` (`0x48e28f`–`0x48e2aa`) and treats the code as a
  five-entry channel-family index: `0x48c810(base 0x19201002, 5)` at
  `0x48e2e3`, index stored at global `0x76fc38` (`0x48e2fb`), sound and
  reconnect (`0x48e30c`–`0x48e379`). It never calls `ChangeState` or the
  Tutorial helper.

Recorded contradiction: `docs/planet.md:201` labels DRUM `0x19201006` as
TUTORIAL while `docs/planet.md:216` labels QUEST as DRUM and TUTORIAL as FREE
`0x19201005`. Static code treats both codes identically; the live
rect↔emblem mapping pinned via `docs/opi-lessons.md` arbitrates which art
each code hosts.

For #358: the FIRSTSTEP chain is pinned; the Planet-plate hypothesis is
statically refuted (dead control). Senior verification required before any
doc or issue change.

## 3 — #360: Loading credits live in the cover JPEG (`S-PLAY-B4`)

- The provenance-backed p347 reference frame (hash `b555123f…` cited in
  `docs/captures/journey-audit/292-loading.md`) shows `Artist by 손담비`,
  `Remix by (주)SNI`, `Pattern Designed by NoteFactory`, `Illustrated by
  Cap.Cloudless` at the lower right of the full-bleed cover.
- Offline chart read reproduces those lines **in the JPEG**: unwrap
  `/home/arnim/scratch/run-full/Music/o2ma757.ojn` with the repo algorithm
  (`crates/o2jam-file/src/ojn.rs:406-433`, period 6 XOR) → decrypted header
  slice ends `+288 = 0x3abc`, `+292 = 0x699c`, `+296 = 0x9c54` (40020);
  `ff d8` SOI at 40020; decoded 800x600 JPEG contains all four credit lines.
  Header strings are title `@108`, artist `@172 = "손담비"`, label
  `@204 = "NoteFactory"`, filename `@236` — matching names, but not separate
  text draws. Old-format Tutorial cover pin from `docs/file-formats/ojn.md`:
  `@288/@292/@296 = 11784`.
- Loading gate `0x4e47a0` (file `0xe47a0`, reused `ev_68d920c2…`) has one
  dynamic caption: the **title** from the `OJNList.dat` record. `0x4e4d63`–
  `0x4e4d72` computes `record = [0x77952c] + index*0x1b4`, reads string at
  record `+0x6c`; `0x5f27f0` copies it to state `+0x404` (`0x4e4d83`);
  `0x48bfd0` creates the caption control at `state+0x51c` (`0x4e4db2`).
  No read of `+0xac` (artist) or `+0xcc` (label) exists in this function or
  anywhere in the Loading state range. Whole-image sweep finds those field
  reads only in other states/dialogs: `0x4483a0`/`0x448ec3`,
  `0x45cb18`/`0x45cb77`, `0x5218ef`/`0x521965`, `0x527467`/`0x527492`.
- Consequence for #360: the missing credits in our Loading frame are a
  consequence of missing matched-start OJN cover staging (tracked by #355),
  not a missing GDI/Arial painter. The Arial 7x14 mapping at
  `0x4e47a0+0x528` remains OPEN per `docs/loading-type.md`; do not close it
  with this static finding.

No live boot, no game-data edits, no code or tracker writes in this leg.
