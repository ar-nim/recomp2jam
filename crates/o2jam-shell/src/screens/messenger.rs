//! Messenger sheet identity and the unpinned PopUpMenu action labels.
//!
//! The six `DIALOG_MESSENGER_*` blocks and their five PopUpMenu button
//! codes come from the shipped `ControlList_Interface.txt` (spec #315
//! evidence map). Their `.bnd` rects stage, but the qualified B8/B6
//! packs only pin the server-unavailable notice: no mounted sheet, no
//! action frame, and no send opcode. This module therefore names the
//! five buttons without inventing a semantic action or a wire shape.

use super::side::{myroom_p0_verdict, MYROOM_ALARM_CODE, MYROOM_ALARM_POPUP_CODE};

// The window module is private to `o2jam-shell`; re-export the arms
// this family's live legs drive through the public messenger seam.
pub use crate::window::{
    display_mode_plan, double_click_notice, error_popup_from_code, error_popup_from_supplied,
    window_close_disposition, ErrorPopup, QuitDisposition, WindowHandler,
};

/// One lobby hosting the Messenger control family. The five codes
/// come from the spec #315 evidence map. MyRoom (B8/B6) and Room
/// (wave-2 `w2-messenger-exe`) have qualified press packs that pin a
/// one-OK liveness sheet; the other three name a capture step instead
/// of a response (tickets #648/#650/#652).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessengerHost {
    /// Room board (`IDC_BUTTON_SROOM_MESSENGER`).
    Room,
    /// Item Shop board.
    Shop,
    /// Avatar MyRoom side screen (the B8 press pack).
    MyRoom,
    /// Waiting room board.
    Waiting,
    /// Music Shop board.
    MusicShop,
}

impl MessengerHost {
    /// `ControlList` control code pinned by the spec evidence map.
    #[must_use]
    pub const fn control_code(self) -> u32 {
        match self {
            Self::Room => 0x1320_0048,
            Self::Shop => 0x1420_00B8,
            Self::MyRoom => super::side::MYROOM_MESSENGER_CODE,
            Self::Waiting => 0x1620_006A,
            Self::MusicShop => 0x1A20_0088,
        }
    }

    /// The one exe-observed response to this host's press, if any.
    /// `None` is an evidence gap, never a dead verdict.
    #[must_use]
    pub const fn exe_response_pin(self) -> Option<&'static str> {
        match self {
            Self::Room => Some("W2 one-OK liveness sheet"),
            Self::MyRoom => Some("B8 one-OK liveness sheet"),
            Self::Shop | Self::Waiting | Self::MusicShop => None,
        }
    }

    /// Next evidence step for an unpinned host arm. A pinned arm keeps
    /// its pack name instead, so a guard can tell them apart.
    #[must_use]
    pub const fn next_evidence(self) -> &'static str {
        match self {
            Self::Room => "W2 pack w2-messenger-exe pins the liveness sheet",
            Self::MyRoom => "B8 pack f9d4a695...f8b pins the liveness sheet",
            Self::Shop | Self::Waiting | Self::MusicShop => {
                "N-HOST: qualified exe press frame and input log on this host board"
            }
        }
    }
}

/// All five host controls in spec order. Three arms stay OPEN
/// (`N-HOST`), so this table names codes and evidence, not live dispatch.
pub const MESSENGER_HOSTS: [MessengerHost; 5] = [
    MessengerHost::Room,
    MessengerHost::Shop,
    MessengerHost::MyRoom,
    MessengerHost::Waiting,
    MessengerHost::MusicShop,
];

/// `DIALOG_MESSENGER_MAIN` block: 77 parsed rects.
pub const MESSENGER_MAIN_BLOCK: &str = "DIALOG_MESSENGER_MAIN";

/// No qualified exe pack mounts the main sheet; its 77 rects are a
/// geometry park only. This marker stays `None` until a capture names
/// the host, frame and every tab (#656).
pub const MESSENGER_MAIN_MOUNT_PIN: Option<&'static str> = None;

/// Next evidence that could mount the main sheet. No list or tab state
/// is invented from the parked art (spec #315 M10).
pub const MESSENGER_MAIN_MOUNT_NEXT_STEP: &str = concat!(
    "N-FRAMES plus N-BACKEND: one qualified exe frame with the main sheet ",
    "mounted, naming the friend, note and four chat tabs at its 77 rects"
);

