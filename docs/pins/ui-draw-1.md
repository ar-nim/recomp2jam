# Static pins — ui-draw-1 bucket (57 functions, coverage burn-down)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Tools: Ghidra 12.2 / REA 6.10.1, analysis profile digest
`547bdf93bd6e607cbb88c4dbd6133b7dc54ce7ab567c389324768dbdb3e68c2c`.
Evidence IDs below are the parent IDs of REA `analyze_function` dossiers on
that exact artifact, written in dotted four-hex groups so
`scripts/check-cited-hashes.sh` does not read them as commits. File offsets are
REA `address_to_file_offset` results, checked one by one; each equals the VA
minus the PE image base `0x400000`. Behavior text is our own description of
provider output; no disassembly or decompiled text is reproduced.

Scope: the 57 addresses in `slice-ui-draw-1.txt` from the coverage triage,
all `bucket=game-reachable,pinned_in_repo=no`. Worktree branch
`cb2/pins-ui-draw-1`; brief base `a15244e`, observed HEAD
`a42ca1ed3c8d94f66d23f40937c19ab57b17816c`. Pinned here: **57/57**; left: **0**.

A static pin is not a player-visible close. "Reached from" records provider
direct-caller edges plus raw xref sites, not a runtime press. Census rows stay
OPEN until a live caller chain from boot is recorded; see
`docs/live-path-reachability.md`.

## Surface and control fallback (4)

### `0x415730` — fall back to a second virtual control query after a lost-surface sentinel

- **VA / file offset**: `0x415730` / `0x15730` (87,856).
- **Behavior**: Tests the 4-mask at control `+0x54`. In either branch it calls the two small field getters at `0x415ce0` and `0x415d00` and ignores their returns, obtains the current page object from `0x42d040`, asks virtual slot `+0x60`, and when the answer is negative reads virtual slot `+0x6c` as the result. It repeats the query once for each getter branch.
- **Reached from**: 1 direct caller(s): `0x4153b0`; 2 raw xref site(s).
- **Player-visible reach**: support — only caller `0x4153b0` invokes it after a control draw/hit-test answer equals the lost-surface sentinel; it paints no pixels itself.
- **Evidence**: ev 2af6.d9d3.95a1.9654.0503.71a5.d25f.4df9.849a.f0e3.2e13.8ee9.6f54.7030.83d1.d863; file-offset mapping ev 3e46.dcb8.5a65.7615.d514.a5cb.7621.8481.18f4.2886.d954.e139.2240.bbeb.d2dd.936b; incoming xrefs ev c0ea.8bbd.2891.2e23.27d3.7f11.1b28.2e63.3185.8c69.dff7.988c.2530.6ad9.6b46.846c.
- **Our client**: No counterpart by VA. The client has no DirectDraw page object; the fallback renderer is `paint_native_scene` (`crates/o2jam-render/src/lib.rs:1120`).

### `0x4160e0` — return the 4-mask from a control flag word

- **VA / file offset**: `0x4160e0` / `0x160e0` (90,336).
- **Behavior**: Reads the word at `+0x54`, keeps its 4-mask, and returns that value. No other work.
- **Reached from**: 1 direct caller(s): `0x42bbb0`; 1 raw xref site(s).
- **Player-visible reach**: support — direct caller `0x42bbb0` uses it while deciding whether to fall back to surface restore.
- **Evidence**: ev 7e13.9e65.75d7.62fc.856f.c258.e948.c85c.598e.065d.67ab.75df.f733.6b26.e079.81ad; file-offset mapping ev acf6.3fd2.8d59.cb21.1e0a.0656.f2b0.101b.117a.e9b3.7692.c045.08d7.24cd.fa78.fba6; incoming xrefs ev 3b14.12e3.3c8f.da09.10e0.e2de.1d84.985d.28e5.65c0.b5a8.0560.a6e3.604e.0ec7.fb09.
- **Our client**: No counterpart by VA; the shell has no control flag word or DirectDraw restore path.

### `0x417870` — query display capacity and restore on a low-memory answer

- **VA / file offset**: `0x417870` / `0x17870` (96,368).
- **Behavior**: With both inputs zero it returns zero. Otherwise it clears a 16-byte query record, sets one word in it, asks the display object through virtual `+0x5c` for two outputs, and when the first input is nonzero with a small second output calls restore `0x52f010`. It returns the first output shifted down twenty bits.
- **Reached from**: 8 direct caller(s): `0x413030`, `0x423b40`, `0x4249f0`, `0x498410`, `0x4984d0`, `0x4db6e0`, …; 8 raw xref site(s).
- **Player-visible reach**: support — called from the focus-change handler `0x5a4b90` and other UI sites; it does not paint pixels.
- **Evidence**: ev fe76.bcb4.6478.491c.f677.3971.8b8a.6c32.b766.b990.3ad6.a94e.c1f7.5d84.e4c6.3ca8; file-offset mapping ev a4af.9ab9.9c62.4fab.fb2d.9146.ef78.0618.d437.74b9.5d11.8d66.5ba8.252b.5e9f.3038; incoming xrefs ev 5356.f1ab.19a2.4e82.af0e.d3c7.2e0e.820a.8f8f.be44.5fcd.fc50.ec1b.8c5b.1db7.029c.
- **Our client**: No counterpart by VA; the client has no display-device query or restore call.

### `0x418d90` — test the display-state lost flag

- **VA / file offset**: `0x418d90` / `0x18d90` (101,776).
- **Behavior**: Reads the global display state through `0x415c60` and reports whether the `0x80000` bit of its `+8` word is set.
- **Reached from**: 19 direct caller(s): `0x45e010`, `0x45e450`, `0x45ea60`, `0x45ecd0`, `0x45f190`, `0x45f480`, …; 33 raw xref site(s).
- **Player-visible reach**: support — one caller is the `WM_ACTIVATEAPP` handler `0x5a4b90`; the flag gates its notice and audio branches.
- **Evidence**: ev 735e.4ee0.32f8.5221.3d5e.3f2a.f1c4.4e3c.6711.b3a8.a409.e4f1.56d7.062f.9e60.5488; file-offset mapping ev d3f4.8054.9038.364e.dfb5.a882.0537.8608.e39c.8298.0b74.fc0a.b077.e3af.5261.443c; incoming xrefs ev a692.3aa2.c68e.ce86.6bce.082e.1637.1de8.e5d2.a7ac.07fd.45a8.ff38.3732.d369.e216.
- **Our client**: No VA counterpart. Focus events reach `crates/o2jam-shell/src/window.rs:512` (`should_exit_after_frame`) but no device-lost flag is consumed.

## Image-to-surface pixel copy (5)

### `0x416100` — copy one image frame into a page rectangle with optional color key

- **VA / file offset**: `0x416100` / `0x16100` (90,368).
- **Behavior**: When a frame argument is supplied it temporarily selects that frame through `0x41e0f0` and restores the old one afterwards. It takes the destination rectangle from the caller or from the current frame record, snapshots page state and color key, locks the page, copies 16-bit pixel rows through `0x416310`, then unlocks.
- **Reached from**: 2 direct caller(s): `0x417770`, `0x4177f0`; 2 raw xref site(s).
- **Player-visible reach**: yes — this is the pixel upload used when a Playing skin or control image is staged into a surface.
- **Evidence**: ev b637.b5c1.ea1b.7d81.e010.a2f1.74ef.e598.fb40.b472.970b.ac4d.31cb.4a67.45ea.6c66; file-offset mapping ev ec3a.87c0.7971.cf7e.9da5.3850.37ad.a829.d4d7.baf7.0be0.e7ec.6d45.3093.d733.6a6e; incoming xrefs ev a33e.3df2.1580.f266.1b26.1723.2409.d974.78ca.ac40.9097.36ea.b8b5.8378.e128.3777.
- **Our client**: No VA counterpart. Rust paints decoded figures through `blit_rgb888` (`crates/o2jam-render/src/lib.rs:1080`) and `blit_skips_colorkey_and_clips` (`:2424`); frame records come from `crates/o2jam-file/src/sprites.rs:52`.

### `0x416250` — paste a packed run list from the current image frame

