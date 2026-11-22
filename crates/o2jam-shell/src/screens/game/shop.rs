//! Item-shop goods, pages, and the checked cart behind the shop
//! side screen (#139 cluster 3).

use super::super::side::{
    lanking_page_window, shop_checkout_decision, shop_goods, shop_next_planet_index,
    shop_page_count, shop_try_on_decision, CheckoutDecision, CheckoutGate, CheckoutMode,
    CheckoutNotice, ShopAction, ShopBagNotice, ShopChromeNotice, ShopGender, ShopGood,
    ShopMessengerNotice, ShopMessengerState, ShopPanel, ShopRefresh, ShopRefreshContext,
    TryOnDecision, TryOnGate, TryOnNotice, WalletState, SHOP_BAG_CELLS, SHOP_CART_ROWS,
    SHOP_ITEM_ROWS, SHOP_PLANET_FIRST_INDEX, SHOP_PLANET_GRID_CELLS, SHOP_PLANET_LAST_INDEX,
    SHOP_PLANET_LAYOUT_FLAGS,
};
use o2jam_net::{CharacterInfo, EQUIPMENT_SLOTS};
use std::collections::HashMap;

/// Equipped block the ITEMBACK arm restores. The exe takes `0x2190`
/// bytes from global `0x77cd00` and writes them over the local copy at
/// object `+0x65` before repainting (`ev_f3ad15be…`, arm
/// `0x1420006c`): a whole-block revert, never a per-part undo. Only
/// the worn-equipment slice is modeled here — the rest of that block
/// (wallet, inventory) belongs to server-authoritative state and is
/// never minted locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquippedAvatarBlock {
    equipment: [i32; EQUIPMENT_SLOTS],
}

impl EquippedAvatarBlock {
    /// Snapshot the equipped slice when a character stages.
    pub(crate) fn from_character(character: &CharacterInfo) -> Self {
        Self {
            equipment: character.equipment,
        }
    }

    /// Overwrite every equipped slot from the snapshot.
    pub(crate) fn restore_equipment(&self, character: &mut CharacterInfo) {
        character.equipment = self.equipment;
    }
}

