# Scene parity checklist — 7.55 client vs `o2jam-shell`

Reference client stays read-only (`AngelO2Jam.exe`, 5,029,888 B,
SHA-256 `83a04541…72c823b`). Every 7.55 claim below traces to
`docs/claims.md` (the oracle); anything else is marked **OPEN**.
Placeholder shell art is a known mismatch, marked **FAIL** — never a pass.

Verdicts per `GLOSSARY.md`: **PASS** = exe evidence + implementation + test.
**FAIL** = known gap. **OPEN** = untraced or unclaimed with a next step.

Harness: `crates/o2jam-shell/tests/scenes.rs` asserts art-backed
composition per scene (a staged figure behind the live UI: figure
ink present, UI ink on top, frame never one flat fill) plus the flat
fallback (non-uniform card-over-background scenes, per-scene
markers) and the full transition matrix plus input/timing edges
through the public API (`ScreenGame` incl. `load_scene`/`load_splash`,
`paint_screen`, `ScreenFlow`, `can_enter`) only. Staged figures stay
synthetic in the repo (real `.ojs`/`.bnd` bytes stay out), but the
local-only real-art soak now runs: `cargo test -p recomp2jam shots`
stages genuine `Interface1`/`Playing1` art from git-ignored
`install-root` (10/10 green, zero skips, 2026-11-07), and the six
PNGs below are genuine-art frames. Composition cells stay FAIL where
invented placement remains (cited per cell), never for lack of art.
