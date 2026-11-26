# Statics pins — Playing + Result unpinned functions (coverage burn-down 3)

Version pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`. Provider: REA 6.10.1 / Ghidra 12.2; target bound from `install-root/O2Jam/OTwo.exe`.
Branch `cb2/pins-result-playing`; base commit `b872971` (ancestor of this branch's `ed556bf`).

Scope: the 19 rows in `procedures.csv` with `bucket=game-reachable`,
`pinned_in_repo=no`, and subsystem `Playing / state lifecycle` (4) or
`Result / records` (15) in `triage-final.json`. No row remains unpinned in
this bucket.

Evidence notation: an evidence ID is written as `ev ` followed by its hex
in four-digit groups separated by dots. Remove the dots and join after
`ev_` to obtain the full REA ID, for example `ev f697.0e65.6ae8.8dec.149a.c30e.9ebe.b33f.ef65.b4df.94c9.7b7a.bffe.9fef.5195.c421`.
The SHA-256 above uses the same grouping. Grouping keeps
`scripts/check-cited-hashes.sh` from treating evidence hex as a commit citation.

Prior pins are cited, not re-derived: `docs/parity/playing.md` (highway,
input, clock), `docs/parity/result.md` (grade chooser `0x4c4dd0` and digit
anchors), `docs/gameplay.md` (judgement, score and life rules),
`docs/lifecycle.md` §10 (Playing construction sequence), and the Playing /
Result sections of `docs/interaction-census.md`.

## Inventory

| VA | File offset | Triage bucket | Role | REA evidence |
|---|---|---|---|---|
| `0x489df0` | `0x89df0` | Playing lifecycle | Shared state base construction | ev 231b.fdc4.9a21.1e31.9710.b0ee.3a07.423f.fdaa.1c1f.c1ce.017a.1a0a.1939.6146.ba9e |
| `0x4d8940` | `0xd8940` | Playing lifecycle | Playing game object construction | ev 8ad9.b71b.79a2.33c8.88ac.4fef.c859.04b1.ae34.85d1.67ed.893f.af6a.0cbd.c2d3.e8b7 |
| `0x4e4760` | `0xe4760` | Playing lifecycle | Live Playing constructor tail | ev 250d.5016.eed6.7bac.0a26.ffd2.86a5.393d.4f0b.1138.bd32.c807.60d3.3c8a.d2f0.cc75 |
| `0x4eb860` | `0xeb860` | Playing lifecycle | Normal Playing constructor tail | ev 0046.1bd0.9aab.7515.b1d2.61b8.010a.4e90.1796.3102.9c71.a701.2168.46cb.c07f.cc2d |
| `0x4c0b00` | `0xc0b00` | Result / records (cluster label) | Insert N eight-byte records into a vector | ev f697.0e65.6ae8.8dec.149a.c30e.9ebe.b33f.ef65.b4df.94c9.7b7a.bffe.9fef.5195.c421 |
| `0x4c0f1f` | `0xc0f1f` | Result / records (cluster label) | Eight-byte insert epilogue | ev 2002.5ae6.b279.1172.fcb9.d0e2.e3f5.8230.096a.3faa.f60d.fdaa.20f9.80de.2824.594a |
| `0x4c1410` | `0xc1410` | Result / records (cluster label) | Fill eight-byte record slots | ev 364b.7fe4.693a.51ac.3bf1.2f16.1c3f.ea0b.0b1d.0099.e280.fb1f.ebf7.481c.054a.6935 |
| `0x4c14c3` | `0xc14c3` | Result / records (cluster label) | Eight-byte fill unwind stub | ev 8b6a.676e.1c6e.33c4.ca67.580a.dfbf.6d5d.ef6e.9ed0.803d.ddf0.b40a.0820.c40e.7dd0 |
| `0x4c15f0` | `0xc15f0` | Result / records (cluster label) | Eight-byte copy wrapper | ev bc1c.1860.ea52.bca9.d3eb.be8e.31de.16a7.b5ad.8b57.5ff4.2e09.1f0c.8e0d.f9dc.ae9d |
| `0x4c1640` | `0xc1640` | Result / records (cluster label) | Copy a range of eight-byte records | ev 9181.e4aa.cc6f.af37.9ec3.afa5.3420.b860.7596.8960.e232.c322.749f.8d19.bdb9.9705 |
| `0x4c16f5` | `0xc16f5` | Result / records (cluster label) | Eight-byte copy unwind stub | ev 8a3d.c9c5.829b.30b9.e945.7835.7f63.8127.1802.98bf.14aa.1d41.e669.cf27.8133.8468 |
| `0x4c5940` | `0xc5940` | Result / records | Insert N six-byte grade/wire records | ev f95a.05cd.0c30.aff1.0e62.cd05.41c3.7367.522e.b2cc.a3ea.d687.cfbf.3350.7052.e325 |
| `0x4c5d7f` | `0xc5d7f` | Result / records | Six-byte insert epilogue | ev 5605.4439.bf32.d430.fdc9.1358.15a9.6dbe.d3c8.feea.a6f1.bc2c.edc0.b0a2.6197.775f |
| `0x4c6290` | `0xc6290` | Result / records | Fill six-byte records | ev 49d4.1e05.6184.7b83.5656.7f1a.13e4.c860.855e.faed.1296.6bca.1ad5.5e8b.8806.be82 |
| `0x4c6345` | `0xc6345` | Result / records | Six-byte fill unwind stub | ev 2a6f.81ef.c6fe.27be.5cc1.1954.a8ea.8689.865f.51b3.bd45.66aa.8bbd.8386.c8ce.0ddc |
| `0x4c6480` | `0xc6480` | Result / records | Six-byte copy wrapper | ev a0cc.edd8.57df.5b9a.1410.786d.4447.f085.85a0.ffaa.2cf1.33b0.3d97.a6f9.e2a8.beee |
| `0x4c64d0` | `0xc64d0` | Result / records | Copy a range of six-byte records | ev eaa8.8e8f.a518.d572.f13d.aca8.df32.f251.6736.82bc.07f8.8726.97d0.dccf.305c.bd5a |
| `0x4c6587` | `0xc6587` | Result / records | Six-byte copy unwind stub | ev b891.6e40.e1a3.c9d2.984f.5323.b976.beb4.65af.ace2.aef6.fff2.a3f0.6b42.21bf.c6e0 |
| `0x4c6ba0` | `0xc6ba0` | Result / records (triaged; actually Music Shop) | Music Shop state init helper | ev f38e.498f.461c.9e79.c733.c5eb.efd3.4576.c8e7.1f66.ee22.cfdc.e826.566c.9e05.cdbf |

## Entries

### `0x489df0` — Playing lifecycle

- VA / file offset: `0x489df0` / `0x89df0`.
- Behavior (own words): Runs the common state-object setup: installs the structured exception frame, calls the shared state initializer, registers the replacement window procedure through the input hook helper, writes the CStateBase vftable at two object slots and finishes with a second shared initializer.
- Reach and player-visible role: Support. Base constructor reused by Playing, Music Shop and other states; it establishes the window input route but does not itself judge a key or paint a pixel. The Playing key-down/key-up rows in `docs/interaction-census.md` stay OPEN.
- Our status: `crates/o2jam-shell/src/screen.rs:32` names the Playing screen; no constructor-level Rust counterpart.
- Gameplay-rule note: The window-procedure registration is shared plumbing. It does not change the verified judgement or life rules in `docs/gameplay.md`; no gameplay-rule implication to apply.
- Evidence: analysis ev 231b.fdc4.9a21.1e31.9710.b0ee.3a07.423f.fdaa.1c1f.c1ce.017a.1a0a.1939.6146.ba9e; incoming call sites ev 10fd.c891.36ee.28ca.a914.1579.18b9.affa.272d.6dce.738a.0b45.15d5.583b.073d.3e82 (observed direct callers: 0x48db90, 0x48e700, 0x497f60, 0x4a3560, 0x4b5d10, 0x4c1730, 0x4c6760, 0x4d8940, 0x4f33f0, 0x4f45e0, 0x4f5630). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4d8940` — Playing lifecycle

