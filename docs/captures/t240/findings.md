# T240 — Wine pair qualification run: findings (2026-11-17)

Ticket: #240 (Wine leg of release qualification). This note records one
qualified run of the 7.55 exe through the unified rig kit, its provenance
manifest, the A/B check against the blessed native pair, and the open arms
that block the waiting-room and loading scenes. Captures and pack stay
local (git-ignored `~/scratch/240-fix/`); the manifests, the drive log, and
the host log are committed beside this note.

**Answer in one line:** the Wine pair boots from a blob-minted argv through
the kit on its own display and per-run sink, and drives
`planet -> channel -> room` with three pairwise-distinct captures under a
complete provenance manifest that `rig-provenance.sh cite --claim exe`
accepts; the join into the waiting room remains blocked and is the OPEN arm
named below.

## Pins

| Item | Value |
|---|---|
| Client | `OTwo.exe` 5,029,888 B, sha256 `83a045418b6219ba340681766b33de2bfbb5ff51b813ef4e2e9dd1def72c823b` (7.55) |
| Run from | `~/scratch/run-full` (cwd must be the game dir; `docs/wine-boot.md` Attempt 12) |
| Wine | 11.22 (wow64), prefix `~/scratch/240-fix/prefix` (Korean clone, CP949 fonts) |
| Display | Xvfb `:160` 1024x768x24, started by `scripts/rig-run.sh` (Wine desktop `o2jam` 800x600 at the origin) |
| Sink | `rig_null_1509711` (per-run owned null sink, monitor tap via `parec`) |
| Build | `f0f5a6d0e5509e3a22d8c4d9d87cecb7b0017f81` (main at the run) |
| argv | `scripts/rig-mint-launch-argv.sh` -> #265 builder (`o2jam_net::auth_parameters::assemble_launch_arguments`), blob + 5 `|test|??|127.0.0.1|15010` mirrors, argv sha256 `1c95d47820e67954823e35787853d71f5401acdac47576a64d636045e2aab7d5` |
| Injector | `sendinput-helper.exe.so` built from `scripts/sendinput-helper.c`, sha256 prefix `29770ddd3574` |
| Capture | `ffmpeg` n9.0.2, x11grab 800x600 at the display origin |

## What ran (one command per phase, all kit calls)

```
scripts/rig-wine-run.sh ~/scratch/240-fix/wine2 --target room
  mint     scripts/rig-mint-launch-argv.sh --user-index-id 1
  boot     scripts/rig-run.sh run :160 <out> -- scripts/rig-wine.sh launch <out>/argv.txt
  drive    scripts/wine-scene.py --attach <out> --display :160 --target room
  stop     scripts/rig-run.sh stop <out>
  package  run-manifest.sh write + rig-provenance.sh emit (exe, 7.55, pinned sha256)
  verify   run-manifest.sh verify, rig-audio.sh verify, rig-provenance.sh cite --claim exe
```

Result: `status=PASS`, exit 0.

## Scene by scene

Each row is one capture in `artifacts/screenshots/`, labelled only because
its own pixels matched the scene signature measured on this leg (a frame
that shows something else is refused and reported, never relabelled).

