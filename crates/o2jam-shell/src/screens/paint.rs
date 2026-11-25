//! Screen scenes: real `.ojs`/`.bnd` art behind each [`Screen`],
//! with the working UI ink on top.
//!
//! Each scene paints its verified archive art first (lifecycle §5–11:
//! `O2mediaLogo.ojs`, channel/room/waiting/playing/loading/result blocks)
//! through the scene kit in [`crate::avatar`] (`load_scene_figures`,
//! `bind_scene_rects`, `anchor_placements`, `paint_scene_figures`).
//! Rect ordinal `(code & 0xFFF) − 1` pairs each entry (Playing: the
//! `EXIT`..`VOLUME` run, entries 213–229, ordinals 1–17); every layer
//! paints 1:1 at its record anchor scaled into the frame — the
//! verified draw chain (the blit adds the anchor to the view
//! position; anchored bodies author their absolute position) — over
//! the view origin its placement stages: a bound rect positions
//! anchor-free bodies (lobby row icons, avatar covers), unbound rects
//! leave the frame origin. Painting stays in file order with the
//! background first.
//! Number-font bodies (`Room_Number.ojs` and kin) paint live values
//! digit by digit; the lobby row icons (`Room_State.ojs`,
//! `Room_Mode.ojs`, `Room_Om.ojs`) paint the highlighted room's
//! frames at their `SET 0x11` template rects; entry codes gate
//! visibility by state byte. The result screen pins its readouts as
//! GDI text on the native canvas and selects verdict/grade frames
//! from the staged bodies (`docs/cresult-anchors.md`). Bare
//! [`paint_screen`] keeps the flat-color fallback while no art is
//! staged; staged scenes never paint flat cards.

use std::collections::HashMap;

mod myroom;
mod options_game;
pub use self::myroom::*;

use super::super::avatar::{
    anchor_in_frame, clone_instance_live, paint_behind_placements, paint_foreground_placements,
    paint_placement_layer, paint_scene_figures, paint_scene_figures_live,
    paint_scene_figures_selected, placement_is_behind, scene_block, scene_sprite_names,
    shared_sprite_names, AvatarFigure, FigureLayer, PlacementRole, ScenePlacement, AVI_BLOCK,
    CHANNEL_BLOCK, LANKING_BLOCK, LOADING_BLOCK, LOADING_COVER_SPRITE, LOADING_SPRITE,
    MUSICSHOP_BLOCK, MYROOM_BLOCK, PLANET_BLOCK, RESULT_BLOCK, ROOM_BLOCK, SHOP_BLOCK,
    WAITING_BLOCK, WAIT_COVER_SPRITE, WAIT_TEAM_SPRITES,
};
use super::super::playing::{HighwayGeometry, PlayingView};
use super::super::screen::Screen;
use super::game::OwnedRow;
use super::side::{
    lanking_bpm_caption, lanking_diff_caption, lanking_genre_caption, lanking_lview_slots,
    lanking_rank_cells, lanking_rview_slots, lanking_tiled_rect, music_shop_buy_caption,
    music_shop_caption_rect, music_shop_cart_page_rects, music_shop_cart_slots,
    music_shop_owned_slots, music_shop_page_rects, music_shop_slots, music_shop_tiled_rect,
    planet_action_for_code, planet_band_rect, planet_caption_for_code, planet_plate_clip,
    shop_bag_page_rects, shop_cart_page_rects, shop_cart_slots, shop_cart_total_rects,
    wallet_amount_caption, wallet_row_caption, LankingSong, MusicShopFace, MusicShopNotice,
    MusicShopRow, PlanetAction, PlanetBand, PlanetOptionTab, PlanetPromo, ShopGood, ShopPanel,
    WalletRow, WalletState, GEM_LABEL, INFORMATION_MESSAGE_CODE, INFORMATION_OK_CODE,
    LANKING_LVIEW_TEXT_CODES, LANKING_MUSICNAME_CODE, LANKING_RVIEW_TEXT_CODES,
    MSHOP_BOX_FRAME_COUNT, MSHOP_BOX_FRAME_FIRST_CODE, MSHOP_CART_ITEMNAME_CODE,
    MSHOP_CART_NUM_CODE, MSHOP_CART_ROWS, MSHOP_CASHFILLUP_FALLBACK_CAPTION,
    MSHOP_CASH_FILLUP_CODE, MSHOP_CURRENT_PROGRESS_CODE, MSHOP_DIFF_CODE, MSHOP_DOWN_TEXT_CODE,
    MSHOP_ENTRY_FREE_SONG_IDS, MSHOP_GEM_PRICE_CODE, MSHOP_LIGHTBOARD_ENTRY_NOTICE,
    MSHOP_LIGHTBOARD_FALLBACK, MSHOP_LIGHTBOARD_TOOLTIP_CODE, MSHOP_MONEY_GEM_CODE,
    MSHOP_MONEY_WON_CODE, MSHOP_OWNED_FREE_OWNER, MSHOP_OWNED_SERVICE_ENDED_OWNER,
    MSHOP_OWNED_TIME_TEXT_CODE, MSHOP_OWNED_TITLE_TEXT_CODE, MSHOP_ROW_TEXT_CODES,
    MSHOP_SEARCH_EDIT_CODE, MSHOP_SEARCH_HINT, MSHOP_TAB_DELETE_CODE, MSHOP_TAB_DOWN_CODE,
    MSHOP_TOTAL_PROGRESS_CODE, MSHOP_WON_PRICE_CODE, NO_RESPONSE_MESSAGE_CP949, O2_CASH_LABEL,
    PLANET_PROMO_IMAGE_CODE, QUIT_MESSAGE_CP949, RELAY_DOWN_MESSAGE_CP949, SHOP_AVATAR_CODE,
    SHOP_BAGSORT_CODE, SHOP_CART_ITEMNAME_CODE, SHOP_CART_NUM_CODE, SHOP_CART_PRICE_CODE,
    SHOP_CART_ROWS, SHOP_GEMMONEY_CODE, SHOP_GEM_UNPRICED_WORD, SHOP_ITEMCOMMENT_CODE,
    SHOP_ITEMGEM_FIRST_CODE, SHOP_ITEMNAME_FIRST_CODE, SHOP_ITEMSHOW_FIRST_CODE,
    SHOP_ITEMWON_FIRST_CODE, SHOP_ITEM_ROWS, SHOP_MGASH_CODE, SHOP_NOTICE_TEXT_CODE,
    SHOP_O2CASH_SHORTFALL_CP949, SHOP_PLAYER_LEVEL_CAPTION_CODE, SHOP_SKILLCOUNT_CELLS,
    SHOP_SKILLCOUNT_FIRST_CODE,
};
use crate::options::OptionsSettings;
use crate::staging::ShopPriceFonts;
use o2jam_file::ControlList;
use o2jam_game::RoomTeams;
use o2jam_net::{
    rooms::om_icon_frame, ChannelState, CharacterInfo, Difficulty, GameMode, GameSpeed, JoinedRoom,
    KeyMode, MemberRole, RankEntry, RoomListEntry, RoomSlot, RoomSlotState,
};
use o2jam_render::font::{draw_text_cell, text_width_cell, Cell};
use o2jam_render::{
    blit_rgb555, blit_rgb555_scaled, draw_number, draw_text, fill_rect, number_width, pack_xrgb,
    scale_rect_to_frame, text_width, Frame, Rect, Rgb555Image, NATIVE_HEIGHT, NATIVE_WIDTH,
};

/// Gameplay block: real UI lives in `Playing1.opi` (235 entries,
/// lifecycle §10).
pub const PLAYING_BLOCK: &str = "STATE_PLAYING";
/// Gameplay layout bind file: 17 rects over the bound run (entries
/// 213–229, `EXIT`..`VOLUME`), paired by code ordinal 1–17.
pub const PLAYING_BIND_FILE: &str = "State_Playing.bnd";
/// Pre-game room layout bind file.
pub const WAITING_BIND_FILE: &str = "State_Waiting.bnd";
/// Results layout bind file (7 rects).
pub const RESULT_BIND_FILE: &str = "State_Result.bnd";

/// Staged art for one scene: `.bnd` placements plus figures loaded
/// through `load_scene_figures`. Empty while nothing is staged; every
/// scene painter falls back to flat colors then.
#[derive(Debug, Clone, Default)]
pub struct SceneArt {
    /// `.bnd` layout placements in file order.
    pub placements: Vec<ScenePlacement>,
    /// Decoded sprite bodies by archive entry name.
    pub figures: HashMap<String, AvatarFigure>,
}

/// Cash Charge view: the Item Shop board behind the modal plus the
/// staged dialog art (frame, web client, close). Kept separate so the
/// board paints first and the chrome paints over it.
#[derive(Debug, Clone, Default)]
pub struct CashChargeArt {
    /// Item Shop board mounted behind the window (b8 host board).
    pub host: SceneArt,
    /// `Dialog_ChargeCash.bnd` frame, client rect, and close control.
    pub dialog: SceneArt,
}

impl SceneArt {
    /// Whether any figure decoded (placements alone paint nothing).
    #[must_use]
    pub fn has_figures(&self) -> bool {
        !self.figures.is_empty()
    }

    /// Paint staged figures at their verbatim native origins with no
    /// flat fill: art-less states contribute zero staged pixels (an
    /// art-less block stages no controls with art and the render loop
    /// draws stored controls only), and no backbuffer fill is evidenced
    /// in the exe (no `FillRect` import or `Fill`/`Paint`/`Clear` fill
    /// vocabulary; `CWindow`/`CDialogBase` survive only as RTTI names),
    /// so an empty stage leaves the frame untouched and the live ink
    /// layers on top. Callers without any staged art keep the flat
    /// `paint_screen` card fallback.
    pub fn paint_backdrop(&self, frame: &mut Frame<'_>, _screen: Screen) {
        if !self.has_figures() {
            // Pinned no-fill rule (`docs/channel-fill.md`): the exe
            // contributes zero pixels behind art-less states and no
            // fill call is evidenced, so the shell paints nothing and
            // leaves the backbuffer to the presenter.
            return;
        }
        self.paint_overlay(frame);
    }

    /// Paint placed figures over the current frame content (gauges
    /// and panels ring the highway; see [`paint_playing_overlay`]).
    pub fn paint_overlay(&self, frame: &mut Frame<'_>) {
        paint_scene_figures(frame, &self.placements, &self.figures);
    }

    /// [`Self::paint_backdrop`] behind a per-placement filter (the
    /// `SET` clone model: scenes show only the clones of live rows
    /// and seats; see [`clone_instance_live`]).
    pub fn paint_backdrop_live(
        &self,
        frame: &mut Frame<'_>,
        _screen: Screen,
        keep: impl Fn(&ScenePlacement) -> bool,
    ) {
        if !self.has_figures() {
            return;
        }
        paint_scene_figures_live(frame, &self.placements, &self.figures, keep);
    }

    /// [`Self::paint_backdrop_live`] with one state frame selected per
    /// body (`None` keeps the normal all-layers stack). Used by scenes
    /// whose button bodies store their states as sprite-55 frames.
    pub fn paint_backdrop_live_selected(
        &self,
        frame: &mut Frame<'_>,
        _screen: Screen,
        keep: impl Fn(&ScenePlacement) -> bool,
        frame_index: impl Fn(&ScenePlacement) -> Option<usize>,
    ) {
        if !self.has_figures() {
            return;
        }
        paint_scene_figures_selected(frame, &self.placements, &self.figures, keep, frame_index);
    }
}

/// Exe-verified dialog X ("close") button rect in native px,
/// `Dialog_ChannelInfo.bnd` rect 1 `(541,151)-(589,189)` (48x38,
/// right/bottom exclusive). The bnd pairs to the `0x8C` block by
/// code ordinal `(code & 0xFFF) - 1` and its rects 2-5 equal the
/// exe-pinned row rects exactly (4/4), which verifies the pairing
/// (`docs/channel-fill.md` "METHOD CORRECTION 2026-11-10"). A
/// native click at (577,171) closes the dialog (verified live
/// 2026-11-09); it lands inside this rect. Live measurement agreed
/// within ±2 px (brightness-run tolerance, live 2026-11-09: visible
/// plate (546,155)-(583,187)).
pub(crate) const CHANNEL_DIALOG_X_RECT: Rect = Rect {
    x: 541,
    y: 151,
    w: 48,
    h: 38,
};

/// True while native pixel (`x`, `y`) sits inside the measured
/// channel-dialog X zone (see [`CHANNEL_DIALOG_X_RECT`]).
pub(crate) fn channel_dialog_x_hit(x: i32, y: i32) -> bool {
    rect_hit(&CHANNEL_DIALOG_X_RECT, x, y)
}

/// Slot of one screen in the per-scene art table.
#[must_use]
pub const fn scene_index(screen: Screen) -> usize {
    match screen {
        Screen::Avi => 0,
        Screen::Channel => 1,
        Screen::Room => 2,
        Screen::Waiting => 3,
        Screen::Loading => 4,
        Screen::Playing => 5,
        Screen::Result => 6,
        Screen::Planet => 7,
        Screen::Lanking => 8,
        Screen::Myroom => 9,
        Screen::Shop => 10,
        Screen::MusicShop => 11,
    }
}

/// Channel-list hover highlight bar: the code-mounted
/// `Dialog_ChannelSelectBar.ojs` body (one 348x44 body). Live-confirmed
/// mount: the sprite record sits at dialog+`0x8C` of the hosting dialog
/// object (mounted record h/w/pitch 44/348/696 at +0x170/+0x174/+0x178;
/// stopped-state winedbg read, live 2026-11-09). It is code-fetched
/// (named by no block), so it is never part of the `STATE_CHANNEL`
/// scene. The painter paints it at the hovered row's rect
/// ((216, row_top), 348x44 — live pixel-measured, follows the hovered
/// row; hover responds to mouse motion alone). Hover-driven, exe-
/// confirmed (live 2026-11-09).
pub const DIALOG_BAR_SPRITE: &str = "Dialog_ChannelSelectBar.ojs";

/// The channel-list dialog body (`Dialog_ChannelInfo.ojs`, one raw
/// 800x466 layer at its record anchor (0,0)). Staged at the record
/// anchor: the reference capture's measured bands — panel approx
/// x185-615 y150-464, title plate y150-195, row tops 232+50i, pager
/// strip y439-464, X zone x560-620 — all fall inside the 800x466 body
/// at (0,0) (live capture `/tmp/chan2-list.png`; method in
/// `docs/channel-fill.md`), so the body's own anchor reproduces every
/// measured band without inventing a position. Reference-measured,
/// exe-OPEN; the `STATE_CHANNEL` block still names zero sprites (the
/// dialog is code-mounted, host unpinned).
pub const CHANNEL_DIALOG_SPRITE: &str = "Dialog_ChannelInfo.ojs";

/// The join ("입장") button body (`ChannelInfo_CH_Enter.ojs`, three
/// 44x21 frames at anchor (0,0)). Staged at the bnd-derived face
/// origins: bar origin (row origin +1 inset) + bar-local (285,11),
/// so row 0 face = (502,236) — exact against the reference capture
/// (`docs/channel-fill.md` "METHOD CORRECTION 2026-11-10"); frame 0
/// paints (no hover or press state is evidenced). The staged art
/// replaces the procedural enter-button plates only while it covers
/// the same measured faces.
pub const CHANNEL_ENTER_SPRITE: &str = "ChannelInfo_CH_Enter.ojs";

/// Sprite bodies one `ControlList` block names, in file order (empty
/// for unknown blocks).
fn block_sprites(list: &ControlList, block: &str) -> Vec<String> {
    scene_block(list, block).map_or_else(Vec::new, scene_sprite_names)
}

/// Sprite bodies one block names, or the single-sprite fallback when
/// the block is missing or names nothing (splash/loading are
/// one-entry states).
fn block_or_single(list: &ControlList, block: &str, single: &str) -> Vec<String> {
    let names = block_sprites(list, block);
    if names.is_empty() {
        vec![single.to_owned()]
    } else {
        names
    }
}

/// Sprite bodies one main scene needs, in block order (lifecycle
/// §5–11). Loading resolves to its single art sprite
/// (code-mounted, not block-named); every other scene names its
/// block entries for `bind_scene_rects`, including the splash
/// (`STATE_AVI` names `O2mediaLogo.ojs` on 7.55 — no fallback:
/// without the block nothing stages)
/// (`STATE_CHANNEL` names zero sprites on 7.55 — verified byte walk
/// in `docs/channel-fill.md` — so the channel scene stages nothing:
/// sprite-less entries draw no art per recipe step 6 and the scene
/// keeps its flat background plus the live list).
#[must_use]
pub fn main_scene_sprites(list: &ControlList, screen: Screen) -> Vec<String> {
    match screen {
        Screen::Avi => block_sprites(list, AVI_BLOCK),
        Screen::Channel => block_sprites(list, CHANNEL_BLOCK),
        Screen::Room => block_sprites(list, ROOM_BLOCK),
        Screen::Waiting => block_sprites(list, WAITING_BLOCK),
        Screen::Loading => block_or_single(list, LOADING_BLOCK, LOADING_SPRITE),
        Screen::Playing => block_sprites(list, PLAYING_BLOCK),
        Screen::Result => block_sprites(list, RESULT_BLOCK),
        Screen::Planet => block_sprites(list, PLANET_BLOCK),
        Screen::Lanking => block_sprites(list, LANKING_BLOCK),
        Screen::Myroom => block_sprites(list, MYROOM_BLOCK),
        Screen::Shop => block_sprites(list, SHOP_BLOCK),
        Screen::MusicShop => block_sprites(list, MUSICSHOP_BLOCK),
    }
}

/// Placements for code-mounted sprites without a block entry
/// (loading gate): no entry means no id, code,
/// or layout to claim, so each sprite paints 1:1 at its own record
/// anchor with an empty rect — never full-bleed. Entries of bare-
/// `BOUND` blocks keep their real id and code through
/// [`anchor_placements`](super::super::avatar::anchor_placements);
/// this bare-name form is only for entry-less mounted art.
#[must_use]
pub fn unbound_scene_placements(names: &[String]) -> Vec<ScenePlacement> {
    names
        .iter()
        .map(|name| ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code: 0,
            sprite: Some(name.clone()),
            rect: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
        })
        .collect()
}

/// Title ink on lobby cards (dark on the light card).
fn title_color() -> u32 {
    pack_xrgb(0x20, 0x20, 0x20)
}

/// Scrim ink behind live UI over staged art: near-black, distinct
/// from every UI ink so text stays readable on bright OPI panels.
fn scrim_color() -> u32 {
    pack_xrgb(0x08, 0x08, 0x10)
}

/// Title ink over staged art (white on the scrim).
fn art_title_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Top-left of a centered title (shared by the card and art scenes).
fn title_origin(frame: &Frame<'_>, title: &str) -> (i32, i32) {
    let text_w = text_width(title) as i32;
    (
        frame.w as i32 / 2 - text_w / 2,
        frame.h as i32 / 2 - frame.h as i32 / 8 + 2,
    )
}

/// Scene label over an art-only lobby backdrop (dark scrim plus white
/// ink): the empty-list staging path has no live ink, so the label
/// keeps the scene readable. Genuine art never gets this stamp — the
/// 7.55 room capture (`/home/arnim/scratch/ac-room.png`) carries no
/// `ROOM` text and no scrim band, and the archive art labels itself.
fn paint_title_over_art(frame: &mut Frame<'_>, screen: Screen) {
    let title = title_of(screen);
    if title.is_empty() {
        return;
    }
    let (x, y) = title_origin(frame, title);
    let text_w = text_width(title) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 4,
            y: y - 2,
            w: (text_w + 8).max(0) as u32,
            h: 9,
        },
        scrim_color(),
    );
    draw_text(frame, x, y, title, art_title_ink());
}

/// Dark strip behind `row_count` channel/room rows so list ink
/// stays readable over staged panels (skips the frame origin where
/// staged art anchors at the native origin).
fn paint_list_scrim(frame: &mut Frame<'_>, row_count: usize) {
    if row_count == 0 || frame.w == 0 || frame.h == 0 {
        return;
    }
    let w = frame.w as i32;
    let h = frame.h as i32;
    fill_rect(
        frame,
        &Rect {
            x: w / 4 - 8,
            y: h / 4 - 2,
            w: (w / 2 + 16).max(0) as u32,
            h: (row_count as i32 * CHANNEL_ROW_H + 4).max(0) as u32,
        },
        scrim_color(),
    );
}

/// Row height of one channel entry.
const CHANNEL_ROW_H: i32 = 8;

/// Exe-pinned Channel row rects `(left, top, right, bottom)` in
/// native 800x600 px, read from the hosting `CStatePlanet` dialog
/// object at dialog+`0x94` (live Wine memory read 2026-11-09; method,
/// hashes, and session artifacts in `docs/channel-fill.md`). The four
/// rects are the dialog's visible viewport slots; scrolled rows reuse
/// them cyclically. The last visible bottom (436) sits just above the
/// measured pager strip (439) — the capture's `232+50i` text tops
/// were these rects' interior (+8 px text inset, pitch 50.3 avg).
const CHANNEL_ROW_RECTS: [(i32, i32, i32, i32); 4] = [
    (216, 224, 572, 275),
    (216, 275, 572, 324),
    (216, 324, 572, 375),
    (216, 375, 572, 436),
];

/// Viewport slot count of the pinned row table.
const CHANNEL_SLOT_COUNT: usize = CHANNEL_ROW_RECTS.len();

/// Native top of Channel viewport slot `index`. Slots past the table
/// extrapolate at the last measured pitch (51).
fn channel_slot_top(index: usize) -> i32 {
    if index < CHANNEL_SLOT_COUNT {
        return CHANNEL_ROW_RECTS[index].1;
    }
    let last = i64::from(CHANNEL_ROW_RECTS[CHANNEL_SLOT_COUNT - 1].1);
    let extra = 51_i64.saturating_mul(i64::try_from(index + 1 - CHANNEL_SLOT_COUNT).unwrap_or(0));
    i32::try_from(last + extra).unwrap_or(i32::MAX)
}

/// Row top of Channel viewport slot `index` on a frame of height `h`.
fn channel_row_top(index: usize, h: i32) -> i32 {
    let scaled = i64::from(channel_slot_top(index)) * i64::from(h) / i64::from(NATIVE_HEIGHT);
    i32::try_from(scaled).unwrap_or(i32::MAX)
}

/// Channel viewport slot under a native-space (`x`, `y`) (`None` outside
/// every pinned slot or below the shown count). Inverse of the pinned
/// [`CHANNEL_ROW_RECTS`] rects (both axes: the scroll track and the
/// dialog margins share the bands' y range); mouse clicks arrive
/// pre-mapped to native space, so no frame dims.
#[must_use]
pub(crate) fn channel_row_hit(x: i32, y: i32, row_count: usize) -> Option<usize> {
    let slots = row_count.min(CHANNEL_SLOT_COUNT);
    (0..slots).find(|&i| {
        let (left, top, right, bottom) = CHANNEL_ROW_RECTS[i];
        (left..right).contains(&x) && (top..bottom).contains(&y)
    })
}

/// Bnd-pinned row-bar geometry (`Dialog_ChannelInfo.bnd`, pairing
/// exe-verified by the exact row-rect match; `docs/channel-fill.md`
/// "METHOD CORRECTION 2026-11-10"). Live measurement agreed within ±2
/// px (brightness-run tolerance, live 2026-11-09: face art measured
/// (501,row_top+10) 44x21 vs the bnd-derived (502,row.t+12)): the `CH_Back` bar mounts at the
/// row origin +1 inset, and every bar child is bar-local. The enter
/// face is bar-local (285,11) with size 44x21, so the face origin of
/// row `i` is `(row.l+286, row.t+12)` — row 0 reads (502,236), exact
/// against the exe-pinned face inset (face Y 236 = row0 top 224 + 12,
/// live memory read 2026-11-09).
const CHANNEL_BAR_INSET: i32 = 1;
const CHANNEL_FACE_LOCAL_X: i32 = 285;
const CHANNEL_FACE_LOCAL_Y: i32 = 11;
const CHANNEL_FACE_W: i32 = 44;
const CHANNEL_FACE_H: i32 = 21;

/// Bar-local digit slots (13x18 each) and the count plate (185x19)
/// — positions bnd-pinned (see above); the sizes ride the staged
/// figures, so only the origins are constants.
const CHANNEL_DIGIT_LOCALS: [(i32, i32); 2] = [(50, 11), (63, 11)];
const CHANNEL_COUNT_LOCAL: (i32, i32) = (90, 12);

/// Bnd-pinned scroll track `(573,213)-(594,429)` (21x216). Live
/// measurement agreed within ±2 px (brightness-run tolerance, live
/// 2026-11-09: visible plate column x575-595, track y231-414). The
/// arrows are scroll-local (0,0) and (0,196) (21x20 each); the 21x45
/// bar body rides between them — its exact body offset inside the
/// arrow-bounded track is the one derived value here (mounted under
/// the up arrow at (0,20)); every other value is bnd-pinned.
const CHANNEL_SCROLL_BAR_BODY_H: i32 = 45; // 21x45 bar body between arrows
const CHANNEL_SCROLL_TRACK: Rect = Rect {
    x: 573,
    y: 213,
    w: 21,
    h: 216,
};
const CHANNEL_SCROLL_UP_LOCAL: (i32, i32) = (0, 0);
const CHANNEL_SCROLL_ARROW_H: i32 = 20;
const CHANNEL_SCROLL_DOWN_LOCAL: (i32, i32) = (0, 196);

/// Scroll-column arrow answering a click: the bnd-pinned arrows at
/// scroll-local (0,0)/(0,196) (21x20 each). The bar body between
/// them answers nothing to a click — the exe drags it (press zones
/// set drag-active `+0xc4`, motion tick `0x50e7c0`), but press+motion
/// drag has no shell handler yet (ticket #130 recommendation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelScroll {
    /// Up arrow: scroll the list toward the first row.
    Up,
    /// Down arrow: scroll the list toward the last row.
    Down,
}

/// Native rect of one scroll-column arrow: the track origin plus the
/// scroll-local arrow offset (bnd-pinned, same derivation as the
/// painted scroll art).
fn channel_scroll_arrow(local: (i32, i32)) -> Rect {
    Rect {
        x: CHANNEL_SCROLL_TRACK.x.saturating_add(local.0),
        y: CHANNEL_SCROLL_TRACK.y.saturating_add(local.1),
        w: CHANNEL_SCROLL_TRACK.w,
        h: u32::try_from(CHANNEL_SCROLL_ARROW_H).unwrap_or(0),
    }
}

/// Scroll arrow under native pixel (`x`, `y`) (`None` off both
/// arrows, including the bar body between them).
#[must_use]
pub(crate) fn channel_scroll_hit(x: i32, y: i32) -> Option<ChannelScroll> {
    if rect_hit(&channel_scroll_arrow(CHANNEL_SCROLL_UP_LOCAL), x, y) {
        return Some(ChannelScroll::Up);
    }
    if rect_hit(&channel_scroll_arrow(CHANNEL_SCROLL_DOWN_LOCAL), x, y) {
        return Some(ChannelScroll::Down);
    }
    None
}

/// True while native pixel (`x`, `y`) sits inside the enter face of
/// viewport `slot`: the bnd bar-local face (285,11) 44x21 over the
/// slot's bar origin — row 0 reads (502,236)-(546,257), tops and
/// lefts inclusive like the row rects.
#[must_use]
pub(crate) fn channel_face_hit(x: i32, y: i32, slot: usize) -> bool {
    let (face_x, face_y) = channel_enter_origin(slot);
    let right = face_x.saturating_add(CHANNEL_FACE_W);
    let bottom = face_y.saturating_add(CHANNEL_FACE_H);
    (face_x..right).contains(&x) && (face_y..bottom).contains(&y)
}

/// Bar origin (native px) of viewport slot `slot`: the pinned row
/// origin + the bnd +1 inset (row 0 bar at (217,225)).
fn channel_bar_origin(slot: usize) -> (i32, i32) {
    let row = CHANNEL_ROW_RECTS[slot.min(CHANNEL_SLOT_COUNT - 1)];
    (
        row.0.saturating_add(CHANNEL_BAR_INSET),
        row.1.saturating_add(CHANNEL_BAR_INSET),
    )
}

/// Face origin (native px) of viewport slot `slot`: the bar origin
/// plus the bnd bar-local face offset (285,11) — row 0 reads
/// (502,236), exact against both the exe-pinned face inset and the
/// reference capture.
#[must_use]
pub(crate) fn channel_enter_origin(slot: usize) -> (i32, i32) {
    let (bar_x, bar_y) = channel_bar_origin(slot);
    (
        bar_x.saturating_add(CHANNEL_FACE_LOCAL_X),
        bar_y.saturating_add(CHANNEL_FACE_LOCAL_Y),
    )
}
/// Live-measured fallback row chrome (native px, live Wine capture
/// 2026-11-09, row 0; followers re-base on the pinned row tops). The
/// wells are baked into the staged `ChannelInfo_CH_Back.ojs` bar art
/// when present — these constants drive the no-archive fallback only.
/// Measured rects: number well (225,228) 71x32 ("Ch. 0N" well; the
/// old (205,227) 94x35 pill was 20 px off); track well (298,229)
/// 195x32 (10 segments, pitch 18).
const CHANNEL_PILL_X: i32 = 225;
const CHANNEL_PILL_Y: i32 = 228;
const CHANNEL_PILL_W: i32 = 71;
const CHANNEL_PILL_H: i32 = 32;
/// Measured pill tone (near-black).
const CHANNEL_PILL_TONE: u32 = pack_xrgb(19, 20, 21);
/// Track chrome rect in native px (see above: live-measured
/// (298,229) 195x32).
const CHANNEL_TRACK_X: i32 = 298;
const CHANNEL_TRACK_Y: i32 = 229;
const CHANNEL_TRACK_W: i32 = 195;
const CHANNEL_TRACK_H: i32 = 32;
/// Measured track tones: bright top edge, mid sides and bottom.
const CHANNEL_TRACK_TOP_TONE: u32 = pack_xrgb(230, 227, 230);
const CHANNEL_TRACK_SIDE_TONE: u32 = pack_xrgb(150, 148, 150);

/// Channel row chrome for viewport slot `slot`, scaled to the frame:
/// name pill plus hollow track. Position stand-ins (see above).
fn paint_channel_row_chrome(frame: &mut Frame<'_>, slot: usize) {
    // Re-base the measured slot-0 chrome on the pinned row tops.
    let shift = channel_slot_top(slot).saturating_sub(CHANNEL_ROW_RECTS[0].1);
    let pill = scale_rect_to_frame(
        &Rect {
            x: CHANNEL_PILL_X,
            y: CHANNEL_PILL_Y.saturating_add(shift),
            w: CHANNEL_PILL_W as u32,
            h: CHANNEL_PILL_H as u32,
        },
        frame.w,
        frame.h,
    );
    fill_rect(frame, &pill, CHANNEL_PILL_TONE);
    let track = scale_rect_to_frame(
        &Rect {
            x: CHANNEL_TRACK_X,
            y: CHANNEL_TRACK_Y.saturating_add(shift),
            w: CHANNEL_TRACK_W as u32,
            h: CHANNEL_TRACK_H as u32,
        },
        frame.w,
        frame.h,
    );
    let edge = scale_rect_to_frame(
        &Rect {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        },
        frame.w,
        frame.h,
    );
    let (ex, ey) = (edge.w as i32, edge.h as i32);
    let (tx, ty, tw, th) = (track.x, track.y, track.w as i32, track.h as i32);
    fill_rect(
        frame,
        &Rect {
            x: tx,
            y: ty,
            w: track.w,
            h: ey as u32,
        },
        CHANNEL_TRACK_TOP_TONE,
    );
    fill_rect(
        frame,
        &Rect {
            x: tx,
            y: ty + th - ey,
            w: track.w,
            h: ey as u32,
        },
        CHANNEL_TRACK_SIDE_TONE,
    );
    fill_rect(
        frame,
        &Rect {
            x: tx,
            y: ty,
            w: ex as u32,
            h: track.h,
        },
        CHANNEL_TRACK_SIDE_TONE,
    );
    fill_rect(
        frame,
        &Rect {
            x: tx + tw - ex,
            y: ty,
            w: ex as u32,
            h: track.h,
        },
        CHANNEL_TRACK_SIDE_TONE,
    );
}
const CHANNEL_ENTER_TONE: u32 = pack_xrgb(150, 148, 151);

/// Orange CLOSE ink on inactive rows, sampled off the exe
/// closed rows (`~/scratch/t207/closed-compare.png`: the median of
/// 1079 glyph pixels is (247,144,40)).
const CHANNEL_CLOSE_TONE: u32 = pack_xrgb(0xF7, 0x90, 0x28);

/// Dark well behind the CLOSE marking: the exe closed-row face zone
/// reads near-black (most-common bucket (16,16,16)) where the staged
/// bar frame-0 well is blank gray.
const CHANNEL_CLOSE_WELL_TONE: u32 = pack_xrgb(0x10, 0x10, 0x10);

/// Whether a row shows the baked FULL bar: a live row at (or past)
/// capacity shows the `CH_Back` frame-2 FULL face instead of the
/// enter button. Zero capacity never reads full — the quiet-server
/// all-zero rows paint an empty track, not FULL. Which capacity the
/// exe compares (slot capacity vs channel cap) rides the wire row;
/// the frame-1 bar state stays OPEN (no capture pins its trigger).
fn channel_row_full(channel: &ChannelState) -> bool {
    channel.active && channel.capacity > 0 && channel.population >= channel.capacity
}

/// Native width of the count strip art (`CH_ConnectCount` 185x19).
const CHANNEL_COUNT_W: i64 = 185;

/// Lit cells in the count strip art (10 segments across 185 native px).
const CHANNEL_COUNT_SEGMENTS: i64 = 10;

/// Divide a nonnegative value by a positive divisor, rounding up.
fn ceil_div(numerator: i64, divisor: i64) -> i64 {
    let quotient = numerator / divisor;
    if numerator % divisor == 0 {
        quotient
    } else {
        quotient + 1
    }
}

/// Whole segments lit by the live population. The 1003 row's population
/// and capacity are exe-pinned (`docs/net-protocol.md`); a live proxy pin
/// rewrote only that population word and measured 1, 10, 9 and 50 visitors
/// against capacity 100 as 1, 1, 1 and 5 lit segments. That rules out a
/// pixel floor and a floor-plus-one rule: the exe rounds the segment share
/// UP. Zero population or zero capacity lights nothing; an over-capacity
/// row saturates at the full strip.
fn channel_fill_segments(channel: &ChannelState) -> i64 {
    if channel.capacity <= 0 || channel.population <= 0 {
        return 0;
    }
    let share = i64::from(channel.population) * CHANNEL_COUNT_SEGMENTS;
    ceil_div(share, i64::from(channel.capacity)).clamp(0, CHANNEL_COUNT_SEGMENTS)
}

/// Filled width of the count strip in native px: whole lit segments
/// rounded up to whole pixels. One segment is 19 px, five are 93 px and
/// ten fill all 185 px.
fn channel_fill_width(channel: &ChannelState) -> usize {
    let filled = ceil_div(
        channel_fill_segments(channel) * CHANNEL_COUNT_W,
        CHANNEL_COUNT_SEGMENTS,
    );
    usize::try_from(filled).unwrap_or(0)
}

/// Channel number the digit slots show: the wire `channel_id` **plus
/// one**, pinned by the 2026-11-10 rig capture (`docs/channel-fill.md`,
/// "Channel row digits — EXE-PINNED to `channel_id + 1`"): a proxy
/// rewrote the 1003 ids under the exe, and the wells followed the id
/// (runs `+5`, `+10` read `Ch.06 07 08 09` / `Ch.11 12 13 14`), which
/// refutes the list-ordinal reading. Slot order stays the wire list
/// order. The tens slot shows `(id+1)/10 % 10`, the ones slot
/// `(id+1) % 10`; ids past 98 wrap to their low two digits.
///
/// The value is presentation only: the entry face still sends the raw
/// `channel_id` (capture run 1: the row painted `Ch.10` entered
/// channel `09`).
fn channel_digit(channel: &ChannelState, tens: bool) -> usize {
    let shown = u32::from(channel.channel_id).saturating_add(1);
    let digit = if tens { shown / 10 % 10 } else { shown % 10 };
    usize::try_from(digit).unwrap_or(0)
}

/// Left `width` px of one layer image (native px): the count strip
/// clips to the population share instead of painting whole.
fn crop_layer_width(layer: &FigureLayer, width: usize) -> Rgb555Image {
    let take = width.min(layer.image.w);
    let mut words = Vec::with_capacity(take * layer.image.h);
    for row in 0..layer.image.h {
        let start = row * layer.image.w;
        words.extend_from_slice(&layer.image.words[start..start + take]);
    }
    Rgb555Image {
        w: take,
        h: layer.image.h,
        words,
    }
}

/// Paint one staged layer at a bar-local origin (native px): the
/// digit/plate paint path — one frame per slot, never the stacked
/// whole-figure paint (the loader stacks every frame as layers, so
/// a whole-figure paint would smear all ten digits into each well).
fn paint_bar_layer(
    frame: &mut Frame<'_>,
    layer: &FigureLayer,
    bar_origin: (i32, i32),
    local: (i32, i32),
) {
    let (dx, dy) = anchor_in_frame(
        bar_origin.0 + local.0,
        bar_origin.1 + local.1,
        frame.w,
        frame.h,
    );
    blit_rgb555(
        frame,
        &layer.image.view(),
        dx + layer.dx,
        dy + layer.dy,
        Some(layer.colorkey),
    );
}

/// Staged row-bar art for viewport slot `slot`: the `CH_Back` body at
/// the bnd bar origin (row origin +1) plus the digit plates and the
/// count plate at their bnd bar-local slots. Returns `true` while the
/// bar body staged — it then replaces the procedural pill/track
/// chrome (same row-interior role); each plate paints only while its
/// own figure staged (absent parts fall back to no plate, never a
/// stand-in). The bar shows its status frame (FULL rows show the
/// baked frame-2 face, every other row frame 0), the digit slots
/// show the live channel number's tens/ones frames, and the count
/// strip clips to the live population share.
fn paint_channel_bar_art(
    frame: &mut Frame<'_>,
    slot: usize,
    channel: &ChannelState,
    art: &SceneArt,
) -> bool {
    let Some(bar) = art
        .figures
        .get(CHANNEL_BAR_SPRITE)
        .filter(|figure| !figure.is_empty())
    else {
        return false;
    };
    if frame.w == 0 || frame.h == 0 {
        return false;
    }
    // Status frame first: FULL rows show the baked frame-2 face
    // (which covers the enter well), every other row frame 0.
    let full = channel_row_full(channel);
    let frame_index = if full { 2 } else { 0 };
    let Some(body) = bar.layer(frame_index).or(bar.layer(0)) else {
        return false;
    };
    let bar_origin = channel_bar_origin(slot);
    let (dx, dy) = anchor_in_frame(bar_origin.0, bar_origin.1, frame.w, frame.h);
    blit_rgb555(
        frame,
        &body.image.view(),
        dx + body.dx,
        dy + body.dy,
        Some(body.colorkey),
    );
    // Inactive rows paint the baked empty slot: no digits, no count
    // strip, and (see `paint_channel_face`) no enter face.
    if channel.active {
        if let Some(digits) = art
            .figures
            .get(CHANNEL_DIGIT_SPRITE)
            .filter(|figure| !figure.is_empty())
        {
            for (tens, local) in [
                (true, CHANNEL_DIGIT_LOCALS[0]),
                (false, CHANNEL_DIGIT_LOCALS[1]),
            ] {
                if let Some(shown) = digits.layer(channel_digit(channel, tens)) {
                    paint_bar_layer(frame, shown, bar_origin, local);
                }
            }
        }
        if let Some(count) = art
            .figures
            .get(CHANNEL_COUNT_SPRITE)
            .filter(|figure| !figure.is_empty())
        {
            if let Some(strip) = count.layer(0) {
                let width = channel_fill_width(channel);
                if width > 0 {
                    let cropped = crop_layer_width(strip, width);
                    let (dx, dy) = anchor_in_frame(
                        bar_origin.0 + CHANNEL_COUNT_LOCAL.0,
                        bar_origin.1 + CHANNEL_COUNT_LOCAL.1,
                        frame.w,
                        frame.h,
                    );
                    blit_rgb555(
                        frame,
                        &cropped.view(),
                        dx + strip.dx,
                        dy + strip.dy,
                        Some(strip.colorkey),
                    );
                }
            }
        }
    }
    true
}

/// Staged scroll bodies riding the bnd track (573,213): the up/down
/// arrows at scroll-local (0,0)/(0,196) with the bar body between
/// them. Each part paints only while its own figure staged.
fn paint_channel_scroll_art(frame: &mut Frame<'_>, art: &SceneArt, top: usize, len: usize) {
    // Thumb paint for the channel scrollbar. The exe recomputes the
    // thumb at `0x50ee50` as `round(pos * scale) + origin` and writes
    // the bar rect back through the pairing table (`+0xc8` scale
    // source is UNPINNED, see ticket #174 / #130).
    // OPEN: +0xc8 scale equation unpinned — here using the linear
    // fraction `pos / max_top` over origin travel 233..=(409 - 45)
    // (45 px bar body) with thumb-height compensation.
    if frame.w == 0 || frame.h == 0 {
        return;
    }
    let track = &CHANNEL_SCROLL_TRACK;
    let max_top = len.saturating_sub(CHANNEL_VISIBLE_ROWS);
    let frac = if max_top == 0 {
        0.0
    } else {
        (top.min(max_top) as f32 / max_top as f32).clamp(0.0, 1.0)
    };
    // OPEN: +0xc8 scale equation unpinned — linear fraction over
    // origin travel 233..=(409 - body height) (45 px bar body).
    let track_top = 233i32;
    let track_bot = 409i32;
    let body_h = CHANNEL_SCROLL_BAR_BODY_H;
    let origin_bot = track_bot - body_h;
    let thumb_y = track_top + (frac * (origin_bot - track_top) as f32).round() as i32;
    let paint_part = |frame: &mut Frame<'_>, figure_name: &str, local: (i32, i32)| {
        let Some(figure) = art
            .figures
            .get(figure_name)
            .filter(|figure| !figure.is_empty())
        else {
            return;
        };
        let (dx, dy) = anchor_in_frame(track.x + local.0, track.y + local.1, frame.w, frame.h);
        figure.paint_scaled_anchors(frame, dx, dy, frame.w, frame.h);
    };
    // The dialog sheet bakes both arrow plates at the bnd track ends
    // (white rule outside each triangle); the messenger sprites use the
    // opposite rule order. Only the movable thumb is a separate body.
    if !channel_dialog_sheet_staged(Some(art)) {
        paint_part(frame, CHANNEL_SCROLL_UP_SPRITE, CHANNEL_SCROLL_UP_LOCAL);
        paint_part(frame, CHANNEL_SCROLL_DOWN_SPRITE, CHANNEL_SCROLL_DOWN_LOCAL);
    }
    paint_part(frame, CHANNEL_SCROLL_BAR_SPRITE, (0, thumb_y - track.y));
}

/// Entry-button plate for one viewport slot, scaled to the frame:
/// the bnd-derived face origin plus the bnd face size (44x21).
fn channel_enter_rect(slot: usize, w: i32, h: i32) -> Rect {
    let wide = i64::from(w);
    let high = i64::from(h);
    let native_w = i64::from(NATIVE_WIDTH);
    let native_h = i64::from(NATIVE_HEIGHT);
    let (face_x, face_y) = channel_enter_origin(slot);
    Rect {
        x: i32::try_from(i64::from(face_x) * wide / native_w).unwrap_or(0),
        y: i32::try_from(i64::from(face_y) * high / native_h).unwrap_or(0),
        w: u32::try_from((i64::from(CHANNEL_FACE_W) * wide / native_w).max(0)).unwrap_or(0),
        h: u32::try_from((i64::from(CHANNEL_FACE_H) * high / native_h).max(0)).unwrap_or(0),
    }
}

/// Entry-button plates at the measured faces: face-tone boxes as
/// position stand-ins. The exe button is staged art with baked glyphs
/// (mount unpinned; the micro-font has no CP949 glyphs), so plates
/// mark placement, never art. Only the viewport slice paints.
fn paint_channel_buttons(frame: &mut Frame<'_>, shown: usize) {
    if shown == 0 || frame.w == 0 || frame.h == 0 {
        return;
    }
    let (w, h) = (
        i32::try_from(frame.w).unwrap_or(0),
        i32::try_from(frame.h).unwrap_or(0),
    );
    for slot in 0..shown {
        paint_channel_button(frame, slot, w, h);
    }
}

/// One entry-button plate at its measured face (see
/// [`paint_channel_buttons`]): the per-row fallback when no staged
/// enter art covers the face. Faces paint after the bar — the bar
/// frame's face well is opaque art, not a cutout — so this rides the
/// same per-row slot as the staged face.
fn paint_channel_button(frame: &mut Frame<'_>, slot: usize, w: i32, h: i32) {
    fill_rect(frame, &channel_enter_rect(slot, w, h), CHANNEL_ENTER_TONE);
}

/// CLOSE marking on an inactive row: a dark well over the staged
/// face rect with orange `CLOSE` centered in it, per the exe
/// closed-row capture (`docs/captures/t207/t207-findings.md`
/// closed-row verdict). No staged CLOSE figure exists — the pinned
/// 7.55 dialog block names no `CH_Close` sprite, `CH_Back` frames are
/// normal/etched-입장/FULL, and `CH_Enter` frames are 입장-only
/// (`docs/channel-fill.md` "Ticket #12 row-art decode") — so the
/// marking is procedural. The well bevel stays OPEN.
fn paint_channel_close_marking(frame: &mut Frame<'_>, slot: usize, w: i32, h: i32) {
    const CLOSE_WORD: &str = "CLOSE";
    const CLOSE_GLYPH_H: i32 = 12;
    let rect = channel_enter_rect(slot, w, h);
    fill_rect(frame, &rect, CHANNEL_CLOSE_WELL_TONE);
    let word_w = i32::try_from(text_width(CLOSE_WORD)).unwrap_or(0);
    let x = rect.x + (rect.w as i32 - word_w).max(0) / 2;
    let y = rect.y + (rect.h as i32 - CLOSE_GLYPH_H).max(0) / 2;
    draw_text(frame, x, y, CLOSE_WORD, CHANNEL_CLOSE_TONE);
}

/// Per-row enter face after the bar: the CLOSE marking on inactive
/// rows (no staged CLOSE figure exists, so it paints procedurally —
/// see [`paint_channel_close_marking`]), staged [`CHANNEL_ENTER_SPRITE`]
/// frame 0 for joinable rows, nothing for FULL rows (the baked FULL
/// bar covers the well), and the procedural plate where no enter art
/// staged. Faces ride above the bar because the bar frame's face
/// well is opaque art (decoded `CH_Back` frame 0 carries a gray
/// well, not a cutout).
fn paint_channel_face(frame: &mut Frame<'_>, slot: usize, channel: &ChannelState, art: &SceneArt) {
    let w = i32::try_from(frame.w).unwrap_or(0);
    let h = i32::try_from(frame.h).unwrap_or(0);
    if !channel.active {
        paint_channel_close_marking(frame, slot, w, h);
        return;
    }
    let Some(figure) = art
        .figures
        .get(CHANNEL_ENTER_SPRITE)
        .filter(|figure| !figure.is_empty())
    else {
        paint_channel_button(frame, slot, w, h);
        return;
    };
    if channel_row_full(channel) {
        return;
    }
    let Some(face) = figure.layer(0) else {
        return;
    };
    // Face origin (native px): the bnd-derived bar origin + the
    // bnd bar-local face offset (285,11).
    let (face_x, face_y) = channel_enter_origin(slot);
    let (dx, dy) = anchor_in_frame(face_x, face_y, frame.w, frame.h);
    blit_rgb555(
        frame,
        &face.image.view(),
        dx + face.dx,
        dy + face.dy,
        Some(face.colorkey),
    );
}

/// Row bar body (`ChannelInfo_CH_Back.ojs`, 347x41, 3 frames):
/// mounted at the row origin +1 inset per `Dialog_ChannelInfo.bnd`
/// rect 2+i (row 0 bar at (217,225); pairing exe-verified by the
/// exact row-rect match, `docs/channel-fill.md` "METHOD CORRECTION
/// 2026-11-10"). Replaces the procedural pill/track chrome while
/// staged (same row-interior role).
pub const CHANNEL_BAR_SPRITE: &str = "ChannelInfo_CH_Back.ojs";

/// Channel number digit plate (`ChannelInfo_CH_Num.ojs`, 13x18),
/// two slots per row bar at bar-local (50,11) and (63,11) (bnd-pinned).
pub const CHANNEL_DIGIT_SPRITE: &str = "ChannelInfo_CH_Num.ojs";

/// Population/capacity count plate (`ChannelInfo_CH_ConnectCount.ojs`,
/// 185x19) at bar-local (90,12) (bnd-pinned).
pub const CHANNEL_COUNT_SPRITE: &str = "ChannelInfo_CH_ConnectCount.ojs";

/// Scroll bar body (`ChannelInfo_Scroll_Bar.ojs`, 21x45) riding the
/// bnd scroll track (bnd-pinned).
pub const CHANNEL_SCROLL_BAR_SPRITE: &str = "ChannelInfo_Scroll_Bar.ojs";

/// Scroll up arrow (`Messenger_Main_Scroll_Up.ojs`, 21x20) at
/// scroll-local (0,0) (bnd-pinned).
pub const CHANNEL_SCROLL_UP_SPRITE: &str = "Messenger_Main_Scroll_Up.ojs";

/// Scroll down arrow (`Messenger_Main_Scroll_Down.ojs`, 21x20) at
/// scroll-local (0,196) (bnd-pinned).
pub const CHANNEL_SCROLL_DOWN_SPRITE: &str = "Messenger_Main_Scroll_Down.ojs";

/// Whether the staged dialog sheet already paints the dark list field.
/// Its baked rows and scroll track cover the fallback scrim; repainting
/// the scrim would hide the sheet's own chrome.
fn channel_dialog_sheet_staged(art: Option<&SceneArt>) -> bool {
    art.is_some_and(|art| {
        art.figures
            .get(CHANNEL_DIALOG_SPRITE)
            .is_some_and(|sheet| !sheet.is_empty())
    })
}

/// Enter-button faces for the visible rows: staged
/// [`CHANNEL_ENTER_SPRITE`] frame 0 at the bnd-derived face origins
/// (row origin +1 inset + bar-local (285,11); row 0 = (502,236),
/// exact against the reference measurement) while the row stays
/// joinable; FULL rows paint no face (the baked FULL bar covers the
/// well). The staged art covers the same measured rect the plates
/// marked, so the replacement invents nothing. One frame only: the
/// loader stacks all three frames as layers, and a whole-figure
/// paint would smear them. Faces paint per row after the bar (see
/// [`paint_channel_face`): the bar frame's face well is opaque art,
/// so a pre-row face layer would vanish under it.
/// Dark strip behind Channel rows over the pinned viewport slots
/// (2 px bleed past the first top and last bottom).
fn paint_channel_scrim(frame: &mut Frame<'_>, shown: usize) {
    if shown == 0 || frame.w == 0 || frame.h == 0 {
        return;
    }
    let w = i32::try_from(frame.w).unwrap_or(0);
    let h = i32::try_from(frame.h).unwrap_or(0);
    let first = channel_row_top(0, h) - 2;
    let last_index = (shown - 1).min(CHANNEL_SLOT_COUNT - 1);
    let bottom = i32::try_from(
        i64::from(CHANNEL_ROW_RECTS[last_index].3) * i64::from(h) / i64::from(NATIVE_HEIGHT),
    )
    .unwrap_or(i32::MAX)
        + 2;
    fill_rect(
        frame,
        &Rect {
            x: w / 4 - 8,
            y: first,
            w: (w / 2 + 16).max(0) as u32,
            h: (bottom - first).max(0) as u32,
        },
        scrim_color(),
    );
}

/// Reference-measured visible Channel rows: the live dialog shows
/// four rows with the rest reachable by scroll (`/tmp/chan2-list.png`).
/// Reference-grade, exe-OPEN; scroll interaction itself stays OPEN.
pub(crate) const CHANNEL_VISIBLE_ROWS: usize = 4;

/// Visible Channel slice for a stored list top: clamped origin plus
/// shown count. The scroll arrows drive this origin directly — the
/// exe scrollbar owns its position (`this+0xb8`, clamped
/// `[0, max-page]` in `0x50ec20`, file `0x10ec20`) and forwards each
/// delta to the list top (`CDChannelInfo+0x184`, handlers `0x4749a0`/
/// `0x4749f0`, files `0x749a0`/`0x749f0`) — while the keyboard
/// highlight never enters it. Observed with GNU objdump 2.47 slices
/// plus Python 3.14.10 byte scans over the read-only 7.55 copy.
#[must_use]
pub(crate) fn channel_viewport_from_top(top: usize, len: usize) -> (usize, usize) {
    let max_top = len.saturating_sub(CHANNEL_VISIBLE_ROWS);
    let top = top.min(max_top);
    (top, len.saturating_sub(top).min(CHANNEL_VISIBLE_ROWS))
}

/// Item shop: staged art behind an empty catalogue, flat card
/// fallback. Catalogue rows need item data joins (OPEN) — no rows,
/// prices, or selection state.
pub fn paint_shop_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Shop);
}

/// Song shop: staged art behind an empty catalogue, flat card
/// fallback. Catalogue rows need the song index join (OPEN) — no
/// rows, prices, or selection state.
pub fn paint_music_shop_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::MusicShop);
}

/// Rankings: staged art behind an empty list-view, flat card
/// fallback. Rank rows need the fetch opcode (OPEN) — no data, no
/// sorts, no selection state.
pub fn paint_lanking_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Lanking);
}

/// Cash Charge chrome over the staged host board: the dialog's
/// staged `.bnd` placements and their sprite anchors place the frame
/// and the X close; no staged dialog art paints nothing (ADR-0003).
pub fn paint_cash_charge_frame(frame: &mut Frame<'_>, dialog: &SceneArt) {
    dialog.paint_overlay(frame);
}

/// Lanking board overlay over the staged side art: left-list song
/// rows (TITLE/ARTIST/EDITOR/GENRE/BPM/DIFF) tiled over the staged
/// `LVIEW_EX` slots, right-list fetched rank rows
/// (RANK/NICKNAME/BATTLES/WINS/WINRATE/ARROW — the viewer's own
/// line on the first `RVIEW_EX` slot, the board page on the rest)
/// tiled over the staged `RVIEW_EX` slots, and the picked-song
/// caption — each inked into its staged TEXT template rect shifted
/// by the staged slot origin (no staged slots or templates:
/// paints nothing, art-only). Geometry always rides staged rects,
/// never literals; text inks white through the system face (owner
/// decision `e8d01` — rasterization is presentation).
pub fn paint_lanking_board(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    songs: &[LankingSong],
    self_entry: Option<&RankEntry>,
    ranks: &[RankEntry],
    selected_title: Option<&str>,
) {
    paint_lanking_songs(frame, placements, songs);
    paint_lanking_ranks(frame, placements, self_entry, ranks);
    if let Some(title) = selected_title {
        paint_lanking_caption(frame, placements, title);
    }
}

/// Native rect of one staged TEXT template (`None` when unstaged).
fn lanking_template_rect(placements: &[ScenePlacement], code: u32) -> Option<Rect> {
    placements
        .iter()
        .find(|placed| placed.code == code)
        .map(|placed| placed.rect)
}

/// Left-list song rows into the six staged `LVIEW` TEXT templates,
/// one row per staged `LVIEW_EX` slot (unstaged slots or templates:
/// no rows).
fn paint_lanking_songs(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    songs: &[LankingSong],
) {
    let slots = lanking_lview_slots(placements);
    let Some(base) = slots.first() else {
        return;
    };
    let templates: Vec<Rect> = LANKING_LVIEW_TEXT_CODES
        .iter()
        .filter_map(|code| lanking_template_rect(placements, *code))
        .collect();
    if templates.len() != LANKING_LVIEW_TEXT_CODES.len() {
        return;
    }
    for (slot, song) in slots.iter().zip(songs.iter()) {
        let cells = [
            song.title.clone(),
            song.artist.clone(),
            song.editor.clone(),
            lanking_genre_caption(song.genre),
            lanking_bpm_caption(song.bpm),
            lanking_diff_caption(song.levels),
        ];
        for (template, text) in templates.iter().zip(cells.iter()) {
            paint_text_into_rect(
                frame,
                lanking_tiled_rect(*template, base, slot),
                text,
                CP949_INK,
            );
        }
    }
}

/// Right-list fetched rank rows into the six staged `RVIEW` TEXT
/// templates: the viewer's own line on the first staged `RVIEW_EX`
/// slot, the board page on the rest (blank fill rows ink nothing;
/// unstaged slots or templates: no rows).
fn paint_lanking_ranks(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    self_entry: Option<&RankEntry>,
    ranks: &[RankEntry],
) {
    let slots = lanking_rview_slots(placements);
    let Some(base) = slots.first() else {
        return;
    };
    let templates: Vec<Rect> = LANKING_RVIEW_TEXT_CODES
        .iter()
        .filter_map(|code| lanking_template_rect(placements, *code))
        .collect();
    if templates.len() != LANKING_RVIEW_TEXT_CODES.len() {
        return;
    }
    // Slot zero is the viewer's own line (ASSUMED read — see
    // `lanking_rview_slots`); a blank self line inks nothing.
    if let Some(entry) = self_entry {
        paint_lanking_rank_row(frame, &templates, base, base, entry);
    }
    for (slot, entry) in slots.iter().skip(1).zip(ranks.iter()) {
        paint_lanking_rank_row(frame, &templates, base, slot, entry);
    }
}

/// One rank line into the six staged `RVIEW` templates at its slot
/// (blank fill rows ink nothing).
fn paint_lanking_rank_row(
    frame: &mut Frame<'_>,
    templates: &[Rect],
    base: &Rect,
    slot: &Rect,
    entry: &RankEntry,
) {
    for (template, text) in templates.iter().zip(lanking_rank_cells(entry).iter()) {
        if text.is_empty() {
            continue;
        }
        paint_text_into_rect(
            frame,
            lanking_tiled_rect(*template, base, slot),
            text,
            CP949_INK,
        );
    }
}

/// Picked-song title into the staged `MUSICNAME` rect.
fn paint_lanking_caption(frame: &mut Frame<'_>, placements: &[ScenePlacement], title: &str) {
    if let Some(rect) = lanking_template_rect(placements, LANKING_MUSICNAME_CODE) {
        paint_text_into_rect(frame, rect, title, CP949_INK);
    }
}

/// Staged board data behind one Item Shop overlay pass. Bundled so
/// the painter takes one parameter object instead of six arguments.
pub struct ShopBoard<'a> {
    /// Lower-left panel face (entry MY BAG, row-pick SHOPPING BAG).
    pub panel: ShopPanel,
    /// True on the live state-3 entry: goods ink the four featured
    /// card cells, not the eight raw `ITEMNAME` rows.
    pub featured_cards: bool,
    /// Visible catalog goods for the item rows.
    pub goods: &'a [ShopGood],
    /// Shared wallet behind the two currency cells.
    pub wallet: &'a WalletState,
    /// Checked cart goods behind the cart rows.
    pub cart: &'a [ShopGood],
    /// Current cart page (zero-based).
    pub cart_page: usize,
    /// Cart page count (at least one).
    pub cart_pages: usize,
}

/// Whether the item-sort plate paints. `FUN_004a1010` writes the
/// control's `+0x24` enable word from `cart_count == 0`: the code push
/// at `0x4a106d`, `SETZ` at `0x4a1088`, and store through
/// `FUN_00409bf0` (REA 7.55 `ev_7baf8f3c…`, `ev_694dfc15…`). A
/// checked cart hides the control; its `.bnd` rect stays as geometry.
#[must_use]
pub const fn shop_bagsort_kept(code: u32, cart_count: usize) -> bool {
    code != SHOP_BAGSORT_CODE || cart_count == 0
}

/// Item-shop board overlay over the staged side art: the visible
/// goods' names and cash prices into the staged `ITEMNAME`/`ITEMWON`
/// TEXT rects (the exe's observed gem zero inks the staged child
/// cell until a per-item word is pinned), the wallet gem and cash
/// cells (`GEMMONEY`, `MGASH`), and — on the SHOPPING BAG face the
/// row pick opens — the checked cart rows with checkout totals and
/// pager readout. Every cell uses a staged rect, never a literal.
/// Text renders through the system face (owner decision `e8d01`).
pub fn paint_shop_board(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    board: ShopBoard<'_>,
) {
    if board.featured_cards {
        let fonts = crate::staging::staged_shop_price_fonts();
        paint_shop_featured_captions(frame, placements, board.goods, fonts.as_deref());
    } else {
        paint_shop_rows(frame, placements, board.goods);
    }
    paint_shop_wallet(frame, placements, board.wallet);
    // The entry face is MY BAG (audit D4); cart ink only rides the
    // face the row pick opens. Both faces share the stage geometry.
    if board.panel == ShopPanel::ShoppingBag {
        paint_shop_cart(
            frame,
            placements,
            board.cart,
            board.cart_page,
            board.cart_pages,
        );
    }
}

/// Left-anchored `TEXT` cell with the same GDI bearing as the
/// centered variant. The set-item card's title/price cells ink from
/// the cell's left edge in the 7.55 entry frame.
fn paint_text_left_in_cell(
    frame: &mut Frame<'_>,
    native_rect: Rect,
    text: &str,
    ink: [u8; 4],
    ink_origin_y: i32,
) {
    if frame.w == 0 || frame.h == 0 || native_rect.w == 0 || native_rect.h == 0 || text.is_empty() {
        return;
    }
    let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let scale = cp949_fit_scale(text, rect.w as f32, rect.h as f32);
    if scale <= 0.0 {
        return;
    }
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        0,
        ink_origin_y,
        text,
        ink,
        scale,
    );
    composite_text_image(frame, &rect, &image);
}

/// GDI ink bearing for the featured card's `TEXT` cells. The 7.55
/// TEXT control centers ink in its staged rect (`docs/gdi-text.md`,
/// `DrawTextA` with `DT_CENTER`); one rasterizer step lands our cap
/// ink on the exe's cell ink top.
const SHOP_CARD_INK_ORIGIN_Y: i32 = 3;

/// Card attribute cells: the exe formats two percentages from the
/// selected item row (`FUN_00529230`, file `0x529230`). Its format
/// strings live at VA `0x64c6cc` (`경험치`) and `0x64c6b4` (`잼머니`);
/// a zero word blanks that cell (REA `ev_ee6fcc7…fb`). The header is
/// the card's own caption, pinned from the v3 exe entry frame.
fn shop_card_attribute_lines(
    experience_percent: u32,
    jam_money_percent: u32,
) -> [Option<String>; 3] {
    let bonus_line =
        |label: &str, percent: u32| (percent != 0).then(|| format!("- {label} : {percent}% 증가"));
    [
        Some("고스트속성:".to_owned()),
        bonus_line("경험치", experience_percent),
        bonus_line("잼머니", jam_money_percent),
    ]
}

/// Card price-row geometry and ink, pinned to the 7.55 card-price
/// writer (REA `ev_8c9dfd4d…7011`, `ev_1c918ec2…ada7b`,
/// `ev_085d45d0…b50b8e6e`; `docs/shop.md`). The exe inks this row from
/// `Shop_Font_ItemWon.ojs`/`Shop_Font_ItemGem.ojs` sprite frames: digits
/// `0..9`, won prefix `11`, gold `G` prefix `10` from the won atlas.
/// The modern-font ink below stays only as the no-archive fallback
/// (owner decision `e8d01`); with staged art the raster is the exe's.
const SHOP_SET_PRICE_CODE: u32 = 0x1440_0094;
const SHOP_PRICE_CELL_SHIFT_X: i32 = 0x28;
const SHOP_WON_PRICE_ANCHOR_X: i32 = 5;
const SHOP_GEM_PRICE_ANCHOR_X: i32 = 0x46;
const SHOP_PRICE_DIGIT_PITCH: i32 = 8;
const SHOP_PRICE_GLYPH_HEIGHT: u32 = 12;
const SHOP_WON_GLYPH_WIDTH: u32 = 13;
const SHOP_GEM_GLYPH_WIDTH: u32 = 9;
const SHOP_WON_GLYPH_BACKOFF: i32 = 7;
const SHOP_GEM_GLYPH_BACKOFF: i32 = 4;
const SHOP_WON_PREFIX_FRAME: usize = 11;
const SHOP_GEM_PREFIX_FRAME: usize = 10;
const SHOP_PRICE_DIGIT_INK: [u8; 4] = [0x62, 0x62, 0x62, 0xFF];
const SHOP_WON_GLYPH_INK: [u8; 4] = [0x6A, 0xC5, 0x31, 0xFF];
const SHOP_GEM_GLYPH_INK: [u8; 4] = [0xFF, 0xCD, 0x00, 0xFF];

/// Digit count of a decimal price word. Counting characters, not
/// bytes, keeps the name true; saturate on an unexpectedly long word
/// instead of wrapping.
fn shop_price_word_digit_count(word: &str) -> i32 {
    i32::try_from(word.chars().count()).unwrap_or(i32::MAX)
}

/// Digit count and leading place value of one staged price word
/// (a zero word has one digit and place value one).
fn shop_price_places(price: u32) -> (i32, u32) {
    let mut divisor = 1;
    let mut count = 1;
    while price / divisor >= 10 {
        divisor = divisor.saturating_mul(10);
        count += 1;
    }
    (count, divisor)
}

/// Left edge of a currency prefix: the exe backs off one digit pitch
/// per price digit, then the glyph's own inset from the anchor.
fn shop_price_prefix_left(anchor_x: i32, digit_count: i32, glyph_backoff: i32) -> i32 {
    anchor_x
        .saturating_sub(SHOP_PRICE_DIGIT_PITCH.saturating_mul(digit_count))
        .saturating_sub(glyph_backoff)
}

/// One sprite frame from a staged runtime font, pasted at its exe
/// anchor: the font bodies carry `(0,0)` anchors, so `left`/`top`
/// land the glyph directly on the cloned `SET_PRICE` cell.
fn paint_shop_price_sprite_glyph(frame: &mut Frame<'_>, left: i32, top: i32, layer: &FigureLayer) {
    let (x, y) = anchor_in_frame(left, top, frame.w, frame.h);
    blit_rgb555(
        frame,
        &layer.image.view(),
        x.saturating_add(layer.dx),
        y.saturating_add(layer.dy),
        Some(layer.colorkey),
    );
}

/// Decimal digits at the exe's right-anchored 8-px pitch: the most
/// significant digit starts `pitch × (count − 1)` left of the anchor,
/// and every digit selects its own font frame.
fn paint_shop_price_sprite_digits<'a>(
    frame: &mut Frame<'_>,
    anchor_x: i32,
    top: i32,
    price: u32,
    glyph: impl Fn(u8) -> Option<&'a FigureLayer>,
) {
    let (count, mut divisor) = shop_price_places(price);
    let mut left = anchor_x.saturating_sub(SHOP_PRICE_DIGIT_PITCH.saturating_mul(count - 1));
    while divisor > 0 {
        let digit = u8::try_from((price / divisor) % 10).unwrap_or(0);
        if let Some(layer) = glyph(digit) {
            paint_shop_price_sprite_glyph(frame, left, top, layer);
        }
        left = left.saturating_add(SHOP_PRICE_DIGIT_PITCH);
        divisor /= 10;
    }
}

/// Featured-card price from the staged sprite atlases. The gem
/// prefix comes from the won atlas frame `10`; gem digits come from
/// the gem atlas. An unpriced gem reads `0`, the pinned entry word.
fn paint_shop_price_sprite_row(
    frame: &mut Frame<'_>,
    top: i32,
    won_anchor: i32,
    gem_anchor: i32,
    good: &ShopGood,
    fonts: &ShopPriceFonts,
) {
    let (won_count, _) = shop_price_places(good.price);
    let won_glyph_left = shop_price_prefix_left(won_anchor, won_count, SHOP_WON_GLYPH_BACKOFF);
    if let Some(prefix) = fonts.won_frame(SHOP_WON_PREFIX_FRAME) {
        paint_shop_price_sprite_glyph(frame, won_glyph_left, top, prefix);
    }
    paint_shop_price_sprite_digits(frame, won_anchor, top, good.price, |digit| {
        fonts.won_frame(usize::from(digit))
    });

    let gem_price = good.gem_price.unwrap_or(0);
    let (gem_count, _) = shop_price_places(gem_price);
    let gem_glyph_left = shop_price_prefix_left(gem_anchor, gem_count, SHOP_GEM_GLYPH_BACKOFF);
    if let Some(prefix) = fonts.won_frame(SHOP_GEM_PREFIX_FRAME) {
        paint_shop_price_sprite_glyph(frame, gem_glyph_left, top, prefix);
    }
    paint_shop_price_sprite_digits(frame, gem_anchor, top, gem_price, |digit| {
        fonts.gem_frame(usize::from(digit))
    });
}

/// No-archive fallback: the same exe origins, pitch, and colors in
/// modern-font text (owner decision `e8d01`). Kept for clients with
/// no game data; the sprite row replaces it whenever art staged.
fn paint_shop_price_text_row(
    frame: &mut Frame<'_>,
    top: i32,
    won_anchor: i32,
    gem_anchor: i32,
    good: &ShopGood,
) {
    let won = good.price.to_string();
    let won_count = shop_price_word_digit_count(&won);
    let won_glyph_left = shop_price_prefix_left(won_anchor, won_count, SHOP_WON_GLYPH_BACKOFF);
    paint_text_left_in_cell(
        frame,
        Rect {
            x: won_glyph_left,
            y: top,
            w: SHOP_WON_GLYPH_WIDTH,
            h: SHOP_PRICE_GLYPH_HEIGHT,
        },
        "₩",
        SHOP_WON_GLYPH_INK,
        0,
    );
    paint_shop_price_text_digits(frame, won_anchor, top, &won);

    let gem = good.gem_price.map_or_else(
        || SHOP_GEM_UNPRICED_WORD.to_owned(),
        |price| price.to_string(),
    );
    let gem_count = shop_price_word_digit_count(&gem);
    let gem_glyph_left = shop_price_prefix_left(gem_anchor, gem_count, SHOP_GEM_GLYPH_BACKOFF);
    paint_text_left_in_cell(
        frame,
        Rect {
            x: gem_glyph_left,
            y: top,
            w: SHOP_GEM_GLYPH_WIDTH,
            h: SHOP_PRICE_GLYPH_HEIGHT,
        },
        "G",
        SHOP_GEM_GLYPH_INK,
        0,
    );
    paint_shop_price_text_digits(frame, gem_anchor, top, &gem);
}

/// Text fallback digits in one rect, right-anchored on the exe pitch.
fn paint_shop_price_text_digits(frame: &mut Frame<'_>, anchor_x: i32, top: i32, word: &str) {
    let count = shop_price_word_digit_count(word);
    let span = SHOP_PRICE_DIGIT_PITCH.saturating_mul(count);
    let left =
        anchor_x.saturating_sub(SHOP_PRICE_DIGIT_PITCH.saturating_mul(count.saturating_sub(1)));
    paint_text_left_in_cell(
        frame,
        Rect {
            x: left,
            y: top,
            w: u32::try_from(span).unwrap_or(u32::MAX),
            h: SHOP_PRICE_GLYPH_HEIGHT,
        },
        word,
        SHOP_PRICE_DIGIT_INK,
        0,
    );
}

/// Card price row from the cloned `SET_PRICE` cell. The exe shifts
/// that cell by `0x28`, anchors won digits at `+5` and gem digits at
/// `+0x46`, then backs each glyph off one pitch per digit.
fn paint_shop_card_price_row(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    parent: &str,
    good: &ShopGood,
    fonts: Option<&ShopPriceFonts>,
) {
    let Some(cell) = placements
        .iter()
        .find(|placed| placed.code == SHOP_SET_PRICE_CODE && placed.id == parent)
        .map(|placed| placed.rect)
    else {
        return;
    };
    let origin_x = cell.x.saturating_add(SHOP_PRICE_CELL_SHIFT_X);
    let won_anchor = origin_x.saturating_add(SHOP_WON_PRICE_ANCHOR_X);
    let gem_anchor = origin_x.saturating_add(SHOP_GEM_PRICE_ANCHOR_X);
    if let Some(fonts) = fonts {
        paint_shop_price_sprite_row(frame, cell.y, won_anchor, gem_anchor, good, fonts);
    } else {
        paint_shop_price_text_row(frame, cell.y, won_anchor, gem_anchor, good);
    }
}

/// Featured card captions: the state-3 entry's four goods ink their
/// own staged `SET_TITLE` cell and the cloned `SET_PRICE` row. Each
/// child is a clone under one `SETITEM_EX` parent; text stays left
/// anchored in that cloned rect.
fn paint_shop_featured_captions(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    goods: &[ShopGood],
    fonts: Option<&ShopPriceFonts>,
) {
    for (slot, good) in goods.iter().enumerate().take(4) {
        let Some(parent) = super::side::SHOP_SET_CARD_PARENT_IDS.get(slot) else {
            return;
        };
        // Each card inks the bonuses of its own selected good into the
        // cloned `SET_TYPE1..3` cells; a zero bonus leaves its cell dark.
        let attribute_lines =
            shop_card_attribute_lines(good.experience_bonus_percent, good.jam_money_bonus_percent);
        for (code, line) in attribute_lines.iter().enumerate() {
            let Some(line) = line else {
                continue;
            };
            let code = 0x1440_008F + u32::try_from(code).unwrap_or(0);
            let Some(rect) = placements
                .iter()
                .find(|placed| placed.code == code && placed.id == *parent)
                .map(|placed| placed.rect)
            else {
                continue;
            };
            paint_text_left_in_cell(frame, rect, line, CP949_INK, SHOP_CARD_INK_ORIGIN_Y);
        }
        if let Some(rect) = placements
            .iter()
            .find(|placed| placed.code == 0x1440_008E && placed.id == *parent)
            .map(|placed| placed.rect)
        {
            paint_text_left_in_cell(frame, rect, &good.name, CP949_INK, SHOP_CARD_INK_ORIGIN_Y);
        }
        paint_shop_card_price_row(frame, placements, parent, good, fonts);
    }
}

/// Visible goods into the staged `ITEMNAME`/`ITEMWON` templates
/// (`None` per missing template: paints nothing).
fn paint_shop_rows(frame: &mut Frame<'_>, placements: &[ScenePlacement], goods: &[ShopGood]) {
    for (slot, good) in goods.iter().enumerate().take(SHOP_ITEM_ROWS) {
        let slot_u32 = u32::try_from(slot).unwrap_or(u32::MAX);
        if let Some(rect) = shop_template_rect(placements, SHOP_ITEMNAME_FIRST_CODE + slot_u32) {
            paint_text_into_rect(frame, rect, &good.name, CP949_INK);
        }
        if let Some(rect) = shop_template_rect(placements, SHOP_ITEMWON_FIRST_CODE + slot_u32) {
            paint_text_into_rect(frame, rect, &good.price.to_string(), CP949_INK);
        }
        if let Some(rect) = shop_template_rect(placements, SHOP_ITEMGEM_FIRST_CODE + slot_u32) {
            let word = good.gem_price.map_or_else(
                || SHOP_GEM_UNPRICED_WORD.to_owned(),
                |price| price.to_string(),
            );
            paint_text_into_rect(frame, rect, &word, CP949_INK);
        }
    }
}

/// Wallet gem cell (`GEMMONEY`) plus the cash cell (`MGASH`):
/// `--` until their wire words fold (gem via 5028/5001/5003, cash
/// via the 5001 prefix; the `MGASH`→`o2_cash` join stays OPEN).
fn paint_shop_wallet(frame: &mut Frame<'_>, placements: &[ScenePlacement], wallet: &WalletState) {
    if let Some(rect) = shop_template_rect(placements, SHOP_GEMMONEY_CODE) {
        let row = WalletRow::named(GEM_LABEL, wallet.gem);
        paint_text_into_rect(frame, rect, &wallet_row_caption(&row), CP949_INK);
    }
    if let Some(rect) = shop_template_rect(placements, SHOP_MGASH_CODE) {
        let row = WalletRow::named(O2_CASH_LABEL, wallet.o2_cash);
        paint_text_into_rect(frame, rect, &wallet_row_caption(&row), CP949_INK);
    }
}

/// Checked cart rows into the staged row-local `CART` templates
/// riding the staged `CART_EX` slot origins (position, name, price;
/// unstaged slots or templates: no rows), plus the cart pager
/// readout (`cur/tot` into the staged digit-pair unions).
fn paint_shop_cart(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    cart: &[ShopGood],
    cart_page: usize,
    cart_pages: usize,
) {
    // Totals ride their own TEXT rects, never the row templates:
    // they ink even when no cart row stages.
    paint_shop_cart_totals(frame, placements, cart);
    let slots = shop_cart_slots(placements);
    let num = shop_template_rect(placements, SHOP_CART_NUM_CODE);
    let name = shop_template_rect(placements, SHOP_CART_ITEMNAME_CODE);
    let price = shop_template_rect(placements, SHOP_CART_PRICE_CODE);
    let (Some(num), Some(name), Some(price)) = (num, name, price) else {
        return;
    };
    if slots.is_empty() {
        return;
    }
    for (index, (slot, good)) in slots.iter().zip(cart.iter()).enumerate() {
        paint_text_into_rect(
            frame,
            music_shop_tiled_rect(num, slot),
            &shop_cart_position(cart_page, index),
            CP949_INK,
        );
        paint_text_into_rect(
            frame,
            music_shop_tiled_rect(name, slot),
            &good.name,
            CP949_INK,
        );
        paint_text_into_rect(
            frame,
            music_shop_tiled_rect(price, slot),
            &good.price.to_string(),
            CP949_INK,
        );
    }
    if let Some((cur, tot)) = shop_cart_page_rects(placements) {
        paint_text_into_rect(frame, cur, &(cart_page + 1).to_string(), CP949_INK);
        paint_text_into_rect(frame, tot, &cart_pages.to_string(), CP949_INK);
    }
}

/// Previewed good into the staged `ITEMCOMMENT` TEXT rect (the
/// tooltip panel pairing is ASSUMED — `ITEMCOMMENT2` mirrors it;
/// next step: rig capture of a preview press). Paints nothing
/// without a staged comment rect or preview pick.
pub fn paint_shop_preview(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    preview: Option<&ShopGood>,
) {
    let Some(good) = preview else {
        return;
    };
    if let Some(rect) = shop_template_rect(placements, SHOP_ITEMCOMMENT_CODE) {
        paint_text_into_rect(frame, rect, &good.name, CP949_INK);
    }
}

/// Owned-panel pager readout: current page into the left digit
/// union, total pages into the right one (L = current / R = total
/// is ASSUMED; next step: rig capture of a bag-page turn). Paints
/// nothing when any readout digit is unstaged.
pub fn paint_shop_bag_pager(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    bag_page: usize,
    bag_pages: usize,
) {
    if let Some((cur, tot)) = shop_bag_page_rects(placements) {
        paint_text_into_rect(frame, cur, &(bag_page + 1).to_string(), CP949_INK);
        paint_text_into_rect(frame, tot, &bag_pages.to_string(), CP949_INK);
    }
}

/// NOTICE caption into the staged top-strip TEXT cell (spec #178 D5).
/// The board art itself stages with the bound layout; its 7.55 writer
/// string is unread, so only an explicitly staged caption inks here.
pub fn paint_shop_notice_text(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    notice_text: Option<&str>,
) {
    if let (Some(text), Some(rect)) = (
        notice_text,
        shop_template_rect(placements, SHOP_NOTICE_TEXT_CODE),
    ) {
        paint_text_into_rect(frame, rect, text, CP949_INK);
    }
}

/// Player identity and SKILLCOUNT cells staged by `FUN_004a0660`
/// (`ev_3e020a4ada701e31…`). The level/nickname caption comes from the
/// live character record; the ten counts are explicitly staged because
/// the exe's value writer stays OPEN. Unstaged cells paint nothing.
pub fn paint_shop_identity_captions(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    identity: Option<&str>,
    skill_counts: &[Option<u32>; SHOP_SKILLCOUNT_CELLS],
) {
    if let (Some(identity), Some(rect)) = (
        identity,
        shop_template_rect(placements, SHOP_PLAYER_LEVEL_CAPTION_CODE),
    ) {
        paint_text_into_rect(frame, rect, identity, CP949_INK);
    }
    for (slot, count) in skill_counts.iter().enumerate() {
        let Some(count) = count else {
            continue;
        };
        let Ok(slot) = u32::try_from(slot) else {
            continue;
        };
        let Some(rect) = shop_template_rect(placements, SHOP_SKILLCOUNT_FIRST_CODE + slot) else {
            continue;
        };
        paint_text_into_rect(frame, rect, &count.to_string(), CP949_INK);
    }
}

/// Cart checkout totals: the summed `ITEMWON` prices into
/// `CHECKMONEY`, `--` into `CHECKGEM` (no gem-price source is
/// pinned — same OPEN as the `ITEMGEM` row cells).
///
/// `FUN_004a1010` (`ev_ec83763a…`) enables these cells only when the
/// cart-count word `+0x2334` is non-zero, so an empty cart leaves both
/// staged cells dark. The exe's own total-ink writer stays OPEN.
fn paint_shop_cart_totals(frame: &mut Frame<'_>, placements: &[ScenePlacement], cart: &[ShopGood]) {
    if cart.is_empty() {
        return;
    }
    let Some((gem_cell, money_cell)) = shop_cart_total_rects(placements) else {
        return;
    };
    paint_text_into_rect(frame, gem_cell, super::side::UNKNOWN_AMOUNT, CP949_INK);
    let total = cart
        .iter()
        .map(|good| good.price)
        .fold(0u32, |total, price| total.saturating_add(price));
    paint_text_into_rect(frame, money_cell, &total.to_string(), CP949_INK);
}

/// Item-shop avatar pane over the staged side art: the equipped (or
/// preview) worn figure at the staged `IDC_IMAGE_SSHOP_AVATAR` rect
/// on the paint clock (ticket #196, same breathing path as the room
/// slot). `None` paints no figure; without the staged rect the pane
/// stays art-only.
pub fn paint_shop_avatar(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figure: Option<&AvatarFigure>,
    now_ms: u64,
) {
    if let (Some(figure), Some(slot)) = (figure, shop_template_rect(placements, SHOP_AVATAR_CODE)) {
        let slot = scale_rect_to_frame(&slot, frame.w, frame.h);
        figure.paint_visible_centered(frame, &slot, frame.w, frame.h, now_ms);
    }
}

/// Per-row item portraits over the staged shop art: slot 0 of each
/// visible catalog good's `Itemdata_Korea.dat` row, painted at the staged
/// `ITEMSHOW1..8` rects (#363). Geometry rides the `.bnd` rects; a
/// good without a decoded portrait, or a row without a staged rect,
/// simply paints nothing in that cell.
pub fn paint_shop_item_icons(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    goods: &[ShopGood],
    portraits: &std::collections::HashMap<u32, AvatarFigure>,
) {
    for (slot, good) in goods.iter().enumerate().take(SHOP_ITEM_ROWS) {
        let Some(portrait) = portraits.get(&good.item_id) else {
            continue;
        };
        let code = SHOP_ITEMSHOW_FIRST_CODE + u32::try_from(slot).unwrap_or(u32::MAX);
        let Some(rect) = shop_template_rect(placements, code) else {
            continue;
        };
        let rect = scale_rect_to_frame(&rect, frame.w, frame.h);
        portrait.paint_visible_centered(frame, &rect, frame.w, frame.h, 0);
    }
}

/// Native blit offsets from each `SETITEM_EX` parent origin to the
/// pose frame's image top-left. Pinned against the v6 7.55 entry
/// frame: the four slot-2 bodies draw 1:1. Their last-ink rows are
/// 233/233/233/234 and exclusive bottoms 234/234/234/235; image
/// origins are (212,66), (380,66), (522,87), (669,94) — not stretched
/// into the card's `SET_AVATAR` cell. Geometry still
/// starts from staged parent rects, never literals in the painter
/// (owner pin f38f3).
const SHOP_FEATURED_FIGURE_BLIT_OFFSETS: [(i32, i32); 4] = [(-1, 4), (26, 4), (27, 25), (33, 32)];

/// Four featured card figures: the exe paints slot-2 full-body sprites
/// at a fixed pose frame, native size, under their own archive color
/// key. Geometry is the staged `SETITEM_EX` parent origin plus the
/// pinned blit offset; a good without a decoded body paints nothing.
pub fn paint_shop_featured_icons(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    goods: &[ShopGood],
    figures: &std::collections::HashMap<u32, AvatarFigure>,
) {
    for (slot, good) in goods.iter().enumerate().take(4) {
        let Some((offset_x, offset_y)) = SHOP_FEATURED_FIGURE_BLIT_OFFSETS.get(slot) else {
            return;
        };
        let Some(figure) = figures.get(&good.item_id) else {
            continue;
        };
        let Some(layer) = figure.layer(0) else {
            continue;
        };
        let parent_code = 0x1401_D088 + u32::try_from(slot).unwrap_or(u32::MAX);
        let Some(parent) = placements
            .iter()
            .find(|placed| placed.code == parent_code)
            .map(|placed| placed.rect)
        else {
            continue;
        };
        // The decoded layer carries the archive sprite's placement
        // anchor; subtract it so `paint` lands the image at the
        // measured top-left, with the sprite's own color key.
        figure.paint(
            frame,
            parent.x + offset_x - layer.dx,
            parent.y + offset_y - layer.dy,
        );
    }
}

/// Cart-row position caption (`NUM` cell): the 1-based row order
/// across cart pages, never the good id.
fn shop_cart_position(cart_page: usize, index: usize) -> String {
    (cart_page
        .saturating_mul(SHOP_CART_ROWS)
        .saturating_add(index)
        .saturating_add(1))
    .to_string()
}

/// Native rect of one staged shop template (`None` unstaged).
fn shop_template_rect(placements: &[ScenePlacement], code: u32) -> Option<Rect> {
    placements
        .iter()
        .find(|placed| placed.code == code)
        .map(|placed| placed.rect)
}

/// Client-only chrome for the last picked catalog row: a stripe
/// inside that row's staged `MS_EX` rect. The exe row-press frame is
/// still OPEN (`E-T`), so the tone is ours-by-design; the geometry
/// and the pick→cart state are not. A missing row or unstaged slot
/// paints nothing (ADR-0003).
const MUSIC_SHOP_PICK_TONE: u32 = pack_xrgb(0x2E, 0xC4, 0xF2);

/// Client-only marker for the pressed TOP100 board-chrome control:
/// a thin band inside that control's staged `State_MShop.bnd` rect.
/// The qualified press frame shows no ranking board (`docs/musicshop.md`),
/// so this inks no standing, sort, or wire claim. Ours-by-design tone,
/// staged geometry only (ADR-0003).
const MUSIC_SHOP_TOP100_TONE: u32 = pack_xrgb(0xE2, 0x8A, 0x3C);

/// Own-by-design ink for the active owned-face tab: a band inside
/// the tab's staged rect. The exe pins the tab codes and the flag
/// writes, not the face pixel, so no control-visibility claim rides
/// on this marker (#820).
const MUSIC_SHOP_FACE_TONE: u32 = pack_xrgb(0xE2, 0xC8, 0x4A);

/// Own-by-design ink for a row whose `o2ma<id>.ojn` exists: a small
/// tick inside the row's staged hit rect. The exe's bit `0x8` is
/// pinned; the tick's size and color are presentation (§1).
const MUSIC_SHOP_INSTALLED_TONE: u32 = pack_xrgb(0x50, 0xE0, 0x70);

/// Own-by-design inks for the two owned-row properties: the check
/// mark and the download-status mark. The exe sets the two per-row
/// properties (`FUN_004cbdd0`); their sprite faces are OPEN.
const MUSIC_SHOP_OWNED_MARK_TONE: u32 = pack_xrgb(0x30, 0xC4, 0xF2);
const MUSIC_SHOP_OWNED_STATUS_TONE: u32 = pack_xrgb(0xE0, 0xC0, 0x40);

/// Own-by-design band behind a pinned Korean shop notice. The text
/// and the host control code are pinned; the band is presentation.
const MUSIC_SHOP_NOTICE_TONE: u32 = pack_xrgb(0x30, 0x30, 0x90);

/// Own-by-design marker for a title-search hit. The exe calls its
/// hit-marking helper on each match; the band is presentation over
/// the staged `MS_EX` rect (#815).
const MUSIC_SHOP_SEARCH_HIT_TONE: u32 = pack_xrgb(0xC0, 0xD0, 0x40);

/// Gray hint ink sampled from the v3 exe frame at (60,542):
/// `rgb(99,113,107)`. The white edit plate makes white CP949 ink
/// invisible; the typed draft keeps the normal white ink.
const MUSIC_SHOP_SEARCH_HINT_INK: [u8; 4] = [0x63, 0x71, 0x6B, 0xFF];

/// Ink the search edit's Korean hint or typed draft, and mark the
/// matching song rows. Unstaged edit/rows paint nothing.
pub fn paint_music_shop_search(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    draft: &str,
    hits: &[u32],
    visible_rows: &[MusicShopRow],
) {
    if let Some(edit) = placements
        .iter()
        .find(|placed| placed.code == MSHOP_SEARCH_EDIT_CODE)
    {
        let (text, ink) = if draft.is_empty() {
            (MSHOP_SEARCH_HINT, MUSIC_SHOP_SEARCH_HINT_INK)
        } else {
            (draft, CP949_INK)
        };
        paint_text_into_rect(frame, edit.rect, text, ink);
    }
    for (row, slot) in visible_rows.iter().zip(music_shop_slots(placements)) {
        if !hits.contains(&row.song.song_id) {
            continue;
        }
        let rect = scale_rect_to_frame(&slot, frame.w, frame.h);
        let band_height = 2_u32.min(rect.h);
        if band_height == 0 {
            continue;
        }
        fill_rect(
            frame,
            &Rect {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: band_height,
            },
            MUSIC_SHOP_SEARCH_HIT_TONE,
        );
    }
}

/// Own-by-design marker for the active box-line frame. The exe stages
/// sprite frame `filter index + 1` into four controls; our shell has no
/// box-line sprite, so the one-based index positions one marker row in
/// each staged rect. This is presentation, not an exe pixel claim (#813).
const MUSIC_SHOP_BOX_FRAME_TONE: u32 = pack_xrgb(0xD8, 0xB0, 0x48);

/// Ink the active color filter's frame marker in the four staged
/// box-line rects. Unstaged controls paint nothing.
pub fn paint_music_shop_genre_frames(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    frame_index: u32,
) {
    for offset in 0..MSHOP_BOX_FRAME_COUNT {
        let Some(code) = MSHOP_BOX_FRAME_FIRST_CODE.checked_add(offset) else {
            continue;
        };
        let Some(staged) = placements.iter().find(|placed| placed.code == code) else {
            continue;
        };
        let rect = scale_rect_to_frame(&staged.rect, frame.w, frame.h);
        if rect.h == 0 || rect.w == 0 {
            continue;
        }
        let marker_row = frame_index.saturating_sub(1).min(rect.h - 1);
        let Some(y) = rect
            .y
            .checked_add(i32::try_from(marker_row).unwrap_or(i32::MAX))
        else {
            continue;
        };
        fill_rect(
            frame,
            &Rect {
                x: rect.x,
                y,
                w: rect.w,
                h: 1,
            },
            MUSIC_SHOP_BOX_FRAME_TONE,
        );
    }
}

/// Download bar inks: the exe's mode-2 style is pinned; the
/// numeric value and the notify behind it stay OPEN (#832), so the
/// split here is own-by-design presentation over staged rects.
const MUSIC_SHOP_PROGRESS_BASE_TONE: u32 = pack_xrgb(0x20, 0x60, 0x90);
const MUSIC_SHOP_PROGRESS_FILL_TONE: u32 = pack_xrgb(0x40, 0xB0, 0xE0);
/// Idle bar ink sampled from the v3 exe entry frame at (450,455)
/// and (600,498): `rgb(33,49,74)`. The staged progress sprite inks
/// a bright fill, so an idle panel covers it inside the staged rect.
const MUSIC_SHOP_PROGRESS_IDLE_TONE: u32 = pack_xrgb(0x21, 0x31, 0x4A);

/// Paint the download bars and row readout while a shop transfer
/// owns the panel. The completed/total pair comes from the staged
/// queue; unstaged rects or an idle panel paint nothing.
pub fn paint_music_shop_progress(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    readout: Option<(usize, usize)>,
) {
    let Some((completed, total)) = readout else {
        for code in [MSHOP_CURRENT_PROGRESS_CODE, MSHOP_TOTAL_PROGRESS_CODE] {
            if let Some(staged) = placements.iter().find(|placed| placed.code == code) {
                fill_rect(
                    frame,
                    &scale_rect_to_frame(&staged.rect, frame.w, frame.h),
                    MUSIC_SHOP_PROGRESS_IDLE_TONE,
                );
            }
        }
        return;
    };
    // The exe styles both bars with mode 2; only the current bar
    // carries the visible advance. The total bar keeps the base ink
    // because its numeric meaning is still OPEN (#832).
    for (code, filled) in [
        (MSHOP_CURRENT_PROGRESS_CODE, completed),
        (MSHOP_TOTAL_PROGRESS_CODE, 0),
    ] {
        let Some(staged) = placements.iter().find(|placed| placed.code == code) else {
            continue;
        };
        let rect = scale_rect_to_frame(&staged.rect, frame.w, frame.h);
        fill_rect(frame, &rect, MUSIC_SHOP_PROGRESS_BASE_TONE);
        let filled_width = if total == 0 {
            0
        } else {
            u64::from(rect.w)
                .saturating_mul(u64::try_from(filled.min(total)).unwrap_or(u64::MAX))
                .checked_div(u64::try_from(total).unwrap_or(u64::MAX))
                .and_then(|width| u32::try_from(width).ok())
                .unwrap_or(0)
        };
        fill_rect(
            frame,
            &Rect {
                x: rect.x,
                y: rect.y,
                w: filled_width,
                h: rect.h,
            },
            MUSIC_SHOP_PROGRESS_FILL_TONE,
        );
    }
    if let Some(native_rect) = music_shop_caption_rect(placements, MSHOP_DOWN_TEXT_CODE) {
        let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
        paint_text_into_rect(frame, rect, &format!("{completed}/{total}"), CP949_INK);
    }
}

/// Ink the lightboard tooltip's lines inside its staged rect while
/// the pointer hovers. An empty line vector uses the exe's pinned
/// fallback banner. The exe's `굴림체` face is presentation here:
/// the owner's cross-platform rasterizer supplies the glyphs.
pub fn paint_music_shop_tooltip(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    pointer: Option<(i32, i32)>,
    lines: &[String],
) {
    let Some((x, y)) = pointer else {
        return;
    };
    let Some(staged) = placements
        .iter()
        .find(|placed| placed.code == MSHOP_LIGHTBOARD_TOOLTIP_CODE)
    else {
        return;
    };
    if !rect_hit(&staged.rect, x, y) {
        return;
    }
    let rect = scale_rect_to_frame(&staged.rect, frame.w, frame.h);
    if rect.h == 0 || rect.w == 0 {
        return;
    }
    let fallback = [MSHOP_LIGHTBOARD_FALLBACK.to_owned()];
    let shown: &[String] = if lines.is_empty() { &fallback } else { lines };
    let Ok(count) = u32::try_from(shown.len()) else {
        return;
    };
    let line_height = (rect.h / count).max(1);
    for (index, line) in shown.iter().enumerate() {
        let Ok(offset) = u32::try_from(index) else {
            continue;
        };
        let Some(top) = rect
            .y
            .checked_add(i32::try_from(offset.saturating_mul(line_height)).unwrap_or(i32::MAX))
        else {
            continue;
        };
        paint_text_into_rect(
            frame,
            Rect {
                x: rect.x,
                y: top,
                w: rect.w,
                h: line_height,
            },
            line,
            CP949_INK,
        );
    }
}

/// Magenta ink of the lightboard rest notice, sampled from the v4
/// quiesced exe frame (dominant magenta glyph pixel `FF55FF`).
const MSHOP_LIGHTBOARD_ENTRY_INK: [u8; 4] = [0xFF, 0x55, 0xFF, 0xFF];

/// Rest-face lightboard notice. The v4 paired frame carries this
/// caption while the pointer is parked; a hover swaps in the pinned
/// tooltip lines. Geometry comes only from the staged control.
pub fn paint_music_shop_lightboard_entry_notice(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    hovered: bool,
) {
    if hovered {
        return;
    }
    let Some(staged) = placements
        .iter()
        .find(|placed| placed.code == MSHOP_LIGHTBOARD_TOOLTIP_CODE)
    else {
        return;
    };
    paint_caption_in_cell(
        frame,
        staged.rect,
        MSHOP_LIGHTBOARD_ENTRY_NOTICE,
        MSHOP_LIGHTBOARD_ENTRY_INK,
        CaptionAlignment::Right,
    );
}

/// Ink the CASHFILLUP id-only notice inside its staged rect. The
/// exe posts id `0x12` but its copy is OPEN; this caption is ours by
/// design, never an exe-text claim (#836).
pub fn paint_music_shop_cashfillup_notice(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    notice_id: Option<u16>,
) {
    let Some(id) = notice_id else {
        return;
    };
    let Some(staged) = placements
        .iter()
        .find(|placed| placed.code == MSHOP_CASH_FILLUP_CODE)
    else {
        return;
    };
    let caption = format!("{MSHOP_CASHFILLUP_FALLBACK_CAPTION} {id:#04x}");
    paint_text_into_rect(frame, staged.rect, &caption, CP949_INK);
}

/// Ink a standing shop notice inside the staged rect of the control
/// that posted it. An unstaged control paints nothing (ADR-0003).
pub fn paint_music_shop_notice(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    notice: Option<(MusicShopNotice, u32)>,
) {
    let Some((notice, code)) = notice else {
        return;
    };
    let Some(staged) = placements.iter().find(|placed| placed.code == code) else {
        return;
    };
    let rect = scale_rect_to_frame(&staged.rect, frame.w, frame.h);
    fill_rect(frame, &rect, MUSIC_SHOP_NOTICE_TONE);
    paint_text_into_rect(frame, rect, notice.text(), CP949_INK);
}

/// Ink the twelve owned rows' check and installed-status marks in
/// their staged `MY_EX` rects. The exe's -2/-4 corner offsets and
/// the twelve codes are pinned; the small marks are presentation.
pub fn paint_music_shop_owned_rows(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    rows: &[OwnedRow],
) {
    let title_cell = shop_template_rect(placements, MSHOP_OWNED_TITLE_TEXT_CODE);
    let time_cell = shop_template_rect(placements, MSHOP_OWNED_TIME_TEXT_CODE);
    for (row, slot) in rows.iter().zip(music_shop_owned_slots(placements)) {
        // The exe inks the two MY templates (left title, centered
        // creator/status) inside each staged MY_EX slot. A board
        // that stages only the hit area keeps the old full-slot
        // fallback; geometry still rides staged rects.
        match title_cell {
            Some(cell) => paint_caption_in_cell(
                frame,
                music_shop_tiled_rect(cell, &slot),
                &row.title,
                CP949_INK,
                CaptionAlignment::Left,
            ),
            None => paint_text_into_rect(frame, slot, &row.title, CP949_INK),
        }
        if let Some(cell) = time_cell {
            paint_caption_in_cell(
                frame,
                music_shop_tiled_rect(cell, &slot),
                if MSHOP_ENTRY_FREE_SONG_IDS.contains(&u32::from(row.song_id)) {
                    MSHOP_OWNED_FREE_OWNER
                } else {
                    MSHOP_OWNED_SERVICE_ENDED_OWNER
                },
                CP949_INK,
                CaptionAlignment::Center,
            );
        }
        let slot = scale_rect_to_frame(&slot, frame.w, frame.h);
        if row.marked {
            let mark = Rect {
                x: slot.x + i32::try_from(slot.w).unwrap_or(0) - 2,
                y: slot.y + i32::try_from(slot.h).unwrap_or(0) - 4,
                w: 2_u32.min(slot.w),
                h: 2_u32.min(slot.h),
            };
            fill_rect(frame, &mark, MUSIC_SHOP_OWNED_MARK_TONE);
        }
        if row.installed {
            let status = Rect {
                x: slot.x + 6,
                y: slot.y + 5,
                w: 2_u32.min(slot.w),
                h: 2_u32.min(slot.h),
            };
            fill_rect(frame, &status, MUSIC_SHOP_OWNED_STATUS_TONE);
        }
    }
}

/// Ink the selected owned-face tab inside its staged `.bnd` rect.
/// An unstaged tab paints nothing (ADR-0003).
pub fn paint_music_shop_face(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    face: MusicShopFace,
) {
    let code = match face {
        MusicShopFace::Download => MSHOP_TAB_DOWN_CODE,
        MusicShopFace::Cart => MSHOP_TAB_DELETE_CODE,
    };
    let Some(staged) = placements.iter().find(|placed| placed.code == code) else {
        return;
    };
    let rect = scale_rect_to_frame(&staged.rect, frame.w, frame.h);
    let band_height = 2_u32.min(rect.h);
    if band_height == 0 {
        return;
    }
    fill_rect(
        frame,
        &Rect {
            x: rect.x,
            y: rect.y + i32::try_from(rect.h - band_height).unwrap_or(0),
            w: rect.w,
            h: band_height,
        },
        MUSIC_SHOP_FACE_TONE,
    );
}

/// Ink the pressed TOP100 board control (if any) at its staged rect.
/// A missing rect or no press paints nothing.
pub fn paint_music_shop_top100_chrome(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    pressed_code: Option<u32>,
) {
    let Some(code) = pressed_code else {
        return;
    };
    let Some(staged) = placements.iter().find(|placed| placed.code == code) else {
        return;
    };
    let rect = scale_rect_to_frame(&staged.rect, frame.w, frame.h);
    let band_height = 2_u32.min(rect.h);
    if band_height == 0 {
        return;
    }
    fill_rect(
        frame,
        &Rect {
            x: rect.x,
            y: rect.y + i32::try_from(rect.h - band_height).unwrap_or(0),
            w: rect.w,
            h: band_height,
        },
        MUSIC_SHOP_TOP100_TONE,
    );
}

pub fn paint_music_shop_row_selection(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    selected: Option<usize>,
) {
    let Some(slot) = selected.and_then(|index| music_shop_slots(placements).into_iter().nth(index))
    else {
        return;
    };
    let slot = scale_rect_to_frame(&slot, frame.w, frame.h);
    let stripe_width = 3_u32.min(slot.w);
    let bar_height = 2_u32.min(slot.h);
    let bands = [
        Rect {
            x: slot.x,
            y: slot.y,
            w: stripe_width,
            h: slot.h,
        },
        Rect {
            x: slot.x,
            y: slot.y,
            w: slot.w,
            h: bar_height,
        },
        Rect {
            x: slot.x,
            y: slot.y + i32::try_from(slot.h - bar_height).unwrap_or(0),
            w: slot.w,
            h: bar_height,
        },
    ];
    for band in bands {
        fill_rect(frame, &band, MUSIC_SHOP_PICK_TONE);
    }
}

/// Native verdict band for a refused billing confirmation (#405 music,
/// #468 Payment2 gift). The geometry and flat `#67BAF0` blue are
/// ours-by-design from the archived `mcash_error_CashType.asp` tone
/// (ASSUMED older-version reference, NOT 7.55-oracle): no 7.55 notice
/// pixel or banner art survives, so no exe parity is claimed. 626x290
/// is the archived table size, centered in the 800x600 native canvas.
pub const BILLING_FAILURE_NOTICE_RECT: Rect = Rect {
    x: 87,
    y: 155,
    w: 626,
    h: 290,
};

/// Flat body blue of the archived error page (`bgcolor="#67BAF0"`).
const BILLING_FAILURE_NOTICE_TONE: u32 = pack_xrgb(0x67, 0xBA, 0xF0);
/// Black 9pt-like ink of the archived notice (rasterizer presentation,
/// owner decision `e8d01`).
const BILLING_FAILURE_NOTICE_INK: [u8; 4] = [0x00, 0x00, 0x00, 0xFF];
/// Top inset and pitch of the centered copy inside the band. The
/// source page leaves a 50 px header for its unarchived banner art;
/// the native stand-in keeps the space instead of inventing art.
const BILLING_FAILURE_NOTICE_TEXT_TOP: i32 = 44;
const BILLING_FAILURE_NOTICE_LINE_PITCH: i32 = 42;
const BILLING_FAILURE_NOTICE_LINE_HEIGHT: u32 = 24;

/// CP949 lines of the archived failed-purchase notice, in source
/// order, decoded from the Wayback body at
/// `~/scratch/o2jam-research/html/20090228054122_*_mcash_error_CashType_asp.html`.
/// Graded ASSUMED-from-older-version; the 7.55 notice copy remains
/// OPEN (`E-W-5011`) with a scripted-gateway tapped capture as the
/// next evidence step.
pub const BILLING_FAILURE_NOTICE_LINES_CP949: &[&[u8]] = &[
    &[
        0xA2, 0xC3, 0xA2, 0xC3, 0xA2, 0xC3, 0xA2, 0xC3, 0xA2, 0xC3, 0x20, 0xB0, 0xE1, 0xC1, 0xA6,
        0x20, 0xBF, 0xC0, 0xB7, 0xF9, 0x20, 0xA2, 0xC3, 0xA2, 0xC3, 0xA2, 0xC3, 0xA2, 0xC3, 0xA2,
        0xC3,
    ],
    &[
        0xBB, 0xF3, 0xC7, 0xB0, 0x20, 0xC1, 0xA4, 0xBA, 0xB8, 0xB0, 0xA1, 0x20, 0xBF, 0xC3, 0xB9,
        0xD9, 0xB8, 0xA3, 0xC1, 0xF6, 0x20, 0xBE, 0xCA, 0xBD, 0xC0, 0xB4, 0xCF, 0xB4, 0xD9, 0x2E,
    ],
    &[
        0xC0, 0xE1, 0xBD, 0xC3, 0xC8, 0xC4, 0x20, 0xB4, 0xD9, 0xBD, 0xC3, 0x20, 0xB0, 0xE1, 0xC1,
        0xA6, 0xB8, 0xA6, 0x20, 0xC7, 0xCF, 0xBF, 0xA9, 0x20, 0xC1, 0xD6, 0xBD, 0xCA, 0xBD, 0xC3,
        0xBF, 0xC0, 0x2E,
    ],
    &[
        0xB1, 0xD7, 0xB7, 0xA1, 0xB5, 0xB5, 0x20, 0xB0, 0xE1, 0xC1, 0xA6, 0xB0, 0xA1, 0x20, 0xB5,
        0xC7, 0xC1, 0xF6, 0x20, 0xBE, 0xCA, 0xC0, 0xBB, 0xBD, 0xC3, 0xBF, 0xA1, 0xB4, 0xC2,
    ],
    &[
        0xBF, 0xC0, 0xC5, 0xF5, 0xC0, 0xEB, 0x20, 0xC8, 0xA8, 0xC6, 0xE4, 0xC0, 0xCC, 0xC1, 0xF6,
        0xB3, 0xBB, 0x20, 0xB9, 0xAE, 0xC0, 0xC7, 0xC7, 0xCF, 0xB1, 0xE2, 0xB8, 0xA6, 0x20, 0xC0,
        0xCC, 0xBF, 0xEB, 0xC7, 0xCF, 0xBD, 0xC3, 0xB1, 0xE2, 0x20, 0xB9, 0xD9, 0xB6, 0xF8, 0xB4,
        0xCF, 0xB4, 0xD9, 0x2E,
    ],
];

/// Advance width of `text` at `scale`, with the same font-absent
/// stand-in the caption fitter uses, so centering survives a system
/// with no Korean face installed.
fn cp949_text_width_at_scale(text: &str, scale: f32) -> f32 {
    match o2jam_text::system_font() {
        Some(font) => o2jam_text::measure_text(font, text, scale),
        None => o2jam_text::font::measure_text_stand_in(text, scale),
    }
}

/// One centered line of the native notice inside its frame-scaled
/// slice of the band. The line fits the band width; the x origin is
/// the leftover half-width, never a hand-tuned literal.
fn paint_centered_billing_notice_line(frame: &mut Frame<'_>, line_rect: Rect, cp949_bytes: &[u8]) {
    let text = o2jam_text::decode_cp949_lossy(cp949_bytes);
    let scale = cp949_fit_scale(&text, line_rect.w as f32, line_rect.h as f32);
    if scale <= 0.0 {
        return;
    }
    let text_width = cp949_text_width_at_scale(&text, scale);
    let left = (line_rect.w as f32 - text_width).max(0.0) / 2.0;
    let mut image = o2jam_text::Image::blank(line_rect.w, line_rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        left as i32,
        0,
        &text,
        BILLING_FAILURE_NOTICE_INK,
        scale,
    );
    composite_text_image(frame, &line_rect, &image);
}

/// Paint the native billing-refusal notice over the current board:
/// flat blue band with the archived copy centered line by line. No
/// staged `.bnd` rect is involved — this is the owner-approved native
/// stand-in for the dead billing page (Spec #750), not exe geometry.
/// Both #405's scripted 5011 failure and #468's rejected gift claim
/// use it; the 7.55 notice pixels stay OPEN with a scripted capture.
pub fn paint_billing_failure_notice(frame: &mut Frame<'_>) {
    let band = scale_rect_to_frame(&BILLING_FAILURE_NOTICE_RECT, frame.w, frame.h);
    fill_rect(frame, &band, BILLING_FAILURE_NOTICE_TONE);
    for (index, cp949_bytes) in BILLING_FAILURE_NOTICE_LINES_CP949.iter().enumerate() {
        let native_line = Rect {
            x: BILLING_FAILURE_NOTICE_RECT.x,
            y: BILLING_FAILURE_NOTICE_RECT.y
                + BILLING_FAILURE_NOTICE_TEXT_TOP
                + i32::try_from(index).unwrap_or(0) * BILLING_FAILURE_NOTICE_LINE_PITCH,
            w: BILLING_FAILURE_NOTICE_RECT.w,
            h: BILLING_FAILURE_NOTICE_LINE_HEIGHT,
        };
        paint_centered_billing_notice_line(
            frame,
            scale_rect_to_frame(&native_line, frame.w, frame.h),
            cp949_bytes,
        );
    }
}

/// Song-shop board overlay over the staged side art: the visible
/// rows' catalog cells into the staged row-local `MMS` templates
/// riding the staged `MS_EX` slot origins (title, bought flag,
/// artist, editor, genre, tempo; price/diff cells stay blank — no
/// pinned source), plus the list pager readout. Text renders
/// through the system face (owner decision `e8d01`).
pub fn paint_music_shop_board(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    rows: &[MusicShopRow],
    list_page: usize,
    list_pages: usize,
) {
    paint_music_shop_rows(frame, placements, rows);
    if let Some((cur, tot)) = music_shop_page_rects(placements) {
        paint_text_into_rect(frame, cur, &(list_page + 1).to_string(), CP949_INK);
        paint_text_into_rect(frame, tot, &list_pages.to_string(), CP949_INK);
    }
}

/// Visible rows into the six staged `MMS` templates at their slots
/// (unstaged slots or templates: no rows).
fn paint_music_shop_rows(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    rows: &[MusicShopRow],
) {
    let slots = music_shop_slots(placements);
    if slots.is_empty() {
        return;
    }
    let templates: Vec<Rect> = MSHOP_ROW_TEXT_CODES
        .iter()
        .filter_map(|code| shop_template_rect(placements, *code))
        .collect();
    if templates.len() != MSHOP_ROW_TEXT_CODES.len() {
        return;
    }
    for (slot, row) in slots.iter().zip(rows.iter()) {
        if row.installed {
            let scaled = scale_rect_to_frame(slot, frame.w, frame.h);
            fill_rect(
                frame,
                &Rect {
                    x: scaled.x + 2,
                    y: scaled.y + 2,
                    w: 3_u32.min(scaled.w),
                    h: 3_u32.min(scaled.h),
                },
                MUSIC_SHOP_INSTALLED_TONE,
            );
        }
        let cells = [
            row.song.title.clone(),
            music_shop_buy_caption(row),
            row.song.artist.clone(),
            row.song.editor.clone(),
            lanking_genre_caption(row.song.genre),
            lanking_bpm_caption(row.song.bpm),
        ];
        for (template, text) in templates.iter().zip(cells.iter()) {
            if text.is_empty() {
                continue;
            }
            paint_text_into_rect(
                frame,
                music_shop_tiled_rect(*template, slot),
                text,
                CP949_INK,
            );
        }
        // Optional cells stay independent of the six required MMS
        // templates: a board that stages only the original templates
        // still paints, while the qualified 7.55 dump inks all three.
        let optional = [
            (MSHOP_DIFF_CODE, lanking_diff_caption(row.song.levels)),
            (
                MSHOP_GEM_PRICE_CODE,
                row.gem_price
                    .map(|price| format!("G {price}"))
                    .unwrap_or_default(),
            ),
            (
                MSHOP_WON_PRICE_CODE,
                row.won_price
                    .map(|price| format!("W {price}"))
                    .unwrap_or_default(),
            ),
        ];
        for (code, text) in optional {
            if text.is_empty() {
                continue;
            }
            if let Some(rect) = shop_template_rect(placements, code) {
                paint_text_into_rect(frame, music_shop_tiled_rect(rect, slot), &text, CP949_INK);
            }
        }
    }
}

/// Bottom money texts (`0x1A400019`/`0x1A40001A`) from the one wallet
/// the shops share: `G`/`W` amounts stay `--` until 5029 folds them.
pub fn paint_music_shop_wallet(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    wallet: &WalletState,
) {
    let cells = [
        (MSHOP_MONEY_GEM_CODE, wallet.gem),
        (MSHOP_MONEY_WON_CODE, wallet.cash_point),
    ];
    for (code, amount) in cells {
        if let Some(rect) = music_shop_caption_rect(placements, code) {
            // The G/W labels are baked into the staged backdrop; the
            // TEXT cells hold only the amount (v3 exe pair).
            let text = wallet_amount_caption(amount);
            // The v3 exe pair inks the amount at the cell's right
            // edge (x 135 in rect 41..136); left anchoring missed it
            // by half the cell (senior note, commit 3701ddb).
            paint_caption_in_cell(frame, rect, &text, CP949_INK, CaptionAlignment::Right);
        }
    }
}

/// Checked music-cart rows into the staged row-local music `CART`
/// templates (position, title; price stays blank — same OPEN as the
/// list cells), plus the cart pager readout.
pub fn paint_music_shop_cart(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    cart: &[LankingSong],
    cart_page: usize,
    cart_pages: usize,
) {
    let slots = music_shop_cart_slots(placements);
    let num = shop_template_rect(placements, MSHOP_CART_NUM_CODE);
    let name = shop_template_rect(placements, MSHOP_CART_ITEMNAME_CODE);
    let (Some(num), Some(name)) = (num, name) else {
        return;
    };
    if slots.is_empty() {
        return;
    }
    for (index, (slot, song)) in slots.iter().zip(cart.iter()).enumerate() {
        let position = cart_page
            .saturating_mul(MSHOP_CART_ROWS)
            .saturating_add(index)
            .saturating_add(1)
            .to_string();
        paint_text_into_rect(
            frame,
            music_shop_tiled_rect(num, slot),
            &position,
            CP949_INK,
        );
        paint_text_into_rect(
            frame,
            music_shop_tiled_rect(name, slot),
            &song.title,
            CP949_INK,
        );
    }
    if let Some((cur, tot)) = music_shop_cart_page_rects(placements) {
        paint_text_into_rect(frame, cur, &(cart_page + 1).to_string(), CP949_INK);
        paint_text_into_rect(frame, tot, &cart_pages.to_string(), CP949_INK);
    }
}

/// Ink for CP949 caption and message text: white. Presentation
/// choice (owner decision `e8d01` — glyph rasterization is
/// presentation, not gameplay); the exe ink for these `TEXT`
/// controls is unpinned, the rects and strings are.
const CP949_INK: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

/// One GDI cell height of the TEXT control face (`Gulimche 6x12`,
/// `docs/gdi-text.md` font table). Explicit `\n` line breaks in a
/// CP949 message advance by one cell, not by the fitted glyph em.
const CP949_TEXT_CELL_HEIGHT_PX: f32 = 12.0;

/// Em extent fitting `text` into a `w`×`h` rect: the smaller of the
/// height cap and the widest-row fit (advances scale linearly, so
/// one measure at 1 px/em fixes both). The font-absent stand-in
/// measures through the same fit, so layout survives without the
/// system face.
fn cp949_fit_scale(text: &str, w: f32, h: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 || text.is_empty() {
        return h.max(0.0);
    }
    let width_per_em = cp949_width_at_scale(text, 1.0);
    let width_fit = if width_per_em > 0.0 {
        w / width_per_em
    } else {
        h
    };
    h.min(width_fit).max(0.0)
}

/// Scale for a message with stored line breaks: share the rect
/// height between rows, but never enlarge a GDI 12 px text cell.
fn cp949_multiline_scale(text: &str, w: f32, h: f32) -> f32 {
    let row_count = text.split('\n').count();
    let row_height = h / row_count as f32;
    cp949_fit_scale(text, w, row_height).min(CP949_TEXT_CELL_HEIGHT_PX)
}

/// Top inset that centers a block of `row_count` fixed-height rows
/// inside the staged rect — the three-line channel notice lands
/// centered in its TEXT rect in the 7.55 capture (#882).
fn multiline_top_inset(rect_height: u32, row_count: usize, scale: f32) -> i32 {
    let block_height = row_count as f32 * scale;
    ((rect_height as f32 - block_height) * 0.5).round() as i32
}

/// Advance of `text` at `px_scale`, through the same font the painter uses.
fn cp949_width_at_scale(text: &str, px_scale: f32) -> f32 {
    match o2jam_text::system_font() {
        Some(font) => o2jam_text::measure_text(font, text, px_scale),
        None => o2jam_text::font::measure_text_stand_in(text, px_scale),
    }
}

/// Exe TEXT cell width for dialogs: Gulimche 6x12 (`docs/gdi-text.md`,
/// stager `0x50f280`). The modern face keeps this cell; only the glyph
/// outlines differ (owner decision `e8d01`).
const MESSAGE_CELL_WIDTH_PX: f32 = 6.0;
/// Exe TEXT cell height for dialogs, in native pixels.
const MESSAGE_CELL_HEIGHT_PX: i32 = 12;
/// First wrapped ink row below the staged message rect top: the paired
/// 2026-11-23 cart frame inks at y=285 in rect `(341,247)-(523,358)`.
const WRAPPED_MESSAGE_INK_TOP_PX: i32 = 38;
/// Left bearing of that ink inside the staged rect (x=342 in x=341).
const WRAPPED_MESSAGE_INK_LEFT_PX: i32 = 1;

/// Split a caption at word boundaries so every line fits `max_width`.
/// A single word longer than the rect stays whole; the canvas clips it.
fn wrap_caption_lines(text: &str, max_width: f32, px_scale: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if current.is_empty() || cp949_width_at_scale(&candidate, px_scale) <= max_width {
            current = candidate;
        } else {
            lines.push(current);
            current = word.to_owned();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Paint a caption too wide for one line at the exe's cells, wrapping it
/// inside its staged `.bnd` rect. The first line's bearing and the line
/// pitch come from the paired exe frame; only glyph shapes are ours.
fn paint_message_caption_in_rect(
    frame: &mut Frame<'_>,
    native_rect: Rect,
    text: &str,
    ink: [u8; 4],
) {
    if frame.w == 0 || frame.h == 0 || native_rect.w == 0 || native_rect.h == 0 || text.is_empty() {
        return;
    }
    let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let frame_scale = rect.w as f32 / native_rect.w as f32;
    let cell_per_em = cp949_width_at_scale("가", 1.0);
    if cell_per_em <= 0.0 {
        return;
    }
    let px_scale = MESSAGE_CELL_WIDTH_PX * frame_scale / cell_per_em;
    let lines = wrap_caption_lines(text, rect.w as f32, px_scale);
    if lines.len() < 2 {
        paint_text_into_rect(frame, native_rect, text, ink);
        return;
    }
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    for (row, line) in lines.iter().enumerate() {
        let top = (WRAPPED_MESSAGE_INK_TOP_PX + row as i32 * MESSAGE_CELL_HEIGHT_PX) as f32;
        let y = (top * frame_scale).round() as i32;
        if y >= rect.h as i32 {
            break;
        }
        let x = (WRAPPED_MESSAGE_INK_LEFT_PX as f32 * frame_scale).round() as i32;
        o2jam_text::draw_text(
            &mut image,
            o2jam_text::system_font(),
            x,
            y,
            line,
            ink,
            px_scale,
        );
    }
    composite_text_image(frame, &rect, &image);
}

/// Paint the composited text image over the frame at `rect`.
fn composite_text_image(frame: &mut Frame<'_>, rect: &Rect, image: &o2jam_text::Image) {
    let (frame_w, frame_h) = (frame.w as i32, frame.h as i32);
    for row in 0..rect.h as i32 {
        let dest_y = rect.y + row;
        if dest_y < 0 || dest_y >= frame_h {
            continue;
        }
        for col in 0..rect.w as i32 {
            let dest_x = rect.x + col;
            if dest_x < 0 || dest_x >= frame_w {
                continue;
            }
            let Some(src) = image
                .rgba
                .get(((row as usize) * (image.width as usize) + col as usize) * 4..)
            else {
                continue;
            };
            let alpha = u32::from(src[3]);
            if alpha == 0 {
                continue;
            }
            let at = dest_y as usize * frame.w + dest_x as usize;
            let Some(dest) = frame.buf.get_mut(at) else {
                continue;
            };
            // Stored rgb is pre-multiplied (`ink * alpha / 255`), so
            // alpha-over is add the un-inked back portion, per channel.
            let blend = |ink: u8, back: u32| -> u32 { u32::from(ink) + back * (255 - alpha) / 255 };
            let red = blend(src[0], (*dest >> 16) & 0xFF);
            let green = blend(src[1], (*dest >> 8) & 0xFF);
            let blue = blend(src[2], *dest & 0xFF);
            *dest = (red << 16) | (green << 8) | blue;
        }
    }
}

/// Paint one string into its native rect (scaled to the frame),
/// left-anchored at the rect origin and size-fit to the rect (the
/// same `TEXT` rule as the planet captions). Geometry always comes
/// from the caller (staged `.bnd` rects) — never from literals
/// here. Bare ink, no scrim: the string fits the rect by
/// construction.
fn paint_text_into_rect(frame: &mut Frame<'_>, native_rect: Rect, text: &str, ink: [u8; 4]) {
    if frame.w == 0 || frame.h == 0 || native_rect.w == 0 || native_rect.h == 0 || text.is_empty() {
        return;
    }
    let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let row_count = text.split('\n').count();
    let scale = if row_count > 1 {
        cp949_multiline_scale(text, rect.w as f32, rect.h as f32)
    } else {
        cp949_fit_scale(text, rect.w as f32, rect.h as f32)
    };
    if scale <= 0.0 {
        return;
    }
    let top_inset = if row_count > 1 {
        multiline_top_inset(rect.h, row_count, scale)
    } else {
        0
    };
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        0,
        top_inset,
        text,
        ink,
        scale,
    );
    composite_text_image(frame, &rect, &image);
}

/// Horizontal ink placement inside an exe TEXT cell. `DrawTextA`
/// flags are pinned in `docs/gdi-text.md`; each family carries its
/// own pin, so this enum never guesses a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CaptionAlignment {
    /// Left edge of the cell (plain `TextOutA` families).
    Left,
    /// Center of the cell (`DT_CENTER`).
    Center,
    /// Right edge of the cell (`DT_RIGHT`).
    Right,
}

/// Rasterizer origin inside a TEXT cell, in pixels. The 7.55 pair
/// inks two pixels below the cell top (`docs/gdi-text.md` GDI
/// bearing); our glyph cap rounds one pixel above the requested
/// origin (same pin as commit `3701ddb` on `cb2/fix880b`).
const CAPTION_INK_ORIGIN_Y: i32 = 3;

/// Paint one caption in its staged TEXT cell with the exe's ink
/// bearing and the family's own horizontal alignment. Geometry
/// always comes from the staged `.bnd` rect; only presentation
/// (glyph outlines) comes from the modern rasterizer (owner `e8d01`).
pub(super) fn paint_caption_in_cell(
    frame: &mut Frame<'_>,
    native_rect: Rect,
    text: &str,
    ink: [u8; 4],
    alignment: CaptionAlignment,
) {
    if frame.w == 0 || frame.h == 0 || native_rect.w == 0 || native_rect.h == 0 || text.is_empty() {
        return;
    }
    let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let scale = cp949_fit_scale(text, rect.w as f32, rect.h as f32);
    if scale <= 0.0 {
        return;
    }
    let text_width = cp949_width_at_scale(text, scale);
    let leftover = (rect.w as f32 - text_width).max(0.0);
    let left = match alignment {
        CaptionAlignment::Left => 0.0,
        CaptionAlignment::Center => leftover * 0.5,
        CaptionAlignment::Right => leftover,
    };
    let left = left.round() as i32;
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        left,
        CAPTION_INK_ORIGIN_Y,
        text,
        ink,
        scale,
    );
    composite_text_image(frame, &rect, &image);
}

/// Packed lobby ink as composited caption RGBA (opaque).
fn caption_ink(ink: u32) -> [u8; 4] {
    [(ink >> 16) as u8, (ink >> 8) as u8, ink as u8, 0xFF]
}

/// Paint one CP949 string into its native rect (scaled to the
/// frame), left-anchored at the rect origin (the same `TEXT` rule
/// as the waiting captions) and size-fit to the rect. Geometry
/// always comes from the caller (staged `.bnd` rects or the
/// dialog-relative message rect) — never from literals here.
fn paint_cp949_into_rect(frame: &mut Frame<'_>, native_rect: Rect, cp949_bytes: &[u8]) {
    let text = o2jam_text::decode_cp949_lossy(cp949_bytes);
    paint_text_into_rect(frame, native_rect, &text, CP949_INK);
}

/// Rasterizer origin inside an emblem caption's TEXT cell, in native
/// pixels. The 7.55 oracle pair inks at y=127 in a cell whose top is
/// y=125 (`~/scratch/fix880b-findings.md`); our glyph cap rounds one
/// pixel above the requested origin, so this origin lands the ink at
/// the exe's +2 px GDI bearing. Geometry stays in the staged rect.
const PLANET_CAPTION_INK_ORIGIN_Y: i32 = 3;

/// Paint one emblem caption the way the exe's `TEXT` control does:
/// horizontally centered in its staged cell (`docs/gdi-text.md`,
/// `DT_CENTER` mode) with the pinned GDI ink bearing below the cell
/// top. Shared text callers keep their own left/top anchoring.
fn paint_planet_caption_in_cell(frame: &mut Frame<'_>, native_rect: Rect, text: &str) {
    if frame.w == 0 || frame.h == 0 || native_rect.w == 0 || native_rect.h == 0 || text.is_empty() {
        return;
    }
    let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let scale = cp949_fit_scale(text, rect.w as f32, rect.h as f32);
    if scale <= 0.0 {
        return;
    }
    let text_width = cp949_width_at_scale(text, scale);
    let left = ((rect.w as f32 - text_width).max(0.0) * 0.5).round() as i32;
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        left,
        PLANET_CAPTION_INK_ORIGIN_Y,
        text,
        CP949_INK,
        scale,
    );
    composite_text_image(frame, &rect, &image);
}

/// Emblem captions over the staged board: each `TEXT` placement
/// paints its runtime string into its own `.bnd` rect (ordinals
/// 6..10, `docs/planet.md`). No placement, no caption — geometry
/// comes from the parsed bind file only (owner rule `f38f3`).
fn paint_planet_captions(frame: &mut Frame<'_>, placements: &[ScenePlacement]) {
    for placement in placements {
        if let Some(caption) = planet_caption_for_code(placement.code) {
            let text = o2jam_text::decode_cp949_lossy(caption.cp949);
            paint_planet_caption_in_cell(frame, placement.rect, &text);
        }
    }
}

/// Carousel inputs the planet painter needs: the mounted banner frame
/// and the rotation flag that selects the PLAYSTOP face (7.55 capture,
/// `docs/hover.md` #711 pin). Grouped so the painter
/// consumes staged view state, never the carousel object itself.
#[derive(Debug, Clone, Copy)]
pub struct PlanetPromoPaint {
    /// One-based banner frame; `0` paints no banner.
    pub banner_index: u32,
    /// True while the carousel rotates (stop-square face).
    pub spinning: bool,
}

impl PlanetPromoPaint {
    /// Board state while the carousel rotates.
    #[must_use]
    pub const fn spinning(banner_index: u32) -> Self {
        Self {
            banner_index,
            spinning: true,
        }
    }

    /// Board state after PLAYSTOP holds the carousel.
    #[must_use]
    pub const fn stopped(banner_index: u32) -> Self {
        Self {
            banner_index,
            spinning: false,
        }
    }
}

/// Planet: staged solo-mission art with the exe-pinned emblem
/// captions, flat card fallback. The captions ride the `TEXT`
/// placements' `.bnd` rects (ordinals 6..10) with their
/// `docs/planet.md`-pinned strings, rasterized through `o2jam-text`
/// (owner decision `e8d01`); without staged placements no caption
/// paints (no fallback geometry is invented). A picked band draws
/// the shared amber cursor inside its pinned `.bnd` button rect —
/// highlight rendering is unevidenced, so this cursor is an OPEN
/// stand-in (docs/planet.md), not exe parity. The hovered board
/// control (`hover`, from pointer motion alone) repaints its pinned
/// highlight art on top (see [`paint_planet_hover`]); `None` keeps
/// the rest stack, so unhover clears the highlight by repaint. The
/// held board control (`press`, armed on left-down over the bare
/// board) repaints its pressed frame over that (see
/// [`paint_planet_press`]); `None` keeps the hover/rest paint. The
/// carousel `promo` picks the banner frame (1-based — `0` paints no
/// banner, [`paint_planet_promo_frame`]) and the PLAYSTOP face
/// ([`paint_planet_playstop_face`]).
pub fn paint_planet_scene(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    selection: Option<PlanetBand>,
    hover: Option<PlanetAction>,
    press: Option<PlanetAction>,
    promo: PlanetPromoPaint,
) {
    // The banner body's three frames stack in the shared rest paint
    // ([`SceneArt::paint_backdrop`] keeps the verified all-layers
    // stack every other single body relies on), but the exe's image
    // control mounts ONE frame per view: paint the rest WITHOUT the
    // banner placement, then lay only the selected frame over it —
    // its colorkey holes show the backdrop, never a sibling frame
    // (#170 ghosting; the exe refresh arm VA `0x48e4b0`).
    paint_planet_board(frame, art, promo);
    paint_planet_hover(frame, art, hover);
    paint_planet_press(frame, art, press);
    paint_planet_captions(frame, &art.placements);
    if let Some(band) = selection {
        let rect = planet_band_rect(band);
        paint_cursor(frame, rect.x + 10, rect.y + rect.h as i32 / 2);
    }
}

/// Staged planet board plus the banner frame the exe's image control
/// mounts. The channel dialog paints this same board behind its sheet,
/// so it needs the carousel index, not just the staged art.
#[derive(Debug, Clone, Copy)]
pub struct StagedPlanetBoard<'a> {
    /// Bound `State_Planet` scene art.
    pub art: &'a SceneArt,
    /// Banner frame and PLAYSTOP face mounted by the exe's controls.
    pub promo: PlanetPromoPaint,
}

/// Planet board beneath a scene: the exe's image control mounts ONE
/// banner frame (refresh arm VA `0x48e4b0`), so the rest stack omits
/// the banner placement and the selected layer is laid over it. Both
/// the bare board and the board under the channel dialog use this path.
/// The five emblem buttons are omitted from the rest stack too: their
/// dim rest art is baked into the bound `State_Planet.ojs` sheet, and
/// the exe mounts a button body only on hover/press. The matched #361
/// A/B measures sheet versus exe within ~1 level per channel; the old
/// all-layers stack painted the bright button body over the sheet.
/// Other bodies keep their verified all-layers stack.
fn paint_planet_board(frame: &mut Frame<'_>, art: &SceneArt, promo: PlanetPromoPaint) {
    art.paint_backdrop_live(frame, Screen::Planet, |placed| {
        placed.code != PLANET_PROMO_IMAGE_CODE
            && planet_action_for_code(placed.code)
                != Some(PlanetAction::Promo {
                    cell: PlanetPromo::Stop,
                })
            && !matches!(
                planet_action_for_code(placed.code),
                Some(PlanetAction::Emblem { .. })
            )
    });
    paint_planet_promo_frame(frame, art, promo.banner_index);
    paint_planet_playstop_face(frame, art, promo.spinning);
}

/// Raw record of `Planet_Btn_Promotion_PlayStop.ojs` painted while the
/// carousel rotates: the stop square. Direct record-vs-capture match,
/// 624/624 opaque px in five captures (boot rest, second click, +7 s),
/// `docs/hover.md` 2026-11-11 method; record sha256
/// `5c035ae665967786b177716c28e3d467147d132990f24b541fa4225f86915ed8`.
const PLAYSTOP_SPINNING_RECORD: usize = 0;

/// Raw record painted once PLAYSTOP holds the carousel: the play
/// triangle, 624/624 opaque px after the first click and again after
/// two 3000 ms ticks. Record sha256
/// `812e25c0d23da5154a2f3dd2e6c10f35a57d5af69e1ca5c5c0421c713991d448`.
const PLAYSTOP_STOPPED_RECORD: usize = 3;

/// Raw record of the pressed stop square (dark variant): best match
/// 521/624 px in the `clean-playstop-press` capture, the rest being
/// cursor ink. The exact pressed index stays OPEN in `docs/hover.md`;
/// this keeps the #111 press art distinct from the explicit rest face.
const PLAYSTOP_PRESSED_RECORD: usize = 2;

/// PLAYSTOP state face: the exe paints raw record 0 while the carousel
/// rotates (stop square) and record 3 once it holds (play triangle),
/// even though all six records share one anchor (#711 capture pair).
/// The button is omitted from the rest stack so the selected record is
/// the only face underneath hover and press. A stage without record 3
/// keeps record 0 — never invented art.
fn paint_planet_playstop_face(frame: &mut Frame<'_>, art: &SceneArt, spinning: bool) {
    let Some(placement) = art.placements.iter().find(|placed| {
        planet_action_for_code(placed.code)
            == Some(PlanetAction::Promo {
                cell: PlanetPromo::Stop,
            })
    }) else {
        return;
    };
    let Some(figure) = placement
        .sprite
        .as_ref()
        .and_then(|sprite| art.figures.get(sprite))
    else {
        return;
    };
    let record = if !spinning && figure.layer(PLAYSTOP_STOPPED_RECORD).is_some() {
        PLAYSTOP_STOPPED_RECORD
    } else {
        PLAYSTOP_SPINNING_RECORD
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placement
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    paint_placement_layer(frame, placement, &art.figures, record, shares_body, None);
}

/// Promotion-banner frame: the exe's image control paints one frame
/// per view (refresh arm VA `0x48e4b0` → `set_frame(index)`), so the
/// shell paints the rest stack without the banner placement
/// ([`paint_planet_scene`]) and then repaints the staged
/// `IDC_IMAGE_SPLANET_PROMOTION` body's selected layer over it. The
/// frames share one anchor and size (the shipped body carries three
/// 701x333 records at (49,163)), and the colorkey-0 holes show the
/// backdrop — never a sibling frame (#170). Index 0 or an unstaged
/// body paints nothing; an index past the staged layers keeps the
/// backdrop (the shell never invents frames). Runs before hover/press
/// so their repaints stay on top.
fn paint_planet_promo_frame(frame: &mut Frame<'_>, art: &SceneArt, promo_index: u32) {
    let Some(placement) = art
        .placements
        .iter()
        .find(|placed| placed.code == PLANET_PROMO_IMAGE_CODE)
    else {
        return;
    };
    let Some(layer) = promo_index
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return;
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placement
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    paint_placement_layer(frame, placement, &art.figures, layer, shares_body, None);
}

/// Planet-board hover highlight: the pinned per-control art from
/// the Wine sweep (`docs/hover.md`, EXE `83a04541…72c823b`) — three
/// lit emblems reveal their body's own 84x18 plate band (frame 0
/// clipped to the staged button rect's plate band, no frame swap),
/// the gear and exit discs and the promo arrows repaint body frame 0
/// at full colour over the dimmed rest, and only the PLAYSTOP cell
/// swaps frames (rest raw record 0, hover raw record 1 — pinned
/// 2026-11-11 by direct record-vs-capture match, `docs/hover.md`).
/// The QUEST/TUTORIAL plates are EXE-static (0 changed px on both
/// plates, `docs/hover.md`) and paint nothing. Every pixel comes
/// from the staged body — a control with no staged figure (or no
/// layer for the arm) paints nothing, never a stand-in.
/// The rest stack underneath is untouched (its dim/selected-frame
/// rendering stays OPEN, `docs/planet.md`), so unhover clears the
/// highlight by repaint.
fn paint_planet_hover(frame: &mut Frame<'_>, art: &SceneArt, hover: Option<PlanetAction>) {
    let Some(action) = hover else {
        return;
    };
    // Static QUEST/TUTORIAL plates: EXE-static, no highlight
    // (0 changed px on both plates, `docs/hover.md`).
    if matches!(action, PlanetAction::Emblem { band: None }) {
        return;
    }
    let Some(placement) = art
        .placements
        .iter()
        .find(|placed| planet_action_for_code(placed.code) == Some(action))
    else {
        return;
    };
    let (layer_index, clip) = match action {
        // Static QUEST/TUTORIAL plates never reach hover paint.
        PlanetAction::Emblem { band: None } => return,
        // Plate reveal: layer 0 clipped to the plate band the staged
        // button rect carries (`side::planet_plate_clip`).
        PlanetAction::Emblem { band: Some(band) } => {
            (0, Some(planet_plate_clip(&placement.rect, band)))
        }
        // The one true frame swap: hover arms layer 1 (the staged
        // second record; a single-record stage keeps the rest look).
        PlanetAction::Promo {
            cell: PlanetPromo::Stop,
        } => (playstop_hover_layer(art, action), None),
        // Dimmed-rest buttons: hover restores frame 0 at full colour.
        PlanetAction::QuitConfirm | PlanetAction::OptionsDialog | PlanetAction::Promo { .. } => {
            (0, None)
        }
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placement
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    paint_placement_layer(
        frame,
        placement,
        &art.figures,
        layer_index,
        shares_body,
        clip,
    );
}

/// Planet-board pressed frame (#111): while the left button holds a
/// board control, the control repaints its pressed frame over the
/// hover/rest paint. The swap is per-control, pinned against the
/// `clean-playstop`/`clean-promo-left` captures (344 px / 255 px,
/// repaint confined to the control rect): a staged body carrying a
/// second record repaints its alternate record (over the rest/state
/// face — the same repaint arm the hover's full-colour restore uses;
/// PLAYSTOP uses record 2 once #711 makes record 0 its explicit rest
/// face); a single-record control (the option gear, no
/// visible pressed art in the `geartest` crops) paints nothing, never
/// a stand-in. Emblems paint nothing either (emblem press == hover
/// pixel-identical, #111 arm 1 caveat; the orange ring is the
/// channel-dialog ACTIVE marker, not a pressed frame). Which raw
/// record the exe's pressed art is, per control, stays OPEN; the
/// `clean-playstop-press` crop best-matches record 2 (521/624 px,
/// the rest cursor ink), which is why PLAYSTOP uses that record.
fn paint_planet_press(frame: &mut Frame<'_>, art: &SceneArt, press: Option<PlanetAction>) {
    let Some(action) = press else {
        return;
    };
    // Emblems: no visible pressed art pinned (press == hover).
    if matches!(action, PlanetAction::Emblem { .. }) {
        return;
    }
    let Some(figure) = hovered_placement_figure(art, action) else {
        return;
    };
    // Per-control rule: no second record, no pressed frame (the gear).
    if figure.layer(1).is_none() {
        return;
    }
    // PLAYSTOP's rest face is now the explicitly selected record 0,
    // so its pressed art must stay a distinct record (#111). The raw
    // press index is an OPEN stand-in pinned only by the dark-square
    // crop; hover still paints its own record on top.
    let pressed_record = if matches!(
        action,
        PlanetAction::Promo {
            cell: PlanetPromo::Stop
        }
    ) {
        PLAYSTOP_PRESSED_RECORD
    } else {
        0
    };
    if figure.layer(pressed_record).is_none() {
        return;
    }
    let Some(placement) = art
        .placements
        .iter()
        .find(|placed| planet_action_for_code(placed.code) == Some(action))
    else {
        return;
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placement
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    // The pressed frame repaint: the control's alternate record
    // full-colour over the rest/state face, confined to the
    // placement's own rect by the blit — the pinned repaint region.
    paint_placement_layer(
        frame,
        placement,
        &art.figures,
        pressed_record,
        shares_body,
        None,
    );
}

/// Frame index the PLAYSTOP hover arms: 1 while the staged body
/// carries a second record, else 0 (a single-record stage has no swap
/// to show, so the hover keeps the rest look).
fn playstop_hover_layer(art: &SceneArt, action: PlanetAction) -> usize {
    usize::from(hovered_figure_has_second_record(art, action))
}

/// Whether the hovered control's staged body carries a second record
/// (the PLAYSTOP hover frame: raw record 1 against rest record 0,
/// pinned 2026-11-11 by direct record-vs-capture match over the six
/// raw 26x24 records of `Planet_Btn_Promotion_PlayStop.ojs` — the old
/// two-record model in `docs/hover.md` described the frame pairing,
/// not the record count).
fn hovered_figure_has_second_record(art: &SceneArt, action: PlanetAction) -> bool {
    hovered_placement_figure(art, action).is_some_and(|figure| figure.layer(1).is_some())
}

/// Staged figure behind the hovered control (`None` while the control,
/// its sprite, or its figure is unstaged — hover then paints nothing).
fn hovered_placement_figure(art: &SceneArt, action: PlanetAction) -> Option<&AvatarFigure> {
    art.placements
        .iter()
        .find(|placed| planet_action_for_code(placed.code) == Some(action))
        .and_then(|placement| placement.sprite.as_ref())
        .and_then(|sprite| art.figures.get(sprite))
}

/// Dialog origin (native px) of a staged `DIALOG_*` block: the
/// `BACK` placement's rect origin (the backdrop sprite anchor —
/// e.g. `Dialog_Question1.ojs` at (240,189), `Dialog_Option_*.ojs`
/// at (217,42)). Dialog children author dialog-local anchors
/// (verify: `Question1_OK.ojs` at (52,181) must land at absolute
/// (292,370) inside the backdrop, matching capture `exit-state.png`;
/// `Option_Save.ojs` at (160,477) lands at (377,519) inside its
/// backdrop), so children paint and hit-test at origin + local.
/// `(0,0)` without a staged backdrop (select-music-like sheets
/// then paint absolute, correct for their near-origin backdrops).
#[must_use]
pub fn dialog_origin(placements: &[ScenePlacement]) -> (i32, i32) {
    placements
        .iter()
        .find(|placement| is_dialog_backdrop(placement))
        .map_or((0, 0), |placement| (placement.rect.x, placement.rect.y))
}

/// True for a dialog's sheet-frame placement. The frame is scenery:
/// its body already paints at its parsed anchor, so control hit tests
/// and frame-state repaints must skip it. Treating it as a control
/// adds the dialog origin to a record that is already absolute and
/// paints a second copy of the whole sheet (#729).
#[must_use]
pub(crate) fn is_dialog_backdrop(placement: &ScenePlacement) -> bool {
    placement
        .sprite
        .as_deref()
        .is_some_and(|sprite| sprite.starts_with("Dialog_"))
}

/// Paint one dialog placement at its dialog-relative origin: the
/// backdrop (`Dialog_*.ojs`) at its absolute anchor (its anchor
/// *is* the dialog origin, painted at view `(0,0)`); every child
/// one frame per distinct record anchor — the first layer at each
/// anchor — at `origin + local`, where `local` is the record anchor
/// when the record carries one and the placement's `.bnd` rect
/// origin when the record anchors at `(0,0)` (the pinned bound-body
/// rule applied dialog-relative, `docs/geometry-audit.md` § Options
/// dialog). One frame per anchor is the exe image control's
/// one-frame-per-view with the loader default frame 0; multi-anchor
/// sheets (`OptionKey_Select`'s seven key indicators,
/// `Option_Sound_BarEdge`'s slider edges) keep every authored
/// position until per-control frame selection lands (OPEN).
fn paint_dialog_figure(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figure: &AvatarFigure,
    origin: (i32, i32),
) {
    if is_dialog_backdrop(placement) {
        figure.paint_scaled_anchors(frame, 0, 0, frame.w, frame.h);
        return;
    }
    let mut painted_anchors: Vec<(i32, i32)> = Vec::new();
    for index in 0..figure.len() {
        let Some(layer) = figure.layer(index) else {
            continue;
        };
        // A `(0,0)` record carries no position: the `.bnd` rect is
        // its only placement data (verified bound-body rule).
        let at = dialog_layer_origin(placement, layer.dx, layer.dy);
        if painted_anchors.contains(&at) {
            continue;
        }
        painted_anchors.push(at);
        paint_dialog_layer(frame, placement, figure, origin, index);
    }
}

/// One record's dialog-relative origin: zero-anchor bodies land at
/// their `.bnd` rect, authored anchors keep their absolute position.
fn dialog_layer_origin(placement: &ScenePlacement, dx: i32, dy: i32) -> (i32, i32) {
    if dx == 0 && dy == 0 {
        (placement.rect.x + dx, placement.rect.y + dy)
    } else {
        (dx, dy)
    }
}

/// Repaint one record of a staged dialog control at the same origin
/// the base dialog pass uses. The frame-state pass calls this so a
/// hover/active record lands exactly over its rest face.
fn paint_dialog_layer(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figure: &AvatarFigure,
    origin: (i32, i32),
    index: usize,
) {
    paint_dialog_layer_at_anchor(frame, placement, figure, origin, index, index);
}

/// One staged record painted at another staged record's anchor.
/// The caller uses this to cover the footprint of a face whose
/// archive anchor differs from the base face's anchor.
fn paint_dialog_layer_at_anchor(
    frame: &mut Frame<'_>,
    placement: &ScenePlacement,
    figure: &AvatarFigure,
    origin: (i32, i32),
    index: usize,
    anchor_index: usize,
) {
    let (Some(layer), Some(anchor)) = (figure.layer(index), figure.layer(anchor_index)) else {
        return;
    };
    let at = dialog_layer_origin(placement, anchor.dx, anchor.dy);
    let (view_x, view_y) = anchor_in_frame(origin.0 + at.0, origin.1 + at.1, frame.w, frame.h);
    blit_rgb555(
        frame,
        &layer.image.view(),
        view_x,
        view_y,
        Some(layer.colorkey),
    );
}

/// Paint a staged dialog scene dialog-relative (options and
/// quit-confirm sheets; one back-to-front pass in file order).
/// Placements without a loaded figure paint nothing.
pub fn paint_dialog_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    paint_dialog_scene_filtered(frame, art, |_| true);
}

/// Bodies whose rest face is already baked into the parent sheet.
/// The qualified 7.55 rest frames show the sheet's gray OK/Options
/// plates and the blue 3-key plate, not the record-0 bodies staged
/// from `Interface1.opi` (`~/scratch/paired-shots-v3` channel-popup and
/// options-key pairs; `docs/dialog-frame-states.md` pins rest as the
/// staged stack). The MESSAGE sheet follows the same rule (#938):
/// `FUN_0x4346f0` writes only placements, and both button bodies hold
/// blue records, so their gray rest face is the `Dialog_Question1.ojs`
/// plate. Hover/press still repaint the body through
/// `ScreenGame::paint_dialog_frame_states`; geometry stays with the
/// parsed `.bnd` placements.
const BAKED_PLATE_BODIES: [&str; 8] = [
    "Info_OK.ojs",
    "OptionKey_Btn_3Key.ojs",
    "OptionKey_Btn_7Key.ojs",
    "Option_Default.ojs",
    "Option_Save.ojs",
    "Option_Exit.ojs",
    "Question1_OK.ojs",
    "Question1_Cancel.ojs",
];

/// Paint a dialog sheet, withholding placements the caller keeps inert.
/// A withheld body leaves the sheet's own baked art visible: the create-room
/// key plates use that face while the exe's mode sentinel is unset (#844).
pub fn paint_dialog_scene_filtered(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    keep: impl Fn(&ScenePlacement) -> bool,
) {
    if !art.has_figures() {
        return;
    }
    let origin = dialog_origin(&art.placements);
    for placement in &art.placements {
        if !keep(placement) {
            continue;
        }
        let Some(name) = placement.sprite.as_ref() else {
            continue;
        };
        if BAKED_PLATE_BODIES.contains(&name.as_str()) {
            continue;
        }
        let Some(figure) = art.figures.get(name) else {
            continue;
        };
        if figure.is_empty() {
            continue;
        }
        paint_dialog_figure(frame, placement, figure, origin);
    }
}

/// Dialog-relative hit for `code`: origin + parsed `.bnd` rect,
/// right/bottom exclusive (`rect_hit`). `false` without a staged
/// placement (un-staged dialogs hit nothing — Escape still
/// closes them).
#[must_use]
pub(crate) fn dialog_rect_hit(placements: &[ScenePlacement], code: u32, x: i32, y: i32) -> bool {
    let (origin_x, origin_y) = dialog_origin(placements);
    placements
        .iter()
        .find(|placement| placement.code == code && !is_dialog_backdrop(placement))
        .is_some_and(|placement| {
            rect_hit(
                &Rect {
                    x: placement.rect.x + origin_x,
                    y: placement.rect.y + origin_y,
                    w: placement.rect.w,
                    h: placement.rect.h,
                },
                x,
                y,
            )
        })
}

/// Dialog control under a native pointer, using the same staged
/// rects as the click path. Code-less backdrop/cover placements are
/// skipped, and the smallest containing rect wins, so a button wins
/// over the sheet frame beneath it. `None` off every control.
#[must_use]
pub(crate) fn dialog_control_code(placements: &[ScenePlacement], x: i32, y: i32) -> Option<u32> {
    let (origin_x, origin_y) = dialog_origin(placements);
    placements
        .iter()
        .filter(|placed| placed.code != 0 && !is_dialog_backdrop(placed))
        .filter(|placed| {
            rect_hit(
                &Rect {
                    x: placed.rect.x + origin_x,
                    y: placed.rect.y + origin_y,
                    w: placed.rect.w,
                    h: placed.rect.h,
                },
                x,
                y,
            )
        })
        .min_by_key(|placed| placed.rect.w.saturating_mul(placed.rect.h))
        .map(|placed| placed.code)
}

/// Repaint one staged dialog control's chosen frame over its rest
/// face. The sheet frame is scenery and never repaints here: its
/// record is already absolute, so adding the dialog origin would
/// paint a second copy of the whole sheet (#729). The caller owns
/// the state choice; a missing placement, figure, or record paints
/// nothing (ADR-0003).
pub(crate) fn paint_dialog_frame_record(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    code: u32,
    record: usize,
) {
    let Some((placed, figure, origin)) = dialog_frame_parts(art, code) else {
        return;
    };
    paint_dialog_layer(frame, placed, figure, origin, record);
}

/// Repaint one staged control record at another record's anchor.
/// The 7.55 create-room mode bodies author their selected face one
/// pixel above the base face; painting the base at that anchor first
/// erases the whole stale face before its own anchored repaint.
/// Every anchor comes from staged records, never a pixel literal.
pub(crate) fn paint_dialog_frame_record_at_anchor_of(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    code: u32,
    record: usize,
    anchor_record: usize,
) {
    let Some((placed, figure, origin)) = dialog_frame_parts(art, code) else {
        return;
    };
    paint_dialog_layer_at_anchor(frame, placed, figure, origin, record, anchor_record);
}

/// Placement, body, and sheet origin of one staged dialog control.
/// The sheet frame is scenery and never repaints here: its record is
/// already absolute, so adding the dialog origin would paint a
/// second copy of the whole sheet (#729).
fn dialog_frame_parts(
    art: &SceneArt,
    code: u32,
) -> Option<(&ScenePlacement, &AvatarFigure, (i32, i32))> {
    let placed = art.placements.iter().find(|placed| placed.code == code)?;
    if is_dialog_backdrop(placed) {
        return None;
    }
    let sprite = placed.sprite.as_deref()?;
    let figure = art.figures.get(sprite)?;
    Some((placed, figure, dialog_origin(&art.placements)))
}

/// Repaint one record on a parent-relative staged sheet.
///
/// `stage_at_parent_origin` has already moved the `.bnd` child to a canvas
/// rect, so a zero-anchor bound body paints at that rect. A nonzero record
/// anchor stays local to the sheet frame and still needs the parsed frame
/// origin. The two cases keep a staged child from being offset twice.
pub fn paint_messenger_frame_record(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    code: u32,
    record: usize,
) {
    let Some(placed) = art.placements.iter().find(|placed| placed.code == code) else {
        return;
    };
    if is_dialog_backdrop(placed) {
        return;
    }
    let Some(sprite) = placed.sprite.as_deref() else {
        return;
    };
    let Some(figure) = art.figures.get(sprite) else {
        return;
    };
    let Some(layer) = figure.layer(record) else {
        return;
    };
    let origin = if layer.dx == 0 && layer.dy == 0 {
        (0, 0)
    } else {
        dialog_origin(&art.placements)
    };
    paint_dialog_layer(frame, placed, figure, origin, record);
}

/// Archive-less fallback for the one-OK information sheet: the
/// `Dialog_Information.bnd` back origin (241,206) plus its OK rect
/// (102,150,117,26), the geometry the exe paints and the qualified
/// capture shows (`docs/bnd-semantics.md:41`, #244). Used only when
/// the Interface archive is absent; staged boots read the `.bnd`
/// placement directly.
#[must_use]
pub(crate) fn information_ok_fallback_rect() -> Rect {
    Rect {
        x: 343,
        y: 356,
        w: 117,
        h: 26,
    }
}

/// Archive-less CANCEL box beside the one-OK Information sheet's
/// button. Same sheet family, so the confirm sheet keeps the OK
/// hit rect the qualified Information packs pin (#244).
#[must_use]
pub(crate) fn information_cancel_fallback_rect() -> Rect {
    let ok = information_ok_fallback_rect();
    Rect {
        x: ok.x + i32::try_from(ok.w).unwrap_or(0) + 10,
        y: ok.y,
        w: ok.w,
        h: ok.h,
    }
}

/// Frame-centered quit-fallback boxes (native 800x600 space): used
/// only while no dialog art is staged, so the popup still shows
/// and clicks without archives. Returns (message, OK, CANCEL)
/// rects as fractions of the canvas — never staged geometry.
/// With [`paint_dialog_scene`] present the bnd rects own layout
/// (see [`dialog_rect_hit`]) and these boxes never paint.
#[must_use]
pub(crate) fn quit_fallback_rects() -> (Rect, Rect, Rect) {
    const W: i32 = 800;
    const H: i32 = 600;
    let message = Rect {
        x: W / 4,
        y: H / 3,
        w: (W / 2) as u32,
        h: 26,
    };
    let ok = Rect {
        x: W / 2 - 90,
        y: H / 3 + 40,
        w: 80,
        h: 26,
    };
    let cancel = Rect {
        x: W / 2 + 10,
        y: H / 3 + 40,
        w: 80,
        h: 26,
    };
    (message, ok, cancel)
}

/// One `Dialog_Question1` MESSAGE sheet: staged art when installed,
/// frame-centred OK/CANCEL plate otherwise. The caller owns the caption;
/// geometry always comes from the parsed `.bnd` rects (ADR-0003).
fn paint_question1_message(frame: &mut Frame<'_>, art: Option<&SceneArt>, caption: &[u8]) {
    if let Some(art) = art.filter(|art| art.has_figures()) {
        paint_question1_staged(frame, art, caption);
    } else {
        paint_question1_fallback(frame, caption);
    }
}

/// Staged sheet: bodies from the parsed `.bnd`, caption in its TEXT rect.
fn paint_question1_staged(frame: &mut Frame<'_>, art: &SceneArt, caption: &[u8]) {
    paint_dialog_scene(frame, art);
    let origin = dialog_origin(&art.placements);
    for placement in &art.placements {
        if placement.code == super::side::QUIT_MESSAGE_CODE {
            paint_message_caption_in_rect(
                frame,
                Rect {
                    x: placement.rect.x + origin.0,
                    y: placement.rect.y + origin.1,
                    w: placement.rect.w,
                    h: placement.rect.h,
                },
                &o2jam_text::decode_cp949_lossy(caption),
                CP949_INK,
            );
        }
    }
}

/// Archive-less stand-in: a scrim, the caption, and two button words.
fn paint_question1_fallback(frame: &mut Frame<'_>, caption: &[u8]) {
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: frame.w as u32,
            h: frame.h as u32,
        },
        scrim_color(),
    );
    let (message, ok, cancel) = quit_fallback_rects();
    paint_cp949_into_rect(frame, message, caption);
    for (button, word) in [(ok, "OK"), (cancel, "CANCEL")] {
        let scaled = scale_rect_to_frame(&button, frame.w, frame.h);
        fill_rect(frame, &scaled, CHANNEL_ENTER_TONE);
        draw_text(frame, scaled.x + 8, scaled.y + 8, word, art_title_ink());
    }
}

/// Quit-confirm popup over the bare board: OK quits, CANCEL closes
/// (clicks route through [`dialog_rect_hit`] or [`quit_fallback_rects`]).
pub fn paint_planet_quit(frame: &mut Frame<'_>, art: Option<&SceneArt>) {
    paint_question1_message(frame, art, QUIT_MESSAGE_CP949);
}

/// Item-shop BUYITEM confirmation (#867): the same native MESSAGE sheet
/// with the pinned O2CASH-shortfall caption, over the shop board.
pub fn paint_shop_charge_confirm(frame: &mut Frame<'_>, art: Option<&SceneArt>) {
    paint_question1_message(frame, art, SHOP_O2CASH_SHORTFALL_CP949);
}

/// Relay-unavailable message box over the planet board: the staged
/// `Dialog_Information` sheet with the exe string in its TEXT rect,
/// or the same frame-centred fallback plate the other one-button
/// notices use. Geometry comes from `Dialog_Information.bnd`
/// rects when art is staged (code-ordinal pairing,
/// `docs/bnd-semantics.md:41`); the fallback is only the
/// archive-less stand-in. There is exactly one `OK` (#244 capture).
pub fn paint_relay_down_dialog(frame: &mut Frame<'_>, art: Option<&SceneArt>) {
    paint_information_message(frame, art, RELAY_DOWN_MESSAGE_CP949);
}

/// Room-state no-response sheet (#367): the very same staged
/// Information block, with the exe's generic fuse string in TEXT
/// `0x21400003`. Geometry still comes only from the parsed `.bnd`
/// rects (owner rule `f38f3`); the fallback is archive-less only.
pub fn paint_room_no_response_dialog(frame: &mut Frame<'_>, art: Option<&SceneArt>) {
    paint_information_message(frame, art, NO_RESPONSE_MESSAGE_CP949);
}

/// One-OK MyRoom notice: the same staged `Dialog_Information` sheet
/// with a UTF-8 caption, for strings read from the 7.55 byte tables
/// and decoded before staging (the executable consumes cp949, our
/// font rasterizer takes UTF-8). Geometry still comes only from the
/// staged `.bnd` rects or the archive-less fallback; no notice text
/// is invented here.
pub fn paint_myroom_information(frame: &mut Frame<'_>, art: Option<&SceneArt>, message: &str) {
    if let Some(art) = art.filter(|art| art.has_figures()) {
        paint_dialog_scene(frame, art);
        let (origin_x, origin_y) = dialog_origin(&art.placements);
        for placement in &art.placements {
            if placement.code == INFORMATION_MESSAGE_CODE {
                paint_text_into_rect(
                    frame,
                    Rect {
                        x: placement.rect.x + origin_x,
                        y: placement.rect.y + origin_y,
                        w: placement.rect.w,
                        h: placement.rect.h,
                    },
                    message,
                    CP949_INK,
                );
            }
        }
        return;
    }
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: frame.w as u32,
            h: frame.h as u32,
        },
        scrim_color(),
    );
    let (text, _, _) = quit_fallback_rects();
    paint_text_into_rect(frame, text, message, CP949_INK);
    let ok = scale_rect_to_frame(&information_ok_fallback_rect(), frame.w, frame.h);
    fill_rect(frame, &ok, CHANNEL_ENTER_TONE);
    draw_text(frame, ok.x + 8, ok.y + 8, "OK", art_title_ink());
}

/// One MyRoom confirm sheet (OK/CANCEL): the staged dialog's `.bnd`
/// rects for text and buttons, or the archive-less fallback boxes.
/// The caller owns the UTF-8 caption; no geometry or copy is invented
/// by the painter.
pub fn paint_myroom_confirm(frame: &mut Frame<'_>, art: Option<&SceneArt>, message: &str) {
    if let Some(art) = art.filter(|art| art.has_figures()) {
        paint_dialog_scene(frame, art);
        let (origin_x, origin_y) = dialog_origin(&art.placements);
        for placed in &art.placements {
            let rect = Rect {
                x: placed.rect.x + origin_x,
                y: placed.rect.y + origin_y,
                w: placed.rect.w,
                h: placed.rect.h,
            };
            match placed.code {
                INFORMATION_MESSAGE_CODE => {
                    paint_text_into_rect(frame, rect, message, CP949_INK);
                }
                INFORMATION_OK_CODE => {
                    draw_text(frame, rect.x + 8, rect.y + 8, "OK", art_title_ink());
                }
                _ => {}
            }
        }
        // The Information sheet stamps CANCEL nowhere; the fallback
        // box stays the only archive-less geometry for the second arm.
        draw_text(
            frame,
            information_cancel_fallback_rect().x + 8,
            information_cancel_fallback_rect().y + 8,
            "CANCEL",
            art_title_ink(),
        );
        return;
    }
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: frame.w as u32,
            h: frame.h as u32,
        },
        scrim_color(),
    );
    let (text, _, _) = quit_fallback_rects();
    paint_text_into_rect(frame, text, message, CP949_INK);
    for (rect, word) in [
        (information_ok_fallback_rect(), "OK"),
        (information_cancel_fallback_rect(), "CANCEL"),
    ] {
        let scaled = scale_rect_to_frame(&rect, frame.w, frame.h);
        fill_rect(frame, &scaled, CHANNEL_ENTER_TONE);
        draw_text(frame, scaled.x + 8, scaled.y + 8, word, art_title_ink());
    }
}

/// One-OK sheet painter shared by the relay notice, the Room fuse, and
/// the refused channel enter (#717). The caller owns the exact CP949
/// bytes; geometry comes only from the staged `.bnd` rects or the
/// archive-less fallback rect.
pub fn paint_information_message(frame: &mut Frame<'_>, art: Option<&SceneArt>, message: &[u8]) {
    if let Some(art) = art.filter(|art| art.has_figures()) {
        paint_dialog_scene(frame, art);
        let origin = dialog_origin(&art.placements);
        for placement in &art.placements {
            if placement.code == INFORMATION_MESSAGE_CODE {
                paint_cp949_into_rect(
                    frame,
                    Rect {
                        x: placement.rect.x + origin.0,
                        y: placement.rect.y + origin.1,
                        w: placement.rect.w,
                        h: placement.rect.h,
                    },
                    message,
                );
            }
        }
        return;
    }
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: frame.w as u32,
            h: frame.h as u32,
        },
        scrim_color(),
    );
    let (text, _, _) = quit_fallback_rects();
    paint_cp949_into_rect(frame, text, message);
    let scaled = scale_rect_to_frame(&information_ok_fallback_rect(), frame.w, frame.h);
    fill_rect(frame, &scaled, CHANNEL_ENTER_TONE);
    draw_text(frame, scaled.x + 8, scaled.y + 8, "OK", art_title_ink());
}

/// Options dialog over the bare board: the staged
/// `Dialog_Option_*` sheet for `tab` (dialog-relative), or a
/// title-only fallback without staged art. The gear's first tab is
/// pinned to Sound by the b3/b4 captures (see
/// [`PLANET_OPTION_FIRST_TAB`](super::side::PLANET_OPTION_FIRST_TAB)).
/// Escape (and the tab's `Option_Exit` button, when staged) closes it.
pub fn paint_planet_options(
    frame: &mut Frame<'_>,
    art: Option<&SceneArt>,
    tab: PlanetOptionTab,
    settings: &OptionsSettings,
) {
    if let Some(art) = art.filter(|art| art.has_figures()) {
        if tab == PlanetOptionTab::Game {
            options_game::paint_game_tab(frame, art, settings);
            return;
        }
        paint_dialog_scene(frame, art);
        return;
    }
    let _ = tab;
    let title = "OPTIONS";
    let (x, y) = title_origin(frame, title);
    let text_w = text_width(title) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 4,
            y: y - 2,
            w: (text_w + 8).max(0) as u32,
            h: 9,
        },
        scrim_color(),
    );
    draw_text(frame, x, y, title, art_title_ink());
}

/// Reference-measured bare-board geometry (planet scene, native px;
/// 2026-11-09 capture `cap-1.png`, quiet server). Documented for
/// future wiring — no hit-testing binds these yet (only the pinning
/// test reads them, so the dead-code allowance rides along).
/// Promo pager nav cluster (left arrow / play-stop / right arrow
/// cells ride inside).
pub const PLANET_BOARD_PROMO_NAV_RECT: Rect = Rect {
    x: 345,
    y: 509,
    w: 107,
    h: 27,
};
/// OPTION disc (gear, exe art 40x40) origin.
pub const PLANET_BOARD_OPTION_ORIGIN: (i32, i32) = (712, 562);
/// EXIT disc (power, exe art 40x40) origin.
pub const PLANET_BOARD_EXIT_ORIGIN: (i32, i32) = (753, 562);
/// MENU back-arrow circle (center ~(145,578)).
pub const PLANET_BOARD_MENU_BACK_RECT: Rect = Rect {
    x: 130,
    y: 562,
    w: 31,
    h: 32,
};

/// Figure-less fallback row label, in the exe's pinned reading
/// (claim D25, `docs/channel-fill.md`): `Ch. ` plus the one-based
/// wire `channel_id` as two decimal digits (ids past 98 wrap to
/// their low two digits, matching the staged digit wells), with
/// ` / FULL` on rows the pinned FULL rule marks. The exe paints no
/// row text — the staged path shows baked `Ch.` art plus digit
/// frames instead — so this string is the labeled no-archive
/// stand-in only, and it deliberately names the channel id alone
/// (never the gateway id or population).
#[must_use]
fn channel_row_text(channel: &ChannelState) -> String {
    let shown = (u32::from(channel.channel_id).saturating_add(1)) % 100;
    let mut text = format!("Ch. {shown:02}");
    if channel_row_full(channel) {
        text.push_str(" / FULL");
    }
    text
}

/// Channel rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene). Only the stored-top viewport
/// paints (no pager: the 7.55 dialog has none — see
/// [`channel_viewport_from_top`]). `art` selects
/// staged enter-button art over the procedural plates in the art path
/// (`None` is the flat fallback).
fn paint_channel_rows(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    top: usize,
    highlight: usize,
    art: Option<&SceneArt>,
    hovered_row: Option<usize>,
) {
    let (top, shown) = channel_viewport_from_top(top, channels.len());
    if !channel_dialog_sheet_staged(art) {
        paint_channel_scrim(frame, shown);
    }
    let visible: Vec<(usize, &ChannelState)> =
        channels.iter().skip(top).take(shown).enumerate().collect();
    let w = i32::try_from(frame.w).unwrap_or(0);
    let h = i32::try_from(frame.h).unwrap_or(0);
    let text_x = w / 4;
    if art.is_none() {
        paint_channel_buttons(frame, shown);
    }
    for (slot, channel) in &visible {
        let row = channel_row_top(*slot, h);
        // Staged CH_Back replaces the procedural pill/track chrome;
        // no bar figure keeps today's stand-ins.
        let staged_bar = art.is_some_and(|art| paint_channel_bar_art(frame, *slot, channel, art));
        if !staged_bar {
            paint_channel_row_chrome(frame, *slot);
        }
        // Staged digit plates name the row in art (tens/ones frames
        // at their bnd slots); without them the ASCII row string
        // stays the labeled fallback. The exe paints no row text —
        // live captures show digits, segments, and the enter face
        // only — so staged rows carry no `draw_text` overlay.
        let digits_staged = art.is_some_and(|art| {
            art.figures
                .get(CHANNEL_DIGIT_SPRITE)
                .is_some_and(|figure| !figure.is_empty())
        });
        if !digits_staged {
            let entry = channel_row_text(channel);
            let ink = if channel.active {
                pack_xrgb(0xFF, 0xFF, 0xFF)
            } else {
                pack_xrgb(0x70, 0x70, 0x70)
            };
            draw_text(frame, text_x, row, &entry, ink);
        }
        // The face rides above the bar (opaque face well, see
        // `paint_channel_face`); the plates mark the same faces
        // where no enter art staged.
        if let Some(art) = art {
            paint_channel_face(frame, *slot, channel, art);
        }
        // The select bar follows the row under the pointer (the exe
        // arms it on mouse motion alone, live-confirmed 2026-11-09);
        // with no hovered row it stays on the keyboard highlight.
        let bar_row = hovered_row.unwrap_or(highlight);
        if top + *slot == bar_row {
            paint_channel_highlight(frame, *slot, text_x, row, art);
        }
    }
    if let Some(art) = art {
        paint_channel_scroll_art(frame, art, top, channels.len());
    }
}

/// Row of list entry `index` (channel and room lists share the shape).
fn index_as_row(index: usize, h: i32) -> i32 {
    h / 4 + i32::try_from(index).unwrap_or(i32::MAX) * CHANNEL_ROW_H
}

/// True while native pixel (`x`, `y`) sits inside `rect`
/// (right/bottom edges exclusive; zero-area rects never hit).
#[must_use]
pub(crate) fn rect_hit(rect: &Rect, x: i32, y: i32) -> bool {
    let right = i64::from(rect.x).checked_add(i64::from(rect.w));
    let bottom = i64::from(rect.y).checked_add(i64::from(rect.h));
    let inside_x =
        i64::from(x) >= i64::from(rect.x) && right.is_some_and(|edge| i64::from(x) < edge);
    let inside_y =
        i64::from(y) >= i64::from(rect.y) && bottom.is_some_and(|edge| i64::from(y) < edge);
    inside_x && inside_y
}

/// True while staged art binds `id` and its `.bnd` rect contains
/// native pixel (`x`, `y`) — verbatim-rect hit-testing for staged
/// buttons, where the exe leaves untouched icons alone.
#[must_use]
pub(crate) fn staged_rect_hit(placements: &[ScenePlacement], id: &str, x: i32, y: i32) -> bool {
    placements
        .iter()
        .find(|placed| placed.id == id)
        .is_some_and(|placed| rect_hit(&placed.rect, x, y))
}

/// Row-band hit for the lobby lists: the inverse of
/// [`index_as_row`] — row `index` spans
/// `[h/4 + 8·index, h/4 + 8·index + 8)` inside the scrim strip
/// (`[w/4 - 8, w/4 + w/2 + 16)`, mirroring [`paint_list_scrim`]).
/// Native pixels in, list index out (`None` outside every band).
#[must_use]
pub(crate) fn list_row_hit(
    frame_w: i32,
    frame_h: i32,
    row_count: usize,
    x: i32,
    y: i32,
) -> Option<usize> {
    if x < frame_w / 4 - 8 || x >= frame_w / 4 + frame_w / 2 + 16 {
        return None;
    }
    let offset = y.checked_sub(frame_h / 4)?;
    if offset < 0 {
        return None;
    }
    let index = usize::try_from(offset / CHANNEL_ROW_H).ok()?;
    (index < row_count).then_some(index)
}

/// Amber highlight cursor left of a list row.
fn paint_cursor(frame: &mut Frame<'_>, text_x: i32, y: i32) {
    fill_rect(
        frame,
        &Rect {
            x: text_x - 6,
            y,
            w: 3,
            h: 5,
        },
        pack_xrgb(0xFF, 0xD0, 0x20),
    );
}

/// Native geometry of the hover highlight bar (`Dialog_ChannelSelectBar`,
/// 348x44 — the size rides the staged figure, so only the left edge
/// is a constant): left edge and size pinned by the live measurement
/// (bar border (216,row_top)-(561,row_top+41), live 2026-11-09); the
/// row top comes from the exe-pinned [`CHANNEL_ROW_RECTS`] table.
const CHANNEL_SELECT_BAR_X: i32 = 216;

/// Channel-list highlight: the code-mounted select-bar body at the
/// hovered row's rect (216,row_top,348x44, scaled to the frame) while
/// the figure stages — the exe paints the bar on the hovered row
/// (hover-driven, live-confirmed 2026-11-09); the amber cursor
/// stand-in paints otherwise (no figure staged).
fn paint_channel_highlight(
    frame: &mut Frame<'_>,
    slot: usize,
    text_x: i32,
    row: i32,
    art: Option<&SceneArt>,
) {
    if let Some(figure) = art
        .and_then(|art| art.figures.get(DIALOG_BAR_SPRITE))
        .filter(|figure| !figure.is_empty())
    {
        let row_top = CHANNEL_ROW_RECTS[slot.min(CHANNEL_SLOT_COUNT - 1)].1;
        let (dx, dy) = anchor_in_frame(CHANNEL_SELECT_BAR_X, row_top, frame.w, frame.h);
        figure.paint_scaled_anchors(frame, dx, dy, frame.w, frame.h);
        return;
    }
    paint_cursor(frame, text_x, row);
}

/// Paint the online channel list with a highlight cursor.
pub(crate) fn paint_channel_list(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    top: usize,
    highlight: usize,
) {
    fill_background(frame, Screen::Channel);
    paint_title(frame, "CHANNEL");
    paint_channel_rows(frame, channels, top, highlight, None, None);
}

/// Paint the staged channel dialog body ([`CHANNEL_DIALOG_SPRITE`]) at
/// its record anchor (0,0), behind the live list ink. Reference-
/// measured mount (panel band approx x185-615 y150-464 inside the
/// 800x466 body; see the constant's doc). Returns `true` while a body
/// painted; `false` without one (no-fill corner untouched).
fn paint_channel_dialog_body(frame: &mut Frame<'_>, art: &SceneArt) -> bool {
    let Some(figure) = art.figures.get(CHANNEL_DIALOG_SPRITE) else {
        return false;
    };
    if figure.is_empty() || frame.w == 0 || frame.h == 0 {
        return false;
    }
    figure.paint_scaled_anchors(frame, 0, 0, frame.w, frame.h);
    true
}

/// Paint staged channel art with the live list and cursor on top,
/// over the planet board beneath. Verified first screen (2026-11-09
/// captures, `docs/planet.md` "Real board structure" + `docs/lifecycle.md`
/// §6 correction): the channel dialog (opaque 800x466 sheet covering
/// y0-468) opens OVER the planet board (launcher `My_channel`/
/// `My_planet` preselection), so `board` — the bound `State_Planet`
/// block placements when staged: backdrop + emblem buttons + promo +
/// MENU bar — paints first and its MENU bar ink stays visible in the
/// bottom strip (y>468) under the dialog. No-fill corner: without
/// staged figures (board or dialog) nothing paints and the frame
/// starts from device black (the live loop reuses one backbuffer
/// across screens while the exe rebuilds every frame); pixels no art
/// covers stay untouched, and without board art the strip below the
/// dialog stays byte-identical to the art-less frame. The board's
/// promotion image follows `promo_index`: the exe's image control
/// mounts one frame, so the flanks show that frame, not all three
/// stacked raw layers.
pub fn paint_channel_scene(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    top: usize,
    highlight: usize,
    art: &SceneArt,
    board: Option<StagedPlanetBoard<'_>>,
    hovered_row: Option<usize>,
) {
    if !art.has_figures() && !board.is_some_and(|board| board.art.has_figures()) {
        frame.buf.fill(0);
    }
    if let Some(board) = board.filter(|board| board.art.has_figures()) {
        paint_planet_board(frame, board.art, board.promo);
    }
    paint_channel_dialog_body(frame, art);
    art.paint_backdrop(frame, Screen::Channel);
    // An art-less channel scene has no label ink of its own, so the
    // shell title keeps it readable (the pinned no-fill test); staged
    // art never gets the stamp (#36).
    if !art.has_figures() && !board.is_some_and(|board| board.art.has_figures()) {
        paint_title_over_art(frame, Screen::Channel);
    }
    paint_channel_rows(frame, channels, top, highlight, Some(art), hovered_row);
}

/// Title tucked just inside the card's top edge.
fn paint_title(frame: &mut Frame<'_>, title: &str) {
    if title.is_empty() {
        return;
    }
    let (x, y) = title_origin(frame, title);
    draw_text(frame, x, y, title, title_color());
}

/// Staged lobby art behind an empty list: verbatim backdrop plus
/// a readable title (no flat card).
pub fn paint_lobby_backdrop(frame: &mut Frame<'_>, screen: Screen, art: &SceneArt) {
    art.paint_backdrop(frame, screen);
    paint_title_over_art(frame, screen);
}

/// Lobby backdrop behind a per-placement filter (the room empty-list
/// arm: staged art minus the exe-hidden bundled chrome, which the
/// shared unfiltered backdrop would paint unconditionally).
pub fn paint_lobby_backdrop_live(
    frame: &mut Frame<'_>,
    screen: Screen,
    art: &SceneArt,
    keep: impl Fn(&ScenePlacement) -> bool,
) {
    art.paint_backdrop_live(frame, screen, keep);
    paint_title_over_art(frame, screen);
}

/// Client version stamped on the splash (matches the 7.55 target).
const SPLASH_VERSION: u32 = 755;
/// Splash version color (white on the logo card).
fn version_color() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// Paint the splash (logo) scene.
fn paint_avi(frame: &mut Frame<'_>) {
    paint_card(
        frame,
        pack_xrgb(0x08, 0x08, 0x10),
        pack_xrgb(0x20, 0x60, 0xC0),
    );
    let text_w = number_width(SPLASH_VERSION) as i32;
    draw_number(
        frame,
        frame.w as i32 / 2 - text_w / 2,
        frame.h as i32 / 2 - 2,
        SPLASH_VERSION,
        version_color(),
    );
}

/// Paint staged splash art (`O2mediaLogo.ojs`): the real logo
/// replaces the card plus version stamp.
pub fn paint_avi_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Avi);
}

/// Paint a lobby scene (channel / room / waiting share the shape).
fn paint_lobby(frame: &mut Frame<'_>, background: u32) {
    paint_card(frame, background, pack_xrgb(0xD0, 0xD0, 0xD0));
}

/// Loading gate state staged while the song table walks (exe
/// `0x4e47a0`, file `0xe47a0`, ev `68d920c2…`): the song-table
/// fraction plus the song title/artist being loaded. The exe sets
/// `Trebuchet MS`/`Arial` fonts there; the shell micro-font stands
/// in (typography gap). Track and caption rects below stay OPEN placeholders: `MusicLoading.ojs` is one 800x600 body at (0,0)
/// with no bar/caption sub-rect, the `0x4e47a0` dossier carries
/// fonts plus the overlay mount plus the song-table walk with no
/// literal bar/caption rect, and adjacent bar bodies belong to
/// other dialogs — so only the fraction and the caption are live.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LoadingStatus {
    /// Song-table walk fraction (`0.0`..=`1.0`, clamped at paint).
    pub progress: f32,
    /// Song title being loaded (OJN header / `OJNList.dat` record).
    pub title: String,
    /// Song artist being loaded.
    pub artist: String,
}

impl LoadingStatus {
    /// Walk fraction clamped to `0.0`..=`1.0` (NaN saturates to
    /// `0.0`): never `==`-compares floats (PERF-25).
    #[must_use]
    pub fn clamped_progress(&self) -> f32 {
        if self.progress.is_nan() {
            0.0
        } else {
            self.progress.clamp(0.0, 1.0)
        }
    }

    /// `TITLE - ARTIST` caption (`None` while both are blank:
    /// nothing paints rather than a wrong line).
    #[must_use]
    pub fn caption(&self) -> Option<String> {
        let title = self.title.trim();
        let artist = self.artist.trim();
        match (title.is_empty(), artist.is_empty()) {
            (true, true) => None,
            (false, true) => Some(title.to_owned()),
            (true, false) => Some(artist.to_owned()),
            (false, false) => Some(format!("{title} - {artist}")),
        }
    }
}

/// Width of the live fill inside a `track_w` track at `progress`:
/// clamped, then narrowed via `TryFrom`, never `as` (PERF-07).
#[must_use]
fn loading_fill_width(track_w: u32, progress: f32) -> u32 {
    let clamped = if progress.is_nan() {
        0.0
    } else {
        progress.clamp(0.0, 1.0)
    };
    let scaled = f64::from(track_w) * f64::from(clamped);
    let rounded = scaled.round() as u64;
    u32::try_from(rounded).unwrap_or(u32::MAX).min(track_w)
}

/// Track origin of the OPEN placeholder bar: centered `w / 4,
/// h / 2 - 8` with no exe-grounded rect. Measured 7.55
/// exclusion table (read-only sprite-55 walks, 2026-11-08):
/// `MusicLoading.ojs` is one 800x600 layer at (0,0) (`colorkey 0x0`,
/// raw 960000 B) — no sub-rect; `Loading_Sign.ojs` is three 134x37
/// frames at (0,0) (`colorkey 0x7D5F`); `Wait_Downloding_Bar.ojs`
/// is one 65x7 layer at (0,0); `Payment_BackLoading.ojs` is seven
/// 119x23 layers at (619,529); `AlbumLoading_Panel.ojs` is one
/// 197x179 layer at (410,210); the OJN cover is a full-bleed
/// 800x600 photo with no bead-pill rows or bar shell. `0x4e47a0`
/// names no rect (fonts plus overlay mount plus song-table walk
/// only); bnd-audit 35 (no `.bnd` for loading).
fn loading_track_origin(frame: &Frame<'_>) -> (i32, i32) {
    (frame.w as i32 / 4, frame.h as i32 / 2 - 8)
}

/// Inner width of the OPEN placeholder track on this frame
/// (`w / 2`: centered placeholder, no exe-grounded width — none
/// of the measured bar bodies above is the loading fill).
fn loading_track_width(frame: &Frame<'_>) -> u32 {
    u32::try_from(frame.w as i32 / 2).unwrap_or(0)
}

/// Song-caption origin: one row below the OPEN placeholder bar
/// (`w / 4, h / 2 + 12` — no verified caption rect; the cover art
/// carries baked credits, not a caption sub-rect).
fn loading_song_origin(frame: &Frame<'_>) -> (i32, i32) {
    (frame.w as i32 / 4, frame.h as i32 / 2 + 12)
}

/// Visibility gate for the OPEN loading-bar placeholder.
///
/// OPEN (7.55, SHA-256 `83a045…`): the exe's `(200,272,320,28)` crop has
/// no bar at load-complete (`blit-pass/pack/frames/ref/loading.png`) nor
/// mid-load (`no-response/f1-playing-qualified/artifacts/screenshots/
/// loading.png`), while the Rust frame (`live-cover/green/frames/
/// frame-000146-loading.png`) carries the invented green fill. Keep the
/// painter as a tracked element, but do not show it until the pin: real
/// bar art and the visibility rule from `MusicLoading.ojs` — where the
/// exe's indicator is, when it appears, and whether it tracks download
/// progress (the shell models no download, so every fraction is synthetic).
/// OPEN labeling follows `docs/geometry-audit.md`; next step is a rig read
/// of the gate mount plus a capture with a real download in flight.
const LOADING_BAR_VISIBLE: bool = false;

/// Loading progress bar over a dark track (no background fill:
/// shared by the flat fallback and the art scene). The track rect
/// is an OPEN placeholder; only the fill fraction is live. Hidden from
/// the loading frames until [`LOADING_BAR_VISIBLE`] is evidence-pinned.
fn paint_loading_bar(frame: &mut Frame<'_>) {
    paint_loading_bar_with_progress(frame, 1.0);
}

/// Loading bar with the live song-table walk fraction: the track
/// stays put, the green fill spans `progress` of it.
fn paint_loading_bar_with_progress(frame: &mut Frame<'_>, progress: f32) {
    let (track_x, track_y) = loading_track_origin(frame);
    let track_w = loading_track_width(frame);
    fill_rect(
        frame,
        &Rect {
            x: track_x - 2,
            y: track_y - 2,
            w: track_w.saturating_add(4),
            h: 20,
        },
        scrim_color(),
    );
    let fill_w = loading_fill_width(track_w, progress);
    if fill_w > 0 {
        fill_rect(
            frame,
            &Rect {
                x: track_x,
                y: track_y,
                w: fill_w,
                h: 16,
            },
            pack_xrgb(0x30, 0xB0, 0x30),
        );
    }
}

/// Song title/artist line below the bar (OPEN placeholder row):
/// dark scrim plus white ink at the gate display face
/// (`Cell::Loading`, Trebuchet MS 27x65 per `docs/loading-type.md`).
/// Which caption takes the display face is the spec's
/// recommendation, not an exe pin. Blank captions paint nothing.
fn paint_loading_song(frame: &mut Frame<'_>, status: &LoadingStatus) {
    let Some(caption) = status.caption() else {
        return;
    };
    let (x, y) = loading_song_origin(frame);
    let advance = text_width_cell(&caption, Cell::Loading);
    let scrim_w = u32::try_from(advance).unwrap_or(u32::MAX).saturating_add(8);
    let scrim_h = Cell::Loading.size().1.saturating_add(4);
    fill_rect(
        frame,
        &Rect {
            x: x - 4,
            y: y - 2,
            w: scrim_w,
            h: scrim_h,
        },
        scrim_color(),
    );
    draw_text_cell(frame, x, y, &caption, art_title_ink(), Cell::Loading);
}

/// Paint the loading gate scene. The OPEN bar stays hidden; see
/// [`LOADING_BAR_VISIBLE`].
fn paint_loading(frame: &mut Frame<'_>) {
    fill_background(frame, Screen::Loading);
    if LOADING_BAR_VISIBLE {
        paint_loading_bar(frame);
    }
}

/// Per-song illustration behind the loading bar/caption: the staged
/// [`LOADING_COVER_SPRITE`] figure (see
/// [`crate::avatar::ojn_cover_figure`]) stretched opaque over the
/// whole frame — the OJN cover is a full-bleed 800x600 body with no
/// colour key, so the verified opaque album path paints it, never
/// the keyed scene pass. `true` while a cover staged (the caller
/// skips the planet fallback then); `false` without one, leaving the
/// genuine `MusicLoading.ojs` fallback path alone. Staging recipe:
/// insert the decoded cover figure under [`LOADING_COVER_SPRITE`]
/// beside the planet art — no placement needed, this lookup reads
/// the figure directly and no scene pass paints it twice.
fn paint_loading_cover(frame: &mut Frame<'_>, art: &SceneArt) -> bool {
    let Some(layer) = art
        .figures
        .get(LOADING_COVER_SPRITE)
        .and_then(|figure| figure.layer(0))
    else {
        return false;
    };
    if layer.image.w == 0 || layer.image.h == 0 {
        return false;
    }
    let dest = Rect {
        x: 0,
        y: 0,
        w: u32::try_from(frame.w).unwrap_or(u32::MAX),
        h: u32::try_from(frame.h).unwrap_or(u32::MAX),
    };
    if dest.w == 0 || dest.h == 0 {
        return false;
    }
    paint_album_cover(frame, &layer.image, dest);
    true
}

/// Paint the loading gate with its live state: the staged song
/// illustration ([`LOADING_COVER_SPRITE`], the walked chart's OJN
/// cover section) full-bleed first when one staged, else the genuine
/// `MusicLoading.ojs` planet fallback — the exe mounts that overlay
/// only when the song load returns 0 (file `0xe47a0`, ev
/// `68d920c2…`), so a present cover (the load succeeded) replaces
/// it. The OPEN fractional bar stays hidden until [`LOADING_BAR_VISIBLE`];
/// the song title/artist still paints. The caption now routes through
/// `Cell::Loading` (the gate Trebuchet 27x65
/// recommendation per `docs/loading-type.md`); Arial 7x14 still has
/// no cell and the OJN/Vorbis path stays a gap. Unpainted while the
/// shell carries no walk state.
pub fn paint_loading_scene_with_status(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    status: &LoadingStatus,
) {
    if !paint_loading_cover(frame, art) {
        art.paint_backdrop(frame, Screen::Loading);
    }
    if LOADING_BAR_VISIBLE {
        paint_loading_bar_with_progress(frame, status.clamped_progress());
    }
    paint_loading_song(frame, status);
}

/// Paint staged loading art (`MusicLoading.ojs`) with no visible bar:
/// the legacy full-bar form is gated off by [`LOADING_BAR_VISIBLE`].
/// Callers with a walk fraction or song names use
/// [`paint_loading_scene_with_status`].
pub fn paint_loading_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    paint_loading_scene_with_status(
        frame,
        art,
        &LoadingStatus {
            progress: 1.0,
            title: String::new(),
            artist: String::new(),
        },
    );
}

mod chat;
mod result;
mod room;
mod waiting;

pub use self::chat::{
    channel_chat_row_text, channel_chat_visible_messages, paint_channel_chat, paint_room_chat,
    paint_waiting_chat, rank_notice_board_text, waiting_chat_row_text,
    waiting_chat_visible_messages, ChannelChatMessage, WaitingChatMessage,
};
// The existing paint guards exercise the chat block's private metrics;
// keep that seam test-only so the library build has no unused re-export.
#[cfg(test)]
pub(crate) use self::chat::{
    chat_admin_ink, chat_row_ink, chat_user_ink, waiting_chat_edit_origin,
    waiting_chat_list_origin, waiting_chat_row_y, CHANNEL_CHAT_ROW_H, CHANNEL_CHAT_VISIBLE_ROWS,
    ROOM_CHAT_CARET_INSET, ROOM_CHAT_EDIT_CODE, ROOM_CHAT_LIST_CODE, ROOM_CHAT_ROW_H,
    WAITING_CHAT_CARET_INSET, WAITING_CHAT_CARET_SIZE, WAITING_CHAT_EDIT_RECT,
    WAITING_CHAT_LIST_RECT, WAITING_CHAT_ROW_H, WAITING_CHAT_VISIBLE_ROWS,
};
pub(crate) use self::result::*;
pub use self::result::{
    battle_verdict, grade_frame, paint_result_scene, result_grade_frame, result_loader_sprites,
    result_portrait_slots, winlose_frame, BattleVerdict, ResultPaint, ResultPortraitSlot,
    ResultRankRow, ResultVerdict, RESULT_BOTTOM_FONT_SPRITE, RESULT_BOTTOM_SPRITE,
    RESULT_DISPLAY_MODE_TABLE_10, RESULT_DISPLAY_MODE_TABLE_F, RESULT_LIFE_FONT_SPRITE,
    RESULT_MUSIC_BOX, RESULT_PORTRAIT_TABLE_MODE_10, RESULT_PORTRAIT_TABLE_MODE_F,
    RESULT_RANK_SPRITE, RESULT_TOP_SPRITE, RESULT_WINLOSE_SPRITE, WINNER_IMAGE_SPRITE,
};
pub(crate) use self::room::*;
pub use self::room::{
    paint_room_scene, paint_room_scene_with_hover, paint_room_scene_with_selection,
    paint_room_user_panel, ROOM_HOVER_SPRITE, ROOM_MODE_SPRITE, ROOM_MUSIC_FLOOR, ROOM_OM_SPRITE,
    ROOM_PASSWORD_SPRITE, ROOM_STATE_SPRITE,
};
pub(crate) use self::waiting::{
    paint_waiting_members, waiting_bnd_origin, waiting_local_is_master,
};
pub use self::waiting::{
    paint_waiting_scene, paint_waiting_scene_with_confirm, paint_waiting_seat_avatar,
    paint_waiting_seat_avatar_at, paint_waiting_seat_avatars, waiting_bpm_caption,
    waiting_confirm_control_id, waiting_map_cycle, waiting_map_name, waiting_skillcount_text,
    WaitingCaptions, WaitingChatChannel, WaitingConfirmState, WaitingEffect, WaitingLevel,
    WaitingMapStep, WaitingOpenCaption, WaitingSkillItem, WaitingSkillSlots, WaitingSkillStep,
    WaitingSong, WAITING_MAP_DEFAULT_FRAMES, WAIT_BOTTOM_EFFECT_CODE, WAIT_TOP_EFFECT_CODE,
};
// The existing paint guards exercise the Waiting block's private metrics;
// keep that seam test-only so the library build has no unused re-export.
#[cfg(test)]
pub(crate) use self::waiting::{
    paint_waiting_bpm, paint_waiting_covers, paint_waiting_mapname, paint_waiting_music,
    paint_waiting_seat_marks, paint_waiting_skillcounts, paint_waiting_skills, paint_waiting_speed,
    paint_waiting_staged, paint_waiting_team_highlight, paint_waiting_title, paint_waiting_wins,
    ready_dot_color, seat_origin, staged_seats, waiting_control_armed_for_role,
    waiting_level_layer, waiting_music_caption, waiting_row_text, waiting_seat_caption,
    waiting_seats, SeatKind, StagedSeat, WAITING_AVATAR_SLOT_ID, WAITING_BPM_ID, WAITING_LEVEL_ID,
    WAITING_MAPNAME_ID, WAITING_MUSICNAME_ID, WAITING_ROOMNAME_ID, WAITING_SEATS,
    WAITING_SEAT_PARENT_IDS, WAITING_SKILLCOUNT_IDS, WAITING_SKILL_IDS, WAITING_SPEED_NUM_ID,
    WAITING_TEAM_IDS, WAITING_WINNERSCORE_ID,
};

/// Score digits anchor top-left (`LivePlay` draws points at
/// `(16, 16)` and combo at `(16, 32)`): the behind scene stays out
/// of that digits box, in frame pixels.
/// Frame-space zones the behind-highway scene never paints: the lane
/// columns only. The exe paints the stage backdrop first of all
/// (applier `0x4e7850`, file order `BACK1` before `BACK2`/`BACK3`),
/// so the art fills the frame whole — no HUD cutouts — and the passes
/// after it (staged gauges, avatar-scene captions, tally strip) keep
/// their top slot; only the falling lanes must stay uncovered, since
/// notes ride them.
fn playing_lane_exclusions(frame_w: usize, frame_h: usize) -> Vec<Rect> {
    HighwayGeometry::centered(frame_w, frame_h)
        .lanes
        .into_iter()
        .collect()
}

/// Paint the behind-highway scene with no exclusions: the caller
/// paints the highway after it, so lanes and UI ink cover the scene
/// wherever they overlap. Staged overlay panels are untouched here;
/// see [`paint_playing_overlay`] for the post-highway path.
pub fn paint_playing_backdrop(frame: &mut Frame<'_>, art: &SceneArt) {
    paint_behind_placements(frame, &art.placements, &art.figures, &[]);
}

/// Whether the staged Playing scene holds a behind-highway backdrop
/// placement (the venue/note-background set): the game loop then
/// paints over the staged backdrop and its flat placeholder fill
/// stays out (A44; the fill is ADR-0003's no-archive fallback).
pub fn playing_backdrop_staged(art: &SceneArt) -> bool {
    art.placements.iter().any(placement_is_behind)
}

/// Paint the foreground placements only (the non-behind scene set:
/// overlay panels, bound gauges, buttons) — the game loop's final
/// staged pass, after the highway painted over the backdrop.
pub fn paint_playing_foreground(frame: &mut Frame<'_>, art: &SceneArt) {
    paint_foreground_placements(frame, &art.placements, &art.figures);
}

/// Paint staged gameplay art around the highway: the behind-highway
/// scene first (`Playing_BG<stage>.ojs` plus the `Note_BG` siblings —
/// see `avatar::PLAYING_BEHIND_SPRITES`; `Note_BG.ojs` rides its
/// exe-selected frame only, `avatar::playing_member_frame`), clipped
/// only around the
/// lane columns per the exe paint order (the backdrop sits at the
/// bottom; staged gauges, captions, and the tally strip paint over
/// it), then the static backdrop panels (`Playing_Menu.ojs`, the
/// life/jam edge gauges, the level plate) and the bound gauges and
/// buttons (`State_Playing.bnd` ordinals 1–17) on top. Lanes stay
/// uncovered and notes keep their top slot. Procedural peer gauges sit
/// above this overlay: `LivePlay::paint_avatar_scene` paints them after,
/// in file order over the venue art they cover.
///
/// The game loop does not call this combined pass: `paint_playing`
/// threads the staged set through the loop's own blits —
/// [`paint_playing_backdrop`] first (the exe paints the backdrop
/// first of all), the highway over it, then [`paint_playing_foreground`]
/// (A44). This combined form stays for the direct painter tests.
pub fn paint_playing_overlay(frame: &mut Frame<'_>, art: &SceneArt) {
    let exclusions = playing_lane_exclusions(frame.w, frame.h);
    paint_behind_placements(frame, &art.placements, &art.figures, &exclusions);
    paint_foreground_placements(frame, &art.placements, &art.figures);
}

/// Fill the whole frame with one screen's background.
fn fill_background(frame: &mut Frame<'_>, screen: Screen) {
    fill_rect(
        frame,
        &Rect {
            x: 0,
            y: 0,
            w: u32::try_from(frame.w).unwrap_or(u32::MAX),
            h: u32::try_from(frame.h).unwrap_or(u32::MAX),
        },
        background_of(screen),
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
pub(crate) fn background_of(screen: Screen) -> u32 {
    match screen {
        Screen::Avi => pack_xrgb(0x08, 0x08, 0x10),
        Screen::Channel => pack_xrgb(0x10, 0x20, 0x50),
        Screen::Room => pack_xrgb(0x10, 0x50, 0x20),
        Screen::Waiting => pack_xrgb(0x50, 0x40, 0x10),
        Screen::Loading => pack_xrgb(0x18, 0x18, 0x18),
        Screen::Playing => pack_xrgb(0x0A, 0x0A, 0x0A),
        Screen::Result => pack_xrgb(0x10, 0x10, 0x08),
        Screen::Planet => pack_xrgb(0x14, 0x0C, 0x1E),
        Screen::Lanking => pack_xrgb(0x0C, 0x14, 0x1E),
        Screen::Myroom => pack_xrgb(0x10, 0x14, 0x1C),
        Screen::Shop => pack_xrgb(0x14, 0x14, 0x10),
        Screen::MusicShop => pack_xrgb(0x10, 0x18, 0x14),
    }
}

/// Label for the no-archive Item Shop card (#388). The offline walk
/// has no gateway session, so the flat fallback must say so; the
/// staged 7.55 board never carries this word.
pub(crate) const SHOP_OFFLINE_MARKER: &str = "OFFLINE STAND-IN";
/// Marker ink, distinct from the card/title colors so a guard can
/// pin the label itself rather than the card fill.
pub(crate) const SHOP_OFFLINE_MARKER_INK: u32 = pack_xrgb(0x80, 0x20, 0x80);

/// Paint the offline stand-in label under the fallback card title.
fn paint_shop_offline_marker(frame: &mut Frame<'_>) {
    let title_y = title_origin(frame, title_of(Screen::Shop)).1 + 14;
    let marker_x = frame.w as i32 / 2 - text_width(SHOP_OFFLINE_MARKER) as i32 / 2;
    draw_text(
        frame,
        marker_x,
        title_y,
        SHOP_OFFLINE_MARKER,
        SHOP_OFFLINE_MARKER_INK,
    );
}

/// Title drawn on each lobby card.
fn title_of(screen: Screen) -> &'static str {
    match screen {
        Screen::Channel => "CHANNEL",
        Screen::Room => "ROOM",
        Screen::Waiting => "WAITING",
        Screen::Planet => "PLANET",
        Screen::Lanking => "LANKING",
        Screen::Myroom => "MYROOM",
        Screen::Shop => "SHOP",
        Screen::MusicShop => "MUSICSHOP",
        _ => "",
    }
}

/// Paint one screen's scene.
pub fn paint_screen(frame: &mut Frame<'_>, screen: Screen) {
    match screen {
        Screen::Avi => paint_avi(frame),
        Screen::Channel
        | Screen::Room
        | Screen::Waiting
        | Screen::Planet
        | Screen::Lanking
        | Screen::Myroom
        | Screen::Shop
        | Screen::MusicShop => {
            paint_lobby(frame, background_of(screen));
            paint_title(frame, title_of(screen));
        }
        Screen::Loading => paint_loading(frame),
        Screen::Playing => {
            PlayingView::default().paint(frame, &HighwayGeometry::centered(frame.w, frame.h));
        }
        Screen::Result => paint_result(frame, &o2jam_game::Score::default(), &[], None),
    }
    if screen == Screen::Shop {
        paint_shop_offline_marker(frame);
    }
}

#[cfg(test)]
mod paint_tests {
    /// One solid probe figure at a known sprite anchor. Its ink lands
    /// where the blit puts the image origin, so the test needs no game
    /// data and can name the pinned pixel coordinates directly.
    fn featured_figure_probe() -> AvatarFigure {
        let pixels = [0x00_u8, 0x7C, 0x00, 0x7C, 0x00, 0x7C, 0x00, 0x7C];
        let mut blob = Vec::new();
        blob.extend_from_slice(&0_u16.to_le_bytes());
        blob.extend_from_slice(&o2jam_file::SPRITE55_MAGIC.to_le_bytes());
        blob.extend_from_slice(&1_u16.to_le_bytes());
        blob.extend_from_slice(&0x7D5F_u16.to_le_bytes());
        for word in [1_u16, 1, 2, 2] {
            blob.extend_from_slice(&word.to_le_bytes());
        }
        blob.extend_from_slice(&0_u32.to_le_bytes());
        blob.extend_from_slice(
            &u32::try_from(pixels.len())
                .expect("probe blob size fits")
                .to_le_bytes(),
        );
        blob.extend_from_slice(&0_u32.to_le_bytes());
        blob.extend_from_slice(&pixels);
        let sprite = o2jam_file::parse_sprite55(&blob).expect("synthetic pose frame");
        AvatarFigure::from_sprite55_frame(&sprite, 0)
    }

    #[test]
    fn featured_card_figures_land_on_the_pinned_exe_baseline_for_each_parent() {
        // Live caller chain: `write_shots` -> `shoot_side_native` ->
        // `paint_at` -> `paint_side` -> `paint_shop_featured_icons`.
        // Oracle: the 7.55 entry frame `060ab2af…de680` matches each
        // full-body sprite at native size; last-ink rows are
        // 233/233/233/234 (exclusive bottoms 234/234/234/235). The
        // image origins are (212,66), (380,66), (522,87), (669,94).
        let figure = featured_figure_probe();
        let figures: std::collections::HashMap<u32, AvatarFigure> =
            (1..=4).map(|id| (id, figure.clone())).collect();
        let goods: Vec<ShopGood> = (1..=4)
            .map(|item_id| ShopGood {
                item_id,
                name: String::new(),
                price: 0,
                gem_price: None,
                experience_bonus_percent: 0,
                jam_money_bonus_percent: 0,
            })
            .collect();
        let placements = [
            shop_placement(0x1401_D088, 213, 62, 138, 301),
            shop_placement(0x1401_D089, 354, 62, 138, 301),
            shop_placement(0x1401_D08A, 495, 62, 138, 301),
            shop_placement(0x1401_D08B, 636, 62, 138, 301),
        ];
        let mut buf = vec![0_u32; 800 * 600];
        paint_shop_featured_icons(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &goods,
            &figures,
        );
        for (slot, (left, top)) in [(212, 66), (380, 66), (522, 87), (669, 94)]
            .into_iter()
            .enumerate()
        {
            let x = usize::try_from(left).expect("card origin");
            let y = usize::try_from(top).expect("card origin");
            let at = |x: usize, y: usize| buf[y * 800 + x];
            assert_ne!(at(x, y), 0, "card {slot} figure");
            assert_eq!(at(x - 1, y), 0, "card {slot} left edge");
            assert_eq!(at(x, y - 1), 0, "card {slot} top edge");
        }
    }

    #[test]
    fn featured_card_blits_the_full_body_sprite_at_the_exe_native_box() {
        // Live caller chain: `write_shots` -> `shoot_side_native` ->
        // `paint_at` -> `paint_side` -> `paint_shop_featured_icons`.
        // The seam uses the installed sprite and a staged parent rect.
        // Interim post-fix evidence: the reviewer frame
        // `~/scratch/review-shop-figure/postfix-featured.png` (loss
        // 0.4 at each claimed origin). The live proof is the post-fix
        // v6 re-pair rerun after merge, not the pre-fix wave.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../install-root");
        let Some(source) = crate::avatar_source::read_installed_avatar(&root) else {
            println!("skip: missing install-root avatar archive");
            return;
        };
        let figures = crate::avatar::load_shop_featured_figures(source.archive(), source.table());
        let figure = figures.get(&1541).expect("card full-body figure");
        let layer = figure.layer(0).expect("one pose frame");
        let goods = [ShopGood {
            item_id: 1541,
            name: "블루".to_owned(),
            price: 4500,
            gem_price: None,
            experience_bonus_percent: 10,
            jam_money_bonus_percent: 10,
        }];
        let placements = [shop_placement(0x1401_D088, 213, 62, 138, 301)];
        let mut buf = vec![0_u32; 800 * 600];
        paint_shop_featured_icons(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &goods,
            &figures,
        );
        let (left, top) = (213 - 1, 62 + 4);
        let changed = |x: usize, y: usize| buf[y * 800 + x] != 0;
        assert!(changed(left + 60, top + 80), "figure body inks its box");
        assert!(!changed(left - 1, top + 80), "left edge is pinned");
        assert!(!changed(left + 60, top - 1), "top edge is pinned");
        let last_ink = (0..layer.image.words.len())
            .rev()
            .find(|&index| layer.image.words[index] != layer.colorkey)
            .expect("figure has visible pixels");
        let last_x = last_ink % layer.image.w;
        let last_y = last_ink / layer.image.w;
        assert!(
            changed(left + last_x, top + last_y),
            "last inked pixel lands"
        );
        assert!(!changed(left + layer.image.w, top + last_y));
    }

    #[test]
    fn card_attribute_lines_name_the_exe_experience_and_jam_bonuses() {
        // Live caller chain: `paint_side` -> `paint_shop_board` ->
        // `paint_shop_featured_captions` -> this line builder. The
        // 7.55 strings are pinned at exe VAs `0x64c6cc`/`0x64c6b4`;
        // both values come from the selected item row.
        assert_eq!(
            shop_card_attribute_lines(10, 10),
            [
                Some("고스트속성:".to_owned()),
                Some("- 경험치 : 10% 증가".to_owned()),
                Some("- 잼머니 : 10% 증가".to_owned()),
            ]
        );
    }

    #[test]
    fn card_attribute_lines_leave_zero_bonus_cells_blank() {
        assert_eq!(
            shop_card_attribute_lines(0, 10),
            [
                Some("고스트속성:".to_owned()),
                None,
                Some("- 잼머니 : 10% 증가".to_owned()),
            ]
        );
    }

    #[test]
    fn featured_price_row_inks_a_green_won_glyph_and_a_gold_gem_glyph() {
        // Live caller chain: boot -> stage_live_scenes ->
        // ScreenGame::stage_exe_item_shop_catalog -> paint_side ->
        // paint_shop_board -> paint_shop_featured_captions -> this row.
        // A seam fixture suffices for the glyph colors: it stages the
        // BND-derived SET_PRICE clone and needs no install-root art.
        if o2jam_text::system_font().is_none() {
            return;
        }
        let buf = featured_price_fixture();
        let has_glyph =
            |x: i32, y: i32, w: i32, h: i32, matches: fn((u32, u32, u32)) -> bool| -> bool {
                (y..y + h).any(|row| {
                    (x..x + w)
                        .any(|col| matches(rgb_channels(buf[row as usize * 800 + col as usize])))
                })
            };
        assert!(
            has_glyph(225, 238, 13, 12, is_green_ink),
            "won prefix inks the pinned green glyph"
        );
        assert!(
            has_glyph(317, 238, 9, 12, is_gold_ink),
            "gem prefix inks the pinned gold glyph"
        );
    }

    #[test]
    fn featured_price_digits_stay_bright_neutral_inside_both_digit_boxes() {
        // Same live chain as the glyph guard. The exe's number-font
        // digit frames use RGB555 core `0x318c`; only the prefix
        // glyphs are colored. A near-black antialias pixel is not
        // enough: the guard needs a bright neutral and no currency
        // ink inside either digit box (review mutation red proof).
        if o2jam_text::system_font().is_none() {
            return;
        }
        let buf = featured_price_fixture();
        let has_bright_neutral = |x: i32, w: i32| -> bool {
            (238..250).any(|row| {
                (x..x + w).any(|col| {
                    let (red, green, blue) = rgb_channels(buf[(row * 800 + col) as usize]);
                    red.min(green).min(blue) >= 0x40
                        && red.abs_diff(green) <= 6
                        && green.abs_diff(blue) <= 6
                })
            })
        };
        let has_currency_ink = |x: i32, w: i32| -> bool {
            (238..250).any(|row| {
                (x..x + w).any(|col| {
                    let rgb = rgb_channels(buf[(row * 800 + col) as usize]);
                    is_green_ink(rgb) || is_gold_ink(rgb)
                })
            })
        };
        for (part, x, w) in [("won", 240, 32), ("gem", 329, 7)] {
            assert!(
                !has_currency_ink(x, w),
                "{part} digit box carries no green or gold ink"
            );
            assert!(
                has_bright_neutral(x, w),
                "{part} digits ink a bright neutral"
            );
        }
    }

    #[test]
    fn featured_price_row_blits_one_sprite_frame_per_glyph_at_the_pinned_pitch() {
        // Live caller chain: boot -> stage_live_scenes ->
        // ScreenGame::stage_exe_item_shop_catalog -> paint_side ->
        // paint_shop_board -> paint_shop_featured_captions -> this row.
        // The seam injects 12 synthetic sprite frames per atlas so the
        // glyph source and the 8-px frame pitch stay observable without
        // install-root art: one solid frame per glyph, each in its own
        // color. Modern-font text cannot produce these exact pixels.
        let fonts = synthetic_price_fonts();
        let buf = featured_price_fixture_with_fonts(Some(&fonts));
        let pixel = |x: usize, y: usize| buf[y * 800 + x];
        let ink = |word: u16| {
            let (red, green, blue) = o2jam_file::rgb555_to_rgb888(word);
            pack_xrgb(red, green, blue)
        };

        // Won prefix: frame 11 at anchor - 8*digits - 7 = 225.
        assert_eq!(
            pixel(225, 238),
            ink(0x3706),
            "won prefix is sprite frame 11"
        );
        // Four won digits (4500) at 240/248/256/264, one 8-px pitch
        // apart. Each slot selects its own atlas frame, not the slot
        // index: the tens and ones zeros must both be frame 0.
        for (x, digit) in [(240, 4), (248, 5), (256, 0), (264, 0)] {
            assert_eq!(
                pixel(x, 238),
                ink(digit_word(0, digit)),
                "won digit {digit} at x={x}"
            );
        }
        // Gem prefix: frame 10 from the won atlas at 329 - 8 - 4 = 317.
        assert_eq!(
            pixel(317, 238),
            ink(0x7F20),
            "gem prefix is sprite frame 10"
        );
        assert_eq!(
            pixel(329, 238),
            ink(digit_word(1, 0)),
            "gem digit is a sprite frame at its anchor"
        );
    }

    #[test]
    fn staged_shop_price_fonts_come_from_the_exe_archive_atlas() {
        // Live caller chain: boot -> stage_live_art ->
        // live_art::stage_interface_scenes -> stage_state(State::Shop)
        // -> stage_shop_price_fonts. The install-root arm is the only
        // place the literal sprite names and their frame shapes can
        // be checked against the pin; skip cleanly without game data.
        let Some(archive) = interface_archive() else {
            return;
        };
        let entries = o2jam_file::parse_opi(&archive).expect("Interface directory parses");
        for (name, offset) in [
            (SHOP_WON_PRICE_FONT_SPRITE, 31_401_138usize),
            (SHOP_GEM_PRICE_FONT_SPRITE, 31_398_754usize),
        ] {
            let entry = o2jam_file::find_opi_entry(&entries, name).expect("font entry staged");
            assert_eq!(entry.data_offset, offset, "{name} sits at the pinned slot");
            assert_eq!(entry.size, 2384, "{name} has the pinned body size");
        }
        let fonts = stage_shop_price_fonts(&archive, StageContext::default())
            .expect("both runtime fonts decode");
        let frame_shape = |layer: &FigureLayer| {
            let ink = layer
                .image
                .words
                .iter()
                .filter(|word| **word != layer.colorkey)
                .count();
            (layer.image.w, layer.image.h, ink)
        };
        assert_eq!(
            fonts.won_frame(11).map(frame_shape),
            Some((13, 12, 111)),
            "green won glyph is won-atlas frame 11"
        );
        assert_eq!(
            fonts.won_frame(10).map(frame_shape),
            Some((9, 12, 95)),
            "gold G glyph is won-atlas frame 10"
        );
        assert_eq!(
            fonts.gem_frame(0).map(frame_shape),
            Some((7, 12, 74)),
            "digit 0 is a seven-pixel gray frame"
        );
    }

    #[test]
    fn featured_price_row_inks_the_real_archive_glyph_raster() {
        // Same live chain as the color guards, with real archive
        // frames: the sprite raster, not just its pinned position,
        // must reach the canvas. Skips without install-root.
        let Some(archive) = interface_archive() else {
            return;
        };
        let fonts = stage_shop_price_fonts(&archive, StageContext::default())
            .expect("both runtime fonts decode");
        let buf = featured_price_fixture_with_fonts(Some(&fonts));
        let buf = &buf;
        let nonzero_pixels = |x: i32, w: i32| {
            (238..250)
                .flat_map(|row| (x..x + w).map(move |col| buf[row as usize * 800 + col as usize]))
                .filter(|word| *word != 0)
                .count()
        };
        assert_eq!(nonzero_pixels(225, 13), 111, "won prefix raster is intact");
        assert_eq!(nonzero_pixels(317, 9), 95, "gem prefix raster is intact");
        assert_eq!(
            nonzero_pixels(240, 32),
            60 + 75 + 74 + 74,
            "four digit frames paint in sequence"
        );
    }

    #[test]
    fn caption_cell_ink_lands_two_pixels_below_the_exe_cell_top() {
        // Live caller chain: `paint_side` -> `paint_myroom_overlay` /
        // `paint_music_shop_owned_rows` -> this cell painter. Oracle:
        // the v3 exe rows ink at y=95 in a cell whose top is y=93,
        // centered in the staged creator cell; the shared left/top
        // helper misses both axes (senior note, commit 3701ddb).
        if o2jam_text::system_font().is_none() {
            return;
        }
        let mut buf = vec![0_u32; 800 * 600];
        paint_caption_in_cell(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            Rect {
                x: 100,
                y: 50,
                w: 96,
                h: 12,
            },
            "1234",
            CP949_INK,
            CaptionAlignment::Center,
        );
        let ink: Vec<(i32, i32)> = buf
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel != 0)
            .map(|(at, _)| ((at % 800) as i32, (at / 800) as i32))
            .collect();
        assert!(!ink.is_empty(), "the cell inks");
        assert_eq!(ink.iter().map(|(_, y)| *y).min(), Some(52), "GDI bearing");
        let left = ink.iter().map(|(x, _)| *x).min().unwrap_or_default();
        let right = ink.iter().map(|(x, _)| *x).max().unwrap_or_default();
        assert!((left + right - 2 * 148).abs() <= 2, "ink stays centered");
    }

    use super::super::side::{
        MSHOP_ARTIST_CODE, MSHOP_BPM_CODE, MSHOP_CART_TOGGLE_FIRST_CODE, MSHOP_EDITOR_CODE,
        MSHOP_EX_FIRST_CODE, MSHOP_GENRE_CODE, MSHOP_ISBUY_CODE, MSHOP_PAGE_CUR_ONES_CODE,
        MSHOP_PAGE_CUR_TENS_CODE, MSHOP_PAGE_TOT_ONES_CODE, MSHOP_PAGE_TOT_TENS_CODE,
        MSHOP_TITLE_CODE, PLANET_OPTION_CODE, PLANET_PROMO_IMAGE_CODE, SHOP_BAG_PAGE_CUR_ONES_CODE,
        SHOP_BAG_PAGE_CUR_TENS_CODE, SHOP_BAG_PAGE_TOT_ONES_CODE, SHOP_BAG_PAGE_TOT_TENS_CODE,
        SHOP_CART_CHECKGEM_CODE, SHOP_CART_CHECKMONEY_CODE, SHOP_CART_PAGE_CUR_ONES_CODE,
        SHOP_CART_PAGE_CUR_TENS_CODE, SHOP_CART_PAGE_TOT_ONES_CODE, SHOP_CART_PAGE_TOT_TENS_CODE,
        SHOP_CART_TOGGLE_FIRST_CODE, SHOP_ITEMGEM_FIRST_CODE,
    };
    use super::*;
    use crate::avatar::load_worn_figure;
    use crate::avatar::FigureLayer;
    use crate::screen::Screen;
    use crate::staging::{
        stage_shop_price_fonts, stage_state, ShopPriceFonts, StageContext, State,
        SHOP_GEM_PRICE_FONT_SPRITE, SHOP_WON_PRICE_FONT_SPRITE,
    };
    use o2jam_net::{JoinedRoom, RoomMember, RoomSlot};
    use o2jam_render::{has_ink_near, near_xrgb};
    use o2jam_render::{pack_xrgb, Frame, Rgb555Image};

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

    /// Read the pinned Interface archive, `None` without install-root.
    fn interface_archive() -> Option<Vec<u8>> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image/Interface1.opi"),
        )
        .ok()
    }

    /// One red pixel figure (colorkey white, so the pixel paints).
    fn red_figure() -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Art staging one red pixel at the frame origin.
    fn corner_art() -> SceneArt {
        SceneArt {
            placements: unbound_scene_placements(&["Logo.ojs".to_owned()]),
            figures: HashMap::from([("Logo.ojs".to_owned(), red_figure())]),
        }
    }

    fn paint_art(screen: Screen, art: &SceneArt, w: usize, h: usize) -> Vec<u32> {
        let mut buf = vec![0u32; w * h];
        let mut frame = Frame {
            buf: &mut buf,
            w,
            h,
        };
        match screen {
            Screen::Avi => paint_avi_scene(&mut frame, art),
            Screen::Channel => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Room => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Waiting => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Loading => paint_loading_scene(&mut frame, art),
            Screen::Playing => {
                paint_screen(&mut frame, screen);
                paint_playing_overlay(&mut frame, art);
            }
            Screen::Planet => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Lanking => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Myroom => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Shop => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::MusicShop => paint_lobby_backdrop(&mut frame, screen, art),
            Screen::Result => paint_result_scene(
                &mut frame,
                &ResultPaint {
                    portrait_slots: None,
                    score: &o2jam_game::Score::default(),
                    rows: &[],
                    art,
                    verdict: ResultVerdict::Solo,
                    grade: None,
                    life_readout: None,
                    bottom_counter: None,
                },
            ),
        }
        buf
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn shop_gem_cell_reads_the_exe_zero_when_no_price_word_is_pinned() {
        // D3 red proof: the exe entry frame `05b24e64…cb00` inks
        // `0` in every visible ITEMGEM cell (won `4500` beside it).
        // The table→`+0x18` map is still OPEN, so the staged catalog
        // row carries `gem_price: None`; the cell must still read
        // the exe's observed zero at the +70 child origin.
        let placements = vec![shop_placement(SHOP_ITEMGEM_FIRST_CODE, 300, 239, 38, 13)];
        let goods = vec![ShopGood {
            item_id: 1,
            name: "Hair".to_owned(),
            price: 4500,
            gem_price: None,
            experience_bonus_percent: 0,
            jam_money_bonus_percent: 0,
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            ShopBoard {
                featured_cards: false,
                panel: ShopPanel::ShoppingBag,
                goods: &goods,
                wallet: &WalletState::default(),
                cart: &[],
                cart_page: 0,
                cart_pages: 1,
            },
        );
        let mut with_word = vec![0u32; 800 * 600];
        paint_shop_board(
            &mut Frame {
                buf: &mut with_word,
                w: 800,
                h: 600,
            },
            &placements,
            ShopBoard {
                featured_cards: false,
                panel: ShopPanel::ShoppingBag,
                goods: &[ShopGood {
                    gem_price: Some(0),
                    experience_bonus_percent: 0,
                    jam_money_bonus_percent: 0,
                    ..goods[0].clone()
                }],
                wallet: &WalletState::default(),
                cart: &[],
                cart_page: 0,
                cart_pages: 1,
            },
        );
        assert_eq!(
            buf, with_word,
            "the fallback word has the exe's observed `0` glyph shape"
        );
        assert!(
            has_ink_in(&buf, 800, 300, 239, 38, 13),
            "the observed gem zero inks the staged child rect"
        );
        assert!(
            !has_ink_in(&buf, 800, 300, 169, 38, 13),
            "the raw BND row stays dark (the figure paints there)"
        );
    }

    #[test]
    fn shop_notice_text_inks_only_the_staged_top_strip_pair() {
        use super::super::side::{
            shop_notice_rects, SHOP_NOTICE_BOARD_CODE, SHOP_NOTICE_TEXT_CODE,
        };
        // Spec #178 D5 pin: `IMAGE_SSHOP_NOTICE_TEXT 0x142000b3` and
        // `IMAGE_SSHOP_NOTICEBOARD 0x142000b4` stage in the top strip;
        // the white tooltip is a separate, hover-gated rect. NOTICE
        // content has no 7.55 source pin, so the painter inks only an
        // explicitly staged string. Pure seam, hidden-arm safe.
        let text_rect = Rect {
            x: 100,
            y: 10,
            w: 200,
            h: 14,
        };
        let board_rect = Rect {
            x: 90,
            y: 5,
            w: 220,
            h: 30,
        };
        let placements = vec![
            shop_placement(SHOP_NOTICE_TEXT_CODE, 100, 10, 200, 14),
            shop_placement(SHOP_NOTICE_BOARD_CODE, 90, 5, 220, 30),
            shop_placement(
                super::super::side::SHOP_TOOLTIP_CODES[1],
                455,
                404,
                242,
                119,
            ),
        ];
        assert_eq!(
            shop_notice_rects(&placements),
            Some((text_rect, board_rect))
        );

        let mut buf = vec![0u32; 800 * 600];
        paint_shop_notice_text(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some("NOTICE"),
        );
        assert!(has_ink_in(&buf, 800, 100, 10, 200, 14), "notice ink");
        assert!(
            !has_ink_in(&buf, 800, 455, 404, 242, 119),
            "the tooltip stays a separate hover-gated rect"
        );

        let mut empty = vec![0u32; 800 * 600];
        paint_shop_notice_text(
            &mut Frame {
                buf: &mut empty,
                w: 800,
                h: 600,
            },
            &placements,
            None,
        );
        assert!(!has_ink_in(&empty, 800, 100, 10, 200, 14));
    }

    #[test]
    fn shop_identity_and_ten_skillcount_captions_ink_staged_cells() {
        use super::super::side::{
            SHOP_PLAYER_LEVEL_CAPTION_CODE, SHOP_SKILLCOUNT_CELLS, SHOP_SKILLCOUNT_FIRST_CODE,
        };
        // Static pin `ev_3e020a4ada701e31…`: `FUN_004a0660` inks
        // ` Lv.%d:%s` into `0x1440002a` and positions ten SKILLCOUNT
        // captions at `0x144010a7..b0`. Their value source stays OPEN,
        // so this guard stages counts instead of inventing one.
        // Pure painter with synthetic rects; hidden-arm safe.
        let mut placements = vec![shop_placement(
            SHOP_PLAYER_LEVEL_CAPTION_CODE,
            100,
            10,
            160,
            14,
        )];
        for slot in 0..SHOP_SKILLCOUNT_CELLS {
            placements.push(shop_placement(
                SHOP_SKILLCOUNT_FIRST_CODE + slot as u32,
                300 + slot as i32 * 30,
                10,
                24,
                14,
            ));
        }
        let mut counts = [None; SHOP_SKILLCOUNT_CELLS];
        counts[0] = Some(7);
        counts[9] = Some(999);
        let mut buf = vec![0u32; 800 * 600];

        paint_shop_identity_captions(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(" Lv.7:Alice"),
            &counts,
        );

        assert!(has_ink_in(&buf, 800, 100, 10, 160, 14), "level caption");
        assert!(has_ink_in(&buf, 800, 300, 10, 24, 14), "first skill slot");
        assert!(has_ink_in(&buf, 800, 570, 10, 24, 14), "tenth skill slot");
        assert!(!has_ink_in(&buf, 800, 330, 10, 24, 14), "unstaged slot");
    }

    #[test]
    fn shop_gem_price_cell_inks_an_explicit_pinned_word() {
        // #395 partial: the exe's second price cell has a painter, but
        // the live catalog still stages `gem_price: None` — the gifted
        // boot showed won 4500 / gem 0, so no table→entry `+0x18` map
        // is pinned. This guard only proves that an explicitly staged
        // word inks the staged ITEMGEM rect; it claims no live value.
        let placements = vec![shop_placement(SHOP_ITEMGEM_FIRST_CODE, 300, 169, 38, 13)];
        let goods = vec![ShopGood {
            item_id: 1,
            name: "Hair".to_owned(),
            price: 4500,
            gem_price: Some(3000),
            experience_bonus_percent: 0,
            jam_money_bonus_percent: 0,
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            ShopBoard {
                featured_cards: false,
                panel: ShopPanel::ShoppingBag,
                goods: &goods,
                wallet: &WalletState::default(),
                cart: &[],
                cart_page: 0,
                cart_pages: 1,
            },
        );
        assert!(has_ink_in(&buf, 800, 300, 169, 38, 13), "ITEMGEM inks");
    }

    #[test]
    fn shop_board_paints_names_prices_and_wallet_at_staged_rects() {
        // Synthetic `STATE_SHOP` board: two goods ink their names
        // and prices into the staged ITEMNAME/ITEMWON rects, the
        // wallet folds a gem word into GEMMONEY, and the pager
        // unions carry the cart page — every cell white ink on
        // black, so any lit pixel proves the overlay.
        let placements = vec![
            shop_placement(SHOP_ITEMNAME_FIRST_CODE, 231, 69, 100, 12),
            shop_placement(SHOP_ITEMNAME_FIRST_CODE + 1, 231, 222, 100, 12),
            shop_placement(SHOP_ITEMWON_FIRST_CODE, 247, 169, 38, 13),
            shop_placement(SHOP_ITEMWON_FIRST_CODE + 1, 247, 322, 38, 13),
            shop_placement(SHOP_GEMMONEY_CODE, 82, 288, 95, 13),
            shop_placement(SHOP_CART_PAGE_CUR_TENS_CODE, 247, 567, 14, 17),
            shop_placement(SHOP_CART_PAGE_CUR_ONES_CODE, 261, 567, 14, 17),
            shop_placement(SHOP_CART_PAGE_TOT_TENS_CODE, 287, 567, 14, 17),
            shop_placement(SHOP_CART_PAGE_TOT_ONES_CODE, 301, 567, 14, 17),
            shop_placement(SHOP_CART_NUM_CODE, 0, 0, 14, 13),
            shop_placement(SHOP_CART_ITEMNAME_CODE, 35, 0, 149, 15),
            shop_placement(SHOP_CART_PRICE_CODE, 188, 0, 50, 13),
            shop_placement(SHOP_CART_TOGGLE_FIRST_CODE, 137, 464, 282, 15),
        ];
        let goods = vec![
            ShopGood {
                item_id: 1,
                name: "Hair".to_owned(),
                price: 1000,
                gem_price: None,
                experience_bonus_percent: 0,
                jam_money_bonus_percent: 0,
            },
            ShopGood {
                item_id: 2,
                name: "Boots".to_owned(),
                price: 2500,
                gem_price: None,
                experience_bonus_percent: 0,
                jam_money_bonus_percent: 0,
            },
        ];
        let wallet = WalletState {
            gem: Some(1200),
            ..WalletState::default()
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            ShopBoard {
                featured_cards: false,
                panel: ShopPanel::ShoppingBag,
                goods: &goods,
                wallet: &wallet,
                cart: &goods[..1],
                cart_page: 0,
                cart_pages: 1,
            },
        );
        let lit = |x: i32, y: i32, w: u32, h: u32| -> bool { has_ink_in(&buf, 800, x, y, w, h) };
        assert!(lit(231, 69, 100, 12), "row name paints");
        assert!(lit(247, 169, 38, 13), "row price paints");
        assert!(lit(82, 288, 95, 13), "wallet gem paints");
        assert!(lit(247, 567, 28, 17), "cur page paints");
        assert!(lit(287, 567, 28, 17), "tot page paints");
        assert!(lit(137, 464, 149, 15), "cart row paints");
        assert!(!lit(0, 0, 100, 12), "unstaged band stays dark");
    }

    #[test]
    fn shop_preview_and_bag_pager_ink_their_staged_rects() {
        // Synthetic `STATE_SHOP` browse chrome: the preview pick
        // inks its name into the staged ITEMCOMMENT rect, the bag
        // pager inks the current page into the left digit union and
        // the total into the right one. The ITEMGEM cell at the raw
        // BND row stays dark: staging moves that cell +70 px before
        // it reaches this painter (#722 D3).
        let placements = vec![
            shop_placement(SHOP_ITEMCOMMENT_CODE, 468, 421, 163, 89),
            shop_placement(SHOP_BAG_PAGE_CUR_TENS_CODE, 317, 568, 14, 17),
            shop_placement(SHOP_BAG_PAGE_CUR_ONES_CODE, 331, 568, 14, 17),
            shop_placement(SHOP_BAG_PAGE_TOT_TENS_CODE, 357, 568, 14, 17),
            shop_placement(SHOP_BAG_PAGE_TOT_ONES_CODE, 371, 568, 14, 17),
            shop_placement(SHOP_ITEMGEM_FIRST_CODE, 300, 239, 38, 13),
            shop_placement(SHOP_ITEMNAME_FIRST_CODE, 231, 69, 100, 12),
            shop_placement(SHOP_ITEMWON_FIRST_CODE, 247, 169, 38, 13),
        ];
        let preview = ShopGood {
            item_id: 7,
            name: "PreviewHair".to_owned(),
            price: 14000,
            gem_price: None,
            experience_bonus_percent: 0,
            jam_money_bonus_percent: 0,
        };
        let goods = vec![ShopGood {
            item_id: 1,
            name: "Hair".to_owned(),
            price: 1000,
            gem_price: None,
            experience_bonus_percent: 0,
            jam_money_bonus_percent: 0,
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            ShopBoard {
                featured_cards: false,
                panel: ShopPanel::ShoppingBag,
                goods: &goods,
                wallet: &WalletState::default(),
                cart: &[],
                cart_page: 0,
                cart_pages: 1,
            },
        );
        paint_shop_preview(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(&preview),
        );
        paint_shop_bag_pager(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            1,
            3,
        );
        let lit = |x: i32, y: i32, w: u32, h: u32| -> bool { has_ink_in(&buf, 800, x, y, w, h) };
        assert!(lit(468, 421, 163, 89), "preview name paints");
        assert!(lit(317, 568, 28, 17), "bag cur page paints");
        assert!(lit(357, 568, 28, 17), "bag tot page paints");
        assert!(
            !lit(300, 169, 38, 13),
            "raw ITEMGEM row stays dark; staging owns the +70 origin"
        );
    }

    #[test]
    fn shop_board_paints_won_balance_and_cart_totals_at_staged_rects() {
        // Ticket #196: the `MGASH` cash cell and the cart checkout
        // totals ink at their staged rects — the cash word from the
        // 5001 prefix, the money total summed from the row prices,
        // `--` for the gem total (no gem-price source is pinned).
        let placements = vec![
            shop_placement(SHOP_MGASH_CODE, 200, 288, 95, 13),
            shop_placement(SHOP_CART_CHECKGEM_CODE, 82, 320, 95, 13),
            shop_placement(SHOP_CART_CHECKMONEY_CODE, 200, 320, 95, 13),
        ];
        let wallet = WalletState {
            gem: Some(1200),
            o2_cash: Some(50),
            ..WalletState::default()
        };
        let cart = vec![ShopGood {
            item_id: 1,
            name: "Hair".to_owned(),
            price: 1000,
            gem_price: None,
            experience_bonus_percent: 0,
            jam_money_bonus_percent: 0,
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            ShopBoard {
                featured_cards: false,
                panel: ShopPanel::ShoppingBag,
                goods: &[],
                wallet: &wallet,
                cart: &cart,
                cart_page: 0,
                cart_pages: 1,
            },
        );
        let lit = |x: i32, y: i32, w: u32, h: u32| -> bool { has_ink_in(&buf, 800, x, y, w, h) };
        assert!(lit(200, 288, 95, 13), "won balance paints");
        assert!(lit(82, 320, 95, 13), "cart gem total paints");
        assert!(lit(200, 320, 95, 13), "cart money total paints");
    }

    #[test]
    fn cart_total_cells_ink_only_when_the_cart_count_gate_is_open() {
        // Live caller chain:
        //   main.rs::run_play -> OnlineSession::paint ->
        //   ScreenGame::paint_shop -> paint_shop_board ->
        //   paint_shop_cart_totals -> staged CHECKMONEY/CHECKGEM rects.
        // Static pins: FUN_004a1010 (ev_ec83763a…) enables the total
        // cells by cart-count `+0x2334`; FUN_004a0660
        // (ev_3e020a4a…) stages their rects into object `+0x247c`/
        // `+0x248c`. The exe's own total-ink writer stays OPEN; our
        // staged-price sum remains the labeled source.
        let placements = vec![
            shop_placement(SHOP_CART_CHECKGEM_CODE, 82, 320, 95, 13),
            shop_placement(SHOP_CART_CHECKMONEY_CODE, 200, 320, 95, 13),
        ];
        let good = ShopGood {
            item_id: 1,
            name: "TotalGood".to_owned(),
            price: 1200,
            gem_price: None,
            experience_bonus_percent: 0,
            jam_money_bonus_percent: 0,
        };
        let cells_ink = |cart: &[ShopGood]| {
            let mut buf = vec![0u32; 800 * 600];
            let mut frame = Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            };
            paint_shop_board(
                &mut frame,
                &placements,
                ShopBoard {
                    featured_cards: false,
                    panel: ShopPanel::ShoppingBag,
                    goods: &[],
                    wallet: &WalletState::default(),
                    cart,
                    cart_page: 0,
                    cart_pages: 1,
                },
            );
            (
                has_ink_in(&buf, 800, 82, 320, 95, 13),
                has_ink_in(&buf, 800, 200, 320, 95, 13),
            )
        };

        assert_eq!(
            cells_ink(&[]),
            (false, false),
            "an empty cart leaves both cells dark"
        );
        assert_eq!(
            cells_ink(std::slice::from_ref(&good)),
            (true, true),
            "the staged-price writer inks both cells"
        );
    }

    #[test]
    fn shop_avatar_pane_paints_staged_figure_on_paint_clock() {
        // Ticket #196: the equipped (or preview) figure inks at the
        // staged `IDC_IMAGE_SSHOP_AVATAR` rect on the paint clock
        // (ticket #193 breathing path, no fork); `None` paints
        // nothing so the staged art stays alone.
        let placements = vec![shop_placement(SHOP_AVATAR_CODE, 600, 100, 120, 200)];
        let figure = red_figure();
        let mut buf = vec![0u32; 800 * 600];
        paint_shop_avatar(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(&figure),
            0,
        );
        assert!(
            has_ink_in(&buf, 800, 600, 100, 120, 200),
            "staged figure paints in the avatar pane"
        );
        let mut dark = vec![0u32; 800 * 600];
        paint_shop_avatar(
            &mut Frame {
                buf: &mut dark,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            0,
        );
        assert!(
            !has_ink_in(&dark, 800, 600, 100, 120, 200),
            "unstaged pane paints no figure"
        );
    }

    #[test]
    fn idle_download_bars_ink_the_exe_dark_face() {
        // Live caller chain: `ScreenGame::paint_side` ->
        // `paint_music_shop_progress` with no active transfer. The
        // v3 exe entry frame inks RGB (33,49,74) at (450,455) and
        // (600,498); the staged progress sprite alone paints bright
        // blue, so idle must cover it inside the staged rect.
        let placements = [
            shop_placement(MSHOP_CURRENT_PROGRESS_CODE, 436, 447, 334, 15),
            shop_placement(MSHOP_TOTAL_PROGRESS_CODE, 436, 491, 334, 15),
        ];
        let mut buf = vec![0_u32; 800 * 600];
        paint_music_shop_progress(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            None,
        );
        assert_eq!(buf[455 * 800 + 450], pack_xrgb(0x21, 0x31, 0x4A));
        assert_eq!(buf[498 * 800 + 600], pack_xrgb(0x21, 0x31, 0x4A));
    }

    #[test]
    fn music_shop_board_paints_catalog_cells_and_pager_at_staged_rects() {
        // Synthetic `STATE_MUSICSHOP` board: one catalog row inks
        // its cells into the row-local MMS templates riding the
        // staged MS_EX origin, and the pager unions carry the
        // list page — blank ISBUY paints nothing (no flags staged).
        let placements = vec![
            shop_placement(MSHOP_EX_FIRST_CODE, 18, 104, 353, 86),
            shop_placement(MSHOP_TITLE_CODE, 89, 5, 213, 11),
            shop_placement(MSHOP_ISBUY_CODE, 281, 4, 66, 11),
            shop_placement(MSHOP_ARTIST_CODE, 122, 23, 106, 11),
            shop_placement(MSHOP_EDITOR_CODE, 265, 23, 82, 11),
            shop_placement(MSHOP_GENRE_CODE, 122, 38, 65, 11),
            shop_placement(MSHOP_BPM_CODE, 223, 38, 25, 11),
            shop_placement(MSHOP_CART_NUM_CODE, 0, 0, 14, 13),
            shop_placement(MSHOP_CART_ITEMNAME_CODE, 35, 0, 149, 15),
            shop_placement(MSHOP_CART_TOGGLE_FIRST_CODE, 481, 429, 282, 15),
            shop_placement(MSHOP_PAGE_CUR_TENS_CODE, 246, 534, 14, 17),
            shop_placement(MSHOP_PAGE_CUR_ONES_CODE, 260, 534, 14, 17),
            shop_placement(MSHOP_PAGE_TOT_TENS_CODE, 286, 534, 14, 17),
            shop_placement(MSHOP_PAGE_TOT_ONES_CODE, 300, 534, 14, 17),
        ];
        let rows = vec![MusicShopRow {
            song: LankingSong {
                song_id: 757,
                title: "First".to_owned(),
                artist: "Band".to_owned(),
                editor: "NoteFactory".to_owned(),
                genre: 3,
                bpm: 124.0,
                levels: [5, 9, 14],
            },
            free: false,
            listed: false,
            gem_price: Some(3000),
            won_price: Some(0),
            installed: false,
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_music_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &rows,
            0,
            1,
        );
        let lit = |x: i32, y: i32, w: u32, h: u32| -> bool { has_ink_in(&buf, 800, x, y, w, h) };
        assert!(lit(107, 109, 213, 11), "row title paints");
        assert!(lit(140, 127, 106, 11), "row artist paints");
        assert!(!lit(299, 108, 66, 11), "blank ISBUY paints nothing");
        assert!(lit(246, 534, 28, 17), "cur page paints");
        assert!(!lit(481, 429, 282, 15), "empty cart paints nothing");
    }

    #[test]
    fn music_shop_board_inks_diff_at_the_staged_cell_from_catalog_levels() {
        // Red proof for #398: the DIFF cell (`0x1A40003C`) is staged,
        // but the row painter only inks the six original MMS cells,
        // so the catalog levels never reach the canvas.
        let placements = vec![
            shop_placement(MSHOP_EX_FIRST_CODE, 18, 104, 353, 86),
            shop_placement(MSHOP_TITLE_CODE, 89, 5, 213, 11),
            shop_placement(MSHOP_ISBUY_CODE, 281, 4, 66, 11),
            shop_placement(MSHOP_ARTIST_CODE, 122, 23, 106, 11),
            shop_placement(MSHOP_EDITOR_CODE, 265, 23, 82, 11),
            shop_placement(MSHOP_GENRE_CODE, 122, 38, 65, 11),
            shop_placement(MSHOP_BPM_CODE, 223, 38, 25, 11),
            shop_placement(0x1A40_003C, 297, 38, 50, 11),
            shop_placement(0x1A40_0040, 264, 54, 41, 11),
            shop_placement(0x1A40_0045, 264, 70, 41, 11),
        ];
        let rows = vec![MusicShopRow {
            song: LankingSong {
                song_id: 757,
                title: "First".to_owned(),
                artist: "Band".to_owned(),
                editor: "NoteFactory".to_owned(),
                genre: 3,
                bpm: 124.0,
                levels: [3, 7, 10],
            },
            free: false,
            listed: false,
            gem_price: Some(3000),
            won_price: Some(0),
            installed: false,
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_music_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &rows,
            0,
            1,
        );
        assert!(
            has_ink_in(&buf, 800, 315, 142, 50, 11),
            "catalog DIFF levels ink the staged DIFF cell"
        );
        assert!(
            has_ink_in(&buf, 800, 282, 158, 41, 11),
            "catalog gem price inks the staged GEM cell"
        );
        assert!(
            has_ink_in(&buf, 800, 282, 174, 41, 11),
            "catalog won price inks the staged WON cell"
        );
    }

    #[test]
    fn music_shop_bottom_wallet_inks_gem_and_won_from_the_shared_wallet() {
        // The bottom money texts stage at absolute rects and read
        // the one wallet the shops share; unknown amounts stay `--`.
        let placements = vec![
            shop_placement(MSHOP_MONEY_GEM_CODE, 41, 567, 95, 13),
            shop_placement(MSHOP_MONEY_WON_CODE, 166, 567, 95, 13),
        ];
        let wallet = WalletState {
            gem: Some(15527),
            cash_point: Some(0),
            ..WalletState::default()
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_music_shop_wallet(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &wallet,
        );
        assert!(has_ink_in(&buf, 800, 41, 567, 95, 13), "G amount paints");
        assert!(has_ink_in(&buf, 800, 166, 567, 95, 13), "W amount paints");
        assert!(
            has_ink_in(&buf, 800, 124, 567, 12, 13),
            "the v3 exe inks the amount at the right edge of the staged cell"
        );
    }

    #[test]
    fn music_shop_board_paints_owned_marks_at_staged_rects() {
        // Owned-state marks (FREE/OWNED) ride the staged ISBUY cell.
        let placements = vec![
            shop_placement(MSHOP_EX_FIRST_CODE, 18, 104, 353, 86),
            shop_placement(MSHOP_TITLE_CODE, 89, 5, 213, 11),
            shop_placement(MSHOP_ISBUY_CODE, 281, 4, 66, 11),
            shop_placement(MSHOP_ARTIST_CODE, 122, 23, 106, 11),
            shop_placement(MSHOP_EDITOR_CODE, 265, 23, 82, 11),
            shop_placement(MSHOP_GENRE_CODE, 122, 38, 65, 11),
            shop_placement(MSHOP_BPM_CODE, 223, 38, 25, 11),
        ];
        let rows = vec![MusicShopRow {
            song: LankingSong {
                song_id: 757,
                title: "First".to_owned(),
                artist: "Band".to_owned(),
                editor: "NoteFactory".to_owned(),
                genre: 3,
                bpm: 124.0,
                levels: [5, 9, 14],
            },
            free: true,
            listed: false,
            gem_price: None,
            won_price: None,
            installed: false,
        }];
        let mut buf = vec![0u32; 800 * 600];
        paint_music_shop_board(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &rows,
            0,
            1,
        );
        let lit = |x: i32, y: i32, w: u32, h: u32| -> bool { has_ink_in(&buf, 800, x, y, w, h) };
        assert!(lit(299, 108, 66, 11), "FREE owned mark paints");
    }

    fn shop_placement(code: u32, x: i32, y: i32, w: u32, h: u32) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        }
    }

    /// One solid sprite-55 frame, in the archive's raw RGB555 layout.
    fn solid_glyph_layer(width: u32, word: u16) -> FigureLayer {
        let words = word
            .to_le_bytes()
            .repeat(usize::try_from(width * 12).unwrap_or(0));
        FigureLayer {
            image: Rgb555Image::decode(&words, width as usize, 12)
                .expect("synthetic glyph decodes"),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        }
    }

    /// Neutral gray for glyph `index` inside atlas `atlas`, packed as
    /// RGB555 so the assertion can name the exact frame it expects.
    fn digit_word(atlas: u16, index: u16) -> u16 {
        let level = 4 + atlas + index;
        (level << 10) | (level << 5) | level
    }

    /// Twelve frames per atlas in stored order: `0..9` digits, `10`
    /// gold, `11` green — the 7.55 font-body frame map (`docs/shop.md`).
    fn synthetic_price_fonts() -> ShopPriceFonts {
        let atlas = |atlas: u16| {
            let mut figure = AvatarFigure::default();
            for index in 0..12 {
                let (width, word) = match index {
                    10 => (9, 0x7F20),
                    11 => (13, 0x3706),
                    _ => (7, digit_word(atlas, index)),
                };
                figure.push_layer(solid_glyph_layer(width, word));
            }
            figure
        };
        ShopPriceFonts {
            won: atlas(0),
            gem: atlas(1),
        }
    }

    /// Painted price row of one featured card, staged from the
    /// BND-derived `SET_PRICE` clone. Shared by the guards so they
    /// differ only in the behavior they assert.
    fn featured_price_fixture() -> Vec<u32> {
        featured_price_fixture_with_fonts(None)
    }

    /// The same fixture with an injected font atlas, for the sprite
    /// raster guard; `None` keeps the no-data text fallback.
    fn featured_price_fixture_with_fonts(fonts: Option<&ShopPriceFonts>) -> Vec<u32> {
        let parent = super::super::side::SHOP_SET_CARD_PARENT_IDS[0].to_owned();
        let placements = [ScenePlacement {
            role: PlacementRole::Clone,
            id: parent,
            code: 0x1440_0094,
            sprite: None,
            rect: Rect {
                x: 219,
                y: 238,
                w: 126,
                h: 12,
            },
        }];
        let goods = [ShopGood {
            item_id: 1541,
            name: "블루".to_owned(),
            price: 4500,
            gem_price: None,
            experience_bonus_percent: 0,
            jam_money_bonus_percent: 0,
        }];
        let mut buf = vec![0_u32; 800 * 600];
        paint_shop_featured_captions(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &goods,
            fonts,
        );
        buf
    }

    fn rgb_channels(pixel: u32) -> (u32, u32, u32) {
        let rgb = pixel & 0xFF_FFFF;
        ((rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF)
    }

    fn is_green_ink(rgb: (u32, u32, u32)) -> bool {
        let (red, green, blue) = rgb;
        green > red + 20 && green > blue + 20
    }

    fn is_gold_ink(rgb: (u32, u32, u32)) -> bool {
        let (red, green, blue) = rgb;
        red > blue + 40 && green > blue + 40 && red > 100
    }

    /// The paired 2026-11-23 cart frame puts the pinned O2CASH shortfall
    /// caption in two rows inside the `Dialog_Question1.bnd` message rect.
    /// Live chain: `main.rs::run_play` -> `OnlineSession::ingest_snapshot`
    /// -> `click_side_screen` -> `click_shop` -> `ShopAction::CartBuy`
    /// -> `shop_charge_confirm_open` -> `paint_side`
    /// -> `paint_shop_charge_confirm` -> `paint_message_caption_in_rect`.
    /// Seam suffices for the bearing itself: the test uses the parsed BND
    /// rect and caption bytes, while the paired re-shot proves the live leg.
    #[test]
    fn shop_shortfall_caption_inks_both_exe_lines_in_the_staged_message_rect() {
        let backdrop = ScenePlacement {
            role: PlacementRole::Single,
            id: "DIALOG_QUESTION1".to_owned(),
            code: 0x2240_0000,
            sprite: Some("Dialog_Question1.ojs".to_owned()),
            rect: Rect {
                x: 240,
                y: 189,
                w: 320,
                h: 221,
            },
        };
        let art = SceneArt {
            placements: vec![
                backdrop,
                shop_placement(super::super::side::QUIT_MESSAGE_CODE, 101, 58, 182, 111),
            ],
            figures: HashMap::from([(
                "Dialog_Question1.ojs".to_owned(),
                solid_figure(1, 1, [0, 0x7C]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };

        paint_shop_charge_confirm(&mut frame, Some(&art));

        assert!(has_ink_in(&buf, 800, 342, 285, 188, 8), "first exe line");
        assert!(has_ink_in(&buf, 800, 342, 296, 120, 8), "second exe line");
    }

    fn has_ink_in(buf: &[u32], w: usize, x: i32, y: i32, rw: u32, rh: u32) -> bool {
        for row in y..y + rh as i32 {
            for col in x..x + rw as i32 {
                if row >= 0 && col >= 0 {
                    if let Some(pixel) = buf.get(row as usize * w + col as usize) {
                        if *pixel != 0 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    #[test]
    fn shop_staged_art_paints_at_anchor() {
        // Synthetic staged figure (red pixel): unreachable screens
        // still paint through their scene function directly.
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_shop_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn music_shop_top100_press_inks_only_the_pressed_staged_rect() {
        // #408: the board chrome marker rides the staged control
        // rects, never a ranking row or an invented panel. No press
        // paints nothing; pressing SELECTALL leaves the TOP100 button
        // untouched (and vice versa).
        use super::super::side::{MSHOP_TOP100_CODE, MSHOP_TOP100_LAST_CODE};
        let placements = vec![
            shop_placement(MSHOP_TOP100_CODE, 248, 0, 99, 37),
            shop_placement(MSHOP_TOP100_LAST_CODE, 436, 31, 52, 20),
        ];
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        paint_music_shop_top100_chrome(&mut frame, &placements, None);
        assert!(!frame.buf.contains(&MUSIC_SHOP_TOP100_TONE));

        let top100_band = 35 * 800 + 250;
        let selectall_band = 49 * 800 + 440;
        paint_music_shop_top100_chrome(&mut frame, &placements, Some(MSHOP_TOP100_CODE));
        assert_eq!(frame.buf[top100_band], MUSIC_SHOP_TOP100_TONE);
        assert_eq!(frame.buf[selectall_band], 0, "unpressed arm stays unmarked");

        frame.buf.fill(0);
        paint_music_shop_top100_chrome(&mut frame, &placements, Some(MSHOP_TOP100_LAST_CODE));
        assert_eq!(frame.buf[selectall_band], MUSIC_SHOP_TOP100_TONE);
        assert_eq!(frame.buf[top100_band], 0, "TOP100 button stays unmarked");
    }

    #[test]
    fn music_shop_staged_art_paints_at_anchor() {
        // Synthetic staged figure (red pixel): unreachable screens
        // still paint through their scene function directly.
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_music_shop_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn myroom_staged_art_paints_at_anchor() {
        // Synthetic staged figure (red pixel): anchor placements put
        // art at sprite anchors without any `.bnd`. Unreachable screens
        // still paint through their scene function directly.
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_myroom_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn lanking_staged_art_paints_at_anchor() {
        // Synthetic staged figure (red pixel): the scene painter puts
        // art behind the (data-less) list view. Unreachable screens
        // still paint through their scene function directly.
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_lanking_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn splash_paints_logo_at_native_origin_without_stamp() {
        // Unbound logo art paints 1:1 at the native origin: logo
        // pixel at the origin, no flat card, no version-stamp white.
        let buf = paint_art(Screen::Avi, &corner_art(), 32, 24);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!buf.contains(&pack_xrgb(0x20, 0x60, 0xC0)));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn shop_no_archive_fallback_carries_the_offline_stand_in_marker() {
        // #388: with no archive, the Item Shop card is a labeled
        // stand-in, never an online-shop claim. The marker ink is
        // distinct so the guard pins the label itself, not the card
        // fill. The staged-art path stays unlabeled.
        let mut fallback = vec![0u32; 160 * 120];
        paint_screen(
            &mut Frame {
                buf: &mut fallback,
                w: 160,
                h: 120,
            },
            Screen::Shop,
        );
        assert!(
            fallback.contains(&SHOP_OFFLINE_MARKER_INK),
            "no-archive Item Shop fallback must paint its OFFLINE STAND-IN marker"
        );
        let mut staged = vec![0u32; 160 * 120];
        paint_shop_scene(
            &mut Frame {
                buf: &mut staged,
                w: 160,
                h: 120,
            },
            &corner_art(),
        );
        assert!(
            !staged.contains(&SHOP_OFFLINE_MARKER_INK),
            "staged 7.55 shop art must not carry the fallback marker"
        );
    }

    #[test]
    fn flat_fallback_covers_bare_frames() {
        // While no art is staged the flat colors remain (the flat
        // list/member painters depend on them); staged scenes never
        // use them.
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
            Screen::Planet,
            Screen::Lanking,
            Screen::Myroom,
            Screen::Shop,
            Screen::MusicShop,
        ] {
            let buf = painted(screen, 16, 12);
            assert_eq!(buf[0], background_of(screen), "{screen:?}");
        }
    }

    #[test]
    fn every_art_scene_hides_flat_fill() {
        // Staged art-first composition: the logo pixel survives at
        // the origin and no flat background shows through.
        let art = corner_art();
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let buf = paint_art(screen, &art, 16, 12);
            assert_eq!(buf[0], pack_xrgb(255, 0, 0), "{screen:?}");
            assert!(
                !buf.iter().all(|pixel| *pixel == background_of(screen)),
                "{screen:?} paints a flat fill"
            );
        }
    }

    #[test]
    fn scene_indexes_cover_every_screen_once() {
        let indexes = [
            scene_index(Screen::Avi),
            scene_index(Screen::Channel),
            scene_index(Screen::Room),
            scene_index(Screen::Waiting),
            scene_index(Screen::Loading),
            scene_index(Screen::Playing),
            scene_index(Screen::Result),
            scene_index(Screen::Planet),
            scene_index(Screen::Lanking),
            scene_index(Screen::Myroom),
            scene_index(Screen::Shop),
            scene_index(Screen::MusicShop),
        ];
        let mut sorted = indexes;
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn unbound_placements_pin_origin_without_size() {
        // No `.bnd` means no layout: unbound sprites paint 1:1 at
        // the native origin with an empty rect, never full-bleed.
        let placed = unbound_scene_placements(&["A.ojs".to_owned(), "B.ojs".to_owned()]);
        assert_eq!(placed.len(), 2);
        for placement in &placed {
            assert_eq!(
                placement.rect,
                Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                }
            );
        }
        assert_eq!(placed[0].sprite.as_deref(), Some("A.ojs"));
    }

    #[test]
    fn lobby_titles_stay_readable_over_art() {
        // 128x96 channel backdrop: white title ink over the dark
        // scrim, never the flat card gray.
        let buf = paint_art(Screen::Channel, &corner_art(), 128, 96);
        let scrim = pack_xrgb(0x08, 0x08, 0x10);
        let white = pack_xrgb(0xFF, 0xFF, 0xFF);
        let card = pack_xrgb(0xD0, 0xD0, 0xD0);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&white));
        assert!(buf.contains(&scrim));
        assert!(!buf.contains(&card));
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
            slots: vec![
                RoomSlot {
                    index: 0,
                    state_code: 1,
                    member: Some(member),
                },
                RoomSlot {
                    index: 1,
                    state_code: 0,
                    member: None,
                },
                RoomSlot {
                    index: 2,
                    state_code: 2,
                    member: None,
                },
            ],
            skills: Vec::new(),
            premium: false,
            champion: None,
            champion_streak: None,
        }
    }

    #[test]
    fn waiting_seats_follow_wire_slots_and_pad_to_eight() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(seats.len(), WAITING_SEATS);
        assert_eq!(seats[0].display_name(), Some("host"));
        assert!(seats[0].is_ready());
        assert_eq!(seats[1].display_name(), None);
        assert_eq!(seats[2].kind, SeatKind::Locked);
        assert!(seats[3..].iter().all(|seat| seat.kind == SeatKind::Empty));
    }

    #[test]
    fn waiting_row_text_shows_name_team_and_ready() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(waiting_row_text(0, &seats[0]), "0 A host R");
        assert_eq!(waiting_row_text(1, &seats[1]), "1 - ---");
        assert_eq!(waiting_row_text(2, &seats[2]), "2 - LOCK");
    }

    #[test]
    fn waiting_header_wells_show_one_wire_digit_each() {
        // Live caller chain:
        //   main.rs::run_play -> setup_online -> OnlineSession::paint
        //   -> ScreenGame::paint -> ScreenGame::paint_waiting
        //   -> paint_waiting_scene -> paint_waiting_staged
        //   -> paint_waiting_header_numbers.
        // p343 exe frame: Ch.01 / No.000 under byte-equal wire. The
        // old stacked figure showed the last glyph (9) in every well.
        let mut placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_COVERAVATAR1",
            Some("Wait_Cover_Avatar.ojs"),
            (10, 200, 151, 375),
        )];
        for (id, x) in [
            ("IDC_IMAGE_SWAIT_CHNUM_10", 100),
            ("IDC_IMAGE_SWAIT_CHNUM_01", 117),
            ("IDC_IMAGE_SWAIT_RMNUM_100", 159),
            ("IDC_IMAGE_SWAIT_RMNUM_010", 176),
            ("IDC_IMAGE_SWAIT_RMNUM_001", 193),
        ] {
            placements.push(waiting_probe(
                id,
                Some("Wait_RoomNumber.ojs"),
                (x, 19, x + 13, 32),
            ));
        }
        let art = SceneArt {
            placements,
            figures: HashMap::from([
                ("Wait_Cover_Avatar.ojs".to_owned(), red_figure()),
                ("Wait_RoomNumber.ojs".to_owned(), digit_figure()),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions {
                channel_number: Some(0),
                room_number: Some(0),
                ..Default::default()
            },
            false,
            false,
            MemberRole::Spectator,
        );
        assert_eq!(buf[22 * 800 + 103], digit_ink(0), "Ch.01 tens well");
        assert_eq!(buf[22 * 800 + 120], digit_ink(1), "Ch.01 ones well");
        assert_eq!(buf[22 * 800 + 162], digit_ink(0), "No.000 hundreds");
        assert_eq!(buf[22 * 800 + 179], digit_ink(0), "No.000 tens");
        assert_eq!(buf[22 * 800 + 196], digit_ink(0), "No.000 ones");
    }

    #[test]
    fn waiting_header_class_chips_stay_unpainted_without_a_source() {
        // Live caller chain: same as
        // `waiting_header_wells_show_one_wire_digit_each`.
        // p343 exe frame has neither the premium nor the free-room
        // caption; both staged bodies used to stack unconditionally.
        let mut placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_COVERAVATAR1",
            Some("Wait_Cover_Avatar.ojs"),
            (10, 200, 151, 375),
        )];
        for (id, rect) in [
            ("IDC_IMAGE_SWAIT_SKILLSHOW", (407, 13, 454, 39)),
            ("IDC_IMAGE_SWAIT_FREEUSER", (571, 13, 770, 39)),
        ] {
            placements.push(waiting_probe(id, Some("Wait_Premium.ojs"), rect));
        }
        let art = SceneArt {
            placements,
            figures: HashMap::from([
                ("Wait_Cover_Avatar.ojs".to_owned(), red_figure()),
                ("Wait_Premium.ojs".to_owned(), red_figure()),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        let red = pack_xrgb(255, 0, 0);
        assert_ne!(buf[13 * 800 + 407], red, "premium chip stays hidden");
        assert_ne!(buf[13 * 800 + 571], red, "free-room chip stays hidden");
    }

    #[test]
    fn guest_sees_the_wire_master_on_the_left_cover() {
        // Live caller chain:
        //   main.rs::run_play -> setup_online -> OnlineSession::paint
        //   -> ScreenGame::paint -> ScreenGame::paint_waiting
        //   -> paint_waiting_scene -> staged_seats.
        // p343 wire body: member_id 1, slot 0 t207probe master/ready,
        // slot 1 tester. The exe crop puts the master on cover 0.
        let mut room = waiting_room();
        room.member_id = 1;
        let mut guest = room.slots[0].member.clone().unwrap();
        guest.master = false;
        room.slots[1] = RoomSlot {
            index: 1,
            state_code: 1,
            member: Some(guest),
        };
        let seats = staged_seats(&room, None, false);
        assert_eq!(seats[0].name, "host", "slot 0 stays on the left");
        assert!(seats[0].master, "the slot-0 master keeps its star");
        assert!(seats[0].ready, "the slot-0 READY plate stays left");
        assert_eq!(seats[1].name, "host", "the guest slot follows in order");
        assert!(!seats[1].master, "no duplicate master emblem");
    }

    #[test]
    fn guest_self_seat_lands_at_its_member_id_cover() {
        // Live caller chain:
        //   main.rs::run_play -> setup_online -> OnlineSession::paint
        //   -> ScreenGame::paint -> ScreenGame::paint_waiting
        //   -> paint_waiting_scene -> staged_seats.
        // p343 snapshot: member_id 1, slot 0 occupied by the master;
        // the guest's own cover is slot 1, not cover 0.
        let mut room = waiting_room();
        room.member_id = 1;
        let seats = staged_seats(&room, Some("tester"), false);
        assert_eq!(seats[0].name, "host", "master keeps cover 0");
        assert_eq!(seats[1].name, "tester", "self uses cover member_id");
        assert!(seats[1].present, "the self cover is occupied");
        assert!(!seats[1].master, "no duplicate master emblem");
    }

    /// Staged waiting art holding one control pair at the same rect:
    /// the first id paints red, the second green (staging order is
    /// paint order, so the second wins wherever both survive).
    fn waiting_role_pair_art(ids: [&str; 2]) -> SceneArt {
        let place = |id: &str, sprite: &str| ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code: 0,
            sprite: Some(sprite.to_owned()),
            rect: Rect {
                x: 10,
                y: 10,
                w: 4,
                h: 4,
            },
        };
        SceneArt {
            placements: vec![place(ids[0], "Role_A.ojs"), place(ids[1], "Role_B.ojs")],
            figures: HashMap::from([
                ("Role_A.ojs".to_owned(), solid_figure(1, 1, [0x00, 0x7C])),
                ("Role_B.ojs".to_owned(), solid_figure(1, 1, [0xE0, 0x03])),
            ]),
        }
    }

    /// Pixel at the shared control rect origin after painting the
    /// waiting scene as `member_id`.
    fn waiting_role_pixel(art: &SceneArt, member_id: u8) -> u32 {
        let mut room = waiting_room();
        room.member_id = member_id;
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &room,
            art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        buf[10 * 800 + 10]
    }

    #[test]
    fn waiting_master_paints_start_while_guest_paints_ready() {
        // docs/room-enums.md "Waiting master gate": the master arm
        // enables START 0x16202012 (file 0xae3d3) and the non-master
        // arm enables READY 0x16202013 (file 0xae835), while both
        // controls stay staged at one rect.
        let art = waiting_role_pair_art(["IDC_BUTTON_SWAIT_START", "IDC_BUTTON_SWAIT_READY"]);
        assert_eq!(
            waiting_role_pixel(&art, 0),
            pack_xrgb(255, 0, 0),
            "master keeps START"
        );
        assert_eq!(
            waiting_role_pixel(&art, 1),
            pack_xrgb(0, 255, 0),
            "guest keeps READY"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn waiting_master_paints_no_inactive_dress_over_the_confirm_rect() {
        // The master arm frame-resets both inactive-dress overlays
        // (fetch file 0xae796/0xae7cb, reset at file 0xae7c6/0xae7fb).
        // The state-3 guest branch shows both (docs/room-enums.md
        // "Waiting master gate"), so a guest keeps the overlay over
        // READY at rest (#883).
        for overlay in [
            "IDC_IMAGE_SWAIT_READYBUTTEONINACTIVE",
            "IDC_IMAGE_SWAIT_SPEEDBUTTEONINACTIVE",
        ] {
            let art = waiting_role_pair_art(["IDC_BUTTON_SWAIT_START", overlay]);
            assert_eq!(
                waiting_role_pixel(&art, 0),
                pack_xrgb(255, 0, 0),
                "{overlay} must not cover START for the master"
            );
            assert_eq!(
                waiting_role_pixel(&art, 1),
                pack_xrgb(0, 255, 0),
                "{overlay} stays for the state-3 guest"
            );
        }
    }

    /// Staged waiting art holding one confirm control at (10,10) whose
    /// body carries `records` distinct 1x1 records: record n paints the
    /// `n`-th word (red, green, blue).
    fn waiting_confirm_art(id: &str, records: &[[u8; 2]]) -> SceneArt {
        let mut body = AvatarFigure::default();
        for word in records {
            body.push_layer(FigureLayer {
                image: Rgb555Image::decode(word, 1, 1).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: id.to_owned(),
                code: 0,
                sprite: Some("Wait_Start.ojs".to_owned()),
                rect: Rect {
                    x: 10,
                    y: 10,
                    w: 4,
                    h: 4,
                },
            }],
            figures: HashMap::from([("Wait_Start.ojs".to_owned(), body)]),
        }
    }

    /// Pixel at the confirm rect origin after painting the waiting
    /// scene as `member_id` with the face `confirm_armed`.
    fn waiting_confirm_pixel(art: &SceneArt, member_id: u8, confirm_armed: bool) -> u32 {
        let mut room = waiting_room();
        room.member_id = member_id;
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &room,
            art,
            &WaitingCaptions::default(),
            confirm_armed,
            false,
            MemberRole::Spectator,
        );
        buf[10 * 800 + 10]
    }

    #[test]
    fn waiting_confirm_face_paints_the_armed_record_for_either_role() {
        // #284: `Wait_Start.ojs`/`Wait_Ready.ojs` carry 3 raw records and
        // the exe paints exactly one frame of each — the rest record, or
        // the third record (absolute frame 3, kind-`0x2000` dispatch arm
        // file `0x4f6b7c`) while the local request stands. Stacking them
        // would paint the armed face over the rest face forever.
        let records = [[0x00, 0x7C], [0xE0, 0x03], [0x1F, 0x00]];
        for (member_id, confirm) in [
            (0u8, "IDC_BUTTON_SWAIT_START"),
            (1, "IDC_BUTTON_SWAIT_READY"),
        ] {
            let art = waiting_confirm_art(confirm, &records);
            assert_eq!(
                waiting_confirm_pixel(&art, member_id, false),
                pack_xrgb(255, 0, 0),
                "{confirm} at rest paints its first record"
            );
            assert_eq!(
                waiting_confirm_pixel(&art, member_id, true),
                pack_xrgb(0, 0, 255),
                "{confirm} armed paints its third record"
            );
        }
    }

    #[test]
    fn waiting_confirm_face_keeps_the_rest_record_without_pressed_art() {
        // A body staged without a third record keeps its rest record
        // while armed: never a blank rect and never a stand-in face.
        let art = waiting_confirm_art("IDC_BUTTON_SWAIT_START", &[[0x00, 0x7C]]);
        assert_eq!(
            waiting_confirm_pixel(&art, 0, true),
            pack_xrgb(255, 0, 0),
            "a single-record body stays on its rest record"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn waiting_confirm_face_leaves_the_other_role_control_unpainted() {
        // The role filter stays the single home for which control shows
        // (`waiting_control_armed_for_role`): arming the local face never
        // lights the control the other role owns.
        let records = [[0x00, 0x7C], [0xE0, 0x03], [0x1F, 0x00]];
        let art = waiting_confirm_art("IDC_BUTTON_SWAIT_READY", &records);
        assert_eq!(
            waiting_confirm_pixel(&art, 0, true),
            0,
            "a master never paints the guest's READY body"
        );
    }

    #[test]
    fn waiting_paints_ready_dots_for_seats() {
        let room = waiting_room();
        let mut buf = vec![0u32; 80 * 64];
        paint_waiting_members(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            &room,
        );
        let waiting_bg = background_of(Screen::Waiting);
        let ready_green = pack_xrgb(0x30, 0xE0, 0x30);
        assert_eq!(buf[0], waiting_bg);
        // Slot 0 (ready wire host) sits in column 0, row 0 of the
        // 4x2 grid; the guest's own slot is not painted first.
        let (host_x, host_y) = seat_origin(0, 80, 64);
        assert_eq!((host_x, host_y), (5, 16));
        assert_eq!(
            buf[(host_y + 1) as usize * 80 + (host_x - 6 + 1) as usize],
            ready_green
        );
        // Slot 1 is empty in this snapshot: no second ready dot.
        let (empty_x, empty_y) = seat_origin(1, 80, 64);
        assert_ne!(
            buf[(empty_y + 1) as usize * 80 + (empty_x - 6 + 1) as usize],
            ready_green,
            "empty slot stays unready"
        );
        // Seat 2 (empty) has no dot: the seat scrim shows through.
        let (empty_x, empty_y) = seat_origin(2, 80, 64);
        assert_eq!(
            buf[(empty_y + 1) as usize * 80 + (empty_x - 6 + 1) as usize],
            scrim_color()
        );
    }

    #[test]
    fn waiting_seat_grid_is_four_columns_by_two_rows() {
        // Wine `ab-wait.png` (+ `ad-wait.png`, `z-wait.png` siblings):
        // seats 0..3 share the top row left to right, seats 4..7 share
        // the bottom row. The old 2-column by 4-row fallback stacked
        // seat 2 under seat 0; it must sit right of seat 1 instead.
        let top: Vec<(i32, i32)> = (0..4).map(|i| seat_origin(i, 80, 64)).collect();
        assert!(top[0].0 < top[1].0 && top[1].0 < top[2].0 && top[2].0 < top[3].0);
        assert!(top.iter().all(|(_, y)| *y == top[0].1));
        assert_eq!(seat_origin(4, 80, 64).0, seat_origin(0, 80, 64).0);
        assert!(seat_origin(4, 80, 64).1 > top[0].1);
        assert_eq!(seat_origin(7, 80, 64).0, seat_origin(3, 80, 64).0);
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

    fn scene_control_text() -> String {
        [
            "NUMBER_OF_STATE 0x03",
            "STATE_CHANNEL 0x02",
            "{",
            "BOUND",
            "IDC_BAR 0X00100000",
            "IDC_SELECT 0X10500001 \"Dialog_ChannelSelectBar.ojs\"",
            "}",
            "STATE_ROOM 0x02",
            "{",
            "BOUND \"State_Room.bnd\"",
            "IDC_ROOM 0X00100000",
            "IDC_LIST 0X10500001 \"Room_List.ojs\"",
            "}",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn main_sprites_name_channel_dialog_art() {
        let list = o2jam_file::parse_control_list(&scene_control_text()).unwrap();
        assert_eq!(
            main_scene_sprites(&list, Screen::Channel),
            vec!["Dialog_ChannelSelectBar.ojs".to_owned()]
        );
        assert_eq!(
            main_scene_sprites(&list, Screen::Room),
            vec!["Room_List.ojs".to_owned()]
        );
    }

    #[test]
    fn main_sprites_fall_back_to_single_art() {
        let list = o2jam_file::parse_control_list(&scene_control_text()).unwrap();
        assert!(
            main_scene_sprites(&list, Screen::Avi).is_empty(),
            "the splash names its block entries: no block, no art"
        );
        assert_eq!(
            main_scene_sprites(&list, Screen::Loading),
            vec![LOADING_SPRITE.to_owned()]
        );
        assert!(main_scene_sprites(&list, Screen::Playing).is_empty());
        assert!(main_scene_sprites(&list, Screen::Result).is_empty());
    }

    /// One 1x1 pixel figure at a native anchor (colorkey white).
    fn anchored_pixel(dx: i32, dy: i32, bytes: &[u8]) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(bytes, 1, 1).unwrap(),
            dx,
            dy,
            colorkey: 0xFFFF,
        });
        figure
    }

    #[test]
    fn playing_overlay_layers_bound_panels_over_backdrop() {
        // Backdrop-first file order: the bound button pixel wins
        // the shared menu-strip pixel, the strip survives beside
        // it, and the lane pixel keeps its highway ink.
        let mut menu = AvatarFigure::default();
        for dx in [750, 751] {
            menu.push_layer(FigureLayer {
                image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
                dx,
                dy: 550,
                colorkey: 0xFFFF,
            });
        }
        let art = SceneArt {
            placements: vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_IMAGE_SPLAY_BACK3".to_owned(),
                    code: 0,
                    sprite: Some("Playing_Menu.ojs".to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 0,
                        h: 0,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_BUTTON_SPLAY_EXIT".to_owned(),
                    code: 0,
                    sprite: Some("Playing_Exit.ojs".to_owned()),
                    rect: Rect {
                        x: 751,
                        y: 550,
                        w: 10,
                        h: 10,
                    },
                },
            ],
            figures: HashMap::from([
                ("Playing_Menu.ojs".to_owned(), menu),
                (
                    "Playing_Exit.ojs".to_owned(),
                    anchored_pixel(751, 550, &[0x1F, 0x00]),
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        assert_eq!(buf[550 * 800 + 750], pack_xrgb(255, 0, 0));
        assert_eq!(buf[550 * 800 + 751], pack_xrgb(0, 0, 255));
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn playing_overlay_skips_panels_without_figures() {
        // A named-but-undecodable panel paints nothing: the highway
        // ink underneath survives.
        let art = SceneArt {
            placements: unbound_scene_placements(&["Missing.ojs".to_owned()]),
            figures: HashMap::new(),
        };
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[0], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    /// Behind scene with red pixels across the frame: one in a lane
    /// column (10,100), the venue body box (204,100), and the former
    /// HUD-zone cutouts — judgement band row (400,447), peer-stack
    /// corner (760,4), jam-bar row (600,540) — so the exe-order rule
    /// is observable: only lane columns stay clear.
    fn venue_backdrop_art() -> SceneArt {
        let mut venue = AvatarFigure::default();
        for (dx, dy) in [(10, 100), (204, 100), (400, 447), (760, 4), (600, 540)] {
            venue.push_layer(FigureLayer {
                image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
                dx,
                dy,
                colorkey: 0xFFFF,
            });
        }
        SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_IMAGE_SPLAY_BACK1".to_owned(),
                code: 0,
                sprite: Some("Playing_BG.ojs".to_owned()),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
            }],
            figures: HashMap::from([("Playing_BG.ojs".to_owned(), venue)]),
        }
    }

    #[test]
    fn playing_overlay_keeps_only_the_lanes_clear_of_behind_scene() {
        // Exe paint order: the backdrop sits at the bottom (applier
        // `0x4e7850`, `BACK1` first), so no HUD cutouts remain — the
        // art fills the former score/judgement/peer/jam zones while
        // the lane column alone keeps its highway ink.
        let art = venue_backdrop_art();
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        let highway = pack_xrgb(0x0A, 0x0A, 0x0A);
        assert_eq!(buf[100 * 800 + 10], highway);
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(255, 0, 0));
        assert_eq!(buf[447 * 800 + 400], pack_xrgb(255, 0, 0));
        assert_eq!(buf[4 * 800 + 760], pack_xrgb(255, 0, 0));
        assert_eq!(buf[540 * 800 + 600], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn playing_backdrop_paints_unclipped_before_highway() {
        // The pre-highway API paints the whole scene body with no
        // exclusions: lane and venue pixels turn red alike, and the
        // highway shell covers every one of them after.
        let art = venue_backdrop_art();
        let mut buf = vec![0u32; 800 * 600];
        paint_playing_backdrop(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(255, 0, 0));
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(255, 0, 0));
        assert_eq!(buf[447 * 800 + 400], pack_xrgb(255, 0, 0));
        PlayingView::default().paint(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &HighwayGeometry::centered(800, 600),
        );
        assert_eq!(buf[100 * 800 + 10], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(0x0A, 0x0A, 0x0A));
        assert_eq!(buf[100 * 800 + 400], pack_xrgb(0x0A, 0x0A, 0x0A));
    }

    #[test]
    fn playing_backdrop_paints_the_staged_arena_theme_placement() {
        // The themed placement reaches the same behind-scene painter
        // as the numbered venue: the staged figure's third frame is
        // the top layer at the staged venue anchor (#848).
        use crate::avatar::PLAYING_VENUE_THEME_SPRITE;

        let mut theme = AvatarFigure::default();
        for color in [0x7C00u16, 0x03E0, 0x001F] {
            theme.push_layer(FigureLayer {
                image: Rgb555Image::decode(&color.to_le_bytes(), 1, 1).unwrap(),
                dx: 204,
                dy: 100,
                colorkey: 0xFFFF,
            });
        }
        let mut art = venue_backdrop_art();
        art.placements[0].sprite = Some(PLAYING_VENUE_THEME_SPRITE.to_owned());
        art.figures.insert(
            PLAYING_VENUE_THEME_SPRITE.to_owned(),
            theme.layers_through(2),
        );
        let mut buf = vec![0u32; 800 * 600];
        paint_playing_backdrop(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        assert_eq!(
            buf[100 * 800 + 204],
            pack_xrgb(0, 0, 255),
            "the staged arena theme frame paints as a behind placement"
        );
    }

    #[test]
    fn playing_overlay_layers_gauges_over_behind_scene() {
        // Bound gauges keep their top slot: the shared strip pixel
        // shows the gauge ink while the scene survives beside it.
        let mut art = venue_backdrop_art();
        art.placements.push(ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_SPLAY_EXIT".to_owned(),
            code: 0,
            sprite: Some("Playing_Exit.ojs".to_owned()),
            rect: Rect {
                x: 400,
                y: 100,
                w: 10,
                h: 10,
            },
        });
        art.figures.insert(
            "Playing_Exit.ojs".to_owned(),
            anchored_pixel(400, 100, &[0x1F, 0x00]),
        );
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        PlayingView::default().paint(&mut frame, &HighwayGeometry::centered(800, 600));
        paint_playing_overlay(&mut frame, &art);
        assert_eq!(buf[100 * 800 + 400], pack_xrgb(0, 0, 255));
        assert_eq!(buf[100 * 800 + 204], pack_xrgb(255, 0, 0));
    }

    #[test]
    fn avi_scene_shows_logo_without_stamp() {
        // Corner art paints the logo pixel; the stamp row that the
        // flat splash whites out stays unpainted (blank canvas)
        // under staged art.
        let buf = paint_art(Screen::Avi, &corner_art(), 32, 24);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 32 + 11], 0);
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
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
    fn channel_scene_layers_list_over_art() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn channel_flanks_paint_the_selected_promo_frame_not_the_stack_top() {
        // The exe mounts ONE banner frame in the planet image control
        // (refresh arm VA `0x48e4b0`); the channel dialog opens over that
        // same board, so its flanks must show that frame, never the last
        // raw layer of the three-frame body.
        use super::super::side::PLANET_PROMO_IMAGE_CODE;
        let sprite = "Planet_Promotion.ojs";
        let mut banner = AvatarFigure::default();
        for word in [0x7C00u16, 0x03E0, 0x001F] {
            banner.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.to_le_bytes().repeat(16), 4, 4).unwrap(),
                dx: 49,
                dy: 163,
                colorkey: 0xFFFF,
            });
        }
        let board = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_PROMO_IMAGE_CODE,
                sprite: Some(sprite.to_owned()),
                rect: Rect {
                    x: 49,
                    y: 163,
                    w: 701,
                    h: 333,
                },
            }],
            figures: HashMap::from([(sprite.to_owned(), banner)]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            0,
            0,
            &SceneArt {
                placements: Vec::new(),
                figures: HashMap::new(),
            },
            Some(StagedPlanetBoard {
                art: &board,
                promo: PlanetPromoPaint::spinning(1),
            }),
            None,
        );
        assert_eq!(
            buf[164 * 800 + 50],
            pack_xrgb(0xFF, 0x00, 0x00),
            "the channel board must show the carousel-selected first frame"
        );
    }

    /// One wire 1003 row; the fields the label must NOT read are
    /// parameterized so the pinned-source guard can move them alone.
    fn channel_row(
        gateway_id: u16,
        channel_id: u16,
        capacity: i32,
        population: i32,
        active: bool,
    ) -> ChannelState {
        ChannelState {
            gateway_id,
            channel_id,
            capacity,
            population,
            active,
        }
    }

    #[test]
    fn channel_row_label_paints_the_pinned_value_source() {
        // Claim D25 (#92): the label is the wire channel_id one-based
        // in the exe's `Ch. NN` reading — never the gateway id, the
        // population, or the capacity (a non-full row). Ids past 98
        // wrap to their low two digits, matching the digit wells.
        let open = channel_row(1, 2, 100, 5, true);
        assert_eq!(channel_row_text(&open), "Ch. 03");
        let other_gateway = channel_row(7, 2, 100, 5, true);
        assert_eq!(channel_row_text(&other_gateway), "Ch. 03");
        let busy = channel_row(1, 2, 100, 80, true);
        assert_eq!(channel_row_text(&busy), "Ch. 03");
        let wrapped = channel_row(0, 99, 100, 0, true);
        assert_eq!(channel_row_text(&wrapped), "Ch. 00");
    }

    #[test]
    fn channel_row_label_marks_full_rows() {
        // FULL rows carry the ` / FULL` suffix (the baked CH_Back
        // frame-2 face is the staged-path equivalent); the open row
        // with the same id does not.
        let full = channel_row(0, 9, 100, 100, true);
        assert_eq!(channel_row_text(&full), "Ch. 10 / FULL");
        let open = channel_row(0, 9, 100, 99, true);
        assert_eq!(channel_row_text(&open), "Ch. 10");
        // The pinned FULL rule: zero capacity never reads full.
        let zero_capacity = channel_row(0, 9, 0, 0, true);
        assert_eq!(channel_row_text(&zero_capacity), "Ch. 10");
    }

    #[test]
    fn channel_rows_dim_inactive_channels() {
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_list(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_channels(),
            0,
            1,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(buf.contains(&pack_xrgb(0x70, 0x70, 0x70)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    fn two_rooms() -> Vec<RoomListEntry> {
        vec![RoomListEntry {
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
        }]
    }

    /// One staged row panel plus its password clone: the live room
    /// painter paints the clone's sole body layer at the clone rect.
    fn room_password_art() -> SceneArt {
        let panel = |id: &str, x: i32| ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code: 0x1301_201B,
            sprite: None,
            rect: Rect {
                x,
                y: 82,
                w: 288,
                h: 91,
            },
        };
        let padlock = |id: &str, x: i32| ScenePlacement {
            role: PlacementRole::Clone,
            id: id.to_owned(),
            code: 0x1350_002C,
            sprite: Some(ROOM_PASSWORD_SPRITE.to_owned()),
            rect: Rect {
                x,
                y: 140,
                w: 15,
                h: 16,
            },
        };
        SceneArt {
            placements: vec![
                panel("IDC_BUTTON_SROOM_EXBUTTON1", 26),
                panel("IDC_BUTTON_SROOM_EXBUTTON2", 317),
                padlock("IDC_BUTTON_SROOM_EXBUTTON1", 200),
                padlock("IDC_BUTTON_SROOM_EXBUTTON2", 500),
            ],
            figures: HashMap::from([(ROOM_PASSWORD_SPRITE.to_owned(), red_figure())]),
        }
    }

    #[test]
    fn unlocked_room_row_hides_the_padlock_but_locked_room_row_shows_it() {
        // #700: the exe's flag=0 row (frame 30cac4c5..., 2005 count 1)
        // has no padlock; flag=1 stages the one-image body. Both arms
        // run through ScreenGame::paint_room → paint_room_scene_with_hover;
        // staged art drives that same painter without a live server.
        let unlocked = RoomListEntry {
            number: 1,
            has_password: false,
            ..two_rooms()[0].clone()
        };
        let locked = RoomListEntry {
            number: 2,
            has_password: true,
            ..unlocked.clone()
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_room_scene_with_hover(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[unlocked, locked],
            0,
            &room_password_art(),
            None,
            None,
        );
        assert_eq!(
            buf[140 * 800 + 200],
            0,
            "unlocked flag leaves the padlock site unpainted"
        );
        assert_eq!(
            buf[140 * 800 + 500],
            pack_xrgb(255, 0, 0),
            "locked flag paints the padlock body"
        );
    }

    #[test]
    fn room_rows_anchor_to_staged_panels_in_two_columns() {
        // Staged `EXBUTTON` panels (`State_Room.bnd` rects 26/27, the
        // left column top two) position the rows; the legacy
        // quarter-frame anchor paints nowhere on the staged path.
        // The `ROOMNAME`/`ROOM_USER` template rects size the split
        // title and count fields (rows carry no combined string).
        use o2jam_render::Rect;
        let template = |id: &str, code: u32, x: i32, y: i32, w: u32, h: u32| ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        };
        let placements = vec![
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
                code: 0x1301_201B,
                sprite: None,
                rect: Rect {
                    x: 26,
                    y: 82,
                    w: 288,
                    h: 91,
                },
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_EXBUTTON2".to_owned(),
                code: 0x1301_201C,
                sprite: None,
                rect: Rect {
                    x: 26,
                    y: 177,
                    w: 286,
                    h: 91,
                },
            },
            template("IDC_TEXT_SROOM_ROOMNAME", 0x1340_0021, 66, 13, 158, 16),
            template("IDC_TEXT_SROOM_ROOM_USER", 0x1340_0022, 226, 13, 32, 16),
        ];
        let art = SceneArt {
            placements,
            figures: HashMap::new(),
        };
        let rooms = vec![
            RoomListEntry {
                number: 1,
                state_code: 1,
                title: "Alpha".to_owned(),
                has_password: false,
                music_id: 0,
                difficulty: 0,
                key_mode: 0,
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
            RoomListEntry {
                number: 2,
                state_code: 1,
                title: "Beta".to_owned(),
                has_password: false,
                music_id: 0,
                difficulty: 0,
                key_mode: 0,
                game_mode: 0,
                speed: 1,
                capacity: 8,
                user_count: 2,
                min_level: 0,
                max_level: 0,
                skills: vec![],
                premium: false,
                room_type: 0,
            },
        ];
        let w: usize = 800;
        let h: usize = 600;
        let mut buf = vec![0u32; w * h];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            &rooms,
            1,
            &art,
            None,
        );
        let window_has_ink = |x: i32, y: i32, ww: i32, hh: i32, ink: u32| -> bool {
            for dy in 0..hh {
                for dx in 0..ww {
                    let px = x + dx;
                    let py = y + dy;
                    let (Some(px), Some(py)) = (usize::try_from(px).ok(), usize::try_from(py).ok())
                    else {
                        continue;
                    };
                    if px < w && py < h && buf[py * w + px] == ink {
                        return true;
                    }
                }
            }
            false
        };
        let white = pack_xrgb(0xFF, 0xFF, 0xFF);
        let amber = pack_xrgb(0xFF, 0xD0, 0x20);
        // Exe-measured 2026-11-09: the split row fields anchor at
        // panel origin +12/+12 (number), +64/+11 (title), +194/+10
        // (count) — panel 0 (26,82) puts the number glyphs at (38,94).
        assert!(
            window_has_ink(38, 94, 90, 10, white),
            "first room inks its staged left-top panel"
        );
        assert!(
            window_has_ink(26, 189, 90, 10, amber),
            "highlight cursor rides the staged second panel (number origin)"
        );
        assert!(
            !window_has_ink(200, 150, 60, 6, white),
            "legacy quarter-frame anchor paints nothing staged"
        );
    }

    #[test]
    fn room_rows_split_number_title_and_count_to_pinned_offsets() {
        // Pinned live splits (`~/scratch/geom-open-close/findings.md`
        // §2): the number at panel +(12,12), the title at +(64,+11),
        // the seat count at +(194,+10) — the old combined string
        // spanned from the number origin, so the count zone stayed
        // empty, title ink crowded the number, and caps stood 8 px
        // tall instead of the live ~11 px.
        use o2jam_render::Rect;
        let placements = vec![
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
                code: 0x1301_201B,
                sprite: None,
                rect: Rect {
                    x: 26,
                    y: 82,
                    w: 288,
                    h: 91,
                },
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_TEXT_SROOM_ROOMNAME".to_owned(),
                code: 0x1340_0021,
                sprite: None,
                rect: Rect {
                    x: 66,
                    y: 13,
                    w: 158,
                    h: 16,
                },
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_TEXT_SROOM_ROOM_USER".to_owned(),
                code: 0x1340_0022,
                sprite: None,
                rect: Rect {
                    x: 226,
                    y: 13,
                    w: 32,
                    h: 16,
                },
            },
        ];
        let art = SceneArt {
            placements,
            figures: HashMap::new(),
        };
        let w: usize = 800;
        let h: usize = 600;
        let mut buf = vec![0u32; w * h];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            &two_rooms(),
            0,
            &art,
            None,
        );
        let white = pack_xrgb(0xFF, 0xFF, 0xFF);
        let window_has_ink = |x: i32, y: i32, ww: i32, hh: i32| -> bool {
            (y..y + hh).any(|py| {
                (x..x + ww).any(|px| {
                    usize::try_from(px).is_ok_and(|px| {
                        usize::try_from(py)
                            .is_ok_and(|py| px < w && py < h && near_xrgb(buf[py * w + px], white))
                    })
                })
            })
        };
        assert!(
            window_has_ink(220, 92, 40, 16),
            "seat count inks its pinned field (panel +194/+10)"
        );
        assert!(
            !window_has_ink(56, 94, 30, 10),
            "title vacates the number's run (combined string crowded here)"
        );
        assert!(
            window_has_ink(90, 102, 22, 4),
            "title caps reach past micro-font height at real size"
        );
    }

    #[test]
    fn room_scene_layers_rows_over_art() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 48];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_rooms(),
            0,
            &art,
            None,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    /// Room fixture with every detail field set to a distinct value.
    fn detailed_room() -> RoomListEntry {
        RoomListEntry {
            number: 3,
            state_code: 1,
            title: "Jam".to_owned(),
            has_password: true,
            music_id: 757,
            difficulty: 2,
            key_mode: 1,
            game_mode: 1,
            speed: 3,
            capacity: 8,
            user_count: 5,
            min_level: 10,
            max_level: 99,
            skills: vec![],
            premium: false,
            room_type: 0,
        }
    }

    #[test]
    fn room_mode_names_each_game_mode() {
        assert_eq!(room_mode_label(0), "NORMAL");
        assert_eq!(room_mode_label(1), "LIVE");
        assert_eq!(room_mode_label(9), "MODE 9");
    }

    #[test]
    fn room_music_shows_song_id() {
        assert_eq!(room_music_label(757), "M757");
        assert_eq!(room_music_label(0), "M0");
    }

    #[test]
    fn room_difficulty_names_each_band() {
        assert_eq!(room_difficulty_label(0), "EX");
        assert_eq!(room_difficulty_label(1), "NX");
        assert_eq!(room_difficulty_label(2), "HX");
        assert_eq!(room_difficulty_label(3), "RX");
        assert_eq!(room_difficulty_label(9), "DIF 9");
    }

    #[test]
    fn room_key_names_each_key_mode() {
        assert_eq!(room_key_label(0), "3KEY");
        assert_eq!(room_key_label(1), "5KEY");
        assert_eq!(room_key_label(2), "7KEY");
        assert_eq!(room_key_label(7), "KEY 7");
    }

    #[test]
    fn room_speed_names_each_step() {
        let expected = [
            "X0.5", "X1", "X1.5", "X2", "X2.5", "X3", "X3.5", "X4", "X4.5", "X5", "X6", "XR",
        ];
        for (code, label) in expected.iter().enumerate() {
            assert_eq!(&room_speed_label(code as u8), label);
        }
        assert_eq!(room_speed_label(12), "SPD 12");
    }

    #[test]
    fn room_capacity_shows_seats_over_capacity() {
        assert_eq!(room_capacity_label(&detailed_room()), "5/8");
    }

    #[test]
    fn room_level_shows_min_to_max_gate() {
        assert_eq!(room_level_label(&detailed_room()), "LV 10-99");
    }

    #[test]
    fn room_lock_names_password_state() {
        assert_eq!(room_lock_label(true), "LOCK");
        assert_eq!(room_lock_label(false), "OPEN");
        assert_eq!(
            room_lock_ink(true),
            pack_xrgb(0xFF, 0x50, 0x50),
            "locked line reads red"
        );
    }

    #[test]
    fn room_detail_entries_cover_every_panel_line() {
        let entries = room_detail_entries(&detailed_room());
        assert_eq!(entries.len(), ROOM_DETAIL_LINES);
        let ids: Vec<&str> = entries.iter().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "each line anchors to its own rect");
        let by_id = |want: &str| {
            entries
                .iter()
                .find(|(id, _)| *id == want)
                .map(|(_, text)| text.clone())
                .unwrap()
        };
        assert_eq!(by_id(ROOM_DETAIL_TITLE_ID), "Jam");
        assert_eq!(by_id(ROOM_DETAIL_USERS_ID), "5/8");
        assert_eq!(by_id(ROOM_DETAIL_MUSIC_ID), "M757");
        assert_eq!(by_id(ROOM_DETAIL_DIFFICULTY_ID), "HX");
        assert_eq!(by_id(ROOM_DETAIL_KEY_ID), "5KEY");
        assert_eq!(by_id(ROOM_DETAIL_MODE_ID), "LIVE");
        assert_eq!(by_id(ROOM_DETAIL_SPEED_ID), "X2");
        assert_eq!(by_id(ROOM_DETAIL_LEVEL_ID), "LV 10-99");
        assert_eq!(by_id(ROOM_DETAIL_LOCK_ID), "LOCK");
    }

    /// Placements pinning the title rect apart from the rest: the
    /// title lands at native (80, 60), every other line at native
    /// (720, 540).
    fn detail_placements() -> Vec<ScenePlacement> {
        let title = ScenePlacement {
            role: PlacementRole::Single,
            id: ROOM_DETAIL_TITLE_ID.to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 80,
                y: 60,
                w: 80,
                h: 60,
            },
        };
        let rest = room_detail_entries(&detailed_room())
            .iter()
            .filter(|(id, _)| *id != ROOM_DETAIL_TITLE_ID)
            .map(|(id, _)| ScenePlacement {
                role: PlacementRole::Single,
                id: (*id).to_owned(),
                code: 0,
                sprite: None,
                rect: Rect {
                    x: 720,
                    y: 540,
                    w: 80,
                    h: 60,
                },
            })
            .collect::<Vec<_>>();
        std::iter::once(title).chain(rest).collect()
    }

    #[test]
    fn room_details_anchor_to_bnd_rects() {
        // Native frame with the panel at the frame origin: the song
        // field inks at panel + template offset — native (67, 35) —
        // while the row-owned title paints nothing in the details
        // (its template rect stays a row-field size).
        let mut placements = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 0,
                y: 0,
                w: 288,
                h: 91,
            },
        }];
        // The song field rides its own template rect (the shared
        // helper parks every other non-title line at (720, 540)) —
        // staged first so the lookup finds it.
        placements.push(ScenePlacement {
            role: PlacementRole::Single,
            id: ROOM_DETAIL_MUSIC_ID.to_owned(),
            code: 0,
            sprite: None,
            rect: Rect {
                x: 67,
                y: 35,
                w: 205,
                h: 19,
            },
        });
        placements.extend(detail_placements());
        let rooms = vec![detailed_room()];
        let art = SceneArt {
            placements,
            figures: HashMap::new(),
        };
        let w: usize = 800;
        let h: usize = 600;
        let mut buf = vec![0u32; w * h];
        paint_room_details(
            &mut Frame {
                buf: &mut buf,
                w,
                h,
            },
            &rooms,
            0,
            Some(&art),
        );
        let white = art_title_ink();
        let window_has_ink = |x: i32, y: i32, ww: i32, hh: i32| -> bool {
            (y..y + hh).any(|py| {
                (x..x + ww).any(|px| {
                    usize::try_from(px).is_ok_and(|px| {
                        usize::try_from(py)
                            .is_ok_and(|py| px < w && py < h && near_xrgb(buf[py * w + px], white))
                    })
                })
            })
        };
        assert!(
            window_has_ink(67, 35, 70, 19),
            "song field inks its panel-relative template rect"
        );
        assert!(
            !window_has_ink(80, 60, 80, 10),
            "row-owned title paints nothing in the details"
        );
    }

    #[test]
    fn room_details_fall_back_under_list_without_staged_ids() {
        // Flat fallback stacks the first detail line under the one
        // list row: x 50, y 49, so its scrim opens at (46, 47) —
        // below the row ink (y 37..42) and the list scrim (y 35..47).
        let rooms = vec![detailed_room()];
        let mut buf = vec![0u32; 200 * 150];
        paint_room_list(
            &mut Frame {
                buf: &mut buf,
                w: 200,
                h: 150,
            },
            &rooms,
            0,
        );
        assert_eq!(buf[47 * 200 + 46], scrim_color());
        // List plus cursor keep working under the panel.
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
        // Locked room paints the red lock line.
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0x50, 0x50)));
    }

    #[test]
    fn room_details_stay_empty_without_a_room() {
        let mut buf = vec![0u32; 64 * 48];
        paint_room_list(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[],
            0,
        );
        assert_eq!(buf[0], background_of(Screen::Room));
        assert!(!buf.contains(&scrim_color()));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0x50, 0x50)));
        assert!(!buf.contains(&pack_xrgb(0x90, 0x90, 0x90)));
    }

    #[test]
    fn waiting_scene_keeps_ready_dots_over_art() {
        let art = corner_art();
        let room = waiting_room();
        let mut buf = vec![0u32; 80 * 64];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 80,
                h: 64,
            },
            &room,
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn loading_scene_hides_the_open_bar_over_art() {
        // OPEN: no exe bar at load-complete or mid-load, so neither
        // the green fill nor its dark track may reach the frame.
        let buf = paint_art(Screen::Loading, &corner_art(), 64, 48);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
        assert!(!buf.contains(&scrim_color()));
    }

    #[test]
    fn result_scene_keeps_small_frames_number_free() {
        // 64x48 is not the exe's native canvas, so the pinned GDI
        // numbers wait (frame_is_native): the staged art and the
        // avatar paint, no white digit ink anywhere.
        let art = corner_art();
        let score = o2jam_game::Score {
            points: 200,
            max_combo: 7,
            ..o2jam_game::Score::default()
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &ResultPaint {
                portrait_slots: None,
                score: &score,
                rows: &[],
                art: &art,
                verdict: ResultVerdict::Solo,
                grade: None,
                life_readout: None,
                bottom_counter: None,
            },
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(255, 0, 0)));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xFF, 0x40)));
    }

    #[test]
    fn playing_overlay_keeps_highway_marks() {
        let buf = paint_art(Screen::Playing, &corner_art(), 70, 60);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0xE0, 0xE0, 0xE0)));
    }

    #[test]
    fn empty_art_falls_back_to_flat() {
        // No figures staged anywhere: Avi paints nothing at all (no
        // logo, no stamp); lobbies keep only the readable title over
        // the untouched frame; loading keeps only the live bar; the
        // highway and result scenes keep their own painters. No staged
        // path invents a fill (`docs/channel-fill.md` no-fill rule).
        let art = SceneArt::default();
        assert!(!art.has_figures());
        let avi = paint_art(Screen::Avi, &art, 16, 12);
        assert!(avi.iter().all(|pixel| *pixel == 0));
        for screen in [Screen::Channel, Screen::Room, Screen::Waiting] {
            let buf = paint_art(screen, &art, 16, 12);
            assert_eq!(buf[0], 0, "{screen:?} paints no fill");
            assert!(
                !buf.iter().all(|pixel| *pixel == background_of(screen)),
                "{screen:?} paints no flat fill"
            );
            assert!(
                has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)),
                "{screen:?} keeps title ink"
            );
        }
        let loading = paint_art(Screen::Loading, &art, 16, 12);
        assert!(!loading.contains(&background_of(Screen::Loading)));
        assert!(!loading.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
        let playing = paint_art(Screen::Playing, &art, 16, 12);
        assert_eq!(playing[0], background_of(Screen::Playing));
        let result = paint_art(Screen::Result, &art, 16, 12);
        assert_eq!(result[0], background_of(Screen::Result));
    }

    fn battle_room() -> RoomTeams {
        let mut teams = RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.submit(0, 100_000);
        teams.submit(1, 150_000);
        teams
    }

    #[test]
    fn result_loader_names_top_bottom_and_winlose() {
        assert_eq!(
            result_loader_sprites(),
            [
                "StateResult_Top.ojs",
                "StateResult_Bottom.ojs",
                "Result_WinLose.ojs"
            ]
        );
        assert_eq!(RESULT_WINLOSE_SPRITE, "Result_WinLose.ojs");
        assert_eq!(WINNER_IMAGE_SPRITE, "Winner_image.ojs");
        assert_eq!(RESULT_RANK_SPRITE, "Result_Imgs_Rank.ojs");
    }

    #[test]
    fn battle_verdict_goes_to_rank_one_by_score() {
        let teams = battle_room();
        assert_eq!(teams.winner(), Some(1));
        assert_eq!(battle_verdict(&teams, Some(1)), Some(BattleVerdict::Win));
        assert_eq!(battle_verdict(&teams, Some(0)), Some(BattleVerdict::Lose));
    }

    #[test]
    fn battle_verdict_ranks_three_teams_to_other() {
        let mut teams = RoomTeams::new();
        teams.set_team(0, 0);
        teams.set_team(1, 1);
        teams.set_team(2, 2);
        teams.submit(0, 300_000);
        teams.submit(1, 200_000);
        teams.submit(2, 100_000);
        assert_eq!(battle_verdict(&teams, Some(0)), Some(BattleVerdict::Win));
        assert_eq!(battle_verdict(&teams, Some(1)), Some(BattleVerdict::Lose));
        assert_eq!(battle_verdict(&teams, Some(2)), Some(BattleVerdict::Other));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn battle_verdict_stays_empty_without_winner_or_self() {
        let empty = RoomTeams::new();
        assert_eq!(battle_verdict(&empty, Some(0)), None);
        let teams = battle_room();
        assert_eq!(battle_verdict(&teams, None), None);
    }

    // Pinned digit-sprite cells (dossier `rea-cresult-anchors`,
    // re-filed #86): the life readout and the bottom counter run
    // through the frame-select/draw pair over eight fixed 24 px
    // cells, least-significant digit rightmost, digit 1 kerned +5
    // (shared painter bodies at files `0xc44c0`/`0xc4630`). The
    // FONT BINDING is pinned: life readout <- `PointNum.ojs`
    // (`this+0x10d10`, painter VA `0x4c44c0`), bottom counter <-
    // `Result_NumFont_Blue.ojs` (`this+0x10d0c`, painter VA
    // `0x4c4630`, its sole use site). The value sources (globals
    // `0x77ee58`+`0x77ee60`, u16 `0x76fc66`) stay OPEN — the shell
    // paints a field only when a caller stages a value.

    // Row geometry (row painter `FUN_004c3b40`, file `0xc3b40`;
    // dossier OPEN 4/7): row base y = 320 + 22·row. Column x values
    // now live beside the row model; only the base stays here for
    // the empty-band check.
    const RESULT_ROW_BASE_Y: i32 = 320;

    /// Cell x values of one eight-cell digit field, rightmost last.
    fn digit_cell_xs(last_x: i32) -> [i32; RESULT_DIGIT_CELLS] {
        let mut xs = [0; RESULT_DIGIT_CELLS];
        let mut index = 0;
        while index < RESULT_DIGIT_CELLS {
            let back = i32::try_from(RESULT_DIGIT_CELLS - 1 - index).unwrap_or(0);
            xs[index] = last_x - back * RESULT_DIGIT_PITCH;
            index += 1;
        }
        xs
    }

    /// Flat native result for `score` plus `rows`, exactly as the
    /// no-art fallback paints it.
    fn flat_native_rows(score: &o2jam_game::Score, rows: &[ResultRankRow]) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            score,
            rows,
            None,
        );
        buf
    }

    /// Flat native result for `score` with no rank rows, exactly as
    /// the no-art fallback paints it.
    fn flat_native_result(score: &o2jam_game::Score) -> Vec<u32> {
        flat_native_rows(score, &[])
    }

    /// Staged native result scene for one verdict and grade.
    fn painted_native_scene(art: &SceneArt, verdict: ResultVerdict, grade: Option<u8>) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &ResultPaint {
                portrait_slots: None,
                score: &o2jam_game::Score::default(),
                rows: &[],
                art,
                verdict,
                grade,
                life_readout: None,
                bottom_counter: None,
            },
        );
        buf
    }

    /// Frame body with one opaque 2x2 layer per `anchors` entry, each
    /// a distinct solid RGB555 ink (black colorkey) — the pinned
    /// banner/plate bodies, shrunk to probe pixels.
    fn frame_body(anchors: &[(i32, i32)]) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        for (index, (dx, dy)) in anchors.iter().enumerate() {
            figure.push_layer(FigureLayer {
                image: Rgb555Image {
                    w: 2,
                    h: 2,
                    words: vec![0x0001u16 << (index + 1); 4],
                },
                dx: *dx,
                dy: *dy,
                colorkey: 0x0000,
            });
        }
        figure
    }

    /// Solid ink of frame-body layer `index` (see [`frame_body`]).
    fn frame_ink(index: usize) -> u32 {
        let (r, g, b) = o2jam_render::expand_rgb555(0x0001u16 << (index + 1));
        pack_xrgb(r, g, b)
    }

    /// Score with every pinned field set to a distinct full-top-row
    /// digit (the bitmap glyphs of 8/9/3/2/7/5 all carry a lit
    /// top-left pixel; the TTF glyphs anchor the same left edge).
    fn full_score() -> o2jam_game::Score {
        let mut score = o2jam_game::Score {
            points: 87_654_321,
            max_combo: 77,
            counts: [800, 900, 300, 200],
            ..o2jam_game::Score::default()
        };
        for _ in 0..51 {
            score.jam.note(o2jam_game::Judgement::Cool);
        }
        score
    }

    /// One pinned field: ink must light inside the anchor's cell box
    /// (the TTF rasterizes within the cell, antialiased) and the
    /// column left of the anchor must stay flat (the decimal starts
    /// exactly at the pinned x).
    fn assert_field(buf: &[u32], x: i32, y: i32) {
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        let ink = result_number_ink();
        let cell_lit = (x..x + 10).any(|px| (y..y + 18).any(|py| near_xrgb(at(px, py), ink)));
        assert!(cell_lit, "field at ({x},{y}) must light ink");
        for py in y..y + 16 {
            assert!(
                !near_xrgb(at(x - 1, py), ink),
                "left of ({x},{py}) must stay flat"
            );
        }
    }

    #[test]
    fn cool_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_COOL_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn good_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_GOOD_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn bad_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_BAD_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn miss_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_MISS_X,
            RESULT_COUNT_Y,
        );
    }

    #[test]
    fn max_combo_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_MAX_COMBO_X,
            RESULT_COMBO_Y,
        );
    }

    #[test]
    fn jam_combo_field_paints_the_jam_level_chain() {
        // 51 Cools climb the jam to level 2 (progress 102 over the
        // level-2 threshold; 50 Cools park at progress 100, which the
        // strict-greater grant does not fire on): the JAM COMBO field
        // shows the chain, the wire's u16 `jam` field.
        assert_eq!(full_score().jam.level(), 2);
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_JAM_COMBO_X,
            RESULT_COMBO_Y,
        );
    }

    #[test]
    fn total_score_field_paints_at_its_pinned_anchor() {
        assert_field(
            &flat_native_result(&full_score()),
            RESULT_SCORE_X,
            RESULT_SCORE_Y,
        );
    }

    #[test]
    fn result_numbers_clamp_at_the_32_bit_field() {
        // The `%8d` read is the exe's 32-bit field: an oversized u64
        // accumulator saturates instead of wrapping.
        let score = o2jam_game::Score {
            points: u64::from(u32::MAX) + 5,
            ..o2jam_game::Score::default()
        };
        let expected = flat_native_result(&o2jam_game::Score {
            points: u64::from(u32::MAX),
            ..o2jam_game::Score::default()
        });
        let actual = flat_native_result(&score);
        assert_eq!(actual, expected);
    }

    #[test]
    fn result_numbers_wait_for_the_native_canvas() {
        // 70x60 is not the exe canvas: no field paints, matching the
        // small-frame contract of tests/scenes.rs.
        let mut buf = vec![0u32; 70 * 60];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &full_score(),
            &[],
            None,
        );
        assert!(!buf.contains(&result_number_ink()));
    }

    #[test]
    fn result_digit_fields_stay_unpainted_without_staged_values() {
        // The fonts bind (painter VAs 0x4c44c0/0x4c4630) but the value
        // globals stay OPEN (register-held writers), so an unstaged
        // field paints nothing — the GDI numbers above never touch
        // the (292..460, 77) or (8..176, 536) cell rows.
        let buf = flat_native_result(&full_score());
        let ink = result_number_ink();
        for x in digit_cell_xs(RESULT_LIFE_LAST_X) {
            let pixel = buf[RESULT_LIFE_Y as usize * 800 + x as usize];
            assert_ne!(pixel, ink, "life cell x={x}");
        }
        for x in digit_cell_xs(RESULT_BOTTOM_LAST_X) {
            let pixel = buf[RESULT_BOTTOM_Y as usize * 800 + x as usize];
            assert_ne!(pixel, ink, "bottom cell x={x}");
        }
        assert_eq!(digit_cell_xs(RESULT_LIFE_LAST_X).len(), 8);
        assert_eq!(digit_cell_xs(RESULT_LIFE_LAST_X)[0], 292);
        assert_eq!(digit_cell_xs(RESULT_BOTTOM_LAST_X)[0], 8);
    }

    /// One native staged scene with both digit fonts (the shared
    /// ten-layer test font) and the given staged values, exactly as
    /// [`paint_result_scene`] paints it.
    fn painted_digit_scene(life_readout: Option<u32>, bottom_counter: Option<u32>) -> Vec<u32> {
        let mut figures = HashMap::new();
        figures.insert(RESULT_LIFE_FONT_SPRITE.to_owned(), digit_font_figure());
        figures.insert(RESULT_BOTTOM_FONT_SPRITE.to_owned(), digit_font_figure());
        let art = SceneArt {
            placements: Vec::new(),
            figures,
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &ResultPaint {
                portrait_slots: None,
                score: &o2jam_game::Score::default(),
                rows: &[],
                art: &art,
                verdict: ResultVerdict::Solo,
                grade: None,
                life_readout,
                bottom_counter,
            },
        );
        buf
    }

    #[test]
    fn life_readout_paints_pointnum_digits_through_the_pinned_cells() {
        // Staged value 7: one digit in the last life cell (x=460,
        // y=77) through the PointNum layer of the same index; the
        // seven leading cells stay clear (digit count, not 8).
        let buf = painted_digit_scene(Some(7), None);
        let ink = pack_xrgb(0xFF, 0, 0xFF); // layer 7 word 0x7C1F
        assert_eq!(
            buf[RESULT_LIFE_Y as usize * 800 + 460],
            ink,
            "digit 7 layer expected at (460,77)"
        );
        for x in digit_cell_xs(RESULT_LIFE_LAST_X)[..7].iter() {
            let pixel = buf[RESULT_LIFE_Y as usize * 800 + *x as usize];
            assert_ne!(pixel, ink, "leading life cell x={x} stays clear");
        }
    }

    #[test]
    fn bottom_counter_paints_numfont_blue_digits_through_the_pinned_cells() {
        // Staged value 52: digits 5 and 2 right-aligned at the two
        // last bottom cells (x=152/176, y=536) through the
        // Result_NumFont_Blue layers of the same indexes.
        let buf = painted_digit_scene(None, Some(52));
        assert_eq!(
            buf[RESULT_BOTTOM_Y as usize * 800 + 152],
            pack_xrgb(0xFF, 0xFF, 0), // layer 5 word 0x7FE0
            "digit 5 layer expected at (152,536)"
        );
        assert_eq!(
            buf[RESULT_BOTTOM_Y as usize * 800 + 176],
            pack_xrgb(0, 0, 0xFF), // layer 2 word 0x001F
            "digit 2 layer expected at (176,536)"
        );
    }

    #[test]
    fn result_digit_one_carries_the_pinned_five_pixel_kern() {
        // Digit 1 shifts its cell +5 (painter constant, both bodies):
        // value 1 lands at x=465, not 460.
        let buf = painted_digit_scene(Some(1), None);
        let ink = pack_xrgb(0, 0xFF, 0); // layer 1 word 0x03E0
        assert_eq!(
            buf[RESULT_LIFE_Y as usize * 800 + 465],
            ink,
            "digit 1 kerned to x=465"
        );
        assert_ne!(
            buf[RESULT_LIFE_Y as usize * 800 + 460],
            ink,
            "unkerned origin stays clear"
        );
    }

    #[test]
    fn result_digit_fields_ignore_unstaged_fonts() {
        // Values without their font body paint nothing (a staged
        // value never invents a font); the GDI numbers and the flat
        // card still paint around the cell rows.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::new(),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &ResultPaint {
                portrait_slots: None,
                score: &o2jam_game::Score::default(),
                rows: &[],
                art: &art,
                verdict: ResultVerdict::Solo,
                grade: None,
                life_readout: Some(7),
                bottom_counter: Some(52),
            },
        );
        assert_ne!(
            buf[RESULT_LIFE_Y as usize * 800 + 460],
            pack_xrgb(0xFF, 0, 0xFF)
        );
        assert_ne!(
            buf[RESULT_BOTTOM_Y as usize * 800 + 176],
            pack_xrgb(0, 0, 0xFF)
        );
    }

    /// One blank row (every column `None`): the row band keeps its
    /// backdrop, whatever the score carries.
    fn blank_row() -> ResultRankRow {
        ResultRankRow {
            rank: None,
            name: None,
            judgements: None,
            combos: None,
            score: None,
            bonus: None,
            speed: None,
            seat_byte: None,
        }
    }

    /// One fully live row: rank 8, HOST, 825/20/0/2, MAX 320 JAM 5,
    /// 110371 points, +50 bonus, `X2` speed.
    fn live_row() -> ResultRankRow {
        ResultRankRow {
            rank: Some(8),
            name: Some("HOST".to_owned()),
            judgements: Some([825, 20, 0, 2]),
            combos: Some((320, 5)),
            score: Some(110_371),
            bonus: Some(50),
            speed: Some(3),
            seat_byte: None,
        }
    }

    /// True while white result ink lands in the `width`-wide text
    /// box at a rank-cell origin (glyph top-left cells are often
    /// blank, so scan right and below the pinned left edge; narrow
    /// boxes keep neighbor columns out of absence checks).
    fn row_ink_near(buf: &[u32], origin: (i32, i32), width: i32) -> bool {
        let (ox, oy) = origin;
        for row in oy..oy + 16 {
            for col in ox..ox + width {
                let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
                    continue;
                };
                if r < 600 && c < 800 && near_xrgb(buf[r * 800 + c], result_number_ink()) {
                    return true;
                }
            }
        }
        false
    }

    /// True while white result ink lands near a rank-cell origin.
    fn row_white_near(buf: &[u32], origin: (i32, i32)) -> bool {
        row_ink_near(buf, origin, 72)
    }

    #[test]
    fn empty_rank_rows_leave_the_row_band_unpainted() {
        // No row data, no row ink: the band keeps the flat card —
        // none of the pinned row columns (rank 91, name 128, judge
        // 230, combo 416, score 530, bonus 626, speed 685) take GDI
        // ink, even with a full top-panel score behind them.
        for rows in [&[][..], &[blank_row()][..]] {
            let buf = flat_native_rows(&full_score(), rows);
            for origin in [
                (RESULT_RANK_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y),
                (RESULT_ROW_JUDGE_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_COMBO_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_SCORE_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_BONUS_X, RESULT_ROW_TEXT_Y),
                (RESULT_ROW_SPEED_X, RESULT_ROW_TEXT_Y),
            ] {
                assert!(!row_white_near(&buf, origin), "row cell {origin:?}");
            }
        }
        assert_eq!(RESULT_ROW_PITCH, 22);
        assert_eq!(RESULT_ROW_BASE_Y, 320);
    }

    #[test]
    fn rank_column_paints_the_live_rank_byte() {
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(&buf, (RESULT_RANK_X, RESULT_ROW_TEXT_Y)));
    }

    #[test]
    fn name_column_paints_the_member_nickname() {
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(&buf, (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y)));
    }

    #[test]
    fn judgements_column_paints_the_combined_counts() {
        // Combined `"%5d /%5d /%3d /%3d"` text, like the exe row
        // painter passes it.
        assert_eq!(
            result_judgements_text([825, 20, 0, 2]),
            "  825 /   20 /  0 /  2"
        );
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_JUDGE_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn combos_column_paints_max_then_jam() {
        // Combined `"%4d /%4d"`: MAX COMBO then JAM COMBO order.
        assert_eq!(result_combos_text((320, 5)), " 320 /   5");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_COMBO_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn score_column_paints_the_live_points() {
        assert_eq!(result_score_text(110_371), "  110371");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_SCORE_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn bonus_column_paints_the_hold_bonus() {
        // `"+ %d"` of the live hold accumulator (dossier OPEN 7
        // leaves the semantic open; the value itself is live).
        assert_eq!(result_bonus_text(50), "+ 50");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_BONUS_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn speed_column_paints_the_shared_speed_label() {
        // Speed code 3 reads `X2` through the shared label chain.
        assert_eq!(room_speed_label(3), "X2");
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[live_row()]);
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_SPEED_X, RESULT_ROW_TEXT_Y)
        ));
    }

    #[test]
    fn long_nicknames_truncate_past_one_hundred_chars() {
        assert_eq!(
            truncate_result_name(&"HOST".repeat(30)).chars().count(),
            100
        );
        assert_eq!(truncate_result_name("HOST"), "HOST");
        let mut row = blank_row();
        row.name = Some("HOST".repeat(30));
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[row]);
        assert!(row_white_near(&buf, (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y)));
    }

    #[test]
    fn peer_rows_paint_names_without_invented_numbers() {
        // Per-member 4014/4016 tallies stay server-side: a peer row
        // carries its name (and speed) while every number column
        // stays unpainted — never zero-filled.
        let peer = ResultRankRow {
            name: Some("HOST".to_owned()),
            speed: Some(3),
            ..blank_row()
        };
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[peer]);
        assert!(row_white_near(&buf, (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y)));
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_SPEED_X, RESULT_ROW_TEXT_Y)
        ));
        // Narrow boxes: the nickname opens at x=128 inside the
        // rank box, and the speed label opens at x=685 inside the
        // bonus box.
        assert!(!row_ink_near(&buf, (RESULT_RANK_X, RESULT_ROW_TEXT_Y), 37));
        assert!(!row_ink_near(
            &buf,
            (RESULT_ROW_BONUS_X, RESULT_ROW_TEXT_Y),
            58
        ));
        for origin in [
            (RESULT_ROW_JUDGE_X, RESULT_ROW_TEXT_Y),
            (RESULT_ROW_COMBO_X, RESULT_ROW_TEXT_Y),
            (RESULT_ROW_SCORE_X, RESULT_ROW_TEXT_Y),
        ] {
            assert!(!row_white_near(&buf, origin), "peer cell {origin:?}");
        }
    }

    #[test]
    fn rank_rows_stack_at_the_22px_pitch() {
        // Row 1 starts 22 px below row 0 on both text lines.
        let buf = flat_native_rows(&o2jam_game::Score::default(), &[blank_row(), live_row()]);
        assert!(!row_white_near(&buf, (RESULT_RANK_X, RESULT_ROW_TEXT_Y)));
        assert!(row_white_near(
            &buf,
            (RESULT_RANK_X, RESULT_ROW_TEXT_Y + 22)
        ));
        assert!(row_white_near(
            &buf,
            (RESULT_ROW_NAME_X, RESULT_ROW_NAME_Y + 22)
        ));
    }

    #[test]
    fn rank_rows_wait_for_the_native_canvas() {
        // Small frames paint no row ink, like the top-panel fields.
        let mut buf = vec![0u32; 70 * 60];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 70,
                h: 60,
            },
            &o2jam_game::Score::default(),
            &[live_row()],
            None,
        );
        assert!(!buf.contains(&result_number_ink()));
    }

    #[test]
    fn rank_rows_cap_at_eight_seats() {
        // Nine live rows in, eight paint: row 8 (y 493) stays
        // backdrop while row 7 (y 471) inks.
        let rows = vec![live_row(); 9];
        let buf = flat_native_rows(&o2jam_game::Score::default(), &rows);
        assert!(row_white_near(
            &buf,
            (RESULT_RANK_X, RESULT_ROW_TEXT_Y + 7 * 22)
        ));
        assert!(!row_white_near(
            &buf,
            (RESULT_RANK_X, RESULT_ROW_TEXT_Y + 8 * 22)
        ));
    }

    #[test]
    fn backdrop_keeps_top_and_bottom_panels() {
        // Dossier note 6: the fill never draws the Top/Bottom row
        // objects itself; the per-frame draw-all owns them. The
        // shell backdrop stages their bodies, skipping only the
        // frame bodies (the verdict placement must not paint).
        let art = SceneArt {
            placements: vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_TOP".to_owned(),
                    code: 0,
                    sprite: Some(RESULT_TOP_SPRITE.to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 4,
                        h: 4,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_BOTTOM".to_owned(),
                    code: 0,
                    sprite: Some(RESULT_BOTTOM_SPRITE.to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 250,
                        w: 4,
                        h: 4,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_VERDICT".to_owned(),
                    code: 0,
                    sprite: Some(RESULT_WINLOSE_SPRITE.to_owned()),
                    rect: Rect {
                        x: 400,
                        y: 100,
                        w: 4,
                        h: 4,
                    },
                },
            ],
            figures: HashMap::from([
                (RESULT_TOP_SPRITE.to_owned(), red_figure()),
                (RESULT_BOTTOM_SPRITE.to_owned(), red_figure()),
                (RESULT_WINLOSE_SPRITE.to_owned(), red_figure()),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_result_backdrop(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        let red = pack_xrgb(255, 0, 0);
        assert_eq!(buf[0], red, "top panel paints");
        assert_eq!(buf[250 * 800], red, "bottom panel paints");
        assert_eq!(buf[100 * 800 + 400], 0, "verdict body skipped");
    }

    #[test]
    fn winlose_frames_follow_the_pinned_rule() {
        // `FUN_004c3120`: solo 4; battle flag 1 -> 1, flag 2 -> 2,
        // else 3. The shell resolves the same standing from team
        // totals, so every dossier frame stays reachable.
        assert_eq!(winlose_frame(ResultVerdict::Solo), Some(4));
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Win))),
            Some(1)
        );
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Lose))),
            Some(2)
        );
        assert_eq!(
            winlose_frame(ResultVerdict::Battle(Some(BattleVerdict::Other))),
            Some(3)
        );
        assert_eq!(winlose_frame(ResultVerdict::Battle(None)), None);
    }

    #[test]
    fn result_fallback_rects_equal_the_state_result_bind_rects() {
        // The `State_Result.bnd` 7-rect walk (7.55 `Interface1.opi`;
        // rect # + corners in the `RESULT_RETRY_RECT` doc): the
        // button fallbacks are rects 5/6 verbatim.
        assert_eq!(
            RESULT_RETRY_RECT,
            Rect {
                x: 625,
                y: 570,
                w: 83,
                h: 22
            }
        );
        assert_eq!(
            RESULT_BACK_RECT,
            Rect {
                x: 711,
                y: 569,
                w: 83,
                h: 24
            }
        );
        // The grade plate rides rect 7 (`IDC_IMAGE_SRESULT_RANKINGFIELD`):
        // every frame anchors at its origin (395,531).
        let plate = Rect {
            x: 395,
            y: 531,
            w: 79,
            h: 41,
        };
        assert_eq!((plate.x, plate.y), (395, 531));
        assert_eq!(RESULT_RETRY_ID, "IDC_BUTTON_SRESULT_RETRY");
        assert_eq!(RESULT_BACK_ID, "IDC_BUTTON_SRESULT_BACK");
    }

    #[test]
    fn winner_image_never_joins_the_result_frame_bodies() {
        // `Winner_image.ojs` is play-scene network-variant art (entry
        // 230 rides unloaded; never re-read in the result region):
        // only the verdict banner and the grade plate are frame
        // bodies, so winner art cannot bleed through the backdrop.
        assert!(RESULT_FRAME_BODIES.contains(&RESULT_WINLOSE_SPRITE));
        assert!(RESULT_FRAME_BODIES.contains(&RESULT_RANK_SPRITE));
        assert!(!RESULT_FRAME_BODIES.contains(&WINNER_IMAGE_SPRITE));
        assert!(!result_loader_sprites().contains(&WINNER_IMAGE_SPRITE));
    }

    /// Banner art at the pinned image anchors: images 0/1 at (100,
    /// 190), image 2 at (151,190), image 3 at (126,190).
    fn banner_art() -> SceneArt {
        SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                RESULT_WINLOSE_SPRITE.to_owned(),
                frame_body(&[(100, 190), (100, 190), (151, 190), (126, 190)]),
            )]),
        }
    }

    #[test]
    fn solo_result_paints_the_fourth_banner_image() {
        // Frame 4 clamps to image 3, the solo banner at (126,190);
        // the battle images never paint.
        let buf = painted_native_scene(&banner_art(), ResultVerdict::Solo, None);
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(126, 190), frame_ink(3));
        assert_ne!(at(100, 190), frame_ink(0));
        assert_ne!(at(151, 190), frame_ink(2));
    }

    #[test]
    fn battle_win_paints_the_first_banner_image() {
        let buf = painted_native_scene(
            &banner_art(),
            ResultVerdict::Battle(Some(BattleVerdict::Win)),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(100, 190), frame_ink(0));
        assert_ne!(at(126, 190), frame_ink(3));
        assert_ne!(at(151, 190), frame_ink(2));
    }

    #[test]
    fn battle_lose_paints_the_second_banner_image() {
        let buf = painted_native_scene(
            &banner_art(),
            ResultVerdict::Battle(Some(BattleVerdict::Lose)),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(100, 190), frame_ink(1));
        assert_ne!(at(100, 190), frame_ink(0));
        assert_ne!(at(151, 190), frame_ink(2));
        assert_ne!(at(126, 190), frame_ink(3));
    }

    #[test]
    fn battle_other_paints_the_third_banner_image() {
        let buf = painted_native_scene(
            &banner_art(),
            ResultVerdict::Battle(Some(BattleVerdict::Other)),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(151, 190), frame_ink(2));
        assert_ne!(at(100, 190), frame_ink(0));
        assert_ne!(at(100, 190), frame_ink(1));
        assert_ne!(at(126, 190), frame_ink(3));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn unresolved_battle_paints_no_banner() {
        let buf = painted_native_scene(&banner_art(), ResultVerdict::Battle(None), None);
        for ink in [frame_ink(0), frame_ink(1), frame_ink(2), frame_ink(3)] {
            assert!(!buf.contains(&ink));
        }
    }

    /// Grade plate art: eight frames all anchored (395,531), each a
    /// distinct solid ink.
    fn rank_art() -> SceneArt {
        SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_IMAGE_SRESULT_RANKINGFIELD".to_owned(),
                code: 0x1750_0007,
                sprite: Some(RESULT_RANK_SPRITE.to_owned()),
                rect: Rect {
                    x: 395,
                    y: 531,
                    w: 79,
                    h: 41,
                },
            }],
            figures: HashMap::from([(RESULT_RANK_SPRITE.to_owned(), frame_body(&[(395, 531); 8]))]),
        }
    }

    #[test]
    fn grade_thresholds_climb_the_pinned_table() {
        // `FUN_004c4dd0` f64 compares at 0.30/0.50/0.70/0.80/0.90/
        // 0.95, each lifting the plate one frame.
        let cases = [
            (0.29, 0u8),
            (0.30, 1),
            (0.49, 1),
            (0.50, 2),
            (0.69, 2),
            (0.70, 3),
            (0.79, 3),
            (0.80, 4),
            (0.89, 4),
            (0.90, 5),
            (0.949, 5),
            (0.95, 6),
            (1.0, 6),
        ];
        for (ratio, frame) in cases {
            assert_eq!(grade_frame(ratio, false, false), frame, "ratio {ratio}");
        }
    }

    #[test]
    fn grade_promotes_combo_top_and_max_score() {
        // Max combo == note count - 1 promotes grade 6 only; a score
        // equal to the max reference gives 8 outright.
        assert_eq!(grade_frame(0.95, true, false), 7);
        assert_eq!(grade_frame(0.90, true, false), 5);
        assert_eq!(grade_frame(0.30, false, true), 8);
    }

    /// Score with a fixed live total behind the plate selector.
    fn scored_plate(points: u64, max_combo: u32) -> o2jam_game::Score {
        o2jam_game::Score {
            points,
            max_combo,
            ..o2jam_game::Score::default()
        }
    }

    #[test]
    fn live_plate_climbs_one_frame_per_band() {
        // Word 51 pins the reference at 10_450 (full-word base), so
        // each point total below lands its band's frame through the
        // live ratio.
        let cases = [
            (3_134, 0u8),
            (3_135, 1),
            (5_225, 2),
            (7_315, 3),
            (8_360, 4),
            (9_405, 5),
            (9_928, 6),
        ];
        for (points, frame) in cases {
            let grade = result_grade_frame(&scored_plate(points, 0), 51);
            assert_eq!(grade, (frame >= 1).then_some(frame), "{points} points");
        }
    }

    #[test]
    fn live_plate_promotes_combo_top_and_max_score() {
        // 9_928 points read past 0.95 (frame 6): combo 50 (word 51 −
        // 1) promotes to 7, combo 49 stays at 6, and an exact-max
        // total gives 8 outright.
        assert_eq!(result_grade_frame(&scored_plate(9_928, 50), 51), Some(7));
        assert_eq!(result_grade_frame(&scored_plate(9_928, 49), 51), Some(6));
        assert_eq!(result_grade_frame(&scored_plate(9_405, 50), 51), Some(5));
        assert_eq!(result_grade_frame(&scored_plate(10_450, 0), 51), Some(8));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn live_plate_stays_off_without_chart_or_ratio() {
        // No word, or a word with no reference (26 unscored records),
        // or a ratio under 0.30: the enter painter's frame 0 stays.
        assert_eq!(result_grade_frame(&scored_plate(5_250, 0), 0), None);
        assert_eq!(result_grade_frame(&scored_plate(0, 0), 26), None);
        assert_eq!(result_grade_frame(&scored_plate(1_574, 0), 51), None);
    }

    #[test]
    fn live_plate_paints_its_frame_at_the_pinned_anchor() {
        // End-to-end wiring: the live ratio selects the frame and the
        // staged plate paints it at (395,531) — band 1 paints image
        // 0, the promoted band paints image 6.
        let at = |buf: &[u32], x: i32, y: i32| buf[y as usize * 800 + x as usize];
        let low = result_grade_frame(&scored_plate(3_135, 0), 51);
        let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, low);
        assert_eq!(at(&buf, 395, 531), frame_ink(0));
        let top = result_grade_frame(&scored_plate(9_928, 50), 51);
        let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, top);
        assert_eq!(at(&buf, 395, 531), frame_ink(6));
    }

    #[test]
    fn grade_plate_paints_the_selected_frame_at_its_pinned_anchor() {
        // Grade 3 paints image 2 at (395,531); the wider max-score
        // image stays out.
        let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, Some(3));
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(395, 531), frame_ink(2));
        assert_ne!(at(395, 531), frame_ink(7));
    }

    #[test]
    fn sub_threshold_grades_keep_the_plate_off() {
        // Grade 0 (ratio under 0.30) and no ratio at all paint no
        // letter: the enter painter's frame 0 stays the empty state.
        for grade in [None, Some(0)] {
            let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, grade);
            for index in 0..8 {
                assert!(!buf.contains(&frame_ink(index)));
            }
        }
    }

    #[test]
    fn portrait_slots_use_the_pinned_mode_tables() {
        // The builder FUN_004c2390 writes one of two pinned 8-entry
        // tables by the display-mode word +0x24 (files
        // 0xc245d–0xc24b8 / 0xc24bf–0xc251a): mode 0xf selects
        // {0x7c02,…}, mode 0x10 selects {0xf802,…}; any other mode
        // writes neither and the shell stages no table.
        let mode_f = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_F).expect("mode 0xf table");
        assert_eq!(
            mode_f.map(|slot| slot.word),
            [0x7c02, 0x7ee2, 0x7be0, 0x4f2a, 0x37bc, 0x001c, 0x4d5a, 0x5923]
        );
        let mode_10 = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_10).expect("mode 0x10 table");
        assert_eq!(
            mode_10.map(|slot| slot.word),
            [0xf802, 0xfde2, 0xf7e0, 0x9e6a, 0x6f5c, 0x001c, 0x9a9a, 0xb263]
        );
        assert_eq!(result_portrait_slots(0), None);
        assert_eq!(result_portrait_slots(0x11), None);
    }

    #[test]
    fn portrait_slots_carry_the_builder_fixed_fields() {
        // Every slot gets obj+0x15e = trunc(22/100·256) = 56 and
        // obj+0x160 = 0.22f through 0x421610(obj, entry, 22),
        // whatever the table entry (files 0x2161c–0x21688).
        for slots in [
            result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_F),
            result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_10),
        ]
        .into_iter()
        .flatten()
        {
            for _slot in slots {
                assert_eq!(ResultPortraitSlot::NUMERATOR, 56);
                assert!((ResultPortraitSlot::RATIO - 0.22).abs() < f64::EPSILON);
            }
        }
    }

    #[test]
    fn seat_byte_picks_the_slot_one_based() {
        // FUN_004c3b40 zero-extends the record +0x31 byte and indexes
        // the eight this+0x5c objects 1-based (files
        // 0xc3be2–0xc3bfe): byte 1 picks element 0, byte 8 picks
        // element 7; every member sharing a byte gets the same slot.
        let slots = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_F).unwrap();
        assert_eq!(
            portrait_slot_for_seat_byte(1, &slots),
            Some(ResultPortraitSlot { word: 0x7c02 })
        );
        assert_eq!(
            portrait_slot_for_seat_byte(8, &slots),
            Some(ResultPortraitSlot { word: 0x5923 })
        );
        let mode_10 = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_10).unwrap();
        assert_eq!(
            portrait_slot_for_seat_byte(1, &mode_10),
            Some(ResultPortraitSlot { word: 0xf802 })
        );
    }

    #[test]
    fn seat_byte_zero_skips_the_row_as_the_battle_bucket() {
        // The same byte is FUN_004c49a0's battle-team bucket: byte 0
        // skips the member (file 0xc4a87), so no chip resolves.
        let slots = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_F).unwrap();
        assert_eq!(portrait_slot_for_seat_byte(0, &slots), None);
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn seat_byte_above_eight_resolves_nothing() {
        // The exe index is unguarded — byte 0 reads below the array
        // and bytes 9+ read above it. The shell resolves neither
        // instead of inventing the out-of-range object
        // (docs/cresult-anchors.md, "Peer-row portraits").
        let slots = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_F).unwrap();
        assert_eq!(portrait_slot_for_seat_byte(9, &slots), None);
        assert_eq!(portrait_slot_for_seat_byte(u8::MAX, &slots), None);
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn portrait_chip_resolution_paints_nothing_visible_yet() {
        // The eight slots are image-less sprite objects and what the
        // draw path renders from +0x15c stays OPEN, so a fully
        // seat-byte-populated row set through a built table leaves
        // the frame pixel-identical to no rows at all — the chip
        // seam resolves, no chip pixels are invented.
        let rows = |byte| ResultRankRow {
            seat_byte: Some(byte),
            ..live_row()
        };
        let slots = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_F);
        let paint_rows = |rows: &[ResultRankRow]| {
            let mut buf = vec![0u32; 800 * 600];
            paint_result(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &o2jam_game::Score::default(),
                rows,
                slots.as_ref(),
            );
            buf
        };
        // Identical rows minus the seat bytes: the chip resolution
        // adds no pixels on top of the row text the seats paint.
        let unseated: Vec<ResultRankRow> = [1u8, 5, 8, 0]
            .into_iter()
            .map(|byte| ResultRankRow {
                seat_byte: None,
                ..rows(byte)
            })
            .collect();
        assert_eq!(
            paint_rows(&[rows(1), rows(5), rows(8), rows(0)]),
            paint_rows(&unseated)
        );
    }

    #[test]
    fn frame_bodies_never_stack_through_the_backdrop() {
        // The rank placement binds rect 7 like the real block: the
        // whole-layer backdrop pass must skip the body (its last
        // layer would win the stack), leaving only the selected
        // frame.
        let buf = painted_native_scene(&rank_art(), ResultVerdict::Solo, Some(1));
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(395, 531), frame_ink(0), "grade 1 paints image 0");
        assert_ne!(at(395, 531), frame_ink(7), "the stack's last layer");
    }

    #[test]
    fn result_scene_paints_staged_art_without_the_invented_verdict() {
        let art = corner_art();
        let mut buf = vec![0u32; 64 * 64];
        paint_result_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 64,
            },
            &ResultPaint {
                portrait_slots: None,
                score: &o2jam_game::Score::default(),
                rows: &[],
                art: &art,
                verdict: ResultVerdict::Battle(Some(BattleVerdict::Win)),
                grade: None,
                life_readout: None,
                bottom_counter: None,
            },
        );
        // The staged art paints (the corner banner body rides frame 1
        // onto its clamped layer); the retired amber caption ink
        // never comes back.
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(!has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    /// Ten-layer number font: layer `i` paints digit `i`.
    fn digit_font_figure() -> AvatarFigure {
        let words = [
            0x7C00u16, 0x03E0, 0x001F, 0x7FFF, 0x0000, 0x7FE0, 0x03FF, 0x7C1F, 0x4210, 0x2108,
        ];
        let mut figure = AvatarFigure::default();
        for word in words {
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.to_le_bytes(), 1, 1).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    /// `ROOMNO` digit places as `SET 0x11` clones under panel 1, at
    /// the cloned rects (ones, tens, hundreds left to right — the
    /// authoring ordinal order).
    fn room_number_clones() -> Vec<ScenePlacement> {
        [(0x1350_002E, 30), (0x1350_002F, 20), (0x1350_0030, 10)]
            .into_iter()
            .map(|(code, x)| ScenePlacement {
                role: PlacementRole::Clone,
                id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
                code,
                sprite: Some(ROOM_NUMBER_SPRITE.to_owned()),
                rect: Rect {
                    x,
                    y: 10,
                    w: 4,
                    h: 4,
                },
            })
            .collect()
    }

    #[test]
    fn room_number_clones_paint_their_digit_frames_in_place() {
        // Value 12 over ones-first clones: the control ordinal picks
        // the digit — ones blue (2), tens green (1), hundreds red (0)
        // — and every digit lands at its own clone rect.
        let art = SceneArt {
            placements: room_number_clones(),
            figures: HashMap::from([(ROOM_NUMBER_SPRITE.to_owned(), digit_font_figure())]),
        };
        let mut rooms = two_rooms();
        rooms[0].number = 12;
        let mut buf = vec![0u32; 800 * 600];
        paint_room_row_number(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            &rooms[0],
            "IDC_BUTTON_SROOM_EXBUTTON1",
        );
        assert_eq!(buf[10 * 800 + 10], pack_xrgb(255, 0, 0), "hundreds");
        assert_eq!(buf[10 * 800 + 20], pack_xrgb(0, 255, 0), "tens");
        assert_eq!(buf[10 * 800 + 30], pack_xrgb(0, 0, 255), "ones");
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn room_number_clones_add_their_record_anchor() {
        // Verified draw math carries into clones: the clone rect
        // positions and the digit layer's anchor adds (clone anchors
        // stay template-local).
        let mut figure = AvatarFigure::default();
        for _ in 0..10 {
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&0x7C00u16.to_le_bytes(), 1, 1).unwrap(),
                dx: 2,
                dy: 1,
                colorkey: 0xFFFF,
            });
        }
        let art = SceneArt {
            placements: room_number_clones(),
            figures: HashMap::from([(ROOM_NUMBER_SPRITE.to_owned(), figure)]),
        };
        let mut rooms = two_rooms();
        rooms[0].number = 5;
        let mut buf = vec![0u32; 800 * 600];
        paint_room_row_number(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            &rooms[0],
            "IDC_BUTTON_SROOM_EXBUTTON1",
        );
        // Only the ones clone has layer 0 in a one-layer body: the
        // glyph lands at (30 + 2, 10 + 1).
        assert_eq!(buf[11 * 800 + 32], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 800 + 30], 0);
    }

    #[test]
    fn room_scene_paints_number_digits_at_the_cloned_places() {
        // Panel 0 plus the staged `Room_Number.ojs` clones: value 12
        // paints hundreds/tens/ones into the three cloned place rects
        // (the exe's parent-offset write-back), never at the authored
        // template rects.
        use o2jam_render::Rect;
        let panel = ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
            code: 0x1301_201B,
            sprite: None,
            rect: Rect {
                x: 26,
                y: 82,
                w: 288,
                h: 91,
            },
        };
        let mut placements = vec![panel];
        placements.extend(room_number_clones());
        let art = SceneArt {
            placements,
            figures: HashMap::from([(ROOM_NUMBER_SPRITE.to_owned(), digit_font_figure())]),
        };
        let mut rooms = two_rooms();
        rooms[0].number = 12;
        let mut buf = vec![0u32; 800 * 600];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rooms,
            0,
            &art,
            None,
        );
        assert_eq!(buf[10 * 800 + 10], pack_xrgb(255, 0, 0));
        assert_eq!(buf[10 * 800 + 20], pack_xrgb(0, 255, 0));
        assert_eq!(buf[10 * 800 + 30], pack_xrgb(0, 0, 255));
    }

    /// Multi-layer icon body: layer `i` paints one solid RGB555
    /// `words[i]` image (colorkey white, so every pixel paints).
    fn icon_figure(words: &[u16]) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        for word in words {
            let pixels = vec![word.to_le_bytes(); 40 * 20].concat();
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&pixels, 40, 20).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    /// The three lobby row-icon placements at their `State_Room.bnd`
    /// template rects, with a distinct-layer figure per icon (green,
    /// red, blue, white in frame order; last layers feed the backdrop
    /// pass, selected frames the icon pass). The panel seats the
    /// template: icons land at template + panel origin (26,82).
    fn room_icon_art() -> SceneArt {
        // Icons are `SET 0x11` clones of panel 1 at their cloned rects
        // (parent origin already added by staging, see
        // `avatar::expand_set_clones`).
        let cloned = |code: u32, sprite: &str, x: i32, y: i32| ScenePlacement {
            role: PlacementRole::Clone,
            id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
            code,
            sprite: Some(sprite.to_owned()),
            rect: Rect { x, y, w: 40, h: 20 },
        };
        SceneArt {
            placements: vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "IDC_BUTTON_SROOM_EXBUTTON1".to_owned(),
                    code: 0x1301_201B,
                    sprite: None,
                    rect: Rect {
                        x: 26,
                        y: 82,
                        w: 288,
                        h: 91,
                    },
                },
                cloned(0x1350_0028, ROOM_MODE_SPRITE, 426, 362),
                cloned(0x1350_0029, ROOM_OM_SPRITE, 476, 362),
                cloned(0x1350_002D, ROOM_STATE_SPRITE, 526, 382),
            ],
            figures: HashMap::from([
                (
                    ROOM_MODE_SPRITE.to_owned(),
                    icon_figure(&[0x03E0, 0x7C00, 0x001F]),
                ),
                (
                    ROOM_OM_SPRITE.to_owned(),
                    icon_figure(&[0x03E0, 0x7C00, 0x001F, 0x7FE0]),
                ),
                (
                    ROOM_STATE_SPRITE.to_owned(),
                    icon_figure(&[0x03E0, 0x7C00, 0x001F]),
                ),
            ]),
        }
    }

    /// One room row overriding the listed fields.
    fn room_with(state_code: u8, key_mode: u8, music_id: u16) -> RoomListEntry {
        RoomListEntry {
            state_code,
            key_mode,
            music_id,
            ..two_rooms()[0].clone()
        }
    }

    #[test]
    fn icon_frame_layers_clamp_like_the_exe_setter() {
        // Setter `0x2f870`: frame n paints image n−1, saturating at 0
        // for frame 0 and clamping at the last decoded layer.
        assert_eq!(icon_frame_layer(0, 2), Some(0));
        assert_eq!(icon_frame_layer(1, 2), Some(0));
        assert_eq!(icon_frame_layer(2, 2), Some(1));
        assert_eq!(icon_frame_layer(9, 2), Some(1));
        assert_eq!(icon_frame_layer(1, 4), Some(0));
        assert_eq!(icon_frame_layer(4, 4), Some(3));
        assert_eq!(icon_frame_layer(0, 0), None);
    }

    #[test]
    fn room_state_icon_frame_tracks_the_state_byte() {
        assert_eq!(room_state_icon_frame(0), 0);
        assert_eq!(room_state_icon_frame(1), 1);
        assert_eq!(room_state_icon_frame(2), 2);
    }

    #[test]
    fn room_key_icon_frames_follow_the_3_and_7_key_rule() {
        // Icon sites `0x1267ad`–`0x1267e2`: key 0 → frame 1, key 2 →
        // frame 2, anything else untouched.
        assert_eq!(room_key_icon_frame(0), Some(1));
        assert_eq!(room_key_icon_frame(2), Some(2));
        assert_eq!(room_key_icon_frame(1), None);
        assert_eq!(room_key_icon_frame(9), None);
    }

    #[test]
    fn room_om_icon_bands_follow_the_level_mapper() {
        // Mapper `0x126cb0` via `om_icon_frame`: 0–4 → 1, 5–8 → 2,
        // 9–12 → 3, 13+ → 4.
        for (level, frame) in [
            (0u16, 1),
            (4, 1),
            (5, 2),
            (8, 2),
            (9, 3),
            (12, 3),
            (13, 4),
            (99, 4),
        ] {
            assert_eq!(room_om_icon_frame(200, Some(level)), Some(frame), "{level}");
        }
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn room_om_icon_shows_the_no_song_frame_below_the_music_floor() {
        // Floor compare `0x1251da` + constant level 1 (`0x1266b9`):
        // below the floor the row shows frame 1; past it an unknown
        // level (no staged song table) paints nothing.
        assert_eq!(room_om_icon_frame(0, None), Some(1));
        assert_eq!(room_om_icon_frame(ROOM_MUSIC_FLOOR - 1, None), Some(1));
        assert_eq!(room_om_icon_frame(ROOM_MUSIC_FLOOR, None), None);
        assert_eq!(room_om_icon_frame(u16::MAX, None), None);
    }

    #[test]
    fn room_icons_paint_their_frames_at_the_rect_origins() {
        // State 2 (frame 2 → layer 1 red), key 2 (frame 2 → layer 1
        // red), no song (frame 1 → layer 0 green): each at its
        // template rect origin plus the panel origin, none at the
        // frame corner.
        let art = room_icon_art();
        let rooms = vec![room_with(2, 2, 0)];
        let mut buf = vec![0u32; 800 * 600];
        paint_room_row_icons(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            &rooms,
        );
        let red = pack_xrgb(255, 0, 0);
        let green = pack_xrgb(0, 255, 0);
        assert_eq!(buf[382 * 800 + 526], red, "state icon frame");
        assert_eq!(buf[362 * 800 + 426], red, "key-mode icon frame");
        assert_eq!(buf[362 * 800 + 476], green, "om icon frame");
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn room_icons_skip_untouched_frames_and_missing_art() {
        // Key 1 leaves the mode icon untouched (no icon ink at the
        // panel-offset rect), a song without a staged level leaves
        // the om icon untouched, and an unstaged body paints nothing
        // — while the backdrop still shows its own layers at the
        // template rects.
        let mut art = room_icon_art();
        art.figures.remove(ROOM_STATE_SPRITE);
        let rooms = vec![room_with(1, 1, ROOM_MUSIC_FLOOR)];
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        art.paint_overlay(&mut frame);
        paint_room_row_icons(&mut frame, &art, &rooms);
        // The clone base pass paints each icon's frame-0 layer.
        assert_eq!(
            buf[362 * 800 + 426],
            pack_xrgb(0, 255, 0),
            "backdrop mode layer"
        );
        assert_eq!(
            buf[362 * 800 + 476],
            pack_xrgb(0, 255, 0),
            "backdrop om layer"
        );
        // An untouched icon keeps its clone's frame-0 layer in the base
        // pass (no live frame selects another one); a body that never
        // decoded paints nothing at all.
        assert_eq!(
            buf[362 * 800 + 426],
            pack_xrgb(0, 255, 0),
            "mode icon keeps frame 0 without a live frame"
        );
        assert_eq!(
            buf[362 * 800 + 476],
            pack_xrgb(0, 255, 0),
            "om icon keeps frame 0 without a level"
        );
        assert_eq!(buf[382 * 800 + 526], 0, "missing state body paints nothing");
    }

    #[test]
    fn room_scene_layers_icons_under_the_detail_ink() {
        // Full pipeline: the backdrop paints every layer, the icon
        // pass overlays the selected frames at template + panel
        // origin, and the detail text draws into its own template
        // offsets — the icons survive clear of the detail ink.
        let art = room_icon_art();
        let rooms = vec![room_with(2, 2, 0)];
        let mut buf = vec![0u32; 800 * 600];
        paint_room_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rooms,
            0,
            &art,
            None,
        );
        let red = pack_xrgb(255, 0, 0);
        assert_eq!(buf[382 * 800 + 526], red, "state icon at its panel offset");
        assert_eq!(
            buf[362 * 800 + 426],
            red,
            "key-mode icon at its panel offset"
        );
    }

    #[test]
    fn entry_less_mounted_art_paints_at_its_authored_anchor_not_the_canvas() {
        // Entry-less mounted art (`Dialog_ChannelSelectBar.ojs` anchors at
        // `(0, 0)`, 347x41 on 7.55) paints 1:1 at that authored anchor —
        // never stretched over the canvas. It is dialog art, not part of
        // the `STATE_CHANNEL` scene (which names zero sprites and stages
        // nothing).
        let art = SceneArt {
            placements: unbound_scene_placements(&[DIALOG_BAR_SPRITE.to_owned()]),
            figures: HashMap::from([(DIALOG_BAR_SPRITE.to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 128 * 96];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 128,
                h: 96,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0), "the authored anchor");
        assert_eq!(buf[70 * 128 + 100], 0, "no canvas-wide stretch");
    }

    #[test]
    fn loading_gate_paints_its_full_native_rect() {
        // `MusicLoading.ojs` is one 800x600 body at anchor (0,0):
        // staged unbound it covers exactly its native extent at 1:1 —
        // the art's own geometry is the rect, not a claimed scaler.
        let pixels = [[0x00, 0x7C]; 4 * 3].concat();
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&pixels, 4, 3).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let art = SceneArt {
            placements: unbound_scene_placements(&[LOADING_SPRITE.to_owned()]),
            figures: HashMap::from([(LOADING_SPRITE.to_owned(), figure)]),
        };
        let mut buf = vec![0u32; 32 * 24];
        paint_loading_scene(
            &mut Frame {
                buf: &mut buf,
                w: 32,
                h: 24,
            },
            &art,
        );
        let red = pack_xrgb(255, 0, 0);
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(buf[y * 32 + x], red, "native rect pixel ({x},{y})");
            }
        }
        assert_eq!(buf[4], 0, "nothing past the body's own extent");
    }

    #[test]
    fn loading_cover_paints_full_bleed_behind_caption() {
        // Staged OJN cover (2x2 opaque stand-in, distinct corners —
        // geometry only, never real art) stretches over the whole
        // frame while the caption ink paints on top. The OPEN bar
        // stays hidden.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(LOADING_COVER_SPRITE.to_owned(), cover_figure())]),
        };
        let status = LoadingStatus {
            progress: 1.0,
            title: "Title".to_owned(),
            artist: "Artist".to_owned(),
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_loading_scene_with_status(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
            &status,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0), "cover top-left");
        assert_eq!(buf[63], pack_xrgb(0, 255, 0), "cover top-right");
        assert_eq!(buf[47 * 64], pack_xrgb(0, 0, 255), "cover bottom-left");
        assert_eq!(
            buf[47 * 64 + 63],
            scrim_color(),
            "69-row Loading scrim covers the bottom-right on 64x48"
        );
        assert!(
            !buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)),
            "OPEN bar fill stays off the cover"
        );
        assert!(
            buf.contains(&art_title_ink()),
            "caption survives over the cover"
        );
    }

    #[test]
    fn loading_caption_routes_through_the_display_cell() {
        // `docs/loading-type.md` routing: the `TITLE - ARTIST`
        // caption lays out through `Cell::Loading`, whose advance
        // exceeds the shared `Cell::Box` face for the same text.
        let caption = "Title - Artist";
        let box_w = text_width_cell(caption, Cell::Box);
        let loading_w = text_width_cell(caption, Cell::Loading);
        assert!(loading_w > box_w, "{loading_w} must exceed {box_w}");
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn planet_scene_without_art_paints_nothing() {
        // Pinned no-fill rule: an empty stage contributes zero
        // staged pixels and no selection means no cursor, so the
        // frame stays untouched (the flat card path is the caller's
        // fallback, never this painter).
        let art = SceneArt::default();
        let mut buf = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            None,
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn planet_caption_ink_lands_on_the_exe_cell_anchor() {
        // Oracle: `~/scratch/paired-shots-v3/states/planet/exe.png`.
        // Each 7.55 caption inks from y 127 in its 94x13 TEXT cell
        // (cell top y 125) and centers on that cell. Our rasterizer's
        // cap-top anchor starts at the cell's left/top, so the fix
        // must move both axes.
        use super::super::side::PLANET_CAPTIONS;
        if o2jam_text::system_font().is_none() {
            return;
        }
        // Parsed `State_Planet.bnd` rects 7..11, code order.
        let cells = [
            (338, 125, 94_u32, 13_u32),
            (209, 125, 94, 13),
            (91, 125, 94, 13),
            (619, 125, 94, 13),
            (512, 125, 94, 13),
        ];
        for (caption, (x, y, w, h)) in PLANET_CAPTIONS.iter().zip(cells) {
            let art = SceneArt {
                placements: vec![ScenePlacement {
                    role: PlacementRole::Single,
                    id: String::new(),
                    code: caption.code,
                    sprite: None,
                    rect: Rect { x, y, w, h },
                }],
                figures: HashMap::new(),
            };
            let mut buf = vec![0_u32; 800 * 600];
            paint_planet_scene(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &art,
                None,
                None,
                None,
                PlanetPromoPaint::spinning(1),
            );
            let (mut left, mut top, mut right, mut bottom) = (i32::MAX, i32::MAX, 0, 0);
            for (at, pixel) in buf.iter().enumerate() {
                if *pixel == 0 {
                    continue;
                }
                let (ink_x, ink_y) = ((at % 800) as i32, (at / 800) as i32);
                left = left.min(ink_x);
                top = top.min(ink_y);
                right = right.max(ink_x);
                bottom = bottom.max(ink_y);
            }
            assert!(left <= right, "caption {} inks nothing", caption.code);
            assert_eq!(top, y + 2, "caption {} ink bearing", caption.code);
            let cell_bottom = y + i32::try_from(h).unwrap_or(0);
            assert!(
                bottom < cell_bottom,
                "caption {} ink leaves its cell",
                caption.code
            );
            let cell_center = x + i32::try_from(w).unwrap_or(0) / 2;
            let ink_center = (left + right) / 2;
            assert!(
                (ink_center - cell_center).abs() <= 1,
                "caption {} centered at {ink_center}, exe cell center {cell_center}",
                caption.code
            );
        }
    }

    #[test]
    fn planet_selection_paints_the_amber_cursor_stand_in() {
        // Highlight rendering is evidence-OPEN, so a picked band
        // draws only the shared amber cursor inside its pinned
        // button rect: HARD's rect center-left, nothing elsewhere.
        let art = SceneArt::default();
        let mut buf = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            Some(PlanetBand::Hard),
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        let amber = pack_xrgb(0xFF, 0xD0, 0x20);
        // paint_cursor draws at (x - 6, y): (313 + 10 - 6, 32 + 56).
        assert_eq!(buf[88 * 800 + 317], amber);
        assert_eq!(
            buf.iter().filter(|pixel| **pixel == amber).count(),
            3 * 5,
            "only the 3x5 cursor inks"
        );
    }

    /// One two-frame anchored figure: layer 0 in `first`, layer 1
    /// in `second`, both at (`dx`,`dy`) under a white colorkey — a
    /// stand-in for the two-record planet button bodies (rest frame
    /// plus hover/selected frame in stored order).
    fn anchored_two_frame_figure(
        w: usize,
        h: usize,
        dx: i32,
        dy: i32,
        first: [u8; 2],
        second: [u8; 2],
    ) -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        for word in [first, second] {
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.repeat(w * h), w, h).unwrap(),
                dx,
                dy,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    #[test]
    fn hovered_emblem_reveals_the_pinned_plate_band() {
        // Live caller chain: main.rs::run_play -> setup_online ->
        // live_art::stage_interface_scenes -> ScreenGame::paint_planet ->
        // paint_planet_scene -> paint_planet_board -> rest omits the
        // emblem button; hover -> paint_planet_hover -> plate band.
        // Matched #361 A/B: rest is the sheet's baked dim art (within
        // ~1 level/channel), not the bright button body; hover mounts
        // button frame 0 only inside EASY's pinned band
        // (94,44)-(178,62). No frame swap outside the band.
        use super::super::side::{PlanetAction, PLANET_BUTTON_EASY_CODE};
        let sprite = "Planet_Btn_Easy.ojs";
        let mut sheet = AvatarFigure::default();
        sheet.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0xE0, 0x03].repeat(99 * 112), 99, 112)
                .expect("sheet plate"),
            dx: 87,
            dy: 32,
            colorkey: 0xFFFF,
        });
        let art = SceneArt {
            placements: vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: "sheet".to_owned(),
                    code: 0,
                    sprite: Some("State_Planet_Fixture.ojs".to_owned()),
                    rect: Rect {
                        x: 87,
                        y: 32,
                        w: 99,
                        h: 112,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: String::new(),
                    code: PLANET_BUTTON_EASY_CODE,
                    sprite: Some(sprite.to_owned()),
                    rect: Rect {
                        x: 87,
                        y: 32,
                        w: 99,
                        h: 112,
                    },
                },
            ],
            figures: HashMap::from([
                ("State_Planet_Fixture.ojs".to_owned(), sheet),
                (
                    sprite.to_owned(),
                    anchored_two_frame_figure(99, 112, 87, 32, [0x1F, 0x00], [0x00, 0x7C]),
                ),
            ]),
        };
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let red = pack_xrgb(0xFF, 0x00, 0x00);
        let mut rest = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut rest,
                w: 800,
                h: 600,
            },
            &art,
            None,
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert_eq!(
            rest[50 * 800 + 136],
            green,
            "rest uses the baked sheet, not either button frame"
        );
        assert_eq!(rest[100 * 800 + 136], green, "rest body is the sheet");
        assert_eq!(
            rest[50 * 800 + 90],
            green,
            "rest beside the band is the sheet"
        );
        let mut hovered = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut hovered,
                w: 800,
                h: 600,
            },
            &art,
            None,
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Easy),
            }),
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert_eq!(hovered[50 * 800 + 136], blue, "plate band reveals layer 0");
        assert_eq!(
            hovered[100 * 800 + 136],
            green,
            "below the band the sheet stands"
        );
        assert_eq!(
            hovered[50 * 800 + 90],
            green,
            "left of the band the sheet stands"
        );
        assert_ne!(red, green, "the two button frames stay distinct fixtures");
    }

    #[test]
    fn quest_and_tutorial_emblems_ignore_hover() {
        // FREE/DRUM are static (0 changed pixels, `docs/hover.md`):
        // hovering them paints exactly the rest frame — the dimmed
        // plates never highlight.
        use super::super::side::{PlanetAction, PLANET_BUTTON_FREE_CODE};
        let sprite = "Planet_Btn_Free.ojs";
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_BUTTON_FREE_CODE,
                sprite: Some(sprite.to_owned()),
                rect: Rect {
                    x: 614,
                    y: 32,
                    w: 99,
                    h: 112,
                },
            }],
            figures: HashMap::from([(
                sprite.to_owned(),
                anchored_two_frame_figure(99, 112, 614, 32, [0x1F, 0x00], [0x00, 0x7C]),
            )]),
        };
        let mut rest = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut rest,
                w: 800,
                h: 600,
            },
            &art,
            None,
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        let mut hovered = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut hovered,
                w: 800,
                h: 600,
            },
            &art,
            None,
            Some(PlanetAction::Emblem { band: None }),
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert_eq!(hovered, rest, "static emblems paint no highlight");
    }

    /// Staged FREE + DRUM placements with two-record bodies (the
    /// exe-pinned codes and `.bnd` rects: DRUM/QUEST
    /// `(510,32)-(609,144)`, FREE/TUTORIAL `(614,32)-(713,144)`),
    /// so the #137 condition sweep paints both dimmed plates.
    fn quest_and_tutorial_art() -> SceneArt {
        use super::super::side::{PLANET_BUTTON_DRUM_CODE, PLANET_BUTTON_FREE_CODE};
        const FREE_SPRITE: &str = "Planet_Btn_Free.ojs";
        const DRUM_SPRITE: &str = "Planet_Btn_Drum.ojs";
        SceneArt {
            placements: vec![
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: String::new(),
                    code: PLANET_BUTTON_FREE_CODE,
                    sprite: Some(FREE_SPRITE.to_owned()),
                    rect: Rect {
                        x: 614,
                        y: 32,
                        w: 99,
                        h: 112,
                    },
                },
                ScenePlacement {
                    role: PlacementRole::Single,
                    id: String::new(),
                    code: PLANET_BUTTON_DRUM_CODE,
                    sprite: Some(DRUM_SPRITE.to_owned()),
                    rect: Rect {
                        x: 510,
                        y: 32,
                        w: 99,
                        h: 112,
                    },
                },
            ],
            figures: HashMap::from([
                (
                    FREE_SPRITE.to_owned(),
                    anchored_two_frame_figure(99, 112, 614, 32, [0x1F, 0x00], [0x00, 0x7C]),
                ),
                (
                    DRUM_SPRITE.to_owned(),
                    anchored_two_frame_figure(99, 112, 510, 32, [0x1F, 0x00], [0x00, 0x7C]),
                ),
            ]),
        }
    }

    /// Pinned pixels of one dimmed plate: the staged `.bnd` rect
    /// owned by `code` in `quest_and_tutorial_art`.
    fn quest_or_tutorial_rect(code: u32, buf: &[u32]) -> Vec<u32> {
        let x0 = if code == super::super::side::PLANET_BUTTON_FREE_CODE {
            614
        } else {
            510
        };
        (32..144)
            .flat_map(|y| (x0..x0 + 99).map(move |x| buf[y * 800 + x]))
            .collect()
    }

    #[test]
    fn quest_and_tutorial_emblems_hold_rest_across_press_and_selection() {
        // Ticket #137 (rule pinned in `docs/planet.md`): QUEST and
        // TUTORIAL never light — rest, hover, press, and a picked
        // H/M/G family all paint the same plate pixels. The sweep
        // covers both emblem codes in every pointer/selection
        // state; the exe shows them dim in all four (byte-identical
        // rects rest-to-dialog-open, `clean-emblem` series).
        use super::super::side::{
            PlanetAction, PlanetBand, PLANET_BUTTON_DRUM_CODE, PLANET_BUTTON_FREE_CODE,
        };
        let art = quest_and_tutorial_art();
        let paint = |selection: Option<PlanetBand>,
                     hover: Option<PlanetAction>,
                     press: Option<PlanetAction>| {
            let mut buf = vec![0u32; 800 * 600];
            paint_planet_scene(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &art,
                selection,
                hover,
                press,
                PlanetPromoPaint::spinning(1),
            );
            buf
        };
        let rest = paint(None, None, None);
        let states = [
            (
                "hover",
                paint(None, Some(PlanetAction::Emblem { band: None }), None),
            ),
            (
                "press",
                paint(None, None, Some(PlanetAction::Emblem { band: None })),
            ),
            ("hard-selected", paint(Some(PlanetBand::Hard), None, None)),
        ];
        for code in [PLANET_BUTTON_FREE_CODE, PLANET_BUTTON_DRUM_CODE] {
            let rest_rect = quest_or_tutorial_rect(code, &rest);
            for (state, buf) in &states {
                assert_eq!(
                    quest_or_tutorial_rect(code, buf),
                    rest_rect,
                    "code {code:#X} holds its rest plate while {state}"
                );
            }
        }
    }

    #[test]
    #[ignore = "superseded by #361: rest uses the baked sheet, this fixture has no sheet layer"]
    fn quest_and_tutorial_rest_paints_the_dimmed_frame() {
        // Historical model: the exe runs both plates at ~0.32x staged
        // brightness (`docs/planet.md`). The matched #361 A/B settled
        // the mechanism — the dim plates are baked into
        // `State_Planet.ojs`, and the button bodies are not mounted at
        // rest — so this desaturated-button fixture is superseded by
        // the sheet-based rest guard. Kept ignored as evidence history.

        use super::super::side::{PLANET_BUTTON_DRUM_CODE, PLANET_BUTTON_FREE_CODE};
        const BODY_WORD: u16 = 0x7C00;
        let dimmed = desaturate_rgb555(BODY_WORD);
        assert_ne!(
            dimmed, BODY_WORD,
            "the modeled frames are provably distinct"
        );
        let art = quest_and_tutorial_art();
        let mut buf = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            None,
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        for (code, x) in [
            (PLANET_BUTTON_FREE_CODE, 614),
            (PLANET_BUTTON_DRUM_CODE, 510),
        ] {
            assert_eq!(
                buf[82 * 800 + (x + 49)],
                pack_xrgb(
                    ((dimmed >> 10) << 3) as u8,
                    (((dimmed >> 5) & 0x1F) << 3) as u8,
                    ((dimmed & 0x1F) << 3) as u8
                ),
                "code {code:#X} rests dimmed, not full-colour"
            );
        }
    }

    /// Six distinct record colours at the PLAYSTOP anchor, standing
    /// in for the six 26x24 raw records of the shipped OPI body
    /// (#711 pin). Record 0 is the spinning stop square, record 3 the
    /// stopped play triangle, record 1 the hover face.
    fn playstop_record_art() -> SceneArt {
        use super::super::side::PLANET_PROMO_STOP_CODE;
        let sprite = "Planet_Btn_Promotion_PlayStop.ojs";
        let mut playstop = AvatarFigure::default();
        for word in [0x001Fu16, 0x7C00, 0x7C1F, 0x03E0, 0x03FF, 0x7FFF] {
            playstop.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.to_le_bytes().repeat(26 * 24), 26, 24)
                    .expect("playstop record"),
                dx: 387,
                dy: 511,
                colorkey: 0xFFFF,
            });
        }
        SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_PROMO_STOP_CODE,
                sprite: Some(sprite.to_owned()),
                rect: Rect {
                    x: 387,
                    y: 511,
                    w: 26,
                    h: 24,
                },
            }],
            figures: HashMap::from([(sprite.to_owned(), playstop)]),
        }
    }

    fn paint_playstop_pixel(promo: PlanetPromoPaint, hover: bool) -> u32 {
        let art = playstop_record_art();
        let hover_action = hover.then_some(super::super::side::PlanetAction::Promo {
            cell: super::super::side::PlanetPromo::Stop,
        });
        let mut buf = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            None,
            hover_action,
            None,
            promo,
        );
        buf[520 * 800 + 400]
    }

    #[test]
    fn promo_spinning_paints_the_stop_face_record() {
        // Live caller chain: main.rs::run_play -> setup_online ->
        // live_art::stage_interface_scenes -> ScreenGame::paint_planet ->
        // paint_planet_scene -> paint_planet_board ->
        // paint_planet_playstop_face. Exe capture: boot rest and the
        // second click both match raw record 0, 624/624 px (#711).
        assert_eq!(
            paint_playstop_pixel(PlanetPromoPaint::spinning(1), false),
            pack_xrgb(0x00, 0x00, 0xFF),
            "spinning must paint the stop square, not the stacked top"
        );
    }

    #[test]
    fn promo_stopped_paints_the_play_face_record() {
        // Same live chain after PLAYSTOP toggles the rotation flag:
        // the #711 capture matches raw record 3, 624/624 px both
        // immediately and after two 3000 ms ticks.
        assert_eq!(
            paint_playstop_pixel(PlanetPromoPaint::stopped(1), false),
            pack_xrgb(0x00, 0xFF, 0x00),
            "stopped must paint the play triangle, not the stop square"
        );
    }

    #[test]
    fn promo_playstop_hover_paints_the_second_frame() {
        // Hover composes on top of the state face: its pinned record 1
        // still wins while the carousel spins (224 changed px at
        // (392,515)-(407,529), `docs/hover.md`).
        assert_eq!(
            paint_playstop_pixel(PlanetPromoPaint::spinning(1), true),
            pack_xrgb(0xFF, 0x00, 0x00),
            "hover swaps to record 1, not the state face"
        );
    }

    #[test]
    fn promo_banner_frame_follows_the_carousel_index() {
        // The banner body paints its selected frame alone (exe
        // refresh arm VA `0x48e4b0` → `set_frame(index)`): index 1
        // shows the first frame, index 3 the top frame.
        use super::super::side::PLANET_PROMO_IMAGE_CODE;
        let sprite = "Planet_Promotion.ojs";
        let mut banner = AvatarFigure::default();
        for (word, anchor) in [
            (0x7C00u16, (49, 163)),
            (0x03E0u16, (49, 163)),
            (0x001Fu16, (49, 163)),
        ] {
            banner.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.to_le_bytes().repeat(16), 4, 4).unwrap(),
                dx: anchor.0,
                dy: anchor.1,
                colorkey: 0xFFFF,
            });
        }
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_PROMO_IMAGE_CODE,
                sprite: Some(sprite.to_owned()),
                rect: Rect {
                    x: 49,
                    y: 163,
                    w: 701,
                    h: 333,
                },
            }],
            figures: HashMap::from([(sprite.to_owned(), banner)]),
        };
        let paint_index = |index: u32| {
            let mut buf = vec![0u32; 800 * 600];
            paint_planet_scene(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &art,
                None,
                None,
                None,
                PlanetPromoPaint::spinning(index),
            );
            buf[164 * 800 + 50]
        };
        assert_eq!(
            paint_index(1),
            pack_xrgb(0xFF, 0x00, 0x00),
            "boot shows the first banner frame"
        );
        assert_eq!(
            paint_index(3),
            pack_xrgb(0x00, 0x00, 0xFF),
            "index 3 shows the top banner frame"
        );
    }

    #[test]
    fn promo_banner_paints_only_the_selected_frame() {
        // Real 7.55 bytes (skips without install-root): the banner
        // stages three 701x333 frames sharing the (49,163) anchor with
        // colorkey-0 holes. The exe's image control mounts ONE frame
        // per view (refresh arm VA `0x48e4b0` → `set_frame`), so the
        // view at index N must equal the same painter fed a one-frame
        // body of frame N — the #170 stacked rest let the selected
        // frame's holes show the sibling frames (ghost of the previous
        // banner where the exe shows the dialog backdrop).
        let Some(archive) = interface_archive() else {
            return;
        };
        let staged = stage_state(&archive, State::Planet, StageContext::default())
            .art
            .expect("planet scene stages with real bytes");
        let art = SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        };
        let sprite = art
            .placements
            .iter()
            .find(|placed| placed.code == PLANET_PROMO_IMAGE_CODE)
            .expect("promotion banner stages")
            .sprite
            .clone()
            .expect("banner names a sprite");
        let banner = art.figures.get(&sprite).expect("banner figure loads");
        assert!(
            banner.len() >= 2,
            "the guard needs the multi-frame banner body"
        );

        /// The scene art with the banner body swapped for a one-frame
        /// body carrying only record `keep` — the exe control with one
        /// record mounted shows it at the same anchor.
        fn with_one_frame_banner(art: &SceneArt, sprite: &str, keep: usize) -> SceneArt {
            let mut body = AvatarFigure::default();
            if let Some(layer) = art.figures[sprite].layer(keep) {
                body.push_layer(layer.clone());
            }
            let mut figures = art.figures.clone();
            figures.insert(sprite.to_owned(), body);
            SceneArt {
                placements: art.placements.clone(),
                figures,
            }
        }

        let paint = |art: &SceneArt, index: u32| {
            let mut buf = vec![0u32; 800 * 600];
            paint_planet_scene(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                art,
                None,
                None,
                None,
                PlanetPromoPaint::spinning(index),
            );
            buf
        };
        let expected_one = paint(&with_one_frame_banner(&art, &sprite, 0), 1);
        let expected_two = paint(&with_one_frame_banner(&art, &sprite, 1), 1);
        let one = paint(&art, 1);
        let two = paint(&art, 2);
        assert_ne!(one, expected_two, "the guard needs distinct frames");
        assert_eq!(one, expected_one, "index 1 paints only frame 1");
        assert_eq!(
            two, expected_two,
            "index 2 paints only frame 2 — no sibling frame in the holes"
        );
    }

    #[test]
    fn planet_zero_key_black_plates_cover_the_board_art() {
        // Live caller chain: main.rs::run_play -> setup_online ->
        // live_art::stage_interface_scenes -> ScreenGame::paint_planet ->
        // paint_planet_scene -> paint_planet_board. Both bodies stage
        // with colorkey 0; 7.55 installs a DirectDraw source colorkey
        // only for a nonzero key (docs/claims-render.md rule 4), so a
        // stored black pixel must cover the board art beneath it.
        let Some(archive) = interface_archive() else {
            return;
        };
        let staged = stage_state(&archive, State::Planet, StageContext::default())
            .art
            .expect("planet scene stages with real bytes");
        let art = SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        };
        let sentinel = pack_xrgb(0xFF, 0x00, 0xFF);
        let mut buf = vec![sentinel; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            None,
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        let black = pack_xrgb(0, 0, 0);
        assert_eq!(
            buf[452 * 800 + 586],
            black,
            "State_Planet black plate pixel paints over the board art"
        );
        assert_eq!(
            buf[258 * 800 + 345],
            black,
            "Planet_Promotion black plate pixel hides the board art"
        );
    }

    #[test]
    fn waiting_notice_board_zero_words_leave_the_header_backdrop_visible() {
        // Live caller chain: main.rs::run_play -> setup_online ->
        // live_art::stage_interface_scenes -> ScreenGame::paint ->
        // paint_waiting_scene -> SceneArt::paint_backdrop ->
        // paint_scene_figures -> AvatarFigure::paint_scaled_anchors ->
        // blit_rgb555. `Lanking_NoticeBoard.oji` stores key 0 and its
        // transparent pixels are zero words: the band must not hide
        // the header backdrop, unlike the key-0 `.ojs` plates.
        let Some(archive) = interface_archive() else {
            return;
        };
        let staged = stage_state(&archive, State::Waiting, StageContext::default())
            .art
            .expect("waiting scene stages with real bytes");
        let art = SceneArt {
            placements: staged.placements,
            figures: staged.figures,
        };
        let sentinel = pack_xrgb(0xFF, 0x00, 0xFF);
        let mut buf = vec![sentinel; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        assert_ne!(
            buf[50 * 800 + 400],
            pack_xrgb(0, 0, 0),
            "notice-board zero word leaves the header backdrop visible"
        );
    }

    #[test]
    fn channel_flanks_show_opaque_vinyl_through_zero_key_black_pixels() {
        // Live caller chain: main.rs::run_play -> setup_online ->
        // live_art::stage_channel_dialog -> ScreenGame::paint_channel ->
        // paint_channel_scene -> paint_planet_board ->
        // paint_planet_promo_frame. The dialog sheet's key is 0x7D5F
        // (transparent), but the board and promo bodies carry key 0:
        // their black pixels are opaque in 7.55, so the flanks show
        // dark vinyl instead of the board art under the dialog holes.
        let Some(archive) = interface_archive() else {
            return;
        };
        let staged_board = stage_state(&archive, State::Planet, StageContext::default())
            .art
            .expect("planet scene stages with real bytes");
        let staged_dialog = crate::staging::stage_dialog(
            &archive,
            crate::staging::Dialog::ChannelInfo,
            StageContext::default(),
        )
        .art
        .expect("channel dialog stages with real bytes");
        let board = SceneArt {
            placements: staged_board.placements,
            figures: staged_board.figures,
        };
        let dialog = SceneArt {
            placements: staged_dialog.placements,
            figures: staged_dialog.figures,
        };
        let mut buf = vec![pack_xrgb(0xFF, 0x00, 0xFF); 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            0,
            0,
            &dialog,
            Some(StagedPlanetBoard {
                art: &board,
                promo: PlanetPromoPaint::spinning(1),
            }),
            None,
        );
        let black = pack_xrgb(0, 0, 0);
        for (x, y) in [(182, 171), (62, 226), (170, 279), (66, 289)] {
            assert_eq!(
                buf[y * 800 + x],
                black,
                "flank pixel ({x},{y}) is opaque vinyl, not staged board art"
            );
        }
    }

    #[test]
    fn gear_hover_restores_the_first_frame() {
        // OPTION/EXIT and the promo arrows (`docs/hover.md`): hover
        // repaints body frame 0 at full colour over the dimmed rest.
        use super::super::side::{PlanetAction, PLANET_OPTION_CODE};
        let sprite = "Planet_Btn_Option.ojs";
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_OPTION_CODE,
                sprite: Some(sprite.to_owned()),
                rect: Rect {
                    x: 710,
                    y: 561,
                    w: 38,
                    h: 38,
                },
            }],
            figures: HashMap::from([(
                sprite.to_owned(),
                anchored_two_frame_figure(38, 38, 710, 561, [0x1F, 0x00], [0x00, 0x7C]),
            )]),
        };
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let red = pack_xrgb(0xFF, 0x00, 0x00);
        let mut rest = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut rest,
                w: 800,
                h: 600,
            },
            &art,
            None,
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert_eq!(rest[575 * 800 + 725], red, "rest keeps the top layer");
        let mut hovered = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut hovered,
                w: 800,
                h: 600,
            },
            &art,
            None,
            Some(PlanetAction::OptionsDialog),
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert_eq!(hovered[575 * 800 + 725], blue, "hover restores frame 0");
    }

    #[test]
    fn exit_hover_restores_the_first_frame() {
        // Ticket #163 (planet leg): the EXIT disc rides the same
        // hover arm as OPTION (`docs/hover.md`: EXIT diff bbox
        // 754,565-783,594, hover paints body frame 0 at full
        // colour). Two stacked layers stand in for rest over
        // hover-top, so the guard pins the repaint, not the
        // exe's desaturated rest (OPEN, `docs/hover.md`).
        use super::super::side::{PlanetAction, PLANET_EXIT_CODE};
        let sprite = "Planet_Btn_Exit.ojs";
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_EXIT_CODE,
                sprite: Some(sprite.to_owned()),
                rect: Rect {
                    x: 750,
                    y: 561,
                    w: 38,
                    h: 38,
                },
            }],
            figures: HashMap::from([(
                sprite.to_owned(),
                anchored_two_frame_figure(38, 38, 750, 561, [0x1F, 0x00], [0x00, 0x7C]),
            )]),
        };
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let red = pack_xrgb(0xFF, 0x00, 0x00);
        let mut rest = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut rest,
                w: 800,
                h: 600,
            },
            &art,
            None,
            None,
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert_eq!(rest[575 * 800 + 765], red, "rest keeps the top layer");
        let mut hovered = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut hovered,
                w: 800,
                h: 600,
            },
            &art,
            None,
            Some(PlanetAction::QuitConfirm),
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert_eq!(hovered[575 * 800 + 765], blue, "hover restores frame 0");
    }

    /// The offA/onA-modeled hover pair (ticket #63, rule in
    /// `docs/parity/art-delta-guards.md`): the measured dimmed-rest pattern for the
    /// planet board buttons (`docs/hover.md`, OPTION gear diff bbox
    /// (714,565)-(743,594), 615 changed px — hover paints body
    /// frame 0 at full colour over the desaturated rest frame 0).
    /// The staged body has the real-art shape (one layer,
    /// full-colour frame 0) and the rest frame is the desaturated
    /// decode of that same frame, so the two frames differ at every
    /// saturated pixel — provably distinct, unlike the stacked
    /// two-layer stand-ins whose frames differ by construction.
    /// Synthetic words only; no captured bytes.
    struct OffOnHoverPair {
        /// Staged single-frame body (the figure the scene paints).
        figure: AvatarFigure,
        /// Frame 0 at full colour — the hover paint.
        hover_word: u16,
        /// The desaturated frame 0 — the rest paint once ticket #26
        /// implements rest-dim.
        rest_word: u16,
    }

    /// Archive-entry name of the synthetic offA/onA body.
    const OFFON_HOVER_SPRITE: &str = "OffOn_Gear_Fixture.ojs";

    /// Build the pair: a 38x38 saturated red body at the gear disc's
    /// pinned anchor (710,561), full colour vs its desaturated
    /// decode.
    fn offon_hover_fixture() -> OffOnHoverPair {
        const BODY_W: usize = 38;
        const BODY_H: usize = 38;
        const FULL_COLOUR_RED: u16 = 0x7C00;
        let hover_word = FULL_COLOUR_RED;
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(
                &FULL_COLOUR_RED.to_le_bytes().repeat(BODY_W * BODY_H),
                BODY_W,
                BODY_H,
            )
            .expect("fixture body decodes"),
            dx: 710,
            dy: 561,
            colorkey: 0xFFFF,
        });
        OffOnHoverPair {
            figure,
            hover_word,
            rest_word: desaturate_rgb555(hover_word),
        }
    }

    /// Rec. 601 luma of an RGB555 word, folded back into all three
    /// channels (the modeled desaturation of the rest frame).
    fn desaturate_rgb555(word: u16) -> u16 {
        let red = u32::from(word >> 10);
        let green = u32::from((word >> 5) & 0x1F);
        let blue = u32::from(word & 0x1F);
        let luma = ((77 * red + 150 * green + 29 * blue) / 256) as u16;
        luma << 10 | luma << 5 | luma
    }

    /// Paint the planet scene at 800x600 into an owned buffer.
    fn painted_planet_frame(art: &SceneArt, hover: Option<PlanetAction>) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            art,
            None,
            hover,
            None,
            PlanetPromoPaint::spinning(1),
        );
        buf
    }

    #[test]
    #[ignore = "rest-dim is OPEN: ticket #26's fix activates this guard (kept red-by-design per the #63 art-delta-guard rule; see also #26)"]
    fn gear_hover_repaints_over_the_dimmed_rest_frame() {
        // Ticket #63 (rule: `docs/parity/art-delta-guards.md` "Art-dependent delta
        // guards"): with the offA/onA-modeled fixture — rest =
        // desaturated frame 0, hover = frame 0 at full colour — the
        // guard asserts the rest ink differs from the hover ink at
        // the pinned pixel. Today the rest stack paints frame 0 at
        // full colour, so hover repaints identical pixels and this
        // guard is red; ticket #26's rest-dim flips it to running.
        let fixture = offon_hover_fixture();
        assert_ne!(
            fixture.rest_word, fixture.hover_word,
            "the modeled frames are provably distinct at the pinned pixel"
        );
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_OPTION_CODE,
                sprite: Some(OFFON_HOVER_SPRITE.to_owned()),
                rect: Rect {
                    x: 710,
                    y: 561,
                    w: 38,
                    h: 38,
                },
            }],
            figures: HashMap::from([(OFFON_HOVER_SPRITE.to_owned(), fixture.figure)]),
        };
        let pinned = 575 * 800 + 725;
        let rest = painted_planet_frame(&art, None);
        let hovered = painted_planet_frame(&art, Some(PlanetAction::OptionsDialog));
        assert_ne!(
            rest[pinned], hovered[pinned],
            "rest-dim keeps the measured delta: hover must repaint the dimmed frame"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn planet_hover_without_staged_figure_paints_nothing() {
        // No highlight without art: the hovered control stages no
        // figure, so the frame stays untouched (no invented pixels).
        use super::super::side::{PlanetAction, PlanetBand, PLANET_BUTTON_EASY_CODE};
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Single,
                id: String::new(),
                code: PLANET_BUTTON_EASY_CODE,
                sprite: Some("Planet_Btn_Easy.ojs".to_owned()),
                rect: Rect {
                    x: 87,
                    y: 32,
                    w: 99,
                    h: 112,
                },
            }],
            figures: HashMap::new(),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
            None,
            Some(PlanetAction::Emblem {
                band: Some(PlanetBand::Easy),
            }),
            None,
            PlanetPromoPaint::spinning(1),
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn loading_cover_wins_over_planet_art() {
        // Cover present means the song load succeeded, so the exe
        // `MusicLoading.ojs` fallback (mounted only when the load
        // returns 0) stays out: no planet pixel survives.
        let mut planet = AvatarFigure::default();
        planet.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x1F, 0x7C], 1, 1).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let art = SceneArt {
            placements: unbound_scene_placements(&[LOADING_SPRITE.to_owned()]),
            figures: HashMap::from([
                (LOADING_SPRITE.to_owned(), planet),
                (LOADING_COVER_SPRITE.to_owned(), cover_figure()),
            ]),
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_loading_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
        );
        let magenta = pack_xrgb(255, 0, 255);
        assert!(!buf.contains(&magenta), "planet fallback stays out");
        assert_eq!(buf[0], pack_xrgb(255, 0, 0), "cover owns the frame");
    }

    /// 2x2 opaque stand-in for a decoded OJN cover (distinct corners;
    /// never real art — geometry only).
    fn cover_figure() -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image {
                w: 2,
                h: 2,
                words: vec![0x7C00, 0x03E0, 0x001F, 0x7FFF],
            },
            dx: 0,
            dy: 0,
            colorkey: 0x0000,
        });
        figure
    }

    #[test]
    fn loading_fill_spans_the_walk_fraction_at_the_painter_seam() {
        // Seam-only guard: the live scene call is gated off by
        // [`LOADING_BAR_VISIBLE`] until the exe bar is pinned, so this
        // pins the hidden painter's math, not a player-visible frame.
        // 32x24 frame: track x8 w16, half fill x8..16 at 0.5.
        let mut buf = vec![0u32; 32 * 24];
        paint_loading_bar_with_progress(
            &mut Frame {
                buf: &mut buf,
                w: 32,
                h: 24,
            },
            0.5,
        );
        let green = pack_xrgb(0x30, 0xB0, 0x30);
        assert_eq!(buf[4 * 32 + 8], green, "fill starts at the track");
        assert_eq!(buf[4 * 32 + 15], green, "half fill reaches x15");
        assert_ne!(buf[4 * 32 + 16], green, "nothing past the fraction");
    }

    #[test]
    fn loading_fill_clamps_out_of_range_progress() {
        // NaN/negative/over-one fractions saturate to empty/full —
        // never a wrapped or inverted fill (PERF-25: no `==`).
        assert_eq!(loading_fill_width(16, f32::NAN), 0);
        assert_eq!(loading_fill_width(16, -0.5), 0);
        assert_eq!(loading_fill_width(16, 0.0), 0);
        assert_eq!(loading_fill_width(16, 2.0), 16);
        assert_eq!(loading_fill_width(16, 1.0), 16);
    }

    #[test]
    fn loading_caption_joins_title_and_artist() {
        // Song-table walk names the song: both halves join, one
        // half alone still names itself, blank paints nothing.
        let both = LoadingStatus {
            progress: 0.0,
            title: "Title".to_owned(),
            artist: "Artist".to_owned(),
        };
        assert_eq!(both.caption(), Some("Title - Artist".to_owned()));
        let title_only = LoadingStatus {
            title: "Title".to_owned(),
            ..LoadingStatus::default()
        };
        assert_eq!(title_only.caption(), Some("Title".to_owned()));
        assert_eq!(LoadingStatus::default().caption(), None);
    }

    #[test]
    fn loading_scene_paints_the_song_caption() {
        // Caption row below the bar (placeholder row 32x24: x8,
        // y24 clipped — so use 64x48: song at y36, scrim behind).
        let art = SceneArt::default();
        let status = LoadingStatus {
            progress: 1.0,
            title: "Title".to_owned(),
            artist: "Artist".to_owned(),
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_loading_scene_with_status(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &art,
            &status,
        );
        assert!(has_ink_near(&buf, art_title_ink()), "caption ink paints");
        assert!(buf.contains(&scrim_color()), "caption scrim paints");
        let blank = vec![0u32; 64 * 48];
        let mut probe = blank.clone();
        paint_loading_scene_with_status(
            &mut Frame {
                buf: &mut probe,
                w: 64,
                h: 48,
            },
            &art,
            &LoadingStatus::default(),
        );
        assert!(
            !probe.contains(&art_title_ink()),
            "blank caption paints no white ink"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn loading_bar_geometry_stays_open_placeholder() {
        // OPEN placeholder guard: the centered track/caption math
        // below is not exe-grounded (`MusicLoading.ojs` is a single
        // 800x600 body, `0x4e47a0` names no rect, adjacent bars are
        // other dialogs), so this test pins the placeholder formula
        // and fails loudly if anyone "grounds" it without evidence.
        let frame = Frame {
            buf: &mut vec![0u32; 64 * 48],
            w: 64,
            h: 48,
        };
        assert_eq!(loading_track_origin(&frame), (16, 16));
        assert_eq!(loading_track_width(&frame), 32);
        assert_eq!(loading_song_origin(&frame), (16, 36));
    }

    #[test]
    fn loading_placeholders_clear_every_measured_bar_body() {
        // Honest-OPEN negative pin (read-only 7.55 sprite-55 walks):
        // the centered placeholder track/caption match none of the
        // measured bar/panel bodies, so none can ground them.
        let frame = Frame {
            buf: &mut vec![0u32; 800 * 600],
            w: 800,
            h: 600,
        };
        let track = loading_track_origin(&frame);
        let width = loading_track_width(&frame);
        let song = loading_song_origin(&frame);
        assert_eq!((track, width, song), ((200, 292), 400, (200, 312)));
        for measured in [
            ((0, 0), (134, 37)),
            ((0, 0), (65, 7)),
            ((0, 0), (100, 45)),
            ((619, 529), (119, 23)),
            ((410, 210), (197, 179)),
        ] {
            assert_ne!((track, (width, 16)), measured);
            assert_ne!(song, measured.0);
        }
    }

    #[test]
    fn waiting_covers_paint_per_seat_rects() {
        // `Wait_Cover_Avatar.ojs` anchors at (0,0): its eight rect-
        // bound placements (COVERAVATAR1..8, codes 0x1650003D..44)
        // each paint at their own rect origin — the view-from-rect
        // case ui-draw.md leaves open, resolved for anchor-free
        // bodies. Occupied covers then take the live seat line (the
        // scrim covers the art pixel); empty covers stay untouched
        // so the staged empty panels show.
        let mut placements: Vec<ScenePlacement> = (0..8)
            .map(|seat| ScenePlacement {
                role: PlacementRole::Single,
                id: format!("IDC_IMAGE_SWAIT_COVERAVATAR{}", seat + 1),
                code: 0x1650_003D + seat as u32,
                sprite: Some("Wait_Cover_Avatar.ojs".to_owned()),
                rect: Rect {
                    x: 10 + seat * 60,
                    y: 200 + seat * 45,
                    w: 141,
                    h: 175,
                },
            })
            .collect();
        // The member line belongs to the seat template's AVATARINFO
        // child, not the cover body. These synthetic parents put its
        // ink at each cover origin so the old per-cover assertions stay
        // a pin on the cover/line pairing (#883).
        for (seat, parent_id) in WAITING_SEAT_PARENT_IDS.iter().enumerate() {
            placements.push(ScenePlacement {
                role: PlacementRole::Single,
                id: (*parent_id).to_owned(),
                code: 0x1601_5022 + seat as u32,
                sprite: None,
                rect: Rect {
                    x: 8 + (seat as i32) * 60,
                    y: 39 + (seat as i32) * 45,
                    w: 123,
                    h: 150,
                },
            });
        }
        placements.push(waiting_probe(
            "IDC_TEXT_SWAIT_AVATARINFO",
            None,
            (1, 157, 122, 172),
        ));
        let art = SceneArt {
            placements,
            figures: HashMap::from([("Wait_Cover_Avatar.ojs".to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        let red = pack_xrgb(255, 0, 0);
        // Empty wire slot 1 keeps its staged art pixel: no live line.
        // Slot 2 is locked in this snapshot and legitimately inks LOCK.
        assert_eq!(buf[245 * 800 + 70], red, "empty cover stays art-only");
        // Occupied slot 0 takes the live line: scrim opens at
        // (10 − 4, 200 − 2), covering the art pixel.
        assert_eq!(buf[198 * 800 + 6], scrim_color(), "seat line scrim");
        assert_ne!(buf[200 * 800 + 10], red, "live line covers art");
        assert!(has_ink_near(&buf, art_title_ink()));
        assert_eq!(buf[0], 0, "covers no longer stack at the frame corner");
    }

    /// #303 red guard (seam): the seat template's `BOSSMARK` clone
    /// stages only on the creator's seat. The host is seat 0; a
    /// duplicate of its own wire slot in the peer list would paint a
    /// second emblem on seat 1.
    #[test]
    fn master_emblem_stages_only_on_the_creator_seat() {
        let mut room = waiting_room();
        let guest = RoomMember {
            nickname: "guest".to_owned(),
            master: false,
            ..room.slots[0].member.clone().expect("host seated")
        };
        room.slots[1].member = Some(guest);
        let boss_placement = |seat: usize| ScenePlacement {
            role: PlacementRole::Clone,
            id: format!("IDC_BUTTON_SWAIT_EXTEND{}", seat + 1),
            code: 0x1650_002F,
            sprite: Some("Wait_BossMark.ojs".to_owned()),
            rect: Rect {
                x: 20 + (seat * 60) as i32,
                y: 40,
                w: 28,
                h: 28,
            },
        };
        let art = SceneArt {
            placements: vec![boss_placement(0), boss_placement(1)],
            figures: HashMap::from([("Wait_BossMark.ojs".to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &room,
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        let red = pack_xrgb(255, 0, 0);
        assert_eq!(buf[40 * 800 + 20], red, "creator seat shows the emblem");
        assert_ne!(
            buf[40 * 800 + 80],
            red,
            "no duplicate emblem on the peer seat"
        );
    }

    /// Synthetic two-record Waiting art for #343: record 0 is the
    /// plain card grid, record 1 the VS sheet, and the popup control
    /// paints at its bnd rect. The exe's image control sets one record;
    /// stacking both paints the VS sheet over the top cards.
    const WAITING_TEST_PLAIN: u16 = 0x04A1;
    const WAITING_TEST_VERSUS: u16 = 0x001F;
    const WAITING_TEST_POPUP: u16 = 0x7C00;

    fn waiting_role_art() -> SceneArt {
        let layer = |word: u16| FigureLayer {
            image: Rgb555Image::decode(&word.to_le_bytes().repeat(16), 4, 4).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        };
        let mut backdrop = AvatarFigure::default();
        backdrop.push_layer(layer(WAITING_TEST_PLAIN));
        backdrop.push_layer(layer(WAITING_TEST_VERSUS));
        let mut popup_figure = AvatarFigure::default();
        popup_figure.push_layer(layer(WAITING_TEST_POPUP));
        SceneArt {
            placements: vec![
                waiting_probe(
                    "IDC_BACK_SWAIT_1",
                    Some("State_Waiting.ojs"),
                    (100, 100, 104, 104),
                ),
                waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR1", None, (10, 200, 151, 375)),
                ScenePlacement {
                    code: 0x1650_0074,
                    ..waiting_probe(
                        "IDC_IMAGE_SWAIT_MUSIC_SELECT_POPUP",
                        Some("Waiting_Music_Select_Popup.ojs"),
                        (200, 200, 204, 204),
                    )
                },
            ],
            figures: HashMap::from([
                ("State_Waiting.ojs".to_owned(), backdrop),
                ("Waiting_Music_Select_Popup.ojs".to_owned(), popup_figure),
            ]),
        }
    }

    fn painted_waiting_role(role: MemberRole) -> Vec<u32> {
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &waiting_role_art(),
            &WaitingCaptions::default(),
            false,
            false,
            role,
        );
        buf
    }

    fn waiting_test_ink(word: u16) -> u32 {
        let expand =
            |channel: u16| u8::try_from((channel << 3) | (channel >> 2)).expect("five-bit channel");
        pack_xrgb(
            expand((word >> 10) & 31),
            expand((word >> 5) & 31),
            expand(word & 31),
        )
    }

    /// #343 red guard: a two-record `State_Waiting.ojs` backdrop
    /// must paint record 0 (the eight light-blue cards) at base-ready,
    /// and the Challenger-only popup must stay unpainted.
    #[test]
    fn base_ready_waits_on_the_plain_backdrop_without_a_select_popup() {
        let buf = painted_waiting_role(MemberRole::Spectator);
        assert_eq!(
            buf[100 * 800 + 100],
            waiting_test_ink(WAITING_TEST_PLAIN),
            "plain seat card"
        );
        assert_eq!(buf[200 * 800 + 200], 0, "no Challenger popup");
    }

    /// The exe arms the VS sheet and popup in its MemberRole 2 branch
    /// (waiting setup `0x4aac80`, state update `0x4ae350`).
    #[test]
    fn challenger_waits_on_the_versus_sheet_with_the_select_popup() {
        let buf = painted_waiting_role(MemberRole::Challenger);
        assert_eq!(
            buf[100 * 800 + 100],
            waiting_test_ink(WAITING_TEST_VERSUS),
            "Challenger VS sheet"
        );
        assert_eq!(
            buf[200 * 800 + 200],
            waiting_test_ink(WAITING_TEST_POPUP),
            "Challenger popup"
        );
    }

    /// Champion selects the VS sheet but never arms the Challenger popup.
    #[test]
    fn champion_waits_on_the_versus_sheet_without_the_select_popup() {
        let buf = painted_waiting_role(MemberRole::Champion);
        assert_eq!(
            buf[100 * 800 + 100],
            waiting_test_ink(WAITING_TEST_VERSUS),
            "Champion VS sheet"
        );
        assert_eq!(buf[200 * 800 + 200], 0, "no Challenger popup");
    }

    /// Verbatim `State_Waiting.bnd` rects for one region probe (local
    /// 116-rect walk; ordinal = `(code & 0xFFF) − 1`).
    fn waiting_probe(id: &str, sprite: Option<&str>, rect: (i32, i32, i32, i32)) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code: 0,
            sprite: sprite.map(str::to_owned),
            rect: Rect {
                x: rect.0,
                y: rect.1,
                w: u32::try_from(rect.2 - rect.0).unwrap_or(0),
                h: u32::try_from(rect.3 - rect.1).unwrap_or(0),
            },
        }
    }

    /// Placements-only stage (probe rects, no art bodies): the caption
    /// guards need the rects, never a decoded sprite.
    fn placements_stage(placements: &[ScenePlacement]) -> SceneArt {
        SceneArt {
            placements: placements.to_vec(),
            figures: HashMap::new(),
        }
    }

    /// Waiting room with every staged field set: champion seat 1,
    /// streak 3, song 757, speed code 5 (`X3`), one skill ring.
    fn champion_room() -> JoinedRoom {
        let mut room = waiting_room();
        let mut challenger = room.slots[0].member.clone().unwrap();
        challenger.master = false;
        room.slots[1] = RoomSlot {
            index: 1,
            state_code: 1,
            member: Some(challenger),
        };
        room.champion = Some(1);
        room.champion_streak = Some(3);
        room.music_id = 757;
        room.speed = 5;
        room.skills = vec![2101];
        room
    }

    #[test]
    fn waiting_cover_names_ink_the_template_text_cell() {
        // Quiesced 7.55 pair `~/scratch/paired-shots-v3/states/waiting-master/exe.png`:
        // `Lv.1:t207probe` inks inside the red team plate at y 217, not
        // on the cover art at y 53. The seat template's `AVATARINFO`
        // TEXT rect `(1,157)` plus seat-parent `(37,55)` is the pin.
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR1", None, (28, 53, 169, 228)),
            waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR2", None, (170, 53, 311, 228)),
            waiting_probe(WAITING_SEAT_PARENT_IDS[0], None, (37, 55, 160, 205)),
            waiting_probe(WAITING_SEAT_PARENT_IDS[1], None, (179, 55, 302, 205)),
            waiting_probe("IDC_TEXT_SWAIT_AVATARINFO", None, (1, 157, 122, 172)),
        ];
        let mut buf = vec![background_of(Screen::Waiting); 800 * 600];
        paint_waiting_covers(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
            None,
        );
        let mut expected = vec![background_of(Screen::Waiting); 800 * 600];
        for cover_x in [28, 170] {
            fill_rect(
                &mut Frame {
                    buf: &mut expected,
                    w: 800,
                    h: 600,
                },
                &Rect {
                    x: cover_x - 6,
                    y: 53,
                    w: 3,
                    h: 5,
                },
                pack_xrgb(0x30, 0xE0, 0x30),
            );
        }
        for parent_x in [37, 179] {
            paint_room_detail_line(
                &mut Frame {
                    buf: &mut expected,
                    w: 800,
                    h: 600,
                },
                parent_x + 1 + 1,
                55 + 157 + 4,
                "Lv.9:host",
                art_title_ink(),
            );
        }
        let mismatch = buf
            .iter()
            .zip(expected.iter())
            .position(|(actual, want)| actual != want);
        assert!(
            mismatch.is_none(),
            "first mismatch at {:?}; names must ink the template cell in Lv.<n>:<name> form",
            mismatch
        );
        assert!(
            !buf[40 * 800..100 * 800].contains(&scrim_color()),
            "no cover-art caption remains above the team plate"
        );
        // Ready dots keep their own cover-line origin.
        assert_eq!(buf[55 * 800 + 165], pack_xrgb(0x30, 0xE0, 0x30));
    }

    #[test]
    fn waiting_zero_skill_counts_leave_the_x_boxes_bare() {
        // Exe oracle: the three SKILL X plates are ink-free when no
        // owned-item count is staged. Rust printed `0` in all three.
        let placements = vec![
            waiting_probe("IDC_TEXT_SWAIT_1SKILLCOUNT", None, (712, 208, 731, 223)),
            waiting_probe("IDC_TEXT_SWAIT_2SKILLCOUNT", None, (712, 246, 731, 261)),
            waiting_probe("IDC_TEXT_SWAIT_3SKILLCOUNT", None, (712, 284, 731, 299)),
        ];
        let mut buf = vec![background_of(Screen::Waiting); 800 * 600];
        paint_waiting_skillcounts(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[0, 0, 0],
            None,
            &placements,
        );
        assert_eq!(
            buf,
            vec![background_of(Screen::Waiting); 800 * 600],
            "a zero count must not ink the baked X plate"
        );
    }

    /// The room panel's staged wells plus its avatar slot, at the rect
    /// table the 7.55 `State_Room.bnd` pairs (`IDC_TEXT_SROOM_*`).
    fn room_user_panel_placements() -> Vec<ScenePlacement> {
        [
            (ROOM_NICKNAME_ID, 0x1340_0012, (651, 92, 119, 17)),
            (ROOM_PANEL_LEVEL_ID, 0x1340_003E, (688, 285, 89, 11)),
            (ROOM_PANEL_EXP_ID, 0x1340_003F, (688, 308, 89, 11)),
            (ROOM_PANEL_GRADE_ID, 0x1340_0040, (688, 331, 89, 11)),
            (ROOM_PANEL_CASH_ID, 0x1340_0041, (714, 354, 63, 11)),
            (ROOM_PANEL_NEXT_UP_EXP_ID, 0x1340_0042, (720, 377, 57, 11)),
            (ROOM_PANEL_PENALTY_COUNT_ID, 0x1340_0043, (714, 400, 63, 11)),
            (ROOM_PANEL_PENALTY_LEVEL_ID, 0x1340_0044, (714, 423, 63, 11)),
            (ROOM_PANEL_BAND_ID, 0x1340_0045, (705, 446, 72, 11)),
            (ROOM_AVATAR_ID, 0x1370_0013, (649, 117, 123, 150)),
        ]
        .into_iter()
        .map(|(id, code, (x, y, w, h))| ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        })
        .collect()
    }

    /// Inked pixels inside one native rect.
    fn ink_in_rect(buf: &[u32], w: usize, rect: Rect) -> usize {
        let mut count = 0;
        for y in rect.y..rect.y + i32::try_from(rect.h).expect("h fits") {
            for x in rect.x..rect.x + i32::try_from(rect.w).expect("w fits") {
                if buf[y as usize * w + x as usize] != 0 {
                    count += 1;
                }
            }
        }
        count
    }

    #[test]
    fn room_user_panel_paints_character_values_into_staged_wells() {
        let placements = room_user_panel_placements();
        let character = CharacterInfo {
            nickname: "tester".to_owned(),
            level: 2,
            experience: 2133,
            bonus_point: 7,
            penalty_count: 1,
            penalty_level: 3,
            ..CharacterInfo::default()
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(&character),
            None,
            0,
        );
        for id in [
            ROOM_NICKNAME_ID,
            ROOM_PANEL_LEVEL_ID,
            ROOM_PANEL_EXP_ID,
            ROOM_PANEL_CASH_ID,
            ROOM_PANEL_NEXT_UP_EXP_ID,
            ROOM_PANEL_PENALTY_COUNT_ID,
            ROOM_PANEL_PENALTY_LEVEL_ID,
        ] {
            let rect = placements
                .iter()
                .find(|placed| placed.id == id)
                .expect("well staged")
                .rect;
            assert!(
                ink_in_rect(&buf, 800, rect) > 0,
                "{id} must carry its value ink"
            );
        }
        // The grade and band wells carry the exe's own placeholder.
        for id in [ROOM_PANEL_GRADE_ID, ROOM_PANEL_BAND_ID] {
            let rect = placements
                .iter()
                .find(|placed| placed.id == id)
                .expect("well staged")
                .rect;
            assert!(ink_in_rect(&buf, 800, rect) > 0, "{id} shows 준비중..");
        }
    }

    #[test]
    fn room_user_panel_without_a_character_paints_only_the_placeholders() {
        let placements = room_user_panel_placements();
        let mut buf = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            None,
            0,
        );
        for id in [
            ROOM_NICKNAME_ID,
            ROOM_PANEL_LEVEL_ID,
            ROOM_PANEL_EXP_ID,
            ROOM_PANEL_CASH_ID,
            ROOM_PANEL_NEXT_UP_EXP_ID,
            ROOM_PANEL_PENALTY_COUNT_ID,
            ROOM_PANEL_PENALTY_LEVEL_ID,
        ] {
            let rect = placements
                .iter()
                .find(|placed| placed.id == id)
                .expect("well staged")
                .rect;
            assert_eq!(ink_in_rect(&buf, 800, rect), 0, "{id} stays empty");
        }
        assert!(
            ink_in_rect(
                &buf,
                800,
                placements
                    .iter()
                    .find(|placed| placed.id == ROOM_PANEL_GRADE_ID)
                    .expect("grade well")
                    .rect
            ) > 0
        );
    }

    #[test]
    fn room_user_panel_paints_next_up_exp_from_current_experience() {
        // The 7.55 room capture (`/home/arnim/scratch/ac-room.png`, tester
        // Lv 2) shows `2133` in `Next Up Exp.`, equal to the `Exp.` well —
        // a single-capture pin, so this guard names the observed value,
        // not a threshold rule (see the `NEXTUPEXP` note on the painter).
        let placements = room_user_panel_placements();
        let character = CharacterInfo {
            experience: 2133,
            ..CharacterInfo::default()
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(&character),
            None,
            0,
        );
        let rect = placements
            .iter()
            .find(|placed| placed.id == ROOM_PANEL_NEXT_UP_EXP_ID)
            .expect("next-up well staged")
            .rect;
        assert!(
            ink_in_rect(&buf, 800, rect) > 0,
            "Next Up Exp. must ink the staged well"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn room_user_panel_leaves_the_desc_well_empty_without_a_web_source() {
        // The bottom plate is the staged `IDC_TEXT_SROOM_DESC` well
        // (`0x13400046`, `(646,468)-(776,534)`): the 7.55 room capture
        // shows web-served notice text there, which the client never
        // receives — so the plate paints bare even with a character.
        let mut placements = room_user_panel_placements();
        placements.push(ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_TEXT_SROOM_DESC".to_owned(),
            code: 0x1340_0046,
            sprite: None,
            rect: Rect {
                x: 646,
                y: 468,
                w: 130,
                h: 66,
            },
        });
        let character = CharacterInfo {
            nickname: "tester".to_owned(),
            level: 2,
            experience: 2133,
            ..CharacterInfo::default()
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(&character),
            None,
            0,
        );
        let rect = placements
            .iter()
            .find(|placed| placed.id == "IDC_TEXT_SROOM_DESC")
            .expect("desc well staged")
            .rect;
        assert_eq!(
            ink_in_rect(&buf, 800, rect),
            0,
            "the DESC plate stays bare without a web source"
        );
    }

    #[test]
    fn room_user_panel_paints_the_worn_figure_in_its_slot() {
        let placements = room_user_panel_placements();
        let slot = placements
            .iter()
            .find(|placed| placed.id == ROOM_AVATAR_ID)
            .expect("slot staged")
            .rect;
        let figure = icon_figure(&[0x7C00]);
        let mut buf = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            Some(&figure),
            0,
        );
        assert!(
            ink_in_rect(&buf, 800, slot) > 0,
            "figure ink must land inside the staged slot"
        );
    }

    /// Read the pinned avatar archive, `None` without install-root.
    fn avatar_archive() -> Option<Vec<u8>> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image/avatar.opa"),
        )
        .ok()
    }

    /// The live tester2 equipment row (`~/memoryer-local` SQLite
    /// `O2JAM.db`, `t_o2jam_item` `USER_INDEX_ID=2`, read 2026-11-13,
    /// ticket #171 round 2): hair 16, body 30, top 376, bottom 383,
    /// shoes 389 at `Equip1..Equip5` plus the default male face 35 at
    /// `Equip12` (the pinned type-5 → slot `0xb` rule,
    /// `docs/equip.md`); every other `Equip` column is 0. The server
    /// copies the row 1:1 into the 17-slot `CharacterInfo.equipment`
    /// (live wire probe under the stack-every-slot picker read 60 and
    /// 447 layers; the picker now wears one body per part block, so the
    /// face-only row now stages the pinned default body beneath its
    /// face (ticket #243: exe oracle, `docs/equip.md`), and the kitted
    /// row 8 bodies).
    /// Column placement of the five kit ids is fixture freedom (the
    /// stacker resolves ids, not slot semantics); only the face column
    /// is pinned.
    const LIVE_TESTER2_EQUIPMENT_ROW: [i32; 17] =
        [16, 30, 376, 383, 389, 0, 0, 0, 0, 0, 0, 35, 0, 0, 0, 0, 0];

    /// Stack one 17-slot equipment row on real 7.55 bytes (`None`
    /// without install-root) through the shared `load_worn_figure`
    /// path — the same stacker the waiting seats paint.
    fn stacked_figure_for_row(equipment: &[i32], male: bool) -> Option<AvatarFigure> {
        let archive = avatar_archive()?;
        let entries = o2jam_file::parse_opi(&archive).ok()?;
        let table_entry = entries
            .iter()
            .find(|entry| entry.name == "Itemdata_Korea.dat")?;
        let size = usize::try_from(table_entry.size).ok()?;
        let body = archive.get(table_entry.data_offset..table_entry.data_offset + size)?;
        let table = o2jam_file::parse_avatar_item_table(body).ok()?;
        let figure = load_worn_figure(&archive, &table, equipment, male);
        (!figure.is_empty()).then_some(figure)
    }

    /// The staged hero's worn figure: the live tester2 equipment row
    /// (below) at the live male parity (the 2001 reply reads
    /// `Gender: Male` for tester2).
    fn stacked_hero_figure() -> Option<AvatarFigure> {
        stacked_figure_for_row(&LIVE_TESTER2_EQUIPMENT_ROW, true)
    }

    /// Ink bounding box of the figure alone: pixels the painted
    /// frame inks over the bare frame, so panel captions and
    /// placeholders never count as figure ink.
    fn figure_only_bbox(painted: &[u32], bare: &[u32]) -> Option<(i32, i32, i32, i32)> {
        let mut ink: Option<(i32, i32, i32, i32)> = None;
        for (at, pixel) in painted.iter().enumerate() {
            if *pixel == 0 || bare.get(at) != Some(&0) {
                continue;
            }
            let x = i32::try_from(at % 800).unwrap_or(0);
            let y = i32::try_from(at / 800).unwrap_or(0);
            ink = Some(match ink {
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                None => (x, y, x, y),
            });
        }
        ink
    }

    /// The ink box must sit centered and contained in its slot: the
    /// full stack (head to feet) lands in the avatar rect instead of
    /// a head-sized blob at the border.
    fn assert_figure_fills_slot(painted: &[u32], bare: &[u32], slot: Rect) {
        let (x0, y0, x1, y1) =
            figure_only_bbox(painted, bare).expect("the stacked figure must ink the scene");
        assert!(
            x1 - x0 + 1 >= 40 && y1 - y0 + 1 >= 100,
            "the room paint must show the full stack (ink {w}x{h}), not a head-sized blob",
            w = x1 - x0 + 1,
            h = y1 - y0 + 1,
        );
        assert!(
            x0 >= slot.x
                && y0 >= slot.y
                && x1 < slot.x + slot.w as i32
                && y1 < slot.y + slot.h as i32,
            "figure ink ({x0},{y0})-({x1},{y1}) must stay inside its slot {slot:?}"
        );
        let center = |low: i32, high: i32| (low + high) / 2;
        assert!(
            (center(x0, x1) - center(slot.x, slot.x + slot.w as i32)).abs() <= 12
                && (center(y0, y1) - center(slot.y, slot.y + slot.h as i32)).abs() <= 12,
            "figure ink ({x0},{y0})-({x1},{y1}) must center in its slot {slot:?}"
        );
    }

    #[test]
    fn room_user_panel_centers_the_stacked_worn_figure_in_its_slot() {
        // Ticket #171: the room panel showed a head-sized blob at the
        // slot's border instead of the full worn figure. The staged
        // hero stacks head to feet through the waiting-seat stacker,
        // so the room paint must land that same stack centered and
        // contained in the `IDC_IMAGE_SROOM_AVATA` slot.
        let Some(figure) = stacked_hero_figure() else {
            println!("skip: missing install-root avatar.opa");
            return;
        };
        let placements = room_user_panel_placements();
        let slot = placements
            .iter()
            .find(|placed| placed.id == ROOM_AVATAR_ID)
            .expect("slot staged")
            .rect;
        let mut bare = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut bare,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            None,
            0,
        );
        let mut buf = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            Some(&figure),
            0,
        );
        assert_figure_fills_slot(&buf, &bare, slot);
    }

    #[test]
    fn waiting_seat_avatar_centers_the_stacked_worn_figure_in_its_slot() {
        // The proven reference for #171: the same hero stack through
        // the waiting seat-0 slot must land centered and contained
        // too — the room panel matches this figure for the same
        // equipment, both through the shared seat-slot helper.
        let Some(figure) = stacked_hero_figure() else {
            println!("skip: missing install-root avatar.opa");
            return;
        };
        let slot = Rect {
            x: 37,
            y: 55,
            w: 123,
            h: 150,
        };
        let placements = vec![
            ScenePlacement {
                role: PlacementRole::Single,
                id: WAITING_SEAT_PARENT_IDS[0].to_owned(),
                code: 0x1601_5022,
                sprite: None,
                rect: slot,
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: WAITING_AVATAR_SLOT_ID.to_owned(),
                code: 0x1670_002E,
                sprite: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 123,
                    h: 150,
                },
            },
        ];
        let mut bare = vec![0u32; 800 * 600];
        paint_waiting_seat_avatar(
            &mut Frame {
                buf: &mut bare,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            0,
        );
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_seat_avatar(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(&figure),
            0,
        );
        assert_figure_fills_slot(&buf, &bare, slot);
    }

    /// Ticket #375: the same player-visible waiting-seat painter must
    /// ink the bare arms/hands for user 1's face-only wire. The old
    /// body+face composition is the red comparison; the fallback kit
    /// adds item 31's one-image arms. Diff pixels name the painted
    /// hands region, not just a stacker run count.
    ///
    /// Live caller chain:
    ///   main.rs::run_play -> WindowHandler::paint
    ///   -> Shell::paint_backbuffer -> ScreenGame::paint_at
    ///   -> paint_waiting_scene -> paint_waiting_seat_avatar
    ///   -> paint_figure_in_seat_slot -> AvatarFigure::paint_visible_centered.
    /// Archives absent -> skipped (CI has no game data).
    #[test]
    fn waiting_seat_paints_bare_arms_for_the_face_only_row() {
        let mut face_only = [0i32; 17];
        face_only[11] = 35;
        let Some(full) = stacked_figure_for_row(&face_only, true) else {
            println!("skip: missing install-root avatar.opa");
            return;
        };
        let Some(old) = stacked_figure_for_row(&[30, 35], true) else {
            println!("skip: missing install-root avatar.opa");
            return;
        };
        let slot = Rect {
            x: 37,
            y: 55,
            w: 123,
            h: 150,
        };
        let placements = vec![
            ScenePlacement {
                role: PlacementRole::Single,
                id: WAITING_SEAT_PARENT_IDS[0].to_owned(),
                code: 0x1601_5022,
                sprite: None,
                rect: slot,
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: WAITING_AVATAR_SLOT_ID.to_owned(),
                code: 0x1670_002E,
                sprite: None,
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 123,
                    h: 150,
                },
            },
        ];
        let paint = |figure: &AvatarFigure| {
            let mut buf = vec![0u32; 800 * 600];
            paint_waiting_seat_avatar(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &placements,
                Some(figure),
                0,
            );
            buf
        };
        let with_arms = paint(&full);
        let without_arms = paint(&old);
        let mut arm_pixels = 0;
        for y in slot.y..slot.y + slot.h as i32 {
            for x in slot.x..slot.x + slot.w as i32 {
                let at = (y as usize) * 800 + (x as usize);
                if with_arms[at] != 0 && with_arms[at] != without_arms[at] {
                    arm_pixels += 1;
                }
            }
        }
        // The pinned arm sprite has 298 opaque pixels; half the count
        // is far above the old composition's zero painted arm pixels.
        const MIN_VISIBLE_ARM_PIXELS: usize = 149;
        assert!(
            arm_pixels >= MIN_VISIBLE_ARM_PIXELS,
            "the face-only wire paints the bare arm/hand pixels ({arm_pixels})"
        );
    }

    #[test]
    fn room_user_panel_paints_default_body_for_the_face_only_row() {
        // Ticket #243 supersedes #171 round 2: the live face-only wire
        // carried item 35 at `Equip12`, and the exe still paints the
        // naked default body (item 30) beneath it — #336 EV-336-A..D,
        // `docs/equip.md`. This guard pins the full-height ink in the
        // room user panel, not the old head-only reading.
        let mut face_only = [0i32; 17];
        face_only[11] = 35;
        let Some(figure) = stacked_figure_for_row(&face_only, true) else {
            println!("skip: missing install-root avatar.opa");
            return;
        };
        let placements = room_user_panel_placements();
        let slot = placements
            .iter()
            .find(|placed| placed.id == ROOM_AVATAR_ID)
            .expect("slot staged")
            .rect;
        let mut bare = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut bare,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            None,
            0,
        );
        let mut buf = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            Some(&figure),
            0,
        );
        let (x0, y0, x1, y1) =
            figure_only_bbox(&buf, &bare).expect("the face stack must ink the scene");
        assert!(
            y1 - y0 + 1 >= 75,
            "the face-only row must paint the default body ({h} tall), not head-only ink",
            h = y1 - y0 + 1,
        );
        assert!(
            x0 >= slot.x
                && y0 >= slot.y
                && x1 < slot.x + slot.w as i32
                && y1 < slot.y + slot.h as i32,
            "default figure ink ({x0},{y0})-({x1},{y1}) must stay inside its slot {slot:?}"
        );
    }

    #[test]
    fn waiting_seat_avatar_paints_the_local_figure_into_its_slot() {
        // `IDC_IMAGE_SWAIT_AVATAR 0x1670002E` carries a seat-relative
        // rect `(0,0,123,150)`; the painter adds seat 0's parent rect
        // (`IDC_BUTTON_SWAIT_EXTEND1 (37,55)`) like the clone write-back.
        let seat = |id: &str, code: u32, (x, y, w, h): (i32, i32, u32, u32)| ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        };
        let placements = vec![
            seat(WAITING_SEAT_PARENT_IDS[0], 0x1601_5022, (37, 55, 123, 150)),
            seat(WAITING_AVATAR_SLOT_ID, 0x1670_002E, (0, 0, 123, 150)),
        ];
        let figure = icon_figure(&[0x7C00]);
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_seat_avatar(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(&figure),
            0,
        );
        assert!(
            ink_in_rect(
                &buf,
                800,
                Rect {
                    x: 37,
                    y: 55,
                    w: 123,
                    h: 150
                }
            ) > 0,
            "figure ink must land inside seat 0's slot"
        );
        assert_eq!(
            ink_in_rect(
                &buf,
                800,
                Rect {
                    x: 0,
                    y: 0,
                    w: 20,
                    h: 20
                }
            ),
            0
        );
        // Without a staged figure the slot stays empty.
        let mut empty = vec![0u32; 800 * 600];
        paint_waiting_seat_avatar(
            &mut Frame {
                buf: &mut empty,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            0,
        );
        assert!(empty.iter().all(|pixel| *pixel == 0));
    }

    /// Eight seat parents plus the seat-relative avatar slot, in one
    /// synthetic row so per-seat ink never overlaps its neighbour.
    fn waiting_avatar_placements() -> Vec<ScenePlacement> {
        let mut placements = Vec::with_capacity(WAITING_SEATS + 1);
        for (index, id) in WAITING_SEAT_PARENT_IDS.iter().enumerate() {
            placements.push(ScenePlacement {
                role: PlacementRole::Single,
                id: (*id).to_owned(),
                code: 0x1601_5022 + index as u32,
                sprite: None,
                rect: Rect {
                    x: 10 + index as i32 * 130,
                    y: 50,
                    w: 123,
                    h: 150,
                },
            });
        }
        placements.push(ScenePlacement {
            role: PlacementRole::Single,
            id: WAITING_AVATAR_SLOT_ID.to_owned(),
            code: 0x1670_002E,
            sprite: None,
            rect: Rect {
                x: 0,
                y: 0,
                w: 123,
                h: 150,
            },
        });
        placements
    }

    /// Seat slot box of one synthetic parent (mirrors the painter's
    /// parent-origin plus slot-rect rule).
    fn waiting_slot_box(seat: usize) -> Rect {
        Rect {
            x: 10 + seat as i32 * 130,
            y: 50,
            w: 123,
            h: 150,
        }
    }

    #[test]
    fn waiting_seat_avatars_paint_each_occupied_seat_slot() {
        // `waiting_room` has one occupied wire slot (the host at
        // slot 0); slots 1.. stay empty/locked. The occupied slot
        // takes its figure through the clone rect — never the raw
        // `(0,0,123,150)` rect.
        let placements = waiting_avatar_placements();
        let room = waiting_room();
        let figures: Vec<Option<AvatarFigure>> = (0..WAITING_SEATS)
            .map(|index| (index < 2).then(|| icon_figure(&[0x7C00])))
            .collect();
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_seat_avatars(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &room,
            &figures,
            0,
        );
        assert!(
            ink_in_rect(&buf, 800, waiting_slot_box(0)) > 0,
            "the occupied wire slot takes its figure"
        );
        assert_eq!(
            ink_in_rect(&buf, 800, waiting_slot_box(1)),
            0,
            "the vacant slot stays empty"
        );
        assert_eq!(
            ink_in_rect(
                &buf,
                800,
                Rect {
                    x: 0,
                    y: 0,
                    w: 9,
                    h: 49
                }
            ),
            0,
            "no template child paints at the raw rect"
        );
    }

    #[test]
    fn waiting_seat_avatars_skip_empty_and_locked_seats() {
        // Figures staged for the empty seat 2 and the locked seat 3
        // never reach the frame: vacant seats keep their bare plate.
        let placements = waiting_avatar_placements();
        let room = waiting_room();
        let figures: Vec<Option<AvatarFigure>> = (0..WAITING_SEATS)
            .map(|index| (index == 2 || index == 3).then(|| icon_figure(&[0x7C00])))
            .collect();
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_seat_avatars(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            &room,
            &figures,
            0,
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn empty_covers_leave_the_staged_panels_untouched() {
        // Cover rect 61 `(170,53,311,228)` is wire slot 1: empty in
        // `waiting_room`, so no scrim, no dot — the staged panel shows.
        let placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_COVERAVATAR2",
            None,
            (170, 53, 311, 228),
        )];
        let calm = background_of(Screen::Waiting);
        let mut buf = vec![calm; 800 * 600];
        paint_waiting_covers(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &placements,
            None,
        );
        assert_eq!(buf[53 * 800 + 170], calm);
        assert!(!buf.contains(&scrim_color()));
    }

    #[test]
    fn staged_worn_figures_hide_the_baked_cover_sketch() {
        // Live caller chain: recomp2jam e2e -> E2eWalker::walk ->
        // ScreenGame::paint_at -> ScreenGame::paint_waiting ->
        // paint_waiting_scene -> backdrop cover filter (#336).
        // The 7.55 shot has no sketch on an empty seat; once item
        // figures are staged the placeholder plate must stand down.
        let placements = vec![ScenePlacement {
            role: PlacementRole::Single,
            id: "IDC_IMAGE_SWAIT_COVERAVATAR2".to_owned(),
            code: 0x1650_003E,
            sprite: Some("Wait_Cover_Avatar.ojs".to_owned()),
            rect: Rect {
                x: 170,
                y: 53,
                w: 141,
                h: 175,
            },
        }];
        let art = SceneArt {
            placements,
            figures: HashMap::from([("Wait_Cover_Avatar.ojs".to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
            false,
            true,
            MemberRole::Spectator,
        );
        assert_eq!(
            buf[53 * 800 + 170],
            0,
            "staged figures leave no baked cover sketch"
        );
    }

    #[test]
    fn team_highlight_marks_our_team_button() {
        // Team rects 3/4 `(652,65,676,89)`/`(682,65,706,89)` with
        // their staged `Wait_Team?.ojs` bodies: our team (1 = B)
        // takes the amber bar, A stays art-only.
        let placements = vec![
            waiting_probe(
                "IDC_BUTTON_SWAIT_TEAMA",
                Some("Wait_TeamA.ojs"),
                (652, 65, 676, 89),
            ),
            waiting_probe(
                "IDC_BUTTON_SWAIT_TEAMB",
                Some("Wait_TeamB.ojs"),
                (682, 65, 706, 89),
            ),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_team_highlight(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
            true,
            true,
        );
        assert_eq!(buf[65 * 800 + 682], pack_xrgb(0xFF, 0xD0, 0x20));
        assert_eq!(buf[65 * 800 + 652], 0);
    }

    #[test]
    fn wins_box_shows_the_champion_streak() {
        // `WINNERSCORE` rect 108 `(190,95,240,115)`: the wire
        // streak only; per-member tallies stay server-side.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_WINNERSCROE",
            None,
            (190, 95, 240, 115),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_wins(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[93 * 800 + 186], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn wins_box_stays_empty_without_a_champion() {
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_WINNERSCROE",
            None,
            (190, 95, 240, 115),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_wins(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &placements,
        );
        assert!(!buf.contains(&scrim_color()));
    }

    #[test]
    fn speed_suffix_inks_the_bound_chip_without_a_shell_plate() {
        // `SPEED_NUM` rect 101 `(681,378,735,413)`: speed code 5 reads
        // `X3` through the shared label chain. The 7.55 Wine capture
        // (`ab-wait.png`) shows `X1` bare on the staged blue chip, so
        // the painter must ink text at the reference bearing and never
        // fill the chip corner with shell scrim.
        let placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_SPEED_NUM",
            None,
            (681, 378, 735, 413),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_speed(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            champion_room().speed,
            &placements,
        );
        assert_eq!(buf[376 * 800 + 677], 0, "the chip has no shell scrim");
        let mut expected = vec![0u32; 800 * 600];
        draw_text(
            &mut Frame {
                buf: &mut expected,
                w: 800,
                h: 600,
            },
            681 + 6,
            378 + 7,
            "X3",
            art_title_ink(),
        );
        assert_eq!(buf, expected, "speed ink must use the bare GDI bearing");
    }

    #[test]
    fn waiting_speed_and_random_cards_paint_one_record_each() {
        // Exe oracle: SPEED_NUM shows one `X1` record for code 1, and
        // RANDMAP keeps its `?` rest card. Stacking every record used
        // to end on a green `?` and a song cover (#883/#884).
        let mut speed = solid_figure(54, 35, [0x00, 0x7C]);
        speed.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x1F, 0x00].repeat(54 * 35), 54, 35).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let mut random = solid_figure(89, 65, [0x00, 0x7C]);
        random.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x1F, 0x00].repeat(89 * 65), 89, 65).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let art = SceneArt {
            placements: vec![
                waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR1", None, (28, 53, 169, 228)),
                waiting_probe(
                    WAITING_SPEED_NUM_ID,
                    Some("SpeedProbe.ojs"),
                    (681, 378, 735, 413),
                ),
                waiting_probe(
                    "IDC_IMAGE_SWAIT_RANDMAP",
                    Some("RandomProbe.ojs"),
                    (619, 468, 708, 533),
                ),
            ],
            figures: HashMap::from([
                ("SpeedProbe.ojs".to_owned(), speed),
                ("RandomProbe.ojs".to_owned(), random),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        assert_eq!(
            buf[390 * 800 + 700],
            pack_xrgb(0, 0, 255),
            "speed code 1 paints its own record"
        );
        assert_eq!(
            buf[480 * 800 + 650],
            pack_xrgb(255, 0, 0),
            "random card keeps the rest record"
        );
    }

    #[test]
    fn waiting_speed_arrows_keep_the_bright_rest_record() {
        // Quiesced pair `(720,400)`: exe shows bright blue arrows,
        // rust stacked the two-record sprites and ended on the dark
        // face (#883). Both arrow ids must use record 0.
        let mut arrow = solid_figure(33, 36, [0x1F, 0x00]);
        arrow.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0x10, 0x42].repeat(33 * 36), 33, 36).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        let art = SceneArt {
            placements: vec![
                waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR1", None, (28, 53, 169, 228)),
                waiting_probe(
                    "IDC_BUTTON_SWAIT_SPEED_LEFT",
                    Some("ArrowProbe.ojs"),
                    (647, 378, 680, 414),
                ),
                waiting_probe(
                    "IDC_BUTTON_SWAIT_SPEED_RIGHT",
                    Some("ArrowProbe.ojs"),
                    (736, 378, 769, 414),
                ),
            ],
            figures: HashMap::from([("ArrowProbe.ojs".to_owned(), arrow)]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        assert_eq!(
            buf[390 * 800 + 655],
            pack_xrgb(0, 0, 255),
            "left arrow keeps its bright rest record"
        );
        assert_eq!(
            buf[390 * 800 + 745],
            pack_xrgb(0, 0, 255),
            "right arrow keeps its bright rest record"
        );
    }

    #[test]
    fn waiting_map_cell_keeps_the_baked_random_label() {
        // Exe region `(640,480)`: an arena-0 room leaves `Random` from
        // the sheet visible. Our map-cycle caption printed a Korean
        // stage name plus a black scrim on top of it (#884).
        let placements = vec![waiting_probe(
            WAITING_MAPNAME_ID,
            None,
            (620, 448, 707, 462),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_mapname(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &placements,
            false,
        );
        assert!(
            buf.iter().all(|pixel| *pixel == 0),
            "no map caption for the arena-0 room in the oracle pair"
        );
    }

    #[test]
    fn skill_slots_show_staged_ring_ids_only() {
        // `1SKILL` rect 87 `(701,208,731,238)`: the staged ring id;
        // `2SKILL` (no second ring) stays untouched X art.
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_1SKILL", None, (701, 208, 731, 238)),
            waiting_probe("IDC_IMAGE_SWAIT_2SKILL", None, (701, 246, 731, 276)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_skills(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            None,
            &placements,
        );
        assert_eq!(buf[206 * 800 + 697], scrim_color());
        assert_eq!(buf[246 * 800 + 701], 0);
    }

    #[test]
    fn song_title_inks_the_bound_plate_without_a_shell_plate() {
        // `MUSICNAME` rect 22 `(573,15,773,38)`: bare song id while
        // the staged catalog names no such song. The 7.55 Wine capture
        // (`ab-wait.png`) inks the title at `(579,22)` — six px right
        // and seven px down from the rect origin — on the staged green
        // plate, with no black rectangle behind it.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_MUSICNAME",
            None,
            (573, 15, 773, 38),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_music(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(WaitingSong {
                music_id: 757,
                title: None,
            }),
            false,
        );
        assert_eq!(buf[13 * 800 + 569], 0, "the plate has no shell scrim");
        let mut expected = vec![0u32; 800 * 600];
        draw_text(
            &mut Frame {
                buf: &mut expected,
                w: 800,
                h: 600,
            },
            573 + 6,
            15 + 7,
            "M757",
            art_title_ink(),
        );
        assert_eq!(buf, expected, "title ink must use the bare GDI bearing");
    }

    #[test]
    fn waiting_music_caption_prefers_the_catalog_title() {
        // Room-fill string flow: the staged catalog names the
        // room's song, so `MUSICNAME` carries the title.
        assert_eq!(waiting_music_caption(757, Some("First")), "First");
    }

    #[test]
    fn waiting_music_caption_falls_back_to_song_id() {
        // No staged title (or a blank one): the bare id, never an
        // invented name.
        assert_eq!(waiting_music_caption(757, None), "M757");
        assert_eq!(waiting_music_caption(757, Some("")), "M757");
        assert_eq!(waiting_music_caption(0, None), "M0");
    }

    #[test]
    fn waiting_musicname_paints_the_catalog_title() {
        // Same rect as the id fallback, but the staged title ink:
        // the two captions share the anchor, the source differs.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_MUSICNAME",
            None,
            (573, 15, 773, 38),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_music(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            Some(WaitingSong {
                music_id: 757,
                title: Some("First"),
            }),
            false,
        );
        assert!(!buf.contains(&scrim_color()));
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn musicname_paints_nothing_without_a_song_source() {
        // No joined room and no staged pick: the caption has no song
        // to name and leaves the pinned rect untouched (the staged
        // backdrop paints it).
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_MUSICNAME",
            None,
            (573, 15, 773, 38),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_music(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &placements,
            None,
            false,
        );
        assert!(!has_ink_near(&buf, art_title_ink()));
        assert_eq!(buf[13 * 800 + 569], 0);
    }

    #[test]
    fn waiting_seat_caption_prints_level_and_name_only() {
        // Quiesced 7.55 pair: `Lv.1:t207probe`, not the legacy row text
        // with team letter, champion role or ready flag (#883).
        let host = StagedSeat {
            name: "host".to_owned(),
            level: Some(9),
            team: 0,
            ready: true,
            present: true,
            locked: false,
            master: true,
            playing: false,
            has_song: true,
            download_badge_armed: false,
            no_gem: false,
            pc_room: false,
        };
        assert_eq!(waiting_seat_caption(&host), "Lv.9:host");
        let local = StagedSeat {
            name: "YOU".to_owned(),
            level: None,
            team: 1,
            ready: false,
            present: true,
            locked: false,
            master: false,
            playing: false,
            has_song: false,
            download_badge_armed: false,
            no_gem: false,
            pc_room: false,
        };
        assert_eq!(waiting_seat_caption(&local), "Lv.-:YOU");
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn waiting_chat_box_paints_no_history_without_a_source() {
        // Chat controls (`EDIT_SWAIT_CHAT` rect 19 `(20,563,460,583)`,
        // `LIST_SWAIT_CHAT` rect 42 `(24,455,511,553)`) name the history
        // view, but no text source reaches the painter: waiting-room chat
        // rides 3011/3012/3013 (opcode numbers only), `o2jam-net` decodes
        // none, and `JoinedRoom` carries no chat field — so staged chat
        // rects stay untouched instead of invented.
        let placements = vec![
            waiting_probe("IDC_LIST_SWAIT_CHAT", None, (24, 455, 511, 553)),
            waiting_probe("IDC_EDIT_SWAIT_CHAT", None, (20, 563, 460, 583)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements_stage(&placements),
            &WaitingCaptions::default(),
            true,
            false,
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn waiting_chat_row_text_right_aligns_the_sender_in_a_12_wide_field() {
        // The enqueue printf formats `[%12s] %s` (format VA `0x64d8d4`,
        // call file `0x15f076`, `docs/channel-fill.md`): bracket, sender
        // right-aligned to width 12, bracket, space, text.
        let row = WaitingChatMessage::user("host", "hello");
        assert_eq!(waiting_chat_row_text(&row), "[        host] hello");
    }

    #[test]
    fn waiting_chat_rows_stack_at_the_exe_measured_pitch() {
        // Exe-measured 2026-11-09 (rig session, six live 3011 rows in
        // ~/scratch/geom-open-close/t1-chat-box.png): glyph tops at
        // 456+17k; only the newest 5 rows render in the 98 px list.
        assert_eq!(waiting_chat_row_y(455, 0), 455);
        assert_eq!(waiting_chat_row_y(455, 1), 472);
        assert_eq!(waiting_chat_row_y(455, WAITING_CHAT_VISIBLE_ROWS - 1), 523);
        assert_eq!(WAITING_CHAT_VISIBLE_ROWS, 5);
    }

    #[test]
    fn waiting_chat_row_text_brackets_admin_rows_like_user_rows() {
        // The 3013 stub enqueues the same `[%12s] %s` line; only the
        // appended ink differs (`0x71e6` vs `0xdedede`). The `[ADMIN]`
        // stand-in prefix is gone.
        let row = WaitingChatMessage::admin("op", "quiet");
        assert_eq!(waiting_chat_row_text(&row), "[          op] quiet");
    }

    #[test]
    fn waiting_chat_without_live_rows_paints_only_the_caret() {
        // No packets → no history (the no-invention pin); the input
        // line still shows its focus caret (exe-measured 2026-11-09,
        // ~/scratch/geom-open-close/t1-waiting-empty.png: empty
        // history, caret ink bbox (23,570) 2x9 inside rect 19) — inset
        // (3,7) of the edit rect.
        let placements = vec![
            waiting_probe("IDC_LIST_SWAIT_CHAT", None, (24, 455, 511, 553)),
            waiting_probe("IDC_EDIT_SWAIT_CHAT", None, (20, 563, 460, 583)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            "",
            &placements,
        );
        let lit: Vec<usize> = buf
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel != 0)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(lit.len(), 18, "only the 2x9 caret may light");
        for index in lit {
            assert_eq!(buf[index], art_title_ink());
            let (x, y) = (index % 800, index / 800);
            assert!((23..=24).contains(&x), "caret x {x}");
            assert!((570..=578).contains(&y), "caret y {y}");
        }
    }

    #[test]
    fn waiting_chat_paints_user_rows_in_the_exe_gray_at_the_list_rect() {
        // The 3012 append ink is COLORREF `0xdedede` (call file
        // `0xb9f8c`, `docs/channel-fill.md`) — light gray, drawn
        // straight on the box art (no per-row scrim band).
        let placements = vec![waiting_probe(
            "IDC_LIST_SWAIT_CHAT",
            None,
            (24, 455, 511, 553),
        )];
        let rows = vec![WaitingChatMessage::user("host", "hello")];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "",
            &placements,
        );
        assert_eq!(buf[453 * 800 + 20], 0, "no scrim band over the row");
        // The probe rides the history band: the white caret below the
        // list is near gray at the 64 ink tolerance and would mask the
        // row ink.
        let row_band = &buf[440 * 800..555 * 800];
        assert!(
            row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xDE, 0xDE, 0xDE))),
            "user gray ink on the row"
        );
        assert!(
            !row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xE6, 0x71, 0x00))),
            "no admin ink on a user row"
        );
    }

    #[test]
    fn waiting_chat_paints_admin_rows_in_the_exe_amber() {
        // The 3013 append ink is COLORREF `0x71e6` (call file
        // `0xb9fbf`) — `0x00BBGGRR` through GDI `SetTextColor`, a warm
        // amber (the reference rows all read warm; `docs/parity/verify-photo.md`
        // carries the photo residual).
        let placements = vec![waiting_probe(
            "IDC_LIST_SWAIT_CHAT",
            None,
            (24, 455, 511, 553),
        )];
        let rows = vec![WaitingChatMessage::admin("op", "quiet")];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "",
            &placements,
        );
        assert_eq!(buf[453 * 800 + 20], 0, "no scrim band over the row");
        assert!(has_ink_near(&buf, pack_xrgb(0xE6, 0x71, 0x00)));
        // User gray belongs to 3012 rows only.
        let row_band = &buf[440 * 800..555 * 800];
        assert!(
            !row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xDE, 0xDE, 0xDE))),
            "no user ink on an admin row"
        );
    }

    #[test]
    fn waiting_chat_shows_only_the_newest_rows() {
        let placements = vec![waiting_probe(
            "IDC_LIST_SWAIT_CHAT",
            None,
            (24, 455, 511, 553),
        )];
        let mut rows = Vec::new();
        for index in 0..WAITING_CHAT_VISIBLE_ROWS + 2 {
            rows.push(WaitingChatMessage::user("host", &format!("line{index}")));
        }
        let visible = waiting_chat_visible_messages(&rows);
        assert_eq!(visible.len(), WAITING_CHAT_VISIBLE_ROWS);
        assert_eq!(visible[0].text, "line2");
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "",
            &placements,
        );
        assert!(buf.contains(&pack_xrgb(0xDE, 0xDE, 0xDE)));
        assert!(!buf.contains(&scrim_color()), "no scrim bands");
    }

    #[test]
    fn waiting_chat_paints_the_draft_at_the_edit_rect_with_the_caret_after_it() {
        let placements = vec![waiting_probe(
            "IDC_EDIT_SWAIT_CHAT",
            None,
            (20, 563, 460, 583),
        )];
        let input = "go";
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            input,
            &placements,
        );
        assert_eq!(buf[561 * 800 + 16], 0, "no scrim band over the draft");
        assert!(has_ink_near(&buf, art_title_ink()));
        // The caret rides the insertion point: exe-measured inset
        // (3,7) plus the typed width (empty-input case, rig session
        // 2026-11-09).
        let caret_x = 20 + 3 + text_width(input) as i32;
        assert_eq!(buf[570 * 800 + caret_x as usize], art_title_ink());
        assert_eq!(buf[578 * 800 + (caret_x + 1) as usize], art_title_ink());
    }

    #[test]
    fn waiting_chat_paints_the_caret_on_an_empty_input() {
        let placements = vec![waiting_probe(
            "IDC_EDIT_SWAIT_CHAT",
            None,
            (20, 563, 460, 583),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[],
            "",
            &placements,
        );
        // Exe-measured caret: inset (3,7) of rect 19, 2x9 bar —
        // ink at (23,570) and (24,578) (rig session 2026-11-09).
        assert_eq!(buf[570 * 800 + 23], art_title_ink());
        assert_eq!(buf[578 * 800 + 24], art_title_ink());
    }

    #[test]
    fn waiting_chat_native_fallback_rects_match_the_state_waiting_bnd() {
        // File-traced (geometry-audit.md Waiting rows): the no-art
        // fallbacks must equal `State_Waiting.bnd` rect 42
        // `(24,455)-(511,553)` and rect 19 `(20,563)-(460,583)`,
        // paired to `IDC_LIST_SWAIT_CHAT 0x1680002B` (ordinal 42) and
        // `IDC_EDIT_SWAIT_CHAT 0x16300014` (ordinal 19).
        assert_eq!(
            WAITING_CHAT_LIST_RECT,
            Rect {
                x: 24,
                y: 455,
                w: 487,
                h: 98
            }
        );
        assert_eq!(
            WAITING_CHAT_EDIT_RECT,
            Rect {
                x: 20,
                y: 563,
                w: 440,
                h: 20
            }
        );
    }

    #[test]
    fn waiting_chat_falls_back_to_native_rects_without_staged_art() {
        let rows = vec![WaitingChatMessage::user("host", "hello")];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &rows,
            "go",
            &[],
        );
        assert_eq!(buf[453 * 800 + 20], 0, "no scrim over the row");
        assert_eq!(buf[561 * 800 + 16], 0, "no scrim over the draft");
        let row_band = &buf[440 * 800..555 * 800];
        assert!(
            row_band
                .iter()
                .any(|pixel| near_xrgb(*pixel, pack_xrgb(0xDE, 0xDE, 0xDE))),
            "user gray ink on the row"
        );
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn channel_chat_row_text_brackets_like_the_waiting_box() {
        let row = ChannelChatMessage::user("host", "hello");
        assert_eq!(channel_chat_row_text(&row), "[        host] hello");
        let admin = ChannelChatMessage::admin("op", "quiet");
        assert_eq!(channel_chat_row_text(&admin), "[          op] quiet");
    }

    #[test]
    fn channel_whisper_row_names_both_ends() {
        let row = ChannelChatMessage::whisper("host", "me", "psst");
        assert_eq!(
            channel_chat_row_text(&row),
            "[        host ->           me] psst"
        );
    }

    #[test]
    fn channel_chat_shows_only_the_newest_rows() {
        let mut rows = Vec::new();
        for index in 0..CHANNEL_CHAT_VISIBLE_ROWS + 2 {
            rows.push(ChannelChatMessage::user("host", &format!("line{index}")));
        }
        let visible = channel_chat_visible_messages(&rows);
        assert_eq!(visible.len(), CHANNEL_CHAT_VISIBLE_ROWS);
        assert_eq!(visible[0].text, "line2");
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn channel_chat_without_rows_paints_nothing() {
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[],
            2,
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn channel_chat_paints_user_rows_in_the_exe_gray() {
        let rows = vec![ChannelChatMessage::user("host", "hello")];
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &rows,
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xDE, 0xDE, 0xDE)));
    }

    #[test]
    fn channel_chat_paints_admin_rows_in_the_exe_amber() {
        let rows = vec![ChannelChatMessage::admin("op", "quiet")];
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &rows,
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xE6, 0x71, 0x00)));
    }

    #[test]
    fn announcement_row_paints_bare_text_in_admin_amber() {
        let row = ChannelChatMessage::announcement("hello");
        assert!(row.admin);
        assert_eq!(channel_chat_row_text(&row), "hello");
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_chat(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[row],
            0,
        );
        assert!(has_ink_near(&buf, pack_xrgb(0xE6, 0x71, 0x00)));
    }

    #[test]
    fn rank_row_paints_the_notice_board_sentence_bare() {
        let row = ChannelChatMessage::rank("host", "M757", 1);
        assert!(row.admin);
        assert!(row.sender.is_empty());
        assert_eq!(row.text, rank_notice_board_text("host", "M757", 1));
        assert_eq!(
            channel_chat_row_text(&row),
            "host 음악전사님께서 [M757] 곡에 새로운 랭킹 [1]위가 되셨습니다~"
        );
    }

    #[test]
    fn rank_notice_board_names_nickname_song_and_rank_in_exe_order() {
        assert_eq!(
            rank_notice_board_text("host", "M757", 1),
            "host 음악전사님께서 [M757] 곡에 새로운 랭킹 [1]위가 되셨습니다~"
        );
    }

    #[test]
    fn rank_notice_board_brackets_song_and_rank_with_exe_separators() {
        let text = rank_notice_board_text("host", "TITLE", 2);
        assert!(text.starts_with("host 음악전사님께서 [TITLE]"));
        assert!(text.contains("곡에 새로운 랭킹 [2]위가 되셨습니다~"));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn waiting_random_cards_carry_no_card_data() {
        // The `?` cards name no per-card wire data: the big card sits
        // on `RANDMAP` (rect 78 `(619,468,708,533)`) and the small cards
        // on the spriteless `EFFECT_1/2` rects (68/69). A plain song
        // must still leave both mini rects empty — the random-song
        // mount is guarded separately by the next test.
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_RANDMAP", None, (619, 468, 708, 533)),
            waiting_probe("IDC_IMAGE_SWAIT_EFFECT_1", None, (719, 450, 779, 498)),
            waiting_probe("IDC_IMAGE_SWAIT_EFFECT_2", None, (719, 507, 779, 555)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements_stage(&placements),
            &WaitingCaptions::default(),
            true,
            false,
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn random_song_inks_both_mini_cards_from_the_staged_card_body() {
        // Statics: a random song mounts `Wait_Effect_Rand1/2.ojs` onto
        // EFFECT_1/2 (`FUN_004ad350`, file `0xad350`); the shipped body
        // is a 60x48 `?` card. Our staged Waiting block has no sprite
        // for those two controls, so the painter inks the pinned rects
        // from the staged Random card body instead (#883/#884). Red:
        // before the fix both rects stay empty.
        let art = SceneArt {
            placements: vec![
                waiting_probe(
                    "IDC_IMAGE_SWAIT_RANDMAP",
                    Some("RandomProbe.ojs"),
                    (619, 468, 708, 533),
                ),
                waiting_probe("IDC_IMAGE_SWAIT_EFFECT_1", None, (719, 450, 779, 498)),
                waiting_probe("IDC_IMAGE_SWAIT_EFFECT_2", None, (719, 507, 779, 555)),
            ],
            figures: HashMap::from([(
                "RandomProbe.ojs".to_owned(),
                solid_figure(89, 65, [0x00, 0x7C]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &art,
            &WaitingCaptions {
                random: true,
                local_random_selection: true,
                ..WaitingCaptions::default()
            },
            true,
            false,
        );
        assert_eq!(
            buf[460 * 800 + 740],
            pack_xrgb(255, 0, 0),
            "the first mini card shows the staged ? body"
        );
        assert_eq!(
            buf[520 * 800 + 740],
            pack_xrgb(255, 0, 0),
            "the second mini card shows the staged ? body"
        );
    }

    #[test]
    fn messenger_lamps_hide_at_rest_and_ink_only_on_an_alert() {
        // Statics: `FUN_004c0690(0)` stops the two lamp animations
        // (`FUN_0042f4f0`, file `0x2f4f0`) and never paints a rest
        // record; the quiesced pair shows only the purple MESSENGER
        // body, no white popup. Red: the old toggle-group painted a
        // chat bubble for both lamps while unlit.
        let mut first = waiting_probe(
            "IDC_IMAGE_SWAIT_MESSENGER_ALRAM",
            Some("LampProbe.ojs"),
            (668, 569, 708, 593),
        );
        first.code = 0x1650_006B;
        let mut second = waiting_probe(
            "IDC_IMAGE_SWAIT_MESSENGER_ALRAM1",
            Some("LampProbe.ojs"),
            (657, 545, 691, 574),
        );
        second.code = 0x1650_006C;
        let art = SceneArt {
            placements: vec![first, second],
            figures: HashMap::from([(
                "LampProbe.ojs".to_owned(),
                solid_figure(40, 24, [0x00, 0x7C]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        paint_waiting_staged(
            &mut frame,
            &champion_room(),
            &art,
            &WaitingCaptions {
                messenger_lamp_lit: false,
                ..WaitingCaptions::default()
            },
            true,
            false,
        );
        assert!(
            frame.buf.iter().all(|pixel| *pixel == 0),
            "both lamps stay hidden without an alert"
        );
        paint_waiting_staged(
            &mut frame,
            &champion_room(),
            &art,
            &WaitingCaptions {
                messenger_lamp_lit: true,
                ..WaitingCaptions::default()
            },
            true,
            false,
        );
        assert_eq!(
            frame.buf[575 * 800 + 675],
            pack_xrgb(255, 0, 0),
            "an alert inks the first lamp"
        );
        assert_eq!(
            frame.buf[550 * 800 + 665],
            pack_xrgb(255, 0, 0),
            "an alert inks the popup lamp"
        );
    }

    #[test]
    fn room_title_anchors_to_the_roomname_rect() {
        // `ROOMNAME` rect 21 `(274,15,466,38)`: the joined-room title.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_ROOMNAME",
            None,
            (274, 15, 466, 38),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_title(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[13 * 800 + 270], scrim_color());
    }

    #[test]
    fn the_waiting_notice_takes_the_room_title_line() {
        // docs/music-availability.md §4: the START gate's refusal notice
        // stands over the top-bar line while it is set (the exe's notice
        // box above the START control); with no notice the room title
        // paints there. Same `ROOMNAME` rect, so the ink replaces it.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_ROOMNAME",
            None,
            (274, 15, 466, 38),
        )];
        let art = placements_stage(&placements);
        let mut titled = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut titled,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &art,
            &WaitingCaptions::default(),
            true,
            false,
        );
        let mut noticed = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut noticed,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &art,
            &WaitingCaptions {
                notice: Some("Some players are not ready."),
                ..Default::default()
            },
            true,
            false,
        );
        assert_eq!(titled[13 * 800 + 270], scrim_color());
        assert_eq!(noticed[13 * 800 + 270], scrim_color());
        assert_ne!(titled, noticed, "the notice inks its own glyphs");
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn unstaged_regions_paint_nothing() {
        // Missing placements paint nothing: the exe leaves untouched
        // icons alone — no invented coordinates anywhere.
        let empty: Vec<ScenePlacement> = Vec::new();
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements_stage(&empty),
            &WaitingCaptions::default(),
            true,
            false,
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn staged_scene_prefers_bnd_anchors_over_the_legacy_grid() {
        // Covers staged: the seat line anchors to the template
        // `AVATARINFO` cell at `(39,216)` and the legacy grid slot
        // `(100,150)` stays untouched — an empty stage paints no fill
        // (`docs/channel-fill.md`, #883).
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR1", None, (28, 53, 169, 228)),
            waiting_probe(WAITING_SEAT_PARENT_IDS[0], None, (37, 55, 160, 205)),
            waiting_probe("IDC_TEXT_SWAIT_AVATARINFO", None, (1, 157, 122, 172)),
        ];
        let art = SceneArt {
            placements,
            figures: HashMap::new(),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &waiting_room(),
            &art,
            &WaitingCaptions::default(),
            false,
            false,
            MemberRole::Spectator,
        );
        assert_eq!(buf[214 * 800 + 35], scrim_color());
        assert_eq!(buf[150 * 800 + 100], 0);
    }

    #[test]
    fn host_paints_the_select_button_while_peer_keeps_the_bpm_cover() {
        // Owner decision 2026-11-16 (ticket #278): the host sees the
        // staged `Wait_MusicSelect.ojs` button on the song rect and no
        // `BPM` panel, a peer keeps the `BPM` panel and never sees the
        // button. `Wait_AlbumMode.ojs` paints for neither role. The exe
        // display rule is OPEN (`docs/room-enums.md` "The waiting song
        // rect"), so this seam pins the shell rule only.
        let placements = vec![
            waiting_probe(
                "IDC_BUTTON_SWAIT_MUSELECT",
                Some("Wait_MusicSelect.ojs"),
                (540, 493, 594, 542),
            ),
            waiting_probe(
                "IDC_IMAGE_SWAIT_COVERMUSIC",
                Some("Wait_Cover_Music.ojs"),
                (538, 489, 595, 544),
            ),
            waiting_probe(
                "IDC_IMAGE_SWAIT_ALBUMMODE",
                Some("Wait_AlbumMode.ojs"),
                (533, 441, 601, 551),
            ),
        ];
        let art = SceneArt {
            placements,
            figures: HashMap::from([
                ("Wait_MusicSelect.ojs".to_owned(), red_figure()),
                (
                    "Wait_Cover_Music.ojs".to_owned(),
                    solid_figure(57, 97, [0xE0, 0x03]),
                ),
                (
                    "Wait_AlbumMode.ojs".to_owned(),
                    solid_figure(68, 110, [0xFF, 0x7F]),
                ),
            ]),
        };
        let paint = |member_id: u8| {
            let mut room = waiting_room();
            room.member_id = member_id;
            let mut buf = vec![0u32; 800 * 600];
            paint_waiting_scene(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &room,
                &art,
                &WaitingCaptions::default(),
                false,
                false,
                MemberRole::Spectator,
            );
            buf
        };
        let host = paint(0);
        assert_eq!(
            host[493 * 800 + 540],
            pack_xrgb(255, 0, 0),
            "host paints the select button at its rect origin"
        );
        assert_eq!(host[460 * 800 + 536], 0, "host paints no album panel");

        let peer = paint(5);
        let cover_pixel = peer[515 * 800 + 560];
        assert_ne!(cover_pixel, 0, "peer keeps the BPM cover");
        assert!(!host.contains(&cover_pixel), "host paints no BPM cover");
        assert!(
            !peer.contains(&pack_xrgb(255, 0, 0)),
            "peer never shows the button"
        );
        assert_eq!(peer[460 * 800 + 536], 0, "album panel stays off for peers");
    }

    #[test]
    fn waiting_song_rect_bodies_swap_with_the_role() {
        // Rule pin for the two pixel guards above (owner decision
        // 2026-11-16, ticket #278). A peer's opaque `BPM` cover hides a
        // stray button completely, so the frames cannot see a peer-armed
        // button; this pin names the arms themselves: host keeps the
        // song-select button, peer keeps the `BPM` cover, and the
        // album-mode body stays off for both roles.
        assert!(waiting_control_armed_for_role(
            "IDC_BUTTON_SWAIT_MUSELECT",
            true
        ));
        assert!(!waiting_control_armed_for_role(
            "IDC_BUTTON_SWAIT_MUSELECT",
            false
        ));
        assert!(!waiting_control_armed_for_role(
            "IDC_IMAGE_SWAIT_COVERMUSIC",
            true
        ));
        assert!(waiting_control_armed_for_role(
            "IDC_IMAGE_SWAIT_COVERMUSIC",
            false
        ));
        assert!(!waiting_control_armed_for_role(
            "IDC_IMAGE_SWAIT_ALBUMMODE",
            true
        ));
        assert!(!waiting_control_armed_for_role(
            "IDC_IMAGE_SWAIT_ALBUMMODE",
            false
        ));
        // #765: the rest of the pinned role set. Master arms map arrows
        // and all six skill buttons; guests show the three skill covers.
        for id in [
            "IDC_BUTTON_SWAIT_MAPLEFT",
            "IDC_BUTTON_SWAIT_MAPRIGHT",
            "IDC_BUTTON_SWAIT_1SKILLLEFT",
            "IDC_BUTTON_SWAIT_1SKILLRIGHT",
            "IDC_BUTTON_SWAIT_2SKILLLEFT",
            "IDC_BUTTON_SWAIT_2SKILLRIGHT",
            "IDC_BUTTON_SWAIT_3SKILLLEFT",
            "IDC_BUTTON_SWAIT_3SKILLRIGHT",
        ] {
            assert!(waiting_control_armed_for_role(id, true), "{id} master");
            assert!(!waiting_control_armed_for_role(id, false), "{id} guest");
        }
        for id in [
            "IDC_IMAGE_SWAIT_COVERSKILLLEFT",
            "IDC_IMAGE_SWAIT_COVERSKILLRIGHT",
            "IDC_IMAGE_SWAIT_COVERSKILLSEL",
        ] {
            assert!(!waiting_control_armed_for_role(id, true), "{id} master");
            assert!(waiting_control_armed_for_role(id, false), "{id} guest");
        }
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn map_arrow_cycle_skips_the_pinned_stages_and_leaves_random_wrap_open() {
        // Seam pin for the pure cycle: the live click path is guarded
        // in `screens::game::game_tests`, and this table leg needs no
        // staged art or game data. Exe `FUN_004bb570` (evidence
        // `ev_c9d69d84…`); skip words at rdata `0x649ac4`.
        use super::WaitingMapStep::{Next, Previous};
        assert_eq!(waiting_map_cycle(7, Next, 14), Some(8));
        assert_eq!(waiting_map_cycle(7, Previous, 14), Some(6));
        assert_eq!(waiting_map_cycle(139, Next, 14), Some(141));
        assert_eq!(waiting_map_cycle(141, Previous, 14), Some(139));
        assert_eq!(waiting_map_cycle(14, Next, 14), None, "random wrap OPEN");
        assert_eq!(waiting_map_cycle(1, Previous, 14), None, "random wrap OPEN");
        assert_eq!(
            waiting_map_cycle(i32::MIN + 5, Next, 14),
            Some(1),
            "random room word takes the pinned low branch"
        );
    }

    #[test]
    fn waiting_control_ids_match_their_staged_bodies() {
        // The seat covers stage one named body; the team grid stages
        // one body per team in order (`ControlList_Interface.txt`).
        assert_eq!(WAIT_COVER_SPRITE, "Wait_Cover_Avatar.ojs");
        assert_eq!(WAIT_TEAM_SPRITES[0], "Wait_TeamA.ojs");
        assert_eq!(WAIT_TEAM_SPRITES[7], "Wait_TeamH.ojs");
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn spriteless_channel_block_stages_no_scene_art() {
        // Shape-faithful `STATE_CHANNEL` skeleton (bare `BOUND`, the
        // real image/button/text/`SET` code classes, zero quoted
        // sprites — `docs/channel-fill.md`): no block entry names art,
        // so the scene stages nothing and the painter keeps its flat
        // background plus the live list.
        let list = o2jam_file::parse_control_list(
            &[
                "NUMBER_OF_STATE 0x01",
                "STATE_CHANNEL 0x04",
                "{",
                "BOUND",
                "IDC_STATE_CHANNEL 0X12000000",
                "IDC_BACK_SCHANNEL_1 0X12100001",
                "IDC_BUTTON_SCHANNEL_BACK 0X12200002",
                "IDC_BUTTON_SCHANNEL_EXBUTTON1 0X12011007",
                "SET 0X02 0X00011000",
                "{",
                "IDC_IMAGE_SCHANNEL_ANI 0X12500011",
                "IDC_IMAGE_SCHANNEL_NUMBER1 0X12500012",
                "}",
                "IDC_TEXT_SCHANNEL_TITLE1 0X1240001E",
                "}",
                "",
            ]
            .join("\n"),
        )
        .unwrap();
        assert!(main_scene_sprites(&list, Screen::Channel).is_empty());
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn empty_channel_art_paints_no_fill() {
        // Pinned no-fill rule (`docs/channel-fill.md`): no staged
        // figures means no staged ink and no invented fill — the bare
        // corner stays untouched while the live title still paints.
        let art = SceneArt::default();
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &[],
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(buf[0], 0);
        assert!(!buf.contains(&background_of(Screen::Channel)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn staged_empty_backdrop_leaves_every_frame_untouched() {
        // The exe contributes zero pixels behind art-less states, so
        // an empty stage paints nothing on any screen: a patterned
        // backbuffer survives `paint_backdrop` verbatim.
        let art = SceneArt::default();
        assert!(!art.has_figures());
        for screen in [
            Screen::Avi,
            Screen::Channel,
            Screen::Room,
            Screen::Waiting,
            Screen::Loading,
            Screen::Playing,
            Screen::Result,
        ] {
            let mut buf = vec![0x123456u32; 16 * 12];
            art.paint_backdrop(
                &mut Frame {
                    buf: &mut buf,
                    w: 16,
                    h: 12,
                },
                screen,
            );
            assert_eq!(buf, vec![0x123456u32; 16 * 12], "{screen:?}");
        }
    }

    #[test]
    fn staged_dialog_sheet_keeps_its_scroll_track_outside_the_scrim() {
        // Live caller chain:
        //   main.rs::run_play -> ScreenGame::paint -> paint_at
        //   -> ScreenGame::paint_channel -> paint_channel_scene
        //   -> paint_channel_rows -> paint_channel_scrim.
        // The exe's `Dialog_ChannelInfo.ojs` sheet bakes the dark
        // list field and the scroll track; the fallback scrim must not
        // repaint them once that sheet is staged.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(800, 466, [0x1F, 0x00]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(
            buf[300 * 800 + 580],
            pack_xrgb(0, 0, 255),
            "staged sheet owns the scroll track, not the fallback scrim"
        );
    }

    #[test]
    fn staged_dialog_sheet_keeps_its_scroll_arrows_but_not_the_thumb() {
        // Live caller chain:
        //   ScreenGame::paint_channel -> paint_channel_scene
        //   -> paint_channel_rows -> paint_channel_scroll_art.
        // The exe sheet bakes both arrow plates (`Dialog_ChannelInfo.ojs`,
        // native x573..594, arrows y213..233 and y409..429); only the
        // movable thumb is a separate staged body. A repaint of the
        // messenger arrow sprites would swap the white-rule order.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_DIALOG_SPRITE.to_owned(),
                    solid_figure(800, 466, [0x1F, 0x00]),
                ),
                (
                    CHANNEL_SCROLL_UP_SPRITE.to_owned(),
                    solid_figure(21, 20, [0x00, 0x7C]),
                ),
                (
                    CHANNEL_SCROLL_DOWN_SPRITE.to_owned(),
                    solid_figure(21, 20, [0x00, 0x7C]),
                ),
                (
                    CHANNEL_SCROLL_BAR_SPRITE.to_owned(),
                    solid_figure(21, 45, [0xE0, 0x03]),
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(
            buf[220 * 800 + 580],
            pack_xrgb(0, 0, 255),
            "up arrow stays the sheet's baked plate"
        );
        assert_eq!(
            buf[419 * 800 + 580],
            pack_xrgb(0, 0, 255),
            "down arrow stays the sheet's baked plate"
        );
        assert_eq!(
            buf[250 * 800 + 580],
            pack_xrgb(0, 255, 0),
            "thumb still repaints as a staged body"
        );
    }

    #[test]
    fn channel_bar_art_layers_behind_the_live_list() {
        // The code-fetched hover bar (`DIALOG_BAR_SPRITE`, never a
        // `STATE_CHANNEL` entry) stages entry-less and paints as the
        // list highlight at the highlighted row's rect (scaled
        // (216,row_top)): bar ink lands there, the live list on top,
        // the amber cursor stand-in paints nothing, no fill.
        let art = SceneArt {
            placements: unbound_scene_placements(&[DIALOG_BAR_SPRITE.to_owned()]),
            figures: HashMap::from([(DIALOG_BAR_SPRITE.to_owned(), red_figure())]),
        };
        let mut buf = vec![0u32; 64 * 48];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(buf[17 * 64 + 17], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(!buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)));
        assert!(!buf.contains(&background_of(Screen::Channel)));
    }

    /// One solid-colour layer figure (`w` x `h` at (0,0), colorkey
    /// white so every pixel paints).
    fn solid_figure(w: usize, h: usize, word: [u8; 2]) -> AvatarFigure {
        let mut pixels = vec![0u8; w * h * 2];
        for px in pixels.as_chunks_mut::<2>().0 {
            px.copy_from_slice(&word);
        }
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&pixels, w, h).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// Ten-layer digit figure: layer `i` is a solid 13x18 block in
    /// its own colour (colorkey white, so every pixel paints) — a
    /// stand-in for the ten `CH_Num` digit frames in stored order.
    fn digit_figure() -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        for digit in 0u16..10 {
            // Distinct nonzero words: word 0 would read as
            // transparent under a zero colorkey, and the decode is
            // injective so each layer keeps its own colour.
            let word = 0x0821_u16.wrapping_add(digit.wrapping_mul(0x1042));
            let pixels = word.to_le_bytes().repeat(13 * 18);
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&pixels, 13, 18).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    /// Decoded colour of one digit-figure layer (see
    /// [`digit_figure`]): the assertion-side twin of the blit path.
    fn digit_ink(digit: u16) -> u32 {
        let word = 0x0821_u16.wrapping_add(digit.wrapping_mul(0x1042));
        let (r, g, b) = o2jam_file::rgb555_to_rgb888(word);
        pack_xrgb(r, g, b)
    }

    /// Three-layer status bar: layers 0-1 green, layer 2 red — a
    /// stand-in for the `CH_Back` normal/FULL frames (frame 2 carries
    /// the baked FULL face).
    fn status_bar_figure() -> AvatarFigure {
        let mut figure = AvatarFigure::default();
        for word in [[0xE0, 0x03], [0xE0, 0x03], [0x00, 0x7C]] {
            let pixels = word.repeat(347 * 41);
            figure.push_layer(FigureLayer {
                image: Rgb555Image::decode(&pixels, 347, 41).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        figure
    }

    #[test]
    fn channel_digits_select_tens_and_ones_frames() {
        // `CH_Num` decodes one layer per digit in stored order
        // (0-9, Interface1.opi): the tens slot shows `(id+1)/10 % 10`
        // and the ones slot `(id+1) % 10` (the rig-pinned one-based
        // number). Painting the whole figure at both slots would
        // smear every digit into each well.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (CHANNEL_DIGIT_SPRITE.to_owned(), digit_figure()),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[ChannelState {
                gateway_id: 0,
                channel_id: 27,
                capacity: 100,
                population: 5,
                active: true,
            }],
            0,
            0,
            &art,
            None,
            None,
        );
        // Bar-local (50,11)/(63,11) over the row-0 bar origin
        // (217,225): tens well centre (273,245), ones (286,245).
        // The wells show `channel_id + 1` (rig-pinned), so id 27
        // reads `Ch.28`.
        assert_eq!(buf[245 * 800 + 273], digit_ink(2), "tens slot");
        assert_eq!(buf[245 * 800 + 286], digit_ink(8), "ones slot");
    }

    #[test]
    fn channel_digits_read_one_based_off_the_wire_id() {
        // Rig-pinned rule (`docs/channel-fill.md` 2026-11-10 capture,
        // proxy rewrote the 1003 ids): the wells show `channel_id + 1`
        // and slot order follows the wire list. Ids 9 and 10 carry
        // across the tens digit, so `Ch.10` and `Ch.11`.
        let ten = ChannelState {
            gateway_id: 0,
            channel_id: 9,
            capacity: 100,
            population: 0,
            active: true,
        };
        let eleven = ChannelState {
            gateway_id: 0,
            channel_id: 10,
            capacity: 100,
            population: 0,
            active: true,
        };
        assert_eq!(channel_digit(&ten, true), 1, "id 9 tens → 1");
        assert_eq!(channel_digit(&ten, false), 0, "id 9 ones → 0");
        assert_eq!(channel_digit(&eleven, true), 1, "id 10 tens → 1");
        assert_eq!(channel_digit(&eleven, false), 1, "id 10 ones → 1");
    }

    #[test]
    fn channel_fill_clips_to_population_share() {
        // The count strip paints the live population share of its
        // 185x19 plate: half-full fills the left half and leaves
        // the right half bare; a full row fills the whole plate;
        // zero capacity fills nothing.
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (
                    CHANNEL_COUNT_SPRITE.to_owned(),
                    solid_figure(185, 19, [0xE0, 0x03]), // solid green
                ),
            ]),
        };
        let paint_row = |channel: ChannelState| {
            let mut buf = vec![0u32; 800 * 600];
            paint_channel_scene(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &[channel],
                0,
                0,
                &art,
                None,
                None,
            );
            buf
        };
        // Row-0 strip at bar-local (90,12) over the bar origin
        // (217,225): native (307,237)..(492,256).
        let half = paint_row(ChannelState {
            gateway_id: 0,
            channel_id: 1,
            capacity: 100,
            population: 50,
            active: true,
        });
        assert_eq!(half[245 * 800 + 310], green, "filled left half");
        assert_eq!(half[245 * 800 + 450], blue, "bare right half");
        let full = paint_row(ChannelState {
            gateway_id: 0,
            channel_id: 1,
            capacity: 100,
            population: 100,
            active: true,
        });
        assert_eq!(full[245 * 800 + 490], green, "full row fills all");
        let empty = paint_row(ChannelState {
            gateway_id: 0,
            channel_id: 1,
            capacity: 0,
            population: 0,
            active: false,
        });
        assert_eq!(empty[245 * 800 + 310], blue, "zero cap fills none");
    }

    /// Paint one synthetic row and return its frame buffer so the count
    /// strip can be read at pinned native coordinates.
    fn channel_strip_frame(population: i32, capacity: i32) -> Vec<u32> {
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (
                    CHANNEL_COUNT_SPRITE.to_owned(),
                    solid_figure(185, 19, [0xE0, 0x03]), // solid green
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[ChannelState {
                gateway_id: 0,
                channel_id: 0,
                capacity,
                population,
                active: true,
            }],
            0,
            0,
            &art,
            None,
            None,
        );
        buf
    }

    #[test]
    fn channel_single_visitor_lights_one_whole_population_segment() {
        // Live caller chain:
        //   main.rs::run_play -> setup_online -> OnlineSession::paint
        //   -> ScreenGame::hold_channel_rows_for_relay_notice
        //   -> ScreenGame::paint -> paint_channel_scene -> paint_channel_rows
        //   -> paint_channel_bar_art -> channel_fill_width.
        // Proxy-pinned exe frame (population 1, capacity 100) lights one
        // whole segment, not the floor-rounded 1 px of the strip.
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let frame = channel_strip_frame(1, 100);
        assert_eq!(frame[245 * 800 + 310], green, "first segment lights");
        assert_eq!(frame[245 * 800 + 330], blue, "second segment stays dark");
    }

    #[test]
    fn channel_ten_visitors_light_one_segment_not_two() {
        // Live caller chain: same as
        // `channel_single_visitor_lights_one_whole_population_segment`.
        // The live proxy rewrite of population 10/capacity 100 lit exactly
        // one segment, refuting the old pixel-share or floor-plus-one rule.
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let frame = channel_strip_frame(10, 100);
        assert_eq!(frame[245 * 800 + 325], green, "whole first segment lights");
        assert_eq!(frame[245 * 800 + 330], blue, "second segment stays dark");
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn inactive_row_paints_empty_slot_no_digits() {
        // Ticket #190: an inactive row paints the baked empty slot —
        // no digit plates over the tens/ones wells, so the wells keep
        // the bar ink. Red on HEAD (digits painted unconditionally),
        // green after the active-gated fix.
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (CHANNEL_DIGIT_SPRITE.to_owned(), digit_figure()),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[ChannelState {
                gateway_id: 0,
                channel_id: 27,
                capacity: 0,
                population: 0,
                active: false,
            }],
            0,
            0,
            &art,
            None,
            None,
        );
        // Bar-local (50,11)/(63,11) over the row-0 bar origin
        // (217,225): tens well centre (273,245), ones (286,245).
        assert_eq!(
            buf[245 * 800 + 273],
            blue,
            "inactive tens well keeps bar ink"
        );
        assert_eq!(
            buf[245 * 800 + 286],
            blue,
            "inactive ones well keeps bar ink"
        );
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn inactive_row_paints_empty_slot_no_count_strip() {
        // Ticket #190 strip leg: an inactive row paints no count
        // strip, so the strip position keeps the bar ink even with a
        // nonzero population share.
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (
                    CHANNEL_COUNT_SPRITE.to_owned(),
                    solid_figure(185, 19, [0xE0, 0x03]), // solid green
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[ChannelState {
                gateway_id: 0,
                channel_id: 1,
                capacity: 100,
                population: 50,
                active: false,
            }],
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(
            buf[245 * 800 + 310],
            blue,
            "inactive row paints no green strip"
        );
        assert!(
            !buf.contains(&green),
            "inactive row paints no count strip at all"
        );
    }

    #[test]
    fn inactive_row_paints_close_marking_over_the_face_well() {
        // Live caller chain:
        //   main.rs::run_play -> ScreenGame::paint -> paint_at
        //   -> ScreenGame::paint_channel -> paint_channel_scene
        //   -> paint_channel_rows -> paint_channel_face.
        // Ticket #212: an inactive row paints the CLOSE marking at
        // the staged face well (dark well + orange CLOSE) instead of
        // the bar frame's blank gray well. Red while
        // `paint_channel_face` early-returns on `!active`.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (
                    CHANNEL_ENTER_SPRITE.to_owned(),
                    solid_figure(44, 21, [0xE0, 0x7C]),
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[ChannelState {
                gateway_id: 0,
                channel_id: 7,
                capacity: 0,
                population: 0,
                active: false,
            }],
            0,
            0,
            &art,
            None,
            None,
        );
        let face = channel_enter_rect(0, 800, 600);
        assert_eq!(
            buf[face.y as usize * 800 + face.x as usize],
            CHANNEL_CLOSE_WELL_TONE,
            "inactive face well carries the CLOSE dark well, not bar ink"
        );
        let mut close_inked = 0;
        for y in face.y..face.y + face.h as i32 {
            for x in face.x..face.x + face.w as i32 {
                if near_xrgb(buf[y as usize * 800 + x as usize], CHANNEL_CLOSE_TONE) {
                    close_inked += 1;
                }
            }
        }
        assert!(
            close_inked >= 8,
            "inactive face well shows orange CLOSE ink, found {close_inked}"
        );
    }

    #[test]
    fn channel_full_row_uses_full_frame_without_face() {
        // A row at capacity shows the bar's baked FULL frame (layer
        // 2) instead of the normal bar plus the enter face; a live
        // row below capacity keeps frame 0 and its face.
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        let red = pack_xrgb(0xFF, 0x00, 0x00);
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (CHANNEL_BAR_SPRITE.to_owned(), status_bar_figure()),
                (
                    CHANNEL_ENTER_SPRITE.to_owned(),
                    solid_figure(44, 21, [0x1F, 0x00]), // solid blue
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[
                ChannelState {
                    gateway_id: 0,
                    channel_id: 1,
                    capacity: 100,
                    population: 100,
                    active: true,
                },
                ChannelState {
                    gateway_id: 0,
                    channel_id: 2,
                    capacity: 100,
                    population: 5,
                    active: true,
                },
            ],
            0,
            0,
            &art,
            None,
            None,
        );
        // Row 0 (full): baked FULL frame over bar and face well.
        assert_eq!(buf[240 * 800 + 250], red, "full bar frame");
        assert_eq!(buf[240 * 800 + 510], red, "no face on a full row");
        // Row 1 (live): normal bar with the staged face on top.
        assert_eq!(buf[290 * 800 + 250], green, "live bar frame");
        assert_eq!(buf[290 * 800 + 510], blue, "face on a live row");
    }

    #[test]
    fn channel_staged_digits_replace_ascii_text() {
        // The exe paints no row text (captures show digits, segments,
        // and the enter face only): staged digit plates suppress the
        // ASCII fallback string, which stays for the figure-less
        // fallback only.
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (CHANNEL_DIGIT_SPRITE.to_owned(), digit_figure()),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        // The old ASCII anchor (text_x 200, row 0 top 224) sits
        // inside the staged bar: no white/gray text ink paints there.
        assert_eq!(buf[226 * 800 + 250], blue, "bar ink, no text");
        assert!(
            !buf.contains(&pack_xrgb(0x70, 0x70, 0x70)),
            "inactive gray text paints nowhere staged"
        );
    }

    #[test]
    fn channel_dialog_body_stages_behind_the_live_list() {
        // Reference-measured mount: the Dialog_ChannelInfo body paints
        // at its record anchor (0,0), the live list and cursor ink on
        // top, and the region the body never covers (bottom strip,
        // native y 467..600) stays untouched — no invented fill.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(800, 466, [0xE0, 0x03]), // solid green
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        // Body ink inside the measured panel band, outside row chrome.
        assert_eq!(buf[100 * 800 + 700], green);
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
        // No-fill corner: below the 466-tall body nothing paints.
        assert_eq!(buf[599 * 800 + 799], 0);
        assert_eq!(buf[599 * 800], 0);
    }

    #[test]
    fn channel_enter_art_covers_the_measured_faces() {
        // Staged ChannelInfo_CH_Enter frame 0 replaces the procedural
        // plates at the reference-measured faces (502,236+50i): face
        // pixels carry staged ink, the plate tone paints nothing.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_ENTER_SPRITE.to_owned(),
                solid_figure(44, 21, [0x1F, 0x00]), // solid blue
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        // Face of viewport slot 0 (native 502,236 + 44x21 art).
        assert_eq!(buf[240 * 800 + 510], blue);
        // Face of viewport slot 1 is an inactive row: the enter face
        // is not painted (ticket #190).
        assert_ne!(buf[290 * 800 + 510], blue);
        assert!(!buf.contains(&CHANNEL_ENTER_TONE));
    }

    #[test]
    fn channel_bar_art_covers_the_measured_pill_track_band() {
        // Staged ChannelInfo_CH_Back at the bnd bar origin (row 0 bar
        // at (217,225)) replaces the procedural pill/track chrome:
        // measured band pixels carry staged ink, no pill or track
        // tone paints.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([
                (
                    CHANNEL_BAR_SPRITE.to_owned(),
                    solid_figure(347, 41, [0x1F, 0x00]), // solid blue
                ),
                (
                    CHANNEL_ENTER_SPRITE.to_owned(),
                    solid_figure(44, 21, [0x1F, 0x00]),
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        // Measured pill band (205,227 94x35) and track band
        // (297,228 203x33) both sit inside the staged 347x41 bar.
        assert_eq!(buf[240 * 800 + 250], blue, "pill band");
        assert_eq!(buf[240 * 800 + 350], blue, "track band");
        assert!(!buf.contains(&CHANNEL_PILL_TONE));
        assert!(!buf.contains(&CHANNEL_TRACK_TOP_TONE));
        assert!(!buf.contains(&CHANNEL_TRACK_SIDE_TONE));
    }

    #[test]
    fn channel_scene_without_bar_figure_keeps_procedural_chrome() {
        // No staged bar art: the row-interior fallback stays the
        // procedural pill/track chrome (today's behavior).
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(8, 8, [0xE0, 0x03]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert!(buf.contains(&CHANNEL_PILL_TONE));
        assert!(buf.contains(&CHANNEL_TRACK_TOP_TONE));
    }

    #[test]
    fn channel_select_bar_paints_at_the_highlighted_row() {
        // Staged Dialog_ChannelSelectBar (348x44) is the list
        // highlight: it paints at the highlighted row's rect
        // ((216,row_top), live-measured 2026-11-09) and the amber
        // cursor stand-in paints nothing.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                DIALOG_BAR_SPRITE.to_owned(),
                solid_figure(348, 44, [0x1F, 0x00]), // solid blue
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            1,
            &art,
            None,
            None,
        );
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        // Highlight 1 → viewport slot 1, row top 275: bar at (216,275).
        assert_eq!(buf[280 * 800 + 220], blue, "select bar at row 1");
        assert_eq!(buf[300 * 800 + 500], blue, "select bar bottom-right");
        // Row 0 (top 224) carries no bar: above the bar nothing blue.
        assert_ne!(buf[230 * 800 + 220], blue, "no bar on row 0");
        assert!(
            !buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)),
            "amber cursor paints with the select bar staged"
        );
    }

    #[test]
    fn channel_highlight_falls_back_to_the_cursor_without_select_bar() {
        // Without the staged select bar the highlight is the amber
        // cursor (fallback), even with other dialog figures staged.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(8, 8, [0xE0, 0x03]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert!(
            buf.contains(&pack_xrgb(0xFF, 0xD0, 0x20)),
            "amber cursor paints without the select bar"
        );
    }

    #[test]
    fn channel_fallback_chrome_uses_live_measured_wells() {
        // No-archive fallback: the number well (225,228) 71x32 and the
        // track well (298,229) 195x32 paint at the 2026-11-09
        // live-measured rects (row 0; followers re-base on the row
        // tops).
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_list(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
        );
        // Number well interior.
        assert_eq!(buf[250 * 800 + 250], CHANNEL_PILL_TONE, "number well");
        assert_ne!(buf[250 * 800 + 220], CHANNEL_PILL_TONE, "left of the well");
        assert_ne!(buf[250 * 800 + 300], CHANNEL_PILL_TONE, "right of the well");
        // Track well: bright top edge, hollow interior (backdrop only).
        assert_eq!(
            buf[229 * 800 + 300],
            CHANNEL_TRACK_TOP_TONE,
            "track top edge"
        );
        assert_eq!(
            buf[240 * 800 + 350],
            scrim_color(),
            "track interior stays hollow"
        );
    }

    #[test]
    fn planet_board_reference_geometry_is_pinned() {
        // Bare-board constants (2026-11-09 reference measurement) stay
        // pinned for future wiring.
        assert_eq!(
            PLANET_BOARD_PROMO_NAV_RECT,
            Rect {
                x: 345,
                y: 509,
                w: 107,
                h: 27
            }
        );
        assert_eq!(PLANET_BOARD_OPTION_ORIGIN, (712, 562));
        assert_eq!(PLANET_BOARD_EXIT_ORIGIN, (753, 562));
        assert_eq!(
            PLANET_BOARD_MENU_BACK_RECT,
            Rect {
                x: 130,
                y: 562,
                w: 31,
                h: 32
            }
        );
    }

    #[test]
    fn channel_scene_without_enter_figure_keeps_procedural_plates() {
        // No staged enter art: the art path falls back to the
        // procedural plates at the same measured faces.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(8, 8, [0xE0, 0x03]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert!(buf.contains(&CHANNEL_ENTER_TONE));
    }

    #[test]
    fn channel_row_hit_mirrors_the_pinned_row_rects() {
        // Hit-test inverse of the exe-pinned slot rects (CStatePlanet
        // dialog+0x94): a pixel inside the face of slot i maps back to
        // slot i; the rect edges (left/top inclusive, right/bottom
        // exclusive) and pixels outside the rect hit nothing.
        for (slot, &(left, band_top, right, band_bottom)) in CHANNEL_ROW_RECTS.iter().enumerate() {
            let face_y = channel_enter_origin(slot).1 + 5;
            assert_eq!(channel_row_hit(300, face_y, 4), Some(slot));
            assert_eq!(channel_row_hit(300, band_top, 4), Some(slot));
            assert_eq!(channel_row_hit(300, band_bottom - 1, 4), Some(slot));
            assert_eq!(channel_row_hit(left, band_top, 4), Some(slot));
            assert_eq!(channel_row_hit(right - 1, band_bottom - 1, 4), Some(slot));
            assert_eq!(channel_row_hit(right, band_top, 4), None);
        }
        // Bottom edge is exclusive: the last band ends at 436.
        assert_eq!(channel_row_hit(300, 435, 4), Some(3));
        assert_eq!(channel_row_hit(300, 436, 4), None);
        assert_eq!(channel_row_hit(300, 100, 4), None);
        assert_eq!(channel_row_hit(300, 560, 4), None);
        // Issue #29: the scroll track and margins share the bands'
        // y range but sit outside the pinned x range.
        assert_eq!(channel_row_hit(583, 240, 4), None);
        assert_eq!(channel_row_hit(100, 240, 4), None);
    }

    #[test]
    fn channel_face_hit_covers_the_measured_faces() {
        // Ticket #56: the bnd bar-local face (285,11) 44x21 over each
        // slot's bar origin — row 0 reads (502,236)-(546,257), lefts
        // and tops inclusive like the row rects.
        for slot in 0..CHANNEL_SLOT_COUNT {
            let (face_x, face_y) = channel_enter_origin(slot);
            assert!(channel_face_hit(face_x + 22, face_y + 10, slot));
            assert!(channel_face_hit(face_x, face_y, slot));
            assert!(!channel_face_hit(face_x + 44, face_y, slot));
            assert!(!channel_face_hit(face_x, face_y + 21, slot));
        }
        assert_eq!(channel_enter_origin(0), (502, 236));
        assert!(!channel_face_hit(300, 240, 0), "row body is no face");
        assert!(!channel_face_hit(524, 246, 1), "slot 1 face, not slot 0");
    }

    #[test]
    fn thumb_tracks_list_top() {
        // Ticket #174: the scrollbar thumb paints at the list-top
        // fraction of the pinned track (y233-409); arrow-scrolling the
        // list moves it.
        fn six_channels() -> Vec<ChannelState> {
            (0..6)
                .map(|i| ChannelState {
                    gateway_id: 0,
                    channel_id: i as u16,
                    capacity: 10,
                    population: 1,
                    active: true,
                })
                .collect()
        }
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let art = SceneArt {
            placements: Vec::new(),
            figures: std::collections::HashMap::from([(
                CHANNEL_SCROLL_BAR_SPRITE.to_owned(),
                solid_figure(21, 45, [0x1F, 0x00]),
            )]),
        };
        // top = 0 -> thumb at track top (y233).
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &six_channels(),
            0,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(buf[233 * 800 + 583], blue, "thumb at track top");
        // top = 2 (max) -> thumb origin at track bottom (y364).
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &six_channels(),
            2,
            0,
            &art,
            None,
            None,
        );
        assert_eq!(buf[364 * 800 + 583], blue, "thumb at track bottom");
        // no ink below down arrow (y429)
        let mut any = false;
        for y in 429..600 {
            if buf[y * 800 + 583] == blue {
                any = true;
                break;
            }
        }
        assert!(!any, "thumb never paints below down arrow");
    }

    #[test]
    fn channel_scroll_hit_uses_the_bnd_track_arrows() {
        // Ticket #55: the bnd-pinned arrows at scroll-local
        // (0,0)/(0,196) (21x20 each) over the (573,213) track — the
        // bar body between them and pixels off the track hit nothing.
        assert_eq!(channel_scroll_hit(583, 223), Some(ChannelScroll::Up));
        assert_eq!(channel_scroll_hit(573, 213), Some(ChannelScroll::Up));
        assert_eq!(channel_scroll_hit(593, 232), Some(ChannelScroll::Up));
        assert_eq!(channel_scroll_hit(583, 419), Some(ChannelScroll::Down));
        assert_eq!(channel_scroll_hit(583, 300), None, "bar body");
        assert_eq!(channel_scroll_hit(300, 223), None, "row band");
        assert_eq!(channel_scroll_hit(572, 223), None, "left of track");
        assert_eq!(channel_scroll_hit(594, 223), None, "right of track");
    }

    #[test]
    fn channel_dialog_x_hit_uses_the_bnd_rect() {
        // Bnd-pinned X button (541,151)-(589,189), right/bottom
        // exclusive (`docs/channel-fill.md` "METHOD CORRECTION
        // 2026-11-10"): the verified native close click (577,171)
        // hits; the rect edges read 541/151 inclusive, 589/189
        // exclusive; row faces stay outside.
        assert!(channel_dialog_x_hit(577, 171), "verified native X click");
        assert!(channel_dialog_x_hit(541, 151), "left/top edge inclusive");
        assert!(channel_dialog_x_hit(588, 188), "last inside pixel");
        assert!(!channel_dialog_x_hit(589, 151), "right edge exclusive");
        assert!(!channel_dialog_x_hit(541, 189), "bottom edge exclusive");
        assert!(!channel_dialog_x_hit(540, 170), "left of the rect");
        assert!(!channel_dialog_x_hit(300, 240), "a row face");
    }

    #[test]
    fn channel_enter_face_is_bar_local_per_row() {
        // Bnd bar-local math: bar origin = row origin +1; face = bar +
        // (285,11). Row 0 reads (502,236) — exact against the exe-
        // pinned face inset and the reference capture.
        assert_eq!(channel_enter_origin(0), (502, 236));
        assert_eq!(channel_enter_origin(1), (502, 287));
        assert_eq!(channel_enter_origin(2), (502, 336));
        assert_eq!(channel_enter_origin(3), (502, 387));
        for (slot, &(row_l, row_t, ..)) in CHANNEL_ROW_RECTS.iter().enumerate() {
            assert_eq!(
                channel_enter_origin(slot),
                (row_l + 1 + 285, row_t + 1 + 11),
                "face of row {slot} = bar + (285,11)"
            );
        }
    }

    #[test]
    fn channel_scene_paints_the_planet_board_under_the_dialog() {
        // Verified first screen (docs/planet.md, 2026-11-09): the
        // channel dialog opens OVER the planet board. The staged
        // board (bound State_Planet placements: backdrop + emblems +
        // promo + MENU bar) paints first — its MENU bar band stays
        // visible in the bottom strip (y>468), under the opaque
        // dialog sheet — and the no-fill corner rule keeps pixels no
        // art covers untouched. Board emblem ink inside the dialog's
        // covered band is replaced by the opaque sheet.
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(800, 466, [0xE0, 0x03]), // solid green
            )]),
        };
        let board = SceneArt {
            placements: unbound_scene_placements(&["Board_Backdrop.ojs".to_owned()]),
            figures: HashMap::from([(
                "Board_Backdrop.ojs".to_owned(),
                solid_figure(800, 600, [0x1F, 0x00]), // solid blue
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            Some(StagedPlanetBoard {
                art: &board,
                promo: PlanetPromoPaint::spinning(0),
            }),
            None,
        );
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        let green = pack_xrgb(0x00, 0xFF, 0x00);
        // The opaque dialog sheet covers y0-468 with its own ink.
        assert_eq!(buf[100 * 800 + 700], green);
        // The board's MENU bar band stays visible in the bottom strip.
        assert_eq!(buf[599 * 800 + 799], blue);
        assert_eq!(buf[599 * 800], blue);
        // No-fill corner: with the full-bleed board nothing stays
        // blank; a board-less corner keeps the rule (see the
        // dialog-body test above).
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
    }

    #[test]
    fn channel_scene_without_board_keeps_the_art_less_strip() {
        // No board staged: the strip below the 466-tall dialog body
        // stays untouched (pinned no-fill corner, unchanged rule for
        // the nothing-beneath case).
        let art = SceneArt {
            placements: Vec::new(),
            figures: HashMap::from([(
                CHANNEL_DIALOG_SPRITE.to_owned(),
                solid_figure(800, 466, [0xE0, 0x03]),
            )]),
        };
        let empty_board = SceneArt::default();
        let mut buf = vec![0u32; 800 * 600];
        paint_channel_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &two_channels(),
            0,
            0,
            &art,
            Some(StagedPlanetBoard {
                art: &empty_board,
                promo: PlanetPromoPaint::spinning(0),
            }),
            None,
        );
        assert_eq!(buf[599 * 800 + 799], 0, "no-fill corner untouched");
    }

    #[test]
    fn waiting_caption_ids_name_the_pinned_controls() {
        // Codes from `docs/caption-sources.md`: MAPNAME `0x16400018`,
        // BPM `0x16400069`, skill counts `0x1640005E..60`.
        assert_eq!(WAITING_MAPNAME_ID, "IDC_TEXT_SWAIT_MAPNAME");
        assert_eq!(WAITING_BPM_ID, "IDC_TEXT_SWAIT_BPM");
        assert_eq!(
            WAITING_SKILLCOUNT_IDS,
            [
                "IDC_TEXT_SWAIT_1SKILLCOUNT",
                "IDC_TEXT_SWAIT_2SKILLCOUNT",
                "IDC_TEXT_SWAIT_3SKILLCOUNT",
            ]
        );
    }

    #[test]
    fn waiting_mapname_cycles_the_room_word_table() {
        // Map-cycle path: word 0 selects slot 3, otherwise the low
        // word modulo 4 (files `0xb3cc6`–`0xb3ce3`). The three
        // CP949-sourced Korean names ride the embedded TTF.
        assert_eq!(waiting_map_name(0), "원하지않은결말");
        assert_eq!(waiting_map_name(1), "한가한대전장의오후");
        assert_eq!(waiting_map_name(2), "패자의역습");
        assert_eq!(waiting_map_name(3), "원하지않은결말");
        assert_eq!(waiting_map_name(4), "None");
    }

    #[test]
    fn waiting_mapname_paints_at_the_staged_rect() {
        // MAPNAME probe `(480,15,570,38)`: room word 1 names slot 1,
        // so the scrim opens at (476,13) with white glyph ink.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_MAPNAME",
            None,
            (480, 15, 570, 38),
        )];
        let mut room = champion_room();
        room.arena = 1;
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_mapname(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &room,
            &placements,
            false,
        );
        assert_eq!(buf[13 * 800 + 476], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_bpm_formats_whole_tempos() {
        // `%0.f` over the catalog column (store file `0xbe1d5`).
        assert_eq!(waiting_bpm_caption(Some(124.0)), "124");
        assert_eq!(waiting_bpm_caption(Some(124.4)), "124");
        assert_eq!(waiting_bpm_caption(Some(168.8)), "169");
    }

    #[test]
    fn waiting_bpm_defaults_to_mismatch_without_a_record() {
        // Mismatch default `-0-` (rdata file `0x24a1d0`): no staged
        // record, a blank lookup, or a non-finite tempo.
        assert_eq!(waiting_bpm_caption(None), "-0-");
        assert_eq!(waiting_bpm_caption(Some(f32::NAN)), "-0-");
        assert_eq!(waiting_bpm_caption(Some(f32::INFINITY)), "-0-");
    }

    #[test]
    fn waiting_bpm_paints_at_the_staged_rect() {
        // BPM probe `(620,60,680,80)`: staged 124 BPM inks the rect.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_BPM",
            None,
            (620, 60, 680, 80),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_bpm(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            Some(124.0),
            &placements,
        );
        assert_eq!(buf[58 * 800 + 616], 0, "the blue well has no shell scrim");
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_bpm_paints_the_mismatch_default() {
        // Same rect without a staged record: `-0-` inks it, never a
        // blank or an invented tempo.
        let placements = vec![waiting_probe(
            "IDC_TEXT_SWAIT_BPM",
            None,
            (620, 60, 680, 80),
        )];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_bpm(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            &placements,
        );
        assert_eq!(buf[58 * 800 + 616], 0, "the blue well has no shell scrim");
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn waiting_skillcounts_pad_and_cap_late_slots() {
        // Width-3 decimal (helper `0x62c109`); slots 2–3 clamp at
        // 999 while slot 1 shows the full count.
        assert_eq!(waiting_skillcount_text(7, false), "  7");
        assert_eq!(waiting_skillcount_text(42, true), " 42");
        assert_eq!(waiting_skillcount_text(1500, false), "1500");
        assert_eq!(waiting_skillcount_text(1500, true), "999");
    }

    #[test]
    fn cycled_skill_slot_repaints_its_selected_id_and_count() {
        // #767 paint leg: after a RIGHT cycle the staged selector owns
        // item 22 with count 42, so both the ring id and the width-3
        // count ink the exact glyphs the exe's handler would store
        // through `0x50f520`. Seam pin (synthetic probes); the live
        // click chain is guarded in `screens::game::game_tests`.
        let mut slots = WaitingSkillSlots::default();
        assert!(slots.set_slot(
            0,
            vec![
                WaitingSkillItem { id: 11, count: 7 },
                WaitingSkillItem { id: 22, count: 42 },
            ]
        ));
        assert!(slots.cycle(0, WaitingSkillStep::Next));
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_1SKILL", None, (701, 208, 731, 238)),
            waiting_probe("IDC_TEXT_SWAIT_1SKILLCOUNT", None, (740, 208, 780, 238)),
        ];
        let mut actual = vec![0u32; 800 * 600];
        paint_waiting_skills(
            &mut Frame {
                buf: &mut actual,
                w: 800,
                h: 600,
            },
            &champion_room(),
            Some(&slots),
            &placements,
        );
        paint_waiting_skillcounts(
            &mut Frame {
                buf: &mut actual,
                w: 800,
                h: 600,
            },
            &[0, 0, 0],
            Some(&slots),
            &placements,
        );
        let mut expected = vec![0u32; 800 * 600];
        paint_room_detail_line(
            &mut Frame {
                buf: &mut expected,
                w: 800,
                h: 600,
            },
            701,
            208,
            "22",
            art_title_ink(),
        );
        paint_room_detail_line(
            &mut Frame {
                buf: &mut expected,
                w: 800,
                h: 600,
            },
            740,
            208,
            " 42",
            art_title_ink(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn waiting_skillcounts_paint_at_their_staged_rects() {
        // Count probes below the ring rects: `[7, 42, 1500]` inks
        // all three, the late slots through the 999 cap.
        let placements = vec![
            waiting_probe("IDC_TEXT_SWAIT_1SKILLCOUNT", None, (740, 208, 780, 238)),
            waiting_probe("IDC_TEXT_SWAIT_2SKILLCOUNT", None, (740, 246, 780, 276)),
            waiting_probe("IDC_TEXT_SWAIT_3SKILLCOUNT", None, (740, 284, 780, 314)),
        ];
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_skillcounts(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &[7, 42, 1500],
            None,
            &placements,
        );
        assert_eq!(buf[206 * 800 + 736], scrim_color());
        assert_eq!(buf[244 * 800 + 736], scrim_color());
        assert_eq!(buf[282 * 800 + 736], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn waiting_leaves_instname_and_notice_rects_unpainted() {
        // `INSTNAME` names zero code immediates (OPEN 1) and the
        // NOTICE dialog `+0x46c` fill is untraced (OPEN 2): their
        // rects stay untouched while the pinned MAPNAME rect inks.
        let placements = vec![
            waiting_probe("IDC_TEXT_SWAIT_MAPNAME", None, (480, 15, 570, 38)),
            waiting_probe("IDC_TEXT_SWAIT_INSTNAME", None, (480, 40, 570, 60)),
            waiting_probe("IDC_IMAGE_SWAIT_NOTICE_TEXT", None, (100, 455, 400, 475)),
            waiting_probe("IDC_IMAGE_SWAIT_NOTICEBOARD", None, (100, 480, 400, 530)),
        ];
        let mut room = champion_room();
        room.arena = 1;
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_staged(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &room,
            &placements_stage(&placements),
            &WaitingCaptions::default(),
            true,
            false,
        );
        assert_eq!(buf[13 * 800 + 476], scrim_color(), "mapname inks");
        assert_eq!(buf[40 * 800 + 480], 0, "instname untouched");
        assert_eq!(buf[455 * 800 + 100], 0, "notice text untouched");
        assert_eq!(buf[480 * 800 + 100], 0, "notice board untouched");
    }

    /// Fold one word into a stable characterization hash (FNV-style
    /// mixing; no dependency on `DefaultHasher` keys).
    fn fold_pin_word(hash: &mut u64, word: u64) {
        *hash = hash.wrapping_mul(0x100_0000_01B3).rotate_left(17) ^ word;
    }

    fn fold_pin_text(hash: &mut u64, text: &str) {
        fold_pin_word(hash, 0xA1A1);
        for byte in text.as_bytes() {
            fold_pin_word(hash, u64::from(*byte));
        }
        fold_pin_word(hash, 0xB2B2);
    }

    fn fold_pin_pixels(hash: &mut u64, pixels: &[u32]) {
        fold_pin_word(hash, 0xC3C3);
        for pixel in pixels {
            fold_pin_word(hash, u64::from(*pixel));
        }
        fold_pin_word(hash, 0xD4D4);
    }

    #[test]
    fn result_screen_generation_pins_verdict_grade_readouts_rows_and_painted_pixels() {
        // Seam pin for the result paint generator. The live chain is
        // `ScreenGame::paint_at` → `paint_result` → `paint_result_content`
        // and `paint_result_scene`; reaching it from boot needs a
        // finished chart and live 4018 rank rows. Staged fixtures drive
        // that same generator here, so the pin covers the painter's
        // outputs without depending on the shell loop or the wire.
        let mut hash = 0u64;

        for sprite in result_loader_sprites() {
            fold_pin_text(&mut hash, sprite);
        }
        for verdict in [
            ResultVerdict::Solo,
            ResultVerdict::Battle(Some(BattleVerdict::Win)),
            ResultVerdict::Battle(Some(BattleVerdict::Lose)),
            ResultVerdict::Battle(Some(BattleVerdict::Other)),
            ResultVerdict::Battle(None),
        ] {
            fold_pin_word(&mut hash, u64::from(winlose_frame(verdict).unwrap_or(0)));
        }
        let teams = battle_room();
        for self_team in [Some(1u8), Some(0), None] {
            let standing = match battle_verdict(&teams, self_team) {
                Some(BattleVerdict::Win) => 1,
                Some(BattleVerdict::Lose) => 2,
                Some(BattleVerdict::Other) => 3,
                None => 0,
            };
            fold_pin_word(&mut hash, standing);
        }

        let score = full_score();
        for note_count in [0u16, 100, 500, 800, 1_200] {
            fold_pin_word(
                &mut hash,
                u64::from(result_grade_frame(&score, note_count).unwrap_or(0)),
            );
        }
        for text in [
            result_judgements_text(score.counts),
            result_combos_text((score.max_combo, 5)),
            result_score_text(87_654_321),
            result_bonus_text(50),
            truncate_result_name(&"x".repeat(120)),
        ] {
            fold_pin_text(&mut hash, &text);
        }
        for row in [0usize, 1, 7, usize::MAX] {
            for y in [result_rank_text_y(row), result_rank_name_y(row)] {
                let signed = y.map_or(0i64, i64::from);
                fold_pin_word(&mut hash, u64::try_from(signed).unwrap_or(u64::MAX));
            }
        }
        for mode in [
            RESULT_DISPLAY_MODE_TABLE_F,
            RESULT_DISPLAY_MODE_TABLE_10,
            0x11,
        ] {
            let slots = result_portrait_slots(mode);
            fold_pin_word(&mut hash, u64::from(slots.is_some()));
            if let Some(slots) = slots {
                for slot in slots {
                    fold_pin_word(&mut hash, u64::from(slot.word));
                }
                for byte in [1u8, 8, 0, 9] {
                    let resolved = portrait_slot_for_seat_byte(byte, &slots)
                        .map_or(0, |slot| u64::from(slot.word));
                    fold_pin_word(&mut hash, resolved);
                }
            }
        }

        let rows = [
            live_row(),
            ResultRankRow {
                rank: Some(2),
                name: Some("RIVAL".to_owned()),
                judgements: Some([700, 50, 10, 4]),
                combos: Some((211, 3)),
                score: Some(98_765),
                bonus: Some(7),
                speed: Some(1),
                seat_byte: Some(8),
            },
        ];
        let flat_slots = result_portrait_slots(RESULT_DISPLAY_MODE_TABLE_F);
        let mut flat = vec![0u32; 800 * 600];
        paint_result(
            &mut Frame {
                buf: &mut flat,
                w: 800,
                h: 600,
            },
            &score,
            &rows,
            flat_slots.as_ref(),
        );
        fold_pin_pixels(&mut hash, &flat);

        let art = rank_art();
        let mut staged = vec![0u32; 800 * 600];
        paint_result_scene(
            &mut Frame {
                buf: &mut staged,
                w: 800,
                h: 600,
            },
            &ResultPaint {
                portrait_slots: flat_slots,
                score: &score,
                rows: &rows,
                art: &art,
                verdict: ResultVerdict::Solo,
                grade: result_grade_frame(&score, 1_000),
                life_readout: Some(1_234),
                bottom_counter: Some(4_321),
            },
        );
        fold_pin_pixels(&mut hash, &staged);

        // #294 T4 removed the walked-OJN cover layer; the new pin
        // certifies the OPI-only painter at the same staged inputs.
        assert_eq!(
            hash, 0x3C69_15F1_2ACB_0F03,
            "result paint generation changed; re-check the exe pins before updating"
        );
    }

    #[test]
    fn room_screen_generation_pins_rows_details_icons_and_painted_pixels() {
        // Seam pin for the Room paint generator. The live chain is
        // `ScreenGame::paint_at` → `paint_room_scene_with_hover` and
        // `paint_room_user_panel`; reaching it from boot needs a live
        // lobby with staged `State_Room` art. Staged fixtures drive
        // that same generator here, so the pin covers the painter's
        // outputs without depending on the shell loop or the wire.
        let mut hash = 0u64;

        let mut rooms = two_rooms();
        rooms.push(detailed_room());
        for room in &rooms {
            fold_pin_text(&mut hash, &room_row_text(room));
        }
        for game_mode in [0u8, 1, 9, 255] {
            fold_pin_text(&mut hash, &room_mode_label(game_mode));
        }
        for music_id in [0u16, 100, 757] {
            fold_pin_text(&mut hash, &room_music_label(music_id));
        }
        for difficulty in [0u8, 1, 2, 3, 255] {
            fold_pin_text(&mut hash, &room_difficulty_label(difficulty));
        }
        for key_mode in [0u8, 1, 2, 7, 255] {
            fold_pin_text(&mut hash, &room_key_label(key_mode));
        }
        for speed in [0u8, 1, 3, 11, 255] {
            fold_pin_text(&mut hash, &room_speed_label(speed));
        }
        for room in &rooms {
            for (id, text) in room_detail_entries(room) {
                fold_pin_text(&mut hash, id);
                fold_pin_text(&mut hash, &text);
            }
            fold_pin_word(&mut hash, u64::from(room_lock_ink(room.has_password)));
        }
        for state_code in [0u8, 1, 2, 7, 255] {
            fold_pin_word(&mut hash, u64::from(room_state_icon_frame(state_code)));
        }
        for key_mode in [0u8, 1, 2, 3, 7, 255] {
            fold_pin_word(
                &mut hash,
                u64::from(room_key_icon_frame(key_mode).unwrap_or(0xF0)),
            );
        }
        for (music_id, level) in [
            (0u16, None),
            (99, Some(1u16)),
            (100, Some(1)),
            (757, Some(2)),
            (757, Some(8)),
            (757, None),
        ] {
            fold_pin_word(
                &mut hash,
                u64::from(room_om_icon_frame(music_id, level).unwrap_or(0xF0)),
            );
        }
        for (frame, layers) in [(0u8, 0usize), (0, 4), (1, 4), (4, 4), (9, 4)] {
            let layer = icon_frame_layer(frame, layers);
            fold_pin_word(
                &mut hash,
                layer.map_or(u64::MAX, |index| u64::try_from(index).unwrap_or(u64::MAX)),
            );
        }
        for id in ROOM_PANEL_IDS {
            fold_pin_text(&mut hash, id);
        }
        let art = room_icon_art();
        for id in room_panel_instances(&art.placements) {
            fold_pin_text(&mut hash, id);
        }
        for code in [0x1320_0038, 0x1320_0039, 0x1320_003A, 0x1350_002E] {
            fold_pin_word(&mut hash, u64::from(room_chrome_visible(code)));
        }
        for (x, y) in [(26, 82), (317, 177), (0, 0)] {
            let panel = room_panel_hit(&art.placements, x, y);
            fold_pin_word(
                &mut hash,
                panel.map_or(u64::MAX, |index| u64::try_from(index).unwrap_or(u64::MAX)),
            );
        }

        let mut flat = vec![0u32; 800 * 600];
        paint_room_list(
            &mut Frame {
                buf: &mut flat,
                w: 800,
                h: 600,
            },
            &rooms,
            1,
        );
        fold_pin_pixels(&mut hash, &flat);

        let mut staged = vec![0u32; 800 * 600];
        paint_room_scene_with_hover(
            &mut Frame {
                buf: &mut staged,
                w: 800,
                h: 600,
            },
            &rooms,
            1,
            &art,
            None,
            Some(1),
        );
        fold_pin_pixels(&mut hash, &staged);

        let character = CharacterInfo {
            nickname: "tester".to_owned(),
            level: 2,
            experience: 2_133,
            bonus_point: 7,
            penalty_count: 1,
            penalty_level: 3,
            ..CharacterInfo::default()
        };
        let mut panel = vec![0u32; 800 * 600];
        paint_room_user_panel(
            &mut Frame {
                buf: &mut panel,
                w: 800,
                h: 600,
            },
            &room_user_panel_placements(),
            Some(&character),
            None,
            1_234,
        );
        fold_pin_pixels(&mut hash, &panel);

        assert_eq!(
            hash, 0xBAD4_1640_C080_E23A,
            "room paint generation changed; re-check the exe pins before updating"
        );
    }

    #[test]
    fn chat_boxes_pin_rows_ink_geometry_and_painted_pixels() {
        // Seam pin for the three live chat boxes. The caller chain is
        // `ScreenGame::paint_at` → `paint_waiting` / `paint_channel` /
        // `paint_room` → `paint_waiting_chat` / `paint_channel_chat` /
        // `paint_room_chat`; reaching it from boot needs a decoded
        // 3012/3013/2013/2014/2016/2019/2029 packet stream and staged
        // `State_Waiting`/`State_Room` rects. Staged fixtures drive
        // these same painters below, so the seam test covers their
        // generated ink and pixels without the wire.
        let mut hash = 0u64;

        let waiting_rows = vec![
            WaitingChatMessage::user("host", "hello"),
            WaitingChatMessage::admin("op", "quiet"),
            WaitingChatMessage::user("a-longer-nick", "a longer line"),
        ];
        for row in &waiting_rows {
            fold_pin_text(&mut hash, &waiting_chat_row_text(row));
            fold_pin_word(&mut hash, u64::from(row.admin));
            fold_pin_word(&mut hash, u64::from(chat_row_ink(row.admin)));
        }
        fold_pin_word(&mut hash, u64::from(chat_user_ink()));
        fold_pin_word(&mut hash, u64::from(chat_admin_ink()));

        let channel_rows = vec![
            ChannelChatMessage::user("host", "hello"),
            ChannelChatMessage::admin("op", "quiet"),
            ChannelChatMessage::whisper("host", "me", "psst"),
            ChannelChatMessage::announcement("notice"),
            ChannelChatMessage::rank("host", "M757", 1),
        ];
        for row in &channel_rows {
            fold_pin_text(&mut hash, &channel_chat_row_text(row));
            fold_pin_word(&mut hash, u64::from(row.admin));
            fold_pin_word(&mut hash, u64::from(chat_row_ink(row.admin)));
        }
        fold_pin_text(&mut hash, &rank_notice_board_text("host", "M757", 1));

        let mut long_waiting = Vec::new();
        for index in 0..WAITING_CHAT_VISIBLE_ROWS + 2 {
            long_waiting.push(WaitingChatMessage::user("host", &format!("line{index}")));
        }
        for row in waiting_chat_visible_messages(&long_waiting) {
            fold_pin_text(&mut hash, &row.text);
        }
        let mut long_channel = Vec::new();
        for index in 0..CHANNEL_CHAT_VISIBLE_ROWS + 2 {
            long_channel.push(ChannelChatMessage::user("host", &format!("line{index}")));
        }
        for row in channel_chat_visible_messages(&long_channel) {
            fold_pin_text(&mut hash, &row.text);
        }

        for rect in [WAITING_CHAT_LIST_RECT, WAITING_CHAT_EDIT_RECT] {
            for word in [rect.x, rect.y, rect.w as i32, rect.h as i32] {
                fold_pin_word(&mut hash, u64::try_from(word).unwrap_or(u64::MAX));
            }
        }
        for word in [
            WAITING_CHAT_ROW_H,
            WAITING_CHAT_VISIBLE_ROWS as i32,
            WAITING_CHAT_CARET_INSET.0,
            WAITING_CHAT_CARET_INSET.1,
            WAITING_CHAT_CARET_SIZE.0 as i32,
            WAITING_CHAT_CARET_SIZE.1 as i32,
            CHANNEL_CHAT_VISIBLE_ROWS as i32,
            CHANNEL_CHAT_ROW_H,
            ROOM_CHAT_ROW_H,
            ROOM_CHAT_CARET_INSET.0,
            ROOM_CHAT_CARET_INSET.1,
        ] {
            fold_pin_word(&mut hash, u64::try_from(word).unwrap_or(u64::MAX));
        }
        fold_pin_word(&mut hash, u64::from(ROOM_CHAT_LIST_CODE));
        fold_pin_word(&mut hash, u64::from(ROOM_CHAT_EDIT_CODE));

        let staged_waiting = vec![
            waiting_probe("IDC_LIST_SWAIT_CHAT", None, (24, 455, 511, 553)),
            waiting_probe("IDC_EDIT_SWAIT_CHAT", None, (20, 563, 460, 583)),
        ];
        let mut scratch = vec![0u32; 800 * 600];
        let frame = Frame {
            buf: &mut scratch,
            w: 800,
            h: 600,
        };
        for placements in [Some(staged_waiting.as_slice()), None] {
            let placements = placements.unwrap_or(&[]);
            for origin in [
                waiting_chat_list_origin(&frame, placements),
                waiting_chat_edit_origin(&frame, placements),
            ] {
                fold_pin_word(&mut hash, u64::try_from(origin.0).unwrap_or(u64::MAX));
                fold_pin_word(&mut hash, u64::try_from(origin.1).unwrap_or(u64::MAX));
            }
        }
        for offset in 0..WAITING_CHAT_VISIBLE_ROWS {
            fold_pin_word(
                &mut hash,
                u64::try_from(waiting_chat_row_y(455, offset)).unwrap_or(u64::MAX),
            );
        }

        let mut waiting_staged_pixels = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut waiting_staged_pixels,
                w: 800,
                h: 600,
            },
            &long_waiting,
            "go",
            &staged_waiting,
        );
        fold_pin_pixels(&mut hash, &waiting_staged_pixels);

        let mut waiting_native_pixels = vec![0u32; 800 * 600];
        paint_waiting_chat(
            &mut Frame {
                buf: &mut waiting_native_pixels,
                w: 800,
                h: 600,
            },
            &waiting_rows,
            "",
            &[],
        );
        fold_pin_pixels(&mut hash, &waiting_native_pixels);

        let mut channel_pixels = vec![0u32; 800 * 600];
        paint_channel_chat(
            &mut Frame {
                buf: &mut channel_pixels,
                w: 800,
                h: 600,
            },
            &long_channel,
            3,
        );
        fold_pin_pixels(&mut hash, &channel_pixels);

        let room_stage = vec![
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_LIST_SROOM_CHAT".to_owned(),
                code: ROOM_CHAT_LIST_CODE,
                sprite: None,
                rect: Rect {
                    x: 171,
                    y: 432,
                    w: 412,
                    h: 119,
                },
            },
            ScenePlacement {
                role: PlacementRole::Single,
                id: "IDC_EDIT_SROOM_CHAT".to_owned(),
                code: ROOM_CHAT_EDIT_CODE,
                sprite: None,
                rect: Rect {
                    x: 167,
                    y: 561,
                    w: 335,
                    h: 19,
                },
            },
        ];
        let mut room_pixels = vec![0u32; 800 * 600];
        paint_room_chat(
            &mut Frame {
                buf: &mut room_pixels,
                w: 800,
                h: 600,
            },
            &channel_rows,
            "draft",
            &room_stage,
        );
        fold_pin_pixels(&mut hash, &room_pixels);

        assert_eq!(
            hash, 0x0ADA_B313_1DE3_E861,
            "chat box generation changed; re-check the exe pins before updating"
        );
    }

    /// Synthetic Waiting art with two backdrop records, one live seat
    /// clone, a confirm face, a 60-cell badge, and every caption rect.
    fn waiting_golden_art() -> SceneArt {
        let mut art = waiting_role_art();
        let star = solid_figure(1, 1, [0x1F, 0x00]);
        let mut badge = AvatarFigure::default();
        for cell in 0u8..60 {
            let word = 0x7C00 | (u16::from(cell % 8) << 6) | u16::from(cell / 8);
            badge.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.to_le_bytes(), 1, 1).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        let mut confirm = AvatarFigure::default();
        for word in [[0x00, 0x7C], [0xE0, 0x03], [0x1F, 0x00]] {
            confirm.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word, 1, 1).unwrap(),
                dx: 0,
                dy: 0,
                colorkey: 0xFFFF,
            });
        }
        let caption = |id: &'static str, rect: (i32, i32, i32, i32)| waiting_probe(id, None, rect);
        art.placements.extend([
            ScenePlacement {
                code: 0x1650_002F,
                ..waiting_probe(
                    WAITING_SEAT_PARENT_IDS[0],
                    Some("Wait_BossMark.ojs"),
                    (20, 60, 40, 80),
                )
            },
            waiting_probe(WAITING_LEVEL_ID, Some("Wait_Level.ojs"), (218, 12, 273, 40)),
            waiting_probe(
                "IDC_BUTTON_SWAIT_START",
                Some("Wait_Start.ojs"),
                (10, 10, 50, 30),
            ),
            caption(WAITING_ROOMNAME_ID, (280, 12, 560, 36)),
            caption(WAITING_MUSICNAME_ID, (573, 15, 773, 38)),
            caption(WAITING_MAPNAME_ID, (400, 40, 560, 60)),
            caption(WAITING_BPM_ID, (600, 40, 660, 60)),
            caption(WAITING_SPEED_NUM_ID, (660, 40, 720, 60)),
            caption(WAITING_WINNERSCORE_ID, (720, 80, 780, 100)),
            caption(WAITING_TEAM_IDS[1], (260, 100, 320, 120)),
            caption(WAITING_AVATAR_SLOT_ID, (0, 0, 123, 150)),
            caption(WAITING_SEAT_PARENT_IDS[0], (37, 55, 160, 205)),
        ]);
        art.placements.extend(
            WAITING_SKILL_IDS
                .iter()
                .zip(WAITING_SKILLCOUNT_IDS)
                .enumerate()
                .flat_map(|(slot, (skill, count))| {
                    let y = 200 + slot as i32 * 20;
                    [
                        caption(skill, (620, y, 650, y + 15)),
                        caption(count, (660, y, 720, y + 15)),
                    ]
                }),
        );
        art.figures.insert("Wait_BossMark.ojs".to_owned(), star);
        art.figures.insert("Wait_Level.ojs".to_owned(), badge);
        art.figures.insert("Wait_Start.ojs".to_owned(), confirm);
        art
    }

    #[test]
    fn waiting_scene_generation_pins_captions_faces_and_painted_pixels() {
        // Seam pin for the Waiting paint generator. The live chain is
        // `ScreenGame::paint_at` → `paint_waiting` → `paint_waiting_scene`,
        // with `paint_waiting_members` and `paint_waiting_seat_avatars`
        // on the same screen. Reaching it from boot needs a joined room
        // and staged `State_Waiting` art. Synthetic probes drive those
        // same painters here, so the pin covers their generated text
        // and full painted frames without the wire or the archives.
        let mut hash = 0u64;
        let room = waiting_room();
        let champion = champion_room();

        for (index, seat) in waiting_seats(&champion).iter().enumerate() {
            fold_pin_text(&mut hash, &waiting_row_text(index, seat));
            fold_pin_word(&mut hash, u64::from(ready_dot_color(seat.is_ready())));
            let (x, y) = seat_origin(index, 800, 600);
            fold_pin_word(&mut hash, u64::try_from(x).unwrap_or(u64::MAX));
            fold_pin_word(&mut hash, u64::try_from(y).unwrap_or(u64::MAX));
        }
        let staged_seats = staged_seats(&champion, None, false);
        for seat in &staged_seats {
            fold_pin_text(&mut hash, &waiting_seat_caption(seat));
        }
        for arena in [0, 1, 2, 3, 4, 9] {
            fold_pin_text(&mut hash, waiting_map_name(arena));
        }
        for bpm in [None, Some(0.0), Some(124.4), Some(f32::NAN)] {
            fold_pin_text(&mut hash, &waiting_bpm_caption(bpm));
        }
        for (count, capped) in [(0, false), (7, false), (42, true), (1500, true)] {
            fold_pin_text(&mut hash, &waiting_skillcount_text(count, capped));
        }
        for (music_id, title) in [(757, None), (757, Some("Song")), (0, None)] {
            fold_pin_text(&mut hash, &waiting_music_caption(music_id, title));
        }
        for level in [
            WaitingLevel {
                difficulty: 0,
                speed: 0,
                ex_chart_level: None,
            },
            WaitingLevel {
                difficulty: 2,
                speed: 5,
                ex_chart_level: None,
            },
            WaitingLevel {
                difficulty: 4,
                speed: 11,
                ex_chart_level: None,
            },
            WaitingLevel {
                difficulty: 1,
                speed: 12,
                ex_chart_level: None,
            },
        ] {
            let layer = waiting_level_layer(level).unwrap_or(usize::MAX);
            fold_pin_word(&mut hash, u64::try_from(layer).unwrap_or(u64::MAX));
        }
        for id in [
            "IDC_BUTTON_SWAIT_START",
            "IDC_BUTTON_SWAIT_READY",
            "IDC_IMAGE_SWAIT_ALBUMMODE",
        ] {
            for master in [false, true] {
                fold_pin_word(
                    &mut hash,
                    u64::from(waiting_control_armed_for_role(id, master)),
                );
            }
        }

        let mut flat = vec![0u32; 800 * 600];
        paint_waiting_members(
            &mut Frame {
                buf: &mut flat,
                w: 800,
                h: 600,
            },
            &room,
        );
        fold_pin_pixels(&mut hash, &flat);

        let art = waiting_golden_art();
        let captions = WaitingCaptions {
            song: Some(WaitingSong {
                music_id: 757,
                title: Some("Painted Title"),
            }),
            song_bpm: Some(168.4),
            skill_counts: [2101, 42, 1500],
            level: Some(WaitingLevel {
                difficulty: 2,
                speed: 5,
                ex_chart_level: None,
            }),
            notice: None,
            ..Default::default()
        };
        let mut staged = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut staged,
                w: 800,
                h: 600,
            },
            &room,
            &art,
            &captions,
            false,
            false,
            MemberRole::Spectator,
        );
        fold_pin_pixels(&mut hash, &staged);

        let noticed = WaitingCaptions {
            notice: Some("MAINTENANCE"),
            ..captions
        };
        let mut armed = vec![0u32; 800 * 600];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut armed,
                w: 800,
                h: 600,
            },
            &champion,
            &art,
            &noticed,
            true,
            true,
            MemberRole::Champion,
        );
        fold_pin_pixels(&mut hash, &armed);

        let mut scaled = vec![0u32; 400 * 300];
        paint_waiting_scene(
            &mut Frame {
                buf: &mut scaled,
                w: 400,
                h: 300,
            },
            &champion,
            &art,
            &captions,
            false,
            false,
            MemberRole::Challenger,
        );
        fold_pin_pixels(&mut hash, &scaled);

        let figures = [
            Some(solid_figure(4, 4, [0x00, 0x7C])),
            None,
            Some(solid_figure(4, 4, [0x1F, 0x00])),
        ];
        let mut avatars = vec![0u32; 800 * 600];
        paint_waiting_seat_avatars(
            &mut Frame {
                buf: &mut avatars,
                w: 800,
                h: 600,
            },
            &art.placements,
            &champion,
            &figures,
            1_234,
        );
        fold_pin_pixels(&mut hash, &avatars);

        // #372 re-pinned after the exe-matched seat-index mapping:
        // peer slots keep their `slot.index` cover and the local member
        // sits at `member_id`; header class chips stay unpainted.
        // #535 re-cut: `SPEED_NUM` now inks caption level's pick-or-room
        // speed instead of the room's own speed, so the four folded
        // frames differ at that rect alone. #883 re-pins the seat line
        // to the exe's `Lv.<n>:<name>` form (captain/ready words removed)
        // and #884 drops zero skill counts from the baked X plates.
        // #883 second pass also leaves the arena-0 MAPNAME cell at the
        // sheet's baked `Random`, inks BPM bare, and keeps the bright
        // speed-arrow record. Named guards + quiesced pairs carry it.
        assert_eq!(
            hash, 0x283B_C1B8_F2A8_B058,
            "waiting paint generation changed; re-check the exe pins before updating"
        );
    }

    /// Ticket #838: after the boot-wave3 refutation, only S2C `4035`
    /// arms the local seat's DOWNLOADING badge and bar. The full A/B
    /// pixel pair runs through `paint_at` in
    /// `game_tests::acquire_music_4035_paints_the_local_seat_badge_and_bar`
    /// and `game_tests::music_state_4037_and_progress_4039_leave_the_seat_chrome_dark`.
    /// Live chain: session frame ingest -> `OnlineSession::apply_acquire_music`
    /// -> `answer_acquire_music` -> `ScreenGame::arm_local_download_badge`
    /// -> `paint_at` -> `paint_waiting_scene` -> `staged_seats` ->
    /// `paint_waiting_seat_marks`. Rect origins are the parsed Waiting
    /// seat-template `.bnd` records (EXTEND1 `(48,127)` / EXTEND2
    /// `(190,127)`, bar y163), not painter literals; the figures are
    /// synthetic so CI runs without art.
    #[test]
    fn acquire_music_4035_paints_only_its_own_bnd_badge_and_bar() {
        let mut room = waiting_room();
        let mut peer = RoomMember {
            nickname: "peer".to_owned(),
            ..room.slots[0].member.clone().unwrap()
        };
        peer.master = false;
        peer.music_state = 4;
        room.slots.push(RoomSlot {
            index: 1,
            state_code: 1,
            member: Some(peer),
        });
        let seats = staged_seats(&room, None, true);
        assert!(
            seats[0].download_badge_armed,
            "4035 arms the local seat badge"
        );
        let unarmed = staged_seats(&room, None, false);
        assert!(
            !unarmed[0].download_badge_armed,
            "no 4035 arm leaves the seat badge dark"
        );

        let placements = [
            (
                "IDC_BUTTON_SWAIT_EXTEND1",
                48,
                127,
                100,
                45,
                "Wait_Downloding.ojs",
            ),
            (
                "IDC_BUTTON_SWAIT_EXTEND2",
                190,
                127,
                100,
                45,
                "Wait_Downloding.ojs",
            ),
            (
                "IDC_BUTTON_SWAIT_EXTEND1",
                66,
                163,
                65,
                7,
                "Wait_Downloding_Bar.ojs",
            ),
            (
                "IDC_BUTTON_SWAIT_EXTEND2",
                208,
                163,
                65,
                7,
                "Wait_Downloding_Bar.ojs",
            ),
        ]
        .into_iter()
        .map(|(id, x, y, w, h, sprite)| ScenePlacement {
            role: PlacementRole::Clone,
            id: id.to_owned(),
            code: 0,
            sprite: Some(sprite.to_owned()),
            rect: Rect { x, y, w, h },
        })
        .collect();
        let art = SceneArt {
            placements,
            figures: HashMap::from([
                (
                    "Wait_Downloding.ojs".to_owned(),
                    solid_figure(20, 10, [0x00, 0x7C]),
                ),
                (
                    "Wait_Downloding_Bar.ojs".to_owned(),
                    solid_figure(20, 7, [0xE0, 0x03]),
                ),
            ]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_seat_marks(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &seats,
            &art,
        );
        assert_eq!(buf[129 * 800 + 50], pack_xrgb(0xFF, 0, 0), "local badge");
        assert_eq!(buf[165 * 800 + 68], pack_xrgb(0, 0xFF, 0), "local bar");
        assert_eq!(buf[129 * 800 + 192], 0, "peer badge stays dark");
        assert_eq!(buf[165 * 800 + 210], 0, "peer bar stays dark");
    }

    /// Ticket #842: the exe paints this 123x16 plate band at
    /// x=179..301, y=211..226, while an uncorrected clone paints it at
    /// y=189..204 (paired frames `p559-badge-ab4` and `av-sweep/waiting`).
    /// Live chain: `main.rs::run_play` -> `setup_online` ->
    /// `OnlineSession::ingest_snapshot` -> `ScreenGame::paint_at` ->
    /// `paint_waiting_scene_with_confirm` -> `paint_waiting_seat_marks`.
    /// Synthetic art pins the band without reading archives.
    #[test]
    fn seat_two_team_plate_paints_at_the_exe_band_anchor() {
        const PLATE_X: i32 = 179;
        const PLATE_Y: i32 = 189;
        const PLATE_W: u32 = 123;
        const EXE_BAND_TOP: usize = 211;
        const EXE_BAND_BOTTOM: usize = 226;

        let mut room = waiting_room();
        let mut peer = room.slots[0].member.clone().expect("host seat");
        peer.master = false;
        room.slots[1].member = Some(peer);
        let seats = staged_seats(&room, None, false);
        let art = SceneArt {
            placements: vec![ScenePlacement {
                role: PlacementRole::Clone,
                id: WAITING_SEAT_PARENT_IDS[1].to_owned(),
                code: 0x1650_002C,
                sprite: Some("Wait_ReadyWindow.ojs".to_owned()),
                rect: Rect {
                    x: PLATE_X,
                    y: PLATE_Y,
                    w: PLATE_W,
                    h: 16,
                },
            }],
            figures: HashMap::from([(
                "Wait_ReadyWindow.ojs".to_owned(),
                solid_figure(123, 16, [0x00, 0x7C]),
            )]),
        };
        let mut buf = vec![0u32; 800 * 600];
        paint_waiting_seat_marks(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &seats,
            &art,
        );

        let ink = pack_xrgb(0xFF, 0, 0);
        assert_eq!(
            buf[EXE_BAND_TOP * 800 + usize::try_from(PLATE_X).expect("plate x")],
            ink,
            "exe band top edge must carry the plate"
        );
        assert_eq!(
            buf[EXE_BAND_BOTTOM * 800
                + usize::try_from(PLATE_X).expect("plate x")
                + usize::try_from(PLATE_W - 1).expect("plate width")],
            ink,
            "exe band right edge must carry the plate"
        );
        assert_eq!(
            buf[(EXE_BAND_TOP - 1) * 800 + usize::try_from(PLATE_X).expect("plate x")],
            0,
            "row above the exe band stays dark"
        );
        assert_eq!(
            buf[usize::try_from(PLATE_Y).expect("staged y") * 800
                + usize::try_from(PLATE_X).expect("plate x")],
            0,
            "old 22 px high band stays dark"
        );
    }
}

#[cfg(test)]
mod dialog_paint_tests {
    //! Dialog paint guards (Refs: #131, #132): painted art and
    //! [`dialog_rect_hit`] must answer the same dialog-relative
    //! rect, so a click always lands on visible art. The pairing
    //! (`parse_bnd` + code-ordinal) is unchanged — the rule under
    //! guard is the per-record placement: a record anchor when the
    //! record carries one, the placement's `.bnd` rect origin when
    //! the record anchors at `(0,0)`, one frame per distinct anchor.

    use super::*;
    use crate::avatar::{AvatarFigure, FigureLayer, PlacementRole, ScenePlacement};
    use crate::staging::{stage_dialog, Dialog, StageContext};
    use o2jam_render::{blit_rgb555, Frame, Rgb555Image};

    /// One solid bound-body figure in a contrasting RGB555 colour.
    fn filled_figure(w: usize, h: usize, word: [u8; 2]) -> AvatarFigure {
        let mut pixels = vec![0u8; w * h * 2];
        for pair in pixels.as_chunks_mut::<2>().0 {
            pair.copy_from_slice(&word);
        }
        let mut figure = AvatarFigure::default();
        figure.push_layer(FigureLayer {
            image: Rgb555Image::decode(&pixels, w, h).unwrap(),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        figure
    }

    /// One 1x1 red pixel layer (colorkey white, so the pixel paints).
    fn red_layer(dx: i32, dy: i32) -> FigureLayer {
        FigureLayer {
            image: Rgb555Image::decode(&[0x00, 0x7C], 1, 1).unwrap(),
            dx,
            dy,
            colorkey: 0xFFFF,
        }
    }

    fn placement(id: &str, code: u32, sprite: &str, rect: Rect) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: id.to_owned(),
            code,
            sprite: Some(sprite.to_owned()),
            rect,
        }
    }

    /// Ink bounds of one painted frame (`None` when no pixel painted).
    fn ink_bbox(buf: &[u32], w: usize) -> Option<(i32, i32, i32, i32)> {
        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for (at, word) in buf.iter().enumerate() {
            if *word == 0 {
                continue;
            }
            let (x, y) = ((at % w) as i32, (at / w) as i32);
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x1, y1, x2, y2)) => (x1.min(x), y1.min(y), x2.max(x), y2.max(y)),
            });
        }
        bounds
    }

    #[test]
    fn zero_anchor_children_paint_at_their_bnd_rects() {
        // Synthetic, archive-independent: a child whose records all
        // anchor at (0,0) carries no position of its own — the `.bnd`
        // rect is its only placement data. Its ink must land at
        // `dialog_origin + rect` (the exact rect `dialog_rect_hit`
        // answers), and `(0,0)`-anchored records are frames of one
        // view, so only the first paints — never a stack at the
        // dialog origin (the #132 top-left pile).
        let mut backdrop = AvatarFigure::default();
        backdrop.push_layer(red_layer(240, 189));
        let mut glyph = AvatarFigure::default();
        for _ in 0..3 {
            glyph.push_layer(red_layer(0, 0));
        }
        let placements = vec![
            placement(
                "IDC_BACK",
                1,
                "Dialog_Synthetic.ojs",
                Rect {
                    x: 240,
                    y: 189,
                    w: 320,
                    h: 221,
                },
            ),
            placement(
                "IDC_GLYPH",
                2,
                "Glyph.ojs",
                Rect {
                    x: 52,
                    y: 181,
                    w: 107,
                    h: 26,
                },
            ),
        ];
        let art = SceneArt {
            placements,
            figures: HashMap::from([
                ("Dialog_Synthetic.ojs".to_owned(), backdrop),
                ("Glyph.ojs".to_owned(), glyph),
            ]),
        };
        let origin = dialog_origin(&art.placements);
        assert_eq!(origin, (240, 189), "the backdrop rect authors the origin");
        assert!(
            dialog_rect_hit(&art.placements, 2, 292, 370),
            "hit and paint must agree on the dialog-relative rect"
        );
        let mut buf = vec![0u32; 800 * 600];
        paint_dialog_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        // Backdrop 1px at its anchor + exactly one glyph frame at
        // origin + (52,181) = (292,370); the two other frames stay unpainted.
        assert_eq!(
            buf.iter().filter(|word| **word != 0).count(),
            2,
            "one backdrop pixel plus one glyph frame pixel"
        );
        assert_eq!(ink_bbox(&buf, 800), Some((240, 189, 292, 370)));
    }

    /// Read the pinned Interface archive, `None` without install-root.
    fn interface_archive() -> Option<Vec<u8>> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../install-root/O2Jam/Image/Interface1.opi"),
        )
        .ok()
    }

    #[test]
    fn option_sheets_paint_on_their_hit_rects() {
        // Real 7.55 bytes (skips without install-root): every child of
        // all three `Dialog_Option_*` sheets paints its first ink at
        // `dialog_origin + (record anchor, else the .bnd rect origin)`
        // — the same rect `dialog_rect_hit` answers — so every tab's
        // art and hit tests land together (#131 inert controls, #132
        // stacked sprites both traced here).
        let Some(archive) = interface_archive() else {
            return;
        };
        let ctx = StageContext::default();
        let mut checked = 0usize;
        for dialog in [
            Dialog::PlanetOption,
            Dialog::PlanetOptionSound,
            Dialog::PlanetOptionGame,
        ] {
            let art = stage_dialog(&archive, dialog, ctx)
                .art
                .expect("option sheet stages with real bytes");
            let origin = dialog_origin(&art.placements);
            for placement in &art.placements {
                // The backdrop paints view-`(0,0)` at its absolute
                // anchor (its anchor *is* the dialog origin).
                if placement
                    .sprite
                    .as_deref()
                    .is_some_and(|sprite| sprite.starts_with("Dialog_"))
                {
                    continue;
                }
                let Some(name) = placement.sprite.as_ref() else {
                    continue;
                };
                let Some(figure) = art.figures.get(name) else {
                    continue;
                };
                if figure.is_empty() {
                    continue;
                }
                let mut buf = vec![0u32; 800 * 600];
                paint_dialog_figure(
                    &mut Frame {
                        buf: &mut buf,
                        w: 800,
                        h: 600,
                    },
                    placement,
                    figure,
                    origin,
                );
                let Some((x1, y1, _, _)) = ink_bbox(&buf, 800) else {
                    panic!("{} paints no ink", placement.id);
                };
                if figure.has_anchor_offset() {
                    // Anchored records author their own position: the
                    // painted buffer must equal every distinct anchor's
                    // first frame blitted at `origin + anchor`.
                    let mut expected = vec![0u32; 800 * 600];
                    let mut anchors: Vec<(i32, i32)> = Vec::new();
                    for index in 0..figure.len() {
                        let Some(layer) = figure.layer(index) else {
                            continue;
                        };
                        let at = (layer.dx, layer.dy);
                        if anchors.contains(&at) {
                            continue;
                        }
                        anchors.push(at);
                        let (view_x, view_y) =
                            anchor_in_frame(origin.0 + at.0, origin.1 + at.1, 800, 600);
                        blit_rgb555(
                            &mut Frame {
                                buf: &mut expected,
                                w: 800,
                                h: 600,
                            },
                            &layer.image.view(),
                            view_x,
                            view_y,
                            Some(layer.colorkey),
                        );
                    }
                    assert_eq!(
                        buf, expected,
                        "{} ({}) paints its anchored frames dialog-relative",
                        placement.id, name
                    );
                } else {
                    // `(0,0)`-anchored records carry no position: the
                    // `.bnd` rect is their only placement data, so the
                    // first ink must sit on `origin + rect` — the same
                    // rect `dialog_rect_hit` answers.
                    let expected = (origin.0 + placement.rect.x, origin.1 + placement.rect.y);
                    assert!(
                        (x1 - expected.0).abs() <= 2 && (y1 - expected.1).abs() <= 2,
                        "{} ({}) first ink at ({x1},{y1}), expected {expected:?}",
                        placement.id,
                        name
                    );
                }
                checked += 1;
            }
        }
        assert!(checked >= 60, "all three sheets guard, got {checked}");
    }

    #[test]
    fn keytext_sheet_paints_one_frame_per_key_button() {
        // Real 7.55 bytes (skips without install-root):
        // `OptionKey_Keytext.ojs` decodes 37 `(0,0)`-anchored glyph
        // records (the key-letter font). Painting a lane button keeps
        // exactly one frame — the loader default frame 0 — at its
        // `.bnd` rect; the #132 regression stacked all 37 records at
        // the dialog origin.
        let Some(archive) = interface_archive() else {
            return;
        };
        let art = stage_dialog(&archive, Dialog::PlanetOption, StageContext::default())
            .art
            .expect("key sheet stages with real bytes");
        let origin = dialog_origin(&art.placements);
        let placement = art
            .placements
            .iter()
            .find(|placement| placement.code == super::super::side::OPTION_KEY_LANE_FIRST)
            .expect("KEY1 button stages");
        let figure = art
            .figures
            .get(placement.sprite.as_ref().expect("KEY1 names a sprite"))
            .expect("KEY1 figure loads");
        assert!(
            figure.len() > 1,
            "the guard needs the multi-record glyph sheet"
        );
        let mut painted = vec![0u32; 800 * 600];
        paint_dialog_figure(
            &mut Frame {
                buf: &mut painted,
                w: 800,
                h: 600,
            },
            placement,
            figure,
            origin,
        );
        let mut frame0 = vec![0u32; 800 * 600];
        let layer = figure.layer(0).expect("glyph frame 0 decodes");
        let (view_x, view_y) = anchor_in_frame(
            origin.0 + placement.rect.x,
            origin.1 + placement.rect.y,
            800,
            600,
        );
        blit_rgb555(
            &mut Frame {
                buf: &mut frame0,
                w: 800,
                h: 600,
            },
            &layer.image.view(),
            view_x,
            view_y,
            Some(layer.colorkey),
        );
        assert_eq!(
            painted, frame0,
            "a lane button paints exactly its first glyph frame"
        );
    }

    /// One dialog sheet with a contrasting body over a baked plate.
    /// The backdrop is a `Dialog_*` sprite (view origin), and the body
    /// uses the bound-body `(0,0)` record so the `.bnd` rect owns it.
    fn baked_plate_scene(sprite: &str, code: u32) -> SceneArt {
        let backdrop = filled_figure(100, 100, [0x00, 0x7C]);
        let body = filled_figure(20, 10, [0x1F, 0x00]);
        SceneArt {
            placements: vec![
                ScenePlacement {
                    id: "sheet".to_owned(),
                    code: 0x2110_0001,
                    sprite: Some("Dialog_Test.ojs".to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 100,
                        h: 100,
                    },
                    role: PlacementRole::Single,
                },
                ScenePlacement {
                    id: "body".to_owned(),
                    code,
                    sprite: Some(sprite.to_owned()),
                    rect: Rect {
                        x: 10,
                        y: 10,
                        w: 20,
                        h: 10,
                    },
                    role: PlacementRole::Single,
                },
            ],
            figures: HashMap::from([
                ("Dialog_Test.ojs".to_owned(), backdrop),
                (sprite.to_owned(), body),
            ]),
        }
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn dialog_rest_leaves_baked_button_plates_visible() {
        // #885/#845: the 7.55 rest frames show the OK and Options
        // button plates baked into their sheets, not the blue bodies
        // our loader stages at record 0 (`~/scratch/paired-shots-v3`
        // channel-popup + options-key diffs). Hover still repaints
        // the body through the frame-state pass.
        let red = pack_xrgb(255, 0, 0);
        for (sprite, code) in [
            ("Info_OK.ojs", 0x2120_0002),
            ("OptionKey_Btn_3Key.ojs", 0x2520_1003),
            ("Option_Default.ojs", 0x2520_0015),
        ] {
            let art = baked_plate_scene(sprite, code);
            let mut buf = vec![0u32; 800 * 600];
            paint_dialog_scene(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &art,
            );
            assert_eq!(
                buf[15 * 800 + 15],
                red,
                "{sprite} must leave its sheet's baked plate visible at rest"
            );
        }
    }

    #[test]
    fn dialog_rest_stages_controls_without_a_baked_plate() {
        // The suppression is per staged body, not a blanket hide of
        // every dialog control: another body still paints at rest.
        let art = baked_plate_scene("Other_Button.ojs", 0x2120_0099);
        let mut buf = vec![0u32; 800 * 600];
        paint_dialog_scene(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &art,
        );
        assert_eq!(
            buf[15 * 800 + 15],
            pack_xrgb(0, 0, 255),
            "an unbaked body keeps its staged frame"
        );
    }

    /// MESSAGE-sheet stand-in: a baked gray plate and a two-record
    /// button body. The records stand in for `Question1_OK.ojs` blue
    /// frames; no installed art is needed to test the repaint seam.
    fn baked_message_button_scene() -> SceneArt {
        let mut body = filled_figure(20, 10, [0x1F, 0x00]);
        body.push_layer(FigureLayer {
            image: Rgb555Image::decode(&[0xE0, 0x03].repeat(20 * 10), 20, 10)
                .expect("message press test frame"),
            dx: 0,
            dy: 0,
            colorkey: 0xFFFF,
        });
        SceneArt {
            placements: vec![
                ScenePlacement {
                    id: "sheet".to_owned(),
                    code: 0x2210_0001,
                    sprite: Some("Dialog_Test.ojs".to_owned()),
                    rect: Rect {
                        x: 0,
                        y: 0,
                        w: 100,
                        h: 100,
                    },
                    role: PlacementRole::Single,
                },
                ScenePlacement {
                    id: "ok".to_owned(),
                    code: super::super::side::QUIT_OK_CODE,
                    sprite: Some("Question1_OK.ojs".to_owned()),
                    rect: Rect {
                        x: 10,
                        y: 10,
                        w: 20,
                        h: 10,
                    },
                    role: PlacementRole::Single,
                },
            ],
            figures: HashMap::from([
                (
                    "Dialog_Test.ojs".to_owned(),
                    filled_figure(100, 100, [0x10, 0x42]),
                ),
                ("Question1_OK.ojs".to_owned(), body),
            ]),
        }
    }

    #[test]
    fn message_button_hover_and_press_records_repaint_over_the_baked_plate() {
        // Live caller chain: boot stages `Dialog_Question1` ->
        // `click_shop` -> `ShopAction::CartBuy` ->
        // `shop_charge_confirm_open` -> `paint_shop_charge_confirm` ->
        // `ScreenGame::paint_dialog_frame_states` ->
        // `paint_dialog_frame_record`. The state choice is tested in
        // `dialog_frame.rs`; this seam pins the record repaint on the
        // body the rest pass now withholds (#938).
        let art = baked_message_button_scene();
        let mut buf = vec![0u32; 800 * 600];
        let mut frame = Frame {
            buf: &mut buf,
            w: 800,
            h: 600,
        };
        let at = 15 * 800 + 15;
        paint_dialog_scene(&mut frame, &art);
        assert_eq!(
            frame.buf[at],
            pack_xrgb(132, 132, 132),
            "rest keeps the baked gray plate"
        );
        paint_dialog_frame_record(&mut frame, &art, super::super::side::QUIT_OK_CODE, 0);
        assert_eq!(
            frame.buf[at],
            pack_xrgb(0, 0, 255),
            "hover repaints the withheld body's record 0"
        );
        paint_dialog_frame_record(&mut frame, &art, super::super::side::QUIT_OK_CODE, 1);
        assert_eq!(
            frame.buf[at],
            pack_xrgb(0, 255, 0),
            "press repaints the withheld body's record 1"
        );
    }

    /// The relay notice's exe string stores three explicit line
    /// breaks. Every line must land inside the staged TEXT rect,
    /// even when no dialog art is installed (#882).
    #[test]
    fn relay_down_notice_paints_ink_in_each_message_row() {
        let mut buf = vec![0u32; 800 * 600];
        paint_information_message(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            None,
            RELAY_DOWN_MESSAGE_CP949,
        );
        let (message, _, _) = quit_fallback_rects();
        let paints_white_row = |row: i32| -> bool {
            let row = usize::try_from(row).unwrap();
            (message.x..message.x + i32::try_from(message.w).unwrap()).any(|col| {
                let at = row * 800 + usize::try_from(col).unwrap();
                buf[at] != scrim_color()
            })
        };
        let third = i32::try_from(message.h / 3).unwrap();
        for (line, start) in [
            ("first", message.y),
            ("second", message.y + third),
            ("third", message.y + 2 * third),
        ] {
            assert!(
                (start..start + third).any(paints_white_row),
                "{line} notice line must paint inside the message rect"
            );
        }
    }
}
