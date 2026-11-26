//! winit event handling: window open, input routing, presentation.
//!
//! [`WindowHandler`] owns the [`Shell`] once the event loop runs;
//! [`Shell::run`](super::Shell::run) only builds it and checks the
//! paint error afterwards.

use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use std::cell::Cell;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, OwnedDisplayHandle};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

use super::{
    route_key_event, GameLoop, InputEdge, InputSnapshot, KeyRoute, Shell, DEFAULT_HEIGHT,
    DEFAULT_WIDTH,
};
use crate::options;

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
/// `WM_APP+0x31`, the hub arm that hides both global web handles
/// through `0x5a8ba0` (`docs/pins/input-msg.md`, window hub evidence).
pub const WM_APP_HIDE_WEB_SHEETS: u32 = 0x231;

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

/// State word of the hub arm whose opener reaches the coupon branch
/// (`0x4f5020` → `0x4f4bd0`, `docs/pins/payment.md`).
pub const COUPON_HOST_STATE_WORD: u32 = BILLING_PAYMENT2_STATE_WORD;
/// Opener selector that names the coupon page; zero names jamcard.
pub const COUPON_PAGE_SELECTOR: u32 = 1;

/// Native replacement for the dead coupon page branch. The gate is
/// the pinned hub arm; the form itself is ours by design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouponHostArm {
    /// No coupon page branch was selected.
    None,
    /// Open the native coupon entry form.
    OpenNativeForm,
}

/// Decode the state-13 hub arm with the pinned opener selector.
#[must_use]
pub const fn coupon_host_arm(
    message: u32,
    state_word: u32,
    page_selector_word: u32,
    web_handle_set: bool,
) -> CouponHostArm {
    if message == WM_APP_UI_EVENT
        && state_word == COUPON_HOST_STATE_WORD
        && page_selector_word == COUPON_PAGE_SELECTOR
        && web_handle_set
    {
        CouponHostArm::OpenNativeForm
    } else {
        CouponHostArm::None
    }
}

/// What the exe's `WM_APP+0x31` window arm does to the web sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSheetVisibilityArm {
    /// No hide: the message is not the hide arm or no handle is stored.
    Preserve,
    /// Hide both stored handles through `0x5a8ba0`.
    HideBoth,
}

/// Decode one window message into the web-sheet visibility arm.
/// `0x5a5190` hides both handles only on `WM_APP+0x31` with a stored
/// handle; the `WM_APP+0x32` sheet-open message never hides them.
#[must_use]
pub const fn web_sheet_visibility_arm(message: u32, web_handle_set: bool) -> WebSheetVisibilityArm {
    if message == WM_APP_HIDE_WEB_SHEETS && web_handle_set {
        WebSheetVisibilityArm::HideBoth
    } else {
        WebSheetVisibilityArm::Preserve
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
const SCREENSHOT_PIXEL_COUNT: usize = DEFAULT_WIDTH as usize * DEFAULT_HEIGHT as usize;

/// Why one Print Screen file was not written.
#[derive(Debug)]
enum ScreenShotWriteError {
    /// The supplied backbuffer is not one pinned 800x600 frame.
    FrameSize { expected: usize, actual: usize },
    /// The folder or file could not be opened.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The JPEG encoder refused the frame.
    Encode {
        path: PathBuf,
        source: image::ImageError,
    },
}

impl std::fmt::Display for ScreenShotWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameSize { expected, actual } => {
                write!(
                    formatter,
                    "screenshot frame has {actual} pixels, expected {expected}"
                )
            }
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "cannot write screenshot {}: {source}",
                    path.display()
                )
            }
            Self::Encode { path, source } => {
                write!(
                    formatter,
                    "cannot encode screenshot {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ScreenShotWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FrameSize { .. } => None,
            Self::Io { source, .. } => Some(source),
            Self::Encode { source, .. } => Some(source),
        }
    }
}

/// Observable result of one Print Screen write. A missing path means
/// the press was suppressed or exhausted; an error means the file was
/// refused and the counter was rolled back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenShotWriteResult {
    path: Option<PathBuf>,
    error: Option<String>,
}

impl ScreenShotWriteResult {
    fn written(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            error: None,
        }
    }

    fn failed(path: PathBuf, error: impl std::fmt::Display) -> Self {
        Self {
            path: Some(path),
            error: Some(error.to_string()),
        }
    }

    fn skipped() -> Self {
        Self {
            path: None,
            error: None,
        }
    }

    /// Whether a JPEG was actually written.
    #[must_use]
    pub const fn is_written(&self) -> bool {
        self.path.is_some() && self.error.is_none()
    }

    /// Whether the file or encoder refused the write.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.error.is_some()
    }

    /// Path of the attempted or written file.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Error text when the write failed.
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Convert the XRGB backbuffer words into RGB triplets, as the exe's
/// writer receives one RGB row per source row (`0x57d800`).
fn native_frame_to_rgb(native_frame: &[u32]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(native_frame.len() * 3);
    for word in native_frame {
        rgb.push((word >> 16) as u8);
        rgb.push((word >> 8) as u8);
        rgb.push(*word as u8);
    }
    rgb
}

