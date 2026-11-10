//! winit event handling: window open, input routing, presentation.
//!
//! [`WindowHandler`] owns the [`Shell`] once the event loop runs;
//! [`Shell::run`](super::Shell::run) only builds it and checks the
//! paint error afterwards.

use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, OwnedDisplayHandle};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

use super::{menu_of, CaptureState, Shell, DEFAULT_HEIGHT, DEFAULT_WIDTH};

/// Trace-capture state owned by the handler: paint counting, last
/// screen/content fingerprints, and the last dumped paint.
struct CaptureSink {
    state: CaptureState,
    paint: u64,
    last_screen: &'static str,
    last_content_hash: u64,
    last_dump_paint: u64,
}

/// Event-loop state: shell plus its softbuffer surface.
pub struct WindowHandler {
    shell: Shell,
    surface: Option<softbuffer::Surface<OwnedDisplayHandle, Arc<Window>>>,
    surface_size: Option<(u32, u32)>,
    paint_error: Option<anyhow::Error>,
    cursor: Option<(f64, f64)>,
    capture: Option<CaptureSink>,
}

impl WindowHandler {
    /// Wrap a shell; no window or surface yet. A capture plan on the
    /// shell becomes the trace sink (frame dumps + applied-event log).
    pub fn new(mut shell: Shell) -> Self {
        let capture = shell.capture.take().map(|state| CaptureSink {
            state,
            paint: 0,
            last_screen: "",
            last_content_hash: 0,
            last_dump_paint: 0,
        });
        Self {
            shell,
            surface: None,
            surface_size: None,
            paint_error: None,
            cursor: None,
            capture,
        }
    }

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
                let Some(layout) = present_layout(width, height, buffer.len(), pixels.len()) else {
                    return;
                };
                let Ok(window_w) = usize::try_from(layout.window_w) else {
                    return;
                };
                let Ok(window_h) = usize::try_from(layout.window_h) else {
                    return;
                };
                if !o2jam_render::present_native_frame(
                    &mut buffer,
                    window_w,
                    window_h,
                    pixels,
                    LETTERBOX,
                ) {
                    return;
                }
                // The framebuffer words are XRGB (alpha byte 0, see
                // `pack_xrgb`); compositors that honor the alpha byte
                // (Wayland ARGB shm) would blend the window with the
                // desktop — white speckles, transparent letterbox bars.
                // Force every presented pixel opaque.
                for word in buffer.iter_mut() {
                    *word |= 0xFF00_0000;
                }
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
        let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
            return;
        };
        if let Err(error) = surface.resize(width, height) {
            self.paint_error = Some(anyhow::anyhow!("surface resize failed: {error:?}"));
            event_loop.exit();
        } else {
            self.surface_size = Some((width.get(), height.get()));
        }
    }

    /// Open the window and attach a sized surface to it.
    fn attach_window(&mut self, event_loop: &ActiveEventLoop) {
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

    /// Route one key event to a lane or menu hook.
    fn route_key(&mut self, code: winit::keyboard::KeyCode, down: bool) {
        self.log_event(&format!(
            "key-{} {code:?}",
            if down { "down" } else { "up" }
        ));
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

    /// Route a left press at window pixels into a native-frame
    /// click (letterbox bars never click through).
    fn route_click(&mut self, cursor_x: f64, cursor_y: f64) {
        let Some((window_w, window_h)) = self.surface_size else {
            return;
        };
        if let Some((x, y)) = window_to_native(window_w, window_h, cursor_x, cursor_y) {
            self.log_event(&format!("click {x} {y}"));
            self.shell.game.mouse_click(x, y);
        }
    }

    /// Feed committed text into the game (winit `Ime::Commit`, or a
    /// pressed key's produced text when no input method owns the
    /// key — Xvfb rigs have no XIM, so chat typing arrives this way).
    /// Control characters never reach the log or the game: Enter's
    /// produced `\r` would poison the capture log's line format.
    fn route_text(&mut self, text: &str) {
        let clean: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if clean.is_empty() {
            return;
        }
        for ch in clean.chars() {
            self.log_event(&format!("text {ch}"));
        }
        self.shell.game.text_input(&clean);
    }

    /// Erase one draft character (Backspace press).
    fn route_erase(&mut self) {
        self.log_event("erase");
        self.shell.game.delete_text();
    }

    /// Append one applied-event line to the capture log (no-op
    /// without a log): the trace harness matches these against the
    /// injected trace events.
    fn log_event(&mut self, event: &str) {
        let Some(capture) = self.capture.as_mut() else {
            return;
        };
        let wall_ms = capture.state.started.elapsed().as_millis();
        if let Some(log) = capture.state.log.as_mut() {
            use std::io::Write;
            let _ = writeln!(log, "event {wall_ms} {event}");
        }
    }

    /// FNV-1a over the backbuffer words: the content fingerprint
    /// gating frame dumps.
    fn content_hash(pixels: &[u32]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for word in pixels {
            for shift in (0..32).step_by(8) {
                hash ^= u64::from((*word >> shift) as u8);
                hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
            }
        }
        hash
    }

    /// Record one painted frame in the capture log and dump it when
    /// the plan says so: on a screen change or content change after
    /// `min_paint_gap` paints, or on the heartbeat interval either
    /// way. Evidence volume stays bounded during Playing scroll.
    fn capture_frame(&mut self) {
        let Some(capture) = self.capture.as_mut() else {
            return;
        };
        capture.paint = capture.paint.saturating_add(1);
        let content = Self::content_hash(&self.shell.pixels);
        let screen = self.shell.game.screen_name();
        let wall_ms = capture.state.started.elapsed().as_millis();
        let state = self.shell.game.state_hash();
        if let Some(log) = capture.state.log.as_mut() {
            use std::io::Write;
            let _ = writeln!(
                log,
                "frame {0} wall_ms={wall_ms} screen={screen} state={state:016x} content={content:016x}",
                capture.paint
            );
        }
        let since_dump = capture.paint.saturating_sub(capture.last_dump_paint);
        let screen_changed = screen != capture.last_screen;
        let content_changed = content != capture.last_content_hash;
        let due = (screen_changed
            || (content_changed && since_dump >= capture.state.min_paint_gap))
            || since_dump >= capture.state.heartbeat_paints;
        capture.last_screen = screen;
        capture.last_content_hash = content;
        if !due {
            return;
        }
        capture.last_dump_paint = capture.paint;
        let rgb = xrgb_to_rgb(&self.shell.pixels);
        let png = o2jam_render::png::encode_png(DEFAULT_WIDTH, DEFAULT_HEIGHT, &rgb);
        let name = format!("frame-{:06}-{screen}.png", capture.paint);
        let path = capture.state.dir.join(name);
        if let Err(source) = std::fs::write(&path, png) {
            tracing::warn!(%source, "frame dump failed");
        }
    }

    /// Take a stored paint failure, if any.
    pub fn paint_error(&mut self) -> Option<anyhow::Error> {
        self.paint_error.take()
    }
}

/// Letterbox bar color: black bars around the aspect-fit image.
pub const LETTERBOX: u32 = 0x000000;

/// Strip XRGB backbuffer words down to RGB triplets for PNG dumps.
fn xrgb_to_rgb(buf: &[u32]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(buf.len() * 3);
    for pixel in buf {
        rgb.push((pixel >> 16) as u8);
        rgb.push((pixel >> 8) as u8);
        rgb.push(*pixel as u8);
    }
    rgb
}

/// Validated present geometry: the centered uniform-scale draw rect
/// of the native backbuffer inside the window buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentLayout {
    /// Window-buffer width in pixels.
    pub window_w: u32,
    /// Window-buffer height in pixels.
    pub window_h: u32,
    /// Left offset of the scaled image inside the window.
    pub offset_x: u32,
    /// Top offset of the scaled image inside the window.
    pub offset_y: u32,
    /// Scaled image width (uniform scale, aspect preserved).
    pub draw_w: u32,
    /// Scaled image height (uniform scale, aspect preserved).
    pub draw_h: u32,
}

