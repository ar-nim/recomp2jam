# Statics pins — Net handlers B (callers of net handlers, coverage burn-down)
Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`. Provider: REA 6.10.1 / Ghidra 12.2; target bound from
`install-root/O2Jam/OTwo.exe` with the pinned digest above.

Branch `cb2/pins-net-b`. Pre-flight `git rev-parse --show-toplevel` printed
`/home/arnim/Projects/recomp2jam-lane-pins-net-b`; brief base `a15244e` and
observed HEAD `e5e4fb7` are both commits in this checkout (the brief's base is
the parent of a later merge; no reset was made).

Scope: all 48 addresses in `slice-net-b.txt`, bucket `Net / packet handlers`,
`pinned_in_repo=no` in `procedures.csv` / `triage-final.json`. Pinned here:
**48/48**; left: **0**. The address-cluster label covers fifteen shared
cleanup, container and record helpers; the remaining 33 bodies are wire
handlers. Evidence IDs are dotted four-digit groups: remove the
dots and join after `ev_` to obtain the full REA ID. File offsets come from
REA `address_to_file_offset` (image base `0x400000`); they were checked for
all 48 addresses and match VA minus base, e.g. `0x551390` → `0x151390`.

A static pin is not a player-visible close. Census rows stay OPEN until a
live caller chain from boot is recorded (Tier 3 where the row demands it);
see `docs/live-path-reachability.md`. "Reached from" below is the provider's
direct-caller edge, not proof of a runtime packet or press.

## Shared dispatch placement

Every wire handler below is a direct callee of game-TCP dispatcher `0x558080`
(analysis ev 35df.f6f3.0b2f.a741.45bc.5c5b.9320.dd39.5187.54ae.d584.f61f.eb5d.207a.c14b.ac71;
dispatch body ev 5032.6588.ab15.2e39.7a49.5695.3331.7425.09e6.8c5e.caaf.afe3.2518.03ff.782c.8ae9;
xrefs ev 924d.9480.def9.b9c8.007d.f0f1.c88c.0981.9d00.0707.f1b5.74c9.f970.b84e.7e04.db5c).
The dispatcher subtracts a base opcode, bounds it, and indexes a table; the
handler address is also stored as a callback pointer at data VA `0x64d028`
(bytes ev 8e48.e0e9.e53c.75d5.8d5a.0c48.f47c.c6a1.76e2.79b8.26e0.4645.d732.71a0.250a.c897).
The static graph is `handler ← 0x558080 ← data slot`; the boot chain that
installs that slot is not resolved here and stays OPEN. Opcode numbers in the
entries come from that dispatch table, not from the handler bodies.

## Inventory

| VA | File offset | Opcode | Role | Analysis evidence |
|---|---|---|---|---|
| `0x551390` | `0x151390` |  | support | ev 5d9c.27fd.c70b.7ec4.bfd8.b63b.9402.964f.6cb8.775b.e9be.2cf4.e3ce.85fa.3850.e9b8 |
| `0x551850` | `0x151850` |  | support | ev 969f.2c04.6ae0.87b4.13d9.ee4c.a7c8.d92f.f3f4.2a04.f053.f020.c75d.e271.cc54.d457 |
| `0x556ed0` | `0x156ed0` |  | support | ev 80aa.239a.323d.8000.8895.b70b.8862.e72b.1489.b30d.9250.4040.70dc.e75e.99ed.7da7 |
| `0x5572ef` | `0x1572ef` |  | support | ev fe11.160a.1a15.7648.ac02.2d50.b3c2.b259.f7ca.859b.0cfc.205b.25d7.631f.a366.14d0 |
| `0x55756d` | `0x15756d` |  | support | ev 96ae.e104.1ab5.2fb9.2e6c.e229.7377.7c67.fccf.e514.453c.36e1.b591.7725.8105.c548 |
| `0x557850` | `0x157850` |  | support | ev 3d4f.e34e.7904.aba6.d48e.d34f.eeb0.f526.0e7e.03ce.6fc6.7df0.95b7.e82b.b890.dc95 |
| `0x5578f0` | `0x1578f0` |  | support | ev 705b.51ba.b6e3.de90.d868.1220.1934.cd89.8796.f635.098c.86fc.45fe.aadd.fd87.093f |
| `0x5579a0` | `0x1579a0` |  | support | ev 7823.0faa.d5aa.c5d9.3919.b587.7c8e.9825.8ee2.3d1c.cefa.e855.6aed.94ab.443d.d25c |
| `0x557a53` | `0x157a53` |  | support | ev 8462.a830.9345.14dd.6791.c74b.cf0d.8034.a735.15da.81dc.347e.c6f6.22a3.1338.1238 |
| `0x557a70` | `0x157a70` |  | support | ev b60a.d768.8035.4deb.91c7.ef80.a147.cf78.d56e.165d.31f0.946d.fcc7.6abe.444d.8b85 |
| `0x557af0` | `0x157af0` |  | support | ev eb99.0131.9a63.f1e6.1d2b.b8bd.b16a.162b.3ef3.1f2e.20ee.a381.3745.e5c4.2e81.aae2 |
| `0x557b80` | `0x157b80` |  | support | ev aea2.ae76.c353.d72a.a198.d842.eb9a.ef7b.4353.22c2.c8f3.2482.6258.8dfc.6a7f.b4bf |
| `0x557bd0` | `0x157bd0` |  | support | ev ad06.c92f.d6a2.e9e3.c180.727e.32fa.b323.5caa.1034.5d8a.3ab0.670d.42f4.b8e8.8458 |
| `0x557c85` | `0x157c85` |  | support | ev 2d41.dd77.717c.3a08.8535.4f6e.068c.1458.f669.f012.d96e.8a88.4f4d.fd3a.7ac1.cc15 |
| `0x5598b0` | `0x1598b0` | 0xfc3 / 4035 | 293 | ev 6f2d.0208.a112.8e4e.e270.e840.57f3.d35f.e7f8.c86f.7468.9a5f.86e8.8ebc.af42.042f |
| `0x55a9a0` | `0x15a9a0` | 0x7ea / 2026 | 290 | ev ddf4.92e9.3f61.b9ca.8771.e7f4.c5d7.65f3.24ab.2b0e.4c1a.2c29.f8c9.5bbd.998d.8172 |
| `0x55b1f0` | `0x15b1f0` |  | support | ev a8ea.0283.a60f.8e16.b115.2c45.eb34.7044.a2e0.689d.206b.ff24.8c55.3ea0.62a1.1d88 |
| `0x55b6d0` | `0x15b6d0` | 0x7f3 / 2035 | 291 | ev 2a4a.9653.505c.ecfa.56fb.762d.b6b7.1d87.fa79.a234.0ca6.88f7.13cc.a043.832f.ae3d |
| `0x55b710` | `0x15b710` | 0x7d3 / 2003 | 290 | ev 0969.ac96.a7dd.50df.66f1.07f5.d3ff.ff93.b722.ed52.816c.49dc.8f90.0d0e.dc83.3215 |
| `0x55b780` | `0x15b780` | 0x7d3 row stager | 290 | ev c287.f8aa.ef0b.7e3a.d516.b2d9.36ce.778b.68e5.4c54.6e00.4112.3ef0.8e36.64fe.9587 |
| `0x55b9a0` | `0x15b9a0` | 0x7d5 / 2005 | 290 | ev 6112.88a8.b82f.c248.6702.a330.3a88.d1e6.6887.74bd.5076.1328.dd0a.b343.ca47.c578 |
| `0x55bb40` | `0x15bb40` | 0x7d6 / 2006 | 290 | ev a884.2fc3.b50a.bf4e.5047.4272.1b41.00f9.9175.9d4a.d22b.c0de.db4c.bea7.f5be.a7c2 |
| `0x55bc60` | `0x15bc60` | 0x7d7 / 2007 | 290 | ev f71b.c31f.0571.e74d.f754.9d35.bd40.6319.27e8.607d.5cfa.fd27.90bb.e9ce.ca06.3e82 |
| `0x55bcb0` | `0x15bcb0` | 0x7d8 / 2008 | 290 | ev fffc.d837.63aa.6962.4cdf.ffda.d243.86eb.31b9.42cf.4f75.b968.624f.0237.c8e6.42f9 |
| `0x55bdf0` | `0x15bdf0` | 0x7d9 / 2009 | 290 | ev e766.2b4b.cbd2.e5a3.23d6.350c.5d24.1a8b.3779.b7c1.a3fe.244a.dabc.9ff4.32fa.b6d9 |
| `0x55bec0` | `0x15bec0` | 0x7db / 2011 | 290 | ev 5b92.288a.7a99.143d.df36.d333.8493.3592.80b0.ee59.c59f.b913.73df.d487.4468.6478 |
| `0x55bfa0` | `0x15bfa0` | 0x7dd / 2013 | 290 | ev 7447.43e5.a0e4.fd60.caac.ebe9.313f.7f53.ff54.93c5.8a76.de46.b3fd.026b.7b04.4ed1 |
| `0x55c020` | `0x15c020` | 0x7de / 2014 | 290 | ev 840b.d119.cdcc.b082.6bd1.d2c4.eac9.fcdb.3fd4.f72b.872e.4445.d9f6.abca.9165.9d30 |
| `0x55c190` | `0x15c190` | 0x7e2 / 2018 | 290 | ev 4abe.96c9.9ad4.1dde.2de8.e524.bed3.8c46.f40c.9759.baf4.3149.573c.e44c.d987.b161 |
| `0x55c280` | `0x15c280` | 0x7e3 / 2019 | 290 | ev e462.0242.004d.c9f9.e34b.3eb1.7260.9d27.07d3.de66.d034.8048.b4b2.3d50.95b1.e455 |
| `0x55c600` | `0x15c600` | 0x7e4 / 2020 | 290 | ev deab.f4a7.a427.a95e.d3cf.a244.acda.05ae.5844.7d77.4989.abf7.4eab.d276.5704.52f6 |
| `0x55c940` | `0x15c940` | 0x7e9 / 2025 | 290 | ev 64aa.4e91.2392.67ff.e5d1.da9c.60b9.2d54.ff1d.4f0b.5603.1cdf.aa66.cbfe.a7b4.7dab |
| `0x55ca00` | `0x15ca00` | 0xbb9 / 3001 | 291 | ev ae9d.da90.b2cb.90aa.a413.970b.42ff.cb2e.9c2c.ec0a.6db2.9e69.3ff8.8e47.4770.f5e0 |
| `0x55d420` | `0x15d420` | 0xbbc / 3004 | 291 | ev 2795.4de2.7512.b1c2.0a62.197b.28b1.06d7.efcc.1514.b081.960d.e7f1.c3cc.79a4.d4c6 |
| `0x55d700` | `0x15d700` | 0xbbe / 3006 | 291 | ev bca1.1fba.8a07.5b28.2c5a.71b8.24ac.8890.b544.ee5a.e3e3.995d.cdbd.481f.c260.a91d |
| `0x55e120` | `0x15e120` | 0xbc1 / 3009 | 291 | ev cd1d.330e.3ea0.5f10.2143.e560.ab69.cd51.0017.fead.5590.be6c.9daf.1126.b0f1.0e81 |
| `0x55e2f0` | `0x15e2f0` | 0xbc2 / 3010 | 291 | ev e910.89f5.1d6c.9591.61da.81f0.1b5c.1d6f.51f0.0bfe.10c6.9998.b962.9b65.9941.cce8 |
| `0x55f210` | `0x15f210` | 0xbc6 / 3014 | 291 | ev 5d9c.48f9.d8fe.2788.7b34.e15b.4bc1.1ec3.13a9.527b.93b0.7a15.b70d.b96c.1786.72ee |
| `0x55f3f0` | `0x15f3f0` | 0xfa5 / 4005 | 291 | ev 8bcf.c03d.c639.621f.40e8.21b2.79f7.c903.36de.549c.d160.e173.7815.f7b6.da23.e4a1 |
| `0x55f5b0` | `0x15f5b0` | 0xfa9 / 4009 | 291 | ev 78d2.c75b.cd33.a3f8.5518.bcca.0469.1c3c.5aec.8d70.61fb.8a0f.6b95.35f7.a48f.0265 |
| `0x55f630` | `0x15f630` | 0xfab / 4011 | 291 | ev 5b27.4ae7.5d53.de37.7e24.d69d.1449.e058.83b5.2bdd.fbe1.95dd.e3dd.6081.121c.f25b |
| `0x55f950` | `0x15f950` | 0xfad / 4013 | 291 | ev 0df9.193a.7757.6645.aaaa.d7df.6bac.a9c3.3ad9.76a1.2a89.f5eb.aa1b.0975.6d7e.5c7a |
| `0x55ff00` | `0x15ff00` | 0xfbd / 4029 | 291 | ev ff1a.465e.d578.e050.3c46.f593.d00c.c924.5a08.d844.2d2c.95da.7d8a.7025.9995.ce77 |
| `0x5602a0` | `0x1602a0` | 0xfb8 / 4024 | 291 | ev d5b7.f887.9796.f022.2ab8.cc23.10fc.5071.4cdf.a1d2.dd27.15b6.4d17.1207.d6cc.80ee |
| `0x560600` | `0x160600` | 0xfda / 4058 | 291 | ev 36a5.2203.891f.23ce.beee.65f9.99d4.2a40.0501.2e00.93dd.89bb.e709.ced2.6f0d.1945 |
| `0x560960` | `0x160960` | 0xfba / 4026 | 291 | ev b021.e2b9.b323.9746.52db.6c04.b1d8.2253.9487.409e.465d.42b0.e952.9794.5aea.eab8 |
| `0x560a00` | `0x160a00` | 0xfbc / 4028 | 291 | ev 6b99.7876.86d7.12fe.cede.68fa.4fdb.ffc6.43b7.e76a.5b9b.62f3.ea35.eafd.95a3.a408 |
| `0x560d70` | `0x160d70` | 0x1393 / 5011 | 293 | ev d864.4600.2ebe.d61e.353a.a249.183e.7bc2.b4ce.3755.507e.d1fb.fc2c.ff10.0be7.82d9 |

## Packet staging and shared helpers (15 pins)

### `0x551390` — Clear the 2,000-slot packet pointer table (support)

- **VA / file offset**: `0x551390` / `0x151390`.
- **Behavior**: Checks a sentinel word on the owning object, walks a 2,000-entry pointer array at +0x34220, releases each non-null block through a shared helper and then zeroes the whole 8,000-byte table.
- **Reached from**: Shared cleanup helper; direct call sites 0x4c1730, 0x550950, 0x56ff50, 0x5734e0. Call-graph place: packet/session teardown helpers, not a wire handler.
- **Player-visible reach**: Support. Frees staging storage; emits no pixels and sends no frame.
- **Our client**: No counterpart by VA. Rust `Vec` drop in the socket/session cleanup path supplies the same role; no explicit 2,000-slot table in our client.
- **Evidence**: analysis ev 5d9c.27fd.c70b.7ec4.bfd8.b63b.9402.964f.6cb8.775b.e9be.2cf4.e3ce.85fa.3850.e9b8; dossier ev e754.7bab.b7dd.b94d.52cb.1f67.9555.ec25.954b.9667.0135.5e02.4448.b592.799a.fc5d; incoming xrefs ev 6ef5.5437.0ab2.e39f.09a4.7d8c.6ac9.bb4f.8804.f694.38d5.5da9.6c64.794e.19e3.0083 (direct call sites: 0x4c1730, 0x550950, 0x56ff50, 0x5734e0); caller facet ev 9201.3858.9003.a0b1.7a78.2f44.6e85.8bf0.f354.6228.8828.9faf.22c3.fa79.faa0.cc11; names facet ev 903f.5014.2082.3a15.4949.91b6.e817.85ae.d13d.0b2b.aa08.d3ab.02a9.516e.512f.8cc5; file-offset mapping ev 6d89.800a.3c4a.0082.6252.4b63.7f89.1572.f7a4.f76b.c22c.bae4.1bed.b7cb.19de.610f.

### `0x551850` — Forward one argument to the shared cleanup helper (support)

- **VA / file offset**: `0x551850` / `0x151850`.
- **Behavior**: Single-argument forwarding wrapper: passes its argument to 0x54bce0 and returns without other work.
- **Reached from**: Shared wrapper; direct call sites 0x43af70, 0x43b2b0, 0x43bf30, 0x44e2e0, 0x48a190, 0x5a4b90.
- **Player-visible reach**: Support. Thin teardown/state helper; no player-visible decision in this body.
- **Our client**: No counterpart by VA; state teardown in `ScreenGame`/`OnlineSession` uses Rust drop, not a wrapper table.
- **Evidence**: analysis ev 969f.2c04.6ae0.87b4.13d9.ee4c.a7c8.d92f.f3f4.2a04.f053.f020.c75d.e271.cc54.d457; dossier ev dfa7.cb98.92d7.50bb.5268.de7a.a97d.2db5.9f6a.b966.e55b.2f10.6b03.16f2.7408.653f; incoming xrefs ev 9e88.49dc.0c56.4d23.05b6.b19a.2130.8969.9d89.157e.2c7b.6672.90dc.cc1a.27f1.c8c4 (direct call sites: 0x43af70, 0x43b2b0, 0x43bf30, 0x44e2e0, 0x48a190, 0x5a4b90); caller facet ev ddaa.169e.5f55.2e4b.ed4a.4e9c.0da9.982d.4bdf.b55a.c4bf.ef2d.8911.dd90.fd7a.dfb9; names facet ev f3b7.9db6.db95.db75.24ce.d7e0.d02f.fa65.2a81.882d.33ad.42e2.7857.7241.33a0.0334; file-offset mapping ev a928.7123.f171.a34e.7734.7934.ff78.5b26.a267.6545.8c16.3ba8.68f1.4021.c186.6df3.

### `0x556ed0` — Insert eight-byte records into the packet staging vector (support)

- **VA / file offset**: `0x556ed0` / `0x156ed0`.
- **Behavior**: Inserts a caller-supplied run of eight-byte records at a position. It grows storage when the tail lacks room, moves the old tail, copies the two-word prototype into the gap, and takes one of two cleanup epilogues for the grow or in-place arms.
- **Reached from**: Only direct caller is 0x556b60, the packet-field staging routine; the address-range label is a work bucket, not a wire action.
- **Player-visible reach**: Support for incoming wire rows; it changes staging memory only.
- **Our client**: No counterpart by VA. `Vec::insert` / `Vec::extend_from_slice` in the net codecs supply the same role.
- **Evidence**: analysis ev 80aa.239a.323d.8000.8895.b70b.8862.e72b.1489.b30d.9250.4040.70dc.e75e.99ed.7da7; dossier ev b992.9ecf.f6e0.16a9.e354.3cc6.6091.b61e.eb04.872b.70c4.f7fd.a56a.fc6d.bbc9.d333; incoming xrefs ev 5a7d.bfc0.6472.affb.6fdf.dcbc.b7d5.1228.f002.eec7.b023.df0c.32ad.9c4a.a718.000c (direct call sites: 0x556b60); caller facet ev 88c3.8dea.7ce1.0b5b.ddd2.96a7.ad71.62a6.1943.427e.9b1f.6c83.4334.0315.8c5d.69fb; names facet ev 6587.1be5.d92d.59ba.8e37.8805.2743.f60c.10c3.7003.320f.dbb5.e35c.13c9.5981.bf1c; file-offset mapping ev df4e.f6b3.457b.0696.4395.a6cd.682d.1dfc.82b6.c9b6.1810.a1c8.61b2.0d14.50c8.2e5c.

### `0x5572ef` — Rebalance storage after the grow arm of an eight-byte insert (support)

- **VA / file offset**: `0x5572ef` / `0x1572ef`.
- **Behavior**: Counts the staged eight-byte records, advances a saved end pointer, frees the old allocation after a stride walk, and rewrites begin, end and capacity words.
- **Reached from**: Only direct caller is 0x556ed0 (vector insert grow arm).
- **Player-visible reach**: Support; allocator bookkeeping with no pixels or wire.
- **Our client**: No counterpart by VA; Rust allocation handles this.
- **Evidence**: analysis ev fe11.160a.1a15.7648.ac02.2d50.b3c2.b259.f7ca.859b.0cfc.205b.25d7.631f.a366.14d0; dossier ev fd11.1641.39e3.71e3.5ca0.3231.a2dc.6d57.01eb.e99e.9bda.799d.e53f.2937.1c47.37d1; incoming xrefs ev b7cb.a418.5c4c.7850.177a.a4b4.a105.39aa.4315.573d.b66b.050a.f514.3e41.f820.1c43 (direct call sites: 0x556ed0); caller facet ev 7e6c.2739.0f87.5113.f28f.b245.9271.1fa9.1164.43f8.5a96.3907.3477.928d.4ad0.d61a; names facet ev 343e.da52.c309.7c64.3e62.856a.e7d2.17a2.37a3.da93.a950.0631.6a5e.94f1.7cee.8733; file-offset mapping ev 1dc6.04ce.a024.ade6.5572.8c2a.7743.9b7a.4824.63c1.8a9d.c317.0fac.490d.3753.965e.

### `0x55756d` — Finish the in-place arm of an eight-byte insert (support)

- **VA / file offset**: `0x55756d` / `0x15756d`.
- **Behavior**: Advances the staging vector's end by the inserted count and fills the new range from the two-word prototype before leaving through its exception stub.
- **Reached from**: Only direct caller is 0x556ed0 (vector insert in-place arm).
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec` fill idiom.
- **Evidence**: analysis ev 96ae.e104.1ab5.2fb9.2e6c.e229.7377.7c67.fccf.e514.453c.36e1.b591.7725.8105.c548; dossier ev f268.62f7.cb72.3560.3e1c.e236.8e38.9b8a.e630.bdec.9fb0.3344.23f0.a1ab.7d4c.6f89; incoming xrefs ev 708e.f31f.4134.c185.3a1f.48c7.c475.85c5.1083.0563.9e5d.e13f.3c4e.46b3.6d5b.6a04 (direct call sites: 0x556ed0); caller facet ev 6c03.22c6.4a6a.03ff.d5dd.2399.cac9.5b23.9747.a81f.8b64.8e9f.d418.cda9.9c08.2a1a; names facet ev 0a2f.32de.7011.2083.12fc.484e.3c1f.8b31.79a3.32d3.8b62.b811.88d3.0151.86d9.b7c9; file-offset mapping ev 3605.3132.03e1.69ac.710e.b633.408c.9ff9.4b6f.de1f.e2cd.5db9.9f7b.85f9.2aec.09a6.

