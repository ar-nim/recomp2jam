//! `o2jam-shell` — window, keyboard input, and the fixed-timestep
//! frame loop for the game client. OS surface only: game logic lives
//! in `o2jam-game`, audio time in `o2jam-audio`. Headless tests cover
//! the key map and the timestep math; window creation needs hardware.

pub mod avatar;
pub mod playing;
pub mod screen;
pub mod screens;

pub use avatar::{
    load_archive_sprite, load_worn_figure, pick_wear_sprites, AvatarFigure, FigureLayer,
};
pub use playing::{HighwayGeometry, LivePlay, PlayingView};
pub use screen::{can_enter, BadTransition, Screen, ScreenFlow};
pub use screens::{paint_screen, ScreenGame};

use std::collections::HashSet;
use std::sync::Arc;
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

/// Menu keys (non-lane UI input).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    /// Confirm / advance (Enter).
    Confirm,
    /// Back out (Escape).
    Back,
    /// Highlight up (ArrowUp).
    Up,
    /// Highlight down (ArrowDown).
    Down,
}

/// Menu key for a physical key, if any.
#[must_use]
pub const fn menu_of(code: KeyCode) -> Option<MenuKey> {
    match code {
        KeyCode::Enter => Some(MenuKey::Confirm),
        KeyCode::Escape => Some(MenuKey::Back),
        KeyCode::ArrowUp => Some(MenuKey::Up),
        KeyCode::ArrowDown => Some(MenuKey::Down),
        _ => None,
    }
}

/// Window event sink: the game implements [`GameLoop`] to receive
/// input and paint requests. Paint targets an XRGB8888 `Frame`
/// over the shell backbuffer; the shell presents it with softbuffer.
pub trait GameLoop {
    /// A lane key changed state (`down` = press).
    fn lane_key(&mut self, lane: usize, down: bool);
    /// A menu key changed state (`down` = press).
    fn menu_key(&mut self, _key: MenuKey, _down: bool) {}
    /// Paint the current frame into the backbuffer view.
    fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>);
}

/// Backbuffer words (`DEFAULT_WIDTH` x `DEFAULT_HEIGHT` XRGB8888).
const BACKBUFFER_WORDS: usize = DEFAULT_WIDTH as usize * DEFAULT_HEIGHT as usize;

/// Minimal winit shell: one window, keyboard routed to lanes,
/// game-painted backbuffer presented on demand. Constructed
/// headless-safe; [`Shell::run`] needs a display and never returns
/// until the window closes.
pub struct Shell {
    keymap: Keymap,
    window: Option<Arc<Window>>,
    game: Box<dyn GameLoop>,
    pixels: Vec<u32>,
}

impl Shell {
    /// Create a shell around a game loop. No window yet.
    pub fn new(game: Box<dyn GameLoop>) -> Self {
        Self {
            keymap: Keymap::new(),
            window: None,
            game,
            pixels: vec![0; BACKBUFFER_WORDS],
        }
    }

    /// Run the game painter over a caller buffer (headless-safe).
    pub fn paint_into(&mut self, buf: &mut [u32], w: usize, h: usize) {
        let mut frame = o2jam_render::Frame { buf, w, h };
        self.game.paint(&mut frame);
    }

    /// Paint the next frame into the backbuffer.
    fn paint_backbuffer(&mut self) {
        let mut frame = o2jam_render::Frame {
            buf: &mut self.pixels,
            w: DEFAULT_WIDTH as usize,
            h: DEFAULT_HEIGHT as usize,
        };
        self.game.paint(&mut frame);
    }

