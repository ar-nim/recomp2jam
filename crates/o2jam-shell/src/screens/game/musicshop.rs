//! Song-shop songs, sorts, pages, and the fetched flag board
//! behind the music-shop side screen (#139 cluster 4).

use std::cmp::Ordering;
use std::collections::HashMap;

use o2jam_file::{InstalledMusic, OjnListEntry, OjnMidParamEntry};
use o2jam_net::play::{MusicListAnswer, MusicListEntry};

use super::super::side::{
    lanking_page_window, lanking_songs, music_shop_rows, shop_page_count, song_installed,
    sort_lanking_songs, LankingSong, LankingSort, MusicShopAction, MusicShopFace,
    MusicShopGenreFilter, MusicShopNotice, MusicShopPrice, MusicShopRow, OwnedSort,
    MSHOP_CART_ROWS, MSHOP_DOWN_CANCEL_CODE, MSHOP_MUSICDELETE_CODE, MSHOP_OWNED_FIRST_CODE,
    MSHOP_OWNED_ROWS, MSHOP_SEARCH_CODE, MSHOP_SONG_ROWS,
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
    /// A confirmed DOWN_CANCEL dropped the queued fetches.
    CancelDownloads,
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
    /// Last staged song-shop preview plate control code. The exe
    /// clears `0x1A200003`/`0x1A500002` on a page turn; our shell has
    /// no live producer for those art plates, so this records the
    /// staged code and only pins the clearing arm (#814).
    pub(crate) music_shop_preview_plate: Option<u32>,
    /// Active color filter. `NEW` is enabled at entry
    /// (`FUN_004c6d70`); a filter press resets the song page and
    /// stages the box-line frame at `filter index + 1` (#813).
    pub(crate) music_shop_genre_filter: MusicShopGenreFilter,
    /// Search-result flag (exe state `+0x88`). A filter press clears
    /// it; the SEARCH arm sets it on a successful match (#815).
    pub(crate) music_shop_search_active: bool,
    /// Text in the staged `0x1A30007B` edit. Typed characters go
    /// here while the music-shop side owns the keyboard (#815).
    pub(crate) music_shop_search_draft: String,
    /// Last committed query (exe state `+0x95`); an empty or equal
    /// draft posts the prompt instead of searching again.
    pub(crate) music_shop_search_query: Option<String>,
    /// Catalog song ids whose lowercased title contains the query.
    pub(crate) music_shop_search_hits: Vec<u32>,
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
    /// Visible owned-list page (twelve ids per page, exe state
    /// `+0x25`). The MYALL pager wraps left, clamps right, and is
    /// gated while a download stands (#817).
    pub(crate) music_shop_owned_page: usize,
    /// Active owned-list sort key and its per-key direction word
    /// (exe `state+0x370 + index*4`, #816).
    pub(crate) music_shop_owned_sort: Option<OwnedSort>,
    pub(crate) music_shop_owned_sort_descending: [bool; 3],
    /// Standing notice and the staged control code whose rect hosts
    /// it. The exe posts these strings through its notice helper;
    /// the host rect is ours-by-design chrome over the bnd layout.
    pub(crate) music_shop_notice: Option<(MusicShopNotice, u32)>,
    /// Last SELECTALL toggle (`None` until the first press). The exe
    /// flips a state word and walks both boards; which value is
    /// "all" is OPEN, so the shell records the observed toggle.
    pub(crate) music_shop_select_all: Option<bool>,
    /// A shop-initiated transfer stands. The exe reads its handle at
    /// `state+0x58` (`FUN_004f7a10`); the queue itself lives in the
    /// session's staged engine, so this only marks ownership.
    pub(crate) music_shop_download_active: bool,
    /// Songs completed by that transfer. The worker advances the
    /// visible index at `0x77a868+0x9c` once per finished song.
    pub(crate) music_shop_download_index: usize,
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
        self.music_shop_search_active = false;
        self.music_shop_search_hits.clear();
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
        self.music_shop_owned_page = 0;
        self.music_shop_owned_marks = [false; MSHOP_OWNED_ROWS];
        self.music_shop_owned_sort = None;
        self.music_shop_owned_sort_descending = [false; 3];
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

    /// Completed-song index of the shop transfer (0 before the first
    /// completion). Paired with the queue length by the caller.
    pub(crate) fn download_index(&self) -> usize {
        self.music_shop_download_index
    }

    /// Whether a shop-initiated transfer owns the download panel.
    pub(crate) fn is_download_active(&self) -> bool {
        self.music_shop_download_active
    }

    /// Advance the visible row readout once per finished transfer.
    pub(crate) fn note_download_completion(&mut self, count: usize) {
        if self.music_shop_download_active {
            self.music_shop_download_index = self.music_shop_download_index.saturating_add(count);
        }
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

    /// Owned-list page count (at least one, like the song pager).
    pub(crate) fn owned_pages(&self) -> usize {
        self.music_shop_board.as_ref().map_or(1, |board| {
            shop_page_count(board.entries.len(), MSHOP_OWNED_ROWS)
        })
    }

    /// Visible owned-list page index.
    pub(crate) fn owned_page(&self) -> usize {
        self.music_shop_owned_page
    }

    /// Active owned-list sort key.
    pub(crate) fn owned_sort(&self) -> Option<OwnedSort> {
        self.music_shop_owned_sort
    }

    /// Direction of the active owned-list sort key.
    pub(crate) fn owned_sort_descending(&self) -> bool {
        self.music_shop_owned_sort
            .is_some_and(|sort| self.music_shop_owned_sort_descending[sort.index()])
    }

    /// Catalog title of one owned song id, or the bare id.
    fn owned_title(&self, song_id: u16) -> String {
        self.music_shop_songs
            .iter()
            .find(|song| song.song_id == u32::from(song_id))
            .map_or_else(|| song_id.to_string(), |song| song.title.clone())
    }

    /// Exe-backed compare for one owned header. Play time and
    /// download date have no staged value source, so their local
    /// fallback keys are id and installed bit respectively.
    fn compare_owned_entries(
        &self,
        sort: OwnedSort,
        first: &MusicListEntry,
        second: &MusicListEntry,
    ) -> Ordering {
        let order = match sort {
            OwnedSort::Title => self
                .owned_title(first.id)
                .to_lowercase()
                .cmp(&self.owned_title(second.id).to_lowercase())
                .then_with(|| first.id.cmp(&second.id)),
            OwnedSort::PlayTime => first.id.cmp(&second.id),
            OwnedSort::DownloadStatus => {
                song_installed(u32::from(first.id), self.music_shop_installed.as_ref())
                    .cmp(&song_installed(
                        u32::from(second.id),
                        self.music_shop_installed.as_ref(),
                    ))
                    .then_with(|| first.id.cmp(&second.id))
            }
        };
        if self.music_shop_owned_sort_descending[sort.index()] {
            order.reverse()
        } else {
            order
        }
    }

    /// Owned rows from the fetched 4031 ids joined to the catalog
    /// titles and the local chart scan, twelve at most on this page.
    pub(crate) fn owned_rows(&self) -> Vec<OwnedRow> {
        let Some(board) = self.music_shop_board.as_ref() else {
            return Vec::new();
        };
        let mut entries = board.entries.clone();
        if let Some(sort) = self.music_shop_owned_sort {
            entries.sort_by(|first, second| self.compare_owned_entries(sort, first, second));
        }
        let (start, end) =
            lanking_page_window(entries.len(), MSHOP_OWNED_ROWS, self.music_shop_owned_page);
        entries[start..end]
            .iter()
            .enumerate()
            .map(|(slot, entry)| OwnedRow {
                song_id: entry.id,
                title: self.owned_title(entry.id),
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

    /// Active color filter.
    pub(crate) fn genre_filter(&self) -> MusicShopGenreFilter {
        self.music_shop_genre_filter
    }

    /// Box-line frame staged by the active filter (one-based).
    pub(crate) fn frame_index(&self) -> u32 {
        self.music_shop_genre_filter.frame_index()
    }

    /// Whether the visible songs are a search result.
    pub(crate) fn search_active(&self) -> bool {
        self.music_shop_search_active
    }

    /// Current edit text.
    pub(crate) fn search_draft(&self) -> &str {
        &self.music_shop_search_draft
    }

    /// Marked title hits from the last successful search.
    pub(crate) fn search_hits(&self) -> &[u32] {
        &self.music_shop_search_hits
    }

    /// Type one non-control character into the search edit.
    pub(crate) fn push_search_char(&mut self, typed: char) {
        if !typed.is_control() {
            self.music_shop_search_draft.push(typed);
        }
    }

    /// Erase one character from the search edit.
    pub(crate) fn delete_search_char(&mut self) {
        self.music_shop_search_draft.pop();
    }

    /// Stage the search flag at the board seam. The SEARCH arm owns
    /// it once that arm lands (#815).
    pub(crate) fn stage_search_active(&mut self, active: bool) {
        self.music_shop_search_active = active;
    }

    /// Last staged preview plate code, if any.
    pub(crate) fn preview_plate(&self) -> Option<u32> {
        self.music_shop_preview_plate
    }

    /// Stage a preview plate at the board seam. No live click arm
    /// stages one yet; the pinned behavior is the page-turn clear.
    pub(crate) fn stage_preview_plate(&mut self, code: u32) {
        self.music_shop_preview_plate = Some(code);
    }

    /// Drop the art a page turn makes stale: preview plate, picked
    /// row, and pressed rank key (exe `MSALL` arm, #814).
    fn clear_page_plates(&mut self) {
        self.music_shop_preview_plate = None;
        self.music_shop_selected_row = None;
        self.music_shop_top100_pressed = None;
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
        let mut songs: Vec<LankingSong> = if self.music_shop_search_active {
            self.music_shop_songs
                .iter()
                .filter(|song| self.music_shop_search_hits.contains(&song.song_id))
                .cloned()
                .collect()
        } else {
            self.music_shop_songs.clone()
        };
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
            MusicShopAction::DownCancel => {
                let confirmation = Some((MusicShopNotice::CancelDownload, MSHOP_DOWN_CANCEL_CODE));
                if armed_notice == confirmation {
                    self.music_shop_download_active = false;
                    MusicShopEffect::CancelDownloads
                } else {
                    self.post_notice(MusicShopNotice::CancelDownload, MSHOP_DOWN_CANCEL_CODE);
                    MusicShopEffect::None
                }
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
                    self.music_shop_download_active = true;
                    self.music_shop_download_index = 0;
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
            MusicShopAction::Search => {
                let draft = self.music_shop_search_draft.clone();
                if draft.is_empty()
                    || self.music_shop_search_query.as_deref() == Some(draft.as_str())
                {
                    self.post_notice(MusicShopNotice::EmptySearch, MSHOP_SEARCH_CODE);
                    return MusicShopEffect::None;
                }
                let needle = draft.to_lowercase();
                let hits: Vec<u32> = self
                    .music_shop_songs
                    .iter()
                    .filter(|song| song.title.to_lowercase().contains(&needle))
                    .map(|song| song.song_id)
                    .collect();
                if hits.is_empty() {
                    self.post_notice(MusicShopNotice::NoSearchHits, MSHOP_SEARCH_CODE);
                    return MusicShopEffect::None;
                }
                self.music_shop_search_query = Some(draft);
                self.music_shop_search_hits = hits;
                self.music_shop_search_active = true;
                self.music_shop_page = 0;
                self.clear_page_plates();
                MusicShopEffect::None
            }
            MusicShopAction::OwnedSort(sort) => {
                let direction = sort.index();
                if self.music_shop_owned_sort == Some(sort) {
                    self.music_shop_owned_sort_descending[direction] =
                        !self.music_shop_owned_sort_descending[direction];
                } else {
                    self.music_shop_owned_sort = Some(sort);
                }
                self.music_shop_owned_page = 0;
                self.music_shop_owned_marks = [false; MSHOP_OWNED_ROWS];
                MusicShopEffect::None
            }
            MusicShopAction::Genre(genre) => {
                self.music_shop_genre_filter = genre;
                self.music_shop_search_active = false;
                self.music_shop_page = 0;
                self.clear_page_plates();
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
                let pages = self.list_pages();
                self.music_shop_page = self
                    .music_shop_page
                    .checked_sub(1)
                    .unwrap_or_else(|| pages.saturating_sub(1));
                self.clear_page_plates();
                MusicShopEffect::None
            }
            MusicShopAction::MsNext => {
                let next = self.music_shop_page.saturating_add(1);
                if next < self.list_pages() {
                    self.music_shop_page = next;
                    self.clear_page_plates();
                }
                MusicShopEffect::None
            }
            MusicShopAction::MyPrev => {
                if !self.music_shop_download_active {
                    let pages = self.owned_pages();
                    self.music_shop_owned_page = self
                        .music_shop_owned_page
                        .checked_sub(1)
                        .unwrap_or_else(|| pages.saturating_sub(1));
                    self.music_shop_owned_marks = [false; MSHOP_OWNED_ROWS];
                }
                MusicShopEffect::None
            }
            MusicShopAction::MyNext => {
                if !self.music_shop_download_active {
                    let next = self.music_shop_owned_page.saturating_add(1);
                    if next < self.owned_pages() {
                        self.music_shop_owned_page = next;
                        self.music_shop_owned_marks = [false; MSHOP_OWNED_ROWS];
                    }
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