### `0x557850` — Copy a forward range of eight-byte records (support)

- **VA / file offset**: `0x557850` / `0x157850`.
- **Behavior**: Copies records from the source range to the destination and returns the destination advanced past the copied run.
- **Reached from**: Shared container helper reached from the eight-byte insert family; no wire handler calls it directly.
- **Player-visible reach**: Support; memory movement only.
- **Our client**: No counterpart by VA; `copy_from_slice` idiom.
- **Evidence**: analysis ev 3d4f.e34e.7904.aba6.d48e.d34f.eeb0.f526.0e7e.03ce.6fc6.7df0.95b7.e82b.b890.dc95; dossier ev d9bd.5cef.a475.2a92.f1af.6b9d.8e75.0346.2ad4.0f05.4a9c.ffa6.9dc1.8e60.bf84.0004; incoming xrefs ev 1942.3edd.678a.715a.2314.4149.cc88.6b50.e97c.2dad.0a4d.7093.f684.6bb5.2f52.1e6d (direct call sites: 0x5569e0, 0x556c80); caller facet ev 3c5e.071a.60ff.3c06.3e55.2f01.921c.4bf4.ea5f.7c2a.c094.15b2.535a.9675.3cbc.9f82; names facet ev 9ebf.463b.d3e8.dce2.a568.43fd.fcf4.ccc2.2282.469a.27fc.39e4.41f3.f3e8.1d70.592b; file-offset mapping ev ac22.850f.9a00.6989.1f2b.5455.31fd.4858.e88e.2be9.7d9a.3823.80e7.d60c.69a8.7e04.

### `0x5578f0` — Forward a three-argument range move to the reverse-copy helper (support)

- **VA / file offset**: `0x5578f0` / `0x1578f0`.
- **Behavior**: Passes all three arguments unchanged to 0x557af0.
- **Reached from**: Only direct caller is 0x556ed0 (vector insert).
- **Player-visible reach**: Support; thin wrapper.
- **Our client**: No counterpart by VA; Rust slice copy handles this.
- **Evidence**: analysis ev 705b.51ba.b6e3.de90.d868.1220.1934.cd89.8796.f635.098c.86fc.45fe.aadd.fd87.093f; dossier ev 2cdd.1a60.6cf4.715a.81e7.e07c.98b8.5d04.e9cb.ef7b.4e95.cb91.3951.81d7.d239.ac8a; incoming xrefs ev 15cb.ca4f.2c23.ec65.69f1.9466.3a0a.55e6.5c6f.12b4.bba9.0bc0.b10f.d45d.e284.4281 (direct call sites: 0x556ed0); caller facet ev 293f.a2fd.1641.9052.b229.dbdc.223e.71a5.0bca.a327.c8ab.4552.1e6e.cd0e.66fd.f515; names facet ev 007f.76fc.c4cf.2884.26e4.c4b5.08d8.7a9d.3302.278a.f188.d2f0.02d7.c427.39d2.1c89; file-offset mapping ev 8e56.ec46.3bae.8aaf.4dc6.1b76.124b.edd9.edbd.a9cc.d1df.3074.b514.5d3c.1685.bbb8.

