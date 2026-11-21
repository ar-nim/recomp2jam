//! Avatar-room (MyRoom) scene cluster (#139): control dispatch,
//! selection state, staged rows, and caption helpers. Pure scene data
//! and pure functions; `ScreenGame` keeps the flow and staged art.

use super::super::super::avatar::{avatar_item_by_id, ScenePlacement};
use super::super::paint::rect_hit;
use super::{
    equipped_item_view, next_up_threshold, wallet_amount_caption, SelectMusicSong, ShopItemView,
    WalletState,
};
use o2jam_file::{AvatarItemRow, AvatarItemTable};
use o2jam_net::{CharacterInfo, GiftItemInfo, GiftMusicInfo};

// --- Avatar-room (MyRoom) interaction ---
//
// Control codes from the shipped `STATE_MYROOM` block dump (file
// order, `docs/myroom.md`): 11 art-less slot selectors
// (`0X15200005..0F`), 10 bag images (`0X15600023..2C`), the bag
// pager pair, the gift pager pair plus 5 gift-box cells, and the
// BACK/GOSHOP/VACATE/INVENTORY buttons. Caption codes are the
// `0X1540xxxx` TEXT controls of the same block.

/// First slot-selector button code (`IDC_BUTTON_SMYROOM_INSTRUMENT`).
pub const MYROOM_SLOT_FIRST_CODE: u32 = 0x1520_0005;
/// Slot-selector button count (instrument through shoes).
pub const MYROOM_SLOT_BUTTONS: usize = 11;
/// Slot count as `u32` for code arithmetic without casts.
pub const MYROOM_SLOT_BUTTONS_U32: u32 = 11;
/// Slot button names in code order (ASCII stand-ins from the IDC
/// ids; the exe's Korean slot comments have no pinned string source).
pub const MYROOM_SLOT_NAMES: [&str; MYROOM_SLOT_BUTTONS] = [
    "INSTRUMENT",
    "HAIR",
    "ARTICLE",
    "GLOVES",
    "NECKLACE",
    "JACKET",
    "PANTS",
    "GLASSES",
    "EARRING",
    "CLOTHINGART",
    "SHOES",
];
/// MY BAG tab anchor (`IDC_IMAGE_SMYROOM_MYBAG_POS`): bnd ordinal
/// `0x45`, rect (6,383)-(94,404). The exe3 `8515a7e5…` frame shows
/// this press leaving the gift panel open, so it records no change.
pub const MYROOM_BAG_TAB_CODE: u32 = 0x1560_0045;
/// PRESENT BOX tab anchor (`IDC_IMAGE_SMYROOM_GIFTBOX_POS`): bnd
/// ordinal `0x46`, rect (98,384)-(186,406). Its press swaps the lower
/// panel to the gift list (exe3 `6e29e51a…`).
pub const MYROOM_GIFT_TAB_CODE: u32 = 0x1560_0046;
/// First bag-cell image code (`IDC_IMAGE_SMYROOM_MYBAGITEM1`).
pub const MYROOM_BAG_FIRST_CODE: u32 = 0x1560_0023;
/// Bag cells per page (10 `MYBAGITEM` images).
pub const MYROOM_BAG_CELLS: usize = 10;
/// Bag pager LEFT button (`Shop_Btn_BagLeft.ojs`).
pub const MYROOM_BAG_LEFT_CODE: u32 = 0x1520_002D;
/// Bag pager RIGHT button (`Shop_Btn_BagRight.ojs`).
pub const MYROOM_BAG_RIGHT_CODE: u32 = 0x1520_002E;
/// First gift-box cell code (`IDC_BUTTON_SMYROOM_GIFTBOX_EX1`).
pub const MYROOM_GIFT_FIRST_CODE: u32 = 0x1501_C04A;
/// Gift-box cells per page (5 `GIFTBOX_EX` buttons).
pub const MYROOM_GIFT_CELLS: usize = 5;
/// Gift pager LEFT button (`Myroom_Btn_GiftLeft.ojs`).
pub const MYROOM_GIFT_LEFT_CODE: u32 = 0x1520_0048;
/// Gift pager RIGHT button (`Myroom_Btn_GiftRight.ojs`).
pub const MYROOM_GIFT_RIGHT_CODE: u32 = 0x1520_0049;
/// Gift claim button (`Myroom_Btn_GiftMove.ojs`: moves the
/// selected gift into the bag). The control choice is
/// ours-by-design — no exe click sequence is pinned — while the
/// queued `ClaimGift` 5023 shape is exe-pinned (`docs/gifts.md`);
/// next step: rig walk of the gift-box claim gesture.
pub const MYROOM_MOVE_CODE: u32 = 0x1550_0053;
/// BACK button (`Myroom_Back.ojs`): returns to the opening lobby and
/// queues the exe's empty 5007 ExitShop send (#802).
pub const MYROOM_BACK_CODE: u32 = 0x1520_0002;
/// GOSHOP button (`Myroom_GoShop.ojs`): opens the item shop.
/// Records only — nested sides stay rejected, so the hop is OPEN
/// with a leave-then-visit hand step.
pub const MYROOM_GOSHOP_CODE: u32 = 0x1520_0003;
/// Vacate button (`MyRoom_Btn_Vacate.ojs`): clears the selected worn
/// slot locally and restages the figure; queues nothing — the vacate
/// send shape is unpinned (OPEN, `docs/myroom.md`).
pub const MYROOM_VACATE_CODE: u32 = 0x1520_0004;
/// INVENTORY button (`Myroom_Inventory.ojs`): commits the
/// selected bag cell (equip-apply). The control choice is
/// ours-by-design — the exe send sites (`0xa4af4` shop-control,
/// `0xa4bcb` apply, `docs/equip.md`) name no control — while the
/// queued `EquipItem` 5004 shape and slot derivation are
/// exe-pinned; next step: rig walk of the myroom apply gesture.
pub const MYROOM_APPLY_CODE: u32 = 0x1520_3034;
/// Static blue selection frame (`IDC_IMAGE_SMYROOM_MYBAGSELECT`,
/// `Myroom_MybagSelect.oji`): archive slot 1305, 86x69 prefixed
/// body; `.bnd` ordinal `0x33` = (0,0,86,69). The captured faces never
/// show it at that rect (`docs/myroom.md` p305 note), so the painter
/// uses it as the real frame body over a selected cell while its
/// placement geometry still comes from the staged `.bnd` rects.
pub const MYROOM_SELECT_CODE: u32 = 0x1550_0033;
/// Avatar portrait (`0X15700012` AVATAR image, `docs/myroom.md`
/// block walk): the staged worn figure paints here, never text.
/// Keyed by code like the other MyRoom image families (bag, gift,
/// slot covers) — the painter finds the staged `.bnd` rect, never
/// a literal.
pub const MYROOM_AVATAR_CODE: u32 = 0x1570_0012;
/// O2-money caption (`IDC_TEXT_SMYROOM_O2MONEY`): no wire source
/// pinned — stays unpainted, OPEN (`docs/myroom.md` OPEN 4).
pub const MYROOM_O2MONEY_CODE: u32 = 0x1540_0010;
/// Charge-money caption (`IDC_TEXT_SMYROOM_CHARGEMONEY`): no wire
/// source pinned — stays unpainted, OPEN (`docs/myroom.md` OPEN 4).
pub const MYROOM_CHARGEMONEY_CODE: u32 = 0x1540_0011;
/// Nickname caption (`IDC_TEXT_SMYROOM_NICKNAME`).
pub const MYROOM_NICKNAME_CODE: u32 = 0x1540_0037;
/// Level caption (`IDC_TEXT_SMYROOM_LEVEL`).
pub const MYROOM_LEVEL_CODE: u32 = 0x1540_0038;
/// Bonus-point caption (`IDC_TEXT_SMYROOM_BONUSPOINT`). The exe's
/// hash-pinned STATUS cells print `Cash Point` here instead, so the
/// caption paints the 2001 cash-point field as `N PT`.
pub const MYROOM_BONUS_CODE: u32 = 0x1540_0039;
/// Alias for the observed Cash Point cell at `MYROOM_BONUS_CODE`.
pub const MYROOM_CASH_POINT_CODE: u32 = MYROOM_BONUS_CODE;
/// Experience caption (`IDC_TEXT_SMYROOM_EXP`).
pub const MYROOM_EXP_CODE: u32 = 0x1540_003A;
/// Next-up-exp caption (`IDC_TEXT_SMYROOM_NEXTUPEXP`): paints the
/// staged experience, following the room-panel precedent (the 7.55
/// room capture shows `Exp.` there; `docs/myroom.md` OPEN 4).
pub const MYROOM_NEXTUPEXP_CODE: u32 = 0x1540_003B;
/// Win-rate caption (`IDC_TEXT_SMYROOM_WINRATION`, exe spelling):
/// integer percent from staged wins/battles (`docs/myroom.md` OPEN 4).
pub const MYROOM_WINRATE_CODE: u32 = 0x1540_003C;
/// Rank caption (`IDC_TEXT_SMYROOM_RANK`): web-served, no client
/// source — stays unpainted, OPEN (`docs/myroom.md` OPEN 4).
pub const MYROOM_RANK_CODE: u32 = 0x1540_003D;
/// Guild caption (`IDC_TEXT_SMYROOM_GUILD`): no wire source pinned
/// — stays unpainted, OPEN (`docs/myroom.md` OPEN 4).
pub const MYROOM_GUILD_CODE: u32 = 0x1540_003E;
/// MGame-money caption (`IDC_TEXT_SMYROOM_MGAME_MONEY`): the G
/// readout at rect (685,421)-(780,434). The exe's G 15527 / G 626
/// frames pair with the live 5029 gem words `a73c0000` / `72020000`,
/// so this cell inks [`WalletState::gem`].
pub const MYROOM_MGAME_MONEY_CODE: u32 = 0x1540_003F;
/// O2Jam-money caption (`IDC_TEXT_SMYROOM_O2JAM_MONEY`): the W
/// readout inks the second settled 5029 word, the wallet cash slot
/// (`docs/shop-buy.md`, file `0x163900`).
pub const MYROOM_O2JAM_MONEY_CODE: u32 = 0x1540_0040;
/// First free-info caption (`IDC_TEXT_SMYROOM_FREEINFO1`).
pub const MYROOM_FREEINFO_FIRST_CODE: u32 = 0x1540_0041;
/// Free-info caption count (FREEINFO1..4).
pub const MYROOM_FREEINFO_COUNT: usize = 4;
/// First skill-count caption (`IDC_TEXT_SMYROOM_SKILLCOUNT1`): source
/// OPEN — stays unpainted (`docs/myroom.md` OPEN 4).
pub const MYROOM_SKILLCOUNT_FIRST_CODE: u32 = 0x1540_005C;
/// Skill-count caption count (SKILLCOUNT1..10).
pub const MYROOM_SKILLCOUNT_COUNT: usize = 10;
/// Penalty-count caption (`IDC_TEXT_SMYROOM_PENALTYCOUNT`).
pub const MYROOM_PENALTY_COUNT_CODE: u32 = 0x1540_0066;
/// Penalty-level caption (`IDC_TEXT_SMYROOM_PENALTYLEVEL`).
pub const MYROOM_PENALTY_LEVEL_CODE: u32 = 0x1540_0067;
/// O2-cash caption (`IDC_TEXT_SMYROOM_O2CASH`).
pub const MYROOM_O2CASH_CODE: u32 = 0x1540_0059;
/// Music-cash caption (`IDC_TEXT_SMYROOM_MUSICCASH`).
pub const MYROOM_MUSICCASH_CODE: u32 = 0x1540_005A;
/// Item-cash caption (`IDC_TEXT_SMYROOM_ITEMCASH`).
pub const MYROOM_ITEMCASH_CODE: u32 = 0x1540_005B;
/// Notice-text image (`IDC_IMAGE_SMYROOM_NOTICE_TEXT`): content OPEN
/// like the waiting NOTICE (`docs/caption-sources.md`) — paints nothing.
pub const MYROOM_NOTICE_TEXT_CODE: u32 = 0x1550_0068;
/// Notice-board image (`IDC_IMAGE_SMYROOM_NOTICEBOARD`).
pub const MYROOM_NOTICEBOARD_CODE: u32 = 0x1550_0069;
/// PC-room mark (`IDC_IMAGE_SMYROOM_PCROOMMARK`): P0, visibly inert.
pub const MYROOM_PCROOMMARK_CODE: u32 = 0x1550_006A;
/// Web-avatar button (`IDC_BUTTON_SMYROOM_WEBABATAR`): P0, queues nothing.
pub const MYROOM_WEBAVATAR_CODE: u32 = 0x1520_0058;
/// Conference button (`IDC_BUTTON_SMYROOM_CONFERENCE`): P0, queues nothing.
pub const MYROOM_CONFERENCE_CODE: u32 = 0x1520_006B;
/// Bag-sort button (`IDC_BUTTON_SMYROOM_BAGSORT`): P0, queues nothing.
pub const MYROOM_BAGSORT_CODE: u32 = 0x1520_006C;
/// Messenger button (`IDC_BUTTON_SMYROOM_MESSENGER`): P0, queues nothing.
pub const MYROOM_MESSENGER_CODE: u32 = 0x1520_006D;
/// Messenger alarm (`IDC_IMAGE_SMYROOM_MESSENGER_ALRAM`): P0, paints nothing.
pub const MYROOM_ALARM_CODE: u32 = 0x1550_006E;
/// Messenger alarm popup (`IDC_IMAGE_SMYROOM_MESSENGER_ALRAM1`): P0.
pub const MYROOM_ALARM_POPUP_CODE: u32 = 0x1550_006F;
/// Gift-count caption (`IDC_TEXT_GIFT_NUM`).
pub const MYROOM_GIFT_NUM_CODE: u32 = 0x1540_004F;
/// Gift resource-name caption (`IDC_TEXT_GIFT_ITEMNAME`).
pub const MYROOM_GIFT_ITEMNAME_CODE: u32 = 0x1540_0051;
/// Gift sender caption (`IDC_TEXT_GIFT_SENDER`).
pub const MYROOM_GIFT_SENDER_CODE: u32 = 0x1540_0052;
/// First equipment-well image (`0X15600013..22`, 16 Korean-labelled
/// slot images in the `STATE_MYROOM` block, `docs/myroom.md`).
pub const MYROOM_SLOT_IMAGE_FIRST_CODE: u32 = 0x1560_0013;
/// Count of bound equipment wells (`0X15600013..22`).
pub const MYROOM_EQUIPMENT_WELLS: usize = 16;
/// Selector buttons that pair with a labelled equipment well (the
/// first eleven wells). Selection is still button-only; the well
/// join itself uses the item record byte, see [`myroom_worn_well_items`].
pub const MYROOM_SLOT_WELLS: usize = MYROOM_SLOT_BUTTONS;

