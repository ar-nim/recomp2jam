//! Audio for recomp2jam: Vorbis decode (replacing libvorbis) plus a
//! backend trait so game logic stays testable without hardware:
//! [`NullBackend`] (silent, deterministic), [`decode_vorbis`] via
//! `lewton`, and [`CpalBackend`] for live output.

mod bgm;

pub use bgm::{BgmSelection, CueOutcome, SceneBgm, SceneCue, BGM_SILENCE_ID};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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
///
/// # Errors
///
/// `AudioError::Decode` when the bytes are not a decodable OGG stream.
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
///
/// The song clock is [`AudioBackend::samples_played`]: device frames
/// consumed so far. Mixing a key sound never moves the clock; the voice
/// starts at the playhead (front of the not-yet-played queue), so its
/// onset latency is one callback quantum plus the device base latency
/// (see [`LatencyProbe`])..
pub trait AudioBackend {
    /// Queue decoded PCM for playback (fire-and-forget at this layer).
    ///
    /// # Errors
    ///
    /// `AudioError::Backend` when the device rejects the samples.
    fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), AudioError>;
    /// Mix one struck key sound at the current song position.
    ///
    /// Linear `gain` (`1.0` is unity); silent gains (`<= 0.0`) are a
    /// no-op. The default routes through [`AudioBackend::play_pcm`] so
    /// clock-only sinks (like [`NullBackend`]) keep counting struck
    /// frames; [`CpalBackend`] overrides this with a true playhead mix.
    ///
    /// # Errors
    ///
    /// `AudioError::Backend` when the device rejects the voice.
    fn play_key(&mut self, pcm: &PcmSamples, gain: f32) -> Result<(), AudioError> {
        if gain <= 0.0 {
            return Ok(());
        }
        self.play_pcm(pcm)
    }
    /// Start (repeat) or stop (`None`) the scene BGM behind the
    /// queued audio (the exe's `play_scene_music` contract, see
    /// [`SceneBgm`]): a selection replaces the playing one.
    ///
    /// The default counts the cue once through
    /// [`AudioBackend::play_pcm`] (clock-only sinks stay silent and
    /// deterministic); a live tail that actually repeats is a
    /// [`CpalBackend`] feature. `repeat` is the cue's loop flag:
    /// `true` repeats the record until a new selection replaces
    /// it, `false` plays it once.
    ///
    /// # Errors
    ///
    /// `AudioError::Backend` when the backend rejects the voice.
    fn set_scene_bgm(&mut self, pcm: Option<&PcmSamples>, repeat: bool) -> Result<(), AudioError> {
        let _ = repeat;
        if let Some(pcm) = pcm {
            self.play_pcm(pcm)?;
        }
        Ok(())
    }
    /// Whether the scene-BGM voice is still sounding: `true` while
    /// a cued track plays, `false` once it stopped (a once-voice
    /// that played out, or a stop cue). The shell's cue policy asks
    /// this before it keeps a running track instead of restarting it
    /// (see [`SceneBgm::cue`]).
    ///
    /// The default answers `false`: a sink that models no BGM voice
    /// cannot confirm playback, so every cue re-applies (the exe's
    /// unconditional push).
    fn scene_bgm_playing(&self) -> bool {
        false
    }
    /// Song position in device frames (see trait docs).
    fn samples_played(&self) -> u64;
    /// Device sample rate in Hz: the rate [`Self::samples_played`]
    /// counts at. The song clock converts frames with this, so a
    /// live device at (say) 48000 Hz runs at the right speed.
    /// Sinks that count staged frames at their own rate return it.
    fn sample_rate(&self) -> u32 {
        44_100
    }
    /// Queued-but-unplayed frames behind the playhead. Defaults to
    /// zero for clock-only sinks; live backends report real depth for
    /// [`LatencyProbe`].
    fn pending_frames(&self) -> u64 {
        0
    }
}

/// Silent deterministic backend for tests and headless runs.
#[derive(Debug, Default)]
pub struct NullBackend {
    played: u64,
    bgm: Option<NullBgmVoice>,
}

/// The scene-BGM voice a [`NullBackend`] holds: a clock-only sink
/// takes every pushed frame at once, so only a repeating track stays
/// installed (a once track plays out inside its own push).
#[derive(Debug, Clone, Copy)]
struct NullBgmVoice {
    repeat: bool,
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

