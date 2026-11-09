# AGENTS.md — recomp2jam

Clean-room recompilation of the O2Jam 7.55 client
(C++/CDX/Boost/libvorbis/MSVC2005) in Rust. Re-implement behavior from
docs, never from original bytes.

Dual goal: a runnable modern-platform client **and the reference
implementation of 7.55 behavior** — the executable spec that settles
any "how did the original do X" question. Every behavior must trace to
pinned evidence (version, hash, offsets, evidence IDs) or be explicitly
labeled OPEN with the next evidence step. Where sources conflict, the
exe is the behavior oracle; the contradiction is documented, never
silently resolved.

Pick one branch:

- Code in `crates/` → crate map in `docs/architecture.md`.
- RE findings → notes in `docs/` first, then code; method in
  `docs/reversing.md`, format specs in `docs/file-formats/`,
  gameplay behavior in `docs/gameplay.md`.
- Game data → local-only under `install-root/` + `extracted/`
  (git-ignored); layout in `docs/install-layout.md`, status in
  `docs/progress.md`.
- Implementing → invoke `/skill:clean-code` and hold its 10/10:
  intent-revealing names, small functions, scenario-named tests.

Rules:

- Keep copyrighted bytes out of the repo (exe/ojm/ojn/opi/opa/artwork,
  music). Only synthetic fixtures live in `fixtures/`.
- Never paste disassembly or decompiled code. Describe behavior in your
  own words with file offsets + REA evidence IDs, then re-implement.
- Pin every RE note to version 7.55 with hashes, sizes, tool versions.
- Conventional Commits (`feat`/`fix`/`docs`/…), imperative subjects.

Done means: `cargo fmt --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` are green,
and the work is committed and pushed.
