//! winit event handling: window open, input routing, presentation.
//!
//! [`WindowHandler`] owns the [`Shell`] once the event loop runs;
//! [`Shell::run`](super::Shell::run) only builds it and checks the
//! paint error afterwards.

use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, OwnedDisplayHandle};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

use super::{
    route_key_event, GameLoop, InputEdge, InputSnapshot, KeyRoute, Shell, DEFAULT_HEIGHT,
    DEFAULT_WIDTH,
};

mod capture;

use crate::screens::messenger::error_text_for_code;
use capture::CaptureAdapter;

/// Message ids of the 7.55 window hub (`0x5a5190`, file `0x1a5190`,
/// evidence `ev_ef50a9ada383cfaaf038eefbdfd96333c757d6aa83393521db7d`
/// prefix). `0xf060` is the SYSCOMMAND *command* that the `0x112`
/// arm routes to exit-confirm, so it is named as its own arm here.
pub(crate) const WM_KEY_DOWN: u32 = 0x100;
pub(crate) const WM_ACTIVATE_APP: u32 = 0x1c;
pub(crate) const SYS_CLOSE_COMMAND: u32 = 0xf060;
pub(crate) const WM_APP_UI_EVENT: u32 = 0x232;

/// One-second wait the exe's popup-or-close helper performs after it
/// sends `WM_CLOSE` (`0x5a8ab0`, file `0x1a8ab0`, evidence
/// `ev_a02f086e979ed751d9148dcb216bac12b61…`).
pub(crate) const ERROR_CLOSE_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

/// Choice made by the exe's popup-or-close helper for a fatal error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FatalPathDisposition {
    /// UI flag set: post the supplied error text as a popup.
    ShowMessage,
    /// UI flag clear: send `WM_CLOSE` and wait one second.
    CloseWindowAfter(std::time::Duration),
}

/// Decide the fatal path from the shared UI flag (`0x5a8ab0`).
pub(crate) const fn fatal_path_disposition(ui_flag: bool) -> FatalPathDisposition {
    if ui_flag {
        FatalPathDisposition::ShowMessage
    } else {
        FatalPathDisposition::CloseWindowAfter(ERROR_CLOSE_DELAY)
    }
}

/// State words the window hub reads before a billing rebuild
/// (`0x5a5190`, message `0x232`): `STATE_PAYMENT` and `STATE_PAYMENT2`.
pub const BILLING_PAYMENT_STATE_WORD: u32 = 12;
pub const BILLING_PAYMENT2_STATE_WORD: u32 = 13;

/// The billing rebuild arm a window message selects. No rebuild
/// happens without the UI message, a billing state, and a stored
/// web handle (`docs/pins/misc-tail.md`, `0x4f3d00`/`0x4f5020`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingWindowArm {
    /// No billing web view is touched.
    None,
    /// Close the stored web window and rebuild Payment.
    RebuildPayment,
    /// Close the stored web window and rebuild Payment2.
    RebuildGift,
}

/// Decode one window-hub message into a billing rebuild arm.
#[must_use]
pub const fn billing_window_arm(
    message: u32,
    state_word: u32,
    web_handle_set: bool,
) -> BillingWindowArm {
    if message != WM_APP_UI_EVENT || !web_handle_set {
        return BillingWindowArm::None;
    }
    match state_word {
        BILLING_PAYMENT_STATE_WORD => BillingWindowArm::RebuildPayment,
        BILLING_PAYMENT2_STATE_WORD => BillingWindowArm::RebuildGift,
        _ => BillingWindowArm::None,
    }
}

/// Suffix of the Print Screen folder under My Pictures. The exe
/// stores the CP949 bytes at `0x64427c`; this is their text
/// form, not a claim about byte encoding on a modern host.
pub const SCREENSHOT_FOLDER_SUFFIX: &str = "오투잼_스크린샷";
/// Frame size requested from the JPEG writer `0x57d800`.
pub const SCREENSHOT_DIMENSIONS: (u32, u32) = (DEFAULT_WIDTH, DEFAULT_HEIGHT);
/// Quality word passed by the screenshot writer (`0x57d800`).
pub const SCREENSHOT_QUALITY: u32 = 80;
/// States in which `0x418dc0` skips the screenshot writer.
pub const PRINT_SCREEN_SUPPRESSED_STATES: [u32; 2] = [10, 11];

/// Scene words the activation handler `0x5a4b90` inspects on a
/// Playing focus exit (`docs/pins/net-c2.md`, ev `3f40.f126…`).
pub const FOCUS_LOADING_SCENE: u32 = 10;
pub const FOCUS_PLAYING_SCENE: u32 = 11;

/// Captions the same handler posts to the UI queue. The bytes sit at
/// VA `0x650d68` (ev `09d7.e5ca…`), `0x650d38` (ev `fe5d.5722…`) and
/// `0x650d08` (ev `89ce.132c…`), decoded here as UTF-8 text.
pub const FOCUS_ROOM_EXIT_CAPTION: &str = "PLAY중 화면이 전환 되어서 Room으로 나왔습니다.";
pub const FOCUS_LOBBY_EXIT_CAPTION: &str = "PLAY중 화면이 전환 되어서 로비로 나왔습니다.";
pub const FOCUS_WAITING_EXIT_CAPTION: &str = "PLAY중 화면이 전환 되어서 대기실로 나왔습니다.";

/// Caption the exe's Playing focus-exit arm posts for the scene it
/// returns to: the Loading scene leaves to Room; a Playing scene in a
/// joined room leaves to Waiting, otherwise to the Lobby. Other
/// scenes post no exit notice. `room_open` is the session's joined-room
/// mirror, not a new wire fact.
#[must_use]
pub const fn focus_exit_caption(scene_word: u32, room_open: bool) -> Option<&'static str> {
    match scene_word {
        FOCUS_LOADING_SCENE => Some(FOCUS_ROOM_EXIT_CAPTION),
        FOCUS_PLAYING_SCENE => Some(if room_open {
            FOCUS_WAITING_EXIT_CAPTION
        } else {
            FOCUS_LOBBY_EXIT_CAPTION
        }),
        _ => None,
    }
}

/// Numbered file name pinned at `0x6447d0`. Rust's `{:03}`
/// keeps the C `ScreenShot%03d.jpg` shape for counters above 999.
#[must_use]
pub fn print_screen_file_name(counter: u32) -> String {
    format!("ScreenShot{counter:03}.jpg")
}

/// Path below My Pictures, in the exe's backslash form. The root is
/// supplied by the host OS, so only the suffix and name are pinned.
#[must_use]
pub fn print_screen_relative_path(counter: u32) -> String {
    format!(
        "{SCREENSHOT_FOLDER_SUFFIX}\\{}",
        print_screen_file_name(counter)
    )
}

/// What one Print Screen press does. `Write` carries the pinned
/// name and writer settings; no JPEG bytes are produced here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrintScreenAction {
    /// States 10 and 11 write nothing.
    Suppressed { state: u32 },
    /// The writer receives a numbered 800x600 JPEG request.
    Write {
        file_name: String,
        width: u32,
        height: u32,
        quality: u32,
    },
    /// The remaining-shot word is zero: the UI queue is posted.
    Notice,
}

