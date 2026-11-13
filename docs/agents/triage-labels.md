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

## Edit tracker

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