### `0x5579a0` — Fill a counted run of eight-byte records (support)

- **VA / file offset**: `0x5579a0` / `0x1579a0`.
- **Behavior**: For the requested count it writes one two-word value into successive eight-byte slots, skipping the loop when the destination pointer is null, then runs its exception stub.
- **Reached from**: Only direct caller is 0x556ed0 (vector insert).
- **Player-visible reach**: Support; no pixels or wire.
- **Our client**: No counterpart by VA; Rust `Vec` fill idiom.
- **Evidence**: analysis ev 7823.0faa.d5aa.c5d9.3919.b587.7c8e.9825.8ee2.3d1c.cefa.e855.6aed.94ab.443d.d25c; dossier ev 3294.fe0d.3d7b.b04f.0c15.7ba9.e049.f1a2.8cd0.3388.a87d.b211.126d.e4f3.e16c.3762; incoming xrefs ev e5cb.a0ed.8566.4358.d556.a451.b4cd.1ed5.4ff6.aef4.bdb3.07cf.d8d2.b6c0.c579.a411 (direct call sites: 0x5568e0, 0x556ed0); caller facet ev 6c61.22c8.b7bd.6d1f.268d.8409.5b88.b3f9.7945.ee2a.5641.77a9.eaad.1780.2884.cd6c; names facet ev c512.8458.6594.5fa5.5a14.90e0.acca.ffe7.8ebe.b17f.1370.9f0c.c793.bfb2.09a8.3896; file-offset mapping ev 3956.7a44.a686.cb9e.ae88.c451.23c9.4286.2e53.3565.713e.1ba9.04a0.9aa3.a0db.c9c8.

### `0x557a53` — Exception stub after the eight-byte fill (support)

- **VA / file offset**: `0x557a53` / `0x157a53`.
- **Behavior**: Clears the local unwind marker and restores the saved exception-list head before returning.
- **Reached from**: Only direct caller is 0x5579a0.
- **Player-visible reach**: Support; compiler-generated unwind fragment.
- **Our client**: No counterpart by VA; Rust unwinding is language-level.
- **Evidence**: analysis ev 8462.a830.9345.14dd.6791.c74b.cf0d.8034.a735.15da.81dc.347e.c6f6.22a3.1338.1238; dossier ev 59d9.c9b6.4497.0375.f518.1483.45d5.e3d8.f87f.fb09.b99d.08b0.d3cf.37f6.51c9.8db6; incoming xrefs ev 42d0.b030.f626.59e9.9f88.661a.58c9.13e0.cf2e.608a.e070.6da5.6da3.81d3.8db0.7e2c (direct call sites: 0x5579a0); caller facet ev 75df.89fb.7b74.87ec.5543.6182.b214.78ec.c783.5c41.7668.3530.a74b.2444.470a.be6e; names facet ev 863f.fd00.37b4.2dc7.6b3f.1259.2852.0afa.8657.bb8d.bff0.9320.f3b2.1c9a.6e5f.b63a; file-offset mapping ev 7528.061f.1550.fa8a.67c2.7ae9.d6e7.3840.d815.cafe.7d2d.df60.6a61.2edb.d144.6d67.

### `0x557a70` — Forward a bounded four-argument range copy (support)

- **VA / file offset**: `0x557a70` / `0x157a70`.
- **Behavior**: Passes four arguments unchanged to 0x557bd0.
- **Reached from**: Direct callers 0x556ed0 and 0x557bd0's own cluster; called from the vector insert grow arm.
- **Player-visible reach**: Support; thin wrapper.
- **Our client**: No counterpart by VA; Rust slice copy handles this.
- **Evidence**: analysis ev b60a.d768.8035.4deb.91c7.ef80.a147.cf78.d56e.165d.31f0.946d.fcc7.6abe.444d.8b85; dossier ev b02b.dfd5.520b.32f6.6275.28c7.a312.2842.d904.b866.bc08.0f29.43dd.cb88.a7e4.698b; incoming xrefs ev 36ae.083b.4a96.296f.8af0.8ec3.d197.9350.076f.ce55.9082.a709.0cb1.b039.d752.7d73 (direct call sites: 0x556ed0); caller facet ev 5ce5.deae.f9c6.6da5.5064.68fc.bded.140a.ff67.a7e7.f580.1f23.c418.f8cc.5f41.f267; names facet ev 4839.843f.408d.9c96.bccd.2992.7c79.3b5f.6c8e.88a1.f8ed.fdfd.f80e.f87c.fda1.1469; file-offset mapping ev c223.24b9.f831.b826.acac.96c7.10d0.ec92.e5f9.3c51.9bdb.c441.dc4b.474d.9480.a623.

### `0x557af0` — Copy eight-byte records backwards (support)

- **VA / file offset**: `0x557af0` / `0x157af0`.
- **Behavior**: Walks both pointers backwards from the supplied ends and copies one two-word record per step, returning the destination stepped back past the moved run.
- **Reached from**: Only direct caller is 0x5578f0; reached from vector insert.
- **Player-visible reach**: Support; memory movement only.
- **Our client**: No counterpart by VA; Rust slice copy handles this.
- **Evidence**: analysis ev eb99.0131.9a63.f1e6.1d2b.b8bd.b16a.162b.3ef3.1f2e.20ee.a381.3745.e5c4.2e81.aae2; dossier ev 0ce9.a3f2.5d18.5237.68b6.5ca9.5bba.8384.5bfa.08af.78f0.72ce.d242.392e.f94d.b565; incoming xrefs ev b91e.4c07.8709.8971.eb7d.2322.86e8.6674.a306.e4a6.99d5.0a12.4e85.f25b.f901.8247 (direct call sites: 0x5578f0); caller facet ev aa4d.464c.3989.e852.9eba.0208.daf3.91ad.6270.b0d3.9e97.0d5f.2700.c3af.f17d.9827; names facet ev fa01.fe7e.3756.d01e.7c9f.1220.7866.68a2.f406.1530.79c2.6969.ec10.fbbe.0b06.4bdb; file-offset mapping ev a3ea.1093.391f.a320.2486.f03e.310d.1f86.7b78.3029.b02a.7234.8c73.d541.5b44.50f3.

### `0x557b80` — Forward another bounded four-argument range copy (support)

- **VA / file offset**: `0x557b80` / `0x157b80`.
- **Behavior**: Passes four arguments unchanged to 0x557bd0.
- **Reached from**: Only direct caller is 0x556ed0.
- **Player-visible reach**: Support; thin wrapper.
- **Our client**: No counterpart by VA; Rust slice copy handles this.
- **Evidence**: analysis ev aea2.ae76.c353.d72a.a198.d842.eb9a.ef7b.4353.22c2.c8f3.2482.6258.8dfc.6a7f.b4bf; dossier ev 705f.f836.8352.b4e7.ba4c.0d0e.28b9.6298.b9d9.28b3.d741.8376.796a.9021.1200.0d54; incoming xrefs ev fd7b.ad14.2fed.da77.cd62.6729.6ae4.d6f4.22cb.6c28.25b5.5173.d685.57c5.a9b2.1646 (direct call sites: 0x556ed0); caller facet ev ca9d.d112.3512.34b0.9808.2b23.1811.eccd.b846.22ab.d35f.a161.3dd7.ac0e.a7c0.04e2; names facet ev fe21.f38f.3ab9.de27.2801.183c.4cd8.2c3c.1c7e.2832.75c1.b1b5.24fa.20d1.1149.7bc2; file-offset mapping ev 6a3c.8f0c.742d.de60.0bad.a347.c731.80df.73ba.3e64.93c1.09d2.519a.c7ca.f5cb.13b3.

### `0x557bd0` — Copy an eight-byte range with a null-destination guard (support)

- **VA / file offset**: `0x557bd0` / `0x157bd0`.
- **Behavior**: Copies one two-word record per slot from source to destination while guarding against a null destination, then leaves through its exception stub.
- **Reached from**: Direct callers 0x557a70 and 0x557b80, both from vector insert.
- **Player-visible reach**: Support; memory movement only.
- **Our client**: No counterpart by VA; Rust slice copy handles this.
- **Evidence**: analysis ev ad06.c92f.d6a2.e9e3.c180.727e.32fa.b323.5caa.1034.5d8a.3ab0.670d.42f4.b8e8.8458; dossier ev 3255.2254.3fd2.a320.8964.c864.9718.c321.85da.42cd.cdbf.dce6.c0e4.7250.b7d7.e7bc; incoming xrefs ev 05b0.f31b.ffc7.e08f.3663.50dc.068d.89a1.b1d1.43c5.4870.0979.965f.8feb.2e6a.b0ff (direct call sites: 0x557a70, 0x557b80); caller facet ev 4837.5d25.0ae9.9bf4.6834.982f.4b54.c22c.0ef8.c042.9624.a5b5.093c.9e97.90f9.13c4; names facet ev 80c4.37ed.af01.3d1f.5163.97c0.a457.3499.1793.768d.adc8.8052.e8b7.2bba.4ed8.103c; file-offset mapping ev b58b.e910.67d8.5c39.670e.7879.76e7.be15.7d5c.cf5f.4b19.250a.2838.3d6a.89d3.abfe.

### `0x557c85` — Exception stub after the eight-byte range copy (support)

- **VA / file offset**: `0x557c85` / `0x157c85`.
- **Behavior**: Clears the local unwind marker, restores the exception-list head, and returns the saved value.
- **Reached from**: Only direct caller is 0x557bd0.
- **Player-visible reach**: Support; compiler-generated unwind fragment.
- **Our client**: No counterpart by VA; Rust unwinding is language-level.
- **Evidence**: analysis ev 2d41.dd77.717c.3a08.8535.4f6e.068c.1458.f669.f012.d96e.8a88.4f4d.fd3a.7ac1.cc15; dossier ev 9b73.6bae.aa13.5018.5f9e.8e3a.92ca.2ffa.0215.20dd.68a5.d598.7f9b.ebdc.4816.c88e; incoming xrefs ev 827e.e3fb.1737.97c2.d08c.79b0.ec12.6fbb.f35c.0725.00ab.65ff.8ead.ca65.e0f5.8e43 (direct call sites: 0x557bd0); caller facet ev 4892.5274.a84d.f420.1849.a2e1.eb4b.1ad4.69c1.4caf.a2da.d47c.8e58.8919.c384.ecfa; names facet ev ee30.7b4d.e7af.0aad.7796.9075.6bde.480b.7a2f.bcad.c727.090b.4c03.4cdb.aced.ef98; file-offset mapping ev b9f1.3eb6.ba0d.444b.302e.8b55.b84b.2705.bda1.d00d.a67e.fadb.29fd.0f71.e3fa.1193.

### `0x55b1f0` — Reset a twelve-byte staging record (support)

- **VA / file offset**: `0x55b1f0` / `0x15b1f0`.
- **Behavior**: Zeroes twelve bytes of the record and sets bytes +4 through +8 to 0xff.
- **Reached from**: Direct callers 0x55a9a0 (2026) and 0x560d70 (5011); shared record initializer.
- **Player-visible reach**: Support: prepares the row records consumed by those two handlers; no decision here.
- **Our client**: No counterpart by VA; Rust struct initialization supplies the role.
- **Evidence**: analysis ev a8ea.0283.a60f.8e16.b115.2c45.eb34.7044.a2e0.689d.206b.ff24.8c55.3ea0.62a1.1d88; dossier ev 17c7.c550.0725.aa72.3af9.5604.21ad.38a9.e6cc.36ee.8ec2.b92e.dcef.a283.9f74.c38a; incoming xrefs ev 6e82.9f86.e80e.c5ac.d06c.e244.9c79.3a3b.1e7c.9b98.553d.daea.b2e3.af6a.2a11.4967 (direct call sites: 0x55a9a0, 0x560d70); caller facet ev 53f3.d12c.9edf.8a3c.c820.efde.c376.374c.e402.2e63.a819.fe08.a8dd.0d89.cf91.4ed3; names facet ev 8d03.08a9.e0d3.9661.26c3.3873.566e.9fa4.00e9.442c.ed56.2830.851d.2a11.06f0.602b; file-offset mapping ev d9ce.9773.e060.0926.1c0d.5534.78e8.d9ed.af65.3bd8.ddb0.8554.3869.af58.661c.b2bd.

