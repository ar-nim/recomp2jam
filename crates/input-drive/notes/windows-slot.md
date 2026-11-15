# Windows follow-up: SendInput slot behind the snapshot seam

Status: UNBUILT slot. No Windows runner exists in this repo, so no
Windows code ships in this ticket.

## What the slot is

A `SnapshotSource` that drives the real Windows desktop the way the
X11 arm drives Xvfb: `SendInput` mouse motion + button down/up at the
game window's root coordinates, frozen per frame into `InputSnapshot`
edges, verdict off the game input log (`click x y` / `click-up x y`).

## Precedent in this repo

- `scripts/rig-drive.sh` already documents the SendInput pattern
  (`cmd_move`, `cmd_click` via the siclik helper family).
- The Wine rig evidence (`docs/wine-boot.md` attempts 9-12,
  `~/scratch/button-uncover/`) shows XTEST motion never reaches some
  DirectDraw canvases while SendInput does — on Windows proper,
  SendInput is the proven path, not a guess.

## Next step

1. Add `crates/input-drive/src/windows.rs` (`#[cfg(target_os =
   "windows")]`) mirroring `x11.rs`: window lookup by title
   (`recomp2jam`), client-to-root mapping through
   `map_native_to_root`-style math, `SendInput` press/hold/release.
2. Extend `InputBackend` with a `WindowsSendInput` arm and the
   `--backend` flag parsing; explicit selection must keep working when
   auto-detect sees no `WAYLAND_DISPLAY`/`DISPLAY`.
3. CI: `windows-latest` runner, window-handle pin required (the runner
   desktop is shared). No accessibility gate like macOS — SendInput
   needs no trust grant.

## Constraint carried from the matrix

Host-desktop driving stays live-only. The slot must never run in a
shared CI job that also moves a human-visible pointer; the Xvfb lesson
(self-spawned server) has no Windows equivalent, so the Windows job
must own its runner desktop.
