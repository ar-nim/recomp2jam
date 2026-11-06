//! winit event handling: window open, input routing, presentation.
//!
//! [`WindowHandler`] owns the [`Shell`] once the event loop runs;
//! [`Shell::run`](super::Shell::run) only builds it and checks the
//! paint error afterwards.

use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, OwnedDisplayHandle};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowAttributes};

use super::{menu_of, Shell, DEFAULT_HEIGHT, DEFAULT_WIDTH};

/// Event-loop state: shell plus its softbuffer surface.
pub struct WindowHandler {
    shell: Shell,
    surface: Option<softbuffer::Surface<OwnedDisplayHandle, Arc<Window>>>,
    surface_size: Option<(u32, u32)>,
    paint_error: Option<anyhow::Error>,
}

impl WindowHandler {
    /// Wrap a shell; no window or surface yet.
    pub fn new(shell: Shell) -> Self {
        Self {
            shell,
            surface: None,
            surface_size: None,
            paint_error: None,
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

    /// Take a stored paint failure, if any.
    pub fn paint_error(&mut self) -> Option<anyhow::Error> {
        self.paint_error.take()
    }
}

/// Letterbox bar color: black bars around the aspect-fit image.
pub const LETTERBOX: u32 = 0x000000;

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
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => self.route_key(code, state == ElementState::Pressed),
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

#[cfg(test)]
mod tests {
    use super::*;

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
