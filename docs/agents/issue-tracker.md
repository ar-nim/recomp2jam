# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues in `ar-nim/recomp2jam`
(https://github.com/ar-nim/recomp2jam). Use the `gh` CLI for all operations.

## Conventions

- **Create an issue (mandatory)**: `scripts/issue-create.sh --title "..." --body-file BODY --type Bug|Task|Feature`. It normalizes doubled type prefixes, rejects an open issue with the same normalized title (exit 3), creates one issue, and reads back number/title/type/labels. Never call bare `gh issue create` (retro #487: a resume re-ran its create step into 36 duplicates). Write the body to a file with a heredoc; never interpolate it.
- **Read an issue**: `gh issue view <number> --json number,title,body,labels,comments`.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Make an issue a sub-issue of a parent**: pass `--parent <parent>` to `scripts/issue-create.sh`, or use `gh issue edit <parent> --add-sub-issue <child>` afterwards (`gh` 2.94+). Older `gh`: `gh api --method POST repos/<owner>/<repo>/issues/<parent>/sub_issues -F sub_issue_id=<child-db-id>` (database id, as in **Blocking** below). Without sub-issues, put `Part of #<parent>` at the top of the child body.
- **Blocking**: GitHub's **native issue dependencies**, the canonical, UI-visible representation. Add an edge by passing `--blocked-by <n>,<n>` to `scripts/issue-create.sh`, or with `gh issue edit <child> --add-blocked-by <blocker>` (`gh` 2.94+). Older `gh`: `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Count open blockers**: `gh api repos/<owner>/<repo>/issues/<n> --jq .issue_dependencies_summary.blocked_by` (open blockers only; 0 means startable). `gh issue view <n> --json blockedBy` also lists closed blockers, so filter it: `--jq '[.blockedBy.nodes[] | select(.state == "OPEN")] | length'`.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: post the explanation with `gh issue comment <number> --body "..."`, then `gh issue close <number>`. On an already-closed issue, `gh issue close --comment` drops the comment and still exits 0. Playtest-class tickets follow the close discipline in `docs/agents/triage-labels.md`.

## Acceptance-criteria review before any close

Owner rule 2026-11-17. A ticket closes only when EVERY acceptance criterion
is met and ticked. An agent reporting "done" is a claim, not evidence.

When a subagent finishes a ticket, the senior verifies each criterion
himself, against the actual implementation, BEFORE merging or closing:

1. Read the ticket body and quote each criterion verbatim.
2. For each, run the command or read the code that proves it; write the
   verdict `MET`, `NOT MET`, or `OPEN` next to the criterion text with the
   command and its output. Green gates are necessary but never sufficient:
   a junior can pass every test and still leave a criterion untouched.
3. If one or more are NOT MET or OPEN, do NOT merge or close. Steer or
   respawn the junior (see `docs/agents/delegation.md`), then re-verify.
4. Post the criterion-by-criterion table in the merge/close comment so the
   owner can audit it. Tick the boxes only for MET criteria.
5. Run the close-handoff steps from `docs/agents/delegation.md` ("Verify,
   don't trust"): the fix commit passes
   `git merge-base --is-ancestor <fix-sha> <branch-under-test>`, and the main
   checkout is back on `main` at end of work (retro #175: the owner's false
   "fix failed" came from a binary built without the fix).

Historical tickets with `## Acceptance criteria` were found closed with all
boxes blank (the 2026-11-17 acceptance audit); this rule is the guard against
that failure recurring.

Infer the repo from `git remote -v`; `gh` does this automatically when run inside a clone.

## Merge protocol (multi-agent pushes)

Retro 2026-11-12: two merge mistakes cost a session — a `git add -A`
during conflict resolution swept a subagent's uncommitted work-in-progress
into a merge, and a resolved merge was committed without compiling.

1. Resolve conflicts; stage **only the conflicted paths and regenerated
   files** — never `git add -A` (the working tree may hold someone else's
   live edits).
2. Run `cargo check --workspace` **before** committing the merge.
3. Commit the merge, then run the full gates.
4. Remove implementer worktrees only **after every spawned subagent has
   reported** — check first; a live agent's worktree is not debris.

## Implementer brief template

Every subagent brief carries, at minimum:

- The worktree path and branch, with a pre-flight check that
  `git rev-parse --show-toplevel` equals the worktree — subagents never
  edit or build in the main checkout or a sibling worktree.
- "Never push; never comment on or close issues" — the orchestrator
  merges, gates, and closes.
- "`git status` shows only intended files before reporting done."
- Input-boundary routing changes (anything in the winit event arms —
  mouse edges, motion, keys, text) require a window-seam test: a
  same-file `#[cfg(test)]` in `crates/o2jam-shell/src/window.rs`
  driving the extracted arm helper (`track_motion`,
  `route_mouse_input`) against a real `ScreenGame`, no `EventLoop`
  (per #119: the suite stayed green while live mouse input died —
  every pre-#119 test drove `ScreenGame` directly, bypassing the
  seam). The live rig (`scripts/rig-run.sh` on `:99`, XTEST driver)
  confirms event-flow questions the headless test cannot reach.
- For research briefs: findings are written to disk **incrementally**
  after every experiment batch — captures without synthesis are not
  progress (gui-drive #64 post-mortem: 4 h, 371 captures, zero findings).
- One ticket per brief; a research ticket carries one leg, not several
  methods.
- Every tracker create goes through `scripts/issue-create.sh`, never bare
  `gh issue create`. A resumed implementer re-lists its own prior creates
  before any tracker write (see `docs/agents/delegation.md`).
- Behavior guards (any test named under `## Guards`) must carry a live
  caller chain from boot or a seam-suffices note per
  `docs/live-path-reachability.md` (close discipline rules 7-8 in
  `docs/agents/triage-labels.md`). Seam-green alone never closes a
  player-visible ticket.

Scoping tickets quote claim ids exactly as minted in the ledger — a
reviewer checks the ids against `docs/claims.md`.

File issues only into the user's own repo. Never file into `mattpocock/skills` (the repo these skills come from) unless the user owns it. In a fork, `gh` can default to the upstream parent, so before the first `scripts/issue-create.sh` invocation of a session, check the target with `gh repo view --json nameWithOwner` and confirm it with the user if it is not their repo.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number>`, then `gh pr view <number> --comments` (off a TTY, `--comments` prints only the comments, not the body), and `gh pr diff <number>` for the diff.
- **List external PRs for triage**: `gh api --paginate 'repos/{owner}/{repo}/pulls?state=open' --jq '.[] | select(.author_association | IN("OWNER","MEMBER","COLLABORATOR") | not) | {number, title, author: .user.login, author_association, labels: [.labels[].name]}'`.
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either: resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue through `scripts/issue-create.sh` (mandatory path).

## When a skill says "fetch the relevant ticket"

Read it as in **Read an issue** above.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. Create it with `scripts/issue-create.sh --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (see **Make an issue a sub-issue of a parent**). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: see **Blocking** under Conventions.
- **Frontier query**: list the map's open children (`gh issue list --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (see **Count open blockers**, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> --add-assignee @me`, the session's first write.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.

## Issue creation hygiene (retro 2026-11-12)

- Create issues ONE at a time with `scripts/issue-create.sh`. This is the
  MANDATED creation path; bare `gh issue create` is not a shortcut. NEVER pipe
  markdown through delimiter-split loops — a `|` in a table once shredded 20
  junk issues out of one heredoc.
- The script normalizes repeated `Bug:`/`Task:` prefixes and refuses a title
  that matches an open issue (after case/whitespace/prefix normalization).
  If it exits 3, the ticket already exists: read the reported issue instead
  of rephrasing the title to force a create.
- The script reads back number, title, and labels; diff intent vs reality on
  the spot. A fragment title or a pipe-prefixed title fails the sanity check.
- On a RESUMED session, re-list your own prior creates before any tracker
  write (`gh issue list --author @me --state all --limit 100`). A resumed
  agent re-ran its create step 36 times (#487); re-listing catches that.
- GitHub has NO CLI/API issue deletion. Mistakes: label `to-remove`, the owner
  bulk-deletes via the UI (filter by label).

## Worktree setup (no-push agent worktrees)

Agent worktrees are created **beside** the main checkout, named
`/home/arnim/Projects/recomp2jam-wt<N>-v<ticket>` on branch `cb2/tt-<ticket>`
(or the assigned branch). Four persistent data slots live at
`~/scratch/install-data/slot{1..4}/`, each with its own real `install-root/`
and `extracted/` copy. Worktrees symlink their data paths at ONE slot each;
no two concurrent worktrees resolve to the same physical dir. A shared path
would make one hidden-arm gate flake every other worktree (ec29b).

Immediately after `git worktree add`, run from inside the new worktree:

    scripts/worktree-setup.sh

This claims a free slot with an atomic `mkdir` marker (all four busy = error,
matching the max-4-agent rule), refreshes that slot from the read-only main
checkout, and symlinks `install-root/`/`extracted/` to it. Runs write through
the symlinks into their own slot (`option.ini`, Temp→Music promotion);
refresh replaces the slot contents from the source, so a previous user's
download drift never leaks. Release on the way out with
`scripts/worktree-release.sh`, which drops the
claim marker but keeps the slot data.

**Never push from an agent worktree.** The orchestrator owns the push path;
stray `cb2/*` branches confuse the merge queue. Setup installs a pre-push
hook in a worktree-only hooks dir (`core.hooksPath` in worktree config) that
refuses pushes locally. It never writes `remote.*` or `protocol.*` config:
those keys are shared by every worktree and once broke the main checkout's
push (issue #488). The hook is a reminder, not a boundary — `--no-verify`
bypasses it, so the reviewer enforces the rule. `scripts/tracker-audit.sh`
fails with exit 1 when the shared push URL drifts from the fetch URL and
lists remote-only `cb2/*` branches. `worktree-setup.sh` refuses to run in the
main checkout. See retro-queue entries "agent pushed to origin (fix-175)"
and the 2026-11-19 resolution.

## Tracker CLI recipes (working snippets)

`gh` field shapes that differ from intuition; each recipe here was burned in
once — use them verbatim (retro 2026-11-18).

- **Sub-issue link**: add via
  `id=$(gh api repos/ar-nim/recomp2jam/issues/<child> --jq .id) &&
  gh api repos/ar-nim/recomp2jam/issues/<parent>/sub_issues -X POST -F sub_issue_id=$id`.
  The REST DELETE (`.../sub_issues/<id>`) 404s on this repo — remove with
  GraphQL instead:
  `gh api graphql -f query='mutation($i:ID!,$s:ID!){ removeSubIssue(input:{issueId:$i, subIssueId:$s}){ issue{number} subIssue{number} } }' -F i=<parentNodeId> -F s=<childNodeId>`
  (node IDs from `gh issue view <n> --json id`).
- **blockedBy / blocking**: `gh issue view <n> --json blockedBy` ERRORS
  ("expected an object but got: array"); use
  `gh issue view <n> --template '{{range .blockedBy.nodes}}{{.number}} {{end}}'`.
  Edits: `gh issue edit <n> --add-blocked-by <m> / --remove-blocked-by <m>`.
- **Issue type**: `gh issue edit <n> --type Bug|Task|Feature` (the old
  `bug`/`enhancement` labels are retired; triage sets type, not a label).