- VA / file offset: `0x4d8940` / `0xd8940`.
- Behavior (own words): Builds the object used by Normal, Live and Tutorial play: calls the state base constructor, installs the CPlayingGame vftable and a message-buffer queue vftable, runs four playback setup helpers, clears per-round arrays and counters, seeds two words with 315, writes 375 and 400 defaults, copies the live-play global into a mode word and sets initial flags.
- Reach and player-visible role: Support with player-visible consequence: every new song starts from these cleared per-round fields before the verified scorer/life path in `docs/gameplay.md` runs. Called by constructors `0x4eb610` (Normal), `0x4e4670` (Live) and `0x4f07f0` (Tutorial), all reached from the state factory `0x477990`.
- Our status: `crates/o2jam-shell/src/screen.rs:32` models the screens; `o2jam-game::Score`/`LifeBar` implement the judged scoring path, not these state fields.
- Gameplay-rule note: No conflict with the pinned Cool/Good/Bad/Miss score and life deltas: those live on the score sub-object and are driven by scorer `0x570510`; this constructor only clears state-local fields and sets presentation/mode defaults. Treat the 315/375/400 words as pinned state defaults, not as new gameplay rules.
- Evidence: analysis ev 8ad9.b71b.79a2.33c8.88ac.4fef.c859.04b1.ae34.85d1.67ed.893f.af6a.0cbd.c2d3.e8b7; incoming call sites ev 877a.0d30.54d7.7444.8ec9.1fe1.da66.eed3.e903.3a41.d0ec.ccb7.19e8.b075.113a.014d (observed direct callers: 0x4e4670, 0x4eb610, 0x4f07f0). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4e4760` — Playing lifecycle

- VA / file offset: `0x4e4760` / `0xe4760`.
- Behavior (own words): Clears two dwords and two words at fixed offsets on a freshly built Live Playing object.
- Reach and player-visible role: Support; sole caller is the `CStateLivePlaying` constructor `0x4e4670`, reached from the state factory. No direct census row.
- Our status: No counterpart; the shell enters `Screen::Playing` without exe object-field state.
- Gameplay-rule note: The cleared words are per-state fields, not the life or score words used by `docs/gameplay.md`; no rule change.
- Evidence: analysis ev 250d.5016.eed6.7bac.0a26.ffd2.86a5.393d.4f0b.1138.bd32.c807.60d3.3c8a.d2f0.cc75; incoming call sites ev f514.f18c.0a58.3ca1.d72e.adc0.db9b.7666.5b2a.1a05.d688.18ee.9d21.3209.99a1.ac6c (observed direct callers: 0x4e4670). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4eb860` — Playing lifecycle

