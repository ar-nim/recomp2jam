//! Messenger sheet identity and the unpinned PopUpMenu action labels.
//!
//! The six `DIALOG_MESSENGER_*` blocks and their five PopUpMenu button
//! codes come from the shipped `ControlList_Interface.txt` (spec #315
//! evidence map). Their `.bnd` rects stage, but the qualified B8/B6
//! packs only pin the server-unavailable notice: no mounted sheet, no
//! action frame, and no send opcode. This module therefore names the
//! five buttons without inventing a semantic action or a wire shape.

use std::net::SocketAddrV4;
use std::time::Duration;

use super::side::{myroom_p0_verdict, MYROOM_ALARM_CODE, MYROOM_ALARM_POPUP_CODE};
use o2jam_net::relay::P2PListResponse;
use o2jam_render::Rect;

// The window module is private to `o2jam-shell`; re-export the arms
// this family's live legs drive through the public messenger seam.
pub use crate::window::{
    billing_window_arm, coupon_host_arm, display_mode_plan, double_click_notice,
    error_popup_from_code, error_popup_from_supplied, print_screen_file_name,
    print_screen_relative_path, web_sheet_visibility_arm, window_close_disposition,
    BillingWindowArm, CouponHostArm, ErrorPopup, PrintScreenAction, QuitDisposition,
    ScreenShotCounter, WebSheetVisibilityArm, WindowHandler, BILLING_PAYMENT2_STATE_WORD,
    BILLING_PAYMENT_STATE_WORD, COUPON_HOST_STATE_WORD, COUPON_PAGE_SELECTOR,
    PRINT_SCREEN_SUPPRESSED_STATES, SCREENSHOT_DIMENSIONS, SCREENSHOT_FOLDER_SUFFIX,
    SCREENSHOT_QUALITY, WM_APP_HIDE_WEB_SHEETS,
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

/// Vftable identity written by the Room friend-list constructor.
/// Exe `0x57f5f0` / file `0x17f5f0`, REA evidence
/// `b8a0.bed9.8a19.eb11.ff6a.69c3.df8e.c533.3861.6cee.1cc2.6a1c.2b73.f8b1.4d1c.5704`.
pub const ROOM_FRIEND_VFTABLE: &str = "CWebFriendCtrl::vftable";

/// Three trailing dwords the same constructor writes at object dword
/// slots `0x407`/`0x408`/`0x409` (byte offsets `0x101c`/`0x1020`/
/// `0x1024`): one, then two zeros.
pub const ROOM_FRIEND_TRAILING_FLAGS: [u32; 3] = [1, 0, 0];

/// The hosted page itself stays outside this client. No
/// `gamefine_main.asp` document, script variable, or wire capture is
/// claimed; the next evidence step is `#405` in `docs/web-surfaces.md`.
pub const ROOM_FRIEND_PAGE_BOUNDARY_OPEN: &str =
    "OPEN: no gamefine_main.asp render or wire capture (#405)";

/// Native stand-in for the exe's Room friend-list host. Only the
/// pinned constructor words are represented: roster rows and gift-on
/// state stay in `ChatState`, and the browser page is not rendered.
/// `ChatState` stores the whole host as `Option`, so this type always
/// holds a mounted word set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRoomFriendHost {
    vftable: &'static str,
    trailing_flags: [u32; 3],
}

impl NativeRoomFriendHost {
    /// Install the friend vftable identity and write the three trailing
    /// flags in the order pinned at exe `0x57f5f0`. Named `mount` to
    /// avoid a `Default` word set; unmounted is `None` in `ChatState`.
    #[must_use]
    pub const fn mount() -> Self {
        Self {
            vftable: ROOM_FRIEND_VFTABLE,
            trailing_flags: ROOM_FRIEND_TRAILING_FLAGS,
        }
    }

    /// Vftable installed by the constructor.
    #[must_use]
    pub const fn vftable(&self) -> &'static str {
        self.vftable
    }

    /// Trailing flag words in object-slot order.
    #[must_use]
    pub const fn trailing_flags(&self) -> [u32; 3] {
        self.trailing_flags
    }

    /// Explicit evidence boundary for the page this client does not host.
    #[must_use]
    pub const fn page_boundary_open(&self) -> &'static str {
        ROOM_FRIEND_PAGE_BOUNDARY_OPEN
    }
}

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

/// Mode word `0x77ee84 == 2` skips both Result-time web arms.
/// The selector word `0x77ee50` chooses hot-time at zero and the
/// ticket arm when nonzero (`ev_64c08939…` and the three qualified
/// solo Result stub logs recorded in `docs/web-url-archive-audit.md`).
pub const RESULT_EVENT_MODE_BATTLE: u32 = 2;

/// One native event/help stand-in. The enum names only pinned pages;
/// no page body, question, reward, or outbound link is represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEventPage {
    /// `0x45ea60`: bingo event; `USER_INDEX_ID=<id>` is appended.
    Bingo,
    /// `0x45f190`: 1:1 Q&A; `USER_INDEX_ID=<id>` is appended.
    InGameQa,
    /// `0x4c1b90` zero-selector arm and `0x4f5730` second host.
    HotTime,
    /// `0x4c1b90` armed selector arm.
    TicketAdd,
}

