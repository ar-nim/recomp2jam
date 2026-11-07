# Twin stage appliers (BGA) — 7.55

Version pin: `AngelO2Jam.exe` 5,029,888 B, SHA-256 `83a04541…72c823b`.
File offset = VA − `0x400000`. Method: read-only byte walks with
Python 3.14 over the installed binary plus read-only objdump static analysis;
**no REA dossier IDs issued** (same precedent as `docs/stage-bga.md`).
No disassembly reproduced. Evidence ids below are file offsets, per
`docs/reversing.md` §3. Main-path behavior lives in `docs/stage-bga.md`.

## Answer

The `_` stage resolver exists in triplicate — one per Playing-family scene —
and all three share the room's stage-numbered art. Beyond the main applier
`FUN_004e7850` (file `0xe7850`, solo playing): `FUN_004ee600` (file `0xee600`)
serves Live (network) playing, and `FUN_004f29c0` (file `0xf29c0`) serves
Tutorial.

## Callers

There are no direct callers. No direct relative call (`E8`/`E9`) targets any
of the three appliers anywhere in 7.55 (whole-file scan; the room picker,
room setter, and slot loader `0x48b2d0` all show ordinary direct callers, so
the scan is sound). Each applier address occurs exactly once in the whole
binary — in its scene's parallel `.rdata` table (files
`0x24ae50`/`0x24b410`/`0x24b510`: same slot, shared base-class head
`0x489f40`/`0x489f50`/`0x489f60` + `0x4dbdd0`). The caller is therefore the
shared indirect scene dispatch through that slot, not a per-scene direct
caller. Upstream loaders per scene:

- Solo: art loader at file `0xdc750`, naming `Playing_NormalBG` at file
  `0xdc82e`; network state names surface inside file `0xe5d90` (naming
  `State_Playing_network` at file `0xe5dcf`).
- Live: sibling loader at file `0xed650`, naming the TimeBar art at file
  `0xed913`.
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

Scene proof is the `.rdata` neighborhood. The `0xee600` family sits beside
`Playing_ChatWindow.oji`, `Playing_Exit.ojs`, `Normal_TimeBar.ojs`, so it
serves `CStateLivePlaying`. The `0xf29c0` family sits beside `Tutorial.ojm`,
so it serves `CStateTutorial`.

## OPEN

1. The exact indirect dispatch routine behind the shared applier slot: the
   three applier addresses occur only in their scene tables (files
   `0x24ae50`/`0x24b410`/`0x24b510`) with no direct call site, so the
   function that reads the slot and calls it stays unidentified.
2. Twin head divergence: the Live twin keeps the 10-case per-entry selector
   pattern of the main path, while the Tutorial twin's head checks differ
   (zero-checks at file `0xf2ae2`/`0xf2aeb`, `$0x9` compare at file
   `0xf2af4`) — cosmetic or scene-driven stays untraced.
