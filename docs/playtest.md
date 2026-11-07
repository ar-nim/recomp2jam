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

## Regression sweep — 2026-11-07 08:48 UTC+8 (HEAD `2ed7b97`, tree dirty mid-sweep)

Live re-proof on the hold-head. Covers the net difficulty-icon pins
(`51a4759`), the shell holds-expiry fix (`c2702fc`), and the docs log
(`2ed7b97`) landed after the life-head baseline `8886d32`.

- `cargo fmt --check`: clean, `exit=0` (re-run on the dirty state:
  clean, `exit=0`).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched. A peer
  landed uncommitted edits mid-sweep (`o2jam-render/src/lib.rs`,
  `o2jam-shell/src/avatar.rs`, 08:45:24/08:45:37 +08:00); clippy was
  re-run fresh after them and re-checked the dirty `o2jam-shell` +
  `recomp2jam`, still clean.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1010 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`,
  identical on the clean start state and re-run on the dirty state.
  Per target: audio 19, dump 0, file 141, game 81, net 333,
  render 72, shell 278, real_art 7, scenes 22, recomp2jam 57
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check, re-run on the dirty state:
  same).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s, re-run on the dirty state) against
  local Memoryer 7.2.0 (`Mode=Full`). Server log for the re-run
  (`08:48:13 +08:00`): `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARN on room delete; a first run at
  `08:45:39 +08:00` logged the same flow).
- **Verdict: no regression — suite is green on the hold-head (1010
  passed; +6 vs the 1004 baseline: game +2, shell +4, all other
  targets unchanged, 0 failures). No commits, no pushes.** This sweep
  touched only `docs/playtest.md`; the two dirty files are the peer's
  uncommitted work, not mine.

## Regression sweep — 2026-11-07 11:07 UTC+8 (HEAD `8bee765`, tree clean)

