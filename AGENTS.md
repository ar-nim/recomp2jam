# AGENTS.md — recomp2jam

This is a clean-room recompilation of the O2Jam 7.55 client in Rust.

- Original: C++ / MSVC 2005, CDX game library, Boost ~1.35/1.37, libvorbis.
- Target: Rust workspace in `crates/` (`o2jam-file`, `o2jam-audio`,
  `o2jam-game`, `recomp2jam`, `o2jam-dump`).
- Never commit copyrighted material: original `.exe`, `.ojm`, `.ojn`,
  `.ojt`, artwork, music. Only synthetic fixtures under `fixtures/`.
- Never paste disassembled/decompiled original code. Describe behavior in
  your own words, then re-implement.
- All reverse-engineering notes are relative to version 7.55. Record
  hashes/sizes/tool versions with every finding.
- Before finishing work: `cargo fmt --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`.
