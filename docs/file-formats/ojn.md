# OJN (note chart) — working notes (UNVERIFIED)

Status: **stub**. `o2jam-file` currently sniffs magic and rejects unknown
data with a typed error. Fill this in from installed 7.55 files.

## What we believe (to confirm)

- Per-song chart file paired with OJM (`o2ma###.ojn` ↔ `o2ma###.ojm` in
  installer strings). Holds metadata (title/artist/genre/BPM?) plus
  per-difficulty note streams for 7-key play.
- Community lore: fixed-size header (~300 bytes), 3 difficulty blocks.
  **Do not trust this yet** — verify field-by-field.

## To record per file

- sha256, size, header layout (offset/width/type/meaning), counts,
  time base (ticks? ms? beats?), BPM/offset handling, long-note encoding,
  per-difficulty note counts cross-checked with in-game display.

## Parity plan

- `o2jam-file`: `parse_ojn` returns `Chart`s; golden note-count + hash test.
- `o2jam-game`: load parsed chart, autoplay with null backend, assert
  deterministic max score.

## Open questions

1. Time base and BPM/offset representation in 7.55?
2. Long-note + simultaneous-press encoding?
3. Difficulty slots: fixed 3 (EZ/NM/HD) or variable?
