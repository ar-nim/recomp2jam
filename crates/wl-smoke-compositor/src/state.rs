//! Dispatch state: frame-callback bookkeeping and paint counters.
//!
//! Bookkeeping lives here (keyed by `ObjectId`), not in the resources'
//! user data, because [`wayland_server::Dispatch`] only hands out an
//! immutable user-data reference — mutating pending attach/frame state
//! between requests requires owning it in the mutable dispatch state.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use wayland_protocols::xdg::shell::server::xdg_wm_base;

use wayland_server::backend::ObjectId;
use wayland_server::protocol::wl_buffer::WlBuffer;
use wayland_server::protocol::wl_callback::WlCallback;
use wayland_server::protocol::wl_pointer::ButtonState;
use wayland_server::protocol::wl_pointer::WlPointer;
use wayland_server::protocol::wl_surface::WlSurface;
use wayland_server::protocol::{wl_compositor, wl_seat, wl_shm};
use wayland_server::{Display, GlobalDispatch, Resource};

/// Linux input event code for the left mouse button (`BTN_LEFT`).
pub const BTN_LEFT: u32 = 0x110;

/// Pacing for [`FrameCallbackPolicy::Flow`]: a frame callback is completed
/// once it has been armed for one interval, capping the client's repaint
/// rate at roughly 60 Hz without any real vsync.
const FLOW_PACING: Duration = Duration::from_millis(16);

/// Whether `wl_callback.done` is ever sent for `wl_surface.frame` requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCallbackPolicy {
    /// Starvation: the callback is recorded and never completed — the
    /// compositor-occlusion condition behind winit #4668 / #118, held
    /// indefinitely and on demand.
    Starve,
    /// Flow: armed callbacks complete after [`FLOW_PACING`] — the sanity
    /// arm proving the game paints and stays healthy when callbacks
    /// arrive, so an idle verdict cannot pass by never painting at all.
    Flow,
}

/// Driver-visible counters; everything the verdict needs, nothing more.
#[derive(Debug, Clone, Copy)]
pub struct CompositorStats {
    /// Commits that attached a buffer — i.e. real paints the compositor
    /// could have shown.
    pub paints: u64,
    /// Frame callbacks armed but not yet completed.
    pub pending_frame_callbacks: usize,
    /// When the first paint landed, if any.
    pub first_paint_at: Option<Instant>,
}

/// A `wl_surface.frame` callback we accepted and have not completed.
struct ArmedFrameCallback {
    /// Owning surface, so destroying the surface can drop its callbacks
    /// before they would be sent on a dead object.
    owner: ObjectId,
    armed_at: Instant,
    callback: WlCallback,
}

/// One server-side pointer event to inject into the client — the
/// wire-level shape of a real pointer gesture, so the injection path
/// cannot drift from what a compositor actually sends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerEvent {
    /// `wl_pointer.enter`: pointer moves onto the surface.
    Enter { x: f64, y: f64 },
    /// `wl_pointer.motion`: pointer moves within the surface.
    Motion { x: f64, y: f64 },
    /// `wl_pointer.button`: one button edge (left/middle/right by code).
    Button { button: u32, pressed: bool },
    /// `wl_pointer.leave`: pointer leaves the surface.
    Leave,
}

impl PointerEvent {
    /// The standard click sequence at surface-local coordinates:
    /// enter, one motion (so a press never arrives positionless), the
    /// left-button down/up pair, leave. A client that tracks the
    /// pointer from motion/enter events — winit's contract — resolves
    /// the press position from the motion here, exactly as with a
    /// human click.
    pub fn click_at(x: f64, y: f64) -> Vec<PointerEvent> {
        vec![
            PointerEvent::Enter { x, y },
            PointerEvent::Motion { x, y },
            PointerEvent::Button {
                button: BTN_LEFT,
                pressed: true,
            },
            PointerEvent::Button {
                button: BTN_LEFT,
                pressed: false,
            },
            PointerEvent::Leave,
        ]
    }
}

/// Dimensions of one shm buffer, kept as its user data so a commit
/// can record the painted surface size (injection needs it to aim at
/// the surface center).
#[derive(Debug, Clone, Copy)]
pub struct BufferData {
    pub width: i32,
    pub height: i32,
}

/// Per-surface state accumulated between commits.
#[derive(Default)]
pub struct PendingSurface {
    /// `attach` since the last commit (`None` = detached or untouched).
    attached: Option<WlBuffer>,
    /// `frame` requests since the last commit; they take effect on commit.
    armed: Vec<WlCallback>,
}

impl PendingSurface {
    /// Record `attach` (or detach, when `buffer` is `None`).
    pub fn set_attached(&mut self, buffer: Option<WlBuffer>) {
        self.attached = buffer;
    }

