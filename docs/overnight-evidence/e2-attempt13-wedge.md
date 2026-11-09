# E2 — Attempt-13 wedge: Playing never starts under Wine

Status: **BLOCKER (documented, unresolved)**. One hypothesis tested this
session (play-stage surface burst). No Rust code changed. Nothing committed.

Pinned client for every number below: `OTwo.exe` 5,029,888 B,
SHA-256 `83a04541…72c823b` (pins in `docs/wine-boot.md`,
`~/scratch/geom-open-close/findings.md`). Rig for the traces: Wine 11.21
wow64, ko prefix, Xvfb `:98`/`:99`, audio on the `wine_null` null sink
(never speakers), Memoryer local server `127.0.0.1:15010`.

## Verdict

The guest reaches the song loading screen and completes the full play
handshake, then never enters `STATE_PLAYING`: zero guest 4014/4016, a
byte-identical frozen frame, a hot render loop, no crash dump. The
handshake itself is exonerated; the stall sits between the loading
screen handoff and the first judgement tick.

## The hypothesis (one)

The LOADING→PLAYING transition stalls inside the first play-surface
staging burst under Wine: applier `0x4e7850` (`docs/stage-bga.md`)
walks 41 Playing entries (loop bound `0x29`) and each fetch runs
slot loader `0x48b2d0` → sprite-55 reader `0x4249F0` (asserts
`.\Display\DXSurface.cpp`, `0x555`/`0x555a` compares at
`0x424c96`/`0x424caf`, ev `103158a8…` in `docs/claims.md`) → page
upload `0x42a580` (create/SetRect/lock/upload/unlock in
`docs/ui-draw.md`). This is the same surface path proven fragile under
Wine by the waiting-room arena reload, whose failure raises the
`DXSurface.cpp:660` interface-information error
(`docs/wine-boot.md`, Attempt 12). The play burst is the largest
single load in the client (37 numbered stage stills
`Playing_BG<1..36>.ojs`, each one 596×558 frame at anchor `(204,0)`
in `docs/stage-bga.md`, plus stage-keyed note kits) — a Wine-only
stall there freezes the client on the last successfully painted
frame, which is exactly the observed loading art.

## Trace evidence (wire is healthy; relay refuted)

Guest-side tee trace `~/scratch/geom-open-close/tee_guest.log`:

- Line 57: server `4011`; lines 58+60: server `4013` twice
  (`0001`, `0101`); line 59: guest `4012` ConfirmMusicLoaded
  (`0000000001`, power 0 / speed 1; layout in
  `docs/net-protocol.md` §4c); line 62: server `4056` — the relay
  confirm answers the guest's `4055` at line 61. The relay is NOT
  the blocker (both Attempt 13 and geom-open-close agree).
- Lines 63–84: eleven `4048`/`4049` latency rounds (guest thread
  alive); lines 85–94: ten server `4015` GameStatsUpdate frames
  (member 0, Jam type; layout in `docs/net-protocol.md` §4c);
  line 99: server `4017` naming the host after the bot's `4016`
  submit. After line 99 the guest sends only 6001 keepalive.
- Guest 4014/4016 counts: zero. The server never emits `4018`
  (it waits for the guest's own `4016`). Attempt 13's census
  matches (`host_play.log`): bot ten `4014`, bot `4016`/`4041`,
  `4017` (slot 00) — guest `4014`/`4016` absent in every cycle.

Two-thread reading: the network thread answers latency and consumes
host ticks while the frame/state thread never advances. The guest
receives `4017` (the song-end signal) and ignores it — this rules
out a waiting-for-signal start barrier.

## Capture evidence (frame frozen, loop hot)

- `~/scratch/geom-open-close/play-01.png` and `play-39.png` are
  byte-identical (md5 `2a0792f97fa22a6bc96c4ba1afe51134`); 40
  consecutive frames show the song loading art (`미쳤어`, 3KEY
  VERSION badge, BPM 124).
- Attempt 13's `~/scratch/z-play{1,2,3,4a,4b}.png` are likewise
  byte-identical (md5 `e2df29e00dff0e08986453886cc92b2a`).
- Loading ignores click/Return/Escape; render loop spins hot
  (~228% CPU presenting the frozen frame). No `.dmp` is written by
  the client's own fault handler (`docs/wine-boot.md` pins
  `SetUnhandledExceptionFilter(0x408b70)`), so the stall is a
  live-lock/spin, not a crash.
- Window focus was pinned to the canvas (`0xc0004e`, 5 s re-assert
  keeper) for the whole Attempt-13 play; mapping is 1:1
  (desktop `0xa00007` → container `0xe00001` → canvas
  `0xc0004e`). Focus is necessary but not sufficient — this wedge
  is not an input/hit-test failure.