    /// Open the window and run the event loop.
    pub fn run(self, event_loop: winit::event_loop::EventLoop<()>) -> anyhow::Result<()> {
        use std::num::NonZeroU32;
        use winit::application::ApplicationHandler;
        use winit::event_loop::OwnedDisplayHandle;
        struct Handler {
            shell: Shell,
            surface: Option<softbuffer::Surface<OwnedDisplayHandle, Arc<Window>>>,
            surface_size: Option<(u32, u32)>,
            paint_error: Option<anyhow::Error>,
        }
        impl Handler {
            /// Stretch the backbuffer over the window buffer (softbuffer
            /// draws mismatched buffers unscaled, so the client scales
            /// itself and every window size shows the whole frame).
            fn present(&mut self, event_loop: &ActiveEventLoop) {
                let Some(surface) = self.surface.as_mut() else {
                    return;
                };
                let pixels = &self.shell.pixels;
                match surface.buffer_mut() {
                    Ok(mut buffer) => {
                        let Some((width, height)) = self.surface_size else {
                            return;
                        };
                        if buffer.len() != width as usize * height as usize {
                            return;
                        }
                        o2jam_render::stretch_blit(
                            &mut buffer,
                            width as usize,
                            height as usize,
                            pixels,
                            DEFAULT_WIDTH as usize,
                            DEFAULT_HEIGHT as usize,
                        );
                        if let Err(error) = buffer.present() {
                            self.paint_error = Some(anyhow::anyhow!("present failed: {error:?}"));
                            event_loop.exit();
                        }
                    }
                    Err(error) => {
                        self.paint_error = Some(anyhow::anyhow!("frame buffer failed: {error:?}"));
                        event_loop.exit();
                    }
                }
            }

            /// Resize the surface to a window size.
            fn resize_surface(&mut self, width: u32, height: u32, event_loop: &ActiveEventLoop) {
                let Some(surface) = self.surface.as_mut() else {
                    return;
                };
                let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height))
                else {
                    return;
                };
                if let Err(error) = surface.resize(width, height) {
                    self.paint_error = Some(anyhow::anyhow!("surface resize failed: {error:?}"));
                    event_loop.exit();
                } else {
                    self.surface_size = Some((width.get(), height.get()));
                }
            }
        }
        impl ApplicationHandler for Handler {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                let attrs = WindowAttributes::default()
                    .with_title("recomp2jam")
                    .with_inner_size(winit::dpi::PhysicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT));
                let window = Arc::new(
                    event_loop
                        .create_window(attrs)
                        .expect("window creation failed"),
                );
                match softbuffer::Context::new(event_loop.owned_display_handle()) {
                    Ok(context) => match softbuffer::Surface::new(&context, window.clone()) {
                        Ok(mut surface) => {
                            let size = window.inner_size();
                            if let (Some(width), Some(height)) =
                                (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                            {
                                if surface.resize(width, height).is_ok() {
                                    self.surface_size = Some((width.get(), height.get()));
                                    self.surface = Some(surface);
                                }
                            }
                            self.shell.window = Some(window.clone());
                        }
                        Err(error) => {
                            self.paint_error = Some(anyhow::anyhow!("surface failed: {error:?}"));
                            event_loop.exit();
                        }
                    },
                    Err(error) => {
                        self.paint_error = Some(anyhow::anyhow!("display failed: {error:?}"));
                        event_loop.exit();
                    }
                }
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
                        } else if let Some(menu) = menu_of(code) {
                            self.shell.game.menu_key(menu, down);
                        }
                    }
                    WindowEvent::Resized(size) => {
                        self.resize_surface(size.width, size.height, event_loop);
                    }
                    WindowEvent::RedrawRequested => {
                        self.shell.paint_backbuffer();
                        self.present(event_loop);
                    }
                    _ => {}
                }
            }
            fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
                if let Some(window) = self.shell.window.as_ref() {
                    window.request_redraw();
                }
            }
        }
        let mut handler = Handler {
            shell: self,
            surface: None,
            surface_size: None,
            paint_error: None,
        };
        event_loop
            .run_app(&mut handler)
            .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
        if let Some(error) = handler.paint_error {
            return Err(error);
        }
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

    struct SolidGame(u32);
    impl GameLoop for SolidGame {
        fn lane_key(&mut self, _lane: usize, _down: bool) {}
        fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
            frame.buf.fill(self.0);
        }
    }

    #[test]
    fn paint_into_runs_game_over_caller_buffer() {
        let mut shell = Shell::new(Box::new(SolidGame(0x0011_2233)));
        let mut buf = vec![0u32; 64];
        shell.paint_into(&mut buf, 8, 8);
        assert!(buf.iter().all(|pixel| *pixel == 0x0011_2233));
    }

    #[test]
    fn backbuffer_matches_verified_mode() {
        let shell = Shell::new(Box::new(SolidGame(0)));
        assert_eq!(shell.pixels.len(), 800 * 600);
    }

    #[test]
    fn frame_clock_saturates_on_huge_deltas() {
        let mut clock = FrameClock::new();
        let (ticks, alpha) = clock.advance(u64::MAX);
        assert!(ticks > 0);
        assert!((0.0..1.0).contains(&alpha));
    }
}