- **VA / file offset**: `0x416250` / `0x16250` (90,704).
- **Behavior**: Reads the current frame record, derives the destination rectangle, asks `0x422660` for the source array, then passes the run data to `0x416410` with the image stride and origin.
- **Reached from**: 2 direct caller(s): `0x417770`, `0x417820`; 2 raw xref site(s).
- **Player-visible reach**: yes — the second image-kind copy path used by Playing skin staging and generic control drawing.
- **Evidence**: ev 8829.1554.be9c.701d.71ca.a077.5ab9.5360.3617.940a.5ff3.0e8b.28ec.a544.6c69.6eab; file-offset mapping ev acda.4ab4.2b01.f6ba.4c8d.484e.8bdc.b582.b136.d929.ece0.b226.74b2.efc2.fb4b.9c4c; incoming xrefs ev 7e00.dbcd.f2a7.1fa6.ce45.4cae.14b7.f789.f15d.0543.d8ac.ad9a.016a.def4.23ee.3975.
- **Our client**: No VA counterpart. Decoded layered bodies are painted through `crates/o2jam-shell/src/avatar.rs:683` and `crates/o2jam-file/src/sprites.rs:205`.

### `0x416310` — copy 16-bit pixel rows with a source stride and color key

- **VA / file offset**: `0x416310` / `0x16310` (90,896).
- **Behavior**: For each destination row it copies `width` source words at the source stride into the destination rectangle. When the color key is nonzero it copies only words that differ from that key.
- **Reached from**: 1 direct caller(s): `0x416100`; 1 raw xref site(s).
- **Player-visible reach**: yes — the row-copy body of the image-to-page blit, so its key handling decides visible background pixels.
- **Evidence**: ev 23cf.efa5.7c7c.1afb.01f5.e480.4c21.0f35.e5dc.485a.345b.9861.2c2e.d8e8.c21d.a047; file-offset mapping ev e5c4.54a9.ac95.f280.219b.1215.fafa.bb69.5a17.3015.7a1a.1f52.16da.f512.d5b0.9e07; incoming xrefs ev ea56.cb7e.9a30.03fb.5938.e2a8.b79e.66b7.09d1.52df.57a3.6d98.80de.a739.c2af.d5c8.
- **Our client**: No VA counterpart. Equivalent rectangle/key behavior is `blit_image_in_rect` (`crates/o2jam-render/src/lib.rs:791`) with guard `blit_skips_colorkey_and_clips` (`:2424`).

### `0x416410` — copy terminated pixel runs into a surface

- **VA / file offset**: `0x416410` / `0x16410` (91,152).
- **Behavior**: Walks a list of short records. A zero length ends the list; each other record gives two offsets and a pixel count, and that many 16-bit words are copied to the destination origin plus the offsets.
- **Reached from**: 1 direct caller(s): `0x416250`; 1 raw xref site(s).
- **Player-visible reach**: yes — it writes the visible pixels for the packed-run image kind.
- **Evidence**: ev fdf8.4bf8.1582.846f.8de6.235b.7dfd.1a2f.5b78.cbe4.03c8.a208.ed7f.6800.0091.e727; file-offset mapping ev c582.c8ba.0a10.36e1.7555.cde0.ccc1.6551.86db.8fb4.a79d.862e.51bb.1742.89c4.84ab; incoming xrefs ev 65d3.d125.0ad3.7a50.9d29.dcec.fa16.7176.c2ed.2567.d2a2.3dc6.0c07.dfd5.cf60.ca50.
- **Our client**: No VA counterpart. Figure layers carry the same per-layer pixel data in `crates/o2jam-file/src/sprites.rs:52` and are composited in `crates/o2jam-shell/src/avatar.rs:683`.

### `0x417770` — choose the image copy path by image kind

- **VA / file offset**: `0x417770` / `0x17770` (96,112).
- **Behavior**: Asks `0x4300f0` for the current image kind. Kind 1 resolves an image with `0x42fe90` and calls `0x416100`; kind 2 resolves one with `0x42feb0` and calls `0x416250`. Both use the current frame from `0x42f990`.
- **Reached from**: 4 direct caller(s): `0x432510`, `0x4dc750`, `0x4dcab0`, `0x4dd290`; 4 raw xref site(s).
- **Player-visible reach**: yes — direct callers include the Playing kit staging functions `0x4dc750`, `0x4dcab0` and `0x4dd290`, so this chooses how playing-scene art reaches the page.
- **Evidence**: ev b7a7.afc8.2638.47cc.3458.34cf.21ce.f598.1034.15e7.1767.8e3a.fc81.bff1.03f7.68cb; file-offset mapping ev 4a02.54ab.6488.3095.6fb1.f0f1.be82.55b3.c24e.d5a3.4d67.5b6f.1731.2741.e65d.b922; incoming xrefs ev 0297.2ce4.8b3d.067a.6354.f688.8aad.c62a.7f00.9231.5768.201c.781a.3dfe.b324.e471.
- **Our client**: No VA counterpart. The staged skin path lives in `crates/o2jam-shell/src/playing.rs:1289` (`lane_sprite`) and `:1304` (`skin_frames`), staged by `crates/o2jam-shell/src/staging.rs:1427`.

## Shared pointer-vector templates (17)

### `0x417900` — append one value to a control image vector

- **VA / file offset**: `0x417900` / `0x17900` (96,512).
- **Behavior**: For a nonzero argument it forwards to `0x4193a0`, which pushes one value into the image/pointer vector.
- **Reached from**: 18 direct caller(s): `0x42fb30`, `0x42fba0`, `0x42fef0`, `0x4351e0`, `0x456af0`, `0x4572c0`, …; 20 raw xref site(s).
- **Player-visible reach**: support — 18 direct call sites across control and state code; registration only, no paint or wire frame.
- **Evidence**: ev 9c4a.d6e2.bfb1.c523.76a8.7c62.581c.a16b.1bec.d312.fda8.3346.ddf3.c94a.47f7.ef85; file-offset mapping ev 1b92.e3e1.70a1.c76f.7e45.6b99.fee2.d8ef.e735.a2fb.3d6c.20f8.0e17.b547.ee0a.cf0f; incoming xrefs ev d982.3aaa.03c9.f7d3.b284.544e.6fbb.19ef.b187.aeb7.7db8.6036.e094.e90e.68c6.9bd8.
- **Our client**: No VA counterpart. Shell staging uses `Vec<ScenePlacement>` (`crates/o2jam-shell/src/staging.rs:1331`).

### `0x41a280` — insert into the first pointer vector with growth

- **VA / file offset**: `0x41a280` / `0x1a280` (107,136).
- **Behavior**: Computes the vector's live length and capacity, checks for overflow, allocates a larger buffer when needed, copies the two halves around the insertion point, fills the inserted slots with one supplied value, and repairs the begin, end and capacity words.
- **Reached from**: 1 direct caller(s): `0x419970`; 1 raw xref site(s).
- **Player-visible reach**: support — reached from the `0x419970` insert wrapper; a container primitive, not a paint action.
- **Evidence**: ev be8c.6c47.6748.dba7.e8ab.bcc4.ce49.bb48.a1ae.150b.d2c6.54fb.60ec.b718.3c90.40eb; file-offset mapping ev af0e.a148.b140.5b85.9a6b.8c26.cbcc.84d2.423d.a148.b90e.a17e.9ddd.1145.d305.70f1; incoming xrefs ev fdb6.99a9.df74.b1f5.5aa0.7c5e.031c.d0c2.66a6.8f06.93f8.985c.050d.1190.145c.f049.
- **Our client**: No counterpart by VA. The shell uses Rust `Vec` for staged placements (`crates/o2jam-shell/src/staging.rs:1331`).

### `0x41a669` — finish growth of the first pointer vector

- **VA / file offset**: `0x41a669` / `0x1a669` (108,137).
- **Behavior**: Frees the old buffer when one exists, then stores the new begin, end and capacity words computed by its caller.
- **Reached from**: 1 direct caller(s): `0x41a280`; 1 raw xref site(s).
- **Player-visible reach**: support — growth epilogue called only by `0x41a280`.
- **Evidence**: ev 1b96.2ca1.d3f9.d57f.d9be.ac8f.7618.80f3.0b03.454b.0337.3d1c.32d2.8d13.15c9.9e9a; file-offset mapping ev ab57.6f09.9df8.5c02.4273.0339.b2a2.96ea.e413.f9c8.8312.623a.2731.54dd.7c60.f3be; incoming xrefs ev cfff.98af.9792.1806.196e.fe21.0fb9.8a01.9a7e.a66f.4f38.0be2.2979.b522.0b04.4c3d.
- **Our client**: No counterpart by VA; allocation is owned by Rust `Vec`.

