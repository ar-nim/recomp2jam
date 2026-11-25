# Static pins — UI draw and control support (ui-draw-2)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256 `83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`. Provider: REA 6.10.1 / Ghidra 12.2, shared target bound from `install-root/O2Jam/OTwo.exe`. Evidence IDs are `ev ` plus the REA hex in four-digit groups; joining the groups after `ev_` gives the REA ID. Grouping keeps `scripts/check-cited-hashes.sh` from reading evidence hex as a commit citation.

Branch `cb2/pins-ui-draw-2`; worktree pre-flight `/home/arnim/Projects/recomp2jam-lane-pins-ui-draw-2`. The brief's base `a15244e` is an ancestor of branch HEAD `a42ca1e`; other wave-1 pin lanes merged in between. No reset was attempted.

Scope: the 56 addresses in `slice-ui-draw-2.txt`, bucket `game-reachable`, `pinned_in_repo=no`. **Pinned here: 56/56; left: 0.** File offsets come from REA `address_to_file_offset` and equal VA minus the `0x400000` image base for every row (0 mismatches). Behavior wording is ours; no decompiled text is reproduced. A static pin is not a player-visible close: census rows stay OPEN until a live caller chain from boot is recorded (`docs/live-path-reachability.md`). 'Reached from' names the provider's direct-caller edge, not a runtime press or frame.

## Inventory

| VA | File offset | Role |
|---|---|---|
| `0x42fb30` | `0x2fb30` | support |
| `0x42feb0` | `0x2feb0` | support |
| `0x42ff60` | `0x2ff60` | support |
| `0x4300f0` | `0x300f0` | support |
| `0x430130` | `0x30130` | visible |
| `0x431dc0` | `0x31dc0` | support |
| `0x431f70` | `0x31f70` | visible |
| `0x432020` | `0x32020` | visible |
| `0x4321b0` | `0x321b0` | visible |
| `0x4321f0` | `0x321f0` | support |
| `0x434060` | `0x34060` | support |
| `0x48c970` | `0x8c970` | visible |
| `0x495a80` | `0x95a80` | visible |
| `0x495c10` | `0x95c10` | visible |
| `0x4c56a0` | `0xc56a0` | visible |
| `0x4f7300` | `0xf7300` | support |
| `0x505130` | `0x105130` | support |
| `0x507510` | `0x107510` | support |
| `0x509df0` | `0x109df0` | support |
| `0x50af90` | `0x10af90` | support |
| `0x50b130` | `0x10b130` | support |
| `0x50bb70` | `0x10bb70` | support |
| `0x50ca60` | `0x10ca60` | support |
| `0x50ce50` | `0x10ce50` | visible |
| `0x50d1d0` | `0x10d1d0` | support |
| `0x50d980` | `0x10d980` | support |
| `0x50ddb0` | `0x10ddb0` | support |
| `0x50e240` | `0x10e240` | support |
| `0x50edb0` | `0x10edb0` | visible |
| `0x50f7f0` | `0x10f7f0` | support |
| `0x512390` | `0x112390` | support |
| `0x5146b0` | `0x1146b0` | support |
| `0x514800` | `0x114800` | support |
| `0x514810` | `0x114810` | support |
| `0x514990` | `0x114990` | visible |
| `0x517d10` | `0x117d10` | support |
| `0x517dd0` | `0x117dd0` | visible |
| `0x517f00` | `0x117f00` | visible |
| `0x518fa0` | `0x118fa0` | visible |
| `0x5195c0` | `0x1195c0` | support |
| `0x519640` | `0x119640` | visible |
| `0x519aa0` | `0x119aa0` | support |
| `0x519fe0` | `0x119fe0` | visible |
| `0x51a080` | `0x11a080` | visible |
| `0x51a4f0` | `0x11a4f0` | visible |
| `0x51c480` | `0x11c480` | support |
| `0x51c869` | `0x11c869` | support |
| `0x51ca3f` | `0x11ca3f` | support |
| `0x51cc10` | `0x11cc10` | support |
| `0x51d390` | `0x11d390` | support |
| `0x51d550` | `0x11d550` | support |
| `0x51d620` | `0x11d620` | support |
| `0x51d740` | `0x11d740` | support |
| `0x51d880` | `0x11d880` | support |
| `0x51e290` | `0x11e290` | support |
| `0x51e334` | `0x11e334` | support |

## Pins

### `0x42fb30` — initialize the cursor sprite object

- **VA / file offset**: `0x42fb30` / `0x2fb30`.
- **Behavior**: Stores four supplied words and one handle in the cursor object, sets its mode word to 1, routes the mode through the mode setter, and registers a sprite handle through the shared image helpers.
- **Reached from**: `0x432090`, `0x48bfd0`, `0x50b320`, `0x5138e0`.
- **Direct callees**: `0x417900`, `0x41e140`, `0x42f760`, `0x430130`.
- **Player-visible reach**: Support. It builds the pointer object; the mode setter and cursor hub make the visible choice.
- **Our client**: No counterpart by VA. Pointer motion is tracked in `crates/o2jam-shell/src/window.rs:262` and `:554`; staged text cursors paint in `crates/o2jam-shell/src/screens/paint.rs:4332`.
- **Evidence**: analysis ev aafb.defe.b769.5ac2.3713.08ad.d4f8.b4b1.9c6f.b437.863f.5210.c124.33b0.a8f9.f0f1; callers ev bbae.5cb1.0dc2.83d1.f30d.5956.2195.ff8c.caa6.3dde.952c.b83f.5846.f274.4a46.e847; file-offset ev 1238.89c3.9c99.b5fc.29c4.3210.b96f.da5a.12ed.f0ef.24d4.14c5.2c91.76a4.d2d3.2b0f; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x42feb0` — read the cursor object's frame word

- **VA / file offset**: `0x42feb0` / `0x2feb0`.
- **Behavior**: Returns one stored word from the cursor object and does no other work.
- **Reached from**: `0x417770`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for cursor dispatch; it emits no pixels itself.
- **Our client**: No counterpart by VA. `crates/o2jam-shell/src/window.rs:262` keeps the last pointer sample.
- **Evidence**: analysis ev 5b46.3a4a.dab0.67b6.1f87.ce97.71ce.c978.2d9c.a8d3.6149.1538.71e8.1f40.e05b.e04f; callers ev c23b.e431.2841.1cc6.1a89.8144.aa98.0196.1cd0.b825.bfe0.547c.6dd4.d980.ac51.15f1; file-offset ev fcb5.7677.1b1d.16b9.a6b2.da60.f6ed.8344.bcb5.623c.ad80.8434.6fd0.8c1f.cd72.bf1c; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x42ff60` — copy a stored cursor pair to a caller buffer

- **VA / file offset**: `0x42ff60` / `0x2ff60`.
- **Behavior**: Copies two adjacent words from the object into the caller's two-word output and returns that output pointer.
- **Reached from**: `0x43af70`, `0x4dcab0`, `0x5143b0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for hit-test and cursor dispatch; no pixels or wire.
- **Our client**: No counterpart by VA; native window-to-frame mapping is in `crates/o2jam-shell/src/window.rs:390`.
- **Evidence**: analysis ev a285.713a.2ef8.dd97.dda5.497e.4d7c.d789.3526.f2c0.6c89.6f1e.1a46.0368.32f7.8a2d; callers ev 318f.fe63.0914.3944.f825.9b87.e2a3.9a2b.83ef.7a00.f520.02b7.2de0.b0df.2a9e.6fab; file-offset ev 4eef.c5dd.3a64.e789.4a3b.b688.ca3b.ed96.f945.3d94.a5c5.bb26.353f.2bdc.b15f.04eb; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x4300f0` — read the stored cursor mode word