- VA / file offset: `0x4eb860` / `0xeb860`.
- Behavior (own words): Clears three words at fixed offsets on a freshly built Normal Playing object.
- Reach and player-visible role: Support; sole caller is the `CStateNormalPlaying` constructor `0x4eb610`, reached from the state factory. No direct census row.
- Our status: No counterpart; shell play state is built in `o2jam-shell` scene code.
- Gameplay-rule note: Same field class as the Live tail; no effect on pinned judgement/life semantics.
- Evidence: analysis ev 0046.1bd0.9aab.7515.b1d2.61b8.010a.4e90.1796.3102.9c71.a701.2168.46cb.c07f.cc2d; incoming call sites ev 02c2.61b0.b398.23a2.1745.7a9e.641c.8381.55a2.109c.3391.af5c.8456.86c1.6d74.9e88 (observed direct callers: 0x4eb610). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c0b00` — Result / records (cluster label)

- VA / file offset: `0x4c0b00` / `0xc0b00`.
- Behavior (own words): Grows a vector of eight-byte records when needed, fills the new region with a value pair, copies the original tail and moves the end pointer. Capacity growth, fill and copy are delegated to nearby helpers.
- Reach and player-visible role: Support. Reached through insert caller `0x4c0830` from the eight-byte push-back wrapper `0x4c0730`, whose callers include Waiting-side setup `0x4bc310` and net handlers; the triage Result label is an address-cluster attribution, not a Result-screen behavior.
- Our status: No counterpart; our shell has no vector-of-records mirror.
- Gameplay-rule note: Not a Result record writer. No census row closes on this pin alone.
- Evidence: analysis ev f697.0e65.6ae8.8dec.149a.c30e.9ebe.b33f.ef65.b4df.94c9.7b7a.bffe.9fef.5195.c421; incoming call sites ev 75a3.2690.56b7.f6bb.9e19.060a.7b5f.d67b.b97b.6cdd.f27c.eb15.c0f3.ef62.e1a9.6740 (observed direct callers: 0x4c0830). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c0f1f` — Result / records (cluster label)

