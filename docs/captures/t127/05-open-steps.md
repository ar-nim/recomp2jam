# t127 — OPEN items with next steps

OPEN-1: keyboard edges through the fixture.
The compositor announces pointer-only seats.
Evidence: `crates/wl-smoke-compositor/src/state.rs` `announce_seat_capabilities`.
Lane and menu keys have no Wayland path yet.
Next step: extend the fixture with a keyboard capability or scope X11 as the key path, then map to `InputEdge::Lane` and `InputEdge::Menu`.

OPEN-2: intra-frame tap preservation per source.
ADR-0013 keeps an edge queue so a down/up pair inside one frame still registers.
Each backend must prove it: script a tap between two freezes, assert one snapshot carries both edges.
Evidence: `crates/o2jam-shell/src/window.rs` `snapshot_intake_delivers_both_edges_in_one_freeze`.
Next step: add the tap test to the build ticket.

OPEN-3: snapshot timing source.
Snapshots freeze at frame boundaries, but the driver has no clock contract yet.
Sequence numbers order snapshots (`InputSnapshot::sequence`), wall time is unsettled.
Next step: decide whether the harness stamps wall time or reuses the game log clock, with the #76 latency budget as the consumer.

OPEN-4: Windows and macOS slots.
No runner exists here. SendInput and CGEventPost stay untested behind the trait.
Next step: file the two follow-up tickets from the build branch with CI constraints (macOS accessibility permission).
