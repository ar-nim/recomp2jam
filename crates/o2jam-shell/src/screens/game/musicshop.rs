//! Song-shop songs, sorts, pages, and the fetched flag board
//! behind the music-shop side screen (#139 cluster 4).

use std::collections::HashMap;

use o2jam_file::{InstalledMusic, OjnListEntry, OjnMidParamEntry};
use o2jam_net::play::{MusicListAnswer, MusicListEntry};

use super::super::side::{
    lanking_page_window, lanking_songs, music_shop_rows, shop_page_count, song_installed,
    sort_lanking_songs, LankingSong, LankingSort, MusicShopAction, MusicShopFace, MusicShopNotice,
    MusicShopPrice, MusicShopRow, MSHOP_CART_ROWS, MSHOP_MUSICDELETE_CODE, MSHOP_OWNED_FIRST_CODE,
    MSHOP_OWNED_ROWS, MSHOP_SONG_ROWS,
};

/// One result of a music-shop control press. The board owns the
/// state change; the caller owns the side effects (the 5010 outbox
/// and the staged download queue).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MusicShopEffect {
    /// No caller-side effect.
    None,
    /// Queue the pinned 5010 music purchase.
    Buy,
    /// Hand these owned song ids to the staged download engine.
    StartDownloads(Vec<u16>),
}

/// One owned-list row: a 4031 song id, its catalog title, the local
/// chart bit, and this page's check mark. The two per-row properties
/// are pinned; their exact sprite faces stay OPEN (no mixed-state
/// capture yet), so the shell names them by control role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedRow {
    /// Song id from the fetched 4031 list.
    pub song_id: u16,
    /// Catalog title, or the bare id when no catalog row joined.
    pub title: String,
    /// Local `o2ma<id>.ojn` chart present (bit `0x8`, §1).
    pub installed: bool,
    /// This row's check mark on the visible owned page.
    pub marked: bool,
}

