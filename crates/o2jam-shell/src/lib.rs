//! `o2jam-shell` — window, keyboard input, and the fixed-timestep
//! frame loop for the game client. OS surface only: game logic lives
//! in `o2jam-game`, audio time in `o2jam-audio`. Headless tests cover
//! the key map and the timestep math; window creation needs hardware.

pub mod screen;

pub use screen::{can_enter, BadTransition, Screen, ScreenFlow};

use std::collections::HashSet;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

/// Game tick rate: the original client ran its pump at display rate;
/// logic advances in fixed 60 Hz steps interpolated for render.
pub const TICK_HZ: u64 = 60;

/// Nanos per logic tick.
pub const TICK_NANOS: u64 = 1_000_000_000 / TICK_HZ;

/// Seven play lanes, left to right.
pub const LANE_COUNT: usize = 7;

/// Default window size, VERIFIED against 7.55: the CDX display init
/// hardcodes 800x600 (`SetWindowPos(hwnd, 0, 0, 0, 800, 600,
/// SWP_SHOWWINDOW)` at file `0x14b4c`/`0x14baf` via the USER32 IAT slot
/// `0x63f3a0`; matching 800/600 pushes at `0x4169xx`/`0x416bxx`/
/// `0x416cxx`; `DirectDrawCreateEx` exclusive-mode init via thunk
/// `0x5adccc` from `0x413080`/`0x413c2f`). Window creation forks at
/// file `0x41c0b` on a runtime flag bit: default `WS_POPUP|WS_VISIBLE`
/// (`0x90000000`, fullscreen) vs `WS_CAPTION` (`0xC00000`, windowed)
/// — no config/registry toggle for the flag exists in 7.55, so the
/// client runs fullscreen 800x600 in practice.
pub const DEFAULT_WIDTH: u32 = 800;
/// Default window height in px (see [`DEFAULT_WIDTH`]).
pub const DEFAULT_HEIGHT: u32 = 600;

/// Maps physical keys to the seven play lanes (S D F Space J K L,
/// the club-standard layout) plus menu keys.
#[derive(Debug, Default)]
pub struct Keymap {
    held: HashSet<KeyCode>,
}

impl Keymap {
    /// Create an empty key map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a key press; returns the lane when a lane key goes down.
    pub fn press(&mut self, code: KeyCode) -> Option<usize> {
        self.held.insert(code);
        lane_of(code)
    }

    /// Record a key release; returns the lane when a lane key goes up.
    pub fn release(&mut self, code: &KeyCode) -> Option<usize> {
        self.held.remove(code);
        lane_of(*code)
    }

    /// Whether a lane key is currently held.
    #[must_use]
    pub fn lane_held(&self, lane: usize) -> bool {
        lane_key(lane).is_some_and(|code| self.held.contains(&code))
    }
}

/// Lane index for a play key, if any.
#[must_use]
pub const fn lane_of(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::KeyS => Some(0),
        KeyCode::KeyD => Some(1),
        KeyCode::KeyF => Some(2),
        KeyCode::Space => Some(3),
        KeyCode::KeyJ => Some(4),
        KeyCode::KeyK => Some(5),
        KeyCode::KeyL => Some(6),
        _ => None,
    }
}

/// Play key for a lane, if any.
#[must_use]
pub const fn lane_key(lane: usize) -> Option<KeyCode> {
    match lane {
        0 => Some(KeyCode::KeyS),
        1 => Some(KeyCode::KeyD),
        2 => Some(KeyCode::KeyF),
        3 => Some(KeyCode::Space),
        4 => Some(KeyCode::KeyJ),
        5 => Some(KeyCode::KeyK),
        6 => Some(KeyCode::KeyL),
        _ => None,
    }
}

/// Fixed-timestep accumulator: feeds whole logic ticks from variable
/// frame deltas and reports the leftover fraction for interpolation.
#[derive(Debug, Default)]
pub struct FrameClock {
    accumulator_nanos: u64,
}