### `0x41a83f` — fill the overwritten range of the first pointer vector

- **VA / file offset**: `0x41a83f` / `0x1a83f` (108,607).
- **Behavior**: Walks a pointer range from its start to its end and writes one supplied word into every slot.
- **Reached from**: 1 direct caller(s): `0x41a280`; 1 raw xref site(s).
- **Player-visible reach**: support — overwrite epilogue called only by `0x41a280`.
- **Evidence**: ev ba28.bd98.c4c2.ed13.c3b9.e7ba.97c7.1250.63cc.e053.7cc8.4562.1067.a972.031a.baec; file-offset mapping ev 51da.9433.1294.5604.0839.3c23.8b1e.f674.66d2.ec17.8342.8141.7d2e.d78a.a0f6.d9b2; incoming xrefs ev 0e6f.6eb6.7535.2741.73c3.f175.0894.4326.861d.2cfa.8b85.8a97.ba15.4883.ca29.0be0.
- **Our client**: No counterpart by VA.

### `0x41aa10` — fill a range and return its end pointer

- **VA / file offset**: `0x41aa10` / `0x1aa10` (109,072).
- **Behavior**: Calls `0x41c230` to write a value into a range, then returns the address one past that range.
- **Reached from**: 1 direct caller(s): `0x41a280`; 1 raw xref site(s).
- **Player-visible reach**: support — helper of the first pointer-vector insert path.
- **Evidence**: ev a05d.aa32.068e.9a62.3c4f.3db4.50f4.8bd5.927c.3bc3.3b30.71b5.9fce.d664.05ed.f3d0; file-offset mapping ev 01e8.7d4d.941b.2bfb.49d9.7994.8893.c853.504c.964f.fe23.f1fb.86f9.0e98.b5fe.ecbc; incoming xrefs ev 7552.c0f4.9c55.59ed.abb2.2cff.db61.d2cf.1f6c.d205.6751.00ec.8608.4c1d.28d6.6367.
- **Our client**: No counterpart by VA.

### `0x41be40` — shift a pointer range to open an insertion gap

- **VA / file offset**: `0x41be40` / `0x1be40` (114,240).
- **Behavior**: Counts how many words lie between two pointers and moves that many words to a destination computed from the third pointer.
- **Reached from**: 1 direct caller(s): `0x41a280`; 1 raw xref site(s).
- **Player-visible reach**: support — helper of the first pointer-vector insert path.
- **Evidence**: ev d206.bcbd.c5f8.2991.4361.a5e6.6152.ac37.b0e9.8bc8.6645.b6b2.74d9.d0a0.70e4.db0a; file-offset mapping ev a4e5.d404.26a6.68bf.2d8c.8288.f346.2575.536e.f48c.c82c.70c8.32bf.e751.b431.2bef; incoming xrefs ev d5f2.c255.4a61.1204.6f25.acca.47c0.bd3a.21df.3646.19d7.8273.d589.ccda.016c.056d.
- **Our client**: No counterpart by VA.

### `0x41c140` — forward a pointer range move

- **VA / file offset**: `0x41c140` / `0x1c140` (115,008).
- **Behavior**: Passes its three arguments to `0x41c380` and returns.
- **Reached from**: 1 direct caller(s): `0x41a280`; 1 raw xref site(s).
- **Player-visible reach**: support — thin wrapper called only by `0x41a280`.
- **Evidence**: ev 7ab9.5be9.905b.1d0d.759f.f837.9c10.6c2c.4ab2.c3ae.1374.f0ab.89bd.2d5a.b84e.2091; file-offset mapping ev cb8b.ae64.a414.2079.9dc2.b4fb.eda1.c393.1e06.6673.3182.6a29.2a58.3d86.df4c.93ab; incoming xrefs ev 7a18.0849.9234.079f.f6c0.2be1.af59.6b4d.dc5c.9db0.2c4c.97b6.b19a.18a8.555b.f958.
- **Our client**: No counterpart by VA.

### `0x41c230` — fill a pointer range with one value

- **VA / file offset**: `0x41c230` / `0x1c230` (115,248).
- **Behavior**: Steps a cursor through a range and writes the same supplied value at each slot.
- **Reached from**: 3 direct caller(s): `0x4193a0`, `0x41a280`, `0x41aa10`; 3 raw xref site(s).
- **Player-visible reach**: support — called by `0x4193a0`, `0x41a280` and `0x41aa10`; also used for single-value appends.
- **Evidence**: ev 1c23.cafd.5c0f.74f8.661d.79a8.1b69.9d58.368c.d7cc.b7b4.e90a.9ed6.9cbd.8dca.fa9a; file-offset mapping ev b442.d9f9.217c.0615.1d77.fe36.3cf2.d8b2.3d4b.f3b0.f39d.19bd.b0c4.6258.1bb6.3767; incoming xrefs ev 5082.5f9b.2cf9.66de.e64f.caf2.3b2c.36dc.3687.07fa.ec21.3080.e039.18c3.1b31.5e4c.
- **Our client**: No counterpart by VA.

### `0x41c380` — move a pointer range and return the destination end

- **VA / file offset**: `0x41c380` / `0x1c380` (115,584).
- **Behavior**: Counts the words between two pointers, moves them to the supplied destination, and returns one past the destination end.
- **Reached from**: 2 direct caller(s): `0x41a280`, `0x41c140`; 4 raw xref site(s).
- **Player-visible reach**: support — called by `0x41a280` and `0x41c140`.
- **Evidence**: ev f4d4.ab7b.0cf3.5611.2d29.be26.2925.7ccd.574e.9357.39b6.ad87.f094.23bd.5782.8528; file-offset mapping ev 2726.f3c0.663a.aee7.a05c.261a.92d0.dd48.0ac3.9f79.89c3.6320.4bae.6825.acc4.340c; incoming xrefs ev 7b7f.f4b3.aa62.a961.ef49.cc8a.bd9a.ae95.efc3.5cf1.3218.b1d0.bdd7.fa79.4a2c.5e5c.
- **Our client**: No counterpart by VA.

### `0x41d0e0` — insert into the second pointer vector with growth

- **VA / file offset**: `0x41d0e0` / `0x1d0e0` (119,008).
- **Behavior**: Same insert-with-growth shape as the first vector: measure length and capacity, grow when needed, move the halves, fill the inserted slots, then repair begin, end and capacity words.
- **Reached from**: 1 direct caller(s): `0x41cfc0`; 1 raw xref site(s).
- **Player-visible reach**: support — reached from the `0x41cfc0` insert wrapper; a container primitive, not a paint action.
- **Evidence**: ev 8be9.3227.e5c0.dbd3.3407.2e3b.96dc.8a2b.9d28.c88e.c3d5.b963.e6eb.962b.3a7b.8401; file-offset mapping ev 7854.f102.8358.10fb.dc13.6ba5.b661.8ea4.9118.349e.e56d.fb90.1adc.bcd1.03b1.488f; incoming xrefs ev d5ea.00ae.8c8d.44d3.07db.77ee.778f.2a02.1fcd.8638.7507.0dd1.e576.76d8.9b02.99ed.
- **Our client**: No counterpart by VA. The shell uses Rust `Vec` for staged placements (`crates/o2jam-shell/src/staging.rs:1331`).

### `0x41d4c9` — finish growth of the second pointer vector

- **VA / file offset**: `0x41d4c9` / `0x1d4c9` (120,009).
- **Behavior**: Frees the old buffer when present and installs the new begin, end and capacity words from the caller's frame.
- **Reached from**: 1 direct caller(s): `0x41d0e0`; 1 raw xref site(s).
- **Player-visible reach**: support — growth epilogue called only by `0x41d0e0`.
- **Evidence**: ev a559.7358.b086.9530.5e58.59c4.5c34.0f1a.6e4e.b6ec.acc3.fc03.16c0.e997.91ab.fb6a; file-offset mapping ev ed01.5fb5.ecb1.4920.c099.5da0.1663.9e18.4977.f9cb.6d63.31e3.0acb.0d39.1f72.60aa; incoming xrefs ev e59e.1e2a.47cd.c051.f1bd.1f4c.2d66.4e19.3f94.aa33.3362.97af.6426.c48b.d6a5.34dd.
- **Our client**: No counterpart by VA.