impl NativeEventPage {
    /// Short caption of the pinned page, not page content.
    #[must_use]
    pub const fn caption(self) -> &'static str {
        match self {
            Self::Bingo => "2009 bingo event - native notice",
            Self::InGameQa => "1:1 QnA help - native stand-in",
            Self::HotTime => "hot-time event - native notice",
            Self::TicketAdd => "event ticket add - native notice",
        }
    }

    /// The literal URL shape the exe pins. `<id>` stays a placeholder:
    /// the shell stores no user-index value for these pages. The two
    /// event-time arms keep only their literal path because their
    /// assembled suffix and title source are OPEN.
    #[must_use]
    pub const fn pinned_url(self) -> &'static str {
        match self {
            Self::Bingo => concat!(
                "http://o2jam.nopp.co.kr/client/event/2009/01/01_bingo/index.asp",
                "?USER_INDEX_ID=<id>"
            ),
            Self::InGameQa => concat!(
                "http://o2jam.nopp.co.kr/client/mtm/ingame_1n1qna01.phtml",
                "?USER_INDEX_ID=<id>"
            ),
            Self::HotTime => "http://o2jam.nopp.co.kr/client/event/2009/06_moa/hot_time.php",
            Self::TicketAdd => "http://o2jam.nopp.co.kr/client/event/2009/06_moa/ticket_add.php",
        }
    }

    /// The evidence gap printed under the caption. No missing value
    /// is guessed to make the stand-in look complete.
    #[must_use]
    pub const fn open_note(self) -> &'static str {
        match self {
            Self::Bingo => "rules, rewards and page fields OPEN",
            Self::InGameQa => "questions and outbound links OPEN",
            Self::HotTime => "caller appends <id>; separator/key OPEN",
            Self::TicketAdd => "USER_INDEX_ID=<id>&title=<title>; title source OPEN",
        }
    }
}

/// Result entry's two web arms from `FUN_004c1b90`. Battle mode
/// (`0x77ee84 == 2`) skips both; a zero selector takes hot-time and a
/// nonzero selector takes the ticket page. Only the zero arm was seen
/// in the three qualified solo boots, so the armed arm keeps its
/// OPEN title source.
#[must_use]
pub const fn result_event_page(mode_word: u32, selector_word: u32) -> Option<NativeEventPage> {
    if mode_word == RESULT_EVENT_MODE_BATTLE {
        return None;
    }
    if selector_word == 0 {
        Some(NativeEventPage::HotTime)
    } else {
        Some(NativeEventPage::TicketAdd)
    }
}

/// Stand-in band for one native event/help notice. No event sheet
/// rect from the exe is pinned, so the band stays in the top margin
/// above the Result readouts it must not hide. Presentation is ours
/// by design (owner decision `e8d01`), and the label says so on
/// screen.
pub const NATIVE_EVENT_NOTICE_RECT: Rect = Rect {
    x: 100,
    y: 4,
    w: 600,
    h: 60,
};
pub const NATIVE_EVENT_NOTICE_TONE: u32 = 0x0067_BAF0;
pub const NATIVE_EVENT_NOTICE_LINE_PITCH: i32 = 16;

/// Band for the native coupon form. Presentation is ours by design
/// (owner decision `e8d01`); it sits below the event notice band.
pub const COUPON_FORM_RECT: Rect = Rect {
    x: 100,
    y: 72,
    w: 600,
    h: 96,
};

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

/// Publisher arm of our native cash-charge flow. The `0x12` UI event
/// is pinned; the exe's host-selection condition at `0x45e010` is
/// OPEN. The caller supplies the publisher, so this selector is
/// ours-by-design and claims no exe gate.
#[must_use]
pub const fn cash_charge_publisher_for_ui_event(
    code: u16,
    publisher: CashBillPublisher,
) -> Option<CashBillPublisher> {
    match web_sheet_for_ui_event(code) {
        Some(WebSheet::CashBill) => Some(publisher),
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

    /// Whether an opaque handle is stored, the gate the exe's
    /// `WM_APP+0x31` hide arm tests before calling `0x5a8ba0`.
    #[must_use]
    pub const fn has_stored_handle(&self) -> bool {
        self.handles[0].is_some() || self.handles[1].is_some()
    }
}

/// Billing hosts, paths and publisher identities are re-exports of the
/// pinned rows in `o2jam-net::wallet::BILLING_PAGE_TABLE`. The shell
/// keeps no literal copy of a host, page path or publisher host set.
pub use o2jam_net::wallet::{
    BillingPublisher as CashBillPublisher, BillingWebPage, BILLING_HOST, CASH_BILL_HOSTS,
    CASH_BILL_PATH, COUPON_HOST,
};

/// One observable step of a billing web rebuild. `0x4f3d00` and
/// `0x4f5020` close a stored window handle before calling their
/// URL opener; the order is the behavior under guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillingWebEvent {
    /// The stored handle was closed and cleared.
    ClosedPrevious(u16),
    /// The rebuilt URL was mounted on a new handle.
    Mounted { handle: u16, url: String },
}

/// The one billing web-window handle word, held across rebuilds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BillingWebHandle {
    handle: Option<u16>,
}

impl BillingWebHandle {
    /// No stored window yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the handle word before a rebuild. The exe only takes the
    /// close arm when this word is set.
    #[must_use]
    pub const fn with_handle(handle: u16) -> Self {
        Self {
            handle: Some(handle),
        }
    }