- **VA / file offset**: `0x4300f0` / `0x300f0`.
- **Behavior**: Returns the mode word at the object's `+0x14` offset.
- **Reached from**: `0x417770`, `0x4dbf60`, `0x4deec0`, `0x4e0bf0`, `0x4e0ff0`, `0x4e21c0`, `0x4e2ec0`, `0x4e30f0`, and 6 more.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for the mode branches in cursor dispatch and scene input handlers.
- **Our client**: No counterpart by VA. `crates/o2jam-shell/src/window.rs:554` routes pointer events without a mode word.
- **Evidence**: analysis ev a4de.6e13.0cc3.7ac8.92cc.1565.4e3f.5d52.b350.fecb.a48a.383b.7702.b15f.a9bc.86f9; callers ev 0f0b.deae.8f6d.632c.eb38.7902.7970.eaf3.19d4.62bd.2163.8b6f.f2e7.2437.f429.4f45; file-offset ev ace0.9314.173f.c345.1796.1f34.3d15.da0b.b40b.2e7c.843d.c406.9d0e.c182.cb1a.8c32; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x430130` — switch the cursor or image mode

- **VA / file offset**: `0x430130` / `0x30130`.
- **Behavior**: Writes the requested mode word. Mode 2 calls one vftable method; mode 1 may be promoted to 2 by a global mode query before the other vftable method runs.
- **Reached from**: `0x42fb30`, `0x42fba0`, `0x42fc10`, `0x42fc70`, `0x42fcd0`, `0x42fd70`, `0x4a2f60`, `0x4a9890`, and 10 more.
- **Direct callees**: `0x424170`.
- **Player-visible reach**: Player-visible through the image or cursor face chosen by the mode dispatch.
- **Our client**: No counterpart by VA. Our pointer stays an OS cursor; staged text cursor at `crates/o2jam-shell/src/screens/paint.rs:4332` has no mode switch.
- **Evidence**: analysis ev 44c4.8965.be3e.1838.770e.f9c9.4fc3.ee62.d259.099e.c792.b603.10fa.12c3.a323.2005; callers ev 2a2a.5e09.b72a.74c4.153b.1065.3044.5e4e.5f91.0725.c1df.93ac.46cb.139b.04f9.8938; file-offset ev 9a58.1867.5d8b.ff83.258d.d7ee.951b.7810.913e.8bb0.c5d4.4716.11ed.7f85.0f01.9d90; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x431dc0` — release the stored cursor object

- **VA / file offset**: `0x431dc0` / `0x31dc0`.
- **Behavior**: Calls the object's first vftable method, then its destroy method, and clears the stored pointer.
- **Reached from**: `0x431d90`, `0x5a4b70`, `0x5a4b90`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for scene teardown; no pixels or wire.
- **Our client**: No counterpart by VA; Rust drop handles object release.
- **Evidence**: analysis ev 81a6.9655.2e88.4935.b060.a952.0e53.fb40.1604.9dbf.ed37.8b06.9c01.ac76.676f.f984; callers ev d682.ca3d.813f.9cee.e5b3.2b7b.290e.a393.29f1.7efb.2def.c27f.239c.9b1a.0161.0af0; file-offset ev 5d16.9414.6c4c.d000.e8f7.df52.a142.6260.4e00.a991.c708.f04f.949f.c80a.a06e.f963; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x431f70` — apply the custom-cursor arm and hide the OS cursor

- **VA / file offset**: `0x431f70` / `0x31f70`.
- **Behavior**: With the custom arm off it hides the cursor, samples the pointer position, and forwards that position. With the arm on it selects one of two stored cursor handles, or hides the cursor for other values.
- **Reached from**: `0x5a4ac0`, `0x5a5040`.
- **Direct callees**: `EXTERNAL:0x91`, `EXTERNAL:0x92`, `0x42f870`, `0x432020`, `0x50dd90`.
- **Player-visible reach**: Player-visible: the pointer disappears or changes face as the option and mode change.
- **Our client**: No counterpart by VA. `crates/o2jam-shell/src/window.rs:554` keeps the platform cursor; no `Window_Cursor.ojs` handle is installed.
- **Evidence**: analysis ev 043f.22b5.b78a.862d.06e8.e530.ceb0.0f8f.14d6.4a05.5405.4fe5.1ecf.5d71.bcb5.f6f7; callers ev fac8.f28d.38a4.1471.3994.5534.10ae.2b8e.f828.52f1.d517.5460.6bd2.74f2.079a.bf10; file-offset ev 1424.25a6.4797.5272.c7cc.f31a.bb7b.1a2e.262c.6d58.3e30.50e4.4052.ffad.6335.ba4b; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x432020` — forward the sampled pointer position to cursor tracking

- **VA / file offset**: `0x432020` / `0x32020`.
- **Behavior**: When the custom arm is off and no tracking object exists it creates one; it then sends the pointer coordinates to the shared cursor tracker and to the stored image helper.
- **Reached from**: `0x431f70`, `0x5a4af0`.
- **Direct callees**: `0x42ff10`, `0x432090`, `0x50dd90`.
- **Player-visible reach**: Player-visible through hit-test and cursor-face selection; no direct paint call here.
- **Our client**: No counterpart by VA. Pointer sampling is in `crates/o2jam-shell/src/window.rs:262`; hit tests are in `crates/o2jam-shell/src/window.rs:236`.
- **Evidence**: analysis ev 858c.43b2.e006.6a78.4f56.4014.ffcf.6db8.7247.10ad.04f3.01ca.7665.f420.72d6.8347; callers ev ae35.e2ba.f754.f75b.5e56.5015.2eb1.f996.b121.978f.2048.814a.be1b.4083.a705.0037; file-offset ev 90a2.9d6f.46b9.4a0d.8f14.6672.ca93.7fd2.173d.3810.55bd.849e.b8c7.7e5f.d3d4.6e88; body batch ev 17fd.de85.8b73.9050.95a9.00a8.56c6.45c6.d917.f0ce.b4e4.140f.b5ef.cbb7.0b4e.cd30.

### `0x4321b0` — store the custom-cursor flag and reset the pointer

- **VA / file offset**: `0x4321b0` / `0x321b0`.
- **Behavior**: Writes the custom-cursor flag. When both the flag and a tracking object are present it resets the pointer to the origin and starts the cursor update helpers.
- **Reached from**: `0x5a4b10`.
- **Direct callees**: `0x42f870`, `0x50dd90`.
- **Player-visible reach**: Player-visible: the pointer mode changes and the pointer snaps to the reset position.
- **Our client**: No counterpart by VA; no custom cursor state exists in `crates/o2jam-shell/src/window.rs`.
- **Evidence**: analysis ev 1b9a.9915.7588.6522.a268.d05d.948a.71ca.0efc.ee26.99ea.fcb5.5968.49a9.dbcf.6e26; callers ev f262.f9d1.d38d.4691.838e.d739.131b.d6cf.56bb.b15a.a807.316e.81da.baa4.8f7b.a3a4; file-offset ev 07f7.0138.de1b.7733.de94.67b9.c5bb.fb09.cf0d.5b3d.cc99.8463.02af.ac56.073d.ced6; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x4321f0` — read the custom-cursor flag

- **VA / file offset**: `0x4321f0` / `0x321f0`.
- **Behavior**: Returns the stored custom-cursor flag word.
- **Reached from**: `0x5a4b50`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for the main window hub that calls the cursor setter.
- **Our client**: No counterpart by VA.
- **Evidence**: analysis ev 081f.7356.562d.143b.6a90.867a.ed9e.efe5.49a0.957f.cde9.3152.2f3d.c0e6.75f6.cf8e; callers ev 8b5e.dd81.472e.a321.68eb.21d1.b14d.8bfd.9031.4cd9.8006.122b.b489.7d2b.f4ef.1b05; file-offset ev 9b1a.dd83.931a.8c82.bad4.badc.0bbd.c3d4.d8c6.9361.216d.b73f.ae2a.39aa.a148.de12; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x434060` — store a scene-state word

- **VA / file offset**: `0x434060` / `0x34060`.
- **Behavior**: Writes one supplied word into the object's `+0x184` field.
- **Reached from**: `0x477990`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support. The caller sets it while rebuilding a scene record list; it paints nothing itself.
- **Our client**: No VA counterpart; related list-top state lives in `crates/o2jam-shell/src/screens/game/mod.rs:816`.
- **Evidence**: analysis ev 378e.2efb.1a71.0232.473e.8822.6842.a7f2.00ea.4486.a17d.ae3f.d8aa.392b.e6c6.1492; callers ev 812a.b667.5e57.04cc.a938.e87c.5213.c985.068e.74cd.a614.64aa.34f2.8001.7c7d.e997; file-offset ev 7051.3caa.3063.eefc.52c8.c241.97bd.245d.27ab.6196.afea.33ad.d8f2.082b.df52.17c4; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x48c970` — measure text to fit a labelled cell

- **VA / file offset**: `0x48c970` / `0x8c970`.
- **Behavior**: Measures the byte span with the shared device context. A null string measures zero; a zero length uses the string length. The result is the pixel width used by callers to truncate or keep a caption.
- **Reached from**: `0x491af0`, `0x4a0660`, `0x4a7a80`, `0x4c3b40`.
- **Direct callees**: `EXTERNAL:0x49`, `0x415d00`, `0x42c0a0`, `0x42c160`, `0x5f1bc0`.
- **Player-visible reach**: Player-visible: labels such as the level-and-name row shrink or wrap to the measured width.
- **Our client**: No counterpart by VA. Rust measurement is `crates/o2jam-render/src/font.rs:363`; room rows use `crates/o2jam-shell/src/screens/paint/room.rs:164`.
- **Evidence**: analysis ev 2490.1c59.cb76.ac72.0966.e768.ee0a.adfa.5bcc.5fcf.e720.1093.3cac.0f40.a0ef.f8eb; callers ev c833.2a14.ab36.8727.b4a9.0652.858b.f9b0.90f0.c8e3.186f.7271.6e72.9d36.c18c.5aa6; file-offset ev 8cd9.0724.bb7c.65d7.851d.1b97.eb95.5b36.8d18.b34b.5539.0def.e545.2c46.6d27.3add; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x495a80` — write a room-row cell from stored row data

