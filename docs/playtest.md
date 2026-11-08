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

## Regression sweep — 2026-11-07 21:46 UTC+8 (HEAD `9e12282`, tree clean)

Live re-proof on the negatives-head. Covers the waiting-negatives commit
landed after the fill-head baseline `a90b5c1`: recorded chat/cards
negatives in `paint_waiting` docs (`game.rs`: `EDIT_SWAIT_CHAT` /
`LIST_SWAIT_CHAT` / `TEXT_SWAIT_CHAT` rects and `Random` `?` cards stay
staged art with no text source), paint-side absence pins (`paint.rs`),
and the `channel-fill.md` rework, plus the sweep log (`9e12282`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1240 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 338,
  render 73, shell 440, real_art 7, scenes 22, recomp2jam 93
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:45:53 +08:00`):
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
  broadcast-failure stack on room delete).
- **Verdict: no regression — suite is green on the negatives-head (1240
  passed; +2 vs the 1238 fill-head baseline, all shell 438 → 440
  from the chat/cards absence tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 21:58 UTC+8 (HEAD `08911d5`, tree clean at start)

Live re-proof on the chat-head. Covers the net feat commit landed after
the negatives-head baseline `9e12282`: decode waiting chat packets
(`b94db55`), plus the docs log (`08911d5`).

- `cargo fmt --check`: clean, `exit=0` (on clean HEAD at sweep start).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1248 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 440, real_art 7, scenes 22, recomp2jam 93
  (6 doc-test targets, 0 each). Identical counts across 3 runs.
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`21:58:55 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the chat-head (1248
  passed; +8 vs the 1240 negatives-head baseline, all net 338 → 346
  from the waiting-chat packet decode tests, all other targets
  unchanged, 0 failures). No lint findings of mine. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted in-flight work mid/late-sweep
  in `crates/o2jam-shell/src/screens/paint.rs` (grew from +13 to +294
  lines between checks: new `paint_waiting_chat` /
  `waiting_chat_visible_messages` chat-paint code + tests; HEAD carries
  none of it). Every count above matches clean HEAD. Re-run on the
  dirty state: `cargo clippy --workspace --all-targets -- -D warnings`
  stays clean, but `cargo fmt --check` FAILS on one of the peer's new
  lines (long `waiting_chat_visible_messages` signature, needs
  rewrapping). None of that is mine: per the sweep rules (touch only
  `docs/playtest.md`, fix only my own lint findings) the peer's file
  was left alone, and the peer was still editing when this log was
  written, so no suite re-run on the moving dirty state was taken.

## Regression sweep — 2026-11-07 22:08 UTC+8 (HEAD `27d67a0`, tree clean at start, dirtied mid-sweep)

Live re-proof on the chat-head, one feat later: covers the shell commit
landed after the chat-head baseline `08911d5` (wire live waiting chat
into the waiting scene, `31f14fb`), plus the sweep log (`27d67a0`).

- `cargo fmt --check`: clean, `exit=0` (on clean HEAD at sweep start).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1263 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 455, real_art 7, scenes 22, recomp2jam 93
  (6 doc-test targets, 0 each). Ran twice, identical counts.
- `cargo build --workspace --release`: clean, `exit=0` (plus a
  touched-`main.rs` rebuild check: 0 warning lines).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **NOT RUN
  GREEN — blocked, no pass claimed.** A peer saved uncommitted
  in-flight work in `crates/recomp2jam/src/shots.rs` (22:07 +08:00,
after every clean-HEAD gate above had finished: scripted 3012/3013
  chat fixture via `stage_waiting_chat`, +41/-6) that does not
  compile: `error[E0432]: unresolved imports
  o2jam_net::WaitingAdminMessage, o2jam_net::WaitingUserMessage`
  (`bin "recomp2jam" test` target; HEAD exports neither name at the
  `o2jam_net` root). The suite gates above all compiled and passed
  before that save, so every count above is the clean-HEAD result.
- **Verdict: no regression on clean HEAD (1263 passed; +15 vs the
  1248 chat-head baseline, all shell 440 → 455 from the wire-live-chat
  tests, all other targets unchanged, 0 failures) — but the live flow
  has no green run on this HEAD: the dirty tree does not compile, so
  no live pass is recorded.** No lint findings of mine. No commits, no
  pushes. This sweep touched only `docs/playtest.md`.
- Tree note: on the dirty state `cargo fmt --check` also shows a diff
  on one of the peer's new lines (the long two-name `use o2jam_net`
  import, needs rewrapping). Untracked peer scratch files present
  (`.tmp-*.py`, left alone). None of that is mine: per the sweep rules
  (touch only `docs/playtest.md`, fix only my own lint findings) the
  peer's files were left alone, and the peer appears to still be
  editing, so no suite re-run on the moving dirty state was taken.

## Regression sweep — 2026-11-07 22:16 UTC+8 (HEAD `035990f`, tree clean)