Live re-proof on the hold-migration head. Covers the three commits landed
after the hold-head baseline `2ed7b97`: multi-entry proof skip without
archives (`e107697`), inert expiry arm release judging (`cc10667`), and
armed tails judged on release (`8bee765`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1047 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 149, game 84, net 333,
  render 72, shell 299, real_art 7, scenes 22, recomp2jam 62
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`11:06:40 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broken-pipe broadcast WARNs on room delete).
- **Verdict: no regression — suite is green on the hold-migration head
  (1047 passed; +37 vs the 1010 baseline: file +8, game +3, shell +21,
  recomp2jam +5 from the three commits above, all other targets
  unchanged, 0 failures). No commits, no pushes.** This sweep touched
  only `docs/playtest.md`.

## Regression sweep — 2026-11-07 14:54 UTC+8 (HEAD `e9fd95f`, tree clean)

Live re-proof on the parity-head. Covers the playing-screen parity commit
(`e9fd95f`) landed after the hold-migration baseline `8bee765`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1060 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, file 149, game 84, net 337, render 72,
  shell 305, real_art 7, scenes 22, recomp2jam 65
  (7 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`14:54:41 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the parity-head (1060
  passed; +13 vs the 1047 baseline: net +4, shell +6, recomp2jam +3,
  all other targets unchanged, 0 failures). No lint findings. No
  commits, no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 15:55 UTC+8 (HEAD `a1ead7f`, tree clean)

Live re-proof on the claims-head. Covers the two commits landed after the
parity-head baseline `e9fd95f`: om-band polarity/divergence claims docs
(`e06eaba`) and lane walk end-marker discipline proof (`00688fc`), plus
the docs log (`a1ead7f`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1062 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 149, game 86, net 337, render 72,
  shell 305, real_art 7, scenes 22, recomp2jam 65
  (7 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`15:55:12 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get character info`).
- **Verdict: no regression — suite is green on the claims-head (1062
  passed; +2 vs the 1060 baseline: game +2 from the lane walk
  end-marker tests, all other targets unchanged, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-07 16:29 UTC+8 (HEAD `cb4a956`, tree clean)

Live re-proof on the icons-head. Covers the scene-icons/hold-bits commit
(`cb4a956`: game hold-bit pins, shell icon paint in `avatar.rs` +
`screens/paint.rs`, shots refresh, claims/gameplay docs) landed after the
claims-head baseline `a1ead7f`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1080 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 149, game 91, net 337,
  render 72, shell 318, real_art 7, scenes 22, recomp2jam 65
  (7 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`16:28:41 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` → `Start game: [000]` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARNs on session close).
- **Verdict: no regression — suite is green on the icons-head (1080
  passed; +18 vs the 1062 baseline: game +5, shell +13, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 16:52 UTC+8 (HEAD `03657b3`, tree clean)

Live re-proof on the icons-head, re-taken one commit later: covers the
result numfont sweep-claims commit (`03657b3`) landed after the
icons-head baseline `cb4a956`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1086 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 149, game 91, net 337,
  render 72, shell 321, real_art 7, scenes 22, recomp2jam 68
  (7 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`16:52:12 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast WARN on room delete; a
  second session walked `Get channel list: [0]` → `Enter channel` →
  `Get character info`).
- **Verdict: no regression — suite is green on `03657b3` (1086
  passed; +6 vs the 1080 icons-head baseline: shell +3, recomp2jam
  +3 from the numfont sweep-claims tests, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 18:26 UTC+8 (HEAD `44831e1`, tree clean)

Live re-proof on the result-anchor head. Covers the result-anchors
commit (`44831e1`: game result-anchor life-residual + stage-refs
archive-lookup work) landed after the icons-head baseline `03657b3`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1111 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 149, game 92, net 337,
  render 72, shell 342, real_art 7, scenes 22, recomp2jam 71
  (7 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`18:25:45 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARNs on session close).
- **Verdict: no regression — suite is green on the result-anchor
  head (1111 passed; +25 vs the 1086 baseline: game +1, shell +21,
  recomp2jam +3, all other targets unchanged, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-07 19:07 UTC+8 (HEAD `a1a4599`, tree clean)

Live re-proof on the musicselect-head. Covers the music-select dialog
structural parity commit (`17e3a6f`) plus the claims-sweep and icons2
docs logs (`c98e101`, `a1a4599`) landed after the result-anchor baseline
`44831e1`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1125 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 149, game 92, net 337,
  render 72, shell 356, real_art 7, scenes 22, recomp2jam 71
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`19:07:30 +08:00`):
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the musicselect-head
  (1125 passed; +14 vs the 1111 baseline, all shell +14 from the
  music-select dialog tests, all other targets unchanged, 0 failures).
  No lint findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-07 19:18 UTC+8 (HEAD `691235a`, tree clean)

Live re-proof on the waiting-head. Covers the waiting scene region parity
commit (`df65e26`) plus the sweep log (`691235a`) landed after the
musicselect-head baseline `a1a4599`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1137 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 0, file 149, game 92, net 337, render 72,
  shell 368, real_art 7, scenes 22, recomp2jam 71
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`19:18:03 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` → `Start game: [000]` →
  `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete).
- **Verdict: no regression — suite is green on the waiting-head (1137
  passed; +12 vs the 1125 baseline, all shell +12 from the waiting
  region-parity tests, all other targets unchanged, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-07 19:26 UTC+8 (HEAD `15ff02e`, tree clean)

Live re-proof on the dump-head. Covers the two feat commits landed after
the waiting-head baseline `691235a`: dump pretty-print ojm and ojn
(`64c0acd`) and side-content close result items (`5677e2f`), plus the
sweep log (`15ff02e`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1156 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 380, real_art 7, scenes 22, recomp2jam 71
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check; mtime only, no content
  change).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`19:25:58 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` → `Update room music settings` →
  `Get music list: [0/00] 562 music` →
  `Sync room member music state` → `download progress` →
  `Start game: [000]` → `User music loaded` → `Game [000] Update` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the dump-head (1156
  passed; +19 vs the 1137 baseline: dump +7 from the new pretty-print
  test target, shell +12 from the side-content tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 19:32 UTC+8 (HEAD `c5a5015`, tree clean)

Live re-proof on the channel-head. Covers the fix commit landed after the
dump-head baseline `15ff02e`: stage nothing for spriteless channel
(`a9ade5c`), plus the sweep log (`c5a5015`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1157 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 381, real_art 7, scenes 22, recomp2jam 71
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`19:32:13 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the channel-head (1157
  passed; +1 vs the 1156 baseline, shell +1 from the spriteless-channel
  fix tests, all other targets unchanged, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-07 19:38 UTC+8 (HEAD `188bee3`, tree clean)

Live re-proof on the result-head. Covers only the docs log commit landed
after the channel-head baseline `c5a5015` (`188bee3`: result-screen
verify sweep-ten notes in `claims.md`/`cresult-anchors.md`/`opi-ignored.md`
plus the sweep log and a `waiting.png` refresh) — no code changed, so no
new tests were expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1157 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 381, real_art 7, scenes 22, recomp2jam 71
  (6 doc-test targets, 0 each) — identical to the baseline, as
  expected for a docs-only HEAD.
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check; mtime only, no content
  change).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`19:37:47 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `download progress` → `Start game: [000]` →
  `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the result-head (1157
  passed; identical to the channel-head baseline, docs-only delta,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits mid-sweep (`side.rs` +118:
  `WalletState` live wallet balances folded from 5029/5001 wire words
  for the payment/result displays; `gifts.md` notes). Re-ran on that
  dirty state: `cargo fmt --check` clean, `cargo clippy --workspace
  --all-targets -- -D warnings` clean, `cargo test --workspace`
  **1165 passed, 0 failed, 1 ignored** (shell 381 → 389 from the new
  wallet tests, all other targets unchanged), live flow re-run →
  **ok** (1 passed, 0.59 s). The peer then kept editing: the current
  dirty state (`paint.rs` saved 19:39:09, in-flight `paint_result_scene`
  signature change with callers not yet updated) does NOT compile —
  4× E0061 arg-count errors in `o2jam-shell` lib test — and carries
  peer fmt diffs. None of that is mine: per the sweep rules (touch
  only `docs/playtest.md`, fix only my own lint findings) I left the
  peer's files alone. Clean-HEAD verdict above stands; the dirty files
  are the peer's uncommitted work, not mine.

## Regression sweep — 2026-11-07 19:45 UTC+8 (HEAD `d212d50`, tree dirty)

Live re-proof on the wallet-head. Covers the feat commit landed after the
result-head baseline `188bee3`: rank rows wire wallet display (`04e2462`:
`WalletState` live gem/cash balances folded from 5029/5001 wire words for
the payment/result displays), plus the docs log (`d212d50`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1184 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 408, real_art 7, scenes 22, recomp2jam 71
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`19:45:04 +08:00`,
  session `127.0.0.1:42462`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green (1184 passed; +27 vs the
  1157 baseline, all shell 381 → 408, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits before the first gate
  (`crates/o2jam-shell/src/screens/side.rs` +42/-1: new
  `WalletState::apply_sell_response` folding the 5003 success gem word
  into the shared gem slot plus 2 tests; `docs/shop-buy.md` +4/-1:
  matching sell-gem notes), so every gate above ran on the dirty
  state. Of the +27 shell tests, 25 come from the committed wallet
  work and 2 (`wallet_applies_sell_success_gem_word`,
  `wallet_keeps_gem_on_sell_error`) are the peer's uncommitted tests —
  both observed passing, none of it mine. Per the sweep rules (touch
  only `docs/playtest.md`, fix only my own lint findings) the peer's
  files were left alone.

## Regression sweep — 2026-11-07 19:52 UTC+8 (HEAD `b4e29a6`, tree clean)

Live re-proof on the wallet-head. Covers the feat commit landed after the
result-head baseline `188bee3`: fold sell refund show free pass
(`2912a65`: `WalletState` sell-refund gem fold + free-pass show flag in
`side.rs`, `gifts.md`/`shop-buy.md` notes), plus the docs log (`b4e29a6`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1191 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 415, real_art 7, scenes 22, recomp2jam 71
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`19:50:17 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the wallet-head (1191
  passed; +7 vs the 1184 baseline, all shell 408 → 415 from the
  sell-refund/show-free-pass tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 20:00 UTC+8 (HEAD `5b9808c`, tree clean)

Live re-proof on the verify-head. Covers the docs-log commit landed after
the wallet-head baseline `b4e29a6`: shots re-proof scoring the result
claims sweep (`5b9808c`: new result-shot pins in `shots.rs`, claims/parity
notes, `result.png` refresh) on top of the sell-refund feat (`2912a65`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1192 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 415, real_art 7, scenes 22, recomp2jam 72
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`20:00:38 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `download progress` → `Start game: [000]` →
  `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the verify-head (1192
  passed; +1 vs the 1191 baseline, recomp2jam 71 → 72 from the new
  result-shot pins, all other targets unchanged, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-07 20:24 UTC+8 (HEAD `46f0a7d`, tree clean at start)

Live re-proof on the verdict-head. Covers the two feat commits landed
after the verify-head baseline `5b9808c`: music-select shot reverify
(`08cc4ca`: `musicselect.png` staged shot + claims/parity re-proof) and
dossier verdict frames (`46f0a7d`: battle walk shows-lose frames in
`paint.rs`/`shots.rs`, `result-variants.md` dossier, shot refreshes).

- `cargo fmt --check`: clean, `exit=0` (on clean HEAD).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1199 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 417, real_art 7, scenes 22, recomp2jam 77
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`20:24:13 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the verdict-head (1199
  passed; +7 vs the 1192 baseline: shell 415 → 417, recomp2jam
  72 → 77 from the verdict-frames/musicselect tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits at 20:24:19 +08:00,
  after every clean-HEAD gate above had finished
  (`crates/recomp2jam/src/shots.rs` +26/-4: native-canvas
  `playing-native`/`result-native` companions,
  `Vec::with_capacity(10)`). Re-ran on that dirty state:
  `cargo fmt --check` FAILS on one of the peer's lines (long
  `shoot_checked_native` call, needs rewrapping); `cargo clippy
  --workspace --all-targets -- -D warnings` stays clean;
  `cargo test -p recomp2jam` → **74 passed, 3 failed**
  (`walk_covers_eight_distinct_scenes`,
  `writer_saves_seven_scenes_and_keeps_avi`,
  `staging_attempt_never_breaks_the_walk` — expectations still pin
  the old 8-scene/7-file walk, not yet updated for the 2 native
  companions). None of that is mine: per the sweep rules (touch
  only `docs/playtest.md`, fix only my own lint findings) the peer's
  file was left alone. Clean-HEAD verdict above stands; the dirty
  file is the peer's uncommitted work, not mine.

## Regression sweep — 2026-11-07 20:37 UTC+8 (HEAD `0975314`, tree clean)

Live re-proof on the native-head. Covers the feat commit landed after the
verdict-head baseline `46f0a7d`: native verification shots with scripted
rank rows (`3b604ae`: native-canvas `playing-native`/`result-native`
companions sharing the walked source frame, scripted rank fixture,
`playing.rs`/`game.rs` rank rows, shot refreshes), plus the sweep log
(`0975314`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1204 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 419, real_art 7, scenes 22, recomp2jam 80
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`20:37:03 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARNs on session close).
- **Verdict: no regression — suite is green on the native-head (1204
  passed; +5 vs the 1199 baseline: shell 417 → 419, recomp2jam
  77 → 80 from the native-shots/scripted-rank tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits after every gate above
  had finished on clean HEAD (`o2jam-shell/src/screens/paint.rs`:
  `channel_row_text` helper folding the gateway id into the channel
  row; `o2jam-shell/src/screens/game.rs` +14). Every count above is
  the clean-HEAD result. Per the sweep rules (touch only
  `docs/playtest.md`, fix only my own lint findings) the peer's
  files were left alone.

## Regression sweep — 2026-11-07 20:43 UTC+8 (HEAD `ff4faf6`, tree clean)

Live re-proof on the channel-head. Covers the fix commit landed after the
native-head baseline `0975314`: name gateway in channel rows (`4284c44`:
`channel_row_text` helper folding the gateway id into the channel row),
plus the docs log (`ff4faf6`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1208 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 337,
  render 72, shell 423, real_art 7, scenes 22, recomp2jam 80
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`20:42:57 +08:00`):
  `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete).
- **Verdict: no regression — suite is green on the channel-head (1208
  passed; +4 vs the 1204 baseline, all shell 419 → 423 from the
  gateway channel-row tests, all other targets unchanged, 0 failures).
  No lint findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits mid-sweep
  (`crates/o2jam-shell/src/avatar.rs` +4, saved 20:42:53 +08:00;
  `crates/o2jam-shell/src/screens/paint.rs` +145/-20ish, saved
  20:43:06 +08:00). The full-suite/fmt/clippy/build gates above all ran
  before either edit, so every count above is the clean-HEAD result.
  The live flow (server-logged 20:42:57–58 +08:00) compiled after the
  `avatar.rs` save, so it ran with that edit present — still **ok**;
  the edit is a doc-comment plus a `LOADING_BLOCK` const, no song-flow
  behavior change. The `paint.rs` edit landed after every gate had
  finished; nothing was re-run on it. Per the sweep rules (touch only
  `docs/playtest.md`, fix only my own lint findings) the peer's files
  were left alone.

## Regression sweep — 2026-11-07 20:58 UTC+8 (HEAD `1619301`, tree clean)

Live re-proof on the ranks-head. Covers the two feat commits landed after
the channel-head baseline `ff4faf6`: loading gate art-fraction captions
(`b9be8ea`: `LOADING_BLOCK` const + caption paint in `avatar.rs`/`paint.rs`)
and live 4015 rank bytes (`bec299a`: `play.rs` rank-byte pins + `relay.md`
notes), plus the docs log (`1619301`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1213 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 338,
  render 72, shell 427, real_art 7, scenes 22, recomp2jam 80
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`20:58:04 +08:00`):
  `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `download progress` → `Start game: [000]` →
  `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Get channel list: [0]` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the ranks-head (1213
  passed; +5 vs the 1208 baseline: net 337 → 338 from the 4015
  rank-byte tests, shell 423 → 427 from the loading-gate caption
  tests, all other targets unchanged, 0 failures). No lint findings.
  No commits, no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 21:07 UTC+8 (HEAD `91bcff1`, tracked tree clean)

Live re-proof on the solo-head. Covers the feat commit landed after the
ranks-head baseline `1619301`: solo result walk live loading progress
(`91bcff1`), plus the ranks sweep log (`1619301`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1231 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 338,
  render 72, shell 433, real_art 7, scenes 22, recomp2jam 92
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:07:08 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the solo-head (1231
  passed; +18 vs the 1213 baseline: shell 427 → 433, recomp2jam
  80 → 92 from the solo result-walk/loading-progress tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: one untracked scratch file present (`check_solo.py`, not
  mine, left alone); no tracked file modified.

## Regression sweep — 2026-11-07 21:14 UTC+8 (HEAD `fb31866`, tree clean)

Live re-proof on the fill-head. Covers the channel-fill commit landed
after the solo-head baseline `91bcff1`: channel fill stays open
(`paint.rs`: spriteless channel keeps the documented placeholder fill
instead of staging no scene art; renamed test +
`empty_channel_art_keeps_the_documented_placeholder_fill`), plus
`docs/channel-fill.md` and the solo sweep log (`fb31866`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1232 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 338,
  render 72, shell 434, real_art 7, scenes 22, recomp2jam 92
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:13:40 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Sync room member [000] [0] music state` → `download progress` →
  `Start game: [000]` → `User music loaded` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the fill-head (1232
  passed; +1 vs the 1231 solo-head baseline, shell 433 → 434 from
  the new placeholder-fill test, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 21:26 UTC+8 (HEAD `c49f655`, tree clean)

Live re-proof on the nofill-head. Covers the fix commit landed after the
fill-head baseline `fb31866`: paint nothing for art-less states
(`paint.rs`: art-less states paint nothing instead of the documented
placeholder fill; `docs/channel-fill.md` reworked), plus the waiting
shots log (`0d5c402`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1234 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 338,
  render 72, shell 436, real_art 7, scenes 22, recomp2jam 92
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:25:46 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the nofill-head (1234
  passed; +2 vs the 1232 fill-head baseline, all shell 434 → 436
  from the art-less paint tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 21:32 UTC+8 (HEAD `364f3e3`, tree clean)

Live re-proof on the preserve-head. Covers the two commits landed after the
nofill-head baseline `c49f655`: preserve stale frame letterbox bars
(`ce69eb2`: render keeps the stale letterbox bars on resize, shell
`window.rs` pins, `docs/channel-fill.md` rework) and the empty channel
scene guard shot (`58cfe20`: cli guard shot for the empty channel scene,
`channel.png` refresh, `docs/parity.md` notes), plus the docs log
(`364f3e3`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1237 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 338,
  render 73, shell 437, real_art 7, scenes 22, recomp2jam 93
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:31:52 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the preserve-head (1237
  passed; +3 vs the 1234 nofill-head baseline: render 72 → 73 from the
  stale-frame letterbox tests, shell 436 → 437 and recomp2jam 92 → 93
  from the empty-channel guard-shot tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 21:40 UTC+8 (HEAD `a90b5c1`, tree clean)

Live re-proof on the fill-head. Covers the docs-log commit landed after the
preserve-head baseline `364f3e3`: loading geometry open sprite-less trace
re-proof (`a90b5c1`: `paint.rs` loading-geometry trace + `channel-fill.md`
notes), on top of the stale-letterbox (`ce69eb2`) and empty-channel
guard-shot (`58cfe20`) work.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1238 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 338,
  render 73, shell 438, real_art 7, scenes 22, recomp2jam 93
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:39:31 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete).
- **Verdict: no regression — suite is green on the fill-head (1238
  passed; +1 vs the 1237 preserve-head baseline, shell 437 → 438
  from the loading-geometry trace tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.
