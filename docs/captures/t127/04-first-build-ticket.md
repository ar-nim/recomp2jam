# t127 — first build ticket contents

Crate: `crates/input-drive`, dev-only, `publish = false`.
No production code in this note. The build ticket follows it.

Build the seam first.
Define `SnapshotSource` with one method returning `InputSnapshot`.
Add a `Vec`-backed scripted source for deterministic tests.
Name the caller chain: harness freezes source, calls `ingest_snapshot`, paints.

Port one existing case.
Route `crates/wl-smoke-compositor/examples/input_smoke.rs` through the seam.
Phase 1 (click routes to the input log) must stay green unchanged-verdict.
Phase 2 (gear click repaints the planet) must stay green unchanged-verdict.
Evidence of the contract: `crates/o2jam-shell/src/window.rs` `deliver_snapshot` log lines (`click`, `click-up`).

Add the X11 source.
Self-spawned Xvfb plus XTEST. Same scripted gesture as the Wayland arm.
Assert via the game input log. Both Linux arms select explicitly and auto-detect (`WAYLAND_DISPLAY` with our fixture, else `DISPLAY`).

File follow-ups from the build branch: Windows slot, macOS slot, Android/iOS investigation.
Update the input-testing matrix doc.

Acceptance: crate builds; both Linux arms drive the same case; selection override tested; gates green both arms.
