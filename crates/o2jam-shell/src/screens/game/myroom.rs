//! Avatar-room screen behavior moved out of `ScreenGame` (#139).
//!
//! Staging, selection, equip/gift actions, captions, and the room
//! painter live with the other per-screen modules. The methods stay
//! on `ScreenGame`, so every live caller keeps its name and path.

use std::collections::HashMap;

use super::ScreenGame;
use crate::avatar::{avatar_item_by_id, AvatarFigure, ScenePlacement};
use crate::screen::Screen;
use crate::screens::paint::{
    paint_myroom_data, paint_myroom_figure, paint_myroom_slot_icons, paint_screen,
};
use crate::screens::side::{
    myroom_action_at, myroom_bag_pages, myroom_bag_rows, myroom_equip_slot, myroom_gift_captions,
    myroom_gift_page, myroom_gift_pages, myroom_gift_views, myroom_identity_captions,
    myroom_panel_kept, myroom_record_slot, myroom_wallet_captions, MyRoomAction, MyRoomLowerTab,
    MyRoomPanel, MyRoomState, SideScreen, MYROOM_BAG_CELLS,
};
use o2jam_file::AvatarItemTable;
use o2jam_net::gift::{ClaimGift, GiftType};
use o2jam_net::shop::{EquipItem, EquipItemResponse};
use o2jam_net::CharacterInfo;
use o2jam_render::Frame;

/// Notice posted when a worn well's id has no owned-bag record.
/// Exact bytes read from the oracle at file `0x2491f4` (VA
/// `0x6491f4`, cp949); the ticket's "connection lost" gloss is
/// contradicted by the string, so the exe text wins (EV
/// `ev_85bc1fde…`, `docs/myroom.md`).
pub(super) const WORN_WELL_MISS_NOTICE: &str = "최소한 하나의 빈 슬롯이 필요합니다.";

/// Ceiling on the MyRoom per-slot count vector after a 5005 reply.
const MYROOM_COUNT_CAP: u32 = 999;

/// Nickname-item confirm, exact bytes read at file `0x2487f8`.
pub(super) const NICKNAME_ITEM_CONFIRM: &str = "\n현재 사용중이신 닉네임을 \n변경할 수 있는 아이템입니다.\n\n사용 후 아이템은 사라집니다.\n\n아이템을 사용하시겠습니까?";
/// Penalty-reset-item confirm, exact bytes at file `0x248788`.
pub(super) const PENALTY_ITEM_CONFIRM: &str =
    "\n패널티 레벨을 'O' 으로\n초기화 시켜주는 아이템입니다.\n\n사용 후 아이템은 사라집니다.\n\n아이템을 사용하시겠습니까?";
/// Slot-expansion-item confirm, exact bytes at file `0x248718`.
pub(super) const EXPANSION_ITEM_CONFIRM: &str =
    "\n현재 사용중이신 슬롯을 \n확장할 수 있는 아이템입니다.\n\n사용 후 아이템은 사라집니다.\n\n아이템을 사용하시겠습니까?";

/// BAGSORT failure notice, exact cp949 string decoded from the
/// oracle at file `0x248f40` (VA `0x648f40`).
pub(super) const BAG_SORT_FAILURE_NOTICE: &str =
    "마이백 아이템 정렬 실행 중\n오류가 발생되었습니다.\n\n잠시 후 다시 시도해주세요.";

/// Vacate notice at file `0x2491c0` (VA `0x6491c0`).
pub(super) const VACATE_NO_SELECTION_NOTICE: &str = "선택된 아이템이 없습니다.";
/// Vacate notice at file `0x248acc` (VA `0x648acc`).
pub(super) const VACATE_DEFAULT_NOTICE: &str = "기본 아이템은 지울 수 없습니다.";
/// Vacate confirm pattern at file `0x248a90` (VA `0x648a90`).
pub(super) const VACATE_CONFIRM_PATTERN: &str =
    "아이템 : {name}\n\n이 아이템을 비우시면 보상 받을 수 없습니다.";

/// Which local edit an OK on a MyRoom confirm sheet commits. Every
/// arm is own-side: the exe's wire shape for vacate stays OPEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MyRoomConfirmAction {
    /// Clear the named worn item from one equipment slot.
    VacateSlot {
        /// Equipment array index
        slot: usize,
        /// Item id the confirm named; a changed slot is left alone.
        item_id: i32,
    },
    /// Spend one special bag item after its confirm sheet.
    UseSpecialItem {
        /// Special item family, from the record's type byte.
        kind: SpecialItemKind,
        /// Selected bag cell, the exe's first request word.
        selection: i32,
        /// Item id, the exe's second request word where present.
        item_id: i32,
    },
}

