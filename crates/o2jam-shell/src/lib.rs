//! `o2jam-shell` — window, keyboard input, and the fixed-timestep
//! frame loop for the game client. Scene staging and the OS surface:
//! the staging declaration says what each State stages; game logic
//! lives in `o2jam-game`, audio time in `o2jam-audio`. Headless tests
//! cover the key map and the timestep math; window creation needs
//! hardware.

pub mod avatar;
pub mod input_latency;
pub mod options;
pub mod playing;
pub mod screen;
pub mod screens;
pub mod song_clock_replay;
pub mod staging;
mod window;

pub use avatar::{
    load_archive_sprite, load_worn_figure, pick_wear_sprites, AvatarFigure, FigureLayer,
};
pub use playing::{HighwayGeometry, LivePlay, PlaySkin, PlayingView};
pub use screen::{can_enter, BadTransition, Screen, ScreenFlow};
pub use screens::{paint_screen, ScreenGame};

use std::collections::HashSet;
use std::sync::Arc;
use winit::keyboard::KeyCode;
use winit::window::Window;

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
/// the club-standard layout) plus menu keys. The binds are
/// rebindable: the window syncs them from the game loop before
/// every key event, so the options Key tab rebinds answer in the
/// lanes without touching the default map.
#[derive(Debug)]
pub struct Keymap {
    held: HashSet<KeyCode>,
    binds: [KeyCode; LANE_COUNT],
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            held: HashSet::new(),
            binds: default_lane_binds(),
        }
    }
}

impl Keymap {
    /// Create an empty key map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the lane binds (the options Key tab owns these;
    /// the window syncs them from the game loop per key event).
    pub fn set_binds(&mut self, binds: [KeyCode; LANE_COUNT]) {
        self.binds = binds;
    }

    /// Lane index for a play key under the current binds, if any.
    #[must_use]
    pub fn lane_of_code(&self, code: KeyCode) -> Option<usize> {
        self.binds.iter().position(|bind| *bind == code)
    }

    /// Record a key press; returns the lane when a lane key goes down.
    pub fn press(&mut self, code: KeyCode) -> Option<usize> {
        self.held.insert(code);
        self.lane_of_code(code)
    }

    /// Record a key release; returns the lane when a lane key goes up.
    pub fn release(&mut self, code: &KeyCode) -> Option<usize> {
        self.held.remove(code);
        self.lane_of_code(*code)
    }

    /// Whether a lane key is currently held.
    #[must_use]
    pub fn lane_held(&self, lane: usize) -> bool {
        self.binds
            .get(lane)
            .is_some_and(|code| self.held.contains(code))
    }
}

/// Default lane binds (mirrors [`lane_key`]).
fn default_lane_binds() -> [KeyCode; LANE_COUNT] {
    let mut binds = [KeyCode::KeyS; LANE_COUNT];
    for (lane, bind) in binds.iter_mut().enumerate() {
        if let Some(code) = lane_key(lane) {
            *bind = code;
        }
    }
    binds
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
    /// Highlight up (`ArrowUp`).
    Up,
    /// Highlight down (`ArrowDown`).
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
    /// A raw physical key changed state before lane/menu routing:
    /// `true` consumes the event (the options Key tab uses this to
    /// capture the next press for a lane rebind). Defaults to no
    /// capture so existing implementors keep compiling.
    fn capture_raw_key(&mut self, _code: KeyCode, _down: bool) -> bool {
        false
    }
    /// Current lane binds, when the game owns rebindable keys (the
    /// window syncs its key map from these per key event). `None`
    /// keeps the default S D F Space J K L map.
    fn lane_bindings(&self) -> Option<[KeyCode; LANE_COUNT]> {
        None
    }
    /// A lane key changed state (`down` = press).
    fn lane_key(&mut self, lane: usize, down: bool);
    /// A menu key changed state (`down` = press).
    fn menu_key(&mut self, _key: MenuKey, _down: bool) {}
    /// Committed text arrived (winit `Ime::Commit`); chat drafts consume it.
    fn text_input(&mut self, _text: &str) {}
    /// The Backspace key went down; chat drafts consume it as erase.
    fn delete_text(&mut self) {}
    /// A left mouse click at native frame pixels (`x`, `y` in
    /// 800x600 space; the shell maps window pixels before calling).
    /// Default ignores clicks so existing implementors keep compiling.
    fn mouse_click(&mut self, _x: i32, _y: i32) {}
    /// The pointer moved to native frame pixels (`x`, `y` in 800x600
    /// space; the shell maps window pixels before calling). Hover
    /// state rides this — the exe arms hover art on mouse motion alone
    /// (`Dialog_ChannelSelectBar.ojs` follows the row under the
    /// pointer, live-confirmed 2026-11-09). Default ignores motion so
    /// existing implementors keep compiling.
    fn mouse_moved(&mut self, _x: i32, _y: i32) {}
    /// Whether the game asked to quit (the planet quit-confirm OK).
    /// The windowed shell exits the event loop on it; headless walks
    /// read the flag instead. Default never quits so existing
    /// implementors keep compiling.
    fn quit_requested(&self) -> bool {
        false
    }
    /// Name of the screen currently shown (`"unknown"` by default):
    /// the capture log's transition marker, so input traces replayed
    /// through the real window can sync on scene changes. Test
    /// affordance, not gameplay.
    fn screen_name(&self) -> &'static str {
        "unknown"
    }
    /// Hash of the observable game state (screen, drafts, histories,
    /// live score): the trace harness's final-state evidence. Default
    /// zero (no observable state) so existing implementors keep
    /// compiling. Test affordance, not gameplay.
    fn state_hash(&self) -> u64 {
        0
    }
    /// Paint the current frame into the backbuffer view.
    fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>);
}