    /// Queue a `frame` request to arm on the next commit.
    pub fn push_armed(&mut self, callback: WlCallback) {
        self.armed.push(callback);
    }
}

/// The dispatch state threaded through every [`wayland_server::Dispatch`]
/// callback.
pub struct State {
    policy: FrameCallbackPolicy,
    /// xdg configure serial source (monotonic per client, never reused).
    next_serial: u32,
    surfaces: HashMap<ObjectId, PendingSurface>,
    armed_frame_callbacks: Vec<ArmedFrameCallback>,
    paints: u64,
    first_paint_at: Option<Instant>,
    /// Pointer object the client created via `wl_seat.get_pointer`.
    /// The game creates exactly one; injection sends events on it.
    pointer: Option<WlPointer>,
    /// Surface decorated by the (single) xdg_toplevel — the injection
    /// target. The game has exactly one window.
    toplevel: Option<WlSurface>,
    /// Last committed attach dimensions per surface (buffer width and
    /// height), so injection can aim at the painted surface's center.
    surface_sizes: HashMap<ObjectId, (i32, i32)>,
    /// Clock anchor for the `time` argument of pointer events.
    started: Instant,
}

impl State {
    pub fn new(policy: FrameCallbackPolicy) -> State {
        State {
            policy,
            next_serial: 0,
            surfaces: HashMap::new(),
            armed_frame_callbacks: Vec::new(),
            paints: 0,
            first_paint_at: None,
            pointer: None,
            toplevel: None,
            surface_sizes: HashMap::new(),
            started: Instant::now(),
        }
    }

    /// Record the pointer object a `wl_seat.get_pointer` created.
    pub fn set_pointer(&mut self, pointer: WlPointer) {
        self.pointer = Some(pointer);
    }

    /// Record the surface an `xdg_toplevel` decorates.
    pub fn set_toplevel(&mut self, surface: WlSurface) {
        self.toplevel = Some(surface);
    }

    /// Injection can start once the client bound a pointer and created
    /// its toplevel.
    pub fn pointer_ready(&self) -> bool {
        self.pointer.is_some() && self.toplevel.is_some()
    }

    /// Surface-local center of the toplevel's last committed buffer —
    /// where a click lands. `None` before the first paint.
    pub fn toplevel_center(&self) -> Option<(f64, f64)> {
        let (width, height) = self.surface_sizes.get(&self.toplevel.as_ref()?.id())?;
        Some((f64::from(*width) / 2.0, f64::from(*height) / 2.0))
    }

    /// Queue one pointer event on the client's pointer object. Events
    /// flush with the next server turn; a dead client object makes the
    /// event a no-op (the game may exit between the driver's ready
    /// check and the injection).
    pub fn dispatch_pointer_event(&mut self, event: &PointerEvent) {
        // Handle clones, not borrows: `next_serial` needs `&mut self`
        // while the sends hold the resources.
        let (Some(pointer), Some(surface)) = (self.pointer.clone(), self.toplevel.clone()) else {
            return;
        };
        if !pointer.is_alive() || !surface.is_alive() {
            return;
        }
        let now_ms = self.started.elapsed().as_millis() as u32;
        match event {
            PointerEvent::Enter { x, y } => {
                pointer.enter(self.next_serial(), &surface, *x, *y);
            }
            PointerEvent::Motion { x, y } => {
                pointer.motion(now_ms, *x, *y);
            }
            PointerEvent::Button { button, pressed } => {
                let state = if *pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                pointer.button(self.next_serial(), now_ms, *button, state);
            }
            PointerEvent::Leave => {
                pointer.leave(self.next_serial(), &surface);
            }
        }
    }

    /// Next protocol serial (enter/button/leave each take one).
    fn next_serial(&mut self) -> u32 {
        self.next_serial = self.next_serial.wrapping_add(1);
        self.next_serial
    }

    pub fn stats(&self) -> CompositorStats {
        CompositorStats {
            paints: self.paints,
            pending_frame_callbacks: self.armed_frame_callbacks.len(),
            first_paint_at: self.first_paint_at,
        }
    }

    /// Mutable slot for the surface's between-commit accumulation.
    pub fn surface_pending(&mut self, surface: ObjectId) -> &mut PendingSurface {
        self.surfaces.entry(surface).or_default()
    }

    /// Fold one surface commit into the global state: its frame callbacks
    /// arm, and an attached buffer counts as a paint and is released (we
    /// never render, so the client may reuse it immediately).
    pub fn commit(&mut self, surface: ObjectId) {
        let Some(pending) = self.surfaces.remove(&surface) else {
            return;
        };
        let PendingSurface { attached, armed } = pending;
        let now = Instant::now();
        for callback in armed {
            self.armed_frame_callbacks.push(ArmedFrameCallback {
                owner: surface.clone(),
                armed_at: now,
                callback,
            });
        }
        if let Some(buffer) = attached {
            self.paints += 1;
            if self.first_paint_at.is_none() {
                self.first_paint_at = Some(now);
            }
            // Remember what the client painted so injection can aim at
            // the surface center (the configure is 0x0; the size is the
            // client's own choice).
            if let Some(data) = buffer.data::<BufferData>() {
                self.surface_sizes
                    .insert(surface.clone(), (data.width, data.height));
            }
            // Generated senders swallow InvalidId for dead objects, which
            // is the detach-then-destroy race handled.
            buffer.release();
        }
    }