## Wire handlers — Room (#290) (15 pins)

### `0x55a9a0` — 2026 MusicPremiumTimeList: rebuild premium rows and channel description lines

- **VA / file offset**: `0x55a9a0` / `0x15a9a0`.
- **Behavior**: Reads a 32-bit row count. When the user-list vector is populated it clears it and stages the count, then reads a 16-bit song id plus five status bytes per row. It formats several description lines from a global mode bitmask and a premium flag, posts them through the text-queue helper with ink 2, and finishes with the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7ea (2026) in dispatcher 0x558080. Also calls the shared record reset 0x55b1f0.
- **Player-visible reach**: Player-visible: Channel/room description lines and premium-time rows.
- **Our client**: No counterpart by VA or opcode. 2026 falls through the `Generic::from_id` no-op arms (`crates/recomp2jam/src/session.rs:3528`, `:3974`); channel text in our client comes from the staged channel state, not this packet.
- **Evidence**: analysis ev ddf4.92e9.3f61.b9ca.8771.e7f4.c5d7.65f3.24ab.2b0e.4c1a.2c29.f8c9.5bbd.998d.8172; dossier ev e49b.3ba3.a0fd.ba45.deba.3f4f.3441.5e9e.96c2.bc7c.5c2c.16c6.ed24.943f.0c5c.3271; incoming xrefs ev bf0b.4afa.7712.5055.481c.e606.25c1.7824.a678.deee.817f.8f4f.bc2d.2142.3cf1.a5ae (direct call sites: 0x558080); caller facet ev 27f9.fb06.ba7d.85cb.e719.6e57.55bb.5929.16c3.2fbc.6fea.fe2d.b716.607b.e6c3.4e58; names facet ev f23f.f397.005a.93d0.0605.394f.09d2.73b0.c20b.87e6.1083.1e7c.6a37.57e7.7b66.1ca7; file-offset mapping ev 15d7.a731.bd3f.580b.ff80.553e.0cfc.ca49.5ef6.d664.7fac.1f8f.cae0.41f5.3f3b.794c.

### `0x55b710` — 2003 RoomList: stage each row and refresh the room board

- **VA / file offset**: `0x55b710` / `0x15b710`.
- **Behavior**: Reads the row count into a global, calls the per-row stager once for each entry, then posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7d3 (2003) in dispatcher 0x558080; per-row stager is 0x55b780.
- **Player-visible reach**: Player-visible: the Room list rows and their refresh.
- **Our client**: Yes. `apply_room_list` (`crates/recomp2jam/src/session.rs:4077`) decodes and calls `ScreenGame::set_rooms` (`crates/o2jam-shell/src/screens/game/mod.rs:2545`); guard `room_list_paints_rows_and_cursor` (`mod.rs:15390`, Tier 0).
- **Evidence**: analysis ev 0969.ac96.a7dd.50df.66f1.07f5.d3ff.ff93.b722.ed52.816c.49dc.8f90.0d0e.dc83.3215; dossier ev 1d21.9ba9.2b21.3cb0.9bae.1f7c.a7d9.7fbb.96b2.6e8a.3a16.f739.533d.97ea.b4be.bb75; incoming xrefs ev 9bd7.8e79.b7cd.98d3.d155.a8e4.3546.de8d.3787.c070.2df4.76b4.f04a.fc2f.85bc.dc5b (direct call sites: 0x558080); caller facet ev 30b0.1654.4cf7.c805.9c46.7e63.5ee4.0d91.b512.26a7.fb23.81c5.0583.306d.a257.e95d; names facet ev d2e7.127e.b5a6.8b23.a7d6.1678.23a4.2275.51b6.7d97.1849.a266.7f0b.807c.7207.622b; file-offset mapping ev 1278.ff1c.2f31.fd93.8f92.89ee.a03c.23a4.0f1c.4715.2712.7b8e.ac2f.691f.bbcc.0347.

### `0x55b780` — Stage one 2003 room row into the 0xb4-byte record

- **VA / file offset**: `0x55b780` / `0x15b780`.
- **Behavior**: Clears the record at the supplied index, reads a 32-bit room number, a null-terminated name, a byte, a 16-bit flag word and ten trailing bytes. It splits the flag word into a low 12-bit part plus a shifted high part, then reads a counted list of skill ids and two trailing fields.
- **Reached from**: Called only from 0x55b710 once per 2003 row; call-graph place: dispatcher → 2003 handler → row stager.
- **Player-visible reach**: Player-visible support: builds the row that the 2003 UI message paints.
- **Our client**: No VA counterpart. `RoomListEntry` decode at `crates/o2jam-net/src/rooms/list.rs:185` and staging at `set_rooms` (`mod.rs:2545`) fill the same fields with Rust ownership.
- **Evidence**: analysis ev c287.f8aa.ef0b.7e3a.d516.b2d9.36ce.778b.68e5.4c54.6e00.4112.3ef0.8e36.64fe.9587; dossier ev ad39.d863.97fe.93a8.60d0.f754.f3c9.8788.4780.6d39.2bd3.9233.108e.2bee.5c87.f8ba; incoming xrefs ev c953.f445.4208.1b1e.1f67.8355.c257.c219.92d9.7b38.91da.8d5a.18d3.10b1.a591.6006 (direct call sites: 0x55b710); caller facet ev acfa.a9a0.dfdc.e628.bf95.ae9a.29f7.741d.3b0a.c0fd.5523.d8b8.d70e.ac19.ca22.f207; names facet ev 7fd3.750c.a9f9.c537.2a0f.15b3.f4b6.55ab.0a1a.4316.e92a.f5c8.12fa.a1c1.6cee.993a; file-offset mapping ev 7358.4c39.078e.9e98.6708.7391.015e.e684.a091.3098.f5f6.49ad.f085.bc91.88e2.adcc.

### `0x55b9a0` — 2005 RoomCreated: insert a new room row

- **VA / file offset**: `0x55b9a0` / `0x15b9a0`.
- **Behavior**: Reads a room number, bounds it against the staged count, clears that record, reads identity and limit fields into it, sets the exe's default state words, and posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7d5 (2005) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: a newly created room appears in the Room list.
- **Our client**: Yes. `apply_room_created` (`crates/recomp2jam/src/session.rs:4064`) calls `ScreenGame::upsert_room` (`mod.rs:2638`); guard `room_created_maps_the_wire_descriptor_to_a_listed_row` (`session.rs:7984`, Tier 0), with live chain in `:8018`.
- **Evidence**: analysis ev 6112.88a8.b82f.c248.6702.a330.3a88.d1e6.6887.74bd.5076.1328.dd0a.b343.ca47.c578; dossier ev 946b.8c5c.364d.21db.693c.347e.9310.7fe5.7c0f.ac7d.bbde.aeab.2faf.2f92.3bb3.7448; incoming xrefs ev 580c.707d.fa59.c9d3.18af.d436.5bcf.3067.eb64.604e.8bfd.aec8.d85b.fbdf.966d.42b3 (direct call sites: 0x558080); caller facet ev ae50.63e8.2a79.6332.7f38.9652.70fc.5356.b6ea.588f.27fb.8cd7.e497.1824.b797.e0ba; names facet ev 3ba1.ba71.23e0.d622.17f1.1dbe.7208.202e.33bf.4ec8.639f.227b.04c9.8e94.ca94.714a; file-offset mapping ev e8fc.c2f2.857f.1efa.fc57.6b25.0949.c870.137c.6a2d.c3c2.551c.8671.e3fc.38f8.db34.

### `0x55bb40` — 2006 CreateRoom answer: reset Waiting state on success

- **VA / file offset**: `0x55bb40` / `0x15bb40`.
- **Behavior**: Reads a 32-bit result word. On zero it stages room identity fields, sets a cluster of room/mode flags, switches the UI state to 5 and clears or arms eight seat records before returning.
- **Reached from**: Wire handler: opcode 0x7d6 (2006) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: create success lands in Waiting with fresh seat chrome.
- **Our client**: Partial. The create flow decodes `CreateRoomResponse` at `crates/recomp2jam/src/session.rs:933` and stages a joined room; there is no separate 2006 reply handler with the exe's seat reset.
- **Evidence**: analysis ev a884.2fc3.b50a.bf4e.5047.4272.1b41.00f9.9175.9d4a.d22b.c0de.db4c.bea7.f5be.a7c2; dossier ev 1392.efdb.d352.d590.1595.4d8b.b384.48b6.5d3b.a129.f65a.893b.8f83.693d.f94a.b9c9; incoming xrefs ev 2d4e.6025.98bc.3780.3003.092f.d132.c54b.bf03.90ba.1e04.413c.0cd8.3047.3542.a2c3 (direct call sites: 0x558080); caller facet ev 4895.d756.9d9f.c83e.96f8.2679.d867.ea97.e802.5c24.274a.23fc.c659.51f1.11e9.7365; names facet ev 1c2c.8184.bcc4.ee04.7a1f.a817.7ba4.e9db.41b8.62ac.e71d.79c2.780d.998c.887d.fe59; file-offset mapping ev 986c.ab1c.800d.20e0.17aa.681a.b2a8.c07d.ff4d.3a83.1d2e.3533.449a.a263.556e.3325.

### `0x55bc60` — 2007 RoomRemoved: clear one room row

- **VA / file offset**: `0x55bc60` / `0x15bc60`.
- **Behavior**: Reads a 32-bit room number and, when it is within the staged count, clears that 0xb4-byte record and posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7d7 (2007) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: the disbanded room disappears from the Room list.
- **Our client**: Yes. `apply_room_removed` (`crates/recomp2jam/src/session.rs:4089`) calls `ScreenGame::remove_room` (`mod.rs:2662`); codec guard `room_removed_decodes_the_live_disband_broadcast` (`crates/o2jam-net/src/message.rs:1833`, Tier 0). Live observer leg remains a separate row.
- **Evidence**: analysis ev f71b.c31f.0571.e74d.f754.9d35.bd40.6319.27e8.607d.5cfa.fd27.90bb.e9ce.ca06.3e82; dossier ev 4ac0.bc5c.3a52.d1f2.b484.ef9e.c637.13cf.edd8.c2cc.9f8c.6008.19c9.1723.9236.7ead; incoming xrefs ev fca2.fb34.c34d.fcc8.3d62.907a.68dc.0bc8.da0a.848b.a014.434d.c8d6.0c17.d586.db10 (direct call sites: 0x558080); caller facet ev 01ee.25a3.faa2.9368.8e92.5dd5.2035.ef64.10ef.e7bf.caf5.b75a.db1e.b952.2dd7.ec4d; names facet ev 80c1.1175.467d.c46c.069b.cc11.efde.271b.959b.5f5b.afa2.a3df.ba05.a406.f5c4.375b; file-offset mapping ev f844.5652.fca8.fe3f.6481.4ee4.b2df.f933.cbc3.b668.02d7.305a.e382.bdd5.c222.5cfb.

### `0x55bcb0` — 2008 RoomParamsChanged: rewrite room descriptor fields

- **VA / file offset**: `0x55bcb0` / `0x15bcb0`.
- **Behavior**: Reads an index and, when it is in range, replaces two name strings, several descriptor bytes and a 32-bit word in the staged 0xb4-byte row, then posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7d8 (2008) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: Room row title, limits and descriptor badges update in place.
- **Our client**: No counterpart. 2008 is still `OPEN: body layout of RoomParamsChanged` in `docs/dispatch-inventory.md:52`; no codec or apply arm exists.
- **Evidence**: analysis ev fffc.d837.63aa.6962.4cdf.ffda.d243.86eb.31b9.42cf.4f75.b968.624f.0237.c8e6.42f9; dossier ev 7a59.55c9.d178.b21c.10a1.a8fe.4450.77a6.8799.792f.41dd.2322.1471.5142.eb7e.8e47; incoming xrefs ev 9908.0127.4490.0922.b50b.d152.7134.1849.5bf4.0405.12aa.0ada.6066.1261.70da.82e4 (direct call sites: 0x558080); caller facet ev 3e6f.3dec.ca07.39d0.a5b0.9faa.a135.6055.459e.8860.33dc.629f.e361.2946.f3bd.03c4; names facet ev 58e0.49c2.0a48.f45e.abad.bf60.fb3f.8d19.acfe.9b31.29f9.42e6.521b.3be3.cd55.871c; file-offset mapping ev fe98.a538.25a0.cba1.9449.cc9d.9d3d.e68c.833d.e2c7.8b30.31c3.11f0.7687.7521.9645.

### `0x55bdf0` — 2009 RoomUserCountChanged: rewrite occupancy fields

