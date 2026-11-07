# Playtest — end-to-end vs local Memoryer (2026-11-06)

Server: `~/memoryer-local`, Memoryer 7.2.0, `Mode=Full`,
`127.0.0.1:15010`, `FreeMusic=true`, `SessionExpiry=0`.
Account: `tester` (USER_INDEX_ID 1,
token `61dcb457-5da9-4e22-af4b-5a4c5658f239`).
No game bytes touched; Memoryer used as reference only.

## Full loop (network) — PASS

`cargo test -p recomp2jam live_host_song_flow_against_local_gateway`
with `O2JAM_LIVE_GATEWAY=127.0.0.1:15010` → **ok** (0.62 s).
Server log shows the same session, in order:

- `Authorize session` → `Get channel list` → `Enter channel [0/00]` →
  `Get character info [tester]`
- `Create room` (hosted room `songflow`)
- `Update room music` (select 757) → `Get music list` (562 entries)
  → `Sync room member music state` → `Update download progress` →
  `Start game` → `User music loaded` → `Game Update` (Jam tick +
  peer-bar echo) → `Game Submit score: 2210`
- `Finalize rank` + `Leave game` are sent by `send_tally` and were
  server-logged for the same flow earlier today (`tester2` run:
  `Submit score` → `Finalize rank` → `Leave game`); this run closed
  the session before the server flushed them (broken-pipe broadcast
  race on room delete, benign).

CLI `recomp2jam connect --host 127.0.0.1 --user-id 1 --token … \
--gateway-id 0 --channel-id 0 --character` → **ok**:

- `authorize: Success (0)`, `channels: 10` (0–4, 9 open),
  `channel: Success (0) (unrestricted)`,
  `character: tester (Lv 1, Male)`, `rooms: 0 open`.

Room join via CLI was not possible: 0 open rooms existed, and joining
a missing room hits the known Memoryer wart (throws
`ArgumentOutOfRangeException`, no reply — see `docs/client-server-plan.md`
Phase 2). The room step is proven by the live test's hosted create
instead. No invented passes.

## Scenes (in-process capture, no window) — 7 PASS, 0 FAIL

`cargo run -p recomp2jam -- shots` drives `ScreenGame` with
scripted `lane_key`/`menu_key` calls, rasterizes each scene with
`paint_at` at 400x300, and encodes std-only PNGs (none filter,
stored deflate; `crates/recomp2jam/src/shots.rs`). No OS key
injection, no window. Seeded data: 2 channels, 2 rooms, 1 ready
host, chart note at 5 s (struck for Result). `avi.png` is untouched:
the checked-in file stays the real-`O2mediaLogo.ojs` evidence and
the writer skips that stem.

| Scene | Verdict | Evidence |
|---|---|---|
| Avi (splash) | **PASS** (unchanged) | `avi.png`: real Nowcom logo from `install-root/O2Jam/Image/Interface1.opi` |
| Channel | **PASS** | `channel.png` (360,393 B): rows `2 5/100`, `9 0/0` + amber cursor |
| Room | **PASS** | `room.png` (360,393 B): rows `Jam`, locked `Locked` + cursor |
| Waiting | **PASS** | `waiting.png` (360,393 B): 8 seat rows, green ready dot for host |
| Loading | **PASS** | `loading.png` (360,393 B): green progress bar card |
| Playing | **PASS** | `playing.png` (360,393 B): highway + falling note at 4.5 s |
| Result | **PASS** | `result.png` (360,393 B): score `200`, combo `1` after chart end |

Placeholder shell art is still the known composition gap
(`docs/parity.md`, 7 FAIL cells) — these shots prove the walk and
the rasterizer, never art fidelity.

Prior windowed attempt (spectacle + XTEST/uinput) stays on record:
`input-blocked.png` showed the client stuck on Avi — XTEST drops
under Xwayland, uinput got no seat ACL, no Xephyr/Xvfb/xdotool/wtype
ydotool here. The in-process walk above supersedes it.

## Regression

`cargo test --workspace` (no live env): **444 passed, 0 failed**
(15 binaries), `exit=0`. The live-gateway test above ran separately
with env vars and passed.

## Regression sweep — 2026-11-06 12:28 UTC (HEAD `77f22ff`)

