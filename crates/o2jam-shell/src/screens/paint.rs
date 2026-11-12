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

use super::super::avatar::{
    anchor_in_frame, clone_instance_live, paint_behind_placements, paint_foreground_placements,
    paint_placement_layer, paint_scene_figures, paint_scene_figures_live, placement_is_behind,
    scene_block, scene_sprite_names, shared_sprite_names, AvatarFigure, FigureLayer, PlacementRole,
    ScenePlacement, AVI_BLOCK, CHANNEL_BLOCK, LANKING_BLOCK, LOADING_BLOCK, LOADING_COVER_SPRITE,
    LOADING_SPRITE, MUSICSHOP_BLOCK, MYROOM_BLOCK, PLANET_BLOCK, RESULT_BLOCK, ROOM_BLOCK,
    SHOP_BLOCK, WAITING_BLOCK, WAIT_COVER_SPRITE, WAIT_TEAM_SPRITES,
};
use super::super::playing::{HighwayGeometry, PlayingView};
use super::super::screen::Screen;
use super::side::{
    lanking_bpm_caption, lanking_diff_caption, lanking_genre_caption, lanking_lview_slots,
    lanking_rank_cells, lanking_rview_slots, lanking_tiled_rect, music_shop_buy_caption,
    music_shop_cart_page_rects, music_shop_cart_slots, music_shop_page_rects, music_shop_slots,
    music_shop_tiled_rect, myroom_action_for_code, planet_action_for_code, planet_band_rect,
    planet_caption_for_code, planet_plate_clip, shop_cart_page_rects, shop_cart_slots,
    wallet_row_caption, LankingSong, MusicShopRow, MyRoomAction, MyRoomState, PlanetAction,
    PlanetBand, PlanetOptionTab, PlanetPromo, ShopGood, WalletRow, WalletState, GEM_LABEL,
    LANKING_LVIEW_TEXT_CODES, LANKING_MUSICNAME_CODE, LANKING_RVIEW_TEXT_CODES,
    MSHOP_CART_ITEMNAME_CODE, MSHOP_CART_NUM_CODE, MSHOP_CART_ROWS, MSHOP_ROW_TEXT_CODES,
    MYROOM_BAG_FIRST_CODE, MYROOM_GIFT_FIRST_CODE, MYROOM_SLOT_FIRST_CODE, PLANET_PROMO_IMAGE_CODE,
    QUIT_MESSAGE_CP949, SHOP_CART_ITEMNAME_CODE, SHOP_CART_NUM_CODE, SHOP_CART_PRICE_CODE,
    SHOP_CART_ROWS, SHOP_GEMMONEY_CODE, SHOP_ITEMNAME_FIRST_CODE, SHOP_ITEMWON_FIRST_CODE,
    SHOP_ITEM_ROWS,
};
use o2jam_file::ControlList;
use o2jam_game::RoomTeams;
use o2jam_net::{
    rooms::om_icon_frame, ChannelState, CharacterInfo, Difficulty, GameMode, GameSpeed, JoinedRoom,
    KeyMode, RankEntry, RoomListEntry, RoomSlot, RoomSlotState,
};
use o2jam_render::font::{draw_text_cell, text_width_cell, Cell};
use o2jam_render::{
    blit_rgb555, draw_number, draw_text, fill_rect, number_width, pack_xrgb, scale_rect_to_frame,
    text_width, Frame, Rect, Rgb555Image, NATIVE_HEIGHT, NATIVE_WIDTH,
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
/// them answers nothing (drag stays OPEN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelScroll {
    /// Up arrow: step toward the first row.
    Up,
    /// Down arrow: step toward the last row.
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

/// Filled width of the count strip in native px: the live population
/// share of capacity, clamped to the strip. Zero capacity or zero
/// population fills nothing. Segment quantization stays OPEN: live
/// captures pin one lit segment without pinning the rounding rule,
/// so this floor rule is the labeled stand-in, not exe proof.
fn channel_fill_width(channel: &ChannelState) -> usize {
    if channel.capacity <= 0 || channel.population <= 0 {
        return 0;
    }
    let filled = CHANNEL_COUNT_W * i64::from(channel.population) / i64::from(channel.capacity);
    usize::try_from(filled.clamp(0, CHANNEL_COUNT_W)).unwrap_or(0)
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
    true
}

/// Staged scroll bodies riding the bnd track (573,213): the up/down
/// arrows at scroll-local (0,0)/(0,196) with the bar body between
/// them. Each part paints only while its own figure staged.
fn paint_channel_scroll_art(frame: &mut Frame<'_>, art: &SceneArt) {
    if frame.w == 0 || frame.h == 0 {
        return;
    }
    let track = &CHANNEL_SCROLL_TRACK;
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
    paint_part(frame, CHANNEL_SCROLL_UP_SPRITE, CHANNEL_SCROLL_UP_LOCAL);
    paint_part(
        frame,
        CHANNEL_SCROLL_BAR_SPRITE,
        (0, CHANNEL_SCROLL_ARROW_H),
    );
    paint_part(frame, CHANNEL_SCROLL_DOWN_SPRITE, CHANNEL_SCROLL_DOWN_LOCAL);
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

/// Per-row enter face after the bar: staged [`CHANNEL_ENTER_SPRITE`]
/// frame 0 for joinable rows, nothing for FULL rows (the baked FULL
/// bar covers the well), and the procedural plate where no enter art
/// staged. Faces ride above the bar because the bar frame's face
/// well is opaque art (decoded `CH_Back` frame 0 carries a gray
/// well, not a cutout).
fn paint_channel_face(frame: &mut Frame<'_>, slot: usize, channel: &ChannelState, art: &SceneArt) {
    let w = i32::try_from(frame.w).unwrap_or(0);
    let h = i32::try_from(frame.h).unwrap_or(0);
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

/// Visible Channel slice for `highlight`: viewport top plus shown
/// count (the highlight always lands inside). Single source shared by
/// paint and click mapping. NO PAGER: the 7.55 channel dialog has no
/// pager readout or arrows (bnd has no rects; sheet art bottom is rim
/// chrome + scroll down-arrow, decoded 2026-11-10; the block's
/// LEFT/RIGHT buttons stage zero rects and are statically unreachable).
#[must_use]
pub(crate) fn channel_viewport(highlight: usize, len: usize) -> (usize, usize) {
    let top = viewport_top(highlight, len);
    (top, len.saturating_sub(top).min(CHANNEL_VISIBLE_ROWS))
}
#[must_use]
pub(crate) fn viewport_top(highlight: usize, len: usize) -> usize {
    let max_top = len.saturating_sub(CHANNEL_VISIBLE_ROWS);
    highlight
        .saturating_sub(CHANNEL_VISIBLE_ROWS - 1)
        .min(max_top)
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

/// Avatar room: staged bound art, flat card fallback.
/// Slot images name no bodies and captions join nowhere, so figures
/// alone paint — no slot icons, captions, or selection state.
pub fn paint_myroom_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Myroom);
}

/// Avatar-room rows and captions over staged placements: bag names
/// into the `MYBAGITEM` rects, gift names into the `GIFTBOX_EX`
/// rects, identity/gift strings into their TEXT rects, and the
/// amber stand-in cursor at the selected slot, bag cell, and gift
/// cell (highlight rendering is unevidenced — the cursor is an OPEN
/// stand-in like the planet band cursor, never exe parity).
/// Everything rides staged `.bnd` rects in file order; without
/// staged rows nothing paints (no fallback geometry is invented).
#[allow(clippy::too_many_arguments)]
pub fn paint_myroom_data(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    state: MyRoomState,
    bag_names: &[String],
    gift_names: &[String],
    captions: &[(u32, String)],
) {
    for (code, text) in captions {
        if text.is_empty() {
            continue;
        }
        if let Some(placed) = placements.iter().find(|placed| placed.code == *code) {
            paint_text_into_rect(frame, placed.rect, text, CP949_INK);
        }
    }
    paint_myroom_cells(frame, placements, bag_names, MYROOM_BAG_FIRST_CODE);
    paint_myroom_cells(frame, placements, gift_names, MYROOM_GIFT_FIRST_CODE);
    if let Some(placed) =
        myroom_family_placement(placements, MYROOM_SLOT_FIRST_CODE, state.selected_slot())
    {
        paint_cursor(frame, placed.rect.x + 10, myroom_rect_mid_y(&placed.rect));
    }
    if let Some(cell) = state.bag_cell() {
        if let Some(placed) = myroom_cell_placement(placements, MYROOM_BAG_FIRST_CODE, cell) {
            paint_cursor(frame, placed.rect.x + 10, myroom_rect_mid_y(&placed.rect));
        }
    }
    if let Some(cell) = state.gift_cell() {
        if let Some(placed) = myroom_cell_placement(placements, MYROOM_GIFT_FIRST_CODE, cell) {
            paint_cursor(frame, placed.rect.x + 10, myroom_rect_mid_y(&placed.rect));
        }
    }
}

/// Vertical middle of one staged rect (native pixels): the amber
/// stand-in cursor's anchor, matching the planet band cursor rule.
fn myroom_rect_mid_y(rect: &Rect) -> i32 {
    let half = i32::try_from(rect.h).unwrap_or(0) / 2;
    rect.y + half
}

/// One family member by ordinal within its code run: the staged
/// placement whose code is `first + index` (slot buttons) — `None`
/// past the run or when the rect never staged.
fn myroom_family_placement(
    placements: &[ScenePlacement],
    first: u32,
    index: u8,
) -> Option<&ScenePlacement> {
    let code = first.checked_add(u32::from(index))?;
    placements.iter().find(|placed| placed.code == code)
}

/// One row cell by position within its staged family run (bag and
/// gift cells count in staged file order, the same order the click
/// dispatch counts — painter and clicker share the cells).
fn myroom_cell_placement(
    placements: &[ScenePlacement],
    first: u32,
    index: usize,
) -> Option<&ScenePlacement> {
    let mut seen = 0usize;
    for placed in placements {
        let is_cell = myroom_action_for_code(placed.code).is_some_and(|action| match action {
            MyRoomAction::PickBagCell(_) => first == MYROOM_BAG_FIRST_CODE,
            MyRoomAction::PickGiftCell(_) => first == MYROOM_GIFT_FIRST_CODE,
            _ => false,
        });
        if !is_cell {
            continue;
        }
        if seen == index {
            return Some(placed);
        }
        seen += 1;
    }
    None
}

/// Row strings into one staged cell family in file order: the
/// `n`th string paints the `n`th staged cell rect. Extra strings
/// paint nowhere; missing strings leave later cells blank.
fn paint_myroom_cells(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    names: &[String],
    first: u32,
) {
    let cells: Vec<&ScenePlacement> = {
        let mut cells = Vec::new();
        let mut index = 0usize;
        while let Some(placed) = myroom_cell_placement(placements, first, index) {
            cells.push(placed);
            index += 1;
        }
        cells
    };
    for (placed, name) in cells.iter().zip(names.iter()) {
        if !name.is_empty() {
            paint_text_into_rect(frame, placed.rect, name, CP949_INK);
        }
    }
}

/// Rankings: staged art behind an empty list-view, flat card
/// fallback. Rank rows need the fetch opcode (OPEN) — no data, no
/// sorts, no selection state.
pub fn paint_lanking_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    art.paint_backdrop(frame, Screen::Lanking);
}

/// Billing-unwired notice over the Payment side screen: the notice
/// inks the largest staged placement rect (geometry rides staged
/// `.bnd` rects, never literals), so the billing stall reads
/// instead of painting art alone. No staged rect paints nothing.
pub fn paint_payment_notice(frame: &mut Frame<'_>, placements: &[ScenePlacement], notice: &str) {
    let Some(rect) = placements
        .iter()
        .map(|placed| placed.rect)
        .max_by_key(|rect| rect.w.saturating_mul(rect.h))
    else {
        return;
    };
    paint_text_into_rect(frame, rect, notice, CP949_INK);
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

/// Item-shop board overlay over the staged side art: the visible
/// goods' names and cash prices into the staged `ITEMNAME`/`ITEMWON`
/// TEXT rects (gem cells stay blank — no pinned source), the wallet
/// gem cell, the checked cart rows, and both pager readouts — each
/// inked into staged rects, never literals. Text renders through
/// the system face (owner decision `e8d01`).
pub fn paint_shop_board(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    goods: &[ShopGood],
    wallet: &WalletState,
    cart: &[ShopGood],
    cart_page: usize,
    cart_pages: usize,
) {
    paint_shop_rows(frame, placements, goods);
    paint_shop_wallet(frame, placements, wallet);
    paint_shop_cart(frame, placements, cart, cart_page, cart_pages);
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
    }
}

/// Wallet gem cell (`GEMMONEY`): `--` until the 5028 refresh folds
/// a gem word.
fn paint_shop_wallet(frame: &mut Frame<'_>, placements: &[ScenePlacement], wallet: &WalletState) {
    if let Some(rect) = shop_template_rect(placements, SHOP_GEMMONEY_CODE) {
        let row = WalletRow::named(GEM_LABEL, wallet.gem);
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

/// Em extent fitting `text` into a `w`×`h` rect: the smaller of the
/// height cap and the width-fit scale (advances scale linearly, so
/// one measure at 1 px/em fixes both). The font-absent stand-in
/// measures through the same fit, so layout survives without the
/// system face.
fn cp949_fit_scale(text: &str, w: f32, h: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 || text.is_empty() {
        return h.max(0.0);
    }
    let width_per_em = match o2jam_text::system_font() {
        Some(font) => o2jam_text::measure_text(font, text, 1.0),
        None => o2jam_text::font::measure_text_stand_in(text, 1.0),
    };
    let width_fit = if width_per_em > 0.0 {
        w / width_per_em
    } else {
        h
    };
    h.min(width_fit).max(0.0)
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
    let scale = cp949_fit_scale(text, rect.w as f32, rect.h as f32);
    if scale <= 0.0 {
        return;
    }
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        0,
        0,
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

/// Emblem captions over the staged board: each `TEXT` placement
/// paints its runtime string into its own `.bnd` rect (ordinals
/// 6..10, `docs/planet.md`). No placement, no caption — geometry
/// comes from the parsed bind file only (owner rule `f38f3`).
fn paint_planet_captions(frame: &mut Frame<'_>, placements: &[ScenePlacement]) {
    for placement in placements {
        if let Some(caption) = planet_caption_for_code(placement.code) {
            paint_cp949_into_rect(frame, placement.rect, caption.cp949);
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
/// carousel banner (`promo_index`, 1-based — `0` keeps the rest
/// stack) repaints its selected frame over the backdrop (see
/// [`paint_planet_promo_frame`]).
pub fn paint_planet_scene(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    selection: Option<PlanetBand>,
    hover: Option<PlanetAction>,
    press: Option<PlanetAction>,
    promo_index: u32,
) {
    art.paint_backdrop(frame, Screen::Planet);
    paint_planet_promo_frame(frame, art, promo_index);
    paint_planet_hover(frame, art, hover);
    paint_planet_press(frame, art, press);
    paint_planet_captions(frame, &art.placements);
    if let Some(band) = selection {
        let rect = planet_band_rect(band);
        paint_cursor(frame, rect.x + 10, rect.y + rect.h as i32 / 2);
    }
}

/// Promotion-banner frame: the exe's image control paints one frame
/// per view (refresh arm VA `0x48e4b0` → `set_frame(index)`), so the
/// shell repaints the staged `IDC_IMAGE_SPLANET_PROMOTION` body's
/// selected layer over the backdrop's full stack. The frames share
/// one anchor and size (the shipped body carries three 701x333
/// records at (49,163)), so the selected frame covers the stack
/// exactly. Index 0 or an unstaged body paints nothing; an index past
/// the staged layers keeps the rest stack (the shell never invents
/// frames). Runs before hover/press so their repaints stay on top.
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
/// second record repaints its alternate record (layer 0 over the
/// stacked rest top — the same repaint arm the hover's full-colour
/// restore uses); a single-record control (the option gear, no
/// visible pressed art in the `geartest` crops) paints nothing, never
/// a stand-in. Emblems paint nothing either (emblem press == hover
/// pixel-identical, #111 arm 1 caveat; the orange ring is the
/// channel-dialog ACTIVE marker, not a pressed frame). Which raw
/// record the exe's pressed art is, per control, stays OPEN — next
/// step: the record-vs-capture match of the press crops against the
/// staged records (the `docs/hover.md` 2026-11-11 playstop method).
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
    // The pressed frame repaint: layer 0 (the staged alternate
    // record) full-colour over the rest top, confined to the
    // placement's own rect by the blit — the pinned repaint region.
    paint_placement_layer(frame, placement, &art.figures, 0, shares_body, None);
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
pub(crate) fn dialog_origin(placements: &[ScenePlacement]) -> (i32, i32) {
    placements
        .iter()
        .find(|placement| {
            placement
                .sprite
                .as_deref()
                .is_some_and(|sprite| sprite.starts_with("Dialog_"))
        })
        .map_or((0, 0), |placement| (placement.rect.x, placement.rect.y))
}

/// Paint one dialog placement at its dialog-relative origin: the
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
    let is_backdrop = placement
        .sprite
        .as_deref()
        .is_some_and(|sprite| sprite.starts_with("Dialog_"));
    if is_backdrop {
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
        let at = if layer.dx == 0 && layer.dy == 0 {
            (placement.rect.x + layer.dx, placement.rect.y + layer.dy)
        } else {
            (layer.dx, layer.dy)
        };
        if painted_anchors.contains(&at) {
            continue;
        }
        painted_anchors.push(at);
        let (view_x, view_y) = anchor_in_frame(origin.0 + at.0, origin.1 + at.1, frame.w, frame.h);
        blit_rgb555(
            frame,
            &layer.image.view(),
            view_x,
            view_y,
            Some(layer.colorkey),
        );
    }
}

/// Paint a staged dialog scene dialog-relative (options and
/// quit-confirm sheets; one back-to-front pass in file order).
/// Placements without a loaded figure paint nothing.
pub fn paint_dialog_scene(frame: &mut Frame<'_>, art: &SceneArt) {
    if !art.has_figures() {
        return;
    }
    let origin = dialog_origin(&art.placements);
    for placement in &art.placements {
        let Some(name) = placement.sprite.as_ref() else {
            continue;
        };
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
        .find(|placement| placement.code == code)
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

/// Quit-confirm popup over the bare board: the staged
/// `Dialog_Question1` sheet (dialog-relative) with the
/// capture-pinned message in its `TEXT` rect, or the frame-centered
/// fallback (message + OK/CANCEL words) without staged art.
/// OK quits, CANCEL closes (the game routes the clicks through
/// [`dialog_rect_hit`], fallback [`quit_fallback_rects`]).
pub fn paint_planet_quit(frame: &mut Frame<'_>, art: Option<&SceneArt>) {
    if let Some(art) = art.filter(|art| art.has_figures()) {
        paint_dialog_scene(frame, art);
        let origin = dialog_origin(&art.placements);
        for placement in &art.placements {
            if placement.code == super::side::QUIT_MESSAGE_CODE {
                paint_cp949_into_rect(
                    frame,
                    Rect {
                        x: placement.rect.x + origin.0,
                        y: placement.rect.y + origin.1,
                        w: placement.rect.w,
                        h: placement.rect.h,
                    },
                    QUIT_MESSAGE_CP949,
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
    let (message, ok, cancel) = quit_fallback_rects();
    paint_cp949_into_rect(frame, message, QUIT_MESSAGE_CP949);
    for (button, word) in [(ok, "OK"), (cancel, "CANCEL")] {
        let scaled = scale_rect_to_frame(&button, frame.w, frame.h);
        fill_rect(frame, &scaled, CHANNEL_ENTER_TONE);
        draw_text(frame, scaled.x + 8, scaled.y + 8, word, art_title_ink());
    }
}

/// Options dialog over the bare board: the staged
/// `Dialog_Option_*` sheet for `tab` (dialog-relative), or a
/// title-only fallback without staged art (tab choice and mount
/// stay OPEN — see [`PLANET_OPTION_FIRST_TAB`](super::side::PLANET_OPTION_FIRST_TAB)/// ). Escape (and the tab's `Option_Exit` button, when staged)
/// closes it.
pub fn paint_planet_options(frame: &mut Frame<'_>, art: Option<&SceneArt>, tab: PlanetOptionTab) {
    if let Some(art) = art.filter(|art| art.has_figures()) {
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
/// the flat fallback and the art scene). Only the viewport around the
/// highlight paints (no pager: the 7.55 dialog has none — see
/// [`channel_viewport`]). `art` selects
/// staged enter-button art over the procedural plates in the art path
/// (`None` is the flat fallback).
fn paint_channel_rows(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    highlight: usize,
    art: Option<&SceneArt>,
    hovered_row: Option<usize>,
) {
    let (top, shown) = channel_viewport(highlight, channels.len());
    paint_channel_scrim(frame, shown);
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
        paint_channel_scroll_art(frame, art);
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
    highlight: usize,
) {
    fill_background(frame, Screen::Channel);
    paint_title(frame, "CHANNEL");
    paint_channel_rows(frame, channels, highlight, None, None);
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
/// dialog stays byte-identical to the art-less frame.
pub fn paint_channel_scene(
    frame: &mut Frame<'_>,
    channels: &[ChannelState],
    highlight: usize,
    art: &SceneArt,
    board: Option<&SceneArt>,
    hovered_row: Option<usize>,
) {
    if !art.has_figures() && !board.is_some_and(|board| board.has_figures()) {
        frame.buf.fill(0);
    }
    if let Some(board) = board.filter(|board| board.has_figures()) {
        board.paint_backdrop(frame, Screen::Planet);
    }
    paint_channel_dialog_body(frame, art);
    art.paint_backdrop(frame, Screen::Channel);
    // An art-less channel scene has no label ink of its own, so the
    // shell title keeps it readable (the pinned no-fill test); staged
    // art never gets the stamp (#36).
    if !art.has_figures() && !board.is_some_and(|board| board.has_figures()) {
        paint_title_over_art(frame, Screen::Channel);
    }
    paint_channel_rows(frame, channels, highlight, Some(art), hovered_row);
}

/// One room row: `3 TITLE 5/8`, locked rooms suffixed `!`.
fn room_row_text(room: &RoomListEntry) -> String {
    let mut text = format!(
        "{} {} {}/{}",
        room.number, room.title, room.user_count, room.capacity
    );
    if room.has_password {
        text.push('!');
    }
    text
}

/// Room panel buttons in `ControlList_Interface.txt` file order:
/// `EXBUTTON1,2,3` then `5,4,6` — the file lists 5 before 4, so file
/// order reads the left column top-to-bottom then the right column
/// top-to-bottom. Ordinals `0x1B..0x20` pair `.bnd` rects 26..31
/// (`(26,82)-(314,173)` through `(317,272)-(605,363)`), the verified
/// 2-column x 3-row grid (left `x=26`, right `x=317`; rows `y=82`,
/// `177`, `272`).
const ROOM_PANEL_IDS: [&str; 6] = [
    "IDC_BUTTON_SROOM_EXBUTTON1",
    "IDC_BUTTON_SROOM_EXBUTTON2",
    "IDC_BUTTON_SROOM_EXBUTTON3",
    "IDC_BUTTON_SROOM_EXBUTTON5",
    "IDC_BUTTON_SROOM_EXBUTTON4",
    "IDC_BUTTON_SROOM_EXBUTTON6",
];

/// Control ids of the staged room panels that host the row
/// template, in row order (the `SET` clone instances the painter may
/// show; clones past the listed rooms stay hidden).
#[must_use]
pub(crate) fn room_panel_instances(placements: &[ScenePlacement]) -> Vec<&str> {
    placements
        .iter()
        .filter(|placed| placed.role == PlacementRole::Single)
        .filter(|placed| ROOM_PANEL_IDS.contains(&placed.id.as_str()))
        .map(|placed| placed.id.as_str())
        .collect()
}

/// Native origins of the staged room panels in file order (the
/// panel path positions fields panel-relatively; each field rect
/// scales to the frame on paint. Skips panels the staged art does
/// not name; empty while no staged art names any panel).
fn room_panel_origins(placements: &[ScenePlacement]) -> Vec<(i32, i32)> {
    ROOM_PANEL_IDS
        .iter()
        .filter_map(|id| {
            placements
                .iter()
                .find(|placed| placed.id == *id)
                .map(|placed| (placed.rect.x, placed.rect.y))
        })
        .collect()
}

/// Whether staged art binds any room panel (the panel path owns
/// layout while one is bound; otherwise the legacy row bands do).
#[must_use]
pub(crate) fn has_room_panels(placements: &[ScenePlacement]) -> bool {
    ROOM_PANEL_IDS
        .iter()
        .any(|id| placements.iter().any(|placed| placed.id == *id))
}

/// File-order index of the room panel whose staged `.bnd` rect
/// contains native pixel (`x`, `y`) (`None` while no bound panel
/// covers the point — stale panels past the list never hit).
#[must_use]
pub(crate) fn room_panel_hit(placements: &[ScenePlacement], x: i32, y: i32) -> Option<usize> {
    ROOM_PANEL_IDS
        .iter()
        .position(|id| staged_rect_hit(placements, id, x, y))
}

/// Room rows plus highlight cursor (no background fill: shared by
/// the flat fallback and the art scene). `None` art (or art binding
/// no panel) takes the legacy quarter-frame bands; staged panels
/// take the pinned per-panel fields.
fn paint_room_rows(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: Option<&SceneArt>,
) {
    match art {
        Some(art) => {
            let origins = room_panel_origins(&art.placements);
            if origins.is_empty() {
                paint_room_fallback_rows(frame, rooms, highlight);
            } else {
                paint_room_panel_rows(frame, rooms, highlight, art, &origins);
            }
        }
        None => paint_room_fallback_rows(frame, rooms, highlight),
    }
}

/// Exe-measured room row field origins (native px, panel-relative;
/// rig 2026-11-09 `t2-room-stage.png`,
/// `~/scratch/geom-open-close/findings.md` §2): the number plate at
/// panel +(12,12), the title at +(64,+11), the seat count at
/// +(194,+10). Title/count splits of the old combined row string ride
/// these instead of the photo-era single anchor (+34/+1,
/// `/tmp/aa-room.png` — retired as a misread).
///
/// Digit pitch of the number plate (native px): the pinned on-screen
/// digits sit 11 px apart (38/49/60 on panel 0), the `ROOMNO` place
/// width (`State_Room.bnd` r45..47).
const ROOM_ROW_DIGIT_PITCH: i32 = 11;
/// Place weights of the three room-number digit slots in control
/// ordinal order (ones, tens, hundreds).
const ROOM_ROW_DIGIT_WEIGHTS: [u32; 3] = [1, 10, 100];
/// Staged room-number digit font (the `ROOMNO` places' shared body,
/// `IDC_IMAGE_SROOM_ROOMNO1/10/100`).
const ROOM_NUMBER_SPRITE: &str = "Room_Number.ojs";
/// Panel-relative origin of the number plate (native px): panel
/// origin +12/+12 (rig pin above). Hundreds, tens, ones run
/// left-to-right at the digit pitch.
const ROOM_ROW_NUMBER_ORIGIN: (i32, i32) = (12, 12);
/// Panel-relative origin of the title field (native px): panel
/// origin +64/+11 (rig pin above; the `ROOMNAME` rect sits at +66 —
/// the 2 px run-in is the GDI left margin).
const ROOM_ROW_TITLE_ORIGIN: (i32, i32) = (64, 11);
/// Panel-relative origin of the seat-count field (native px): panel
/// origin +194/+10 (rig pin above — the count rides the row, not
/// the `ROOM_USER` rect, which the capture shows as background).
const ROOM_ROW_COUNT_ORIGIN: (i32, i32) = (194, 10);
/// GDI cap-height share of a TEXT control rect: the live room title
/// spans ~11 px in its 16 px rect, the music line ~14 px in 19 px
/// (`t2-room-stage.png`); text sizes to 70% of the rect height.
const TEXT_CAP_FRACTION: f32 = 0.7;

/// Em extent fitting `text` to a `w`×`h` rect at the exe's GDI cap
/// share: cap height fills [`TEXT_CAP_FRACTION`] of the rect height
/// (the full-height [`cp949_fit_scale`] would undershoot the live
/// glyphs by half). Width still clamps long strings into the rect.
fn room_text_scale(text: &str, w: f32, h: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 || text.is_empty() {
        return 0.0;
    }
    let (width_per_em, cap_em) = match o2jam_text::system_font() {
        Some(font) => (
            o2jam_text::measure_text(font, text, 1.0),
            font.cap_height_em(),
        ),
        None => (o2jam_text::font::measure_text_stand_in(text, 1.0), 1.0),
    };
    if width_per_em <= 0.0 || cap_em <= 0.0 {
        return 0.0;
    }
    let height_fit = TEXT_CAP_FRACTION * h / cap_em;
    height_fit.min(w / width_per_em).max(0.0)
}

/// Paint one room row string into its field rect (native, usually
/// panel-relative): the field origin is the pinned row offset while
/// the size rides the staged template rect — the exe's GDI text
/// fills the TEXT control at its cap share. Bare ink, no scrim.
fn paint_room_text_into_rect(frame: &mut Frame<'_>, native_rect: Rect, text: &str, ink: [u8; 4]) {
    if frame.w == 0 || frame.h == 0 || native_rect.w == 0 || native_rect.h == 0 || text.is_empty() {
        return;
    }
    let rect = scale_rect_to_frame(&native_rect, frame.w, frame.h);
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let scale = room_text_scale(text, rect.w as f32, rect.h as f32);
    if scale <= 0.0 {
        return;
    }
    let mut image = o2jam_text::Image::blank(rect.w, rect.h);
    o2jam_text::draw_text(
        &mut image,
        o2jam_text::system_font(),
        0,
        0,
        text,
        ink,
        scale,
    );
    composite_text_image(frame, &rect, &image);
}

/// Native rect of one staged template control (`None` while the
/// staged art names no such control — the row field then stays
/// unpainted, like the exe's untouched icons).
#[must_use]
fn staged_field_rect(placements: &[ScenePlacement], id: &str) -> Option<Rect> {
    placements
        .iter()
        .find(|placed| placed.id == id)
        .map(|placed| placed.rect)
}

/// Room number through the staged digit font: the row template's
/// number places are `SET 0x11` clones under the row panel, ordered by
/// control ordinal (ones, tens, hundreds — the authoring order
/// `ROOMNO1 0x1350002E`, `ROOMNO10 0x1350002F`, `ROOMNO100 0x13500030`),
/// each place painting the frame of its digit at its own clone rect.
/// GDI number text (sized to the title band) while no clone stages.
fn paint_room_row_number(
    frame: &mut Frame<'_>,
    art: &SceneArt,
    room: &RoomListEntry,
    panel_id: &str,
) {
    let value = u32::try_from(room.number.max(0)).unwrap_or(0);
    let mut places: Vec<&ScenePlacement> = art
        .placements
        .iter()
        .filter(|placed| placed.role == PlacementRole::Clone)
        .filter(|placed| placed.id == panel_id)
        .filter(|placed| placed.sprite.as_deref() == Some(ROOM_NUMBER_SPRITE))
        .collect();
    places.sort_by_key(|placed| placed.code & 0xFFF);
    if places.is_empty() {
        let Some(panel) = room_panel_origin(&art.placements, panel_id) else {
            return;
        };
        let band =
            staged_field_rect(&art.placements, ROOM_DETAIL_TITLE_ID).map_or(0, |rect| rect.h);
        paint_room_text_into_rect(
            frame,
            Rect {
                x: panel.0 + ROOM_ROW_NUMBER_ORIGIN.0,
                y: panel.1 + ROOM_ROW_NUMBER_ORIGIN.1,
                w: (u32::try_from(ROOM_ROW_DIGIT_PITCH).unwrap_or(0)).saturating_mul(3),
                h: band,
            },
            &value.to_string(),
            caption_ink(art_title_ink()),
        );
        return;
    }
    for (rank, place) in places.iter().enumerate() {
        let Some(weight) = ROOM_ROW_DIGIT_WEIGHTS.get(rank).copied() else {
            break;
        };
        let digit = (value / weight) % 10;
        paint_clone_digit(frame, &art.figures, place, digit);
    }
}

/// One clone body painted at its rect with the layer index `frame`
/// (digit frames are stored in digit order: layer *n* is glyph *n*,
/// `Room_Number.ojs` decodes ten 11x13 digit layers).
fn paint_clone_digit(
    frame: &mut Frame<'_>,
    figures: &HashMap<String, AvatarFigure>,
    place: &ScenePlacement,
    digit: u32,
) {
    let Some(sprite) = place.sprite.as_deref() else {
        return;
    };
    let Some(layer) = figures
        .get(sprite)
        .and_then(|figure| figure.layer(usize::try_from(digit).unwrap_or(usize::MAX)))
    else {
        return;
    };
    let dest = scale_rect_to_frame(&place.rect, frame.w, frame.h);
    blit_rgb555(
        frame,
        &layer.image.view(),
        dest.x + layer.dx,
        dest.y + layer.dy,
        Some(layer.colorkey),
    );
}

/// Native origin of one staged room panel (`None` when the staged art
/// binds no such panel).
fn room_panel_origin(placements: &[ScenePlacement], panel_id: &str) -> Option<(i32, i32)> {
    placements
        .iter()
        .find(|placed| placed.id == panel_id)
        .map(|placed| (placed.rect.x, placed.rect.y))
}

/// Room rows at their staged `.bnd` panel origins, one live room per
/// panel: the number at its pinned plate origin, the title and the
/// seat count at theirs, each sized to its template rect — the live
/// splits (`findings.md` §2) replace the old combined string. Bare
/// on the panel art (no scrim: the panels carry their own chrome).
/// Rooms past the staged panels stay off-page (the Left/Right pager
/// is untraced); the highlight cursor rides the selected panel.
fn paint_room_panel_rows(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
    origins: &[(i32, i32)],
) {
    let ink = caption_ink(art_title_ink());
    let panels = room_panel_instances(&art.placements);
    for (index, room) in rooms.iter().enumerate() {
        let Some((panel_x, panel_y)) = origins.get(index).copied() else {
            break;
        };
        let Some(panel_id) = panels.get(index).copied() else {
            break;
        };
        paint_room_row_number(frame, art, room, panel_id);
        if let Some(title) = staged_field_rect(&art.placements, ROOM_DETAIL_TITLE_ID) {
            paint_room_text_into_rect(
                frame,
                Rect {
                    x: panel_x + ROOM_ROW_TITLE_ORIGIN.0,
                    y: panel_y + ROOM_ROW_TITLE_ORIGIN.1,
                    w: title.w,
                    h: title.h,
                },
                &room.title,
                ink,
            );
        }
        if let Some(count) = staged_field_rect(&art.placements, ROOM_DETAIL_USERS_ID) {
            paint_room_text_into_rect(
                frame,
                Rect {
                    x: panel_x + ROOM_ROW_COUNT_ORIGIN.0,
                    y: panel_y + ROOM_ROW_COUNT_ORIGIN.1,
                    w: count.w,
                    h: count.h,
                },
                &room_capacity_label(room),
                ink,
            );
        }
        if index == highlight {
            let cursor = anchor_in_frame(
                panel_x + ROOM_ROW_NUMBER_ORIGIN.0,
                panel_y + ROOM_ROW_NUMBER_ORIGIN.1,
                frame.w,
                frame.h,
            );
            paint_cursor(frame, cursor.0, cursor.1);
        }
    }
}

/// Room rows at the legacy quarter-frame anchor (flat fallback and
/// art-less staged scenes only; the staged path above owns layout
/// while panels are bound).
fn paint_room_fallback_rows(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    paint_list_scrim(frame, rooms.len());
    let w = frame.w as i32;
    let h = frame.h as i32;
    let text_x = w / 4;
    for (index, room) in rooms.iter().enumerate() {
        let row = index_as_row(index, h);
        draw_text(
            frame,
            text_x,
            row,
            &room_row_text(room),
            pack_xrgb(0xFF, 0xFF, 0xFF),
        );
        if index == highlight {
            paint_cursor(frame, text_x, row);
        }
    }
}

/// Title anchor of the highlighted-room panel (`SET 0x11` row
/// template in `STATE_ROOM`).
const ROOM_DETAIL_TITLE_ID: &str = "IDC_TEXT_SROOM_ROOMNAME";
/// Seat-count anchor (`ROOM_USER`).
const ROOM_DETAIL_USERS_ID: &str = "IDC_TEXT_SROOM_ROOM_USER";
/// Selected-song anchor.
const ROOM_DETAIL_MUSIC_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE";
/// Difficulty anchor (second music-title slot).
const ROOM_DETAIL_DIFFICULTY_ID: &str = "IDC_TEXT_SROOM_MUSICTITLE2";
/// Scroll-speed icon anchor; text stands in for the icon body.
const ROOM_DETAIL_SPEED_ID: &str = "IDC_IMAGE_SROOM_SPEED";
/// Game-mode icon anchor (`Room_Mode.ojs`); text stands in.
const ROOM_DETAIL_MODE_ID: &str = "IDC_IMAGE_SROOM_MODE";
/// Key-mode anchor (`Room_Om.ojs` option-mode icon).
const ROOM_DETAIL_KEY_ID: &str = "IDC_IMAGE_SROOM_OM";
/// Level-gate text anchor beside `Room_LevelLimit.ojs`.
const ROOM_DETAIL_LEVEL_ID: &str = "IDC_TEXT_SROOM_LEVEL_RANGE";
/// Lock-icon anchor (`Room_Password.ojs`); text stands in.
const ROOM_DETAIL_LOCK_ID: &str = "IDC_IMAGE_SROOM_PASSWORD";

/// Detail lines per panel: title plus the eight entry fields.
const ROOM_DETAIL_LINES: usize = 9;

/// Game-mode label (`NORMAL`/`LIVE`; raw code while unknown).
#[must_use]
fn room_mode_label(game_mode: u8) -> String {
    match GameMode::from_u8(game_mode) {
        Ok(GameMode::Normal) => "NORMAL".to_owned(),
        Ok(GameMode::Live) => "LIVE".to_owned(),
        Err(_) => format!("MODE {game_mode}"),
    }
}

/// Selected-song label (the song table lives server-side, so the id).
#[must_use]
fn room_music_label(music_id: u16) -> String {
    format!("M{music_id}")
}

/// Difficulty label (`EX`/`NX`/`HX`/`RX`; raw code while unknown).
#[must_use]
fn room_difficulty_label(difficulty: u8) -> String {
    match Difficulty::from_code(difficulty) {
        Some(Difficulty::Ex) => "EX".to_owned(),
        Some(Difficulty::Nx) => "NX".to_owned(),
        Some(Difficulty::Hx) => "HX".to_owned(),
        Some(Difficulty::Rx) => "RX".to_owned(),
        None => format!("DIF {difficulty}"),
    }
}

/// Key-mode label (`3KEY`/`5KEY`/`7KEY`; raw code while unknown).
#[must_use]
fn room_key_label(key_mode: u8) -> String {
    match KeyMode::from_u8(key_mode) {
        Ok(KeyMode::ThreeKeys) => "3KEY".to_owned(),
        Ok(KeyMode::FiveKeys) => "5KEY".to_owned(),
        Ok(KeyMode::SevenKeys) => "7KEY".to_owned(),
        Err(_) => format!("KEY {key_mode}"),
    }
}

/// Scroll-speed label (`X0.5`..`X6`, `XR`; raw code while unknown).
#[must_use]
fn room_speed_label(speed: u8) -> String {
    match GameSpeed::from_code(speed) {
        Some(GameSpeed::X05) => "X0.5".to_owned(),
        Some(GameSpeed::X10) => "X1".to_owned(),
        Some(GameSpeed::X15) => "X1.5".to_owned(),
        Some(GameSpeed::X20) => "X2".to_owned(),
        Some(GameSpeed::X25) => "X2.5".to_owned(),
        Some(GameSpeed::X30) => "X3".to_owned(),
        Some(GameSpeed::X35) => "X3.5".to_owned(),
        Some(GameSpeed::X40) => "X4".to_owned(),
        Some(GameSpeed::X45) => "X4.5".to_owned(),
        Some(GameSpeed::X50) => "X5".to_owned(),
        Some(GameSpeed::X60) => "X6".to_owned(),
        Some(GameSpeed::Xr) => "XR".to_owned(),
        None => format!("SPD {speed}"),
    }
}

/// Seat label (`seated/capacity`).
#[must_use]
fn room_capacity_label(room: &RoomListEntry) -> String {
    format!("{}/{}", room.user_count, room.capacity)
}

/// Level-gate label (`LV min-max`).
#[must_use]
fn room_level_label(room: &RoomListEntry) -> String {
    format!("LV {}-{}", room.min_level, room.max_level)
}

/// Lock label (`LOCK` with a password, `OPEN` without).
#[must_use]
fn room_lock_label(has_password: bool) -> &'static str {
    if has_password {
        "LOCK"
    } else {
        "OPEN"
    }
}

/// Lock-line ink: red while a password is set, dim otherwise.
fn room_lock_ink(locked: bool) -> u32 {
    if locked {
        pack_xrgb(0xFF, 0x50, 0x50)
    } else {
        pack_xrgb(0x90, 0x90, 0x90)
    }
}

/// Ordered detail entries for one room: (`State_Room.bnd`
/// control id, text).
#[must_use]
fn room_detail_entries(room: &RoomListEntry) -> [(&'static str, String); ROOM_DETAIL_LINES] {
    [
        (ROOM_DETAIL_TITLE_ID, room.title.clone()),
        (ROOM_DETAIL_USERS_ID, room_capacity_label(room)),
        (ROOM_DETAIL_MUSIC_ID, room_music_label(room.music_id)),
        (
            ROOM_DETAIL_DIFFICULTY_ID,
            room_difficulty_label(room.difficulty),
        ),
        (ROOM_DETAIL_KEY_ID, room_key_label(room.key_mode)),
        (ROOM_DETAIL_MODE_ID, room_mode_label(room.game_mode)),
        (ROOM_DETAIL_SPEED_ID, room_speed_label(room.speed)),
        (ROOM_DETAIL_LEVEL_ID, room_level_label(room)),
        (
            ROOM_DETAIL_LOCK_ID,
            room_lock_label(room.has_password).to_owned(),
        ),
    ]
}

/// Fallback origin of detail line `slot` without staged art: stacked
/// rows under the list so the list plus cursor keep their room.
fn room_detail_fallback_origin(slot: usize, row_count: usize, frame: &Frame<'_>) -> (i32, i32) {
    let rows = i32::try_from(row_count).unwrap_or(i32::MAX);
    let slot_offset = i32::try_from(slot).unwrap_or(i32::MAX);
    (
        frame.w as i32 / 4,
        frame.h as i32 / 4 + rows * CHANNEL_ROW_H + 4 + slot_offset * CHANNEL_ROW_H,
    )
}

/// One detail line: dark scrim plus ink (shared by the flat fallback
/// and the art scene).
fn paint_room_detail_line(frame: &mut Frame<'_>, x: i32, y: i32, text: &str, ink: u32) {
    let text_w = text_width(text) as i32;
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
    draw_text(frame, x, y, text, ink);
}

/// Lobby row-state icon body (`SET 0x11` row template, child 12):
/// frame = the raw state byte (icon site `0x1268ae`), so frames 0/1/2
/// drive its images (docs/room-enums.md).
pub const ROOM_STATE_SPRITE: &str = "Room_State.ojs";
/// Lobby key-mode icon body (child 7): frame 1 = 3-key, frame 2 =
/// 7-key — the key-mode mark, not the battle mark (icon sites
/// `0x1267ad`–`0x1267e2`).
pub const ROOM_MODE_SPRITE: &str = "Room_Mode.ojs";
/// Lobby option-mode icon body (child 8): frames 1–4 depict 1–4
/// pudding mascots, banded by the selected song's level word (band
/// mapper `0x126cb0`).
pub const ROOM_OM_SPRITE: &str = "Room_Om.ojs";
/// Music floor a room row must pass to run the per-row fill (row
/// `+0x88` vs `0x64`, file `0x1251da`); below it the room shows the
/// no-song display state.
pub const ROOM_MUSIC_FLOOR: u16 = 0x64;

/// `Room_State.ojs` frame for one room: the state byte unmodified
/// (icon site `0x1268ae`, docs/room-enums.md).
#[must_use]
fn room_state_icon_frame(state_code: u8) -> u8 {
    state_code
}

/// `Room_Mode.ojs` frame for one room: key mode 0 → frame 1, 2 →
/// frame 2, anything else untouched (`None` — the exe leaves the
/// icon alone; icon sites `0x1267ad`–`0x1267e2`).
#[must_use]
fn room_key_icon_frame(key_mode: u8) -> Option<u8> {
    match KeyMode::from_u8(key_mode) {
        Ok(KeyMode::ThreeKeys) => Some(1),
        Ok(KeyMode::SevenKeys) => Some(2),
        _ => None,
    }
}

/// `Room_Om.ojs` frame for one room: below the pinned music floor the
/// row shows the no-song display state → constant level 1 → frame 1
/// (mapper call `0x1266b9`); past the floor the song's level word
/// maps through the band mapper ([`om_icon_frame`], bands 0–4/5–8/
/// 9–12/13+ → frames 1–4). The shell stages no song table (it lives
/// server-side), so a song without a known level paints nothing
/// rather than a wrong pudding count.
#[must_use]
fn room_om_icon_frame(music_id: u16, level: Option<u16>) -> Option<u8> {
    if music_id < ROOM_MUSIC_FLOOR {
        Some(1)
    } else {
        level.map(om_icon_frame)
    }
}

/// Image index the exe sprite setter paints for frame value `frame`
/// (setter `0x2f870`: store verbatim, clamp against the stored count,
/// subtract 1 before the image lookup — frame *n* paints image *n−1*):
/// saturating at image 0 for frame 0, clamped at the last decoded
/// layer otherwise. `None` for an empty body.
#[must_use]
fn icon_frame_layer(frame: u8, layers: usize) -> Option<usize> {
    if layers == 0 {
        return None;
    }
    Some(usize::from(frame.saturating_sub(1)).min(layers - 1))
}

/// Paint one frame of a staged row-template icon under one panel
/// instance: the `SET` clone owned by `panel_id` supplies the rect
/// (parent origin already added by staging), and the selected frame
/// blits 1:1 at that rect plus its record anchor (clones keep
/// template-local anchors, see [`PlacementRole::Clone`]). A missing
/// clone, figure, frame, or layer paints nothing — the exe leaves an
/// untouched icon exactly so.
fn paint_icon_frame(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    icon_frame: Option<u8>,
    panel_id: &str,
) {
    let Some(icon_frame) = icon_frame else {
        return;
    };
    let Some(placed) = placements.iter().find(|placed| {
        placed.id == panel_id
            && placed.role == PlacementRole::Clone
            && placed.sprite.as_deref() == Some(sprite)
    }) else {
        return;
    };
    let Some(figure) = figures.get(sprite) else {
        return;
    };
    let Some(layer_index) = icon_frame_layer(icon_frame, figure.len()) else {
        return;
    };
    let Some(layer) = figure.layer(layer_index) else {
        return;
    };
    let dest = scale_rect_to_frame(&placed.rect, frame.w, frame.h);
    blit_rgb555(
        frame,
        &layer.image.view(),
        dest.x + layer.dx,
        dest.y + layer.dy,
        Some(layer.colorkey),
    );
}

/// Row icons of every populated room: state, key mode, and Om
/// pudding count, painted from that row's wire fields into the
/// `SET 0x11` clone staged under its panel (`Room_State.ojs`
/// `(203,63)`, `Room_Mode.ojs` `(9,62)`, `Room_Om.ojs` `(56,62)` —
/// panel `(26,82)` + rect in the 7.55 room capture,
/// `/home/arnim/scratch/ac-room.png`). Panels past the list keep
/// their bare plate: the exe clones the template only for populated
/// rows. Missing art paints nothing; the detail text draws on top.
fn paint_room_row_icons(frame: &mut Frame<'_>, art: &SceneArt, rooms: &[RoomListEntry]) {
    let panel_ids = room_panel_instances(&art.placements);
    for (index, room) in rooms.iter().enumerate() {
        let Some(panel_id) = panel_ids.get(index) else {
            break;
        };
        paint_icon_frame(
            frame,
            &art.placements,
            &art.figures,
            ROOM_STATE_SPRITE,
            Some(room_state_icon_frame(room.state_code)),
            panel_id,
        );
        paint_icon_frame(
            frame,
            &art.placements,
            &art.figures,
            ROOM_MODE_SPRITE,
            room_key_icon_frame(room.key_mode),
            panel_id,
        );
        paint_icon_frame(
            frame,
            &art.placements,
            &art.figures,
            ROOM_OM_SPRITE,
            room_om_icon_frame(room.music_id, None),
            panel_id,
        );
    }
}

/// Icon body behind one icon-rect detail label (`None` for the
/// text-only fields): the label stands in for the icon body, so a
/// staged figure silences it and the icon pass owns the rect.
fn room_icon_sprite_for_detail(id: &str) -> Option<&'static str> {
    if id == ROOM_DETAIL_MODE_ID {
        Some(ROOM_MODE_SPRITE)
    } else if id == ROOM_DETAIL_KEY_ID {
        Some(ROOM_OM_SPRITE)
    } else {
        None
    }
}

/// Details of the highlighted room: song, difficulty, key mode,
/// game mode, speed, level gate, lock — the template fields the row
/// string does not carry (number, title, and seat count ride the
/// pinned row fields instead; the count's `ROOM_USER` rect paints
/// background in the live capture). Staged details paint at the
/// highlighted panel's origin plus their `State_Room.bnd` rects, sized
/// to the rects at real sizes (the TEXT rects are 16-20 px tall; the
/// icon-rect labels ride the same fit); icon-rect labels stand down
/// while their icon figure stages (the icon pass owns the rect).
/// Without staged art the stacked fallback lines keep the list
/// readable. Nothing paints without a room, or without a staged
/// panel under the highlight.
fn paint_room_details(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: Option<&SceneArt>,
) {
    let Some(room) = rooms.get(highlight) else {
        return;
    };
    let Some(art) = art else {
        for (slot, (id, text)) in room_detail_entries(room).iter().enumerate() {
            let ink = if *id == ROOM_DETAIL_LOCK_ID {
                room_lock_ink(room.has_password)
            } else {
                art_title_ink()
            };
            let (x, y) = room_detail_fallback_origin(slot, rooms.len(), frame);
            paint_room_detail_line(frame, x, y, text, ink);
        }
        return;
    };
    let placements = &art.placements;
    let Some((panel_x, panel_y)) = room_panel_origins(placements).get(highlight).copied() else {
        return;
    };
    for (id, text) in room_detail_entries(room).iter() {
        // Number, title, and seat count ride the pinned row fields
        // instead of the template rects (the count's `ROOM_USER` rect
        // paints background in the live capture).
        if *id == ROOM_DETAIL_TITLE_ID || *id == ROOM_DETAIL_USERS_ID {
            continue;
        }
        // A staged icon figure owns its rect; the text label stands
        // down (it is the figure-less stand-in).
        if room_icon_sprite_for_detail(id).is_some_and(|sprite| art.figures.contains_key(sprite)) {
            continue;
        }
        let Some(placement) = placements.iter().find(|entry| entry.id == *id) else {
            continue;
        };
        let ink = if *id == ROOM_DETAIL_LOCK_ID {
            room_lock_ink(room.has_password)
        } else {
            art_title_ink()
        };
        paint_room_text_into_rect(
            frame,
            Rect {
                x: placement.rect.x + panel_x,
                y: placement.rect.y + panel_y,
                w: placement.rect.w,
                h: placement.rect.h,
            },
            text,
            caption_ink(ink),
        );
    }
}

/// Paint the online room list with a highlight cursor.
pub(crate) fn paint_room_list(frame: &mut Frame<'_>, rooms: &[RoomListEntry], highlight: usize) {
    fill_background(frame, Screen::Room);
    paint_title(frame, "ROOM");
    paint_room_rows(frame, rooms, highlight, None);
    paint_room_details(frame, rooms, highlight, None);
}

/// Paint staged lobby art with the live room list and cursor on top.
///
/// `hovered_code` is the staged chrome control under the pointer, if
/// any: the exe repaints that control at full body frame 0 while the
/// pointer sits on it (`docs/hover.md`). Row panels stay static — they
/// are not chrome controls, so the pointer over a panel repaints
/// nothing (negative evidence: 0 changed px on all six panels,
/// `Room_MOver.ojs` unmatched in either state).
pub fn paint_room_scene(
    frame: &mut Frame<'_>,
    rooms: &[RoomListEntry],
    highlight: usize,
    art: &SceneArt,
    hovered_code: Option<u32>,
) {
    let panels = room_panel_instances(&art.placements);
    art.paint_backdrop_live(frame, Screen::Room, |placed| {
        clone_instance_live(placed, &panels, rooms.len())
    });
    paint_room_row_icons(frame, art, rooms);
    paint_room_details(frame, rooms, highlight, Some(art));
    // Rows paint last so the live list and highlight cursor stay
    // visible over the staged template bodies (UI ink on top): the
    // `SET 0x11` template rects are panel-relative while rows ride
    // per-panel rects, so the template art overlaps its own panel —
    // the exe's per-row template clone offsets stay OPEN past the
    // title/number/count pins (`findings.md` §2).
    paint_room_rows(frame, rooms, highlight, Some(art));
    paint_room_hover(frame, art, hovered_code);
}

/// Hover repaint for one staged room chrome button (`docs/hover.md`
/// 2026-11-10 rig sweep): the exe repaints the hovered control at
/// full body frame 0, so the shell repaints that frame alone over the
/// stacked rest paint. The rest-state desaturation transform is
/// measured but unexplained (rest matches neither body record), so the
/// rest paint keeps today's stacked look — labeled OPEN, next step a
/// stopped-state winedbg draw-state read per `docs/hover.md`. Active
/// filter/tab codes never reach this painter (the resolver holds them
/// at rest — EXE-pinned no-change), so every code here repaints layer
/// 0. A missing placement, sprite, or frame paints nothing.
pub(crate) fn paint_room_hover(frame: &mut Frame<'_>, art: &SceneArt, hovered_code: Option<u32>) {
    let Some(code) = hovered_code else {
        return;
    };
    let Some(placed) = art.placements.iter().find(|placed| placed.code == code) else {
        return;
    };
    let shared = shared_sprite_names(&art.placements);
    let shares_body = placed
        .sprite
        .as_deref()
        .is_some_and(|sprite| shared.contains(sprite));
    paint_placement_layer(frame, placed, &art.figures, 0, shares_body, None);
}

/// Room user-panel control ids (`STATE_ROOM` text/image wells; the
/// rects pair through the verified ordinal rule).
const ROOM_NICKNAME_ID: &str = "IDC_TEXT_SROOM_NICKNAME";
/// Level well (`Level` row).
const ROOM_PANEL_LEVEL_ID: &str = "IDC_TEXT_SROOM_MYLEVEL";
/// Experience well (`Exp.` row).
const ROOM_PANEL_EXP_ID: &str = "IDC_TEXT_SROOM_EXP";
/// Grade well (`정·직` row).
const ROOM_PANEL_GRADE_ID: &str = "IDC_TEXT_SROOM_WINLOSE";
/// Cash-point well (`Cash Point` row).
const ROOM_PANEL_CASH_ID: &str = "IDC_TEXT_SROOM_BONUSPOINT";
/// Penalty-count well (`패널티횟수` row).
const ROOM_PANEL_PENALTY_COUNT_ID: &str = "IDC_TEXT_SROOM_PENALTYCOUNT";
/// Penalty-level well (`패널티레벨` row).
const ROOM_PANEL_PENALTY_LEVEL_ID: &str = "IDC_TEXT_SROOM_PENALTYLEVEL";
/// Band well (`나의밴드` row).
const ROOM_PANEL_BAND_ID: &str = "IDC_TEXT_SROOM_GUILD";
/// Worn-figure slot (`IDC_IMAGE_SROOM_AVATA`, code `0x13700013`).
const ROOM_AVATAR_ID: &str = "IDC_IMAGE_SROOM_AVATA";

/// Placeholder the exe paints in the grade and band wells: the 7.55
/// room capture (`/home/arnim/scratch/ac-room.png`, tester Lv 2) shows
/// `준비중..` in both, beside real values in the other wells.
const ROOM_PANEL_PLACEHOLDER: &str = "준비중..";

/// Paint the room's user panel: the worn figure in its staged slot and
/// the identity readouts in their staged text wells.
///
/// Wells come from the parsed `.bnd` rects (owner rule `f38f3`); a well
/// the staged art does not name stays unpainted. Values ride
/// `GetCharacterInfo` 2001 (`CharacterInfo`) — the same record the
/// MyRoom scene reads — and the nickname line prefixes the level, as
/// the 7.55 room capture shows (`Lv.2 tester`).
///
/// OPEN wells (nothing painted, next steps named):
/// - `NEXTUPEXP`: needs the level-up experience table (not parsed).
/// - `MYRANK` / `NOTICE_TEXT` / `DESC`: web-served text, no client
///   source (the capture shows art text in `DESC`).
pub fn paint_room_user_panel(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    character: Option<&CharacterInfo>,
    figure: Option<&AvatarFigure>,
) {
    if let (Some(figure), Some(slot)) = (figure, staged_field_rect(placements, ROOM_AVATAR_ID)) {
        let slot = scale_rect_to_frame(&slot, frame.w, frame.h);
        figure.paint_extent_centered(frame, &slot, frame.w, frame.h);
    }
    if let Some(character) = character {
        let ink = caption_ink(art_title_ink());
        let nickname = format!("Lv.{} {}", character.level, character.nickname);
        paint_room_panel_value(frame, placements, ROOM_NICKNAME_ID, &nickname, ink);
        paint_room_panel_number(frame, placements, ROOM_PANEL_LEVEL_ID, character.level, ink);
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_EXP_ID,
            character.experience,
            ink,
        );
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_CASH_ID,
            character.bonus_point,
            ink,
        );
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_PENALTY_COUNT_ID,
            i32::from(character.penalty_count),
            ink,
        );
        paint_room_panel_number(
            frame,
            placements,
            ROOM_PANEL_PENALTY_LEVEL_ID,
            i32::from(character.penalty_level),
            ink,
        );
    }
    // The grade and band wells carry the exe's own placeholder.
    let ink = caption_ink(art_title_ink());
    for id in [ROOM_PANEL_GRADE_ID, ROOM_PANEL_BAND_ID] {
        paint_room_panel_value(frame, placements, id, ROOM_PANEL_PLACEHOLDER, ink);
    }
}

/// One integer into its staged well.
fn paint_room_panel_number(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    id: &str,
    value: i32,
    ink: [u8; 4],
) {
    paint_room_panel_value(frame, placements, id, &value.to_string(), ink);
}

/// One value into its staged well: left-anchored, sized to the rect at
/// the exe's GDI cap share (the same fit the row fields use).
fn paint_room_panel_value(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    id: &str,
    text: &str,
    ink: [u8; 4],
) {
    let Some(rect) = staged_field_rect(placements, id) else {
        return;
    };
    paint_text_into_rect(frame, rect, text, ink);
}

/// Seats shown on the waiting screen: self plus the seven wire slots.
pub(crate) const WAITING_SEATS: usize = 8;
/// Peer slots carried beside self in [`JoinedRoom`].
const WAITING_PEERS: usize = 7;
/// Waiting rows share the lobby row height.
const WAITING_ROW_H: i32 = 8;

/// One waiting-room seat view: display name plus team/ready state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaitingSeat {
    team: u8,
    name: String,
    kind: SeatKind,
}

/// Seat occupancy: only occupied seats carry a name and ready flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeatKind {
    Occupied { ready: bool },
    Empty,
    Locked,
}

impl WaitingSeat {
    /// Display name (`None` for vacant/locked seats).
    #[must_use]
    pub(crate) fn display_name(&self) -> Option<&str> {
        match self.kind {
            SeatKind::Occupied { .. } => Some(&self.name),
            SeatKind::Empty | SeatKind::Locked => None,
        }
    }

    /// Ready flag (vacant/locked seats are never ready).
    #[must_use]
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self.kind, SeatKind::Occupied { ready: true })
    }
}

/// Team button letter (`0` -> `A` .. `7` -> `H`); `?` when out of range.
fn team_letter(team: u8) -> char {
    if team < 8 {
        (b'A' + team) as char
    } else {
        '?'
    }
}

/// Our own seat: name is local-only (the wire sends no self nickname).
fn self_seat(room: &JoinedRoom) -> WaitingSeat {
    // Own readiness rides the member slot like every peer (the ready
    // toggle folds there); unknown seats keep the unready look.
    let ready = room
        .slots
        .iter()
        .find(|slot| slot.index == room.member_id)
        .and_then(|slot| slot.member.as_ref())
        .is_some_and(|member| member.ready);
    WaitingSeat {
        team: room.team,
        name: "YOU".to_owned(),
        kind: SeatKind::Occupied { ready },
    }
}

/// Vacant seat placeholder.
fn empty_seat() -> WaitingSeat {
    WaitingSeat {
        team: 0,
        name: String::new(),
        kind: SeatKind::Empty,
    }
}

/// View for one wire slot: named when occupied, locked/empty otherwise.
fn peer_seat(slot: &RoomSlot) -> WaitingSeat {
    match &slot.member {
        Some(member) => WaitingSeat {
            team: member.team,
            name: member.nickname.clone(),
            kind: SeatKind::Occupied {
                ready: member.ready,
            },
        },
        None if slot.state_code == RoomSlotState::Locked as i32 => WaitingSeat {
            team: 0,
            name: String::new(),
            kind: SeatKind::Locked,
        },
        None => empty_seat(),
    }
}

/// Eight seat views: our seat first, then the wire slots, padded empty.
///
/// The wire carries seven peer slots beside self (`JoinedRoom.slots`);
/// together they are the eight waiting-room seats. OPEN: `slots`
/// includes our own index (`member_id`), so this view paints our
/// member twice — the seat-to-cover mapping needs evidence (next step:
/// a 7.55 waiting capture with two players seated in known slots, then
/// compare which cover each name lands on).
#[must_use]
pub(crate) fn waiting_seats(room: &JoinedRoom) -> Vec<WaitingSeat> {
    let mut seats = Vec::with_capacity(WAITING_SEATS);
    seats.push(self_seat(room));
    for slot in room.slots.iter().take(WAITING_PEERS) {
        seats.push(peer_seat(slot));
    }
    while seats.len() < WAITING_SEATS {
        seats.push(empty_seat());
    }
    seats
}

/// One seat row: seat, team, name, flag (`R` ready, `-` waiting).
/// Vacant seats keep their index with a `---`/`LOCK` placeholder.
fn waiting_row_text(index: usize, seat: &WaitingSeat) -> String {
    match seat.kind {
        SeatKind::Occupied { ready } => {
            let flag = if ready { 'R' } else { '-' };
            format!(
                "{} {} {} {}",
                index,
                team_letter(seat.team),
                seat.name,
                flag
            )
        }
        SeatKind::Empty => format!("{index} - ---"),
        SeatKind::Locked => format!("{index} - LOCK"),
    }
}

/// Ready-dot ink: green when ready, dim otherwise.
fn ready_dot_color(ready: bool) -> u32 {
    if ready {
        pack_xrgb(0x30, 0xE0, 0x30)
    } else {
        pack_xrgb(0x50, 0x50, 0x50)
    }
}

/// Top-left of seat `index`: four columns by two rows, relative to frame
/// (Wine `ab-wait.png` + siblings: four seats across the top row,
/// four across the bottom — seat 4 opens the second row under seat 0).
#[must_use]
pub(crate) fn seat_origin(index: usize, w: usize, h: usize) -> (i32, i32) {
    let col = (index % 4) as i32;
    let row = (index / 4) as i32;
    (
        w as i32 / 16 + col * (w as i32 / 4),
        h as i32 / 4 + row * WAITING_ROW_H,
    )
}

/// Seat rows plus ready dots (no background fill: shared by the flat
/// fallback and the art scene).
fn paint_waiting_rows(frame: &mut Frame<'_>, room: &JoinedRoom) {
    for (index, seat) in waiting_seats(room).iter().enumerate() {
        paint_waiting_seat(frame, index, seat);
    }
}

/// Paint the eight waiting-room seats with names and ready flags.
pub(crate) fn paint_waiting_members(frame: &mut Frame<'_>, room: &JoinedRoom) {
    fill_background(frame, Screen::Waiting);
    paint_title(frame, "WAITING");
    paint_waiting_rows(frame, room);
}

/// Live caption sources for the staged waiting rects. Every field
/// has a pinned shell source (`docs/caption-sources.md`): the map
/// name rides the room word, the BPM rides the staged catalog
/// column for the room's song, and the counts ride the staged
/// inventory. `Default` is the no-source state (mismatch `-0-`,
/// zeroed counts, no title) — the exe paints exactly this without
/// a staged record.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WaitingCaptions<'a> {
    /// Staged catalog title for the room's song, if it names one.
    pub song_title: Option<&'a str>,
    /// Staged catalog BPM of the room's song, if it names one.
    pub song_bpm: Option<f32>,
    /// Staged inventory counts, one per skill slot.
    pub skill_counts: [u32; 3],
}

/// Paint staged waiting art (team buttons, avatar covers) with the
/// live seats, ready dots, and pinned captions on top: staged
/// placements anchor every region (covers, team grid, READY, skills,
/// counts, map, BPM, speed, music, wins), while the flat fallback
/// below keeps the invented grid. Without staged cover placements
/// the legacy rows keep their slot so the bare scene still reads.
pub fn paint_waiting_scene(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    art: &SceneArt,
    captions: &WaitingCaptions<'_>,
) {
    let seats = staged_seats(room);
    art.paint_backdrop_live(frame, Screen::Waiting, |placed| {
        waiting_seat_clone_live(placed, &seats)
    });
    paint_waiting_seat_marks(frame, &seats, art);
    if waiting_has_staged_covers(&art.placements) {
        paint_waiting_staged(frame, room, &art.placements, captions);
    } else {
        paint_waiting_rows(frame, room);
    }
}

/// Seat avatar slot (`IDC_IMAGE_SWAIT_AVATAR`, code `0x1670002E`): a
/// spriteless `SET 0x0C` member whose rect is seat-relative (the seat
/// template's own origin), so the painter adds the seat parent's rect
/// origin — the same write-back the clone expansion applies to the
/// members that carry art.
const WAITING_AVATAR_SLOT_ID: &str = "IDC_IMAGE_SWAIT_AVATAR";

/// Native avatar slot of one seat: the spriteless template rect added
/// onto the seat parent's origin (the clone write-back rule), `None`
/// while either placement is unstaged — the slot then stays empty,
/// never a raw-rect paint at the frame corner.
#[must_use]
fn waiting_seat_slot_rect(placements: &[ScenePlacement], seat: usize) -> Option<Rect> {
    let slot = placements
        .iter()
        .find(|placed| placed.id == WAITING_AVATAR_SLOT_ID)
        .map(|placed| placed.rect)?;
    let parent = WAITING_SEAT_PARENT_IDS
        .get(seat)
        .and_then(|id| placements.iter().find(|placed| placed.id == *id))
        .map(|placed| placed.rect)?;
    Some(Rect {
        x: parent.x.saturating_add(slot.x),
        y: parent.y.saturating_add(slot.y),
        w: slot.w,
        h: slot.h,
    })
}

/// Paint one worn figure into one seat slot, centred on the slot's
/// scaled box (the shared seat-slot path: self and peers alike).
fn paint_figure_in_seat_slot(frame: &mut Frame<'_>, figure: &AvatarFigure, slot: &Rect) {
    let scaled = scale_rect_to_frame(slot, frame.w, frame.h);
    figure.paint_extent_centered(frame, &scaled, frame.w, frame.h);
}

/// Paint the local player's worn figure into their own seat's avatar
/// slot (seat 0, the `IDC_BUTTON_SWAIT_EXTEND1` instance the exe seats
/// the room host on). Thin wrapper over the shared seat-slot path.
pub fn paint_waiting_seat_avatar(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    figure: Option<&AvatarFigure>,
) {
    let Some(figure) = figure else {
        return;
    };
    if let Some(slot) = waiting_seat_slot_rect(placements, 0) {
        paint_figure_in_seat_slot(frame, figure, &slot);
    }
}

/// Paint each occupied seat's staged figure into its own avatar slot
/// through the same slot path as the local seat (seat-relative rect
/// plus the seat parent's origin). Vacant and locked seats keep their
/// bare plate even when a figure is staged; unstaged seats stay empty.
/// No template child ever paints at the raw rect: every figure goes
/// through its seat's parent offset.
pub fn paint_waiting_seat_avatars(
    frame: &mut Frame<'_>,
    placements: &[ScenePlacement],
    room: &JoinedRoom,
    figures: &[Option<AvatarFigure>],
) {
    let seats = staged_seats(room);
    for (index, staged_figure) in figures.iter().enumerate() {
        if seats.get(index).is_none_or(|seat| !seat.present) {
            continue;
        }
        let (Some(figure), Some(slot)) = (
            staged_figure.as_ref(),
            waiting_seat_slot_rect(placements, index),
        ) else {
            continue;
        };
        paint_figure_in_seat_slot(frame, figure, &slot);
    }
}

/// Seat-cover control ids in seat order (`COVERAVATAR1..8`, codes
/// `0x1650003D..44` → `.bnd` rects 60..67): each cover rect positions
/// its own seat, anchor-free bodies included (the placement lesson).
const WAITING_COVER_IDS: [&str; 8] = [
    "IDC_IMAGE_SWAIT_COVERAVATAR1",
    "IDC_IMAGE_SWAIT_COVERAVATAR2",
    "IDC_IMAGE_SWAIT_COVERAVATAR3",
    "IDC_IMAGE_SWAIT_COVERAVATAR4",
    "IDC_IMAGE_SWAIT_COVERAVATAR5",
    "IDC_IMAGE_SWAIT_COVERAVATAR6",
    "IDC_IMAGE_SWAIT_COVERAVATAR7",
    "IDC_IMAGE_SWAIT_COVERAVATAR8",
];
/// Team-button control ids in team order (`TEAMA..H`, codes
/// `0x16201004..0B` → `.bnd` rects 3..10).
const WAITING_TEAM_IDS: [&str; 8] = [
    "IDC_BUTTON_SWAIT_TEAMA",
    "IDC_BUTTON_SWAIT_TEAMB",
    "IDC_BUTTON_SWAIT_TEAMC",
    "IDC_BUTTON_SWAIT_TEAMD",
    "IDC_BUTTON_SWAIT_TEAME",
    "IDC_BUTTON_SWAIT_TEAMF",
    "IDC_BUTTON_SWAIT_TEAMG",
    "IDC_BUTTON_SWAIT_TEAMH",
];
/// Room-title text (`TEXT_SWAIT_ROOMNAME`, code `0x16400016` → rect 21).
const WAITING_ROOMNAME_ID: &str = "IDC_TEXT_SWAIT_ROOMNAME";
/// Selected-song text (`TEXT_SWAIT_MUSICNAME`, code `0x16400017` →
/// rect 22, top bar beside the title).
const WAITING_MUSICNAME_ID: &str = "IDC_TEXT_SWAIT_MUSICNAME";
/// Champion win-count text (`TEXT_SWAIT_WINNERSCROE`, code `0x1640006D`
/// → rect 108, the Wins box inside the champion card).
const WAITING_WINNERSCORE_ID: &str = "IDC_TEXT_SWAIT_WINNERSCROE";
/// Scroll-speed value art (`SPEED_NUM`, code `0x16500066` → rect 101).
const WAITING_SPEED_NUM_ID: &str = "IDC_IMAGE_SWAIT_SPEED_NUM";
/// Skill-slot art ids top to bottom (`1/2/3SKILL`, codes
/// `0x16500058..5A` → rects 87..89, the X boxes of the SKILL panel).
const WAITING_SKILL_IDS: [&str; 3] = [
    "IDC_IMAGE_SWAIT_1SKILL",
    "IDC_IMAGE_SWAIT_2SKILL",
    "IDC_IMAGE_SWAIT_3SKILL",
];
/// Map-name text (`TEXT_SWAIT_MAPNAME`, code `0x16400018` →
/// rect 23: `(code & 0xFFF) − 1`, beside the title).
const WAITING_MAPNAME_ID: &str = "IDC_TEXT_SWAIT_MAPNAME";
/// BPM text (`TEXT_SWAIT_BPM`, code `0x16400069` → rect 104).
const WAITING_BPM_ID: &str = "IDC_TEXT_SWAIT_BPM";
/// Skill-count texts (`TEXT_SWAIT_1/2/3SKILLCOUNT`, codes
/// `0x1640005E..60` → rects 93..95, beside the X boxes).
const WAITING_SKILLCOUNT_IDS: [&str; 3] = [
    "IDC_TEXT_SWAIT_1SKILLCOUNT",
    "IDC_TEXT_SWAIT_2SKILLCOUNT",
    "IDC_TEXT_SWAIT_3SKILLCOUNT",
];

/// One staged seat view: the wire member plus the champion mark; the
/// self seat carries no level (the wire sends no self nickname or
/// level — `self_seat` documents the local-only name).
struct StagedSeat {
    name: String,
    level: Option<i32>,
    team: u8,
    ready: bool,
    present: bool,
    locked: bool,
    champion: bool,
    /// `RoomMember.master`: the room creator's seat.
    master: bool,
    /// `RoomMember.playing_state` nonzero (the member is playing).
    playing: bool,
    /// The room names a song (`JoinedRoom.music_id` nonzero).
    has_song: bool,
}

/// Seat parents of the `SET 0x0C 0x00015000` seat template, in seat
/// order: `IDC_BUTTON_SWAIT_EXTEND1..8` (`0x16015022..29`) carry the
/// eight seat rects, and each one hosts one clone of the template's
/// 12 members (`docs/ui-loader.md` step 8: the group key is
/// `code & 0xFF000` = `0x15000`).
const WAITING_SEAT_PARENT_IDS: [&str; WAITING_SEATS] = [
    "IDC_BUTTON_SWAIT_EXTEND1",
    "IDC_BUTTON_SWAIT_EXTEND2",
    "IDC_BUTTON_SWAIT_EXTEND3",
    "IDC_BUTTON_SWAIT_EXTEND4",
    "IDC_BUTTON_SWAIT_EXTEND5",
    "IDC_BUTTON_SWAIT_EXTEND6",
    "IDC_BUTTON_SWAIT_EXTEND7",
    "IDC_BUTTON_SWAIT_EXTEND8",
];

/// A seat indicator body of the seat template and the live flag that
/// arms it. Bodies with no pinned flag stay dark (see
/// [`seat_indicator`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeatIndicator {
    /// `Wait_ReadyWindow.ojs`: the seat's name plate, one frame per
    /// team colour (eight layers = teams A–H).
    TeamPlate,
    /// `Wait_BossMark.ojs`: the master's star.
    MasterStar,
    /// `Wait_ReadyText.ojs`: the ready plate.
    ReadyPlate,
    /// `Wait_NoMusicImage.ojs`: the room names no song yet.
    NoSong,
    /// `Wait_Playing.ojs`: the member is playing.
    Playing,
}

impl SeatIndicator {
    /// Body name of this indicator.
    const fn sprite(self) -> &'static str {
        match self {
            SeatIndicator::TeamPlate => "Wait_ReadyWindow.ojs",
            SeatIndicator::MasterStar => "Wait_BossMark.ojs",
            SeatIndicator::ReadyPlate => "Wait_ReadyText.ojs",
            SeatIndicator::NoSong => "Wait_NoMusicImage.ojs",
            SeatIndicator::Playing => "Wait_Playing.ojs",
        }
    }

    /// Icon-control frame for this seat (frame *n* paints layer *n−1*;
    /// the setter clamps, see [`icon_frame_layer`]).
    fn frame(self, seat: &StagedSeat) -> Option<u8> {
        if !seat.present {
            return None;
        }
        match self {
            SeatIndicator::TeamPlate => Some(seat.team.saturating_add(1)),
            SeatIndicator::MasterStar => seat.master.then_some(1),
            SeatIndicator::ReadyPlate => seat.ready.then_some(1),
            SeatIndicator::NoSong => (!seat.has_song).then_some(1),
            SeatIndicator::Playing => seat.playing.then_some(1),
        }
    }
}

/// Indicator body behind one seat-template sprite name. Bodies with no
/// pinned flag return `None` and stay dark:
///
/// - `Wait_PCRoomMark.ojs` — PC-room badge; the wire carries no
///   matching field (OPEN: find the flag the exe reads).
/// - `Wait_Downloding.ojs` / `Wait_Downloding_Bar.ojs` — song download
///   progress; the shell models no download (OPEN).
/// - `Waiting_P2P_Image.ojs` — P2P versus banner; needs the room's
///   venue class (OPEN).
/// - `Wait_NoGemImage.ojs` — gem-less marker; `RoomMember.gem` exists,
///   but which value arms the plate is untraced (OPEN).
fn seat_indicator(sprite: &str) -> Option<SeatIndicator> {
    [
        SeatIndicator::TeamPlate,
        SeatIndicator::MasterStar,
        SeatIndicator::ReadyPlate,
        SeatIndicator::NoSong,
        SeatIndicator::Playing,
    ]
    .into_iter()
    .find(|indicator| indicator.sprite() == sprite)
}

/// Seat index owning one staged clone (`None` for other placements).
fn seat_clone_index(placed: &ScenePlacement) -> Option<usize> {
    if placed.role != PlacementRole::Clone {
        return None;
    }
    WAITING_SEAT_PARENT_IDS
        .iter()
        .position(|id| *id == placed.id)
}

/// Whether one seat-template clone reaches the frame: occupied seats
/// show the armed indicators, vacant and locked seats keep their bare
/// cover plate (the exe clones the template per seated player only).
fn waiting_seat_clone_live(placed: &ScenePlacement, seats: &[StagedSeat]) -> bool {
    let Some(index) = seat_clone_index(placed) else {
        return true;
    };
    let Some(seat) = seats.get(index) else {
        return false;
    };
    if !seat.present {
        return false;
    }
    let Some(indicator) = placed.sprite.as_deref().and_then(seat_indicator) else {
        return false;
    };
    indicator.frame(seat).is_some()
}

/// Paint every armed seat indicator at its clone rect with the seat's
/// frame. The clone base pass already painted frame 0 there, so this
/// pass owns the live frame; unarmed bodies were filtered out above.
fn paint_waiting_seat_marks(frame: &mut Frame<'_>, seats: &[StagedSeat], art: &SceneArt) {
    for placed in &art.placements {
        let Some(index) = seat_clone_index(placed) else {
            continue;
        };
        let Some(seat) = seats.get(index) else {
            continue;
        };
        let Some(indicator) = placed.sprite.as_deref().and_then(seat_indicator) else {
            continue;
        };
        let (Some(figure_name), Some(icon_frame)) =
            (placed.sprite.as_deref(), indicator.frame(seat))
        else {
            continue;
        };
        let Some(figure) = art.figures.get(figure_name) else {
            continue;
        };
        let Some(layer) = icon_frame_layer(icon_frame, figure.len()).and_then(|i| figure.layer(i))
        else {
            continue;
        };
        let dest = scale_rect_to_frame(&placed.rect, frame.w, frame.h);
        blit_rgb555(
            frame,
            &layer.image.view(),
            dest.x + layer.dx,
            dest.y + layer.dy,
            Some(layer.colorkey),
        );
    }
}

/// Staged seats in cover order: self first, then the seven wire slots
/// (`JoinedRoom.slots`), padded empty — the same eight seats as
/// [`waiting_seats`], with levels and the champion mark kept.
fn staged_seats(room: &JoinedRoom) -> Vec<StagedSeat> {
    let mut seats = Vec::with_capacity(WAITING_SEATS);
    let own = room.slots.iter().find(|slot| slot.index == room.member_id);
    seats.push(StagedSeat {
        name: "YOU".to_owned(),
        level: None,
        team: room.team,
        ready: own
            .and_then(|slot| slot.member.as_ref())
            .is_some_and(|member| member.ready),
        present: true,
        locked: false,
        champion: room.champion == Some(0),
        master: room.member_id == 0,
        playing: own
            .and_then(|slot| slot.member.as_ref())
            .is_some_and(|member| member.playing_state != 0),
        has_song: room.music_id != 0,
    });
    for (offset, slot) in room.slots.iter().take(WAITING_PEERS).enumerate() {
        let index = u8::try_from(offset + 1).unwrap_or(u8::MAX);
        seats.push(staged_peer_seat(slot, room.champion == Some(index)));
    }
    while seats.len() < WAITING_SEATS {
        seats.push(empty_staged_seat());
    }
    seats
}

/// Staged view of one wire slot.
fn staged_peer_seat(slot: &RoomSlot, champion: bool) -> StagedSeat {
    match &slot.member {
        Some(member) => StagedSeat {
            name: member.nickname.clone(),
            level: Some(member.level),
            team: member.team,
            ready: member.ready,
            present: true,
            locked: false,
            champion,
            master: member.master,
            playing: member.playing_state != 0,
            has_song: !member.music_ids.is_empty(),
        },
        None if slot.state_code == RoomSlotState::Locked as i32 => StagedSeat {
            name: String::new(),
            level: None,
            team: 0,
            ready: false,
            present: false,
            locked: true,
            champion: false,
            master: false,
            playing: false,
            has_song: false,
        },
        None => empty_staged_seat(),
    }
}

/// Vacant staged seat.
fn empty_staged_seat() -> StagedSeat {
    StagedSeat {
        name: String::new(),
        level: None,
        team: 0,
        ready: false,
        present: false,
        locked: false,
        champion: false,
        master: false,
        playing: false,
        has_song: false,
    }
}

/// Role prefix of one seat: the champion seat names `CHAMPION`, every
/// other occupied seat `CHALLENGER` (the reference 1v1 pair); rooms
/// without a champion mark name no roles (normal rooms carry none).
fn staged_role(has_champion: bool, seat: &StagedSeat) -> Option<&'static str> {
    if !has_champion || !seat.present {
        return None;
    }
    Some(if seat.champion {
        "CHAMPION"
    } else {
        "CHALLENGER"
    })
}

/// One staged seat line: role plus team plus `Lv` (unknown levels
/// show `-`: the self seat carries no wire level) plus name plus the
/// ready flag — the legacy row shape, role-prefixed.
fn staged_seat_text(seat: &StagedSeat, role: Option<&str>) -> String {
    let flag = if seat.ready { 'R' } else { '-' };
    let level = seat.level.map_or("-".to_owned(), |level| level.to_string());
    let team = team_letter(seat.team);
    match role {
        Some(role) => format!("{role} {team} Lv:{level} {} {flag}", seat.name),
        None => format!("{team} Lv:{level} {} {flag}", seat.name),
    }
}

/// Scaled frame origin of one waiting control (`None` while the staged
/// art names no such control — the exe leaves untouched icons alone).
fn waiting_bnd_origin(
    placements: &[ScenePlacement],
    id: &str,
    frame: &Frame<'_>,
) -> Option<(i32, i32)> {
    let placement = placements.iter().find(|placed| placed.id == id)?;
    let scaled = scale_rect_to_frame(&placement.rect, frame.w, frame.h);
    Some((scaled.x, scaled.y))
}

/// True while one cover binds the staged cover body (or a
/// sprite-less slot, as in synthetic probes): any other body on the
/// cover id stays untouched, like the exe's unmatched icons.
fn cover_stages_body(placements: &[ScenePlacement], cover_id: &str) -> bool {
    placements
        .iter()
        .find(|placed| placed.id == cover_id)
        .is_none_or(|placed| {
            placed
                .sprite
                .as_deref()
                .is_none_or(|sprite| sprite == WAIT_COVER_SPRITE)
        })
}

/// True while staged art binds at least one seat cover: the staged
/// path anchors to verbatim `.bnd` rects, otherwise the legacy grid
/// keeps the bare scene readable.
fn waiting_has_staged_covers(placements: &[ScenePlacement]) -> bool {
    WAITING_COVER_IDS
        .iter()
        .any(|id| placements.iter().any(|placed| &placed.id == id))
}

/// Staged waiting regions, each at its verbatim `.bnd` rect: seat
/// covers, team highlight, title, wins, speed, skills, skill counts,
/// map name, BPM, and song (the song caption resolves through the
/// staged catalog, `M{id}` while it names no such song; the BPM
/// caption resolves through its BPM column, `-0-` on mismatch).
/// `INSTNAME` (`TEXT_SWAIT_INSTNAME`, `0x16400019`) stays unpainted:
/// zero code immediates name it (OPEN 1), so no shell source reaches
/// its rect. `NOTICE_TEXT`/`NOTICEBOARD` (`0x16500062/63`) stay
/// unpainted: the text body is imageless and its dialog `+0x46c`
/// fill is untraced (OPEN 2).
fn paint_waiting_staged(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
    captions: &WaitingCaptions<'_>,
) {
    paint_waiting_title(frame, room, placements);
    paint_waiting_covers(frame, room, placements);
    paint_waiting_team_highlight(frame, room, placements);
    paint_waiting_wins(frame, room, placements);
    paint_waiting_speed(frame, room, placements);
    paint_waiting_skills(frame, room, placements);
    paint_waiting_skillcounts(frame, &captions.skill_counts, placements);
    paint_waiting_mapname(frame, room, placements);
    paint_waiting_bpm(frame, captions.song_bpm, placements);
    paint_waiting_music(frame, room, placements, captions.song_title);
}

/// Room title at the `ROOMNAME` rect (top bar, rect 21).
fn paint_waiting_title(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_ROOMNAME_ID, frame) {
        paint_room_detail_line(frame, x, y, &room.title, art_title_ink());
    }
}

/// Seat lines at the cover rects (rects 60..67): champion names its
/// role and wins box, the challenger its role; empty covers stay
/// untouched so the staged empty panels show (reference bottom row).
/// Locked covers name `LOCK` (their art carries no lock body).
fn paint_waiting_covers(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    let has_champion = room.champion.is_some();
    for (cover_id, seat) in WAITING_COVER_IDS.iter().zip(staged_seats(room)) {
        if !cover_stages_body(placements, cover_id) {
            continue;
        }
        let Some((x, y)) = waiting_bnd_origin(placements, cover_id, frame) else {
            continue;
        };
        if seat.locked {
            paint_room_detail_line(frame, x, y, "LOCK", art_title_ink());
        } else if seat.present {
            let role = staged_role(has_champion, &seat);
            paint_room_detail_line(frame, x, y, &staged_seat_text(&seat, role), art_title_ink());
            paint_cover_ready_dot(frame, x, y, seat.ready);
        }
    }
}

/// Ready dot beside one cover line (the `Wait_ReadyText.ojs` plate is
/// staged art; the dot is the live flag, same inks as the lobby).
fn paint_cover_ready_dot(frame: &mut Frame<'_>, x: i32, y: i32, ready: bool) {
    fill_rect(
        frame,
        &Rect {
            x: x - 6,
            y,
            w: 3,
            h: 5,
        },
        ready_dot_color(ready),
    );
}

/// Team highlight at our team button rect (rects 3..10): the exe
/// highlight arm (`FUN_0048c810`) lights exactly the pressed button;
/// the amber bar is the live mark over the staged `Wait_Team?.ojs`.
fn paint_waiting_team_highlight(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
) {
    let index = usize::from(room.team);
    let (Some(button), Some(expected)) =
        (WAITING_TEAM_IDS.get(index), WAIT_TEAM_SPRITES.get(index))
    else {
        return;
    };
    let staged = placements.iter().find(|placed| &placed.id == button);
    if staged.is_none_or(|placed| placed.sprite.as_deref() == Some(*expected)) {
        if let Some((x, y)) = waiting_bnd_origin(placements, button, frame) {
            paint_cursor(frame, x + 6, y);
        }
    }
}

/// Champion win count at the `WINNERSCORE` rect (rect 108, the Wins
/// box): the wire streak only (`JoinedRoom.champion_streak`); past
/// it nothing paints (per-member tallies stay server-side).
fn paint_waiting_wins(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    let Some(streak) = room.champion_streak else {
        return;
    };
    if room.champion.is_none() {
        return;
    }
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_WINNERSCORE_ID, frame) {
        paint_room_detail_line(frame, x, y, &streak.to_string(), art_title_ink());
    }
}

/// Scroll-speed label at the `SPEED_NUM` rect (rect 101): the shared
/// `X0.5`..`XR` chain (`room-enums.md`, codes 0..11).
fn paint_waiting_speed(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_SPEED_NUM_ID, frame) {
        paint_room_detail_line(frame, x, y, &room_speed_label(room.speed), art_title_ink());
    }
}

