# Input architecture for #128 — primary-source research

Research date: 2026-11-12. No code changes; this file is the deliverable.
For ticket #128 (ADR): which input architecture should replace the current
winit-push + `dyn`-forward chain for a faithful drop-in recompilation of the
O2Jam 7.55 client.

## The question

The 7.55 exe is the behavior oracle. Pinned exe evidence (claim A46,
`docs/mouse-tracking.md`, from #112): the exe follows the pointer by
**polling** — `GetCursorPos` per frame minus a self-derived client origin
(`GetClientRect`+`ClientToScreen`, no `ScreenToClient` import), a motion gate
against stored globals, and a per-frame `PtInRect` sweep over placed bound
rects whose hits feed a queued-event drain through vtable[1]. **No
`WM_MOUSEMOVE` handler exists in the image.** Discrete events are queued and
drained at the frame boundary. Consumption model: per-frame snapshot (polled
position + queued discretes), hover = position at paint time.

Our current shape (the deviation): winit **pushes** events; `window.rs`
forwards each through `Box<dyn GameLoop>` chains whose methods have default
no-op traps. The #118 incident (live playtest 2026-11-12): an `OnlineSession`
`GameLoop` impl did not override the #115 press edges, so every live
press/release/motion vanished while `menu_key` forwarded — input silently
dropped at a forwarding seam by construction.

Three options from the ticket: (a) keep push, remove the footgun (required
trait methods or a single pass-through message); (b) exe-style per-frame
pulled state (shell caches latest pointer/buttons/keys + queues discretes;
the game reads state at frame start and does its own hit sweep — A46
exactly); (c) hybrid (push discretes, polled position for hover).

## Source positions

### 1. winit 0.30.13 — push has no per-frame sampling; position exists only as events

First-party: docs.rs `winit 0.30.13` (`WindowEvent`, `DeviceEvent`) and the
pinned registry source read for `docs/window-input-seam.md`.

- `WindowEvent::CursorMoved`: "The cursor has moved on the window", carrying
  `(x,y) coords in pixels relative to the top-left corner of the window`
  with the warning that the range "is limited by the display area and it may
  have been transformed by the OS to implement effects such as cursor
  acceleration". No sampling, staleness, or delivery-timing guarantee is
  documented — it is a notification of something that already happened, not a
  readable state.
  <https://docs.rs/winit/latest/winit/event/enum.WindowEvent.html>
- `WindowEvent::MouseInput`: "An mouse button press has been received" with
  fields `{device_id, state, button}` — **no position field**. Position
  tracking is client-side by design on every backend.
  (Same page.)
- `DeviceEvent` (raw hardware motion): "raw, unfiltered physical motion. Not
  to be confused with `WindowEvent::CursorMoved`", "delivered regardless of
  input focus". Raw deltas, no window position — not a pollable cursor
  either. <https://docs.rs/winit/latest/winit/event/enum.DeviceEvent.html>
- winit itself does **not** collapse the motion stream on native backends:
  each `XI_Motion`/enter event becomes one `CursorMoved`
  (`platform_impl/linux/x11/event_processor.rs` ~L1128/1221/1304), each
  `XI_ButtonPress` becomes one position-less `MouseInput` (`xinput2_button_input`,
  ~L1047). The only coalescing logic in the tree is Web
  (`platform_impl/web/web_sys/event.rs`, expanding the browser's
  `getCoalescedEvents()`; changelog v0.29: "On Web, handle coalesced pointer
  events"). Coalescing below winit (X11 motion compression, Wayland
  per-commit motion, browser) still applies — intermediate positions can be
  lost before winit ever sees them.
- Load-bearing consequence, already pinned in `docs/window-input-seam.md`:
  winit 0.30 removed the `Window::cursor_position()` query, so the tracked
  `CursorMoved` position is the **only** position source a portable client
  has. The exe's poll (`GetCursorPos` at frame time) cannot be replicated by
  API; it can only be approximated by a shell-maintained cache sampled at the
  frame boundary.

Parity-critical answer: **yes, push can observe a different pointer position
at paint time than a per-frame poll of latest state would** — two mechanisms:
(1) queue backlog under load: events are delivered as received with no
frame-boundary semantics, so a game that acts on events as they arrive reads
event-time positions, while a poll reads paint-time position; (2) protocol-
level motion compression below winit means even the event stream is already a
subsample the exe's poll would not have taken (the exe reads the true current
position every frame).

### 2. SDL2 — the canonical push-vs-pull statement (first-party wiki)

- `SDL_PollEvent`: "Poll for currently pending events… the next event is
  removed from the queue". "The common practice is to **fully process the
  event queue once every frame, usually as a first step before updating the
  game's state**: `while (SDL_PollEvent(&event)) { … }` // update game state,
  draw the current frame". I.e. even in SDL, the queue is drained **at the
  frame boundary**, then the frame runs on the result.
  <https://wiki.libsdl.org/SDL2/SDL_PollEvent>