impl PrintScreenAction {
    /// Exact UI event words posted by the exhausted-counter arm of
    /// `0x418dc0` (file `0x18dc0`), senior REA
    /// `ev_c03ce8c9615cd035da6ff9a8b8046c4af72a85938a908b3ecf67ec3ba33094c8`.
    /// This arm has no caption literal.
    pub const NOTICE_EVENT: [u16; 4] = [0x12, 0x67, 0, 0];

    /// Words this press posts to the UI queue. Only the exhausted
    /// branch posts an event in this headless seam.
    #[must_use]
    pub const fn notice_event(&self) -> Option<[u16; 4]> {
        match self {
            Self::Notice => Some(Self::NOTICE_EVENT),
            _ => None,
        }
    }
}

/// The session's used-name counter and remaining-shot word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenShotCounter {
    used: u32,
    remaining: u32,
}

impl ScreenShotCounter {
    /// Start of a session with `remaining` shots available.
    #[must_use]
    pub const fn new(remaining: u32) -> Self {
        Self { used: 0, remaining }
    }

    /// Take one press: suppress it in states 10/11, post a notice
    /// when no shots remain, otherwise consume the next name.
    pub fn press(&mut self, state_word: u32) -> PrintScreenAction {
        if PRINT_SCREEN_SUPPRESSED_STATES.contains(&state_word) {
            return PrintScreenAction::Suppressed { state: state_word };
        }
        if self.remaining == 0 {
            return PrintScreenAction::Notice;
        }
        let file_name = print_screen_file_name(self.used);
        self.used = self.used.saturating_add(1);
        self.remaining -= 1;
        let (width, height) = SCREENSHOT_DIMENSIONS;
        PrintScreenAction::Write {
            file_name,
            width,
            height,
            quality: SCREENSHOT_QUALITY,
        }
    }

    /// Number of names the session has consumed.
    #[must_use]
    pub const fn used(&self) -> u32 {
        self.used
    }

    /// Shots still available before the notice branch.
    #[must_use]
    pub const fn remaining(&self) -> u32 {
        self.remaining
    }
}

/// The exe handler a window message reaches. Our client has no
/// Win32 `WndProc`; the winit arms below call the same message
/// decoder so a guard can name arm -> handler without a real window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExeWindowHandler {
    /// `0x5a50d0`: the F8/print-screen key gate.
    KeyDown,
    /// `0x5a4b90`: activation and Playing-scene notices.
    ActivateApp,
    /// `0x5a5600`: exit confirmation (the system close command).
    ExitConfirm,
    /// `0x45e450`/`0x45ecd0`/`0x45f480` plus the scene-sheet closers
    /// `0x495a30`/`0x4f3d00`/`0x4f5020`.
    AppUiEvent,
    /// Any unlisted message reaches `DefWindowProcA`.
    DefaultProcedure,
}

/// Decode one pinned message arm to its handler. `0xf060` is a
/// command value, handled by the `WM_SYSCOMMAND` branch of the exe
/// hub; the other three are message ids.
pub(crate) const fn exe_window_handler(message: u32) -> ExeWindowHandler {
    match message {
        WM_KEY_DOWN => ExeWindowHandler::KeyDown,
        WM_ACTIVATE_APP => ExeWindowHandler::ActivateApp,
        SYS_CLOSE_COMMAND => ExeWindowHandler::ExitConfirm,
        WM_APP_UI_EVENT => ExeWindowHandler::AppUiEvent,
        _ => ExeWindowHandler::DefaultProcedure,
    }
}

/// Full rebuild the exe performs when its display-mode word changes
/// (`0x416a80`/file `0x16a80`, evidence
/// `ev_be7d984a8bbc798086b159b7106b4174c7632c68572e3f14561225234b49e68e`).
/// No winit caller drives the word yet; the plan is the staged seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayModePlan {
    /// Fullscreen style, or captioned windowed style.
    pub fullscreen: bool,
    /// The exe resizes the client to its fixed native frame.
    pub client_size: (u32, u32),
    /// DirectDraw surfaces are rebuilt on a change.
    pub rebuild_surfaces: bool,
    /// The window is shown and focused after the rebuild.
    pub show_and_focus: bool,
    /// The mode change records a fresh clock value.
    pub record_clock: bool,
}

/// Decide whether a display-mode request rebuilds the window.
/// A repeated request leaves the stored word alone and plans nothing.
#[must_use]
pub const fn display_mode_plan(requested: u16, stored: u16) -> Option<DisplayModePlan> {
    if requested == stored {
        return None;
    }
    Some(DisplayModePlan {
        fullscreen: requested != 0,
        client_size: (DEFAULT_WIDTH, DEFAULT_HEIGHT),
        rebuild_surfaces: true,
        show_and_focus: true,
        record_clock: true,
    })
}

/// Popup arm chosen by the exe's popup-or-close helper
/// (`0x5a8ab0`, file `0x1a8ab0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPopup {
    /// UI flag set: the supplied message reaches the modal box.
    ShowMessage(String),
    /// UI flag clear: close the window after the pinned delay.
    CloseWindowAfter(std::time::Duration),
}

/// Route one supplied error string to the popup-or-close decision.
/// `0x5a35c0` formats it and asks `0x5a8ab0`; this client has no
/// Win32 `MessageBoxA`, so the message is staged as data.
#[must_use]
pub fn error_popup_from_supplied(message: &str, ui_flag: bool) -> ErrorPopup {
    match fatal_path_disposition(ui_flag) {
        FatalPathDisposition::ShowMessage => ErrorPopup::ShowMessage(message.to_owned()),
        FatalPathDisposition::CloseWindowAfter(delay) => ErrorPopup::CloseWindowAfter(delay),
    }
}

/// Resolve one numeric error code through `0x5a3820` before asking
/// the popup-or-close helper. Code zero takes the exe's silent arm.
#[must_use]
pub fn error_popup_from_code(code: u32, ui_flag: bool) -> Option<ErrorPopup> {
    (code != 0).then(|| error_popup_from_supplied(error_text_for_code(code), ui_flag))
}

/// UI notice the exe posts from a double-click when the state bit is
/// set (`0x5a50b0` -> `0x548b10`, notice `10,3,0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiNotice {
    /// First notice word.
    pub kind: u16,
    /// Second notice word.
    pub code: u16,
    /// Third notice word.
    pub param: u16,
    /// Press x carried by the forwarded call.
    pub x: i32,
    /// Press y carried by the forwarded call.
    pub y: i32,
}

impl UiNotice {
    /// Build one pinned notice with its press position.
    #[must_use]
    pub const fn new(kind: u16, code: u16, param: u16, x: i32, y: i32) -> Self {
        Self {
            kind,
            code,
            param,
            x,
            y,
        }
    }
}

/// Bit of the state word the double-click helper tests.
pub const DOUBLE_CLICK_STATE_BIT: u16 = 1 << 2;

/// Decide one double-click arm. The winit loop has no Win32
/// `WM_LBUTTONDBLCLK`; the notice is the pinned contract until that
/// arm is wired by a lane that owns the game's notice queue.
#[must_use]
pub fn double_click_notice(state_word: u16, x: i32, y: i32) -> Option<UiNotice> {
    (state_word & DOUBLE_CLICK_STATE_BIT != 0).then(|| UiNotice::new(10, 3, 0, x, y))
}