### `0x41d69f` — fill the overwritten range of the second pointer vector

- **VA / file offset**: `0x41d69f` / `0x1d69f` (120,479).
- **Behavior**: Writes one supplied word into every pointer slot of the given range.
- **Reached from**: 1 direct caller(s): `0x41d0e0`; 1 raw xref site(s).
- **Player-visible reach**: support — overwrite epilogue called only by `0x41d0e0`.
- **Evidence**: ev 1395.3782.c723.4f3c.3d85.5bbf.3cc5.e33e.f270.69c9.7a76.0df9.c85f.334a.f67c.fb33; file-offset mapping ev fd66.1252.997f.397b.8fe0.c4b0.603e.21d6.8f7e.4097.f485.12f9.7e44.eaea.c376.7f91; incoming xrefs ev ff97.2be8.6865.ee5c.5f6d.2d7e.a50d.f803.9921.dadb.3676.416c.fc39.4897.d9b6.3971.
- **Our client**: No counterpart by VA.

### `0x41d870` — fill a range and return its end pointer in the second vector path

- **VA / file offset**: `0x41d870` / `0x1d870` (120,944).
- **Behavior**: Calls `0x41db70` to fill the range, then returns the end address.
- **Reached from**: 1 direct caller(s): `0x41d0e0`; 1 raw xref site(s).
- **Player-visible reach**: support — helper of the second pointer-vector insert path.
- **Evidence**: ev ecce.76d5.9b3c.9a44.7da9.9cea.ecc2.ff0c.1750.c68f.e3e8.7f8d.86d7.8e52.50d9.8ed0; file-offset mapping ev 3c5f.cd78.53a8.3613.c106.af8b.760e.639b.61a1.919e.0c61.d155.a593.f694.53cf.6705; incoming xrefs ev 2a68.478b.a988.d3a7.0c03.da65.5348.7d63.deb1.4364.a57a.7ce1.2821.f896.9826.2ab6.
- **Our client**: No counterpart by VA.

### `0x41da20` — shift a pointer range in the second vector path

- **VA / file offset**: `0x41da20` / `0x1da20` (121,376).
- **Behavior**: Moves the words between two pointers to a destination derived from a third pointer.
- **Reached from**: 1 direct caller(s): `0x41d0e0`; 1 raw xref site(s).
- **Player-visible reach**: support — helper of the second pointer-vector insert path.
- **Evidence**: ev be58.c6f5.1df8.80f9.c9fe.e0ce.c09d.acb9.6b28.48e5.766f.1886.fe9e.beb1.f001.aa13; file-offset mapping ev 3d04.8152.1fed.c1d8.529c.daa9.e4bb.7fc7.bea7.bfd0.445d.fbef.d6c8.698c.106f.bc47; incoming xrefs ev de23.76de.f874.3de4.9366.4a84.e6a9.8d4f.b33f.91a1.9d12.f701.3f28.e27c.7864.f25d.
- **Our client**: No counterpart by VA.

### `0x41db20` — forward a pointer range move in the second vector path

- **VA / file offset**: `0x41db20` / `0x1db20` (121,632).
- **Behavior**: Passes its arguments to `0x41dbe0` and returns.
- **Reached from**: 1 direct caller(s): `0x41d0e0`; 1 raw xref site(s).
- **Player-visible reach**: support — thin wrapper called only by `0x41d0e0`.
- **Evidence**: ev eece.a561.4cbf.92e5.44de.84dd.0ca3.fc45.2b68.0708.1387.7f96.04e6.7a44.c505.cfe0; file-offset mapping ev acc0.f6b7.b83f.7596.e735.d333.f273.731d.0a06.75e8.833f.d14c.29fc.9e6e.4ebd.ba48; incoming xrefs ev ca3f.5bec.97d1.d553.9180.2dc3.1566.a3b9.1ab0.a6ca.a15e.090f.ab91.cefd.a38d.0ff2.
- **Our client**: No counterpart by VA.

### `0x41db70` — fill a pointer range with one value in the second vector path

- **VA / file offset**: `0x41db70` / `0x1db70` (121,712).
- **Behavior**: Writes the supplied value into each slot of the range.
- **Reached from**: 3 direct caller(s): `0x41ce80`, `0x41d0e0`, `0x41d870`; 3 raw xref site(s).
- **Player-visible reach**: support — called by `0x41ce80`, `0x41d0e0` and `0x41d870`.
- **Evidence**: ev 07db.0f84.1cb1.b4f4.3bd7.e5e2.7a64.8d81.ae2c.b759.82d2.43e7.350b.19e3.2c28.32ab; file-offset mapping ev 7305.cbfb.c8b8.1606.72f1.cc8c.e689.2040.3220.a2bf.5ee3.b2be.44fe.816a.54eb.9263; incoming xrefs ev 1359.5095.35c8.1d5a.f3cd.21e2.8620.d3bf.4652.cba1.94d0.3c3d.b5cf.4034.6f48.8ada.
- **Our client**: No counterpart by VA.

### `0x41dbe0` — move a pointer range and return the destination end in the second vector path

- **VA / file offset**: `0x41dbe0` / `0x1dbe0` (121,824).
- **Behavior**: Counts the pointer words, moves them, and returns the address after the destination range.
- **Reached from**: 2 direct caller(s): `0x41d0e0`, `0x41db20`; 4 raw xref site(s).
- **Player-visible reach**: support — called by `0x41d0e0` and `0x41db20`.
- **Evidence**: ev f2d2.2b64.55bc.cb61.d654.d797.d8e4.d272.8921.ea37.8113.650d.812b.e9b0.6dad.c6c7; file-offset mapping ev 8ae8.62f6.0705.234b.0cf5.3cd0.4603.0eee.8815.94a9.e6fa.c9a1.15f3.7634.f442.9cf7; incoming xrefs ev a7bd.45cd.c23e.ba96.8b62.57e6.5094.4cf8.4f5f.b0ee.56b7.19b6.36a3.16e5.f03b.e41b.
- **Our client**: No counterpart by VA.

## Image and font field state (7)

### `0x41c580` — reset a CDXFont object

- **VA / file offset**: `0x41c580` / `0x1c580` (116,096).
- **Behavior**: Clears six fields at low offsets and zeroes a 256-byte scratch block.
- **Reached from**: 1 direct caller(s): `0x41c4d0`; 1 raw xref site(s).
- **Player-visible reach**: support — reached from the `0x41c4d0` constructor; it initializes font state but draws no glyph.
- **Evidence**: ev e99a.ec94.3398.7cb5.5cf8.8b80.661e.11c3.1823.1f83.667a.d3f3.5d4e.0c15.5de1.18ff; file-offset mapping ev 4e2d.3f7c.db91.3d5f.23d6.8aab.05f8.0b0e.ffbb.69b0.2c39.9f1b.b082.b6fe.49b2.95f5; incoming xrefs ev aa4b.1a3e.7aac.ba94.05b0.e931.4712.f425.5147.9098.d417.8515.6978.791a.1b8c.82b7.
- **Our client**: No VA counterpart. The client rasterizes text with `crates/o2jam-text/src/font.rs:99` (`system_font`) and `:361` (`draw_text`).

### `0x41de70` — reset a CDXImageBase object

- **VA / file offset**: `0x41de70` / `0x1de70` (122,480).
- **Behavior**: Clears frame and animation fields, sets two defaults (one and three), and zeroes scratch blocks of 256, 8 and 16 bytes.
- **Reached from**: 1 direct caller(s): `0x41dd90`; 1 raw xref site(s).
- **Player-visible reach**: support — reached from the `0x41dd90` constructor; it establishes image state but paints no pixels.
- **Evidence**: ev 8040.e698.386d.75c3.6b0f.abb5.4715.51c2.d95d.36dc.35cd.0175.2cf9.5813.ad70.539b; file-offset mapping ev 7593.7112.d08a.ea80.a87c.959b.5e46.afcc.2709.cef3.e8be.ef42.0ad0.ada2.acba.9e9b; incoming xrefs ev 0501.ae95.4c2a.2f24.7ec2.3c95.12a6.7f3b.6bc8.ba00.c9c0.5767.4df5.9c81.75c2.8418.
- **Our client**: No VA counterpart. The closest Rust state is decoded frame records in `crates/o2jam-file/src/sprites.rs:52`.

### `0x41e0d0` — store the image kind word

