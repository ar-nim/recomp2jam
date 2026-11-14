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

Every new issue enters with `needs-triage`, and a triage decision precedes any
fix work: the triage step assigns the category (issue **type**: Bug, Task,
or Feature) and the
role (`ready-for-agent`, `ready-for-human`, `needs-info`, `wontfix`), and
sizes the acceptance. An issue that is not yet triaged is not worked, however
small it looks.

Two recorded exceptions, both with the triage record visible on the tracker:

- Wayfinder map tickets carry `wayfinder:` labels and are triaged by the map
  itself (charting is the triage act).
- Execution issues cut by a scoping ticket enter directly at their decided
  role; the scoping ticket's resolution comment is their triage record and
  stays linked.
- When the only remaining leg is owner hands or eyes, apply `needs-owner`
  alongside the state role (never instead of it) with a comment naming
  the exact ask, and stop delegating until the owner responds.

## Close discipline

Rules for closing playtest-class tickets (retro 2026-11-11, issues #26/#30/#53–#56):

1. A hand-playtest ticket closes only with live `:99` rig evidence: frame/content-hash
   before/after plus the wall run. Seam guards alone never close one.
2. Only the merge/close step closes a ticket, after review and gates. Implementer
   branches never close.
3. File tracker issues sequentially in one chain and read back the title per number;
   never assume parallel `gh issue create` output order.
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

6. Every commit hash cited as evidence in a tracker comment is verified to
   exist before the comment is posted: run `git cat-file -t <sha>` and confirm
   it prints `commit`. Failure mode prevented: an agent stalls on "commit not
   found" when a later step (or a reviewer) tries to resolve the cited object,
   because the hash was mistyped or never existed (precedent: #167 cited
   `ea4e098b`, which resolves to nothing; the real fix was `ea4f098`). This
   check runs before any `gh issue comment`.


7. **Live-path reachability for behavior guards.** Every guard named under
   `## Guards` carries, in the same closing comment, one of:
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

8. **Player-visible close leg.** A player-visible ticket closes only after a
   live/behavior confirmation leg: owner playtest, a headless live-path walk
   (`live_boot_gate`, `session_walk`, `live_stats`), or a scripted-gateway round
   trip through the real boot entry. Seam guards alone never close such a
   ticket. Docs and presentational fixes use the `(none -- docs: ...)`
   / `(none -- presentational: ...)` exemption from rule 5 and skip this leg.

## Edit tracker

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
