//! [`ScreenGame`]: the window's game loop, owning the [`ScreenFlow`]
//! and walking screens on input.

mod lanking;
mod musicshop;
mod planet;
mod shop;
mod waiting;

use std::collections::HashMap;

use self::lanking::LankingState;
use self::musicshop::MusicShopState;
use self::planet::PlanetBoardState;
use self::shop::ShopState;
use self::waiting::PendingOutbox;
use super::super::avatar::{
    avatar_item_by_id, load_worn_figure, ojn_cover_figure, AvatarFigure, ScenePlacement,
    LOADING_COVER_SPRITE,
};
use super::super::options::OptionsSettings;
use super::super::playing::{HighwayGeometry, LivePlay, PlaySkin};
use super::super::screen::{Screen, ScreenFlow};
use super::super::GameLoop;
use super::paint::{
    battle_verdict, channel_dialog_x_hit, channel_face_hit, channel_row_hit, channel_scroll_hit,
    channel_viewport, channel_viewport_from_top, dialog_origin, dialog_rect_hit, has_room_panels,
    list_row_hit, paint_avi_scene, paint_channel_chat, paint_channel_list, paint_channel_scene,
    paint_dialog_scene, paint_lanking_board, paint_lanking_scene, paint_loading_scene,
    paint_loading_scene_with_status, paint_lobby_backdrop, paint_music_shop_board,
    paint_music_shop_cart, paint_music_shop_scene, paint_myroom_data, paint_myroom_scene,
    paint_payment_notice, paint_planet_options, paint_planet_quit, paint_planet_scene,
    paint_playing_backdrop, paint_playing_foreground, paint_result, paint_result_scene,
    paint_room_chat, paint_room_hover, paint_room_list, paint_room_scene, paint_room_user_panel,
    paint_screen, paint_shop_board, paint_shop_scene, paint_waiting_chat, paint_waiting_members,
    paint_waiting_scene, paint_waiting_seat_avatar, paint_waiting_seat_avatars,
    playing_backdrop_staged, quit_fallback_rects, rect_hit, room_panel_hit, scene_index,
    staged_rect_hit, BattleVerdict, ChannelChatMessage, ChannelScroll, LoadingStatus, ResultPaint,
    ResultRankRow, ResultVerdict, SceneArt, WaitingCaptions, WaitingChatMessage,
    CHANNEL_VISIBLE_ROWS, RESULT_BACK_ID, RESULT_BACK_RECT, RESULT_RETRY_ID, RESULT_RETRY_RECT,
};
use super::select_paint::{
    paint_select_chosung_rows, paint_select_filter_columns, paint_select_ok,
    paint_select_result_tabs, paint_select_search_row, paint_select_status_row, paint_select_table,
};
use super::side::{
    can_return_side, can_visit_side, create_room_action_at, lanking_action_at,
    music_shop_action_at, myroom_action_at, myroom_bag_pages, myroom_bag_rows, myroom_equip_slot,
    myroom_gift_captions, myroom_gift_page, myroom_gift_pages, myroom_gift_views,
    myroom_identity_captions, myroom_record_slot, paint_side_scene, password_action_at,
    payment_side_notice, planet_action_at, select_music_rows, select_music_songs, select_row_hit,
    shop_action_at, side_scene_slot, waiting_song_detail_line, waiting_song_title_line,
    waiting_team_for_code, wallet_row_caption, CreateRoomAction, CreateRoomForm, LankingSong,
    LankingSort, MusicShopRow, MyRoomAction, MyRoomState, PasswordAction, PasswordForm,
    PlanetAction, PlanetOptionTab, PlanetPromo, RoomDialog, SelectMusicSong, ShopGood, SideScreen,
    WalletRow, WalletState, GEM_LABEL, OPTION_GAME_BLEND_NO, OPTION_GAME_BLEND_YES,
    OPTION_GAME_EFFECT_HIGH, OPTION_GAME_EFFECT_LOW, OPTION_GAME_EFFECT_NORMAL,
    OPTION_GAME_GUIDE_LONG, OPTION_GAME_GUIDE_MIDDLE, OPTION_GAME_GUIDE_NOUSE,
    OPTION_GAME_GUIDE_SHORT, OPTION_GAME_LONGNOTE_NONE, OPTION_GAME_LONGNOTE_USED,
    OPTION_GAME_MODE_FULL, OPTION_GAME_MODE_WIN, OPTION_GAME_NOTETYPE1, OPTION_GAME_NOTETYPE2,
    OPTION_KEY_3KEY_FIRST, OPTION_KEY_3KEY_MODE, OPTION_KEY_7KEY_MODE, OPTION_KEY_LANE_FIRST,
    OPTION_KEY_TEST, OPTION_SOUND_BG_EFFECT_DOWN, OPTION_SOUND_BG_EFFECT_UP,
    OPTION_SOUND_BG_MUSIC_DOWN, OPTION_SOUND_BG_MUSIC_UP, OPTION_SOUND_GAME_EFFECT_DOWN,
    OPTION_SOUND_GAME_EFFECT_UP, OPTION_SOUND_GAME_MUSIC_DOWN, OPTION_SOUND_GAME_MUSIC_UP,
    PLANET_OPTION_FIRST_TAB, PLANET_PROMO_IMAGE_CODE, QUIT_CANCEL_CODE, QUIT_OK_CODE,
};
use crate::{menu_of, InputEdge, InputSnapshot, MenuKey};
use o2jam_audio::{
    song_position_ms, AudioBackend, BgmSelection, CueOutcome, PcmSamples, SceneBgm, SceneCue,
};
use o2jam_file::{AvatarItemTable, OjnListEntry};
use o2jam_game::{PeerBars, QueueEvent, RankDelivery, RoomTeams, PEER_SLOT_COUNT};
use o2jam_net::gift::{ClaimGift, GetGiftMessageList, GiftType};
use o2jam_net::play::{MusicListAnswer, SetRoomMusic};
use o2jam_net::rooms::{
    Announcement, MainRoomAdminMessage, MainRoomUserMessage, MusicPlayRankingAnswer, RankEntry,
    RankNotification, ReceiveWhisper, SendMainRoomMessage, SendWaitingMessage, SendWhisper,
    WaitingAdminMessage, WaitingUserMessage,
};
use o2jam_net::shop::{EquipItem, SyncItemPurchase, SyncMusicPurchase};
use o2jam_net::wallet::SyncCashPointResponse;
use o2jam_net::{
    ChannelState, CharacterInfo, CreateRoom, Difficulty, ExitPlaying, Gender, JoinRoom, JoinedRoom,
    Ready, RoomListEntry, RoomTeamChanged, SetRoomTeam, StartGame,
};
use o2jam_render::{
    draw_text, fill_rect, pack_xrgb, scale_rect_to_frame, text_width, Frame, Rect, Rgb555Image,
};
use winit::keyboard::KeyCode;

/// Loading-gate phase capacity (lifecycle section 9, steps 2-5):
/// staging `0x4f8b80` (ev `d2a69b45...`), OJN open/decrypt `0x412180`
/// (ev `011eab73...`), codec init `0x54db40` plus sample decode init
/// `0x54d460` (ev `0ee1373d...` / `3f154198...`), and the loading
/// screen `0x4e47a0` (ev `68d920c2...`) arming the song clock. Phase
/// 0 is gate entry (select validation `0x5718b0`, ev `66e60d16...`);
/// reaching [`LOADING_GATE_PHASES`] is the armed fraction the shot
/// walk holds. Decrypt and Vorbis init precede the arm: the gate
/// never reaches the armed phase before both.
pub const LOADING_GATE_PHASES: usize = 4;

/// Stage the Playing backdrop defaults to before a room names one:
/// the exe's room entry replaces a zero stage with a real one (the
/// waiting-room picker's value, store `0x4aacea`, `docs/stage-bga.md`),
/// so the composer never carries stage 0; the first stage stands in
/// for the picker's pick (exact default untraced).
const DEFAULT_ARENA_STAGE: u16 = 1;

/// Live waiting-room chat rows kept per room (oldest first): enough for
/// the 12-row history box plus scrollback, bounded so a loud room
/// cannot grow memory without limit (PERF-07: `usize`, never `as`).
const MAX_WAITING_CHAT_ROWS: usize = 50;

/// Live rows kept per lobby chat panel (oldest first): the
/// waiting-box bound, reused so a loud channel or room cannot grow
/// memory without limit (PERF-07: `usize`, never `as`).
const MAX_MAIN_ROOM_CHAT_ROWS: usize = 50;

/// One lobby chat panel that carries the main-room feed: the unsent
/// draft line plus the live history, oldest first. The Channel scene
/// and the Room lobby each own one (the room's chat IS the main-room
/// chat, ticket #37: `SendMainRoomMessage` 2012 out, 2013/2014 in), so
/// the row folding, the draft cap, and the history bound live here
/// once.
#[derive(Debug, Clone, Default)]
pub struct LobbyChatPanel {
    draft: String,
    history: Vec<ChannelChatMessage>,
}

impl LobbyChatPanel {
    /// Unsent draft line (the 2012 content source).
    #[must_use]
    pub fn draft(&self) -> &str {
        &self.draft
    }

    /// Live history rows, oldest first (empty until a packet arrives).
    #[must_use]
    pub fn history(&self) -> &[ChannelChatMessage] {
        &self.history
    }

    /// Type one character into the draft: control characters never
    /// enter the line and the draft stops at [`MAX_CHAT_INPUT_CHARS`]
    /// characters.
    fn type_char(&mut self, typed: char) {
        if typed.is_control() || self.draft.chars().count() >= MAX_CHAT_INPUT_CHARS {
            return;
        }
        self.draft.push(typed);
    }

    /// Erase the last draft character (Backspace; one whole character,
    /// never half of a multibyte one).
    fn erase_char(&mut self) {
        self.draft.pop();
    }

    /// Stage the draft line, truncated to [`MAX_CHAT_INPUT_CHARS`].
    fn set_draft(&mut self, line: &str) {
        self.draft = line.chars().take(MAX_CHAT_INPUT_CHARS).collect();
    }

    /// Forget the unsent draft.
    fn clear_draft(&mut self) {
        self.draft.clear();
    }

    /// Append one row and bound the history to
    /// [`MAX_MAIN_ROOM_CHAT_ROWS`] (oldest rows drop first).
    fn push_row(&mut self, row: ChannelChatMessage) {
        self.history.push(row);
        let overflow = self.history.len().saturating_sub(MAX_MAIN_ROOM_CHAT_ROWS);
        self.history.drain(..overflow);
    }

    /// Outgoing 2012 body for the draft (`None` while the draft is
    /// empty, so no empty send is invented).
    #[must_use]
    fn send_message(&self) -> Option<SendMainRoomMessage> {
        if self.draft.is_empty() {
            return None;
        }
        Some(SendMainRoomMessage {
            content: self.draft.as_bytes().to_vec(),
        })
    }

    /// Outgoing 2017 whisper body for the draft (`None` while the draft
    /// or the recipient is empty, so no empty send is invented).
    #[must_use]
    fn whisper_message(&self, recipient: &str) -> Option<SendWhisper> {
        if self.draft.is_empty() || recipient.is_empty() {
            return None;
        }
        Some(SendWhisper {
            recipient: recipient.to_owned(),
            content: self.draft.clone(),
        })
    }
}

use RoomControlAction::{Back, Create, Options, Side};

/// How the room lobby answers one staged `IDC_BUTTON_SROOM_*` control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoomControlAction {
    /// Leave for the channel scene (the Back edge).
    Back,
    /// Open the create-room dialog.
    Create,
    /// Open the options dialog over the lobby.
    Options,
    /// Visit the side screen the control already names.
    Side(SideScreen),
    /// No evidenced target yet: the router records the press and the
    /// evidence step that would wire it. Never a silent drop.
    Unwired {
        /// Next evidence step named by the click log.
        next_step: &'static str,
    },
}

/// One staged room chrome control: the bnd code, the `ControlList` id,
/// and the behaviour the tree evidences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoomControl {
    code: u32,
    id: &'static str,
    action: RoomControlAction,
}

/// Record of the last room chrome control that had no evidenced target
/// ([`RoomControlAction::Unwired`]): a test affordance that proves the
/// click was answered instead of dropped (tickets #32/#34).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomControlNoop {
    /// Staged control id (`IDC_BUTTON_SROOM_*`).
    pub id: &'static str,
    /// Next evidence step that would wire the control.
    pub next_step: &'static str,
}

/// Next evidence step for a staged room control whose click handler the
/// tree has not traced yet: the exe `CStateRoom` click dispatch.
const HANDLER_STEP: &str =
    "rig click on the control in the live room plus a memory read of the CStateRoom click dispatch (docs/parity/room.md Input row)";
/// Next evidence step for `IDC_BUTTON_SROOM_FIRSTSTEP` (ticket #34):
/// `STATE_ROOM` carries no tutorial control, so the tutorial emblem's
/// entry path owns the question.
const TUTORIAL_STEP: &str =
    "pin the tutorial entry path (docs/parity/tutorial.md TUTORIAL_ENTRY_NEXT_STEP); STATE_ROOM itself carries no tutorial control";
/// Next evidence step for the three mode filters.
const MODE_FILTER_STEP: &str =
    "rig click on each mode filter plus a capture of the filtered room list (the list filter rule is untraced)";
/// Next evidence step for the four chat-tab buttons.
const CHAT_TAB_STEP: &str =
    "rig click on each chat tab plus a capture of the filtered chat panel (the tab filters are untraced)";
/// Next evidence step when a room dialog sheet never staged (no
/// install-root data): the modal must not own input it cannot
/// paint, so the click records a noop and the lobby stays usable.
const DIALOG_STAGE_STEP: &str =
    "stage Dialog_CreateRoom plus Dialog_Password art in the live boot path (live_art.rs stage_room_dialogs); without the sheet the modal stays shut";

/// Every staged `IDC_BUTTON_SROOM_*` control of `STATE_ROOM`, in the
/// order the router tests them, with the behaviour this tree evidences
/// (tickets #32/#34).
///
/// A control that already has a target keeps it (`BACK`, `CREATE`,
/// `OPTION`, the four side visits cited below); every other staged
/// button carries its named next evidence step. The codes pair with the
/// `.bnd` rects by control ordinal (owner rule `f38f3`), so each arm is
/// a rect lookup plus a dispatch — no geometry is invented here.
///
/// Not listed: the row templates (`IDC_BUTTON_SROOM_EXBUTTON1..6`, the
/// `SET` clone parents) whose rects ARE the room rows and therefore
/// fall through to the row hit test, the stage images (`0x135…`), and
/// the scroll widgets (`0x13200017..1A`), whose behaviour belongs to the
/// scrollbar ticket.
#[rustfmt::skip]
const ROOM_CONTROLS: &[RoomControl] = &[
    routed(0x1320_0002, "IDC_BUTTON_SROOM_BACK", Back),
    routed(0x1320_0007, "IDC_BUTTON_SROOM_CREATE", Create),
    routed(0x1320_000B, "IDC_BUTTON_SROOM_OPTION", Options),
    // The four controls whose target already ships in the shell.
    routed(0x1320_0008, "IDC_BUTTON_SROOM_ITEMSHOP", Side(SideScreen::Shop)),
    routed(0x1320_0009, "IDC_BUTTON_SROOM_MYROOM", Side(SideScreen::MyRoom)),
    routed(0x1320_0037, "IDC_BUTTON_SROOM_MUSICSHOP", Side(SideScreen::MusicShop)),
    // `MYBAND` opens the ranking board: the band standings board the tree
    // ships is `SideScreen::Lanking` (the room has no band view).
    routed(0x1320_003A, "IDC_BUTTON_SROOM_MYBAND", Side(SideScreen::Lanking)),
    // No evidenced target yet: the press is recorded with its next step.
    unwired(0x1320_0006, "IDC_BUTTON_SROOM_QUICKJOIN", HANDLER_STEP),
    unwired(0x1320_000A, "IDC_BUTTON_SROOM_COUPON", HANDLER_STEP),
    unwired(0x1320_0032, "IDC_BUTTON_SROOM_FIRSTSTEP", TUTORIAL_STEP),
    unwired(0x1320_0038, "IDC_BUTTON_SROOM_MYCAPSULE", HANDLER_STEP),
    unwired(0x1320_0039, "IDC_BUTTON_SROOM_EVENT", HANDLER_STEP),
    unwired(0x1320_003D, "IDC_BUTTON_SROOM_CONFERENCE", HANDLER_STEP),
    unwired(0x1320_0048, "IDC_BUTTON_SROOM_MESSENGER", HANDLER_STEP),
    // The block spells this code `0x1320C04B` (the `0xC0` nibble rides in
    // the low word), unlike its neighbours.
    unwired(0x1320_C04B, "IDC_BUTTON_SROOM_WATINGVIEW", HANDLER_STEP),
    unwired(0x1320_000C, "IDC_BUTTON_SROOM_LEFT", HANDLER_STEP),
    unwired(0x1320_000D, "IDC_BUTTON_SROOM_RIGHT", HANDLER_STEP),
    unwired(0x1320_1003, "IDC_BUTTON_SROOM_SHOW_ALLMODE", MODE_FILTER_STEP),
    unwired(0x1320_1004, "IDC_BUTTON_SROOM_SHOW_NORMALMODE", MODE_FILTER_STEP),
    unwired(0x1320_1005, "IDC_BUTTON_SROOM_SHOW_BATTLEMODE", MODE_FILTER_STEP),
    unwired(0x1320_100E, "IDC_BUTTON_SROOM_CHAT_ALL", CHAT_TAB_STEP),
    unwired(0x1320_100F, "IDC_BUTTON_SROOM_CHAT_FRIEND", CHAT_TAB_STEP),
    unwired(0x1320_1010, "IDC_BUTTON_SROOM_CHAT_GUILD", CHAT_TAB_STEP),
    unwired(0x1320_1011, "IDC_BUTTON_SROOM_CHAT_WHISPER", CHAT_TAB_STEP),
];

/// One staged control with an evidenced behaviour.
const fn routed(code: u32, id: &'static str, action: RoomControlAction) -> RoomControl {
    RoomControl { code, id, action }
}

/// One staged control with no evidenced behaviour yet.
const fn unwired(code: u32, id: &'static str, next_step: &'static str) -> RoomControl {
    routed(code, id, RoomControlAction::Unwired { next_step })
}

/// True while the staged rect of `code` covers native pixel (`x`, `y`).
fn staged_code_hit(placements: &[ScenePlacement], code: u32, x: i32, y: i32) -> bool {
    placements
        .iter()
        .any(|placed| placed.code == code && rect_hit(&placed.rect, x, y))
}

/// Room chrome already showing its active frame: the default-active
/// mode filter (`SHOW_ALLMODE`) and chat tab (`CHAT_ALL`) paint body
/// frame 1 and never change on hover (EXE-pinned, `docs/hover.md`).
/// Hover over them resolves to nothing, so the painter never repaints
/// frame 1 with frame 0.
const ROOM_ACTIVE_CHROME_CODES: [u32; 2] = [0x1320_1003, 0x1320_100E];

/// Which lobby chat panel a call addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyChat {
    /// Channel scene panel.
    Channel,
    /// Room lobby panel (staged `IDC_LIST_SROOM_CHAT`).
    Room,
}

impl LobbyChat {
    /// Panel the on-screen scene owns (`None`: the scene shows no lobby
    /// chat panel).
    #[must_use]
    pub fn of_screen(screen: Screen) -> Option<Self> {
        match screen {
            Screen::Channel => Some(Self::Channel),
            Screen::Room => Some(Self::Room),
            _ => None,
        }
    }
}

/// Unsent chat draft cap (characters, not bytes: multibyte input
/// counts once per character so no slice ever splits UTF-8).
const MAX_CHAT_INPUT_CHARS: usize = 50;

/// Planet-board modal dialog open over the bare board (the power
/// quit-confirm or the gear options dialog). The board stays the
/// current screen; the modal owns clicks until closed, like the
/// channel dialog owns rows while `Screen::Channel` shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetModal {
    /// Quit-confirm (`게임을 종료하시겠습니까?` + OK/CANCEL): OK
    /// quits the client, CANCEL returns to the bare board.
    QuitConfirm,
    /// Options dialog (`Dialog_Option_*` sheet): the tab's EXIT
    /// button or Escape closes it; tab switching stays OPEN.
    Options,
}

/// Native click canvas width (the shell maps window pixels into
/// 800x600 and freezes them into the per-frame [`InputSnapshot`](crate::InputSnapshot)
/// the game sweeps).
const CLICK_W: i32 = 800;
/// Native click canvas height (see [`CLICK_W`]).
const CLICK_H: i32 = 600;

/// Frame clear colour: device black, the exe backbuffer state.
///
/// Pinned by the 7.55 planet capture
/// (`/home/arnim/scratch/planet-shot.png`): every pixel the staged
/// `STATE_PLANET` art leaves uncovered — 30,725 px, 6.40 % of the
/// canvas — is pure black there (mean RGB `0,0,0`, 100 % of the hole
/// mask). The exe therefore starts each frame on black; scenes still
/// contribute no fill of their own (`docs/channel-fill.md`).
const FRAME_CLEAR_COLOR: u32 = 0;

/// Waiting confirm-button candidates under the `IDC_BUTTON_SWAIT_*`
/// convention (the `START 0x16202012` / `READY 0x16202013` arms,
/// `docs/room-enums.md`): the exact staged names stay untraced, so
/// clicks only fire while staged art binds one of these ids.
const WAITING_CONFIRM_IDS: [&str; 2] = ["IDC_BUTTON_SWAIT_START", "IDC_BUTTON_SWAIT_READY"];
/// Waiting back-button candidates (same convention caveat as
/// [`WAITING_CONFIRM_IDS`]).
const WAITING_BACK_IDS: [&str; 2] = ["IDC_BUTTON_SWAIT_BACK", "IDC_BUTTON_SWAIT_EXIT"];

/// Splash auto-advance delay in milliseconds: the exe leaves
/// `STATE_AVI` for the bare planet board 5 s after the logo mounts
/// with no input. Measured twice on the isolated null-sink Wine rig
/// (display `:97`, copied prefix + data, 1 Hz frame poll so ±1 s):
/// `~/scratch/wt24-rig/hold1/timeline.tsv` stamps splash-up 62.8 s
/// and board 67.8 s (5.0 s), `hold2` stamps 63.3 s → 68.6 s
/// (5.3 s) — both rounds to a 5 s hold at the poll's ±1 s honesty;
/// both `gone` frames show the bare planet board, and an Enter-key
/// run (`key-enter/`) reaches the same board on the same ~5 s shape
/// (Enter does not fast-dismiss, so the edge is the timer, not
/// the key).
///
/// The Avi cue is record 8, `bgLogo.ogg`, 3.923 s — so the logo
/// track ends ~1 s before the advance, which is the ticket's
/// expected shape (hold past the BGM end, then advance). Ticket #24
/// replaces the old 3 s stand-in (which cut the logo track 0.92 s
/// short) with this measured value.
const SPLASH_AUTO_ADVANCE_MS: u64 = 5_000;
/// Splash hold poll gap while windowed (ticket #138): the Avi
/// auto-advance rides the wall clock (`advance_splash_on_wall_clock`
/// polls it per paint), so a modest wakeup checks the hold instead of
/// the loop's 60 Hz tick — the advance lands within this of the hold.
const SPLASH_HOLD_POLL_MS: u64 = 100;
/// Planet promo heartbeat while windowed (ticket #167): the banner
/// rotates on the exe's 3000 ms cadence (`PLANET_PROMO_TICK_MS`,
/// pinned `0xBB8` push) while the rotation flag holds, so the bare
/// board owes the loop one modest wakeup per second — the advance
/// lands within one pulse of the tick. Stopped, covered, or
/// frameless boards paint nothing new and stay fully idle (the
/// #138 parked-loop result holds).
const PLANET_PROMO_HEARTBEAT_MS: u64 = 1_000;

/// Game loop that shows each screen's scene and walks the flow.
///
/// Input wiring so far: any lane key or canvas click dismisses the
/// splash onto the bare planet board, and the splash auto-advances
/// there on its measured 5 s hold (ticket #24, guard:
/// `splash_auto_advances_at_timeout`). Windowed runs ride the wall
/// clock for the hold (ticket #54, claims B38); the fixed-clock
/// path stays the headless seam. The dismiss arms stay
/// stand-ins: the exe set is only partly pinned (Escape dismisses
/// within ~0.3 s, `~/scratch/avi-hold/run3-escape/`; Enter and Space
/// show no fast dismiss on the isolated rig — `~/scratch/wt24-rig/`
/// `key-enter/` and `key-space/` both follow the ~5 s timer shape
/// instead — yet Space is a lane bind, so the any-lane-key mapping
/// is unpinned by counterexample). OPEN: the full `VK` sweep plus
/// the exe lane-key mapping (guards: `lane_key_dismisses_splash_into_planet_board`,
/// `click_dismisses_splash_into_planet_board`,
/// `splash_click_edge_lands_the_same_bare_board`,
/// `escape_dismisses_splash_into_planet_board`,
/// `splash_menu_keys_keep_splash`).
/// Room/waiting/loading inputs land with their UIs.
pub struct ScreenGame {
    flow: ScreenFlow,
    playing: LivePlay,
    /// Song-clock anchor: backend samples already played when
    /// Playing was entered (`AudioBackend::samples_played` clock).
    /// Re-based on every Playing entry; never the wall clock.
    song_anchor_sample: u64,
    /// Device samples per second for the song-clock conversion
    /// ([`song_position_ms`]); 44100 default — the shipped chart
    /// audio rate (VERIFIED `Music/BGM.ojm` decode, 44100 Hz, and
    /// the null sink counts at the staged rate). Playing entries
    /// auto-sync it from the backend's `sample_rate` (live devices
    /// resample to their own rate); [`Self::set_song_rate`]
    /// overrides for tests.
    song_rate: u32,
    /// Fixed-clock test affordance: when set, [`Self::now_ms`] serves
    /// this virtual clock (advanced by `fixed_clock_step_ms` per
    /// `paint`) instead of the audio sample clock — trace replays
    /// through the real window stay deterministic without an audio
    /// device. `None` in production runs (the sample clock rules).
    fixed_clock_step_ms: Option<u64>,
    /// Virtual clock position while the fixed clock is enabled
    /// (paints × step; menus and Playing share it).
    fixed_now_ms: u64,
    /// Splash hold in ms before the auto-advance (default
    /// [`SPLASH_AUTO_ADVANCE_MS`]): tests set 0 for an instant
    /// advance; production never changes it.
    splash_hold_ms: u64,
    /// Wall-clock instant the splash mounted (recorded on every Avi
    /// entry, including boot): the windowed auto-advance rides the
    /// wall clock (claims B38), never the song clock — a track that
    /// stops at its end would otherwise freeze the splash.
    avi_entered_wall: Option<std::time::Instant>,
    audio: Option<AudioRig>,
    avatar: Option<AvatarFigure>,
    channels: Vec<ChannelState>,
    highlight: usize,
    /// Channel list-top offset (the scrolled origin): the
    /// `CDChannelInfo+0x184` analog. The scroll arrows step this
    /// offset (`0x50ebf0`→`0x50ec20` mutates the scrollbar's own
    /// position and forwards the delta to the list, handlers
    /// `0x4749a0`/`0x4749f0`); the highlight/selection is never
    /// touched on that path (ticket #130). Row clicks map
    /// slot-through-top (`code + top`), key steps re-seat the top
    /// under the highlight (exe key rule stays OPEN).
    channel_top: usize,
    /// Native pointer position from the last snapshot (`None` until
    /// the pointer moves): the hover source for scenes with pinned hover
    /// art (the channel select bar). Test affordance and live input both
    /// feed it through [`GameLoop::ingest_snapshot`].
    pointer: Option<(i32, i32)>,
    /// Channel-scene lobby chat panel (main-room 2013/2014/2016/2019/
    /// 2029 rows plus the 2012 draft).
    channel_chat: LobbyChatPanel,
    /// Room-lobby chat panel: the same main-room feed, painted inside
    /// the staged `IDC_LIST_SROOM_CHAT`/`IDC_EDIT_SROOM_CHAT` rects.
    room_chat: LobbyChatPanel,
    /// Last staged room chrome control the router answered without an
    /// evidenced target (see [`RoomControlNoop`]); `None` until such a
    /// press happens.
    last_room_control_noop: Option<RoomControlNoop>,
    /// Last whisper peer behind `/r` (the most recent incoming 2019
    /// sender or outgoing 2017 recipient; `None` until a whisper
    /// flows either way, so a reply with no peer falls back to chat).
    last_whisper_peer: Option<String>,
    rooms: Vec<RoomListEntry>,
    room_highlight: usize,
    waiting: Option<JoinedRoom>,
    /// Live waiting-room chat rows (3012/3013 packets only, oldest first).
    waiting_chat: Vec<WaitingChatMessage>,
    /// Unsent chat draft for the input line (3011 source).
    chat_input: String,
    selected_music: Option<SetRoomMusic>,
    /// Live loading status (walk fraction + title–artist, lifecycle
    /// §9): `None` paints the indeterminate full bar.
    loading_status: Option<LoadingStatus>,
    /// Loading-gate phase held by the walk (0 entry ..= 4 armed;
    /// [`LOADING_GATE_PHASES`]): advances only through staged
    /// [`ScreenGame::set_loading_status`] fractions, never on Confirm
    /// alone, so an unstaged gate stays at entry and holds Loading.
    loading_gate_phase: usize,
    /// A failed loading leg (bad chart, undecryptable OJN, missing
    /// audio): latched until [`ScreenGame::clear_loading`], holds
    /// Loading against any later fraction.
    loading_gate_failed: bool,
    /// Room arena stage the Playing `_` sprites resolve through
    /// (`Playing_BG<stage>.ojs` and the staged kits; see
    /// [`Self::arena_stage`]).
    arena_stage: u16,
    /// Planet-board state (the difficulty pick, promo carousel,
    /// press memory, and modal; #139 cluster 5): every planet/promo
    /// word lives here; the paint overlay and the hash fold read it
    /// through the same-named delegates and field paths.
    planet_board: PlanetBoardState,
    /// Quit-confirm OK was pressed: the windowed shell exits the
    /// event loop ([`GameLoop::quit_requested`]); headless walks
    /// read the flag instead.
    quit_requested: bool,
    /// Staged options-dialog sheets, one per `Dialog_Option_*` tab
    /// (`None` until staged; the fallback title paints instead).
    /// Tab order follows [`PlanetOptionTab`] declaration order
    /// (Key, Sound, Game) — see [`Self::option_tab_slot`].
    planet_option_art: [Option<SceneArt>; 3],
    /// Open options tab (first-open stand-in, OPEN — see
    /// [`PLANET_OPTION_FIRST_TAB`]).
    planet_option_tab: PlanetOptionTab,
    /// Options-dialog values (`option.ini` beside the install root;
    /// loaded at boot, written back on the tab SAVE commit).
    options: OptionsSettings,
    /// Install root the options load from and save beside (`None`
    /// until [`Self::load_options`]: SAVE then warns and keeps the
    /// settings in memory, never failing the scene).
    options_root: Option<std::path::PathBuf>,
    /// Lane armed for rebind by a Key-tab `KEY1..KEY7` click (`None`
    /// idle): the next mapped key press becomes that lane's key.
    option_rebind: Option<usize>,
    /// Long-note switch snapshotted at song setup (the exe caches
    /// `LongnoteMode` into the play object at `+0x17d8` instead of
    /// re-reading mid-play): `None` until the first Playing entry.
    play_longnote: Option<i32>,
    /// Staged quit-confirm sheet (`Dialog_Question1` placements
    /// plus figures; `None` until staged, the fallback popup then
    /// paints instead).
    planet_quit_art: Option<SceneArt>,
    /// Staged select-music catalog (`OJNList.dat` order, owned copies;
    /// see [`Self::set_song_catalog`]).
    catalog_songs: Vec<SelectMusicSong>,
    /// Staged catalog tempos by song id (`OJNList.dat` row +20 BPM
    /// column, the waiting `BPM` caption source; the title table
    /// above drops the column).
    catalog_bpm: HashMap<u32, f32>,
    /// Staged catalog container tokens by song id (`OJNList.dat` row
    /// +240 filename, e.g. `o2ma757.ojm`): the album-cover path source
    /// (see [`Self::resolve_song_cover`]).
    catalog_chart_files: HashMap<u32, String>,
    /// Played song's album cover: the OJN cover section of the walked
    /// chart, decoded to RGB555 for the result music-name box (see
    /// [`Self::resolve_song_cover`]). `None` paints the staged box
    /// as-is; nothing is invented.
    song_cover: Option<Rgb555Image>,
    /// Staged inventory counts, one per skill slot (the waiting
    /// `SKILLCOUNT` source; `JoinedRoom.skills` carries the ring ids
    /// only). Zeroed until staged.
    skill_counts: [u32; 3],
    select_music_art: Option<SceneArt>,
    /// Open room dialog, if any (create-room or password entry over Room).
    room_dialog: Option<RoomDialog>,
    /// Create-room form behind the dialog (cleared on open).
    create_room_form: CreateRoomForm,
    /// Password form behind the join dialog (cleared on open).
    password_form: PasswordForm,
    /// Staged create-room sheet (`DIALOG_CREATEROOM` placements+figures).
    create_room_art: Option<SceneArt>,
    /// Staged password sheet (`DIALOG_INPUTPASSWORD` placements+figures).
    password_art: Option<SceneArt>,
    /// Staged billing sheets (`STATE_PAYMENT` + `STATE_PAYMENT2`
    /// placements+figures, merged like the shots side-walk). `None`
    /// until the live boot stages them; the painter still reads the
    /// side slots, so paint wiring rides a later ticket.
    payment_art: Option<SceneArt>,
    /// Staged exe-literal Playing sheets (`State_Playing_network` +
    /// `Avatar_Info` + `Map_Avatar` placements, no figures — no block
    /// names a body), keyed by bind file. Empty until the live boot
    /// stages them beside the Playing scene; paint wiring rides a
    /// later ticket.
    byname_art: HashMap<String, SceneArt>,
    /// Queued session-bridge sends (the wire outbox, #139 cluster 1):
    /// every `pending_*` queue lives here and drains through its
    /// `take_*` accessor; the hash fold reads it for the two hashed
    /// sends.
    outbox: PendingOutbox,
    /// Create-room edit focus: title until the password EDIT is hit.
    create_focus_password: bool,
    battle_teams: RoomTeams,
    battle_self_team: Option<u8>,
    /// Notes of the loaded chart, remembered for the grade plate's
    /// stand-in ratio (see [`Self::grade`]).
    chart_notes: u32,
    side: Option<SideScreen>,
    side_origin: Option<Screen>,
    /// Lanking board state (the catalog, sort, pages, board, and
    /// pick; #139 cluster 2): every `lanking_*` word lives here;
    /// the hash fold reads it for the six hashed words.
    lanking: LankingState,
    /// Item-shop board state (the catalog, pages, and cart; #139
    /// cluster 3): every `shop_*` word lives here; the hash fold
    /// reads it for the four hashed words.
    shop: ShopState,
    /// Live wallet words behind the shop gem cells (5028/5001/5003
    /// folds; `--` until data).
    wallet: WalletState,
    /// Song-shop board state (the catalog, sort, pages, board, and
    /// cart; #139 cluster 4): every `music_shop_*` word lives here;
    /// the hash fold reads it for the six hashed words.
    music_shop: MusicShopState,
    side_placements: Vec<ScenePlacement>,
    side_figures: HashMap<String, AvatarFigure>,
    /// Live avatar-room selection (slot pick, bag/gift pages+cursors).
    myroom: MyRoomState,
    /// Staged avatar-item table (`Itemdata_Korea.dat`): bag, equip
    /// and gift name source (`None` until staged — rows and captions
    /// then stay unpainted).
    myroom_items: Option<AvatarItemTable>,
    /// Staged character data (`GetCharacterInfo` 2001): equipment,
    /// inventory, gifts, identity and cash captions (`None` until the
    /// session stages it).
    myroom_character: Option<CharacterInfo>,
    /// Staged worn figure per waiting seat (seat 0 self, seats 1..7
    /// wire peers): the same `avatar.opa` stack the Result banner and
    /// the Room panel paint, keyed by seat so each occupied slot
    /// paints its own member. `None` seats stay empty.
    waiting_figures: [Option<AvatarFigure>; 8],
    /// Avatar archive plus item table backing the per-seat worn
    /// figures (`None` until the launcher stages `--avatar-opa`):
    /// every later `set_joined_room` restages the seats through it,
    /// so peers wear their wire equipment as soon as the snapshot
    /// lands. The self seat still needs the fetched character
    /// (`set_myroom_character`) — without it only the peer seats
    /// stage.
    avatar_source: Option<(Vec<u8>, AvatarItemTable)>,
    /// Worn figure the Room lobby paints in its avatar slot
    /// (`IDC_IMAGE_SROOM_AVATA`): the same stack the binary builds from
    /// `avatar.opa` for the Result screen. `None` paints no figure.
    room_avatar: Option<AvatarFigure>,
    scene_art: [Option<SceneArt>; 12],
}

/// Key-sound bank plus backend for windowed play, with the
/// scene-BGM channel alongside: the bank holds one slot per
/// `Music/BGM.ojm` record (blanks as `None`), and every screen
/// entry re-cues through [`Screen::bgm_cue`] (silence until a
/// bank stages).
struct AudioRig {
    backend: Box<dyn AudioBackend>,
    bank: HashMap<u32, PcmSamples>,
    bgm: SceneBgm,
    bgm_bank: Vec<Option<PcmSamples>>,
}

/// Team index under native pixel (`x`, `y`): the staged team-button
/// placement whose code maps through [`waiting_team_for_code`] and
/// whose parsed `.bnd` rect contains the point (owner rule `f38f3`:
/// geometry from parsed rects, never literals). `None` elsewhere.
#[must_use]
fn waiting_team_at(placements: &[ScenePlacement], x: i32, y: i32) -> Option<u8> {
    placements.iter().find_map(|placed| {
        waiting_team_for_code(placed.code).filter(|_| rect_hit(&placed.rect, x, y))
    })
}
/// Stage word behind one room arena value (the Playing applier's
/// seeding rule, exe `0x4e78db`, `docs/stage-bga.md`): the low 16
/// bits when a high word is set, the full value otherwise. `None`
/// names no stage — a zero stage (room entry replaces zero with a
/// real one, store `0x4aacea`) or a negative word.
#[must_use]
/// Saturating `f32` to `i16` for scaled scene-BGM samples.
///
/// PERF-07 prefers `TryFrom` over `as` for narrowing conversions,
/// but `std` offers no `TryFrom<f32>` for integer types, so the
/// float step keeps one `as`: the input is clamped to the `i16`
/// range first, which makes the conversion exact — Rust's
/// float-to-int `as` saturates, and a clamped finite input never
/// reaches the saturation or NaN paths, so unity gain stays
/// bit-exact.
fn saturating_f32_to_i16(value: f32) -> i16 {
    value.clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
}

/// Scale one scene-BGM cue by a linear gain: i16 samples scaled
/// with saturation toward silence. Unity gain is bit-exact (every
/// i16 value survives the f32 round trip), so a full-volume cue
/// keeps the bank bytes.
fn scale_scene_bgm(pcm: &PcmSamples, gain: f32) -> PcmSamples {
    let samples = pcm
        .samples
        .iter()
        .map(|sample| {
            let scaled = (f32::from(*sample) * gain).round();
            saturating_f32_to_i16(scaled)
        })
        .collect();
    PcmSamples {
        samples,
        sample_rate: pcm.sample_rate,
        channels: pcm.channels,
    }
}

fn arena_stage_word(arena: i32) -> Option<u16> {
    let word = u32::try_from(arena).ok()?;
    let high = word >> 16;
    let stage = if high != 0 { word & 0xFFFF } else { word };
    u16::try_from(stage).ok().filter(|stage| *stage != 0)
}

/// Loading-gate phase named by one staged walk fraction: the
/// fraction maps onto [`LOADING_GATE_PHASES`] (0 entry, 0.25
/// staging, 0.5 OJN decrypt, 0.75 Vorbis decode, 1.0 clock arm),
/// so callers thread phases through their staged statuses
/// (`loading_phase_status` in the CLI layer) and the shell never
/// invents a phase from the clock. NaN/negative fractions read as
/// entry (the clamped paint rule); never `as` (PERF-07).
#[must_use]
fn loading_phase_for_progress(progress: f32) -> usize {
    let clamped = if progress.is_nan() {
        0.0_f32
    } else {
        progress.clamp(0.0, 1.0)
    };
    let scaled = clamped * LOADING_GATE_PHASES as f32;
    usize::try_from(scaled.round() as i64)
        .unwrap_or(0)
        .min(LOADING_GATE_PHASES)
}

/// Install `Music` path of one catalog container token: the record's
/// `.ojm` token names its chart file with the `.ojn` extension
/// (`o2ma757.ojm` → `Music/o2ma757.ojn`; the shipped tokens match
/// their chart files 14/14, both generations). `None` when the token
/// carries no `.ojm` extension (no invented fallback path).
#[must_use]
fn chart_cover_path(token: &str) -> Option<std::path::PathBuf> {
    let stem_len = token.len().checked_sub(".ojm".len())?;
    let stem = token
        .get(..stem_len)
        .filter(|_| token[stem_len..].eq_ignore_ascii_case(".ojm"))?;
    Some(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music")
            .join(format!("{stem}.ojn")),
    )
}

/// Decoded cover pixels as an RGB555 image (5-bit channels; the frame
/// blit composes in RGB555 words). `None` when the pixel buffer does
/// not tile the claimed dimensions.
#[must_use]
fn cover_image(decoded: &o2jam_file::JpegRgbImage) -> Option<Rgb555Image> {
    let pixels = decoded.width.checked_mul(decoded.height)?.checked_mul(3)?;
    if decoded.pixels.len() != pixels {
        return None;
    }
    let words = decoded
        .pixels
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| {
            let [r, g, b] = *pixel;
            (u16::from(r) >> 3) << 10 | (u16::from(g) >> 3) << 5 | u16::from(b) >> 3
        })
        .collect();
    Some(Rgb555Image {
        w: decoded.width,
        h: decoded.height,
        words,
    })
}

/// Parsed chat draft behind the lobby/waiting slash grammar: the exe
/// lobbies its 2017 whisper send behind `/w`, `/r`, `/h`, `/help`
/// literals (VAs `0x647d60`+, send at file `0x8fa02`), and the waiting
/// room sends 2017 the same way (files `0xb8ffc`/`0xb9369`). Unknown
/// commands fall back to plain chat, so only these four tokens divert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    /// The draft is empty: nothing to send, so the scene may advance.
    Empty,
    /// Plain chat: the 2012 (Channel) or 3011 (waiting) body.
    Chat(String),
    /// Whisper: the 2017 recipient plus text.
    Whisper {
        /// Named addressee (`/w` name, or the last peer for `/r`).
        recipient: String,
        /// Whisper text (the words after the name).
        content: String,
    },
    /// Help (`/h`, `/help`): local only, no packet.
    Help,
}

/// Split a `/`-draft into its command token plus the rest
/// (`/w bob hi` to `("w", " bob hi")`). `None` while the draft
/// names no command at all.
fn split_command_token(draft: &str) -> Option<(&str, &str)> {
    let body = draft.strip_prefix('/')?;
    let end = body.find(char::is_whitespace).unwrap_or(body.len());
    Some((body.get(..end)?, body.get(end..).unwrap_or("")))
}

/// Split a `/w` tail into name plus text (`"  bob   hi there "` to
/// `("bob", "hi there")`; extra gaps collapse, ends trim).
/// `None` while the name or the text is missing — the draft then
/// falls back to plain chat.
fn parse_whisper_rest(rest: &str) -> Option<(String, String)> {
    let name_start = rest.find(|c: char| !c.is_whitespace())?;
    let unprefixed = rest.get(name_start..)?;
    let name_end = unprefixed
        .find(char::is_whitespace)
        .unwrap_or(unprefixed.len());
    let recipient = unprefixed.get(..name_end)?;
    if recipient.is_empty() {
        return None;
    }
    let content = unprefixed.get(name_end..)?.trim();
    if content.is_empty() {
        return None;
    }
    Some((recipient.to_owned(), content.to_owned()))
}

/// Split a `/r` tail against the last whisper peer (the tail is the
/// whole text). `None` with no peer or no text — the draft then
/// falls back to plain chat.
fn parse_reply_text(rest: &str, reply_to: Option<&str>) -> Option<(String, String)> {
    let peer = reply_to.filter(|peer| !peer.is_empty())?;
    let content = rest.trim();
    if content.is_empty() {
        return None;
    }
    Some((peer.to_owned(), content.to_owned()))
}

/// Parse one draft through the slash grammar. The draft stays whole
/// for `Chat` (unknown or incomplete commands send as-is); `Help`
/// carries no packet; `Empty` names the empty draft.
fn parse_slash_draft(draft: &str, reply_to: Option<&str>) -> SlashCommand {
    if draft.is_empty() {
        return SlashCommand::Empty;
    }
    if !draft.starts_with('/') {
        return SlashCommand::Chat(draft.to_owned());
    }
    let Some((command, rest)) = split_command_token(draft) else {
        return SlashCommand::Chat(draft.to_owned());
    };
    match command {
        "w" => parse_whisper_rest(rest).map_or_else(
            || SlashCommand::Chat(draft.to_owned()),
            |(recipient, content)| SlashCommand::Whisper { recipient, content },
        ),
        "r" => parse_reply_text(rest, reply_to).map_or_else(
            || SlashCommand::Chat(draft.to_owned()),
            |(recipient, content)| SlashCommand::Whisper { recipient, content },
        ),
        "h" | "help" => SlashCommand::Help,
        _ => SlashCommand::Chat(draft.to_owned()),
    }
}

/// Stack one worn figure per occupied waiting seat: the self seat
/// wears the fetched character equipment, peers wear their wire
/// `RoomMember.equipment`, all through the shared [`load_worn_figure`]
/// path. Unresolvable pieces stay empty; without a joined room this
/// stages nothing. Free function so the public stager and the join
/// restage share one body without borrowing the whole game.
fn stage_seat_figures(
    waiting: &Option<JoinedRoom>,
    character: &Option<CharacterInfo>,
    archive: &[u8],
    table: &AvatarItemTable,
    figures: &mut [Option<AvatarFigure>; 8],
) {
    let Some(room) = waiting.as_ref() else {
        return;
    };
    if let Some(character) = character.as_ref() {
        let male = character.gender == Gender::Male;
        let figure = load_worn_figure(archive, table, &character.equipment, male);
        if !figure.is_empty() {
            figures[0] = Some(figure);
        }
    }
    for (offset, slot) in room.slots.iter().take(7).enumerate() {
        let Some(member) = slot.member.as_ref() else {
            continue;
        };
        let male = Gender::from_byte(member.gender) == Some(Gender::Male);
        let figure = load_worn_figure(archive, table, &member.equipment, male);
        if !figure.is_empty() {
            figures[offset + 1] = Some(figure);
        }
    }
}

impl ScreenGame {
    /// Start at the splash screen. The song clock is the audio
    /// backend's sample counter anchored on Playing entry
    /// (`song_anchor_sample`); splash paints until an audio device
    /// proves otherwise (live splash auto-advance keys off the
    /// same `now_ms`, so without audio it waits for input — tests
    /// drive splash timing with explicit clocks).
    #[must_use]
    pub fn new() -> Self {
        Self {
            flow: ScreenFlow::new(),
            playing: LivePlay::new(&[]),
            song_anchor_sample: 0,
            song_rate: 44_100,
            fixed_clock_step_ms: None,
            fixed_now_ms: 0,
            splash_hold_ms: SPLASH_AUTO_ADVANCE_MS,
            avi_entered_wall: Some(std::time::Instant::now()),
            audio: None,
            avatar: None,
            channels: Vec::new(),
            highlight: 0,
            channel_top: 0,
            pointer: None,
            channel_chat: LobbyChatPanel::default(),
            room_chat: LobbyChatPanel::default(),
            last_room_control_noop: None,
            last_whisper_peer: None,
            rooms: Vec::new(),
            room_highlight: 0,
            waiting: None,
            waiting_chat: Vec::new(),
            chat_input: String::new(),
            selected_music: None,
            loading_status: None,
            loading_gate_phase: 0,
            loading_gate_failed: false,
            arena_stage: DEFAULT_ARENA_STAGE,
            planet_board: PlanetBoardState::default(),
            quit_requested: false,
            planet_option_art: [None, None, None],
            planet_option_tab: PLANET_OPTION_FIRST_TAB,
            options: OptionsSettings::default(),
            options_root: None,
            option_rebind: None,
            play_longnote: None,
            planet_quit_art: None,
            catalog_songs: Vec::new(),
            catalog_bpm: HashMap::new(),
            catalog_chart_files: HashMap::new(),
            song_cover: None,
            skill_counts: [0, 0, 0],
            select_music_art: None,
            room_dialog: None,
            create_room_form: CreateRoomForm::new(),
            password_form: PasswordForm::new(),
            create_room_art: None,
            password_art: None,
            payment_art: None,
            byname_art: HashMap::new(),
            outbox: PendingOutbox::default(),
            create_focus_password: false,
            battle_teams: RoomTeams::new(),
            battle_self_team: None,
            chart_notes: 0,
            side: None,
            side_origin: None,
            lanking: LankingState::default(),
            shop: ShopState::default(),
            wallet: WalletState::default(),
            music_shop: MusicShopState::default(),
            side_placements: Vec::new(),
            side_figures: HashMap::new(),
            myroom: MyRoomState::new(),
            myroom_items: None,
            myroom_character: None,
            waiting_figures: std::array::from_fn(|_| None),
            avatar_source: None,
            room_avatar: None,
            scene_art: std::array::from_fn(|_| None),
        }
    }

    /// Load the result-screen avatar figure.
    pub fn load_avatar(&mut self, figure: AvatarFigure) -> &mut Self {
        self.avatar = Some(figure);
        self
    }

    /// Attach key sounds; struck notes play through the backend.
    /// Missing samples stay silent (no bank entry, no sound). The
    /// scene-BGM bank stages separately ([`Self::stage_scene_bgm`]).
    pub fn with_audio(
        &mut self,
        backend: Box<dyn AudioBackend>,
        bank: HashMap<u32, PcmSamples>,
    ) -> &mut Self {
        self.audio = Some(AudioRig {
            backend,
            bank,
            bgm: SceneBgm::new(),
            bgm_bank: Vec::new(),
        });
        self
    }

    /// Stage the scene-BGM bank (one slot per `Music/BGM.ojm`
    /// record) and cue the current screen through it: boot cues
    /// Avi (id 8, once), and every later [`Self::enter_screen`]
    /// re-cues its own screen (Planet 9 loop, silence `-1` on
    /// leave to cue-less screens) — a re-cue that repeats the
    /// sounding record leaves the running track alone. Without an
    /// audio rig the bank drops (headless walks cue nothing).
    pub fn stage_scene_bgm(&mut self, bank: Vec<Option<PcmSamples>>) -> &mut Self {
        if let Some(rig) = self.audio.as_mut() {
            rig.bgm_bank = bank;
        }
        self.apply_scene_bgm();
        self
    }

    /// Cue one scene-BGM pair through the rig (silence without one):
    /// the one application every scene entry flows through.
    ///
    /// The channel keeps the running track when the cue names the
    /// record already latched in the same repeat mode at the same
    /// volume (see [`SceneBgm::cue`]): Planet and its channel dialog
    /// share record 9, and a shared track must not restart from
    /// sample 0 on every emblem click.
    fn apply_bgm_cue(&mut self, cue: (i32, bool)) {
        let Some(rig) = self.audio.as_mut() else {
            return;
        };
        let (cue_id, cue_repeat) = cue;
        let AudioRig {
            backend,
            bgm,
            bgm_bank,
            ..
        } = rig;
        // The BGMusic volume (`Sound`/`GameBGVolume`, default 255 is
        // unity) scales the lobby cue: the Sound tab moves the live
        // scene mix (a gain change re-cues, the only path that
        // carries the level to the mixer). The in-song mix path stays
        // OPEN.
        let gain = self.options.bgm_gain();
        let request = SceneCue {
            id: cue_id,
            repeat: cue_repeat,
            gain,
        };
        let sounding = backend.scene_bgm_playing();
        match bgm.cue(request, bgm_bank, sounding) {
            CueOutcome::KeepRunning => {}
            CueOutcome::Restart(pcm) => {
                let scaled = scale_scene_bgm(pcm, gain);
                let _ = backend.set_scene_bgm(Some(&scaled), cue_repeat);
            }
            CueOutcome::Stop => {
                let _ = backend.set_scene_bgm(None, cue_repeat);
            }
        }
    }

    /// Cue the current screen's scene music through the rig
    /// (silence without one). The one entry hook every screen
    /// change flows through — see [`Self::enter_screen`].
    fn apply_scene_bgm(&mut self) {
        let cue = self.flow.current().bgm_cue();
        self.apply_bgm_cue(cue);
    }

    /// Latched scene-BGM selection, if a track is playing (`None`
    /// silent, rig-less, or cue-less — headless walks cue nothing).
    #[must_use]
    pub fn scene_bgm_selection(&self) -> Option<BgmSelection> {
        self.audio.as_ref().and_then(|rig| rig.bgm.current())
    }

    /// Load a chart's onsets (with key-sound ids) into the highway.
    /// The selected chart's difficulty re-applies to the fresh
    /// [`LivePlay`] (see [`Self::apply_life_difficulty`]); the note
    /// count feeds the result grade's ratio.
    pub fn load_chart(&mut self, onsets: Vec<(u8, u64, u32)>) {
        self.chart_notes = u32::try_from(onsets.len()).unwrap_or(u32::MAX);
        self.playing = LivePlay::new(&onsets);
        self.apply_life_difficulty();
    }

    /// Stage real note/key art into the highway paint (procedural
    /// fallback stays on unstaged lanes).
    pub fn set_play_skin(&mut self, skin: PlaySkin) {
        self.playing.set_skin(skin);
    }

    /// Caption the avatar scene with the local player (`Lv.<n> <name>`;
    /// display only, scoring untouched).
    pub fn set_player(&mut self, level: u32, name: &str) {
        self.playing.scene_mut().set_player(level, name);
    }

    /// Caption the avatar scene song line (`X3.5 <title>`); the waiting
    /// choice re-syncs each Playing paint while one is selected.
    pub fn set_song_caption(&mut self, title: &str, speed_code: u8) {
        self.playing.scene_mut().set_song(title, speed_code);
    }

    /// Caption the VOL readout (display only).
    pub fn set_volume(&mut self, volume: u8) {
        self.playing.scene_mut().set_volume(volume);
    }

    /// Play every queued key sound through the attached backend at
    /// the GameEffect volume (`Sound`/`NoteKeyVolume`, default 255
    /// is unity): the Sound tab moves the live key mix.
    fn pump_sounds(&mut self) {
        let gain = self.options.key_gain();
        let Some(rig) = self.audio.as_mut() else {
            self.playing.drain_sounds();
            return;
        };
        for sample in self.playing.drain_sounds() {
            if let Some(pcm) = rig.bank.get(&sample) {
                let _ = rig.backend.play_key(pcm, gain);
            }
        }
    }

    /// Set the device sample rate for the song-clock conversion
    /// ([`song_position_ms`]). Playing entries auto-sync it from
    /// the backend's [`AudioBackend::sample_rate`] (so a 48000 Hz
    /// device runs at the right speed); this manual override is
    /// for tests and forced rates (rate 0 parks the song at 0).
    pub fn set_song_rate(&mut self, rate: u32) {
        self.song_rate = rate;
    }

    /// Enable the fixed-clock test affordance: every `paint`
    /// advances the virtual clock by `step_ms` and [`Self::now_ms`]
    /// serves it instead of the audio sample clock. Trace replays
    /// through the real window use this (null audio would otherwise
    /// park the song at 0 forever). Production runs never call this —
    /// the sample clock rules there.
    pub fn enable_fixed_clock(&mut self, step_ms: u64) -> &mut Self {
        self.fixed_clock_step_ms = Some(step_ms);
        self
    }

    /// Test affordance: override the splash hold (0 advances on the
    /// first paint, so wall-clock guards need no sleeps).
    pub fn set_splash_hold_ms(&mut self, hold_ms: u64) -> &mut Self {
        self.splash_hold_ms = hold_ms;
        self
    }

    /// True while the wall hold since the Avi entry covered the
    /// splash hold (claims B38: menus pace on the wall clock).
    fn splash_wall_hold_reached(&self) -> bool {
        self.avi_entered_wall
            .is_some_and(|entered| entered.elapsed().as_millis() >= u128::from(self.splash_hold_ms))
    }

    /// Windowed splash auto-advance on the wall clock: skipped while
    /// the fixed clock rules (the headless seam owns timing there),
    /// so null-audio runs still reach Planet on the hold.
    fn advance_splash_on_wall_clock(&mut self) {
        if self.fixed_clock_step_ms.is_some() || self.flow.current() != Screen::Avi {
            return;
        }
        if self.splash_wall_hold_reached() {
            self.enter(Screen::Planet);
        }
    }

    /// Song position in ms from the audio backend's sample
    /// counter, anchored at Playing entry
    /// (`song_anchor_sample`) — strike timing, scroll, and gauge
    /// advances all consume this, never the wall clock. Without
    /// an audio rig the song parks at 0 (tests and headless runs
    /// drive `now_ms` explicitly through `paint_at` / `strike_at`).
    /// The fixed-clock test affordance overrides everything: the
    /// virtual clock serves menus (splash auto-advance) and Playing
    /// alike so trace replays stay deterministic without audio.
    fn now_ms(&mut self) -> u64 {
        if self.fixed_clock_step_ms.is_some() {
            return self.fixed_now_ms;
        }
        let Some(rig) = self.audio.as_ref() else {
            return 0;
        };
        song_position_ms(
            rig.backend
                .samples_played()
                .saturating_sub(self.song_anchor_sample),
            self.song_rate,
        )
    }

    /// Step the flow, re-anchoring the song clock on Playing entry.
    fn enter(&mut self, next: Screen) {
        // A scene change while the button is held drops the press:
        // the pressed control belongs to the scene being left.
        self.planet_board.arm_press(None);
        self.enter_screen(next);
    }

    /// Step the flow through the verified edge gates (shot/test
    /// driver): `false` when `can_enter` rejects, so no transition is
    /// invented. The Loading-to-Playing edge is the gate: it holds
    /// unless [`Self::loading_gate_ready`], so an unvalidated,
    /// unstaged, undecrypted, or audio-less gate never invents the
    /// transition and never arms the clock. Entering Loading seeds
    /// the gate from any status the loader already staged (the
    /// session stages the clock-armed caption before the 4011 edge),
    /// else starts it at entry. On a held Playing entry the song
    /// clock anchors to backend samples there and the new screen's
    /// scene music re-cues.
    pub fn enter_screen(&mut self, next: Screen) -> bool {
        if next == Screen::Playing
            && self.flow.current() == Screen::Loading
            && !self.loading_gate_ready()
        {
            return false;
        }
        let entered = self.flow.go(next).is_ok();
        if entered {
            if next == Screen::Avi {
                self.avi_entered_wall = Some(std::time::Instant::now());
            }
            if next == Screen::Room {
                self.room_dialog = None;
            }
            if next == Screen::Loading {
                self.loading_gate_failed = false;
                self.loading_gate_phase = self
                    .loading_status
                    .as_ref()
                    .map_or(0, |status| loading_phase_for_progress(status.progress));
            }
            if next == Screen::Playing {
                self.song_anchor_sample = self
                    .audio
                    .as_ref()
                    .map_or(0, |rig| rig.backend.samples_played());
                if let Some(rig) = self.audio.as_ref() {
                    self.song_rate = rig.backend.sample_rate();
                }
                // Song setup snapshots the long-note switch (the exe
                // caches `LongnoteMode` at play `+0x17d8`, file
                // `0x16fb06`): later dialog edits never reach the
                // running song.
                self.play_longnote = Some(self.options.longnote_mode());
            }
            self.apply_scene_bgm();
        }
        entered
    }

    /// Strike a lane at an explicit song clock, playing its key
    /// sound. Tests use this; live keys read the sample clock
    /// ([`Self::now_ms`]).
    pub fn strike_at(&mut self, lane: usize, now_ms: u64) {
        self.playing.strike_lane(lane as u8, now_ms);
        self.pump_sounds();
    }

    /// Release a lane (key-up) at an explicit song clock, judging
    /// the armed hold tail through the verified release path.
    /// Tests use this; live key-ups are not yet routed (lane_key
    /// ignores key-up — OPEN, own lane).
    pub fn release_at(&mut self, lane: usize, now_ms: u64) {
        self.playing.release_lane(lane as u8, now_ms);
        self.pump_sounds();
    }

    /// Load a chart with long-note spans: `(lane, head ms, tail ms,
    /// sample)` — a tail at or before its head degrades to a tap
    /// ([`LivePlay::new_holds`]). The chart's difficulty re-applies
    /// and the note count feeds the grade ratio, like
    /// [`Self::load_chart`].
    pub fn load_chart_holds(&mut self, entries: Vec<(u8, u64, u64, u32)>) {
        self.chart_notes = u32::try_from(entries.len()).unwrap_or(u32::MAX);
        self.playing = LivePlay::new_holds(&entries);
        self.apply_life_difficulty();
    }

    /// Screen currently shown.
    #[must_use]
    pub fn screen(&self) -> Screen {
        self.flow.current()
    }

    /// Channels from the last online list (empty offline). Refreshing
    /// the list clears the channel chat history: rows come from that
    /// list's 2013/2014/2016/2019/2029 packets only, never carried
    /// across lists.
    pub fn set_channels(&mut self, channels: Vec<ChannelState>) -> &mut Self {
        self.channels = channels;
        self.highlight = 0;
        // The exe zeroes the list top beside the populate (the
        // `+0x184 = 0` + scrollbar `+0xb8 = 0` joint reset, file
        // `0x747ba`): a fresh list always opens unscrolled.
        self.channel_top = 0;
        self.channel_chat = LobbyChatPanel::default();
        self
    }

    /// Rooms from the last online list (empty offline). Free slots are
    /// not listed: the server numbers rooms from the first free slot
    /// (so a real first room is number 0) and marks free slots with
    /// an empty title — filter on both, the data stays unfiltered
    /// upstream. Corroborated against the reference server layout
    /// (`AddRoom` first-free-slot ids, `GetRoomList` empty titles);
    /// the 7.55 display rule itself stays OPEN.
    pub fn set_rooms(&mut self, rooms: Vec<RoomListEntry>) -> &mut Self {
        self.rooms = rooms.into_iter().filter(Self::is_listed_room).collect();
        self.room_highlight = 0;
        self
    }

    /// Whether a room-list row names a real room (see [`Self::set_rooms`]).
    fn is_listed_room(room: &RoomListEntry) -> bool {
        room.number != 0 || !room.title.is_empty()
    }

    /// Remember the joined room; Waiting paints its eight seats, and
    /// the room's arena word sets the Playing backdrop stage (zero or
    /// negative words keep the current stage: the exe replaces a zero
    /// stage on room entry, it never stores one). Joining a new room
    /// clears the chat history and draft: rows come from that room's
    /// 3012/3013 packets only, never carried across rooms.
    pub fn set_joined_room(&mut self, room: JoinedRoom) -> &mut Self {
        if let Some(stage) = arena_stage_word(room.arena) {
            self.arena_stage = stage;
        }
        self.waiting = Some(room);
        self.waiting_chat.clear();
        self.chat_input.clear();
        // A new room means new seats: drop the old per-seat figures,
        // then peers wear their wire equipment as soon as the snapshot
        // lands (no-op without a staged avatar source).
        self.waiting_figures = std::array::from_fn(|_| None);
        self.restage_waiting_figures();
        self
    }

    /// Mark one member's ready flag in place (ready echoes and the
    /// local toggle fold here — never rebuilds the room, so waiting
    /// chat histories survive, unlike [`Self::set_joined_room`]).
    /// Unknown members and no room are silent no-ops.
    pub fn set_member_ready(&mut self, member_id: u8, ready: bool) -> &mut Self {
        if let Some(room) = self.waiting.as_mut() {
            if let Some(slot) = room.slots.iter_mut().find(|slot| slot.index == member_id) {
                if let Some(member) = slot.member.as_mut() {
                    member.ready = ready;
                }
            }
        }
        self
    }

    /// Toggle-select one waiting team button (the toggle-group map).
    ///
    /// The pressed `TEAMA..H` button maps to its team index with the
    /// shared toggle helper `FUN_0048c810` (file `0x8c810`, REA
    /// `ev_0f6bbde1...`; also the E1 planet-emblem toggle): the helper
    /// lights exactly the pressed button and returns
    /// `pressed - 0x16201004` (team 0..7). The highlight always moves
    /// (`battle_self_team` + the joined room's self team, so the
    /// staged `Wait_Team?.ojs` highlight follows).
    ///
    /// The send follows the dispatch `FUN_004b22d0` (file `0xb22d0`):
    /// the gate helper `FUN_00556680` (file `0x16680`, opcode-id scan)
    /// plus the room-mode bytes (`0x77cd30..32`) decide whether the
    /// press sends. Gated presses only refresh the highlight; ungated
    /// presses queue `SetRoomTeam` 4004 (team index) then `Ready` 4008
    /// (ready cleared on a team switch) for the session bridge.
    ///
    /// OPEN: the handler-table contents behind the gate helper and the
    /// room-mode byte meanings are untraced; the shell treats "no
    /// joined room" as gated (highlight only) and joined as sendable.
    pub fn select_team(&mut self, team: u8) {
        // Toggle-group map (FUN_0048c810): exactly the pressed team
        // lights; the room mirror follows so the staged highlight
        // tracks the selection.
        self.battle_self_team = Some(team);
        if let Some(room) = self.waiting.as_mut() {
            room.team = team;
        }
        // Ungated join sends the pair in exe order (4004 then 4008).
        // Without a joined room there is no send target (gated arm).
        if self.waiting.is_some() {
            self.outbox.pending_room_team = Some(SetRoomTeam { team });
            self.outbox.pending_team_ready = true;
        }
    }
    /// Queued team send for the session bridge, if any (4004). One
    /// take sends once; the next press replaces the queued team.
    pub fn take_room_team(&mut self) -> Option<SetRoomTeam> {
        self.outbox.take_room_team()
    }
    /// Queued ready-clear for the session bridge, if any (4008).
    /// The team switch queues it directly behind its 4004.
    pub fn take_team_ready(&mut self) -> Option<Ready> {
        self.outbox.take_team_ready()
    }
    /// Whether a team-button pair is queued for the session bridge.
    #[must_use]
    pub fn has_pending_team_send(&self) -> bool {
        self.outbox.has_pending_team_send()
    }
    /// Queued song pick for the session bridge, if any (4000). One
    /// take sends once; the next row press replaces the queued pick.
    pub fn take_music_send(&mut self) -> Option<SetRoomMusic> {
        self.outbox.take_music_send()
    }
    /// Whether a dialog row pick is queued for the session bridge.
    #[must_use]
    pub fn has_pending_music_send(&self) -> bool {
        self.outbox.has_pending_music_send()
    }
    /// Queued RETRY press for the session bridge, if any (`StartGame`
    /// 4010, `0xfaa` — the exe's result RETRY relay event,
    /// `FUN_004c2f00` range, `ev_77ce98f1…`). One take sends once;
    /// the next press replaces the queue.
    pub fn take_result_retry(&mut self) -> Option<StartGame> {
        self.outbox.take_result_retry()
    }
    /// Queued BACK press for the session bridge, if any (`ExitPlaying`
    /// 4021, `0xfb5` — the exe's result BACK relay event,
    /// `docs/cresult-anchors.md` `ev_77ce98f1…`). One take sends once;
    /// the next press replaces the queue.
    pub fn take_result_back(&mut self) -> Option<ExitPlaying> {
        self.outbox.take_result_back()
    }
    /// Queued battle result entry for the session bridge, if any
    /// (`GetLiveState` 4042, `0xfca` — byte-verified empty,
    /// `docs/relay.md` §10, gated on the battle global `0x7789e6`).
    /// One take sends once; the next battle entry replaces the queue.
    pub fn take_result_battle_entry(&mut self) -> Option<o2jam_net::relay::GetLiveState> {
        self.outbox.take_result_battle_entry()
    }
    /// Team broadcast slot-refresh path (4007).
    ///
    /// Handler `FUN_0055f470` (file `0x15f470`, REA `ev_71e3467...`)
    /// guards the slot with `< 8` and forwards `(0x12, 4007, slot, 0)`
    /// into the UI event queue. This mirrors that forward: `Some(slot)`
    /// while the slot is accepted (the repaint key), `None` past it.
    ///
    /// OPEN: the member-key lookup turning the broadcast key into a
    /// team byte (guest slot-fold, live-fold gap) — the shell never
    /// decodes the key as a team; the caller repaints the slot.
    pub fn apply_room_team_changed(&self, broadcast: &RoomTeamChanged) -> Option<usize> {
        if broadcast.slot >= 8 {
            return None;
        }
        Some(usize::from(broadcast.slot))
    }
    /// Stage the create-room sheet (`DIALOG_CREATEROOM` placements plus
    /// figures through `bind_scene_rects` on `Dialog_CreateRoom.bnd`).
    /// Staging never moves the flow.
    pub fn load_create_room_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.create_room_art = Some(SceneArt {
            placements,
            figures,
        });
        self
    }
    /// True while the create-room sheet is staged (the live-boot
    /// staging gate asserts every dialog `paint_room` can open has
    /// art before the dialog opens).
    #[must_use]
    pub fn create_room_art_staged(&self) -> bool {
        self.create_room_art.is_some()
    }

    /// Stage the password sheet (`DIALOG_INPUTPASSWORD` placements plus
    /// figures through `bind_scene_rects` on `Dialog_Password.bnd`).
    /// Staging never moves the flow.
    pub fn load_password_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.password_art = Some(SceneArt {
            placements,
            figures,
        });
        self
    }
    /// True while the password sheet is staged (the live-boot
    /// staging gate asserts every dialog `paint_room` can open has
    /// art before the dialog opens).
    #[must_use]
    pub fn password_art_staged(&self) -> bool {
        self.password_art.is_some()
    }

    /// Stage the billing sheets (`STATE_PAYMENT` + `STATE_PAYMENT2`
    /// placements plus figures through the declaration, merged like
    /// the shots side-walk). Staging never moves the flow; the live
    /// boot path never visits a side screen, so without this arm the
    /// Payment overlay painted the unwired notice over empty art.
    pub fn load_payment_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.payment_art = Some(SceneArt {
            placements,
            figures,
        });
        self
    }
    /// True while the billing sheets are staged (the live-boot
    /// staging gate asserts the live path stages them like every
    /// other Interface sheet).
    #[must_use]
    pub fn payment_art_staged(&self) -> bool {
        self.payment_art.is_some()
    }

    /// Stage one exe-literal Playing sheet beside the Playing scene:
    /// the by-name row's parsed rects plus its (today empty) figures,
    /// keyed by bind file so paint wiring can find each sheet later.
    /// Staging never moves the flow.
    pub fn load_byname_sheet(
        &mut self,
        bind_file: &str,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.byname_art.insert(
            bind_file.to_owned(),
            SceneArt {
                placements,
                figures,
            },
        );
        self
    }
    /// True while the named literal sheet is staged (the live-boot
    /// staging gate asserts the live path stages the trio beside the
    /// Playing scene).
    #[must_use]
    pub fn byname_art_staged(&self, bind_file: &str) -> bool {
        self.byname_art.contains_key(bind_file)
    }

    /// Open room dialog, if any.
    #[must_use]
    pub fn room_dialog(&self) -> Option<RoomDialog> {
        self.room_dialog
    }
    /// Open the create-room dialog over Room with a cleared form.
    pub fn open_create_room_dialog(&mut self) {
        self.create_room_form = CreateRoomForm::new();
        self.create_focus_password = false;
        self.room_dialog = Some(RoomDialog::CreateRoom);
    }
    /// Open the password dialog for one locked room number.
    pub fn open_password_dialog(&mut self, number: i32) {
        self.password_form = PasswordForm::new();
        self.room_dialog = Some(RoomDialog::Password { number });
    }
    /// Committed create-room body awaiting the session send (2004).
    pub fn take_create_room(&mut self) -> Option<CreateRoom> {
        self.outbox.take_create_room()
    }
    /// Committed locked-room join awaiting the session send (3002).
    pub fn take_join(&mut self) -> Option<JoinRoom> {
        self.outbox.take_join()
    }
    /// `CreateRoom` form behind the open dialog.
    #[must_use]
    pub fn create_room_form(&self) -> &CreateRoomForm {
        &self.create_room_form
    }
    /// `Password` form behind the open dialog.
    #[must_use]
    pub fn password_form(&self) -> &PasswordForm {
        &self.password_form
    }
    /// Room dialog click: the CREATE/CANCEL/mode/edit actions on the
    /// staged `Dialog_CreateRoom.bnd` rects, or OK/CANCEL on the
    /// staged `Dialog_Password.bnd` rects. The modal owns Room while
    /// open; unstaged clicks land nowhere (geometry from parsed
    /// rects only, owner rule `f38f3`).
    pub fn click_room_dialog(&mut self, x: i32, y: i32) {
        match self.room_dialog {
            Some(RoomDialog::CreateRoom) => {
                let placements: &[ScenePlacement] = self
                    .create_room_art
                    .as_ref()
                    .map_or(&[], |art| &art.placements);
                match create_room_action_at(placements, x, y) {
                    Some(CreateRoomAction::NormalMode) => {
                        self.create_room_form.live_battle = false;
                    }
                    Some(CreateRoomAction::LiveMode) => {
                        self.create_room_form.live_battle = true;
                    }
                    Some(CreateRoomAction::ThreeKey) => {
                        self.create_room_form.seven_key = false;
                    }
                    Some(CreateRoomAction::SevenKey) => {
                        self.create_room_form.seven_key = true;
                    }
                    Some(CreateRoomAction::FocusTitle) => {
                        self.create_focus_password = false;
                    }
                    Some(CreateRoomAction::FocusPassword) => {
                        self.create_focus_password = true;
                    }
                    Some(CreateRoomAction::Commit) => {
                        if let Some(body) = self.create_room_form.commit() {
                            self.outbox.pending_create_room = Some(body);
                            self.room_dialog = None;
                        }
                    }
                    Some(CreateRoomAction::Cancel) => {
                        self.room_dialog = None;
                    }
                    None => {}
                }
            }
            Some(RoomDialog::Password { number }) => {
                let placements: &[ScenePlacement] = self
                    .password_art
                    .as_ref()
                    .map_or(&[], |art| &art.placements);
                match password_action_at(placements, x, y) {
                    Some(PasswordAction::Commit) => {
                        self.outbox.pending_join = Some(JoinRoom {
                            number,
                            music_state: 0,
                            password: self.password_form.password.clone(),
                        });
                        self.room_dialog = None;
                        self.enter(Screen::Waiting);
                    }
                    Some(PasswordAction::Cancel) => {
                        self.room_dialog = None;
                    }
                    None => {}
                }
            }
            None => {}
        }
    }
    /// Type one char into the open room dialog's focused edit (the last
    /// EDIT click owns focus; the single password edit owns itself).
    /// Only Room with a dialog open accepts typing; control chars and
    /// overlong input never enter (form caps bound the line).
    pub fn push_room_dialog_char(&mut self, typed: char) {
        if self.flow.current() != Screen::Room || self.room_dialog.is_none() {
            return;
        }
        match self.room_dialog {
            Some(RoomDialog::Password { .. }) => self.password_form.push_char(typed),
            _ => {
                if self.create_focus_password {
                    self.create_room_form.push_password_char(typed);
                } else {
                    self.create_room_form.push_title_char(typed);
                }
            }
        }
    }
    /// Lease turn: no-op kept for symmetry with the chat draft driver.
    /// Room-dialog deletion routes through [`Self::delete_room_dialog_char`].
    pub fn delete_room_dialog_char(&mut self) {
        if self.flow.current() != Screen::Room {
            return;
        }
        match self.room_dialog {
            Some(RoomDialog::Password { .. }) => self.password_form.pop(),
            _ => {
                if self.create_focus_password {
                    self.create_room_form.pop_password();
                } else {
                    self.create_room_form.pop_title();
                }
            }
        }
    }
    /// Append one player chat line (3012) to the waiting history.
    pub fn push_waiting_user_message(&mut self, message: &WaitingUserMessage) -> &mut Self {
        self.push_chat_row(&message.sender, &message.content_text(), false);
        self
    }

    /// Append one administrator chat line (3013) to the waiting history.
    pub fn push_waiting_admin_message(&mut self, message: &WaitingAdminMessage) -> &mut Self {
        self.push_chat_row(&message.sender, &message.content_text(), true);
        self
    }

    /// Live waiting-room chat rows, oldest first (empty until a
    /// 3012/3013 packet arrives).
    #[must_use]
    pub fn waiting_chat(&self) -> &[WaitingChatMessage] {
        &self.waiting_chat
    }

    /// The Channel scene's lobby chat panel.
    #[must_use]
    pub fn lobby_chat(&self, chat: LobbyChat) -> &LobbyChatPanel {
        match chat {
            LobbyChat::Channel => &self.channel_chat,
            LobbyChat::Room => &self.room_chat,
        }
    }

    /// Append one player chat line (2013) to `chat`'s history.
    pub fn push_lobby_user_message(
        &mut self,
        chat: LobbyChat,
        message: &MainRoomUserMessage,
    ) -> &mut Self {
        let row = ChannelChatMessage::user(&message.sender, &message.content_text());
        self.lobby_panel_mut(chat).push_row(row);
        self
    }

    /// Append one administrator chat line (2014) to `chat`'s history.
    pub fn push_lobby_admin_message(
        &mut self,
        chat: LobbyChat,
        message: &MainRoomAdminMessage,
    ) -> &mut Self {
        let row = ChannelChatMessage::admin(&message.sender, &message.content_text());
        self.lobby_panel_mut(chat).push_row(row);
        self
    }

    /// Append one incoming whisper (2019) to `chat`'s history.
    /// `recipient` names the local addressee (the wire event carries
    /// only the sender), so the row keeps its whisper context. The
    /// sender becomes the `/r` peer.
    pub fn push_lobby_whisper(
        &mut self,
        chat: LobbyChat,
        message: &ReceiveWhisper,
        recipient: &str,
    ) -> &mut Self {
        let row = ChannelChatMessage::whisper(&message.sender, recipient, &message.content);
        self.lobby_panel_mut(chat).push_row(row);
        self.remember_whisper_peer(&message.sender);
        self
    }

    /// Append one lobby announcement (2016) to `chat`'s history: the
    /// exe fans the notice text out as `0x7de` admin-message rows
    /// (`docs/net-protocol.md`), so the row paints bare in the admin
    /// amber — no sender brackets a sender-less notice. Empty notices
    /// stage no row, so they never consume a visible slot.
    pub fn push_lobby_announcement(
        &mut self,
        chat: LobbyChat,
        message: &Announcement,
    ) -> &mut Self {
        if message.content.is_empty() {
            return self;
        }
        let row = ChannelChatMessage::announcement(&message.content);
        self.lobby_panel_mut(chat).push_row(row);
        self
    }

    /// Append one rank notice (2029) to `chat`'s history, unless the
    /// packet carries no rank: the exe drops the packet when the
    /// invalid word is nonzero (`docs/net-protocol.md`). The row paints
    /// the pinned notice-board sentence (`rank_notice_board_text` field
    /// order nickname/title/rank, `docs/gifts.md`) in the admin amber;
    /// the song title resolves through the staged catalog, `M{id}`
    /// while it names no such song.
    pub fn push_lobby_rank_notification(
        &mut self,
        chat: LobbyChat,
        message: &RankNotification,
    ) -> &mut Self {
        if message.invalid {
            return self;
        }
        let song_title = self.rank_notice_song_title(message.music_id);
        let row = ChannelChatMessage::rank(&message.nickname, &song_title, message.ranking);
        self.lobby_panel_mut(chat).push_row(row);
        self
    }

    /// Append one player chat line (2013) to the channel history.
    pub fn push_channel_user_message(&mut self, message: &MainRoomUserMessage) -> &mut Self {
        self.push_lobby_user_message(LobbyChat::Channel, message)
    }

    /// Append one administrator chat line (2014) to the channel history.
    pub fn push_channel_admin_message(&mut self, message: &MainRoomAdminMessage) -> &mut Self {
        self.push_lobby_admin_message(LobbyChat::Channel, message)
    }

    /// Append one incoming whisper (2019) to the channel history.
    pub fn push_channel_whisper(&mut self, message: &ReceiveWhisper, recipient: &str) -> &mut Self {
        self.push_lobby_whisper(LobbyChat::Channel, message, recipient)
    }

    /// Append one lobby announcement (2016) to the channel history.
    pub fn push_channel_announcement(&mut self, message: &Announcement) -> &mut Self {
        self.push_lobby_announcement(LobbyChat::Channel, message)
    }

    /// Append one rank notice (2029) to the channel history.
    pub fn push_channel_rank_notification(&mut self, message: &RankNotification) -> &mut Self {
        self.push_lobby_rank_notification(LobbyChat::Channel, message)
    }

    /// Mutable panel behind [`Self::lobby_chat`].
    fn lobby_panel_mut(&mut self, chat: LobbyChat) -> &mut LobbyChatPanel {
        match chat {
            LobbyChat::Channel => &mut self.channel_chat,
            LobbyChat::Room => &mut self.room_chat,
        }
    }

    /// Song title of one 2029 rank notice: the staged catalog's title,
    /// `M{id}` while it names no such song.
    fn rank_notice_song_title(&self, music_id: i32) -> String {
        match u16::try_from(music_id)
            .ok()
            .and_then(|id| self.catalog_song_title(id))
        {
            Some(title) => title.to_owned(),
            None => format!("M{music_id}"),
        }
    }

    /// Live channel chat rows, oldest first (empty until a
    /// 2013/2014/2016/2019/2029 packet arrives).
    #[must_use]
    pub fn channel_chat(&self) -> &[ChannelChatMessage] {
        self.channel_chat.history()
    }

    /// Live room-lobby chat rows, oldest first (the same main-room
    /// feed as [`Self::channel_chat`], painted in the room panel).
    #[must_use]
    pub fn room_chat(&self) -> &[ChannelChatMessage] {
        self.room_chat.history()
    }

    /// Last staged room chrome control the router answered without an
    /// evidenced target (tickets #32/#34): proves the click was
    /// recorded instead of dropped.
    #[must_use]
    pub fn room_control_noop(&self) -> Option<RoomControlNoop> {
        self.last_room_control_noop
    }

    /// Stage the unsent input line (the 3011 source on send).
    /// Overlong lines truncate to [`MAX_CHAT_INPUT_CHARS`] characters.
    pub fn set_chat_input(&mut self, line: &str) -> &mut Self {
        self.chat_input = line.chars().take(MAX_CHAT_INPUT_CHARS).collect();
        self
    }

    /// Type one character into the waiting draft. Only the Waiting
    /// scene accepts typing; control characters never enter the line
    /// and the draft stops at [`MAX_CHAT_INPUT_CHARS`] characters.
    pub fn push_chat_char(&mut self, typed: char) {
        if self.flow.current() != Screen::Waiting || typed.is_control() {
            return;
        }
        if self.chat_input.chars().count() >= MAX_CHAT_INPUT_CHARS {
            return;
        }
        self.chat_input.push(typed);
    }

    /// Type one character into the lobby chat draft the on-screen
    /// scene owns: the Channel panel or the Room panel (every other
    /// scene accepts no lobby typing). While a room dialog is open it
    /// owns the keyboard, so the character feeds the dialog form
    /// instead (ticket #37).
    pub fn push_lobby_char(&mut self, typed: char) {
        if self.room_dialog.is_some() {
            self.push_room_dialog_char(typed);
            return;
        }
        if let Some(chat) = LobbyChat::of_screen(self.flow.current()) {
            self.lobby_panel_mut(chat).type_char(typed);
        }
    }

    /// Erase the last draft character (Backspace). Only the Waiting
    /// scene accepts it; an empty draft stays empty (`String::pop`
    /// removes one whole character, never half of a multibyte one).
    pub fn delete_chat_char(&mut self) {
        if self.flow.current() != Screen::Waiting {
            return;
        }
        self.chat_input.pop();
    }

    /// Erase the last character of the lobby chat draft the on-screen
    /// scene owns (Backspace; an empty draft stays empty). The open
    /// room dialog owns Backspace first.
    pub fn delete_lobby_char(&mut self) {
        if self.room_dialog.is_some() {
            self.delete_room_dialog_char();
            return;
        }
        if let Some(chat) = LobbyChat::of_screen(self.flow.current()) {
            self.lobby_panel_mut(chat).erase_char();
        }
    }
    #[must_use]
    pub fn chat_input(&self) -> &str {
        &self.chat_input
    }

    /// Outgoing chat packet for the draft (`None` while the input
    /// line is empty, so no empty send is invented). Callers send
    /// the returned body, then [`Self::clear_chat_input`].
    #[must_use]
    pub fn chat_send_message(&self) -> Option<SendWaitingMessage> {
        if self.chat_input.is_empty() {
            return None;
        }
        Some(SendWaitingMessage {
            content: self.chat_input.as_bytes().to_vec(),
        })
    }

    /// Forget the unsent input line.
    pub fn clear_chat_input(&mut self) -> &mut Self {
        self.chat_input.clear();
        self
    }

    /// Take the 3011 body for the draft and clear the input line so
    /// one Enter sends once (`None` while the draft is empty). The
    /// online session transmits the packet (see `recomp2jam::session`);
    /// the shell owns no transport, so offline takes still drop it here.
    pub fn take_chat_message(&mut self) -> Option<SendWaitingMessage> {
        let message = self.chat_send_message()?;
        self.clear_chat_input();
        Some(message)
    }

    /// Stage one lobby chat draft (the 2012 source on send). Overlong
    /// lines truncate to [`MAX_CHAT_INPUT_CHARS`] characters.
    pub fn set_lobby_input(&mut self, chat: LobbyChat, line: &str) -> &mut Self {
        self.lobby_panel_mut(chat).set_draft(line);
        self
    }

    /// Outgoing 2012 body for one lobby chat draft (`None` while the
    /// draft is empty, so no empty send is invented). Callers send the
    /// returned body, then [`Self::clear_lobby_input`].
    #[must_use]
    pub fn lobby_send_message(&self, chat: LobbyChat) -> Option<SendMainRoomMessage> {
        self.lobby_chat(chat).send_message()
    }

    /// Outgoing 2017 body for one lobby chat draft to `recipient`
    /// (`None` while the draft or the recipient is empty). The draft
    /// carries the whisper text; the recipient rides beside it (the
    /// wire 2017 shape).
    #[must_use]
    pub fn lobby_whisper_message(&self, chat: LobbyChat, recipient: &str) -> Option<SendWhisper> {
        self.lobby_chat(chat).whisper_message(recipient)
    }

    /// Forget one lobby chat draft.
    pub fn clear_lobby_input(&mut self, chat: LobbyChat) -> &mut Self {
        self.lobby_panel_mut(chat).clear_draft();
        self
    }

    /// Stage the unsent Channel input line (the 2012 source on send).
    pub fn set_channel_input(&mut self, line: &str) -> &mut Self {
        self.set_lobby_input(LobbyChat::Channel, line)
    }

    /// Live Channel input line.
    #[must_use]
    pub fn channel_input(&self) -> &str {
        self.lobby_chat(LobbyChat::Channel).draft()
    }

    /// Outgoing 2012 body for the Channel draft.
    #[must_use]
    pub fn channel_send_message(&self) -> Option<SendMainRoomMessage> {
        self.lobby_send_message(LobbyChat::Channel)
    }

    /// Outgoing 2017 body for the Channel draft to `recipient`.
    #[must_use]
    pub fn channel_whisper_message(&self, recipient: &str) -> Option<SendWhisper> {
        self.lobby_whisper_message(LobbyChat::Channel, recipient)
    }

    /// Forget the unsent Channel input line.
    pub fn clear_channel_input(&mut self) -> &mut Self {
        self.clear_lobby_input(LobbyChat::Channel)
    }

    /// Take the 2012 body for the Channel draft and clear the input
    /// line so one Enter sends once (`None` while the draft is
    /// empty). The online session transmits the packet (see
    /// `recomp2jam::session`); the shell owns no transport, so
    /// offline takes still drop it here.
    pub fn take_channel_message(&mut self) -> Option<SendMainRoomMessage> {
        let message = self.channel_send_message()?;
        self.clear_channel_input();
        Some(message)
    }

    /// Take the 2017 body for the Channel draft to `recipient` and
    /// clear the input line so one action sends once (`None` while
    /// the draft or the recipient is empty). The recipient becomes
    /// the `/r` peer, like an incoming whisper does.
    pub fn take_channel_whisper(&mut self, recipient: &str) -> Option<SendWhisper> {
        let message = self.channel_whisper_message(recipient)?;
        self.clear_channel_input();
        self.remember_whisper_peer(recipient);
        Some(message)
    }

    /// Outgoing whisper packet for the waiting draft to `recipient`
    /// (`None` while the draft or the recipient is empty, so no empty
    /// send is invented). The draft carries the whisper text; the
    /// recipient rides beside it (the wire 2017 shape, like the
    /// Channel whisper — the exe sends 2017 from the waiting room too,
    /// files `0xb8ffc`/`0xb9369`).
    #[must_use]
    pub fn chat_whisper_message(&self, recipient: &str) -> Option<SendWhisper> {
        if self.chat_input.is_empty() || recipient.is_empty() {
            return None;
        }
        Some(SendWhisper {
            recipient: recipient.to_owned(),
            content: self.chat_input.clone(),
        })
    }

    /// Take the 2017 body for the waiting draft to `recipient` and
    /// clear the input line so one action sends once (`None` while
    /// the draft or the recipient is empty). The recipient becomes
    /// the `/r` peer.
    pub fn take_chat_whisper(&mut self, recipient: &str) -> Option<SendWhisper> {
        let message = self.chat_whisper_message(recipient)?;
        self.clear_chat_input();
        self.remember_whisper_peer(recipient);
        Some(message)
    }

    /// Last whisper peer behind `/r` (`None` until a whisper flows
    /// either way).
    #[must_use]
    pub fn last_whisper_peer(&self) -> Option<&str> {
        self.last_whisper_peer.as_deref()
    }

    /// Remember one whisper peer for `/r`: the newest incoming 2019
    /// sender or outgoing 2017 recipient wins. Empty names never
    /// become the peer (a reply to nobody falls back to chat).
    pub fn remember_whisper_peer(&mut self, peer: &str) -> &mut Self {
        if !peer.is_empty() {
            self.last_whisper_peer = Some(peer.to_owned());
        }
        self
    }

    /// Peek one lobby chat draft through the slash grammar (the exe
    /// lobby parser beside the 2012 send at file `0x8fa1e` and the 2017
    /// send at file `0x8fa02`).
    #[must_use]
    pub fn lobby_slash_command(&self, chat: LobbyChat) -> SlashCommand {
        parse_slash_draft(
            self.lobby_chat(chat).draft(),
            self.last_whisper_peer.as_deref(),
        )
    }

    /// Peek the Channel draft through the slash grammar.
    #[must_use]
    pub fn channel_slash_command(&self) -> SlashCommand {
        self.lobby_slash_command(LobbyChat::Channel)
    }

    /// Peek the waiting draft through the same slash grammar (the exe
    /// sends 2017 from the waiting room too, files `0xb8ffc`/`0xb9369`).
    #[must_use]
    pub fn chat_slash_command(&self) -> SlashCommand {
        parse_slash_draft(&self.chat_input, self.last_whisper_peer.as_deref())
    }

    /// Take one lobby chat draft through the slash grammar and clear
    /// the input line so one Enter acts once (`Empty` leaves the empty
    /// draft alone). A whisper take remembers its recipient for `/r`.
    pub fn take_lobby_slash(&mut self, chat: LobbyChat) -> SlashCommand {
        let command = self.lobby_slash_command(chat);
        if command != SlashCommand::Empty {
            self.clear_lobby_input(chat);
        }
        if let SlashCommand::Whisper { ref recipient, .. } = command {
            self.remember_whisper_peer(recipient);
        }
        command
    }

    /// Take the Channel draft through the slash grammar.
    pub fn take_channel_slash(&mut self) -> SlashCommand {
        self.take_lobby_slash(LobbyChat::Channel)
    }

    /// Take the Room-lobby draft through the slash grammar.
    pub fn take_room_slash(&mut self) -> SlashCommand {
        self.take_lobby_slash(LobbyChat::Room)
    }

    /// Take the waiting draft through the slash grammar and clear the
    /// input line so one Enter acts once (`Empty` leaves the empty
    /// draft alone). A whisper take remembers its recipient for `/r`.
    pub fn take_chat_slash(&mut self) -> SlashCommand {
        let command = self.chat_slash_command();
        if command != SlashCommand::Empty {
            self.clear_chat_input();
        }
        if let SlashCommand::Whisper { ref recipient, .. } = command {
            self.remember_whisper_peer(recipient);
        }
        command
    }

    /// Append one chat row and bound the history to
    /// [`MAX_WAITING_CHAT_ROWS`] (oldest rows drop first).
    fn push_chat_row(&mut self, sender: &str, text: &str, admin: bool) {
        self.waiting_chat.push(WaitingChatMessage {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin,
        });
        self.truncate_waiting_chat();
    }

    /// Drop the oldest rows past the history cap.
    fn truncate_waiting_chat(&mut self) {
        let overflow = self
            .waiting_chat
            .len()
            .saturating_sub(MAX_WAITING_CHAT_ROWS);
        self.waiting_chat.drain(..overflow);
    }

    /// Room arena stage the Playing `_` sprites resolve through: the
    /// default until a joined room names one, the room's arena word
    /// afterwards (`docs/stage-bga.md` global `0x7789dc`). Loaders
    /// build `Playing_BG<stage>.ojs` and the stage-keyed kits from it.
    #[must_use]
    pub fn arena_stage(&self) -> u16 {
        self.arena_stage
    }

    /// Wire key mode of the joined room (7-key while none is joined):
    /// the Playing art families split members by it
    /// (`avatar::PlayingArtMode`).
    #[must_use]
    pub fn room_key_mode(&self) -> u8 {
        self.waiting
            .as_ref()
            .map_or(o2jam_net::KeyMode::SevenKeys as u8, |room| room.key_mode)
    }

    /// Whether the joined room plays team/versus (`Live`); solo
    /// (`Normal`) otherwise, including while no room is joined.
    #[must_use]
    pub fn room_team_play(&self) -> bool {
        self.waiting
            .as_ref()
            .and_then(|room| o2jam_net::GameMode::from_u8(room.game_mode).ok())
            .is_some_and(|mode| mode == o2jam_net::GameMode::Live)
    }

    /// Self room-master state: the joined room's seat id names our
    /// seat, and seat 0 is the creator — the 4043 header derives the
    /// local `+0x30` flag as 1 exactly when the requesting member id
    /// is 0 (files `0x159bdf`/`0x159c1b`/`0x159c4d`, `docs/relay.md`
    /// §10). The waiting setup swaps START/READY on this flag (file
    /// `0xae38e`: the master arm enables `START 0x16202012` and hides
    /// `READY 0x16202013`; the non-master arm at file `0xae805`
    /// reverses them), and the `MUSELECT 0x16200003` click handler
    /// drops non-master clicks (file `0xb2622`, `je 0xb268d`).
    #[must_use]
    pub fn is_room_master(&self) -> bool {
        self.waiting
            .as_ref()
            .is_some_and(|room| room.member_id == 0)
    }

    /// Every occupied peer seat ready: the `START 0x16202012` click
    /// path runs this check (file `0xb5560`) — it skips our own seat
    /// (member id at `0x77cd04`), requires each occupied seat
    /// (`[i + 0x778a16] == 1`) to carry a nonzero ready byte
    /// (`[i * 0x2190 + 0x77eec2]`, slot base `0x77ee90` + `0x32`),
    /// and returns -1 while any occupied peer is unready, so START
    /// errors instead of emitting. Vacant/locked seats (`None`)
    /// never block.
    #[must_use]
    pub fn all_peers_ready(&self) -> bool {
        let Some(room) = self.waiting.as_ref() else {
            return false;
        };
        room.slots.iter().all(|slot| {
            slot.member
                .as_ref()
                .is_none_or(|member| slot.index == room.member_id || member.ready)
        })
    }

    /// The master may start only with every peer ready; non-masters
    /// never start — they ready up (4008, owned by the session
    /// bridge), so their Confirm stays in Waiting.
    #[must_use]
    pub fn can_start_game(&self) -> bool {
        self.is_room_master() && self.all_peers_ready()
    }

    /// Run the waiting Confirm gate shared by Enter and clicks:
    /// slash-free drafts advance offline walkers and ready masters;
    /// guests and masters with unready peers stay put (see
    /// [`Self::can_start_game`]).
    fn confirm_waiting(&mut self) {
        if self.take_chat_slash() == SlashCommand::Empty
            && (self.waiting.is_none() || self.can_start_game())
        {
            self.enter(Screen::Loading);
        }
    }

    /// Advance a joined guest to Loading on the 4011 start broadcast.
    /// The `START 0x16202012` gate passes masters only (file
    /// `0xb5560`), so a guest Confirm can never satisfy
    /// [`Self::can_start_game`]; the session calls this on 4011
    /// instead of Confirm. Masters never advance here — they keep
    /// the readiness rule through Confirm — and neither do the
    /// offline walk (no joined room), other screens, or a pending
    /// draft (peek-only, so typed text is never eaten). `true`
    /// entered Loading.
    pub fn force_advance_to_loading(&mut self) -> bool {
        if self.flow.current() != Screen::Waiting
            || self.waiting.is_none()
            || self.is_room_master()
            || self.chat_slash_command() != SlashCommand::Empty
        {
            return false;
        }
        self.enter(Screen::Loading);
        true
    }

    /// Native pointer position, when the pointer has moved this run.
    #[must_use]
    pub fn pointer(&self) -> Option<(i32, i32)> {
        self.pointer
    }

    /// Channel row under the pointer, mapped back through the
    /// stored list top (`None` off the rows, off the Channel
    /// screen, under an open side overlay, on the X/scroll chrome, on a
    /// row body, or while a slash draft owns the click).
    #[must_use]
    pub fn hovered_channel_row(&self) -> Option<usize> {
        if self.flow.current() != Screen::Channel || self.side.is_some() {
            return None;
        }
        let (x, y) = self.pointer?;
        let (top, shown) = channel_viewport_from_top(self.channel_top, self.channels.len());
        let slot = channel_row_hit(x, y, shown)?;
        (top + slot < self.channels.len()).then_some(top + slot)
    }

    /// Highlighted channel row (the keyboard Confirm target;
    /// `None` off the Channel screen or with no listed rows). The
    /// session bridge logs into it before advancing (ticket #30).
    #[must_use]
    pub fn highlighted_channel(&self) -> Option<ChannelState> {
        if self.flow.current() != Screen::Channel {
            return None;
        }
        self.channels.get(self.highlight).copied()
    }

    /// Channel row whose enter face covers (`x`, `y`) (the mouse
    /// entry target; `None` off the Channel screen, on the X/scroll
    /// chrome, on a row body, or while a slash draft owns the click).
    /// Mirrors [`Self::click_channel_row`]'s enter arm so the session
    /// bridge can exchange the login before forwarding the press
    /// (ticket #30).
    #[must_use]
    pub fn channel_enter_at(&self, x: i32, y: i32) -> Option<ChannelState> {
        if self.flow.current() != Screen::Channel {
            return None;
        }
        if channel_dialog_x_hit(x, y) || channel_scroll_hit(x, y).is_some() {
            return None;
        }
        let (top, shown) = channel_viewport_from_top(self.channel_top, self.channels.len());
        let slot = channel_row_hit(x, y, shown)?;
        let index = top + slot;
        if index >= self.channels.len() {
            return None;
        }
        if !channel_face_hit(x, y, slot) {
            return None;
        }
        // A slash draft consumes the click instead of entering
        // (the click path takes it); peek here so the query stays
        // side-effect free.
        if self.channel_slash_command() != SlashCommand::Empty {
            return None;
        }
        self.channels.get(index).copied()
    }

    /// Board control under the pointer, through the same staged
    /// resolution the click path rides ([`Self::click_planet`])
    /// (`None` off the board, off the Planet phase, or while a modal
    /// owns the board). Ticket #26 consumes ticket #33's pointer
    /// state here — no second hover store; motion alone arms the
    /// painter's highlight, exactly like the channel select bar.
    #[must_use]
    pub fn hovered_planet_action(&self) -> Option<PlanetAction> {
        if self.flow.current() != Screen::Planet || self.planet_modal().is_some() {
            return None;
        }
        let (x, y) = self.pointer?;
        let placements: &[ScenePlacement] = self
            .scene_art(Screen::Planet)
            .map_or(&[], |art| &art.placements);
        planet_action_at(placements, x, y)
    }

    /// Channel dialog click: the bnd-pinned X button rect
    /// (541,151)-(589,189) closes the dialog back to the bare planet
    /// board (verified native close click (577,171); `docs/planet.md`),
    /// otherwise a click inside a pinned row rect highlights the
    /// entry and advances like Confirm while the draft is action-free
    /// (slash drafts consume the click instead of starting).
    /// Step the channel highlight one row toward `scroll`, clamped at
    /// the ends: the Up/Down-key path (the scroll arrows no longer
    /// ride this — ticket #130 pins them to the list top instead).
    /// The top re-seats under the highlight so the key cursor stays
    /// visible; the exe key rule itself stays OPEN.
    fn step_channel(&mut self, scroll: ChannelScroll) {
        match scroll {
            ChannelScroll::Up => {
                self.highlight = self.highlight.saturating_sub(1);
            }
            ChannelScroll::Down => {
                let last = self.channels.len().saturating_sub(1);
                self.highlight = (self.highlight + 1).min(last);
            }
        }
        self.channel_top = channel_viewport(self.highlight, self.channels.len()).0;
    }

    /// Step the channel list-top offset one row toward `scroll`,
    /// clamped to `[0, len - shown]`: the scroll-arrow path. The exe
    /// scrollbar owns its position (`this+0xb8`, clamped
    /// `[0, max-page]` in `0x50ec20`, file `0x10ec20`) and the arrow
    /// zones (mouse arm `0x50e8d0`, files `0x10ea1a`/`0x10ea6b`)
    /// step it by `∓1` through `0x50ebf0` (file `0x10ebf0`),
    /// forwarding each delta to the list top (`CDChannelInfo+0x184`,
    /// handlers `0x4749a0`/`0x4749f0`); the highlight is never
    /// touched on that path. Observed with GNU objdump 2.47 slices
    /// plus Python 3.14.10 byte scans over the read-only 7.55 copy.
    fn step_channel_list(&mut self, scroll: ChannelScroll) {
        let max_top = self.channels.len().saturating_sub(CHANNEL_VISIBLE_ROWS);
        match scroll {
            ChannelScroll::Up => {
                self.channel_top = self.channel_top.saturating_sub(1);
            }
            ChannelScroll::Down => {
                self.channel_top = (self.channel_top + 1).min(max_top);
            }
        }
    }

    fn click_channel_row(&mut self, x: i32, y: i32) {
        if channel_dialog_x_hit(x, y) {
            self.enter(Screen::Planet);
            return;
        }
        // Scroll-column arrows scroll the list top (`step_channel_list`);
        // the bar body between them answers nothing to a click.
        if let Some(scroll) = channel_scroll_hit(x, y) {
            self.step_channel_list(scroll);
            return;
        }
        // Viewport-relative bands: the visible slot maps back through
        // the stored list top (see `channel_viewport_from_top`).
        let (top, shown) = channel_viewport_from_top(self.channel_top, self.channels.len());
        let Some(slot) = channel_row_hit(x, y, shown) else {
            return;
        };
        let index = top + slot;
        self.highlight = index;
        // Only the Korean-text enter face enters Room (ticket #56);
        // a body click moves the highlight and leaves the dialog up.
        if channel_face_hit(x, y, slot) && self.take_channel_slash() == SlashCommand::Empty {
            self.enter(Screen::Room);
        }
    }

    /// Room click: a staged chrome button first ([`Self::click_room_control`]),
    /// then the row hit test — staged panels (verbatim `.bnd` rects in
    /// file order) ahead of the legacy row bands. Either row path
    /// highlights the row and advances like Confirm.
    fn click_room_row(&mut self, x: i32, y: i32) {
        if self.click_room_control(x, y) {
            return;
        }
        let Some(index) = self.room_index_at(x, y) else {
            return;
        };
        self.room_highlight = index;
        // Locked rooms gate through the password dialog (lifecycle §7:
        // `DIALOG_INPUTPASSWORD` `CDInputPassword`); unlocked rows join
        // straight to Waiting.
        if let Some(room) = self.rooms.get(index) {
            if room.has_password {
                if self.password_art.is_some() {
                    self.open_password_dialog(room.number);
                } else {
                    self.last_room_control_noop = Some(RoomControlNoop {
                        id: "LOCKED_ROOM_ROW",
                        next_step: DIALOG_STAGE_STEP,
                    });
                }
                return;
            }
        }
        self.enter(Screen::Waiting);
    }

    /// Answer the staged room chrome control under the click (tickets
    /// #32/#34): the behaviour comes from [`ROOM_CONTROLS`], and a
    /// control with no evidenced target still records its press instead
    /// of dropping it silently. `true` consumed the click, so the room
    /// row hit test never sees a button press.
    fn click_room_control(&mut self, x: i32, y: i32) -> bool {
        let Some(control) = self.room_control_at(x, y) else {
            return false;
        };
        match control.action {
            // Leave for the channel scene (lifecycle §7: the same edge
            // the Back key rides).
            RoomControlAction::Back => {
                self.enter(Screen::Channel);
            }
            // Open the create-room dialog (lifecycle §7:
            // `DIALOG_CREATEROOM` `CDCreateRoom`); without the staged
            // sheet the modal would own input it cannot paint, so the
            // click records a noop and the lobby stays usable.
            RoomControlAction::Create => {
                if self.create_room_art.is_some() {
                    self.open_create_room_dialog();
                } else {
                    self.last_room_control_noop = Some(RoomControlNoop {
                        id: "IDC_BUTTON_SROOM_CREATE",
                        next_step: DIALOG_STAGE_STEP,
                    });
                }
            }
            // Open the options dialog over the lobby (the same
            // `Dialog_Option_*` sheet the Planet gear mounts:
            // EXIT/Escape closes it back onto Room, the flow never
            // moves).
            RoomControlAction::Options => self.planet_board.open_modal(PlanetModal::Options),
            // Visit a side screen the tree already ships; the lobby
            // gate is `can_visit_side`.
            RoomControlAction::Side(side) => {
                self.visit_side(side);
            }
            // No evidenced target: record the press and its next
            // evidence step — never a silent drop.
            RoomControlAction::Unwired { next_step } => {
                tracing::debug!(
                    control = control.id,
                    code = format_args!("0x{:08X}", control.code),
                    next_step,
                    "room control has no evidenced behaviour yet; press recorded"
                );
                self.last_room_control_noop = Some(RoomControlNoop {
                    id: control.id,
                    next_step,
                });
            }
        }
        true
    }

    /// Staged room chrome control under the click, if the table names
    /// its code. Geometry comes from the parsed staged rect only (owner
    /// rule `f38f3`).
    ///
    /// Not listed: the row templates (`IDC_BUTTON_SROOM_EXBUTTON1..6`,
    /// the `SET` clone parents) whose rects ARE the room rows and
    /// therefore fall through to the row hit test — EXE-static by
    /// negative evidence (0 changed px on all six panels,
    /// `docs/hover.md`), so the hover resolver below never names
    /// them either.
    fn room_control_at(&self, x: i32, y: i32) -> Option<&'static RoomControl> {
        let placements = &self.scene_art(Screen::Room)?.placements;
        ROOM_CONTROLS
            .iter()
            .find(|control| staged_code_hit(placements, control.code, x, y))
    }

    /// Staged room chrome control under the pointer (`None` off every
    /// control, off the Room Scene, or while a dialog, modal, or side
    /// Scene covers the lobby — clicks route to those layers first,
    /// so hover follows). The shared hover source ticket #26 consumes
    /// for the planet board is the pointer itself (the snapshot pointer);
    /// this resolver maps it onto the room chrome the same way clicks
    /// resolve (`staged_code_hit` on the parsed rects). Row panels
    /// (`EXBUTTON1..6`) are not chrome controls: the pointer over a
    /// panel resolves to nothing and repaints nothing — EXE-static by
    /// negative evidence (`docs/hover.md`: 0 changed px, no
    /// `Room_MOver.ojs` overlay in either state).
    fn hovered_room_control(&self) -> Option<&'static RoomControl> {
        if self.flow.current() != Screen::Room {
            return None;
        }
        if self.room_dialog.is_some() || self.planet_modal().is_some() || self.side.is_some() {
            return None;
        }
        let (x, y) = self.pointer?;
        let control = self.room_control_at(x, y)?;
        // Active filter/tab already shows frame 1 and never changes on
        // hover (EXE-pinned no-change, `docs/hover.md`), so hovering
        // one resolves to nothing: the hover must not repaint frame 1
        // with frame 0. Filter/tab switching is unwired, so the
        // default-active pair below never moves in the shell.
        (!ROOM_ACTIVE_CHROME_CODES.contains(&control.code)).then_some(control)
    }

    /// Room-row index under a click, staged panels first (see
    /// [`Self::click_room_row`]).
    fn room_index_at(&self, x: i32, y: i32) -> Option<usize> {
        let staged = self.room_panel_click(x, y);
        let band = || list_row_hit(CLICK_W, CLICK_H, self.rooms.len(), x, y);
        staged
            .or_else(band)
            .filter(|index| *index < self.rooms.len())
    }

    /// Room number under a click (`None` while no listed room covers
    /// the point). The session bridge joins it before advancing, so a
    /// click sends what Confirm sends.
    #[must_use]
    pub fn room_number_at(&self, x: i32, y: i32) -> Option<i32> {
        self.room_index_at(x, y)
            .and_then(|index| self.rooms.get(index))
            .map(|room| room.number)
    }

    /// Staged room-panel click: file-order panel index covering
    /// (`x`, `y`) (`None` while no bound panel covers the point).
    fn room_panel_click(&self, x: i32, y: i32) -> Option<usize> {
        let placements = &self.scene_art(Screen::Room)?.placements;
        if !has_room_panels(placements) {
            return None;
        }
        room_panel_hit(placements, x, y)
    }

    /// `SetRoomMusic` body behind one dialog row: the row's song id
    /// plus its difficulty slot (row slots repeat Easy/Normal/Hard,
    /// the `Difficulty` code order), at the surviving speed. `None`
    /// when the row, the slot code, the song id, or the speed has no
    /// source — nothing is invented.
    fn select_row_pick(&self, index: usize) -> Option<SetRoomMusic> {
        let row = select_music_rows(&self.catalog_songs).get(index).copied()?;
        let difficulty = u8::try_from(row.slot).ok()?;
        Difficulty::from_code(difficulty)?;
        Some(SetRoomMusic {
            music_id: u16::try_from(row.song.song_id).ok()?,
            difficulty,
            speed: self.staged_pick_speed()?,
        })
    }
    /// Speed surviving a dialog pick: the staged selection's speed
    /// first, else the joined room's. `None` with no source.
    fn staged_pick_speed(&self) -> Option<u8> {
        self.selected_music
            .map(|selection| selection.speed)
            .or_else(|| self.waiting.as_ref().map(|room| room.speed))
    }
    /// Commit one dialog row as the room song: the room state takes
    /// the pick at once (title/speed captions repaint) and the 4000
    /// body queues for the session bridge. `false` stages nothing
    /// (no joined room to commit to). The no-catalog path stays
    /// display-only — empty rows never invent a song.
    fn pick_select_music_row(&mut self, index: usize) -> bool {
        if self.waiting.is_none() {
            return false;
        }
        let Some(pick) = self.select_row_pick(index) else {
            return false;
        };
        self.set_selected_music(pick);
        self.outbox.pending_music_send = Some(pick);
        true
    }
    /// Song-dialog click: a search-table row hit commits the row's
    /// song through [`Self::pick_select_music_row`]. Only the room
    /// master sees the dialog (paint gates it) and only masters pick
    /// — the `MUSELECT 0x16200003` click handler drops non-master
    /// clicks (file `0xb2622`). Anything else falls through to the
    /// seat buttons below: the dialog chrome (filters, pager,
    /// search, chosung, OK) has no evidenced click behavior, so it
    /// stays display-only and BACK stays reachable. `true` consumed.
    fn click_select_music(&mut self, x: i32, y: i32) -> bool {
        if !self.is_room_master() || self.select_music_art.is_none() {
            return false;
        }
        let rows = select_music_rows(&self.catalog_songs);
        let Some(index) = select_row_hit(x, y, rows.len()) else {
            return false;
        };
        self.pick_select_music_row(index)
    }
    /// Whether a click hits a staged Waiting confirm button
    /// (START/READY ids only; art-less clicks land nowhere — no pinned
    /// fallback geometry names these buttons). The session bridge runs
    /// its networked Confirm gate on a hit instead of the local one.
    #[must_use]
    pub fn waiting_confirm_hit(&self, x: i32, y: i32) -> bool {
        let Some(art) = self.scene_art(Screen::Waiting) else {
            return false;
        };
        WAITING_CONFIRM_IDS
            .iter()
            .any(|id| staged_rect_hit(&art.placements, id, x, y))
    }

    /// Waiting button click: staged START/READY runs the Confirm
    /// gate, staged BACK leaves for rooms. Art-less clicks land
    /// nowhere — no pinned fallback geometry names these buttons.
    fn click_waiting_button(&mut self, x: i32, y: i32) {
        let Some(art) = self.scene_art(Screen::Waiting) else {
            return;
        };
        let placements = &art.placements;
        // Team buttons first: the toggle-group map (`FUN_0048c810`,
        // `waiting_team_for_code`) lights the pressed team and queues
        // the 4004+4008 send pair through `select_team`.
        if let Some(team) = waiting_team_at(placements, x, y) {
            self.select_team(team);
            return;
        }
        let back = WAITING_BACK_IDS
            .iter()
            .any(|id| staged_rect_hit(placements, id, x, y));
        let confirm = WAITING_CONFIRM_IDS
            .iter()
            .any(|id| staged_rect_hit(placements, id, x, y));
        if back {
            self.enter(Screen::Room);
        } else if confirm {
            self.confirm_waiting();
        }
    }

    /// Hit one result button: the staged `.bnd` rect verbatim where
    /// bound, the pinned `docs/cresult-anchors.md` rect otherwise.
    fn result_button_hit(&self, id: &str, fallback: &Rect, x: i32, y: i32) -> bool {
        match self.scene_art(Screen::Result) {
            Some(art) if art.placements.iter().any(|placed| placed.id == id) => {
                staged_rect_hit(&art.placements, id, x, y)
            }
            _ => rect_hit(fallback, x, y),
        }
    }

    /// Result button click: RETRY re-enters Waiting, BACK leaves
    /// for rooms — the keyboard Confirm/Back arms on mouse. Both
    /// presses queue their pinned relay body for the session bridge
    /// (4010 `0xfaa` / 4021 `0xfb5`, both empty).
    fn click_result_button(&mut self, x: i32, y: i32) {
        if self.result_button_hit(RESULT_RETRY_ID, &RESULT_RETRY_RECT, x, y) {
            self.outbox.pending_result_retry = true;
            self.enter(Screen::Waiting);
        } else if self.result_button_hit(RESULT_BACK_ID, &RESULT_BACK_RECT, x, y) {
            self.outbox.pending_result_back = true;
            self.enter(Screen::Room);
        }
    }

    /// Remember the host song choice (`SetRoomMusic` 4000 body);
    /// Waiting paints its title and difficulty panel, and the chosen
    /// chart's difficulty byte selects the playing life-bar deltas.
    /// Also resolves the album cover (see [`Self::resolve_song_cover`]).
    pub fn set_selected_music(&mut self, selection: SetRoomMusic) -> &mut Self {
        self.selected_music = Some(selection);
        self.apply_life_difficulty();
        self.resolve_song_cover();
        self
    }

    /// Stage a live loading status (real gate fractions only — see
    /// `recomp2jam::loading`; unknown stays `None`, never invented).
    /// The fraction is the gate phase ([`LOADING_GATE_PHASES`]):
    /// callers thread entry plus the staging/decrypt/decode/arm
    /// steps through `loading_phase_status` (or the armed full
    /// fraction straight from a completed chart stage), and reaching
    /// the armed fraction is the only way the gate phase reaches the
    /// arm — Confirm alone never advances it. A latched failure
    /// still holds the phase back.
    pub fn set_loading_status(&mut self, status: LoadingStatus) -> &mut Self {
        let phase = loading_phase_for_progress(status.progress);
        if !self.loading_gate_failed {
            self.loading_gate_phase = self.loading_gate_phase.max(phase);
        }
        self.loading_status = Some(status);
        self
    }

    /// Forget the walk state: Loading paints indeterminate again;
    /// the gate phase drops to entry and any latched failure clears,
    /// so the next song re-walks the gate from validation.
    pub fn clear_loading(&mut self) -> &mut Self {
        self.loading_status = None;
        self.loading_gate_phase = 0;
        self.loading_gate_failed = false;
        self
    }

    /// The pick the song-select staged (`None` until a music is
    /// picked): the loading gate validates it on the Loading edge.
    #[must_use]
    pub fn selected_music(&self) -> Option<SetRoomMusic> {
        self.selected_music
    }

    /// Whether an audio rig is attached (the gate requires one: the
    /// sample clock rides its backend).
    #[must_use]
    pub fn has_audio(&self) -> bool {
        self.audio.is_some()
    }

    /// Gate phase held by the walk (0 entry, 1 staging, 2 OJN
    /// decrypt, 3 Vorbis decode, 4 clock armed) — where the staged
    /// fractions currently stand. Stays at entry while nothing
    /// stages.
    #[must_use]
    pub fn loading_gate_phase(&self) -> usize {
        self.loading_gate_phase
    }

    /// Whether the gate has latched a failure (bad chart,
    /// undecryptable OJN, missing audio): the gate then holds
    /// Loading until a fresh [`Self::clear_loading`] re-walks it.
    #[must_use]
    pub fn loading_gate_failed(&self) -> bool {
        self.loading_gate_failed
    }

    /// Latch a loading failure (bad chart bytes, an OJN the decrypt
    /// rejects, a bank leg that decoded nothing): the gate holds
    /// Loading and never advances or arms on it, whatever fraction a
    /// later status stages. The caller clears it with
    /// [`Self::clear_loading`] when the song changes.
    pub fn fail_loading_gate(&mut self) {
        self.loading_gate_failed = true;
    }

    /// Select validation behind the Loading-to-Playing gate
    /// (lifecycle section 9, step 1: `0x5718b0`, ev `66e60d16...`):
    /// the picked song carries a nonzero id and a known difficulty
    /// slot (`Difficulty::from_code`; Rx maps to the Hard slice
    /// downstream, like the session's level map), and when the song
    /// table is staged the id must name one of its songs. No pick or
    /// no table match rejects before any file work matters.
    ///
    /// OPEN: with no catalog staged the table membership stays
    /// unverifiable (the offline/manual chart paths stage no table),
    /// so only the id/difficulty shape gates there — the table check
    /// engages once the catalog stages.
    #[must_use]
    pub fn validate_loading_select(&self) -> bool {
        let Some(selection) = self.selected_music.as_ref() else {
            return false;
        };
        if selection.music_id == 0 {
            return false;
        }
        if Difficulty::from_code(selection.difficulty).is_none() {
            return false;
        }
        if !self.catalog_songs.is_empty()
            && !self
                .catalog_songs
                .iter()
                .any(|song| song.song_id == u32::from(selection.music_id))
        {
            return false;
        }
        true
    }

    /// Whether the gate stands ready to arm the song clock and hand
    /// off to Playing: no latched failure, validation holds, the
    /// walk reached the armed phase (staging, OJN decrypt, and
    /// Vorbis init all staged before it), and an audio rig is
    /// attached — missing audio never arms, so the sample clock
    /// [`Self::now_ms`] always has a backend behind it. Confirm and
    /// the test driver hold in Loading while this is `false`; the
    /// scene stays honest.
    #[must_use]
    pub fn loading_gate_ready(&self) -> bool {
        !self.loading_gate_failed
            && self.loading_gate_phase >= LOADING_GATE_PHASES
            && self.validate_loading_select()
            && self.audio.is_some()
    }

    /// Live loading status, if a real gate walk staged one.
    #[must_use]
    pub fn loading_status(&self) -> Option<&LoadingStatus> {
        self.loading_status.as_ref()
    }

    /// Stage the walked chart's OJN cover into the loading scene under
    /// [`LOADING_COVER_SPRITE`]: the full-bleed song illustration the
    /// loading painter stretches first, replacing the genuine
    /// `MusicLoading.ojs` planet fallback (the exe mounts that overlay
    /// only when the song load returns 0, file `0xe47a0`). `true` while
    /// the bytes decode to a cover; any miss — non-chart bytes, unknown
    /// magic, undecodable stream — drops a previously staged cover, so
    /// the miss keeps the planet fallback. Archives stay read-only;
    /// nothing is invented.
    pub fn stage_loading_cover(&mut self, chart_bytes: &[u8]) -> bool {
        match ojn_cover_figure(chart_bytes) {
            Some(figure) => {
                self.scene_art[scene_index(Screen::Loading)]
                    .get_or_insert_with(SceneArt::default)
                    .figures
                    .insert(LOADING_COVER_SPRITE.to_owned(), figure);
                true
            }
            None => {
                self.clear_loading_cover();
                false
            }
        }
    }

    /// Whether the walked chart's OJN cover is staged for the loading
    /// gate (present means the chart load succeeded, so the painter
    /// skips the planet fallback).
    #[must_use]
    pub fn has_loading_cover(&self) -> bool {
        self.scene_art[scene_index(Screen::Loading)]
            .as_ref()
            .is_some_and(|art| art.figures.contains_key(LOADING_COVER_SPRITE))
    }

    /// Drop any staged loading cover so the next paint falls back to
    /// the planet overlay.
    fn clear_loading_cover(&mut self) {
        if let Some(art) = self.scene_art[scene_index(Screen::Loading)].as_mut() {
            art.figures.remove(LOADING_COVER_SPRITE);
        }
    }

    /// Seed the playing life bar from the selected chart's difficulty
    /// (Ex/Nx/Hx → Easy/Normal/Hard deltas per `FUN_00570f90`); codes
    /// without a verified arm (Rx, unknown) keep the current set. Runs
    /// on both setup points — chart load and song selection — so the
    /// flow order never matters.
    fn apply_life_difficulty(&mut self) {
        let Some(selection) = self.selected_music.as_ref() else {
            return;
        };
        self.playing.set_life_difficulty(selection.difficulty);
    }

    /// Stage the song catalog (`OJNList.dat` order, owned copies) for
    /// the select-music table and the chosen-song caption. Dead slots
    /// stay out; a missing catalog paints dialog art without rows.
    /// The BPM column stages alongside for the waiting `BPM` caption,
    /// and the container tokens for the album-cover path (see
    /// [`Self::resolve_song_cover`]).
    pub fn set_song_catalog(&mut self, entries: &[OjnListEntry<'_>]) -> &mut Self {
        self.catalog_songs = select_music_songs(entries);
        self.catalog_bpm = entries
            .iter()
            .filter(|entry| entry.is_song())
            .map(|entry| (entry.song_id, entry.bpm))
            .collect();
        self.catalog_chart_files = entries
            .iter()
            .filter(|entry| entry.is_song())
            .map(|entry| (entry.song_id, entry.ojm_file.clone()))
            .collect();
        self.resolve_song_cover();
        self
    }

    /// Load the selected song's album cover from the staged catalog
    /// plus the install's `Music` folder. Runs on both staging points —
    /// catalog and song selection — so the flow order never matters.
    /// The selection's id names a catalog record, whose container token
    /// (`o2ma757.ojm`) swaps to the chart file (`o2ma757.ojn`; the
    /// shipped tokens match their chart files 14/14) and the OJN cover
    /// section ([`o2jam_file::ojn_cover_jpeg`],
    /// [`o2jam_file::decode_ojn_cover`], docs/file-formats/ojn.md)
    /// decodes to the box image. Any miss — no selection, no catalog
    /// record, no install, undecodable cover — leaves `None` and the
    /// staged music-name box paints as-is; nothing is invented.
    fn resolve_song_cover(&mut self) {
        self.song_cover = None;
        self.clear_loading_cover();
        let Some(selection) = self.selected_music.as_ref() else {
            return;
        };
        let Some(token) = self.catalog_chart_files.get(&u32::from(selection.music_id)) else {
            return;
        };
        let Some(path) = chart_cover_path(token) else {
            return;
        };
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Ok(cover) = o2jam_file::ojn_cover_jpeg(&bytes) else {
            return;
        };
        let Ok(decoded) = o2jam_file::decode_ojn_cover(&cover) else {
            return;
        };
        self.song_cover = cover_image(&decoded);
        // The same walked bytes feed the loading scene: the cover the
        // gate walked is the illustration the loading painter stretches.
        self.stage_loading_cover(&bytes);
    }

    /// Stage real song-select dialog art (`DAILOG_SELECTMUSIC` placements
    /// plus figures loaded through `load_scene_figures`). Waiting layers
    /// it over the seats; empty art paints nothing extra.
    pub fn load_select_music_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.select_music_art = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Staged select-music catalog songs (catalog order, dead slots
    /// filtered out). Empty without catalog data.
    #[must_use]
    pub fn select_songs(&self) -> &[SelectMusicSong] {
        &self.catalog_songs
    }

    /// Played song's album cover, if it resolved from the walked
    /// chart's OJN cover section (see [`Self::resolve_song_cover`]).
    #[must_use]
    pub fn song_cover(&self) -> Option<&Rgb555Image> {
        self.song_cover.as_ref()
    }

    /// Catalog title for one song id, if staged.
    fn catalog_song_title(&self, music_id: u16) -> Option<&str> {
        let wanted = u32::from(music_id);
        self.catalog_songs
            .iter()
            .find(|song| song.song_id == wanted)
            .map(|song| song.title.as_str())
    }

    /// Catalog title for the chosen song, if staged.
    fn selected_song_title(&self) -> Option<&str> {
        let selection = self.selected_music.as_ref()?;
        self.catalog_song_title(selection.music_id)
    }

    /// Catalog title for the joined room's song, if staged: the
    /// `MUSICNAME` caption source (room-fill string flow,
    /// `docs/channel-fill.md`); `None` paints the bare `M{id}`
    /// fallback.
    fn room_song_title(&self) -> Option<&str> {
        let room = self.waiting.as_ref()?;
        self.catalog_song_title(room.music_id)
    }

    /// Catalog BPM for one song id, if a live song record stages it.
    fn catalog_song_bpm(&self, music_id: u16) -> Option<f32> {
        self.catalog_bpm.get(&u32::from(music_id)).copied()
    }

    /// Catalog BPM for the joined room's song, if staged: the `BPM`
    /// caption source (`OJNList.dat` row +20 column); `None` paints
    /// the pinned `-0-` mismatch default.
    fn room_song_bpm(&self) -> Option<f32> {
        let room = self.waiting.as_ref()?;
        self.catalog_song_bpm(room.music_id)
    }

    /// Stage the owned-item inventory counts, one per skill slot (the
    /// waiting `SKILLCOUNT` caption source; `JoinedRoom.skills`
    /// carries the ring ids only).
    pub fn set_skill_counts(&mut self, counts: [u32; 3]) -> &mut Self {
        self.skill_counts = counts;
        self
    }

    /// Staged inventory counts, one per skill slot (zeroed until
    /// staged).
    #[must_use]
    pub fn skill_counts(&self) -> [u32; 3] {
        self.skill_counts
    }

    /// Waiting song panel rows: title row plus difficulty row.
    /// `None` while the choice or its title is missing; dialog art
    /// still paints then.
    fn selected_song_lines(&self) -> Option<[String; 2]> {
        let selection = self.selected_music.as_ref()?;
        let title = self.selected_song_title()?;
        Some([
            waiting_song_title_line(selection.music_id, title),
            waiting_song_detail_line(selection),
        ])
    }

    /// Stage battle totals (`RoomTeams` mirrors the server-side 4014/4016
    /// accumulation); the result verdict reads rank 1 from these totals.
    pub fn set_battle_teams(&mut self, teams: RoomTeams) -> &mut Self {
        self.battle_teams = teams;
        self
    }

    /// Claim the local battle team (winner art compares this seat team
    /// against rank 1; `None` clears the claim and hides the verdict).
    pub fn set_battle_self_team(&mut self, team: Option<u8>) -> &mut Self {
        self.battle_self_team = team;
        self
    }

    /// Deliver scripted rank bytes through the peer mirror (shot-walk
    /// fixture only — same honesty as the duo nicknames): seeds the
    /// holder bytes, then broadcasts the packet through the verified
    /// rank-remap rule (`PeerBars::apply_rank_broadcast`). Returns the
    /// per-position deliveries. Zero convention unchanged: unmatched
    /// bytes leave their rows zeroed and hidden.
    pub fn stage_scripted_ranks(
        &mut self,
        global: u8,
        stored: [u8; PEER_SLOT_COUNT],
        packet: [u8; PEER_SLOT_COUNT],
    ) -> Vec<RankDelivery> {
        self.playing.set_rank_holders(global, stored);
        self.playing.apply_peer_ranks(packet)
    }

    /// Live play behind the highway: the session's tick source
    /// (Life/Jam channels read score, life, and bonus through it).
    #[must_use]
    pub fn live_play(&self) -> &LivePlay {
        &self.playing
    }

    /// Live life word for the session Life channel: the avatar
    /// gauge's bar value (row 0 of [`Self::result_rank_rows`]
    /// carries points, jam level, and bonus — never life).
    #[must_use]
    pub fn life(&self) -> u16 {
        self.playing.life()
    }

    /// Fold one live 4015 echo into the Playing peer mirror: the
    /// stats write first (seating its gauge), then the 8-byte rank
    /// broadcast (filling result rows under the remap rule) — the
    /// same order the session bridge applies, so the session mirror
    /// and the painted bars never diverge. Returns the queue forward
    /// plus the rank deliveries.
    pub fn apply_peer_echo(
        &mut self,
        slot: u8,
        kind: u16,
        value: u16,
        packet: [u8; PEER_SLOT_COUNT],
    ) -> (Option<QueueEvent>, Vec<RankDelivery>) {
        let queue = self.playing.apply_peer_stats(slot, kind, value);
        let ranks = self.playing.apply_peer_ranks(packet);
        (queue, ranks)
    }

    /// Rank-1 battle verdict for the claimed local team, if known.
    #[must_use]
    pub fn battle_verdict(&self) -> Option<BattleVerdict> {
        battle_verdict(&self.battle_teams, self.battle_self_team)
    }

    /// Result verdict for the banner: solo rooms (and no room at
    /// all) take the solo banner; team battles carry the rank-1
    /// verdict when it resolves (`FUN_004c3120` branches).
    #[must_use]
    pub fn result_verdict(&self) -> ResultVerdict {
        if self.room_team_play() {
            ResultVerdict::Battle(self.battle_verdict())
        } else {
            ResultVerdict::Solo
        }
    }

    /// Saturated local points for the rank row's `%8d` field (the
    /// exe's 32-bit read; the u64 accumulator saturates — PERF-07:
    /// `TryFrom`, never `as`).
    fn saturate_rank_score(points: u64) -> u32 {
        u32::try_from(points).unwrap_or(u32::MAX)
    }

    /// Rank number for display row `position`: the `PeerBars` byte
    /// there, hidden solo (the exe draws `%d` at x=91 only with 2+
    /// players) and skipped while zero (empty seats ride zero bytes
    /// — the `live_rank_rows` convention in `side.rs`). Zero means
    /// no broadcast delivered a rank there yet: the song path never
    /// seeds stored ranks, so only a real 4015 broadcast with
    /// matching members fills this in.
    fn result_row_rank(peers: &PeerBars, position: usize, row_count: usize) -> Option<u8> {
        if row_count < 2 {
            return None;
        }
        let rank = peers.rank_row(position)?;
        (rank != 0).then_some(rank)
    }

    /// Live result rank rows: the local `Score`/`LongNoteBonus` on
    /// row 0, `PeerBars` rank bytes per position, joined-room member
    /// names in seat order (self `YOU` first — local-only, the wire
    /// sends no self nickname — then the occupied wire slots, capped
    /// at eight seats). Peer rows carry no
    /// judgements/combos/score/bonus (per-member 4014/4016 tallies
    /// stay server-side, so those columns stay unpainted there);
    /// speed is the shared room code on every named row, absent
    /// offline.
    #[must_use]
    pub fn result_rank_rows(&self) -> Vec<ResultRankRow> {
        let score = self.playing.score();
        let bonus_points = self.playing.bonus().points();
        let peers = self.playing.peers();
        let room = self.waiting.as_ref();
        let speed = room.map(|joined| joined.speed);
        let mut names: Vec<(String, Option<u8>)> = vec![("YOU".to_owned(), None)];
        if let Some(joined) = room {
            for slot in joined.slots.iter().take(7) {
                if let Some(member) = slot.member.as_ref() {
                    // The member record +0x31 the row painter reads
                    // is the first 4043 MemberInfo seat byte; the
                    // shell stages no 4043 table, so the joined
                    // room's team byte rides in its place, raw. The
                    // waiting-room team rides 0–7 while the 4043
                    // byte is consumed 1-based (byte 0 = team-bucket
                    // skip): whether the result copy already holds
                    // 1-based bytes stays OPEN (next step: populated
                    // battle-row capture, docs/cresult-anchors.md).
                    names.push((member.nickname.clone(), Some(member.team)));
                }
            }
        }
        let row_count = names.len().min(8);
        names
            .into_iter()
            .take(8)
            .enumerate()
            .map(|(position, (name, seat_byte))| {
                let local = position == 0;
                ResultRankRow {
                    rank: Self::result_row_rank(peers, position, row_count),
                    name: Some(name),
                    judgements: local.then_some(score.counts),
                    combos: local.then_some((score.max_combo, score.jam.level())),
                    score: local.then_some(Self::saturate_rank_score(score.points)),
                    bonus: local.then_some(bonus_points),
                    speed,
                    // The local row's +0x31 writer is unpinned (the
                    // shell stages no local member record): row 0
                    // resolves no chip until a source lands — the
                    // exe attaches a constant slot per row, so this
                    // only gates the unpinned local byte, never a
                    // worn figure (OPEN, docs/cresult-anchors.md).
                    seat_byte,
                }
            })
            .collect()
    }

    /// Grade plate frame for the finished play, if a chart loaded.
    /// The ratio runs the live scorer total against the pinned
    /// `grade_max_reference` denominator for the chart's note word
    /// (docs/cresult-anchors.md) through the pinned table
    /// [`result_grade_frame`]. The word counts chart onsets; the
    /// exe's 26 unscored records are a documented gap.
    #[must_use]
    pub fn grade(&self) -> Option<u8> {
        let word = u16::try_from(self.chart_notes).unwrap_or(u16::MAX);
        super::paint::result_grade_frame(self.playing.score(), word)
    }

    /// Stage real main-flow art for one screen: `.bnd` placements
    /// (via `bind_scene_rects`, or full-canvas for unbound blocks)
    /// plus figures loaded through `load_scene_figures`. Paint
    /// layers the live UI over it; empty art keeps the flat fallback.
    pub fn load_scene(
        &mut self,
        screen: Screen,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.scene_art[scene_index(screen)] = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Merge extra figures into one screen's staged art — a sprite
    /// the exe mounts from another archive, like `PointNum.ojs` on
    /// the Result scene (`this+0x10d10`, Playing archive;
    /// `docs/cresult-anchors.md`). Inserts into the staged map and
    /// stages empty art when the screen holds none; placements stay
    /// untouched. Figures whose names already stage are overwritten.
    pub fn merge_scene_figures(
        &mut self,
        screen: Screen,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        let art = self.scene_art[scene_index(screen)].get_or_insert_with(SceneArt::default);
        art.figures.extend(figures);
        self
    }

    /// Staged art for one screen, if any.
    fn scene_art(&self, screen: Screen) -> Option<&SceneArt> {
        self.scene_art[scene_index(screen)].as_ref()
    }

    /// Open side screen, if any.
    #[must_use]
    pub fn side(&self) -> Option<SideScreen> {
        self.side
    }

    /// Stage real side art: `.bnd` placements plus figures loaded
    /// through [`load_scene_figures`](super::super::avatar::load_scene_figures).
    pub fn load_side_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.side_placements = placements;
        self.side_figures = figures;
        self
    }

    /// Planet board click through the control-code → action
    /// dispatch table ([`planet_action_at`]): lit-emblem presses
    /// record the family pick and open the channel dialog through
    /// the flow gate (`Planet`→`Channel`, the dialog-over-board
    /// phase flag `parent+0xF64` in the exe); power opens the
    /// quit-confirm modal, gear the options modal, promotion cells
    /// record the press and drive the carousel (LEFT/RIGHT step
    /// with wrap, STOP toggles rotation — exe-pinned,
    /// `docs/planet.md`).
    /// The emblems are server channel families (live-verified 2026-11-09,
    /// `docs/planet.md` "Live verification"). FREE/DRUM stay
    /// unbound (P0).
    ///
    /// OPEN links (never invented): the exe emblem handler ends at
    /// family-select + reconnect (E1,
    /// `docs/overnight-evidence/e1-channel-dialog-trigger.md`) —
    /// the emblem → dialog-visibility edge is dynamic-only, with
    /// the named next step a stopped-state winedbg memory watch on
    /// the dialog flag during an emblem click. The same-family
    /// blank-rows effect (re-selecting the active family reconnects
    /// without re-fetching, live-verified) rides the session's
    /// channel refetch, not this shell flag. Launcher preselection
    /// (the boot auto-open observation) stays OPEN: the shell boots
    /// the bare board and the dialog opens only here, never at boot.
    /// The amber-cursor stand-in stays labeled OPEN.
    pub fn click_planet(&mut self, x: i32, y: i32) {
        // The whole-gesture driver walks the pinned split: the press
        // arms on down, the action dispatches on release-inside up.
        self.planet_pointer_down(x, y);
        self.planet_pointer_up(x, y);
    }

    fn click_dispatch(&mut self, x: i32, y: i32) {
        if self.click_side_screen(x, y) {
            return;
        }
        if self.planet_modal().is_some() {
            self.click_planet_modal(x, y);
            return;
        }
        if self.flow.current() == Screen::Room && self.room_dialog.is_some() {
            self.click_room_dialog(x, y);
            return;
        }
        match self.flow.current() {
            Screen::Avi => {
                self.enter(Screen::Planet);
            }
            Screen::Channel => self.click_channel_row(x, y),
            Screen::Room => self.click_room_row(x, y),
            Screen::Waiting => self.click_waiting(x, y),
            Screen::Result => self.click_result_button(x, y),
            // Planet board click: hit-test at the pinned `.bnd`
            // button rects and record the difficulty pick. What a
            // pick DOES on confirm is evidence-OPEN (docs/planet.md
            // OPEN 6: solo entry), and the FREE/DRUM back ends were
            // never wired (P0), so every outcome stays a no-op.
            Screen::Planet => {
                self.click_planet(x, y);
            }
            // Ranking clicks on the flow screen land nowhere: the
            // live board answers through the side-screen path
            // (`click_lanking` over staged `side_placements`).
            Screen::Lanking => {}
            // Avatar-room flow clicks land nowhere: the flow screen
            // is not side-entered (the transition map has no lobby
            // edge into it) — the side overlay owns the dispatch
            // (`click_myroom_side`).
            Screen::Myroom => {}
            // Shop clicks land nowhere: buy, gift, cart, filter, and
            // pager semantics are all unevidenced (P0).
            Screen::Shop | Screen::MusicShop => {}
            Screen::Loading | Screen::Playing => {}
        }
    }

    /// The open side screen answers its named controls at staged
    /// rects (lanking: sorts, pagers, row picks; shop: row toggles,
    /// buys, pagers, sorts; my room: the side overlay); every other
    /// side screen stays click-silent. Returns whether the click was
    /// consumed.
    fn click_side_screen(&mut self, x: i32, y: i32) -> bool {
        if self.side == Some(SideScreen::Lanking) {
            self.click_lanking(x, y);
            return true;
        }
        if self.side == Some(SideScreen::Shop) {
            self.click_shop(x, y);
            return true;
        }
        if self.side == Some(SideScreen::MusicShop) {
            self.click_music_shop(x, y);
            return true;
        }
        if self.side.is_some() {
            if self.side == Some(SideScreen::MyRoom) {
                self.click_myroom_side(x, y);
            }
            return true;
        }
        false
    }

    /// Waiting clicks: the song-select dialog consumes first, the
    /// staged room buttons answer only what it passes over.
    fn click_waiting(&mut self, x: i32, y: i32) {
        if !self.click_select_music(x, y) {
            self.click_waiting_button(x, y);
        }
    }

    /// The planet board with no side screen and no modal owns it —
    /// the only scene with a pinned press/release split (#111).
    fn bare_planet_board(&self) -> bool {
        self.side.is_none()
            && self.planet_modal().is_none()
            && self.flow.current() == Screen::Planet
    }

    /// Apply one press edge at native frame pixels: the shared
    /// helper behind the snapshot press edge — a press lands where
    /// the pointer is, so hover
    /// and press agree even when no motion preceded the press.
    fn apply_pointer_down(&mut self, x: i32, y: i32) {
        self.pointer = Some((x, y));
        if self.bare_planet_board() {
            // Pinned press feedback (#111): the planet board arms the
            // press on down and fires the action on release-inside.
            self.planet_pointer_down(x, y);
            return;
        }
        // Every other scene keeps its down-dispatch (no press-split
        // evidence there; the room-scene quickjoin arm is the named
        // next capture, `docs/hover.md`).
        self.click_dispatch(x, y);
    }

    /// Apply one release edge at native frame pixels: the shared
    /// helper behind the snapshot release edge (release-inside
    /// fires, pinned #111 arm 3).
    fn apply_pointer_up(&mut self, x: i32, y: i32) {
        self.pointer = Some((x, y));
        if self.bare_planet_board() {
            self.planet_pointer_up(x, y);
        }
    }

    /// Arm the board press on left-down over the bare board (pinned
    /// #111 arm 1: the pressed frame swaps on ButtonDown, per control
    /// — only when the staged art carries a pressed frame). Elsewhere
    /// — and while a modal owns the board — the press lands nowhere
    /// (the same gate the click router uses).
    fn planet_pointer_down(&mut self, x: i32, y: i32) {
        if self.flow.current() != Screen::Planet || self.planet_modal().is_some() {
            return;
        }
        let placements: &[ScenePlacement] = self
            .scene_art(Screen::Planet)
            .map_or(&[], |art| &art.placements);
        let press = planet_action_at(placements, x, y);
        self.planet_board.arm_press(press);
    }

    /// Release the board press (left-up). Release-inside reverts and
    /// fires the action on up (pinned #111 arm 3); release-outside
    /// reverts to REST with no action (arm 4 — the revert trigger,
    /// rect-exit vs up, stays OPEN, `docs/hover.md`; this stand-in
    /// reverts on up).
    fn planet_pointer_up(&mut self, x: i32, y: i32) {
        let Some(pressed) = self.planet_board.take_press() else {
            return;
        };
        if self.flow.current() != Screen::Planet || self.planet_modal().is_some() {
            return;
        }
        let placements: &[ScenePlacement] = self
            .scene_art(Screen::Planet)
            .map_or(&[], |art| &art.placements);
        if planet_action_at(placements, x, y) == Some(pressed) {
            self.dispatch_planet_action(pressed);
        }
    }

    /// Run one board action (the release-inside click, pinned on up
    /// not down, #111 arm 3). The board state owns the arms; the
    /// flow step rides the reported family pick.
    fn dispatch_planet_action(&mut self, action: PlanetAction) {
        if self
            .planet_board
            .apply_action(action, self.promo_frame_count())
        {
            self.enter(Screen::Channel);
        }
    }

    /// Open planet-board modal, if any (paint/click driver).
    #[must_use]
    pub fn planet_modal(&self) -> Option<PlanetModal> {
        self.planet_board.planet_modal
    }

    /// Whether the quit-confirm OK was pressed (the windowed shell
    /// exits on it via [`GameLoop::quit_requested`]).
    #[must_use]
    pub fn quit_confirmed(&self) -> bool {
        self.quit_requested
    }

    /// Stage the options-dialog sheet for the first-open tab
    /// (`Dialog_Option_*` placements plus figures, OPEN — see
    /// [`PLANET_OPTION_FIRST_TAB`]). The gear modal paints the
    /// open tab's sheet dialog-relative; empty art keeps the
    /// fallback title. Staging never moves the flow.
    pub fn load_planet_option_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.load_planet_option_tab(PLANET_OPTION_FIRST_TAB, placements, figures)
    }

    /// Stage one options-dialog tab sheet. The dialog paints the
    /// open tab's sheet when staged; tab switches flip between
    /// staged sheets, unstaged tabs keep the fallback title.
    /// Staging never moves the flow.
    pub fn load_planet_option_tab(
        &mut self,
        tab: PlanetOptionTab,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.planet_option_art[Self::option_tab_slot(tab)] = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Storage slot for one options tab (declaration order: Key,
    /// Sound, Game).
    const fn option_tab_slot(tab: PlanetOptionTab) -> usize {
        match tab {
            PlanetOptionTab::Key => 0,
            PlanetOptionTab::Sound => 1,
            PlanetOptionTab::Game => 2,
        }
    }

    /// Open options tab (the dialog paints this tab's staged sheet).
    #[must_use]
    pub const fn planet_option_tab(&self) -> PlanetOptionTab {
        self.planet_option_tab
    }

    /// Switch the open options tab (the `KEYSET`/`SOUNDSET`/
    /// `GAMESET` buttons): flips the painted sheet, keeps the
    /// dialog open, and disarms any pending rebind.
    pub fn switch_option_tab(&mut self, tab: PlanetOptionTab) {
        self.planet_option_tab = tab;
        self.option_rebind = None;
    }

    /// Options-dialog values (every tab's settings).
    #[must_use]
    pub const fn options(&self) -> &OptionsSettings {
        &self.options
    }

    /// Lane armed for rebind by a Key-tab click (`None` idle).
    #[must_use]
    pub const fn option_rebind_armed(&self) -> Option<usize> {
        self.option_rebind
    }

    /// Long-note switch snapshotted at the last song setup (`None`
    /// until the first Playing entry — the exe caches
    /// `LongnoteMode` at play `+0x17d8`, never re-reads mid-play).
    #[must_use]
    pub const fn play_longnote_mode(&self) -> Option<i32> {
        self.play_longnote
    }

    /// Load `option.ini` beside `install_root` at boot and remember
    /// the root for SAVE commits: a missing or unreadable file
    /// keeps the exe defaults (misses warn, never fail the scene).
    pub fn load_options(&mut self, install_root: &std::path::Path) -> &mut Self {
        self.options = OptionsSettings::load_from_dir(install_root);
        self.options_root = Some(install_root.to_owned());
        self
    }

    /// Persist the settings beside the remembered install root (the
    /// tab SAVE commit path). Without a root the settings stay in
    /// memory with a warning — the scene never fails.
    ///
    /// # Errors
    ///
    /// `std::io::Error` when the profile write or rename fails.
    pub fn save_options(&self) -> std::io::Result<()> {
        let Some(root) = self.options_root.as_ref() else {
            tracing::warn!("options SAVE without an install root; settings stay in memory");
            return Ok(());
        };
        self.options.save_to_dir(root)
    }

    /// Stage the quit-confirm sheet (`Dialog_Question1` placements
    /// plus figures). The power modal paints it dialog-relative
    /// when open; empty art keeps the fallback popup. Staging never
    /// moves the flow.
    pub fn load_planet_quit_scene(
        &mut self,
        placements: Vec<ScenePlacement>,
        figures: HashMap<String, AvatarFigure>,
    ) -> &mut Self {
        self.planet_quit_art = Some(SceneArt {
            placements,
            figures,
        });
        self
    }

    /// Modal click: the quit popup's OK/CANCEL (staged dialog
    /// rects when art is staged, the fallback boxes otherwise) or
    /// the options dialog's staged buttons (EXIT closes without
    /// saving, SAVE commits to `option.ini`, DEFAULT restores the
    /// tab's exe defaults, tab switches flip the sheet, and the
    /// Key/Sound/Game buttons edit the settings — staged rects
    /// only, Escape closes the unstaged dialog). Other clicks land
    /// nowhere: the modal owns the board while open.
    fn click_planet_modal(&mut self, x: i32, y: i32) {
        match self.planet_board.planet_modal {
            Some(PlanetModal::QuitConfirm) => {
                let (ok_hit, cancel_hit) = self.quit_button_hits(x, y);
                if ok_hit {
                    self.quit_requested = true;
                    self.planet_board.close_modal();
                } else if cancel_hit {
                    self.planet_board.close_modal();
                }
            }
            Some(PlanetModal::Options) => {
                self.click_options_dialog(x, y);
            }
            None => {}
        }
    }

    /// Staged hit for one options-dialog code on the open tab
    /// (`false` while the tab's sheet is unstaged — unstaged
    /// dialogs answer no button, Escape still closes them).
    fn options_hit(&self, code: u32, x: i32, y: i32) -> bool {
        self.planet_option_art[Self::option_tab_slot(self.planet_option_tab)]
            .as_ref()
            .filter(|art| art.has_figures())
            .is_some_and(|art| dialog_rect_hit(&art.placements, code, x, y))
    }

    /// One click inside the open options tab: EXIT/SAVE/DEFAULT,
    /// tab switches, then the tab's value buttons. SAVE persists
    /// beside the install root and stays open; EXIT closes without
    /// saving; a failed write warns and stays open (never fails
    /// the scene).
    fn click_options_dialog(&mut self, x: i32, y: i32) {
        let tab = self.planet_option_tab;
        if self.options_hit(tab.exit_code(), x, y) {
            self.planet_board.close_modal();
            self.option_rebind = None;
            return;
        }
        if self.options_hit(tab.save_code(), x, y) {
            if let Err(source) = self.save_options() {
                tracing::warn!(%source, "options SAVE failed; settings stay in memory");
            }
            self.option_rebind = None;
            return;
        }
        if self.options_hit(tab.default_code(), x, y) {
            match tab {
                PlanetOptionTab::Key => self.options.reset_binds(),
                PlanetOptionTab::Sound => self.options.reset_volumes(),
                PlanetOptionTab::Game => self.options.reset_game(),
            }
            self.option_rebind = None;
            return;
        }
        for code in [
            super::side::OPTION_KEY_SOUNDSET,
            super::side::OPTION_KEY_GAMESET,
            super::side::OPTION_SOUND_KEYSET,
            super::side::OPTION_SOUND_GAMESET,
            super::side::OPTION_GAME_KEYSET,
            super::side::OPTION_GAME_SOUNDSET,
        ] {
            if self.options_hit(code, x, y) {
                if let Some(next) = tab.switch_tab(code) {
                    self.switch_option_tab(next);
                }
                return;
            }
        }
        match tab {
            PlanetOptionTab::Key => self.click_option_key_tab(x, y),
            PlanetOptionTab::Sound => self.click_option_sound_tab(x, y),
            PlanetOptionTab::Game => self.click_option_game_tab(x, y),
        }
    }

    /// Key-tab value buttons: `KEY1..KEY7` arm their lane for
    /// rebind (the next mapped key press sticks), `3KEY_1..4` arm
    /// lanes 0..3 the same way (the 3-key lane mapping stays
    /// OPEN), `3KEY`/`7KEY` record the key-count selection, and
    /// `KEYTEST` flips its checkbox (both selection-only, OPEN).
    fn click_option_key_tab(&mut self, x: i32, y: i32) {
        for lane in 0..7u32 {
            if self.options_hit(OPTION_KEY_LANE_FIRST + lane, x, y) {
                self.option_rebind = usize::try_from(lane).ok();
                return;
            }
        }
        for lane in 0..4u32 {
            if self.options_hit(OPTION_KEY_3KEY_FIRST + lane, x, y) {
                self.option_rebind = usize::try_from(lane).ok();
                return;
            }
        }
        if self.options_hit(OPTION_KEY_3KEY_MODE, x, y) {
            self.options.set_three_key(true);
        } else if self.options_hit(OPTION_KEY_7KEY_MODE, x, y) {
            self.options.set_three_key(false);
        } else if self.options_hit(OPTION_KEY_TEST, x, y) {
            let tested = self.options.key_test();
            self.options.set_key_test(!tested);
        }
    }

    /// Sound-tab slider buttons: each Down/Up pair steps its volume
    /// by the ours-by-design step inside the 0–255 domain (which
    /// slider writes which `Sound` key is ours-by-design, OPEN).
    fn click_option_sound_tab(&mut self, x: i32, y: i32) {
        if self.options_hit(OPTION_SOUND_BG_MUSIC_DOWN, x, y) {
            self.options.step_bgm_music(false);
        } else if self.options_hit(OPTION_SOUND_BG_MUSIC_UP, x, y) {
            self.options.step_bgm_music(true);
        } else if self.options_hit(OPTION_SOUND_BG_EFFECT_DOWN, x, y) {
            self.options.step_bg_effect(false);
        } else if self.options_hit(OPTION_SOUND_BG_EFFECT_UP, x, y) {
            self.options.step_bg_effect(true);
        } else if self.options_hit(OPTION_SOUND_GAME_MUSIC_DOWN, x, y) {
            self.options.step_game_music(false);
        } else if self.options_hit(OPTION_SOUND_GAME_MUSIC_UP, x, y) {
            self.options.step_game_music(true);
        } else if self.options_hit(OPTION_SOUND_GAME_EFFECT_DOWN, x, y) {
            self.options.step_game_effect(false);
        } else if self.options_hit(OPTION_SOUND_GAME_EFFECT_UP, x, y) {
            self.options.step_game_effect(true);
        }
    }

    /// Game-tab buttons: effect arms (in-memory — no ini key was
    /// found), EQ Yes/No → 1/0, NoteType1/2 → 0/1, guides →
    /// 3/2/1/0, ModeFull/Win → 1/2, Longnote Used/None → 1/0.
    /// Button→value orders are ours-by-design (OPEN, see the
    /// options module docs).
    fn click_option_game_tab(&mut self, x: i32, y: i32) {
        if self.options_hit(OPTION_GAME_EFFECT_HIGH, x, y) {
            self.options.set_effect_level(0);
        } else if self.options_hit(OPTION_GAME_EFFECT_NORMAL, x, y) {
            self.options.set_effect_level(1);
        } else if self.options_hit(OPTION_GAME_EFFECT_LOW, x, y) {
            self.options.set_effect_level(2);
        } else if self.options_hit(OPTION_GAME_BLEND_YES, x, y) {
            self.options.set_blend_half(1);
        } else if self.options_hit(OPTION_GAME_BLEND_NO, x, y) {
            self.options.set_blend_half(0);
        } else if self.options_hit(OPTION_GAME_NOTETYPE1, x, y) {
            self.options.set_note_type(0);
        } else if self.options_hit(OPTION_GAME_NOTETYPE2, x, y) {
            self.options.set_note_type(1);
        } else if self.options_hit(OPTION_GAME_GUIDE_LONG, x, y) {
            self.options.set_note_tail(3);
        } else if self.options_hit(OPTION_GAME_GUIDE_MIDDLE, x, y) {
            self.options.set_note_tail(2);
        } else if self.options_hit(OPTION_GAME_GUIDE_SHORT, x, y) {
            self.options.set_note_tail(1);
        } else if self.options_hit(OPTION_GAME_GUIDE_NOUSE, x, y) {
            self.options.set_note_tail(0);
        } else if self.options_hit(OPTION_GAME_MODE_FULL, x, y) {
            self.options.set_mode(1);
        } else if self.options_hit(OPTION_GAME_MODE_WIN, x, y) {
            self.options.set_mode(2);
        } else if self.options_hit(OPTION_GAME_LONGNOTE_USED, x, y) {
            self.options.set_longnote_mode(1);
        } else if self.options_hit(OPTION_GAME_LONGNOTE_NONE, x, y) {
            self.options.set_longnote_mode(0);
        }
    }

    /// Quit-popup OK/CANCEL hits: staged dialog rects when art is
    /// staged ([`dialog_rect_hit`]), the native fallback boxes
    /// otherwise ([`quit_fallback_rects`]).
    fn quit_button_hits(&self, x: i32, y: i32) -> (bool, bool) {
        let staged = self
            .planet_quit_art
            .as_ref()
            .filter(|art| art.has_figures());
        if let Some(art) = staged {
            (
                dialog_rect_hit(&art.placements, QUIT_OK_CODE, x, y),
                dialog_rect_hit(&art.placements, QUIT_CANCEL_CODE, x, y),
            )
        } else {
            let (_, ok, cancel) = quit_fallback_rects();
            (rect_hit(&ok, x, y), rect_hit(&cancel, x, y))
        }
    }

    /// Picked planet difficulty band, if any (paint/hit-test driver).
    #[must_use]
    pub fn planet_selection(&self) -> Option<super::side::PlanetBand> {
        self.planet_board.planet.band()
    }

    /// Last pressed promotion-carousel cell, if any (paint/click
    /// driver; `None` until a staged promo cell answers a click).
    #[must_use]
    pub fn planet_promo(&self) -> Option<PlanetPromo> {
        self.planet_board.planet_promo
    }

    /// Visible promo banner frame, 1-based (exe `this+0x80`; boot 1).
    #[must_use]
    pub fn planet_promo_index(&self) -> u32 {
        self.planet_board.planet_promo_carousel.index()
    }

    /// Whether the carousel rotation tick advances the banner (exe
    /// `this+0x90` nonzero; boot on).
    #[must_use]
    pub fn planet_promo_spinning(&self) -> bool {
        self.planet_board.planet_promo_carousel.spinning()
    }

    /// Staged promo banner frame count: the layers of the staged
    /// `IDC_IMAGE_SPLANET_PROMOTION` body (`0` while unstaged — the
    /// exe's wrap then lands on 0 and its refresh paints nothing).
    fn promo_frame_count(&self) -> u32 {
        self.scene_art(Screen::Planet)
            .and_then(|art| {
                art.placements
                    .iter()
                    .find(|placed| placed.code == PLANET_PROMO_IMAGE_CODE)
            })
            .and_then(|placed| placed.sprite.as_deref())
            .and_then(|sprite| {
                self.scene_art(Screen::Planet)
                    .and_then(|art| art.figures.get(sprite))
            })
            .map_or(0, |figure| u32::try_from(figure.len()).unwrap_or(0))
    }

    /// One carousel rotation step: advance a frame while spinning,
    /// hold while stopped (the tick body, VA `0x48e3b0`, minus the
    /// cadence gate). The clocked paint path owns the exe's
    /// `PLANET_PROMO_TICK_MS` cadence (`tick_planet_promo_at`, pulsed
    /// by the windowed heartbeat); this stays the driver-invoked
    /// single step.
    pub fn tick_planet_promo(&mut self) {
        let frames = self.promo_frame_count();
        self.planet_board.tick_promo(frames);
    }

    /// Clocked rotation advance: on the bare board while spinning,
    /// anchor the accumulator on the first paint, then step one
    /// banner frame per `PLANET_PROMO_TICK_MS` of frame time (the
    /// exe's per-frame update, VA `0x48e3b0`, fires the body on the
    /// same cadence). Covered boards, a cleared flag, and frameless
    /// boards never touch the accumulator, so the banner holds.
    /// Frame time is the paint seam's explicit instant (sample clock
    /// windowed, fixed clock on replays): no wall clock here.
    fn tick_planet_promo_at(&mut self, now_ms: u64) {
        if !self.bare_planet_board()
            || !self.planet_board.planet_promo_carousel.spinning()
            || self.promo_frame_count() == 0
        {
            return;
        }
        self.planet_board
            .tick_promo_clocked(now_ms, self.promo_frame_count());
    }

    /// Stage the avatar-item table (`Itemdata_Korea.dat`): bag, equip
    /// and gift name source. Staging never moves the flow.
    pub fn set_myroom_items(&mut self, table: AvatarItemTable) -> &mut Self {
        self.myroom_items = Some(table);
        self
    }

    /// Stage fetched character data (`GetCharacterInfo` 2001):
    /// equipment, inventory, gifts, identity and cash captions.
    /// Staging never moves the flow.
    pub fn set_myroom_character(&mut self, character: CharacterInfo) -> &mut Self {
        self.myroom_character = Some(character);
        self
    }

    /// Fetched character data (`GetCharacterInfo` 2001), if the live
    /// boot staged it (`None` until the windowed entries fetch on
    /// channel login).
    #[must_use]
    pub fn myroom_character(&self) -> Option<&CharacterInfo> {
        self.myroom_character.as_ref()
    }

    /// Stage the worn figure the Room lobby paints in its avatar slot
    /// (`IDC_IMAGE_SROOM_AVATA`, code `0x13700013`). The binary stacks it
    /// from `avatar.opa` exactly like the Result figure; a missing
    /// archive stages nothing and the slot stays empty.
    pub fn load_room_avatar(&mut self, figure: AvatarFigure) -> &mut Self {
        self.room_avatar = Some(figure);
        self
    }

    /// Stage one pre-stacked waiting figure, keyed by seat (seat 0
    /// self, seats 1..=7 wire peers). Out-of-range seats are ignored;
    /// empty figures stay home (`None` keeps the slot empty).
    pub fn set_waiting_avatar(&mut self, seat: usize, figure: AvatarFigure) -> &mut Self {
        if figure.is_empty() {
            return self;
        }
        if let Some(slot) = self.waiting_figures.get_mut(seat) {
            *slot = Some(figure);
        }
        self
    }

    /// Staged waiting figure of one seat (`None` while unstaged).
    #[must_use]
    pub fn waiting_avatar(&self, seat: usize) -> Option<&AvatarFigure> {
        self.waiting_figures
            .get(seat)
            .and_then(|slot| slot.as_ref())
    }

    /// Stack one worn figure per occupied waiting seat from the
    /// `avatar.opa` archive plus the item table, through the shared
    /// [`load_worn_figure`] path (no fork: the same stacker the
    /// MyRoom figure and the Result banner use). The self seat wears
    /// the fetched character equipment, peers wear their wire
    /// `RoomMember.equipment`; `Any` gender wears the female parity
    /// (the connect slice's documented guess). Unresolvable pieces
    /// stay empty — one bad sprite never blanks a seat. Without a
    /// joined room this stages nothing. The production caller is
    /// [`Self::set_avatar_source`] plus the `set_joined_room`
    /// restage: the self arm fires only with a staged character
    /// (`set_myroom_character`), which the windowed entries fetch on
    /// channel login (ticket #58; the live-boot gate pins the fetch).
    /// Peers live; the self seat restages from live data on every join.
    pub fn stage_waiting_figures(&mut self, archive: &[u8], table: &AvatarItemTable) -> &mut Self {
        stage_seat_figures(
            &self.waiting,
            &self.myroom_character,
            archive,
            table,
            &mut self.waiting_figures,
        );
        self
    }

    /// Stage the avatar archive plus item table behind the per-seat
    /// worn figures (the launcher's `--avatar-opa` source); the seats
    /// restage at once through it and again on every later join, so
    /// peers wear their wire equipment as soon as the snapshot lands.
    pub fn set_avatar_source(&mut self, archive: Vec<u8>, table: AvatarItemTable) -> &mut Self {
        self.avatar_source = Some((archive, table));
        self.restage_waiting_figures();
        self
    }

    /// Restage per-seat worn figures through the staged avatar source
    /// (no-op without one): called when the source lands and on every
    /// later join.
    fn restage_waiting_figures(&mut self) {
        let Some((archive, table)) = self.avatar_source.as_ref() else {
            return;
        };
        stage_seat_figures(
            &self.waiting,
            &self.myroom_character,
            archive,
            table,
            &mut self.waiting_figures,
        );
    }

    /// Live avatar-room selection (slot pick, bag/gift pages+cursors).
    #[must_use]
    pub const fn myroom(&self) -> MyRoomState {
        self.myroom
    }

    /// Committed equip send for the session bridge, if any (5004).
    /// One take sends once; the next apply replaces the queued send.
    pub fn take_equip_send(&mut self) -> Option<EquipItem> {
        self.outbox.take_equip_send()
    }

    /// Whether an equip send is queued for the session bridge.
    #[must_use]
    pub fn has_pending_equip_send(&self) -> bool {
        self.outbox.has_pending_equip_send()
    }

    /// Committed gift claim for the session bridge, if any (5023).
    /// One take sends once; the next MOVE replaces the queued claim.
    pub fn take_gift_claim(&mut self) -> Option<ClaimGift> {
        self.outbox.take_gift_claim()
    }

    /// Gift-list fetch for the session bridge, if any (5017; queued
    /// on avatar-room entry). One take sends once.
    pub fn take_gift_list_request(&mut self) -> Option<GetGiftMessageList> {
        self.outbox.take_gift_list_request()
    }

    /// Bag pages over the staged inventory (page 0 without data).
    #[must_use]
    fn myroom_bag_page_count(&self) -> usize {
        match (&self.myroom_items, &self.myroom_character) {
            (Some(table), Some(character)) => myroom_bag_pages(&character.inventory, table),
            _ => 1,
        }
    }

    /// Gift-box pages over the staged box run (page 0 without data).
    #[must_use]
    fn myroom_gift_page_count(&self) -> usize {
        match (&self.myroom_items, &self.myroom_character) {
            (Some(table), Some(character)) => {
                let views = myroom_gift_views(table, &self.catalog_songs, character);
                myroom_gift_pages(&views)
            }
            _ => 1,
        }
    }

    /// Avatar-room click over the side overlay: the staged `.bnd`
    /// placement whose code dispatches and whose rect contains the
    /// point drives one [`MyRoomAction`]. BACK returns to the
    /// opening lobby; other clicks land on the selection state.
    /// The flow screen shares no dispatch: the transition map has
    /// no lobby edge into it, so only the overlay routes here.
    pub fn click_myroom_side(&mut self, x: i32, y: i32) {
        if self.side != Some(SideScreen::MyRoom) {
            return;
        }
        let action = myroom_action_at(&self.side_placements, x, y);
        match action {
            Some(MyRoomAction::BackToLobby) => {
                self.leave_side();
            }
            Some(action) => self.apply_myroom_action(action),
            None => {}
        }
    }

    /// Apply one avatar-room action to the selection state (BACK
    /// aside: callers own the leave). Equip and claim resolve
    /// against staged data and queue their exe-pinned wire bodies
    /// for the session bridge; without staged rows the press
    /// records nothing. Server-driven proofs ride a scripted stub
    /// behind this seam and stay labeled ASSUMED, never exe proof.
    fn apply_myroom_action(&mut self, action: MyRoomAction) {
        match action {
            MyRoomAction::SelectSlot(slot) => self.myroom.select_slot(slot),
            MyRoomAction::BagPage(step) => {
                let pages = self.myroom_bag_page_count();
                self.myroom.turn_bag_page(step, pages);
            }
            MyRoomAction::PickBagCell(cell) => self.myroom.pick_bag_cell(Some(cell)),
            MyRoomAction::GiftPage(step) => {
                let pages = self.myroom_gift_page_count();
                self.myroom.turn_gift_page(step, pages);
            }
            MyRoomAction::PickGiftCell(cell) => self.myroom.pick_gift_cell(Some(cell)),
            MyRoomAction::ApplyEquip => self.apply_myroom_equip(),
            MyRoomAction::ClaimGift => self.apply_myroom_claim(),
            MyRoomAction::VacateSlot | MyRoomAction::OpenShop | MyRoomAction::BackToLobby => {}
        }
    }

    /// Equip the selected bag cell: the slot derives from the item
    /// record exactly like the exe derives it (`docs/equip.md` —
    /// type 5 forces slot 11, else the record slot byte, below 17),
    /// then the 5004 body queues for the session bridge and the
    /// staged worn set mirrors it. Without a picked cell, staged
    /// rows, or a derivable slot the press records nothing.
    fn apply_myroom_equip(&mut self) {
        let Some(cell) = self.myroom.bag_cell() else {
            return;
        };
        let page = self.myroom.bag_page();
        let staged = self.myroom_items.as_ref().and_then(|table| {
            self.myroom_character.as_ref().and_then(|character| {
                let rows = myroom_bag_rows(table, &character.inventory, page);
                rows.get(cell).map(|row| row.item_id)
            })
        });
        let Some(item_id) = staged else {
            return;
        };
        let record = self
            .myroom_items
            .as_ref()
            .and_then(|table| avatar_item_by_id(table, item_id))
            .map(|row| (row.item_type, myroom_record_slot(row)));
        let Some((item_type, record_slot)) = record else {
            return;
        };
        let Some(slot) = myroom_equip_slot(item_type, record_slot) else {
            return;
        };
        let Ok(item_word) = i32::try_from(item_id) else {
            return;
        };
        self.outbox.pending_equip = Some(EquipItem {
            slot: i32::from(slot),
            item_id: item_word,
        });
        if let Some(character) = self.myroom_character.as_mut() {
            character.equipment[usize::from(slot)] = item_word;
        }
        self.restage_worn_figure();
    }

    /// Restage the worn figure after the equipment table changed
    /// (ticket #30 stale-figure leftover): the Room lobby slot and
    /// the Result banner re-stack from the staged avatar source like
    /// the channel-login staging does, and the waiting seats restage
    /// through it when a room is mounted. A missing source or an
    /// unstackable table keeps the old figures, never blanking them.
    fn restage_worn_figure(&mut self) {
        if let (Some((archive, table)), Some(character)) =
            (self.avatar_source.as_ref(), self.myroom_character.as_ref())
        {
            let figure = load_worn_figure(
                archive,
                table,
                &character.equipment,
                character.gender == Gender::Male,
            );
            if !figure.is_empty() {
                self.room_avatar = Some(figure.clone());
                self.avatar = Some(figure);
            }
        }
        self.restage_waiting_figures();
    }

    /// Claim the selected gift cell: the 5023 body queues for the
    /// session bridge (`GiftType` follows the picked run — item or
    /// song — while `is_attributive` stays false: countable stacks
    /// are unpinned on this path). Without a picked cell or staged
    /// box rows the press records nothing.
    fn apply_myroom_claim(&mut self) {
        let Some(cell) = self.myroom.gift_cell() else {
            return;
        };
        let page = self.myroom.gift_page();
        let picked = self.myroom_items.as_ref().and_then(|table| {
            self.myroom_character
                .as_ref()
                .map(|character| myroom_gift_views(table, &self.catalog_songs, character))
        });
        let Some(views) = picked else {
            return;
        };
        let Some(view) = myroom_gift_page(&views, page).get(cell) else {
            return;
        };
        let gift_type = if view.is_music {
            GiftType::Music
        } else {
            GiftType::Item
        };
        self.outbox.pending_gift_claim = Some(ClaimGift {
            gift_type,
            gift_id: view.gift_id,
            resource_id: view.resource_id,
            is_attributive: false,
        });
    }

    /// Open a side screen from a lobby; rejects other origins and
    /// remembers the lobby to return to. Entry cues the side
    /// screen's scene music (see [`SideScreen::bgm_cue`]); a
    /// rejected visit cues nothing.
    pub fn visit_side(&mut self, side: SideScreen) -> bool {
        if self.side.is_some() || !can_visit_side(self.flow.current(), side) {
            return false;
        }
        self.side_origin = Some(self.flow.current());
        self.side = Some(side);
        self.mount_staged_side_art(side);
        self.apply_bgm_cue(side.bgm_cue());
        if side == SideScreen::MyRoom {
            // Entry gift-list fetch (5017): the wire shape is
            // exe-pinned (`docs/gifts.md`); queueing it on entry is
            // ours-by-design — the exe entry path is unpinned — and
            // stays labeled until the rig names the entry fetch.
            self.outbox.pending_gift_list = true;
        }
        true
    }

    /// Mount the declaration-staged art behind one side screen on
    /// entry (ticket #136): the live boot stages every Interface
    /// State into its full-screen scene slot, while the shots walk
    /// fills these side slots directly — both name the same State
    /// (`state_of_screen` == `state_of_side`), so mounting the
    /// staged scene art reproduces the walk's backdrop on the live
    /// path. Paint and click dispatch both read the side slots, so
    /// the mount fixes the black screen and the dead clicks
    /// together. No staged art (the walk-only Tutorial backdrop)
    /// leaves walk-staged slots untouched.
    fn mount_staged_side_art(&mut self, side: SideScreen) {
        if side == SideScreen::Payment {
            if let Some(art) = self.payment_art.clone() {
                self.side_placements = art.placements;
                self.side_figures = art.figures;
            }
            return;
        }
        if let Some(slot) = side_scene_slot(side) {
            if let Some(art) = self.scene_art(slot).cloned() {
                self.side_placements = art.placements;
                self.side_figures = art.figures;
            }
        }
    }

    /// Return to the lobby that opened the side screen. The lobby
    /// is a scene entry again, so it re-cues its own scene music
    /// (see [`Screen::bgm_cue`]); a rejected return cues nothing.
    pub fn leave_side(&mut self) -> bool {
        let (Some(side), Some(origin)) = (self.side, self.side_origin) else {
            return false;
        };
        if !can_return_side(side, origin) {
            return false;
        }
        self.side = None;
        self.side_origin = None;
        self.apply_bgm_cue(origin.bgm_cue());
        true
    }

    /// Stage the lanking board catalog (`OJNList.dat` order, owned
    /// copies so the board outlives the parse borrow; mirrors
    /// [`Self::set_song_catalog`]). Resets sort and pages.
    pub fn set_lanking_catalog(&mut self, entries: &[OjnListEntry<'_>]) -> &mut Self {
        self.lanking.set_catalog(entries);
        self
    }

    /// Stage one fetched rank board (`2028 MusicPlayRankingAnswer`)
    /// behind the scripted-stub seam: the shell proves the rows,
    /// the wire ride stays ASSUMED until a live gateway replay.
    /// Resets the right-list page.
    pub fn stage_lanking_board(&mut self, answer: MusicPlayRankingAnswer) -> &mut Self {
        self.lanking.stage_board(answer);
        self
    }

    /// Active left-list sort (`None`: catalog order).
    #[must_use]
    pub fn lanking_sort(&self) -> Option<LankingSort> {
        self.lanking.sort()
    }

    /// Left-list page (eleven rows per page).
    #[must_use]
    pub fn lanking_page(&self) -> usize {
        self.lanking.page()
    }

    /// Sorted left-list songs on the current page (owned page:
    /// small, five rows at most).
    #[must_use]
    pub fn lanking_visible_songs(&self) -> Vec<LankingSong> {
        self.lanking.visible_songs()
    }

    /// Fetched rank lines on the current right-list page (wire
    /// order, ten rows at most past the self line; empty until
    /// staged).
    #[must_use]
    pub fn lanking_visible_ranks(&self) -> Vec<RankEntry> {
        self.lanking.visible_ranks()
    }

    /// Viewer's own fetched rank line for the first right-list
    /// slot (`None` until a board stages).
    #[must_use]
    pub fn lanking_self_entry(&self) -> Option<RankEntry> {
        self.lanking.self_entry()
    }

    /// Picked song title for the `MUSICNAME` caption (`None` until
    /// a row toggle or LANK button picks a visible row).
    #[must_use]
    pub fn lanking_selected_title(&self) -> Option<String> {
        self.lanking.selected_title()
    }

    /// Board click through the control-code dispatch
    /// ([`lanking_action_at`]): sorts resort and reset the page,
    /// pagers step and hold at the ends, row picks name the
    /// caption song. Staged placements only — unstaged clicks land
    /// nowhere (no fallback literals, ADR-0003). Only answers on
    /// the open lanking side screen.
    pub fn click_lanking(&mut self, x: i32, y: i32) {
        if self.side != Some(SideScreen::Lanking) {
            return;
        }
        let Some(action) = lanking_action_at(&self.side_placements, x, y) else {
            return;
        };
        self.lanking.apply_action(action);
    }

    /// Stage the item-shop catalog (`Itemdata_Korea.dat` order,
    /// owned copies so the board outlives the parse borrow). Resets
    /// the item page; the cart keeps its checks.
    pub fn set_shop_catalog(&mut self, table: &o2jam_file::AvatarItemTable) -> &mut Self {
        self.shop.set_catalog(table);
        self
    }

    /// Item-list page (eight rows per page).
    #[must_use]
    pub fn shop_page(&self) -> usize {
        self.shop.page()
    }

    /// Cart-list page (five rows per page).
    #[must_use]
    pub fn shop_cart_page(&self) -> usize {
        self.shop.cart_page()
    }

    /// Checked cart item ids in toggle order.
    #[must_use]
    pub fn shop_cart(&self) -> Vec<u32> {
        self.shop.cart()
    }

    /// Check one catalog good into the cart (id must exist; the
    /// click seam feeds the same cart — this stages restores).
    pub fn shop_cart_push(&mut self, item_id: u32) {
        self.shop.cart_push(item_id);
    }

    /// Sorted catalog goods on the current item-list page (owned
    /// page: small, eight rows at most). No sorts exist on the
    /// item board: catalog order always.
    #[must_use]
    pub fn shop_visible_goods(&self) -> Vec<ShopGood> {
        self.shop.visible_goods()
    }

    /// Checked cart goods on the current cart page (toggle order,
    /// five rows at most; unknown ids drop out).
    #[must_use]
    pub fn shop_visible_cart(&self) -> Vec<ShopGood> {
        self.shop.visible_cart()
    }

    /// Cart page count (at least one: the pager paints `1/1` on an
    /// empty cart, never `0/0`).
    #[must_use]
    pub fn shop_cart_pages(&self) -> usize {
        self.shop.cart_pages()
    }

    /// Item-list page count (at least one, same empty-cart rule).
    #[must_use]
    pub fn shop_list_pages(&self) -> usize {
        self.shop.list_pages()
    }

    /// Wallet gem cell caption (`GEM 1200`); `--` until the 5028
    /// refresh arrives.
    #[must_use]
    pub fn shop_gem_caption(&self) -> String {
        wallet_row_caption(&WalletRow::named(GEM_LABEL, self.wallet.gem))
    }

    /// Fold a 5028 wallet refresh into the shop wallet.
    pub fn apply_shop_cash_point(&mut self, response: &SyncCashPointResponse) {
        self.wallet.apply_cash_point(response);
    }

    /// Fold a 5003 sell refund into the shop wallet.
    pub fn apply_shop_sell(&mut self, response: &o2jam_net::shop::SellItemResponse) {
        self.wallet.apply_sell_response(response);
    }

    /// Fold the 5001 purchase prefix words into the shop wallet.
    pub fn apply_shop_purchase_prefix(&mut self, prefix: &o2jam_net::shop::SyncItemPurchasePrefix) {
        self.wallet.apply_purchase_prefix(prefix);
    }

    /// Fold the 5001 post-list wallet pair into the shop wallet.
    pub fn apply_shop_purchase_wallets(
        &mut self,
        wallets: &o2jam_net::shop::SyncItemPurchaseWallets,
    ) {
        self.wallet.apply_purchase_wallets(wallets);
    }

    /// Queued cart buy for the session bridge, if any (5000). One
    /// body per `BUYITEM` commit; the seam drains it.
    pub fn take_shop_buy(&mut self) -> Option<SyncItemPurchase> {
        self.outbox.take_shop_buy()
    }

    /// Item-shop click through the control-code dispatch
    /// ([`shop_action_at`]): row toggles check the cart, `BUYITEM`
    /// commits a non-empty cart through the pinned 5000 send, and
    /// both pagers step and hold at the ends. Staged placements
    /// only — unstaged clicks land nowhere. Only answers on the
    /// open shop side screen.
    pub fn click_shop(&mut self, x: i32, y: i32) {
        if self.side != Some(SideScreen::Shop) {
            return;
        }
        let Some(action) = shop_action_at(&self.side_placements, x, y) else {
            return;
        };
        if self.shop.apply_action(action) {
            self.outbox.pending_shop_buy = Some(SyncItemPurchase);
        }
    }

    /// Stage the song-shop catalog (`OJNList.dat` order, owned
    /// copies so the board outlives the parse borrow; mirrors
    /// [`Self::set_lanking_catalog`]). Resets sort and pages.
    pub fn set_music_shop_catalog(&mut self, entries: &[OjnListEntry<'_>]) -> &mut Self {
        self.music_shop.set_catalog(entries);
        self
    }

    /// Stage the fetched song flags (`GetMusicList` 4031 answer)
    /// behind the scripted-stub seam: the shell proves the rows,
    /// the wire ride stays ASSUMED until a live gateway replay.
    /// Resets the song-list page.
    pub fn stage_music_shop_board(&mut self, answer: MusicListAnswer) -> &mut Self {
        self.music_shop.stage_board(answer);
        self
    }

    /// Active song-shop sort (`None`: catalog order).
    #[must_use]
    pub fn music_shop_sort(&self) -> Option<LankingSort> {
        self.music_shop.sort()
    }

    /// Song-list page (five rows per page).
    #[must_use]
    pub fn music_shop_page(&self) -> usize {
        self.music_shop.page()
    }

    /// Music-cart page (five rows per page).
    #[must_use]
    pub fn music_shop_cart_page(&self) -> usize {
        self.music_shop.cart_page()
    }

    /// Checked song-shop cart ids in pick order.
    #[must_use]
    pub fn music_shop_cart(&self) -> Vec<u32> {
        self.music_shop.cart()
    }

    /// Pick one catalog song into the cart (id must exist; the
    /// click seam feeds the same cart — this stages restores).
    pub fn music_shop_cart_push(&mut self, song_id: u32) {
        self.music_shop.cart_push(song_id);
    }

    /// Checked music-cart songs on the current cart page (pick
    /// order, five rows at most; unknown ids drop out).
    #[must_use]
    pub fn music_shop_visible_cart(&self) -> Vec<LankingSong> {
        self.music_shop.visible_cart()
    }

    /// Music-cart page count (at least one, same empty-cart rule).
    #[must_use]
    pub fn music_shop_cart_pages(&self) -> usize {
        self.music_shop.cart_pages()
    }

    /// Song-list page count (at least one, same empty-cart rule).
    #[must_use]
    pub fn music_shop_list_pages(&self) -> usize {
        self.music_shop.list_pages()
    }

    /// Sorted song-shop rows with live flags on the current page
    /// (owned page: small, five rows at most).
    #[must_use]
    pub fn music_shop_visible_rows(&self) -> Vec<MusicShopRow> {
        self.music_shop.visible_rows()
    }

    /// Queued song buy for the session bridge, if any (5010). One
    /// body per row or cart commit; the seam drains it.
    pub fn take_music_buy(&mut self) -> Option<SyncMusicPurchase> {
        self.outbox.take_music_buy()
    }

    /// Song-shop click through the control-code dispatch
    /// ([`music_shop_action_at`]): row picks stage the cart, row
    /// buy buttons and the cart `BUYITEM` queue the pinned 5010
    /// send, sorts resort and reset the page, and the pager steps
    /// and holds at the ends. Staged placements only — unstaged
    /// clicks land nowhere. Only answers on the open song-shop
    /// side screen.
    pub fn click_music_shop(&mut self, x: i32, y: i32) {
        if self.side != Some(SideScreen::MusicShop) {
            return;
        }
        let Some(action) = music_shop_action_at(&self.side_placements, x, y) else {
            return;
        };
        if self.music_shop.apply_action(action) {
            self.outbox.pending_music_buy = Some(SyncMusicPurchase);
        }
    }

    /// Paint with an explicit song clock (tests use this; live
    /// paint reads the sample clock, [`Self::now_ms`]).
    ///
    /// Every frame starts on device black: the exe's backbuffer is
    /// black wherever a scene paints no pixel (7.55 planet capture
    /// `/home/arnim/scratch/planet-shot.png` is black in exactly the
    /// 30,725 px the staged `STATE_PLANET` art leaves uncovered, 100 %
    /// agreement), so a stale frame can never leak through staged
    /// art's transparent pixels. Scene painters still contribute no
    /// fill of their own (`docs/channel-fill.md`).
    pub fn paint_at(&mut self, frame: &mut Frame<'_>, now_ms: u64) {
        frame.buf.fill(FRAME_CLEAR_COLOR);
        if self.side.is_some() {
            self.paint_side(frame);
            return;
        }
        if self.flow.current() == Screen::Avi && now_ms >= self.splash_hold_ms {
            self.enter(Screen::Planet);
        }
        if self.flow.current() == Screen::Playing && self.playing.is_finished(now_ms) {
            // Song end enters Result through the entry hook, so
            // its scene-music cue (`0x13`, once — silent on the
            // 10-record bank, OPEN) fires like every other entry.
            // A battle game's entry posts the live-state ask
            // (4042 `0xfca`, empty — the exe's battle-gated send,
            // `docs/relay.md` §10); solo entry queues nothing here
            // (its event-ticket web call owns the solo arm and the
            // shell owns no web seam: OPEN).
            if self.room_team_play() {
                self.outbox.pending_result_battle_entry = true;
            }
            self.enter_screen(Screen::Result);
        }
        // The promo carousel rotates on board time: each paint feeds
        // the frame clock to the 3000 ms accumulator (the windowed
        // heartbeat in `idle_repaint_interval` supplies the pulse).
        self.tick_planet_promo_at(now_ms);
        match self.flow.current() {
            Screen::Playing => self.paint_playing(frame, now_ms),
            Screen::Avi => self.paint_avi(frame),
            Screen::Channel => self.paint_channel(frame),
            Screen::Room => self.paint_room(frame),
            Screen::Waiting => self.paint_waiting(frame),
            Screen::Loading => self.paint_loading(frame),
            Screen::Result => self.paint_result(frame),
            Screen::Planet => self.paint_planet(frame),
            Screen::Lanking => self.paint_lanking(frame),
            Screen::Myroom => self.paint_myroom(frame),
            Screen::Shop => self.paint_shop(frame),
            Screen::MusicShop => self.paint_music_shop(frame),
        }
    }

    /// The active side screen: staged art plus its board overlay
    /// (billing notice, lanking, shop, song-shop, my room). Each
    /// overlay inks into staged TEXT rects; without staged rows or
    /// goods it paints nothing (art-only).
    fn paint_side(&mut self, frame: &mut Frame<'_>) {
        paint_side_scene(frame, &self.side_placements, &self.side_figures);
        // The billing views complete no web call, so the
        // Payment overlay inks the unwired notice over the
        // staged art (miss path paints nothing); every other
        // side screen paints art alone.
        if self.side == Some(SideScreen::Payment) {
            paint_payment_notice(frame, &self.side_placements, &payment_side_notice());
        }
        // The lanking overlay inks rows, ranks, and the picked
        // caption into staged TEXT rects over the bound art;
        // without staged rows it paints nothing (art-only).
        if self.side == Some(SideScreen::Lanking) {
            paint_lanking_board(
                frame,
                &self.side_placements,
                &self.lanking_visible_songs(),
                self.lanking_self_entry().as_ref(),
                &self.lanking_visible_ranks(),
                self.lanking_selected_title().as_deref(),
            );
        }
        // The shop overlay inks goods, prices, the wallet cell,
        // the cart, and both pager readouts into staged TEXT
        // rects; without staged goods it paints nothing.
        if self.side == Some(SideScreen::Shop) {
            paint_shop_board(
                frame,
                &self.side_placements,
                &self.shop_visible_goods(),
                &self.wallet,
                &self.shop_visible_cart(),
                self.shop_cart_page(),
                self.shop_cart_pages(),
            );
        }
        // The song-shop overlay inks catalog rows with live
        // flags plus the cart and both pager readouts; without
        // staged songs it paints nothing.
        if self.side == Some(SideScreen::MusicShop) {
            paint_music_shop_board(
                frame,
                &self.side_placements,
                &self.music_shop_visible_rows(),
                self.music_shop_page(),
                self.music_shop_list_pages(),
            );
            paint_music_shop_cart(
                frame,
                &self.side_placements,
                &self.music_shop_visible_cart(),
                self.music_shop_cart_page(),
                self.music_shop_cart_pages(),
            );
        }
        if self.side == Some(SideScreen::MyRoom) {
            self.paint_myroom_overlay(frame, &self.side_placements);
        }
    }

    /// Item shop: staged art when present, else the flat card. Buy,
    /// gift, cart, and set-item flows stay unevidenced — paint only,
    /// no selection state.
    fn paint_shop(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Shop) {
            Some(art) => paint_shop_scene(frame, art),
            None => paint_screen(frame, Screen::Shop),
        }
    }

    /// Song shop: staged art when present, else the flat card. Preview,
    /// download, cart, and buy flows stay unevidenced — paint only,
    /// no selection state.
    fn paint_music_shop(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::MusicShop) {
            Some(art) => paint_music_shop_scene(frame, art),
            None => paint_screen(frame, Screen::MusicShop),
        }
    }

    /// Avatar room: staged art when present, else the flat card.
    /// Staged character and item rows paint the bag names, gift
    /// names, and identity captions into their staged `.bnd` rects;
    /// without staged data the art paints alone (oracle-neutral).
    /// Slot selection, equip commit, bag/gift pagers, and gift claim
    /// ride [`Self::click_myroom_side`]; vacate/goshop/shop-hop send
    /// nothing (OPEN, `docs/myroom.md`).
    fn paint_myroom(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Myroom) {
            Some(art) => {
                paint_myroom_scene(frame, art);
                self.paint_myroom_overlay(frame, &art.placements);
            }
            None => paint_screen(frame, Screen::Myroom),
        }
    }

    /// Avatar-room rows and captions over staged placements: the
    /// visible bag names, gift names, and identity/gift captions.
    /// Reads staged data only — without a staged item table and
    /// character nothing paints (no fallback geometry is invented).
    fn paint_myroom_overlay(&self, frame: &mut Frame<'_>, placements: &[ScenePlacement]) {
        let Some(table) = self.myroom_items.as_ref() else {
            return;
        };
        let Some(character) = self.myroom_character.as_ref() else {
            return;
        };
        let bag: Vec<String> = myroom_bag_rows(table, &character.inventory, self.myroom.bag_page())
            .iter()
            .map(|row| row.name.to_string())
            .collect();
        let views = myroom_gift_views(table, &self.catalog_songs, character);
        let page_gifts = myroom_gift_page(&views, self.myroom.gift_page());
        let gifts: Vec<String> = page_gifts.iter().map(|view| view.name.clone()).collect();
        let mut captions = myroom_identity_captions(character);
        captions.extend(myroom_gift_captions(
            &views,
            self.myroom.gift_page(),
            self.myroom.gift_cell(),
        ));
        paint_myroom_data(frame, placements, self.myroom, &bag, &gifts, &captions);
    }

    /// Rankings: staged art when present, else the flat card. The
    /// flow screen stays art-only; the live board (rows, sorts,
    /// pagers, ranks) answers through the side-screen path, so its
    /// overlay paints over `side_placements` instead of here.
    fn paint_lanking(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Lanking) {
            Some(art) => paint_lanking_scene(frame, art),
            None => paint_screen(frame, Screen::Lanking),
        }
    }

    /// Planet: staged art with emblem captions when present, else
    /// the flat card. A picked band draws the amber cursor stand-in
    /// inside its pinned button rect (highlight rendering is
    /// evidence-OPEN — see
    /// [`paint_planet_scene`](super::paint::paint_planet_scene)),
    /// then any open modal (quit-confirm or options) layers over
    /// the bare board.
    fn paint_planet(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Planet) {
            Some(art) => paint_planet_scene(
                frame,
                art,
                self.planet_board.planet.band(),
                self.hovered_planet_action(),
                self.planet_board.planet_press,
                self.planet_board.planet_promo_carousel.index(),
            ),
            None => paint_screen(frame, Screen::Planet),
        }
        self.paint_planet_modal(frame);
    }

    /// Open planet-board modal over the bare board, if any.
    fn paint_planet_modal(&self, frame: &mut Frame<'_>) {
        match self.planet_board.planet_modal {
            Some(PlanetModal::QuitConfirm) => {
                paint_planet_quit(frame, self.planet_quit_art.as_ref());
            }
            Some(PlanetModal::Options) => {
                paint_planet_options(
                    frame,
                    self.planet_option_art[Self::option_tab_slot(self.planet_option_tab)].as_ref(),
                    self.planet_option_tab,
                );
            }
            None => {}
        }
    }

    /// Splash: staged logo art when present, else the flat card. One
    /// staging path serves the splash: the bound `STATE_AVI` record
    /// (`IDC_BACK_LOGO` at its `.bnd` rect) through `load_scene`.
    fn paint_avi(&self, frame: &mut Frame<'_>) {
        match self.scene_art(Screen::Avi) {
            Some(art) => paint_avi_scene(frame, art),
            None => paint_screen(frame, Screen::Avi),
        }
    }

    /// Channel: staged art behind the live list, flat card fallback.
    /// The channel dialog opens OVER the planet board (verified first
    /// screen, `docs/planet.md`): the staged bound `State_Planet`
    /// block (backdrop + emblems + promo + MENU bar) paints beneath
    /// the opaque dialog sheet while the dialog art is staged. Live
    /// 2013/2014/2019 rows paint below the list in both cases; empty
    /// histories paint nothing.
    fn paint_channel(&self, frame: &mut Frame<'_>) {
        match (self.scene_art(Screen::Channel), self.channels.is_empty()) {
            (Some(art), false) => {
                paint_channel_scene(
                    frame,
                    &self.channels,
                    self.channel_top,
                    self.highlight,
                    art,
                    self.scene_art(Screen::Planet),
                    self.hovered_channel_row(),
                );
            }
            (Some(art), true) => paint_lobby_backdrop(frame, Screen::Channel, art),
            (None, false) => {
                paint_channel_list(frame, &self.channels, self.channel_top, self.highlight)
            }
            (None, true) => paint_screen(frame, Screen::Channel),
        }
        paint_channel_chat(frame, self.channel_chat.history(), self.channels.len());
    }

    /// Room lobby: staged art behind the live list, flat card fallback.
    fn paint_room(&self, frame: &mut Frame<'_>) {
        // The exe arms chrome hover on an empty live room too (every
        // `docs/hover.md` sweep row shows the six empty row panels
        // while the chrome buttons repaint on hover), so both staged
        // arms take the hovered code.
        let hovered_code = self.hovered_room_control().map(|control| control.code);
        match (self.scene_art(Screen::Room), self.rooms.is_empty()) {
            (Some(art), false) => {
                paint_room_scene(frame, &self.rooms, self.room_highlight, art, hovered_code);
            }
            (Some(art), true) => {
                paint_lobby_backdrop(frame, Screen::Room, art);
                paint_room_hover(frame, art, hovered_code);
            }
            (None, false) => paint_room_list(frame, &self.rooms, self.room_highlight),
            (None, true) => paint_screen(frame, Screen::Room),
        }
        // The lobby chat panel: live main-room rows plus the draft at
        // the staged `IDC_LIST_SROOM_CHAT`/`IDC_EDIT_SROOM_CHAT` rects
        // (ticket #37). Dialogs layer over it below.
        if let Some(art) = self.scene_art(Screen::Room) {
            paint_room_chat(
                frame,
                self.room_chat.history(),
                self.room_chat.draft(),
                &art.placements,
            );
            // The user panel rides its staged wells: the worn figure in
            // its slot and the identity readouts (ticket #30). Without a
            // fetched character the values stay empty; without a staged
            // figure the slot stays empty.
            paint_room_user_panel(
                frame,
                &art.placements,
                self.myroom_character.as_ref(),
                self.room_avatar.as_ref(),
            );
        }
        // The open room dialog layers over the lobby: the staged sheet
        // dialog-relative first (child `.bnd` rects are parent-relative
        // in storage, pinned #114 — the same `dialog_origin + rect`
        // the planet dialogs paint), then the form texts at the parsed
        // edit rects. Empty fields paint nothing (the staged plate shows
        // through).
        match self.room_dialog {
            Some(RoomDialog::CreateRoom) => {
                if let Some(art) = self.create_room_art.as_ref() {
                    paint_dialog_scene(frame, art);
                    self.paint_create_room_texts(frame, &art.placements);
                }
            }
            Some(RoomDialog::Password { .. }) => {
                if let Some(art) = self.password_art.as_ref() {
                    paint_dialog_scene(frame, art);
                    self.paint_password_text(frame, &art.placements);
                }
            }
            None => {}
        }
        // The options dialog layers over the lobby exactly as over
        // the Planet board (EXIT/Escape closes back onto Room).
        self.paint_planet_modal(frame);
    }
    /// Create-room texts at the parsed title/password edit rects,
    /// dialog-relative like every other dialog child (pinned #114).
    fn paint_create_room_texts(&self, frame: &mut Frame<'_>, placements: &[ScenePlacement]) {
        for (code, text) in [
            (
                super::side::CREATE_ROOM_TITLE_CODE,
                self.create_room_form.title.as_str(),
            ),
            (
                super::side::CREATE_ROOM_PASSWORD_CODE,
                self.create_room_form.password.as_str(),
            ),
        ] {
            if text.is_empty() {
                continue;
            }
            let Some(placed) = placements.iter().find(|placed| placed.code == code) else {
                continue;
            };
            let (origin_x, origin_y) = dialog_origin(placements);
            let placed_rect = Rect {
                x: placed.rect.x + origin_x,
                y: placed.rect.y + origin_y,
                w: placed.rect.w,
                h: placed.rect.h,
            };
            let scaled = scale_rect_to_frame(&placed_rect, frame.w, frame.h);
            draw_text(frame, scaled.x, scaled.y, text, 0xFFFF_FFFF);
        }
    }
    /// Password text at the parsed password edit rect, dialog-relative
    /// like every other dialog child (pinned #114).
    fn paint_password_text(&self, frame: &mut Frame<'_>, placements: &[ScenePlacement]) {
        if self.password_form.password.is_empty() {
            return;
        }
        let Some(placed) = placements
            .iter()
            .find(|placed| placed.code == super::side::PASSWORD_EDIT_CODE)
        else {
            return;
        };
        let (origin_x, origin_y) = dialog_origin(placements);
        let placed_rect = Rect {
            x: placed.rect.x + origin_x,
            y: placed.rect.y + origin_y,
            w: placed.rect.w,
            h: placed.rect.h,
        };
        let scaled = scale_rect_to_frame(&placed_rect, frame.w, frame.h);
        draw_text(
            frame,
            scaled.x,
            scaled.y,
            &self.password_form.password,
            0xFFFF_FFFF,
        );
    }

    /// Waiting: staged art behind the live seats, flat card fallback,
    /// then the live chat box, then the staged song-select dialog: art,
    /// its structure overlay (catalog-driven table), and the
    /// chosen-song panel. The staged MAPNAME/BPM/SKILLCOUNT captions
    /// paint from the room word, the catalog BPM column, and the
    /// staged inventory (`INSTNAME`/`NOTICE` stay unpainted, OPEN 1/2).
    ///
    /// Chat rows come from live 3012/3013 packets only
    /// ([`Self::push_waiting_user_message`]/[`Self::push_waiting_admin_message`])
    /// and paint at the `EDIT_SWAIT_CHAT 0x16300014`/`LIST_SWAIT_CHAT
    /// 0x1680002B` rects through the `TEXT_SWAIT_CHAT 0x1640003C`
    /// template stacking ([`paint_waiting_chat`]); empty histories paint
    /// nothing. The `Random` `?` cards name no per-card entries (big card
    /// on `RANDMAP 0x1650004F` → rect 78, small cards on spriteless
    /// `EFFECT_1/2` rects 68/69); `SelectMusic_Random*.ojs` bodies belong
    /// to `DAILOG_SELECTMUSIC` filter buttons. Cards stay staged art:
    /// `paint_waiting_staged` never paints those rects (see the
    /// `paint_waiting_music` docs and the chat/cards absence tests).
    fn paint_waiting(&self, frame: &mut Frame<'_>) {
        let captions = WaitingCaptions {
            song_title: self.room_song_title(),
            song_bpm: self.room_song_bpm(),
            skill_counts: self.skill_counts,
        };
        match (self.scene_art(Screen::Waiting), &self.waiting) {
            (Some(art), Some(room)) => {
                paint_waiting_scene(frame, room, art, &captions);
                paint_waiting_seat_avatars(frame, &art.placements, room, &self.waiting_figures);
                // Dual self-figure source, intended fallback: the staged
                // seat figure (fetched-character equipment) wins; the
                // lobby `room_avatar` (same worn stack, CLI-staged)
                // covers seat 0 only until the fetched character lands
                // (the windowed entries fetch it on channel login).
                if self.waiting_figures[0].is_none() {
                    paint_waiting_seat_avatar(frame, &art.placements, self.room_avatar.as_ref());
                }
                paint_waiting_chat(frame, &self.waiting_chat, &self.chat_input, &art.placements);
            }
            (Some(art), None) => paint_lobby_backdrop(frame, Screen::Waiting, art),
            (None, Some(room)) => {
                paint_waiting_members(frame, room);
                paint_waiting_chat(frame, &self.waiting_chat, &self.chat_input, &[]);
            }
            (None, None) => paint_screen(frame, Screen::Waiting),
        }
        // Song selection is the room master's alone (the `MUSELECT
        // 0x16200003` click handler drops non-master clicks, file
        // `0xb2622`): only a master self sees the dialog and the
        // chosen-song panel. The staged `BPM` caption stays for
        // everyone (non-masters pick speed from it).
        if self.is_room_master() {
            self.paint_select_music_art(frame);
            self.paint_select_music_structure(frame);
            self.paint_selected_song_panel(frame);
        }
    }

    /// Paint the song-select dialog structure over the staged art:
    /// filter columns, result tabs, the catalog-driven search table,
    /// status + pager, search row, chosung grid, and the OK pill.
    /// Rows come from the staged catalog alone; no art is invented for
    /// the check or 다운/듣기/정보 icon cells (documented gaps).
    fn paint_select_music_structure(&self, frame: &mut Frame<'_>) {
        if self.select_music_art.is_none() {
            return;
        }
        let rows = select_music_rows(&self.catalog_songs);
        paint_select_filter_columns(frame);
        paint_select_result_tabs(frame, rows.len());
        paint_select_table(frame, &rows, self.selected_music);
        paint_select_status_row(frame, rows.len());
        paint_select_search_row(frame);
        let placements: &[ScenePlacement] = self
            .select_music_art
            .as_ref()
            .map_or(&[], |art| &art.placements);
        paint_select_chosung_rows(frame, placements);
        paint_select_ok(frame);
    }

    /// Layer staged song-select dialog figures over the waiting seats.
    /// Placements without a loaded figure paint nothing.
    fn paint_select_music_art(&self, frame: &mut Frame<'_>) {
        let Some(art) = self.select_music_art.as_ref() else {
            return;
        };
        art.paint_overlay(frame);
    }

    /// Paint the chosen-song panel rows (title, then difficulty).
    /// Missing choices or titles paint nothing: art without rows.
    fn paint_selected_song_panel(&self, frame: &mut Frame<'_>) {
        let Some([title_line, detail_line]) = self.selected_song_lines() else {
            return;
        };
        Self::paint_song_panel_line(frame, 0, &title_line);
        Self::paint_song_panel_line(frame, 1, &detail_line);
    }

    /// Panel rows stacked below the seat grid: title row 0, detail row 1.
    const SONG_PANEL_ROWS: usize = 2;
    /// Row step matching the lobby list height.
    const SONG_PANEL_ROW_H: i32 = 8;

    /// Top-left of song panel row `slot` (bottom-left, below the seats).
    fn song_panel_origin(frame: &Frame<'_>, slot: usize) -> (i32, i32) {
        let width = i32::try_from(frame.w).unwrap_or(i32::MAX);
        let height = i32::try_from(frame.h).unwrap_or(i32::MAX);
        let step = i32::try_from(slot).unwrap_or(i32::MAX);
        (width / 8, height * 3 / 4 + step * Self::SONG_PANEL_ROW_H)
    }

    /// One song panel row: dark scrim plus white ink.
    fn paint_song_panel_line(frame: &mut Frame<'_>, slot: usize, text: &str) {
        if slot >= Self::SONG_PANEL_ROWS || text.is_empty() {
            return;
        }
        let (x, y) = Self::song_panel_origin(frame, slot);
        let text_w = u32::try_from(text_width(text)).unwrap_or(u32::MAX);
        fill_rect(
            frame,
            &Rect {
                x: x - 4,
                y: y - 2,
                w: text_w.saturating_add(8),
                h: 9,
            },
            pack_xrgb(0x08, 0x08, 0x10),
        );
        draw_text(frame, x, y, text, pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    /// Loading gate: staged art behind the progress bar. A staged
    /// walk fraction paints the live bar plus title–artist; unknown
    /// (no status, or no art) keeps the indeterminate full bar.
    fn paint_loading(&self, frame: &mut Frame<'_>) {
        match (
            self.scene_art(Screen::Loading),
            self.loading_status.as_ref(),
        ) {
            (Some(art), Some(status)) => {
                paint_loading_scene_with_status(frame, art, status);
            }
            (Some(art), None) => paint_loading_scene(frame, art),
            (None, _) => paint_screen(frame, Screen::Loading),
        }
    }

    /// Gameplay: highway first, then staged gauges/panels ring the
    /// frame edges while notes keep falling underneath, then the
    /// right-side avatar scene (life bar, figure, song, VOL) with the
    /// bottom tally strip over both. Missing figures paint art
    /// without avatar.
    fn paint_playing(&mut self, frame: &mut Frame<'_>, now_ms: u64) {
        let geom = HighwayGeometry::centered(frame.w, frame.h);
        // Re-caption before the art pass: the caption rides the
        // right-side avatar scene painted at the end of this
        // function, so syncing it first keeps the frame's paint
        // order while letting the Playing art borrow live across
        // the whole pass (one lookup, no per-frame placement clone).
        self.sync_song_caption();
        // Game-loop staged blits (A44): the staged scene set threads
        // through the loop's own per-frame blits — the behind-highway
        // backdrop first (the exe paints the backdrop first of all;
        // A5 paint order, A39 blit chain), the highway over it, then
        // the foreground placements. The unstaged arm keeps the flat
        // placeholder fill (ADR-0003's no-archive fallback).
        let playing_art = self.scene_art[scene_index(Screen::Playing)].as_ref();
        let staged_backdrop = playing_art.is_some_and(playing_backdrop_staged);
        if staged_backdrop {
            if let Some(art) = playing_art {
                paint_playing_backdrop(frame, art);
            }
            self.playing
                .paint_over_staged_backdrop(frame, &geom, now_ms);
        } else {
            self.playing.paint(frame, &geom, now_ms);
        }
        if let Some(art) = playing_art {
            paint_playing_foreground(frame, art);
            self.playing.paint_avatar_scene_with_placements(
                frame,
                self.avatar.as_ref(),
                now_ms,
                Some(&art.placements),
            );
        } else {
            self.playing.paint_avatar_scene_with_placements(
                frame,
                self.avatar.as_ref(),
                now_ms,
                None,
            );
        }
    }

    /// Re-caption the song line from the waiting choice while one is
    /// selected (manual captions stand while nothing is selected).
    fn sync_song_caption(&mut self) {
        let Some(selection) = self.selected_music.as_ref() else {
            return;
        };
        let speed_code = selection.speed;
        let title = self.selected_song_title().unwrap_or("").to_owned();
        self.playing.scene_mut().set_song(&title, speed_code);
    }

    /// Results: staged art (`StateResult_Top/Bottom.ojs` plus
    /// `Result_WinLose.ojs` via `FUN_004c2390`, file `0xc2390`) behind
    /// the exe-selected verdict banner and grade plate frames, the
    /// pinned GDI numbers from the live score, the rank rows from
    /// live peers plus room names, and the per-row portrait-chip
    /// resolution. The played song's album cover paints into the
    /// music-name box when it resolved (see [`Self::resolve_song_cover`]).
    fn paint_result(&self, frame: &mut Frame<'_>) {
        let album = self.song_cover.as_ref();
        match self.scene_art(Screen::Result) {
            Some(art) => paint_result_scene(
                frame,
                &ResultPaint {
                    // The eight constant slots build from the
                    // display-mode word (0xf/0x10 select the table);
                    // the shell pins no live source for that word
                    // (getter file `0x15d2a`), so no table stages and
                    // no chip paints — the exe likewise writes no
                    // table for other modes (OPEN,
                    // docs/cresult-anchors.md).
                    portrait_slots: None,
                    score: self.playing.score(),
                    rows: &self.result_rank_rows(),
                    art,
                    verdict: self.result_verdict(),
                    grade: self.grade(),
                    album,
                    // The exe life readout sums globals 0x77ee58 +
                    // 0x77ee60 and the bottom counter reads u16
                    // 0x76fc66; both writers are register-held, so no
                    // live value maps yet (docs/cresult-anchors.md
                    // OPEN 3) — neither field paints until one is
                    // pinned, and none is invented.
                    life_readout: None,
                    bottom_counter: None,
                },
            ),
            None => paint_result(
                frame,
                self.playing.score(),
                &self.result_rank_rows(),
                album,
                None,
            ),
        }
    }

    /// Splash keys: Escape dismisses (rig-pinned,
    /// `~/scratch/avi-hold/run3-escape/`), while Confirm (Enter), Up,
    /// and Down keep the splash (Enter shows no fast dismiss on the
    /// isolated rig, `~/scratch/wt24-rig/key-enter/`).
    fn avi_key(&mut self, key: MenuKey) {
        if key == MenuKey::Back {
            self.enter(Screen::Planet);
        }
    }

    /// Keyboard activation of the already-active emblem: Confirm
    /// opens the dialog like the mouse emblem click (the mouse edge
    /// is the pinned one; this is the keyboard stand-in the session
    /// walks drive).
    fn planet_key(&mut self, key: MenuKey) {
        if key == MenuKey::Confirm {
            self.enter(Screen::Channel);
        }
    }

    fn channel_key(&mut self, key: MenuKey) {
        match key {
            MenuKey::Up => self.step_channel(ChannelScroll::Up),
            MenuKey::Down => self.step_channel(ChannelScroll::Down),
            MenuKey::Back => self.enter(Screen::Planet),
            MenuKey::Confirm => {
                if self.take_channel_slash() == SlashCommand::Empty {
                    self.enter(Screen::Room);
                }
            }
        }
    }

    /// Lobby Confirm rides the main-room chat first: a non-empty
    /// draft (plain chat or a slash command) consumes it, an empty
    /// draft advances to Waiting like the Channel Confirm does
    /// (ticket #37). An open room dialog owns the keyboard, so a
    /// leftover draft neither sends nor holds the Enter.
    fn room_key(&mut self, key: MenuKey) {
        match key {
            MenuKey::Up => {
                self.room_highlight = self.room_highlight.saturating_sub(1);
            }
            MenuKey::Down => {
                let last = self.rooms.len().saturating_sub(1);
                self.room_highlight = (self.room_highlight + 1).min(last);
            }
            MenuKey::Back => self.enter(Screen::Channel),
            MenuKey::Confirm => {
                if self.room_dialog.is_some() {
                    // The open sheet owns the keyboard: Enter neither
                    // sends the lobby draft nor leaves the lobby. Its
                    // own controls (create/join, password OK) commit
                    // through their staged rects.
                } else if self.take_room_slash() == SlashCommand::Empty {
                    self.enter(Screen::Waiting);
                }
            }
        }
    }

    /// Loading Confirm runs the gate (`enter_screen` holds the edge
    /// while the gate is unready): validation, staging, OJN decrypt,
    /// and Vorbis init all staged, an audio rig attached, and no
    /// latched failure. Until then Confirm leaves the scene in
    /// Loading (honest).
    fn loading_key(&mut self, key: MenuKey) {
        match key {
            MenuKey::Confirm => self.enter(Screen::Playing),
            MenuKey::Back => self.enter(Screen::Waiting),
            _ => {}
        }
    }

    /// `START 0x16202012` runs the all-ready check (file
    /// `0xb5560`): the master starts only with every occupied peer
    /// ready, and non-masters never start — they ready up (4008,
    /// owned by the session bridge), so their Confirm stays in
    /// Waiting. With no joined room there is no master state, and
    /// the offline walk keeps advancing. Slash drafts (`/w`, `/r`,
    /// `/h`, `/help`) consume the Enter without starting: only an
    /// empty draft advances.
    fn waiting_key(&mut self, key: MenuKey) {
        match key {
            MenuKey::Back => self.enter(Screen::Room),
            MenuKey::Confirm => self.confirm_waiting(),
            _ => {}
        }
    }

    fn result_key(&mut self, key: MenuKey) {
        match key {
            MenuKey::Back => {
                self.outbox.pending_result_back = true;
                self.enter(Screen::Room);
            }
            MenuKey::Confirm => {
                self.outbox.pending_result_retry = true;
                self.enter(Screen::Waiting);
            }
            _ => {}
        }
    }
}

impl Default for ScreenGame {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLoop for ScreenGame {
    fn capture_raw_key(&mut self, code: KeyCode, down: bool) -> bool {
        // Key-tab rebind capture: while a lane is armed, the next
        // mapped non-menu key press becomes that lane's key and the
        // press is consumed (menu keys keep their routing so Escape
        // still closes and disarms).
        if !down
            || self.planet_modal() != Some(PlanetModal::Options)
            || self.planet_option_tab != PlanetOptionTab::Key
        {
            return false;
        }
        let Some(lane) = self.option_rebind else {
            return false;
        };
        if menu_of(code).is_some() {
            return false;
        }
        if self.options.rebind_lane(lane, code) {
            self.option_rebind = None;
            true
        } else {
            false
        }
    }

    fn lane_bindings(&self) -> Option<[KeyCode; 7]> {
        Some(self.options.lane_keys())
    }

    fn lane_key(&mut self, lane: usize, down: bool) {
        if !down {
            return;
        }
        match self.flow.current() {
            Screen::Avi => {
                self.enter(Screen::Planet);
            }
            Screen::Playing => {
                let now = self.now_ms();
                self.strike_at(lane, now);
            }
            _ => {}
        }
    }

    fn menu_key(&mut self, key: MenuKey, down: bool) {
        if !down {
            return;
        }
        if self.side.is_some() {
            if key == MenuKey::Back {
                self.leave_side();
            }
            return;
        }
        if self.planet_modal().is_some() {
            // Back cancels the quit-confirm and closes options
            // (Escape closes the options dialog, live-pinned).
            // Confirm stays inert so the quit needs the explicit OK.
            if key == MenuKey::Back {
                self.planet_board.close_modal();
                self.option_rebind = None;
            }
            return;
        }
        match self.flow.current() {
            Screen::Avi => self.avi_key(key),
            Screen::Planet => self.planet_key(key),
            Screen::Channel => self.channel_key(key),
            Screen::Room => self.room_key(key),
            Screen::Loading => self.loading_key(key),
            Screen::Waiting => self.waiting_key(key),
            Screen::Result => self.result_key(key),
            _ => {}
        }
    }

    fn text_input(&mut self, text: &str) {
        for typed in text.chars() {
            if self.flow.current() == Screen::Waiting {
                self.push_chat_char(typed);
            } else {
                self.push_lobby_char(typed);
            }
        }
    }

    fn delete_text(&mut self) {
        if self.flow.current() == Screen::Waiting {
            self.delete_chat_char();
        } else {
            self.delete_lobby_char();
        }
    }

    fn ingest_snapshot(&mut self, snapshot: &InputSnapshot) {
        // Snapshot-native sweep (ADR-0013): the pointer syncs at
        // paint-time position, then each drained edge applies in
        // arrival order through the press/release helpers.
        if let Some((x, y)) = snapshot.pointer() {
            self.pointer = Some((x, y));
        }
        for edge in snapshot.edges() {
            match *edge {
                InputEdge::PointerDown { x, y } => self.apply_pointer_down(x, y),
                InputEdge::PointerUp { x, y } => self.apply_pointer_up(x, y),
                InputEdge::Lane { lane, down } => self.lane_key(lane, down),
                InputEdge::Menu { key, down } => self.menu_key(key, down),
            }
        }
    }

    fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    fn screen_name(&self) -> &'static str {
        match self.flow.current() {
            Screen::Avi => "avi",
            Screen::Planet => "planet",
            Screen::Channel => "channel",
            Screen::Room => "room",
            Screen::Waiting => "waiting",
            Screen::Loading => "loading",
            Screen::Playing => "playing",
            Screen::Result => "result",
            Screen::Shop => "shop",
            Screen::MusicShop => "musicshop",
            Screen::Myroom => "myroom",
            Screen::Lanking => "lanking",
        }
    }

    fn state_hash(&self) -> u64 {
        observable_state_hash(self)
    }

    fn idle_repaint_interval(&self) -> Option<std::time::Duration> {
        match self.flow.current() {
            // Song scroll, receptor flash, and the end-of-song Result
            // edge all ride the sample clock (`paint_at` polls it), so
            // Playing keeps the loop's 60 Hz tick while a song runs.
            Screen::Playing => Some(std::time::Duration::from_nanos(crate::TICK_NANOS)),
            // The splash auto-advance rides the wall clock, so a modest
            // poll checks the hold; fixed-clock replays keep the tick
            // pace the headless seam always drove (`paint_at` advances
            // the virtual clock per paint there).
            Screen::Avi if self.fixed_clock_step_ms.is_none() => {
                Some(std::time::Duration::from_millis(SPLASH_HOLD_POLL_MS))
            }
            Screen::Avi => Some(std::time::Duration::from_nanos(crate::TICK_NANOS)),
            // The promo carousel rotates on its 3000 ms cadence while
            // the rotation flag holds (#167), so the bare board owes
            // the loop one modest pulse per second — the advance lands
            // within one pulse of the tick. Stopped, covered, or
            // frameless boards paint nothing new, so they stay fully
            // idle (the #138 parked-loop result holds).
            Screen::Planet
                if self.bare_planet_board()
                    && self.planet_promo_spinning()
                    && self.promo_frame_count() > 0 =>
            {
                Some(std::time::Duration::from_millis(PLANET_PROMO_HEARTBEAT_MS))
            }
            // Every other scene is event-driven (input, resizes, lobby
            // drains owned by the session layer): no timer of its own.
            _ => None,
        }
    }

    fn paint(&mut self, frame: &mut Frame<'_>) {
        self.advance_splash_on_wall_clock();
        let now = self.now_ms();
        self.paint_at(frame, now);
        if let Some(step_ms) = self.fixed_clock_step_ms {
            self.fixed_now_ms = self.fixed_now_ms.saturating_add(step_ms);
        }
    }
}

/// FNV-1a over one string (length-prefixed, UTF-8 bytes).
fn hash_str(hash: &mut u64, text: &str) {
    hash_u64(hash, u64::try_from(text.len()).unwrap_or(u64::MAX));
    for byte in text.as_bytes() {
        hash_byte(hash, *byte);
    }
}

/// FNV-1a over one byte.
fn hash_byte(hash: &mut u64, byte: u8) {
    *hash ^= u64::from(byte);
    *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
}

/// FNV-1a over one `u64` word (little-endian bytes).
fn hash_u64(hash: &mut u64, word: u64) {
    for shift in (0..64).step_by(8) {
        hash_byte(hash, (word >> shift) as u8);
    }
}

/// Hash of everything a player can observe without pixels: the
/// screen, both chat drafts and histories, the staged selection, the
/// loading gate, and the live score/life. The trace harness compares
/// this across replays (same stimuli → same final state hash); it is
/// a test affordance, never gameplay input.
/// Word of one held board press for the observable hash fold
/// (`None` rides as MAX; the emblem arms ride their band, the promo
/// arms their cell, the two dialog-open arms their own word).
fn planet_press_word(press: Option<PlanetAction>) -> u64 {
    match press {
        None => u64::MAX,
        Some(PlanetAction::Emblem { band }) => 100 + band.map_or(0, |band| band as u64),
        Some(PlanetAction::Promo { cell }) => 200 + cell as u64,
        Some(PlanetAction::QuitConfirm) => 300,
        Some(PlanetAction::OptionsDialog) => 400,
    }
}

fn observable_state_hash(game: &ScreenGame) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    hash_str(&mut hash, game.screen_name());
    hash_str(&mut hash, &game.chat_input);
    hash_str(&mut hash, game.channel_chat.draft());
    hash_str(&mut hash, game.room_chat.draft());
    for row in &game.waiting_chat {
        hash_str(&mut hash, &row.text);
    }
    for row in game.channel_chat.history() {
        hash_str(&mut hash, &row.text);
    }
    for row in game.room_chat.history() {
        hash_str(&mut hash, &row.text);
    }
    if let Some(selection) = &game.selected_music {
        hash_u64(&mut hash, u64::from(selection.music_id));
        hash_u64(&mut hash, u64::from(selection.difficulty));
        hash_u64(&mut hash, u64::from(selection.speed));
    }
    hash_u64(&mut hash, game.loading_gate_phase as u64);
    let score = game.playing.score();
    hash_u64(&mut hash, score.points);
    hash_u64(&mut hash, u64::from(score.max_combo));
    for count in score.counts {
        hash_u64(&mut hash, u64::from(count));
    }
    hash_u64(&mut hash, u64::from(game.playing.life()));
    hash_u64(
        &mut hash,
        game.lanking
            .lanking_sort
            .map_or(u64::MAX, |sort| sort as u64),
    );
    hash_u64(&mut hash, game.lanking.lanking_page as u64);
    hash_u64(&mut hash, game.lanking.lanking_rpage as u64);
    hash_u64(&mut hash, game.lanking.lanking_songs.len() as u64);
    hash_u64(
        &mut hash,
        game.lanking
            .lanking_board
            .as_ref()
            .map_or(0, |board| board.entries.len() as u64),
    );
    if let Some(selected) = game.lanking.lanking_selected {
        hash_u64(&mut hash, u64::from(selected));
    }
    hash_u64(&mut hash, game.shop.shop_page as u64);
    hash_u64(&mut hash, game.shop.shop_cart_page as u64);
    hash_u64(&mut hash, game.shop.shop_goods.len() as u64);
    for id in &game.shop.shop_cart {
        hash_u64(&mut hash, u64::from(*id));
    }
    hash_u64(
        &mut hash,
        game.wallet.gem.map_or(u64::MAX, |gem| gem as u64),
    );
    hash_u64(
        &mut hash,
        game.music_shop
            .music_shop_sort
            .map_or(u64::MAX, |sort| sort as u64),
    );
    hash_u64(&mut hash, game.music_shop.music_shop_page as u64);
    hash_u64(&mut hash, game.music_shop.music_shop_cart_page as u64);
    hash_u64(&mut hash, game.music_shop.music_shop_songs.len() as u64);
    hash_u64(
        &mut hash,
        game.music_shop
            .music_shop_board
            .as_ref()
            .map_or(0, |board| board.entries.len() as u64),
    );
    for id in &game.music_shop.music_shop_cart {
        hash_u64(&mut hash, u64::from(*id));
    }
    hash_u64(
        &mut hash,
        game.planet_board
            .planet
            .band()
            .map_or(u64::MAX, |band| band as u64),
    );
    hash_u64(
        &mut hash,
        game.planet_board
            .planet_promo
            .map_or(u64::MAX, |cell| cell as u64),
    );
    hash_u64(
        &mut hash,
        u64::from(game.planet_board.planet_promo_carousel.index()),
    );
    hash_u64(
        &mut hash,
        u64::from(game.planet_board.planet_promo_carousel.spinning()),
    );
    hash_u64(
        &mut hash,
        game.planet_board.promo_last_advance_ms.unwrap_or(0),
    );
    hash_u64(&mut hash, planet_press_word(game.planet_board.planet_press));
    hash_u64(
        &mut hash,
        game.planet_board
            .planet_modal
            .map_or(u64::MAX, |modal| modal as u64),
    );
    hash_u64(&mut hash, game.outbox.pending_shop_buy.map_or(0, |_| 5_000));
    hash_u64(
        &mut hash,
        game.outbox.pending_music_buy.map_or(0, |_| 5_010),
    );
    hash
}

#[cfg(test)]
mod game_tests {
    use super::super::paint::{
        background_of, channel_chat_row_text, rank_notice_board_text, seat_origin, RESULT_MUSIC_BOX,
    };
    use super::super::select_paint::{select_active_ink, select_dim_ink, select_scrim};
    use super::super::side::{
        LANKING_LPAGE_NEXT_CODE, LANKING_LPAGE_PREV_CODE, LANKING_LVIEW_TEXT_CODES,
        LANKING_LVIEW_TOGGLE_FIRST_CODE, LANKING_MUSICNAME_CODE, LANKING_RPAGE_NEXT_CODE,
        LANKING_RPAGE_PREV_CODE, LANKING_RVIEW_TEXT_CODES, LANKING_RVIEW_TOGGLE_FIRST_CODE,
        LANKING_SORT_TITLE_CODE, MSHOP_BPM_SORT_CODE, MSHOP_CART_BUY_CODE, MSHOP_CART_LEFT_CODE,
        MSHOP_CART_RIGHT_CODE, MSHOP_DIFF_SORT_CODE, MSHOP_EX_FIRST_CODE, MSHOP_GEM_BUY_CODE,
        MSHOP_MSALL_LEFT_CODE, MSHOP_MSALL_RIGHT_CODE, MSHOP_PAGE_CUR_ONES_CODE,
        MSHOP_PAGE_CUR_TENS_CODE, MSHOP_PAGE_TOT_ONES_CODE, MSHOP_PAGE_TOT_TENS_CODE,
        MSHOP_TITLE_SORT_CODE, MSHOP_WON_BUY_CODE, ROOM_BACK_BUTTON_CODE, ROOM_BACK_BUTTON_ID,
        ROOM_OPTION_BUTTON_CODE, ROOM_OPTION_BUTTON_ID, SHOP_BUYITEM_CODE, SHOP_CART_LEFT_CODE,
        SHOP_CART_PAGE_CUR_ONES_CODE, SHOP_CART_PAGE_CUR_TENS_CODE, SHOP_CART_PAGE_TOT_ONES_CODE,
        SHOP_CART_PAGE_TOT_TENS_CODE, SHOP_CART_RIGHT_CODE, SHOP_GEMMONEY_CODE,
        SHOP_ITEMBUY_FIRST_CODE, SHOP_ITEMNAME_FIRST_CODE, SHOP_ITEMWON_FIRST_CODE,
        SHOP_SCROLL_CODE, SHOP_SCROLL_LEFT_CODE, SHOP_SCROLL_RIGHT_CODE,
    };
    use super::*;
    use crate::avatar::AvatarFigure;
    use crate::avatar::PlacementRole;
    use crate::screen::Screen;
    use crate::{GameLoop, MenuKey};
    use o2jam_net::{ChannelState, JoinedRoom, RoomListEntry, RoomMember, RoomSlot};
    use o2jam_render::{has_ink_near, near_xrgb};
    use o2jam_render::{pack_xrgb, Frame};
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn select_chosung_rows_ride_the_staged_index_inventory() {
        // FILE wiring (docs/geometry-audit.md): with the dialog's
        // INDEX buttons staged, the letters land by the staged
        // inventory pitch (20 here, deliberately not the constant
        // 15); without them the audited constants drive the grid.
        let mut staged = vec![0u32; 800 * 600];
        let pitch20 = (0..42)
            .map(|index| ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: 0x8C00_0000 | (88 + index as u32),
                sprite: None,
                rect: Rect {
                    x: 345 + 20 * (index % 27),
                    y: if index < 27 { 440 } else { 455 },
                    w: 14,
                    h: 14,
                },
            })
            .collect::<Vec<_>>();
        paint_select_chosung_rows(
            &mut Frame {
                buf: &mut staged,
                w: 800,
                h: 600,
            },
            &pitch20,
        );
        // Cell 14 is 'A': at pitch 20 it inks at x 632, and the
        // pitch-15 spot (x 562) stays bare.
        let band = |buf: &[u32], x: i32| {
            (446..458)
                .flat_map(|y| (x..x + 6).map(move |px| buf[y * 800 + px as usize]))
                .any(|pixel| pixel != 0)
        };
        assert!(band(&staged, 632), "'A' must ride the staged pitch 20");
        assert!(!band(&staged, 562), "pitch-15 spot must stay bare");
        let mut fallback = vec![0u32; 800 * 600];
        paint_select_chosung_rows(
            &mut Frame {
                buf: &mut fallback,
                w: 800,
                h: 600,
            },
            &[],
        );
        assert!(
            band(&fallback, 562),
            "without placements the constant pitch 15 drives 'A'"
        );
    }

    #[test]
    fn loaded_chart_scales_notes_down_the_highway() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 1500, 7)]);
        for next in [
            Screen::Planet,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
        ] {
            game.flow.go(next).unwrap();
        }
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            1000,
        );
        let note_y = 48 - (500.0 * 48.0 / 2000.0) as i32;
        // Lane 2 spans x 4..6 at 70 wide (native x55 w28); the note
        // insets one pixel each side.
        assert_eq!(buf[note_y as usize * 70 + 4], pack_xrgb(0x40, 0xC0, 0xFF));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn strikes_play_banked_samples() {
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut game = ScreenGame::new();
        let bank = HashMap::from([(
            7u32,
            PcmSamples {
                samples: vec![0i16; 8],
                sample_rate: 8000,
                channels: 1,
            },
        )]);
        game.with_audio(Box::new(NullBackend::new()), bank);
        game.load_chart(vec![(2, 5000, 7), (2, 6000, 999)]);
        for next in [
            Screen::Planet,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
        ] {
            game.flow.go(next).unwrap();
        }
        game.strike_at(2, 5000);
        assert_eq!(game.playing.score().combo, 1);
        assert!(game.playing.drain_sounds().is_empty());
    }

    /// 2x1 sprite-55 figure, transparent black then red: the worn
    /// avatar the result screen must ignore per the exe convention.
    fn tiny_avatar_sprite() -> AvatarFigure {
        use o2jam_file::{parse_sprite55, SPRITE55_MAGIC};
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 2u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x7C]);
        let sprite = parse_sprite55(&blob).unwrap();
        AvatarFigure::from_sprite55(&sprite)
    }

    #[test]
    fn result_rows_carry_the_peer_seat_byte_for_portrait_chips() {
        // The row painter's record +0x31 (first 4043 MemberInfo seat
        // byte) rides the joined room's team byte in the shell, raw
        // (docs/cresult-anchors.md, "Peer-row portraits"): occupied
        // peer seats feed `seat_byte`, the local row stays `None`
        // until a local-record writer is pinned, and the battle-skip
        // zero travels through untouched.
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.slots[0].member.as_mut().unwrap().nickname = "HOST".to_owned();
        room.slots[0].member.as_mut().unwrap().team = 3;
        let zero = {
            let mut member = room.slots[0].member.clone().expect("occupied seat");
            member.nickname = "ZERO".to_owned();
            member.team = 0;
            member
        };
        room.slots.push(RoomSlot {
            index: 1,
            state_code: 1,
            member: Some(zero),
        });
        game.waiting = Some(room);
        let rows = game.result_rank_rows();
        assert_eq!(rows[0].seat_byte, None, "local record byte OPEN");
        assert_eq!(rows[1].seat_byte, Some(3));
        assert_eq!(
            rows[2].seat_byte,
            Some(0),
            "battle-skip zero passes through"
        );
        assert_eq!(rows.len(), 3, "only occupied seats feed rows");
    }

    #[test]
    fn result_screen_ignores_the_worn_figure_per_the_exe_convention() {
        // The exe attaches a constant slot object for EVERY row and
        // never reads worn equipment on this path (the eight slots
        // are image-less; what they render stays OPEN) — the shell's
        // row-0 worn avatar was the documented deviation: a staged
        // avatar figure walks the whole session without ever inking
        // the result frame.
        let mut game = ScreenGame::new();
        game.load_avatar(tiny_avatar_sprite());
        for next in [
            Screen::Planet,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            game.flow.go(next).unwrap();
        }
        let mut buf = vec![0u32; 256 * 256];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 256,
                h: 256,
            },
            0,
        );
        // The exe row-0 slot region (78,316 → (24,134) at 256x256)
        // keeps the flat card: no worn-figure pixel lands there.
        assert_eq!(buf[134 * 256 + 24], pack_xrgb(0x10, 0x10, 0x08));
        assert_eq!(buf[134 * 256 + 25], pack_xrgb(0x10, 0x10, 0x08));
    }

    /// Dismiss the splash onto the bare board, then open the dialog
    /// through a lit-emblem click (EASY center): the S0.3 boot walk
    /// every test driver shares.
    fn dismiss_splash_to_dialog(game: &mut ScreenGame) {
        game.lane_key(3, true);
        game.click_planet(136, 88);
    }

    /// Stage a gate-ready song before the Loading Confirm: the
    /// session/shots shape — a validated pick staged against a
    /// staged catalog, an audio rig attached (null sink where the
    /// test brought none), and the walk walked to the armed phase
    /// (decrypt and Vorbis init already staged). Fills only what
    /// the test did not stage, so staged banks/captions survive.
    fn stage_armed_loading_gate(game: &mut ScreenGame) {
        if game.selected_music.is_none() {
            game.selected_music = Some(SetRoomMusic {
                music_id: 757,
                difficulty: 1,
                speed: 0,
            });
        }
        if game.catalog_songs.is_empty() {
            game.catalog_songs = vec![SelectMusicSong {
                song_id: 757,
                title: "First".to_owned(),
                artist: "Band".to_owned(),
            }];
        }
        if game.audio.is_none() {
            game.with_audio(Box::new(o2jam_audio::NullBackend::new()), HashMap::new());
        }
        game.set_loading_status(LoadingStatus {
            progress: 1.0,
            title: "First".to_owned(),
            artist: "Band".to_owned(),
        });
    }

    fn walk_to_playing(game: &mut ScreenGame) {
        use MenuKey;
        dismiss_splash_to_dialog(game);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        stage_armed_loading_gate(game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
    }

    #[test]
    fn menu_walks_lobbies_to_playing() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        stage_armed_loading_gate(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(game.song_anchor_sample, 0);
    }

    #[test]
    fn song_clock_totals_backend_samples_not_wall_time() {
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut game = ScreenGame::new();
        // No audio rig yet: parked at 0, never wall time.
        assert_eq!(game.now_ms(), 0);
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        game.load_chart(vec![(2, 1000, 7)]);
        walk_to_playing(&mut game);
        // Entry anchors at 0 samples; 44100 played frames at
        // 44100 Hz is one song second regardless of wall time.
        {
            let rig = game.audio.as_mut().expect("rig");
            rig.backend
                .play_pcm(&PcmSamples {
                    samples: vec![0i16; 44_100 * 2],
                    sample_rate: 44_100,
                    channels: 2,
                })
                .unwrap();
            assert_eq!(rig.backend.samples_played(), 44_100);
        }
        assert_eq!(game.now_ms(), 1000);
        // The entry anchor moves: position restarts from here.
        game.song_anchor_sample = 44_100;
        assert_eq!(game.now_ms(), 0);
    }

    #[test]
    fn live_lane_key_rates_on_the_sample_clock() {
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut game = ScreenGame::new();
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        game.load_chart(vec![(2, 1000, 7)]);
        walk_to_playing(&mut game);
        {
            let rig = game.audio.as_mut().expect("rig");
            rig.backend
                .play_pcm(&PcmSamples {
                    samples: vec![0i16; 44_100 * 2],
                    sample_rate: 44_100,
                    channels: 2,
                })
                .unwrap();
        }
        // Lane key down reads `now_ms` (1000) and rates the note.
        game.lane_key(2, true);
        assert_eq!(game.live_play().score().counts, [1, 0, 0, 0]);
    }

    #[test]
    fn input_latency_gate_holds_the_dispatch_budget() {
        use crate::input_latency::{
            measure_input_latency, JudgementStamp, StampState, INPUT_LATENCY_BUDGET_TICKS,
        };
        use o2jam_audio::{NullBackend, PcmSamples};
        use std::cell::RefCell;
        use std::rc::Rc;
        use winit::keyboard::KeyCode;

        // Reference: identical walk, one queued song second, then a
        // direct lane strike — the judged state the probed run must
        // reach through the windowed dispatch path.
        let mut reference = ScreenGame::new();
        reference.with_audio(Box::new(NullBackend::new()), HashMap::new());
        reference.load_chart(vec![(0, 1000, 7)]);
        walk_to_playing(&mut reference);
        reference
            .audio
            .as_mut()
            .expect("rig")
            .backend
            .play_pcm(&PcmSamples {
                samples: vec![0i16; 44_100 * 2],
                sample_rate: 44_100,
                channels: 2,
            })
            .unwrap();
        reference.lane_key(0, true);
        let expected = reference.state_hash();

        let mut game = ScreenGame::new();
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        game.load_chart(vec![(0, 1000, 7)]);
        walk_to_playing(&mut game);
        game.audio
            .as_mut()
            .expect("rig")
            .backend
            .play_pcm(&PcmSamples {
                samples: vec![0i16; 44_100 * 2],
                sample_rate: 44_100,
                channels: 2,
            })
            .unwrap();
        let state = Rc::new(RefCell::new(StampState::new()));
        let shell = crate::Shell::new(Box::new(JudgementStamp::new(game, state.clone())));
        let mut handler = crate::window::WindowHandler::new(shell);
        // A scripted KeyS with a modeled 12-tick queue: the probe
        // must reach the judgement arm without the shell adding a
        // single tick, and rate the same note as the reference.
        let sample = measure_input_latency(&mut handler, KeyCode::KeyS, 12, &state)
            .expect("lane strike reached the judgement arm");
        assert_eq!(sample.added_ticks, 0, "shell-added latency: {sample:?}");
        assert!(sample.within_budget());
        assert_eq!(INPUT_LATENCY_BUDGET_TICKS, 0);
        assert_eq!(state.borrow().judged_lanes(), [0]);
        assert_eq!(
            handler.shell.game.state_hash(),
            expected,
            "probed strike judged the same note as the direct strike"
        );
    }

    #[test]
    fn menu_backs_out_of_lobbies() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Channel);
        // Back on the dialog-up phase closes to the bare board
        // (Esc symmetry with the verified X close).
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn chart_end_moves_to_result_with_score() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            7000,
        );
        assert_eq!(game.screen(), Screen::Result);
        // 70x60 is not the native canvas, so the pinned number
        // fields wait (frame_is_native); the score lives in state
        // (`playing.score()`).
        assert_ne!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn unfinished_chart_stays_playing() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            4995,
        );
        assert_eq!(game.screen(), Screen::Playing);
    }

    #[test]
    fn life_read_reports_the_avatar_gauge_word() {
        // Life-channel seam: full until play moves it, always the
        // bar word behind the gauge (rank row 0 never carries it).
        let mut game = ScreenGame::new();
        assert_eq!(game.life(), o2jam_game::LIFE_MAX);
        game.load_chart(vec![(0, 1000, 7), (0, 2000, 7)]);
        game.strike_at(0, 1000);
        assert_eq!(game.life(), game.live_play().score().life.life());
    }

    #[test]
    fn peer_echo_seats_a_gauge_in_playing_paint() {
        // Session-mirror wiring: one Life echo seats slot 0 (queue
        // forward plus the all-zero rank broadcast), and the Playing
        // paint fills its top-right gauge green.
        let mut game = ScreenGame::new();
        game.load_chart(vec![(0, 5000, 7)]);
        walk_to_playing(&mut game);
        let (queue, ranks) =
            game.apply_peer_echo(0, o2jam_game::STAT_LIFE, o2jam_game::LIFE_MAX, [0; 8]);
        assert!(queue.is_some());
        assert_eq!(ranks.len(), 8);
        assert_eq!(game.live_play().peers().life(0), Some(o2jam_game::LIFE_MAX));
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            0,
        );
        assert_eq!(buf[65], pack_xrgb(0x20, 0xE0, 0x20));
    }

    fn two_channels() -> Vec<ChannelState> {
        vec![
            ChannelState {
                gateway_id: 0,
                channel_id: 2,
                capacity: 100,
                population: 5,
                active: true,
            },
            ChannelState {
                gateway_id: 0,
                channel_id: 9,
                capacity: 0,
                population: 0,
                active: false,
            },
        ]
    }

    #[test]
    fn highlight_moves_with_bounds() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Up, true);
        game.menu_key(MenuKey::Up, true);
        game.menu_key(MenuKey::Up, true);
        // Down,Down clamps at 1; three Ups clamp back at 0.
        let mut probe = ScreenGame::new();
        probe.set_channels(two_channels());
        probe.lane_key(0, true);
        probe.click_planet(136, 88);
        probe.menu_key(MenuKey::Down, true);
        probe.menu_key(MenuKey::Down, true);
        assert_eq!(game.highlight, 0);
        assert_eq!(probe.highlight, 1);
        // Empty list never moves.
        let mut bare = ScreenGame::new();
        bare.lane_key(0, true);
        bare.click_planet(136, 88);
        bare.menu_key(MenuKey::Down, true);
        assert_eq!(bare.highlight, 0);
    }

    #[test]
    fn channel_confirm_enters_highlighted_channel() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Down, true);
        assert_eq!(game.highlight, 1);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        assert_eq!(game.highlight, 1);
    }

    fn waiting_game_with_button(id: &str, rect: Rect) -> ScreenGame {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.load_scene(
            Screen::Waiting,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: id.to_owned(),
                code: 0,
                sprite: None,
                rect,
            }],
            HashMap::new(),
        );
        game
    }

    #[test]
    fn click_dismisses_splash_into_planet_board() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        game.ingest_snapshot(&InputSnapshot::click_at(400, 300));
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn face_click_selects_channel_row_and_enters_rooms() {
        // Ticket #56: only the face enters — row 1 face
        // (502,287)-(546,308).
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.ingest_snapshot(&InputSnapshot::click_at(524, 297));
        assert_eq!(game.highlight, 1);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn face_click_enters_the_highlighted_channel() {
        // Ticket #56: only the Korean-text enter face enters Room —
        // row 0 face (502,236)-(546,257).
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn body_click_highlights_without_entering() {
        // Ticket #56: a row-body click moves the highlight only,
        // never enters Room.
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.ingest_snapshot(&InputSnapshot::click_at(300, 240));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Channel);
        game.ingest_snapshot(&InputSnapshot::click_at(300, 290));
        assert_eq!(game.highlight, 1);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn click_outside_channel_rows_keeps_channel() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.ingest_snapshot(&InputSnapshot::click_at(10, 10));
        game.ingest_snapshot(&InputSnapshot::click_at(700, 152));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn click_scrollbar_column_and_row_margins_keep_channel() {
        // Issue #29: the scroll track (x573-593) and the dialog
        // margins share the row bands' y range; the router must
        // read the pinned row rect (both axes), not the y band.
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        // y = 240 sits inside row 0's band (224-275).
        game.ingest_snapshot(&InputSnapshot::click_at(583, 240));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Channel);
        game.ingest_snapshot(&InputSnapshot::click_at(100, 240));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Channel);
        game.ingest_snapshot(&InputSnapshot::click_at(700, 290));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Channel);
    }

    /// Walk to the channel dialog over six listed rows (the pinned
    /// viewport shows four, so the arrows have room to step).
    fn six_channel_game() -> ScreenGame {
        let mut game = ScreenGame::new();
        game.set_channels(six_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game
    }

    #[test]
    fn scroll_down_arrow_steps_the_channel_highlight() {
        // Ticket #130 (exe-pin, supersedes #55): the down arrow
        // scrolls the LIST, not the highlight — the click lands in
        // the list-top offset while the highlight stays put.
        let mut game = six_channel_game();
        game.ingest_snapshot(&InputSnapshot::click_at(583, 419));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.channel_top, 1);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn scroll_up_arrow_at_top_clamps_the_highlight() {
        // Ticket #130: the up arrow scrolls the list top (bnd-pinned
        // arrow (573,213)-(594,233)); at the first row the top
        // clamps at 0 and the highlight stays put.
        let mut game = six_channel_game();
        game.ingest_snapshot(&InputSnapshot::click_at(583, 223));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.channel_top, 0);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn scroll_arrows_clamp_at_the_last_row() {
        // Ticket #130: the list top pins at `len - shown` (six rows
        // show four, so the top clamps at 2) while the highlight
        // rides the keys alone — arrows never touch it.
        use MenuKey;
        let mut game = six_channel_game();
        for _ in 0..6 {
            game.menu_key(MenuKey::Down, true);
        }
        assert_eq!(game.highlight, 5);
        assert_eq!(game.channel_top, 2);
        game.ingest_snapshot(&InputSnapshot::click_at(583, 419));
        assert_eq!(game.highlight, 5);
        assert_eq!(game.channel_top, 2);
        game.ingest_snapshot(&InputSnapshot::click_at(583, 223));
        assert_eq!(game.highlight, 5);
        assert_eq!(game.channel_top, 1);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn scrolled_row_click_maps_through_the_list_top() {
        // Ticket #130: one down-arrow scroll (top 0 → 1) re-seats
        // the viewport, so a body click on slot 0 highlights the
        // absolute row `top + slot` (the exe row faces add the top
        // to the slot, files `0x7459a`/`0x74678`).
        let mut game = six_channel_game();
        game.ingest_snapshot(&InputSnapshot::click_at(583, 419));
        assert_eq!(game.channel_top, 1);
        game.ingest_snapshot(&InputSnapshot::click_at(300, 240));
        assert_eq!(game.highlight, 1);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn repopulating_channels_resets_the_scrolled_list_top() {
        // Ticket #130 (exe-pin, file `0x747ba`): populate zeroes
        // the list top (`+0x184 = 0`) beside the scrollbar reset
        // (`+0xb8 = 0`) — a fresh list always opens unscrolled.
        let mut game = six_channel_game();
        game.ingest_snapshot(&InputSnapshot::click_at(583, 419));
        assert_eq!(game.channel_top, 1);
        game.set_channels(six_channels());
        assert_eq!(game.channel_top, 0);
        assert_eq!(game.highlight, 0);
    }

    #[test]
    fn x_button_click_closes_the_dialog_to_the_planet_board() {
        // Ticket #163 (channel leg): the bnd-pinned X rect
        // (541,151)-(589,189) closes the dialog back to the bare
        // board — the verified native close click (577,171).
        let mut game = six_channel_game();
        game.ingest_snapshot(&InputSnapshot::click_at(577, 171));
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn channel_chrome_sweep_arms_rows_but_not_chrome() {
        // Ticket #163 (channel leg, HOVER): the pointer sweep arms
        // a row over faces and bodies alike, but the X zone and
        // the scroll arrows are not rows — hovering them arms no
        // select-bar row and names no enter face.
        let mut game = six_channel_game();
        game.ingest_snapshot(&InputSnapshot::motion_to(524, 246));
        assert_eq!(game.hovered_channel_row(), Some(0));
        assert!(game.channel_enter_at(524, 246).is_some());
        game.ingest_snapshot(&InputSnapshot::motion_to(300, 240));
        assert_eq!(game.hovered_channel_row(), Some(0));
        game.ingest_snapshot(&InputSnapshot::motion_to(300, 290));
        assert_eq!(game.hovered_channel_row(), Some(1));
        game.ingest_snapshot(&InputSnapshot::motion_to(565, 170));
        assert_eq!(game.hovered_channel_row(), None);
        assert_eq!(game.channel_enter_at(565, 170), None);
        game.ingest_snapshot(&InputSnapshot::motion_to(583, 223));
        assert_eq!(game.hovered_channel_row(), None);
        assert_eq!(game.channel_enter_at(583, 223), None);
        game.ingest_snapshot(&InputSnapshot::motion_to(583, 419));
        assert_eq!(game.hovered_channel_row(), None);
        assert_eq!(game.channel_enter_at(583, 419), None);
    }

    #[test]
    fn channel_scroll_bar_body_click_is_a_labelled_noop() {
        // Ticket #163 (channel leg, CLICK) + #130 (exe-pin): the
        // 21x45 bar body between the scroll arrows answers nothing
        // to a bare CLICK — only the arrows scroll the list top.
        // The exe DOES drag the bar (press zones set drag-active
        // `+0xc4` with capture `0x547cd0`; motion tick `0x50e7c0`
        // re-reads the live cursor into the absolute setter
        // `0x50ed00`; release arm clears `+0xc4`), but press+motion
        // drag has no shell handler yet — implementation stays a
        // recommendation (scale `+0xc8` source unpinned), not a gap.
        // The dialog carries no pager readout (NO PAGER).
        let mut game = six_channel_game();
        game.ingest_snapshot(&InputSnapshot::click_at(583, 320));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.channel_top, 0);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn empty_channel_list_answers_no_hover_or_click() {
        // Ticket #163 (channel leg, empty-list arm): with no rows
        // listed, the row bands arm nothing and clicks hold the
        // dialog — the empty dialog still paints over the splash.
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.ingest_snapshot(&InputSnapshot::motion_to(300, 240));
        assert_eq!(game.hovered_channel_row(), None);
        game.ingest_snapshot(&InputSnapshot::click_at(300, 240));
        assert_eq!(game.highlight, 0);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn channel_hover_follows_the_scrolled_viewport() {
        // Ticket #163 (channel leg, scrolled arm): with the
        // highlight on the last of six rows, the viewport shows
        // rows 2..5 — the pointer over slot 3 arms row 5, mapped
        // back through `channel_viewport` like the click path.
        use MenuKey;
        let mut game = six_channel_game();
        for _ in 0..5 {
            game.menu_key(MenuKey::Down, true);
        }
        assert_eq!(game.highlight, 5);
        game.ingest_snapshot(&InputSnapshot::motion_to(300, 400));
        assert_eq!(game.hovered_channel_row(), Some(5));
        game.ingest_snapshot(&InputSnapshot::click_at(300, 400));
        assert_eq!(game.highlight, 5);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn channel_hover_survives_a_slash_draft() {
        // Ticket #163 (channel leg): a slash draft consumes face
        // clicks without entering, but hover still names the row
        // under the pointer — the hover/click split stays pinned.
        let mut game = six_channel_game();
        game.set_channel_input("/w bob hi");
        game.ingest_snapshot(&InputSnapshot::motion_to(300, 290));
        assert_eq!(game.hovered_channel_row(), Some(1));
    }

    #[test]
    fn click_with_slash_draft_consumes_without_advancing() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.set_channel_input("/w bob hi");
        game.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        assert_eq!(game.screen(), Screen::Channel);
        assert!(game.channel_input().is_empty());
    }

    #[test]
    fn click_selects_room_row_and_enters_waiting() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        // First row band: y in [150, 158).
        game.ingest_snapshot(&InputSnapshot::click_at(300, 154));
        assert_eq!(game.room_highlight, 0);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn click_selects_staged_room_panel() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        // Left-column second panel, file order index 1
        // (`IDC_BUTTON_SROOM_EXBUTTON2`, (26,177)-(314,268)).
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_EXBUTTON2".to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 26,
                    y: 177,
                    w: 288,
                    h: 91,
                },
            }],
            HashMap::new(),
        );
        game.ingest_snapshot(&InputSnapshot::click_at(100, 200));
        assert_eq!(game.room_highlight, 1);
        // Row 1 is the locked room: the click gates through the
        // password dialog instead of joining straight to Waiting.
        // The sheet stages first: without staged art the click
        // records a noop and the lobby stays usable (the no-art arm
        // below).
        game.load_password_scene(Vec::new(), HashMap::new());
        game.ingest_snapshot(&InputSnapshot::click_at(100, 200));
        assert_eq!(game.room_dialog(), Some(RoomDialog::Password { number: 7 }));
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn click_staged_ready_enters_loading() {
        let mut game = waiting_game_with_button(
            "IDC_BUTTON_SWAIT_READY",
            Rect {
                x: 600,
                y: 100,
                w: 80,
                h: 30,
            },
        );
        game.ingest_snapshot(&InputSnapshot::click_at(640, 115));
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn click_staged_back_returns_to_rooms() {
        let mut game = waiting_game_with_button(
            "IDC_BUTTON_SWAIT_BACK",
            Rect {
                x: 20,
                y: 560,
                w: 80,
                h: 30,
            },
        );
        game.ingest_snapshot(&InputSnapshot::click_at(60, 575));
        assert_eq!(game.screen(), Screen::Room);
    }

    /// Room walked to with the pinned back button staged:
    /// `IDC_BUTTON_SROOM_BACK` (`0X13200002`) at the `State_Room.bnd`
    /// rect 1 (711,569)-(794,593) it pairs with by code ordinal.
    fn room_back_game() -> ScreenGame {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: ROOM_BACK_BUTTON_ID.to_owned(),
                code: ROOM_BACK_BUTTON_CODE,
                sprite: None,
                rect: Rect {
                    x: 711,
                    y: 569,
                    w: 83,
                    h: 24,
                },
            }],
            HashMap::new(),
        );
        game
    }

    #[test]
    fn click_staged_room_back_returns_to_channel() {
        let mut game = room_back_game();
        game.ingest_snapshot(&InputSnapshot::click_at(752, 581));
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn click_outside_room_back_keeps_room() {
        let mut game = room_back_game();
        game.ingest_snapshot(&InputSnapshot::click_at(10, 10));
        assert_eq!(game.screen(), Screen::Room);
    }

    /// Room lobby behind the hover tests: splash dismissed, planet
    /// clicked through to Room.
    fn hover_room_lobby() -> ScreenGame {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game
    }

    /// Two-layer chrome figure: layer 0 hover blue, layer 1 rest green
    /// (the EXE-pinned hover/rest pairing, `docs/hover.md`).
    fn chrome_two_layer_figure(w: usize, h: usize) -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let pixels = |word: u16| word.to_le_bytes().repeat(w * h);
        let mut figure = AvatarFigure::default();
        for word in [0x001Fu16, 0x03E0u16] {
            figure.push_layer(FigureLayer {
                image: o2jam_render::Rgb555Image::decode(&pixels(word), w, h)
                    .expect("hover layers"),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    /// Layered promo figure for the press tests: layer 0 the pressed
    /// frame (red), layer 1 the rest frame (green) — distinct frames
    /// per the #63 guard. `layers` 1 models a control with no pressed
    /// art (the option-gear rule): its one record IS the rest frame
    /// (green), so down paints nothing.
    fn press_promo_figure(w: usize, h: usize, layers: usize) -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let pixels = |word: u16| word.to_le_bytes().repeat(w * h);
        let mut figure = AvatarFigure::default();
        let words: &[u16] = match layers {
            1 => &[0x03E0],
            _ => &[0x7C00, 0x03E0],
        };
        for word in words {
            figure.push_layer(FigureLayer {
                image: o2jam_render::Rgb555Image::decode(&pixels(*word), w, h)
                    .expect("press layers"),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    /// Bare planet board with staged promo cells: PLAYSTOP carries a
    /// pressed frame (two records), the LEFT cell a single record
    /// (no pressed art). Rects are the parsed `.bnd` rects verbatim
    /// (rects 12/14, `ControlList_Interface.txt` STATE_PLANET).
    fn press_planet_game() -> ScreenGame {
        use crate::screens::side::PLANET_PROMO_LEFT_CODE;
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.load_scene(
            Screen::Planet,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-left".to_owned(),
                    code: PLANET_PROMO_LEFT_CODE,
                    sprite: Some("Promo_Left.ojs".to_owned()),
                    rect: Rect {
                        x: 353,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "playstop".to_owned(),
                    code: 0x1920_C00F,
                    sprite: Some("PlayStop.ojs".to_owned()),
                    rect: Rect {
                        x: 387,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
            ],
            HashMap::from([
                ("Promo_Left.ojs".to_owned(), press_promo_figure(26, 24, 1)),
                ("PlayStop.ojs".to_owned(), press_promo_figure(26, 24, 2)),
            ]),
        );
        game
    }

    /// One native pixel of the planet board at the shell seam.
    fn planet_pixel(game: &mut ScreenGame, x: usize, y: usize) -> u32 {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        buf[y * 800 + x]
    }

    /// Staged room lobby with a two-frame BACK button: layer 0 is the
    /// EXE-pinned hover frame, layer 1 the stacked rest top (the rig
    /// pins hover to body frame 0, `docs/hover.md`).
    fn hover_room_game() -> ScreenGame {
        let mut game = hover_room_lobby();
        let figure = chrome_two_layer_figure(83, 24);
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: ROOM_BACK_BUTTON_ID.to_owned(),
                code: ROOM_BACK_BUTTON_CODE,
                sprite: Some("Room_Back.ojs".to_owned()),
                rect: Rect {
                    x: 711,
                    y: 569,
                    w: 83,
                    h: 24,
                },
            }],
            HashMap::from([("Room_Back.ojs".to_owned(), figure)]),
        );
        game
    }

    /// Room lobby with staged chrome but an empty live list: the
    /// fresh-server state the owner playtest exercised (no rooms
    /// created, so the empty-list paint arm serves the scene).
    fn hover_empty_room_game() -> ScreenGame {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: ROOM_BACK_BUTTON_ID.to_owned(),
                code: ROOM_BACK_BUTTON_CODE,
                sprite: Some("Room_Back.ojs".to_owned()),
                rect: Rect {
                    x: 711,
                    y: 569,
                    w: 83,
                    h: 24,
                },
            }],
            HashMap::from([("Room_Back.ojs".to_owned(), chrome_two_layer_figure(83, 24))]),
        );
        game
    }

    /// One native pixel of the room scene at the shell seam.
    fn room_pixel(game: &mut ScreenGame, x: usize, y: usize) -> u32 {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        buf[y * 800 + x]
    }

    #[test]
    fn hovered_room_control_resolves_the_staged_chrome_button() {
        // The pointer is the shared hover source (ticket #26 consumes
        // it for the planet board); the room resolver maps it onto
        // staged chrome the same way clicks resolve.
        let mut game = hover_room_game();
        assert_eq!(game.hovered_room_control(), None);
        game.ingest_snapshot(&InputSnapshot::motion_to(752, 581));
        assert_eq!(
            game.hovered_room_control().map(|control| control.code),
            Some(ROOM_BACK_BUTTON_CODE)
        );
        assert_eq!(
            game.hovered_room_control().map(|control| control.id),
            Some(ROOM_BACK_BUTTON_ID)
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(10, 10));
        assert_eq!(game.hovered_room_control(), None);
    }

    #[test]
    fn hovered_room_control_clears_off_the_room_scene() {
        // Leaving the lobby drops hover even though the pointer still
        // sits inside the button rect (BACK answers the click).
        let mut game = hover_room_game();
        game.ingest_snapshot(&InputSnapshot::motion_to(752, 581));
        assert_eq!(
            game.hovered_room_control().map(|control| control.code),
            Some(ROOM_BACK_BUTTON_CODE)
        );
        game.ingest_snapshot(&InputSnapshot::click_at(752, 581));
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.hovered_room_control(), None);
    }

    #[test]
    fn hovered_room_button_repaints_its_first_frame() {
        // The exe repaints the hovered button at full body frame 0
        // over the stacked rest paint (`docs/hover.md` 2026-11-10
        // sweep); leaving the control restores the rest paint.
        let mut game = hover_room_game();
        let rest = pack_xrgb(0x00, 0xFF, 0x00);
        let hover = pack_xrgb(0x00, 0x00, 0xFF);
        assert_eq!(room_pixel(&mut game, 752, 581), rest);
        game.ingest_snapshot(&InputSnapshot::motion_to(752, 581));
        assert_eq!(room_pixel(&mut game, 752, 581), hover);
        game.ingest_snapshot(&InputSnapshot::motion_to(10, 10));
        assert_eq!(room_pixel(&mut game, 752, 581), rest);
    }

    #[test]
    fn hovered_room_button_repaints_on_an_empty_room_list() {
        // The exe arms chrome hover on an empty live room — every
        // `docs/hover.md` sweep row shows the six empty row panels
        // while BACK/CREATE/QUICKJOIN still repaint on hover — so the
        // empty-list paint arm must repaint the hovered frame too,
        // not only the non-empty-list `paint_room_scene` arm.
        let mut game = hover_empty_room_game();
        let rest = pack_xrgb(0x00, 0xFF, 0x00);
        let hover = pack_xrgb(0x00, 0x00, 0xFF);
        assert_eq!(room_pixel(&mut game, 752, 581), rest);
        game.ingest_snapshot(&InputSnapshot::motion_to(752, 581));
        assert_eq!(
            game.hovered_room_control().map(|control| control.code),
            Some(ROOM_BACK_BUTTON_CODE)
        );
        assert_eq!(room_pixel(&mut game, 752, 581), hover);
        game.ingest_snapshot(&InputSnapshot::motion_to(10, 10));
        assert_eq!(room_pixel(&mut game, 752, 581), rest);
    }

    #[test]
    fn active_room_control_ignores_hover() {
        // SHOW_ALLMODE rests at frame 1 and the rig pins no change on
        // hover (`docs/hover.md`): hovering the active filter resolves
        // to no control, so the frame stays byte-identical instead of
        // taking the frame-0 repaint.
        let mut game = hover_room_lobby();
        let rect = Rect {
            x: 112,
            y: 22,
            w: 90,
            h: 41,
        };
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_SHOW_ALLMODE".to_owned(),
                code: 0x1320_1003,
                sprite: Some("Room_Btn_Show_AllMode.ojs".to_owned()),
                rect,
            }],
            HashMap::from([(
                "Room_Btn_Show_AllMode.ojs".to_owned(),
                chrome_two_layer_figure(90, 41),
            )]),
        );
        let mut rest = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut rest,
                w: 800,
                h: 600,
            },
            0,
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(150, 40));
        assert_eq!(game.hovered_room_control(), None);
        let mut hovered = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut hovered,
                w: 800,
                h: 600,
            },
            0,
        );
        assert_eq!(rest, hovered, "active filter repaints nothing on hover");
    }

    #[test]
    fn room_row_panels_stay_static_under_the_pointer() {
        // EXE-static by negative evidence (0 changed px on every
        // panel, no `Room_MOver.ojs` overlay — `docs/hover.md`),
        // so the pointer over a panel resolves to no control and the
        // frame paints byte-identical with and without the hover.
        let mut game = hover_room_game();
        use crate::avatar::FigureLayer;
        let pixels = |word: u16| word.to_le_bytes().repeat(288 * 91);
        let mut figure = AvatarFigure::default();
        for word in [0x001Fu16, 0x03E0u16] {
            figure.push_layer(FigureLayer {
                image: o2jam_render::Rgb555Image::decode(&pixels(word), 288, 91)
                    .expect("panel layers"),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
                code: 0x1301_201B,
                sprite: Some("Room_ButtonRoom.ojs".to_owned()),
                rect: Rect {
                    x: 26,
                    y: 82,
                    w: 288,
                    h: 91,
                },
            }],
            HashMap::from([("Room_ButtonRoom.ojs".to_owned(), figure)]),
        );
        let mut rest = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut rest,
                w: 800,
                h: 600,
            },
            0,
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(100, 120));
        assert_eq!(game.hovered_room_control(), None);
        let mut hovered = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut hovered,
                w: 800,
                h: 600,
            },
            0,
        );
        assert_eq!(rest, hovered, "row panels repaint nothing on hover");
    }

    #[test]
    fn room_hover_hides_behind_the_open_dialog() {
        // Clicks route to the open dialog first, so hover follows:
        // chrome behind the dialog highlights nothing.
        let mut game = hover_room_game();
        game.ingest_snapshot(&InputSnapshot::motion_to(752, 581));
        assert_eq!(
            game.hovered_room_control().map(|control| control.code),
            Some(ROOM_BACK_BUTTON_CODE)
        );
        game.open_create_room_dialog();
        assert_eq!(game.hovered_room_control(), None);
    }

    #[test]
    fn click_staged_room_option_opens_options_over_room() {
        let mut game = room_back_game();
        game.load_scene(
            Screen::Room,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: ROOM_BACK_BUTTON_ID.to_owned(),
                    code: ROOM_BACK_BUTTON_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 711,
                        y: 569,
                        w: 83,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: ROOM_OPTION_BUTTON_ID.to_owned(),
                    code: ROOM_OPTION_BUTTON_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 727,
                        y: 1,
                        w: 60,
                        h: 32,
                    },
                },
            ],
            HashMap::new(),
        );
        game.ingest_snapshot(&InputSnapshot::click_at(757, 17));
        assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.planet_modal(), None);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn click_without_staged_buttons_ignores_waiting() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.ingest_snapshot(&InputSnapshot::click_at(640, 115));
        assert_eq!(game.screen(), Screen::Waiting);
    }

    /// Stage the waiting chrome row with exe-pinned code/rect pairs
    /// (`Interface1.opi` `STATE_WAITING` + `State_Waiting.bnd` — the
    /// same parse the #57 no-overlap guard re-derives in
    /// `tests/real_art.rs`): the shared driver for the sweep guards
    /// below (ADR-0003: staged rects, never literals). Spriteless:
    /// clicks resolve by id/code plus rect, so no figures are needed.
    fn swept_waiting_game() -> ScreenGame {
        let button = |id: &str, code: u32, rect: Rect| ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect,
        };
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_scene(
            Screen::Waiting,
            vec![
                button(
                    "IDC_BUTTON_SWAIT_START",
                    0x1620_2012,
                    Rect {
                        x: 628,
                        y: 126,
                        w: 142,
                        h: 37,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_READY",
                    0x1620_2013,
                    Rect {
                        x: 628,
                        y: 126,
                        w: 142,
                        h: 37,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_BACK",
                    0x1620_0002,
                    Rect {
                        x: 711,
                        y: 569,
                        w: 83,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_MUSELECT",
                    0x1620_0003,
                    Rect {
                        x: 540,
                        y: 493,
                        w: 54,
                        h: 49,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_MAPLEFT",
                    0x1620_000C,
                    Rect {
                        x: 631,
                        y: 539,
                        w: 32,
                        h: 20,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_MAPRIGHT",
                    0x1620_000D,
                    Rect {
                        x: 667,
                        y: 539,
                        w: 33,
                        h: 20,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_CHANGE_TITLE",
                    0x1620_0047,
                    Rect {
                        x: 478,
                        y: 13,
                        w: 86,
                        h: 26,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_EMOTION",
                    0x1620_0048,
                    Rect {
                        x: 533,
                        y: 562,
                        w: 61,
                        h: 23,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_1SKILLLEFT",
                    0x1620_0050,
                    Rect {
                        x: 662,
                        y: 204,
                        w: 34,
                        h: 38,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_1SKILLRIGHT",
                    0x1620_0051,
                    Rect {
                        x: 736,
                        y: 204,
                        w: 34,
                        h: 38,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_2SKILLLEFT",
                    0x1620_0052,
                    Rect {
                        x: 662,
                        y: 242,
                        w: 34,
                        h: 38,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_2SKILLRIGHT",
                    0x1620_0053,
                    Rect {
                        x: 736,
                        y: 242,
                        w: 34,
                        h: 38,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_3SKILLLEFT",
                    0x1620_0054,
                    Rect {
                        x: 662,
                        y: 280,
                        w: 34,
                        h: 38,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_3SKILLRIGHT",
                    0x1620_0055,
                    Rect {
                        x: 736,
                        y: 280,
                        w: 34,
                        h: 38,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_SPEED_LEFT",
                    0x1620_0064,
                    Rect {
                        x: 647,
                        y: 378,
                        w: 33,
                        h: 36,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_SPEED_RIGHT",
                    0x1620_0065,
                    Rect {
                        x: 736,
                        y: 378,
                        w: 33,
                        h: 36,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_MESSENGER",
                    0x1620_006A,
                    Rect {
                        x: 668,
                        y: 569,
                        w: 40,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAMA",
                    0x1620_1004,
                    Rect {
                        x: 652,
                        y: 65,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAMB",
                    0x1620_1005,
                    Rect {
                        x: 682,
                        y: 65,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAMC",
                    0x1620_1006,
                    Rect {
                        x: 712,
                        y: 65,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAMD",
                    0x1620_1007,
                    Rect {
                        x: 742,
                        y: 65,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAME",
                    0x1620_1008,
                    Rect {
                        x: 652,
                        y: 93,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAMF",
                    0x1620_1009,
                    Rect {
                        x: 682,
                        y: 93,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAMG",
                    0x1620_100A,
                    Rect {
                        x: 712,
                        y: 93,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TEAMH",
                    0x1620_100B,
                    Rect {
                        x: 742,
                        y: 93,
                        w: 24,
                        h: 24,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_CHAT_ALL",
                    0x1620_100E,
                    Rect {
                        x: 415,
                        y: 562,
                        w: 23,
                        h: 23,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_CHAT_FRIEND",
                    0x1620_100F,
                    Rect {
                        x: 441,
                        y: 562,
                        w: 23,
                        h: 23,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_CHAT_GUILD",
                    0x1620_1010,
                    Rect {
                        x: 467,
                        y: 562,
                        w: 23,
                        h: 23,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_CHAT_WHISPER",
                    0x1620_1011,
                    Rect {
                        x: 493,
                        y: 562,
                        w: 23,
                        h: 23,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_TOPEFFECT",
                    0x1620_1020,
                    Rect {
                        x: 719,
                        y: 450,
                        w: 60,
                        h: 48,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_BOTTOMEFFECT",
                    0x1620_1021,
                    Rect {
                        x: 719,
                        y: 507,
                        w: 60,
                        h: 48,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_MASTER_SKILLSEL",
                    0x1620_2056,
                    Rect {
                        x: 629,
                        y: 327,
                        w: 140,
                        h: 35,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_SKILLSEL",
                    0x1620_2057,
                    Rect {
                        x: 629,
                        y: 327,
                        w: 140,
                        h: 35,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_EXTEND1",
                    0x1601_5022,
                    Rect {
                        x: 37,
                        y: 55,
                        w: 123,
                        h: 150,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_EXTEND2",
                    0x1601_5023,
                    Rect {
                        x: 179,
                        y: 55,
                        w: 123,
                        h: 150,
                    },
                ),
                button(
                    "IDC_BUTTON_SWAIT_EXTEND5",
                    0x1601_5026,
                    Rect {
                        x: 37,
                        y: 245,
                        w: 123,
                        h: 150,
                    },
                ),
                button(
                    "IDC_IMAGE_SWAIT_COVERAVATAR1",
                    0x1650_003D,
                    Rect {
                        x: 28,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                ),
                button(
                    "IDC_IMAGE_SWAIT_COVERAVATAR2",
                    0x1650_003E,
                    Rect {
                        x: 170,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                ),
                button(
                    "IDC_IMAGE_SWAIT_COVERAVATAR5",
                    0x1650_0041,
                    Rect {
                        x: 28,
                        y: 242,
                        w: 141,
                        h: 175,
                    },
                ),
                button(
                    "IDC_SCROLL_SWAIT_CHAT",
                    0x1620_0038,
                    Rect {
                        x: 496,
                        y: 451,
                        w: 18,
                        h: 18,
                    },
                ),
                button(
                    "IDC_SCROLL_SWAIT_CHAT",
                    0x1620_003A,
                    Rect {
                        x: 496,
                        y: 469,
                        w: 18,
                        h: 18,
                    },
                ),
                button(
                    "IDC_SCROLL_SWAIT_CHAT",
                    0x1620_0039,
                    Rect {
                        x: 496,
                        y: 538,
                        w: 18,
                        h: 18,
                    },
                ),
            ],
            HashMap::new(),
        );
        game
    }

    /// Native centers of every staged waiting chrome control above,
    /// in staging order: the HOVER sweep visits each one.
    const WAITING_CHROME_POINTS: [(i32, i32); 40] = [
        (699, 144),
        (699, 144),
        (752, 581),
        (567, 517),
        (647, 549),
        (683, 549),
        (521, 26),
        (563, 573),
        (679, 223),
        (753, 223),
        (679, 261),
        (753, 261),
        (679, 299),
        (753, 299),
        (663, 396),
        (752, 396),
        (688, 581),
        (664, 77),
        (694, 77),
        (724, 77),
        (754, 77),
        (664, 105),
        (694, 105),
        (724, 105),
        (754, 105),
        (426, 573),
        (452, 573),
        (478, 573),
        (504, 573),
        (749, 474),
        (749, 531),
        (699, 344),
        (98, 130),
        (240, 130),
        (98, 320),
        (98, 140),
        (240, 140),
        (98, 329),
        (505, 478),
        (505, 547),
    ];

    #[test]
    fn waiting_chrome_sweep_arms_no_hover_state() {
        // Ticket #163 (waiting leg, HOVER stand-in): no waiting
        // chrome was ever hover-captured (`docs/hover.md` covers
        // room + planet only), and the shell stages no hover
        // resolver — motion over START/READY/BACK, every team
        // button, MUSELECT, the map pager, chat tabs, seats, covers,
        // or the scroll column arms nothing: the screen holds, no
        // send queues, and the frame paints pixel-identical.
        // Next evidence step: the SendInput hover sweep on the live
        // waiting chrome (same rig as `docs/hover.md`).
        let mut game = swept_waiting_game();
        let before = paint_waiting_frame(&mut game, 800, 600);
        for (x, y) in WAITING_CHROME_POINTS {
            game.ingest_snapshot(&InputSnapshot::motion_to(x, y));
            assert_eq!(game.screen(), Screen::Waiting);
        }
        assert!(!game.has_pending_team_send());
        assert!(!game.has_pending_music_send());
        let after = paint_waiting_frame(&mut game, 800, 600);
        assert_eq!(before, after, "hover paints no waiting chrome state");
    }

    #[test]
    fn waiting_start_press_fires_on_down_and_holds_on_release() {
        // Ticket #163 (waiting leg, CLICK press + release-inside):
        // the master START press fires on down — Waiting owns no
        // press-split (that evidence is planet-only, #111), so the
        // down-dispatch stands — and release-inside fires nothing
        // more: the scene holds Loading exactly once.
        // Next evidence step: the quickjoin-style release-inside
        // capture for lobby chrome (`docs/hover.md` press section).
        let mut game = swept_waiting_game();
        assert!(game.is_room_master());
        game.ingest_snapshot(&InputSnapshot::press_at(699, 144));
        assert_eq!(game.screen(), Screen::Loading);
        game.ingest_snapshot(&InputSnapshot::release_at(699, 144));
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn waiting_back_commits_on_down_and_holds_on_release_outside() {
        // Ticket #163 (waiting leg, CLICK press + release-outside):
        // BACK commits on down (the Room edge), so an outside
        // release cannot revert it — the scene holds Room. Same
        // down-dispatch stand-in as START (planet press-split OPEN).
        let mut game = swept_waiting_game();
        game.ingest_snapshot(&InputSnapshot::press_at(752, 581));
        assert_eq!(game.screen(), Screen::Room);
        game.ingest_snapshot(&InputSnapshot::release_at(10, 10));
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn waiting_ready_guest_press_arms_nothing_and_releases_clean() {
        // Ticket #163 (waiting leg, CLICK guest arm): a guest READY
        // press stays in Waiting (guests ready through the session
        // bridge, never Confirm) — down arms nothing, release
        // outside reverts nothing, and no send queues.
        let mut game = swept_waiting_game();
        game.waiting.as_mut().expect("joined room").member_id = 4;
        assert!(!game.is_room_master());
        game.ingest_snapshot(&InputSnapshot::press_at(699, 144));
        assert_eq!(game.screen(), Screen::Waiting);
        game.ingest_snapshot(&InputSnapshot::release_at(10, 10));
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(!game.has_pending_team_send());
        assert!(!game.has_pending_music_send());
    }

    #[test]
    fn waiting_team_sweep_queues_4004_then_4008_per_button() {
        // Ticket #163 (waiting leg, CLICK battle vs normal): every
        // TEAMA..H press lights its team and queues the
        // `SetRoomTeam` 4004 + `Ready` 4008 pair on down in both
        // Normal and Live rooms — the exe gate helper behind the
        // send (`FUN_00556680`) is untraced, so the shell sends in
        // both (OPEN, same as `select_team` documents).
        for live in [false, true] {
            let mut room = waiting_room();
            if live {
                room.game_mode = o2jam_net::GameMode::Live as u8;
            }
            let mut game = ScreenGame::new();
            game.set_joined_room(room);
            walk_to_waiting(&mut game);
            game.load_scene(
                Screen::Waiting,
                [
                    ("IDC_BUTTON_SWAIT_TEAMA", 0x1620_1004u32, (652, 65)),
                    ("IDC_BUTTON_SWAIT_TEAMB", 0x1620_1005u32, (682, 65)),
                    ("IDC_BUTTON_SWAIT_TEAMC", 0x1620_1006u32, (712, 65)),
                    ("IDC_BUTTON_SWAIT_TEAMD", 0x1620_1007u32, (742, 65)),
                    ("IDC_BUTTON_SWAIT_TEAME", 0x1620_1008u32, (652, 93)),
                    ("IDC_BUTTON_SWAIT_TEAMF", 0x1620_1009u32, (682, 93)),
                    ("IDC_BUTTON_SWAIT_TEAMG", 0x1620_100Au32, (712, 93)),
                    ("IDC_BUTTON_SWAIT_TEAMH", 0x1620_100Bu32, (742, 93)),
                ]
                .into_iter()
                .map(|(id, code, (x, y))| ScenePlacement {
                    role: PlacementRole::Single,
                    id: id.to_owned(),
                    code,
                    sprite: None,
                    rect: Rect { x, y, w: 24, h: 24 },
                })
                .collect(),
                HashMap::new(),
            );
            for (team, (x, y)) in [
                (664, 77),
                (694, 77),
                (724, 77),
                (754, 77),
                (664, 105),
                (694, 105),
                (724, 105),
                (754, 105),
            ]
            .into_iter()
            .enumerate()
            {
                let team = u8::try_from(team).expect("eight teams fit");
                game.ingest_snapshot(&InputSnapshot::click_at(x, y));
                assert_eq!(game.waiting.as_ref().expect("joined room").team, team);
                assert_eq!(game.battle_self_team, Some(team));
                assert_eq!(game.take_room_team(), Some(SetRoomTeam { team }));
                assert_eq!(game.take_team_ready(), Some(Ready));
                assert!(!game.has_pending_team_send());
            }
            assert_eq!(game.screen(), Screen::Waiting);
        }
    }

    #[test]
    fn waiting_seat_and_cover_clicks_land_nowhere_empty_or_populated() {
        // Ticket #163 (waiting leg, CLICK empty vs populated
        // seats): seat EXTEND plates and avatar covers are not
        // chrome — clicks on them select nothing in an empty room
        // or a populated one. The exe seat-click target (if any) is
        // unpinned; the shell holds the scene with no send.
        // Next evidence step: seat-click capture on live waiting
        // (empty vs occupied vs locked seats).
        for populated in [false, true] {
            let mut game = swept_waiting_game();
            if populated {
                let mut peer = waiting_room().slots[0].member.clone().expect("host seat");
                peer.master = false;
                game.waiting
                    .as_mut()
                    .expect("joined room")
                    .slots
                    .push(RoomSlot {
                        index: 1,
                        state_code: 1,
                        member: Some(peer),
                    });
            } else {
                game.waiting.as_mut().expect("joined room").slots.clear();
            }
            for (x, y) in [
                (98, 130),
                (240, 130),
                (98, 320),
                (98, 140),
                (240, 140),
                (98, 329),
            ] {
                game.ingest_snapshot(&InputSnapshot::click_at(x, y));
                assert_eq!(game.screen(), Screen::Waiting);
            }
            assert_eq!(game.waiting.as_ref().expect("joined room").team, 1);
            assert!(!game.has_pending_team_send());
            assert!(!game.has_pending_music_send());
        }
    }

    #[test]
    fn waiting_unwired_chrome_clicks_land_nowhere() {
        // Ticket #163 (waiting leg, CLICK labelled no-ops):
        // MUSELECT-open, the map pager, change-title, emotion, skill
        // pagers, speed stepper, messenger, effect plates, skill
        // select, chat tabs, and the chat scroll column stage no
        // handler — clicks hold Waiting with no send. Each names
        // the same next step: the SendInput sweep on the live
        // waiting board (`~/scratch/planet-verify/` recipe).
        let mut game = swept_waiting_game();
        for (x, y) in [
            (567, 517),
            (647, 549),
            (683, 549),
            (521, 26),
            (563, 573),
            (679, 223),
            (753, 223),
            (679, 261),
            (753, 261),
            (679, 299),
            (753, 299),
            (663, 396),
            (752, 396),
            (688, 581),
            (749, 474),
            (749, 531),
            (699, 344),
            (426, 573),
            (452, 573),
            (478, 573),
            (504, 573),
            (505, 460),
            (505, 478),
            (505, 547),
        ] {
            game.ingest_snapshot(&InputSnapshot::click_at(x, y));
            assert_eq!(
                game.screen(),
                Screen::Waiting,
                "click at ({x}, {y}) lands nowhere"
            );
        }
        assert!(!game.has_pending_team_send());
        assert!(!game.has_pending_music_send());
    }

    #[test]
    fn waiting_dialog_up_routes_rows_and_holds_chrome() {
        // Ticket #163 (waiting leg, CLICK dialog-up arm): with the
        // master song dialog staged, a search-table row click still
        // commits its song (4000), a seat-cover click still lands
        // nowhere, and the START pill still confirms into Loading —
        // the dialog consumes row hits first, chrome answers
        // everywhere else.
        let mut game = swept_waiting_game();
        game.load_select_music_scene(Vec::new(), HashMap::new());
        stage_waiting_song(&mut game);
        assert!(game.is_room_master());
        game.ingest_snapshot(&InputSnapshot::click_at(98, 140));
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(!game.has_pending_music_send());
        game.ingest_snapshot(&InputSnapshot::click_at(400, 150 + 23 + 11));
        assert_eq!(
            game.take_music_send(),
            Some(SetRoomMusic {
                music_id: 757,
                difficulty: 1,
                speed: 3
            }),
            "row click commits the 4000 pick"
        );
        game.ingest_snapshot(&InputSnapshot::click_at(699, 144));
        assert_eq!(game.screen(), Screen::Loading);
    }

    fn result_game() -> ScreenGame {
        let mut game = ScreenGame::new();
        for next in [
            Screen::Planet,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            game.flow.go(next).unwrap();
        }
        assert_eq!(game.screen(), Screen::Result);
        game
    }

    #[test]
    fn click_retries_result_into_waiting() {
        let mut game = result_game();
        // Pinned RETRY rect (625,570)-(708,592).
        game.ingest_snapshot(&InputSnapshot::click_at(666, 581));
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn click_leaves_result_into_rooms() {
        let mut game = result_game();
        // Pinned BACK rect (711,569)-(794,593).
        game.ingest_snapshot(&InputSnapshot::click_at(752, 581));
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn click_outside_result_buttons_keeps_result() {
        let mut game = result_game();
        game.ingest_snapshot(&InputSnapshot::click_at(400, 300));
        assert_eq!(game.screen(), Screen::Result);
    }

    #[test]
    fn channel_back_closes_to_the_planet_board() {
        // Esc symmetry with the verified X close (bnd-pinned rect
        // (541,151)-(589,189), verified native click (577,171)):
        // Back on the dialog-up phase returns to the bare board.
        // Exe key-code pin stays OPEN (rig next step in parity).
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn channel_list_paints_text_and_cursor() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        // 64x48 list: rows from y 18, text x 16; '2' top bar first.
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        // 64x48 list: rows from y 18, text x 16; the row text inks
        // near its origin, the amber cursor fills solid at x 10.
        assert!(white_near(&buf, 64, 48, (16, 18)));
        assert_eq!(buf[18 * 64 + 10], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[0], background_of(Screen::Channel));
    }

    fn two_rooms() -> Vec<RoomListEntry> {
        vec![
            RoomListEntry {
                number: 3,
                state_code: 1,
                title: "Jam".to_owned(),
                has_password: false,
                music_id: 0,
                difficulty: 0,
                key_mode: 2,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 5,
                min_level: 0,
                max_level: 0,
                skills: vec![],
                premium: false,
                room_type: 0,
            },
            RoomListEntry {
                number: 7,
                state_code: 1,
                title: "Locked".to_owned(),
                has_password: true,
                music_id: 0,
                difficulty: 0,
                key_mode: 2,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 1,
                min_level: 0,
                max_level: 0,
                skills: vec![],
                premium: false,
                room_type: 0,
            },
        ]
    }

    #[test]
    fn room_highlight_moves_with_bounds() {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Down, true);
        assert_eq!(game.room_highlight, 1);
        game.menu_key(MenuKey::Up, true);
        assert_eq!(game.room_highlight, 0);
    }

    #[test]
    fn room_highlight_selects_details_panel() {
        // 200x150 fits all nine fallback detail lines; the open room
        // paints OPEN dim while the locked room paints LOCK red.
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(super::MenuKey::Confirm, true);
        let mut buf = vec![0u32; 200 * 150];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0x90, 0x90, 0x90)));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0x50, 0x50)));
        game.menu_key(super::MenuKey::Down, true);
        let mut buf = vec![0u32; 200 * 150];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0x50, 0x50)));
        // List plus cursor keep working under the panel.
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn room_list_paints_rows_and_cursor() {
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(super::MenuKey::Confirm, true);
        // 64x48 list: rows from y 12, text x 16; '3' top bar first.
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        // 64x48 list: rows from y 12, text x 16; the row text inks
        // near its origin, the amber cursor fills solid at x 10.
        assert!(white_near(&buf, 64, 48, (16, 12)));
        assert_eq!(buf[12 * 64 + 10], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[0], background_of(Screen::Room));
    }

    fn waiting_room() -> JoinedRoom {
        let member = RoomMember {
            nickname: "host".to_owned(),
            level: 9,
            gender: 0,
            gem: 0,
            master: true,
            team: 0,
            ready: true,
            music_state: 0,
            equipment: [0; 17],
            music_ids: Vec::new(),
            cash_point: 0,
            free_pass: 0,
            playing_state: 0,
            administrator: false,
            super_manager: false,
        };
        JoinedRoom {
            member_id: 0,
            team: 1,
            title: "Jam".to_owned(),
            music_id: 0,
            arena: 0,
            key_mode: 2,
            game_mode: 0,
            difficulty: 0,
            speed: 1,
            has_super_manager: false,
            user_count: 2,
            slots: vec![RoomSlot {
                index: 0,
                state_code: 1,
                member: Some(member),
            }],
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    fn walk_to_waiting(game: &mut ScreenGame) {
        dismiss_splash_to_dialog(game);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn waiting_paints_joined_members_with_ready_flags() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 80 * 64];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Waiting));
        // Ready host sits at seat 1: green ready dot left of its row.
        let (x, y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(y + 1) as usize * 80 + (x - 6 + 1) as usize],
            pack_xrgb(0x30, 0xE0, 0x30)
        );
    }

    /// One solid test figure: a single pixel layer of one RGB555 word.
    fn solid_test_figure(word: [u8; 2]) -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&word, 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        figure
    }

    /// One solid red test figure (a single red pixel layer).
    fn waiting_test_figure() -> AvatarFigure {
        solid_test_figure([0x00, 0x7C])
    }

    /// Seat-0 avatar slot scene: the `EXTEND1` parent plus the avatar
    /// slot template, so the painter resolves the seat-0 slot box.
    fn waiting_seat_zero_scene() -> Vec<ScenePlacement> {
        vec![
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SWAIT_EXTEND1".to_owned(),
                code: 0x1601_5022,
                sprite: None,
                rect: Rect {
                    x: 37,
                    y: 55,
                    w: 123,
                    h: 150,
                },
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_IMAGE_SWAIT_AVATAR".to_owned(),
                code: 0x1670_002E,
                sprite: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 123,
                    h: 150,
                },
            },
        ]
    }

    /// Center pixel of the seat-0 slot box above: the 1x1 figure lands
    /// at parent origin + half the box (`paint_extent_centered`).
    const WAITING_SEAT_ZERO_PIXEL: (usize, usize) = (98, 129);

    #[test]
    fn waiting_staged_seat_beats_room_avatar_fallback() {
        // Dual self-figure source, intended fallback: the staged seat-0
        // figure (fetched-character equipment) wins over the lobby
        // `room_avatar`; without a staged seat the fallback covers
        // seat 0 instead.
        let red = pack_xrgb(0xFF, 0x00, 0x00);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let paint_seat = |game: &mut ScreenGame| {
            let mut buf = vec![0u32; 800 * 600];
            game.paint_at(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                0,
            );
            buf[WAITING_SEAT_ZERO_PIXEL.1 * 800 + WAITING_SEAT_ZERO_PIXEL.0]
        };
        let mut fallback_only = ScreenGame::new();
        fallback_only.set_joined_room(waiting_room());
        walk_to_waiting(&mut fallback_only);
        fallback_only.load_room_avatar(solid_test_figure([0x1F, 0x00]));
        fallback_only.load_scene(Screen::Waiting, waiting_seat_zero_scene(), HashMap::new());
        assert_eq!(paint_seat(&mut fallback_only), blue);
        let mut staged_wins = ScreenGame::new();
        staged_wins.set_joined_room(waiting_room());
        walk_to_waiting(&mut staged_wins);
        staged_wins.load_room_avatar(solid_test_figure([0x1F, 0x00]));
        staged_wins.set_waiting_avatar(0, waiting_test_figure());
        staged_wins.load_scene(Screen::Waiting, waiting_seat_zero_scene(), HashMap::new());
        assert_eq!(paint_seat(&mut staged_wins), red);
    }

    #[test]
    fn waiting_avatar_slots_key_figures_by_seat() {
        // Pre-stacked figures stage per seat: seats 0 and 1 hold,
        // seat 7 stays empty, and seat 8 (past the eight) is ignored.
        let mut game = ScreenGame::new();
        game.set_waiting_avatar(0, waiting_test_figure())
            .set_waiting_avatar(1, waiting_test_figure())
            .set_waiting_avatar(8, waiting_test_figure());
        assert!(game.waiting_avatar(0).is_some());
        assert!(game.waiting_avatar(1).is_some());
        assert!(game.waiting_avatar(7).is_none());
        assert!(game.waiting_avatar(8).is_none());
        // An empty figure never claims a slot.
        game.set_waiting_avatar(2, AvatarFigure::default());
        assert!(game.waiting_avatar(2).is_none());
    }

    #[test]
    fn stage_waiting_figures_without_decodable_archive_stages_nothing() {
        // The shared `load_worn_figure` path skips unresolvable
        // pieces instead of failing: junk bytes stage no seat.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let table = o2jam_file::AvatarItemTable { rows: Vec::new() };
        game.stage_waiting_figures(&[0x7F; 64], &table);
        assert!((0..8).all(|seat| game.waiting_avatar(seat).is_none()));
        // Without a joined room the stager is a no-op too.
        let mut bare = ScreenGame::new();
        bare.stage_waiting_figures(&[0x7F; 64], &table);
        assert!((0..8).all(|seat| bare.waiting_avatar(seat).is_none()));
    }

    #[test]
    fn join_restages_and_clears_stale_seat_figures() {
        // A new join drops the old seats, then restages through the
        // avatar source (the junk archive stages nothing, but the
        // hook runs the shared stager without failing).
        let mut game = ScreenGame::new();
        game.set_waiting_avatar(0, waiting_test_figure());
        assert!(game.waiting_avatar(0).is_some());
        let table = o2jam_file::AvatarItemTable { rows: Vec::new() };
        game.set_avatar_source(vec![0x7F; 64], table);
        assert!(game.waiting_avatar(0).is_some());
        game.set_joined_room(waiting_room());
        assert!((0..8).all(|seat| game.waiting_avatar(seat).is_none()));
    }

    #[test]
    fn waiting_without_room_paints_lobby_card() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 16 * 12];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 16,
                h: 12,
            },
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Waiting));
    }

    fn waiting_song_catalog_blob() -> Vec<u8> {
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&757u32.to_le_bytes());
        rec[20..24].copy_from_slice(&124.0f32.to_le_bytes());
        rec[112..117].copy_from_slice(b"First");
        rec[176..180].copy_from_slice(b"Band");
        rec[240..251].copy_from_slice(b"o2ma757.ojm");
        rec
    }

    fn waiting_song_selection() -> o2jam_net::play::SetRoomMusic {
        o2jam_net::play::SetRoomMusic {
            music_id: 757,
            difficulty: 2,
            speed: 3,
        }
    }

    fn stage_waiting_song(game: &mut ScreenGame) {
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_selected_music(waiting_song_selection());
    }

    #[test]
    fn chart_cover_path_maps_the_catalog_token_to_the_chart_file() {
        // The record's `.ojm` token names its chart file with the
        // `.ojn` extension under the install's Music folder (14/14
        // shipped tokens match); case-blind like the Windows client;
        // a non-token names no path.
        let numbered = chart_cover_path("o2ma757.ojm").unwrap();
        assert!(numbered
            .to_str()
            .is_some_and(|path| path.ends_with("Music/o2ma757.ojn")));
        let old = chart_cover_path("Tutorial.ojm").unwrap();
        assert!(old
            .to_str()
            .is_some_and(|path| path.ends_with("Music/Tutorial.ojn")));
        let upper = chart_cover_path("O2MA999.OJM").unwrap();
        assert!(upper
            .to_str()
            .is_some_and(|path| path.ends_with("Music/O2MA999.ojn")));
        assert!(chart_cover_path("bgEffect.zip").is_none());
    }

    #[test]
    fn cover_image_converts_jpeg_pixels_to_rgb555() {
        let decoded = o2jam_file::JpegRgbImage {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 0, 255, 0],
        };
        let image = cover_image(&decoded).unwrap();
        assert_eq!((image.w, image.h), (2, 1));
        assert_eq!(image.words, vec![0x7C00, 0x03E0]);
        let short = o2jam_file::JpegRgbImage {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 0, 255],
        };
        assert!(cover_image(&short).is_none());
    }

    #[test]
    fn selection_without_a_catalog_record_keeps_no_cover() {
        // The cover path runs through the staged catalog's container
        // token; a selection with no staged record resolves nothing
        // (regardless of the install), so the box stays as-is.
        let mut game = ScreenGame::new();
        game.set_selected_music(waiting_song_selection());
        assert!(game.song_cover().is_none());
    }

    #[test]
    fn staged_catalog_and_selection_load_the_walked_cover() {
        // End to end on a real install: the 757 token maps to
        // `Music/o2ma757.ojn`, whose OJN cover section decodes to the
        // 800x600 album art (docs/file-formats/ojn.md). Skips when
        // `install-root` is absent.
        let chart = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(_) = std::fs::read(&chart) else {
            return;
        };
        let mut game = ScreenGame::new();
        stage_waiting_song(&mut game);
        let cover = game.song_cover().expect("the walked cover resolves");
        assert_eq!((cover.w, cover.h), (800, 600));
    }

    #[test]
    fn result_paint_layers_the_cover_over_the_scene_ink() {
        // The full loop: the result scene paints, the cover fills the
        // box interior at its pinned rect (dest (0,0) samples the
        // cover's own (0,0)), driven by the staged catalog + selection.
        // Skips when `install-root` is absent.
        let chart = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&chart) else {
            return;
        };
        let cover_bytes = o2jam_file::ojn_cover_jpeg(&bytes).unwrap();
        let cover = cover_image(&o2jam_file::decode_ojn_cover(&cover_bytes).unwrap());
        let mut game = ScreenGame::new();
        stage_waiting_song(&mut game);
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            7_000,
        );
        assert_eq!(game.screen(), Screen::Result);
        let at =
            |x: i32, y: i32| buf[usize::try_from(y).unwrap() * 800 + usize::try_from(x).unwrap()];
        // The box interior's top-left pixel carries the cover's
        // top-left ink, widened to the frame (the wiring target).
        let word = cover.as_ref().expect("cover resolves").words[0];
        let (r, g, b) = (
            (((word >> 10) & 31) as u8) << 3 | (((word >> 10) & 31) as u8) >> 2,
            (((word >> 5) & 31) as u8) << 3 | (((word >> 5) & 31) as u8) >> 2,
            ((word & 31) as u8) << 3 | ((word & 31) as u8) >> 2,
        );
        assert_eq!(
            at(RESULT_MUSIC_BOX.x, RESULT_MUSIC_BOX.y),
            pack_xrgb(r, g, b)
        );
    }

    fn paint_waiting_frame(game: &mut ScreenGame, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            0,
        );
        buf
    }

    #[test]
    fn waiting_selected_song_paints_title_and_difficulty_rows() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // Panel sits below the seats: title `M...` at (10, 48),
        // detail `H...` at (10, 56); each line inks inside its row.
        assert!(ink_near(
            &buf,
            80,
            64,
            (10, 48),
            (20, 8),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
        assert!(ink_near(
            &buf,
            80,
            64,
            (10, 56),
            (20, 8),
            pack_xrgb(0xFF, 0xFF, 0xFF)
        ));
        // Seats keep painting under the panel.
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    /// Staged waiting covers plus the `MUSICNAME` top-bar rect
    /// (rects 60 and 22), no figures: TEXT-only stage for the song
    /// caption tests.
    fn musicname_scene() -> (Vec<ScenePlacement>, HashMap<String, AvatarFigure>) {
        (
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_IMAGE_SWAIT_COVERAVATAR1".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 28,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_TEXT_SWAIT_MUSICNAME".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 573,
                        y: 15,
                        w: 200,
                        h: 23,
                    },
                },
            ],
            HashMap::new(),
        )
    }

    /// Waiting game on the staged TEXT scene with the room naming
    /// song 757, catalog optional.
    fn musicname_game(with_catalog: bool) -> ScreenGame {
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.music_id = 757;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        if with_catalog {
            let blob = waiting_song_catalog_blob();
            let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
            game.set_song_catalog(&entries);
        }
        let (placements, figures) = musicname_scene();
        game.load_scene(Screen::Waiting, placements, figures);
        game
    }

    #[test]
    fn waiting_musicname_resolves_the_catalog_title() {
        // Room-fill string flow end to end: the staged catalog names
        // the room's song, so `MUSICNAME` inks the title scrim plus
        // white ink — and the same scene without a catalog inks a
        // different frame (the bare id), never an invented name.
        let mut titled = musicname_game(true);
        let mut bare = musicname_game(false);
        let hit = paint_waiting_frame(&mut titled, 800, 600);
        let miss = paint_waiting_frame(&mut bare, 800, 600);
        // Both ink the caption row white; the title and the bare id
        // differ, so the frames differ (paint.rs pins the exact
        // strings and the rect scrim).
        assert!(has_ink_near(&hit, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&miss, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert_ne!(hit, miss);
    }

    #[test]
    fn waiting_without_catalog_paints_art_without_rows() {
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        game.set_selected_music(waiting_song_selection());
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // Dialog art lands at the origin; the unresolved panel leaves
        // the bottom strip on the flat background.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[48 * 80 + 10], background_of(Screen::Waiting));
        assert_eq!(buf[56 * 80 + 10], background_of(Screen::Waiting));
    }

    #[test]
    fn select_music_structure_layers_over_waiting_seats() {
        // The dialog is modal: with its art staged, the structure's
        // table band covers the seat grid while the art and the
        // chosen-song panel stay visible on top.
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(white_near(&buf, 80, 64, (10, 48)));
        let (x, y) = seat_origin(1, 80, 64);
        assert_eq!(
            buf[(y + 1) as usize * 80 + (x - 6 + 1) as usize],
            select_scrim(),
            "the modal dialog structure covers the seats"
        );
    }

    #[test]
    fn set_song_catalog_stages_select_songs() {
        let mut game = ScreenGame::new();
        assert!(game.select_songs().is_empty());
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        assert_eq!(game.select_songs().len(), 1);
        assert_eq!(game.select_songs()[0].song_id, 757);
        assert_eq!(game.select_songs()[0].title, "First");
        assert_eq!(game.select_songs()[0].artist, "Band");
    }

    #[test]
    fn waiting_captions_carry_catalog_bpm_and_inventory_counts() {
        // Room song 757 stages the catalog BPM column (124.0); the
        // inventory stages alongside, zeroed until set.
        let mut game = ScreenGame::new();
        assert_eq!(game.skill_counts(), [0, 0, 0]);
        let mut room = waiting_room();
        room.music_id = 757;
        game.set_joined_room(room);
        assert_eq!(game.room_song_bpm(), None);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_skill_counts([7, 42, 1500]);
        assert_eq!(game.room_song_bpm(), Some(124.0));
        assert_eq!(game.catalog_song_bpm(757), Some(124.0));
        assert_eq!(game.catalog_song_bpm(999), None);
        assert_eq!(game.skill_counts(), [7, 42, 1500]);
    }

    #[test]
    fn waiting_staged_captions_ink_their_rects() {
        // Staged waiting covers plus three caption rects: the map
        // name (room word 1), the catalog BPM (124), and the first
        // inventory count ink their rects.
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.music_id = 757;
        room.arena = 1;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_skill_counts([7, 42, 1500]);
        game.load_scene(
            Screen::Waiting,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_IMAGE_SWAIT_COVERAVATAR1".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 28,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_TEXT_SWAIT_MAPNAME".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 480,
                        y: 15,
                        w: 90,
                        h: 23,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_TEXT_SWAIT_BPM".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 620,
                        y: 60,
                        w: 60,
                        h: 20,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_TEXT_SWAIT_1SKILLCOUNT".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 740,
                        y: 208,
                        w: 40,
                        h: 30,
                    },
                },
            ],
            HashMap::new(),
        );
        let buf = paint_waiting_frame(&mut game, 800, 600);
        let scrim = pack_xrgb(0x08, 0x08, 0x10);
        assert_eq!(buf[13 * 800 + 476], scrim, "mapname scrim");
        assert_eq!(buf[58 * 800 + 616], scrim, "bpm scrim");
        assert_eq!(buf[206 * 800 + 736], scrim, "skillcount scrim");
        assert!(
            has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)),
            "caption ink paints"
        );
    }

    /// Waiting game with dialog art and the one-song catalog staged:
    /// the structure-paint scenario.
    fn select_structure_game() -> ScreenGame {
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        stage_waiting_song(&mut game);
        game
    }

    #[test]
    fn select_structure_paints_regions_over_dialog_art() {
        let mut game = select_structure_game();
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        // Staged art stays visible behind the structure.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        // Filter column one: header scrim plus EASY button at its
        // reference pitch (129 + 33).
        assert_eq!(buf[95 * 800 + 30], select_scrim());
        assert_eq!(buf[162 * 800 + 30], select_scrim());
        // Row one: green E letter at the icon column (text inks
        // inside the band), white title ink at the TITLE column,
        // unknown dashes at LVL and GRADE.
        assert!(ink_near(
            &buf,
            800,
            600,
            (392, 150),
            (10, 16),
            pack_xrgb(0x30, 0xE0, 0x30)
        ));
        assert!(white_near(&buf, 800, 600, (417, 150)));
        assert!(ink_near(
            &buf,
            800,
            600,
            (628, 152),
            (12, 16),
            select_dim_ink()
        ));
        assert!(ink_near(
            &buf,
            800,
            600,
            (716, 152),
            (12, 16),
            select_dim_ink()
        ));
        // Search tab (amber caption), status banner, pager, and the
        // centered OK caption.
        assert!(ink_near(
            &buf,
            800,
            600,
            (355, 95),
            (40, 16),
            select_active_ink()
        ));
        assert!(white_near(&buf, 800, 600, (346, 398)));
        assert!(white_near(&buf, 800, 600, (744, 398)));
        assert!(white_near(&buf, 800, 600, (380, 523)));
        // Chosung letter A (row 1, cell 14) at its grid slot.
        assert!(white_near(&buf, 800, 600, (563, 444)));
    }

    #[test]
    fn select_structure_paints_without_catalog_rows() {
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        // The zero-row search tab still counts (amber ink); no row
        // band paints, so the row area keeps the waiting backdrop.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(ink_near(
            &buf,
            800,
            600,
            (355, 95),
            (40, 16),
            select_active_ink()
        ));
        assert_eq!(
            buf[160 * 800 + 400],
            super::super::paint::background_of(Screen::Waiting)
        );
    }

    #[test]
    fn select_structure_stays_hidden_without_dialog_art() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        stage_waiting_song(&mut game);
        let buf = paint_waiting_frame(&mut game, 80, 64);
        // No dialog art staged: the structure never paints (the
        // scaled filter-column anchor stays waiting background).
        assert_eq!(
            buf[10 * 80 + 2],
            super::super::paint::background_of(Screen::Waiting)
        );
    }

    fn myroom_test_table() -> o2jam_file::AvatarItemTable {
        let row = |item_id: u32, item_type: u32, slot_byte: u8| o2jam_file::AvatarItemRow {
            item_id,
            item_type,
            price: 100,
            params: [0, 0, u32::from(slot_byte) << 24, 0],
            flags: (0, 0),
            name: format!("Gear{item_id}"),
            description: String::new(),
            sprites: Vec::new(),
        };
        // Type 2 rows ride record slots 0..11; the type-5 row pins
        // the exe's force-slot-11 arm (`docs/equip.md`).
        let mut rows: Vec<o2jam_file::AvatarItemRow> = (1..=12)
            .map(|id| row(id, 2, u8::try_from(id - 1).expect("twelve rows")))
            .collect();
        rows.push(row(13, 5, 0));
        o2jam_file::AvatarItemTable { rows }
    }

    fn myroom_test_character(inventory: Vec<i32>) -> CharacterInfo {
        CharacterInfo {
            nickname: "Tester".to_owned(),
            level: 12,
            experience: 3450,
            inventory,
            ..CharacterInfo::default()
        }
    }

    fn myroom_hit_placement(code: u32, x: i32, y: i32) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w: 40, h: 20 },
        }
    }

    fn myroom_open_overlay() -> ScreenGame {
        use super::super::side::{
            MYROOM_APPLY_CODE, MYROOM_BACK_CODE, MYROOM_BAG_FIRST_CODE, MYROOM_BAG_LEFT_CODE,
            MYROOM_BAG_RIGHT_CODE, MYROOM_GIFT_FIRST_CODE, MYROOM_GIFT_LEFT_CODE,
            MYROOM_GIFT_RIGHT_CODE, MYROOM_GOSHOP_CODE, MYROOM_MOVE_CODE, MYROOM_NICKNAME_CODE,
            MYROOM_SLOT_FIRST_CODE, MYROOM_VACATE_CODE,
        };
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Waiting);
        let table = myroom_test_table();
        game.set_myroom_items(table);
        game.set_myroom_character(myroom_test_character((1..=12).collect()));
        let placements = vec![
            myroom_hit_placement(MYROOM_SLOT_FIRST_CODE + 3, 10, 10),
            myroom_hit_placement(MYROOM_BAG_LEFT_CODE, 100, 10),
            myroom_hit_placement(MYROOM_BAG_RIGHT_CODE, 150, 10),
            myroom_hit_placement(MYROOM_BAG_FIRST_CODE, 200, 10),
            myroom_hit_placement(MYROOM_BAG_FIRST_CODE + 1, 250, 10),
            myroom_hit_placement(MYROOM_GIFT_LEFT_CODE, 300, 10),
            myroom_hit_placement(MYROOM_GIFT_RIGHT_CODE, 350, 10),
            myroom_hit_placement(MYROOM_GIFT_FIRST_CODE, 400, 10),
            myroom_hit_placement(MYROOM_APPLY_CODE, 450, 10),
            myroom_hit_placement(MYROOM_MOVE_CODE, 500, 10),
            myroom_hit_placement(MYROOM_VACATE_CODE, 10, 40),
            myroom_hit_placement(MYROOM_GOSHOP_CODE, 60, 40),
            myroom_hit_placement(MYROOM_NICKNAME_CODE, 10, 100),
            myroom_hit_placement(MYROOM_BACK_CODE, 550, 10),
        ];
        game.load_side_scene(placements, HashMap::new());
        assert!(game.visit_side(SideScreen::MyRoom));
        game
    }

    #[test]
    fn myroom_slot_click_selects_through_staged_rect() {
        let mut game = myroom_open_overlay();
        assert_eq!(game.myroom().selected_slot(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(11, 11));
        assert_eq!(game.myroom().selected_slot(), 3);
        // Outside every staged placement the selection holds.
        game.ingest_snapshot(&InputSnapshot::click_at(0, 500));
        assert_eq!(game.myroom().selected_slot(), 3);
    }

    #[test]
    fn myroom_bag_pager_turns_over_staged_inventory() {
        let mut game = myroom_open_overlay();
        assert_eq!(game.myroom().bag_page(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(151, 11));
        assert_eq!(game.myroom().bag_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(151, 11));
        assert_eq!(game.myroom().bag_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(101, 11));
        assert_eq!(game.myroom().bag_page(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(101, 11));
        assert_eq!(game.myroom().bag_page(), 0);
    }

    #[test]
    fn myroom_equip_queues_5004_with_derived_slot() {
        use super::super::side::MYROOM_BAG_FIRST_CODE;
        let mut game = myroom_open_overlay();
        // Pick the first bag cell (item 1, type 2, record slot 0).
        game.ingest_snapshot(&InputSnapshot::click_at(201, 11));
        assert_eq!(game.myroom().bag_cell(), Some(0));
        game.ingest_snapshot(&InputSnapshot::click_at(451, 11));
        let send = game.take_equip_send().expect("apply queues the equip send");
        assert_eq!((send.slot, send.item_id), (0, 1));
        assert!(game.take_equip_send().is_none());
        // The staged worn set mirrors the commit (slot 0 wears 1).
        game.ingest_snapshot(&InputSnapshot::click_at(251, 11));
        game.ingest_snapshot(&InputSnapshot::click_at(451, 11));
        let second = game.take_equip_send().expect("second apply queues");
        assert_eq!((second.slot, second.item_id), (1, 2));
        let _ = MYROOM_BAG_FIRST_CODE;
    }

    /// Highlighted channel row (the keyboard Confirm target).
    #[test]
    fn highlighted_channel_names_the_confirm_target() {
        let mut game = ScreenGame::new();
        assert_eq!(game.highlighted_channel(), None);
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(
            game.highlighted_channel().map(|row| row.channel_id),
            Some(2)
        );
        game.menu_key(MenuKey::Down, true);
        assert_eq!(
            game.highlighted_channel().map(|row| row.channel_id),
            Some(9)
        );
    }

    /// Mouse entry target: only an enter-face click names its row.
    #[test]
    fn channel_enter_at_names_only_face_clicks() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        // Row 0 enter face (ticket #56: (502,236)-(546,257)).
        assert_eq!(
            game.channel_enter_at(524, 246).map(|row| row.channel_id),
            Some(2)
        );
        // Row bodies highlight only, never enter.
        assert_eq!(game.channel_enter_at(300, 240), None);
        assert_eq!(game.channel_enter_at(300, 290), None);
        // Off the rows entirely.
        assert_eq!(game.channel_enter_at(10, 10), None);
        // The X chrome closes instead of entering.
        assert_eq!(game.channel_enter_at(577, 171), None);
    }

    /// Minimal sprite55 body (mirrors the avatar-figure builders): a
    /// 1x1 red pixel with a black colorkey.
    fn worn_test_sprite() -> Vec<u8> {
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x7C]);
        blob
    }

    /// Minimal `.opa` image (header + bodies + trailing entry table;
    /// mirrors the avatar-figure builders).
    fn worn_test_archive(bodies: &[(&str, &[u8])]) -> Vec<u8> {
        use o2jam_file::{
            OPI_ENTRY_LEN, OPI_HEADER_LEN, OPI_NAME_OFF, OPI_OFFSET_OFF, OPI_RESERVED_OFF,
            OPI_SIZE_OFF,
        };
        let mut blob = vec![0u8; OPI_HEADER_LEN];
        blob[..4].copy_from_slice(&1u32.to_le_bytes());
        blob[4..8].copy_from_slice(&(bodies.len() as u32).to_le_bytes());
        let mut off = OPI_HEADER_LEN as u32;
        let mut table = Vec::new();
        for (name, body) in bodies {
            let mut entry = vec![0u8; OPI_ENTRY_LEN];
            entry[..4].copy_from_slice(&1u32.to_le_bytes());
            entry[OPI_NAME_OFF..OPI_NAME_OFF + name.len()].copy_from_slice(name.as_bytes());
            entry[OPI_OFFSET_OFF..OPI_OFFSET_OFF + 4].copy_from_slice(&off.to_le_bytes());
            entry[OPI_SIZE_OFF..OPI_SIZE_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            entry[OPI_RESERVED_OFF..OPI_RESERVED_OFF + 4]
                .copy_from_slice(&(body.len() as u32).to_le_bytes());
            table.push(entry);
            off += body.len() as u32;
        }
        for (_, body) in bodies {
            blob.extend_from_slice(body);
        }
        for entry in table {
            blob.extend_from_slice(&entry);
        }
        blob
    }

    /// One wearable test row: item 1, type 2, record slot 0, with
    /// male- and female-parity wear sprites resolving into the test
    /// archive.
    fn worn_test_table() -> o2jam_file::AvatarItemTable {
        o2jam_file::AvatarItemTable {
            rows: vec![o2jam_file::AvatarItemRow {
                item_id: 1,
                item_type: 2,
                price: 100,
                params: [0, 0, 0, 0],
                flags: (0, 0),
                name: "Gear1".to_owned(),
                description: String::new(),
                sprites: vec![
                    (1, "icon.oji".to_owned()),
                    (1, "icon.oji".to_owned()),
                    (1, "01F01001.oji".to_owned()),
                    (1, "01F01001.oji".to_owned()),
                ],
            }],
        }
    }

    /// Ticket #30 stale-figure leftover: applying equipment must
    /// re-stack the Room lobby figure, not just the worn table.
    #[test]
    fn myroom_apply_restages_the_room_avatar() {
        let sprite = worn_test_sprite();
        let archive = worn_test_archive(&[("01F01001.ojs", &sprite)]);
        let table = worn_test_table();
        let mut game = ScreenGame::new();
        game.set_myroom_items(table.clone());
        game.set_myroom_character(myroom_test_character(vec![1]));
        game.set_avatar_source(archive, table);
        game.load_room_avatar(solid_test_figure([0x1F, 0x00]));
        let paint_first_pixel = |figure: &AvatarFigure| {
            let mut buf = vec![0u32; 1];
            figure.paint(
                &mut o2jam_render::Frame {
                    buf: &mut buf,
                    w: 1,
                    h: 1,
                },
                0,
                0,
            );
            buf[0]
        };
        let before = game.room_avatar.clone().expect("stale figure staged");
        // Pick the first bag cell (item 1) and apply it.
        game.myroom.pick_bag_cell(Some(0));
        game.apply_myroom_equip();
        assert_eq!(
            game.myroom_character
                .as_ref()
                .expect("staged character")
                .equipment[0],
            1
        );
        let after = game
            .room_avatar
            .clone()
            .expect("equip restages the room figure");
        assert_ne!(
            paint_first_pixel(&after),
            paint_first_pixel(&before),
            "the lobby figure follows the new equipment"
        );
        assert_eq!(
            paint_first_pixel(&after),
            o2jam_render::pack_xrgb(255, 0, 0)
        );
    }

    #[test]
    fn myroom_gift_claim_queues_5023_for_the_picked_cell() {
        use super::super::side::MYROOM_GIFT_FIRST_CODE;
        use o2jam_net::gift::GiftType;
        let mut game = myroom_open_overlay();
        let gift = o2jam_net::GiftItemInfo {
            gift_id: 7,
            item_id: 1,
            sender: "Giver".to_owned(),
        };
        game.myroom_character
            .as_mut()
            .expect("staged character")
            .item_gifts
            .push(gift);
        game.ingest_snapshot(&InputSnapshot::click_at(401, 11));
        assert_eq!(game.myroom().gift_cell(), Some(0));
        game.ingest_snapshot(&InputSnapshot::click_at(501, 11));
        let claim = game.take_gift_claim().expect("MOVE queues the gift claim");
        assert_eq!(claim.gift_type, GiftType::Item);
        assert_eq!((claim.gift_id, claim.resource_id), (7, 1));
        assert!(game.take_gift_claim().is_none());
        let _ = MYROOM_GIFT_FIRST_CODE;
    }

    #[test]
    fn myroom_entry_queues_gift_list_fetch() {
        let mut game = myroom_open_overlay();
        assert!(game.take_gift_list_request().is_some());
        assert!(game.take_gift_list_request().is_none());
    }

    #[test]
    fn myroom_captions_paint_into_their_bnd_rects() {
        // The nickname TEXT placement inks its own rect; the beside
        // rect stays blank; without staged data nothing paints.
        let mut game = myroom_open_overlay();
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        let inks = |buf: &[u32], x: i32, y: i32, w: i32| -> bool {
            // Single-row span (no row-boundary wrap): ink inside the
            // rect reads back, the beside span stays black.
            buf[(y * 800 + x) as usize..(y * 800 + x + w) as usize]
                .iter()
                .any(|pixel| *pixel != 0)
        };
        assert!(inks(&buf, 10, 100, 40), "nickname rect stays blank");
        assert!(!inks(&buf, 60, 100, 40), "caption ink leaked");
        let mut bare = ScreenGame::new();
        walk_to_lobby(&mut bare, Screen::Waiting);
        bare.load_side_scene(Vec::new(), HashMap::new());
        assert!(bare.visit_side(SideScreen::MyRoom));
        let mut bare_buf = vec![0u32; 800 * 600];
        bare.paint_at(
            &mut Frame {
                buf: &mut bare_buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(bare_buf.iter().all(|pixel| *pixel == 0));
    }

    fn payment_hit_placement(code: u32, x: i32, y: i32, w: u32, h: u32) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        }
    }

    fn payment_open_overlay() -> ScreenGame {
        // Two staged billing rects: the notice must ink the
        // larger one and leave the small one blank.
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Waiting);
        game.load_side_scene(
            vec![
                payment_hit_placement(0x0010_0001, 10, 10, 40, 20),
                payment_hit_placement(0x0010_0002, 100, 100, 200, 60),
            ],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Payment));
        game
    }

    fn region_has_ink(buf: &[u32], frame_w: usize, x: i32, y: i32, w: i32, h: i32) -> bool {
        buf.chunks(frame_w)
            .skip(y as usize)
            .take(h as usize)
            .any(|row| {
                row.iter()
                    .skip(x as usize)
                    .take(w as usize)
                    .any(|pixel| *pixel != 0)
            })
    }

    #[test]
    fn payment_side_paints_the_unwired_billing_notice() {
        // The billing views complete no web call, so the side
        // screen inks the unwired notice into its largest staged
        // rect instead of painting art alone.
        let mut game = payment_open_overlay();
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(
            region_has_ink(&buf, 800, 100, 100, 200, 60),
            "notice missing from the largest staged rect"
        );
        assert!(
            !region_has_ink(&buf, 800, 10, 10, 40, 20),
            "notice leaked outside the largest staged rect"
        );
    }

    #[test]
    fn payment_side_without_staged_rects_paints_no_notice() {
        // No staged geometry: the miss path paints nothing
        // (ADR-0003 — never a literal fallback rect).
        let mut bare = ScreenGame::new();
        walk_to_lobby(&mut bare, Screen::Waiting);
        bare.load_side_scene(Vec::new(), HashMap::new());
        assert!(bare.visit_side(SideScreen::Payment));
        let mut bare_buf = vec![0u32; 800 * 600];
        bare.paint_at(
            &mut Frame {
                buf: &mut bare_buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(bare_buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn payment_entry_queues_no_wire_call() {
        // Billing completes no calls: visiting either billing view
        // stages nothing on any send seam (the run stays silent).
        let mut game = payment_open_overlay();
        assert!(game.take_equip_send().is_none());
        assert!(game.take_gift_claim().is_none());
        assert!(game.take_gift_list_request().is_none());
        assert!(game.take_create_room().is_none());
        assert!(game.take_join().is_none());
        assert!(game.take_music_send().is_none());
        assert!(game.take_room_team().is_none());
        assert!(game.take_team_ready().is_none());
    }

    #[test]
    fn payment_tutorial_hand_session_visits_both_and_returns() {
        // Hand-driven billing/tutorial pass (trace
        // `fixtures/input-trace/payment-tutorial-hand-session.json`,
        // smoke note `docs/smoke/payment-tutorial-hand-session.md`):
        // Waiting opens the Payment overlay (notice ink proven by
        // `payment_side_paints_the_unwired_billing_notice`, silence
        // by `payment_entry_queues_no_wire_call`), BACK returns to
        // the lobby, then the Tutorial overlay opens over its
        // staged backdrop and BACK returns again.
        let mut game = payment_open_overlay();
        assert_eq!(game.side(), Some(SideScreen::Payment));
        assert!(game.leave_side());
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Waiting);
        game.load_side_scene(Vec::new(), HashMap::new());
        assert!(game.visit_side(SideScreen::Tutorial));
        assert_eq!(game.side(), Some(SideScreen::Tutorial));
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(game.leave_side());
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn myroom_hand_session_walks_slots_bag_and_gifts() {
        // Hand-driven avatar-room pass (trace
        // `fixtures/input-trace/myroom-hand-session.json`, smoke note
        // `docs/smoke/myroom-hand-session.md`): one overlay visit
        // walks slot selection, bag paging, equip commit, gift paging
        // and gift claim, then BACK returns to the lobby. Server
        // round trips ride a scripted stub behind the take-seam and
        // stay labeled ASSUMED, never exe proof.
        use o2jam_net::gift::GiftType;
        let mut game = myroom_open_overlay();
        game.myroom_character
            .as_mut()
            .expect("staged character")
            .item_gifts
            .push(o2jam_net::GiftItemInfo {
                gift_id: 7,
                item_id: 1,
                sender: "Giver".to_owned(),
            });
        // Slot pick: the staged slot button (code ordinal 3) at its
        // staged rect records the selection.
        game.ingest_snapshot(&InputSnapshot::click_at(11, 11));
        assert_eq!(game.myroom().selected_slot(), 3);
        // Bag pager: 12 owned rows turn exactly one page.
        game.ingest_snapshot(&InputSnapshot::click_at(151, 11));
        assert_eq!(game.myroom().bag_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(101, 11));
        assert_eq!(game.myroom().bag_page(), 0);
        // Equip: first bag cell (item 1, record slot 0) applies.
        game.ingest_snapshot(&InputSnapshot::click_at(201, 11));
        game.ingest_snapshot(&InputSnapshot::click_at(451, 11));
        let equip = game.take_equip_send().expect("equip queued");
        assert_eq!((equip.slot, equip.item_id), (0, 1));
        // Gift: first box cell claims item gift 7 from Giver.
        game.ingest_snapshot(&InputSnapshot::click_at(401, 11));
        game.ingest_snapshot(&InputSnapshot::click_at(501, 11));
        let claim = game.take_gift_claim().expect("claim queued");
        assert_eq!(claim.gift_type, GiftType::Item);
        assert_eq!((claim.gift_id, claim.resource_id), (7, 1));
        // BACK returns to the lobby that opened the room.
        game.ingest_snapshot(&InputSnapshot::click_at(551, 11));
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn myroom_back_click_returns_to_the_lobby() {
        let mut game = myroom_open_overlay();
        game.ingest_snapshot(&InputSnapshot::click_at(551, 11));
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn avi_paints_staged_logo_without_a_splash_slot() {
        use crate::avatar::FigureLayer;
        // One staging path: the bound `STATE_AVI` placement carries
        // the logo (`IDC_BACK_LOGO` at its `.bnd` rect). No second
        // splash slot paints on top.
        let mut logo = AvatarFigure::default();
        logo.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut game = ScreenGame::new();
        game.load_scene(
            Screen::Avi,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BACK_LOGO".to_owned(),
                code: 0x1810_0001,
                sprite: Some("O2mediaLogo.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            }],
            HashMap::from([("O2mediaLogo.ojs".to_owned(), logo)]),
        );
        let mut buf = vec![0u32; 8 * 8];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(
            buf[63], 0,
            "staged art paints no flat fill (pinned no-fill rule)"
        );
    }

    #[test]
    fn lane_key_dismisses_splash_into_planet_board() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        let mut buf = vec![0u32; 16 * 12];
        game.paint(&mut Frame {
            buf: &mut buf,
            w: 16,
            h: 12,
        });
        assert_eq!(buf[0], background_of(Screen::Planet));
    }

    #[test]
    fn lane_release_keeps_splash() {
        let mut game = ScreenGame::new();
        game.lane_key(0, false);
        assert_eq!(game.screen(), Screen::Avi);
    }

    fn paint_splash_frame(game: &mut ScreenGame, now_ms: u64) {
        let mut buf = vec![0u32; 16 * 12];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 16,
                h: 12,
            },
            now_ms,
        );
    }

    #[test]
    fn splash_stays_before_timeout() {
        let mut game = ScreenGame::new();
        paint_splash_frame(&mut game, SPLASH_AUTO_ADVANCE_MS - 1);
        assert_eq!(game.screen(), Screen::Avi);
    }

    #[test]
    fn splash_auto_advances_at_timeout() {
        let mut game = ScreenGame::new();
        paint_splash_frame(&mut game, SPLASH_AUTO_ADVANCE_MS);
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn splash_auto_advances_after_timeout() {
        let mut game = ScreenGame::new();
        paint_splash_frame(&mut game, SPLASH_AUTO_ADVANCE_MS + 1_000);
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn splash_scene_polls_its_wall_hold_at_a_modest_gap() {
        // Ticket #138: the splash auto-advance rides the wall clock,
        // so the window owes one modest wakeup per hold check — not
        // the loop's 60 Hz tick — while the logo sits untouched.
        let game = ScreenGame::new();
        assert_eq!(game.screen(), Screen::Avi);
        assert_eq!(
            GameLoop::idle_repaint_interval(&game),
            Some(std::time::Duration::from_millis(SPLASH_HOLD_POLL_MS))
        );
    }

    #[test]
    fn fixed_clock_splash_keeps_the_tick_pace() {
        // Trace replays drive the virtual clock per paint, so the
        // windowed replay keeps the tick pace the headless seam
        // always drove (no replay slows down under #138).
        let mut game = ScreenGame::new();
        game.enable_fixed_clock(16);
        assert_eq!(
            GameLoop::idle_repaint_interval(&game),
            Some(std::time::Duration::from_nanos(crate::TICK_NANOS))
        );
    }

    #[test]
    fn playing_scene_holds_the_sixty_hertz_tick() {
        // Song scroll, receptor flash, and the end-of-song Result edge
        // all ride the sample clock per paint: Playing keeps ticking.
        let mut game = ScreenGame::new();
        walk_to_playing(&mut game);
        assert_eq!(
            GameLoop::idle_repaint_interval(&game),
            Some(std::time::Duration::from_nanos(crate::TICK_NANOS))
        );
    }

    #[test]
    fn settled_scenes_idle_without_a_timer() {
        // Every other scene is event-driven (input, resizes, and the
        // session layer's lobby drains): the game sets no timer of its
        // own, so the windowed loop parks on these boards. The walk
        // follows the pinned menu path (`menu_walks_lobbies_to_playing`).
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(GameLoop::idle_repaint_interval(&game), None);
        for (key, screen) in [
            (MenuKey::Confirm, Screen::Channel),
            (MenuKey::Confirm, Screen::Room),
            (MenuKey::Confirm, Screen::Waiting),
            (MenuKey::Confirm, Screen::Loading),
        ] {
            game.menu_key(key, true);
            assert_eq!(game.screen(), screen);
            assert_eq!(
                GameLoop::idle_repaint_interval(&game),
                None,
                "{screen:?} must idle without a timer"
            );
        }
        // Side overlays ride the lobby screen underneath: no timer
        // while the shop sheet sits over Waiting.
        assert!(game.enter_screen(Screen::Waiting));
        assert!(game.visit_side(super::super::side::SideScreen::Shop));
        assert_eq!(GameLoop::idle_repaint_interval(&game), None);
        assert!(game.leave_side());
    }

    #[test]
    fn zero_hold_advances_on_first_windowed_paint() {
        // Ticket #54: the windowed auto-advance rides the wall
        // clock, so a zero hold leaves the splash on the first
        // windowed paint even with no audio and no fixed clock.
        let mut game = ScreenGame::new();
        game.set_splash_hold_ms(0);
        assert_eq!(game.screen(), Screen::Avi);
        let mut buf = vec![0u32; 16 * 12];
        game.paint(&mut Frame {
            buf: &mut buf,
            w: 16,
            h: 12,
        });
        assert_eq!(game.screen(), Screen::Planet);
    }

    /// Backend stub shaped like the 3.9 s logo once-track every real
    /// backend caps at: `samples_played` parks at the track end
    /// (3.9 s of 44.1 kHz frames) and never moves again, so the song
    /// clock reads 3900 ms forever — the #54 freeze shape.
    struct FinishedOnceTrack {
        inner: o2jam_audio::NullBackend,
    }

    impl FinishedOnceTrack {
        /// 3.9 s of frames at 44.1 kHz (the `bgLogo.ogg` length pinned
        /// in the STATE_AVI Timing row).
        const TRACK_END_FRAMES: u64 = 171_990;
    }

    impl AudioBackend for FinishedOnceTrack {
        fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), o2jam_audio::AudioError> {
            self.inner.play_pcm(pcm)
        }

        fn set_scene_bgm(
            &mut self,
            pcm: Option<&PcmSamples>,
            repeat: bool,
        ) -> Result<(), o2jam_audio::AudioError> {
            self.inner.set_scene_bgm(pcm, repeat)
        }

        fn scene_bgm_playing(&self) -> bool {
            self.inner.scene_bgm_playing()
        }

        fn samples_played(&self) -> u64 {
            Self::TRACK_END_FRAMES
        }
    }

    #[test]
    fn default_hold_outlasts_a_shorter_once_track_on_the_wall_clock() {
        // Ticket #59: the default 5 s hold plus a non-repeating track
        // shorter than the hold (the 3.9 s logo) still advances — the
        // hold rides the wall clock (claims B38) while song position
        // rides the sample clock. No input, no audio device; condition
        // polling only, no sleeps (the #52 flake lesson).
        let mut game = ScreenGame::new();
        game.with_audio(
            Box::new(FinishedOnceTrack {
                inner: o2jam_audio::NullBackend::new(),
            }),
            HashMap::new(),
        );
        game.stage_scene_bgm(marker_bank(&[8]));
        assert_eq!(game.screen(), Screen::Avi);
        // The song clock is parked at the track end, short of the
        // default hold: only the wall clock can advance the splash.
        assert!(game.now_ms() < game.splash_hold_ms);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut buf = vec![0u32; 16 * 12];
        while game.screen() == Screen::Avi {
            assert!(
                std::time::Instant::now() < deadline,
                "splash never advanced past the 3.9 s track end"
            );
            game.paint(&mut Frame {
                buf: &mut buf,
                w: 16,
                h: 12,
            });
            std::thread::yield_now();
        }
        assert_eq!(game.screen(), Screen::Planet);
    }

    /// Boot onto the staged logo splash: the bound `STATE_AVI`
    /// record (`IDC_BACK_LOGO`, full-bleed) with a one-pixel figure —
    /// the shared start every splash-edge guard walks.
    fn staged_splash_boot() -> ScreenGame {
        use crate::avatar::FigureLayer;
        let mut logo = AvatarFigure::default();
        logo.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut game = ScreenGame::new();
        game.load_scene(
            Screen::Avi,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BACK_LOGO".to_owned(),
                code: 0x1810_0001,
                sprite: Some("O2mediaLogo.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            }],
            HashMap::from([("O2mediaLogo.ojs".to_owned(), logo)]),
        );
        game
    }

    #[test]
    fn avi_boot_reaches_the_bare_planet_board_on_both_edges() {
        // Key edge: boot shows the staged splash, one lane key
        // dismisses onto the bare board.
        let mut keyed = staged_splash_boot();
        assert_eq!(keyed.screen(), Screen::Avi);
        let boot_hash = keyed.state_hash();
        keyed.lane_key(3, true);
        assert_eq!(keyed.screen(), Screen::Planet);
        assert_ne!(
            keyed.state_hash(),
            boot_hash,
            "the trace moves with the screen"
        );
        // Timer edge: the same boot reaches the same board and the
        // same hash without any input.
        let mut timed = staged_splash_boot();
        paint_splash_frame(&mut timed, SPLASH_AUTO_ADVANCE_MS);
        assert_eq!(timed.screen(), Screen::Planet);
        assert_eq!(
            timed.state_hash(),
            keyed.state_hash(),
            "both boot edges land the same board state"
        );
    }

    #[test]
    fn splash_menu_keys_keep_splash() {
        // Dismiss-set boundary: only Back (Escape) leaves the splash —
        // Confirm (Enter), Up, and Down keep it (rig-pinned Enter
        // non-dismiss, `~/scratch/wt24-rig/key-enter/`; lane keys and
        // canvas clicks are the stand-in dismiss arms, ticket #24).
        let mut game = staged_splash_boot();
        for key in [MenuKey::Confirm, MenuKey::Up, MenuKey::Down] {
            game.menu_key(key, true);
            assert_eq!(game.screen(), Screen::Avi, "{key:?} keeps the splash");
        }
    }

    #[test]
    fn escape_dismisses_splash_into_planet_board() {
        // Escape (Back) dismisses within ~0.3 s on the rig
        // (`~/scratch/avi-hold/run3-escape/`): Back leaves the splash
        // for the bare planet board, unlike the kept menu keys above.
        let mut game = staged_splash_boot();
        assert_eq!(game.screen(), Screen::Avi);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn splash_click_edge_lands_the_same_bare_board() {
        // Third boot edge: a canvas click reaches the same bare board
        // (and the same state hash) as the lane-key edge — every
        // dismiss arm converges, no input invents a scene.
        let mut keyed = staged_splash_boot();
        keyed.lane_key(3, true);
        assert_eq!(keyed.screen(), Screen::Planet);
        let mut clicked = staged_splash_boot();
        clicked.ingest_snapshot(&InputSnapshot::click_at(400, 300));
        assert_eq!(clicked.screen(), Screen::Planet);
        assert_eq!(
            clicked.state_hash(),
            keyed.state_hash(),
            "click and key edges land the same board state"
        );
    }

    fn walk_to_lobby(game: &mut ScreenGame, lobby: Screen) {
        dismiss_splash_to_dialog(game);
        if lobby == Screen::Channel {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        if lobby == Screen::Room {
            return;
        }
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn side_screens_open_from_every_lobby() {
        use super::super::side::ALL_SIDE_SCREENS;
        for lobby in [Screen::Channel, Screen::Room, Screen::Waiting] {
            for side in ALL_SIDE_SCREENS {
                let mut game = ScreenGame::new();
                walk_to_lobby(&mut game, lobby);
                assert!(game.visit_side(side), "{lobby:?} -> {side:?}");
                assert_eq!(game.side(), Some(side));
                assert_eq!(game.screen(), lobby);
            }
        }
    }

    #[test]
    fn side_screens_reject_main_flow_origins() {
        use super::super::side::ALL_SIDE_SCREENS;
        for side in ALL_SIDE_SCREENS {
            let mut splash = ScreenGame::new();
            assert!(!splash.visit_side(side));
            assert_eq!(splash.side(), None);
        }
        for side in ALL_SIDE_SCREENS {
            let mut game = ScreenGame::new();
            walk_to_playing(&mut game);
            assert!(!game.visit_side(side));
            assert_eq!(game.side(), None);
        }
    }

    #[test]
    fn side_visit_rejects_nested_sides() {
        use super::super::side::SideScreen;
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Room);
        assert!(game.visit_side(SideScreen::Shop));
        assert!(!game.visit_side(SideScreen::Planet));
        assert_eq!(game.side(), Some(SideScreen::Shop));
    }

    #[test]
    fn side_back_key_returns_to_lobby() {
        use super::super::side::SideScreen;
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Room);
        assert!(game.visit_side(SideScreen::MyRoom));
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn side_leaves_without_entry() {
        let mut game = ScreenGame::new();
        assert!(!game.leave_side());
    }

    #[test]
    fn side_paints_loaded_art() {
        use super::super::side::SideScreen;
        use crate::avatar::{AvatarFigure, FigureLayer, ScenePlacement};
        use o2jam_render::{Rect, Rgb555Image};
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Channel);
        game.load_side_scene(
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_SHOP".to_owned(),
                code: 0,
                sprite: Some("Shop_Item.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            }],
            HashMap::from([("Shop_Item.ojs".to_owned(), figure)]),
        );
        assert!(game.visit_side(SideScreen::Shop));
        let mut buf = vec![0u32; 8 * 8];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn side_visit_mounts_staged_scene_art_and_keeps_walk_only_sides() {
        // Ticket #136: the live boot stages each Interface State
        // into its full-screen scene slot while the side slots stay
        // empty, so the room-entered sides painted black. Entry
        // mounts the staged scene art into the side slots (paint and
        // click dispatch both read them); a walk-staged side with no
        // scene slot (the Tutorial Playing backdrop) keeps its slots.
        use super::super::side::SideScreen;
        use crate::avatar::ScenePlacement;
        use o2jam_render::Rect;
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Room);
        game.load_scene(
            Screen::Shop,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_SHOP".to_owned(),
                code: 0,
                sprite: Some("Shop_Item.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            }],
            HashMap::from([("Shop_Item.ojs".to_owned(), red_scene_figure())]),
        );
        assert!(game.visit_side(SideScreen::Shop));
        let mut buf = vec![0u32; 8 * 8];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 8,
                h: 8,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(game.leave_side());
        let mut bare = ScreenGame::new();
        walk_to_lobby(&mut bare, Screen::Waiting);
        bare.load_side_scene(
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_TUTORIAL".to_owned(),
                code: 0,
                sprite: Some("Playing1_back.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 800,
                    h: 600,
                },
            }],
            HashMap::from([("Playing1_back.ojs".to_owned(), red_scene_figure())]),
        );
        assert!(bare.visit_side(SideScreen::Tutorial));
        let mut walk_buf = vec![0u32; 8 * 8];
        bare.paint_at(
            &mut Frame {
                buf: &mut walk_buf,
                w: 8,
                h: 8,
            },
            0,
        );
        assert_eq!(walk_buf[0], pack_xrgb(255, 0, 0));
    }

    /// One red pixel figure for staged scene art.
    fn red_scene_figure() -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Three live channels (the hover test needs row 2 in view).
    fn three_channels() -> Vec<ChannelState> {
        (1..=3)
            .map(|channel_id| ChannelState {
                gateway_id: 0,
                channel_id,
                capacity: 100,
                population: 5,
                active: true,
            })
            .collect()
    }

    /// One solid RGB555 block of a given size (chair/plate fixtures).
    fn solid_figure(w: usize, h: usize, word: u16) -> AvatarFigure {
        use crate::avatar::FigureLayer;
        let pixels = vec![word.to_le_bytes(); w * h].concat();
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&pixels, w, h).expect("solid block"),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Full-canvas placement pinning one sprite at the frame origin.
    fn corner_scene_art(name: &str) -> (Vec<ScenePlacement>, HashMap<String, AvatarFigure>) {
        use crate::screens::paint::{unbound_scene_placements, SceneArt};
        let _ = SceneArt::default();
        let names = vec![name.to_owned()];
        let figures = HashMap::from([(name.to_owned(), red_scene_figure())]);
        (unbound_scene_placements(&names), figures)
    }

    #[test]
    fn scene_art_backs_channel_list() {
        let (placements, figures) = corner_scene_art("Dialog_ChannelSelectBar.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Channel, placements, figures);
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        // The staged select bar is the highlight: it paints at the
        // highlighted row's rect (scaled (216,row_top)), so bar ink
        // lands near (17,17) — not the frame origin — and the amber
        // cursor stand-in paints nothing.
        assert_eq!(buf[17 * 64 + 17], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn hover_moves_the_channel_select_bar_off_the_highlight() {
        // The exe arms the select bar on mouse motion alone
        // (`Dialog_ChannelSelectBar.ojs` follows the row under the
        // pointer, live-confirmed 2026-11-09), so with the keyboard
        // highlight on row 0 the bar must follow the pointer down to
        // row 2 — pinned row rect 2 `(216,324)-(572,375)`.
        let mut game = ScreenGame::new();
        // Three rows: the pinned viewport shows four slots, so row 2
        // exists and the pointer can reach it.
        game.set_channels(three_channels());
        let figures = HashMap::from([(
            "Dialog_ChannelSelectBar.ojs".to_owned(),
            solid_figure(348, 44, 0x7C00),
        )]);
        game.load_scene(Screen::Planet, Vec::new(), figures.clone());
        game.load_scene(Screen::Channel, Vec::new(), figures);
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.ingest_snapshot(&InputSnapshot::motion_to(300, 340));
        assert_eq!(game.hovered_channel_row(), Some(2));
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        let bar = pack_xrgb(0xFF, 0x00, 0x00);
        assert_eq!(buf[340 * 800 + 300], bar, "bar follows the pointer row");
        assert_ne!(buf[250 * 800 + 300], bar, "row 0 lost the bar");
        // Off the rows the bar returns to the keyboard highlight.
        game.ingest_snapshot(&InputSnapshot::motion_to(20, 20));
        assert_eq!(game.hovered_channel_row(), None);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert_eq!(buf[250 * 800 + 300], bar, "highlight row keeps the bar");
    }

    #[test]
    fn hovered_planet_action_tracks_pointer_motion() {
        // Ticket #26 consumes ticket #33's pointer state: motion
        // over the EASY emblem rect arms the family pick, motion
        // over FREE arms the (static) unbound emblem, and motion
        // off the board arms nothing. Staged placements win
        // (ADR-0003); the pinned `.bnd` rects back the art-less run.
        use crate::screens::side::{
            PlanetAction, PlanetBand, PLANET_BUTTON_EASY_CODE, PLANET_BUTTON_FREE_CODE,
        };
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.load_scene(
            Screen::Planet,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "easy".to_owned(),
                    code: PLANET_BUTTON_EASY_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 87,
                        y: 32,
                        w: 99,
                        h: 112,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "free".to_owned(),
                    code: PLANET_BUTTON_FREE_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 614,
                        y: 32,
                        w: 99,
                        h: 112,
                    },
                },
            ],
            HashMap::new(),
        );
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(game.hovered_planet_action(), None);
        game.ingest_snapshot(&InputSnapshot::motion_to(136, 88));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Easy)
            })
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(663, 88));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Emblem { band: None })
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(400, 300));
        assert_eq!(game.hovered_planet_action(), None);
        // Leaving the board for the channel dialog clears board
        // hover: the dialog phase owns the pointer now.
        game.ingest_snapshot(&InputSnapshot::motion_to(136, 88));
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.hovered_planet_action(), None);
    }

    /// Stage the bare planet board with every emblem and disc
    /// placement (parsed `.bnd` rects verbatim, codes from
    /// `ControlList_Interface.txt` `STATE_PLANET`): the shared
    /// driver for the disc/emblem sweep guards below (ADR-0003:
    /// staged rects, never literals).
    fn swept_planet_game() -> ScreenGame {
        use crate::screens::side::{
            PLANET_BUTTON_DRUM_CODE, PLANET_BUTTON_EASY_CODE, PLANET_BUTTON_FREE_CODE,
            PLANET_BUTTON_HARD_CODE, PLANET_BUTTON_NORMAL_CODE, PLANET_DRUM_RECT, PLANET_EASY_RECT,
            PLANET_EXIT_CODE, PLANET_EXIT_RECT, PLANET_FREE_RECT, PLANET_HARD_RECT,
            PLANET_NORMAL_RECT, PLANET_OPTION_CODE, PLANET_OPTION_RECT,
        };
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.load_scene(
            Screen::Planet,
            vec![
                ("hard", PLANET_BUTTON_HARD_CODE, PLANET_HARD_RECT),
                ("normal", PLANET_BUTTON_NORMAL_CODE, PLANET_NORMAL_RECT),
                ("easy", PLANET_BUTTON_EASY_CODE, PLANET_EASY_RECT),
                ("free", PLANET_BUTTON_FREE_CODE, PLANET_FREE_RECT),
                ("drum", PLANET_BUTTON_DRUM_CODE, PLANET_DRUM_RECT),
                ("exit", PLANET_EXIT_CODE, PLANET_EXIT_RECT),
                ("option", PLANET_OPTION_CODE, PLANET_OPTION_RECT),
            ]
            .into_iter()
            .map(|(id, code, rect)| ScenePlacement {
                role: PlacementRole::Single,
                id: id.to_owned(),
                code,
                sprite: None,
                rect,
            })
            .collect(),
            HashMap::new(),
        );
        game
    }

    #[test]
    fn disc_sweep_arms_the_gear_and_exit_discs() {
        // Ticket #163 (planet leg, `docs/hover.md` gear/exit rows:
        // hover repaints the disc at full body frame 0): sweeping
        // the pointer over the OPTION disc arms the options dialog,
        // over the EXIT disc the quit-confirm, and off the board
        // arms nothing. Motion alone drives it (polled PtInRect
        // sweep, `docs/mouse-tracking.md`).
        use crate::screens::side::PlanetAction;
        let mut game = swept_planet_game();
        assert_eq!(game.screen(), Screen::Planet);
        game.ingest_snapshot(&InputSnapshot::motion_to(729, 580));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::OptionsDialog)
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(769, 580));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::QuitConfirm)
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(400, 300));
        assert_eq!(game.hovered_planet_action(), None);
    }

    #[test]
    fn hard_normal_and_drum_sweeps_arm_their_emblems() {
        // Ticket #163 (planet leg): the sweep covers every emblem
        // the EASY-only guard never visits — lit HARD/NORMAL arm
        // their bands, DRUM arms the static unbound emblem (its
        // QUEST plate never highlights, `docs/hover.md`).
        use crate::screens::side::{PlanetAction, PlanetBand};
        let mut game = swept_planet_game();
        game.ingest_snapshot(&InputSnapshot::motion_to(386, 88));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Hard)
            })
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(255, 88));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Normal)
            })
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(559, 88));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Emblem { band: None })
        );
    }

    #[test]
    fn free_and_drum_clicks_are_labelled_noops() {
        // Ticket #163 (planet leg): the QUEST/TUTORIAL plates are
        // EXE-static on hover (0 changed px, `docs/hover.md`) and
        // their backends were never wired (P0) — a click records
        // no family, opens no dialog, and holds the bare board.
        // Next evidence step: the exe backend (if any) behind
        // FREE/DRUM, same as the populated-room OPEN in #33.
        let mut game = swept_planet_game();
        game.ingest_snapshot(&InputSnapshot::click_at(663, 88));
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(game.planet_selection(), None);
        assert_eq!(game.planet_modal(), None);
        assert_eq!(game.planet_promo(), None);
        game.ingest_snapshot(&InputSnapshot::click_at(559, 88));
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(game.planet_selection(), None);
        assert_eq!(game.planet_modal(), None);
        assert_eq!(game.planet_promo(), None);
    }

    #[test]
    fn gear_release_inside_opens_the_options_dialog() {
        // Ticket #163 (planet leg, pinned #111 arms 1+3): the press
        // arms on down with no action yet, and release-inside fires
        // on up — the gear opens the options dialog.
        let mut game = swept_planet_game();
        game.ingest_snapshot(&InputSnapshot::press_at(729, 580));
        assert_eq!(
            game.planet_modal(),
            None,
            "the action waits for the release"
        );
        game.ingest_snapshot(&InputSnapshot::release_at(729, 580));
        assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn exit_release_outside_reverts_without_action() {
        // Ticket #163 (planet leg, pinned #111 arm 4): a down on
        // the EXIT disc released off the board reverts to REST and
        // opens nothing. The revert trigger (rect-exit vs up)
        // stays OPEN — the stand-in reverts on up (`docs/hover.md`).
        let mut game = swept_planet_game();
        game.ingest_snapshot(&InputSnapshot::press_at(769, 580));
        assert_eq!(game.planet_modal(), None);
        game.ingest_snapshot(&InputSnapshot::release_at(10, 10));
        assert_eq!(game.planet_modal(), None);
        assert_eq!(game.screen(), Screen::Planet);
        assert!(!game.quit_confirmed());
    }

    #[test]
    fn board_click_while_quit_modal_open_lands_nowhere() {
        // Ticket #163 (planet leg, modal gating arm): while the
        // quit-confirm owns the board, an emblem click is not a
        // family pick — the modal stays open and the board holds.
        let mut game = swept_planet_game();
        game.ingest_snapshot(&InputSnapshot::click_at(769, 580));
        assert_eq!(game.planet_modal(), Some(PlanetModal::QuitConfirm));
        game.ingest_snapshot(&InputSnapshot::click_at(136, 88));
        assert_eq!(game.planet_modal(), Some(PlanetModal::QuitConfirm));
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(game.planet_selection(), None);
    }

    #[test]
    fn planet_hover_stays_off_while_a_modal_owns_the_board() {
        // The quit-confirm modal layers over the bare board: motion
        // over an emblem must not arm board hover behind it.
        use crate::screens::side::{PlanetAction, PlanetBand};
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.load_scene(Screen::Planet, Vec::new(), HashMap::new());
        game.click_planet(769, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::QuitConfirm));
        game.ingest_snapshot(&InputSnapshot::motion_to(136, 88));
        assert_eq!(game.hovered_planet_action(), None);
        // Sanity: the same motion arms the emblem with no modal.
        game.ingest_snapshot(&InputSnapshot::click_at(450, 253));
        game.ingest_snapshot(&InputSnapshot::motion_to(136, 88));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Easy)
            })
        );
    }

    #[test]
    fn playstop_hover_arms_the_frame_swap_cell() {
        // Regression: PLAYSTOP is `0x1920C00F` at rect 14 and RIGHT
        // is `0x1920000E` at rect 13 (`ControlList_Interface.txt`
        // `STATE_PLANET`; `docs/hover.md`). Hover must arm Stop on
        // the middle cell and Right on the right cell, never crossed.
        use crate::screens::side::{PlanetAction, PlanetPromo, PLANET_PROMO_RIGHT_CODE};
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.load_scene(
            Screen::Planet,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "playstop".to_owned(),
                    code: 0x1920_C00F,
                    sprite: None,
                    rect: Rect {
                        x: 387,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "right".to_owned(),
                    code: PLANET_PROMO_RIGHT_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 421,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
            ],
            HashMap::new(),
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(400, 523));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Stop
            })
        );
        game.ingest_snapshot(&InputSnapshot::motion_to(434, 523));
        assert_eq!(
            game.hovered_planet_action(),
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Right
            })
        );
    }

    #[test]
    fn planet_press_down_swaps_to_the_pressed_frame() {
        // Pinned #111 arm 1: ButtonDown swaps the control to its
        // pressed frame within 0.15 s (playstop, 344 px, repaint
        // confined to the control rect — the layer repaint touches
        // the staged rect only).
        let mut game = press_planet_game();
        let rest = pack_xrgb(0x00, 0xFF, 0x00);
        let pressed = pack_xrgb(0xFF, 0x00, 0x00);
        // #63 distinct-frame guard: rest and pressed frames differ.
        assert_ne!(rest, pressed);
        assert_eq!(planet_pixel(&mut game, 400, 523), rest);
        game.ingest_snapshot(&InputSnapshot::press_at(400, 523));
        assert_eq!(planet_pixel(&mut game, 400, 523), pressed);
    }

    #[test]
    fn planet_press_hold_keeps_the_pressed_frame() {
        // Pinned #111 arm 2: the pressed art is stable for the whole
        // hold — 0 px across 0.85 s and 3.5 s holds; no timeout
        // revert, no animation. Motion mid-hold changes nothing.
        let mut game = press_planet_game();
        let pressed = pack_xrgb(0xFF, 0x00, 0x00);
        game.ingest_snapshot(&InputSnapshot::press_at(400, 523));
        assert_eq!(planet_pixel(&mut game, 400, 523), pressed);
        assert_eq!(planet_pixel(&mut game, 400, 523), pressed);
        game.ingest_snapshot(&InputSnapshot::motion_to(401, 523));
        assert_eq!(planet_pixel(&mut game, 400, 523), pressed);
    }

    #[test]
    fn planet_release_inside_reverts_and_fires_the_action_on_up() {
        // Pinned #111 arm 3: release-inside reverts fast (≤30–90 ms)
        // and fires the click action on UP, never on down. The
        // revert target (hover vs rest frame) is OPEN; on the planet
        // board rest == hover art, so the revert shows the rest frame.
        use crate::screens::side::PlanetPromo;
        let mut game = press_planet_game();
        let rest = pack_xrgb(0x00, 0xFF, 0x00);
        game.ingest_snapshot(&InputSnapshot::press_at(366, 523));
        assert_eq!(
            game.planet_promo(),
            None,
            "the action waits for the release"
        );
        game.ingest_snapshot(&InputSnapshot::release_at(366, 523));
        assert_eq!(game.planet_promo(), Some(PlanetPromo::Left));
        assert_eq!(planet_pixel(&mut game, 400, 523), rest);
    }

    #[test]
    fn planet_release_outside_reverts_without_action() {
        // Pinned #111 arm 4: release-outside reverts to REST art and
        // fires no action. The revert trigger (rect-exit vs up) stays
        // OPEN — the stand-in reverts on up, labeled in the painter.
        let mut game = press_planet_game();
        let rest = pack_xrgb(0x00, 0xFF, 0x00);
        let pressed = pack_xrgb(0xFF, 0x00, 0x00);
        game.ingest_snapshot(&InputSnapshot::press_at(400, 523));
        assert_eq!(planet_pixel(&mut game, 400, 523), pressed);
        game.ingest_snapshot(&InputSnapshot::release_at(10, 10));
        assert_eq!(game.planet_promo(), None);
        assert_eq!(planet_pixel(&mut game, 400, 523), rest);
    }

    #[test]
    fn planet_press_on_control_without_pressed_art_paints_nothing() {
        // Pinned #111 arm 1 (per-control rule): the option gear shows
        // no visible pressed art, and the shell must not invent one —
        // a single-record staged control paints nothing on down.
        let mut game = press_planet_game();
        let rest = pack_xrgb(0x00, 0xFF, 0x00);
        game.ingest_snapshot(&InputSnapshot::press_at(366, 523));
        assert_eq!(planet_pixel(&mut game, 366, 523), rest);
    }

    /// Bare planet board with the promo banner body (three distinct
    /// frames) plus the three carousel cells at their parsed `.bnd`
    /// rects (rects 11–14, `ControlList_Interface.txt`
    /// STATE_PLANET). The banner anchors at its `.bnd` rect origin
    /// (49,163), like the shipped `Planet_Promotion.ojs` records.
    fn promo_carousel_game() -> ScreenGame {
        use crate::avatar::FigureLayer;
        use crate::screens::side::{
            PLANET_PROMO_IMAGE_CODE, PLANET_PROMO_LEFT_CODE, PLANET_PROMO_RIGHT_CODE,
            PLANET_PROMO_STOP_CODE,
        };
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        let banner = "Planet_Promotion.ojs";
        let mut frames = AvatarFigure::default();
        for word in [0x7C00u16, 0x03E0u16, 0x001Fu16] {
            frames.push_layer(FigureLayer {
                image: o2jam_render::Rgb555Image::decode(&word.to_le_bytes().repeat(16), 4, 4)
                    .expect("promo frames"),
                dx: 49,
                dy: 163,
                colorkey: 0xFFFF,
            });
        }
        game.load_scene(
            Screen::Planet,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-banner".to_owned(),
                    code: PLANET_PROMO_IMAGE_CODE,
                    sprite: Some(banner.to_owned()),
                    rect: Rect {
                        x: 49,
                        y: 163,
                        w: 701,
                        h: 333,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-left".to_owned(),
                    code: PLANET_PROMO_LEFT_CODE,
                    sprite: Some("Promo_Left.ojs".to_owned()),
                    rect: Rect {
                        x: 353,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-right".to_owned(),
                    code: PLANET_PROMO_RIGHT_CODE,
                    sprite: Some("Promo_Right.ojs".to_owned()),
                    rect: Rect {
                        x: 421,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-stop".to_owned(),
                    code: PLANET_PROMO_STOP_CODE,
                    sprite: Some("Promo_Stop.ojs".to_owned()),
                    rect: Rect {
                        x: 387,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
            ],
            HashMap::from([(banner.to_owned(), frames)]),
        );
        game
    }

    #[test]
    fn promo_boots_on_the_first_banner_and_spinning() {
        // Exe boot pins (`OTwo.exe` 7.55): the carousel index opens
        // at 1 (VA `0x48dc4c`, `this+0x80 = 1`) with rotation enabled
        // (VA `0x48dc8c`, `this+0x90 = 1`), `docs/planet.md`.
        let game = promo_carousel_game();
        assert_eq!(game.planet_promo_index(), 1);
        assert!(game.planet_promo_spinning());
    }

    #[test]
    fn promo_left_steps_the_banner_back_with_wrap() {
        // LEFT arm (VA `0x48e0eb`): index 1 wraps to the last frame,
        // then steps back one frame per press.
        use crate::screens::side::PlanetPromo;
        let mut game = promo_carousel_game();
        game.click_planet(366, 523);
        assert_eq!(game.planet_promo(), Some(PlanetPromo::Left));
        assert_eq!(game.planet_promo_index(), 3);
        game.click_planet(366, 523);
        assert_eq!(game.planet_promo_index(), 2);
    }

    #[test]
    fn promo_right_steps_the_banner_forward_with_wrap() {
        // RIGHT arm (VA `0x48e13c`): forward one frame per press,
        // wrapping the last frame back to the first.
        let mut game = promo_carousel_game();
        game.click_planet(434, 523);
        assert_eq!(game.planet_promo_index(), 2);
        game.click_planet(434, 523);
        assert_eq!(game.planet_promo_index(), 3);
        game.click_planet(434, 523);
        assert_eq!(game.planet_promo_index(), 1);
    }

    #[test]
    fn promo_stop_toggles_the_rotation_flag() {
        // PLAYSTOP arm (VA `0x48e190`): toggles the rotation flag
        // `0 <-> 1` with no immediate repaint.
        let mut game = promo_carousel_game();
        game.click_planet(400, 523);
        assert!(!game.planet_promo_spinning());
        assert_eq!(game.planet_promo_index(), 1);
        game.click_planet(400, 523);
        assert!(game.planet_promo_spinning());
        assert_eq!(game.planet_promo_index(), 1);
    }

    #[test]
    fn promo_tick_advances_only_while_spinning() {
        // Rotation tick (VA `0x48e3b0` minus the wall-clock gate):
        // one step forward per tick while spinning, frozen while
        // stopped — STOP halts the timer without moving the frame.
        let mut game = promo_carousel_game();
        game.tick_planet_promo();
        assert_eq!(game.planet_promo_index(), 2);
        game.click_planet(400, 523);
        game.tick_planet_promo();
        assert_eq!(
            game.planet_promo_index(),
            2,
            "stopped carousel holds its frame"
        );
        game.click_planet(400, 523);
        game.tick_planet_promo();
        assert_eq!(game.planet_promo_index(), 3);
    }

    #[test]
    fn promo_heartbeat_pulses_while_the_board_spins() {
        // Ticket #167: the bare spinning board owes the loop one
        // modest wakeup per second — the 3000 ms rotation tick rides
        // the repaint pulse, so the advance lands within one pulse
        // of the cadence.
        let mut game = promo_carousel_game();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(
            GameLoop::idle_repaint_interval(&game),
            Some(std::time::Duration::from_millis(PLANET_PROMO_HEARTBEAT_MS))
        );
    }

    #[test]
    fn promo_heartbeat_parks_while_stopped() {
        // Ticket #167: STOP clears the rotation flag, so the board
        // owes the loop nothing — stopped stays fully idle (the #138
        // parked-loop result holds on the promo board too).
        let mut game = promo_carousel_game();
        game.lane_key(3, true);
        game.click_planet(400, 523);
        assert!(!game.planet_promo_spinning());
        assert_eq!(GameLoop::idle_repaint_interval(&game), None);
    }

    #[test]
    fn promo_heartbeat_parks_while_the_quit_modal_covers_the_board() {
        // Ticket #167: the heartbeat mirrors the bare-board gate —
        // the quit-confirm modal owns the board, so no pulse even
        // while the flag spins. (Sides mount only over the
        // Channel/Room/Waiting lobbies through the public seam, so
        // the modal arm carries the covered-board case here.)
        let mut game = promo_carousel_game();
        game.lane_key(3, true);
        game.click_planet(769, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::QuitConfirm));
        assert!(game.planet_promo_spinning());
        assert_eq!(GameLoop::idle_repaint_interval(&game), None);
    }

    /// Deterministic clock pump: the paint seam's explicit instant
    /// is the frame time the carousel accumulator reads.
    fn paint_planet_frame_at(game: &mut ScreenGame, now_ms: u64) {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            now_ms,
        );
    }

    #[test]
    fn promo_clock_advances_the_banner_past_three_seconds() {
        // Ticket #167: with the flag set, pumping the frame clock
        // past the 3000 ms cadence advances the banner one frame —
        // the heartbeat pulse drives the accumulator through paint.
        let mut game = promo_carousel_game();
        game.lane_key(3, true);
        for now_ms in [0, 1_000, 2_000, 3_000, 4_000] {
            paint_planet_frame_at(&mut game, now_ms);
        }
        assert_eq!(game.planet_promo_index(), 2);
    }

    #[test]
    fn promo_clock_holds_the_banner_while_stopped() {
        // Ticket #167: with the flag clear, the same clock pump
        // leaves the banner on its frame — the accumulator never
        // arms while stopped.
        let mut game = promo_carousel_game();
        game.lane_key(3, true);
        game.click_planet(400, 523);
        assert!(!game.planet_promo_spinning());
        for now_ms in [0, 1_000, 2_000, 3_000, 4_000] {
            paint_planet_frame_at(&mut game, now_ms);
        }
        assert_eq!(game.planet_promo_index(), 1);
    }

    #[test]
    fn promo_left_press_repaints_the_banner_region() {
        // The visible banner follows the carousel index: at boot the
        // board shows frame 1, after LEFT it shows frame 3 (the
        // 1-to-max wrap), repainted over the stacked rest frames.
        let mut game = promo_carousel_game();
        let first = pack_xrgb(0xFF, 0x00, 0x00);
        let last = pack_xrgb(0x00, 0x00, 0xFF);
        assert_eq!(planet_pixel(&mut game, 50, 164), first);
        game.click_planet(366, 523);
        assert_eq!(game.planet_promo_index(), 3);
        assert_eq!(planet_pixel(&mut game, 50, 164), last);
    }

    #[test]
    fn empty_channel_art_clears_stale_splash_pixels() {
        // Live Channel stages no art (`stage_empty_channel_scene`), so
        // entering from the splash must not leave logo pixels behind:
        // the list paints over device black, matching the pinned
        // art-less shot (`docs/screenshots/channel.png`).
        const STALE_SPLASH: u32 = pack_xrgb(0xEB, 0xE3, 0xE4);
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Channel, Vec::new(), HashMap::new());
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        let mut buf = vec![STALE_SPLASH; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert!(
            !buf.contains(&STALE_SPLASH),
            "live channel keeps splash pixels"
        );
        assert_eq!(buf[0], 0, "live channel clears to device black");
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn staged_room_art_paints_live_room_rows() {
        // Live Room shows staged panels but no row ink: narrow whether
        // the painter drops rows when art is staged (data present).
        let (placements, figures) = corner_scene_art("Room_ButtonRoom.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Room, placements, figures);
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert!(
            has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)),
            "room rows missing under staged art"
        );
    }

    #[test]
    fn listed_rooms_keep_titled_zero_numbered_first_room() {
        // The server numbers rooms from the first free slot, so the
        // first room is number 0; free slots share the number but
        // carry an empty title.
        let titled_first = RoomListEntry {
            number: 0,
            state_code: 1,
            title: "smoke".to_owned(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 1,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        };
        let free_slot = RoomListEntry {
            number: 0,
            state_code: 0,
            title: String::new(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 0,
            game_mode: 0,
            speed: 1,
            capacity: 0,
            user_count: 0,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        };
        assert!(ScreenGame::is_listed_room(&titled_first));
        assert!(!ScreenGame::is_listed_room(&free_slot));
        assert!(ScreenGame::is_listed_room(&two_rooms()[0]));
    }

    #[test]
    fn room_number_at_names_clicked_rows() {
        // Band fallback needs no staged art: native bands resolve the
        // row whose number the session bridge joins before advancing.
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        assert_eq!(game.room_number_at(400, 154), Some(3));
        assert_eq!(game.room_number_at(400, 162), Some(7));
        assert_eq!(game.room_number_at(400, 100), None);
    }

    #[test]
    fn quiet_server_slot_table_leaves_no_rows_to_click() {
        // Live 2026-11-09: the quiet server sends an all-`Number`-0
        // slot table, so the empty-slot filter leaves blank panels —
        // no rows to click (correct: the painter proves rows with
        // live data, never with free slots).
        let free_slot = || RoomListEntry {
            number: 0,
            state_code: 0,
            title: String::new(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 0,
            game_mode: 0,
            speed: 1,
            capacity: 0,
            user_count: 0,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        };
        let mut game = ScreenGame::new();
        game.set_rooms(vec![
            free_slot(),
            free_slot(),
            free_slot(),
            free_slot(),
            free_slot(),
            free_slot(),
        ]);
        walk_to_lobby(&mut game, Screen::Room);
        assert_eq!(game.room_number_at(38, 94), None);
        assert_eq!(game.room_number_at(400, 154), None);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(!white_near(&buf, 800, 600, (38, 94)));
        assert!(!white_near(&buf, 800, 600, (90, 93)));
    }

    #[test]
    fn room_lobby_session_replays_like_its_twin() {
        // Hand-driven lobby session: seed rooms, walk Channel into
        // Room, step the highlight down and back, confirm into
        // Waiting — the twin game driven the same way hashes the
        // same (the trace harness compares replays through
        // `state_hash`).
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_rooms(two_rooms());
        walk_to_lobby(&mut game, Screen::Room);
        game.menu_key(MenuKey::Down, true);
        game.menu_key(MenuKey::Up, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        let mut twin = ScreenGame::new();
        twin.set_rooms(two_rooms());
        walk_to_lobby(&mut twin, Screen::Room);
        twin.menu_key(MenuKey::Down, true);
        twin.menu_key(MenuKey::Up, true);
        twin.menu_key(MenuKey::Confirm, true);
        assert_eq!(twin.screen(), Screen::Waiting);
        assert_eq!(game.state_hash(), twin.state_hash());
    }

    #[test]
    fn waiting_confirm_hit_needs_staged_buttons() {
        // Art-less clicks land nowhere (no pinned fallback geometry).
        let mut game = ScreenGame::new();
        assert!(!game.waiting_confirm_hit(150, 130));
        let placements = vec![crate::avatar::ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_SWAIT_READY".to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 100,
                y: 100,
                w: 200,
                h: 60,
            },
        }];
        game.load_scene(Screen::Waiting, placements, HashMap::new());
        assert!(game.waiting_confirm_hit(150, 130));
        assert!(!game.waiting_confirm_hit(10, 10));
    }

    #[test]
    fn channel_rows_ink_at_measured_native_tops() {
        // Row-text anchors on a native frame, scaled like staged
        // placements elsewhere. Re-measured 2026-11-11 with the
        // pinned `Ch. NN` fallback label (#93): the Gulimche 'C'
        // glyph tops sit 8 px above the old digit-led stand-in
        // (224+50·index was 232+50·index).
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(white_near(&buf, 800, 600, (200, 224)));
        assert_eq!(buf[240 * 800 + 230], pack_xrgb(19, 20, 21));
        assert_eq!(buf[229 * 800 + 350], pack_xrgb(230, 227, 230));
        assert!(ink_near(
            &buf,
            800,
            600,
            (502, 236),
            (24, 12),
            pack_xrgb(150, 148, 151)
        ));
        assert!(ink_near(
            &buf,
            800,
            600,
            (200, 282),
            (64, 9),
            pack_xrgb(0x70, 0x70, 0x70)
        ));
    }

    #[test]
    fn channel_row_hit_maps_the_exe_pinned_rects() {
        // Exe-pinned slot rects (CStatePlanet dialog+0x94):
        // (216,224,572,275), (216,275,572,324), (216,324,572,375),
        // (216,375,572,436) — left/top inclusive, right/bottom
        // exclusive. x = 300 sits inside every band's x range.
        assert_eq!(channel_row_hit(300, 224, 10), Some(0));
        assert_eq!(channel_row_hit(300, 240, 10), Some(0));
        assert_eq!(channel_row_hit(300, 274, 10), Some(0));
        assert_eq!(channel_row_hit(300, 275, 10), Some(1));
        assert_eq!(channel_row_hit(300, 282, 10), Some(1));
        assert_eq!(channel_row_hit(300, 375, 10), Some(3));
        assert_eq!(channel_row_hit(300, 435, 10), Some(3));
        assert_eq!(channel_row_hit(300, 436, 10), None);
        assert_eq!(channel_row_hit(300, 100, 10), None);
        assert_eq!(channel_row_hit(300, 732, 10), None);
        assert_eq!(channel_row_hit(300, 240, 0), None);
        // Issue #29: the y band alone is not a hit — x must sit
        // inside the pinned rect too. The scroll track (x573-593)
        // and the dialog margins share the bands' y range.
        assert_eq!(channel_row_hit(583, 240, 10), None);
        assert_eq!(channel_row_hit(100, 240, 10), None);
        assert_eq!(channel_row_hit(700, 290, 10), None);
        assert_eq!(channel_row_hit(216, 240, 10), Some(0));
        assert_eq!(channel_row_hit(571, 240, 10), Some(0));
        assert_eq!(channel_row_hit(572, 240, 10), None);
    }

    #[test]
    fn bound_room_panels_paint_numbered_rows() {
        // Live Room stages bound panels yet shows no row ink: narrow
        // whether the panel path drops rows when data is present.
        // Rows sit inside the panel at the reference-measured split
        // offsets — the number at panel +12/+12, sized to the
        // `ROOMNAME` band while the digit font stages no figure.
        let placements = vec![
            crate::avatar::ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 26,
                    y: 82,
                    w: 288,
                    h: 91,
                },
            },
            crate::avatar::ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_TEXT_SROOM_ROOMNAME".to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 66,
                    y: 13,
                    w: 158,
                    h: 16,
                },
            },
        ];
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Room, placements, HashMap::new());
        game.set_rooms(vec![RoomListEntry {
            number: 0,
            state_code: 1,
            title: "smoke".to_owned(),
            has_password: false,
            music_id: 0,
            difficulty: 0,
            key_mode: 2,
            game_mode: 0,
            speed: 1,
            capacity: 8,
            user_count: 1,
            min_level: 0,
            max_level: 0,
            skills: vec![],
            premium: false,
            room_type: 0,
        }]);
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        // Exe-measured 2026-11-09 (rig session, ~/scratch/geom-open-close):
        // the number field anchors at panel origin +12/+12 — panel 0
        // at (26,82) puts the number glyphs at (38,94).
        assert!(white_near(&buf, 800, 600, (38, 94)));
    }

    #[test]
    fn channel_viewport_follows_highlight() {
        assert_eq!(channel_viewport(0, 10), (0, 4));
        assert_eq!(channel_viewport(5, 10), (2, 4));
        assert_eq!(channel_viewport(9, 10), (6, 4));
        assert_eq!(channel_viewport(0, 2), (0, 2));
        assert_eq!(channel_viewport(0, 0), (0, 0));
    }

    #[test]
    fn channel_click_maps_through_viewport() {
        // Highlight on row 5 shows rows 2-5; the top visible band
        // selects row 2.
        let mut game = ScreenGame::new();
        game.set_channels(six_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        for _ in 0..5 {
            game.menu_key(MenuKey::Down, true);
        }
        assert_eq!(game.highlight, 5);
        // Ticket #56: the face of visible slot 0 enters; the
        // viewport maps it back to row 2.
        game.ingest_snapshot(&InputSnapshot::click_at(524, 246));
        assert_eq!(game.highlight, 2);
        assert_eq!(game.screen(), Screen::Room);
    }

    fn six_channels() -> Vec<ChannelState> {
        (0u16..6)
            .map(|id| ChannelState {
                gateway_id: 0,
                channel_id: id,
                capacity: 100,
                population: 5,
                active: true,
            })
            .collect()
    }

    #[test]
    fn planet_paints_staged_art_at_anchor() {
        // Synthetic staged figure (red pixel): art paints, nothing else.
        let (placements, figures) = corner_scene_art("State_Planet.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Planet, placements, figures);
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn planet_fallback_paints_card_and_title() {
        // No staged art: flat card plus the placeholder title.
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Planet));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn planet_captions_have_real_glyphs_in_the_system_font() {
        // The painter routes captions through `o2jam-text` with the
        // system face (owner decision `e8d01`); the ASCII stand-in
        // only covers the font-absent fallback. Every caption char
        // must exist in that face, or the board would paint tofu.
        // No system face (bare CI) skips cleanly.
        use super::super::side::PLANET_CAPTIONS;
        let Some(font) = o2jam_text::system_font() else {
            return;
        };
        for caption in PLANET_CAPTIONS {
            let text = o2jam_text::decode_cp949(caption.cp949).expect("pinned CP949");
            assert!(!text.is_empty());
            for character in text.chars() {
                assert!(
                    font.has_glyph(character),
                    "system face lacks {character:?} for caption {text:?}"
                );
            }
        }
    }

    #[test]
    fn planet_captions_paint_into_their_bnd_rects() {
        // Synthetic TEXT placements at the parsed bind positions
        // (ordinals 6..10): each caption inks only its own rect,
        // and no placement paints nothing (never literals).
        use super::super::side::PLANET_CAPTIONS;
        let placements: Vec<ScenePlacement> = [(10, 10), (120, 10)]
            .into_iter()
            .zip(PLANET_CAPTIONS.iter())
            .map(|((x, y), caption)| ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: caption.code,
                sprite: None,
                rect: Rect { x, y, w: 94, h: 13 },
            })
            .collect();
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Planet, placements, HashMap::new());
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        let in_rect = |buf: &[u32], x: i32, y: i32| -> bool {
            (y..y + 13)
                .any(|row| ((row * 800 + x)..(row * 800 + x + 94)).any(|at| buf[at as usize] != 0))
        };
        assert!(in_rect(&buf, 10, 10), "first caption rect stays blank");
        assert!(in_rect(&buf, 120, 10), "second caption rect stays blank");
        // Ink stays inside the two rects: a pixel far below them
        // keeps the frame black.
        assert_eq!(buf[500 * 800 + 400], 0, "caption ink leaked");
        // Without placements no caption paints (no fallback geometry).
        let mut bare = ScreenGame::new();
        bare.load_scene(Screen::Planet, Vec::new(), HashMap::new());
        bare.lane_key(3, true);
        let mut bare_buf = vec![0u32; 800 * 600];
        bare.paint_at(
            &mut Frame {
                buf: &mut bare_buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(bare_buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn power_disc_opens_quit_confirm_and_ok_quits() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        game.click_planet(769, 580);
        assert_eq!(
            game.planet_modal(),
            Some(PlanetModal::QuitConfirm),
            "power disc must open the quit-confirm"
        );
        assert!(!game.quit_confirmed());
        // The fallback popup paints its message (frame is varied).
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        assert!(buf.iter().any(|pixel| *pixel != buf[0]));
        // OK (fallback box center) quits and closes; CANCEL instead
        // closes without quitting (fresh game below).
        game.ingest_snapshot(&InputSnapshot::click_at(350, 253));
        assert!(game.quit_confirmed(), "OK must quit");
        assert_eq!(game.planet_modal(), None);
    }

    #[test]
    fn lit_emblem_click_opens_the_channel_dialog_and_records_the_pick() {
        use crate::screens::side::PlanetBand;
        // Lit-emblem press (HARD badge through its pinned `.bnd`
        // rect 1): records the family pick and opens the dialog
        // over the board (live-verified 2026-11-09, `docs/planet.md`).
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        game.click_planet(386, 88);
        assert_eq!(
            game.screen(),
            Screen::Channel,
            "lit emblem opens the dialog-over-board phase"
        );
        assert_eq!(game.planet_selection(), Some(PlanetBand::Hard));
    }

    #[test]
    fn planet_confirm_opens_the_channel_dialog() {
        // Keyboard stand-in for the emblem press (the exe entry key
        // stays OPEN): Confirm advances Planet → Channel exactly
        // like the mouse emblem edge, so the board always goes
        // somewhere. No bare-board → Room path is evidenced.
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Channel);
    }

    #[test]
    fn promo_press_records_the_cell_without_leaving_the_board() {
        use crate::screens::side::{PlanetPromo, PLANET_PROMO_LEFT_CODE, PLANET_PROMO_STOP_CODE};
        // Staged promo placements (parsed `.bnd` rects verbatim,
        // rects 12/14): each press records its cell, the board
        // stays, and no modal opens (carousel semantics OPEN).
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.load_scene(
            Screen::Planet,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-left".to_owned(),
                    code: PLANET_PROMO_LEFT_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 353,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-stop".to_owned(),
                    code: PLANET_PROMO_STOP_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 387,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
            ],
            HashMap::new(),
        );
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(game.planet_promo(), None);
        game.click_planet(366, 523);
        assert_eq!(game.planet_promo(), Some(PlanetPromo::Left));
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(game.planet_modal(), None);
        game.click_planet(400, 523);
        assert_eq!(game.planet_promo(), Some(PlanetPromo::Stop));
        assert_eq!(game.screen(), Screen::Planet);
        // Outside every staged cell the press lands nowhere and
        // the recorded cell stands.
        game.click_planet(400, 500);
        assert_eq!(game.planet_promo(), Some(PlanetPromo::Stop));
    }

    #[test]
    fn quit_confirm_cancel_closes_without_quitting() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(769, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::QuitConfirm));
        // CANCEL fallback box center (native 450,253).
        game.ingest_snapshot(&InputSnapshot::click_at(450, 253));
        assert!(!game.quit_confirmed());
        assert_eq!(game.planet_modal(), None);
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn gear_disc_opens_options_and_escape_closes_it() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(729, 580);
        assert_eq!(
            game.planet_modal(),
            Some(PlanetModal::Options),
            "gear disc must open the options dialog"
        );
        // Board clicks ride the modal now (the emblem stays put).
        game.click_planet(386, 88);
        assert_eq!(game.screen(), Screen::Planet);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.planet_modal(), None, "Escape closes options");
        game.click_planet(386, 88);
        assert_eq!(game.screen(), Screen::Channel, "board is live again");
    }

    /// Synthetic options sheet: a `Dialog_*` backdrop (the
    /// dialog-relative origin) plus one 36x20 button placement per
    /// `(code, x, y)`, with a figure so the staged path answers.
    fn stage_option_buttons(
        game: &mut ScreenGame,
        tab: PlanetOptionTab,
        buttons: &[(u32, i32, i32)],
    ) {
        use crate::avatar::FigureLayer;
        let backdrop = match tab {
            PlanetOptionTab::Key => "Dialog_Option_Key.ojs",
            PlanetOptionTab::Sound => "Dialog_Option_Sound.ojs",
            PlanetOptionTab::Game => "Dialog_Option_Game.ojs",
        };
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: o2jam_render::Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        let mut placements = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_DIALOG".to_owned(),
            code: 0,
            sprite: Some(backdrop.to_owned()),
            rect: Rect {
                x: 0,
                y: 0,
                w: 800,
                h: 600,
            },
        }];
        for (index, (code, x, y)) in buttons.iter().enumerate() {
            placements.push(ScenePlacement {
                role: PlacementRole::Single,
                id: format!("IDC_BUTTON_{index}"),
                code: *code,
                sprite: None,
                rect: Rect {
                    x: *x,
                    y: *y,
                    w: 36,
                    h: 20,
                },
            });
        }
        game.load_planet_option_tab(
            tab,
            placements,
            HashMap::from([(backdrop.to_owned(), figure)]),
        );
    }

    /// Open the options dialog on the planet board (gear disc
    /// fallback rect, no staged board art needed).
    fn open_options(game: &mut ScreenGame) {
        game.lane_key(3, true);
        game.click_planet(729, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
    }

    /// Click the center of a synthetic 36x20 button at `(x, y)`.
    fn click_button(game: &mut ScreenGame, x: i32, y: i32) {
        game.ingest_snapshot(&InputSnapshot::click_at(x + 18, y + 10));
    }

    #[test]
    fn options_key_tab_rebind_answers_on_the_new_lane() {
        use winit::keyboard::KeyCode;
        let mut game = ScreenGame::new();
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Key,
            &[(OPTION_KEY_LANE_FIRST, 100, 100)],
        );
        open_options(&mut game);
        click_button(&mut game, 100, 100);
        assert_eq!(game.option_rebind_armed(), Some(0));
        assert!(game.capture_raw_key(KeyCode::KeyA, true));
        assert_eq!(game.option_rebind_armed(), None);
        assert_eq!(game.options().lane_keys()[0], KeyCode::KeyA);
        assert_eq!(
            game.lane_bindings().map(|binds| binds[0]),
            Some(KeyCode::KeyA),
            "the window syncs the rebound lane"
        );
    }

    #[test]
    fn options_key_tab_rebind_refuses_unmappable_keys() {
        use winit::keyboard::KeyCode;
        let mut game = ScreenGame::new();
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Key,
            &[(OPTION_KEY_LANE_FIRST + 1, 140, 100)],
        );
        open_options(&mut game);
        click_button(&mut game, 140, 100);
        assert_eq!(game.option_rebind_armed(), Some(1));
        assert!(
            !game.capture_raw_key(KeyCode::F1, true),
            "keys without a DirectInput code never persist"
        );
        assert_eq!(game.option_rebind_armed(), Some(1));
        assert_eq!(game.options().lane_keys()[1], KeyCode::KeyD);
        assert!(
            !game.capture_raw_key(KeyCode::Escape, true),
            "menu keys keep their routing while armed"
        );
    }

    #[test]
    fn options_sound_tab_steps_move_volumes_with_exe_bounds() {
        let mut game = ScreenGame::new();
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Sound,
            &[
                (OPTION_SOUND_GAME_EFFECT_DOWN, 100, 100),
                (OPTION_SOUND_GAME_EFFECT_UP, 140, 100),
            ],
        );
        open_options(&mut game);
        game.switch_option_tab(PlanetOptionTab::Sound);
        click_button(&mut game, 100, 100);
        assert_eq!(
            game.options().game_effect(),
            255 - crate::options::SOUND_SLIDER_STEP
        );
        click_button(&mut game, 140, 100);
        assert_eq!(game.options().game_effect(), 255);
        click_button(&mut game, 140, 100);
        assert_eq!(
            game.options().game_effect(),
            255,
            "steps saturate at the byte ceiling"
        );
    }

    #[test]
    fn options_game_tab_buttons_map_to_the_pinned_ini_values() {
        let mut game = ScreenGame::new();
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Game,
            &[
                (OPTION_GAME_NOTETYPE2, 100, 100),
                (OPTION_GAME_GUIDE_LONG, 140, 100),
                (OPTION_GAME_BLEND_NO, 180, 100),
                (OPTION_GAME_MODE_WIN, 220, 100),
                (OPTION_GAME_LONGNOTE_USED, 260, 100),
                (OPTION_GAME_EFFECT_LOW, 300, 100),
            ],
        );
        open_options(&mut game);
        game.switch_option_tab(PlanetOptionTab::Game);
        click_button(&mut game, 100, 100);
        click_button(&mut game, 140, 100);
        click_button(&mut game, 180, 100);
        click_button(&mut game, 220, 100);
        click_button(&mut game, 260, 100);
        click_button(&mut game, 300, 100);
        assert_eq!(game.options().note_type(), 1);
        assert_eq!(game.options().note_tail(), 3);
        assert_eq!(game.options().blend_half(), 0);
        assert_eq!(game.options().mode(), 2);
        assert!(game.options().longnote_used());
        assert_eq!(game.options().effect_level(), 2);
    }

    #[test]
    fn options_tab_switches_ride_the_staged_codes() {
        let mut game = ScreenGame::new();
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Key,
            &[(super::super::side::OPTION_KEY_SOUNDSET, 100, 100)],
        );
        stage_option_buttons(&mut game, PlanetOptionTab::Sound, &[]);
        open_options(&mut game);
        assert_eq!(game.planet_option_tab(), PlanetOptionTab::Key);
        click_button(&mut game, 100, 100);
        assert_eq!(game.planet_option_tab(), PlanetOptionTab::Sound);
        assert_eq!(
            game.option_rebind_armed(),
            None,
            "switching tabs disarms a pending rebind"
        );
    }

    #[test]
    fn options_save_commits_beside_the_install_root() {
        let root = std::env::temp_dir().join(format!("recomp2jam-options-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut game = ScreenGame::new();
        game.load_options(&root);
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Key,
            &[(OPTION_KEY_LANE_FIRST, 100, 100)],
        );
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Sound,
            &[(PlanetOptionTab::Sound.save_code(), 100, 100)],
        );
        open_options(&mut game);
        click_button(&mut game, 100, 100);
        assert!(game.capture_raw_key(winit::keyboard::KeyCode::KeyA, true));
        game.switch_option_tab(PlanetOptionTab::Sound);
        click_button(&mut game, 100, 100);
        let text = std::fs::read_to_string(root.join("option.ini")).unwrap();
        assert!(
            text.contains("KEY_1=30"),
            "rebound A persists as its DirectInput code: {text}"
        );
        let reloaded = crate::options::OptionsSettings::load_from_dir(&root);
        assert_eq!(reloaded, *game.options(), "a restart keeps them");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn options_exit_closes_without_saving() {
        let root =
            std::env::temp_dir().join(format!("recomp2jam-options-exit-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut game = ScreenGame::new();
        game.load_options(&root);
        let exit = PlanetOptionTab::Key.exit_code();
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Key,
            &[(OPTION_GAME_NOTETYPE2, 100, 100), (exit, 200, 100)],
        );
        open_options(&mut game);
        game.switch_option_tab(PlanetOptionTab::Game);
        // The Game sheet is unstaged: no button answers, and the
        // Key-tab EXIT rect belongs to the other sheet.
        click_button(&mut game, 200, 100);
        assert_eq!(
            game.planet_modal(),
            Some(PlanetModal::Options),
            "unstaged tabs answer nothing"
        );
        game.switch_option_tab(PlanetOptionTab::Key);
        click_button(&mut game, 200, 100);
        assert_eq!(game.planet_modal(), None, "EXIT closes");
        assert!(
            !root.join("option.ini").exists(),
            "EXIT abandons: only SAVE commits"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn options_default_restores_the_exe_defaults_per_tab() {
        let mut game = ScreenGame::new();
        let default = PlanetOptionTab::Key.default_code();
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Game,
            &[
                (OPTION_GAME_NOTETYPE2, 100, 100),
                (OPTION_GAME_GUIDE_LONG, 140, 100),
                (default, 180, 100),
            ],
        );
        open_options(&mut game);
        game.switch_option_tab(PlanetOptionTab::Game);
        click_button(&mut game, 100, 100);
        click_button(&mut game, 140, 100);
        assert_eq!(game.options().note_type(), 1);
        // DEFAULT belongs to the Key tab sheet here, so the Game
        // values stand; staging it on the open tab restores them.
        click_button(&mut game, 180, 100);
        assert_eq!(game.options().note_type(), 1);
        stage_option_buttons(
            &mut game,
            PlanetOptionTab::Game,
            &[(PlanetOptionTab::Game.default_code(), 180, 100)],
        );
        click_button(&mut game, 180, 100);
        assert_eq!(game.options().note_type(), 0);
        assert_eq!(game.options().note_tail(), 2);
    }

    #[test]
    fn longnote_mode_snapshots_at_playing_entry() {
        let mut game = ScreenGame::new();
        assert_eq!(game.play_longnote_mode(), None);
        game.options.set_longnote_mode(1);
        walk_to_playing(&mut game);
        assert_eq!(game.play_longnote_mode(), Some(1));
        game.options.set_longnote_mode(0);
        assert_eq!(
            game.play_longnote_mode(),
            Some(1),
            "later dialog edits never reach the running song"
        );
    }

    #[test]
    fn key_strikes_play_at_the_game_effect_gain() {
        use o2jam_audio::{AudioBackend, AudioError, PcmSamples};
        use std::cell::RefCell;
        use std::rc::Rc;
        struct GainProbe {
            gains: Rc<RefCell<Vec<f32>>>,
        }
        impl AudioBackend for GainProbe {
            fn play_pcm(&mut self, _pcm: &PcmSamples) -> Result<(), AudioError> {
                Ok(())
            }
            fn play_key(&mut self, _pcm: &PcmSamples, gain: f32) -> Result<(), AudioError> {
                self.gains.borrow_mut().push(gain);
                Ok(())
            }
            fn samples_played(&self) -> u64 {
                0
            }
        }
        let gains = Rc::new(RefCell::new(Vec::new()));
        let probe = GainProbe {
            gains: Rc::clone(&gains),
        };
        let bank = HashMap::from([(
            7u32,
            PcmSamples {
                samples: vec![1000i16; 8],
                sample_rate: 8000,
                channels: 1,
            },
        )]);
        let mut game = ScreenGame::new();
        game.with_audio(Box::new(probe), bank);
        game.load_chart(vec![(2, 5000, 7), (2, 6000, 7)]);
        for next in [
            Screen::Planet,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
        ] {
            game.flow.go(next).unwrap();
        }
        game.strike_at(2, 5000);
        assert_eq!(gains.borrow().len(), 1);
        assert!((gains.borrow()[0] - 1.0).abs() < 1e-6);
        for _ in 0..64 {
            game.options.step_game_effect(false);
        }
        game.strike_at(2, 6000);
        assert_eq!(gains.borrow().len(), 2);
        assert!(gains.borrow()[1].abs() < 1e-6, "muted effect stays silent");
    }

    #[test]
    fn scene_bgm_cue_scales_with_the_bgmusic_volume() {
        use o2jam_audio::{AudioBackend, AudioError, PcmSamples};
        use std::cell::RefCell;
        use std::rc::Rc;
        struct PeakProbe {
            peaks: Rc<RefCell<Vec<i32>>>,
        }
        impl AudioBackend for PeakProbe {
            fn play_pcm(&mut self, _pcm: &PcmSamples) -> Result<(), AudioError> {
                Ok(())
            }
            fn set_scene_bgm(
                &mut self,
                pcm: Option<&PcmSamples>,
                _repeat: bool,
            ) -> Result<(), AudioError> {
                let peak = pcm.map_or(0, |pcm| {
                    pcm.samples
                        .iter()
                        .map(|s| i32::from(*s).abs())
                        .max()
                        .unwrap_or(0)
                });
                self.peaks.borrow_mut().push(peak);
                Ok(())
            }
            fn samples_played(&self) -> u64 {
                0
            }
        }
        let peaks = Rc::new(RefCell::new(Vec::new()));
        let probe = PeakProbe {
            peaks: Rc::clone(&peaks),
        };
        let mut bank = vec![None; 10];
        bank[8] = Some(PcmSamples {
            samples: vec![1000i16; 441],
            sample_rate: 44100,
            channels: 1,
        });
        let mut game = ScreenGame::new();
        game.with_audio(Box::new(probe), HashMap::new());
        game.stage_scene_bgm(bank);
        game.enter(Screen::Planet);
        assert_eq!(peaks.borrow().last(), Some(&1000));
        game.options.step_bgm_music(false);
        game.enter(Screen::Channel);
        // 1000 * 247 / 255 = 968.6, rounds to 969.
        assert_eq!(peaks.borrow().last(), Some(&969));
    }

    #[test]
    fn team_select_lights_joined_and_queues_4004_then_4008() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        assert_eq!(game.waiting.as_ref().unwrap().team, 1);
        game.select_team(5);
        assert_eq!(game.waiting.as_ref().unwrap().team, 5);
        assert_eq!(game.battle_self_team, Some(5));
        assert!(game.has_pending_team_send());
        assert_eq!(game.take_room_team(), Some(SetRoomTeam { team: 5 }));
        assert_eq!(game.take_team_ready(), Some(Ready));
        assert!(!game.has_pending_team_send());
        assert_eq!(game.take_room_team(), None);
    }

    #[test]
    fn click_staged_team_button_queues_4004_then_4008() {
        // Team buttons answer through their staged `.bnd` rects
        // (rects 3/4, the `team_highlight` pins): a TEAMA click at
        // the staged rect moves the highlight to team 0 and queues
        // the `SetRoomTeam` 4004 + `Ready` 4008 send pair through
        // `select_team` — the click path, not the direct call.
        use super::super::side::WAITING_TEAM_FIRST_CODE;
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        assert_eq!(game.waiting.as_ref().unwrap().team, 1);
        game.load_scene(
            Screen::Waiting,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_BUTTON_SWAIT_TEAMA".to_owned(),
                    code: WAITING_TEAM_FIRST_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 652,
                        y: 65,
                        w: 24,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_BUTTON_SWAIT_TEAMB".to_owned(),
                    code: WAITING_TEAM_FIRST_CODE + 1,
                    sprite: None,
                    rect: Rect {
                        x: 682,
                        y: 65,
                        w: 24,
                        h: 24,
                    },
                },
            ],
            HashMap::new(),
        );
        walk_to_waiting(&mut game);
        game.ingest_snapshot(&InputSnapshot::click_at(660, 75));
        assert_eq!(game.waiting.as_ref().unwrap().team, 0);
        assert_eq!(game.battle_self_team, Some(0));
        assert!(game.has_pending_team_send());
        assert_eq!(game.take_room_team(), Some(SetRoomTeam { team: 0 }));
        assert_eq!(game.take_team_ready(), Some(Ready));
        assert!(!game.has_pending_team_send());
        // A click outside both staged buttons queues nothing.
        game.ingest_snapshot(&InputSnapshot::click_at(10, 10));
        assert!(!game.has_pending_team_send());
    }

    #[test]
    fn waiting_hand_session_joins_team_readies_and_starts() {
        // Hand-driven waiting session trace (join team, ready, host
        // start — no gateway): the master clicks TEAMA through the
        // staged rect, the 4004+4008 pair stages the session sends,
        // and Confirm runs the START readiness gate into Loading.
        // Smoke: one trace, team moved, sends drained, Loading held.
        use super::super::side::WAITING_TEAM_FIRST_CODE;
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        game.load_scene(
            Screen::Waiting,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SWAIT_TEAMA".to_owned(),
                code: WAITING_TEAM_FIRST_CODE,
                sprite: None,
                rect: Rect {
                    x: 652,
                    y: 65,
                    w: 24,
                    h: 24,
                },
            }],
            HashMap::new(),
        );
        walk_to_waiting(&mut game);
        assert!(game.is_room_master());
        game.ingest_snapshot(&InputSnapshot::click_at(660, 75));
        assert_eq!(game.waiting.as_ref().unwrap().team, 0);
        assert!(game.take_room_team().is_some(), "4004 staged");
        assert!(game.take_team_ready().is_some(), "4008 staged");
        assert!(game.all_peers_ready());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn team_select_without_room_lights_only() {
        let mut game = ScreenGame::new();
        game.select_team(2);
        assert_eq!(game.battle_self_team, Some(2));
        assert!(!game.has_pending_team_send());
        assert_eq!(game.take_room_team(), None);
    }

    #[test]
    fn team_broadcast_guards_slot_below_eight() {
        let game = ScreenGame::new();
        let accepted = RoomTeamChanged {
            slot: 3,
            member_key: 42,
        };
        assert_eq!(game.apply_room_team_changed(&accepted), Some(3));
        let wide = RoomTeamChanged {
            slot: 8,
            member_key: 42,
        };
        assert_eq!(game.apply_room_team_changed(&wide), None);
    }

    #[test]
    fn picked_select_row_highlights_its_letter() {
        // The committed row reads back on the table: only its
        // difficulty letter inks the active orange. Row 0 is the
        // Easy slot (green unpicked), so the probe stays red without
        // the highlight.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        game.load_select_music_scene(Vec::new(), HashMap::new());
        stage_waiting_song(&mut game);
        walk_to_waiting(&mut game);
        game.ingest_snapshot(&InputSnapshot::click_at(400, 150 + 11));
        let buf = paint_waiting_frame(&mut game, 800, 600);
        assert!(ink_near(
            &buf,
            800,
            600,
            (392, 150),
            (12, 12),
            pack_xrgb(0xFF, 0xA0, 0x20)
        ));
        assert!(!ink_near(
            &buf,
            800,
            600,
            (392, 196),
            (12, 12),
            pack_xrgb(0xFF, 0xA0, 0x20)
        ));
    }

    #[test]
    fn click_staged_select_row_commits_song_and_queues_4000() {
        // The host song pick commits through the real dialog: a row
        // click on the staged search table sets the room song (the
        // row's song id plus its difficulty slot) and queues the
        // `SetRoomMusic` 4000 send — the click path, not the direct
        // call. Row 1 is song 757 at the Normal slot; the staged
        // speed survives the pick.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        game.load_select_music_scene(Vec::new(), HashMap::new());
        stage_waiting_song(&mut game);
        walk_to_waiting(&mut game);
        assert!(game.is_room_master());
        game.ingest_snapshot(&InputSnapshot::click_at(400, 150 + 23 + 11));
        let picked = game.selected_music().expect("row click commits the song");
        assert_eq!(picked.music_id, 757);
        assert_eq!(picked.difficulty, 1);
        assert_eq!(picked.speed, 3);
        assert!(game.has_pending_music_send());
        assert_eq!(game.take_music_send(), Some(picked));
        assert!(!game.has_pending_music_send());
        // A click outside the staged rows queues nothing.
        game.ingest_snapshot(&InputSnapshot::click_at(10, 10));
        assert!(!game.has_pending_music_send());
    }

    #[test]
    fn select_row_click_without_master_or_art_commits_nothing() {
        // `MUSELECT 0x16200003` drops non-master clicks, and an
        // art-less master sees no dialog: both click nowhere.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        game.load_select_music_scene(Vec::new(), HashMap::new());
        stage_waiting_song(&mut game);
        walk_to_waiting(&mut game);
        game.waiting.as_mut().expect("joined room").member_id = 4;
        assert!(!game.is_room_master());
        game.ingest_snapshot(&InputSnapshot::click_at(400, 150 + 11));
        assert!(!game.has_pending_music_send());
        let mut bare = ScreenGame::new();
        bare.set_joined_room(waiting_room());
        walk_to_waiting(&mut bare);
        bare.ingest_snapshot(&InputSnapshot::click_at(400, 150 + 11));
        assert!(!bare.has_pending_music_send());
    }

    #[test]
    fn waiting_hand_session_picks_song_then_starts() {
        // Hand-driven song pick through the real window (no gateway):
        // the master clicks row 1 (song 757, Normal slot) through
        // the staged table, the 4000 send stages, and Confirm still
        // runs the START readiness gate into Loading. Smoke: screen
        // path waiting→waiting→loading, 4000 drained, Loading held.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        game.load_select_music_scene(Vec::new(), HashMap::new());
        stage_waiting_song(&mut game);
        walk_to_waiting(&mut game);
        assert!(game.is_room_master());
        game.ingest_snapshot(&InputSnapshot::click_at(400, 150 + 23 + 11));
        assert_eq!(
            game.take_music_send(),
            Some(o2jam_net::SetRoomMusic {
                music_id: 757,
                difficulty: 1,
                speed: 3,
            }),
            "4000 staged"
        );
        assert!(game.all_peers_ready());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn create_room_commit_needs_title_and_stages_2004() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.open_create_room_dialog();
        assert_eq!(game.room_dialog(), Some(RoomDialog::CreateRoom));
        // Empty title commits nothing; typed title proves the commit.
        let placements = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_DCROOM_CREATE".to_owned(),
            code: crate::screens::side::CREATE_ROOM_CREATE_CODE,
            sprite: None,
            rect: Rect {
                x: 10,
                y: 10,
                w: 50,
                h: 30,
            },
        }];
        let figures = HashMap::new();
        game.load_create_room_scene(placements, figures);
        game.click_room_dialog(20, 20);
        assert_eq!(game.room_dialog(), Some(RoomDialog::CreateRoom));
        game.push_room_dialog_char('x');
        game.click_room_dialog(20, 20);
        assert_eq!(game.room_dialog(), None);
        let body = game.take_create_room().expect("committed 2004");
        assert_eq!(body.title, "x");
        assert!(!body.has_password);
    }

    #[test]
    fn create_room_create_clicks_the_parent_offset_rect_with_real_bytes() {
        // Real 7.55 bytes (skips without install-root): CREATE commits
        // at the #114 parent-offset rect, never at the verbatim storage
        // rect (ticket #134). The back sits at (188,71) with CREATE
        // dialog-local at (146,374), so the commit lands at (397,467)
        // while the storage center (209,396) answers nothing.
        use crate::screens::side::CREATE_ROOM_CREATE_CODE;
        use crate::staging::{stage_dialog, Dialog, StageContext};
        let mut game = ScreenGame::new();
        let Ok(bytes) = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image/Interface1.opi"),
        ) else {
            return;
        };
        let art = stage_dialog(&bytes, Dialog::CreateRoom, StageContext::default())
            .art
            .expect("create-room stages with real bytes");
        let placements = art.placements.clone();
        game.load_create_room_scene(art.placements, art.figures);
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.open_create_room_dialog();
        game.push_room_dialog_char('x');
        assert_eq!(
            super::super::paint::dialog_origin(&placements),
            (188, 71),
            "the back rect authors the dialog origin"
        );
        game.click_room_dialog(209, 396);
        assert_eq!(
            game.room_dialog(),
            Some(RoomDialog::CreateRoom),
            "the verbatim storage center is dialog-local, never a hit"
        );
        assert!(game.take_create_room().is_none());
        click_staged_dialog_button(&mut game, &placements, CREATE_ROOM_CREATE_CODE);
        assert_eq!(game.room_dialog(), None);
        let body = game.take_create_room().expect("committed 2004");
        assert_eq!(body.title, "x");
    }

    #[test]
    fn create_click_without_staged_sheet_records_noop_and_keeps_lobby() {
        // The chrome rect stages but the dialog sheet never does (no
        // install-root data): the click must not open a modal it
        // cannot paint — the lobby stays usable and the miss is
        // recorded.
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_CREATE".to_owned(),
                code: 0x1320_0007,
                sprite: None,
                rect: Rect {
                    x: 508,
                    y: 372,
                    w: 96,
                    h: 35,
                },
            }],
            HashMap::new(),
        );
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.ingest_snapshot(&InputSnapshot::click_at(555, 389));
        assert_eq!(game.room_dialog(), None, "unstaged sheet never owns input");
        assert!(
            game.room_control_noop().is_some(),
            "miss recorded with the staging next step"
        );
        assert_eq!(game.screen(), Screen::Room);
    }

    #[test]
    fn locked_row_click_without_staged_sheet_records_noop() {
        // Locked row 1 (room 7) with staged panels but no password
        // sheet: gates the dialog the same way the CREATE chrome
        // does — no invisible modal, lobby stays usable.
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.load_scene(
            Screen::Room,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_EXBUTTON2".to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 26,
                    y: 177,
                    w: 288,
                    h: 91,
                },
            }],
            HashMap::new(),
        );
        game.ingest_snapshot(&InputSnapshot::click_at(100, 200));
        assert_eq!(game.room_highlight, 1);
        assert_eq!(game.room_dialog(), None, "unstaged sheet never owns input");
        assert!(
            game.room_control_noop().is_some(),
            "miss recorded with the staging next step"
        );
    }

    #[test]
    fn password_dialog_commits_join_and_enters_waiting() {
        let mut game = ScreenGame::new();
        // Room screen needed for the dialog commit path to advance.
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.open_password_dialog(7);
        let placements = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_DPASSWORD_OK".to_owned(),
            code: crate::screens::side::PASSWORD_OK_CODE,
            sprite: None,
            rect: Rect {
                x: 10,
                y: 10,
                w: 50,
                h: 30,
            },
        }];
        game.load_password_scene(placements, HashMap::new());
        game.push_room_dialog_char('p');
        game.push_room_dialog_char('w');
        game.click_room_dialog(20, 20);
        assert_eq!(game.room_dialog(), None);
        assert_eq!(game.screen(), Screen::Waiting);
        let joined = game.take_join().expect("committed 3002");
        assert_eq!(joined.number, 7);
        assert_eq!(joined.password, "pw");
    }

    /// Ten-slot scene-BGM bank whose records carry their own number
    /// as the constant sample, so a cue log reads the record off the
    /// pushed PCM. Made-up audio only: tests stay contained.
    fn marker_bank(records: &[i16]) -> Vec<Option<PcmSamples>> {
        let mut bank = vec![None; 10];
        for record in records {
            bank[usize::try_from(*record).expect("record fits a slot") - 1] = Some(PcmSamples {
                samples: vec![*record; 441],
                sample_rate: 44_100,
                channels: 1,
            });
        }
        bank
    }

    /// Null sink that logs each scene-BGM push as its marker sample
    /// (`-1` for a stop), so a test can see which records the shell
    /// cued and when.
    struct CueLog {
        inner: o2jam_audio::NullBackend,
        pushes: Rc<RefCell<Vec<i16>>>,
    }

    impl AudioBackend for CueLog {
        fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), o2jam_audio::AudioError> {
            self.inner.play_pcm(pcm)
        }

        fn set_scene_bgm(
            &mut self,
            pcm: Option<&PcmSamples>,
            repeat: bool,
        ) -> Result<(), o2jam_audio::AudioError> {
            let marker = pcm
                .and_then(|pcm| pcm.samples.first().copied())
                .unwrap_or(-1);
            self.pushes.borrow_mut().push(marker);
            self.inner.set_scene_bgm(pcm, repeat)
        }

        fn scene_bgm_playing(&self) -> bool {
            self.inner.scene_bgm_playing()
        }

        fn samples_played(&self) -> u64 {
            self.inner.samples_played()
        }
    }

    /// Boot a rig-bound game onto the planet board through the splash.
    fn game_on_the_board(pushes: &Rc<RefCell<Vec<i16>>>) -> ScreenGame {
        let mut game = ScreenGame::new();
        game.with_audio(
            Box::new(CueLog {
                inner: o2jam_audio::NullBackend::new(),
                pushes: Rc::clone(pushes),
            }),
            HashMap::new(),
        );
        game.stage_scene_bgm(marker_bank(&[3, 8, 9]));
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        game
    }

    #[test]
    fn emblem_round_trip_keeps_the_shared_board_track_running() {
        // Planet and the channel dialog it opens share cue record 9
        // (`Screen::bgm_cue`): mounting the dialog over the board must
        // not restart the track (ticket #27, null sink only).
        let pushes = Rc::new(RefCell::new(Vec::new()));
        let mut game = game_on_the_board(&pushes);
        assert_eq!(*pushes.borrow(), vec![8, 9], "boot then planet");
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(
            *pushes.borrow(),
            vec![8, 9],
            "the emblem round trip re-pushes nothing"
        );
    }

    #[test]
    fn different_track_and_silence_still_cue_on_the_scene_change() {
        // The keep-running policy never swallows a genuine cue change:
        // Room is record 3, Loading cues silence.
        let pushes = Rc::new(RefCell::new(Vec::new()));
        let mut game = game_on_the_board(&pushes);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        assert_eq!(*pushes.borrow(), vec![8, 9, 3], "room switches to 3");
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        assert_eq!(*pushes.borrow(), vec![8, 9, 3, -1], "loading silences");
    }

    #[test]
    fn scene_bgm_selection_follows_every_screen_entry() {
        // Null backend only (containment): a 10-slot bank with
        // tracks at 8 (Avi), 9 (Planet/Channel), and 3 (Room).
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut bank = vec![None; 10];
        for record in [3, 8, 9] {
            bank[record - 1] = Some(PcmSamples {
                samples: vec![1000i16; 441],
                sample_rate: 44100,
                channels: 1,
            });
        }
        let mut game = ScreenGame::new();
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        game.stage_scene_bgm(bank);
        assert_eq!(game.scene_bgm_selection().map(|sel| sel.record), Some(8));
        game.lane_key(3, true);
        assert_eq!(game.screen(), Screen::Planet);
        assert_eq!(game.scene_bgm_selection().map(|sel| sel.record), Some(9));
        assert!(game.scene_bgm_selection().is_some_and(|sel| sel.repeat));
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(
            game.scene_bgm_selection().map(|sel| sel.record),
            Some(9),
            "dialog shares the planet track"
        );
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        assert_eq!(game.scene_bgm_selection().map(|sel| sel.record), Some(3));
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        assert_eq!(
            game.scene_bgm_selection(),
            None,
            "loading cues silence (leave −1)"
        );
    }

    #[test]
    fn side_visit_and_leave_recue_through_the_single_table() {
        // Null backend only (containment): shop track 4, channel
        // track 9, room track 3, payment shares the room track.
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut bank = vec![None; 10];
        for record in [3, 4, 9] {
            bank[record - 1] = Some(PcmSamples {
                samples: vec![1000i16; 441],
                sample_rate: 44100,
                channels: 1,
            });
        }
        let mut game = ScreenGame::new();
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        game.stage_scene_bgm(bank);
        dismiss_splash_to_dialog(&mut game);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.scene_bgm_selection().map(|sel| sel.record), Some(9));
        assert!(game.visit_side(SideScreen::Shop));
        assert_eq!(
            game.scene_bgm_selection().map(|sel| sel.record),
            Some(4),
            "shop entry cues record 4"
        );
        // A second visit while open is rejected and cues nothing.
        assert!(!game.visit_side(SideScreen::Payment));
        assert_eq!(game.scene_bgm_selection().map(|sel| sel.record), Some(4));
        assert!(game.leave_side());
        assert_eq!(
            game.scene_bgm_selection().map(|sel| sel.record),
            Some(9),
            "returning to the channel lobby re-cues record 9"
        );
        assert!(game.visit_side(SideScreen::Payment));
        assert_eq!(
            game.scene_bgm_selection().map(|sel| sel.record),
            Some(3),
            "payment shares the room track"
        );
        assert!(game.leave_side());
        // Silence-side entry stops the channel; leaving restores it.
        assert!(game.visit_side(SideScreen::Lanking));
        assert_eq!(game.scene_bgm_selection(), None, "lanking cues silence");
        assert!(game.leave_side());
        assert_eq!(game.scene_bgm_selection().map(|sel| sel.record), Some(9));
    }

    fn lanking_record(song_id: u32, bpm: f32, title: &str, artist: &str) -> Vec<u8> {
        let mut rec = vec![0u8; 300];
        rec[8..11].copy_from_slice(b"ojn");
        rec[4..8].copy_from_slice(&song_id.to_le_bytes());
        rec[20..24].copy_from_slice(&bpm.to_le_bytes());
        rec[112..112 + title.len()].copy_from_slice(title.as_bytes());
        rec[176..176 + artist.len()].copy_from_slice(artist.as_bytes());
        let token = format!("o2ma{song_id}.ojm");
        rec[240..240 + token.len()].copy_from_slice(token.as_bytes());
        rec
    }

    fn lanking_catalog_blob() -> Vec<u8> {
        let mut blob = lanking_record(757, 124.0, "Zulu", "BandB");
        blob.extend_from_slice(&lanking_record(770, 168.0, "Alpha", "BandA"));
        blob
    }

    fn lanking_placement(code: u32, x: i32, y: i32, h: i32) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect {
                x,
                y,
                w: 60,
                h: h as u32,
            },
        }
    }

    fn lanking_side_placements() -> Vec<ScenePlacement> {
        // Sort + pager buttons, five left-row hit areas at pitch
        // 84, eleven right-row hit areas (self + ten), and the TEXT
        // templates the overlay inks (all synthetic rects: geometry
        // under test rides staged rects, never literals).
        let mut placements = vec![
            lanking_placement(LANKING_SORT_TITLE_CODE, 10, 10, 12),
            lanking_placement(LANKING_LPAGE_NEXT_CODE, 80, 10, 12),
            lanking_placement(LANKING_LPAGE_PREV_CODE, 150, 10, 12),
            lanking_placement(LANKING_RPAGE_PREV_CODE, 570, 380, 12),
            lanking_placement(LANKING_RPAGE_NEXT_CODE, 640, 380, 12),
        ];
        for slot in 0i32..5 {
            placements.push(lanking_placement(
                LANKING_LVIEW_TOGGLE_FIRST_CODE + slot as u32,
                700,
                100 + slot * 84,
                80,
            ));
        }
        placements.push(lanking_placement(
            LANKING_RVIEW_TOGGLE_FIRST_CODE,
            500,
            400,
            12,
        ));
        for slot in 0i32..10 {
            placements.push(lanking_placement(
                LANKING_RVIEW_TOGGLE_FIRST_CODE + 1 + slot as u32,
                500,
                430 + slot * 20,
                12,
            ));
        }
        for (slot, code) in LANKING_LVIEW_TEXT_CODES.iter().enumerate() {
            placements.push(lanking_placement(*code, 200 + slot as i32 * 70, 100, 12));
        }
        for (slot, code) in LANKING_RVIEW_TEXT_CODES.iter().enumerate() {
            placements.push(lanking_placement(*code, 200 + slot as i32 * 70, 400, 12));
        }
        placements.push(lanking_placement(LANKING_MUSICNAME_CODE, 300, 360, 12));
        placements
    }

    fn open_lanking_board(song_count: usize) -> ScreenGame {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        let mut blob = Vec::new();
        for index in 0..song_count {
            let song_id = 757 + index as u32;
            let title = format!("Song{index:03}");
            let artist = format!("Band{index:03}");
            blob.extend_from_slice(&lanking_record(song_id, 120.0, &title, &artist));
        }
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_lanking_catalog(&entries);
        game.load_side_scene(lanking_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::Lanking));
        game
    }

    #[test]
    fn lanking_sort_click_resorts_the_staged_board() {
        // Catalog order first (Zulu before Alpha); the TITLE sort
        // button at its exe code reorders to Alpha first and the
        // pager resets to the first page.
        let mut game = open_lanking_board(2);
        assert_eq!(game.lanking_visible_songs()[0].title, "Song000");
        game.click_lanking(15, 12);
        assert_eq!(game.lanking_sort(), Some(LankingSort::Title));
        assert_eq!(game.lanking_page(), 0);
        let visible = game.lanking_visible_songs();
        let titles: Vec<&str> = visible.iter().map(|song| song.title.as_str()).collect();
        assert_eq!(titles, vec!["Song000", "Song001"]);
    }

    #[test]
    fn lanking_sort_title_orders_out_of_sequence_catalogs() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        let blob = lanking_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_lanking_catalog(&entries);
        game.load_side_scene(lanking_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::Lanking));
        assert_eq!(game.lanking_visible_songs()[0].title, "Zulu");
        game.click_lanking(15, 12);
        let visible = game.lanking_visible_songs();
        let titles: Vec<&str> = visible.iter().map(|song| song.title.as_str()).collect();
        assert_eq!(titles, vec!["Alpha", "Zulu"]);
    }

    #[test]
    fn lanking_pager_turns_five_row_pages() {
        // Twelve songs: pages show 5/5/2, NEXT holds past the end,
        // PREV steps back.
        let mut game = open_lanking_board(12);
        assert_eq!(game.lanking_visible_songs().len(), 5);
        game.click_lanking(85, 12);
        assert_eq!(game.lanking_page(), 1);
        assert_eq!(game.lanking_visible_songs().len(), 5);
        game.click_lanking(85, 12);
        assert_eq!(game.lanking_page(), 2);
        assert_eq!(game.lanking_visible_songs().len(), 2);
        game.click_lanking(85, 12);
        assert_eq!(game.lanking_page(), 2, "pager holds past the end");
        game.click_lanking(155, 12);
        assert_eq!(game.lanking_page(), 1);
        assert_eq!(game.lanking_visible_songs().len(), 5);
    }

    #[test]
    fn lanking_row_pick_names_the_musicname_caption() {
        // The first-row hit area picks Song000; the overlay inks
        // its title into the staged MUSICNAME rect only.
        let mut game = open_lanking_board(2);
        game.click_lanking(705, 125);
        assert_eq!(game.lanking_selected_title(), Some("Song000".to_string()));
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        let inked = |x: i32, y: i32, w: i32, h: i32| -> bool {
            (y..y + h)
                .any(|row| ((row * 800 + x)..(row * 800 + x + w)).any(|at| buf[at as usize] != 0))
        };
        assert!(
            inked(300, 360, 60, 12),
            "picked title misses its staged rect"
        );
        assert!(
            !inked(300, 300, 60, 12),
            "caption ink leaked outside its rect"
        );
    }

    #[test]
    fn lanking_rview_renders_the_staged_2028_board() {
        // The scripted 2028 stub answers the right list: nicknames
        // ink the staged RVIEW rects; the zeroed fill row inks
        // nothing anywhere.
        use o2jam_net::{MusicPlayRankingAnswer, RankDelta, RankEntry};
        let mut game = open_lanking_board(2);
        let row = |rank: i32, nickname: &str| RankEntry {
            rank,
            nickname: nickname.to_string(),
            battles: 10,
            wins: 7,
            win_rate: 70,
            delta: RankDelta::Up,
            rank_delta: 0,
        };
        game.stage_lanking_board(MusicPlayRankingAnswer {
            self_entry: RankEntry::default(),
            entries: vec![row(1, "Champ"), RankEntry::default(), row(2, "Rival")],
        });
        assert_eq!(game.lanking_visible_ranks().len(), 3);
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        let inked = |x: i32, y: i32, w: i32, h: i32| -> bool {
            (y..y + h)
                .any(|row| ((row * 800 + x)..(row * 800 + x + w)).any(|at| buf[at as usize] != 0))
        };
        assert!(
            inked(270, 430, 60, 12),
            "ranked nickname misses its staged rect"
        );
        assert!(
            inked(270, 470, 60, 12),
            "second ranked nickname misses its staged rect"
        );
        // The blank fill row tiles the middle slot: its NICKNAME
        // rect stays empty.
        assert!(
            !inked(270, 450, 60, 12),
            "zeroed fill row must paint no cells"
        );
    }

    #[test]
    fn lanking_hand_session_sorts_pages_picks_and_renders() {
        // Offline hand-driven rankings session (no gateway): the
        // board opens from Waiting, TITLE sorts, the pager turns,
        // a row toggle picks the caption song, and the scripted
        // 2028 stub answers the right list — every step through
        // the public seams the windowed binary feeds, each
        // mutation moving the observable hash.
        use o2jam_net::{MusicPlayRankingAnswer, RankDelta, RankEntry};
        let mut game = open_lanking_board(12);
        let entered = game.state_hash();
        game.click_lanking(15, 12);
        let sorted = game.state_hash();
        assert_ne!(sorted, entered, "sort click changed nothing");
        game.click_lanking(85, 12);
        let paged = game.state_hash();
        assert_eq!(game.lanking_page(), 1);
        assert_ne!(paged, sorted, "pager click changed nothing");
        game.click_lanking(705, 105);
        let picked = game.state_hash();
        assert!(
            game.lanking_selected_title().is_some(),
            "row toggle picked nothing"
        );
        assert_ne!(picked, paged, "row pick changed nothing");
        game.stage_lanking_board(MusicPlayRankingAnswer {
            self_entry: RankEntry::default(),
            entries: vec![RankEntry {
                rank: 1,
                nickname: "Champ".to_string(),
                battles: 10,
                wins: 7,
                win_rate: 70,
                delta: RankDelta::Up,
                rank_delta: 0,
            }],
        });
        let boarded = game.state_hash();
        assert_ne!(boarded, picked, "board staging changed nothing");
        println!("lanking hand session: {entered:016x} -> {sorted:016x} -> {paged:016x} -> {picked:016x} -> {boarded:016x}");
        assert!(game.leave_side(), "board returns to Waiting");
    }

    fn open_shop_board(good_count: u32) -> ScreenGame {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        let rows: Vec<o2jam_file::AvatarItemRow> = (1..=good_count)
            .map(|item_id| o2jam_file::AvatarItemRow {
                item_id,
                item_type: 0,
                price: item_id * 100,
                params: [0; 4],
                flags: (0, 0),
                name: format!("Good{item_id:02}"),
                description: String::new(),
                sprites: Vec::new(),
            })
            .collect();
        game.set_shop_catalog(&o2jam_file::AvatarItemTable { rows });
        game.load_side_scene(shop_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::Shop));
        game
    }

    fn shop_side_placements() -> Vec<ScenePlacement> {
        // Synthetic `STATE_SHOP` board: eight ITEMBUY hit areas, the
        // scroll with its relative pager children, the cart buy and
        // pagers, five cart slots, and the TEXT templates plus page
        // and wallet cells the overlay inks (geometry under test
        // rides staged rects, never literals).
        let mut placements = Vec::new();
        let area = |code: u32, x: i32, y: i32, w: u32, h: u32| ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        };
        for slot in 0..8u32 {
            placements.push(area(
                SHOP_ITEMBUY_FIRST_CODE + slot,
                220 + (slot % 4) as i32 * 141,
                187 + (slot / 4) as i32 * 153,
                81,
                17,
            ));
            placements.push(area(SHOP_ITEMNAME_FIRST_CODE + slot, 231, 69, 100, 12));
            placements.push(area(SHOP_ITEMWON_FIRST_CODE + slot, 247, 169, 38, 13));
        }
        placements.push(area(SHOP_SCROLL_CODE, 221, 374, 545, 18));
        placements.push(area(SHOP_SCROLL_LEFT_CODE, 0, 0, 18, 18));
        placements.push(area(SHOP_SCROLL_RIGHT_CODE, 527, 0, 18, 18));
        placements.push(area(SHOP_BUYITEM_CODE, 17, 563, 49, 23));
        placements.push(area(SHOP_CART_LEFT_CODE, 198, 564, 47, 22));
        placements.push(area(SHOP_CART_RIGHT_CODE, 317, 564, 47, 22));
        placements.push(area(SHOP_GEMMONEY_CODE, 82, 288, 95, 13));
        placements.push(area(SHOP_CART_PAGE_CUR_TENS_CODE, 247, 567, 14, 17));
        placements.push(area(SHOP_CART_PAGE_CUR_ONES_CODE, 261, 567, 14, 17));
        placements.push(area(SHOP_CART_PAGE_TOT_TENS_CODE, 287, 567, 14, 17));
        placements.push(area(SHOP_CART_PAGE_TOT_ONES_CODE, 301, 567, 14, 17));
        placements
    }

    #[test]
    fn shop_browse_toggles_cart_and_commits_5000() {
        // Ten goods over eight slots: the first ITEMBUY checks row
        // zero into the cart, the cart BUYITEM queues the pinned
        // empty 5000 send, and the seam drains it once.
        let mut game = open_shop_board(10);
        assert!(game.shop_cart().is_empty());
        game.click_shop(221, 188);
        assert_eq!(game.shop_cart(), vec![1]);
        game.click_shop(221, 188);
        assert!(game.shop_cart().is_empty());
        game.click_shop(221, 188);
        game.click_shop(18, 564);
        assert_eq!(game.take_shop_buy(), Some(SyncItemPurchase));
        assert_eq!(game.take_shop_buy(), None);
    }

    #[test]
    fn shop_buyitem_without_cart_queues_nothing() {
        // An empty cart commits nothing: no body reaches the seam.
        let mut game = open_shop_board(10);
        game.click_shop(18, 564);
        assert_eq!(game.take_shop_buy(), None);
    }

    #[test]
    fn shop_pager_turns_eight_row_pages() {
        // Ten goods paint eight per page; the scroll pager steps and
        // holds at the ends.
        let mut game = open_shop_board(10);
        assert_eq!(game.shop_visible_goods().len(), 8);
        game.click_shop(753, 379);
        assert_eq!(game.shop_page(), 1);
        assert_eq!(game.shop_visible_goods().len(), 2);
        game.click_shop(753, 379);
        assert_eq!(game.shop_page(), 1);
        game.click_shop(226, 379);
        assert_eq!(game.shop_page(), 0);
        game.click_shop(226, 379);
        assert_eq!(game.shop_page(), 0);
    }

    #[test]
    fn shop_wallet_gem_caption_waits_for_wire_data() {
        // The GEMMONEY cell reads the staged gem word: placeholder
        // until the 5028 refresh arrives.
        let mut game = open_shop_board(2);
        assert_eq!(game.shop_gem_caption(), "GEM --");
        game.apply_shop_cash_point(&SyncCashPointResponse {
            gem: 1200,
            cash_point: 50,
        });
        assert_eq!(game.shop_gem_caption(), "GEM 1200");
    }

    #[test]
    fn shop_unstaged_clicks_land_nowhere() {
        // No placements and no catalog: every click is a no-op.
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        assert!(game.visit_side(SideScreen::Shop));
        game.click_shop(221, 188);
        assert!(game.shop_cart().is_empty());
        assert_eq!(game.shop_page(), 0);
        assert!(game.shop_visible_goods().is_empty());
        assert_eq!(game.take_shop_buy(), None);
    }

    fn open_music_shop_board(song_count: usize) -> ScreenGame {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        let mut blob = Vec::new();
        for index in 0..song_count {
            let song_id = 757 + index as u32;
            blob.extend_from_slice(&lanking_record(
                song_id,
                120.0,
                &format!("Tune{index:03}"),
                &format!("Band{index:03}"),
            ));
        }
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_music_shop_catalog(&entries);
        game.load_side_scene(music_shop_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::MusicShop));
        game
    }

    fn music_shop_side_placements() -> Vec<ScenePlacement> {
        // Synthetic `STATE_MUSICSHOP` board: five MS_EX slots, the
        // row-local buy templates, sorts, the MSALL pager, the cart
        // buy, and the page-number cells (geometry under test rides
        // staged rects, never literals).
        let mut placements = Vec::new();
        let area = |code: u32, x: i32, y: i32, w: u32, h: u32| ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        };
        for slot in 0..5u32 {
            placements.push(area(
                MSHOP_EX_FIRST_CODE + slot,
                18,
                104 + slot as i32 * 84,
                353,
                86,
            ));
        }
        placements.push(area(MSHOP_GEM_BUY_CODE, 322, 51, 25, 16));
        placements.push(area(MSHOP_WON_BUY_CODE, 322, 67, 25, 16));
        placements.push(area(MSHOP_TITLE_SORT_CODE, 133, 84, 59, 21));
        placements.push(area(MSHOP_DIFF_SORT_CODE, 192, 84, 60, 21));
        placements.push(area(MSHOP_BPM_SORT_CODE, 252, 84, 58, 21));
        placements.push(area(MSHOP_MSALL_LEFT_CODE, 198, 532, 45, 21));
        placements.push(area(MSHOP_MSALL_RIGHT_CODE, 317, 532, 46, 21));
        placements.push(area(MSHOP_CART_BUY_CODE, 423, 528, 49, 23));
        placements.push(area(MSHOP_CART_LEFT_CODE, 542, 529, 47, 22));
        placements.push(area(MSHOP_CART_RIGHT_CODE, 661, 529, 47, 22));
        placements.push(area(MSHOP_PAGE_CUR_TENS_CODE, 246, 534, 14, 17));
        placements.push(area(MSHOP_PAGE_CUR_ONES_CODE, 260, 534, 14, 17));
        placements.push(area(MSHOP_PAGE_TOT_TENS_CODE, 286, 534, 14, 17));
        placements.push(area(MSHOP_PAGE_TOT_ONES_CODE, 300, 534, 14, 17));
        placements
    }

    #[test]
    fn music_shop_sorts_pages_and_buys_5010() {
        // Seven songs over five slots: TITLE sorts, the MSALL pager
        // turns and holds, and the row GEM_BUY queues the pinned
        // empty 5010 send through the seam exactly once.
        let mut game = open_music_shop_board(7);
        assert_eq!(game.music_shop_visible_rows().len(), 5);
        game.click_music_shop(134, 85);
        assert_eq!(game.music_shop_sort(), Some(LankingSort::Title));
        assert_eq!(game.music_shop_page(), 0);
        game.click_music_shop(318, 533);
        assert_eq!(game.music_shop_page(), 1);
        assert_eq!(game.music_shop_visible_rows().len(), 2);
        game.click_music_shop(318, 533);
        assert_eq!(game.music_shop_page(), 1);
        game.click_music_shop(199, 533);
        assert_eq!(game.music_shop_page(), 0);
        game.click_music_shop(341, 156);
        assert_eq!(game.take_music_buy(), Some(SyncMusicPurchase));
        assert_eq!(game.take_music_buy(), None);
    }

    #[test]
    fn music_shop_row_pick_stages_the_cart_and_cart_buy_queues_5010() {
        // The MS_EX row stages the song into the cart; the cart
        // BUYITEM commits it through the same pinned 5010 send.
        let mut game = open_music_shop_board(7);
        game.click_music_shop(19, 105);
        assert_eq!(game.music_shop_cart(), vec![757]);
        game.click_music_shop(424, 529);
        assert_eq!(game.take_music_buy(), Some(SyncMusicPurchase));
        assert_eq!(game.take_music_buy(), None);
    }

    #[test]
    fn music_shop_unstaged_clicks_land_nowhere() {
        // No placements and no catalog: every click is a no-op.
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        assert!(game.visit_side(SideScreen::MusicShop));
        game.click_music_shop(19, 105);
        assert_eq!(game.music_shop_sort(), None);
        assert_eq!(game.music_shop_page(), 0);
        assert!(game.music_shop_visible_rows().is_empty());
        assert_eq!(game.take_music_buy(), None);
    }

    #[test]
    fn shops_hand_session_browses_and_buys() {
        // Offline hand-driven shop session (no gateway): the item
        // board opens from Waiting, the pager turns, a row toggle
        // checks the cart, and the cart BUYITEM queues the pinned
        // 5000 send through the seam; the song board then sorts,
        // pages, stages the cart, and queues the pinned 5010 send —
        // every step through the public seams the windowed binary
        // feeds, each mutation moving the observable hash.
        let mut game = open_shop_board(10);
        let entered = game.state_hash();
        game.click_shop(753, 379);
        let paged = game.state_hash();
        assert_eq!(game.shop_page(), 1);
        assert_ne!(paged, entered, "pager click changed nothing");
        game.click_shop(226, 379);
        game.click_shop(221, 188);
        let checked = game.state_hash();
        assert_eq!(game.shop_cart(), vec![1]);
        assert_ne!(checked, paged, "row toggle changed nothing");
        game.click_shop(18, 564);
        let bought = game.state_hash();
        use o2jam_net::shop::SyncItemPurchase;
        assert_eq!(game.take_shop_buy(), Some(SyncItemPurchase));
        assert_ne!(bought, checked, "buy commit changed nothing");
        assert!(game.leave_side(), "shop returns to Waiting");
        println!(
            "shop hand session: {entered:016x} -> {paged:016x} -> {checked:016x} -> {bought:016x}"
        );
    }

    #[test]
    fn music_shop_hand_session_sorts_pages_and_buys() {
        // Song-board half of the offline session: TITLE sorts, the
        // MSALL pager turns, a row pick stages the cart, and the
        // row GEM_BUY queues the pinned 5010 send through the seam.
        let mut game = open_music_shop_board(7);
        let entered = game.state_hash();
        game.click_music_shop(134, 85);
        let sorted = game.state_hash();
        assert_eq!(game.music_shop_sort(), Some(LankingSort::Title));
        assert_ne!(sorted, entered, "sort click changed nothing");
        game.click_music_shop(318, 533);
        let paged = game.state_hash();
        assert_eq!(game.music_shop_page(), 1);
        assert_ne!(paged, sorted, "pager click changed nothing");
        game.click_music_shop(199, 533);
        game.click_music_shop(19, 105);
        let picked = game.state_hash();
        assert_eq!(game.music_shop_cart(), vec![757]);
        assert_ne!(picked, paged, "row pick changed nothing");
        game.click_music_shop(341, 156);
        let bought = game.state_hash();
        use o2jam_net::shop::SyncMusicPurchase;
        assert_eq!(game.take_music_buy(), Some(SyncMusicPurchase));
        assert_ne!(bought, picked, "buy click changed nothing");
        assert!(game.leave_side(), "song shop returns to Waiting");
        println!("music shop hand session: {entered:016x} -> {sorted:016x} -> {paged:016x} -> {picked:016x} -> {bought:016x}");
    }

    #[test]
    fn lanking_unstaged_clicks_land_nowhere() {
        // No placements and no catalog: every click is a no-op and
        // every view reads empty.
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        assert!(game.visit_side(SideScreen::Lanking));
        game.click_lanking(15, 12);
        assert_eq!(game.lanking_sort(), None);
        assert_eq!(game.lanking_page(), 0);
        assert!(game.lanking_visible_songs().is_empty());
        assert!(game.lanking_visible_ranks().is_empty());
        assert_eq!(game.lanking_selected_title(), None);
    }

    // --- Ticket #163 final legs: side screens + loading/splash ---
    //
    // Side chrome is partially click-silent by design (the ledger's
    // side-state OPEN cells): the sweeps below assert the documented
    // silence per control (no hover state arms anywhere on a side
    // screen; clicks on unwired chrome land nowhere) or pin the wired
    // outcome where the dispatch exists. Unevidenced arms guard the
    // LABELED stand-in with the next rig step named, never an
    // invented behavior.

    fn open_planet_side_overlay() -> ScreenGame {
        // Synthetic planet-side chrome: the solo-mission board
        // stages placements but answers no click yet (OPEN).
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Waiting);
        let area = |code: u32, x: i32, y: i32| ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w: 80, h: 30 },
        };
        game.load_side_scene(
            vec![area(0x1500_0001, 100, 100), area(0x1500_0002, 300, 200)],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Planet));
        game
    }

    fn open_tutorial_overlay() -> ScreenGame {
        // Same synthetic pair behind the tutorial backdrop.
        let mut game = ScreenGame::new();
        walk_to_lobby(&mut game, Screen::Waiting);
        let area = |code: u32, x: i32, y: i32| ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w: 80, h: 30 },
        };
        game.load_side_scene(
            vec![area(0x0010_0001, 100, 100), area(0x0010_0002, 300, 200)],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::Tutorial));
        game
    }

    fn side_chrome_centers(game: &ScreenGame) -> Vec<(i32, i32)> {
        // Every staged placement center: the sweep walks chrome,
        // never literals at the call site.
        game.side_placements
            .iter()
            .map(|placed| {
                (
                    placed.rect.x + placed.rect.w as i32 / 2,
                    placed.rect.y + placed.rect.h as i32 / 2,
                )
            })
            .collect()
    }

    fn assert_no_side_hover(game: &mut ScreenGame, x: i32, y: i32) {
        // No side screen owns hover art: the sweep arms nothing.
        game.ingest_snapshot(&InputSnapshot::motion_to(x, y));
        assert!(game.hovered_planet_action().is_none());
        assert!(game.hovered_channel_row().is_none());
        assert!(game.hovered_room_control().is_none());
    }

    #[test]
    fn side_hover_sweep_arms_no_hover_state() {
        // Ticket #163 (side legs, HOVER): all seven sides open from
        // Waiting; the pointer sweeps every staged control center
        // plus a blank corner and arms no hover state anywhere — the
        // side stays open, the lobby holds. Documented silence: the
        // next evidence step is the rig hover sweep naming a side
        // hover frame, if any exists.
        use super::super::side::ALL_SIDE_SCREENS;
        for side in ALL_SIDE_SCREENS {
            let mut game = match side {
                SideScreen::Shop => open_shop_board(10),
                SideScreen::MusicShop => open_music_shop_board(7),
                SideScreen::MyRoom => myroom_open_overlay(),
                SideScreen::Lanking => open_lanking_board(12),
                SideScreen::Payment => payment_open_overlay(),
                SideScreen::Planet => open_planet_side_overlay(),
                SideScreen::Tutorial => open_tutorial_overlay(),
            };
            assert_eq!(game.side(), Some(side));
            for (x, y) in side_chrome_centers(&game) {
                assert_no_side_hover(&mut game, x, y);
                assert_eq!(game.side(), Some(side), "hover left {side:?}");
                assert_eq!(game.screen(), Screen::Waiting);
            }
            assert_no_side_hover(&mut game, 799, 599);
            assert_eq!(game.side(), Some(side));
            assert_eq!(game.screen(), Screen::Waiting);
        }
    }

    #[test]
    fn side_hover_from_channel_arms_no_channel_row() {
        // Ticket #163 (side legs, HOVER wiring): the channel select
        // bar must not arm under an open side overlay — clicks route
        // to the side layer first, so hover follows (the room
        // resolver's pinned rule). Failing-first: the resolver missed
        // the side gate.
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.load_side_scene(shop_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::Shop));
        for (x, y) in [(300, 240), (300, 290), (524, 246)] {
            game.ingest_snapshot(&InputSnapshot::motion_to(x, y));
            assert_eq!(
                game.hovered_channel_row(),
                None,
                "channel row armed under the shop overlay at ({x},{y})"
            );
        }
        assert_eq!(game.side(), Some(SideScreen::Shop));
    }

    #[test]
    fn lanking_chrome_sweep_presses_every_staged_control() {
        // Ticket #163 (lanking leg, CLICK): press+release through the
        // routed gesture on every staged control — five LVIEW row
        // toggles pick their caption songs, both pagers step and hold
        // at the ends on both lists, and display-only chrome (RVIEW
        // toggles, TEXT templates, MUSICNAME) lands nowhere. Sorts
        // beyond TITLE ride the same `from_code` arm unstaged.
        use o2jam_net::{MusicPlayRankingAnswer, RankEntry};
        let mut game = open_lanking_board(12);
        for (slot, title) in ["Song000", "Song001", "Song002", "Song003", "Song004"]
            .iter()
            .enumerate()
        {
            game.ingest_snapshot(&InputSnapshot::click_at(705, 105 + slot as i32 * 84));
            assert_eq!(
                game.lanking_selected_title(),
                Some((*title).to_owned()),
                "LVIEW row {slot} picked nothing"
            );
        }
        game.ingest_snapshot(&InputSnapshot::click_at(85, 12));
        assert_eq!(game.lanking_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(85, 12));
        assert_eq!(game.lanking_page(), 2);
        game.ingest_snapshot(&InputSnapshot::click_at(85, 12));
        assert_eq!(game.lanking_page(), 2, "NEXT holds past the end");
        game.ingest_snapshot(&InputSnapshot::click_at(155, 12));
        assert_eq!(game.lanking_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(155, 12));
        assert_eq!(game.lanking_page(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(155, 12));
        assert_eq!(game.lanking_page(), 0, "PREV holds at the start");
        game.ingest_snapshot(&InputSnapshot::click_at(15, 12));
        assert_eq!(game.lanking_sort(), Some(LankingSort::Title));
        assert_eq!(game.lanking_page(), 0, "sort resets the page");
        let row = |rank: i32| RankEntry {
            rank,
            ..RankEntry::default()
        };
        game.stage_lanking_board(MusicPlayRankingAnswer {
            self_entry: RankEntry::default(),
            entries: (1..=12).map(row).collect(),
        });
        game.ingest_snapshot(&InputSnapshot::click_at(645, 386));
        assert_eq!(game.lanking.lanking_rpage, 1);
        game.ingest_snapshot(&InputSnapshot::click_at(645, 386));
        assert_eq!(
            game.lanking.lanking_rpage, 1,
            "right NEXT holds past the end"
        );
        game.ingest_snapshot(&InputSnapshot::click_at(605, 386));
        assert_eq!(game.lanking.lanking_rpage, 0);
        game.ingest_snapshot(&InputSnapshot::click_at(605, 386));
        assert_eq!(
            game.lanking.lanking_rpage, 0,
            "right PREV holds at the start"
        );
        let pinned = (
            game.lanking_sort(),
            game.lanking_page(),
            game.lanking_selected_title(),
        );
        for (x, y) in [(505, 405), (505, 455), (205, 105), (305, 365)] {
            game.ingest_snapshot(&InputSnapshot::click_at(x, y));
            assert_eq!(
                (
                    game.lanking_sort(),
                    game.lanking_page(),
                    game.lanking_selected_title()
                ),
                pinned,
                "display-only chrome at ({x},{y}) changed the board"
            );
        }
        assert_eq!(game.side(), Some(SideScreen::Lanking));
    }

    #[test]
    fn shop_chrome_sweep_presses_every_staged_control() {
        // Ticket #163 (shop leg, CLICK): press+release through the
        // routed gesture on every staged control — the empty cart
        // commits nothing, all eight ITEMBUY toggles check and uncheck,
        // the scroll pager turns and holds, BUYITEM queues the pinned
        // 5000 send, the cart pager turns over a full cart, and the
        // display-only chrome (GEMMONEY, page digits, scroll bar body,
        // ITEMNAME/ITEMWON templates) lands nowhere.
        use o2jam_net::shop::SyncItemPurchase;
        let mut game = open_shop_board(10);
        game.ingest_snapshot(&InputSnapshot::click_at(40, 570));
        assert_eq!(game.take_shop_buy(), None, "empty cart commits nothing");
        for slot in 0..8u32 {
            let x = 220 + (slot % 4) as i32 * 141 + 40;
            let y = 187 + (slot / 4) as i32 * 153 + 8;
            game.ingest_snapshot(&InputSnapshot::click_at(x, y));
        }
        assert_eq!(game.shop_cart(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
        game.ingest_snapshot(&InputSnapshot::click_at(261, 196));
        assert_eq!(game.shop_cart(), vec![2, 3, 4, 5, 6, 7, 8]);
        game.ingest_snapshot(&InputSnapshot::click_at(753, 379));
        assert_eq!(game.shop_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(261, 196));
        assert_eq!(game.shop_cart(), vec![2, 3, 4, 5, 6, 7, 8, 9]);
        game.ingest_snapshot(&InputSnapshot::click_at(753, 379));
        assert_eq!(game.shop_page(), 1, "scroll NEXT holds past the end");
        game.ingest_snapshot(&InputSnapshot::click_at(226, 379));
        assert_eq!(game.shop_page(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(226, 379));
        assert_eq!(game.shop_page(), 0, "scroll PREV holds at the start");
        game.ingest_snapshot(&InputSnapshot::click_at(40, 570));
        assert_eq!(game.take_shop_buy(), Some(SyncItemPurchase));
        assert_eq!(game.take_shop_buy(), None, "the seam drains once");
        assert_eq!(game.shop_visible_cart().len(), 5);
        game.ingest_snapshot(&InputSnapshot::click_at(340, 575));
        assert_eq!(game.shop_visible_cart().len(), 3);
        game.ingest_snapshot(&InputSnapshot::click_at(340, 575));
        assert_eq!(game.shop_visible_cart().len(), 3, "cart NEXT holds");
        game.ingest_snapshot(&InputSnapshot::click_at(220, 575));
        assert_eq!(game.shop_visible_cart().len(), 5);
        game.ingest_snapshot(&InputSnapshot::click_at(220, 575));
        assert_eq!(game.shop_visible_cart().len(), 5, "cart PREV holds");
        let pinned = (game.shop_cart(), game.shop_page());
        for (x, y) in [(100, 290), (254, 575), (400, 383), (250, 75), (260, 175)] {
            game.ingest_snapshot(&InputSnapshot::click_at(x, y));
            assert_eq!(
                (game.shop_cart(), game.shop_page()),
                pinned,
                "display-only chrome at ({x},{y}) changed the board"
            );
            assert_eq!(game.take_shop_buy(), None);
        }
        assert_eq!(game.side(), Some(SideScreen::Shop));
    }

    #[test]
    fn music_shop_chrome_sweep_presses_every_staged_control() {
        // Ticket #163 (music-shop leg, CLICK): press+release through
        // the routed gesture on every staged control — all five MS_EX
        // rows stage the cart, the MSALL pager turns and holds, each
        // sort resorts and resets the page, the tiled row buys and the
        // cart BUYITEM queue the pinned 5010 send, the cart pager turns
        // over a full cart, and the page digits land nowhere. PRICE
        // stays display-only with no price source (OPEN).
        use o2jam_net::shop::SyncMusicPurchase;
        let mut game = open_music_shop_board(7);
        for (slot, song_id) in [757, 758, 759, 760, 761].iter().enumerate() {
            // Slot centers: the staged slots tile edge-to-edge, so
            // row tops would re-hit the row above.
            game.ingest_snapshot(&InputSnapshot::click_at(19, 144 + slot as i32 * 84));
            assert!(
                game.music_shop_cart().contains(song_id),
                "MS_EX row {slot} staged nothing"
            );
        }
        game.ingest_snapshot(&InputSnapshot::click_at(318, 533));
        assert_eq!(game.music_shop_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(19, 105));
        assert!(game.music_shop_cart().contains(&762));
        game.ingest_snapshot(&InputSnapshot::click_at(199, 533));
        assert_eq!(game.music_shop_page(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(318, 533));
        assert_eq!(game.music_shop_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(134, 85));
        assert_eq!(game.music_shop_sort(), Some(LankingSort::Title));
        assert_eq!(game.music_shop_page(), 0, "TITLE resets the page");
        game.ingest_snapshot(&InputSnapshot::click_at(200, 90));
        assert_eq!(game.music_shop_sort(), Some(LankingSort::Diff));
        game.ingest_snapshot(&InputSnapshot::click_at(260, 90));
        assert_eq!(game.music_shop_sort(), Some(LankingSort::Bpm));
        game.ingest_snapshot(&InputSnapshot::click_at(341, 156));
        assert_eq!(game.take_music_buy(), Some(SyncMusicPurchase));
        assert_eq!(game.take_music_buy(), None, "the seam drains once");
        game.ingest_snapshot(&InputSnapshot::click_at(350, 179));
        assert_eq!(
            game.take_music_buy(),
            Some(SyncMusicPurchase),
            "tiled WON buy queues 5010"
        );
        game.ingest_snapshot(&InputSnapshot::click_at(440, 535));
        assert_eq!(
            game.take_music_buy(),
            Some(SyncMusicPurchase),
            "cart BUYITEM queues 5010"
        );
        assert_eq!(game.take_music_buy(), None);
        assert_eq!(game.music_shop_visible_cart().len(), 5);
        game.ingest_snapshot(&InputSnapshot::click_at(680, 535));
        assert_eq!(game.music_shop_visible_cart().len(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(680, 535));
        assert_eq!(game.music_shop_visible_cart().len(), 1, "cart NEXT holds");
        game.ingest_snapshot(&InputSnapshot::click_at(560, 535));
        assert_eq!(game.music_shop_visible_cart().len(), 5);
        game.ingest_snapshot(&InputSnapshot::click_at(560, 535));
        assert_eq!(game.music_shop_visible_cart().len(), 5, "cart PREV holds");
        let pinned = (
            game.music_shop_cart(),
            game.music_shop_page(),
            game.music_shop_sort(),
        );
        game.ingest_snapshot(&InputSnapshot::click_at(250, 540));
        assert_eq!(
            (
                game.music_shop_cart(),
                game.music_shop_page(),
                game.music_shop_sort()
            ),
            pinned,
            "page digits changed the board"
        );
        assert_eq!(game.take_music_buy(), None);
        assert_eq!(game.side(), Some(SideScreen::MusicShop));
    }

    #[test]
    fn myroom_chrome_sweep_presses_every_staged_control() {
        // Ticket #163 (myroom leg, CLICK): press+release through the
        // routed gesture on every staged control — slot select, bag
        // cell pick + equip commit (5004), bag/gift pagers turning and
        // holding, gift cell pick (claim with no staged rows queues
        // nothing), the caption landing nowhere, VACATE and GOSHOP as
        // labeled no-ops (send shape OPEN; nested sides rejected), and
        // BACK returning to the lobby last. Slot-filter semantics stay
        // OPEN (`docs/myroom.md`).
        let mut game = myroom_open_overlay();
        // Entry queues the 5017 gift-list fetch (pinned by
        // `myroom_entry_queues_gift_list_fetch`); the sweep drains it
        // so later takes prove the chrome added nothing.
        assert!(game.take_gift_list_request().is_some());
        game.ingest_snapshot(&InputSnapshot::click_at(451, 11));
        assert_eq!(
            game.take_equip_send(),
            None,
            "no picked cell equips nothing"
        );
        game.ingest_snapshot(&InputSnapshot::click_at(501, 11));
        assert_eq!(
            game.take_gift_claim(),
            None,
            "no picked gift claims nothing"
        );
        game.ingest_snapshot(&InputSnapshot::click_at(11, 41));
        assert_eq!(game.take_equip_send(), None);
        assert_eq!(game.take_gift_claim(), None);
        assert_eq!(game.take_gift_list_request(), None);
        assert_eq!(game.myroom().selected_slot(), 0, "VACATE records only");
        game.ingest_snapshot(&InputSnapshot::click_at(61, 41));
        assert_eq!(
            game.side(),
            Some(SideScreen::MyRoom),
            "GOSHOP keeps the room"
        );
        assert_eq!(game.take_equip_send(), None);
        assert_eq!(game.take_gift_claim(), None);
        game.ingest_snapshot(&InputSnapshot::click_at(11, 11));
        assert_eq!(game.myroom().selected_slot(), 3);
        game.ingest_snapshot(&InputSnapshot::click_at(201, 11));
        assert_eq!(game.myroom().bag_cell(), Some(0));
        game.ingest_snapshot(&InputSnapshot::click_at(251, 11));
        assert_eq!(game.myroom().bag_cell(), Some(1));
        game.ingest_snapshot(&InputSnapshot::click_at(451, 11));
        let equip = game.take_equip_send().expect("apply queues the equip send");
        assert_eq!((equip.slot, equip.item_id), (1, 2));
        game.ingest_snapshot(&InputSnapshot::click_at(201, 11));
        game.ingest_snapshot(&InputSnapshot::click_at(451, 11));
        let first = game.take_equip_send().expect("re-apply queues again");
        assert_eq!((first.slot, first.item_id), (0, 1));
        game.ingest_snapshot(&InputSnapshot::click_at(151, 11));
        assert_eq!(game.myroom().bag_page(), 1);
        game.ingest_snapshot(&InputSnapshot::click_at(151, 11));
        assert_eq!(game.myroom().bag_page(), 1, "bag NEXT holds past the end");
        game.ingest_snapshot(&InputSnapshot::click_at(101, 11));
        assert_eq!(game.myroom().bag_page(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(101, 11));
        assert_eq!(game.myroom().bag_page(), 0, "bag PREV holds at the start");
        game.ingest_snapshot(&InputSnapshot::click_at(351, 11));
        assert_eq!(game.myroom().gift_page(), 0, "gift NEXT holds on one page");
        game.ingest_snapshot(&InputSnapshot::click_at(301, 11));
        assert_eq!(game.myroom().gift_page(), 0, "gift PREV holds at the start");
        game.ingest_snapshot(&InputSnapshot::click_at(401, 11));
        assert_eq!(game.myroom().gift_cell(), Some(0));
        game.ingest_snapshot(&InputSnapshot::click_at(501, 11));
        assert_eq!(
            game.take_gift_claim(),
            None,
            "claim with no staged rows records nothing"
        );
        game.ingest_snapshot(&InputSnapshot::click_at(11, 101));
        assert_eq!(game.myroom().selected_slot(), 3, "caption is not a button");
        assert_eq!(game.myroom().bag_page(), 0);
        game.ingest_snapshot(&InputSnapshot::click_at(551, 11));
        assert_eq!(game.side(), None);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn payment_chrome_sweep_clicks_land_nowhere() {
        // Ticket #163 (payment leg, CLICK): the billing views complete
        // no web call, so every staged rect and every blank point
        // routes nothing — the side stays open, the lobby holds, and
        // no send seam queues anything. Labeled stand-in: the click
        // semantics of a real billing view ride a live-gateway replay
        // (next: rig capture of a payment click with Memoryer up).
        let mut game = payment_open_overlay();
        for (x, y) in [(30, 20), (200, 130), (799, 599)] {
            game.ingest_snapshot(&InputSnapshot::click_at(x, y));
            assert_eq!(game.side(), Some(SideScreen::Payment));
            assert_eq!(game.screen(), Screen::Waiting);
        }
        assert!(game.take_equip_send().is_none());
        assert!(game.take_gift_claim().is_none());
        assert!(game.take_gift_list_request().is_none());
        assert!(game.take_create_room().is_none());
        assert!(game.take_join().is_none());
        assert!(game.take_music_send().is_none());
        assert!(game.take_room_team().is_none());
        assert!(game.take_team_ready().is_none());
        assert!(game.take_shop_buy().is_none());
        assert!(game.take_music_buy().is_none());
        assert!(game.leave_side());
    }

    #[test]
    fn planet_side_and_tutorial_clicks_land_nowhere() {
        // Ticket #163 (planet-side + tutorial legs, CLICK): the
        // solo-mission board and the tutorial backdrop stage art but
        // answer no control yet — clicks on staged chrome and blank
        // points route nothing, queue nothing, and keep the side open.
        // Labeled stand-ins: the solo-entry edge stays OPEN
        // (`docs/planet.md` OPEN 6); the next step names the rig
        // capture of a planet-side emblem press.
        for mut game in [open_planet_side_overlay(), open_tutorial_overlay()] {
            let side = game.side();
            for (x, y) in [(140, 115), (340, 215), (799, 599)] {
                game.ingest_snapshot(&InputSnapshot::click_at(x, y));
                assert_eq!(game.side(), side);
                assert_eq!(game.screen(), Screen::Waiting);
            }
            assert!(game.take_equip_send().is_none());
            assert!(game.take_gift_claim().is_none());
            assert!(game.take_gift_list_request().is_none());
            assert!(game.take_create_room().is_none());
            assert!(game.take_join().is_none());
            assert!(game.take_music_send().is_none());
            assert!(game.take_shop_buy().is_none());
            assert!(game.take_music_buy().is_none());
            assert!(game.leave_side());
        }
    }

    #[test]
    fn loading_chrome_absence_sweep_routes_no_click_or_hover() {
        // Ticket #163 (loading leg): the gate has no chrome — clicks
        // anywhere (corners, center, the staged READY rect) route
        // nothing and arm no hover state; the screen holds Loading.
        // Deliberate: menu keys still route (BACK returns below), so
        // the absence pins clicks, not input.
        let mut game = waiting_game_with_button(
            "IDC_BUTTON_SWAIT_READY",
            Rect {
                x: 600,
                y: 100,
                w: 80,
                h: 30,
            },
        );
        game.ingest_snapshot(&InputSnapshot::click_at(640, 115));
        assert_eq!(game.screen(), Screen::Loading);
        for (x, y) in [
            (0, 0),
            (799, 0),
            (0, 599),
            (799, 599),
            (400, 300),
            (640, 115),
        ] {
            game.ingest_snapshot(&InputSnapshot::click_at(x, y));
            assert_eq!(
                game.screen(),
                Screen::Loading,
                "click at ({x},{y}) left Loading"
            );
            assert_no_side_hover(&mut game, x, y);
            assert_eq!(game.screen(), Screen::Loading);
        }
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn splash_chrome_absence_sweep_every_click_lands_the_same_board() {
        // Ticket #163 (splash leg): the splash has no chrome — no
        // hover state arms anywhere and every canvas click dismisses
        // to the same bare board (no per-control routing).
        // Deliberate: the whole-screen dismiss is the gesture, not a
        // control.
        let mut hashes = Vec::new();
        for (x, y) in [(0, 0), (799, 0), (0, 599), (799, 599), (400, 300)] {
            let mut game = ScreenGame::new();
            assert_eq!(game.screen(), Screen::Avi);
            assert_no_side_hover(&mut game, x, y);
            assert_eq!(game.screen(), Screen::Avi);
            game.ingest_snapshot(&InputSnapshot::click_at(x, y));
            assert_eq!(game.screen(), Screen::Planet);
            hashes.push(game.state_hash());
        }
        assert!(
            hashes.windows(2).all(|pair| pair[0] == pair[1]),
            "all splash clicks converge on one board"
        );
    }

    #[test]
    fn finished_song_enters_result_and_cues_silence_until_its_bank_is_found() {
        // Null backend only (containment): playing track 3 cues
        // while playing; result's `0x13` names no `BGM.ojm` record
        // (10 records), so the song-end entry silences the channel.
        // OPEN: identify which bank the worker holds when `0x4C1B90`
        // cues it (`BGMwinter.ojm` candidate) — never invent a track.
        use o2jam_audio::{NullBackend, PcmSamples};
        let mut bank = vec![None; 10];
        bank[2] = Some(PcmSamples {
            samples: vec![1000i16; 441],
            sample_rate: 44100,
            channels: 1,
        });
        let mut game = ScreenGame::new();
        game.with_audio(Box::new(NullBackend::new()), HashMap::new());
        game.stage_scene_bgm(bank);
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(game.scene_bgm_selection(), None, "playing cues silence");
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            7000,
        );
        assert_eq!(game.screen(), Screen::Result);
        assert_eq!(
            game.scene_bgm_selection(),
            None,
            "result entry cues `0x13` once: silent on a 10-record bank"
        );
    }

    #[test]
    fn set_member_ready_marks_seats_in_place() {
        // Ready folds must not rebuild the room (which clears chat).
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        game.set_member_ready(0, false);
        let room = game.waiting.as_ref().expect("joined room");
        assert!(!room.slots[0].member.as_ref().expect("host").ready);
        game.set_member_ready(0, true);
        let room = game.waiting.as_ref().expect("joined room");
        assert!(room.slots[0].member.as_ref().expect("host").ready);
        // Unknown members and no room stay silent no-ops.
        game.set_member_ready(7, true);
        let mut bare = ScreenGame::new();
        bare.set_member_ready(0, true);
        assert!(bare.waiting.is_none());
    }

    #[test]
    fn scene_art_backs_waiting_seats() {
        let (placements, figures) = corner_scene_art("Wait_TeamA.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Waiting, placements, figures);
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        let mut buf = vec![0u32; 80 * 64];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn scene_art_overlays_playing_without_hiding_notes() {
        let (placements, figures) = corner_scene_art("Playing_Panel.ojs");
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 1500, 7)]);
        game.load_scene(Screen::Playing, placements, figures);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            1000,
        );
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x40, 0xC0, 0xFF)));
    }

    #[test]
    fn staged_backdrop_replaces_the_loop_placeholder_fill() {
        // A44: the staged scene set blits through the game loop —
        // the behind-highway backdrop paints first, so the loop's
        // flat placeholder fill (ADR-0003's no-archive fallback)
        // stays out and the staged art shows through whole.
        let (placements, figures) = corner_scene_art("Playing_BG.ojs");
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 1500, 7)]);
        game.load_scene(Screen::Playing, placements, figures);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            1000,
        );
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(
            !buf.contains(&pack_xrgb(0x0A, 0x0A, 0x0A)),
            "the loop's placeholder fill paints over/under the staged backdrop"
        );
    }

    #[test]
    fn scene_art_backs_result_without_small_frame_digits() {
        let (placements, figures) = corner_scene_art("Result_WinLose.ojs");
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        game.load_scene(Screen::Result, placements, figures);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            7000,
        );
        assert_eq!(game.screen(), Screen::Result);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        // Small frame: the pinned number fields stay off (the solo
        // banner body paints its corner layer instead).
        assert_ne!(buf[16 * 70 + 16], pack_xrgb(0xFF, 0xFF, 0xFF));
    }

    #[test]
    fn scene_art_backs_loading_bar() {
        use MenuKey;
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
    }

    fn battle_teams_self_wins() -> o2jam_game::RoomTeams {
        let mut teams = o2jam_game::RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.submit(0, 100_000);
        teams.submit(1, 150_000);
        teams
    }

    fn finish_song(game: &mut ScreenGame) {
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(game);
        game.strike_at(2, 5000);
    }

    fn paint_result_frame(game: &mut ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 70 * 70];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 70,
            },
            7000,
        );
        buf
    }

    #[test]
    fn result_without_battle_claim_paints_no_verdict() {
        let mut game = ScreenGame::new();
        finish_song(&mut game);
        assert_eq!(game.result_verdict(), ResultVerdict::Solo);
        let buf = paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        // The retired caption inks never come back; the banner is a
        // sprite frame of the staged art (none here).
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_verdict_is_battle_only_in_team_play() {
        // No room: solo banner. A joined Live room carries the
        // rank-1 verdict once teams and the claim resolve.
        let mut game = ScreenGame::new();
        assert_eq!(game.result_verdict(), ResultVerdict::Solo);
        let mut live = waiting_room();
        live.game_mode = o2jam_net::GameMode::Live as u8;
        game.set_joined_room(live);
        assert_eq!(game.result_verdict(), ResultVerdict::Battle(None));
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        assert_eq!(
            game.result_verdict(),
            ResultVerdict::Battle(Some(BattleVerdict::Win))
        );
        game.set_battle_self_team(Some(0));
        assert_eq!(
            game.result_verdict(),
            ResultVerdict::Battle(Some(BattleVerdict::Lose))
        );
    }

    #[test]
    fn result_rows_carry_local_score_solo_without_rank_or_speed() {
        // Solo offline: one local row only. The rank hides (2+
        // players only) and speed waits for a room; the numbers
        // ride the live score, the bonus the live hold
        // accumulator (zero with no holds — live, not invented).
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, Some("YOU".to_owned()));
        assert_eq!(rows[0].rank, None);
        assert_eq!(rows[0].speed, None);
        assert_eq!(rows[0].judgements, Some([1, 0, 0, 0]));
        assert_eq!(rows[0].combos, Some((1, 0)));
        assert_eq!(rows[0].score, Some(200));
        assert_eq!(rows[0].bonus, Some(0));
    }

    #[test]
    fn result_rows_name_peers_and_share_the_room_speed() {
        // Joined room with the host seated: self plus the wire
        // member, both naming the shared room speed code.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, Some("YOU".to_owned()));
        assert_eq!(rows[1].name, Some("host".to_owned()));
        assert_eq!(rows[0].speed, Some(1));
        assert_eq!(rows[1].speed, Some(1));
    }

    #[test]
    fn result_rows_leave_peer_numbers_unpainted() {
        // Per-member 4014/4016 tallies stay server-side: only the
        // local row carries numbers; the peer row paints its name
        // (and speed/rank when live) with every number unpainted.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let rows = game.result_rank_rows();
        assert!(rows[0].judgements.is_some());
        assert!(rows[0].combos.is_some());
        assert!(rows[0].score.is_some());
        assert!(rows[0].bonus.is_some());
        assert_eq!(rows[1].judgements, None);
        assert_eq!(rows[1].combos, None);
        assert_eq!(rows[1].score, None);
        assert_eq!(rows[1].bonus, None);
    }

    #[test]
    fn result_rows_hide_ranks_until_a_broadcast_lands() {
        // The song path never seeds stored ranks, so the mirror
        // holds zeros and both rows stay rank-less (the zero-byte
        // convention) — never a fabricated standing.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let rows = game.result_rank_rows();
        assert_eq!(rows[0].rank, None);
        assert_eq!(rows[1].rank, None);
    }

    #[test]
    fn result_rows_show_scripted_ranks_after_a_staged_broadcast() {
        // Scripted walk fixture: the local seat holds rank 1, the
        // wire peer rank 2 — the broadcast delivers each byte to its
        // holder through the rank-remap rule, and only those rows
        // paint (trailing zeros stay hidden).
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        let deliveries = game.stage_scripted_ranks(
            0xFF,
            [1, 2, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            [1, 2, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(deliveries.len(), 2);
        let rows = game.result_rank_rows();
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[1].rank, Some(2));
    }

    #[test]
    fn result_rows_skip_vacant_seats_and_cap_at_eight() {
        // The fixture seats one member beside two vacant slots, so
        // only two rows build; seven occupied wire slots cap the
        // table at eight (self plus seven).
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        assert_eq!(game.result_rank_rows().len(), 2);
        let mut full = waiting_room();
        full.slots = (0..7)
            .map(|index| RoomSlot {
                index,
                state_code: 1,
                member: Some(RoomMember {
                    nickname: format!("peer{index}"),
                    level: 1,
                    gender: 0,
                    gem: 0,
                    master: false,
                    team: 0,
                    ready: true,
                    music_state: 0,
                    equipment: [0; 17],
                    music_ids: Vec::new(),
                    cash_point: 0,
                    free_pass: 0,
                    playing_state: 0,
                    administrator: false,
                    super_manager: false,
                }),
            })
            .collect();
        game.set_joined_room(full);
        let rows = game.result_rank_rows();
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[7].name, Some("peer6".to_owned()));
    }

    #[test]
    fn grade_runs_the_pinned_table_on_the_chart() {
        // One-note chart fully Cool: the reference word holds a single
        // record, so `grade_max_reference` is zero and the plate stays
        // off — no chart can read as graded below 27 records
        // (PINNED `FUN_004c22c0` minus 26).
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.strike_at(2, 5000);
        assert_eq!(game.grade(), None);
        assert_eq!(game.playing.score().points, 200);
    }

    #[test]
    fn grade_stays_below_the_lowest_threshold_on_a_miss() {
        // The timeout miss floors the points at zero: ratio 0 stays
        // under 0.30, the plate's empty state.
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let mut buf = vec![0u32; 70 * 60];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            5_100,
        );
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        assert_eq!(game.playing.score().counts[3], 1);
        assert_eq!(game.grade(), None);
    }

    #[test]
    fn grade_needs_a_chart() {
        // No chart, no ratio: the plate waits (frame None).
        let game = ScreenGame::new();
        assert_eq!(game.grade(), None);
    }

    #[test]
    fn result_win_paints_no_invented_verdict_for_rank_one() {
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        assert_eq!(game.battle_verdict(), Some(BattleVerdict::Win));
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        // The retired text verdict never comes back; the banner is a
        // sprite frame of the staged art (none in this flat frame).
        assert_ne!(buf[48 * 70 + 16], pack_xrgb(0xFF, 0xD0, 0x20));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn result_lose_paints_no_invented_verdict_off_rank_one() {
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(0));
        assert_eq!(game.battle_verdict(), Some(BattleVerdict::Lose));
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        assert_ne!(buf[48 * 70 + 16], pack_xrgb(0x90, 0x90, 0x90));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn result_scene_art_paints_without_the_invented_verdict() {
        use crate::screens::paint::unbound_scene_placements;
        let names = vec![
            "Winner_image.ojs".to_owned(),
            "Result_WinLose.ojs".to_owned(),
        ];
        let figures = HashMap::from([
            (names[0].clone(), red_scene_figure()),
            (names[1].clone(), red_scene_figure()),
        ]);
        let placements = unbound_scene_placements(&names);
        let mut game = ScreenGame::new();
        game.set_battle_teams(battle_teams_self_wins());
        game.set_battle_self_team(Some(1));
        game.load_scene(Screen::Result, placements, figures);
        finish_song(&mut game);
        let buf = paint_result_frame(&mut game);
        // The staged winner/winlose art paints; the retired text
        // caption ink does not.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    /// Result scene with one staged button behind `id`: the bound
    /// arm pairs the rect by code ordinal, so one placement stages
    /// the hit-test without the archive.
    fn stage_result_button(game: &mut ScreenGame, id: &str, rect: Rect) {
        game.load_scene(
            Screen::Result,
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: id.to_owned(),
                code: 0,
                sprite: None,
                rect,
            }],
            HashMap::new(),
        );
    }

    #[test]
    fn staged_retry_click_returns_to_waiting() {
        // RETRY (`IDC_BUTTON_SRESULT_RETRY`, rect 6
        // (625,570)-(708,592)): the walk stages the button through
        // the bound arm, the click hands back to Waiting — and
        // queues the exe's relay body (4010 `0xfaa`, empty) for the
        // session bridge.
        let mut game = ScreenGame::new();
        stage_result_button(&mut game, RESULT_RETRY_ID, RESULT_RETRY_RECT);
        finish_song(&mut game);
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        game.ingest_snapshot(&InputSnapshot::click_at(666, 581));
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(game.take_result_retry().is_some());
        assert!(game.take_result_retry().is_none(), "one take sends once");
    }

    #[test]
    fn staged_back_click_returns_to_room() {
        // BACK (`IDC_BUTTON_SRESULT_BACK`, rect 5
        // (711,569)-(794,593)): the walk stages the button, the
        // click leaves for the room list — and queues the exe's
        // relay body (4021 `0xfb5`, empty) for the session bridge.
        let mut game = ScreenGame::new();
        stage_result_button(&mut game, RESULT_BACK_ID, RESULT_BACK_RECT);
        finish_song(&mut game);
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        game.ingest_snapshot(&InputSnapshot::click_at(752, 581));
        assert_eq!(game.screen(), Screen::Room);
        assert!(game.take_result_back().is_some());
        assert!(game.take_result_back().is_none(), "one take sends once");
    }

    #[test]
    fn result_keys_mirror_the_staged_buttons() {
        // Confirm/Back are the keyboard arms of RETRY/BACK: the
        // pinned fallback rects answer while the buttons stage
        // nothing, and a miss lands nowhere.
        use MenuKey;
        let mut game = ScreenGame::new();
        finish_song(&mut game);
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        game.ingest_snapshot(&InputSnapshot::click_at(0, 0));
        assert_eq!(game.screen(), Screen::Result);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        let mut back = ScreenGame::new();
        finish_song(&mut back);
        paint_result_frame(&mut back);
        back.menu_key(MenuKey::Back, true);
        assert_eq!(back.screen(), Screen::Room);
    }

    #[test]
    fn result_retry_and_back_queue_the_pinned_relay_bodies() {
        // The keyboard arms queue the same pinned bodies as the
        // buttons: Confirm on Result queues `StartGame` 4010
        // (`0xfaa`, empty — the exe's result RETRY relay event,
        // `FUN_004c2f00` range, `ev_77ce98f1…`), Back queues
        // `ExitPlaying` 4021 (`0xfb5`, empty — `ev_77ce98f1…`).
        use MenuKey;
        let mut game = ScreenGame::new();
        finish_song(&mut game);
        paint_result_frame(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(game.take_result_retry().is_some());
        assert!(game.take_result_back().is_none());
        let mut back = ScreenGame::new();
        finish_song(&mut back);
        paint_result_frame(&mut back);
        back.menu_key(MenuKey::Back, true);
        assert_eq!(back.screen(), Screen::Room);
        assert!(back.take_result_back().is_some());
        assert!(back.take_result_retry().is_none());
    }

    #[test]
    fn result_entry_stages_the_pinned_ticket_wire_call() {
        // Result entry arms split by mode: a battle (Live) entry
        // queues `GetLiveState` 4042 (`0xfca`, empty — byte-verified,
        // `docs/relay.md` §10, the exe's battle-global-gated send);
        // a solo entry queues nothing on the gateway seam — its
        // event-ticket web call (`ticket_add.php?USER_INDEX_ID=<index>`,
        // file `0x24a338`) is an HTTP GET, not a gateway frame, and
        // the shell owns no web seam, so it stays OPEN (next steps:
        // the web-seam decision, then the live request-line capture).
        let mut game = ScreenGame::new();
        finish_song(&mut game);
        paint_result_frame(&mut game);
        assert!(game.take_result_battle_entry().is_none());
        let mut live = ScreenGame::new();
        let mut room = waiting_room();
        room.game_mode = o2jam_net::GameMode::Live as u8;
        live.set_joined_room(room);
        finish_song(&mut live);
        paint_result_frame(&mut live);
        assert!(
            live.take_result_battle_entry().is_some(),
            "battle entry queues the live-state ask"
        );
        assert!(
            live.take_result_battle_entry().is_none(),
            "one take sends once"
        );
        // The web call is not a gateway frame: no 4010/4021/4042 arm
        // may stand in for it.
        assert!(game.take_result_retry().is_none());
        assert!(game.take_result_back().is_none());
        assert!(game.take_room_team().is_none());
        assert!(game.take_team_ready().is_none());
        assert!(game.take_music_send().is_none());
        assert!(game.take_create_room().is_none());
        assert!(game.take_join().is_none());
    }

    #[test]
    fn populated_outbox_pins_the_observable_state_hash() {
        // Characterization for the PendingOutbox extraction (#139,
        // cluster 1) extended by the LankingState extraction
        // (cluster 2), the ShopState population (cluster 3), the
        // MusicShopState population (cluster 4), and the planet-board
        // population (cluster 5): all thirteen wire queues populated
        // through their queueing arms plus the lanking
        // catalog/sort/page/pick/board, the shop
        // catalog/toggle/page/cart-page, the music-shop
        // catalog/sort/page/board/pick/cart-page, and the planet
        // band/promo/carousel/modal/press populations below, then
        // the observable hash pinned byte-identical. The extraction
        // must not move one hash bit:
        // the take_* drains below prove every arm queued, the pinned
        // word proves the hash fold is untouched. Only the two shop
        // sends feed the fold today; the rest ride along so a future
        // fold change re-pins here.
        use super::super::side::{
            MYROOM_APPLY_CODE, MYROOM_BAG_FIRST_CODE, MYROOM_GIFT_FIRST_CODE, MYROOM_MOVE_CODE,
        };
        use MenuKey;
        let mut game = ScreenGame::new();

        // Battle entry first: the song-end arm needs the offline
        // walk, so the Live room stages before the first finish.
        let mut live = waiting_room();
        live.game_mode = o2jam_net::GameMode::Live as u8;
        game.set_joined_room(live);
        finish_song(&mut game);
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);

        // Result arms: BACK queues 4021 (leaves for Room), then the
        // re-walked song queues 4042 again and Confirm queues 4010.
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Room);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        finish_song(&mut game);
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);

        // Team arms: the toggle queues 4004 + 4008.
        game.select_team(5);
        // Song-pick arm: the staged row click queues 4000.
        game.load_select_music_scene(Vec::new(), HashMap::new());
        stage_waiting_song(&mut game);
        game.ingest_snapshot(&InputSnapshot::click_at(400, 150 + 23 + 11));

        // Avatar-room arms: entry queues the 5017 fetch, the bag
        // APPLY queues 5004, the gift MOVE queues 5023.
        game.set_myroom_items(myroom_test_table());
        let mut character = myroom_test_character((1..=12).collect());
        character.item_gifts.push(o2jam_net::GiftItemInfo {
            gift_id: 7,
            item_id: 1,
            sender: "Giver".to_owned(),
        });
        game.set_myroom_character(character);
        game.load_side_scene(
            vec![
                myroom_hit_placement(MYROOM_BAG_FIRST_CODE, 200, 10),
                myroom_hit_placement(MYROOM_APPLY_CODE, 450, 10),
                myroom_hit_placement(MYROOM_GIFT_FIRST_CODE, 400, 10),
                myroom_hit_placement(MYROOM_MOVE_CODE, 500, 10),
            ],
            HashMap::new(),
        );
        assert!(game.visit_side(SideScreen::MyRoom));
        game.ingest_snapshot(&InputSnapshot::click_at(201, 11));
        game.ingest_snapshot(&InputSnapshot::click_at(451, 11));
        game.ingest_snapshot(&InputSnapshot::click_at(401, 11));
        game.ingest_snapshot(&InputSnapshot::click_at(501, 11));
        assert!(game.leave_side());

        // Item-shop arm: the row toggle plus cart BUYITEM queues 5000.
        let rows: Vec<o2jam_file::AvatarItemRow> = (1..=10)
            .map(|item_id| o2jam_file::AvatarItemRow {
                item_id,
                item_type: 0,
                price: item_id * 100,
                params: [0; 4],
                flags: (0, 0),
                name: format!("Good{item_id:02}"),
                description: String::new(),
                sprites: Vec::new(),
            })
            .collect();
        game.set_shop_catalog(&o2jam_file::AvatarItemTable { rows });
        game.load_side_scene(shop_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::Shop));
        // Shop arm (#139 cluster 3): the catalog + row toggle +
        // item pager step + cart pager hold + cart BUYITEM feed the
        // hash fold; the extraction must not move one hash bit.
        game.click_shop(221, 188);
        assert_eq!(game.shop_cart(), vec![1]);
        game.click_shop(753, 379);
        assert_eq!(game.shop_page(), 1);
        game.click_shop(340, 575);
        assert_eq!(game.shop_cart(), vec![1]);
        game.click_shop(18, 564);
        assert!(game.leave_side());

        // Song-shop arm (#139 cluster 4): the catalog + TITLE sort +
        // MSALL pager step + staged 4031 board + row picks on both
        // pages + cart pager step + cart BUYITEM feed the hash fold;
        // the extraction must not move one hash bit.
        let mut blob = Vec::new();
        for index in 0..7 {
            let song_id = 757 + index as u32;
            blob.extend_from_slice(&lanking_record(
                song_id,
                120.0,
                &format!("Tune{index:03}"),
                &format!("Band{index:03}"),
            ));
        }
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_music_shop_catalog(&entries);
        game.stage_music_shop_board(o2jam_net::play::MusicListAnswer {
            entries: (757..764)
                .map(|id| o2jam_net::play::MusicListEntry {
                    id: id as u16,
                    notes_ex: 100,
                    notes_nx: 200,
                    notes_hx: 300,
                })
                .collect(),
            free_music: false,
        });
        game.load_side_scene(music_shop_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::MusicShop));
        game.click_music_shop(134, 85);
        assert_eq!(game.music_shop_sort(), Some(LankingSort::Title));
        game.click_music_shop(318, 533);
        assert_eq!(game.music_shop_page(), 1);
        game.click_music_shop(19, 105);
        game.click_music_shop(19, 231);
        assert_eq!(game.music_shop_cart().len(), 2);
        game.click_music_shop(199, 533);
        assert_eq!(game.music_shop_page(), 0);
        game.click_music_shop(19, 105);
        game.click_music_shop(19, 231);
        game.click_music_shop(19, 315);
        game.click_music_shop(19, 399);
        game.click_music_shop(19, 483);
        assert_eq!(game.music_shop_cart().len(), 7);
        game.click_music_shop(662, 530);
        game.click_music_shop(424, 529);
        assert!(game.leave_side());

        // Lanking arm (#139 cluster 2): the catalog + TITLE sort +
        // left pager step + row pick + staged 2028 board + right
        // pager step feed the hash fold; the extraction must not
        // move one hash bit.
        let mut blob = Vec::new();
        for index in 0..12u32 {
            blob.extend_from_slice(&lanking_record(
                757 + index,
                120.0,
                &format!("Song{index:03}"),
                &format!("Band{index:03}"),
            ));
        }
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_lanking_catalog(&entries);
        game.load_side_scene(lanking_side_placements(), HashMap::new());
        assert!(game.visit_side(SideScreen::Lanking));
        game.click_lanking(15, 12);
        assert_eq!(game.lanking_sort(), Some(LankingSort::Title));
        game.click_lanking(85, 12);
        assert_eq!(game.lanking_page(), 1);
        game.click_lanking(705, 105);
        assert!(game.lanking_selected_title().is_some());
        game.stage_lanking_board(o2jam_net::MusicPlayRankingAnswer {
            self_entry: o2jam_net::RankEntry::default(),
            entries: (1..=12)
                .map(|rank| o2jam_net::RankEntry {
                    rank,
                    nickname: format!("Rival{rank:02}"),
                    battles: 10,
                    wins: 7,
                    win_rate: 70,
                    delta: o2jam_net::RankDelta::Up,
                    rank_delta: 0,
                })
                .collect(),
        });
        game.click_lanking(645, 382);
        assert_eq!(game.lanking_visible_ranks().len(), 2);
        assert!(game.leave_side());

        // Create-room arm: the dialog commit queues 2004.
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Room);
        game.open_create_room_dialog();
        game.load_create_room_scene(
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_DCROOM_CREATE".to_owned(),
                code: crate::screens::side::CREATE_ROOM_CREATE_CODE,
                sprite: None,
                rect: Rect {
                    x: 10,
                    y: 10,
                    w: 50,
                    h: 30,
                },
            }],
            HashMap::new(),
        );
        game.push_room_dialog_char('x');
        game.click_room_dialog(20, 20);
        assert_eq!(game.room_dialog(), None);

        // Password-join arm: the OK commit queues 3002 (Waiting).
        game.open_password_dialog(7);
        game.load_password_scene(
            vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_DPASSWORD_OK".to_owned(),
                code: crate::screens::side::PASSWORD_OK_CODE,
                sprite: None,
                rect: Rect {
                    x: 10,
                    y: 10,
                    w: 50,
                    h: 30,
                },
            }],
            HashMap::new(),
        );
        game.click_room_dialog(20, 20);
        assert_eq!(game.screen(), Screen::Waiting);

        // Planet-board arm (#139 cluster 5): the promo LEFT press
        // (records + wraps the 3-frame banner) plus the STOP pair
        // (spin off, re-armed), the gear modal open + Escape close,
        // the HARD band pick (via Channel and back), and a held
        // promo press feed the hash fold; the extraction must not
        // move one hash bit.
        use crate::avatar::FigureLayer;
        use crate::screens::side::PlanetBand;
        use crate::screens::side::{
            PLANET_PROMO_IMAGE_CODE, PLANET_PROMO_LEFT_CODE, PLANET_PROMO_STOP_CODE,
        };
        game.menu_key(MenuKey::Back, true);
        game.menu_key(MenuKey::Back, true);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);
        let banner = "Planet_Promotion.ojs";
        let mut frames = AvatarFigure::default();
        for word in [0x7C00u16, 0x03E0u16, 0x001Fu16] {
            frames.push_layer(FigureLayer {
                image: o2jam_render::Rgb555Image::decode(&word.to_le_bytes().repeat(16), 4, 4)
                    .expect("promo frames"),
                dx: 49,
                dy: 163,
                colorkey: 0xFFFF,
            });
        }
        game.load_scene(
            Screen::Planet,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-banner".to_owned(),
                    code: PLANET_PROMO_IMAGE_CODE,
                    sprite: Some(banner.to_owned()),
                    rect: Rect {
                        x: 49,
                        y: 163,
                        w: 701,
                        h: 333,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-left".to_owned(),
                    code: PLANET_PROMO_LEFT_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 353,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "promo-stop".to_owned(),
                    code: PLANET_PROMO_STOP_CODE,
                    sprite: None,
                    rect: Rect {
                        x: 387,
                        y: 511,
                        w: 26,
                        h: 24,
                    },
                },
            ],
            HashMap::from([(banner.to_owned(), frames)]),
        );
        game.click_planet(366, 523);
        assert_eq!(game.planet_promo(), Some(PlanetPromo::Left));
        assert_eq!(game.planet_promo_index(), 3);
        game.click_planet(400, 523);
        assert!(!game.planet_promo_spinning());
        game.click_planet(400, 523);
        assert!(game.planet_promo_spinning());
        game.click_planet(729, 580);
        assert_eq!(game.planet_modal(), Some(PlanetModal::Options));
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.planet_modal(), None);
        game.click_planet(386, 88);
        assert_eq!(game.planet_selection(), Some(PlanetBand::Hard));
        assert_eq!(game.screen(), Screen::Channel);
        game.menu_key(MenuKey::Back, true);
        assert_eq!(game.screen(), Screen::Planet);
        game.ingest_snapshot(&InputSnapshot::press_at(366, 523));

        // Every arm queued without a drain so far.
        assert!(game.has_pending_team_send());
        assert!(game.has_pending_music_send());
        assert!(game.has_pending_equip_send());

        let populated = game.state_hash();
        assert_eq!(
            populated, 0xFD87_E3A6_7367_3B3B,
            "outbox+lanking+shop+musicshop+planet population must not move the observable hash"
        );

        // The seam drains every queue exactly once.
        assert!(game.take_result_battle_entry().is_some());
        assert!(game.take_result_retry().is_some());
        assert!(game.take_result_back().is_some());
        assert!(game.take_room_team().is_some());
        assert!(game.take_team_ready().is_some());
        assert!(game.take_music_send().is_some());
        assert!(game.take_equip_send().is_some());
        assert!(game.take_gift_claim().is_some());
        assert!(game.take_gift_list_request().is_some());
        assert!(game.take_shop_buy().is_some());
        assert!(game.take_music_buy().is_some());
        assert!(game.take_create_room().is_some());
        assert!(game.take_join().is_some());
        assert!(!game.has_pending_team_send());
        assert!(!game.has_pending_music_send());
        assert!(!game.has_pending_equip_send());
        assert!(game.take_result_battle_entry().is_none());
        assert!(game.take_result_retry().is_none());
        assert!(game.take_result_back().is_none());
        assert!(game.take_room_team().is_none());
        assert!(game.take_team_ready().is_none());
        assert!(game.take_music_send().is_none());
        assert!(game.take_equip_send().is_none());
        assert!(game.take_gift_claim().is_none());
        assert!(game.take_gift_list_request().is_none());
        assert!(game.take_shop_buy().is_none());
        assert!(game.take_music_buy().is_none());
        assert!(game.take_create_room().is_none());
        assert!(game.take_join().is_none());
    }

    /// Stage the result chrome with exe-pinned code/rect pairs
    /// (`State_Result.bnd` r5/r6, `docs/cresult-anchors.md`): the
    /// shared driver for the sweep guards below (ADR-0003: staged
    /// rects, never literals).
    fn swept_result_game() -> ScreenGame {
        use super::super::paint::{RESULT_BACK_CODE, RESULT_RETRY_CODE};
        let mut game = ScreenGame::new();
        finish_song(&mut game);
        game.load_scene(
            Screen::Result,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: RESULT_RETRY_ID.to_owned(),
                    code: RESULT_RETRY_CODE,
                    sprite: None,
                    rect: RESULT_RETRY_RECT,
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: RESULT_BACK_ID.to_owned(),
                    code: RESULT_BACK_CODE,
                    sprite: None,
                    rect: RESULT_BACK_RECT,
                },
            ],
            HashMap::new(),
        );
        paint_result_frame(&mut game);
        assert_eq!(game.screen(), Screen::Result);
        game
    }

    #[test]
    fn result_retry_press_fires_on_down_and_queues_4010_once() {
        // Ticket #163 (result leg, CLICK press + release-inside):
        // RETRY fires on down — Waiting plus the pinned 4010
        // `0xfaa` relay body (#88) — and release-inside queues
        // nothing more: one take sends once. Result owns no
        // press-split (that evidence is planet-only, #111), so the
        // down-dispatch stands (same stand-in as the waiting leg).
        let mut game = swept_result_game();
        game.ingest_snapshot(&InputSnapshot::press_at(666, 581));
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(game.take_result_retry().is_some());
        game.ingest_snapshot(&InputSnapshot::release_at(666, 581));
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(game.take_result_retry().is_none(), "one take sends once");
    }

    #[test]
    fn result_back_release_outside_holds_rooms() {
        // Ticket #163 (result leg, CLICK press + release-outside):
        // BACK commits on down — Room plus the pinned 4021 `0xfb5`
        // relay body (#87) — so an outside release cannot revert
        // it: the scene holds Room.
        let mut game = swept_result_game();
        game.ingest_snapshot(&InputSnapshot::press_at(752, 581));
        assert_eq!(game.screen(), Screen::Room);
        assert!(game.take_result_back().is_some());
        game.ingest_snapshot(&InputSnapshot::release_at(10, 10));
        assert_eq!(game.screen(), Screen::Room);
        assert!(game.take_result_back().is_none());
    }

    #[test]
    fn result_chrome_sweep_paints_no_hover_state() {
        // Ticket #163 (result leg, HOVER stand-in solo + battle):
        // no result chrome was ever hover-captured (`docs/hover.md`
        // covers room + planet only), and the shell stages no hover
        // resolver — motion over RETRY, BACK, or the row field arms
        // nothing in a solo or a battle result: the screen holds
        // and the frame paints pixel-identical.
        // Next evidence step: the SendInput hover sweep on the live
        // result chrome (same rig as `docs/hover.md`).
        for battle in [false, true] {
            let mut game = swept_result_game();
            if battle {
                let mut room = waiting_room();
                room.game_mode = o2jam_net::GameMode::Live as u8;
                game.set_joined_room(room);
                game.set_battle_teams(battle_teams_self_wins());
                game.set_battle_self_team(Some(1));
            }
            let before = paint_result_frame(&mut game);
            for (x, y) in [
                (666, 581),
                (752, 581),
                (200, 350),
                (200, 450),
                (400, 300),
                (600, 200),
            ] {
                game.ingest_snapshot(&InputSnapshot::motion_to(x, y));
                assert_eq!(game.screen(), Screen::Result);
            }
            assert!(game.take_result_retry().is_none());
            assert!(game.take_result_back().is_none());
            let after = paint_result_frame(&mut game);
            assert_eq!(before, after, "hover paints no result chrome state");
        }
    }

    #[test]
    fn result_rank_rows_are_not_chrome() {
        // Ticket #163 (result leg, CLICK rank rows solo + battle):
        // rank rows paint live data but answer no click — the wire
        // calls landed on RETRY/BACK only (#87/#88). Clicks across
        // the row field hold Result with no relay queued, whether
        // the rows carry the solo local score or battle peer rows.
        for battle in [false, true] {
            let mut game = swept_result_game();
            if battle {
                let mut room = waiting_room();
                room.game_mode = o2jam_net::GameMode::Live as u8;
                game.set_joined_room(room);
                game.set_battle_teams(battle_teams_self_wins());
                game.set_battle_self_team(Some(1));
            }
            for (x, y) in [(200, 350), (200, 450), (400, 300), (600, 200)] {
                game.ingest_snapshot(&InputSnapshot::click_at(x, y));
                assert_eq!(game.screen(), Screen::Result);
            }
            assert!(game.take_result_retry().is_none());
            assert!(game.take_result_back().is_none());
        }
    }

    #[test]
    fn result_solo_and_battle_chrome_agree() {
        // Ticket #163 (result leg, CLICK solo vs battle): RETRY and
        // BACK answer identically with and without a battle claim —
        // RETRY hands back to Waiting plus 4010, BACK leaves for
        // rooms plus 4021. The battle verdict banner stays
        // display-only: it never joins the click dispatch.
        for battle in [false, true] {
            for retry in [true, false] {
                let mut game = swept_result_game();
                if battle {
                    let mut room = waiting_room();
                    room.game_mode = o2jam_net::GameMode::Live as u8;
                    game.set_joined_room(room);
                    game.set_battle_teams(battle_teams_self_wins());
                    game.set_battle_self_team(Some(1));
                }
                if retry {
                    game.ingest_snapshot(&InputSnapshot::click_at(666, 581));
                    assert_eq!(game.screen(), Screen::Waiting);
                    assert!(game.take_result_retry().is_some());
                    assert!(game.take_result_back().is_none());
                } else {
                    game.ingest_snapshot(&InputSnapshot::click_at(752, 581));
                    assert_eq!(game.screen(), Screen::Room);
                    assert!(game.take_result_back().is_some());
                    assert!(game.take_result_retry().is_none());
                }
            }
        }
    }

    fn paint_playing_frame(game: &mut ScreenGame, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            1000,
        );
        buf
    }

    /// True while white text ink lands near a caption origin (glyph
    /// top-left cells are often blank, so scan the scrim box).
    /// True while glyph ink lights in a 64x9 box at `origin`: the
    /// TTF rasterizes inside the cell with antialiased coverage, so
    /// probes band the ink color instead of one exact pixel.
    fn white_near(buf: &[u32], w: usize, h: usize, origin: (i32, i32)) -> bool {
        ink_near(buf, w, h, origin, (64, 9), pack_xrgb(0xFF, 0xFF, 0xFF))
    }

    /// True while ink nears `ink` inside the `probe` box at `origin`.
    fn ink_near(
        buf: &[u32],
        w: usize,
        h: usize,
        origin: (i32, i32),
        probe: (i32, i32),
        ink: u32,
    ) -> bool {
        let (ox, oy) = origin;
        let (pw, ph) = probe;
        for row in oy..oy + ph {
            for col in ox..ox + pw {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < h && c < w && near_xrgb(buf[r * w + c], ink) {
                    return true;
                }
            }
        }
        false
    }

    fn playing_avatar_game() -> ScreenGame {
        use o2jam_file::{parse_sprite55, SPRITE55_MAGIC};
        // 2x1 sprite-55: transparent black, then red (result-test shape).
        let mut blob = Vec::new();
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.extend_from_slice(&0u16.to_le_bytes());
        for field in [0u16, 0u16, 2u16, 1u16] {
            blob.extend_from_slice(&field.to_le_bytes());
        }
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&[0x00, 0x00, 0x00, 0x7C]);
        let sprite = parse_sprite55(&blob).unwrap();
        let mut game = ScreenGame::new();
        game.load_avatar(AvatarFigure::from_sprite55(&sprite));
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.set_player(43, "HOST");
        game.set_song_caption("First", 6);
        game.set_volume(5);
        game.strike_at(2, 5000);
        game
    }

    #[test]
    fn playing_paints_avatar_scene_with_captions_and_tally() {
        use crate::playing::AvatarLayout;
        let mut game = playing_avatar_game();
        let buf = paint_playing_frame(&mut game, 800, 600);
        assert_eq!(game.screen(), Screen::Playing);
        let layout = AvatarLayout::for_frame(800, 600);
        // Worn figure extent-centers in the seat: the 2x1 sprite's
        // red pixel (layer x 1) lands right of the centered box origin.
        assert_eq!(
            buf[310 * 800 + 457],
            pack_xrgb(255, 0, 0),
            "figure extent-centers in the seat"
        );
        // Life gauge shows full amber; plate, speed, song, VOL, counts,
        // EXIT ink white.
        assert_eq!(buf[215 * 800 + 400], pack_xrgb(255, 180, 16));
        for origin in [
            layout.level_origin,
            layout.speed_origin,
            layout.title_origin,
            layout.volume_origin,
            layout.counts_origins[0],
            layout.exit_origin,
        ] {
            assert!(white_near(&buf, 800, 600, origin));
        }
        // The struck note's COOL feeds the tally through the score.
        assert_eq!(game.playing.score().counts[0], 1);
    }

    #[test]
    fn playing_without_figure_paints_scene_without_avatar() {
        use crate::playing::AvatarLayout;
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        game.set_player(43, "HOST");
        let buf = paint_playing_frame(&mut game, 800, 600);
        let layout = AvatarLayout::for_frame(800, 600);
        // No figure: the seat center keeps the highway backdrop while
        // the plate still paints.
        assert_eq!(
            buf[310 * 800 + 456],
            pack_xrgb(0x0A, 0x0A, 0x0A),
            "empty seat shows the backdrop"
        );
        let (lx, ly) = layout.level_origin;
        assert!(white_near(&buf, 800, 600, (lx, ly)));
    }

    #[test]
    fn playing_syncs_song_line_from_waiting_choice() {
        use crate::playing::AvatarLayout;
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.set_selected_music(waiting_song_selection());
        walk_to_playing(&mut game);
        let buf = paint_playing_frame(&mut game, 800, 600);
        // `M757` catalog title with speed X2 (code 3): the strip lines ink
        // at the bound text rects.
        let layout = AvatarLayout::for_frame(800, 600);
        assert!(white_near(&buf, 800, 600, layout.speed_origin));
        assert!(white_near(&buf, 800, 600, layout.title_origin));
    }

    /// Paint Playing past the (5000 ms) note's bad window, rating the
    /// timeout Miss on the explicit clock.
    fn paint_playing_after_timeout_miss(game: &mut ScreenGame) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            5000 + 25 + 1,
        );
        buf
    }

    #[test]
    fn selected_chart_difficulty_drains_the_avatar_life_bar() {
        use crate::playing::AvatarLayout;
        // Chart load before selection: the choice still drives deltas.
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        game.set_selected_music(waiting_song_selection()); // difficulty 2 = Hard
        walk_to_playing(&mut game);
        let buf = paint_playing_after_timeout_miss(&mut game);
        // Hard Miss drains 30 (Easy would drain 50); the bar paints
        // the live 970/1000 amber fill (top row empty, rows below lit).
        assert_eq!(game.playing.score().life.life(), 1000 - 30);
        let bar = AvatarLayout::for_frame(800, 600).life_bar;
        assert_eq!(
            buf[bar.y as usize * 800 + bar.x as usize],
            pack_xrgb(0x30, 0x10, 0x10)
        );
        assert_eq!(
            buf[(bar.y + 1) as usize * 800 + bar.x as usize],
            pack_xrgb(255, 180, 16)
        );
    }

    #[test]
    fn selection_before_chart_load_still_sets_life_difficulty() {
        let mut game = ScreenGame::new();
        game.set_selected_music(waiting_song_selection()); // difficulty 2 = Hard
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let _ = paint_playing_after_timeout_miss(&mut game);
        assert_eq!(game.playing.score().life.life(), 1000 - 30);
    }

    #[test]
    fn unselected_charts_drain_on_the_easy_default() {
        let mut game = ScreenGame::new();
        game.load_chart(vec![(2, 5000, 7)]);
        walk_to_playing(&mut game);
        let _ = paint_playing_after_timeout_miss(&mut game);
        // No selection: the bar keeps the exe's init set (difficulty
        // byte defaults to Easy, file `0x9515c`).
        assert_eq!(game.playing.score().life.life(), 1000 - 50);
    }

    #[test]
    fn arena_stage_defaults_to_a_real_stage_before_any_room() {
        // Exe room entry replaces a zero stage with a real one (store
        // `0x4aacea`), so the composer never carries stage 0.
        let game = ScreenGame::new();
        assert_eq!(game.arena_stage(), DEFAULT_ARENA_STAGE);
        assert_ne!(game.arena_stage(), 0);
    }

    #[test]
    fn joined_room_arena_sets_the_backdrop_stage() {
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.arena = 7;
        game.set_joined_room(room);
        assert_eq!(game.arena_stage(), 7);
        let mut next = waiting_room();
        next.arena = 36;
        game.set_joined_room(next);
        assert_eq!(game.arena_stage(), 36);
    }

    #[test]
    fn zero_arena_keeps_the_current_stage() {
        // Room entry replaces zero with a real stage, it never stores
        // one, so a zero word names no stage and the default stands.
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.arena = 0;
        game.set_joined_room(room);
        assert_eq!(game.arena_stage(), DEFAULT_ARENA_STAGE);
    }

    #[test]
    fn arena_high_word_seeds_the_low_stage_word() {
        // Applier seeding (`0x4e78db`): the low word when a high word
        // is set, the full value otherwise; negative names no stage.
        assert_eq!(arena_stage_word((1 << 16) | 5), Some(5));
        assert_eq!(arena_stage_word(5), Some(5));
        assert_eq!(arena_stage_word(0), None);
        assert_eq!(arena_stage_word(-1), None);
    }

    fn loading_status_at_half() -> LoadingStatus {
        LoadingStatus {
            progress: 0.5,
            title: "First".to_owned(),
            artist: "Band".to_owned(),
        }
    }

    /// One native-frame paint through the `GameLoop` path (the
    /// fixed-clock advance rides this, never `paint_at`).
    fn paint_game_frame(game: &mut ScreenGame) {
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        game.paint(&mut frame);
    }

    #[test]
    fn fixed_clock_advances_the_splash_without_input() {
        // Trace-rig affordance: with the virtual clock enabled, the
        // splash auto-advances on painted frames alone (null audio
        // would otherwise park `now_ms` at 0 forever).
        let mut game = ScreenGame::new();
        game.enable_fixed_clock(25);
        for _ in 0..(SPLASH_AUTO_ADVANCE_MS / 25 + 2) {
            paint_game_frame(&mut game);
        }
        assert_eq!(game.screen(), Screen::Planet);
    }

    #[test]
    fn fixed_clock_runs_a_loaded_song_to_result() {
        // The same affordance drives Playing: notes auto-miss on the
        // virtual clock and the finish edge (last tail + 1.5 s) fires
        // without an audio device.
        let mut game = ScreenGame::new();
        game.enable_fixed_clock(25);
        dismiss_splash_to_dialog(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        game.load_chart(vec![(0, 1000, 1), (3, 2000, 2)]);
        stage_armed_loading_gate(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
        for _ in 0..((2000 + 1500) / 25 + 10) {
            paint_game_frame(&mut game);
        }
        assert_eq!(game.screen(), Screen::Result);
    }

    #[test]
    fn screen_name_tracks_the_flow() {
        let mut game = ScreenGame::new();
        assert_eq!(game.screen_name(), "avi");
        dismiss_splash_to_dialog(&mut game);
        assert_eq!(game.screen_name(), "channel");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen_name(), "room");
    }

    #[test]
    fn state_hash_moves_with_state_and_copies_it() {
        let mut game = ScreenGame::new();
        let boot = game.state_hash();
        dismiss_splash_to_dialog(&mut game);
        assert_ne!(game.state_hash(), boot, "screen change moves the hash");
        let in_channel = game.state_hash();
        game.set_channel_input("hello");
        assert_ne!(game.state_hash(), in_channel, "draft change moves the hash");
        game.set_channel_input("");
        assert_eq!(
            game.state_hash(),
            in_channel,
            "restored state restores the hash"
        );
        // A second game walked the same way hashes the same: the
        // trace harness compares replays through this.
        let mut twin = ScreenGame::new();
        dismiss_splash_to_dialog(&mut twin);
        assert_eq!(twin.state_hash(), in_channel);
    }

    fn walk_to_loading(game: &mut ScreenGame) {
        use MenuKey;
        dismiss_splash_to_dialog(game);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn loading_status_starts_empty() {
        assert_eq!(ScreenGame::new().loading_status(), None);
    }

    #[test]
    fn set_loading_status_remembers_the_walk() {
        let mut game = ScreenGame::new();
        game.set_loading_status(loading_status_at_half());
        assert_eq!(game.loading_status(), Some(&loading_status_at_half()));
    }

    #[test]
    fn loading_phases_walk_entry_through_arm() {
        // The staged fractions name the phases: entry, staging,
        // OJN decrypt, Vorbis decode, clock arm (gate `LOADING_GATE_PHASES`).
        let mut game = ScreenGame::new();
        assert_eq!(game.loading_gate_phase(), 0);
        for (progress, phase) in [(0.0, 0), (0.25, 1), (0.5, 2), (0.75, 3), (1.0, 4)] {
            game.set_loading_status(LoadingStatus {
                progress,
                title: "First".to_owned(),
                artist: "Band".to_owned(),
            });
            assert_eq!(game.loading_gate_phase(), phase, "progress {progress}");
        }
        // A later partial fraction never walks the gate backwards.
        game.set_loading_status(loading_status_at_half());
        assert_eq!(game.loading_gate_phase(), 4);
    }

    #[test]
    fn loading_confirm_without_a_walk_stays_in_loading() {
        use MenuKey;
        // No validation, no phases, no audio: the gate holds the
        // scene in Loading instead of inventing the handoff.
        let mut game = ScreenGame::new();
        walk_to_loading(&mut game);
        assert!(!game.loading_gate_ready());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        assert_eq!(game.song_anchor_sample, 0);
        assert_eq!(game.now_ms(), 0);
    }

    #[test]
    fn loading_confirm_without_audio_stays_in_loading() {
        use MenuKey;
        // Validated and armed, but no audio rig: missing audio
        // never arms the sample clock, so Loading holds.
        let mut game = ScreenGame::new();
        game.selected_music = Some(SetRoomMusic {
            music_id: 757,
            difficulty: 1,
            speed: 0,
        });
        walk_to_loading(&mut game);
        assert!(!game.loading_gate_ready());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn loading_gate_rejects_an_unknown_song() {
        use MenuKey;
        // A pick the staged table does not name fails validation
        // (`0x5718b0` table check): armed and audio-backed, yet the
        // gate holds.
        let mut game = ScreenGame::new();
        stage_armed_loading_gate(&mut game);
        game.selected_music = Some(SetRoomMusic {
            music_id: 9999,
            difficulty: 1,
            speed: 0,
        });
        walk_to_loading(&mut game);
        assert!(!game.validate_loading_select());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn loading_gate_rejects_an_unknown_difficulty() {
        let mut game = ScreenGame::new();
        stage_armed_loading_gate(&mut game);
        game.selected_music = Some(SetRoomMusic {
            music_id: 757,
            difficulty: 9,
            speed: 0,
        });
        assert!(!game.validate_loading_select());
        // Rx (3) maps to the Hard slice downstream, so it validates.
        game.selected_music = Some(SetRoomMusic {
            music_id: 757,
            difficulty: 3,
            speed: 0,
        });
        assert!(game.validate_loading_select());
        assert!(game.loading_gate_ready());
    }

    #[test]
    fn latched_loading_failure_holds_loading() {
        use MenuKey;
        // Bad chart / undecryptable OJN: even an armed fraction
        // cannot arm the clock past a latched failure.
        let mut game = ScreenGame::new();
        stage_armed_loading_gate(&mut game);
        game.fail_loading_gate();
        assert!(game.loading_gate_failed());
        walk_to_loading(&mut game);
        // Entering Loading clears the latch only for a fresh walk
        // seeded from a staged status: this game staged none before
        // the walk, so re-staging the armed fraction after entry
        // still holds while the latch stands.
        game.fail_loading_gate();
        stage_armed_loading_gate(&mut game);
        assert!(!game.loading_gate_ready());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        game.clear_loading();
        stage_armed_loading_gate(&mut game);
        assert!(game.loading_gate_ready());
    }

    #[test]
    fn armed_loading_handoff_anchors_the_sample_clock() {
        use o2jam_audio::PcmSamples;
        use MenuKey;
        // Gate armed on the null sink: Playing entry anchors the
        // backend sample already spent, so the song starts at 0.
        let mut game = ScreenGame::new();
        walk_to_loading(&mut game);
        stage_armed_loading_gate(&mut game);
        {
            let rig = game.audio.as_mut().expect("rig");
            rig.backend
                .play_pcm(&PcmSamples {
                    samples: vec![0i16; 44_100 * 2],
                    sample_rate: 44_100,
                    channels: 2,
                })
                .unwrap();
            assert_eq!(rig.backend.samples_played(), 44_100);
        }
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
        assert_eq!(game.now_ms(), 0);
    }

    #[test]
    fn loading_hand_session_holds_then_arms_and_hands_off() {
        // Hand-driven loading passage through the real window
        // seam: Confirm with no staged walk holds Loading (gate
        // unready, clock unanchored), then the staged walk
        // (validation, staging/decrypt/decode/arm phases, audio
        // rig) arms the gate and Confirm hands off to Playing. A
        // twin driven the same way ends on the same state hash:
        // the passage is a trace, not a one-off. Smoke: screen
        // path loading→loading→playing.
        use MenuKey;
        let mut game = ScreenGame::new();
        walk_to_loading(&mut game);
        let mut path = vec![game.screen_name().to_owned()];
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
        assert!(!game.loading_gate_ready());
        path.push(game.screen_name().to_owned());
        stage_armed_loading_gate(&mut game);
        assert!(game.loading_gate_ready());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Playing);
        path.push(game.screen_name().to_owned());
        assert_eq!(path, ["loading", "loading", "playing"]);
        let mut twin = ScreenGame::new();
        walk_to_loading(&mut twin);
        twin.menu_key(MenuKey::Confirm, true);
        stage_armed_loading_gate(&mut twin);
        twin.menu_key(MenuKey::Confirm, true);
        assert_eq!(twin.screen(), Screen::Playing);
        assert_eq!(
            twin.state_hash(),
            game.state_hash(),
            "same stimuli reach the same hashed state"
        );
    }

    #[test]
    fn clear_loading_restores_the_indeterminate_bar() {
        let mut game = ScreenGame::new();
        game.set_loading_status(loading_status_at_half());
        game.clear_loading();
        assert_eq!(game.loading_status(), None);
    }

    #[test]
    fn loading_paints_the_staged_walk_fraction() {
        // 64x48: track x16 w32 y16; half fill spans x16..32.
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        game.set_loading_status(loading_status_at_half());
        walk_to_loading(&mut game);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(buf[16 * 64 + 16], pack_xrgb(0x30, 0xB0, 0x30));
        assert_ne!(buf[16 * 64 + 40], pack_xrgb(0x30, 0xB0, 0x30));
    }

    #[test]
    fn loading_paints_the_staged_song_caption() {
        // Song row sits one placeholder row below the bar (y36).
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        game.set_loading_status(loading_status_at_half());
        walk_to_loading(&mut game);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert!(white_near(&buf, 64, 48, (16, 36)));
    }

    #[test]
    fn loading_without_a_walk_keeps_the_full_bar() {
        // No status: the legacy indeterminate full bar (x16..48).
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        walk_to_loading(&mut game);
        let mut buf = vec![0u32; 64 * 48];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            0,
        );
        assert_eq!(game.loading_status(), None);
        assert_eq!(buf[16 * 64 + 47], pack_xrgb(0x30, 0xB0, 0x30));
    }

    #[test]
    fn loading_cover_miss_keeps_the_planet_fallback() {
        // Garbage bytes stage nothing: no cover flag, and the staged
        // planet body survives for the fallback paint.
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        assert!(!game.stage_loading_cover(b"garbage"));
        assert!(!game.stage_loading_cover(&[]));
        assert!(!game.has_loading_cover());
        let art = game.scene_art(Screen::Loading).expect("loading art staged");
        assert!(art.figures.contains_key("MusicLoading.ojs"));
        assert!(!art.figures.contains_key(LOADING_COVER_SPRITE));
    }

    #[test]
    fn loading_cover_stages_the_walked_chart() {
        // End to end on a real install: song 757's walked bytes decode
        // to the 800x600 illustration, staged beside the planet body.
        // Skips when `install-root` is absent.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        assert!(game.stage_loading_cover(&bytes));
        assert!(game.has_loading_cover());
        let art = game.scene_art(Screen::Loading).expect("loading art staged");
        assert!(
            art.figures.contains_key("MusicLoading.ojs"),
            "planet fallback survives beside the cover"
        );
        let cover = art
            .figures
            .get(LOADING_COVER_SPRITE)
            .expect("cover staged under its sprite name");
        assert_eq!(cover.len(), 1);
        let layer = cover.layer(0).expect("one opaque layer");
        assert_eq!((layer.image.w, layer.image.h), (800, 600));
    }

    #[test]
    fn loading_cover_miss_clears_a_staged_cover() {
        // A later miss drops the earlier cover (stale art must never
        // hide the planet fallback). Skips when `install-root` is absent.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        assert!(game.stage_loading_cover(&bytes));
        assert!(!game.stage_loading_cover(b"garbage"));
        assert!(!game.has_loading_cover());
        let art = game.scene_art(Screen::Loading).expect("loading art staged");
        assert!(art.figures.contains_key("MusicLoading.ojs"));
        assert!(!art.figures.contains_key(LOADING_COVER_SPRITE));
    }

    #[test]
    fn selecting_the_walked_song_stages_its_loading_cover() {
        // Catalog + selection resolve reads the same walked bytes the
        // gate walks, so real loads feed the loading scene with no
        // extra call. Skips when `install-root` is absent.
        let chart = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/o2ma757.ojn");
        let Ok(_) = std::fs::read(&chart) else {
            return;
        };
        let (placements, figures) = corner_scene_art("MusicLoading.ojs");
        let mut game = ScreenGame::new();
        game.load_scene(Screen::Loading, placements, figures);
        stage_waiting_song(&mut game);
        assert!(game.has_loading_cover());
    }

    fn chat_user(sender: &str, text: &str) -> o2jam_net::rooms::WaitingUserMessage {
        o2jam_net::rooms::WaitingUserMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
            member_id: 1,
        }
    }

    fn chat_admin(sender: &str, text: &str) -> o2jam_net::rooms::WaitingAdminMessage {
        o2jam_net::rooms::WaitingAdminMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
            member_id: 7,
        }
    }

    fn channel_user(sender: &str, text: &str) -> o2jam_net::rooms::MainRoomUserMessage {
        o2jam_net::rooms::MainRoomUserMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
        }
    }

    fn channel_admin(sender: &str, text: &str) -> o2jam_net::rooms::MainRoomAdminMessage {
        o2jam_net::rooms::MainRoomAdminMessage {
            sender: sender.to_owned(),
            content: text.as_bytes().to_vec(),
        }
    }

    fn channel_whisper(sender: &str, text: &str) -> o2jam_net::rooms::ReceiveWhisper {
        o2jam_net::rooms::ReceiveWhisper {
            sender: sender.to_owned(),
            content: text.to_owned(),
        }
    }

    fn channel_announcement(text: &str) -> o2jam_net::rooms::Announcement {
        o2jam_net::rooms::Announcement {
            content: text.to_owned(),
        }
    }

    fn channel_rank(
        nickname: &str,
        music_id: i32,
        ranking: i32,
    ) -> o2jam_net::rooms::RankNotification {
        o2jam_net::rooms::RankNotification {
            invalid: false,
            music_id,
            nickname: nickname.to_owned(),
            ranking,
        }
    }

    #[test]
    fn waiting_chat_starts_empty() {
        let game = ScreenGame::new();
        assert!(game.waiting_chat().is_empty());
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.chat_send_message(), None);
    }

    #[test]
    fn pushing_a_user_message_stages_one_chat_row() {
        let mut game = ScreenGame::new();
        game.push_waiting_user_message(&chat_user("host", "hello"));
        assert_eq!(game.waiting_chat().len(), 1);
        assert_eq!(game.waiting_chat()[0].sender, "host");
        assert_eq!(game.waiting_chat()[0].text, "hello");
        assert!(!game.waiting_chat()[0].admin);
    }

    #[test]
    fn pushing_an_admin_message_marks_the_row() {
        let mut game = ScreenGame::new();
        game.push_waiting_admin_message(&chat_admin("op", "quiet"));
        assert_eq!(game.waiting_chat().len(), 1);
        assert!(game.waiting_chat()[0].admin);
    }

    #[test]
    fn joining_a_room_clears_chat_history_and_draft() {
        let mut game = ScreenGame::new();
        game.push_waiting_user_message(&chat_user("host", "hello"));
        game.set_chat_input("draft");
        game.set_joined_room(waiting_room());
        assert!(game.waiting_chat().is_empty());
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn channel_chat_starts_empty() {
        let game = ScreenGame::new();
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn pushing_a_channel_user_message_stages_one_row() {
        let mut game = ScreenGame::new();
        game.push_channel_user_message(&channel_user("host", "hello"));
        assert_eq!(game.channel_chat().len(), 1);
        assert_eq!(game.channel_chat()[0].sender, "host");
        assert_eq!(game.channel_chat()[0].text, "hello");
        assert!(!game.channel_chat()[0].admin);
        assert_eq!(game.channel_chat()[0].whisper_recipient, None);
    }

    #[test]
    fn pushing_a_channel_admin_message_marks_the_row() {
        let mut game = ScreenGame::new();
        game.push_channel_admin_message(&channel_admin("op", "quiet"));
        assert_eq!(game.channel_chat().len(), 1);
        assert!(game.channel_chat()[0].admin);
    }

    #[test]
    fn pushing_a_channel_whisper_keeps_the_recipient() {
        let mut game = ScreenGame::new();
        game.push_channel_whisper(&channel_whisper("host", "psst"), "me");
        assert_eq!(game.channel_chat().len(), 1);
        assert_eq!(game.channel_chat()[0].sender, "host");
        assert_eq!(game.channel_chat()[0].text, "psst");
        assert_eq!(
            game.channel_chat()[0].whisper_recipient.as_deref(),
            Some("me")
        );
    }

    #[test]
    fn pushing_a_channel_announcement_stages_a_bare_admin_row() {
        let mut game = ScreenGame::new();
        game.push_channel_announcement(&channel_announcement("hello"));
        assert_eq!(game.channel_chat().len(), 1);
        assert!(game.channel_chat()[0].admin);
        assert_eq!(channel_chat_row_text(&game.channel_chat()[0]), "hello");
    }

    #[test]
    fn empty_announcement_stages_no_row() {
        let mut game = ScreenGame::new();
        game.push_channel_announcement(&channel_announcement(""));
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn pushing_a_channel_rank_notification_paints_the_board_sentence() {
        let mut game = ScreenGame::new();
        game.push_channel_rank_notification(&channel_rank("host", 757, 1));
        assert_eq!(game.channel_chat().len(), 1);
        assert!(game.channel_chat()[0].admin);
        assert_eq!(
            channel_chat_row_text(&game.channel_chat()[0]),
            rank_notice_board_text("host", "M757", 1)
        );
    }

    #[test]
    fn channel_rank_notification_resolves_the_catalog_title() {
        let mut game = ScreenGame::new();
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.push_channel_rank_notification(&channel_rank("host", 757, 1));
        assert_eq!(
            channel_chat_row_text(&game.channel_chat()[0]),
            rank_notice_board_text("host", "First", 1)
        );
    }

    #[test]
    fn invalid_rank_notification_stages_no_row() {
        let mut game = ScreenGame::new();
        game.push_channel_rank_notification(&o2jam_net::rooms::RankNotification {
            invalid: true,
            music_id: 757,
            nickname: "host".to_owned(),
            ranking: 1,
        });
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn refreshing_channels_clears_notice_rows() {
        let mut game = ScreenGame::new();
        game.push_channel_announcement(&channel_announcement("hello"));
        game.push_channel_rank_notification(&channel_rank("host", 757, 1));
        game.set_channels(two_channels());
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn channel_notices_share_the_fifty_row_bound() {
        let mut game = ScreenGame::new();
        for index in 0..55 {
            game.push_channel_announcement(&channel_announcement(&format!("line{index}")));
        }
        assert_eq!(game.channel_chat().len(), 50);
        assert_eq!(game.channel_chat()[0].text, "line5");
    }

    #[test]
    fn refreshing_channels_clears_the_channel_history() {
        let mut game = ScreenGame::new();
        game.push_channel_user_message(&channel_user("host", "hello"));
        game.set_channels(two_channels());
        assert!(game.channel_chat().is_empty());
    }

    #[test]
    fn channel_history_bounds_to_fifty_rows() {
        let mut game = ScreenGame::new();
        for index in 0..55 {
            game.push_channel_user_message(&channel_user("host", &format!("line{index}")));
        }
        assert_eq!(game.channel_chat().len(), 50);
        assert_eq!(game.channel_chat()[0].text, "line5");
    }

    #[test]
    fn chat_send_builds_a_3011_body_from_the_input_line() {
        let mut game = ScreenGame::new();
        game.set_chat_input("go");
        assert_eq!(game.chat_input(), "go");
        let send = game.chat_send_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        game.clear_chat_input();
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.chat_send_message(), None);
    }

    #[test]
    fn take_chat_message_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        assert_eq!(game.take_chat_message(), None);
        game.set_chat_input("go");
        let send = game.take_chat_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.take_chat_message(), None);
    }

    #[test]
    fn channel_send_builds_a_2012_body_from_the_input_line() {
        let mut game = ScreenGame::new();
        game.set_channel_input("go");
        assert_eq!(game.channel_input(), "go");
        let send = game.channel_send_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        game.clear_channel_input();
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.channel_send_message(), None);
    }

    #[test]
    fn take_channel_message_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        assert_eq!(game.take_channel_message(), None);
        game.set_channel_input("go");
        let send = game.take_channel_message().unwrap();
        assert_eq!(send.content, b"go".to_vec());
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.take_channel_message(), None);
    }

    #[test]
    fn channel_whisper_needs_both_draft_and_recipient() {
        let mut game = ScreenGame::new();
        assert_eq!(game.channel_whisper_message("host"), None);
        game.set_channel_input("psst");
        assert_eq!(game.channel_whisper_message(""), None);
        let whisper = game.channel_whisper_message("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
    }

    #[test]
    fn take_channel_whisper_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        game.set_channel_input("psst");
        let whisper = game.take_channel_whisper("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.take_channel_whisper("host"), None);
    }

    #[test]
    fn take_channel_whisper_remembers_the_peer_for_reply() {
        let mut game = ScreenGame::new();
        game.set_channel_input("psst");
        game.take_channel_whisper("host");
        assert_eq!(game.last_whisper_peer(), Some("host"));
    }

    #[test]
    fn channel_whisper_draft_parses_name_and_text() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w host psst");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "psst".to_owned(),
            }
        );
    }

    #[test]
    fn channel_whisper_draft_collapses_extra_gaps() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w  host   hi there ");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "hi there".to_owned(),
            }
        );
    }

    #[test]
    fn waiting_draft_parses_whisper_the_same_way() {
        let mut game = ScreenGame::new();
        game.set_chat_input("/w host psst");
        assert_eq!(
            game.chat_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "psst".to_owned(),
            }
        );
    }

    #[test]
    fn whisper_draft_without_text_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w host");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/w host".to_owned())
        );
        game.set_channel_input("/w");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/w".to_owned())
        );
    }

    #[test]
    fn reply_targets_the_last_whisper_sender() {
        let mut game = ScreenGame::new();
        game.push_channel_whisper(&channel_whisper("host", "psst"), "me");
        assert_eq!(game.last_whisper_peer(), Some("host"));
        game.set_channel_input("/r hello");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "hello".to_owned(),
            }
        );
    }

    #[test]
    fn reply_without_a_peer_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        assert_eq!(game.last_whisper_peer(), None);
        game.set_channel_input("/r hello");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/r hello".to_owned())
        );
    }

    #[test]
    fn reply_without_text_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        game.push_channel_whisper(&channel_whisper("host", "psst"), "me");
        game.set_chat_input("/r");
        assert_eq!(
            game.chat_slash_command(),
            SlashCommand::Chat("/r".to_owned())
        );
    }

    #[test]
    fn help_commands_parse_without_sending() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/h");
        assert_eq!(game.channel_slash_command(), SlashCommand::Help);
        game.set_channel_input("/help");
        assert_eq!(game.channel_slash_command(), SlashCommand::Help);
        game.set_chat_input("/help");
        assert_eq!(game.chat_slash_command(), SlashCommand::Help);
    }

    #[test]
    fn unknown_slash_command_falls_back_to_plain_chat() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/dance party");
        assert_eq!(
            game.channel_slash_command(),
            SlashCommand::Chat("/dance party".to_owned())
        );
        game.set_chat_input("/dance party");
        assert_eq!(
            game.chat_slash_command(),
            SlashCommand::Chat("/dance party".to_owned())
        );
    }

    #[test]
    fn empty_draft_parses_to_empty() {
        let game = ScreenGame::new();
        assert_eq!(game.channel_slash_command(), SlashCommand::Empty);
        assert_eq!(game.chat_slash_command(), SlashCommand::Empty);
    }

    #[test]
    fn take_channel_slash_consumes_a_whisper_and_remembers_the_peer() {
        let mut game = ScreenGame::new();
        game.set_channel_input("/w host psst");
        assert_eq!(
            game.take_channel_slash(),
            SlashCommand::Whisper {
                recipient: "host".to_owned(),
                content: "psst".to_owned(),
            }
        );
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.last_whisper_peer(), Some("host"));
        assert_eq!(game.take_channel_slash(), SlashCommand::Empty);
    }

    #[test]
    fn take_chat_slash_consumes_help_without_a_peer() {
        let mut game = ScreenGame::new();
        game.set_chat_input("/help");
        assert_eq!(game.take_chat_slash(), SlashCommand::Help);
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.last_whisper_peer(), None);
    }

    #[test]
    fn chat_whisper_needs_both_draft_and_recipient() {
        let mut game = ScreenGame::new();
        assert_eq!(game.chat_whisper_message("host"), None);
        game.set_chat_input("psst");
        assert_eq!(game.chat_whisper_message(""), None);
        let whisper = game.chat_whisper_message("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
    }

    #[test]
    fn take_chat_whisper_builds_and_clears_in_one_step() {
        let mut game = ScreenGame::new();
        game.set_chat_input("psst");
        let whisper = game.take_chat_whisper("host").unwrap();
        assert_eq!(whisper.recipient, "host");
        assert_eq!(whisper.content, "psst");
        assert_eq!(game.chat_input(), "");
        assert_eq!(game.last_whisper_peer(), Some("host"));
        assert_eq!(game.take_chat_whisper("host"), None);
    }

    #[test]
    fn channel_confirm_with_a_whisper_draft_consumes_without_advancing() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.set_channel_input("/w host psst");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.last_whisper_peer(), Some("host"));
    }

    #[test]
    fn waiting_confirm_with_a_help_draft_consumes_without_starting() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.set_chat_input("/help");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn typing_appends_to_the_channel_draft_only_while_there() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.text_input("hi");
        assert_eq!(game.channel_input(), "hi");
        assert_eq!(game.chat_input(), "");
        game.take_channel_message();
        walk_to_waiting(&mut game);
        game.text_input("yo");
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.chat_input(), "yo");
    }

    #[test]
    fn channel_backspace_erases_one_character_only_while_there() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.text_input("héllo");
        game.delete_text();
        assert_eq!(game.channel_input(), "héll");
        game.clear_channel_input();
        walk_to_waiting(&mut game);
        game.delete_text();
        assert_eq!(game.channel_input(), "");
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn channel_confirm_with_a_draft_sends_and_stays_in_channel() {
        let mut game = ScreenGame::new();
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.set_channel_input("go");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Channel);
        assert_eq!(game.channel_input(), "");
    }

    #[test]
    fn refreshing_channels_clears_the_channel_draft() {
        let mut game = ScreenGame::new();
        game.set_channel_input("draft");
        game.set_channels(two_channels());
        assert_eq!(game.channel_input(), "");
    }

    #[test]
    fn waiting_paints_live_chat_rows_without_staged_art() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        let calm = paint_waiting_frame(&mut game, 80, 64);
        game.push_waiting_user_message(&chat_user("host", "hello"));
        let live = paint_waiting_frame(&mut game, 80, 64);
        assert_ne!(calm, live);
        assert!(live.contains(&pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    fn waiting_without_packets_paints_no_chat_history() {
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        assert!(game.waiting_chat().is_empty());
    }

    #[test]
    fn typing_appends_to_the_draft_only_while_waiting() {
        let mut game = ScreenGame::new();
        game.text_input("hi");
        assert_eq!(game.chat_input(), "");
        walk_to_waiting(&mut game);
        game.text_input("hi");
        assert_eq!(game.chat_input(), "hi");
    }

    #[test]
    fn typing_ignores_control_characters() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.text_input("a\u{0}b\n");
        assert_eq!(game.chat_input(), "ab");
    }

    #[test]
    fn typing_stops_at_fifty_characters() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.text_input(&"x".repeat(60));
        assert_eq!(game.chat_input().chars().count(), 50);
        game.text_input("y");
        assert_eq!(game.chat_input().chars().count(), 50);
    }

    #[test]
    fn staging_an_overlong_line_truncates_to_fifty_characters() {
        let mut game = ScreenGame::new();
        game.set_chat_input(&"x".repeat(60));
        assert_eq!(game.chat_input().chars().count(), 50);
    }

    #[test]
    fn backspace_erases_one_character_only_while_waiting() {
        let mut game = ScreenGame::new();
        game.delete_text();
        assert_eq!(game.chat_input(), "");
        walk_to_waiting(&mut game);
        game.delete_text();
        assert_eq!(game.chat_input(), "");
        game.text_input("héllo");
        game.delete_text();
        assert_eq!(game.chat_input(), "héll");
    }

    #[test]
    fn confirm_with_a_draft_sends_and_stays_in_waiting() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.text_input("go");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        assert_eq!(game.chat_input(), "");
    }

    #[test]
    fn confirm_with_an_empty_draft_advances_to_loading() {
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    /// Seat of a non-master self: the creator keeps seat 0, so any
    /// other seat id is a guest (`+0x30` derives from id 0,
    /// `docs/relay.md` §10).
    fn guest_room() -> JoinedRoom {
        let mut room = waiting_room();
        room.member_id = 5;
        room.slots[0].index = 1;
        room
    }

    #[test]
    fn waiting_non_master_hides_select_but_keeps_bpm() {
        // Only the master opens music selection (file `0xb2622`
        // drops non-master `MUSELECT` clicks): a guest self sees no
        // dialog art, no structure, and no chosen-song panel — but
        // the staged `BPM` caption stays (guests pick speed from it).
        let (placements, figures) = corner_scene_art("AlbumList.ojs");
        let mut game = ScreenGame::new();
        let mut room = guest_room();
        room.music_id = 757;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        game.load_select_music_scene(placements, figures);
        stage_waiting_song(&mut game);
        let blob = waiting_song_catalog_blob();
        let entries = o2jam_file::parse_ojn_list(&blob).unwrap();
        game.set_song_catalog(&entries);
        game.load_scene(
            Screen::Waiting,
            vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_IMAGE_SWAIT_COVERAVATAR1".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 28,
                        y: 53,
                        w: 141,
                        h: 175,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_TEXT_SWAIT_BPM".to_owned(),
                    code: 0,
                    sprite: None,
                    rect: Rect {
                        x: 620,
                        y: 60,
                        w: 60,
                        h: 20,
                    },
                },
            ],
            HashMap::new(),
        );
        assert!(!game.is_room_master());
        let buf = paint_waiting_frame(&mut game, 800, 600);
        // No dialog art at the origin, no structure scrims, no
        // chosen-song panel rows below the seats (staged art without
        // figures leaves the frame untouched, so absence reads 0) —
        // but the BPM scrim inks its rect.
        assert_eq!(buf[0], 0);
        assert_eq!(buf[95 * 800 + 30], 0);
        assert_eq!(buf[450 * 800 + 100], 0);
        assert_eq!(buf[458 * 800 + 100], 0);
        assert_eq!(buf[58 * 800 + 616], pack_xrgb(0x08, 0x08, 0x10));
    }

    #[test]
    fn waiting_non_master_confirm_stays_in_waiting() {
        // Guests never start (no `START` arm for them, file
        // `0xae805`): their Confirm readies up through the session
        // bridge instead, so the shell stays in Waiting.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        walk_to_waiting(&mut game);
        assert!(!game.is_room_master());
        assert!(game.all_peers_ready());
        assert!(!game.can_start_game());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn guest_force_advance_reaches_loading_on_broadcast() {
        // Guests never pass the Confirm gate (no `START` arm for
        // them, file `0xae805`), so the 4011 broadcast advances them
        // through the force path instead.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        walk_to_waiting(&mut game);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        assert!(game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_refuses_master_who_keeps_the_confirm_gate() {
        // Masters keep the readiness rule through Confirm: the force
        // path never advances them, even with every peer ready.
        let mut game = ScreenGame::new();
        game.set_joined_room(waiting_room());
        walk_to_waiting(&mut game);
        assert!(game.can_start_game());
        assert!(!game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Waiting);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_refuses_the_offline_walk() {
        // With no joined room the offline walk advances through
        // Confirm, never through the force path.
        let mut game = ScreenGame::new();
        walk_to_waiting(&mut game);
        assert!(!game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Waiting);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_holds_while_a_draft_is_pending() {
        // A pending draft holds the advance (peek-only): the text
        // survives, and clearing it unblocks the next broadcast.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        walk_to_waiting(&mut game);
        game.set_chat_input("hi");
        assert!(!game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Waiting);
        assert_eq!(game.chat_input(), "hi");
        game.clear_chat_input();
        assert!(game.force_advance_to_loading());
        assert_eq!(game.screen(), Screen::Loading);
    }

    #[test]
    fn force_advance_refuses_other_screens() {
        // Off Waiting there is nothing to advance: the call stays a
        // no-op instead of teleporting the flow.
        let mut game = ScreenGame::new();
        game.set_joined_room(guest_room());
        assert!(!game.force_advance_to_loading());
        assert_ne!(game.screen(), Screen::Loading);
    }

    #[test]
    fn waiting_master_start_needs_every_peer_ready() {
        // `START 0x16202012` runs the all-ready check (file
        // `0xb5560`): an unready occupied peer blocks the master,
        // and the start fires once every peer is ready.
        let mut game = ScreenGame::new();
        let mut room = waiting_room();
        room.slots[0].index = 1;
        room.slots[0]
            .member
            .as_mut()
            .expect("fixture seats a peer")
            .ready = false;
        game.set_joined_room(room);
        walk_to_waiting(&mut game);
        assert!(game.is_room_master());
        assert!(!game.all_peers_ready());
        assert!(!game.can_start_game());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
        game.waiting.as_mut().expect("room joined").slots[0]
            .member
            .as_mut()
            .expect("fixture seats a peer")
            .ready = true;
        assert!(game.can_start_game());
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Loading);
    }
    /// One room chrome control staged on its own test rect, on the
    /// walked Avi → Channel → Room path.
    fn room_lobby_with_control(code: u32, id: &str) -> ScreenGame {
        room_lobby_with_controls(&[(code, id)])
    }

    /// Room lobby with the given staged room placements.
    fn room_lobby_with_placements(placements: Vec<(u32, &str, Rect)>) -> ScreenGame {
        use MenuKey;
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.set_rooms(two_rooms());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        let placements = placements
            .into_iter()
            .map(|(code, id, rect)| ScenePlacement {
                role: PlacementRole::Single,
                id: id.to_owned(),
                code,
                sprite: None,
                rect,
            })
            .collect();
        game.load_scene(Screen::Room, placements, HashMap::new());
        game
    }

    /// Room lobby with every given control staged on the shared test
    /// rect: the point is the code dispatch, so the fixture claims no
    /// geometry.
    fn room_lobby_with_controls(controls: &[(u32, &str)]) -> ScreenGame {
        room_lobby_with_placements(
            controls
                .iter()
                .map(|(code, id)| (*code, *id, ROOM_CONTROL_TEST_RECT))
                .collect(),
        )
    }

    /// Test rect for one staged chrome control: clear of the room row
    /// bands and of every other staged control.
    const ROOM_CONTROL_TEST_RECT: Rect = Rect {
        x: 400,
        y: 380,
        w: 40,
        h: 20,
    };
    /// Click point inside [`ROOM_CONTROL_TEST_RECT`].
    const ROOM_CONTROL_TEST_CLICK: (i32, i32) = (410, 385);

    #[test]
    fn room_enter_with_an_open_dialog_stays_in_the_lobby() {
        // The open room sheet owns the keyboard: Enter must neither
        // send the lobby draft nor leave for Waiting, and the draft
        // survives for when the sheet closes.
        let mut game = ScreenGame::new();
        // Boot → Planet → Channel → Room (the documented walk).
        game.lane_key(3, true);
        game.click_planet(136, 88);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.open_create_room_dialog();
        game.set_lobby_input(LobbyChat::Room, "hello");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room, "dialog keeps the lobby");
        assert_eq!(
            game.lobby_chat(LobbyChat::Room).draft(),
            "hello",
            "the draft is untouched"
        );
        // Entering the lobby again drops the sheet (the flow's own
        // room-entry reset); with an empty draft Enter leaves for
        // Waiting as before.
        game.menu_key(MenuKey::Back, true);
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        game.set_lobby_input(LobbyChat::Room, "");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn staged_room_chrome_buttons_reach_their_side_views() {
        // Ticket #32: every room chrome button with a target already in
        // the tree answers on its staged rect and stays a lobby visit.
        for (code, id, side) in [
            (0x1320_0008, "IDC_BUTTON_SROOM_ITEMSHOP", SideScreen::Shop),
            (0x1320_0009, "IDC_BUTTON_SROOM_MYROOM", SideScreen::MyRoom),
            (
                0x1320_0037,
                "IDC_BUTTON_SROOM_MUSICSHOP",
                SideScreen::MusicShop,
            ),
            (0x1320_003A, "IDC_BUTTON_SROOM_MYBAND", SideScreen::Lanking),
        ] {
            let mut game = room_lobby_with_control(code, id);
            game.ingest_snapshot(&InputSnapshot::click_at(
                ROOM_CONTROL_TEST_CLICK.0,
                ROOM_CONTROL_TEST_CLICK.1,
            ));
            assert_eq!(game.side(), Some(side), "{id}");
            assert_eq!(game.screen(), Screen::Room, "{id}");
            assert_eq!(game.room_control_noop(), None, "{id}");
        }
    }

    #[test]
    fn staged_room_control_without_a_target_records_a_labelled_noop() {
        // Ticket #34: `FIRSTSTEP` is the closest staged control to a
        // reported tutorial button, and `STATE_ROOM` carries no
        // tutorial entry at all — the click must be recorded, never
        // silently dropped.
        let mut game = room_lobby_with_control(0x1320_0032, "IDC_BUTTON_SROOM_FIRSTSTEP");
        game.ingest_snapshot(&InputSnapshot::click_at(
            ROOM_CONTROL_TEST_CLICK.0,
            ROOM_CONTROL_TEST_CLICK.1,
        ));
        assert_eq!(game.screen(), Screen::Room);
        assert_eq!(game.side(), None);
        assert_eq!(game.room_dialog(), None);
        let noop = game.room_control_noop().expect("recorded press");
        assert_eq!(noop.id, "IDC_BUTTON_SROOM_FIRSTSTEP");
        // The press points at the tutorial entry's own OPEN item, so
        // the label names that path — never a bare "unwired".
        assert!(
            noop.next_step.contains("tutorial entry path"),
            "next step names the tutorial entry OPEN item: {}",
            noop.next_step
        );
    }

    #[test]
    fn staged_firststep_rect_matches_the_room_block_rect() {
        // Install-guarded (ticket #34): the parsed 7.55 `STATE_ROOM`
        // block pairs `IDC_BUTTON_SROOM_FIRSTSTEP` (`0x13200032`) with
        // its `.bnd` rect; the click dispatch hit-tests exactly that
        // staged rect. Skips when `install-root` is absent.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Image/Interface1.opi");
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let report = crate::staging::stage_state(
            &bytes,
            crate::staging::State::Room,
            crate::staging::StageContext::default(),
        );
        let art = report.art.expect("STATE_ROOM stages");
        let placement = art
            .placements
            .iter()
            .find(|placement| placement.code == 0x1320_0032)
            .unwrap_or_else(|| panic!("{}: FIRSTSTEP placement stages", path.display()));
        // The parsed block is the oracle (owner rule `f38f3`):
        // `GLOSSARY.md` pins FIRSTSTEP at (660,1)-(720,33); the
        // `docs/hover.md` pixel box (660,1)-(718,32) is the changed-art
        // extent, two pixels inside the rect on each axis.
        assert_eq!(
            placement.rect,
            Rect {
                x: 660,
                y: 1,
                w: 60,
                h: 32,
            },
            "FIRSTSTEP stages on its .bnd rect"
        );
    }

    #[test]
    fn click_away_from_every_room_control_stays_in_the_lobby() {
        let mut game = room_lobby_with_control(0x1320_0008, "IDC_BUTTON_SROOM_ITEMSHOP");
        game.ingest_snapshot(&InputSnapshot::click_at(10, 10));
        assert_eq!(game.screen(), Screen::Room);
        assert_eq!(game.side(), None);
        assert_eq!(game.room_control_noop(), None);
    }

    #[test]
    fn unstaged_room_controls_stay_unanswered() {
        // No staged rect names the code, so the click falls through to
        // the row hit test exactly as before (nothing is invented).
        let mut game = room_lobby_with_controls(&[]);
        game.ingest_snapshot(&InputSnapshot::click_at(
            ROOM_CONTROL_TEST_CLICK.0,
            ROOM_CONTROL_TEST_CLICK.1,
        ));
        assert_eq!(game.screen(), Screen::Room);
        assert_eq!(game.side(), None);
        assert_eq!(game.room_control_noop(), None);
    }

    #[test]
    fn room_row_bands_still_answer_beside_the_control_table() {
        let mut game = room_lobby_with_control(0x1320_0008, "IDC_BUTTON_SROOM_ITEMSHOP");
        // First row band of the legacy list: y in [150, 158).
        game.ingest_snapshot(&InputSnapshot::click_at(300, 154));
        assert_eq!(game.room_highlight, 0);
        assert_eq!(game.screen(), Screen::Waiting);
    }

    #[test]
    fn typing_on_the_room_scene_fills_the_room_chat_draft() {
        let mut game = room_lobby_with_controls(&[]);
        game.text_input("hi");
        assert_eq!(game.lobby_chat(LobbyChat::Room).draft(), "hi");
        assert_eq!(game.channel_input(), "");
    }

    #[test]
    fn room_typing_leaves_the_channel_draft_alone() {
        let mut game = room_lobby_with_controls(&[]);
        game.set_channel_input("chan");
        game.text_input("hi");
        assert_eq!(game.lobby_chat(LobbyChat::Room).draft(), "hi");
        assert_eq!(game.channel_input(), "chan");
    }

    #[test]
    fn channel_typing_leaves_the_room_draft_alone() {
        let mut game = ScreenGame::new();
        game.set_channels(two_channels());
        game.lane_key(3, true);
        game.click_planet(136, 88);
        assert_eq!(game.screen(), Screen::Channel);
        game.set_lobby_input(LobbyChat::Room, "room");
        game.text_input("hi");
        assert_eq!(game.channel_input(), "hi");
        assert_eq!(game.lobby_chat(LobbyChat::Room).draft(), "room");
    }

    #[test]
    fn room_typing_feeds_the_open_create_dialog_instead() {
        let mut game = room_lobby_with_controls(&[]);
        game.open_create_room_dialog();
        game.text_input("Jam");
        assert_eq!(game.lobby_chat(LobbyChat::Room).draft(), "");
        assert_eq!(game.create_room_form().title, "Jam");
        game.delete_text();
        assert_eq!(game.create_room_form().title, "Ja");
        assert_eq!(game.lobby_chat(LobbyChat::Room).draft(), "");
    }

    #[test]
    fn room_confirm_with_a_draft_stays_in_the_lobby_and_clears_it() {
        let mut game = room_lobby_with_controls(&[]);
        game.set_lobby_input(LobbyChat::Room, "hello");
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.screen(), Screen::Room);
        assert!(game.lobby_chat(LobbyChat::Room).draft().is_empty());
        assert_eq!(game.channel_input(), "");
    }

    #[test]
    fn room_dialog_keeps_the_leftover_draft_out_of_the_enter() {
        // The open dialog owns the keyboard, so a draft typed before it
        // opened neither sends nor holds the Enter.
        let mut game = room_lobby_with_controls(&[]);
        game.set_lobby_input(LobbyChat::Room, "hello");
        game.open_create_room_dialog();
        game.menu_key(MenuKey::Confirm, true);
        assert_eq!(game.lobby_chat(LobbyChat::Room).draft(), "hello");
    }

    #[test]
    fn room_chat_history_bounds_to_fifty_rows() {
        let mut game = ScreenGame::new();
        for index in 0..MAX_MAIN_ROOM_CHAT_ROWS.saturating_add(5) {
            game.push_lobby_user_message(
                LobbyChat::Room,
                &channel_user("host", &format!("line{index}")),
            );
        }
        assert_eq!(game.room_chat().len(), MAX_MAIN_ROOM_CHAT_ROWS);
        assert_eq!(game.room_chat()[0].text, "line5");
    }

    #[test]
    fn room_chat_rows_paint_inside_the_staged_list_rect() {
        // The `State_Room.bnd` chat rects the block pairs by code
        // ordinal: `IDC_LIST_SROOM_CHAT 0x13800014` (171,432)-(583,551)
        // and `IDC_EDIT_SROOM_CHAT 0x13300015` (167,561)-(502,580).
        let list = Rect {
            x: 171,
            y: 432,
            w: 412,
            h: 119,
        };
        let mut game = room_lobby_with_placements(vec![
            (0x1380_0014, "IDC_LIST_SROOM_CHAT", list),
            (
                0x1330_0015,
                "IDC_EDIT_SROOM_CHAT",
                Rect {
                    x: 167,
                    y: 561,
                    w: 335,
                    h: 19,
                },
            ),
        ]);
        // Differential paint: the row's ink is what the pushed line
        // changes, so unrelated lobby ink cannot mask a stray row.
        let mut empty = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut empty,
                w: 800,
                h: 600,
            },
            0,
        );
        game.push_lobby_user_message(LobbyChat::Room, &channel_user("host", "hello"));
        let mut buf = vec![0u32; 800 * 600];
        game.paint_at(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            0,
        );
        let rows: Vec<usize> = (0..buf.len())
            .filter(|index| buf[*index] != empty[*index])
            .collect();
        assert!(!rows.is_empty(), "the chat row paints");
        // The embedded TTF cell may ink one pixel above or left of
        // its origin (baseline rounding, `draw_ttf_text`), so the
        // containment check carries a 1 px margin.
        const GLYPH_OVERSHOOT: usize = 1;
        let left = usize::try_from(list.x).unwrap_or(0) - GLYPH_OVERSHOOT;
        let top = usize::try_from(list.y).unwrap_or(0) - GLYPH_OVERSHOOT;
        let (right, bottom) = (
            left + GLYPH_OVERSHOOT + usize::try_from(list.w).unwrap_or(0) + GLYPH_OVERSHOOT,
            top + GLYPH_OVERSHOOT + usize::try_from(list.h).unwrap_or(0) + GLYPH_OVERSHOOT,
        );
        for index in rows {
            let (x, y) = (index % 800, index / 800);
            assert!(
                (left..right).contains(&x) && (top..bottom).contains(&y),
                "row ink at ({x},{y}) leaves the staged list rect"
            );
        }
    }

    /// Stage one real `Dialog_Option_*` sheet (skips without
    /// install-root): the genuine 7.55 block + `.bnd` pairing through
    /// the staging declaration, like the live client stages it. The
    /// placements stay readable for dialog-relative click centers.
    fn stage_real_option_sheet(
        game: &mut ScreenGame,
        tab: PlanetOptionTab,
    ) -> Option<Vec<ScenePlacement>> {
        use crate::staging::{stage_dialog, StageContext};
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image/Interface1.opi"),
        )
        .ok()?;
        let dialog = match tab {
            PlanetOptionTab::Key => crate::staging::Dialog::PlanetOption,
            PlanetOptionTab::Sound => crate::staging::Dialog::PlanetOptionSound,
            PlanetOptionTab::Game => crate::staging::Dialog::PlanetOptionGame,
        };
        let art = stage_dialog(&bytes, dialog, StageContext::default())
            .art
            .expect("option sheet stages with real bytes");
        let placements = art.placements.clone();
        game.load_planet_option_tab(tab, art.placements, art.figures);
        Some(placements)
    }

    /// Click the center of one staged dialog code's `.bnd` rect
    /// (dialog-relative: `dialog_origin + rect`, exactly what
    /// `dialog_rect_hit` answers).
    fn click_staged_dialog_button(game: &mut ScreenGame, placements: &[ScenePlacement], code: u32) {
        let origin = super::super::paint::dialog_origin(placements);
        let placement = placements
            .iter()
            .find(|placement| placement.code == code)
            .unwrap_or_else(|| panic!("code {code:#010X} stages"));
        game.ingest_snapshot(&InputSnapshot::click_at(
            origin.0 + placement.rect.x + i32::try_from(placement.rect.w).unwrap_or(0) / 2,
            origin.1 + placement.rect.y + i32::try_from(placement.rect.h).unwrap_or(0) / 2,
        ));
    }

    #[test]
    fn options_save_click_commits_ini_at_the_staged_rect() {
        // Real 7.55 bytes (skips without install-root): the Key-tab
        // SAVE button at its staged `.bnd` rect commits `option.ini`
        // beside the install root and stays open — the #131 report
        // (dead SAVE) guarded at the exact staged geometry (#15/#16
        // dispatch chain on top of real dialog geometry).
        let mut game = ScreenGame::new();
        let Some(placements) = stage_real_option_sheet(&mut game, PlanetOptionTab::Key) else {
            return;
        };
        let root = std::env::temp_dir().join(format!("recomp2jam-opt-save-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temp install root");
        game.load_options(&root);
        open_options(&mut game);
        click_staged_dialog_button(&mut game, &placements, PlanetOptionTab::Key.save_code());
        assert_eq!(
            game.planet_modal(),
            Some(PlanetModal::Options),
            "SAVE commits and keeps the dialog open"
        );
        let ini = std::fs::read_to_string(root.join("option.ini")).expect("SAVE writes ini");
        assert!(ini.contains("[Sound]"), "committed ini:\n{ini}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn options_exit_click_closes_at_the_staged_rect() {
        // Real 7.55 bytes (skips without install-root): the staged
        // EXIT button closes the dialog without saving (#131).
        let mut game = ScreenGame::new();
        let Some(placements) = stage_real_option_sheet(&mut game, PlanetOptionTab::Key) else {
            return;
        };
        open_options(&mut game);
        click_staged_dialog_button(&mut game, &placements, PlanetOptionTab::Key.exit_code());
        assert_eq!(game.planet_modal(), None, "EXIT closes the dialog");
    }

    #[test]
    fn options_key_tab_clicks_arm_rebind_and_switch_tabs_at_staged_rects() {
        // Real 7.55 bytes (skips without install-root): the Key-tab
        // lane buttons arm their rebind at the staged rects, and the
        // staged SOUNDSET header flips the sheet — the #131 "key tab
        // cannot be changed" leg at the real geometry.
        let mut game = ScreenGame::new();
        let Some(placements) = stage_real_option_sheet(&mut game, PlanetOptionTab::Key) else {
            return;
        };
        stage_real_option_sheet(&mut game, PlanetOptionTab::Sound);
        open_options(&mut game);
        click_staged_dialog_button(&mut game, &placements, OPTION_KEY_LANE_FIRST + 2);
        assert_eq!(game.option_rebind_armed(), Some(2), "KEY3 arms lane 2");
        click_staged_dialog_button(
            &mut game,
            &placements,
            super::super::side::OPTION_KEY_SOUNDSET,
        );
        assert_eq!(game.planet_option_tab(), PlanetOptionTab::Sound);
        assert_eq!(
            game.option_rebind_armed(),
            None,
            "switching tabs disarms the pending rebind"
        );
    }
}
