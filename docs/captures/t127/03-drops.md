# t127 — what drops

The push trait drops.
`InputDriver::move_to`, `button`, `key`, and the default `click` go away.
Nothing may call into the game between freezes.
The forwarding gap behind #118 must stay structurally impossible.

Direct `GameLoop` event methods drop as injection targets.
Old push seams (`lane_key`, `menu_key`, raw key capture) are not driven directly.
Evidence: `crates/o2jam-shell/src/lib.rs` `GameLoop::lane_key`, `GameLoop::menu_key`, `GameLoop::capture_raw_key`.
Tests feed snapshots through `ingest_snapshot` only.

The idea of extracting pointer injection out of the fixture drops.
`wl-smoke-compositor` keeps its `PointerEvent` sender as the server side.
Evidence: `crates/wl-smoke-compositor/src/state.rs` `State::dispatch_pointer_event`; `crates/wl-smoke-compositor/examples/input_smoke.rs` (two-phase verdict: click routes, then game acts).
Duplicating that sender into a client-side crate would create drift.
The driver reuses the fixture, not a copy.

Host-desktop driving drops from every gate.
KWin, uinput, XTEST-against-the-owner-session, and manual clicks are live-only.
No test may depend on them.
