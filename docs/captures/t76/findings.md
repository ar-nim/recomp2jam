# Ticket 76 capture leg — judged-path latency

Status: BLOCKED (measurement not obtained).

## What was checked

- Issue #76 comments, including the owner decision record (input only, p99 end-to-end ms plus max, median of 3 runs, null sink, CI skip-cleans).
- B53 claim in `docs/claims.md` and `crates/o2jam-shell/src/input_latency.rs` (shell-added 0 ticks; queue/exe sides OPEN).
- `~/scratch/planet-verify/findings.md` (boot recipe, dialog-up detection, siclick SendInput, null sink).
- `scripts/rig-run.sh` selftest (PASS) and Memoryer on 127.0.0.1:15010 (LISTEN, pid 448111).
- `docs/captures/t223/01-live-cadence-method.md` (live wall run not executed; same rig blockers).

## Blocker

No judged-strike sampling path exists that can produce the required p99/max pairs. The exe's judgement event is internal; the winedbg protocol forbids `cont` after a stop (wow64 helper fault), so per-strike sampling at highway cadence is not safe. The repo has no judged-strike detector or logger on either side, and building one (DLL injection or pixel detector) is out of scope for this ticket ("No client code changes expected"). The planet-verify recipe covers navigation and dialog state, not judged-strike timestamps.

## Skipped

One skip: the live Wine null-sink capture pair. Sources checked above. No further skips taken; stopped rather than fabricate numbers.

## Next evidence step

Add a minimal judged-strike logger (exe side via a safe trace hook or pixel/aural detector; Rust side via the existing `InputLatencyProbe`/`JudgementStamp` wired to wall clock) before re-running this leg. Threshold number remains the owner's decision on rig evidence.