- VA / file offset: `0x4c0f1f` / `0xc0f1f`.
- Behavior (own words): Terminal cleanup of the insert path: advances the stored end pointer, walks and frees the old allocation when one existed, then installs the new begin/end/capacity bookkeeping.
- Reach and player-visible role: Support; terminal call from `0x4c0b00`. No player-visible row.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 2002.5ae6.b279.1172.fcb9.d0e2.e3f5.8230.096a.3faa.f60d.fdaa.20f9.80de.2824.594a; incoming call sites ev 2dfa.67ef.a4cd.f350.bbc4.95d9.2a03.8deb.e5cf.15dd.0198.5870.cd99.41e4.5fce.d186 (observed direct callers: 0x4c0b00). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c1410` — Result / records (cluster label)

- VA / file offset: `0x4c1410` / `0xc1410`.
- Behavior (own words): Copies a two-word value into N consecutive eight-byte slots, then leaves through the small exception-unwind stub.
- Reach and player-visible role: Support; called by the eight-byte push-back wrapper `0x4c0730` and by insert `0x4c0b00`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 364b.7fe4.693a.51ac.3bf1.2f16.1c3f.ea0b.0b1d.0099.e280.fb1f.ebf7.481c.054a.6935; incoming call sites ev d892.8bf6.a071.0cce.6fae.f63f.bbc0.574a.cd42.6de1.ddf6.8f82.010c.41ff.6031.eb25 (observed direct callers: 0x4c0730, 0x4c0b00). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c14c3` — Result / records (cluster label)

- VA / file offset: `0x4c14c3` / `0xc14c3`.
- Behavior (own words): Tiny cleanup fragment that resets the unwind marker and restores the exception list for its caller.
- Reach and player-visible role: Support; sole caller is the eight-byte fill helper `0x4c1410`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 8b6a.676e.1c6e.33c4.ca67.580a.dfbf.6d5d.ef6e.9ed0.803d.ddf0.b40a.0820.c40e.7dd0; incoming call sites ev 2292.6545.dd42.d318.6490.7ef3.5f18.ab1c.73e5.1214.38af.1acb.d6fc.bca7.c931.f407 (observed direct callers: 0x4c1410). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c15f0` — Result / records (cluster label)

- VA / file offset: `0x4c15f0` / `0xc15f0`.
- Behavior (own words): Forwards its arguments unchanged to the range-copy helper.
- Reach and player-visible role: Support; called by insert `0x4c0b00`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev bc1c.1860.ea52.bca9.d3eb.be8e.31de.16a7.b5ad.8b57.5ff4.2e09.1f0c.8e0d.f9dc.ae9d; incoming call sites ev cf26.323a.8969.d36f.3a50.1f5d.1377.9a48.b1d3.faaa.98a0.b575.279d.877d.f4ab.c64d (observed direct callers: 0x4c0b00). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c1640` — Result / records (cluster label)

- VA / file offset: `0x4c1640` / `0xc1640`.
- Behavior (own words): Walks source up to a destination pointer, copies each two-word record when the target is non-null, then leaves through its unwind stub.
- Reach and player-visible role: Support; reached from `0x4c14e0` and the forwarding wrapper `0x4c15f0`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 9181.e4aa.cc6f.af37.9ec3.afa5.3420.b860.7596.8960.e232.c322.749f.8d19.bdb9.9705; incoming call sites ev 66f7.7ec3.d07d.c992.2435.500f.f772.db08.2398.1432.7c56.eb63.2435.5c22.533f.4f5d (observed direct callers: 0x4c14e0, 0x4c15f0). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c16f5` — Result / records (cluster label)

- VA / file offset: `0x4c16f5` / `0xc16f5`.
- Behavior (own words): Tiny cleanup fragment that restores the exception list and returns the caller-held result.
- Reach and player-visible role: Support; sole caller is range-copy `0x4c1640`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 8a3d.c9c5.829b.30b9.e945.7835.7f63.8127.1802.98bf.14aa.1d41.e669.cf27.8133.8468; incoming call sites ev 9df2.2c0a.bb99.3fa4.99d7.69b0.1abf.70cd.ca33.b324.2bd8.a483.b52a.7845.cf05.b284 (observed direct callers: 0x4c1640). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c5940` — Result / records

