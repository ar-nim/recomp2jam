//! Audio for recomp2jam: Vorbis decode (replacing libvorbis) plus a
//! backend trait so game logic stays testable without hardware.
//!
//! Day-0: [`NullBackend`] (silent, deterministic) + [`decode_vorbis`]
//! via `lewton`. A `cpal` backend lands once latency is measured.

use thiserror::Error;

/// Audio errors.
#[derive(Debug, Error)]
pub enum AudioError {
    /// Vorbis decode failure.
    #[error("vorbis decode failed: {0}")]
    Decode(String),
    /// Backend failure.
    #[error("audio backend failed: {0}")]
    Backend(String),
}

/// Decoded PCM: interleaved `i16` samples + sample rate + channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmSamples {
    /// Interleaved little-endian PCM16.
    pub samples: Vec<i16>,
    /// Sample rate in Hz (e.g. 44100).
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u8,
}

impl PcmSamples {
    /// Duration in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0;
        }
        let frames = self.samples.len() as u64 / u64::from(self.channels);
        frames * 1000 / u64::from(self.sample_rate)
    }
}

/// Decode an OGG Vorbis blob (e.g. one `.ojm` payload) to PCM16.
pub fn decode_vorbis(ogg: &[u8]) -> Result<PcmSamples, AudioError> {
    let cursor = std::io::Cursor::new(ogg);
    let mut reader = lewton::inside_ogg::OggStreamReader::new(cursor)
        .map_err(|e| AudioError::Decode(e.to_string()))?;
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    let channels = reader.ident_hdr.audio_channels;
    let mut samples = Vec::new();
    while let Some(packet) = reader
        .read_dec_packet_itl()
        .map_err(|e| AudioError::Decode(e.to_string()))?
    {
        samples.extend_from_slice(&packet);
    }
    Ok(PcmSamples {
        samples,
        sample_rate,
        channels,
    })
}

/// Minimal audio sink. Game code plays key-sounds/BGM through this;
/// tests use [`NullBackend`].
pub trait AudioBackend {
    /// Queue decoded PCM for playback (fire-and-forget at this layer).
    fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), AudioError>;
    /// Samples played so far (for song-clock anchoring later).
    fn samples_played(&self) -> u64;
}

/// Silent deterministic backend for tests and headless runs.
#[derive(Debug, Default)]
pub struct NullBackend {
    played: u64,
}

impl NullBackend {
    /// Create a new silent backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AudioBackend for NullBackend {
    fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), AudioError> {
        self.played += pcm.samples.len() as u64 / pcm.channels.max(1) as u64;
        Ok(())
    }

    fn samples_played(&self) -> u64 {
        self.played
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_duration_math() {
        let pcm = PcmSamples {
            samples: vec![0i16; 44100 * 2],
            sample_rate: 44100,
            channels: 2,
        };
        assert_eq!(pcm.duration_ms(), 1000);
    }

    #[test]
    fn null_backend_counts_frames() {
        let mut b = NullBackend::new();
        let pcm = PcmSamples {
            samples: vec![0i16; 100],
            sample_rate: 44100,
            channels: 2,
        };
        b.play_pcm(&pcm).unwrap();
        assert_eq!(b.samples_played(), 50);
    }

    #[test]
    fn garbage_is_not_valid_ogg() {
        assert!(matches!(
            decode_vorbis(b"definitely not ogg data................"),
            Err(AudioError::Decode(_))
        ));
    }

    /// Local-only parity gate: decodes the real `bgRoom.ogg` from a
    /// 7.55 install. Ignored in CI (needs git-ignored game data).
    /// Verified 2026-11-05: stereo 44100 Hz, 4746240 samples, 53.8 s.
    #[test]
    #[ignore]
    fn decodes_real_bgm_sample() {
        let bytes =
            std::fs::read("../../install-root/O2Jam/Music/BGM.ojm").expect("7.55 install required");
        let blob = &bytes[128..128 + 2381130];
        let pcm = decode_vorbis(blob).expect("bgRoom decodes");
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.sample_rate, 44100);
        assert_eq!(pcm.samples.len(), 4746240);
        assert_eq!(pcm.duration_ms(), 53812);
    }
}