/// The three special bag-item families the exe's bag-cell dispatch
/// opens confirms for (`FUN_004a3ee0`, file `0xa41d4`..`0xa42ac`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpecialItemKind {
    /// Record type `0x19`: nickname change (file `0x2487f8`).
    Nickname,
    /// Record type `0x1a`: penalty reset (file `0x248788`).
    PenaltyReset,
    /// Record type `0x1b`: slot expansion (file `0x248718`).
    BagExpansion,
}

impl SpecialItemKind {
    /// Classify one item-record type byte, `None` for ordinary items.
    #[must_use]
    pub(crate) const fn from_item_type(item_type: u32) -> Option<Self> {
        match item_type & 0xFF {
            0x19 => Some(Self::Nickname),
            0x1A => Some(Self::PenaltyReset),
            0x1B => Some(Self::BagExpansion),
            _ => None,
        }
    }

    /// Exact confirm caption read from the oracle's byte tables.
    #[must_use]
    pub(crate) const fn confirm_text(self) -> &'static str {
        match self {
            Self::Nickname => NICKNAME_ITEM_CONFIRM,
            Self::PenaltyReset => PENALTY_ITEM_CONFIRM,
            Self::BagExpansion => EXPANSION_ITEM_CONFIRM,
        }
    }
}

/// One MyRoom confirm sheet: the exe's exact caption plus the local
/// edit its OK commits. CANCEL only closes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MyRoomConfirm {
    /// Caption inked into the staged TEXT rect
    pub(crate) text: String,
    /// Local edit queued behind the OK press
    pub(crate) action: MyRoomConfirmAction,
}

