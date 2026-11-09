# E1: Channel dialog-open trigger (emblem click → `0x8C` block mount) — static hunt verdict

Version pin (all rows): `OTwo.exe` 5029888 B, sha256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Method: offset-only LE32 scans of the full image plus REA function
dossiers via the Ghidra provider (12.1.2). Offsets are file offsets
unless marked VA (VA = file + `0x400000`). No original bytes,
disassembly, or decompiled code are reproduced; behavior is
described in own words with file offsets and REA evidence IDs.

## Verdict

The emblem-click handler **does not mount the `0x8C` dialog block and
does not write the dialog phase flag**. In static code the emblem path
is a server-channel-family select plus reconnect. The dialog block is
staged by the dialog class's own stage virtual through the dialog
stager, and the phase flag has only a constructor clear and a timed
toggle in the parent pump. The emblem → dialog-visibility edge is
therefore **dynamic-only** and stays OPEN with the named winedbg
memory-watch next step below. Nothing here changes the shell; the
shell's Channel ↔ Planet flag flip remains a stand-in for the OPEN
link.

Two `0x8C` values must not be conflated (this doc uses all three with
qualification):

- **Block code `0x8C`**: the `DIALOG_CHANNEL_INFO` block (26 records
  with quoted sprite names; see `docs/channel-fill.md` live-read
  section). Its table index in the dialog stager is `0x1B` (27).
- **Object offset `+0x8C`**: the planet/dialog object's channel-side
  art slot (holds the select-bar body). The planet promo side rides
  `+0x84`.

## Emblem handler — family select + reconnect (PINNED)

`CStatePlanet` is registered at vftable file `0x247b20` (COL VA
`0x667104`). Its emblem dispatcher (VA `0x48e280`, file `0x8e280`,
REA `ev_da9c910b6e8f386132fdb0aa95ad3774e03eb2ca472f1c5f2149ba08c9e7023f`)
covers exactly the five emblem button codes `0x19201002`..`0x19201006`
(the `State_Planet.bnd` HARD/NORMAL/EASY/FREE/DRUM cells). Behavior
in own words:

- It consults a flag getter (returns the dword at `+0x58` of the `nw`
  status object; VA `0x4f7a10`, file `0xf7a10`, REA
  `ev_b9a4e73069bc3639590313f507266c3fb1b93ae5d6326a06c64c28b154df6713`)
  and, when the flag reads active, posts a parent-queue record
  (first argument 1) instead of switching.