- `cargo test --workspace`: **507 passed, 0 failed** (1 ignored in
  `o2jam_audio`), `exit=0`. Per binary: audio 14, file 86, game 55,
  net 115, render 35, shell 149, real_art 7, scenes 22, recomp2jam 24.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`.
- `cargo fmt --check`: clean, `exit=0`.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test -p
  recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0.64 s) against local Memoryer 7.2.0 (`Mode=Full`).
  Note: the test requires both env vars; gateway alone aborts with
  `O2JAM_LIVE_TOKEN must hold a gateway session guid`.
- **Verdict: no regression vs the 507-passed baseline.**

## Regression sweep — 2026-11-06 20:56 UTC+8 (HEAD `0e5f3ca`, tree dirty)

- `cargo build --workspace --release`: clean, `exit=0`. 7 dead-code
  warnings on the peer's unwired WIP (`live_stats` fns, `SongFlowReport`
  peer fields).
- `recomp2jam shots` re-ran after scene changes: only
  `docs/screenshots/playing.png` bytes changed (peer-bar mirror paint
  from `0e5f3ca`); `avi.png` untouched (writer skips that stem).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  --release -p recomp2jam live_host_song_flow_against_local_gateway` →
  **ok** (1 passed, 0.62 s) against local Memoryer 7.2.0 (`Mode=Full`).
- `cargo test --workspace --no-fail-fast`: **600 passed, 4 failed**,
  `exit=101`. Failures, all in `o2jam-file` (peer's uncommitted `ojn.rs`
  rework, edited mid-run): `ojn::header_fields_parse_from_their_own_slots`,
  `ojn::tutorial_real_header_values_parse`,
  `ojn::old_format_levels_walk_with_exact_ranges`,
  `tests::cross_kind_is_type_error` (`parse_ojn(b"M30\\x00....")` no longer
  `Err(UnknownFormat)`). The 3 earlier `live_stats` rank-broadcast failures
  (`applied.ranks.len()` 0 vs 8) were fixed by the peer mid-sweep;
  `recomp2jam` bin ends 34 passed, 0 failed.
- **Verdict: REGRESSION vs the baseline — suite is red (4 failures),
  all inside concurrent uncommitted peer-mirror edits, not committed HEAD.
  No separate HEAD-only run was taken (would require stashing the peer's
  work). No commits, no pushes.**

## Regression sweep — 2026-11-06 13:01 UTC (HEAD `1e20e21`, tree clean)

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **604 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 106, game 68, net 156,
  render 35, shell 157, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Token re-verified present in
  `~/memoryer-local/O2JAM.db` `t_o2jam_login` (user `tester`, id 1).
  Server log for this run (`21:01:27 +08:00`): `Authorize session` →
  `Create room: songflow` → `Get music list: 562 music` →
  `Start game` → `Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (604 passed).
  No commits, no pushes.**

## Regression sweep — 2026-11-06 13:11 UTC (HEAD `3267ba1`, tree clean)

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **621 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 107, game 68, net 172,
  render 35, shell 157, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.64 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:10:57 +08:00`):
  `Create room: songflow` → `Get music list: 562 music` →
  `Start game` → `Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (621 passed;
  +17 vs the 604 baseline from new file/net tests, 0 failures).
  No commits, no pushes.**

## Regression sweep — 2026-11-06 13:21 UTC (HEAD `cd9e999`, tree clean)

Covers the equip (`4bcb4a8`) and winner (`5d87c4b`) HEAD.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **641 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 107, game 68, net 181,
  render 35, shell 168, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:21:40 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (641 passed;
  +20 vs the 621 baseline from new net/shell equip + winner tests,
  0 failures). No commits, no pushes.**

## Regression sweep — 2026-11-06 13:37 UTC (HEAD `6ac6dd0`, tree clean)

Covers the punch/relay (`05d52be`), song-wheel (`1dff270`), and side-screen
(`b5bbf6e`) HEADs.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **653 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 107, game 68, net 185,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines on a touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:37:09 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (653 passed;
  +12 vs the 641 baseline from new net/shell tests, 0 failures).
  No commits, no pushes.**

## Regression sweep — 2026-11-06 13:50 UTC (HEAD `ae36b9e`, tree clean at start)

Covers the gift-head re-proof: adjunct host + gift flows (`766f75e`)
and the wheel-filter leave-entry-open pin (`a84ca93`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **693 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 107, game 68, net 225,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines on a touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:50:31 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00]` → `Start game: [000]` →
  `Submit score: 2210`.
- **Verdict: no regression — suite is green on HEAD (693 passed;
  +40 vs the 653 baseline from new net gift/adjunct-flow tests,
  0 failures). No commits, no pushes.**
- Tree note: a peer landed uncommitted doc-comment-only edits mid-sweep
  (`o2jam-file/src/tables.rs`, `docs/file-formats/ojn.md` — comment text,
  no behavior change); `cargo test -p o2jam-file` re-run after: 107 passed,
  0 failed. This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-06 14:02 UTC (HEAD `b1d8529`, tree clean)