/// Skill-ring ids at the `1/2/3SKILL` rects (rects 87..89, the X
/// boxes): empty slots stay untouched so the staged X art shows.
/// The counts paint beside them at the `SKILLCOUNT` rects from the
/// staged inventory ([`paint_waiting_skillcounts`]).
fn paint_waiting_skills(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    for (slot, skill_id) in WAITING_SKILL_IDS.iter().zip(room.skills.iter()) {
        if let Some((x, y)) = waiting_bnd_origin(placements, slot, frame) {
            paint_room_detail_line(frame, x, y, &skill_id.to_string(), art_title_ink());
        }
    }
}

/// Map-cycle names behind the `MAPNAME` caption, in room-word order
/// (map-cycle path, rdata file `0x249990`): the exe centers entry
/// `low == 0 ? 3 : low % 4` of (`None`, `한가한대전장의오후`,
/// `패자의역습`, `원하지않은결말`). The CP949-sourced Korean entries
/// paint their real glyphs now that the embedded Noto Sans KR subset
/// carries Hangul (formerly ASCII stand-ins — the micro-font had no
/// CP949 glyphs). The `0x4ad350` applier `Random`/`NoteFactory` arm
/// stays OPEN (`docs/caption-sources.md` OPEN 3) and never paints.
const WAITING_MAP_CYCLE_NAMES: [&str; 4] =
    ["None", "한가한대전장의오후", "패자의역습", "원하지않은결말"];

