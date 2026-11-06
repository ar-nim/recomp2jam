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