Live re-proof on the chat-rows head. Covers the feat commit landed after
the chat-head baseline `27d67a0`: script chat rows close row feed
(`035990f`: scripted 3012/3013 waiting-chat fixture staged into the
waiting walk, `waiting.png` refresh, `channel-fill.md`/`parity.md` notes),
plus the sweep log in the same commit.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1266 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 455, real_art 7, scenes 22, recomp2jam 96
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:15:54 +08:00`):
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
  broadcast-failure WARNs on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the chat-rows head (1266
  passed; +3 vs the 1263 chat-head baseline, all recomp2jam 93 → 96
  from the scripted chat-rows shot/walk tests, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 22:23 UTC+8 (HEAD `2bc5c1a`, tree clean)

Live re-proof on the setter-head. Covers only the docs-log commit landed
after the chat-rows baseline `035990f` (`2bc5c1a`: join setter chain
notes in `channel-fill.md` plus the sweep log) — no code changed, so no
new tests were expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1266 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 455, real_art 7, scenes 22, recomp2jam 96
  (6 doc-test targets, 0 each) — identical to the baseline, as
  expected for a docs-only HEAD.
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:23:07 +08:00`):
  `Authorize session` → `Get channel list` →
  `Enter channel` → `Get character info: [tester]` →
  `Create room: songflow` → `Get music list` →
  `Sync room member music state` → `download progress` →
  `Start game: [000]` → `User music loaded` →
  `Game [000] Update #1` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the setter-head (1266
  passed; identical to the chat-rows baseline, docs-only delta,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.
- Tree note: a peer saved uncommitted edits in `docs/channel-fill.md`
  mid-sweep (after the clean-tree check at sweep start). Every gate
  above ran before that save, so every count above is the clean-HEAD
  result. Per the sweep rules (touch only `docs/playtest.md`, fix only
  my own lint findings) the peer's file was left alone.

## Regression sweep — 2026-11-07 22:27 UTC+8 (HEAD `a3473fc`, tree clean)

Live re-proof on the strings-head. Covers only the docs-log commit landed
after the setter-head baseline `2bc5c1a` (`a3473fc`: string-flows notes in
`channel-fill.md` plus the sweep log) — no code changed, so no new tests
were expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1266 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 455, real_art 7, scenes 22, recomp2jam 96
  (6 doc-test targets, 0 each) — identical to the baseline, as
  expected for a docs-only HEAD.
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check; mtime only, no content
  change).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:27:14 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the strings-head (1266
  passed; identical to the setter-head baseline, docs-only delta,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 22:43 UTC+8 (HEAD `2c00969`, tree clean)

Live re-proof on the captions-head. Covers the feat commit landed after the
strings-head baseline `a3473fc`: paint live text captions (`6776b31`:
waiting musicname/music caption paint in `game.rs`/`paint.rs` resolving the
catalog title with song-id fallback), plus the docs log (`2c00969`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1271 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 460, real_art 7, scenes 22, recomp2jam 96
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:42:48 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the captions-head (1271
  passed; +5 vs the 1266 strings-head baseline, all shell 455 → 460
  from the live music-caption/musicname tests, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 22:52 UTC+8 (HEAD `434c002`, tree clean)

Live re-proof on the strings2-head. Covers only the docs-log commit
landed after the captions-head baseline `2c00969` (`434c002`: caption
target/source pins in `caption-sources.md`/`channel-fill.md` plus the
sweep log) — no code changed, so no new tests were expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1271 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 460, real_art 7, scenes 22, recomp2jam 96
  (6 doc-test targets, 0 each) — identical to the baseline, as
  expected for a docs-only HEAD.
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check; mtime only, no content
  change).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`22:52:48 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the strings2-head (1271
  passed; identical to the captions-head baseline, docs-only delta,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 23:03 UTC+8 (HEAD `9caee7a`, tree clean)

Live re-proof on the captions-head. Covers the feat commit landed after the
strings2-head baseline `434c002`: paint mapname/bpm/skillcount captions
(`c3c88f2`: `game.rs`/`paint.rs` caption paint), plus the docs log
(`9caee7a`: resolve-song-word notes in `caption-sources.md`/`stage-bga.md`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1283 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 96
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:03:00 +08:00`):
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
- **Verdict: no regression — suite is green on the captions-head (1283
  passed; +12 vs the 1271 strings2-head baseline, all shell 460 → 472
  from the mapname/bpm/skillcount caption tests, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 23:12 UTC+8 (HEAD `4a2e306`, tree clean)

Live re-proof on the captions2-head. Covers the feat commit landed after the
captions-head baseline `9caee7a`: verify waiting captions settle stage
override (`4a2e306`: `stage_waiting_captions` helper + `staged_catalog_bpm`
fixture in `shots.rs`, waiting-shot caption verify, `waiting.png` refresh,
`parity.md`/`stage-bga.md` notes).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1284 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 97
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:11:32 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the captions2-head (1284
  passed; +1 vs the 1283 captions-head baseline, recomp2jam 96 → 97
  from the waiting-caption settle verify test (`shots::tests::
  waiting_shot_paints_mapname_bpm_skillcount_captions`, new in this
  HEAD), all other targets unchanged, 0 failures). No lint findings.
  No commits, no pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer saved uncommitted docs edits mid-sweep
  (`docs/stage-bga.md`, saved 23:11:31 +08:00 — stage/avatar z-order
  CLOSED verdict, docs-only, no code impact). The tree was clean at
  sweep start; every gate above ran against HEAD code unaffected by a
  markdown edit, and `cargo fmt --check` was re-run clean after both
  files settled. Per the sweep rules (touch only `docs/playtest.md`,
  fix only my own lint findings) the peer's file was left alone.

## Regression sweep — 2026-11-07 23:18 UTC+8 (HEAD `4bd3d3a`, tree clean)

Live re-proof on the zorder-head. Covers only the docs-log commit landed
after the captions2-head baseline `4a2e306` (`4bd3d3a`: z-order twin
appliers pins in `stage-bga.md`/`stage-twins.md` plus the sweep log) — no
code changed, so no new tests were expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1284 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 346,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 97
  (6 doc-test targets, 0 each) — identical to the baseline, as
  expected for a docs-only HEAD.
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:17:54 +08:00`):
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
  broadcast-failure WARNs on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the zorder-head (1284
  passed; identical to the captions2-head baseline, docs-only delta,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.
- Tree note: nine untracked scratch files present (`tmp-scan1.py`
  through `tmp-scan9.py`, not mine, left alone); no tracked file
  modified.

## Regression sweep — 2026-11-07 23:25 UTC+8 (HEAD `98ca756`, tree clean)

Live re-proof on the arena-head. Covers the net feat commit landed after
the zorder-head baseline `4bd3d3a`: decode room arena plus request id
(`0cbc27b`), plus the docs log (`98ca756`: ani high-word notes).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1290 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 352,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 97
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines on a touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:24:58 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the arena-head (1290
  passed; +6 vs the 1284 zorder-head baseline, all net 346 → 352
  from the room-arena decode tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 23:46 UTC+8 (HEAD `d559acd`, tree clean)

Live re-proof on the echo-head. Covers the net feat commits landed after
the arena-head baseline `98ca756`: decode arena echo fix live normal
labels (`9a75951`: `play.rs` arena-echo decode + live label pins,
`net-protocol.md` notes) and route arena echo event id (`d559acd`:
`command.rs` echo dispatch), plus the twin dispatch docs log (`9686bce`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1298 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 360,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 97
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:45:47 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARN on session close; a second session walked
  `Get channel list` → `Enter channel` → `Get room list`).
- **Verdict: no regression — suite is green on the echo-head (1298
  passed; +8 vs the 1290 arena-head baseline, all net 352 → 360
  from the arena-echo decode tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-07 23:52 UTC+8 (HEAD `a0bceec`, tree clean at start, dirtied mid-sweep)

Live re-proof on the twins-head. Covers only the docs-log commit landed
after the echo-head baseline `d559acd` (`a0bceec`: twin divergence close
notes in `stage-twins.md` plus the sweep log) — no code changed, so no
new tests were expected.

- `cargo fmt --check`: clean, `exit=0` (on clean HEAD at sweep start;
  re-run clean on the dirty state after).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1298 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 360,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 97
  (6 doc-test targets, 0 each) — identical to the baseline, as
  expected for a docs-only HEAD. Ran twice, identical counts.
- `cargo build --workspace --release`: `exit=0` with 1 pre-existing
  warning (not mine, left alone per the sweep rules):
  `field arena_echo is never read` (`connect.rs:214`, `SongFlowReport`;
  read only under test cfg, from the arena-echo commits `9a75951` +
  `d559acd`). Surfaces only on a forced (`touch main.rs`) rebuild;
  cached rebuilds print no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **NOT RUN
  GREEN — blocked, no pass claimed.** A peer saved uncommitted
  in-flight work (`connect.rs` +150ish, `main.rs` +20, saved
  23:51:35/40 +08:00, after every clean-HEAD gate above had finished:
  `--arena-id` CLI plumbing, `SongFlowOptions::arena_id`, report-side
  `song.arena_echo.arena` line) that does not compile: 2× `error[E0063]:
  missing field arena_echo in initializer of SongFlowReport`
  (`main.rs:1232` + `main.rs:1313`, `bin "recomp2jam" test` target).
  The suite/fmt/clippy/build gates above all compiled and passed
  before those saves (1298 counts impossible on the dirty state, which
  fails to build the test target), so every count above is the
  clean-HEAD result.
- **Verdict: no regression on clean HEAD (1298 passed; identical to
  the echo-head baseline, docs-only delta, 0 failures) — but the live
  flow has no green run on this HEAD: the dirty tree does not compile,
  so no live pass is recorded.** No lint findings of mine. No commits,
  no pushes. This sweep touched only `docs/playtest.md`.
- Tree note: on the dirty state `cargo fmt --check` stays clean
  (the peer's new lines are fmt-clean); no suite/clippy re-run on the
  moving dirty state was taken (the peer appears to still be editing).
  Per the sweep rules (touch only `docs/playtest.md`, fix only my own
  lint findings) the peer's files were left alone.

## Regression sweep — 2026-11-07 23:57 UTC+8 (HEAD `e277f9a`, tree clean)

Live re-proof on the arena-head. Covers the cli feat commit landed after the
twins-head baseline `a0bceec`: drive arena stage through live song flow
(`e277f9a`: `--arena-id` CLI plumbing, `SongFlowOptions::arena_id`,
report-side `song.arena_echo.arena` line in `connect.rs`/`main.rs`), plus
the committed twins-head sweep log.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1300 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 360,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 99
  (6 doc-test targets, 0 each). Ran twice, identical counts.
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check). The twins-head note about
  `field arena_echo is never read` (`connect.rs:214`) is gone: the
  arena-stage drive now reads the field, so the forced rebuild is
  warning-free.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`23:57:01 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1]` (new — the arena-stage drive) →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARN on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green on the arena-head (1300
  passed; +2 vs the 1298 twins-head baseline, all recomp2jam 97 → 99
  from the new arena-stage flow tests
  (`arena_stage_sends_word_and_asserts_echo`,
  `arena_echo_mismatch_fails_the_flow`), all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 00:18 UTC+8 (HEAD `9b42aaf`, tree clean)

Live re-proof on the venue-head. Covers the single commit landed after the
arena-head baseline `e277f9a`: solo composition stage-two venue pin
(`9b42aaf`: `shots.rs` playing-shot builder hardening, staged screenshots,
`stage-bga.md` notes).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1301 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 360,
  render 73, shell 472, real_art 7, scenes 22, recomp2jam 100
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`00:18:43 +08:00`):
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARNs on session close; a second session walked
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get room list: [0/00]` →
  `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the venue-head (1301
  passed; +1 vs the 1300 arena-head baseline, all recomp2jam 99 → 100
  from the new shots test
  `playing_shot_pins_solo_composition_despite_live_room`, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 00:57 UTC+8 (HEAD `939b483`, tree clean)

Live re-proof on the burst-head. Covers the two commits landed after the
venue-head baseline `9b42aaf`: burst art arm held beam bound in shots
(`e26c6e5`) and the venue sweep docs log (`939b483`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1306 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 149, game 92, net 360,
  render 73, shell 475, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (shell + recomp2jam recompiled, rest cached).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`00:54:48 +08:00`):
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete; a second session walked
  `Get character info` → `Get room list`).
- **Verdict: no regression — suite is green on the burst-head (1306
  passed; +5 vs the 1301 venue-head baseline: shell 472 → 475,
  recomp2jam 100 → 102 from the burst-art shots tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 02:45 UTC+8 (HEAD `d203614`, tree clean)

Live re-proof on the codec-head. Covers the three feat commits landed after
the burst-head baseline `939b483`: decode jpeg and route prefixed oji bodies
(`d203614`), modern ttf text with korean glyphs (`547f841`), and wire
strided combo glow playback (`dc9e947`), plus the two docs logs (`70f7871`,
`31f68be`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1346 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 159, game 92, net 360,
  render 89, shell 489, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check; mtime only, no content
  change; o2jam-file/render + zune-jpeg recompiled).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`02:42:33 +08:00`, session
  `127.0.0.1:51402`): `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broken-pipe broadcast WARNs on room delete; a second session
  `127.0.0.1:51412` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the codec-head (1346
  passed; +40 vs the 1306 burst-head baseline: file 149 → 159,
  render 73 → 89, shell 475 → 489 from the jpeg-decode/ttf-text/
  combo-glow tests, all other targets unchanged, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-08 06:24 UTC+8 (HEAD `887de12`, tree clean)

Live re-proof on the oja-head. Covers the two feat commits landed after the
codec-head baseline `d203614`: result album art oji stragglers fmt
(`6a992b4`: `jpeg.rs`/`ojn.rs`/`sprites.rs` decode, `avatar.rs`/`game.rs`/
`paint.rs` album-art staging, shot refreshes) and decode oja55 frames close
codec coverage (`887de12`: `sprites.rs` +623 oja55 frame decode, `avatar.rs`
straggler wiring, `codec-coverage.md` notes).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1371 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 174, game 92, net 360,
  render 89, shell 499, real_art 7, scenes 22, recomp2jam 102
  (7 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`06:24:08 +08:00`, session
  `127.0.0.1:38204`): `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARNs on session close; a second session
  `127.0.0.1:38218` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the oja-head (1371
  passed; +25 vs the 1346 codec-head baseline: file 159 → 174,
  shell 489 → 499 from the oji-straggler/oja55-frame tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 07:23 UTC+8 (HEAD `69d3bf1`, tree clean)

Live re-proof on the chatrows-head. Covers the two feat commits landed
after the oja-head baseline `887de12`: stage oja effects tap jam holds
(`e72d7ae`) and decode jam oja tail frames (`69d3bf1`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1382 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 177, game 92, net 360,
  render 89, shell 507, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check; mtime only, no content
  change; tree still clean after).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s; run twice, both ok) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for the second run
  (`07:22:49 +08:00`, session `127.0.0.1:33844`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast WARNs on room delete; a
  second session `127.0.0.1:33846` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the chatrows-head
  (1382 passed; +11 vs the 1371 oja-head baseline: shell 499 → 507
  from the oja-effects-hold tests (`e72d7ae`), file 174 → 177 from
  the jam-oja-tail tests (`69d3bf1`), all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 07:48 UTC+8 (HEAD `756ae59`, tree clean)

Live re-proof on the chatfmt-head. Covers the two commits landed after the
chatrows-head baseline `69d3bf1`: match chat box reference format
(`613eccc`) and the twin-chain-pin docs refutation (`756ae59`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1384 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 177, game 92, net 360,
  render 89, shell 509, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check, `Compiling recomp2jam`
  confirmed; mtime only, no content change; tree clean after).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`07:47:23 +08:00`, session
  `127.0.0.1:46002`): `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast WARNs on room delete).
- **Verdict: no regression — suite is green on the chatfmt-head (1384
  passed; +2 vs the 1382 chatrows-head baseline: shell 507 → 509
  from the chat-box-format tests (`613eccc`), all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 08:17 UTC+8 (HEAD `7654952`, tree clean)

Live re-proof on the chatchain-head. Covers the docs log commit landed
after the chatfmt-head baseline `756ae59`: close live chat writer chain
(`7654952`, docs-only).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1384 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 177, game 92, net 360,
  render 89, shell 509, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check, `Compiling recomp2jam`
  confirmed; mtime only, no content change; tree still clean after).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`08:17:30 +08:00`, session
  `127.0.0.1:56288`): `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast WARNs on room delete; a
  second session `127.0.0.1:56304` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the chatchain-head
  (1384 passed, identical to the chatfmt-head baseline; the head is
  docs-only, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 08:36 UTC+8 (HEAD `1a6ea79`, tree clean)

Live re-proof on the chatchain-head, re-taken one docs commit later:
covers the chatchain re-proof log (`1a6ea79`) landed after the
`7654952` baseline.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1384 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 177, game 92, net 360,
  render 89, shell 509, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`08:36:38 +08:00`, session
  `127.0.0.1:36632`): `Authorize session` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast WARNs on room delete; a
  second session `127.0.0.1:36648` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the chatchain
  docs-head (1384 passed, identical to the `7654952` chatchain-head
  baseline; the delta commit is docs-only, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-08 08:50 UTC+8 (HEAD `1480651`, tree clean)

Live re-proof on the endmarker-head. Covers the two commits landed after
the chatchain docs-head baseline `1a6ea79`: lane end-marker discipline
pins (`8891811`) and the stale-claims audit docs log (`1480651`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1386 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 177, game 94, net 360,
  render 89, shell 509, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (3 crates recompiled).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.60 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`08:50:27 +08:00`, session
  `127.0.0.1:55930`): `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast WARNs on room delete; a
  second session `127.0.0.1:55932` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the endmarker-head
  (1386 passed; +2 vs the 1384 chatchain docs-head baseline: game
  92 → 94 from the lane end-marker tests (`8891811`), all other
  targets unchanged, 0 failures). No lint findings. No commits,
  no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 08:58 UTC+8 (HEAD `1efaf71`, tree clean)