/// Map-cycle slot for one room word (files `0xb3cc6`–`0xb3ce3`):
/// the low word, with word 0 selecting slot 3. Byte access keeps
/// the low word without a narrowing cast (PERF-07).
#[must_use]
fn waiting_map_cycle_slot(arena: i32) -> usize {
    let bytes = arena.to_le_bytes();
    let low = u16::from_le_bytes([bytes[0], bytes[1]]);
    if low == 0 {
        WAITING_MAP_CYCLE_NAMES.len() - 1
    } else {
        usize::from(low) % WAITING_MAP_CYCLE_NAMES.len()
    }
}

/// `MAPNAME` caption for one room word: the map-cycle table entry
/// (pinned path only).
#[must_use]
pub fn waiting_map_name(arena: i32) -> &'static str {
    WAITING_MAP_CYCLE_NAMES
        .get(waiting_map_cycle_slot(arena))
        .copied()
        .unwrap_or(WAITING_MAP_CYCLE_NAMES[0])
}

/// Map name at the `MAPNAME` rect: the map-cycle entry of the room
/// word. An unstaged rect stays untouched.
fn paint_waiting_mapname(frame: &mut Frame<'_>, room: &JoinedRoom, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_MAPNAME_ID, frame) {
        paint_room_detail_line(frame, x, y, waiting_map_name(room.arena), art_title_ink());
    }
}

