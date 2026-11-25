# Static pins — net-a bucket (36 functions, coverage burn-down)

Target pin: `OTwo.exe` 7.55, 5,029,888 B, SHA-256
`83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
Tools: Ghidra 12.2 / REA 6.10.1. Evidence IDs below are the parent IDs of
REA `analyze_function` dossiers on that exact artifact, written as the dotted
four-hex group form so `scripts/check-cited-hashes.sh` does not read them as
commits. Worktree branch `cb2/pins-net-a`; brief base
`a15244e1cebcc72a16f8d9d84401ccab895343f2`, observed HEAD
`e5e4fb7f918752fd12e49ab606e6bc07db7f5c7c`.

Scope: the 36 addresses in `slice-net-a.txt` from the coverage triage, all
`bucket=game-reachable,pinned_in_repo=no`. Pinned here: **36/36**; left: **0**.
File offsets were checked one by one with REA `address_to_file_offset`; all 36
equal VA − `0x400000`. Behavior text is our own description of REA output;
no disassembly or decompiled text is reproduced.

A static pin is not a player-visible close. "Reached from" records the
provider's direct-caller edges, not a runtime press. Rows in
`docs/interaction-census.md` stay OPEN until a live caller chain from boot
is recorded; see `docs/live-path-reachability.md`.

## Member record wire readers and direct setters (24)

### `0x418dc0` — enqueue a capture request, or drain one

- **VA / file offset**: `0x418dc0` / `0x18dc0` (101,824).
- **Behavior**: Reads a pending-request count and a stored request id on the owner object. With no pending count it sends one internal event 0x67 through the queue writer. Otherwise it passes the stored id to two workers, advances the id and reduces the count.
- **Reached from**: 2 direct callers observed: `0x4e15f0`, `0x5a5190`.
- **Player-visible reach**: support — the WndProc calls it for key 0x2c (PrintScreen) and the game ticker calls it each frame; no pixels are painted here.
- **Evidence**: `ev 52e5.59f7.df3d.44f6.e10c.9c57.ce6d.aa2f.dd77.a376.1c27.b615.eb00.0092.62fb.6928`.
- **Our client**: No counterpart by VA. The client has a CLI screenshot path (`crates/recomp2jam/src/main.rs:171`), but no in-game capture-request ring.

### `0x5400d0` — set the member team byte directly

- **VA / file offset**: `0x5400d0` / `0x1400d0` (1,310,928).
- **Behavior**: Stores the supplied byte at member-slot `+0x31`; no packet read.
- **Reached from**: 12 direct callers observed: `0x495110`, `0x4aac80`, `0x4ae350`, `0x4b4d30`, `0x4f16f0`, `0x559b50`, ….
- **Player-visible reach**: yes — feeds the team mark on Room/Waiting seat rows.
- **Evidence**: `ev 4e62.b1ac.8a78.a55a.5708.60bd.5c4e.e88e.e895.41f7.8eb8.1011.9fa5.3d1f.9075.e046`.
- **Our client**: `MEMBER_OFF_TEAM` (`crates/o2jam-net/src/relay/commands.rs:918`) and `MemberInfo::team` (`:1201`).

### `0x5400f0` — set the member ready byte directly

- **VA / file offset**: `0x5400f0` / `0x1400f0` (1,310,960).
- **Behavior**: Stores the supplied byte at member-slot `+0x32`; no packet read.
- **Reached from**: 20 direct callers observed: `0x495110`, `0x4ab330`, `0x4ae070`, `0x4ae350`, `0x4b0a50`, `0x4b4a00`, ….
- **Player-visible reach**: yes — feeds the ready mark; the exe later derives ready from seat position for some senders.
- **Evidence**: `ev 1ae1.d33a.bfc6.b416.a73c.b349.b17b.a74a.aa21.b990.d0c8.18d0.e9ec.9025.bbb2.27cf`.
- **Our client**: `MEMBER_OFF_READY` (`crates/o2jam-net/src/relay/commands.rs:926`) and `slot_position_marks` (`:1065`).

### `0x540110` — set the local-only spare flag

- **VA / file offset**: `0x540110` / `0x140110` (1,310,992).
- **Behavior**: Stores the supplied byte at member-slot `+0x33`; no packet read. Static traces find no wire reader for this field.
- **Reached from**: 28 direct callers observed: `0x495110`, `0x4c2cf0`, `0x4db6e0`, `0x4de400`, `0x4e5130`, `0x4e5250`, ….
- **Player-visible reach**: support — ticker-side effect selector, not a wire field.
- **Evidence**: `ev 8e89.49c9.dd0c.5fb0.5b7f.9709.78ca.8bad.3874.fe19.d102.ded2.db35.aad2.d70e.5e55`.
- **Our client**: `MEMBER_OFF_SPARE_FLAG` (`crates/o2jam-net/src/relay/commands.rs:1038`).

### `0x540bd0` — decode one member id byte

- **VA / file offset**: `0x540bd0` / `0x140bd0` (1,313,744).
- **Behavior**: Consumes one byte from the shared packet cursor into member-slot `+0x04`.
- **Reached from**: 3 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d820`.
- **Player-visible reach**: yes — seat identity in the Room/Waiting roster.
- **Evidence**: `ev a57b.2c56.1225.00fa.8558.cbec.da46.e1d4.81c4.ca63.87d7.6414.a535.e104.41bd.24c8`.
- **Our client**: `MEMBER_OFF_ID` (`crates/o2jam-net/src/relay/commands.rs:901`); decoded by `MemberInfo::decode_body` (`:1352`).