/// Item-shop goods, pages, and the checked cart behind the shop
/// side screen (#139 cluster 3: the Data Clump of all four
/// `shop_*` fields). Pure board state — no side dispatch, paint,
/// or wire queue — so the board reads without touching the
/// `ScreenGame` God class. `ScreenGame` keeps thin delegates under
/// the same names, so the click seam and the snapshot surface never
/// change. The cart commit reports itself to the caller (cluster 1):
/// `apply_action` opens no wire queue; the caller mounts the local web
/// purchase surface instead (#397).
#[derive(Default)]
pub(crate) struct ShopState {
    /// Lower-left panel face (`MY BAG` on entry, `SHOPPING BAG`
    /// after a row pick; audit D4).
    pub(crate) panel: ShopPanel,
    /// Item-shop goods (`Itemdata_Korea.dat` order, owned copies;
    /// see [`Self::set_shop_catalog`]).
    pub(crate) shop_goods: Vec<ShopGood>,
    /// Item-list page (eight rows per page, the staged `ITEMBUY`
    /// hit areas).
    pub(crate) shop_page: usize,
    /// Checked cart item ids in toggle order (the `ITEMBUY`
    /// selection behind the cart `BUYITEM` commit).
    pub(crate) shop_cart: Vec<u32>,
    /// Previewed catalog item id (`ITEMPRE` try-on, ticket #196):
    /// paints in the avatar pane without touching the equipped set
    /// or queueing wire. `None` shows the equipped avatar.
    pub(crate) shop_try_on: Option<u32>,
    /// Cart-list page (five rows per page, the staged `CART_EX`
    /// slots).
    pub(crate) shop_cart_page: usize,
    /// Planet index `0x17` (1..=12). Our board opens on planet 1;
    /// the exe's entry word is unpinned.
    planet_index: u8,
    /// Number of staged nonzero cells in the exe's 12×3 grid.
    planet_grid_nonzero: usize,
    /// Whether the figure-read helper arms `FUN_0042f870`.
    planet_caption_armed: bool,
    /// Caption setter argument from the last planet turn.
    planet_caption_arg: Option<u8>,
    /// Cells hidden by the last planet turn's first grid pass.
    planet_hidden_cells: usize,
    /// `FUN_00411e60` calls on the last turn (two: `+0x8d2/+0x8d3`).
    planet_reseeds: u32,
    /// Three layout words reset after the seeds (`+0x8d4..6`).
    planet_layout_flags: [u8; 3],
    /// Refresher call made by the last planet press.
    last_planet_refresh: Option<ShopRefresh>,
    /// Gender byte `+0x18` written by the GIRL/BOY arms (`None`
    /// before the first press — the entry byte is unpinned).
    gender: Option<ShopGender>,
    /// Which context refresher the object state selects.
    refresh_context: ShopRefreshContext,
    /// The exe's "lucky" word (1 at planet 12), staged by the
    /// planet arm; passed to every context refresher.
    lucky_view: bool,
    /// Last refresher call from a gender press (test affordance).
    last_gender_refresh: Option<ShopRefresh>,
    /// Selected category tab (`None` on entry). The FASHION press
    /// frame `d4074ce7…` moves only the tab highlight; no wire frame
    /// and no catalogue filter rule is pinned yet.
    pub(crate) shop_category: Option<usize>,
    /// Previewed item id (`ITEMPRE` pick, `ITEMBACK` clears; `None`
    /// until picked). Ids stay stable across pages (unlike slots).
    pub(crate) preview: Option<u32>,
    /// Try-on gate words staged per item id (`+4`/`+6` of the exe
    /// record). The file→record join is not pinned, so an ungated
    /// good never stages a figure.
    try_on_gates: HashMap<u32, TryOnGate>,
    /// Active worn slot (exe global `0x77cd28`) and its occupancy
    /// word (object `+0x8ad`); slot 2 bypasses both slot checks.
    active_worn_slot: Option<u8>,
    occupied_worn_slot: bool,
    /// Last notice the ITEMPRE gate posted (`None` on a clean try-on
    /// or ITEMBACK). The notice sheet that mounts these bytes stays
    /// OPEN.
    last_try_on_notice: Option<TryOnNotice>,
    /// Staged BUYITEM/GIFTITEM gate inputs and the resulting mode
    /// word (`+0x969`) plus notice. The 5015 send itself belongs to
    /// the outbox, not this board state.
    checkout_gate: CheckoutGate,
    payment_mode: Option<CheckoutMode>,
    checkout_notice: Option<CheckoutNotice>,
    /// Owned-panel page (ten `MYBAGITEM` icons per page, the staged
    /// `BAGLEFT`/`BAGRIGHT` pager).
    pub(crate) bag_page: usize,
    /// Exe's selected row word `DAT_0077ccc0` (global owned index,
    /// `None` is the exe's cleared `-1`).
    bag_selection: Option<usize>,
    /// Notice posted by the last bag-cell press.
    bag_notice: Option<ShopBagNotice>,
    /// Ext-menu state `0x5b` (true: filters shown).
    ext_menu_open: bool,
    /// Gate word `0x5f`. The exe writer is unpinned; a staged
    /// EXTMENU control is our ready seam.
    ext_menu_ready: bool,
    /// Accepted toggle count (gate-closed presses do not toggle).
    ext_menu_toggles: u32,
    /// `FUN_00411e60` calls on ext-menu toggles (one per press).
    ext_menu_reseeds: u32,
    /// Word `0x5d`, cleared on every open.
    ext_menu_secondary_word: u8,
    /// Notice posted by ITEMVACATE or CONFERENCE.
    chrome_notice: Option<ShopChromeNotice>,
    /// Messenger branch selected by the staged read of global `0x7684d8`.
    messenger_state: ShopMessengerState,
    /// Notice posted by the last MESSENGER press.
    messenger_notice: Option<ShopMessengerNotice>,
    /// Staged confirmation-pair result for the vacate ladder.
    vacate_confirmed: bool,
    /// Staged item code for the vacate ladder (`0x23`/`0x24` branch).
    vacate_item_code: u32,
    /// Staged item word inked into the type-2 vacate string.
    vacate_item_word: u32,
    /// Staged category bytes for owned items (`+4` of the exe entry).
    bag_item_categories: HashMap<u32, u8>,
    /// Staged worn-slot occupancy for owned items.
    bag_item_worn: HashMap<u32, bool>,
    /// Staged special-item pair that refuses a selection.
    bag_item_locked: HashMap<u32, bool>,
}

