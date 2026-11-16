# T279 — rig audio capture lane: findings (2026-11-16)

Ticket: #279 (rig audio capture path, virtual sink + monitor tap). This
note records the method, the two acceptance runs, and the OPEN Wine leg.
It is **not** exe evidence: both acceptance runs drove the native Rust
client. Wine-leg claims here rest on the pinned Wine-boot evidence noted
at the end.

## Method (exact)

```
scripts/rig-run.sh run :140 <out-dir> -- ./target/debug/recomp2jam <args>
# drive / let the scene play
scripts/rig-run.sh stop <out-dir>
scripts/rig-audio.sh verify <out-dir>
```

`run` creates a per-run owned sink (`rig_null_<pid>`), exports
`PULSE_SINK` and `PIPEWIRE_NODE` at it, opens the monitor tap
(`parec -d <sink>.monitor`, `float32le` 48 kHz stereo) into
`<out-dir>/audio/audio.wav`, then starts the client. `stop` reaps the
recorder by process group, measures the artifact, and finalizes
`<out-dir>/audio/manifest`.

Host (provenance of the probe facts): PipeWire 1.6.11, pulse-compat
15.0.0, `pactl`/`pacat` 17.0-134-g77d25, ffmpeg n9.0.2,
`libasound_module_pcm_pipewire.so` sha256
`e09cf936292d229d62aaf43848e1efeed202a46d0fc627e06e3ba5e3fcdd3c1d`.
Xvfb on a private display `:140` (no owner seat, no default-sink audio).

## Routing probe results (native leg)

| Producer | Lever | Result |
|---|---|---|
| cpal/ALSA default (client stack) | `PIPEWIRE_NODE=<sink>` | sink-input on the rig sink (`Sink: 1606`), monitor tap records it |
| `sox → -t alsa default` | `PIPEWIRE_NODE=<sink>` | rig sink, tap peak −21.07 dBFS |
| `pacat --playback` (pulse client) | `PULSE_SINK=<sink>` | rig sink, tap peak −1.36 dBFS |
| same ALSA producer, no lever | — | system default sink (the leak the levers prevent) |

Negatives that shaped the guards (observed, not assumed):

1. `pw-record --target does-not-exist` recorded the default *source*
   with no error (RMS −47 dB room audio).
2. `parec -d does-not-exist.monitor` recorded the default source
   (digital silence at that moment) with no error.
3. A remembered PipeWire stream-restore mute leaves the stream running
   but the tap digital-silent (see run 1 below).

## Acceptance run 1 — native autoplay (key sounds)

- Out-dir: `~/scratch/279-fix/accept-native2` (local, git-ignored).
- Command:
  `recomp2jam play --chart install-root/O2Jam/Music/o2ma757.ojn --ojm install-root/O2Jam/Music/o2ma757.ojm --level 0`
- Binary sha256 `a6f5c5f8ee0dc9228021ec0074b71af7795bc84b9d6ecc63a271b54726adb46a`
  (debug build, branch `cb2/tt-279` at 1169dbb base).
- Artifact: `audio/audio.wav`, 12583000 bytes, sha256
  `997fa0a53ae0d0bde43ede963266056167d977580f8377915ad07ad3b09b445a`,
  32.768 s, `peak_dbfs=0.0`, `mean_dbfs=-14.3`.
- Sidecar: `expect=sound`, `state=captured`, `source_verified=yes`,
  `route=sink`, `sink_spec=float32le 2ch 48000Hz`,
  `resampling=none (tap matches the sink)`, `client_streams=1`,
  `streams_normalized=1`, `muted_streams=0`.
- Verifier: `scripts/rig-audio.sh verify` → exit 0
  (`audio OK (peak 0.0 dBFS > floor -60 dBFS)`).

An earlier run (same command, tap `s16le`) produced **39.25 s of
−91 dBFS**: the client stream carried `Mute: yes` from PipeWire
stream-restore, so the recorder ran and the graph looked healthy while
the artifact was silent. That run is why the lane normalizes mute/volume
on its own sink's streams (`route-muted-stream` guard).

## Acceptance run 2 — native windowed boot (scene BGM cue)

- Out-dir: `~/scratch/279-fix/accept-bgm`.
- Command: `recomp2jam play --window` — the launch BGM cue fires
  (`launch BGM cued cue_id=8 cue_repeat=false` in `client.log`).
- Artifact: `audio/audio.wav`, sha256
  `073e1b5ea09c126c50baf52c678f8e2b87a4bad343c0d513c3ca3b9408e4aafc`,
  27.307 s, `peak_dbfs=-3.8`, `mean_dbfs=-25.5`.
- Sidecar: `expect=sound`, `state=captured`, `route=sink`,
  `resampling=none (tap matches the sink)`.
- Verifier: exit 0 (`audio OK (peak -3.8 dBFS > floor -60 dBFS)`).

## Wine leg — OPEN (separate leg, next step named)

Not attempted here: a Wine boot needs the minted `AuthParameters` blob
plus the five gateway mirrors (`docs/wine-boot.md`, Attempt 4) and its
own launch/aim work, which is #240's launcher rather than this lane.

What this ticket did establish for that leg: the premise "switch the
prefix from the Null driver to winepulse" is stale. Wine 11.22 ships the
pulse driver, and the pinned Wine-boot note already records the exe's
audio reaching `wine_null` via `PULSE_SINK`
(`docs/wine-boot.md`, "Audio containment (per owner request)",
2026-11-08: `Sink Input` for `OTwo.exe` on the null sink). So the sink,
tap, mute normalisation, route guard, and verifier are leg-agnostic; the
next evidence step is to bind a blob-launched `OTwo.exe` boot to
`rig_null_<pid>` in one `rig-run.sh` invocation and run
`rig-audio.sh verify` on that artifact.

## Guards and their annotations

- `probe-silence` / `verify-sound-silent` / `verify-sound-tone` /
  `verify-declared-silence` / `verify-failed-lane` /
  `verify-missing-lane` (`scripts/rig-audio.sh selftest`):
  live chain `rig-run.sh stop` → `cmd_stop` → `rig_audio_capture_stop`
  → sidecar → `rig-audio.sh verify`.
- `capture-roundtrip` / `capture-source-mismatch`: live chain
  `rig-run.sh run` → `cmd_run` → `preflight_run_checks` →
  `rig_audio_capture_start` → `parec -d <sink>.monitor`.
- `route-run-sink` / `route-muted-stream` / `route-foreign-sink` /
  `route-default-sink`: live chain `rig-run.sh run` → `cmd_run` →
  `assert_audio_route` → `rig_audio_assert_route` →
  `rig_audio_normalize_streams`.
- `audio-lane`, `audio-silent-declared-sound`, `audio-declared-silence`,
  `audio-route-run-sink`, `audio-route-breach`
  (`scripts/rig-run.sh selftest`): the same chains driven through
  `run`/`stop` end to end on stub PipeWire tools.