- **VA / file offset**: `0x495a80` / `0x95a80`.
- **Behavior**: Resolves a control id, reads its stored value, and writes that value into the row's text helper. It then advances the shared cursor. It returns early when the control or session is absent.
- **Reached from**: `0x4925a0`.
- **Direct callees**: `0x42f870`, `0x42fc70`, `0x48bcf0`, `0x50dc60`, `0x50dd90`.
- **Player-visible reach**: Player-visible: room rows show the stored badge or name cell.
- **Our client**: No counterpart by VA. Row cells paint from staged records in `crates/o2jam-shell/src/screens/paint/room.rs:119` and `:633`.
- **Evidence**: analysis ev 27d4.8d7c.cde0.1cc4.0304.15df.d4e6.4e62.bd13.241c.74eb.68c3.8572.b8a2.4c78.b73a; callers ev 8880.96d3.9ccd.9e80.4ede.81e7.ebb7.7940.534e.0622.1919.66a0.35ab.5828.64e5.268d; file-offset ev c57a.a0c1.3a48.5de8.eb4b.32b1.8141.cd0e.572d.96e5.5339.9dec.a4c3.362b.28ac.3e2d; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x495c10` — switch two room control captions with a mode word

- **VA / file offset**: `0x495c10` / `0x95c10`.
- **Behavior**: Stores the mode word, then visits two control ids. When the mode is zero it clears them; otherwise it writes a default value to a control that still has no value.
- **Reached from**: `0x48fd20`, `0x597df0`, `0x5980b0`, `0x598870`, `0x599130`, `0x599780`, `0x59a280`.
- **Direct callees**: `0x42f400`, `0x42f4f0`, `0x48bcf0`, `0x50dd90`.
- **Player-visible reach**: Player-visible: the two room captions change with the mode sent to the dispatcher.
- **Our client**: No counterpart by VA. Mode and key captions are staged at `crates/o2jam-shell/src/screens/paint/room.rs:491` and `:519`.
- **Evidence**: analysis ev 7426.1dcb.6773.a582.f2e6.f776.fe4a.f4f7.bcb1.fa69.2089.1e72.88f3.bba2.9306.325f; callers ev 8b92.9088.76cf.41b1.ef5d.6296.e01e.02bf.1abb.2b85.7be5.afde.e3da.9f67.39e7.618f; file-offset ev 0108.0730.c081.29a1.aafb.a9b9.22eb.13c5.cde9.cf44.4b2e.fb23.195e.ba9f.7ea3.095c; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x4c56a0` — position and paint one computed badge

- **VA / file offset**: `0x4c56a0` / `0xc56a0`.
- **Behavior**: Resolves a control id, writes a supplied value into the shared cursor position word, and triggers the badge draw helper.
- **Reached from**: `0x4c1b90`, `0x4c38a0`.
- **Direct callees**: `0x42f560`, `0x42f870`, `0x48bcf0`, `0x50dd90`.
- **Player-visible reach**: Player-visible: the badge drawn after the stat counters moves with the supplied value.
- **Our client**: No counterpart by VA. Staged badge/counter painting is in `crates/o2jam-shell/src/screens/paint/result.rs` and `crates/o2jam-shell/src/screens/paint.rs:2995`.
- **Evidence**: analysis ev 08cc.66b7.116b.0d5c.99da.505a.56c8.9806.63ba.276c.190c.51a5.5ffd.b422.54e3.8e3c; callers ev e674.ae12.75de.c135.386f.49fc.d2f6.a346.e21f.bd3d.d565.6340.b843.7acb.a59c.8192; file-offset ev 1b98.0b05.891f.aca6.42b6.a57c.7472.2e3b.0f4e.36b0.24a1.9f6d.064a.b431.771d.950a; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x4f7300` — construct an edit box

- **VA / file offset**: `0x4f7300` / `0xf7300`.
- **Behavior**: Calls the shared control initializer and installs the edit-box vftable.
- **Reached from**: `0x5146b0`.
- **Direct callees**: `0x50d980`.
- **Player-visible reach**: Support. It creates the control but writes no caption or wire itself.
- **Our client**: No counterpart by VA. Chat drafts live in `crates/o2jam-shell/src/screens/game/chat.rs:142`.
- **Evidence**: analysis ev 193b.88f0.739e.08d7.948b.071a.c4de.f1f4.bd97.a891.2c50.38d5.cda6.abfd.893b.7fc4; callers ev 9cea.d628.e2cf.1c0e.ebad.4ef6.f137.6193.36ef.3ce4.fc07.c034.8e8e.fa53.8266.47c7; file-offset ev b4bb.c424.7404.29c4.1075.d476.9b9d.2b0f.1161.6729.aa95.d3fd.b572.e5b9.db10.1c58; body batch ev 0763.ba97.51d4.7350.64f2.83a0.9b86.01eb.3fb0.aaf9.de03.768e.d76b.5e99.4556.366a.

### `0x505130` — poll a worker thread and close a finished handle

- **VA / file offset**: `0x505130` / `0x105130`.
- **Behavior**: When the handle word is unset it queries the thread exit code; a finished worker is closed, its word becomes the sentinel, and a cleanup helper runs. It reports whether the worker is still live.
- **Reached from**: `0x48e990`, `0x48f3a0`, `0x4b8d00`, `0x4ba6c0`, `0x4c7b80`.
- **Direct callees**: `EXTERNAL:0x11b`, `EXTERNAL:0x14a`, `0x5049e0`.
- **Player-visible reach**: Support for asynchronous UI work; no pixels or wire.
- **Our client**: No counterpart by VA. Rust threads are joined or dropped with ownership rules.
- **Evidence**: analysis ev 1812.d63c.fc80.8204.9203.7593.4430.f691.6e36.aeea.09fc.3d96.91a4.bec1.e3f6.ceb4; callers ev 0212.3618.7b1d.8bd9.c0d5.463b.209f.093f.c4e5.d437.12a7.bd7a.8914.81cf.200c.74a6; file-offset ev 40e6.1ce7.c791.0587.2582.bfbf.765f.5e38.08a2.1225.38ec.32ea.c7d2.b0f4.a6a4.62f0; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x507510` — fill a byte buffer at an offset

- **VA / file offset**: `0x507510` / `0x107510`.
- **Behavior**: Chooses the inline or heap buffer by a size word, then writes one byte directly for a count of one or calls the byte-fill helper for larger counts.
- **Reached from**: `0x5069c0`, `0x507490`.
- **Direct callees**: `0x5f1ef0`.
- **Player-visible reach**: Support for string storage; no player-visible decision in this body.
- **Our client**: No counterpart by VA; `Vec` slice fill supplies the same role.
- **Evidence**: analysis ev 1f0b.479c.6667.37ba.6604.3418.9c09.6e81.4084.cdb2.b62c.b546.cec7.7c2e.e8a3.e603; callers ev cc2a.4b76.f812.8371.e0fe.9bd0.d830.805f.a676.40c3.153d.5a49.c48b.34a4.f139.d32f; file-offset ev 60a7.99a4.a459.a0bb.4b94.ea3b.8b5e.5b8b.963b.e389.c2ed.2426.5033.977c.9915.38fd; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x509df0` — arm a graphics-state flag

- **VA / file offset**: `0x509df0` / `0x109df0`.
- **Behavior**: Checks two state bits and a query result, combines a flag when needed, and forwards the combined flags to the state helper.
- **Reached from**: `0x509a00`, `0x58d580`, `0x58dbd0`, `0x58dd20`, `0x58de60`.
- **Direct callees**: `0x508df0`.
- **Player-visible reach**: Support for the graphics state used by UI painting; no direct pixels.
- **Our client**: No counterpart by VA; the renderer owns its own state in `crates/o2jam-render/src`.
- **Evidence**: analysis ev 2559.e84e.2ff0.3617.fcc2.c8d8.1645.44be.51f2.60de.f5db.b4cc.6253.5e65.aa65.78bf; callers ev 02e2.9203.afa7.ebd5.5849.02ec.4631.5de0.08fa.1ef8.0437.00fc.b5f1.b8e7.19c5.b8a5; file-offset ev 777b.2b68.8aa3.8fd7.4f6b.e8f7.46bf.cc44.812d.0f81.df57.f4db.edbd.e759.c869.d285; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x50af90` — construct an image box

