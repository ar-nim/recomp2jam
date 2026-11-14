# shop-t209 batch — manifest + pass/fail gate (#209)

Nightly-able batch wiring the #207 captures, the #208 traces, and the
live-Memoryer leg into per-row verdicts. Runner: `scripts/shop-batch.sh`
(manifest shape: `manifest.json` in this directory).

## Manifest shape (the skill's `references/manifest-schema.md` does not exist
in this repo — this file documents the shape instead)

Per row: stable `id`, `scene`, `platform`, `build`, `input_traces[]`
(file + staging family + exact counts + final side), `guards[]` (package +
cargo-test filter, exactly one test must pass), `reference_captures[]`
(scene file + pinned sha256 + capture id + provenance), `requires`
(install-root optional / Memoryer live-only), `status_when_green`
(`passed` or `needs_review`), `known_open[]` (OPENs that stay visible).
`threshold_notes` states the rule: exact assertions where pinned,
7.55-derived tolerances where rig-measured.

## Rows

| row | legs | green verdict |
|---|---|---|
| item-shop-browse-buy | 3 traces ×2 runs + 4 guards + shop-native bytes | passed |
| music-shop-browse-buy | 3 guards + musicshop-native bytes | needs_review (DIFF OPEN, no music trace) |
| myroom-enter-equip | 2 traces ×2 runs + 5 guards + myroom-native bytes | needs_review (wire effects stub-side) |
| channel-closed-list | 3 guards + channel-native bytes | needs_review (CLOSE marking OPEN) |
| live-channel-list | opt-in `--live` only: connect → 1003 shape | interop evidence, separate section |

`needs_review` is how a KNOWN OPEN stays visible instead of silently
passing. Gate exit is 0 iff `failed == 0`; needs_review/skipped never
count as passes.

## Running

```sh
# full nightly (present data): gates + rows + record
scripts/shop-batch.sh --record-dir batches/shop-t209/reports/<stamp>
# CI shape (no data): captures skip cleanly
scripts/shop-batch.sh --hide-install-root
# interop evidence (local Memoryer up)
scripts/shop-batch.sh --live
# red proof (breaks BUYITEM commit, restores, proves recovery)
scripts/shop-batch.sh --red-proof
```

Headless only: no display claimed or touched, `PULSE_SINK=wine_null`,
Memoryer probed for `--live` rows only. PNGs stay under
`~/scratch/shop-t209/` (local-only); only `summary.md` + `report.json`
(no PNGs) are copied to `--record-dir` for the committed record.

## Red proof

`--red-proof` patches `ScreenGame::take_shop_buy` to drop the queued
5000 commit (one guarded behavior), rebuilds, runs the rows with gates
skipped by design, and requires exactly `item-shop-browse-buy=failed`
with the other three rows green. It then restores the file, rebuilds,
and re-runs the two failing guards to prove recovery. The break patch
is refused on a dirty tree (fail closed).