Live re-proof on the consistency-head. Covers the docs-only commit
`1efaf71` (claims table consistency pass log re-proof) landed after the
endmarker-head baseline `1480651`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1386 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0`.
  Per target: audio 19, dump 7, file 177, game 94, net 360,
  render 89, shell 509, real_art 7, scenes 22, recomp2jam 102
  (6 doc-test targets, 0 each) — identical to the `1480651` baseline.
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`08:57:16 +08:00`, session
  `127.0.0.1:43034`): `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  `ScoreTracker::OnGameCompleted` broadcast WARNs on room delete; no
  `Finalize rank`/`Leave game` flushed before session close; a second
  session `127.0.0.1:43038` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the consistency-head
  (1386 passed, identical to the `1480651` endmarker-head baseline;
  the delta commit is docs-only, 0 failures). No lint findings. No
  commits, no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 09:57 UTC+8 (HEAD `1efaf71`, tree dirty — peer editing throughout)

Live re-proof on the consistency-head, re-taken: HEAD is unchanged
(`1efaf71`, docs-only over the endmarker baseline `1480651`), but a
peer kept uncommitted in-flight work landing all through this sweep
(`crates/recomp2jam/src/shots.rs` +106/-1, `docs/parity.md`,
`docs/screenshots/loading.png`, plus the earlier uncommitted 08:58
sweep section in this file). Every gate below ran while the peer was
saving; code affecting the gates is HEAD unless noted.

- `cargo fmt --check`: clean, `exit=0`, on HEAD code at sweep start.
  Re-run on the dirty state: FAILS (`exit=1`) on three of the peer's
  new `shots.rs` hunks (import ordering `loading::{...}` x2, one
  over-long `game.select_songs()` chain). Not mine — left alone per
  the sweep rules (touch only `docs/playtest.md`, fix only my own
  lint findings), so no fmt fix was applied.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`, on HEAD code and re-run clean on the dirty state. No
  lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1386 passed, 0 failed, 1 ignored** (`o2jam_audio`), `exit=0` —
  ran twice, identical counts, both on HEAD code (the peer's
  `shots.rs` edit was reverted between the runs; `git status` showed
  no code files dirty either time). Per target: audio 19, dump 7,
  file 177, game 94, net 360, render 89, shell 509, real_art 7,
  scenes 22, recomp2jam 102 (6 doc-test targets, 0 each) — identical
  to the `1480651` baseline, as expected for a docs-only HEAD.
- `cargo build --workspace --release`: clean, `exit=0`, on HEAD code.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s, run twice green) against local
  Memoryer 7.2.0 (`Mode=Full`). First attempt at `09:54:36 +08:00`
  did NOT compile: it raced a peer save carrying a broken
  intermediate (`E0502` borrow in `shots.rs` test code); the peer
  fixed it within the minute and both re-runs passed. Server log for
  the confirming run (`09:56:07 +08:00`, session `127.0.0.1:51870`):
  `Authorize session` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session
  `127.0.0.1:51872` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
  The live path (`connect.rs`) is HEAD code; the peer's dirty files
  (`shots.rs`, `parity.md`, `loading.png`) do not touch it.
- **Verdict: no regression — suite is green on the consistency-head
  (1386 passed, identical to the endmarker-head baseline; docs-only
  delta, 0 failures) and the live song flow is green (Submit score
  2210 server-logged). The only red is `cargo fmt --check` on the
  current dirty state, caused solely by the peer's uncommitted
  `shots.rs` hunks — not fixed per the sweep rules. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 10:13 UTC+8 (HEAD `86d05e7`, tree clean)

Live re-proof on the guard-head. Covers the three commits landed after the
consistency-head baseline `1efaf71`: drive loading status into the shot log
(`ad4832b`), skip the staged loading test without archives (`b1cc51c`), and
guard the real-BGM test skip cleanly in CI (`86d05e7`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1389 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 509, real_art 7, scenes 22, recomp2jam 104
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`10:13:03 +08:00`):
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete; a second session walked
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get room list` →
  `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the guard-head (1389
  passed; +3 vs the 1386 consistency-head baseline: audio 19+1 ignored
  → 20 passed from the BGM-skip guard (`86d05e7`), recomp2jam
  102 → 104 from the loading-status shot-log work (`ad4832b`),
  all other targets unchanged, 0 failures). No lint findings. No
  commits, no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 10:26 UTC+8 (HEAD `0d0ad3a`, tree clean)

Live re-proof on the chatinput-head. Covers the feat commit landed after
the guard-head baseline `86d05e7`: type chat text into waiting draft
(`0d0ad3a`: waiting-draft keystroke handling in `game.rs`, key plumbing
in `window.rs`, wiring in `lib.rs`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1400 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 520, real_art 7, scenes 22, recomp2jam 104
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`10:25:29 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete; a second session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the chatinput-head
  (1400 passed; +11 vs the 1389 guard-head baseline, all shell
  509 → 520 from the waiting-draft typing tests, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.
- Tree note: a peer saved uncommitted in-flight work after every
  clean-HEAD gate above had finished (`game.rs` saved 10:26:00 +08:00:
  `send_chat_draft` → `pub take_chat_message` rename + doc rework;
  new untracked `crates/recomp2jam/src/session.rs` plus a `mod session;`
  line in `main.rs`, still being edited).
  Every count above is the clean-HEAD result. Re-run on the dirty
  state: `cargo fmt --check` stays clean, but `cargo clippy --workspace
  --all-targets -- -D warnings` FAILS with 7 errors, all inside the
  peer's new `session.rs` (dead `OnlineSession`/`into_parts`/`Write`
  code plus `bool_assert_comparison` and needless-`mut` lints in its
  tests). None of that is mine: per the sweep rules (touch only
  `docs/playtest.md`, fix only my own lint findings) the peer's files
  were left alone, and no suite re-run on the moving dirty state was
  taken.