/// Mismatch BPM caption (rdata file `0x24a1d0`): the exe stores
/// `-0-` when no song record matches the room song.
const WAITING_BPM_MISMATCH: &str = "-0-";

/// `BPM` caption for one staged catalog lookup: the `OJNList.dat`
/// row +20 BPM column formatted with `%0.f` (rdata file `0x24a1d4`,
/// store file `0xbe1d5`); a missing or non-finite lookup falls back
/// to the pinned mismatch default, never an invented tempo.
#[must_use]
pub fn waiting_bpm_caption(song_bpm: Option<f32>) -> String {
    match song_bpm {
        Some(bpm) if bpm.is_finite() => format!("{bpm:.0}"),
        _ => WAITING_BPM_MISMATCH.to_owned(),
    }
}

/// BPM at the `BPM` rect: the catalog column or `-0-`. An unstaged
/// rect stays untouched.
fn paint_waiting_bpm(frame: &mut Frame<'_>, song_bpm: Option<f32>, placements: &[ScenePlacement]) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_BPM_ID, frame) {
        paint_room_detail_line(frame, x, y, &waiting_bpm_caption(song_bpm), art_title_ink());
    }
}

/// `SKILLCOUNT` caption for one inventory slot: the count word at
/// owned-item vector `+8` as decimal, space-padded to width 3
/// (helper `0x62c109`); slots 2–3 additionally clamp at 999, so
/// wider counts show `999` there.
#[must_use]
pub fn waiting_skillcount_text(count: u32, cap_at_999: bool) -> String {
    let shown = if cap_at_999 && count > 999 {
        999
    } else {
        count
    };
    format!("{shown:>3}")
}

