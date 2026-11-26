# Static pins — misc tail (game-reachable backlog)
Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`.
Provider: REA 6.10.1 / Ghidra 12.2 on the bound artifact. Evidence IDs are
`ev ` plus the full ID in four-digit groups; strip the dots and prefix `ev_`
to recover the REA ID. File offsets come from REA
`address_to_file_offset` and were checked against VA minus `0x400000`
for all 48 rows. Behavior wording is ours; no disassembly or decompiled
text is reproduced.
Branch `cb2/pins-misc-tail`; worktree pre-flight
`/home/arnim/Projects/recomp2jam-lane-pins-misc-tail`. Brief base
`a15244e` and observed HEAD `04cf4681453afb65a5205e1881f11d77c1c0b79e`
are commits; the brief base is an ancestor of HEAD.
Scope: the 48 addresses in `slice-misc-tail.txt` from the coverage
triage bucket `game-reachable`, `pinned_in_repo=no`. **Pinned here:
48/48; left: 0.** One row (`0x57d800`) is a player-triggered file
write; the two web-sheet closers are player-visible support; the other
45 rows are container, lookup, threading or JPEG codec support bodies.
A static pin is not a player-visible close: census rows stay OPEN until
a live caller chain from boot is recorded
(`docs/live-path-reachability.md`). "Reached from" names the provider's
direct-caller edge, not a runtime press or frame.
Shared context: the Print Screen dispatch in `0x5a5190` is dossier ev dbeb.88ad.7f31.83f8.9d0c.b173.ce38.b3b2.8bab.55c5.8662.2866.5ba0.cef4.2a9b.d42a. The screenshot writer `0x42d0a0` is dossier ev 5447.ceb6.c70e.2299.892a.85c5.4fda.8243.deae.5ce2.ab33.ef2b.b236.7b40.3eb3.5134; the tutorial constructor `0x4f07f0` is dossier ev 94a3.65b0.d09c.c8e8.36f1.6077.454e.644a.8372.0598.12bd.4ca7.620c.74cd.516c.e42b. The three caller-body batches are ev 1393.8cd0.33e0.0a9f.44e9.8d80.975a.b116.cfad.83b6.d55d.c91e.3352.0f87.8dc9.0d48, ev 36e2.75b6.b1c4.9bb9.2424.69f7.164c.2151.c464.f6ad.e497.8cb4.8512.625d.e20e.baa9, ev 70f0.aea9.070f.54f5.6973.c2c4.0036.049b.8ad6.27c7.4e21.d099.31c0.a472.0625.1f3d.
## Inventory
| VA | File offset | Role |
|---|---|---|
| `0x4f08c0` | `0xf08c0` | support |
| `0x4f3d00` | `0xf3d00` | player-visible |
| `0x4f5020` | `0xf5020` | player-visible |
| `0x4fc4e0` | `0xfc4e0` | support |
| `0x4fc530` | `0xfc530` | support |
| `0x5049e0` | `0x1049e0` | support |
| `0x57d800` | `0x17d800` | player-visible |
| `0x5e84e0` | `0x1e84e0` | support |
| `0x5e85b0` | `0x1e85b0` | support |
| `0x5e85d0` | `0x1e85d0` | support |
| `0x5e8620` | `0x1e8620` | support |
| `0x5e8880` | `0x1e8880` | support |
| `0x5e8900` | `0x1e8900` | support |
| `0x5e8aa0` | `0x1e8aa0` | support |
| `0x5e8b80` | `0x1e8b80` | support |
| `0x5e8bc0` | `0x1e8bc0` | support |
| `0x5e8c00` | `0x1e8c00` | support |
| `0x5e8c20` | `0x1e8c20` | support |
| `0x5e8d20` | `0x1e8d20` | support |
| `0x5e8d80` | `0x1e8d80` | support |
| `0x5e8e20` | `0x1e8e20` | support |
| `0x5e8ea0` | `0x1e8ea0` | support |
| `0x5e9450` | `0x1e9450` | support |
| `0x5e9580` | `0x1e9580` | support |
| `0x5e9730` | `0x1e9730` | support |
| `0x5ea350` | `0x1ea350` | support |
| `0x5ea390` | `0x1ea390` | support |
| `0x5ea3b0` | `0x1ea3b0` | support |
| `0x5ea3d0` | `0x1ea3d0` | support |
| `0x5ea3f0` | `0x1ea3f0` | support |
| `0x5eace0` | `0x1eace0` | support |
| `0x5eada0` | `0x1eada0` | support |
| `0x5eae10` | `0x1eae10` | support |
| `0x5eae20` | `0x1eae20` | support |
| `0x5eae30` | `0x1eae30` | support |
| `0x5eae40` | `0x1eae40` | support |
| `0x5eaf00` | `0x1eaf00` | support |
| `0x5eb090` | `0x1eb090` | support |
| `0x5ebd40` | `0x1ebd40` | support |
| `0x5ec6a0` | `0x1ec6a0` | support |
| `0x5ed2b0` | `0x1ed2b0` | support |
| `0x5ed880` | `0x1ed880` | support |
| `0x5edd10` | `0x1edd10` | support |
| `0x5ede40` | `0x1ede40` | support |
| `0x5ee860` | `0x1ee860` | support |
| `0x5eeff0` | `0x1eeff0` | support |
| `0x5ef0b0` | `0x1ef0b0` | support |
| `0x5ef280` | `0x1ef280` | support |

## Tutorial state and web sheets

### `0x4f08c0` — reset the Tutorial state fields (support)

- **VA / file offset**: `0x4f08c0` / `0xf08c0` (985,280).
- **Behavior**: Clears the 13 page-deadline words at `+0x898` and the page counter at `+0x8cc`, plus five adjacent state words and three short blocks of selection/chat words. Sets no new value and emits nothing itself.
- **Reached from**: The `CStateTutorial` constructor `0x4f07f0`, right after it installs the state vftable; that constructor is called from the tutorial entry helper `0x477990`.
- **Player-visible reach**: support — it supplies the initial lesson-page state that the Tutorial paint and audio updates read.
- **Evidence**: dossier ev b269.9612.6f20.27d7.a12b.4d2a.0231.283c.0105.dc0d.608f.7b7d.5ce4.2e27.7c11.62b6; file-offset mapping ev 1d9d.089d.8bed.46d3.c250.b2e8.a82d.c53a.a60f.b67b.7dc1.e942.88d2.6a82.c2c9.1a09; caller facet ev 5730.f44d.0779.0067.edd2.5477.2069.8152.d726.af6f.5a68.96f2.3696.3869.412a.755c; callee facet ev fd1d.6f98.23c3.786d.fc47.a73c.8a2b.fe74.aa2f.fd33.e373.7505.51a2.ec47.a660.578c; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2; tutorial constructor dossier ev 94a3.65b0.d09c.c8e8.36f1.6077.454e.644a.8372.0598.12bd.4ca7.620c.74cd.516c.e42b.
- **Our client**: no counterpart by VA; the tutorial fields start at `ScreenGame::new` (`crates/o2jam-shell/src/screens/game/mod.rs:1542`, `:1560`) and reset on entry at `:7261`/`:7370`; page advance is `:1724`.

### `0x4f3d00` — close and rebuild the Payment web view

- **VA / file offset**: `0x4f3d00` / `0xf3d00` (998,656).
- **Behavior**: When a session is present and the stored web-window handle is non-null, sends a close message to that window and clears the handle. Then calls `0x4f3b30`, which rebuilds the Payment web URL and mounts the web view again on the staged Payment chrome.
- **Reached from**: The shared window hook `0x5a5190`, on message `0x232`, when the state word from `0x478a00` is 12 and the web handle is set.
- **Player-visible reach**: player-visible — the Payment web image control is dismissed and remounted; web content itself is outside this client.
- **Evidence**: dossier ev 67c4.702e.0830.2902.68df.a4a8.17a4.8d43.d352.34bb.fd6e.37dd.849a.4c5f.4638.fbee; file-offset mapping ev 2cd9.9930.66c5.f9bb.eb36.dae7.936d.93c7.33de.3f20.9c2a.dabd.6193.b412.4951.78fd; caller facet ev e3db.00c0.26cf.7524.3d6a.df20.a562.3306.ab49.d3f6.936d.a4b2.bc41.b901.8d1d.7d72; callee facet ev bb9e.bea5.60e8.1eb1.f02d.45b5.fb18.e4ba.e150.859a.b287.f4c9.93af.cef1.12ab.b784; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2; window-hook dossier ev dbeb.88ad.7f31.83f8.9d0c.b173.ce38.b3b2.8bab.55c5.8662.2866.5ba0.cef4.2a9b.d42a.
- **Our client**: no web-window owner; the staged Payment chrome and busy badge are `ScreenGame::stage_payment_web_busy` (`crates/o2jam-shell/src/screens/game/mod.rs:8197`, `crates/o2jam-shell/src/screens/game/shop.rs:385`) and the page labels are `crates/o2jam-shell/src/screens/side.rs:6391`.

### `0x4f5020` — close and rebuild the Payment2 gift/coupon web view

- **VA / file offset**: `0x4f5020` / `0xf5020` (1,003,552).
- **Behavior**: Uses the same close-and-clear step as `0x4f3d00`, then calls `0x4f4bd0`. That opener reads `USER_INDEX_ID` from `Identifaction.ini` and builds either the `jamcard.asp` or the NOP coupon URL before mounting the web view.
- **Reached from**: The shared window hook `0x5a5190`, on message `0x232`, when the state word is 13 and the web handle is set.
- **Player-visible reach**: player-visible — the gift/coupon web sheet is dismissed and remounted.
- **Evidence**: dossier ev f1f9.55f4.bd48.eefb.9bb6.b75e.2e5f.7f35.6508.9642.0a89.b57e.4f8a.1571.b639.ac95; file-offset mapping ev 66a6.cf56.7ac3.6d06.c994.6570.eaf9.4262.4e47.35d6.658b.e966.69b4.4ed4.4a3a.52f6; caller facet ev 1f7b.0c97.e757.fb7f.b931.d674.2743.2c58.cce5.5771.1a6f.fc76.d0c6.460b.a896.3c1f; callee facet ev ee2f.0791.5144.46b6.f8f0.96c9.23d8.6a78.7684.3c23.c9ed.c1d0.e271.3e4c.c584.8dcd; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2; window-hook dossier ev dbeb.88ad.7f31.83f8.9d0c.b173.ce38.b3b2.8bab.55c5.8662.2866.5ba0.cef4.2a9b.d42a.
- **Our client**: no web-window owner; the gift page label is `GIFT_PAGE` (`crates/o2jam-shell/src/screens/side.rs:6393`) and the staged sheet is `stage_payment_pages` (`crates/o2jam-shell/src/staging.rs:840`).

## String lookup and worker-thread helpers

### `0x4fc4e0` — choose the string buffer and compare it (support)

- **VA / file offset**: `0x4fc4e0` / `0xfc4e0` (1,033,440).
- **Behavior**: Picks either a short inline character buffer or the stored heap pointer, depending on a length field, then passes the chosen text and the caller's text to `0x4fc530`. Returns the comparison result to a lookup caller.
- **Reached from**: Six callers: the FTP song-name lookup `0x4f7a30` (with `0x4fc650`/`0x4fdf90`) and the Playing digit table lookup `0x52e1a0` (with `0x531b10`, `0x531d50`, `0x533ec0`), plus `0x4fc2a0`.
- **Player-visible reach**: support — name matching in song download and digit lookup; no pixels or wire of its own.
- **Evidence**: dossier ev ed68.fc91.53f3.b8b8.7470.b84b.058e.dd2d.9aee.8ae0.46d4.e759.1198.9a11.776c.39da; file-offset mapping ev aef0.491b.5add.5f32.63ea.4312.afcb.fe32.6434.8e0a.77a4.4be2.ca32.476e.75e4.8b9b; caller facet ev 7024.6d36.7bba.5629.18bf.8a1e.4e09.cf95.1c18.b2db.7381.68c4.f6cb.45b7.76b7.750d; callee facet ev b0df.d899.239e.7c72.084c.a37c.eb95.0261.754b.9318.e73e.af97.afe0.800c.1fed.9778; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2; caller-body batch ev 1393.8cd0.33e0.0a9f.44e9.8d80.975a.b116.cfad.83b6.d55d.c91e.3352.0f87.8dc9.0d48.
- **Our client**: no counterpart by VA; song-name construction is `song_file_name` (`crates/o2jam-shell/src/music_download.rs:30`) and digit lookup is handled by staged controls in `crates/o2jam-shell/src/screens/game/mod.rs`.

### `0x4fc530` — bounded byte comparison (support)

- **VA / file offset**: `0x4fc530` / `0xfc530` (1,033,520).
- **Behavior**: Refuses an offset beyond the stored size through the allocator failure helper. Clamps the second length to the bytes left after the offset, takes the smaller of that and the caller's length, compares that prefix, then returns zero for an exact full match, a negative value when the request is longer than the stored text, and a positive value otherwise.
- **Reached from**: Only `0x4fc4e0` (the string-buffer chooser); indirect callers are the FTP and Playing digit lookups above.
- **Player-visible reach**: support — comparison primitive for the two lookup families.
- **Evidence**: dossier ev 3690.8ffa.5cee.6ac8.e510.604a.e3bf.d0ea.38cf.f250.d294.d66b.875c.2e57.2564.2298; file-offset mapping ev e2d4.a4bc.c35b.5e4e.8010.e404.1864.e518.a626.b2d3.9ca0.dd20.5445.92ab.3036.9631; caller facet ev a95b.5e5e.39ae.8c83.14b6.2e6d.4342.b9b3.e616.5d36.3c43.06fb.458d.7ca8.5749.a28f; callee facet ev ef8e.c2de.e516.8af6.c9e5.10b5.f759.1838.ba3f.c7c5.4cbe.1132.b9e5.8eae.79cd.ffe5; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2; caller-body batch ev 36e2.75b6.b1c4.9bb9.2424.69f7.164c.2151.c464.f6ad.e497.8cb4.8512.625d.e20e.baa9.
- **Our client**: no counterpart by VA; Rust `str` equality in the same call sites.

### `0x5049e0` — start a worker thread (support)

- **VA / file offset**: `0x5049e0` / `0x1049e0` (1,067,488).
- **Behavior**: Calls the thread API with the handler at `0x504a20`, stores the returned handle in the caller object at `+4`, and reports success by testing that handle.
- **Reached from**: The polling helper `0x505130` (Room/Waiting/Playing chat and network pumps) and the client boot path `0x5a5710`.
- **Player-visible reach**: support — supplies the background pump; no pixels or wire itself.
- **Evidence**: dossier ev 6a1d.6dae.2f6c.6cfc.408d.ebbc.ffd9.c453.2d17.0628.984f.4635.fe31.781c.e55b.98c9; file-offset mapping ev 823e.0479.1643.f42f.9030.4ab0.17a6.7969.8f13.102c.9e58.361c.08c2.c918.5a6a.8c25; caller facet ev 84fe.054f.cc88.1920.33dd.7abe.654c.ed09.89eb.850d.596b.d633.dade.0d83.f700.42aa; callee facet ev c78b.9a72.0b50.85fd.5f98.7f5a.8733.4871.f73b.fd80.4c7b.cd31.a578.f163.0559.8b0c; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2; caller-body batch ev 1393.8cd0.33e0.0a9f.44e9.8d80.975a.b116.cfad.83b6.d55d.c91e.3352.0f87.8dc9.0d48.
- **Our client**: no Windows handle counterpart; Rust threads are used by `crates/o2jam-net/src/client.rs:206` and the download pump in `crates/o2jam-shell/src/music_download.rs:77`.

## Player screenshot entry

### `0x57d800` — save an 800x600 RGB frame as a JPEG screenshot

- **VA / file offset**: `0x57d800` / `0x17d800` (1,562,624).
- **Behavior**: Creates a JPEG compressor, installs the file destination, requests 800 by 600 pixels at quality 80, writes one scanline per supplied source row, finishes the stream, closes the file and destroys the compressor. The caller supplies the RGB buffer and the output path.
- **Reached from**: The screenshot writer `0x42d0a0`, called by `0x418dc0` when the Print Screen key (`0x2c`) arrives in `0x5a5190` outside states 10 and 11. `0x42d0a0` reads the 800x600 client backbuffer, builds the folder from `SHGetSpecialFolderPathA` (My Pictures) plus a CP949 suffix at `0x64427c` (`오투잼_스크린샷`), and the name `ScreenShot%03d.jpg` at `0x6447d0` from a per-session counter. When the remaining-shot counter at `+0x9c` reaches zero, `0x418dc0` calls `0x478d50` instead of writing a file.
- **Player-visible reach**: player-visible — pressing Print Screen writes a numbered JPEG to the My Pictures subfolder; a depleted counter shows a popup instead.
- **Evidence**: dossier ev efb6.f8db.ebe7.5702.f288.dd84.8af9.3d97.c971.7b21.4f57.0507.d38f.a1bd.529b.6bf2; file-offset mapping ev e558.8072.dd15.182f.7a6b.1b72.04a5.6bf3.3564.8627.8bf7.5444.f875.8cf7.cde4.cbf1; caller facet ev 40f4.1d2c.eadf.0dae.5fe9.8a3e.f13f.8aa8.165e.1174.0755.5317.80ae.c50b.b4e9.6bc1; callee facet ev 8437.f753.4a07.a680.5eb8.6ef4.d0c2.5cf4.9a8b.9830.4223.b562.23fc.635a.ac32.0ef7; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2; screenshot writer dossier ev 5447.ceb6.c70e.2299.892a.85c5.4fda.8243.deae.5ce2.ab33.ef2b.b236.7b40.3eb3.5134.
- **Our client**: no player Print Screen writer and no JPEG encoder; harness-only PNG shots live in `crates/o2jam-render/src/png.rs:4`, `crates/recomp2jam/src/shots.rs:1` and `crates/recomp2jam/src/session_walk.rs:1`.

## JPEG compressor support (all reached only from the screenshot writer)

### `0x5e84e0` — initialize the JPEG compressor object

- **VA / file offset**: `0x5e84e0` / `0x1e84e0` (2,000,096).
- **Behavior**: Installs the compressor vtable and error/memory hooks, requests the provider's 0x3e and 0x168 structure sizes, zeroes the working arrays, then installs the destination-independent setup and defaults. Ends with state word 100.
- **Reached from**: Only the screenshot writer `0x57d800`, before any file write.
- **Player-visible reach**: support — first leg of the JPEG screenshot encoder.
- **Evidence**: dossier ev f099.0b4b.550f.f850.8e07.0997.6c44.62f7.ff1b.3f2b.86e8.bed7.b717.014f.1b4c.0441; file-offset mapping ev 5527.f866.6f52.ec5f.a8c6.32d6.fb48.5757.c3ef.f532.1ee3.58e6.4bfc.c00a.5a82.511d; caller facet ev d764.4882.a5c6.8d43.edfd.c7af.33db.09a5.988e.580a.288e.76eb.56d4.86dd.e3f3.6485; callee facet ev 42c3.c466.3753.51c2.56ad.fb49.eb86.c84a.1ff3.1c73.70bc.7d59.5e4b.66f5.08ef.821a; body batch ev eff5.579f.7fdb.b779.8210.fbaa.80bb.01c7.4e93.8f15.027e.2825.53c7.c3c4.dd58.f1a2.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e85b0` — destroy the JPEG compressor (forwarder)