impl ScreenGame {
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
        // The character record feeds the single wallet (#202): its
        // cash lands here, and every money caption paints from the
        // wallet afterwards — never from this record again.
        self.wallet.apply_character_cash(&character);
        self.myroom_character = Some(character);
        self.restage_shop_avatar();
        self
    }

    /// Fetched character data (`GetCharacterInfo` 2001), if the live
    /// boot staged it (`None` until the windowed entries fetch on
    /// channel login).
    #[must_use]
    pub fn myroom_character(&self) -> Option<&CharacterInfo> {
        self.myroom_character.as_ref()
    }

    /// Live avatar-room selection (slot pick, bag/gift pages+cursors).
    #[must_use]
    pub const fn myroom(&self) -> MyRoomState {
        self.myroom
    }

    /// Show the other upper-right MyRoom face. Seam suffices: the exe
    /// pack pins the default face but no switching gesture, so no
    /// staged control maps here yet (capture OPEN in `docs/myroom.md`).
    pub fn show_myroom_panel(&mut self, panel: MyRoomPanel) -> &mut Self {
        self.myroom.show_panel(panel);
        self
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
            MyRoomAction::ShowBagTab => {
                // Exe3 `8515a7e5…`: the MY BAG press leaves the gift
                // panel open, so no lower-tab or panel state changes.
            }
            MyRoomAction::ShowGiftTab => self.myroom.show_lower_tab(MyRoomLowerTab::Gifts),
            MyRoomAction::BagPage(step) => {
                let pages = self.myroom_bag_page_count();
                self.myroom.turn_bag_page(step, pages);
            }
            MyRoomAction::PickBagCell(cell) => self.apply_myroom_bag_pick(cell),
            MyRoomAction::GiftPage(step) => {
                let pages = self.myroom_gift_page_count();
                self.myroom.turn_gift_page(step, pages);
            }
            MyRoomAction::PickGiftCell(cell) => self.myroom.pick_gift_cell(Some(cell)),
            MyRoomAction::PressWornWell(well) => self.apply_myroom_well_press(well),
            MyRoomAction::ApplyEquip => self.apply_myroom_equip(),
            MyRoomAction::ClaimGift => self.apply_myroom_claim(),
            MyRoomAction::VacateSlot => self.apply_myroom_vacate(),
            MyRoomAction::BagSort => self.apply_myroom_bag_sort(),
            MyRoomAction::MessengerLiveness => self.messenger_liveness_open = true,
            MyRoomAction::OpenShop | MyRoomAction::BackToLobby => {}
        }
    }

    /// Pick one owned bag cell: record the cursor and queue the
    /// exe's 5004 immediately. The send case (file `0xa4bb6`..`0xa4bcb`)
    /// derives the slot from the record's type/slot bytes and passes
    /// the clicked bag index as the second word; the worn id it reads
    /// is never appended. The server's 5005 then owns the visible
    /// restage, so the pick itself mirrors no local equipment.
    fn apply_myroom_bag_pick(&mut self, cell: usize) {
        self.myroom.pick_bag_cell(Some(cell));
        let Some(row_item_id) = self.myroom_items.as_ref().and_then(|table| {
            self.myroom_character.as_ref().and_then(|character| {
                myroom_bag_rows(table, &character.inventory, self.myroom.bag_page())
                    .get(cell)
                    .map(|row| row.item_id)
            })
        }) else {
            return;
        };
        let Some((item_type, record_slot)) = self
            .myroom_items
            .as_ref()
            .and_then(|table| avatar_item_by_id(table, row_item_id))
            .map(|row| (row.item_type, myroom_record_slot(row)))
        else {
            return;
        };
        let Some(slot) = myroom_equip_slot(item_type, record_slot) else {
            return;
        };
        let global_index = self
            .myroom
            .bag_page()
            .saturating_mul(MYROOM_BAG_CELLS)
            .saturating_add(cell);
        let Ok(bag_index) = i32::try_from(global_index) else {
            return;
        };
        self.outbox.pending_equip = Some(EquipItem {
            slot: i32::from(slot),
            item_id: bag_index,
        });
        if let Some(kind) = SpecialItemKind::from_item_type(item_type) {
            self.myroom_notice = None;
            self.myroom_confirm = Some(MyRoomConfirm {
                text: kind.confirm_text().to_owned(),
                action: MyRoomConfirmAction::UseSpecialItem {
                    kind,
                    selection: bag_index,
                    item_id: i32::try_from(row_item_id).unwrap_or(0),
                },
            });
        }
    }

    /// Apply a successful 5005 equip reply: the exe repaints the wells
    /// and bag from its staged records, advances the count vector for
    /// the answered slot (capped at 999, file `0x4a54e0` reply arm)
    /// and clears the cell cursor now that the rows have moved.
    /// Painting is already a projection of staged data; this seam
    /// changes only the server-confirmed state.
    pub fn apply_myroom_equip_reply(&mut self, response: &EquipItemResponse) {
        let Some(tail) = response.tail else {
            return;
        };
        if let Ok(slot) = usize::try_from(tail.slot_select) {
            if let Some(count) = self.myroom_counts.get_mut(slot) {
                *count = count.saturating_add(1).min(MYROOM_COUNT_CAP);
            }
        }
        self.myroom.pick_bag_cell(None);
    }

    /// BAGSORT press: queue the empty 5038 RefershBag frame. The
    /// exe's send builder writes only the opcode and length word for
    /// this case (`0x4a540f`, file `0xa540f`); the 5039 reply repaints
    /// the bag and the failure string is filed at `0x248f40`.
    fn apply_myroom_bag_sort(&mut self) {
        self.outbox.queue_myroom_send(5038, Vec::new());
    }

    /// Equip the selected bag cell: the slot derives from the item
    /// record exactly like the exe derives it (`docs/equip.md` —
    /// type 5 forces slot 11, else the record slot byte, below 17),
    /// then the 5004 body queues for the session bridge and the
    /// staged worn set mirrors it. Without a picked cell, staged
    /// rows, or a derivable slot the press records nothing.
    pub(super) fn apply_myroom_equip(&mut self) {
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

    /// Press one worn equipment well: the exe passes the well
    /// ordinal as the 5004 slot word and the owned-bag index its
    /// worn-id selector finds as the second word (`FUN_004a49f0`,
    /// files `0xa4aec`/`0xa4af4`; selector `0x537dd0`). A missing id
    /// posts the file `0x2491f4` notice and queues no send. The live
    /// mouse event reaches this seam through the staged well rect;
    /// which native event fires the exe handler stays OPEN (#793).
    fn apply_myroom_well_press(&mut self, well: u8) {
        let Some(character) = self.myroom_character.as_ref() else {
            return;
        };
        let Some(worn_id) = character.equipment.get(usize::from(well)).copied() else {
            return;
        };
        if worn_id == 0 {
            return;
        }
        let Some(bag_index) = character.inventory.iter().position(|id| *id == worn_id) else {
            self.myroom_notice = Some(WORN_WELL_MISS_NOTICE.to_owned());
            return;
        };
        let Ok(item_id) = i32::try_from(bag_index) else {
            return;
        };
        self.outbox.pending_equip = Some(EquipItem {
            slot: i32::from(well),
            item_id,
        });
    }

    /// Vacate the selected slot per exe case `0x15200004`: an empty
    /// cell posts the file `0x2491c0` notice, ids `0x23/0x24` the file
    /// `0x248acc` refusal, and any other item opens the file `0x248a90`
    /// no-compensation confirm. Only its OK clears the same-ordinal
    /// equipment cell and restages through the #193 hook. The
    /// button-ordinal to equipment-slot identity is ours-by-design;
    /// no vacate wire shape is pinned (OPEN, `docs/myroom.md`).
    fn apply_myroom_vacate(&mut self) {
        let slot = usize::from(self.myroom.selected_slot());
        let Some(item_id) = self
            .myroom_character
            .as_ref()
            .and_then(|character| character.equipment.get(slot))
            .copied()
        else {
            return;
        };
        if item_id == 0 {
            self.myroom_notice = Some(VACATE_NO_SELECTION_NOTICE.to_owned());
            self.myroom_confirm = None;
            return;
        }
        if item_id == 0x23 || item_id == 0x24 {
            self.myroom_notice = Some(VACATE_DEFAULT_NOTICE.to_owned());
            self.myroom_confirm = None;
            return;
        }
        let item_name = self
            .myroom_items
            .as_ref()
            .and_then(|table| {
                u32::try_from(item_id)
                    .ok()
                    .and_then(|id| avatar_item_by_id(table, id))
            })
            .map_or_else(|| item_id.to_string(), |row| row.name.clone());
        self.myroom_notice = None;
        self.myroom_confirm = Some(MyRoomConfirm {
            text: VACATE_CONFIRM_PATTERN.replace("{name}", &item_name),
            action: MyRoomConfirmAction::VacateSlot { slot, item_id },
        });
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

    /// MyRoom money captions from the single wallet (the O2/MUSIC/
    /// ITEM cash TEXT controls): the same state the shop readout and
    /// both post-purchase refreshes share.
    #[must_use]
    pub fn myroom_money_captions(&self) -> Vec<(u32, String)> {
        myroom_wallet_captions(&self.wallet)
    }

    /// Avatar room: staged art when present, else the flat card.
    /// Staged character and item rows paint the bag names, gift
    /// names, and identity captions into their staged `.bnd` rects;
    /// without staged data the art paints alone (oracle-neutral).
    /// Slot selection, equip commit, bag/gift pagers, and gift claim
    /// ride [`Self::click_myroom_side`]; vacate clears the selected worn
    /// slot and restages but queues nothing (send shape OPEN,
    /// `docs/myroom.md`); goshop/shop-hop send nothing (same OPEN).
    pub(super) fn paint_myroom(&self, frame: &mut Frame<'_>, now_ms: u64) {
        match self.scene_art(Screen::Myroom) {
            Some(art) => {
                let panel = self.myroom.panel();
                art.paint_backdrop_live(frame, Screen::Myroom, |placed| {
                    myroom_panel_kept(placed, panel)
                });
                paint_myroom_figure(frame, &art.placements, self.myroom_worn_figure(), now_ms);
                self.paint_myroom_overlay(frame, &art.placements, &art.figures);
            }
            None => paint_screen(frame, Screen::Myroom),
        }
    }

    /// Staged worn figure behind the avatar-room portrait: the same
    /// stack the Room lobby slot and the Result banner show (kept live
    /// by the login staging and the equip restage hook), so the MyRoom
    /// portrait always wears the current equipment. `None` until the
    /// live boot stages a character with a stackable archive.
    pub(super) fn myroom_worn_figure(&self) -> Option<&AvatarFigure> {
        self.room_avatar.as_ref()
    }

    /// Avatar-room rows and captions over staged placements: the
    /// visible bag names, gift names, and identity/gift captions.
    /// Reads staged data only — without a staged item table and
    /// character nothing paints (no fallback geometry is invented).
    pub(super) fn paint_myroom_overlay(
        &self,
        frame: &mut Frame<'_>,
        placements: &[ScenePlacement],
        figures: &HashMap<String, AvatarFigure>,
    ) {
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
        paint_myroom_slot_icons(
            frame,
            placements,
            &character.equipment,
            &self.shop_item_portraits,
        );
        // Identity fields belong to the status face; the exe's default
        // Inventory face hides them with the rest of that plate (#369).
        let mut captions = if self.myroom.panel() == MyRoomPanel::Status {
            myroom_identity_captions(character)
        } else {
            Vec::new()
        };
        // Money paints from the single wallet (#202): the character
        // record fed it at staging, purchases refresh it after.
        captions.extend(myroom_wallet_captions(&self.wallet));
        captions.extend(myroom_gift_captions(
            &views,
            self.myroom.gift_page(),
            self.myroom.gift_cell(),
        ));
        paint_myroom_data(
            frame,
            placements,
            figures,
            self.myroom,
            &bag,
            &gifts,
            &captions,
        );
    }
}