/// No qualified exe frame shows a friend or note scroll column reaching
/// either end. Channel scroll clamps are a different control family.
pub const MESSENGER_SCROLL_EXE_PIN: Option<&'static str> = None;

/// Next evidence before a Messenger scroll clamp can be claimed (spec
/// #315 M11). The absent backend and absent mount leave no live list.
pub const MESSENGER_SCROLL_CLAMP_NEXT_STEP: &str = concat!(
    "N-FRAMES: one qualified exe pack scrolling a mounted friend and note ",
    "column to both ends, with the clamped frame at each end"
);
/// `DIALOG_MESSENGER_FRIENDADD` block: 6 parsed rects.
pub const MESSENGER_FRIENDADD_BLOCK: &str = "DIALOG_MESSENGER_FRIENDADD";
/// `DIALOG_MESSENGER_QUESTION` block: 7 parsed rects.
pub const MESSENGER_QUESTION_BLOCK: &str = "DIALOG_MESSENGER_QUESTION";
/// `DIALOG_MESSENGER_NOTE` block: 13 parsed rects.
pub const MESSENGER_NOTE_BLOCK: &str = "DIALOG_MESSENGER_NOTE";
/// `DIALOG_MESSENGER_POPUPMENU` block: 6 parsed rects.
pub const MESSENGER_POPUPMENU_BLOCK: &str = "DIALOG_MESSENGER_POPUPMENU";
/// `DIALOG_MESSENGER_INFORMATION` block: 5 parsed rects.
pub const MESSENGER_INFORMATION_BLOCK: &str = "DIALOG_MESSENGER_INFORMATION";

/// `IDC_BUTTON_MESSENGER_NOTE_SEND` in `DIALOG_MESSENGER_NOTE`
/// (`0x89200007` in the shipped ControlList). The Note sheet stages
/// its 13 rects, but no qualified pack mounts it or shows a SEND
/// reply body, so the control stays a labeled no-op.
pub const NOTE_SEND_CODE: u32 = 0x8920_0007;

/// Why SEND sends nothing: the next step is one tapped exe SEND press
/// naming its opcode and body (`N-WIRE`), then the P0 replacement
/// (`N-P0`). Until then no send shape is invented.
const NOTE_SEND_NOOP_LABEL: &str = "IDC_BUTTON_MESSENGER_NOTE_SEND: no exe send pin; no wire";

/// Resolve the NOTE SEND control to its labeled no-op. `None` for
/// every other Note code (e.g. CANCEL), which has no no-op verdict.
#[must_use]
pub fn note_noop_label(code: u32) -> Option<&'static str> {
    (code == NOTE_SEND_CODE).then_some(NOTE_SEND_NOOP_LABEL)
}

/// The five `IDC_BUTTON_MESSENGER_POPUPMENU0n` codes in ControlList
/// order. The shipped names number the buttons but say nothing about
/// what each one does, so dispatch stays a labeled no-op.
pub const POPUP_MENU_CODES: [u32; 5] = [
    0x8A20_0002,
    0x8A20_0003,
    0x8A20_0004,
    0x8A20_0005,
    0x8A20_0006,
];

/// No-op label for one PopUpMenu code: the control name plus the
/// reason it sends nothing. `None` off the five pinned buttons.
const POPUP_MENU_NOOP_LABELS: [&str; POPUP_MENU_CODES.len()] = [
    "IDC_BUTTON_MESSENGER_POPUPMENU01: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU02: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU03: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU04: action unpinned; no wire",
    "IDC_BUTTON_MESSENGER_POPUPMENU05: action unpinned; no wire",
];

/// Resolve one pinned PopUpMenu button to its labeled no-op. A press
/// never queues a send: the exe action frames stay OPEN (`N-BACKEND`).
#[must_use]
pub fn popup_menu_noop_label(code: u32) -> Option<&'static str> {
    POPUP_MENU_CODES
        .iter()
        .position(|pinned| *pinned == code)
        .map(|index| POPUP_MENU_NOOP_LABELS[index])
}

