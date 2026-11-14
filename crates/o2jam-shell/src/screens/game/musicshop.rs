//! Song-shop songs, sorts, pages, and the fetched flag board
//! behind the music-shop side screen (#139 cluster 4).

use o2jam_file::OjnListEntry;
use o2jam_net::play::{MusicListAnswer, MusicListEntry};

use super::super::side::{
    lanking_page_window, lanking_songs, music_shop_rows, shop_page_count, sort_lanking_songs,
    LankingSong, LankingSort, MusicShopAction, MusicShopRow, MSHOP_CART_ROWS, MSHOP_SONG_ROWS,
};

/// Song-shop songs, sorts, pages, and the fetched flag board behind
/// the music-shop side screen (#139 cluster 4: the Data Clump of all
/// six `music_shop_*` fields). Pure board state — no side dispatch,
/// paint, or wire queue — so the board reads without touching the
/// `ScreenGame` God class. `ScreenGame` keeps thin delegates under
/// the same names, so the click seam and the snapshot surface never
/// change. Row and cart commits still queue on the outbox (cluster
/// 1): `apply_action` only reports them, the caller queues the
/// pinned 5010 send.
#[derive(Default)]
pub(crate) struct MusicShopState {
    /// Song-shop catalog (`OJNList.dat` order, owned copies; see
    /// [`super::ScreenGame::set_music_shop_catalog`]).
    pub(crate) music_shop_songs: Vec<LankingSong>,
    /// Active song-shop sort (`None`: catalog order until a sort
    /// button answers; directions ASSUMED ascending).
    pub(crate) music_shop_sort: Option<LankingSort>,
    /// Song-list page (five rows per page, the staged `MS_EX` hit
    /// areas).
    pub(crate) music_shop_page: usize,
    /// Fetched song flags (`GetMusicList` 4031 answer) staged
    /// behind the scripted-stub seam: the shell proves the rows,
    /// the wire ride stays ASSUMED until a live gateway replay.
    pub(crate) music_shop_board: Option<MusicListAnswer>,
    /// Checked song-shop cart ids in pick order (the `MS_EX` row
    /// selection behind the cart `BUYITEM` commit).
    pub(crate) music_shop_cart: Vec<u32>,
    /// Cart-list page (five rows per page, the staged music-cart
    /// `CART_EX` slots).
    pub(crate) music_shop_cart_page: usize,
}