/// Large upper-right panel: `IDC_IMAGE_SMYROOM_SLOTCOVER`
/// (`Myroom_SlotCover.ojs`, 109x81-derived 494x312 body at `.bnd` rect
/// (240,46)-(734,358)). This is the exe's default Inventory face.
pub const MYROOM_SLOTCOVER_CODE: u32 = 0x1550_0035;
/// Same panel's `IDC_IMAGE_SMYROOM_USERINFO` (`Myroom_Status.ojs`,
/// rect (392,22)-(776,379)). Later in staged order, so it covers the
/// slot grid whenever both bodies paint.
pub const MYROOM_USERINFO_CODE: u32 = 0x1550_0036;

/// Upper-right face shown on MyRoom entry. The exe b8 capture
/// (`1743d36d…fae0`) opens on `Inventory`; the status plate is the other
/// face. A gesture that switches faces is not pinned by that pack
/// (next step: qualified slot/button press capture), so this state is
/// driven only through [`MyRoomState::show_panel`] until one is found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MyRoomPanel {
    /// Equipment-slot grid plus the background's baked STATUS button.
    #[default]
    Inventory,
    /// `Myroom_Status.ojs` plate plus the INVENTORY button body.
    Status,
}

/// Messenger branch selected by the exe's global words before
/// `FUN_004a3ee0` case `0x1520006d` (file `0xa3ee0`,
/// EV `ev_aa875251…`). The disconnected branch is the B8 pack's one-OK
/// liveness sheet; the other two strings are pinned at files
/// `0x247dc0`/`0x247e08`, and modal 21 has no pinned caption. The wire
/// writer of those globals is OPEN, so callers stage the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MyRoomMessengerState {
    /// No messenger server: one-OK connection-lost notice (default).
    #[default]
    Disconnected,
    /// Info loading: one-OK loading notice.
    Loading,
    /// Info available: modal 21, caption unpinned.
    Available,
}