- VA / file offset: `0x4c5940` / `0xc5940`.
- Behavior (own words): Grows a vector of six-byte records (a two-byte id/value, a four-byte word and a one-byte marker), fills the new region, copies the old tail and fixes the end pointer. Capacity growth, fill and copy are handled by local helpers.
- Reach and player-visible role: Indirect player visibility: the same six-byte vector is used by the Result grade chooser `0x4c4dd0` (see `docs/parity/result.md`, grade-band rule) and by wire reader `0x55b590`, which stores count-prefixed six-byte rows and queues UI message `0x12`/`0x7ef`. This pin covers storage, not the threshold rule itself.
- Our status: Our grade plate uses `o2jam_game::grade_frame` (`crates/o2jam-game/src/lib.rs:1648`) and the pinned threshold table; no vector-of-records mirror exists.
- Gameplay-rule note: Consistent with `docs/gameplay.md` and `docs/parity/result.md`: the record shape carries band/marker data, not new score arithmetic. No new rule to apply.
- Evidence: analysis ev f95a.05cd.0c30.aff1.0e62.cd05.41c3.7367.522e.b2cc.a3ea.d687.cfbf.3350.7052.e325; incoming call sites ev e67e.8037.7b29.a72a.f795.d461.d507.59fc.f860.a323.8ec6.5474.d686.6639.092c.76c6 (observed direct callers: 0x4c5810). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c5d7f` — Result / records

- VA / file offset: `0x4c5d7f` / `0xc5d7f`.
- Behavior (own words): Terminal cleanup of the six-byte insert path: computes the new size, frees the old buffer when present and writes the begin/end/capacity words.
- Reach and player-visible role: Support; terminal call from `0x4c5940`. Serves the grade/wire record table only via that caller.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 5605.4439.bf32.d430.fdc9.1358.15a9.6dbe.d3c8.feea.a6f1.bc2c.edc0.b0a2.6197.775f; incoming call sites ev dc30.1935.cb25.eb77.4eae.a4ef.5359.cbd3.ce5f.3eb9.3b29.1981.12af.5004.ff86.596f (observed direct callers: 0x4c5940). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c6290` — Result / records

- VA / file offset: `0x4c6290` / `0xc6290`.
- Behavior (own words): Copies a six-byte template (word, dword, byte) into N consecutive records.
- Reach and player-visible role: Support; called by six-byte push-back `0x4c5700` and insert `0x4c5940`; those reach the grade chooser and the wire reader.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 49d4.1e05.6184.7b83.5656.7f1a.13e4.c860.855e.faed.1296.6bca.1ad5.5e8b.8806.be82; incoming call sites ev fbcf.b659.10b7.e95a.9073.52ce.fb28.21b7.7b70.37f1.66a1.63e5.043c.1d75.0870.0ecb (observed direct callers: 0x4c5700, 0x4c5940). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c6345` — Result / records

- VA / file offset: `0x4c6345` / `0xc6345`.
- Behavior (own words): Tiny cleanup fragment that restores the exception list for the six-byte fill helper.
- Reach and player-visible role: Support; sole caller is `0x4c6290`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev 2a6f.81ef.c6fe.27be.5cc1.1954.a8ea.8689.865f.51b3.bd45.66aa.8bbd.8386.c8ce.0ddc; incoming call sites ev 64ec.8def.af81.e149.4e4e.80a4.d5ce.3f67.39f7.339e.6c18.eb58.586d.95ee.2302.bcab (observed direct callers: 0x4c6290). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c6480` — Result / records