- **VA / file offset**: `0x5e85b0` / `0x1e85b0` (2,000,304).
- **Behavior**: Calls the shared memory-manager teardown `0x5ea390` with the supplied compressor object. No other work.
- **Reached from**: Only `0x57d800`, after the file is closed.
- **Player-visible reach**: support — releases screenshot encoder memory.
- **Evidence**: dossier ev fb79.c348.b4ca.3538.1b84.5d67.6705.e157.9244.6759.9d81.98a6.375b.d801.1295.e2a7; file-offset mapping ev eba4.f1de.56fe.5fce.e830.3a22.0fbd.ec80.41cb.58c4.0337.4812.a0c4.181a.e220.c79b; caller facet ev b07c.fee8.df47.26c0.c53c.3660.fd59.7d0e.68a2.4597.ebc2.2b2a.c7b2.514f.93f5.28c4; callee facet ev 8474.c443.4d6c.82de.50a2.e3fa.08cb.a19f.982e.247c.8122.ffbb.2db3.d81a.faa4.b7a2; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e85d0` — set the byte flags on four streams and eight tables

- **VA / file offset**: `0x5e85d0` / `0x1e85d0` (2,000,336).
- **Behavior**: Writes one caller-supplied byte at `+0x80` of each of the four stream records and at `+0x111` of the eight paired records, skipping null pointers.
- **Reached from**: Only `0x5e8880`, while starting a JPEG compression.
- **Player-visible reach**: support — stream/table flag setup in the screenshot encoder.
- **Evidence**: dossier ev cd0d.c19f.ea09.08ac.2077.d0a6.387f.5e90.c386.ef5d.96e4.951a.6f59.8c57.0739.d04d; file-offset mapping ev 284f.0aa2.416c.1bfc.37ca.ba9e.f2ff.dd1b.6ff8.66cf.1ce8.fd15.1457.877e.566e.215c; caller facet ev ed98.08a6.522f.9594.1127.730f.9891.830e.c6ff.a4d8.5238.c57b.bfcb.845f.51fc.9c93; callee facet ev eb2d.46c3.74be.4bb7.3ff1.52d2.3882.5041.a68d.b1c5.7619.fff3.3c97.32e4.3a82.b2da; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8620` — finish the JPEG stream

