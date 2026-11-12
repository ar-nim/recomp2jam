//! Left-list songs, sorts, pages, and the fetched rank board
//! behind the lanking side screen (#139 cluster 2).

use o2jam_file::OjnListEntry;
use o2jam_net::rooms::{MusicPlayRankingAnswer, RankEntry};

use super::super::side::{
    lanking_page_window, lanking_songs, sort_lanking_songs, LankingAction, LankingSong,
    LankingSort, LANKING_LVIEW_ROWS, LANKING_RVIEW_ROWS,
};

/// Left-list songs, sorts, pages, and the fetched rank board behind
/// the lanking side screen (#139 cluster 2: the Data Clump of all
/// six `lanking_*` fields). Pure board state — no side dispatch,
/// paint, or input — so the board reads without touching the
/// `ScreenGame` God class. `ScreenGame` keeps thin delegates under
/// the same names, so the click seam and the snapshot surface never
/// change.
#[derive(Default)]
pub(crate) struct LankingState {
    /// Left-list songs behind the lanking board (`OJNList.dat`
    /// order, owned copies; see [`super::ScreenGame::set_lanking_catalog`]).
    pub(crate) lanking_songs: Vec<LankingSong>,
    /// Active left-list sort (`None`: catalog order until a sort
    /// button answers; directions ASSUMED ascending).
    pub(crate) lanking_sort: Option<LankingSort>,
    /// Left-list page (five rows per page, the staged `LVIEW_EX` hit areas).
    pub(crate) lanking_page: usize,
    /// Right-list page (ten board rows per page past the self line).
    pub(crate) lanking_rpage: usize,
    /// Fetched rank board (`2028 MusicPlayRankingAnswer`) staged
    /// behind the scripted-stub seam: proven shell-side, ASSUMED
    /// wire-side until a live gateway replay confirms it.
    pub(crate) lanking_board: Option<MusicPlayRankingAnswer>,
    /// Picked left-list song id (row toggle or LANK button); names
    /// the `MUSICNAME` caption. The song-to-board fetch join stays
    /// OPEN (no live gateway).
    pub(crate) lanking_selected: Option<u32>,
}

impl LankingState {
    /// Stage the catalog (`OJNList.dat` order, owned copies so the
    /// board outlives the parse borrow). Resets sort and pages.
    pub(crate) fn set_catalog(&mut self, entries: &[OjnListEntry<'_>]) {
        self.lanking_songs = lanking_songs(entries);
        self.lanking_sort = None;
        self.lanking_page = 0;
        self.lanking_selected = None;
    }

    /// Stage one fetched rank board behind the scripted-stub seam.
    /// Resets the right-list page.
    pub(crate) fn stage_board(&mut self, answer: MusicPlayRankingAnswer) {
        self.lanking_board = Some(answer);
        self.lanking_rpage = 0;
    }

    /// Active left-list sort (`None`: catalog order).
    pub(crate) fn sort(&self) -> Option<LankingSort> {
        self.lanking_sort
    }

    /// Left-list page (eleven rows per page).
    pub(crate) fn page(&self) -> usize {
        self.lanking_page
    }

    /// Sorted left-list songs on the current page (owned page:
    /// small, five rows at most).
    pub(crate) fn visible_songs(&self) -> Vec<LankingSong> {
        let mut songs = self.lanking_songs.clone();
        if let Some(sort) = self.lanking_sort {
            sort_lanking_songs(&mut songs, sort);
        }
        let (start, end) = lanking_page_window(songs.len(), LANKING_LVIEW_ROWS, self.lanking_page);
        songs[start..end].to_vec()
    }

    /// Fetched rank lines on the current right-list page (wire
    /// order, ten rows at most past the self line; empty until
    /// staged).
    pub(crate) fn visible_ranks(&self) -> Vec<RankEntry> {
        let Some(board) = self.lanking_board.as_ref() else {
            return Vec::new();
        };
        let (start, end) =
            lanking_page_window(board.entries.len(), LANKING_RVIEW_ROWS, self.lanking_rpage);
        board.entries[start..end].to_vec()
    }

    /// Viewer's own fetched rank line for the first right-list
    /// slot (`None` until a board stages).
    pub(crate) fn self_entry(&self) -> Option<RankEntry> {
        self.lanking_board
            .as_ref()
            .map(|board| board.self_entry.clone())
    }

    /// Picked song title for the `MUSICNAME` caption (`None` until
    /// a row toggle or LANK button picks a visible row).
    pub(crate) fn selected_title(&self) -> Option<String> {
        let picked = self.lanking_selected?;
        self.lanking_songs
            .iter()
            .find(|song| song.song_id == picked)
            .map(|song| song.title.clone())
    }

    /// Board action through the control-code dispatch: sorts resort
    /// and reset the page, pagers step and hold at the ends, row
    /// picks name the caption song.
    pub(crate) fn apply_action(&mut self, action: LankingAction) {
        match action {
            LankingAction::Sort(sort) => {
                self.lanking_sort = Some(sort);
                self.lanking_page = 0;
            }
            LankingAction::LViewPrev => {
                self.lanking_page = self.lanking_page.saturating_sub(1);
            }
            LankingAction::LViewNext => {
                if self
                    .lanking_page
                    .saturating_add(1)
                    .saturating_mul(LANKING_LVIEW_ROWS)
                    < self.lanking_songs.len()
                {
                    self.lanking_page = self.lanking_page.saturating_add(1);
                }
            }
            LankingAction::RViewPrev => {
                self.lanking_rpage = self.lanking_rpage.saturating_sub(1);
            }
            LankingAction::RViewNext => {
                let rows = self
                    .lanking_board
                    .as_ref()
                    .map_or(0, |board| board.entries.len());
                if self
                    .lanking_rpage
                    .saturating_add(1)
                    .saturating_mul(LANKING_RVIEW_ROWS)
                    < rows
                {
                    self.lanking_rpage = self.lanking_rpage.saturating_add(1);
                }
            }
            LankingAction::SelectRow(slot) => {
                self.lanking_selected = self.visible_songs().get(slot).map(|song| song.song_id);
            }
        }
    }
}
