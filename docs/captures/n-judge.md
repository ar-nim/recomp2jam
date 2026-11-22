# N-JUDGE — tutorial key input vs the judgement scheduler

Static dossier for spec #313 story #516. Behavior descriptions are ours;
no disassembly or decompiled original code is stored. Oracle:
`OTwo.exe` 7.55, 5,029,888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Tools: REA CLI 6.9.0, Ghidra 12.2 (Linux x64), 2026-11-24. Addresses
below are VAs; file offsets are 0x400000 lower.

## Boundaries

The tutorial update `FUN_004f19e0` handles its scene state and chart-time
page deadlines. It calls the setup helper, song-ms reader, page-cue
helpers and the one-second re-cue, but never the Playing advance, the
judge, or the key-ring forward (REA decompile evidence
`ev_e3dc9b5c0f4d71f00168d19014e98b5a101ed4c1daffc870e13c677288d4abca`).
Its notes share the Playing note entities because Init
`FUN_004f0950` binds the same `FUN_0056f6a0`/`FUN_0056ff30` pair; note
entities are not judgement consumers (#863 findings).

The judgement scheduler is reachable only from the Playing update:

| Step | Address | Evidence |
| --- | --- | --- |
| Key/head judge callers | `0x575c58`, `0x575cb4` | `ev_504ed59b93452ea223e6ad4b95ee62ea877cff21af05d07af261eb05b21b0f72` |
| Judge wrappers called only by play advance | `0x575f38`, `0x575f4a` | `ev_d45f066ef6c4fc80baea1c56473bcd29926a41f9e1c341d35a2675176938e01d`, `ev_332fdddcb7b0d70036a6179768268bad4d3f6441deeb24e2ba431dd89db6cea9` |
| Autoplay judge called only by those wrappers | `0x575c3e`, `0x575c9a` | `ev_564747e497254f0db0d42b47a9e522c4a1cf0d2ec34eb97fb5c5db3ff7b7a027` |
| Play advance called only by Playing advance | `0x571109` | `ev_fa4522600866de723102d065e05d630122d88448cbab435352386d432d8cb474` |
| Playing advance called only by Playing update | `0x4de80e` | `ev_0e0e849603fd6eb93337ccbe0008a7456c1403257234710c8930f59e6514116c` |
| Playing update runs it only when play state is 2 | `FUN_004de7e0` | `ev_166fb293795f47bedb9d38e7fcd9e5e589aa7b3bfdac4a7f7729c0e7b4d3548d` |

Pressed keys enter the shared input-state queue through the scancode
poster `FUN_00544b50` → `FUN_005429b0` → `FUN_00545910` (evidence
`ev_0531b53760d001984f700f67a578a4bcfe25b3197393af88814fcc0c91e923d4`,
`ev_a29afd8bef6dda5974edd4f274905d07e7f225e88c7ce6a33f2b712908abccae`,
`ev_daafcff073e990aa074db8414321a272ef2044b47bebeb635b52b6e0d918a620`).
The judge's event-ring pop `FUN_00579c90` has one caller, inside the
judge at `0x576559` (`ev_25b60ab79795a80a0aa05df27a9a3d0781bf0a907ebeceb114d9e2cd49da07d1`);
the ring producer forward `FUN_005714b0` is called only by the event
queue consumer `FUN_004e15f0` (`ev_a22190bd84ba57a3a590ba07b4fd9883a3d9d9e0e48b17f78c34802ca2a5feff`).

## Verdict

A tutorial key press can light its key art from the shared input state.
No tally can change: the only ring consumer is the judge, and the judge
has no caller on the tutorial update path. Off-note presses therefore
leave COOL/BAD/GOOD/MISS at 0, matching the qualified boot-2 key frames
(input `989abc137caec5cb429d98d1b0e19c094032fc1c900a350c423f6ae0e82c088f`).
The chart-timed Rust replay guard lives next to the FIRSTSTEP tutorial
e2e guard in `crates/recomp2jam/src/e2e.rs`.

## OPEN

A live Wine capture that pairs a chart-timed Tutorial press with a HUD
pixel read remains outside this static dossier; boot-2 frames pin only
the off-note case. This note does not claim an on-note Tutorial rating.