/// Every messenger-owned P0 control that still answers nothing. The set
/// comes from spec #315 M14/M16/M19 and the no-op labels landed in
/// `02ecfd2` and `e123bfc`. The MyRoom host arm is absent: its B8 pack
/// replaced that P0 verdict with the liveness sheet.
pub const MESSENGER_P0_NOOP_CODES: [u32; 8] = [
    NOTE_SEND_CODE,
    POPUP_MENU_CODES[0],
    POPUP_MENU_CODES[1],
    POPUP_MENU_CODES[2],
    POPUP_MENU_CODES[3],
    POPUP_MENU_CODES[4],
    MYROOM_ALARM_CODE,
    MYROOM_ALARM_POPUP_CODE,
];

/// Label that keeps one P0 control inert. `None` off the P0 set.
#[must_use]
pub fn messenger_p0_noop_label(code: u32) -> Option<&'static str> {
    note_noop_label(code)
        .or_else(|| popup_menu_noop_label(code))
        .or_else(|| match code {
            MYROOM_ALARM_CODE | MYROOM_ALARM_POPUP_CODE => myroom_p0_verdict(code),
            _ => None,
        })
}

/// Exe send pin that would replace a P0 no-op. No pin exists today, so
/// a dispatch cannot be added without first naming its evidence here;
/// the P0 guard goes red at that point (`N-P0`).
#[must_use]
pub const fn messenger_p0_send_pin(_code: u32) -> Option<&'static str> {
    None
}

/// Exe scene word pinned with the Room/Playing ids in `options.rs`.
/// The Q&A pin names scene 2; the story calls the shift a Playing
/// offset. Both constants stay here so the contradiction is visible
/// to the guard instead of being resolved by a local guess.
pub const EXE_SCENE_ROOM: u32 = 2;
pub const EXE_SCENE_PLAYING: u32 = 11;

/// `WM_APP+0x32` UI-event codes routed to the three web-sheet
/// openers (`0x45e450`, `0x45ecd0`, `0x45f480`).
pub const CASH_BILL_UI_EVENT: u16 = 0x12;
pub const BINGO_UI_EVENT: u16 = 0x13;
pub const PLAYING_QA_UI_EVENT: u16 = 0x14;

/// Magnitude of the Q&A rectangle shift in exe scene 2. Its sign is
/// not named by the pin and stays OPEN.
pub const QA_PLAYING_RECT_SHIFT: i32 = 50;

/// One hosted sheet. Only host identity, handle and visibility are
/// modeled: this client has no web bridge, so no page is rendered
/// (`#948`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSheet {
    CashBill,
    BingoEvent,
    PlayingQa,
}

/// The UI-event arm that opens a sheet, if it names one.
#[must_use]
pub const fn web_sheet_for_ui_event(code: u16) -> Option<WebSheet> {
    match code {
        CASH_BILL_UI_EVENT => Some(WebSheet::CashBill),
        BINGO_UI_EVENT => Some(WebSheet::BingoEvent),
        PLAYING_QA_UI_EVENT => Some(WebSheet::PlayingQa),
        _ => None,
    }
}

/// What one sheet open did to the two global handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebSheetTransition {
    /// The stored handle was closed before the new one was taken.
    pub closed_previous: bool,
    /// Both stored handles were shown.
    pub visible: bool,
    /// The shown sheet took focus.
    pub focused: bool,
    /// Rectangle shift applied by this open (Q&A, scene 2 only).
    pub qa_rect_shift: i32,
}

/// What the app event did to the two global handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebSheetVisibility {
    /// Both stored handles were hidden.
    pub both_hidden: bool,
}

/// Two global web-window handles plus the visible-sheet state.
/// Handles are opaque words; no browser object is created here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WebSheetHost {
    handles: [Option<u16>; 2],
    active: Option<WebSheet>,
    visible: bool,
    focused: bool,
    qa_rect_shift: i32,
}

impl WebSheetHost {
    /// Empty host: no handle and no sheet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed both global handle words before a sheet opens. The exe
    /// shows and focuses both stored handles when it opens one sheet
    /// (`0x5a8b20`); the second word is only observable this way in
    /// a headless client.
    #[must_use]
    pub fn with_global_handles(first: u16, second: u16) -> Self {
        Self {
            handles: [Some(first), Some(second)],
            ..Self::default()
        }
    }

