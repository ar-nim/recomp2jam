# T240 join leg — hover-press stimulus (2026-11-17)

Ticket: #240 (Wine pair, room→waiting join). This note records ONE bounded
exe leg: two boots, each with one intended stimulus, both fully packaged.
Captures and packs stay local under `~/scratch/240-join/` (git-ignored);
only this findings note is committed. No exe screenshot or audio bytes are
in the repo.

**Answer in one line:** the boot and the drive to the room stage are
qualified with provenance again, but the hosted room row did not paint in
either boot, so the hover-press stimulus was never sent and no `0BBA` was
observed; the join remains OPEN with a sharper next step below.

## Version pins

| Item | Value |
|---|---|
| Client | `OTwo.exe` 5,029,888 B, sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` (7.55) |
| Wine | 11.22 (wow64), prefix `~/scratch/240-fix/prefix` |
| argv | `scripts/rig-mint-launch-argv.sh`, sha256 `1c95d47820e67…` (blob + 5 mirrors) |
| Injector | `sendinput-helper.exe.so` sha256 prefix `f49e6583b0a7`, rebuilt from `scripts/sendinput-helper.c` with the new `press` verb |
| Capture | ffmpeg n9.0.2, x11grab 800x600 |
| Sink | per-run `rig_null_<pid>` with a `parec` monitor tap; never the owner's sink |
| Build | `1f55e53` (boot 1), `3ce1c83` (boot 2), both clean trees |
| Displays | `:222` (boot 1), `:223` (boot 2), private Xvfb |

## What the stimulus is

`docs/captures/t240/findings.md` OPEN arm 1 names the next evidence step:
one stimulus per boot, in the painted-row window — X-warp the pointer onto
the row to confirm the hotspot from the **exe's own hover highlight**, then
one `SendInput` press with no `SetCursorPos`. That is the mechanical
difference from the proven `siclick3` shape
(`~/scratch/button-unblock/siclick3.c`): this drive's `dclick` path calls
`SetCursorPos` inside Wine before sending. Implemented as
`--join-mode hover-press` (`scripts/wine-scene.py`,
`scripts/rig-wine-run.sh`, `scripts/sendinput-helper.c`), with four
scenario-named seam tests in `scripts/tests/test_join_stimulus.py`.
`--row-repaint recreate` adds the Attempt 12 host-side room rebuild that
repaints the list from fresh 2005 data.

## Boot 1 — qualified negative (hover1)

`scripts/rig-wine-run.sh ~/scratch/240-join/hover1 --display :222
--target waiting --join-mode hover-press --music-id 757`

- All checks PASS: `run-manifest.sh verify`, `rig-audio.sh verify`
  (peak 0.0 dBFS, 162 s), `rig-provenance.sh cite --claim exe`.
- Four distinct screenshots: `planet.png`, `channel.png`, `room.png`,
  `probe-failure.png`. Citation manifest sha256
  `0798a90f3ad5a62f7a41f5f48aa5cf7ee4f9e0aefe8264ddeab7c4f27e797e44`.
- Drive log: `13:13:53 host room created`; `13:13:59 frame 3 screen=room
  cue=server:03EC`; `13:14:39 room row painted=False`; `join attempt 1
  joined=False`; `stage waiting FAILED reason=no 0BBA`.
- `painted=False` short-circuits the join, so no X-warp, no hover capture,
  and no press was sent. The independent pixel probe on the committed
  `room.png` confirms the green banner fraction is 0.0 (`row_painted`
  needs > 0.04). The `07DA`/`07E8` member/music poll did not paint the row
  inside the 40 s window; the client dismissed one message box at
  `13:14:33` and then the idle fuse returned it to the board.
- Audio route verdict was `foreign-sink` with `client_streams=1`: no
  default-sink breach, and the run's own tap captured non-silent audio.
  The verdict did not positively prove the exe stream was on the run sink.

## Boot 2 — aborted at boot by the containment guard (hover2)

`scripts/rig-wine-run.sh ~/scratch/240-join/hover2 --display :223
--target waiting --join-mode hover-press --row-repaint recreate`

- `phases-boot.log`: `rig-preflight: FAIL audio-route: 1 stream(s) of
  client 'rig-wine.sh' on the default sink
  'alsa_output.pci-0000_0a_00.4.analog-stereo'; agent audio must never
  reach the owner's speakers (exit 12)`. The run was stopped immediately;
  no drive, no join stimulus, no screenshots.
- Audio sidecar: `route=default-sink-breach`, `client_streams=1`,
  `peak_dbfs=-91.0` (silence on the run's own sink). This is the #311
  guard working as designed: before the fix this same condition reported
  `route=unseen` and the run continued.
- **Not claimed**: that the boot was isolated from the owner's audio path.
  One stream did appear on the owner's default sink. The guard aborted
  within its 10 s poll and the pack holds no exe pixels. Whether any
  audible sound reached the owner's speakers was not measured and is
  recorded as an OPEN containment question.
- Cause under investigation: `PULSE_SINK` / `PIPEWIRE_NODE` were set to the
  run sink in both boots (`env.sh` verified), yet boot 2's Wine stream
  resolved to the default sink. A stale `wineserver` for the t240 prefix
  was reset before boot 1; two unrelated wineservers (music-extract and
  installer work) were left untouched. Next step: capture `pactl list
  sink-inputs` at boot time and pin which process owns the stream.

## What this qualifies (and what it does not)

- Qualified: the kit boots the blob-minted exe on a private Xvfb with a
  per-run sink, drives `planet → channel → room`, and packages a
  provenance-complete, citation-accepted pack. Four seam tests pin the
  hover-press stimulus order. The #311 guard now aborts a real
  default-sink breach instead of reporting `unseen`.
- Not qualified: the waiting-room join, any `0BBA`, any Waiting/Loading
  pixel signature, and the hover-press hypothesis itself (the stimulus
  never fired). No claim about exe hover-highlight pixels.
- The earlier `docs/captures/t240/` record remains the qualified room-stage
  pack; this leg does not supersede it.

## OPEN arms and named next steps

1. **Row paint.** The 07DA/07E8 poll must land inside the window. Next step:
   one qualified boot that captures the room-stage wire log alongside the
   pixels, confirms `2005` room data reached the client, and only then
   sends the single hover-press stimulus. The host `recreate` rebuild is
   available but boot 2 never reached the drive.
2. **Containment.** Next step: a boot-time `pactl` capture that names the
   process behind the default-sink stream; then either a Wine-side sink fix
   or a documented pre-boot stream check. Until then, a Wine boot can be
   aborted by its own guard, which is correct but blocks the join leg.
3. **`route=foreign-sink`.** Next step: record `on_default_sink`,
   `default_sink`, and `run_sink_streams` from the verdict in the sidecar
   so the route claim is auditable, not inferred.
4. **Stimulus.** Next step: once the row paints, capture
   `room-row-hover.png` with the X-warp highlight delta, send exactly one
   press, and record `0BBA` (or the negative) with the run's manifest.