## Regression sweep — 2026-11-08 10:34 UTC+8 (HEAD `b9b604b`, tree clean)

Live re-proof on the bridge-head. Covers the feat commit landed after the
chatinput-head baseline `0d0ad3a`: bridge shell chat draft to gateway
transport (`b9b604b`: `session.rs` online-session chat bridge,
`connect.rs`/`main.rs` wiring, `game.rs` draft-take rename).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1406 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 521, real_art 7, scenes 22, recomp2jam 109
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.61 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`10:34:03 +08:00`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete; a second session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the bridge-head (1406
  passed; +6 vs the 1400 chatinput-head baseline: shell 520 → 521,
  recomp2jam 104 → 109 from the chat-bridge work, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 10:46 UTC+8 (HEAD `5499e17`, tree dirty mid-sweep)

Live re-proof on the stats-head. Covers the feat commit landed after the
bridge-head baseline `b9b604b`: send live stats over session bridge
(`5499e17`: `crates/recomp2jam/src/session.rs` +424/-25 plus the sweep
log in this file).

- `cargo fmt --check`: clean, `exit=0`, and re-run clean on the dirty
  state below.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`, and re-run clean on the dirty state below. No lint
  findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1414 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 521, real_art 7, scenes 22, recomp2jam 117
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`10:44:48 +08:00`):
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARNs on room delete; a second session walked
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get room list: [0/00]`).
  Re-run on the dirty state (`10:45:58 +08:00`): **ok**
  (1 passed, 0 failed, 0.61 s; 117 filtered out, i.e. the peer's
  +1 in-flight session test compiled in and the flow still passes).
- **Verdict: no regression — suite is green on the stats-head (1414
  passed; +8 vs the 1406 bridge-head baseline, all recomp2jam
  109 → 117 from the stats-commit's session-bridge tests — `git show
  HEAD` confirms exactly 8 added `#[test]` — all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.
- Tree note: a peer saved uncommitted in-flight work three times
  mid-sweep (`o2jam-shell/src/playing.rs` 10:44:11, `screens/game.rs`
  10:44:32, `recomp2jam/src/session.rs` 10:45:02 +08:00; +212/-35
  overall, 4 added `#[test]` with none removed). The workspace-test
  counts above pin to HEAD — the peer's 4 tests are absent from them
  (shell 521 = baseline, recomp2jam 117 = 109 + the stats commit's 8).
  fmt, clippy, and the live flow were each re-run after the saves and
  stayed green on the dirty state. Per the sweep rules (touch only
  `docs/playtest.md`, fix only my own lint findings) the peer's files
  were left alone.

## Regression sweep — 2026-11-08 10:51 UTC+8 (HEAD `c9083d6`, tree clean)

Live re-proof on the life-mirror head. Covers the feat commit landed after
the stats-head baseline `5499e17`: close life mirror over session bridge
(`c9083d6`: `recomp2jam/src/session.rs` rework plus shell
`playing.rs`/`screens/game.rs` life-mirror work, plus the sweep log in
this file).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1418 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 524, real_art 7, scenes 22, recomp2jam 118
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.59 s; 117 filtered out) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`10:50:54 +08:00`): `Create room: songflow` →
  `Update room [000] music settings` → `Update room [000] arena` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] music state` →
  `Update room member [000] download progress` →
  `Start game: [000]` → `User music loaded: [000]` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the life-mirror head
  (1418 passed; +4 vs the 1414 stats-head baseline: shell 521 → 524,
  recomp2jam 117 → 118 from the life-mirror commit's tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 11:03 UTC+8 (HEAD `37081a0`, tree clean at start)

Live re-proof on the mirror-shot head. Covers the feat commit landed after
the life-mirror baseline `c9083d6`: show session-fed peer gauges in shots
(`37081a0`: `shots.rs` session-fed peer gauges, shot refreshes
(`playing.png`, `playing-native.png`, `waiting.png`), `parity.md` notes),
plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0` (on clean HEAD at sweep start).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1419 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 524, real_art 7, scenes 22, recomp2jam 119
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 118 filtered out, 0.60 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`11:02:03 +08:00`, session `127.0.0.1:39730`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session
  `127.0.0.1:39736` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the mirror-shot head
  (1419 passed; +1 vs the 1418 life-mirror baseline, all recomp2jam
  118 → 119 from the session-fed peer-gauge shot test, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer saved uncommitted in-flight work after every gate
  above had finished (`crates/o2jam-shell/src/playing.rs` +63,
  saved 11:02:20 +08:00 — after the 11:02:04 live run). Every count
  above is the clean-HEAD result. Re-run on the dirty state:
  `cargo fmt --check` FAILS (`exit=1`) on one of the peer's new hunks
  (import ordering in `playing.rs`, needs rewrapping), while
  `cargo clippy --workspace --all-targets -- -D warnings` stays clean.
  Per the sweep rules (touch only `docs/playtest.md`, fix only my own
  lint findings) the peer's file was left alone.

## Regression sweep — 2026-11-08 11:16 UTC+8 (HEAD `dff765d`, tree clean)

Live re-proof on the peer-bars head. Covers the feat commit landed after
the mirror-shot baseline `37081a0`: layer peer bars gate master start
(`dff765d`: `playing.rs` +111, `screens/game.rs` +181 peer-bar layering
with master-start gating, `paint.rs` +4, `room-enums.md` +48), plus the
sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1423 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 528, real_art 7, scenes 22, recomp2jam 119
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 118 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`11:15:59 +08:00`, session `127.0.0.1:35026`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session
  `127.0.0.1:35038` walked `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get room list: [0/00]` → `Get character info: [tester]`).
