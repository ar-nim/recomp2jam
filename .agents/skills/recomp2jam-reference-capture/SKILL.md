---
name: recomp2jam-reference-capture
description: Capture and normalize O2Jam 7.55 reference behavior and recomp2jam Rust outputs for parity review. Use when working with Wine/Xvfb, scene screenshots, audio/video recordings, capture manifests, sync points, or reference provenance.
---

# recomp2jam Reference Capture

Produce reproducible evidence for the Windows 7.55 client and the portable
Rust client. Keep the two roles explicit: `reference` means the original
client; `test` means recomp2jam.

## Prepare the run

1. Read `docs/install-layout.md`, `docs/wine-boot.md`, `docs/playtest.md`, and
   the relevant scene in `docs/parity.md`. Current environment notes override
   stale display numbers, process IDs, tokens, and commands.
2. Verify the reference file identity against `docs/progress.md`: normally
   `AngelO2Jam.exe` or its byte-identical `OTwo.exe`, 5,029,888 bytes, with the
   pinned 7.55 SHA-256. Record the full hash in local capture metadata.
3. Use a scratch copy of the install and an isolated Wine prefix and display.
   Keep `install-root/` read-only. Put captures, helper scripts, and logs under
   `$HOME/scratch` or another git-ignored location, not in the repository.
4. Check for existing clients, Xvfb displays, servers, and ports before
   starting a run. Do not disturb a user's desktop or another worker's rig.
5. Record Wine, Xvfb, ffmpeg, Rust, OS, and relevant server versions. Record
   launch arguments and settings without putting credentials in committed
   files.

## Capture comparable scenes

1. Select named anchors such as notice/channel, room, waiting, music select,
   loading, playing, and result. Note that the current Wine path can bypass
   `STATE_AVI`; capture what actually happened rather than forcing the expected
   lifecycle onto the evidence.
2. Preserve native 800x600, 4:3 frames where the scene uses the verified 7.55
   canvas. Save raw captures before trimming, scaling, or resampling.
3. Lock the scenario, room/song/difficulty, speed, input trace, and settings.
   Record unavoidable scenario or vintage differences instead of hiding them.
4. Define a visible or audible sync event and a monotonic capture timeline.
   Save timestamps for scene transitions and input application when timing is
   under test.
5. Capture long enough to include the complete behavior and a stable lead-in.
   Record dropped frames, focus changes, popups, network fallbacks, and input
   injection failures as run metadata, not as game behavior.

## Contain and validate audio

- Route Wine and Rust test output to a dedicated virtual/null sink. Never let
  automated game audio reach the user's speakers.
- A null sink can provide a monitor source, but verify that the recording
  contains non-silent game audio. A successful ffmpeg process alone does not
  prove audio capture.
- Record sample rate, channel layout, sink/source names, and any resampling.
  Keep clock drift and offset measurements separate from loudness metrics.

## Normalize without erasing evidence

1. Preserve the raw file and its hash. Derive trimmed or transcoded copies with
   recorded commands and exact timestamps.
2. Match dimensions, frame rate, sample rate, and channel layout only when the
   comparison requires it. Document every transformation.
3. Keep capture-tool overlays and watermarks out of the evidence. If fonts or
   CP949 glyphs are unavailable, record that environment limitation.
4. Review a short aligned segment before running batch metrics. Reject runs
   with the wrong scene, blocked input, silent audio, or uncontrolled settings.

## Metadata and completion

Store one metadata record per capture with: capture ID, role, scene, start and
end times, source hash, install provenance, command/settings, input trace ID,
video/audio format, sync event, known limitations, and raw/derived file hashes.

A capture is complete only when another engineer can reproduce its scenario,
locate its sync point, and tell reference evidence apart from Rust output.
Keep proprietary media local; commit only synthetic fixtures and textual
findings permitted by `AGENTS.md`.
