//! The staged song download: the request queue a START gate fills.
//!
//! `docs/music-availability.md` §4: when the room master's START finds a
//! member without the room song, the client does not start — it asks the
//! server for the member states (`AcquireMusicRequest` 4034). The 4035
//! answer hands the member a non-zero state, and that member's client
//! **enqueues the room song and pumps the download engine** (VA
//! `0x4bb1e0`) instead of sending `4010 StartGame`.
//!
//! This module is the pure half of that engine: the queue of songs to
//! fetch plus the "a download is running" flag the START control and the
//! waiting scene read (`state+0x4fc`). The transfer itself — WinInet
//! FTP into `MUSIC\Temp\`, the temp→final promote, the `4038` progress
//! and the `5012` announce — is slice 4 and stays out of here.

/// Highest number of queued fetches the staging keeps. The exe's
/// manager vector (`manager+0x64`, element stride 8) has no cap of its
/// own; a room download path enqueues one song at a time, so a small
/// bound keeps a repeated kick from growing the queue without limit.
const MAX_QUEUED_DOWNLOADS: usize = 8;

/// One queued fetch: the song whose `o2ma<id>.ojn` the room needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MusicDownloadRequest {
    /// Song id to fetch.
    pub song_id: u16,
}

/// The staged download queue and its running flag (§4).
///
/// The exe keeps both in the download-manager singleton (global
/// `0x77a868`: queue at `+0x64`, pump at `0x4f9410`) and mirrors the
/// running state on the waiting scene (`+0x4fc`, set while a transfer
/// stands). Enqueuing the same song twice keeps one request, like the
/// manager answering "transfer active?" before it enqueues again
/// (`0x4f7a10`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MusicDownload {
    requests: Vec<MusicDownloadRequest>,
}

impl MusicDownload {
    /// Enqueue one room song for fetching and report whether the queue
    /// took it.
    ///
    /// `false` for song id `0` (no room song staged: nothing names a
    /// chart file) and for a song already queued. The caller marks the
    /// scene's download running on `true` — an enqueued request is what
    /// makes the engine non-idle (§4: "queue empty -> return").
    pub fn enqueue(&mut self, song_id: u16) -> bool {
        if song_id == 0
            || self.requests.len() >= MAX_QUEUED_DOWNLOADS
            || self
                .requests
                .iter()
                .any(|request| request.song_id == song_id)
        {
            return false;
        }
        self.requests.push(MusicDownloadRequest { song_id });
        true
    }

    /// Queued fetches in enqueue order.
    #[must_use]
    pub fn requests(&self) -> &[MusicDownloadRequest] {
        &self.requests
    }

    /// Whether a transfer stands: the scene's `+0x4fc` word.
    #[must_use]
    pub fn is_running(&self) -> bool {
        !self.requests.is_empty()
    }

    /// Drop one finished request (slice 4's pump calls this when a
    /// transfer completes; the last drop clears [`Self::is_running`]).
    pub fn finish(&mut self, song_id: u16) {
        self.requests.retain(|request| request.song_id != song_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueuing_a_room_song_marks_the_download_running() {
        // §4: StartMusicDownload enqueues the room song and the scene
        // marks `+0x4fc = 1` while the queue is non-empty.
        let mut download = MusicDownload::default();
        assert!(!download.is_running());
        assert!(download.enqueue(757));
        assert!(download.is_running());
        assert_eq!(
            download.requests(),
            &[MusicDownloadRequest { song_id: 757 }]
        );
    }

    #[test]
    fn the_same_room_song_is_queued_once() {
        // A repeated kick (every 4035 answer, every START press) never
        // stacks the same fetch: the manager answers "transfer
        // active?" first (§4 step 1).
        let mut download = MusicDownload::default();
        assert!(download.enqueue(757));
        assert!(!download.enqueue(757));
        assert_eq!(download.requests().len(), 1);
    }

    #[test]
    fn an_unstaged_room_song_queues_nothing() {
        // Song id 0 names no chart file, so a kick without a staged
        // room song stays a no-op instead of fetching `o2ma0.ojn`.
        let mut download = MusicDownload::default();
        assert!(!download.enqueue(0));
        assert!(!download.is_running());
    }

    #[test]
    fn finishing_the_last_transfer_clears_the_running_flag() {
        // Slice 4's pump drops a completed request; the scene flag
        // follows the queue (§4: an empty queue returns before the
        // pump).
        let mut download = MusicDownload::default();
        download.enqueue(757);
        download.enqueue(758);
        assert!(download.is_running());
        download.finish(757);
        assert!(download.is_running());
        download.finish(758);
        assert!(!download.is_running());
    }
}
