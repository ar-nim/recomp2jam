# Evidence method matrix

For each class of parity claim, the minimum oracle method and the stop
rule. Methods: static sweep (objdump/byte walk, minutes), REA dossier
(minutes, gives evidence ids), Wine rig capture pair (~40 min/session),
proxy rewrite under the rig (one session settled the channel digit
source by rewriting the 1003 field), stopped-state memory read under
winedbg (fragile), negative evidence (the exe provably never paints or
does X).

## Matrix

| Claim class | Minimum method | When to go stronger |
|---|---|---|
| Rendering geometry (rects, anchors, paint order) | Static sweep or DATA walk | Live capture when the file contradicts the capture (ADR-0002) or when a field (not just a value) is in question |
| Input dispatch (keys, clicks, hover) | Static sweep for the dispatch arms | Rig capture pair for the armed frame or hover state; proxy rewrite when the question is which field drives the dispatch |
| Timing (holds, gate legs, splash) | Static sweep for the arming call | Rig capture for the measured duration; a stand-in is never the pin |
| Protocol shape (opcode, body layout) | Static sweep for the builder arms | Proxy rewrite for the field-level question; stopped-state read only when the field is not on the wire |
| Server-computed rule (judgement windows, rank math) | Proxy rewrite to force the answer | Stopped-state read when the server will not cooperate; never a static sweep alone |
| Absence of a behaviour | Structural sweep + named paths | Only when the absence is structural (no consumer, dead code path, statically unreachable) |

## Negative evidence

A negative claim may close a cell when the sweep covers every reachable
path and the absence is structural. The row names the paths checked.

## Stop rule

Stop buying evidence when the method costs more than the cell's tier
warrants, the next step is named, and the verdict is honest about the
gap. An R cell that cannot reach PASS files OPEN with the cheapest next
step, not the most thorough.

## Queue

The two-rig limit keeps a lightweight queue: a ticket claims a rig slot
(a comment on the ticket), and a session waits if both are busy. No
per-cell budgets.