### `0x540c50` — decode one member level word

- **VA / file offset**: `0x540c50` / `0x140c50` (1,313,872).
- **Behavior**: Consumes four bytes from the packet cursor into member-slot `+0x20`.
- **Reached from**: 6 direct callers observed: `0x559b50`, `0x55a350`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x560160`.
- **Player-visible reach**: yes — level shown on member rows.
- **Evidence**: `ev cb14.5e90.0ad5.54f1.4cc6.c8e8.f35e.9051.4562.a9f3.313b.c198.5db1.f001.0ac6.6c9c`.
- **Our client**: `MEMBER_OFF_LEVEL` (`crates/o2jam-net/src/relay/commands.rs:908`); `MemberInfo::level` (`:1195`).

### `0x540c90` — decode gender and select the control id pair

- **VA / file offset**: `0x540c90` / `0x140c90` (1,313,936).
- **Behavior**: Consumes one byte into member-slot `+0x28` through the shared setter. That setter stores the byte and selects control id 0x24 for zero or 0x23 for nonzero.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55a350`, `0x55cad0`, `0x55d420`, `0x55d820`.
- **Player-visible reach**: yes — gender state plus the paired icon/control set used by the seat and room UI.
- **Evidence**: `ev b85e.5113.1d44.3596.677c.092e.7ecb.f1ee.aaa8.c25f.2809.6bbc.3999.3b00.c994.d631`.
- **Our client**: `MEMBER_OFF_GENDER` (`crates/o2jam-net/src/relay/commands.rs:912`); no counterpart for the 0x23/0x24 control-id choice.

### `0x540cc0` — decode one member gem word

- **VA / file offset**: `0x540cc0` / `0x140cc0` (1,313,984).
- **Behavior**: Consumes four bytes into member-slot `+0x2c`.
- **Reached from**: 4 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`.
- **Player-visible reach**: yes — gem count on member rows.
- **Evidence**: `ev d70f.a865.abd7.1421.7afe.1a25.b3be.f202.313f.58c5.eb54.825a.76bb.2113.7511.f452`.
- **Our client**: `MEMBER_OFF_GEM` (`crates/o2jam-net/src/relay/commands.rs:915`).

### `0x540ce0` — decode one cash-point word

- **VA / file offset**: `0x540ce0` / `0x140ce0` (1,314,016).
- **Behavior**: Consumes four bytes into member-slot `+0x2128`.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x560160`.
- **Player-visible reach**: yes — cash readout on member rows.
- **Evidence**: `ev c876.191d.5847.f50f.b9c7.e68c.3ec4.d0e6.ece3.1472.4341.4d55.261c.8046.f990.baf0`.
- **Our client**: `MEMBER_OFF_CASH_POINT` (`crates/o2jam-net/src/relay/commands.rs:981`).

### `0x540d20` — decode the room-master byte

