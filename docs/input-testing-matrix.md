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

## Tier 1 — environment-agnostic live driver (recommend python-uinput)

### Options surveyed

| driver | scope | mechanism | verdict |
|---|---|---|---|
| XTEST / python-xlib | X11 only | X protocol events | proven on the :99 rig; blind on Wayland; motion-only into some canvases (wine DirectDraw lesson) |
| **python-uinput (uinput)** | X11 + Wayland + console, compositor-independent | kernel virtual device via `/dev/uinput`, ctypes-only pure Python, no compiled extension, no daemon, no deps | **recommended L1 — no-install**: single-file, works with the existing `/dev/uinput` ACL; GPL-3.0+ so test-tool only, never vendored |
| ydotool (uinput) | same | kernel virtual device | verified upstream: daemon (`ydotoold`) now MANDATORY, usually needs root, CMake build — a fallback if a daemon is ever preferred, not the no-install path |
| OS-native: SendInput | Windows (also under Wine) | OS event queue | proven (hovctl/siclick rig); needed for L2-Windows anyway |
| OS-native: CGEventCreateMouseEvent | macOS | CoreGraphics event post | works locally; **Accessibility trust required** — unreliable on CI runners |
| wlroots `zwlr_virtual_pointer_v1` | wlroots compositors only | compositor protocol | precise, but not portable to weston/KWin |
| `weston-test` protocol | weston test builds only | internal protocol (`move_pointer`/`send_button`/`send_key`) | exactly how weston's own suite injects input headlessly; module must never be installed for production |

uinput-driver caveats (verified against both upstream READMEs): `/dev/uinput`
access (on the owner machine a named ACL already grants `arnim` rw — **no
config change needed**; python-uinput's bundled udev rule exists only to
widen those perms and is unneeded here); the virtual device takes a moment
to be recognized/tagged by udev for libinput — the driver must wait/poll
for readiness before the first event; it moves the *real* seat pointer, so
on a live desktop it drives whatever is under the cursor — target-window
discipline required.

### Integration

- New `scripts/rig-drive.sh` (or a python-uinput backend inside the
  existing python driver): `wait-for-device → move → click → key`, with
  the same out-dir/PID discipline as `scripts/rig-run.sh`.
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
- **weston's own test suite**: compositor-side `weston-test` protocol
  input injection (`move_pointer`/`send_button`/`send_key`/`send_touch`)
  from a test plugin — the oldest Wayland input-in-CI recipe (verified in
  `protocol/weston-test.xml`). Note: weston's *headless* backend itself
  runs "without input or output" per the official docs, so it pairs with
  protocol injection, not uinput.
- **KWin autotests**: in-process compositor + `org.kde.kwin.fakeinput`
  interface; `autotests/integration/fakeinput_test.cpp` and the 95 KB
  `pointer_input.cpp` drive press/move/release through the real input stack.
- **i3 testsuite (X11)**: the docs literally say "tests are run under Xvfb
  display :99" with scripted X11 driving — canonical proof XTEST-class
  injection works headlessly in CI.

Conclusion: nobody injects real OS-level input in CI except via (a) X
XTEST under Xvfb, (b) Wayland virtual-pointer protocols against wlroots
or weston test builds. Our L1/L2 design matches the state of the art.

### L2 matrix

| job | runner | stack | injector | feasibility |
|---|---|---|---|---|
| L2-X11 | ubuntu-latest | Xvfb + client (X11 backend) | XTEST via python-xlib (existing rig code) | **proven** — port of the :99 rig; note Xvfb has no WM: keyboard focus stays unset (rig doc §4; matches winit #4822 PointerRoot-focus drop) — run a minimal WM or accept keys-untested |
| L2-Wayland | ubuntu-latest | **weston `--backend=x11-backend.so` nested inside Xvfb** + client (Wayland backend) | XTEST at the X level — weston's x11 backend translates to Wayland | **recommended first Wayland CI job**: everything apt-installable on ubuntu-24.04 (`weston` is in noble), no uinput/udev seats, no GPU, winit sees a genuine Wayland backend. **Verified constraint**: GH-hosted runners cannot `modprobe` (runner-images#8623, EPERM) and have no `/dev/kvm` (#14549), so uinput injection is impossible there — XTEST/protocol injection only. Compositor noted: this arm runs **weston**, not KWin 6.8.2 (owner desktop) — frame-callback and focus behavior differ; see the compositor-caveat note above the matrix. |
| L2-Wayland (alt) | ubuntu-latest | wlroots headless (`WLR_BACKENDS=headless`, e.g. tinywl) + client | `zwlr_virtual_pointer_v1`/`virtual_keyboard` protocol client (the wayvnc mechanism) | more "pure" (no X in the loop, no kernel input device needed at all) but needs a compositor we control; keep as second step |
| L2-Windows | windows-latest | client native | SendInput helper (same siclick pattern as the wine rig) | feasible; GH windows runners accept UI automation (FlaUI-class projects do this); must pin the client window handle |
| L2-macOS | macos-latest | client native | CGEventCreateMouseEvent + CGEventPost | **weak**: Accessibility trust on runners is unreliable; mark optional/ignored-fail until proven |

### Cost/complexity

L2-X11: low (script port). L2-Wayland-weston: medium — new rig script
mode + ydotool-or-XTEST driver + focus/activate handling (weston desktop
shell focuses clicked surfaces; the client window is alone on the
output). L2-Windows: medium — one helper binary + job wiring.
L2-macOS: low effort, uncertain payoff.

## Recommended minimal adoption order

1. **L1 python-uinput driver + `rig-run.sh` Wayland mode** — cheapest
   (no install at all), and it is also the #118 desktop retest tool.
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
  owner approval; both user-level, no system config). Note the L1
  driver is python-uinput (no install); only `weston` needs approval.

