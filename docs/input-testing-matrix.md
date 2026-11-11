# Input-testing matrix — OS/environment-agnostic auto-testing of the window input boundary

Date: 2026-11-12. Ticket #121. Related: #118 (live regression), #119 (L0 seam gate, landed).
Read-only research; no repo code changed. Local inventory at
`~/scratch/input-testing/findings-inventory.md`.

## The gap being closed

The #118 regression lived at the winit boundary and was invisible to the
suite; the :99 X11 rig could not reproduce it; the Wayland backend is
untested anywhere. The goal: test `real pointer/keyboard event → client
routing → game dispatch` across OS/display-server environments without
human hands, and stop chasing one environment at a time.

## Tier 0 — headless seam tests (the gate) — LANDED (#119)

Same-file `WindowHandler` tests drive `CursorMoved`/`MouseInput` edges
through the real routing arms. Cost: zero marginal. Catches: routing
rewrites like f05545a, cold-cache drops, letterbox math. Does **not**
catch: backend-level event starvation, compositor focus quirks, button
mapping at the seat, event-loop pathologies (the #118 desktop trigger
class). This is by design — L0 is the merge gate, not the parity proof.

## Tier 1 — environment-agnostic live driver (recommend ydotool)

### Options surveyed

| driver | scope | mechanism | verdict |
|---|---|---|---|
| XTEST / python-xlib | X11 only | X protocol events | proven on the :99 rig; blind on Wayland; motion-only into some canvases (wine DirectDraw lesson) |
| **ydotool (uinput)** | X11 + Wayland + console, compositor-independent | kernel virtual device; picked up by libinput in any compositor | **recommended L1** |
| OS-native: SendInput | Windows (also under Wine) | OS event queue | proven (hovctl/siclick rig); needed for L2-Windows anyway |
| OS-native: CGEventCreateMouseEvent | macOS | CoreGraphics event post | works locally; **Accessibility trust required** — unreliable on CI runners |
| wlroots `zwlr_virtual_pointer_v1` | wlroots compositors only | compositor protocol | precise, but not portable to weston/KWin |
| `weston-test` protocol | weston test builds only | internal protocol (`move_pointer`/`send_button`/`send_key`) | exactly how weston's own suite injects input headlessly; module must never be installed for production |

ydotool caveats (upstream README): `/dev/uinput` access (on the owner
machine a named ACL already grants `arnim` rw — **no config change
needed**); the virtual device takes a moment to be recognized (udev) —
driver must wait/poll before the first event; it moves the *real* seat
pointer, so on a live desktop it drives whatever is under the cursor —
target-window discipline required.

### Integration

- New `scripts/rig-drive.sh` (or a ydotool backend inside the existing
  python driver): `wait-for-device → move → click → key`, with the same
  out-dir/PID discipline as `scripts/rig-run.sh`.
- `rig-run.sh` needs a Wayland counterpart mode: instead of sanitizing
  `WAYLAND_DISPLAY` away, it must *set* `WAYLAND_DISPLAY=<rig socket>`,
  unset `DISPLAY`/`WINIT_UNIX_BACKEND`, launch the client against a rig
  compositor, record PIDs. (Today the script refuses any leaked Wayland
  env — correct for X rigs, blocking for Wayland rigs.)
- Owner-session use: ydotool is the only surveyed tool that can drive
  the real Plasma Wayland desktop natively (XTEST only reaches XWayland
  clients) — this directly re-tests #118's desktop configuration.

### Cost/complexity

Low-medium. One tool to install (user level), one driver script, one
rig-run mode. Unblocks #118 desktop retest, all Wayland rigs, and gives
L2 its injector for free.

## Tier 2 — per-OS CI jobs

### What the ecosystem actually does (survey, 2026-11-12)