    /// Close the stored handle, rebuild the URL from the session id,
    /// then mount it. Returns the steps in the order they occurred.
    pub fn rebuild(
        &mut self,
        page: BillingWebPage,
        user_index_id: &str,
        handle: u16,
    ) -> Vec<BillingWebEvent> {
        let mut steps = Vec::with_capacity(2);
        if let Some(previous) = self.handle.take() {
            steps.push(BillingWebEvent::ClosedPrevious(previous));
        }
        self.handle = Some(handle);
        steps.push(BillingWebEvent::Mounted {
            handle,
            url: page.url(user_index_id),
        });
        steps
    }

    /// Rebuild a cash-bill charge for one explicit publisher. The
    /// close-then-mount order is the merged Payment rebuild arm; the
    /// host comes from the pinned table, never from an invented gate.
    pub fn rebuild_cash_bill_charge(
        &mut self,
        publisher: CashBillPublisher,
        user_index_id: &str,
        handle: u16,
    ) -> Vec<BillingWebEvent> {
        self.rebuild(BillingWebPage::CashBill(publisher), user_index_id, handle)
    }

    /// The handle the host currently holds.
    #[must_use]
    pub const fn handle(&self) -> Option<u16> {
        self.handle
    }
}

/// `WM_APP+0x32` hub message that reaches the state-13 opener.
pub const COUPON_UI_EVENT: u32 = 0x232;

/// Raw SHA-256 of the surviving `coupon_in.phtml?myid=` body pinned in
/// `docs/pins/payment.md` (`a283a1d9…aaa67`). It names the archived
/// form contract; the dead host is not reachable from this client.
pub const ARCHIVED_COUPON_BODY_SHA256: &str =
    "a283a1d95fe293ab43a485ee925b9eedfbcbc7c157c63fc9fa3fc6368d8aaa67";

/// Form name from the archived body. The native replacement keeps the
/// field names so a report-back can pin the same shape.
pub const COUPON_FORM_NAME: &str = "frmCouponCheck";
/// `cp_id` select field from the archived body.
pub const COUPON_ID_FIELD: &str = "cp_id";
/// `frmCouponNum` text field from the archived body (30-char cap).
pub const COUPON_NUMBER_FIELD: &str = "frmCouponNum";

/// One `cp_id` select value from the archived body, in page order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouponKind {
    /// 교학사 총정리.
    Khcjr,
    /// 테일즈런너 가이드북2.
    Trgdb,
    /// 테일즈런너 수학 킹왕짱.
    Trkwj,
    /// 테일즈런너 이야기책 마법전사.
    Triyg,
    /// 10곡 1일 무료이용권.
    Viliv,
}

impl CouponKind {
    /// All five select values, in archived page order.
    pub const ALL: [Self; 5] = [
        Self::Khcjr,
        Self::Trgdb,
        Self::Trkwj,
        Self::Triyg,
        Self::Viliv,
    ];

    /// The exact `option value` from the archived form.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Khcjr => "KHCJR",
            Self::Trgdb => "TRGDB",
            Self::Trkwj => "TRKWJ",
            Self::Triyg => "TRIYG",
            Self::Viliv => "VILIV",
        }
    }
}

/// `maxlength` of the archived `frmCouponNum` text input.
pub const COUPON_NUMBER_MAX_CHARS: usize = 30;

/// Notice shown when a valid form is reported back locally. The page
/// host is dead; this client never claims a coupon was accepted.
pub const COUPON_REPORT_BACK_NOTICE: &str =
    "coupon report-back staged locally; dead host, no acceptance claim";

/// Why an entered coupon number cannot be submitted, in the order
/// the archived page checks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouponNumberError {
    /// The page's first check: no `-` anywhere in the value.
    MissingHyphen,
    /// The page's second check: not 11 or 17 characters.
    WrongLength,
    /// The page's regex check: groups are not five alphanumerics.
    WrongGroups,
}

/// Native stand-in for the archived `frmCouponCheck` fields. The
/// number follows the page's key-up uppercasing and 30-character cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CouponForm {
    kind: CouponKind,
    number: String,
}

impl CouponForm {
    /// Empty form with the select's first archived option.
    #[must_use]
    pub fn new() -> Self {
        Self {
            kind: CouponKind::Khcjr,
            number: String::new(),
        }
    }

    /// Choose one of the five pinned `cp_id` values.
    pub fn select(&mut self, kind: CouponKind) {
        self.kind = kind;
    }

    /// Chosen `cp_id` value.
    #[must_use]
    pub const fn kind(&self) -> CouponKind {
        self.kind
    }

    /// Current `frmCouponNum` text.
    #[must_use]
    pub fn number(&self) -> &str {
        &self.number
    }

    /// Type one character, as the page's key-up handler uppercases it.
    pub fn push_char(&mut self, character: char) {
        let remaining = COUPON_NUMBER_MAX_CHARS.saturating_sub(self.number.chars().count());
        self.number.extend(character.to_uppercase().take(remaining));
    }

    /// Backspace one character.
    pub fn pop_char(&mut self) {
        self.number.pop();
    }

    /// Submit the form to the local report-back seam. No opcode and
    /// no host reply are produced: the archived host is unreachable.
    #[must_use]
    pub fn submit(&self) -> CouponSubmit {
        match validate_coupon_number(&self.number) {
            Ok(()) => CouponSubmit::Reported(CouponReportBack {
                kind: self.kind,
                number: self.number.clone(),
            }),
            Err(error) => CouponSubmit::Rejected(error),
        }
    }
}

