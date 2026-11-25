# Statics pins — Net packet handlers and staging support (net-c1)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`.
Provider: REA 6.10.1 / Ghidra 12.2; shared target bound from
`install-root/O2Jam/OTwo.exe`. Evidence IDs are written as `ev ` plus the
full hex in four-digit groups. Join the groups after `ev_` to get the REA ID;
the grouping keeps `scripts/check-cited-hashes.sh` from treating evidence
hex as a commit citation. File offsets come from REA
`address_to_file_offset` and match VA minus the `0x400000` image base for all
40 rows (0 mismatches). Behavior wording is ours; no decompiled text is
reproduced.

Branch `cb2/pins-net-c1`; worktree pre-flight
`/home/arnim/Projects/recomp2jam-lane-pins-net-c1`. The brief's base
`a15244e` is an ancestor of branch HEAD `80075d1` (other wave-1 pin lanes
merged in between); no reset was attempted. The brief names
`slice-net-c-00.txt`; the file present is `slice-net-c-00`, with the same
40 VAs in the same order.

Scope: the 40 addresses in `slice-net-c-00`, from the coverage triage
`Net / packet handlers` bucket (`game-reachable`, `pinned_in_repo=no`).
**Pinned here: 40/40; left: 0.** Fourteen are receive-side wire handlers
or the relay parser forwarder; 26 are shared vector/fill/unwind support
bodies in the same address cluster. A static pin is not a player-visible
close: census rows stay OPEN until a live caller chain from boot is
recorded (`docs/live-path-reachability.md`). "Reached from" below is the
provider's direct-caller edge, not proof of a runtime wire frame.

Prior pins are cited, not re-derived: `docs/gifts.md` (5014/5016/5018/5024/5026),
`docs/myroom.md` (5031/5033/5035/5037/5039 and pass word `0x768b34`),
`docs/relay.md` (4045 parser and 4052/4054/4056 forwarders),
`docs/dispatch-inventory.md` and `docs/net-protocol.md` (opcode inventory),
`docs/dispatch-inventory.md` (4050 listing), and the issue bodies `#290`,
`#291`, `#293` (spec story homes).

## Inventory

| VA | File offset | Role | Opcode |
|---|---|---|---|
| `0x562e40` | `0x162e40` | wire handler | 5014 SyncPointAnswer |
| `0x562e90` | `0x162e90` | wire handler | 5016 StartPaymentResponse |
| `0x562eb0` | `0x162eb0` | wire handler | 5018 GetGiftMessageListResponse |
| `0x563010` | `0x163010` | wire handler | 5024 ClaimGiftResponse |
| `0x5631b0` | `0x1631b0` | wire handler | 5026 SyncFreePassResponse |
| `0x563880` | `0x163880` | wire handler | 5027 SyncMemberMusicList |
| `0x563a20` | `0x163a20` | wire handler | 5035 PenaltyResetAnswer |
| `0x563a60` | `0x163a60` | wire handler | 5037 BagExpansionAnswer |
| `0x563b20` | `0x163b20` | wire handler | 5039 RefreshBagAnswer |
| `0x563d10` | `0x163d10` | wire handler | 4052 PlayingEvent |
| `0x563d30` | `0x163d30` | wire handler | 4054 PlayingEvent |
| `0x563d50` | `0x163d50` | wire handler | 4056 ConfirmRelayAnswer |
| `0x563d70` | `0x163d70` | wire handler | 4045 P2PListResponse |
| `0x563fe0` | `0x163fe0` | wire handler | 4050 PlayingEvent / latency sync |
| `0x564232` | `0x164232` | support | — |
| `0x564692` | `0x164692` | support | — |
| `0x564ae2` | `0x164ae2` | support | — |
| `0x564d40` | `0x164d40` | support | — |
| `0x564fd2` | `0x164fd2` | support | — |
| `0x565d30` | `0x165d30` | support | — |
| `0x565df0` | `0x165df0` | support | — |
| `0x565ed0` | `0x165ed0` | support | — |
| `0x56637e` | `0x16637e` | support | — |
| `0x5667e0` | `0x1667e0` | support | — |
| `0x566c37` | `0x166c37` | support | — |
| `0x567040` | `0x167040` | support | — |
| `0x567497` | `0x167497` | support | — |
| `0x5678a0` | `0x1678a0` | support | — |
| `0x567ce2` | `0x167ce2` | support | — |
| `0x5680f0` | `0x1680f0` | support | — |
| `0x56859e` | `0x16859e` | support | — |
| `0x568e1f` | `0x168e1f` | support | — |
| `0x569580` | `0x169580` | support | — |
| `0x569860` | `0x169860` | support | — |
| `0x5698e0` | `0x1698e0` | support | — |
| `0x569960` | `0x169960` | support | — |
| `0x5699e0` | `0x1699e0` | support | — |
| `0x569a60` | `0x169a60` | support | — |
| `0x569b15` | `0x169b15` | support | — |
| `0x569b30` | `0x169b30` | support | — |

## Entries

### `0x562e40` — Stage the 5014 point-sync word; post it only when the session flag is set

- **VA / file offset**: `0x562e40` / `0x162e40`.
- **Behavior**: Reads one 4-byte wire word into the shared slot at 0x768b4c. When the session flag at 0x762b44 is nonzero it forwards (0x12, 5014, 0, 0) to the UI event queue at 0x478d50; otherwise it consumes the word and posts nothing.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. The dispatcher case is 0x1396 (5014), listed with an OPEN body in docs/dispatch-inventory.md:162.
- **Player-visible reach**: Player-visible support: the posted event is the only UI effect in this body; the consumer of the staged word is OPEN.
- **Evidence**: analysis ev 8a79.ece0.2026.9c04.0f4c.a110.2331.6307.9746.9321.d668.3e50.8470.33a5.76c9.9ea1; dossier ev 6565.8ce7.96bf.fc98.47f2.3b0a.6c46.2dfe.4874.16e0.802d.f051.41d9.b99f.59e0.eb57; incoming xrefs ev e21c.1ec9.e088.18e9.af00.d28b.5f7a.6e38.902d.d810.30f4.c5e5.7094.8b4e.12eb.97fb (direct call sites: 0x55887e); caller facet ev a1db.414b.75c5.ce52.718b.f000.ffbe.1449.f454.bcd6.f35a.9e2a.3783.7462.87de.94fc; callee facet ev 467a.c15f.c32e.3c6b.891f.c601.d50f.e47a.f7fc.6e61.7d25.ebdb.5a83.cf63.0ba3.2cc4; file-offset mapping ev ac50.6f6b.250d.d09e.448e.97b0.e9d3.69ac.c45a.0fb7.947a.067b.0d52.cc84.d6fe.2ddd.
- **Our client**: No counterpart by VA. `docs/dispatch-inventory.md:162` still lists the 5014 body as OPEN; no codec or receive drain exists in `crates/o2jam-net`.

### `0x562e90` — Post the body-ignored 5016 start-payment notice

- **VA / file offset**: `0x562e90` / `0x162e90`.
- **Behavior**: Posts (0x12, 5016, 0, 0) to the UI event queue and reads no wire bytes at all.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. The dispatcher case is 0x1398 (5016), pinned in docs/gifts.md:83.
- **Player-visible reach**: Player-visible: the event is the whole behavior; the byte layout is deliberately ignored by the exe.
- **Evidence**: analysis ev 6e8a.569a.0c39.a402.db1b.d5d0.07ac.f1c6.127f.5125.caa3.2278.2031.7c1c.8c94.d774; dossier ev f296.8736.7b29.b348.1c6e.51ef.4bbc.df24.3708.d1eb.9d0f.4a31.a569.c921.6949.998c; incoming xrefs ev 8fe6.b30c.87fb.b2bc.8a44.fa36.7309.43a6.6595.2f5c.1717.e363.c41d.9808.43c5.4944 (direct call sites: 0x55888f); caller facet ev 2bd1.f496.50f5.59ea.351c.a6d1.828d.cfcc.7a44.9c7f.13ca.beeb.7218.85e2.c58e.fbd5; callee facet ev de31.5892.77ef.1949.a630.ef61.9b30.2726.d4b4.fee6.455d.59f9.2f8d.229d.e409.1b40; file-offset mapping ev 8ede.37d9.a43a.6945.145f.b0d7.1bfe.72e2.98a1.cba0.8c32.e2dd.ee7f.4ef8.70e6.f1f7.
- **Our client**: Empty codec `StartPaymentResponse` at `crates/o2jam-net/src/wallet.rs:30`; no receive-side UI drain by VA.

### `0x562eb0` — Stage 5018 gift-message rows and post the list notice

