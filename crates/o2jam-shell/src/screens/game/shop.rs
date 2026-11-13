//! Item-shop goods, pages, and the checked cart behind the shop
//! side screen (#139 cluster 3).

use super::super::side::{
    lanking_page_window, shop_goods, shop_page_count, ShopAction, ShopGood, SHOP_BAG_CELLS,
    SHOP_CART_ROWS, SHOP_ITEM_ROWS,
};

/// Item-shop goods, pages, and the checked cart behind the shop
/// side screen (#139 cluster 3: the Data Clump of all four
/// `shop_*` fields). Pure board state — no side dispatch, paint,
/// or wire queue — so the board reads without touching the
/// `ScreenGame` God class. `ScreenGame` keeps thin delegates under
/// the same names, so the click seam and the snapshot surface never
/// change. The cart commit still queues on the outbox (cluster 1):
/// `apply_action` only reports it, the caller queues the send.
#[derive(Default)]
pub(crate) struct ShopState {
    /// Item-shop goods (`Itemdata_Korea.dat` order, owned copies;
    /// see [`Self::set_shop_catalog`]).
    pub(crate) shop_goods: Vec<ShopGood>,
    /// Item-list page (eight rows per page, the staged `ITEMBUY`
    /// hit areas).
    pub(crate) shop_page: usize,
    /// Checked cart item ids in toggle order (the `ITEMBUY`
    /// selection behind the cart `BUYITEM` commit).
    pub(crate) shop_cart: Vec<u32>,
    /// Cart-list page (five rows per page, the staged `CART_EX`
    /// slots).
    pub(crate) shop_cart_page: usize,
    /// Previewed item id (`ITEMPRE` pick, `ITEMBACK` clears; `None`
    /// until picked). Ids stay stable across pages (unlike slots).
    pub(crate) preview: Option<u32>,
    /// Owned-panel page (ten `MYBAGITEM` icons per page, the staged
    /// `BAGLEFT`/`BAGRIGHT` pager).
    pub(crate) bag_page: usize,
}

impl ShopState {
    /// Stage the item-shop catalog (`Itemdata_Korea.dat` order,
    /// owned copies so the board outlives the parse borrow). Resets
    /// the item page, the preview pick, and the owned-panel page;
    /// the cart keeps its checks.
    pub(crate) fn set_catalog(&mut self, table: &o2jam_file::AvatarItemTable) {
        self.shop_goods = shop_goods(table);
        self.shop_page = 0;
        self.preview = None;
        self.bag_page = 0;
    }

    /// Item-list page (eight rows per page).
    pub(crate) fn page(&self) -> usize {
        self.shop_page
    }

    /// Cart-list page (five rows per page).
    pub(crate) fn cart_page(&self) -> usize {
        self.shop_cart_page
    }

    /// Checked cart item ids in toggle order.
    pub(crate) fn cart(&self) -> Vec<u32> {
        self.shop_cart.clone()
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
    /// non-empty cart): the caller queues the pinned 5000 send on
    /// the outbox, so this board state never touches the wire.
    /// Preview and bag-pager arms report no send (browse tools only).
    /// `owned_len` is the staged owned-good count the bag pager
    /// holds against (same empty-panel rule as the cart pager).
    pub(crate) fn apply_action(&mut self, action: ShopAction, owned_len: usize) -> bool {
        match action {
            ShopAction::ToggleRow(slot) => {
                self.toggle_row(slot);
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
                self.shop_cart_page = self.shop_cart_page.saturating_sub(1);
                false
            }
            ShopAction::CartNext => {
                if self
                    .shop_cart_page
                    .saturating_add(1)
                    .saturating_mul(SHOP_CART_ROWS)
                    < self.shop_cart.len()
                {
                    self.shop_cart_page = self.shop_cart_page.saturating_add(1);
                }
                false
            }
            ShopAction::PreviewRow(slot) => {
                self.select_preview(slot);
                false
            }
            ShopAction::PreviewBack => {
                self.clear_preview();
                false
            }
            ShopAction::BagPrev => {
                self.bag_page = self.bag_page.saturating_sub(1);
                false
            }
            ShopAction::BagNext => {
                if self
                    .bag_page
                    .saturating_add(1)
                    .saturating_mul(SHOP_BAG_CELLS)
                    < owned_len
                {
                    self.bag_page = self.bag_page.saturating_add(1);
                }
                false
            }
        }
    }
}