impl ShopState {
    /// Stage the item-shop catalog (`Itemdata_Korea.dat` order,
    /// owned copies so the board outlives the parse borrow). Resets
    /// the item page, the preview pick, and the owned-panel page;
    /// the cart keeps its checks.
    pub(crate) fn set_catalog(&mut self, table: &o2jam_file::AvatarItemTable) {
        self.panel = ShopPanel::MyBag;
        self.shop_goods = shop_goods(table);
        self.shop_page = 0;
        self.preview = None;
        self.bag_page = 0;
        self.shop_category = None;
        self.bag_selection = None;
        self.bag_notice = None;
        self.chrome_notice = None;
        self.messenger_notice = None;
    }

    /// Item-list page (eight rows per page).
    pub(crate) fn page(&self) -> usize {
        self.shop_page
    }

    /// Cart-list page (five rows per page).
    pub(crate) fn cart_page(&self) -> usize {
        self.shop_cart_page
    }

    /// Cart flag (exe object `+0x8cd`): only the SHOPPING BAG face
    /// answers MYCART_LEFT/RIGHT. MYBACK clears it, MYCART_POS sets
    /// it, and a row check enters the face (`ev_f3ad15be…`).
    pub(crate) fn cart_face_open(&self) -> bool {
        self.panel == ShopPanel::ShoppingBag
    }

    /// Current planet index (1..=12).
    pub(crate) fn planet_index(&self) -> u8 {
        self.planet_index.max(SHOP_PLANET_FIRST_INDEX)
    }

    /// Argument the last planet turn passed to `FUN_0042f870`.
    pub(crate) fn planet_caption_arg(&self) -> Option<u8> {
        self.planet_caption_arg
    }

    /// Grid cells hidden by the last planet turn.
    pub(crate) fn planet_hidden_cells(&self) -> usize {
        self.planet_hidden_cells
    }

    /// Reseed count from the last planet turn (two in the exe).
    pub(crate) fn planet_reseeds(&self) -> u32 {
        self.planet_reseeds
    }

    /// Three layout words after the last planet turn.
    pub(crate) fn planet_layout_flags(&self) -> [u8; 3] {
        self.planet_layout_flags
    }

    /// Refresher call made by the last planet press.
    pub(crate) fn planet_refresh(&self) -> Option<ShopRefresh> {
        self.last_planet_refresh
    }

    /// Stage how many 12×3 grid cells currently hold planet art.
    pub(crate) fn stage_planet_grid_nonzero(&mut self, cells: usize) {
        self.planet_grid_nonzero = cells.min(SHOP_PLANET_GRID_CELLS);
    }

    /// Stage whether the caption setter is armed by the figure read.
    pub(crate) fn stage_planet_caption_armed(&mut self, armed: bool) {
        self.planet_caption_armed = armed;
    }

    /// Gender byte `+0x18` staged by the last GIRL/BOY press.
    pub(crate) fn gender(&self) -> Option<ShopGender> {
        self.gender
    }

    /// Button the last gender press hides (the "other" button).
    pub(crate) fn hidden_gender_button(&self) -> Option<u32> {
        self.gender.map(ShopGender::hidden_button)
    }

    /// Refresher call made by the last gender press.
    pub(crate) fn gender_refresh(&self) -> Option<ShopRefresh> {
        self.last_gender_refresh
    }

    /// Stage which object context selects the press refresher. The
    /// screen→object-word mapping stays OPEN (frame pair).
    pub(crate) fn stage_refresh_context(&mut self, context: ShopRefreshContext) {
        self.refresh_context = context;
    }

    /// Stage the "lucky" word (planet index 12 sets it).
    pub(crate) fn stage_lucky_view(&mut self, lucky_view: bool) {
        self.lucky_view = lucky_view;
    }

    /// Selected category tab (`None` until a FASHION-class press).
    pub(crate) fn category(&self) -> Option<usize> {
        self.shop_category
    }

    /// Checked cart item ids in toggle order.
    pub(crate) fn cart(&self) -> Vec<u32> {
        self.shop_cart.clone()
    }