| Scene | Cue (the exe's own frame, or the board probe) | mean luma | Capture sha256 | md5 |
|---|---|---|---|---|
| planet (notice carousel over the board) | `probe:board` (navy MENU band + emblem strip) | 61.2 | `235f5352…dc21` | `5942f6af6e0248176ced25a45c238b35` |
| channel (Channel dialog, rows live) | `server:03EA` = the client's GetChannelList | 82.6 | `ea5cc6d6…c563` | `1b262c4be9f1e2d2d3f11f711ac43b23` |
| room (room stage, list chrome) | `server:03EC` = the client's GetRoomList | 118.6 | `f91adc41…542f` | `4b8bd838865d1914c898c1bfcdd20231` |

The three md5s differ; `run-manifest.sh verify` enforces that (it fails a
renamed copy, the #278 retraction mode).

Aim points used, each printed with its evidence into `wine-drive.log`:

| Target | Aim | Evidence |
|---|---|---|
| EASY emblem (opens the dialog) | (136,88) | `~/scratch/planet-verify/findings.md` (reopen by `siclick (136,88)`) |
| NORMAL-mode message box OK | (400,365) | same note (the box blocks canvas clicks until dismissed) |
| Channel `입장` face, row 0 | (526,248) | `docs/channel-fill.md` enter face (502,236) 44x21 |
| Room row title bar | (100,100) | `docs/wine-boot.md` Attempt 13 hot zone |
| Waiting READY pill (not reached) | (680,143) | `docs/wine-boot.md` Attempt 11 |

## Manifest and citation

| Artifact | sha256 |
|---|---|
| `manifest.json` | `57089d0365191ebba4261abe5f5d38785908f528a3e7ca80e905e357cf3f3cfa` |
| `provenance.json` | `f6f7f52ceaa70fe26e9dffde9fba1d252bac50a68410f83296b52a35ee3be994` |
| provenance `manifest_sha256` | `a47d90cd5fde27c2e7ce7000833178ac5e829fa54a0bd3f75e5dc99b92f5b69c` |
| `audio/audio.wav` (90.1 s) | `e7562862717bac8c05339111ba5fb3a4df171e2e86ec0401439ed4d28876ec2b` |

`rig-provenance.sh cite --claim exe` accepted the pack (producer kind `exe`,
version 7.55, pinned sha256, full command, tool versions, every capture
hashed). The printed paragraph is committed as `wine-citation.txt`.

Audio: the exe's stream landed on the run's own sink (verified live:
`pactl list sink-inputs` showed `application.name = "OTwo.exe"` on
`rig_null_1179011` in the first boot of this leg) and the tap measured
peak 0.0 dBFS / mean −21.8 dBFS over 90 s — the Wine audio lane
(`PULSE_SINK`, no driver switch) works, which is the row the runbook left
OPEN for this ticket.

## A/B against the blessed pair

The blessed pair is Xvfb + XTEST driving our client (`docs/rig-runbook.md`
"Scope and pairing"). Fresh pack, same day, same build:

```
scripts/rig-afk.sh ~/scratch/240-fix/native --pair native --target loading \
    --music-id 757 --binary target/debug/recomp2jam
```

| | Wine pack | Native pack (blessed) |
|---|---|---|
| pair | `wine-x11-sendinput` | `native-xvfb-xtest` |
| display | `:160` (rig-run started it) | `:150` (runner allocated it) |
| scene sequence | `planet -> channel -> room` | `planet -> channel -> room -> waiting -> loading` |
| captures | 3 PNG, md5 pairwise distinct | 5 PNG, md5 pairwise distinct |
| audio | peak 0.0 dBFS, 90.1 s | peak −3.6 dBFS, 45.2 s |
| manifest sha256 | `57089d03…` | `646afd57…` |
| provenance sha256 | `f6f7f52c…` | `aba3d037…` |

Cross-pair check (the machine gate, run against both manifests):

```
scripts/run-manifest.sh verify <wine>/manifest.json <native>/manifest.json
run-manifest: artifact set complete
rig-provenance: manifest complete
rig-audio: audio OK (peak 0.0 dBFS > floor -60 dBFS): .../wine2/audio/audio.wav
exit 0
```

The native pack's provenance was re-emitted once by the kit so that the
committed copy carries no session token in `session.command` (the value is
the local Memoryer test account's guid, and `--token <sha256 …>` keeps its
identity checkable). Its screenshots were also copied from `frames/` into
`artifacts/screenshots`, which is the dir its manifest declares — that copy
is what gives the cross-pair distinctness check anything to compare.

The check is not vacuous on this pack set: verifying the Wine manifest
against itself reports the duplicates and exits 1 —

```
run-manifest: CROSS-PAIR duplicate screenshots
  dupe: channel.png duplicates channel.png
```

Two notes on how the two packs differ, so nobody reads this as pixel
parity: the stimuli are the same *scene script contract* (attach to a
booted session, drive planet→…, one capture per scene), not one shared
input trace — the Wine pair presses canvas controls through SendInput at
exe-pixel aims while the native pair drives control ids through XTEST, so
per `docs/rig-runbook.md` ("compare only on matching IDs") the comparable
record here is the scene sequence plus capture distinctness, and the Wine
sequence is a prefix of the native one (the join below is the gap).

## OPEN arm 1 — the waiting-room join (blocks waiting + loading)

Observed, repeatedly: after the client enters the room stage (`03EC`), the
hosted room's row paints only when the client's member/music poll
(`07DA`/`07E8`) has run. On a cold session that poll is ~36 s away, and the
room stage's idle fuse (the documented `서버로 부터 응답이 없습니다` cycle)
can take the client back to the board first. Two attempts did have the row
on screen at click time (`room row painted=True`, captured in
`~/scratch/240-fix/explore-5`, `explore-6`) and still produced no `0BBA`
from a double-click at the documented hot zone (100,100). A scratch
session tried six further stimuli in one cycle — quick-join (443,392),
single (100,100), double at (150,105), (60,100), (250,110), and single plus
`Return` — none joined; at least one of those landed after the stage had
already returned to the board, so that sample proves nothing but the
timing.

So: 2 stocked attempts at the documented aim plus 6 exploratory stimuli, 0
joins, against a host bot that provably creates and holds the room
(`wine-host.log`: `host room created number=0 title=WINEJOIN`, music
757/2/1) on an account no other rig was using (`t207probe`).

Next evidence step (one stimulus per boot, in the painted-row window):
hover the row first (X warp, settle, capture) to confirm the hotspot from
the client's own highlight, then one `SendInput` press; and test the
X-warp-then-SendInput order against the helper's `SetCursorPos`-then-press,
which is the one mechanical difference from the proven `siclick3` usage
(`~/scratch/button-unblock/`).

## OPEN arm 2 — smaller gaps this leg found

- **Loading has no pixel signature yet.** `planet`, `channel`, and `room`
  are gated on measured signatures; the waiting room and the loading gate
  are accepted as "none of the three calibrated scenes" on their wire cue
  only. This run never reached them, so the arm is untested, not closed.
- **Route guard cannot see the Wine stream.** `scripts/rig-run.sh`'s
  containment check maps client streams by the *client basename*, which for
  this leg is the launcher `scripts/rig-wine.sh`, so it reports
  `route=unseen` while `OTwo.exe` sits on the run sink. The level probe
  still judges the artifact (0.0 dBFS here), and this leg checked the route
  by hand; the guard should match the exe's own stream name for the Wine
  leg (follow-up ticket).
- **`run-manifest.sh write` and `rig-afk.sh` disagree about screenshots.**
  The manifest declares `artifacts/screenshots`, the AFK pack writes
  `frames/`, so a cross-pair verify against a fresh AFK pack compares
  nothing and passes vacuously. This note's native pack was completed by
  copying its five frames into `artifacts/screenshots` and re-emitting
  provenance, which is what made the A/B check bite.
- **Message box flicker.** The blue body's probe reads 0.99 when the box is
  up but a single frame can read low (the ring animates), which silently
  swallowed a click once; the drive now samples twice 0.4 s apart.
- **Environment facts for the next Wine run.** Wine 11.22 offers to install
  `wine-mono` on a prefix cloned from 11.21 and blocks the boot behind that
  dialog until the prefix is updated once with
  `WINEDLLOVERRIDES="mscoree,mshtml=" wineboot -u`; a relaunch against a
  live `wineserver` dies in `X_CreateWindow` (BadWindow `0xa00007`), so the
  launcher resets its own prefix's server; and the Memoryer console log
  carries ANSI colouring, so a log follower must strip escapes before it
  reads opcodes.

## What this qualifies (and what it does not)

- Qualified: the Wine pair boots the exe from a blob-minted argv through the
  kit, on its own display and per-run sink, authenticates to local
  Memoryer, and drives three scenes with provenance-complete captures.
- Not qualified: any claim about the waiting room, song loading, or
  gameplay under Wine, and any pixel-parity claim against the native pair.
  The earlier #278 retraction was about citing captures without manifests;
  every capture cited here carries one, and the three captures are distinct
  files.
