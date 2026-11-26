# Statics pins — Input / messenger (input-msg)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`.
Provider: REA 6.10.1 / Ghidra 12.2 on the bound 7.55 artifact. Evidence IDs are
`ev ` followed by the full ID in four-digit groups; strip the dots and prefix
`ev_` to recover the REA ID. File offsets come from REA
`address_to_file_offset` (PE image base `0x400000`) and match VA minus
`0x400000` for all 58 rows. Behavior wording is ours; no disassembly or
decompiled text is reproduced.

Branch `cb2/pins-input-msg`; worktree pre-flight
`/home/arnim/Projects/recomp2jam-lane-pins-input-msg`. Brief base `a15244e`
and observed HEAD `a42ca1ed3c8d94f66d23f40937c19ab57b17816c` are both
commits; the brief base is an ancestor of HEAD. No reset was attempted.

Scope: the 58 addresses in `slice-input-msg.txt` from the coverage triage
bucket `game-reachable`, `pinned_in_repo=no`. **Pinned here: 58/58; left: 0.**
Fourteen rows are player-visible routes; the other 44 are message-buffer,
container, synchronization or boot-support bodies. A static pin is not a
player-visible close: census rows stay OPEN until a live caller chain from
boot is recorded (`docs/live-path-reachability.md`). "Reached from" names
the provider's direct-caller edge, not a runtime press or frame.

Shared context: the packet dispatcher `0x558080` dossier is ev 5032.6588.ab15.2e39.7a49.5695.3331.7425.09e6.8c5e.caaf.afe3.2518.03ff.782c.8ae9.
The 3003 refusal captions were read as static bytes and the web-sheet
openers as static bodies; no live capture is claimed by this lane.

## Inventory

| VA | File offset | Role |
|---|---|---|
| `0x406640` | `0x6640` | support |
| `0x408810` | `0x8810` | support |
| `0x408860` | `0x8860` | support |
| `0x416a80` | `0x16a80` | player-visible |
| `0x4581d0` | `0x581d0` | support |
| `0x4586cb` | `0x586cb` | support |
| `0x458e50` | `0x58e50` | support |
| `0x458f3b` | `0x58f3b` | support |
| `0x4590a0` | `0x590a0` | support |
| `0x4590f0` | `0x590f0` | support |
| `0x4591e0` | `0x591e0` | support |
| `0x45e450` | `0x5e450` | player-visible |
| `0x45ecd0` | `0x5ecd0` | player-visible |
| `0x45f480` | `0x5f480` | player-visible |
| `0x5148e0` | `0x1148e0` | support |
| `0x55c790` | `0x15c790` | player-visible |
| `0x55cad0` | `0x15cad0` | player-visible |
| `0x5630d0` | `0x1630d0` | support |
| `0x563db0` | `0x163db0` | support |
| `0x58d580` | `0x18d580` | player-visible |
| `0x59025a` | `0x19025a` | support |
| `0x590320` | `0x190320` | support |
| `0x5903c0` | `0x1903c0` | support |
| `0x590590` | `0x190590` | support |
| `0x590600` | `0x190600` | support |
| `0x592380` | `0x192380` | support |
| `0x5a35c0` | `0x1a35c0` | player-visible |
| `0x5a36d0` | `0x1a36d0` | player-visible |
| `0x5a3820` | `0x1a3820` | support |
| `0x5a4ac0` | `0x1a4ac0` | player-visible |
| `0x5a4b10` | `0x1a4b10` | support |
| `0x5a4b50` | `0x1a4b50` | support |
| `0x5a5020` | `0x1a5020` | player-visible |
| `0x5a5040` | `0x1a5040` | support |
| `0x5a50b0` | `0x1a50b0` | player-visible |
| `0x5a7060` | `0x1a7060` | support |
| `0x5a8a30` | `0x1a8a30` | support |
| `0x5a8b20` | `0x1a8b20` | player-visible |
| `0x5a8ba0` | `0x1a8ba0` | player-visible |
| `0x5ac690` | `0x1ac690` | support |
| `0x5afae0` | `0x1afae0` | support |
| `0x5afbb0` | `0x1afbb0` | support |
| `0x5b01c0` | `0x1b01c0` | support |
| `0x5b02a0` | `0x1b02a0` | support |
| `0x5b0c50` | `0x1b0c50` | support |
| `0x5b0c70` | `0x1b0c70` | support |
| `0x5b0ea0` | `0x1b0ea0` | support |
| `0x5b0f00` | `0x1b0f00` | support |
| `0x5b1020` | `0x1b1020` | support |
| `0x5b2a60` | `0x1b2a60` | support |
| `0x5b31a0` | `0x1b31a0` | support |
| `0x5b3560` | `0x1b3560` | support |
| `0x5b3720` | `0x1b3720` | support |
| `0x5b3910` | `0x1b3910` | support |
| `0x5b85d0` | `0x1b85d0` | support |
| `0x5b85f0` | `0x1b85f0` | support |
| `0x5c4500` | `0x1c4500` | support |
| `0x5c4520` | `0x1c4520` | support |

## Pins

### `0x406640` — install the event-listener vftable (support)

- **VA / file offset**: `0x406640` / `0x6640`.
- **Behavior**: Writes the CEventListener dispatch table at the start of the object and returns the same object. No state besides the table is touched.
- **Reached from**: 0x431a80, 0x432230, 0x489df0.
- **Player-visible reach**: Support. The object is part of listener plumbing; this body emits no pixels and sends no frame.
- **Evidence**: analysis ev e849.4c72.b954.6f49.229a.2cbe.54dc.6d2e.fb95.5c41.8a2f.6c5b.58a0.6d9b.9865.2769; dossier ev 2157.a206.75f4.3067.9669.d6b2.b11a.4a3a.4e13.c4e7.c874.816f.af8d.8ddc.4f9b.888f; caller facet ev 6f8b.988b.7ecf.f911.c974.8e90.f3a2.2f39.0ffc.eb29.9e75.63cc.cb29.b626.9ebc.bd0e; callee facet ev 34b7.e7b8.6879.0bf8.0dca.2696.b238.5984.ebd2.2e69.eb74.d92e.1504.bf26.e0d5.3faf; incoming xrefs ev 1b52.0cbb.5f9e.d41f.fb4a.9e4a.9e2a.010e.d0bc.6c30.2983.e97f.6e0e.e3b5.efac.7631; body batch ev b31e.52c5.b36c.81f8.46f4.0b5a.5c97.77db.59c5.d07a.67d1.f59d.62af.555e.345f.803d; file-offset mapping ev abda.3aa0.b654.b487.3d49.2d60.aed4.697f.c986.f197.e9fa.085e.0297.13bc.67e3.f6d7.
- **Our client**: No counterpart by VA. Winit event routing lives in `crates/o2jam-shell/src/window.rs:554`; no CEventListener object exists.

### `0x408810` — construct a message buffer

- **VA / file offset**: `0x408810` / `0x8810`.
- **Behavior**: Installs the CMessageBuff dispatch table and calls the companion initializer at `0x4088a0`; no message row is copied here.
- **Reached from**: 12 direct callers, among them `0x48f3a0` (Room dispatch), `0x4b7c60` (Waiting dispatch), `0x55c280` (2019 receive) and `0x513640`.
- **Player-visible reach**: Support. It prepares the container that later chat rows use; it paints nothing on its own.
- **Evidence**: analysis ev c0b4.ba0a.033e.ad9c.8d76.e65f.cee6.4191.057d.e258.9f63.91b9.b8a6.2e2d.9104.cc9e; dossier ev 7584.d431.aafd.26a3.0cb6.6e81.24e6.6ecd.3fe1.5d8c.f740.509c.b1fe.c939.2437.2293; caller facet ev 091b.a7b9.e531.2727.632a.abb5.c04a.bbe6.e3cd.4520.9bbc.0c14.56d9.491e.0d41.7848; callee facet ev 57cf.c8ea.0476.a2e2.b861.fd20.1ead.e280.edf5.5250.3718.4b60.5cfb.9542.8b8d.4811; incoming xrefs ev 2dcc.2eae.62c2.994a.5c58.fbec.d6b6.ad05.d8dc.6d9d.2326.08b8.94ed.f82c.7ba3.bf51; body batch ev b31e.52c5.b36c.81f8.46f4.0b5a.5c97.77db.59c5.d07a.67d1.f59d.62af.555e.345f.803d; file-offset mapping ev 866f.e6df.2f2e.6171.a2fd.d8c1.88c4.369b.bc3b.6d00.00f7.68cc.49da.726f.0d2f.f8d4.
- **Our client**: No native buffer. Chat rows and drafts are `ChatState` fields in `crates/o2jam-shell/src/screens/game/chat.rs:422` and `:529`.

### `0x408860` — destroy a message buffer

- **VA / file offset**: `0x408860` / `0x8860`.
- **Behavior**: Reinstalls the CMessageBuff table, frees the block stored at `+8` when present, and clears the handle. Called from many teardown paths.
- **Reached from**: 31 direct callers; the message-row helpers at `0x4581d0`, `0x4586cb` and `0x458e50` call it after rebalancing.
- **Player-visible reach**: Support. Cleanup only.
- **Evidence**: analysis ev 0c66.a809.fbf4.a67d.ee04.a0a8.f7fa.4fc7.13cd.6b8d.66b8.829c.b717.b868.0f9a.1110; dossier ev 88a2.8cdf.aa41.ab40.61e3.ce48.409f.aebf.1c22.f7de.b0d8.ca4a.368d.8198.4ba5.c9e4; caller facet ev ed60.2f72.fd34.eb5f.0124.1f37.e47f.e929.9aa9.d3bf.1aab.6973.05cd.63d9.7c3d.d47f; callee facet ev 0caa.b6c8.2399.bbd4.05c8.4d9f.63d0.ebed.4ccb.2626.7d9c.5974.4b84.0668.eb6b.f140; incoming xrefs ev e3a1.03df.abb9.fbe2.bade.bd11.8780.be4b.a5c2.e844.4553.6686.0057.88c3.6955.0b3c; body batch ev b31e.52c5.b36c.81f8.46f4.0b5a.5c97.77db.59c5.d07a.67d1.f59d.62af.555e.345f.803d; file-offset mapping ev 70ea.caee.8654.6890.d4ad.c1da.97b3.dea6.5b40.790d.a080.8d06.a8e8.70c0.8370.59fb.
- **Our client**: No counterpart by VA; Rust `Vec` drop owns this role.

### `0x416a80` — apply a display-mode flag and rebuild the 800x600 window

- **VA / file offset**: `0x416a80` / `0x16a80`.
- **Behavior**: Compares the requested mode with the stored word. On change it sets fullscreen or windowed style, rebuilds the DirectDraw surfaces, resizes the client to 800x600, adjusts the outer frame to the chosen style, shows and focuses the window, and records the new clock value. A surface-rebuild error is routed to the numeric error box. Returning to scene 2 also closes the Room web sheet when its flag is set.
- **Reached from**: 0x439f40, 0x477990 and the focus handler `0x5a4b90`.
- **Player-visible reach**: Yes. Switching mode changes the visible window style and surface; a failed switch produces an error popup.
- **Evidence**: analysis ev be7d.984a.8bbc.7980.86b1.59b7.106b.4174.c763.2c68.572e.3f14.5612.2523.4b49.e68e; dossier ev 37f9.893a.788e.2539.214a.4762.25ea.fa5d.d31e.0d75.c0c5.7583.7398.10bb.cb48.f301; caller facet ev 4137.bfeb.e76e.bde9.0ade.8c2a.cc25.7f86.7bdd.bab2.dff3.9861.77a5.fd0e.68a4.d870; callee facet ev 8d9a.faab.2e8a.2ef4.09b3.d2d1.ed2e.eb32.d4f6.1f22.3480.7761.3e20.663f.a5d3.70f0; incoming xrefs ev d0e5.cdca.c60a.5798.0773.d10b.92b3.bbca.321f.3d8f.d8fc.2ebb.ad31.e28d.ba1c.c7ad; body batch ev 76d5.9fcf.c94f.5d81.762a.7a9d.c013.b5e9.2da0.67b7.d162.0882.8e26.2a7b.5238.6a95; file-offset mapping ev 12b0.4eb3.e715.9518.8368.95b3.e4b2.5f50.9198.8716.3539.2af1.0ccf.4a8e.eec5.01d1; display-init web close callee ev 1981.b210.2ccc.fee7.e1cb.6fa0.5edf.430c.720b.6bfb.b177.20ce.e461.70f7.fecf.5ec0.
- **Our client**: Partial. The 800x600 default and fullscreen-versus-captioned style are documented in `crates/o2jam-shell/src/lib.rs:48`; there is no runtime mode-switch leg through the winit window.

### `0x4581d0` — insert a range of 0x10c-byte message rows (support)

- **VA / file offset**: `0x4581d0` / `0x581d0`.
- **Behavior**: Copies the two header words and the 0x100-byte payload of a message template into a vector of 0x10c-byte rows. It grows storage by one-and-a-half or by the exact request when needed, copies into a free gap or an appended range, stamps each new row, then rebalances the end pointers.
- **Reached from**: Only `0x4580a0`.
- **Player-visible reach**: Support. This is the chat/message row allocator; no direct paint or wire.
- **Evidence**: analysis ev 745e.c3b5.43e3.909d.4e5b.a541.af10.c82e.1788.e2ae.6d57.e677.5d0a.1d30.f5fb.8f11; dossier ev 57ba.dbb4.bfcb.6462.c50a.c198.8586.0e01.a4f3.88d6.99ee.26f7.27ad.7e1b.cc96.5e6d; caller facet ev 7638.ec2f.2fe2.4af4.ec7e.dd4f.d89f.6f5f.3308.39ce.9943.33b3.3f02.ed52.ff6e.5a38; callee facet ev 2e7a.c4fc.477f.61d0.e9b5.b8dd.8f81.eb65.55de.ba04.d95d.d7e5.f841.1e62.2f9d.5ad5; incoming xrefs ev f615.f70a.4877.f9f5.ae7c.45f4.00ea.5ada.d9e8.8809.3864.a192.5db1.88ae.cd7c.0117; body batch ev 76d5.9fcf.c94f.5d81.762a.7a9d.c013.b5e9.2da0.67b7.d162.0882.8e26.2a7b.5238.6a95; file-offset mapping ev 197a.6437.9c80.02a7.9965.6bd9.06e1.e612.773a.f730.5e7b.a61b.647a.5d81.80a1.6bfc.
- **Our client**: No counterpart by VA; chat rows are staged in `crates/o2jam-shell/src/screens/game/chat.rs:422`.