/// Outcome of one native coupon submit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CouponSubmit {
    /// The archived page's validation stops the submit.
    Rejected(CouponNumberError),
    /// The pinned fields are reported back to the local notice seam.
    Reported(CouponReportBack),
}

/// The pinned fields carried to the local notice seam. This is a
/// report-back, never a receipt from the dead coupon host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CouponReportBack {
    /// Submitted `cp_id` select value.
    pub kind: CouponKind,
    /// Submitted uppercased `frmCouponNum` value.
    pub number: String,
}

impl CouponReportBack {
    /// Notice text shown with the reported fields.
    #[must_use]
    pub const fn notice(&self) -> &'static str {
        COUPON_REPORT_BACK_NOTICE
    }

    /// One line naming the two pinned fields.
    #[must_use]
    pub fn field_line(&self) -> String {
        format!(
            "{}={} {}={}",
            COUPON_ID_FIELD,
            self.kind.code(),
            COUPON_NUMBER_FIELD,
            self.number
        )
    }
}

impl Default for CouponForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate one coupon number with the archived page's three checks.
pub fn validate_coupon_number(value: &str) -> Result<(), CouponNumberError> {
    if !value.contains('-') {
        return Err(CouponNumberError::MissingHyphen);
    }
    if value.len() != 11 && value.len() != 17 {
        return Err(CouponNumberError::WrongLength);
    }
    if !has_archived_groups(value) {
        return Err(CouponNumberError::WrongGroups);
    }
    Ok(())
}

/// One archived regex group: exactly five ASCII alphanumerics.
fn is_ascii_group(group: &str) -> bool {
    let bytes = group.as_bytes();
    bytes.len() == 5 && bytes.iter().all(|byte| byte.is_ascii_alphanumeric())
}

/// Either `AAAAA-BBBBB` or `AAAAA-BBBBB-CCCCC`.
fn has_archived_groups(value: &str) -> bool {
    let groups: Vec<_> = value.split('-').collect();
    match groups.as_slice() {
        [first, second] => is_ascii_group(first) && is_ascii_group(second),
        [first, second, third] => {
            is_ascii_group(first) && is_ascii_group(second) && is_ascii_group(third)
        }
        _ => false,
    }
}

/// C→S opcode of the relay UHP report (empty body).
pub const UHP_REPORT_OPCODE: u16 = 4046;

/// Success literal written through the relay text sink at `0x18ff70`.
pub const UHP_SUCCESS_CAPTION: &str = "UDP Hole Punching Success~!!";

/// Failure literal written through the same pinned text sink.
pub const UHP_FAILURE_CAPTION: &str = "UHP Start 실패";

/// The exe walks six-byte endpoint rows and four-byte member words
/// (VA `0x58d580`, file `0x18d580`, ev
/// `e8ab.954b.f7d6.fbb8.9a20.6214.83b1.3652.7784.512e.8cb4.649d`).
pub const UHP_ENDPOINT_STRIDE_BYTES: usize = 6;
pub const UHP_MEMBER_STRIDE_BYTES: usize = 4;

/// The readiness poller waits one `Sleep(300)` per pending step.
pub const UHP_POLL_INTERVAL: Duration = Duration::from_millis(300);

/// Success or failure branch of the UHP start sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UhpStartOutcome {
    Success,
    Failure,
}

/// Caption the pinned branch writes for one outcome. Both literals
/// now come from the 7.55 sink pin, not from an invented caption.
#[must_use]
pub const fn uhp_start_caption(outcome: UhpStartOutcome) -> Option<&'static str> {
    match outcome {
        UhpStartOutcome::Success => Some(UHP_SUCCESS_CAPTION),
        UhpStartOutcome::Failure => Some(UHP_FAILURE_CAPTION),
    }
}

/// Opcode the UHP branch sends after its readiness poll.
#[must_use]
pub const fn uhp_report_opcode() -> u16 {
    UHP_REPORT_OPCODE
}

/// The three peer tables the start sequence walks. A `4045` answer
/// stages one public endpoint, one local endpoint and one member word
/// per peer; those are the only rows this client has. The 6-byte and
/// 4-byte strides are the exe's row widths, not a UDP socket claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UhpPeerTables {
    pub public_endpoints: Vec<SocketAddrV4>,
    pub local_endpoints: Vec<SocketAddrV4>,
    pub member_ids: Vec<i32>,
}

impl UhpPeerTables {
    /// Stage the three tables from one decoded peer list.
    #[must_use]
    pub fn from_peer_list(list: &P2PListResponse) -> Self {
        Self {
            public_endpoints: list.peers.iter().map(|peer| peer.public_endpoint).collect(),
            local_endpoints: list.peers.iter().map(|peer| peer.local_endpoint).collect(),
            member_ids: list.peers.iter().map(|peer| peer.member_id).collect(),
        }
    }

    /// Walk each staged table once, in table order. No socket is
    /// opened and no helper thread is started here.
    #[must_use]
    pub fn walk(&self) -> Vec<UhpPeerVisit> {
        let mut visits = Vec::with_capacity(
            self.public_endpoints.len() + self.local_endpoints.len() + self.member_ids.len(),
        );
        visits.extend(self.endpoint_visits(UhpPeerTable::PublicEndpoint, &self.public_endpoints));
        visits.extend(self.endpoint_visits(UhpPeerTable::LocalEndpoint, &self.local_endpoints));
        visits.extend(
            self.member_ids
                .iter()
                .enumerate()
                .map(|(index, member_id)| UhpPeerVisit {
                    table: UhpPeerTable::MemberId,
                    index,
                    entry: UhpPeerEntry::MemberId(*member_id),
                }),
        );
        visits
    }

