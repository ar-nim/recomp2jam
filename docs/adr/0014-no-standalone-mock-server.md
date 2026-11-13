# ADR-0014: No standalone mock Memoryer; scripted in-test gateways are the TDD seam

Date: 2026-11-13

## Context

Net-related TDD needs a red-capable loop against server wire shapes.
Two candidate homes: a standalone mock Memoryer binary with scripted
scenarios, or the scripted TCP servers already hand-rolled inside the
test suites (`fake_gateway` in `recomp2jam/src/session.rs` tests,
`loopback_pair` in `o2jam-net/src/client.rs`, connect.rs loopbacks).

## Decision

Keep the in-test fake gateways; do not build a standalone mock server.

## Rationale

1. The in-test pattern already delivers deterministic red-proven round
   trips (precedents: #172 pending-wire, #175 probes, #194 5000→5001,
   #190 inactive-row guard) with zero infrastructure.
2. A standalone double adds a second binary, port allocation, process
   management in CI, and a new flakiness surface — for no sharper
   signal than the loopback already gives.
3. Live Memoryer stays the confirmation rig (owner playtest,
   `scripts/play-local.sh`), never the TDD loop: agents repeatedly
   stalled demanding live probes they could not run; the loop must be
   agent-runnable offline.

## Consequences

- New net behaviors get scripted-gateway round-trip guards first,
  live confirm second (see the Guards close discipline).
- The three hand-rolled variants converge on one shared helper
  (follow-up ticket); this ADR is the pointer that stops the next
  "should we mock the server?" debate.
