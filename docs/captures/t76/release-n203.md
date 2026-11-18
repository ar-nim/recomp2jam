# #241 release measurement — N=203, min-offset p99 14 ms

## Citable figure

XTEST press to judged-strike entry, release build, **N=203** scheduled lane
presses on the Playing highway (song 757, solo room, private Xvfb `:150`):

- p50: **6 ms**
- p99: **14 ms**
- max: **14 ms**
- offset: epoch base −1795003157737 ms (min-offset normalization)

Provenance travels with the figure:

- Pair: `native-xvfb-xtest`; run report `PASS` (all five self-checks green).
- Client: `recomp2jam` release, binary sha256
  `2d03345b…cad9f5d`, git commit
  `f7a62cdf11200257cde422898875fe00c98c4d21`, cargo profile `release`.
- Params: `music-id=757`, 7 lanes × 29 rounds, `interval=800ms`,
  script `planet-to-waiting.sh --target playing --music-id 757`.
- Containment: owned null sink `rig_null_2785254`, sidecar `route=sink`,
  `on_default_sink=0`, `preboot_breach=0`, monitor tap peak −3.6 dBFS.
  Nothing reached the owner's display or default sink.
- Counts: `injections=203 judged=203 lanes=7/7`. No pairing by raw event
  order; the menu Confirm is excluded from the lane series.

This run replaces the earlier `/tmp/n241d` figure (p50 4 ms, p99 5 ms,
build `3949ade`): that pack disappeared before its artifacts were committed,
so it cannot be cited. The provisional debug p99 122 ms at N=60
(`docs/captures/t76/findings.md`) remains a cautionary tale only.

## Committed artifacts

Text artifacts live in `docs/captures/t76/release-n203/`:

- `manifest.json` — pair, build, params, seed, scene sequence.
- `artifacts/input.log` — the client's frame and judged lines for this run.
- `artifacts/stats/judged.json` — 203/203 pairs, p50/p99/max, offset.
- `artifacts/scene-sequence.txt`, `run-report.md`.
- `record-hashes.txt` pins the committed text; `verification.txt` quotes the
  verifier output and containment facts; `local-pack-hashes.txt` pins the
  git-ignored local pack.

Screenshots and audio stay in the local, git-ignored run pack
(`/home/arnim/scratch/p241-n203g`). Copyrighted pixels and music are never
committed; the committed record is a hash-pinned pointer. `run-manifest.sh
verify docs/captures/t76/release-n203` checks the text record in CI, and the
full check with captures runs on the rig host:
`run-manifest.sh verify /home/arnim/scratch/p241-n203g`.

## Budget ticket #76

This figure is an input to the budget decision, not the decision itself.
Ticket #76 states that the threshold number is a human decision on rig
evidence and that the ticket stays `ready-for-human`. This leg is forbidden
from tracker writes, so nothing was posted to #76 and the ticket's boxes were
not touched. The senior or owner must post this figure with the provenance
above when they ratify the budget. This paragraph is the required "documents
why not" record for #241's third acceptance criterion.

## Run lessons (rig-relevant)

- The NORMAL-mode message box is up on this Memoryer state and blocks canvas
  clicks. `planet-to-waiting.sh` now detects its blue body from the newest
  frame and presses the documented OK face at `(400,365)` first.
- On a shared Memoryer the session's room may not be row 0. The drive reads
  the room number from the client log and clicks that staged row; a guest-
  ready click never stages the host pick.
- The pinned account-1 token can be occupied by another agent (`AlreadyConnected`).
  This run used the read-only account-2 token via `RIG_SESSION_TOKEN` and
  `RIG_SESSION_USER_ID=2`; the token is local to the run's provenance, not in
  the committed record.
- The client writes the same Playing pixels on a cadence. The AFK packager
  stages one copy per sha256 and leaves the client's `frames/` untouched, so
  the distinct-screenshot verifier stays meaningful.
- `pair-stats.py --min-offset` normalizes by the fastest paired delivery, so
  an epoch-clock base and a first-press hitch cannot distort the p99. Both
  legacy modes are unchanged.
- The machine was shared with other agents during this run; the figure is
  honest for those conditions. A quiet-machine repeat is the next evidence
  step if the 14 ms tail needs attribution.