- **VA / file offset**: `0x50af90` / `0x10af90`.
- **Behavior**: Calls the shared control initializer, installs the image-box vftable, and clears one trailing word.
- **Reached from**: `0x4329b0`, `0x479900`, `0x48a620`, `0x48b2d0`.
- **Direct callees**: `0x50d980`.
- **Player-visible reach**: Support. The staged art painter owns visible images.
- **Our client**: No counterpart by VA. Staged image placement is in `crates/o2jam-shell/src/staging.rs:315`.
- **Evidence**: analysis ev 43a1.3fde.7e9b.f963.8f02.e859.641f.29e8.655e.e94c.71cd.7d16.e08d.251b.853a.0dbb; callers ev 9a7e.8615.340d.3e80.4bfe.e76c.bf20.4442.e427.162c.eacc.fa45.aecd.e219.94d1.6570; file-offset ev e1cb.7518.2ea6.5c02.b2fe.2143.fc69.435f.66f9.a365.f2b7.05de.1653.b695.0443.91a2; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x50b130` — construct a lightboard

- **VA / file offset**: `0x50b130` / `0x10b130`.
- **Behavior**: Calls the shared control initializer, installs the lightboard vftable, clears several words, sets two default inks and a default offset, then stores a computed word.
- **Reached from**: `0x48e700`, `0x497f60`, `0x4a3560`, `0x4b5d10`, `0x4c6760`.
- **Direct callees**: `0x403db0`, `0x403f90`, `0x411e60`.
- **Player-visible reach**: Support. It initializes the board; the painter draws its entries.
- **Our client**: No counterpart by VA. Lightboard entries stage in `crates/o2jam-shell/src/screens/paint.rs`.
- **Evidence**: analysis ev 93b7.0e1b.6d8b.6c7e.823c.357a.b3de.284c.a843.c901.78d5.2193.1d94.3e32.1f51.9244; callers ev b955.924a.27bd.dcbc.8469.dd2a.fcab.c8f0.5c19.6e0c.0fe0.f6bd.39bd.2ab7.9a71.8746; file-offset ev ca9d.7381.0797.97e8.f4df.0bc7.53dd.939d.29c8.925b.4a27.161c.ec31.7b1c.511b.e2bd; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x50bb70` — construct a list box

- **VA / file offset**: `0x50bb70` / `0x10bb70`.
- **Behavior**: Calls the shared control initializer and installs the list-box vftable.
- **Reached from**: `0x4329b0`, `0x48a620`, `0x48b2d0`.
- **Direct callees**: `0x50d980`.
- **Player-visible reach**: Support. List rows come from the staged scene data.
- **Our client**: No counterpart by VA. Row lists are staged in `crates/o2jam-shell/src/screens/game/mod.rs`.
- **Evidence**: analysis ev d72d.255b.85bb.3dd5.0054.2175.ca6a.2c80.246c.5bf6.4020.47e3.cc78.ebc8.e5ec.df55; callers ev a2a3.034a.2eac.7809.eb6a.20bf.8a90.3151.bfa4.bb14.21e7.0b4b.bd41.0b9c.37f4.3797; file-offset ev 7da5.3027.eb3a.5afc.9132.c1e7.75fd.9c4b.329c.dfb9.711d.2238.51d3.c01c.58ae.d23d; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x50ca60` — set a list control's maximum count

- **VA / file offset**: `0x50ca60` / `0x10ca60`.
- **Behavior**: Writes the supplied count, or 10000 when zero is supplied, into the control's maximum-count word.
- **Reached from**: `0x4351e0`, `0x438080`, `0x439b00`, `0x43af70`, `0x43d4b0`, `0x43e9b0`, `0x471430`, `0x4944c0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support. The value bounds later list and scroll work; no direct paint here.
- **Our client**: No counterpart by VA. Scrolled list tops are modelled in `crates/o2jam-shell/src/screens/game/mod.rs:816`.
- **Evidence**: analysis ev 8573.b590.9a2d.b7a9.4b44.711d.0744.4c08.b05c.2546.e79e.d115.2935.a38a.c25d.42db; callers ev 0591.d4e2.7c4e.3e43.31d0.9bab.9da2.233b.b42f.8888.2963.4126.7b7b.d76e.71c1.20ee; file-offset ev 5f31.388a.e0ee.3d94.12f4.12d6.1980.d585.b2da.57ba.6bfa.c9c7.1c63.1346.2ce4.c010; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x50ce50` — show a fixed-rect notice caption

- **VA / file offset**: `0x50ce50` / `0x10ce50`.
- **Behavior**: Requires a nonempty text and an existing target control, clears the old area, sets a fixed native rectangle, writes the text and ink through the caption helper, then refreshes two cursor words.
- **Reached from**: `0x55c0a0`.
- **Direct callees**: `EXTERNAL:0x62`, `0x411e60`, `0x41df80`, `0x42f560`, `0x430430`, `0x5f1bc0`.
- **Player-visible reach**: Player-visible: a notice caption appears inside the fixed rectangle.
- **Our client**: No counterpart by VA. Notice captions are staged in `crates/o2jam-shell/src/screens/game/mod.rs:899` and painted at `crates/o2jam-shell/src/screens/paint.rs:2995`.
- **Evidence**: analysis ev f566.c8f5.134d.3618.1760.8261.71b7.9e6d.0407.3fea.0d08.79ae.3aad.179c.5385.f9b0; callers ev abf6.efc1.e9f2.56dc.6261.26f6.a7af.e15a.dbfe.be5b.cb99.d38f.d573.be76.78d9.6907; file-offset ev 8004.d2cc.9ae4.90c0.3fbe.bd3e.96a1.9b63.797b.9b89.0ae5.e07d.bec1.d9d6.fe6c.1628; body batch ev a0e0.e8ff.636a.697c.ab7e.e55a.74f9.ba4e.0222.9612.04da.2265.9b5e.99cc.55ff.0c77.

### `0x50d1d0` — clear seven control buffers

- **VA / file offset**: `0x50d1d0` / `0x10d1d0`.
- **Behavior**: Calls the shared allocator helper once for each of seven object words.
- **Reached from**: `0x416a80`, `0x50d150`.
- **Direct callees**: `0x40c240`.
- **Player-visible reach**: Support for control teardown; no pixels or wire.
- **Our client**: No counterpart by VA; Rust drop handles the buffers.
- **Evidence**: analysis ev 6da2.d841.8003.82bb.ce45.214d.df7b.70fa.61c4.cbf7.015a.b59c.2f3d.16fe.cd83.3603; callers ev 8261.d1dd.678e.a86b.2b5a.18af.0fc3.46b4.0fb3.6085.40f5.55ce.df98.29fb.5f2a.b584; file-offset ev 4688.5372.424a.f89c.aab9.1f25.ba6e.ddf2.bdb3.4b50.a53b.0afb.f1e4.e34f.9103.7e1e; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x50d980` — initialize the shared control base