- **Verdict: no regression — suite is green on the peer-bars head
  (1423 passed; +4 vs the 1419 mirror-shot baseline, all shell
  524 → 528 from the peer-bar layering tests, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.
- Tree note: a peer saved uncommitted in-flight waiting-guest work
  after every HEAD gate above had finished
  (`crates/recomp2jam/src/shots.rs`, doc comments + `shoot_guest_waiting()`
  wired into `capture_all`, capacity 12 → 13). Every count above is the
  clean-HEAD result. Re-run on the dirty state: `cargo fmt --check` clean,
  `cargo clippy --workspace --all-targets -- -D warnings` clean (one
  mid-save compile hit 2 transient errors while the peer was editing;
  fresh re-run clean), `cargo test -p recomp2jam` **115 passed, 4 failed**
  — all in `shots::tests` (`native_shots_share_the_walked_source_frame`:
  `shot waiting-guest: at Room, want Waiting`; plus
  `staging_attempt_never_breaks_the_walk`,
  `walk_covers_all_scenes_plus_solo_result`,
  `writer_saves_eleven_scenes_and_keeps_avi`), i.e. the guest walk is not
  yet staged to `Screen::Waiting` and the writer/scene-count pins still
  expect eleven scenes. Per the sweep rules (touch only `docs/playtest.md`,
  fix only my own lint findings) the peer's file was left alone.

## Regression sweep — 2026-11-08 03:28 UTC (HEAD `1336284`, tracked tree dirty)

Live re-proof on the guest-head. Covers the guest waiting-view commit
(`1336284`: `shoot_guest_waiting` + select-gate in
`crates/recomp2jam/src/shots.rs`, new `waiting-guest.png`, room-enums
docs) over the peer-bars baseline `dff765d`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1426 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 528, real_art 7, scenes 22, recomp2jam 122
  (doc-test targets, 0 each). The 4 dirty-state `shots::tests`
  failures from the last sweep are resolved on this HEAD, including
  `guest_capture_matches_master_waiting_but_not_select_view`.
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 0.62 s) against local Memoryer 7.2.0
  (`Mode=Full`). Server log for this run (`11:27:47 +08:00`):
  `Authorize session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the guest-head (1426
  passed; +3 vs the 1423 peer-bars baseline, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.
- Tree note: `docs/gameplay.md` holds uncommitted peer edits (16
  insertions); left alone per the sweep rules.

## Regression sweep — 2026-11-08 11:36 UTC+8 (HEAD `0f084d4`, tree clean)

Live re-proof on the f2-head. Covers only the docs-log commit landed after
the guest-head baseline `1336284` (`0f084d4`: f2 team-pick refutation in
`gameplay.md` plus the sweep log) — no code changed, so no new tests were
expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1426 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 360,
  render 89, shell 528, real_art 7, scenes 22, recomp2jam 122
  (6 doc-test targets, 0 each) — identical to the baseline, as
  expected for a docs-only HEAD.
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 121 filtered out, 0.59 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`11:35:34 +08:00`): `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the f2-head (1426
  passed; identical to the guest-head baseline, docs-only delta,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 11:47 UTC+8 (HEAD `e525f24`, tree clean)

Live re-proof on the lobby-head. Covers the net commit landed after
the f2-head baseline `0f084d4` (`e525f24`: decode lobby chat and
whispers).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1441 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 375,
  render 89, shell 528, real_art 7, scenes 22, recomp2jam 122
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 121 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`11:47:06 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the lobby-head (1441
  passed; +15 vs the 1426 f2-head baseline, all net +15 from the
  lobby-chat/whisper decode tests, 0 failures). No lint findings.
  No commits, no pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: all gates above ran on a clean tree (HEAD `e525f24`);
  a peer landed uncommitted edits to
  `crates/o2jam-shell/src/screens/game.rs` at ~11:47:21 +08:00,
  after the last gate finished. Left alone per the sweep rules;
  (note: the sentence above is where the prior sweep stopped writing;
  left as-is, not mine to finish).

## Regression sweep — 2026-11-08 11:55 UTC+8 (HEAD `7c81f96`, tree clean at gates)

Live re-proof on the lobby-head. Covers the shell commit landed after
the lobby-head baseline `e525f24`: paint lobby chat rows (`7c81f96`:
lobby chat-row paint in `screens/game.rs` + `screens/paint.rs`), plus
the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1453 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 375,
  render 89, shell 540, real_art 7, scenes 22, recomp2jam 122
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: `exit=0` with 1 pre-existing
  `dead_code` warning (`recomp2jam` bin: `flush_channel_whisper` never
  used in `session.rs`; HEAD code, not mine — left alone per the
  sweep rules, which allow fixing only my own lint findings).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 121 filtered out, 0.59 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`11:54:48 +08:00`): `Create room: songflow` →
  `Game [000] Submit score: 2210` (a trailing session walked
  `Authorize session` → connection-lost close).
- **Verdict: no regression — suite is green on the lobby-head (1453
  passed; +12 vs the 1441 `e525f24` baseline, all shell 528 → 540
  from the lobby chat-row paint tests, all other targets unchanged,
  0 failures). No lint findings of mine. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.
- Tree note: all gates above ran on a clean tree (HEAD `7c81f96`);
  a peer saved uncommitted edits to
  `crates/o2jam-shell/src/screens/game.rs` and
  `crates/recomp2jam/src/session.rs` after the last gate finished.
  Left alone per the sweep rules.

## Regression sweep — 2026-11-08 12:03 UTC+8 (HEAD `8884b91`, tree clean)

Live re-proof on the lobbysend-head. Covers the send commit landed
after the lobby-head baseline `7c81f96`: send lobby chat over session
bridge (`8884b91`: lobby chat send path in `screens/game.rs` +
`session.rs`), plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1467 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 375,
  render 89, shell 548, real_art 7, scenes 22, recomp2jam 128
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 127 filtered out, 0.60 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`12:02:42 +08:00`): `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the lobbysend-head
  (1467 passed; +14 vs the 1453 `7c81f96` baseline: shell 540 → 548
  and recomp2jam 122 → 128 from the lobby-chat send tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 12:12 UTC+8 (HEAD `c9e94ed`, tree clean)

Live re-proof on the whisper-head. Covers the whisper commit landed
after the lobbysend-head baseline `8884b91`: parse whisper slash
commands route 2017 (`c9e94ed`), plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1491 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 375,
  render 89, shell 565, real_art 7, scenes 22, recomp2jam 135
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 134 filtered out, 0.59 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`12:11:51 +08:00`): `Authorize session` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the whisper-head
  (1491 passed; +24 vs the 1467 `8884b91` baseline: shell 548 → 565
  and recomp2jam 128 → 135 from the whisper slash-command tests, all
  other targets unchanged, 0 failures). No lint findings. No commits,
  no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 12:21 UTC+8 (HEAD `5bb1c5b`, tree dirty mid-sweep)

Live re-proof on the notice-head. Covers the notice commit landed
after the whisper-head baseline `c9e94ed`: decode announce and rank
notices (`5bb1c5b`: `command.rs` + `message.rs` notice variants,
`rooms.rs` announce/rank decode, `net-protocol.md` notes), plus the
sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1503 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 387,
  render 89, shell 565, real_art 7, scenes 22, recomp2jam 135
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 134 filtered out, 0.59 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`12:21:00 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression on HEAD — suite is green on the notice-head
  (1503 passed; +12 vs the 1491 `c9e94ed` baseline: net 375 → 387
  from the announce/rank notice decode tests, all other targets
  unchanged, 0 failures). No lint findings on HEAD. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted notice follow-up work after the
  first gate pass (`crates/o2jam-shell/src/screens/paint.rs`: 2016
  announcement + 2029 rank-notice chat rows, `rank_notice_text`,
  bare-text bracket rule; then `screens/game.rs`: `channel_announcement`
  / `channel_rank` constructors). Re-ran on the dirty state: fmt now
  reports 3 diffs, all in the peer's `game.rs`; clippy `-- -D warnings`
  fails with 2 dead-code errors (`channel_announcement`, `channel_rank`
  never used); `cargo test --workspace` stays green at **1511 passed,
  0 failed** (shell 565 → 573 from the peer's +8 tests, all else
  unchanged); `cargo build --workspace --release` clean, `exit=0`.
  Every remaining finding sits inside the peer's uncommitted files —
  left alone per the sweep rules (this sweep fixed nothing and touched
  only `docs/playtest.md`).

## Regression sweep — 2026-11-08 12:31 UTC+8 (HEAD `5556827`, tree clean)

Live re-proof on the notices-head. Covers the notice paint/route commit
landed after the notice-decode baseline `5bb1c5b`: paint and route live
lobby notices (`5556827`: shell notice chat-row paint + routing), plus
the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1518 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 387,
  render 89, shell 573, real_art 7, scenes 22, recomp2jam 142
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings
  (0 warning lines on a touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 141 filtered out, 0.60 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`12:30:36 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARNs on session close).
- **Verdict: no regression on HEAD — suite is green on the notices-head
  (1518 passed; +15 vs the 1503 `5bb1c5b` baseline: shell 565 → 573
  and recomp2jam 135 → 142 from the notice paint/route tests, all
  other targets unchanged, 0 failures). No lint findings on HEAD.
  No commits, no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 12:37 UTC+8 (HEAD `7ae7377`, tree clean)

Live re-proof on the notices2-head. Covers the live 2016 capture commit
landed after the notices-head baseline `5556827`: lock live 2016 capture
log re-proof (`7ae7377`: `rooms.rs` capture test + `net-protocol.md`
notes), plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1519 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 388,
  render 89, shell 573, real_art 7, scenes 22, recomp2jam 142
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 141 filtered out, 0.59 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`12:37:34 +08:00`): `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression on HEAD — suite is green on the notices2-head
  (1519 passed; +1 vs the 1518 `5556827` baseline: net 387 → 388
  from the live 2016 capture test, all other targets unchanged,
  0 failures). No lint findings on HEAD. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 12:49 UTC+8 (HEAD `b2c8c20`, tree clean)

Live re-proof on the p1-head. Covers the join-drain commit landed after
the notices2-head baseline `7ae7377`: join rooms drain chat in online
session (`b2c8c20`: cli online-session chat drain on room join), plus the
audit online-loop docs log (`1065680`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1531 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 388,
  render 89, shell 573, real_art 7, scenes 22, recomp2jam 154
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 153 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`12:48:42 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression on HEAD — suite is green on the p1-head
  (1531 passed; +12 vs the 1519 `7ae7377` baseline: recomp2jam
  142 → 154 from the join-drain chat tests, all other targets
  unchanged, 0 failures). No lint findings on HEAD. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 13:01 UTC+8 (HEAD `da7477b`, tree dirty mid-sweep)

Live re-proof on the p2-head. Covers the cli commit landed after the
p1-head baseline `b2c8c20`: ready start and 4011 advance online
(`da7477b`: `crates/recomp2jam/src/session.rs` +301 ready-start gating
and 4011 start-broadcast advance, plus the sweep log in this file).

- `cargo fmt --check`: clean, `exit=0` (run at sweep start and re-run
  clean on the dirty state below).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0` (run at sweep start and re-run clean on the dirty state
  below). No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1541 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 388,
  render 89, shell 578, real_art 7, scenes 22, recomp2jam 159
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 158 filtered out, 0.60 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`12:59:16 +08:00`, session `127.0.0.1:57116`):
  `Authorize session` → `Get channel list: [0]` →
  `Enter channel: [0/00]` → `Get character info: [tester]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session
  `127.0.0.1:57118` walked `Authorize session` →
  `Get channel list` → `Enter channel` → `Get room list` →
  `Get character info`).
