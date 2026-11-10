# Input traces (`recomp2jam-input-v1`)

Deterministic input traces replayed **through the real window**: the
driver (`replay_window.py`) injects XTEST key/mouse events at the
running windowed binary under Xvfb — never a test-only API bypass —
and the binary's capture log (`--input-log`) records every applied
event plus one `frame` line per paint (wall ms, screen, state hash,
content hash). `--frame-dump` writes the gated PNG captures. Both
flags are documented test affordances.

## Schema

Skill base (`recomp2jam-input-replay`): `schema_version`
(`recomp2jam-input-v1`), `scenario`, `time_base` (clock name + unit),
ordered `events` with `sequence`, `time_ms`, `control`, `action`.
Timestamps are the event's intended menu-clock time (pacing minimums);
sync authority is the `await-screen` events, so server round trips
never become fixed sleeps. Applied times land in the capture log
(skill rule: intended vs applied recorded separately).

Window-harness extensions (validated by `recomp2jam validate-trace`
and mirrored in the driver):

| control | action | payload |
|---|---|---|
| `lane-0`..`lane-6` | `press` / `release` | the seven play keys (S D F Space J K L) |
| `menu-confirm` / `menu-back` / `menu-up` / `menu-down` | `press` / `release` | Enter / Escape / ArrowUp / ArrowDown |
| `click` | `press` | `x`, `y` on the native 800x600 canvas |
| `text` | `type` | `text` (ASCII printable; injected as XTEST keysyms — IME-free rigs) |
| `await-screen` | `reach` | `screen`: hold the replay until the capture log shows it |

Validation rules: strictly increasing sequence, non-decreasing time,
complete press/release pairs, click bounds inside 800x600, known
screens. A malformed trace fails before any injection.

## Running the U1 trace

```sh
# 1. local Memoryer up (127.0.0.1:15010, docs/playtest.md)
# 2. build the binary (release: the fixed clock then tracks ~1.5x
#    wall time; a debug build paints ~4x slower and needs the
#    --await-timeout bump)
cargo build -p recomp2jam --release
# 3. validate + replay (starts its own Xvfb, null audio throughout)
python3 fixtures/input-trace/replay_window.py \
  --trace fixtures/input-trace/u1-windowed-online.json \
  --token 61dcb457-5da9-4e22-af4b-5a4c5658f239 \
  --out ~/scratch/u1-trace
```

The driver preflights the gateway, starts a logging TCP proxy
(wire-opcode evidence), launches `recomp2jam online --fixed-clock 25
--frame-dump ... --input-log ...`, replays the trace, then writes
`manifest.json` (applied vs intended events, final state hash, screen
path, capture listing) into `--out`. Full captures stay in scratch —
they may contain game artwork and are never committed.

## The U1 walk

`u1-windowed-online.json` walks login (CLI args, ours-by-design — the
exe takes launcher cmdline) → planet → channel → room → waiting
(chat typed and sent as 3011) → song select (4000/4002/4030) → START
(4010) → loading (4012) → playing (lane taps through the real window)
→ result (tally 4016/4041/4021 drained) → back to waiting. Login,
room create (`--create-room "U1 Trace"`), and the host pick
(`--music-id 757`) ride CLI args; everything visible rides real
window input.