### `0x4586cb` — rebalance the message buffer after an insert (support)

- **VA / file offset**: `0x4586cb` / `0x586cb`.
- **Behavior**: Derives the live row count from the begin/end pointers, walks rows and invokes each row's dispatch helper, frees the old block, then recomputes begin, end and capacity from the saved buffer and count.
- **Reached from**: Only `0x4581d0`.
- **Player-visible reach**: Support; no pixels or wire.
- **Evidence**: analysis ev 1e50.7a98.215b.a2e0.d3b9.effa.8ea4.5bfc.e9af.848e.be2f.dd84.98d3.801b.eb80.85c4; dossier ev a06b.5ad2.7e0c.a437.3232.156d.dcb3.b1a2.c173.0a28.e5da.0e0f.e2af.6099.7222.7890; caller facet ev da16.6880.015b.562d.6b7d.4d36.0aff.5c0a.5cfa.5e07.dc68.cb24.1d30.72f4.26aa.5a73; callee facet ev 8bb9.bea0.5526.0ecf.25df.3618.928a.5fc6.dd05.2f07.0060.a2c5.f999.291c.3fb3.f5fe; incoming xrefs ev 34bd.1a6a.9bc1.5380.b550.3673.762f.73ac.0325.64cc.370f.4edb.3e54.1395.97dd.5865; body batch ev 76d5.9fcf.c94f.5d81.762a.7a9d.c013.b5e9.2da0.67b7.d162.0882.8e26.2a7b.5238.6a95; file-offset mapping ev c99d.807f.1750.4522.aa38.b52f.23b1.5eda.7131.3ea2.f391.8664.e2ae.96fd.33c4.d333.
- **Our client**: No counterpart by VA; Rust ownership handles the block lifetime.

### `0x458e50` — stamp a counted run of message rows (support)

- **VA / file offset**: `0x458e50` / `0x58e50`.
- **Behavior**: For each requested row, writes the CMessageBuff table, copies two header words from the template and then copies the template's 0x100-byte payload. Advances by one row (0x10c bytes) each pass and runs the unwind stub on return.
- **Reached from**: `0x457f40` and `0x4581d0`.
- **Player-visible reach**: Support; it fills rows for a later reader.
- **Evidence**: analysis ev c1b2.a8ef.4bc7.26b3.8e2f.9a31.5121.e697.9cba.e02c.d6a1.0348.81cf.cc0e.03ef.6c9b; dossier ev a64b.a904.c14a.ee6c.d3ac.2389.5515.7854.3b31.eb30.c93d.d10f.128c.2bb9.ddb0.44e4; caller facet ev 3337.d35f.427c.bca4.4964.083d.348c.c58d.c093.f852.e6c0.c2d5.f234.a6bc.9982.d4d9; callee facet ev 6d37.99c8.4e56.4f64.6b6c.ce96.5089.0303.5966.1c9c.5bfc.891b.530e.6d85.0c0e.c621; incoming xrefs ev 26bc.c620.857c.8afa.a50e.4699.cbcd.a7e7.73ce.75c6.9c74.b3da.fa69.abf3.3549.43de; body batch ev 60a0.f077.f0e0.2d9f.7a6b.8753.58e2.c973.f6d8.dee2.49fa.55bc.7738.45ee.2a31.65ab; file-offset mapping ev 1600.7a00.c1ae.4f56.0f8c.a5c3.b1b9.19d4.388b.883a.047a.d0cb.4728.89e3.ecb4.6184.
- **Our client**: No counterpart by VA; equivalent rows are `ChatState::push_lobby_user` / `push_lobby_admin` (`chat.rs:422`, `:428`).

### `0x458f3b` — clear the row-writer unwind frame (support)

- **VA / file offset**: `0x458f3b` / `0x58f3b`.
- **Behavior**: Marks the exception frame inactive and restores the saved frame pointer. It is the unwind tail of the row-stamping helper.
- **Reached from**: Only `0x458e50`.
- **Player-visible reach**: Support; no pixels or wire.
- **Evidence**: analysis ev 00d1.f8f8.22c8.f657.6325.7923.256c.fe08.2db0.bc91.7e89.b056.05fd.5ab3.151c.14a8; dossier ev 9968.7fbd.0c71.5788.21a9.bb6e.4e16.d6ad.b1b8.2603.6207.d8d7.8f99.6dfd.c899.cc45; caller facet ev f120.cd9a.14e8.4981.f58b.db4c.fef4.aa33.be90.5478.c65d.e5b2.9987.0b6f.4110.fd0d; callee facet ev c442.68de.a3cf.d67f.8010.f902.0fce.c069.ffe6.07a2.2ac8.1321.9526.66e3.50b9.bb08; incoming xrefs ev 306e.c732.e866.8997.4038.4f76.77e5.e77f.b703.8880.695b.4335.94ce.0949.5b1e.a46f; body batch ev 60a0.f077.f0e0.2d9f.7a6b.8753.58e2.c973.f6d8.dee2.49fa.55bc.7738.45ee.2a31.65ab; file-offset mapping ev 785c.2bad.ee0b.12d8.f15e.4050.a8c8.0082.172d.f603.4ab0.44ad.8b99.bc3d.306a.4de7.
- **Our client**: No counterpart by VA.

### `0x4590a0` — forward a message-row copy request (support)

- **VA / file offset**: `0x4590a0` / `0x590a0`.
- **Behavior**: Passes the four arguments unmodified to the row-range copy helper at `0x4590f0`.
- **Reached from**: Only `0x4581d0`.
- **Player-visible reach**: Support.
- **Evidence**: analysis ev f85a.86b3.3687.f724.5eed.3dd3.fd55.9799.b5e6.d34f.3c70.856e.ea91.7b67.a301.648b; dossier ev 4970.33ad.d561.6f21.e043.7187.7a49.f085.98f2.2e46.c7f8.2cfc.7e85.a5d1.3dec.65aa; caller facet ev 1fda.947f.041b.137d.635d.12e2.e168.cbcf.991d.67a5.16f6.dd02.ee8f.2598.8284.6b31; callee facet ev 12e4.5aa9.a0d5.f2b0.24b1.df95.ba31.a1ba.6e90.f9c4.11e7.b3b6.3f83.4fb4.c18e.e153; incoming xrefs ev cfaf.d496.6ccd.1f98.6ac6.d640.906a.c698.ab99.e357.54f5.16da.9382.c5b4.2613.5ac9; body batch ev 60a0.f077.f0e0.2d9f.7a6b.8753.58e2.c973.f6d8.dee2.49fa.55bc.7738.45ee.2a31.65ab; file-offset mapping ev e883.58dc.cded.b87e.d067.b0dd.2f75.7e7b.33d3.90ee.6b8a.50f2.6a1c.d9d7.fad7.5692.
- **Our client**: No counterpart by VA.

### `0x4590f0` — copy a range of message rows (support)

- **VA / file offset**: `0x4590f0` / `0x590f0`.
- **Behavior**: Walks the source range in 0x10c-byte steps and, for each row, writes the CMessageBuff table plus the two header words and the 0x100-byte payload into the destination. Finishes through the unwind helper.
- **Reached from**: `0x458f60`, `0x4590a0` and `0x484490`.
- **Player-visible reach**: Support.
- **Evidence**: analysis ev 22cc.7646.aef4.d9a6.1d96.9997.8505.a6de.a3d3.5892.cc4d.4ed9.4a59.a449.602b.3cb0; dossier ev bfb1.145e.a5dc.b44b.7311.fe6c.cffc.bde5.4140.cc89.ed07.25f2.5d1c.c34f.8e5a.45ef; caller facet ev a137.3038.1fcc.7dea.7b3c.f7cd.9859.65f1.a628.cafd.f434.f38f.0a08.cb85.f924.ed56; callee facet ev f2eb.ec04.6890.f099.40e3.7f6f.cacc.0e38.5679.aacd.2b64.c5d3.73f5.fb77.1f6e.3edd; incoming xrefs ev d51d.5e06.6f2f.e95f.639d.294c.8418.a629.79a7.5df1.b77e.8280.2449.fa2c.7c12.ea75; body batch ev 455e.0bd0.748f.19ec.8449.d306.3cb2.0f54.2967.4f7e.a787.7055.117b.9ad1.c2af.65c0; file-offset mapping ev 3945.baf5.3a45.97eb.e631.7472.44a3.7372.17a6.ef92.75c3.8d4b.4a15.f357.96c6.a90a.
- **Our client**: No counterpart by VA; Rust slice copy serves this role.

### `0x4591e0` — finish the row-copy unwind path (support)

- **VA / file offset**: `0x4591e0` / `0x591e0`.
- **Behavior**: Clears the exception marker, restores the saved frame pointer and returns the saved value.
- **Reached from**: Only `0x4590f0`.
- **Player-visible reach**: Support.
- **Evidence**: analysis ev 90cd.e861.13f4.15c2.4026.1ebd.9e82.6ac8.d7ce.899d.892f.6853.decc.37b2.3c76.9857; dossier ev 6632.1106.4a63.5c85.4cbd.a2a2.fdcc.b641.102a.f100.6122.073b.9610.a90b.8ccc.d52d; caller facet ev 92a8.9a01.8dbf.5f35.7cfc.4b02.083b.ce53.1403.aa61.d293.6dc3.8bc8.518f.c1e3.0c66; callee facet ev c746.8d7e.6204.c0a6.9c13.f5f5.f0b7.92f3.684f.e7e3.8b2b.523a.3d96.e1b5.0712.cc8f; incoming xrefs ev 3d77.348f.37fd.58ad.a429.edbe.0b85.d051.e6f8.1efc.6f47.ebf2.45fe.6382.8ddc.8e2a; body batch ev 455e.0bd0.748f.19ec.8449.d306.3cb2.0f54.2967.4f7e.a787.7055.117b.9ad1.c2af.65c0; file-offset mapping ev 8b1b.f5b7.7e85.0e76.64da.a399.2327.d976.6cbe.863f.b943.3100.1691.7100.2400.9e82.
- **Our client**: No counterpart by VA.

### `0x45e450` — close the previous web sheet, then open the cash-bill view

- **VA / file offset**: `0x45e450` / `0x5e450`.
- **Behavior**: Checks the session helper. If a web-window handle is stored, it sends `WM_CLOSE`, clears the handle and then calls the cash-bill sheet opener at `0x45e010`. That opener builds the cash-bill URL from a host table plus the user-index id, stores the new handle, sets the web-sheet flag at `+0x180`, focuses the window and shows it through `0x5a8b20`.
- **Reached from**: `0x5a5190`, the `WM_APP+0x32` arm for UI-event code `0x12`.
- **Player-visible reach**: Yes. A cash-bill sheet is created and focused after any previous web handle is closed.
- **Evidence**: analysis ev 7eb6.bbd9.9e33.248d.2533.4a0d.5f91.21b0.8694.442f.1ce3.1cfd.6c9c.68bd.1864.3840; dossier ev b459.56d4.83c6.ed41.b06f.ec1a.a19e.6ab7.5d79.a323.258f.1c56.9e20.e9b9.e3b0.4ee2; caller facet ev fc4f.61d2.850f.159b.fcca.6898.cc70.fa6d.9cfa.890a.647a.a6d6.f59c.b45f.e12a.b894; callee facet ev 50cf.9e06.2a67.adc1.34aa.c9eb.efd1.3743.7406.813b.308a.1b5c.98e1.a1c6.0dac.e59b; incoming xrefs ev ec89.8edd.25f3.57dd.8cd4.807d.98e4.d41f.bf24.3d7f.5a51.513b.43fa.a1d1.3e7c.af57; body batch ev 455e.0bd0.748f.19ec.8449.d306.3cb2.0f54.2967.4f7e.a787.7055.117b.9ad1.c2af.65c0; file-offset mapping ev bc95.7d6f.5f55.b519.50e8.27d5.aad3.da46.6af2.ae7b.51aa.403e.2a61.0bb3.f49f.e010; cash-bill opener ev 19f3.affc.ff45.cd42.ec21.a60c.c06c.fed2.a7c7.cef1.28b7.a981.b2ea.6a56.0ccc.f8b0.
- **Our client**: No counterpart by VA. Embedded web clients are deliberately blank (`crates/o2jam-shell/src/screens/game/mod.rs:9018`, `:20619`); the shell has no web bridge.

### `0x45ecd0` — close the previous web sheet, then open the bingo event view

- **VA / file offset**: `0x45ecd0` / `0x5ecd0`.
- **Behavior**: Uses the same close-then-open shape as `0x45e450`, but calls `0x45ea60`: a fixed bingo event URL with the user-index id is stored in the global web handle, the sheet flag is set and the window is shown and focused.
- **Reached from**: `0x5a5190`, the `WM_APP+0x32` arm for UI-event code `0x13`.
- **Player-visible reach**: Yes. It opens the bingo event sheet after closing the previous web window.
- **Evidence**: analysis ev 7be8.66b8.2ee6.3156.3bab.d1e0.bd58.3755.063a.84b7.481d.4033.d512.9c40.97fb.4d87; dossier ev f91a.bbea.0557.8e94.8fa5.c2bf.caba.bb64.27bc.981e.0ac4.d326.2cb8.929e.b045.aa23; caller facet ev b61b.12f2.b3f1.18ce.d646.5927.1204.0b28.12c9.d78a.8d2f.e915.3b45.863f.07d6.bca4; callee facet ev 1d8c.5e5b.bf95.c934.6e24.76d9.152a.c033.9a11.1e17.114e.7ef4.2dfd.cb1a.32a8.11c5; incoming xrefs ev 5ee6.f818.ee13.f29c.6073.fd63.1bb4.5fb6.bee9.8413.2011.d3ae.79ee.37cc.5764.5c5a; body batch ev 9d66.8620.996a.8fc5.195b.b503.c5b4.0c43.2b5e.d79c.6cbd.ec91.b7f3.a454.6744.13db; file-offset mapping ev dc07.a055.b4cf.df84.e8d1.67a4.f389.02a4.ddd4.bb15.e347.a02e.9f11.c5c3.9f1b.6a6a; bingo opener ev 19f3.affc.ff45.cd42.ec21.a60c.c06c.fed2.a7c7.cef1.28b7.a981.b2ea.6a56.0ccc.f8b0.
- **Our client**: No counterpart by VA; the shell leaves embedded web clients blank (`crates/o2jam-shell/src/screens/game/mod.rs:20619`).

