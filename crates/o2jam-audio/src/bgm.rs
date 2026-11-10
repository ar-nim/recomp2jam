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
//! - Scene entries re-cue their own cue (each state entry pushes the
//!   pair), so this channel keeps a sounding track when an incoming
//!   cue names the record already latched: Planet and its channel
//!   dialog both cue record 9, and the exe's dialog lives inside
//!   `CStatePlanet` — restarting the track on every emblem click was
//!   our own defect (ticket #27). The exe's own push stays
//!   unconditional ([`SceneBgm::play_scene_music`]); the keep-running
//!   policy is [`SceneBgm::cue`].
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

/// Linear gain that applies no scaling (`1.0`): the volume a caller
/// that scales nothing (the launch client's path) cues with.
const UNITY_GAIN: f32 = 1.0;

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

/// One cue as a caller asks it: the exe's `(id, loop)` pair plus the
/// linear volume the caller scales the PCM with (`1.0` for no
/// scaling).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneCue {
    /// One-based record over the bank; negative ids silence.
    pub id: i32,
    /// The cue's loop flag (the exe's `loop` argument).
    pub repeat: bool,
    /// Linear volume the caller scales the cued PCM with.
    pub gain: f32,
}

/// What a cue asks the backend to do — the keep-running policy of
/// [`SceneBgm::cue`].
#[derive(Debug, Clone, Copy)]
pub enum CueOutcome<'a> {
    /// The same record in the same repeat mode already sounds at the
    /// same volume: leave the running track untouched.
    KeepRunning,
    /// Start this PCM from its beginning.
    Restart(&'a PcmSamples),
    /// The cue names no record: stop the channel.
    Stop,
}

/// One cue in effect: the latched selection plus the volume its PCM
/// was scaled with.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LatchedCue {
    selection: BgmSelection,
    gain: f32,
}

