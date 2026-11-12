# ADR-0012: Assets-first consumption; exe-dead assets stay dead

## Status

Accepted (owner, 2026-11-12).

## Context

The archives ship more than the live client uses: 52 `.bnd` files (three
`State_*` binds — Bulletin, Top100, MyCapsule — have no mounted state in
7.55), 34 dialog sheets whose features are side-state P0s, and art bodies
with no observed painter. The owner set the consumption principle: make
100% use of the existing assets (OPI/OPA/OJN/OJM), then wire code only
where behavior is needed; exe-dead material is deprecated by definition
and may stay dead.

## Decision

1. **Live assets are consumed completely.** Every asset the live 7.55
   client references is parsed, staged, and consumed — no live archive
   content left unread. The census (#164) tracks this per file; gaps
   become tickets.
2. **Exe-dead assets are documented dead and skipped.** An asset the exe
   never references (dead states, unmounted dialogs, unreferenced bodies)
   is labeled dead **with its evidence** (no live consumer found in the
   exe census) and left unimplemented. Deadness is a verdict, never a
   silence — the census row says why.
3. **Code wiring rides on top of data.** Behavior (actions, dispatch,
   frame rules) is implemented only for consumed, live controls —
   extending ADR-0003: the files control geometry and art; the code adds
   semantics; nothing hardcoded that the files already answer.

## Consequences

- #164 gains a liveness column: live (evidence: exe consumer pinned) or
  dead (evidence: absence pinned, per ADR-0009 negative-evidence rules).
- #163's sweep orders by liveness: live chrome first; dead controls are
  asserted absent, not wired.
- New side-state and dialog work starts from the census, not from memory.
- "Community archives work" (the moddability bar) means: replace any live
  asset and the client re-stages — the test fixtures come from the census
  rows marked live.