/// Skill counts at the `1/2/3SKILLCOUNT` rects: the staged inventory
/// counts beside the rings (slot 1 uncapped, slots 2–3 capped at
/// 999). An unstaged rect stays untouched.
fn paint_waiting_skillcounts(
    frame: &mut Frame<'_>,
    counts: &[u32; 3],
    placements: &[ScenePlacement],
) {
    for (slot, (id, count)) in WAITING_SKILLCOUNT_IDS.iter().zip(counts.iter()).enumerate() {
        if let Some((x, y)) = waiting_bnd_origin(placements, id, frame) {
            paint_room_detail_line(
                frame,
                x,
                y,
                &waiting_skillcount_text(*count, slot > 0),
                art_title_ink(),
            );
        }
    }
}

/// Selected-song label at the `MUSICNAME` rect (rect 22, top bar):
/// the staged catalog title for the room's song when it names one
/// (room-fill string flow, `docs/channel-fill.md`), else the bare
/// song id (the song table lives server-side, as in the lobby).
/// The `COVERMUSIC` button, the select arrow, the `Random` cards, and
/// BACK stay staged art — recorded negatives, not gaps (chat now has a
/// live source: [`paint_waiting_chat`]):
///
/// - Random cards: the 117-entry `STATE_WAITING` block names no per-card
///   entries; the big `?` card sits on `RANDMAP` (`0x1650004F` → rect 78)
///   while the small `?` cards sit on the spriteless `EFFECT_1/2` rects
///   (68/69) — caption and frames baked into the staged backdrop.
///   `SelectMusic_Random*.ojs` bodies belong to `DAILOG_SELECTMUSIC`
///   filter buttons, not to waiting cards, and no wire struct carries
///   card picks — so the painter stages art only.
fn paint_waiting_music(
    frame: &mut Frame<'_>,
    room: &JoinedRoom,
    placements: &[ScenePlacement],
    song_title: Option<&str>,
) {
    if let Some((x, y)) = waiting_bnd_origin(placements, WAITING_MUSICNAME_ID, frame) {
        paint_room_detail_line(
            frame,
            x,
            y,
            &waiting_music_caption(room.music_id, song_title),
            art_title_ink(),
        );
    }
}

/// `MUSICNAME` caption for one room song: the staged catalog title
/// when it names the song, else the bare `M{id}` id (never an
/// invented title — `INSTNAME` stays unpainted for the same reason:
/// no staged source names it).
#[must_use]
fn waiting_music_caption(music_id: u16, song_title: Option<&str>) -> String {
    match song_title {
        Some(title) if !title.is_empty() => title.to_owned(),
        _ => room_music_label(music_id),
    }
}

/// Chat history view (`EDIT_SWAIT_CHAT 0x16300014` → rect 19,
/// `LIST_SWAIT_CHAT 0x1680002B` → rect 42, row template
/// `TEXT_SWAIT_CHAT 0x1640003C` under `SET 0x00031000`): one live row
/// per decoded 3012/3013 packet, fed through [`paint_waiting_chat`].
/// Rows come from live packets only — no history is invented.
const WAITING_CHAT_LIST_ID: &str = "IDC_LIST_SWAIT_CHAT";
/// Chat input line (`EDIT_SWAIT_CHAT` → rect 19).
const WAITING_CHAT_EDIT_ID: &str = "IDC_EDIT_SWAIT_CHAT";
/// Native history rect while no staged art names it. File-traced:
/// `State_Waiting.bnd` rect 42 `(24,455)-(511,553)` = `(24,455,487,98)`,
/// paired to `IDC_LIST_SWAIT_CHAT 0x1680002B` by ControlList code
/// ordinal (`code & 0xFFF` = 43 → ordinal 42) — geometry-audit.md
/// Waiting row, verified against `Interface1.opi`.
const WAITING_CHAT_LIST_RECT: Rect = Rect {
    x: 24,
    y: 455,
    w: 487,
    h: 98,
};
/// Native input rect while no staged art names it. File-traced:
/// `State_Waiting.bnd` rect 19 `(20,563)-(460,583)` = `(20,563,440,20)`,
/// paired to `IDC_EDIT_SWAIT_CHAT 0x16300014` by ControlList code
/// ordinal (`code & 0xFFF` = 20 → ordinal 19) — geometry-audit.md
/// Waiting row, verified against `Interface1.opi`.
const WAITING_CHAT_EDIT_RECT: Rect = Rect {
    x: 20,
    y: 563,
    w: 440,
    h: 20,
};
/// Row pitch of the history box, EXE-MEASURED 2026-11-09 (live rig
/// session, six 3011 rows in `~/scratch/geom-open-close/t1-chat-box.png`):
/// glyph tops at 456+17k, five 17 px intervals exact. (The earlier
/// photo-derived pitch agreed at 17.) Only the newest 5 rows render.
const WAITING_CHAT_ROW_H: i32 = 17;
/// Rows visible in the history box: EXE-OBSERVED 5 — the 98 px `LIST`
/// rect fits five 17 px rows and the sixth slot stays empty (rig
/// session 2026-11-09, `t1-chat-box.png`); was a 6-row derivation.
const WAITING_CHAT_VISIBLE_ROWS: usize = 5;
/// Focus-caret inset inside the input rect, EXE-MEASURED (rig session
/// 2026-11-09, `t1-waiting-empty.png`; capture before any canvas click
/// — the caret hides when the edit loses focus): caret ink bbox
/// (23,570) inside `EDIT_SWAIT_CHAT` rect 19 `(20,563,460,583)`.
const WAITING_CHAT_CARET_INSET: (i32, i32) = (3, 7);
/// Focus-caret bar size, EXE-MEASURED: solid core 1 px at x=24, rows
/// 570..578 (rig session 2026-11-09, same capture).
const WAITING_CHAT_CARET_SIZE: (u32, u32) = (2, 9);

/// One waiting-room chat row from a live 3012/3013 packet
/// (`ScreenGame` converts `WaitingUserMessage`/`WaitingAdminMessage`
/// into this display form; the painter never decodes the wire).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingChatMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text (UTF-8, lossy-decoded from the raw wire bytes).
    pub text: String,
    /// True for 3013 administrator lines — selects the `0x71e6` ink
    /// over the 3012 `0xdedede` (see [`chat_admin_ink`]).
    pub admin: bool,
}

impl WaitingChatMessage {
    /// Player chat row (3012).
    #[must_use]
    pub fn user(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: false,
        }
    }

    /// Administrator chat row (3013).
    #[must_use]
    pub fn admin(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: true,
        }
    }
}

/// Display text of one chat row: the enqueue printf's `[%12s] %s`
/// line (format VA `0x64d8d4`, call file `0x15f076`,
/// `docs/channel-fill.md`) — bracket, sender right-aligned to width
/// 12, bracket, space, text. User and administrator rows share the
/// line; the kind only picks the appended ink. ASCII names pad the
/// field exactly like the exe's C runtime; wider multibyte names ride
/// the presentation stand-in (rasterizer owner decision). The former
/// `[ADMIN]` prefix and the photo-guessed 13-wide field are replaced
/// by this pinned format.
#[must_use]
pub fn waiting_chat_row_text(message: &WaitingChatMessage) -> String {
    format!("[{:>12}] {}", message.sender, message.text)
}

/// Rows the history box shows: the newest [`WAITING_CHAT_VISIBLE_ROWS`].
#[must_use]
pub fn waiting_chat_visible_messages(messages: &[WaitingChatMessage]) -> &[WaitingChatMessage] {
    let skip = messages.len().saturating_sub(WAITING_CHAT_VISIBLE_ROWS);
    messages.get(skip..).unwrap_or(&[])
}

/// Ink of 3012 user rows: the exe poller appends with color
/// `0xdedede` (call file `0xb9f8c`, `docs/channel-fill.md`) through
/// the GDI `SetTextColor` path (`docs/gdi-text.md`) — COLORREF
/// `0x00BBGGRR`, symmetric here: light gray.
fn chat_user_ink() -> u32 {
    pack_xrgb(0xDE, 0xDE, 0xDE)
}

/// Ink of 3013 administrator rows: color `0x71e6` (call file
/// `0xb9fbf`) as a COLORREF — red `0xe6`, green `0x71`, blue `0x00`,
/// the warm amber every reference row reads as.
fn chat_admin_ink() -> u32 {
    pack_xrgb(0xE6, 0x71, 0x00)
}

/// Ink of one history row: the 3013 gate picks the admin amber,
/// every other row the user gray.
fn chat_row_ink(admin: bool) -> u32 {
    if admin {
        chat_admin_ink()
    } else {
        chat_user_ink()
    }
}

/// Frame y of history row `offset`: the list origin plus
/// [`WAITING_CHAT_ROW_H`] steps (photo-pinned pitch).
#[must_use]
fn waiting_chat_row_y(list_y: i32, offset: usize) -> i32 {
    list_y.saturating_add(
        i32::try_from(offset)
            .unwrap_or(i32::MAX)
            .saturating_mul(WAITING_CHAT_ROW_H),
    )
}

/// Frame origin of the history box: the staged `LIST` rect while bound,
/// else the scaled native rect 42.
fn waiting_chat_list_origin(frame: &Frame<'_>, placements: &[ScenePlacement]) -> (i32, i32) {
    if let Some(origin) = waiting_bnd_origin(placements, WAITING_CHAT_LIST_ID, frame) {
        return origin;
    }
    let scaled = scale_rect_to_frame(&WAITING_CHAT_LIST_RECT, frame.w, frame.h);
    (scaled.x, scaled.y)
}

/// Frame origin of the input line: the staged `EDIT` rect while bound,
/// else the scaled native rect 19.
fn waiting_chat_edit_origin(frame: &Frame<'_>, placements: &[ScenePlacement]) -> (i32, i32) {
    if let Some(origin) = waiting_bnd_origin(placements, WAITING_CHAT_EDIT_ID, frame) {
        return origin;
    }
    let scaled = scale_rect_to_frame(&WAITING_CHAT_EDIT_RECT, frame.w, frame.h);
    (scaled.x, scaled.y)
}

/// Paint live chat rows plus the input line: the newest visible rows
/// stack from the history origin at the photo-pinned 17 px pitch,
/// straight on the box art (the reference rows carry no scrim band),
/// each in its appended ink — 3012 user rows `0xdedede`, 3013 admin
/// rows `0x71e6` ([`chat_row_ink`]) — the draft paints at the input
/// origin and the focus caret rides the insertion point (inset plus
/// typed width — the reference shows the caret on an empty input).
/// Rows come only from live packets; the caret is the input-line
/// state, not history. Missing placements fall back to the scaled
/// native rects so the flat fallback still reads.
pub fn paint_waiting_chat(
    frame: &mut Frame<'_>,
    messages: &[WaitingChatMessage],
    input: &str,
    placements: &[ScenePlacement],
) {
    let (list_x, list_y) = waiting_chat_list_origin(frame, placements);
    for (offset, message) in waiting_chat_visible_messages(messages).iter().enumerate() {
        draw_text(
            frame,
            list_x,
            waiting_chat_row_y(list_y, offset),
            &waiting_chat_row_text(message),
            chat_row_ink(message.admin),
        );
    }
    let (edit_x, edit_y) = waiting_chat_edit_origin(frame, placements);
    let caret_x =
        edit_x + WAITING_CHAT_CARET_INSET.0 + i32::try_from(text_width(input)).unwrap_or(i32::MAX);
    let caret_y = edit_y + WAITING_CHAT_CARET_INSET.1;
    if !input.is_empty() {
        draw_text(frame, edit_x, edit_y, input, art_title_ink());
    }
    fill_rect(
        frame,
        &Rect {
            x: caret_x,
            y: caret_y,
            w: WAITING_CHAT_CARET_SIZE.0,
            h: WAITING_CHAT_CARET_SIZE.1,
        },
        art_title_ink(),
    );
}

/// Rows visible in the channel history box: the newest
/// [`CHANNEL_CHAT_VISIBLE_ROWS`], mirroring the waiting box.
const CHANNEL_CHAT_VISIBLE_ROWS: usize = 6;
/// Channel chat rows share the lobby list pitch (8 px).
const CHANNEL_CHAT_ROW_H: i32 = 8;

/// One channel chat row from a live 2013/2014/2016/2019/2029 packet
/// (`ScreenGame` converts the wire message into this display form;
/// the painter never decodes the wire). Announcements (2016) carry
/// notice text with no sender; rank notices (2029) bracket the
/// ranked nickname around the song-plus-rank text; both ride the
/// admin amber like 2014 rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelChatMessage {
    /// Chatter nickname.
    pub sender: String,
    /// Chat text (UTF-8, lossy-decoded from the raw wire bytes).
    pub text: String,
    /// True for 2014 administrator lines (the 3013 `0x71e6` ink).
    pub admin: bool,
    /// Whisper addressee for 2019 rows (`None` for channel lines).
    pub whisper_recipient: Option<String>,
}

impl ChannelChatMessage {
    /// Player chat row (2013).
    #[must_use]
    pub fn user(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: false,
            whisper_recipient: None,
        }
    }

    /// Administrator chat row (2014).
    #[must_use]
    pub fn admin(sender: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: true,
            whisper_recipient: None,
        }
    }

    /// Incoming whisper row (2019): the recipient is the local
    /// addressee, kept so the row names both ends.
    #[must_use]
    pub fn whisper(sender: &str, recipient: &str, text: &str) -> Self {
        Self {
            sender: sender.to_owned(),
            text: text.to_owned(),
            admin: false,
            whisper_recipient: Some(recipient.to_owned()),
        }
    }

    /// Lobby announcement row (2016): sender-less notice text in the
    /// admin amber — the exe fans the string out as `0x7de`
    /// admin-message rows (`docs/net-protocol.md`), so the painter
    /// inks it like a 2014 row and prints it bare (no bracket
    /// without a sender).
    #[must_use]
    pub fn announcement(text: &str) -> Self {
        Self {
            sender: String::new(),
            text: text.to_owned(),
            admin: true,
            whisper_recipient: None,
        }
    }

    /// Rank-notice row (2029): the pinned notice-board sentence
    /// ([`rank_notice_board_text`]) in the admin amber (the same
    /// `0x7de` family as the announcement rows). The nickname rides
    /// inside the sentence, so the row carries no sender and paints
    /// bare; callers pass `M{id}` while the staged catalog names no
    /// song.
    #[must_use]
    pub fn rank(nickname: &str, song_title: &str, ranking: i32) -> Self {
        Self {
            sender: String::new(),
            text: rank_notice_board_text(nickname, song_title, ranking),
            admin: true,
            whisper_recipient: None,
        }
    }
}

/// Notice-board text of one rank notice (2029) in the exe field order:
/// nickname, song title, ranking. The five board writers match the staged
/// music id against the song table, then format nickname plus resolved
/// title plus ranking through VA `0x647f98` (file `0x247f98`) — brackets
/// around the title and the rank, Korean particles between. The shell
/// stages no song table, so callers pass `M{id}` while it names no song.
#[must_use]
pub fn rank_notice_board_text(nickname: &str, song_title: &str, ranking: i32) -> String {
    format!("{nickname} 음악전사님께서 [{song_title}] 곡에 새로운 랭킹 [{ranking}]위가 되셨습니다~")
}

/// Display text of one channel row: the waiting `[%12s] %s` bracket
/// line for channel rows; whisper rows name both ends as
/// `[sender -> recipient] text` so the addressee stays visible;
/// sender-less rows (announcements and rank notices) paint their text
/// bare (no bracket without a sender).
#[must_use]
pub fn channel_chat_row_text(message: &ChannelChatMessage) -> String {
    if let Some(recipient) = message.whisper_recipient.as_deref() {
        return format!(
            "[{:>12} -> {:>12}] {}",
            message.sender, recipient, message.text
        );
    }
    if message.sender.is_empty() {
        return message.text.clone();
    }
    format!("[{:>12}] {}", message.sender, message.text)
}

/// Rows the channel history shows: the newest [`CHANNEL_CHAT_VISIBLE_ROWS`].
#[must_use]
pub fn channel_chat_visible_messages(messages: &[ChannelChatMessage]) -> &[ChannelChatMessage] {
    let skip = messages.len().saturating_sub(CHANNEL_CHAT_VISIBLE_ROWS);
    messages.get(skip..).unwrap_or(&[])
}

/// Paint live channel rows below the channel list: the newest visible
/// rows stack at the lobby 8 px pitch, straight on the scene (no scrim
/// band), each in its appended ink — 2014 admin rows, 2016
/// announcements, and 2029 rank notices amber, every other row user
/// gray ([`chat_row_ink`]). Rows come only from live packets; empty
/// histories paint nothing.
pub fn paint_channel_chat(
    frame: &mut Frame<'_>,
    messages: &[ChannelChatMessage],
    channel_rows: usize,
) {
    for (offset, message) in channel_chat_visible_messages(messages).iter().enumerate() {
        let step = i32::try_from(offset).unwrap_or(i32::MAX);
        let y = index_as_row(channel_rows, frame.h as i32)
            .saturating_add(CHANNEL_ROW_H)
            .saturating_add(step.saturating_mul(CHANNEL_CHAT_ROW_H));
        draw_text(
            frame,
            frame.w as i32 / 4,
            y,
            &channel_chat_row_text(message),
            chat_row_ink(message.admin),
        );
    }
}

