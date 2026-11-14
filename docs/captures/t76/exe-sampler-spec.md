# Ticket 76 — exe judged-path sampler spec (buildable, not built this leg)

## Gateway check

No per-strike gateway packet exists in the 400x table (`docs/net-protocol.md`):
judgement/scoring is local. 4014 UpdateGameStats and 4016 SubmitScore are
aggregate, not per strike. So Memoryer cannot timestamp individual strikes.

## Fallback: pixel detector

- Display: Xvfb 800x600, null audio sink, dialog-up dismissed first.
- Region: the judgement line on the live highway (highway bottom, center;
  exact rect from the staged ControlList, not hardcoded) plus the hit text
  that flashes on a judged strike.
- Signal: the per-strike judgement flash (hit text/number) appearing in
  that region; detect with frame diff or template NCC per frame.
- Cadence: sample every frame (~60 Hz); record the first frame where the
  flash appears after each SendInput; latency = flash frame timestamp -
  SendInput timestamp.
- Stimulus: same chart, same onset schedule, same null sink as pair B.
- Output: p99 and max over all strikes.

## Not built

The detector is a separate leg; this file is the spec only.