- Otherwise it selects the pressed emblem in its toggle group (VA
  `0x48c810`, file `0x8c810`, REA
  `ev_0f6bbde1049b07aa1b34214bea69b2906fcc3aa8734fcb356f1332713ff48221`;
  loops five looked-up controls from base `0x19201002` and drives the
  pressed one's selected visual), records the family index
  (`pressed − 0x19201002`, 0..4) in the family-index global at file
  `0x36fc38` and clears the companion flag at file `0x36fc50`,
  plays the click sound through the sound dispatcher (VA `0x551870`,
  file `0x151870`, REA
  `ev_23b6185bd599450dc697b4ce3931e296eb4b99bf2340a98f5c387bdef7ed3ee1`,
  called with arguments 1, 0 here), then calls the shared reconnect
  (VA `0x58c6c0`, file `0x18c6c0`, REA
  `ev_bb902a0b1b529ba73434233d8f46163786e94e5dc1c93e8c0bb5c6a114399acd`).

The reconnect takes both code families and maps each to a server
endpoint and port before running the socket connect sequence and
marking the session flags. Planet emblems select endpoint 0/1/2 with
ports `0x998f`/`0x9990`/`0x998f` for indexes 0/1/2 (index 3/4 reuse
the connect leg via the `0x8c201014`/`0x8c201015` arms). The channel
row codes `0x8c201011`..`0x8c201015` select the same three endpoints
and the same two ports, so one function serves both clicks. The
dispatcher then posts queue records on the parent-queue helper
(VA `0x478c90`, file `0x78c90`, REA
`ev_466ef5ecfd409f102b1305407cd683569cc379ee49a055da2596cac1e2f8226d`;
it packs the first argument shifted into the high halfword plus the
low twelve bits of the second, copies an optional string, and
enqueues through the `+0x168` queue helper VA `0x47edf0`, file
`0x7edf0`, REA
`ev_e57d4a2bd0a77ae3a64b901cb47f644796172a4cf25b60c5f9fa9b5df3dcb386`
— stride `0x110`, distinct from the `0x10`-stride dispatcher
`0x478d50`, file `0x78d50`, REA
`ev_76aa12d69431d08ba03601c7e8545cb60e5432746d852ee090bcdabf7e20210e`
via `0x47eaf0`, file `0x7eaf0`, REA
`ev_f0fe25176fa4f7dee8c7f98049c412d0cdf9cd81709a14686124cb9d740efb71`).
No call in this handler reaches the dialog stager, writes
parent `+0xF64`/`+0xF6C`, or fetches dialog art.

The second planet input function (promo carousel / EXIT / OPTION,
VA `0x48e000`, file `0x8e000`, REA
`ev_eacdd66ee9ed1b5989c6b1b96bf09cecdb8a5155ab5e21af5cf9d85d02355235`)
is separate and shares only the flag getter and queue helper.

## Channel-row handler duplicates the same tail (PINNED)

The dialog class (`CDChannelInfo`, vftable records at files
`0x247150`/`0x247160`, COL VAs `0x666fe4`/`0x666f8c`, both naming
TD `.?AVCDChannelInfo@@` at file `0x27dc04`; see correction below)
carries a row handler (VA `0x474630`, file `0x74630`, REA
`ev_d6c125e1143730189ac7869ad57ae36b1b49219e40ab0a5076036517cf63a000`)
covering the same five `0x8c201011`..`0x8c201015` codes with the same
family-index writes, sound call, and reconnect call, plus three
dialog-local finishing steps (clearing the scroll offset dword at
object `+0x184`, clearing the dword at `*(+0x208)+0xb8`, and
refreshing the row view). Its sibling row hit-test (VA `0x474570`,
file `0x74570`, REA
`ev_82e1c376811eaa16d0d2c17bb48405a1ee7848b9d236d8eb226440a5a9906af8`)
tests the pointer against the four row rects at object `+0x188` and
routes hits to the row-join function (VA `0x475260`, file `0x75260`,
REA `ev_0afc8e2f5ca72d132120ea8ca33caf03faf4f207e9f1e20262dcc87d07a3a22c`),
which performs the `0x3ec` join post or the queued error post.
Neither writes parent `+0xF64`/`+0xF6C`.

## Dialog block staging (PINNED, mount mechanics)

The dialog stages its block through its stage virtual (VA `0x474110`,
file `0x74110`, REA
`ev_b0816f54acfd2cbaba8b4523d2a4ff343ec9559749d0afafeb34d369cbc10046`),
which calls the dialog stager (VA `0x432510`, file `0x32510`, REA
`ev_bca137a4972046e42521747aeafbee77c7489c8e91522fd4d0458a5de608f253`)
with index `0x1B`. The parent singleton is held at global VA
`0x762b44` (file `0x362b44`). The stager stores the index at object
`+0x10`, resolves the descriptor at parent `+0x1FE8 + (index−1)×0x118`
(that is parent `+0x3C58` for index `0x1B`), loads the stem, and
creates one control per entry whose code class
`code & 0xFF0000 ≠ 0` via the dialog-side factory (VA `0x4329b0`,
file `0x329b0`, REA
`ev_2cfe3af951b2814a74dffc0e8148b4b3b9120f84a6ba45b9c612feab75f76e3c`).
This is the second-table twin of the state-open stager (VA `0x48a060`,
file `0x8a060`, REA
`ev_704c2baeb7dbafb7950792cf4259a41fd6d2428f866af30d3877a26d6eddd2b3`),
which resolves parent `+0xF80 + (index−1)×0x118`. The constructor
(VA `0x4753a0`, file `0x753a0`, REA
`ev_b3187e43ed425da524bfc625f7abaf4d0d9701c50550c06e618b6a7452f8603f`)
allocates 15 entries at parent `+0xF80` and 27 entries at parent
`+0x1FE8`, so the exact second-table base is `+0x1FE8` (see
correction to `docs/channel-fill.md` below).

A separate dialog function (VA `0x474150`, file `0x74150`, REA
`ev_dfb34c390e7a61e99a518e66dd5c7d9f08d5a4fe63cb8541a7d0d3c105dbeed9`)
copies the four `0x8c01a003`..`0x8c01a006` row-control rects into
the mounted planet object's array at object `+0x94` (the live-pinned
row table) and refreshes captions and row views (via VA `0x475000`,
file `0x75000`, REA
`ev_aa34804a62be6f9a429ac6584713f4dde287bd69a423730d882ce7d4412225ac`).
The planet-side art fetch (planet builder VA `0x48dd10`, file
`0x8dd10`, REA
`ev_0a4f6f5734aed8b37c1b5bf01fdcc3c9438b56ea494914fd9e3d7f46179d7110`)
places the select-bar body into the planet object's `+0x8C` slot
through the sprite-name fetch (VA `0x40ced0`, file `0x0ced0`, REA
`ev_c57849982a93a5b2f8bbacc977e148dcb5335895f14a1f39cffc46c768ee9002`).