/// Staged room-lobby chat list (`IDC_LIST_SROOM_CHAT 0x13800014`).
const ROOM_CHAT_LIST_CODE: u32 = 0x1380_0014;
/// Staged room-lobby chat input line (`IDC_EDIT_SROOM_CHAT 0x13300015`).
const ROOM_CHAT_EDIT_CODE: u32 = 0x1330_0015;
/// Row pitch of the room history box. The waiting box is exe-measured
/// at 17 px for the same CListBox-plus-`TEXT_*_CHAT` widget pair
/// ([`WAITING_CHAT_ROW_H`]); the room box is UNMEASURED, so this is a
/// labelled borrow (next step: rig capture of a live room chat list,
/// `~/scratch/planet-verify/` recipe).
const ROOM_CHAT_ROW_H: i32 = 17;
/// Focus-caret inset inside the room input rect: the waiting box's
/// exe-measured inset ([`WAITING_CHAT_CARET_INSET`]) borrowed for the
/// taller-by-one-pixel room edit (UNMEASURED, same next step).
const ROOM_CHAT_CARET_INSET: (i32, i32) = WAITING_CHAT_CARET_INSET;

/// Paint the room lobby chat panel (ticket #37): the newest history
/// rows stack from the staged `IDC_LIST_SROOM_CHAT` origin at
/// [`ROOM_CHAT_ROW_H`], each in its appended ink ([`chat_row_ink`]),
/// and the draft line inks the staged `IDC_EDIT_SROOM_CHAT` origin with
/// the focus caret at the borrowed inset. Both rects come from the
/// parsed staged block (owner rule `f38f3`): without them the painter
/// draws nothing (no fallback geometry is invented). Rows come only
/// from live packets.
pub fn paint_room_chat(
    frame: &mut Frame<'_>,
    messages: &[ChannelChatMessage],
    input: &str,
    placements: &[ScenePlacement],
) {
    let (frame_w, frame_h) = (frame.w, frame.h);
    let staged = |code: u32| {
        placements
            .iter()
            .find(|placed| placed.code == code)
            .map(|placed| scale_rect_to_frame(&placed.rect, frame_w, frame_h))
    };
    if let Some(list) = staged(ROOM_CHAT_LIST_CODE) {
        let pitch = u32::try_from(ROOM_CHAT_ROW_H).unwrap_or(1);
        let rows = usize::try_from(list.h / pitch).unwrap_or(0);
        let skip = messages.len().saturating_sub(rows);
        for (offset, message) in messages.get(skip..).unwrap_or(&[]).iter().enumerate() {
            let step = i32::try_from(offset).unwrap_or(i32::MAX);
            draw_text(
                frame,
                list.x,
                list.y.saturating_add(step.saturating_mul(ROOM_CHAT_ROW_H)),
                &channel_chat_row_text(message),
                chat_row_ink(message.admin),
            );
        }
    }
    let Some(edit) = staged(ROOM_CHAT_EDIT_CODE) else {
        return;
    };
    if !input.is_empty() {
        draw_text(frame, edit.x, edit.y, input, art_title_ink());
    }
    let caret_x = edit
        .x
        .saturating_add(ROOM_CHAT_CARET_INSET.0)
        .saturating_add(i32::try_from(text_width(input)).unwrap_or(i32::MAX));
    fill_rect(
        frame,
        &Rect {
            x: caret_x,
            y: edit.y.saturating_add(ROOM_CHAT_CARET_INSET.1),
            w: WAITING_CHAT_CARET_SIZE.0,
            h: WAITING_CHAT_CARET_SIZE.1,
        },
        art_title_ink(),
    );
}

/// Dark strip behind one seat row so seat ink stays readable over
/// staged art (vacant rows get one too: the placeholder must read).
fn paint_seat_scrim(frame: &mut Frame<'_>, x: i32, y: i32, text: &str) {
    let text_w = text_width(text) as i32;
    fill_rect(
        frame,
        &Rect {
            x: x - 8,
            y: y - 1,
            w: (text_w + 12).max(0) as u32,
            h: 7,
        },
        scrim_color(),
    );
}

/// Paint one seat row plus its ready dot (vacant seats have no dot).
fn paint_waiting_seat(frame: &mut Frame<'_>, index: usize, seat: &WaitingSeat) {
    let occupied = seat.display_name().is_some();
    let (x, y) = seat_origin(index, frame.w, frame.h);
    let row = waiting_row_text(index, seat);
    paint_seat_scrim(frame, x, y, &row);
    let ink = if occupied {
        pack_xrgb(0xFF, 0xFF, 0xFF)
    } else {
        pack_xrgb(0x70, 0x70, 0x70)
    };
    draw_text(frame, x, y, &row, ink);
    if occupied {
        fill_rect(
            frame,
            &Rect {
                x: x - 6,
                y,
                w: 3,
                h: 5,
            },
            ready_dot_color(seat.is_ready()),
        );
    }
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

/// Loading progress bar over a dark track (no background fill:
/// shared by the flat fallback and the art scene). The track rect
/// is an OPEN placeholder; only the fill fraction is live.
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

/// Paint the loading gate scene.
fn paint_loading(frame: &mut Frame<'_>) {
    fill_background(frame, Screen::Loading);
    paint_loading_bar(frame);
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
/// it. Then the fractional bar plus the song title/artist. The caption
/// now routes through `Cell::Loading` (the gate Trebuchet 27x65
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
    paint_loading_bar_with_progress(frame, status.clamped_progress());
    paint_loading_song(frame, status);
}

/// Paint staged loading art (`MusicLoading.ojs`) with the live
/// progress bar on top. Legacy indeterminate form (full bar, no
/// caption): `game.rs` carries no walk fraction or song names yet,
/// so callers with live state use [`paint_loading_scene_with_status`].
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

/// Battle verdict art mounted by the result builder `FUN_004c2390`
/// (file `0xc2390`): `StateResult_Top.ojs` plus
/// `StateResult_Bottom.ojs` plus `Result_WinLose.ojs` (file `0x24a35c`).
/// The result scene paints the `Result_WinLose.ojs` banner only
/// (frame from [`winlose_frame`], `FUN_004c3120`); the play-scene
/// `Winner_image.ojs` banner is a separate art — never re-read in
/// the result region (no rank or role visibility toggle exists on
/// its controls) and its entry rides unloaded in `STATE_PLAYING`
/// (decl `0xE6`, see [`WINNER_IMAGE_SPRITE`]). Winner is rank 1 by
/// team score only (`RoomTeams::winner`); team totals stay
/// server-side (per-member 4014/4016). The grade plate runs on the
/// pinned threshold table ([`grade_frame`]).
pub const RESULT_TOP_SPRITE: &str = "StateResult_Top.ojs";
/// Bottom half mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_BOTTOM_SPRITE: &str = "StateResult_Bottom.ojs";
/// Win/lose verdict mounted by `FUN_004c2390` (file `0xc2390`).
pub const RESULT_WINLOSE_SPRITE: &str = "Result_WinLose.ojs";
/// Winner banner from `STATE_PLAYING` (file `0x24b014`): control
/// `IDC_IMAGE_SPLAY_WINNER` (entry 230, code `0x1B500012`). The
/// decl (`0xE6`) loads entries 0–229 only, so the entry rides
/// unloaded and the sprite never stages in the Playing scene; its
/// `State_Playing_network.bnd` rect 18 (`(265,472)-(389,482)`) is
/// the exit-path variant's placement. Frames anchor `(264,454)`
/// 80×13 (both images; `Challenger_image.ojs` mirrors at
/// `(455,454)`). Kept as the documented-unstaged name — the
/// shell paints it nowhere (a never-staged name is pinned so a
/// future network-variant staging cannot collide silently).
pub const WINNER_IMAGE_SPRITE: &str = "Winner_image.ojs";
/// Grade plate bound by `STATE_RESULT` rect 7
/// (`IDC_IMAGE_SRESULT_RANKINGFIELD`, `Result_Imgs_Rank.ojs`): eight
/// frames, all anchored (395,531) — 35×41 ×2, 39×41 ×4, 38×41, 79×41
/// (dossier `ev_c61d99f8…`). Frame *n* paints image *n−1*, so grade
/// 1..=8 shows images 0..=7.
pub const RESULT_RANK_SPRITE: &str = "Result_Imgs_Rank.ojs";

/// Life-readout digit font mounted by `FUN_004c2390` at `this+0x10d10`
/// from the Playing archive (`Playing1.opi`): the life-readout
/// painter (VA `0x4c44c0`, file `0xc44c0`) loads this body for every
/// frame-select/draw pair (files `0xc45a6`/`0xc45ce`/`0xc45e7`/`0xc460c`,
/// dossier `ev_48c71a81…`). Ten layers in digit order 0–9, all 18 px
/// tall anchored (0,0) (`ev_a03451ed…`).
pub const RESULT_LIFE_FONT_SPRITE: &str = "PointNum.ojs";
/// Bottom-counter digit font mounted at `this+0x10d0c` from the
/// Interface archive (`Interface1.opi`): the bottom-counter painter
/// (VA `0x4c4630`, file `0xc4630`) loads this body at files
/// `0xc4716`/`0xc473e`/`0xc4757`/`0xc477c` (dossier `ev_63391623…`) —
/// its sole use site image-wide (`ev_8f5c69ce…`). Ten layers in digit
/// order 0–9, 13–14 px tall anchored (0,0).
pub const RESULT_BOTTOM_FONT_SPRITE: &str = "Result_NumFont_Blue.ojs";

/// Cell geometry of the two exe digit-sprite fields (both painters
/// build eight cells as `last − 0xa8 ..= last` in 0x18 (24 px) steps
/// and kern digit 1 by +5: `ev_d4362eae…`).
const RESULT_LIFE_Y: i32 = 77;
const RESULT_LIFE_LAST_X: i32 = 460;
const RESULT_BOTTOM_Y: i32 = 536;
const RESULT_BOTTOM_LAST_X: i32 = 176;
const RESULT_DIGIT_PITCH: i32 = 24;
const RESULT_DIGIT_CELLS: usize = 8;

/// Music-name box interior on the native canvas: the
/// `Result_EachResult.ojs` body (`STATE_RESULT` rect 4
/// `IDC_IMAGE_SRESULT_RESULTTAP`, 800×350 anchored (0,250)) bakes a
/// default mascot pane into its inner box, and the played song's
/// album art paints over exactly that interior — the reference photo's
/// green panel measures this rect (body-local interior (70,54)–
/// (733,268) plus the (0,250) anchor; the baked frame borders at
/// x 68–69/734–735 and y 52–53/269–270 stay art). The cover section
/// comes from the walked chart's `.ojn` (docs/file-formats/ojn.md); a
/// missing cover leaves the baked box as-is.
pub const RESULT_MUSIC_BOX: Rect = Rect {
    x: 70,
    y: 304,
    w: 664,
    h: 215,
};

/// Sprites whose frames the result painter selects one of (the
/// verdict banner plus the grade plate): kept out of the whole-stack
/// backdrop pass so unused frames never bleed through, then painted
/// at the exe-selected frame only.
const RESULT_FRAME_BODIES: [&str; 2] = [RESULT_WINLOSE_SPRITE, RESULT_RANK_SPRITE];

/// Sprites the result builder `FUN_004c2390` (file `0xc2390`) mounts,
/// in load order: top, bottom, then the win/lose verdict.
#[must_use]
pub fn result_loader_sprites() -> [&'static str; 3] {
    [
        RESULT_TOP_SPRITE,
        RESULT_BOTTOM_SPRITE,
        RESULT_WINLOSE_SPRITE,
    ]
}

/// Battle verdict: team standing by server-side total (`RoomTeams`
/// mirrors the 4014/4016 accumulation; `Winner_image.ojs` and
/// `Result_WinLose.ojs` render this same comparison server-side).
/// Solo or team, grades play no part (mapping OPEN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleVerdict {
    /// Local team holds rank 1 (exe win-flag 1 → frame 1 → image 0).
    Win,
    /// Local team holds rank 2 (exe win-flag 2 → frame 2 → image 1,
    /// the LOSE! banner of the reference battle photo).
    Lose,
    /// Local team ranks third or worse (exe win-flag anything else
    /// → frame 3 → image 2).
    Other,
}

/// Distinct seated teams in seat order (each `TEAMx` button index once).
#[must_use]
fn seated_battle_teams(teams: &RoomTeams) -> Vec<u8> {
    let mut seen = Vec::new();
    for seat in 0..teams.seats() {
        if let Some(team) = teams.team_of(seat) {
            if !seen.contains(&team) {
                seen.push(team);
            }
        }
    }
    seen
}

/// Zero-based standing of `team` among the seated teams: highest
/// combined total first, ties toward the lower team id (the
/// `RoomTeams::winner` first-seat-wins order). `None` while `team`
/// holds no seat.
#[must_use]
fn battle_team_rank(teams: &RoomTeams, team: u8) -> Option<usize> {
    let mut standings: Vec<(u64, u8)> = seated_battle_teams(teams)
        .into_iter()
        .map(|candidate| (teams.team_total(candidate), candidate))
        .collect();
    standings.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
    standings.iter().position(|(_, seated)| *seated == team)
}

/// Standing verdict for the local team: rank 1 wins, rank 2 loses,
/// rank 3+ falls to the generic other arm. `None` while the local
/// team is unknown or unseated (empty room, solo with no seat claimed).
#[must_use]
pub fn battle_verdict(teams: &RoomTeams, self_team: Option<u8>) -> Option<BattleVerdict> {
    match battle_team_rank(teams, self_team?)? {
        0 => Some(BattleVerdict::Win),
        1 => Some(BattleVerdict::Lose),
        _ => Some(BattleVerdict::Other),
    }
}

/// Result verdict state the banner painter maps to a WinLose frame
/// (`FUN_004c3120`): solo plays take the solo banner, battles the
/// win-flag frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultVerdict {
    /// Solo play (the exe's player-count branch): frame 4, the solo
    /// banner image.
    Solo,
    /// Team battle with the resolved rank-1 verdict, if any (`None`
    /// while the winner or the local team is unknown — no banner,
    /// the exe's observer frame-0 arm being OPEN, dossier note 10).
    Battle(Option<BattleVerdict>),
}

/// WinLose frame for one verdict (`FUN_004c3120`, dossier
/// `ev_38f32576…`): solo draws frame 4; a battle win-flag byte of 1
/// draws frame 1, 2 draws frame 2, anything else frame 3. The shell
/// resolves the same standing from team totals (rank 1 → `Win`,
/// rank 2 → `Lose`, rank 3+ → `Other`), so every dossier frame is
/// reachable. Frame 4 clamps to the fourth image at draw time like
/// the exe setter.
#[must_use]
pub fn winlose_frame(verdict: ResultVerdict) -> Option<u8> {
    match verdict {
        ResultVerdict::Solo => Some(4),
        ResultVerdict::Battle(Some(BattleVerdict::Win)) => Some(1),
        ResultVerdict::Battle(Some(BattleVerdict::Lose)) => Some(2),
        ResultVerdict::Battle(Some(BattleVerdict::Other)) => Some(3),
        ResultVerdict::Battle(None) => None,
    }
}

/// Grade plate frame chooser (`FUN_004c4dd0`), re-exported from
/// `o2jam-game` where the pinned rule lives next to the ratio and
/// promotion arms it consumes.
pub use o2jam_game::grade_frame;

/// Grade LETTERS stay OPEN: the exe grade strings (`ALL COMBO`,
/// `COOL 95%`, …) carry zero `.text` references (full-image LE32
/// scan, `docs/dialog-strings.md`) — result grades are `.ojs`-driven
/// (the plate frames above), so no letter mapping is painted and
/// the select-music GRADE cell stays blank. Named next step: a
/// live rig capture of the grade plate at controlled score bands
/// (or a code consumer for the strings) before any letter is
/// invented.
///
/// Live grade plate for one finished play (`FUN_004c4dd0`, computed at
/// Result paint): the frame comes from the live [`o2jam_game::Score`]
/// over the pinned reference word ([`o2jam_game::Score::grade_frame`]).
/// `None` paints no plate — no chart word, or the ratio sits under
/// the lowest threshold. The staged frame paints at (395,531)
/// through [`paint_result_scene`].
#[must_use]
pub fn result_grade_frame(score: &o2jam_game::Score, note_count: u16) -> Option<u8> {
    score.grade_frame(note_count)
}

/// White ink of the pinned result fields (the exe text helper runs
/// at color `0xffffff`, Arial).
fn result_number_ink() -> u32 {
    pack_xrgb(0xFF, 0xFF, 0xFF)
}

/// True while `frame` is the exe's native canvas: the pinned result
/// anchors live on the 800×600 surface the exe ran on, and the
/// fixed-size glyph text must not scatter over smaller frames (the
/// window and shot pipeline compose at native and scale once at
/// present time).
#[must_use]
fn frame_is_native(frame: &Frame<'_>) -> bool {
    frame.w == usize::try_from(NATIVE_WIDTH).unwrap_or(0)
        && frame.h == usize::try_from(NATIVE_HEIGHT).unwrap_or(0)
}

/// Played points clamped into the exe's 32-bit score field (the
/// `%8d` read; our u64 accumulator saturates instead of wrapping).
#[must_use]
fn result_score_field(score: &o2jam_game::Score) -> u32 {
    u32::try_from(score.points).unwrap_or(u32::MAX)
}

/// Paint the pinned result numbers from the live `Score`
/// (docs/cresult-anchors.md, top-panel painter `FUN_004c38a0`, file
/// `0xc38a0`): COOL/GOOD/BAD/MISS at y=125, MAX/JAM COMBO at y=130,
/// total score at (605,533) — white GDI text under the baked
/// `StateResult_Top` headers, never digit sprites. Each field draws its
/// decimal from the anchor's left edge: every text call in the top
/// painter, the row painter (`FUN_004c3b40`, file `0xc3b40`), and the
/// width wrapper (`FUN_004c43d0`) passes identical trailing text-helper
/// args `(0, 3, 4)`, and the body of `FUN_0042c730` is not dumped, so
/// left-vs-right anchoring stays OPEN (dossier note 5) and the verified
/// passed x is kept as the left edge. JAM COMBO reads the jam level
/// chain (the wire's u16 `jam` field; mapping OPEN 3); non-native
/// frames paint nothing.
fn paint_result_numbers(frame: &mut Frame<'_>, score: &o2jam_game::Score) {
    if !frame_is_native(frame) {
        return;
    }
    let ink = result_number_ink();
    draw_number(frame, RESULT_COOL_X, RESULT_COUNT_Y, score.counts[0], ink);
    draw_number(frame, RESULT_GOOD_X, RESULT_COUNT_Y, score.counts[1], ink);
    draw_number(frame, RESULT_BAD_X, RESULT_COUNT_Y, score.counts[2], ink);
    draw_number(frame, RESULT_MISS_X, RESULT_COUNT_Y, score.counts[3], ink);
    draw_number(
        frame,
        RESULT_MAX_COMBO_X,
        RESULT_COMBO_Y,
        score.max_combo,
        ink,
    );
    draw_number(
        frame,
        RESULT_JAM_COMBO_X,
        RESULT_COMBO_Y,
        score.jam.level(),
        ink,
    );
    draw_number(
        frame,
        RESULT_SCORE_X,
        RESULT_SCORE_Y,
        result_score_field(score),
        ink,
    );
}

/// Pinned result number anchors (docs/cresult-anchors.md, top-panel
/// painter `FUN_004c38a0`): the judgement counts share row y=125,
/// the combo fields sit at y=130, and the total score reads `%8d`
/// at (605,533).
const RESULT_COOL_X: i32 = 151;
/// GOOD column x.
const RESULT_GOOD_X: i32 = 240;
/// BAD column x.
const RESULT_BAD_X: i32 = 337;
/// MISS column x.
const RESULT_MISS_X: i32 = 426;
/// Judgement-count row y.
const RESULT_COUNT_Y: i32 = 125;
/// MAX COMBO column x.
const RESULT_MAX_COMBO_X: i32 = 505;
/// JAM COMBO column x.
const RESULT_JAM_COMBO_X: i32 = 596;
/// Combo row y.
const RESULT_COMBO_Y: i32 = 130;
/// Total score column x.
const RESULT_SCORE_X: i32 = 605;
/// Total score row y.
const RESULT_SCORE_Y: i32 = 533;
/// `State_Result.bnd` (7 rects, parsed from 7.55 `Interface1.opi`;
/// rect # + corners, paired by code ordinal `(code & 0xFFF) - 1`):
///
/// | # | corners | control |
/// |---|---|---|
/// | 1 | (0,0)-(800,600) | `IDC_BACK_SRESULT_1` `State_Result.ojs` |
/// | 2 | (29,316)-(58,406) | `IDC_IMAGE_SRESULT_RANKTAPFIELD` |
/// | 3 | (29,406)-(58,496) | `IDC_IMAGE_SRESULT_RESULTTAPFIELD` |
/// | 4 | (0,0)-(800,600) | `IDC_IMAGE_SRESULT_RESULTTAP` `Result_EachResult.ojs` |
/// | 5 | (711,569)-(794,593) | `IDC_BUTTON_SRESULT_BACK` |
/// | 6 | (625,570)-(708,592) | `IDC_BUTTON_SRESULT_RETRY` |
/// | 7 | (395,531)-(474,572) | `IDC_IMAGE_SRESULT_RANKINGFIELD` `Result_Imgs_Rank.ojs` |
///
/// Negative evidence: no rect covers any numeric readout — the
/// counts/combos/score rows below are exe-written GDI text (top
/// painter `FUN_004c38a0`, file `0xc38a0`: GDI word fields through
/// the `%5d`/`%4d`/`%3d` width wrapper plus the `%8d` total, and
/// the two 8-cell sprite fields through the 24 px-pitch digit
/// painters with the digit-`1` +5 kern), all re-verified with the
/// `rea` static pass (digit-writer calls with the constants
/// above). So the `RESULT_*` anchors stay exe-pinned, never
/// bnd-sourced; only the buttons and the grade plate have rects.
/// Pinned RETRY button rect (`docs/cresult-anchors.md` rect 6,
/// `IDC_BUTTON_SRESULT_RETRY`, (625,570)-(708,592)): the click
/// fallback while staged art binds no such control.
pub(crate) const RESULT_RETRY_RECT: Rect = Rect {
    x: 625,
    y: 570,
    w: 83,
    h: 22,
};
/// Pinned BACK button rect (rect 5, `IDC_BUTTON_SRESULT_BACK`,
/// (711,569)-(794,593)): same fallback role.
pub(crate) const RESULT_BACK_RECT: Rect = Rect {
    x: 711,
    y: 569,
    w: 83,
    h: 24,
};
/// Staged control id behind [`RESULT_RETRY_RECT`].
pub(crate) const RESULT_RETRY_ID: &str = "IDC_BUTTON_SRESULT_RETRY";
/// Staged control id behind [`RESULT_BACK_RECT`].
pub(crate) const RESULT_BACK_ID: &str = "IDC_BUTTON_SRESULT_BACK";