    /// Surface destroyed: drop its never-committed callbacks so a later
    /// `done` can never be sent on a dead object.
    pub fn drop_surface(&mut self, surface: ObjectId) {
        self.surfaces.remove(&surface);
        self.surface_sizes.remove(&surface);
        self.armed_frame_callbacks
            .retain(|armed| armed.owner != surface);
    }

    /// A client-side `wl_callback.destroy` (winit destroys callbacks after
    /// `done`; this covers the rest).
    pub fn drop_frame_callback(&mut self, callback: ObjectId) {
        self.armed_frame_callbacks
            .retain(|armed| armed.callback.id() != callback);
    }

    /// Complete every frame callback whose policy allows it and whose
    /// pacing interval has elapsed.
    pub fn complete_due_frame_callbacks(&mut self) {
        if self.policy == FrameCallbackPolicy::Starve {
            return;
        }
        let now = Instant::now();
        let mut index = 0;
        while index < self.armed_frame_callbacks.len() {
            let due = now.duration_since(self.armed_frame_callbacks[index].armed_at) >= FLOW_PACING;
            if due {
                let armed = self.armed_frame_callbacks.remove(index);
                self.next_serial = self.next_serial.wrapping_add(1);
                armed.callback.done(self.next_serial);
            } else {
                index += 1;
            }
        }
    }

    /// Next xdg configure serial.
    pub fn next_configure_serial(&mut self) -> u32 {
        self.next_serial = self.next_serial.wrapping_add(1);
        self.next_serial
    }
}

/// Advertise the globals the game client binds (see crate docs for why
/// this exact set and these versions).
pub fn create_globals<S>(display: &Display<S>)
where
    S: GlobalDispatch<wl_compositor::WlCompositor, ()>
        + GlobalDispatch<wl_shm::WlShm, ()>
        + GlobalDispatch<wl_seat::WlSeat, ()>
        + GlobalDispatch<xdg_wm_base::XdgWmBase, ()>,
    S: 'static,
{
    display
        .handle()
        .create_global::<S, wl_compositor::WlCompositor, _>(4, ());
    display.handle().create_global::<S, wl_shm::WlShm, _>(1, ());
    display
        .handle()
        .create_global::<S, wl_seat::WlSeat, _>(1, ());
    display
        .handle()
        .create_global::<S, xdg_wm_base::XdgWmBase, _>(3, ());
}

/// wl_shm formats the client can rely on (softbuffer's two).
pub fn announce_shm_formats(shm: &wl_shm::WlShm) {
    shm.format(wl_shm::Format::Argb8888);
    shm.format(wl_shm::Format::Xrgb8888);
}

/// Pointer-only seat capabilities.
pub fn announce_seat_capabilities(seat: &wl_seat::WlSeat) {
    seat.capabilities(wl_seat::Capability::Pointer);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The injected click must be the standard wire sequence: a press
    /// never arrives positionless (the motion precedes it) and the
    /// button edges bracket the pair with enter before and leave after.
    #[test]
    fn click_sequence_is_enter_motion_button_button_leave() {
        let sequence = PointerEvent::click_at(400.0, 300.0);
        assert_eq!(
            sequence,
            vec![
                PointerEvent::Enter { x: 400.0, y: 300.0 },
                PointerEvent::Motion { x: 400.0, y: 300.0 },
                PointerEvent::Button {
                    button: BTN_LEFT,
                    pressed: true
                },
                PointerEvent::Button {
                    button: BTN_LEFT,
                    pressed: false
                },
                PointerEvent::Leave,
            ]
        );
        let is_button = |event: &PointerEvent| matches!(event, PointerEvent::Button { .. });
        let first_button = sequence.iter().position(is_button).expect("a button edge");
        let last_button = sequence.iter().rposition(is_button).expect("a button edge");
        assert!(
            matches!(sequence.first(), Some(PointerEvent::Enter { .. })),
            "enter opens the sequence"
        );
        assert!(
            matches!(sequence.last(), Some(PointerEvent::Leave)),
            "leave closes the sequence"
        );
        assert!(
            !is_button(&sequence[first_button - 1]),
            "a motion arms the press position before the first button edge"
        );
        assert_eq!(
            last_button - first_button,
            1,
            "down and up are adjacent (no stray event between the edges)"
        );
    }
}