- **VA / file offset**: `0x5e8620` / `0x1e8620` (2,000,416).
- **Behavior**: When the requested row count exceeds the written count, emits the remaining placeholder rows; flushes the entropy coder, writes the closing marker, runs the destination finish hook, releases working rows and calls the compressor teardown.
- **Reached from**: Only `0x57d800`, after all screenshot rows are written.
- **Player-visible reach**: support — closes the JPEG screenshot file.
- **Evidence**: dossier ev 6682.905b.1405.ea2e.ff1a.7c8f.b7e9.4da8.ebc9.e6eb.579e.794d.f869.1699.0e29.16ae; file-offset mapping ev 62fb.7b3a.c2d4.9f99.19b8.522e.555a.604e.8621.fe6a.25e3.5e87.9f4d.65a3.c329.9c63; caller facet ev fa9d.d374.e428.3310.ec32.732e.be85.3d47.56eb.b624.83bd.4b8b.db95.ca41.b7ac.5c44; callee facet ev 68a3.4708.6e74.0e10.8071.20c6.f345.0729.3d57.4429.b08e.0d0a.85d4.37df.1229.d0c5; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8880` — start JPEG compression

- **VA / file offset**: `0x5e8880` / `0x1e8880` (2,001,024).
- **Behavior**: Checks the compressor state, clears the stream flags through `0x5e85d0`, emits the start-of-image marker, calls the color-transform and destination hooks, and installs the quant-table state for the next stage.
- **Reached from**: Only `0x57d800`, after the file is opened and the compressor configured.
- **Player-visible reach**: support — opens the JPEG screenshot stream.
- **Evidence**: dossier ev d0ac.cf1a.4014.38a7.2309.2631.814d.2835.cb7f.8893.ff6b.b1de.ceee.d530.7722.c0f3; file-offset mapping ev c410.6911.5f9a.3651.dba2.bab3.457d.a505.53b9.2736.beed.1ca6.e328.d88a.973d.11fb; caller facet ev c1b1.ae7d.6e32.f3fb.4bd0.011b.b915.d45e.1a43.a535.c2e0.0e18.58b2.62ec.9bec.1ac2; callee facet ev 7b07.f520.1431.8d0c.8e00.8ae9.40aa.630b.142c.e516.f9a8.664c.da59.cf76.cdd8.6dce; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8900` — write one JPEG scanline

- **VA / file offset**: `0x5e8900` / `0x1e8900` (2,001,152).
- **Behavior**: Requires the compressor to be in the writing state, installs the row range for the destination manager, advances it if needed, asks the destination hook for up to the requested byte count, writes through its buffer, and advances the row counter by the bytes actually accepted.
- **Reached from**: Only `0x57d800`, once per source row (600 rows per screenshot).
- **Player-visible reach**: support — carries each screenshot row into the JPEG stream.
- **Evidence**: dossier ev 9ea6.efc3.f67d.fe94.a992.b1a1.80f5.a8f9.427d.41f0.b596.0aea.044e.379f.721d.1dcf; file-offset mapping ev c61e.b3cd.4dc1.e627.77d3.35cb.8092.a5b1.ac26.de26.7476.2250.2a16.9c5c.c520.4acb; caller facet ev b874.2622.460f.1d3d.a980.4b26.2ef7.d80a.00e6.5e66.4515.efe0.ae16.ea9a.77b8.012f; callee facet ev b80c.9057.54bd.a967.fd29.b6c9.a8f6.6687.3b6f.da9e.9c06.7168.dca8.1ad9.8a95.dcf2; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8aa0` — build one scaled quantization table

- **VA / file offset**: `0x5e8aa0` / `0x1e8aa0` (2,001,568).
- **Behavior**: Allocates a 130-byte table on first use for the selected of four table slots. Scales each of 64 source values by the caller's factor, clamps to at least one and at most 32767 (or 255 when the narrow flag is set), stores them as 16-bit values and terminates with a zero byte.
- **Reached from**: Only `0x5e8b80`, for each of the two default tables.
- **Player-visible reach**: support — quantization setup for the screenshot JPEG.
- **Evidence**: dossier ev 71bc.2ded.d5fe.0f41.2b3c.0c9f.dd5b.c126.7afd.6960.1e8c.3a0f.6720.aab3.1cc6.2772; file-offset mapping ev 18b8.560d.f7bf.f244.b1a8.3aea.cf4b.40af.6164.6f00.e4cc.c306.0425.c564.2c21.13be; caller facet ev ed73.c7bf.8429.b367.a29a.d4ef.225b.149f.ba70.3d1c.11e2.62ac.446e.d3bf.9480.6a22; callee facet ev f807.6633.71d6.aa9e.d6c9.9ac3.9239.3cb2.4f40.47e4.7785.151a.00dc.9e4d.8618.1b0e; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8b80` — build both default quantization tables