Covers the shop-head re-proof: shop trade + confirm relay empty
(`e9e46e1`), entry-label pins (`ad7248f`), gift-head log (`b1d8529`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **720 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 107, game 68, net 252,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines on a touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.64 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:02:55 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (720 passed;
  +27 vs the 693 baseline from new net shop-trade tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 14:12 UTC (HEAD `eb20dba`, tree dirty)

Covers the cart-head re-proof: cart purchase pair (`bb65587`), combo
+ autoplay/code pins (`492d6de`), filter-append map (`fae630d`), and the
shop-head log (`eb20dba`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **728 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 107, game 68, net 260,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:12:11 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green (728 passed;
  +8 vs the 720 baseline from new net cart tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.
- Tree note: a peer holds uncommitted wheel/filter work
  (`crates/o2jam-file/src/tables.rs`: new `OJN_WHEEL_LIST_COUNT`
  const + slot resolver plus doc comments; `docs/file-formats/ojn.md`:
  matching notes). `o2jam-file` still 107 passed, 0 failed under it.

## Regression sweep — 2026-11-06 14:20 UTC (HEAD `2ed5af3`, tree clean)

Covers the wheel-relation (`ef61672`), cart-tail (`ce3b304`), and
5005-tail-words (`2ed5af3`) HEADs.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **736 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 109, game 68, net 266,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines on a touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:19:56 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (736 passed;
  +8 vs the 728 baseline from new file/net tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 14:36 UTC (HEAD `d56ceec`, tree clean)

Covers the tail batch: live-state + p2p list decode (`03f8355`), gift
widths/verdicts (`4c5d494`), doubled-initial arms resolve (`cbdb2a8`),
and the tail-head log (`d56ceec`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **763 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 111, game 68, net 291,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:36:34 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (763 passed;
  +27 vs the 736 baseline from new net relay/gift + file arms tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 14:48 UTC (HEAD `fdbb601`, tree clean)

Covers the slot batch: member slots + equip tail (`656cdfc`), ojn
header offsets (`c808dee`), and the tail2-head log (`fdbb601`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **774 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 111, game 68, net 302,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:48:23 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (774 passed;
  +11 vs the 763 baseline from new net slot tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 14:57 UTC (HEAD `93036e7`, tree clean)

Covers the slot batch: master-flag map slot branch (`8a0346a`), 5001
prefix + restock pairs (`0af1899`), key-name filter role (`96a718f`),
and the slot-head log (`93036e7`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **791 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 115, game 68, net 315,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:57:41 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (791 passed;
  +17 vs the 774 baseline from new file/net slot tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 15:10 UTC (HEAD `6fbc945`, tree clean)

Covers the slot2 batch: lead-flag + wallet roles (`8ebe330`), inline-clear
table pins (`170296e`), and the slot2-head log (`6fbc945`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **803 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 117, game 68, net 325,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:10:04 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (803 passed;
  +12 vs the 791 baseline from new net slot2 + file table tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 15:18 UTC (HEAD `e923de2`, tree clean)

Covers the tail2 batch: step banner domains (`aa7edaa`), prices verify
5029 / rule out 5040 (`648c44e`), audit refresh (`e28e1f2`), and the
slot2b-head log (`e923de2`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **809 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 123, game 68, net 325,
  render 35, shell 176, real_art 7, scenes 22, recomp2jam 34
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:18:19 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (809 passed;
  +6 vs the 803 baseline from new file step-banner tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 15:37 UTC (HEAD `ee7bd9d`, tree clean)

Covers the shots head: testable builder extraction (`5dd84fa`), tables
split by section (`0952bcb`), art-first scenes + skin highway
(`f720e52`), genuine art staged in scene shots (`600a6db`), and the
slot2b-head log (`ee7bd9d`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **855 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 123, game 68, net 325,
  render 53, shell 184, real_art 7, scenes 22, recomp2jam 54
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.64 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:37:43 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (855 passed;
  +46 vs the 809 baseline from new render/shell/recomp2jam shots and
  builder tests, 0 failures). No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Integration sweep — post skins/room-enums (`890 passed`)

Gates run at integration on HEAD `5e626ea` (shots with staged
skins, shell skin delegate, room-enums, spare slots):

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`.
- `cargo test --workspace`: **890 passed, 0 failed, 1 ignored**
  (`o2jam_audio`), `exit=0`. Per target: audio 19, dump 0,
  file 123, game 68, net 333, render 53, shell 208, real_art 7,
  scenes 22, recomp2jam 57.
- Live flow not re-run in this sweep (last green `23:37:43 +08:00`
  run above stands; no net flow code changed since).

## Regression sweep — 2026-11-06 16:25 UTC (HEAD `7e03b63`, tree clean)

Covers the verbatim batch: native-scale compose (`4265a00`), verbatim
bnd rects (`3751327`), verbatim-derived highway (`5cb01a2`), audit
approximations (`19a0c2b`), and the playing-shot refresh (`7e03b63`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **903 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 123, game 68, net 333,
  render 59, shell 215, real_art 7, scenes 22, recomp2jam 57
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`00:25:13 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on clean HEAD (903 passed;
  +13 vs the 890 baseline from new render/shell verbatim tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 16:55 UTC (HEAD `4312687`, tree clean)

Covers the composer batch: data-driven opi binds/sprites (`7c00602`),
anchor-into-rect paint order (`c0ee463`), literal recipe compose
(`9788d11`), opi recipe-strings parity re-proof (`7556466`), and the
shots refresh (`4312687`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **947 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 136, game 68, net 333,
  render 65, shell 240, real_art 7, scenes 22, recomp2jam 57
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`00:55:37 +08:00`):
  `Authorize session` → `Get channel list` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room music` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `User music loaded` → `Game [000] Update` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broken-pipe broadcast race on room delete).
- **Verdict: no regression — suite is green on clean HEAD (947 passed;
  +44 vs the 903 baseline from new file/render/shell composer tests,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-06 22:27 UTC (HEAD `0e94222`, tracked tree clean)

Live re-proof on the audit-head. Covers the two shell feat commits landed
after the composer-sweep HEAD `4312687`: playing backdrop staging
(`93a8b23`) and avatar scene + tally strip (`a64eda7`), plus their docs
log (`0e94222`). Untracked peer notes present
(`docs/claims*.md`, `docs/ui-*.md`, stray `=433510`); no tracked file
modified, this sweep touched only `docs/playtest.md`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **972 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 136, game 68, net 333,
  render 65, shell 265, real_art 7, scenes 22, recomp2jam 57
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.63 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`06:26:49 +08:00`):
  `Create room: songflow` → `Update room music settings` →
  `Get music list: 562 music` → `Sync room member music state` →
  `Update room member [0: 100%] download progress` → `Start game` →
  `User music loaded` → `Game Update #1 - Jam: 3 (Bonus: 0)` →
  `Game Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast failure on room delete).
- **Verdict: no regression — suite is green on HEAD (972 passed;
  +25 vs the 947 baseline, all new shell tests from the two feat
  commits above, 0 failures). No commits, no pushes.**

## Regression sweep — 2026-11-06 22:42 UTC (HEAD `abc4807`, tree clean)

Live re-proof on the audit-fix head. Covers the three fix commits
`64addcd` (file: pair rects by code ordinal), `8ffe21f` (render:
additive anchor decoded colorkey viewport clip), and `29bcbdc`
(shell: align composer with exe draw chain), plus the claims/draw-chain
docs (`4a55283`) and the shots refresh (`abc4807`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **980 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 138, game 68, net 333,
  render 70, shell 266, real_art 7, scenes 22, recomp2jam 57
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`06:42:25 +08:00`):
  `Authorize session` → `Get channel list` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Sync room member [000] [0]
  music state` → `download progress` → `Start game: [000]` →
  `User music loaded` → `Submit score: 2210` (followed by the known
  benign broadcast-failure stack on room delete).
- **Verdict: no regression — suite is green on the audit-fix head
  (980 passed; +8 vs the 972 baseline: file +2, render +5,
  shell +1 from the fix commits' tests, 0 failures). No commits,
  no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-06 23:22 UTC (HEAD `8886d32`, tree clean at start)

Live re-proof on the life-head. Covers the life-batch commits over the
audit-fix baseline `abc4807`: ojm banks keyed by explicit id (`8d139ad`),
difficulty slice mapping proof (`7df654d`), curtain + hud box rederivation
(`d0dd70f`), life bar from attributed deltas (`8c48ca4`), audit2 re-proof
log (`72a4c4f`), ojm bank selection exe-verified doc (`268f2f6`), live
life-bar drain paint (`b32ea82`), and audit rows flipped to exe evidence
(`8886d32`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1004 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 141, game 79, net 333,
  render 72, shell 274, real_art 7, scenes 22, recomp2jam 57
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`07:21:59 +08:00`):
  `Authorize session` → `Get channel list` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings` → `Get music list: [0/00] 562 music`
  → `Sync room member [000] [0] music state` → `download progress` →
  `Start game: [000]` → `User music loaded` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broken-pipe broadcast WARNs on room delete).
- **Verdict: no regression — suite is green on the life-head (1004
  passed; +24 vs the 980 baseline: file +3, game +11, render +2,
  shell +8 from the life-bar/difficulty-slice tests, 0 failures).
  No commits, no pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits mid-sweep
  (`o2jam-file/src/control.rs`, `o2jam-file/src/sprites.rs`,
  `o2jam-render/src/lib.rs`, `o2jam-shell/src/avatar.rs`; 24 insertions,
  14 deletions, no behavior claims checked). The edits predate the first
  test compile, so every gate above ran on the dirty state; fmt, clippy,
  and the full suite were re-run after and stayed green with identical
  counts.
