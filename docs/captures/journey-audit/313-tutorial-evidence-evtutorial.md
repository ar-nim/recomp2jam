# Tutorial evidence sweep — ev-tutorial lane (spec #313)

Base `97d6ef9`, worktree `cb2/ev-tutorial`. No Rust or script change.
All captures stay in git-ignored scratch. One trace id for the lane:
`S313-EVTUTORIAL-01`. Target: O2Jam 7.55 `OTwo.exe`, 5,029,888 B,
sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.

## Run discipline (every pack)

- Run-class classifier called live with the pinned display: `rig-wine`.
- `scripts/memoryer-start.sh --check-only` -> rc 13, single Memoryer
  pid 37333, boot epoch 1795246022, never restarted.
- Per-session `rig_live_lock_run`; no second lane was booted.
- Display pinned (`RIG_DISPLAY` `:441`…`:450`); caller `DISPLAY` unset before
  `rig-run` so the display-guard cannot trip.
- Both `#731` injector artifacts built in
  `~/scratch/ev-tutorial/injector`: `relay-fastfail.so`
  `a640b214d09e94e258c2d425ae0e6ae6e5f5ea32fca75037482b016699b227f7`,
  `sendinput-helper.exe.so`
  `f49e6583b0a75e635cf906f01e9458b3d7e26c8c5cb5e2bfbfe3fcc72aa85a99`.
  Every boot logged `connect 203.238.136.52:7788 -> ECONNREFUSED` and C2S
  `07DA`/`07E8` within 0–1 s of `03EC` before room clicks.
- SendInput only. `.dmp` Cancel aim pinned at `(449,329)`. No blind Escape.
- Owned null sink and monitor tap; mid-visit `rig-audio.sh assert-route`
  pinned `route=sink`, `on_default_sink=0`, `preboot_breach=0`.
- Tracked `scripts/wine-scene.py` was patched only from scratch and restored
  after every session (`git status` clean at the end).

## Qualified packs

| Pack | Purpose | Result |
|---|---|---|
| `~/scratch/ev-tutorial/runs/w1-audio-route` | tester entry, cue and exit, route proof | `status=PASS`, 34/34 distinct shots, manifest/audio/cite PASS, `route=sink`, peak 0.0 dBFS |
| `~/scratch/ev-tutorial/runs/w2-master-entry2` | master CREATE + entry + key sweep + exit | `status=PASS`, 38/38 distinct shots, `route=sink`, C2S `2004`=1, C2S `4000`=1, `FFF0`=0 |

`w1-entry-audio3` is an earlier PASS pack with `route=unseen`; it supports
pixel pins but not the route claim. `w1-attempt0-no-boot`,
`w1-entry-audio`, `w1-entry-audio2`, `w2-master-entry`, and the four
`w3-loading-linked*` packs are failed or no-boot attempts and are not
cited as evidence.