/// Backbuffer words for the native 800x600 canvas.
#[must_use]
pub fn backbuffer_words() -> usize {
    DEFAULT_WIDTH as usize * DEFAULT_HEIGHT as usize
}

/// Map a window physical-pixel cursor into native 800x600 frame
/// pixels: inverts the aspect-fit present (`None` on degenerate
/// geometry or outside the draw rect, so letterbox bars never
/// click through).
#[must_use]
pub fn window_to_native(
    window_w: u32,
    window_h: u32,
    cursor_x: f64,
    cursor_y: f64,
) -> Option<(i32, i32)> {
    let fit = o2jam_render::native_present(
        usize::try_from(window_w).ok()?,
        usize::try_from(window_h).ok()?,
    )?;
    let draw_w = fit.draw_w as f64;
    let draw_h = fit.draw_h as f64;
    if draw_w <= 0.0 || draw_h <= 0.0 {
        return None;
    }
    let native_w = f64::from(DEFAULT_WIDTH);
    let native_h = f64::from(DEFAULT_HEIGHT);
    let local_x = (cursor_x - fit.offset_x as f64) * native_w / draw_w;
    let local_y = (cursor_y - fit.offset_y as f64) * native_h / draw_h;
    if !(0.0..native_w).contains(&local_x) || !(0.0..native_h).contains(&local_y) {
        return None;
    }
    Some((local_x.floor() as i32, local_y.floor() as i32))
}