## Test cases (verified plan, 2026-11-12)

Assertion style everywhere below: the client runs with an input-event log
(build-gated debug flag or `InputProbe`-equivalent harness mode) writing one
line per routed event (`click x y`, `down x y`, `up x y`, `move x y`,
`key <lane> down|up`). A case passes when the log's exact sequence matches
the expectation; frame-capture diff is the optional second assertion where
marked. Tier naming: L0 seam (in-crate), L1 owner-machine live, L2 CI.

### L0 — existing seam tests (crates/o2jam-shell/src/window.rs `mod tests`, 23 tests — the merge gate)

Input routing through the real `WindowHandler` arms (InputProbe assertions):

1. `committed_text_reaches_the_game` — setup: probe game; action: `Ime::Commit("abc")`; assertion: `texts == ["abc"]`.
2. `erase_reaches_the_game` — action: `Ime::Commit` + Backspace press; assertion: one `deletes`, texts unchanged after.
3. `lane_keys_still_reach_their_lane` — action: lane-key presses; assertion: each press maps to its lane index with `down=true`.
4. `backspace_release_routes_nowhere` — action: Backspace release only; assertion: no `deletes`, no lanes.
5. `left_press_reaches_the_game_at_native_pixels` — action: `MouseInput(Pressed, Left)` after warm motion; assertion: `clicks == [(x,y)]` at native (unscaled) pixels.
6. `mouse_input_arm_routes_motion_then_both_edges_to_the_game` — action: `CursorMoved` then press+release; assertion: `moves` precede `downs` then `ups` in order.
7. `mouse_input_arm_advances_avi_and_arms_the_planet_press_split` — action: motion+press with an AVI frame pending; assertion: AVI advances, split-arm state set, single down.
8. `mouse_input_arm_drops_a_press_in_the_letterbox_bars` — action: press with position inside a letterbox bar; assertion: no `downs`/`clicks`.
9. `mouse_input_arm_drops_the_edge_when_no_position_is_known` — action: press before any `CursorMoved`; assertion: edge dropped (cold-cache guard, the #118 routing class).
10. `clicks_need_a_sized_surface` — action: press with zero-sized surface; assertion: no route, no panic.
11. `letterbox_bars_never_click_through` — action: press at bar coordinate variants; assertion: always dropped.

Capture log hygiene (concurrent-reader correctness):

12. `capture_log_records_events_frames_and_dumps` — assertion: events/frames/dump lines appear in order.
13. `capture_log_flushes_each_line_for_concurrent_readers` — assertion: no torn lines under concurrent reads.

Geometry/present (letterbox math the routing depends on):

14. `window_mapping_rejects_degenerate_geometry` — zero-scale mapping rejected.
15. `present_accepts_native_backbuffer_at_native_size` — 1:1 pass-through.
16. `present_letterboxes_wide_window_with_uniform_scale` — uniform scale, bars top/bottom.
17. `present_letterboxes_tall_window_with_uniform_scale` — uniform scale, bars left/right.
18. `present_rejects_zero_window_sides` / 19. `present_rejects_mismatched_window_buffer` / 20. `present_rejects_short_backbuffer` — invalid present inputs rejected, no panic.
21. `present_layout_covers_small_window_from_full_frame` — downscale covers fully.
22. `present_clears_only_bars_over_stale_pixels` — bars blacked, frame pixels untouched.
23. `present_paints_letterbox_bars_black` — bar pixels are exactly black.

### L1 — python-uinput driver, owner machine (7 new cases)

Setup shared: `uinput.Device([REL_X, REL_Y, BTN_LEFT, KEY_A..])`; poll udev
for libinput pickup before first event (the ydotool-documented latency);
target-window discipline (activate client window first). Compositor: owner
session, **KWin 6.8.2 on Wayland** — named because all #4668 repros are
GNOME/mutter-verified.

1. `l1_click_routes_on_wayland` — setup: client on native Wayland backend (`WAYLAND_DISPLAY` set, `DISPLAY` unset, backend line in log); action: move to known canvas point (100,100 native), BTN_LEFT press+release; assertion: client input log shows `move 100 100`, `down 100 100`, `up 100 100`; capture diff shows scene response.
2. `l1_click_routes_on_x11` — same driver, client forced to X11 (`WAYLAND_DISPLAY` unset); identical assertions → proves the injector is backend-agnostic and isolates backend-specific routing bugs.
3. `l1_motion_warms_cursor_cache_then_click` — action: click at (x,y) **immediately after** device creation with NO prior motion on this device; then repeat with 200 ms motion dwell; assertion: both land identically — guards the cold-cache-drop class end-to-end (L0 #9 covers the unit side only).
4. `l1_letterbox_drop` — setup: client window sized 16:9 over 4:3 backbuffer; action: click inside a measured black bar; assertion: no `down`/`click` lines; adjacent in-frame click does route.
5. `l1_release_outside_is_noop` — action: press inside canvas, move pointer outside the window, release; assertion: exactly one `down`, no `up`, no click dispatched to the game (and no panic on release).
6. `l1_lane_key_routes` — action: `KEY_A` press+release (keycode-based, layout-independent); assertion: `key <lane(A)> down` then `up`.
7. `l1_device_readiness_guard` — action: emit click 0 ms after device open; assertion: driver reports device-not-ready and defers; no event is silently lost (documented udev pickup latency made explicit).

### L2-Wayland-CI — weston nested in Xvfb + XTEST (6 cases; compositor = weston, NOT KWin)

Setup: ubuntu-latest, apt `weston xvfb python-xlib`-class deps, Xvfb :99,
`weston --backend=x11-backend.so --socket=...` on :99, client with
`WAYLAND_DISPLAY=<socket>` `DISPLAY` unset, input-event log asserted.
(Alt arm, later: wlroots headless + `zwlr_virtual_pointer_v1` injector — same
assertions, no X in the loop.)

1. `l2w_click_routes_headless` — action: XTEST motion+click at canvas point; assertion: input log `move/down/up` at native coords (first-ever automated coverage of the winit Wayland backend path).
2. `l2w_motion_warm_then_click` — same as L1 #3 assertions, weston compositor.
3. `l2w_letterbox_drop` — same as L1 #4 assertions.
4. `l2w_release_outside_is_noop` — same as L1 #5 assertions.
5. `l2w_lane_key_routes` — action: XTEST key press/release; assertion: lane down/up in log. Caveat encoded: with no WM the focused-surface path is weston's own click-to-focus; if keys starve, mark case expected-fail referencing winit #4822 (PointerRoot-focus drop) rather than deleting it.
6. `l2w_frame_callback_starvation_probe` — action: occlude/iconify the client surface (or move it fully behind a second surface), wait 1 s, restore, request redraw via driver click; assertion: input log + a client heartbeat line prove the loop resumed without a 100%-CPU wedge (strace fingerprint of #4668 — `epoll_pwait` timeout-0 at kHz rates — is the failure signature; under our `Wait` control flow the spin variant is not expected, the starvation variant is).

### L2-X11-CI — Xvfb + XTEST port of the :99 rig (5 cases)

Setup: ubuntu-latest, Xvfb :99 + minimal WM (e.g. `icewm`, apt-installable) to
give keyboard focus — without a WM keys stay PointerRoot-focused and starve
(winit #4822; rig doc §4). Injector: existing python-xlib/XTEST rig code.

1. `l2x_click_routes` — XTEST click at canvas point → input log `down/up` at native coords.
2. `l2x_motion_warm_then_click` — cold-device click vs dwelled click land identically.
3. `l2x_letterbox_drop` — bar click dropped, frame click routed.
4. `l2x_release_outside_is_noop` — press inside, release outside → no game event.
5. `l2x_lane_key_routes_with_wm` — key press/release under the minimal WM → lane down/up; without WM this case is skipped with the #4822 reference.

Counts: L0 23 existing (no new code needed beyond keeping them green), L1 7
new, L2-Wayland 6 new, L2-X11 5 new — 18 new live/CI cases on top of the 23
seam tests.

## Appendix — verified findings (2026-11-12, web-verified run)

Every item was fetched and re-checked with working web access (raw JSON,
docs pages, and the Context7 doc lookup `/rust-windowing/winit/v0.30.13`).
Full evidence log with raw snippets: `~/scratch/input-testing/findings-survey-verified.md`.

### winit #4668 (the #118 upstream match) — sharper verdict

- https://github.com/rust-windowing/winit/issues/4668 — **OPEN**. Reported on
  0.30.13 = our pin. Mechanism confirmed in-thread: `ControlFlow::Poll` +
  `RedrawRequested` gated behind a `wl_surface.frame` callback the compositor
  withholds (occlusion) → zero-timeout spin ~9.4k `epoll_pwait`/s, no frames,
  no input. Both reporters reproduced on **GNOME/mutter** (owner desktop is
  **KWin 6.8.2** — compositor must be named; KWin behavior may differ).
- Fix PR https://github.com/rust-windowing/winit/pull/4675 — **closed
  UNMERGED** (author withdrew; maintainer wants eframe-side timeout + a
  Wayland `Occluded` event). Diff fetched: touches `winit-wayland/**` only —
  X11 unaffected; bounds the frame-callback wait to 250 ms (verified
  99.9% → 0.1% CPU on the reporter's real app). Successor fix candidates
  #4709 (master) / #4710 (0.30 backport) — both OPEN.
- Releases API: **0.30.13 is the newest 0.30.x** (no 0.30.14); 0.31 is
  beta-only (beta.4 newest) with no fix in beta.3/beta.4 changelogs.
  → upgrading winit does NOT fix it. Path = `[patch]`-git on the PR head
  commit `6793ca7c3785` or vendoring the small diff, tracking #4668/#4709/#4710.
- Our pattern check (window.rs): never calls `set_control_flow` (default
  `Wait`); `request_redraw()` unconditionally in `about_to_wait` — matches the
  official 0.30 doc example verbatim. The Poll-spin variant cannot describe
  us; the callback-starvation half can (silent block under Wait), which keeps
  the contained-repro experiment live. Frame-callback throttling itself is
  documented winit behavior since 0.29.2.
- #2866 (integration-testing meta): OPEN; proposes xvfb for X11; Wayland
  explicitly unsolved upstream. No off-the-shelf answer exists.

### Injectors

- python-uinput (https://github.com/pyinput/python-uinput): `dependencies = []`,
  ctypes-only, no compiled extension; runtime needs nothing beyond a loaded
  `uinput` module + `/dev/uinput` rw (ACL already grants `arnim` rw; its
  bundled udev rule only widens perms and is unneeded). **No-install L1: YES.**
- ydotool README (https://github.com/ReimuNotMoe/ydotool): daemon mandatory
  since v1.0.0, usually root, CMake build — not the no-install path.
- XTEST is X11-only (https://www.x.org/releases/X11R7.7/doc/libXtst/xtestlib.html).
- SendInput: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput.
- CGEventCreateMouseEvent (current numeric-ID URL; the bare-name URL 404s):
  https://developer.apple.com/documentation/coregraphics/1454356-cgeventcreatemouseevent.

### Headless compositors + GitHub Actions

- Weston backends (official docs): "headless – run without input or output,
  useful for test suite" — headless-weston has no input path, so it pairs with
  compositor-side protocol injection (its own `weston-test.xml`:
  `move_pointer`/`send_button`/`send_key`/`send_touch`), not uinput.
  https://wayland.pages.freedesktop.org/weston/toc/running-weston.html
- wlroots virtual pointer protocol (wayvnc mechanism):
  https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/blob/master/unstable/wlr-virtual-pointer-unstable-v1.xml
- KWin nested options verified in kwin source (`--x11-display`,
  `--wayland-display`, `--width/--height/--outputs`): kwin_wayland 6.8.2
  nested is a zero-install contained-repro compositor.
- GH Actions: GH-hosted runners cannot load kernel modules
  (https://github.com/actions/runner-images/issues/8623, EPERM, closed) and
  have no `/dev/kvm` (#14549) — **uinput injection is not feasible on
  ubuntu-latest**; L2-Wayland-CI must use XTEST-into-weston-x11 or the
  virtual-pointer protocol. uinput drivers are for owner-machine/self-hosted
  arms only.

### Precedents (URLs)

1. Weston test suite — protocol
   https://gitlab.freedesktop.org/wayland/weston/-/blob/main/protocol/weston-test.xml
   and harness tests/harness/weston-test-runner.c (compositor runs in-process
   with a test plugin).
2. KWin autotests — https://invent.kde.org/plasma/kwin/-/tree/master/autotests/integration
   (`fakeinput_test.cpp` via `org.kde.kwin.fakeinput`; `pointer_input.cpp`
   drives press/move/release through the seat).
3. i3 testsuite — https://raw.githubusercontent.com/i3/i3/next/docs/testsuite
   ("tests are run under Xvfb display :99"; Perl X11::XCB driving).
