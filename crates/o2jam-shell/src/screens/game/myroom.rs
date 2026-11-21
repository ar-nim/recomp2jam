//! Avatar-room screen behavior moved out of `ScreenGame` (#139).
//!
//! Staging, selection, equip/gift actions, captions, and the room
//! painter live with the other per-screen modules. The methods stay
//! on `ScreenGame`, so every live caller keeps its name and path.

use std::collections::HashMap;

use super::shop::EquippedAvatarBlock;
use super::ScreenGame;
use crate::avatar::{avatar_item_by_id, AvatarFigure, ScenePlacement};
use crate::screen::Screen;
use crate::screens::paint::{
    paint_myroom_data, paint_myroom_figure, paint_myroom_slot_icons, paint_screen,
};
use crate::screens::side::{
    myroom_action_at, myroom_bag_pages, myroom_bag_rows, myroom_equip_slot, myroom_gift_captions,
    myroom_gift_page, myroom_gift_pages, myroom_gift_views, myroom_identity_captions,
    myroom_panel_kept, myroom_record_slot, myroom_wallet_captions, myroom_worn_well_items,
    MyRoomAction, MyRoomLowerTab, MyRoomNotice, MyRoomPanel, MyRoomState, SideScreen,
};
use o2jam_file::AvatarItemTable;
use o2jam_net::gift::{ClaimGift, GiftType};
use o2jam_net::shop::EquipItem;
use o2jam_net::CharacterInfo;
use o2jam_render::Frame;

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
        self.equipped_block = Some(EquippedAvatarBlock::from_character(&character));
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
            MyRoomAction::PickBagCell(cell) => self.myroom.pick_bag_cell(Some(cell)),
            MyRoomAction::GiftPage(step) => {
                let pages = self.myroom_gift_page_count();
                self.myroom.turn_gift_page(step, pages);
            }
            MyRoomAction::PickGiftCell(cell) => self.myroom.pick_gift_cell(Some(cell)),
            MyRoomAction::ApplyEquip => self.apply_myroom_equip(),
            MyRoomAction::ClaimGift => self.apply_myroom_claim(),
            MyRoomAction::VacateSlot => self.apply_myroom_vacate(),
            MyRoomAction::MessengerLiveness => {
                self.post_myroom_notice(MyRoomNotice::MessengerUnavailable);
            }
            MyRoomAction::OpenShop | MyRoomAction::BackToLobby => {}
        }
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

    /// Vacate the selected slot: the same-ordinal equipment cell
    /// clears to 0 and the worn figure restages through the #193 hook.
    /// The button-ordinal to equipment-slot identity is ours-by-design
    /// (the 11 staged buttons name no equipment slot in the block dump;
    /// equip slots 11..16 have no button) — next step: rig walk of the
    /// vacate gesture naming the slot and any wire send. Queues nothing:
    /// the vacate send shape is unpinned (OPEN, `docs/myroom.md`).
    fn apply_myroom_vacate(&mut self) {
        let slot = usize::from(self.myroom.selected_slot());
        if let Some(cell) = self
            .myroom_character
            .as_mut()
            .and_then(|character| character.equipment.get_mut(slot))
        {
            *cell = 0;
        }
        self.restage_worn_figure();
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
            &myroom_worn_well_items(table, &character.equipment),
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