- **VA / file offset**: `0x562eb0` / `0x162eb0`.
- **Behavior**: Reads a 4-byte flag and a 2-byte row count. On a zero flag it reads that many 0x168-byte rows (one 4-byte id, one byte and four string reads each), appends each through staging helper 0x564390, and posts (0x12, 5018, 0, 0). A nonzero flag skips the loop but still posts the event.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0x139a (5018) is pinned in docs/gifts.md:86.
- **Player-visible reach**: Player-visible: the staged rows feed the gift list; the exe reader consumes fields that the codec labels exe-ignored.
- **Evidence**: analysis ev 865e.1b28.8558.5c95.af98.024d.00c8.d6a6.00df.ea5d.0fa2.7f57.bcfd.9863.8b87.ce6b; dossier ev dd32.dd5a.b9eb.c2d5.fa14.63de.e2f9.0dbf.efec.f2d7.ad36.599e.7d31.b5c2.3fdc.50f2; incoming xrefs ev 3b16.c3c5.6850.1121.dcf1.3f12.c0b4.de58.76cb.5901.03be.b345.ddd3.e268.7d78.83e1 (direct call sites: 0x5588a0); caller facet ev 2b5a.7f66.162f.c709.3178.59cc.7763.5150.4698.4902.df0e.db5d.1beb.b104.470e.721e; callee facet ev 8623.1f4e.500e.a436.c20f.708b.032c.899e.3205.bbe7.64e7.08df.7312.4c08.9342.e21a; file-offset mapping ev 9884.e159.8017.d1bc.44bd.ff91.b5fa.68c3.6ebf.4577.7418.ce2f.c3f5.b497.8a88.e05a.
- **Our client**: `GetGiftMessageListResponse` codec at `crates/o2jam-net/src/gift.rs:262` and `:282`; no receive-side row staging by these bytes. `docs/gifts.md:86` marks the Memoryer widths exe-ignored.

### `0x563010` — Judge the 5024 claim answer by its validity word

- **VA / file offset**: `0x563010` / `0x163010`.
- **Behavior**: Reads one 4-byte validity word. A nonzero word sends a one-byte code through caption writer 0x478c90: codes 1 and 2 share the pinned 'Not defined' caption, while 3 and 4 use two other pinned captions. Every path forwards (0x12, 5024, validity word, 0).
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0x13a0 (5024) is pinned in docs/gifts.md:93.
- **Player-visible reach**: Player-visible: the validity word picks the notice and the claim arm; the verdict byte is never read.
- **Evidence**: analysis ev 1160.7a8d.a3ef.cbd8.25ab.3247.5bfa.69e0.0ec8.3842.60e1.04d1.9b3f.1add.cf55.57de; dossier ev 384e.096f.f89b.e2b0.f437.4b2d.4566.f754.3347.6b88.cecf.cea5.d4c3.ef51.856e.e335; incoming xrefs ev 8ee3.24bb.8f39.5e53.e276.8b3d.2279.d428.82e3.6fb4.a1f1.e158.4a68.4ce0.945a.c3d9 (direct call sites: 0x5588b1); caller facet ev 873b.c3ac.f6be.dccf.16cd.2172.45fd.a81a.5de8.1df6.a6f4.dba2.4485.adb7.36f0.ec2a; callee facet ev 251c.3c6e.a453.be19.348e.d068.125e.c4f5.7878.2208.fab6.5b57.dd8e.b479.2a48.ce37; file-offset mapping ev 28d7.8277.aea4.3267.d0bf.839b.d53f.a41c.9310.2ef7.497f.d67f.9904.2fc4.60a9.4b7e.
- **Our client**: `ClaimGiftResponse::classify` at `crates/o2jam-net/src/gift.rs:405` and Tier 2 guard `valid_5024_with_memoryer_verdict_four_moves_the_gift_into_the_bag` at `crates/recomp2jam/src/session.rs:6712` already fold valid answers; the verdict arm stays exe-ignored.

### `0x5631b0` — Write 5026 free-pass lines from the staged pass word

- **VA / file offset**: `0x5631b0` / `0x1631b0`.
- **Behavior**: Reads a 4-byte result. Result -0x5e posts the event with no pass lines. Result zero reads a group of 4-byte globals through the shared reader, applies two of them through 0x540090 and 0x540d00, then writes one to four notice lines through 0x479010. The pass word at 0x768b34 selects the FREEINFO captions; the flag at 0x768b80 adds a suffix. The body ends with (0x12, 5026, 0, 0).
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0x13a2 (5026) is pinned in docs/gifts.md:101; caption word mapping is pinned in docs/myroom.md:310.
- **Player-visible reach**: Player-visible: this body writes the pass lines the MyRoom side paints.
- **Evidence**: analysis ev ffda.e60e.2efc.06eb.c208.443c.dca8.2517.b352.3a55.a23b.1255.a746.5192.8684.5d6c; dossier ev 31d6.16e9.f609.83c7.ba3d.2254.3d9d.a45a.8839.3c38.ce24.da76.dc24.6722.f9c1.da4b; incoming xrefs ev 2b44.e0fc.6bc1.a6d2.56ba.e4fb.18fd.a541.6cd4.6e8d.323c.9c15.5d07.3c56.7a18.90fc (direct call sites: 0x5588bf); caller facet ev 3a3a.3d69.3065.9240.33ba.87ed.0085.e265.575e.60d6.d1e9.184f.bfa8.64e6.3dd0.24be; callee facet ev 4f2a.3d90.4300.b7e3.a81a.32f8.3e39.bf90.f958.41df.64c6.34b2.62c5.a75a.4270.d3f7; file-offset mapping ev 5424.7f7b.cb32.6628.82fb.6108.8ace.51cc.e503.6b57.0796.7fa5.8f6a.5959.a215.43fa.
- **Our client**: `SyncFreePassResponse` codec at `crates/o2jam-net/src/wallet.rs:199`; `FreePassState::apply_free_pass_response` at `crates/o2jam-shell/src/screens/side.rs:6694`; guard `myroom_staged_pass_inks_the_pinned_freeinfo_lines` at `crates/o2jam-shell/src/screens/game/mod.rs:20167`. The receive-side writer of 0x768b34 stays OPEN.

### `0x563880` — Read one member's 5027 music list and post the member id

- **VA / file offset**: `0x563880` / `0x163880`.
- **Behavior**: Reads one byte from the packet slot's fourth byte. When it is below 8, a byte equal to the local member byte at 0x77cd04 is kept as the local id; the member-music reader 0x540db0 then reads a 4-byte count and that many 2-byte words into the member slot (low twelve bits music id, high four bits split out), and the body posts (0x12, 5027, member byte, 0).
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0x13a3 (5027) is listed in docs/dispatch-inventory.md:172.
- **Player-visible reach**: Player-visible support: it stages another seat's music list and notifies the Waiting grid; no pixels are painted in this body.
- **Evidence**: analysis ev c129.c15e.1d0b.02f2.c35a.4032.d00f.81ce.2b3d.e633.6c43.3efb.e78d.5439.ce25.0068; dossier ev e5a1.182e.d408.b147.60cd.273f.0489.7bd9.ba11.a745.43a2.b1cc.4813.f265.be60.9b89; incoming xrefs ev 9cd4.6b10.bd8e.5eb7.0325.9807.f8bb.f9bc.92ba.6220.e36a.ca3f.0d7e.0827.a539.4610 (direct call sites: 0x5588cd); caller facet ev b656.9889.c68b.26cd.5332.32bf.46df.e2e6.647d.0180.59d0.8f13.273e.7010.8442.89f6; callee facet ev 62c2.68fd.a316.d520.aa81.39e9.352c.c365.0fa3.f1be.a683.17e1.ce90.0267.35db.6815; file-offset mapping ev bada.d433.f93d.4d9f.d7b5.36c7.2074.f52f.e187.866a.b1c2.5cda.daa3.0186.9fbe.cb18.
- **Our client**: No counterpart by VA. `docs/dispatch-inventory.md:172` still lists the 5027 body as OPEN; no receive-side decoder in `crates/o2jam-net`.

### `0x563a20` — Forward the 5035 penalty-reset code

- **VA / file offset**: `0x563a20` / `0x163a20`.
- **Behavior**: Reads one 4-byte code through the shared reader and forwards (0x12, 5035, code, 0) to the UI event queue. No other field is read.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0x13ab (5035) is listed in docs/myroom.md:347.
- **Player-visible reach**: Player-visible: the forwarded code selects the pinned penalty notice through the later UI consumer.
- **Evidence**: analysis ev 6d7f.6b6d.3ae7.87df.56ce.1570.1eec.df6a.493c.095b.cd7c.b057.bcf3.e2ca.08af.123c; dossier ev 96c3.236e.1178.54f0.7fda.87a2.b974.5d93.1965.4071.d404.4347.adaf.0bf8.acf8.1bf5; incoming xrefs ev 0e12.46ef.1e71.31ad.4e4a.211f.6c1f.e78e.4d50.651e.f586.965f.e552.3306.3471.7d36 (direct call sites: 0x558905); caller facet ev 4f22.ba43.a0f7.ddec.7418.15e3.7acf.6dd8.a523.478e.cb32.1980.e0df.6871.3abb.31f2; callee facet ev 8c84.0b6c.ee41.bfd6.95b2.3f82.0626.8d7f.d25a.d87d.5556.91ef.140f.528e.84d1.febc; file-offset mapping ev 4d93.0c26.ea0d.b9d9.35eb.7b1e.e3d0.0001.247a.2271.8111.7bd9.ffe5.79b3.ab16.9f20.
- **Our client**: Notice captions at `crates/o2jam-shell/src/screens/side/myroom_notice.rs:53` and `:56`; no receive-side driver from the handler exists. `OnlineSession::poll_myroom_replies` at `crates/recomp2jam/src/session.rs:3095` only drains expected short replies.

