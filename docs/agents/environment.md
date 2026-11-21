# Environment and operational gotchas

Canonical home for the machine, shell, and tooling facts that used to live in
agent memory. Domain behavior lives in the domain docs (each section here
points there); this file is the index and the home for the rest.

Memory lessons may restate what this file and the other docs carry — that is
fine as a routing hint. The rule is no DIVERGENCE: when a doc and a lesson
disagree, the doc wins and the lesson is corrected or removed at the next
touch. Lessons are pruned for staleness, never for merely duplicating a doc.

## Shell

- The default login shell is **fish**; `bash` is installed but must be
  invoked explicitly for scripts that assume it (`docs/wine-boot.md`).

## Exit codes

- Never `cargo test ... | tail` — the pipeline's exit code is tail's, and a
  red suite reads green. Run un-piped, or use `pipefail` / `${PIPESTATUS[0]}`.

## Process management

- `pgrep -f` / `pkill -f` patterns self-match the invoking shell's own
  command line (killed our own shell repeatedly). Use a non-self-matching
  character class (`pgrep -f "recomp2jam onl[i]ne"`) or track PIDs in files
  the way `rig-run.sh` does (`docs/rig-runbook.md`).

## Git and worktrees

- Git silently clobbers git-ignored DIRECTORIES when a tracked file or
  symlink arrives with the same name (2026-11-11: a committed `extracted`
  symlink destroyed `extracted/`; recovery #101). Trailing-slash .gitignore
  matches dirs only, not symlinks. Verify worktree symlinks with
  `readlink -e` after any setup or merge touching them.
- The install-root hidden-arm shuffle (`mv install-root` aside for the
  no-data test arm) breaks every concurrent present-arm run machine-wide
  because the slots share one data dir. Only one agent runs the hidden arm
  at a time; seniors re-run before believing a single failure
  (`docs/agents/delegation.md`, parallel-lane section).

## Gate data root (issue #736)

`scripts/lib/data-root.sh` is the one runtime contract for data-dependent
arms. `O2JAM_INSTALL_ROOT` names the archive root; without it the fallback
is this checkout's `install-root`. `scripts/gate-check.sh` exports the
variable for every test arm and asserts, before the hidden arm, that both
the named root and the fallback are unreachable. A binary built in another
worktree cannot make a hidden arm pass with borrowed data. The probe seam
tests live in `scripts/tests/test_gate_data_root.py`. A dangling
`install-root` symlink is a valid no-data state, not a gate failure.
- Every git commit uses the owner identity Arif Rahman Ibrahim. Never
  override `git user.name/email` or pass `-c user.name` — including when a
  subagent commits.

## Machine budget (15 GB)

A full-suite test binary holds ~3 GB and every worktree builds its own
`target/`, so memory is the binding constraint, not cores. Rule: ONE
full-suite run at a time machine-wide, owned by the senior at merge time.
Lanes gate per ticket with `gate-check.sh --no-push --quick -- <crates>`
(the tiering lives in `docs/agents/triage-labels.md`); a lane runs the full
suite only when its acceptance names both arms, and says so. SIGKILL
(signal 9) in a test log under fleet load = this rule was violated - wait
for a quieter machine, never re-run into the OOM.

## GitHub / tracker

- Issue images: upload them (gist git-push accepts binaries; `gh gist
  create` rejects them) and embed the raw URL. Never paste a local
  filesystem path and call it attached (`docs/agents/issue-tracker.md`).

## Subagent orchestration

- Spawning batches of subagents from a script: keep names as a separate
  tuple field, never destructure a joined string (a destructured name
  became the first character; agents spawned as c/s/a/r). Print the full
  spawn result (real name + model).
- Parallel lanes touching the same file region run sequenced, or budget one
  rebase round each (`docs/adr/0008-parity-verdicts.md`).
- A textually clean merge is not a compiling one: semantic conflicts slip
  through (`docs/agents/delegation.md`, "Merge-conflict discipline" — run
  the guards, never reconstruct a side from memory).
- Every subagent brief that involves writing Rust names the clean-code and
  rust-best-practices skills with their paths and holds their bar; the
  shorthand "10/10" alone is not enough (owner rule 2026-11-20).

## Web surfaces (owner pin 2026-11-20)

The original client embeds Internet Explorer pages in some surfaces
(payment pages, the TOP100 board content, ticket/gift web views). The
original page content does not survive - no remnants exist - so these
surfaces are EXPECTED to render blank. Blank is correct, not a parity
gap: no audit, lane, or reviewer should file blank web surfaces as bugs.
A labeled stub is a later build item (parked in the tracker); until then
"the code works" is the bar - the surrounding chrome, navigation, and
wire behavior stay pinned as usual.

## Rig corrections (brief authors read this)

- Every test reading install-root skips cleanly when the archives are
  absent (CARGO_MANIFEST_DIR-anchored guard + early return), and the suite
  is verified green both with data present and hidden; CI has no game data.
- To pin what value the exe renders from a wire field, put a scratch TCP
  proxy in front of Memoryer and rewrite the one field in the reply: the
  framing is plaintext `[u16 len][u16 id][body]`, so a value-source question
  becomes a pixel read. (Recipe lived in ~/scratch/digits-rig/proxy.py -
  recreate from this section.)
- Research lanes write findings to disk INCREMENTALLY after every
  experiment batch, and a killed lane's work is continued from an inventory
  of its worktree and packs, never redone blind
  (`docs/agents/delegation.md`, "Resume discipline").

- NEVER blind-Escape a booting client (W6 finding 2026-11-21): bare
  Escape froze the Wine desktop mid-boot; the NORMAL-mode popup needs the
  popup-aware OK at its pinned face - verify the popup state from a frame
  before any key.
- There is **no** `scripts/playwright.py`. The blessed shot kit is
  `scripts/rig-run.sh` + `scripts/rig-wine.sh` + `scripts/wine-scene.py`.
- The channel doc is `docs/parity/channel.md` (not `docs/specs/`).
- Boot lanes hold the flock only around active boots, never via long keeper
  shells (`scripts/lib/rig-live-lock.sh`).

## Static analysis (REA)

- The REA MCP works once the owner enables the `rea` server in Pi Pocket's
  MCP page. The senior opens `OTwo.exe` via `open_binary` (pinned sha256
  `83a04541...72c823b`); subagents lack `open_binary`, so with a target open
  they may use any rea MCP tool, and with no target they use the REA 6.9.0
  CLI (Ghidra 12.2).

## Desktop repro notes

- The owner desktop is KDE Plasma on Wayland (KWin, Arch). Upstream winit
  Wayland bugs verified on GNOME/mutter may reproduce differently — always
  name the compositor in repro notes.
