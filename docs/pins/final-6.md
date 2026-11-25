# Static pins — final six behavior-relevant functions

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`.
Provider: REA 6.10.1 / Ghidra 12.2 on the bound artifact. Evidence IDs are
`ev ` plus the full ID in four-digit groups; strip the dots and prefix `ev_`
to recover the REA ID. File offsets come from REA
`address_to_file_offset` and equal VA minus the PE image base `0x400000`
for all six rows. Behavior wording is ours; no disassembly or decompiled
text is reproduced.

Branch `cb2/pins-final6`; worktree pre-flight
`/home/arnim/Projects/recomp2jam-lane-pins-final6`. Brief base commit
`6cb6efc` (`6cb6efcf5d73166c1d651bf5f59022d4cc159367`) is the observed HEAD
and is a commit.

Scope: the six addresses in the brief for the last unpinned
behavior-relevant procedures. Owner ruling excludes the container bucket.
**Pinned here: 6/6; left: 0.** Five bodies are support; one
(`0x520dd0`) is player-visible class identity. A static pin is not a
player-visible close: census rows stay OPEN until a live caller chain from
boot is recorded (`docs/live-path-reachability.md`). "Reached from" names
the provider's direct-caller edge, not a runtime press or frame.

Shared context: the packed-file name lookup `0x40a9c0` reads `0x98`-byte
records (`PackingFile.cpp`) and calls the seek wrapper at `0x406ca0`; its
body is dossier ev 0775.411c.c9c7.61de.ddb0.2c8f.50a5.a22b.facd.81dc.111b.d57e.cbf8.ba57.2e8c.d5c4
and body batch ev 05a7.206f.ea26.457f.ac4a.525f.0959.bc49.60a3.7c5b.f29f.fe72.edb4.5979.408b.487a.
The control factory and host/slot loaders `0x4329b0`, `0x48a620` and
`0x48b2d0` build the sibling button classes documented in
`docs/pins/shops-myroom.md` (`0x5238f0` `CCBtnMShopRList`, `0x51ea80`
`CCBtnGiftList`); their body batch is ev
197e.8f15.133e.8edd.0c6f.67e7.9c3d.fed3.0a8b.d887.323e.bf6e.59d4.eefd.0fd5.0f3e.
Vftable symbols were resolved by REA `address_name`: `CDXSurface::vftable`
at `0x644554`, ev
1c81.e95e.277c.f788.26b5.0993.abd5.4b5f.4c5e.a306.adaa.a91c.05c3.8255.10c5.7841; and
`CCBtnMShopLList::vftable` at `0x64c494`, ev
0d39.12e0.42a8.902a.8476.7dd6.81ea.27fd.104d.b35d.ba20.8747.e83d.4538.414a.7380.

## Inventory

| VA | File offset | Role |
|---|---|---|
| `0x406ca0` | `0x6ca0` | support — packed-file seek |
| `0x4227a0` | `0x227a0` | support — surface construction |
| `0x4337c0` | `0x337c0` | support — child-control lookup |
| `0x478fd0` | `0x78fd0` | support — 16-byte record count |
| `0x4797d0` | `0x797d0` | support — gated archive lookup |
| `0x520dd0` | `0x120dd0` | player-visible — Music Shop list button class |

## Packed-file support

### `0x406ca0` — seek a packed-file handle (support)

- **VA / file offset**: `0x406ca0` / `0x6ca0` (27,808).
- **Behavior**: Treats the handle word at `+0x104` as invalid when it is
  `-1` and returns zero in that case. Otherwise calls `SetFilePointer` with
  the caller's signed displacement, a null high word, and the caller's seek
  origin; the returned position passes through unchanged.
- **Reached from**: 10 direct callers: five packed-file wrappers
  (`0x406ce0`, `0x406d10`, `0x40a8a0`, `0x40a9c0`, `0x410f40`), surface and
  loader helpers (`0x420da0`, `0x42a130`, `0x4dc390`), the Playing-side
  reader `0x52cff0`, and the boot loader `0x5a7710`. The `0x40a9c0` arm
  seeks backwards to the `0x98`-byte record table before comparing entry
  names; 21 raw xref sites were observed.
- **Player-visible reach**: support — positions the archive stream used by
  boot and Playing resource loads; emits no pixels or wire itself.
- **Evidence**: dossier ev 10e2.7879.e3b7.bf57.ea82.97da.a37f.d792.6edd.a69d.5eaf.19d7.bb08.e7c4.144e.9c66;
  file-offset mapping ev bfe7.b8b1.e2e5.581f.d4ee.d421.9b20.5129.1d3b.b900.77de.024f.88b4.a0ed.b16a.c951;
  incoming xrefs ev 1a8d.728b.4d8b.5e28.0426.626f.a74e.a7e6.24f7.3bad.59c6.5a1a.536e.fc24.2306.5854;
  caller facet ev c5d4.3e5a.8c84.f826.b6f2.29f2.0cf8.6721.f07f.cff5.29f7.6e20.f6a8.08a0.3dd1.a418;
  callee facet ev 67c8.1f57.e407.c1a0.54b5.9d20.da5d.428a.e620.17a6.e3e2.540a.ec1a.9d13.a678.c313;
  names facet ev 47a1.f641.9bb3.04db.6b24.a432.8348.f4f6.828d.6edf.a89d.ee1a.dffa.5960.2402.0961;
  body batch ev 6ca4.9499.d47b.810d.c323.5ae1.439a.bc3d.d90a.b481.614f.6f94.516a.edde.a840.09d3.
- **Our client**: no counterpart by VA. Archives are read as byte slices by
  `parse_opi_slots` / `parse_opi` (`crates/o2jam-file/src/opi.rs:134`, `:178`);
  the seek-to-record-table strategy is not reimplemented. Synthetic fixtures
  live in `crates/o2jam-file/src/test_archive.rs:25`.

### `0x4797d0` — gate the packed-file name lookup (support)

- **VA / file offset**: `0x4797d0` / `0x797d0` (497,616).
- **Behavior**: When the first argument is below three, forwards the other
  two arguments to the packed-file name lookup `0x40a9c0` and returns its
  result. For any first argument of three or more it returns zero without
  touching the archive.
- **Reached from**: 12 direct callers across file and state helpers
  (`0x402670`, `0x402be0`, `0x420da0`, `0x424190`, `0x4249f0`, `0x42a130`,
  `0x45c560`, `0x4795e0`, `0x521350`, `0x52f9f0`, `0x547db0`, `0x548450`);
  12 raw xref sites. The sole callee is `0x40a9c0`, which seeks and walks the
  `0x98`-byte record table.
- **Player-visible reach**: support — a mode gate on resource lookup; the
  result later selects art or sound, but this body emits nothing itself.
- **Evidence**: dossier ev 2803.f6e4.1f3c.205f.6d9e.b51d.3525.ed18.52d8.562c.542e.3dd8.96e8.7268.529f.1a30;
  file-offset mapping ev 2479.6169.619c.9600.dafb.0548.6dc5.3faa.40b4.5f02.e807.0d71.0d41.9346.ac89.85a2;
  incoming xrefs ev d0fe.34e6.2e3a.af8b.a4c1.be00.ec24.ba84.514c.c1aa.119f.8673.33d8.9fa7.5a62.34fc;
  caller facet ev 16ac.f13a.9858.202c.3271.c2f7.594c.beac.1285.725c.e38b.87f8.939c.fbd8.eb94.2e06;
  callee facet ev 6e56.998e.e3d0.407c.aa7d.46c3.20f9.c2aa.1a6d.5dd2.e16b.21da.20ce.d21a.5836.90f3;
  names facet ev 6f58.5ccb.ec19.117d.7a92.4658.fd5c.30d2.b72d.554a.187e.7719.399b.3d45.543f.384b;
  body batch ev f55f.87fd.2ca0.94cb.d80e.a076.6f5e.864c.2c24.13c9.d8b2.e4fa.c90c.94e4.4128.ef05.
- **Our client**: no counterpart by VA. The Rust parsers in
  `crates/o2jam-file/src/opi.rs` always try the table; no three-way mode gate
  exists. Which caller-selected mode values three and above mean remains
  OPEN (next step: read the mode word at one live resource-load seam).

## Surface and control support

### `0x4227a0` — construct a CDXSurface (support)

- **VA / file offset**: `0x4227a0` / `0x227a0` (141,216).
- **Behavior**: Writes the `CDXSurface` vftable pointer to the object start
  and calls the shared surface base initializer `0x41de50`. No allocation,
  blit or field clear happens in this body.
- **Reached from**: 19 direct callers. The nearest is the allocation factory
  `0x422770`, which invokes this constructor and frees the object when its
  flag bit is set; other direct callers are surface constructors
  (`0x41e4b0`, `0x41e7b0`, `0x52bcb0`, `0x52c0c0`, `0x537770`, `0x537950`,
  `0x56f820`) and unwind fragments. 24 raw xref sites were observed. The
  vftable target at `0x644554` resolves to `CDXSurface::vftable`.
- **Player-visible reach**: support — supplies surface objects to the paint
  path (`docs/ui-draw.md`, `docs/gdi-text.md`); no frame is painted here.
- **Evidence**: dossier ev 8914.a13d.1990.5e04.c6c9.4b3f.3b66.6287.9c65.a0fc.4414.4fb8.f17b.084a.fec7.164f;
  file-offset mapping ev 5a3d.1039.4903.6681.383b.c079.8217.9b8b.6071.ba5c.cc92.a6d7.20d7.579a.1a07.1559;
  incoming xrefs ev 4f62.d510.b7a3.c657.77de.5d3b.bf15.d41f.d878.fef4.62a9.df4d.8d1a.3814.b36c.d71e;
  caller facet ev 6d5d.9d4a.9390.b950.4ed7.8e31.0430.50dc.a781.d8f0.80e2.2930.923d.0563.6928.9728;
  callee facet ev cbf1.9985.f3e1.bc4e.362e.7639.ebe5.6c8c.f454.0639.18f7.6571.6ffe.d176.76fc.fde7;
  names facet ev 32b7.a954.d738.4ce0.250a.5004.b0a2.a109.ef59.aac9.3e61.ad8a.ecf9.54e9.1706.c7f4;
  body batch ev 6b0f.61e4.ad54.124a.8636.64de.bf8a.8ddb.9ae5.2000.64f6.a9e4.a29c.41ea.7b1c.a2d2;
  allocation-factory dossier ev 0013.115f.bfdf.ab07.86e4.fcd0.04f0.81ca.39eb.a745.6d70.fff3.2fd8.761e.b4ff.592a.
- **Our client**: no counterpart by VA and no DirectDraw surface object.
  The software compositor owns `Frame` (`crates/o2jam-render/src/lib.rs:32`)
  and staged rects (`crates/o2jam-shell/src/staging.rs`).

### `0x4337c0` — find a child control by its virtual identifier (support)

- **VA / file offset**: `0x4337c0` / `0x337c0` (210,880).
- **Behavior**: Walks the child pointer array at `+0x4c`, bounded by the count
  word reached through `+0x50`, and asks each non-null child for its
  identifier through virtual slot `+0xc`. It returns the first child whose
  identifier equals the argument, or zero when none matches.
- **Reached from**: 99 direct callers and 452 raw xref sites. The retained
  caller facet page (32 of 99 rows, ev below) starts with control-factory
  helpers `0x433af0`, `0x433b60`, `0x433c90`, `0x434080` and continues
  through the UI control cluster. In `0x433af0`, a non-null answer from this
  body triggers a message through `0x4f72e0` / `0x4f72b0`, so the lookup also
  serves duplicate-id reporting during control setup.
- **Player-visible reach**: support — control identity lookup used by UI
  setup and dispatch; no pixels, wire or hit test of its own.
- **Evidence**: dossier ev f4e8.4627.bd2a.3993.6527.4cc4.7675.6b75.80d5.0aae.f2b5.aaeb.eb2d.d21f.b94d.f661;
  file-offset mapping ev 2f92.ae6c.a2fa.9362.bb77.29c3.e3f3.06e3.b683.12c6.16fb.0b1d.1e9e.d99a.d995.0ce6;
  incoming xrefs ev 244f.b935.156f.ef78.34b6.2c74.9738.8e6d.9e44.ba53.bccf.df00.0edf.98b3.f611.bdcb;
  caller facet, first page ev f00e.6428.1ea6.34f6.9c72.ff78.f90d.b9e8.4177.8b62.150a.3b1a.afde.3e48.e2ad.fafc;
  callee facet ev e871.6c46.4922.e3f9.dcb9.f32b.b0ab.38b8.b163.50fc.5d6f.acf2.1503.e75e.0ac8.2bf3;
  names facet ev ff1c.3865.a936.52e3.509c.60fc.bab6.b4a7.830d.09e6.ce14.290e.d302.11a2.2657.c107;
  body batch ev 78af.5716.d416.786c.a403.5461.9ef0.9aac.9f98.2e5e.a49a.9164.5653.98c7.dc7b.2a75;
  first-page continuation ev 22a2.fe31.18f3.1d6c.1279.ee6e.6fc0.baa2.6238.da13.8913.6c05.c8c3.a9eb.2077.1c7e
- **Our client**: no native child-control object. Placements are matched by
  authored ordinal (`control_ordinal` in `crates/o2jam-shell/src/staging.rs:1088`,
  lookup at `:1096`); whether the virtual identifier equals that ordinal is
  OPEN (next step: read one control-factory identifier against a staged
  placement).

## Tutorial list support

### `0x478fd0` — count sixteen-byte records in a list (support)

- **VA / file offset**: `0x478fd0` / `0x78fd0` (495,568).
- **Behavior**: If the begin word at `+0x40` is null, returns zero. Otherwise
  subtracts begin from end (`+0x44`) and shifts the difference right by four
  to report how many sixteen-byte records the list holds.
- **Reached from**: 3 direct callers (`0x476020`, `0x476330` and the state
  factory `0x477990`); 3 raw xref sites. The factory is the shared entry
  helper that constructs the Tutorial state, so this count feeds tutorial
  setup and related list iteration.
- **Player-visible reach**: support — computes a count for later list walks;
  it never paints a lesson page or emits a cue itself.
- **Evidence**: dossier ev 6581.5726.4d1b.ba00.4938.ba59.1f03.7ad3.b10d.5d86.9824.5d17.bbba.75f2.42bf.19c8;
  file-offset mapping ev 1a74.675b.b5c7.2f3a.6a97.2378.2c92.9b6a.056d.f876.e0b0.47a6.581d.cea5.c920.4289;
  incoming xrefs ev b44a.02b2.031b.02bd.67e4.5b61.fee5.9de9.ef0f.50c3.36b1.f348.bee8.e22b.bb5d.e736;
  caller facet ev ac4f.9fb7.267d.ed36.e3cf.00d4.0a02.bf4a.93d8.b82c.9094.9597.042f.8115.594c.5d92;
  callee facet ev 894a.6d40.f853.f891.9931.9318.20c7.f161.4cf3.b03a.52db.9c84.e7dd.bb87.b591.f7f5;
  names facet ev e7be.3457.41fd.356d.21a4.58fa.6ab8.0b28.6893.a2c2.132f.503e.e622.77c4.3b3d.1fe0;
  body batch ev 48b8.64bc.ae28.e2bf.a689.dd5d.b962.d550.4310.862b.7716.ccee.cc0f.dadd.4f67.e465;
  tutorial factory dossier ev 310a.1525.81f9.96d8.db97.bae3.3cb5.b5c1.2393.7d49.78fb.8479.e670.b9d3.861a.12e2.
- **Our client**: no counterpart by VA. Tutorial page state is initialized in
  `ScreenGame::new` (`crates/o2jam-shell/src/screens/game/mod.rs:1560`), reset
  on entry (`:7261`, `:7370`) and advanced at `:1724`. What the exe's
  sixteen-byte records hold stays OPEN; next step is one static read of the
  list elements at `0x476020`.

## Music Shop button class

### `0x520dd0` — construct a music-shop left-list button (`CCBtnMShopLList`)

- **VA / file offset**: `0x520dd0` / `0x120dd0` (1,183,184).
- **Behavior**: Calls the shared button-base initializer `0x4f6040`, installs
  the `CCBtnMShopLList` vftable, sets up two stack-side list helper records
  through `0x5229e0` with a zero argument, clears the word at `+0x64`, the
  byte at `+0x1d` and the bytes at `+0x75` and `+0x76`, then returns the
  object pointer.
- **Reached from**: 3 direct callers: the shared control factory `0x4329b0`
  and host/slot loaders `0x48a620` and `0x48b2d0`; 3 raw xref sites. These are
  the same callers as the pinned `CCBtnMShopRList` (`0x5238f0`) and
  `CCBtnGiftList` (`0x51ea80`) constructors in `docs/pins/shops-myroom.md`.
  The vftable target at `0x64c494` resolves to `CCBtnMShopLList::vftable`.
- **Player-visible reach**: player-visible class identity — the Music Shop
  left-list buttons are built from this class. The body installs behavior;
  it does not hit-test or paint, so a live click leg is still required.
- **Evidence**: dossier ev c575.d8c5.c659.e752.9798.477e.1474.600f.1aea.177a.e4d8.af8b.0016.c943.769e.e5a5;
  file-offset mapping ev 1d73.149c.1ed7.8867.0337.c903.4be7.5546.f6c7.6af5.01dd.22dd.c8bb.eba5.a09f.cd60;
  incoming xrefs ev c2a3.9519.8457.479c.84bf.16a9.a7ff.ea74.d64e.94dd.259b.1d4f.c1af.1129.1c9d.c1e8;
  caller facet ev 6192.b950.be73.9551.e4d3.b1a7.c441.fc92.d35f.bc8c.449a.2351.a07e.6d7b.6eaa.43e8;
  callee facet ev 8e18.b826.ce55.779e.eeda.7b11.9c36.4bb2.1d02.6d9f.0f70.1669.1ea6.0634.ce81.b7dd;
  names facet ev 82e5.62ca.6b49.2764.fa63.ba53.8c87.006a.ffef.0bfa.1206.a619.6a87.6d67.5728.1289;
  body batch ev 6956.4bd8.c41b.762d.186e.34b2.e027.2ece.2a6a.674e.1a88.1e36.36ad.dca7.32f3.a54c;
  vftable name ev 0d39.12e0.42a8.902a.8476.7dd6.81ea.27fd.104d.b35d.ba20.8747.e83d.4538.414a.7380.
- **Our client**: no native button object. Staged Music Shop controls are
  dispatched by `music_shop_action_at`
  (`crates/o2jam-shell/src/screens/side.rs:3008`); Tier 0 guard
  `music_shop_sort_and_pager_answer_their_staged_buttons`
  (`side.rs:8305`). No guard pins the left-list class defaults, and no live
  Music Shop → left-list click capture exists; both remain OPEN.