    fn endpoint_visits(
        &self,
        table: UhpPeerTable,
        endpoints: &[SocketAddrV4],
    ) -> Vec<UhpPeerVisit> {
        endpoints
            .iter()
            .enumerate()
            .map(|(index, endpoint)| UhpPeerVisit {
                table,
                index,
                entry: UhpPeerEntry::Endpoint(*endpoint),
            })
            .collect()
    }
}

/// Which of the three staged tables owns one walk step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UhpPeerTable {
    PublicEndpoint,
    LocalEndpoint,
    MemberId,
}

/// One 6-byte endpoint row or one 4-byte member word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UhpPeerEntry {
    Endpoint(SocketAddrV4),
    MemberId(i32),
}

/// One recorded step of the peer walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UhpPeerVisit {
    pub table: UhpPeerTable,
    pub index: usize,
    pub entry: UhpPeerEntry,
}

/// The runtime effects of UHP start. The shell owns the caption and
/// completion word; the session owns the single empty report.
pub trait UhpStartHost {
    fn post_uhp_caption(&mut self, caption: &'static str);
    fn send_uhp_report(&mut self, opcode: u16, body: &[u8]) -> bool;
    fn mark_uhp_complete(&mut self);
}

/// Everything observable from one scripted UHP start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UhpStartTrace {
    pub peer_visits: Vec<UhpPeerVisit>,
    pub polls: usize,
    pub waits: Vec<Duration>,
    pub outcome: UhpStartOutcome,
    pub caption: &'static str,
    pub report: Option<(u16, Vec<u8>)>,
    pub report_delivered: bool,
    pub completed: bool,
}

/// Run the UHP start order with an injected readiness script and an
/// injected wait callback. This is a seam, not a hole punch: it does
/// not send UDP, open a socket, or create a worker thread.
pub struct UhpStartSequence<Wait> {
    readiness: Vec<bool>,
    wait: Wait,
}