- **Verdict: no regression — suite is green (1541 passed; +10 vs the
  1531 `b2c8c20` baseline: recomp2jam 154 → 159 from the commit's 5
  new ready-start/4011 session tests, shell 573 → 578 from the
  peer's 5 uncommitted `force_advance` tests below, all other targets
  unchanged, 0 failures). No lint findings of mine. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: the tree was clean at sweep start, but a peer saved
  uncommitted work mid-sweep (`crates/o2jam-shell/src/screens/game.rs`
  +87, saved 12:58:12 +08:00: `force_advance_to_loading` guest-4011
  advance plus 5 tests). The pinned full-suite counts above ran on
  that dirty state — shell 578 = 573 HEAD + the peer's 5 tests, all
  passing — and fmt, clippy, build, and the live flow were each
  (re-)run green with it present. Per the sweep rules (touch only
  `docs/playtest.md`, fix only my own lint findings) the peer's file
  was left alone.

## Regression sweep — 2026-11-08 13:16 UTC+8 (HEAD `f6d4f21`, tree clean)

Live re-proof on the mirrors-head. Covers the two member-update commits
landed after the p2-head baseline `da7477b`: decode member updates
correct ready empty (`1769da3`) and drain member updates into session
mirrors (`f6d4f21`), plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1558 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 402,
  render 89, shell 578, real_art 7, scenes 22, recomp2jam 162
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 161 filtered out, 0.60 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`13:15:59 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the mirrors-head
  (1558 passed; +17 vs the 1541 `da7477b` baseline: net 388 → 402
  from the member-updates decode tests, recomp2jam 159 → 162 from
  the session-mirrors drain tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 13:26 UTC+8 (HEAD `6dddd6c`, tree clean at gates)

Live re-proof on the p3-head. Covers the cli commit landed after the
mirrors-head baseline `f6d4f21`: windowed song select with chart fetch
(`6dddd6c`), plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0` (on clean HEAD at sweep start).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1568 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 402,
  render 89, shell 578, real_art 7, scenes 22, recomp2jam 172
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 171 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`13:25:02 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the p3-head (1568
  passed; +10 vs the 1558 `f6d4f21` baseline, all recomp2jam
  162 → 172 from the windowed song-select chart-fetch tests, all
  other targets unchanged, 0 failures). No lint findings. No commits,
  no pushes.** This sweep touched only `docs/playtest.md`.
- Tree note: a peer saved uncommitted in-flight work after every
  clean-HEAD gate above had finished
  (`crates/recomp2jam/src/session.rs` +426/-15, saved 13:25:16 +08:00).
  Every count above is the clean-HEAD result. Re-run on the dirty
  state: `cargo fmt --check` FAILS (`exit=1`, 2 diff hunks, both in
  the peer's `session.rs`: a `send_frame` chain wrap and a test
  assert); `cargo clippy --workspace --all-targets -- -D warnings`
  stays clean; `cargo test -p recomp2jam` → **177 passed, 0 failed**
  (+5 from the peer's new session tests, all passing); the live flow
  re-run on the dirty state → **ok** (1 passed, 176 filtered out,
  0.59 s; server-logged `Submit score: 2210` at `13:26:04 +08:00`).
  None of the fmt diffs are mine: per the sweep rules (touch only
  `docs/playtest.md`, fix only my own lint findings) the peer's file
  was left alone.

## Regression sweep — 2026-11-08 13:33 UTC+8 (HEAD `c04be2f`, tree clean)

Live re-proof on the tally-head. Covers the cli commit landed after the
p3-head baseline `6dddd6c`: loading sync and tally in session
(`c04be2f`: `crates/recomp2jam/src/session.rs` +447/-15 loading-sync
and tally work), plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1573 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 402,
  render 89, shell 578, real_art 7, scenes 22, recomp2jam 177
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 176 filtered out, 0.59 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`13:31:43 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get character info`).
- **Verdict: no regression — suite is green on the tally-head (1573
  passed; +5 vs the 1568 `6dddd6c` baseline, all recomp2jam
  172 → 177 from the loading-sync/tally session tests, all other
  targets unchanged, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 13:48 UTC+8 (HEAD `66d782e`, tree clean)

Live re-proof on the broadcasts-head. Covers the net/cli commit landed
after the tally-head baseline `c04be2f`: score-broadcast decode
server mirror (`66d782e`: new `crates/o2jam-net/src/play.rs` +
`session.rs` broadcast-mirror work), plus the sweep log in this file.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1590 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 418,
  render 89, shell 578, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`13:47:51 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Update room [000] arena: [1] (0)` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get character info`).
- **Verdict: no regression — suite is green on the broadcasts-head
  (1590 passed; +17 vs the 1573 `c04be2f` baseline: net 402 → 418
  from the score-broadcast decode tests, recomp2jam 177 → 178 from
  the session mirror test, all other targets unchanged, 0 failures).
  No lint findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-08 13:58 UTC+8 (HEAD `ab8f52f`, tree clean)

Live re-proof on the dossiers-head. Covers the docs-only commit landed
after the broadcasts-head baseline `66d782e`: file score dossiers log
(`ab8f52f`: `docs/net-protocol.md` 4017 ScoreSubmission / 4018
ScoreCompleted + row-reader / 4040 GameRankUpdate exe-verified handler
notes, plus the sweep log) — no code changed, so no new tests were
expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1590 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 418,
  render 89, shell 578, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.60 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`13:58:24 +08:00`): `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the dossiers-head
  (1590 passed; identical to the 1590 `66d782e` baseline, as expected
  for a docs-only head, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 06:05 UTC (HEAD `da5bdd8`, tree clean)

Live re-proof on the scoreopen-head. Covers the code commit landed
after the dossiers-head baseline `ab8f52f`: score open-items close
(`da5bdd8`: `crates/o2jam-net/src/play.rs` + `docs/net-protocol.md`
4017/4018 handler notes, plus the sweep log).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1592 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 94, net 420,
  render 89, shell 578, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`14:04:56 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210` (followed by the known benign
  connection-lost WARN on session close; a trailing session walked
  `Authorize session` → `Get channel list` → `Enter channel`).
- **Verdict: no regression — suite is green on the scoreopen-head
  (1592 passed; +2 vs the 1590 `ab8f52f` baseline: net 418 → 420
  from the score open-items tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — grade-ratio head (1601 passed)

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`.
- `cargo test --workspace`: **1601 passed, 0 failed, 0 ignored**,
  `exit=0`.
- `cargo build --workspace --release`: clean, `exit=0`.
- Live `live_host_song_flow_against_local_gateway`: **ok**
  (1 passed, 0.62 s) vs local Memoryer 7.2.0.
- **Verdict: no regression.** Grade denominator pinned
  (`grade_max_reference`); numerator narrowed to pointer writes.

## Regression sweep — board-sentence head (1595 passed)

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`.
- `cargo test --workspace`: **1595 passed, 0 failed, 0 ignored**,
  `exit=0`.
- `cargo build --workspace --release`: clean, `exit=0`.
- Live `live_host_song_flow_against_local_gateway`: **ok**
  (1 passed, 0.59 s) vs local Memoryer 7.2.0.
- **Verdict: no regression.** Rank rows render the exe board
  sentence; the stale session shot test mirrors the new wording.

## Regression sweep — grade numerator head (1604 passed)

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`.
- `cargo test --workspace`: **1604 passed, 0 failed, 0 ignored**,
  `exit=0`.
- `cargo build --workspace --release`: clean, `exit=0`.
- Live `live_host_song_flow_against_local_gateway`: **ok**
  (1 passed, 0.59 s) vs local Memoryer 7.2.0.
- **Verdict: no regression.** Grade numerator pinned as the live
  per-judgement total (`grade_numerator()`); ratio fully modeled.

## Regression sweep — 2026-11-08 14:51 UTC+8 (HEAD `d7c3873`, tree clean)

Live re-proof on the grade-head. Covers the three game commits landed
above the scoreopen baseline `da5bdd8`: exe board sentence for rank
rows (`0589b3a`), grade max-reference pin (`a98d89c`), live-total
grade numerator (`2130d0e`), plus the ratio wiring that retires the
stand-in tests (`d7c3873`: live `grade_numerator`/`grade_max_reference`
ratio in `o2jam-game`, shell `game.rs`/`paint.rs` rank rows, `shots.rs`
refresh).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1612 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 177, game 107, net 420,
  render 89, shell 585, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.63 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`14:50:55 +08:00`): `Authorize session` →
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the grade-head (1612
  passed; +8 vs the 1604 numerator-head baseline, 0 failures). No
  lint findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-08 07:01 UTC (HEAD `5610f55`, clean at start)

Live re-proof on the unscored-head. Covers the file commit landed
above the grade-head baseline `d7c3873`: lane backing counts split
narrow unscored (`5610f55`).