### `0x45f480` — close the previous web sheet, then open the in-game Q&A view

- **VA / file offset**: `0x45f480` / `0x5f480`.
- **Behavior**: Closes any stored web handle, then calls `0x45f190`. That body reads a rectangle from the ControlList record; in scene 2 it shifts the rectangle by 50 pixels before building the 1:1 Q&A URL, stores the new handle, sets the sheet flag and shows/focuses the window.
- **Reached from**: `0x5a5190`, the `WM_APP+0x32` arm for UI-event code `0x14`.
- **Player-visible reach**: Yes. It opens the 1:1 Q&A sheet; Playing uses a shifted rectangle.
- **Evidence**: analysis ev 286d.c423.cbdb.5748.5c95.d5c3.2cbc.843b.cf47.a138.f12f.b237.8872.89b8.73c5.4b98; dossier ev a3dd.48b4.4aaa.8419.f681.0491.15db.bedb.d416.f333.c9cc.10ed.e04c.163e.53c2.244f; caller facet ev 3d91.f9d0.5694.55d4.fca8.1247.ecca.2168.9f64.6010.127a.d972.2ef5.b851.7266.7b5a; callee facet ev fefd.0dec.e355.5391.7192.167a.cfaa.0190.bc9a.9342.f248.480e.efd7.4537.964b.ffea; incoming xrefs ev 867d.c17a.6bff.c879.3fe5.32da.3468.e726.2c47.6367.6d7c.b1ba.8a42.9329.0e26.7847; body batch ev 9d66.8620.996a.8fc5.195b.b503.c5b4.0c43.2b5e.d79c.6cbd.ec91.b7f3.a454.6744.13db; file-offset mapping ev c145.a2d8.2629.3ebe.a509.3487.7c23.8933.2970.344a.31b8.6ca9.b509.5b7d.c050.470a; Q&A opener ev 19f3.affc.ff45.cd42.ec21.a60c.c06c.fed2.a7c7.cef1.28b7.a981.b2ea.6a56.0ccc.f8b0.
- **Our client**: No counterpart by VA; no embedded-IE host exists in the Rust client.

### `0x5148e0` — install a window-subclass procedure once (support)

- **VA / file offset**: `0x5148e0` / `0x1148e0`.
- **Behavior**: Calls the companion initializer and, when the global slot is still zero, installs the procedure at `0x518230` on the main window with `SetWindowLong`. Always returns 1.
- **Reached from**: Only `0x514990`.
- **Player-visible reach**: Support. It installs the callback that later routes input; no direct paint.
- **Evidence**: analysis ev 1791.1a47.9a33.39c1.0523.7536.b58f.adfa.8c5a.2f1d.53be.624b.fcfa.8f81.054f.4920; dossier ev 0d6a.c704.514a.7060.3761.0a74.e68e.cb2d.2a13.cb0f.5f40.5fe0.f7b5.c587.9f98.c4c2; caller facet ev 2284.6db8.ab18.2627.1c6c.e07a.f840.b69f.844b.caed.983c.8163.83c2.dc37.cfff.31bb; callee facet ev e872.57f2.dec4.98ab.d4d5.0689.ae4e.9f29.faf1.ad20.31e9.a7d1.f7f7.3ad9.74da.1497; incoming xrefs ev 7f9a.b45f.1c9e.6fa4.80a6.f764.45a1.c4c5.5580.d450.41b6.e7dc.e8d2.c6ea.c67f.089d; body batch ev 9d66.8620.996a.8fc5.195b.b503.c5b4.0c43.2b5e.d79c.6cbd.ec91.b7f3.a454.6744.13db; file-offset mapping ev ce93.f54f.9466.c5dd.c5ff.eb79.7dbe.e2ce.ecd2.0682.3b43.ba18.4c2a.1ff8.4b81.2dd3.
- **Our client**: No counterpart by VA. Window events arrive through winit at `crates/o2jam-shell/src/window.rs:554`.

### `0x55c790` — 2023 RoomParameterChanged: stage one member word and post the event

- **VA / file offset**: `0x55c790` / `0x15c790`.
- **Behavior**: Reads a four-byte packet field (its low word is the target index), then two one-byte fields. It masks and recombines them into a word at the per-member slot `0x76fd24 + index*0xb4` when the index is in range, and posts opcode `0x7e7` (2023) with that index to the UI queue.
- **Reached from**: The packet dispatcher `0x558080`, case `0x7e7`; call site `0x558527`. Dispatcher dossier: ev 5032.6588.ab15.2e39.7a49.5695.3331.7425.09e6.8c5e.caaf.afe3.2518.03ff.782c.8ae9.
- **Player-visible reach**: Yes, as support for room chrome: the staged word is the input to a later UI row, but this body paints no pixels itself.
- **Evidence**: analysis ev dce9.1ea8.d80f.e0bd.0ae3.39f0.1dcf.0433.d231.2026.de8b.b0ce.87d3.356f.1230.7848; dossier ev 64b5.ecb3.edc5.91dc.4db0.6d9f.4240.38e6.2e60.1b86.4d24.3a30.1341.d714.0abb.c217; caller facet ev de70.977b.49eb.3a66.7dc4.4445.80f2.2f7b.abd3.6c37.2f43.4083.d9e9.9562.c759.2527; callee facet ev e198.13fb.ff9e.de30.c1ca.046b.9ea2.cf39.b11c.0729.36db.f452.5dbf.076a.dd74.27c2; incoming xrefs ev 1640.71b6.307f.97bb.ac49.ffd3.c90c.4f6a.f61d.c5e1.b96d.a391.3f7e.e7ca.2818.8bd3; body batch ev 8a3c.8794.0ca7.4284.4345.f48c.9151.8558.2203.5c90.59f0.d9e6.9573.ebdd.44da.77d4; file-offset mapping ev 5daf.0ba7.3f7e.d1a0.941b.e658.9410.502d.7426.fa84.352b.8174.48a4.8d6d.a0cd.91da.
- **Our client**: No codec or apply arm for 2023; `docs/dispatch-inventory.md:67` still lists the body layout as OPEN.

### `0x55cad0` — 3003 JoinWaiting: stage the joined room or show a refusal

- **VA / file offset**: `0x55cad0` / `0x15cad0`.
- **Behavior**: The success case reads a member count, a song word, option bytes, seven seat-enable bytes and skill/member vectors into the Waiting globals, then sets mode flags. Negative result cases post one of seven CP949 refusal captions and send opcode `0xbbd` (3005) to leave the room; two cases also clear a global sheet flag.
- **Reached from**: The packet dispatcher `0x558080`, case `0xbbb` (3003); call site `0x5585af`. Dispatcher dossier: ev 5032.6588.ab15.2e39.7a49.5695.3331.7425.09e6.8c5e.caaf.afe3.2518.03ff.782c.8ae9. Caption bytes were read statically, not from a live frame.
- **Player-visible reach**: Yes. Success enters and stages Waiting; each refusal shows a popup and returns to the prior scene.
- **Evidence**: analysis ev 62dc.049e.5d65.45ca.23f8.81a2.f397.2900.1d50.0bb9.7b81.5b82.3c30.ce81.3bcf.b8fb; dossier ev c281.07bb.cd3d.1028.2656.51ea.aec5.fec6.5a27.b527.a3ff.af66.8be1.4d88.90ce.4e2a; caller facet ev f2e8.b094.70f9.83f1.445b.1749.d3c5.a33f.0859.eabc.f22f.7fb0.acab.536a.1c18.a56a; callee facet ev c8f7.aa15.db68.fc32.7875.a721.297b.7ff8.a6e9.07c5.2e8b.6c3a.0197.a732.ad48.6415; incoming xrefs ev 3835.27b7.c7a4.76bd.517c.60ed.fbf6.819b.af6b.6c10.b469.5899.a076.476f.5d80.734f; body batch ev 8a3c.8794.0ca7.4284.4345.f48c.9151.8558.2203.5c90.59f0.d9e6.9573.ebdd.44da.77d4; file-offset mapping ev 456f.7c54.29be.638a.c1b4.2178.c340.1df6.5e4f.bf77.4faf.26ef.031d.5d43.507b.cc91; refusal caption at 0x64d928 ev f1e9.c032.eaa2.5a2f.207f.fb1c.6585.75da.11c2.1219.c902.68e5.57b2.7f0e.1677.9338; refusal caption at 0x64d958 ev f334.3c7c.1b71.4284.b6ea.3c3d.b82b.4261.2d44.d152.a6d0.b7ba.65fc.dfdf.e477.371e; refusal caption at 0x64d988 ev 21ee.bb13.e0e0.48d5.6918.9d45.abc4.b4a2.e62f.dc4a.f45e.9ffb.cd55.3739.8737.bae7; refusal caption at 0x64d9a0 ev f31a.a383.a4c4.b495.0cba.8721.611f.245c.dc7d.114d.8621.09da.9bb5.6caa.8ecd.b061; refusal caption at 0x64d9b8 ev 472b.21c3.e32d.b77d.e598.7735.e730.6a5c.d9e0.9da1.efa3.bfaf.13de.ed9c.d102.0b01; refusal caption at 0x64d9d0 ev 9ad6.d501.ae32.8135.2a30.58a8.2bf7.2595.2b2e.22cb.f925.5ed1.d8b3.cc79.5e49.fdbf; refusal caption at 0x64d9f4 ev 1ab7.60fe.7366.ae09.0a8a.d3cc.3f52.d797.ab98.568f.854c.334a.4d02.bc4b.326b.d41f.
- **Our client**: Codec `JoinRoom` 3003 at `crates/o2jam-net/src/rooms/join.rs:376`; live apply through `crates/recomp2jam/src/session.rs:1005` and `:2246` into `ScreenGame::set_joined_room`. Refusal captions have no staged counterpart. Guard: `live_room_quickjoin_click_posts_3002_from_boot` (`crates/recomp2jam/src/live_input_legs.rs:1462`, Tier 3).

### `0x5630d0` — build the 6000 legacy-ping message (support)

- **VA / file offset**: `0x5630d0` / `0x1630d0`.
- **Behavior**: Fills a CMsg record with opcode 6000, a four-byte length and no payload, then posts it to the message sink. The body never reads the socket itself.
- **Reached from**: Only the packet dispatcher `0x558080`, the exact 6000 case at call site `0x55892f`. Dispatcher dossier: ev 5032.6588.ab15.2e39.7a49.5695.3331.7425.09e6.8c5e.caaf.afe3.2518.03ff.782c.8ae9.
- **Player-visible reach**: Support keepalive. It emits a message, not a pixel.
- **Evidence**: analysis ev f7fb.5aec.317d.4c10.d212.315d.a122.e3d3.4344.8cee.0bd7.78b4.a3fc.ea03.74d3.9cf6; dossier ev 59de.2ee2.11b1.46bb.736d.9a97.6b06.bea4.be4b.49a1.7dfe.1820.6242.bdff.f08b.ac35; caller facet ev 2ea1.b231.1516.a2af.a8aa.3c09.d30f.e804.597a.6090.8c85.2156.5a23.d02d.95cb.1691; callee facet ev 72cf.3533.ee1e.6b84.681f.26fa.5176.ceb0.8a1a.f888.f256.cee0.0a38.56b5.1e2a.cba5; incoming xrefs ev 2be0.8c34.653a.f8cd.5e39.8248.24f4.a86a.1232.3501.c23f.49f8.28a1.4d16.167b.2156; body batch ev 8a3c.8794.0ca7.4284.4345.f48c.9151.8558.2203.5c90.59f0.d9e6.9573.ebdd.44da.77d4; file-offset mapping ev 4286.0014.f4b8.9c48.7577.a294.aa5c.f374.7a45.f538.85cc.ce8d.feac.67df.c64b.4dc0.
- **Our client**: No receive-side driver by VA. The opcode enum has `Generic::LegacyPing` at `crates/o2jam-net/src/command.rs:70`.

### `0x563db0` — stage the 4049 latency answer for the 4048 UI message (support)

- **VA / file offset**: `0x563db0` / `0x163db0`.
- **Behavior**: Calls the 4049 reader at `0x586470`. When the stored room pointer is live and the member index matches, it writes the index, result, a fresh clock value and a tier flag into the latency record, then posts opcode `0xfd0` (4048).
- **Reached from**: Only the packet dispatcher `0x558080`, case `0xfd1` (4049); call site `0x5587e0`. Dispatcher dossier: ev 5032.6588.ab15.2e39.7a49.5695.3331.7425.09e6.8c5e.caaf.afe3.2518.03ff.782c.8ae9.
- **Player-visible reach**: Support for a visible ping/status readout. The wire body is already pinned in `docs/pins/net-c2.md`; this pin adds the staging wrapper only.
- **Evidence**: analysis ev e70e.74ff.db35.b78c.544f.ee3e.807a.1385.7cde.9965.c7bb.96df.281a.1265.46e3.52ec; dossier ev 6d57.6ca3.7bb9.35a5.e05e.f91b.e25d.2422.ebeb.a2bc.5300.7ae4.f2ab.ce66.3294.3d37; caller facet ev 5784.1592.8030.c3e5.c998.9f5d.7f53.c660.7e69.c2a1.f012.fc13.a131.b768.0b1c.1eed; callee facet ev 8581.a65f.4399.0c18.e54b.0a77.57c0.a92d.e4e4.ed12.892b.1f7f.c321.2c51.2d7d.1ec3; incoming xrefs ev 4e5a.ae49.c95d.37b0.3303.90e9.b545.b2f4.3a86.728b.8c26.1af1.3134.0606.fb12.d1a1; body batch ev bd33.1da2.69f7.3c6d.32ce.949e.c13b.9e51.2db5.2133.3873.dfe8.4140.af57.74b1.f280; file-offset mapping ev a3f0.a9b2.3054.f1de.3d0a.3a3d.a069.dc0b.051e.f9c8.a005.d27a.1022.4aa9.d612.10a8.
- **Our client**: Codec `ReportUdpPunchHoleAnswer` and the 4049 reader are described in `crates/o2jam-net/src/relay/commands.rs:673` and `docs/pins/net-c2.md:398`; no UI apply guard by VA.