    /// Stage the exe's try-on gate words for one catalog good.
    pub(crate) fn stage_try_on_gate(&mut self, item_id: u32, category: u8, worn_slot: u8) {
        if self.shop_goods.iter().any(|good| good.item_id == item_id) {
            self.try_on_gates.insert(
                item_id,
                TryOnGate {
                    category,
                    worn_slot,
                },
            );
        }
    }

    /// Stage the occupied-slot word (exe object `+0x8ad`).
    pub(crate) fn stage_occupied_worn_slot(&mut self, occupied: bool) {
        self.occupied_worn_slot = occupied;
    }

    /// Notice posted by the last try-on press, if any.
    pub(crate) fn try_on_notice(&self) -> Option<TryOnNotice> {
        self.last_try_on_notice
    }

    /// Stage the exe's gem-price word for one good. The parsed table
    /// carries no gem word, so only an explicitly staged pin may set
    /// it (#395).
    pub(crate) fn stage_item_gem_price(&mut self, item_id: u32, price: u32) {
        if let Some(good) = self
            .shop_goods
            .iter_mut()
            .find(|good| good.item_id == item_id)
        {
            good.gem_price = Some(price);
        }
    }

    /// Stage the owned-bag capacity used by the BUYITEM ladder.
    pub(crate) fn stage_checkout_bag_capacity(&mut self, slots: usize) {
        self.checkout_gate.bag_slots = Some(slots);
    }

    /// Stage the owned-bag rows already used (`0x77cce4` list count).
    pub(crate) fn stage_checkout_bag_used(&mut self, used: usize) {
        self.checkout_gate.bag_used = used;
    }

    /// Stage the web-busy gate (`FUN_00556680(0)`).
    pub(crate) fn stage_payment_web_busy(&mut self, busy: bool) {
        self.checkout_gate.web_busy = busy;
    }

    /// Mode word written at `+0x969` by the last approved press.
    pub(crate) fn payment_mode(&self) -> Option<CheckoutMode> {
        self.payment_mode
    }

    /// Notice posted by the last checkout press, if any.
    pub(crate) fn checkout_notice(&self) -> Option<CheckoutNotice> {
        self.checkout_notice
    }

    /// Run the exe's validation ladder. Returns `true` only when the
    /// mode word was written and the caller must queue 5015.
    pub(crate) fn approve_checkout(&mut self, mode: CheckoutMode, wallet: &WalletState) -> bool {
        let cart: Vec<ShopGood> = self
            .shop_cart
            .iter()
            .filter_map(|item_id| {
                self.shop_goods
                    .iter()
                    .find(|good| good.item_id == *item_id)
                    .cloned()
            })
            .collect();
        let decision = shop_checkout_decision(&cart, wallet, &self.checkout_gate, mode);
        match decision {
            CheckoutDecision::Approved(approved) => {
                self.payment_mode = Some(approved);
                self.checkout_notice = None;
                true
            }
            CheckoutDecision::Notice(notice) => {
                self.checkout_notice = Some(notice);
                false
            }
            CheckoutDecision::Silent | CheckoutDecision::Unpinned => false,
        }
    }

    /// Summed `ITEMWON` prices over the checked cart (saturating;
    /// unknown ids drop out like the cart rows do).
    pub(crate) fn cart_total(&self) -> u32 {
        self.shop_cart
            .iter()
            .filter_map(|item_id| self.shop_goods.iter().find(|good| good.item_id == *item_id))
            .map(|good| good.price)
            .fold(0u32, |total, price| total.saturating_add(price))
    }

    /// Previewed catalog item id (`None`: equipped avatar shows).
    pub(crate) fn try_on(&self) -> Option<u32> {
        self.shop_try_on
    }