### `0x563a60` — Grow the bag on a successful 5037 answer

- **VA / file offset**: `0x563a60` / `0x163a60`.
- **Behavior**: Reads a 4-byte result. A zero result reads a second 4-byte count, calls 0x537950 to extend the bag by that many slots, and initialises 30 slots through 0x537c10. Every path forwards (0x12, 5037, result, 0).
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0x13ad (5037) is listed in docs/myroom.md:348.
- **Player-visible reach**: Player-visible: slot growth and the later bag caption change the MyRoom bag board.
- **Evidence**: analysis ev 52c1.26fb.3df6.c426.7d2c.1f46.879b.d304.7f64.38e2.5603.ad7d.ffcf.4e3a.fcbf.bdd5; dossier ev 32ea.912e.499f.fbb1.77cf.e955.a970.052e.7143.56bf.29d9.567c.3a65.2804.f04e.bcb9; incoming xrefs ev 0568.d4c8.0f8c.304a.be8a.3cd9.f7e5.5ca6.266c.d528.8b66.c3ef.b315.889c.168b.f4f2 (direct call sites: 0x558913); caller facet ev 6c07.39e8.d5ed.5077.3da5.f698.971a.51cc.7627.4c6e.8275.2ded.bcb5.e916.a187.2602; callee facet ev 48c7.4c68.c7cd.e439.cadf.8b87.a304.7268.82d0.d062.a54d.a665.aec0.440b.a6cd.f675; file-offset mapping ev 2ecd.0016.ae98.e975.f5cb.33fb.6a08.d4c0.35f2.8303.b56e.4a85.f984.fd58.264a.80a0.
- **Our client**: Notice captions at `crates/o2jam-shell/src/screens/side/myroom_notice.rs:57` and `:59`; no receive-side driver from this handler exists.

### `0x563b20` — Rebuild bag slots from the 5039 answer

- **VA / file offset**: `0x563b20` / `0x163b20`.
- **Behavior**: Reads a 4-byte result. On zero it reads a 4-byte row count, then for each row reads two words, maps the item word to a slot through 0x537e90, writes the slot through 0x538240 and stages a face byte through 0x4a9b60; it also initialises four face slots. Every path forwards (0x12, 5039, result, 0).
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0x13af (5039) is pinned in docs/myroom.md:349 and `crates/o2jam-shell/src/screens/game/mod.rs:19182`.
- **Player-visible reach**: Player-visible: the staged slots and face bytes feed the bag board and its failure caption.
- **Evidence**: analysis ev 5dfe.0e3a.1811.97d5.103c.06e5.a543.5059.564d.202a.ef27.e848.8b10.f62f.7b83.12b3; dossier ev 1218.ddaf.bdf5.ec67.66f1.2303.435e.12fe.8a83.b306.78fd.3dbc.cce7.dcd3.6f30.aed2; incoming xrefs ev 9e27.e30a.a58a.c39f.9312.eb2c.4978.696a.62fc.1aa5.2ea4.e615.1b18.371d.e1b5.55a5 (direct call sites: 0x558921); caller facet ev 09ce.48e1.e08e.018a.cdde.b534.0a68.a700.7612.10ec.e721.c9b2.97e7.8328.9958.500e; callee facet ev f66d.dbfc.2bb0.78ba.4cf4.fd8e.525d.3e77.6f14.ac55.aebc.5ad6.96ab.34ea.90f8.3c2b; file-offset mapping ev e581.01cd.975e.b59b.5745.bbec.7620.2586.2726.1e01.1239.f9a6.86b7.552a.ce7a.0a47.
- **Our client**: `ScreenGame::apply_myroom_reply` at `crates/o2jam-shell/src/screens/game/mod.rs:2377` drops the bag cell cursor; Tier 2 ignored guard `myroom_bagsort_sends_5038_and_5039_reply_through_scripted_gateway` at `crates/recomp2jam/src/session.rs:6598`.

### `0x563d10` — Forward the body-ignored 4052 playing event

- **VA / file offset**: `0x563d10` / `0x163d10`.
- **Behavior**: Posts (0x12, 4052, 0, 0) to the UI event queue without reading any wire bytes.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0xfd4 (4052) is named in docs/relay.md:388 and docs/dispatch-inventory.md:143.
- **Player-visible reach**: Player-visible: the event is the whole body; live captures tie 4052 to the Playing gate, but no field is consumed here.
- **Evidence**: analysis ev 59fe.f2a7.bac2.b6f3.6fd4.4531.35b2.31c1.825c.4c00.4098.eea2.1cc4.e2da.e2be.4929; dossier ev 81e8.5c83.1aaa.55ae.e54a.4ba7.4d2a.b42f.78ab.c7c6.df3c.d161.028c.e8b2.98f2.faec; incoming xrefs ev 3201.294d.a938.92a3.809a.5611.fc30.dcbf.1a32.9e11.1e5c.a92f.ea03.84e5.a02d.530f (direct call sites: 0x558802); caller facet ev e136.8b2d.8229.4ae0.0793.d547.937a.1cee.e9f3.77ba.cae5.fa76.0fcc.761c.4648.ef82; callee facet ev 9441.17b7.6e08.023b.6cc6.0223.771f.2029.f936.bed2.2b72.374d.adbc.fadd.951c.6313; file-offset mapping ev b6b0.4dcd.f046.2142.724d.9fcd.8b1c.98de.22bb.c49d.7c5d.1c88.a1d9.4c36.8f86.901f.
- **Our client**: No opcode codec or receive driver by VA in `crates/o2jam-net`.

### `0x563d30` — Forward the body-ignored 4054 playing event

- **VA / file offset**: `0x563d30` / `0x163d30`.
- **Behavior**: Posts (0x12, 4054, 0, 0) to the UI event queue and reads no wire bytes.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0xfd6 (4054) is named in docs/relay.md:388 and docs/dispatch-inventory.md:145.
- **Player-visible reach**: Player-visible: the event is the whole body. The owner pin keeps relay paths in live battle only.
- **Evidence**: analysis ev 1f0c.7727.e2e4.abbc.318a.9d5e.e545.0ff7.528b.5a91.6086.a973.a4fb.0429.dbae.3ca3; dossier ev ec4a.f2d6.d91b.b718.34b7.10ec.4bda.1aa9.0a88.09b2.3645.30f7.75ae.fd22.4b7d.fbd2; incoming xrefs ev 56c3.be19.d083.e49f.415f.1770.f0f8.34ec.e006.c138.2ff7.37cb.d930.5e19.4c51.5b58 (direct call sites: 0x558813); caller facet ev 31e1.241a.3a5d.1db5.1846.dca2.4f0d.664f.464a.7a38.6382.6ce6.c58d.beca.f973.0207; callee facet ev 711a.2638.4e0f.278f.0a0f.8f91.cfd6.7635.6648.ce2f.9c1b.c205.c776.0a96.2c04.469f; file-offset mapping ev 7749.9912.9c1e.7dec.1e1f.a2cd.78b3.5b25.54a1.7702.ed2d.e080.bae3.f4ca.57d8.3f33.
- **Our client**: No opcode codec or receive driver by VA in `crates/o2jam-net`.

### `0x563d50` — Forward the body-ignored 4056 relay confirm

- **VA / file offset**: `0x563d50` / `0x163d50`.
- **Behavior**: Posts (0x12, 4056, 0, 0) to the UI event queue without reading a body word.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0xfd8 (4056) is pinned in docs/relay.md:388.
- **Player-visible reach**: Player-visible support: the confirm event belongs to the live-battle relay handshake; no normal Playing gate may be inferred from it.
- **Evidence**: analysis ev cc43.1c11.6f27.c77a.7906.4127.96f8.3c0b.ec07.329f.f685.58a0.9257.cd63.1af5.4263; dossier ev 8a7d.7953.6503.2b92.e6de.968b.3dcc.46b1.b81e.fbe1.d7ba.3530.3fc7.4e75.688b.8594; incoming xrefs ev f9ff.d0f7.2a65.ce47.3222.ab59.bad1.bd49.3a84.62c6.8c53.7fb8.c3f1.716a.201c.37a8 (direct call sites: 0x558824); caller facet ev 4444.d4b1.972a.6835.230c.d401.7897.3743.ef9e.2352.660a.a265.e308.15c2.625d.ae20; callee facet ev 5b2f.e4a8.439c.9dd4.697d.7ad9.28bc.2924.bf37.f2fe.ed65.c263.f905.be77.6f24.0860; file-offset mapping ev 0110.aa14.def2.d7ee.2797.6834.2c4e.509f.2d83.ab94.7c7f.3f30.5831.d618.f748.c561.
- **Our client**: Body-ignored codec `ConfirmRelayAnswer` at `crates/o2jam-net/src/relay/commands.rs:743` and `:753`; no receive driver from the handler.

### `0x563d70` — Hand 4045 to the relay peer-list parser

