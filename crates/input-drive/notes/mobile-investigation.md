# Mobile follow-up: Android/iOS investigation

Status: INVESTIGATION, not a build slot. No mobile harness, runner, or
game surface exists — this note scopes what would have to be true
before any code is written.

## What we know

- The game client targets desktop window managers (winit Wayland/X11
  on Linux, native Windows/macOS slots above). There is no Android or
  iOS surface, activity, or lifecycle owner in the tree.
- `InputSnapshot`/`GameLoop::ingest_snapshot` (ADR-0013) is already
  platform-neutral: pointer + held state + ordered edges. A future
  touch source would translate touch down/move/up into the existing
  `InputEdge::PointerDown`/`PointerUp` plus pointer motion — no seam
  change needed. The seam is ready; the platform is not.

## What would have to be true first

1. A runnable mobile surface: either a winit mobile backend that
   reaches `WindowHandler` intact, or a new shell owner for the
   activity lifecycle. Until paint + `deliver_snapshot` run on-device,
   injection has no target.
2. An injection path per OS: Android `InputManager` instrumentation or
   `adb shell input tap/swipe` against a self-owned emulator (the
   Xvfb analogue); iOS XCUITest taps against a simulator. Both need
   device labs this repo does not have.
3. A verdict path: the game input log over `adb logcat`/simulator
   logs, or frame dumps pulled off-device — the capture plan assumes a
   local filesystem.

## Next step

None scheduled. Revisit only after (1) exists. If a mobile surface
ever lands, the acceptance for its source is the same contract as the
Linux arms: the shared scripted gesture, control motion silent, click
routes, game acts — applied to the touch translation.