## Static evidence (what the init must do)

Loading gate (`docs/lifecycle.md` §9): select validation `0x5718b0`
(ev `66e60d16…`), staging `0x4f8b80` (ev `d2a69b45…`), OJN
open/decrypt `0x412180` (ev `011eab73…`), codec init `0x54db40`
(ev `0ee1373d…`), loading screen `0x4e47a0` (ev `68d920c2…`, creates
`Trebuchet MS`/`Arial` fonts and `MusicLoading.ojs`). The loading art
renders, so steps 1–5 succeeded: Basic DirectDraw surface creation
works under Wine, and the failure is specific to the post-handoff
burst, not to the API itself.

Playing sequence (`docs/lifecycle.md` §10): song start `0x56ffa0`
(ev `f3ce92fb…`), playback setup `0x571d60` (ev `c0325c85…`),
per-frame background-effect loop `0x550db0` (ev `4164031c…`). First
4014 send sites (Cool/Good Jam tails files `0x16e660`/`0x16e7dd`,
Life tick file `0x4eaec0`; `docs/net-protocol.md` §4c,
`docs/claims.md` D5) never run — the Judgement path is never reached.

No-BGA closure (`docs/stage-bga.md`): there is no song-supplied video
to load — OJM containers carry audio only, the OJN-embedded JPEG is
album art, and the stage backdrop is one static `Playing_BG<n>.ojs`
staged through the same applier. The BGA-video explanation is
excluded; the remaining surface suspects are the 41-entry stage
applier burst feeding blit consumer `CDXSurface::Draw` `0x42b4d0`
(ev `867ac248…`) and page dispatch `0x42b450` (ev `08a33fe4…`).

## What is NOT the cause

| Candidate | Status | Pin |
|---|---|---|
| Relay / handshake barrier | REFUTED | tee lines 57–99 above; `4056` answers `4055` |
| Start barrier on host tick | REFUTED | guest consumes ten `4015` frames and ignores `4017` |
| BGA video surface | CLOSED | `docs/stage-bga.md` (no song video exists) |
| Basic DirectDraw broken | REFUTED | loading screen + waiting room render through the same reader/upload path |
| Focus / hit-test | REFUTED | focus pinned to `0xc0004e`, 1:1 mapping, Attempt 13 |
| Audio init | REFUTED | codec init is gate step 4, before the rendered loading screen; null-sink audio initializes on every run |

## Blocker

`COMBO_ROW_ABOVE_LINE` (`playing.rs:268`) and the avatar plate
offsets `AVATAR_LEVEL_OFFSET`/`AVATAR_LIFE_OFFSET`
(`playing.rs:2492–2496`) stay OPEN, blocked by this wedge — the
live Playing scene cannot be captured under Wine to pin combo-row
height or avatar anchors (`docs/geometry-audit.md` OPEN list).

Unresolved question (static evidence cannot separate these): the
hot loop spins, so the stall is either a repeated-failure retry
spin in the surface/lock path or a clock-wait spin in the
frame-advance path. Only a stopped-state read names the thread's
instruction pointer.

## Next named step

One stopped-state winedbg inspection at the wedge, under full
containment (own prefix, Xvfb, `PULSE_SINK=wine_null`), following
the 9e297 discipline exactly: launch `OTwo.exe` directly (never
explorer), PTY with CR line endings, ONE Ctrl-C stop only, queue all
reads before it lands, never `cont` (cont dies on the wow64
helper-thread fault `0xffbc11b0`). Read, in order:

1. `info thread` + per-thread EIP (network thread healthy vs
   render thread spinning — name which instruction frame spins).
2. Parent object `[0x762B44]`, current state `[P+0xF6C]` with vtable
   (the planet-verify pins), phase flag `[P+0xF64]`, stage global
   `0x7789dc`.
3. Whether slot loader `0x48b2d0` / reader `0x4249F0` / upload
   `0x42a580` / draw `0x42b4d0` frames sit on the spinning thread's
   stack, and what the last successful upload left as the current
   frame.

If the spinning frame is inside the surface path, the DXSurface
hypothesis confirms and the fix direction is a Windows-rig control
capture. If it is inside the song-clock wait, the blocker moves to
the clock arm (`0x4e47a0` step 5) and the surface suspect closes.
A Windows rig is the standing fallback control (see
`docs/geometry-audit.md` next step wording).

## Hygiene

Scratch only; no repo bytes touched by the traces (all captures and
logs under `~/scratch/geom-open-close/` and `~/scratch/`); game
copy read-only; `install-root/` untouched (archive reads only);
all processes reaped (`pkill -9 -x OTwo.exe`, bot + tee killed);
no `.dmp` in the run dir; audio stayed on `wine_null`; nothing
committed.