/// Exit decision of the window-close arm (`WM_DESTROY` -> `WM_QUIT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitDisposition {
    /// The event loop ends on a close request.
    ExitLoop,
}

/// The winit close arm mirrors the Win32 quit message.
#[must_use]
pub const fn window_close_disposition() -> QuitDisposition {
    QuitDisposition::ExitLoop
}

/// Event-loop state: shell plus its softbuffer surface.
pub struct WindowHandler {
    pub(crate) shell: Shell,
    surface: Option<softbuffer::Surface<OwnedDisplayHandle, Arc<Window>>>,
    surface_size: Option<(u32, u32)>,
    paint_error: Option<anyhow::Error>,
    cursor: Option<(f64, f64)>,
    capture: Option<CaptureAdapter>,
    /// Display-mode word the exe compares before rebuilding.
    display_mode: u16,
    /// Queued discrete edges since the last freeze: winit events are
    /// OS transport only (ADR-0013), so every decoded edge waits here
    /// for the frame boundary instead of reaching the game directly.
    pending_edges: Vec<InputEdge>,
    /// Left-button state at the freeze (persists across snapshots;
    /// edges alone would lose a hold that spans frames).
    left_held: bool,
    /// Freeze order stamped on each delivered snapshot.
    snapshot_sequence: u64,
    /// An input or resize arrived since the last `about_to_wait` pass:
    /// the frame it may have changed owes one repaint (ticket #138).
    input_dirty: bool,
    /// Next heartbeat deadline while a time/wire-driven scene sets one
    /// (`GameLoop::idle_repaint_interval`); `None` on static scenes.
    next_heartbeat: Option<std::time::Instant>,
}