    /// Preview the visible row's good (`ITEMPRE` try-on): applies the
    /// pinned category/slot/occupancy gate, stages the id only when
    /// the exe would repaint, and records a notice otherwise. Never
    /// touches the cart or the wire.
    pub(crate) fn preview_slot(&mut self, slot: usize) -> bool {
        let Some(good) = self.visible_goods().get(slot).cloned() else {
            return false;
        };
        let gate = self.try_on_gates.get(&good.item_id).copied();
        match shop_try_on_decision(gate, self.active_worn_slot, self.occupied_worn_slot) {
            TryOnDecision::Stage => {
                self.shop_try_on = Some(good.item_id);
                if let Some(gate) = gate {
                    self.active_worn_slot = Some(gate.worn_slot);
                }
                self.last_try_on_notice = None;
                true
            }
            TryOnDecision::WrongCategory => {
                self.last_try_on_notice = None;
                false
            }
            TryOnDecision::Notice(notice) => {
                self.last_try_on_notice = Some(notice);
                false
            }
        }
    }

    /// Revert the preview to the equipped avatar (`ITEMBACK`).
    pub(crate) fn clear_try_on(&mut self) {
        self.shop_try_on = None;
        self.last_try_on_notice = None;
    }

    /// Check one catalog good into the cart (id must exist; the
    /// click seam feeds the same cart — this stages restores).
    pub(crate) fn cart_push(&mut self, item_id: u32) {
        if self.shop_goods.iter().any(|good| good.item_id == item_id)
            && !self.shop_cart.contains(&item_id)
        {
            self.shop_cart.push(item_id);
        }
    }

    /// Previewed good, if the picked id still catalogs (`None`
    /// until an `ITEMPRE` pick lands, or after `ITEMBACK`).
    pub(crate) fn preview_good(&self) -> Option<ShopGood> {
        let picked = self.preview?;
        self.shop_goods
            .iter()
            .find(|good| good.item_id == picked)
            .cloned()
    }

    /// Pick one visible row for preview (`ITEMPRE` slot on the
    /// current page; unknown slots keep the old pick).
    pub(crate) fn select_preview(&mut self, slot: usize) {
        if let Some(good) = self.visible_goods().get(slot).cloned() {
            self.preview = Some(good.item_id);
        }
    }

    /// Clear the preview pick (`ITEMBACK`).
    pub(crate) fn clear_preview(&mut self) {
        self.preview = None;
    }

    /// Whether the 12 ext-menu filters are currently shown.
    pub(crate) fn ext_menu_open(&self) -> bool {
        self.ext_menu_open
    }

    /// Accepted ext-menu toggle count.
    pub(crate) fn ext_menu_toggles(&self) -> u32 {
        self.ext_menu_toggles
    }

    /// Reseeds made by ext-menu toggles.
    pub(crate) fn ext_menu_reseeds(&self) -> u32 {
        self.ext_menu_reseeds
    }

    /// Word `0x5d` after the last toggle.
    pub(crate) fn ext_menu_secondary_word(&self) -> u8 {
        self.ext_menu_secondary_word
    }

    /// Stage the gate word `0x5f` that allows an ext-menu toggle.
    pub(crate) fn stage_ext_menu_ready(&mut self, ready: bool) {
        self.ext_menu_ready = ready;
    }

    /// Notice posted by the last ITEMVACATE/CONFERENCE press.
    pub(crate) fn chrome_notice(&self) -> Option<ShopChromeNotice> {
        self.chrome_notice
    }

    /// Stage the branch read from messenger global `0x7684d8`.
    pub(crate) fn stage_messenger_state(&mut self, state: ShopMessengerState) {
        self.messenger_state = state;
        self.messenger_notice = None;
    }

    /// Notice posted by the last MESSENGER press.
    pub(crate) fn messenger_notice(&self) -> Option<ShopMessengerNotice> {
        self.messenger_notice
    }

    /// Stage the confirmation-pair result for the vacate ladder.
    pub(crate) fn stage_vacate_confirmed(&mut self, confirmed: bool) {
        self.vacate_confirmed = confirmed;
    }

    /// Stage the item code the vacate ladder branches on.
    pub(crate) fn stage_vacate_item_code(&mut self, code: u32) {
        self.vacate_item_code = code;
    }

    /// Stage the item word the type-2 vacate notice inks.
    pub(crate) fn stage_vacate_item_word(&mut self, word: u32) {
        self.vacate_item_word = word;
    }

    /// Selected owned row (global index across all bag pages).
    pub(crate) fn bag_selection(&self) -> Option<usize> {
        self.bag_selection
    }