- **VA / file offset**: `0x5e8b80` / `0x1e8b80` (2,001,792).
- **Behavior**: Calls `0x5e8aa0` twice, for table slots zero and one, using the two pinned 64-value arrays and the same scale and width flag.
- **Reached from**: Only `0x5e8c00`.
- **Player-visible reach**: support — quantization setup.
- **Evidence**: dossier ev 3c85.e086.ded6.82e4.fe08.76c8.b1cb.2b90.8285.c732.02ee.593b.4651.17cd.f10a.05f7; file-offset mapping ev cd3f.4252.8ac3.47ae.7d31.d1e2.4379.cb80.2d9d.0d3a.6a32.87b5.35ee.5bf6.aa69.f663; caller facet ev fca8.c12e.19d3.1e8f.8559.cec3.e557.c7c0.32e6.3d20.15c2.0974.155d.3aec.4747.f2d5; callee facet ev c8f5.5cd9.ab0d.d72c.b0b1.51ae.a9a2.fca2.9395.dec7.6c1a.9833.4f33.174c.c87a.c108; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8bc0` — map a scale word to a JPEG quality value

- **VA / file offset**: `0x5e8bc0` / `0x1e8bc0` (2,001,856).
- **Behavior**: Returns 5000 for zero, `(100 - n) * 2` for values at or above 100, `5000 / n` for values below 50, and `(100 - n) * 2` otherwise. Integer division truncates.
- **Reached from**: Only `0x5e8c00`, before the two quantization tables are built.
- **Player-visible reach**: support — turns the screenshot quality word into a table scale.
- **Evidence**: dossier ev e4d1.48b8.4445.9112.1b9a.8d35.0f39.ad26.96fa.c9e1.25e5.7425.1eda.3b69.80ca.78a9; file-offset mapping ev 9540.ace8.b95f.4806.390e.005f.38e1.b3a2.87fa.9278.5424.a57f.7854.6f7b.ace3.f9ff; caller facet ev 61cd.e23a.9778.8716.f64e.96e6.27fc.2a08.127d.bc64.22fc.2b25.e589.266e.b6f3.108f; callee facet ev 75ea.bd80.8eec.7667.44f5.c4a7.269c.5dcb.6e57.91b4.bd34.b4d8.6edd.d8c1.2a78.1c17; body batch ev 605a.daa3.d8eb.dce3.0ea8.ab09.4f05.e625.3bfd.26ea.bec5.de23.a631.5061.3240.7ab7.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8c00` — apply quality to both quantization tables

- **VA / file offset**: `0x5e8c00` / `0x1e8c00` (2,001,920).
- **Behavior**: Computes the scale with `0x5e8bc0` and fills both quantization tables through `0x5e8b80` with the caller's width flag.
- **Reached from**: The screenshot writer `0x57d800` (quality word 80) and the compressor defaults `0x5e8c20` (quality word 75).
- **Player-visible reach**: support — quality selection for the screenshot JPEG.
- **Evidence**: dossier ev d4df.8cb0.da24.f61e.bdba.ac32.6074.722e.af81.53c3.3e90.a72c.4353.cdc3.aea1.49c6; file-offset mapping ev beb5.089d.4bef.7be4.ff16.239c.16fd.fa92.e0f7.a4a0.46b7.b447.d835.d219.c004.bf55; caller facet ev 0e90.29e2.64ce.8f13.3a4c.dbc8.f25d.af01.cdb1.6d84.64b6.ceb6.23ed.a4bc.0fd8.a293; callee facet ev 4258.ed5d.836a.ddd3.94f8.24e5.02ba.40dd.d5a0.13a0.bab1.d9f0.8abc.d3df.3b59.2c41; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8c20` — install JPEG compressor defaults

- **VA / file offset**: `0x5e8c20` / `0x1e8c20` (2,001,952).
- **Behavior**: Requires state 100, allocates a 0x348-byte component table on first use, sets the sample factor to eight, fills the two quantization tables at quality 75, writes a 16-byte pattern and several default flag words, then calls the color-space selector `0x5e8e20`.
- **Reached from**: Only `0x57d800`, right after the compressor is created.
- **Player-visible reach**: support — default JPEG parameters for screenshots.
- **Evidence**: dossier ev 658e.59ce.62a2.c81a.d237.a8a4.02ac.310b.7e97.9103.3fe0.5d3f.ccdc.6221.a777.3862; file-offset mapping ev ce19.bb44.d23e.f5d8.7984.c353.4bd3.7dfb.7204.2e99.aa63.3bd8.92ce.b075.2bd4.18c2; caller facet ev c5be.dbde.9da7.ba4c.f626.06d0.1ede.ac43.5c8a.2c0c.7e7a.f620.dfde.58f1.b9b4.d9b5; callee facet ev a690.50cd.b775.1930.5e72.16a5.0d17.ab78.22de.862f.0232.1396.9aa9.d34c.720c.ef8b; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8d20` — install the four default Huffman tables

- **VA / file offset**: `0x5e8d20` / `0x1e8d20` (2,002,208).
- **Behavior**: Calls `0x5e8d80` four times with the four pinned code-length/value pairs, filling the compressor's DC and AC table slots.
- **Reached from**: Only `0x5e8c20`.
- **Player-visible reach**: support — entropy tables for the screenshot JPEG.
- **Evidence**: dossier ev 1fd7.c117.a09f.ca7d.c7df.4122.dffe.2442.c08b.c805.76cd.9f11.ca34.e5ff.6ed5.ee41; file-offset mapping ev 4ec1.7c9e.52d8.cf02.3375.d43b.8a70.08ae.b5ae.1d0d.0c07.9ef5.fb4c.806d.0f5e.768e; caller facet ev a578.4d46.085e.8ec4.466f.08a7.1802.5a27.a505.2c29.b508.87ef.818e.7b5f.d1ca.6931; callee facet ev ba05.704e.6a2c.b297.a7cd.a755.2681.bcda.5844.7afb.dddb.7e63.9dce.31b8.ad7b.6659; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8d80` — fill one Huffman table record

- **VA / file offset**: `0x5e8d80` / `0x1e8d80` (2,002,304).
- **Behavior**: Allocates a 0x112-byte record on first use, copies 17 code-length counts and a trailing byte from the pinned descriptor, validates that the count sum lies between 1 and 256, copies that many values, and clears the table's used flag.
- **Reached from**: Only `0x5e8d20`, for each of the four default tables.
- **Player-visible reach**: support — entropy table setup.
- **Evidence**: dossier ev f0af.9c46.5ba7.9a1e.f820.fc12.4aee.2934.9876.cd49.d708.ad4c.9845.c7bb.30f6.b603; file-offset mapping ev 2b67.d8b6.d70e.616b.82ea.3782.4c2c.c58b.9786.c924.a43c.14b7.52f1.a01c.4123.dba7; caller facet ev b26e.8f0e.d25f.9abe.274e.4267.387a.f021.2d01.5060.5eef.e4ff.8ab5.bff8.1a19.35e8; callee facet ev 73ae.dafe.a521.ab03.e389.deb8.8c4e.64f7.2872.0447.56df.37eb.acf7.1c91.b2ab.6352; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8e20` — route the requested JPEG color space

