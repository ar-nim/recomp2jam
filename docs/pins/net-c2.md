# Statics pins — Net-c2: staging support, 4045/4049 bodies, window hub (net-c2)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`.
Provider: REA 6.10.1 / Ghidra 12.2; shared target bound from
`install-root/O2Jam/OTwo.exe`. Evidence IDs are `ev ` plus the full
hex in four-digit groups; joining the groups after `ev_` gives the REA ID.
The grouping keeps `scripts/check-cited-hashes.sh` from reading evidence
hex as a commit citation. File offsets come from REA
`address_to_file_offset` and equal VA minus the `0x400000` image base for
all 40 rows (0 mismatches). Behavior wording is ours; no decompiled text
is reproduced.

Branch `cb2/pins-net-c2`; worktree pre-flight
`/home/arnim/Projects/recomp2jam-lane-pins-net-c2`. The brief's base
`a15244e` is an ancestor of branch HEAD `9cb93ac` (other wave-1 pin lanes
merged in between); no reset was attempted. The brief names
`slice-net-c-01.txt`; the file present is `slice-net-c-01` (no extension),
with the same 40 VAs in the same order.

Scope: the 40 addresses in `slice-net-c-01`, from the coverage triage
bucket `game-reachable`, `pinned_in_repo=no`. **Pinned here: 40/40;
left: 0.** Twenty-seven are STL fill/copy/unwind bodies, four are
Playing-constructor sprite-surface initializers, two are the 4045/4049
wire bodies, one is a shared state setter, and six are window/option
handlers (`0x5a49d0`, `0x5a4b90`, `0x5a50d0`, `0x5a5190`, `0x5a5600`,
`0x5a8ab0`). A static pin is not a player-visible close: census rows stay
OPEN until a live caller chain from boot is recorded
(`docs/live-path-reachability.md`). "Reached from" is the provider's
direct-caller edge, not proof of a runtime press or frame.

The packet dispatcher at `0x558080` was re-read for placement: case
`0xfcd` (4045) calls `0x563d70`, which forwards to `0x58cf50`; case
`0xfd1` (4049) calls `0x563db0`, which calls `0x586470`. Dispatcher
dossier: ev 9328.353a.2da1.5afe.caa5.2002.9fe4.232f.fbe7.50c0.a588.1a74.0abf.feaa.5c12.2b15.
Prior pins are cited, not re-derived: `docs/relay.md:447` (4045 layout),
`docs/dispatch-inventory.md:140` (4049 body still OPEN),
`docs/wine-boot.md:1017` (live 4048/4049 exchange), `docs/net-protocol.md:121`,
and `docs/live-path-reachability.md` (guard tiers).

## Inventory

| VA | File offset | Role |
|---|---|---|
| `0x569be0` | `0x169be0` | support |
| `0x569c00` | `0x169c00` | support |
| `0x569cb0` | `0x169cb0` | support |
| `0x569cd0` | `0x169cd0` | support |
| `0x569d89` | `0x169d89` | support |
| `0x569db0` | `0x169db0` | support |
| `0x569e65` | `0x169e65` | support |
| `0x569e80` | `0x169e80` | support |
| `0x569f33` | `0x169f33` | support |
| `0x56a600` | `0x16a600` | support |
| `0x56a650` | `0x16a650` | support |
| `0x56a6a0` | `0x16a6a0` | support |
| `0x56a6f0` | `0x16a6f0` | support |
| `0x56a740` | `0x16a740` | support |
| `0x56a790` | `0x16a790` | support |
| `0x56a7e0` | `0x16a7e0` | support |
| `0x56a89b` | `0x16a89b` | support |
| `0x56a8c0` | `0x16a8c0` | support |
| `0x56a972` | `0x16a972` | support |
| `0x56a990` | `0x16a990` | support |
| `0x56aa42` | `0x16aa42` | support |
| `0x56aa60` | `0x16aa60` | support |
| `0x56ab1b` | `0x16ab1b` | support |
| `0x56ab40` | `0x16ab40` | support |
| `0x56abfb` | `0x16abfb` | support |
| `0x56ac20` | `0x16ac20` | support |
| `0x56acd5` | `0x16acd5` | support |
| `0x56ad00` | `0x16ad00` | support |
| `0x56ae40` | `0x16ae40` | support |
| `0x56af90` | `0x16af90` | support |
| `0x56b0b0` | `0x16b0b0` | support |
| `0x586470` | `0x186470` | wire-reader 4049 |
| `0x58cf50` | `0x18cf50` | wire-parser 4045 |
| `0x5a49d0` | `0x1a49d0` | F8 cursor option |
| `0x5a4b90` | `0x1a4b90` | focus handler |
| `0x5a50d0` | `0x1a50d0` | F8 key gate |
| `0x5a5190` | `0x1a5190` | main window WndProc |
| `0x5a5600` | `0x1a5600` | exit confirm |
| `0x5a8840` | `0x1a8840` | support |
| `0x5a8ab0` | `0x1a8ab0` | popup/close helper |

## Pins

### `0x569be0` — clear a fill body's unwind frame (support)

- **VA / file offset**: `0x569be0` / `0x169be0`.
- **Behavior**: Writes the local exception marker with the failure value and restores the saved exception-list head before returning. No wire or pixel work.
- **Reached from**: `0x569b30`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; MSVC unwind scaffolding has no Rust equivalent.
- **Evidence**: analysis ev 88ff.557c.1e9d.a3c5.825a.a795.a1a0.cd62.5e70.deb7.8849.2e1a.3372.4636.0bf0.7fce; dossier ev c27a.0492.5b7c.81ad.999a.8ed6.c9d5.e36a.e7b1.d290.73b7.5b05.342a.3bc0.73da.13b6; body batch ev c332.7e9e.ca3e.26c3.3d75.9f16.4eeb.6474.fbe4.cf6a.a1ef.1f57.36a0.3807.4c4b.6fba; caller facet ev 6e42.fdba.0d1c.d64f.72bf.0b1c.4984.5217.ce1b.6f0f.6a3d.f751.c84a.c057.45f1.e13d; callee facet ev 453f.e080.a42d.62eb.998c.f01f.f6ce.3b3e.4bb9.579d.8a66.c123.05b3.56ae.2de2.f891; incoming xrefs ev 0118.1cde.5c85.1de9.cde0.1d2d.7e7d.8aeb.a2dd.3197.a5e2.9358.1e01.8aa6.06ba.927a (`0x569bb3`); file-offset mapping ev 7c98.76fd.a029.b948.6f68.6b28.725d.89de.6fa9.0e78.73aa.d5db.0aaf.4929.f37c.4eb4.

### `0x569c00` — repeat a 56-byte prototype into a destination run (support)

- **VA / file offset**: `0x569c00` / `0x169c00`.
- **Behavior**: For a requested count it copies fourteen words at a time from the prototype into each destination record, skipping null destinations, advancing destination and source by 56 bytes, then clears its unwind frame.
- **Reached from**: `0x564c30`, `0x567040`.
- **Direct callees**: `0x569cb0`.
- **Player-visible reach**: Support for vector fill; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec` fill and copy supply the same role.
- **Evidence**: analysis ev f920.c228.62c5.8be2.3fd3.0d4e.9a27.a43f.dede.38ad.ad94.4ab3.8d6a.5a9f.0f21.04c2; dossier ev 1e37.3d61.f507.499c.270c.7024.c5fe.a950.f12e.af0d.0066.d732.ee1e.775b.7285.5c77; body batch ev c6ee.d528.4d13.3acb.25d6.6cbc.2705.a37d.158b.72c2.fafd.0d7b.2c09.3864.16c1.f06a; caller facet ev e1a5.4078.e9f5.3f63.1f6e.fa3d.33fc.b691.c9a5.fb16.6708.7338.9eb8.7bb8.4bf0.7513; callee facet ev ba94.2aa6.1e20.c46f.7539.4c94.0edb.e070.9289.316d.44db.82a8.8405.314a.12af.50b0; incoming xrefs ev 82b2.6907.8a22.1e23.9951.55a7.ac50.1c82.f3b5.ff0a.08af.cfea.5b48.4c72.ee85.b2e5 (`0x564cc3`, `0x56737f`, `0x5676a2`); file-offset mapping ev 4748.4ee8.b73c.48a0.bdd9.0459.cb4a.3348.950c.c3cc.b045.4178.52d9.275e.7ae1.aafa.

### `0x569cb0` — clear the 56-byte fill body's unwind frame (support)

- **VA / file offset**: `0x569cb0` / `0x169cb0`.
- **Behavior**: Restores the saved exception-list head and marks the local frame as failed; reached only from the 56-byte repeat body.
- **Reached from**: `0x569c00`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev dd66.f007.9658.a323.adfd.33a0.5838.1941.1903.4d73.f119.6e58.0902.96ad.4c38.844b; dossier ev b7b8.5df9.a358.4e0d.63d9.8760.0606.6fb6.36e6.60a1.f520.64ac.28ff.1b43.4246.414b; body batch ev fe42.a591.9b16.7ffb.1816.75be.53c0.c3f5.a135.0d85.ae40.4fca.ed45.af71.2677.ca3d; caller facet ev 04fc.1532.3663.c929.9f58.c064.1cfa.7185.0978.9980.c087.27d5.dc8c.4ce1.d65e.36b2; callee facet ev 7e30.9f0c.f084.6e82.6a55.4b96.75ad.a3e0.4d85.fd68.c637.4dd9.6bb7.29a7.f967.a909; incoming xrefs ev 2234.e1fb.ef2b.a6dd.e602.b03d.6219.b314.e774.1501.07d4.c983.264b.987b.1d08.4da5 (`0x569c83`); file-offset mapping ev 8d57.0c19.615f.5cf4.cac2.32d6.9ccf.aca5.05af.d9db.4569.da88.9af0.5082.4f89.10ce.

### `0x569cd0` — repeat a 12-byte prototype into a destination run (support)

- **VA / file offset**: `0x569cd0` / `0x169cd0`.
- **Behavior**: Copies three words per destination record for the requested count, skipping null destinations, advancing by 12 bytes, then clears its unwind frame.
- **Reached from**: `0x565120`, `0x5678a0`.
- **Direct callees**: `0x569d89`.
- **Player-visible reach**: Support for vector fill; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec` fill and copy supply the same role.
- **Evidence**: analysis ev 8bcc.d0a0.7eb3.8a61.b600.3c89.8f6f.3859.fb64.8a04.6c6c.9fcd.691e.e531.729a.b715; dossier ev 7be2.1f98.1cca.aa6f.03ca.9c35.28f5.dc5b.598f.88a3.ab88.b123.ccc4.cbaa.da8c.752c; body batch ev 8f5c.effa.a1cf.01dd.ea73.f330.2d10.2416.bf40.fe84.4e99.8cac.45a1.ceb3.6a78.e5bc; caller facet ev 9425.b72b.dc2a.0a7d.509b.add0.5133.11ef.a928.41a7.af29.f385.39c3.772c.76f0.11da; callee facet ev 12e9.2bb9.7119.f02b.88c1.dc53.616a.5d6f.2fa0.6932.0d3e.5293.0899.6cb5.0d68.f370; incoming xrefs ev f29f.5309.ba6e.b971.ed1f.f05a.4baf.1b14.3b56.4fe4.53a7.1a8d.68ae.b8f4.b3de.90dd (`0x5651b3`, `0x567bca`, `0x567eed`); file-offset mapping ev 44be.89a3.f44a.033a.e38d.5b82.b48c.4233.1e65.e089.f368.43b2.1e69.7a9e.b579.e4d9.

