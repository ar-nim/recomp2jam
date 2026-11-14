# t127 — which backends stay

Four injection paths exist today. Each maps to a `SnapshotSource`.

Wayland fixture stays. It is the primary Linux backend.
Evidence: `crates/wl-smoke-compositor/src/lib.rs` `Compositor::inject_pointer`, `Compositor::turn`; `crates/wl-smoke-compositor/src/state.rs` `PointerEvent`, `PointerEvent::click_at`, `State::dispatch_pointer_event`.
The compositor sends the wire sequence of a real click.
The client path (winit backend into `deliver_snapshot`) stays exercised.
No code is extracted from the fixture. The driver reads its events and freezes snapshots from them.

X11 XTEST stays. Ticket #127 already scoped it: XTEST into a self-spawned Xvfb.
The driver owns the server. No external display is needed.
The snapshot translation is new: X11 events become edges, frozen per frame.

Windows SendInput stays as an untested slot behind the trait.
macOS CGEventPost stays as an untested slot with an accessibility-permission note.
Neither has a runner here. Both are defined, not built.

L1 rig driver stays as a live-only source, not a CI backend.
Evidence: `scripts/rig-drive.sh` `cmd_move`, `cmd_click`, `cmd_key` (ydotool over uinput).
It drives the real desktop for owner-machine sessions.
It never runs in CI. It feeds the same snapshot seam when used.