    /// Notice posted by the last bag-cell press.
    pub(crate) fn bag_notice(&self) -> Option<ShopBagNotice> {
        self.bag_notice
    }

    /// Clear the exe's selected-row word (`DAT_0077ccc0 = -1`) and
    /// the notice that selection posted.
    fn clear_bag_selection(&mut self) {
        self.bag_selection = None;
        self.bag_notice = None;
    }

    /// Stage one owned item's category byte. The file→entry join is
    /// not pinned, so callers stage it explicitly.
    pub(crate) fn stage_bag_item_category(&mut self, item_id: u32, category: u8) {
        self.bag_item_categories.insert(item_id, category);
    }

    /// Stage whether one owned item occupies a worn slot.
    pub(crate) fn stage_bag_item_worn(&mut self, item_id: u32, worn: bool) {
        self.bag_item_worn.insert(item_id, worn);
    }

    /// Stage the special-item pair that refuses a selection.
    pub(crate) fn stage_bag_item_locked(&mut self, item_id: u32, locked: bool) {
        self.bag_item_locked.insert(item_id, locked);
    }

    /// Owned bag page (ten `MYBAGITEM` icons per page).
    pub(crate) fn bag_page(&self) -> usize {
        self.bag_page
    }

    /// Owned goods in inventory order, skipping zero and unknown
    /// ids exactly like the avatar-room bag join: one bad id never
    /// blanks the panel.
    pub(crate) fn bag_goods(&self, inventory: &[i32]) -> Vec<ShopGood> {
        inventory
            .iter()
            .filter_map(|id| {
                u32::try_from(*id)
                    .ok()
                    .filter(|id| *id != 0)
                    .and_then(|item_id| {
                        self.shop_goods
                            .iter()
                            .find(|good| good.item_id == item_id)
                            .cloned()
                    })
            })
            .collect()
    }

    /// Owned goods on the current bag page (owned page: small, ten
    /// cells at most).
    pub(crate) fn visible_bag(&self, inventory: &[i32]) -> Vec<ShopGood> {
        let owned = self.bag_goods(inventory);
        let (start, end) = lanking_page_window(owned.len(), SHOP_BAG_CELLS, self.bag_page);
        owned[start..end].to_vec()
    }

    /// Owned-panel page count (at least one, same empty-panel rule
    /// as the cart pager).
    pub(crate) fn bag_pages(&self, inventory: &[i32]) -> usize {
        shop_page_count(self.bag_goods(inventory).len(), SHOP_BAG_CELLS)
    }

    /// Sorted catalog goods on the current item-list page (owned
    /// page: small, eight rows at most). No sorts exist on the
    /// item board: catalog order always.
    pub(crate) fn visible_goods(&self) -> Vec<ShopGood> {
        let (start, end) =
            lanking_page_window(self.shop_goods.len(), SHOP_ITEM_ROWS, self.shop_page);
        self.shop_goods[start..end].to_vec()
    }

    /// Checked cart goods on the current cart page (toggle order,
    /// five rows at most; unknown ids drop out).
    pub(crate) fn visible_cart(&self) -> Vec<ShopGood> {
        let goods: Vec<ShopGood> = self
            .shop_cart
            .iter()
            .filter_map(|item_id| {
                self.shop_goods
                    .iter()
                    .find(|good| good.item_id == *item_id)
                    .cloned()
            })
            .collect();
        let (start, end) = lanking_page_window(goods.len(), SHOP_CART_ROWS, self.shop_cart_page);
        goods[start..end].to_vec()
    }

    /// Cart page count (at least one: the pager paints `1/1` on an
    /// empty cart, never `0/0`).
    pub(crate) fn cart_pages(&self) -> usize {
        shop_page_count(self.shop_cart.len(), SHOP_CART_ROWS)
    }

    /// Item-list page count (at least one, same empty-cart rule).
    pub(crate) fn list_pages(&self) -> usize {
        shop_page_count(self.shop_goods.len(), SHOP_ITEM_ROWS)
    }

    /// Row toggle through the control-code dispatch: checks the
    /// good into the cart, unchecks a held one. Never commits.
    pub(crate) fn toggle_row(&mut self, slot: usize) {
        if let Some(good) = self.visible_goods().get(slot).cloned() {
            if let Some(held) = self.shop_cart.iter().position(|id| *id == good.item_id) {
                self.shop_cart.remove(held);
            } else {
                self.shop_cart.push(good.item_id);
            }
        }
    }