### `0x569d89` — clear the 12-byte fill body's unwind frame (support)

- **VA / file offset**: `0x569d89` / `0x169d89`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head; reached only from the 12-byte repeat body.
- **Reached from**: `0x569cd0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev fc7c.6754.e2e7.4705.0b96.63ec.78c7.a502.4c6e.a1a9.bc3e.4507.28b9.e0b7.0359.5950; dossier ev 5f63.cf89.096f.4f49.eade.0c23.7c03.8cc9.8c91.eaa1.5727.f843.10c9.a9bc.91ce.573f; body batch ev 0d46.a2b5.355f.4dc1.4443.5065.6b69.ff45.c5fb.4bc0.fec6.d3ba.ad93.aacb.cf94.0c51; caller facet ev 865f.100d.fd9a.c160.a466.8a6f.ea5c.6a5a.1e13.05f5.588d.a0fa.68ec.7e80.aab8.98c7; callee facet ev 569c.b908.9776.9fe2.4ef2.66e9.b1e9.ea05.d443.1c8b.8de2.9003.4b83.3fab.94a6.b4e1; incoming xrefs ev 7140.c118.c453.25a6.43df.a915.e305.682c.be69.da6a.7fe4.8e2f.6a3b.69d1.b855.e6dd (`0x569d5c`); file-offset mapping ev 272b.5a12.890a.fbf2.a337.33d6.a7ef.54fd.aeef.41b1.dfc9.8926.feff.6c9b.45e0.664d.

### `0x569db0` — repeat a 292-byte prototype into a destination run (support)

- **VA / file offset**: `0x569db0` / `0x169db0`.
- **Behavior**: Copies 73 words per record for the requested count, skipping null destinations, advancing by 292 bytes, then clears its unwind frame.
- **Reached from**: `0x565290`, `0x5680f0`.
- **Direct callees**: `0x569e65`.
- **Player-visible reach**: Support for large-row fill; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec` fill and copy supply the same role.
- **Evidence**: analysis ev 4c4e.50a6.74f5.6934.bd26.3d0e.fbbb.1506.35ae.af63.01e3.4048.b6d9.ef2f.d40d.290e; dossier ev e139.0d9d.8bd2.f21a.87a3.6a75.f5c6.3982.aaae.58a9.11f7.f414.57b1.bec3.42da.2d68; body batch ev 0e7d.84d9.180d.773a.8e7d.5e7f.5ab4.5c13.1945.3ff7.d5e9.f9da.452d.45b7.ca9b.10b4; caller facet ev eb41.b48b.7679.e818.1fee.7c85.e57c.74c8.b895.1f48.06e6.b69d.5798.6c29.4795.4747; callee facet ev f538.c8e0.8185.3ef9.acb2.8540.4062.537f.bf68.d8bb.ea46.f95d.3b07.2e05.f9c4.9710; incoming xrefs ev e057.4d40.d973.506d.e64c.9065.1ff8.7b57.e356.3d1a.355c.39a7.7ba3.9251.6e5f.79c3 (`0x565323`, `0x56846e`, `0x5687c3`); file-offset mapping ev 9ecd.ab71.eea0.62c4.c29b.56e6.1d5e.23a1.85c5.e28a.b1cc.1cb0.d3de.c50a.172d.f804.

### `0x569e65` — clear the 292-byte fill body's unwind frame (support)

- **VA / file offset**: `0x569e65` / `0x169e65`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head; reached only from the 292-byte repeat body.
- **Reached from**: `0x569db0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev 6b9b.a600.358a.2edf.bc26.57d7.413e.857f.1571.b6cb.cb6e.2958.8a86.5874.0e50.91a6; dossier ev 585e.1925.428f.e8c9.34c2.a1a3.278f.5b82.123f.0c52.3c85.0bf1.735d.024b.66f4.141b; body batch ev 37bb.9d4e.ff27.1a74.faac.1b0d.fb81.9f07.818d.72db.622f.79d6.ea64.b1e4.db2c.571f; caller facet ev 451a.177b.10c6.5a06.83fa.46ff.8852.33ef.d014.0c34.884d.bce7.48f0.26b8.e833.19f6; callee facet ev 25e8.6d83.7c11.9ba9.def2.d70a.20df.3e28.ec69.6808.f470.be42.4272.c122.8a5f.ba7f; incoming xrefs ev 4ce3.ab0e.a279.b0a2.f11a.07cd.4b37.521b.8435.79b9.1b82.6e52.d3d3.8492.2b85.911d (`0x569e36`); file-offset mapping ev cf37.b24f.bada.c305.d3c0.dd9f.bc25.6e7c.6d5e.0b2e.5010.b640.8c24.13c7.6506.907d.

### `0x569e80` — repeat an 8-byte prototype into a destination run (support)

- **VA / file offset**: `0x569e80` / `0x169e80`.
- **Behavior**: Copies two words per destination record for the requested count, skipping null destinations, advancing by 8 bytes, then clears its unwind frame.
- **Reached from**: `0x5653c0`, `0x568a00`.
- **Direct callees**: `0x569f33`.
- **Player-visible reach**: Support for vector fill; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec` fill and copy supply the same role.
- **Evidence**: analysis ev c56a.4f65.9e11.a210.a3b4.59a5.ade0.f7bd.4dc5.43ca.5a5d.2b23.7e84.c28c.a868.dea1; dossier ev 78d6.728c.5730.8914.f524.3cd4.c609.5910.a7eb.e461.ab7d.c622.c80f.2023.187f.cd60; body batch ev 2180.db7c.ce93.fe5a.84e6.0069.e3f9.4247.6c64.05d7.2e29.d14f.1918.35be.5ed6.f7ef; caller facet ev d2b8.e29e.5616.9f0a.8412.1b07.b0fc.c961.2bc9.6f0b.6e15.1bfb.95b7.c500.b50a.fc1c; callee facet ev c1a6.e5b5.73bb.6729.bdcc.e498.c385.fcab.7843.373c.22e0.f77d.8fd3.e643.0b99.0aee; incoming xrefs ev 907f.500c.158c.2d83.0017.a367.57cb.1af7.9c55.47ad.02a4.34aa.1df7.9767.b3f6.70f1 (`0x565449`, `0x568d07`, `0x569014`); file-offset mapping ev 0976.2249.13ff.d96d.9fe9.5db4.afc2.1f11.80ee.7106.172a.ee51.73ea.53f9.e8f0.54d0.

### `0x569f33` — clear the 8-byte fill body's unwind frame (support)

- **VA / file offset**: `0x569f33` / `0x169f33`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head; reached only from the 8-byte repeat body.
- **Reached from**: `0x569e80`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev d79f.7634.b7f9.9332.df91.1b71.0634.6854.4774.2c0c.ae6c.7ae3.2c4e.38a8.1587.7140; dossier ev 02ee.583b.1680.8514.abac.d27e.61e3.633a.7e8a.0704.7fb9.2ccb.cd1d.91aa.1913.be70; body batch ev 0508.b5b1.286a.e30d.2f6e.c4d2.ffe2.f2a1.ea2c.b0e2.b001.c7d3.12b3.6446.4530.23c7; caller facet ev 39ef.25e7.ce39.331c.f637.62b4.13e3.5379.91e3.77ca.70f6.d528.a536.ba36.44ee.73e9; callee facet ev d1cd.4fd4.e8ba.94c0.f8d3.303e.bae1.4483.a44e.a934.7600.335b.64f1.eb7f.b17b.1862; incoming xrefs ev 5058.884f.0d37.513b.21e5.53f2.8f75.f7f7.7dab.ca72.b068.5f13.2ef4.1fd0.4101.a75f (`0x569f06`); file-offset mapping ev acf7.ca67.a606.4db3.8e28.9aaf.1932.828d.afef.7f98.2c61.4204.c6b2.270c.5b6b.cc89.

### `0x56a600` — forward to the 360-byte range-copy tail (support)

- **VA / file offset**: `0x56a600` / `0x16a600`.
- **Behavior**: Passes all four arguments unchanged to the 360-byte copy body at `0x56a7e0`.
- **Reached from**: `0x565ed0`.
- **Direct callees**: `0x56a7e0`.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust slice copy supplies the role.
- **Evidence**: analysis ev 0d82.f4a4.9556.c474.b6c0.7e6f.01f9.da6b.d74d.3bd0.938d.4043.00d5.3b0c.d72d.44f2; dossier ev d8e1.0e61.d691.f734.e0ed.4344.1995.ff1d.bece.5385.c592.64fc.f687.7930.75f1.721f; body batch ev b130.bddd.6eda.e81b.eb19.ed82.d757.b0a0.df24.083c.f25e.dc7b.4430.1479.3c63.57c5; caller facet ev 5c83.f73e.6282.8f74.b54d.cb55.8de8.cea3.7991.d66c.44d9.b682.86a8.8387.a166.1e0c; callee facet ev 6903.4068.6782.c62b.3392.9ba5.746b.f159.ff23.8fdb.3ee4.ae00.3867.2843.6ba2.5503; incoming xrefs ev 3819.aa8e.8035.0543.c5ce.283e.9eb7.48fa.def4.4007.f0a1.52b5.1eeb.56fe.0246.0f98 (`0x5661ed`, `0x5662fb`); file-offset mapping ev b52c.aca6.af06.1a1b.9447.6752.42c9.8c72.47df.013e.46e7.5e26.b429.114f.36ed.006d.