impl MusicShopState {
    /// Stage the song-shop catalog (`OJNList.dat` order, owned
    /// copies so the board outlives the parse borrow). Resets sort
    /// and pages.
    pub(crate) fn set_catalog(&mut self, entries: &[OjnListEntry<'_>]) {
        self.music_shop_songs = lanking_songs(entries);
        self.music_shop_sort = None;
        self.music_shop_page = 0;
        self.music_shop_cart_page = 0;
    }

    /// Stage the fetched song flags (`GetMusicList` 4031 answer).
    /// Resets the song-list page.
    pub(crate) fn stage_board(&mut self, answer: MusicListAnswer) {
        self.music_shop_board = Some(answer);
        self.music_shop_page = 0;
    }

    /// Fold the 5011 acquired-song ids into the fetched-flags board:
    /// the reply carries the server-authoritative owned list (unlike
    /// the id-less 5001 item reply), so each id joins the 4031
    /// entries and its row paints OWNED. Without a staged 4031 board
    /// the ids seed one with the free flag cleared — no free flag is
    /// invented. Merged rows carry zeroed note counts: nothing reads
    /// them, only ids feed the owned check.
    pub(crate) fn fold_purchased_songs(&mut self, song_ids: &[u16]) {
        let board = self.music_shop_board.get_or_insert(MusicListAnswer {
            entries: Vec::new(),
            free_music: false,
        });
        for song_id in song_ids {
            if !board.entries.iter().any(|entry| entry.id == *song_id) {
                board.entries.push(MusicListEntry {
                    id: *song_id,
                    notes_ex: 0,
                    notes_nx: 0,
                    notes_hx: 0,
                });
            }
        }
    }

    /// Active song-shop sort (`None`: catalog order).
    pub(crate) fn sort(&self) -> Option<LankingSort> {
        self.music_shop_sort
    }

    /// Song-list page (five rows per page).
    pub(crate) fn page(&self) -> usize {
        self.music_shop_page
    }

    /// Cart-list page (five rows per page).
    pub(crate) fn cart_page(&self) -> usize {
        self.music_shop_cart_page
    }

    /// Checked song-shop cart ids in pick order.
    pub(crate) fn cart(&self) -> Vec<u32> {
        self.music_shop_cart.clone()
    }

    /// Pick one catalog song into the cart (id must exist; the
    /// click seam feeds the same cart — this stages restores).
    pub(crate) fn cart_push(&mut self, song_id: u32) {
        if self
            .music_shop_songs
            .iter()
            .any(|song| song.song_id == song_id)
            && !self.music_shop_cart.contains(&song_id)
        {
            self.music_shop_cart.push(song_id);
        }
    }

    /// Checked music-cart songs on the current cart page (pick
    /// order, five rows at most; unknown ids drop out).
    pub(crate) fn visible_cart(&self) -> Vec<LankingSong> {
        let songs: Vec<LankingSong> = self
            .music_shop_cart
            .iter()
            .filter_map(|song_id| {
                self.music_shop_songs
                    .iter()
                    .find(|song| song.song_id == *song_id)
                    .cloned()
            })
            .collect();
        let (start, end) =
            lanking_page_window(songs.len(), MSHOP_CART_ROWS, self.music_shop_cart_page);
        songs[start..end].to_vec()
    }

    /// Music-cart page count (at least one, same empty-cart rule).
    pub(crate) fn cart_pages(&self) -> usize {
        shop_page_count(self.music_shop_cart.len(), MSHOP_CART_ROWS)
    }

    /// Song-list page count (at least one, same empty-cart rule).
    pub(crate) fn list_pages(&self) -> usize {
        shop_page_count(self.music_shop_songs.len(), MSHOP_SONG_ROWS)
    }

    /// Sorted song-shop rows with live flags on the current page
    /// (owned page: small, five rows at most).
    pub(crate) fn visible_rows(&self) -> Vec<MusicShopRow> {
        let mut songs = self.music_shop_songs.clone();
        if let Some(sort) = self.music_shop_sort {
            sort_lanking_songs(&mut songs, sort);
        }
        let (start, end) = lanking_page_window(songs.len(), MSHOP_SONG_ROWS, self.music_shop_page);
        music_shop_rows(&songs[start..end], self.music_shop_board.as_ref())
    }

    /// Board action through the control-code dispatch: row picks
    /// stage the cart, sorts resort and reset the page, and the
    /// pagers step and hold at the ends. Reports whether the action
    /// queues the pinned 5010 send (a row buy on a visible row, or
    /// the cart `BUYITEM`): the caller queues it on the outbox, so
    /// this board state never touches the wire.
    pub(crate) fn apply_action(&mut self, action: MusicShopAction) -> bool {
        match action {
            MusicShopAction::SelectRow(slot) => {
                if let Some(row) = self.visible_rows().get(slot) {
                    let song_id = row.song.song_id;
                    if !self.music_shop_cart.contains(&song_id) {
                        self.music_shop_cart.push(song_id);
                    }
                }
                false
            }
            MusicShopAction::BuyRow { slot, gem: _ } => self.visible_rows().get(slot).is_some(),
            MusicShopAction::Sort(sort) => {
                self.music_shop_sort = Some(sort);
                self.music_shop_page = 0;
                false
            }
            MusicShopAction::MsPrev => {
                self.music_shop_page = self.music_shop_page.saturating_sub(1);
                false
            }
            MusicShopAction::MsNext => {
                if self
                    .music_shop_page
                    .saturating_add(1)
                    .saturating_mul(MSHOP_SONG_ROWS)
                    < self.music_shop_songs.len()
                {
                    self.music_shop_page = self.music_shop_page.saturating_add(1);
                }
                false
            }
            MusicShopAction::CartBuy => true,
            MusicShopAction::CartPrev => {
                self.music_shop_cart_page = self.music_shop_cart_page.saturating_sub(1);
                false
            }
            MusicShopAction::CartNext => {
                if self
                    .music_shop_cart_page
                    .saturating_add(1)
                    .saturating_mul(MSHOP_CART_ROWS)
                    < self.music_shop_cart.len()
                {
                    self.music_shop_cart_page = self.music_shop_cart_page.saturating_add(1);
                }
                false
            }
        }
    }
}
