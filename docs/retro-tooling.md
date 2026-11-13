# Retro tooling: how to queue items for retrospective

Research for #183. Question: how should this repo queue retro items with
GitHub only (no Jira), and is another tool needed?

## Recommendation

**No new tool. Keep `docs/retro-queue.md` as the single capture inbox.**
Promote an item to a GitHub Issue only when a retro accepts it as
follow-up work. Raw captures never enter the tracker.

Concretely:

1. **Capture** — anyone (owner, senior, subagent) appends a short entry to
   `docs/retro-queue.md` during work. No `gh` call, no labels, no triage.
   Entry template:
   - `## <date> — <one-line what happened> (<linked issue>)`
   - What happened (2–4 lines, with evidence: hashes, issue numbers).
   - Effect (who got stuck, what it cost).
   - Open questions / process-fix candidate.
2. **Recall** — the retro skill reads the file top to bottom (the existing
   memory pointer already routes the owner there).
3. **Dispose** — each retro archives consumed entries into the dated retro
   note and clears the queue, so the inbox never grows stale. Rejected
   items are archived in place with the reason.
4. **Promote** — accepted items become normal tracked Issues (type
   Task/Bug, standard triage, owner, `## Guards` where behavior fixes are
   involved). The file's git history is the audit trail.
5. **Non-goals** — no `retro:` label, no Discussions category, no Projects
   board for the queue, no external retro tool.

## Why (against the five criteria)

- **Capture cost during work.** Appending ~5 lines of markdown in the
  current worktree is the cheapest possible capture: no context switch to
  the tracker, no title/body/labels ceremony, works offline, and any agent
  can do it without `gh` write permissions. An Issue per capture costs a
  `gh issue create` plus the repo's one-at-a-time hygiene
  (`scripts/issue-create.sh`, after the `|`-split incident that shredded
  20 junk issues, #143–162).
- **Recall at retro time.** One file, read top to bottom, beats querying a
  label across the tracker — and the routing already exists (memory
  pointer into the retro skill). GitHub's own guidance puts open-ended,
  not-yet-actionable conversation outside Issues: Discussions "often do
  not result in an actionable task" and "often don't have a clear owner"
  ([Communicating on GitHub](https://docs.github.com/en/get-started/using-github/communicating-on-github)).
  A parked retro observation is exactly that: not yet actionable.
- **Searchability.** Both surfaces are searchable (`grep`/git log for the
  file; GitHub search for Issues). The file wins on proximity: the
  evidence (hashes, guard names, issue numbers) sits next to the code and
  is reviewable in the same diff as the work. Accepted follow-ups live in
  Issues, where GitHub search and cross-references apply.
- **Ownership / follow-up.** This is the file's real weakness: markdown
  has no assignee, no state, and can rot. The promotion rule fixes it at
  the right stage — ownership attaches when the retro accepts the item,
  via a normal Issue with an owner and close discipline, instead of
  taxing every half-formed observation with the triage flow (every new
  issue enters `needs-triage` per `docs/agents/triage-labels.md`). The
  clearing rule (archive + empty the queue each retro) bounds the rot.
- **Tool sprawl.** The repo already runs on Issues + labels + scripts with
  documented conventions (`docs/agents/issue-tracker.md`). Every new
  surface is a second inbox agents must learn, query, and keep
  consistent. The current queue has one entry; a second system for one
  entry is pure overhead.

## What was ruled out

- **An Issue per capture (e.g. `retro:` label).** GitHub Issues are for
  "bug reports, planned improvements and feedback" with "a clear owner"
  — GitHub calls Issues its "bug-tracking system"
  ([Communicating on GitHub](https://docs.github.com/en/get-started/using-github/communicating-on-github);
  [About issues](https://docs.github.com/en/issues/tracking-your-work-with-issues/learning-about-issues/about-issues):
  "plan, discuss, and track work", assignable, closeable). A parked
  retro note is none of those yet. Each one would also pay the triage tax
  (`needs-triage` on entry, type + role decision) and re-open the
  junk-issue risk the repo just cauterized. A `retro:` label with no
  state machine would rot the same way a file can, but scattered across
  the tracker.
- **GitHub Discussions.** Designed for "open-form ideas and discussions
  where collaboration is important" and "community knowledge base"
  building across a community — this repo has one maintainer plus
  delegated agents, no community forum to run. Worse for agents: `gh
  discussion` is in **preview and subject to change** (`gh discussion
  --help`), so automation against it is unstable next to the stable `gh
  issue` CRUD. Adds a second searchable surface for no gain.
- **GitHub Projects as the queue.** Projects is "an adaptable table,
  board, and roadmap that integrates with your issues and pull requests
  … to plan and track work"
  ([About Projects](https://docs.github.com/en/issues/planning-and-tracking-with-projects/learning-about-projects/about-projects)).
  It tracks Issues that already exist; it is not a capture inbox. It also
  costs extra setup (views, fields) and a wider token scope (`gh project`
  requires the `project` scope — friction for AFK-delegated agents).
  Revisit only if accepted retro follow-ups ever need their own board.
- **External retro tools (Retrium, EasyRetro, Parabol, etc.).** Accounts,
  auth, cost, data outside the repo, unreachable from agents and from
  `git log`, and built for multi-human teams running live retro meetings.
  This team is one AFK owner + agents with an async file-based retro.
  Maximum sprawl, zero fit.

## Doubts and open points (flagged, not hidden)

- **Sample size is one.** The queue holds a single entry (the #183
  evidence-hash typo). This recommendation optimizes for the observed
  rate (~1 item per retro). If capture ever exceeds ~10 open items, the
  file wants sections or the promotion threshold wants lowering — revisit
  then, not now.
- **Concurrent appends.** Two subagents appending to the same file region
  in one worktree can conflict; keep appends sequenced or let the senior
  own the edit. Not verified under load.
- **Rot if retros slip.** The clearing rule only works if retros happen.
  Backstop is the existing memory pointer (owner reads the file when the
  retro skill runs); if a retro is skipped, the file grows but nothing is
  lost — unlike dropped chat messages.
- **Discussions-enabled state of the repo not verified** (no GitHub reads
  performed from this worktree). Irrelevant to the recommendation, since
  Discussions is ruled out regardless.

## Sources

All claims above trace to first-party GitHub documentation and the repo
itself:

- GitHub, "Communicating on GitHub" (Issues vs Discussions guidance):
  <https://docs.github.com/en/get-started/using-github/communicating-on-github>
- GitHub, "About issues":
  <https://docs.github.com/en/issues/tracking-your-work-with-issues/learning-about-issues/about-issues>
- GitHub, "About discussions" (opt-in per repo, categories, labels):
  <https://docs.github.com/en/discussions/collaborating-with-your-community-using-discussions/about-discussions>
- GitHub, "About Projects":
  <https://docs.github.com/en/issues/planning-and-tracking-with-projects/learning-about-projects/about-projects>
- GitHub, "Managing labels":
  <https://docs.github.com/en/issues/using-labels-and-milestones-to-track-work/managing-labels>
- `gh issue --help`, `gh discussion --help` (preview status),
  `gh project --help` (token scope) — GitHub CLI, observed 2026-11-13.
- Repo: `docs/retro-queue.md`, `docs/agents/issue-tracker.md`,
  `docs/agents/triage-labels.md`, `scripts/issue-create.sh`.
