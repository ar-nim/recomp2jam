# OJM (music container) — working notes (UNVERIFIED)

Status: **stub**. `o2jam-file` currently sniffs magic and rejects unknown
data with a typed error. Fill this in from installed 7.55 files.

## What we believe (to confirm)

- Container of OGG Vorbis payloads (BGM + key sounds), consistent with the
  libvorbis link and installer strings (`BGM.ojm`, `bgEffect.ojm`,
  `DialogBGM.ojm`, `Event.ojm`, `o2ma###.ojm`).
- Community lore: `M30`/`M32` magic variants, sample table + растут
  offsets. **Do not trust this yet** — verify against real files and
  record hashes.

## To record per file

- sha256, size, magic bytes + offset, header fields with offsets/widths,
  sample count, per-sample codec/length/loop, decoded PCM hash (via
  `lewton`) vs. reference.

## Parity plan

- `o2jam-file`: `parse_ojm` returns samples + metadata; round-trip test on
  synthetic fixture; golden test on real file hashes (hashes only in repo,
  files stay local).
- `o2jam-audio`: decode each sample with `lewton`; PCM equality within
  stated tolerance.

## Open questions

1. Exact magic + version field for 7.55's OJM writer?
2. Encryption/obfuscation on headers or payloads (XOR/mask)?
3. Loop points and per-note sample mapping — in OJM or OJN side?