### `0x56a650` — forward to the 36-byte range-copy tail (support)

- **VA / file offset**: `0x56a650` / `0x16a650`.
- **Behavior**: Passes all four arguments unchanged to the 36-byte copy body at `0x56a8c0`.
- **Reached from**: `0x5667e0`.
- **Direct callees**: `0x56a8c0`.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust slice copy supplies the role.
- **Evidence**: analysis ev 9c9d.2a42.282a.4939.ee25.9826.76d6.95c2.3d9a.a31c.0b78.f6eb.d7d6.f0a9.ecc9.7d8a; dossier ev a8d4.b708.0999.8eb7.f309.52bc.ca82.4735.e8bd.ae99.d7a7.e9f9.a3bf.5a49.aa96.4c9d; body batch ev 2746.6eb6.30a4.2b63.722c.5575.9c5a.115e.d7a2.f6a7.4e07.13c6.85c1.cfcd.2269.f976; caller facet ev cffb.ad48.3ad2.8fd6.d966.05d9.d435.b877.4f19.7b9e.017d.57c5.26f3.eec3.899f.b6cd; callee facet ev ea34.a13f.4d59.b8a7.642b.b63b.e43b.a625.ee5b.6983.71c4.a013.8eb4.21b9.d8ac.e7d6; incoming xrefs ev 201c.c1d4.87a5.e26c.cc1c.2bb2.b6ea.9af1.0a60.98aa.27ab.623d.fb02.5b3b.7d35.ba4b (`0x566ac7`, `0x566bc0`); file-offset mapping ev b94a.0012.c75f.2738.dbb2.4183.1bfd.84c5.81e4.2a39.fde7.c02b.fc23.162d.d890.e426.

### `0x56a6a0` — forward to the 56-byte range-copy tail (support)

- **VA / file offset**: `0x56a6a0` / `0x16a6a0`.
- **Behavior**: Passes all four arguments unchanged to the 56-byte copy body at `0x56a990`.
- **Reached from**: `0x567040`.
- **Direct callees**: `0x56a990`.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust slice copy supplies the role.
- **Evidence**: analysis ev ef0a.e0a4.6d28.5f17.6e76.aa18.0776.b528.700d.a698.f007.0a38.f556.794e.09fb.e214; dossier ev ed71.f9e8.183d.c4d8.fd0f.70ba.4b23.5fb2.22dd.57e9.b45d.053a.13c1.fe83.3593.0450; body batch ev 9e51.84a9.40a9.94a6.249d.a588.37f3.ee47.b9e3.4b32.534e.b41c.6834.3959.2e48.d018; caller facet ev 321a.4e32.431c.9597.46fc.3d6e.7d43.9c72.de75.f3b5.0bd8.40c8.9427.a210.9837.8e62; callee facet ev 4fda.7424.3cca.c347.92e2.2e10.ffff.cc2e.a639.bdb3.0e39.e830.0ad5.7227.51a7.d5e0; incoming xrefs ev 89f9.19f4.5913.626b.c750.1d3d.4ca2.f1c6.43cf.76e8.f849.7b08.0ebb.f3b6.0d53.089a (`0x567327`, `0x567420`); file-offset mapping ev 7d5a.1612.4ead.4553.04fa.1d05.ce16.71f0.8d66.593e.d7d9.9e44.3389.ed52.69ee.51c4.

### `0x56a6f0` — forward to the 12-byte range-copy tail (support)

- **VA / file offset**: `0x56a6f0` / `0x16a6f0`.
- **Behavior**: Passes all four arguments unchanged to the 12-byte copy body at `0x56aa60`.
- **Reached from**: `0x5678a0`.
- **Direct callees**: `0x56aa60`.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust slice copy supplies the role.
- **Evidence**: analysis ev 9292.2968.b37b.071c.4089.af49.baea.c294.6eb3.e338.0114.9e98.c45b.cd77.61fb.f6b2; dossier ev 8290.519b.dd53.e404.c2f9.92eb.971c.5363.73fb.5313.2170.0083.595a.7724.cbc2.a147; body batch ev b434.f5e9.edc2.09b4.9bd4.03c9.9ca9.60d5.856b.42cc.45ea.bbeb.ef6a.bed6.6fc6.af58; caller facet ev 90f6.1e75.3e82.8388.b6b8.9e58.2723.34b3.bde3.f034.20c4.1d5a.cf53.1e2d.5440.ded8; callee facet ev 2172.aa2a.c1c9.1007.b9da.61fa.97f2.8efa.0d02.1e15.c3e3.7a06.6265.76d7.9975.1ac8; incoming xrefs ev f294.ea59.e795.d115.387e.9671.4115.e888.86b7.af2e.f555.9fee.5581.6baa.c820.ef97 (`0x567b72`, `0x567c6b`); file-offset mapping ev 60eb.026e.46dd.52a5.3c39.df57.5672.c5d6.e99e.669c.122f.812a.30d6.c6a2.65a0.ba3c.

### `0x56a740` — forward to the 292-byte range-copy tail (support)

- **VA / file offset**: `0x56a740` / `0x16a740`.
- **Behavior**: Passes all four arguments unchanged to the 292-byte copy body at `0x56ab40`.
- **Reached from**: `0x5680f0`.
- **Direct callees**: `0x56ab40`.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust slice copy supplies the role.
- **Evidence**: analysis ev 1750.4e51.858f.0cf8.0ff8.2f5d.a458.5260.550f.29d3.c503.0049.fa46.64b1.b913.1be2; dossier ev b2bd.fdd8.fc6a.206f.9d18.22dc.1c8c.54a2.e1a6.2ab6.9bf5.1ad6.06a5.440d.b524.3e66; body batch ev 5de8.4858.cfb8.6f11.7e28.5d0d.29bb.8862.3f20.80aa.7a2c.4d83.d7e4.eaeb.af37.0c36; caller facet ev b25e.bb3c.cd88.ab29.b839.e004.d2c6.622d.165e.a67d.b470.f05b.3c1f.4620.959f.bf36; callee facet ev 46aa.9124.afe0.75c6.2be8.2f07.c484.0afb.a25e.aabc.5ecc.3ed0.d0c9.3627.3cf1.f741; incoming xrefs ev 06f9.1a28.439a.4344.00e2.86f6.fbc5.60a5.4f91.0624.1f66.5602.8d87.792f.57c1.55c9 (`0x56840d`, `0x56851b`); file-offset mapping ev a0b1.1239.bfa4.f1ef.df56.6c6a.4c1b.5bbe.4586.805b.8fca.32c8.5ef0.4118.7fe6.6cfd.

### `0x56a790` — forward to the 8-byte range-copy tail (support)

- **VA / file offset**: `0x56a790` / `0x16a790`.
- **Behavior**: Passes all four arguments unchanged to the 8-byte copy body at `0x56ac20`.
- **Reached from**: `0x568a00`.
- **Direct callees**: `0x56ac20`.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust slice copy supplies the role.
- **Evidence**: analysis ev 5e35.3412.95aa.4b47.5b1e.0ca5.e602.ec4b.bd98.20ef.5a6c.d6fb.e54d.032c.1d13.c1d0; dossier ev 590c.b07d.06a2.2132.bb3a.3ffe.c624.7e1a.057e.2eb4.377a.be4a.8a59.54b2.d03e.f47d; body batch ev 9cbc.2d47.c707.e398.9cb8.4e4a.07a8.b54c.83c0.2a0b.52ad.72e7.386f.624a.36b5.5dd0; caller facet ev a953.c8d8.4479.af59.583b.2f84.7af6.2e91.2663.94a8.c892.f778.6690.2927.c474.f419; callee facet ev 9fb1.f94c.6d7d.5b6c.a8ed.658f.1fd4.56fe.040f.27ea.efb4.c3bd.7d37.8673.2d82.3159; incoming xrefs ev 5bc1.3ca1.2252.47c5.75c2.ed68.df78.a26a.dc00.bacb.b973.80e9.4c17.6e83.6ee4.6269 (`0x568caf`, `0x568da8`); file-offset mapping ev 752b.72b7.c756.a1ed.bb63.7edd.bb89.339b.9392.b9ab.ee77.91d0.ce76.4b00.4dae.6761.

### `0x56a7e0` — copy a half-open range of 360-byte records (support)