## Phase flag `parent+0xF64` — no emblem writer (PINNED negative)

Exhaustive LE32 scan for displacement `0xF64` returns three hits
image-wide (files `0x758b9`, `0x7605c`, `0x76069`): the constructor
clear and the pump toggle pair. No emblem-route writer exists.

- The constructor helper (VA `0x475820`, file `0x75820`, REA
  `ev_bc21f32510ed002fe44e50211df11b265e58cfb8cf45fc8d10294dd744e81a22`,
  called from VA `0x4753a0`) sets parent `+0xF64 = 0` alongside
  `+0xF58 = 0`, `+0xF60 = 0`, `+0xF6C = 0`, `+0xF68 = 0x20`, and
  zeroes the descriptor tables.
- The parent pump (VA `0x476020`, file `0x76020`, REA
  `ev_f458efa5b6e54418588b247c5efcf792f22570551a56ca29b6646b8b2ffe7a56`,
  vftable `0x647920` slot 5; constructor vftable store in
  `0x4753a0`) flips parent `+0xF64` whenever the elapsed helper
  (VA `0x411ea0`, file `0x11ea0`, REA
  `ev_cb69c37b6eb1298745aee05fe0ecebb62b8418ce6360131d75ffdb124ca3244b`;
  compares the tick dword at object `+0x3c` against the stored stamp
  and threshold, resetting the stamp on fire when asked) reports
  elapsed. The call passes the stamp at parent `+0xF58`, threshold
  500, and reset requested, so the flag toggles on a 500-tick cadence
  when the pump runs. The pump runs on the parent's own scheduling;
  relating 500 ticks to the live ~21 s auto-open delay or to the
  observed dialog-stable capture is itself OPEN (the tick unit and
  the pump rate are not pinned statically).

The mounted-child pointer `parent+0xF6C` shares the same negative
shape. In disp32 form its only non-zero-creating stores are the
state-router lazy allocations inside VA `0x477990` (file `0x77990`,
REA `ev_5cde070e7f8732586648a40b57ef34d00187921b22f493a44d67752b4fec6922`),
which dispatches on parent `+0x50` (subtract 2, jump table at file
`0x789c4`) and, when `+0xF6C` is zero, allocates and constructs the
state object then stores the pointer. The planet arm allocates
`0xD8` bytes and constructs via VA `0x48db90` (file `0x8db90`, REA
`ev_6e1c05cbb2bfbb44a114c5c80fcd3b709a5c2bec13cfb1df68700b3eb5544974`,
installs the planet vftable). The emblem handlers never write parent
`+0x50`, `+0xF6C`, or `+0xF64`. The constructor and destructor clear
`+0xF6C` (files `0x758c9`, `0x76013`).