- **VA / file offset**: `0x50d980` / `0x10d980`.
- **Behavior**: Calls the ribbon progress initializer, installs the base control vftable, clears five words, and zeroes a rectangle.
- **Reached from**: `0x4f5d00`, `0x4f6040`, `0x4f7300`, `0x50a820`, `0x50af90`, `0x50bb70`, `0x50e240`, `0x50f160`, and 1 more.
- **Direct callees**: `EXTERNAL:0x62`, `0x4099a0`.
- **Player-visible reach**: Support. Every staged control body uses this base before its own vftable.
- **Our client**: No counterpart by VA. Staged controls are described in `crates/o2jam-shell/src/staging.rs`.
- **Evidence**: analysis ev d026.9e44.b5ff.e2c2.57f7.3097.9482.94c8.7d20.0859.61e6.9347.19cf.78a7.0616.103b; callers ev c0ba.4bca.36da.92a2.90fa.8929.4a59.a00f.b5e2.0c12.538e.e749.7704.ff57.9c78.cb9b; file-offset ev a30b.2c42.a28a.b36c.d15b.536c.addf.7664.453e.fd8e.1686.6264.c351.289e.1c53.94b8; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x50ddb0` — store one control word

- **VA / file offset**: `0x50ddb0` / `0x10ddb0`.
- **Behavior**: Writes the supplied value into the object's `+0x5c` word.
- **Reached from**: `0x4329b0`, `0x479900`, `0x48a620`, `0x48b2d0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for control factories; no visible decision here.
- **Our client**: No counterpart by VA.
- **Evidence**: analysis ev ee5d.2575.8b9e.d869.a520.e5f3.5a21.fd5f.19ac.ee2c.5318.d1f5.5341.c846.e976.fe9c; callers ev 69ce.cf92.2273.fcf5.4ec7.a6d2.b233.b736.89b1.ee4b.66db.aec4.9c0b.98f5.a585.9eac; file-offset ev 0742.7b12.3e31.60cf.ef48.cb28.5b51.e482.64e3.7460.7152.0825.83a7.16d0.fd54.8748; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x50e240` — construct a scroll bar

- **VA / file offset**: `0x50e240` / `0x10e240`.
- **Behavior**: Calls the shared initializer, installs the scroll-bar vftable, clears several words, and zeroes a range.
- **Reached from**: `0x4329b0`, `0x48a620`, `0x48b2d0`.
- **Direct callees**: `0x50d980`, `0x5f1ef0`.
- **Player-visible reach**: Support. Scroll-bar appearance comes from staged art and the scroll state.
- **Our client**: No counterpart by VA. Scroll-bar parameters stage nothing per `crates/o2jam-shell/src/staging.rs:1585`; arrow behaviour is guarded in `crates/o2jam-shell/src/screens/game/mod.rs:11839`.
- **Evidence**: analysis ev f7c5.c19b.2ba8.d801.53aa.7816.511e.8498.1dd6.1f74.85b9.6a60.1689.41c2.0ec2.86bd; callers ev fb60.7258.1fdb.ad4d.4a8a.4a9a.2f1f.6149.0a78.6ff0.0e6c.b53b.1a3b.41e0.b254.bfca; file-offset ev 9db4.2de6.92cd.4eb4.4fd5.bfb2.8f20.a601.c867.5129.1b3d.707a.2f06.7e4c.e290.2e05; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x50edb0` — clamp and advance a scroll position

- **VA / file offset**: `0x50edb0` / `0x10edb0`.
- **Behavior**: Stores a requested position, clamps it into the visible window, clamps it at zero, and forwards the signed change to the scroll helper.
- **Reached from**: `0x50efe0`.
- **Direct callees**: `0x50ee50`.
- **Player-visible reach**: Player-visible: the scrolled list top changes or clamps at the ends.
- **Our client**: No counterpart by VA. Arrow-clamped scrolled tops are guarded at `crates/o2jam-shell/src/screens/game/mod.rs:11863`.
- **Evidence**: analysis ev c5fe.35c5.4d86.2a4b.3807.6da7.f318.70cb.aa32.c250.b0b0.911a.a75d.155b.8c8d.bbe8; callers ev ac5f.29ed.6cb6.f201.781a.117d.a860.b6d6.fdc5.039e.0f2e.c409.2f42.9c48.21f4.716a; file-offset ev ea04.8926.ae6f.1e25.5cf1.3909.b831.f176.1e67.3289.72bc.92e4.e60f.5618.7a91.a92b; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x50f7f0` — construct a chat message box

- **VA / file offset**: `0x50f7f0` / `0x10f7f0`.
- **Behavior**: Calls a chat-box initializer, installs the chat-box vftable, and clears two trailing words.
- **Reached from**: `0x4329b0`, `0x479900`, `0x48a620`, `0x48b2d0`.
- **Direct callees**: `0x50f160`, `0x512390`.
- **Player-visible reach**: Support. Chat rows paint from staged chat state.
- **Our client**: No counterpart by VA. Chat rows are staged in `crates/o2jam-shell/src/screens/game/chat.rs:399` and painted in `crates/o2jam-shell/src/screens/paint/chat.rs:398`.
- **Evidence**: analysis ev 765e.0067.e93e.5a8e.fb12.ceeb.c662.2425.be9e.49d8.13f0.ba9b.717d.b4ce.d34e.8c2c; callers ev 96a6.fefc.7390.73ac.190f.adce.34a2.fb4a.3ed6.d9dc.1f29.de69.ed9e.90ab.e1ce.94a8; file-offset ev 6a5a.922d.7bd1.3ab6.9faf.5278.8fe6.5ceb.9840.f15a.dee0.6d70.bdca.624b.41b4.0e6e; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x512390` — initialize a chat text buffer

- **VA / file offset**: `0x512390` / `0x112390`.
- **Behavior**: Clears three buffer words. For a nonzero request it allocates a buffer, sets begin and end to it, and computes a capacity from the requested length; oversized requests go to the failure helper.
- **Reached from**: `0x50f7f0`.
- **Direct callees**: `0x512f80`, `0x513120`.
- **Player-visible reach**: Support for chat text storage; no pixels or wire.
- **Our client**: No counterpart by VA; `Vec` with a capacity supplies the same role in Rust.
- **Evidence**: analysis ev d1ad.0801.b6d6.e005.7d3b.bdec.f399.abe0.870a.68cf.3f74.ff22.727d.6ced.ceef.1506; callers ev fe71.f261.d186.53ea.00b3.9330.86e9.79ef.11c4.2d20.14ac.9f21.3b72.e063.4bcf.597c; file-offset ev cdc0.972e.dea0.f26c.dd06.8a32.cc54.c82a.1e71.6e1f.4346.1f67.d8a0.fc43.3d2b.ec21; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x5146b0` — construct an edit control

- **VA / file offset**: `0x5146b0` / `0x1146b0`.
- **Behavior**: Calls an edit-box constructor, installs the edit-control vftable, initializes text buffers, and calls the state-reset helper.
- **Reached from**: `0x4329b0`, `0x479900`, `0x48a620`, `0x48b2d0`.
- **Direct callees**: `0x403db0`, `0x4f7300`, `0x514810`, `0x519140`.
- **Player-visible reach**: Support. The edit control's visible text is set by the setup and append helpers.
- **Our client**: No counterpart by VA. Draft state is in `crates/o2jam-shell/src/screens/game/chat.rs:142`.
- **Evidence**: analysis ev 631f.ff0c.f579.453f.4e9b.8405.7e80.0c57.ce7e.a060.ac04.8a2c.17bf.3fd9.eda0.1ca6; callers ev 762d.b68a.adb2.4406.ba70.e51b.a0ea.0369.cc75.0fbc.a0e0.cce0.2956.fcf0.988e.2047; file-offset ev 773e.3a2c.726f.d078.f7da.4da3.3226.7d45.a656.7db8.49fe.7d8b.355d.322b.6f1d.e057; body batch ev f5e2.9b1d.1a49.4580.7f0c.122d.fd49.9b6c.c1c1.fba6.3e33.680a.8e04.eacf.f20f.ab36.

### `0x514800` — publish the active edit-control pointer

- **VA / file offset**: `0x514800` / `0x114800`.
- **Behavior**: Writes the supplied edit-control pointer into a global slot.
- **Reached from**: `0x432510`, `0x432840`, `0x4351e0`, `0x435aa0`, `0x43d4b0`, `0x456af0`, `0x4572c0`, `0x459310`, and 20 more.
- **Direct callees**: none observed.
- **Player-visible reach**: Support. Many control handlers call it before routing typing; it paints nothing itself.
- **Our client**: No counterpart by VA. Chat drafts are keyed by panel in `crates/o2jam-shell/src/screens/game/chat.rs:264`.
- **Evidence**: analysis ev 55fd.7b9f.bb01.a896.d7db.e0cb.1562.991e.6552.9945.740d.c571.bbf5.0792.bcdd.c092; callers ev 3b74.0ad4.7daa.bfe3.6dd8.e3dc.0c16.540e.7916.6c1e.ee8f.ba85.412e.e0c7.9f34.d252; file-offset ev e1fd.8ec7.142f.6058.b6eb.4d4b.ae59.94e0.11b8.36b2.8b09.0790.82bb.5e0e.acbb.2f87; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x514810` — reset edit-control defaults

