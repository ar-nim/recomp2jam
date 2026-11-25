//! The staged song download: the request queue a START gate fills.
//!
//! `docs/music-availability.md` §4: when the room master's START finds a
//! member without the room song, the client does not start — it asks the
//! server for the member states (`AcquireMusicRequest` 4034). The 4035
//! answer hands the member a non-zero state, and that member's client
//! **enqueues the room song and pumps the download engine** (VA
//! `0x4bb1e0`) instead of sending `4010 StartGame`.
//!
//! This module owns that engine's staging half: the queue of songs to
//! fetch, the "a download is running" flag the START control reads
//! (`state+0x4fc`), and the temp→final promote of a fetched chart. The
//! `4038` progress and `5012` announce ride the session's gateway.

/// Highest number of queued fetches the staging keeps. The exe's
/// manager vector (`manager+0x64`, element stride 8) has no cap of its
/// own; a room download path enqueues one song at a time, so a small
/// bound keeps a repeated kick from growing the queue without limit.
const MAX_QUEUED_DOWNLOADS: usize = 8;

use std::path::{Path, PathBuf};

use crate::music_install::install_music_dir;

/// Temp folder under `Music\` where the exe parks a transfer before
/// promoting it (`docs/music-availability.md` §4, `MUSIC\Temp\`).
pub const DOWNLOAD_TEMP_FOLDER: &str = "Temp";

/// Chart file name the exe builds for one song id (§4, district VA
/// `0x4f7a30`): `o2ma<id>.ojn`.
#[must_use]
pub fn chart_file_name(song_id: u16) -> String {
    format!("o2ma{song_id}.ojn")
}

/// One finished transfer: the song whose chart now sits in `Music\`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicDownloadCompletion {
    /// Song id the room requested.
    pub song_id: u16,
    /// Promoted chart file name.
    pub file_name: String,
}

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicDownload {
    requests: Vec<MusicDownloadRequest>,
    music_dir: PathBuf,
    source_dir: Option<PathBuf>,
}

impl Default for MusicDownload {
    fn default() -> Self {
        Self {
            requests: Vec::new(),
            music_dir: install_music_dir(),
            source_dir: None,
        }
    }
}

impl MusicDownload {
    /// Point the transfer at its install folder and fetch source.
    ///
    /// The production source is the exe's music FTP host (§4); the
    /// shell takes a local mirror folder so a synthetic chart can
    /// prove the whole promote chain without game data or network.
    /// `None` leaves the engine idle: no source, no transfer.
    pub fn set_transfer_dirs(&mut self, music_dir: PathBuf, source_dir: Option<PathBuf>) {
        self.music_dir = music_dir;
        self.source_dir = source_dir;
    }

    /// Folder the promoted charts land in (`Music\`).
    #[must_use]
    pub fn music_dir(&self) -> &Path {
        &self.music_dir
    }

    /// Advance queued requests whose chart file is available.
    ///
    /// Returns each promoted transfer for the session to report
    /// (`4038` progress, then `5012` completion). A fetch that is not
    /// there yet stays queued for the next frame.
    #[must_use]
    pub fn pump(&mut self) -> Vec<MusicDownloadCompletion> {
        let Some(source_dir) = self.source_dir.clone() else {
            return Vec::new();
        };
        let pending = self.requests.clone();
        let mut completions = Vec::new();
        for request in pending {
            let file_name = chart_file_name(request.song_id);
            let Some(bytes) = read_source_chart(&source_dir, &file_name) else {
                continue;
            };
            if promote_chart(&self.music_dir, &file_name, &bytes) {
                self.finish(request.song_id);
                completions.push(MusicDownloadCompletion {
                    song_id: request.song_id,
                    file_name,
                });
            }
        }
        completions
    }

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

    /// Drop one finished request (the pump calls this when a transfer
    /// completes; the last drop clears [`Self::is_running`]).
    pub fn finish(&mut self, song_id: u16) {
        self.requests.retain(|request| request.song_id != song_id);
    }

    /// Drop every queued fetch after a confirmed shop DOWN_CANCEL.
    /// Only the staged queue changes: no thread handle, file, or
    /// failure notify is claimed by this seam (#831).
    pub fn clear(&mut self) {
        self.requests.clear();
    }
}