/// Whether one staged MyRoom placement belongs to the visible panel
/// face. The Inventory face hides the status plate and the INVENTORY
/// button body: the latter would cover the background's STATUS label.
/// Every other placement (backdrop, avatar, bag, gift) stays live.
#[must_use]
pub fn myroom_panel_kept(placed: &ScenePlacement, panel: MyRoomPanel) -> bool {
    !matches!(
        (placed.code, panel),
        (
            MYROOM_USERINFO_CODE | MYROOM_APPLY_CODE,
            MyRoomPanel::Inventory
        )
    )
}

/// Lower-panel tab whose rows the painter inks. The exe gift-tab
/// frame (`6e29e51a…`) hides the bag grid and shows the gift list;
/// the pinned MY BAG press (`8515a7e5…`) does not switch it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MyRoomLowerTab {
    /// Owned-item grid under the MY BAG tab (default on entry).
    #[default]
    Bag,
    /// Gift rows under the PRESENT BOX tab.
    Gifts,
}

/// One pager step: which end of the row run the press moves toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MyRoomStep {
    /// LEFT pager button: toward the first rows.
    Previous,
    /// RIGHT pager button: toward the last rows.
    Next,
}

/// One avatar-room control press: the code → action dispatch output.
/// Board input routes here, never through per-button literals at
/// the call site. Geometry never lives here: it rides the staged
/// `.bnd` placements (owner rule `f38f3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MyRoomAction {
    /// Slot-selector button pressed: button ordinal `0..11`.
    /// What the selection filters stays OPEN (`docs/myroom.md`).
    SelectSlot(u8),
    /// Bag pager pressed: turn the owned-item page.
    BagPage(MyRoomStep),
    /// MY BAG tab pressed: records nothing (exe3 no-op pin).
    ShowBagTab,
    /// PRESENT BOX tab pressed: show the gift list below.
    ShowGiftTab,
    /// Bag cell pressed: cell index within the page `0..10`.
    PickBagCell(usize),
    /// Gift pager pressed: turn the gift-box page.
    GiftPage(MyRoomStep),
    /// Gift cell pressed: cell index within the page `0..5`.
    PickGiftCell(usize),
    /// INVENTORY pressed: equip the selected bag cell.
    ApplyEquip,
    /// Vacate pressed: clears the selected worn slot (button ordinal =
    /// equipment slot, ours-by-design) and restages; queues nothing
    /// (send shape OPEN).
    VacateSlot,
    /// BACK pressed: return to the opening lobby.
    BackToLobby,
    /// GOSHOP pressed: records only (nested sides rejected).
    OpenShop,
    /// Gift MOVE pressed: claim the selected gift cell.
    ClaimGift,
    /// MESSENGER pressed: open the branch pinned by the staged
    /// messenger state. No wire send.
    MessengerLiveness,
    /// CONFERENCE pressed: open modal 20. No caption is pinned, so
    /// only the staged sheet art paints. No wire send.
    OpenConference,
    /// WEBAVATAR pressed: show the pinned register notice. Whether
    /// the exe helper also opens a browser stays OPEN; no browser
    /// claim is made here.
    ShowWebAvatar,
}