- **VA / file offset**: `0x514810` / `0x114810`.
- **Behavior**: Resets a text buffer, clears three words, then sets default margin, line, and flag words.
- **Reached from**: `0x5146b0`.
- **Direct callees**: `0x51b050`, `0x51b150`.
- **Player-visible reach**: Support for the edit-control initial state.
- **Our client**: No counterpart by VA. Draft clearing is in `crates/o2jam-shell/src/screens/game/chat.rs:182`.
- **Evidence**: analysis ev 55db.3565.f399.2b6d.4743.703b.5e7f.f407.04eb.e644.1d99.9689.2720.644d.76df.26c0; callers ev 13c0.be76.617b.d378.e090.fc1c.b585.4971.4246.347f.6a4c.ec9d.768f.2dff.5080.310d; file-offset ev 8d97.da21.a169.b0e0.0ab2.231b.52ba.33e4.0cf4.afbd.f4f7.56cd.13f9.33fb.ed2b.315b; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x514990` — configure an edit control before typing

- **VA / file offset**: `0x514990` / `0x114990`.
- **Behavior**: Stores an identity word, sets IME conversion and a composition flag, copies four bounds words, creates a font from supplied metrics, clamps the text rectangle, sets the visible line count, and chooses an initial cursor word.
- **Reached from**: `0x4351e0`, `0x43d4b0`, `0x456af0`, `0x4572c0`, `0x459310`, `0x45b7e0`, `0x45fb60`, `0x46e220`, and 7 more.
- **Direct callees**: `0x5148e0`, `0x517d10`, `0x517dd0`, `0x517f00`, `0x519aa0`, `0x51a4f0`.
- **Player-visible reach**: Player-visible: the text box has the exe's font, bounds, and caret before the player types.
- **Our client**: No counterpart by VA. Modern font rasterization is `crates/o2jam-render/src/font.rs:24`; chat draft painting is `crates/o2jam-shell/src/screens/paint/chat.rs:398`.
- **Evidence**: analysis ev 32d2.cddf.6e07.196a.e546.8444.e48e.8420.1921.5b68.9361.5572.a163.2fa1.1fee.0015; callers ev e05e.72a6.0e9f.0f71.f111.0400.943b.f8a3.7607.d5da.d0e7.a796.878f.cc72.f610.5f5a; file-offset ev cd18.0d1e.835e.103f.83aa.e2d4.1a39.af84.24fb.eb9f.b64e.6f2c.e864.855c.e6aa.4d92; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x517d10` — create the edit font and read its metrics

- **VA / file offset**: `0x517d10` / `0x117d10`.
- **Behavior**: Creates a font from supplied size, weight, and face words, stores the handle, then obtains the device context and stores average character width and line height.
- **Reached from**: `0x514990`.
- **Direct callees**: `EXTERNAL:0x4a`, `EXTERNAL:0x4e`, `0x42c0a0`, `0x42c160`.
- **Player-visible reach**: Support for edit text layout; the metrics feed the visible rectangle and line count.
- **Our client**: No counterpart by VA. Owner decision pins a cross-platform rasterizer; see `crates/o2jam-render/src/font.rs:24`.
- **Evidence**: analysis ev 9248.214c.477a.83f8.ef4d.9d90.16cf.6e9d.eadf.3776.5ce8.5710.f742.24e9.834e.a267; callers ev d6e8.e452.49eb.1f48.64be.ea9f.8027.b649.bedd.423b.5701.ca0d.1380.3d35.2fcd.7e8e; file-offset ev f3aa.4c99.f4c2.8f3c.f5c3.f171.02c0.ae40.488b.9804.ecb2.d9bf.5df7.e542.8dcb.f71d; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x517dd0` — clamp the edit text rectangle

- **VA / file offset**: `0x517dd0` / `0x117dd0`.
- **Behavior**: Copies four bounds words, raises the right and bottom edges to fit one character plus padding, and disables the box when any bound is negative or beyond the screen size.
- **Reached from**: `0x514990`.
- **Direct callees**: `0x415c80`, `0x415ca0`.
- **Player-visible reach**: Player-visible: text area and caret placement follow the clamped rectangle.
- **Our client**: No counterpart by VA; staged text rectangles are clipped by `crates/o2jam-shell/src/screens/paint.rs:2995`.
- **Evidence**: analysis ev 7b11.9d44.8ec3.0328.67a3.7a31.2c55.a3cc.2dc9.9aee.ead1.cb62.ce7c.c933.3ae4.40b8; callers ev 0446.c57b.e004.7b07.4472.7d7f.fc58.efcf.4207.c210.7b82.e3a6.1bf5.554a.c59e.af0a; file-offset ev d58b.a3ab.96c3.701f.60a8.454c.5e15.37f1.8590.042f.b117.9322.cad4.8d15.8820.19bd; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x517f00` — set the edit control's visible line count

- **VA / file offset**: `0x517f00` / `0x117f00`.
- **Behavior**: Derives a line count from rectangle height and average character width, then stores either that count or a supplied fallback value.
- **Reached from**: `0x4351e0`, `0x435aa0`, `0x43d4b0`, `0x456af0`, `0x4572c0`, `0x459310`, `0x45b7e0`, `0x45fb60`, and 8 more.
- **Direct callees**: none observed.
- **Player-visible reach**: Player-visible: it decides how many typed lines fit in the box.
- **Our client**: No counterpart by VA. Chat row cap is modelled in `crates/o2jam-shell/src/screens/game/chat.rs:279`.
- **Evidence**: analysis ev 2021.efc5.2874.a258.5664.30aa.6b7f.ffa9.d4c8.2788.16a6.3d94.44b4.355b.9d00.90b6; callers ev 28c0.9d13.803c.22ee.86d1.5d95.f7c7.fb64.023e.c17b.fff2.6bee.a041.6285.6d06.3e3a; file-offset ev 94bd.bae6.81be.c59c.62fc.ee8f.20da.fc65.9eca.d429.d885.0f4f.9cc8.a2d2.83f3.fad7; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x518fa0` — move the active edit caret to the end

- **VA / file offset**: `0x518fa0` / `0x118fa0`.
- **Behavior**: When the active-control slot equals this object it calls the caret-to-end helper; otherwise it writes a start offset and copies another word into it.
- **Reached from**: `0x4351e0`, `0x436a00`, `0x518000`, `0x518e80`.
- **Direct callees**: `0x519640`.
- **Player-visible reach**: Player-visible: the caret and selected text move when the edit control becomes active.
- **Our client**: No counterpart by VA. Draft text is held in `crates/o2jam-shell/src/screens/game/chat.rs:150`.
- **Evidence**: analysis ev 626f.7bca.33bd.0548.20a8.1b26.f8f9.af7c.f1fa.d2aa.a2a5.dcfc.94b3.6cab.0da3.8d87; callers ev f69e.6ea0.8909.af2d.5aab.d7ad.de86.3ee3.1712.6046.c3a5.4d88.b652.d91c.73f9.c624; file-offset ev eb1b.6c80.5c54.8d96.5084.f724.00ba.2cbe.c655.32f6.9d1b.63e4.f69a.cec2.b8f7.e851; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x5195c0` — read two edit state words

- **VA / file offset**: `0x5195c0` / `0x1195c0`.
- **Behavior**: Copies two adjacent words from the edit object into caller outputs.
- **Reached from**: `0x519000`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for the edit painting path; no pixels itself.
- **Our client**: No counterpart by VA.
- **Evidence**: analysis ev fa11.19e8.4599.34bb.6fe4.d9f5.60cf.8c58.9558.3a2f.b9e3.a359.8379.ed4f.ea9a.5b66; callers ev e19f.61f3.1dbd.38e6.b128.0415.974a.86d0.c73b.3939.69e8.5323.ce3e.4f39.1e14.e6e7; file-offset ev 597f.70e2.4e5a.fbdf.8a04.9dfa.34f0.7e8f.f8ad.8a17.eb58.9f2b.9895.b711.427d.5fc4; body batch ev 647a.17b5.12a0.6a19.12e1.4078.7edd.0b68.10e8.cb29.9f10.a099.2ae5.768c.c22c.c42b.

### `0x519640` — clamp two edit state words