- **VA / file offset**: `0x56a7e0` / `0x16a7e0`.
- **Behavior**: Walks from the first boundary to the second in 360-byte steps and copies 90 words into each destination record, skipping null destinations; advances destination by the same stride and clears its unwind frame at the end.
- **Reached from**: `0x569860`, `0x569f50`, `0x56a600`.
- **Direct callees**: `0x56a89b`.
- **Player-visible reach**: Support for insert/copy work; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec`/slice copy supplies the role.
- **Evidence**: analysis ev 5c1b.c181.08cb.2fde.d2d7.db17.3d82.ee69.640c.1510.4881.6be3.aa64.11d2.df35.78ce; dossier ev 4efd.99c5.6e0d.2dbc.e908.a2b5.488e.b69f.74a8.9334.67b2.cd7c.c15f.182a.2a69.7122; body batch ev 0be6.b576.fb9d.1ed7.a15d.44ae.b6d7.82f0.618d.8f96.2040.ab41.1515.57c7.9975.b9e5; caller facet ev f3f5.6d71.208c.4e71.c6fe.b77d.4f5e.a6ee.c828.0302.caa9.5931.0422.cebd.e1d6.132f; callee facet ev fafb.3145.1319.187e.499c.f7c6.2840.ba81.d42c.a98a.ef92.c84e.f978.3461.84a3.d81b; incoming xrefs ev a1f0.c393.4bdc.88eb.0aea.58d4.fc4a.595a.2c57.7f27.07ab.14d4.b25d.ca56.4351.758e (`0x5698ca`, `0x569fba`, `0x56a635`); file-offset mapping ev 176e.f220.37dc.c721.7247.3d86.effa.4efe.790d.6d72.c22c.e61b.fad2.ead9.d45c.25b9.

### `0x56a89b` — clear the 360-byte copy body's unwind frame (support)

- **VA / file offset**: `0x56a89b` / `0x16a89b`.
- **Behavior**: Marks the local frame failed, restores the saved exception-list head and returns the saved return-value word.
- **Reached from**: `0x56a7e0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev bb20.f609.d629.d93b.41ae.a8b6.88b4.7ea5.ed77.83ae.2285.f741.1a7d.3f16.a9a3.597d; dossier ev 07f7.0445.5780.716a.8aa5.5889.f82c.9e07.72d0.c46e.4925.50c3.68d9.b5cb.91c4.85f6; body batch ev e9f2.2a9b.dc48.011d.c489.6af8.2ee2.2aab.e11c.6e5e.59b2.b3d2.c458.4275.3af5.fe23; caller facet ev 27c1.6f53.5bff.d4a6.a12f.90f0.133c.89b6.6eb2.36fc.d4a3.25b6.6d71.3e71.58ec.88da; callee facet ev c482.f30b.90f4.cf79.0629.9cee.1694.3605.63b3.7878.8945.0287.ad62.3ddd.8941.ae3e; incoming xrefs ev 1a95.09a1.d4bc.3b8f.8972.0d1c.43da.14b3.c52e.a303.c92f.2624.f321.7479.1494.a2c6 (`0x56a86b`); file-offset mapping ev 03d3.bb79.a0fa.b5cf.f5b3.59e8.9254.c857.032d.d042.c7a8.3901.1ff0.00dc.4e79.bd86.

### `0x56a8c0` — copy a half-open range of 36-byte records (support)

- **VA / file offset**: `0x56a8c0` / `0x16a8c0`.
- **Behavior**: Walks the source boundaries in 36-byte steps and copies nine words per record into the destination, skipping null destinations, then clears its unwind frame.
- **Reached from**: `0x5698e0`, `0x56a070`, `0x56a650`.
- **Direct callees**: `0x56a972`.
- **Player-visible reach**: Support for insert/copy work; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec`/slice copy supplies the role.
- **Evidence**: analysis ev b9f9.dcda.041e.fe3e.c374.504b.d240.d3ba.4a85.dd26.5382.e85e.04d5.b505.652e.90e1; dossier ev c813.3241.b18e.a960.fec2.a58b.fe31.d2fc.ad13.35a6.5058.1231.d3b6.949b.fdfe.67fd; body batch ev 4ef3.ddeb.637f.db6f.e1eb.ce39.e454.2f6e.944f.ed3f.4a8c.a205.da20.cb59.96a3.f965; caller facet ev c0b6.b5de.7465.6cc6.8b29.2098.8b8d.cef3.8db9.638a.8a45.9b12.7347.0d58.38c2.9dec; callee facet ev 409f.4a18.fa5c.6260.864f.3aa1.e884.63f2.133f.b765.c294.2dea.cdb3.3311.e4d0.0954; incoming xrefs ev 6ce2.1b2f.f7cd.27a5.303a.7aad.19da.2a8e.1870.942a.9572.7b4b.7e29.9aec.9c28.62d7 (`0x56994a`, `0x56a0da`, `0x56a685`); file-offset mapping ev d6ec.5441.6eaa.c81f.36e2.407e.6992.afd4.466f.9ec6.f93c.483a.a2b7.acde.5208.490d.

### `0x56a972` — clear the 36-byte copy body's unwind frame (support)

- **VA / file offset**: `0x56a972` / `0x16a972`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head.
- **Reached from**: `0x56a8c0`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev 8742.ec3b.225e.b33e.18bf.554a.a011.859f.ce88.6d90.700b.0a7b.f4fb.42e3.d90d.64bf; dossier ev 4822.436d.df9e.f0a0.bcf6.a388.b02f.a0f1.294d.d3e3.5f53.4771.b865.580e.a88d.5235; body batch ev 34af.88c8.4917.7da4.132f.cc48.9195.10d0.d9fc.ae50.dea9.bd8b.5142.a2f1.5c65.dae3; caller facet ev 6d83.0924.3bd7.7631.3fb0.4c24.fb98.7407.7254.d68b.1ee4.2881.3e67.17f0.e6e7.6c92; callee facet ev 3c60.61fa.4aab.8e0a.301c.cd2d.79f5.9d92.3b5f.b029.4478.1f4b.337c.bae7.29fa.bfdc; incoming xrefs ev 9b6c.4336.4dcc.e163.d450.25b6.ed1e.e31d.3297.0113.d2c3.6ff9.82d6.faca.9870.df98 (`0x56a945`); file-offset mapping ev 0eeb.01ac.8b84.f588.be26.7eee.9f22.a4a4.d5e5.092d.5e09.1f32.77af.feb1.31f5.c906.

### `0x56a990` — copy a half-open range of 56-byte records (support)

- **VA / file offset**: `0x56a990` / `0x16a990`.
- **Behavior**: Walks the source boundaries in 56-byte steps and copies fourteen words per record into the destination, skipping null destinations, then clears its unwind frame.
- **Reached from**: `0x569960`, `0x56a190`, `0x56a6a0`.
- **Direct callees**: `0x56aa42`.
- **Player-visible reach**: Support for insert/copy work; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec`/slice copy supplies the role.
- **Evidence**: analysis ev 9246.586c.1173.b8a7.5858.3f14.3a3d.e7eb.1524.8c28.0b3c.6d30.c736.541c.0e08.6419; dossier ev 09ff.6ca2.b83c.1205.26c5.9f14.e494.13e5.d958.7a6a.821a.06ba.2584.05d4.f9f5.677c; body batch ev 2fa8.13c0.d3c4.d1d8.f06b.b9b4.c15e.7ecc.bebf.700a.4a3e.e003.a27a.7ad3.2d96.a32d; caller facet ev 28f9.74d9.d868.405d.4fe4.2f79.aa44.d14f.55cb.edb1.818c.394f.63cd.5c22.e3ec.7a83; callee facet ev 9989.658f.72bb.a33e.10a1.7b32.8752.1b66.fcee.add5.2145.7800.13b4.93c4.6d6e.2bca; incoming xrefs ev 3a99.03ea.5e9b.d9fb.7502.27e7.df1d.b0a8.9fa3.a54e.ca52.9ee2.3a23.c443.f200.588f (`0x5699ca`, `0x56a1fa`, `0x56a6d5`); file-offset mapping ev 859d.a667.9c95.7c73.3d04.a4b3.dfcd.29d3.cc59.1e17.7559.d02a.734e.173f.0e81.89a9.

### `0x56aa42` — clear the 56-byte copy body's unwind frame (support)

- **VA / file offset**: `0x56aa42` / `0x16aa42`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head.
- **Reached from**: `0x56a990`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev b057.e8b2.71e1.da2a.f8a5.072b.6744.932e.6ac0.5f54.8c6d.a79c.79b3.2fad.33c1.f41d; dossier ev c4a5.fe89.fba4.733d.e98f.b5ba.c0c3.adc4.b51a.ccf7.076b.f530.b7c9.b895.3228.fa50; body batch ev 7544.10a9.1021.fe81.771e.afed.7918.fa12.3f00.0c95.98ef.a970.e033.cdda.c1ed.dbba; caller facet ev 59b9.aed9.e91a.78e8.dd8f.d307.de19.6ad0.252a.5f31.2d7f.a008.4b9e.ef09.8678.5fc0; callee facet ev 26ab.ead2.1178.70cd.773e.2719.a000.23e5.3e9e.70c4.a6c4.39b4.4730.2788.a86c.2350; incoming xrefs ev 0b44.0033.e609.cc04.8da1.747c.1353.b8dd.3aff.6135.edd2.9cca.97cf.f996.c42c.a435 (`0x56aa15`); file-offset mapping ev 98f1.7569.d6c0.5ce0.6af7.e44a.6df6.212d.2732.c771.38b7.0e39.e243.a9f1.2311.41dd.

### `0x56aa60` — copy a half-open range of 12-byte records (support)

