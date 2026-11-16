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

## Commit-or-lose-it rule

No load-bearing script lives in /tmp: helpers land in `scripts/` or
their full reconstruction recipe is pinned in the runbook. Two builders
died in /tmp in one night (exe auth-blob builder, wine host-room); both
recipes survive in `docs/wine-boot.md`, and the wine host script is
committed. (#264)
`scripts/rig-run.sh` records every PID it started in `<out-dir>/pids`
and every claim it owns in `<out-dir>/claims`; it never pattern-kills,
and cleanup failures keep the evidence instead of deleting it.

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
- Displays: Xvfb is the workbench. Agent rig work always runs headless
  and invisible; nothing ever lands on the live seat unless the owner
  needs eyes on it. A seat run means a normal window the owner can
  see, announced first, pointer restored after, and never a
  fullscreen takeover (owner rule 2026-11-16).
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

Rules:

- The agent path never touches the owner path: no real-seat windows,
  no default-sink audio, no ambient session env (the #267 fence;
  rig-run refuses leaked WAYLAND_DISPLAY). The owner path never feeds
  budget numbers (existing pair rule).
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
| Input    | XTEST + epoch-stamped injector | SendInput helper + traces | Wine pair proof pending; exe launch unblocked by the #265 blob builder |
| Video    | frame dump + scene probe | framebuffer capture | provenance emitted by `scripts/rig-provenance.sh` (#281) |
| Audio    | suppressed (--no-sound, null sink) | suppressed (Null driver) | no capture path: need virtual sink + monitor tap on both legs, non-silence validation, wav artifacts |

The audio row is the architectural hole: containment exists, capture
does not. Fixing it is the prerequisite for any audio parity claim
(A/V compare, loudness, drift).