/// Single-track scene-BGM channel: a new selection replaces the old
/// one; nothing else ever sounds.
#[derive(Debug, Default)]
pub struct SceneBgm {
    latched: Option<LatchedCue>,
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
        self.latched.map(|latched| latched.selection)
    }

    /// Whether a selection is playing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.latched.is_some()
    }

    /// Play one `Music/BGM.ojm` record as the scene BGM: a negative
    /// id silences, a new selection replaces the playing track, and
    /// `repeat` asks the backend to loop it until replaced. The push
    /// is unconditional, as the exe's own call is.
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
        self.latch(
            SceneCue {
                id,
                repeat,
                gain: UNITY_GAIN,
            },
            bank,
        )
    }

    /// Cue the channel and say what the backend must do — the policy
    /// that keeps a shared track running across scene entries.
    ///
    /// A cue that names the latched record in the same repeat mode at
    /// the same volume while `sounding` reports the backend still
    /// playing it keeps the running track
    /// ([`CueOutcome::KeepRunning`]); a different record, a different
    /// volume, or stopped playback restarts it. Silence over an
    /// already silent channel changes nothing either.
    pub fn cue<'a>(
        &mut self,
        request: SceneCue,
        bank: &'a [Option<PcmSamples>],
        sounding: bool,
    ) -> CueOutcome<'a> {
        if self.keeps_running(request, bank, sounding) {
            return CueOutcome::KeepRunning;
        }
        self.latch(request, bank)
            .map_or(CueOutcome::Stop, CueOutcome::Restart)
    }

    /// Silence the BGM channel.
    pub fn silence(&mut self) {
        self.latched = None;
    }

    /// Whether `request` leaves what sounds untouched (see
    /// [`Self::cue`]).
    fn keeps_running(
        &self,
        request: SceneCue,
        bank: &[Option<PcmSamples>],
        sounding: bool,
    ) -> bool {
        let Some(latched) = self.latched else {
            // A silent channel stays silent on another silence cue.
            return Self::resolve(request.id, request.repeat, bank).is_none();
        };
        let incoming =
            Self::resolve(request.id, request.repeat, bank).map(|(selection, _pcm)| selection);
        incoming == Some(latched.selection) && latched.gain == request.gain && sounding
    }

    /// Latch `request` over `bank` and return its PCM (`None` latches
    /// silence).
    fn latch<'a>(
        &mut self,
        request: SceneCue,
        bank: &'a [Option<PcmSamples>],
    ) -> Option<&'a PcmSamples> {
        match Self::resolve(request.id, request.repeat, bank) {
            Some((selection, pcm)) => {
                self.latched = Some(LatchedCue {
                    selection,
                    gain: request.gain,
                });
                Some(pcm)
            }
            None => {
                self.latched = None;
                None
            }
        }
    }

    /// The selection `id` names over `bank` and its PCM, `None` when
    /// the cue silences (negative, `0`, past the end, or a blank
    /// slot).
    fn resolve(
        id: i32,
        repeat: bool,
        bank: &[Option<PcmSamples>],
    ) -> Option<(BgmSelection, &PcmSamples)> {
        let slot = usize::try_from(id).ok().and_then(|id| id.checked_sub(1))?;
        let pcm = bank.get(slot)?.as_ref()?;
        Some((
            BgmSelection {
                record: slot + 1,
                repeat,
            },
            pcm,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioBackend, NullBackend};

    const BOARD_CUE: SceneCue = SceneCue {
        id: 9,
        repeat: true,
        gain: UNITY_GAIN,
    };

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

    #[test]
    fn play_scene_music_pushes_the_same_record_every_call() {
        // The exe's own push is unconditional; the keep-running
        // policy belongs to the shell's cue seam, not to this call.
        let bank = bank_with_track(10, 9);
        let mut bgm = SceneBgm::new();
        assert!(bgm.play_scene_music(9, true, &bank).is_some());
        assert!(bgm.play_scene_music(9, true, &bank).is_some());
    }

    #[test]
    fn same_record_cue_keeps_the_running_track() {
        let bank = bank_with_track(10, 9);
        let mut bgm = SceneBgm::new();
        assert!(matches!(
            bgm.cue(BOARD_CUE, &bank, false),
            CueOutcome::Restart(_)
        ));
        assert!(bgm.keeps_running(BOARD_CUE, &bank, true));
        assert!(matches!(
            bgm.cue(BOARD_CUE, &bank, true),
            CueOutcome::KeepRunning
        ));
    }

    #[test]
    fn cue_restarts_when_the_backend_stopped_playing() {
        let bank = bank_with_track(10, 9);
        let mut bgm = SceneBgm::new();
        bgm.cue(BOARD_CUE, &bank, false);
        // A played-out voice is no longer running, so the same cue
        // must start the record again.
        assert!(matches!(
            bgm.cue(BOARD_CUE, &bank, false),
            CueOutcome::Restart(_)
        ));
    }

    #[test]
    fn cue_switches_on_a_different_record() {
        let mut bank = bank_with_track(10, 9);
        bank[2] = Some(PcmSamples {
            samples: vec![3000i16; 441],
            sample_rate: 44_100,
            channels: 1,
        });
        let mut bgm = SceneBgm::new();
        bgm.cue(BOARD_CUE, &bank, true);
        let room = SceneCue {
            id: 3,
            repeat: true,
            gain: UNITY_GAIN,
        };
        assert!(matches!(bgm.cue(room, &bank, true), CueOutcome::Restart(_)));
        assert_eq!(bgm.current().map(|selection| selection.record), Some(3));
    }

    #[test]
    fn cue_switches_on_a_repeat_mode_change() {
        let bank = bank_with_track(10, 9);
        let mut bgm = SceneBgm::new();
        bgm.cue(BOARD_CUE, &bank, true);
        let once = SceneCue {
            repeat: false,
            ..BOARD_CUE
        };
        assert!(matches!(bgm.cue(once, &bank, true), CueOutcome::Restart(_)));
    }

    #[test]
    fn cue_restarts_on_a_volume_change() {
        // The live volume only reaches the mixer with the cued PCM,
        // so a gain change must re-cue the same record.
        let bank = bank_with_track(10, 9);
        let mut bgm = SceneBgm::new();
        bgm.cue(BOARD_CUE, &bank, true);
        let quieter = SceneCue {
            gain: 0.5,
            ..BOARD_CUE
        };
        assert!(matches!(
            bgm.cue(quieter, &bank, true),
            CueOutcome::Restart(_)
        ));
    }

    #[test]
    fn cue_silences_a_playing_track_and_keeps_a_silent_channel() {
        let bank = bank_with_track(10, 9);
        let mut bgm = SceneBgm::new();
        bgm.cue(BOARD_CUE, &bank, true);
        let silence = SceneCue {
            id: BGM_SILENCE_ID,
            repeat: false,
            gain: UNITY_GAIN,
        };
        assert!(matches!(bgm.cue(silence, &bank, true), CueOutcome::Stop));
        assert!(!bgm.is_playing());
        // Silence over an already silent channel changes nothing.
        assert!(matches!(
            bgm.cue(silence, &bank, false),
            CueOutcome::KeepRunning
        ));
    }
}