- **VA / file offset**: `0x55bdf0` / `0x15bdf0`.
- **Behavior**: Reads an index and, when in range, replaces two bytes, a 16-bit word and one byte in the staged room row, then posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7d9 (2009) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: the room row's occupancy/count readout changes.
- **Our client**: No counterpart. `crates/recomp2jam/src/session.rs:3971` recognizes the opcode but explicitly leaves the row count unchanged; no observer arm is staged.
- **Evidence**: analysis ev e766.2b4b.cbd2.e5a3.23d6.350c.5d24.1a8b.3779.b7c1.a3fe.244a.dabc.9ff4.32fa.b6d9; dossier ev 263b.49c5.2690.c9bc.a90d.ab0d.9ee7.ba2d.d544.1ad9.0063.e049.8afd.db95.05d6.ab80; incoming xrefs ev fbc9.dbdc.05d1.e4a1.18f9.30e8.642c.fe00.30f4.6cc3.ff79.7d7b.6678.3475.5973.da19 (direct call sites: 0x558080); caller facet ev 497e.7c89.b51c.80e8.c702.6649.5d9d.544a.385e.4743.f0ca.a07d.45aa.175e.d242.0ef1; names facet ev ba61.803d.01db.5b5d.0302.40a2.221a.7436.4d7d.7914.e95b.a9cb.e19e.a3b9.0334.a707; file-offset mapping ev 105c.62f4.3d18.ef46.0230.02a0.211d.403f.e668.dc13.f369.98ba.8dc2.1537.2c47.acb3.

### `0x55bec0` — 2011 GetUserList: stage a counted user-list sheet

- **VA / file offset**: `0x55bec0` / `0x15bec0`.
- **Behavior**: Clears the sheet, reads a 32-bit row count and calls a sizing helper. For each row it reads an id, two null-terminated strings and a 32-bit field, stages the record through a shared helper, then posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7db (2011) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: the web/user-list sheet contents and gift flag source.
- **Our client**: No counterpart. 2011 is `OPEN: wire body` in `docs/dispatch-inventory.md:55`; the shell deliberately leaves the desc well empty without a web source (guard `room_user_panel_leaves_the_desc_well_empty_without_a_web_source`, `crates/o2jam-shell/src/screens/paint.rs:11999`, Tier 0).
- **Evidence**: analysis ev 5b92.288a.7a99.143d.df36.d333.8493.3592.80b0.ee59.c59f.b913.73df.d487.4468.6478; dossier ev c6ed.dfc6.3ab7.e0ab.8216.4a54.37c1.3c9a.7987.52e7.d635.d846.1c6b.eea1.2467.8877; incoming xrefs ev 667a.9dc3.b047.3d51.db62.2af1.1dc3.1850.80e5.2424.49cf.d072.308e.4fd1.e195.2dc8 (direct call sites: 0x558080); caller facet ev 70d9.4e35.87ed.0292.8073.0e5d.4466.8d1b.d35e.8b70.e0b7.52f6.a9ae.bd83.e22c.8829; names facet ev 9574.bdca.2b45.3eeb.4dbf.c605.5523.bbbe.b36a.cd01.3be7.7359.e616.f3c7.456d.6525; file-offset mapping ev 7315.97c9.96e1.707b.d563.026c.a9df.e533.e2fd.fee1.a9e5.35d1.9553.dffa.5bf9.9a4c.

### `0x55bfa0` — 2013 MainRoomUserMessage: append a player chat line

- **VA / file offset**: `0x55bfa0` / `0x15bfa0`.
- **Behavior**: Reads two null-terminated strings through the string reader, formats them as a fixed-width sender plus message line, and feeds the result to the ControlList parent through the text queue.
- **Reached from**: Wire handler: opcode 0x7dd (2013) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: channel chat row from another player.
- **Our client**: Yes. Codec `MainRoomUserMessage` (`crates/o2jam-net/src/rooms/chat.rs:95`); `apply_lobby_user_message` (`crates/recomp2jam/src/session.rs:3998`) → `ChatState::push_lobby_user` (`chat.rs:422`); guard `pushing_a_user_message_stages_one_chat_row` (`crates/o2jam-shell/src/screens/game/mod.rs:34740`, Tier 0).
- **Evidence**: analysis ev 7447.43e5.a0e4.fd60.caac.ebe9.313f.7f53.ff54.93c5.8a76.de46.b3fd.026b.7b04.4ed1; dossier ev 5bdd.0420.b6ae.ba56.c8cf.383e.db8c.457f.2108.2746.0edc.6f94.51e7.bccf.852b.839d; incoming xrefs ev 954d.4d69.c034.a78b.7b34.bc42.a9c8.6f26.87f7.2fb9.fd65.1976.bed3.e260.1312.66ca (direct call sites: 0x558080); caller facet ev d4e9.50f6.799b.62d4.6dc8.f747.480f.7d60.1a25.ed26.e0b5.9f93.e5f0.6156.7c27.8151; names facet ev 5259.d8dc.e036.eda9.3d38.470b.5c3a.cb87.e255.a966.23b0.d3a5.381b.7dd4.7378.dd97; file-offset mapping ev b2ab.decf.2af1.6aa3.9f65.fc10.c55f.676f.f5a6.6f87.6ce5.e654.3c5a.eb79.911e.4627.

### `0x55c020` — 2014 MainRoomAdminMessage: append an admin chat line

- **VA / file offset**: `0x55c020` / `0x15c020`.
- **Behavior**: Same two-string read and fixed-width line format as the 2013 handler, but feeds the admin variant of the text queue to the same ControlList parent.
- **Reached from**: Wire handler: opcode 0x7de (2014) in dispatcher 0x558080; also used as the fan-out id for announcements and whispers.
- **Player-visible reach**: Player-visible: channel chat row from an admin.
- **Our client**: Yes. Codec `MainRoomAdminMessage` (`crates/o2jam-net/src/rooms/chat.rs:143`); `apply_lobby_admin_message` (`crates/recomp2jam/src/session.rs:4008`) → `ChatState::push_lobby_admin` (`chat.rs:428`).
- **Evidence**: analysis ev 840b.d119.cdcc.b082.6bd1.d2c4.eac9.fcdb.3fd4.f72b.872e.4445.d9f6.abca.9165.9d30; dossier ev 5999.11bd.3bd8.8ada.d656.6e00.220f.ae7f.d03a.28b5.049a.522c.5d0d.7958.0bdc.c80e; incoming xrefs ev d2d3.88d1.ad83.b3ac.33e2.1fe6.ee94.5f34.072e.a59e.a8b1.d707.1b3c.f7fc.6a51.fc7f (direct call sites: 0x558080); caller facet ev 8474.2699.dddb.7332.4fa9.e8bc.39b6.af02.b1b8.c568.133d.b6d4.6a39.22ed.09ef.d298; names facet ev e83a.5597.305d.2725.56fe.d5de.9a7a.ba98.82c0.4f34.8678.f0d3.a31c.34dc.8e82.ec06; file-offset mapping ev f874.049a.0029.a990.fb4c.19f5.e397.9c00.1373.ab66.0ce9.84a5.fcc6.95c8.ed18.4c85.

### `0x55c190` — 2018 SendWhisper answer: show delivery report or failure name

- **VA / file offset**: `0x55c190` / `0x15c190`.
- **Behavior**: Reads a 32-bit result word. The zero branch reads a sender string and posts a `<<` delivery line with a fixed-width sender and the local name; the other branch reads a name and posts a failure line to the ControlList parent with zero ink.
- **Reached from**: Wire handler: opcode 0x7e2 (2018) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: whisper send acknowledgement or refusal line.
- **Our client**: Partial. Codec `SendWhisperResponse` (`crates/o2jam-net/src/rooms/chat.rs:231`) exists, but the lobby drain treats 2018 as a no-op (`crates/recomp2jam/src/session.rs:3974`); no `<<` line is staged.
- **Evidence**: analysis ev 4abe.96c9.9ad4.1dde.2de8.e524.bed3.8c46.f40c.9759.baf4.3149.573c.e44c.d987.b161; dossier ev 6f96.1f6a.0178.a973.e0eb.0d84.c452.e98e.fb6d.caa2.4d58.5f57.c5a9.42aa.ac7b.2dd6; incoming xrefs ev 41bc.c4cf.180c.9c03.49f4.67ac.cf0b.9fd5.a3a5.b16c.34d1.e739.104e.000d.fce0.3274 (direct call sites: 0x558080); caller facet ev ee91.e90a.4cf0.c921.95a2.7ab4.f4d3.802d.b87e.7990.bdba.32cd.37df.1aaa.580e.5c26; names facet ev 2870.afd3.81db.3aa1.0f44.aaa0.6410.097a.7b85.9cc2.004b.4c32.ad0a.cbe9.741b.afde; file-offset mapping ev 789e.1384.0366.8906.e62c.fce8.dbc6.f355.4e4b.8c9c.4dad.4f38.b160.3f13.6264.9997.

### `0x55c280` — 2019 ReceiveWhisper: look up sender and post an incoming whisper line

- **VA / file offset**: `0x55c280` / `0x15c280`.
- **Behavior**: Reads two strings, searches a 0x10c-stride friend record vector for the sender, obtains the display name, formats a `>>` line and posts it through the text queue with the opcode id.
- **Reached from**: Wire handler: opcode 0x7e3 (2019) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: incoming whisper row and sender name.
- **Our client**: Partial. Codec `ReceiveWhisper` (`crates/o2jam-net/src/rooms/chat.rs:290`); `apply_lobby_whisper` (`crates/recomp2jam/src/session.rs:4021`) → `ChatState::push_lobby_whisper` (`chat.rs:437`); guard `pushing_a_channel_whisper_keeps_the_recipient` (`mod.rs:34793`, Tier 0). Friend-name lookup has no counterpart.
- **Evidence**: analysis ev e462.0242.004d.c9f9.e34b.3eb1.7260.9d27.07d3.de66.d034.8048.b4b2.3d50.95b1.e455; dossier ev b74f.a783.11b4.885b.864c.19c0.30d9.c2fe.8330.4f54.4fc8.7572.bbd1.6245.b620.3571; incoming xrefs ev 665f.9844.6e74.b4fd.3625.27e3.0bb6.44fb.4f28.5160.5f2b.b348.8bef.6a5d.ebc6.d6a7 (direct call sites: 0x558080); caller facet ev 5ec3.c669.c5cc.f3fb.09d8.d363.a3c6.d23d.1825.cff5.afb9.aec9.713b.52d7.7d72.1c83; names facet ev 0428.274a.6845.5466.623e.1328.ad64.a06b.9b63.c680.fa53.25e9.2e89.8e66.c878.f497; file-offset mapping ev 5ab7.721d.f9c4.5d91.589b.3f41.2812.d1b1.5c30.16e6.d55a.8f8a.a869.f7ca.c7de.1052.

### `0x55c600` — 2020 RoomStateChanged: update one room state byte

- **VA / file offset**: `0x55c600` / `0x15c600`.
- **Behavior**: Reads an index and, when in range, writes one state byte into the staged room record and posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7e4 (2020) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: the room's state marker in a Room row.
- **Our client**: No counterpart. 2020 is `OPEN: body layout of RoomStateChanged` in `docs/dispatch-inventory.md:64`; no codec or apply arm exists.
- **Evidence**: analysis ev deab.f4a7.a427.a95e.d3cf.a244.acda.05ae.5844.7d77.4989.abf7.4eab.d276.5704.52f6; dossier ev 1798.c501.6d1f.fe0d.d891.c2bc.41f1.0412.23a0.ce5c.327f.d05d.e273.a1ba.ee54.e8c2; incoming xrefs ev 6485.5be8.3ea7.e2e0.0277.05f1.ed15.f2a5.f275.ed28.323c.3f1b.7e6a.bb48.b1c6.a2bc (direct call sites: 0x558080); caller facet ev ee8e.6b91.1b32.3b53.ac10.aaf0.f057.2f60.bf17.b07b.bd6f.7b9c.4ec8.0568.e7e5.39e9; names facet ev bc44.84c4.1e85.c2eb.ac78.20d2.ead8.8851.e758.89e1.b4c8.7c3b.2d64.b3b7.150c.21bc; file-offset mapping ev 4350.ccc5.017a.f933.9005.c667.99a4.45a8.c4b7.73cc.2d17.dc4f.5d42.611c.2461.4f32.

### `0x55c940` — 2025 RoomSkillChanged: replace a room row's skill list