/// Rank number column x (`%d` at x=91, drawn only with 2+ players).
const RESULT_RANK_X: i32 = 91;
/// Nickname column x (at 318+22·row, truncates over 100 chars).
const RESULT_ROW_NAME_X: i32 = 128;
/// Judgements column x (`%5d /%5d /%3d /%3d` at 317+22·row).
const RESULT_ROW_JUDGE_X: i32 = 230;
/// Combo-pair column x (`%4d /%4d`: MAX then JAM COMBO).
const RESULT_ROW_COMBO_X: i32 = 416;
/// Row score column x (`%8d`).
const RESULT_ROW_SCORE_X: i32 = 530;
/// Bonus column x (`+ %d`).
const RESULT_ROW_BONUS_X: i32 = 626;
/// Speed column x (`X 0.5`…`X R`).
const RESULT_ROW_SPEED_X: i32 = 685;
/// Row text y for every column but the nickname (317+22·row: only
/// the judgement and nickname heights are pinned, so the other
/// columns share the judgement line).
const RESULT_ROW_TEXT_Y: i32 = 317;
/// Nickname text y (318+22·row).
const RESULT_ROW_NAME_Y: i32 = 318;
/// Row pitch (row base y = 320+22·row).
const RESULT_ROW_PITCH: i32 = 22;
/// Seats on the member table: rows never exceed eight.
const RESULT_RANK_ROWS_MAX: usize = 8;
/// Nickname display limit (the exe truncates over 100 chars).
const RESULT_ROW_NAME_LIMIT: usize = 100;

/// One result rank-table row from live data (row painter
/// `FUN_004c3b40`, file `0xc3b40`): every column is `Option` — rows
/// without data stay unpainted with documented reasons, never
/// invented values. Rank bytes ride `PeerBars` (nonzero deliveries
/// only); names ride the joined room (`YOU` is local-only, the wire
/// sends no self nickname); judgements/combos/score/bonus ride the
/// local `Score`/`LongNoteBonus` (per-member 4014/4016 tallies stay
/// server-side, so those columns stay unpainted on peer rows); speed
/// rides the shared room speed code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRankRow {
    /// Rank number (`%d` at x=91, 2+ players only).
    pub rank: Option<u8>,
    /// Nickname at x=128 (truncated past 100 display chars).
    pub name: Option<String>,
    /// COOL/GOOD/BAD/MISS at x=230 (local row only).
    pub judgements: Option<[u32; 4]>,
    /// MAX then JAM combo at x=416 (local row only).
    pub combos: Option<(u32, u32)>,
    /// Score at x=530 (local row only).
    pub score: Option<u32>,
    /// Hold bonus at x=626 (local row only).
    pub bonus: Option<u64>,
    /// Room speed code at x=685 (every named row shares it).
    pub speed: Option<u8>,
    /// Member record `+0x31` — the first 4043 `MemberInfo` seat byte:
    /// a 1-based index into the eight constant portrait slots
    /// (`FUN_004c3b40`), doubling as the `FUN_004c49a0` battle-team
    /// bucket (`0` skips the row). `None` where no member record
    /// feeds the row (the local record's `+0x31` writer stays
    /// unpinned — OPEN).
    pub seat_byte: Option<u8>,
}

/// Nickname as shown: first 100 display chars (the exe truncates
/// over 100 chars of CP949; the shell counts chars).
#[must_use]
fn truncate_result_name(name: &str) -> String {
    if name.chars().count() > RESULT_ROW_NAME_LIMIT {
        name.chars().take(RESULT_ROW_NAME_LIMIT).collect()
    } else {
        name.to_owned()
    }
}

/// Combined judgement text (`%5d /%5d /%3d /%3d`).
#[must_use]
fn result_judgements_text(counts: [u32; 4]) -> String {
    format!(
        "{:>5} /{:>5} /{:>3} /{:>3}",
        counts[0], counts[1], counts[2], counts[3]
    )
}

/// Combined combo text (`%4d /%4d`: MAX then JAM).
#[must_use]
fn result_combos_text(combos: (u32, u32)) -> String {
    format!("{:>4} /{:>4}", combos.0, combos.1)
}

/// Row score text (`%8d`).
#[must_use]
fn result_score_text(score: u32) -> String {
    format!("{score:>8}")
}

/// Row bonus text (`+ %d`).
#[must_use]
fn result_bonus_text(bonus: u64) -> String {
    format!("+ {bonus}")
}

/// Text-line y of display row `row` (`None` on overflow).
#[must_use]
fn result_rank_text_y(row: usize) -> Option<i32> {
    let step = i32::try_from(row).ok()?;
    let offset = step.checked_mul(RESULT_ROW_PITCH)?;
    RESULT_ROW_TEXT_Y.checked_add(offset)
}

/// Nickname-line y of display row `row` (`None` on overflow).
#[must_use]
fn result_rank_name_y(row: usize) -> Option<i32> {
    let step = i32::try_from(row).ok()?;
    let offset = step.checked_mul(RESULT_ROW_PITCH)?;
    RESULT_ROW_NAME_Y.checked_add(offset)
}

/// Paint one rank row: each `Some` column draws white GDI text from
/// its pinned left edge, each `None` column stays untouched.
fn paint_result_rank_row(frame: &mut Frame<'_>, row: usize, entry: &ResultRankRow, ink: u32) {
    let Some(y) = result_rank_text_y(row) else {
        return;
    };
    if let Some(rank) = entry.rank {
        draw_text(frame, RESULT_RANK_X, y, &rank.to_string(), ink);
    }
    if let Some(name) = entry.name.as_ref() {
        if let Some(name_y) = result_rank_name_y(row) {
            let shown = truncate_result_name(name);
            if !shown.is_empty() {
                draw_text(frame, RESULT_ROW_NAME_X, name_y, &shown, ink);
            }
        }
    }
    if let Some(counts) = entry.judgements {
        draw_text(
            frame,
            RESULT_ROW_JUDGE_X,
            y,
            &result_judgements_text(counts),
            ink,
        );
    }
    if let Some(combos) = entry.combos {
        draw_text(
            frame,
            RESULT_ROW_COMBO_X,
            y,
            &result_combos_text(combos),
            ink,
        );
    }
    if let Some(score) = entry.score {
        draw_text(frame, RESULT_ROW_SCORE_X, y, &result_score_text(score), ink);
    }
    if let Some(bonus) = entry.bonus {
        draw_text(frame, RESULT_ROW_BONUS_X, y, &result_bonus_text(bonus), ink);
    }
    if let Some(speed) = entry.speed {
        draw_text(frame, RESULT_ROW_SPEED_X, y, &room_speed_label(speed), ink);
    }
}

/// Paint the rank table (row painter `FUN_004c3b40`): up to eight
/// rows from live data, native canvas only. Empty models paint
/// nothing — the row band keeps its backdrop.
fn paint_result_rank_rows(frame: &mut Frame<'_>, rows: &[ResultRankRow]) {
    if !frame_is_native(frame) {
        return;
    }
    let ink = result_number_ink();
    for (row, entry) in rows.iter().enumerate().take(RESULT_RANK_ROWS_MAX) {
        paint_result_rank_row(frame, row, entry, ink);
    }
}

/// Paint one selected frame of a result body (the verdict banner or
/// the grade plate): frame *n* paints image *n−1* through the
/// saturating [`icon_frame_layer`], blitted 1:1 at the layer's
/// record anchor scaled into the frame — both pinned bodies anchor
/// every frame themselves (banner images at y=190 with their own x;
/// plate frames at (395,531)), so the anchor alone positions them
/// exactly where the exe draws (`docs/cresult-anchors.md`). A
/// missing body, frame, or layer paints nothing.
fn paint_result_body_frame(
    frame: &mut Frame<'_>,
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    body_frame: Option<u8>,
) {
    let Some(body_frame) = body_frame else {
        return;
    };
    let Some(figure) = figures.get(sprite) else {
        return;
    };
    let Some(layer_index) = icon_frame_layer(body_frame, figure.len()) else {
        return;
    };
    let Some(layer) = figure.layer(layer_index) else {
        return;
    };
    let (x, y) = anchor_in_frame(layer.dx, layer.dy, frame.w, frame.h);
    blit_rgb555(frame, &layer.image.view(), x, y, Some(layer.colorkey));
}

/// Stretch the album cover over one rect, opaque: JPEG pixels carry
/// no transparency, and any 5-bit colorkey would punch holes where
/// cover colors match the key (the render cover blit compares decoded
/// colors). Nearest-neighbor sampling matches `blit_image_in_rect`;
/// pixels clip to the frame. 5-bit channels widen to 8 by
/// bit-replication, matching the render RGB555 expansion.
fn paint_album_cover(frame: &mut Frame<'_>, cover: &Rgb555Image, dest: Rect) {
    if cover.w == 0 || cover.h == 0 || dest.w == 0 || dest.h == 0 {
        return;
    }
    let dest_w = dest.w as usize;
    let dest_h = dest.h as usize;
    let frame_w = i32::try_from(frame.w).unwrap_or(i32::MAX);
    let frame_h = i32::try_from(frame.h).unwrap_or(i32::MAX);
    for row in 0..dest_h {
        let Some(y) = i32::try_from(row)
            .ok()
            .and_then(|offset| dest.y.checked_add(offset))
        else {
            continue;
        };
        if y < 0 || y >= frame_h {
            continue;
        }
        let src_row = row * cover.h / dest_h;
        for col in 0..dest_w {
            let Some(x) = i32::try_from(col)
                .ok()
                .and_then(|offset| dest.x.checked_add(offset))
            else {
                continue;
            };
            if x < 0 || x >= frame_w {
                continue;
            }
            let word = cover.words[src_row * cover.w + col * cover.w / dest_w];
            // 5-bit channels widened by bit-replication (the render
            // RGB555 expansion).
            let (r, g, b) = (
                ((word >> 10) & 31) as u8,
                ((word >> 5) & 31) as u8,
                (word & 31) as u8,
            );
            frame.buf[y as usize * frame.w + x as usize] = pack_xrgb(
                (r << 3) | (r >> 2),
                (g << 3) | (g >> 2),
                (b << 3) | (b >> 2),
            );
        }
    }
}

/// Paint the played song's album cover into the music-name box
/// ([`RESULT_MUSIC_BOX`], scaled into the frame like the placements):
/// the exe fills the box interior with the song's art over the baked
/// default pane. `None` paints nothing — the staged box stays as-is.
fn paint_result_album(frame: &mut Frame<'_>, album: Option<&Rgb555Image>) {
    let Some(cover) = album else {
        return;
    };
    paint_album_cover(
        frame,
        cover,
        scale_rect_to_frame(&RESULT_MUSIC_BOX, frame.w, frame.h),
    );
}

/// One eight-cell digit-sprite field through the exe painters
/// (`FUN_004c44c0`/`FUN_004c4630`, files `0xc44c0`/`0xc4630`): the
/// decimal digits of `value` (at least one, so 0 paints a 0) fill the
/// rightmost cells at the 24 px pitch, least significant digit
/// rightmost, digit 1 kerned +5 (painter constant, both bodies);
/// leading cells the count does not reach stay unpainted. Each digit
/// selects the font body's layer of the same index (the frame pushed
/// to the frame-select is the raw digit value, not ASCII) and blits
/// at the cell origin under the body colour key. Native canvas only
/// (the exe fields are fixed native pixels); a missing font or digit
/// layer paints nothing.
fn paint_result_digit_field(
    frame: &mut Frame<'_>,
    font: Option<&AvatarFigure>,
    value: u32,
    last_x: i32,
    y: i32,
) {
    let Some(font) = font else {
        return;
    };
    if !frame_is_native(frame) {
        return;
    }
    let mut count = 1usize;
    while count < RESULT_DIGIT_CELLS && value >= 10_u32.pow(count as u32) {
        count += 1;
    }
    for place in 0..count {
        let back = i32::try_from(count - 1 - place).unwrap_or(0);
        let digit = (value / 10_u32.pow((count - 1 - place) as u32)) % 10;
        let kern = if digit == 1 { 5 } else { 0 };
        let x = last_x - back * RESULT_DIGIT_PITCH + kern;
        let Some(layer) = font.layer(digit as usize) else {
            continue;
        };
        let (dx, dy) = anchor_in_frame(x + layer.dx, y + layer.dy, frame.w, frame.h);
        blit_rgb555(frame, &layer.image.view(), dx, dy, Some(layer.colorkey));
    }
}

/// Paint the result backdrop: the staged scene minus the frame
/// bodies ([`RESULT_FRAME_BODIES`]) in file order, so their unused
/// frames never stack through the whole-layer pass. The Top/Bottom
/// panels (`StateResult_Top.ojs` at (0,0), `StateResult_Bottom.ojs`
/// at (0,250), overlapping y=250..316 with Bottom on top) ride this
/// pass: the fill (`FUN_004c3120`, file `0xc3120`) never issues draw
/// calls for the Top/Bottom row objects (`this+0x60[0..1]`), and the
/// per-frame draw-all (slot 9, `FUN_0048c570`) owns every control —
/// so staging their bodies here is the visible equivalent. How the
/// post-mount commit call registers them with that renderer stays
/// OPEN (dossier note 6). Empty art falls back to the flat
/// background; placements without a loaded figure are skipped.
fn paint_result_backdrop(frame: &mut Frame<'_>, art: &SceneArt) {
    if !art.has_figures() {
        fill_background(frame, Screen::Result);
        return;
    }
    let stacked: Vec<ScenePlacement> = art
        .placements
        .iter()
        .filter(|placed| {
            placed
                .sprite
                .as_deref()
                .is_none_or(|sprite| !RESULT_FRAME_BODIES.contains(&sprite))
        })
        .cloned()
        .collect();
    paint_scene_figures(frame, &stacked, &art.figures);
}

/// Paint the pinned result numbers, the rank rows, and the per-row
/// portrait chips (shared by the flat fallback and the staged scene;
/// the numbers and rows are GDI text the exe paints regardless of
/// art). The album cover paints first into the music-name box so the
/// text layers over it, exactly on the green panel of the reference
/// photo; the portrait chips resolve before the rows draw, the exe
/// attach order.
fn paint_result_content(
    frame: &mut Frame<'_>,
    score: &o2jam_game::Score,
    rows: &[ResultRankRow],
    album: Option<&Rgb555Image>,
    portrait_slots: Option<&[ResultPortraitSlot; 8]>,
) {
    paint_result_album(frame, album);
    paint_result_numbers(frame, score);
    paint_result_rank_rows(frame, rows);
    paint_result_portrait_chips(frame, rows, portrait_slots);
}

/// Mode `0xf` portrait-slot table (`FUN_004c2390`, files
/// `0xc245d`–`0xc24b8`): the eight constants the builder pushes
/// through `0x421610(obj, entry, 22)` into `obj+0x15c` of each slot
/// (`docs/cresult-anchors.md`, "Peer-row portraits",
/// `ev_ccdb4679…`).
pub const RESULT_PORTRAIT_TABLE_MODE_F: [u16; 8] = [
    0x7c02, 0x7ee2, 0x7be0, 0x4f2a, 0x37bc, 0x001c, 0x4d5a, 0x5923,
];
/// Mode `0x10` portrait-slot table (files `0xc24bf`–`0xc251a`).
pub const RESULT_PORTRAIT_TABLE_MODE_10: [u16; 8] = [
    0xf802, 0xfde2, 0xf7e0, 0x9e6a, 0x6f5c, 0x001c, 0x9a9a, 0xb263,
];
/// Display-mode words selecting a table (field `+0x24` of the
/// display-mode object `0x415cc0([0x76816c+0x80])`, getter file
/// `0x15d2a`).
pub const RESULT_DISPLAY_MODE_TABLE_F: u32 = 0xf;
pub const RESULT_DISPLAY_MODE_TABLE_10: u32 = 0x10;
/// `obj+0x15e` of every slot: `trunc(22/100·256)` (f64 constants
/// 100.0 at `0x644548`, 256.0 at `0x644540`).
const RESULT_PORTRAIT_NUMERATOR: u8 = 56;
/// `obj+0x160` of every slot: `22/100` as f64.
const RESULT_PORTRAIT_RATIO: f64 = 0.22;

/// One constant portrait slot object of the eight at result
/// `this+0x5c`: an image-less sprite subclass (vtable `0x6444d8`,
/// ctor `0x420b00`) built through the `0x40d100` null-name
/// early-exit path — no archive body is ever loaded. Each slot is
/// parameterized only by its mode-table u16; the two derived fields
/// are identical across the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultPortraitSlot {
    /// Mode-table constant at `obj+0x15c`, consumed as a paint
    /// argument by the subclass draw path (`mov 0x15c(%ecx),%dx`,
    /// file `0x2260f`, onward to `0x421cf0`).
    pub word: u16,
}

impl ResultPortraitSlot {
    /// `obj+0x15e` of every slot: `trunc(22/100·256)` — the
    /// builder's second argument through `0x421610`.
    pub const NUMERATOR: u8 = RESULT_PORTRAIT_NUMERATOR;
    /// `obj+0x160` of every slot: `22/100` as f64.
    pub const RATIO: f64 = RESULT_PORTRAIT_RATIO;
}

/// Build the eight constant portrait slots for a display-mode word.
/// Mode `0xf`/`0x10` write their tables; any other mode writes
/// neither (the exe table keeps its prior contents) — the shell
/// stages no prior table, so `None` resolves no chips.
#[must_use]
pub fn result_portrait_slots(display_mode_word: u32) -> Option<[ResultPortraitSlot; 8]> {
    let table = match display_mode_word {
        RESULT_DISPLAY_MODE_TABLE_F => RESULT_PORTRAIT_TABLE_MODE_F,
        RESULT_DISPLAY_MODE_TABLE_10 => RESULT_PORTRAIT_TABLE_MODE_10,
        _ => return None,
    };
    Some(table.map(|word| ResultPortraitSlot { word }))
}

/// Resolve a member seat byte (member record `+0x31`, the first 4043
/// `MemberInfo` seat byte) to its constant slot: the row painter
/// `FUN_004c3b40` uses the byte as a 1-based, unguarded index into
/// the eight `this+0x5c` objects (byte 1 → element 0, byte 8 →
/// element 7; files `0xc3be2`–`0xc3bfe`). Byte 0 is the
/// `FUN_004c49a0` battle-team skip (file `0xc4a87`), and the exe
/// reads out of bounds for byte 0 and bytes 9+ — the shell resolves
/// neither instead of inventing the out-of-range object.
#[must_use]
fn portrait_slot_for_seat_byte(
    seat_byte: u8,
    slots: &[ResultPortraitSlot; 8],
) -> Option<ResultPortraitSlot> {
    let index = usize::from(seat_byte).checked_sub(1)?;
    slots.get(index).copied()
}

/// Paint the per-row portrait chips (row painter `FUN_004c3b40`):
/// each row's seat byte resolves to its constant slot before the row
/// draws, exactly the exe attach order. The slot art source stays
/// OPEN — the eight objects are image-less and what the subclass
/// draw path renders from `+0x15c` is unsettled (next step per
/// `docs/cresult-anchors.md`: read the `0x6444d8` vtable draw path
/// or capture a populated battle row) — so the resolution runs and
/// nothing visible paints yet; no chip pixels are invented.
fn paint_result_portrait_chips(
    frame: &mut Frame<'_>,
    rows: &[ResultRankRow],
    slots: Option<&[ResultPortraitSlot; 8]>,
) {
    let Some(slots) = slots else {
        return;
    };
    for row in rows.iter().take(RESULT_RANK_ROWS_MAX) {
        let Some(_chip) = row
            .seat_byte
            .and_then(|byte| portrait_slot_for_seat_byte(byte, slots))
        else {
            continue;
        };
        // OPEN: `_chip.word` (`obj+0x15c`) is the paint argument of
        // the slot's draw path; its visual meaning (colour chip,
        // gauge, nothing) lands with the pinned art source.
    }
    let _ = frame;
}

/// Paint the flat results fallback: card, album cover, the pinned
/// result numbers, the rank rows, and the portrait-chip resolution.
/// The retired text verdict stays gone — the banner is a sprite frame
/// of the staged art.
pub(crate) fn paint_result(
    frame: &mut Frame<'_>,
    score: &o2jam_game::Score,
    rows: &[ResultRankRow],
    album: Option<&Rgb555Image>,
    portrait_slots: Option<&[ResultPortraitSlot; 8]>,
) {
    paint_card(
        frame,
        pack_xrgb(0x10, 0x10, 0x08),
        pack_xrgb(0xC0, 0xA0, 0x20),
    );
    paint_result_content(frame, score, rows, album, portrait_slots);
}

/// One staged result paint: everything the scene layers over the
/// backdrop, grouped so the painter keeps a readable signature
/// (`docs/cresult-anchors.md` + the album cover in the music-name
/// box).
#[derive(Debug, Clone, Copy)]
pub struct ResultPaint<'a> {
    /// Constant portrait slots for the current display mode
    /// (`result_portrait_slots`; `None` where no display-mode word
    /// is pinned — the exe writes no table for other modes).
    pub portrait_slots: Option<[ResultPortraitSlot; 8]>,
    /// Live score behind the pinned GDI numbers.
    pub score: &'a o2jam_game::Score,
    /// Rank-table rows from live data.
    pub rows: &'a [ResultRankRow],
    /// Staged `.bnd` art for the scene.
    pub art: &'a SceneArt,
    /// Verdict the banner frame maps from.
    pub verdict: ResultVerdict,
    /// Grade plate frame (1..=8; `None` paints no plate).
    pub grade: Option<u8>,
    /// Played song's album cover (music-name box; `None` paints the
    /// staged box as-is).
    pub album: Option<&'a Rgb555Image>,
    /// Life-readout value for the eight `PointNum.ojs` cells
    /// (y=77, last cell x=460). The exe sums globals `0x77ee58 +
    /// 0x77ee60` (file `0xc394e`); both writers are register-held
    /// (only absolute reads exist — dossier `ev_a1662e22…`), so the
    /// shell maps no live value yet (`docs/cresult-anchors.md` OPEN
    /// 3). `None` paints nothing — no value is invented.
    pub life_readout: Option<u32>,
    /// Bottom-counter value for the eight `Result_NumFont_Blue.ojs`
    /// cells (y=536, last cell x=176). The exe reads the u16 global
    /// `0x76fc66` (file `0xc3a7f`), whose writer is likewise
    /// register-held and whose semantic stays OPEN. `None` paints
    /// nothing.
    pub bottom_counter: Option<u32>,
}