/// Action for one avatar-room control code (`None` for caption and
/// image codes: text and slot art are not buttons).
#[must_use]
pub fn myroom_action_for_code(code: u32) -> Option<MyRoomAction> {
    if (MYROOM_SLOT_FIRST_CODE..MYROOM_SLOT_FIRST_CODE + MYROOM_SLOT_BUTTONS_U32).contains(&code) {
        let slot = u8::try_from(code - MYROOM_SLOT_FIRST_CODE).ok()?;
        return Some(MyRoomAction::SelectSlot(slot));
    }
    if (MYROOM_BAG_FIRST_CODE..MYROOM_BAG_FIRST_CODE + MYROOM_BAG_CELLS_U32).contains(&code) {
        let cell = usize::try_from(code - MYROOM_BAG_FIRST_CODE).ok()?;
        return Some(MyRoomAction::PickBagCell(cell));
    }
    if (MYROOM_GIFT_FIRST_CODE..MYROOM_GIFT_FIRST_CODE + MYROOM_GIFT_CELLS_U32).contains(&code) {
        let cell = usize::try_from(code - MYROOM_GIFT_FIRST_CODE).ok()?;
        return Some(MyRoomAction::PickGiftCell(cell));
    }
    match code {
        MYROOM_BAG_TAB_CODE => Some(MyRoomAction::ShowBagTab),
        MYROOM_GIFT_TAB_CODE => Some(MyRoomAction::ShowGiftTab),
        MYROOM_BAG_LEFT_CODE => Some(MyRoomAction::BagPage(MyRoomStep::Previous)),
        MYROOM_BAG_RIGHT_CODE => Some(MyRoomAction::BagPage(MyRoomStep::Next)),
        MYROOM_GIFT_LEFT_CODE => Some(MyRoomAction::GiftPage(MyRoomStep::Previous)),
        MYROOM_GIFT_RIGHT_CODE => Some(MyRoomAction::GiftPage(MyRoomStep::Next)),
        MYROOM_APPLY_CODE => Some(MyRoomAction::ApplyEquip),
        MYROOM_VACATE_CODE => Some(MyRoomAction::VacateSlot),
        MYROOM_BACK_CODE => Some(MyRoomAction::BackToLobby),
        MYROOM_GOSHOP_CODE => Some(MyRoomAction::OpenShop),
        MYROOM_MOVE_CODE => Some(MyRoomAction::ClaimGift),
        MYROOM_MESSENGER_CODE => Some(MyRoomAction::MessengerLiveness),
        MYROOM_CONFERENCE_CODE => Some(MyRoomAction::OpenConference),
        MYROOM_WEBAVATAR_CODE => Some(MyRoomAction::ShowWebAvatar),
        _ => None,
    }
}

