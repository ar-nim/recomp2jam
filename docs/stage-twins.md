# Twin stage appliers (BGA) — 7.55

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`.
File offset = VA − `0x400000`. Method: read-only byte walks with
Python 3.14 over the installed binary plus read-only objdump static analysis;
**no REA dossier IDs issued** (same precedent as `docs/stage-bga.md`).
No disassembly reproduced. Evidence ids below are file offsets, per
`docs/reversing.md` §3. Main-path behavior lives in `docs/stage-bga.md`.

## Answer

The `_` stage resolver exists in triplicate — one per Playing-family scene —
and all three share the room's stage-numbered art. `FUN_004e7850` (file
`0xe7850`) serves Live (network) playing — the arm `docs/stage-bga.md`
traces. `FUN_004ee600` (file `0xee600`) serves Normal (solo) playing.
`FUN_004f29c0` (file `0xf29c0`) serves Tutorial.

The shared dispatch is a C++ virtual call through one slot. Each applier sits
at slot offset `+0x74` (index 29) in its scene's primary state vtable:
Live base file `0x24addc` (COL file `0x26758c`), Normal base file `0x24b39c`
(COL file `0x267774`), Tutorial base file `0x24b49c` (COL file `0x2677e4`).
Slot math is identical: `0x24ae50 - 0x24addc = 0x74`,
`0x24b410 - 0x24b39c = 0x74`, `0x24b510 - 0x24b49c = 0x74`. The caller loads
the current state object and calls `[eax+0x74]`. Which arm runs depends only
on which state object is current: Normal on solo entry, Live on network
entry, Tutorial on tutorial entry.

## Callers

There are no direct callers. No direct relative call (`E8`/`E9`) targets any
of the three appliers anywhere in 7.55 (whole-file scan; the room picker,
room setter, and slot loader `0x48b2d0` all show ordinary direct callers, so
the scan is sound). Each applier address occurs exactly once in the whole
binary — at its slot (`0x24ae50`/`0x24b410`/`0x24b510`). The vtables are
installed by constructors that write the primary base plus the `+0x3c`
secondary base: Live at files `0xe46a8`/`0xe46b1` and `0xe473a`/`0xe4743`
(`0x64addc` + `0x64adc8`, COLs `0x66758c`/`0x6675e8`); Normal at files
`0xeb648`/`0xeb651` and `0xeb83a`/`0xeb843` (`0x64b39c` + `0x64b38c`, COLs
`0x667774`/`0x6677d0`); Tutorial at files `0xf0828`/`0xf0831` and
`0xf089a`/`0xf08a3` (`0x64b49c` + `0x64b488`, COLs `0x6677e4`/`0x667840`).
Upstream loaders per scene:

- Solo (Normal): art loader at file `0xdc750`, naming `Playing_NormalBG` at
  file `0xdc82e`; sibling loader at file `0xed650`, naming the TimeBar art
  at file `0xed913`.
- Live (network): method at file `0xe5d90`, naming `State_Playing_network`
  at file `0xe5dcf` and `Playing_network_exit` (string file `0x24b064`, ref
  file `0xe5e01`).
- Tutorial: art loader at file `0xdd290` (owning `Playing_TutorialBG` at
  file `0xdd2f1` and `KeyEffect_Tutorial_` at file `0xdd3e6`), sibling
  loader at file `0xf1140` (naming `Tutorial.ojm` at file `0xf1274`), plus
  `Tutorial.ojm` loaders at files `0xf0950`/`0xf0d30` (naming it at files
  `0xf0a49`/`0xf1034`).

## Why these twins (offsets)

All three read the same stage globals (`0x7789dc` at files
`0xe78db`/`0xee68b`/`0xf2a4b`; `0x7789e0` at files
`0xe7927`/`0xee6d7`/`0xf2a97`), gate on the same archive-open flag
(`0x76816c`), walk the same 41-entry list (`$0x29` bound at files
`0xe7958`/`0xee708`/`0xf2ac9`), test the `_` prefix the same way (`$0x5f`
compare at files `0xe8031`/`0xee7e3`/`0xf2b60`), and format the same names
(`Note_Click` `%s%d_%d.ojt` site at files `0xe814e`/`0xee8f5`/`0xf2c73`;
`Playing_Effect_Jam` site at files `0xe822a`/`0xee9dc`/`0xf2d5a`).

Scene proof is RTTI plus strings. The `0x4e7850` vtable carries the Live COL
(file `0x26758c`, type file `0x27e1d4` = `CStateLivePlaying`) and its method
at file `0xe5d90` names the network state and exit. The `0x4ee600` vtable
carries the Normal COL (file `0x267774`, type file `0x27e29c` =
`CStateNormalPlaying`) and its loader at file `0xed650` names
`Normal_TimeBar`. The `0x4f29c0` vtable carries the Tutorial COL (file
`0x2677e4`, type file `0x27e2f8` = `CStateTutorial`) and sits beside
`Tutorial.ojm` (string file `0x24b524`).

## OPEN

1. CLOSED — dispatch is the shared primary-vtable slot `+0x74` (index 29).
   Evidence: identical slot math in all three primaries (files
   `0x24ae50`/`0x24b410`/`0x24b510`), COL-linked vtables (files
   `0x26758c`/`0x267774`/`0x2677e4`), constructor installs (files `0xe46a8`/
   `0xeb648`/`0xf0828` plus secondaries), zero direct callers whole-file.
   Arm selection is object identity (current `CStateNormalPlaying` vs
   `CStateLivePlaying` vs `CStateTutorial`), not a branch in the caller.
2. CLOSED — the Tutorial head divergence is code shape, not behavior. Stage rule is identical: same `0x7789dc` / `0x7789e0` reads (files `0xf2a4b` / `0xf2a97`), same high-word strip, same variant seed (files `0xf2a40`–`0xf2aa8` match Live files `0xe78d0`–`0xe7938` up to relocations and frame offsets). Same 41-entry walk (`$0x29` bound file `0xf2ac9`), same `$0x5f` `_` test (file `0xf2b60`), same kit formats (format `0x64b154` pushed at files `0xe8143` / `0xee8f5` / `0xf2c73`; Jam arms at files `0xe8210` / `0xee9c0` / `0xf2d40` share one shape). Only the per-index fan-out is narrower: Live spreads indices 0..20 over a 10-arm table (file `0xe85ac`, 21-byte selector file `0xe85d4`, bound `$0x14` file `0xe7973`, overflow to generic file `0xe7fde`); Normal spreads 0..10 over 4 arms (file `0xeec60`, 11-byte selector file `0xeec70`, bound `$0x0a` file `0xee723`); Tutorial chains three range checks on the loop index (zero-checks files `0xf2ae2` / `0xf2aeb`, `$0x9` compare file `0xf2af4`): index 0 takes the verbatim-load arm (file `0xf2b01`, same shape as Live arm0 file `0xe79c2`); 1..9 skip back to the loop increment (file `0xf2aff` jumps to file `0xf2aba`, same as Live arm2 file `0xe79c0`); 10 and above fall into the generic `_` path (file `0xf2b0e`, same shape as Live generic file `0xe7fde`). The middle check (file `0xf2aeb` branches on unsigned `<= 0` right after file `0xf2ae2` already took `== 0`) is unreachable — compiler shape. Tutorial drops Live's index 10..20 specials because its control list carries its own art (`Playing_TutorialBG` file `0xdd2f1`, `KeyEffect_Tutorial_` file `0xdd3e6`); every stage-kit name still resolves through the identical generic path, so no distinct stage number or kit name results.
