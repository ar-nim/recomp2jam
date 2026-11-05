# Contributing to recomp2jam

Thanks for helping recompile O2Jam 7.55 in Rust.

## Ground rules

1. **Clean room only.** Never paste disassembled original code into this
   repo. Describe behavior in your own words; re-implement from the
   description.
2. **No copyrighted assets.** The original installer, installed binaries,
   `.ojm`/`.ojn`/`.ojt`, artwork, and music must never be committed.
   They are covered by `.gitignore`. Use small synthetic fixtures instead.
3. **Pin the version.** All RE notes are relative to **7.55** unless the
   doc says otherwise. Record hashes, file sizes, and tool versions.

## Workflow

- File-format work: parser in `crates/o2jam-file` + spec in
  `docs/file-formats/` + fixture + round-trip test.
- Behavior work: logic in `crates/o2jam-game` + parity test describing
  original behavior and tolerance (e.g. ±1 ms timing).
- Docs-only RE notes are welcome as PRs to `docs/`.

## Checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same three commands on every PR.

## Commit messages

Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`,
`chore:`), e.g. `feat(file): stub OJN header parser`.