- **VA / file offset**: `0x41e0d0` / `0x1e0d0` (123,088).
- **Behavior**: Writes the supplied word at `+0x14`; control staging sets one of its small kind values there.
- **Reached from**: 2 direct caller(s): `0x413030`, `0x413be0`; 4 raw xref site(s).
- **Player-visible reach**: support — the value selects which draw branch a staged image takes.
- **Evidence**: ev 6d32.217f.386f.cff0.53e1.0b25.e406.d542.50fa.0369.bc73.ce0b.b80a.8765.7300.7462; file-offset mapping ev 68a2.2940.7a2c.9917.f9dc.2215.2eda.4834.945b.b91a.5542.7ece.3e42.61cd.9010.0c91; incoming xrefs ev e03b.3808.44ac.22ee.1999.a9c8.f22d.171b.fb33.9aec.6c41.d510.0a52.d3cf.2754.d522.
- **Our client**: No VA counterpart. Kind selection in the client is implicit in the staged layer type (`crates/o2jam-file/src/sprites.rs:52`).

### `0x41e120` — return the current image frame index

- **VA / file offset**: `0x41e120` / `0x1e120` (123,168).
- **Behavior**: Reads and returns the frame index word at `+0x134`.
- **Reached from**: 3 direct caller(s): `0x416100`, `0x416250`, `0x4a21a0`; 3 raw xref site(s).
- **Player-visible reach**: yes — both image copy paths use this value to pick the frame they upload.
- **Evidence**: ev aec1.1f01.dcce.903e.b401.c538.d1c4.d83a.cb2f.5404.55d5.2eca.dce8.9ed1.a803.2698; file-offset mapping ev 6e7d.3746.8050.801a.917f.b30b.e4bd.ca13.b7ee.fad1.e3a3.c13a.2053.8f9e.82a2.a776; incoming xrefs ev e937.cc6b.74c3.2289.2585.9039.dc94.0099.5e13.fadb.586b.d82c.7a3e.2b14.eb46.222d.
- **Our client**: No VA counterpart. Frame selection in Rust is explicit in `stepped_frame` (`crates/o2jam-shell/src/playing.rs:1270`).

### `0x41e140` — return the image frame count

- **VA / file offset**: `0x41e140` / `0x1e140` (123,200).
- **Behavior**: Reads and returns the 16-bit count at `+0x11c`.
- **Reached from**: 10 direct caller(s): `0x41e0f0`, `0x42fb30`, `0x42fba0`, `0x42fc10`, `0x42fc70`, `0x42fcd0`, …; 11 raw xref site(s).
- **Player-visible reach**: yes — the count clamps frame selection and is staged into the control frame count by `0x42fc70`/`0x42fc10`.
- **Evidence**: ev e368.b966.618b.e1a3.ada9.2f8e.34d5.9d9c.97ad.d0c7.1cac.8355.8095.e5be.fa30.d20a; file-offset mapping ev 8a43.2481.a9a0.b399.4245.65f5.5f89.dd9f.4733.5e15.6e5b.8669.9968.13d8.a826.af7f; incoming xrefs ev 0c63.052d.4b09.42ff.ff53.f79e.d73f.2915.2908.e088.dcfc.08fb.cea3.6a78.f2f6.f057.
- **Our client**: No VA counterpart. Frame counts come from decoded bodies (`crates/o2jam-file/src/sprites.rs:71`).

### `0x41e180` — return one frame record by index

- **VA / file offset**: `0x41e180` / `0x1e180` (123,264).
- **Behavior**: For the all-ones input it uses the current frame index; otherwise it accepts an index below the frame count. It returns the address at the frame-array base plus that index times twenty bytes, or null when out of range.
- **Reached from**: 8 direct caller(s): `0x416100`, `0x416250`, `0x423b40`, `0x42ebc0`, `0x4da630`, `0x4e4ee0`, …; 9 raw xref site(s).
- **Player-visible reach**: yes — the record carries the signed anchor and size used to place an image frame.
- **Evidence**: ev 22a0.27ee.9471.2817.f026.c430.cedf.d734.e00d.aa1c.51bd.be92.0345.e826.51b0.64dd; file-offset mapping ev ebb0.64cf.f0f9.304a.99c9.99ea.e886.b534.9696.ad86.cf9e.7f50.fa46.8a5b.7a2c.5199; incoming xrefs ev 41dd.1b91.fd86.8f3b.7af5.288f.2a17.b0ff.d1db.f67b.5cb2.e70c.4060.f00f.2ba7.89bc.
- **Our client**: No VA counterpart. Rust reads the same record fields through `Sprite55Image` (`crates/o2jam-file/src/sprites.rs:52`) and `frame_data` (`:205`); guard `anchored_blit_adds_signed_anchor_to_position` (`crates/o2jam-render/src/lib.rs:3436`).

### `0x41e1f0` — append one frame record to the image vector

- **VA / file offset**: `0x41e1f0` / `0x1e1f0` (123,376).
- **Behavior**: Passes the image object's frame-vector address and the supplied record to `0x005f27f0`.
- **Reached from**: 1 direct caller(s): `0x40c190`; 1 raw xref site(s).
- **Player-visible reach**: support — loader-side registration only; the vector is read later by `0x41e180`.
- **Evidence**: ev 64bb.4503.1e38.da22.eea0.4e90.8490.f2e3.f1b2.c0db.9f53.aa0d.f859.9a90.f399.5e33; file-offset mapping ev d4a0.e4ac.0f79.8443.9e7f.b79a.52fe.510c.a7b4.8fa9.2781.6931.85c0.d5be.8c39.87b3; incoming xrefs ev 26f0.7905.4ee0.d9db.0884.121c.3257.4874.e4f5.b65b.5860.5b62.06a4.2999.5daa.3b29.
- **Our client**: No VA counterpart. Frame vectors are built by the parsers in `crates/o2jam-file/src/sprites.rs:71`.

## Surface page helpers (12)

### `0x422660` — return a pointer from the surface index array

- **VA / file offset**: `0x422660` / `0x22660` (140,896).
- **Behavior**: Indexes the pointer array at `+0x124` with the supplied word and returns the stored pointer.
- **Reached from**: 2 direct caller(s): `0x416250`, `0x423b40`; 2 raw xref site(s).
- **Player-visible reach**: support — `0x416250` uses it to find the source pixels for a packed-run copy.
- **Evidence**: ev 1a0a.9052.698d.7a6c.94bf.f0d6.1765.42f3.15a0.4304.c8fe.3c91.6bf8.90d4.dcf4.4dc9; file-offset mapping ev 9a6c.350d.b490.4b63.0041.ecdc.1183.0492.3494.1e2a.a12d.79af.8cf6.c624.aa52.ea31; incoming xrefs ev f565.182b.18a7.4e4d.8d63.7907.823c.6567.9fc3.2138.4a22.a28b.a7ed.bf85.f379.a7e3.
- **Our client**: No counterpart by VA.

### `0x4227c0` — reset a CDXSurface object

- **VA / file offset**: `0x4227c0` / `0x227c0` (141,248).
- **Behavior**: Clears the page and slot arrays and several count words, sets one capacity value and one default, then calls virtual `+0x1c` with one.
- **Reached from**: 1 direct caller(s): `0x422710`; 1 raw xref site(s).
- **Player-visible reach**: support — reached from the `0x422710` CDXSurface constructor; setup only, no painted frame.
- **Evidence**: ev ef0b.78d1.b265.85fe.f770.430a.5640.a39b.2842.1beb.07f3.42b9.5800.b40d.eefc.059d; file-offset mapping ev 0fab.4ed0.6432.f3fb.6266.010a.dc3d.8e14.af1e.97a5.851e.7c87.9c4d.69ce.3c49.6d0b; incoming xrefs ev 705a.6f2f.0900.14ec.0035.b90b.5c2b.8106.8ee2.bb5e.a2a5.61f3.fb8f.4969.5a00.784e.
- **Our client**: No counterpart by VA; the client has no DirectDraw surface object.

### `0x422e50` — release a CDXSurface object

