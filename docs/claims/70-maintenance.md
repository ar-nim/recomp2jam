## Maintenance

- Edit the fragments under `docs/claims/`, never this file; regenerate
  with `cargo run -p recomp2jam --bin ledger-check -- gen claims`
  (ledger-check fails on a stale render).
- Update a row when its level changes; keep one claim per row.
- New EXE evidence for an ASSUMED row retires its risk note.
- The sibling list is the checklist for every new positional claim:
  find the explicit ordinal/id field first.