- **VA / file offset**: `0x55c940` / `0x15c940`.
- **Behavior**: Reads an index and a count, clears the row's skill list, reads that many 32-bit skill ids into the shared skill staging helper, then posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0x7e9 (2025) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: skill badges shown on the Room row.
- **Our client**: No counterpart. 2025 is `OPEN: body layout of RoomSkillChanged` in `docs/dispatch-inventory.md:69`; our rows stage skills from local state only.
- **Evidence**: analysis ev 64aa.4e91.2392.67ff.e5d1.da9c.60b9.2d54.ff1d.4f0b.5603.1cdf.aa66.cbfe.a7b4.7dab; dossier ev 3755.62fa.b810.496b.5b04.0ab3.a295.7b03.7085.3428.66b9.303c.f916.6b6e.88e7.d728; incoming xrefs ev 309e.0b0f.9eae.0b3f.53a4.118b.0d69.c963.148c.90c1.def2.1534.c492.d31b.f107.1917 (direct call sites: 0x558080); caller facet ev 006b.5ba7.abfd.fafc.a801.3e15.cf0a.9473.2bad.1fd6.f4f0.49c0.0ef5.0257.3085.5201; names facet ev 8f75.ad43.2b73.6f5a.75f7.120b.9f9d.33b9.aed7.485f.78b3.72be.b233.30ab.168e.1356; file-offset mapping ev 5a17.13d6.2fa7.c5eb.7078.7f39.47ba.c97f.8d49.ea5f.be2c.4c1d.08e2.3a5e.796f.26f8.

## Wire handlers — Waiting (#291) (16 pins)

### `0x55b6d0` — 2035 FreeMusicStatusAnswer: stage the auto-pick level ceiling

- **VA / file offset**: `0x55b6d0` / `0x15b6d0`.
- **Behavior**: Reads a 32-bit word and, when it is zero, reads one byte into global 0x7798bc. No other work.
- **Reached from**: Wire handler: opcode 0x7f3 (2035) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible indirectly: 0x7798bc is the level ceiling read by the Waiting auto-pick arm (`docs/parity/waiting.md:342`).
- **Our client**: No apply handler for 2035. Our client stages the same value through `ScreenGame::set_auto_pick_level_cap` (`crates/o2jam-shell/src/screens/game/mod.rs:4781`); its comment still calls the writer OPEN, now pinned here.
- **Evidence**: analysis ev 2a4a.9653.505c.ecfa.56fb.762d.b6b7.1d87.fa79.a234.0ca6.88f7.13cc.a043.832f.ae3d; dossier ev b9c7.9635.3182.3154.c3dd.2cd0.a79d.32a5.f650.4672.3def.b73a.6a60.7c7e.1320.8a05; incoming xrefs ev 9a7f.e1ee.0d12.67bc.e890.a9e6.b408.7288.bd19.b37f.1f09.9c35.bc4e.5cd4.5b38.bd66 (direct call sites: 0x558080); caller facet ev 575a.89d3.c32e.e8fc.7954.8b02.8303.7628.258e.5c15.02aa.8e39.a32c.5aaa.d497.c417; names facet ev 3e38.d4d6.0abf.5f80.a43d.ae98.cdeb.572e.ebfc.039d.4794.e7bf.a45c.50a3.cc6a.b49f; file-offset mapping ev 7054.5317.6bf7.bb33.4359.07bb.5841.e43e.1db6.6970.f133.2b7b.471b.d5ce.0d83.e1ce.

### `0x55ca00` — 3001 KeyModeChanged: rewrite the room descriptor and high flags

- **VA / file offset**: `0x55ca00` / `0x15ca00`.
- **Behavior**: Reads two strings, several descriptor bytes and a 16-bit flag word, then ORs the shifted high bits into global 0x7789d8 and posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0xbb9 (3001) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: key-mode/descriptor change in Waiting.
- **Our client**: Partial. Codec `KeyModeChanged` (`crates/o2jam-net/src/rooms/mod.rs:888`) and `apply_key_mode_changed` (`crates/recomp2jam/src/session.rs:3834`) exist; the exe's descriptor-string writes are not mirrored.
- **Evidence**: analysis ev ae9d.da90.b2cb.90aa.a413.970b.42ff.cb2e.9c2c.ec0a.6db2.9e69.3ff8.8e47.4770.f5e0; dossier ev 3836.8f01.14c2.8b85.3d50.011f.7f8e.5f18.7ba3.ec95.f8a0.33a3.c16f.a009.e6a9.90dd; incoming xrefs ev 43bc.438e.31fa.c472.deaa.a3a9.0652.5590.5336.06eb.e1cf.2233.adf9.8338.9ffb.e24e (direct call sites: 0x558080); caller facet ev 9a51.2875.a0ea.3a57.b2e6.955a.dbc9.b8ab.4283.eea6.dff5.9d4c.bb74.d1ee.7c85.40ce; names facet ev d1ae.511d.d0ec.3134.ec34.2843.4846.a315.2cb5.76fe.3c62.a63d.13de.91e3.3579.f42d; file-offset mapping ev 965e.82cd.a169.d06e.113c.c549.70a7.c116.faf8.4482.1a7b.452d.945f.9ab4.576b.e0ea.

### `0x55d420` — 3004 UserJoinWaiting: seat a joining member and stage their fields

- **VA / file offset**: `0x55d420` / `0x15d420`.
- **Behavior**: Reads a member id and guards it below eight. It then adjusts seat flags and counts, calls a sequence of per-seat field setters, chooses a per-seat mode from a global room word, optionally appends a chat byte, and posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0xbbc (3004) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: a joining member appears in the Waiting seat grid.
- **Our client**: Yes. Codec `UserJoinedWaiting` (`crates/o2jam-net/src/rooms/mod.rs:818`); `apply_user_joined` (`crates/recomp2jam/src/session.rs:3817`) → seat staging. Body layout is pinned in `docs/net-protocol.md:130`.
- **Evidence**: analysis ev 2795.4de2.7512.b1c2.0a62.197b.28b1.06d7.efcc.1514.b081.960d.e7f1.c3cc.79a4.d4c6; dossier ev b0c4.f8db.7546.c6bb.83d0.639c.45eb.ef58.acbd.122e.72d3.b12e.3959.5a30.be6d.57c0; incoming xrefs ev da77.92d8.5dde.277d.cca7.980e.79eb.c885.b526.cbc4.f8b0.307f.a5fd.970e.5787.1651 (direct call sites: 0x558080); caller facet ev 4406.ff4a.c811.3ec4.0e01.7260.e016.2392.8ee1.0693.7d6a.7472.d2d4.c67a.cabe.8cc0; names facet ev 2dfb.5d09.e24c.c266.9b51.21dc.1d87.9661.56d3.32e4.9700.0b09.f1d5.1c79.6d2a.092b; file-offset mapping ev adcb.1c33.5ade.fa05.a031.bc5f.9a6e.0778.faa0.d728.f011.d3b9.5764.333f.5f80.4993.

### `0x55d700` — 3006 ExitWaiting answer: clear Waiting state on success

- **VA / file offset**: `0x55d700` / `0x15d700`.
- **Behavior**: Reads a 32-bit result word. On zero it clears per-seat mode and flag globals, walks the eight seats and releases any active ones, resets shared room objects, switches the UI state to 2 and takes the optional song/state reset arm.
- **Reached from**: Wire handler: opcode 0xbbe (3006) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: leaving Waiting returns the client to the Room board.
- **Our client**: No counterpart. Our client only sends `ExitWaiting` 3005 (`crates/recomp2jam/src/session.rs:2670`, `ScreenGame::take_waiting_exit` at `mod.rs:3051`); there is no 3006 answer handler.
- **Evidence**: analysis ev bca1.1fba.8a07.5b28.2c5a.71b8.24ac.8890.b544.ee5a.e3e3.995d.cdbd.481f.c260.a91d; dossier ev fc03.ba1f.9fa3.e01d.8e1b.a50e.3263.2810.0022.5dee.d340.57ce.9e9f.f5ea.ab41.71eb; incoming xrefs ev fec5.ea7e.b553.2fd6.bcbb.1c06.4614.cb08.c55e.cb29.8a37.8625.7cf1.b2a6.8dd5.a0d9 (direct call sites: 0x558080); caller facet ev 51e8.d256.6f57.c22a.7e83.d701.0c91.4806.bb9f.64d1.0d74.df1f.b432.575e.e68e.f990; names facet ev bd13.c664.802d.5a68.5649.5439.ffd5.e6f6.2d7e.2dc1.9013.a287.6543.fdf2.2a5b.944f; file-offset mapping ev a44e.45a3.af29.abf9.ab4f.ad73.7c68.ca97.ef25.c256.e07f.d6af.3423.a9fd.c879.01b1.

### `0x55e120` — 3009 RoomSlotUpdate: apply seat state and kick notice

- **VA / file offset**: `0x55e120` / `0x15e120`.
- **Behavior**: Reads a slot byte and a state byte. For slots below eight, state 0 clears occupancy and decrements the count, state 2 sets occupancy and increments it, and state 3 for a non-local seat clears the seat, decrements the count, posts a `[정보] %s님이 강퇴 당하셨습니다.` caption, or takes a fallback mode arm. The opcode-tagged UI message closes the handler.
- **Reached from**: Wire handler: opcode 0xbc1 (3009) in dispatcher 0x558080. Caption bytes read at VA 0x64da3c.
- **Player-visible reach**: Player-visible: seat fills/empties and the kick notice.
- **Our client**: Partial. Codec `RoomSlotUpdate` (`crates/o2jam-net/src/rooms/join.rs:465`); `apply_slot_update` (`crates/recomp2jam/src/session.rs:3781`) stages the echo. The CP949 kick caption has no client counterpart.
- **Evidence**: analysis ev cd1d.330e.3ea0.5f10.2143.e560.ab69.cd51.0017.fead.5590.be6c.9daf.1126.b0f1.0e81; dossier ev 316c.72a2.edee.06d8.c6a9.5014.745a.62bb.db31.4b74.1ec2.4342.26b9.fc25.a2e7.a72c; incoming xrefs ev c231.2339.4953.7e9a.7cb2.7608.d39c.8534.68c3.2c06.deaf.0c37.a866.3d3d.5f0e.30f5 (direct call sites: 0x558080); caller facet ev e7de.6896.58ba.2c94.bce9.1cda.0f0c.f269.af00.cecd.eb3d.cc72.09b5.e525.bea0.8613; names facet ev bc86.2847.355b.ddb8.564c.fc35.465b.5942.71f2.c2cb.4c4f.35e0.8996.e08e.2dd2.3939; file-offset mapping ev dc48.b497.1e87.e3c6.7f22.23ed.ef0d.5aa4.1b3c.7278.1a4d.924b.b49f.5682.9e4c.1761.
- **Extra evidence**: kick caption bytes ev 7a55.7d5e.89b9.9107.2907.c37d.f40e.d1d3.1001.dee4.e31f.45b5.bdbe.fa65.67ce.4267 (VA 0x64da3c).

### `0x55e2f0` — 3010 WaitingRoomEvent: choose the room-removal caption

- **VA / file offset**: `0x55e2f0` / `0x15e2f0`.
- **Behavior**: Consults the 0x4bb700 helper when the session and room slot are in the pinned states, resets all Waiting seat state, then posts one of two CP949 captions through the ControlList parent: removed for a missing song or plain removal.
- **Reached from**: Wire handler: opcode 0xbc2 (3010) in dispatcher 0x558080; the helper pin is in `docs/pins/room-waiting.md`. Caption bytes at 0x64da88 and 0x64da60.
- **Player-visible reach**: Player-visible: the notice explaining why the Waiting room closed.
- **Our client**: No counterpart. `ScreenGame::waiting_notice` (`crates/o2jam-shell/src/screens/game/mod.rs:3918`) covers START-gate notices, not this pair.
- **Evidence**: analysis ev e910.89f5.1d6c.9591.61da.81f0.1b5c.1d6f.51f0.0bfe.10c6.9998.b962.9b65.9941.cce8; dossier ev c843.d3de.3faf.1eb1.f0a0.a47e.234b.302a.78c9.ce5b.7fe1.dcf2.c3d7.b30e.5bfd.c58d; incoming xrefs ev f281.9bae.1577.cb46.8e89.7246.128c.f33a.27cb.c3d4.919d.529a.6619.b86f.3525.1447 (direct call sites: 0x558080); caller facet ev 265e.622d.040e.6d7e.bd35.6c76.e092.e523.4781.8115.8108.7782.2766.3771.49ea.5344; names facet ev d1ab.d5a5.81d7.ade1.9ee1.ee29.ac3e.f4a7.ec2c.b913.28ee.ce6b.72fa.aa07.731f.bf71; file-offset mapping ev 2029.23b6.e745.74e5.951e.744b.258f.4244.eda5.202c.72d6.40f4.fe4a.1dd4.2cf8.86ef.
- **Extra evidence**: caption bytes ev 7600.2610.edda.009d.51cb.52c8.f6b3.2e9d.6374.704a.ae0d.43d0.51a6.6ec7.a02e.2e23 (VA 0x64da88) and ev 4774.f085.86b9.ddf6.a6b9.c642.60aa.df88.a0e4.0358.894e.63c5.6a5b.77de.8d78.9248 (VA 0x64da60).