    fn set_scene_bgm(&mut self, pcm: Option<&PcmSamples>, repeat: bool) -> Result<(), AudioError> {
        self.bgm = pcm.map(|_| NullBgmVoice { repeat });
        match pcm {
            Some(pcm) => self.play_pcm(pcm),
            None => Ok(()),
        }
    }

    fn scene_bgm_playing(&self) -> bool {
        self.bgm.is_some_and(|voice| voice.repeat)
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

/// Convert decoded PCM to normalized device-rate `f32` frames.
///
/// Shared by the live queue and [`SongMixer`] voices so a struck key
/// sounds identical mixed live or baked offline: mono duplicates to
/// every output channel, wider input folds down by averaging, narrower
/// input picks its channel subset. Output holds whole frames only.
fn voice_frames(pcm: &PcmSamples, out_channels: usize, out_rate: u32) -> Vec<f32> {
    let out_channels = out_channels.max(1);
    let in_channels = pcm.channels.max(1) as usize;
    let mono: Vec<f32> = pcm
        .samples
        .iter()
        .map(|s| f32::from(*s) / 32768.0)
        .collect();
    let resampled = resample_linear(&mono, in_channels, pcm.sample_rate, out_rate);
    let in_frames = resampled.len() / in_channels.max(1);
    let mut frames = Vec::with_capacity(in_frames * out_channels);
    for i in 0..in_frames {
        if in_channels == 1 {
            for _ in 0..out_channels {
                frames.push(resampled[i]);
            }
        } else if in_channels >= out_channels {
            for ch in 0..out_channels {
                frames.push(resampled[i * in_channels + ch]);
            }
        } else {
            let mut sum = 0.0;
            for c in 0..in_channels {
                sum += resampled[i * in_channels + c];
            }
            let average = sum / in_channels as f32;
            for _ in 0..out_channels {
                frames.push(average);
            }
        }
    }
    frames
}

/// A scene-BGM voice behind the queued audio: device-rate frames
/// converted once at selection time, pulled by the realtime
/// callback only when the queue is dry. `repeat` wraps the voice
/// when it ends; a once-voice clears itself when played out.
#[derive(Debug)]
struct LoopVoice {
    frames: Vec<f32>,
    pos: usize,
    repeat: bool,
}

/// Device-side sample queue: converted/resampled frames plus the
/// consumed-frame counter that anchors the song clock.
#[derive(Debug, Default)]
struct PcmQueue {
    buf: std::collections::VecDeque<f32>,
    consumed_frames: u64,
    bgm: Option<LoopVoice>,
}

impl PcmQueue {
    fn push_pcm(&mut self, pcm: &PcmSamples, out_channels: usize, out_rate: u32) {
        self.buf.extend(voice_frames(pcm, out_channels, out_rate));
    }

    /// Layer `voice` (device-rate frames, already gain-scaled) over the
    /// pending queue starting at the playhead: indices before the
    /// playhead do not exist, so mixing starts at the front of what is
    /// still queued and extends the tail when the voice rings past it.
    /// Sums clamp to `[-1.0, 1.0]` (no wraparound), matching
    /// [`SongMixer::mix`]. Never moves `consumed_frames`.
    fn mix_at_playhead(&mut self, voice: &[f32]) {
        for (i, sample) in voice.iter().enumerate() {
            match self.buf.get_mut(i) {
                Some(slot) => *slot = (*slot + *sample).clamp(-1.0, 1.0),
                None => self.buf.push_back(sample.clamp(-1.0, 1.0)),
            }
        }
    }

    /// Samples still queued behind the playhead.
    fn pending_samples(&self) -> u64 {
        u64::try_from(self.buf.len()).unwrap_or(u64::MAX)
    }

    /// Buffered sample count for the realtime callback's underflow check.
    fn buffered_samples(&self) -> usize {
        self.buf.len()
    }

    /// Pop one pending sample; the BGM voice sounds only when the
    /// queue is dry; silence when both are dry.
    fn pop_sample(&mut self) -> f32 {
        if let Some(sample) = self.buf.pop_front() {
            sample
        } else {
            self.next_bgm_sample().unwrap_or(0.0)
        }
    }

    /// Note one fully served device frame (realtime side only).
    fn mark_frame_consumed(&mut self) {
        self.consumed_frames = self.consumed_frames.saturating_add(1);
    }

    /// Start a scene-BGM voice (the one at a time): a selection
    /// replaces the playing voice. Empty audio starts no voice.
    fn start_bgm(&mut self, pcm: &PcmSamples, out_channels: usize, out_rate: u32, repeat: bool) {
        let frames = voice_frames(pcm, out_channels, out_rate);
        if frames.is_empty() {
            self.stop_bgm();
            return;
        }
        self.bgm = Some(LoopVoice {
            frames,
            pos: 0,
            repeat,
        });
    }

    /// Stop the scene-BGM voice.
    fn stop_bgm(&mut self) {
        self.bgm = None;
    }

    /// Whether a BGM voice is installed (the queue never runs dry
    /// while one plays).
    fn has_bgm(&self) -> bool {
        self.bgm.is_some()
    }

    /// Next BGM sample, wrapping on `repeat`; a played-out
    /// once-voice clears itself. `None` with no voice installed.
    fn next_bgm_sample(&mut self) -> Option<f32> {
        let voice = self.bgm.as_mut()?;
        if voice.frames.is_empty() {
            return None;
        }
        let sample = voice.frames[voice.pos];
        let at_end = voice.pos + 1 >= voice.frames.len();
        voice.pos = if at_end { 0 } else { voice.pos + 1 };
        if at_end && !voice.repeat {
            self.bgm = None;
        }
        Some(sample)
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
    ///
    /// # Errors
    ///
    /// `AudioError::Backend` with no device, config, or stream.
    pub fn try_new() -> Result<Self, AudioError> {
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
        let queue = std::sync::Arc::new(std::sync::Mutex::new(PcmQueue::default()));
        let worker = std::sync::Arc::clone(&queue);
        let channels = config.channels as usize;
        let stream = device
            .build_output_stream(
                *config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    // Realtime context: never block, allocate, or panic
                    // (CONCURRENCY-02/20, ERRORS-06, PERF-07). A contended
                    // or poisoned queue emits silence for this quantum;
                    // the song clock stalls with it instead of tearing.
                    let Ok(mut guard) = worker.try_lock() else {
                        for slot in data.iter_mut() {
                            *slot = T::from_sample(0.0);
                        }
                        return;
                    };
                    for frame in data.chunks_mut(channels.max(1)) {
                        if guard.buffered_samples() < frame.len() && !guard.has_bgm() {
                            for slot in frame.iter_mut() {
                                *slot = T::from_sample(0.0);
                            }
                            continue;
                        }
                        for slot in frame.iter_mut() {
                            *slot = T::from_sample(guard.pop_sample());
                        }
                        guard.mark_frame_consumed();
                    }
                },
                |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .map_err(|e| AudioError::Backend(e.to_string()))?;
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

    /// Layer one struck key sound over the pending queue at the current
    /// song position (the playhead). Same conversion as [`SongMixer`]
    /// voices, so live strikes match baked mixes; silent gains
    /// (`<= 0.0`) are a no-op and the consumed-frame clock is untouched.
    ///
    /// # Errors
    ///
    /// `AudioError::Backend` when the queue lock is poisoned.
    pub fn play_key(&mut self, pcm: &PcmSamples, gain: f32) -> Result<(), AudioError> {
        if gain <= 0.0 {
            return Ok(());
        }
        let voice: Vec<f32> = voice_frames(pcm, usize::from(self.channels.max(1)), self.rate)
            .iter()
            .map(|sample| sample * gain)
            .collect();
        let mut guard = self
            .queue
            .lock()
            .map_err(|_| AudioError::Backend("audio queue poisoned".into()))?;
        guard.mix_at_playhead(&voice);
        Ok(())
    }

    /// Start (repeat) or stop (`None`) the scene BGM behind the
    /// queued audio: the voice converts once here (never in the
    /// callback) and sounds once the queue drains.
    ///
    /// # Errors
    ///
    /// `AudioError::Backend` when the queue lock is poisoned.
    pub fn set_scene_bgm(
        &mut self,
        pcm: Option<&PcmSamples>,
        repeat: bool,
    ) -> Result<(), AudioError> {
        let mut guard = self
            .queue
            .lock()
            .map_err(|_| AudioError::Backend("audio queue poisoned".into()))?;
        match pcm {
            Some(pcm) => guard.start_bgm(pcm, usize::from(self.channels.max(1)), self.rate, repeat),
            None => guard.stop_bgm(),
        }
        Ok(())
    }

    /// Queued-but-unplayed device frames behind the playhead — the
    /// BGM tail is not counted: it is the (possibly endless) voice
    /// behind the queue, not queue depth.
    #[must_use]
    pub fn pending_frames(&self) -> u64 {
        let channels = u64::from(self.channels.max(1));
        self.queue
            .lock()
            .map_or(0, |q| q.pending_samples() / channels)
    }

    /// Snapshot a [`LatencyProbe`] for this backend: pending queue depth
    /// at the device rate plus the caller's device base latency.
    #[must_use]
    pub fn latency_probe(&self, device_latency_frames: u64) -> LatencyProbe {
        LatencyProbe::new(self.rate, self.pending_frames(), device_latency_frames)
    }
}

impl AudioBackend for CpalBackend {
    fn play_pcm(&mut self, pcm: &PcmSamples) -> Result<(), AudioError> {
        let mut guard = self
            .queue
            .lock()
            .map_err(|_| AudioError::Backend("audio queue poisoned".into()))?;
        guard.push_pcm(pcm, usize::from(self.channels.max(1)), self.rate);
        Ok(())
    }

    fn play_key(&mut self, pcm: &PcmSamples, gain: f32) -> Result<(), AudioError> {
        CpalBackend::play_key(self, pcm, gain)
    }

    fn set_scene_bgm(&mut self, pcm: Option<&PcmSamples>, repeat: bool) -> Result<(), AudioError> {
        CpalBackend::set_scene_bgm(self, pcm, repeat)
    }

    fn scene_bgm_playing(&self) -> bool {
        self.queue.lock().is_ok_and(|queue| queue.has_bgm())
    }

    fn samples_played(&self) -> u64 {
        self.queue.lock().map_or(0, |q| q.consumed_frames)
    }

    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn pending_frames(&self) -> u64 {
        CpalBackend::pending_frames(self)
    }
}

/// One mixed voice: normalized frames plus a linear gain and a start
/// offset in song frames. Gain `1.0` is unity; `0.0` is silent.
#[derive(Debug, Clone)]
struct MixerVoice {
    frames: Vec<f32>,
    gain: f32,
    start_frame: u64,
}

/// Song mixer: sums BGM plus per-note key-sounds into one PCM blob.
///
/// Voices are resampled to the mixer rate on `add_voice`, then summed
/// in `mix` with clamping (no wraparound). Mono voices duplicate to
/// every mixer channel, extra channels fold down by averaging.
#[derive(Debug, Default)]
pub struct SongMixer {
    sample_rate: u32,
    channels: usize,
    voices: Vec<MixerVoice>,
}

impl SongMixer {
    /// Create a mixer for song-rate output (`channels` > 0).
    #[must_use]
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        Self {
            sample_rate,
            channels: channels.max(1),
            voices: Vec::new(),
        }
    }

    /// Mixer output rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Mixer channel count.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Queue one note/BGM voice with a linear per-note `gain`.
    /// Negative gains clamp to silence; the voice starts at
    /// `start_frame` song frames.
    pub fn add_voice(&mut self, pcm: &PcmSamples, gain: f32, start_frame: u64) {
        let gain = gain.max(0.0);
        if gain == 0.0 {
            return;
        }
        let in_channels = pcm.channels.max(1) as usize;
        let mono: Vec<f32> = pcm
            .samples
            .iter()
            .map(|s| f32::from(*s) / 32768.0)
            .collect();
        let resampled = resample_linear(&mono, in_channels, pcm.sample_rate, self.sample_rate);
        let in_frames = resampled.len() / in_channels.max(1);
        let mut frames = Vec::with_capacity(in_frames * self.channels);
        for i in 0..in_frames {
            for ch in 0..self.channels {
                let sample = if in_channels == 1 {
                    resampled[i]
                } else if in_channels >= self.channels {
                    resampled[i * in_channels + ch]
                } else {
                    let mut sum = 0.0;
                    for c in 0..in_channels {
                        sum += resampled[i * in_channels + c];
                    }
                    sum / in_channels as f32
                };
                frames.push(sample);
            }
        }
        self.voices.push(MixerVoice {
            frames,
            gain,
            start_frame,
        });
    }

    /// Song length in frames (longest voice end).
    #[must_use]
    pub fn song_frames(&self) -> u64 {
        self.voices
            .iter()
            .map(|voice| voice.start_frame + voice.frames.len() as u64 / self.channels as u64)
            .max()
            .unwrap_or(0)
    }

    /// Sum all voices into one clamped PCM blob.
    #[must_use]
    pub fn mix(&self) -> PcmSamples {
        let total = self.song_frames() as usize;
        let mut acc = vec![0.0f32; total * self.channels];
        for voice in &self.voices {
            let start = voice.start_frame as usize * self.channels;
            for (i, sample) in voice.frames.iter().enumerate() {
                if let Some(slot) = acc.get_mut(start + i) {
                    *slot += *sample * voice.gain;
                }
            }
        }
        let samples = acc
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16)
            .collect();
        PcmSamples {
            samples,
            sample_rate: self.sample_rate,
            channels: self.channels as u8,
        }
    }
}

/// Convert a device frame counter to song milliseconds. This wires the
/// song clock to audio time: callers pass
/// [`AudioBackend::samples_played`] and the device rate.
#[must_use]
pub fn song_position_ms(samples_played: u64, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }
    samples_played.saturating_mul(1000) / u64::from(sample_rate)
}

/// Song clock anchored on audio device frames: the single seam
/// between game timing and the audio backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongClock {
    sample_rate: u32,
    start_sample: u64,
}