- **VA / file offset**: `0x563d70` / `0x163d70`.
- **Behavior**: Forwards the whole packet to relay parser 0x58cf50. That parser reads a 4-byte count, then per peer a 4-byte member key, a 6-byte public address and a 6-byte local address, and adds each through 0x58d510 before a final 0x58d030 commit. This body posts no UI event itself.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0xfcd (4045) and the parser are pinned in docs/relay.md:447.
- **Player-visible reach**: Player-visible support: it stages live-battle peer endpoints; it paints nothing and runs only on the relay path.
- **Evidence**: analysis ev 96b8.f376.46f3.5867.8160.c21f.5406.f326.7302.0a85.b1b4.8e39.f033.1f0e.d979.c75a; dossier ev e159.fcba.ec32.6772.f81e.bec8.dafc.3f74.3569.3cb3.2508.54ef.96b1.b8c2.bc24.9cfd; incoming xrefs ev d740.a067.95bc.6ff8.c858.fc9a.1164.2866.382b.4d45.b95b.df68.4bd6.f2cd.4284.b75c (direct call sites: 0x5587be); caller facet ev 08e2.6aa7.9029.5f5c.fb05.2c8d.94bb.9a04.0720.facc.4711.21fb.4422.8f60.544c.f47d; callee facet ev 4bfb.189f.000e.0af2.579b.e5d6.ba1b.8e7b.5ee8.7757.f0cc.02f0.3c47.a9eb.b401.66e2; file-offset mapping ev 32db.4d78.7a29.b0b3.9a11.4587.6438.b4a3.882e.7e0e.183d.89bd.e22a.87cf.1afa.30c2.
- **Our client**: `P2PListResponse` codec at `crates/o2jam-net/src/relay/commands.rs:1396` and `:1415`; driver from the handler into the relay session stays OPEN.

### `0x563fe0` — Write the 4050 speed bytes into the self member slot

- **VA / file offset**: `0x563fe0` / `0x163fe0`.
- **Behavior**: Reads two single-byte wire fields and writes each through 0x541580 into the self member slot at +0x2168 (the speed byte used by Result captions). It then sets the flag at 0x768358+0x103ac to 1 and drains one 4-byte wire word. No UI event is posted.
- **Reached from**: Only direct caller is the packet dispatcher at 0x558080; one incoming call site. Dispatch case 0xfd2 (4050) is named in docs/dispatch-inventory.md:141 and the live handoff is described in docs/captures/journey-audit/293-playing.md.
- **Player-visible reach**: Player-visible support: the staged speed byte and flag affect Playing/Result, but this body emits no pixels. Why both bytes target the same slot stays OPEN.
- **Evidence**: analysis ev 7689.1c2f.ded7.76d1.3e6f.784e.e5d4.2f28.a5ca.9ec7.c917.7385.bc8b.62bc.b15e.693d; dossier ev 2193.e8fb.1c13.928f.72b4.0040.3ad1.3b22.a88d.7357.5cd7.3ed9.c3cd.4376.fa96.95d7; incoming xrefs ev a3f0.b64d.5861.3e3a.f942.3ca6.96c1.4d02.440d.8c57.3fd6.c89d.58c9.59ac.0d18.2a97 (direct call sites: 0x5587f1); caller facet ev dfb2.c7f7.6ce7.c4e8.a9dd.1e1f.1ad6.54b5.9baa.ac02.9ffe.82a3.3fd0.a922.9421.8a36; callee facet ev fab3.22b7.fb10.ed23.fd59.fdf2.2fb8.02e5.0721.0514.f3e3.60a0.6a2b.88a0.67b5.02e7; file-offset mapping ev dcb1.709f.9bcf.2916.89b8.a47d.e06b.e3f2.679d.cda6.96ec.2c0c.41ab.4219.6dbc.05b8.
- **Our client**: No counterpart by VA. `docs/client-server-plan.md:132` names the 4050 synced-latency event; there is no codec or receive drain in `crates/o2jam-net`.

### `0x564232` — Finish a reserve of 360-byte vector records (support)

- **VA / file offset**: `0x564232` / `0x164232`.
- **Behavior**: Reserve epilogue for 360-byte records: counts the live range, walks and frees the old allocation, then rewrites begin, end and capacity from the caller's arguments.
- **Reached from**: Only observed direct caller(s): 0x564040, the reserve wrapper for the 5018 gift-list vector..
- **Player-visible reach**: Support. Shared capacity-growth epilogue with no pixels or wire.
- **Evidence**: analysis ev f6a9.5a5b.0711.388b.e17d.3727.f163.8be1.be13.c560.25b9.5fe6.bcf7.10c4.58b1.bf47; dossier ev d134.c51a.1028.4894.a41e.1273.3909.24de.8d02.22dc.b5e3.55b8.757a.618e.a53b.7eb7; incoming xrefs ev 01f9.2064.1e46.8f21.cb81.494d.c8cd.21a5.1b20.0459.699a.9042.c5ba.6a70.87b4.f0cb (direct call sites: 0x56420e); caller facet ev bbdb.9822.b612.bd8b.00e6.ad85.759b.a68f.8a34.5895.ede6.8f83.6055.d7c2.55e8.8c33; callee facet ev 6228.ea8e.7f99.8694.49c5.31b4.42d0.a23f.1d71.20ec.63a5.a829.87c0.5296.1b25.701c; file-offset mapping ev 9a34.ed9c.2743.d460.335f.fbc0.e367.e50e.92a6.229a.06d6.6ed0.9a91.80c7.847b.5753.
- **Our client**: No counterpart by VA; Rust `Vec` growth handles the role.

### `0x564692` — Finish a reserve of 36-byte vector records (support)

- **VA / file offset**: `0x564692` / `0x164692`.
- **Behavior**: Reserve epilogue for 36-byte records: same count-free-rewrite sequence at a 36-byte stride.
- **Reached from**: Only observed direct caller(s): 0x5644a0, the 36-byte reserve wrapper..
- **Player-visible reach**: Support. Shared capacity-growth epilogue with no pixels or wire.
- **Evidence**: analysis ev 142d.f9d5.d898.dfb9.8691.a9d4.63ab.65e4.4cbf.390e.46e9.4502.801f.0e05.b3a5.f518; dossier ev 121b.66c5.f2ef.c20b.fc30.f840.cca4.bef7.cef3.3f02.473a.b804.0b5d.d797.5c2a.444e; incoming xrefs ev abdc.6511.2e70.7172.c946.bb8a.84d5.4c99.f6b1.32ef.478b.bcaa.aad8.08e5.ed80.869b (direct call sites: 0x56466e); caller facet ev 4631.e89b.f25b.3213.f875.9d46.07cb.2818.e93c.99f1.32fa.8734.be1e.8000.65f5.d871; callee facet ev c2b8.b355.5d57.81a5.5729.f1f0.6509.b048.7c86.6739.da87.0ac5.3200.397b.a7ab.3714; file-offset mapping ev 45e5.d918.8e57.97fe.bfcb.b272.3d56.e5a2.4696.2f62.2ba1.e19c.754b.2b32.2637.6106.
- **Our client**: No counterpart by VA; Rust `Vec` growth handles the role.

### `0x564ae2` — Finish a reserve of 56-byte vector records (support)

- **VA / file offset**: `0x564ae2` / `0x164ae2`.
- **Behavior**: Reserve epilogue for 56-byte records: same count-free-rewrite sequence at a 56-byte stride.
- **Reached from**: Only observed direct caller(s): 0x5648f0, the 56-byte reserve wrapper..
- **Player-visible reach**: Support. Shared capacity-growth epilogue with no pixels or wire.
- **Evidence**: analysis ev dc15.18b1.bc60.84ba.078f.6e1a.19e9.f643.dcd9.4ac4.bda3.e16b.7965.dcf2.5063.7dfd; dossier ev f140.2fe6.8c9d.9dcd.3d87.4364.e2e1.a368.9937.4ef8.f347.0fc2.d323.8c5e.1894.0e51; incoming xrefs ev 3f65.62ab.03c5.c96e.85ba.2824.5270.313c.b54d.b210.a2f4.caff.7b2a.117a.ef2b.8a4a (direct call sites: 0x564abe); caller facet ev ce21.0507.960a.32c5.1927.ac8f.42cc.8209.e8ea.2510.d00a.0c42.c7f0.96c0.e6f6.43db; callee facet ev 2601.a811.720e.3760.d497.9045.3135.0f58.7e9a.9196.e548.a1b2.1742.d6bf.e863.4965; file-offset mapping ev 6400.add9.0d73.7cda.f8c5.9698.9678.cca3.b974.6c27.b4e5.5aff.a06a.6034.db1c.cb77.
- **Our client**: No counterpart by VA; Rust `Vec` growth handles the role.

### `0x564d40` — Emplace a 436-byte record and return its iterator pair (support)

