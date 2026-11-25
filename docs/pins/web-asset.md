# Static pins — Web asset / browser and relay support (game-reachable backlog)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a0.4541.8b62.19ba.3406.8176.6b33.de2b.fbb5.ff51.b813.ef4e.2e9d.d1de.f72c.823b`.
Provider: REA 6.10.1 / Ghidra 12.2 on that exact artifact. Evidence IDs are
`ev ` plus the full ID in four-digit groups; strip the dots and prefix `ev_`
to recover the REA ID. File offsets come from REA `address_to_file_offset`
(PE image base `0x400000`) and were checked for every row: all 64 match VA minus
`0x400000`. Behavior wording is ours; no disassembly or decompiled text is
reproduced.

Branch `cb2/pins-web-asset`; worktree pre-flight
`/home/arnim/Projects/recomp2jam-lane-pins-web-asset`. Brief base `a15244e`
and observed HEAD `04cf4681453afb65a5205e1881f11d77c1c0b79e` are both
commits in this checkout; the brief base is an ancestor of HEAD. No reset was
attempted (other lanes merged into the worktree after the brief was written).

Scope: the 64 addresses in `slice-web-asset.txt` from the coverage triage
bucket `game-reachable`, `pinned_in_repo=no`. **Pinned here: 64/64; left: 0.**
One row stages the window cursor; three rows construct browser-host controls;
the rest are file, container, map/set, timing and live-battle relay support.
A static pin is not a player-visible close: census rows stay OPEN until a live
caller chain from boot is recorded (`docs/live-path-reachability.md`).
"Reached from" names the provider's direct-caller edge, not a runtime press.
Relay rows are battle-only; the owner pin in memory and `docs/relay.md` keeps
them out of normal Playing hypotheses.

## Inventory

| VA | File offset | Role |
|---|---|---|
| `0x406570` | `0x6570` | support |
| `0x4065f0` | `0x65f0` | support |
| `0x406610` | `0x6610` | support |
| `0x4066c0` | `0x66c0` | support |
| `0x406710` | `0x6710` | support |
| `0x406730` | `0x6730` | support |
| `0x4067e0` | `0x67e0` | support |
| `0x4068b0` | `0x68b0` | support |
| `0x406940` | `0x6940` | support |
| `0x4088a0` | `0x88a0` | support |
| `0x409a70` | `0x9a70` | support |
| `0x409b30` | `0x9b30` | support |
| `0x40a830` | `0xa830` | support |
| `0x40aac0` | `0xaac0` | support |
| `0x40ab10` | `0xab10` | support |
| `0x40ab30` | `0xab30` | support |
| `0x432090` | `0x32090` | player-visible |
| `0x452d70` | `0x52d70` | support |
| `0x5251b0` | `0x1251b0` | support |
| `0x57dd20` | `0x17dd20` | support |
| `0x57f5f0` | `0x17f5f0` | support |
| `0x581dc0` | `0x181dc0` | support |
| `0x5864c0` | `0x1864c0` | support |
| `0x586590` | `0x186590` | support |
| `0x586600` | `0x186600` | support |
| `0x586950` | `0x186950` | support |
| `0x586980` | `0x186980` | support |
| `0x5869c0` | `0x1869c0` | support |
| `0x586a30` | `0x186a30` | support |
| `0x586a80` | `0x186a80` | support |
| `0x586ce0` | `0x186ce0` | support |
| `0x588430` | `0x188430` | support |
| `0x588880` | `0x188880` | support |
| `0x588d00` | `0x188d00` | support |
| `0x589a10` | `0x189a10` | support |
| `0x589ad0` | `0x189ad0` | support |
| `0x58aaf0` | `0x18aaf0` | support |
| `0x58aba0` | `0x18aba0` | support |
| `0x58ac50` | `0x18ac50` | support |
| `0x58ad2f` | `0x18ad2f` | support |
| `0x58af10` | `0x18af10` | support |
| `0x58b34c` | `0x18b34c` | support |
| `0x58c0c0` | `0x18c0c0` | support |
| `0x58c175` | `0x18c175` | support |
| `0x58c2a0` | `0x18c2a0` | support |
| `0x58c2f0` | `0x18c2f0` | support |
| `0x58c3a7` | `0x18c3a7` | support |
| `0x58d030` | `0x18d030` | support |
| `0x58d050` | `0x18d050` | support |
| `0x58d440` | `0x18d440` | support |
| `0x58d4b0` | `0x18d4b0` | support |
| `0x58d4d0` | `0x18d4d0` | support |
| `0x58d510` | `0x18d510` | support |
| `0x58d910` | `0x18d910` | support |
| `0x58da46` | `0x18da46` | support |
| `0x58db10` | `0x18db10` | support |
| `0x58dbd0` | `0x18dbd0` | support |
| `0x58dd20` | `0x18dd20` | support |
| `0x58dd96` | `0x18dd96` | support |
| `0x58ddb0` | `0x18ddb0` | support |
| `0x58de10` | `0x18de10` | support |
| `0x58f6b0` | `0x18f6b0` | support |
| `0x58fa99` | `0x18fa99` | support |
| `0x58ff70` | `0x18ff70` | support |

## Per-function pins

### `0x406570` — install the `CDL_Base` vftable (support)

- **VA / file offset**: `0x406570` / `0x6570` (25968).
- **Behavior**: Writes the `CDL_Base` vftable pointer at the object start, then calls `0x406610` to clear three following state words.
- **Reached from**: `0x4099a0`, `0x40a170`, `0x41c4d0`, `0x41dd90`, `0x4eb3d0`.
- **Player-visible reach**: support.
- **Evidence**: ev 1b58.3562.4568.b22b.b592.1227.b27a.1800.a78a.1352.becc.f488.b922.6421.5b17.a51d.
- **Our client**: No counterpart by VA. Shell control state is held in Rust structs; staged rects are painted from `crates/o2jam-shell/src/staging.rs:738`.

### `0x4065f0` — install the `CDL_Base` vftable only (support)