- **VA / file offset**: `0x422e50` / `0x22e50` (142,928).
- **Behavior**: If the surface table exists it calls the virtual release method, frees each non-null surface entry, frees the table, clears a flag word, and repeats the free for the table pointer.
- **Reached from**: 1 direct caller(s): `0x52f550`; 1 raw xref site(s).
- **Player-visible reach**: support — reached from cleanup `0x52f550`; teardown only.
- **Evidence**: ev 304b.4f10.3aaf.d922.75af.8bff.df23.78d5.af22.73ce.9f50.7fa5.b0dc.6591.d2ab.1f79; file-offset mapping ev 7445.9d32.9793.762f.a40a.b1a0.e7a0.3584.d7c6.0415.afb6.1169.a58c.b3e4.0b87.7eaa; incoming xrefs ev bb61.7d30.9f68.aa9b.b95e.9892.d0be.7c5f.6392.d142.262b.4aa1.9b95.ea82.dc57.2035.
- **Our client**: No counterpart by VA.

### `0x424170` — return the display mode word

- **VA / file offset**: `0x424170` / `0x24170` (147,824).
- **Behavior**: Reads the 16-bit word at `+0x148` and returns it.
- **Reached from**: 1 direct caller(s): `0x430130`; 1 raw xref site(s).
- **Player-visible reach**: support — `0x430130` uses it to select which page object receives a mode change.
- **Evidence**: ev 8b07.8591.2fa0.2091.ce13.e0aa.89b1.07f7.537f.34c2.68c1.8955.5c7a.b54a.b289.24a0; file-offset mapping ev 12a8.3b3c.8fd6.662e.5cb4.00e7.467a.2bb5.d1bd.656f.36c6.1d89.29d5.fdd5.8730.39d7; incoming xrefs ev fcc0.f91e.0f02.aa88.bec2.0031.c1c1.162b.a250.31cc.e15a.6568.3503.e150.4a7c.ae53.
- **Our client**: No counterpart by VA; the client keeps no DirectDraw page mode word.

### `0x42afd0` — clamp a rectangle to the current page

- **VA / file offset**: `0x42afd0` / `0x2afd0` (176,080).
- **Behavior**: Compares a four-word rectangle against the current page bounds, pulls each edge back inside, and returns whether the rectangle was in bounds.
- **Reached from**: 2 direct caller(s): `0x42aec0`, `0x42ccb0`; 3 raw xref site(s).
- **Player-visible reach**: yes — this is the clip gate applied before a page blit, so it can change visible edges.
- **Evidence**: ev 5092.1018.06db.51d9.ce1d.f730.96aa.55e2.cf92.b5f1.1a84.073d.780f.a5b1.d83f.98f6; file-offset mapping ev ed9c.d3e3.6bdf.a6fc.550f.6be6.efc7.b62a.77c4.26ff.5a07.701d.7bae.791d.296b.b68d; incoming xrefs ev 88e4.7b3e.9a1a.b560.2705.fb77.20f7.61d4.ee18.86c7.a59e.bd4e.c1d3.835b.1eb9.e182.
- **Our client**: No VA counterpart. Rectangle clipping is implemented in `blit_image_in_rect` (`crates/o2jam-render/src/lib.rs:791`) with guard `blit_skips_colorkey_and_clips` (`:2424`).

### `0x42b9b0` — copy a twelve-byte surface record

- **VA / file offset**: `0x42b9b0` / `0x2b9b0` (178,608).
- **Behavior**: Copies twelve bytes from surface `+0x1f8` into the caller's buffer.
- **Reached from**: 1 direct caller(s): `0x415bd0`; 1 raw xref site(s).
- **Player-visible reach**: support — paired with `0x415ce0` to read a small surface state record.
- **Evidence**: ev 78f7.1372.67cc.159b.3a9d.dec8.808b.8214.b479.277c.3adc.bd4b.0482.a079.afbc.073a; file-offset mapping ev 8b88.b4e2.2cbd.4ed1.0d6d.069a.9db6.e11f.3576.bf68.8006.85d7.2ebb.76dc.8b00.ffef; incoming xrefs ev 7709.c89d.ea9c.ca52.9491.0391.e546.45a9.0d65.fcb7.b182.968b.39cb.f321.a064.7b10.
- **Our client**: No counterpart by VA.

### `0x42ccb0` — draw a clamped rectangle through the current page

- **VA / file offset**: `0x42ccb0` / `0x2ccb0` (183,472).
- **Behavior**: Requires an active flag and a device object, clamps the rectangle through `0x42afd0`, then calls the page object's virtual `+0x14` with fixed draw flags and a small option array. Gate failures return one.
- **Reached from**: 6 direct caller(s): `0x50bfb0`, `0x50c4b0`, `0x513990`, `0x513c60`, `0x5141c0`, `0x514250`; 6 raw xref site(s).
- **Player-visible reach**: yes — one of the two page-draw entry points, so it puts visible pixels on the page.
- **Evidence**: ev 2f0f.f472.b269.4839.b3bd.fbcb.46c4.4f78.a614.3f9f.e2ee.f981.1d31.d885.da3e.0758; file-offset mapping ev 9aef.99b9.bbb7.cf69.db03.8e3b.8a38.8c1a.f623.b5f7.a366.1bc9.8938.c3b1.49f4.49a0; incoming xrefs ev 4821.93ed.2d95.6360.2e07.b3aa.613b.43d8.d745.b9b3.45cc.afb3.e20f.c04f.28ae.ab9f.
- **Our client**: No VA counterpart. The Rust drawing entry is `paint_native_scene` (`crates/o2jam-render/src/lib.rs:1120`).

### `0x42cd60` — draw an unclamped rectangle through the current page

- **VA / file offset**: `0x42cd60` / `0x2cd60` (183,648).
- **Behavior**: Applies the same active-flag and device gates as `0x42ccb0`, but passes the caller's rectangle straight to the page virtual `+0x14` without clamping.
- **Reached from**: 6 direct caller(s): `0x413030`, `0x413be0`, `0x42fa50`, `0x430230`, `0x4a21a0`, `0x520fc0`; 8 raw xref site(s).
- **Player-visible reach**: yes — the second page-draw entry point; it paints without the clamp step.
- **Evidence**: ev c65e.e5e4.a649.5592.cfba.f192.ba40.99ae.f6ad.8f69.72a5.6ddb.57b7.b914.edcd.7c14; file-offset mapping ev 0206.0327.e86d.5e1a.61a3.0164.8112.1f66.5bc6.ffd7.c4ca.fa98.fde8.7942.5189.23d3; incoming xrefs ev ac3e.d581.b03a.b2e0.5ea4.21e7.b1c1.1b99.9603.8f39.b052.b4a6.98ee.da32.4d3b.8564.
- **Our client**: No VA counterpart. The Rust drawing entry is `paint_native_scene` (`crates/o2jam-render/src/lib.rs:1120`).

### `0x42cef0` — copy a page state snapshot

- **VA / file offset**: `0x42cef0` / `0x2cef0` (184,048).
- **Behavior**: Copies thirty-one 32-bit words from surface `+0x168` to the caller's buffer.
- **Reached from**: 3 direct caller(s): `0x416100`, `0x422040`, `0x42b5e0`; 9 raw xref site(s).
- **Player-visible reach**: support — the image copy path reads this snapshot before it locks the page.
- **Evidence**: ev 057b.e6fa.0dd5.72f9.5443.fee5.d843.0537.c64e.f394.b20a.ff6b.251f.5ff5.cd41.24ff; file-offset mapping ev 5689.15d5.98b1.7656.d460.add9.28b5.170c.484f.7aed.151e.187d.e791.1b56.db37.552a; incoming xrefs ev cbec.a0ad.8ecc.200e.1f30.7c33.e8ca.54d3.6bdb.670d.3c03.aaf3.19a5.1ec3.5315.80ad.
- **Our client**: No counterpart by VA.

### `0x42cf20` — return the page color key word

- **VA / file offset**: `0x42cf20` / `0x2cf20` (184,096).
- **Behavior**: Reads the word at `+0x1f0` and returns it.
- **Reached from**: 1 direct caller(s): `0x416100`; 1 raw xref site(s).
- **Player-visible reach**: support — the image row copier uses this value as its transparent color key.
- **Evidence**: ev cb29.8dc3.febf.4882.d223.6a7f.7bbe.576f.d1c6.89b0.aa12.9b36.56b8.a29c.ecf4.ef12; file-offset mapping ev 6c4a.1241.0e9f.7207.835c.87fc.8277.9889.2d45.de28.bdbb.ecf8.640d.ca44.366a.7b5a; incoming xrefs ev dbf9.ff20.c657.b40d.8d48.44d0.ec35.d6f4.d535.830c.949f.e91d.773b.a6b6.7081.00c2.
- **Our client**: No VA counterpart. Color keys are authoring data in Rust (`crates/o2jam-render/src/lib.rs:3480`, guard `anchor_blit_compares_colorkey_after_decode`).