- **VA / file offset**: `0x564d40` / `0x164d40`.
- **Behavior**: Copies one 436-byte record from the caller's source, rewinds the destination end by one record, and writes two caller words into an output pair before returning it.
- **Reached from**: Only observed direct caller(s): 0x5593b0, a shared container caller..
- **Player-visible reach**: Support. Container write staged by a shared caller; no player-visible decision.
- **Evidence**: analysis ev 17c5.acdf.8c43.9ac1.b03a.aa86.1303.09c6.4169.769e.f660.a821.5c24.7ad1.ac69.7fb5; dossier ev ff86.d06d.e9fc.f928.51f8.e48f.542a.8b46.accd.5b00.6405.7a6b.13f1.24af.ed38.200b; incoming xrefs ev b7bc.0930.e7ff.9180.cfba.d933.d127.f389.0c2d.3f79.6838.0d6b.3222.ba58.3cb8.c136 (direct call sites: 0x55975f); caller facet ev 3d6f.24c4.ab00.40ba.b4e9.9573.f1ae.664a.d3fe.7a00.5262.c9bc.febd.f92d.2e42.94c4; callee facet ev d49b.137c.a63d.e37e.b2b6.5bbc.f241.b0bf.cfac.aaa7.e646.0950.3c0d.9e42.de78.27c8; file-offset mapping ev e228.7390.28d6.7523.b2cb.20fc.eaaf.41c1.e802.ff35.8f42.884f.fdfb.a98c.3700.745e.
- **Our client**: No counterpart by VA; Rust `Vec::insert` idiom.

### `0x564fd2` — Finish a reserve of 12-byte vector records (support)

- **VA / file offset**: `0x564fd2` / `0x164fd2`.
- **Behavior**: Reserve epilogue for 12-byte records: count, stride walk, free and pointer rewrite at a 12-byte stride.
- **Reached from**: Only observed direct caller(s): 0x564de0, the 12-byte reserve wrapper..
- **Player-visible reach**: Support. Shared capacity-growth epilogue with no pixels or wire.
- **Evidence**: analysis ev ab8b.1221.db5c.29d6.9481.e488.2c0f.3c3a.aaa0.c958.1717.4cb6.616a.c0ed.9213.a0be; dossier ev cccd.613f.c8d1.0591.f800.644e.3617.dfb2.bd80.f2a0.90b7.9f58.b12b.4fb9.0e94.d96f; incoming xrefs ev 1a97.6436.b48b.f7bf.7584.086f.6b80.1fcb.c60c.679d.82a7.1962.ed65.53f2.c050.6e01 (direct call sites: 0x564fae); caller facet ev f849.9346.6cb1.f0ca.aa0b.777d.d0b6.3003.a220.c4c6.809b.4713.cd69.b2d6.5057.0e46; callee facet ev ec66.9cdc.640f.3aef.e9c9.572e.b583.c049.ceeb.b669.e71f.61e4.85e5.3573.8788.6bef; file-offset mapping ev 3e85.21c9.3b37.7778.2d17.615e.0dc2.f780.32c7.7103.2732.d198.ac80.854e.f5ca.0395.
- **Our client**: No counterpart by VA; Rust `Vec` growth handles the role.

### `0x565d30` — Reserve storage for eight-byte elements (support)

- **VA / file offset**: `0x565d30` / `0x165d30`.
- **Behavior**: Clears the vector pointers, allocates count times 8 bytes when count is nonzero and below the limit, and reports failure through the shared error path.
- **Reached from**: Only observed direct caller(s): 0x5593b0, the shared container caller..
- **Player-visible reach**: Support. Allocator bookkeeping with no pixels or wire.
- **Evidence**: analysis ev b1c5.f52f.7f59.564e.a2bf.dd6b.aa7e.ce4e.89e4.1469.90a0.330e.055f.8b42.bbe8.6133; dossier ev 1b8b.9573.a642.beae.73bd.9c7b.e658.8f32.643a.cedc.add0.76a3.a26e.acdb.cff9.e3e0; incoming xrefs ev 0448.6eef.d4b5.669c.a1c5.f356.1e93.492f.4a15.c921.6093.2e9f.8213.5f15.aa9a.a62a (direct call sites: 0x5593f6); caller facet ev 5967.c77d.4464.388d.0509.3fc8.3768.0897.3d7d.019b.c532.f945.57ce.28b5.dd23.b267; callee facet ev 5d1a.39fa.c94b.31ea.6f30.76c7.2b7f.892f.057a.870b.2517.8902.665c.9b45.d8ae.aa4f; file-offset mapping ev d0cd.5c04.e11d.29c6.3957.75bc.6255.12ef.19b7.b6d9.9e25.c9de.9a54.cfd0.cf3f.de3d.
- **Our client**: No counterpart by VA; Rust `Vec::with_capacity` idiom.

### `0x565df0` — Free a counted eight-byte vector range and reset it (support)

- **VA / file offset**: `0x565df0` / `0x165df0`.
- **Behavior**: Walks a live eight-byte range, frees the old block and resets begin, end and capacity to zero.
- **Reached from**: Only observed direct caller(s): 0x5593b0 and 0x5653a0..
- **Player-visible reach**: Support. Vector clear/destroy; no pixels or wire.
- **Evidence**: analysis ev 1cec.63d4.8087.3d98.d40a.1bfa.93e3.3d25.adb3.34e6.fbeb.8ae6.4ffb.72ea.d11b.75de; dossier ev bca0.662b.8f58.14bf.125d.7832.669e.ed55.e4b7.6c1e.db8e.c055.5953.50ef.825b.3151; incoming xrefs ev 6b4b.fcca.d385.5633.be7d.b08b.53bb.013d.9f5c.9d16.eb8e.75e1.a31b.ef87.98ef.e14b (direct call sites: 0x559889, 0x5653ac); caller facet ev 3607.fb52.3c66.2c9a.dfba.4faa.4b54.7517.20a0.01a1.5343.5d2f.fd56.2332.ffc6.1140; callee facet ev 81c8.cf71.7b13.a221.305b.9e7a.3d1f.2771.0dfe.e8cd.edd1.4bf8.aad2.964e.ef55.8642; file-offset mapping ev 37b0.8529.c46d.7ab3.fec4.0af4.feb9.355f.b18d.c283.2e7d.69a9.1fa9.db3c.c9ff.06bc.
- **Our client**: No counterpart by VA; Rust drop handles the role.

### `0x565ed0` — Insert a run of 360-byte gift-list records (support)

- **VA / file offset**: `0x565ed0` / `0x165ed0`.
- **Behavior**: Copies a 90-dword prototype, then inserts a counted run of 360-byte records at a position: it grows when capacity is short, moves the old tail, fills the gap, and takes one of two cleanup epilogues.
- **Reached from**: Only observed direct caller(s): 0x565570, the 360-byte vector insert caller..
- **Player-visible reach**: Support for 5018 row staging; it changes container memory only.
- **Evidence**: analysis ev a594.3003.b482.cc91.4e5a.bbf0.ddd8.0821.de95.4cfe.70fc.2c52.634c.f88d.f424.18a8; dossier ev 1e89.cf50.1beb.4190.0ee9.271a.63bd.b818.8332.0629.2c3b.fcf2.ddf2.c52f.114e.1795; incoming xrefs ev a9df.0ce7.7b93.e827.db43.91ba.4468.d98a.2709.65ca.173b.2241.5ebc.e6bc.996b.78c9 (direct call sites: 0x565629); caller facet ev 918b.9467.664a.8fde.3e63.5b75.e7a1.b7bc.538b.e94d.11cf.73bc.c7cd.d841.45c3.cce1; callee facet ev 8916.0cba.d502.9109.c2aa.5273.7b26.bb48.1898.869f.d718.6b51.6348.2dd4.eba3.0778; file-offset mapping ev 0f7d.f880.43a0.1bc7.320d.e779.0eff.ea45.de43.ba00.adc5.6166.e104.0eaf.a4ac.c1fc.
- **Our client**: No counterpart by VA; Rust `Vec::insert` in the gift codec path supplies the role.

### `0x56637e` — Finish the grow arm of the 360-byte insert (support)

- **VA / file offset**: `0x56637e` / `0x16637e`.
- **Behavior**: Grow-arm epilogue for the 360-byte insert: adds the old count to the end offset, frees the old allocation after a stride walk, and rewrites begin, end and capacity.
- **Reached from**: Only observed direct caller(s): 0x565ed0, the 360-byte insert..
- **Player-visible reach**: Support. Allocator epilogue of the 5018 staging insert.
- **Evidence**: analysis ev 4d1e.d103.9d0d.9597.e419.d4bb.a141.7add.1a88.e244.5e91.b1b0.d494.443c.0523.24b9; dossier ev a4c9.9378.e575.c4e5.99e1.6dcd.62c4.823a.9671.c95a.ec39.11b8.78ef.e5b9.02da.6eb0; incoming xrefs ev 02ff.e5b3.4a48.1372.1824.b3b6.1ac9.8f7c.e402.e401.96b3.9c4c.9f91.3b5a.c3bc.012e (direct call sites: 0x566303); caller facet ev 9f1e.54c2.e409.8cfb.381f.86a1.969b.329b.6122.a7d1.d3d3.9e83.8809.e845.9e10.9b9e; callee facet ev a089.72e1.d5a4.68fe.1348.2613.a190.5f0a.04d7.8133.72ae.e9cb.e1c3.c51b.5947.69f5; file-offset mapping ev 11ee.e536.9e07.6ca4.6977.c408.f384.0eeb.7c88.f698.34db.aa81.7fa8.8f40.26dc.7185.
- **Our client**: No counterpart by VA; Rust allocation handles the role.