/// Whether a backbuffer holds exactly one native frame.
#[must_use]
pub fn backbuffer_ready(pixels_len: usize) -> bool {
    pixels_len == backbuffer_words()
}

/// Validate a present: nonzero window size, a window buffer that
/// exactly covers it, and a complete native backbuffer. The draw
/// rect is the largest aspect-preserving fit of 800x600, centered
/// with letterbox bars. `None` means skip the frame rather than
/// show a torn one.
#[must_use]
pub fn present_layout(
    width: u32,
    height: u32,
    window_buffer_len: usize,
    backbuffer_len: usize,
) -> Option<PresentLayout> {
    let window_w = usize::try_from(width).ok()?;
    let window_h = usize::try_from(height).ok()?;
    let window_len = window_w.checked_mul(window_h)?;
    if window_len == 0 || window_buffer_len != window_len {
        return None;
    }
    if !backbuffer_ready(backbuffer_len) {
        return None;
    }
    let fit = o2jam_render::native_present(window_w, window_h)?;
    Some(PresentLayout {
        window_w: width,
        window_h: height,
        offset_x: u32::try_from(fit.offset_x).ok()?,
        offset_y: u32::try_from(fit.offset_y).ok()?,
        draw_w: u32::try_from(fit.draw_w).ok()?,
        draw_h: u32::try_from(fit.draw_h).ok()?,
    })
}

