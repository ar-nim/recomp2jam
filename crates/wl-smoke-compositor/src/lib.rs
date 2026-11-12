//! A minimal headless Wayland compositor that exists only to answer two
//! questions deterministically: does the game idle when the compositor
//! withholds `wl_surface.frame` callbacks, and does a protocol-level
//! pointer click reach the game's input routing?
//!
//! Ticket #126 (refs #118). The owner-machine KWin harness
//! (`scripts/wayland-loop-smoke.sh`, L1) proved the fix (7c3fcfd,
//! `pre_present_notify`) on one compositor, one session, one machine. This
//! fixture is the platform/environment-agnostic counterpart: a
//! `publish = false` workspace crate that is itself the compositor, so the
//! red-green runs on any Linux box with no display server, no KWin, no
//! weston, no installs — CI included.
//!
//! What it implements — deliberately the smallest surface winit 0.30.13 +
//! softbuffer need to map and paint:
//!
//! - `wl_compositor` (v4): surfaces, regions.
//! - `wl_shm` (v1): pools and buffers tracked; every attached buffer is
//!   released immediately on commit (we never render, so the client may
//!   reuse its pool at once — in both modes, so a pre-fix free-running
//!   client is throttled by nothing but its own loop).
//! - `wl_seat` (v1, pointer capability only): enough for winit to treat
//!   the seat as usable; no keymap needed because no keyboard is announced.
//!   Pointer *event injection* (wl_pointer.enter/motion/button/leave) is
//!   implemented here — it is the #118 no-input leg (see
//!   [`PointerEvent`] and `examples/input_smoke.rs`): the compositor
//!   sends the exact wire sequence a live click produces, so the
//!   environment (KWin/uinput/XTEST routing) is bypassed entirely.
//! - `xdg_wm_base` (v3): initial `xdg_toplevel.configure(0, 0)` +
//!   `xdg_surface.configure` on `get_toplevel`, `ack_configure` accepted,
//!   the surface is never killed.
//! - `wl_surface.frame`: the request is recorded; [`FrameCallbackPolicy`]
//!   decides whether `wl_callback.done` is ever sent (`Starve`) or paced
//!   at ~60 Hz (`Flow`).
//!
//! Deliberately absent: rendering, outputs, keyboard, data devices,
//! focus/activation, damage tracking. None are needed to starve (or feed)
//! frame callbacks.

mod handlers;
mod state;

pub mod driver;

pub use state::{CompositorStats, FrameCallbackPolicy, PointerEvent};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use wayland_server::{Display, ListeningSocket};

use crate::state::State;

/// Sleep granularity of one server turn. The turn loop is
/// sleep-then-drain, not fd-poll: a smoke fixture does not need sub-turn
/// latency, and a 20 ms quantum is far below every deadline this harness
/// samples against.
const TURN_QUANTUM: std::time::Duration = std::time::Duration::from_millis(20);

/// One running headless compositor: a listening socket plus the client
/// dispatch state. The driver (see `examples/frame_callback_smoke.rs`)
/// pumps turns and reads [`Compositor::stats`].
pub struct Compositor {
    display: Display<State>,
    listener: ListeningSocket,
    socket_path: PathBuf,
    state: State,
}

impl Compositor {
    /// Bind `socket_name` inside `runtime_dir` (the directory is also what
    /// the client sees as `XDG_RUNTIME_DIR`) and advertise the globals.
    pub fn bind(
        runtime_dir: &Path,
        socket_name: &str,
        policy: FrameCallbackPolicy,
    ) -> Result<Compositor> {
        let socket_path = runtime_dir.join(socket_name);
        let listener =
            ListeningSocket::bind_absolute(socket_path.clone()).context("bind wayland socket")?;
        let display = Display::new().context("create wayland display")?;
        state::create_globals(&display);
        Ok(Compositor {
            display,
            listener,
            socket_path,
            state: State::new(policy),
        })
    }

    /// Absolute path of the socket the client should connect to.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// One server turn: accept pending clients, drain their requests,
    /// complete any due frame callbacks, flush replies.
    pub fn turn(&mut self) -> Result<()> {
        self.accept_pending_clients()?;
        self.display.dispatch_clients(&mut self.state)?;
        self.state.complete_due_frame_callbacks();
        self.display.flush_clients()?;
        Ok(())
    }

    /// Pump [`Compositor::turn`] until `deadline`, returning the first
    /// error (a dead game client surfaces here as a dispatch I/O error or
    /// as the caller noticing the child exited).
    pub fn run_until(&mut self, deadline: std::time::Instant) -> Result<()> {
        while std::time::Instant::now() < deadline {
            std::thread::sleep(TURN_QUANTUM);
            self.turn()?;
        }
        Ok(())
    }

    /// Snapshot of the counters the verdict is built from.
    pub fn stats(&self) -> CompositorStats {
        self.state.stats()
    }

    /// Whether the client bound a pointer and created its toplevel —
    /// the precondition for [`Compositor::inject_pointer`].
    pub fn pointer_ready(&self) -> bool {
        self.state.pointer_ready()
    }

    /// Surface-local center of the toplevel's last committed buffer
    /// (where an injected click lands). `None` before the first paint.
    pub fn toplevel_center(&self) -> Option<(f64, f64)> {
        self.state.toplevel_center()
    }

    /// Queue one server-side pointer event (`enter`/`motion`/`button`/
    /// `leave`) on the client's pointer object, aimed at its toplevel
    /// surface. Events flush with the next [`Compositor::turn`]; a dead
    /// client object makes the event a no-op.
    pub fn inject_pointer(&mut self, event: PointerEvent) {
        self.state.dispatch_pointer_event(&event);
    }

    /// Accept every connection queued on the listening socket.
    fn accept_pending_clients(&mut self) -> Result<()> {
        while let Some(stream) = self.listener.accept().context("accept wayland client")? {
            // `()` implements `ClientData`: we keep no per-client state.
            self.display.handle().insert_client(stream, Arc::new(()))?;
        }
        Ok(())
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        // The lock file disappears with the tempdir; the socket path is
        // removed explicitly so a scratch dir that outlives the process
        // (driver logs) does not confuse a later bind.
        let _ = fs::remove_file(&self.socket_path);
    }
}