- **VA / file offset**: `0x56aa60` / `0x16aa60`.
- **Behavior**: Walks the source boundaries in 12-byte steps and copies three words per record into the destination, skipping null destinations, then clears its unwind frame.
- **Reached from**: `0x5699e0`, `0x56a2b0`, `0x56a6f0`.
- **Direct callees**: `0x56ab1b`.
- **Player-visible reach**: Support for insert/copy work; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec`/slice copy supplies the role.
- **Evidence**: analysis ev 0db7.8115.ea58.836c.f36a.5310.f471.2d00.5d83.f524.c675.987b.b565.2470.9f85.058d; dossier ev f73a.5f88.6e1e.e935.366f.3647.0b84.2229.67a2.90d5.427f.4232.a7df.f08c.4122.c64f; body batch ev 8470.147d.6b71.ae61.5117.2cd0.afd6.bbd8.d8ab.c3f1.d589.0e47.5aef.043a.03c9.92e3; caller facet ev 9563.6c99.8c56.e70d.e543.d14f.25c5.f92b.506e.72b1.4f93.18a5.65df.9453.1fe3.9deb; callee facet ev 8f7c.a23d.9a4b.5be0.fb80.9e97.0238.2119.50db.71a0.dfd6.b447.7ae3.4c71.3e12.f9bf; incoming xrefs ev 06cc.f522.5f82.d521.edfe.ab22.233d.e18f.966e.0bb1.809a.f5f2.d058.f282.bdae.d4f8 (`0x569a4a`, `0x56a31a`, `0x56a725`); file-offset mapping ev 7f3b.6c18.3385.25d7.1ca9.48b3.55ea.6855.d899.7ff5.f46a.7cda.8bbd.1f83.7cbf.4afa.

### `0x56ab1b` — clear the 12-byte copy body's unwind frame (support)

- **VA / file offset**: `0x56ab1b` / `0x16ab1b`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head.
- **Reached from**: `0x56aa60`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev 52b2.168a.1dc4.4638.70fc.7718.c147.5a94.842c.41ed.fc4c.1f20.483c.7f47.3108.a6b1; dossier ev e0b7.9920.284e.e7ef.7e39.fd43.2019.cfbb.c87f.d9d7.0bb7.db23.c6b4.9134.82e9.3179; body batch ev 0a89.7eff.4c96.2c12.29fa.ae31.765d.7ee2.8452.f874.d6aa.2977.cc31.193a.b8ce.aebd; caller facet ev 1d12.c970.8a6b.6568.acb7.041d.b816.55ff.9631.a8e2.cc03.f783.8af9.2e19.7317.6894; callee facet ev 27de.aff8.569b.6339.e9a5.b267.f274.1aee.d0e1.665b.e23a.977a.cbf8.93b3.d456.0da1; incoming xrefs ev 5ff8.c2e8.19a8.6d0e.4806.0162.78b9.7beb.4709.c4cc.354f.6d54.ef4a.8011.3f23.23cf (`0x56aaee`); file-offset mapping ev d83a.09de.89d3.22fb.26b5.627e.527d.38aa.260b.3aaf.f98b.d3ce.7f57.4fd9.9cf6.2ac0.

### `0x56ab40` — copy a half-open range of 292-byte records (support)

- **VA / file offset**: `0x56ab40` / `0x16ab40`.
- **Behavior**: Walks the source boundaries in 292-byte steps and copies 73 words per record into the destination, skipping null destinations, then clears its unwind frame.
- **Reached from**: `0x56a3d0`, `0x56a740`.
- **Direct callees**: `0x56abfb`.
- **Player-visible reach**: Support for insert/copy work; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec`/slice copy supplies the role.
- **Evidence**: analysis ev 4157.f6ad.5416.0dc0.6b47.4d0f.769a.fbd4.5aef.a4dd.b7ca.ebac.dc60.7608.7c6d.f12a; dossier ev 693c.f743.cb61.2550.71fe.aeec.9ee6.b940.4e8c.5ff8.0d85.63cc.d1df.223d.6f1a.259d; body batch ev 5169.626b.30b3.7a6e.627c.7f19.1b03.787e.c2a3.7e6a.52fc.44fa.5902.b106.bede.0ac7; caller facet ev 0c84.1399.cc0a.8857.f8bb.3ebc.9734.6669.c4f6.d807.be20.7b62.eb69.31ce.011f.e75e; callee facet ev 0337.3090.e36a.d5e0.fa98.021a.b9b7.3ff3.24e0.4008.5f06.3674.a282.3490.5a8f.5264; incoming xrefs ev 365d.cc5c.f560.7cae.469e.a18e.58c8.8d5a.72a4.1374.4f05.e3ae.394d.484e.00a0.388a (`0x56a43a`, `0x56a775`); file-offset mapping ev 54c3.4d81.94b7.96a7.8314.bc5f.737c.8d9c.fd29.0465.59a8.9d9f.6118.d004.97e3.041b.

### `0x56abfb` — clear the 292-byte copy body's unwind frame (support)

- **VA / file offset**: `0x56abfb` / `0x16abfb`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head.
- **Reached from**: `0x56ab40`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev bc74.2c6a.7adb.c569.99f4.9d61.0cf0.40b1.1bcb.d831.d1a1.5610.d485.acfc.4f19.197b; dossier ev f7c4.462f.b93e.47e4.8980.f1d3.6dbd.8004.360f.9232.8eca.1392.db1c.0928.be4f.f1de; body batch ev 9379.01b3.63e8.b956.607c.d501.6a48.c525.1c9e.a19c.e853.9b3c.c90d.3362.7105.375a; caller facet ev 0a94.d92e.c67d.e1d0.0512.ad36.e068.1691.eebc.5c3f.1668.e85a.3499.cf30.831b.9a29; callee facet ev 560e.dcfe.4bd2.bc7c.7381.766d.1753.ca33.7c29.a962.c613.5139.85c4.19e3.0656.1204; incoming xrefs ev a88e.39fa.17b9.945b.32ef.401e.7286.37a9.ce16.3f33.65e7.3a73.8db4.ba71.b361.f55b (`0x56abcb`); file-offset mapping ev 77bd.c45e.d9b0.ab83.8481.126d.a156.925f.8c49.3054.9e31.2080.f4bd.7ef3.83d2.71f2.

### `0x56ac20` — copy a half-open range of 8-byte records (support)