- **VA / file offset**: `0x540d20` / `0x140d20` (1,314,080).
- **Behavior**: Consumes one byte into member-slot `+0x30`.
- **Reached from**: 3 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d820`.
- **Player-visible reach**: yes — master badge and master-only buttons.
- **Evidence**: `ev adfb.276f.245e.3a6b.b041.0467.81e0.b1ed.3975.c0b4.8bb2.2b2d.46c7.066a.a810.fc5c`.
- **Our client**: `MEMBER_OFF_ROOM_MASTER` (`crates/o2jam-net/src/relay/commands.rs:1021`); `trailing_flag_is_room_master_not_administrator` (`:2484`).

### `0x540d40` — decode the team byte and advance it

- **VA / file offset**: `0x540d40` / `0x140d40` (1,314,112).
- **Behavior**: Consumes one byte into member-slot `+0x31`, then increments that same byte.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x55f3f0`.
- **Player-visible reach**: yes — team marking on seat rows; the increment is part of the stored value.
- **Evidence**: `ev ca8b.21e6.2b6c.802f.51b8.e416.51cf.8691.6619.ffea.81f7.5481.2255.507a.01e5.4f90`.
- **Our client**: `MEMBER_OFF_TEAM` (`crates/o2jam-net/src/relay/commands.rs:918`); the increment has no separate Rust counterpart.

### `0x540d70` — decode the wire ready byte

- **VA / file offset**: `0x540d70` / `0x140d70` (1,314,160).
- **Behavior**: Consumes one byte into member-slot `+0x32`. Later slot-index branches may overwrite it without a new wire read.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x55f5b0`.
- **Player-visible reach**: yes — ready badge on seat rows.
- **Evidence**: `ev fed8.f64f.57c2.8069.2b1d.0cd8.7103.f101.b834.9df7.eac6.eec6.def8.00f7.e13f.eb65`.
- **Our client**: `MEMBER_OFF_READY` (`crates/o2jam-net/src/relay/commands.rs:926`); `slot_position_marks` (`:1065`).

### `0x540db0` — decode the owned-music list

- **VA / file offset**: `0x540db0` / `0x140db0` (1,314,224).
- **Behavior**: Consumes a four-byte count, then that many two-byte entries. The low twelve bits go to `+0x38` slots and the high bits to paired `+0x1038` slots.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x563880`.
- **Player-visible reach**: yes — owned-song catalog used by Waiting music selection.
- **Evidence**: `ev 0cb8.7310.0008.4183.02d9.dba8.7362.d81c.4948.2b56.8f14.ca72.030a.b331.59c3.0078`.
- **Our client**: `MEMBER_OFF_MUSIC_COUNT` / `MEMBER_OFF_MUSIC_IDS` (`crates/o2jam-net/src/relay/commands.rs:975`, `:978`); `MemberInfo::music_ids` (`:1209`).

### `0x540e40` — decode equipment words and choose default control ids

- **VA / file offset**: `0x540e40` / `0x140e40` (1,314,368).
- **Behavior**: Consumes a count at `+0x2038`, then reads the equipment words through a table of offsets. It looks up the channel record named by `+0x2068`, parks that record in a small slot table, then writes two different sets of default control ids based on the channel id and a stored byte. It returns 1 when the channel record has type byte 5.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55a350`, `0x55cad0`, `0x55d420`, `0x55d820`.
- **Player-visible reach**: yes — equipment readout and the paired channel/mode buttons.
- **Evidence**: `ev 00bb.3bfc.4b71.115a.d2ff.e4a9.3226.6f57.2564.2b9a.351d.a336.0faf.5c24.f105.3c8c`.
- **Our client**: `MEMBER_OFF_EQUIP_COUNT` / `MEMBER_OFF_EQUIPMENTS` (`crates/o2jam-net/src/relay/commands.rs:968`, `:971`); the default control-id choice has no counterpart.

### `0x541440` — decode one trailing member word

- **VA / file offset**: `0x541440` / `0x141440` (1,315,904).
- **Behavior**: Consumes four bytes into member-slot `+0x2178`. The callers place it with the other trailing member fields.
- **Reached from**: 1 direct caller observed: `0x55f630`.
- **Player-visible reach**: support — stored with member state; no in-exe consumer pinned, so its display role is OPEN.
- **Evidence**: `ev ae33.81a4.cf5f.2ce6.7032.2c44.5dab.e7ee.9691.bae1.2772.bbf0.61db.78df.e611.3bb9`.
- **Our client**: No counterpart by VA.

### `0x541480` — decode the music-state position byte

- **VA / file offset**: `0x541480` / `0x141480` (1,315,968).
- **Behavior**: Consumes one byte into member-slot `+0x217c`.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x560960`.
- **Player-visible reach**: yes — Waiting download/ready state; exact value semantics stay OPEN.
- **Evidence**: `ev 8af1.18b6.6524.0166.e378.3012.eb59.0e45.8e6e.a648.12f2.fe2d.578a.72ad.3ba4.6f74`.
- **Our client**: `MEMBER_OFF_MUSIC_STATE` (`crates/o2jam-net/src/relay/commands.rs:937`); `apply_member_music_state` (`crates/recomp2jam/src/session.rs:3573`).