### `0x58d580` — run the UDP hole-punching start sequence

- **VA / file offset**: `0x58d580` / `0x18d580`.
- **Behavior**: Walks three peer tables with 6-byte and 4-byte strides, calls a punch helper for each entry, writes a success or failure caption through `0x58ff70`, waits on a readiness poller in 300 ms sleeps, sends opcode `0xfce` (4046), and sets the completion flag.
- **Reached from**: Only `0x58d030`.
- **Player-visible reach**: Yes. The UHP start text is written to the visible caption sink and the success/failure branch is staged in a global flag.
- **Evidence**: analysis ev e8ab.954b.f7d6.fbb8.9a20.6214.83b1.3652.7784.512e.8cb4.649d.d712.8c99.0c68.14e6; dossier ev 4bbd.440f.02ed.021a.4806.d1d3.e492.6598.6928.e85d.de37.e2d7.8cc4.de32.ee60.7cb1; caller facet ev 4e18.15c5.6475.f399.a46c.043a.c276.c3c9.ce4e.17f1.13ce.d64e.7cfd.c1b2.8de7.c3ed; callee facet ev 88ed.5acd.12bf.edc4.221a.d355.b748.1ad1.6614.621d.a8e4.7bda.4f34.3719.7c17.5f01; incoming xrefs ev 1898.e1da.0e40.11b1.5345.ef5d.366a.76d9.cfc1.cc12.547d.7fbd.2ea3.c0dc.f48c.0873; body batch ev bd33.1da2.69f7.3c6d.32ce.949e.c13b.9e51.2db5.2133.3873.dfe8.4140.af57.74b1.f280; file-offset mapping ev a71e.1646.2ec6.8481.8dd0.b8e3.cc99.9e02.a503.611c.513e.8558.9d6f.217b.6170.5390; text sink caller ev fb98.4805.64b8.3528.658a.eec0.855b.d638.bf3e.8c6b.38bf.2646.cabb.e878.34ea.d6b3.
- **Our client**: Codec `ReportUdpPunchHole` 4046 with a byte-verified empty body at `crates/o2jam-net/src/relay/commands.rs:643`; no caption pump or UHP loop exists in the Rust client.

### `0x59025a` — finalize a text popup or log line (support)

- **VA / file offset**: `0x59025a` / `0x19025a`.
- **Behavior**: Copies a style word from the text object, ORs a bit when the underlying data slot is absent, applies the style through `0x508df0`, then runs the two refresh helpers. Called at the end of the text-writing helper.
- **Reached from**: Only `0x58ff70`; text sink dossier: ev fb98.4805.64b8.3528.658a.eec0.855b.d638.bf3e.8c6b.38bf.2646.cabb.e878.34ea.d6b3.
- **Player-visible reach**: Support for visible captions; no text is chosen here.
- **Evidence**: analysis ev da50.1243.deca.2322.70ea.a59c.8c77.e0a0.e288.f644.9ec6.7b10.391c.565b.8895.10a5; dossier ev 1cd2.eeaf.396b.2524.6d56.0ffb.6f89.39ff.85d8.98ff.321e.a760.7d65.0024.85eb.38f3; caller facet ev 0381.b469.6775.880b.b646.95f0.7743.89cb.c669.7a83.557a.41d2.b013.b9e0.6ead.e4e1; callee facet ev 8bdf.104e.2dcc.5d57.b789.786c.e999.1cc7.0535.a4bc.e6cd.b9a5.3c81.884b.3631.a709; incoming xrefs ev 3938.742f.4722.9088.7636.50dc.fd91.cf9e.2332.1a04.e51d.a8f5.9651.d7a1.d6fc.6371; body batch ev bd33.1da2.69f7.3c6d.32ce.949e.c13b.9e51.2db5.2133.3873.dfe8.4140.af57.74b1.f280; file-offset mapping ev 2186.178d.293d.6cb3.45f4.b5aa.3705.f45f.5216.6f30.d6ea.2cca.63d4.8664.692e.45b5.
- **Our client**: No counterpart by VA; the Rust side has no equivalent caption sink.

### `0x590320` — copy a run of six-byte peer records (support)

- **VA / file offset**: `0x590320` / `0x190320`.
- **Behavior**: Steps through the source range in six-byte increments, copies each record through `0x5afbb0`, and returns the destination end computed from the record count.
- **Reached from**: `0x58f310` and `0x5b3720`.
- **Player-visible reach**: Support for the UHP peer table; no pixels or wire.
- **Evidence**: analysis ev 2f9c.d1dc.60de.4a31.1c96.6317.a817.71b9.f3ee.a388.efaf.6739.cecb.9b17.b775.9e27; dossier ev cad0.c1b5.5002.00e3.4145.b0f4.88d0.cb5b.eca0.52bd.3773.c016.de3d.1eb1.af6a.8a30; caller facet ev 12c7.cc28.6de6.4b62.63e7.9b63.2ad7.ceef.9f75.a570.c885.21ce.0b30.fc69.9c5d.dfb3; callee facet ev 2689.a873.0f3b.7cbb.c275.a1e4.8ae4.6f50.a867.e4a4.c969.8a11.b424.ec25.0a2c.3b7f; incoming xrefs ev 6ddd.3dc3.531f.2dfc.05a5.7cf9.5ede.9cab.3611.d53f.265f.4346.273c.b444.b72a.1492; body batch ev 501b.854d.8bb4.d951.841d.9a62.55e4.2725.6fe1.6bf8.5b8c.7f47.df7e.b769.cff8.74bd; file-offset mapping ev 3ab0.007b.ea5b.6c39.342e.40f0.f325.8dc4.7a89.0ce0.3920.4c17.df30.2329.00d2.2805.
- **Our client**: No counterpart by VA; Rust slice copy covers this role.

### `0x5903c0` — move a range of four-byte words forward (support)

- **VA / file offset**: `0x5903c0` / `0x1903c0`.
- **Behavior**: Computes the word count between two pointers and calls `memmove` into the destination, returning the end pointer. It acts even for a zero count.
- **Reached from**: Only `0x58f4e0`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev 2878.1574.1b94.eeb0.d955.2d64.addd.1481.c67e.4e39.50b9.d4fb.45b2.3b68.68cd.4af9; dossier ev d7f8.0913.0236.aced.4f50.2d9d.e070.18c3.922a.98cb.c8ec.3e25.fbfc.4757.9fb9.e70c; caller facet ev b8d0.bc60.9d12.a505.36ca.7b75.89c2.ce79.159e.4b18.2ff2.1d8b.9aea.6448.449e.ba40; callee facet ev 82fa.37b6.66f7.0e85.0b29.d1d6.15df.dc31.7bb6.2415.c3a4.8358.172e.d2a2.0368.91be; incoming xrefs ev bb3e.4092.42a6.b390.824a.73d2.e574.da55.3d71.68c3.18ee.3b8a.61ed.ae47.9bd8.4647; body batch ev 501b.854d.8bb4.d951.841d.9a62.55e4.2725.6fe1.6bf8.5b8c.7f47.df7e.b769.cff8.74bd; file-offset mapping ev 8ce4.38e9.d510.9d1e.f018.0676.abef.0bbb.0ec4.5e75.27af.55ac.0b63.5180.9730.b516.
- **Our client**: No counterpart by VA; `Vec` handles this.

### `0x590590` — fill four-byte words from a source (support)

- **VA / file offset**: `0x590590` / `0x190590`.
- **Behavior**: Copies one word per iteration from the source into the destination for the requested count; leaves the source untouched.
- **Reached from**: `0x58f160`, `0x58f6b0` and `0x58fe40`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev abc3.ce61.1842.0cbf.f4f2.440d.b89c.c854.2d99.b632.92be.e147.d721.9d4f.d3d6.6c93; dossier ev c9c8.3ebb.5eed.cda7.de39.a108.6a57.7382.014f.b9df.163b.45af.bf71.0fee.2305.6857; caller facet ev 7d4b.8d97.9c31.7585.38c1.e30b.5e68.7e65.8ed4.5d9a.3998.df45.ac47.5262.b7f6.abb3; callee facet ev 7b17.fe7d.b835.c546.1114.4f68.3a40.ff0a.8f56.fb3b.89fa.4690.effa.ad05.71a4.86ec; incoming xrefs ev cc1d.ed28.455e.a746.3444.09a8.36b3.cd11.9f9e.fa63.cbb2.259c.c53c.15bb.fe32.4b15; body batch ev 501b.854d.8bb4.d951.841d.9a62.55e4.2725.6fe1.6bf8.5b8c.7f47.df7e.b769.cff8.74bd; file-offset mapping ev 03b8.9d7f.0ad6.10f8.4381.7c40.9488.cc52.f778.8ae6.d300.99c8.531b.ba4d.426f.2863.
- **Our client**: No counterpart by VA.

### `0x590600` — move a nonzero range of four-byte words (support)

- **VA / file offset**: `0x590600` / `0x190600`.
- **Behavior**: Same forward `memmove` shape as `0x5903c0` but skips the call when the count is zero.
- **Reached from**: `0x58f6b0` and `0x590540`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev e470.23c4.df7e.ee58.73a1.6855.91ed.d0e3.26f3.768f.b001.9a0c.8eed.2dd9.a880.3e64; dossier ev 2908.9c5d.6d68.52cd.e1e0.3a25.f497.1618.dbc3.4cf3.2ca3.88d2.7285.3831.69b9.0244; caller facet ev f37f.965b.8810.7792.2455.3836.09d8.e6fe.21e8.82b2.3a1e.22f2.620a.b05d.a5aa.66be; callee facet ev 4ad0.84d1.be00.3c34.b922.e4ed.549a.e4b6.f1de.4780.94d1.a9f4.318f.d328.0cb9.6d47; incoming xrefs ev be1f.7ec0.c1e9.af14.2ee9.177f.3f41.eaa5.457b.4b5f.f4a6.5fd6.04d1.ebe2.3316.5242; body batch ev a3d5.af6a.cb1e.dc16.dda1.f6af.19de.d3bd.f728.ea45.960c.aacc.2029.447f.6762.c62d; file-offset mapping ev eb70.4e15.58b3.c512.0d18.26b8.1bfc.20c3.5a73.c1d7.f40d.ccc3.7d2f.6376.5d62.7b9c.
- **Our client**: No counterpart by VA.

### `0x592380` — initialize the UHP sender and its static state (support)

- **VA / file offset**: `0x592380` / `0x192380`.
- **Behavior**: Calls the lazy state initializer `0x5b02a0` first, then the sender setup `0x5b01c0`, which enters the UHP thread setup path.
- **Reached from**: Only `0x586b50`.
- **Player-visible reach**: Support for the UDP hole-punching sender; no paint.
- **Evidence**: analysis ev 2fd8.ee65.9494.41c1.7193.0785.870b.b7e0.4469.5e14.c93a.d9f0.b603.b144.d08e.f89f; dossier ev 4b15.9233.2688.75c7.07ab.458c.c859.4c03.9fe1.8176.f9ef.ec65.9347.858d.2cb2.7f7e; caller facet ev 0a51.b83f.adc8.ba4e.e15c.ea1a.41cd.aca2.18c5.3ec9.9ca6.a844.7060.7766.3fa5.b0f6; callee facet ev bcd6.4ddf.9c57.f421.dff5.5321.a697.8a9e.1b73.afed.adbf.9899.d7b1.d434.ce25.15c7; incoming xrefs ev 1500.2f1c.c212.8a03.de3a.96ab.47a5.b474.8bf3.55ba.4db6.b486.8f2d.39f7.4615.ddc4; body batch ev a3d5.af6a.cb1e.dc16.dda1.f6af.19de.d3bd.f728.ea45.960c.aacc.2029.447f.6762.c62d; file-offset mapping ev e2c7.7e48.3bec.1be0.e560.3ddc.52d3.2dd6.d9cd.516f.931d.6312.0f30.9821.f8d8.69af.
- **Our client**: No counterpart by VA; the Rust relay has no UHP thread.

### `0x5a35c0` — show an error box from a supplied message

- **VA / file offset**: `0x5a35c0` / `0x1a35c0`.
- **Behavior**: When the enable flag is set, formats either a bare string or a string plus numeric code and context into a 260-byte buffer, asks the popup-or-close helper `0x5a8ab0`, and displays a `MessageBoxA` when that helper returns zero.
- **Reached from**: 22 direct callers across display init, dialog and shop code, including `0x40a5e0`, `0x411b70` and `0x52f9f0`.
- **Player-visible reach**: Yes. A failure can produce a modal error box with the supplied text.
- **Evidence**: analysis ev 571e.ba13.e0ff.24af.d973.211a.6bbb.422c.2b9e.c94a.8ac3.f942.7e7f.1433.9706.4d3e; dossier ev 8a35.12a0.40dc.5103.a1b1.0cc6.351c.5f8c.e06e.74b7.a77a.508b.1c3e.bca5.1371.186a; caller facet ev ffa8.cc1d.1b67.4731.6083.d539.858c.08b3.bb9c.319f.6618.40a7.6018.42ad.952a.0823; callee facet ev 9d06.fd30.3bc5.7297.e2b5.0e8f.e869.4073.f180.5892.1f7c.a5cc.7ceb.f243.b0f0.7eaf; incoming xrefs ev 2752.86c2.b70a.5264.b7de.d5d4.a92e.2ba9.2eaf.efb3.b818.9d21.e2c6.e0b8.1971.d9b1; body batch ev a3d5.af6a.cb1e.dc16.dda1.f6af.19de.d3bd.f728.ea45.960c.aacc.2029.447f.6762.c62d; file-offset mapping ev 9195.0529.67c1.0077.1473.c353.55de.c8f6.c456.3744.090e.e859.2bb6.b796.65c7.26a2; window hub context ev 1981.b210.2ccc.fee7.e1cb.6fa0.5edf.430c.720b.6bfb.b177.20ce.e461.70f7.fecf.5ec0.
- **Our client**: No MessageBox equivalent. The shell's quit/confirm state lives at `crates/o2jam-shell/src/screens/side.rs:7499`; generic error popups are unwired.

### `0x5a36d0` — show an error box from a numeric error code

