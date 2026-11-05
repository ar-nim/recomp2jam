# AGENTS.md — recomp2jam

Clean-room recompilation of the O2Jam 7.55 client
(C++/CDX/Boost/libvorbis/MSVC2005) in Rust. Re-implement behavior from
docs, never from original bytes.

Pick one branch:

- Code in `crates/` → crate map in `docs/architecture.md`.
- RE findings → notes in `docs/` first, then code; method in
  `docs/reversing.md`, format specs in `docs/file-formats/`.
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