/// Exe's five-song selection cap (`FUN_004cdca0`, VA `0x64a7d8`).
const MAX_SELECTED_SONGS: usize = 5;

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
    /// Last picked `MS_EX` row on the visible page. The exe press
    /// frame is unpinned (`E-T`); this is client-only chrome over the
    /// staged row rect, never a wire or ownership claim (#526).
    pub(crate) music_shop_selected_row: Option<usize>,
    /// Checked song-shop cart ids in pick order (the `MS_EX` row
    /// selection behind the cart `BUYITEM` commit).
    pub(crate) music_shop_cart: Vec<u32>,
    /// Cart-list page (five rows per page, the staged music-cart
    /// `CART_EX` slots).
    pub(crate) music_shop_cart_page: usize,
    /// Per-song gem/won prices from `OJNList.dat` table 3
    /// (`param_b`/`param_a`); the boot stages them once with the
    /// catalog, and a table without a song leaves its cells blank.
    pub(crate) music_shop_prices: HashMap<u32, MusicShopPrice>,
    /// Last pressed TOP100 board-chrome control code (`None` until a
    /// press). The qualified exe frame shows no ranking board, so
    /// this marks a client-only press inside the staged rect — never
    /// a standing, a sort, or a wire request (#408).
    pub(crate) music_shop_top100_pressed: Option<u32>,
    /// Owned face selected by the two tab images (`0x1A600076/77`).
    /// The exe's two flag writes are pinned; the meaning of the flag
    /// beyond the tab names stays OPEN (no visibility frame captured).
    pub(crate) music_shop_face: MusicShopFace,
    /// Local `Music\` scan (§1); `None` means no local knowledge, so
    /// no installed mark is invented from the 4031 flag list.
    pub(crate) music_shop_installed: Option<InstalledMusic>,
    /// Check marks of the twelve owned rows on this page. The exe
    /// keeps a twelve-column per-page mark table; presses toggle one
    /// cell (`FUN_004cdca0`).
    pub(crate) music_shop_owned_marks: [bool; MSHOP_OWNED_ROWS],
    /// Standing notice and the staged control code whose rect hosts
    /// it. The exe posts these strings through its notice helper;
    /// the host rect is ours-by-design chrome over the bnd layout.
    pub(crate) music_shop_notice: Option<(MusicShopNotice, u32)>,
    /// Last SELECTALL toggle (`None` until the first press). The exe
    /// flips a state word and walks both boards; which value is
    /// "all" is OPEN, so the shell records the observed toggle.
    pub(crate) music_shop_select_all: Option<bool>,
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
        self.music_shop_selected_row = None;
        self.music_shop_top100_pressed = None;
    }

    /// Stage the local `Music\` scan (§1) behind the shop. The
    /// server's 4031 flags never write this bit: only a chart file
    /// the client can play does.
    pub(crate) fn set_installed(&mut self, installed: Option<&InstalledMusic>) {
        self.music_shop_installed = installed.cloned();
    }

    /// Stage the fetched song flags (`GetMusicList` 4031 answer).
    /// Resets the song-list page.
    pub(crate) fn stage_board(&mut self, answer: MusicListAnswer) {
        self.music_shop_board = Some(answer);
        self.music_shop_page = 0;
    }

    /// Stage the per-song price words from `OJNList.dat` table 3.
    /// `param_b` is the gem price and `param_a` the won price;
    /// songs the table omits have no price cells.
    pub(crate) fn stage_prices(&mut self, params: &[OjnMidParamEntry]) {
        self.music_shop_prices = params
            .iter()
            .map(|param| {
                (
                    param.song_id,
                    MusicShopPrice {
                        gem: param.param_b,
                        won: param.param_a,
                    },
                )
            })
            .collect();
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

    /// Last picked row on the visible page, if any.
    pub(crate) fn selected_row(&self) -> Option<usize> {
        self.music_shop_selected_row
    }

    /// Last SELECTALL toggle, if the control was pressed.
    pub(crate) fn select_all(&self) -> Option<bool> {
        self.music_shop_select_all
    }

    /// Standing shop notice, if any, with its staging code.
    pub(crate) fn notice(&self) -> Option<(MusicShopNotice, u32)> {
        self.music_shop_notice
    }

    /// Post one notice against a staged control rect. A later notice
    /// replaces the standing one; the next board press dismisses it.
    pub(crate) fn post_notice(&mut self, notice: MusicShopNotice, code: u32) {
        self.music_shop_notice = Some((notice, code));
    }

    /// Owned rows from the fetched 4031 ids joined to the catalog
    /// titles and the local chart scan, twelve at most.
    pub(crate) fn owned_rows(&self) -> Vec<OwnedRow> {
        let Some(board) = self.music_shop_board.as_ref() else {
            return Vec::new();
        };
        board
            .entries
            .iter()
            .take(MSHOP_OWNED_ROWS)
            .enumerate()
            .map(|(slot, entry)| OwnedRow {
                song_id: entry.id,
                title: self
                    .music_shop_songs
                    .iter()
                    .find(|song| song.song_id == u32::from(entry.id))
                    .map_or_else(|| entry.id.to_string(), |song| song.title.clone()),
                installed: song_installed(u32::from(entry.id), self.music_shop_installed.as_ref()),
                marked: self.music_shop_owned_marks[slot],
            })
            .collect()
    }

    /// Owned face selected by the two tab images.
    pub(crate) fn face(&self) -> MusicShopFace {
        self.music_shop_face
    }

    /// Last pressed TOP100 board-chrome control code, if any.
    pub(crate) fn top100_pressed(&self) -> Option<u32> {
        self.music_shop_top100_pressed
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
        music_shop_rows(
            &songs[start..end],
            self.music_shop_board.as_ref(),
            &self.music_shop_prices,
            self.music_shop_installed.as_ref(),
        )
    }

    /// Board action through the control-code dispatch: row picks
    /// stage the cart, sorts resort and reset the page, and the
    /// pagers step and wrap. Reports the caller-side effect — a
    /// pinned 5010 buy or the marked song ids for the staged worker
    /// — so this board state never touches the wire or the queue.
    pub(crate) fn apply_action(&mut self, action: MusicShopAction) -> MusicShopEffect {
        // A notice owns only the next board press: any control that
        // follows dismisses it, and the arm may post a fresh one or
        // consume an armed confirmation.
        let armed_notice = self.music_shop_notice;
        self.music_shop_notice = None;
        match action {
            MusicShopAction::SelectRow(slot) => {
                self.music_shop_selected_row = self.visible_rows().get(slot).map(|_| slot);
                if let Some(row) = self.visible_rows().get(slot) {
                    let song_id = row.song.song_id;
                    if !self.music_shop_cart.contains(&song_id) {
                        self.music_shop_cart.push(song_id);
                    }
                }
                MusicShopEffect::None
            }
            MusicShopAction::BuyRow { slot, gem: _ } => {
                if self.visible_rows().get(slot).is_some() {
                    MusicShopEffect::Buy
                } else {
                    MusicShopEffect::None
                }
            }
            MusicShopAction::Top100Chrome { code } => {
                self.music_shop_top100_pressed = Some(code);
                MusicShopEffect::None
            }
            MusicShopAction::Face(face) => {
                self.music_shop_face = face;
                MusicShopEffect::None
            }
            MusicShopAction::DownStart => {
                let song_ids: Vec<u16> = self
                    .owned_rows()
                    .iter()
                    .filter(|row| row.marked)
                    .map(|row| row.song_id)
                    .collect();
                if song_ids.is_empty() {
                    MusicShopEffect::None
                } else {
                    MusicShopEffect::StartDownloads(song_ids)
                }
            }
            MusicShopAction::MusicDelete => {
                let confirmation = Some((MusicShopNotice::DeleteFiles, MSHOP_MUSICDELETE_CODE));
                if armed_notice == confirmation {
                    // No file removal is claimed: the confirmed arm
                    // clears the marked rows, and the delete write
                    // stays OPEN until a synthetic-song capture (#824).
                    self.music_shop_owned_marks = [false; MSHOP_OWNED_ROWS];
                } else if self.owned_rows().iter().any(|row| row.marked) {
                    self.post_notice(MusicShopNotice::DeleteFiles, MSHOP_MUSICDELETE_CODE);
                }
                MusicShopEffect::None
            }
            MusicShopAction::SelectAll => {
                let select_all = !self.music_shop_select_all.unwrap_or(false);
                self.music_shop_select_all = Some(select_all);
                self.music_shop_owned_marks = [select_all; MSHOP_OWNED_ROWS];
                MusicShopEffect::None
            }
            MusicShopAction::OwnedRow(slot) => {
                if slot < self.music_shop_owned_marks.len() && self.owned_rows().get(slot).is_some()
                {
                    let already_marked = self.music_shop_owned_marks[slot];
                    let mark_count = self
                        .music_shop_owned_marks
                        .iter()
                        .filter(|cell| **cell)
                        .count();
                    if !already_marked && mark_count >= MAX_SELECTED_SONGS {
                        self.post_notice(
                            MusicShopNotice::FiveSongCap,
                            MSHOP_OWNED_FIRST_CODE + u32::try_from(slot).unwrap_or(0),
                        );
                    } else {
                        self.music_shop_owned_marks[slot] = !already_marked;
                    }
                }
                MusicShopEffect::None
            }
            MusicShopAction::Sort(sort) => {
                self.music_shop_sort = Some(sort);
                self.music_shop_page = 0;
                self.music_shop_selected_row = None;
                self.music_shop_top100_pressed = None;
                MusicShopEffect::None
            }
            MusicShopAction::MsPrev => {
                self.music_shop_page = self.music_shop_page.saturating_sub(1);
                self.music_shop_selected_row = None;
                self.music_shop_top100_pressed = None;
                MusicShopEffect::None
            }
            MusicShopAction::MsNext => {
                if self
                    .music_shop_page
                    .saturating_add(1)
                    .saturating_mul(MSHOP_SONG_ROWS)
                    < self.music_shop_songs.len()
                {
                    self.music_shop_page = self.music_shop_page.saturating_add(1);
                    self.music_shop_selected_row = None;
                    self.music_shop_top100_pressed = None;
                }
                MusicShopEffect::None
            }
            MusicShopAction::CartBuy => MusicShopEffect::Buy,
            MusicShopAction::CartRemove(slot) => {
                if let Some(song) = self.visible_cart().get(slot) {
                    let song_id = song.song_id;
                    self.music_shop_cart.retain(|id| *id != song_id);
                    let last_page = self.cart_pages().saturating_sub(1);
                    if self.music_shop_cart_page > last_page {
                        self.music_shop_cart_page = last_page;
                    }
                }
                MusicShopEffect::None
            }
            MusicShopAction::CartPrev => {
                // Exe state +0x13B wraps modulo +0x13C at the first
                // page; the empty cart still parks the index at zero.
                let pages = self.cart_pages();
                if pages > 0 {
                    self.music_shop_cart_page = (self.music_shop_cart_page + pages - 1) % pages;
                }
                MusicShopEffect::None
            }
            MusicShopAction::CartNext => {
                if self.music_shop_cart_page.saturating_add(1) < self.cart_pages() {
                    self.music_shop_cart_page = self.music_shop_cart_page.saturating_add(1);
                }
                MusicShopEffect::None
            }
            // BACK is a side leave: `click_music_shop` owns the
            // transition, so the board state never changes here.
            MusicShopAction::BackToLobby => MusicShopEffect::None,
        }
    }
}