- **VA / file offset**: `0x5e8e20` / `0x1e8e20` (2,002,464).
- **Behavior**: Switches on the color-space word: passes 0, 1, 2/3, 4 or 5 to `0x5e8ea0`; an out-of-range word raises the bad-parameter error path.
- **Reached from**: Only `0x5e8c20`.
- **Player-visible reach**: support — chooses RGB, YCbCr or CMYK component layout.
- **Evidence**: dossier ev b3b6.d093.6d07.32d7.e676.3b2e.9938.4d77.f179.6e57.16c3.b386.2e84.94fd.7a3c.45ca; file-offset mapping ev df3a.08e8.5500.184b.bb3a.c250.9317.62ef.08dd.0e43.1211.c6a8.56c6.a3ef.14f1.aa80; caller facet ev 0c2e.7d94.6f41.3e76.2566.4e2c.f7be.3ae0.25fe.a1c3.a2c3.d14c.a262.aaf7.9029.bfb0; callee facet ev 912c.e4ac.aa69.fe69.a1c6.21fd.8d8d.10c0.987e.5e2b.ff4c.3ec3.5f29.22fe.d41a.18ff; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e8ea0` — configure JPEG components for a color space

- **VA / file offset**: `0x5e8ea0` / `0x1e8ea0` (2,002,592).
- **Behavior**: Requires state 100 and checks the table slot index. Allocates the component array on first use, then for each mode writes component count, component identifiers (ASCII R/G/B, C/M/Y/K or numeric YCbCr IDs), sampling factors and quantization table indexes into successive component records.
- **Reached from**: Only `0x5e8e20`.
- **Player-visible reach**: support — component layout for the screenshot JPEG.
- **Evidence**: dossier ev 7fe9.4936.1bd5.d84a.6efe.8dfc.eb98.319f.ee82.0a21.11df.8e02.fd59.623d.6874.3158; file-offset mapping ev ceb4.ce9a.2924.7ec3.2d50.a033.1722.7baf.6b87.11e2.fa91.83c6.9d72.178e.36fd.f9f2; caller facet ev e6c5.cd6d.c198.48e5.c3a3.6383.36ce.05db.85cc.0b83.bfa8.da97.8f8f.c511.bd62.996d; callee facet ev e50b.34c2.b875.48ae.43e1.0697.8cbf.3054.6dba.6597.70ed.d72c.dd15.4c0f.ca67.ff89; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e9450` — attach a file destination to the compressor

- **VA / file offset**: `0x5e9450` / `0x1e9450` (2,004,048).
- **Behavior**: Allocates the destination record on first use, installs the open, write and close callbacks plus the caller's file handle, and stores it on the compressor.
- **Reached from**: Only `0x57d800`, after opening the JPEG file with `wb`.
- **Player-visible reach**: support — connects the compressor to the screenshot file.
- **Evidence**: dossier ev b449.1510.f3a8.59a6.1b5d.2ef1.8a0d.f46e.5626.580a.8b1a.b3c4.88d5.828f.384e.d95c; file-offset mapping ev 6be6.2a8b.9852.d3ce.5d00.cda9.2737.8bb3.7477.d9ba.4496.8ec2.fb56.cbcd.a6c5.a0e2; caller facet ev 678e.1c15.26e2.cc95.0a47.b45e.2ed4.7e84.ac0c.c89c.90aa.433d.30f7.ea50.c9a6.05cb; callee facet ev 6549.573d.2edb.a8c8.8190.7c80.294b.c061.2e35.7a03.4beb.f955.a449.59b0.c9e6.7913; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e9580` — initialize the JPEG error manager

- **VA / file offset**: `0x5e9580` / `0x1e9580` (2,004,352).
- **Behavior**: Installs the error-exit routine, message table, message-code pointer and trace fields on the supplied error record, and clears the message parameters.
- **Reached from**: Only `0x57d800`, before the compressor object is created.
- **Player-visible reach**: support — error handling for the screenshot encoder.
- **Evidence**: dossier ev f1bf.37bd.1ebd.ad95.a696.9a74.c741.be6e.ba6e.fd93.793e.d273.1ea0.79ab.99db.10a9; file-offset mapping ev 1ce7.66f7.806a.98e6.e3f4.74ac.26dc.46da.a53e.f0ae.30f0.5c07.66cc.a4f2.7a13.d97e; caller facet ev b2e9.0ed7.9bad.b74d.15ca.556f.c9ea.7c95.ac81.fc3e.d9cc.bc64.28e4.e6db.bbdf.7655; callee facet ev 77a3.4c36.642f.b3f5.6f0d.2eff.0649.e151.d549.573c.9c61.dc73.d77a.ac80.bcbf.6ea0; body batch ev 7acd.4789.981f.eb7d.e972.68cc.15b9.c740.793e.5c99.9015.c59f.da0b.5513.bc78.110a.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5e9730` — initialize the JPEG memory manager

- **VA / file offset**: `0x5e9730` / `0x1e9730` (2,004,784).
- **Behavior**: Installs the allocation and free callbacks, allocates a 0x54-byte error record through the new manager, installs the standard error routines and a default memory limit, then reads `JPEGMEM` and converts a suffixed `m`/`M` value to bytes or a plain value to thousands of bytes.
- **Reached from**: Only `0x5e84e0`, when the compressor is initialized.
- **Player-visible reach**: support — allocation and error hooks for the screenshot encoder.
- **Evidence**: dossier ev 2765.1438.ce55.2c8d.7c44.21ae.8ede.d5f1.1876.94cf.a2ff.c51e.9520.632a.ec77.d9fe; file-offset mapping ev 463d.9bc2.2b99.2574.b9d6.4544.030f.5b01.daaa.42f9.1de4.fc8e.8c27.4d62.a527.f76f; caller facet ev d259.78e7.2dcf.8c4f.62a5.92db.6bd9.4644.312f.a5a6.852e.6c73.e5cd.f362.9ac8.33b4; callee facet ev fff2.d107.eb45.929b.6ff6.f8c6.5ea6.4f03.6f04.8900.92d5.250e.8461.a418.30af.8b08; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ea350` — flush and release the JPEG destination

- **VA / file offset**: `0x5ea350` / `0x1ea350` (2,007,888).
- **Behavior**: Calls the destination's close hook when present, sets the state to 100 (or 200 if a component flag is set), and clears the object's controller word.
- **Reached from**: Only `0x5e8620`, at the end of compression.
- **Player-visible reach**: support — closes the JPEG destination.
- **Evidence**: dossier ev 99df.20b6.6e51.5f91.da89.4c20.1566.5a88.3b76.d69f.101e.0cc2.b1dc.930d.fe69.a944; file-offset mapping ev ab1c.1fb6.0f4c.d687.0e9d.a89f.6fdc.ec54.a136.b073.a177.4145.fc6c.74b6.4abe.c9a1; caller facet ev 674c.c4f0.e414.a3f9.356e.fa77.5c18.9f9d.02e6.6b59.f5eb.ac4c.9df9.a959.2870.5280; callee facet ev ce1a.05bd.d1d4.4375.e006.8e41.296c.7f75.a53b.7e4b.794c.c794.54d5.24fa.feec.f42d; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ea390` — release all JPEG memory blocks

- **VA / file offset**: `0x5ea390` / `0x1ea390` (2,007,952).
- **Behavior**: Calls the manager's free hook for every stored block, clears the manager words and resets the compressor state word to zero.
- **Reached from**: The compressor teardown `0x5e85b0` and the error-exit path of `0x5e95d0`.
- **Player-visible reach**: support — frees screenshot encoder memory.
- **Evidence**: dossier ev 90cf.9623.2257.f3d1.a9f8.c021.f608.15d8.8dbb.2756.ebd0.5865.d049.56df.86a2.84d7; file-offset mapping ev f7f7.4dad.01b2.23c3.0b00.551b.6188.2b68.ed0b.e0cd.f9da.bc5d.e858.e23b.b66d.66d7; caller facet ev 0aec.e1c6.aa49.4be9.5d49.1c44.b07a.ba35.d638.da16.4e15.bd14.88e9.78ad.b6ea.a931; callee facet ev ea6e.9b3b.df53.7c1f.0052.0ca7.6d67.1c84.4fd1.d680.060e.a1b2.b02b.c816.e45a.fbb7; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ea3b0` — allocate a 130-byte quantization table (support)

- **VA / file offset**: `0x5ea3b0` / `0x1ea3b0` (2,007,984).
- **Behavior**: Asks the memory manager for 0x82 bytes and clears the trailing byte.
- **Reached from**: Only `0x5e8aa0`, the first time a table slot is used.
- **Player-visible reach**: support.
- **Evidence**: dossier ev 2013.ae35.846c.e489.f09f.276b.d2ac.a3bd.3e6c.676b.e7bd.dfa4.8f5e.9e73.80b0.3675; file-offset mapping ev 863a.581d.05fb.2ae7.1988.6d33.48c0.301e.3d73.9020.7e4c.bef3.f844.83be.6b34.d65c; caller facet ev 1d65.f862.8c72.939c.8479.d5d5.9843.68c0.8617.06d0.7acc.b4b9.4a4b.a6a1.5869.dbcb; callee facet ev 1bb2.2108.ea9e.0b17.a261.2486.b004.2485.5092.43bb.9523.8fe3.0d8e.53f8.a464.8ad3; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ea3d0` — allocate a 274-byte Huffman table (support)