- **winit's own CI** (`.github/workflows/ci.yml`): runs `cargo test -p
  winit-x11` / `-p winit-wayland` — backend-crate unit tests with **no
  display server and no event injection**. Integration testing is an
  open winit ask (#2866). So the ecosystem's foundation does not model
  this; we are not missing an off-the-shelf answer.
- **bevy**: builds/tests with Wayland dev packages; input tested at the
  `bevy_input` event level, never injected at the OS level.
- **iced**: `ICED_TEST_BACKEND=tiny-skia` software backend for tests;
  widget-level driving only.
- **egui/eframe**: `egui_kittest` runs real apps in-process and
  synthesizes input at the egui event layer (one layer above the #119
  seam tests) — same class as L0, not OS-level.
- **tauri/wry**: e2e via WebDriver (`tauri-driver`) — webview level;
  wry CI does not open windows.
- **wlroots CI**: `WLR_BACKENDS=headless` with virtual input protocols —
  a working precedent for compositor-side headless testing.
- **weston's own test suite**: headless compositor + `weston-test`
  protocol input injection — the oldest existing Wayland input-in-CI
  recipe.

Conclusion: nobody injects real OS-level input in CI except via (a) X
XTEST under Xvfb, (b) Wayland virtual-pointer protocols against wlroots
or weston test builds. Our L1/L2 design matches the state of the art.

### L2 matrix

| job | runner | stack | injector | feasibility |
|---|---|---|---|---|
| L2-X11 | ubuntu-latest | Xvfb + client (X11 backend) | XTEST via python-xlib (existing rig code) | **proven** — port of the :99 rig; note Xvfb has no WM: keyboard focus stays unset (rig doc §4; matches winit #4822 PointerRoot-focus drop) — run a minimal WM or accept keys-untested |
| L2-Wayland | ubuntu-latest | **weston `--backend=x11-backend.so` nested inside Xvfb** + client (Wayland backend) | XTEST at the X level — weston's x11 backend translates to Wayland | **recommended first Wayland CI job**: everything apt-installable on ubuntu-24.04 (`weston` is in noble), no uinput/udev seats, no GPU, winit sees a genuine Wayland backend |
| L2-Wayland (alt) | ubuntu-latest | wlroots headless (`WLR_BACKENDS=headless`, e.g. tinywl) + client | `zwlr_virtual_pointer_v1`/`virtual_keyboard` | more "pure" (no X in the loop) but needs a compositor we control; keep as second step |
| L2-Windows | windows-latest | client native | SendInput helper (same siclick pattern as the wine rig) | feasible; GH windows runners accept UI automation (FlaUI-class projects do this); must pin the client window handle |
| L2-macOS | macos-latest | client native | CGEventCreateMouseEvent + CGEventPost | **weak**: Accessibility trust on runners is unreliable; mark optional/ignored-fail until proven |

### Cost/complexity

L2-X11: low (script port). L2-Wayland-weston: medium — new rig script
mode + ydotool-or-XTEST driver + focus/activate handling (weston desktop
shell focuses clicked surfaces; the client window is alone on the
output). L2-Windows: medium — one helper binary + job wiring.
L2-macOS: low effort, uncertain payoff.

## Recommended minimal adoption order

1. **L1 ydotool driver + `rig-run.sh` Wayland mode** — cheapest, and it
   is also the #118 desktop retest tool.
2. **L2-X11 CI port** — encode the existing :99 rig as a job so the rig
   and CI stop drifting.
3. **L2-Wayland CI (weston-in-Xvfb)** — first-ever automated coverage
   of the Wayland backend; with L0 this closes the "untested backend"
   hole for routing regressions.
4. **Contained #118 reproduction experiment** (owner machine; needs
   weston + ydotool installed — approval required).
5. **L2-Windows**, then L2-macOS as capacity allows.

## Tie-back to #118

### Upstream match found: winit #4668 (open, winit 0.30.13 — our pinned version)

"Wayland: event loop spins at 100% CPU polling with a zero timeout; no
frames, no input." Root cause per the thread + PR #4675 + eframe #8523:
winit 0.30.13's Wayland backend gates `RedrawRequested` behind the
compositor's `wl_surface.frame` callback; a compositor may withhold that
callback (occlusion); with `ControlFlow::Poll` the loop pins a core from
launch, and in the wedge **no input reaches the app at all**. Confirmed
by two independent reporters (GNOME/mutter; NixOS). The owner symptom —
first native-Wayland launch at 98% CPU — sits squarely in this family.

Read-only check of `crates/o2jam-shell/src/window.rs`: the client never
calls `set_control_flow` (default `ControlFlow::Wait`) and calls
`request_redraw()` unconditionally in `about_to_wait`. That matches the
"never-delivered RedrawRequested" half of #4668 under any control flow;
the pure-`Wait` busy-spin variant is *not* explained by the upstream
thread and is itself a finding the contained repro should settle.
Related upstream items: winit #3933 (request_redraw tight loop = high
CPU on Wayland, closed "expected"), winit #4822 (X11 keyboard dropped
with PointerRoot focus — explains the Xvfb rig's key starvation).

### The ONE most likely contained reproduction

**Nested weston on the owner's current session + ydotool injection.**
`weston --backend=wayland-backend.so` runs as one contained window
inside the Plasma/KWin session (backend choice per `weston(1)`:
wayland/x11/headless/rdp/vnc/pipewire). The client launches with
`WAYLAND_DISPLAY=<weston socket>`, `DISPLAY` unset, backend logged —
winit runs its real Wayland backend against a real, non-KWin
compositor, and no system config changes. ydotool (uinput; ACL already
present) moves the real pointer over the nested window and clicks.
Zero-install fallback compositor candidate: `kwin_wayland` nested
(6.8.2 already installed).

### Confirm/deny checks

1. **Backend + loop fingerprint**: winit backend logged at info;
   `strace -c` on the client — compare against #4668's cycle
   (`epoll_pwait` timeout 0 ~9.4k/s + `timerfd_settime` 1:1 +
   `read(eventfd)=EAGAIN`). Match ⇒ same winit pathology, contained.
2. **Input landing**: instrumented client (input-event log or one-line
   debug print of `CursorMoved`/`MouseInput{state,button}`) —
   - clicks land under weston but died under KWin ⇒ compositor-specific
     (KWin/XWayland path), winit backend exonerated;
   - clicks dead under weston too ⇒ reproduced contained; bisect
     ControlFlow/present path next.
3. **Spin crossover**: same client under nested weston — if 98% CPU
   reproduces, check whether `RedrawRequested` fires at all (frame-
   callback gating); if it fires and the loop still spins, the cause is
   client-side (the unexplained pure-Wait variant).
4. **Deny check**: if everything works under nested weston, the owner
   failure is KWin/XWayland-specific → retest desktop with
   `env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11` plus a `CursorMoved`
   debug print (seam-doc hypothesis 3).

## Open items named for tickets

- L1: ydotool driver script + `rig-run.sh` Wayland mode.
- L2: Wayland CI job (weston-in-Xvfb; XTEST injection; ubuntu-latest).
- L2 (later): Windows SendInput job; wlroots virtual-pointer variant;
  macOS optional.
- #118: contained-reproduction experiment per the checks above
  (requires installing `weston` + `ydotool` on the owner machine —
  owner approval; both user-level, no system config).