- VA / file offset: `0x4c6480` / `0xc6480`.
- Behavior (own words): Forwards its arguments unchanged to the six-byte range-copy helper.
- Reach and player-visible role: Support; called by six-byte insert `0x4c5940`, and it forwards to copy helper `0x4c64d0`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev a0cc.edd8.57df.5b9a.1410.786d.4447.f085.85a0.ffaa.2cf1.33b0.3d97.a6f9.e2a8.beee; incoming call sites ev a0ff.e206.037d.a744.8410.f280.8445.9c9d.730e.4f16.c9a7.eef0.c96c.3ceb.84f7.be29 (observed direct callers: 0x4c5940). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c64d0` — Result / records

- VA / file offset: `0x4c64d0` / `0xc64d0`.
- Behavior (own words): Walks a source range and copies each six-byte record to the target when non-null.
- Reach and player-visible role: Support; reached from `0x4c6480` and a local copy caller. Same grade/wire table as `0x4c5940`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev eaa8.8e8f.a518.d572.f13d.aca8.df32.f251.6736.82bc.07f8.8726.97d0.dccf.305c.bd5a; incoming call sites ev 8f0a.0c88.62d5.47e2.7ff9.00a4.e68a.e25e.1ec4.4563.9892.fc95.7828.7691.5537.98b3 (observed direct callers: 0x4c6360, 0x4c6480). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c6587` — Result / records

- VA / file offset: `0x4c6587` / `0xc6587`.
- Behavior (own words): Tiny cleanup fragment that restores the exception list and returns the caller-held result.
- Reach and player-visible role: Support; sole caller is six-byte range-copy `0x4c64d0`.
- Our status: No counterpart.
- Gameplay-rule note: No gameplay-rule implication.
- Evidence: analysis ev b891.6e40.e1a3.c9d2.984f.5323.b976.beb4.65af.ace2.aef6.fff2.a3f0.6b42.21bf.c6e0; incoming call sites ev b0c8.b92c.d3f5.029e.7d4f.3b73.d36b.1b5e.393f.3bf7.0983.79f6.2461.c740.b148.53e9 (observed direct callers: 0x4c64d0). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

### `0x4c6ba0` — Result / records (triaged; actually Music Shop)

- VA / file offset: `0x4c6ba0` / `0xc6ba0`.
- Behavior (own words): Clears a block of state fields and several byte buffers, seeds defaults (including two ones and one zero), reads the clock helper `0x411e60` into a field, clears a global slot at `0x77a868` plus offsets and resets a further buffer.
- Reach and player-visible role: Support for the Music Shop state, not Result: sole caller is `CStateMShop` constructor `0x4c6760`, which is installed by the state factory `0x477990`. The triage Result-range label is refuted by this caller chain; no Result census row is served.
- Our status: Shell models this state as `Screen::MusicShop` (`crates/o2jam-shell/src/screen.rs:47`, `SideScreen::MusicShop.state_class()`); no field mirror of this helper.
- Gameplay-rule note: No gameplay-rule implication; belongs to spec issue #179 Music Shop, not #294.
- Evidence: analysis ev f38e.498f.461c.9e79.c733.c5eb.efd3.4576.c8e7.1f66.ee22.cfdc.e826.566c.9e05.cdbf; incoming call sites ev 0b66.425d.8906.7535.2ab4.dc48.37c6.00d6.d3d2.c01c.b8ed.2987.e41f.340a.ff91.006f (observed direct callers: 0x4c6760). File offset from REA `address_to_file_offset`; body and calls from REA `analyze_function` / `procedure_callers`, with the saved batch decompile used only to read behavior in our own words.

## Contradictions and OPENs

- The triage labels eight functions at `0x4c0b00`–`0x4c16f5` and seven at
  `0x4c5940`–`0x4c6587` as Result / records by address cluster. Body and caller
  evidence shows generic vector helpers for eight-byte and six-byte records.
  The six-byte family is consumed by the grade chooser and a wire reader;
  the eight-byte family is consumed by Waiting-side setup and net handlers.
  Neither family is a Result record writer by itself. The labels are retained
  only to make the burn-down countable.