- **VA / file offset**: `0x5a36d0` / `0x1a36d0`.
- **Behavior**: When the code is nonzero, resolves it to an English message through `0x5a3820`, optionally appending a code and context, asks `0x5a8ab0` to popup or close, then calls `MessageBoxA` when the helper declines the popup.
- **Reached from**: 31 direct callers, among them the display helpers `0x413030`, `0x414bc0`, the window resize path `0x416a80` and popup wrappers.
- **Player-visible reach**: Yes. Display or surface failures surface as a modal box with the mapped text.
- **Evidence**: analysis ev 3370.0e96.16b1.e02a.874c.e3a4.8d7a.e24d.46c2.b16d.e66e.1b23.902d.5346.ef0c.5441; dossier ev a7d6.cf64.73a2.49bc.3ea6.8f96.d633.4adf.282c.1def.73d9.a721.094d.b1f7.29d9.9ae8; caller facet ev 4c80.b0ff.2010.8c0e.c840.b145.72ae.5582.0aa8.0c50.8504.47b3.ed90.2791.96cc.a3cd; callee facet ev 0510.ad80.9add.0da0.7f26.e2cc.474f.e429.5190.9f0e.3693.450d.50b2.bb55.a309.32ca; incoming xrefs ev 3155.06e5.3a57.18f4.fc4f.73ba.4888.e0b1.cf4f.5a4e.7462.3f1b.8f93.5ff9.a263.4aea; body batch ev fd40.e114.4072.8d5a.cabf.d575.fc41.0995.c6ff.5c4a.d283.859d.1a5e.7c17.9f9e.ce74; file-offset mapping ev 972a.cfb5.dbab.3929.e00e.ea9c.6e69.f95e.e96b.4fdf.dc90.e9ec.eb7d.c683.f922.eb16; window hub context ev 1981.b210.2ccc.fee7.e1cb.6fa0.5edf.430c.720b.6bfb.b177.20ce.e461.70f7.fecf.5ec0.
- **Our client**: No counterpart by VA; `lib.rs:48` documents the same 800x600 init site but no error-box path.

### `0x5a3820` — map a DirectDraw/Direct3D error code to text (support)

- **VA / file offset**: `0x5a3820` / `0x1a3820`.
- **Behavior**: A branch table over negative HRESULT-like values returns English strings for pixel-format, surface, palette, device and memory failures, or a generic 'Unrecognized error value.' fallback. It is the text source for the numeric error box.
- **Reached from**: Only `0x5a36d0`.
- **Player-visible reach**: Support for a visible popup; this body chooses text only and paints nothing.
- **Evidence**: analysis ev 8b93.2964.2608.6a22.ffe3.9355.19fe.29cf.1cc2.9619.6096.b193.073d.4706.58c8.1aed; dossier ev f8fb.d38b.1350.608a.218f.ccdf.e5ab.8d86.9dad.4617.245b.8e33.b09e.7ba9.b17f.51b0; caller facet ev f11f.99cc.58e5.1abe.6d20.d3c4.a351.8609.8d1d.f2b1.dff5.5ac2.33fd.3ed4.5a8f.b74e; callee facet ev 47dd.f9d6.c75d.e563.5730.c2b1.f3c3.99d0.ea09.1f77.086f.bca5.4ea2.243a.447b.fb6b; incoming xrefs ev 5529.7548.f1af.d2a7.ce4c.a5f6.8b70.1d53.1cd8.4df9.0df0.8cfe.2e22.6854.e714.a8fb; body batch ev fd40.e114.4072.8d5a.cabf.d575.fc41.0995.c6ff.5c4a.d283.859d.1a5e.7c17.9f9e.ce74; file-offset mapping ev e974.6786.2372.4ae4.8b3e.29be.1586.86d1.e067.2b0f.40c5.a274.aafb.8755.6a6b.b195.
- **Our client**: No counterpart by VA; Rust errors use `Result` and shell logs, not HRESULT text mapping.

### `0x5a4ac0` — store and apply the requested cursor mode

- **VA / file offset**: `0x5a4ac0` / `0x1a4ac0`.
- **Behavior**: Writes the request into the object's `+0x20` word and calls the shared cursor applier `0x431f70`, which clears or sets the Windows cursor and can switch to a stored image cursor.
- **Reached from**: `0x477990` (display init), `0x544aa0`, `0x546bc0` and `0x5474a0`.
- **Player-visible reach**: Yes. The pointer image changes on apply.
- **Evidence**: analysis ev 2ced.dd82.e803.a363.670f.b6e7.8425.8d0d.2ce0.4faf.5054.be0b.6891.ffff.a21c.54e1; dossier ev bebd.d4e8.a4b6.33aa.f71e.ce29.4764.afc1.7592.cdf4.d9ef.4209.2197.08c2.ec1f.8e63; caller facet ev 530b.7429.3386.e175.7853.3064.c9e4.81cb.ae92.535f.235a.b452.769a.facc.ff35.13e0; callee facet ev 5fa4.7243.ecf3.97a1.9cec.7f74.f4ee.69ee.c4f7.b10b.5c2d.9992.953e.0d73.f087.8da1; incoming xrefs ev 3435.baa8.c7fd.799c.d8d8.8eb9.88b1.0c9d.44ad.1284.85e3.4cf8.8eb7.2f26.953c.419e; body batch ev fd40.e114.4072.8d5a.cabf.d575.fc41.0995.c6ff.5c4a.d283.859d.1a5e.7c17.9f9e.ce74; file-offset mapping ev 823b.5a52.e933.7842.00c2.aa0e.5178.4a2b.7a38.383e.94bd.4cdd.3d6b.2115.e9fb.10c9; cursor applier ev eaac.be9d.2281.2dfb.e907.ff3f.a467.441f.9b7b.94b3.3fd9.39bc.1c54.4304.2fef.698f.
- **Our client**: No runtime cursor mode in the Rust client; the amber cursor in `crates/recomp2jam/src/shots.rs:119` is a shot stand-in. Winit events reach `crates/o2jam-shell/src/window.rs:554`.

### `0x5a4b10` — write the cursor-mode flag and clear custom art (support)

- **VA / file offset**: `0x5a4b10` / `0x1a4b10`.
- **Behavior**: Forwards the requested mode to `0x4321b0`, which stores it at `+0x24` and clears the custom cursor and image resources when a nonzero mode is selected. This is the arm used by the pin for F8 in `docs/pins/net-c2.md`.
- **Reached from**: `0x4c1910`, `0x4c2390`, `0x4c47a0`, `0x5a49d0` (F8 option) and `0x5a5710`.
- **Player-visible reach**: Support for the visible F8 pointer switch; already covered by the F8 pin in net-c2.
- **Evidence**: analysis ev 54be.be49.de14.092a.4b0f.865c.193c.ccc2.87d1.f502.9b8f.9ded.c698.bcd1.40f3.aec5; dossier ev 4f0b.df04.2502.a80f.23e4.8668.b07f.21d6.8897.291e.cf6c.f9ca.5e8b.39c5.6afe.4c0b; caller facet ev 732b.d3e3.11ec.af9d.68f3.82a5.c868.9a8c.1917.a6f5.cbcb.6206.541a.36f1.e388.0c66; callee facet ev 50b1.022f.441c.c980.697b.8771.5f22.9583.0df3.8cdb.de1b.4079.d6dc.4d10.6658.58f4; incoming xrefs ev a784.7d83.cf1d.50ad.1865.680d.b4bb.6680.ac4b.764d.d208.9a2c.87a0.524e.1e6a.5228; body batch ev b6be.ce0b.566b.f4a2.1eab.5093.814a.c72e.a8ad.4bad.ee02.6d79.be9e.d006.37d7.00c3; file-offset mapping ev cbf2.0c86.351d.3bfc.02af.39b1.9a3e.98a8.d162.a933.51a5.756a.178a.97a9.b000.a218; cursor flag writer ev eaac.be9d.2281.2dfb.e907.ff3f.a467.441f.9b7b.94b3.3fd9.39bc.1c54.4304.2fef.698f.
- **Our client**: No cursor option in the shell; see `docs/pins/net-c2.md:421`.

### `0x5a4b50` — read the current cursor mode (support)

- **VA / file offset**: `0x5a4b50` / `0x1a4b50`.
- **Behavior**: Returns the object's `+0x24` word without side effects; used by the F8 option to decide which caption to show.
- **Reached from**: `0x4c2390` and `0x5a49d0`.
- **Player-visible reach**: Support for the F8 pointer switch.
- **Evidence**: analysis ev 90e2.b179.7eab.062d.7f81.911e.aeff.e437.b848.6d21.6c5e.6921.63fb.55d2.4b74.3983; dossier ev 1376.8f3f.7edd.5582.18b3.2d81.95f8.1191.31a4.628e.283a.409c.73de.3cf5.aaf5.7bbb; caller facet ev 74e1.45b7.fa87.751b.9bab.cf58.e9ae.8e7c.db47.5b31.0195.aa69.838f.fb1e.1701.40d3; callee facet ev 7007.eb39.7fc4.9c97.3e34.e3d7.c702.924e.1bdd.3cd3.f5f3.2157.0574.8432.05c9.7ec9; incoming xrefs ev e89c.8b01.3c1c.9145.4ffb.a3ab.240a.6c9c.f234.00c6.1e60.6782.6b73.7031.ef38.a0b2; body batch ev b6be.ce0b.566b.f4a2.1eab.5093.814a.c72e.a8ad.4bad.ee02.6d79.be9e.d006.37d7.00c3; file-offset mapping ev dabe.1b8a.f493.2fb4.55cd.b289.1ff9.3c4f.8c68.5b31.fcfe.9a4c.4db4.2a26.2ea5.9aad.
- **Our client**: No counterpart by VA.

### `0x5a5020` — post the quit message on window destruction

- **VA / file offset**: `0x5a5020` / `0x1a5020`.
- **Behavior**: Clears the object's active flag at `+0x14` and posts `WM_QUIT` with code zero.
- **Reached from**: Only `0x5a5190`, the `WM_DESTROY` arm. WndProc dossier: ev 1981.b210.2ccc.fee7.e1cb.6fa0.5edf.430c.720b.6bfb.b177.20ce.e461.70f7.fecf.5ec0.
- **Player-visible reach**: Yes. Closing the window terminates the message loop.
- **Evidence**: analysis ev 60ed.584b.2b1f.4c3b.7590.f277.ec64.3e2c.c509.b73f.67a0.b11b.fec1.07b8.5834.3dcc; dossier ev 2bad.9b5a.799a.c819.717a.712e.d8a2.5693.f912.25c7.72a3.1556.4546.9f48.339e.d22f; caller facet ev 63ea.4574.f450.e5bc.2d7c.dca8.bcad.650a.162e.65a5.25d5.6cc3.ff91.9d1d.a492.657a; callee facet ev ee26.9350.9fd4.5cbf.1f4e.6a99.a072.9a8a.3879.7746.fcee.4f5e.e3bd.a4a6.f8e0.ef65; incoming xrefs ev 47c5.f68a.7b2c.3da8.dbb4.5b3d.cd7f.ccb4.03e1.a101.ee42.e2ce.da90.85c9.c369.bbe2; body batch ev b6be.ce0b.566b.f4a2.1eab.5093.814a.c72e.a8ad.4bad.ee02.6d79.be9e.d006.37d7.00c3; file-offset mapping ev 232a.5f62.5aee.f568.b67b.6a6c.e08a.fc24.0167.0e8c.4982.467a.8e2f.a533.5421.d330.
- **Our client**: Partial. Quit-confirm state lives at `crates/o2jam-shell/src/screens/game/mod.rs:6691`; the loop observes `quit_requested`, but there is no Win32 `WM_DESTROY` path.

### `0x5a5040` — refocus the stored window when the client is active (support)

- **VA / file offset**: `0x5a5040` / `0x1a5040`.
- **Behavior**: If the object's active flag at `+0x10` is nonzero, applies the stored window handle through the cursor applier `0x431f70`. Always returns 1.
- **Reached from**: Only `0x5a5190`, the `WM_SETFOCUS` arm.
- **Player-visible reach**: Support for focus/cursor behavior; no frame is painted here.
- **Evidence**: analysis ev c6f1.00fa.cfb8.c01b.ae3c.44d9.6cb8.4a68.5a5f.e7cd.a47a.5912.ee33.ee90.015d.bdb8; dossier ev 6008.1593.b14c.d306.1b5b.79f0.53ed.294c.34aa.4e99.1b6c.e38c.419a.0edb.04c1.5be4; caller facet ev 7267.f483.0a3a.04f1.2732.90c8.59b2.a02a.b3b7.7388.12d9.b8c6.88b8.5c78.8313.e208; callee facet ev 38e9.fd41.d44f.58e7.6078.3b6c.e8d6.afaa.de61.c6e8.03d5.8315.a843.5290.bfc9.de1a; incoming xrefs ev 7574.3dbf.4c18.4568.189f.0636.99cf.6ad7.e10e.c3f9.6499.3e8b.b88e.5226.5e93.800b; body batch ev 8593.f574.8cbe.be38.0739.1288.65ec.bec4.dc64.fa6b.f6b2.0921.4a04.6a7c.0f0d.03ac; file-offset mapping ev 3243.cf6a.1c14.e4ca.9a9e.97cb.6b55.14f1.cc24.682a.589b.d6c0.8d89.9041.4d41.9a7b; cursor applier ev eaac.be9d.2281.2dfb.e907.ff3f.a467.441f.9b7b.94b3.3fd9.39bc.1c54.4304.2fef.698f.
- **Our client**: Winit focus events reach `crates/o2jam-shell/src/window.rs:554`; no exe-equivalent arm exists.

### `0x5a50b0` — forward a double-click to the UI notice helper