impl SongClock {
    /// Anchor a song at `start_sample` device frames.
    #[must_use]
    pub fn new(sample_rate: u32, start_sample: u64) -> Self {
        Self {
            sample_rate,
            start_sample,
        }
    }

    /// Song position in ms for a `samples_played` reading.
    #[must_use]
    pub fn position_ms(&self, samples_played: u64) -> u64 {
        song_position_ms(
            samples_played.saturating_sub(self.start_sample),
            self.sample_rate,
        )
    }

    /// Signed drift in ms: positive means the audio clock runs ahead
    /// of the expected (frame-clock) position.
    #[must_use]
    pub fn drift_ms(&self, samples_played: u64, expected_ms: u64) -> i64 {
        self.position_ms(samples_played) as i64 - expected_ms as i64
    }
}

/// Output-latency probe: queued-but-unplayed frames plus the device's
/// own base latency, all reported in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyProbe {
    sample_rate: u32,
    queued_frames: u64,
    device_latency_frames: u64,
}

impl LatencyProbe {
    /// Snapshot a probe reading; zeros guard divide-by-zero callers.
    #[must_use]
    pub fn new(sample_rate: u32, queued_frames: u64, device_latency_frames: u64) -> Self {
        Self {
            sample_rate,
            queued_frames,
            device_latency_frames,
        }
    }