/// Bag-cell count as `u32` for code arithmetic without casts.
const MYROOM_BAG_CELLS_U32: u32 = 10;
/// Gift-cell count as `u32` for code arithmetic without casts.
const MYROOM_GIFT_CELLS_U32: u32 = 5;

/// Avatar-room action under native pixel (`x`, `y`): the staged
/// `.bnd` placement whose code dispatches and whose rect contains
/// the point. Cell indices count the hit placement among its own
/// family in staged order (file order on staged art), so the painter
/// and the clicker share the same cells with no literals. `None`
/// outside every dispatching placement — unstaged clicks land
/// nowhere (no fallback literals, ADR-0003).
#[must_use]
pub fn myroom_action_at(placements: &[ScenePlacement], x: i32, y: i32) -> Option<MyRoomAction> {
    let mut bag_seen = 0usize;
    let mut gift_seen = 0usize;
    for placed in placements {
        let Some(action) = myroom_action_for_code(placed.code) else {
            continue;
        };
        let indexed = match action {
            MyRoomAction::PickBagCell(_) => {
                let index = bag_seen;
                bag_seen += 1;
                MyRoomAction::PickBagCell(index)
            }
            MyRoomAction::PickGiftCell(_) => {
                let index = gift_seen;
                gift_seen += 1;
                MyRoomAction::PickGiftCell(index)
            }
            _ => action,
        };
        if rect_hit(&placed.rect, x, y) {
            return Some(indexed);
        }
    }
    None
}

/// Equipment slot for one item record (`docs/equip.md` VERIFIED
/// static sites: send-builder file `0xa4ec8`, derivation files
/// `0xa3cc9`/`0xa3cf0`/`0xa3cfd`, range check files
/// `0xa4baa..0xa4bb4`): type 5 forces slot 11, otherwise the record
/// slot byte holds, and the final slot must land below 17.
#[must_use]
pub const fn myroom_equip_slot(item_type: u32, record_slot: u8) -> Option<u8> {
    let slot = if (item_type & 0xFF) == 5 {
        11
    } else {
        record_slot
    };
    if slot < 17 {
        Some(slot)
    } else {
        None
    }
}

/// Record slot byte behind one parsed item row: `Itemdata_Korea.dat`
/// file byte +0x18, the top byte of the third param word (file bytes
/// 21..24 of the 30-byte prefix). The file → in-memory record layout
/// join is ASSUMED — the exe reads come from the looked-up record
/// (`0x52dba0`, `docs/equip.md`) — while the pure rule above is
/// exe-pinned; next step: REA trace of the lookup showing the record
/// layout.
#[must_use]
pub fn myroom_record_slot(row: &AvatarItemRow) -> u8 {
    let word = row.params[2];
    u8::try_from((word >> 24) & 0xFF).unwrap_or(0)
}

/// Well number and item id for each worn 2001 word. The exe's
/// state entry (`FUN_004a3730`, file `0xa3730`, EV `ev_004adf63…`)
/// scans equipment words and stores each at the index named by its
/// item record byte `+0x18` when that byte is below 17; a later word
/// naming the same well wins. This CONTRADICTS the earlier ordinal
/// join (`ee8aa26`, only correct for sequential record bytes): the
/// 7.55 behavior oracle is the record-byte join. OPEN next step: a
/// live non-sequential account with paired wells and 2001 body.
#[must_use]
pub fn myroom_worn_well_items(table: &AvatarItemTable, equipment: &[i32]) -> Vec<(u8, u32)> {
    let mut wells: [Option<u32>; MYROOM_EQUIPMENT_WELLS] = [None; MYROOM_EQUIPMENT_WELLS];
    for word in equipment {
        let Some(item_id) = u32::try_from(*word).ok() else {
            continue;
        };
        let Some(row) = avatar_item_by_id(table, item_id) else {
            continue;
        };
        let well = usize::from(myroom_record_slot(row));
        if let Some(well_item) = wells.get_mut(well) {
            *well_item = Some(item_id);
        }
    }
    wells
        .into_iter()
        .enumerate()
        .filter_map(|(well, item)| {
            let well = u8::try_from(well).ok()?;
            item.map(|item_id| (well, item_id))
        })
        .collect()
}