- **VA / file offset**: `0x56ac20` / `0x16ac20`.
- **Behavior**: Walks the source boundaries in 8-byte steps and copies two words per record into the destination, skipping null destinations, then clears its unwind frame.
- **Reached from**: `0x56a4f0`, `0x56a790`.
- **Direct callees**: `0x56acd5`.
- **Player-visible reach**: Support for insert/copy work; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec`/slice copy supplies the role.
- **Evidence**: analysis ev 1a0c.263d.4298.7b84.cbd4.9d76.180d.ef25.87c0.ee95.6a1b.d993.4268.501d.9815.b38a; dossier ev c0fd.b0d1.d7f4.6a2b.2a1d.d846.8323.50f4.e238.0515.7f36.5968.e154.1b07.be69.5437; body batch ev ac04.8421.6cd7.7902.4178.601e.5d13.0d98.dfd9.f70a.0b34.046e.4818.9ebb.dd14.1b1e; caller facet ev a15d.5eac.2c04.52ba.a60a.6167.530d.36ac.17d1.ccd7.50dd.6781.87cb.6f41.1b1d.f960; callee facet ev 1884.15d9.31a7.7c63.a10c.1ae8.fb41.a519.ba9a.f08d.bfc6.560f.0737.4fa5.f667.5051; incoming xrefs ev c074.eaa6.a998.589b.d7a8.467e.67f7.45ec.a27e.102f.a963.cd4a.63ad.62d4.9a83.f561 (`0x56a55a`, `0x56a7c5`); file-offset mapping ev 4cd1.ce2d.f462.03e9.8606.b891.d9b9.22b8.adbe.a664.de69.71ed.f675.906c.e860.01c1.

### `0x56acd5` — clear the 8-byte copy body's unwind frame (support)

- **VA / file offset**: `0x56acd5` / `0x16acd5`.
- **Behavior**: Marks the local frame failed and restores the saved exception-list head.
- **Reached from**: `0x56ac20`.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; compiler-generated unwind code.
- **Evidence**: analysis ev 7f3c.d94d.f3c7.e11c.ae4b.b4f7.5227.221d.7bf1.2877.3c92.32e0.7d23.d362.20e2.18a5; dossier ev 1780.f1ad.e8ed.5c40.6d39.b3dd.1a13.40da.dbd1.ec76.6fe7.7876.f52b.12a5.53df.bb99; body batch ev e8ab.9192.05b1.b43b.0032.5bb4.6870.bf6f.a2ef.20e6.f98c.32ca.5060.d0e4.161e.472c; caller facet ev d12c.9b3b.88dc.0909.d168.7e7c.6069.9b9e.68c9.50fe.3c7f.06c8.9337.279d.c9ba.5e69; callee facet ev f245.0d70.e0a3.45de.9c09.f1a5.15f7.ca77.f2b7.d495.6ab3.974f.4d8e.a63f.e4b8.5d61; incoming xrefs ev a249.cba6.a6d9.c385.4f5b.50bc.cf81.6d79.b06f.5ebb.eb15.8b81.7bc8.b814.d1bc.e2f5 (`0x56aca8`); file-offset mapping ev 5b3f.38a6.86e9.d1be.a6b6.ed90.cda7.5f67.61e6.f9ab.90d7.fa79.7597.4eb6.505c.8bb0.

### `0x56ad00` — initialize the Playing state's first embedded sprite surface (support)

- **VA / file offset**: `0x56ad00` / `0x16ad00`.
- **Behavior**: Calls the shared sprite-surface initializer at `0x584d20`, which installs the surface vftable and clears three words, then returns the same object pointer. The Playing constructor calls this as its first initialization stage.
- **Reached from**: `0x4d8940`. This is init stage 1 of the `CPlayingGame` constructor.
- **Direct callees**: `0x584d20`.
- **Player-visible reach**: Support; no pixels or wire on its own.
- **Our client**: No counterpart by VA; Rust playing state lives in `crates/o2jam-shell/src/playing.rs:1509`.
- **Evidence**: analysis ev df8b.adec.2e24.0d7f.8025.6c5f.1563.14e8.1b3b.259d.3264.fb46.c755.3439.7e85.83bc; dossier ev ec2b.59cc.1b73.70e4.97a8.61fb.3e0d.1ffb.f8cd.de65.27cb.7c2e.712c.2af6.8623.bb28; body batch ev ce4a.be0c.8b64.31c7.a454.9379.55cc.4419.8c2b.8e77.90a7.9099.935b.d2c2.3cff.2fb9; caller facet ev a76f.ecd9.4cd5.0b76.85f5.4ce8.6825.1da9.a41a.6b41.6483.c8bc.156a.184a.4f7c.8bc8; callee facet ev 1b18.1471.b1e4.22d3.cdc3.b459.8a2c.b885.cbab.8465.e7e0.14d3.e964.0d48.b362.d3ae; incoming xrefs ev 495a.a3a2.ce6a.36ff.b6d6.2bca.6c85.6823.8ab0.b87f.4465.24e7.3c1a.aa9e.5a2b.feb3 (`0x4d89d4`); file-offset mapping ev b7fc.a2c9.4953.4f73.5d5b.ee5f.687f.2ca9.680b.acec.1fe2.5f13.320b.46a1.9bdd.9e7d.

### `0x56ae40` — initialize the Playing state's second embedded sprite surface (support)

- **VA / file offset**: `0x56ae40` / `0x16ae40`.
- **Behavior**: Same shared sprite-surface initializer and clear, used as the Playing constructor's second stage.
- **Reached from**: `0x4d8940`. This is init stage 2 of that constructor.
- **Direct callees**: `0x584d20`.
- **Player-visible reach**: Support; no pixels or wire on its own.
- **Our client**: No counterpart by VA; Rust playing state lives in `crates/o2jam-shell/src/playing.rs:1509`.
- **Evidence**: analysis ev f2c1.a3d9.74af.d234.ccf5.8e6b.525f.82ab.4246.3841.56e3.b83d.d9d0.de16.70d5.ca59; dossier ev 2963.f92a.2375.0827.09a1.ebf3.a1dd.738f.d1bc.06f5.9983.346e.ae8e.9ab2.2f0b.5d88; body batch ev d7b7.b0fb.3b1a.c66f.7865.83ab.5cf4.6498.5889.a119.59d8.7674.83f4.ca07.2a01.193f; caller facet ev 8ff6.6a45.2af6.f6a9.be8e.0771.5f9d.b5bd.a72b.1a7c.0f49.a0d1.632c.13fc.c44a.e128; callee facet ev e4e1.9edd.440a.0779.19f4.6bda.abcf.8b9b.e4ce.b426.4f36.75c9.4395.e47a.0d0c.5812; incoming xrefs ev 6a2a.a1b9.0da9.516f.a138.6ef1.2335.9bad.30a1.cd55.fa6a.beef.e495.6c2f.2f49.c966 (`0x4d89e6`); file-offset mapping ev c100.48ac.0c89.0c98.69f7.cdc3.8161.14f1.2a79.e400.bd2d.e26c.bd2c.3869.a6c1.5009.

### `0x56af90` — initialize and clear the Playing state's third embedded surface (support)

- **VA / file offset**: `0x56af90` / `0x16af90`.
- **Behavior**: Calls the shared sprite-surface initializer, then clears one additional word in the object and returns it. Playing constructor stage 3.
- **Reached from**: `0x4d8940`. This is init stage 3 of that constructor.
- **Direct callees**: `0x584d20`.
- **Player-visible reach**: Support; no pixels or wire on its own.
- **Our client**: No counterpart by VA; Rust playing state lives in `crates/o2jam-shell/src/playing.rs:1509`.
- **Evidence**: analysis ev 2155.a545.4350.2c14.5f8f.93e7.893c.9813.ffa2.623f.e663.d280.d354.4bb2.3b27.ffa7; dossier ev 695e.c8e2.969b.a443.10f4.b544.18a9.6988.63b7.8857.6d0c.6077.3b6b.b8a3.3f82.b8a9; body batch ev 8549.bfcb.6627.4838.c82a.5df0.684b.278a.71b6.c901.89aa.7538.cf01.d39a.db3c.b306; caller facet ev c371.6d11.2516.9f25.5979.cb7c.53e1.7e61.948b.037a.527d.f56c.bb25.2dd0.f6d7.601b; callee facet ev 711e.771f.bbcd.07a3.6973.8404.e8f0.569f.7fff.21df.ec6f.81ba.ba70.b388.ddf5.c3c2; incoming xrefs ev 5fac.5051.ff4c.0d0b.d6e5.174e.63cb.0d8a.17f6.c826.168b.16fe.c5c6.dd93.b849.b7b5 (`0x4d89f8`); file-offset mapping ev 90c3.8504.29ea.3a90.444f.dbb4.fcad.6379.b085.4c60.1b42.908c.15d5.534a.4643.a9a9.

### `0x56b0b0` — zero the Playing state's fourth embedded surface block (support)

- **VA / file offset**: `0x56b0b0` / `0x16b0b0`.
- **Behavior**: Clears six consecutive words of the object and returns it. Unlike the three sibling helpers it does not install a vftable. Playing constructor stage 4.
- **Reached from**: `0x4d8940`. This is init stage 4 of that constructor.
- **Direct callees**: none observed.
- **Player-visible reach**: Support; no pixels or wire on its own.
- **Our client**: No counterpart by VA; Rust playing state lives in `crates/o2jam-shell/src/playing.rs:1509`.
- **Evidence**: analysis ev a77e.ce55.5f21.c32b.ed41.a589.127b.a0ef.004f.4208.a7ed.9849.10fc.b463.72c8.a144; dossier ev 6866.bbb2.00b0.e045.a744.9cdc.a029.5b85.95c0.ab01.f78c.bffc.6cb7.f326.23a2.4ca0; body batch ev 2b8a.0604.5df7.dbae.ab16.1239.6048.c70f.954a.ea85.9c1c.443f.5c13.be8b.420d.7dbc; caller facet ev 8b04.2b9f.6d6b.f393.2884.5dd1.b563.9bac.623c.8e4c.69d1.dc21.dcf8.2764.8c76.b6a6; callee facet ev f40d.8440.360d.654b.1695.d873.e58d.0a22.2abd.3a41.fae8.aeb0.8aa0.96e2.f001.943f; incoming xrefs ev 147c.0de7.5548.13df.d7d6.b7e6.fa24.622e.b154.da1c.e8cc.f87d.6e35.8d5a.51f5.c9b5 (`0x4d8a0a`); file-offset mapping ev 0de0.cd60.02ab.40eb.e85b.c638.6872.7a12.ecaa.457c.c5d6.32fc.dc73.d23e.5204.61f3.

### `0x586470` — read the 20-byte 4049 latency answer and forward five words

- **VA / file offset**: `0x586470` / `0x186470`.
- **Behavior**: Calls the shared fixed-size reader at `0x555d20` to fill the object's `+0x18` area with 20 wire bytes, then passes the five 4-byte fields to `0x5864c0`. That consumer matches the first field against the local member byte and, when the object's `+0x1e0` flag is set, derives a scaled time-difference value from the second field against two object words and stores it in an eight-byte row slot; otherwise it sets a per-member/per-index word. A nonzero fifth field sets another per-member flag.
- **Reached from**: `0x563db0`. Direct caller `0x563db0`; that function is the packet-dispatcher arm for case `0xfd1` (4049). Dispatcher dossier: ev 9328.353a.2da1.5afe.caa5.2002.9fe4.232f.fbe7.50c0.a588.1a74.0abf.feaa.5c12.2b15.
- **Direct callees**: `0x555d20`, `0x5864c0`.
- **Player-visible reach**: Player-visible support: the 4048/4049 exchange runs after 4013 in live Playing wire logs (`docs/wine-boot.md:1017`); the body reads and stages the answer. No pixels are painted in this body.
- **Our client**: No codec or receive handler by VA. `docs/dispatch-inventory.md:140` still lists the 4049 body layout as OPEN; `docs/net-protocol.md:121` lists the opcode. Live capture wire logs exist, but no scripted-gateway leg asserts the five-word stage.
- **Evidence**: analysis ev 8908.f66c.90a7.f1c6.2189.f7f3.ac92.b346.3945.9d0d.48d3.051e.758a.9a7e.ec2e.4572; dossier ev 5837.31bf.1c7e.895d.abdd.5cfd.05e3.ccb0.4ac0.ad8e.32ba.fb45.85dc.7262.848b.5856; body batch ev 3ffe.2ae3.205e.2e11.8fef.68e6.088e.678d.90f7.c344.851f.f29b.a039.26c7.3552.2916; caller facet ev dd9e.f34d.8813.9c03.8dba.63cd.0529.7b44.5727.77c3.a573.4b72.8e69.671f.8658.a774; callee facet ev a050.df5e.dd44.d814.10e7.dc2e.a8ed.a7cf.4694.9949.04a0.9685.c714.eb9f.380f.0351; incoming xrefs ev dcbc.0f32.6242.2726.8e5f.4239.4748.0ba9.6c3b.738d.88ea.eb90.07ad.b4f0.0662.9c55 (`0x563def`); file-offset mapping ev c7e3.ec8d.7c4c.e951.9134.17b3.2721.53e7.a52d.39b9.a9fb.0637.d72c.d758.f807.7fee.

### `0x58cf50` — stage the peer endpoints of the 4045 relay answer

- **VA / file offset**: `0x58cf50` / `0x18cf50`.
- **Behavior**: Resets the peer table with two calls to `0x5afae0`, seeds the local member through `0x58d4b0`, runs two table setup helpers, reads a signed 4-byte count through `0x555d20`, and for each peer reads a 4-byte member word plus two 6-byte endpoint blocks (four-byte address and two-byte port each). Each row is staged through `0x58d510`; `0x58d030` closes the table. A negative count yields no peers.
- **Reached from**: `0x563d70`. Direct caller `0x563d70`, a forwarder from packet-dispatcher case `0xfcd` (4045). The same layout is pinned in `docs/relay.md:447`.
- **Direct callees**: `0x555d20`, `0x58d030`, `0x58d050`, `0x58d4b0`, `0x58d4d0`, `0x58d510`, `0x5afae0`.
- **Player-visible reach**: Player-visible support in live battle only: the staged endpoints feed the relay peer table. The owner domain pin keeps relay paths in live battle; no normal Playing gate follows from this pin.
- **Our client**: Codec `P2PListResponse` at `crates/o2jam-net/src/relay/commands.rs:1383` (peer type) and `:1409`/`:1414` (body decode) matches these widths. No receive-side staging driver by VA in `crates/recomp2jam`.
- **Evidence**: analysis ev 41b6.1271.1f0d.3c28.16f9.1f83.a124.8c35.37c8.f443.ee3e.f864.b518.07a8.416c.50e8; dossier ev c8f7.b31a.d4f6.0867.aa25.d857.4d3d.0dde.f06c.1018.b292.6675.efdc.a02f.1cd8.7b6a; body batch ev e5ea.42b8.58b9.4ee4.5a92.3ca9.0d05.f4f8.f840.5884.ea59.dbaf.aa3d.1f68.c159.3a80; caller facet ev e796.4e1d.f6c8.b9c3.cc72.c0b8.3a2e.bafd.a423.468a.63bc.40fe.5f01.486f.4a6b.a358; callee facet ev 5e86.60b5.388c.7c68.5aea.c322.58f5.fec0.3a4c.bebd.d743.0ad0.7d5b.81b6.fff9.300e; incoming xrefs ev d61d.8c5c.845c.44e8.da53.57c0.4f25.bf1b.0912.5434.a26a.fde5.3add.9b5b.fbdc.b513 (`0x563d7d`); file-offset mapping ev f095.1cb9.d2d0.9081.be97.3b08.16c9.cf71.29a2.7b0b.3823.4831.e19d.b277.8bbf.084e.

### `0x5a49d0` — write the F8 cursor mode to the registry and raise its notices

- **VA / file offset**: `0x5a49d0` / `0x1a49d0`.
- **Behavior**: Reads the current cursor mode through `0x5a4b50` and acts only when the shared UI flag at `+0x914` is set. A zero request enters the write arm directly; a nonzero request with the option currently off first shows the image-cursor caption at `0x650c58`; a nonzero request with the option already on instead asks at `0x650cb0` whether to return to the Windows cursor. The write arm toggles one bit, creates the subkey returned by `0x5a8a30` under the current-user registry root, writes a four-byte value named `Mouse_Cursor`, closes the key, and applies the mode through `0x5a4b10`.
- **Reached from**: `0x48c360`, `0x5a50d0`. Direct callers: `0x48c360` (dialog-confirm dispatcher, case `0xffb`) and `0x5a50d0` (F8 key gate). Caption bytes were read from the artifact: ev cdde.31ff.12ea.5e88.33d7.6959.286a.cd28.f645.d7e0.50b8.8d35.a1a7.58bc.c321.68b6 and ev 6b7f.1562.a35d.10b7.d2be.2e44.112b.57e4.901e.3b1e.ee30.ea3d.529d.647d.0087.5169.
- **Direct callees**: `EXTERNAL:0x5e`, `EXTERNAL:0x5f`, `EXTERNAL:0x60`, `0x478c90`, `0x5a4b10`, `0x5a4b50`, `0x5a8a30`.
- **Player-visible reach**: the F8 cursor switch changes the pointer image and persists that choice across runs. This body performs no direct paint.
- **Our client**: No counterpart by VA. The amber cursor in `crates/recomp2jam/src/shots.rs:119` is a shot stand-in, not a runtime option; pointer events are routed in `crates/o2jam-shell/src/window.rs:554`.
- **Evidence**: analysis ev f3e3.9d04.e30a.df4a.a3f9.d73b.e44e.f75c.b166.9e28.8534.44e2.5792.6b7c.4c52.f23f; dossier ev d692.513f.ad00.396a.42ff.085f.fddb.6535.f29f.9eef.159e.9afc.3e1b.a059.8fbd.903c; body batch ev 0123.a545.cd34.5f10.7002.bcac.70dc.0a76.5975.c602.e0b2.b066.6d8b.e3e4.3ad2.a1d2; caller facet ev 5d8b.565e.6b8d.6508.01ce.0c78.fc9f.b1fd.4c71.46b2.5539.b93f.d153.6073.7891.6555; callee facet ev 09a4.dc30.6db9.dced.cfcf.f975.58f5.df9e.81ed.faf6.c5fa.3874.75e6.906a.7114.9585; incoming xrefs ev d0a9.19cf.1aba.7ddd.1fc0.5707.1cce.d9c9.eef9.2d49.a96b.4a70.4c5c.d2ef.1951.08e6 (`0x48c413`, `0x5a5180`); file-offset mapping ev 3a6d.dfbc.a462.be40.1eba.61b5.cc63.e5d8.e09e.aa5c.68ed.53c4.7845.86fa.85db.86a8.

### `0x5a4b90` — handle application focus change and the Playing-scene exit notices

- **VA / file offset**: `0x5a4b90` / `0x1a4b90`.
- **Behavior**: Derives an active flag from the activation value (1 or 2) and stores it at `+0x14` plus a one-shot marker at `+0xc`. On deactivation it suspends audio through `0x551850(0)` and sets the pause flag at `+0x18`. On activation it clears that flag and restores audio through `0x551820(-1,1)`; in Playing it checks the focus bit at `+0x1c` and either shows the Room-exit caption at `0x650d68` for the loading scene, or resets the recorded score/life globals, applies the other reset helpers and shows the Lobby (`0x650d38`) or Waiting (`0x650d08`) caption. In scenes 9, 8 and 2 it refocuses the stored window handle; event class 5 triggers `0x5426d0(0x314)`.
- **Reached from**: `0x5a5190`. Direct caller `0x5a5190` on the `WM_ACTIVATEAPP` message. Caption bytes: ev 09d7.e5ca.a446.82f2.4d4a.1aa8.ed6c.fdad.087f.d557.dca4.f09f.3123.63f1.f0f2.171b, ev fe5d.5722.efc6.2768.ba19.8c6c.299e.b2cb.9e05.c115.4397.0b4a.5b97.f305.9d7a.0481, ev 89ce.132c.5c18.9cad.512f.7a1c.e6d0.716e.8af0.bce2.9540.059b.0db0.6162.b797.cacc.
- **Direct callees**: `EXTERNAL:0xa0`, `0x416a80`, `0x417870`, `0x418d90`, `0x431dc0`, `0x478a00`, `0x478a60`, `0x478c50`, `0x478c90`, `0x485dd0`, `0x4863a0`, `0x53fd10`, `0x53fd30`, `0x53fff0`, `0x5400b0`, `0x5400d0`, `0x5400f0`, `0x540110`, `0x541460`, `0x5426d0`, `0x551820`, `0x551850`, `0x5a4b30`, `0x5a8b20`, `0x5f1bc0`.
- **Player-visible reach**: losing focus during a song exits to a named scene with a notice; activation also gates audio.
- **Our client**: No counterpart by VA. Winit events reach `crates/o2jam-shell/src/window.rs:554`, but no exe focus-scene exit chain is wired.
- **Evidence**: analysis ev 3f40.f126.8a35.d370.4085.15ff.8526.fdd4.2d44.f953.19e6.ca13.647c.f2d7.7a61.9ca2; dossier ev 5ee2.5af6.8620.de33.658c.288d.a482.288d.b584.9eb9.ef12.dde5.2e56.13ae.af48.5e9a; body batch ev b314.42b8.f147.e70b.3c96.6e28.8764.cbd0.0f32.ca6a.71b9.de2e.6b33.3347.a3cc.b9d5; caller facet ev b7a7.0d47.b14e.3709.ca02.f117.deb3.a9c3.d349.d416.27a6.af6a.18b1.f9be.c792.dbe5; callee facet ev 62e7.adc6.395d.c9cf.1f6d.6bb8.a0e9.cd35.ef35.b922.7a15.0be0.040b.c925.f234.7b61; incoming xrefs ev b6f3.7cd7.7dde.eb26.c51b.f4d0.13fe.4b85.e34b.181e.4abb.16d6.c66b.3e88.a144.5c99 (`0x5a522e`); file-offset mapping ev f6de.f67a.d22a.9c0e.ecff.0b69.07a7.568b.9148.0b4f.d172.e4e5.942f.00e9.c749.b4fa.

### `0x5a50d0` — gate the F8 key press by scene and forward it to the cursor option

- **VA / file offset**: `0x5a50d0` / `0x1a50d0`.
- **Behavior**: Requires the shared UI flag at `+0x914`, records the key through `0x479cb0`, and then checks a list of scene ids and a second event word before calling `0x5a49d0(1)` for the F8 key code (`0x77`). The listed scenes are excluded, so the cursor switch is unavailable there.
- **Reached from**: `0x5a5190`. Direct caller `0x5a5190`, its `WM_KEYDOWN` arm.
- **Direct callees**: `0x478a00`, `0x478a60`, `0x479cb0`, `0x5a49d0`.
- **Player-visible reach**: F8 is accepted in the allowed scenes only.
- **Our client**: No counterpart by VA; key routing exists in `crates/o2jam-shell/src/window.rs:554`, but no F8 branch or scene-exclusion list is present.
- **Evidence**: analysis ev d7e6.9453.5df7.438b.1c87.30de.a6c2.bdf7.0021.6b9f.1daa.03c2.7543.ddf8.bf6a.d1ec; dossier ev cb4b.3830.dc45.40d5.7329.1121.012d.03e5.fcf4.f61a.ff53.be51.3698.b508.8ab3.b0a3; body batch ev 2f59.2b48.1778.d19c.d234.5274.bd65.f348.6c3d.56bd.d9a7.9f93.679c.d31d.0454.47c9; caller facet ev 4971.22a1.df5c.7ee8.3a2a.b821.2bbe.6a0a.cf31.98b5.4811.1de3.0afa.a33c.4a10.e727; callee facet ev d45a.d7a6.468f.a3c8.574b.d528.c7c2.5a12.9d69.2c8e.6291.02c1.be71.73eb.cf5c.d5a0; incoming xrefs ev 7b51.2743.4c13.97cf.2cf9.5d30.2f49.c7f5.c490.b453.1d29.9c33.62f8.a96f.bf89.cfde (`0x5a52cc`); file-offset mapping ev e06c.0fdb.d83c.bc45.f64d.7592.0833.c513.6aa4.d53f.c7df.22c2.1ef7.e84a.5dd7.46c4.

### `0x5a5190` — route main-window messages to the shared handlers

- **VA / file offset**: `0x5a5190` / `0x1a5190`.
- **Behavior**: Dispatches a key-up code `0x2c` to `0x418dc0` outside scenes 10 and 11; destruction to `0x5a5020`; activation changes to `0x5a4b90`; focus to `0x5a5040`; key-down to `0x5a50d0`; the system close command (`0xf060`) to the exit-confirm body `0x5a5600`; and the `WM_APP+0x32` message to three UI-event consumers plus the scene-sheet closers `0x495a30`, `0x4f3d00` and `0x4f5020` for scene ids 2, 12 and 13. All other messages go to the default window procedure.
- **Reached from**: `0x5ac690`. Direct caller `0x5ac690`, the window creation/boot path.
- **Direct callees**: `EXTERNAL:0x7d`, `0x418dc0`, `0x45e450`, `0x45ecd0`, `0x45f480`, `0x478a00`, `0x478a60`, `0x478ad0`, `0x495a30`, `0x4f3d00`, `0x4f5020`, `0x5a4b90`, `0x5a5020`, `0x5a5040`, `0x5a50b0`, `0x5a50d0`, `0x5a5600`, `0x5a8ba0`.
- **Player-visible reach**: this is the input and window-event hub for every scene.
- **Our client**: No Win32 WndProc in the Rust client; the equivalent entry is the winit event loop in `crates/o2jam-shell/src/window.rs:554`.
- **Evidence**: analysis ev ef50.a9ad.a383.cfaa.f038.eefb.dfd9.6333.c757.d6aa.8339.3521.db7d.83d7.f6af.0768; dossier ev dbeb.88ad.7f31.83f8.9d0c.b173.ce38.b3b2.8bab.55c5.8662.2866.5ba0.cef4.2a9b.d42a; body batch ev bff5.9db0.5763.0369.7ea7.092e.9ba8.50b8.997b.0dde.d20b.7e80.df67.c14f.d7ef.5374; caller facet ev 5586.ae45.9a9f.22a0.b847.8532.9fb0.6469.aeb1.54b8.c6aa.7c81.e891.67eb.52ad.3238; callee facet ev 3a9e.ea7d.0be2.c152.6d0d.8b3f.1dbb.bcfb.8317.f6b5.46ff.6758.5042.d856.ecaf.ec74; incoming xrefs ev 543d.6a0e.6729.9cb5.4e96.81f8.0134.9ba6.33ca.d26f.f0c7.e141.fee5.2dc4.7295.e975 (`0x5ac6af`); file-offset mapping ev dfa7.e6a0.4f89.f83e.daba.3360.c14a.edd6.9b8f.3705.1de5.8ff5.25b7.4e06.b7ce.9a93.

### `0x5a5600` — ask for exit confirmation with the download-cancel arm

- **VA / file offset**: `0x5a5600` / `0x1a5600`.
- **Behavior**: Requires the shared UI flag, an idle UI event queue and a scene id that is neither 7 nor 10. When the download flag returned by `0x4f7a10` is 1 it shows the two-line caption at `0x647278` (cancel download and exit) and then calls `0x5a8b20`; otherwise it shows the plain quit caption at `0x647258`.
- **Reached from**: `0x5a5190`. Direct caller `0x5a5190` for the system close command. Caption bytes: ev 5173.9405.119f.4b37.1900.72fb.ae50.4b87.bd04.1411.cd54.4d82.88f5.174b.137b.42c7 and ev 33b2.6dd7.1572.cf94.1b9d.5453.51cb.643b.ae05.104b.d7d2.2b3a.d1b4.2b21.d066.9743.
- **Direct callees**: `0x478a00`, `0x478a60`, `0x478c90`, `0x4f7a10`, `0x5a8b20`, `0x5f1aa2`, `0x5f2c4d`.
- **Player-visible reach**: the exit dialog text depends on an active download.
- **Our client**: Partial. Quit-confirm caption and OK/CANCEL state exist at `crates/o2jam-shell/src/screens/side.rs:7499` and `crates/o2jam-shell/src/screens/game/mod.rs:6691`/`:6713`; the exe download-cancel arm from `0x4f7a10` has no counterpart.
- **Evidence**: analysis ev 8c1e.0368.78b1.1637.ffeb.e708.b72a.e602.37af.3974.9f00.e329.e935.131d.4647.ba67; dossier ev 97d5.6445.4be7.1819.a314.a71b.9fc8.027f.b17e.15a6.5808.82a7.d223.0e13.8c49.13a8; body batch ev 7d20.1f39.6305.28bc.a63c.ee32.40e1.f781.1c95.9c33.120b.ab28.787f.f9e8.7ec7.25b0; caller facet ev f547.73ad.8483.0223.6f37.5111.43b3.2a70.d39b.d9ce.66d8.f40e.bc06.2af2.dcfa.7c43; callee facet ev 696d.bf29.e6b0.102e.19b2.8053.2b40.6e88.fd67.89b0.cdc4.8d70.698d.0365.2dea.ead3; incoming xrefs ev c9ca.21e7.f68e.a503.3140.f410.5886.8600.7c2a.582d.3cf3.945e.39fb.6158.b4db.8161 (`0x5a5305`); file-offset mapping ev 1e4c.7de1.3b58.ec6d.fb25.359a.0434.c0e8.db9d.670f.cde4.20b4.796c.8b79.b563.995b.

### `0x5a8840` — store one shared word and refresh the caption state (support)

- **VA / file offset**: `0x5a8840` / `0x1a8840`.
- **Behavior**: Writes the first argument into the object's `+0x900` word, resizes the buffer at `+0x928` when the second argument is nonzero, and calls the caption-refresh helper `0x478c50(0x10)` when both initialization flags at `+0x10` and `+0x914` are set.
- **Reached from**: `0x40a5e0`, `0x410f40`, `0x433c50`, `0x434310`, `0x434960`, `0x4351a0`, `0x438040`, `0x439ac0`, `0x43af30`, `0x43d3e0`, `0x456ab0`, `0x457280`, `0x4592d0`, `0x459c10`, `0x45a1d0`, `0x45b140`, `0x45b7a0`, `0x45c520`, `0x45d850`, `0x45ddb0`, `0x45e800`, `0x45ef20`, `0x45fa70`, `0x46e1b0`, `0x46eb40`, `0x46fe50`, `0x472540`, `0x472c40`, `0x474110`, `0x48dd10`, `0x48e990`, `0x4984d0`, `0x4a3730`, `0x4b6100`, `0x4c2390`, `0x4c6d70`, `0x4f3590`, `0x4f4740`, `0x4f5730`, `0x5a5710`, `0x5a70a0`. Forty-two direct callers, mostly shared control and dialog wrappers across scenes.
- **Direct callees**: `0x478c50`, `0x5f27f0`.
- **Player-visible reach**: Support; the body only stages state and requests a refresh.
- **Our client**: No counterpart by VA; shared state lives in `ScreenGame` (`crates/o2jam-shell/src/screens/game/mod.rs:936`).
- **Evidence**: analysis ev 04c8.1ace.0d54.7305.70c3.3542.d8db.e784.036f.a1a9.5bc6.c7cb.8991.4cda.3ec0.bc6b; dossier ev 6fab.adc9.b493.3e6b.bfc3.933f.bb99.0942.6382.03fa.d82a.e119.0cb5.7082.db0d.0e64; body batch ev 790c.49ac.84ff.f685.adea.a4a1.1d59.1e31.6e83.1860.69fa.2c5b.9ead.21bf.cc80.f20a; caller facet ev 4c19.bab8.0ca3.c693.98bf.e50b.9c8d.342b.8a23.8d36.f0a4.7f66.5430.1d15.5b7c.0edb; callee facet ev 45ea.5b34.9909.5e7f.587e.b497.6ceb.7e37.3692.63ea.93b6.14c1.2544.a2ee.311e.303b; incoming xrefs ev 2878.4f94.3796.dc0c.ce09.f4f3.5eab.723d.6a63.c1f9.699b.c196.bf13.77e3.f604.54ba (`0x40a75a`, `0x40a783`, `0x40a7ac`, `0x41101c`, `0x411045`, `0x41106e`, `0x433c72`, `0x434332`, `0x434982`, `0x4351c2`, `0x43806a`, `0x439ae2`, `0x43af52`, `0x43d402`, `0x456ad2`, `0x4572a2`, `0x4592f2`, `0x459c32`, `0x45a1f2`, `0x45b162`, `0x45b7c2`, `0x45c542`, `0x45d872`, `0x45ddd2`, `0x45e822`, `0x45ef42`, `0x45fa92`, `0x46e1d2`, `0x46eb62`, `0x46fe72`, `0x472562`, `0x472c62`, `0x474132`, `0x48dd59`, `0x48e9e2`, `0x498536`, `0x4a378f`, `0x4b6149`, `0x4c23f8`, `0x4c6e20`, `0x4f35df`, `0x4f4792`, `0x4f5785`, `0x5a5fa0`, `0x5a7214`); file-offset mapping ev 6140.501f.b1b6.4339.ab47.bfbd.fea1.a175.2da1.a929.54dd.6dd0.9492.9baf.167c.0406.

### `0x5a8ab0` — show a popup or close the window when a UI flag is set

- **VA / file offset**: `0x5a8ab0` / `0x1a8ab0`.
- **Behavior**: Returns the state of the shared UI flag at `+0x914`. When set, it posts a popup with the supplied text through `0x478c90(1, 0xffe, text, 0)`. When clear, it sends `WM_CLOSE` (`0x10`) to the main window handle from `0x431e80` and waits one second before returning.
- **Reached from**: `0x5a35c0`, `0x5a36d0`. Both callers sit in the same window-support cluster; their call sites are the recorded xrefs.
- **Direct callees**: `EXTERNAL:0x8f`, `EXTERNAL:0x136`, `0x431e80`, `0x478c90`.
- **Player-visible reach**: this helper chooses between an on-screen message and closing the client.
- **Our client**: Partial. Quit-confirm state at `crates/o2jam-shell/src/screens/game/mod.rs:6691`/`:6713`; no VA-equivalent popup-or-close decision or one-second delay.
- **Evidence**: analysis ev a02f.086e.979e.d751.d914.8dcb.b216.bac1.2b61.1709.b95f.14ad.1bf1.7c09.af82.2a15; dossier ev 072b.f285.430f.e4cf.7d4a.f800.6558.b919.74e1.967a.84f0.1216.d4d7.598a.3a45.8284; body batch ev 521f.c935.461e.18ec.4127.93ee.58a3.0cf1.c9c0.4456.8e01.5abd.1afc.e415.7a23.d51d; caller facet ev 2a97.c180.687e.4b3a.23bc.5381.3f3b.ad3f.489d.4ff4.3abf.8bc0.f523.b793.bafe.66f8; callee facet ev d2ce.e484.59d2.a4a4.d164.c3b7.32e7.b954.e3b6.fc2d.8309.f316.9154.f039.1bd2.9f08; incoming xrefs ev f40c.9db7.c286.9ef3.4b4b.7f71.572e.a05f.e3d5.ba4e.1caf.2928.2081.45b4.feb7.b4b8 (`0x5a3634`, `0x5a3757`); file-offset mapping ev db76.e70d.2c0b.2a96.5db0.670a.1e62.17b2.7600.7442.66a2.cac3.5bc6.17c2.4fe6.f32b.
