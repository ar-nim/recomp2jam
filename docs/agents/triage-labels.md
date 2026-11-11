# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

The category roles `bug` and `enhancement` already exist in the tracker under those names.

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

## Triage flow

Every new issue enters with `needs-triage`, and a triage decision precedes any
fix work: the triage step assigns the category (`bug`/`enhancement`) and the
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
   survive on a closed issue, and the category label (`bug`/`enhancement`) is
   applied at triage time, not left for the close.

## Edit tracker

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.