/// Paint the staged result scene (`docs/cresult-anchors.md`): the
/// `.bnd`-bound block art first (minus the frame bodies), then the
/// exe-selected verdict banner and grade plate frames, then the
/// played song's album cover in the music-name box, the avatar
/// figure, the pinned GDI numbers, and the rank rows on top — the
/// fill order of `FUN_004c3120` (banner, then top-panel painter,
/// then rows).
pub fn paint_result_scene(frame: &mut Frame<'_>, paint: &ResultPaint<'_>) {
    paint_result_backdrop(frame, paint.art);
    paint_result_body_frame(
        frame,
        &paint.art.figures,
        RESULT_WINLOSE_SPRITE,
        winlose_frame(paint.verdict),
    );
    paint_result_body_frame(
        frame,
        &paint.art.figures,
        RESULT_RANK_SPRITE,
        paint.grade.filter(|g| *g >= 1),
    );
    paint_result_digit_field_if_staged(
        frame,
        &paint.art.figures,
        RESULT_LIFE_FONT_SPRITE,
        paint.life_readout,
        RESULT_LIFE_LAST_X,
        RESULT_LIFE_Y,
    );
    paint_result_digit_field_if_staged(
        frame,
        &paint.art.figures,
        RESULT_BOTTOM_FONT_SPRITE,
        paint.bottom_counter,
        RESULT_BOTTOM_LAST_X,
        RESULT_BOTTOM_Y,
    );
    paint_result_content(
        frame,
        paint.score,
        paint.rows,
        paint.album,
        paint.portrait_slots.as_ref(),
    );
}

/// [`paint_result_digit_field`] gated on a staged value: the exe
/// fields paint from globals whose writers are register-held (OPEN),
/// so the shell paints a field only when the caller stages a resolved
/// value and never invents one.
fn paint_result_digit_field_if_staged(
    frame: &mut Frame<'_>,
    figures: &HashMap<String, AvatarFigure>,
    sprite: &str,
    value: Option<u32>,
    last_x: i32,
    y: i32,
) {
    if let Some(value) = value {
        paint_result_digit_field(frame, figures.get(sprite), value, last_x, y);
    }
}

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
        Screen::Result => paint_result(frame, &o2jam_game::Score::default(), &[], None, None),
    }
}

#[cfg(test)]
mod paint_tests {
    use super::super::side::{
        MSHOP_ARTIST_CODE, MSHOP_BPM_CODE, MSHOP_CART_TOGGLE_FIRST_CODE, MSHOP_EDITOR_CODE,
        MSHOP_EX_FIRST_CODE, MSHOP_GENRE_CODE, MSHOP_ISBUY_CODE, MSHOP_PAGE_CUR_ONES_CODE,
        MSHOP_PAGE_CUR_TENS_CODE, MSHOP_PAGE_TOT_ONES_CODE, MSHOP_PAGE_TOT_TENS_CODE,
        MSHOP_TITLE_CODE, PLANET_OPTION_CODE, SHOP_CART_PAGE_CUR_ONES_CODE,
        SHOP_CART_PAGE_CUR_TENS_CODE, SHOP_CART_PAGE_TOT_ONES_CODE, SHOP_CART_PAGE_TOT_TENS_CODE,
        SHOP_CART_TOGGLE_FIRST_CODE,
    };
    use super::*;
    use crate::avatar::FigureLayer;
    use crate::screen::Screen;
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
                    album: None,
                    life_readout: None,
                    bottom_counter: None,
                },
            ),
        }
        buf
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
            },
            ShopGood {
                item_id: 2,
                name: "Boots".to_owned(),
                price: 2500,
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
            &goods,
            &wallet,
            &goods[..1],
            0,
            1,
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

    fn shop_placement(code: u32, x: i32, y: i32, w: u32, h: u32) -> ScenePlacement {
        ScenePlacement {
            role: PlacementRole::Single,
            id: String::new(),
            code,
            sprite: None,
            rect: Rect { x, y, w, h },
        }
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
    fn waiting_seats_pad_self_plus_slots_to_eight() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(seats.len(), WAITING_SEATS);
        assert_eq!(seats[0].display_name(), Some("YOU"));
        assert_eq!(seats[1].display_name(), Some("host"));
        assert!(seats[1].is_ready());
        assert!(seats[0].is_ready());
        assert_eq!(seats[2].display_name(), None);
        assert_eq!(seats[3].kind, SeatKind::Locked);
        assert!(seats[4..].iter().all(|seat| seat.kind == SeatKind::Empty));
    }

    #[test]
    fn waiting_row_text_shows_name_team_and_ready() {
        let seats = waiting_seats(&waiting_room());
        assert_eq!(waiting_row_text(1, &seats[1]), "1 A host R");
        assert_eq!(waiting_row_text(0, &seats[0]), "0 B YOU R");
        assert_eq!(waiting_row_text(2, &seats[2]), "2 - ---");
        assert_eq!(waiting_row_text(3, &seats[3]), "3 - LOCK");
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
        // Seat 1 (ready host) sits in column 1, row 0 of the 4x2 grid.
        let (host_x, host_y) = seat_origin(1, 80, 64);
        assert_eq!((host_x, host_y), (25, 16));
        assert_eq!(
            buf[(host_y + 1) as usize * 80 + (host_x - 6 + 1) as usize],
            ready_green
        );
        // Seat 0 (self, ready host) keeps a green dot like any ready seat.
        let (self_x, self_y) = seat_origin(0, 80, 64);
        assert_eq!(
            buf[(self_y + 1) as usize * 80 + (self_x - 6 + 1) as usize],
            ready_green
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
            &art,
            None,
            None,
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xD0, 0x20)));
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
        );
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xE0, 0x30)));
    }

    #[test]
    fn loading_scene_keeps_bar_over_art() {
        let buf = paint_art(Screen::Loading, &corner_art(), 64, 48);
        assert_eq!(buf[0], pack_xrgb(255, 0, 0));
        assert!(buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
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
                album: None,
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
        assert!(loading.contains(&pack_xrgb(0x30, 0xB0, 0x30)));
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
                album: None,
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
                album: None,
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
                album: None,
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
                None,
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
                album: None,
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

    /// Red/green 2x1 cover stand-in: the widened inks probe pixels
    /// (5-bit channels widened by replication, so red lands (255,0,0)).
    fn tiny_cover() -> Rgb555Image {
        Rgb555Image {
            w: 2,
            h: 1,
            words: vec![0x7C00, 0x03E0],
        }
    }

    #[test]
    fn album_cover_paints_into_the_music_name_box() {
        // The OJN cover section (game-resolved) stretches opaque over
        // the box interior ([RESULT_MUSIC_BOX]): the left half of the
        // 2x1 cover covers the box's left columns, the right half its
        // right column, and the frame border pixel left of the box
        // stays card ink.
        let mut buf = vec![0u32; 800 * 600];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &o2jam_game::Score::default(),
            &[],
            Some(&tiny_cover()),
            None,
        );
        let at = |x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(70, 304), pack_xrgb(255, 0, 0));
        assert_eq!(at(733, 518), pack_xrgb(0, 255, 0));
        assert_eq!(at(60, 304), pack_xrgb(0x10, 0x10, 0x08));
    }

    #[test]
    fn missing_cover_paints_the_box_as_is() {
        // No resolved cover: the box region keeps whatever the scene
        // stages — the flat card here — with zero album pixels.
        let without = {
            let mut buf = vec![0u32; 800 * 600];
            paint_result(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &o2jam_game::Score::default(),
                &[],
                None,
                None,
            );
            buf
        };
        let with = {
            let mut buf = vec![0u32; 800 * 600];
            paint_result(
                &mut Frame {
                    buf: &mut buf,
                    w: 800,
                    h: 600,
                },
                &o2jam_game::Score::default(),
                &[],
                Some(&tiny_cover()),
                None,
            );
            buf
        };
        let at = |buf: &[u32], x: i32, y: i32| buf[y as usize * 800 + x as usize];
        assert_eq!(at(&without, 70, 304), pack_xrgb(0x10, 0x10, 0x08));
        assert_eq!(at(&with, 60, 304), at(&without, 60, 304));
    }

    #[test]
    fn album_cover_scales_into_small_frames() {
        // Non-native frames scale the box rect like the placements;
        // the cover still fills it (the window composes at native and
        // scales once, so this is the small-probe path only).
        let mut buf = vec![0u32; 64 * 48];
        paint_result(
            &mut Frame {
                buf: &mut buf,
                w: 64,
                h: 48,
            },
            &o2jam_game::Score::default(),
            &[],
            Some(&tiny_cover()),
            None,
        );
        let dest = scale_rect_to_frame(&RESULT_MUSIC_BOX, 64, 48);
        let at = |x: i32, y: i32| buf[y as usize * 64 + x as usize];
        assert_eq!(at(dest.x, dest.y), pack_xrgb(255, 0, 0));
        let last_x = dest.x + dest.w as i32 - 1;
        let last_y = dest.y + dest.h as i32 - 1;
        assert_eq!(at(last_x, last_y), pack_xrgb(0, 255, 0));
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
    fn loading_cover_paints_full_bleed_behind_bar_and_caption() {
        // Staged OJN cover (2x2 opaque stand-in, distinct corners —
        // geometry only, never real art) stretches over the whole
        // frame while the live bar and caption ink on top.
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
            buf.contains(&pack_xrgb(0x30, 0xB0, 0x30)),
            "bar fill survives over the cover"
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
            1,
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
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
            1,
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
        // Wine sweep (`docs/hover.md`): hovering EASY/NORMAL/HARD
        // adds the body's own 84x18 light plate band (EASY
        // (94,44)-(178,62)) with no frame swap. The rest stack keeps
        // the top layer (the dark selected plate), so the hover
        // repaints layer 0 clipped to the pinned band rect — and
        // only there.
        use super::super::side::{PlanetAction, PLANET_BUTTON_EASY_CODE};
        let sprite = "Planet_Btn_Easy.ojs";
        let art = SceneArt {
            placements: vec![ScenePlacement {
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
            }],
            figures: HashMap::from([(
                sprite.to_owned(),
                anchored_two_frame_figure(99, 112, 87, 32, [0x1F, 0x00], [0x00, 0x7C]),
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
            1,
        );
        assert_eq!(rest[50 * 800 + 136], red, "rest keeps the top layer");
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
            1,
        );
        assert_eq!(hovered[50 * 800 + 136], blue, "plate band reveals layer 0");
        assert_eq!(
            hovered[100 * 800 + 136],
            red,
            "below the band the rest stack stands"
        );
        assert_eq!(
            hovered[50 * 800 + 90],
            red,
            "left of the band the rest stack stands"
        );
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
            1,
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
            1,
        );
        assert_eq!(hovered, rest, "static emblems paint no highlight");
    }

    #[test]
    fn promo_playstop_hover_paints_the_second_frame() {
        // The one true frame swap (`docs/hover.md`): PLAYSTOP rest
        // is frame 0, hover is body frame 1 (224 changed pixels at
        // (392,515)-(407,529)). Unhovered the rest stack stands.
        use super::super::side::{PlanetAction, PlanetPromo, PLANET_PROMO_STOP_CODE};
        let sprite = "Planet_Btn_Promotion_PlayStop.ojs";
        // Three layers on purpose: the hover must take layer 1
        // (the pinned hover frame), never the stack top.
        let mut playstop = AvatarFigure::default();
        for (word, anchor) in [
            ([0xE0, 0x03], (387, 511)),
            ([0x00, 0x7C], (387, 511)),
            ([0x1F, 0x00], (387, 511)),
        ] {
            playstop.push_layer(FigureLayer {
                image: Rgb555Image::decode(&word.repeat(26 * 24), 26, 24).unwrap(),
                dx: anchor.0,
                dy: anchor.1,
                colorkey: 0xFFFF,
            });
        }
        let art = SceneArt {
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
            1,
        );
        assert_eq!(rest[520 * 800 + 400], blue, "rest keeps the top layer");
        let mut hovered = vec![0u32; 800 * 600];
        paint_planet_scene(
            &mut Frame {
                buf: &mut hovered,
                w: 800,
                h: 600,
            },
            &art,
            None,
            Some(PlanetAction::Promo {
                cell: PlanetPromo::Stop,
            }),
            None,
            1,
        );
        assert_eq!(
            hovered[520 * 800 + 400],
            red,
            "hover swaps to frame 1, not the stack top"
        );
    }

    #[test]
    fn promo_banner_frame_follows_the_carousel_index() {
        // The banner body paints its selected frame over the stacked
        // rest (exe refresh arm VA `0x48e4b0` → `set_frame(index)`):
        // index 1 shows the first frame, index 3 the top frame.
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
                index,
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
            1,
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
            1,
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
            1,
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
            1,
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
            1,
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
            1,
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
    fn loading_fill_spans_the_walk_fraction() {
        // Fractional bar: the track stays put, the green fill spans
        // `progress` of the `w/2` inner width (32x24 frame: track
        // x8 w16, half fill x8..16 at 0.5).
        let art = SceneArt::default();
        let status = LoadingStatus {
            progress: 0.5,
            title: String::new(),
            artist: String::new(),
        };
        let mut buf = vec![0u32; 32 * 24];
        paint_loading_scene_with_status(
            &mut Frame {
                buf: &mut buf,
                w: 32,
                h: 24,
            },
            &art,
            &status,
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
        let placements = (0..8)
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
        );
        let red = pack_xrgb(255, 0, 0);
        // Empty seat 2 keeps its staged art pixel: no live line.
        assert_eq!(buf[290 * 800 + 130], red, "empty cover stays art-only");
        // Occupied seat 0 takes the live line: scrim opens at
        // (10 − 4, 200 − 2), covering the art pixel.
        assert_eq!(buf[198 * 800 + 6], scrim_color(), "seat line scrim");
        assert_ne!(buf[200 * 800 + 10], red, "live line covers art");
        assert!(has_ink_near(&buf, art_title_ink()));
        assert_eq!(buf[0], 0, "covers no longer stack at the frame corner");
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

    /// Waiting room with every staged field set: champion seat 1,
    /// streak 3, song 757, speed code 5 (`X3`), one skill ring.
    fn champion_room() -> JoinedRoom {
        let mut room = waiting_room();
        room.champion = Some(1);
        room.champion_streak = Some(3);
        room.music_id = 757;
        room.speed = 5;
        room.skills = vec![2101];
        room
    }

    #[test]
    fn staged_covers_name_champion_and_challenger() {
        // Cover rects 60/61 `(28,53,169,228)`/`(170,53,311,228)`:
        // seat 1 (wire host) names CHAMPION with its wire level,
        // seat 0 (self, no wire level) names CHALLENGER with `-`.
        let placements = vec![
            waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR1", None, (28, 53, 169, 228)),
            waiting_probe("IDC_IMAGE_SWAIT_COVERAVATAR2", None, (170, 53, 311, 228)),
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
        );
        assert_eq!(buf[51 * 800 + 166], scrim_color(), "champion scrim");
        assert_eq!(buf[51 * 800 + 24], scrim_color(), "challenger scrim");
        assert!(has_ink_near(&buf, art_title_ink()));
        // Ready dots ride the cover lines: both seats carry their own
        // wire ready flag (the self seat reads `slots[member_id]`, the
        // same source the flat fallback and the READY toggle use).
        assert_eq!(buf[55 * 800 + 165], pack_xrgb(0x30, 0xE0, 0x30));
        assert_eq!(buf[55 * 800 + 23], pack_xrgb(0x30, 0xE0, 0x30));
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
        );
        for id in [
            ROOM_NICKNAME_ID,
            ROOM_PANEL_LEVEL_ID,
            ROOM_PANEL_EXP_ID,
            ROOM_PANEL_CASH_ID,
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
        );
        for id in [
            ROOM_NICKNAME_ID,
            ROOM_PANEL_LEVEL_ID,
            ROOM_PANEL_EXP_ID,
            ROOM_PANEL_CASH_ID,
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
        );
        assert!(
            ink_in_rect(&buf, 800, slot) > 0,
            "figure ink must land inside the staged slot"
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
        // `waiting_room` seats self (seat 0) plus the wire host
        // (seat 1); seats 2.. stay empty/locked. Both occupied slots
        // take their figure through the same slot path — never the
        // raw `(0,0,123,150)` rect.
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
        );
        assert!(
            ink_in_rect(&buf, 800, waiting_slot_box(0)) > 0,
            "self seat takes its figure"
        );
        assert!(
            ink_in_rect(&buf, 800, waiting_slot_box(1)) > 0,
            "occupied peer seat takes its figure"
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
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn empty_covers_leave_the_staged_panels_untouched() {
        // Cover rect 62 `(312,53,453,228)` is seat 2 (empty wire
        // slot): no scrim, no dot — the staged empty panel shows.
        let placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_COVERAVATAR3",
            None,
            (312, 53, 453, 228),
        )];
        let calm = background_of(Screen::Waiting);
        let mut buf = vec![calm; 800 * 600];
        paint_waiting_covers(
            &mut Frame {
                buf: &mut buf,
                w: 800,
                h: 600,
            },
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[53 * 800 + 312], calm);
        assert!(!buf.contains(&scrim_color()));
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
    fn speed_value_anchors_to_the_speed_num_rect() {
        // `SPEED_NUM` rect 101 `(681,378,735,413)`: speed code 5
        // reads `X3` through the shared label chain (reference row).
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
            &champion_room(),
            &placements,
        );
        assert_eq!(buf[376 * 800 + 677], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
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
            &placements,
        );
        assert_eq!(buf[206 * 800 + 697], scrim_color());
        assert_eq!(buf[246 * 800 + 701], 0);
    }

    #[test]
    fn song_title_anchors_to_the_musicname_rect() {
        // `MUSICNAME` rect 22 `(573,15,773,38)`: bare song id while
        // the staged catalog names no such song (the song table
        // lives server-side, as in the lobby).
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
            &champion_room(),
            &placements,
            None,
        );
        assert_eq!(buf[13 * 800 + 569], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
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
            &champion_room(),
            &placements,
            Some("First"),
        );
        assert_eq!(buf[13 * 800 + 569], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
    fn staged_seat_text_shows_level_and_name() {
        // Room-fill `Lv.%d:%s` flow: role prefix, team letter, wire
        // level, nickname, ready flag.
        let champion = StagedSeat {
            name: "host".to_owned(),
            level: Some(9),
            team: 0,
            ready: true,
            present: true,
            locked: false,
            champion: true,
            master: true,
            playing: false,
            has_song: true,
        };
        assert_eq!(
            staged_seat_text(&champion, Some("CHAMPION")),
            "CHAMPION A Lv:9 host R"
        );
        assert_eq!(staged_seat_text(&champion, None), "A Lv:9 host R");
        let challenger = StagedSeat {
            name: "YOU".to_owned(),
            level: None,
            team: 1,
            ready: false,
            present: true,
            locked: false,
            champion: false,
            master: false,
            playing: false,
            has_song: false,
        };
        assert_eq!(
            staged_seat_text(&challenger, Some("CHALLENGER")),
            "CHALLENGER B Lv:- YOU -"
        );
    }

    #[test]
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
            &placements,
            &WaitingCaptions::default(),
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
    fn waiting_random_cards_carry_no_card_data() {
        // The `?` cards name no per-card data: the big card sits on
        // `RANDMAP` (rect 78 `(619,468,708,533)`) and the small cards on
        // the spriteless `EFFECT_1/2` rects (68/69) — caption and frames
        // baked into the staged backdrop, no wire field behind them.
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
            &placements,
            &WaitingCaptions::default(),
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
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
            &empty,
            &WaitingCaptions::default(),
        );
        assert!(buf.iter().all(|pixel| *pixel == 0));
    }

    #[test]
    fn staged_scene_prefers_bnd_anchors_over_the_legacy_grid() {
        // Covers staged: the seat line anchors to the cover rect
        // and the legacy grid slot `(100,150)` stays untouched — an
        // empty stage paints no fill (`docs/channel-fill.md`).
        let placements = vec![waiting_probe(
            "IDC_IMAGE_SWAIT_COVERAVATAR1",
            None,
            (28, 53, 169, 228),
        )];
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
        );
        assert_eq!(buf[51 * 800 + 24], scrim_color());
        assert_eq!(buf[150 * 800 + 100], 0);
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
            &art,
            None,
            None,
        );
        assert_eq!(buf[0], 0);
        assert!(!buf.contains(&background_of(Screen::Channel)));
        assert!(has_ink_near(&buf, pack_xrgb(0xFF, 0xFF, 0xFF)));
    }

    #[test]
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
            &art,
            None,
            None,
        );
        let blue = pack_xrgb(0x00, 0x00, 0xFF);
        // Face of viewport slot 0 (native 502,236 + 44x21 art).
        assert_eq!(buf[240 * 800 + 510], blue);
        // Face of viewport slot 1.
        assert_eq!(buf[290 * 800 + 510], blue);
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
            &art,
            Some(&board),
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
            &art,
            Some(&empty_board),
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
        assert_eq!(buf[58 * 800 + 616], scrim_color());
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
        assert_eq!(buf[58 * 800 + 616], scrim_color());
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
            &placements,
        );
        assert_eq!(buf[206 * 800 + 736], scrim_color());
        assert_eq!(buf[244 * 800 + 736], scrim_color());
        assert_eq!(buf[282 * 800 + 736], scrim_color());
        assert!(has_ink_near(&buf, art_title_ink()));
    }

    #[test]
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
            &placements,
            &WaitingCaptions::default(),
        );
        assert_eq!(buf[13 * 800 + 476], scrim_color(), "mapname inks");
        assert_eq!(buf[40 * 800 + 480], 0, "instname untouched");
        assert_eq!(buf[455 * 800 + 100], 0, "notice text untouched");
        assert_eq!(buf[480 * 800 + 100], 0, "notice board untouched");
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
}