### `0x55f210` — 3014 WaitingRoomEvent: post the room-disappeared caption

- **VA / file offset**: `0x55f210` / `0x15f210`.
- **Behavior**: Clears the same Waiting seat globals and state, then posts the CP949 caption at VA 0x64daa4 through the ControlList parent. No field read precedes it.
- **Reached from**: Wire handler: opcode 0xbc6 (3014) in dispatcher 0x558080. Caption bytes at 0x64daa4.
- **Player-visible reach**: Player-visible: `공연대기실이 사라졌습니다.` notice when the Waiting room vanishes.
- **Our client**: No counterpart; same notice gap as 3010.
- **Evidence**: analysis ev 5d9c.48f9.d8fe.2788.7b34.e15b.4bc1.1ec3.13a9.527b.93b0.7a15.b70d.b96c.1786.72ee; dossier ev 756d.00f1.e69a.2e32.7902.7967.4e0e.6b9d.f31b.b93c.b139.ef00.2575.d454.8bed.4864; incoming xrefs ev aafc.9e3f.0929.29d1.29fd.a29f.906f.6d10.7890.d678.f0f6.3586.cd14.bb3d.21ed.5712 (direct call sites: 0x558080); caller facet ev beef.758f.770b.67c6.8834.cbb1.d295.e2ce.70ad.85bf.f239.1a8d.6602.9b38.b452.8961; names facet ev 3507.1e45.29a3.d5be.75e5.e522.7aca.e389.f515.88f3.af9f.db5f.7dbc.12f8.7d35.6a99; file-offset mapping ev 063d.407d.1c2b.b986.063d.55ab.47e8.01fa.315c.4286.f2a4.6e63.3f8c.0184.5e0a.0982.
- **Extra evidence**: caption bytes ev 1ddb.3899.7ade.fa94.ac76.72d7.2534.87b0.ee69.646b.869f.780e.36af.3913.dc70.e0f5 (VA 0x64daa4).

### `0x55f3f0` — 4005 SetRoomMusicEcho: update one seat's music selection

- **VA / file offset**: `0x55f3f0` / `0x15f3f0`.
- **Behavior**: Reads one slot byte and guards it below eight, composes the slot with the local seat id, calls the per-seat music setter on both branches, and posts the opcode-tagged UI message naming the slot.
- **Reached from**: Wire handler: opcode 0xfa5 (4005) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: the picked song / BPM caption follows a peer's pick echo.
- **Our client**: No counterpart. 4005 is `OPEN: body layout of SetRoomMusicEcho` in `docs/dispatch-inventory.md:100`; the shell stages picks locally from its own 4000 sends.
- **Evidence**: analysis ev 8bcf.c03d.c639.621f.40e8.21b2.79f7.c903.36de.549c.d160.e173.7815.f7b6.da23.e4a1; dossier ev cb78.3fc5.63e4.16e4.f8d6.89db.c09e.3228.4672.f240.abb2.ec53.5890.5781.719b.738f; incoming xrefs ev 9683.65fe.0d60.1892.bd56.b30d.5d88.8980.ee23.1f72.dc3d.969b.dc7d.961c.34d4.7f90 (direct call sites: 0x558080); caller facet ev 84dc.a100.3d21.2f9e.a12c.10c1.1c58.ab90.ae3e.471d.78b3.702d.fca2.d3cc.01e8.208a; names facet ev 17b2.b590.a05d.fda5.4640.18af.91e0.e619.0f49.7908.cc4e.d3b0.b44e.f2e0.9d47.a8e8; file-offset mapping ev 8f7e.1ce9.eab0.7cd3.e92e.7b30.d335.64e1.7e28.f1ab.2008.8600.1eda.9dba.e26b.8ee2.

### `0x55f5b0` — 4009 UserReadyStateChanged: apply a peer's ready echo

- **VA / file offset**: `0x55f5b0` / `0x15f5b0`.
- **Behavior**: Reads one slot byte, guards it below eight, calls the per-seat ready setter on both branches, and posts the opcode-tagged UI message naming the slot.
- **Reached from**: Wire handler: opcode 0xfa9 (4009) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: the ready mark lights on the named Waiting seat.
- **Our client**: Yes. Codec `MemberReadyStateChanged` (`crates/o2jam-net/src/command.rs`) and `apply_ready_echo` (`crates/recomp2jam/src/session.rs:3793`) stage the echo; guard `ready_echo_drains_into_the_ready_mirror` (`session.rs:11573`, Tier 0).
- **Evidence**: analysis ev 78d2.c75b.cd33.a3f8.5518.bcca.0469.1c3c.5aec.8d70.61fb.8a0f.6b95.35f7.a48f.0265; dossier ev 471d.c282.c455.d58e.830f.d895.a81c.4e8a.39a0.90af.b77d.3486.65f7.efa1.3965.7c85; incoming xrefs ev a2a2.381d.4f75.6e3c.89cb.575a.4734.e0c0.94d6.55ee.c062.c5cf.2179.144f.700a.450b (direct call sites: 0x558080); caller facet ev b11a.e9d1.85d1.4a19.94d7.1c21.1892.e1ab.16b7.b7e3.4b0d.2e30.0576.d34d.b5d1.e720; names facet ev b857.cccb.f97b.4e82.19a5.130a.8a0f.0835.ef99.4f74.a7f1.332d.460f.9117.064a.5d0a; file-offset mapping ev a209.0971.5020.1579.757c.0013.181b.70e4.c75f.43cc.cc6e.b3a1.7f25.55a0.0fb2.2c6a.

### `0x55f630` — 4011 StartGame answer: enter Playing or show a refusal caption

- **VA / file offset**: `0x55f630` / `0x15f630`.
- **Behavior**: Reads a 32-bit result word and clears the user-list/seat state first. Result 0 validates room music and state, sets the local ready flag, switches the UI state to 10 and recounts unready seats. Results 1–4 post four refusal captions through a dialog control; a state mismatch posts a fifth caption.
- **Reached from**: Wire handler: opcode 0xfab (4011) in dispatcher 0x558080. Caption bytes at 0x64db58, 0x64db44, 0x64db18, 0x64daec and 0x64dac0.
- **Player-visible reach**: Player-visible: START either enters Playing or shows one of the exe's refusal dialogs.
- **Our client**: No counterpart. 4011 is `OPEN: body layout of StartGameAnswer` in `docs/dispatch-inventory.md:106`; our client gates START locally and has no answer handler.
- **Evidence**: analysis ev 5b27.4ae7.5d53.de37.7e24.d69d.1449.e058.83b5.2bdd.fbe1.95dd.e3dd.6081.121c.f25b; dossier ev c5c0.d170.0305.a024.4a02.d1c1.45db.ae90.8299.1878.f139.3b5f.c7f8.d33b.712b.4187; incoming xrefs ev bd72.48a3.2405.921a.fd1e.edda.c9a5.163e.de07.4d91.b958.4b6d.f34f.b133.d244.f171 (direct call sites: 0x558080); caller facet ev d84d.9890.33fd.e91f.c108.e3a2.ab22.e4b3.039c.6611.ab2a.e7e1.aa8d.0d3f.d2c5.4bf6; names facet ev d3bd.5fbc.f92e.d454.cf40.d7b8.ab7f.33be.052b.72f5.2af5.b2a0.375f.dbfa.dfdc.8fac; file-offset mapping ev 1eed.271f.1cb3.e6e1.8a5e.3e9f.2934.03ad.7a41.6c0a.c20e.a276.8da3.c3f3.6271.c9a7.
- **Extra evidence**: caption bytes ev 47d3.642a.1749.13ee.41fc.6241.43ec.0714.d569.6cc8.9663.6fa3.2b70.0b6c.97e5.d622 (VA 0x64db58, not ready), ev f1ae.4369.5891.52d3.4986.770b.42ce.bf6d.9706.a638.3426.a0e6.8ca6.e71f.3a2e.3df0 (VA 0x64db44, team mismatch), ev 4c96.2d41.fc5d.3a5b.6b9d.09b2.c0dd.5832.c5d3.7d38.367a.c4e4.6d63.0786.74dc.c79a (VA 0x64db18, VS solo), ev 6e84.1be7.ecdc.a278.d510.b9c9.9704.86d3.2942.f9dc.baed.f607.8497.9ac9.a6fe.c6e7 (VA 0x64daec, server info), ev 6ba4.b473.aa59.37d3.e4ab.99b1.eeac.e5a7.895a.80ed.d309.266d.68ba.ceac.0172.dd6c (VA 0x64dac0, config error).

### `0x55f950` — 4013 MusicLoaded: stage one seat's loaded state

- **VA / file offset**: `0x55f950` / `0x15f950`.
- **Behavior**: Reads two bytes. When the named slot is the local seat it stores the second byte in global 0x77ee30; for other slots below eight it writes into the per-seat state array. It increments a global counter and posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0xfad (4013) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: per-seat loaded state that governs DOWNLOADING badges and the START gate.
- **Our client**: No counterpart. 4013 is `OPEN: body layout of MusicLoaded` in `docs/dispatch-inventory.md:107`; shell seat state is staged locally, not from this reply.
- **Evidence**: analysis ev 0df9.193a.7757.6645.aaaa.d7df.6bac.a9c3.3ad9.76a1.2a89.f5eb.aa1b.0975.6d7e.5c7a; dossier ev 651d.4b2a.f4ed.e73d.a679.3cd1.5333.a43c.ef78.4c39.8f5a.8766.0b29.9d15.bf04.ba79; incoming xrefs ev e3b3.652c.40cf.ce90.95b8.631f.6a18.a349.c16e.2157.6666.6941.f2f2.960b.cec3.1c91 (direct call sites: 0x558080); caller facet ev f5fe.aafc.6371.cd7f.226d.f224.577b.4b3f.99d7.ed63.c5f5.3742.4cfd.0590.37f0.e866; names facet ev c5c0.2249.a56b.79d0.cfb7.fcb0.8ddd.acd9.86ea.07fb.5a82.3e10.0f33.bc0b.e059.6add; file-offset mapping ev 811e.8aa0.bd96.8aa3.8253.72a2.a52b.734c.46f9.f7bf.be7c.8c52.f66f.c53c.c7f4.fb00.

### `0x55ff00` — 4029 WaitingEvent: stage counted member state rows

- **VA / file offset**: `0x55ff00` / `0x15ff00`.
- **Behavior**: Reads a 32-bit count and loops over member/value pairs. For in-range members with value 1 it calls a per-seat state helper, reads one more byte and copies the whole local seat record into that seat when local, or sets a flag otherwise. Out-of-range members take a separate helper pair. It ends by counting flags and posting the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0xfbd (4029) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: Waiting seat sync rows and their badges.
- **Our client**: No counterpart. 4029 is `OPEN: body layout of WaitingEvent` in `docs/dispatch-inventory.md:122`; no codec or apply arm exists.
- **Evidence**: analysis ev ff1a.465e.d578.e050.3c46.f593.d00c.c924.5a08.d844.2d2c.95da.7d8a.7025.9995.ce77; dossier ev 781f.57c9.123c.55d2.8480.3877.e713.9d5d.0b9c.6d18.d12f.bafe.11ab.3c6e.e786.a6fb; incoming xrefs ev 6ef6.ddda.cea8.6e09.e637.23f3.8b30.01fb.2072.1dca.186a.4331.cd43.34ae.e3a9.fc0b (direct call sites: 0x558080); caller facet ev 84d5.174d.7bd0.754a.0b56.e016.ff46.4757.1179.a137.4c55.0b51.9bd8.8fc6.aa05.bab4; names facet ev b9f4.09fc.c7ec.cd15.c14d.f25a.d6bd.0148.154b.0506.0d1f.2b1d.6be7.0d51.c165.e361; file-offset mapping ev 3e4d.b64a.daf0.d7af.a11a.b3dc.aedf.0199.e5c9.4252.6719.f959.b76f.9efb.2dc3.c860.

### `0x5602a0` — 4024 WaitingSkillChanged: update the first skill list and seat value

