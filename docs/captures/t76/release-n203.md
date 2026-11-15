# #241 release measurement (ticket #76 input): N=203, p99 5 ms

## Number

XTEST key press to judged-strike entry, N=203 lane presses,
Playing highway, solo room, song 757, **release** build, Xvfb :95:

- min: 0 ms (by construction)
- p50: 4 ms
- p90: 5 ms
- p99: 5 ms
- max: 5 ms

Method: min-offset (offset = luckiest delivery; deltas relative to it).
Provenance: build `3949ade` release, pair native-xvfb-xtest,
2026-11-15, params `release N=203 interval=800ms music=757`,
manifest `/tmp/n241d/manifest.json` (verified complete).

## What this supersedes

The debug-build provisional (pair-b-rig.md: p99 122 ms, N=60) was
dominated by the debug 9 fps render loop (114 ms frames). Release
renders full rate; transit plus quantization is 4-5 ms. Do not quote
the debug number anywhere except as a cautionary tale.

## Run lessons (rig-relevant)

- The drive must pass a music id (`--music-id 757`): without a staged
  pick the chart never loads, `tally_speed` finds nothing, 4012 never
  sends, and Loading stalls past the server ready-timeout.
- Enter must land within the server ready window after START: the
  drive script now sends it seconds after Loading appears.
- Pairing must exclude the Loading-confirm Enter key-down from the
  lane series, or every pair shifts by one (seen: one 19 s phantom
  delta). Pair by lane-key order, not raw event order.
- `pair-stats --relative` normalizes by the FIRST pair and can print
  negatives when the first delivery hitched; min-offset keeps all
  deltas non-negative. OPEN: add a min-offset mode to the stats script.