- `0x4c6ba0` is not Result at all: its only caller is the Music Shop state
  constructor `0x4c6760`. It is pinned here for bucket completeness and
  handed to spec issue #179 Music Shop.
- No live or rig leg was run in this lane. Player-visible rows cited above stay
  OPEN per `docs/interaction-census.md`; statics pins do not close them.
  In particular the Playing seven-lane key-up dispatch and the Result
  grade/digit live-art rows keep their existing next steps.
- Ghidra reports terminal calls and synthetic exception-handler fragments for
  the small unwind stubs; their role is inferred from the caller's body and is
  not a runtime observation. All details are Ghidra observations, not original
  source; PDB is absent.

### `0x4c2b20` / `0x4c2cf0` — Result keyboard-Back mirror and the BACK/RETRY arm (2026-11-25 senior REA pass)

- **VA / file offsets**: `0x4c2b20` / `0xc2b20` (queue drain), `0x4c2cf0` / `0xc2cf0` (dispatch arm).
- **Behavior (own words)**: the Result state drains its queued input
  events in `0x4c2b20`: for an event of class 2 (key) with code 8 or 9
  (the Win32 VK order makes those Backspace and Tab), once the settle
  flag at `+0x434a` is set (the same flag the settle timer
  `0x4c28e0` waits on before enabling BACK), it invokes the state's
  `+0x28` dispatch arm with the BACK control id `0x17200005` — the same
  arm a mouse click on BACK reaches. The arm `0x4c2cf0` posts the `0x23`
  notice and, when the settle flag stands, sends the empty 4021 body
  through the `+0x5c` sender; its RETRY half (`0x17202006`) posts 4010
  unless a battle peer is live (then the `0x64a3a0` caption with a
  3000 ms show). No keyboard arm maps to RETRY.
- **Consequence for our client**: the exe HAS a keyboard-Back mirror and
  its target is the BACK button's own arm (4021, then Waiting after the
  4022 answer). Our `result_keys_mirror_the_staged_buttons`
  (`crates/o2jam-shell/src/screens/game/mod.rs:33690`) diverges twice:
  its Back goes to a local Room shortcut (the exe's goes to the button's
  Waiting path) and its Confirm mirror has no exe counterpart. Align:
  keyboard Back (Backspace/Tab class) drives the BACK arm after the
  settle flag; drop or re-pin the Confirm mirror.
- **Evidence**: `ev_dbe3dc5432238ed4edd59f054accf0703cade787d2b88db3bdd72eab8503bf15`
  (drain dossier), `ev_70fafb64080f1a1a9e7485cf14a1c102c9e82a512e6b968122a6f04d89f71fbd`
  (arm dossier), `ev_9727bf792adf6ac2fd6ca1fa25744f1bed91796afca7fcf15222a16c7010c221`
  (settle timer), `ev_9239cf8242aec5b5180ab9cbb7367a8461ded13d975166673c33a99104b08fc4`
  (BACK enable helper).

### `0x5598b0` notice-contract addendum — 4035 posts events, not caption text (2026-11-25 senior REA pass)

- **Body**: one u8 count, then `count` pairs of `(u8 member, u8 state)`.
- **Behavior (own words)**: count zero posts one UI event
  `(0x12, 0xfc3, 0, 0)`. A nonzero count posts one event per row,
  `(0x12, 0xfc3, member, count)`, after applying the state through the
  shared setter. The self/other member branches call the same setter
  (dead distinction in this build). There is NO caption string and no
  text popup in this handler.
- **Consequence for #1037**: the acquire notice is the event stream, not
  a shell caption. Guard the exact empty/nonempty event sequences; do not
  invent notice pixels.
- **Evidence**: `ev_9856c8aed1ea517462b8c4505b6eea444ef72aba7b0f2e05db78f821c9d94dad`.

### `0x541580` speed-setter addendum (2026-11-25 senior REA pass)

`0x541580` stores its byte argument at the game object's `+0x2168`.
The Result board's member-speed read is therefore the low byte of that
word; the 4050 handler's two writes leave the second write as the final
speed. No extra transformation is applied by this setter.

Evidence: `ev_98b8a43fc02d90d285c7c80eec3233d2656562f010e80490bab0417c2dbaec30`.
