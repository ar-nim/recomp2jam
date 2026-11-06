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