- **VA / file offset**: `0x5a50b0` / `0x1a50b0`.
- **Behavior**: Passes the x/y arguments to `0x548b10`. That helper reads a state word, and when bit 2 is set posts UI notice `10,3,0`.
- **Reached from**: Only `0x5a5190`, the `WM_LBUTTONDBLCLK` arm. Notice helper dossier: ev eaac.be9d.2281.2dfb.e907.ff3f.a467.441f.9b7b.94b3.3fd9.39bc.1c54.4304.2fef.698f.
- **Player-visible reach**: Yes. A double-click can raise a UI notice when the state bit is set.
- **Evidence**: analysis ev 4d26.dc54.2c54.78ac.7f5c.ed2d.61eb.30e8.53f7.0d19.7246.d0b3.1faf.3e23.3c84.1f30; dossier ev 695f.8689.2906.14e9.9c3c.9d49.60b5.e6c7.6142.92a3.df69.f58d.1550.20b4.70de.7ea0; caller facet ev f9ee.3b4e.1c98.777c.e98a.7535.d509.e93f.7e0c.bdfc.e2b3.e397.83e4.6ee8.0234.94bf; callee facet ev f22f.cc18.d999.b679.9c4f.1261.f084.a1eb.e8de.6913.f780.0e2e.39a2.6c17.a903.f782; incoming xrefs ev be07.8c61.4b96.052c.5331.62cb.0f65.5144.1e13.d814.4745.3b05.2dd0.c10c.b127.7ed3; body batch ev 8593.f574.8cbe.be38.0739.1288.65ec.bec4.dc64.fa6b.f6b2.0921.4a04.6a7c.0f0d.03ac; file-offset mapping ev 6881.3dd7.5a69.602a.6295.e7ee.e557.4647.24d1.a4a7.0e96.e69a.b157.e24b.42dd.e56e.
- **Our client**: No counterpart by VA; pointer events are staged through `GameLoop::ingest_snapshot`, with no double-click notice arm.

### `0x5a7060` — clear the member table when physical memory is low (support)

- **VA / file offset**: `0x5a7060` / `0x1a7060`.
- **Behavior**: Reads the global memory status and, only when the enable flag is set and available physical memory is below 0x280000 bytes, calls the roster-clearing helper `0x52f550`.
- **Reached from**: `0x424190`, `0x4249f0` and `0x42a130`.
- **Player-visible reach**: Support. It changes staged member state, not pixels.
- **Evidence**: analysis ev 9175.b4b7.8d43.430b.b77f.24f4.bffd.f056.ab1f.ca35.46f7.c6f9.08a8.4edc.fd8b.d8dc; dossier ev 06c9.ff1f.da5f.48d7.cce3.a5d1.d55c.d170.c4c6.7a58.995e.6137.0ba3.068e.efb0.2172; caller facet ev 9147.3376.68c5.2e10.a1e2.cf6f.47df.bdf6.d4d9.7aca.56bc.f7f2.92cb.4ad4.ec45.1a5e; callee facet ev a3ee.d61a.a7ec.eccb.d98c.238d.f076.5347.96dc.3e8f.0354.8cdb.601f.03b7.d09b.491d; incoming xrefs ev 7bf1.7b83.bfaa.9964.bedd.c14c.1fd5.61fd.cf40.b729.1891.b2b4.ad10.aa4b.3a07.d3f2; body batch ev 8593.f574.8cbe.be38.0739.1288.65ec.bec4.dc64.fa6b.f6b2.0921.4a04.6a7c.0f0d.03ac; file-offset mapping ev 0eec.6858.6828.2926.3719.a100.2a80.996f.28a6.3721.465d.6d91.77a9.f761.8160.0cdf; roster clear helper ev 1981.b210.2ccc.fee7.e1cb.6fa0.5edf.430c.720b.6bfb.b177.20ce.e461.70f7.fecf.5ec0.
- **Our client**: No counterpart by VA; no low-memory branch exists in the shell.

### `0x5a8a30` — select an inline or heap string buffer (support)

- **VA / file offset**: `0x5a8a30` / `0x1a8a30`.
- **Behavior**: When the stored length is below 16 it returns the address of the inline buffer at `+0x38`; otherwise it returns the heap pointer stored there.
- **Reached from**: Nine callers, among them registry helpers `0x415d90`–`0x415f40` and option wrappers `0x5a4800`, `0x5a49d0`.
- **Player-visible reach**: Support for path/registry work.
- **Evidence**: analysis ev fea8.d283.102d.1a1e.7645.4798.78b3.167f.d956.8800.633f.aea4.6143.582e.38ff.af80; dossier ev 3c2e.bee5.89d3.a179.82e8.8998.4bf2.8aec.6d15.a22e.6757.3510.5bb3.99dc.0e26.bcd3; caller facet ev b0a7.f236.31aa.5708.8a1d.5aed.5243.bee1.4330.fc7f.1e09.4aca.d279.df31.b8a0.8d13; callee facet ev 8a35.ba30.ea2c.e54f.7e0c.debc.14ac.dc0e.a8ec.632d.af51.016e.960c.da21.10e1.c191; incoming xrefs ev f64d.ed4e.32e2.7680.e1ee.81d9.5377.4f24.7e1c.7c62.eaa2.4e69.fe64.6a5a.f04a.c893; body batch ev 5fd6.3ea9.8f0c.9410.31b6.61b4.ca7d.de9d.a88d.f267.474e.7798.7d7b.512d.4bef.157c; file-offset mapping ev da3f.2587.3e0a.5d47.bd16.791a.fb4c.d140.95f8.33d1.62fb.044d.66db.d698.fec7.4670.
- **Our client**: No counterpart by VA; Rust `String` provides this role.

### `0x5a8b20` — show and focus both global web windows

- **VA / file offset**: `0x5a8b20` / `0x1a8b20`.
- **Behavior**: For each of the two global handles, calls `ShowWindow` with `SW_SHOWNOACTIVATE`, updates the window and sets focus. Returns 1 regardless of handle state.
- **Reached from**: Eight callers, including the three web-sheet openers `0x45e010`, `0x45ea60`, `0x45f190` and the focus handler `0x5a4b90`.
- **Player-visible reach**: Yes. It makes the web sheet visible and focused.
- **Evidence**: analysis ev 2ea0.2e8e.050a.9b41.13a2.7230.dd73.04b6.1954.37c5.19ff.a1d0.1b54.17c3.02c1.c8ad; dossier ev 7754.b89a.42f1.5828.74da.a16f.4f42.6c96.9f6c.aa1d.9848.4a3d.8cae.7c81.2a69.df82; caller facet ev 0dfb.b60f.75f1.948d.703b.35db.fefa.6c7b.c431.d7c9.eb0f.0770.9ed4.2f66.b6ed.0329; callee facet ev 5713.1cb8.77c6.7f6a.e534.5ffc.b417.9957.dbae.a8ed.6120.1cee.0d54.2737.aa6c.994f; incoming xrefs ev 7b4f.d7b9.71bd.7f83.f4aa.2de2.4a90.fbd6.9605.21fe.89c5.b9c5.1de8.b00e.059e.bac4; body batch ev 5fd6.3ea9.8f0c.9410.31b6.61b4.ca7d.de9d.a88d.f267.474e.7798.7d7b.512d.4bef.157c; file-offset mapping ev 63d2.43f3.9445.522f.f331.d039.662f.6a6d.d3bc.3bcb.f19d.a209.a49f.eb38.9df7.586e; web opener (cash bill) ev 19f3.affc.ff45.cd42.ec21.a60c.c06c.fed2.a7c7.cef1.28b7.a981.b2ea.6a56.0ccc.f8b0.
- **Our client**: No counterpart by VA; embedded web clients are blank by design (`crates/o2jam-shell/src/screens/game/mod.rs:20619`).

### `0x5a8ba0` — hide both global web windows

- **VA / file offset**: `0x5a8ba0` / `0x1a8ba0`.
- **Behavior**: Calls `ShowWindow` with `SW_HIDE` on each stored handle when present, then returns 1.
- **Reached from**: Only `0x5a5190`, the `WM_APP+0x31` arm. WndProc dossier: ev 1981.b210.2ccc.fee7.e1cb.6fa0.5edf.430c.720b.6bfb.b177.20ce.e461.70f7.fecf.5ec0.
- **Player-visible reach**: Yes. It removes the web sheets from view.
- **Evidence**: analysis ev d4dc.30e5.81bd.b874.b463.4d8e.a682.a967.a585.d2ff.a62e.38c5.c096.a96d.3b3f.2b5b; dossier ev c5c4.b0cb.d98c.43b1.713e.8532.c639.4bf6.a145.0a8b.9c85.28a2.c440.78c5.5cda.6be3; caller facet ev 5f3c.ebfc.7b84.97fb.db44.03ef.c87d.2ca3.b7fd.52c0.f3aa.d3d6.f919.a451.4e73.d33b; callee facet ev 2b55.073a.f07b.267d.bbfc.94b9.60bb.d288.4bb9.2707.0d88.9922.538b.4c86.333f.5532; incoming xrefs ev 7ad4.2d61.ff41.355f.19e7.4e49.30f7.6a8e.2faf.9d8b.17e0.3900.7794.66a8.13c3.6e2a; body batch ev 5fd6.3ea9.8f0c.9410.31b6.61b4.ca7d.de9d.a88d.f267.474e.7798.7d7b.512d.4bef.157c; file-offset mapping ev 6e99.dc6c.d5b3.fabc.9abe.d0c9.3940.78f8.d3b5.81a7.656f.eaac.8de3.f30f.7537.dc71.
- **Our client**: No counterpart by VA; no global web handles in the Rust client.

### `0x5ac690` — forward the boot window callback to the WndProc (support)

- **VA / file offset**: `0x5ac690` / `0x1ac690`.
- **Behavior**: Passes the four window arguments to `0x5a5190` and returns. No direct caller edge is recorded; one incoming reference at `0x5a595e` places it on the window-creation path.
- **Reached from**: No resolved direct callers; incoming xref `0x5a595e`.
- **Player-visible reach**: Support. It installs the routing entry; the visible behavior is in the WndProc.
- **Evidence**: analysis ev f7f9.2fad.5fa8.af2d.0455.1353.cbe3.1f9c.3659.6d19.db03.8086.2ee0.9032.b025.3f88; dossier ev 140b.091c.cf68.e2c8.fea2.82f0.a9c8.285d.4cf3.c52b.fa56.9a65.1abb.3d3d.796c.c8e6; caller facet ev e228.70a7.560e.9bc0.083b.b65a.ea2b.4396.7c29.b4b5.63e3.df4d.f926.c8cd.51f1.ecc2; callee facet ev f9af.3c61.39ee.9283.9751.2a94.acef.bb70.74a6.572a.a6ed.1d3c.9566.5b05.bf48.c4aa; incoming xrefs ev 174a.9606.02bc.8d18.1cb3.73f4.6867.d0e6.9bce.c6bf.5418.c90c.4e05.cd32.1356.2487; body batch ev 737d.7d15.da02.5fc7.ee0e.03fe.027e.063e.d143.7bec.2e14.24b0.6044.c1c1.ccca.ead2; file-offset mapping ev 6467.ceab.8e78.b379.d551.b678.7cdf.099f.ff2c.b522.304c.054a.440e.8590.5ff9.62ea; forwarded WndProc ev 1981.b210.2ccc.fee7.e1cb.6fa0.5edf.430c.720b.6bfb.b177.20ce.e461.70f7.fecf.5ec0.
- **Our client**: No counterpart by VA; winit's event loop starts at `crates/o2jam-shell/src/window.rs:554`.

### `0x5afae0` — clear a six-byte record (support)

- **VA / file offset**: `0x5afae0` / `0x1afae0`.
- **Behavior**: Writes zero to the four-byte head and zero to the trailing two-byte field.
- **Reached from**: Ten callers in the net and UHP clusters, including `0x58cf50`, `0x58d260`, `0x58e2a0` and `0x591c60`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev caa0.ae73.fda3.6d6d.f917.ee88.d331.99e9.4e65.3288.6d6c.8c49.226d.7f21.c412.0925; dossier ev d9cb.0bba.657b.8418.5497.56af.db83.e010.a34e.470f.ef24.53ac.ece6.a622.d423.8b64; caller facet ev 583a.e094.e754.79be.52ac.ec23.3088.6186.a01a.0ea4.51eb.c84b.4f77.b853.b7ac.6652; callee facet ev f47a.27b1.fe13.ac69.c749.31b6.f9e1.006c.b885.5be1.d2c9.49ce.d160.2a96.6068.7e56; incoming xrefs ev 8f8b.0710.0dd5.431b.d1b4.06c9.43ad.53ae.0937.658b.162e.3717.c371.5c72.258b.716e; body batch ev 737d.7d15.da02.5fc7.ee0e.03fe.027e.063e.d143.7bec.2e14.24b0.6044.c1c1.ccca.ead2; file-offset mapping ev c67b.eee4.8ad4.4363.c3d1.0d6f.7f8d.b088.d793.eae5.d64c.95fd.c302.6728.ea87.2db7.
- **Our client**: No counterpart by VA.

### `0x5afbb0` — copy one six-byte record (support)

- **VA / file offset**: `0x5afbb0` / `0x1afbb0`.
- **Behavior**: Copies the four-byte head and two-byte tail from the source record into the destination record.
- **Reached from**: 24 callers across the packet/UHP vector cluster, including `0x590320`, `0x5b31a0` and `0x5917c0`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev 6a50.c562.a2e2.8b3f.579d.b0f7.6566.6a5a.c613.0155.8996.ed5b.3eb0.7136.a960.5f9d; dossier ev eae9.8ba7.2527.df6e.5e29.3aae.6c5e.5ca9.3933.a0e6.3897.d987.e160.ed49.b5e1.a189; caller facet ev 3109.8963.bcca.81d8.0ee5.c73a.be7e.deb8.a24b.decf.7533.899b.e695.473e.6ab8.ab81; callee facet ev 623a.c609.02a5.5ae0.39fb.7b4e.6969.4a0f.b16b.9cf1.ceb3.de09.16a9.e841.3af2.19f8; incoming xrefs ev 4dc5.cbdd.71d0.887e.0b45.9e38.bdca.aade.f39f.ca91.d98d.9303.53a4.d8d6.2674.ee3c; body batch ev 737d.7d15.da02.5fc7.ee0e.03fe.027e.063e.d143.7bec.2e14.24b0.6044.c1c1.ccca.ead2; file-offset mapping ev 87ee.6f3d.ecaa.e56f.583e.e10e.c363.75bc.a4ca.48ac.dac8.3387.bbcc.fd62.f719.f531.
- **Our client**: No counterpart by VA.

### `0x5b01c0` — enter the UHP sender setup path (support)

