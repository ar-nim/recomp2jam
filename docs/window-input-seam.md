# Window input seam (#118 live report, #119 test gap)

Diagnosis for the live playtest report "no click works on any screen;
Enter (keyboard) advances scenes" on the owner's desktop
(`WINIT_UNIX_BACKEND=x11`, Plasma Wayland session with `DISPLAY=:0` =
XWayland). Date: 2026-11-11. Tools: winit 0.30.13 (pinned in
`Cargo.lock`), Xvfb rig via `scripts/rig-run.sh` on `:99`,
python-xlib XTEST driver, `winit-0.30.13` registry source read.

## Rig method

A temporary probe example (a bare `ScreenGame` in a window with the
capture input log; deleted after the diagnosis, per rig discipline)
ran on the `:99` rig. XTEST motion/button/key events drove it; routed
events were read from `/tmp/x11-probe/input.log` (`event <ms> click x
y`, `click-up x y`, `key-down …` lines). Nothing ran on the owner's
display.

## What the rig proved (X11, winit 0.30.13)

1. **Every press/release routed**, in all flows: motion-then-press,
   press on the cached position, and — decisive — a press on a fresh
   window whose pointer never moved. Coordinates were correct.
2. The cold-cache press routed because winit's X11 backend warms the
   client through enter events: mapping a window under the pointer
   (or the pointer entering it) emits `XI_Enter`, which winit turns
   into `WindowEvent::CursorMoved` (`xinput2_mouse_enter`,
   `platform_impl/linux/x11/event_processor.rs` ~L1385); motion adds
   `XI_Motion` → `CursorMoved`. Our `WindowHandler.cursor` cache is
   written by exactly these events (`track_motion` in
   `crates/o2jam-shell/src/window.rs`).
3. `XI_ButtonPress`/`XI_ButtonRelease` emit **only**
   `WindowEvent::MouseInput` — no position, no cursor backfill
   (`xinput2_button_input`, same file ~L1047). The Wayland protocol
   likewise carries no button coordinates. Position tracking is
   client-side by design.
4. Keyboard: XTEST keys reach only the X input-focus holder. With no
   window manager on the rig, focus stays unset and **no** key events
   routed; under a real WM/desktop focus works — matching the live
   report where Enter advanced scenes.
5. The avi screen advances to planet on wall clock alone (splash
   timer in `paint`); a routed click is not needed. Phantom "spontaneous"
   transitions in early captures were this timer, not input.

## The structural constraint (why there is no code fallback)

winit 0.30 removed the `Window::cursor_position()` query; the tracked
`CursorMoved` position is the **only** position source a portable
client has. The 7.55 exe does not share this weakness: it polls
`GetCursorPos` every frame and subtracts its own client origin
(`docs/mouse-tracking.md` §2–3) — pointer state is read, never
waited for. The seam therefore cannot replicate the poll; it can only
track events. A backend/WM combination that delivered button events
with no preceding enter/motion would drop every click on every screen
while keyboard kept working — the exact #118 symptom — and no winit
0.30 API exists to repair it at press time.

## Verdict and open items

The current seam code is **correct on standard X11** (rig-proven,
including the cold-cache case); the suspected f05545a regression is
not in the window arms (that commit only split down/up routing; the
cursor cache predates it and works). The exact desktop trigger is
**OPEN**, ranked hypotheses:

1. **Backend leak**: this machine exports `WAYLAND_DISPLAY` globally;
   if the playtest process did not actually carry
   `WINIT_UNIX_BACKEND=x11` in its environment, the client ran the
   Wayland backend (on which it is known to busy-spin). Next step:
   rerun on the desktop from a shell with `env -u WAYLAND_DISPLAY
   WINIT_UNIX_BACKEND=x11`, log the chosen backend (winit logs it at
   info level), and retest clicks.
2. **Button mapping**: a left-handed libinput swap reports the primary
   button as `MouseButton::Right`; the arm matches `Left` only
   (exe-pinned semantics — extending it is evidence-OPEN). Next step:
   `xev`-style log or a one-line debug print of `MouseInput { state,
   button }` on the desktop.
3. **Focus/enter edge**: an XWayland stacking quirk where neither
   `XI_Enter` nor `XI_Motion` reaches the window for the whole
   session. Next step: same debug print for `CursorMoved`/`CursorLeft`.

The #119 window-seam tests (same-file, headless) now pin this
boundary — motion→press→release routing through the real arm helpers,
avi→planet advance and the planet press split through a live
`ScreenGame`, letterbox drops, and the cold-cache drop — so any future
regression in the arms fails in CI instead of at a live playtest.