- **VA / file offset**: `0x5ea3d0` / `0x1ea3d0` (2,008,016).
- **Behavior**: Asks the memory manager for 0x112 bytes and clears the trailing used flag.
- **Reached from**: Only `0x5e8d80`, the first time a table slot is used.
- **Player-visible reach**: support.
- **Evidence**: dossier ev b60c.a229.a5cd.8394.2895.e442.82d7.5b5d.9d90.01fa.e8f2.9e7b.e906.62e5.6c01.d154; file-offset mapping ev 4963.ceea.6524.0a15.d48f.2b33.2749.5837.c653.b8e0.554a.093d.2d0f.daa6.3948.e718; caller facet ev 331e.4bd4.14cb.592b.aed4.f9d3.8aef.ac02.33f3.3ea2.b538.41ac.7932.2fd9.53b5.5f75; callee facet ev ac66.40d8.97a6.22f3.6f5f.b31d.a47d.08a8.294d.bde0.bb0c.6500.daf6.5022.5b53.66e0; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ea3f0` — allocate the color-conversion controller (support)

- **VA / file offset**: `0x5ea3f0` / `0x1ea3f0` (2,008,048).
- **Behavior**: Asks the memory manager for 0x20 bytes, stores the pointer on the compressor and installs the seven-entry conversion hook table with its first entry null.
- **Reached from**: Only `0x5eace0`, during compression setup.
- **Player-visible reach**: support.
- **Evidence**: dossier ev 12ed.2cd0.807d.12d4.e1eb.db41.a172.a12b.d650.8923.fb90.5de4.f1c0.c7d1.65ad.3890; file-offset mapping ev 1500.f13f.5fba.673e.ef83.1c04.7b25.d458.8a58.9b2e.1632.efca.50ee.341b.6059.1ab1; caller facet ev 320e.329c.9042.c378.05cd.b76c.99e6.d57b.a678.231d.68d1.d868.7d8f.2913.ec09.0e1f; callee facet ev fa2c.7672.05e1.8780.17cd.61ef.d7bf.7f10.10b6.182d.97e0.ebcd.a263.e56e.68a7.e531; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eace0` — run JPEG compression setup phases

- **VA / file offset**: `0x5eace0` / `0x1eace0` (2,010,336).
- **Behavior**: Calls the dimension controller, installs the color-conversion path by flag, selects either the two-pass or one-pass color quantization record, chooses the downsampler, installs buffers and coefficient rows, picks the color converter and entropy variant, then releases the quantization tables and finishes the setup chain.
- **Reached from**: Only `0x5e8880`, after the JPEG start marker is written.
- **Player-visible reach**: support — orchestrates screenshot encoder setup.
- **Evidence**: dossier ev 6bfd.d2c4.46e5.f700.b3dc.36e5.524b.4429.51fe.ca99.41a4.ab25.6b90.31ff.6c9d.414f; file-offset mapping ev 80c3.bb60.08a4.f675.8764.eb35.0d6a.f411.839e.60cf.d3b7.0126.08c9.ee3e.e262.0851; caller facet ev 6e7a.0412.7f1c.e8b8.e95b.2a9c.44bf.e9c2.b130.68a0.a8d6.18b9.c18e.378d.a891.b0c1; callee facet ev 7c95.b531.acc8.01da.8a37.efd0.82c5.3f28.28ee.2f16.b238.78f4.e855.7716.d43f.213e; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eada0` — allocate through the JPEG memory manager (support)

- **VA / file offset**: `0x5eada0` / `0x1eada0` (2,010,528).
- **Behavior**: Forwards the byte count to the manager's allocation callback and returns its pointer.
- **Reached from**: `0x5e9730` and `0x5e9870` (error-record creation).
- **Player-visible reach**: support.
- **Evidence**: dossier ev f6a5.f01a.441a.16d3.4ceb.cea5.8fe6.2900.d8b1.8d3e.2615.24d1.32ed.1853.71ea.469e; file-offset mapping ev 3f30.107d.97d2.9277.fb2e.b9ca.b668.ad90.9f28.5d4b.ea48.7d60.619f.bf2f.69e4.d917; caller facet ev cbca.908e.4bd2.1391.b6fb.571c.5f69.a4ec.c492.f209.f574.d720.769e.a6d5.8105.23e3; callee facet ev f0f1.0a9a.e401.69d6.585b.9889.d180.2576.03ec.036f.c414.caf8.de39.0ab0.6de9.a26a; body batch ev 865d.fa42.c511.7a41.44ab.6ced.f43d.c6f4.8f95.7943.3a36.60a6.ab41.9343.de5f.d480.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eae10` — return zero (support)

- **VA / file offset**: `0x5eae10` / `0x1eae10` (2,010,640).
- **Behavior**: Returns the constant zero; no side effect.
- **Reached from**: Only `0x5e9730`, as a hook installed on the error record.
- **Player-visible reach**: support.
- **Evidence**: dossier ev 4e3a.a371.26a2.3db6.8473.289e.0b79.6bb7.ccd8.15d9.bbc7.19b6.06d2.5a62.5563.d2aa; file-offset mapping ev daf4.52a8.9926.565a.5519.eadf.110b.ba02.18cc.3f0f.ba6f.ef8e.7d9e.cf00.0f71.bd65; caller facet ev 83ff.ea0e.bd5c.d91a.a521.7315.d437.d146.b400.a4d5.90b9.6cbc.aaf7.22c0.07fd.a10c; callee facet ev 63e2.010d.3590.e7d7.227e.3389.f4a0.5055.4a88.1591.9d7c.0287.15ec.9f34.edd8.c9f1; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eae20` — empty callback (support)

- **VA / file offset**: `0x5eae20` / `0x1eae20` (2,010,656).
- **Behavior**: Returns without work.
- **Reached from**: Only `0x5e9730`, as a hook installed on the error record.
- **Player-visible reach**: support.
- **Evidence**: dossier ev 6c07.7092.c7bb.6886.c635.cb20.3a68.43fc.f891.080e.39b1.4da5.20b0.dda7.eebe.91f4; file-offset mapping ev f40e.d3e4.16c2.4349.2737.d491.1f41.f2a7.d145.232c.3c22.902f.4265.5a90.0452.32e2; caller facet ev 9526.749a.bc24.afa3.80f9.30ea.246d.7933.fa63.62d6.87b1.aee7.9a48.cf9d.c5fa.d71a; callee facet ev 58b9.dba6.c320.3ff0.b4cd.de06.8dc2.c421.67b4.f8dc.d618.3a47.3041.9ced.9070.e44f; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eae30` — round up by a divisor (support)

- **VA / file offset**: `0x5eae30` / `0x1eae30` (2,010,672).
- **Behavior**: Returns `(a - 1 + b) / b`, with integer division, for the sampling-dimension calculations.
- **Reached from**: The JPEG dimension controller `0x5ef0b0` and `0x5ef8d0`.
- **Player-visible reach**: support.
- **Evidence**: dossier ev db33.3398.ac23.eccb.fe7e.e3b0.e088.af7e.c15e.8c4f.dc01.f92e.2e0f.c1ab.0f9b.2289; file-offset mapping ev fb1b.e381.e308.8607.a8fc.dd79.e17e.6597.db19.443a.a517.e7ef.9b50.50ce.0565.8a9e; caller facet ev 9b84.ca2f.e355.4d67.f1a3.e20a.1603.6079.22d4.aa4a.ad97.5a55.4352.54f0.933e.97ea; callee facet ev 308c.9c03.4fd7.3a67.eeba.acf3.bd75.7b08.2cbb.6c60.befc.a665.45b8.4102.c231.5989; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eae40` — round down to a multiple (support)

- **VA / file offset**: `0x5eae40` / `0x1eae40` (2,010,688).
- **Behavior**: Computes `a - 1 + b`, subtracts its remainder modulo `b`, and returns the result.
- **Reached from**: Only `0x5eb090`, when allocating downsampler rows.
- **Player-visible reach**: support.
- **Evidence**: dossier ev f4fc.13ad.656e.cbbf.a3af.deda.a622.73cd.2fea.ddb5.93ea.575e.9b63.ec5e.1200.1d7f; file-offset mapping ev 1a36.6cff.ad65.a7d5.e964.36be.65a9.aec1.99a4.9e97.e9db.ba30.0d80.5be2.f558.657e; caller facet ev 98a2.18bf.4a97.1da4.0a27.d73a.796e.d44d.c4c8.64cd.80e5.e057.b205.a04c.3819.6812; callee facet ev 874e.e835.00e0.161f.15a4.3237.bbc0.2a02.eec8.1b74.0a0a.eb33.36b5.c8a5.0af9.00e8; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eaf00` — allocate per-component quantization rows