### `0x5667e0` — Insert a run of 36-byte member records (support)

- **VA / file offset**: `0x5667e0` / `0x1667e0`.
- **Behavior**: Copies a 9-dword prototype, then inserts a counted run of 36-byte records with grow and in-place arms.
- **Reached from**: Only observed direct caller(s): 0x5656a0, the 36-byte vector insert caller..
- **Player-visible reach**: Support for member-list staging; no pixels or wire.
- **Evidence**: analysis ev 0869.2cb9.e8c4.c56d.077d.e35e.3ad7.7008.db13.8475.14ec.86a6.7767.854c.d787.9d1b; dossier ev d655.6b8a.cf68.1a8f.0d09.93c4.13f0.aa30.b2bd.ec7d.6953.da78.1f94.ebcc.6250.840a; incoming xrefs ev 7a26.8644.659b.205c.c6bd.d7b3.14c8.d4cb.c7b2.0759.dcda.e397.e323.c8f1.7af4.de97 (direct call sites: 0x565759); caller facet ev e119.4287.6bf2.b773.1194.0927.cd8b.2292.2d35.4185.b787.7a00.f21f.2502.6913.6627; callee facet ev b204.bef7.d94c.0108.0a1b.4ebd.ca59.6771.0f3a.8201.2247.6409.9956.4411.6be3.2f78; file-offset mapping ev 6c14.9080.4b71.5e95.d72d.cbf5.f4df.fc42.61f1.cdf5.58f2.5514.4b11.782b.5c24.8592.
- **Our client**: No counterpart by VA; Rust `Vec::insert` idiom.

### `0x566c37` — Finish the grow arm of the 36-byte insert (support)

- **VA / file offset**: `0x566c37` / `0x166c37`.
- **Behavior**: Grow-arm epilogue for the 36-byte insert, including the stride walk and free before pointer rewrite.
- **Reached from**: Only observed direct caller(s): 0x5667e0, the 36-byte insert..
- **Player-visible reach**: Support. Allocator epilogue of the 36-byte insert.
- **Evidence**: analysis ev 8732.4657.c56e.1db5.d603.5e98.151c.16ae.7d3a.9422.f106.dff9.0402.3424.0676.e374; dossier ev 4b5e.cd08.caec.2812.dc80.0122.5978.fd27.e283.924e.9be2.c6a5.5184.80f7.ba7d.e4c6; incoming xrefs ev 14a3.f152.af0b.281f.91ac.7a52.107e.f5e5.b349.6322.bef4.f3f1.f2a7.8a26.bc6f.6219 (direct call sites: 0x566bc8); caller facet ev 74ee.4162.4191.2c97.7664.8fc5.2ab9.3790.69a3.36aa.23dd.d603.8a1f.4448.c698.6900; callee facet ev 5001.b34f.6c34.a726.cdf5.fb89.dc4f.4fae.b312.cad2.196b.25f1.92e0.1315.cb42.7e78; file-offset mapping ev 8737.fdb6.eeed.1f9e.d057.2559.3725.91b7.505d.5641.5c91.f42d.02ed.4f4f.c5a4.b38e.
- **Our client**: No counterpart by VA; Rust allocation handles the role.

### `0x567040` — Insert a run of 56-byte records (support)

- **VA / file offset**: `0x567040` / `0x167040`.
- **Behavior**: Copies a 14-dword prototype, then inserts a counted run of 56-byte records with grow and in-place arms.
- **Reached from**: Only observed direct caller(s): 0x5657d0, the 56-byte vector insert caller..
- **Player-visible reach**: Support for a staged record family; no pixels or wire.
- **Evidence**: analysis ev 02d8.93b7.382e.5f0d.d647.c10e.5cfe.7809.0926.94a9.98e5.13c1.2c5b.fa88.b9da.fe21; dossier ev 39d3.6fd4.e376.b49d.6334.8510.c871.b909.d1d2.49a1.bc97.6c85.d434.e81a.42f2.92d8; incoming xrefs ev ef5a.f9de.127a.5c35.5216.8bfa.0d08.f8f7.c394.8bee.d4b2.618f.f63b.9716.6879.47c4 (direct call sites: 0x565889); caller facet ev 3a8a.accf.3209.1561.bc54.67b1.8edb.efe0.50a2.66d2.92b7.c9db.754f.89d6.0feb.3f5d; callee facet ev fdda.4322.0dbd.b481.8479.7308.d0e7.523b.f96a.d6f4.1b1e.cb82.44a8.3513.2a91.ab1d; file-offset mapping ev e815.b674.a00d.7d39.eb5c.9f6c.0763.a68a.9a0f.9c51.c8c4.6ea3.cd89.554c.5b20.dc62.
- **Our client**: No counterpart by VA; Rust `Vec::insert` idiom.

### `0x567497` — Finish the grow arm of the 56-byte insert (support)

- **VA / file offset**: `0x567497` / `0x167497`.
- **Behavior**: Grow-arm epilogue for the 56-byte insert, including the stride walk and free before pointer rewrite.
- **Reached from**: Only observed direct caller(s): 0x567040, the 56-byte insert..
- **Player-visible reach**: Support. Allocator epilogue of the 56-byte insert.
- **Evidence**: analysis ev dbdb.1055.4828.4f68.018d.0846.2bb6.6501.0cd5.ebc7.ba07.09e8.8c8d.ea7f.c2bc.e635; dossier ev 56af.c7f8.9ea6.0377.813b.868b.4a7d.50e4.9e72.f07b.5259.954a.f3e4.1dd0.bf87.669f; incoming xrefs ev 6cf0.91e6.d8c5.ad11.eb7d.24b3.270a.5cb2.825e.aee3.e8b7.55b7.e21c.09ac.af59.eb96 (direct call sites: 0x567428); caller facet ev 5feb.f4e5.0687.9447.55ee.5fe0.c82f.cb49.9e72.963d.025e.0c15.a7d1.063e.cc3b.0f90; callee facet ev 512c.5e53.c4a6.306d.1ca4.8d54.534f.316e.6c5d.b030.5c04.3ed3.9c7f.845e.7a79.f92b; file-offset mapping ev 260a.37e3.a562.e87d.b5cf.4ee4.2333.93f4.c55a.041f.209f.f97d.e48c.4597.9dec.406d.
- **Our client**: No counterpart by VA; Rust allocation handles the role.

### `0x5678a0` — Insert a run of 12-byte records (support)

- **VA / file offset**: `0x5678a0` / `0x1678a0`.
- **Behavior**: Copies a 3-dword prototype, then inserts a counted run of 12-byte records with grow and in-place arms.
- **Reached from**: Only observed direct caller(s): 0x565900, the 12-byte vector insert caller..
- **Player-visible reach**: Support for a staged record family; no pixels or wire.
- **Evidence**: analysis ev 43a1.6442.5bf1.a574.114b.5b3a.4213.1723.5a2a.f28a.ca2b.2ab3.9dd5.518b.05dd.81cf; dossier ev c6b0.7f1a.e974.6f84.4b5f.02f3.e24c.d5e5.e73a.9cb8.63cf.23d4.d9e0.0391.e2cc.2a64; incoming xrefs ev ac34.a77d.98b1.b626.feb7.1d09.c1b2.e3ba.3959.ceb5.fe5f.efcf.b40f.0f47.899d.0345 (direct call sites: 0x5659b9); caller facet ev 4aa6.3073.4907.c933.7ab9.1c71.ca71.d9fe.30bd.c15e.a7c0.be8c.a74d.38df.186e.d857; callee facet ev 8a85.7d74.ed5a.8c81.42a1.b435.6b1d.ce8f.989d.36ee.b12a.af84.aad5.d4e0.2df9.16d1; file-offset mapping ev b414.3a35.9d52.8e39.0f87.4d37.2ad4.0ded.4ba9.8b1a.3302.680f.3bc3.e198.005d.5608.
- **Our client**: No counterpart by VA; Rust `Vec::insert` idiom.

### `0x567ce2` — Finish the grow arm of the 12-byte insert (support)

- **VA / file offset**: `0x567ce2` / `0x167ce2`.
- **Behavior**: Grow-arm epilogue for the 12-byte insert, including the stride walk and free before pointer rewrite.
- **Reached from**: Only observed direct caller(s): 0x5678a0, the 12-byte insert..
- **Player-visible reach**: Support. Allocator epilogue of the 12-byte insert.
- **Evidence**: analysis ev 40e9.6357.c6a9.7ab7.29de.73ab.8308.a7b1.58ea.eeaa.32d0.04f9.95cf.89ec.ea4e.046b; dossier ev ebd7.db08.580e.b535.3ae7.1188.bb5b.c9f9.ba9a.4adc.2d39.51e5.0ca6.e759.b0cf.ec13; incoming xrefs ev 84ef.6153.dc7d.92f6.f36a.81f7.93dc.dde5.a31e.a494.a55d.c693.0128.9b1b.3590.6237 (direct call sites: 0x567c73); caller facet ev 36c3.885e.b8fb.07b7.bf69.94f3.2eba.3662.78a3.3e0b.223a.c9c3.e4d9.92bd.7c6f.8d8f; callee facet ev 23fe.a1cd.8028.1921.1f3f.8958.700c.a9fc.dfe4.188e.e1e3.3ac5.c47a.9ee2.0666.6179; file-offset mapping ev cb1f.ecdc.adc3.609a.fe71.3930.6d41.3864.cc6f.0bce.4880.af9b.7452.9652.a30b.afda.
- **Our client**: No counterpart by VA; Rust allocation handles the role.