### `0x541500` — set the first slot-group word directly

- **VA / file offset**: `0x541500` / `0x141500` (1,316,096).
- **Behavior**: Stores the supplied 16-bit value at member-slot `+0x2184`; no packet read.
- **Reached from**: 6 direct callers observed: `0x4aab60`, `0x4b0640`, `0x4ef300`, `0x4f1ca0`, `0x559b50`, `0x55d820`.
- **Player-visible reach**: yes — derived slot position used by the seat roster.
- **Evidence**: `ev 1d6c.c853.b2df.c19d.4d3a.a42f.04ff.3589.5648.2d5e.c5e8.282c.4834.5b63.aee7.dd84`.
- **Our client**: `slot_position_marks` (`crates/o2jam-net/src/relay/commands.rs:1065`) covers the paired derived values.

### `0x541520` — set the slot-group word directly

- **VA / file offset**: `0x541520` / `0x141520` (1,316,128).
- **Behavior**: Stores the supplied 16-bit value at member-slot `+0x2186`; no packet read.
- **Reached from**: 6 direct callers observed: `0x4aab60`, `0x4db6e0`, `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`.
- **Player-visible reach**: yes — derived seat grouping.
- **Evidence**: `ev f0cc.0cae.2057.3149.34f2.1da5.28ad.7503.99f1.46cf.8838.2448.efe3.7def.739b.a021`.
- **Our client**: `MEMBER_OFF_SLOT_GROUP` (`crates/o2jam-net/src/relay/commands.rs:1046`); `slot_position_marks` (`:1065`).

### `0x541560` — set the win-streak word directly

- **VA / file offset**: `0x541560` / `0x141560` (1,316,192).
- **Behavior**: Stores the supplied 32-bit value at member-slot `+0x218c`; no packet read.
- **Reached from**: 3 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d820`.
- **Player-visible reach**: yes — streak readout on member rows.
- **Evidence**: `ev a9a9.4f0d.3712.1dac.0ed3.ae35.2d59.334e.936f.3f56.ded8.0f3c.57c8.99d6.0787.54ce`.
- **Our client**: `MEMBER_OFF_WIN_STREAK` (`crates/o2jam-net/src/relay/commands.rs:1050`).

### `0x541580` — set the result speed byte directly

- **VA / file offset**: `0x541580` / `0x141580` (1,316,224).
- **Behavior**: Stores the supplied byte at member-slot `+0x2168`; no packet read.
- **Reached from**: 1 direct caller observed: `0x563fe0`.
- **Player-visible reach**: yes — speed digit on the Result board.
- **Evidence**: `ev 7d49.4808.4368.d8a1.ace3.1616.5479.e8de.883d.be15.35c2.7fb7.2235.6ad4.eaac.520f`.
- **Our client**: `o2jam-shell::set_song_rate` (`crates/o2jam-shell/src/screens/game/mod.rs:1846`); wire reader documented at `docs/net-protocol.md:184`.

### `0x5415a0` — decode the free-pass word

- **VA / file offset**: `0x5415a0` / `0x1415a0` (1,316,256).
- **Behavior**: Consumes four bytes into member-slot `+0x212c`.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x560160`.
- **Player-visible reach**: yes — free-pass readout on member rows.
- **Evidence**: `ev e2af.6081.f5d8.d8d2.6e78.0f05.1530.c688.a772.ce65.71c2.d841.e729.4070.88c2.fbd6`.
- **Our client**: `MEMBER_OFF_FREE_PASS` (`crates/o2jam-net/src/relay/commands.rs:984`).

### `0x5415c0` — set the free-pass word directly

