# macOS follow-up: CGEventPost slot behind the snapshot seam

Status: UNBUILT slot. No macOS runner exists in this repo, and the
platform gate below makes CI unlikely — this stays an
owner-machine-live investigation until proven otherwise.

## What the slot is

A `SnapshotSource` mirroring `x11.rs`: `CGEventCreateMouseEvent` +
`CGEventPost` motion/click at the game window's screen coordinates,
frozen per frame, verdict off the game input log.

## The platform gate

macOS requires an **Accessibility trust grant** for the process posting
synthetic events
(`CGEventCreateMouseEvent`, Apple docs 1454356). On CI runners the
grant cannot be scripted reliably, so the job would stay
optional/ignored-fail — a slot that cannot gate is documentation, not
coverage. Recorded here instead of the tracker so the constraint
travels with the code.

## Next step (owner machine only)

1. Grant the terminal/IDE Accessibility trust by hand (one-time,
   human-only step — this is why it never runs headless).
2. Add `crates/input-drive/src/macos.rs` (`#[cfg(target_os =
   "macos")]`) mirroring `x11.rs`, plus an `InputBackend` arm and
   `--backend` spelling.
3. Prove the same scripted gesture (control motion silent, center
   click routes, gear click repaints) against a windowed client, then
   decide whether any CI form is worth the flake budget.

## Constraint carried from the matrix

Mark the case expected-fail-with-reference until the grant story is
proven on a fresh machine, the same way the X11-without-WM key cases
carry the winit #4822 reference instead of being deleted.