/// Live avatar-room selection: slot pick, bag page/cursor, gift
/// page/cursor. Slot semantics stay OPEN; the state only records
/// and clamps so painter and clicker share one selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MyRoomState {
    /// Selected slot-selector button ordinal `0..11`.
    selected_slot: u8,
    /// Visible bag page (clamped to the staged page count).
    bag_page: usize,
    /// Selected bag cell within the page (`None` until picked).
    bag_cell: Option<usize>,
    /// Visible gift page (clamped to the staged page count).
    gift_page: usize,
    /// Selected gift cell within the page (`None` until picked).
    gift_cell: Option<usize>,
    /// Visible upper-right panel face (exe default: Inventory).
    panel: MyRoomPanel,
    /// Visible lower-panel tab (exe default: MY BAG).
    lower_tab: MyRoomLowerTab,
    /// Branch of the MESSENGER notice (exe globals, staged here).
    messenger_state: MyRoomMessengerState,
}

impl Default for MyRoomState {
    /// Cleared selection (same as [`MyRoomState::new`]).
    fn default() -> Self {
        Self::new()
    }
}

impl MyRoomState {
    /// Cleared selection: first slot, first pages, no cursors.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected_slot: 0,
            bag_page: 0,
            bag_cell: None,
            gift_page: 0,
            gift_cell: None,
            panel: MyRoomPanel::Inventory,
            lower_tab: MyRoomLowerTab::Bag,
            messenger_state: MyRoomMessengerState::Disconnected,
        }
    }

    /// Selected slot-selector button ordinal.
    #[must_use]
    pub const fn selected_slot(self) -> u8 {
        self.selected_slot
    }

    /// Visible bag page.
    #[must_use]
    pub const fn bag_page(self) -> usize {
        self.bag_page
    }

    /// Selected bag cell within the page, if any.
    #[must_use]
    pub const fn bag_cell(self) -> Option<usize> {
        self.bag_cell
    }

    /// Visible gift page.
    #[must_use]
    pub const fn gift_page(self) -> usize {
        self.gift_page
    }

    /// Selected gift cell within the page, if any.
    #[must_use]
    pub const fn gift_cell(self) -> Option<usize> {
        self.gift_cell
    }

    /// Visible upper-right panel face.
    #[must_use]
    pub const fn panel(self) -> MyRoomPanel {
        self.panel
    }

    /// Visible lower-panel tab.
    #[must_use]
    pub const fn lower_tab(self) -> MyRoomLowerTab {
        self.lower_tab
    }

    /// Switch the panel face. Seam suffices: no live input gesture is
    /// pinned yet (qualified exe press capture OPEN), so no control
    /// maps here; the state keeps both faces reachable for the painter.
    pub fn show_panel(&mut self, panel: MyRoomPanel) {
        self.panel = panel;
    }

    /// Show one lower-panel tab. The MY BAG press is a no-op in the
    /// pinned exe frame, so only the gift tab has a caller today.
    pub fn show_lower_tab(&mut self, tab: MyRoomLowerTab) {
        self.lower_tab = tab;
    }

    /// Selected MESSENGER branch.
    #[must_use]
    pub const fn messenger_state(self) -> MyRoomMessengerState {
        self.messenger_state
    }

    /// Stage the MESSENGER branch named by the exe globals. No wire
    /// send is implied: this only selects which sheet answers a press.
    pub fn set_messenger_state(&mut self, state: MyRoomMessengerState) {
        self.messenger_state = state;
    }

    /// Record a slot-button press; past the eleventh button the
    /// press lands nowhere (button ordinals are `0..11`).
    pub fn select_slot(&mut self, slot: u8) {
        if usize::from(slot) < MYROOM_SLOT_BUTTONS {
            self.selected_slot = slot;
        }
    }

    /// Turn the bag page one step, clamped to `pages` (at least the
    /// first page paints, so an empty count still holds page 0).
    /// Turning clears the cell cursor: rows change under it.
    pub fn turn_bag_page(&mut self, step: MyRoomStep, pages: usize) {
        let last = pages.saturating_sub(1);
        match step {
            MyRoomStep::Previous => {
                self.bag_page = self.bag_page.saturating_sub(1);
            }
            MyRoomStep::Next => {
                self.bag_page = self.bag_page.saturating_add(1).min(last);
            }
        }
        self.bag_cell = None;
    }

    /// Record a bag-cell pick (`None` clears the cursor).
    pub fn pick_bag_cell(&mut self, cell: Option<usize>) {
        if cell.is_none_or(|index| index < MYROOM_BAG_CELLS) {
            self.bag_cell = cell;
        }
    }

    /// Turn the gift page: LEFT wraps from the first page to the last,
    /// RIGHT clamps at the last (the gift pager differs from the bag
    /// pager here — `FUN_004a3ee0` cases `0x15200048/49`, file `0xa3ee0`,
    /// EV `ev_aa875251…`). Turning clears the gift cursor. A one-page
    /// box never moves.
    pub fn turn_gift_page(&mut self, step: MyRoomStep, pages: usize) {
        let last = pages.saturating_sub(1);
        match step {
            MyRoomStep::Previous => {
                self.gift_page = if self.gift_page == 0 {
                    last
                } else {
                    self.gift_page - 1
                };
            }
            MyRoomStep::Next => {
                self.gift_page = self.gift_page.saturating_add(1).min(last);
            }
        }
        self.gift_cell = None;
    }

    /// Record a gift-cell pick (`None` clears the cursor).
    pub fn pick_gift_cell(&mut self, cell: Option<usize>) {
        if cell.is_none_or(|index| index < MYROOM_GIFT_CELLS) {
            self.gift_cell = cell;
        }
    }
}