### `0x42d020` — compute the current page descriptor address

- **VA / file offset**: `0x42d020` / `0x2d020` (184,352).
- **Behavior**: Returns the array base at `+0x1e4` plus the current index at `+0x134` times sixteen.
- **Reached from**: 8 direct caller(s): `0x42dbf0`, `0x42ebc0`, `0x4dbf60`, `0x4e9b70`, `0x4ef920`, `0x4f20c0`, …; 10 raw xref site(s).
- **Player-visible reach**: support — page bounds and blits all derive their page from this address.
- **Evidence**: ev 7c01.cbae.d5ef.7097.86f1.98a6.721f.5a10.2353.a776.fa1c.9a43.ac1b.637d.4d4c.904f; file-offset mapping ev ab2f.d005.0fad.8906.239b.2675.088c.5f30.8bb7.654c.3f9f.a323.1df1.8352.0370.ca46; incoming xrefs ev c692.427c.9a31.7e63.4c72.e965.ffe5.3578.8867.b804.e954.ac7b.d136.d9d3.14e5.5a8c.
- **Our client**: No counterpart by VA; the client has no page descriptor table.

### `0x42d040` — return the current page object

- **VA / file offset**: `0x42d040` / `0x2d040` (184,384).
- **Behavior**: Reads the pointer stored in the current page descriptor and returns it.
- **Reached from**: 22 direct caller(s): `0x413030`, `0x413be0`, `0x414bc0`, `0x4153b0`, `0x4156d0`, `0x415730`, …; 44 raw xref site(s).
- **Player-visible reach**: support — 44 xref sites span UI controls, stage blits and cleanup; this is the shared way to reach the active page.
- **Evidence**: ev d3be.14ae.06af.17be.e6f8.b97d.9718.ec36.3186.3302.461d.663b.4124.e7db.0d27.4e44; file-offset mapping ev 691d.262d.9412.9c95.4476.6e93.8f46.7817.1e79.fe1d.5206.ca4b.9cca.dea2.115c.7927; incoming xrefs ev a0f2.03bb.1dd8.cab0.018e.2d43.c68d.a6bd.fbbb.e4dd.1a33.604b.9ce9.3855.5497.429e.
- **Our client**: No counterpart by VA; there is no COM page object in the Rust renderer.

## Image animation state (12)

### `0x42eaa0` — store a twelve-byte image geometry triple

- **VA / file offset**: `0x42eaa0` / `0x2eaa0` (191,136).
- **Behavior**: Copies the supplied three-word geometry value into the image object at `+0x44`.
- **Reached from**: 2 direct caller(s): `0x42dbf0`, `0x42ebc0`; 2 raw xref site(s).
- **Player-visible reach**: yes — the stored triple is the view geometry read by the image draw path.
- **Evidence**: ev a2d0.a564.d279.9f0f.c2c0.163e.44b2.ce60.2c50.1582.aab7.054b.258a.357f.bf5d.6c3a; file-offset mapping ev 4609.c68f.3beb.4e65.5151.cc83.c6bc.54e8.6d65.b502.c711.b8b5.520e.2566.4550.1ec2; incoming xrefs ev 4600.477c.ba68.7bc3.0de8.b918.7987.4c20.f019.1dc5.1fe2.15a9.70ba.add6.c6b9.cdd6.
- **Our client**: No VA counterpart. Staged placement geometry is carried by `ScenePlacement` (`crates/o2jam-shell/src/staging.rs:1313`).

### `0x42eb80` — return the image source word

- **VA / file offset**: `0x42eb80` / `0x2eb80` (191,360).
- **Behavior**: Reads and returns the word at `+0x28`.
- **Reached from**: 2 direct caller(s): `0x42fcd0`, `0x42fe40`; 2 raw xref site(s).
- **Player-visible reach**: support — `0x42fe40` selects this value as the draw source for image kind three.
- **Evidence**: ev ec31.ff96.a953.6b72.47f3.79cd.6f5e.d5eb.0ef6.71b5.d465.5f44.24fc.b564.23cc.187f; file-offset mapping ev e140.b435.0e95.a68d.113a.b908.dfd7.9323.8cc8.5c89.ea47.fef4.61d2.eea6.b915.fe9a; incoming xrefs ev 2acf.d2b4.717f.8992.4493.fad5.18a8.65fe.d7a6.a93e.8394.54ad.fd97.b778.fa9c.e7e2.
- **Our client**: No counterpart by VA.

### `0x42eba0` — store the image mode word

- **VA / file offset**: `0x42eba0` / `0x2eba0` (191,392).
- **Behavior**: Writes the supplied value at `+0x54`, the same flag tested by `0x4160e0` and `0x415730`.
- **Reached from**: 13 direct caller(s): `0x42f040`, `0x42f4f0`, `0x42f810`, `0x42fff0`, `0x4dbf60`, `0x4e0bf0`, …; 35 raw xref site(s).
- **Player-visible reach**: yes — the value selects which control draw/dispatch branch a staged image takes.
- **Evidence**: ev ecd7.c85c.1d0e.7f0a.3c06.cae3.0d74.62f1.de57.a716.4a11.46e0.c914.3f3d.92f1.6e77; file-offset mapping ev 72c5.25f4.6cd4.9b86.0c46.7bda.7bb0.29fb.416a.212a.1586.8832.3ef2.839d.4ac8.8ec8; incoming xrefs ev 7100.f22a.b9ce.dec6.124d.f978.0f66.3c2c.b32d.58ff.5808.0ad0.43e3.44a5.3879.7723.
- **Our client**: No VA counterpart; image kind in the client comes from parsed body type (`crates/o2jam-file/src/sprites.rs:52`).

### `0x42ebc0` — build a two-float image transform

- **VA / file offset**: `0x42ebc0` / `0x2ebc0` (191,424).
- **Behavior**: Reads the current frame record and the current page size, then stores two computed floats at `+0x44` from the frame origin, the input offsets, half the leftover page space and a fixed four-hundred-unit scale.
- **Reached from**: 1 direct caller(s): `0x42fff0`; 1 raw xref site(s).
- **Player-visible reach**: yes — the float pair is the geometry used when kind-three images are drawn.
- **Evidence**: ev 078e.67c3.cae0.dd7e.4698.d764.2cbc.b720.ca54.4e5f.aa9e.1c08.e64f.5e21.089c.0d01; file-offset mapping ev 4ab4.6b87.3925.fa01.0930.fcef.05c7.5136.67e5.8f9f.c8ab.35c9.e338.f9f6.a09e.4120; incoming xrefs ev 3d91.2448.a7c3.153d.7476.70ad.3f27.5d58.bdbd.9837.9db3.7b2f.1a4b.d95e.8af3.06e7.
- **Our client**: No VA counterpart. Anchored screen placement is pinned by `anchor_blit_lands_byte_exact_on_matching_rect` (`crates/o2jam-render/src/lib.rs:3331`).

### `0x42f560` — enable or disable image drawing

- **VA / file offset**: `0x42f560` / `0x2f560` (193,888).
- **Behavior**: Writes the supplied word at `+0x160`, the flag that gates the call to the image draw helper.
- **Reached from**: 40 direct caller(s): `0x418800`, `0x438080`, `0x4383d0`, `0x4392d0`, `0x439b00`, `0x43a5f0`, …; 76 raw xref site(s).
- **Player-visible reach**: yes — 40 direct callers in control factories and scene code use it to turn staged art on or off.
- **Evidence**: ev 2482.1a7f.f049.2665.162a.7a4e.dcb9.4efa.81d5.3401.eaef.9502.eb78.91d4.b711.e0cf; file-offset mapping ev 5195.10e4.f235.def1.da8a.dcaa.2c6e.902d.9bf5.7b62.0385.a729.db99.24c7.dd89.8552; incoming xrefs ev db55.f0a0.11b1.33f1.d508.ff4e.d9ab.3b38.9735.bd4a.dc53.f73b.2457.132d.8b7e.8a1e.
- **Our client**: No VA counterpart. The client's nearest flag is `SceneArt::has_figures` (`crates/o2jam-shell/src/screens/paint.rs:123`).

