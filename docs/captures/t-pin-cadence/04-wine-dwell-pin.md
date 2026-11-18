# T229 batch 4 — Wine dwell pin for the worn-avatar cadence

## Answer

The exe advances one worn-avatar frame per **66 ms** (12-frame body cycle
**792 ms**). A live 4 s Wine dwell on the Room peer-avatar panel shows 12
recurring crop states with a median full cycle of **799.89 ms**
(66.66 ms/frame), which confirms the static pin within 2 % on the live rig.
The earlier #229 blocker (whole-bbox 10 Hz hashes) was a measurement
problem, not an OPEN cadence.

Evidence IDs:

- `EV-P229-STATIC-1`: exe 7.55, `OTwo.exe` 5,029,888 B, SHA-256
  `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b`.
  CAvatar ctor VA `0x52b290` (file `0x12b290`) stores `0x42` at `+0x1f0`;
  paint VA `0x52b9b0` (file `0x12b9b0`) advances the phase by elapsed/66
  and wraps at 48 (`docs/equip.md`, commit `7857d38`, claim C33).
- `EV-P229-DWELL-1`: live dwell pack `~/scratch/p229-dwell/wine1`.
  The citation quotes provenance `manifest_sha256`
  `89d22b1a5d04ff4d08fd189eb13920e578e5522a47c3a9950e3b09d8f2c8e086`;
  file SHA-256s are `manifest.json`
  `e5c614e4ad969db64299aabee7d862241e97dc9c1c964773b915367742152135`,
  `provenance.json`
  `b64bb3de3f4cc99333a49ba20b8646fd26310645b670fc84331809e4d368181e`,
  `citation.txt`
  `91ea7ffcf0391390a801b9966975c04263dd8903bab56e451607abc2bc4f8f8f`.
  `rig-provenance.sh cite <pack> --claim exe` exited 0; the citation is
  kept in the local pack, not in this repository.

## Red before green

Hypothesis under test: the exe's visible body cycle is the 250 ms
placeholder cycle (3000 ms for 12 frames), not the 66 ms static pin.

- Red prediction: a 4 s dwell shows about 1.3 full cycles and at most
  ~16 frame changes; a 10 Hz whole-bbox hash cannot separate one step.
  The previous #229 attempt (`4ce472b`) stopped there with 200 distinct
  hashes but no cadence.
- Green observation: 917 crop samples over 4 s, 52 visible changes,
  12 distinct avatar states, five phase-zero recurrences at
  738.15 / 794.89 / 809.13 / 806.61 / 799.89 ms. Median 799.89 ms =
  66.66 ms per frame. The 250 ms/3000 ms model is rejected.

## Method (one live boot)

1. Checked `ss -tnp | grep 15010`: busy for 60 s by another agent's host
   bot against the shared, unmodified Memoryer. No process was touched;
   the guest used account `tester2` (id 2) on that server.
2. Booted the pinned `OTwo.exe` through the kit on a private Xvfb `:162`
   with a Korean prefix copy and the run-owned null sink
   `rig_null_p229`: `scripts/rig-run.sh run :162 <out> -- scripts/rig-wine.sh
   launch <out>/argv.txt`. Route sidecar: `route=sink`,
   `on_default_sink=0`, `preboot_breach=0`; the sink was unloaded on stop.
3. Drove `planet -> channel -> room` with
   `scripts/wine-scene.py --attach <out> --display :162 --target room
   --host-user-index-id 0 --account tester2` (cues `03EA`/`03EC` in
   `drive.log`, scene PNGs in the pack). `WAYLAND_DISPLAY` and
   `XDG_SESSION_TYPE` were unset before every boot.
4. Sampled only the Room peer-avatar panel (x 648, y 120, 132x146 px) with
   `python-xlib` `XGetImage` at ~4.2 ms intervals for 4 s
   (`~/scratch/p229-dwell/cadence_capture.py`). Wall nanoseconds and the
   SHA-256 of each crop went to `cadence/timing.csv`; raw crops and the
   cycle analysis stay in the git-ignored pack.
5. Stopped with `scripts/rig-run.sh stop <out>`; zero owned processes
   remain. No winedbg was used: no `cont`, no disassembly pasted.

## Pack checks

`run-manifest.sh verify` PASS, `rig-audio.sh verify` PASS (peak 0.0 dBFS,
tap on the owned sink), `rig-provenance.sh cite --claim exe` PASS
(exit 0). Key artifact hashes:

| Artifact | SHA-256 |
|---|---|
| `cadence/analysis.json` | `c292d1595875ce829ac4d4a33e42f7504dc66d83672f3839860c406e9dc04cfb` |
| `cadence/timing.csv` | `a1aad093bc0911ccbcbfd07e1aa0e5d480f264affa5599e7d8663a537435afce` |
| `cadence/avatar-crops.bin` | `ea846be3b065b44258fca7a0124d2631c0b5878a564d98179bf2687c1fc702e4` |
| `artifacts/screenshots/room.png` | `bfe9ace5801ad2079bdbc67a037f5a3cbe049744342b41e7cfda885bde474268` |

The raw pack holds copyrighted pixels and audio and is never committed;
only this textual pin and the hashes are tracked.

## Consequence

`#223` consumes this pin: 66 ms tick and 66 ms repaint pulse are already
in `crates/o2jam-shell/src/avatar.rs:47` and
`crates/o2jam-shell/src/screens/game/mod.rs:6870`. `#223` now needs the
fresh live re-observe named in its spec-audit comment, not a new cadence
guess. Interpolation stays out of scope without an owner decision.