Key hashes (full file hashes in each pack's `citation.txt`):

| Artifact | sha256 |
|---|---|
| `w1-audio-route/manifest.json` | `00473a2db074e9143e3236277a65618ba23eed7f4fa90590539f049189b429a0` |
| `w1-audio-route/citation.txt` | `6bde2eb82f30be78bf50121ecf4c6e60b0ce5c69047322769c841ce3d103bb6b` |
| `w1-audio-route/…/tutorial.png` | `672f351a4ba405221625320632c73dd47d8903c998f4f0f7c26476b7f0394cfc` |
| `w1-audio-route/…/tutorial-exit-power.png` | `63a73000e018ddf9b6230731892717898053df6cfd2d84dfd1895962407b49bd` |
| `w1-audio-route/…/planet-tutorial-plate-tap.png` | `8303498534a14344c1588a37a93076893289f49401b04425c38d5eae7eb1fc79` |
| `w2-master-entry2/manifest.json` | `cfcd641badc76fcd024c4cf19342f93791f8903287b3b4fff6b425f7d543fa20` |
| `w2-master-entry2/citation.txt` | `ec73255a3edb257b56d8e1376d137ed2f0873de26576f9e737e776b23d558bd4` |
| `w2-master-entry2/…/create-05-waiting-master.png` | in pack `citation.txt` |
| `w2-master-entry2/…/tutorial.png` | `02f741795479d27745c52a1f4a406f280fb25bf176bb465fa0addf52df534356` |
| `w2-master-entry2/…/tutorial-key-s-down.png` | `a5be16a85dbcfae25feded221fa9fb49d4681cbaee432b753df159c5c5f6ff8d` |
| `w2-master-entry2/…/tutorial-exit-power.png` | `9f696d25bfc0329271c131d108872623d70a2105504c78f121f7d8f9c13adb72` |

## Verdicts and next recipes

### #503 — qualified FIRSTSTEP and dimmed Planet plate sweep: PARTIAL PASS

The dimmed TUTORIAL plate (`0x19201006`, live rect `(614,32)-(713,144)`)
received one SendInput tap at `(663,88)` in both qualified packs. The frame
stayed `planet`; the 220 px changed region `(663,88)-(680,112)` is the
pointer overlay, not art. FIRSTSTEP (`0x13200032`, rect `(660,1)-(718,32)`)
received two taps at `(689,16)`; `firststep-tap-1/2.png` and `tutorial.png`
pin the visit. This closes the click sweep, not the dispatch REA arm.

Next: REA xref factory `0x4f07f0` and helper `0x495110` under one evidence
id, paired with these frame hashes; no new click is needed.

### #529 — master key sweep and exit: PASS (master seat)

In `w2-master-entry2`, uid 1 created a room (C2S `2004`) before the visit.
All seven lane keys `s d f space j k l` have distinct down/up frames; the
exit click `(777,567)` returns the Room board. `w1-audio-route` repeats the
sweep on the tester seat. The rig's `scene_label()` reads `planet` on some
Tutorial key frames; the pixels and the side-scene guard show Tutorial.

Next: none for the visual story. Judgement tallies stay outside this lane.

### #499 / #552 — Tutorial OJM cue and cue moment: OPEN

`~/scratch/ev-tutorial/logs/w1-audio-route.cue-trace.txt` (sha256
`fc711ca6a1ead765443fc5bea6d66d1fc0946fe60b73e72f7e1625faf5f106c1`)
gives 0.25 s RMS windows aligned with the driver's own anchors: room BGM
t=132 s, Tutorial entry t=166 s, key sweep 194–211 s, exit t=217 s.
The tap is non-silent at entry (peak −11.5 dBFS, median −16.0 dBFS over the
±8 s window); spectrum shifts at t≈166–168 s. There is no entry silence gap
in this path. No sample identity is proven: correlations against the 12 raw
`bgsplit-*.ogg` tails of `Tutorial.ojm` peak r=0.089, and the 30 decoded
mode-2 PCM entries peak r≈0.64 only in the key-sweep window, not at entry.

Next: one direct Room → FIRSTSTEP run (skip the MusicShop detour) with an
owned-sink A/B across a 5 s pre-entry silence window; pair the first 2 s
after the entry frame with a mode-2 sample correlation at exact sample
rate, and xref the `Tutorial.ojm` loaders at files `0xf0950`/`0xf0d30`.
Do not promote loudness alone to cue identity.

### #555 — host BGM re-cued on exit: PARTIAL PASS

`w1-audio-route` keeps the tap non-silent to t≈245 s after exit, with a
level shift at t≈220 s (~3 s after the exit click) and anchors at +6/+12/+20 s.
Source identity is still unpinned.

Next: run the same A/B against the `BGM.ojm` `bgRoom` / `bgMainRoom` tails
with the route snapshot already pinned by this pack; name the re-cue frame
and the matched sample before closing.

### #558 / #507 — Tutorial + Loading coexistence: OPEN

Four attempts (`w3-loading-linked*`). The best pack, `w3-loading-linked2`,
has a qualified Tutorial frame (`tutorial-dwell-06.png`, page `2/13`) and a
master CREATE (`2004`), but the post-Tutorial room row never opened Waiting
(`0BBA` absent). No pack holds both Tutorial and Loading frames.

Next: one pack with (a) exit guarded by `room_reference_delta` < 30,
(b) `RoomPaintWait` until `07DA`/`07E8`, (c) hover the row at `(100,100)`
and capture the highlight before the press, (d) require server `0BBA`, then
(d) master START at `(680,143)` and `capture_scene(..., "loading")`. Keep
one trace id; do not close on the seam guard.

### #521 / #525 — master entry and first-page art: PASS

`w2-master-entry2` has a master-owned `2004` room, both FIRSTSTEP taps,
`create-05-waiting-master.png`, `room-gated.png`, `tutorial.png` and the
seven-key frames under one input log. Bubble, coach and receptor row are
visible at their staged anchors.

### #542 / #546 — pages 2–13 and coach records: PARTIAL PASS

The master dwell frames show page `1/13` through at least page `2/13`
(`w2-master-entry2/…/tutorial-dwell-06.png`, sha256
`51b0571e2064c7e07a78cd7f952a043264eeca16029b7772bd10c36abbd74be4`
at +24 s); a later frame in `w3-loading-linked2` shows `3/13`. The coach
crop `(645,350)-(783,533)` changes between every 4 s probe (2,921–5,568 px
above 16/255), so motion is observed. Record indices 2–13 and the 12 coach
records are not paired.

Next: capture one page frame per record at 4 s intervals for 60 s, read the
`N/13` counter from each, and pair frame hash with `Guide_Text.ojs` record
index; do the same for `Guide_Ani.ojs`, or xref the record-index writer in
REA before any client change.

### #516 / #549 — P3, untouched

Judgement replay and chart-timed note bodies stay with the chart-replay
lane. No verdict is claimed here.

## Reproducer

The scratch kit lives in `~/scratch/ev-tutorial/`: `session.sh` (preflight,
lock, classifier, kit call, cleanup), `patch-kit.py` (scratch-only seam and
restore), `evt_driver.py` (leg modes), `audio_cue_trace.py`, `cue_correlate.py`,
and `decode-tutorial/` (scratch Rust decoder for the mode-2 bank). Example:

```sh
bash ~/scratch/ev-tutorial/session.sh <name> <entry_audio|master|loading> :NNN 1 0 [loading]
```

Screenshots and audio are local-only; no capture bytes enter the repo.