- **VA / file offset**: `0x5eaf00` / `0x1eaf00` (2,010,880).
- **Behavior**: Requires state 100. Allocates a 0x40-byte hook record, then for each component allocates a row buffer of eight times its width and stores the pointer in the matching slot.
- **Reached from**: Only `0x5eace0`.
- **Player-visible reach**: support — per-component buffers for the screenshot JPEG.
- **Evidence**: dossier ev da08.6b18.bbbe.4ed8.0cb8.16e5.5dd0.5cf1.ec1b.ee05.ad9b.c948.e237.71e7.f3ae.4cc1; file-offset mapping ev bcd4.07f8.63ef.f1fd.e263.84fa.90a7.fc7d.caff.c8e3.7bec.6439.1244.c3f7.d855.4b88; caller facet ev c610.7961.40d2.7ab8.87db.847d.0603.0612.be85.517f.5d2f.8b97.dc59.b0cd.34d1.a8f4; callee facet ev 609f.2042.d2f1.e230.bdc8.3c7a.6502.b9ed.ceb7.7202.5456.b35c.d712.9481.83ef.dc0e; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eb090` — allocate downsampler rows

- **VA / file offset**: `0x5eb090` / `0x1eb090` (2,011,280).
- **Behavior**: Allocates a 0x68-byte hook record. In the plain case allocates one 0x500-byte scratch block and ten pointers into it; otherwise allocates one row buffer per component using the rounded-down dimensions helper `0x5eae40`.
- **Reached from**: Only `0x5eace0`.
- **Player-visible reach**: support — downsampling buffers.
- **Evidence**: dossier ev c076.cdb4.7d95.23ef.8ebb.fdd8.90d3.e747.d4c4.46d7.c64d.3c66.651d.8ddf.16b2.a417; file-offset mapping ev 07f1.7964.51ff.3ef4.4781.6f76.221d.390b.e4b2.11a7.e476.558e.12cb.9012.e6d6.fb7e; caller facet ev 2f9f.4c28.b313.6d8e.16ca.4d13.5f76.727c.f562.3735.ed65.631a.3efa.5476.ba41.2840; callee facet ev 2824.5e6e.0499.f1d4.5fc1.f24a.8d43.ca17.fb1d.5d83.d26a.0642.d8a0.bbe7.bc4f.7951; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ebd40` — allocate the two-pass color quantization record

- **VA / file offset**: `0x5ebd40` / `0x1ebd40` (2,014,528).
- **Behavior**: Allocates a 0x6c-byte record, installs its hook table and clears four four-entry groups of counters.
- **Reached from**: Only `0x5eace0`, when the one-pass flag is clear.
- **Player-visible reach**: support — quantization variant selection.
- **Evidence**: dossier ev ff08.4cc1.be4c.db5b.c623.d406.e446.d904.eb9d.9df3.3136.c660.d553.cafe.de13.f238; file-offset mapping ev 5847.e9d6.ffdd.081d.dfef.856d.5a40.1423.362a.facc.f637.0c48.6d79.a9cb.4dc7.5955; caller facet ev 4b47.2f89.2a99.f0d4.652e.3843.65f0.ec94.61a1.9598.59ea.8e3c.5bc8.7584.f10a.0a4d; callee facet ev 8c23.cc34.8910.4882.e003.9f93.bc87.03a5.4b14.9ea1.8c3d.59ab.2511.8478.1e97.887f; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ec6a0` — allocate the one-pass color quantization record

- **VA / file offset**: `0x5ec6a0` / `0x1ec6a0` (2,016,928).
- **Behavior**: Allocates the same 0x6c-byte record with the alternate hook table, clears four four-entry groups and one trailing word.
- **Reached from**: Only `0x5eace0`, when the one-pass flag is set.
- **Player-visible reach**: support — quantization variant selection.
- **Evidence**: dossier ev d7fd.31d8.c1ea.530d.0daf.d57d.c912.711e.e145.e895.199c.51d1.3c8d.ea26.bdc6.197a; file-offset mapping ev 2f19.4fbb.c944.fefd.1f13.9ecc.9594.781c.f7a8.c531.a8b3.b831.24ae.4890.6b99.d046; caller facet ev 8cce.9686.75b8.8183.9ca0.9c2b.4612.2f22.2bbe.6f99.4766.f165.b30c.cb21.c040.1c90; callee facet ev e8dc.9da9.80aa.5136.3171.8948.c227.73dd.6fce.4993.81fb.c9a4.f8a0.4621.6aac.db39; body batch ev 7bae.a6dc.7ca7.38fd.7224.1a29.b602.7319.2982.90a2.4c3c.6e0b.1414.5b45.4e7c.11c5.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ed2b0` — choose the JPEG color converter

- **VA / file offset**: `0x5ed2b0` / `0x1ed2b0` (2,020,016).
- **Behavior**: Allocates a 0x30-byte converter record and installs one of three hook pairs based on the converter word; an out-of-range word raises the bad-parameter error path.
- **Reached from**: Only `0x5eace0`.
- **Player-visible reach**: support — converter selection for screenshots.
- **Evidence**: dossier ev 769f.9954.c5cc.cbc6.ec39.4287.a287.6170.9919.a690.5507.2462.0684.d684.ef69.3300; file-offset mapping ev c4d4.9fc1.fbe7.b90f.da8a.e214.b40d.c682.8fd0.6614.7c9f.e9b6.d3b6.fc97.e527.fd83; caller facet ev a40f.2a83.a730.f315.f0f3.4d01.2ca8.8253.ffcd.66b2.f9a8.f80d.4832.0a8c.f428.ad97; callee facet ev c59d.d066.cd5c.b102.60a5.3448.5c44.0edc.1f43.861c.914a.b535.d1d6.de77.3262.637d; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ed880` — allocate coefficient rows

- **VA / file offset**: `0x5ed880` / `0x1ed880` (2,021,504).
- **Behavior**: Requires state 100 and rejects the alternate flag. Allocates a 0x40-byte hook record, then either allocates one row per component with width derived from its sampling factors, or calls `0x5edd10` to build the interleaved row table.
- **Reached from**: Only `0x5eace0`.
- **Player-visible reach**: support — coefficient buffers.
- **Evidence**: dossier ev fc5d.0291.2932.6d83.b024.2a06.1e00.2f24.964d.64c9.a4e2.2770.691c.ced6.21a4.a2ed; file-offset mapping ev ad11.9839.bd64.c097.4217.0f6b.c62c.ccfd.a9e0.87df.8086.7a9a.9000.4fa0.7ac1.ac06; caller facet ev e63b.fc5d.daeb.7648.deb4.82d8.35e1.1893.b707.0929.a880.ff3c.b1d0.a962.4dbd.f17d; callee facet ev 9ddf.389a.b806.e490.4697.48f4.d891.260c.88e3.bc16.98fc.9955.752e.472c.a7d9.b904; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5edd10` — build interleaved coefficient row pointers

- **VA / file offset**: `0x5edd10` / `0x1edd10` (2,022,672).
- **Behavior**: Allocates one block sized for the component count, sample width and row count, then fills a pointer table: for each component it copies one scratch row, swaps the first two words, and stores the resulting row start into that component's table slot.
- **Reached from**: Only `0x5ed880`, in the interleaved branch.
- **Player-visible reach**: support — interleaved coefficient rows.
- **Evidence**: dossier ev a117.51d5.710e.dd42.8e9a.340a.c8cb.8caf.9d02.152e.c4b4.dc98.bb0e.170c.e9b7.7e7b; file-offset mapping ev 3c15.92c1.7322.0337.db30.3101.f91d.94eb.f586.1b4d.1095.be0e.a24a.5419.73e7.1a36; caller facet ev 8425.a491.0fef.013c.953d.76ef.ef3b.6eb0.f0be.9258.0dc2.7fac.622c.1b92.55dd.0b62; callee facet ev 4afd.7e3a.1f30.882a.d41e.3851.5e27.1463.3098.93e4.c92e.7d02.0a38.b8e2.ef4e.43b5; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ede40` — choose a downsampler per component

- **VA / file offset**: `0x5ede40` / `0x1ede40` (2,022,976).
- **Behavior**: Allocates a 0x34-byte hook record, then for each component compares its horizontal and vertical sampling factors with the image maxima. Selects an exact, half-only or general routine accordingly; unsupported ratios raise the bad-parameter error path.
- **Reached from**: Only `0x5eace0`.
- **Player-visible reach**: support — downsampler selection.
- **Evidence**: dossier ev d093.6bb1.ed30.c18a.4a0b.1b00.e91e.59e2.065b.3249.befd.2384.7cb0.4490.feda.0ba2; file-offset mapping ev f671.8f12.ff4c.e540.7299.d36e.317b.0d3c.bc91.4792.a98e.c8a6.6cde.06d1.5c1b.14e7; caller facet ev c757.7d68.f2bc.ff51.f250.b4c4.9d54.a1ca.617a.1fe4.2e84.bcbf.a79e.74dd.cdf0.9ca7; callee facet ev 700d.b810.a22c.000c.dd89.5ca2.29bd.eaf5.42a5.bfb4.0390.ea4a.5512.0acc.b688.48b3; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ee860` — choose a color conversion routine

