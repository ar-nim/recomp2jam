# Pair-2 judged-distribution comparison — NOT RUN

## Spec rule

`#233` Testing Decisions: "Each pair's qualification is itself a test:
identical scripted input through both pairs with overlapping judged
distributions, committed as artifacts." The trigger is a live seat run on an
unlocked seat with the owner present; the live seat never feeds a budget
number (`docs/rig-runbook.md`, pair table).

## Baseline (native, qualified)

Pointer to the committed record from #241, not a copy of its captures:

- `docs/captures/t76/release-n203.md`
- `docs/captures/t76/release-n203/manifest.json` — sha256
  `bd295a41a4dd0ac063b5fe571bc2b689f8a8e163f14f67653ca559ca05ab8022`
- `docs/captures/t76/release-n203/artifacts/stats/judged.json` — sha256
  `f2663e7e52f5dd8a4b2dd356a1b26c09b34f4ef1e24e84de7e297d4d824f6eb9`
- Pair `native-xvfb-xtest`, release `f7a62cdf…`, N=203, 7 lanes × 29 rounds,
  800 ms interval, min-offset p50 6 ms, p99 14 ms, max 14 ms.
- Containment: owned sink, `route=sink`, `on_default_sink=0`,
  `preboot_breach=0`.

## Candidate (live seat)

NOT RUN. No seat manifest, input log, stats, or screenshots exist for this
ticket. The only seat evidence committed here is the no-pointer-moved
dry-run in `seat-dry-run.txt`. No values are copied from the native pack
into a seat row; the earlier rejected #242 branch did exactly that and was
retracted.

When the supervised run happens, fill this table from the two packs' own
`artifacts/stats/judged.json` files:

| Field | Native baseline | Live seat |
| --- | --- | --- |
| Pair | `native-xvfb-xtest` | OPEN |
| Build / profile | `f7a62cdf…` release | OPEN |
| N / lanes | 203 / 7 | OPEN |
| Trace (lanes × rounds, interval) | 7 × 29, 800 ms | OPEN |
| p50 / p99 / max (min-offset) | 6 / 14 / 14 ms | OPEN |
| Manifest sha256 | pinned above | OPEN |
| Verdict | baseline | OPEN |

The pair qualifies only when the stimulus is the same (lane counts and
interval), the counts match, and the two judged distributions overlap.
A reviewer reads the overlap from the two paired epoch distributions; this
document invents no numeric tolerance and reports no figure until the seat
pack exists.

## Cross-pair screenshot check

Command for the supervised session, after both local packs exist:

```sh
scripts/run-manifest.sh verify /home/arnim/scratch/p241-n203g \
  /path/to/supervised-seat-pack
```

It must print both `artifact set complete` and no `CROSS-PAIR duplicate
screenshots`. Copyrighted pixels stay in git-ignored local packs; only this
text record and hash pins are committed. Running this check today is not
possible: there is no seat pack. A green single-pack verify on the native
baseline is not a substitute.