impl WindowHandler {
    /// Wrap a shell; no window or surface yet. A capture plan on the
    /// shell becomes the trace sink (frame dumps + applied-event log).
    pub fn new(mut shell: Shell) -> Self {
        let capture = shell.capture.take().map(CaptureAdapter::new);
        Self {
            shell,
            surface: None,
            surface_size: None,
            paint_error: None,
            cursor: None,
            capture,
            display_mode: 0,
            pending_edges: Vec::new(),
            left_held: false,
            snapshot_sequence: 0,
            input_dirty: false,
            next_heartbeat: None,
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
        // Clone the window handle up front: `buffer_mut` borrows the surface
        // mutably for the whole present, and the gate call needs the window.
        let window = surface.window().clone();
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
                // Arm winit 0.30.13's frame-callback gate: it engages ONLY
                // via `pre_present_notify` (winit #4668-adjacent; verified by
                // the winit-diligence sweep). Without this call our
                // `request_redraw` in `about_to_wait` re-pings the loop every
                // pass and the Wayland event loop busy-repaints at 100% CPU
                // (reproduced on KWin 6.8.2, 2026-11-12). With the gate
                // armed, the loop blocks on the compositor's frame callback
                // and idles. See ~/scratch/winit-4668/findings.md.
                window.pre_present_notify();
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

    /// Route one key event to a lane or menu edge. The game sees
    /// the raw press first (rebind capture), then the key map —
    /// synced from the game's current binds — resolves the lane.
    /// Edges queue for the next freeze; nothing reaches the game
    /// here (ADR-0013). `pub(crate)`: the input-latency probe (#100)
    /// drives scripted key events through this exact path.
    pub(crate) fn route_key(&mut self, code: winit::keyboard::KeyCode, down: bool) -> KeyRoute {
        self.route_key_with_text(code, down, None)
    }

    /// Same boundary as [`route_key_event`], with the key's committed
    /// text. The capture harness gets one line per applied text/erase
    /// edge and one per physical key edge.
    fn route_key_with_text(
        &mut self,
        code: winit::keyboard::KeyCode,
        down: bool,
        text: Option<&str>,
    ) -> KeyRoute {
        // Any key edge may move game state (dismiss, rebind, strike),
        // so the frame it changed owes one repaint (ticket #138).
        self.input_dirty = true;
        let route = route_key_event(
            self.shell.game.as_mut(),
            &mut self.shell.keymap,
            code,
            down,
            text,
        );
        // Log only what the game actually received. A captured key
        // leaves no text or erase line in the replay trace (#314).
        if let Some(text) = &route.text {
            for ch in text.chars() {
                self.log_event(&format!("text {ch}"));
            }
        }
        if route.erase {
            self.log_event("erase");
        }
        self.log_event(&format!(
            "key-{} {code:?}",
            if down { "down" } else { "up" }
        ));
        self.pending_edges.extend(route.edges.iter().copied());
        route
    }

    /// Queue a left press/release at window pixels as a native-frame
    /// press edge (letterbox bars never press through). The down
    /// edge keeps the `click x y` log line the capture harness
    /// already replays; the up edge logs its own `click-up` line.
    /// Queued only — the next freeze delivers it (ADR-0013).
    fn route_mouse_edge(&mut self, down: bool, cursor_x: f64, cursor_y: f64) {
        // Press feedback arms on down and fires on release-inside
        // (pinned #111): the changed frame owes one repaint.
        self.input_dirty = true;
        let Some((window_w, window_h)) = self.surface_size else {
            return;
        };
        if let Some((x, y)) = window_to_native(window_w, window_h, cursor_x, cursor_y) {
            if down {
                self.log_event(&format!("click {x} {y}"));
                self.pending_edges.push(InputEdge::PointerDown { x, y });
                self.left_held = true;
            } else {
                self.log_event(&format!("click-up {x} {y}"));
                self.pending_edges.push(InputEdge::PointerUp { x, y });
                self.left_held = false;
            }
        }
    }

    /// Remember one pointer-motion sample: the pending cursor cache
    /// the next freeze reads as the snapshot pointer (hover = paint-
    /// time position, ADR-0013). Queued only — the `MouseInput` arm
    /// below is the press source for a later edge (that event carries
    /// no position of its own), so the cache write lives here, in
    /// the one path motion events take.
    fn track_motion(&mut self, cursor_x: f64, cursor_y: f64) {
        self.cursor = Some((cursor_x, cursor_y));
        // Hover art follows the pointer alone (the channel bar tracks
        // motion with no press), so motion owes one repaint too.
        self.input_dirty = true;
    }

    /// Queue one left-button edge from the `MouseInput` window-event
    /// arm. The edge position is the cursor tracked from prior
    /// motion/enter events: `MouseInput` carries no position (X11
    /// button events and the Wayland protocol both omit coordinates,
    /// so a client must track the pointer itself), and winit 0.30
    /// exposes no pointer-position query to fall back to — the
    /// tracked cursor is the only source. A backend that ever
    /// pressed with no preceding `CursorMoved`/`CursorEntered` would
    /// drop every click (the #118 live symptom); the rig evidence and
    /// the open desktop chain live in `docs/window-input-seam.md`.
    /// Queued only — the next freeze delivers it (ADR-0013).
    fn route_mouse_input(&mut self, down: bool) {
        if let Some((x, y)) = self.cursor {
            self.route_mouse_edge(down, x, y);
        }
    }

    /// Freeze the frame's input snapshot and deliver it to the game:
    /// the pending pointer maps to native frame pixels, the queued
    /// edges drain in arrival order, and the button state carries
    /// over. Exactly one consumption point per frame (ADR-0013) —
    /// the live loop calls this at each `RedrawRequested` before
    /// painting; headless tests call it after queueing events.
    /// Returns the frozen snapshot for trace/test assertions.
    pub(crate) fn deliver_snapshot(&mut self) -> InputSnapshot {
        let pointer = self.cursor.and_then(|(cursor_x, cursor_y)| {
            let (window_w, window_h) = self.surface_size?;
            pointer_sample_for_tracking(window_w, window_h, cursor_x, cursor_y)
        });
        let mut snapshot = InputSnapshot::new();
        snapshot.set_pointer(pointer);
        snapshot.set_left_held(self.left_held);
        snapshot.set_sequence(self.snapshot_sequence);
        self.snapshot_sequence = self.snapshot_sequence.saturating_add(1);
        // The adapter stamps judged strikes from the pending edges before
        // this frame's one drain (ADR-0013); it borrows, never consumes.
        if let Some(capture) = self.capture.as_mut() {
            capture.record_judged_before_drain(&self.pending_edges);
        }
        for edge in self.pending_edges.drain(..) {
            snapshot.push_edge(edge);
        }
        self.shell.game.ingest_snapshot(&snapshot);
        snapshot
    }

    /// Feed committed text into the game (winit `Ime::Commit`, or a
    /// pressed key's produced text when no input method owns the
    /// key — Xvfb rigs have no XIM, so chat typing arrives this way).
    /// Control characters never reach the log or the game: Enter's
    /// produced `\r` would poison the capture log's line format.
    fn route_text(&mut self, text: &str) {
        // Chat drafts ink every committed char: the draft owes a repaint.
        self.input_dirty = true;
        let clean: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if clean.is_empty() {
            return;
        }
        for ch in clean.chars() {
            self.log_event(&format!("text {ch}"));
        }
        self.shell.game.text_input(&clean);
    }

    /// Deliver one activation edge to the game. The winit `Focused`
    /// event stands in for `WM_ACTIVATEAPP`; `GameLoop::focus_changed`
    /// is the same hole the exe's hub `0x5a5190` fills by calling the
    /// activation handler `0x5a4b90`.
    fn route_focus_change(&mut self, active: bool) {
        self.shell.game.focus_changed(active);
    }

    /// Erase one draft character (Backspace press).
    fn route_erase(&mut self) {
        self.route_key(winit::keyboard::KeyCode::Backspace, true);
    }

    /// Decode one pinned window-message arm and write it to the
    /// capture log. The winit event loop is the cross-platform
    /// stand-in for the exe's `WndProc`, but the arm -> handler
    /// mapping is the one the guard pins.
    fn log_message_arm(&mut self, message: u32) -> ExeWindowHandler {
        let handler = exe_window_handler(message);
        self.log_event(&format!("wm-{message:#x} -> {handler:?}"));
        handler
    }

    /// Forward one applied-event line to the capture adapter (no-op
    /// when this run has no capture plan).
    fn log_event(&mut self, event: &str) {
        if let Some(capture) = self.capture.as_mut() {
            capture.record_event(event);
        }
    }

    /// Hand one painted frame to the capture adapter. The adapter owns
    /// the log line, content fingerprint and dump gate.
    fn capture_frame(&mut self) {
        let Some(capture) = self.capture.as_mut() else {
            return;
        };
        let screen = self.shell.game.screen_name();
        let state_hash = self.shell.game.state_hash();
        capture.record_frame(&self.shell.pixels, screen, state_hash);
    }

    /// Apply one display-mode request to the stored word and, when a
    /// window exists, its fullscreen style. Surface rebuild follows
    /// the winit resize event; a headless caller gets only the plan.
    pub fn apply_display_mode(&mut self, requested: u16) -> Option<DisplayModePlan> {
        let plan = display_mode_plan(requested, self.display_mode)?;
        self.display_mode = requested;
        self.input_dirty = true;
        if let Some(window) = self.shell.window.as_ref() {
            let fullscreen = plan
                .fullscreen
                .then(|| winit::window::Fullscreen::Borderless(None));
            window.set_fullscreen(fullscreen);
            window.request_redraw();
        }
        Some(plan)
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

/// One sampled pointer position on its way to the shared tracker
/// (`0x432020`, file `0x32020`): the window sample becomes a native
/// point before the frame freeze sends it to the game. The stored
/// image helper is not present in this client, so its arm stays OPEN
/// and is not claimed by this guard.
#[must_use]
pub fn pointer_sample_for_tracking(
    window_w: u32,
    window_h: u32,
    cursor_x: f64,
    cursor_y: f64,
) -> Option<(i32, i32)> {
    window_to_native(window_w, window_h, cursor_x, cursor_y)
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

/// The next point on a fixed heartbeat grid. Steps already passed by a
/// slow paint are skipped, so paint cost never stretches the step
/// (the 7.55 avatar step is 66 ms, `EV-P229-DWELL-1`).
fn next_heartbeat_deadline(
    due: std::time::Instant,
    now: std::time::Instant,
    gap: std::time::Duration,
) -> std::time::Instant {
    if gap.is_zero() {
        return now;
    }
    let missed_steps = now.saturating_duration_since(due).as_nanos() / gap.as_nanos();
    let steps = u32::try_from(missed_steps)
        .unwrap_or(u32::MAX)
        .saturating_add(1);
    let Some(step) = gap.checked_mul(steps) else {
        return now;
    };
    due.checked_add(step).unwrap_or(now)
}

/// One `about_to_wait` paint decision: whether this pass owes the
/// compositor a frame, and the next heartbeat deadline. Pure so the
/// idle policy stays pinned without a window (scenario guards below).
///
/// Static scenes (`None`) repaint only the pass an input or resize
/// dirtied. Heartbeat scenes repaint that pass too, then follow their
/// fixed grid: input re-arms from the input paint, while an overdue
/// heartbeat steps from its own deadline. Playing holds its tick, lobby
/// drains ride their poll gap, and the first pass paints immediately.
fn plan_repaint(
    input_dirty: bool,
    gap: Option<std::time::Duration>,
    next_due: Option<std::time::Instant>,
    now: std::time::Instant,
) -> (bool, Option<std::time::Instant>) {
    let Some(gap) = gap else {
        return (input_dirty, None);
    };
    if input_dirty || next_due.is_none_or(|due| now >= due) {
        let due = if input_dirty {
            now + gap
        } else {
            next_due.map_or(now + gap, |due| next_heartbeat_deadline(due, now, gap))
        };
        (true, Some(due))
    } else {
        (false, next_due)
    }
}

/// Whether the frame just painted answered a quit-confirm OK. The
/// decision is extracted so the window seam can guard the same
/// `GameLoop` call the `RedrawRequested` arm makes (#691).
fn should_exit_after_frame(game: &dyn GameLoop) -> bool {
    game.quit_requested()
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
            WindowEvent::CloseRequested => {
                self.log_message_arm(SYS_CLOSE_COMMAND);
                match window_close_disposition() {
                    QuitDisposition::ExitLoop => event_loop.exit(),
                }
            }
            WindowEvent::Focused(focused) => {
                self.log_message_arm(WM_ACTIVATE_APP);
                self.input_dirty = true;
                self.route_focus_change(focused);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Backspace),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.log_message_arm(WM_KEY_DOWN);
                self.route_erase();
            }
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
                // Typed characters reach the chat drafts only when no
                // dialog captured the key first (#314). Xvfb rigs have
                // no XIM; real IMEs commit through `Ime::Commit`.
                if state == ElementState::Pressed {
                    self.log_message_arm(WM_KEY_DOWN);
                }
                self.route_key_with_text(code, state == ElementState::Pressed, text.as_deref());
            }
            WindowEvent::Ime(Ime::Commit(text)) => self.route_text(&text),
            WindowEvent::CursorMoved { position, .. } => {
                self.track_motion(position.x, position.y);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.route_mouse_input(state == ElementState::Pressed);
            }
            WindowEvent::Resized(size) => {
                self.resize_surface(size.width, size.height, event_loop);
                // A new surface size rescales the whole frame.
                self.input_dirty = true;
            }
            WindowEvent::RedrawRequested => {
                // Frame boundary (ADR-0013): freeze the queued input
                // into one snapshot and sweep it before painting, so
                // the frame the compositor gets already answers the
                // frame's input at paint-time positions.
                self.deliver_snapshot();
                self.shell.paint_backbuffer();
                self.present(event_loop);
                self.capture_frame();
                // Planet quit-confirm OK (`GameLoop::quit_requested`):
                // the game asked to leave, so the loop exits.
                if should_exit_after_frame(self.shell.game.as_ref()) {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.shell.window.clone() else {
            return;
        };
        let input_dirty = std::mem::replace(&mut self.input_dirty, false);
        let gap = self.shell.game.idle_repaint_interval();
        let now = std::time::Instant::now();
        let (repaint, next_due) = plan_repaint(input_dirty, gap, self.next_heartbeat, now);
        self.next_heartbeat = next_due;
        // A static scene (`None`) parks the loop in `Wait`: it sleeps
        // until the next input or resize instead of spinning the
        // frame-callback cycle (ticket #138). A heartbeat scene
        // re-arms its own deadline so Playing holds its tick and the
        // lobby drains keep arriving.
        match next_due {
            Some(due) => event_loop.set_control_flow(ControlFlow::WaitUntil(due)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
        if repaint {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{GameLoop, InputEdge};
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    // Live chain (Tier 1): `main.rs::run_play` -> `Shell::run` ->
    // `WindowHandler::window_event` -> `exe_window_handler`. The
    // Win32 hub `0x5a5190` routes key-down, activation, system close
    // and `WM_APP+0x32` to four distinct handler families; a winit
    // run has no HWND, but the arms name the same handlers.
    fn window_message_arms_name_their_exe_handlers() {
        // The `WM_APP+0x32` arm has no winit source: our client posts
        // no Win32 UI events. Its entry in the table pins the exe
        // handler; a real-window leg stays OPEN.
        assert_eq!(WM_APP_UI_EVENT, 0x232);
        assert_eq!(exe_window_handler(0x100), ExeWindowHandler::KeyDown);
        assert_eq!(exe_window_handler(0x1c), ExeWindowHandler::ActivateApp);
        assert_eq!(exe_window_handler(0xf060), ExeWindowHandler::ExitConfirm);
        assert_eq!(exe_window_handler(0x232), ExeWindowHandler::AppUiEvent);
        assert_eq!(
            exe_window_handler(0x10),
            ExeWindowHandler::DefaultProcedure,
            "an unlisted message still reaches the default procedure"
        );
    }

    #[test]
    // seam: the popup-or-close decision is a pure contract of the
    // missing fatal-error caller (`0x5a35c0`/`0x5a36d0`); no live
    // path in this client reaches it. A real-window leg stays OPEN.
    fn fatal_error_flag_selects_the_popup_or_the_delayed_window_close() {
        assert_eq!(
            fatal_path_disposition(true),
            FatalPathDisposition::ShowMessage
        );
        assert_eq!(
            fatal_path_disposition(false),
            FatalPathDisposition::CloseWindowAfter(std::time::Duration::from_secs(1)),
            "the exe waits one second after WM_CLOSE"
        );
    }

    /// Game probe recording routed input for headless key-event tests.
    #[derive(Default)]
    struct InputProbe {
        texts: Vec<String>,
        deletes: usize,
        lanes: Vec<(usize, bool)>,
        /// One entry per delivered snapshot: the intake count pins
        /// the single-consumption-point shape (ADR-0013).
        ingests: usize,
        /// Every drained edge, in arrival order.
        edges: Vec<super::super::InputEdge>,
        /// Latest snapshot pointer per delivery (hover reads it).
        pointers: Vec<Option<(i32, i32)>>,
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

        fn ingest_snapshot(&mut self, snapshot: &super::super::InputSnapshot) {
            let edges: Vec<super::super::InputEdge> = snapshot.edges().to_vec();
            {
                let mut probe = self.borrow_mut();
                probe.ingests += 1;
                probe.pointers.push(snapshot.pointer());
                probe.edges.extend(edges.iter().copied());
                for edge in &edges {
                    if let InputEdge::Lane { lane, down } = *edge {
                        probe.lanes.push((lane, down));
                    }
                }
            }
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
    fn static_scene_repaints_only_on_input() {
        // The #138 idle arm: a static scene with no input owes the
        // compositor nothing, so the loop parks instead of spinning.
        let now = std::time::Instant::now();
        assert_eq!(plan_repaint(false, None, None, now), (false, None));
        assert_eq!(
            plan_repaint(false, None, Some(now), now),
            (false, None),
            "a stale deadline never outlives its scene"
        );
    }

    #[test]
    fn static_scene_repaints_the_dirtied_pass() {
        let now = std::time::Instant::now();
        assert_eq!(plan_repaint(true, None, None, now), (true, None));
    }

    #[test]
    fn heartbeat_scene_paints_its_first_pass_immediately() {
        let now = std::time::Instant::now();
        let gap = std::time::Duration::from_millis(100);
        assert_eq!(
            plan_repaint(false, Some(gap), None, now),
            (true, Some(now + gap))
        );
    }

    #[test]
    fn heartbeat_scene_paces_repaints_off_the_last_paint() {
        let start = std::time::Instant::now();
        let gap = std::time::Duration::from_millis(100);
        let due = start + gap;
        // Before the deadline the loop sleeps through the pass.
        assert_eq!(
            plan_repaint(false, Some(gap), Some(due), start),
            (false, Some(due))
        );
        // At the deadline it repaints and re-arms off this paint.
        assert_eq!(
            plan_repaint(false, Some(gap), Some(due), due),
            (true, Some(due + gap))
        );
    }

    #[test]
    fn avatar_step_survives_a_paint_that_eats_the_deadline_slack() {
        // Live caller chain: about_to_wait -> plan_repaint ->
        // set_control_flow(WakeUpUntil(due)) -> winit RedrawRequested ->
        // WindowHandler::present/capture_frame -> shell.paint_at(now_ms) ->
        // avatar_animation_phase. The 7.55 avatar step is 66 ms
        // (EV-P229-DWELL-1); a 45 ms paint inside that step must not
        // add its cost to the next deadline.
        let start = std::time::Instant::now();
        let gap = std::time::Duration::from_millis(66);
        let due = start + gap;
        let painted_at = due + std::time::Duration::from_millis(45);
        assert_eq!(
            plan_repaint(false, Some(gap), Some(due), painted_at),
            (true, Some(due + gap))
        );
    }

    #[test]
    fn heartbeat_skips_deadlines_missed_by_a_long_paint() {
        // The same live chain as avatar_step_survives_a_paint_that_eats_
        // the_deadline_slack: a paint longer than a whole step must
        // resume on the next future grid point, not busily repaint.
        let start = std::time::Instant::now();
        let gap = std::time::Duration::from_millis(66);
        let due = start + gap;
        let painted_at = due + gap + std::time::Duration::from_millis(10);
        assert_eq!(
            plan_repaint(false, Some(gap), Some(due), painted_at),
            (true, Some(due + gap + gap))
        );
    }

    #[test]
    fn heartbeat_scene_repaints_at_once_on_input() {
        // Input never waits for the deadline — and the deadline
        // re-arms off the input paint, so the pace never doubles up.
        let start = std::time::Instant::now();
        let gap = std::time::Duration::from_millis(250);
        // The deadline sits a full extra gap out, so an input at
        // `start` re-arming off the input paint is distinguishable
        // from one that merely keeps (or doubles) the old deadline.
        let due = start + gap + gap;
        assert_eq!(
            plan_repaint(true, Some(gap), Some(due), start),
            (true, Some(start + gap))
        );
    }

    #[test]
    fn committed_text_reaches_the_game() {
        let (mut handler, probe) = probed_handler();
        handler.route_text("hello");
        assert_eq!(probe.borrow().texts, vec!["hello".to_owned()]);
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
        handler.deliver_snapshot();
        assert_eq!(probe.borrow().lanes, vec![(0, true), (0, false)]);
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn backspace_release_routes_nowhere() {
        let (mut handler, probe) = probed_handler();
        handler.route_key(KeyCode::Backspace, false);
        handler.deliver_snapshot();
        assert_eq!(probe.borrow().deletes, 0);
        assert!(probe.borrow().lanes.is_empty());
    }

    #[test]
    fn left_press_reaches_the_game_at_native_pixels() {
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.route_mouse_edge(true, 100.0, 200.0);
        handler.route_mouse_edge(false, 100.0, 200.0);
        handler.deliver_snapshot();
        assert_eq!(
            probe.borrow().edges,
            vec![
                InputEdge::PointerDown { x: 100, y: 200 },
                InputEdge::PointerUp { x: 100, y: 200 },
            ]
        );
    }

    #[test]
    fn mouse_input_arm_routes_motion_then_both_edges_to_the_game() {
        // The live case (#118 playtest): a motion event lands first,
        // the press and release follow through the MouseInput arm.
        // Every stage must reach the game through the SAME helpers
        // the winit match arms call — a real event loop never runs
        // in CI, so the arm bodies are the seam under test (#119).
        // Queued only: one freeze delivers the whole frame.
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.track_motion(100.0, 200.0);
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        handler.deliver_snapshot();
        assert_eq!(
            probe.borrow().pointers,
            vec![Some((100, 200))],
            "hover forwarded at paint-time position"
        );
        assert_eq!(
            probe.borrow().edges,
            vec![
                InputEdge::PointerDown { x: 100, y: 200 },
                InputEdge::PointerUp { x: 100, y: 200 },
            ],
            "press and release routed"
        );
    }

    #[test]
    fn snapshot_intake_delivers_both_edges_in_one_freeze() {
        // #141 seam-drift guard, kept through the snapshot migration:
        // the failure direction it pins is an implementor or a seam
        // that drops a press edge while keys keep flowing (the #118
        // no-input leg). The probe counts intakes precisely, so this
        // test fails the day the window stops delivering whole
        // snapshots and something between the arms drops an edge.
        // Snapshot evolution (ADR-0013): the freeze delivers exactly
        // one snapshot carrying both edges — the seam delivers
        // snapshots, never per-event pushes.
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.track_motion(100.0, 200.0);
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        let snapshot = handler.deliver_snapshot();
        assert_eq!(snapshot.sequence(), 0);
        assert_eq!(snapshot.edges().len(), 2, "one freeze, both edges");
        assert_eq!(
            probe.borrow().ingests,
            1,
            "the seam delivers snapshots, never gestures"
        );
        assert_eq!(
            probe.borrow().edges,
            vec![
                InputEdge::PointerDown { x: 100, y: 200 },
                InputEdge::PointerUp { x: 100, y: 200 },
            ],
            "both edges still reach the game"
        );
    }

    #[test]
    // Tier 1 window-input seam: `WindowHandler::window_event`
    // `CursorMoved` arm -> `track_motion` -> `deliver_snapshot` ->
    // `GameLoop::ingest_snapshot`. The exe's image-helper hop is not
    // in this client; Tier 3 hover capture stays OPEN.
    fn pointer_sample_reaches_the_tracker_and_the_game_snapshot() {
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.track_motion(10.0, 20.0);
        let snapshot = handler.deliver_snapshot();
        assert_eq!(snapshot.pointer(), Some((10, 20)));
        assert_eq!(probe.borrow().pointers, vec![Some((10, 20))]);
    }

    #[test]
    fn frozen_snapshot_drains_the_queue_exactly_once() {
        // The per-frame edge queue (ADR-0013): one freeze carries
        // every queued edge in order, and the next freeze carries
        // none — edges never replay and never leak across frames.
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.track_motion(10.0, 20.0);
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        let first = handler.deliver_snapshot();
        assert_eq!(first.sequence(), 0);
        assert_eq!(first.pointer(), Some((10, 20)));
        assert_eq!(first.edges().len(), 2);
        let second = handler.deliver_snapshot();
        assert_eq!(second.sequence(), 1);
        assert_eq!(second.pointer(), Some((10, 20)), "position persists");
        assert!(second.edges().is_empty(), "the queue drained");
        assert_eq!(probe.borrow().ingests, 2);
        assert_eq!(probe.borrow().edges.len(), 2);
    }

    #[test]
    fn button_state_survives_empty_freezes_between_edges() {
        // State vs edges (SDL `GetKeyboardState` warning): a hold
        // spanning frames stays held across snapshots with no edges,
        // and only its release edge frees it.
        let (mut handler, _probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.track_motion(10.0, 20.0);
        handler.route_mouse_input(true);
        let pressed = handler.deliver_snapshot();
        assert!(pressed.left_held());
        let held = handler.deliver_snapshot();
        assert!(held.edges().is_empty());
        assert!(held.left_held(), "the hold spans the empty frame");
        handler.route_mouse_input(false);
        let released = handler.deliver_snapshot();
        assert!(!released.left_held());
    }

    /// Log of the exact game callbacks the seam made: the event
    /// name plus native-frame coordinates (lane presses store the
    /// lane index in `x`).
    type RoutedEvents = Rc<RefCell<Vec<(&'static str, i32, i32)>>>;

    /// A live `ScreenGame` behind a recording delegate: the real
    /// scene logic answers every routed event, while the delegate
    /// logs the exact callbacks the seam made — the window owns the
    /// game as `Box<dyn GameLoop>`, so the shared cell is the only
    /// test-side handle on the concrete state.
    struct SeamGame {
        inner: Rc<RefCell<super::super::ScreenGame>>,
        routed: RoutedEvents,
    }

    impl GameLoop for SeamGame {
        fn lane_key(&mut self, lane: usize, down: bool) {
            self.routed
                .borrow_mut()
                .push(("lane", lane as i32, down as i32));
            self.inner.borrow_mut().lane_key(lane, down);
        }

        fn menu_key(&mut self, key: super::super::MenuKey, down: bool) {
            self.inner.borrow_mut().menu_key(key, down);
        }

        fn text_input(&mut self, text: &str) {
            self.inner.borrow_mut().text_input(text);
        }

        fn delete_text(&mut self) {
            self.inner.borrow_mut().delete_text();
        }

        fn ingest_snapshot(&mut self, snapshot: &super::super::InputSnapshot) {
            // Record each drained edge in arrival order, then let the
            // real scene logic answer — the routed log is exactly
            // what the seam delivered, in order.
            for edge in snapshot.edges() {
                let (name, x, y) = match *edge {
                    InputEdge::PointerDown { x, y } => ("down", x, y),
                    InputEdge::PointerUp { x, y } => ("up", x, y),
                    InputEdge::Lane { lane, down } => ("lane", lane as i32, down as i32),
                    InputEdge::Menu { .. } => continue,
                };
                self.routed.borrow_mut().push((name, x, y));
            }
            self.inner.borrow_mut().ingest_snapshot(snapshot);
        }

        fn quit_requested(&self) -> bool {
            self.inner.borrow().quit_requested()
        }

        fn screen_name(&self) -> &'static str {
            self.inner.borrow().screen_name()
        }

        fn state_hash(&self) -> u64 {
            self.inner.borrow().state_hash()
        }

        fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
            self.inner.borrow_mut().paint(frame);
        }
    }

    /// A handler around a live `ScreenGame` plus handles on both the
    /// concrete game (scene assertions) and the routed-event log.
    fn seamed_handler() -> (
        WindowHandler,
        Rc<RefCell<super::super::ScreenGame>>,
        RoutedEvents,
    ) {
        let inner = Rc::new(RefCell::new(super::super::ScreenGame::new()));
        let routed = Rc::new(RefCell::new(Vec::new()));
        let shell = Shell::new(Box::new(SeamGame {
            inner: inner.clone(),
            routed: routed.clone(),
        }));
        (WindowHandler::new(shell), inner, routed)
    }

    #[test]
    fn mouse_input_arm_arms_the_planet_press_split_after_the_pinned_key() {
        // A real ScreenGame behind the seam, two scenes deep:
        //
        // 1. The pinned ESC key leaves Avi (#385); a canvas click no
        //    longer advances the flow through the MouseInput arm.
        // 2. Planet press at the pinned option-gear rect (#115 split):
        //    the down edge only ARMS the press (no modal yet), the up
        //    edge fires it (the Options dialog opens).
        let (mut handler, game, routed) = seamed_handler();
        handler.surface_size = Some((800, 600));
        handler.route_key(KeyCode::Escape, true);
        handler.route_key(KeyCode::Escape, false);
        handler.deliver_snapshot();
        assert_eq!(
            handler.shell.game.screen_name(),
            "planet",
            "the pinned key advanced the flow through the seam"
        );
        assert!(routed.borrow().is_empty(), "no click crossed the seam");
        handler.track_motion(729.0, 580.0);
        handler.route_mouse_input(true);
        handler.deliver_snapshot();
        assert_eq!(
            game.borrow().planet_modal(),
            None,
            "the planet down edge arms the press without firing"
        );
        handler.route_mouse_input(false);
        handler.deliver_snapshot();
        assert_eq!(
            game.borrow().planet_modal(),
            Some(super::super::screens::game::PlanetModal::Options),
            "the planet up edge fires the armed press"
        );
        assert_eq!(
            routed.borrow()[..],
            [("down", 729, 580), ("up", 729, 580)],
            "both planet edges crossed the seam"
        );
    }

    /// The window exit arm reads the same `GameLoop` call the
    /// `RedrawRequested` handler uses (#691). Live chain:
    /// `WindowHandler::window_event` -> `route_mouse_input` ->
    /// `deliver_snapshot` -> `SeamGame::ingest_snapshot` ->
    /// `ScreenGame::click_planet_modal` -> `confirm_quit` ->
    /// `should_exit_after_frame`.
    #[test]
    fn quit_confirm_ok_reaches_the_window_exit_decision() {
        let (mut handler, game, _) = seamed_handler();
        handler.surface_size = Some((800, 600));
        handler.route_key(KeyCode::Escape, true);
        handler.route_key(KeyCode::Escape, false);
        handler.deliver_snapshot();
        assert_eq!(handler.shell.game.screen_name(), "planet");

        handler.track_motion(769.0, 580.0);
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        handler.deliver_snapshot();
        assert_eq!(
            game.borrow().planet_modal(),
            Some(super::super::screens::game::PlanetModal::QuitConfirm)
        );
        assert!(!should_exit_after_frame(handler.shell.game.as_ref()));

        handler.track_motion(350.0, 253.0);
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        handler.deliver_snapshot();

        assert_eq!(
            game.borrow().last_quit_confirm_outcome(),
            Some(super::super::screens::game::QuitConfirmOutcome::Confirmed)
        );
        assert!(should_exit_after_frame(handler.shell.game.as_ref()));
    }

    /// A window focus probe: only the activation edge matters here,
    /// so the other `GameLoop` methods are deliberately inert.
    struct FocusProbe(Rc<RefCell<Vec<bool>>>);

    impl GameLoop for FocusProbe {
        fn lane_key(&mut self, _lane: usize, _down: bool) {}
        fn ingest_snapshot(&mut self, _snapshot: &super::super::InputSnapshot) {}
        fn focus_changed(&mut self, active: bool) {
            self.0.borrow_mut().push(active);
        }
        fn paint(&mut self, _frame: &mut o2jam_render::Frame<'_>) {}
    }

    #[test]
    fn focus_arm_forwards_both_activation_edges_to_the_game_loop() {
        // Live chain (Tier 1): `main.rs::run_play` -> `Shell::run` ->
        // `WindowHandler::window_event` (Focused) -> `route_focus_change`
        // -> `GameLoop::focus_changed`. The winit event stands in for
        // `WM_ACTIVATEAPP`; Tier 4 real-window capture stays OPEN.
        let seen = Rc::new(RefCell::new(Vec::new()));
        let mut handler = WindowHandler::new(Shell::new(Box::new(FocusProbe(Rc::clone(&seen)))));

        handler.route_focus_change(false);
        handler.route_focus_change(true);

        assert_eq!(*seen.borrow(), [false, true], "off and on both reached");
    }

    #[test]
    fn playing_focus_exit_selects_the_three_pinned_scene_captions() {
        // Live chain (Tier 1): `main.rs::run_play` -> `Shell::run` ->
        // `WindowHandler::window_event` -> `GameLoop::focus_changed` ->
        // `ScreenGame::apply_focus_change` -> `focus_exit_caption`.
        // Scene ids come from the exe's pinned screen-name words.
        let loading = crate::options::f8_scene_id_of("loading");
        let playing = crate::options::f8_scene_id_of("playing");
        assert_eq!(loading, Some(FOCUS_LOADING_SCENE));
        assert_eq!(playing, Some(FOCUS_PLAYING_SCENE));
        assert_eq!(
            focus_exit_caption(FOCUS_LOADING_SCENE, true),
            Some(FOCUS_ROOM_EXIT_CAPTION)
        );
        assert_eq!(
            focus_exit_caption(FOCUS_PLAYING_SCENE, true),
            Some(FOCUS_WAITING_EXIT_CAPTION)
        );
        assert_eq!(
            focus_exit_caption(FOCUS_PLAYING_SCENE, false),
            Some(FOCUS_LOBBY_EXIT_CAPTION)
        );
        assert_eq!(focus_exit_caption(2, true), None, "Room is not an exit");
    }

    #[test]
    fn intra_frame_tap_registers_exactly_one_click() {
        // The exe's per-frame drain (A46): a press and a release
        // queued between two snapshots must still register as one
        // click — pure state would lose the pair entirely (SDL
        // `GetKeyboardState` warning). One freeze carries both
        // edges; the next freeze carries none, so the tap fires
        // once and never replays.
        let (mut handler, game, routed) = seamed_handler();
        handler.surface_size = Some((800, 600));
        handler.route_key(KeyCode::Escape, true);
        handler.route_key(KeyCode::Escape, false);
        handler.deliver_snapshot();
        assert_eq!(handler.shell.game.screen_name(), "planet");
        // The whole tap lands inside one frame: no freeze between
        // the press and the release.
        handler.track_motion(729.0, 580.0);
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        let tap = handler.deliver_snapshot();
        assert_eq!(tap.edges().len(), 2, "one freeze carries the tap");
        assert_eq!(
            game.borrow().planet_modal(),
            Some(super::super::screens::game::PlanetModal::Options),
            "the intra-frame tap fires exactly once"
        );
        assert_eq!(routed.borrow()[..], [("down", 729, 580), ("up", 729, 580)]);
        let replay = handler.deliver_snapshot();
        assert!(replay.edges().is_empty(), "the queue drained");
        assert_eq!(
            game.borrow().planet_modal(),
            Some(super::super::screens::game::PlanetModal::Options),
            "no second firing without new edges"
        );
    }

    #[test]
    fn state_without_edges_never_clicks() {
        // The counterpart the SDL warning demands: snapshots that
        // carry pointer motion but no edges arm and fire nothing —
        // clicks ride the drained queue, never the polled state.
        let (mut handler, game, _) = seamed_handler();
        handler.surface_size = Some((800, 600));
        handler.route_key(KeyCode::Escape, true);
        handler.route_key(KeyCode::Escape, false);
        handler.deliver_snapshot();
        assert_eq!(handler.shell.game.screen_name(), "planet");
        handler.track_motion(729.0, 580.0);
        handler.deliver_snapshot();
        handler.deliver_snapshot();
        assert_eq!(
            game.borrow().planet_modal(),
            None,
            "motion-only snapshots never fire the gear"
        );
    }

    #[test]
    fn mouse_input_arm_drops_a_press_in_the_letterbox_bars() {
        // 1600x900 letterboxes the native frame with 200 px side
        // bars: a motion over a bar caches the position, and the
        // press at the same spot must still route nothing (the
        // window-to-native mapping rejects it before the game).
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((1600, 900));
        handler.track_motion(100.0, 450.0);
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        let snapshot = handler.deliver_snapshot();
        assert!(snapshot.edges().is_empty(), "letterbox edges never queue");
        assert!(probe.borrow().edges.is_empty(), "letterbox press dropped");
        assert_eq!(probe.borrow().pointers, vec![None]);
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn mouse_input_arm_drops_the_edge_when_no_position_is_known() {
        // No motion/enter ever arrived (the headless case): the edge
        // has no position source — winit 0.30 offers no pointer query
        // — and routes nothing. This cold-cache drop is the only
        // code-level explanation for the #118 live symptom; why the
        // desktop session never delivered cursor events stays OPEN
        // (docs/window-input-seam.md).
        let (mut handler, probe) = probed_handler();
        handler.surface_size = Some((800, 600));
        handler.route_mouse_input(true);
        handler.route_mouse_input(false);
        let snapshot = handler.deliver_snapshot();
        assert!(snapshot.edges().is_empty(), "no position, no edges");
        assert!(snapshot.pointer().is_none());
        assert!(probe.borrow().edges.is_empty());
    }

    #[test]
    fn clicks_need_a_sized_surface() {
        let (mut handler, probe) = probed_handler();
        handler.route_mouse_edge(true, 100.0, 200.0);
        handler.route_mouse_edge(false, 100.0, 200.0);
        handler.deliver_snapshot();
        assert!(probe.borrow().edges.is_empty());
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

    #[test]
    fn mode_change_rebuilds_the_800x600_surface_and_window_style() {
        // seam: the display-mode word has no winit caller yet, so the
        // staged plan is the whole behavior the pin names. The Tier 3
        // leg boots through the same window handler seam.
        let changed = display_mode_plan(1, 0).expect("a changed mode has a plan");
        assert!(changed.fullscreen, "mode 1 takes fullscreen style");
        assert!(changed.rebuild_surfaces, "surfaces are rebuilt");
        assert_eq!(changed.client_size, (800, 600), "fixed client size");
        assert!(changed.show_and_focus, "the window shows and focuses");
        assert!(changed.record_clock, "the new clock value is recorded");
        assert_eq!(
            display_mode_plan(0, 0),
            None,
            "an unchanged mode does not rebuild anything"
        );
    }

    #[test]
    fn supplied_error_string_reaches_the_modal_popup_arm() {
        // seam: the missing fatal-error caller (`0x5a35c0`) is the
        // reason the popup-or-close decision is tested at this arm;
        // no Win32 MessageBox exists in the winit client.
        assert_eq!(
            error_popup_from_supplied("surface failed", true),
            ErrorPopup::ShowMessage("surface failed".to_owned())
        );
        assert_eq!(
            error_popup_from_supplied("surface failed", false),
            ErrorPopup::CloseWindowAfter(ERROR_CLOSE_DELAY)
        );
    }

    #[test]
    fn numeric_error_code_resolves_before_the_popup_arm() {
        // seam: `0x5a36d0` chooses text through `0x5a3820` before it
        // asks the popup-or-close helper; neither has a live caller.
        assert_eq!(error_popup_from_code(0, true), None, "code 0 is silent");
        assert_eq!(
            error_popup_from_code(0x8876_0091, true),
            Some(ErrorPopup::ShowMessage(
                "The pixel format was invalid as specified.".to_owned()
            ))
        );
    }

    #[test]
    fn double_click_raises_the_pinned_notice_only_with_the_state_bit() {
        // seam: the winit event loop has no `WM_LBUTTONDBLCLK`; the
        // notice contract (`0x5a50b0` -> `0x548b10`, 10,3,0) is the
        // whole behavior until that arm is wired by the senior.
        let notice = double_click_notice(0x0004, 21, 42).expect("bit 2 raises");
        assert_eq!(notice, UiNotice::new(10, 3, 0, 21, 42));
        assert_eq!(notice.x, 21, "press x reaches the notice");
        assert_eq!(notice.y, 42, "press y reaches the notice");
        assert_eq!(
            double_click_notice(0, 21, 42),
            None,
            "clear bit stays silent"
        );
    }

    #[test]
    fn window_close_request_quits_the_loop() {
        // seam: `WindowEvent::CloseRequested` is the winit stand-in
        // for `WM_DESTROY`; the decision is extracted so a guard and
        // the Tier 3 boot leg name the same exit arm.
        assert_eq!(window_close_disposition(), QuitDisposition::ExitLoop);
    }
}