- `cargo fmt --check`: clean, `exit=0` (also clean on the dirty
  state below).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0` (also clean on the dirty state below). No lint findings,
  nothing fixed, no files touched.
- `cargo test --workspace` on clean HEAD (no live env; the live test
  self-skips): **1615 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 107, net 420,
  render 89, shell 585, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (also 0 warnings on the dirty state below).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`15:00:17 +08:00`): `Authorize session` →
  `Get channel list: [0]` → `Enter channel: [0/00]` →
  `Get character info: [tester]` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `music state` →
  `download progress` → `Start game: [000]` →
  `Submit score: 2210`.
- **Verdict: no regression — suite is green on the unscored-head
  (1615 passed; +3 vs the 1612 grade-head baseline, all file +3 from
  the lane-backing split, 0 failures). No lint findings. No commits,
  no pushes.**
- Tree note: a peer landed uncommitted edits mid-sweep
  (`crates/o2jam-game/src/lib.rs`: `grade_max_reference` reworked to
  a full-word `note_count * 200` base with an early `0` return below
  27 records; `docs/cresult-anchors.md`: matching notes). Re-ran the
  suite on that dirty state with `--no-fail-fast`: **1613 passed,
  3 failed** (game 108 = +1 new peer test passing; shell 582 + 3
  failures, all `live_plate_*` paint tests asserting the old
  denominator — e.g. `live_plate_paints_its_frame_at_the_pinned_anchor`:
  `left: 0, right: 16`). fmt, clippy, and the release build stay clean
  (0 warnings) under the dirty state. The 3 failures belong to the
  peer's uncommitted work, not to HEAD `5610f55`. This sweep touched
  only `docs/playtest.md`.

## Regression sweep — 2026-11-08 07:38 UTC (HEAD `72ea4ca`, tree clean)

Live re-proof on the boot-head. Covers the grade-base fix commit landed
above the unscored-head baseline `5610f55`: wine boot proof (`72ea4ca`:
game grade-base rework, shell paint alignment, `docs/wine-boot.md`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1616 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 89, shell 585, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`15:38:25 +08:00`): `Authorize session` →
  `Create room: songflow` → `Get music list: [0/00] 562 music` →
  `Start game: [000]` → `Game [000] Submit score: 2210`
  (followed by the known benign broadcast-failure stack on room
  delete; a second session walked `Get channel list` →
  `Enter channel` → `Get character info`).
- **Verdict: no regression — suite is green on the boot-head (1616
  passed; +1 vs the 1615 unscored-head baseline, game +1 from the
  grade-base fix, all other targets unchanged, 0 failures). No lint
  findings. No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-08 08:20 UTC (HEAD `e725060`, tree clean)

Live re-proof on the argv-head. Covers the launcher argv pin + wine boot
proof docs commit (`e725060`) landed above the boot-head baseline `72ea4ca`
(docs-only; no code change expected).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1616 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 89, shell 585, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`16:20:54 +08:00`): `Create room: songflow` →
  `Start game: [000]` → `Game [000] Submit score: 2210`
  (followed by the known benign broadcast-failure stack on room
  delete).
- **Verdict: no regression — suite is green on the argv-head (1616
  passed; +0 vs the 1616 boot-head baseline, as expected for a
  docs-only commit, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 08:41 UTC (HEAD `398d5ee`, tree clean)

Live re-proof on the wine-head. Covers the two docs-only commits landed
above the argv-head baseline `e725060`: channel live over wine log
re-proof (`a3fba88`) and channel input blocked over wine (`398d5ee`);
no code change expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1616 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 89, shell 585, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`16:41:34 +08:00`): `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Get channel list` → `Enter channel` → `Get character info`).
- **Verdict: no regression — suite is green on the wine-head (1616
  passed; +0 vs the 1616 argv-head baseline, as expected for two
  docs-only commits, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 08:46 UTC (HEAD `f3f1a94`, tree clean)

Live re-proof on the handoff-head. Covers the docs-only commit landed
above the wine-head baseline `398d5ee`: handoff log refresh (`f3f1a94`);
no code change expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1616 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 89, shell 585, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`16:46:46 +08:00`): `Create room: songflow` →
  `Start game: [000]` → `Game [000] Submit score: 2210` (followed by
  the known benign broadcast-failure stack on room delete; a second
  session walked `Authorize session` → `Get channel list` →
  `Enter channel` → `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the handoff-head (1616
  passed; +0 vs the 1616 wine-head baseline, as expected for a
  docs-only commit, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 13:25 UTC (HEAD `b908833`, tree clean)

Live re-proof on the wedge-head. Covers the seven docs-only commits
landed above the handoff-head baseline `f3f1a94`: input-path log
(`61e7c2f`), room-stage click proof (`a024c4f`), room-journey blockers
(`fd3227a`), waiting-room reach (`32da26b`), loading-screen
reach (`6ef9ae2`), scratch-dir rule (`ae7c642`), and the loading-wedge
log (`b908833`); no code change expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1616 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 89, shell 585, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines (touched-`main.rs` rebuild check).
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.60 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`21:25:07 +08:00`): `Authorize session` →
  `Create room: songflow` → `Update room [000] music settings` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the wedge-head (1616
  passed; +0 vs the 1616 handoff-head baseline, as expected for seven
  docs-only commits, 0 failures). No lint findings. No commits, no
  pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 13:33 UTC (HEAD `c4c0317`, tree clean)

Live re-proof on the seatgrid-head. Covers the shell fix commit landed
above the wedge-head baseline `b908833`: four-column seat grid
(`c4c0317`, `screens/paint.rs` + parity/playtest logs); one new shell
test expected.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1617 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 89, shell 586, real_art 7, scenes 22, recomp2jam 178
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 177 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`21:33:07 +08:00`): `Authorize session` →
  `Create room: songflow` → `Update room [000] music settings` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure stack on room delete; a second session walked
  `Authorize session` → `Get channel list` → `Enter channel` →
  `Get room list` → `Get character info`).
- **Verdict: no regression — suite is green on the seatgrid-head (1617
  passed; +1 vs the 1616 wedge-head baseline, shell +1 from the seat-grid
  fix tests, all other targets unchanged, 0 failures). No lint findings.
  No commits, no pushes.** This sweep touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 14:03 UTC (HEAD `7646ce4`, tree clean at start)

Live re-proof on the cover-head. Covers the three cover commits landed
above the seatgrid-head baseline `c4c0317`: stage ojn cover illustration
loading (`9970dd1`), feed walked ojn into loading cover (`2aba6b8`), and
paint cover on real chart loads (`7646ce4`).

- `cargo fmt --check`: clean, `exit=0` on the clean start state.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1636 passed, 0 failed, 0 ignored**, `exit=0` (two identical runs).
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 89, shell 595, real_art 7, scenes 22, recomp2jam 188
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 187 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`22:02:03 +08:00`): `Authorize session` →
  `Create room: songflow` →
  `Update room [000] music settings: [HX / X10 / o2ma757]` →
  `Get music list: [0/00] 562 music` →
  `Sync room member [000] [0] music state` →
  `Update room member [000] [0: 100%] download progress` →
  `Start game: [000]` → `User music loaded: [000] - 0 / X10` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the cover-head (1636
  passed; +19 vs the 1617 seatgrid-head baseline: shell +9,
  recomp2jam +10 from the cover-illustration tests, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits mid-sweep
  (`crates/o2jam-render/src/font.rs`,
  `crates/o2jam-shell/src/screens/paint.rs`, `docs/channel-fill.md`;
  143 insertions, 26 deletions). Mid-edit states briefly showed a
  `cargo fmt` import-order diff and one failing shell cover test
  (`loading_cover_paints_full_bleed_behind_bar_and_caption`, caption
  scrim bottom-left); the peer kept editing and the final dirty-state
  re-run is green: `cargo fmt --check` clean, `cargo clippy` clean,
  `cargo test --workspace --no-fail-fast` **1639 passed, 0 failed**
  (render 90, shell 597 — the peer's in-progress additions passing).
  The 1636-passed verdict above is the clean-HEAD result.

## Regression sweep — 2026-11-08 14:09 UTC (HEAD `3af901f`, tree clean at start)

Live re-proof on the type-head. Covers the loading-bar exclusion
trebuchet-cell work (`3af901f`: render `font.rs` + shell `paint.rs` +
`docs/loading-type.md`) landed above the cover-head baseline `7646ce4`.

- `cargo fmt --check`: **FAIL on clean HEAD**, `exit=1`. Two diffs,
  both in `crates/o2jam-shell/src/screens/paint.rs` as committed by
  this head: the `use o2jam_render::font::{...}` import sorts after
  the `use o2jam_render::{...}` group, and one multi-line `assert!`
  collapses to one line. Not fixed — this sweep touches only
  `docs/playtest.md`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1638 passed, 0 failed, 0 ignored**, `exit=0` (two identical runs).
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 90, shell 596, real_art 7, scenes 22, recomp2jam 188
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 187 filtered out, 0.62 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`22:09:17 +08:00`): `Authorize session` → `Create room` →
  `Get music list` → `Start game` → `Game [000] Submit score: 2210`.
- **Verdict: REGRESSION — suite is green on the type-head (1638
  passed; +2 vs the 1636 cover-head baseline: render +1, shell +1
  from the trebuchet-cell tests, 0 failures) but `cargo fmt --check`
  is red on committed HEAD. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.
- Tree note: a peer landed uncommitted edits mid-sweep
  (`crates/o2jam-shell/src/screens/paint.rs`: 37 insertions,
  8 deletions). A `cargo fmt --check` re-run on the dirty state is
  clean (`exit=0`); no other gate was re-run after the peer edit, so
  the counts above are the clean-HEAD result.

## Windowed session walk — PASS (2026-11-08 22:31 +08:00, HEAD dirty)

`cargo run -p recomp2jam -- session-walk
--token 61dcb457-5da9-4e22-af4b-5a4c5658f239
--out-dir docs/screenshots/session` → **exit=0**, 11 PNGs, one per
stage (`docs/screenshots/session/01-connect-avi.png` … `11-result.png`,
400x300 each). New code (uncommitted, no pushes):
`crates/recomp2jam/src/session_walk.rs` (headless `OnlineSession`
walk + PNG writer), `session.rs` (`stage_hosted_room`,
`create_and_join_room`, `note_arena_selected`, `select_song` now
consumes the pending pick), `main.rs` (`session-walk` subcommand).
Headless `paint_at` framebuffers only; audio never attached (silent).

Server log for this run (room `walk173`, song 757 HX, arena 1):

- `03E8` Authorize → `03EA` Get channel list →
  `03EC` Enter channel [0/00] → `07D2` Get room list
- `07D4` Create room → `07D2` room list (snapshot rows)
- `0FA0` Update room music [HX/o2ma757] → `0FA2` arena [1] →
  `0FBE` Get music list (562) → `0FAA` Start game →
  `0FAC` User music loaded → `0FAE` Game Update Jam + Life →
  `0FB0` Submit score: **144964** → `0FC9` Finalize rank →
  `0FB5` Leave game

505 chart onsets struck at their own clocks (exact-onset Cools), so
the tally is the honestly played sheet, not a fixture. Two
findings, both handled: re-joining our own room breaks the server
master check (the walk maps the room list instead of `JoinWaiting`);
the 4011 broadcast trails the 1 ms paint drain (the walk polls with
rests, then bails loudly instead of faking the stage).

Honest gaps: 05/06 share the Waiting screen (pick vs chart staged —
placeholder art renders them identically); 09/10/11 share the Result
screen (submit landed, 4018 table folded, final — identical pixels at
this size). Per-stage opcodes are in the walk stdout
(`~/scratch/walk.log`).

Gates for the touched files: `cargo test -p recomp2jam`
**193 passed, 0 failed** (incl. live-guarded
`walk_reaches_result_against_local_gateway`); `cargo fmt --check`
clean; `cargo clippy -p recomp2jam --all-targets -- -D warnings`
clean. No commits, no pushes.

## Regression sweep — 2026-11-08 22:47 UTC+8 (HEAD `de98e62`, tree clean)

Live re-proof on the session-head. Covers the staged-archive session
walk (`47fc0fe`) plus cover-on-real-loads (`7646ce4`), loading-caption
(`3c0542a`), and barbar log (`3af901f`) commits landed after the
type-head baseline, plus the `session-walk` stage-art commit
(`de98e62`).

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1647 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 90, shell 597, real_art 7, scenes 22, recomp2jam 196
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, no warnings.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 195 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`22:47:05 +08:00`): `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Submit score: 2210`.
- **Verdict: no regression — suite is green on the session-head (1647
  passed; +9 vs the 1638 type-head baseline: file +1, render +1,
  shell +4, recomp2jam +3 from the staged session-walk work,
  0 failures). No commits, no pushes.** This sweep touched only
  `docs/playtest.md`.

