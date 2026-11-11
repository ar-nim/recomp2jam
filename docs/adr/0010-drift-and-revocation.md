# Drift and revocation

Which changes invalidate which evidence, what revokes a PASS, and what
every cell records so drift is machine-checkable.

## Change-area table

| Change area | Evidence to re-run | Cells affected |
|---|---|---|
| `crates/o2jam-shell/src/screens/paint.rs` or `staging.rs` | `scripts/verify-shots.sh` + the real-art soak (`crates/o2jam-shell/tests/real_art.rs`) | Every cell whose guards touch the changed files |
| `crates/o2jam-file/src/**` (parser) | Parser fixture tests + every format cell (C-claims) | Every cell whose guards touch the changed parser |
| `crates/o2jam-net/src/**` (wire) | Session tests + every wire cell (D-claims) | Every cell whose guards touch the changed wire code |
| `crates/o2jam-game/src/**` (rules) | Game rule tests + every gameplay cell (B-claims) | Every cell whose guards touch the changed rules |
| `scripts/verify-shots.sh` or the shots walk | The full shots walk | Every cell with a `shots` guard |

## Revocation

A PASS is revoked to OPEN when any of the following fires:
- A guard test named in the cell fails.
- A driving claim changes level (EXE → DATA, DATA → ASSUMED, or refuted).
- The evidence file the cell names rots (hash mismatch, capture missing).

Any agent or CI may revoke by opening a ticket and downgrading the
cell. The owner may also revoke manually. A revoked cell is OPEN with a
named next step: re-run the invalidated evidence or re-verify the
claim.

## Per-cell record

Every cell in `docs/parity.md` records, in a machine-checkable form:
- Guard test names.
- Evidence pointer (file offset + evidence id, or capture path).
- Last-verified commit hash.
- Tier (R / G / R+G).
- Verdict.

The consistency check reads these fields and flags: a guard test that
no longer exists, a verdict with no evidence pointer, a tier with no
matching gate rule, and a last-verified commit older than the most
recent change to the cell's driving claims.

## Guard deletion or rename

When a guard test is deleted or renamed, every cell that names it is
invalidated. The rename or deletion commit must either update the
cells in the same commit or open a ticket that lists them.