/// Bag pages over owned rows: at least page 0 paints, so an empty
/// bag still holds its first (blank) page.
#[must_use]
pub fn myroom_bag_pages(inventory: &[i32], table: &AvatarItemTable) -> usize {
    myroom_owned_rows(table, inventory)
        .len()
        .div_ceil(MYROOM_BAG_CELLS)
        .max(1)
}

/// Owned bag rows in inventory order, skipping zero and unknown ids
/// exactly like [`super::my_room_equipment`]: one bad id never blanks the bag.
fn myroom_owned_rows<'a>(table: &'a AvatarItemTable, inventory: &[i32]) -> Vec<ShopItemView<'a>> {
    inventory
        .iter()
        .filter_map(|id| equipped_item_view(table, *id))
        .collect()
}

/// Visible bag rows: the owned run sliced to one page of
/// [`MYROOM_BAG_CELLS`] cells. Past the last page paints nothing.
#[must_use]
pub fn myroom_bag_rows<'a>(
    table: &'a AvatarItemTable,
    inventory: &[i32],
    page: usize,
) -> Vec<ShopItemView<'a>> {
    let owned = myroom_owned_rows(table, inventory);
    let start = page.saturating_mul(MYROOM_BAG_CELLS);
    owned
        .into_iter()
        .skip(start)
        .take(MYROOM_BAG_CELLS)
        .collect()
}

/// One gift-box row: item and song gifts share the page run (items
/// first — order is ours-by-design, no exe sequence is pinned).
/// Names resolve from parsed data (item table, song catalog);
/// unknown ids skip exactly like the equipment rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MyRoomGiftView {
    /// Gift id (`GiftItemInfo`/`GiftMusicInfo`).
    pub gift_id: i32,
    /// Item id or song id being claimed.
    pub resource_id: i32,
    /// Resolved resource name (owned: item table and song catalog
    /// are different sources with different borrows).
    pub name: String,
    /// Gifter nickname.
    pub sender: String,
    /// True for song gifts, false for item gifts.
    pub is_music: bool,
}

/// Gift-box rows in box order (item gifts first, then song gifts)
/// from fetched character data (`GetCharacterInfo` 2001).
#[must_use]
pub fn myroom_gift_views(
    table: &AvatarItemTable,
    songs: &[SelectMusicSong],
    character: &CharacterInfo,
) -> Vec<MyRoomGiftView> {
    let mut views = Vec::new();
    for gift in &character.item_gifts {
        if let Some(view) = gift_item_view(table, gift) {
            views.push(view);
        }
    }
    for gift in &character.music_gifts {
        if let Some(view) = gift_music_view(songs, gift) {
            views.push(view);
        }
    }
    views
}

/// One item gift resolved against the item table (`None` for
/// unknown ids — one bad id never blanks the box).
fn gift_item_view(table: &AvatarItemTable, gift: &GiftItemInfo) -> Option<MyRoomGiftView> {
    let item_id = u32::try_from(gift.item_id).ok()?;
    if item_id == 0 {
        return None;
    }
    let row = avatar_item_by_id(table, item_id)?;
    Some(MyRoomGiftView {
        gift_id: gift.gift_id,
        resource_id: gift.item_id,
        name: row.name.clone(),
        sender: gift.sender.clone(),
        is_music: false,
    })
}

/// One song gift resolved against the staged song catalog (`None`
/// for unlisted songs).
fn gift_music_view(songs: &[SelectMusicSong], gift: &GiftMusicInfo) -> Option<MyRoomGiftView> {
    let wanted = u32::try_from(gift.music_id).ok()?;
    let song = songs.iter().find(|song| song.song_id == wanted)?;
    Some(MyRoomGiftView {
        gift_id: gift.gift_id,
        resource_id: gift.music_id,
        name: song.title.clone(),
        sender: gift.sender.clone(),
        is_music: true,
    })
}

/// Gift-box pages over the staged box run (first page paints even
/// when the box is empty).
#[must_use]
pub fn myroom_gift_pages(views: &[MyRoomGiftView]) -> usize {
    views.len().div_ceil(MYROOM_GIFT_CELLS).max(1)
}

/// Visible gift-box rows: the box run sliced to one page of
/// [`MYROOM_GIFT_CELLS`] cells.
#[must_use]
pub fn myroom_gift_page(views: &[MyRoomGiftView], page: usize) -> &[MyRoomGiftView] {
    let start = page.saturating_mul(MYROOM_GIFT_CELLS);
    let rows = views.get(start..).unwrap_or(&[]);
    rows.get(..rows.len().min(MYROOM_GIFT_CELLS)).unwrap_or(&[])
}