- `SDL_PumpEvents`: "updates the event queue **and internal input device
  state**… gathers all the pending input information from devices and places
  it in the event queue." Queue and state are updated together, once per
  pump. <https://wiki.libsdl.org/SDL2/SDL_PumpEvents>
- `SDL_GetMouseState`: "Retrieve the **current state** of the mouse… set to
  the mouse cursor position relative to the focus window" plus a button
  bitmask. Instantaneous read, not an event.
  <https://wiki.libsdl.org/SDL2/SDL_GetMouseState>
- `SDL_GetKeyboardState`: "Get a **snapshot of the current state** of the
  keyboard… **Use `SDL_PumpEvents()` to update the state array.**" And the
  load-bearing sentence for this ticket: "**This function gives you the
  current state after all events have been processed, so if a key or button
  has been pressed and released before you process events, then the pressed
  state will never show up** in the `SDL_GetKeyboardState()` calls."
  <https://wiki.libsdl.org/SDL2/SDL_GetKeyboardState>

SDL's contract is therefore: **events for transitions, state-functions for
frame-sampled truth, and the two disagree exactly when a transition completes
inside one frame** — which is why real-time apps use both (drain the queue
for edges, read state for positions/holds) at the top of each frame.

### 3. Game Programming Patterns (Nystrom, first-party site)

- Game Loop chapter, the canonical loop: "**it processes user input, but
  doesn't wait for it. The loop always keeps spinning**:
  `processInput(); update(); render();`" where "**`processInput()` handles
  any user input that has happened since the last call**." Input is consumed
  once per crank, as a prologue to the tick — the pull-at-frame-start shape.
  <https://gameprogrammingpatterns.com/game-loop.html>
- Update Method: "Once per frame, the game loop walks the collection and
  calls `update()` on each object. This gives each one a chance to perform
  one frame's worth of behavior." Behavior (including the exe's hover sweep)
  belongs to the frame, not to the event arrival.
  <https://gameprogrammingpatterns.com/update-method.html>
- Event Queue, intent: "**Decouple when a message or event is sent from when
  it is processed**… The application **pulls in the event when it wants
  it**… When user input comes in, the OS adds it to a queue of unprocessed
  events. When you call `getNextEvent()`, that pulls the oldest event off the
  queue." And the decisive line for (a)-style push: "**Most games aren't
  event-driven like this**… crack open the Game Loop chapter" for why.
  <https://gameprogrammingpatterns.com/event-queue.html>

### 4. Gaffer on Games (Fiedler, first-party) — determinism needs fixed input per tick

