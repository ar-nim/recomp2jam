# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual strings used in this repo's issue tracker.

**Category = GitHub issue TYPE, not a label** (owner decision 2026-11-12): `Bug`,
`Task`, `Feature` (`gh issue edit <n> --type Bug`). The old `bug`/`enhancement`
labels are retired — never recreate them. Labels carry everything else: the
state role, the component, claims, and the vocabularies below.

| Role in mattpocock/skills | Where it lives here                        | Meaning                                  |
| -------------------------- | ------------------------------------------ | ---------------------------------------- |
| category `bug`             | issue type **Bug**                         | Something is broken                      |
| category `enhancement`     | issue type **Feature**                     | New feature or improvement               |
| (tracking / multi-issue)   | issue type **Task**                        | Programs, maps, multi-issue tracks       |
| `needs-triage`             | label `needs-triage`                       | Maintainer needs to evaluate this issue  |
| `needs-info`               | label `needs-info`                         | Waiting on reporter for more information |
| `ready-for-agent`          | label `ready-for-agent`                    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | label `ready-for-human`                    | Requires human implementation            |
| `wontfix`                  | label `wontfix`                            | Will not be actioned                     |

Component labels name the owning crate or surface: `component:shell`,
`component:net`, `component:audio`, `component:game`, `component:render`,
`component:file`, `component:testing`, `component:docs`, plus scene components
(`component:room`, `component:planet`, `component:waiting`, `component:channel`).
Add a component label at triage time; add a claim label when a ticket is
claim-specific.

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string or issue type from the table above.

## Flag labels (orthogonal to roles)

`needs-owner`, `hand-playtest`, and `question` are flags, not state
roles: they ride alongside the one state role and never replace it.

- `needs-owner`: the only remaining leg needs the owner's own hands
  or eyes (live-confirm on their rig, a screenshot of a symptom only
  they can see, a rig-eyes verdict). Agents cannot advance it: do not
  spawn, steer, or respawn on it while the flag stands. Apply with a
  comment naming exactly what is needed from the owner; remove the
  moment the owner responds (back to `ready-for-agent` for the next
  agent leg, or close). It never survives on a closed issue (owner
  decision 2026-11-15: owner hand/eye legs are marked, never polled).
- `hand-playtest`: close-discipline marker (rule 1 below); the ticket
  closes only on live evidence.
- `question`: the tracker holds a question for the owner; answer it
  and close, or convert it to work.

## Triage flow

Every issue enters with `needs-triage` — no exception for the reporter or
for what the issue was cut from. The flow is always the same: issue created
→ triage → work. After a work round, the issue moves to whatever the
evidence supports — another work round, `needs-info` (waiting on the
reporter or owner), `needs-owner`, `ready-for-human`, `wontfix`, or close —
there is no implied promise to loop until fixed. The triage step assigns the
category
(issue **type**: Bug, Task, or Feature) and the role (`ready-for-agent`,
`ready-for-human`, `needs-info`, `wontfix`), adds the component label, and
sizes the acceptance. Triage may be fast — one pass may conclude the issue
is already fully specified (its sizing may live in a parent record) and
promote it straight to `ready-for-agent` — but the triage step itself is
never skipped: an issue is never born pre-labeled into a work role (owner
decision 2026-11-16). An issue that is not yet triaged is not worked, however
small it looks.

Owner decision 2026-11-16: an exe behavior mismatch is a Bug, not a
question and not a feature request. Where the exe is the behavior
oracle and our client diverges, file it as Bug with the exe evidence
required in acceptance.

Two recorded exceptions, both with the triage record visible on the tracker:

- Wayfinder map tickets carry `wayfinder:` labels and are triaged by the map
  itself (charting is the triage act).
- Execution issues cut by a scoping ticket still enter with `needs-triage`;
  the triage pass promotes them in one step, citing the scoping ticket's
  resolution comment as the triage record (which stays linked).
- When the only remaining leg is owner hands or eyes, apply `needs-owner`
  alongside the state role (never instead of it) with a comment naming
  the exact ask, and stop delegating until the owner responds.

## Spec tickets (owner decision 2026-11-16)

A SPEC is an issue labeled `spec` carrying a story checklist for one game
state or one cross-cutting domain. Rules:

- Specs stay OPEN while any story is uncovered; a spec closes only when
  every story box is checked AND each has a covering ticket closure.
- A checked box requires the covering ticket to close the SAME claim, not
  a neighboring one. Read the covering ticket's own closing record: if it
  leaves the claim OPEN or invalidates its own coverage, the box stays
  unchecked with a RIG annotation naming that ticket and the next evidence
  step. Retro #312: #278 closed the music-select control symptom while
  leaving the exe host/peer display rule OPEN; #57's own final comment says
  the overlap was ROOM-side, so it cannot cover Waiting placements.
  Keep those covering tickets closed — an invalidated cover is a spec
  evidence gap, not a reason to reopen the execution work.
- Execution tickets map to their spec as GitHub sub-issues; the spec
  body holds the story list, comments hold the audit trail.
- One spec per state (the STAGE_TABLE states plus Channel and Loading);
  cross-cutting domains (net protocol, file formats, avatar system,
  audio, rig) carry their own specs. Dialogs are stories inside their
  host state spec, not separate specs.
- Owner decision 2026-11-17: a dialog system that spans several hosts
  or carries its own wire district is a cross-cutting concern and MAY
  own a separate spec. Applied to #314 Options, #315 Messenger and
  #316 app-wide utility dialogs. Host-specific dialogs stay inside
  their host state spec. A single new seam — the host dispatch
  boundary — serves the dialog specs; guards drive it with no display.