- **VA / file offset**: `0x5b01c0` / `0x1b01c0`.
- **Behavior**: Calls `0x5b29b0`. That body creates a `CThread<CUHPSender>` object, starts the worker once and sleeps 10 ms.
- **Reached from**: Only `0x592380`. Sender setup dossier: ev 8e25.304e.42d3.7947.d150.b188.1aa8.812e.5252.2a27.642e.799b.8f46.4bfa.637b.730c.
- **Player-visible reach**: Support for the UDP hole-punching thread.
- **Evidence**: analysis ev 0ca3.cd79.e64e.f47d.c4c3.3eb6.a6bd.5fa6.7b51.cb02.1a9f.095f.29e4.df9b.53ce.0ec1; dossier ev 8162.16c8.64ab.55a0.2f8f.14ab.4bbf.6fbb.921a.0888.66fe.ddeb.a06a.d936.8cec.fd8f; caller facet ev 3d93.455b.8e33.a1be.3da8.c5e0.a1e5.d5c9.8d11.dd4a.c6a5.338e.ca24.0bfd.ba89.7e01; callee facet ev b0ce.e00e.dd8f.e894.bb12.2104.c205.886a.befd.caa2.ba2e.c6f6.d95f.f51b.8a6d.d961; incoming xrefs ev ddc3.60ff.155b.ce3f.e411.65bb.8e5b.68f1.6a47.b0fe.3cbf.24b3.b34f.32d6.f468.f4c9; body batch ev 8026.c71c.622b.cdeb.4a78.0b64.2db5.f155.4860.aaec.e8ec.ae06.71bc.a4f0.dfb7.a62c; file-offset mapping ev 49e7.bf36.205d.95e2.497a.d06e.848b.c173.161c.78dd.c2ca.4675.fc32.6e29.abdb.edff.
- **Our client**: No counterpart by VA; the Rust relay has no UHP worker.

### `0x5b02a0` — lazily initialize the UHP static state (support)

- **VA / file offset**: `0x5b02a0` / `0x1b02a0`.
- **Behavior**: On first use sets a guard bit, zeroes four static words and registers an `atexit` cleanup. Later calls only return the static address.
- **Reached from**: Seven callers in the `0x591f50`–`0x592380` UHP cluster.
- **Player-visible reach**: Support.
- **Evidence**: analysis ev 8ade.0b44.4ad3.2ba3.fff0.8c9f.8c90.8809.7e23.0249.f2c1.e6eb.6dd8.f077.f445.9ebb; dossier ev 91be.55d9.133c.2029.8322.bcf8.9b28.4ef2.f899.45a3.4995.9798.bb6a.dba4.4ead.6ce3; caller facet ev 42a9.8bd4.31b3.1ee2.a8c2.b59f.d010.ded4.c4bd.9617.d88e.9a38.1666.692a.8da0.8338; callee facet ev 08e7.7405.7289.383d.089d.67ac.6479.0359.035e.caca.c3e0.60c6.0644.e01b.fb62.f4ed; incoming xrefs ev 899f.199e.fc78.a056.ac91.22d4.b204.fbb8.735f.9b14.930f.03f5.4424.c89c.9ce2.53f5; body batch ev 8026.c71c.622b.cdeb.4a78.0b64.2db5.f155.4860.aaec.e8ec.ae06.71bc.a4f0.dfb7.a62c; file-offset mapping ev fb51.f13c.d979.c49b.898e.df46.c28e.32cc.dafa.8ef9.ee19.0705.8432.6027.d379.bc8c.
- **Our client**: No counterpart by VA; Rust statics/`OnceLock` serve this role.

### `0x5b0c50` — copy four-byte words into a destination (support)

- **VA / file offset**: `0x5b0c50` / `0x1b0c50`.
- **Behavior**: Advances one word per iteration and copies from the shared source until the destination end is reached.
- **Reached from**: Only `0x5b1020`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev ca41.947c.73ed.769c.9a05.50a6.140f.5c91.e9b2.81d3.8443.c2d7.2c48.1916.acb9.2adb; dossier ev 2a70.2ec8.01f4.a83a.66ba.dbfd.89a5.4592.0b7c.3d37.6949.c150.c8e2.5adf.5aa3.09b3; caller facet ev 42e7.46fa.c96d.d996.83f6.b2ab.6a75.55d2.c268.5be3.7eb4.c154.05c2.39bf.7655.fdcc; callee facet ev 58e7.ed5b.8375.65a4.1398.bef2.0284.2a7e.1495.4af2.4a03.dbdb.85f7.ebfc.3b19.22b4; incoming xrefs ev 85c5.a83d.876f.7fe5.a957.234b.b32f.1723.2671.f723.fcc8.e53b.6717.b0e4.e736.fd1a; body batch ev 8026.c71c.622b.cdeb.4a78.0b64.2db5.f155.4860.aaec.e8ec.ae06.71bc.a4f0.dfb7.a62c; file-offset mapping ev 6625.c213.5e9f.7cf3.4c2a.6d67.fefd.a1dc.8da6.a77e.2368.ea31.fb6f.8b73.3ad6.9f03.
- **Our client**: No counterpart by VA.

### `0x5b0c70` — copy four-byte words behind the destination (support)

- **VA / file offset**: `0x5b0c70` / `0x1b0c70`.
- **Behavior**: Computes the word count between two pointers and `memmove`s it to the destination shifted back by that count.
- **Reached from**: Only `0x5b1020`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev aa65.ceb3.0cb4.e083.0a25.b170.0133.0e8f.5a1a.cc50.18c1.4a22.3cea.3a3f.59ad.f43b; dossier ev ee48.ed26.8fee.ac43.e9db.4264.8e4c.61c4.c9f3.d309.bf32.a792.6790.1c05.3f88.2b37; caller facet ev 3c1d.47df.c5dd.261a.cc01.3327.0ff1.c170.0454.6d35.b7ca.1478.f52d.7a39.3204.d4d6; callee facet ev a1d7.f038.13b9.3dbc.be90.7edf.19fa.6036.8be5.5b2d.ee74.47f7.bc0e.dc36.e2f5.822c; incoming xrefs ev 47da.c84b.fd39.b194.a63e.80d7.3617.82e0.457c.e4ab.11c6.51d0.90f5.a33e.f322.dd21; body batch ev 2af9.3c6c.7f96.9459.55c1.9fd2.79b2.e4f8.f71f.76e1.b2fa.4391.5301.e72e.b6cc.1917; file-offset mapping ev e435.78da.4baf.4651.a7d7.46f9.aade.5090.60fb.1879.c0c9.dd9b.d939.dc34.9fba.bdce.
- **Our client**: No counterpart by VA.

### `0x5b0ea0` — move four-byte words and return the end pointer (support)

- **VA / file offset**: `0x5b0ea0` / `0x1b0ea0`.
- **Behavior**: Computes the byte size from the pointer difference, calls `memmove` when nonzero, and returns destination plus that byte size.
- **Reached from**: Only `0x5b1020`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev 4fa1.7fca.ad55.e444.dcb1.15ee.851f.b4cb.d377.3d3c.e9b2.9168.6586.3892.5782.8e5c; dossier ev 7784.0e9f.4c31.8ddc.b66d.73b8.d343.01a0.36e9.70e0.2c54.ca4d.bbf8.f304.7720.2305; caller facet ev 2dfc.e3d8.cda1.e0d0.f7d6.1d22.a81e.7ea4.3da0.2946.2728.aac0.1330.9eb4.6704.269f; callee facet ev be5a.57b0.e6d3.e8f6.a5e7.82e5.3c90.319b.f39c.a35e.c4ed.a005.146c.51b6.5277.f1d6; incoming xrefs ev e0af.24e3.d32e.72c4.79bf.17f8.0dd5.c69d.916f.62ec.d212.d8f7.50e5.8bc3.0cb0.2d34; body batch ev 2af9.3c6c.7f96.9459.55c1.9fd2.79b2.e4f8.f71f.76e1.b2fa.4391.5301.e72e.b6cc.1917; file-offset mapping ev aa43.5080.ae7f.17f3.14bf.1e29.cbbf.4823.2dbb.60ad.20be.183f.7bc0.47d0.b39e.d598.
- **Our client**: No counterpart by VA.

### `0x5b0f00` — fill four-byte words and return the end pointer (support)

- **VA / file offset**: `0x5b0f00` / `0x1b0f00`.
- **Behavior**: Copies the count words from the source into the destination, then returns the end address.
- **Reached from**: Only `0x5b1020`.
- **Player-visible reach**: Support container operation.
- **Evidence**: analysis ev 527f.15b7.c246.b655.bea8.1f83.2397.f2f4.e97b.72f0.6146.9cf4.29b9.2a1b.889c.0054; dossier ev 0674.54b2.4c76.26b9.f589.a8cf.395a.6c21.0ca2.b77e.2a8a.a2f9.b6f5.1091.3802.9a41; caller facet ev 09bd.0b93.86c4.6858.c1a8.a418.fa08.f19b.d6c6.04aa.968c.48e2.6ead.41c9.9db1.8769; callee facet ev b787.bc02.58ce.ff03.eb82.274d.b11c.50e8.3dc3.8413.028f.29e6.c164.7c27.fc72.cc0b; incoming xrefs ev c857.5aae.9cd7.d315.ce5b.8699.70ec.b560.c876.0fd5.dbed.96ab.4335.8221.7591.b153; body batch ev 2af9.3c6c.7f96.9459.55c1.9fd2.79b2.e4f8.f71f.76e1.b2fa.4391.5301.e72e.b6cc.1917; file-offset mapping ev 1c67.f699.4b5e.d8bd.d08b.d647.9f0b.b074.7f80.9544.c140.f901.22f2.ba62.1c15.58ab.
- **Our client**: No counterpart by VA.

### `0x5b1020` — insert a range into a four-byte vector (support)

- **VA / file offset**: `0x5b1020` / `0x1b1020`.
- **Behavior**: Checks capacity against the requested count, grows with a 1.5x policy when needed, moves the suffix, copies the new range from a pointed-to source, frees the old block and updates begin/end/capacity.
- **Reached from**: Only `0x5b11f0`.
- **Player-visible reach**: Support for the UHP vector; no pixels or wire.
- **Evidence**: analysis ev 457b.217a.d041.fe72.4ac6.8f2f.d048.9ab5.3cb1.a103.493a.dc03.e18d.f41f.1d8e.d3d9; dossier ev b9ba.9b4a.2d40.0473.860b.a157.cb66.a8e3.e3a1.9834.3ebc.2170.3ec4.d209.37bf.bc87; caller facet ev de47.cdbc.5422.b9bc.63fb.0f48.7d64.e5c7.325a.e07c.f965.288a.7cce.7890.1cc6.2787; callee facet ev e1dd.9e27.026a.d12e.7a7d.b415.0f24.2617.e5f7.60be.8410.cb66.d914.1f9c.1e44.2f41; incoming xrefs ev ed5b.ca80.eab8.c338.fc3c.c178.b04f.caf6.ccdc.53ac.29eb.b544.4291.56e4.818d.6f44; body batch ev 75e7.bf79.fc50.f3dd.35dd.eea8.bb84.f4c7.ef8a.1ccd.e2ce.58dd.c426.4351.c1fc.3e97; file-offset mapping ev 7b47.3a2b.82d6.cbc1.22af.0138.3cfb.5b5f.5d92.18ca.5703.b078.2d00.a520.cdb7.f239.
- **Our client**: No counterpart by VA; `Vec::insert`/`extend_from_slice` serve this role.

### `0x5b2a60` — count six-byte records in a vector (support)

- **VA / file offset**: `0x5b2a60` / `0x1b2a60`.
- **Behavior**: Returns zero when the begin pointer is null, otherwise the end-minus-begin distance divided by six.
- **Reached from**: Only `0x5b3720`.
- **Player-visible reach**: Support.
- **Evidence**: analysis ev f2e9.dd6e.d805.8bb0.8799.4320.0cbf.d8fe.59ff.9581.c320.b071.b6ae.e9b4.092f.3869; dossier ev b6a0.7e78.da7a.1138.9f60.be93.edcc.60b3.2328.4e35.dd5f.fe1a.9ea7.264a.decf.2e92; caller facet ev 0814.818a.5669.0dac.3be8.1fb2.f134.9900.3df0.034a.ab7d.f9e0.5038.81c3.58c5.1428; callee facet ev 8249.e618.d83b.7e54.09aa.aeb7.916a.e8c9.fb2b.658b.3131.56ea.32d2.7c46.5968.665a; incoming xrefs ev 4849.2cec.819d.ae2e.5a66.1703.5cd6.b35e.e877.8294.b547.eded.f403.7140.2100.ba4f; body batch ev 75e7.bf79.fc50.f3dd.35dd.eea8.bb84.f4c7.ef8a.1ccd.e2ce.58dd.c426.4351.c1fc.3e97; file-offset mapping ev e184.8bba.73dc.99bf.6f0e.4e56.6543.8f89.8ba8.7a5c.a21d.1586.c25e.9bba.3077.a09c.
- **Our client**: No counterpart by VA.

### `0x5b31a0` — copy a run of six-byte records (support)

- **VA / file offset**: `0x5b31a0` / `0x1b31a0`.
- **Behavior**: Walks source to destination in six-byte steps, copies each record through `0x5afbb0`, and returns the base plus the rounded record count.
- **Reached from**: Only `0x5b3720`.
- **Player-visible reach**: Support.
- **Evidence**: analysis ev 13f9.d7ba.7a49.a4e8.76bb.038b.c88c.c3a4.51b4.9742.2fff.a991.358f.4641.79e4.b960; dossier ev d710.7163.b24e.b43e.8cd7.34cd.9018.e188.d34b.68ca.4d31.ca57.1707.2053.22a8.20c4; caller facet ev ee34.a56d.f0ef.34f6.29f3.5643.48a5.1062.c51a.1921.bf74.c002.1950.db29.64ac.b038; callee facet ev dbae.d9c4.9a8d.c60b.67c9.6fe5.6804.d89d.123c.76e6.2e23.f44d.54e8.3ad3.18d0.cab8; incoming xrefs ev c2bf.bfa5.3bcd.ae0c.4953.6ed1.8f6a.c759.5772.44b8.5095.1068.0e14.5e8e.d96e.1dcd; body batch ev 75e7.bf79.fc50.f3dd.35dd.eea8.bb84.f4c7.ef8a.1ccd.e2ce.58dd.c426.4351.c1fc.3e97; file-offset mapping ev e158.d32e.460f.d059.8f3b.f229.e2cf.3ae3.75c4.5590.519c.dd44.2857.00f6.6ab6.4532.
- **Our client**: No counterpart by VA.