/// Trace-harness capture plan for a windowed run (test affordance):
/// per-paint input/screen/hash lines plus gated frame dumps.
#[derive(Debug, Clone)]
pub struct CapturePlan {
    /// Directory for `frame-NNNNNN-<screen>.png` captures.
    pub dir: std::path::PathBuf,
    /// Minimum paints between dumps while the frame keeps changing
    /// (bounds Playing-scroll capture volume).
    pub min_paint_gap: u64,
    /// Force a dump when no content change happened for this many
    /// paints (menus at rest still leave a heartbeat trail).
    pub heartbeat_paints: u64,
    /// Optional input log path: one line per routed event, one
    /// `frame` line per paint (applied wall ms, screen, state hash,
    /// content hash).
    pub input_log: Option<std::path::PathBuf>,
}

/// Prepared capture state: the plan plus its opened log and clock.
pub(crate) struct CaptureState {
    /// Frame-dump directory (created at attach time).
    pub dir: std::path::PathBuf,
    /// Minimum paints between content-change dumps.
    pub min_paint_gap: u64,
    /// Force-dump interval in paints.
    pub heartbeat_paints: u64,
    /// Opened applied-event log (`None` when the file refused).
    pub log: Option<std::io::BufWriter<std::fs::File>>,
    /// Wall-clock anchor for the log's `wall_ms` stamps.
    pub started: std::time::Instant,
}

/// Backbuffer words (`DEFAULT_WIDTH` x `DEFAULT_HEIGHT` XRGB8888).
const BACKBUFFER_WORDS: usize = DEFAULT_WIDTH as usize * DEFAULT_HEIGHT as usize;

/// Minimal winit shell: one window, keyboard routed to lanes,
/// game-painted backbuffer presented on demand. Constructed
/// headless-safe; [`Shell::run`] needs a display and never returns
/// until the window closes.
pub struct Shell {
    pub(crate) keymap: Keymap,
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) game: Box<dyn GameLoop>,
    pub(crate) pixels: Vec<u32>,
    pub(crate) capture: Option<CaptureState>,
}

impl Shell {
    /// Create a shell around a game loop. No window yet.
    pub fn new(game: Box<dyn GameLoop>) -> Self {
        Self {
            keymap: Keymap::new(),
            window: None,
            game,
            pixels: vec![0; BACKBUFFER_WORDS],
            capture: None,
        }
    }

    /// Attach a trace-harness capture plan (frame dumps plus an
    /// applied-event log); consumed by the window handler. Failures
    /// to create the directory or open the log degrade to no-capture
    /// (the game loop never depends on evidence plumbing).
    pub fn with_capture(mut self, plan: CapturePlan) -> Self {
        if let Err(source) = std::fs::create_dir_all(&plan.dir) {
            tracing::warn!(%source, "frame-dump directory unusable; dumps disabled");
        }
        let log = plan
            .input_log
            .as_ref()
            .and_then(|path| std::fs::File::create(path).ok())
            .map(std::io::BufWriter::new);
        self.capture = Some(CaptureState {
            dir: plan.dir,
            min_paint_gap: plan.min_paint_gap.max(1),
            heartbeat_paints: plan.heartbeat_paints.max(1),
            log,
            started: std::time::Instant::now(),
        });
        self
    }

    /// Run the game painter over a caller buffer (headless-safe).
    pub fn paint_into(&mut self, buf: &mut [u32], w: usize, h: usize) {
        let mut frame = o2jam_render::Frame { buf, w, h };
        self.game.paint(&mut frame);
    }

    /// Paint the next frame into the backbuffer.
    pub(crate) fn paint_backbuffer(&mut self) {
        let mut frame = o2jam_render::Frame {
            buf: &mut self.pixels,
            w: DEFAULT_WIDTH as usize,
            h: DEFAULT_HEIGHT as usize,
        };
        self.game.paint(&mut frame);
    }

    /// Open the window and run the event loop.
    ///
    /// # Errors
    ///
    /// Paint-backend failures and event-loop errors.
    pub fn run(self, event_loop: winit::event_loop::EventLoop<()>) -> anyhow::Result<()> {
        let mut handler = window::WindowHandler::new(self);
        event_loop
            .run_app(&mut handler)
            .map_err(|e| anyhow::anyhow!("event loop failed: {e:?}"))?;
        if let Some(error) = handler.paint_error() {
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