- **VA / file offset**: `0x5602a0` / `0x1602a0`.
- **Behavior**: Reads a 32-bit item-id count. Each id is split into a low id and a high flag bit, mapped to two catalog bytes through a shared lookup, then found or inserted in one of two vectors. It then reads a 32-bit value and a member byte, stores the value in global 0x778a5c and in the named seat's fields, clears all seats for an out-of-range member, and posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0xfb8 (4024) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: skill icons/counts in Waiting and the per-seat value readout.
- **Our client**: Partial. 4024 is recognized by `announces_room_music_state` (`crates/recomp2jam/src/session.rs:397`), so it re-announces music state, but no skill-list apply arm exists; the shell paints staged skills (`crates/o2jam-shell/src/screens/paint/waiting.rs:2305`).
- **Evidence**: analysis ev d5b7.f887.9796.f022.2ab8.cc23.10fc.5071.4cdf.a1d2.dd27.15b6.4d17.1207.d6cc.80ee; dossier ev 6476.00bf.27d2.d194.87af.e253.b750.a577.2606.b28d.b705.e1b6.b651.a3da.1713.ccff; incoming xrefs ev f77b.f2ce.5eff.f592.7995.02f6.ce27.c10f.4680.587f.0329.a62a.b567.4c85.1aff.a6ca (direct call sites: 0x558080); caller facet ev a950.d8a6.4ff1.c192.3bcc.0e8f.b14a.a150.7ba4.122d.c15f.b6b2.c6a2.c88e.322c.d311; names facet ev 3f79.429a.bb26.decc.c8e9.fe63.d714.5cb4.c1f9.48d5.5e0e.4cbb.25ef.ae4d.2dc0.217e; file-offset mapping ev acda.263f.3cc4.3186.b3df.c933.796c.bf8e.381e.fbd4.e267.7485.dc1e.791a.7138.08d3.

### `0x560600` — 4058 WaitingSkillExChanged: update the second skill list and seat value

- **VA / file offset**: `0x560600` / `0x160600`.
- **Behavior**: Same shape as the 4024 handler over a second set of skill vectors: counted ids with a high flag bit, catalog byte lookup, vector insert or flag update, then a value/member pair written to global 0x778a5c and the seat array, with a full clear for out-of-range members. It posts the opcode-tagged UI message.
- **Reached from**: Wire handler: opcode 0xfda (4058) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: the extended skill list and seat readout in Waiting.
- **Our client**: Partial. 4058 is recognized by `announces_room_music_state` (`crates/recomp2jam/src/session.rs:397`) but has no skill-list apply arm.
- **Evidence**: analysis ev 36a5.2203.891f.23ce.beee.65f9.99d4.2a40.0501.2e00.93dd.89bb.e709.ced2.6f0d.1945; dossier ev 1a23.7f3a.a133.c6bb.59f2.e2c3.4d31.9fef.8237.91ee.8317.e84a.ef92.f606.2797.3b2e; incoming xrefs ev 1a27.59b4.d3ff.6b2a.b834.9bd7.74e3.f8cb.1271.b084.3088.ce03.2fe9.5b63.a67a.cc7d (direct call sites: 0x558080); caller facet ev 0fa6.f500.dbb7.88d3.acd3.e617.a070.01e8.1b22.d2c2.6b20.7c3d.1c87.d1b8.6ecf.f7c7; names facet ev f5bb.d071.32f1.b1d6.1292.f8d8.bf49.f4e5.38d5.c017.02ac.b802.2cb9.3d20.f021.2678; file-offset mapping ev ef40.98cb.6cc8.c43b.4512.afe7.ca11.8cfd.90b0.7f77.a48d.0dfd.4c4a.3e10.c7eb.9c48.

### `0x560960` — 4026 MemberStateChanged: apply one member's state

- **VA / file offset**: `0x560960` / `0x160960`.
- **Behavior**: Reads one member byte. For the local seat it calls the first state setter and then the second; for another in-range seat it calls the same pair on that seat. It posts the opcode-tagged UI message with the member id.
- **Reached from**: Wire handler: opcode 0xfba (4026) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible: member state used by the DOWNLOADING badges and START gate.
- **Our client**: No counterpart. 4026 is not handled in `apply_waiting_frame` (`crates/recomp2jam/src/session.rs:3500`); shell state comes from 4037/4039 arms only.
- **Evidence**: analysis ev b021.e2b9.b323.9746.52db.6c04.b1d8.2253.9487.409e.465d.42b0.e952.9794.5aea.eab8; dossier ev f8f0.c123.22ba.29a7.5b2c.77e0.8611.04fe.e34b.5cb1.38f6.9fb2.1476.f3eb.fd47.9fe5; incoming xrefs ev 82c8.e77c.46f1.4a8c.d60f.7107.a8b4.dbff.2b78.6e28.8d57.073b.950d.7f08.df6b.696f (direct call sites: 0x558080); caller facet ev 1a47.32d5.b1c9.4a72.1559.efea.eed0.38ea.d863.b930.e3ec.ffdc.0829.9c80.5eef.a12c; names facet ev 5bba.48b3.33d4.a67f.fbc1.6cbb.64b6.432b.004c.b69d.5f28.da37.6172.2286.a150.06e1; file-offset mapping ev dcc5.4548.5f01.0f40.5c91.e0cf.a99d.8f45.0e92.c970.c372.7ebf.6832.a085.bd76.259e.

### `0x560a00` — 4028 WaitingRoomEvent: stage a value and one member byte

- **VA / file offset**: `0x560a00` / `0x160a00`.
- **Behavior**: Reads a 32-bit word into a global, reads one byte, and posts the opcode-tagged UI message. No other bound checks or state writes appear in this body.
- **Reached from**: Wire handler: opcode 0xfbc (4028) in dispatcher 0x558080.
- **Player-visible reach**: Player-visible support: a Waiting value/member pair that feeds later seat chrome.
- **Our client**: No counterpart. 4028 is not handled in `apply_waiting_frame` (`crates/recomp2jam/src/session.rs:3500`); no staged source for the global is present.
- **Evidence**: analysis ev 6b99.7876.86d7.12fe.cede.68fa.4fdb.ffc6.43b7.e76a.5b9b.62f3.ea35.eafd.95a3.a408; dossier ev 53fd.d4a9.7229.e405.cab4.7436.d519.2bec.28c0.d178.cb26.f870.d8a1.0af8.ccae.7e92; incoming xrefs ev 28ff.d296.86f9.52ed.1e0f.b3a7.645e.5702.c952.6dcc.be92.36e3.5e5b.d20a.edcc.7ebc (direct call sites: 0x558080); caller facet ev 6985.593d.96f2.129d.dce7.4703.57a9.3846.b841.38f3.2481.a575.9cfd.16d2.0391.2cd6; names facet ev 49b2.2bda.9a5b.d3c7.2f15.40c5.90b9.e80a.116b.56e5.9a2f.655f.8dc8.2661.671d.cc8c; file-offset mapping ev 7fb8.40cb.5b62.d3ba.fdc0.03a1.5f9b.ecd0.819e.bf59.ea59.0a93.d7a5.f07d.0b55.18e1.

## Wire handlers — Playing / purchase (#293) (2 pins)

### `0x5598b0` — 4035 AcquireMusic: stage acquired song counts and post a notice

- **VA / file offset**: `0x5598b0` / `0x1598b0`.
- **Behavior**: Reads one byte count. A zero count posts only the opcode-tagged UI message. For each nonzero entry it reads a class byte and a value byte, passes the value to a shared staging helper on both class branches, and posts the UI message carrying class and count.
- **Reached from**: Wire handler: opcode 0xfc3 (4035) in dispatcher 0x558080; call graph is handler ← 0x558080 ← data slot 0x64d028. Deeper boot chain is OPEN in statics.
- **Player-visible reach**: Player-visible: music-acquire/availability notice and staged counts.
- **Our client**: Partial. `apply_acquire_music` (`crates/recomp2jam/src/session.rs:3588`) decodes 4035; `crates/o2jam-shell/src/music_download.rs` supplies the download flow. No per-class UI message equivalent is pinned.
- **Evidence**: analysis ev 6f2d.0208.a112.8e4e.e270.e840.57f3.d35f.e7f8.c86f.7468.9a5f.86e8.8ebc.af42.042f; dossier ev ccb9.0dfb.f723.ecc9.3f8f.7835.722f.e67a.b409.bbce.1a0e.cd46.c9bb.2688.68c4.363d; incoming xrefs ev 670b.f3a4.56bc.5182.ca1a.8633.9190.216f.f8f0.e166.6131.a869.e995.82a1.b6eb.fac5 (direct call sites: 0x558080); caller facet ev bb1f.62f4.5829.67e5.5556.98de.3d08.d006.a83c.f9e4.bba6.4772.377b.7cdf.3aaf.7d0a; names facet ev cb34.a254.f12a.436b.f94b.a47c.ee76.077c.976b.2195.17cf.aca7.8177.5df9.8832.4208; file-offset mapping ev 54ec.823d.97ca.b97e.dcc8.1564.78cd.144c.32fc.53e1.b521.f757.786f.82ab.c150.ee95.

### `0x560d70` — 5011 SyncMusicPurchase: rebuild the owned-music catalog

- **VA / file offset**: `0x560d70` / `0x160d70`.
- **Behavior**: Reads a block of header words, clears and rebuilds two counted lists of music rows (each with a 16-bit id and five status bytes), then walks the 0x1b4-byte catalog records. Matching ids receive the five status bytes and a default flag when absent; related seat records are updated in the same pass. It ends with the opcode-tagged UI message and clears a global latch.
- **Reached from**: Wire handler: opcode 0x1393 (5011) in dispatcher 0x558080. Also calls the shared record reset 0x55b1f0.
- **Player-visible reach**: Player-visible: purchased/available music rows in the Waiting song list and availability badges.
- **Our client**: Yes. `ScreenGame::apply_music_purchase_response` (`crates/o2jam-shell/src/screens/game/mod.rs:7952`) and `ScreenSide::apply_music_purchase` (`side.rs:6578`) stage the same purchase state; codec guard `sync_music_purchase_5011_response_decodes_pinned_wire_bytes` (`crates/o2jam-net/src/shop.rs:1981`, Tier 0).
- **Evidence**: analysis ev d864.4600.2ebe.d61e.353a.a249.183e.7bc2.b4ce.3755.507e.d1fb.fc2c.ff10.0be7.82d9; dossier ev 9a9a.2323.4255.30ea.3074.53f7.050a.4d53.a02b.a288.10ac.7291.25cf.14ef.00b3.9131; incoming xrefs ev ca69.80af.80ff.3389.ab2e.4b66.e32f.37f6.0a88.9bd3.44c4.681b.1c3f.5740.9c5a.06e0 (direct call sites: 0x558080); caller facet ev dafb.53b7.fc69.88b0.b08b.c61c.4662.5f02.d77b.06b6.9508.fe4c.5675.294d.2c81.aa4c; names facet ev cfcc.5d92.3727.b023.95d9.8b2a.7171.8756.0c24.c950.e9ba.8b1b.7106.e39f.7ce1.5721; file-offset mapping ev b573.9279.c948.577c.d836.3ae4.3dc6.5f86.d65a.f73a.ccd0.4e18.81c7.9d70.c15a.acf1.

## Census rows and sibling pins

- Room: 2003/2005/2007 list rows, 2008/2009 descriptor/count fields, 2011
  user sheet, 2013/2014 chat, 2018/2019 whisper lines, 2026 premium rows.
  See `docs/interaction-census.md` Room ord 5, 19–20, 26–31, 69–70.
- Waiting: 3001/3004/3006/3009/3010/3014 seat and notice states, 4005/4009/
  4011/4013/4024/4026/4028/4029/4058 seat and gate states. See
  `docs/interaction-census.md` Waiting ord 33–40 and the notice surface.
- The 3010 caption helper is already pinned at `docs/pins/room-waiting.md`
  (`0x4bb700`); this doc pins the caller that selects and posts the captions.
- Existing opcode/codec maps are cited, not re-derived: `docs/net-protocol.md`
  §4b/§4c, `docs/dispatch-inventory.md`, `docs/parity/waiting.md`,
  `docs/parity/room.md`, `docs/relay.md` §10.

## OPENs / next evidence steps

- Dispatcher boot chain: name the code that installs the `0x64d028` callback
  and join it to the socket pump for a Tier 3 wire-to-UI leg.
- 2008, 2009, 2011, 2020, 2024/2026, 3006, 4011, 4013, 4026, 4028, 4029
  bodies need codecs and apply arms in our client before any census row closes.
- CP949 captions were decoded from static bytes, not a live capture; a
  qualified Wire frame plus pixel pair is needed for each notice story.
- 2035 now pins the writer of global `0x7798bc`; the comment at
  `crates/o2jam-shell/src/screens/game/mod.rs:4781` that calls it OPEN is stale.
- No cargo gates were run (docs-only lane). `scripts/check-cited-hashes.sh`
  must exit 0 on this file before the lane reports done.