    /// Close any previous handle, then show and focus the new one.
    /// `handle` is the opaque word the exe stores for the sheet.
    pub fn open(&mut self, sheet: WebSheet, scene_id: u32, handle: u16) -> WebSheetTransition {
        let closed_previous = self.handles[0].take().is_some();
        self.handles[0] = Some(handle);
        self.active = Some(sheet);
        self.visible = true;
        self.focused = true;
        self.qa_rect_shift = if sheet == WebSheet::PlayingQa && scene_id == EXE_SCENE_ROOM {
            QA_PLAYING_RECT_SHIFT
        } else {
            0
        };
        WebSheetTransition {
            closed_previous,
            visible: self.visible,
            focused: self.focused,
            qa_rect_shift: self.qa_rect_shift,
        }
    }

    /// Hide both stored handles on the app event; keep the words
    /// so a later open can close them.
    pub fn hide_on_app_event(&mut self) -> WebSheetVisibility {
        self.visible = false;
        self.focused = false;
        WebSheetVisibility { both_hidden: true }
    }

    /// The currently hosted sheet, if one was opened.
    #[must_use]
    pub const fn active_sheet(&self) -> Option<WebSheet> {
        self.active
    }

    /// Whether the hosted sheets are visible.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Whether the hosted sheet holds focus.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    /// The two stored handle words.
    #[must_use]
    pub const fn handles(&self) -> [Option<u16>; 2] {
        self.handles
    }

    /// Rectangle shift latched by the last open.
    #[must_use]
    pub const fn qa_rect_shift(&self) -> i32 {
        self.qa_rect_shift
    }
}

/// C→S opcode of the relay UHP report (empty body).
pub const UHP_REPORT_OPCODE: u16 = 4046;

/// Success literal written through the relay text sink at `0x18ff70`.
pub const UHP_SUCCESS_CAPTION: &str = "UDP Hole Punching Success~!!";

/// The failure caption is not named by the pin set.
pub const UHP_FAILURE_CAPTION_NEXT_STEP: &str = concat!(
    "N-CAPTION: read the failure string at 0x18ff70 in a qualified ",
    "live-battle UHP capture before naming it"
);

/// Success or failure branch of the UHP start sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UhpStartOutcome {
    Success,
    Failure,
}

/// Caption the pinned branch writes for one outcome. An unnamed
/// failure caption stays `None` instead of being invented.
#[must_use]
pub const fn uhp_start_caption(outcome: UhpStartOutcome) -> Option<&'static str> {
    match outcome {
        UhpStartOutcome::Success => Some(UHP_SUCCESS_CAPTION),
        UhpStartOutcome::Failure => None,
    }
}

/// Opcode the UHP branch sends after its readiness poll.
#[must_use]
pub const fn uhp_report_opcode() -> u16 {
    UHP_REPORT_OPCODE
}

/// Pin of the error-box mapping at `0x5a3820`/file `0x1a3820`:
/// negative DirectDraw/Direct3D values resolve to pinned English
/// text; every other value takes this fallback.
pub const UNRECOGNIZED_ERROR_TEXT: &str = "Unrecognized error value.";