- **VA / file offset**: `0x4065f0` / `0x65f0` (26096).
- **Behavior**: Writes the `CDL_Base` vftable pointer at the object start and returns; a minimal initializer reused by derived constructors and unwind stubs.
- **Reached from**: `0x4065c0`, `0x409a50`, `0x40a440`, `0x41c560`, `0x41de50`, `0x4eb450` (11 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev 0e4d.3a0a.c6dc.c143.83ac.c528.322e.d498.bf81.d57d.3eb0.06d4.8b9d.21fe.f26c.5fc7.
- **Our client**: No counterpart by VA; Rust vtables are compiler-owned.

### `0x406610` — clear three `CDL_Base` words (support)

- **VA / file offset**: `0x406610` / `0x6610` (26128).
- **Behavior**: Zeroes the object words at `+4`, `+8` and `+0xc`. It is the field-reset half of the `0x406570` constructor.
- **Reached from**: `0x406570`.
- **Player-visible reach**: support.
- **Evidence**: ev d6a9.f824.3d13.b831.08aa.e955.dce5.7ecb.dc7c.716a.9fed.475d.ef6d.06bc.d934.c0c7.
- **Our client**: No counterpart by VA; struct initialization in Rust.

### `0x4066c0` — construct `CFileManager` (support)

- **VA / file offset**: `0x4066c0` / `0x66c0` (26304).
- **Behavior**: Installs the `CFileManager` vftable, then calls `0x406730` to set the file-handle word to the `-1` sentinel.
- **Reached from**: `0x402670`, `0x402be0`, `0x403160`, `0x40a480`, `0x410f40`, `0x420da0` (21 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev dad9.0a6a.6232.75d4.626a.89b8.eeb7.a8e6.66e3.b051.f0ea.0686.e3b1.8c41.99b2.4f8e.
- **Our client**: No counterpart by VA. Asset bytes are read by the parsers in `crates/o2jam-file/src/` and staged in `crates/o2jam-shell/src/staging.rs`.

### `0x406710` — install the `CFileManager` vftable only (support)

- **VA / file offset**: `0x406710` / `0x6710` (26384).
- **Behavior**: Writes the `CFileManager` vftable pointer and returns. It is the lightweight initializer used by many derived and unwind paths.
- **Reached from**: `0x402670`, `0x402be0`, `0x403160`, `0x4066e0`, `0x40a520`, `0x410f40` (45 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev 8124.2960.8e66.1102.d44c.2391.96b3.7ab8.c311.9ea0.4aa9.69c4.6421.78db.1987.461b.
- **Our client**: No counterpart by VA; Rust drop/ownership replaces manager lifetime code.

### `0x406730` — reset the file-handle sentinel (support)

- **VA / file offset**: `0x406730` / `0x6730` (26416).
- **Behavior**: Stores `-1` in the manager's `+0x104` handle word, marking no file open.
- **Reached from**: `0x4066c0`.
- **Player-visible reach**: support.
- **Evidence**: ev d099.1937.e92c.1cc4.2f57.3541.879f.3126.be1c.ec11.d5ba.e251.772f.f497.4685.bbe0.
- **Our client**: No counterpart by VA; `std::fs::File` closes on drop.

### `0x4067e0` — open an asset file and store its handle (support)

- **VA / file offset**: `0x4067e0` / `0x67e0` (26592).
- **Behavior**: Closes an already open handle, calls `CreateFileA` with caller-supplied access, share mode and creation flags, `FILE_ATTRIBUTE_NORMAL`, no security attributes and no template; stores the result and reports success when it is not `-1`.
- **Reached from**: `0x406850`, `0x4068b0`, `0x4068e0`.
- **Player-visible reach**: support.
- **Evidence**: ev 0e2e.7883.4d0e.c18d.a287.c4bb.b56c.fb1f.a750.858b.5c3b.36ba.c603.2602.8e43.80fc.
- **Our client**: No counterpart by VA; file reads in `crates/o2jam-file/src/ojm.rs` and `crates/o2jam-file/src/opi.rs` use Rust I/O.

### `0x4068b0` — open a file read-only (support)

- **VA / file offset**: `0x4068b0` / `0x68b0` (26800).
- **Behavior**: Calls `0x4067e0` with `GENERIC_READ`, share mode 1 and `OPEN_EXISTING`; a thin wrapper used by format readers.
- **Reached from**: `0x403160`, `0x406770`, `0x40a5e0`, `0x410f40`, `0x479f90`, `0x47c510` (9 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev 0b40.7a9a.3e14.5f51.2aa4.6973.333b.e81c.bd71.e401.5dfc.3a7e.c1e6.cab1.98d0.a466.
- **Our client**: No counterpart by VA; read-only opens in the Rust file crate.

### `0x406940` — close the current file handle (support)

- **VA / file offset**: `0x406940` / `0x6940` (26944).
- **Behavior**: Closes the stored handle when it is not `-1`, then restores the `-1` sentinel.
- **Reached from**: `0x403160`, `0x406750`, `0x406770`, `0x40a830`, `0x410f40`, `0x479f90` (11 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev 6946.6758.2fdc.5ed7.a6dd.6a4a.a030.411b.51ec.c4b8.e338.7990.3854.20da.e48d.d953.
- **Our client**: No counterpart by VA; Rust `File` drop closes the descriptor.

### `0x4088a0` — reset a list-row text buffer (support)

- **VA / file offset**: `0x4088a0` / `0x88a0` (34976).
- **Behavior**: Sets the buffer's first word to 1 and second to 0, then zeroes a 256-byte text area. Called by the list-row writer, not by a control handler.
- **Reached from**: `0x408810`, `0x510730`, `0x511750`, `0x5980b0`, `0x598870`, `0x599130`.
- **Player-visible reach**: support.
- **Evidence**: ev 8f8a.cbea.c495.75ae.ee9a.0c97.b803.0f57.5980.23c0.3383.9a7d.ea19.a3ae.a295.19d5.
- **Our client**: No counterpart by VA; row text buffers live in `ChatState` (`crates/o2jam-shell/src/screens/game/chat.rs:142`) and list painters.

### `0x409a70` — reset control-state words (support)

- **VA / file offset**: `0x409a70` / `0x9a70` (39536).
- **Behavior**: Clears eight words and sets two words at `+0x20`/`+0x24` to 1; called by the matching constructor.
- **Reached from**: `0x4099a0`.
- **Player-visible reach**: support.
- **Evidence**: ev 9065.ecff.9e46.7142.94e5.3b03.8c9f.b0cc.8d64.a030.eb2c.c4d3.f293.53a9.3261.9bd0.
- **Our client**: No counterpart by VA; equivalent booleans are fields on shell screens.

### `0x409b30` — read the state/enable word (support)

- **VA / file offset**: `0x409b30` / `0x9b30` (39728).
- **Behavior**: Returns the object's `+0x20` word. One caller is the Room row-join gate `0x493a50`; most callers are control dispatchers.
- **Reached from**: `0x435aa0`, `0x43e1f0`, `0x44f8a0`, `0x462590`, `0x462600`, `0x466610` (31 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev 552a.9490.b44a.a547.53fc.342b.a669.dc1f.e562.cca0.b591.945d.4086.b423.93a2.dc31.
- **Our client**: No VA counterpart; state flags are plain Rust fields.

### `0x40a830` — release a packing-file reference (support)

- **VA / file offset**: `0x40a830` / `0xa830` (43056).
- **Behavior**: If the reference word is zero it logs an assertion naming `Utility/PackingFile.cpp` and returns 0. Otherwise it decrements the word, closes the file through `0x406940` when the count reaches zero, and returns 1.
- **Reached from**: `0x40a590`, `0x479750`, `0x4798e0`.
- **Player-visible reach**: support.
- **Evidence**: ev a5cf.a6a9.d5d4.f897.35cd.98f0.5e96.0f18.0ad1.fe6d.1065.18a2.f903.4ff3.3b20.ab38.
- **Our client**: No counterpart by VA; `Arc`/`Rc` and Rust drop own archive lifetime.

### `0x40aac0` — construct `CParserBase` (support)

- **VA / file offset**: `0x40aac0` / `0xaac0` (43712).
- **Behavior**: Installs the `CParserBase` vftable and calls the empty init hook `0x40ab30`. Callers include the Room chat dispatch at `0x48f3a0`.
- **Reached from**: `0x40a5e0`, `0x410d40`, `0x410f40`, `0x418eb0`, `0x420da0`, `0x4249f0` (17 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev 7139.f04b.7cbb.e234.ac92.e0d1.e8da.6820.4a82.de39.e0fd.532c.73ac.1ecb.eef8.a287.
- **Our client**: No counterpart by VA; parsing is done by the crates under `crates/o2jam-file/src/`.

### `0x40ab10` — install the `CParserBase` vftable only (support)

- **VA / file offset**: `0x40ab10` / `0xab10` (43792).
- **Behavior**: Writes the parser vftable pointer and returns; the lightweight path for derived parser objects.
- **Reached from**: `0x40a5e0`, `0x40aae0`, `0x410e20`, `0x410f40`, `0x418eb0`, `0x420da0` (36 direct callers total).
- **Player-visible reach**: support.
- **Evidence**: ev d2d5.6010.9f8f.8367.e6c9.7138.5742.08c7.92b0.04be.d06d.eb8c.7a52.0bb2.40cb.0992.
- **Our client**: No counterpart by VA; Rust traits are static dispatch.

### `0x40ab30` — empty parser init hook (support)

- **VA / file offset**: `0x40ab30` / `0xab30` (43824).
- **Behavior**: Returns without work. It exists as the overridable init call made by the `CParserBase` constructor.
- **Reached from**: `0x40aac0`.
- **Player-visible reach**: support.
- **Evidence**: ev 3d55.2399.6407.1d16.28c9.3ed4.d804.1885.723f.4440.a87b.4da9.596d.50ca.217b.351a.
- **Our client**: No counterpart by VA or behavior.

### `0x432090` — stage the window-cursor sprite lazily

- **VA / file offset**: `0x432090` / `0x32090` (204944).
- **Behavior**: On first use of the cursor control it sets two flags, loads `Window_Cursor.ojs` through `0x5f27f0`, asks `0x479900` for a surface from the staged control description, and passes width/height flags to `0x42fe90`/`0x42fb30`. The resulting handle is stored at the control's `+0x20`.
- **Reached from**: `0x432020`.
- **Player-visible reach**: player-visible: the staged cursor sprite is the exe's pointer image; reach is recorded from `0x432020` (first-use guard), not a live pointer capture..
- **Evidence**: ev 18ee.0400.f390.fbdd.1283.9b20.267f.6e89.cc3f.ecde.a490.da7b.db50.a7fa.57d6.7360.
- **Our client**: No exe-asset counterpart: the shell paints an amber stand-in (`crates/o2jam-shell/src/screens/paint.rs:4332`), guarded by `planet_selection_paints_the_amber_cursor_stand_in` (`paint.rs:10225`, Tier 0, labeled OPEN).

### `0x452d70` — reserve a vector of 436-byte records (support)

- **VA / file offset**: `0x452d70` / `0x52d70` (339312).
- **Behavior**: Zeroes three vector words; on a nonzero request it allocates through `0x455000` and computes end/capacity from 436-byte (`0x1b4`) slots. Oversized requests go to `0x453840`.
- **Reached from**: `0x43cf60`, `0x485540`, `0x4c6760`.
- **Player-visible reach**: support.
- **Evidence**: ev 96b2.91a8.1240.1e68.6403.5cff.200c.1803.79f0.e3d7.f6e0.5b37.673b.4236.d6f8.ebc0.
- **Our client**: No counterpart by VA; Rust `Vec::with_capacity`.

### `0x5251b0` — bind a Room row record and set its enable word (support)

- **VA / file offset**: `0x5251b0` / `0x1251b0` (1,200,560).
- **Behavior**: Stores the supplied room-record pointer at `+100`. A null pointer, empty name, or a count below 100 clears the enable word via `0x409bd0(0)`; otherwise it sets the word to 1 and calls the detail fill `0x525240`.
- **Reached from**: `0x492300`, `0x4925a0`.
- **Player-visible reach**: support for Room row readouts; the two callers are the Room row refresh loops at `0x492300`/`0x4925a0`. No pixel or wire is emitted here..
- **Evidence**: ev 69c1.2e65.5aa3.4e07.af58.b01c.c859.cb78.a2b0.7d59.2f98.b5b5.1764.55b4.b794.b3c5.
- **Our client**: No counterpart by VA. Room rows are staged and painted by `ScreenGame` (`crates/o2jam-shell/src/screens/game/mod.rs:2545`, `:15390`).

### `0x57dd20` — construct `CWebCtrlBase` (support for browser hosts)

- **VA / file offset**: `0x57dd20` / `0x17dd20` (1,563,936).
- **Behavior**: Installs the base vftable, asks `GetSystemMetrics` for screen width/height and stores each plus two in the control's second and third words, clears two flags, and zeroes two 2 KiB URL/text buffers at `+7` and `+0x207`.
- **Reached from**: `0x57f450`, `0x57f5f0`, `0x580ab0`, `0x580cd0`, `0x581dc0`.
- **Player-visible reach**: support for player-visible IE surfaces: every browser-hosted dialog inherits this client sizing and URL storage..
- **Evidence**: ev 8510.bc3e.a4eb.238d.5b58.6c09.82bb.7765.4b5c.fbe6.dfb1.754a.7413.34cd.ff68.6e69.
- **Our client**: No counterpart by VA. The Rust shell leaves hosted page areas blank (`payment_views_leave_the_embedded_web_client_blank`, `crates/o2jam-shell/src/screens/game/mod.rs:20619`, Tier 0); no IE client is created.

### `0x57f5f0` — construct the Room friend-list web control (support)

- **VA / file offset**: `0x57f5f0` / `0x17f5f0` (1,570,288).
- **Behavior**: Calls `0x57dd20`, installs `CWebFriendCtrl::vftable`, then sets three trailing flags (first to 1, next two to 0).
- **Reached from**: `0x495710`.
- **Player-visible reach**: support for a player-visible surface: its only caller, `0x495710`, builds the Room `gamefine_main.asp` sheet on the staged web rect..
- **Evidence**: ev b8a0.bed9.8a19.eb11.ff6a.69c3.df8e.c533.3861.6cee.1cc2.6a1c.2b73.f8b1.4d1c.5704.
- **Our client**: No browser counterpart. The Room user panel paints native character/well data and leaves the desc well empty without a web source (`crates/o2jam-shell/src/screens/paint.rs:11999`, Tier 0).

### `0x581dc0` — construct the normal web control (support)

- **VA / file offset**: `0x581dc0` / `0x181dc0` (1,580,480).
- **Behavior**: Calls `0x57dd20` and installs `CWebNormalCtrl::vftable`; no extra fields are set here.
- **Reached from**: `0x45e010`, `0x45ea60`, `0x45f190`, `0x4f3b30`, `0x4f4bd0`.
- **Player-visible reach**: support for player-visible browser dialogs: constructed by the cash-charge, bingo, help, gift and payment URL hosts (`0x45e010`, `0x45ea60`, `0x45f190`, `0x4f3b30`, `0x4f4bd0`)..
- **Evidence**: ev e680.538e.c756.afaa.7d3b.c21c.28d5.eea1.8f48.32eb.32d1.1496.8015.a963.b0d3.564b.
- **Our client**: No IE counterpart; the shell's hosted page area stays blank (`crates/o2jam-shell/src/screens/game/mod.rs:20619`, Tier 0).

### `0x5864c0` — accumulate one timing/stat sample (support)

- **VA / file offset**: `0x5864c0` / `0x1864c0` (1,598,656).
- **Behavior**: When the supplied id equals the global id and the object's enable word is nonzero, it computes a 64-bit delta against the stored base, writes it into the current slot, calls `0x586600` and resets the base; otherwise it sets a mark word. The fifth argument sets a separate flag when nonzero.
- **Reached from**: `0x586470`.
- **Player-visible reach**: support: a ten-slot accumulator, not a pixel or wire action. Its only caller is `0x586470`..
- **Evidence**: ev 0939.8543.8d80.6b5d.8fd2.fd03.635c.ebc9.5094.e0c0.b440.1659.9596.73f7.bdb3.3c1b.
- **Our client**: No counterpart by VA; frame pacing is handled outside this shell path.

### `0x586590` — compute the ten-slot average (support)

- **VA / file offset**: `0x586590` / `0x186590` (1,598,864).
- **Behavior**: Sums the ten slot words with carry and divides the 64-bit result by a 64-bit denominator; stores the quotient in the object's first word.
- **Reached from**: `0x586600`.
- **Player-visible reach**: support for the accumulator above; no pixels or wire..
- **Evidence**: ev 6893.1fa9.0b95.562a.17aa.a2c9.b3de.488e.5940.1e76.7c40.5a55.8337.2b3a.e4cb.8361.
- **Our client**: No counterpart by VA.

### `0x586600` — advance the sample counter (support)

- **VA / file offset**: `0x586600` / `0x186600` (1,598,976).
- **Behavior**: If the counter is below 10 it increments it; at 10 it clears the enable word and calls `0x586590` to publish the average.
- **Reached from**: `0x5864c0`.
- **Player-visible reach**: support for the accumulator above..
- **Evidence**: ev 34d5.7415.cba4.96bc.ce51.2d5c.b887.e60d.f02a.c695.3056.2a49.ba56.c6ba.37f9.a120.
- **Our client**: No counterpart by VA.

### `0x586950` — store a three-word value block (support)

- **VA / file offset**: `0x586950` / `0x186950` (1,599,824).
- **Behavior**: Copies the three supplied words into the fields at `+0x130`, `+0x134` and `+0x138`.
- **Reached from**: `0x586900`, `0x58d4d0`.
- **Player-visible reach**: support accessor; no pixels or wire..
- **Evidence**: ev 70a6.f52b.79ff.bb5b.9fa0.c98f.a2c3.c948.31b3.8e25.e6ba.1f86.c3de.9048.e9e7.4eba.
- **Our client**: No counterpart by VA.

### `0x586980` — return the first block address (support)

- **VA / file offset**: `0x586980` / `0x186980` (1,599,872).
- **Behavior**: Returns the address of the three-word field block at `+0x130`.
- **Reached from**: `0x586b50`, `0x586f90`, `0x5874c0`, `0x587870`, `0x587930`, `0x587c50`.
- **Player-visible reach**: support accessor used by tree and relay paths..
- **Evidence**: ev 0702.d157.d480.85ce.191d.61b7.985d.ef53.c978.9f94.0e74.253a.3864.784c.6d8b.1a94.
- **Our client**: No counterpart by VA.

### `0x5869c0` — return the second block address (support)

- **VA / file offset**: `0x5869c0` / `0x1869c0` (1,599,936).
- **Behavior**: Returns the address of the field block at `+0x16c`.
- **Reached from**: `0x586f90`, `0x5874c0`, `0x587c50`, `0x58d580`, `0x5907a0`, `0x590950`.
- **Player-visible reach**: support accessor used by tree and relay paths..
- **Evidence**: ev d9bd.c3b2.ec96.8561.cc2d.1ba2.1889.395e.5a14.faf1.cb60.5981.15cc.630f.c16b.9a6d.
- **Our client**: No counterpart by VA.

### `0x586a30` — reset two intrusive trees (support)

- **VA / file offset**: `0x586a30` / `0x186a30` (1,600,048).
- **Behavior**: Calls `0x588880` and `0x588430` to reset two tree/list heads to themselves.
- **Reached from**: `0x586900`, `0x58d440`.
- **Player-visible reach**: support cleanup called from object init/clear paths (`0x586900`, `0x58d440`)..
- **Evidence**: ev 697b.98de.d350.254e.322d.6313.4f1e.322e.2089.6351.1671.879c.4a90.f52d.cf49.a7de.
- **Our client**: No counterpart by VA; Rust `BTreeMap` drop handles this.

### `0x586a80` — look up a packed key in a tree (support)

- **VA / file offset**: `0x586a80` / `0x186a80` (1,600,128).
- **Behavior**: Packs four inputs into a small key record and queries the tree through `0x588580`.
- **Reached from**: `0x58d580`.
- **Player-visible reach**: support lookup; no pixels or wire..
- **Evidence**: ev c809.750a.b370.715d.363f.5597.7169.7015.7244.8c95.aa2b.6f48.d2e4.e593.aa11.1f16.
- **Our client**: No counterpart by VA; Rust map lookup.

### `0x586ce0` — test a bounded counter (support)

- **VA / file offset**: `0x586ce0` / `0x186ce0` (1,600,736).
- **Behavior**: Calls `0x5c4500`, then returns true when the field at `+0x144` is zero or its value minus one is not greater than the word at `+0x168`; restores state through `0x5c4520`.
- **Reached from**: `0x58d580`.
- **Player-visible reach**: support gate used by the relay handshake wait..
- **Evidence**: ev 1cd0.7ab9.0c6f.7e87.9668.0338.96ae.0a1a.7a9f.74bb.c321.ff36.3709.96f5.49be.ade1.
- **Our client**: No counterpart by VA.

### `0x588430` — reset the first intrusive list (support)

- **VA / file offset**: `0x588430` / `0x188430` (1,606,704).
- **Behavior**: Frees nodes through `0x588d00`, then makes the head point at itself, zeroes a count word and links both child pointers back to the head.
- **Reached from**: `0x586a30`, `0x588b00`.
- **Player-visible reach**: support cleanup called by `0x586a30`/`0x588b00`..
- **Evidence**: ev 0ec0.e3d8.ad89.3f4c.b3f3.fb7e.65bd.ed2e.3867.4186.8d23.a621.ddc3.2052.319f.bfab.
- **Our client**: No counterpart by VA.

### `0x588880` — reset the second intrusive list (support)

- **VA / file offset**: `0x588880` / `0x188880` (1,607,808).
- **Behavior**: Frees nodes through `0x5893f0`, then makes the head self-referential and links its child pointers back to it.
- **Reached from**: `0x586a30`, `0x5891f0`.
- **Player-visible reach**: support cleanup called by `0x586a30`/`0x5891f0`..
- **Evidence**: ev dc0e.a53a.3840.1d29.17b4.d594.ea8a.e1d8.ade6.a144.9a1c.9656.443b.59e7.67f8.6fc5.
- **Our client**: No counterpart by VA.

### `0x588d00` — free marked tree nodes (support)

- **VA / file offset**: `0x588d00` / `0x188d00` (1,608,960).
- **Behavior**: Walks child links while a node's flag byte is clear, recurses into the left child, frees the node and repeats on the saved next pointer.
- **Reached from**: `0x588430`, `0x588d00`.
- **Player-visible reach**: support recursive cleanup; called by the list reset above and by itself..
- **Evidence**: ev 4ca8.0037.2d2b.2084.73d1.fe7c.e1da.6d3b.3b0d.daa8.af8a.c01e.3eb7.190d.cadb.bcf9.
- **Our client**: No counterpart by VA; Rust ownership frees nodes.

### `0x589a10` — reserve a vector of six-byte records (support)

- **VA / file offset**: `0x589a10` / `0x189a10` (1,612,304).
- **Behavior**: Clears begin/end/capacity, allocates through `0x58bf90` for a nonzero request, and computes the end as the begin pointer plus six bytes per element. Oversized requests reach `0x58b7b0`.
- **Reached from**: `0x586b50`, `0x58d260`, `0x5b3720`.
- **Player-visible reach**: support container code; callers include relay-message init at `0x58d260`..
- **Evidence**: ev d382.8a5b.d875.1339.855d.2c24.8584.91e6.577f.a7e3.71e2.d1c4.d51e.bfd2.ea8c.9f4a.
- **Our client**: No counterpart by VA; Rust `Vec`.

### `0x589ad0` — free a vector of six-byte records (support)

- **VA / file offset**: `0x589ad0` / `0x189ad0` (1,612,496).
- **Behavior**: Frees the begin pointer when it is present and then zeroes begin, end and capacity.
- **Reached from**: `0x586b50`, `0x5889d0`, `0x58d3a0`.
- **Player-visible reach**: support container code..
- **Evidence**: ev 520c.25bc.e656.e79b.125c.9d38.a488.988f.449b.5fbf.24d1.d344.4c54.f792.6327.3f87.
- **Our client**: No counterpart by VA; Rust drop.

### `0x58aaf0` — insert a node before an intrusive-list node (support)

- **VA / file offset**: `0x58aaf0` / `0x18aaf0` (1,616,624).
- **Behavior**: Moves the target node's child link to the new position, fixes parent/back pointers and head references, and writes the inserted node's own left/right links.
- **Reached from**: `0x589440`, `0x58a460`.
- **Player-visible reach**: support RB-tree rebalance; no pixels or wire..
- **Evidence**: ev 0480.57ad.8936.eea5.b3f8.8c98.e287.a066.5a41.a36b.528e.c3e3.d82f.6bb8.4ee6.b30b.
- **Our client**: No counterpart by VA; Rust collections own rebalancing.

### `0x58aba0` — replace a node after an intrusive-list move (support)

- **VA / file offset**: `0x58aba0` / `0x18aba0` (1,616,800).
- **Behavior**: Saves a node's child link, rewires parent and head pointers around the moved node, and restores its back link.
- **Reached from**: `0x589440`, `0x58a460`.
- **Player-visible reach**: support RB-tree rebalance; no pixels or wire..
- **Evidence**: ev 83a0.8cb9.cbe4.bb32.fae0.eb33.fdb3.cd8a.085b.d1ea.d439.07cc.5db1.7a98.d864.098b.
- **Our client**: No counterpart by VA.

### `0x58ac50` — allocate and link a tree node (support)

- **VA / file offset**: `0x58ac50` / `0x18ac50` (1,616,976).
- **Behavior**: Allocates a 40-byte node through `0x58c050`, stores nine supplied words plus a flag byte, clears a second flag byte, then runs the insertion epilogue `0x58ad2f`.
- **Reached from**: `0x589440`.
- **Player-visible reach**: support node creation used by the map/set helper `0x589440`..
- **Evidence**: ev 1174.b56e.3ce7.0866.bd67.55da.9d33.8349.b090.2fbf.a630.11cf.dd0e.a8ac.82ac.2b4c.
- **Our client**: No counterpart by VA; Rust `BTreeMap::insert`.

### `0x58ad2f` — finish tree-node insertion (support)

- **VA / file offset**: `0x58ad2f` / `0x18ad2f` (1,617,199).
- **Behavior**: SEH epilogue that restores the exception list and returns the saved value; called only by `0x58ac50`.
- **Reached from**: `0x58ac50`.
- **Player-visible reach**: support; no behavior of its own..
- **Evidence**: ev 3fed.57ae.d7d4.254a.0d4c.8dd9.b0d6.ecd6.2bbe.5e71.2193.0dd4.ec9c.3819.57a0.2afb.
- **Our client**: No counterpart by VA.

### `0x58af10` — insert a range of six-byte records (support)

- **VA / file offset**: `0x58af10` / `0x18af10` (1,617,680).
- **Behavior**: Computes current size/capacity in six-byte units; if needed it allocates a larger block through `0x58bf90`, copies the prefix and suffix with `0x58c2a0`, writes the new records with `0x58c0c0`, then calls `0x58b34c`.
- **Reached from**: `0x5898e0`.
- **Player-visible reach**: support container code called by the six-byte vector's insert path..
- **Evidence**: ev ed51.2f52.7f7e.f56d.d40c.278d.8bca.e22c.4cae.1ec1.2ba6.c2f5.9c01.c01c.8084.d043.
- **Our client**: No counterpart by VA; Rust `Vec::splice`/`insert`.

### `0x58b34c` — rebalance a six-byte vector after insertion (support)

- **VA / file offset**: `0x58b34c` / `0x18b34c` (1,618,764).
- **Behavior**: Recomputes the element count, advances a saved iterator, frees the old block, then writes new begin/end/capacity words derived from the replacement buffer.
- **Reached from**: `0x58af10`.
- **Player-visible reach**: support; inlined allocation epilogue of `0x58af10`..
- **Evidence**: ev cf26.ace3.54f0.4b42.b9c8.80ec.2ebe.dc78.ff1d.d42a.3c71.9687.e53d.2663.5ccb.fc5b.
- **Our client**: No counterpart by VA.

### `0x58c0c0` — copy counted six-byte records (support)

- **VA / file offset**: `0x58c0c0` / `0x18c0c0` (1,622,208).
- **Behavior**: Copies the first four bytes and following two bytes of each record for a caller-supplied count, advancing both pointers by six bytes, then calls the SEH epilogue `0x58c175`.
- **Reached from**: `0x5889f0`, `0x58af10`.
- **Player-visible reach**: support container code called by `0x58af10` and the single-record append path..
- **Evidence**: ev 04f3.e911.dcd4.797b.49d2.4c43.d113.bccf.b1e8.7ad4.7530.cf9c.21ce.4d7a.413c.d7ce.
- **Our client**: No counterpart by VA; Rust slice copy.

### `0x58c175` — finish a six-byte copy (support)

- **VA / file offset**: `0x58c175` / `0x18c175` (1,622,389).
- **Behavior**: SEH epilogue that restores the exception list and returns the saved value; called only by `0x58c0c0`.
- **Reached from**: `0x58c0c0`.
- **Player-visible reach**: support; no behavior of its own..
- **Evidence**: ev 9ae5.a553.0797.8d48.9f8f.e0ba.419f.25ca.b546.6d47.edb6.9f6d.a4a4.f323.6600.39ca.
- **Our client**: No counterpart by VA.

### `0x58c2a0` — forward a six-byte range copy (support)

- **VA / file offset**: `0x58c2a0` / `0x18c2a0` (1,622,688).
- **Behavior**: Passes four arguments unchanged to `0x58c2f0`.
- **Reached from**: `0x58af10`.
- **Player-visible reach**: support thunk; no behavior of its own..
- **Evidence**: ev 92f8.5175.3bc7.7491.5d03.a21c.52c4.cd85.20b5.deb4.2e12.0ed1.ab9f.650d.21df.9bf8.
- **Our client**: No counterpart by VA.

### `0x58c2f0` — copy a range of six-byte records (support)

- **VA / file offset**: `0x58c2f0` / `0x18c2f0` (1,622,768).
- **Behavior**: Loops from source begin to source end, copying four bytes plus two bytes into the destination and advancing both by six, then calls epilogue `0x58c3a7`.
- **Reached from**: `0x58c190`, `0x58c2a0`, `0x5b3560`.
- **Player-visible reach**: support container code used by insert and relocation paths..
- **Evidence**: ev 7d7c.02f6.447c.5591.1078.f22e.d913.b462.70cf.3cd2.4d50.a22f.6b33.d976.8d7d.a358.
- **Our client**: No counterpart by VA.

### `0x58c3a7` — finish a six-byte range copy (support)

- **VA / file offset**: `0x58c3a7` / `0x18c3a7` (1,622,951).
- **Behavior**: SEH epilogue restoring the exception list and returning the saved value; called only by `0x58c2f0`.
- **Reached from**: `0x58c2f0`.
- **Player-visible reach**: support; no behavior of its own..
- **Evidence**: ev a952.832c.da64.7973.b6c6.f80b.16fe.7178.aa90.0bc9.c646.e72e.1e0f.40e7.b176.c587.
- **Our client**: No counterpart by VA.

### `0x58d030` — forward to the relay handshake worker (support)

- **VA / file offset**: `0x58d030` / `0x18d030` (1,626,160).
- **Behavior**: Tail-calls `0x58d580`, which builds the relay entry list, checks UDP hole-punch state, writes a status string, and sends opcode `0xfce`.
- **Reached from**: `0x58cf50`.
- **Player-visible reach**: support for live-battle relay only; reached from the relay reply setup path `0x58cf50`..
- **Evidence**: ev 8b3c.d18c.7701.70d1.5046.b9c1.8c2f.f316.a45d.a04d.37da.eaf6.e01c.24af.a336.ba7d.
- **Our client**: No counterpart by VA; the Rust relay model is in `crates/o2jam-net/src/`; no live-battle guard from this pin.

### `0x58d050` — forward to relay-state clear (support)

- **VA / file offset**: `0x58d050` / `0x18d050` (1,626,192).
- **Behavior**: Tail-calls `0x58d440`, the relay message clear path.
- **Reached from**: `0x58cf50`.
- **Player-visible reach**: support for live-battle relay; reached from `0x58cf50`..
- **Evidence**: ev ec10.e895.d743.afc6.9759.35f2.1d2c.068c.28fb.cac9.f0e8.8dc9.481a.bb69.2b52.a1c1.
- **Our client**: No counterpart by VA.

### `0x58d440` — clear relay message state (support)

- **VA / file offset**: `0x58d440` / `0x18d440` (1,627,200).
- **Behavior**: Clears the word at `+0x101bc`, calls two range helpers (`0x58f260`, twice `0x58f090`), then resets both intrusive trees through `0x586a30`.
- **Reached from**: `0x58d050`.
- **Player-visible reach**: support cleanup of the relay-server info message; no pixels on normal play..
- **Evidence**: ev 68f3.67dd.850c.6578.d334.0b15.022f.0bc4.6c1b.7f68.58d8.1953.6aa0.a6ad.2405.92e2.
- **Our client**: No counterpart by VA.

### `0x58d4b0` — store the relay mode word (support)

- **VA / file offset**: `0x58d4b0` / `0x18d4b0` (1,627,312).
- **Behavior**: Writes the caller's word into the relay object's `+0x101b0`. The caller `0x58cf50` passes the global channel/mode word.
- **Reached from**: `0x58cf50`.
- **Player-visible reach**: support for live-battle relay..
- **Evidence**: ev a18c.eef9.d48d.76b2.0c3f.1edf.f8a2.285c.b24a.a27c.5073.c711.59bb.01ff.e8f4.4f6b.
- **Our client**: No counterpart by VA.

### `0x58d4d0` — set the relay three-word block (support)

- **VA / file offset**: `0x58d4d0` / `0x18d4d0` (1,627,344).
- **Behavior**: Builds the values 3, 0 and `-1` and passes their address to `0x586950`, which stores them at `+0x130`..`+0x138`.
- **Reached from**: `0x58cf50`.
- **Player-visible reach**: support for relay message setup..
- **Evidence**: ev 870c.62c3.a44b.4424.f964.8e63.e758.6173.cb39.a264.587d.fa0f.d1b8.e4b5.5ec5.9067.
- **Our client**: No counterpart by VA.

### `0x58d510` — append two relay record fields (support)

- **VA / file offset**: `0x58d510` / `0x18d510` (1,627,408).
- **Behavior**: Appends a four-byte value through `0x58f160` and a six-byte value twice through `0x5889f0`, then sets a flag byte at `DAT_0x768358 + 0x103e4 + id*4` to 1.
- **Reached from**: `0x58cf50`.
- **Player-visible reach**: support: fills relay record arrays while parsing a wire reply; no pixels or wire here..
- **Evidence**: ev b69e.a818.cd00.645a.205f.62ce.b92a.cdf7.c3be.7759.fbcb.b20f.7aac.40a2.db90.c4ff.
- **Our client**: No counterpart by VA.

### `0x58d910` — prepare the relay status stream (support)

- **VA / file offset**: `0x58d910` / `0x18d910` (1,628,432).
- **Behavior**: Calls `0x58dbd0` to initialize stream state from an object field. If a local flag is set it passes a byte to `0x58db10` and returns the result of `0x58da46`; otherwise it ORs a stream word with 4, calls `0x508df0`, checks a global byte, then flushes via `0x58dd20`/`0x58de10`.
- **Reached from**: `0x58d580`, `0x58de60`.
- **Player-visible reach**: support for the live-battle relay handshake log/status path; called by `0x58d580` and `0x58de60`..
- **Evidence**: ev a4aa.b741.665e.5b9c.0c00.355a.6550.5864.8f77.75e5.3efb.ae22.a860.0b83.6c07.03b7.
- **Our client**: No counterpart by VA.

### `0x58da46` — update relay record flags and flush (support)

- **VA / file offset**: `0x58da46` / `0x18da46` (1,628,742).
- **Behavior**: Forms the address of the current relay record from a base pointer plus an offset, ORs a word with the caller mask (forcing bit 4 when a related word is zero), calls `0x508df0`, checks a global byte, then flushes with `0x58dd20`/`0x58de10`.
- **Reached from**: `0x58d910`.
- **Player-visible reach**: support for relay status records; no pixels on normal play..
- **Evidence**: ev 9a01.2f85.c89a.b098.feef.9254.8546.66b2.320b.51b5.7d48.d3f0.9608.2a77.bba6.77bb.
- **Our client**: No counterpart by VA.

### `0x58db10` — put one byte into a stream buffer (support)

- **VA / file offset**: `0x58db10` / `0x18db10` (1,628,944).
- **Behavior**: If the stream has no put area it calls the stream's virtual put method; otherwise it reserves a slot through `_Pninc`, writes the byte there and returns it.
- **Reached from**: `0x58d910`, `0x58ff70`.
- **Player-visible reach**: support byte-output primitive shared by the relay status path..
- **Evidence**: ev 211c.8e21.1d56.50eb.2b3e.9954.aed8.0607.746e.70fe.641d.f78b.3b27.3fa2.c14d.1638.
- **Our client**: No counterpart by VA; Rust `Write`/`BufWriter`.

### `0x58dbd0` — initialize a stream wrapper (support)

- **VA / file offset**: `0x58dbd0` / `0x18dbd0` (1,629,136).
- **Behavior**: Calls `0x58ddb0` with the supplied stream pointer, then tests two words inside the stream record; when the first is zero and the second nonzero it calls `0x509df0`. Stores the zero/nonzero result in the wrapper's flag field.
- **Reached from**: `0x58d910`, `0x58ff70`.
- **Player-visible reach**: support for relay status stream work..
- **Evidence**: ev 2a45.69e6.fccd.35c9.0b3d.7a75.5f75.e228.36fa.82c7.34e5.348d.ebb5.78b5.ed48.bf70.
- **Our client**: No counterpart by VA.

### `0x58dd20` — flush the relay status stream (support)

- **VA / file offset**: `0x58dd20` / `0x18dd20` (1,629,472).
- **Behavior**: Calls `0x509df0` when bit 2 of a stream flag word is set, then runs the `0x58dd96` epilogue.
- **Reached from**: `0x58da46`, `0x58dcb0`, `0x59025a`.
- **Player-visible reach**: support for relay status stream work..
- **Evidence**: ev 7f12.b5cb.a2e9.30d7.9f1f.d440.3d03.c873.3a43.219c.6f6a.9fed.8ddb.d33e.0bb5.302d.
- **Our client**: No counterpart by VA.

### `0x58dd96` — finish the stream flush (support)

- **VA / file offset**: `0x58dd96` / `0x18dd96` (1,629,590).
- **Behavior**: SEH epilogue restoring the exception list; called only by `0x58dd20`.
- **Reached from**: `0x58dd20`.
- **Player-visible reach**: support; no behavior of its own..
- **Evidence**: ev 8d4b.3334.4dcb.5d13.e694.6e15.b32f.e7ae.58ab.62c0.04d1.888c.bfd9.6cdc.923b.d908.
- **Our client**: No counterpart by VA.

### `0x58ddb0` — store a stream pointer and test a flag (support)

- **VA / file offset**: `0x58ddb0` / `0x18ddb0` (1,629,616).
- **Behavior**: Stores the supplied pointer in the wrapper and calls `0x5f0daa` when a word inside the stream record is nonzero.
- **Reached from**: `0x58dbd0`.
- **Player-visible reach**: support for stream wrapper setup..
- **Evidence**: ev d139.1dba.eb24.93bd.49e1.24ea.4113.b534.9535.03bd.5241.4137.d8cf.6433.61bb.5be2.
- **Our client**: No counterpart by VA.

### `0x58de10` — test the stream flag again (support)

- **VA / file offset**: `0x58de10` / `0x18de10` (1,629,712).
- **Behavior**: Calls `0x5f0db3` when the same stream-record word is nonzero; used after relay status writes.
- **Reached from**: `0x58da46`, `0x58dcb0`, `0x59025a`, `0x636118`, `0x636140`, `0x636170` (7 direct callers total).
- **Player-visible reach**: support for stream wrapper cleanup..
- **Evidence**: ev 5f0b.ffd5.c2b3.7362.cd24.4a2f.2437.44d6.b3e1.4f8f.8f2c.3c2f.c921.2a5d.efe5.bcc0.
- **Our client**: No counterpart by VA.

### `0x58f6b0` — insert a range of four-byte words (support)

- **VA / file offset**: `0x58f6b0` / `0x18f6b0` (1,636,016).
- **Behavior**: Mirrors `0x58af10` for four-byte elements: grows storage through `0x590440`, copies prefix/suffix through `0x590600`, writes the new words through `0x590590`, then calls `0x58fa99`.
- **Reached from**: `0x58f3c0`.
- **Player-visible reach**: support container code called by the four-byte vector insert path..
- **Evidence**: ev 0885.a047.9d65.e3c6.48df.f39f.7f75.1ee0.fd41.c713.e87e.0047.bd63.3278.4ee6.a9a2.
- **Our client**: No counterpart by VA; Rust `Vec`.

### `0x58fa99` — rebalance a four-byte vector after insertion (support)

- **VA / file offset**: `0x58fa99` / `0x18fa99` (1,637,017).
- **Behavior**: Recomputes the element count in four-byte units, frees the old block, and writes replacement begin/end/capacity words.
- **Reached from**: `0x58f6b0`.
- **Player-visible reach**: support; inlined allocation epilogue of `0x58f6b0`..
- **Evidence**: ev 6c32.596b.e726.a13c.c56f.9cfc.d179.f905.f91b.aa9c.0fa3.8920.f812.6927.c8e4.57aa.
- **Our client**: No counterpart by VA.

### `0x58ff70` — compare or skip a text field in a relay stream (support)

- **VA / file offset**: `0x58ff70` / `0x18ff70` (1,638,256).
- **Behavior**: Measures the supplied text, computes the remaining bytes from a stream field, initializes stream state through `0x58dbd0`, skips bytes unless the stream flags select a different path, compares the text through the stream's virtual method, resets the length word and returns the result of `0x59025a`. The relay worker calls it with the `UDP Hole Punching Success~!!` literal.
- **Reached from**: `0x58d580`.
- **Player-visible reach**: support for the live-battle relay handshake status text; no normal-play pixels..
- **Evidence**: ev 144c.37ee.f7aa.66ae.d2ad.8840.eff2.8d19.158e.17bd.dd23.8939.0bd8.1fc6.94c5.5cab.
- **Our client**: No counterpart by VA.

## Census rows touched by this pin set (rows stay OPEN)

- Staged cursor: picked-band and channel/planet highlight rows in
  `docs/interaction-census.md`; the amber stand-in is explicitly OPEN, not
  exe-parity art (`docs/planet.md`, `docs/parity/channel.md`).
- Browser-hosted dialogs: Room web user-list / desc well (census Room ord
  69-70), payment / billing / event / help sheets (spec #315/#316 rows in
  `docs/interaction-census.md`). No live page frame is claimed by this lane.
- Live-battle relay rows: support only; no normal Playing row is touched.

The spec-shaped copy of the player-visible subset is in
`/home/arnim/scratch/lanes-2026-11-25/spec-enrich-pins-web-asset.md`;
the query log and OPENs are in
`/home/arnim/scratch/lanes-2026-11-25/findings-pins-web-asset.md`.