`Fix Your Timestep!` (<https://gafferongames.com/post/fix_your_timestep/>):
fixed-`dt` simulation is "in many ways… ideal", while variable-`dt` makes
"the behavior of your physics simulation depend on the delta time you pass
in" — up to "a slightly different 'feel' depending on framerate". Our shell
already runs a fixed 60 Hz `FrameClock` (`TICK_HZ` in `lib.rs`), so the loop
half of Fiedler's prescription is in place; the open half is feeding each
tick a **stable input snapshot** rather than whatever events happened to
arrive mid-tick. Event-arrival timing is the variable-`dt` of input: the same
physical gesture lands on different ticks depending on machine load, which is
exactly the non-determinism a reference implementation must not have.

### 5. Prior art — frame-locked clients sample input per frame

- **libretro API** (`libretro.h`, first-party header): the frontend hands the
  core two callbacks — `retro_input_poll_t`: "**Polls input**" — and an input
  state query. Contract on `retro_run()`: "the `retro_input_poll_t` callback
  **must be called at least once**" per emulated frame; the core then reads
  state. Every libretro core (i.e. dozens of shipped emulators) runs the (b)
  shape: poll once per frame, then simulate. The discrete-edge problem is
  handled inside the snapshot (per-frame edge bits), not by a parallel push
  path.
- **SDL practice** (same wiki as §2): drain-then-update is "the common
  practice" — the whole queue is emptied as step one of the frame, so all
  downstream logic observes frame-boundary state even in an event API. This
  is the minimal (c)→(b) bridge: whoever drains owns the frame's input truth.

## Analysis for #128

Faithful-parity criterion first (the product is behavior parity; the exe is
the oracle): the exe consumes input as a **per-frame snapshot** — polled
position swept against placed rects at paint time, queued discretes drained
at the frame boundary. Any architecture that routes hover/presses at
event-arrival time can paint a hover or dispatch a click for a pointer
position the exe would never have seen at that frame (stale under backlog,
compressed at the protocol layer). That is a behavior divergence from the
oracle by definition, not a performance nit.

| Option | Parity | Determinism | #76 latency | Footgun |
|---|---|---|---|---|
| (a) keep push, harden forwarding | Fails: event-time positions still differ from paint-time; hover stays message-driven, contradicting A46 | Fails: replay depends on arrival interleaving | Neutral: no added delay, but staleness variance stays | Partial: exhaustiveness helps, but N-method convention surface remains |
| (b) exe-style pulled snapshot | **Matches A46 exactly**: position-at-paint + queued discretes drained per frame | **Best**: one snapshot per frame is loggable/replayable (extends the existing per-paint capture log + `state_hash`) | **Best**: sampling at frame start minimizes input→photon staleness and its variance | **Removes it**: states pull from one snapshot; the `dyn` forward chain seam disappears |
| (c) hybrid | Half: fixes hover, but keeps two truths (event-time edges vs paint-time position) | Partial: edges still arrival-ordered | Good for hover, edges unchanged | Keeps the forwarding chain for discretes — the #118 seam survives |

Note on #76: the latency-budget ticket itself is still an open definition
question ("how is the milestone-3 latency budget defined, measured, and
gated"), so no architecture can claim to meet a number yet. The valid
criterion now is directional: never add queue delay, keep the sampling point
well-defined and measurable. (b) is the only option satisfying both —
sampling at frame start is the earliest well-defined point, and the snapshot
gives the future budget probe a single timestamp to measure from.

Constraint to design around: winit 0.30 offers no position poll, so (b)'s
"poll" is observably-equivalent rather than literally `GetCursorPos`: the
shell maintains the cache it already has (`cursor`, button state, `Keymap.held`)
and the game samples **that** at frame start. The one behavior this cannot
reproduce is a truly current position when the backend delivers no
enter/motion at all (#118's OPEN backend hypotheses) — but no winit API can;
that residual risk is identical under all three options.

One subtlety (b) must preserve, per SDL's `GetKeyboardState` warning: pure
state loses a press-and-release completed inside one frame. A46 already
answers this — the exe **queues** discretes and drains them per frame — so
(b) keeps a per-frame edge queue alongside the state snapshot (press/release-
inside semantics from #111/#115 are re-derived from state transitions at the
frame boundary, not re-pushed). Touch/IME text paths stay pushed; they were
never polled in the exe either (keyboard is the async-key poll).

## Recommendation

**Adopt (b): exe-style per-frame pulled state.** The reasoning chain: the
product is parity with a polling oracle, and only (b) reproduces the oracle's
observable input semantics (hover/presses resolved against paint-time
position, discretes drained per frame); it is the most deterministic shape
(one snapshot per frame feeds replay + the existing trace harness); it best
serves the open #76 budget (no queue delay, one measurable sampling point);
and it structurally removes the #118 footgun instead of guarding it. (c) is
the fallback if the snapshot migration stalls — but it preserves the exact
seam that caused #118.

Migration shape (conceptual — `crates/o2jam-shell/src/`):

- `lib.rs`: introduce one per-frame input snapshot type (pointer position in
  native frame pixels, button state, held-key set, plus the frame's discrete
  edge queue with sequence numbers) that the shell assembles at frame start;
  `GameLoop` gains a single snapshot intake, and the per-event methods
  (`mouse_moved`/`mouse_down`/`mouse_up`/lane/menu edges) become derived
  calls the game makes from the snapshot during its own sweep — or are
  retired once states sweep directly. `Keymap` becomes the keyboard half of
  the snapshot instead of a push-side decoder.
- `window.rs`: `WindowHandler` stops forwarding `CursorMoved`/`MouseInput`/
  `KeyboardInput` straight into game calls; event arms only update the cached
  pointer/buttons/keys and append edge records. The `RedrawRequested`/paint
  path (next to `plan_repaint`/the #138 idle gate) freezes the snapshot, and
  each state does its own `PtInRect`-over-staged-rects sweep at that point —
  matching A46 and the owner rule that geometry comes from staged rects, with
  the painter consuming staged state, never literals.

## Risks / OPEN

1. Backend silence (#118 hypotheses 1–3 in `docs/window-input-seam.md`):
   snapshot-per-frame does not conjure events a backend never delivers; the
   cold-cache drop persists under every option. Mitigation: log the chosen
   backend + a per-frame event-presence bit so the next live report is
   diagnosable from the capture log.
2. Intra-frame taps: must keep the discrete edge queue (A46's drain), never
   pure state — else fast press-release pairs vanish per SDL's warning.
3. Press-split fidelity: #111 (swap on ButtonDown, action on
   release-inside) and #115 (planet arm/fire split) must be re-derived from
   frame-boundary transitions with scenario tests, since edge timing moves
   from arrival-time to frame-time.
4. Test retargeting: the #119 seam tests pin the current arm helpers; they
   move to snapshot assembly + sweep coverage, same file, headless.
5. Text/IME stays pushed (`route_text` path) — out of scope by design.

## Verification notes

All URLs fetched 2026-11-12 via the on-machine browser except the winit
source (pinned `winit-0.30.13` registry source on disk) and `libretro.h`
(raw GitHub fetch of `libretro-common`). File path chosen:
`docs/input-architecture-128.md` — research files in this repo sit at
`docs/` top level (e.g. `docs/refactor-vs-bugfix-order.md`); no
`docs/research/` directory exists, so the top-level name follows convention.