- **VA / file offset**: `0x5415c0` / `0x1415c0` (1,316,288).
- **Behavior**: Stores the supplied 32-bit value at member-slot `+0x212c`; no packet read.
- **Reached from**: 1 direct caller observed: `0x55a350`.
- **Player-visible reach**: yes — derived free-pass state.
- **Evidence**: `ev ce29.6316.2d40.93ad.ce55.39d1.e586.e686.ff1b.13f4.7117.e790.b875.553e.2e74.d19f`.
- **Our client**: `MEMBER_OFF_FREE_PASS` (`crates/o2jam-net/src/relay/commands.rs:984`).

### `0x541670` — decode the playing-state byte

- **VA / file offset**: `0x541670` / `0x141670` (1,316,464).
- **Behavior**: Consumes one byte into member-slot `+0x2130`.
- **Reached from**: 5 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`, `0x560960`.
- **Player-visible reach**: yes — playing-state mark on member rows.
- **Evidence**: `ev 8194.4e12.51d7.4df1.a136.cb5c.c572.081c.524c.5d90.79f5.ae8c.6334.625c.a486.aded`.
- **Our client**: `MEMBER_OFF_PLAYING_STATE` (`crates/o2jam-net/src/relay/commands.rs:987`).

### `0x541690` — decode the live-extra byte

- **VA / file offset**: `0x541690` / `0x141690` (1,316,496).
- **Behavior**: Consumes one byte into member-slot `+0x2131`, directly after the playing-state read.
- **Reached from**: 4 direct callers observed: `0x559b50`, `0x55cad0`, `0x55d420`, `0x55d820`.
- **Player-visible reach**: yes — extra live marker; exact value semantics stay OPEN.
- **Evidence**: `ev 6aeb.2ada.373c.103a.9ed5.0f6b.e571.d2e5.d7b5.8599.19f2.f5a5.bbcd.c655.56b3.c245`.
- **Our client**: `MEMBER_OFF_LIVE_EXTRA` (`crates/o2jam-net/src/relay/commands.rs:992`).


## Control mode and input support (5)

### `0x5426d0` — set the control mode word and clear text caches

- **VA / file offset**: `0x5426d0` / `0x1426d0` (1,320,656).
- **Behavior**: Stores the argument in a mode word at `+0x7c`. When bits 0x8010 are set it asks the child at `+0x38`, through virtual slot `+0x28`, to copy 16 bytes into a 512-byte scratch area. It then clears both scratch buffers.
- **Reached from**: 76 direct callers observed: `0x432840`, `0x433c90`, `0x433d60`, `0x434350`, `0x434420`, `0x4349a0`, ….
- **Player-visible reach**: support — 76 direct callers in UI control factories and state dispatchers; the mode word selects later control behavior, but this function paints nothing.
- **Evidence**: `ev 99e4.f8a0.a7c3.7c47.2c0c.4a8e.6e1d.6907.ba39.36b9.c923.6911.3b3d.3ffa.3051.d2ac`; Xref sweep (76 call sites): ev e431.e5d9.33cd.8be5.86d1.73b5.67f4.d8ee.37ef.f380.d419.b3cb.b64c.ad18.d4c3.0d55.
- **Our client**: No counterpart by VA. Staged controls are dispatched by `ScreenGame::click_waiting_button` (`crates/o2jam-shell/src/screens/game/mod.rs:5206`).

### `0x542750` — read the control mode word

- **VA / file offset**: `0x542750` / `0x142750` (1,320,784).
- **Behavior**: Returns the `+0x7c` mode word. No other work.
- **Reached from**: 2 direct callers observed: `0x432510`, `0x548b10`.
- **Player-visible reach**: support — read by the control stager and the input callback below.
- **Evidence**: `ev e959.d61a.95b7.e5a6.2d6c.8571.0ffc.0ea2.1d06.a1fe.91ff.4dde.b3e3.b2b8.d237.883f`.
- **Our client**: No counterpart by VA.

### `0x543d50` — clear the two control scratch buffers

- **VA / file offset**: `0x543d50` / `0x143d50` (1,326,416).
- **Behavior**: Zeroes 0x100 bytes at `+0x88` and 0x94 bytes at `+0x188`.
- **Reached from**: 3 direct callers observed: `0x5426d0`, `0x543ea0`, `0x543f50`.
- **Player-visible reach**: support — invoked after a mode change and by stream readers; no pixels of its own.
- **Evidence**: `ev 1d7b.b67c.510c.31e7.efbf.e000.2669.37a6.9329.e136.2f6d.c0a9.59db.cd9c.7b9a.6348`.
- **Our client**: No counterpart by VA.

### `0x5488f0` — copy a stored pair out of a UI object

- **VA / file offset**: `0x5488f0` / `0x1488f0` (1,345,776).
- **Behavior**: Writes the two words at `+0x70` and `+0x74` into two caller outputs and also returns them as a pair in the first output.
- **Reached from**: 38 direct callers observed: `0x432090`, `0x43b160`, `0x43e9b0`, `0x460730`, `0x460f50`, `0x461500`, ….
- **Player-visible reach**: support — 38 callers in the loader and state painters; this getter emits no pixels itself.
- **Evidence**: `ev 3b51.2323.1b16.17d6.73b1.b288.00e0.d724.475e.a673.4e01.ecf2.c4bf.21c8.0e53.7d58`.
- **Our client**: No counterpart by VA. Staged geometry lives in `ScreenGame` (`crates/o2jam-shell/src/screens/game/mod.rs:2545`).

### `0x548b10` — enqueue a synthetic key when the mode bit is set

- **VA / file offset**: `0x548b10` / `0x148b10` (1,346,320).
- **Behavior**: Reads the mode word through `0x542750`. If bit 2 is set, it enqueues internal key 10 with phase 3 and no parameter through the queue writer.
- **Reached from**: 1 direct caller observed: `0x5a50b0`.
- **Player-visible reach**: support — reached from the window callback `0x5a50b0`; it feeds the UI input ring, not a paint path.
- **Evidence**: `ev 4e8e.3cba.4492.a528.b040.8a24.4dfe.dcf3.e94c.9cc7.18d4.1532.8236.30db.dae5.63ba`.
- **Our client**: No counterpart by VA. Input edges are `InputEdge` (`crates/o2jam-shell/src/lib.rs:220`); no synthetic phase-3 record is modeled.


## Audio worker and cue wrappers (7)

### `0x54bce0` — stop and re-arm the audio worker

- **VA / file offset**: `0x54bce0` / `0x14bce0` (1,359,072).
- **Behavior**: Sets two flag bits at `+0x80`, optionally a third from the argument, resets the acknowledgement event, signals the start event, then waits up to 1000 ms for the acknowledgement.
- **Reached from**: 1 direct caller observed: `0x551850`.
- **Player-visible reach**: yes — `0x551850` calls it from the focus/state change path `0x5a4b90`; the music stops while the state is inactive.
- **Evidence**: `ev 6cbc.b030.f246.641f.af4e.77ad.133a.8000.069d.f952.53f6.0130.5055.3af4.8ccf.637d`.
- **Our client**: `SceneBgm::silence` (`crates/o2jam-audio/src/bgm.rs:186`); no event handshake is modeled.

### `0x54c630` — release one audio worker object

- **VA / file offset**: `0x54c630` / `0x14c630` (1,361,456).
- **Behavior**: Calls `0x54c650` on the worker pointer.
- **Reached from**: 7 direct callers observed: `0x54b460`, `0x54b600`, `0x550950`, `0x550b80`, `0x550db0`, `0x551250`, ….
- **Player-visible reach**: support — teardown paths only; no sound is produced here.
- **Evidence**: `ev fe00.c637.96b9.67f9.9920.7c44.8ccf.4f05.92d8.db2c.6913.102d.40a8.300a.8216.09a2`.
- **Our client**: No counterpart by VA; Rust drop releases audio state.

### `0x54c650` — invoke the audio worker release method

- **VA / file offset**: `0x54c650` / `0x14c650` (1,361,488).
- **Behavior**: When the worker pointer is non-null, calls its virtual slot `+0x8`.
- **Reached from**: 3 direct callers observed: `0x54c630`, `0x54c680`, `0x54c810`.
- **Player-visible reach**: support — teardown paths only.
- **Evidence**: `ev d237.2843.d8dd.ab2a.a4ad.c302.20bb.f3bf.0c3e.e14c.0bb6.e454.455e.d9a6.b13f.bfd4`.
- **Our client**: No counterpart by VA; Rust drop releases audio state.

### `0x54cb00` — play one queued audio cue

- **VA / file offset**: `0x54cb00` / `0x14cb00` (1,362,688).
- **Behavior**: Requires a worker, runs the prepare step `0x54cbb0`, then queries its flag word. When flag bit 2 is set and a second virtual check fails, it returns that error. Otherwise it calls virtual slot `+0x30` with the cue argument and reports success.
- **Reached from**: 7 direct callers observed: `0x54bd60`, `0x54cc80`, `0x550b80`, `0x551870`, `0x551930`, `0x551a00`, ….
- **Player-visible reach**: yes — called by the scene and one-shot cue wrappers `0x551820` / `0x551870`.
- **Evidence**: `ev e96d.b599.dfd9.5859.1a9c.95ad.b084.a722.c78a.f438.6ba0.2f8a.e4b7.8d4a.538f.cb3d`.
- **Our client**: `SceneBgm::play_scene_music` (`crates/o2jam-audio/src/bgm.rs:147`) and `scene_bgm_cue` (`crates/recomp2jam/src/main.rs:1868`).

### `0x54cbb0` — prepare the audio worker before a cue

- **VA / file offset**: `0x54cbb0` / `0x14cbb0` (1,362,864).
- **Behavior**: Asks `0x54cd70` whether the worker is armed. If so, it calls virtual slots `+0x48` then `+0x34`, returning the first negative result.
- **Reached from**: 11 direct callers observed: `0x54b460`, `0x54bd60`, `0x54cb00`, `0x550950`, `0x551870`, `0x551930`, ….
- **Player-visible reach**: yes — preparation leg of every cue played through `0x54cb00`.
- **Evidence**: `ev 8f8c.60ee.74fc.7dec.0631.354b.30e9.4f66.71e1.d91b.7dd8.9d2a.7fe6.2115.0434.dc39`.
- **Our client**: `SceneBgm::cue` (`crates/o2jam-audio/src/bgm.rs:172`); the worker calls have no direct Rust analogue.

### `0x54ccd0` — select the active audio cue record

- **VA / file offset**: `0x54ccd0` / `0x14ccd0` (1,363,152).
- **Behavior**: When the worker exists and the requested record differs from the stored one, calls virtual slot `+0x3c` with it and saves the new record.
- **Reached from**: 8 direct callers observed: `0x54bd60`, `0x550b80`, `0x551440`, `0x5515c0`, `0x551870`, `0x551930`, ….
- **Player-visible reach**: yes — cue wrappers call it before playback.
- **Evidence**: `ev a6f2.fc55.e179.9b75.2d22.c5da.01ec.6690.e6cb.b21c.d4de.01cf.40a2.de88.ef9b.7dc2`.
- **Our client**: `SceneBgm::latch` (`crates/o2jam-audio/src/bgm.rs:209`).

### `0x54cd70` — query the audio armed flag

- **VA / file offset**: `0x54cd70` / `0x14cd70` (1,363,312).
- **Behavior**: Requires a worker, asks its virtual slot `+0x24` for a flag word, and returns true only when bit 1 is set.
- **Reached from**: 11 direct callers observed: `0x54cbb0`, `0x54cc20`, `0x551870`, `0x551930`, `0x5519c0`, `0x551a00`, ….
- **Player-visible reach**: support — read by prepare and cue wrappers; it emits no sound itself.
- **Evidence**: `ev 04d9.ba62.087f.107f.2d41.7c5c.92d1.066a.8cf9.02f2.9565.5b3c.3823.0cac.2b55.3dff`.
- **Our client**: `SceneBgm::is_playing` (`crates/o2jam-audio/src/bgm.rs:133`) plays the same role in the cue policy.

## Open items and census rows touched

- Every wire reader with an OPEN value semantics stays OPEN for meaning, even though its field offset is pinned: `+0x2178`, `+0x217c`, `+0x2131`, `+0x2186`, and the increment on `+0x31` have no in-exe consumer pinned here.
- The 0x23/0x24 control-id choice in `0x540c90` and the default control-id sets in `0x540e40` lack a Rust counterpart; the live button/id parity leg stays OPEN.
- The 0x5426d0 mode word, 0x548b10 synthetic key, and 0x5488f0 pair getter are support pins; their screen surfaces need a Tier 3 or Tier 4 leg before any census row can close.
- Audio worker pins describe the exe's event handshake; our client uses a latched cue policy. A Wine audio capture with a cue index from the backend remains the verification step.

The spec-shaped copy of the player-visible subset is in
`/home/arnim/scratch/lanes-2026-11-25/spec-enrich-pins-net-a.md`; the query log is
`findings-pins-net-a.md` in the same directory.