/// Identity captions from fetched character data: nickname, level,
/// cash point (`N PT`), experience, next-up exp (the exe's 99-entry
/// level threshold), win-rate (staged wins/battles percent), and
/// penalties. The numeric formats are exe-pinned at file `0xa7a80`
/// (EV `ev_21ce4a71...`): `" %d"` for level, exp and next-up,
/// `"Lv %d"` for penalty level, `"%d"` for penalty count.
/// Cash wallets live on the
/// single [`WalletState`] now ([`myroom_wallet_captions` paints the
/// O2/MUSIC/ITEM cash controls), so they stay out of this record.
/// Money controls without a pinned source (`MGAME_MONEY`,
/// `O2JAM_MONEY`, `O2MONEY`, `CHARGEMONEY`), rank, guild, free-info,
/// skill counts and notice stay unpainted — their mapping is OPEN
/// (`docs/myroom.md`).
#[must_use]
pub fn myroom_win_rate_text(character: &CharacterInfo) -> String {
    if character.battles <= 0 {
        return "0%".to_owned();
    }
    let wins = i64::from(character.wins.max(0));
    let battles = i64::from(character.battles.max(1));
    let percent = (wins * 100 / battles).clamp(0, 100);
    format!("{percent}%")
}

pub fn myroom_next_up_exp_text(character: &CharacterInfo) -> String {
    next_up_threshold(character.level).map_or_else(String::new, |threshold| format!(" {threshold}"))
}

pub fn myroom_identity_captions(character: &CharacterInfo) -> Vec<(u32, String)> {
    vec![
        (MYROOM_NICKNAME_CODE, character.nickname.clone()),
        (MYROOM_LEVEL_CODE, format!(" {}", character.level)),
        (
            MYROOM_CASH_POINT_CODE,
            format!("{} PT", character.cash_point),
        ),
        (MYROOM_EXP_CODE, format!(" {}", character.experience)),
        (MYROOM_NEXTUPEXP_CODE, myroom_next_up_exp_text(character)),
        (MYROOM_WINRATE_CODE, myroom_win_rate_text(character)),
        (
            MYROOM_PENALTY_COUNT_CODE,
            character.penalty_count.to_string(),
        ),
        (
            MYROOM_PENALTY_LEVEL_CODE,
            format!("Lv {}", character.penalty_level),
        ),
    ]
}

pub const fn myroom_p0_verdict(code: u32) -> Option<&'static str> {
    match code {
        MYROOM_BAGSORT_CODE => {
            Some("BAGSORT P0: no wire send; sort semantics unpinned - visibly inert")
        }
        MYROOM_ALARM_CODE | MYROOM_ALARM_POPUP_CODE => {
            Some("MESSENGER_ALARM P0: image only, no wire read - paints nothing new")
        }
        MYROOM_PCROOMMARK_CODE => {
            Some("PCROOMMARK P0: image only, no wire read - paints nothing new")
        }
        MYROOM_NOTICE_TEXT_CODE | MYROOM_NOTICEBOARD_CODE => {
            Some("NOTICE P0: content OPEN like waiting NOTICE - paints nothing")
        }
        _ => None,
    }
}

/// Money captions from the single wallet: the G and W readouts and
/// the three cash TEXT controls read [`WalletState`], never the
/// character record, so the shop readout and the MyRoom plate stay
/// one source (`--` until the matching wire words arrive; the 2001
/// record and the purchase replies are the only feeds). The 5029
/// body's two words are exe-settled (`docs/shop-buy.md`, file
/// `0x163900`): word 1 feeds the shared gem slot, word 2 the wallet
/// cash slot that inks W.
#[must_use]
pub fn myroom_wallet_captions(state: &WalletState) -> Vec<(u32, String)> {
    vec![
        (MYROOM_MGAME_MONEY_CODE, wallet_amount_caption(state.gem)),
        (
            MYROOM_O2JAM_MONEY_CODE,
            wallet_amount_caption(state.cash_point),
        ),
        (MYROOM_O2CASH_CODE, wallet_amount_caption(state.o2_cash)),
        (
            MYROOM_MUSICCASH_CODE,
            wallet_amount_caption(state.music_cash),
        ),
        (MYROOM_ITEMCASH_CODE, wallet_amount_caption(state.item_cash)),
    ]
}

/// Gift captions for the staged box run: the gift count plus the
/// selected cell's resource name and sender (blank until picked).
/// The cell counts within the visible `page`, matching the painted
/// cells. The count format is ours-by-design (no exe caption bytes
/// pinned).
#[must_use]
pub fn myroom_gift_captions(
    views: &[MyRoomGiftView],
    page: usize,
    cell: Option<usize>,
) -> Vec<(u32, String)> {
    let picked = cell.and_then(|index| myroom_gift_page(views, page).get(index));
    vec![
        (MYROOM_GIFT_NUM_CODE, views.len().to_string()),
        (
            MYROOM_GIFT_ITEMNAME_CODE,
            picked.map_or_else(String::new, |view| view.name.clone()),
        ),
        (
            MYROOM_GIFT_SENDER_CODE,
            picked.map_or_else(String::new, |view| view.sender.clone()),
        ),
    ]
}
