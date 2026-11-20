# Rig runbook — single home for rig lore

Rig: the Wine plus Xvfb plus SendInput evidence setup on a private
display with null audio. Captures stay local (git-ignored scratch paths);
only committed manifests, findings, and this runbook live in the repo.

This file is the single home for rig procedure. Every other doc points
here instead of restating rig lore. AGENTS.md carries one pointer line
to this file.

## Scope and pairing

A run drives the client through scenes, injects input, and observes
judged lines. Three input-cross-video pairs exist; a fourth fixture is
protocol-only and never counts as a live pair.

| Pair | Input driver | Video backend | Status | Proof |
|---|---|---|---|---|
| Xvfb + XTEST | XTEST (X11 protocol events) | self-spawned Xvfb, 1024x768 | **Proven** | native pair: Playing reached, 60 judged presses (`docs/captures/t76/findings.md`) |
| Live seat + ydotool | ydotool (kernel uinput) | real compositor seat | Manual spot-check only; never feeds numbers | procedure proven and dry-run pinned at `docs/captures/t242/seat-dry-run.txt`; live execution needs an owner-present supervised unlocked session (#242, ready-for-human) |
| Wine X11 + SendInput | winegcc SendInput helper | Wine under Xvfb | **Qualified to the room stage** | `scripts/rig-wine-run.sh` pack, pair `wine-x11-sendinput`: planet → channel → room with provenance (`docs/captures/t240/findings.md`); waiting/loading OPEN (join wall, same note) |
| Synthetic compositor fixture | `wl_pointer` injection | headless compositor | Protocol-only | CI seam only, never a budget pair (#126, #127) |

Xvfb + XTEST is the blessed baseline. No pair feeds a budget number
before its same-run A/B qualification against a blessed pair
(`docs/captures/t127/02-backends.md`).

## Standard run

One manifest per run records `pair`, `build`, `params`, `script`, and
`seed`. One artifact layout per run holds the manifest copy, the input
log, screenshots, and stats. A verifier rejects incomplete sets (#236).

### Rust-client display pin (#719)

`scripts/play-local.sh` boots only our Rust client. Before build, token lookup,
or spawn, it calls `scripts/lib/rig-client-display.sh` with the caller chain
`play-local.sh -> rig_pin_rust_client_display -> rig_validated_client_display`.
A boot must name a rig Xvfb display as `:NNN` in `RIG_DISPLAY` (preferred) or
`DISPLAY`; the owner's `:0`, short display names, and dead displays are refused
with exit 10. The pin exports `DISPLAY` and `RIG_DISPLAY`, clears
`WAYLAND_DISPLAY`/`WAYLAND_SOCKET` and Wine prefix variables, and forces
`WINIT_UNIX_BACKEND=x11`. The peer-only recipe is:

```sh
env -u WAYLAND_DISPLAY -u WINEPREFIX DISPLAY=:NNN RIG_DISPLAY=:NNN \
  bash scripts/play-local.sh --no-sound --frame-dump DIR --input-log LOG
```

`rig-run.sh` clears an inherited `WINEPREFIX`/`RIG_WINEPREFIX` for native
boots (binary basename `recomp2jam`, or `RIG_CLIENT_KIND=native`) before
`rig_check_prefix`, so a native peer cannot trip exit 15 on a Wine master's
claim. Wine launcher boots keep their prefix. The owner EXE+Wine path
`~/scratch/owner-exe-run.sh` is untouched.

### Screenshot layout contract (#302)

Every manifest declares `artifacts/screenshots` relative to the run
out-dir, and every writer lands its screenshots there. The Wine runner
captures directly into that path. The AFK runner leaves the client's
`frames/*.png` where the client wrote them and copies them into
`artifacts/screenshots` at package time before writing the manifest, so a
fresh native pack has the same comparable layout as a Wine pack.

`run-manifest.sh verify PACK1 PACK2` compares those directories. It fails
loudly when either side has zero screenshot files (`CROSS-PAIR has no
comparable screenshots`) and when any screenshot bytes repeat across the
pair (`CROSS-PAIR duplicate screenshots`). An empty comparison is never a
pass. The hermetic `scripts/run-manifest.sh selftest` pins three scenario
names: `empty-screenshot-set-is-rejected`,
`cross-pair-duplicate-is-rejected`, and `cross-pair-distinct-is-accepted`.

Capture provenance is emitted, not hand-written (#281,
`docs/reversing.md` section 6):

```
scripts/rig-run.sh run <display> <out-dir> -- <binary> [args...]
scripts/rig-provenance.sh emit <out-dir> --producer <binary> \
    --producer-kind exe --producer-version 7.55 --capture-tool ffmpeg
scripts/run-manifest.sh verify <out-dir>      # fails an unmanifested capture
scripts/rig-provenance.sh cite <out-dir> --claim exe   # citation paragraph
```

`emit` reads the run dir that `rig-run.sh` leaves behind (`pids` -> full
command, `env.sh` -> display and sink, `claims` -> display), probes the
tool versions, hashes every capture, and writes `provenance.json` with
its own manifest hash last. `--tool-version ROLE VERSION` pins a tool
whose version query the host cannot answer. An exe run must name its
capture tool: the exe cannot dump its own Wine pixels.

`cite --claim exe` prints the paragraph only when the manifest is
complete and the producer is the exe itself; that is the check that
would have refused the #278 retraction. A run dir written before
`rig-run.sh` recorded `pids` has no discoverable command: pass
`--command` and `--build` by hand, or `verify` fails loud (an unnamed
command is not provenance).

### Memoryer liveness, identity, and dumps (#345)

The shared preflight does more than a TCP connect probe. `rig-run.sh run`
looks up the process listening on the Memoryer port with `ss -tlnp` and
reads its start time from `/proc/<pid>/stat` plus `/proc/stat`'s `btime`.
It writes one line to `<out-dir>/server-identity`:

```
memoryer_pid=92326 memoryer_boot_epoch=1794986056 memoryer_boot_utc=2026-11-18T07:14:16Z
```

Both pack writers (`rig-afk.sh`, `rig-wine-run.sh`) append those fields
to the manifest `params`, so a server restarted between two legs is
visible from the packs alone:

```sh
jq -r '[.params, .created_at] | @tsv' leg-a/manifest.json leg-b/manifest.json
# different memoryer_boot_epoch (or memoryer_pid) = a new server: continuity void
```

Both clients use that one server, so server state matches by construction.
A scenario delta is still no dismissal without wire and server-log proof;
A/B packs must agree on visible chrome — DOWNLOADING badges (07E8) and the
closed gate — before a pixel verdict.

A preflight that expects Memoryer but cannot name the listener fails
with exit 13; it never restarts or signals the server. `RIG_EXPECT_MEMORYER=0`
records `unknown` fields and is for hermetic tests only. Never restart
Memoryer during a live leg: announce a restart, and start the next pack
after it.

The documented start command is `scripts/memoryer-start.sh`; it refuses to
start a second instance when the port is taken (exit 13, identity of the
current listener printed) and starts one with runtime minidumps enabled.
`scripts/play-local.sh` prints that command when the port is dead. On a
crash, `DOTNET_DbgEnableMiniDump=1` writes a managed heap dump
(`DOTNET_DbgMiniDumpType=2`, type `Heap`) to
`~/scratch/memoryer-dumps/coredump.<pid>.<epoch>.dmp` as named by
`DOTNET_DbgMiniDumpName`. This is exactly what the wrapper runs:

```sh
mkdir -p "$HOME/scratch/memoryer-dumps"
cd ~/memoryer-local && setsid nohup env PATH=$HOME/.dotnet:$PATH \
  DOTNET_DbgEnableMiniDump=1 DOTNET_DbgMiniDumpType=2 \
  DOTNET_DbgMiniDumpName="$HOME/scratch/memoryer-dumps/coredump.%p.%t.dmp" \
  dotnet bin/Memoryer.Encore.dll >> server.log 2>&1 < /dev/null &
```

The 14:05:36 SIGABRT was that second-instance class: the stored core holds
`SocketException (98): Address already in use` from
`Memoryer.DefaultWorker.StartAsync` (`DefaultWorker.cs:52`) via
`Encore.Server.TcpServer<T>.Start` and `TcpListener.Start`; the unhandled
exception passed through `Host.StartAsync` and `Program.Main`
(`Program.cs:225`) and the runtime aborted the stray process. The main
instance kept serving.
The runtime dump is the one with a managed stack for `dotnet-dump`;
the kernel dump from systemd-coredump stays at
`/var/lib/systemd/coredump/core.dotnet.1000.<boot-id>.<pid>.<epoch>.zst`
(for example the 14:05:36 abort:
`/var/lib/systemd/coredump/core.dotnet.1000.c0b9ef761ea749a7b60b564bba220889.106781.1794981936000000.zst`,
compressed with zstd; use `coredumpctl info <pid>` to find the path).

The 15:10:49 clean shutdown of the main instance was a KDE reboot
requested by `plasma-shutdown` (`systemd-logind: reboot requested from
client PID 691435 (plasma-shutdown)`), not systemd-oomd or slice
pressure; the session sent SIGTERM to its background jobs. An OOM event
would have a `systemd-oomd` kill line, which the journal lacks.

### Client identity and duplicate sessions (#726)

Memoryer does **not** deduplicate on the 7.55 client id. It matches the
**token** of an already-channeled session
(`Memoryer/Controllers/AuthController.cs:35-39`): a live duplicate gets
`1001` result `-2` (`AlreadyConnected`), and a dead duplicate is exited
before the new session authenticates. The `ClientId` is only logged and
stored on `Actor`. Therefore two same-host clients must use two
`--user-index-id` accounts (distinct tokens), even though both Wine
clients send the same client id. Never restart Memoryer to clear a
duplicate: retire the extra account session instead.

The client id itself is fixed in 7.55: the exe's 1000 builder appends the
literal at file `0x2472c8`, byte-identical to `726PNA-B1YW-28ZG9M-I5SC-Q0OPM6`
(`docs/client-server-plan.md`). The launch blob varies `UserIndexId`,
`Username` (token), and `Gender`; it never varies the client id. A live
socket probe (2026-11-20 22:24+08:00, `memoryer-server` PID 92326) proved
the constraint: three sessions with tokens tester/tester2/t207probe and
the **same** fixed client id all entered channel 0/0 and stayed alive;
a fourth auth with uid 1's token and a **different** client id returned
`-2` and left the older session alive. See
`~/scratch/distinct-clientids/probe-after-adjust.log`.

Our Rust client keeps that exe literal by default. Rig boots that want
one distinct *logged* identity per uid set `RIG_DISTINCT_CLIENT_IDS=1`:
`play-local.sh --user-id N` then exports
`O2JAM_CLIENT_ID=726PNA-B1YW-28ZG9M-I5SC-Q0OPM6-UIDN`, which
`connect::authorize` puts in the 1000 frame. An explicit
`O2JAM_CLIENT_ID` wins. Single-client owner boots are unchanged.
Hermetic guard: `bash scripts/lib/rig-client-identity.sh selftest`.
The override is a rig observability aid, not a substitute for distinct
uids; the exe path cannot use it without editing the oracle.

Rig spec story row (the GitHub issue body stays untouched under the
no-remote rule):

- [x] Concurrent rig clients use distinct uids/tokens; two same-host
  sessions coexist and the guest joins the host's room (#726; two-exe
  pack `win2/f5`, trace `S294-RESULT-MULTI-726-02`). The fixed 7.55
  ClientID is not a session key and is unchanged by default.

Two-exe identity leg: both Wine clients keep the fixed exe client id, so
`ClientID:` lines in `server.log` repeat; that is expected. Use a distinct
uid per exe (master 1, guest 2, optional observer 3). The room must also
fit the guest's mode: tester2 is level 1 and gets the blue
`NORMAL mode only` box when it double-clicks a 7KEY room row. For an
identity-only join, host `NORMAL + 3KEY DRUM` — select NORMAL then 7KEY
(the sequence that makes CREATE OK fire), then click 3KEY DRUM before
OK. Scratch recipe: `~/scratch/result-harness/scripts/run-two-exe-identity-726.sh`.

### Committed capture records (#308)

A committed record such as `docs/captures/t240/` is a hash-pinned pointer,
not a citable run dir: binary captures stay in the git-ignored local pack,
and the committed copies are prefixed (`wine-provenance.json`,
`wine-manifest.json`, `wine-citation.txt`). `record-hashes.txt` pins their
sha256 values.

```
bash scripts/rig-provenance.sh check-record docs/captures/t240
```

`check-record` needs only the committed directory, so it works in CI: it
verifies all three hashes and checks that `wine-citation.txt` quotes the
manifest hash and every artifact hash in the pinned provenance. A changed
byte is a loud nonzero exit. Re-citing with `cite ... --claim exe` requires
the local, git-ignored pack `~/scratch/240-fix/wine2`; `cite` still refuses
the committed directory by design. The local pack is never committed: it holds
the copyrighted screenshots and audio that the pin's citation hashes.

On a machine that holds that pack, add `--local-pack` to tie the pointer to it:

```
bash scripts/rig-provenance.sh check-record docs/captures/t240 \
    --local-pack ~/scratch/240-fix/wine2
```

The extended check hashes the pack's `provenance.json`, `manifest.json` and
`citation.txt`, requires each to equal the matching committed pin, and requires
the pack's citation text to equal `wine-citation.txt`. CI omits the flag and
checks the committed pins alone. Both modes fail loudly on drift
(`scripts/tests/test_rig_provenance_record.py` pins the one-byte red proof).

The scheduled injector stamps one epoch per press to its own record file
across all lanes; the pairing script matches epochs to judged lines,
validates the count, and reports p50, p99, and max. A count mismatch
fails loudly (#237).

Drive the full planet-to-waiting path with control ids and staged rect
centers, so no hand-read coords remain and two consecutive runs produce
the identical scene sequence (#239).

Drive every A/B pair from one shared input trace (`recomp2jam-input-v1`:
ordered press/release events, song-position clock for gameplay,
monotonic clock for menus). Record the trace ID in both manifests and
compare only on matching IDs: what is pressed and where the pointer is
changes what renders, so mismatched stimuli void the pixels (#245).

## Wine leg (#240)

One command produces the Wine pair's pack, composing the same kit:

```
scripts/rig-wine-run.sh OUT_DIR [--target room|loading] [--display :N]
    [--music-id ID] [--user-index-id N] [--host-user-index-id N] [--cross DIR]
```

| Phase | Kit command | Substitution home |
|---|---|---|
| mint argv | `scripts/rig-mint-launch-argv.sh` (#265 builder, read-only DB) | that script |
| boot | `scripts/rig-run.sh run` → `scripts/rig-wine.sh launch` | `scripts/rig-wine.sh` |
| drive | `scripts/wine-scene.py --attach OUT --display :N` | that script |
| host a room | `scripts/wine-room-host.py` (the exe cannot create one) | that script |
| stop | `scripts/rig-run.sh stop` (pidfile only) | `scripts/rig-run.sh` |
| package | `run-manifest.sh write` + `rig-provenance.sh emit --producer-kind exe` | those scripts |
| check | `run-manifest.sh verify [--cross partner]`, `rig-audio.sh verify`, `cite --claim exe` | those scripts |

Wine UI evidence under winedbg launches on `OTwo.exe` directly (never
`explorer`); read stopped state only — `cont` and breakpoints die on the
wow64 helper fault `c0000005@0xffbc11b0`. In the PTY, `\r` ends commands
and Ctrl-C is `\x03`; stop once, then queue all `x` reads
(`docs/wine-boot.md`).

Pins the leg enforces: the exe sha256 (default `83a04541…72c823b`, 7.55),
`WINEPREFIX` (a Korean-locale prefix, claimed by the preflight), the
`o2jam` 800x600 Wine desktop, and a per-run display. The launcher resets
its own prefix's `wineserver` before exec (a relaunch against a stale
window id dies in `X_CreateWindow BadWindow`), and the drive labels a scene
only when its pixels match a measured signature — a frame that shows
another scene is reported, never relabelled.

Known gaps from the qualification run (each with its next step in
`docs/captures/t240/findings.md`): the waiting-room join and a loading-gate
pixel signature. The `artifacts/screenshots` versus `frames/` split was
closed by #302 with the shared layout contract above, and the launcher-vs-
`OTwo.exe` route guard gap was closed by #311 with the exe stream alias
(stub-verified; a live re-run belongs to the next exe leg).

## AFK end-to-end runner (#280)

One unattended command boots, drives, captures, packages, and checks a run:

```
scripts/rig-afk.sh <out-dir> [--pair native] [--scene-script PATH]
    [--target loading|playing] [--music-id ID]
    [--inject-keys lane-0,lane-3,...] [--inject-count N]
    [--inject-interval-ms N] [--min-frames N] [--display :N] [--binary PATH]
scripts/rig-afk.sh selftest
```

It composes the kit instead of repeating it:

| Phase | Kit command | Substitution home |
|---|---|---|
| allocate | display scan (:150-:199) + `rig_claimed_by` | `scripts/rig-afk.sh` |
| boot | `scripts/rig-run.sh run` (preflight, owned sink, audio lane) | `scripts/rig-run.sh` |
| drive | the scene script in `--attach` mode | `scripts/planet-to-waiting.sh` |
| inject | `scripts/rig-inject.sh` (own record file) | `scripts/rig-inject.sh` |
| stop | `scripts/rig-run.sh stop` (pidfile only) | `scripts/rig-run.sh` |
| package | stage `frames/*.png` into `artifacts/screenshots`, then `run-manifest.sh write` + `rig-provenance.sh emit` | those scripts |
| check | scene probe, frame count, `pair-stats.sh`, `rig-audio.sh verify`, `run-manifest.sh verify` | those scripts |

The native session argv (binary, login, room, frame dump, input log) lives in
one place, `scripts/lib/rig-session.sh`, so the runner and the standalone drive
cannot drift. A scene script must accept `--attach OUT_DIR --display :N`;
`scripts/planet-to-waiting.sh` ships that mode (plus `--sound`, `--music-id`,
`--target`).

Self-checks, all of them red on failure and quoted verbatim in
`<out-dir>/report.md` and `<out-dir>/report.json`:

1. `scene` — the scene probe on the run's input log reports the target scene.
2. `frames` — frame PNGs in `<out-dir>/frames` reach the floor (default 4).
3. `judged` — with an injector record, `pair-stats.sh` pairs it with the
   client's `judged` lines and the counts must match; a menu-only pack says
   `SKIP ... (menu-only drive)` instead of passing silently.
4. `audio` — the sidecar must declare `expect=sound`, `state=captured`, and
   `rig-audio.sh verify` must measure above the floor.
5. `manifest` — `run-manifest.sh verify` (artifact set, staged screenshots
   distinct, provenance complete, audio verified).

`--inject-interval-ms` (default 250) keeps the scheduled-press interval in
one place: the injector receives it and the manifest `params` records it.
A gameplay pack also writes `artifacts/stats/judged.json` from
`pair-stats.sh --min-offset`; the figure is normalized by the fastest paired
delivery, with the raw and `--relative` modes left unchanged.

The packager stages one copy of each distinct frame sha256 into
`artifacts/screenshots`. A held scene can write identical pixels many times;
those repeats stay only in the client's `frames/` dump and never count as
comparable screenshots.

`<out-dir>/status` holds `PASS` or `FAILED`; the runner exits 0 or 1. A failed
check keeps the whole pack — nothing is deleted and nothing is cleaned up — and
names the gap in the report. Every phase is bounded
(`RIG_AFK_BOOT_TIMEOUT` 90 s, `RIG_AFK_DRIVE_TIMEOUT` 300 s,
`RIG_AFK_INJECT_TIMEOUT` 120 s, `RIG_AFK_STOP_TIMEOUT` 90 s), every teardown
is PID-file based, and Ctrl-C stops the run through the pidfile before writing
a FAILED report. The runner refuses an out-dir that already holds files: a
mixed pack cannot be cited, and deleting the old one would destroy the record
of the run that failed.

`selftest` is hermetic (stub Xvfb, stub PipeWire tools, stand-in client, drive,
and injector): one happy leg with five green checks, one menu-only leg for the
declared skip, and six red legs — wrong scene, frame floor, judged mismatch,
audio lane switched off, silent audio, provenance dropped — each printing the
`check ... FAIL` line it asserted. `RIG_AFK_FAULT` exists only for that last
leg; a real run leaves it unset.

## Number bar

Every latency or budget figure carries sample count and build profile.
The entry bar for a ticket is 200 samples on a release build. Provisional
debug numbers (for example the 60 judged presses above) never harden
silently and never close a budget ticket (#241). Supersede, do not reuse,
when the bar is met.

## File-separation rule

Rig artifacts never share a path with client outputs. The injector keeps
its own record file; it never writes into the client input log. Manifest,
input log, screenshots, and stats live in the run's out-dir. Client
outputs (input log, frame dump) stay on the paths the client wrote them:
`frames/` remains the client's frame dump, and the staged copy in
`artifacts/screenshots` is the pack's comparable screenshot set (#302).

## Commit-or-lose-it rule

No load-bearing script lives in /tmp: helpers land in `scripts/` or
their full reconstruction recipe is pinned in the runbook. Two builders
died in /tmp in one night (exe auth-blob builder, wine host-room); both
recipes survive in `docs/wine-boot.md`, and the wine host script is
committed. (#264)
`scripts/rig-run.sh` records every PID it started in `<out-dir>/pids`
and every claim it owns in `<out-dir>/claims`; it never pattern-kills,
and cleanup failures keep the evidence instead of deleting it.
`pgrep -f`/`pkill -f` patterns self-match the invoking shell as well as the
target; use the recorded PIDs, never a pattern, for rig processes.

## Boot, drive, aim, observe

- Recipes first, never from memory: quote the documented command for
  the exact leg before adapting it, and record adaptations as their
  own trial. Bare guesses against documented recipes failed every
  time they were tried (exe launch, #261).
- Eat your own rig: every run — headless or supervised seat — goes
  through the kit scripts (preflight, PID discipline, polling waits,
  sanitized env). No bare sleeps, no pattern-kill, no hand-rolled
  display handling where a kit command exists (#263).
- Boot: `scripts/rig-run.sh run <display> <out-dir> -- <binary> [args...]`
  runs the shared preflight (display, audio sink, Memoryer, install-root,
  relay ports, Wine prefix), starts Xvfb when absent, launches the
  binary, and writes `env.sh`. Stop with `scripts/rig-run.sh stop
  <out-dir>`; status with `status`. Wayland mode uses
  `scripts/rig-run.sh wayland <socket> <out-dir> -- <binary>`.
- Game data: `scripts/worktree-setup.sh` claims one of four persistent slots
  in `~/scratch/install-data`, refreshes it from the main checkout, and
  symlinks `install-root/`/`extracted/` there. Slots are never shared, so
  `gate-check.sh` renaming this checkout's symlink hides only this run;
  release with `scripts/worktree-release.sh` (slot data stays on disk).
- Displays: Xvfb is the workbench. Agent rig work always runs headless
  and invisible; nothing ever lands on the live seat unless the owner
  needs eyes on it. A seat run means a normal window the owner can
  see, announced first, pointer restored after, and never a
  fullscreen takeover (owner rule 2026-11-16).
- Drive: `scripts/rig-drive.sh` (ydotool) for move, click, and key;
  `dry-run` prints planned commands without touching input. The XTEST
  backend (#234) drives a native Xvfb client through the same interface.
  `scripts/planet-to-waiting.sh --attach <out-dir>` drives a session the
  caller already booted (the AFK runner's shape); `--music-id` stages the
  host pick (first Waiting Confirm) so the second reaches Loading, and
  `--target playing` Confirms at Loading (4012) into Playing. The drive first
  dismisses the NORMAL-mode message box when its pinned blue body is up
  (`scripts/lib/planet_popup.py`, OK face at `(400,365)`), and joins the room
  row named by the client's own room number: a shared Memoryer can put other
  sessions in row 0. Dialog-up is detected from the pinned X-plate NCC
  template (`docs/captures/t207/channel.md`); the blue NORMAL-mode body must
  clear before any canvas click, including under winedbg (`docs/wine-boot.md`).
- Aim: chrome coords resolve from staged BND rects, never hand-read
  pixels. Native aim uses the dump tool's rect query for a BND control
  id (#235); unknown ids fail loudly. Wine aim cites pinned reference
  evidence for exe-side targets (#240). See `docs/geometry-audit.md`.
- Observe: the scene probe reports the current scene as text from the
  capture log, so navigation costs no image reads (#236). Judged lines
  come from the client input log (`--input-log`); frames from
  `--frame-dump`; audio from the run's virtual sink through the monitor
  tap described below (`<out-dir>/audio/`, #279).

## Boot-failure taxonomy (retro #490)

One live lane at a time is now enforced, not volunteered: every client
boot takes the flock in `scripts/lib/rig-live-lock.sh` (exit 17 on a
timed-out acquire, with `live lane held by <owner>, retrying` while it
waits). The lock is inherited by the client and held until it exits, so
manual `ss -tnp | grep 15010` polling is no longer the only guard. A
lock refusal is not a boot failure: count it as a **contention event**
in the ledger and keep the failed attempt's pack.

Boot failures still recur. Name each one the same way in every wave so
the counts can be compared across waves.

| Mode | First observable | Next step |
| --- | --- | --- |
| Channel flake / popup miss | emblem or channel click lands with no `03EA`/`03EC`, or the NORMAL-mode box is still up / never appears | probe a fresh frame at the transition, retry with a bounded count, and refuse to claim the scene until the wire cue arrives |
| Duplicate-session `1001 feffffff` | auth or channel entry answers `-1` while another lane holds the account session | select a free account index (Memoryer matches the **token**, not the client id, #726), log the wire frame, and stop; never restart Memoryer to force recovery |
| Preflight Wayland leak | `rig-run` exits 10 with `WAYLAND_DISPLAY is set` / `XDG_SESSION_TYPE=wayland` and no client boots | unset the ambient variables in the calling driver's clean env, then re-run |
| Frame-gating miss | stale frame still shows the popup, or the first grab is black before paint | wait for a painted frame and a changed frame hash before any oracle read |
| Driver / setup fault | wrong script path, missing `__main__` guard, bad arg order, wrong row formula, missing `mkdir` | fix the driver and re-run the same leg; keep the failed pack as the negative record |
| Driver crash | XTEST `BadRRModeError` or an unhandled exception mid-drive | record the traceback and display, isolate the one input call, then re-run |
| OOM kill | process SIGKILL with a journal OOM line, not a test assertion | check memory headroom and lower parallel cargo/REA load before re-running |

### Per-wave counting rule

Every wave ledger (the #382-style comment) gets a **boot failure ledger**
table with one row per mode: `mode | count | attempts | evidence`.
Count attempts, not impressions: name the boot id and the findings file
(or pack hash) for each event. A no-boot preflight refusal counts in its
mode with `attempts = 0`; a boot that hits two modes is counted once in
each, and the ledger says so. Read the lane findings *and* the rollup
comment before filling the table — they can describe the same failed
boot at different granularity; record both, never silently merge them.

After the table, cut or link a ticket for the top two modes by count.
Each ticket is born `needs-triage`, gets a triage record citing this
wave ledger, and is then promoted to a work role. If a mode already has
an open ticket, link it instead of cutting a duplicate; if a mode drops
out of the top two, leave its ticket open and count it again next wave.
The ledger names the two ticket numbers so the next wave can verify the
rule was followed.

### Seed: 2026-11-19 session waves

The #382 rollup (`gh issue view 382 --comments`, comment `7003507451`)
reports Waiting/Room as 26 LIVE-FILLED with 34 GAP and says four of six
boots failed early. The close-wr ledger (comment `7003219231`,
`~/scratch/close-wr-findings.md`) spends all six boots: stale popup
frame, row off-by-one, XTEST crash, observer FIFO timeout after CP05,
the same failure after CP06, and driver argument order. Counts below
use the named per-lane findings for the exact mode; the two views are
kept side by side, not reconciled by guesswork.

| Mode | Count | Evidence |
| --- | --- | --- |
| Channel flake / popup miss | 4 | `story-matrix-findings.md` boot 3; `story-planet-findings.md` boot 1; `story-result-findings.md` boots 2 and 4 |
| Preflight Wayland leak | 4 | `story-channel-findings.md` boot attempt 1; `story-myroom-select/exe1/phases-boot.log`; `story-shops/mshop-wire/exe/phases-boot.log`; `side-scenes/b2-dialogs/phases-boot.log` (all no-boot, exit 10) |
| Duplicate-session `1001 feffffff` | 3 | `story-matrix-findings.md` boot 4; `story-result-findings.md` boot 2 (also a channel flake); `rust-seam-331-findings.md` red-before |
| Driver / setup fault | 6 | close-wr boots 2 and 6; `story-payment-findings.md` boots 1 and 2; `story-playing-findings.md` batch 2; `story-planet-findings.md` attempt 0 |
| Frame-gating miss | 2 | close-wr boot 1 (stale popup frame); `~/scratch/close-cp-findings.md` boot 3 (black pre-paint frame) |
| OOM kill | 2 | `~/scratch/journey-wave-plan.md` (one under max-parallel load); `~/scratch/data-dup-findings.md` (first proof attempt SIGKILL) |
| Driver crash | 1 | close-wr boot 3 (XTEST `BadRRModeError`) |

Top two by count: channel flake / popup miss and preflight Wayland leak
(4 each). Their tickets are #497 (channel transition) and #498 (Wayland
env). Both were born `needs-triage`, triaged against this seeded table,
and promoted to `ready-for-agent`.

## Supervised seat sessions

A supervised seat run is owner path. A background agent never starts one,
never announces one, and never treats a probe result as a grant. The owner
calls the session, announces it, confirms the seat is free and unlocked,
and states the scope before any pointer, key, or window is touched (#242).
An agent under a no-tracker-writes rule cannot grant its own scope, so the
run stays OPEN. The seat pair qualifies procedure only; it never feeds a
budget number (see "Scope and pairing" and "Two-path architecture").

No-go conditions (stop before acting):

- The screen is locked or a screensaver is active.
- The owner has resumed seat use, or owner presence is unconfirmed.
- The grant scope is unclear: no named session, time box, or command list.
- Any containment check is red: a leaked Wayland env, default-sink audio,
  or a claim not owned by this run.

Procedure (from the dry-run in `docs/captures/t242/seat-dry-run.txt`):

1. Announce: the owner posts the session, scope, and time box; the agent
   repeats the scope and names the driver commands before touching input.
2. Unlock check: `scripts/rig-seat-probe.sh` must print `seat ready`
   (exit 0), and the pointer position must be read before the run. A
   read-only probe is not owner presence and is not a grant.
3. Act: reuse the release build, music id, input trace, and stats shape of
   the qualified native baseline. Drive through `scripts/rig-drive.sh
   --backend ydotool` on the real seat. No fullscreen takeover, no
   pattern-kill, no hand-rolled display handling; keep both path fences.
4. Restore: stop the client by its PID file, restore the pointer to the
   position read in step 2, and confirm with another pointer read. Keep
   the whole pack, including a failed one.
5. Report: a fresh manifest, input log, stats, and distinct screenshots
   in a seat pack; `run-manifest.sh verify` on both packs and with the
   cross-pair flag; `docs/captures/t242/comparison.md` filled from the two
   stats files. Copies of native pixels are not seat evidence.
6. Abort: on any no-go condition, stop by PID, restore the pointer, keep
   the partial pack, and report the gap as OPEN. A lock or owner mid-use
   needs a new announcement and grant before any relaunch.

## Audio capture (agent path, #279)

Automated runs never reach the speakers, and they no longer suppress
audio either: a rig run captures it. One module owns the lane —
`scripts/rig-audio.sh` — sourced by `scripts/rig-run.sh` (`run`/`stop`
drive it) and callable standalone for `probe` and `verify`.

### Routing

A capture run owns a per-run sink, created by the preflight and unloaded
on `stop` (`rig_null_<pid>`, recorded in `<out-dir>/claims`). Two
environment levers point clients at it, one per client family:

| Client family | Lever | Why |
|---|---|---|
| PulseAudio-protocol clients (`pacat`, winepulse) | `PULSE_SINK` | the pulse server honours it |
| ALSA clients (cpal's Linux host, so the native client) | `PIPEWIRE_NODE` | PipeWire's ALSA plugin (`libasound_module_pcm_pipewire.so`) reads `PIPEWIRE_NODE`; it does not know `PULSE_SINK` |

`rig-run.sh run` exports both into the client environment and records
them in `<out-dir>/env.sh`. Probe evidence (2026-11-16, PipeWire 1.6.11,
pulse-compat 15.0.0): `PIPEWIRE_NODE=<sink>` routed an ALSA producer to
the sink's monitor (peak −21.07 dBFS recorded from the tap);
`PULSE_SINK=<sink>` routed a `pacat` producer (−1.36 dBFS); the same ALSA
producer without the lever landed on the default sink. The native client
opens `cpal::default_host()` → ALSA default → the PipeWire plugin, so the
first lever is what keeps it off the owner's speakers.

### Tap and sidecar

`capture-start` opens the tap before the client starts, so the first cue
is in the artifact: `parec` records `<sink>.monitor` at the sink's own
specification (`float32le`, 48 kHz, stereo — the sidecar records
`resampling=none` when tap and sink agree) into
`<out-dir>/audio/audio.wav`. `capture-stop` signals the
recorder by process group (never by pattern), measures the artifact, and
finalizes the sidecar `<out-dir>/audio/manifest` — one `key=value` line
per fact: `sink`, `source`, `client`, `rate`, `channels`, `format`,
`expect`, `state`, `started_at`, `stopped_at`, `duration_s`,
`peak_dbfs`, `mean_dbfs`, `source_verified`, `route`, `sink_spec`,
`resampling`, `client_streams`, `streams_normalized`, `muted_streams`,
`on_default_sink`, `default_sink`, `run_sink_streams`,
`muted_run_sink_streams`, `client_stream_pid`, `client_stream_binary`,
`foreign_process_streams`, `baseline_streams`,
`baseline_streams_excluded`, `unattributed_default_streams`,
`probe_streams`, `probe_target`, `probe_streams_seen`, `preboot_streams`,
`preboot_breach`, `preboot_sink`, `preboot_stream_index`, `preboot_pid`,
`route_snapshot`, `route_baseline`. The route facts are written before
any breach abort, so a failed run remains auditable (#319).

### Silent-fallback guard

A wrong `-d` device name is not an error on PipeWire: `parec -d
missing.monitor` and `pw-record --target missing` both recorded the
default source (the owner's microphone or the speakers' monitor) instead
of failing — observed in the #279 probes. So `capture-start` proves the
tap before the client starts: it maps the recorder's own process id to a
`pactl source-output` and requires the source to equal `<sink>.monitor`.
A mismatch fails the run (exit 12) and reaps the recorder. Never capture
with `pw-record --target <name>`: it falls back silently.

### Mute normalisation

A remembered mute leaves the tap digitally silent while the stream looks
healthy: the first native acceptance run recorded 39 s of −91 dBFS with
`Mute: yes` on the client's stream (PipeWire stream-restore). The lane
therefore reads `Mute`/`Volume` per stream and, for streams on the run's
own sink only, sets mute off and volume 100% (`streams_normalized`,
`muted_streams` in the sidecar). A stream on any other sink is never
touched.

### Route guard (no path to the default sink)

Before the client starts, `run` records the graph baseline
(`audio/route-baseline.txt`) and applies the **pre-boot stream check**:
if an `OTwo.exe` stream whose process carries this run's `WINEPREFIX` and
`DISPLAY` is already bound to the default sink, the launch is refused with
exit 12 before the exe can open an audio device. A stale stream from
another Wine prefix is recorded but left alone. This is the deterministic
pre-boot part of the Wine routing fence (#319); it does not flip PipeWire
stream-restore, but it prevents launching into a graph that already holds
the owner's sink. The live hover319 baseline held only spotifast and Brave,
so this check would have passed.

After the client starts, `run` writes a full graph snapshot
(`audio/route-snapshot.txt`) at each route check: `pactl
get-default-sink`, `pactl list sinks short`, and `pactl list sink-inputs`,
including each stream's `application.process.id`, `application.process.binary`,
`node.name`, `application.name`, `media.name`, `Sink`, `Corked`, and
`Mute`. Both files are text provenance artifacts, never audio bytes. Pass
`--artifact audio/route-snapshot.txt --artifact audio/route-baseline.txt`
to `rig-provenance.sh emit` so they appear in the pack's provenance.

A stream is attributed to this run only when its name carries the client
basename **or** the Wine exe stream name `OTwo.exe` (#311), its process is
a descendant of the launched client **or** its process environment matches
this run's `DISPLAY`, `PULSE_SINK`, and `WINEPREFIX`, and the stream was not
present in the pre-boot baseline. Name matching alone is not ownership:
two unrelated Wine sessions can both own an `OTwo.exe` stream (#319).
Streams from other processes are counted in `foreign_process_streams` and
never normalized or treated as this run's client. An exe-named stream with
no process id on the default sink still aborts: an unattributed stream is
not assumed harmless.

Wine first opens a corked, unbound `media.name = "format test stream"`
(`Sink: 4294967295`, SPA_ID_INVALID). It is a format query, not a playback
route. The verdict labels it `probe-only`, records `probe_streams` and
`probe_target`, and leaves `client_streams` and `run_sink_streams` empty.
The guard keeps polling until a bound stream appears; `probe_streams_seen`
records that the probe was observed. This closes the hover319 gap where a
`route=foreign-sink` probe stopped the poll and hid the later playback
stream that produced 0.0 dBFS on the run sink.

A bound stream of this run on the system default sink aborts the run:
exit 12, `CONTAINMENT BREACH` on stderr, `route=default-sink-breach` in the
sidecar, and the run is stopped immediately. The abort message and sidecar
name the owning process id. The check polls for up to
`RIG_AUDIO_ROUTE_TRIES` ticks (100, so 10 s): the native client opens its
stream about 4 s into the boot, after chart and bank decode. Both
`route=unseen` and `route=probe-only` stay provisional; check
`foreign_process_streams`, `probe_streams`, and the snapshot before
claiming the graph was empty. They are **not** proof of silence; the level
sidecar (`expect`, `state`, `peak_dbfs`, `mean_dbfs`) and `rig-audio.sh
verify` judge the artifact. The hermetic `scripts/rig-audio.sh selftest`
pins `wine-exe-run-sink` (green, `route=sink`), `wine-exe-default-sink`
(red, breach + exit 12), `wine-exe-unbound-format-probe` (`probe-only`),
`wine-exe-foreign-process-id` (a leftover is not attributed),
`wine-exe-foreign-process-id-default-sink` (a leftover does not abort an
innocent run), `wine-exe-unknown-pid-default-sink` (red), and
`wine-exe-baseline-stream-excluded` (a pre-boot stream is not ours).
`scripts/rig-run.sh selftest` adds `audio-preboot-stream-breach` (exit 12
before the client is launched) and `audio-route-probe-then-playback`
(polling continues until the bound stream reaches the run sink).

### Validation bar

A recorder that ran proves nothing; a measured level does.

- `scripts/rig-audio.sh probe <wav>` prints `peak_dbfs=` and
  `mean_dbfs=`; exit 0 above the floor, 1 below it, 2 when unreadable.
- `scripts/rig-audio.sh verify <out-dir>` is the run-level verifier:
  `expect=sound` (client argv without `--no-sound`) fails when the
  artifact is missing or silent; `expect=silence` (`--no-sound`) accepts
  a silent artifact as the containment record; `expect=disabled`
  (`RIG_AUDIO_CAPTURE=0`) records an explicit opt-out; a run with no lane
  at all fails.
- The verifier call site belongs to the orchestrator:
  `scripts/run-manifest.sh verify` must run `scripts/rig-audio.sh verify
  <out-dir>` when the run declares audio (merge step, #279).

Knobs: `RIG_AUDIO_CAPTURE` (1, the default), `RIG_AUDIO_RATE` (48000),
`RIG_AUDIO_CHANNELS` (2), `RIG_AUDIO_FORMAT` (float32le),
`RIG_AUDIO_FLOOR_DBFS` (−60), `RIG_AUDIO_IDENTITY_TRIES` (40).
`RIG_AUDIO_TEST_STREAM_PIDS` and `RIG_AUDIO_SKIP_STREAM_BASELINE` are
hermetic selftest seams; never set them for a real run.

### Legs

| Leg | Sink | Tap | Status |
|---|---|---|---|
| Native (recomp2jam) | per-run `rig_null_<pid>` | `parec` monitor → wav | proven: #279 acceptance run captured non-silent game audio |
| Wine (OTwo.exe) | same sink, same tap | same tap | Proven (#240): a blob-launched boot bound to the run's own sink — `pactl` showed `OTwo.exe` on `rig_null_<pid>`, and the tap measured peak 0.0 dBFS over 90 s (`docs/captures/t240/findings.md`). The historical pack's sidecar says `route=unseen` because the guard matched only the launcher; #311 added the exe stream alias and #319 made matching run-scoped with a boot-time graph (both stub-verified here). A live re-run with `route=sink` and a non-silent tap is still required. The level sidecar, not `route=unseen`, is the silence evidence |

`--no-sound` stays available for runs that must not emit audio at all;
such a run declares `expect=silence` and its artifact is the proof that
nothing was audible. See `docs/captures/t223/01-live-cadence-method.md`
for the capture cadence.

### Containment fence for test arms (#283)

`cargo test` runs that spawn the real client must never reach the
speakers. Two fences hold, and both are guarded:

- **The spawn argument list carries `--no-sound`.** Every in-repo spawn
  helper ends its argv with the fence: `input-drive` `game_args` (the
  X11 arm), `wl-smoke-compositor` `GAME_ARGS` (the two smoke examples),
  and the rig scripts. A spawned client then boots fenced.
  Guards: `x11_game_args_never_opt_into_hardware_audio`,
  `smoke_game_args_never_opt_into_hardware_audio`.
- **A session starts fenced.** `OnlineSession::new` starts on the null
  sink; only the windowed live boot lifts the fence with `--no-sound`'s
  value (`session.set_silent_audio(opts.no_sound)`). The walk fixtures
  therefore attach banks silently, and the env-gated audible walk stays
  on the null sink. Guards:
  `live_walk_bank_attach_stays_on_the_null_sink`,
  `sink_choice_never_reaches_hardware_for_a_test_arm`.

The sink is one decision, `o2jam_audio::SinkChoice::for_session(live_player,
rig_fenced)`, and one constructor, `SinkChoice::open`. Hardware needs
both a live player and no fence. Never construct `CpalBackend` outside
that seam. Guards read the choice and the argv; they never open a
device, so a red proof stays silent.

Different lanes, different levers: a rig run that must *capture* audio
uses the per-run sink and `PIPEWIRE_NODE` (above, #279); a test arm that
must not *have* audio passes `--no-sound` and inherits the null sink.

## Negatives (do not re-prove)

- Bare Xvfb ignores kernel input devices: uinput/ydotool events never
  reach a client on a bare Xvfb display. Use XTEST for the native pair
  or a real seat for uinput.
- Keys need explicit focus: with no window manager under bare Xvfb,
  focus must be set before key delivery. The focus step is part of the
  standard sequence (#234).
- Verbosity comes from the CLI flag, never the environment: capture
  verbosity is set by client flags (`--input-log`, `--frame-dump`,
  `--no-sound`), not by environment variables such as `WINEDEBUG` or
  `RUST_LOG`.
- XTEST motion only reaches a DirectDraw canvas under Wine: clicks use
  the SendInput helper instead (`docs/wine-boot.md`,
  `docs/captures/t127/02-backends.md`).
- A bare launch of the 7.55 client stops at the launcher-gate dialog;
  no rendered scene appears without the GameStart argv shape
  (`docs/wine-boot.md`).
- A Wayland leak puts the client on the real desktop; `rig-run.sh`
  refuses a leaked `WAYLAND_DISPLAY` or `XDG_SESSION_TYPE=wayland`.
- A PipeWire recorder with a wrong device name records the default
  source instead of failing (`parec -d missing.monitor`,
  `pw-record --target missing`). Verify the recorder's source, or the
  artifact may hold the owner's microphone. (#279)

## Evidence and close discipline

Every live proof names its pair, manifest, and artifacts. Guards pin a
behavior only when they name the live caller chain from boot or state why
the seam test suffices; seam-green alone never closes a player-visible
ticket (`docs/live-path-reachability.md`,
`docs/agents/triage-labels.md` rule 7). Synthetic input guards keep
passing with no game data (#233 story 20).

## Cross references

- Parent spec: #233. Sibling kit tickets #234 (XTEST backend), #235 (rect
  query), #236 (scene probe and manifest), #237 (injector and stats),
  #239 (planet-to-waiting drive), #240 (Wine qualification), #241
  (N-200 number), #242 (live-seat qualification), #279 (audio capture
  lane).
- Backends: `docs/captures/t127/02-backends.md`.
- Wine boot: `docs/wine-boot.md`.
- Cadence capture: `docs/captures/t223/01-live-cadence-method.md`.
- Backend comparison: `docs/captures/t224/02-backend-comparison.md`.
- Run manifest example: `docs/captures/t217/runs-manifest.md`.
- Geometry aim: `docs/geometry-audit.md`.
- Input testing matrix: `docs/input-testing-matrix.md`.
- Playtest loop: `docs/playtest.md`.
- Scene, chrome, dialog, and emblem are the domain words used throughout
  (`GLOSSARY.md`).

## Two-path architecture (owner decision 2026-11-16)

The machine runs two permanently separated evidence paths. Neither
path may borrow devices from the other.

|              | Owner path (playtest)          | Agent path (rig)                          |
|--------------|--------------------------------|--------------------------------------------|
| Video        | real seat / compositor window  | private Xvfb (unique display) or Wine desktop |
| Audio        | default sink (speakers)        | dedicated virtual sink with a monitor tap  |
| Input        | real keyboard/mouse            | XTEST / SendInput / trace replay           |
| Capture      | owner eyes, ad hoc             | manifest'd artifacts only                   |
| Runs when    | owner present                  | any time, unattended (AFK)                 |

Rules (the mechanisms are enforced by the rig kit, not just documented;
#282):

- The agent path never touches the owner path: no real-seat windows,
  no default-sink audio, no ambient session env (the #267 fence;
  rig-run refuses leaked WAYLAND_DISPLAY). The owner path never feeds
  budget numbers (existing pair rule).
- Displays come from one registry: `rig-run.sh registry` lists each live
  display claim with owner PID and purpose. `rig-run.sh run-auto` and
  `rig-afk.sh` call the shared `rig_allocate_display`, which claims a
  free display atomically; a pinned collision exits 10 with a loud
  message. `rig-run.sh stop` releases that run's claims. Nothing
  pattern-kills or signals another run's X server.
- Audio is scoped before launch: `rig_check_audio_sink` asks
  `pactl get-default-sink` and refuses an explicit default-sink target
  with exit 12. `RIG_OWNER_PATH=1` is the only owner-path exception,
  used with the `attended` policy of `play-local.sh`. Scripted agents
  keep the per-run virtual sink and its monitor tap.
- The child environment is sanitized on the same surface: `rig-run.sh`
  unsets `WAYLAND_DISPLAY`, `XDG_SESSION_TYPE`, `QT_QPA_PLATFORM`, and
  the inherited `RIG_OWNER_PATH`, and writes the virtual sink into
  `<out-dir>/env.sh`, which downstream drivers source. An AFK runner strips those ambient vars at the boot
  boundary (as it strips `DISPLAY` for #344).
- The separation guard is `scripts/rig-separation-guard.sh` (also run
  by `rig-run.sh selftest`). It arms a stub owner seat (display up,
  default sink present, session env set) and proves that a leaked
  Wayland env is refused, a default-sink target fails preflight, the
  agent boot lands on a private display and virtual sink, two
  concurrent boots get different displays, and a pinned collision
  fails without killing anyone. It uses stubs only: no real display,
  sound server, or owner process is touched.

Verification (all hermetic; no game data or live seat needed):

```sh
scripts/rig-run.sh selftest          # includes the separation guard
scripts/rig-afk.sh selftest          # includes two concurrent AFK boots
scripts/rig-separation-guard.sh      # guard alone
RIG_STATE_DIR=/tmp/rig-registry-demo scripts/rig-run.sh registry
```
- Every agent run captures all three modalities it can reach — input
  (injector record + client input log), video (frame dump and/or
  framebuffer capture), audio (monitor tap of the virtual sink) — and
  every capture lands in the run out-dir with its manifest entry.
  Audio capture is a first-class lane, not an afterthought:
  comparison tooling (loudness, drift) requires both sides to carry
  audio.

## Modality gaps (2026-11-16 architecture review)

| Modality | Native pair (proven) | Wine pair (pending #240) | Gap |
|----------|----------------------|--------------------------|-----|
| Input    | XTEST + epoch-stamped injector | SendInput helper (winegcc, committed source) | canvas presses proven on three scenes (#240); the room-row join gesture is OPEN |
| Video    | frame dump + scene probe | x11grab (ffmpeg) with measured scene signatures | three scenes captured with provenance (#240); waiting/loading signatures OPEN |
| Audio    | per-run virtual sink + monitor tap, wav + sidecar, non-silence verifier (#279) | per-run sink + monitor tap (same lane, #240) | closed: the Wine leg captures audio on its own sink (peak 0.0 dBFS, `docs/captures/t240/findings.md`) |

The native audio row is closed by #279 (see "Audio capture" above); the
Wine row closes with #240 using the same sink, tap, and verifier.

## Superseded memory lessons (retro #254)

This runbook is the single home for the rig lessons below. Each memory
entry is superseded by the named section here (and its linked evidence),
not silently dropped; the owner approves each removal in the Memory UI.

| ID | Lesson | Runbook home |
|---|---|---|
| `979fb` | Virtual-framebuffer runs must not reach the speakers | Audio capture (agent path, #279); Containment fence for test arms (#283); Two-path architecture |
| `c16e4` | Wine canvas clicks need the winegcc SendInput helper, not XTEST | Negatives (do not re-prove); Wine leg (#240); `docs/wine-boot.md` |
| `a0066` | Wine dialog-up detection and popup-before-click order | Boot, drive, aim, observe (Drive); Wine leg (#240); `docs/wine-boot.md` |
| `9e297` | winedbg launch and stopped-state read discipline | Wine leg (#240); `docs/wine-boot.md` |
| `635dc` | `pgrep -f`/`pkill -f` self-match; use recorded PIDs | Commit-or-lose-it rule; Two-path architecture |
| `d5ebf` | One shared Memoryer; scenario delta needs wire and log proof | Memoryer liveness, identity, and dumps (#345) |