    /// Board action through the control-code dispatch: row toggles
    /// check the cart, all three pagers step and hold at the ends,
    /// `ITEMPRE` picks preview the row and `ITEMBACK` clears it.
    /// Reports whether the action commits the cart (`BUYITEM` on a
    /// non-empty cart): the caller opens the local web purchase
    /// surface, so this board state never touches the wire.
    /// Preview and bag-pager arms report no commit.
    /// `owned` is the staged owned-good list the bag pager and the
    /// cell selector read (same empty-panel rule as the cart pager).
    pub(crate) fn apply_action(&mut self, action: ShopAction, owned: &[ShopGood]) -> bool {
        match action {
            ShopAction::ToggleRow(slot) => {
                self.toggle_row(slot);
                self.panel = ShopPanel::ShoppingBag;
                false
            }
            ShopAction::TryOn(slot) => {
                self.preview_slot(slot);
                false
            }
            ShopAction::RevertTryOn => {
                self.clear_try_on();
                false
            }
            ShopAction::ToggleExtMenu => {
                // Static arm `0x14200099` (`ev_f3ad15be…`): the `0x5f`
                // gate opens the press once, then `0x5b` flips and one
                // seed is taken; open clears `0x5d` and hides the row
                // groups, close restores them and hides the filters.
                if !self.ext_menu_ready {
                    return false;
                }
                self.ext_menu_ready = false;
                self.ext_menu_open = !self.ext_menu_open;
                self.ext_menu_toggles = self.ext_menu_toggles.saturating_add(1);
                self.ext_menu_reseeds = self.ext_menu_reseeds.saturating_add(1);
                if self.ext_menu_open {
                    self.ext_menu_secondary_word = 0;
                }
                false
            }
            ShopAction::VacateItem => {
                // Static arm `0x14200005` (`ev_f3ad15be…`): no arm at
                // all while the cart face is open (`0x8cd` set);
                // otherwise a confirmation pair, a special item code,
                // or the item word selects the notice.
                self.chrome_notice = if self.cart_face_open() {
                    None
                } else if self.vacate_confirmed {
                    Some(ShopChromeNotice::VacateConfirmed)
                } else if matches!(self.vacate_item_code, 0x23 | 0x24) {
                    Some(ShopChromeNotice::VacateSpecialItem)
                } else {
                    Some(ShopChromeNotice::VacateItem(self.vacate_item_word))
                };
                false
            }
            ShopAction::Conference => {
                // Static arm `0x142000b6`: type `0x14`, no text.
                self.chrome_notice = Some(ShopChromeNotice::Conference);
                false
            }
            ShopAction::OpenMessenger => {
                // Static arm `0x142000b8`: `FUN_004a3060(0)` advances
                // the two lamp rects (staging only), then the branch
                // on global `0x7684d8` posts one notice. No opcode.
                self.messenger_notice = Some(match self.messenger_state {
                    ShopMessengerState::Disconnected => ShopMessengerNotice::Disconnected,
                    ShopMessengerState::Loading => ShopMessengerNotice::Loading,
                    ShopMessengerState::Available => ShopMessengerNotice::Available,
                });
                false
            }
            ShopAction::SelectBagItem(slot) => {
                // Static arm `0x1460003d..46`: index = cell + page*10,
                // then sound 0x18 (not modeled here), the selection
                // word, and a category or worn-slot notice.
                let index = slot.saturating_add(self.bag_page.saturating_mul(SHOP_BAG_CELLS));
                let Some(good) = owned.get(index) else {
                    return false;
                };
                if self
                    .bag_item_locked
                    .get(&good.item_id)
                    .copied()
                    .unwrap_or(false)
                {
                    return false;
                }
                self.bag_selection = Some(index);
                self.bag_notice = if self
                    .bag_item_worn
                    .get(&good.item_id)
                    .copied()
                    .unwrap_or(false)
                {
                    Some(ShopBagNotice::SlotOccupied)
                } else {
                    self.bag_item_categories
                        .get(&good.item_id)
                        .copied()
                        .and_then(ShopBagNotice::for_category)
                };
                false
            }
            ShopAction::PlanetUp | ShopAction::PlanetDown => {
                // Static arms `0x14200008/09` (`ev_f3ad15be…`):
                // hide the 12×3 grid, step the index, ink the caption
                // when armed, rerun the three-way context refresh with
                // the index-12 word, then reseed twice and reset the
                // three layout flags.
                self.planet_hidden_cells = self.planet_grid_nonzero;
                self.planet_index = shop_next_planet_index(self.planet_index(), action);
                self.planet_caption_arg = self.planet_caption_armed.then_some(self.planet_index);
                self.lucky_view = self.planet_index == SHOP_PLANET_LAST_INDEX;
                self.last_planet_refresh = Some(self.refresh_context.resolve(self.lucky_view));
                self.planet_reseeds = self.planet_reseeds.saturating_add(2);
                self.planet_layout_flags = SHOP_PLANET_LAYOUT_FLAGS;
                false
            }
            ShopAction::SelectGender(gender) => {
                // Static arm `0x14202006/07` (`ev_f3ad15be…`): write
                // byte `+0x18`, hide the other button (derived from the
                // byte), and call the context refresher the object
                // state selects. The sound slot is not modeled here.
                self.gender = Some(gender);
                self.last_gender_refresh = Some(self.refresh_context.resolve(self.lucky_view));
                false
            }
            ShopAction::GiftRow(_) | ShopAction::GiftCart => false,
            ShopAction::OpenCashCharge | ShopAction::BackToLobby | ShopAction::GoMyRoom => false,
            ShopAction::SelectCategory(slot) => {
                self.shop_category = Some(slot);
                false
            }
            ShopAction::CartBuy => !self.shop_cart.is_empty(),
            ShopAction::ItemPrev => {
                self.shop_page = self.shop_page.saturating_sub(1);
                false
            }
            ShopAction::ItemNext => {
                if self
                    .shop_page
                    .saturating_add(1)
                    .saturating_mul(SHOP_ITEM_ROWS)
                    < self.shop_goods.len()
                {
                    self.shop_page = self.shop_page.saturating_add(1);
                }
                false
            }
            ShopAction::CartPrev => {
                // Exe `0x14200072`: `(page - 1 + count) % count`, only
                // while page and the cart flag are both non-zero. The
                // guard makes the modulo equal to `page - 1`.
                if self.cart_face_open() && self.shop_cart_page != 0 {
                    self.shop_cart_page = (self.shop_cart_page - 1) % self.cart_pages();
                }
                false
            }
            ShopAction::CartNext => {
                // Exe `0x14200073`: hold at the last page, never wrap.
                if self.cart_face_open()
                    && self
                        .shop_cart_page
                        .saturating_add(1)
                        .saturating_mul(SHOP_CART_ROWS)
                        < self.shop_cart.len()
                {
                    self.shop_cart_page = (self.shop_cart_page + 1) % self.cart_pages();
                }
                false
            }
            ShopAction::ShowMyBag => {
                self.panel = ShopPanel::MyBag;
                false
            }
            ShopAction::ShowShoppingBag => {
                self.panel = ShopPanel::ShoppingBag;
                false
            }
            ShopAction::BagPrev => {
                // Exe `0x1420004c`: `(page - 1 + count) % count`
                // wraps at the first page; both pager arms then clear
                // the selection word and refresh.
                let count = shop_page_count(owned.len(), SHOP_BAG_CELLS);
                if self.bag_page != 0 {
                    self.bag_page = (self.bag_page - 1) % count;
                } else if count > 1 {
                    self.bag_page = count - 1;
                }
                self.clear_bag_selection();
                false
            }
            ShopAction::BagNext => {
                // Exe `0x1420004d`: hold at the last page, never wrap.
                if self
                    .bag_page
                    .saturating_add(1)
                    .saturating_mul(SHOP_BAG_CELLS)
                    < owned.len()
                {
                    self.bag_page = self.bag_page.saturating_add(1);
                }
                self.clear_bag_selection();
                false
            }
            // Sorting changes no staged board state: the exe's sort
            // semantics stay OPEN and the wire send belongs to the
            // click caller, not to this pure board.
            ShopAction::SortBag => false,
        }
    }
}