impl ApplicationHandler for WindowHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.attach_window(event_loop);
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
                        physical_key: PhysicalKey::Code(KeyCode::Backspace),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => self.route_erase(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        text,
                        ..
                    },
                ..
            } => {
                // Typed characters reach the chat drafts through the
                // key's produced text when no input method owns the
                // key (Xvfb rigs have no XIM; real IMEs commit through
                // `Ime::Commit` above and never double-feed here).
                if state == ElementState::Pressed {
                    if let Some(text) = text.filter(|text| !text.is_empty()) {
                        self.route_text(&text);
                    }
                }
                self.route_key(code, state == ElementState::Pressed);
            }
            WindowEvent::Ime(Ime::Commit(text)) => self.route_text(&text),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((x, y)) = self.cursor {
                    self.route_click(x, y);
                }
            }
            WindowEvent::Resized(size) => {
                self.resize_surface(size.width, size.height, event_loop);
            }
            WindowEvent::RedrawRequested => {
                self.shell.paint_backbuffer();
                self.present(event_loop);
                self.capture_frame();
                // Planet quit-confirm OK (`GameLoop::quit_requested`):
                // the game asked to leave, so the loop exits.
                if self.shell.game.quit_requested() {
                    event_loop.exit();
                }
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

#[cfg(test)]
mod tests {
    use super::super::GameLoop;
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Game probe recording routed input for headless key-event tests.
    #[derive(Default)]
    struct InputProbe {
        texts: Vec<String>,
        deletes: usize,
        lanes: Vec<(usize, bool)>,
        clicks: Vec<(i32, i32)>,
    }

    impl GameLoop for Rc<RefCell<InputProbe>> {
        fn lane_key(&mut self, lane: usize, down: bool) {
            self.borrow_mut().lanes.push((lane, down));
        }

        fn text_input(&mut self, text: &str) {
            self.borrow_mut().texts.push(text.to_owned());
        }

        fn delete_text(&mut self) {
            self.borrow_mut().deletes += 1;
        }

        fn mouse_click(&mut self, x: i32, y: i32) {
            self.borrow_mut().clicks.push((x, y));
        }

        fn paint(&mut self, _frame: &mut o2jam_render::Frame<'_>) {}
    }

    /// A handler plus a live handle on its recording game.
    fn probed_handler() -> (WindowHandler, Rc<RefCell<InputProbe>>) {
        let probe = Rc::new(RefCell::new(InputProbe::default()));
        let shell = Shell::new(Box::new(probe.clone()));
        (WindowHandler::new(shell), probe)
    }

    #[test]
    fn committed_text_reaches_the_game() {
        let (mut handler, probe) = probed_handler();
        handler.route_text("hello");
        assert_eq!(probe.borrow().texts, vec!["hello".to_owned()]);
    }

    #[test]
    fn capture_log_records_events_frames_and_dumps() {
        let dir = std::env::temp_dir().join(format!("recomp2jam-capture-{}", std::process::id()));
        // The caller owns the output root (the replay driver creates
        // --out first); the shell only creates the frames directory.
        std::fs::create_dir_all(&dir).expect("output root");
        let log_path = dir.join("input.log");
        let probe = Rc::new(RefCell::new(InputProbe::default()));
        let shell = Shell::new(Box::new(probe.clone())).with_capture(super::super::CapturePlan {
            dir: dir.join("frames"),
            min_paint_gap: 1,
            heartbeat_paints: 2,
            input_log: Some(log_path.clone()),
        });
        let mut handler = WindowHandler::new(shell);
        handler.route_key(KeyCode::KeyS, true);
        handler.route_click(100.0, 200.0);
        handler.shell.paint_backbuffer();
        handler.capture_frame();
        handler.shell.paint_backbuffer();
        handler.capture_frame();
        // Flush the log by dropping the handler, then read it back.
        let frames_dir = dir.join("frames");
        drop(handler);
        let text = std::fs::read_to_string(&log_path).expect("input log written");
        assert!(
            text.contains("key-down KeyS"),
            "log has the routed key: {text}"
        );
        assert!(text.contains("frame 1 "), "log has the first paint: {text}");
        assert!(
            text.contains("frame 2 "),
            "log has the second paint: {text}"
        );
        let dumps: Vec<_> = std::fs::read_dir(&frames_dir)
            .expect("frames dumped")
            .flatten()
            .collect();
        assert!(!dumps.is_empty(), "at least one frame captured");
        for dump in &dumps {
            assert!(dump.file_name().to_string_lossy().ends_with("-unknown.png"));
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn erase_reaches_the_game() {
        let (mut handler, probe) = probed_handler();
        handler.route_erase();
        handler.route_erase();
        assert_eq!(probe.borrow().deletes, 2);
    }

    #[test]
    fn lane_keys_still_reach_their_lane() {
        let (mut handler, probe) = probed_handler();
        handler.route_key(KeyCode::KeyS, true);
        handler.route_key(KeyCode::KeyS, false);
        assert_eq!(probe.borrow().lanes, vec![(0, true), (0, false)]);
    }

    #[test]
    fn backspace_release_routes_nowhere() {
        let (mut handler, probe) = probed_handler();
        handler.route_key(KeyCode::Backspace, false);
        assert_eq!(probe.borrow().deletes, 0);
        assert!(probe.borrow().lanes.is_empty());
    }

    #[test]
    fn left_press_reaches_the_game_at_native_pixels() {
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.route_click(100.0, 200.0);
        assert_eq!(probe.borrow().clicks, vec![(100, 200)]);
    }

    #[test]
    fn clicks_need_a_sized_surface() {
        let (mut handler, probe) = probed_handler();
        handler.route_click(100.0, 200.0);
        assert!(probe.borrow().clicks.is_empty());
    }

    #[test]
    fn letterbox_bars_never_click_through() {
        // 1600x900 centers 800x600 with 200 px side bars.
        assert_eq!(window_to_native(1600, 900, 199.9, 450.0), None);
        assert_eq!(window_to_native(1600, 900, 1400.0, 450.0), None);
        assert_eq!(window_to_native(1600, 900, 200.0, 0.0), Some((0, 0)));
        assert_eq!(window_to_native(1600, 900, 1399.9, 899.9), Some((799, 599)));
    }

    #[test]
    fn window_mapping_rejects_degenerate_geometry() {
        assert_eq!(window_to_native(0, 600, 0.0, 0.0), None);
        assert_eq!(window_to_native(800, 0, 0.0, 0.0), None);
    }

    #[test]
    fn present_accepts_native_backbuffer_at_native_size() {
        let layout = present_layout(800, 600, 800 * 600, backbuffer_words());
        assert_eq!(
            layout,
            Some(PresentLayout {
                window_w: 800,
                window_h: 600,
                offset_x: 0,
                offset_y: 0,
                draw_w: 800,
                draw_h: 600,
            })
        );
    }

    #[test]
    fn present_letterboxes_wide_window_with_uniform_scale() {
        let layout = present_layout(1600, 900, 1600 * 900, backbuffer_words());
        assert_eq!(
            layout,
            Some(PresentLayout {
                window_w: 1600,
                window_h: 900,
                offset_x: 200,
                offset_y: 0,
                draw_w: 1200,
                draw_h: 900,
            })
        );
    }

    #[test]
    fn present_letterboxes_tall_window_with_uniform_scale() {
        let layout = present_layout(600, 800, 600 * 800, backbuffer_words());
        assert_eq!(
            layout,
            Some(PresentLayout {
                window_w: 600,
                window_h: 800,
                offset_x: 0,
                offset_y: 175,
                draw_w: 600,
                draw_h: 450,
            })
        );
    }

    #[test]
    fn present_rejects_zero_window_sides() {
        assert_eq!(present_layout(0, 600, 0, backbuffer_words()), None);
        assert_eq!(present_layout(800, 0, 0, backbuffer_words()), None);
    }

    #[test]
    fn present_rejects_mismatched_window_buffer() {
        assert_eq!(
            present_layout(800, 600, 800 * 600 - 1, backbuffer_words()),
            None
        );
    }

    #[test]
    fn present_rejects_short_backbuffer() {
        assert!(!backbuffer_ready(backbuffer_words() - 1));
        assert!(backbuffer_ready(backbuffer_words()));
        assert_eq!(
            present_layout(800, 600, 800 * 600, backbuffer_words() - 1),
            None
        );
    }

    #[test]
    fn present_layout_covers_small_window_from_full_frame() {
        let backbuffer = vec![7u32; backbuffer_words()];
        let layout =
            present_layout(80, 60, 80 * 60, backbuffer.len()).expect("small window presents");
        assert_eq!((layout.offset_x, layout.offset_y), (0, 0));
        assert_eq!((layout.draw_w, layout.draw_h), (80, 60));
        let mut dst = vec![0u32; 80 * 60];
        let presented = o2jam_render::present_native_frame(
            &mut dst,
            usize::try_from(layout.window_w).unwrap(),
            usize::try_from(layout.window_h).unwrap(),
            &backbuffer,
            LETTERBOX,
        );
        assert!(presented);
        assert!(dst.iter().all(|pixel| *pixel == 7));
    }

    #[test]
    fn present_clears_only_bars_over_stale_pixels() {
        // Pinned presenter policy (`docs/channel-fill.md`): the native
        // backbuffer is never cleared, so present fills only the
        // letterbox bars and scales the stale image verbatim over a
        // stale window buffer.
        const STALE_WINDOW: u32 = 0x00DE_AD00;
        let backbuffer: Vec<u32> = (0..backbuffer_words())
            .map(|index| u32::try_from(index).unwrap_or(u32::MAX) | 0x00C0_C0C0)
            .collect();
        let layout =
            present_layout(1600, 900, 1600 * 900, backbuffer.len()).expect("wide window presents");
        let window_w = usize::try_from(layout.window_w).unwrap();
        let bar_end = usize::try_from(layout.offset_x).unwrap();
        let mut window = vec![STALE_WINDOW; 1600 * 900];
        let presented = o2jam_render::present_native_frame(
            &mut window,
            window_w,
            usize::try_from(layout.window_h).unwrap(),
            &backbuffer,
            LETTERBOX,
        );
        assert!(presented);
        assert_eq!(window[0], LETTERBOX);
        assert_eq!(window[bar_end - 1], LETTERBOX);
        // Image corners copy the stale frame verbatim: top-left is
        // native (0,0), bottom-right is the last native pixel.
        assert_eq!(window[bar_end], backbuffer[0]);
        let last_image = usize::try_from(layout.draw_h - 1).unwrap() * window_w
            + bar_end
            + usize::try_from(layout.draw_w - 1).unwrap();
        assert_eq!(window[last_image], backbuffer[backbuffer_words() - 1]);
        // No stale window pixel survives: bars cleared, image scaled over.
        assert!(window.iter().all(|pixel| *pixel != STALE_WINDOW));
    }

    #[test]
    fn present_paints_letterbox_bars_black() {
        let backbuffer = vec![7u32; backbuffer_words()];
        let layout =
            present_layout(1600, 900, 1600 * 900, backbuffer.len()).expect("wide window presents");
        let mut dst = vec![0u32; 1600 * 900];
        let presented = o2jam_render::present_native_frame(
            &mut dst,
            usize::try_from(layout.window_w).unwrap(),
            usize::try_from(layout.window_h).unwrap(),
            &backbuffer,
            LETTERBOX,
        );
        assert!(presented);
        assert_eq!(dst[0], LETTERBOX);
        let bar_end = usize::try_from(layout.offset_x).unwrap();
        assert!(dst[..bar_end].iter().all(|pixel| *pixel == LETTERBOX));
        assert_eq!(dst[bar_end], 7);
    }
}
