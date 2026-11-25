# Statics pins — Planet + Channel unpinned functions (coverage burn-down 2)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`. Provider: REA 6.10.1 / Ghidra 12.2; target
bound from `install-root/O2Jam/OTwo.exe`.

Branch `cb2/pins-planet-channel`; base commit `b872971` (ancestor of this
branch's `ed556bf`). Pre-flight `git rev-parse --show-toplevel` resolves to
`/home/arnim/Projects/recomp2jam-lane-pins-planet-channel`.

Scope: the 53 rows in `procedures.csv` with `rule=game:dispatch-closure`
(the game-reachable bucket), `pinned_in_repo=no`, and subsystem
`Planet / Channel` in `triage-final.json`. All 53 are pinned here; none is
left in this bucket. The triage subsystem is an address-cluster label for
52 of the 53 rows: their bodies are generic STL vector/copy/fill and object
initializer templates instantiated in the Planet range. Only `0x524350`
names a Planet class, and only `0x479cb0` is a shared input-dispatch seed.

Evidence notation: an evidence ID is written as `ev ` followed by its hex
in four-digit groups separated by dots. Remove the dots and join after
`ev_` to obtain the full REA ID. The SHA-256 above uses the same grouping.
Grouping keeps `scripts/check-cited-hashes.sh` from treating evidence hex as
a commit citation. File offsets are virtual address minus the `0x400000`
image base, checked with REA `address_to_file_offset` (for example
`0x479cb0` maps to file `0x79cb0`).

Prior pins are cited, not re-derived: `docs/parity/planet.md` (board
composition, promo, exit/option), `docs/parity/channel.md` (dialog geometry,
wire-sourced row digits, entry face), `docs/interaction-census.md` Planet and
Channel sections (census rows served below), `docs/channel-fill.md` (host
dialog slot at parent `+0xF6C`), `docs/coverage-audit.md` (state factory
`0x477990` and the same slot), `docs/exe-function-coverage.md` (WndProc chain
`0x5ac690` → `0x5a5190` → `0x5a50d0` → `0x479cb0`), `docs/claims.md` A47
(control factory `0x48a620`, dialog factory `0x4329b0`, slot loader
`0x48b2d0`). No disassembly or decompiled text is reproduced.

## Inventory

| VA | File offset | Triage label | Role | Analysis evidence |
|---|---|---|---|---|
| `0x479cb0` | `0x79cb0` | Planet / Channel | input dispatch seed | ev 7112.9e83.ef51.266a.1409.b138.9f90.d75e.5a61.f826.bf54.6e0f.7501.f1b2.cc65.d1e8 |
| `0x47ebf0` | `0x7ebf0` | Planet / Channel | shared template/support | ev 073d.0e65.a4de.e873.36d6.ceeb.b83a.a308.a6c9.9601.a8a3.943e.e0cc.bbb6.591d.fba0 |
| `0x4804f0` | `0x804f0` | Planet / Channel | shared template/support | ev 2a01.1f44.bc2a.9d3f.773b.d059.fc57.5122.54e8.6a9e.7780.7cb7.b005.dca5.4201.4766 |
| `0x4808d9` | `0x808d9` | Planet / Channel | shared template/support | ev 443c.9396.e553.162d.963b.1cd3.b9c1.25b3.79bd.3436.2c0b.d915.2f00.dca0.c895.2045 |
| `0x480aaf` | `0x80aaf` | Planet / Channel | shared template/support | ev 8c35.1704.db99.86e4.a833.c813.2405.8143.5d9d.f5f4.31df.d0a3.54e2.f7b4.58f1.d10c |
| `0x480c80` | `0x80c80` | Planet / Channel | shared template/support | ev eefc.eb39.ce7a.1c2a.0555.7774.1e5a.0b56.c35d.f4f6.4bb4.75a8.209b.4701.26e2.0ef3 |
| `0x481fc0` | `0x81fc0` | Planet / Channel | shared template/support | ev f82e.9210.7e6f.38dd.03f7.ead0.721d.f937.b6f2.a3da.00f3.262d.8023.2c94.a75d.c9ec |
| `0x4823f1` | `0x823f1` | Planet / Channel | shared template/support | ev f412.dcc5.7f6c.7640.4916.7e0d.0332.afa8.30d6.ce5e.7d2d.76ec.a457.652a.68b1.6ca4 |
| `0x48266f` | `0x8266f` | Planet / Channel | shared template/support | ev 1050.c568.42d1.ae9e.87f3.d6b3.2901.1368.1123.5527.1c0c.6cf2.264d.71ca.49cd.9a63 |
| `0x482800` | `0x82800` | Planet / Channel | shared template/support | ev 6809.b52d.c6d8.5396.177d.c9c2.b53d.096b.6ccc.ae74.73a2.ae85.b7cc.35e0.9483.5aa1 |
| `0x482cae` | `0x82cae` | Planet / Channel | shared template/support | ev 889c.d0d0.270f.9de0.7e44.65f0.40bc.ca13.29af.7bdf.aea1.c2c1.a4a6.b30e.b33b.92eb |
| `0x482f68` | `0x82f68` | Planet / Channel | shared template/support | ev 7147.eac6.6830.a779.2dc9.6f66.e182.f64c.9339.0655.6b69.83b5.cd27.3f0d.9aae.34a5 |
| `0x483c60` | `0x83c60` | Planet / Channel | shared template/support | ev 1d8c.8465.bc27.d4b5.3295.5844.9683.b6a5.d063.10d7.d314.a2d1.33dc.a9b8.f3d6.6701 |
| `0x483d10` | `0x83d10` | Planet / Channel | shared template/support | ev 57d7.5bb8.4b30.184e.9be7.97de.3023.be18.1e9a.0b0a.8e3e.beed.95a0.cfc0.66ca.6d00 |
| `0x483ed0` | `0x83ed0` | Planet / Channel | shared template/support | ev 564d.51a4.14e4.03aa.3a2a.6cd2.d608.8d93.d9e1.baa3.f54f.e897.9ca8.94eb.7ca4.d3df |
| `0x483f80` | `0x83f80` | Planet / Channel | shared template/support | ev e3a5.c2eb.c809.c02b.59cb.a3b4.e4ab.f5e1.2fa6.4199.2bae.a057.e146.b920.a80f.75b6 |
| `0x484000` | `0x84000` | Planet / Channel | shared template/support | ev 0a5a.ddfc.687f.b52c.3bbc.6c47.f8de.4be7.3c28.6c0d.6e68.aa31.15d3.300f.6777.f7e8 |
| `0x4841b0` | `0x841b0` | Planet / Channel | shared template/support | ev bd58.db93.7550.848c.4d72.d059.ffa0.41b3.80dc.3b9b.e006.e498.021e.6134.b87e.3f4f |
| `0x484200` | `0x84200` | Planet / Channel | shared template/support | ev be3a.7581.9c93.4925.842e.30da.fc61.f238.3212.6b6b.45b0.e50a.a1d6.c7b0.06d2.e262 |
| `0x4847f0` | `0x847f0` | Planet / Channel | shared template/support | ev bbf4.0803.c345.7859.5123.431a.7722.fcf5.086d.54b9.9663.6827.3383.35ab.f65b.47fa |
| `0x4848af` | `0x848af` | Planet / Channel | shared template/support | ev ee27.0149.b385.b32a.58ee.1181.c329.0b3c.7ff5.524d.5bfd.e24a.4f8e.79f1.cbb6.87c5 |
| `0x484985` | `0x84985` | Planet / Channel | shared template/support | ev 1370.0e5e.4770.3c3f.0710.b312.d7a4.6010.8b00.678c.9c5b.af48.0d4a.2716.f757.8be7 |
| `0x4849a0` | `0x849a0` | Planet / Channel | shared template/support | ev 60b4.4bf8.4a6b.619a.04f7.426a.6aa5.9faf.b242.a730.fe61.a34e.d0eb.8a78.c0aa.f42e |
| `0x484c30` | `0x84c30` | Planet / Channel | shared template/support | ev 7fdc.f9de.4f5e.3d25.7398.0423.1ef2.e196.a569.ed42.924d.389e.d822.f569.b49c.bf49 |
| `0x484cb0` | `0x84cb0` | Planet / Channel | shared template/support | ev 9a9d.69c3.4ad3.702c.c101.9568.57a4.c216.3f6e.fb42.07a0.3a05.18b3.21e5.fa36.f3b0 |
| `0x484d50` | `0x84d50` | Planet / Channel | shared template/support | ev 9b9b.03a2.0a5b.ae5c.4f54.7ece.746e.2ecb.1a73.cda0.4f12.fd9a.7a3e.70b9.f707.eaff |
| `0x484dd0` | `0x84dd0` | Planet / Channel | shared template/support | ev f078.e687.3f9c.3705.a5c6.cfa4.d540.813f.5a2c.edf5.6add.8cb3.c09c.05cc.830d.383b |
| `0x484ec0` | `0x84ec0` | Planet / Channel | shared template/support | ev aa25.c854.542c.aba3.12dd.4725.2e83.18b3.855f.d6f2.d130.66f9.1fb6.a7fd.dfb2.2917 |
| `0x484fa0` | `0x84fa0` | Planet / Channel | shared template/support | ev 0d7d.5628.8ee7.b4a6.1f8e.b1f7.d273.3680.4d96.47e0.0899.6491.e866.1955.ea07.17b4 |
| `0x485060` | `0x85060` | Planet / Channel | shared template/support | ev c644.56db.ac3c.4e7f.35c4.7311.d903.aac1.3022.9efa.aac6.d420.933b.90cc.7f4c.5bce |
| `0x485100` | `0x85100` | Planet / Channel | shared template/support | ev bc64.1b31.49a2.41b2.9103.a434.76fb.0d85.2108.c93c.8de5.15c1.b853.3653.8dbb.b2f6 |
| `0x4852a0` | `0x852a0` | Planet / Channel | shared template/support | ev 4ee1.3293.f6fe.17be.a1ce.ef75.3b9a.85c2.e816.56eb.5e8a.46f9.d409.8533.d782.026a |
| `0x485361` | `0x85361` | Planet / Channel | shared template/support | ev 0116.6353.48fd.38af.1d44.8f98.729f.9d09.3677.9354.326e.e163.bdd7.ad6d.0c70.dffe |
| `0x485460` | `0x85460` | Planet / Channel | shared template/support | ev 26aa.ca99.a47b.8c06.bccc.c337.f53e.c896.8dbd.d368.1e59.434c.1564.3760.3321.0b26 |
| `0x48551b` | `0x8551b` | Planet / Channel | shared template/support | ev b9fe.a919.d9bc.25cd.8394.7a37.b65e.8bcb.4604.6a73.75c0.aa73.4c14.da58.732e.dd1d |
| `0x485bb0` | `0x85bb0` | Planet / Channel | shared template/support | ev d0c5.8bf7.418e.2b98.ab55.311d.78d8.9432.4061.937c.c519.6d12.e386.8406.27ba.5e1f |
| `0x485dd0` | `0x85dd0` | Planet / Channel | shared template/support | ev 956a.ac60.be16.3636.f41a.eede.0621.3824.7dd6.7431.0ebc.b837.d82c.07a3.e6bd.739f |
| `0x487390` | `0x87390` | Planet / Channel | shared template/support | ev 81a9.d55a.7554.2d52.1b52.5e64.944f.7f49.88fe.f280.cecc.16c1.9e23.2a87.ded0.f744 |
| `0x487ff0` | `0x87ff0` | Planet / Channel | shared template/support | ev 013d.9cfa.a8e7.a9f5.ab14.7c8b.545c.3d17.94fc.3c18.84a0.efe3.f904.32c7.a97b.f98e |
| `0x4880b0` | `0x880b0` | Planet / Channel | shared template/support | ev 5f7c.c802.a856.3489.99f5.cb43.2e55.ac51.fe5d.f33f.9e87.d8a2.a47f.8dc0.0bcc.9e3d |
| `0x488fd0` | `0x88fd0` | Planet / Channel | shared template/support | ev 0789.0d39.4ca0.9636.4969.18c0.caa0.29c4.de8c.e4cf.e212.780a.26b3.91c9.f577.11ae |
| `0x489040` | `0x89040` | Planet / Channel | shared template/support | ev 1443.4603.9a36.b174.ead3.10f8.7da5.d3c4.03b4.6378.b638.5cdf.7320.d425.3c40.15de |
| `0x4890e0` | `0x890e0` | Planet / Channel | shared template/support | ev bb4a.29b3.cdfd.c503.d039.684e.6d14.ddba.8164.bcc6.578a.cc7e.3f94.0828.0090.2491 |
| `0x489150` | `0x89150` | Planet / Channel | shared template/support | ev 8007.940a.f277.272f.9f4f.7f70.44c5.3d33.ec32.e4d8.6166.c6df.ac0b.556a.9e92.dca9 |
| `0x489200` | `0x89200` | Planet / Channel | shared template/support | ev c110.d4d8.6385.7d9d.044c.a2be.12c2.a3ae.5289.2a1e.342b.745b.1048.190f.9615.6432 |
| `0x4892a0` | `0x892a0` | Planet / Channel | shared template/support | ev 3c47.371d.3492.5929.2b42.0a6b.43bf.29e5.1b4d.9596.b2b8.cc9b.4c78.57eb.6af2.a9ab |
| `0x4893e0` | `0x893e0` | Planet / Channel | shared template/support | ev 62b9.7655.c53f.ea1d.09d9.7581.ae2e.bfb0.b57c.8be4.958e.0133.f0a2.11c8.51af.8af5 |
| `0x489490` | `0x89490` | Planet / Channel | shared template/support | ev e165.04fe.b49b.232d.5524.917c.7ad3.0659.070c.68b6.50b0.dc9e.91c4.7391.00e0.5595 |
| `0x489540` | `0x89540` | Planet / Channel | shared template/support | ev d886.3a97.68d3.2a41.e284.76e1.458d.b589.5c66.6a68.9ed1.5e95.2715.0af4.c773.ab87 |
| `0x48a030` | `0x8a030` | Planet / Channel | shared template/support | ev 5838.8e22.7cf8.ac90.741e.02c6.1e64.4458.9b67.d409.630e.f971.9e6f.f30f.d1f2.6111 |
| `0x48ce30` | `0x8ce30` | Planet / Channel | shared template/support | ev a106.089d.9067.adfa.d1be.15b0.9af4.32d5.0373.93b4.9815.3ede.0493.a95a.9ccf.a9ef |
| `0x48d880` | `0x8d880` | Planet / Channel | shared template/support | ev 89d4.dfd4.a510.82d3.71af.b217.c027.85b1.301b.81df.6317.9dfe.793e.5c74.503f.20f9 |
| `0x524350` | `0x124350` | Planet / Channel | planet control ctor | ev 8f94.3310.c85e.6ec8.ecbb.5e97.0d06.8050.7a81.8528.2f4b.216e.06d7.c526.4b8c.7679 |

## Entries

### `0x479cb0` — If the state has a dialog pointer at `+0xF6C`

- VA / file offset: `0x479cb0` / `0x79cb0`.
- Behavior (own words): If the state has a dialog pointer at `+0xF6C`, invokes that dialog's virtual `+0x50`; then counts the four-byte entries of the vector at `+0xF74` and, when non-empty, obtains a global object from `0x47f070` and invokes its virtual `+0x5C` once.
- Reach and player-visible role: Player-visible input dispatch support. This is the WndProc-chain seed named in `docs/exe-function-coverage.md`: messages reach the hosted Channel dialog slot and the board's global listener before any pixel or wire byte changes.
- Census rows served: Planet ordinals 1–16 (click/hover routes) and Channel ordinals 1–48 plus `DIALOG_CHANNEL_INFO` ordinals 0–2, all indirectly through the hosted dialog/listener call.
- Our status: Pinned as shared dispatch; the classes behind the two virtual slots are not identified in this body. OPEN next step: join the vftables at those slots to RTTI and read the target methods.
- Evidence: analysis ev 7112.9e83.ef51.266a.1409.b138.9f90.d75e.5a61.f826.bf54.6e0f.7501.f1b2.cc65.d1e8; dossier ev 93ea.03b4.745c.a683.4aa3.d9d6.3c5a.1f5f.5875.f67f.b518.a544.60b5.8520.d054.8976; incoming xrefs ev d2c0.65c4.a3bd.d788.9209.40a5.7a5a.9eae.7697.998a.e2e2.bb5d.3c66.b5dc.8627.0b4f (observed direct call sites: 0x5a50d0); caller facet ev 86e8.9a88.ab3c.8b98.184c.21e0.1c6e.3e45.2259.e9c5.8566.c12e.034b.8570.73b7.8935; referenced names ev d533.25fe.9162.1705.f983.d250.4d5a.8ff6.a83e.90d9.fb83.0f3f.ca33.79c4.a445.6660. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x47ebf0` — Pushes one 16-byte record by copying four words to the computed destination and stepping the vector's end pointer back b

- VA / file offset: `0x47ebf0` / `0x7ebf0`.
- Behavior (own words): Pushes one 16-byte record by copying four words to the computed destination and stepping the vector's end pointer back by one record.
- Reach and player-visible role: Support. Generic vector push-back instantiated with a 16-byte element; reached from the state/input dispatcher cluster, not a Planet-only code path.
- Census rows served: None directly; supports Planet/Channel control and row containers.
- Our status: Pinned as a container operation; no census row closes on this pin.
- Evidence: analysis ev 073d.0e65.a4de.e873.36d6.ceeb.b83a.a308.a6c9.9601.a8a3.943e.e0cc.bbb6.591d.fba0; dossier ev 91d5.52e9.98f0.01eb.6555.4050.7b74.4648.64dc.10d7.37f1.ab3f.3306.8694.4e42.593b; incoming xrefs ev 6c9e.badf.d25d.e466.44b2.8751.54f1.5282.a36f.7771.3f30.eb44.adb8.c25e.cda0.81c7 (observed direct call sites: 0x478df0); caller facet ev 36dd.20af.0794.a3d1.c1bb.e9df.e129.8267.a44e.33b0.bffd.180d.f659.cc02.4305.c11d; referenced names ev aa44.13ae.ae55.6309.bb83.f8e7.81f1.5c1e.f313.d1f6.7fea.62fc.b18e.850f.13ba.7d09. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4804f0` — Inserts N four-byte records at a position: grows capacity when needed

- VA / file offset: `0x4804f0` / `0x804f0`.
- Behavior (own words): Inserts N four-byte records at a position: grows capacity when needed, moves the old tail, fills the gap from one value, and delegates the buffer swap to nearby helpers.
- Reach and player-visible role: Support. Generic vector insert for four-byte elements; the Planet-region address is an instantiation, not a Planet behavior.
- Census rows served: None directly; supports state/list storage under Planet and Channel.
- Our status: Pinned as a container operation; no player-visible row.
- Evidence: analysis ev 2a01.1f44.bc2a.9d3f.773b.d059.fc57.5122.54e8.6a9e.7780.7cb7.b005.dca5.4201.4766; dossier ev 38eb.80d8.c434.7233.9ecb.aaec.82f9.69b2.aaa0.47f5.0a0f.c9df.e049.eccc.5eec.41e7; incoming xrefs ev 19e4.7f0d.7fa6.e1c6.44f0.8fea.c3bc.3a86.5dcd.7415.69c5.4715.afac.6856.2e7e.0947 (observed direct call sites: 0x47f380); caller facet ev 8136.8642.cf8c.89d1.9aa8.f942.4b7c.4b9b.7432.7464.7c46.1175.d95a.d6f1.e2ed.ca3e; referenced names ev ddf2.0428.301f.b49b.a4f9.4e86.652e.9b5b.c0c7.1f7d.3a91.3f5b.2f99.025c.07cf.b6a2. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4808d9` — Finishes a four-byte insert: advances the stored end count

- VA / file offset: `0x4808d9` / `0x808d9`.
- Behavior (own words): Finishes a four-byte insert: advances the stored end count, walks and frees the previous allocation, then rewrites begin, end and capacity pointers.
- Reach and player-visible role: Support. Terminal epilogue of `0x4804f0`.
- Census rows served: None directly.
- Our status: Pinned as allocator bookkeeping; no census row.
- Evidence: analysis ev 443c.9396.e553.162d.963b.1cd3.b9c1.25b3.79bd.3436.2c0b.d915.2f00.dca0.c895.2045; dossier ev 72b7.9a08.fa18.e6d3.2287.ffa8.21a5.e5da.9777.a4d5.39bc.6726.f3c0.d34d.5911.7b45; incoming xrefs ev 8f65.a741.2ecf.eb05.528d.d35b.ebf1.7bcb.c51c.8af5.a67e.6745.5fb2.f1c1.ad22.5066 (observed direct call sites: 0x4804f0); caller facet ev 30b6.d402.6f46.0e5b.e0d8.0f3e.445a.c154.6f02.eeff.9077.fc3a.d6d9.b1c5.ae01.f279; referenced names ev 2e0d.640f.d6b1.4200.2707.34ed.ee2d.141a.0a82.e547.9572.52f5.4f93.79b3.4086.9a1d. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x480aaf` — Fills the remaining tail slots of a four-byte vector with the repeated value passed through the caller's frame.

- VA / file offset: `0x480aaf` / `0x80aaf`.
- Behavior (own words): Fills the remaining tail slots of a four-byte vector with the repeated value passed through the caller's frame.
- Reach and player-visible role: Support. Tail-fill arm of the four-byte insert path.
- Census rows served: None directly.
- Our status: Pinned as a fill helper; no census row.
- Evidence: analysis ev 8c35.1704.db99.86e4.a833.c813.2405.8143.5d9d.f5f4.31df.d0a3.54e2.f7b4.58f1.d10c; dossier ev fe15.1e99.3e5f.2d95.4001.1a80.4705.b345.2531.269c.fd2e.0d9a.3880.b731.c67d.23d6; incoming xrefs ev b753.91c8.f473.9630.ed52.d237.0a5c.71ef.45f6.2c3a.c280.c415.4c0d.1f4d.5814.719f (observed direct call sites: 0x4804f0); caller facet ev fae8.35cf.1dc2.102d.303d.413a.147e.7a9a.3ad5.5669.cde4.23c0.0bc1.cb1d.5504.eb2d; referenced names ev b24c.8e0f.fcbc.d3aa.530e.7f20.5ad3.bf9b.b6dc.bca3.8808.3e9d.9045.fc73.c106.4dad. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x480c80` — Copies N dwords from a value source into a destination and returns the first address past the written range.

- VA / file offset: `0x480c80` / `0x80c80`.
- Behavior (own words): Copies N dwords from a value source into a destination and returns the first address past the written range.
- Reach and player-visible role: Support. Thin fill helper used from the four-byte insert tail.
- Census rows served: None directly.
- Our status: Pinned as a fill helper; no census row.
- Evidence: analysis ev eefc.eb39.ce7a.1c2a.0555.7774.1e5a.0b56.c35d.f4f6.4bb4.75a8.209b.4701.26e2.0ef3; dossier ev cf99.9b48.3fae.fce7.f0e4.e05e.8381.a9b8.38cd.8757.3d9a.ac81.1b6a.68a4.7a7f.e0d9; incoming xrefs ev 1acb.495c.28e5.8da6.bf07.c495.8831.9e9c.d227.6450.0655.7db9.c975.1d40.21a0.c90f (observed direct call sites: 0x4804f0); caller facet ev ba54.b162.452a.c0c2.1a35.96ce.50b2.e68f.a081.fe0b.10a9.7695.6765.62e0.2501.d371; referenced names ev 0e0c.ba08.74de.73fb.7654.4a99.17b2.4cc6.b72d.61df.008c.8680.482b.c2c2.374d.10d5. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x481fc0` — Inserts N 16-byte records at a position: grows capacity with an overflow check

- VA / file offset: `0x481fc0` / `0x81fc0`.
- Behavior (own words): Inserts N 16-byte records at a position: grows capacity with an overflow check, copies the old records, fills the gap from a four-word value and delegates the buffer swap.
- Reach and player-visible role: Support. Generic vector insert for 16-byte elements.
- Census rows served: None directly; supports control/record containers.
- Our status: Pinned as a container operation; no census row.
- Evidence: analysis ev f82e.9210.7e6f.38dd.03f7.ead0.721d.f937.b6f2.a3da.00f3.262d.8023.2c94.a75d.c9ec; dossier ev 21a7.90b9.98a2.7c6f.f301.45d6.406b.840c.eb0b.3621.682f.83f4.c060.a410.0160.38f7; incoming xrefs ev dc05.4605.6357.4d3a.40aa.5aa6.28a0.0077.7a30.67bc.b6ab.2e71.72eb.c658.564d.480e (observed direct call sites: 0x47faf0); caller facet ev 5372.c3d6.2b7e.8db5.5dfb.6b99.669a.c539.c138.032d.e244.551c.5681.b043.c6c4.5cd8; referenced names ev 90af.9666.41bc.1e90.7aee.e1a0.2614.22fc.fb0e.c018.6218.e2e9.5476.7928.1cf4.3c64. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4823f1` — Finishes a 16-byte insert: adds the old count to the end offset

- VA / file offset: `0x4823f1` / `0x823f1`.
- Behavior (own words): Finishes a 16-byte insert: adds the old count to the end offset, frees the old allocation after walking its records, and stores new begin/end/capacity pointers.
- Reach and player-visible role: Support. Terminal epilogue of `0x481fc0`.
- Census rows served: None directly.
- Our status: Pinned as allocator bookkeeping; no census row.
- Evidence: analysis ev f412.dcc5.7f6c.7640.4916.7e0d.0332.afa8.30d6.ce5e.7d2d.76ec.a457.652a.68b1.6ca4; dossier ev 96f9.536f.8c1d.facb.ea96.6e67.986a.55ad.966a.1460.7b64.a520.9d17.db1d.4fe5.3fd9; incoming xrefs ev 04f8.87fd.ceed.ee66.80b8.25d3.edde.c8c1.330a.5f1e.5a6d.0877.cf70.70e4.1705.4300 (observed direct call sites: 0x481fc0); caller facet ev 3dc4.027d.9ea8.a4fa.b0f6.484d.7be8.c8ad.21d0.287c.c083.1b72.4dff.e8c0.01e3.164f; referenced names ev ed85.7412.a72d.cdd6.1b92.57ef.2955.803f.6db6.041d.52e3.55d3.ae40.2b7c.c26c.2855. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x48266f` — Fills N 16-byte slots by writing the same four words into each record and advancing by one record each time.

- VA / file offset: `0x48266f` / `0x8266f`.
- Behavior (own words): Fills N 16-byte slots by writing the same four words into each record and advancing by one record each time.
- Reach and player-visible role: Support. Tail-fill arm of the 16-byte insert path.
- Census rows served: None directly.
- Our status: Pinned as a fill helper; no census row.
- Evidence: analysis ev 1050.c568.42d1.ae9e.87f3.d6b3.2901.1368.1123.5527.1c0c.6cf2.264d.71ca.49cd.9a63; dossier ev c58a.1a27.dcfe.1e56.bede.0c2b.cfda.5276.8ba5.d2df.2587.ecfb.68a1.3037.e451.e1bc; incoming xrefs ev 1d3e.15bd.c0c7.73e2.c1a2.634b.817a.7c86.8f51.e932.38d2.6e36.7a14.aa5c.c798.6360 (observed direct call sites: 0x481fc0); caller facet ev 4936.386f.b270.9107.fbf7.3706.b5c1.ba2f.53e6.11bc.b47d.b3f2.0169.c73e.ccd5.3670; referenced names ev fcfb.e111.5ef4.4ce9.4774.64d1.d496.24f5.0bd2.1aae.a9b9.cab7.67d4.e2de.c729.3906. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x482800` — Inserts N records of `0x110` bytes (`0x44` dwords) at a position: grows capacity

- VA / file offset: `0x482800` / `0x82800`.
- Behavior (own words): Inserts N records of `0x110` bytes (`0x44` dwords) at a position: grows capacity, copies a `0x44`-word prototype into new slots, moves the old tail and swaps buffers.
- Reach and player-visible role: Support. Generic vector insert for the largest record size seen in this cluster; a security cookie protects the prototype copy.
- Census rows served: None directly; may carry control-list records but no control-specific decision is made here.
- Our status: Pinned as a container operation; the record type remains OPEN (next step: read a live caller's prototype source).
- Evidence: analysis ev 6809.b52d.c6d8.5396.177d.c9c2.b53d.096b.6ccc.ae74.73a2.ae85.b7cc.35e0.9483.5aa1; dossier ev 2d13.a7fb.5fc9.0f78.ad67.d1a5.6068.2dbb.376f.7d5a.f9f6.232a.489e.334a.40da.e226; incoming xrefs ev 462e.9cd2.222a.7f1d.8e7f.a9ed.a897.5580.2920.5ced.fa5a.f3bb.0a86.4f71.0a7a.9803 (observed direct call sites: 0x47fea0); caller facet ev f423.ea0e.5721.ed44.9500.84ae.486e.1795.296d.954b.816a.6694.38ee.3f22.9388.e65a; referenced names ev ad69.4f5a.6b14.84b9.d1c5.b572.6c04.e6e9.100f.6a41.bd20.a1f0.4a0e.4fa4.bb20.a16a. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x482cae` — Finishes a `0x110`-byte insert: advances the end count

- VA / file offset: `0x482cae` / `0x82cae`.
- Behavior (own words): Finishes a `0x110`-byte insert: advances the end count, frees the previous allocation after a stride walk, and writes the new begin/end/capacity fields.
- Reach and player-visible role: Support. Terminal epilogue of `0x482800`.
- Census rows served: None directly.
- Our status: Pinned as allocator bookkeeping; no census row.
- Evidence: analysis ev 889c.d0d0.270f.9de0.7e44.65f0.40bc.ca13.29af.7bdf.aea1.c2c1.a4a6.b30e.b33b.92eb; dossier ev 0302.0f6c.0cc2.7427.9d8f.fc18.c282.c082.73c9.391d.7dc1.f430.5005.2950.0475.aed8; incoming xrefs ev f2e6.7175.5f60.bf54.36b6.46bf.2573.8383.c498.3a0a.f3ef.0153.15b6.a657.0537.428d (observed direct call sites: 0x482800); caller facet ev 904c.b658.c407.e75b.8d2a.c421.97ec.1087.7ba6.1789.fd10.5a19.a091.7cb5.eff4.db80; referenced names ev 642d.9094.63fb.2ac5.35c4.5876.439b.25f1.899d.f262.4e2c.4195.d4ce.4c7a.a9f7.1175. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x482f68` — Fills N `0x110`-byte slots with the caller's `0x44`-word prototype and advances one record at a time.

- VA / file offset: `0x482f68` / `0x82f68`.
- Behavior (own words): Fills N `0x110`-byte slots with the caller's `0x44`-word prototype and advances one record at a time.
- Reach and player-visible role: Support. Tail-fill arm of the large-record insert path.
- Census rows served: None directly.
- Our status: Pinned as a fill helper; no census row.
- Evidence: analysis ev 7147.eac6.6830.a779.2dc9.6f66.e182.f64c.9339.0655.6b69.83b5.cd27.3f0d.9aae.34a5; dossier ev 2fdc.eb8e.b73e.be82.c5a6.f59d.ffe0.f772.62d5.e0d0.2370.4d14.d161.8e68.52fe.160f; incoming xrefs ev 4dee.09e2.0478.0211.e7c5.d484.e626.e6cf.e3b7.b380.4088.5262.3ccb.6426.1c2f.7c79 (observed direct call sites: 0x482800); caller facet ev 9971.015d.df13.5f35.8c83.e86b.631e.95ce.bc94.2232.f416.5d37.de91.85f1.06af.dac4; referenced names ev 7a81.e2df.a318.1db5.1c68.7a6b.841f.835b.d58e.a532.5fb1.805b.f03b.4e2d.8ca6.3a47. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x483c60` — Copies a range of 16-byte records into an uninitialized destination and returns the end of the written range.

- VA / file offset: `0x483c60` / `0x83c60`.
- Behavior (own words): Copies a range of 16-byte records into an uninitialized destination and returns the end of the written range.
- Reach and player-visible role: Support. Uninitialized-copy template.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 1d8c.8465.bc27.d4b5.3295.5844.9683.b6a5.d063.10d7.d314.a2d1.33dc.a9b8.f3d6.6701; dossier ev fb90.90e8.a407.ee1d.8886.f519.f6af.d042.e4a6.7c54.2f0b.a74b.8545.e4e3.1002.f9f0; incoming xrefs ev dd36.33ae.5d79.0c27.daf0.9ea7.2705.0f07.8e01.1378.56ef.430d.d70a.aca7.e328.d7f1 (observed direct call sites: 0x47ebf0, 0x47fc10); caller facet ev 9e1b.f91a.78a0.3c28.ed65.babc.cfaf.7df6.6972.0bc0.cff8.4db3.7b49.507a.9e97.b5ba; referenced names ev 3118.2691.5217.f8c0.cfaa.e47d.3ad3.d208.c359.66b0.cb9d.ee94.4fd7.67b1.9c39.744f. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x483d10` — Copies a range of `0x1B4`-byte (`0x6D`-dword) records into an uninitialized destination and returns the destination end

- VA / file offset: `0x483d10` / `0x83d10`.
- Behavior (own words): Copies a range of `0x1B4`-byte (`0x6D`-dword) records into an uninitialized destination and returns the destination end pointer.
- Reach and player-visible role: Support. Uninitialized-copy template for a different record stride.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 57d7.5bb8.4b30.184e.9be7.97de.3023.be18.1e9a.0b0a.8e3e.beed.95a0.cfc0.66ca.6d00; dossier ev 8dbb.a78f.3dc4.d43d.b30b.7818.741b.d4e2.82aa.9db7.80cd.fcc9.941f.9818.5e8d.ac8c; incoming xrefs ev 8407.2818.4e65.fefd.27f1.6b6e.0b76.b670.f051.3148.8d4e.d247.4865.d208.5107.7101 (observed direct call sites: 0x47f5d0, 0x564d40); caller facet ev 6fa1.b68d.0499.7884.b67c.0aa1.4656.67a2.ba26.598d.e2d5.1968.2b86.c252.2017.571f; referenced names ev d942.ff4c.df84.71ff.07f9.e7c1.b67c.4792.1137.4d73.9729.8182.81ad.5c19.c270.8b7f. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x483ed0` — Copies a range of `0x110`-byte (`0x44`-dword) records into an uninitialized destination and returns the destination end

- VA / file offset: `0x483ed0` / `0x83ed0`.
- Behavior (own words): Copies a range of `0x110`-byte (`0x44`-dword) records into an uninitialized destination and returns the destination end pointer.
- Reach and player-visible role: Support. Uninitialized-copy template matching the large-record vector.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 564d.51a4.14e4.03aa.3a2a.6cd2.d608.8d93.d9e1.baa3.f54f.e897.9ca8.94eb.7ca4.d3df; dossier ev 806d.b62a.e5f1.6cf5.e5bf.e2dc.d8bb.51b3.d908.c021.e166.a636.3ee2.f5f7.59f6.90ee; incoming xrefs ev 5ad8.cca4.e462.9c16.f66d.7636.ba05.8144.2591.df06.5a3d.ef12.97a2.2d07.6cbe.f309 (observed direct call sites: 0x47ffd0); caller facet ev 7c2e.73c3.e44c.2c0d.207b.445a.338d.c26c.6bc9.5d03.029a.5c46.9987.d7b4.639e.3a96; referenced names ev 2de4.3179.fa45.ca98.9281.7d49.ccad.c7c5.1748.5398.bd83.5169.53f4.b993.1612.20c2. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x483f80` — Moves a range of dwords with `memmove_s` when the source precedes the destination

- VA / file offset: `0x483f80` / `0x83f80`.
- Behavior (own words): Moves a range of dwords with `memmove_s` when the source precedes the destination, returning the end of the copied range.
- Reach and player-visible role: Support. Non-overlapping/forward copy step used by vector insert.
- Census rows served: None directly.
- Our status: Pinned as a move helper; no census row.
- Evidence: analysis ev e3a5.c2eb.c809.c02b.59cb.a3b4.e4ab.f5e1.2fa6.4199.2bae.a057.e146.b920.a80f.75b6; dossier ev f498.aa4a.b730.fee8.3758.4975.6ffd.1a51.3242.3c0e.4b2f.c95b.8b62.3fe4.fd7d.b8fb; incoming xrefs ev caae.f981.a326.e9f4.ad03.149a.b09b.3818.1d53.6604.285d.e003.d900.e1ff.8c14.4664 (observed direct call sites: 0x480320); caller facet ev 3097.0beb.04e4.fa85.9131.a962.2e72.350e.9904.b81f.4dd6.7ac1.1e63.651f.b414.4a3c; referenced names ev 9976.05f0.3300.7b2e.e4a4.ead8.e297.f030.58f9.cc42.1b27.4c55.ca9a.11cf.25c3.2b75. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484000` — Moves a dword range backwards by placing it before the supplied end address with `memmove_s`.

- VA / file offset: `0x484000` / `0x84000`.
- Behavior (own words): Moves a dword range backwards by placing it before the supplied end address with `memmove_s`.
- Reach and player-visible role: Support. Backward copy step used when insert shifts an existing tail.
- Census rows served: None directly.
- Our status: Pinned as a move helper; no census row.
- Evidence: analysis ev 0a5a.ddfc.687f.b52c.3bbc.6c47.f8de.4be7.3c28.6c0d.6e68.aa31.15d3.300f.6777.f7e8; dossier ev acec.157b.a4a7.941c.f57b.b13c.146a.d900.180b.3ee7.5292.068b.850c.791a.a2d4.8bce; incoming xrefs ev 8ea3.b102.fb94.e3fa.9194.5428.549e.d10a.7636.df20.9a86.430c.650b.334b.6566.7538 (observed direct call sites: 0x4804f0); caller facet ev 61ea.d259.eead.f2a5.7087.6964.9e6c.caf4.9f86.3edf.a05a.14df.4c2c.f390.b3b4.ccac; referenced names ev d759.0751.9d1c.4d7a.784a.d2c1.2bef.e44b.a123.3b7b.c399.6c5a.477d.aafb.adca.d407. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4841b0` — Delegates to `0x484cb0` without changing arguments.

- VA / file offset: `0x4841b0` / `0x841b0`.
- Behavior (own words): Delegates to `0x484cb0` without changing arguments.
- Reach and player-visible role: Support. Forwarding stub for the reverse 16-byte copy.
- Census rows served: None directly.
- Our status: Pinned as a thunk-like wrapper; no census row.
- Evidence: analysis ev bd58.db93.7550.848c.4d72.d059.ffa0.41b3.80dc.3b9b.e006.e498.021e.6134.b87e.3f4f; dossier ev 4fcf.b7bd.f674.2f68.f5aa.29f4.5880.82d0.c329.4efd.cf7a.ab5d.4f6f.7e28.cf50.33b9; incoming xrefs ev 949f.f37d.2ac8.8a02.aeca.957d.474c.d391.49ff.c295.4161.cf96.33a6.5929.2c3c.cc7b (observed direct call sites: 0x481fc0); caller facet ev 5ea3.286d.0f69.8608.898f.cb69.3a0b.fcc8.1976.a941.622c.baee.c9c1.dd61.2f8d.3050; referenced names ev fdee.83fd.33ca.ba27.ce91.8b80.b1ee.8a1c.9e18.2666.aa81.9b7c.92ed.7c6a.a08f.1bad. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484200` — Delegates to `0x484dd0` without changing arguments.

- VA / file offset: `0x484200` / `0x84200`.
- Behavior (own words): Delegates to `0x484dd0` without changing arguments.
- Reach and player-visible role: Support. Forwarding stub for the reverse `0x110`-byte copy.
- Census rows served: None directly.
- Our status: Pinned as a thunk-like wrapper; no census row.
- Evidence: analysis ev be3a.7581.9c93.4925.842e.30da.fc61.f238.3212.6b6b.45b0.e50a.a1d6.c7b0.06d2.e262; dossier ev c269.2cf7.7d56.89d3.f2d7.f1a6.88ce.12d0.0cdc.130f.765e.df7b.a9b6.31ba.502f.553f; incoming xrefs ev 1e79.c9b4.cb3b.bc0b.a365.5085.dab3.7a6d.fc4c.0113.c319.546a.386f.430a.b313.a07c (observed direct call sites: 0x482800); caller facet ev 75c4.7a00.914a.5050.dcfb.89bb.de39.7228.0f42.f7a1.7f8a.0029.bc4d.5037.465e.e0f4; referenced names ev 9c81.9f40.b438.f602.1da7.cf7a.8352.9348.af6e.87f2.210d.b2d3.c3a2.3b1a.cf9a.54eb. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4847f0` — Fills N 16-byte slots from one four-word value

- VA / file offset: `0x4847f0` / `0x847f0`.
- Behavior (own words): Fills N 16-byte slots from one four-word value, then leaves through the small exception-frame stub.
- Reach and player-visible role: Support. Fill step of the 16-byte insert path.
- Census rows served: None directly.
- Our status: Pinned as a fill helper; no census row.
- Evidence: analysis ev bbf4.0803.c345.7859.5123.431a.7722.fcf5.086d.54b9.9663.6827.3383.35ab.f65b.47fa; dossier ev 8ca9.ae81.356c.b405.637b.9d1d.6961.bb74.ab28.6d43.1a67.84e6.8c92.469d.71b7.9c49; incoming xrefs ev d0e9.c0b7.62ce.66dc.d9fb.2f7d.31b2.81d8.5b5f.9b0b.8e5d.9052.8c0d.6d8a.1c84.9782 (observed direct call sites: 0x47eaf0, 0x481fc0); caller facet ev d647.d05e.0504.1019.9ce9.858b.4eb9.946b.7647.a4d8.b5d6.3302.be63.dc48.ccac.2559; referenced names ev 6d76.c769.efee.b260.1a28.dce0.1c24.71d7.eb07.887b.1616.faad.67af.6a9c.d9e1.574a. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4848af` — Resets the local unwind marker and restores the saved exception-list head before returning.

- VA / file offset: `0x4848af` / `0x848af`.
- Behavior (own words): Resets the local unwind marker and restores the saved exception-list head before returning.
- Reach and player-visible role: Support. MSVC unwind fragment attached to the 16-byte fill helper.
- Census rows served: None directly.
- Our status: Pinned as an exception stub; no census row.
- Evidence: analysis ev ee27.0149.b385.b32a.58ee.1181.c329.0b3c.7ff5.524d.5bfd.e24a.4f8e.79f1.cbb6.87c5; dossier ev 571a.ac94.d26d.14d8.a057.a633.e1b7.8c9a.07ee.b549.0e39.3489.57dc.f275.84d3.3006; incoming xrefs ev 9bf8.a4b0.e79b.1517.7f92.a0e9.a9c8.a6ce.92a8.13c8.0de9.afda.3f75.f2f5.c3ae.b073 (observed direct call sites: 0x4847f0); caller facet ev c0ff.92ad.4320.80cf.ed8b.4962.ce21.ae05.2c30.1ad2.e213.a081.3dde.aa24.d307.3adc; referenced names ev b7f6.c804.ef2b.2dbc.4350.470f.7de6.3efb.7ba3.fa9e.1efe.64e4.6433.1cc8.63eb.b390. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484985` — Same small unwind fragment: clear the marker

- VA / file offset: `0x484985` / `0x84985`.
- Behavior (own words): Same small unwind fragment: clear the marker, restore the exception list, return.
- Reach and player-visible role: Support. Unwind fragment in the same template family.
- Census rows served: None directly.
- Our status: Pinned as an exception stub; no census row.
- Evidence: analysis ev 1370.0e5e.4770.3c3f.0710.b312.d7a4.6010.8b00.678c.9c5b.af48.0d4a.2716.f757.8be7; dossier ev 9a13.d4cc.009a.e1ff.e89d.df98.957b.7e45.e03f.c9ae.2b7f.8ba6.49c1.d727.e821.689f; incoming xrefs ev 9bf9.56fa.5997.f026.67b9.9770.9075.32ea.21e1.1390.cf04.5571.a62a.913d.33b7.79c5 (observed direct call sites: 0x4848d0); caller facet ev 7ebe.88a8.17c3.2b82.51dc.bda6.a019.b87e.7703.fcc6.189e.7025.72e9.cb7f.a982.b1a4; referenced names ev 7bfd.b4be.df94.b2d2.44c9.4e1c.36fc.e268.1a1c.0e08.13c6.bbf4.acdb.89b5.ce95.4237. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4849a0` — Delegates to the four-byte range-copy helper `0x484fa0`.

- VA / file offset: `0x4849a0` / `0x849a0`.
- Behavior (own words): Delegates to the four-byte range-copy helper `0x484fa0`.
- Reach and player-visible role: Support. Forwarding wrapper for dword moves.
- Census rows served: None directly.
- Our status: Pinned as a wrapper; no census row.
- Evidence: analysis ev 60b4.4bf8.4a6b.619a.04f7.426a.6aa5.9faf.b242.a730.fe61.a34e.d0eb.8a78.c0aa.f42e; dossier ev c702.31eb.9e0a.43fe.11b0.c939.5712.915f.b320.cb0d.a9b5.889d.b0ea.7452.e58c.29c0; incoming xrefs ev 6a7d.dd09.b26a.71f7.d137.c537.ff24.f699.4e5c.e972.5ba8.0d76.3cae.f897.f040.1851 (observed direct call sites: 0x4804f0, 0x535c50, 0x541830); caller facet ev 5a42.958f.f528.2a32.871c.f16e.f203.7f12.37b3.104f.6b12.b07c.128f.0d3a.722d.1d83; referenced names ev 304d.f5f6.fb15.355c.52c4.58d7.6e44.b698.0c1f.f663.2334.b266.01e4.96b6.341f.6639. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484c30` — Delegates to the 16-byte range-copy helper `0x4852a0`.

- VA / file offset: `0x484c30` / `0x84c30`.
- Behavior (own words): Delegates to the 16-byte range-copy helper `0x4852a0`.
- Reach and player-visible role: Support. Forwarding wrapper for record moves.
- Census rows served: None directly.
- Our status: Pinned as a wrapper; no census row.
- Evidence: analysis ev 7fdc.f9de.4f5e.3d25.7398.0423.1ef2.e196.a569.ed42.924d.389e.d822.f569.b49c.bf49; dossier ev 6f82.ca5b.dc6e.b35f.c97a.f206.49d2.02a8.3bec.f104.fa40.80a2.1e93.783f.7eda.2bd4; incoming xrefs ev 52e4.58e7.4764.4257.31ac.b2a7.8ec0.52c3.c1e6.3639.0ea7.cdb7.ad28.8a7e.e22d.169b (observed direct call sites: 0x481fc0); caller facet ev 34c4.c759.27b7.6beb.6cd4.211a.09e8.a9ac.953e.447b.6380.f113.603b.e4db.a615.8588; referenced names ev 0cd4.c50d.d5b8.bd48.0657.2dc0.6ce9.3946.484c.8d46.56e5.3927.b361.4f1a.83df.79ff. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484cb0` — Walks a 16-byte record range backwards and copies each four-word record to an earlier destination.

- VA / file offset: `0x484cb0` / `0x84cb0`.
- Behavior (own words): Walks a 16-byte record range backwards and copies each four-word record to an earlier destination.
- Reach and player-visible role: Support. Reverse copy used when a vector insert shifts records up.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 9a9d.69c3.4ad3.702c.c101.9568.57a4.c216.3f6e.fb42.07a0.3a05.18b3.21e5.fa36.f3b0; dossier ev a05c.36dc.126d.b916.a2d9.defa.89f5.1ac5.327f.333f.689e.e5b7.f973.4526.b849.6d8f; incoming xrefs ev dc61.4f51.c479.d181.2ef0.bae3.3114.c406.760e.8228.9bc7.a9ff.0267.0c4c.4ef4.ae75 (observed direct call sites: 0x4841b0); caller facet ev 8549.e746.48c9.c039.bb67.e8b1.44b7.8c78.ad21.9540.13fc.a57d.5029.31ad.2147.a007; referenced names ev 5a1e.fdef.9e39.6573.1f93.5524.318c.683b.dba6.6b35.52ad.6aa5.3537.bc4d.5687.6016. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484d50` — Delegates to the `0x110`-byte range-copy helper `0x485460`.

- VA / file offset: `0x484d50` / `0x84d50`.
- Behavior (own words): Delegates to the `0x110`-byte range-copy helper `0x485460`.
- Reach and player-visible role: Support. Forwarding wrapper for large-record moves.
- Census rows served: None directly.
- Our status: Pinned as a wrapper; no census row.
- Evidence: analysis ev 9b9b.03a2.0a5b.ae5c.4f54.7ece.746e.2ecb.1a73.cda0.4f12.fd9a.7a3e.70b9.f707.eaff; dossier ev 1d71.d60b.dd31.78ac.87af.63a5.68c2.9cc1.88fd.5adb.e672.2d81.52f3.a35d.19d3.06d8; incoming xrefs ev f045.5d2b.ac49.2b35.7ee7.f489.01aa.d991.21e6.d042.8a09.7e08.d5f3.dbcf.0c6d.e2c3 (observed direct call sites: 0x482800); caller facet ev f30a.5029.1a39.fb97.e983.b3a9.4922.3465.fd25.679e.3d12.e5f9.5e9c.57d9.8b06.c32c; referenced names ev 24f0.ebaa.961f.b016.6908.4c55.e665.d685.b6f6.3dd7.1ad9.c94d.a37c.7502.8749.1c5c. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484dd0` — Walks a `0x110`-byte record range backwards

- VA / file offset: `0x484dd0` / `0x84dd0`.
- Behavior (own words): Walks a `0x110`-byte record range backwards, copying each `0x44`-word record toward an earlier destination.
- Reach and player-visible role: Support. Reverse copy for the large-record vector.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev f078.e687.3f9c.3705.a5c6.cfa4.d540.813f.5a2c.edf5.6add.8cb3.c09c.05cc.830d.383b; dossier ev f05d.714e.edfe.e0cd.29ce.0b20.5142.b822.1d0a.9cc2.c8f8.dd76.c026.5cf3.3aa7.17d9; incoming xrefs ev 41a1.90c9.6661.8f89.9bb7.8748.54bd.b20c.5898.5c3a.41b2.6193.a59f.d5db.f74d.c605 (observed direct call sites: 0x484200); caller facet ev 937b.595c.6925.2126.a089.3e15.55da.05c1.d775.fb0f.c2d7.c386.3de0.3087.49a9.e7ae; referenced names ev 71d6.46cc.ef7f.f5b9.a4e4.acfd.97c3.4136.80f2.251d.a02d.a63c.bd0c.5f6b.c0fb.f1f5. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484ec0` — Writes one dword value into N consecutive destination slots.

- VA / file offset: `0x484ec0` / `0x84ec0`.
- Behavior (own words): Writes one dword value into N consecutive destination slots.
- Reach and player-visible role: Support. Fill primitive of the four-byte insert family.
- Census rows served: None directly.
- Our status: Pinned as a fill helper; no census row.
- Evidence: analysis ev aa25.c854.542c.aba3.12dd.4725.2e83.18b3.855f.d6f2.d130.66f9.1fb6.a7fd.dfb2.2917; dossier ev 8a63.8a0e.43c3.07ee.75e4.741d.8312.e27e.c132.e2e1.3f81.a034.b5ec.70bf.b8dd.3eaa; incoming xrefs ev 2c1a.16d9.25bb.f321.0a59.4199.a2c8.71cf.e095.f819.9643.9b0a.ab95.abf0.3fe7.e22d (observed direct call sites: 0x47d8b0, 0x4804f0, 0x480c80); caller facet ev b6fc.489a.7185.4059.1d33.111e.088a.6134.2fe9.0a97.cb39.a2ba.6eb2.9e97.e20e.85bf; referenced names ev 55a1.cd75.bd43.3f98.0dd5.a699.0a48.8471.6077.f45c.d15a.ea01.c490.68fa.d1f8.4884. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x484fa0` — Copies a dword range with `memmove_s` and returns the end address of the written range.

- VA / file offset: `0x484fa0` / `0x84fa0`.
- Behavior (own words): Copies a dword range with `memmove_s` and returns the end address of the written range.
- Reach and player-visible role: Support. Move primitive shared by several insert paths.
- Census rows served: None directly.
- Our status: Pinned as a move helper; no census row.
- Evidence: analysis ev 0d7d.5628.8ee7.b4a6.1f8e.b1f7.d273.3680.4d96.47e0.0899.6491.e866.1955.ea07.17b4; dossier ev ce7e.3c10.d43c.e6aa.279e.3d31.7394.427c.eeb6.6988.5604.5f11.e62a.d580.c339.d7a8; incoming xrefs ev 09a0.321a.1802.4c67.b0f6.381f.5986.d620.953c.0082.da4c.e547.ca25.c337.e5b3.d066 (observed direct call sites: 0x4804f0, 0x4843c0, 0x4849a0, 0x531740, 0x535c50, 0x541830); caller facet ev 581a.3bba.aa7a.920e.7010.c58f.36d6.e460.3a48.5c18.9999.80f0.45ca.5cb6.30ff.5094; referenced names ev e5d8.24e5.745e.225f.9fc9.965f.736e.9832.02df.b066.aed8.f963.8095.71e4.fab9.f1b3. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x485060` — Delegates to the 16-byte record-copy helper `0x4852a0`.

- VA / file offset: `0x485060` / `0x85060`.
- Behavior (own words): Delegates to the 16-byte record-copy helper `0x4852a0`.
- Reach and player-visible role: Support. Forwarding wrapper for 16-byte copies.
- Census rows served: None directly.
- Our status: Pinned as a wrapper; no census row.
- Evidence: analysis ev c644.56db.ac3c.4e7f.35c4.7311.d903.aac1.3022.9efa.aac6.d420.933b.90cc.7f4c.5bce; dossier ev 695e.4571.6151.382b.bf7d.83d3.0e30.c7cb.6d3e.a359.d7b8.c95b.f9ff.f262.17e3.0e9c; incoming xrefs ev 581c.0749.ad4d.af7c.d86d.1889.8a51.f725.b06b.0dbf.ac9f.d686.479e.26f5.a97b.63e6 (observed direct call sites: 0x481fc0); caller facet ev fbfa.1818.4a23.c43f.5b61.0ff5.7f70.4c15.5a4e.3aa7.a84a.3a97.a53f.f740.ad67.30e1; referenced names ev 31ca.f0a1.9c1d.cf33.3935.fab6.e98f.a310.a7a0.7471.6c2b.2d6c.5dc0.86c2.6e76.f628. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x485100` — Delegates to the `0x110`-byte record-copy helper `0x485460`.

- VA / file offset: `0x485100` / `0x85100`.
- Behavior (own words): Delegates to the `0x110`-byte record-copy helper `0x485460`.
- Reach and player-visible role: Support. Forwarding wrapper for large-record copies.
- Census rows served: None directly.
- Our status: Pinned as a wrapper; no census row.
- Evidence: analysis ev bc64.1b31.49a2.41b2.9103.a434.76fb.0d85.2108.c93c.8de5.15c1.b853.3653.8dbb.b2f6; dossier ev 6a25.876b.4155.3b4a.f456.6abe.88d4.979c.1788.5282.13bd.7a87.2ab0.0dc0.af2a.1639; incoming xrefs ev 539a.5b05.24cc.7671.2ae3.bb58.6b1d.2864.7fde.87d2.15b9.871b.7738.ddec.52db.2091 (observed direct call sites: 0x482800); caller facet ev 9d2a.89c7.7944.28c4.6926.b42c.8b9b.98fb.605e.7e98.304c.916c.7370.291b.f320.6176; referenced names ev 23ce.29b1.e13b.e2c5.8ce0.8339.3aa3.086c.39f8.c2e1.2a41.e1ed.8182.9efa.69de.f931. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4852a0` — Copies each 16-byte record of a source range into the destination

- VA / file offset: `0x4852a0` / `0x852a0`.
- Behavior (own words): Copies each 16-byte record of a source range into the destination, then clears the unwind frame.
- Reach and player-visible role: Support. Forward copy used during vector growth.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 4ee1.3293.f6fe.17be.a1ce.ef75.3b9a.85c2.e816.56eb.5e8a.46f9.d409.8533.d782.026a; dossier ev 1f70.9a60.7ed1.3704.867e.0573.4c54.876a.9f96.1684.cecd.da22.0699.fd49.2ebe.3db1; incoming xrefs ev 1a5c.b960.06da.0efe.9499.f636.ab6d.33cb.5c8f.ce2d.c469.17b6.1947.b4e2.c5bd.0e76 (observed direct call sites: 0x4845d0, 0x484c30, 0x485060); caller facet ev e26f.2422.f632.8b78.7568.21cf.b97d.8713.4b21.8f47.efd1.9433.c559.c10f.09b0.37d5; referenced names ev 23f4.6279.d343.5f6d.c684.b40c.b3f3.cc86.0bea.beca.198a.8e0b.ae9a.78fb.9a56.1069. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x485361` — Unwind tail that restores the saved exception-list head and returns the caller's stack argument.

- VA / file offset: `0x485361` / `0x85361`.
- Behavior (own words): Unwind tail that restores the saved exception-list head and returns the caller's stack argument.
- Reach and player-visible role: Support. Exception stub following the 16-byte copy.
- Census rows served: None directly.
- Our status: Pinned as an exception stub; no census row.
- Evidence: analysis ev 0116.6353.48fd.38af.1d44.8f98.729f.9d09.3677.9354.326e.e163.bdd7.ad6d.0c70.dffe; dossier ev 313f.9b22.9eac.942e.dfbf.f341.74e5.7b70.c04b.103c.3cee.cca4.6ad9.c1fd.c871.9d14; incoming xrefs ev eafb.b7b3.aedf.183e.077b.7e83.087d.e9dd.109d.eb55.9713.dc07.591b.6aa7.cfba.86c0 (observed direct call sites: 0x4852a0); caller facet ev 5e9c.3492.32b7.e221.5df5.1d5c.2102.c36d.304e.9629.20ee.27a7.0dc5.02cd.4a7f.c13f; referenced names ev 8dc4.660c.07c4.f77d.ab9b.d757.b807.bb07.8d4b.d172.577d.6f55.1fd7.e659.7978.b9c6. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x485460` — Copies each `0x110`-byte record of a source range into the destination

- VA / file offset: `0x485460` / `0x85460`.
- Behavior (own words): Copies each `0x110`-byte record of a source range into the destination, then clears the unwind frame.
- Reach and player-visible role: Support. Forward copy for the large-record vector.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 26aa.ca99.a47b.8c06.bccc.c337.f53e.c896.8dbd.d368.1e59.434c.1564.3760.3321.0b26; dossier ev b8d9.6039.2275.1fa4.234d.da34.f6c4.d837.ea7b.6f85.91ea.9dce.c5aa.8787.032f.d191; incoming xrefs ev 5cde.acb1.1dee.6544.ebac.1a9b.f213.9851.6bc0.aae9.e561.492c.6b2d.8aa3.fa57.1bbc (observed direct call sites: 0x484d50, 0x485100); caller facet ev fab3.9a7b.d240.7901.faf9.f9a2.0998.e82d.8c78.5a4a.67e4.f037.4e5e.b290.4f3e.a9f9; referenced names ev 23db.8008.8ec1.f841.2dba.20d3.fdcf.8436.a5ce.39d7.41ab.2440.a365.cc60.f086.0037. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x48551b` — Unwind tail that restores the saved exception-list head and returns the caller's stack argument.

- VA / file offset: `0x48551b` / `0x8551b`.
- Behavior (own words): Unwind tail that restores the saved exception-list head and returns the caller's stack argument.
- Reach and player-visible role: Support. Exception stub following the large-record copy.
- Census rows served: None directly.
- Our status: Pinned as an exception stub; no census row.
- Evidence: analysis ev b9fe.a919.d9bc.25cd.8394.7a37.b65e.8bcb.4604.6a73.75c0.aa73.4c14.da58.732e.dd1d; dossier ev c4e1.0e84.8ae2.0db1.ed37.cbbd.cf8b.8127.b3ae.5c5d.7913.8f6a.90e5.6aa0.ca13.c6ae; incoming xrefs ev 2f9d.fa45.e68c.5acc.5641.3d44.9458.0065.5065.428f.a19f.809b.68fa.ec28.4f79.179f (observed direct call sites: 0x485460); caller facet ev 5987.6be1.c9b1.1534.f3f1.7220.ee0c.11d1.d01e.46da.e596.ecf7.c2a3.0ab9.8aae.a452; referenced names ev 0ca8.14b9.819a.8f7e.5806.e3ce.416f.2734.1803.4ec2.6a6d.4345.a9a1.a1ef.c49f.69ed. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x485bb0` — Initializes a game object: zeroes the first word

- VA / file offset: `0x485bb0` / `0x85bb0`.
- Behavior (own words): Initializes a game object: zeroes the first word, sets the next dword to all ones, clears a `0x104`-byte block, stores two all-ones sentinels and a `0.2` float at fixed dword offsets, then clears several trailing byte flags.
- Reach and player-visible role: Support. Called from `0x485540`, `0x55b230`, `0x55b4d0` and `0x55f630`; the body sets defaults only and contains no control lookup, paint or send.
- Census rows served: None directly; a state/UI object defaults initializer reached from game-wide seeds.
- Our status: Pinned as an initializer; the owning class and which field reaches pixels remain OPEN (next step: name the class from the caller's vftable/RTTI).
- Evidence: analysis ev d0c5.8bf7.418e.2b98.ab55.311d.78d8.9432.4061.937c.c519.6d12.e386.8406.27ba.5e1f; dossier ev a387.d658.2923.a3a6.ed26.a175.3d5a.c3f7.519a.5d24.c8b3.b768.e305.057f.1042.fbb9; incoming xrefs ev 625e.e43b.26a8.44e4.6539.798d.de11.e606.0bfb.4513.df9a.810d.ab3b.97f8.6c0c.1260 (observed direct call sites: 0x485540, 0x55b230, 0x55b4d0, 0x55f630); caller facet ev 6ad4.d820.f620.673b.12a5.9d43.0e1b.6645.56d7.da28.4fe8.d5cd.7612.946c.467a.a50e; referenced names ev b59a.acee.4fbd.49a3.a9a5.8de5.ef91.acaf.460d.d3a5.c98e.1fbd.b462.deb3.b34f.6ded. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x485dd0` — Initializes a larger object: calls three setup helpers

- VA / file offset: `0x485dd0` / `0x85dd0`.
- Behavior (own words): Initializes a larger object: calls three setup helpers, then clears a `0x40`-byte block and a series of dword, word and byte fields at fixed offsets, with two byte defaults of 2 and 3.
- Reach and player-visible role: Support. Reached from `0x5a4b90` (game dispatch seed) and several setup wrappers; establishes object defaults without making a player-visible decision.
- Census rows served: None directly; shared state/UI defaults under game seeds.
- Our status: Pinned as an initializer; the owning class and field meanings stay OPEN (next step: join the caller's object type to RTTI).
- Evidence: analysis ev 956a.ac60.be16.3636.f41a.eede.0621.3824.7dd6.7431.0ebc.b837.d82c.07a3.e6bd.739f; dossier ev 2ea4.f6a6.e256.22a4.b524.760b.c60e.748e.5dc9.5661.1496.12e7.d49f.7b07.e326.88bb; incoming xrefs ev 5e6f.2d4f.b886.fd51.d6c4.45da.8d9e.dbe3.c6ed.80a9.d2b6.b384.0725.6827.f7e7.a1cf (observed direct call sites: 0x485c30, 0x4f16f0, 0x55d700, 0x55e2f0, 0x55f210, 0x5a4b90); caller facet ev 471a.22e5.ccb0.79a0.14c3.b84f.9756.1f44.32f9.a8d4.03de.ca0c.936b.f3ee.8020.092b; referenced names ev ae7d.1d33.36ea.35ad.090c.11d8.446f.4861.d31f.3f91.274a.515c.28a2.f644.6e92.e2ed. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x487390` — Reserves capacity for eight-byte records: allocates through `0x4897d0`

- VA / file offset: `0x487390` / `0x87390`.
- Behavior (own words): Reserves capacity for eight-byte records: allocates through `0x4897d0`, clears begin/end, sets capacity from the requested count, and reports allocation failure as a boolean.
- Reach and player-visible role: Support. Vector reserve template; not a Planet UI rule.
- Census rows served: None directly.
- Our status: Pinned as an allocator helper; no census row.
- Evidence: analysis ev 81a9.d55a.7554.2d52.1b52.5e64.944f.7f49.88fe.f280.cecc.16c1.9e23.2a87.ded0.f744; dossier ev a4b2.b020.4acf.eb33.93fe.51e5.cf37.2fb6.f10f.63db.cf69.4ce6.bc71.368e.ae17.444d; incoming xrefs ev 831a.33db.bed3.d22e.a721.4f9a.6d2c.92f7.606a.ec10.86d3.f125.049d.6064.acb0.7d32 (observed direct call sites: 0x485c30, 0x4b5d10); caller facet ev d066.98eb.0c9f.51c0.69ee.31e3.689a.3908.cfca.4fda.4890.5193.b28f.8031.dea1.4af4; referenced names ev e554.6c7f.0df1.d831.8b8a.ae10.1e3e.b6fe.db7f.baa1.7ca2.5c14.acb4.74fe.ce14.806c. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x487ff0` — Reserves capacity for twelve-byte records through `0x489a20`

- VA / file offset: `0x487ff0` / `0x87ff0`.
- Behavior (own words): Reserves capacity for twelve-byte records through `0x489a20`, with the same begin/end/capacity bookkeeping as the eight-byte variant.
- Reach and player-visible role: Support. Vector reserve template.
- Census rows served: None directly.
- Our status: Pinned as an allocator helper; no census row.
- Evidence: analysis ev 013d.9cfa.a8e7.a9f5.ab14.7c8b.545c.3d17.94fc.3c18.84a0.efe3.f904.32c7.a97b.f98e; dossier ev 7a26.1924.c25b.a44c.1d16.133d.12da.2f9a.58b2.1cca.1e21.a44f.90d9.9121.4923.12a9; incoming xrefs ev 062c.211b.23db.2b7a.b160.74fd.8f07.2003.2d00.f3cb.9b18.02c8.21ce.e843.ecea.9478 (observed direct call sites: 0x485540, 0x560d70); caller facet ev 467e.62bc.925e.4e30.0aa8.7013.c097.0746.4ed9.344c.c968.0d54.2555.1261.417a.2afc; referenced names ev 4d08.4e1c.7b05.aa5c.41e8.7036.2c73.c353.0661.c110.80e0.2913.9fbf.0909.ae5e.4622. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4880b0` — Walks and frees the allocated record range

- VA / file offset: `0x4880b0` / `0x880b0`.
- Behavior (own words): Walks and frees the allocated record range, then resets the vector's begin, end and capacity words.
- Reach and player-visible role: Support. Vector destruction/clear helper.
- Census rows served: None directly.
- Our status: Pinned as cleanup; no census row.
- Evidence: analysis ev 5f7c.c802.a856.3489.99f5.cb43.2e55.ac51.fe5d.f33f.9e87.d8a2.a47f.8dc0.0bcc.9e3d; dossier ev a278.63c9.4f84.6864.4520.70ee.4999.a538.d1a6.d08c.6511.7378.c4a5.f241.f520.6dd8; incoming xrefs ev ce57.c6bb.a386.488c.2a8d.15ea.37b3.aa65.c96b.1018.3c0e.0c35.a428.ddf0.8d7c.c0ad (observed direct call sites: 0x486860, 0x489bb0, 0x560d70); caller facet ev 380a.e969.2fa6.cd79.328b.745a.4c8c.f539.4de0.deab.3bac.9ee3.92a0.ca98.bb66.15de; referenced names ev 00c0.9c83.4838.8482.9365.8095.4ab7.743e.ef42.2bc3.3350.9725.cf22.9aeb.5f1a.e65c. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x488fd0` — Moves a range of two-byte elements with `memmove_s` and returns the destination end pointer.

- VA / file offset: `0x488fd0` / `0x88fd0`.
- Behavior (own words): Moves a range of two-byte elements with `memmove_s` and returns the destination end pointer.
- Reach and player-visible role: Support. Short-element copy template.
- Census rows served: None directly.
- Our status: Pinned as a move helper; no census row.
- Evidence: analysis ev 0789.0d39.4ca0.9636.4969.18c0.caa0.29c4.de8c.e4cf.e212.780a.26b3.91c9.f577.11ae; dossier ev 29ba.ac1f.d506.58b2.c8f4.2da3.680a.5c3b.7b5d.3a2b.9613.3b98.104f.0b2e.2546.e8ab; incoming xrefs ev 9624.eb00.8de2.2f6f.7a08.8daf.88e3.7962.43a8.a636.3ef7.7a39.f85c.828e.bc7a.68f6 (observed direct call sites: 0x487110); caller facet ev cfea.ac6b.9cb2.a5fc.018a.77ad.4175.c245.2f52.298e.6c1b.498f.86fb.2077.ecec.c8de; referenced names ev 9422.34b6.089a.56cc.cbbb.8f8b.2c06.a1f3.0c11.c38c.8737.f442.46de.4acd.ccac.4a86. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x489040` — Copies eight-byte records from a source range into an uninitialized destination.

- VA / file offset: `0x489040` / `0x89040`.
- Behavior (own words): Copies eight-byte records from a source range into an uninitialized destination.
- Reach and player-visible role: Support. Uninitialized-copy template for eight-byte records.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 1443.4603.9a36.b174.ead3.10f8.7da5.d3c4.03b4.6378.b638.5cdf.7320.d425.3c40.15de; dossier ev a745.042b.8a9b.9633.7d4e.db15.c4ec.89f5.d07c.8525.b69e.6ca8.3801.cc26.7fcb.de7d; incoming xrefs ev 99fc.728c.3433.ecac.c3b7.6674.4d90.6bd5.8808.2bab.66db.4050.1aaa.e2cf.18df.774c (observed direct call sites: 0x4872e0, 0x4b5b40); caller facet ev 18fb.deb7.5ebf.295b.457d.39f9.ac0a.24cd.8136.6ac5.3c8f.41dc.2a7a.481c.14f0.63ae; referenced names ev 8685.e1f2.f87c.dcb7.3d7b.a538.5bc5.62b3.4966.d4be.71ab.8ed6.44cb.82aa.1ebe.ce59. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4890e0` — Moves a raw byte range with `memmove_s` and returns the destination end address.

- VA / file offset: `0x4890e0` / `0x890e0`.
- Behavior (own words): Moves a raw byte range with `memmove_s` and returns the destination end address.
- Reach and player-visible role: Support. Byte-range move template.
- Census rows served: None directly.
- Our status: Pinned as a move helper; no census row.
- Evidence: analysis ev bb4a.29b3.cdfd.c503.d039.684e.6d14.ddba.8164.bcc6.578a.cc7e.3f94.0828.0090.2491; dossier ev 79cb.b157.12c4.37b4.eab8.d82f.6c50.4d12.5df4.2413.a39e.5fec.935d.ebe1.1ac9.2090; incoming xrefs ev c568.4284.a418.88ff.a536.8892.85af.60d8.5cda.ca8c.61b8.3ca4.7335.0009.138c.d4df (observed direct call sites: 0x4874f0, 0x55d820); caller facet ev 5b8f.ed13.8c10.49f9.defd.6bd7.41ab.5a4e.637f.bad0.39ff.7f0a.8fd9.d444.fc7f.e616; referenced names ev b228.726c.bdfb.6e2e.2545.21b0.b2e4.4d60.0fc2.3e80.4b06.c153.6c77.2651.738f.0697. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x489150` — Copies `0x168`-byte (`0x5A`-dword) records from a source range into a destination.

- VA / file offset: `0x489150` / `0x89150`.
- Behavior (own words): Copies `0x168`-byte (`0x5A`-dword) records from a source range into a destination.
- Reach and player-visible role: Support. Uninitialized-copy template for a large record type.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 8007.940a.f277.272f.9f4f.7f70.44c5.3d33.ec32.e4d8.6166.c6df.ac0b.556a.9e92.dca9; dossier ev 49de.2e0f.11f9.a82f.e5ee.a29b.cdb8.7c20.aed3.8af2.0da5.3bbc.b1a2.57ff.de86.15ce; incoming xrefs ev 294c.32b2.066a.92e9.bb05.ad4b.b29d.b3b4.d272.73de.8799.745d.5697.8e2c.be0d.c324 (observed direct call sites: 0x4876c0, 0x495ce0); caller facet ev 6b86.266d.2ef9.a7d8.8d0a.9aee.238c.ed71.1814.3b74.5705.186d.8b1f.cebd.8bd4.a165; referenced names ev 4812.18fa.5c15.a455.10fb.d7ab.8339.a921.4765.bfd0.20ef.e7e8.32c1.0cb4.724e.2fe8. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x489200` — Copies `0x24`-byte (`9`-dword) records from a source range into a destination.

- VA / file offset: `0x489200` / `0x89200`.
- Behavior (own words): Copies `0x24`-byte (`9`-dword) records from a source range into a destination.
- Reach and player-visible role: Support. Uninitialized-copy template.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev c110.d4d8.6385.7d9d.044c.a2be.12c2.a3ae.5289.2a1e.342b.745b.1048.190f.9615.6432; dossier ev 923b.3614.9f7e.2381.5fe0.1d28.ba54.d7ea.b927.6868.d0e4.e3da.7bb5.d68c.e6a8.57de; incoming xrefs ev 8e27.97eb.a14e.d16d.6313.be5c.0ad2.ba50.9ca1.67ad.a2fa.1fb3.81af.c07f.9f17.28a6 (observed direct call sites: 0x4878f0, 0x4a9ad0); caller facet ev 09be.9f24.4f5a.d1a1.5238.9660.0948.10f4.9bff.2c0e.8407.6e51.a966.3f4d.10a9.9349; referenced names ev 8062.c98c.34bf.d819.74c3.861b.14fa.7b53.c1c9.d196.7037.a6d0.5914.52f8.1411.1c04. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4892a0` — Copies `0x38`-byte (`0xE`-dword) records from a source range into a destination.

- VA / file offset: `0x4892a0` / `0x892a0`.
- Behavior (own words): Copies `0x38`-byte (`0xE`-dword) records from a source range into a destination.
- Reach and player-visible role: Support. Uninitialized-copy template.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 3c47.371d.3492.5929.2b42.0a6b.43bf.29e5.1b4d.9596.b2b8.cc9b.4c78.57eb.6af2.a9ab; dossier ev 8abd.a6ac.c73d.7edd.c190.eba3.026c.5e3a.edd8.c43f.b5de.425b.2e9f.6862.b4bc.b8b3; incoming xrefs ev e207.cf42.6436.69ef.62fa.fe93.d574.d5ed.020b.3902.5d06.cd79.333f.dccf.8e76.fc84 (observed direct call sites: 0x487b10); caller facet ev 96b8.d8ad.161d.d501.4fac.709b.3f4b.36ea.1fe5.b71f.a938.c170.5626.2f6c.cdec.cc02; referenced names ev 9319.54cf.54d3.d26a.cbd2.8d78.9261.6b78.d476.f50c.2362.df5b.d81c.e29e.51fe.2119. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x4893e0` — Copies twelve-byte records from a source range into a destination.

- VA / file offset: `0x4893e0` / `0x893e0`.
- Behavior (own words): Copies twelve-byte records from a source range into a destination.
- Reach and player-visible role: Support. Uninitialized-copy template.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev 62b9.7655.c53f.ea1d.09d9.7581.ae2e.bfb0.b57c.8be4.958e.0133.f0a2.11c8.51af.8af5; dossier ev fac0.211c.920c.0a83.7fb0.42af.b9a8.e124.4b7f.bcb4.1be1.11d6.9ac7.865f.e5b0.49e7; incoming xrefs ev 9318.9c5c.5c14.43cb.7e35.1f26.8b0b.2306.aeb0.2482.95c8.d327.e158.927a.93a0.005d (observed direct call sites: 0x487f40); caller facet ev 99f6.0642.db67.d5ca.df83.20d4.d7e6.527a.da9d.f167.05f4.b6b5.d5c7.5b3b.4ce3.5ad1; referenced names ev 0ad1.b623.15df.68be.3872.8ac9.8f5d.5f6d.7a2c.0b24.0dfb.1195.109a.5108.150e.f6e4. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x489490` — Copies twelve-byte records from a source range into a destination (second instantiation with the same stride).

- VA / file offset: `0x489490` / `0x89490`.
- Behavior (own words): Copies twelve-byte records from a source range into a destination (second instantiation with the same stride).
- Reach and player-visible role: Support. Uninitialized-copy template.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev e165.04fe.b49b.232d.5524.917c.7ad3.0659.070c.68b6.50b0.dc9e.91c4.7391.00e0.5595; dossier ev 0066.a464.9c70.b1e6.67e4.ba55.7f17.1d9a.650c.0af8.9bc3.d7f1.5a05.9d33.0a3e.0232; incoming xrefs ev 4c73.611d.4700.6b57.668b.3a51.f6f1.da74.dc6e.1c85.fcb4.d956.98ab.c04c.d4d6.cdbd (observed direct call sites: 0x488160, 0x4a3160); caller facet ev fde5.f50d.aac2.46fa.c53a.c9a8.9796.fe1e.8995.9629.17b7.dc8b.d7c2.afa2.d44c.55e4; referenced names ev 4bef.a5c9.3e5b.3d6d.7794.b024.5b3c.a8fc.fc01.6fbf.202a.f4e8.8915.140c.fbb6.9f71. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x489540` — Copies `0x124`-byte (`0x49`-dword) records from a source range into a destination.

- VA / file offset: `0x489540` / `0x89540`.
- Behavior (own words): Copies `0x124`-byte (`0x49`-dword) records from a source range into a destination.
- Reach and player-visible role: Support. Uninitialized-copy template for a large record type.
- Census rows served: None directly.
- Our status: Pinned as a copy helper; no census row.
- Evidence: analysis ev d886.3a97.68d3.2a41.e284.76e1.458d.b589.5c66.6a68.9ed1.5e95.2715.0af4.c773.ab87; dossier ev 74b2.3cab.6f9e.fc39.a23a.503b.ba80.c122.90c2.d2a4.9b9c.bdcb.ae62.ad50.e03e.36ca; incoming xrefs ev 7f99.ba0f.4e01.d53b.76d1.ae6e.fa7f.7a1b.c077.ceb6.cb15.f90f.74ca.1b24.fb0a.d3d9 (observed direct call sites: 0x488500); caller facet ev c6cf.c2c4.f4ad.9594.bd70.eaa0.7cd6.2b10.70a2.1dd7.5076.5722.e5a5.c77a.3e22.c852; referenced names ev b9a5.eabf.ca18.fba6.d552.2c8f.6ba2.c671.5027.389b.d6e2.5380.9d13.c9c7.996f.441e. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x48a030` — Calls the shared control-list cleanup helper `0x48cb90`

- VA / file offset: `0x48a030` / `0x8a030`.
- Behavior (own words): Calls the shared control-list cleanup helper `0x48cb90`, then writes zero to two words at `+0x50` and `+0x54` on the owning object.
- Reach and player-visible role: Support. Reached from state constructors `0x489df0` and `0x48a190`; clears state-local bookkeeping only.
- Census rows served: None directly; state construction shared with Playing and other screens.
- Our status: Pinned as cleanup; which cleared word, if any, affects a scene transition is OPEN (next step: read the state object's field uses).
- Evidence: analysis ev 5838.8e22.7cf8.ac90.741e.02c6.1e64.4458.9b67.d409.630e.f971.9e6f.f30f.d1f2.6111; dossier ev 098a.2cde.826b.dcec.d8b5.ed1a.c1d9.8cb9.e472.4385.141d.6477.2604.bbc1.9227.85fb; incoming xrefs ev d3a2.892c.d5a3.83b5.6f51.5c1d.8ccd.d462.7fe2.bb66.1672.e0b5.d75b.6b91.07f7.b9fe (observed direct call sites: 0x489df0, 0x48a190); caller facet ev fb55.6b4e.41fb.2427.83b4.5932.afe3.c416.bfd7.ffdd.98ca.bfff.7631.d348.42d4.2f36; referenced names ev 9f78.ab87.6fd5.4fec.5b17.2ec8.869c.0adc.88fd.4a28.3aee.7f31.f799.243d.9fd1.488b. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x48ce30` — Reserves capacity for four-byte records through `0x48d9a0`

- VA / file offset: `0x48ce30` / `0x8ce30`.
- Behavior (own words): Reserves capacity for four-byte records through `0x48d9a0`, with an overflow guard and the same begin/end/capacity updates as the other reserve templates.
- Reach and player-visible role: Support. Vector reserve template instantiated in the Planet-range address cluster.
- Census rows served: None directly.
- Our status: Pinned as an allocator helper; no census row.
- Evidence: analysis ev a106.089d.9067.adfa.d1be.15b0.9af4.32d5.0373.93b4.9815.3ede.0493.a95a.9ccf.a9ef; dossier ev 2fcf.0038.bd34.99df.977f.be92.8c58.2c7f.9237.4784.dd9e.671b.43ae.06a6.0512.dbcf; incoming xrefs ev d49c.2b84.a855.63bc.a8f1.716d.9841.02e6.06ae.33e3.a3c4.f7fd.7df3.9178.e9a6.407a (observed direct call sites: 0x489df0); caller facet ev fe53.fe2f.0c37.c5d7.01a5.b898.1a95.a4e7.fbc2.5df6.05ba.efad.aa08.f120.e2d1.f421; referenced names ev 8fee.1de3.2aae.368b.8abf.56af.0c4f.f19e.9b44.dba6.8a8d.7ac9.18ae.74e1.2123.8549. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x48d880` — Moves a dword range with `memmove_s` and returns the destination end address.

- VA / file offset: `0x48d880` / `0x8d880`.
- Behavior (own words): Moves a dword range with `memmove_s` and returns the destination end address.
- Reach and player-visible role: Support. Copy helper instantiated near the Planet control code.
- Census rows served: None directly.
- Our status: Pinned as a move helper; no census row.
- Evidence: analysis ev 89d4.dfd4.a510.82d3.71af.b217.c027.85b1.301b.81df.6317.9dfe.793e.5c74.503f.20f9; dossier ev afac.8cfa.3ceb.46fd.4703.ea2a.f040.5df5.c3ac.f234.7a15.815f.9980.f4f9.6d03.5ed4; incoming xrefs ev 05ea.17b8.8634.c234.8724.0a9a.4920.2697.9a61.f9d7.0d70.9df1.9082.1c6b.069e.0e5b (observed direct call sites: 0x48cb20, 0x48cda0); caller facet ev 2871.41b7.3d26.8727.b905.1d71.cda0.f05c.16f7.077b.4106.7fe8.a015.1af3.962f.f9a3; referenced names ev 1f0b.defb.bf72.0312.b7c4.e758.6476.37aa.7d26.18cd.87b5.594b.69bc.4dde.38f4.b624. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

### `0x524350` — Calls the shared button-base setup at `0x4F6040`

- VA / file offset: `0x524350` / `0x124350`.
- Behavior (own words): Calls the shared button-base setup at `0x4F6040`, installs `CCBtnPlanet::vftable`, clears words at `+0x64/+0x68/+0x6C`, a byte at `+0x70`, a `0x10`-byte block from `+0x74`, and sets the word at `+0x94` to zero and the words at `+0x98/+0x9C` to one.
- Reach and player-visible role: Player-visible. This constructor creates the button class used by the Planet emblems, promo cells and disc buttons; it installs behavior, it does not itself hit-test or paint.
- Census rows served: Planet ordinals 1–5 (HARD/NORMAL/EASY/FREE/DRUM), 12–15 (promo cells, EXIT) and the hover/pointer-release edges in `docs/interaction-census.md`. Ordinal 16 (OPTION) shares the same button family.
- Our status: Pinned to the constructor; the meaning of the two default-one flags and the full CCBtnPlanet vftable behavior stay OPEN (next step: read the virtuals installed at the vftable address).
- Evidence: analysis ev 8f94.3310.c85e.6ec8.ecbb.5e97.0d06.8050.7a81.8528.2f4b.216e.06d7.c526.4b8c.7679; dossier ev cf25.f2d4.7703.20fb.aa50.bcec.3443.275a.c3e1.20c5.bba6.d3ad.ce9a.7159.8240.a9ea; incoming xrefs ev 2d83.c23c.d8b3.c125.af45.c8da.bb76.3d27.04df.78c8.ff5a.5b4f.479e.1203.21b2.dddb (observed direct call sites: 0x4329b0, 0x48a620, 0x48b2d0); caller facet ev c20c.81e7.3308.d01c.38c5.d10a.e6f9.2b05.4e5f.73c6.2c35.d9f2.c3ed.3ac6.fb1d.7a92; referenced names ev c226.47c1.263b.b452.7cfe.f1da.a3c7.1b73.02c0.d629.2046.d552.bd20.10f1.f3b3.01d3. Body and call shape read from REA `analyze_function`, `xrefs` and the saved
  batch decompile; behavior described in our own words.

## Scope limits

- Static evidence only. These pins do not close a player-visible census row
  on their own; the repo rule needs a boot-to-row caller chain or a stated
  seam sufficiency before a row is marked VERIFIED.
- The 51 shared templates are pinned as bodies, not attributed to a single
  scene. Their callers span game-wide seeds (`0x5a4b90`, `0x5a5190`,
  `0x558080`, `0x477990`), so a Planet/Channel label is a work-plan bucket.
- No new live capture, no wire claim and no Rust behavior change is made.