- **VA / file offset**: `0x519640` / `0x119640`.
- **Behavior**: Writes two supplied words, clamps each at zero and at a stored maximum, and returns.
- **Reached from**: `0x518fa0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Player-visible: caret or selection values stay inside the valid text range.
- **Our client**: No counterpart by VA; draft length bounds live in `crates/o2jam-shell/src/screens/game/chat.rs:142`.
- **Evidence**: analysis ev 237f.3704.0cca.1aa9.aeb6.810b.b0bb.8495.3aa0.c05b.1658.387f.7a80.4303.4882.0e4c; callers ev d2f1.a7c0.0a9a.bd32.0d03.42d5.b638.e982.1225.42ae.21e3.dc7c.84af.fae1.a35d.ef38; file-offset ev 1847.a614.13ec.3dd7.1d73.484e.6f06.89d2.c7ff.5693.6b78.9ab7.4513.99e3.d2b1.cdef; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x519aa0` — set the edit composition byte

- **VA / file offset**: `0x519aa0` / `0x119aa0`.
- **Behavior**: Writes one byte into the object's composition-state word.
- **Reached from**: `0x514990`, `0x518e80`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for IME/typing state; no direct paint.
- **Our client**: No counterpart by VA. Draft characters are stored at `crates/o2jam-shell/src/screens/game/chat.rs:163`.
- **Evidence**: analysis ev a264.445d.df96.88ff.2c9d.8acc.9898.05c9.acae.e8d4.5797.c9a8.e3ac.d005.933c.5369; callers ev 4eae.a23a.afb1.e196.1cd2.85b8.d5ff.277d.337a.a1df.7a3e.efa5.5c72.7eab.441a.5fcc; file-offset ev eaeb.83c1.5ca3.3cda.d2c7.aac8.3170.9297.f541.4954.0a32.45ca.0ea3.8979.7231.b286; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x519fe0` — cancel an IME composition

- **VA / file offset**: `0x519fe0` / `0x119fe0`.
- **Behavior**: Gets the input-method context for the stored window, notifies it to cancel composition, and releases the context when one exists.
- **Reached from**: `0x5194e0`.
- **Direct callees**: `0x5af796`, `0x5af7a2`, `0x5af7b4`.
- **Player-visible reach**: Player-visible: an in-progress composition disappears when typing is reset.
- **Our client**: No counterpart by VA. Rust keyboard input is routed in `crates/o2jam-shell/src/window.rs:554`; no IME context is held.
- **Evidence**: analysis ev 7e1b.49cc.7448.7def.7e21.1af8.0b05.ba26.6e61.82c0.724e.57e5.bdbf.e705.2034.713d; callers ev 2e0e.8522.0184.63c7.887c.4d9c.42a8.0756.88d3.ef10.1be3.5d54.a385.bb86.5fb1.c700; file-offset ev a644.712d.b81b.1f27.6fe6.fcf5.3f7c.581a.ab6f.d860.5ae9.0d45.e551.4247.3308.6527; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x51a080` — append text to the edit buffer

- **VA / file offset**: `0x51a080` / `0x11a080`.
- **Behavior**: Clears a scratch buffer, fetches the current text pointer, appends the supplied bytes, and advances the stored end pointer by the appended byte count.
- **Reached from**: `0x518000`, `0x5181c0`, `0x518230`, `0x518e80`, `0x519a00`, `0x519d30`, `0x51a240`.
- **Direct callees**: `0x51a470`, `0x51b650`, `0x5f1bc0`.
- **Player-visible reach**: Player-visible: typed characters join the displayed edit text.
- **Our client**: No counterpart by VA. Typing appends to the draft in `crates/o2jam-shell/src/screens/game/chat.rs:534`.
- **Evidence**: analysis ev 6770.c7c4.b5a1.78a4.7bf8.dade.aa0e.2123.77cd.8ca1.5397.4de8.d934.3ed7.664a.fb88; callers ev 822a.22a1.f0f4.b50f.dbb4.5664.4484.9e99.4f5e.3b9d.691c.60c2.5c0b.605a.17f1.466d; file-offset ev 3a3e.ad96.054b.dbc7.83b2.22bd.59ac.ee42.b878.a607.51bf.a297.2849.be3a.f07a.610b; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x51a4f0` — switch IME conversion status

- **VA / file offset**: `0x51a4f0` / `0x11a4f0`.
- **Behavior**: Gets the input-method context, reads its conversion status, sets the conversion word from the supplied flag, and releases the context.
- **Reached from**: `0x514990`.
- **Direct callees**: `0x5af796`, `0x5af79c`, `0x5af7a2`, `0x5af7ba`.
- **Player-visible reach**: Player-visible: Hangul/English conversion state changes for the next typed character.
- **Our client**: No counterpart by VA. Our text input carries no IME conversion flag.
- **Evidence**: analysis ev 6c74.627d.c0ab.a9e7.1c7c.9afa.f4a4.3a2c.a422.7280.1539.8935.1733.75ba.1b4f.c03d; callers ev c1c0.1b6f.bdc8.0a79.fb35.7525.028d.3243.9851.ced2.8ee5.cc50.3321.881c.2935.936f; file-offset ev 02f4.92ba.c6fd.86d8.79ca.4a52.66d5.b4b8.fbf4.baa6.f212.7f6e.d19a.eac2.a4a3.556d; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x51c480` — insert a range into the four-byte edit vector

- **VA / file offset**: `0x51c480` / `0x11c480`.
- **Behavior**: Computes used and capacity, grows when needed, moves the tail, writes the inserted values, and updates begin, end, and capacity. Oversized requests go to the failure helper.
- **Reached from**: `0x51b810`.
- **Direct callees**: `0x51c869`, `0x51ca3f`, `0x51cc10`, `0x51cc60`, `0x51d390`, `0x51d430`, `0x51d620`, `0x51d740`, and 1 more.
- **Player-visible reach**: Support for typed-character insertion; no pixels or wire.
- **Our client**: No counterpart by VA; `Vec::insert` supplies the role in Rust.
- **Evidence**: analysis ev ae6d.ceb2.350c.3e71.b3a6.26fa.a38e.03a4.8cbd.3ea3.05c7.5afb.85d2.61aa.f87d.466a; callers ev 90c2.95ba.20d8.f2a8.4995.e267.3341.cedd.b76d.dc1f.ba1c.5a0a.58b2.a6e2.58b6.593a; file-offset ev d121.0c00.ad88.afc7.7cca.80b1.490f.79e0.16bf.588b.fcc0.8ab8.a9a3.3132.73c1.3331; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x51c869` — rebalance the edit vector after an insert

- **VA / file offset**: `0x51c869` / `0x11c869`.
- **Behavior**: Frees the old block and recomputes begin, end, and capacity from the replacement buffer after an insert grew the vector.
- **Reached from**: `0x51c480`.
- **Direct callees**: `0x5f1bb5`.
- **Player-visible reach**: Support for typed-character insertion.
- **Our client**: No counterpart by VA; Rust allocation handles this.
- **Evidence**: analysis ev 79ba.1419.fc19.7b31.c8ca.dd43.f973.ecb5.944b.f669.8d7f.4997.1659.e379.f650.9588; callers ev 40fb.60db.d7bf.e2f6.96c4.8d3d.f731.a2db.9d28.aeec.194e.4045.8e76.d53a.5cf9.ea61; file-offset ev bbd1.bb00.a13d.5494.37fc.342b.abcd.857a.eab0.cc53.f2e2.394f.d82b.a3cb.446a.a51c; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x51ca3f` — fill an inserted range of the edit vector

- **VA / file offset**: `0x51ca3f` / `0x11ca3f`.
- **Behavior**: Advances the end pointer and writes one supplied value through every slot of the inserted range.
- **Reached from**: `0x51c480`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for typed-character insertion.
- **Our client**: No counterpart by VA; `Vec` fill idiom supplies this role.
- **Evidence**: analysis ev a0ee.92cc.9acf.af10.546a.e0bd.9789.2623.36bc.6ffe.aace.48e7.6c11.248d.98ed.3756; callers ev f1ad.02f1.2eb6.c139.159b.48c9.bf44.e143.7168.c039.8e55.8b71.e950.0e87.185b.fd6a; file-offset ev f12f.fb2f.f831.6dd6.2145.f218.4b1a.c494.6f81.7f28.a706.ee69.1a80.4f72.5b2b.8c57; body batch ev 5c6c.23c6.8269.dd2b.a2af.aeec.5f77.1609.f8ff.c1f1.01fd.1849.6405.de24.0824.fe08.

### `0x51cc10` — copy filled values and return the new end

