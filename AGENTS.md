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
  `docs/progress.md`. Fresh worktree? Run `scripts/worktree-setup.sh`:
  it claims one of four persistent slots in `~/scratch/install-data`,
  refreshes it from the main checkout, and symlinks `install-root/` and
  `extracted/` to that slot. No two worktrees share a data dir, so a hidden
  arm cannot flake another run; release with `scripts/worktree-release.sh`.
  Archives resolve from the build dir, so without them the client silently
  paints fallbacks (guard: `scripts/verify-shots.sh`).
- Rig procedure → `docs/rig-runbook.md` (single home for rig lore).
- Implementing → invoke `/skill:clean-code` and hold its 10/10:
  intent-revealing names, small functions, scenario-named tests.

Rules:

- Keep copyrighted bytes out of the repo (exe/ojm/ojn/opi/opa/artwork,
  music). Only synthetic fixtures live in `fixtures/`.
- Never paste disassembly or decompiled code. Describe behavior in your
  own words with file offsets + REA evidence IDs, then re-implement.
- Tests = features: a guard pins a behavior only when it names the live
  caller chain from boot that reaches the guarded code, or states why the
  seam test suffices; seam-green alone never closes a player-visible
  ticket. Authoring template + tier ladder: `docs/live-path-reachability.md`;
  close discipline: `docs/agents/triage-labels.md` (rules 7-8).
- Pin every RE note to version 7.55 with hashes, sizes, tool versions.
- Coverage priority (owner 2026-11-25): pin the exe with static
  analysis FIRST — the goal is 100% of the game-reachable function
  inventory pinned (baseline `docs/exe-function-coverage.md`, 1,129
  procedures). The rig is a verification method, not the primary
  loop: a re-pair below 0.99 means the statics are INCOMPLETE, so the
  next step is always more statics, never more rig fishing. Burn-down
  order follows the backlog triage on #916 (Room/Waiting →
  Planet/Channel → Playing/Result → shops/MyRoom → callers of net
  handlers → support layers; shared containers only on demand).
- Conventional Commits (`feat`/`fix`/`docs`/…), imperative subjects.

Done means: `cargo fmt --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, and `cargo test --workspace` are green,
and the work is committed and pushed.

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues for `ar-nim/recomp2jam` (via the `gh` CLI). All issue creation goes through `scripts/issue-create.sh` (dedup + read-back); bare `gh issue create` is not allowed. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` plus `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### Delegation

Senior/junior delegation playbook (skip bound, steer-vs-respawn, show-two-then-delegate, thinking escalation, verify-don't-trust). See `docs/agents/delegation.md`.