## Corrections to pinned state

- `docs/channel-fill.md` says `CDChannelInfo` never materialized as a
  class and survives only as one RTTI name (file `0x27dc10`).
  Correction: the TD name at file `0x27dc04` is referenced by two
  vftable records (files `0x247150`/`0x247160`), so the class exists
  in code with the method cluster above. What stays OPEN is which
  allocator creates a `CDChannelInfo` instance at runtime (the
  state-router never allocates one) and which virtual call runs its
  stage virtual.
- `docs/channel-fill.md` names the second descriptor table at
  parent `+0x2000`. Correction: the exact base is parent `+0x1FE8`
  (constructor loops and dialog stager agree); `+0x2000` is correct
  only as the table region. The `0x1B` dialog descriptor sits at
  parent `+0x3C58`.
- The earlier "dialog lookup is index-driven" note is confirmed and
  sharpened: the index is the constant `0x1B` inside the stage
  virtual, not a lookup by block code `0x8C`.

## OPEN (with named next step)

1. **The emblem → visibility edge is dynamic-only.** Static code shows
   emblem input ends at family select + reconnect; dialog visibility
   (`parent+0xF64`) moves only via the 500-tick pump toggle. The
   live "emblem click reopens the dialog" observation therefore rides
   a path invisible to direct-call and disp32 scans (virtual dispatch
   through the `+0xF74` list, cached control pointers, or array
   iteration — the known scan-blind shapes). **Named next step:
   stopped-state memory watch on `parent+0xF6C` (and `parent+0xF64`)
   during an emblem click, launched on `OTwo.exe` directly, never
   explorer; queue all reads (`x` family) and take one stop only per
   the rig rule.** Read parent `+0x50`, `+0xF58`, `+0xF64`, `+0xF6C`,
   the dialog object's `+0x84`/`+0x8C`, and the stager counters
   immediately before and after the click to distinguish a click
   write from the timed toggle.
2. Which virtual call runs the dialog stage virtual (VA `0x474110`)
   and with what this pointer; and which allocator creates the dialog
   instance.
3. How a button press reaches the planet dispatcher (VA `0x48e280`):
   no direct callers are pinned; the input fan-out from the staged
   control array to the vftable slot is OPEN.
4. The pump toggle cadence versus the ~21 s auto-open and the X-close
   behavior (which writer sets `+0xF64` to 0 on X-close, and whether
   the 500 threshold is milliseconds).
5. Which named dialog carries parent dialog id 5 / `0xa` from the
   emote-gate getters (VA `0x478a00`, file `0x78a00`, REA
   `ev_4eb0af4c96600f6c3a86610e464887f001a1935d5cec023e0c57ddca7c5f50de`;
   VA `0x478a60`, file `0x78a60`, REA
   `ev_a9a26a3341b3007552008a6105d3b4b677718c176526231bcfdedab41f9f54ae`)
   — including the fetch-callback check that the front dialog id high
   halfword equals `0x1B` (VA `0x48db40`, file `0x8db40`, REA
   `ev_c4fdf7f70342a899c4d61766a8e8f24afc524d26e5288663997c3bcb51b1857a`)
   before row-hover work (VA `0x48e530`, file `0x8e530`, REA
   `ev_b17a2be179c628f83047cef24441fbb5faac5be5d829a6b78ad4efc338b74c11`).

## Shell note

No code changes. Shell stand-ins named in the affected docs stay
open for the same reasons: the dialog-visibility edge (OPEN 1),
button fan-out (OPEN 3), and dialog-instance creation (OPEN 2) are
not grounded by this hunt.