impl FrameClock {
    /// Create a clock with an empty accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance by a frame delta; returns whole ticks owed plus the
    /// interpolation alpha in `[0, 1)`.
    pub fn advance(&mut self, delta_nanos: u64) -> (u64, f32) {
        self.accumulator_nanos = self.accumulator_nanos.saturating_add(delta_nanos);
        let ticks = self.accumulator_nanos / TICK_NANOS;
        self.accumulator_nanos %= TICK_NANOS;
        (ticks, self.accumulator_nanos as f32 / TICK_NANOS as f32)
    }
}

/// Window event sink: the game implements [`GameLoop`] to receive
/// input and redraw requests.
pub trait GameLoop {
    /// A lane key changed state (`down` = press).
    fn lane_key(&mut self, lane: usize, down: bool);
    /// Redraw the current frame.
    fn redraw(&mut self);
}

/// Minimal winit shell: one window, keyboard routed to lanes,
/// redraw on demand. Constructed headless-safe; [`Shell::run`]
/// needs a display and never returns until the window closes.
pub struct Shell {
    keymap: Keymap,
    window: Option<Window>,
    game: Box<dyn GameLoop>,
}

impl Shell {
    /// Create a shell around a game loop. No window yet.
    pub fn new(game: Box<dyn GameLoop>) -> Self {
        Self {
            keymap: Keymap::new(),
            window: None,
            game,
        }
    }

    /// Open the window and run the event loop.
    pub fn run(self, event_loop: winit::event_loop::EventLoop<()>) -> anyhow::Result<()> {
        use winit::application::ApplicationHandler;
        struct Handler {
            shell: Shell,
        }
        impl ApplicationHandler for Handler {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                let attrs = WindowAttributes::default()
                    .with_title("recomp2jam")
                    .with_inner_size(winit::dpi::PhysicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT));
                self.shell.window = Some(
                    event_loop
                        .create_window(attrs)
                        .expect("window creation failed"),
                );
            }
            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                _id: winit::window::WindowId,
                event: WindowEvent,
            ) {
                match event {
                    WindowEvent::CloseRequested => event_loop.exit(),
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(code),
                                state,
                                ..
                            },
                        ..
                    } => {
                        let down = state == ElementState::Pressed;
                        let lane = if down {
                            self.shell.keymap.press(code)
                        } else {
                            self.shell.keymap.release(&code)
                        };
                        if let Some(lane) = lane {
                            self.shell.game.lane_key(lane, down);
                        }
                    }
                    WindowEvent::RedrawRequested => self.shell.game.redraw(),
                    _ => {}
                }
            }
            fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
                if let Some(window) = self.shell.window.as_ref() {
                    window.request_redraw();
                }
            }
        }
        let mut handler = Handler { shell: self };
        event_loop
            .run_app(&mut handler)
            .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lane_keys_cover_all_seven_lanes() {
        for lane in 0..LANE_COUNT {
            let code = lane_key(lane).unwrap();
            assert_eq!(lane_of(code), Some(lane));
        }
        assert_eq!(lane_key(LANE_COUNT), None);
        assert_eq!(lane_of(KeyCode::Enter), None);
    }

    #[test]
    fn keymap_tracks_holds_and_releases() {
        let mut map = Keymap::new();
        assert_eq!(map.press(KeyCode::KeyS), Some(0));
        assert!(map.lane_held(0));
        assert_eq!(map.press(KeyCode::Enter), None);
        assert_eq!(map.release(&KeyCode::KeyS), Some(0));
        assert!(!map.lane_held(0));
    }

    #[test]
    fn default_window_matches_verified_7_55_mode() {
        assert_eq!((DEFAULT_WIDTH, DEFAULT_HEIGHT), (800, 600));
    }

    #[test]
    fn frame_clock_splits_ticks_and_alpha() {
        let mut clock = FrameClock::new();
        let (ticks, alpha) = clock.advance(TICK_NANOS * 2 + TICK_NANOS / 4);
        assert_eq!(ticks, 2);
        assert!((alpha - 0.25).abs() < 1e-6);
        let (ticks, _) = clock.advance(0);
        assert_eq!(ticks, 0);
    }

    #[test]
    fn frame_clock_saturates_on_huge_deltas() {
        let mut clock = FrameClock::new();
        let (ticks, alpha) = clock.advance(u64::MAX);
        assert!(ticks > 0);
        assert!((0.0..1.0).contains(&alpha));
    }
}