- The exe state census lives in `docs/exe-state-inventory.md`: 16
  ControlList state headers, one Playing block in `Playing1.opi`, a
  loading overlay, and 27 dialog blocks. Dead legacy stems
  (`StateRoom`, Bulletin, MyCapsule, Top100) get no spec (ADR-0012).

## Gate tiers (owner 2026-11-20)

`scripts/gate-check.sh` is tiered so the expensive suite runs where it
catches things, not everywhere:

- **Per ticket (lanes):** `gate-check.sh --no-push --quick [-- <crates>]` —
  fmt + clippy + the affected crates' tests. Seconds, run after every commit.
- **Per merge (senior):** full mode (default) — the whole workspace suite,
  present arm, plus the hidden install-root arm. This is the tier that
  catches semantic-merge breaks and missing hidden-arm skips; it is
  mandatory before every push. The 515-chart sweeps are excluded from the
  suite; a hash-pinned dozen-chart sample is checked every merge, and the
  full pack sweep runs when `.gate-full-sweep-stamp` is absent or older
  than seven days (issue #737; no scheduler).
- **Hidden arm on demand:** `--hidden` when a diff touches data-dependent
  guards but the run is not a merge; the hidden arm also runs nightly.

A green quick gate never substitutes for the merge gate (rule 2 below).

## Close discipline

Rules for closing playtest-class tickets (retro 2026-11-11, issues #26/#30/#53–#56):

1. A hand-playtest ticket closes only with live `:99` rig evidence: frame/content-hash
   before/after plus the wall run. Seam guards alone never close one.
2. Only the merge/close step closes a ticket, after review and gates. Implementer
   branches never close. The closer first verifies EVERY acceptance criterion
   against the implementation and records `MET` / `NOT MET` / `OPEN` per
   criterion with its command and output (`docs/agents/issue-tracker.md`,
   "Acceptance-criteria review before any close"). Any NOT MET or OPEN blocks
   the close; green gates alone never tick a box. The close-handoff steps in
   `docs/agents/delegation.md` ("Verify, don't trust") are mandatory too.
3. File tracker issues sequentially through `scripts/issue-create.sh` (the
   mandated creation path) and read back the title per number; never run
   parallel creates or assume their output order (#487).
4. Closing includes triage-label cleanup: `needs-triage` and `needs-info` never
   survive on a closed issue, and the category (issue type) is set at triage
   time, not left for the close.
5. Every behavior-fix closing comment carries a `## Guards` heading listing the
   exact test names that pin the fix (one per line). Historical closures without
   the heading are exempt — a missing heading is never a failure — but any guard
   name listed under the heading must exist in `crates/` (`scripts/tracker-audit.sh`
   enforces this).

   Red-proof: a guard name closes a fix only when the closing report carries
   that guard's PASS line and its FAIL line — break the fix, watch it fail,
   restore the fix, watch it pass — and the reviewer re-ran both. A guard with
   no FAIL line is hollow and does not close the fix (precedent: 4 of 19
   backfilled guards had to be strengthened after the fact).

   Exemption: purely presentational or docs-only fixes carry no real guard;
   use `(none -- <class>: <why>)` in place of a guard name, where `<class>`
   is `docs` or `presentational`. Never fabricate a test name for such a fix.

   Example: `## Guards` followed by `staged_play_time_font_paints_nothing`,
   with its PASS and FAIL lines recorded in the same closing report.

6. Every commit hash in any tracker comment (issue body or later comment)
   must resolve via `git cat-file -t <sha>` and print `commit` at posting
   time. Re-check after each rebase and before every `gh issue comment` or
   body edit. Verify a replacement with `git rev-parse <sha>` (full form),
   `git cat-file -t <sha>` (`commit`), and `git show -s --format='%H %s'
   <sha>` (subject matches the claim); add `git merge-base --is-ancestor
   <sha> HEAD` when the commit must be on the current line. A token that
   fails is replaced by a verified SHA from `git log --all` or marked
   explicitly UNVERIFIED. Version stamps that only look like hashes are
   labeled as stamps, not commits. Post a correction comment on the
   affected issue; do not let a stale token stand in a closed comment's
   history (retro #309). `scripts/tracker-audit.sh` cannot enforce this
   rule; the reviewer check at posting time is the enforcement point.


7. **Live-path reachability for player-visible closes.** Every guard named
   under `## Guards` carries, in the same closing comment, one of:
   - a **live caller chain** from boot to the guarded code, using real entry
     and function names (for example `main.rs::run_play` → `setup_online` →
     `connect::connect_and_login_with_transport` → `stage_online_identity` →
     `ScreenGame::set_avatar_source`); or
   - an explicit **seam-suffices** note (`seam: <reason>`) stating why no live
     chain reaches the code (pure parser/codec/math, seam-defined contract
     cited by an ADR or claim, or unreachable by design).
   A guard with neither is a seam-only guard and cannot close a player-visible
   ticket. Authoring template and tier ladder: `docs/live-path-reachability.md`.
   Precedent: #191/#192/#193 closed on green seam guards while the features
   stayed broken live — the guards never named the live chain.

   The close itself then needs a **confirmation leg**: owner playtest, a
   headless live-path walk (`live_boot_gate`, `session_walk`, `live_stats`),
   or a scripted-gateway round trip through the real boot entry. Seam guards
   alone never close a player-visible ticket. Docs and presentational fixes
   use the `(none -- docs: ...)` / `(none -- presentational: ...)` exemption
   from rule 5 and skip this leg.

## Edit tracker

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