## Regression sweep — 2026-11-08 22:58 UTC+8 (HEAD `ebd2adb`, tree clean)

Live re-proof on the audio-head. Covers the key-sounds session-walk
commit (`ebd2adb`: attach key sounds in session walk) landed after the
session-head baseline `de98e62`.

- `cargo fmt --check`: clean, `exit=0`.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live test self-skips):
  **1655 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 90, shell 597, real_art 7, scenes 22, recomp2jam 204
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warning
  lines.
- Live flow `O2JAM_LIVE_GATEWAY=127.0.0.1:15010
  O2JAM_LIVE_TOKEN=61dcb457-5da9-4e22-af4b-5a4c5658f239 cargo test
  -p recomp2jam live_host_song_flow_against_local_gateway` → **ok**
  (1 passed, 0 failed, 203 filtered out, 0.61 s) against local
  Memoryer 7.2.0 (`Mode=Full`). Server log for this run
  (`22:57:36 +08:00`): `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `Game [000] Update #1 - Jam: 3 (Bonus: 0)` →
  `Game [000] Submit score: 2210` (followed by the known benign
  broadcast-failure WARNs on room delete; a second session walked
  `Get channel list: [0]` → `Enter channel` → `Get character info`).
- **Verdict: no regression — suite is green on the audio-head (1655
  passed; +8 vs the 1647 baseline, all recomp2jam +8 from the
  key-sounds session-walk tests, all other targets unchanged,
  0 failures). No lint findings. No commits, no pushes.** This sweep
  touched only `docs/playtest.md`.

## Regression sweep — 2026-11-08 23:32 UTC+8 (HEAD `dc8128b`, tree dirty)

Live re-proof on the audible-head. Covers the audible commit landed
above the audio-head baseline `ebd2adb`: strike-to-emission probe
through the real bank (`dc8128b`: `session_walk.rs` audible test +
`live_guard` rig lock, token rotation; no walk-shape change).

- `cargo fmt --check`: clean, `exit=0` (final state; was red
  mid-sweep on the peer's unformatted intermediate edit).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean,
  `exit=0`. No lint findings, nothing fixed, no files touched.
- `cargo test --workspace` (no live env; the live tests self-skip):
  **1662 passed, 0 failed, 0 ignored**, `exit=0`.
  Per target: audio 20, dump 7, file 180, game 108, net 420,
  render 90, shell 597, real_art 7, scenes 22, recomp2jam 211
  (6 doc-test targets, 0 each).
- `cargo build --workspace --release`: clean, `exit=0`, 0 warnings.
- Live flow with the rotated token
  `O2JAM_LIVE_TOKEN=eebaccee-fde9-4c50-b941-fda6c7004376`
  (handoff note; the old `61dcb457…` guid no longer authorizes):
  `cargo test -p recomp2jam live_host_song_flow_against_local_gateway`
  → **ok** (1 passed, 0 failed, 210 filtered out, 0.61 s) against
  local Memoryer 7.2.0 (`Mode=Full`); `struck_live_notes_emit_audible_frames_through_real_bank`
  → **ok** (1 passed, 0 failed, 210 filtered out, 11.06 s; walks
  the live session to Playing on song 757, strikes the first chart
  onset through the real decoded bank, probe observes frames out
  plus nonzero PCM). Server log (`23:31:26 +08:00`): `Authorize
  session` → `Create room: songflow` →
  `Get music list: [0/00] 562 music` → `Start game: [000]` →
  `User music loaded` → `Game [000] Update #1` →
  `Game [000] Submit score: 2210`; audible run (`23:31:29 +08:00`):
  `Authorize session` → `Create room: walk890` → `Get music list` →
  `Start game` → `User music loaded` (followed in both runs by the
  known benign broadcast-failure stack on room delete).
- **Verdict: no regression on the dirty state (1662 passed; +7 vs
  the 1655 baseline, all recomp2jam 204 → 211, all other targets
  unchanged, 0 failures). No lint findings. No commits, no pushes.**
  This sweep touched only `docs/playtest.md`.
- Tree note: a peer holds uncommitted wallet WIP
  (`crates/recomp2jam/src/connect.rs` +418: `WalletReport`,
  `ConnectOptions.wallet`, 5029/5001 fetchers; `main.rs` +28;
  `session_walk.rs` +1). Mid-sweep the intermediate state briefly
  failed to compile the `recomp2jam` test target (`fetch_wallet`
  not yet defined, initializers missing `wallet`) and tripped
  `cargo fmt --check`; the peer kept editing and every gate above
  ran green on the finished dirty state. No clean-HEAD-only run
  was taken (would require stashing the peer's work).

## Windowed A/V proof — 2026-11-09 01:30 +08:00 (HEAD `6675c63`, tree clean)

Isolated rigs only (own Xvfb `:94/:95/:96/:97`, null sink, Wayland
stripped): user desktop and speakers never touched.

- **Audible key sounds: PASS.** Offline `play --window --chart
  o2ma757 --ojm o2ma757 --sound` with 18 rounds of S D F Space J K L
  strikes in Playing, `PIPEWIRE_NODE=wine_null` routing proven first
  (`PipeWire ALSA [recomp2jam]` → `WineNull`, nothing to hardware).
  `parec` on `wine_null.monitor`: peak 0 dBFS, RMS −27.3 dBFS over
  ~28 s — signal, not silence. Captures in `~/scratch/smoke-audible/`.
  Residual: peak at full scale hints stacked voices clip; mixer
  behavior is unverified in-exe, so no verdict (OPEN).
- **Offline Result in the presenter: PASS.** Tutorial walked
  splash → Playing with scripted Enters, then unattended ~3m45s to a
  stable all-miss tally (`YOU 0/0/0/20`, gold banner). Life gate never
  reappeared (life still unwired shell-side, OPEN). Captures in
  `~/scratch/smoke-result/` with a 26-event trace.
- **Containment fix shipped:** windowed runs without `--sound` now
  attach `NullBackend` (`audio_backend`, shared with autoplay);
  previously any OJM run opened the default ALSA device. Pinned by
  `audio_backend_off_queues_silently_without_hardware` (fails red
  without the fix: playhead 0 vs queued 50).