/// Resolve one pinned error code to its message text. The table is
/// the branch table of `0x5a3820`, read from the 7.55 artifact named
/// in `docs/pins/input-msg.md` (analysis ev
/// `8b93.2964.2608.6a22.ffe3.9355.19fe.29cf.1cc2.9619.6096.b193`,
/// file-offset ev `e974.6786.2372.4ae4.8b3e.29be.1586.86d1.e067`).
#[must_use]
pub const fn error_text_for_code(code: u32) -> &'static str {
    match code {
        0x00000000 => "Null point.",
        0x80004001 => "Action not supported.",
        0x80004005 => "Generic failure.",
        0x8007000e => "DirectDraw does not have enough memory to perform the operation.",
        0x80070057 => "One or more of the parameters passed to the function are incorrect.",
        0x88760005 => "This object is already initialized.",
        0x8876000a => "This surface can not be attached to the requested surface.",
        0x88760014 => "This surface can not be detached from the requested surface.",
        0x88760028 => "Support is currently not available.",
        0x88760037 => "An exception was encountered while performing the requested operation.",
        0x8876005a => "Height of rectangle provided is not a multiple of reqd alignment.",
        0x8876005f => "Unable to match primary surface creation request with existing primary surface.",
        0x88760064 => "One or more of the caps bits passed to the callback are incorrect.",
        0x8876006e => "DirectDraw does not support the provided cliplist.",
        0x88760078 => "DirectDraw does not support the requested mode.",
        0x88760082 => "DirectDraw received a pointer that was an invalid DIRECTDRAW object.",
        0x88760091 => "The pixel format was invalid as specified.",
        0x88760096 => "Rectangle provided was invalid.",
        0x887600a0 => "Operation could not be carried out because one or more surfaces are locked.",
        0x887600aa => "There is no 3D present.",
        0x887600b4 => "Operation could not be carried out because there is no alpha accleration hardware present or available.",
        0x887600cd => "No cliplist available.",
        0x887600d2 => "Operation could not be carried out because there is no color conversion hardware present or available.",
        0x887600d4 => "Create function called without DirectDraw object method SetCooperativeLevel being called.",
        0x887600d7 => "Surface doesn't currently have a color key",
        0x887600dc => "Operation could not be carried out because there is no hardware support of the destination color key.",
        0x887600e1 => "Operation requires the application to have exclusive mode but the application does not have exclusive mode.",
        0x887600e6 => "Flipping visible surfaces is not supported.",
        0x887600f0 => "There is no GDI present.",
        0x887600fa => "Operation could not be carried out because there is no hardware present or available.",
        0x887600ff => "Requested item was not found.",
        0x88760104 => "Operation could not be carried out because there is no overlay hardware present or available.",
        0x88760118 => "Operation could not be carried out because there is no appropriate raster op hardware present or available.",
        0x88760122 => "Operation could not be carried out because there is no rotation hardware present or available.",
        0x88760136 => "Operation could not be carried out because there is no hardware support for stretching.",
        0x8876013c => "DirectDrawSurface is not in 4 bit color palette and the requested operation requires 4 bit color palette.",
        0x8876013d => "DirectDrawSurface is not in 4 bit color index palette and the requested operation requires 4 bit color index palette.",
        0x88760140 => "DirectDrawSurface is not in 8 bit color mode and the requested operation requires 8 bit color.",
        0x8876014a => "Operation could not be carried out because there is no texture mapping hardware present or available.",
        0x8876014f => "Operation could not be carried out because there is no hardware support for vertical blank synchronized operations.",
        0x88760154 => "Operation could not be carried out because there is no hardware support for zbuffer blitting.",
        0x8876015e => "Overlay surfaces could not be z layered based on their BltOrder because the hardware does not support z layering of overlays.",
        0x88760168 => "The hardware needed for the requested operation has already been allocated.",
        0x8876017c => "DirectDraw does not have enough memory to perform the operation.",
        0x8876017e => "The hardware does not support clipped overlays.",
        0x88760180 => "Can only have ony color key active at one time for overlays.",
        0x88760183 => "Access to this palette is being refused because the palette is already locked by another thread.",
        0x88760190 => "No src color key specified for this operation.",
        0x8876019a => "This surface is already attached to the surface it is being attached to.",
        0x887601a4 => "This surface is already a dependency of the surface it is being made a dependency of.",
        0x887601ae => "Access to this surface is being refused because the surface is already locked by another thread.",
        0x887601b8 => "Access to surface refused because the surface is obscured.",
        0x887601c2 => "Access to this surface is being refused because the surface memory is gone. The DirectDrawSurface object representing this surface should have Restore called on it.",
        0x887601cc => "The requested surface is not attached.",
        0x887601d6 => "Height requested by DirectDraw is too large.",
        0x887601e0 => "Size requested by DirectDraw is too large, but the individual height and width are OK.",
        0x887601ea => "Width requested by DirectDraw is too large.",
        0x887601fe => "FOURCC format requested is unsupported by DirectDraw.",
        0x88760208 => "Bitmask in the pixel format requested is unsupported by DirectDraw.",
        0x88760219 => "Vertical blank is in progress.",
        0x8876021c => "Informs DirectDraw that the previous Blt which is transfering information to or from this Surface is incomplete.",
        0x88760230 => "Rectangle provided was not horizontally aligned on required boundary.",
        0x88760231 => "The GUID passed to DirectDrawCreate is not a valid DirectDraw driver identifier.",
        0x88760232 => "A DirectDraw object representing this driver has already been created for this process.",
        0x88760233 => "A hardware-only DirectDraw object creation was attempted but the driver did not support any hardware.",
        0x88760234 => "This process already has created a primary surface.",
        0x88760235 => "Software emulation not available.",
        0x88760236 => "Region passed to Clipper::GetClipList is too small.",
        0x88760237 => "An attempt was made to set a cliplist for a clipper object that is already monitoring an hwnd.",
        0x88760238 => "No clipper object attached to surface object.",
        0x88760239 => "Clipper notification requires an HWND or no HWND has previously been set as the CooperativeLevel HWND.",
        0x8876023a => "HWND used by DirectDraw CooperativeLevel has been subclassed, this prevents DirectDraw from restoring state.",
        0x8876023b => "The CooperativeLevel HWND has already been set. It can not be reset while the process has surfaces or palettes created.",
        0x8876023c => "No palette object attached to this surface.",
        0x8876023d => "No hardware support for 16 or 256 color palettes.",
        0x8876023e => "Return if a clipper object is attached to the source surface passed into a BltFast call.",
        0x8876023f => "No blitter hardware present.",
        0x88760240 => "No DirectDraw ROP hardware.",
        0x88760241 => "Returned when GetOverlayPosition is called on a hidden overlay.",
        0x88760242 => "Returned when GetOverlayPosition is called on an overlay that UpdateOverlay has never been called on to establish a destination.",
        0x88760243 => "Returned when the position of the overlay on the destination is no longer legal for that destination.",
        0x88760244 => "Returned when an overlay member is called for a non-overlay surface.",
        0x88760245 => "An attempt was made to set the cooperative level when it was already set to exclusive.",
        0x88760246 => "An attempt has been made to flip a surface that is not flippable.",
        0x88760247 => "Can't duplicate primary & 3D surfaces, or surfaces that are implicitly created.",
        0x88760248 => "Surface was not locked.  An attempt to unlock a surface that was not locked at all, or by this process, has been attempted.",
        0x88760249 => "Windows can not create any more DCs.",
        0x8876024a => "No DC was ever created for this surface.",
        0x8876024b => "This surface can not be restored because it was created in a different mode.",
        0x8876024c => "This surface can not be restored because it is an implicitly created surface.",
        0x8876024d => "The surface being used is not a palette-based surface.",
        0x8876081e => "The current texture filters cannot be used together.",
        0x88760821 => "The currently set render states cannot be used together.",
        0x88760826 => "The current textures cannot be used simultaneously.",
        0xc8770bb8 => "The requested operation failed because the system is out of memory.",
        _ => UNRECOGNIZED_ERROR_TEXT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn note_send_stays_a_labeled_noop_without_an_exe_send_pin() {
        // seam: the whole claim is the no-op contract — the 13-rect
        // Note sheet stages, but the qualified packs never mount it
        // or show a SEND action frame (`N-WIRE`, then `N-P0`), so no
        // send opcode or body exists to reach from boot.
        let label = note_noop_label(NOTE_SEND_CODE).expect("SEND has a no-op label");
        assert!(label.contains("IDC_BUTTON_MESSENGER_NOTE_SEND"), "{label}");
        assert!(label.contains("no exe send pin"), "{label}");
        assert!(label.contains("no wire"), "{label}");
        assert_eq!(
            note_noop_label(0x8920_0008),
            None,
            "CANCEL is not the SEND control"
        );
    }

    #[test]
    fn popup_menu_buttons_stay_labeled_noops_off_the_pinned_sheet() {
        // seam: this is the whole action contract — the qualified
        // packs pin five button codes but no action frame, so no
        // dispatch or wire shape exists to reach from boot.
        let labels: Vec<&str> = POPUP_MENU_CODES
            .iter()
            .map(|code| {
                popup_menu_noop_label(*code)
                    .unwrap_or_else(|| panic!("button {code:#x} has no label"))
            })
            .collect();
        assert_eq!(labels.len(), 5, "five ControlList buttons");
        for label in &labels {
            assert!(label.contains("action unpinned"), "{label}");
            assert!(label.contains("no wire"), "{label}");
        }
        assert_eq!(
            popup_menu_noop_label(0x8A20_0007),
            None,
            "backdrop rect has no action label"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn messenger_p0_noops_stay_unpinned_and_answer_nothing() {
        // seam: this is the whole P0 contract. The six Messenger sheets park
        // with no exe mount (`N-MOUNT`) and no send pack (`N-WIRE`), so no
        // boot chain can reach a dispatch; the labels and the pin marker are
        // the complete action surface. Reviewers flip this guard when a pin
        // lands, before any dispatch is written.
        use super::super::side::{
            myroom_p0_verdict, MYROOM_ALARM_CODE, MYROOM_ALARM_POPUP_CODE, MYROOM_MESSENGER_CODE,
            MYROOM_WEBAVATAR_CODE,
        };
        assert_eq!(
            MESSENGER_P0_NOOP_CODES,
            [
                NOTE_SEND_CODE,
                0x8A20_0002,
                0x8A20_0003,
                0x8A20_0004,
                0x8A20_0005,
                0x8A20_0006,
                MYROOM_ALARM_CODE,
                MYROOM_ALARM_POPUP_CODE,
            ],
            "P0 set named by spec #315 M14/M16/M19 and e123bfc"
        );
        for code in MESSENGER_P0_NOOP_CODES {
            let label = messenger_p0_noop_label(code)
                .unwrap_or_else(|| panic!("P0 control {code:#X} lost its label"));
            assert!(
                label.contains("no wire") || label.contains("paints nothing"),
                "P0 control {code:#X} still answers nothing: {label}"
            );
            assert_eq!(
                messenger_p0_send_pin(code),
                None,
                "P0 control {code:#X} has no exe send pin"
            );
        }
        assert_eq!(
            messenger_p0_noop_label(MYROOM_WEBAVATAR_CODE),
            None,
            "a MyRoom P0 outside the Messenger family has no Messenger label"
        );
        assert_eq!(
            myroom_p0_verdict(MYROOM_MESSENGER_CODE),
            None,
            "the B8 pack replaced the host P0 arm; the sheet family P0 stays"
        );
    }

    #[test]
    fn messenger_scroll_clamps_stay_na_without_a_mounted_list() {
        // seam: the whole claim is the NA label. No qualified pack mounts a
        // friend or note column, so there is no live list whose ends a
        // clamp could bound; the channel clamp guards pin a different
        // screen. No scroll opcode or page state is claimed (#315 M11).
        assert_eq!(
            MESSENGER_SCROLL_EXE_PIN, None,
            "no exe frame pins a Messenger scroll end"
        );
        assert!(
            MESSENGER_SCROLL_CLAMP_NEXT_STEP.contains("N-FRAMES"),
            "activation step names the missing capture: {}",
            MESSENGER_SCROLL_CLAMP_NEXT_STEP
        );
        for code in POPUP_MENU_CODES {
            assert_eq!(
                messenger_p0_send_pin(code),
                None,
                "an unpinned menu button cannot carry a scroll send: {code:#x}"
            );
        }
    }

    #[test]
    fn cash_bill_sheet_closes_the_prior_web_handle_before_it_shows() {
        // seam: the exe's `0x5a5190` UI-event arm has no winit
        // counterpart, so the close-then-show host decision is the
        // whole behavior a headless client can observe. Web content
        // stays blank by owner rule (#948).
        let mut host = WebSheetHost::new();
        let first = host.open(WebSheet::CashBill, EXE_SCENE_ROOM, 101);
        assert!(
            first.visible && first.focused,
            "new sheet shows and focuses"
        );
        assert!(!first.closed_previous, "no handle to close on first open");

        let second = host.open(WebSheet::BingoEvent, EXE_SCENE_ROOM, 102);
        assert!(second.closed_previous, "prior handle closes first");
        assert_eq!(
            host.handles(),
            [Some(102), None],
            "the new handle replaces the old one"
        );
        assert_eq!(host.active_sheet(), Some(WebSheet::BingoEvent));
    }

    #[test]
    fn both_global_web_sheets_hide_on_the_app_event() {
        // seam: the Win32 `WM_APP+0x31` app event has no winit
        // counterpart, so `hide_on_app_event` is the whole host
        // visibility behavior in this client.
        let mut host = WebSheetHost::new();
        host.open(WebSheet::CashBill, EXE_SCENE_ROOM, 201);
        assert!(host.is_visible() && host.is_focused());

        let hidden = host.hide_on_app_event();
        assert!(hidden.both_hidden, "both stored handles hide");
        assert!(!host.is_visible(), "closing a sheet is visible");
        assert!(!host.is_focused(), "hidden sheet does not hold focus");
        assert_eq!(
            host.handles(),
            [Some(201), None],
            "hide keeps the stored handle for the next show"
        );
    }

    #[test]
    fn playing_qa_sheet_gets_the_pinned_rect_shift() {
        // seam: the Win32 UI-event arm has no winit counterpart, so
        // the pinned rect shift at open is the whole observable
        // behavior.
        // CONFLICT: the pin names scene 2, while `options.rs` pins
        // scene 2 as Room and Playing as 11; this guard follows the
        // exe pin and leaves the story's Playing wording OPEN.
        let mut host = WebSheetHost::new();
        let scene_two = host.open(WebSheet::PlayingQa, EXE_SCENE_ROOM, 301);
        assert_eq!(scene_two.qa_rect_shift, QA_PLAYING_RECT_SHIFT);
        assert_eq!(host.qa_rect_shift(), 50, "scene 2 shifts the rect");

        let playing = host.open(WebSheet::PlayingQa, EXE_SCENE_PLAYING, 302);
        assert_eq!(playing.qa_rect_shift, 0, "other scenes keep the rect");
    }

    #[test]
    fn ui_event_codes_name_the_three_web_sheet_hosts() {
        // seam: the UI-event code table is the whole claim. The exe
        // reaches it from one window hub (`0x5a5190`), which this
        // client has no Win32 message pump to call.
        assert_eq!(web_sheet_for_ui_event(0x12), Some(WebSheet::CashBill));
        assert_eq!(web_sheet_for_ui_event(0x13), Some(WebSheet::BingoEvent));
        assert_eq!(web_sheet_for_ui_event(0x14), Some(WebSheet::PlayingQa));
        assert_eq!(web_sheet_for_ui_event(0x15), None);
    }

    #[test]
    fn uhp_start_reports_4046_with_the_pinned_success_caption() {
        // seam: the relay UHP branch is battle-only and has no Rust
        // caller yet; caption literal + opcode are the whole pin that
        // can be staged here. Failure text stays OPEN.
        assert_eq!(uhp_report_opcode(), 4046);
        assert_eq!(
            uhp_start_caption(UhpStartOutcome::Success),
            Some("UDP Hole Punching Success~!!")
        );
        assert_eq!(uhp_start_caption(UhpStartOutcome::Failure), None);
        assert!(
            UHP_FAILURE_CAPTION_NEXT_STEP.contains("0x18ff70"),
            "failure text needs the next sink capture"
        );
    }

    #[test]
    fn direct_draw_codes_resolve_to_their_pinned_text() {
        // seam: `0x5a3820` is a pure code-to-text branch table called
        // only by `0x5a36d0`; no runtime caller reaches it in this
        // client, so the table is the whole behavior.
        assert_eq!(error_text_for_code(0), "Null point.");
        assert_eq!(
            error_text_for_code(0x8876_0091),
            "The pixel format was invalid as specified."
        );
        assert_eq!(
            error_text_for_code(0x8876_01c2),
            "Access to this surface is being refused because the surface memory is gone. The DirectDrawSurface object representing this surface should have Restore called on it."
        );
        assert_eq!(
            error_text_for_code(0x8877_00c2),
            UNRECOGNIZED_ERROR_TEXT,
            "an unmapped code keeps the pinned fallback"
        );
    }
}