impl<Wait> UhpStartSequence<Wait>
where
    Wait: FnMut(Duration),
{
    #[must_use]
    pub fn new(readiness: Vec<bool>, wait: Wait) -> Self {
        Self { readiness, wait }
    }

    /// Walk peers, poll readiness, post one caption, send one empty
    /// report, then set the completion flag.
    pub fn run<Host: UhpStartHost>(
        &mut self,
        tables: &UhpPeerTables,
        host: &mut Host,
    ) -> UhpStartTrace {
        let peer_visits = tables.walk();
        let (outcome, polls, waits) = self.poll_readiness();
        let caption = uhp_start_caption(outcome).unwrap_or(UHP_FAILURE_CAPTION);
        host.post_uhp_caption(caption);
        let report = (UHP_REPORT_OPCODE, Vec::new());
        let delivered = host.send_uhp_report(report.0, &report.1);
        host.mark_uhp_complete();

        UhpStartTrace {
            peer_visits,
            polls,
            waits,
            outcome,
            caption,
            report: Some(report),
            report_delivered: delivered,
            completed: true,
        }
    }

    /// Each `false` step consumes one 300 ms wait before the next
    /// poll. Exhausting the script takes the failure branch.
    fn poll_readiness(&mut self) -> (UhpStartOutcome, usize, Vec<Duration>) {
        let mut waits = Vec::new();
        for (index, ready) in self.readiness.iter().copied().enumerate() {
            if ready {
                return (UhpStartOutcome::Success, index + 1, waits);
            }
            if index + 1 < self.readiness.len() {
                (self.wait)(UHP_POLL_INTERVAL);
                waits.push(UHP_POLL_INTERVAL);
            }
        }
        (UhpStartOutcome::Failure, self.readiness.len(), waits)
    }
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
    use std::net::Ipv4Addr;

    use super::*;

    /// One seam host that records the three UHP effects without a
    /// socket or a worker thread.
    #[derive(Default)]
    struct RecordingUhpHost {
        captions: Vec<&'static str>,
        reports: Vec<(u16, Vec<u8>)>,
        completions: usize,
    }

    impl UhpStartHost for RecordingUhpHost {
        fn post_uhp_caption(&mut self, caption: &'static str) {
            self.captions.push(caption);
        }

        fn send_uhp_report(&mut self, opcode: u16, body: &[u8]) -> bool {
            self.reports.push((opcode, body.to_vec()));
            true
        }

        fn mark_uhp_complete(&mut self) {
            self.completions += 1;
        }
    }

    /// Drive one scripted UHP run and check the host saw the same
    /// caption, report and completion the trace records.
    fn run_uhp_start(
        tables: &UhpPeerTables,
        readiness: Vec<bool>,
        waits: &mut Vec<Duration>,
    ) -> UhpStartTrace {
        let mut sequence = UhpStartSequence::new(readiness, |interval| waits.push(interval));
        let mut host = RecordingUhpHost::default();
        let trace = sequence.run(tables, &mut host);
        assert_eq!(host.captions, vec![trace.caption]);
        assert_eq!(
            host.reports,
            vec![trace.report.clone().expect("one UHP report")]
        );
        assert_eq!(host.completions, 1);
        assert!(trace.report_delivered);
        trace
    }

    /// Two peers staged in the three 4045-derived tables.
    fn staged_uhp_peer_tables() -> UhpPeerTables {
        UhpPeerTables {
            public_endpoints: vec![
                SocketAddrV4::new(Ipv4Addr::new(203, 0, 113, 5), 4000),
                SocketAddrV4::new(Ipv4Addr::new(203, 0, 113, 6), 4002),
            ],
            local_endpoints: vec![
                SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 5), 4001),
                SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 6), 4003),
            ],
            member_ids: vec![-2, 3],
        }
    }

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
    fn live_billing_url_reads_the_pinned_host_page_and_session_join() {
        // Live caller chain: main.rs::run_play -> connect::connect_and_login_with_transport
        // -> window hub 0x5a5190 (WM_APP+0x32) -> billing_window_arm
        // -> BillingWebHandle::rebuild -> BillingWebPage::url.
        // The expected literals below are the doc-pinned values, not
        // values read back from the source under test. Cash-bill's
        // publisher gate at 0x45e010 stays OPEN; the guard asserts only
        // the five hosts the docs pin.
        const PAYMENT_HOST: &str = "game1.o2jam.com";
        const GIFT_HOST: &str = "game1.o2jam.com";
        const CASH_BILL_HOSTS: [&str; 5] = [
            "o2jam.monawa.com",
            "o2jam.gameangel.com",
            "o2jam.gameon.nate.com",
            "o2jam.pdbox.co.kr",
            "o2jam.nopp.co.kr",
        ];

        let cases = [
            (
                "payment_input.aspx",
                Some(PAYMENT_HOST),
                BillingWebPage::Payment,
            ),
            (
                "gift_input.aspx",
                Some(GIFT_HOST),
                BillingWebPage::GiftInput,
            ),
            (
                "cash_bill",
                None,
                BillingWebPage::CashBill(CashBillPublisher::Nopp),
            ),
        ];

        for (page, pinned_host, name) in cases {
            let mut handle = BillingWebHandle::with_handle(41);
            let steps = handle.rebuild(name, "7", 42);
            let url = steps
                .iter()
                .find_map(|step| match step {
                    BillingWebEvent::Mounted { url, .. } => Some(url.as_str()),
                    _ => None,
                })
                .unwrap();
            let host = url
                .strip_prefix("http://")
                .unwrap()
                .split('/')
                .next()
                .unwrap();

            assert!(
                url.contains(page),
                "{page}: live URL {url} must name the pinned page"
            );
            assert!(
                url.ends_with("?USER_INDEX_ID=7"),
                "{page}: live URL {url} must join session 7"
            );
            match pinned_host {
                Some(expected) => assert_eq!(
                    host, expected,
                    "{page}: live URL host expected {expected}, got {host}"
                ),
                None => assert!(
                    CASH_BILL_HOSTS.contains(&host),
                    "{page}: live URL host {host} must be one of the five pinned hosts {CASH_BILL_HOSTS:?}"
                ),
            }
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
    fn playing_qa_arm_stages_the_pinned_rect_shift_on_screen_game() {
        // seam: the exe's `WM_APP+0x32` / `0x14` arm has no winit
        // sender. `ScreenGame::stage_native_web_ui_event` is the
        // extracted arm helper against the live game state.
        // CONFLICT: the pin names scene 2 while the story calls the
        // shift Playing; options.rs pins Playing as 11. The exe pin
        // wins here, and the scene-word contradiction stays OPEN.
        use crate::screens::{paint::LoadingStatus, ScreenGame};
        use crate::Screen;
        use o2jam_audio::NullBackend;
        use o2jam_net::play::SetRoomMusic;
        use std::collections::HashMap;

        let mut game = ScreenGame::new();
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
        for scene in [
            Screen::Planet,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
        ] {
            assert!(game.enter_screen(scene), "the walk reaches {scene:?}");
        }
        assert!(game.enter_screen(Screen::Playing), "the gate arms Playing");
        assert_eq!(game.screen(), Screen::Playing);

        // The expected value is read from the exe pin, not from the
        // production constant. See `docs/pins/input-msg.md` for
        // `0x45f480` / file `0x5f480` and the `0x45f190` opener.
        const EXE_QA_RECT_SHIFT_PX: i32 = 50;

        let rect = game
            .stage_native_web_ui_event(PLAYING_QA_UI_EVENT, EXE_SCENE_ROOM)
            .expect("0x14 opens the in-game Q&A sheet");
        assert_eq!(
            rect.qa_rect_shift, EXE_QA_RECT_SHIFT_PX,
            "exe shifts the Q&A rect by 50 in scene 2 (0x45f480 / file 0x5f480; callee VA 0x45f190; ev_286dc423cbdb57485c95d5c32cbc843bcf47a138f12fb237887289b873c54b98)"
        );
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
    fn coupon_select_table_pins_the_five_archived_values() {
        // seam: the five `cp_id` values are defined entirely by the
        // archived body contract (`docs/pins/payment.md`, hash
        // `a283a1d9…aaa67`); there is no live host to reach.
        let codes: Vec<_> = CouponKind::ALL.iter().map(|kind| kind.code()).collect();
        assert_eq!(codes, ["KHCJR", "TRGDB", "TRKWJ", "TRIYG", "VILIV"]);
    }

    #[test]
    fn coupon_number_key_uppercases_each_typed_character() {
        // seam: the key-up uppercasing is defined by the archived body
        // (hash `a283a1d9…aaa67`); the dead host has no live caller.
        let mut form = CouponForm::new();
        for character in "abcde-12345".chars() {
            form.push_char(character);
        }
        assert_eq!(form.number(), "ABCDE-12345");
    }

    #[test]
    fn coupon_number_accepts_the_eleven_character_pattern() {
        // seam: the two regexes are the whole archived form contract.
        assert_eq!(validate_coupon_number("ABCDE-12345"), Ok(()));
    }

    #[test]
    fn coupon_number_accepts_the_seventeen_character_pattern() {
        // seam: the two regexes are the whole archived form contract.
        assert_eq!(validate_coupon_number("ABCDE-12345-FGHIJ"), Ok(()));
    }

    #[test]
    fn coupon_number_rejects_a_value_without_a_hyphen() {
        // seam: the archived page's first check names this branch.
        assert_eq!(
            validate_coupon_number("ABCDE12345"),
            Err(CouponNumberError::MissingHyphen)
        );
    }

    #[test]
    fn coupon_number_rejects_an_eleven_character_value_with_bad_groups() {
        // seam: the archived page's regex names this branch.
        assert_eq!(
            validate_coupon_number("ABCD--12345"),
            Err(CouponNumberError::WrongGroups)
        );
    }

    #[test]
    fn coupon_number_rejects_a_length_outside_eleven_or_seventeen() {
        // seam: the archived page's second check names this branch.
        assert_eq!(
            validate_coupon_number("ABCDE-1234"),
            Err(CouponNumberError::WrongLength)
        );
    }

    #[test]
    fn coupon_number_stops_accepting_characters_at_the_archived_cap() {
        // seam: the archived text input's `maxlength=30` is the whole
        // bound; no page backend exists to accept more.
        let mut form = CouponForm::new();
        for _ in 0..COUPON_NUMBER_MAX_CHARS + 2 {
            form.push_char('A');
        }
        assert_eq!(form.number().chars().count(), COUPON_NUMBER_MAX_CHARS);
    }

    #[test]
    fn coupon_submit_reports_the_pinned_fields_to_the_local_notice_seam() {
        // seam: the dead host makes the local report-back the whole
        // submit behavior (archived hash `a283a1d9…aaa67`).
        let mut form = CouponForm::new();
        form.select(CouponKind::Trgdb);
        for character in "abcde-12345".chars() {
            form.push_char(character);
        }
        assert_eq!(
            form.submit(),
            CouponSubmit::Reported(CouponReportBack {
                kind: CouponKind::Trgdb,
                number: "ABCDE-12345".to_owned(),
            })
        );
    }

    #[test]
    fn coupon_submit_rejects_an_invalid_number_without_a_notice() {
        // seam: the archived page's validation stops before submit.
        let form = CouponForm::new();
        assert_eq!(
            form.submit(),
            CouponSubmit::Rejected(CouponNumberError::MissingHyphen)
        );
    }

    #[test]
    fn coupon_report_back_never_claims_the_dead_host_accepted_it() {
        // seam: the no-claim caption is the whole report-back contract.
        let report = CouponReportBack {
            kind: CouponKind::Viliv,
            number: "ABCDE-12345".to_owned(),
        };
        assert!(
            report.notice().contains("no acceptance claim"),
            "{}",
            report.notice()
        );
    }

    #[test]
    fn coupon_form_cites_the_archived_body_hash() {
        // seam: the hash pins the archived body that defines these
        // fields; no live caller can reach the dead host.
        assert_eq!(
            ARCHIVED_COUPON_BODY_SHA256,
            "a283a1d95fe293ab43a485ee925b9eedfbcbc7c157c63fc9fa3fc6368d8aaa67"
        );
    }

    #[test]
    fn uhp_start_reports_4046_with_both_pinned_captions() {
        // seam: the relay UHP branch is battle-only and has no Rust
        // caller yet; the two caption literals and opcode are the
        // whole static pin that can be staged here.
        assert_eq!(uhp_report_opcode(), 4046);
        assert_eq!(
            uhp_start_caption(UhpStartOutcome::Success),
            Some("UDP Hole Punching Success~!!")
        );
        assert_eq!(
            uhp_start_caption(UhpStartOutcome::Failure),
            Some("UHP Start 실패")
        );
    }

    #[test]
    fn uhp_start_walks_all_three_staged_peer_tables() {
        // seam: the exe's three peer arrays (6-byte endpoints and
        // 4-byte member words) are the whole peer walk; this client
        // stages them from 4045 and sends no UDP packet.
        let tables = staged_uhp_peer_tables();
        let mut waits = Vec::new();
        let trace = run_uhp_start(&tables, vec![true], &mut waits);
        assert_eq!(
            trace.peer_visits,
            vec![
                UhpPeerVisit {
                    table: UhpPeerTable::PublicEndpoint,
                    index: 0,
                    entry: UhpPeerEntry::Endpoint(SocketAddrV4::new(
                        Ipv4Addr::new(203, 0, 113, 5),
                        4000
                    )),
                },
                UhpPeerVisit {
                    table: UhpPeerTable::PublicEndpoint,
                    index: 1,
                    entry: UhpPeerEntry::Endpoint(SocketAddrV4::new(
                        Ipv4Addr::new(203, 0, 113, 6),
                        4002
                    )),
                },
                UhpPeerVisit {
                    table: UhpPeerTable::LocalEndpoint,
                    index: 0,
                    entry: UhpPeerEntry::Endpoint(SocketAddrV4::new(
                        Ipv4Addr::new(10, 0, 0, 5),
                        4001
                    )),
                },
                UhpPeerVisit {
                    table: UhpPeerTable::LocalEndpoint,
                    index: 1,
                    entry: UhpPeerEntry::Endpoint(SocketAddrV4::new(
                        Ipv4Addr::new(10, 0, 0, 6),
                        4003
                    )),
                },
                UhpPeerVisit {
                    table: UhpPeerTable::MemberId,
                    index: 0,
                    entry: UhpPeerEntry::MemberId(-2),
                },
                UhpPeerVisit {
                    table: UhpPeerTable::MemberId,
                    index: 1,
                    entry: UhpPeerEntry::MemberId(3),
                },
            ]
        );
        assert_eq!(UHP_ENDPOINT_STRIDE_BYTES, 6);
        assert_eq!(UHP_MEMBER_STRIDE_BYTES, 4);
    }

    #[test]
    fn uhp_start_polls_readiness_in_300_ms_steps_before_the_success_report() {
        // seam: the injected poll script replaces the exe's real
        // readiness poller and `Sleep(300)`; no thread exists in this
        // client. The report follows the poll, exactly once and empty.
        let mut waits = Vec::new();
        let trace = run_uhp_start(
            &staged_uhp_peer_tables(),
            vec![false, false, true],
            &mut waits,
        );
        assert_eq!(trace.polls, 3);
        assert_eq!(waits, vec![UHP_POLL_INTERVAL, UHP_POLL_INTERVAL]);
        assert_eq!(waits[0], Duration::from_millis(300));
        assert_eq!(trace.outcome, UhpStartOutcome::Success);
        assert_eq!(trace.caption, UHP_SUCCESS_CAPTION);
        assert_eq!(
            trace.report,
            Some((UHP_REPORT_OPCODE, Vec::new())),
            "one empty 4046 follows the readiness poll"
        );
        assert!(trace.completed);
    }

    #[test]
    fn uhp_start_posts_the_pinned_failure_caption_when_readiness_never_passes() {
        // seam: exhausted readiness is the branch whose caption was
        // unnamed before the 7.55 sink pin; the report still follows.
        let mut waits = Vec::new();
        let trace = run_uhp_start(&staged_uhp_peer_tables(), vec![false, false], &mut waits);
        assert_eq!(trace.outcome, UhpStartOutcome::Failure);
        assert_eq!(trace.caption, "UHP Start 실패");
        assert_eq!(trace.report, Some((4046, Vec::new())));
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

    #[test]
    fn result_event_selector_names_only_the_pinned_notice_pages() {
        // Live caller chain (Tier 0 logic): main.rs::run_play ->
        // connect::connect_and_login_with_transport ->
        // ScreenGame::enter_screen -> result_event_page. The pure
        // selector branch is called at Result entry, so this guard
        // pins the arm's name and its explicit OPEN fields.
        assert_eq!(result_event_page(0, 0), Some(NativeEventPage::HotTime));
        assert_eq!(result_event_page(0, 1), Some(NativeEventPage::TicketAdd));
        assert_eq!(result_event_page(2, 1), None, "battle skips both arms");
        assert_eq!(
            NativeEventPage::Bingo.pinned_url(),
            "http://o2jam.nopp.co.kr/client/event/2009/01/01_bingo/index.asp?USER_INDEX_ID=<id>"
        );
        assert_eq!(
            NativeEventPage::InGameQa.pinned_url(),
            "http://o2jam.nopp.co.kr/client/mtm/ingame_1n1qna01.phtml?USER_INDEX_ID=<id>"
        );
        assert_eq!(
            NativeEventPage::HotTime.pinned_url(),
            "http://o2jam.nopp.co.kr/client/event/2009/06_moa/hot_time.php"
        );
        assert!(
            NativeEventPage::HotTime
                .open_note()
                .contains("separator/key OPEN"),
            "the hot-time query must not be invented"
        );
        assert!(
            NativeEventPage::TicketAdd
                .open_note()
                .contains("title source OPEN"),
            "the armed title must not be invented"
        );
        assert!(NativeEventPage::InGameQa
            .open_note()
            .contains("questions and outbound links OPEN"));
    }

    #[test]
    fn room_friend_host_constructor_installs_the_pinned_vftable_and_flags() {
        // seam: no production path opens the Room user-list sheet
        // (`open_room_user_sheet` has test callers only) and no
        // production code reads `room_friend_host()`; both facts are
        // why a seam suffices for the host consumer. This guard stages
        // only the constructor words. The pin defines these words and
        // nothing about page content, which stays OPEN (#405). Expected
        // values are independent literals, not the constants feeding
        // the code.
        let host = NativeRoomFriendHost::mount();
        assert_eq!(host.vftable(), "CWebFriendCtrl::vftable");
        assert_eq!(host.trailing_flags(), [1, 0, 0]);
        assert_eq!(
            host.page_boundary_open(),
            "OPEN: no gamefine_main.asp render or wire capture (#405)"
        );
    }
}
