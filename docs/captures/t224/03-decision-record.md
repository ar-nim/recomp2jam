# T224 leg 3 — decision record (research recommendation)

Status: research input to #224. The call stays the owner's
(per the triage note); this record is the researcher recommendation.

## Decision

Keep winit + softbuffer. The binary does not require hardware
acceleration, and no alternative beats the current backend on this
workload.

## Reasons

1. The exe's own stack needs no GPU: fixed 800x600x16 DirectDraw
   blits with a source colorkey (`01-exe-rendering-stack.md`).
   A software backbuffer is a faithful modern match, not a shortcut.
2. The two known winit costs are already paid and pinned in-tree:
   the `pre_present_notify` frame-callback gate (KWin 6.8.2 repro)
   and the XTEST/decoration rig workarounds (`02-backend-comparison.md`).
   Switching backends re-opens them for no gain.
3. The only backend that changes the capability set (wgpu) serves
   effects no evidenced 7.55 art calls for; it stays OPEN as a future
   upload path, not a replacement.

## Reversal conditions (open an evaluation ticket only if)

- GPU effects enter scope (animated shaders, blended lighting) with
  acceptance numbers for frame pacing, CPU, and backend matrix; or
- a platform the matrix must cover lacks a softbuffer present path;
  or
- cpal disappoints so badly that SDL2's unified audio becomes worth
  its C-toolchain cost.

None of these hold today.
