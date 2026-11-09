//! Scene BGM channel: the exe's `play_scene_music(id, loop)`
//! contract — one track at a time, id `-1` silences, records are
//! one-based over `Music/BGM.ojm`. The replay rate is `repeat`
//! (`true` repeats until replaced, `false` plays once).
//!
//! Pins (OTwo.exe 7.55, 5,029,888 B, SHA-256 `83a04541…72c823b`;
//! file offset = VA − `0x400000`; image base `0x400000`):
//! - The selector is declared at file `0x151820` (VA `0x551820`,
//!   35 B); it forwards its two arguments to the bank sub-object
//!   behind the host at `0x76832C + 0x340C0` — observed with
//!   rea + Ghidra (`ev_3ccc21224ad3c5e1bee5694a9a8df6d066912e73dc979128ed3cdf2e6e7ca471`).
//! - Every immediate caller lowers to an `(id, loop)` push pair at
//!   the call site (id pushed last, loop first). The scene → cue
//!   map is owned by the launch client's table
//!   (`scene_bgm_cue` / `side_scene_bgm_cue` in
//!   `recomp2jam/src/main.rs`), verified call-by-call with the
//!   same rea + Ghidra checks (2026-11-11).
//! - `Music/BGM.ojm` holds 10 fetch-order records (mode-2 then
//!   mode-3, blanks kept: the empty slots carry the scene ids'
//!   positions, e.g. record 3 names the room track, record 10 the
//!   music-shop track) — the alignment this one-based contract
//!   encodes, verified by `o2jam-dump dump-ojm` on the installed
//!   `BGM.ojm`.
//! - Two of our earlier (audit) numbers move toward the exe: the
//!   result cue is `(0x13, once)` (the exe pushes `0x0` for loop)
//!   and the music-shop cue is decimal `10` (the exe pushes `0xA`),
//!   not hex `0x10`. Record 19 then exceeds `BGM.ojm`'s 10
//!   records, so the result scene stays silent here; its bank is
//!   OPEN (next step: identify which bank the worker holds when
//!   `0x4C1B90` cues it — `BGMwinter.ojm`, which carries a result
//!   track, is the candidate).

use crate::PcmSamples;

/// Selection id that silences the BGM channel (the exe cues it as
/// `0xFFFFFFFF`; any negative id silences, only `-1` is pinned).
pub const BGM_SILENCE_ID: i32 = -1;

/// One latched scene-BGM selection: a one-based record over the
/// bank plus the cue's loop flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BgmSelection {
    /// One-based record number (`1` = first bank entry, blanks
    /// included) — the cue's `id`.
    pub record: usize,
    /// Whether the track repeats until replaced (the cue's loop
    /// flag).
    pub repeat: bool,
}

/// Single-track scene-BGM channel: a new selection replaces the old
/// one; nothing else ever sounds.
#[derive(Debug, Default)]
pub struct SceneBgm {
    current: Option<BgmSelection>,
}

impl SceneBgm {
    /// New silent channel.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The latched selection, if a track is playing.
    #[must_use]
    pub fn current(&self) -> Option<BgmSelection> {
        self.current
    }

    /// Whether a selection is playing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.current.is_some()
    }

    /// Play one `Music/BGM.ojm` record as the scene BGM: a negative
    /// id silences, a new selection replaces the playing track, and
    /// `repeat` asks the backend to loop it until replaced.
    ///
    /// The bank holds one slot per record in bank order (blank
    /// records as `None`); `id` is the one-based record number.
    /// An id naming no record (`0`, past the end, or a blank slot)
    /// silences the channel and returns `None`; otherwise the
    /// selection latches and the record's PCM is returned.
    pub fn play_scene_music<'a>(
        &mut self,
        id: i32,
        repeat: bool,
        bank: &'a [Option<PcmSamples>],
    ) -> Option<&'a PcmSamples> {
        let slot = usize::try_from(id).ok().and_then(|id| id.checked_sub(1));
        let pcm = slot
            .and_then(|slot| bank.get(slot))
            .and_then(|entry| entry.as_ref());
        self.current = match (slot, pcm) {
            (Some(slot), Some(_)) => Some(BgmSelection {
                record: slot + 1,
                repeat,
            }),
            _ => None,
        };
        pcm
    }

    /// Silence the BGM channel.
    pub fn silence(&mut self) {
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioBackend, NullBackend};

    fn silent_bank(slots: usize) -> Vec<Option<PcmSamples>> {
        vec![None; slots]
    }

    fn bank_with_track(slots: usize, record: usize) -> Vec<Option<PcmSamples>> {
        let mut bank = silent_bank(slots);
        bank[record - 1] = Some(PcmSamples {
            samples: vec![1000i16; 441],
            sample_rate: 44100,
            channels: 1,
        });
        bank
    }

    #[test]
    fn negative_id_silences_a_playing_track() {
        let bank = bank_with_track(10, 3);
        let mut bgm = SceneBgm::new();
        assert!(bgm.play_scene_music(3, true, &bank).is_some());
        assert!(bgm.is_playing());
        assert_eq!(bgm.play_scene_music(-1, true, &bank), None);
        assert!(!bgm.is_playing());
    }

    #[test]
    fn new_selection_replaces_the_playing_track() {
        let mut bank = bank_with_track(10, 3);
        bank[3] = Some(PcmSamples {
            samples: vec![2000i16; 441],
            sample_rate: 44100,
            channels: 1,
        });
        let mut bgm = SceneBgm::new();
        bgm.play_scene_music(3, true, &bank);
        bgm.play_scene_music(4, true, &bank);
        assert_eq!(
            bgm.current(),
            Some(BgmSelection {
                record: 4,
                repeat: true
            })
        );
    }

    #[test]
    fn one_based_record_selects_the_matching_bank_entry() {
        let bank = bank_with_track(10, 3);
        let mut bgm = SceneBgm::new();
        // Record 3 is the third slot; records 1/2 are blank slots.
        assert!(bgm.play_scene_music(3, true, &bank).is_some());
        assert_eq!(
            bgm.current(),
            Some(BgmSelection {
                record: 3,
                repeat: true
            })
        );
        assert_eq!(bgm.play_scene_music(1, true, &bank), None);
        assert!(!bgm.is_playing());
    }

    #[test]
    fn zero_and_unknown_ids_silence_the_channel() {
        let bank = bank_with_track(10, 3);
        let mut bgm = SceneBgm::new();
        bgm.play_scene_music(3, true, &bank);
        assert_eq!(bgm.play_scene_music(0, true, &bank), None);
        assert!(!bgm.is_playing());
        bgm.play_scene_music(3, true, &bank);
        assert_eq!(bgm.play_scene_music(19, false, &bank), None);
        assert!(!bgm.is_playing());
    }

    #[test]
    fn cue_through_null_backend_counts_frames_and_stays_contained() {
        // Containment rule: tests exercise the null audio path only.
        let bank = bank_with_track(10, 8);
        let mut bgm = SceneBgm::new();
        let pcm = bgm.play_scene_music(8, false, &bank);
        let mut backend = NullBackend::new();
        backend.set_scene_bgm(pcm, false).unwrap();
        assert_eq!(backend.samples_played(), 441);
        backend.set_scene_bgm(None, false).unwrap();
        assert_eq!(backend.samples_played(), 441);
    }

    #[test]
    fn silence_call_drops_a_playing_track() {
        let bank = bank_with_track(10, 3);
        let mut bgm = SceneBgm::new();
        bgm.play_scene_music(3, true, &bank);
        bgm.silence();
        assert_eq!(bgm.current(), None);
    }
}