    /// Milliseconds of audio queued but not yet played.
    #[must_use]
    pub fn pending_ms(&self) -> u64 {
        song_position_ms(self.queued_frames, self.sample_rate)
    }

    /// Total output latency: device base plus queued audio.
    #[must_use]
    pub fn total_ms(&self) -> u64 {
        song_position_ms(
            self.queued_frames
                .saturating_add(self.device_latency_frames),
            self.sample_rate,
        )
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
    /// 7.55 install. Skips when `install-root` is absent.
    /// Verified 2026-11-05: stereo 44100 Hz, 4746240 samples, 53.8 s.
    #[test]
    fn decodes_real_bgm_sample() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../install-root/O2Jam/Music/BGM.ojm");
        let Ok(bytes) = std::fs::read(&path) else {
            println!("skip: missing {}", path.display());
            return;
        };
        let blob = &bytes[128..128 + 2_381_130];
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

    /// Drain whole frames the way the realtime callback does: full
    /// frames pop and advance the song clock, short tails stay silent
    /// without moving it.
    fn drain_frames(queue: &mut PcmQueue, channels: usize, out: &mut [f32]) -> usize {
        let mut frames = 0;
        for frame in out.chunks_mut(channels.max(1)) {
            if queue.buffered_samples() < frame.len() {
                break;
            }
            for sample in frame.iter_mut() {
                *sample = queue.pop_sample();
            }
            queue.mark_frame_consumed();
            frames += 1;
        }
        frames
    }

    #[test]
    fn queue_counts_consumed_device_frames() {
        let mut queue = PcmQueue::default();
        queue.push_pcm(&stereo_pcm(&[1000, -1000, 2000, -2000]), 2, 44100);
        let mut out = [0.0f32; 4];
        assert_eq!(drain_frames(&mut queue, 2, &mut out), 2);
        assert_eq!(queue.consumed_frames, 2);
        assert_eq!(drain_frames(&mut queue, 2, &mut out), 0);
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
        assert_eq!(drain_frames(&mut queue, 2, &mut out), 1);
        assert_eq!(out, [0.5, 0.5]);
    }

    #[test]
    fn key_voice_mixes_over_pending_audio_at_playhead() {
        let mut queue = PcmQueue::default();
        queue.push_pcm(&stereo_pcm(&[10000, -10000, 20000, -20000]), 2, 44100);
        let voice = voice_frames(&stereo_pcm(&[10000, 10000]), 2, 44100);
        queue.mix_at_playhead(&voice);
        let mut out = [0.0f32; 4];
        assert_eq!(drain_frames(&mut queue, 2, &mut out), 2);
        assert!((out[0] - 20000.0 / 32768.0).abs() < 1e-6);
        assert_eq!(queue.consumed_frames, 2);
    }

    #[test]
    fn key_voice_rings_past_tail_without_moving_clock() {
        let mut queue = PcmQueue::default();
        let voice = voice_frames(&mono_pcm(&[16384]), 1, 44100);
        queue.mix_at_playhead(&voice);
        assert_eq!(queue.consumed_frames, 0);
        let mut out = [0.0f32; 1];
        assert_eq!(drain_frames(&mut queue, 1, &mut out), 1);
        assert!((out[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn looping_bgm_repeats_until_replaced() {
        let mut queue = PcmQueue::default();
        queue.start_bgm(&mono_pcm(&[16384, -16384]), 1, 44100, true);
        let mut out = [0.0f32; 6];
        for slot in &mut out {
            *slot = queue.pop_sample();
        }
        assert_eq!(out, [0.5, -0.5, 0.5, -0.5, 0.5, -0.5]);
        assert!(queue.has_bgm());
        queue.stop_bgm();
        assert_eq!(queue.pop_sample(), 0.0);
    }

    #[test]
    fn once_bgm_plays_the_track_once_then_stops() {
        let mut queue = PcmQueue::default();
        queue.start_bgm(&mono_pcm(&[16384]), 1, 44100, false);
        assert_eq!(queue.pop_sample(), 0.5);
        assert!(!queue.has_bgm());
        assert_eq!(queue.pop_sample(), 0.0);
    }

    #[test]
    fn new_bgm_selection_replaces_the_looping_voice() {
        let mut queue = PcmQueue::default();
        queue.start_bgm(&mono_pcm(&[16384]), 1, 44100, true);
        queue.start_bgm(&mono_pcm(&[-16384]), 1, 44100, true);
        assert_eq!(queue.pop_sample(), -0.5);
        assert_eq!(queue.pop_sample(), -0.5);
    }

    #[test]
    fn queued_audio_sounds_before_the_bgm_tail() {
        let mut queue = PcmQueue::default();
        queue.push_pcm(&mono_pcm(&[8192]), 1, 44100);
        queue.start_bgm(&mono_pcm(&[16384]), 1, 44100, true);
        assert_eq!(queue.pop_sample(), 0.25);
        assert_eq!(queue.pop_sample(), 0.5);
    }

    #[test]
    fn empty_bgm_starts_no_voice() {
        let mut queue = PcmQueue::default();
        let empty = PcmSamples {
            samples: vec![],
            sample_rate: 44100,
            channels: 1,
        };
        queue.start_bgm(&empty, 1, 44100, true);
        assert!(!queue.has_bgm());
    }

    #[test]
    fn null_backend_key_hit_counts_frames_like_struck_note() {
        let mut backend = NullBackend::new();
        let pcm = PcmSamples {
            samples: vec![0i16; 100],
            sample_rate: 44100,
            channels: 2,
        };
        backend.play_key(&pcm, 1.0).unwrap();
        assert_eq!(backend.samples_played(), 50);
        assert_eq!(backend.pending_frames(), 0);
    }

    #[test]
    fn null_backend_silent_key_gain_leaves_clock_untouched() {
        let mut backend = NullBackend::new();
        let pcm = PcmSamples {
            samples: vec![1000i16; 100],
            sample_rate: 44100,
            channels: 2,
        };
        backend.play_key(&pcm, 0.0).unwrap();
        assert_eq!(backend.samples_played(), 0);
    }

    #[test]
    fn mixer_voice_conversion_matches_live_queue_depth() {
        let pcm = stereo_pcm(&[16384, -16384, 16384, -16384]);
        let mut queue = PcmQueue::default();
        queue.push_pcm(&pcm, 2, 44100);
        assert_eq!(queue.pending_samples(), 4);
    }

    fn mono_pcm(values: &[i16]) -> PcmSamples {
        PcmSamples {
            samples: values.to_vec(),
            sample_rate: 44100,
            channels: 1,
        }
    }

    #[test]
    fn mixer_applies_per_note_gain() {
        let mut mixer = SongMixer::new(44100, 1);
        mixer.add_voice(&mono_pcm(&[10000, 10000]), 1.0, 0);
        mixer.add_voice(&mono_pcm(&[10000, 10000]), 0.5, 0);
        let mixed = mixer.mix();
        assert_eq!(mixed.samples.len(), 2);
        assert!((mixed.samples[0] - 15000).abs() <= 2);
        assert!((mixed.samples[1] - 15000).abs() <= 2);
    }

    #[test]
    fn mixer_clamps_instead_of_wrapping() {
        let mut mixer = SongMixer::new(44100, 1);
        mixer.add_voice(&mono_pcm(&[30000]), 1.0, 0);
        mixer.add_voice(&mono_pcm(&[30000]), 1.0, 0);
        let mixed = mixer.mix();
        assert_eq!(mixed.samples, vec![32767]);
    }

    #[test]
    fn mixer_offsets_late_voices_with_silence() {
        let mut mixer = SongMixer::new(44100, 1);
        mixer.add_voice(&mono_pcm(&[10000]), 1.0, 0);
        mixer.add_voice(&mono_pcm(&[20000]), 1.0, 2);
        let mixed = mixer.mix();
        assert_eq!(mixed.samples.len(), 3);
        assert!((mixed.samples[0] - 10000).abs() <= 2);
        assert_eq!(mixed.samples[1], 0);
        assert!((mixed.samples[2] - 20000).abs() <= 2);
    }

    #[test]
    fn mixer_ignores_silent_and_negative_gains() {
        let mut mixer = SongMixer::new(44100, 1);
        mixer.add_voice(&mono_pcm(&[10000]), 0.0, 0);
        mixer.add_voice(&mono_pcm(&[10000]), -1.0, 0);
        assert_eq!(mixer.song_frames(), 0);
        assert!(mixer.mix().samples.is_empty());
    }

    #[test]
    fn song_position_converts_frames_to_ms() {
        assert_eq!(song_position_ms(44100, 44100), 1000);
        assert_eq!(song_position_ms(0, 44100), 0);
        assert_eq!(song_position_ms(22050, 0), 0);
    }

    #[test]
    fn song_clock_tracks_audio_time_and_drift() {
        let clock = SongClock::new(44100, 44100);
        assert_eq!(clock.position_ms(88200), 1000);
        assert_eq!(clock.position_ms(0), 0);
        assert_eq!(clock.drift_ms(88200, 900), 100);
        assert_eq!(clock.drift_ms(88200, 1100), -100);
    }

    #[test]
    fn latency_probe_reports_pending_and_total() {
        let probe = LatencyProbe::new(44100, 4410, 4410);
        assert_eq!(probe.pending_ms(), 100);
        assert_eq!(probe.total_ms(), 200);
        assert_eq!(LatencyProbe::new(0, 999, 999).total_ms(), 0);
    }
}