### `0x5b3560` — forward a six-byte record copy to `0x58c2f0` (support)

- **VA / file offset**: `0x5b3560` / `0x1b3560`.
- **Behavior**: Passes the range and destination to the shared six-byte copy helper. That helper copies each record and runs its unwind tail.
- **Reached from**: Only `0x5b3720`. Record-copy dossier: ev 8e25.304e.42d3.7947.d150.b188.1aa8.812e.5252.2a27.642e.799b.8f46.4bfa.637b.730c.
- **Player-visible reach**: Support.
- **Evidence**: analysis ev b9a3.88e3.c8b0.e6a8.2e6d.57b8.a17a.4906.3bc3.aab1.415a.f633.8f39.9d1c.a0d6.d267; dossier ev 9110.7683.51eb.d321.87b6.b3bb.a578.5107.c937.2dbf.94fe.5b4f.23ac.8a97.68a0.7fc9; caller facet ev 43ce.697b.941f.7181.b929.636c.1221.ff42.e83f.79b3.6a5b.d54f.ddad.f0fd.b176.d3c0; callee facet ev b832.2102.f59c.df87.9752.a809.b970.fac6.e6d4.6c3b.74a7.d617.25ab.75eb.bc29.2e2f; incoming xrefs ev ecad.2d2e.7f47.51a3.160f.ab07.6b76.4b46.a4b6.0770.8d1a.e214.1890.9f8e.05bb.641b; body batch ev 0bfd.df80.bf09.8004.5384.5a49.01fb.8428.1c4b.6304.b10d.7610.56e2.da38.c7ac.e749; file-offset mapping ev 43f8.7e41.04f2.3c07.1690.2a35.c259.ed4a.09dd.b36e.1157.80a3.c1a3.1fb3.eea8.80a7.
- **Our client**: No counterpart by VA.

### `0x5b3720` — merge a six-byte endpoint range into the UHP vector

- **VA / file offset**: `0x5b3720` / `0x1b3720`.
- **Behavior**: Derives the source count, compares it with the destination size, and either appends the range, grows/replaces storage after a capacity failure, or clears the destination. Uses the count, copy, insert and range helpers pinned above.
- **Reached from**: Only `0x5b29b0`, the UHP sender setup body.
- **Player-visible reach**: Support for the UHP peer table.
- **Evidence**: analysis ev 30b9.40d0.79b5.d100.e3ea.99de.c8d1.fe55.d037.5235.21c7.6d23.f3b0.a254.b09c.bef9; dossier ev 563e.614e.39b5.dcb9.0d6f.9b4d.37af.eeaf.6fc2.dd4c.6a15.a1b8.7ce1.a09a.23e7.3134; caller facet ev 03cd.0d35.36df.f3ec.90a6.15c4.7621.40ac.e53d.a5e3.8e41.3957.7e0e.91f2.568b.3c50; callee facet ev b68a.9c24.218b.ae30.7a22.5770.d6a8.071a.12d1.c9bd.c3be.b637.3986.9920.e355.f355; incoming xrefs ev 93d2.ee4e.4dea.764b.ee31.22fb.5e62.ad5e.c516.a9c6.618c.626b.02aa.d466.856e.2203; body batch ev 0bfd.df80.bf09.8004.5384.5a49.01fb.8428.1c4b.6304.b10d.7610.56e2.da38.c7ac.e749; file-offset mapping ev f7c5.c5d4.7fea.8083.6c1a.ec7a.0504.db04.d72a.cd22.82b9.cdf0.e639.dbca.b58d.79f6.
- **Our client**: No counterpart by VA; the Rust relay stores UDP ports as constants (`crates/o2jam-net/src/relay/udp.rs:22`).

### `0x5b3910` — spawn the UHP worker thread once (support)

- **VA / file offset**: `0x5b3910` / `0x1b3910`.
- **Behavior**: When the stored thread handle is still the sentinel, clears two local counters, starts a thread at `0x5b2f10` with a pointer to the object's storage and saves the handle.
- **Reached from**: Only `0x5b29b0`.
- **Player-visible reach**: Support for the UHP sender.
- **Evidence**: analysis ev a9bd.62b7.da8c.9ff5.b579.af10.e843.5fcd.c23c.75f1.0b40.5a5e.7f0f.da2f.dab1.ee8d; dossier ev a83a.a48d.b612.2a45.9f7d.c8f3.3f68.b927.af69.2bbb.6072.4fda.9b42.14cb.fea8.9202; caller facet ev 7528.5ea0.56a0.2f04.3347.f688.1d6d.7044.ad91.a278.0553.996a.5f47.0d02.4ae7.17b8; callee facet ev 2cd8.fccc.ec9a.b9ad.e264.e070.a5c6.c18f.21a9.3ca7.9407.e9c0.f50f.e7c4.46e1.dc9f; incoming xrefs ev 652d.a7cd.c9b6.e877.ade9.38b8.e630.5b73.2870.8a09.fc8f.66fa.49b5.822d.a398.3769; body batch ev 0bfd.df80.bf09.8004.5384.5a49.01fb.8428.1c4b.6304.b10d.7610.56e2.da38.c7ac.e749; file-offset mapping ev 192c.abe0.aab0.075a.b04c.fb8a.b7e9.b2b5.1df9.9288.bd24.7f1c.e768.011c.a29d.a99f.
- **Our client**: No counterpart by VA.

### `0x5b85d0` — take the UHP critical section and raise the lock flag

- **VA / file offset**: `0x5b85d0` / `0x1b85d0`.
- **Behavior**: Enters the critical section stored at `+4` and atomically sets the lock word at `+0x1c` from zero to one.
- **Reached from**: 43 callers in the UHP and container cluster, including `0x5b29b0`, `0x5b1680` and `0x5b39a0`.
- **Player-visible reach**: Support synchronization; no pixels or wire.
- **Evidence**: analysis ev 4de7.3efc.2b51.ec2b.ada4.8702.f7fc.e285.1393.87d5.3243.12cf.04f9.ee70.9585.14de; dossier ev d4e3.4920.0113.edda.3dd9.0f80.41c8.4704.926d.19b0.09bf.a74c.cb61.217d.ddc9.7b1e; caller facet ev b47c.a05b.b820.7ea5.14da.5e4b.7172.f33f.25a9.685f.4df1.5863.a02e.9030.79d7.e391; callee facet ev 8fcf.11b8.a20a.20a9.e262.4cb5.19e5.531e.d232.b51d.09d4.791a.383b.65d8.87bb.104b; incoming xrefs ev 1a5c.ddfb.a037.27e1.b557.b2c5.c4bd.63e7.878a.26d3.fa0b.b81f.c592.2123.062a.4ac8; body batch ev becd.6469.362d.a27b.c4ce.8710.651c.7d34.5182.3072.0152.4431.cc96.ea3e.6822.f7b3; file-offset mapping ev 3aac.d6d8.2fe3.ff0e.3b46.4dc5.4d12.46d8.a55f.c880.fb70.617d.ea47.7f9a.0f27.e7a4.
- **Our client**: No counterpart by VA; Rust mutexes give the same exclusion without the flag.

### `0x5b85f0` — release the UHP critical section and clear the lock flag

- **VA / file offset**: `0x5b85f0` / `0x1b85f0`.
- **Behavior**: Leaves the critical section at `+4` and atomically flips the lock word at `+0x1c` from one back to zero.
- **Reached from**: The same 43 callers as the acquire side.
- **Player-visible reach**: Support synchronization.
- **Evidence**: analysis ev 7ac9.e1fa.6fb7.9773.015d.1c2a.b24a.7062.c31f.ba4d.95c3.e61d.9c9a.beab.5ae0.4e38; dossier ev 1931.8a90.ac7a.6572.aad1.87bf.d12c.93a2.fd10.4f10.1a6a.cdfa.5be3.f6ba.1451.4a87; caller facet ev 5af1.0326.6e7a.ed33.22c9.91ed.b312.999c.6284.6ebb.2a0e.1fc2.1c18.45ca.f3c4.f7dc; callee facet ev 236e.d6ea.c673.2391.4af3.d4e7.b8f7.2325.8ff4.5275.7219.2b3c.8150.1eec.28d4.e32e; incoming xrefs ev 2cf3.f353.967c.7721.eedf.6979.cafc.1cd2.b1d5.d0ad.6747.9e05.ce1f.9374.8ac1.8252; body batch ev becd.6469.362d.a27b.c4ce.8710.651c.7d34.5182.3072.0152.4431.cc96.ea3e.6822.f7b3; file-offset mapping ev 3388.5006.8148.83b5.cc59.473a.8082.959a.a26a.0d60.6f88.5503.5b97.b766.fba4.d9c4.
- **Our client**: No counterpart by VA.

### `0x5c4500` — store and enter a critical section (support)

- **VA / file offset**: `0x5c4500` / `0x1c4500`.
- **Behavior**: Saves the supplied critical-section pointer on the object, then enters it. Used as an RAII guard constructor.
- **Reached from**: 25 callers across startup, UHP and crypto/support code, including `0x586ce0` and the `0x401900` startup cluster.
- **Player-visible reach**: Support synchronization.
- **Evidence**: analysis ev fb7c.a23a.17b2.6a0e.02d3.c045.4031.8ed1.3cea.0c96.06e4.b106.6ca8.200d.7a3e.945b; dossier ev 26fb.2ccf.4a91.149b.50f5.97f6.de1d.be54.7b5e.101a.99e6.815f.9467.32d2.024d.2709; caller facet ev 76ac.b593.353f.bda9.3344.202e.34f2.93be.383a.e70a.c806.2bf0.e02a.20a0.82c6.bd7d; callee facet ev d39f.0593.7746.21bc.56a8.02e3.4ac5.ec03.091a.6e1a.c597.d69c.8413.9aad.e4d5.37de; incoming xrefs ev 5333.b16b.e215.9114.650d.3c5e.2383.16a5.a3cf.225e.ffa4.d349.3a0e.097a.2266.b87a; body batch ev becd.6469.362d.a27b.c4ce.8710.651c.7d34.5182.3072.0152.4431.cc96.ea3e.6822.f7b3; file-offset mapping ev c471.4b60.e111.f2dd.eebe.6c4c.27bb.f6ca.98f2.ea75.d1c6.9086.b8b0.f96c.ad3c.4e1e.
- **Our client**: No counterpart by VA; Rust guard types own the lock.

### `0x5c4520` — leave the stored critical section (support)

- **VA / file offset**: `0x5c4520` / `0x1c4520`.
- **Behavior**: Leaves the critical section whose pointer is stored on the object. It is the matching guard destructor.
- **Reached from**: 47 callers, matching the guard constructor's clusters.
- **Player-visible reach**: Support synchronization.
- **Evidence**: analysis ev 523b.9b9e.d510.d1f0.2d0a.93a1.f9c9.5266.ec13.5e91.7af2.5c32.2322.15d6.56d6.87be; dossier ev 3498.3363.07fa.cd58.8ce1.fe9d.2b6e.eec1.e062.8c61.c38e.16b7.e56b.566c.335c.71fe; caller facet ev b4ec.8e1a.eb91.3fe0.e439.4d1e.d684.3a2d.214f.0bec.851e.8361.8d4f.5842.bcd5.adea; callee facet ev 4e90.33d3.1824.599c.44a2.c9bc.4946.2f17.a10d.962a.9123.746f.8444.4e1e.270c.4428; incoming xrefs ev 0583.7c91.ecb3.35f2.4d42.d24b.8ee1.52f5.f7b9.65a2.a23c.7b7b.7a78.5382.0d20.1752; body batch ev be59.e600.ca39.14d9.4968.ff0b.3016.97b4.f97d.1299.612f.c2a9.70e3.188e.731e.b52b; file-offset mapping ev b715.6548.460d.7481.23e4.6d6f.df60.00b1.404f.04ff.7eec.be07.794e.676f.a0ca.398f.
- **Our client**: No counterpart by VA.


### `0x55cad0` refusal-caption addendum — 3003 JoinWaiting result map (2026-11-25 senior REA pass)

- **Result word 0**: success path (stages the joined room and installs
  Waiting state 5). Negative results show a one-OK popup and post the
  `(0x12, 3003, …)` UI event:
  - `-8` -> `0x64d928`: invalid room number, retry.
  - `-7` -> `0x64d958`: free users cannot enter the all-song room.
  - `-6` and `-4` -> `0x64d9a0`: room full.
  - `-5` -> `0x64d988`: game already in progress.
  - `-3` -> `0x64d9b8`: wrong password; the event's third word is 3003.
  - `-2` -> `0x64d9d0`: single room cannot be entered; the Room web
    slot's `+0xf60` word is cleared.
  - `-1` -> `0x64d9f4`: administrator-contact failure message.
- **Consequence for #1018**: the refusal text is pinned; no text needs
  to be invented. Implement the result-code -> caption map and the
  success/refusal split.
- **Evidence**: `ev_5336dbe5254d48f17d33ad7ed0b4fa0789916c58000848416bc3de2d0fc881f3`
  (handler pseudocode).

### `0x5a4b90` focus-return addendum — scene exits happen on activation (2026-11-25 senior REA pass)

`WM_ACTIVATEAPP` (`0x1c`) stores active = (`wParam` is 1 or 2) at
window `+0x14` and sets `+0xc = 1`. When the game object `+0x914` is
present, DEACTIVATION clears `+0x18` after stopping the BGM; it does
not change scenes. ACTIVATION sets `+0x18 = 0`, then checks the pause
reason bits at `+0x1c`:

- Scene 11 (Playing), bit 2 set: install Room state 2, show `0x650d68`.
- Scene 10 (Loading), bit 2 set: if the room title string is empty,
  clear the Waiting seat words, show `0x650d38`, install Room state 2;
  otherwise install Waiting state 5 and show `0x650d08`.

Therefore #1046's transition is on focus RETURN, not focus loss. The
three captions already landed; the missing behavior is the activation
transition. A local pause flag alone is not parity.

Evidence: `ev_029802374255012a8839d968da6743c2d91da5ca03fbe050fbff865e78a8cf49`.