- **VA / file offset**: `0x51cc10` / `0x11cc10`.
- **Behavior**: Copies a counted run from a source to a destination and returns the destination advanced past it.
- **Reached from**: `0x51c480`.
- **Direct callees**: `0x51d740`.
- **Player-visible reach**: Support for edit vector insertion.
- **Our client**: No counterpart by VA; `copy_from_slice` supplies this role.
- **Evidence**: analysis ev 0d20.a9f2.ce09.25f5.e3d1.14f5.ec4e.7f63.fece.596d.6ad1.a55c.f322.e1e8.ee14.20bf; callers ev dbaa.8c7d.ffd5.4fab.d427.976b.e2ac.3028.362c.2069.e517.a07e.1a96.eef1.8409.ec18; file-offset ev 0398.f556.7911.60c5.a81c.7fab.47d2.4644.a910.bbf9.1b7e.2fe9.5df9.2574.b8ce.07f3; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

### `0x51d390` — move a four-byte range backward

- **VA / file offset**: `0x51d390` / `0x11d390`.
- **Behavior**: Computes the slot count between two pointers, moves that many bytes when positive, and returns the destination start.
- **Reached from**: `0x51c480`.
- **Direct callees**: `0x5f22f0`.
- **Player-visible reach**: Support for edit vector insertion.
- **Our client**: No counterpart by VA; Rust slice copy supplies this role.
- **Evidence**: analysis ev cfcb.9867.e6b6.bbe6.2187.4f79.85a9.9347.048a.5e64.8303.da94.ff68.5ad0.c281.fcf1; callers ev eff9.2829.2f7a.f405.b2d8.71e3.ee06.6d0e.4fef.8221.3e70.4f6f.2b21.c144.55fe.f309; file-offset ev b89e.9231.832c.de3d.bab3.b0c4.9d21.1606.74ef.5d0f.70a0.b6eb.cef1.d58f.cb26.4695; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

### `0x51d550` — clear and rebuild the edit text buffers

- **VA / file offset**: `0x51d550` / `0x11d550`.
- **Behavior**: Resets two buffers through the shared string helper and forwards the supplied text range to the rebuild helper.
- **Reached from**: `0x518000`.
- **Direct callees**: `0x507fd0`, `0x51d670`.
- **Player-visible reach**: Support for setting or clearing the edit text.
- **Our client**: No counterpart by VA. Draft replacement is in `crates/o2jam-shell/src/screens/game/chat.rs:177`.
- **Evidence**: analysis ev f800.f6e4.86b2.cc71.15fe.e4af.3983.b221.d045.ebcd.c7ff.2d0f.730b.06a5.8243.8f15; callers ev 4619.b4fe.c673.0df0.43be.663b.0ed4.6a3c.851f.bca1.7518.51da.ca15.46ef.c9c1.00f0; file-offset ev 2d75.7fb5.05fc.98ff.d6f9.6308.ba56.1709.16aa.3cc5.5ca0.9dcf.141c.1fe8.5b24.3bd8; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

### `0x51d620` — forward a bounded four-byte range move

- **VA / file offset**: `0x51d620` / `0x11d620`.
- **Behavior**: Delegates all four arguments unchanged to the shared bounded move helper.
- **Reached from**: `0x51c480`.
- **Direct callees**: `0x51d880`.
- **Player-visible reach**: Support for edit vector insertion.
- **Our client**: No counterpart by VA; Rust slice copy supplies this role.
- **Evidence**: analysis ev eaac.46ca.9e13.4f8d.71c7.92a2.cc35.9e32.9617.8bec.507d.fc7a.0e68.70f2.27f5.062c; callers ev 1156.defd.64e5.ccd3.86f2.e51a.9b0c.62a5.d89f.3673.9eae.7b52.adef.28dc.0424.9826; file-offset ev 49d7.aa04.aed6.d0ea.083d.717b.d99b.fde3.a277.589b.3138.341e.a1fa.3197.bdf2.4106; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

### `0x51d740` — write one value into every slot of a range

- **VA / file offset**: `0x51d740` / `0x11d740`.
- **Behavior**: Loops over a count, writes one source value into each destination slot, and advances both pointers.
- **Reached from**: `0x51b150`, `0x51c480`, `0x51cc10`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support for edit vector insertion.
- **Our client**: No counterpart by VA; `Vec` fill idiom supplies this role.
- **Evidence**: analysis ev d2d2.34da.0b8f.863a.057a.465b.4433.5687.fe76.8420.a433.5dd1.9f8b.a714.26c3.032f; callers ev c21b.92cd.f8fc.b4f1.9040.efb1.17b3.1f4a.7266.6b28.0d6a.e1f4.c279.d441.f409.c9ee; file-offset ev a39a.3141.4431.ca93.9787.2ced.b0fc.4966.f1c9.652b.cc8b.d798.af61.13ae.f8b7.2506; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

### `0x51d880` — move a counted four-byte range

- **VA / file offset**: `0x51d880` / `0x11d880`.
- **Behavior**: Computes the slot count, performs the byte move when nonzero, and returns the destination advanced past the moved range.
- **Reached from**: `0x51c480`, `0x51d620`.
- **Direct callees**: `0x5f22f0`.
- **Player-visible reach**: Support for edit vector insertion.
- **Our client**: No counterpart by VA; Rust slice copy supplies this role.
- **Evidence**: analysis ev ff84.3f91.b191.b4f2.6fb8.8efa.8d1c.0743.0780.d6e9.0a97.685a.29e4.70ca.9c17.ccc1; callers ev 9225.3c83.2a08.5079.d11d.835c.62ce.57cc.3084.4283.2a39.81d9.34f6.4037.0fd7.b95e; file-offset ev 0f3c.df7d.05cd.416c.0675.72c7.b4ff.978b.95c6.c8ec.821a.868e.a1f0.992e.29b4.f5d6; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

### `0x51e290` — fill a byte range during edit rebuild

- **VA / file offset**: `0x51e290` / `0x11e290`.
- **Behavior**: Allocates or resizes a scratch buffer by the pointer span, writes one supplied byte through every slot, and clears the exception frame.
- **Reached from**: `0x51d670`.
- **Direct callees**: `0x5069c0`, `0x51b050`, `0x51e334`.
- **Player-visible reach**: Support for edit text rebuild; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec` fill supplies this role.
- **Evidence**: analysis ev bf52.a5b4.c38b.5705.9e79.add0.cf52.9971.013c.522b.5ef4.795f.d3df.755e.78f6.feec; callers ev 5099.df87.5495.89f4.a50e.f6a7.1d7c.10a2.b365.7b70.f0c9.6157.2beb.bd5b.917b.8f53; file-offset ev 29ca.cc5b.2236.3a44.5600.b592.9157.3116.c89f.b219.60d9.ff2e.443a.2115.b69f.ebb6; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

### `0x51e334` — clear the edit rebuild exception frame

- **VA / file offset**: `0x51e334` / `0x11e334`.
- **Behavior**: Marks the local exception frame as failed and restores the saved exception-list head.
- **Reached from**: `0x51e290`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support. Compiler-generated cleanup body with no player-visible decision.
- **Our client**: No counterpart by VA; MSVC unwind scaffolding has no Rust equivalent.
- **Evidence**: analysis ev cec2.f855.8af4.7492.69c2.3958.ffb2.3ff4.7694.915b.379f.9d9e.1186.6669.d0f2.5e95; callers ev 8ad3.f684.5f55.6d71.3fe6.363c.de65.f86a.0d5a.104c.4744.951d.7909.e3fa.6229.dc03; file-offset ev d1cc.7cfa.4888.ff07.bca4.676a.8e17.d275.f07f.d956.db7e.34be.ad74.3d08.ec85.bca5; body batch ev 5568.2014.6a06.3d48.04cf.5746.5697.8230.639b.4944.ae6e.2093.4834.6dc2.884e.1eea.

## Scope notes and OPENs

- Every row is a static pin on the exact artifact hash above. The provider does not prove a runtime press, hover, frame, or packet reached the body.
- Custom cursor bodies (`0x42fb30`–`0x4321f0`) are pinned from their static callers; a live pointer-mode capture with `Window_Cursor.ojs` art remains OPEN.
- Edit-control and IME bodies (`0x514990`–`0x51e334`) are pinned as one chain from the edit constructor to typed-character insertion. A live typing leg with a real key event and a painted draft remains OPEN; the shell uses a modern font rasterizer by owner decision, so glyph shape is presentation.
- Room row, room caption, notice, and scroll bodies are pinned from their static callers. Wire-driven screenshot legs remain OPEN under `docs/live-path-reachability.md`.
- Spec-shaped copy of the player-visible subset: `/home/arnim/scratch/lanes-2026-11-25/spec-enrich-pins-ui-draw-2.md`; query log: `findings-pins-ui-draw-2.md`.