/// Read one chart from the fetch source. A missing file means the
/// transfer has not landed yet; it is not an error (§4 retries).
fn read_source_chart(source_dir: &Path, file_name: &str) -> Option<Vec<u8>> {
    match std::fs::read(source_dir.join(file_name)) {
        Ok(bytes) => Some(bytes),
        Err(source) => {
            tracing::debug!(%source, file_name, "chart not fetched yet");
            None
        }
    }
}

/// Park a fetched chart in `Music\\Temp\\`, then promote it to `Music\\`
/// (the exe's two-step rename, VA `0x4fa5b0`). A failed write or
/// rename leaves the request queued for the next pump.
fn promote_chart(music_dir: &Path, file_name: &str, bytes: &[u8]) -> bool {
    let temp_dir = music_dir.join(DOWNLOAD_TEMP_FOLDER);
    if let Err(source) = std::fs::create_dir_all(&temp_dir) {
        tracing::warn!(%source, "download temp folder unavailable");
        return false;
    }
    let staged = temp_dir.join(file_name);
    if let Err(source) = std::fs::write(&staged, bytes) {
        tracing::warn!(%source, file_name, "download staging write failed");
        return false;
    }
    let final_path = music_dir.join(file_name);
    if let Err(source) = std::fs::rename(&staged, &final_path) {
        tracing::warn!(%source, file_name, "chart promotion failed");
        let _ = std::fs::remove_file(&staged);
        return false;
    }
    true
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
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn an_unstaged_room_song_queues_nothing() {
        // Song id 0 names no chart file, so a kick without a staged
        // room song stays a no-op instead of fetching `o2ma0.ojn`.
        let mut download = MusicDownload::default();
        assert!(!download.enqueue(0));
        assert!(!download.is_running());
    }

    /// Throwaway folders for a synthetic transfer.
    fn transfer_dirs(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("source");
        let music = root.join("Music");
        std::fs::create_dir_all(&source).expect("source dir");
        std::fs::create_dir_all(&music).expect("music dir");
        (source, music)
    }

    #[test]
    fn a_fetched_chart_is_promoted_from_temp_into_the_music_folder() {
        let (source, music) = transfer_dirs("recomp2jam-chart-promote");
        std::fs::write(source.join("o2ma757.ojn"), b"synthetic").expect("source chart");
        let mut download = MusicDownload::default();
        download.set_transfer_dirs(music.clone(), Some(source));
        assert!(download.enqueue(757));

        let completions = download.pump();

        assert_eq!(
            completions,
            vec![MusicDownloadCompletion {
                song_id: 757,
                file_name: "o2ma757.ojn".to_owned(),
            }]
        );
        assert_eq!(
            std::fs::read(music.join("o2ma757.ojn")).unwrap(),
            b"synthetic"
        );
        assert!(!music
            .join(DOWNLOAD_TEMP_FOLDER)
            .join("o2ma757.ojn")
            .exists());
        assert!(!download.is_running());
        std::fs::remove_dir_all(music.parent().expect("root")).ok();
    }

    #[test]
    fn an_absent_source_chart_keeps_the_request_queued() {
        let (source, music) = transfer_dirs("recomp2jam-chart-pending");
        let root = source.parent().expect("root").to_path_buf();
        let mut download = MusicDownload::default();
        download.set_transfer_dirs(music, Some(source));
        assert!(download.enqueue(757));
        assert!(download.pump().is_empty());
        assert!(download.is_running());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    // OPEN: this negative assertion is not yet exe-pinned.
    // next evidence step: statics pin + live-input guard; re-pin against the exe per #916.
    fn no_fetch_source_keeps_the_transfer_idle() {
        let (source, music) = transfer_dirs("recomp2jam-chart-idle");
        let root = source.parent().expect("root").to_path_buf();
        let mut download = MusicDownload::default();
        download.set_transfer_dirs(music, None);
        assert!(download.enqueue(757));
        assert!(download.pump().is_empty());
        assert!(download.is_running());
        std::fs::remove_dir_all(root).ok();
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
