# t127 — input-drive trait shape against ADR-0013

ADR-0013 makes the game pull one `InputSnapshot` per frame.
The game sweeps the snapshot once through `GameLoop::ingest_snapshot`.
Scenes own hit-testing. Push delivery is gone.

Evidence: `crates/o2jam-shell/src/lib.rs` `InputSnapshot`, `InputEdge`, `GameLoop::ingest_snapshot`; `crates/o2jam-shell/src/window.rs` `deliver_snapshot`.

The old ticket #127 trait pushed gestures (`InputDriver::move_to`, `button`, `key`, `click`).
That shape no longer fits. A push driver would bypass the freeze.
It would hide the exact edge-ordering bugs ADR-0013 exists to kill.

Revised shape: the driver produces snapshots, not calls.

```rust
trait SnapshotSource {
    fn snapshot(&mut self) -> InputSnapshot;
}
```

Concrete sources translate one backend event stream into snapshots.
Each source owns pointer cache, held state, and an edge queue.
Each frame the harness drains one source into one snapshot.
The game cannot tell a synthetic snapshot from a frozen live one.

Builders mirror the snapshot helpers: `click_at`, `press_at`, `release_at`, `motion_to` (`crates/o2jam-shell/src/lib.rs` `InputSnapshot::click_at` and siblings).
Backends stay below this seam. Tests assert on snapshots.