### `0x5680f0` — Insert a run of 292-byte records (support)

- **VA / file offset**: `0x5680f0` / `0x1680f0`.
- **Behavior**: Copies a 73-dword prototype, then inserts a counted run of 292-byte records with grow and in-place arms.
- **Reached from**: Only observed direct caller(s): 0x565a30, the 292-byte vector insert caller..
- **Player-visible reach**: Support for a staged record family; no pixels or wire.
- **Evidence**: analysis ev 637c.7c11.f632.ca95.ef1d.98c3.3b3c.606c.1f7a.c044.ad84.cbbd.edb1.f450.88ec.d8ad; dossier ev a8cc.7a9a.5976.8009.ecb7.c61f.563f.b68b.c333.1e65.62ca.b41e.4750.12ed.ff49.79c1; incoming xrefs ev 7fff.5dbc.4466.64e6.27ed.874e.bcb3.5cb9.c272.2a60.be88.86fb.93f1.2c52.bce1.6a9e (direct call sites: 0x565ae9); caller facet ev 2536.f266.09a2.67ba.bc4c.b827.4ef4.0f13.30da.fc7a.4484.caff.c986.d51f.1ce5.5a23; callee facet ev 08e2.6fb2.2806.f969.9617.9816.4f19.b921.71ab.9e56.d63b.abeb.e6d5.c928.2fae.f022; file-offset mapping ev 7d13.b278.5be1.e849.2d3a.a672.fea7.a260.7a6c.47d8.81c6.c13b.a172.1c58.f921.1b9f.
- **Our client**: No counterpart by VA; Rust `Vec::insert` idiom.

### `0x56859e` — Finish the grow arm of the 292-byte insert (support)

- **VA / file offset**: `0x56859e` / `0x16859e`.
- **Behavior**: Grow-arm epilogue for the 292-byte insert, including the stride walk and free before pointer rewrite.
- **Reached from**: Only observed direct caller(s): 0x5680f0, the 292-byte insert..
- **Player-visible reach**: Support. Allocator epilogue of the 292-byte insert.
- **Evidence**: analysis ev 5266.c653.01d6.ba19.dc5b.8d33.d93d.4e6c.6b08.d0ac.9517.e9c8.547b.8107.f884.6712; dossier ev 9e84.9713.bfc4.41f0.7954.017e.d4a2.03e1.2ce4.eab0.5b0a.b488.6a88.48ad.4ac2.4afe; incoming xrefs ev dd37.a403.3e57.3d32.4fc6.4b2e.ad7c.a01c.5dc6.c376.69ca.55d4.585c.ca3d.aaee.2c2a (direct call sites: 0x568523); caller facet ev 354e.182c.513f.55cf.4d3e.37e6.c174.717e.f639.1fbc.56bf.f7c4.467e.6d3a.0744.311e; callee facet ev c27c.70cf.5556.5ecf.c822.f376.9788.c3b2.7246.ed67.daf0.9a75.a3c6.a9bb.534f.1193; file-offset mapping ev e750.2750.14c6.bf6d.1eca.dbbb.38c3.f8ed.d1d8.8545.4836.f38e.2847.e123.b13e.70f3.
- **Our client**: No counterpart by VA; Rust allocation handles the role.

### `0x568e1f` — Finish the grow arm of the eight-byte insert (support)

- **VA / file offset**: `0x568e1f` / `0x168e1f`.
- **Behavior**: Grow-arm epilogue for the eight-byte insert: adds the old count, frees the old block and rewrites the three pointer words.
- **Reached from**: Only observed direct caller(s): 0x568a00, the eight-byte insert..
- **Player-visible reach**: Support. Allocator epilogue of the eight-byte insert.
- **Evidence**: analysis ev 24ac.4ed1.b311.9df0.30e5.2bc0.0d77.d149.2b7b.74a4.2d0e.83ef.b0c8.623c.02f8.f363; dossier ev 2aed.1ce0.ee09.5966.df20.5a05.312b.032b.4ef6.4bf3.2082.d7d5.1ebd.8612.d098.3117; incoming xrefs ev 40fc.35f8.b7fd.9406.4e16.3f73.8144.8af5.b112.bcac.60c3.9ea1.e746.93ec.ada0.73a9 (direct call sites: 0x568db0); caller facet ev e209.a7f0.dc6f.c941.bc61.d2f4.918b.bedf.eece.e7db.1112.2c2d.c476.74cd.1dc7.ee66; callee facet ev ea9c.3aa2.83c3.4d09.855b.4935.7df3.ec8e.8e8a.d8be.2472.f81c.c4e1.291b.71bc.b767; file-offset mapping ev ae42.05e9.ecff.43bf.0054.e175.b392.5c62.5357.0ce7.8eec.f2dd.113a.1812.ad20.6426.
- **Our client**: No counterpart by VA; Rust allocation handles the role.

### `0x569580` — Copy a range of eight-byte records (support)

- **VA / file offset**: `0x569580` / `0x169580`.
- **Behavior**: Copies a source range of eight-byte records into a destination and returns the destination advanced past the written run.
- **Reached from**: Only observed direct caller(s): 0x565c80, a shared container caller..
- **Player-visible reach**: Support. Uninitialized-copy template; no player-visible decision.
- **Evidence**: analysis ev 66c8.2489.5e36.033c.2b16.9b25.edd6.6963.40ba.28d5.1ea4.4547.06ac.4104.2058.b399; dossier ev 71ba.623b.31ef.a4e7.ba0c.696e.0132.27c5.f00c.bfde.0c34.426b.feac.beee.4371.0430; incoming xrefs ev f3e6.4077.df68.e697.05fd.873a.b4e3.4b2d.3197.164f.de1d.1666.e019.a32a.c529.9c81 (direct call sites: 0x565cc7); caller facet ev fa6f.e92d.34f8.0dd3.5060.dbfa.1e22.87ff.72ff.6b01.b21f.de57.f665.d728.bf90.eb5d; callee facet ev bde7.ab7d.7322.c2d1.d589.04b7.ef67.7497.5623.1564.2cd2.b432.b3b2.c293.f7f6.efd7; file-offset mapping ev 789e.8fbd.da84.2d27.3298.e505.098a.93cd.a9dd.6abe.f8fa.b17f.ee26.f526.2707.6699.
- **Our client**: No counterpart by VA; Rust `copy_from_slice` idiom.

### `0x569860` — Forward four arguments to the 360-byte insert tail (support)

- **VA / file offset**: `0x569860` / `0x169860`.
- **Behavior**: Forwards its second, fourth, fifth and sixth arguments to the 360-byte insert tail at 0x56a7e0.
- **Reached from**: Only observed direct caller(s): 0x564040, the 360-byte reserve wrapper..
- **Player-visible reach**: Support. Thin forwarding wrapper.
- **Evidence**: analysis ev 7d57.d385.10bc.cd78.745f.d237.81d2.7caa.4b01.fa44.f573.56aa.592f.82b2.f401.6645; dossier ev 38f9.8f96.ce58.429f.d804.911f.8ce3.2339.3dcc.e225.e952.fff9.5bbc.2b45.9d04.ba1e; incoming xrefs ev 940f.65cc.bd6a.0ebf.d308.45c0.f6d3.ee28.a924.b5be.d475.53b8.2402.6be4.12aa.4b41 (direct call sites: 0x564206); caller facet ev 5387.8045.45ec.5979.ccb7.5168.5e80.7a54.6442.2273.c5cc.693c.a1e9.115a.4467.5f5d; callee facet ev 361f.5628.51ad.d6bd.7731.1144.a13e.5426.9a04.602e.8022.1342.6ddb.27cf.1116.b7ec; file-offset mapping ev 9b20.9efe.039d.7bbc.1748.a5b6.00f8.9ff1.63bc.4752.54b4.7fd5.7d3b.536f.8941.55f0.
- **Our client**: No counterpart by VA; Rust slice fill handles the role.

### `0x5698e0` — Forward four arguments to the 36-byte insert tail (support)

