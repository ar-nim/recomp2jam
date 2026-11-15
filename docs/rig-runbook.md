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
| Live seat + ydotool | ydotool (kernel uinput) | real compositor seat | Manual spot-check only; never feeds numbers | procedure proven, live execution needs a supervised unlocked session (#242, ready-for-human) |
| Wine X11 + SendInput | winegcc SendInput helper | Wine under Xvfb | Specified | proof pending (#240) |
| Synthetic compositor fixture | `wl_pointer` injection | headless compositor | Protocol-only | CI seam only, never a budget pair (#126, #127) |

Xvfb + XTEST is the blessed baseline. No pair feeds a budget number
before its same-run A/B qualification against a blessed pair
(`docs/captures/t127/02-backends.md`).

## Standard run

One manifest per run records `pair`, `build`, `params`, `script`, and
`seed`. One artifact layout per run holds the manifest copy, the input
log, screenshots, and stats. A verifier rejects incomplete sets (#236).

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
outputs (input log, frame dump) stay on the paths the client wrote them.
`scripts/rig-run.sh` records every PID it started in `<out-dir>/pids`
and every claim it owns in `<out-dir>/claims`; it never pattern-kills,
and cleanup failures keep the evidence instead of deleting it.

## Boot, drive, aim, observe

- Boot: `scripts/rig-run.sh run <display> <out-dir> -- <binary> [args...]`
  runs the shared preflight (display, audio sink, Memoryer, install-root,
  relay ports, Wine prefix), starts Xvfb when absent, launches the
  binary, and writes `env.sh`. Stop with `scripts/rig-run.sh stop
  <out-dir>`; status with `status`. Wayland mode uses
  `scripts/rig-run.sh wayland <socket> <out-dir> -- <binary>`.
- Drive: `scripts/rig-drive.sh` (ydotool) for move, click, and key;
  `dry-run` prints planned commands without touching input. The XTEST
  backend (#234) drives a native Xvfb client through the same interface.
- Aim: chrome coords resolve from staged BND rects, never hand-read
  pixels. Native aim uses the dump tool's rect query for a BND control
  id (#235); unknown ids fail loudly. Wine aim cites pinned reference
  evidence for exe-side targets (#240). See `docs/geometry-audit.md`.
- Observe: the scene probe reports the current scene as text from the
  capture log, so navigation costs no image reads (#236). Judged lines
  come from the client input log (`--input-log`); frames from
  `--frame-dump`; audio stays on the null sink (`--no-sound`).

## Audio containment

Automated runs never reach the speakers. `scripts/rig-run.sh` enforces
a null sink (`wine_null`) in preflight and records the choice; set
`PULSE_SINK` only to another explicitly chosen sink. See
`docs/captures/t223/01-live-cadence-method.md` for the capture cadence
and `--no-sound`.

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

## Evidence and close discipline

Every live proof names its pair, manifest, and artifacts. Guards pin a
behavior only when they name the live caller chain from boot or state why
the seam test suffices; seam-green alone never closes a player-visible
ticket (`docs/live-path-reachability.md`,
`docs/agents/triage-labels.md` rules 7-8). Synthetic input guards keep
passing with no game data (#233 story 20).

## Cross references

- Parent spec: #233. Sibling kit tickets #234 (XTEST backend), #235 (rect
  query), #236 (scene probe and manifest), #237 (injector and stats),
  #239 (planet-to-waiting drive), #240 (Wine qualification), #241
  (N-200 number), #242 (live-seat qualification).
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
