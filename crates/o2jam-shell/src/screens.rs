//! Screen painters: one flat-color scene per [`Screen`].
//!
//! Placeholder art until the OPI sprite pipeline lands: each screen
//! gets a distinct background plus a centered card, all via
//! [`fill_rect`] (clipped, so any buffer size is safe). Layouts are
//! relative to the frame so headless tests can paint tiny buffers.
//! [`ScreenGame`] owns a [`ScreenFlow`] and advances splash → channel
//! on any lane key; later screens wire their own inputs next.

use super::screen::{Screen, ScreenFlow};
use super::GameLoop;
use o2jam_render::{fill_rect, pack_xrgb, Frame, Rect};

/// Paint the splash (logo) scene.
fn paint_avi(frame: &mut Frame<'_>) {
    paint_card(
        frame,
        pack_xrgb(0x08, 0x08, 0x10),
        pack_xrgb(0x20, 0x60, 0xC0),
    );
}

/// Paint a lobby scene (channel / room / waiting share the shape).
fn paint_lobby(frame: &mut Frame<'_>, background: u32) {
    paint_card(frame, background, pack_xrgb(0xD0, 0xD0, 0xD0));
}

/// Paint the loading gate scene.
fn paint_loading(frame: &mut Frame<'_>) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        pack_xrgb(0x18, 0x18, 0x18),
    );
    fill_rect(
        frame,
        &Rect {
            x: w / 4,
            y: h / 2 - 8,
            w: (w / 2) as u32,
            h: 16,
        },
        pack_xrgb(0x30, 0xB0, 0x30),
    );
}

/// Paint the results scene.
fn paint_result(frame: &mut Frame<'_>) {
    paint_card(
        frame,
        pack_xrgb(0x10, 0x10, 0x08),
        pack_xrgb(0xC0, 0xA0, 0x20),
    );
}

/// Background plus a centered card.
fn paint_card(frame: &mut Frame<'_>, background: u32, card: u32) {
    let (w, h) = (frame.w as i32, frame.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: w as u32,
            h: h as u32,
        },
        background,
    );
    fill_rect(
        frame,
        &Rect {
            x: w / 2 - w / 8,
            y: h / 2 - h / 8,
            w: (w / 4) as u32,
            h: (h / 4) as u32,
        },
        card,
    );
}

/// Background color per screen (lobbies share the card shape).
fn background_of(screen: Screen) -> u32 {
    match screen {
        Screen::Avi => pack_xrgb(0x08, 0x08, 0x10),
        Screen::Channel => pack_xrgb(0x10, 0x20, 0x50),
        Screen::Room => pack_xrgb(0x10, 0x50, 0x20),
        Screen::Waiting => pack_xrgb(0x50, 0x40, 0x10),
        Screen::Loading => pack_xrgb(0x18, 0x18, 0x18),
        Screen::Playing => pack_xrgb(0x00, 0x00, 0x00),
        Screen::Result => pack_xrgb(0x10, 0x10, 0x08),
    }
}

/// Paint one screen's scene.
pub fn paint_screen(frame: &mut Frame<'_>, screen: Screen) {
    match screen {
        Screen::Avi => paint_avi(frame),
        Screen::Channel => paint_lobby(frame, background_of(screen)),
        Screen::Room => paint_lobby(frame, background_of(screen)),
        Screen::Waiting => paint_lobby(frame, background_of(screen)),
        Screen::Loading => paint_loading(frame),
        Screen::Playing => paint_card(frame, background_of(screen), pack_xrgb(0x40, 0x40, 0x40)),
        Screen::Result => paint_result(frame),
    }
}

/// Game loop that shows each screen's scene and walks the flow.
///
/// Input wiring so far: any lane key dismisses the splash into the
/// channel screen. Room/waiting/loading inputs land with their UIs.
pub struct ScreenGame {
    flow: ScreenFlow,
}

impl ScreenGame {
    /// Start at the splash screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flow: ScreenFlow::new(),
        }
    }

    /// Screen currently shown.
    #[must_use]
    pub fn screen(&self) -> Screen {
        self.flow.current()
    }
}

impl Default for ScreenGame {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLoop for ScreenGame {
    fn lane_key(&mut self, _lane: usize, down: bool) {
        if down && self.flow.current() == Screen::Avi {
            let _ = self.flow.go(Screen::Channel);
        }
    }

    fn paint(&mut self, frame: &mut Frame<'_>) {
        paint_screen(frame, self.flow.current());
    }
}

#[cfg(test)]
mod screen_paint_tests {
    use super::*;

    fn painted(screen: Screen, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        paint_screen(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            screen,
        );
        buf
    }

    #[test]
    fn splash_covers_background_with_card() {
        let buf = painted(Screen::Avi, 32, 24);
        assert_eq!(buf[0], pack_xrgb(0x08, 0x08, 0x10));
        assert_eq!(buf[12 * 32 + 16], pack_xrgb(0x20, 0x60, 0xC0));
    }

    #[test]
    fn every_screen_paints_its_background() {
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let buf = painted(screen, 16, 12);
            assert_eq!(buf[0], background_of(screen), "{screen:?}");
        }
    }

    #[test]
    fn lane_key_dismisses_splash_into_channel() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Channel);
        let mut buf = vec![0u32; 16 * 12];
        game.paint(&mut Frame {
            buf: &mut buf,
            w: 16,
            h: 12,
        });
        assert_eq!(buf[0], background_of(Screen::Channel));
    }

    #[test]
    fn lane_release_keeps_splash() {
        let mut game = ScreenGame::new();
        game.lane_key(0, false);
        assert_eq!(game.screen(), Screen::Avi);
    }

    #[test]
    fn paint_is_safe_on_empty_frame() {
        let mut buf = vec![];
        paint_screen(
            &mut Frame {
                buf: &mut buf,
                w: 0,
                h: 0,
            },
            Screen::Playing,
        );
    }
}