- **VA / file offset**: `0x5ee860` / `0x1ee860` (2,025,568).
- **Behavior**: Allocates a 0xc-byte hook record, validates component counts against the color-space word, then selects the conversion routine for the requested input/output space pair; a mismatch raises the bad-parameter error path.
- **Reached from**: Only `0x5eace0`.
- **Player-visible reach**: support — conversion routine selection.
- **Evidence**: dossier ev e194.5c94.b955.1937.038a.a121.5dd3.7282.8510.6f36.56f9.7986.735c.f9b4.6851.5a7c; file-offset mapping ev 9bb8.9b54.5684.8c78.838d.9014.5964.2b10.18c3.706a.62f0.c102.346a.c4b7.a071.d350; caller facet ev 61f2.0098.a163.5b35.9546.afa3.aea2.051d.b8e9.70d1.0d07.c37a.9940.7dfb.8129.20ac; callee facet ev 2c6f.cfc9.6a35.a71f.c83b.154e.fd6f.5061.742e.18b5.bc1b.8159.a53d.9910.e4fb.416c; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5eeff0` — install the JPEG compression controller

- **VA / file offset**: `0x5eeff0` / `0x1eeff0` (2,027,504).
- **Behavior**: Allocates a 0x20-byte controller with three hooks, calls the dimension controller `0x5ef0b0`, applies defaults when no quant table is present or calls `0x5ef280`, and sets the output width multiplier from the numeric flag.
- **Reached from**: Only `0x5eace0`.
- **Player-visible reach**: support — output sizing for screenshots.
- **Evidence**: dossier ev 18d8.68b7.9dc5.82b7.3a4d.93f9.cbd7.c733.988a.eedd.9f51.982d.7965.e571.43d2.1947; file-offset mapping ev 69ac.cc00.7052.91f7.4608.f990.2972.5d89.a255.fdef.bc13.bac9.cf4a.57e3.3920.de70; caller facet ev 9df8.af30.df7c.a45e.9236.f938.fc40.1b1d.a14d.1a1a.ef60.9b13.7160.2a10.0fde.6847; callee facet ev 4724.3c13.15e5.edda.f16c.db0b.8f91.7613.61ef.fb6a.e2e3.c51f.1b4c.a2ec.3f7f.2da7; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ef0b0` — validate dimensions and compute JPEG MCU size

- **VA / file offset**: `0x5ef0b0` / `0x1ef0b0` (2,027,696).
- **Behavior**: Rejects zero or oversized width/height, a non-eight data mode, component counts outside one to ten, and component sampling factors outside one to four. Tracks the maximum sampling factors, computes each component's downsampled dimensions and sets the number of rows in an MCU.
- **Reached from**: Only `0x5eeff0`.
- **Player-visible reach**: support — validates the 800x600 screenshot size before compression.
- **Evidence**: dossier ev 28d8.a40a.2b23.2743.bdf1.d6f6.4509.9fb1.613e.2a7f.5362.1574.59b2.75e2.d2eb.1a60; file-offset mapping ev 9607.4fa6.91f9.199b.d86c.a902.eaad.fd47.d10d.625f.0f82.5b5c.3619.58cd.f7e4.5fbc; caller facet ev 6112.ea75.f1c3.6dfe.7bbd.e193.255a.d086.a09f.ca58.ed12.5d9b.8dca.1fcb.2cdf.286a; callee facet ev a84b.e210.9c56.cf6e.75b0.e877.2f42.796a.1a7a.a63c.d8cc.112c.b219.85f8.b84a.31fb; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

### `0x5ef280` — validate quant-table assignments

- **VA / file offset**: `0x5ef280` / `0x1ef280` (2,028,160).
- **Behavior**: Checks each table descriptor's index range, rejects duplicate and out-of-order entries, and marks every component's referenced quant table as used; raises a bad-parameter error on any violation.
- **Reached from**: Only `0x5eeff0`, when a custom quant table exists.
- **Player-visible reach**: support — quant-table validation.
- **Evidence**: dossier ev 0779.4577.1a70.d4ca.1281.6028.e1ec.a45e.52f9.5e2a.c198.83fe.2846.4678.8634.94ce; file-offset mapping ev 0dfe.0549.3fcb.d2ac.6ce7.be12.08f5.bad4.e190.cf00.4ce1.bb50.a646.a600.6510.ee4f; caller facet ev f5fb.6822.6172.4a9d.576d.b08e.013e.25a3.f62f.0a57.7039.03d1.6872.b3a7.72b0.2287; callee facet ev 848e.84b8.ab1d.78b9.8242.ae36.cc33.295b.175f.7a34.1bd2.97f1.fc8b.7991.6855.5c89; body batch ev 3e51.780a.1aa8.3d2f.5d28.6613.80d0.7dd0.04f5.8868.2c48.5824.fe4e.e6d0.b10f.3e27.
- **Our client**: no counterpart by VA; the client has no embedded JPEG compressor (tooling PNG encoder: `crates/o2jam-render/src/png.rs:4`).

## Census rows touched by this pin set (rows stay OPEN)
- Tutorial entry and lesson clock: `docs/audio-cue-inventory.md` and
  `docs/captures/868-tutorial-clock.md`; no live caller chain from boot is
  claimed by this static lane.
- Payment and Payment2 web sheets: `docs/parity/payment.md`,
  `docs/interaction-census.md` Payment rows; the web close/remount loop
  needs a qualified live frame before a row can close.
- Screenshot file output: no census row yet; the Print Screen path
  needs a live key-leg guard that checks the written JPEG name.
- FTP song-name and Playing-digit lookup families: support only; no
  player-visible close is implied by these pins.

The spec-shaped copy of the player-visible subset is in
`/home/arnim/scratch/lanes-2026-11-25/spec-enrich-pins-misc-tail.md`;
the query log and OPENs are in
`/home/arnim/scratch/lanes-2026-11-25/findings-pins-misc-tail.md`.

## Loading-gate font slots have no reader (2026-11-26 senior REA pass)

Constructor `0x4e47a0` (VA; file `0xe47a0`) creates three fonts and
stores them on the Loading object: Trebuchet MS 27×65 w400 at byte
offset `0x524`, Arial 7×14 w400 at `0x528`, and the face at `0x64ae78`'s
neighbour `0x64407c` (Gulimche 6×12) at `0x52c`. The destructor
`0x4e50b0` releases them.

A whole-`.text` masked byte search for every `mov reg,[base+0x524]`,
`+0x528`, and `+0x52c` load form (opcode `8b`/`8a`, all base and index
registers) found no reader in the Loading class range. Every hit is an
unrelated class: `0x45fb60`, `0x466b40`, `0x461500`, `0x4c65c0`,
`0x468650`, `0x4aab60`. The range `0x4dc000`–`0x4f0000` has zero hits.

Consequence: no runtime text is drawn through the three gate fonts. The
Loading credits and caption are baked JPEG ink only. `docs/gdi-text.md`
OPEN 5 is closed statically; no font-consumer leg is owed.

A sibling constructor `0x4eb8a0` (file `0xeb8a0`) makes the same three
fonts into the same three slots for the six-step Loading class and
also draws no text through them; its release path `0x4ebe50` frees
them. The whole-`.text` search above covers this class range as well.
The font-maker's direct callers (`0x41c860`, release `0x41cb20`) name
no other Loading-range owner.

Evidence: `ev_b9463ff71c69fc77404ea181ba54478f6075c312c16aeec05590ce2d38026947`
(sibling constructor `0x4eb8a0` and release `0x4ebe50`),
`ev_8bfe450075a5a6fa5aa48610383c1829e98bbb495d4a7e89a51a3c6cd97a37e8`
(constructor pseudocode), `ev_2b565920968953407045d808c6bb8c9c3ddc93ef527015c5bce200bf48be8a6d`
(constructor assembly), `ev_55249cee006ad35f6677f3e538145019cc8de945112603db9dfbd9ae56438ff7`
(destructor `0x4e50b0` and neighbours), `ev_491d9aa65e12fce028c33105abfa0fd2e57aec077fa792948f738d35c6a61509`
(vtable slot `0x64adec`).
