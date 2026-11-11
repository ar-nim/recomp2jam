<!-- GENERATED from docs/claims/*.md by `ledger-check gen claims` — edit the fragments, never this file. -->
# Behavioral-claims inventory (7.55) — by evidence level

Every behavioral claim in `docs/*.md` (incl. `docs/file-formats/`) plus
doc comments in `crates/*/src/*.rs`, tagged by evidence level. Version
pin for all EXE rows: `AngelO2Jam.exe` 5,029,888 B, SHA-256
`83a04541…72c823b`; DATA rows pin the archives listed in
`docs/file-formats/opi.md`.

## Levels

Short forms live in `GLOSSARY.md`. Normative forms:

- **EXE** — anchored in the client at a file offset (VA = file +
  `0x400000`) with an evidence id: an REA dossier id where one exists,
  else a local-static record id. Rows marked *(offset-only)* cite
  offsets from objdump/byte surveys whose docs state no dossier was
  filed.
- **DATA** — byte-walk inference over shipped data files only; no
  consumer code observed.
- **ASSUMED** — player report, reference-implementation corroboration,
  positional/ordinal guess, placeholder, or documented-unknown.

**Drives** flags what a claim moves on screen or in play: `R`
(rendering), `G` (gameplay), `R+G`, `—`.
