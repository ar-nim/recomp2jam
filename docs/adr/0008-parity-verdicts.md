# Parity verdicts

Every parity cell carries one verdict: PASS, ACCEPTED, OPEN, FAIL, or N/A.

## PASS

7.55 evidence + implementation + guard test together. No capture required;
a successful harness run alone is not PASS. The driving claims must be at
EXE or DATA level; an ASSUMED claim under a PASS cell must be named in the
row's text.

## ACCEPTED

A deliberate, ADR-recorded deviation from 7.55 behaviour. The ADR names
the deviation, why it exists, its cost, and its blast radius. The row
carries a guard test that would fail if someone silently fixed or widened
the deviation. An ACCEPTED cell may not gate milestone 3 without owner sign-off.

## OPEN

Untraced or unimplemented behaviour with a named next evidence step. An
OPEN row without a next step is invalid; the consistency check flags it.

## FAIL

A verified gap against 7.55 behaviour: the evidence says X, the shell does
Y ≠ X. Distinct from OPEN because the gap is confirmed, not merely untraced.

## N/A

No 7.55 behaviour exists for this cell (no dispatch, no timer, no
wire event). The row names why no guard applies.

## Rules

- Every verdict row names at least one guard test, or for N/A names why
  none applies.
- A cell may not rest on an ASSUMED or OPEN claim unless the row says so.
- Only the owner (or an ADR) may change a verdict, a tier, or a cell's
  claim set.
