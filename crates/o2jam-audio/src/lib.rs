//! Audio for recomp2jam: Vorbis decode (replacing libvorbis) plus a
//! backend trait so game logic stays testable without hardware:
//! [`NullBackend`] (silent, deterministic), [`decode_vorbis`] via
//! `lewton`, and [`CpalBackend`] for live output.

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

/// Resample interleaved `f32` frames by linear interpolation.
/// Exact when rates match; empty in/out stays empty.
fn resample_linear(frames: &[f32], channels: usize, from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || from_rate == 0 || to_rate == 0 || channels == 0 {
        return frames.to_vec();
    }
    let in_frames = frames.len() / channels;
    let out_frames = in_frames * to_rate as usize / from_rate as usize;
    let mut out = Vec::with_capacity(out_frames * channels);
    for i in 0..out_frames {
        let pos = i as f64 * from_rate as f64 / to_rate as f64;
        let lo = pos.floor() as usize;
        let frac = (pos - lo as f64) as f32;
        let hi = (lo + 1).min(in_frames.saturating_sub(1));
        for ch in 0..channels {
            let a = frames[lo * channels + ch];
            let b = frames[hi * channels + ch];
            out.push(a + (b - a) * frac);
        }
    }
    out
}

/// Device-side sample queue: converted/resampled frames plus the
/// consumed-frame counter that anchors the song clock.
#[derive(Debug, Default)]
struct PcmQueue {
    buf: std::collections::VecDeque<f32>,
    consumed_frames: u64,
}

impl PcmQueue {
    fn push_pcm(&mut self, pcm: &PcmSamples, out_channels: usize, out_rate: u32) {
        let in_channels = pcm.channels.max(1) as usize;
        let mut frames: Vec<f32> = pcm.samples.iter().map(|s| *s as f32 / 32768.0).collect();
        frames = resample_linear(&frames, in_channels, pcm.sample_rate, out_rate);
        let in_frames = frames.len() / in_channels.max(1);
        for i in 0..in_frames {
            for ch in 0..out_channels {
                let src = if in_channels == 1 {
                    frames[i]
                } else {
                    frames.get(i * in_channels + ch).copied().unwrap_or(0.0)
                };
                self.buf.push_back(src);
            }
        }
    }

    fn pop_into(&mut self, out: &mut [f32], channels: usize) -> usize {
        let mut frames = 0;
        for frame in out.chunks_mut(channels.max(1)) {
            if self.buf.len() < frame.len() {
                break;
            }
            for sample in frame.iter_mut() {
                *sample = self.buf.pop_front().unwrap_or(0.0);
            }
            frames += 1;
        }
        self.consumed_frames += frames as u64;
        frames
    }
}

/// Live audio backend: pushes decoded PCM through the OS mixer and
/// reports device frames consumed, which anchors the song clock.
/// Created with [`CpalBackend::try_new`]; needs real hardware, so no
/// unit test constructs one (see `PcmQueue` for the tested logic).
pub struct CpalBackend {
    queue: std::sync::Arc<std::sync::Mutex<PcmQueue>>,
    channels: u16,
    rate: u32,
    _stream: cpal::Stream,
}

impl CpalBackend {
    /// Open the default output device. Fails cleanly headless.
    pub fn try_new() -> Result<Self, AudioError> {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::Backend("no default output device".into()))?;
        let supported = device
            .default_output_config()
            .map_err(|e| AudioError::Backend(e.to_string()))?;
        let channels = supported.channels();
        let rate = supported.sample_rate();
        match supported.sample_format() {
            cpal::SampleFormat::F32 => Self::build::<f32>(&device, &supported.into()),
            cpal::SampleFormat::I16 => Self::build::<i16>(&device, &supported.into()),
            cpal::SampleFormat::U16 => Self::build::<u16>(&device, &supported.into()),
            fmt => Err(AudioError::Backend(format!("unsupported format {fmt:?}"))),
        }
        .map(|(queue, stream)| Self {
            queue,
            channels,
            rate,
            _stream: stream,
        })
    }

    fn build<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
    ) -> Result<(std::sync::Arc<std::sync::Mutex<PcmQueue>>, cpal::Stream), AudioError>
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32> + Send + 'static,
    {
        use cpal::traits::DeviceTrait;
        let queue = std::sync::Arc::new(std::sync::Mutex::new(PcmQueue::default()));
        let worker = std::sync::Arc::clone(&queue);
        let channels = config.channels as usize;
        let stream = device
            .build_output_stream(
                *config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    let mut guard = worker.lock().expect("audio queue poisoned");
                    let mut tmp = vec![0.0f32; data.len()];
                    guard.pop_into(&mut tmp, channels);
                    for (slot, sample) in data.iter_mut().zip(tmp.iter()) {
                        *slot = T::from_sample(*sample);
                    }
                },
                |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .map_err(|e| AudioError::Backend(e.to_string()))?;
        use cpal::traits::StreamTrait;
        stream
            .play()
            .map_err(|e| AudioError::Backend(e.to_string()))?;
        Ok((queue, stream))
    }

    /// Device sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Device channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

impl AudioBackend for CpalBackend {
    fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), AudioError> {
        let mut guard = self
            .queue
            .lock()
            .map_err(|_| AudioError::Backend("audio queue poisoned".into()))?;
        guard.push_pcm(pcm, self.channels as usize, self.rate);
        Ok(())
    }

    fn samples_played(&self) -> u64 {
        self.queue.lock().map(|q| q.consumed_frames).unwrap_or(0)
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

    fn stereo_pcm(values: &[i16]) -> PcmSamples {
        PcmSamples {
            samples: values.to_vec(),
            sample_rate: 44100,
            channels: 2,
        }
    }

    #[test]
    fn resample_keeps_rate_matched_audio_exact() {
        let frames = vec![0.25, -0.25, 0.5, -0.5];
        assert_eq!(resample_linear(&frames, 2, 44100, 44100), frames);
    }

    #[test]
    fn resample_doubles_constant_frames() {
        let frames = vec![1.0, 1.0, 1.0, 1.0];
        let doubled = resample_linear(&frames, 2, 22050, 44100);
        assert_eq!(doubled.len(), 8);
        assert!(doubled.iter().all(|&s| (s - 1.0).abs() < 1e-6));
    }

    #[test]
    fn queue_counts_consumed_device_frames() {
        let mut queue = PcmQueue::default();
        queue.push_pcm(&stereo_pcm(&[1000, -1000, 2000, -2000]), 2, 44100);
        let mut out = [0.0f32; 4];
        assert_eq!(queue.pop_into(&mut out, 2), 2);
        assert_eq!(queue.consumed_frames, 2);
        assert_eq!(queue.pop_into(&mut out, 2), 0);
        assert_eq!(queue.consumed_frames, 2);
    }

    #[test]
    fn queue_duplicates_mono_to_stereo() {
        let mut queue = PcmQueue::default();
        let mono = PcmSamples {
            samples: vec![16384],
            sample_rate: 44100,
            channels: 1,
        };
        queue.push_pcm(&mono, 2, 44100);
        let mut out = [0.0f32; 2];
        assert_eq!(queue.pop_into(&mut out, 2), 1);
        assert_eq!(out, [0.5, 0.5]);
    }
}