### `0x42f580` — return the image draw flag

- **VA / file offset**: `0x42f580` / `0x2f580` (193,920).
- **Behavior**: Reads and returns the word at `+0x160`.
- **Reached from**: 6 direct caller(s): `0x417570`, `0x50bf40`, `0x50cdd0`, `0x50ddf0`, `0x50dea0`, `0x514340`; 7 raw xref site(s).
- **Player-visible reach**: support — getter paired with `0x42f560`.
- **Evidence**: ev 7e27.ae4b.30c4.b8fa.8ccb.2261.b519.0fa6.5266.0818.d97a.a895.1eea.2cdb.c36c.2e46; file-offset mapping ev cbb6.a868.8245.d130.9458.f148.9341.993a.f102.9ac8.9795.a16a.1794.3d1f.8142.b4ef; incoming xrefs ev 8479.0554.93a1.18c4.9b9b.c82a.5e82.9292.280c.0ac2.ac6b.ffbd.c5ee.cb3d.3d61.85f4.
- **Our client**: No VA counterpart.

### `0x42f760` — set the image frame count

- **VA / file offset**: `0x42f760` / `0x2f760` (194,400).
- **Behavior**: When the argument is nonzero it stores it as the frame count word at `+0x148`; staging calls it with the source image's count.
- **Reached from**: 5 direct caller(s): `0x42fb30`, `0x42fba0`, `0x42fc10`, `0x42fc70`, `0x42fcd0`; 5 raw xref site(s).
- **Player-visible reach**: yes — the count bounds frame advance and rest-frame selection.
- **Evidence**: ev 4a8d.c42b.a452.0037.07c8.7925.917a.faa0.5e27.e0fa.b29d.4fa1.c807.9027.3acd.f5d8; file-offset mapping ev b55f.29a3.a578.25ef.ddbf.81c9.efee.4f1d.a398.c53f.d87e.836c.41aa.7d24.887c.f86c; incoming xrefs ev c698.bb3a.24b9.1b13.ad58.0a06.1d4c.b00f.4ea9.4a0c.47d6.2fde.1321.b399.37a9.d596.
- **Our client**: No VA counterpart. Frame counts are decoded in `crates/o2jam-file/src/sprites.rs:205`.

### `0x42f780` — return the image frame count

- **VA / file offset**: `0x42f780` / `0x2f780` (194,432).
- **Behavior**: Reads and returns the frame count word at `+0x148`.
- **Reached from**: 11 direct caller(s): `0x48dd10`, `0x4bb2c0`, `0x4bb570`, `0x4dea20`, `0x4dec70`, `0x4deec0`, …; 14 raw xref site(s).
- **Player-visible reach**: support — paired getter used by animation and frame-step code.
- **Evidence**: ev c792.14fb.815c.d782.7d47.a9a5.f65e.179d.b8d0.5f16.e768.155c.d8f2.b672.4896.56d6; file-offset mapping ev 06bb.2cce.23e9.d022.71a7.d04c.c1d4.1c26.c1e9.1004.5c43.2e05.b101.ca1b.b3c1.289b; incoming xrefs ev dbbb.b833.f70a.aedf.e50a.3f47.2915.1e88.206a.a947.c720.3749.2b97.d102.44e8.01a1.
- **Our client**: No VA counterpart.

### `0x42f7c0` — clear the animation tick flag and prime special modes

- **VA / file offset**: `0x42f7c0` / `0x2f7c0` (194,496).
- **Behavior**: Clears the tick-flag word at `+0x15c`. For animation modes greater than four and below eight it initializes the current frame word to one past the frame count.
- **Reached from**: 4 direct caller(s): `0x4f6a30`, `0x4f6e20`, `0x4f6fd0`, `0x50e8d0`; 8 raw xref site(s).
- **Player-visible reach**: yes — it decides when a stepped animation restarts from its sentinel frame.
- **Evidence**: ev 5544.27b5.9007.816a.5749.5fae.4048.79f7.e350.c9c7.d02c.3f99.5e6b.70cb.ddf4.51f3; file-offset mapping ev 69ae.4ccd.e2c1.92e1.c07b.57d8.46e1.9ad9.4204.f242.1830.8189.6f75.1bb8.3848.66ac; incoming xrefs ev 9f3d.7055.aaf3.8308.19b3.405f.cbe0.fb6b.4cb7.c584.ed98.83c0.e5f5.5dc2.ca2f.38c0.
- **Our client**: No VA counterpart. Rust frame stepping is `stepped_frame` (`crates/o2jam-shell/src/playing.rs:1270`) with guard `press_flash_steps_the_kit_beam_frames` (`:5551`).

### `0x42f910` — set the rest frame

- **VA / file offset**: `0x42f910` / `0x2f910` (194,832).
- **Behavior**: Stores the supplied frame at `+0x168`; the all-ones input instead stores the current frame count word.
- **Reached from**: 6 direct caller(s): `0x4a0360`, `0x4e0b00`, `0x4f60c0`, `0x4f6a30`, `0x4f6e20`, `0x51e990`; 8 raw xref site(s).
- **Player-visible reach**: yes — the rest frame is the clamp floor used after a stepped animation ends.
- **Evidence**: ev 59ec.a87b.8214.c8a2.afaf.924c.0916.2f4d.5bdc.ae56.6e9f.b2f5.faa9.4709.0049.4d6a; file-offset mapping ev 5568.2fc5.211f.16e2.fd0f.b2e8.aab1.fa15.294f.46e7.4205.eb75.9150.6feb.ad75.303f; incoming xrefs ev f7e8.d30d.7be0.abe2.a592.a91b.e800.648e.7592.b399.8722.9f0f.dc6b.80f0.bb89.2778.
- **Our client**: No VA counterpart. Rust flash frames clamp to the first/last staged frame (`crates/o2jam-shell/src/playing.rs:926`).

### `0x42f990` — return the current frame

- **VA / file offset**: `0x42f990` / `0x2f990` (194,960).
- **Behavior**: Reads and returns the frame word at `+0x164`.
- **Reached from**: 4 direct caller(s): `0x417770`, `0x4f6a30`, `0x4f6e20`, `0x4f7130`; 9 raw xref site(s).
- **Player-visible reach**: yes — `0x417770` uses it to pick the image frame that a Playing skin copies.
- **Evidence**: ev 9f3e.cfb7.3a43.06ba.e654.272a.6745.986e.8589.df58.3ef1.b5e2.eb19.d884.3435.0fb7; file-offset mapping ev cfb5.d60b.9b35.8568.bbc0.1f02.46b3.86e4.f6c8.d988.7d4b.d41a.5629.251d.f1ba.df41; incoming xrefs ev c101.eb4d.a9f8.836f.8d67.94da.f428.2198.425a.e767.3b38.fe96.1535.211f.2277.485e.
- **Our client**: No VA counterpart. Guard `staged_burst_animates_across_flash_frames` (`crates/o2jam-shell/src/playing.rs:5484`) covers the Rust frame clock.

### `0x42f9b0` — step the current frame and prepare its draw

- **VA / file offset**: `0x42f9b0` / `0x2f9b0` (194,992).
- **Behavior**: When a frame count is present it adds the supplied delta, clamps the result between the rest frame and that count, computes the draw transform, sets the image frame, and returns the new frame.
- **Reached from**: 2 direct caller(s): `0x4f6a30`, `0x4f6e20`; 6 raw xref site(s).
- **Player-visible reach**: yes — this is the manual frame-advance body used by Playing scene code, so its clamp decides which sprite frame is visible.
- **Evidence**: ev 8fe1.8a0d.faf7.a707.e70c.680c.b572.f4d9.d7f8.476c.138c.5f98.9a1c.b374.96a7.c186; file-offset mapping ev f018.64c9.64c0.3d06.bf90.f5d5.e219.8ff3.0ea6.7bd8.934e.b658.0ade.94cf.5eec.431d; incoming xrefs ev 2037.c69b.458f.e489.f512.a07c.c52d.7722.e591.1811.9827.4658.e2e0.f1c5.b64b.1eb2.
- **Our client**: No VA counterpart. Equivalent stepping lives in `stepped_frame` (`crates/o2jam-shell/src/playing.rs:1270`); guards `receptor_animates_across_flash_frames` (`:5917`) and `staged_burst_animates_across_flash_frames` (`:5484`).