/// Write one pinned 800x600 JPEG at quality 80.
fn write_print_screen_jpeg(
    path: &Path,
    native_frame: &[u32],
    width: u32,
    height: u32,
    quality: u32,
) -> Result<(), ScreenShotWriteError> {
    if native_frame.len() != SCREENSHOT_PIXEL_COUNT {
        return Err(ScreenShotWriteError::FrameSize {
            expected: SCREENSHOT_PIXEL_COUNT,
            actual: native_frame.len(),
        });
    }
    let quality = u8::try_from(quality).expect("pinned screenshot quality fits in u8");
    let file = std::fs::File::create(path).map_err(|source| ScreenShotWriteError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut encoder = JpegEncoder::new_with_quality(file, quality);
    encoder
        .encode(
            &native_frame_to_rgb(native_frame),
            width,
            height,
            ExtendedColorType::Rgb8,
        )
        .map_err(|source| ScreenShotWriteError::Encode {
            path: path.to_path_buf(),
            source,
        })
}

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

/// Caption the exe's activation arm posts when a paused scene exits
/// (`docs/pins/input-msg.md`, `0x5a4b90` addendum). Playing always
/// leaves to Room. Loading leaves to the lobby when the room title
/// word is empty, otherwise back to Waiting. Other scenes post no
/// exit notice.
#[must_use]
pub const fn focus_exit_caption(scene_word: u32, room_title_empty: bool) -> Option<&'static str> {
    match (scene_word, room_title_empty) {
        (FOCUS_PLAYING_SCENE, _) => Some(FOCUS_ROOM_EXIT_CAPTION),
        (FOCUS_LOADING_SCENE, true) => Some(FOCUS_LOBBY_EXIT_CAPTION),
        (FOCUS_LOADING_SCENE, false) => Some(FOCUS_WAITING_EXIT_CAPTION),
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
/// name and writer settings for [`PrintScreenAction::write_jpeg`].
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

    /// Write this press's frame below the host's My Pictures root.
    /// Suppressed and exhausted presses write nothing and return a
    /// result with no path; a failed file returns its error text.
    #[must_use]
    pub fn write_jpeg(&self, pictures_root: &Path, native_frame: &[u32]) -> ScreenShotWriteResult {
        let Self::Write {
            file_name,
            width,
            height,
            quality,
        } = self
        else {
            return ScreenShotWriteResult::skipped();
        };
        let folder = pictures_root.join(SCREENSHOT_FOLDER_SUFFIX);
        let path = folder.join(file_name);
        if let Err(source) = std::fs::create_dir_all(&folder) {
            return ScreenShotWriteResult::failed(path, source);
        }
        match write_print_screen_jpeg(&path, native_frame, *width, *height, *quality) {
            Ok(()) => ScreenShotWriteResult::written(path),
            Err(error) => ScreenShotWriteResult::failed(path, error),
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

    /// Take one press, write its frame, and commit the name only when
    /// the JPEG is complete. A refused write rolls the reservation back
    /// so the next press reuses the same number.
    #[must_use]
    pub fn capture(
        &mut self,
        state_word: u32,
        pictures_root: &Path,
        native_frame: &[u32],
    ) -> ScreenShotWriteResult {
        let action = self.press(state_word);
        let result = action.write_jpeg(pictures_root, native_frame);
        if result.is_failed() {
            self.undo_last_write();
        }
        result
    }

    /// Roll back the reservation made by a failed `Write` press.
    fn undo_last_write(&mut self) {
        self.used = self.used.saturating_sub(1);
        self.remaining = self.remaining.saturating_add(1);
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

/// Overlay the staged popup on one painted frame. Only the show arm
/// owns pixels; the delayed-close arm leaves the scene untouched.
fn paint_staged_error_popup(popup: &Option<ErrorPopup>, frame: &mut o2jam_render::Frame<'_>) {
    if let Some(ErrorPopup::ShowMessage(message)) = popup {
        crate::screens::game::paint_error_popup(frame, message);
    }
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

/// Windows system default for `GetDoubleClickTime`. The exe hub
/// `0x5a5190` applies the OS rule and has no timer of its own; there
/// is no exe byte pin for this number. OPEN, tracked by #1122. Next
/// evidence step: read `GetDoubleClickTime` and the system rectangle
/// at the hub under a live capture, or pin the default in the exe's
/// import table. winit has no double-click event, so this client
/// restates the OS default explicitly until that pin exists.
pub const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(500);
/// Windows system default double-click rectangle (`SM_CXDOUBLECLK` /
/// `SM_CYDOUBLECLK`), in native pixels. No exe pin; the hub applies
/// the OS rule rather than carrying its own numbers. OPEN, tracked
/// by #1122. Next evidence step: read the system rectangle at the
/// hub under a live capture, or pin the default in the exe's import
/// table.
pub const DOUBLE_CLICK_RECT: i32 = 4;

/// Mode word the exe's dialog-show helpers write (`0x433c90` writes
/// `0x304`, its hide helper `0x433d60` writes zero); bit 2 is the
/// gate the double-click notice helper reads.
pub(crate) const DIALOG_HOST_MODE_WORD: u16 = 0x304;

thread_local! {
    /// Live dialog-host ownership published by `ScreenGame` on every
    /// show, hide and wire-dismiss path. `Shell` owns the game as
    /// `dyn GameLoop`, so the window arm reads this shared cell rather
    /// than a cached word refreshed on a frame schedule.
    static DIALOG_HOST_INPUT_OWNED: Cell<bool> = const { Cell::new(false) };
}

/// Publish the real dialog host's ownership for the window arm.
/// `ScreenGame` calls this on every dialog show, hide and dismissal;
/// painters and the double-click arm read the same owned state.
pub(crate) fn set_dialog_host_input_owned(owned: bool) {
    DIALOG_HOST_INPUT_OWNED.set(owned);
}

/// Read the live dialog-host ownership at the moment of the press.
#[cfg(test)]
pub(crate) fn dialog_host_input_owned() -> bool {
    DIALOG_HOST_INPUT_OWNED.get()
}

/// One left press the double-click arm can pair with a later press.
#[derive(Debug, Clone, Copy)]
struct LeftPress {
    at: Instant,
    x: i32,
    y: i32,
}

/// Whether a second press forms the system double-click pair: inside
/// the time window and inside the rectangle around the first press.
fn is_double_click_pair(first: LeftPress, second: LeftPress) -> bool {
    let Some(elapsed) = second.at.checked_duration_since(first.at) else {
        return false;
    };
    elapsed <= DOUBLE_CLICK_TIME
        && (second.x - first.x).abs() <= DOUBLE_CLICK_RECT
        && (second.y - first.y).abs() <= DOUBLE_CLICK_RECT
}

/// Decide one double-click arm. The winit loop has no Win32
/// `WM_LBUTTONDBLCLK`; the window arm supplies the press pair and
/// this helper pins the exe's notice words.
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
    /// Host-supplied My Pictures root. `None` keeps this client from
    /// writing into a real user folder without an explicit caller.
    screenshot_root: Option<PathBuf>,
    /// Per-session shot counter behind the Print Screen key arm.
    screenshot_counter: ScreenShotCounter,
    /// Popup staged by the induced fatal-error hook. `None` paints no
    /// box; the close arm stages nothing.
    error_popup: Option<ErrorPopup>,
    /// Most recent left press, used only by the double-click arm.
    last_left_press: Option<LeftPress>,
    /// Queued-notice mirror: the exe appends `(10,3,0)` to its UI
    /// input ring (`0x5429b0` -> `0x545910`). This mirror holds the
    /// same record; the exe's ring consumer is unpinned, so no
    /// delivery, caption, or pixel sink is claimed here.
    queued_notice_mirror: Vec<UiNotice>,
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
            screenshot_root: None,
            screenshot_counter: ScreenShotCounter::new(0),
            error_popup: None,
            last_left_press: None,
            queued_notice_mirror: Vec::new(),
        }
    }

    /// Induce the supplied-message fatal arm without a Win32 message
    /// box. The returned decision is the one staged for the next paint.
    pub fn raise_error_from_supplied(&mut self, message: &str, ui_flag: bool) -> ErrorPopup {
        let decision = error_popup_from_supplied(message, ui_flag);
        self.stage_error_decision(Some(decision.clone()));
        decision
    }

    /// Induce the numeric-code fatal arm. Code zero stages no box.
    pub fn raise_error_from_code(&mut self, code: u32, ui_flag: bool) -> Option<ErrorPopup> {
        let decision = error_popup_from_code(code, ui_flag);
        self.stage_error_decision(decision.clone());
        decision
    }

    /// Clear the staged popup so the next frame returns to the scene.
    pub fn dismiss_error_popup(&mut self) {
        self.error_popup = None;
    }

    fn stage_error_decision(&mut self, decision: Option<ErrorPopup>) {
        self.error_popup = decision.filter(|popup| matches!(popup, ErrorPopup::ShowMessage(_)));
    }

    /// Paint one frame through this window's shell. Headless legs use
    /// the same paint path the `RedrawRequested` arm presents.
    pub fn paint_popup_frame(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        self.shell.paint_into(buffer, width, height);
        let mut frame = o2jam_render::Frame {
            buf: buffer,
            w: width,
            h: height,
        };
        paint_staged_error_popup(&self.error_popup, &mut frame);
    }

    /// Backbuffer paint for the window's redraw arm: scene first, then
    /// the staged fatal-error box over it.
    fn paint_backbuffer_with_error_popup(&mut self) {
        self.shell.paint_backbuffer();
        let mut frame = o2jam_render::Frame {
            buf: &mut self.shell.pixels,
            w: DEFAULT_WIDTH as usize,
            h: DEFAULT_HEIGHT as usize,
        };
        paint_staged_error_popup(&self.error_popup, &mut frame);
    }

    /// Install an explicit screenshot root and shot budget. Tests use a
    /// scratch folder; a host that resolves My Pictures can do the same.
    #[cfg(test)]
    pub(crate) fn install_screenshot_root(&mut self, root: PathBuf, remaining: u32) {
        self.screenshot_root = Some(root);
        self.screenshot_counter = ScreenShotCounter::new(remaining);
    }

    /// Write the current native backbuffer for a Print Screen press.
    /// The scene word comes from the same F8 scene table the exe's
    /// key gate reads; states 10 and 11 suppress the writer.
    fn capture_current_frame(&mut self) {
        let Some(root) = self.screenshot_root.clone() else {
            self.log_event("screenshot skipped: no root");
            return;
        };
        let state_word = options::f8_scene_id_of(self.shell.game.screen_name()).unwrap_or(0);
        let result = self
            .screenshot_counter
            .capture(state_word, &root, &self.shell.pixels);
        match result.path() {
            Some(path) => self.log_event(&format!("screenshot {}", path.display())),
            None => self.log_event("screenshot skipped: suppressed or exhausted"),
        }
        if let Some(error) = result.error_message() {
            tracing::warn!(error, "print screen write failed");
            self.log_event(&format!("screenshot error: {error}"));
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
            // Our choice: a resized window is a new hit-test surface,
            // so a press from the old geometry cannot pair. No exe
            // pin; see `resize_clears_the_pending_double_click_press`.
            self.last_left_press = None;
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
        if code == winit::keyboard::KeyCode::PrintScreen {
            self.log_event(&format!(
                "key-{} PrintScreen",
                if down { "down" } else { "up" }
            ));
            if down {
                self.capture_current_frame();
            }
            return KeyRoute {
                captured: true,
                text: None,
                erase: false,
                edges: Vec::new(),
            };
        }
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
    fn route_mouse_edge(&mut self, down: bool, cursor_x: f64, cursor_y: f64) -> Option<(i32, i32)> {
        let native = self
            .surface_size
            .and_then(|(width, height)| window_to_native(width, height, cursor_x, cursor_y));
        self.queue_mouse_edge(down, native);
        native
    }

    /// Queue the press/release edge from one already-mapped native
    /// point. The mapping happens once per window event, whether it
    /// comes from the winit arm or a headless live leg.
    fn queue_mouse_edge(&mut self, down: bool, native: Option<(i32, i32)>) {
        // Press feedback arms on down and fires on release-inside
        // (pinned #111): the changed frame owes one repaint.
        self.input_dirty = true;
        if let Some((x, y)) = native {
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

    /// Queued-notice mirror in post order. The exe's ring consumer is
    /// unpinned; these records are staged, not delivered.
    #[must_use]
    pub fn queued_ui_notices(&self) -> &[UiNotice] {
        &self.queued_notice_mirror
    }

    /// Set the window surface for a headless leg. The real event loop
    /// sets the same geometry in its `Resized` arm; live legs have no
    /// winit window but must still map window pixels to native pixels.
    pub fn set_window_surface_size(&mut self, width: u32, height: u32) {
        self.surface_size = Some((width, height));
        self.last_left_press = None;
    }

    /// Drive one full left click through the same motion and
    /// `MouseInput` helpers the winit event arms call. Headless live
    /// legs use it because winit emits no double-click event.
    pub fn route_window_left_press(&mut self, window_x: f64, window_y: f64) {
        self.track_motion(window_x, window_y);
        self.route_mouse_input(true);
        self.route_mouse_input(false);
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
        let Some((cursor_x, cursor_y)) = self.cursor else {
            return;
        };
        let native = self.route_mouse_edge(down, cursor_x, cursor_y);
        if down {
            match native {
                Some((x, y)) => self.stage_double_click_at_native(x, y),
                None => self.last_left_press = None,
            }
        }
    }

    /// Mirror the exe's `WM_LBUTTONDBLCLK` arm (`0x5a5190` ->
    /// `0x5a50b0` -> `0x548b10`) for a second press inside the window:
    /// read the live dialog-host mode word and post the pinned notice
    /// into the window's input ring.
    fn stage_double_click_at_native(&mut self, x: i32, y: i32) {
        let press = LeftPress {
            at: Instant::now(),
            x,
            y,
        };
        if self
            .last_left_press
            .is_some_and(|first| is_double_click_pair(first, press))
        {
            if let Some(notice) = double_click_notice(self.control_mode_word(), x, y) {
                self.queued_notice_mirror.push(notice);
            }
        }
        self.last_left_press = Some(press);
    }

    /// Mode word from the real dialog host's live ownership. The exe's
    /// show helpers set bit 2 and its hide helpers clear the word; the
    /// shared cell carries that same state from `ScreenGame`.
    fn control_mode_word(&self) -> u16 {
        if DIALOG_HOST_INPUT_OWNED.get() {
            DIALOG_HOST_MODE_WORD
        } else {
            0
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
                self.paint_backbuffer_with_error_popup();
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
    use super::super::{GameLoop, InputEdge, InputSnapshot, MenuKey, Screen, ScreenGame};
    use super::*;
    use crate::screens::messenger::UNRECOGNIZED_ERROR_TEXT;
    use crate::screens::paint::LoadingStatus;
    use crate::screens::side::MyRoomNotice;
    use o2jam_audio::NullBackend;
    use o2jam_net::play::SetRoomMusic;
    use o2jam_net::JoinedRoom;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn coupon_host_arm_needs_the_state_13_selector_and_a_stored_handle() {
        // seam: this Win32 hub arm has no message pump in this client;
        // the pinned gate is the whole behavior a headless leg drives.
        assert_eq!(
            coupon_host_arm(
                WM_APP_UI_EVENT,
                COUPON_HOST_STATE_WORD,
                COUPON_PAGE_SELECTOR,
                true
            ),
            CouponHostArm::OpenNativeForm
        );
    }

    // Evidence: docs/pins/payment.md:20 (coupon/jamcard opener split).
    #[test]
    fn coupon_host_arm_leaves_the_jamcard_selector_alone() {
        // seam: selector zero names jamcard, not the coupon form.
        assert_eq!(
            coupon_host_arm(WM_APP_UI_EVENT, COUPON_HOST_STATE_WORD, 0, true),
            CouponHostArm::None
        );
    }

    #[test]
    fn coupon_host_arm_needs_the_ui_message() {
        // seam: other hub messages do not reach the opener.
        assert_eq!(
            coupon_host_arm(
                WM_APP_HIDE_WEB_SHEETS,
                COUPON_HOST_STATE_WORD,
                COUPON_PAGE_SELECTOR,
                true
            ),
            CouponHostArm::None
        );
    }

    #[test]
    fn exe_app_hide_message_hides_web_sheets_with_a_stored_handle() {
        // seam: `WM_APP+0x31` reaches `0x5a8ba0` only from the Win32
        // hub `0x5a5190`, which this winit client has no message pump
        // to call. The pure arm and the host's hide call are the whole
        // visibility behavior a headless client can observe.
        assert_eq!(WM_APP_HIDE_WEB_SHEETS, 0x231);
        assert_eq!(
            web_sheet_visibility_arm(WM_APP_HIDE_WEB_SHEETS, true),
            WebSheetVisibilityArm::HideBoth
        );
        assert_eq!(
            web_sheet_visibility_arm(WM_APP_UI_EVENT, true),
            WebSheetVisibilityArm::Preserve,
            "the UI-event message opens a sheet; it never hides it"
        );
        assert_eq!(
            web_sheet_visibility_arm(WM_APP_HIDE_WEB_SHEETS, false),
            WebSheetVisibilityArm::Preserve,
            "no stored handle means no sheet to hide"
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

    /// One window around the real one-OK host: the same
    /// `apply_myroom_notice` call the 5035 wire arm uses, not a test
    /// error popup or a hand-set mode word.
    fn dialog_host_handler() -> WindowHandler {
        let mut game = ScreenGame::new();
        game.apply_myroom_notice(MyRoomNotice::PenaltyCleared);
        let mut handler = WindowHandler::new(Shell::new(Box::new(game)));
        handler.set_window_surface_size(800, 600);
        handler
    }

    /// One window whose real host has no dialog up.
    fn bare_game_handler() -> WindowHandler {
        let mut handler = WindowHandler::new(Shell::new(Box::new(ScreenGame::new())));
        handler.set_window_surface_size(800, 600);
        handler
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

        fn focus_changed(&mut self, active: bool) {
            self.inner.borrow_mut().focus_changed(active);
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
            focus_exit_caption(FOCUS_PLAYING_SCENE, false),
            Some(FOCUS_ROOM_EXIT_CAPTION),
            "Playing always exits to Room"
        );
        assert_eq!(
            focus_exit_caption(FOCUS_LOADING_SCENE, true),
            Some(FOCUS_LOBBY_EXIT_CAPTION),
            "an empty room title exits Loading to the lobby"
        );
        assert_eq!(
            focus_exit_caption(FOCUS_LOADING_SCENE, false),
            Some(FOCUS_WAITING_EXIT_CAPTION),
            "a named room exits Loading to Waiting"
        );
        assert_eq!(focus_exit_caption(2, true), None, "Room is not an exit");
    }

    /// A room with only the title word the activation arm inspects.
    fn room_with_title(title: &str) -> JoinedRoom {
        JoinedRoom {
            member_id: 0,
            team: 0,
            title: title.to_owned(),
            music_id: 757,
            arena: 1,
            key_mode: 2,
            game_mode: 0,
            difficulty: 0,
            speed: 1,
            has_super_manager: false,
            user_count: 0,
            slots: Vec::new(),
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    /// Walk the pinned boot keys to Loading, then stage the room title
    /// the exe's activation arm reads there.
    fn game_on_loading(title: &str) -> ScreenGame {
        let mut game = ScreenGame::new();
        game.menu_key(MenuKey::Back, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading, "boot walk reaches Loading");
        game.set_joined_room(room_with_title(title));
        game
    }

    /// Arm the Loading gate the way the chart hand-off does, then
    /// cross to Playing with the same Confirm the player presses.
    fn game_on_playing() -> ScreenGame {
        let mut game = game_on_loading("Lobby");
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        game.set_selected_music(SetRoomMusic {
            music_id: 757,
            difficulty: 1,
            speed: 0,
        });
        game.set_loading_status(LoadingStatus {
            progress: 1.0,
            title: "First".to_owned(),
            artist: "Band".to_owned(),
        });
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing, "the armed gate hands off");
        game
    }

    #[test]
    fn focus_return_from_playing_installs_room_with_the_pinned_caption() {
        // Live chain (Tier 1): `main.rs::run_play` -> `Shell::run` ->
        // `WindowHandler::window_event` (Focused) -> `route_focus_change`
        // -> `GameLoop::focus_changed` -> `ScreenGame::apply_focus_change`
        // -> `ScreenGame::enter_screen(Screen::Room)`. Deactivation only
        // records the pause; Tier 4 real-focus capture stays OPEN.
        let (mut handler, game, _) = seamed_handler();
        *game.borrow_mut() = game_on_playing();
        assert_eq!(game.borrow().screen(), Screen::Playing);

        handler.route_focus_change(false);
        assert_eq!(
            game.borrow().screen(),
            Screen::Playing,
            "deactivation records the pause without leaving the scene"
        );
        assert_eq!(
            game.borrow().focus_exit_notice(),
            None,
            "focus loss posts no exit caption"
        );

        handler.route_focus_change(true);
        assert_eq!(game.borrow().screen(), Screen::Room);
        assert_eq!(
            game.borrow().focus_exit_notice(),
            Some(FOCUS_ROOM_EXIT_CAPTION),
            "activation posts the pinned Room caption"
        );
    }

    #[test]
    fn focus_return_from_loading_with_empty_title_installs_room_with_lobby_caption() {
        // Live chain (Tier 1): `main.rs::run_play` -> `Shell::run` ->
        // `WindowHandler::window_event` (Focused) -> `route_focus_change`
        // -> `GameLoop::focus_changed` -> `ScreenGame::apply_focus_change`
        // -> `ScreenGame::clear_waiting_seats` ->
        // `ScreenGame::enter_screen(Screen::Room)`.
        let (mut handler, game, _) = seamed_handler();
        *game.borrow_mut() = game_on_loading("");
        assert_eq!(game.borrow().screen(), Screen::Loading);

        handler.route_focus_change(false);
        assert_eq!(game.borrow().screen(), Screen::Loading);

        handler.route_focus_change(true);
        assert_eq!(game.borrow().screen(), Screen::Room);
        assert_eq!(
            game.borrow().focus_exit_notice(),
            Some(FOCUS_LOBBY_EXIT_CAPTION),
            "an empty room title posts the pinned lobby caption"
        );
    }

    #[test]
    fn focus_return_from_loading_with_room_title_installs_waiting_with_caption() {
        // Live chain (Tier 1): `main.rs::run_play` -> `Shell::run` ->
        // `WindowHandler::window_event` (Focused) -> `route_focus_change`
        // -> `GameLoop::focus_changed` -> `ScreenGame::apply_focus_change`
        // -> `ScreenGame::enter_screen(Screen::Waiting)`.
        let (mut handler, game, _) = seamed_handler();
        *game.borrow_mut() = game_on_loading("Lobby");
        assert_eq!(game.borrow().screen(), Screen::Loading);

        handler.route_focus_change(false);
        assert_eq!(game.borrow().screen(), Screen::Loading);

        handler.route_focus_change(true);
        assert_eq!(game.borrow().screen(), Screen::Waiting);
        assert_eq!(
            game.borrow().focus_exit_notice(),
            Some(FOCUS_WAITING_EXIT_CAPTION),
            "a named room title posts the pinned Waiting caption"
        );
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

    /// Flat canvas for the induced-failure frame pair. It paints one
    /// constant colour so any changed pixel comes from the popup.
    struct FlatFrameGame;

    impl GameLoop for FlatFrameGame {
        fn lane_key(&mut self, _lane: usize, _down: bool) {}

        fn ingest_snapshot(&mut self, _snapshot: &InputSnapshot) {}

        fn paint(&mut self, frame: &mut o2jam_render::Frame<'_>) {
            frame.buf.fill(0x0020_3040);
        }
    }

    fn paint_handler_frame(handler: &mut WindowHandler) -> Vec<u32> {
        let mut buffer = vec![0u32; backbuffer_words()];
        handler.paint_popup_frame(&mut buffer, DEFAULT_WIDTH as usize, DEFAULT_HEIGHT as usize);
        buffer
    }

    #[test]
    fn supplied_error_paints_a_popup_over_the_window_frame() {
        // seam: no production caller reaches `raise_error_from_supplied`;
        // the induced failure hook stands in for the missing fatal-error
        // caller. The hook -> window redraw paint path is real, and the
        // frame pair below pins the supplied ink.
        let mut handler = WindowHandler::new(Shell::new(Box::new(FlatFrameGame)));
        let scene = paint_handler_frame(&mut handler);
        let decision = handler.raise_error_from_supplied("surface failed", true);
        assert_eq!(
            decision,
            ErrorPopup::ShowMessage("surface failed".to_owned())
        );
        let first = paint_handler_frame(&mut handler);
        assert!(scene != first, "the supplied error must paint a popup");

        handler.raise_error_from_supplied("texture allocation failed", true);
        let second = paint_handler_frame(&mut handler);
        assert!(
            first != second,
            "distinct supplied strings must paint distinct ink"
        );

        handler.raise_error_from_supplied("", true);
        let empty = paint_handler_frame(&mut handler);
        assert!(
            first != empty && second != empty,
            "supplied text must differ from an empty-message popup"
        );
    }

    #[test]
    fn mapped_error_code_paints_its_pinned_text_in_the_popup() {
        // seam: no production caller reaches `raise_error_from_code`; the
        // induced failure hook stands in for the missing fatal-error
        // caller. The code map and the window paint path are real.
        let mut handler = WindowHandler::new(Shell::new(Box::new(FlatFrameGame)));
        let before = paint_handler_frame(&mut handler);
        let decision = handler
            .raise_error_from_code(0x8876_0091, true)
            .expect("a nonzero code stages a popup");
        assert_eq!(
            decision,
            ErrorPopup::ShowMessage("The pixel format was invalid as specified.".to_owned())
        );
        let after = paint_handler_frame(&mut handler);
        assert!(after != before, "the mapped code must paint popup pixels");

        handler.dismiss_error_popup();
        let fallback = handler
            .raise_error_from_code(0xDEAD_BEEF, true)
            .expect("an unmapped code still stages a fallback popup");
        assert_eq!(
            fallback,
            ErrorPopup::ShowMessage(UNRECOGNIZED_ERROR_TEXT.to_owned())
        );
        let fallback_frame = paint_handler_frame(&mut handler);
        assert!(
            after != fallback_frame,
            "the pinned message must differ from the fallback ink"
        );
    }

    // Evidence: docs/pins/input-msg.md:337 (0x5a36d0 shows the box only for a nonzero code).
    #[test]
    fn zero_error_code_leaves_the_window_frame_unchanged() {
        // seam: the popup-or-close helper's silent arm has no winit
        // caller; the induced failure hook is the whole path here.
        let mut handler = WindowHandler::new(Shell::new(Box::new(FlatFrameGame)));
        let before = paint_handler_frame(&mut handler);
        assert_eq!(handler.raise_error_from_code(0, true), None);
        let after = paint_handler_frame(&mut handler);
        assert!(after == before, "code zero must stay silent");
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
    fn window_double_click_queues_the_pinned_notice_when_the_bit_is_set() {
        // Live chain (Tier 1): `main.rs::run_play` -> `Shell::run` ->
        // `WindowHandler::window_event` -> `WindowHandler::route_mouse_input`
        // -> `WindowHandler::stage_double_click_at_native` ->
        // `is_double_click_pair` -> `double_click_notice` ->
        // `WindowHandler::queued_ui_notices`. Bit 2 comes from the real
        // `ScreenGame::apply_myroom_notice` host, not a test flag.
        let mut handler = dialog_host_handler();

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);

        assert_eq!(
            handler.queued_ui_notices(),
            &[UiNotice::new(10, 3, 0, 21, 42)]
        );
    }

    #[test]
    fn one_left_press_queues_no_double_click_notice() {
        // seam: the Win32 hub only gets `WM_LBUTTONDBLCLK` after a
        // second press; this arm is the whole winit-side rule.
        let mut handler = dialog_host_handler();

        handler.route_window_left_press(21.0, 42.0);

        assert!(handler.queued_ui_notices().is_empty());
    }

    #[test]
    fn presses_outside_the_window_queue_no_double_click_notice() {
        // seam: `window_to_native` rejects letterbox/outside points,
        // so no exe-equivalent double-click can be formed there.
        let mut handler = dialog_host_handler();

        handler.route_window_left_press(-20.0, -20.0);
        handler.route_window_left_press(-20.0, -20.0);

        assert!(handler.queued_ui_notices().is_empty());
    }

    #[test]
    fn resize_clears_the_pending_double_click_press() {
        // Our choice, not an exe pin: a resized hit-test surface
        // starts a fresh pair, so a press from the old geometry cannot
        // pair with the next one. Next evidence step: capture two
        // presses spanning a Resized event in the 7.55 window hub.
        let mut handler = dialog_host_handler();

        handler.route_window_left_press(21.0, 42.0);
        handler.set_window_surface_size(800, 600);
        handler.route_window_left_press(21.0, 42.0);

        assert!(
            handler.queued_ui_notices().is_empty(),
            "the resize dropped the first press"
        );
    }

    #[test]
    fn far_apart_press_pair_queues_no_double_click_notice() {
        // seam: the exe hub applies the OS double-click rectangle;
        // this client restates the Windows default because the hub
        // carries no rectangle of its own (no exe number is pinned).
        // A pair 10 px apart exceeds the 4 px default.
        let mut handler = dialog_host_handler();

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(31.0, 42.0);

        assert!(handler.queued_ui_notices().is_empty());
    }

    #[test]
    fn slow_press_pair_queues_no_double_click_notice() {
        // seam: the exe hub applies the OS double-click time; this
        // client restates the Windows 500 ms default because the hub
        // has no timer of its own (no exe number is pinned). A
        // 600 ms pair is outside that window.
        let mut handler = dialog_host_handler();
        handler.last_left_press = Some(LeftPress {
            at: Instant::now() - Duration::from_millis(600),
            x: 21,
            y: 42,
        });

        handler.route_window_left_press(21.0, 42.0);

        assert!(handler.queued_ui_notices().is_empty());
    }

    #[test]
    fn press_pair_with_a_clear_bit_queues_no_double_click_notice() {
        // seam: the exe's `0x548b10` reads bit 2 of the control mode
        // word before it posts anything; no real dialog host means
        // no bit.
        let mut handler = bare_game_handler();

        handler.route_window_left_press(21.0, 42.0);
        handler.route_window_left_press(21.0, 42.0);

        assert!(handler.queued_ui_notices().is_empty());
    }

    #[test]
    fn window_close_request_quits_the_loop() {
        // seam: `WindowEvent::CloseRequested` is the winit stand-in
        // for `WM_DESTROY`; the decision is extracted so a guard and
        // the Tier 3 boot leg name the same exit arm.
        assert_eq!(window_close_disposition(), QuitDisposition::ExitLoop);
    }

    /// Unique scratch directory: tests never touch a real Pictures
    /// folder, and each run owns its own bytes under `~/scratch`.
    struct ScratchRoot(PathBuf);

    impl ScratchRoot {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock follows the epoch")
                .as_nanos();
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/home/arnim"));
            let dir = home
                .join("scratch")
                .join(format!("screenshot-{label}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("scratch root is available");
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for ScratchRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Two-color synthetic frame: red left half, blue right half.
    fn two_color_native_frame() -> Vec<u32> {
        let width = usize::try_from(SCREENSHOT_DIMENSIONS.0).expect("800 fits usize");
        let height = usize::try_from(SCREENSHOT_DIMENSIONS.1).expect("600 fits usize");
        let mut frame = vec![0u32; width * height];
        for (index, word) in frame.iter_mut().enumerate() {
            *word = if index % width < width / 2 {
                o2jam_render::pack_xrgb(240, 16, 16)
            } else {
                o2jam_render::pack_xrgb(16, 16, 240)
            };
        }
        frame
    }

    /// Game probe that reports the scene word source for the key arm.
    struct ScreenProbe(&'static str);

    impl GameLoop for ScreenProbe {
        fn lane_key(&mut self, _lane: usize, _down: bool) {}

        fn ingest_snapshot(&mut self, _snapshot: &InputSnapshot) {}

        fn screen_name(&self) -> &'static str {
            self.0
        }

        fn paint(&mut self, _frame: &mut o2jam_render::Frame<'_>) {}
    }

    fn handler_with_screenshot_root(
        root: &Path,
        remaining: u32,
        screen_name: &'static str,
    ) -> WindowHandler {
        let mut handler = WindowHandler::new(Shell::new(Box::new(ScreenProbe(screen_name))));
        handler.install_screenshot_root(root.to_path_buf(), remaining);
        handler
    }

    #[test]
    fn print_screen_key_writes_the_current_native_frame_as_a_jpeg() {
        // Tier 1 live chain (Tier 0 writer proof): winit KeyboardInput
        // (KeyCode::PrintScreen) -> WindowHandler::route_key_with_text ->
        // WindowHandler::capture_current_frame -> ScreenShotCounter::capture
        // -> PrintScreenAction::write_jpeg -> image::JpegEncoder. The
        // backbuffer is the frame the event loop would present; a headless
        // test supplies it and an explicit scratch root. seam: host My
        // Pictures resolution and a real Print Screen injection stay OPEN.
        let scratch = ScratchRoot::new("two-color");
        let frame = two_color_native_frame();
        let mut handler = handler_with_screenshot_root(scratch.path(), 2, "planet");
        handler.shell.pixels.copy_from_slice(&frame);

        let route = handler.route_key(winit::keyboard::KeyCode::PrintScreen, true);
        assert!(route.captured, "Print Screen is handled at the key seam");

        let path = scratch
            .path()
            .join(SCREENSHOT_FOLDER_SUFFIX)
            .join(print_screen_file_name(0));
        let bytes = std::fs::read(&path).expect("screenshot file exists");
        assert!(!bytes.is_empty(), "the JPEG is not empty");
        assert_eq!(&bytes[..2], &[0xff, 0xd8], "JPEG SOI marker");
        assert_eq!(&bytes[bytes.len() - 2..], &[0xff, 0xd9], "JPEG EOI marker");

        let decoded = image::load_from_memory(&bytes)
            .expect("written bytes decode as JPEG")
            .to_rgb8();
        assert_eq!(decoded.dimensions(), (800, 600));
        let left = decoded.get_pixel(10, 300).0;
        let right = decoded.get_pixel(799 - 10, 300).0;
        assert!(
            left[0] > left[1] + 100 && left[0] > left[2] + 100,
            "left half comes from the supplied red frame: {left:?}"
        );
        assert!(
            right[2] > right[0] + 100 && right[2] > right[1] + 100,
            "right half comes from the supplied blue frame: {right:?}"
        );
        assert_eq!(handler.screenshot_counter.used(), 1, "one name consumed");
    }

    #[test]
    fn failed_print_screen_write_keeps_the_counter_and_surfaces_the_error() {
        // seam: the encoder/file boundary is the whole behavior here;
        // a blocked root stands in for the OS refusing the My Pictures
        // write. No live path can supply that folder in this client.
        let scratch = ScratchRoot::new("failed-write");
        let blocked_root = scratch.path().join("root-is-a-file");
        std::fs::write(&blocked_root, b"not a directory").expect("blocking path exists");
        let mut counter = ScreenShotCounter::new(2);

        let result = counter.capture(0, &blocked_root, &two_color_native_frame());

        assert!(result.is_failed(), "a refused folder is not a written shot");
        assert!(
            result
                .error_message()
                .is_some_and(|message| !message.is_empty()),
            "the error reaches the caller"
        );
        assert_eq!(counter.used(), 0, "a failed write consumes no name");
        assert_eq!(counter.remaining(), 2, "a failed write keeps the shot");
        assert!(!blocked_root.join(SCREENSHOT_FOLDER_SUFFIX).exists());
    }

    // ev_efb6f8dbebe7570f288dd848af9d97c9717b21f4f57050ad38fa1bd529b6bf2
    // (0x57d800 / file 0x17d800, docs/pins/payment.md).
    #[test]
    fn print_screen_in_loading_and_playing_states_writes_no_file() {
        // Tier 1 chain: winit KeyboardInput (Snapshot) ->
        // WindowHandler::route_key_with_text -> capture_current_frame ->
        // ScreenShotCounter::capture. The F8 scene-word table supplies
        // the pinned 10/11 states; no file or name is consumed.
        for screen_name in ["loading", "playing"] {
            let scratch = ScratchRoot::new(screen_name);
            let mut handler = handler_with_screenshot_root(scratch.path(), 2, screen_name);
            handler
                .shell
                .pixels
                .copy_from_slice(&two_color_native_frame());

            handler.route_key(winit::keyboard::KeyCode::PrintScreen, true);

            assert_eq!(handler.screenshot_counter.used(), 0);
            assert!(!scratch.path().join(SCREENSHOT_FOLDER_SUFFIX).exists());
        }
    }
}

#[cfg(test)]
mod cash_charge_host_tests {
    use crate::screens::messenger::{
        cash_charge_publisher_for_ui_event, CashBillPublisher, BINGO_UI_EVENT, CASH_BILL_UI_EVENT,
    };

    #[test]
    fn cash_bill_window_event_passes_the_callers_publisher_through() {
        // Tier 1 seam: the pinned `WM_APP+0x32` code names the cash-bill
        // sheet only. The exe host-selection condition at `0x45e010`
        // is OPEN, so our flow supplies the publisher; this guard
        // rejects an inferred gate without claiming an exe arm.
        let pdbox = CashBillPublisher::Pdbox;
        assert_eq!(
            cash_charge_publisher_for_ui_event(CASH_BILL_UI_EVENT, pdbox),
            Some(pdbox)
        );
        assert_eq!(
            cash_charge_publisher_for_ui_event(BINGO_UI_EVENT, pdbox),
            None
        );
        assert!(CashBillPublisher::selector_provenance().contains("OPEN"));
    }
}
