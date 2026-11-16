# T280 — AFK end-to-end runner: findings (2026-11-16)

Ticket: #280 (one unattended command: boot, drive, capture three modalities,
self-check, package). Pin: base `26accd9`, runner commit `f392cf9`
(branch `cb2/tt-280`), native pair (`native-xvfb-xtest`), host PipeWire 1.6.11
/ pulse-compat 15.0.0, Xvfb on a private display. The acceptance run below
executed at tree `3d52470` (the same code plus this note's parent commit,
`git_dirty false`). This note records the
acceptance run, the red-proofs, and the OPEN legs. It is **not** exe
evidence: both legs here drove the native Rust client.

## Acceptance run — one command, zero owner interaction

```
scripts/rig-afk.sh ~/scratch/280-fix/accept-gameplay3 \
    --pair native --target playing --music-id 757 \
    --inject-keys lane-0,lane-1,lane-2,lane-3,lane-4,lane-5,lane-6 \
    --inject-count 2
```

Run 23:50:26 → 23:51:17 (51 s), display `:150` (allocated, private Xvfb the
runner started), sink `rig_null_1030311` (per-run owned null sink, unloaded on
stop). Result: `status=PASS`, exit 0, five green checks:

| Check | Line |
|---|---|
| scene | `check scene: PASS reached 'playing'` |
| frames | `check frames: PASS 6 frame PNG(s) >= floor 4` |
| judged | `check judged: PASS injections=14 judged=14 lanes=7/7` |
| audio | `check audio: PASS -3.6 dBFS peak, lane captured` |
| manifest | `check manifest: PASS artifact set, provenance and audio verified` |

Pack (`~/scratch/280-fix/accept-gameplay3`, local, git-ignored): `manifest.json`
`eda8663d…`, `provenance.json` `8f5e274c…` (producer `recomp2jam` sha256
`ca88a91c55476c7f…`, `git_commit 3d52470`, `git_dirty false`,
`cargo_profile debug`, tools `python3/xvfb/wine/capture/injector`),
`input.log` (56 `frame` lines), `injector.log` (14 presses, `05283ebf…`),
`frames/` 6 PNGs (planet, channel, room, waiting, loading, playing),
`audio/audio.wav` (`21e0b5a6…`, 21 MB, 45.9 s, `peak -3.6 dBFS`,
`mean -23.6 dBFS`, `expect=sound state=captured route=sink`),
`audio/manifest` `53d9fc33…`, `launch.txt`, `phases/*.log`,
`report.md`, `report.json`, `status`.

Scene sequence the drive produced: `planet → channel → room → waiting →
loading → playing`. Nothing appeared on the owner's seat or speakers: the
client ran on a display the allocator started, and its only audio stream sat
on the run's own sink (`route=sink`, no default-sink breach in
`phases/boot.log`).

## What the drive needed for gameplay (new, pinned)

Reaching Playing needs **two** Waiting Confirms when a host pick is staged,
not one: the first queues the 4000/4002/4030 select and the shell stays in
Waiting until the chart fetch lands (client log `song staged music_id=757
entries=562 free_music=true`, then `key sounds attached entries=25`), the
second sends START 4010 → 4011 → Loading. Loading + Confirm sends
`ConfirmMusicLoaded` 4012 and enters Playing, where lane presses produce
`judged <ms> lane <n>` lines that `pair-stats.sh` pairs with the injector
record. `scripts/planet-to-waiting.sh` carries this as `--music-id` +
`--target playing`; the staging wait is bounded (120 s) and keyed on the
client's own staging line, never a fixed sleep.

## Red-proofs (selftest, verbatim lines)

`scripts/rig-afk.sh selftest` — hermetic stubs (stub `Xvfb`/`xdpyinfo`, the
audio lane's stub `pactl`/`parec`, a stand-in client under
`target/debug/`, stand-in drive script, stand-in injector). Exit 0:

```
selftest: leg happy-path PASS (complete pack, five green checks)
selftest: leg menu-only-skip PASS (judged check declares the menu-only skip)
selftest: leg scene-red PASS (check scene: FAIL probe reports 'planet', expected 'loading')
selftest: leg frames-red PASS (check frames: FAIL 4 frame PNG(s) in <out>/frames, floor 99)
selftest: leg judged-red PASS (check judged: FAIL injector/judged mismatch: FAIL: count mismatch: injections=7 judged=3)
selftest: leg audio-disabled-red PASS (check audio: FAIL lane declares expect='disabled', an AFK pack needs 'sound')
selftest: leg audio-silent-red PASS (check audio: FAIL run declared sound but the artifact is silent (peak ... dBFS))
selftest: leg manifest-red PASS (check manifest: FAIL run-manifest verify failed: missing: manifest: no provenance.json in <tmp>/manifest)
rig-afk: selftest OK
```

Each leg asserts the run's `status` file, the exact `check … FAIL` line, and
that the evidence survives (input log, audio artifact / manifest, report).
The manifest leg injects the fault through `RIG_AFK_FAULT=drop-provenance`,
a selftest-only knob; the other five faults come from real fixtures (wrong
scene word, `--min-frames`, judged-line shortfall, lane switched off, silent
tap).

## Guards and their live caller chains

- `rig-afk selftest` legs happy/menu-only: live chain
  `rig-afk.sh run` → `allocate_display` → `boot_session` → `rig-run.sh run`
  → `drive_scenes` (scene script `--attach`) → `inject_presses` → `stop_session`
  → `package_pack` → `run_self_checks` → `write_report`.
- `scene-*`/`frames-*`: `run_self_checks` → `run_check` → `check_scene` /
  `check_frames`; the real run above exercised both on live pixels.
- `judged-*`: `check_judged` → `scripts/pair-stats.sh` (#237 pairing), live
  chain proven by the acceptance run (14 = 14, lanes 7/7).
- `audio-*`: `check_audio` → `scripts/rig-audio.sh verify` (#279 lane); the
  acceptance run's artifact measured −3.6 dBFS.
- `manifest-*`: `check_manifest` → `scripts/run-manifest.sh verify`
  (artifact set, provenance #281, audio).
- `scripts/rig-run.sh selftest` and `scripts/rig-audio.sh selftest` stay green
  after the stub move (`scripts/lib/rig-stub-display.sh`): exit 0 both.

## Gates

`cargo fmt --check` 0; `cargo clippy --workspace --all-targets -- -D warnings`
0; `cargo test --workspace` install-root present 2577 passed / 0 failed
(exit 0); install-root hidden 2577 passed / 0 failed (exit 0). Shell scripts
changed, so the Rust suite is a regression check, not a new-behavior check.

## OPEN / doubts (never silently resolved)

1. Wine leg — out of scope per the ticket; the runner refuses
   `--pair wine` and names #240. The XTEST backend is native-only.
2. Frame floor default (4) is a floor, not a budget: a healthy native
   planet→playing run emits 6 gated paints. A scene with fewer gated paints
   (a short menu walk) needs `--min-frames` lowered deliberately, not silently.
3. Contradiction noticed, not resolved: `docs/rig-runbook.md`'s pairing table
   cites `docs/captures/t76/findings.md` for "Playing reached, 60 judged
   presses", but that file today records the opposite (pair B blocked, no
   playing). The claim is repeated in this run's shape but with new evidence
   (this note); the t76 citation should be re-pinned or retracted by its owner
   ticket.
4. `--inject-keys` requires `--target playing` (lane presses are only judged
   while Playing); the runner refuses the combination otherwise instead of
   producing a pack whose judged check could never pass.