- **VA / file offset**: `0x5698e0` / `0x1698e0`.
- **Behavior**: Forwards the same four arguments to the 36-byte insert tail at 0x56a8c0.
- **Reached from**: Only observed direct caller(s): 0x5644a0, the 36-byte reserve wrapper..
- **Player-visible reach**: Support. Thin forwarding wrapper.
- **Evidence**: analysis ev 8a6d.e74b.3108.6f13.edc9.ebd9.f933.80d5.0132.52b8.44a8.745d.9955.9f30.06a9.35fa; dossier ev db08.0af6.64dd.0728.ccdb.a865.138a.c793.258c.7dda.4706.ab0f.d65c.a18b.5da4.198e; incoming xrefs ev 3f67.2443.38bf.f1c6.c7d9.b693.ea7b.d1df.37f9.5a72.d85d.f114.ea25.de08.72e6.877b (direct call sites: 0x564666); caller facet ev e4d7.9aa1.4c5c.cfd5.1487.7d8e.dfdd.fb35.628c.159c.4595.8284.b268.48ee.1a64.66c5; callee facet ev 16ee.e2a4.6ab0.33f1.ef3b.a7f0.f837.0ea1.6baf.b12f.dfe6.d635.b8b0.43f6.0655.4307; file-offset mapping ev 315c.b5c3.54fa.5d13.8a80.db69.e74b.9e72.e7a1.5b52.87cf.079b.9967.cccb.4cae.95bb.
- **Our client**: No counterpart by VA; Rust slice fill handles the role.

### `0x569960` — Forward four arguments to the 56-byte insert tail (support)

- **VA / file offset**: `0x569960` / `0x169960`.
- **Behavior**: Forwards the same four arguments to the 56-byte insert tail at 0x56a990.
- **Reached from**: Only observed direct caller(s): 0x5648f0, the 56-byte reserve wrapper..
- **Player-visible reach**: Support. Thin forwarding wrapper.
- **Evidence**: analysis ev a3e0.cb08.09e1.a6a3.15b6.fe80.1c83.6616.5887.6b1e.9c4d.a60b.52d4.ae3d.58fd.55ad; dossier ev c503.f422.c6a8.8791.1cda.709b.5408.7997.809e.958d.bfee.ae6f.1f31.6435.cb62.d10d; incoming xrefs ev ab8a.d545.7bd2.4981.c0e0.be1c.f47b.010f.9ad4.98e9.a2ce.5796.fff3.525f.634d.2991 (direct call sites: 0x564ab6); caller facet ev 0752.f20f.e20e.0b12.046b.b5c0.a272.45c4.05fc.a6a2.3477.20bb.3f2a.0e7b.7f3e.a006; callee facet ev 594e.6e99.fb6a.3a6d.4682.eb3f.fa16.a0d8.492d.a5b0.046b.5e9d.fccd.655e.e3ac.7492; file-offset mapping ev 1489.232f.44a2.e36a.c581.5ac5.7c71.4781.2c1b.092b.9b2b.436e.4a8b.03a5.3934.30bd.
- **Our client**: No counterpart by VA; Rust slice fill handles the role.

### `0x5699e0` — Forward four arguments to the 12-byte insert tail (support)

- **VA / file offset**: `0x5699e0` / `0x1699e0`.
- **Behavior**: Forwards the same four arguments to the 12-byte insert tail at 0x56aa60.
- **Reached from**: Only observed direct caller(s): 0x564de0, the 12-byte reserve wrapper..
- **Player-visible reach**: Support. Thin forwarding wrapper.
- **Evidence**: analysis ev 9258.9854.28fc.fa3a.f575.3978.1ce7.ecb2.193a.1428.2072.15c6.5bb0.5a2a.ab19.d897; dossier ev 9ebe.397e.919b.d2d3.68bd.df14.7a1f.ee12.da0e.8fa6.60e6.7994.27c0.b68e.7a6b.8497; incoming xrefs ev f68c.77be.16d3.fd4a.576e.ea82.4fb6.f3e6.3d37.a323.21e3.b93c.0f36.31f7.f236.463d (direct call sites: 0x564fa6); caller facet ev eec8.8e0a.d145.6699.9159.3978.3316.8107.fc7b.69eb.75d4.64eb.ee5c.41b1.a85d.33bf; callee facet ev 7e59.96ee.9297.e75f.2918.a424.a45a.a9ff.a22a.fe82.3d00.8ab4.dc76.c19e.b5de.08ff; file-offset mapping ev c9dd.b3ea.cae9.7777.a75b.64f0.36cf.d48c.e9ff.ee04.a0b3.4d8a.3c90.22d2.87e4.130e.
- **Our client**: No counterpart by VA; Rust slice fill handles the role.

### `0x569a60` — Fill a counted run of 360-byte records (support)

- **VA / file offset**: `0x569a60` / `0x169a60`.
- **Behavior**: Repeats a 90-dword prototype into a counted run of 360-byte destinations, skipping null destinations, then clears its unwind frame.
- **Reached from**: Only observed direct caller(s): 0x564390 (5018 staging) and 0x565ed0 (insert)..
- **Player-visible reach**: Support. Fill step of the 5018 staging insert; no pixels or wire.
- **Evidence**: analysis ev ef38.5f2c.fcca.068f.f25a.7121.ebc3.83b0.25e8.c66b.66ac.c285.d0fd.a8ea.7525.9d05; dossier ev f6ba.0741.e350.2c70.95a2.3e06.56e7.a9a7.1ee4.c8ce.8d76.a9cc.75cd.7ff5.2ad2.70d1; incoming xrefs ev 148c.7594.9b91.d428.1129.e4e9.ddcf.0e61.1737.5ab9.6d4d.ab5c.b8e5.57df.3311.c003 (direct call sites: 0x564423, 0x56624e, 0x5665a3); caller facet ev 9337.d10b.aa82.3b17.17fa.c170.c19b.c98b.b49c.9950.1a71.da2e.14c4.0c39.c1f9.7063; callee facet ev 212f.c027.4a50.1a1e.1135.a890.b795.33fc.6a8c.b096.82ef.e888.7588.6e39.94c2.d58d; file-offset mapping ev 0ca0.e6b9.93ea.c007.ed15.4865.c0e1.c7b2.dca8.b9fe.86f5.1de9.3ee9.ddcc.8e15.b531.
- **Our client**: No counterpart by VA; Rust repeat/fill idiom.

### `0x569b15` — Unwind fragment after the 360-byte fill (support)

- **VA / file offset**: `0x569b15` / `0x169b15`.
- **Behavior**: Clears the local unwind marker and restores the saved exception-list head before returning.
- **Reached from**: Only observed direct caller(s): 0x569a60, the 360-byte fill..
- **Player-visible reach**: Support. Compiler-generated exception cleanup.
- **Evidence**: analysis ev fc78.59f4.ac94.94bd.476d.0d27.a667.0227.ee4f.e3cf.a3df.aed2.06c9.196b.1534.5a4b; dossier ev 9f1d.27b2.d4ea.a590.9fbf.ea42.c3a1.d2b4.4c0e.83c7.4e05.1beb.afc3.fc5f.40fe.1699; incoming xrefs ev eb5f.0ff5.c3fb.ad3f.ad85.862c.5e41.dfce.bd52.a0a1.dadd.0820.563d.1518.b960.8382 (direct call sites: 0x569ae6); caller facet ev 3bce.4e4f.8d97.9112.ae1b.61b5.8e7c.7b69.d456.6624.a871.0901.cf40.2b18.a3d1.fe5c; callee facet ev 6e44.0bd0.af01.4c45.c04f.e543.4b87.3b00.1a4a.e726.8d0a.704a.06e4.c41d.6b52.747f; file-offset mapping ev 3d47.abe2.f82b.c824.d845.38f2.7ad3.8e8f.1ca8.ca12.8c43.60eb.e48f.dc1c.48d2.8af5.
- **Our client**: No counterpart by VA; Rust unwinding is language-level.

### `0x569b30` — Fill a counted run of 36-byte records (support)

- **VA / file offset**: `0x569b30` / `0x169b30`.
- **Behavior**: Repeats a 9-dword prototype into a counted run of 36-byte destinations, then clears its unwind frame.
- **Reached from**: Only observed direct caller(s): 0x5667e0 (insert) and 0x5647e0..
- **Player-visible reach**: Support. Fill step of the 36-byte insert; no pixels or wire.
- **Evidence**: analysis ev d2e9.40c5.3eaf.5420.0cb0.0757.bf74.795e.ce45.1ba1.8b27.f63f.bfb2.fa17.1f85.d475; dossier ev 40ac.75e6.62d8.7749.c475.970d.f6c0.7929.87e7.a6ab.6864.0499.fad7.3160.2159.ced0; incoming xrefs ev 1b47.a25c.3bf6.9947.f7ac.44ac.d0ef.1b83.07bf.e283.1b8c.15aa.49da.3b04.8bc2.f488 (direct call sites: 0x564873, 0x566b1f, 0x566e42); caller facet ev 983e.7485.f73d.b0fc.1914.2095.9345.5b51.95c8.16f4.18a1.651f.5ece.42d4.2215.39ed; callee facet ev d3ce.5c29.b071.780e.8e07.1bb9.314e.f9d1.66ff.8d5c.3c7e.b16c.0a05.3fa5.9cf5.109c; file-offset mapping ev 5ef6.1da1.f87e.c9e3.c536.f8a0.e3c9.1c06.95f4.a209.1962.880d.dab7.9665.95f2.6e3c.
- **Our client**: No counterpart by VA; Rust repeat/fill idiom.
